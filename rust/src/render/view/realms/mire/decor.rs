//! What dresses the Grey Mire, as boxes: half-sunk cottages of fieldstone under sagging thatch,
//! the drowned chapel and its leaning bell-tower, weeping willows, the dead oak, reed beds, the
//! clockwork gears sunk in the mud, cobwebs, the tower of the Stopped Hourglass, Hel's bed Kör,
//! and Belphegor's shortcuts (rails on trestles and their cart, the ferry's rope and raft, the
//! lift's cable and cage).

use super::super::super::forge::segment;
use super::super::super::*;
use super::pal::*;

/// Wet wood, cloth, slate-dark iron and the gears' brass, as shade modes (`voxel_surface`).
pub const SHADE_WOOD: f32 = 12.0;
pub const SHADE_CLOTH: f32 = 8.0;
pub const SHADE_IRON: f32 = 6.0;
pub const SHADE_BRASS: f32 = 13.0;
pub const SHADE_WET: f32 = 14.0;
/// Moss as fur, and the giant's skin.
pub const SHADE_FUR: f32 = 7.0;
pub const SHADE_SKIN: f32 = 10.0;

/// A box at `pos`, `size`, turned `yaw` about y.
pub fn bx(out: &mut Vec<Instance>, pos: Vec3, size: Vec3, yaw: f32, color: u32, shade: f32) {
    out.push(Instance::new(
        Mat4::from_scale_rotation_translation(size, Quat::from_rotation_y(yaw), pos),
        lin(color),
        1.0,
        Vec3::ZERO,
        shade,
    ));
}

/// A box at `pos` in a colour already made linear.
pub fn bxc(out: &mut Vec<Instance>, pos: Vec3, size: Vec3, yaw: f32, color: Vec3, shade: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(size, Quat::from_rotation_y(yaw), pos), color, 1.0, Vec3::ZERO, shade));
}

/// A box under `root` (a model's own frame).
pub fn rbx(out: &mut Vec<Instance>, root: Mat4, pos: Vec3, size: Vec3, color: u32, shade: f32) {
    out.push(Instance::new(root * Mat4::from_scale_rotation_translation(size, Quat::IDENTITY, pos), lin(color), 1.0, Vec3::ZERO, shade));
}

/// A beam from `a` to `b`, `thick` across.
pub fn beam(out: &mut Vec<Instance>, a: Vec3, b: Vec3, thick: f32, color: u32, shade: f32) {
    out.push(Instance::new(segment(a, b, thick), lin(color), 1.0, Vec3::ZERO, shade));
}

fn h(x: f32, y: f32) -> f32 {
    hash(x as f64 * 1.37 + 0.3, y as f64 * 2.11 + 0.7) as f32
}

/// A clump of reeds at `at`: grey blades of uneven height, a few with brown heads, leaning.
pub fn reeds(out: &mut Vec<Instance>, at: Vec3, seed: f32, tall: f32) {
    let n = 5 + (h(seed, 1.0) * 5.0) as usize;
    for k in 0..n {
        let fk = k as f32 + seed;
        let off = Vec3::new((h(fk, 2.0) - 0.5) * 0.5, 0.0, (h(fk, 3.0) - 0.5) * 0.5);
        let len = tall * (0.45 + h(fk, 4.0) * 0.6);
        let lean = Vec3::new((h(fk, 5.0) - 0.5) * 0.25, 0.0, (h(fk, 6.0) - 0.5) * 0.25);
        let a = at + off;
        let b = a + Vec3::new(0.0, len, 0.0) + lean;
        let c = if k % 3 == 0 {
            REED_D
        } else if k % 3 == 1 {
            REED
        } else {
            REED_L
        };
        beam(out, a, b, 0.03, c, SHADE_LIT);
        if k % 4 == 1 {
            beam(out, b - (b - a) * 0.18, b + (b - a).normalize() * 0.02, 0.06, CATTAIL, SHADE_LIT);
        }
    }
}

/// A tuft of grey bog grass.
pub fn tuft(out: &mut Vec<Instance>, at: Vec3, seed: f32, color: u32) {
    for k in 0..4 {
        let fk = seed + k as f32 * 1.7;
        let a = at + Vec3::new((h(fk, 7.0) - 0.5) * 0.12, 0.0, (h(fk, 8.0) - 0.5) * 0.12);
        let b = a + Vec3::new((h(fk, 9.0) - 0.5) * 0.18, 0.12 + h(fk, 10.0) * 0.16, (h(fk, 11.0) - 0.5) * 0.18);
        beam(out, a, b, 0.025, color, SHADE_LIT);
    }
}

/// A cobweb spun across a corner: threads radiating from `at` in the plane of `u` and `v`, and
/// rings round them.
pub fn cobweb(out: &mut Vec<Instance>, at: Vec3, u: Vec3, v: Vec3, size: f32) {
    let spokes = 6;
    for k in 0..spokes {
        let a = k as f32 / (spokes - 1) as f32 * PI * 0.5;
        let dir = u * a.cos() + v * a.sin();
        beam(out, at, at + dir * size, 0.004, WEB, SHADE_LIT);
    }
    for ring in 1..4 {
        let r = size * ring as f32 / 4.0;
        for k in 0..spokes - 1 {
            let (a0, a1) = (k as f32 / (spokes - 1) as f32 * PI * 0.5, (k + 1) as f32 / (spokes - 1) as f32 * PI * 0.5);
            let p0 = at + (u * a0.cos() + v * a0.sin()) * r;
            let p1 = at + (u * a1.cos() + v * a1.sin()) * r * 0.96;
            beam(out, p0, p1, 0.0035, WEB, SHADE_LIT);
        }
    }
}

/// A half-sunk cottage over `rect`'s tiles: fieldstone walls in uneven courses, a door and a
/// broken window, a thatched roof sagging in the middle and holed, a chimney, a cobweb; it leans
/// by `lean` and has sunk a quarter into the bog.
pub fn cottage(out: &mut Vec<Instance>, x0: i32, z0: i32, x1: i32, z1: i32, lean: f32, seed: f32) {
    let (w, d) = ((x1 - x0 + 1) as f32, (z1 - z0 + 1) as f32);
    let centre = Vec3::new((x0 + x1) as f32 / 2.0, -0.28, (z0 + z1) as f32 / 2.0);
    // Its ridge runs along its long side.
    let along_x = w >= d;
    let (len, wide) = if along_x { (w - 0.15, d - 0.25) } else { (d - 0.15, w - 0.25) };
    let root = Mat4::from_translation(centre)
        * Mat4::from_rotation_y(if along_x { 0.0 } else { PI / 2.0 })
        * Mat4::from_rotation_z(lean)
        * Mat4::from_rotation_x(lean * 0.6);
    let wall_h = 1.35;
    // The walls: courses of stones, each its own box.
    let mut course = 0;
    let mut y = 0.0;
    while y < wall_h {
        let ch = 0.17 + h(seed + course as f32, 1.0) * 0.06;
        for side in 0..4 {
            let (span, fixed, horizontal) = match side {
                0 => (len, wide / 2.0, true),
                1 => (len, -wide / 2.0, true),
                2 => (wide, len / 2.0, false),
                _ => (wide, -len / 2.0, false),
            };
            let mut s = -span / 2.0 + if course % 2 == 0 { 0.0 } else { 0.18 };
            let mut k = 0;
            while s < span / 2.0 {
                let l = (0.28 + h(seed + s * 3.0, course as f32 + side as f32) * 0.3).min(span / 2.0 - s);
                if l < 0.06 {
                    break;
                }
                let mid = s + l / 2.0;
                // The door in the front wall, the window beside it.
                let door = side == 0 && mid.abs() < 0.32 && y < 0.95;
                let window = side == 0 && (mid - len * 0.28).abs() < 0.22 && (0.55..1.0).contains(&y);
                if !door && !window {
                    let c = [STONE, STONE_L, STONE_D, STONE][(h(seed + k as f32, course as f32 * 1.3) * 4.0) as usize % 4];
                    let pos = if horizontal { Vec3::new(mid, y + ch / 2.0, fixed) } else { Vec3::new(fixed, y + ch / 2.0, mid) };
                    let size = if horizontal { Vec3::new(l - 0.02, ch - 0.015, 0.24) } else { Vec3::new(0.24, ch - 0.015, l - 0.02) };
                    rbx(out, root, pos, size, c, SHADE_LIT);
                }
                s += l;
                k += 1;
            }
        }
        y += ch;
        course += 1;
    }
    // The door (rotten planks, ajar), the window (dark, a shutter hanging).
    rbx(out, root, Vec3::new(0.0, 0.47, wide / 2.0 - 0.02), Vec3::new(0.6, 0.94, 0.04), VOID, SHADE_LIT);
    rbx(out, root, Vec3::new(-0.18, 0.47, wide / 2.0 + 0.12), Vec3::new(0.32, 0.9, 0.05), PLANK_D, SHADE_WOOD);
    rbx(out, root, Vec3::new(len * 0.28, 0.78, wide / 2.0 - 0.02), Vec3::new(0.42, 0.42, 0.04), VOID, SHADE_LIT);
    out.push(Instance::new(
        root * Mat4::from_scale_rotation_translation(
            Vec3::new(0.2, 0.42, 0.04),
            Quat::from_rotation_z(0.5),
            Vec3::new(len * 0.28 + 0.32, 0.66, wide / 2.0 + 0.1),
        ),
        lin(PLANK),
        1.0,
        Vec3::ZERO,
        SHADE_WOOD,
    ));
    // The roof: thatch laid in courses up either slope, bundles of uneven length, sagging in the
    // middle, rotten through in places and grown over with moss.
    let pitch = 0.75f32;
    let half = wide / 2.0 + 0.2;
    let slope = (half * half + (half * pitch.tan()).powi(2)).sqrt();
    let rise = half * pitch.tan();
    let segs = (len / 0.34).ceil() as i32;
    let rows = 5;
    for side in [-1.0f32, 1.0] {
        let plane =
            root * Mat4::from_translation(Vec3::new(0.0, wall_h + rise / 2.0, side * half / 2.0)) * Mat4::from_rotation_x(side * pitch);
        for row in 0..rows {
            let v = (row as f32 + 0.5) / rows as f32 - 0.5;
            for k in 0..segs {
                let t = (k as f32 + 0.5) / segs as f32;
                let hk = h(seed + k as f32 + side * 31.0, row as f32);
                if hk > 0.93 && row > 0 && row < rows - 1 {
                    continue; // rotten through
                }
                let x = -len / 2.0 - 0.15 + (len + 0.3) * t;
                let sag = (PI * t).sin() * 0.14 * (1.0 - v.abs());
                let c = if hk < 0.12 { MOSS } else { [THATCH, THATCH, THATCH_D, THATCH_L][(hk * 37.0) as usize % 4] };
                let m = plane * Mat4::from_translation(Vec3::new(x + (hk - 0.5) * 0.06, -sag, v * slope * side));
                out.push(Instance::new(
                    m * Mat4::from_scale(Vec3::new((len + 0.3) / segs as f32 + 0.04, 0.14 + hk * 0.04, slope / rows as f32 + 0.05)),
                    lin(c),
                    1.0,
                    Vec3::ZERO,
                    if c == MOSS { SHADE_FUR } else { SHADE_LIT },
                ));
            }
        }
    }
    rbx(out, root, Vec3::new(0.0, wall_h + half * pitch.tan() - 0.05, 0.0), Vec3::new(len + 0.3, 0.12, 0.18), THATCH_D, SHADE_LIT);
    // A chimney of the same stone at one end.
    for k in 0..5 {
        rbx(
            out,
            root,
            Vec3::new(-len / 2.0 + 0.3, wall_h + 0.2 + k as f32 * 0.2, -0.15),
            Vec3::new(0.34, 0.19, 0.34),
            if k % 2 == 0 { STONE } else { STONE_D },
            SHADE_LIT,
        );
    }
    // A cobweb in the doorway's corner.
    let corner = root.transform_point3(Vec3::new(0.3, 0.94, wide / 2.0 + 0.02));
    let (u, v) = (root.transform_vector3(Vec3::NEG_X).normalize(), root.transform_vector3(Vec3::NEG_Y).normalize());
    cobweb(out, corner, u, v, 0.35);
}

/// The drowned chapel over `rect`'s tiles: stone walls with pointed windows, its roof fallen in to
/// bare rafters, a bell-tower at one end leaning over the pool, its bell still hanging.
pub fn chapel(out: &mut Vec<Instance>, x0: i32, z0: i32, x1: i32, z1: i32, toward: Vec3) {
    let (w, d) = ((x1 - x0 + 1) as f32, (z1 - z0 + 1) as f32);
    let centre = Vec3::new((x0 + x1) as f32 / 2.0, -0.42, (z0 + z1) as f32 / 2.0);
    let along_x = w >= d;
    let (len, wide) = if along_x { (w - 0.1, d - 0.3) } else { (d - 0.1, w - 0.3) };
    // It sinks toward the pool.
    let yaw = if along_x { 0.0 } else { PI / 2.0 };
    let root = Mat4::from_translation(centre) * Mat4::from_rotation_y(yaw) * Mat4::from_rotation_x(0.07) * Mat4::from_rotation_z(-0.05);
    let wall_h = 2.1;
    let mut y = 0.0;
    let mut course = 0;
    while y < wall_h {
        let ch = 0.24;
        for side in [-1.0f32, 1.0] {
            // The long walls, windows every metre and a half.
            let mut s = -len / 2.0;
            while s < len / 2.0 - 0.05 {
                let l = (0.4 + h(s * 2.0 + side, course as f32) * 0.3).min(len / 2.0 - s);
                let mid = s + l / 2.0;
                let win = ((mid + len / 2.0) % 1.5 - 0.75).abs() < 0.22 && (0.7..1.75).contains(&y) && mid.abs() < len / 2.0 - 0.6;
                if !win {
                    let c = [CHAPEL, CHAPEL_L, CHAPEL_D][(h(mid, course as f32 + side) * 3.0) as usize % 3];
                    rbx(out, root, Vec3::new(mid, y + ch / 2.0, side * wide / 2.0), Vec3::new(l - 0.02, ch - 0.02, 0.32), c, SHADE_LIT);
                }
                s += l;
            }
            // The gable walls.
            let gable = side * len / 2.0;
            let mut t = -wide / 2.0;
            while t < wide / 2.0 - 0.05 {
                let l = (0.38 + h(t * 3.0, course as f32 + side * 5.0) * 0.25).min(wide / 2.0 - t);
                let mid = t + l / 2.0;
                let door = side > 0.0 && mid.abs() < 0.4 && y < 1.3;
                if !door {
                    let c = [CHAPEL, CHAPEL_D, CHAPEL_L][(h(mid, course as f32 + side * 2.0) * 3.0) as usize % 3];
                    rbx(out, root, Vec3::new(gable, y + ch / 2.0, mid), Vec3::new(0.32, ch - 0.02, l - 0.02), c, SHADE_LIT);
                }
                t += l;
            }
        }
        y += ch;
        course += 1;
    }
    // The windows' pointed heads.
    for side in [-1.0f32, 1.0] {
        let mut m = -len / 2.0 + 0.75;
        while m < len / 2.0 - 0.6 {
            rbx(out, root, Vec3::new(m, 1.85, side * wide / 2.0), Vec3::new(0.3, 0.12, 0.33), CHAPEL_D, SHADE_LIT);
            rbx(out, root, Vec3::new(m, 1.96, side * wide / 2.0), Vec3::new(0.16, 0.1, 0.33), CHAPEL_D, SHADE_LIT);
            rbx(out, root, Vec3::new(m, 1.2, side * wide / 2.0), Vec3::new(0.42, 1.0, 0.04), VOID, SHADE_LIT);
            m += 1.5;
        }
    }
    rbx(out, root, Vec3::new(len / 2.0, 0.65, 0.0), Vec3::new(0.05, 1.3, 0.8), VOID, SHADE_LIT);
    // The roof fallen in: rafters left, a few slates clinging.
    let pitch = 0.8f32;
    let half = wide / 2.0 + 0.15;
    let mut x = -len / 2.0 + 0.2;
    let mut k = 0;
    while x < len / 2.0 {
        for side in [-1.0f32, 1.0] {
            let a = root.transform_point3(Vec3::new(x, wall_h, side * half));
            let b = root.transform_point3(Vec3::new(x, wall_h + half * pitch.tan(), 0.0));
            if h(x, side) > 0.2 {
                beam(out, a, b, 0.09, PLANK_D, SHADE_WOOD);
            }
            if k % 3 != 1 && h(x, side + 4.0) > 0.4 {
                let m = root
                    * Mat4::from_translation(Vec3::new(x, wall_h + half * pitch.tan() * 0.5, side * half * 0.5))
                    * Mat4::from_rotation_x(side * pitch);
                out.push(Instance::new(m * Mat4::from_scale(Vec3::new(0.42, 0.05, half * 1.15)), lin(SLATE_D), 1.0, Vec3::ZERO, SHADE_LIT));
            }
        }
        x += 0.5;
        k += 1;
    }
    // The bell-tower at the west end, leaning out over the pool.
    let base = root.transform_point3(Vec3::new(-len / 2.0 + 0.55, 0.0, 0.0));
    let lean_dir = (toward - base).normalize_or(Vec3::Z);
    let tower = Mat4::from_translation(base) * Mat4::from_quat(Quat::from_rotation_arc(Vec3::Y, (Vec3::Y + lean_dir * 0.12).normalize()));
    for k in 0..14 {
        let y = k as f32 * 0.25;
        for (dx, dz) in [(-0.4f32, 0.0f32), (0.4, 0.0), (0.0, -0.4), (0.0, 0.4)] {
            // The belfry's openings.
            if (10..=11).contains(&k) {
                continue;
            }
            let size = if dx != 0.0 { Vec3::new(0.2, 0.24, 1.0) } else { Vec3::new(1.0, 0.24, 0.2) };
            rbx(
                out,
                tower,
                Vec3::new(dx, y + 0.12, dz),
                size,
                if (k + (dx * 10.0) as i32) % 3 == 0 { CHAPEL_L } else { CHAPEL },
                SHADE_LIT,
            );
        }
    }
    for (dx, dz) in [(-0.4f32, -0.4f32), (0.4, -0.4), (-0.4, 0.4), (0.4, 0.4)] {
        rbx(out, tower, Vec3::new(dx, 2.75, dz), Vec3::new(0.2, 0.52, 0.2), CHAPEL_D, SHADE_LIT);
    }
    // The spire, and the bell in the belfry.
    for k in 0..5 {
        let s = 1.0 - k as f32 * 0.19;
        rbx(out, tower, Vec3::new(0.0, 3.6 + k as f32 * 0.22, 0.0), Vec3::new(s, 0.22, s), SLATE_D, SLATE_SHADE);
    }
    rbx(out, tower, Vec3::new(0.0, 2.9, 0.0), Vec3::new(0.06, 0.3, 0.06), IRON, SHADE_IRON);
    for k in 0..4 {
        let s = 0.22 + k as f32 * 0.07;
        rbx(out, tower, Vec3::new(0.0, 2.75 - k as f32 * 0.08, 0.0), Vec3::new(s, 0.08, s), BRONZE, SHADE_BRASS);
    }
    let web = tower.transform_point3(Vec3::new(0.3, 3.05, 0.42));
    cobweb(out, web, tower.transform_vector3(Vec3::NEG_X).normalize(), tower.transform_vector3(Vec3::NEG_Y).normalize(), 0.3);
}

/// A slate as a voxel material: matte (the plain lit mode).
pub const SLATE_SHADE: f32 = SHADE_LIT;

/// A weeping willow at `at`: a gnarled trunk, and its crown's grey-green fronds hanging to the water.
pub fn willow(out: &mut Vec<Instance>, at: Vec3, seed: f32) {
    let lean = Vec3::new((h(seed, 1.0) - 0.5) * 0.5, 1.0, (h(seed, 2.0) - 0.5) * 0.5).normalize();
    let top = at + lean * 1.7;
    beam(out, at - Vec3::Y * 0.3, top, 0.34, BARK, SHADE_WOOD);
    beam(out, at + lean * 0.5, at + lean * 0.5 + Vec3::new(0.3, 0.5, 0.1), 0.16, BARK_D, SHADE_WOOD);
    for k in 0..5 {
        let a = k as f32 / 5.0 * TAU + seed;
        let arm = top + Vec3::new(a.cos() * 0.7, 0.35, a.sin() * 0.7);
        beam(out, top, arm, 0.12, BARK, SHADE_WOOD);
    }
    // The crown: clumps of grey-green leaves heaped into a ragged dome, and the long fronds
    // weeping from all round it to the water.
    for k in 0..14 {
        let fk = seed + k as f32 * 1.9;
        let a = fk * 2.4;
        let r = if k < 3 { 0.2 } else { 0.45 + h(fk, 5.0) * 0.6 };
        let up = 0.3 + (1.0 - r) * 0.45 + h(fk, 6.0) * 0.15;
        let size = 0.55 + h(fk, 7.0) * 0.35;
        let c = [LEAF, LEAF_L, LEAF_D, LEAF][(h(fk, 8.0) * 4.0) as usize % 4];
        bx(out, top + Vec3::new(a.cos() * r, up, a.sin() * r), Vec3::new(size, size * 0.5, size * 0.9), a, c, SHADE_FUR);
    }
    let n = 30;
    for k in 0..n {
        let a = k as f32 / n as f32 * TAU + seed * 0.7;
        let r = 0.85 + h(seed + k as f32, 3.0) * 0.45;
        let from = top + Vec3::new(a.cos() * r, 0.45 - (r - 0.85) * 0.4, a.sin() * r);
        let len = 0.9 + h(seed + k as f32, 4.0) * 1.1;
        let mid = from + Vec3::new(a.cos() * 0.18, -len * 0.5, a.sin() * 0.18);
        let to = mid + Vec3::new(a.cos() * 0.05, -len * 0.5, a.sin() * 0.05);
        let c = if k % 3 == 0 { LEAF_L } else { LEAF_D };
        beam(out, from, mid, 0.06, c, SHADE_LIT);
        beam(out, mid, to, 0.045, c, SHADE_LIT);
    }
    let web = top + Vec3::new(0.2, -0.3, 0.2);
    cobweb(out, web, Vec3::new(-0.7, 0.0, 0.7).normalize(), Vec3::NEG_Y, 0.4);
}

/// The dead oak: a split black trunk and bare twisted limbs.
pub fn oak(out: &mut Vec<Instance>, at: Vec3) {
    let top = at + Vec3::new(0.1, 1.6, -0.05);
    beam(out, at - Vec3::Y * 0.3, top, 0.48, BARK_D, SHADE_WOOD);
    let limbs = [(0.3f32, 1.2f32, 0.9f32), (2.1, 1.0, 1.0), (3.6, 1.3, 0.8), (5.0, 0.9, 1.1)];
    for (k, &(a, rise, len)) in limbs.iter().enumerate() {
        let mid = top + Vec3::new(a.cos() * len * 0.5, rise * 0.4, a.sin() * len * 0.5) - Vec3::Y * 0.3;
        let end = mid + Vec3::new(a.cos() * len * 0.7, rise * 0.6, a.sin() * len * 0.7);
        beam(out, top - Vec3::Y * 0.3, mid, 0.2, BARK_D, SHADE_WOOD);
        beam(out, mid, end, 0.11, BARK, SHADE_WOOD);
        let twig = end + Vec3::new((a + 1.2).cos() * 0.4, 0.3, (a + 1.2).sin() * 0.4);
        beam(out, end, twig, 0.05, BARK, SHADE_WOOD);
        if k == 1 {
            cobweb(out, mid, (end - mid).normalize(), Vec3::NEG_Y, 0.45);
        }
    }
    // A hanged lantern, long gone out.
    let hook = top + Vec3::new(0.9, 0.4, 0.4);
    beam(out, hook, hook - Vec3::Y * 0.45, 0.012, ROPE, SHADE_LIT);
    bx(out, hook - Vec3::Y * 0.55, Vec3::new(0.14, 0.18, 0.14), 0.3, IRON, SHADE_IRON);
}

/// A great clockwork gear half sunk in the mud, tilted: rim, teeth, spokes, hub.
pub fn gear(out: &mut Vec<Instance>, at: Vec3, seed: f32) {
    let r = 0.5 + h(seed, 1.0) * 0.35;
    let root = Mat4::from_translation(at + Vec3::new(0.0, r * 0.25, 0.0))
        * Mat4::from_rotation_y(seed * 3.0)
        * Mat4::from_rotation_x(1.1 + h(seed, 2.0) * 0.3);
    let teeth = 14;
    for k in 0..teeth {
        let a = k as f32 / teeth as f32 * TAU;
        let m = root * Mat4::from_rotation_z(a);
        let c = if k % 3 == 0 { RUST } else { BRASS };
        out.push(Instance::new(
            m * Mat4::from_scale_rotation_translation(Vec3::new(r * 0.5, 0.1, 0.1), Quat::IDENTITY, Vec3::new(0.0, r, 0.0)),
            lin(c),
            1.0,
            Vec3::ZERO,
            SHADE_BRASS,
        ));
        out.push(Instance::new(
            m * Mat4::from_scale_rotation_translation(Vec3::new(0.14, 0.14, 0.09), Quat::IDENTITY, Vec3::new(0.0, r + 0.1, 0.0)),
            lin(BRASS_D),
            1.0,
            Vec3::ZERO,
            SHADE_BRASS,
        ));
        if k % 3 == 0 {
            out.push(Instance::new(
                m * Mat4::from_scale_rotation_translation(Vec3::new(0.06, r, 0.06), Quat::IDENTITY, Vec3::new(0.0, r / 2.0, 0.0)),
                lin(RUST),
                1.0,
                Vec3::ZERO,
                SHADE_BRASS,
            ));
        }
    }
    out.push(Instance::new(root * Mat4::from_scale(Vec3::new(0.2, 0.2, 0.16)), lin(BRASS_D), 1.0, Vec3::ZERO, SHADE_BRASS));
}

/// The Stopped Hourglass's tower, its foot on `base` (the tower's 3 by 3 tiles' middle): courses
/// of slate stepping in as it rises, a gallery under the hourglass, its frame's lower plate. (The
/// hourglass itself, which turns over, is drawn each frame: `HOURGLASS_Y`.)
pub fn tower(out: &mut Vec<Instance>, base: Vec3) {
    let courses = 18;
    for k in 0..courses {
        let y = k as f32 * 0.28;
        let w = 2.8 - (k as f32 / courses as f32) * 0.9 - if k < 2 { 0.0 } else { 0.15 };
        let dz = -0.25;
        // Slates laid in courses, the odd one standing proud.
        let c = if k % 2 == 0 { SLATE } else { SLATE_L };
        bx(out, base + Vec3::new(0.0, y + 0.14, dz), Vec3::new(w, 0.26, w * 0.86), 0.0, c, SLATE_SHADE);
        for j in 0..3 {
            let hj = h(k as f32, j as f32);
            if hj > 0.55 {
                let along = (hj - 0.75) * w * 1.6;
                bx(out, base + Vec3::new(along, y + 0.14, dz + w * 0.43 + 0.02), Vec3::new(0.32, 0.22, 0.05), 0.0, SLATE_L, SLATE_SHADE);
            }
        }
        if k % 4 == 3 {
            bx(out, base + Vec3::new(0.0, y + 0.27, dz), Vec3::new(w + 0.12, 0.05, w * 0.86 + 0.12), 0.0, SLATE_D, SLATE_SHADE);
        }
        // Narrow dark windows up its face, and smoked glass in them.
        if k % 5 == 3 && k > 3 {
            for s in [-1.0f32, 1.0] {
                bx(
                    out,
                    base + Vec3::new(s * w * 0.22, y + 0.1, dz + w * 0.43 + 0.01),
                    Vec3::new(0.16, 0.42, 0.04),
                    0.0,
                    GLASS_D,
                    SHADE_LIT,
                );
            }
        }
    }
    let top = courses as f32 * 0.28;
    // The gallery: a broad slab, corbels under it, a parapet of slate teeth.
    bx(out, base + Vec3::new(0.0, top + 0.08, -0.25), Vec3::new(2.6, 0.16, 2.4), 0.0, SLATE_D, SLATE_SHADE);
    for k in 0..8 {
        let a = k as f32 / 8.0 * TAU;
        bx(out, base + Vec3::new(a.cos() * 1.05, top - 0.12, -0.25 + a.sin() * 0.95), Vec3::new(0.22, 0.28, 0.22), a, SLATE, SLATE_SHADE);
        bx(out, base + Vec3::new(a.cos() * 1.2, top + 0.3, -0.25 + a.sin() * 1.1), Vec3::new(0.26, 0.3, 0.26), a, SLATE_L, SLATE_SHADE);
    }
    // Cobwebs in the gallery's corners.
    cobweb(out, base + Vec3::new(1.1, top - 0.05, 0.85), Vec3::NEG_X, Vec3::NEG_Y, 0.5);
    cobweb(out, base + Vec3::new(-1.1, top - 0.05, 0.85), Vec3::X, Vec3::NEG_Y, 0.45);
}

/// The hourglass's middle stands this high over the tower's foot.
pub const HOURGLASS_Y: f32 = 18.0 * 0.28 + 1.55;

/// Kör, Hel's sick-bed, on its threshold-stone (its foot at `root`'s origin, its head toward -z):
/// black posts, a sagging mattress of grey linen, bolsters, a coverlet, a canopy. (Its gleaming
/// curtains sway, drawn each frame.)
pub fn bed(out: &mut Vec<Instance>, root: Mat4) {
    // The threshold-stone over the bog hole.
    rbx(out, root, Vec3::new(0.0, -0.05, 0.0), Vec3::new(1.35, 0.16, 1.5), SLATE_D, SLATE_SHADE);
    rbx(out, root, Vec3::new(0.0, 0.035, 0.0), Vec3::new(1.25, 0.02, 1.4), SLATE, SLATE_SHADE);
    for (x, z) in [(-0.45f32, -0.55f32), (0.45, -0.55), (-0.45, 0.55), (0.45, 0.55)] {
        rbx(out, root, Vec3::new(x, 0.75, z), Vec3::new(0.08, 1.5, 0.08), BEDWOOD, SHADE_WOOD);
        rbx(out, root, Vec3::new(x, 1.54, z), Vec3::new(0.12, 0.08, 0.12), BEDWOOD_L, SHADE_WOOD);
        rbx(out, root, Vec3::new(x, 0.05, z), Vec3::new(0.14, 0.1, 0.14), BEDWOOD_L, SHADE_WOOD);
    }
    // The frame and the headboard (carved with a skull), the footboard.
    rbx(out, root, Vec3::new(0.0, 0.32, 0.0), Vec3::new(0.86, 0.12, 1.1), BEDWOOD, SHADE_WOOD);
    rbx(out, root, Vec3::new(0.0, 0.62, -0.55), Vec3::new(0.86, 0.6, 0.06), BEDWOOD, SHADE_WOOD);
    rbx(out, root, Vec3::new(0.0, 0.78, -0.51), Vec3::new(0.16, 0.16, 0.04), BONE, SHADE_LIT);
    rbx(out, root, Vec3::new(0.0, 0.48, 0.55), Vec3::new(0.86, 0.3, 0.06), BEDWOOD, SHADE_WOOD);
    // The mattress, sagging; the bolsters; the coverlet turned back.
    rbx(out, root, Vec3::new(0.0, 0.44, 0.0), Vec3::new(0.8, 0.12, 1.04), LINEN_D, SHADE_CLOTH);
    rbx(out, root, Vec3::new(0.0, 0.47, 0.05), Vec3::new(0.7, 0.1, 0.5), LINEN_D, SHADE_CLOTH);
    for x in [-0.2f32, 0.2] {
        rbx(out, root, Vec3::new(x, 0.55, -0.4), Vec3::new(0.34, 0.12, 0.2), LINEN, SHADE_CLOTH);
    }
    rbx(out, root, Vec3::new(0.0, 0.53, 0.18), Vec3::new(0.84, 0.06, 0.7), COVER, SHADE_CLOTH);
    rbx(out, root, Vec3::new(0.0, 0.57, -0.14), Vec3::new(0.84, 0.05, 0.12), COVER_L, SHADE_CLOTH);
    // The canopy.
    rbx(out, root, Vec3::new(0.0, 1.6, 0.0), Vec3::new(1.0, 0.05, 1.2), CURTAIN_D, SHADE_CLOTH);
    for (x, z, sx, sz) in [(0.0f32, -0.6f32, 1.0f32, 0.06f32), (0.0, 0.6, 1.0, 0.06), (-0.5, 0.0, 0.06, 1.2), (0.5, 0.0, 0.06, 1.2)] {
        rbx(out, root, Vec3::new(x, 1.55, z), Vec3::new(sx, 0.1, sz), CURTAIN_D, SHADE_CLOTH);
    }
    // Cobwebs from the posts.
    let a = root.transform_point3(Vec3::new(0.45, 1.5, 0.55));
    cobweb(out, a, root.transform_vector3(Vec3::NEG_X), root.transform_vector3(Vec3::NEG_Y), 0.3);
}

/// A shortcut's way, step by step: the rail cart's rails on trestles, the ferry's rope on posts,
/// the lift's cable on tall poles; and its station.
pub fn ride_way(out: &mut Vec<Instance>, kind: crate::logic::realms::mire::RideKind, way: &[Vec3], stop: usize) {
    use crate::logic::realms::mire::RideKind;
    let n = way.len();
    for i in 0..n {
        let p = way[i];
        let next = way[(i + 1).min(n - 1)];
        let prev = way[i.saturating_sub(1)];
        let dir = (next - prev).normalize_or(Vec3::X);
        let yaw = dir.x.atan2(dir.z);
        let across = Vec3::new(dir.z, 0.0, -dir.x);
        match kind {
            RideKind::Cart => {
                if i + 1 < n {
                    for s in [-0.28f32, 0.28] {
                        beam(out, p + across * s + Vec3::Y * 0.06, next + across * s + Vec3::Y * 0.06, 0.05, IRON, SHADE_IRON);
                    }
                }
                bx(out, p + Vec3::Y * 0.0, Vec3::new(0.85, 0.07, 0.16), yaw + PI / 2.0, PLANK_D, SHADE_WOOD);
                if i > 0 && i + 1 < n && i != stop {
                    for s in [-0.36f32, 0.36] {
                        beam(out, p + across * s - Vec3::Y * 0.9, p + across * s, 0.09, PLANK_D, SHADE_WOOD);
                    }
                    beam(out, p + across * 0.36 - Vec3::Y * 0.7, p - across * 0.36 - Vec3::Y * 0.1, 0.05, PLANK_D, SHADE_WOOD);
                }
            }
            RideKind::Ferry => {
                if i % 2 == 0 || i == n - 1 {
                    beam(out, p + across * 0.5 - Vec3::Y * 0.8, p + across * 0.5 + Vec3::Y * 1.0, 0.1, PLANK_D, SHADE_WOOD);
                }
                if i + 1 < n {
                    beam(out, p + across * 0.5 + Vec3::Y * 0.92, next + across * 0.5 + Vec3::Y * 0.9, 0.025, ROPE, SHADE_LIT);
                }
            }
            RideKind::Lift => {
                if i % 3 == 0 || i == n - 1 || i == stop {
                    beam(out, p + across * 0.55 - Vec3::Y * 0.8, p + across * 0.55 + Vec3::Y * 2.6, 0.14, IRON, SHADE_IRON);
                    bx(out, p + across * 0.35 + Vec3::Y * 2.55, Vec3::new(0.5, 0.08, 0.1), yaw + PI / 2.0, IRON, SHADE_IRON);
                }
                if i + 1 < n {
                    beam(out, p + Vec3::Y * 2.5, next + Vec3::Y * 2.5, 0.03, IRON, SHADE_IRON);
                }
            }
        }
    }
    // The station: a jetty of planks, a post with a lamp gone out, a painted board.
    let (p, q) = (way[0], way[1.min(n - 1)]);
    let dir = (q - p).normalize_or(Vec3::X);
    let back = p - dir * 0.3;
    let yaw = dir.x.atan2(dir.z);
    bx(out, back + Vec3::Y * 0.04, Vec3::new(1.0, 0.08, 0.9), yaw, PLANK, SHADE_WOOD);
    let side = Vec3::new(dir.z, 0.0, -dir.x) * 0.6;
    beam(out, back + side, back + side + Vec3::Y * 1.6, 0.09, PLANK_D, SHADE_WOOD);
    bx(out, back + side + Vec3::new(0.0, 1.35, 0.0), Vec3::new(0.7, 0.32, 0.05), yaw, SIGN, SHADE_WOOD);
    bx(out, back + side + Vec3::new(0.0, 1.65, 0.0), Vec3::new(0.14, 0.18, 0.14), 0.0, IRON, SHADE_IRON);
    match kind {
        RideKind::Lift => {
            // A clockwork gantry: a frame and its great gear.
            for s in [-0.5f32, 0.5] {
                beam(out, back + side * s * 2.0 - dir * 0.3, back + side * s * 2.0 - dir * 0.3 + Vec3::Y * 2.7, 0.12, IRON, SHADE_IRON);
            }
            gear(out, back - dir * 0.5 + Vec3::Y * 1.2, 7.0);
        }
        RideKind::Ferry => {
            bx(out, back - dir * 0.2 + Vec3::Y * 0.7, Vec3::new(0.3, 0.6, 0.3), yaw, PLANK_D, SHADE_WOOD);
            bx(out, back - dir * 0.2 + Vec3::Y * 1.05, Vec3::new(0.16, 0.16, 0.16), yaw, BRONZE, SHADE_BRASS);
        }
        RideKind::Cart => {
            bx(out, back - dir * 0.15 + Vec3::Y * 0.25, Vec3::new(0.9, 0.3, 0.2), yaw, PLANK_D, SHADE_WOOD);
        }
    }
    let web = back + side + Vec3::Y * 1.2;
    cobweb(out, web, -side.normalize(), Vec3::NEG_Y, 0.3);
}
