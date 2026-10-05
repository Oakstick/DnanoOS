//! Irradiance probes for the level's rooms (RP18 in the Technical Art and Rendering Plan).
//!
//! When a level is first drawn its static cubes are laid into a grid of solid cells a quarter
//! unit across (`Occupancy`), each knowing its colour and its glow. Then a probe at every tile's
//! centre, on `LAYERS` heights, looks along the six axes: for each, a fan of rays finds what it
//! hits within `REACH`. What it sees there (the surface's colour lit by the level's ambient light,
//! and by the sun or moon if that spot sees it, plus its own glow) is that direction's light; the
//! rays that find nothing see the sky. So each probe knows, per side, how much light comes back
//! off its surroundings and how much of the sky it sees: courtyards and alleys darken, lava and
//! sunlit walls light what faces them, and metal reflects its own room.
//!
//! Rays stop at solid cells, so light does not pass through walls; a probe inside a wall takes
//! its neighbours' values, so sampling near a wall does not blend in its dark inside. A broken
//! wall (`refresh`) changes the cells round it and the probes within a ray's reach, and no more.

use glam::Vec3;

use super::faces::{cover, par_map, Cover};
use super::view::{Atmosphere, Instance};

/// Cell size of the occupancy grid, in world units.
pub const CELL: f32 = 0.25;
/// Probe heights: the lowest, the spacing, and how many.
pub const Y0: f32 = -0.5;
pub const DY: f32 = 0.6;
pub const LAYERS: u32 = 6;
/// How far a probe's rays look, their step, and how many per side.
const REACH: f32 = 8.0;
const STEP: f32 = 0.2;
const RAYS: u32 = 8;
/// How far a sun ray looks for something in the way.
const SUN_REACH: f32 = 24.0;
/// The occupancy grid's height range.
const FLOOR: f32 = -3.0;
const CEILING: f32 = 8.0;

/// The six sides, in the shader's order: +x, -x, +y, -y, +z, -z.
pub const SIDES: [Vec3; 6] = [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z];

/// Solid matter by cell: 0 is empty; otherwise the colour (square-root encoded RGB in the low
/// 24 bits, so dark colours keep their steps) with the top byte set; glow kept apart (rare).
#[derive(Clone)]
struct Occupancy {
    min: Vec3,
    dims: [usize; 3],
    cells: Vec<u32>,
    glow: std::collections::HashMap<usize, Vec3>,
    /// Per cell: 0 unknown, 1 sees the sun, 2 in shadow (worked out for surface cells).
    sunlit: Vec<u8>,
}

impl Occupancy {
    fn new(width: f32, depth: f32) -> Self {
        let min = Vec3::new(-2.0, FLOOR, -2.0);
        let size = Vec3::new(width + 4.0, CEILING - FLOOR, depth + 4.0);
        let dims = [(size.x / CELL).ceil() as usize, (size.y / CELL).ceil() as usize, (size.z / CELL).ceil() as usize];
        let n = dims[0] * dims[1] * dims[2];
        Self { min, dims, cells: vec![0; n], glow: Default::default(), sunlit: vec![0; n] }
    }

    fn index(&self, p: Vec3) -> Option<usize> {
        let q = (p - self.min) / CELL;
        if q.x < 0.0 || q.y < 0.0 || q.z < 0.0 {
            return None;
        }
        let (x, y, z) = (q.x as usize, q.y as usize, q.z as usize);
        (x < self.dims[0] && y < self.dims[1] && z < self.dims[2]).then(|| (z * self.dims[1] + y) * self.dims[0] + x)
    }

    fn centre(&self, i: usize) -> Vec3 {
        let x = i % self.dims[0];
        let y = (i / self.dims[0]) % self.dims[1];
        let z = i / (self.dims[0] * self.dims[1]);
        self.min + (Vec3::new(x as f32, y as f32, z as f32) + 0.5) * CELL
    }

    fn colour(&self, i: usize) -> Vec3 {
        let c = self.cells[i];
        let ch = |s: u32| ((c >> s) & 255) as f32 / 255.0;
        Vec3::new(ch(16), ch(8), ch(0)).powf(2.0)
    }

    /// Lays a cube's bounds into the grid, within `region` (min, max) if given.
    fn lay(&mut self, c: &Instance, region: Option<(Vec3, Vec3)>) {
        if cover(c) == Cover::None {
            return;
        }
        let m = &c.m;
        let a = |i: usize| Vec3::new(m[i].abs(), m[i + 1].abs(), m[i + 2].abs());
        let half = (a(0) + a(4) + a(8)) * 0.5;
        if half.max_element() <= 0.0 {
            return;
        }
        let at = Vec3::new(m[12], m[13], m[14]);
        let (mut lo, mut hi) = (at - half, at + half);
        if let Some((rlo, rhi)) = region {
            lo = lo.max(rlo);
            hi = hi.min(rhi);
            if lo.cmpgt(hi).any() {
                return;
            }
        }
        let gl = (lo - self.min) / CELL;
        let gh = (hi - self.min) / CELL;
        let r = |v: f32, d: usize| (v.round().max(0.0) as usize).min(d);
        let (x0, x1) = (r(gl.x, self.dims[0]), r(gh.x, self.dims[0]));
        let (y0, y1) = (r(gl.y, self.dims[1]), r(gh.y, self.dims[1]));
        let (z0, z1) = (r(gl.z, self.dims[2]), r(gh.z, self.dims[2]));
        // A cube thinner than a cell still blocks the cell it sits in.
        let (x1, y1, z1) = (x1.max((x0 + 1).min(self.dims[0])), y1.max((y0 + 1).min(self.dims[1])), z1.max((z0 + 1).min(self.dims[2])));
        let col = Vec3::new(c.color[0], c.color[1], c.color[2]).max(Vec3::ZERO).min(Vec3::ONE).powf(0.5) * 255.0;
        let packed = 0xff00_0000 | ((col.x as u32) << 16) | ((col.y as u32) << 8) | col.z as u32;
        let shade = c.emissive[3];
        let glow = Vec3::new(c.emissive[0], c.emissive[1], c.emissive[2]);
        let glows = glow.max_element() > 0.01 && shade < 4.5;
        for z in z0..z1 {
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = (z * self.dims[1] + y) * self.dims[0] + x;
                    self.cells[i] = packed;
                    if glows {
                        self.glow.insert(i, glow);
                    }
                }
            }
        }
    }

    /// Every cell whose centre lies in the box `lo`..`hi`, walked by index range.
    fn cells_in(&self, lo: Vec3, hi: Vec3) -> Vec<usize> {
        let g = |v: Vec3| (v - self.min) / CELL - Vec3::splat(0.5);
        let (a, b) = (g(lo).ceil().max(Vec3::ZERO), g(hi).floor());
        let dim = |k: usize| self.dims[k] as f32 - 1.0;
        let (x0, y0, z0) = (a.x as usize, a.y as usize, a.z as usize);
        let (x1, y1, z1) = (b.x.min(dim(0)), b.y.min(dim(1)), b.z.min(dim(2)));
        if x1 < x0 as f32 || y1 < y0 as f32 || z1 < z0 as f32 {
            return Vec::new();
        }
        let mut out = Vec::new();
        for z in z0..=z1 as usize {
            for y in y0..=y1 as usize {
                for x in x0..=x1 as usize {
                    out.push((z * self.dims[1] + y) * self.dims[0] + x);
                }
            }
        }
        out
    }

    fn solid(&self, p: Vec3) -> Option<usize> {
        self.index(p).filter(|&i| self.cells[i] != 0)
    }

    /// Whether a point sees the sun (nothing solid on the way toward `sun`).
    fn sees_sun(&self, from: Vec3, sun: Vec3) -> bool {
        let mut t = CELL;
        while t < SUN_REACH {
            let p = from + sun * t;
            if p.y > CEILING {
                return true;
            }
            if self.solid(p).is_some() {
                return false;
            }
            t += CELL;
        }
        true
    }

    /// Works out which surface cells (solid, with an empty neighbour above or beside) see the sun,
    /// within `region` if given.
    fn light(&mut self, sun: Vec3, region: Option<(Vec3, Vec3)>) {
        let todo: Vec<usize> = match region {
            Some((lo, hi)) => self.cells_in(lo, hi).into_iter().filter(|&i| self.cells[i] != 0).collect(),
            None => (0..self.cells.len()).filter(|&i| self.cells[i] != 0).collect(),
        };
        let me = &*self;
        let lit = par_map(&todo, |_, &i| {
            // Leave the cell on the sun's side, then look on toward the sun.
            let from = me.centre(i) + sun * (CELL * 0.9);
            if me.solid(from).is_some_and(|j| j != i) {
                return 2u8;
            }
            if me.sees_sun(from, sun) {
                1
            } else {
                2
            }
        });
        for (i, l) in todo.into_iter().zip(lit) {
            self.sunlit[i] = l;
        }
    }
}

/// A box of probes in the volume's grid (x, layer, z), first and last inclusive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProbeBox {
    pub lo: [u32; 3],
    pub hi: [u32; 3],
}

impl ProbeBox {
    /// The whole volume of `dims` probes.
    pub fn all(dims: [u32; 3]) -> Self {
        Self { lo: [0; 3], hi: [dims[0] - 1, dims[1] - 1, dims[2] - 1] }
    }

    pub fn size(&self) -> [u32; 3] {
        [self.hi[0] - self.lo[0] + 1, self.hi[1] - self.lo[1] + 1, self.hi[2] - self.lo[2] + 1]
    }
}

/// The level's probes, and the cells they were built from.
#[derive(Clone)]
pub struct ProbeVolume {
    /// Probes along x (tiles), y (layers) and z (tiles).
    pub dims: [u32; 3],
    /// Per side (`SIDES` order), per probe (x fastest, then y, then z): the light coming back
    /// off what that side sees (linear radiance, before exposure), and the share of sky it sees.
    pub sides: [Vec<[f32; 4]>; 6],
    occ: Occupancy,
    atm: Atmosphere,
}

/// A fan of `RAYS` directions about `axis`, spread by the cosine (as light falls on a surface).
fn fan(axis: Vec3) -> Vec<Vec3> {
    let up = if axis.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    let tx = up.cross(axis).normalize();
    let ty = axis.cross(tx);
    (0..RAYS)
        .map(|i| {
            let u = (i as f32 + 0.5) / RAYS as f32;
            let v = (i.reverse_bits() as f64 / 4_294_967_296.0) as f32;
            let r = u.sqrt();
            let a = std::f32::consts::TAU * v;
            (tx * (r * a.cos()) + ty * (r * a.sin()) + axis * (1.0 - u).sqrt()).normalize()
        })
        .collect()
}

impl ProbeVolume {
    /// Builds the probes for a level `width` by `depth` tiles from its static cubes, under `atm`.
    pub fn build(cubes: &[Instance], width: f32, depth: f32, atm: &Atmosphere) -> Self {
        let mut occ = Occupancy::new(width, depth);
        for c in cubes {
            occ.lay(c, None);
        }
        occ.light(atm.sun_dir.normalize_or(Vec3::Y), None);
        let dims = [width.max(1.0) as u32, LAYERS, depth.max(1.0) as u32];
        let n = (dims[0] * dims[1] * dims[2]) as usize;
        let mut v = Self { dims, sides: std::array::from_fn(|_| vec![[0.0; 4]; n]), occ, atm: *atm };
        v.trace(None);
        v
    }

    /// Where probe `i` stands.
    pub fn position(&self, i: usize) -> Vec3 {
        let x = i % self.dims[0] as usize;
        let y = (i / self.dims[0] as usize) % self.dims[1] as usize;
        let z = i / (self.dims[0] * self.dims[1]) as usize;
        Vec3::new(x as f32, Y0 + y as f32 * DY, z as f32)
    }

    /// What one side of a probe at `p` sees.
    fn look(&self, p: Vec3, rays: &[Vec3]) -> [f32; 4] {
        let atm = &self.atm;
        let ambient = atm.ambient + (atm.hemi_sky + atm.hemi_ground) * 0.5;
        let sun = atm.sun_dir.normalize_or(Vec3::Y);
        let (mut light, mut sky) = (Vec3::ZERO, 0.0);
        'ray: for d in rays {
            let mut t = STEP;
            while t < REACH {
                let q = p + *d * t;
                if q.y > CEILING {
                    break;
                }
                if let Some(i) = self.occ.solid(q) {
                    // The surface it hit: its colour lit by the ambient light and, where it sees
                    // it, the sun or moon (on a face turned about half toward it), plus its glow.
                    let lit = if self.occ.sunlit[i] == 1 { 0.6 * sun.dot(-*d).max(0.25) } else { 0.0 };
                    let glow = self.occ.glow.get(&i).copied().unwrap_or(Vec3::ZERO);
                    light += self.occ.colour(i) * (ambient + atm.sun * lit) / std::f32::consts::PI + glow;
                    continue 'ray;
                }
                t += STEP;
            }
            sky += 1.0;
        }
        let n = rays.len() as f32;
        [light.x / n, light.y / n, light.z / n, sky / n]
    }

    /// Traces every probe, or those within `region`.
    fn trace(&mut self, region: Option<(Vec3, Vec3)>) {
        let fans: Vec<Vec<Vec3>> = SIDES.iter().map(|s| fan(*s)).collect();
        let n = (self.dims[0] * self.dims[1] * self.dims[2]) as usize;
        let todo: Vec<usize> = (0..n)
            .filter(|&i| {
                region.is_none_or(|(lo, hi)| {
                    let p = self.position(i);
                    p.cmpge(lo).all() && p.cmple(hi).all()
                })
            })
            .collect();
        let me = &*self;
        let looked: Vec<[[f32; 4]; 6]> = par_map(&todo, |_, &i| {
            let p = me.position(i);
            if me.occ.solid(p).is_some() {
                // Inside a wall: filled from its neighbours below.
                return [[f32::NAN; 4]; 6];
            }
            std::array::from_fn(|s| me.look(p, &fans[s]))
        });
        for (&i, l) in todo.iter().zip(&looked) {
            for s in 0..6 {
                self.sides[s][i] = l[s];
            }
        }
        // Probes in walls take the mean of their open neighbours (or see nothing, if none).
        let (w, h) = (self.dims[0] as i64, self.dims[1] as i64);
        for &i in &todo {
            if !self.sides[0][i][0].is_nan() {
                continue;
            }
            let (x, y, z) = (i as i64 % w, (i as i64 / w) % h, i as i64 / (w * h));
            for s in 0..6 {
                let (mut sum, mut k) = ([0.0f32; 4], 0.0);
                for (dx, dy, dz) in [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)] {
                    let (nx, ny, nz) = (x + dx, y + dy, z + dz);
                    if nx < 0 || ny < 0 || nz < 0 || nx >= w || ny >= h || nz >= self.dims[2] as i64 {
                        continue;
                    }
                    let v = self.sides[s][((nz * h + ny) * w + nx) as usize];
                    if !v[0].is_nan() {
                        for c in 0..4 {
                            sum[c] += v[c];
                        }
                        k += 1.0;
                    }
                }
                self.sides[s][i] = if k > 0.0 { sum.map(|v| v / k) } else { [0.0; 4] };
            }
        }
    }

    /// A broken wall or anything else changed inside the box `lo`..`hi`: lays its cells again from
    /// `cubes` (every cube that reaches into the box; others are left out), works out the sun there
    /// again, and traces the probes that can see it. Which probes those are.
    pub fn refresh(&mut self, cubes: &[Instance], lo: Vec3, hi: Vec3) -> Option<ProbeBox> {
        let region = (lo - Vec3::splat(CELL), hi + Vec3::splat(CELL));
        for i in self.occ.cells_in(region.0, region.1) {
            self.occ.cells[i] = 0;
            self.occ.glow.remove(&i);
            self.occ.sunlit[i] = 0;
        }
        for c in cubes {
            self.occ.lay(c, Some(region));
        }
        let sun = self.atm.sun_dir.normalize_or(Vec3::Y);
        self.occ.light(sun, Some((region.0 - Vec3::splat(2.0), region.1 + Vec3::splat(2.0))));
        let reach = Vec3::splat(REACH);
        let (a, b) = (region.0 - reach, region.1 + reach);
        self.trace(Some((a, b)));
        self.probes_in(a, b)
    }

    /// The probes (x, layer, z: first and last, inclusive) standing within `lo`..`hi`, if any.
    pub fn probes_in(&self, lo: Vec3, hi: Vec3) -> Option<ProbeBox> {
        let last = |k: usize| self.dims[k] as f32 - 1.0;
        let (x0, x1) = (lo.x.ceil().max(0.0), hi.x.floor().min(last(0)));
        let (z0, z1) = (lo.z.ceil().max(0.0), hi.z.floor().min(last(2)));
        let (y0, y1) = (((lo.y - Y0) / DY).ceil().max(0.0), ((hi.y - Y0) / DY).floor().min(last(1)));
        (x0 <= x1 && y0 <= y1 && z0 <= z1)
            .then_some(ProbeBox { lo: [x0 as u32, y0 as u32, z0 as u32], hi: [x1 as u32, y1 as u32, z1 as u32] })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::view::SHADE_LIT;
    use glam::{Mat4, Quat};

    fn slab(at: Vec3, size: Vec3, colour: Vec3) -> Instance {
        Instance::new(Mat4::from_scale_rotation_translation(size, Quat::IDENTITY, at), colour, 1.0, Vec3::ZERO, SHADE_LIT)
    }

    fn atm() -> Atmosphere {
        Atmosphere {
            sun_dir: Vec3::Y,
            sun: Vec3::splat(1.0),
            ambient: Vec3::ZERO,
            hemi_sky: Vec3::splat(0.5),
            hemi_ground: Vec3::splat(0.1),
            ..Atmosphere::forest()
        }
    }

    #[test]
    fn open_ground_sees_the_sky_above_and_the_lit_ground_below() {
        let floor = slab(Vec3::new(4.0, -0.5, 4.0), Vec3::new(12.0, 0.4, 12.0), Vec3::splat(0.5));
        let v = ProbeVolume::build(&[floor], 8.0, 8.0, &atm());
        let i = (4 * LAYERS + 1) as usize * 8 + 4; // x 4, layer 1 (y 0.1), z 4
        let (up, down) = (v.sides[2][i], v.sides[3][i]);
        assert!(up[3] > 0.95, "sky above: {up:?}");
        assert!(down[3] < 0.05 && down[0] > 0.05, "lit ground below: {down:?}");
    }

    #[test]
    fn a_wall_hides_the_sky_and_light_does_not_pass_it() {
        let floor = slab(Vec3::new(4.0, -0.5, 4.0), Vec3::new(12.0, 0.4, 12.0), Vec3::splat(0.5));
        let roof = slab(Vec3::new(4.0, 1.6, 4.0), Vec3::new(12.0, 0.4, 12.0), Vec3::splat(0.5));
        let lava = Instance::new(
            Mat4::from_scale_rotation_translation(Vec3::new(1.0, 1.8, 8.0), Quat::IDENTITY, Vec3::new(7.0, 0.6, 4.0)),
            Vec3::splat(0.2),
            1.0,
            Vec3::new(5.0, 1.0, 0.0),
            SHADE_LIT,
        );
        let wall = slab(Vec3::new(5.5, 0.5, 4.0), Vec3::new(0.5, 3.0, 12.0), Vec3::splat(0.5));
        let open = ProbeVolume::build(&[floor, roof, lava], 8.0, 8.0, &atm());
        let walled = ProbeVolume::build(&[floor, roof, lava, wall], 8.0, 8.0, &atm());
        let i = (4 * LAYERS + 1) as usize * 8 + 4;
        assert!(open.sides[2][i][3] < 0.05, "under a roof: no sky");
        assert!(
            open.sides[0][i][0] > walled.sides[0][i][0] * 2.0,
            "the glow behind the wall is hidden: {:?} {:?}",
            open.sides[0][i],
            walled.sides[0][i]
        );
    }

    #[test]
    fn refreshing_a_region_matches_building_again() {
        let floor = slab(Vec3::new(4.0, -0.5, 4.0), Vec3::new(12.0, 0.4, 12.0), Vec3::splat(0.5));
        let wall = slab(Vec3::new(5.5, 0.5, 4.0), Vec3::new(0.5, 3.0, 2.0), Vec3::splat(0.5));
        let mut v = ProbeVolume::build(&[floor, wall], 8.0, 8.0, &atm());
        let mut gone = wall;
        gone.m = (Mat4::from_translation(Vec3::new(0.0, -50.0, 0.0)) * Mat4::from_scale(Vec3::ZERO)).to_cols_array();
        let changed = v.refresh(&[floor, gone], Vec3::new(5.0, -1.0, 3.0), Vec3::new(6.0, 2.0, 5.0));
        // Every probe within a ray's reach of the box: here the whole small level.
        assert_eq!(changed, Some(ProbeBox::all(v.dims)));
        let fresh = ProbeVolume::build(&[floor], 8.0, 8.0, &atm());
        for s in 0..6 {
            for (a, b) in v.sides[s].iter().zip(&fresh.sides[s]) {
                for c in 0..4 {
                    assert!((a[c] - b[c]).abs() < 1e-4, "side {s}: {a:?} against {b:?}");
                }
            }
        }
    }
}
