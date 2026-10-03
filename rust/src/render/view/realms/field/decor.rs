//! The Burning Field's props, built of boxes about a root (a tile's centre at the ground, unless
//! said otherwise): thickets of broken blades, charred palisades, siege towers, war wagons, torn
//! banners, the litter of the field (bones, helmets, shields, arrows, smouldering wreckage, dead
//! horses), and the herald, the gallows-tree and its horn, the truce stone, Amon's great banner
//! and the ghost riders of the storm.

use super::*;
use crate::render::characters::{mat, Sculpt};

pub(super) mod pal {
    pub const EARTH: [u32; 4] = [0x54402f, 0x4e3c2d, 0x584232, 0x4c3a2c];
    pub const EARTH_D: u32 = 0x2e2018;
    pub const ASH: u32 = 0x6a625e;
    pub const ASH_D: u32 = 0x4a4442;
    pub const CHAR: u32 = 0x1c1410;
    pub const MUD: u32 = 0x4a1410;
    pub const MUD_L: u32 = 0x6a1c16;
    pub const OBSIDIAN: u32 = 0x1a1220;
    pub const OBSIDIAN_L: u32 = 0x3a2a4a;
    pub const STEEL: u32 = 0xa4aab2;
    pub const STEEL_D: u32 = 0x6a7078;
    pub const RUST: u32 = 0x7a4a2a;
    pub const WOOD: u32 = 0x4a3424;
    pub const WOOD_L: u32 = 0x6a4a30;
    pub const WOOD_C: u32 = 0x2a1e16;
    pub const ROPE: u32 = 0x8a6a42;
    pub const RED: u32 = 0xb0201a;
    pub const RED_D: u32 = 0x7a1410;
    pub const BLACK: u32 = 0x161010;
    pub const BONE: u32 = 0xd8ccb0;
    pub const BONE_D: u32 = 0xa89878;
    pub const CREAM: u32 = 0xd8c8a0;
    pub const EMBER: u32 = 0xff6a20;
    pub const LAVA_DEEP: u32 = 0x8a1e06;
    pub const LAVA: u32 = 0xff4a0a;
    pub const LAVA_HOT: u32 = 0xffb040;
    pub const CRUST: u32 = 0x2a120c;
    pub const STONE: u32 = 0xc8c2b6;
    pub const STONE_D: u32 = 0x948c80;
    pub const GOLD: u32 = 0xe8b84a;
    pub const IVORY: u32 = 0xeee2c4;
    pub const HORSE: u32 = 0x4a3020;
}
use pal::*;

/// The voxel materials' shading modes (`vault_decor::shade_of`), for plain boxes.
pub(super) const SH_METAL: f32 = 6.0;
pub(super) const SH_CLOTH: f32 = 8.0;
pub(super) const SH_WOOD: f32 = 12.0;

/// A box at `at` (its centre) under `root`, turned by `rot`.
#[allow(clippy::too_many_arguments)]
pub(super) fn bx(out: &mut Vec<Instance>, root: Mat4, at: Vec3, size: Vec3, rot: Quat, color: Vec3, glow: Vec3, shade: f32) {
    out.push(Instance::new(root * Mat4::from_scale_rotation_translation(size, rot, at), color, 1.0, glow, shade));
}

/// A plain lit box, unturned.
pub(super) fn plain(out: &mut Vec<Instance>, root: Mat4, at: Vec3, size: Vec3, color: u32) {
    bx(out, root, at, size, Quat::IDENTITY, lin(color), Vec3::ZERO, SHADE_LIT);
}

/// The `k`-th draw, 0 to 1, from `seed`.
pub(super) fn rnd(seed: f64, k: u32) -> f32 {
    hash(seed * 1.31 + k as f64 * 7.77 + 0.5, seed * 0.67 - k as f64 * 3.19 + 11.0) as f32
}

/// A stick of `len` along its own y from `base`, tilted by `rot` (a post, a shaft, a blade).
#[allow(clippy::too_many_arguments)]
fn stick(
    out: &mut Vec<Instance>,
    root: Mat4,
    base: Vec3,
    rot: Quat,
    from: f32,
    len: f32,
    w: f32,
    d: f32,
    color: Vec3,
    glow: Vec3,
    shade: f32,
) {
    let at = base + rot * Vec3::new(0.0, from + len / 2.0, 0.0);
    bx(out, root, at, Vec3::new(w, len, d), rot, color, glow, shade);
}

/// A wheel of radius `r` standing in the plane across x at `at` (an octagon of felloes, spokes, a hub).
fn wheel(out: &mut Vec<Instance>, root: Mat4, at: Vec3, r: f32, yaw: f32, broken: bool) {
    let q = Quat::from_rotation_y(yaw);
    let wood = lin(WOOD) * 0.8;
    for k in 0..8 {
        if broken && k % 3 == 1 {
            continue;
        }
        let a = k as f32 * TAU / 8.0;
        let rim = q * Vec3::new(0.0, a.sin() * r, a.cos() * r);
        let rot = q * Quat::from_rotation_x(-a);
        bx(out, root, at + rim, Vec3::new(0.06, 0.08, r * 0.8), rot, wood, Vec3::ZERO, SH_WOOD);
        if k % 2 == 0 {
            let spoke = q * Vec3::new(0.0, a.sin() * r * 0.5, a.cos() * r * 0.5);
            bx(out, root, at + spoke, Vec3::new(0.04, 0.04, r), rot * Quat::from_rotation_x(PI / 2.0), lin(WOOD_C), Vec3::ZERO, SH_WOOD);
        }
    }
    bx(out, root, at, Vec3::new(0.1, 0.12, 0.12), q, lin(STEEL_D), Vec3::ZERO, SH_METAL);
}

// ------------------------------------------------------------------ the blocks

/// A field of blades on one tile: swords, spears and axes planted every way in churned earth, a
/// shield or two fallen against them.
pub(super) fn blades(out: &mut Vec<Instance>, root: Mat4, seed: f64) {
    bx(
        out,
        root,
        Vec3::new(0.0, 0.03, 0.0),
        Vec3::new(0.9, 0.07, 0.9),
        Quat::from_rotation_y(rnd(seed, 0) * 0.6),
        lin(EARTH_D),
        Vec3::ZERO,
        SHADE_LIT,
    );
    let n = 7 + (rnd(seed, 1) * 4.0) as u32;
    for k in 0..n {
        let r = |j: u32| rnd(seed, 10 + k * 7 + j);
        let base = Vec3::new((r(0) - 0.5) * 0.78, 0.0, (r(1) - 0.5) * 0.78);
        let rot = Quat::from_euler(EulerRot::XYZ, (r(2) - 0.5) * 0.75, r(3) * TAU, (r(4) - 0.5) * 0.75);
        let rust = r(5);
        let steel = lin(STEEL).lerp(lin(RUST), (rust * 1.3 - 0.2).clamp(0.0, 1.0));
        let kind = r(6);
        if kind < 0.5 {
            // A sword, point down; one in three snapped short.
            let broken = r(0) < 0.33;
            let len = if broken { 0.25 + r(1) * 0.2 } else { 0.55 + r(2) * 0.25 };
            stick(out, root, base, rot, -0.06, len, 0.06, 0.014, steel, Vec3::ZERO, SH_METAL);
            if !broken {
                stick(out, root, base, rot, len - 0.06, 0.035, 0.22, 0.04, lin(STEEL_D), Vec3::ZERO, SH_METAL);
                stick(out, root, base, rot, len - 0.03, 0.15, 0.032, 0.032, lin(WOOD_C), Vec3::ZERO, SH_WOOD);
                stick(
                    out,
                    root,
                    base,
                    rot,
                    len + 0.12,
                    0.05,
                    0.055,
                    0.055,
                    lin(if rust > 0.6 { RUST } else { GOLD }),
                    Vec3::ZERO,
                    SH_METAL,
                );
            }
        } else if kind < 0.85 {
            // A spear, its head up, or its shaft splintered.
            let snapped = r(3) < 0.35;
            let len = if snapped { 0.5 + r(4) * 0.4 } else { 1.0 + r(4) * 0.45 };
            stick(out, root, base, rot, -0.1, len, 0.035, 0.035, lin(if r(5) < 0.5 { WOOD } else { WOOD_C }), Vec3::ZERO, SH_WOOD);
            if snapped {
                stick(out, root, base, rot * Quat::from_rotation_z(0.4), len - 0.12, 0.12, 0.02, 0.02, lin(WOOD_L), Vec3::ZERO, SH_WOOD);
            } else {
                stick(out, root, base, rot, len - 0.12, 0.2, 0.06, 0.016, steel, Vec3::ZERO, SH_METAL);
                if r(0) < 0.4 {
                    // A rag of a pennant under its head.
                    let p = base + rot * Vec3::new(0.08, len - 0.3, 0.0);
                    bx(out, root, p, Vec3::new(0.16, 0.12, 0.012), rot, lin(RED_D), Vec3::ZERO, SH_CLOTH);
                }
            }
        } else {
            // An axe, its haft up.
            stick(out, root, base, rot, -0.1, 0.8, 0.04, 0.04, lin(WOOD), Vec3::ZERO, SH_WOOD);
            let head = base + rot * Vec3::new(0.08, 0.62, 0.0);
            bx(out, root, head, Vec3::new(0.18, 0.13, 0.025), rot, steel, Vec3::ZERO, SH_METAL);
        }
    }
    if rnd(seed, 2) < 0.55 {
        let a = rnd(seed, 3) * TAU;
        let at = Vec3::new(a.cos() * 0.3, 0.17, a.sin() * 0.3);
        shield(out, root, at, Quat::from_rotation_y(a) * Quat::from_rotation_x(-1.1), seed + 3.0);
    }
}

/// A round shield of planks, painted and charred, facing its own +z, `at` its middle.
pub(super) fn shield(out: &mut Vec<Instance>, root: Mat4, at: Vec3, rot: Quat, seed: f64) {
    let paint = [RED, CREAM, RED_D, BLACK][(rnd(seed, 0) * 4.0) as usize % 4];
    for k in 0..2 {
        let q = rot * Quat::from_rotation_z(k as f32 * PI / 4.0);
        bx(out, root, at, Vec3::new(0.36, 0.36, 0.03), q, lin(paint) * (0.7 + rnd(seed, 1 + k) * 0.3), Vec3::ZERO, SH_WOOD);
    }
    bx(out, root, at + rot * Vec3::new(0.0, 0.0, 0.025), Vec3::new(0.1, 0.1, 0.04), rot, lin(STEEL_D), Vec3::ZERO, SH_METAL);
    // A scorched half.
    bx(out, root, at + rot * Vec3::new(0.08, -0.06, 0.017), Vec3::new(0.18, 0.2, 0.004), rot, lin(CHAR), Vec3::ZERO, SHADE_LIT);
}

/// A run of charred palisade on one tile: three sharpened stakes, lashed to a beam, some tips
/// still glowing. `along_x`: the run goes west to east.
pub(super) fn palisade(out: &mut Vec<Instance>, root: Mat4, seed: f64, along_x: bool) {
    let dir = if along_x { Vec3::X } else { Vec3::Z };
    for k in 0..3 {
        let r = |j: u32| rnd(seed, k * 5 + j);
        let base = dir * ((k as f32 - 1.0) * 0.33 + (r(0) - 0.5) * 0.06);
        let rot = Quat::from_euler(EulerRot::XYZ, (r(1) - 0.5) * 0.18, r(2) * 1.2, (r(3) - 0.5) * 0.18);
        let tall = 0.95 + r(4) * 0.4;
        let wood = lin(if r(2) < 0.5 { WOOD_C } else { WOOD }) * (0.75 + r(3) * 0.3);
        stick(out, root, base, rot, -0.05, tall, 0.17, 0.17, wood, Vec3::ZERO, SH_WOOD);
        // The point: a smaller block turned on its corner.
        let tip = base + rot * Vec3::new(0.0, tall + 0.04, 0.0);
        bx(out, root, tip, Vec3::new(0.11, 0.16, 0.11), rot * Quat::from_rotation_y(PI / 4.0), wood * 0.8, Vec3::ZERO, SH_WOOD);
        if r(0) > 0.6 {
            bx(
                out,
                root,
                tip + Vec3::new(0.0, 0.07, 0.0),
                Vec3::new(0.05, 0.05, 0.05),
                Quat::IDENTITY,
                lin(EMBER),
                lin(EMBER) * 1.6,
                SHADE_LIT,
            );
        }
        // A bark streak on the lit side.
        stick(out, root, base + rot * Vec3::new(0.06, 0.0, 0.07), rot, 0.2, tall * 0.5, 0.04, 0.04, lin(WOOD_L) * 0.8, Vec3::ZERO, SH_WOOD);
    }
    let beam = if along_x { Vec3::new(1.0, 0.07, 0.08) } else { Vec3::new(0.08, 0.07, 1.0) };
    let side = if along_x { Vec3::new(0.0, 0.0, 0.11) } else { Vec3::new(0.11, 0.0, 0.0) };
    plain(out, root, Vec3::new(0.0, 0.55, 0.0) + side, beam, WOOD);
    for k in 0..3 {
        let at = dir * ((k as f32 - 1.0) * 0.33) + side * 1.1 + Vec3::new(0.0, 0.55, 0.0);
        plain(out, root, at, Vec3::new(0.09, 0.1, 0.09), ROPE);
    }
}

/// A siege tower on two by two tiles (the root at the middle of the four), leaning, burnt through
/// in places: four posts, three storeys of planks, boarded sides with holes, a drawbridge half
/// down, a torn pennant, and its great wheels.
pub(super) fn tower(out: &mut Vec<Instance>, root: Mat4, seed: f64) {
    let lean = Quat::from_euler(EulerRot::XYZ, (rnd(seed, 0) - 0.5) * 0.16, rnd(seed, 1) * 0.4 - 0.2, (rnd(seed, 2) - 0.5) * 0.16);
    let r = root * Mat4::from_quat(lean);
    let h = 2.9;
    let half = 0.72;
    for (sx, sz) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        plain(out, r, Vec3::new(sx * half, h / 2.0 + 0.3, sz * half), Vec3::new(0.16, h, 0.16), WOOD_C);
    }
    for (k, y) in [0.55f32, 1.45, 2.35, 3.2].iter().enumerate() {
        // A storey's floor of planks, some gone.
        for p in 0..6 {
            if rnd(seed, 10 + k as u32 * 8 + p) < 0.18 {
                continue;
            }
            let z = -half + 0.12 + p as f32 * 0.26;
            let burnt = rnd(seed, 40 + k as u32 * 8 + p);
            plain(out, r, Vec3::new(0.0, y + 0.3, z), Vec3::new(1.55, 0.06, 0.23), if burnt < 0.3 { CHAR } else { WOOD });
        }
        if k == 3 {
            break;
        }
        // Boards up three sides (the south side open, the drawbridge's).
        for (side, n) in [(0, Vec3::new(0.0, 0.0, -1.0)), (1, Vec3::new(-1.0, 0.0, 0.0)), (2, Vec3::new(1.0, 0.0, 0.0))] {
            for b in 0..5 {
                let q = rnd(seed, 100 + k as u32 * 20 + side * 5 + b);
                if q < 0.22 {
                    continue; // burnt through
                }
                let along = -half + 0.15 + b as f32 * 0.29;
                let at = n * (half + 0.02) + if n.x == 0.0 { Vec3::new(along, 0.0, 0.0) } else { Vec3::new(0.0, 0.0, along) };
                let size = if n.x == 0.0 { Vec3::new(0.27, 0.86, 0.05) } else { Vec3::new(0.05, 0.86, 0.27) };
                let color = lin(if q < 0.4 { CHAR } else { WOOD }) * (0.8 + q * 0.25);
                bx(
                    out,
                    r,
                    at + Vec3::new(0.0, y + 0.77, 0.0),
                    size * Vec3::new(1.0, 0.9 + q * 0.1, 1.0),
                    Quat::IDENTITY,
                    color,
                    Vec3::ZERO,
                    SH_WOOD,
                );
                if q > 0.9 {
                    bx(
                        out,
                        r,
                        at + Vec3::new(0.0, y + 0.5, 0.0) + n * 0.03,
                        Vec3::new(0.06, 0.06, 0.06),
                        Quat::IDENTITY,
                        lin(EMBER),
                        lin(EMBER) * 1.8,
                        SHADE_LIT,
                    );
                }
            }
        }
    }
    // The drawbridge, hanging half down from the top storey, on chains.
    let hinge = Vec3::new(0.0, 2.65, half + 0.05);
    let q = Quat::from_rotation_x(0.9);
    bx(out, r, hinge + q * Vec3::new(0.0, 0.0, 0.55), Vec3::new(1.3, 0.07, 1.1), q, lin(WOOD) * 0.85, Vec3::ZERO, SH_WOOD);
    for sx in [-0.6f32, 0.6] {
        stick(out, r, Vec3::new(sx, 3.2, half), Quat::from_rotation_x(2.3), 0.0, 0.9, 0.03, 0.03, lin(STEEL_D), Vec3::ZERO, SH_METAL);
    }
    // A parapet of boards round the top, and a torn pennant.
    for k in 0..8 {
        let a = k as f32 / 8.0 * TAU;
        let at = Vec3::new(a.cos() * half, 3.75, a.sin() * half);
        if rnd(seed, 200 + k) < 0.35 {
            continue;
        }
        plain(out, r, at, Vec3::new(0.28, 0.4, 0.28), WOOD_C);
    }
    plain(out, r, Vec3::new(half, 4.3, -half), Vec3::new(0.05, 1.3, 0.05), WOOD);
    for k in 0..3 {
        let len = 0.5 - k as f32 * 0.12;
        bx(
            out,
            r,
            Vec3::new(half + len / 2.0, 4.8 - k as f32 * 0.1, -half),
            Vec3::new(len, 0.09, 0.015),
            Quat::IDENTITY,
            lin(RED),
            Vec3::ZERO,
            SH_CLOTH,
        );
    }
    for (sx, sz) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        wheel(out, r, Vec3::new(sx * (half + 0.12), 0.38, sz * 0.5), 0.36, 0.0, rnd(seed, 300) < 0.3 && sx > 0.0);
    }
}

/// A war wagon over two tiles along x (the root between them), tipped where a wheel has gone:
/// its bed and boards, its wheels, a yoke-pole, and a load of spears and sacks.
pub(super) fn wagon(out: &mut Vec<Instance>, root: Mat4, seed: f64) {
    let tip = Quat::from_euler(EulerRot::XYZ, 0.16, (rnd(seed, 0) - 0.5) * 0.3, -0.05);
    let r = root * Mat4::from_translation(Vec3::new(0.0, 0.05, 0.0)) * Mat4::from_quat(tip);
    plain(out, r, Vec3::new(0.0, 0.5, 0.0), Vec3::new(1.7, 0.08, 0.8), WOOD);
    for sz in [-0.4f32, 0.4] {
        plain(out, r, Vec3::new(0.0, 0.68, sz), Vec3::new(1.7, 0.3, 0.05), if sz < 0.0 { WOOD_C } else { WOOD });
    }
    plain(out, r, Vec3::new(-0.85, 0.68, 0.0), Vec3::new(0.05, 0.3, 0.8), WOOD_C);
    for sx in [-0.6f32, 0.6] {
        wheel(out, r, Vec3::new(sx, 0.32, -0.47), 0.32, 0.0, false);
    }
    wheel(out, r, Vec3::new(-0.6, 0.32, 0.47), 0.32, 0.0, rnd(seed, 1) < 0.5);
    // The fourth wheel lies flat beside it.
    wheel(out, root, Vec3::new(0.7, 0.05, 0.75), 0.32, 0.0, true);
    stick(out, r, Vec3::new(0.85, 0.42, 0.0), Quat::from_rotation_z(-1.35), 0.0, 1.1, 0.06, 0.06, lin(WOOD), Vec3::ZERO, SH_WOOD);
    for k in 0..6 {
        let rot = Quat::from_euler(EulerRot::XYZ, 0.0, 0.1 * k as f32, PI / 2.0 - 0.15 + rnd(seed, 10 + k) * 0.1);
        stick(out, r, Vec3::new(-0.6, 0.62, -0.25 + k as f32 * 0.1), rot, 0.0, 1.3, 0.03, 0.03, lin(WOOD_L), Vec3::ZERO, SH_WOOD);
    }
    for k in 0..3 {
        plain(out, r, Vec3::new(0.25 + k as f32 * 0.22, 0.66, 0.1 - k as f32 * 0.12), Vec3::new(0.26, 0.24, 0.22), 0x8a7a5a);
    }
}

/// A torn banner of Amon on its pole amid stakes: a red cloth in strips of every length, burnt at
/// the hems, a black wolf's head on it.
pub(super) fn banner(out: &mut Vec<Instance>, root: Mat4, seed: f64) {
    let lean = Quat::from_euler(EulerRot::XYZ, (rnd(seed, 0) - 0.5) * 0.12, rnd(seed, 1) * TAU, (rnd(seed, 2) - 0.5) * 0.12);
    let r = root * Mat4::from_quat(lean);
    plain(out, r, Vec3::new(0.0, 1.25, 0.0), Vec3::new(0.07, 2.5, 0.07), WOOD_C);
    plain(out, r, Vec3::new(0.0, 2.35, 0.0), Vec3::new(0.8, 0.05, 0.05), WOOD);
    plain(out, r, Vec3::new(0.0, 2.55, 0.0), Vec3::new(0.06, 0.18, 0.06), STEEL_D);
    for k in 0..6 {
        let len = 0.6 + rnd(seed, 10 + k) * 0.7;
        let x = -0.33 + k as f32 * 0.132;
        let sway = Quat::from_rotation_x((rnd(seed, 20 + k) - 0.5) * 0.16);
        let red = lin(if k % 3 == 1 { RED_D } else { RED }) * (0.85 + rnd(seed, 30 + k) * 0.25);
        bx(out, r, Vec3::new(x, 2.33 - len / 2.0, 0.03), Vec3::new(0.13, len, 0.014), sway, red, Vec3::ZERO, SH_CLOTH);
        bx(out, r, Vec3::new(x, 2.33 - len + 0.04, 0.035), Vec3::new(0.13, 0.08, 0.016), sway, lin(CHAR), Vec3::ZERO, SHADE_LIT);
    }
    // The wolf's head: ears, brow and muzzle in black.
    for (at, size) in [
        (Vec3::new(0.0, 1.95, 0.045), Vec3::new(0.3, 0.2, 0.01)),
        (Vec3::new(-0.11, 2.1, 0.045), Vec3::new(0.07, 0.12, 0.01)),
        (Vec3::new(0.11, 2.1, 0.045), Vec3::new(0.07, 0.12, 0.01)),
        (Vec3::new(0.0, 1.8, 0.045), Vec3::new(0.12, 0.14, 0.01)),
    ] {
        plain(out, r, at, size, BLACK);
    }
    for k in 0..5 {
        let a = k as f32 / 5.0 * TAU + rnd(seed, 40) * TAU;
        let rot = Quat::from_euler(EulerRot::XYZ, a.sin() * 0.4, a, a.cos() * 0.4);
        stick(out, root, Vec3::new(a.cos() * 0.28, 0.0, a.sin() * 0.28), rot, -0.05, 0.5, 0.06, 0.06, lin(WOOD_C), Vec3::ZERO, SH_WOOD);
    }
    plain(out, root, Vec3::new(0.0, 0.05, 0.0), Vec3::new(0.5, 0.1, 0.5), ASH_D);
}

// ------------------------------------------------------------------ the field's litter

/// A skull, its eyes dark, `s` its size.
pub(super) fn skull(out: &mut Vec<Instance>, root: Mat4, at: Vec3, yaw: f32, s: f32) {
    let q = Quat::from_rotation_y(yaw);
    bx(out, root, at, Vec3::new(0.13, 0.12, 0.15) * s, q, lin(BONE), Vec3::ZERO, SHADE_LIT);
    bx(out, root, at + q * Vec3::new(0.0, -0.05, 0.05) * s, Vec3::new(0.09, 0.05, 0.07) * s, q, lin(BONE_D), Vec3::ZERO, SHADE_LIT);
    for sx in [-0.035f32, 0.035] {
        bx(out, root, at + q * Vec3::new(sx, 0.01, 0.075) * s, Vec3::new(0.035, 0.035, 0.01) * s, q, lin(CHAR), Vec3::ZERO, SHADE_LIT);
    }
}

/// What lies about on open ground at `root`: bones, a helmet, a shield, arrows, a smouldering
/// heap, the dead; or nothing. Returns where a heap smoulders, if one does (for its glow).
pub(super) fn litter(out: &mut Vec<Instance>, root: Mat4, seed: f64) -> Option<Vec3> {
    let pick = rnd(seed, 0);
    let yaw = rnd(seed, 1) * TAU;
    let q = Quat::from_rotation_y(yaw);
    let off = Vec3::new((rnd(seed, 2) - 0.5) * 0.5, 0.0, (rnd(seed, 3) - 0.5) * 0.5);
    if pick < 0.3 {
        // Bones: a skull, a ribcage's arcs, long bones.
        skull(out, root, off + Vec3::new(0.0, 0.06, 0.0), yaw, 1.0);
        for k in 0..4 {
            let at = off + q * Vec3::new(0.18, 0.025, -0.12 + k as f32 * 0.07);
            bx(out, root, at, Vec3::new(0.22, 0.03, 0.025), q * Quat::from_rotation_z(0.2), lin(BONE_D), Vec3::ZERO, SHADE_LIT);
        }
        bx(
            out,
            root,
            off + q * Vec3::new(-0.15, 0.025, 0.15),
            Vec3::new(0.05, 0.04, 0.38),
            q * Quat::from_rotation_y(0.6),
            lin(BONE),
            Vec3::ZERO,
            SHADE_LIT,
        );
    } else if pick < 0.45 {
        // A helmet, dented, its nasal bent.
        let at = off + Vec3::new(0.0, 0.07, 0.0);
        let tilt = q * Quat::from_rotation_z(0.5);
        bx(out, root, at, Vec3::new(0.18, 0.13, 0.18), tilt, lin(STEEL_D), Vec3::ZERO, SH_METAL);
        bx(out, root, at + tilt * Vec3::new(0.0, 0.08, 0.0), Vec3::new(0.12, 0.05, 0.12), tilt, lin(STEEL), Vec3::ZERO, SH_METAL);
        bx(out, root, at + tilt * Vec3::new(0.0, -0.04, 0.1), Vec3::new(0.03, 0.1, 0.02), tilt, lin(RUST), Vec3::ZERO, SH_METAL);
    } else if pick < 0.57 {
        shield(out, root, off + Vec3::new(0.0, 0.03, 0.0), q * Quat::from_rotation_x(-PI / 2.0), seed);
    } else if pick < 0.7 {
        // Arrows stuck in the earth, fletched.
        for k in 0..4 {
            let r = |j: u32| rnd(seed, 10 + k * 4 + j);
            let base = off + Vec3::new((r(0) - 0.5) * 0.5, 0.0, (r(1) - 0.5) * 0.5);
            let rot = Quat::from_euler(EulerRot::XYZ, 0.5 + r(2) * 0.3, yaw, (r(3) - 0.5) * 0.4);
            stick(out, root, base, rot, -0.05, 0.42, 0.014, 0.014, lin(WOOD_L), Vec3::ZERO, SH_WOOD);
            stick(out, root, base, rot, 0.3, 0.1, 0.04, 0.005, lin(if k % 2 == 0 { CREAM } else { RED_D }), Vec3::ZERO, SH_CLOTH);
        }
    } else if pick < 0.78 {
        // A smouldering heap of charred timber, glowing.
        for k in 0..4 {
            let rot = Quat::from_euler(EulerRot::XYZ, 0.0, yaw + k as f32 * 0.8, 0.15);
            bx(
                out,
                root,
                off + Vec3::new(0.0, 0.05 + k as f32 * 0.03, 0.0),
                Vec3::new(0.5, 0.07, 0.08),
                rot,
                lin(CHAR),
                Vec3::ZERO,
                SH_WOOD,
            );
        }
        bx(out, root, off + Vec3::new(0.0, 0.07, 0.0), Vec3::new(0.18, 0.05, 0.18), q, lin(EMBER), lin(EMBER) * 2.2, SHADE_LIT);
        return Some(off + Vec3::new(0.0, 0.2, 0.0));
    } else if pick < 0.84 {
        horse(out, root, off, yaw, seed);
    } else if pick < 0.92 {
        // A puddle of blood-mud, glossy.
        bx(
            out,
            root,
            off + Vec3::new(0.0, 0.006, 0.0),
            Vec3::new(0.5 + rnd(seed, 4) * 0.3, 0.012, 0.4),
            q,
            lin(MUD_L),
            lin(MUD) * 0.05,
            SHADE_LIT,
        );
    }
    None
}

/// A dead warhorse on its side, legs out stiff, a red caparison torn over it.
fn horse(out: &mut Vec<Instance>, root: Mat4, at: Vec3, yaw: f32, seed: f64) {
    let q = Quat::from_rotation_y(yaw);
    let c = lin(HORSE);
    bx(out, root, at + Vec3::new(0.0, 0.2, 0.0), Vec3::new(0.85, 0.36, 0.4), q, c, Vec3::ZERO, SHADE_LIT);
    bx(out, root, at + q * Vec3::new(0.0, 0.33, 0.0), Vec3::new(0.6, 0.12, 0.42), q, lin(RED_D), Vec3::ZERO, SH_CLOTH);
    let neck = q * Quat::from_rotation_z(0.5);
    bx(out, root, at + q * Vec3::new(0.52, 0.22, 0.0), Vec3::new(0.38, 0.17, 0.2), neck, c * 0.9, Vec3::ZERO, SHADE_LIT);
    bx(out, root, at + q * Vec3::new(0.78, 0.1, 0.0), Vec3::new(0.3, 0.13, 0.15), q, c * 0.85, Vec3::ZERO, SHADE_LIT);
    for (x, z) in [(-0.3f32, 0.25f32), (-0.2, 0.32), (0.25, 0.27), (0.35, 0.34)] {
        let leg = q * Quat::from_rotation_x(-1.25 + rnd(seed, 9) * 0.2);
        bx(out, root, at + q * Vec3::new(x, 0.12, z + 0.12), Vec3::new(0.07, 0.42, 0.07), leg, c * 0.75, Vec3::ZERO, SHADE_LIT);
    }
    bx(out, root, at + q * Vec3::new(-0.5, 0.18, 0.0), Vec3::new(0.12, 0.08, 0.3), q, lin(BLACK), Vec3::ZERO, SHADE_LIT);
}

// ------------------------------------------------------------------ the climax's props

/// The raven-headed herald of Amon, a tall man in mail and a tattered red cloak with a black
/// wolf on his tabard, his raven's head crested, its eyes burning; a staff with a raven's skull in
/// his right hand. Facing +z.
pub(super) fn herald() -> Collected {
    let (mail, mail_d) = (0x6a6e76 | mat::METAL, 0x4a4e56 | mat::METAL);
    let (red, red_d) = (RED | mat::CLOTH, RED_D | mat::CLOTH);
    let black = BLACK | mat::CLOTH;
    let (feather, feather_l) = (0x141218 | mat::FUR, 0x2a2836 | mat::FUR);
    let beak = 0x3a3634 | mat::IVORY;
    let eye = 0xff3018 | mat::GLOW;
    let (boot, leather) = (0x2a1a12 | mat::LEATHER, 0x4a2e1a | mat::LEATHER);
    let wood = WOOD_C | mat::WOOD;
    let bone = BONE | mat::IVORY;
    let mut s = Sculpt::new(22, 46, 16);
    // Legs and boots.
    for x in [8, 12] {
        s.cuboid(x, 0, 6, x + 2, 2, 9, boot);
        s.cuboid(x, 3, 6, x + 2, 14, 8, mail_d);
    }
    // The body: mail, a red tabard with a black wolf, a belt.
    s.ellipsoid((11.0, 21.0, 7.0), (5.0, 8.0, 3.4), 13, mail);
    s.cuboid(7, 14, 9, 15, 27, 10, red);
    s.cuboid(7, 15, 9, 15, 15, 10, red_d);
    s.cuboid(6, 15, 4, 16, 16, 10, leather);
    for (x, y) in [(10, 22), (11, 22), (12, 22), (9, 23), (13, 23), (10, 20), (11, 19), (12, 20), (11, 21)] {
        s.set(x, y, 10, black);
    }
    // The cloak: from the shoulders down his back to the ground, a half-shell flaring toward the
    // hem, ragged there.
    for y in 1..30 {
        let r = 5.0 + (30 - y) as f32 * 0.07;
        for k in 0..28 {
            let a = PI * (k as f32 / 27.0);
            let (x, z) = ((11.0 + a.cos() * r).round() as i32, (6.5 - a.sin() * r * 0.8).round() as i32);
            let hem = ((k * 7 + 3) % 5) as i32;
            if y < 1 + hem {
                continue;
            }
            s.set(x, y, z, if (k + y as usize).is_multiple_of(9) { red_d } else { red });
        }
    }
    // Shoulders and arms: the left hanging, the right holding the staff.
    s.ellipsoid((6.0, 27.0, 6.5), (2.2, 2.0, 2.4), 0, mail_d);
    s.ellipsoid((16.0, 27.0, 6.5), (2.2, 2.0, 2.4), 0, mail_d);
    s.capsule((5.0, 26.0, 7.0), (4.6, 17.0, 8.0), 1.4, 1.2, mail);
    s.capsule((17.0, 26.0, 7.0), (18.0, 20.0, 10.0), 1.4, 1.2, mail);
    s.ellipsoid((18.0, 19.5, 10.5), (1.3, 1.3, 1.3), 0, leather);
    // The staff, and a raven's skull on it.
    s.cuboid(18, 1, 11, 18, 40, 11, wood);
    s.ellipsoid((18.0, 41.0, 11.0), (1.4, 1.4, 1.8), 0, bone);
    s.cuboid(18, 41, 13, 18, 41, 14, bone);
    s.cuboid(19, 36, 11, 21, 39, 11, red);
    // The neck's ruff of black feathers, and the raven's head.
    s.ellipsoid((11.0, 30.5, 6.5), (4.0, 2.0, 3.6), 0, feather_l);
    s.ellipsoid((11.0, 35.0, 7.0), (3.6, 4.2, 3.8), 0, feather);
    s.capsule((11.0, 34.8, 9.5), (11.0, 32.6, 15.6), 1.7, 0.45, beak);
    s.capsule((11.0, 33.6, 10.0), (11.0, 32.4, 14.0), 1.2, 0.4, 0x5a544e | mat::IVORY);
    for x in [9, 13] {
        s.set(x, 35, 10, eye);
        s.set(x, 36, 10, eye);
    }
    // A crest of feathers swept back.
    for k in 0..5 {
        s.set(11, 39 - k / 3, 6 - k, feather_l);
        s.set(10, 38 - k / 3, 6 - k, feather);
        s.set(12, 38 - k / 3, 6 - k, feather);
    }
    s.paint(|x, y, z, c| ((c & 0xffffff) == 0x141218 && (x + y * 3 + z) % 5 == 0).then_some(feather_l));
    s.finish(0.04)
}

/// The war horn: a great curved horn of ivory bound in gold, its mouthpiece to the left and its
/// bell to the right, hanging from a cord at its middle (the origin).
pub(super) fn horn() -> Collected {
    let (ivory, ivory_d) = (IVORY | mat::IVORY, 0xc8b894 | mat::IVORY);
    let gold = GOLD | mat::GOLD;
    let mut s = Sculpt::new(30, 14, 9);
    for k in 0..=24 {
        let t = k as f32 / 24.0;
        let x = 2.0 + t * 25.0;
        let y = 9.0 - (t * PI).sin() * 6.0 + t * 1.5;
        let r = 0.8 + t * t * 3.0;
        let band = k % 6 == 5;
        s.ellipsoid(
            (x, y, 4.0),
            (1.0, r, r),
            0,
            if band {
                gold
            } else if k % 2 == 0 {
                ivory
            } else {
                ivory_d
            },
        );
    }
    // The bell's dark mouth, and the mouthpiece.
    s.ellipsoid((27.6, 10.5, 4.0), (0.6, 2.4, 2.4), 0, 0x1a1410);
    s.cuboid(1, 8, 4, 2, 9, 4, gold);
    s.finish(0.02)
}

/// The truce stone: a tall white standing stone on a low plinth, a ring carved deep in its face.
pub(super) fn truce_stone(out: &mut Vec<Instance>, root: Mat4) {
    plain(out, root, Vec3::new(0.0, 0.08, 0.0), Vec3::new(1.0, 0.16, 0.8), STONE_D);
    plain(out, root, Vec3::new(0.0, 0.95, 0.0), Vec3::new(0.62, 1.6, 0.36), STONE);
    plain(out, root, Vec3::new(0.0, 1.8, 0.0), Vec3::new(0.5, 0.2, 0.3), STONE);
    plain(out, root, Vec3::new(0.06, 1.95, 0.0), Vec3::new(0.3, 0.12, 0.26), STONE_D);
    for k in 0..12 {
        let a = k as f32 / 12.0 * TAU;
        let at = Vec3::new(a.cos() * 0.2, 1.1 + a.sin() * 0.2, 0.185);
        bx(out, root, at, Vec3::new(0.07, 0.07, 0.02), Quat::from_rotation_z(a), lin(STONE_D) * 0.8, Vec3::ZERO, SHADE_LIT);
    }
}

/// The burnt gallows-tree the horn hangs from: a charred, split trunk, a crooked arm out over the
/// south side, roots clawing the ground, a noose of old rope. (The horn and its cord are drawn
/// apart, to swing.) Returns where the cord hangs from.
pub(super) fn gallows(out: &mut Vec<Instance>, root: Mat4) -> Vec3 {
    let bark = |k: u32| lin(if k.is_multiple_of(3) { 0x3a2a20 } else { 0x2a201a }) * (0.9 + (k % 4) as f32 * 0.08);
    let mut at = Vec3::new(0.0, 0.0, 0.0);
    for k in 0..9 {
        let w = 0.42 - k as f32 * 0.03;
        let step = Vec3::new(((k * 7) % 5) as f32 * 0.012 - 0.024, 0.32, -0.015);
        bx(
            out,
            root,
            at + step * 0.5 + Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(w, 0.34, w * 0.9),
            Quat::from_rotation_y(k as f32 * 0.4),
            bark(k),
            Vec3::ZERO,
            SHADE_LIT,
        );
        at += step;
    }
    // A cleft at the top: two snags.
    for s in [-1.0f32, 1.0] {
        let q = Quat::from_rotation_z(s * 0.5);
        stick(out, root, at, q, 0.0, 0.7, 0.14, 0.14, bark(2), Vec3::ZERO, SHADE_LIT);
    }
    // The arm, out to the south (toward +z) and up a little.
    let base = Vec3::new(0.0, 2.3, 0.0);
    let q = Quat::from_rotation_x(PI / 2.0 - 0.25);
    stick(out, root, base, q, 0.0, 0.95, 0.17, 0.15, bark(1), Vec3::ZERO, SHADE_LIT);
    let tip = base + q * Vec3::new(0.0, 0.85, 0.0);
    stick(out, root, tip, Quat::from_rotation_x(0.7), 0.0, 0.35, 0.08, 0.08, bark(5), Vec3::ZERO, SHADE_LIT);
    // Roots.
    for k in 0..5 {
        let a = k as f32 / 5.0 * TAU + 0.3;
        let rot = Quat::from_rotation_y(-a) * Quat::from_rotation_z(1.2);
        stick(out, root, Vec3::new(a.cos() * 0.15, 0.05, a.sin() * 0.15), rot, 0.0, 0.55, 0.12, 0.1, bark(k), Vec3::ZERO, SHADE_LIT);
    }
    // An old noose from the arm's root.
    let noose = base + q * Vec3::new(0.0, 0.3, 0.0);
    plain(out, root, noose - Vec3::new(0.0, 0.35, 0.0), Vec3::new(0.025, 0.7, 0.025), ROPE);
    plain(out, root, noose - Vec3::new(0.0, 0.75, 0.0), Vec3::new(0.14, 0.14, 0.03), ROPE);
    root.transform_point3(tip - Vec3::new(0.0, 0.05, 0.0))
}

/// A rider of the ghost army: a horse at the gallop and its rider, his spear couched, built
/// facing +x.
pub(super) fn rider(out: &mut Vec<Instance>, root: Mat4, gallop: f32) {
    let c = Vec3::ONE;
    let b = |out: &mut Vec<Instance>, at: Vec3, size: Vec3, rot: Quat| bx(out, root, at, size, rot, c, Vec3::ZERO, SHADE_UNLIT);
    let lift = gallop.sin() * 0.06;
    b(out, Vec3::new(0.0, 0.75 + lift, 0.0), Vec3::new(1.0, 0.4, 0.34), Quat::from_rotation_z(gallop.sin() * 0.08));
    b(out, Vec3::new(0.6, 1.0 + lift, 0.0), Vec3::new(0.4, 0.22, 0.2), Quat::from_rotation_z(0.7));
    b(out, Vec3::new(0.82, 1.12 + lift, 0.0), Vec3::new(0.3, 0.14, 0.16), Quat::IDENTITY);
    for (k, x) in [-0.35f32, -0.25, 0.3, 0.4].iter().enumerate() {
        let swing = (gallop + k as f32 * 1.6).sin() * 0.7;
        let q = Quat::from_rotation_z(swing);
        b(
            out,
            Vec3::new(*x, 0.55 + lift, if k % 2 == 0 { 0.12 } else { -0.12 }) + q * Vec3::new(0.0, -0.25, 0.0),
            Vec3::new(0.08, 0.5, 0.08),
            q,
        );
    }
    b(out, Vec3::new(-0.55, 0.85 + lift, 0.0), Vec3::new(0.35, 0.08, 0.08), Quat::from_rotation_z(-0.6));
    // The rider, his cloak streaming, his spear.
    b(out, Vec3::new(0.0, 1.3 + lift, 0.0), Vec3::new(0.24, 0.5, 0.3), Quat::from_rotation_z(-0.2));
    b(out, Vec3::new(0.06, 1.66 + lift, 0.0), Vec3::new(0.18, 0.2, 0.18), Quat::IDENTITY);
    b(out, Vec3::new(-0.4, 1.4 + lift, 0.0), Vec3::new(0.55, 0.06, 0.32), Quat::from_rotation_z(0.3 + gallop.cos() * 0.1));
    b(out, Vec3::new(0.5, 1.35 + lift, 0.15), Vec3::new(1.8, 0.04, 0.04), Quat::from_rotation_z(-0.12));
}

/// Amon's great banner on the hill: a pole twice a man's height and more, a crossbar, and the red
/// cloth with his sign: a wolf with a serpent's tail breathing fire. Facing +z.
pub(super) fn great_banner(out: &mut Vec<Instance>, root: Mat4) {
    plain(out, root, Vec3::new(0.0, 3.4, 0.0), Vec3::new(0.16, 6.8, 0.16), WOOD_C);
    plain(out, root, Vec3::new(0.0, 6.5, 0.05), Vec3::new(2.8, 0.12, 0.12), WOOD);
    bx(out, root, Vec3::new(0.0, 7.0, 0.0), Vec3::new(0.2, 0.5, 0.2), Quat::from_rotation_y(PI / 4.0), lin(STEEL_D), Vec3::ZERO, SH_METAL);
    // The cloth: columns of red, torn at the foot.
    let (w, cols) = (2.6f32, 13);
    for k in 0..cols {
        let len = 3.2 - ((k * 5 + 2) % 7) as f32 * 0.12 - if k % 4 == 3 { 0.5 } else { 0.0 };
        let x = -w / 2.0 + (k as f32 + 0.5) * w / cols as f32;
        let ripple = ((k as f32) * 1.3).sin() * 0.04;
        plain(
            out,
            root,
            Vec3::new(x, 6.45 - len / 2.0, 0.12 + ripple),
            Vec3::new(w / cols as f32 + 0.005, len, 0.03),
            if k % 5 == 2 { RED_D } else { RED },
        );
        plain(out, root, Vec3::new(x, 6.45 - len + 0.06, 0.125 + ripple), Vec3::new(w / cols as f32, 0.12, 0.035), CHAR);
    }
    // The sign: a wolf's body and head, its serpent tail coiling up, fire from its jaws.
    let px = 0.1;
    let sign: [&str; 16] = [
        "......ss..........",
        ".....s..s.........",
        ".....s...s........",
        "......s..s........",
        ".......ss.........",
        "........s.....ww..",
        "........s....wwww.",
        "........wwwwwwwwwf",
        ".......wwwwwwwwwff",
        ".......wwwwwwwwf.f",
        ".......wwwwwww..ff",
        ".......w.w..w.w...",
        ".......w.w..w.w...",
        "......ww.ww.ww.ww.",
        "..................",
        "..................",
    ];
    for (row, line) in sign.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            let (c, glow) = match ch {
                'w' | 's' => (lin(BLACK), Vec3::ZERO),
                'f' => (lin(LAVA_HOT), lin(EMBER) * 1.4),
                _ => continue,
            };
            let at = Vec3::new(-0.9 + col as f32 * px, 6.0 - row as f32 * px, 0.15);
            bx(out, root, at, Vec3::new(px, px, 0.02), Quat::IDENTITY, c, glow, SHADE_LIT);
        }
    }
}
