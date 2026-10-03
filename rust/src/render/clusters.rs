//! Clustered forward lighting (RP17 in the Technical Art and Rendering Plan). The view is cut
//! into `TILES_X` by `TILES_Y` screen tiles and `SLICES` slices of view depth (spaced
//! logarithmically, so near slices are thin). Each frame every light is binned into the clusters
//! its sphere of reach touches, and each pixel shades with its own cluster's lights only.
//!
//! A cluster holds at most `CELL_CAP` lights. Lights arrive in priority order (the hero's lantern
//! first, then nearest the eye), so a cluster that overflows keeps its most important lights and
//! drops the same ones frame after frame; the drops are counted for the diagnostics.

use glam::{Mat4, Vec3, Vec4};

pub const TILES_X: u32 = 16;
pub const TILES_Y: u32 = 9;
pub const SLICES: u32 = 24;
pub const CELLS: u32 = TILES_X * TILES_Y * SLICES;
/// The most lights one cluster shades with.
pub const CELL_CAP: u32 = 32;
/// The most lights a frame can hold.
pub const MAX_LIGHTS: usize = 1024;
/// A light with no cutoff (distance 0) reaches this far.
const UNBOUNDED: f32 = 1000.0;

/// One light as the binning sees it: where it is and how far it reaches.
#[derive(Clone, Copy, Debug)]
pub struct Reach {
    pub at: Vec3,
    pub range: f32,
}

/// The camera the clusters are cut for.
#[derive(Clone, Copy, Debug)]
pub struct ClusterView {
    pub view_proj: Mat4,
    pub eye: Vec3,
    /// Unit view direction; view depth is measured along it.
    pub fwd: Vec3,
    pub near: f32,
    pub far: f32,
}

/// The binned lights: per cluster, where its run of indices starts (shifted 8) and how long it
/// is (low 8 bits), then the runs themselves; and how it went.
#[derive(Default, Debug)]
pub struct ClusterGrid {
    pub cells: Vec<u32>,
    pub indices: Vec<u32>,
    /// Light-in-cluster pairs that did not fit, the most lights any cluster holds, and how many
    /// pairs were kept.
    pub dropped: u32,
    pub busiest: u32,
    pub pairs: u32,
}

/// The depth slice for view depth `d` (as the shader works it out).
pub fn slice_of(d: f32, near: f32, far: f32) -> u32 {
    let k = (d.max(near) / near).ln() * SLICES as f32 / (far / near).ln();
    (k.floor().max(0.0) as u32).min(SLICES - 1)
}

/// The tiles (x0..=x1, y0..=y1) a sphere covers on screen, or None if it is wholly behind the eye.
fn tiles_of(view: &ClusterView, r: &Reach, range: f32) -> Option<(u32, u32, u32, u32)> {
    let mut min = glam::Vec2::splat(f32::INFINITY);
    let mut max = glam::Vec2::splat(f32::NEG_INFINITY);
    for k in 0..8 {
        let corner = r.at
            + Vec3::new(
                if k & 1 == 0 { -range } else { range },
                if k & 2 == 0 { -range } else { range },
                if k & 4 == 0 { -range } else { range },
            );
        let c: Vec4 = view.view_proj * corner.extend(1.0);
        if c.w <= 1e-4 {
            // A corner behind the eye: the sphere may cover any part of the screen.
            return Some((0, TILES_X - 1, 0, TILES_Y - 1));
        }
        let ndc = c.truncate().truncate() / c.w;
        min = min.min(ndc);
        max = max.max(ndc);
    }
    if max.x < -1.0 || min.x > 1.0 || max.y < -1.0 || min.y > 1.0 {
        return None;
    }
    let tx = |x: f32| (((x * 0.5 + 0.5) * TILES_X as f32).floor().max(0.0) as u32).min(TILES_X - 1);
    let ty = |y: f32| (((0.5 - y * 0.5) * TILES_Y as f32).floor().max(0.0) as u32).min(TILES_Y - 1);
    Some((tx(min.x), tx(max.x), ty(max.y), ty(min.y)))
}

/// The depth at the start of slice `s` (the inverse of `slice_of`).
fn slice_start(s: u32, near: f32, far: f32) -> f32 {
    near * (far / near).powf(s as f32 / SLICES as f32)
}

/// Each cluster's box in the world: the corners of its tile at its slice's two depths, found by
/// casting each tile corner's ray (perspective or orthographic alike) to those depths.
fn cluster_boxes(view: &ClusterView) -> Vec<(Vec3, Vec3)> {
    let inv = view.view_proj.inverse();
    // The ray through each tile corner: where it is at depth 0 along the view, and how it moves with depth.
    let mut rays = Vec::with_capacity(((TILES_X + 1) * (TILES_Y + 1)) as usize);
    for y in 0..=TILES_Y {
        for x in 0..=TILES_X {
            let ndc = glam::Vec2::new(x as f32 / TILES_X as f32 * 2.0 - 1.0, 1.0 - y as f32 / TILES_Y as f32 * 2.0);
            let a = inv.project_point3(ndc.extend(0.0));
            let b = inv.project_point3(ndc.extend(1.0));
            let (da, db) = ((a - view.eye).dot(view.fwd), (b - view.eye).dot(view.fwd));
            let step = (b - a) / (db - da).max(1e-6);
            rays.push((a - step * da, step));
        }
    }
    let at = |x: u32, y: u32, d: f32| {
        let (o, step) = rays[(y * (TILES_X + 1) + x) as usize];
        o + step * d
    };
    let mut out = Vec::with_capacity(CELLS as usize);
    for s in 0..SLICES {
        let (d0, d1) =
            (slice_start(s, view.near, view.far), if s + 1 == SLICES { f32::MAX.sqrt() } else { slice_start(s + 1, view.near, view.far) });
        let d1 = d1.min(view.far * 4.0);
        for y in 0..TILES_Y {
            for x in 0..TILES_X {
                let (mut lo, mut hi) = (Vec3::INFINITY, Vec3::NEG_INFINITY);
                for (cx, cy) in [(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)] {
                    for d in [d0, d1] {
                        let p = at(cx, cy, d);
                        lo = lo.min(p);
                        hi = hi.max(p);
                    }
                }
                out.push((lo, hi));
            }
        }
    }
    out
}

/// Bins `lights` (in priority order) into the clusters of `view`: first the tiles and slices
/// their spheres' bounds cover, then only the clusters whose box the sphere really touches.
pub fn build(lights: &[Reach], view: &ClusterView) -> ClusterGrid {
    let cluster_box = cluster_boxes(view);
    let mut boxes = Vec::with_capacity(lights.len());
    for r in lights {
        let range = if r.range > 0.0 { r.range } else { UNBOUNDED };
        let d = (r.at - view.eye).dot(view.fwd);
        if d + range < view.near {
            boxes.push(None);
            continue;
        }
        let slices = (slice_of(d - range, view.near, view.far), slice_of(d + range, view.near, view.far));
        boxes.push(tiles_of(view, r, range).map(|t| (t, slices)));
    }
    let cell = |s: u32, y: u32, x: u32| ((s * TILES_Y + y) * TILES_X + x) as usize;
    let each = |f: &mut dyn FnMut(usize, usize)| {
        for (i, b) in boxes.iter().enumerate() {
            let Some(((x0, x1, y0, y1), (s0, s1))) = *b else { continue };
            let r = &lights[i];
            let range = if r.range > 0.0 { r.range } else { UNBOUNDED };
            for s in s0..=s1 {
                for y in y0..=y1 {
                    for x in x0..=x1 {
                        let c = cell(s, y, x);
                        let (lo, hi) = cluster_box[c];
                        if r.at.clamp(lo, hi).distance_squared(r.at) <= range * range {
                            f(i, c);
                        }
                    }
                }
            }
        }
    };
    // Count (capped), lay the runs out, then fill them in priority order.
    let mut grid = ClusterGrid { cells: vec![0; CELLS as usize], ..Default::default() };
    let mut counts = vec![0u32; CELLS as usize];
    each(&mut |_, c| {
        if counts[c] < CELL_CAP {
            counts[c] += 1;
        } else {
            grid.dropped += 1;
        }
    });
    let mut offsets = vec![0u32; CELLS as usize];
    let mut total = 0;
    for c in 0..CELLS as usize {
        offsets[c] = total;
        total += counts[c];
        grid.busiest = grid.busiest.max(counts[c]);
        grid.cells[c] = (offsets[c] << 8) | counts[c];
    }
    grid.pairs = total;
    grid.indices = vec![0; total as usize];
    let mut filled = vec![0u32; CELLS as usize];
    each(&mut |i, c| {
        if filled[c] < counts[c] {
            grid.indices[(offsets[c] + filled[c]) as usize] = i as u32;
            filled[c] += 1;
        }
    });
    grid
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> ClusterView {
        let eye = Vec3::new(0.0, 2.0, 6.0);
        let target = Vec3::ZERO;
        let vp = Mat4::perspective_rh(50f32.to_radians(), 16.0 / 9.0, 0.1, 80.0) * Mat4::look_at_rh(eye, target, Vec3::Y);
        ClusterView { view_proj: vp, eye, fwd: (target - eye).normalize(), near: 0.1, far: 80.0 }
    }

    fn lights_in(grid: &ClusterGrid, c: usize) -> Vec<u32> {
        let (start, n) = (grid.cells[c] >> 8, grid.cells[c] & 255);
        grid.indices[start as usize..(start + n) as usize].to_vec()
    }

    #[test]
    fn a_light_lands_in_the_cluster_under_it_and_not_far_away() {
        let v = view();
        let grid = build(&[Reach { at: Vec3::ZERO, range: 1.0 }], &v);
        // The cluster at the middle of the screen, at the light's depth.
        let d = (Vec3::ZERO - v.eye).dot(v.fwd);
        let s = slice_of(d, v.near, v.far);
        let mid = ((s * TILES_Y + TILES_Y / 2) * TILES_X + TILES_X / 2) as usize;
        assert_eq!(lights_in(&grid, mid), vec![0]);
        // Not in the screen's corner, nor far behind it in depth.
        assert!(lights_in(&grid, 0).is_empty());
        let deep = (((SLICES - 1) * TILES_Y + TILES_Y / 2) * TILES_X + TILES_X / 2) as usize;
        assert!(lights_in(&grid, deep).is_empty());
        assert_eq!((grid.dropped, grid.busiest), (0, 1));
    }

    #[test]
    fn behind_the_eye_and_off_screen_are_left_out() {
        let v = view();
        let grid = build(&[Reach { at: Vec3::new(0.0, 2.0, 20.0), range: 1.0 }, Reach { at: Vec3::new(60.0, 0.0, 0.0), range: 1.0 }], &v);
        assert_eq!(grid.pairs, 0);
    }

    #[test]
    fn a_full_cluster_keeps_its_first_lights_and_counts_the_rest() {
        let v = view();
        let crowd: Vec<Reach> = (0..CELL_CAP + 5).map(|_| Reach { at: Vec3::ZERO, range: 0.5 }).collect();
        let grid = build(&crowd, &v);
        assert_eq!(grid.busiest, CELL_CAP);
        assert!(grid.dropped >= 5);
        let d = (Vec3::ZERO - v.eye).dot(v.fwd);
        let mid = ((slice_of(d, v.near, v.far) * TILES_Y + TILES_Y / 2) * TILES_X + TILES_X / 2) as usize;
        assert_eq!(lights_in(&grid, mid), (0..CELL_CAP).collect::<Vec<_>>(), "the first in priority order");
    }

    #[test]
    fn slices_run_from_near_to_far() {
        assert_eq!(slice_of(0.05, 0.1, 80.0), 0);
        assert_eq!(slice_of(500.0, 0.1, 80.0), SLICES - 1);
        assert!(slice_of(2.0, 0.1, 80.0) < slice_of(20.0, 0.1, 80.0));
    }
}
