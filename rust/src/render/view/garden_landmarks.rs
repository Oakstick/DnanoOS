//! The Garden of Krampus's landmarks in the crypt's dressed stone (RENDERING.md, Stage B, "The
//! Garden, at the Cisterns' bar"): the round temples on their lawns, the churchyard, the wayside
//! shrines and the gate out. They were voxel sculpts in chalk and plain-lit cubes; they are now
//! built of the shared stone pieces (`crypt.rs`, `story_sets.rs`), each in its own material.

use super::crypt::pal::{AMBER, CHARCOAL, COPPER, IRON, MOSS, PALE_STONE, UMBER, VERDIGRIS};
use super::crypt::{
    arch_cubes, block, block_m, brazier_cubes, glowing_block, jit, masonry, moss_cap, pillar_cubes, statue_cubes, Arch, Wall, SHADE_BRONZE,
    SHADE_FLAG, SHADE_IRON, SHADE_SOFT, SHADE_STONE, SHADE_WOOD,
};
use super::garden_beds::pal::{EARTH, LEAVES, RUST, TURF};
use super::*;

/// Slate for roofs and spires.
const SLATE: u32 = 0x3a3e44;
/// Gilt for the finials.
const GILT: u32 = 0xb08a3a;
/// Oak for the gate's tympanum.
const OAK: u32 = 0x4a3220;

/// A ring of `n` blocks round `c` at radius `r` (their middles), each `size` (along the ring,
/// up, out), coloured by `col(k)`.
#[allow(clippy::too_many_arguments)]
fn ring(out: &mut Vec<Instance>, c: Vec3, r: f32, n: usize, size: Vec3, phase: f32, shade: f32, col: impl Fn(usize) -> Vec3, jitter: f32) {
    for k in 0..n {
        let a = k as f32 / n as f32 * TAU + phase;
        let j = hash(k as f64 * 1.7 + c.x as f64, c.z as f64 + r as f64 * 3.0) as f32 - 0.5;
        let at = c + Vec3::new(a.cos() * (r + j * jitter), j * jitter * 0.5, a.sin() * (r + j * jitter));
        let rot = Quat::from_rotation_y(-PI / 2.0 - a + j * 0.04);
        let len = TAU * r / n as f32 + 0.015;
        block_m(
            out,
            Mat4::from_scale_rotation_translation(Vec3::new(len, size.y, size.z) * Vec3::new(1.0, 1.0 - j.abs() * 0.1, 1.0), rot, at),
            col(k),
            shade,
        );
    }
}

/// A disc of `r` at `c` (its middle), `h` thick: an octagon of two squares.
fn disc(out: &mut Vec<Instance>, c: Vec3, r: f32, h: f32, col: Vec3, shade: f32) {
    for turn in [0.0, PI / 4.0] {
        block(out, c, Vec3::new(r * 1.85, h, r * 1.85), turn, col, shade);
    }
}

/// A round temple (a monopteros) standing at `o` (its foot) on a 2×2 lawn: a stepped plinth of
/// dressed stone in rings, a flagged floor, eight columns of drums (the crypt's pillars) under an
/// entablature and cornice, a verdigris dome with copper ribs and a gilt finial, and a hooded
/// figure inside before a votive flame. Returns where the flame burns.
pub(super) fn temple_cubes(out: &mut Vec<Instance>, o: Vec3, seed: (f64, f64)) -> Vec3 {
    let (hx, hy) = seed;
    let pale = |k: usize| {
        let h = hash(hx + k as f64 * 1.3, hy * 2.0 + k as f64);
        if h < 0.18 {
            jit(CHARCOAL[k % 3], h, 0.08)
        } else {
            jit(PALE_STONE[k % 2], h, 0.06) * 0.8
        }
    };
    // The plinth: three steps, each a ring of chunky blocks round a dark core.
    let mut y = 0.0;
    for (i, (r, hgt)) in [(0.88f32, 0.09f32), (0.78, 0.09), (0.69, 0.08)].into_iter().enumerate() {
        disc(out, o + Vec3::Y * (y + hgt / 2.0), r - 0.1, hgt, jit(CHARCOAL[1], hx, 0.04) * 0.7, SHADE_STONE);
        let n = (r * 22.0) as usize;
        ring(
            out,
            o + Vec3::Y * (y + hgt / 2.0),
            r - 0.08,
            n,
            Vec3::new(0.0, hgt - 0.01, 0.18),
            i as f32 * 0.3,
            SHADE_STONE,
            |k| pale(k + i * 31),
            0.02,
        );
        y += hgt;
    }
    disc(out, o + Vec3::Y * (y + 0.01), 0.6, 0.02, jit(PALE_STONE[1], hy, 0.04) * 0.6, SHADE_FLAG);
    // Moss along the lowest step, here and there.
    for k in 0..3 {
        let a = hash(hx * 3.0 + k as f64, hy) as f32 * TAU;
        block(out, o + Vec3::new(a.cos() * 0.84, 0.095, a.sin() * 0.84), Vec3::new(0.18, 0.02, 0.1), -a, jit(MOSS[k], hy, 0.1), SHADE_SOFT);
    }
    let floor = y + 0.02;
    // The colonnade.
    let col_h = 1.12;
    for k in 0..8 {
        let a = k as f32 / 8.0 * TAU + 0.39;
        pillar_cubes(out, o + Vec3::new(a.cos() * 0.55, floor, a.sin() * 0.55), col_h, 4, 0.36, (hx + k as f64 * 3.0, hy));
    }
    // The entablature and its cornice.
    let top = floor + col_h;
    ring(out, o + Vec3::Y * (top + 0.07), 0.55, 16, Vec3::new(0.0, 0.14, 0.24), 0.1, SHADE_STONE, |k| pale(k + 7), 0.01);
    disc(out, o + Vec3::Y * (top + 0.07), 0.46, 0.14, jit(CHARCOAL[2], hy, 0.04) * 0.7, SHADE_STONE);
    ring(
        out,
        o + Vec3::Y * (top + 0.17),
        0.6,
        18,
        Vec3::new(0.0, 0.06, 0.16),
        0.0,
        SHADE_STONE,
        |k| jit(PALE_STONE[0], k as f64, 0.04) * 0.75,
        0.0,
    );
    // The dome: rings of verdigris on the curve, copper ribs over them, a gilt finial.
    let (base, dr, dh) = (top + 0.2, 0.56f32, 0.44f32);
    for i in 0..4 {
        let phi = i as f32 * PI / 8.0;
        let r = dr * phi.cos();
        let c = jit(VERDIGRIS, hash(hx + i as f64, hy) * 0.8 + 0.1, 0.1) * (0.85 + 0.08 * i as f32);
        disc(out, o + Vec3::Y * (base + dh * phi.sin() + 0.06), r, 0.14, c, SHADE_STONE);
    }
    let at = |a: f32, phi: f32| {
        o + Vec3::new(a.cos() * (dr + 0.02) * phi.cos(), base + (dh + 0.02) * phi.sin() + 0.04, a.sin() * (dr + 0.02) * phi.cos())
    };
    for k in 0..8 {
        let a = k as f32 / 8.0 * TAU + PI / 8.0;
        for s in 0..3 {
            let (p0, p1) = (at(a, s as f32 * PI / 6.6), at(a, (s + 1) as f32 * PI / 6.6));
            let d = p1 - p0;
            let m = Mat4::from_scale_rotation_translation(
                Vec3::new(0.04, d.length() + 0.03, 0.04),
                Quat::from_rotation_arc(Vec3::Y, d.normalize()),
                (p0 + p1) / 2.0,
            );
            block_m(out, m, lin(COPPER) * 0.85, SHADE_BRONZE);
        }
    }
    block(out, o + Vec3::Y * (base + dh + 0.1), Vec3::new(0.12, 0.08, 0.12), PI / 4.0, lin(GILT), SHADE_BRONZE);
    block(out, o + Vec3::Y * (base + dh + 0.22), Vec3::new(0.04, 0.18, 0.04), 0.0, lin(GILT), SHADE_BRONZE);
    // The figure inside on its pedestal, and the flame in a copper dish before it.
    block(out, o + Vec3::Y * (floor + 0.08), Vec3::new(0.26, 0.16, 0.22), 0.0, pale(3), SHADE_STONE);
    statue_cubes(out, o + Vec3::Y * (floor + 0.16), 0.0, 0.42, (hx, hy));
    let flame = o + Vec3::new(0.0, floor + 0.1, 0.3);
    block(out, flame - Vec3::Y * 0.06, Vec3::new(0.05, 0.12, 0.05), 0.0, lin(COPPER) * 0.8, SHADE_BRONZE);
    block(out, flame, Vec3::new(0.11, 0.025, 0.11), PI / 4.0, lin(COPPER), SHADE_BRONZE);
    glowing_block(out, flame + Vec3::Y * 0.04, Vec3::new(0.035, 0.06, 0.035), lin(AMBER), lin(AMBER) * 2.2);
    flame + Vec3::Y * 0.04
}

/// A wayside shrine at `p` (its foot), facing `yaw` (+z turned by it): a short pillar of
/// dressed stone with a dark niche and a candle in it, under a gabled slate roof and an iron
/// cross. Returns where the candle burns.
pub(super) fn shrine_cubes(out: &mut Vec<Instance>, p: Vec3, yaw: f32, seed: (f64, f64)) -> Vec3 {
    let q = Quat::from_rotation_y(yaw);
    let put = |out: &mut Vec<Instance>, off: Vec3, size: Vec3, col: Vec3, shade: f32| {
        block_m(out, Mat4::from_scale_rotation_translation(size, q, p + q * off), col, shade);
    };
    let (hx, hy) = seed;
    let stone = jit(PALE_STONE[(hash(hx, hy) * 2.0) as usize % 2], hy, 0.05) * 0.78;
    put(out, Vec3::new(0.0, 0.05, 0.0), Vec3::new(0.36, 0.1, 0.36), stone * 0.85, SHADE_STONE);
    for (k, w) in [0.26f32, 0.24, 0.25].into_iter().enumerate() {
        let c = if k == 1 { jit(UMBER[0], hx, 0.06) } else { jit(CHARCOAL[k], hy, 0.08) * 1.1 };
        put(
            out,
            Vec3::new((hash(hx + k as f64, hy) as f32 - 0.5) * 0.02, 0.18 + k as f32 * 0.17, 0.0),
            Vec3::new(w, 0.165, w),
            c,
            SHADE_STONE,
        );
    }
    // The niche, the candle in it, and a ledge under it.
    put(out, Vec3::new(0.0, 0.43, 0.105), Vec3::new(0.15, 0.17, 0.06), lin(0x0e0c0a), SHADE_LIT);
    put(out, Vec3::new(0.0, 0.34, 0.14), Vec3::new(0.22, 0.03, 0.06), stone, SHADE_STONE);
    put(out, Vec3::new(0.0, 0.385, 0.1), Vec3::new(0.03, 0.05, 0.03), lin(0xd8d0b8), SHADE_LIT);
    let flame = p + q * Vec3::new(0.0, 0.43, 0.1);
    glowing_block(out, flame, Vec3::new(0.022, 0.04, 0.022), lin(AMBER), lin(AMBER) * 2.4);
    put(out, Vec3::new(0.0, 0.56, 0.0), Vec3::new(0.32, 0.04, 0.32), stone, SHADE_STONE);
    for s in [-1.0f32, 1.0] {
        let roof = q * Quat::from_rotation_z(s * 0.62);
        block_m(
            out,
            Mat4::from_scale_rotation_translation(Vec3::new(0.24, 0.035, 0.38), roof, p + q * Vec3::new(s * 0.085, 0.64, 0.0)),
            lin(SLATE),
            SHADE_STONE,
        );
    }
    let iron = lin(IRON).lerp(lin(RUST), 0.3);
    put(out, Vec3::new(0.0, 0.8, 0.0), Vec3::new(0.022, 0.18, 0.022), iron, SHADE_IRON);
    put(out, Vec3::new(0.0, 0.83, 0.0), Vec3::new(0.1, 0.022, 0.022), iron, SHADE_IRON);
    if hash(hx * 2.0, hy * 3.0) < 0.6 {
        put(out, Vec3::new(0.06, 0.11, 0.13), Vec3::new(0.14, 0.02, 0.08), jit(MOSS[1], hx, 0.1), SHADE_SOFT);
    }
    flame
}

/// A stone urn on a plinth, a few muted flowers in it, at `p` (its foot).
pub(super) fn urn_cubes(out: &mut Vec<Instance>, p: Vec3, seed: (f64, f64)) {
    let (hx, hy) = seed;
    let stone = jit(PALE_STONE[0], hash(hx, hy), 0.05) * 0.75;
    block(out, p + Vec3::Y * 0.06, Vec3::new(0.3, 0.12, 0.3), 0.0, stone * 0.85, SHADE_STONE);
    block(out, p + Vec3::Y * 0.2, Vec3::new(0.2, 0.16, 0.2), 0.0, jit(CHARCOAL[1], hx, 0.06) * 1.1, SHADE_STONE);
    block(out, p + Vec3::Y * 0.3, Vec3::new(0.28, 0.04, 0.28), 0.0, stone, SHADE_STONE);
    block(out, p + Vec3::Y * 0.36, Vec3::new(0.12, 0.08, 0.12), PI / 4.0, stone * 0.9, SHADE_STONE);
    block(out, p + Vec3::Y * 0.45, Vec3::new(0.26, 0.1, 0.26), PI / 4.0, stone, SHADE_STONE);
    block(out, p + Vec3::Y * 0.51, Vec3::new(0.3, 0.03, 0.3), 0.0, stone * 1.05, SHADE_STONE);
    for k in 0..5 {
        let a = k as f32 * 1.3 + hx as f32;
        let c = [0x6a3a3a, 0x7a6a8a, 0x8a7a50][k % 3];
        block(out, p + Vec3::new(a.cos() * 0.07, 0.56, a.sin() * 0.07), Vec3::splat(0.05), a, lin(c), SHADE_SOFT);
    }
    block(out, p + Vec3::Y * 0.54, Vec3::new(0.2, 0.04, 0.2), 0.3, lin(TURF[0]) * 0.8, SHADE_SOFT);
}

/// Instances built round the origin, set down at `at`, turned `yaw` and scaled `s`.
fn placed(out: &mut Vec<Instance>, built: Vec<Instance>, at: Vec3, yaw: f32, s: f32) {
    let m = Mat4::from_scale_rotation_translation(Vec3::splat(s), Quat::from_rotation_y(yaw), at);
    for mut i in built {
        i.m = (m * Mat4::from_cols_array(&i.m)).to_cols_array();
        out.push(i);
    }
}

impl WorldView {
    /// A churchyard tile: uneven dark turf with leaves and tufts; along every path a low kerb
    /// of dressed stone under a pale coping, rusted railings with spear heads on it; the story's
    /// graves (headstones, crosses, obelisks, a chest tomb, sunk slabs) at the garden's scale,
    /// facing the path; the churchyard's dead tree; a grave lantern now and then.
    pub(super) fn churchyard_tile(&mut self, grid: &Grid, p: P, dead_tree: bool, tomb: bool, candle: bool) {
        let (x, y) = (p.x, p.y);
        let (xf, yf) = (x as f32, y as f32);
        let (hx, hy) = (x as f64, y as f64);
        let floor = grid.elevation(x, y) as f32 * STEP;
        let base = floor + 0.08;
        let start = self.static_cubes.len();
        let out = &mut self.static_cubes;
        block(out, Vec3::new(xf, (base - 0.18) / 2.0, yf), Vec3::new(1.0, base + 0.1, 1.0), 0.0, lin(EARTH[1]) * 0.8, SHADE_LIT);
        for (i, (ox, oz)) in [(-0.25f32, -0.25f32), (0.25, -0.25), (-0.25, 0.25), (0.25, 0.25)].into_iter().enumerate() {
            let k = hash(hx * 2.0 + ox as f64 * 3.0, hy * 2.0 + oz as f64 * 5.0) as f32;
            let lift = (k - 0.5) * 0.04;
            block(
                out,
                Vec3::new(xf + ox, base - 0.03 + lift, yf + oz),
                Vec3::new(0.5, 0.06, 0.5),
                0.0,
                jit(TURF[(i + (k * 3.0) as usize) % 3], k as f64, 0.1) * 0.72,
                SHADE_GRASS,
            );
        }
        let roads: Vec<Dir> = CARDINALS
            .iter()
            .copied()
            .filter(|d| {
                let (dx, dy) = d.vec();
                grid.in_bounds(x + dx, y + dy) && grid.get(x + dx, y + dy) != Tile::Rock
            })
            .collect();
        let iron = |h: f64| lin(IRON).lerp(lin(RUST), (h as f32 * 1.6).min(1.0) * 0.45);
        for &d in &roads {
            let (dx, dy) = d.vec();
            let turn = if dx != 0 { PI / 2.0 } else { 0.0 };
            let kerb = 0.3;
            masonry(
                out,
                &Wall {
                    centre: Vec3::new(xf, 0.0, yf),
                    dir: (dx, dy),
                    half: 0.5,
                    base: floor,
                    height: kerb,
                    courses: 1,
                    seed: (hx * 1.9 + 3.0, hy * 2.3 + 1.0),
                    len: (0.24, 0.2),
                    depth: (0.04, 0.05),
                    face: 0.38,
                    wet: false,
                    ruined: false,
                    foot: None,
                    openings: &[],
                },
            );
            let at = |u: f32, h: f32, o: f32| {
                Vec3::new(xf + dx as f32 * o + if dx != 0 { 0.0 } else { u }, h, yf + dy as f32 * o + if dx != 0 { u } else { 0.0 })
            };
            // The coping, in two slabs, a touch askew.
            for (k, u) in [-0.25f32, 0.25].into_iter().enumerate() {
                let h = hash(hx * 3.1 + k as f64 + dx as f64, hy * 1.3 + dy as f64);
                block(
                    out,
                    at(u, floor + kerb + 0.025, 0.42),
                    Vec3::new(0.49, 0.05, 0.2),
                    turn + (h as f32 - 0.5) * 0.04,
                    jit(PALE_STONE[k], h, 0.05) * 0.72,
                    SHADE_STONE,
                );
            }
            // Railings: bars with spear heads, a collar now and then, two rails.
            let rail_base = floor + kerb + 0.05;
            for k in 0..8 {
                let u = -0.44 + k as f32 * 0.125;
                let h = hash(hx * 5.0 + k as f64 + dx as f64 * 3.0, hy * 7.0 + dy as f64);
                let bent = h > 0.93;
                let tilt = if bent { Quat::from_rotation_z(0.12) * Quat::from_rotation_x(0.08) } else { Quat::IDENTITY };
                let len = 0.3;
                block_m(
                    out,
                    Mat4::from_scale_rotation_translation(Vec3::new(0.018, len, 0.018), tilt, at(u, rail_base + len / 2.0, 0.42)),
                    iron(h),
                    SHADE_IRON,
                );
                block_m(
                    out,
                    Mat4::from_scale_rotation_translation(
                        Vec3::new(0.035, 0.035, 0.018),
                        tilt * Quat::from_rotation_z(PI / 4.0),
                        at(u, rail_base + len + 0.012, 0.42),
                    ),
                    iron(h),
                    SHADE_IRON,
                );
            }
            for hgt in [0.06f32, 0.25] {
                block(out, at(0.0, rail_base + hgt, 0.42), Vec3::new(1.0, 0.018, 0.022), turn, iron(0.5), SHADE_IRON);
            }
        }
        // The graves face the nearest path.
        let face = roads.first().copied().unwrap_or(Dir::Down);
        let (fx, fz) = {
            let (a, b) = face.vec();
            (a as f32, b as f32)
        };
        let yaw = fx.atan2(fz);
        let q = Quat::from_rotation_y(yaw);
        let ground = Vec3::new(xf, base, yf);
        if dead_tree {
            let mut tree = Vec::new();
            story_sets::dead_tree(&mut tree, Vec3::ZERO, 3.6, hx * 7.3 + hy);
            placed(out, tree, ground, hash(hx, hy) as f32 * TAU, 0.5);
        } else if tomb {
            let mut g = Vec::new();
            story_sets::grave(&mut g, Vec3::ZERO, 0.72, 0.8);
            placed(out, g, ground + q * Vec3::new(0.0, 0.0, -0.22), yaw, 0.6);
        } else {
            let count = 1 + (hash(hx * 3.0, hy * 5.0) * 1.99) as usize;
            for n in 0..count {
                let k = hash(hx * 13.0 + n as f64, hy * 7.0) as f32;
                // (No chest tombs crowded in pairs; the tall obelisk only alone.)
                let k = if count == 2 && (0.5..0.84).contains(&k) { k * 0.3 } else { k };
                let j = hash(hx * 5.0 + n as f64 * 3.0, hy * 11.0) as f32;
                let slot = if count == 1 { 0.0 } else { (n as f32 - 0.5) * 0.46 };
                let mut g = Vec::new();
                story_sets::grave(&mut g, Vec3::ZERO, k, j);
                placed(out, g, ground + q * Vec3::new(slot, 0.0, -0.26), yaw + (k - 0.5) * 0.2, 0.55);
            }
        }
        // Leaves and tufts in the turf.
        for k in 0..4 {
            let (a, b) = (hash(hx * 3.3 + k as f64, hy * 7.7) as f32, hash(hy * 2.9 + k as f64, hx * 5.1) as f32);
            let at = Vec3::new(xf + (a - 0.5) * 0.8, base + 0.005, yf + (b - 0.5) * 0.8);
            let m = Mat4::from_scale_rotation_translation(Vec3::new(0.06, 0.01, 0.045), Quat::from_rotation_y(a * 9.0), at);
            block_m(out, m, jit(LEAVES[k % 3], b as f64, 0.15) * 0.85, SHADE_SOFT);
        }
        if candle {
            // A grave lantern of stone with a candle glowing inside.
            let o = ground + q * Vec3::new(0.3, 0.0, 0.25);
            let stone = jit(PALE_STONE[1], hx, 0.05) * 0.7;
            block(out, o + Vec3::Y * 0.05, Vec3::new(0.1, 0.1, 0.1), yaw, stone, SHADE_STONE);
            glowing_block(out, o + Vec3::Y * 0.13, Vec3::new(0.04, 0.06, 0.04), lin(AMBER), lin(AMBER) * 2.2);
            block(out, o + Vec3::Y * 0.2, Vec3::new(0.12, 0.03, 0.12), yaw, stone, SHADE_STONE);
            self.candles.push(o + Vec3::Y * 0.2);
        }
        let len = self.static_cubes.len() - start;
        self.rock_index.insert(tile_key(x, y), (start, len));
        self.authored.push((start, len));
        if dead_tree {
            // Over his head: it fades out of the way, as the trees do.
            let shades = self.static_cubes[start..start + len].iter().map(|c| c.emissive[3]).collect();
            self.tree_index.insert(p, self.trees.len());
            self.trees.push(Tree {
                range: (start, len),
                base: ground,
                radius: 0.5,
                height: 1.8,
                fade: 1.0,
                alive: true,
                shades: Some(shades),
            });
        }
    }

    /// The garden's gate in the north wall, in the vaults' dressed stone: two towers of chunky
    /// courses on pale plinths, battlemented on corbels under slate spires; between them a round
    /// arch of wedge stones on pale piers, an oak tympanum over the doors, the wall above in
    /// courses up to a cornice and merlons; copper braziers before the towers (their flames are
    /// the gate's). The doors and lock lamps move, so the gate's own code draws them each frame.
    pub(super) fn build_garden_gate(&mut self, grid: &Grid, gate: P, sockets: &[Vec3]) {
        let floor = grid.elevation(gate.x, gate.y + 1) as f32 * STEP;
        let o = Vec3::new(gate.x as f32, floor, gate.y as f32);
        let (gx, gy) = (gate.x as f64, gate.y as f64);
        let start = self.static_cubes.len();
        let mut smoke = vec![];
        let out = &mut self.static_cubes;
        // Threshold, and the dark way on behind the doors.
        block(out, o + Vec3::new(0.0, -0.04, 0.05), Vec3::new(1.1, 0.1, 0.95), 0.0, jit(PALE_STONE[1], 0.3, 0.0) * 0.7, SHADE_STONE);
        block(out, o + Vec3::new(0.0, 0.85, -0.42), Vec3::new(0.9, 1.8, 0.08), 0.0, lin(0x060606), SHADE_LIT);
        let tower_h = 2.5;
        for side in [-1.0f32, 1.0] {
            let t = o + Vec3::new(side * 0.98, 0.0, 0.0);
            block(
                out,
                t + Vec3::Y * (tower_h / 2.0 - 0.15),
                Vec3::new(0.8, tower_h + 0.3, 0.8),
                0.0,
                jit(CHARCOAL[1], gx, 0.04) * 0.7,
                SHADE_STONE,
            );
            for d in CARDINALS {
                let (dx, dy) = d.vec();
                masonry(
                    out,
                    &Wall {
                        centre: t,
                        dir: (dx, dy),
                        half: 0.44,
                        base: floor - 0.1,
                        height: tower_h + 0.1,
                        courses: 7,
                        seed: (gx + side as f64 * 3.0 + dx as f64, gy + dy as f64 * 2.0),
                        len: (0.22, 0.2),
                        depth: (0.04, 0.05),
                        face: 0.38,
                        wet: false,
                        ruined: false,
                        foot: (dy == 1).then_some(floor),
                        openings: &[],
                    },
                );
            }
            // Corbels, the battlements on them, and the slate spire.
            let top = floor + tower_h;
            for k in 0..12 {
                let a = k as f32 / 12.0 * TAU;
                let p = t + Vec3::new(a.cos().clamp(-0.71, 0.71) * 0.62, 0.0, a.sin().clamp(-0.71, 0.71) * 0.62);
                block(
                    out,
                    Vec3::new(p.x, top + 0.02, p.z),
                    Vec3::new(0.12, 0.1, 0.12),
                    0.0,
                    jit(PALE_STONE[k % 2], k as f64, 0.05) * 0.72,
                    SHADE_STONE,
                );
            }
            block(out, t + Vec3::Y * (tower_h + 0.12), Vec3::new(0.98, 0.1, 0.98), 0.0, jit(PALE_STONE[0], gy, 0.04) * 0.75, SHADE_STONE);
            for (mx, mz) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0), (0.0, 1.0), (0.0, -1.0), (1.0, 0.0), (-1.0, 0.0)] {
                let h = hash(mx as f64 + gx, mz as f64 + side as f64);
                block(
                    out,
                    t + Vec3::new(mx * 0.38, tower_h + 0.28, mz * 0.38),
                    Vec3::new(0.2, 0.24 - h as f32 * 0.04, 0.2),
                    (h as f32 - 0.5) * 0.06,
                    jit(CHARCOAL[(h * 3.0) as usize % 3], h, 0.1),
                    SHADE_STONE,
                );
            }
            for (k, w) in [0.62f32, 0.5, 0.38, 0.26, 0.15].into_iter().enumerate() {
                block(
                    out,
                    t + Vec3::Y * (tower_h + 0.24 + k as f32 * 0.17),
                    Vec3::new(w, 0.17, w),
                    PI / 4.0,
                    lin(SLATE) * (1.0 + k as f32 * 0.04),
                    SHADE_STONE,
                );
            }
            block(out, t + Vec3::Y * (tower_h + 1.18), Vec3::new(0.04, 0.2, 0.04), 0.0, lin(GILT), SHADE_BRONZE);
            block(out, t + Vec3::new(-side * 0.18, tower_h + 0.19, 0.22), Vec3::new(0.26, 0.03, 0.2), 0.3, lin(MOSS[1]), SHADE_SOFT);
            // Arrow slits.
            for y in [0.95f32, 1.65] {
                block(out, t + Vec3::new(0.0, y, 0.45), Vec3::new(0.05, 0.24, 0.02), 0.0, lin(0x0a0908), SHADE_LIT);
            }
            // A copper brazier before each tower (its flames are the gate's).
            let b = o + Vec3::new(side * 0.98, 0.0, 0.62);
            brazier_cubes(out, b - Vec3::Y * 0.09, 1.0, false);
            smoke.push((b + Vec3::new(0.0, 0.75, 0.0), 1.0));
        }
        // The piers, the arch, the tympanum in it over the doors.
        for side in [-1.0f32, 1.0] {
            block(
                out,
                o + Vec3::new(side * 0.53, 0.6, 0.0),
                Vec3::new(0.2, 1.2, 0.4),
                0.0,
                jit(PALE_STONE[1], gy + side as f64, 0.05) * 0.78,
                SHADE_STONE,
            );
            block(
                out,
                o + Vec3::new(side * 0.53, 1.21, 0.0),
                Vec3::new(0.26, 0.08, 0.44),
                0.0,
                jit(PALE_STONE[0], gx, 0.05) * 0.8,
                SHADE_STONE,
            );
        }
        let crown = arch_cubes(
            out,
            &Arch {
                centre: o + Vec3::new(0.0, 0.0, -0.02),
                across_x: true,
                r: 0.53,
                rise: 0.53,
                spring: 1.15,
                depth: 0.38,
                piers: false,
                lintel: false,
                seed: (gx, gy),
            },
        );
        for k in 0..3 {
            let w = 0.84 - k as f32 * 0.2;
            block(
                out,
                o + Vec3::new(0.0, 1.38 + k as f32 * 0.11, -0.06),
                Vec3::new(w, 0.11, 0.06),
                0.0,
                jit(OAK, k as f64, 0.1),
                SHADE_WOOD,
            );
        }
        // The wall over the arch, in courses, to a cornice and merlons.
        let wall_top = crown.y - o.y + 0.46;
        block(
            out,
            o + Vec3::new(0.0, (1.2 + wall_top) / 2.0, -0.08),
            Vec3::new(1.2, wall_top - 1.2, 0.3),
            0.0,
            jit(CHARCOAL[1], gx, 0.04) * 0.7,
            SHADE_STONE,
        );
        for j in 0..4 {
            let h = hash(gx + j as f64 * 2.3, 5.0);
            block(
                out,
                Vec3::new(o.x - 0.45 + j as f32 * 0.3, crown.y + 0.3 + (h as f32 - 0.5) * 0.02, o.z + 0.0),
                Vec3::new(0.29, 0.26, 0.34),
                (h as f32 - 0.5) * 0.04,
                if h < 0.3 { jit(UMBER[j % 2], h, 0.08) } else { jit(CHARCOAL[j % 3], h, 0.1) },
                SHADE_STONE,
            );
        }
        let cornice = crown.y - o.y + 0.5;
        block(out, o + Vec3::new(0.0, cornice, 0.0), Vec3::new(1.32, 0.08, 0.46), 0.0, jit(PALE_STONE[0], gy, 0.05) * 0.78, SHADE_STONE);
        for k in 0..5 {
            let h = hash(k as f64, gx + 9.0);
            block(
                out,
                o + Vec3::new(-0.52 + k as f32 * 0.26, cornice + 0.16, 0.0),
                Vec3::new(0.15, 0.24, 0.3),
                0.0,
                jit(CHARCOAL[k % 3], h, 0.1),
                SHADE_STONE,
            );
        }
        moss_cap(out, o + Vec3::new(0.0, cornice + 0.04, 0.0), 0.8, (gx, gy));
        // Iron sockets for the lock lamps.
        for &s in sockets {
            block(out, o + s - Vec3::new(0.0, 0.0, 0.03), Vec3::new(0.1, 0.1, 0.04), PI / 4.0, lin(IRON), SHADE_IRON);
        }
        self.authored.push((start, self.static_cubes.len() - start));
        self.smoke.extend(smoke);
    }
}
