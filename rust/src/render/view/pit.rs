//! The Pit (FINALE.md §2; `logic::pit` lays it out), to the art direction: the board's charcoal and
//! umber in rough concrete-grey stone and black iron, each level lit in its stage's colour.
//!
//! One level is a square cell of rough grey stone. Its floor is the ledge, slabs of worn concrete
//! round a square hole with an iron lip; its walls rise in uneven courses, stained under their lamps,
//! with an iron door and barred cells in each (a cot, a chain, a skull behind the bars), caged lamps
//! high on them, and the level's number carved huge in the north wall. The floor of the level above
//! is its ceiling, the hole in it right over the hole in this one, and the shaft goes on up past it
//! into the dark; the level below shows through the gap round the platform.
//!
//! In the hole the platform: a heavy stone slab paved in blocks, framed in riveted iron, and on it a
//! long oak table under a cloth, with the feast laid out (a roast pig, a fowl, a cake, bread, cheese,
//! fruit, wine, candles), picked cleaner at every level, to bones and an overturned cup at the
//! seventh. In the last seconds its iron rim glows with the bell. On the way down the platform, the
//! hero and its riders sink, the walls slide up past the camera, the next level's number comes up
//! out of the dark, and the light turns to the next stage's colour.
//!
//! The level he stands on and the one above are static scenery (its walls fade out of the camera's
//! way like the vaults'); the one below is a set the GPU keeps; the platform, the doors' leaves, the
//! numbers, the lamps and the feast move, and are drawn each frame.

use super::*;
use crate::logic::pit::{self as plan, LEVEL_DEPTH, SIZE};

pub(super) mod pal {
    pub const CONCRETE: u32 = 0x67635d;
    pub const CONCRETE_L: u32 = 0x7a756d;
    pub const CONCRETE_D: u32 = 0x524e49;
    pub const CONCRETE_DD: u32 = 0x3a3734;
    pub const FLOOR: u32 = 0x5c5953;
    pub const FLOOR_L: u32 = 0x6d6962;
    pub const FLOOR_D: u32 = 0x4a4743;
    pub const STAIN: u32 = 0x2c2a27;
    pub const IRON: u32 = 0x2b2d31;
    pub const IRON_L: u32 = 0x4c5057;
    pub const IRON_D: u32 = 0x1b1c1f;
    pub const RUST: u32 = 0x5e3420;
    pub const RUST_L: u32 = 0x7e4a2c;
    pub const VOID: u32 = 0x040303;
    pub const SLAB: u32 = 0x8a8478;
    pub const SLAB_L: u32 = 0x9a9488;
    pub const SLAB_D: u32 = 0x6c675e;
    pub const OAK: u32 = 0x2c1c10;
    pub const OAK_L: u32 = 0x3a2616;
    pub const CLOTH: u32 = 0xb2a78f;
    pub const CLOTH_D: u32 = 0x8a7e68;
    pub const WINE: u32 = 0x5a0a14;
    pub const WINE_L: u32 = 0x8a1424;
    pub const PEWTER: u32 = 0x8a8c90;
    pub const BRASS: u32 = 0xb08a3a;
    pub const ROAST: u32 = 0x7a3a14;
    pub const ROAST_L: u32 = 0xa8602a;
    pub const BREAD: u32 = 0xc08a44;
    pub const CHEESE: u32 = 0xe0b840;
    pub const RIND: u32 = 0xb08a2a;
    pub const APPLE: u32 = 0xa8201c;
    pub const GRAPE: u32 = 0x4a2a5a;
    pub const PEAR: u32 = 0xa8b040;
    pub const CAKE: u32 = 0xe6cfae;
    pub const CREAM: u32 = 0xf2ece0;
    pub const CHERRY: u32 = 0xb0182a;
    pub const BONE: u32 = 0xd8ccb0;
    pub const BONE_D: u32 = 0xa89a7a;
    pub const WAX: u32 = 0xe8dcc0;
    pub const FLAME: u32 = 0xffb050;
    pub const LAMP: u32 = 0xfff2e0;
    pub const DOOR_LAMP: u32 = 0xff3a1a;
    pub const BELL: u32 = 0xff4a20;
    pub const FLY: u32 = 0x141210;
}
use pal::*;

/// The voxel materials' shading modes for plain boxes: hammered iron, cloth, ivory, wood, dressed stone.
const SHADE_IRON: f32 = 6.0;
const SHADE_LEATHER: f32 = 9.0;
const SHADE_CLOTH: f32 = 8.0;
const SHADE_BONE: f32 = 11.0;
const SHADE_WOOD: f32 = 12.0;
const SHADE_BRASS: f32 = 13.0;
const SHADE_WET: f32 = 14.0;
const SHADE_STONE: f32 = 18.0;
/// The floor slab's thickness: the floor of a level is the ceiling of the one below.
const SLAB_T: f32 = 0.3;
/// A level's walls rise this high, to the floor of the level above.
const WALL_H: f32 = LEVEL_DEPTH - SLAB_T;
/// The platform's slab stands this far in from the hole's edge all round.
const GAP: f32 = 0.075;
/// The height of the caged lamps on the walls, and where along each wall they hang.
const LAMP_Y: f32 = 6.6;
const LAMP_AT: [f32; 3] = [4.5, 12.0, 19.5];
/// The table top's height.
const TABLE_Y: f32 = 0.8;

/// The Pit as the view needs it this frame: the level, its clock, the descent (seconds into it, and
/// how far the platform has sunk), the bell's last seconds, how far each door stands open, and
/// whether the bottom has been reached.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PitView {
    pub level: u8,
    pub clock: f32,
    pub descent: Option<f32>,
    pub sink: f32,
    pub bell: bool,
    pub doors: [f32; 8],
    pub bottom: bool,
}

/// What the view keeps of the Pit: this frame's state, and the level below, as a set.
pub(super) struct PitScene {
    pub view: PitView,
    lower: InstanceSet,
}

/// How far into the descent the light has turned toward the next level's (0 to 1).
fn toward(v: &PitView) -> f32 {
    v.sink / LEVEL_DEPTH
}

/// The colour of a level's lamps: white, tinged with its stage's light.
fn lamp_color(level: u8) -> Vec3 {
    lin(mix_hex(LAMP, plan::light(level), 0.4))
}

/// Harsh and cold from straight overhead, down the shaft through the holes; the ledges lit by their
/// lamps in the stage's colour; the shaft fading into black above and below.
pub fn pit_atmosphere(level: u8, toward: f32) -> Atmosphere {
    let c = mix_hex(plan::light(level), plan::light(level + 1), toward);
    Atmosphere {
        background: mix_hex(0x030304, c, 0.04),
        fog_near: 16.0,
        fog_far: 46.0,
        hemi_sky: lin(mix_hex(0x8a92a4, c, 0.2)) * 0.42,
        hemi_ground: lin(mix_hex(0x2a2826, c, 0.2)) * 0.5,
        ambient: lin(mix_hex(0x58565a, c, 0.2)) * 0.6,
        sun_dir: Vec3::new(0.1, 1.0, 0.16).normalize(),
        sun: lin(0xf2f4ff) * 1.35,
        exposure: 1.2,
        night_sky: 0.0,
        bloom: 0.85,
        moon_view: Vec3::new(0.0, 1.0, 0.0),
        cloud_tint: lin(0x08080a),
    }
}

/// Its ground haze: the dust of the cell, thin.
pub const HAZE: (f32, f32) = (0.016, 2.2);

// ------------------------------------------------------------------ boxes

fn bx(out: &mut Vec<Instance>, c: Vec3, s: Vec3, color: Vec3, shade: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(s, Quat::IDENTITY, c), color, 1.0, Vec3::ZERO, shade));
}

fn bxr(out: &mut Vec<Instance>, c: Vec3, s: Vec3, rot: Quat, color: Vec3, glow: Vec3, shade: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(s, rot, c), color, 1.0, glow, shade));
}

/// A rod from `a` to `b`, `w` thick.
fn rod(out: &mut Vec<Instance>, a: Vec3, b: Vec3, w: f32, color: Vec3, glow: Vec3, shade: f32) {
    let d = b - a;
    let len = d.length().max(1e-4);
    let rot = Quat::from_rotation_arc(Vec3::Y, d / len);
    out.push(Instance::new(Mat4::from_scale_rotation_translation(Vec3::new(w, len, w), rot, (a + b) * 0.5), color, 1.0, glow, shade));
}

/// A drum (the cylinder mesh) `r` across and `h` tall, standing at `c`.
fn drum(f: &mut Frame, c: Vec3, r: f32, h: f32, rot: Quat, color: Vec3, glow: Vec3, shade: f32) {
    let m = Mat4::from_scale_rotation_translation(Vec3::new(r / 0.37, h / 0.12, r / 0.37), rot, c);
    f.opaque[MeshKind::Cylinder as usize].push(Instance::new(m, color, 1.0, glow, shade));
}

/// A ball, `r` across each way.
fn ball(f: &mut Frame, c: Vec3, r: Vec3, rot: Quat, color: Vec3, glow: Vec3, shade: f32) {
    let m = Mat4::from_scale_rotation_translation(r, rot, c);
    f.opaque[MeshKind::Sphere as usize].push(Instance::new(m, color, 1.0, glow, shade));
}

/// The `k`-th draw, 0 to 1, from `seed`.
fn rnd(seed: f64, k: u32) -> f32 {
    hash(seed * 1.31 + k as f64 * 7.77 + 0.5, seed * 0.67 - k as f64 * 3.19 + 11.0) as f32
}

fn concrete(h: f32) -> Vec3 {
    let set = [CONCRETE, CONCRETE, CONCRETE_L, CONCRETE_D];
    lin(set[(h * 4.0) as usize % 4]) * (0.9 + h * 0.16)
}

/// A wall tile's way into the cell (none for a corner), and the way along the wall.
fn facing(x: i32, z: i32) -> Option<(Vec3, Vec3)> {
    let last = SIZE - 1;
    let corner = (x == 0 || x == last) && (z == 0 || z == last);
    if corner {
        return None;
    }
    let n = if z == 0 {
        Vec3::Z
    } else if z == last {
        Vec3::NEG_Z
    } else if x == 0 {
        Vec3::X
    } else {
        Vec3::NEG_X
    };
    Some((n, Vec3::new(n.z, 0.0, -n.x).abs()))
}

/// What stands in a wall tile.
#[derive(Clone, Copy, PartialEq)]
enum Feature {
    Plain,
    Door,
    Cell,
}

fn feature(x: i32, z: i32) -> Feature {
    let last = SIZE - 1;
    let along = if z == 0 || z == last { x } else { z };
    if plan::DOOR_AT.contains(&along) {
        Feature::Door
    } else if plan::CELLS.contains(&along) {
        Feature::Cell
    } else {
        Feature::Plain
    }
}

/// The lamps of a level whose floor is at `y0`: where each bulb hangs, and the way it faces.
fn lamps(y0: f32) -> Vec<(Vec3, Vec3)> {
    let last = (SIZE - 1) as f32;
    let mut out = Vec::new();
    for &a in &LAMP_AT {
        out.push((Vec3::new(a, y0 + LAMP_Y, 0.5 + 0.42), Vec3::Z));
        out.push((Vec3::new(last - 0.5 - 0.42, y0 + LAMP_Y, a), Vec3::NEG_X));
        out.push((Vec3::new(a, y0 + LAMP_Y, last - 0.5 - 0.42), Vec3::NEG_Z));
        out.push((Vec3::new(0.5 + 0.42, y0 + LAMP_Y, a), Vec3::X));
    }
    out
}

/// A rough course of wall from `y` to `y + h` on the tile at `c`, its face pushed in or out a little.
fn course(out: &mut Vec<Instance>, c: Vec3, n: Option<Vec3>, y: f32, h: f32, seed: f64, k: u32) {
    let jut = (rnd(seed, k) - 0.5) * 0.08;
    let off = n.map(|n| n * jut).unwrap_or(Vec3::ZERO);
    let col = concrete(rnd(seed, k + 100));
    bx(out, Vec3::new(c.x, y + h / 2.0, c.z) + off, Vec3::new(1.0, h - 0.025, 1.0), col, SHADE_STONE);
    // Now and then a block spalled proud of the face, or a darker one.
    if let Some(n) = n {
        let r = rnd(seed, k + 200);
        if r > 0.94 {
            let along = Vec3::new(n.z, 0.0, -n.x);
            let s = 0.22 + rnd(seed, k + 300) * 0.3;
            let p = c + along * (rnd(seed, k + 400) - 0.5) * 0.6 + n * (0.5 + jut);
            bx(
                out,
                Vec3::new(p.x, y + h * (0.3 + rnd(seed, k + 500) * 0.4), p.z),
                Vec3::new(s, s * 0.7, s) + n.abs() * -0.1 * s,
                concrete(rnd(seed, k + 600)),
                SHADE_STONE,
            );
        }
    }
}

/**
 * One copy of a level's shell at floor height `y0`: the ledge's slabs round the hole and its iron
 * lip, the walls in rough courses up to the next floor with their doors (frames, the dark behind),
 * barred cells and lamp brackets. Returns the range of each wall tile's cubes, for fading.
 */
fn shell(out: &mut Vec<Instance>, y0: f32) -> Vec<(P, usize, usize)> {
    let last = SIZE - 1;
    let r = plan::platform();
    // ---- the ledge: worn concrete slabs, stained and cracked
    for z in 1..last {
        for x in 1..last {
            let p = P::new(x, z);
            if plan::on_platform_tile(p) {
                continue;
            }
            let h = hash(x as f64 * 2.3 + 7.0, z as f64 * 1.7 + 3.0) as f32;
            let set = [FLOOR, FLOOR, FLOOR_L, FLOOR_D];
            let col = lin(set[(h * 4.0) as usize % 4]) * (0.92 + h * 0.12);
            let (xf, zf) = (x as f32, z as f32);
            bx(out, Vec3::new(xf, y0 - SLAB_T / 2.0, zf), Vec3::new(0.985, SLAB_T, 0.985), col, SHADE_STONE);
            let seed = x as f64 * 31.0 + z as f64 * 17.0;
            if h > 0.9 {
                // A dark stain, soaked in.
                let a = rnd(seed, 1) * PI;
                bxr(
                    out,
                    Vec3::new(xf + (rnd(seed, 2) - 0.5) * 0.3, y0 + 0.003, zf + (rnd(seed, 3) - 0.5) * 0.3),
                    Vec3::new(0.35 + rnd(seed, 4) * 0.4, 0.004, 0.25 + rnd(seed, 5) * 0.3),
                    Quat::from_rotation_y(a),
                    col.lerp(lin(STAIN), 0.45),
                    Vec3::ZERO,
                    SHADE_STONE,
                );
            } else if h < 0.12 {
                // A crack across the slab.
                let a = rnd(seed, 6) * PI;
                bxr(
                    out,
                    Vec3::new(xf, y0 + 0.003, zf),
                    Vec3::new(0.8, 0.004, 0.025),
                    Quat::from_rotation_y(a),
                    col.lerp(lin(STAIN), 0.7),
                    Vec3::ZERO,
                    SHADE_STONE,
                );
            }
            // Grit and chips by the walls.
            let by_wall = x == 1 || z == 1 || x == last - 1 || z == last - 1;
            if by_wall && rnd(seed, 7) > 0.55 {
                for k in 0..3 {
                    let s = 0.04 + rnd(seed, 10 + k) * 0.07;
                    bx(
                        out,
                        Vec3::new(xf + (rnd(seed, 20 + k) - 0.5) * 0.8, y0 + s / 2.0, zf + (rnd(seed, 30 + k) - 0.5) * 0.8),
                        Vec3::splat(s),
                        concrete(rnd(seed, 40 + k)) * 0.85,
                        SHADE_STONE,
                    );
                }
            }
        }
    }
    // The hole's iron lip: an angle along each edge, riveted.
    let (lo, hi) = (r.x0 as f32 - 0.5, r.x1 as f32 + 0.5);
    let mid = (lo + hi) / 2.0;
    let len = hi - lo + 0.24;
    for (c, s) in [
        (Vec3::new(mid, y0 - 0.005, lo - 0.06), Vec3::new(len, 0.03, 0.14)),
        (Vec3::new(mid, y0 - 0.005, hi + 0.06), Vec3::new(len, 0.03, 0.14)),
        (Vec3::new(lo - 0.06, y0 - 0.005, mid), Vec3::new(0.14, 0.03, len)),
        (Vec3::new(hi + 0.06, y0 - 0.005, mid), Vec3::new(0.14, 0.03, len)),
    ] {
        bx(out, c, s, lin(IRON), SHADE_IRON);
    }
    for k in 0..12 {
        let t = lo + 0.45 + k as f32 * (hi - lo - 0.9) / 11.0;
        for p in [
            Vec3::new(t, y0 + 0.015, lo - 0.06),
            Vec3::new(t, y0 + 0.015, hi + 0.06),
            Vec3::new(lo - 0.06, y0 + 0.015, t),
            Vec3::new(hi + 0.06, y0 + 0.015, t),
        ] {
            bx(out, p, Vec3::new(0.05, 0.025, 0.05), lin(IRON_L), SHADE_IRON);
        }
    }
    // The hole's sides, down through the slab.
    for (c, s) in [
        (Vec3::new(mid, y0 - SLAB_T / 2.0, lo - 0.01), Vec3::new(len - 0.2, SLAB_T, 0.02)),
        (Vec3::new(mid, y0 - SLAB_T / 2.0, hi + 0.01), Vec3::new(len - 0.2, SLAB_T, 0.02)),
        (Vec3::new(lo - 0.01, y0 - SLAB_T / 2.0, mid), Vec3::new(0.02, SLAB_T, len - 0.2)),
        (Vec3::new(hi + 0.01, y0 - SLAB_T / 2.0, mid), Vec3::new(0.02, SLAB_T, len - 0.2)),
    ] {
        bx(out, c, s, lin(IRON_D), SHADE_IRON);
    }

    // ---- the walls
    let mut ranges = Vec::new();
    for z in 0..SIZE {
        for x in 0..SIZE {
            if plan::inside(P::new(x, z)) {
                continue;
            }
            let start = out.len();
            let c = Vec3::new(x as f32, y0, z as f32);
            let seed = x as f64 * 13.0 + z as f64 * 29.0 + 5.0;
            let face = facing(x, z);
            let feat = if face.is_some() { feature(x, z) } else { Feature::Plain };
            let open_top = match feat {
                Feature::Door => 2.45,
                Feature::Cell => 2.3,
                Feature::Plain => 0.0,
            };
            let n = face.map(|f| f.0);
            if let Some((n, along)) = face {
                match feat {
                    Feature::Door => door_frame(out, c, n, along, seed),
                    Feature::Cell => barred_cell(out, c, n, along, seed),
                    Feature::Plain => {
                        // A heavy plinth course, darker, proud of the face.
                        bx(out, c + Vec3::new(0.0, 0.21, 0.0) + n * 0.04, Vec3::new(1.0, 0.42, 1.0), lin(CONCRETE_DD), SHADE_STONE);
                    }
                }
            }
            let mut y = if feat == Feature::Plain { 0.42 } else { open_top };
            let mut k = 0;
            while y < WALL_H - 0.05 {
                let h = (0.5 + rnd(seed, k) * 0.38).min(WALL_H - y);
                course(out, c, n, y0 + y, h, seed, k);
                y += h;
                k += 1;
            }
            if let Some((n, along)) = face {
                // Rust bleeding down the face from the lamps, and from the doors' frames.
                let a = if z == 0 || z == SIZE - 1 { x as f32 } else { z as f32 };
                if LAMP_AT.iter().any(|&l| (l - a).abs() < 0.6) {
                    let p = c + n * 0.505;
                    bx(
                        out,
                        Vec3::new(p.x, y0 + LAMP_Y - 1.4, p.z),
                        Vec3::new(0.0, 2.4, 0.0) + along * 0.12 + n.abs() * 0.01,
                        lin(RUST) * 0.8,
                        SHADE_LIT,
                    );
                }
                // A course of darker stone just under the ceiling.
                let p = c + n * 0.03;
                bx(out, Vec3::new(p.x, y0 + WALL_H - 0.2, p.z), Vec3::new(1.0, 0.4, 1.0), lin(CONCRETE_DD), SHADE_STONE);
            }
            ranges.push((P::new(x, z), start, out.len() - start));
        }
    }
    // ---- the lamps' brackets and cages (their bulbs are drawn lit each frame)
    for (bulb, n) in lamps(y0) {
        let wall = bulb - n * 0.42;
        rod(out, wall + Vec3::new(0.0, 0.25, 0.0), bulb + Vec3::new(0.0, 0.25, 0.0), 0.05, lin(IRON), Vec3::ZERO, SHADE_IRON);
        bx(out, bulb + Vec3::new(0.0, 0.2, 0.0), Vec3::new(0.32, 0.06, 0.32), lin(IRON_D), SHADE_IRON);
        bx(out, bulb + Vec3::new(0.0, -0.17, 0.0), Vec3::new(0.2, 0.04, 0.2), lin(IRON_D), SHADE_IRON);
        for k in 0..4 {
            let a = k as f32 * TAU / 4.0 + PI / 4.0;
            let p = bulb + Vec3::new(a.cos() * 0.13, 0.01, a.sin() * 0.13);
            bx(out, p, Vec3::new(0.022, 0.36, 0.022), lin(IRON), SHADE_IRON);
        }
        bx(
            out,
            wall + Vec3::new(0.0, 0.25, 0.0) + n * 0.02,
            Vec3::new(0.18, 0.24, 0.18) * (Vec3::ONE - n.abs() * 0.8),
            lin(IRON_D),
            SHADE_IRON,
        );
    }
    ranges
}

/// A door's frame in its wall tile: iron jambs and lintel, the alcove's stone sides and sill, the
/// dark behind where the leaf rises, and the red lamp over it (lit while it stands open).
fn door_frame(out: &mut Vec<Instance>, c: Vec3, n: Vec3, along: Vec3, seed: f64) {
    let face = c + n * 0.5;
    let h = 2.25;
    for s in [-1.0f32, 1.0] {
        // The alcove's sides (stone), and the iron jamb at the face.
        let side = c + along * s * 0.47;
        bx(
            out,
            side + Vec3::new(0.0, h / 2.0, 0.0),
            Vec3::new(1.0, h, 1.0) * (along * 0.06 + n.abs() * 1.0 + Vec3::Y),
            concrete(rnd(seed, 1)),
            SHADE_STONE,
        );
        let jamb = face + along * s * 0.44 + Vec3::new(0.0, h / 2.0, 0.0);
        bx(out, jamb, along * 0.1 + n.abs() * 0.12 + Vec3::new(0.0, h, 0.0), lin(IRON), SHADE_IRON);
        for k in 0..5 {
            bx(out, jamb + Vec3::new(0.0, -0.9 + k as f32 * 0.45, 0.0) + n * 0.065, Vec3::splat(0.045), lin(IRON_L), SHADE_IRON);
        }
    }
    bx(out, face + Vec3::new(0.0, h + 0.08, 0.0), along * 1.0 + n.abs() * 0.14 + Vec3::new(0.0, 0.16, 0.0), lin(IRON), SHADE_IRON);
    bx(out, face + Vec3::new(0.0, 0.015, 0.0), along * 0.84 + n.abs() * 0.3 + Vec3::new(0.0, 0.03, 0.0), lin(IRON_D), SHADE_IRON);
    // The dark behind.
    let back = c - n * 0.2;
    out.push(Instance::new(
        Mat4::from_scale_rotation_translation(
            along * 0.9 + n.abs() * 0.05 + Vec3::new(0.0, h, 0.0),
            Quat::IDENTITY,
            back + Vec3::new(0.0, h / 2.0, 0.0),
        ),
        lin(VOID),
        1.0,
        Vec3::ZERO,
        SHADE_UNLIT,
    ));
    // The red lamp's housing over the lintel.
    bx(
        out,
        face + Vec3::new(0.0, h + 0.36, 0.0) + n * 0.04,
        along * 0.22 + n.abs() * 0.12 + Vec3::new(0.0, 0.14, 0.0),
        lin(IRON_D),
        SHADE_IRON,
    );
}

/// A barred cell in its wall tile: a dark stone alcove with a plank cot, a chain on the back wall
/// and a skull in the straw, behind iron bars.
fn barred_cell(out: &mut Vec<Instance>, c: Vec3, n: Vec3, along: Vec3, seed: f64) {
    let face = c + n * 0.5;
    let h = 2.2;
    let depth = 0.85;
    let back = face - n * depth;
    // The alcove: back wall, sides, ceiling, floor.
    bx(
        out,
        back - n * 0.05 + Vec3::new(0.0, h / 2.0, 0.0),
        along * 1.0 + n.abs() * 0.1 + Vec3::new(0.0, h, 0.0),
        lin(CONCRETE_DD) * 0.7,
        SHADE_STONE,
    );
    for s in [-1.0f32, 1.0] {
        let side = face - n * depth / 2.0 + along * s * 0.47;
        bx(
            out,
            side + Vec3::new(0.0, h / 2.0, 0.0),
            along * 0.06 + n.abs() * depth + Vec3::new(0.0, h, 0.0),
            lin(CONCRETE_D) * 0.75,
            SHADE_STONE,
        );
    }
    bx(
        out,
        face - n * depth / 2.0 + Vec3::new(0.0, h + 0.04, 0.0),
        along * 1.0 + n.abs() * depth + Vec3::new(0.0, 0.08, 0.0),
        lin(CONCRETE_DD),
        SHADE_STONE,
    );
    bx(
        out,
        face - n * depth / 2.0 + Vec3::new(0.0, 0.01, 0.0),
        along * 0.9 + n.abs() * depth + Vec3::new(0.0, 0.02, 0.0),
        lin(0x4a4030),
        SHADE_CLOTH,
    );
    // A plank cot against one side, a chain hanging from the back wall, a skull in the straw.
    let s = if rnd(seed, 3) > 0.5 { 1.0 } else { -1.0 };
    bx(
        out,
        face - n * depth / 2.0 + along * s * 0.28 + Vec3::new(0.0, 0.38, 0.0),
        along * 0.3 + n.abs() * 0.7 + Vec3::new(0.0, 0.06, 0.0),
        lin(OAK),
        SHADE_WOOD,
    );
    for k in 0..2 {
        let leg = face - n * (0.15 + k as f32 * 0.55) + along * s * 0.28 + Vec3::new(0.0, 0.18, 0.0);
        bx(out, leg, Vec3::new(0.06, 0.36, 0.06), lin(OAK) * 0.8, SHADE_WOOD);
    }
    let ring = back + n * 0.04 + along * -s * 0.2 + Vec3::new(0.0, 1.5, 0.0);
    for k in 0..6 {
        let p = ring - Vec3::new(0.0, k as f32 * 0.12, 0.0) + n * (k as f32 * 0.01);
        let rot = Quat::from_rotation_y(if k % 2 == 0 { 0.0 } else { PI / 2.0 });
        bxr(out, p, Vec3::new(0.07, 0.11, 0.02), rot, lin(IRON_L), Vec3::ZERO, SHADE_IRON);
    }
    if rnd(seed, 4) > 0.4 {
        let sk = face - n * 0.55 + along * -s * 0.18 + Vec3::new(0.0, 0.08, 0.0);
        bx(out, sk, Vec3::new(0.16, 0.14, 0.18), lin(BONE_D), SHADE_BONE);
        bx(
            out,
            sk + n * 0.09 + Vec3::new(0.0, -0.03, 0.0),
            Vec3::new(0.12, 0.06, 0.08) * (along + Vec3::Y + n.abs()),
            lin(BONE_D) * 0.9,
            SHADE_BONE,
        );
        for e in [-1.0f32, 1.0] {
            bx(out, sk + n * 0.092 + along * e * 0.04 + Vec3::new(0.0, 0.02, 0.0), Vec3::splat(0.035), lin(VOID), SHADE_UNLIT);
        }
    }
    // The bars: six rods, a band across at knee and shoulder, rails top and bottom.
    for k in 0..6 {
        let t = -0.38 + k as f32 * 0.152;
        let p = face + along * t + Vec3::new(0.0, h / 2.0, 0.0);
        bx(out, p, Vec3::new(0.045, h, 0.045), lin(IRON), SHADE_IRON);
    }
    for y in [0.05, 0.75, 1.55, h - 0.04] {
        bx(
            out,
            face + Vec3::new(0.0, y, 0.0) + n * 0.01,
            along * 0.92 + n.abs() * 0.06 + Vec3::new(0.0, 0.07, 0.0),
            lin(IRON_D),
            SHADE_IRON,
        );
    }
}

/// The digits nought to nine in a three-by-five grid, top row first: square strokes, as a chisel
/// cuts them.
const DIGITS: [[&str; 5]; 10] = [
    ["###", "#.#", "#.#", "#.#", "###"],
    [".#.", "##.", ".#.", ".#.", "###"],
    ["###", "..#", "###", "#..", "###"],
    ["###", "..#", ".##", "..#", "###"],
    ["#.#", "#.#", "###", "..#", "..#"],
    ["###", "#..", "###", "..#", "###"],
    ["###", "#..", "###", "#.#", "###"],
    ["###", "..#", "..#", "..#", "..#"],
    ["###", "#.#", "###", "#.#", "###"],
    ["###", "#.#", "###", "..#", "###"],
];

impl WorldView {
    /// The Pit's static scene: this level's shell and the one above (its floor this one's ceiling);
    /// the one below as a set.
    pub(super) fn build_pit(&mut self, _grid: &Grid) {
        let ranges = shell(&mut self.static_cubes, 0.0);
        for (p, s, len) in ranges {
            self.rock_index.insert(tile_key(p.x, p.y), (s, len));
            let shades: Vec<f32> = self.static_cubes[s..s + len].iter().map(|i| i.emissive[3]).collect();
            self.trees.push(fade::Tree {
                range: (s, len),
                base: Vec3::new(p.x as f32, 0.0, p.y as f32),
                radius: 0.72,
                height: WALL_H,
                fade: 1.0,
                alive: true,
                shades: Some(shades),
            });
        }
        shell(&mut self.static_cubes, LEVEL_DEPTH);
        let mut lower = Vec::new();
        shell(&mut lower, -LEVEL_DEPTH);
        self.pit = Some(PitScene { view: PitView { level: 1, ..PitView::default() }, lower: InstanceSet::new(lower) });
    }

    /// This frame's Pit, from the game.
    pub(super) fn update_pit_view(&mut self, s: &SceneState) {
        if let (Some(p), Some(v)) = (self.pit.as_mut(), s.pit) {
            p.view = v;
        }
    }

    /// How far the platform (and all on it) has sunk below its level, in world units.
    pub(super) fn pit_sink(&self) -> f32 {
        self.pit.as_ref().map(|p| p.view.sink).unwrap_or(0.0)
    }

    /// How far the chase camera's pitch lifts on the way down (swelling and fading with the descent).
    pub(super) fn pit_tilt(&self) -> f32 {
        let Some(t) = self.pit.as_ref().and_then(|p| p.view.descent) else { return 0.0 };
        let k = (t / plan::DESCENT_SECONDS as f32).clamp(0.0, 1.0);
        (k * PI).sin() * 0.3
    }

    /// How far over the hole the top camera has moved on the way down (0 to 1): there once the
    /// platform lets go, back as it comes to rest.
    pub(super) fn pit_overhead(&self) -> f32 {
        let Some(t) = self.pit.as_ref().and_then(|p| p.view.descent) else { return 0.0 };
        let (shudder, end) = (plan::SHUDDER_SECONDS as f32, plan::DESCENT_SECONDS as f32);
        let k = ((t - shudder * 0.5) / 0.6).clamp(0.0, 1.0) * (1.0 - ((t - (end - 0.5)) / 0.5).clamp(0.0, 1.0));
        k * k * (3.0 - 2.0 * k)
    }

    /// The middle of the platform, at height `y`.
    pub(super) fn pit_middle(&self, y: f32) -> Vec3 {
        let r = plan::platform();
        Vec3::new((r.x0 + r.x1) as f32 / 2.0, y, (r.y0 + r.y1) as f32 / 2.0)
    }

    /// How far below its floor a body at `p` is drawn: sunk with the platform if it stands on it.
    pub(super) fn pit_sunk_at(&self, p: V) -> f32 {
        let sink = self.pit_sink();
        if sink > 0.0 && plan::on_platform(p) {
            sink
        } else {
            0.0
        }
    }

    /// The Pit's light, if he is in it: the level's, turning toward the next one's on the way down.
    pub(super) fn pit_atmosphere(&self) -> Option<Atmosphere> {
        let v = self.pit.as_ref()?.view;
        Some(pit_atmosphere(v.level, toward(&v)))
    }

    /// The Pit's moving parts and lights, each frame.
    pub(super) fn emit_pit(&self, s: &SceneState, time: f32, hero: Vec3, f: &mut Frame) {
        let Some(p) = self.pit.as_ref() else { return };
        let v = p.view;
        f.sets.push(p.lower.clone());
        let k = toward(&v);
        // The lamps and numbers of this level, the one above and the one below.
        for (copy, y0) in [(0i32, 0.0f32), (1, LEVEL_DEPTH), (-1, -LEVEL_DEPTH)] {
            let level = v.level as i32 - copy;
            if !(1..=plan::LEVELS as i32 + 1).contains(&level) {
                continue;
            }
            // (Below the seventh lies the bottom: no level, no number, no lamps.)
            if level > plan::LEVELS as i32 {
                continue;
            }
            let bright = match copy {
                0 => 1.0 - 0.45 * k,
                1 => 0.3,
                _ => 0.35 + 0.65 * k,
            };
            self.emit_pit_number(f, level as u8, y0);
            self.emit_pit_lamps(f, level as u8, y0, bright, time);
            self.emit_pit_doors(f, if copy == 0 { v.doors } else { [0.0; 8] }, y0, time);
        }
        self.emit_pit_platform(f, &v, s, time);
        self.emit_pit_motes(f, time, hero);
    }

    /// The level's number carved huge in the north wall: deep dark grooves, a pale lip along their
    /// tops, old paint of the level's colour in them.
    fn emit_pit_number(&self, f: &mut Frame, level: u8, y0: f32) {
        let rows = DIGITS[(level as usize).min(9)];
        let on = |r: i32, c: i32| r >= 0 && c >= 0 && r < 5 && c < 3 && rows[r as usize].as_bytes()[c as usize] == b'#';
        let (px, py) = (1.1, 1.15);
        let face = 0.56;
        let x0 = SIZE as f32 / 2.0 - 0.5 - px;
        let top = y0 + 6.0;
        let paint = lin(plan::light(level));
        let lip = lin(CONCRETE_L) * 1.3;
        let shade = lin(0x0a0908);
        let cube = &mut f.opaque[MeshKind::Cube as usize];
        let mut put = |c: Vec3, s: Vec3, color: Vec3, glow: Vec3, mode: f32| {
            cube.push(Instance::new(Mat4::from_scale_rotation_translation(s, Quat::IDENTITY, c), color, 1.0, glow, mode));
        };
        for r in 0..5i32 {
            for c in 0..3i32 {
                if !on(r, c) {
                    continue;
                }
                let x = x0 + c as f32 * px;
                let y = top - r as f32 * py;
                // The groove, deep and dark.
                put(Vec3::new(x, y, face), Vec3::new(px + 0.005, py + 0.005, 0.02), lin(0x161411), Vec3::ZERO, SHADE_STONE);
                // Its bevels: a pale lip where the light from above catches its top and left edges,
                // shadow along its bottom and right.
                if !on(r - 1, c) {
                    put(Vec3::new(x, y + py / 2.0 - 0.04, face + 0.015), Vec3::new(px, 0.08, 0.02), lip, Vec3::ZERO, SHADE_STONE);
                }
                if !on(r + 1, c) {
                    put(Vec3::new(x, y - py / 2.0 + 0.035, face + 0.012), Vec3::new(px, 0.07, 0.02), shade, Vec3::ZERO, SHADE_LIT);
                }
                if !on(r, c - 1) {
                    put(Vec3::new(x - px / 2.0 + 0.035, y, face + 0.013), Vec3::new(0.07, py, 0.02), lip * 0.8, Vec3::ZERO, SHADE_STONE);
                }
                if !on(r, c + 1) {
                    put(Vec3::new(x + px / 2.0 - 0.035, y, face + 0.012), Vec3::new(0.07, py, 0.02), shade, Vec3::ZERO, SHADE_LIT);
                }
                // Old paint of the level's colour left down the stroke, running on into its neighbours.
                let (l, r_) = (if on(r, c - 1) { px / 2.0 } else { px * 0.24 }, if on(r, c + 1) { px / 2.0 } else { px * 0.24 });
                let (u, d) = (if on(r - 1, c) { py / 2.0 } else { py * 0.24 }, if on(r + 1, c) { py / 2.0 } else { py * 0.24 });
                let centre = Vec3::new(x + (r_ - l) / 2.0, y + (u - d) / 2.0, face + 0.011);
                put(centre, Vec3::new(l + r_, u + d, 0.012), paint * 0.22, paint * 0.025, SHADE_LIT);
            }
        }
    }

    /// A level's lamps lit in its colour, `bright` of their full strength, flickering a little.
    fn emit_pit_lamps(&self, f: &mut Frame, level: u8, y0: f32, bright: f32, time: f32) {
        let color = lamp_color(level);
        for (i, (bulb, n)) in lamps(y0).into_iter().enumerate() {
            let fi = i as f32;
            let flick = 0.94 + (time * 7.3 + fi * 1.9).sin() * 0.03 + (time * 23.0 + fi).sin() * 0.03;
            let b = bright * flick;
            ball(f, bulb, Vec3::splat(0.09), Quat::IDENTITY, color, color * 3.0 * b, SHADE_UNLIT);
            glow_orb(f, bulb, 0.55, color, 0.22 * b);
            f.lights.push(Light { pos: bulb + n * 0.25 - Vec3::new(0.0, 0.25, 0.0), color: color * 9.0 * b, distance: 13.0 });
        }
    }

    /// The doors' iron leaves, risen into the wall as far as each stands open, and the red lamps over
    /// those open.
    fn emit_pit_doors(&self, f: &mut Frame, open: [f32; 8], y0: f32, time: f32) {
        for (i, d) in plan::doors().iter().enumerate() {
            let (n, along) = (d.facing.vecf(), d.facing.vecf());
            let n = Vec3::new(n.x as f32, 0.0, n.y as f32);
            let along = Vec3::new(along.y.abs() as f32, 0.0, along.x.abs() as f32);
            let face = Vec3::new(d.tile.x as f32, y0, d.tile.y as f32) + n * 0.5;
            let lift = open[i] * 2.15;
            let leaf = face - n * 0.12 + Vec3::new(0.0, 1.11 + lift, 0.0);
            let cube = &mut f.opaque[MeshKind::Cube as usize];
            let size = along * 0.84 + n.abs() * 0.08 + Vec3::new(0.0, 2.2, 0.0);
            cube.push(Instance::new(
                Mat4::from_scale_rotation_translation(size, Quat::IDENTITY, leaf),
                lin(IRON),
                1.0,
                Vec3::ZERO,
                SHADE_IRON,
            ));
            // Its plates' seams and rivets, and the barred slot at eye height.
            for y in [-0.7f32, 0.0, 0.7] {
                let p = leaf + Vec3::new(0.0, y, 0.0) + n * 0.045;
                cube.push(Instance::new(
                    Mat4::from_scale_rotation_translation(along * 0.8 + n.abs() * 0.02 + Vec3::new(0.0, 0.05, 0.0), Quat::IDENTITY, p),
                    lin(IRON_L),
                    1.0,
                    Vec3::ZERO,
                    SHADE_IRON,
                ));
            }
            let slot = leaf + Vec3::new(0.0, 0.55, 0.0) + n * 0.042;
            cube.push(Instance::new(
                Mat4::from_scale_rotation_translation(along * 0.36 + n.abs() * 0.01 + Vec3::new(0.0, 0.12, 0.0), Quat::IDENTITY, slot),
                lin(VOID),
                1.0,
                Vec3::ZERO,
                SHADE_UNLIT,
            ));
            let rust = leaf + Vec3::new(0.0, -0.85, 0.0) + n * 0.043;
            cube.push(Instance::new(
                Mat4::from_scale_rotation_translation(along * 0.7 + n.abs() * 0.01 + Vec3::new(0.0, 0.4, 0.0), Quat::IDENTITY, rust),
                lin(RUST),
                1.0,
                Vec3::ZERO,
                SHADE_IRON,
            ));
            // The red lamp: lit while it stands open, blinking.
            let lamp = face + Vec3::new(0.0, 2.25 + 0.36, 0.0) + n * 0.11;
            let on = open[i] > 0.0;
            let blink = if on { 0.6 + 0.4 * (time * 9.0).sin().abs() } else { 0.0 };
            let red = lin(DOOR_LAMP);
            cube.push(Instance::new(
                Mat4::from_scale_rotation_translation(along * 0.16 + n.abs() * 0.04 + Vec3::new(0.0, 0.09, 0.0), Quat::IDENTITY, lamp),
                red * 0.4,
                1.0,
                red * 3.0 * blink,
                SHADE_UNLIT,
            ));
            if on {
                glow_orb(f, lamp + n * 0.05, 0.4, red, 0.4 * blink);
                f.lights.push(Light { pos: lamp + n * 0.4, color: red * 4.0 * blink, distance: 5.0 });
            }
        }
    }

    /**
     * The platform where it is (sunk on the way down): its slab paved in stone blocks, its iron frame
     * and rivets (glowing with the bell in the last seconds), and the table with the feast on it.
     */
    fn emit_pit_platform(&self, f: &mut Frame, v: &PitView, _s: &SceneState, time: f32) {
        let y = -v.sink;
        let r = plan::platform();
        let (lo, hi) = (r.x0 as f32 - 0.5 + GAP, r.x1 as f32 + 0.5 - GAP);
        let mid = (lo + hi) / 2.0;
        let w = hi - lo;
        let rim = 0.34;
        // The bell: the frame glows each second, brighter as the end comes; on the way down a dull heat.
        let glow = if v.bell {
            let left = plan::LEVEL_SECONDS as f32 - v.clock;
            let beat = 1.0 - left.fract();
            let urgency = 1.0 - (left / plan::BELL_SECONDS as f32).clamp(0.0, 1.0);
            (0.25 + 0.75 * beat.powi(3)) * (0.5 + urgency)
        } else if let Some(t) = v.descent {
            (1.0 - t / 1.5).max(0.0) * 0.8
        } else {
            0.0
        };
        let ember = lin(BELL) * glow * 2.2;
        let out = &mut f.opaque[MeshKind::Cube as usize];
        // The slab's body under its paving.
        bx(out, Vec3::new(mid, y - 0.45, mid), Vec3::new(w, 0.82, w), lin(SLAB_D), SHADE_STONE);
        // The paving, in blocks with dark joints, worn paler toward the middle.
        let (plo, phi) = (lo + rim, hi - rim);
        for z in r.y0..=r.y1 {
            for x in r.x0..=r.x1 {
                let (x0, x1) = ((x as f32 - 0.5).max(plo), (x as f32 + 0.5).min(phi));
                let (z0, z1) = ((z as f32 - 0.5).max(plo), (z as f32 + 0.5).min(phi));
                if x1 - x0 < 0.05 || z1 - z0 < 0.05 {
                    continue;
                }
                let h = hash(x as f64 * 3.7 + 1.0, z as f64 * 2.9 + 4.0) as f32;
                let set = [SLAB, SLAB_L, SLAB, SLAB_D];
                let col = lin(set[(h * 4.0) as usize % 4]) * (0.9 + h * 0.14);
                bx(
                    out,
                    Vec3::new((x0 + x1) / 2.0, y - 0.04, (z0 + z1) / 2.0),
                    Vec3::new(x1 - x0 - 0.03, 0.08, z1 - z0 - 0.03),
                    col,
                    SHADE_STONE,
                );
            }
        }
        // The iron frame round it: a riveted band on top, the slab's sides plated.
        for (c, s) in [
            (Vec3::new(mid, y - 0.035, lo + rim / 2.0), Vec3::new(w, 0.08, rim)),
            (Vec3::new(mid, y - 0.035, hi - rim / 2.0), Vec3::new(w, 0.08, rim)),
            (Vec3::new(lo + rim / 2.0, y - 0.035, mid), Vec3::new(rim, 0.08, w - rim * 2.0)),
            (Vec3::new(hi - rim / 2.0, y - 0.035, mid), Vec3::new(rim, 0.08, w - rim * 2.0)),
        ] {
            out.push(Instance::new(Mat4::from_scale_rotation_translation(s, Quat::IDENTITY, c), lin(IRON), 1.0, ember, SHADE_IRON));
        }
        for (c, s) in [
            (Vec3::new(mid, y - 0.42, lo - 0.01), Vec3::new(w + 0.02, 0.76, 0.03)),
            (Vec3::new(mid, y - 0.42, hi + 0.01), Vec3::new(w + 0.02, 0.76, 0.03)),
            (Vec3::new(lo - 0.01, y - 0.42, mid), Vec3::new(0.03, 0.76, w)),
            (Vec3::new(hi + 0.01, y - 0.42, mid), Vec3::new(0.03, 0.76, w)),
        ] {
            bx(out, c, s, lin(IRON_D), SHADE_IRON);
        }
        let n = 18;
        for k in 0..=n {
            let t = lo + 0.17 + k as f32 * (w - 0.34) / n as f32;
            for p in [
                Vec3::new(t, y + 0.02, lo + 0.1),
                Vec3::new(t, y + 0.02, hi - 0.1),
                Vec3::new(lo + 0.1, y + 0.02, t),
                Vec3::new(hi - 0.1, y + 0.02, t),
            ] {
                out.push(Instance::new(
                    Mat4::from_scale_rotation_translation(Vec3::new(0.06, 0.04, 0.06), Quat::IDENTITY, p),
                    lin(IRON_L),
                    1.0,
                    ember * 0.6,
                    SHADE_IRON,
                ));
            }
        }
        for (cx, cz) in [(lo, lo), (hi, lo), (lo, hi), (hi, hi)] {
            let c = Vec3::new(cx + if cx < mid { 0.3 } else { -0.3 }, y + 0.0, cz + if cz < mid { 0.3 } else { -0.3 });
            out.push(Instance::new(
                Mat4::from_scale_rotation_translation(Vec3::new(0.62, 0.1, 0.62), Quat::IDENTITY, c),
                lin(IRON_D),
                1.0,
                ember,
                SHADE_IRON,
            ));
            bx(out, c + Vec3::new(0.0, 0.07, 0.0), Vec3::new(0.18, 0.06, 0.18), lin(RUST_L), SHADE_IRON);
            // The guide shoes it rides the shaft by, down the corners.
            bx(out, Vec3::new(cx, y - 0.4, cz), Vec3::new(0.16, 0.8, 0.16), lin(IRON_L), SHADE_IRON);
        }
        if glow > 0.05 {
            for (cx, cz) in [(lo + 0.6, lo + 0.6), (hi - 0.6, lo + 0.6), (lo + 0.6, hi - 0.6), (hi - 0.6, hi - 0.6)] {
                f.lights.push(Light { pos: Vec3::new(cx, y + 0.3, cz), color: lin(BELL) * 5.0 * glow, distance: 4.5 });
            }
        }
        self.emit_pit_table(f, v.level, y, time);
    }

    /// The long table and the feast on it, as much of it as is left at this level.
    fn emit_pit_table(&self, f: &mut Frame, level: u8, y: f32, time: f32) {
        let t = plan::table();
        let (x0, x1) = (t.x0 as f32 - 0.5, t.x1 as f32 + 0.5);
        let cz = t.y0 as f32;
        let cx = (x0 + x1) / 2.0;
        let len = x1 - x0 - 0.1;
        let top = y + TABLE_Y;
        let k = plan::feast_left(level);
        let out = &mut f.opaque[MeshKind::Cube as usize];
        // The oak: three thick planks, and trestles of stone.
        for (i, dz) in [-0.4f32, 0.0, 0.4].into_iter().enumerate() {
            let col = lin(if i == 1 { OAK_L } else { OAK }) * (0.95 + i as f32 * 0.04);
            bx(out, Vec3::new(cx, top - 0.06, cz + dz), Vec3::new(len, 0.12, 0.39), col, SHADE_WOOD);
        }
        for tx in [x0 + 0.5, cx, x1 - 0.5] {
            bx(out, Vec3::new(tx, y + TABLE_Y / 2.0 - 0.06, cz), Vec3::new(0.3, TABLE_Y - 0.12, 0.95), lin(SLAB_D), SHADE_STONE);
            bx(out, Vec3::new(tx, y + 0.06, cz), Vec3::new(0.46, 0.12, 1.1), lin(SLAB_D) * 0.9, SHADE_STONE);
        }
        // The cloth, its hems hanging over the long sides; stained with each level, and dragged
        // askew and torn by the last.
        let skew = (1.0 - k) * 0.12;
        let cloth = lin(CLOTH).lerp(lin(CLOTH_D), 1.0 - k);
        let rot = Quat::from_rotation_y(skew * 0.3);
        bxr(out, Vec3::new(cx + skew, top + 0.008, cz), Vec3::new(len - 0.3 - skew, 0.016, 1.3), rot, cloth, Vec3::ZERO, SHADE_CLOTH);
        for s in [-1.0f32, 1.0] {
            bxr(
                out,
                Vec3::new(cx + skew, top - 0.16, cz + s * 0.66),
                Vec3::new(len - 0.3 - skew * 2.0, 0.34, 0.02),
                rot,
                cloth * 0.92,
                Vec3::ZERO,
                SHADE_CLOTH,
            );
        }
        let stains = ((1.0 - k) * 7.0).round() as u32;
        for i in 0..stains {
            let a = rnd(91.0, i) * PI;
            let p = Vec3::new(x0 + 0.5 + rnd(93.0, i) * (len - 1.0), top + 0.018, cz + (rnd(95.0, i) - 0.5) * 1.0);
            bxr(
                out,
                p,
                Vec3::new(0.18 + rnd(97.0, i) * 0.25, 0.004, 0.12 + rnd(99.0, i) * 0.15),
                Quat::from_rotation_y(a),
                lin(WINE) * 0.9,
                Vec3::ZERO,
                SHADE_CLOTH,
            );
        }
        self.emit_pit_feast(f, Vec3::new(cx, top + 0.02, cz), x0, x1, k, level, y, time);
    }

    /**
     * The feast at `k` of its fill (1 at the first level, 0 at the seventh): a roast pig on its
     * platter in the middle, a fowl, a tiered cake, loaves, cheese, bowls of fruit, a wine jug,
     * goblets and plates, two candelabra. Each goes as the levels go, to bones on its platter, and
     * at the last an overturned cup with its wine run off the edge.
     */
    #[allow(clippy::too_many_arguments)]
    fn emit_pit_feast(&self, f: &mut Frame, c: Vec3, x0: f32, x1: f32, k: f32, level: u8, floor: f32, time: f32) {
        let up = Quat::IDENTITY;
        let pewter = lin(PEWTER);
        let near = c.z + 0.38;
        let far = c.z - 0.38;
        // Plates, in two rows; pushed about once the eating is done.
        for (i, x) in [x0 + 0.65, x0 + 1.8, x1 - 1.8, x1 - 0.65].into_iter().enumerate() {
            for (j, z) in [near, far].into_iter().enumerate() {
                let n = (i * 2 + j) as u32;
                let off = (1.0 - k) * Vec3::new((rnd(5.0, n) - 0.5) * 0.25, 0.0, (rnd(7.0, n) - 0.5) * 0.15);
                drum(f, Vec3::new(x, c.y + 0.012, z) + off, 0.16, 0.025, up, pewter, Vec3::ZERO, SHADE_IRON);
            }
        }
        // The roast pig on its great platter.
        let pig = Vec3::new(c.x, c.y, c.z);
        drum(f, pig + Vec3::new(0.0, 0.015, 0.0), 0.5, 0.03, up, lin(PEWTER) * 1.1, Vec3::ZERO, SHADE_IRON);
        let roast = lin(ROAST);
        if k >= 0.5 {
            ball(f, pig + Vec3::new(0.0, 0.16, 0.0), Vec3::new(0.42, 0.17, 0.2), up, roast, Vec3::ZERO, SHADE_LEATHER);
            ball(f, pig + Vec3::new(0.0, 0.25, 0.0), Vec3::new(0.32, 0.1, 0.14), up, lin(ROAST_L), Vec3::ZERO, SHADE_LEATHER);
            ball(f, pig + Vec3::new(0.42, 0.17, 0.0), Vec3::new(0.14, 0.12, 0.12), up, roast, Vec3::ZERO, SHADE_LEATHER);
            ball(f, pig + Vec3::new(0.55, 0.15, 0.0), Vec3::new(0.05, 0.05, 0.05), up, lin(APPLE), Vec3::ZERO, SHADE_LIT);
            for s in [-1.0f32, 1.0] {
                ball(f, pig + Vec3::new(0.48, 0.27, s * 0.06), Vec3::new(0.035, 0.05, 0.02), up, roast * 0.8, Vec3::ZERO, SHADE_LEATHER);
                for e in [-0.22f32, 0.22] {
                    ball(f, pig + Vec3::new(e, 0.05, s * 0.15), Vec3::new(0.06, 0.05, 0.05), up, roast * 0.85, Vec3::ZERO, SHADE_LEATHER);
                }
            }
        } else if k >= 0.2 {
            // Half eaten: the haunch left, the ribs bare.
            ball(f, pig + Vec3::new(-0.22, 0.12, 0.0), Vec3::new(0.2, 0.13, 0.17), up, roast, Vec3::ZERO, SHADE_LEATHER);
            ball(f, pig + Vec3::new(0.42, 0.12, 0.0), Vec3::new(0.13, 0.11, 0.11), up, roast * 0.8, Vec3::ZERO, SHADE_LEATHER);
            let out = &mut f.opaque[MeshKind::Cube as usize];
            rod(out, pig + Vec3::new(-0.05, 0.1, 0.0), pig + Vec3::new(0.32, 0.1, 0.0), 0.04, lin(BONE), Vec3::ZERO, SHADE_BONE);
            for i in 0..5 {
                let x = 0.0 + i as f32 * 0.07;
                for s in [-1.0f32, 1.0] {
                    rod(
                        out,
                        pig + Vec3::new(x, 0.1, 0.0),
                        pig + Vec3::new(x + 0.02, 0.2, s * 0.13),
                        0.022,
                        lin(BONE),
                        Vec3::ZERO,
                        SHADE_BONE,
                    );
                }
            }
        } else {
            // Picked clean: the skull and a spine on the platter.
            let out = &mut f.opaque[MeshKind::Cube as usize];
            bx(out, pig + Vec3::new(0.38, 0.07, 0.04), Vec3::new(0.22, 0.12, 0.13), lin(BONE_D), SHADE_BONE);
            bx(out, pig + Vec3::new(0.5, 0.05, 0.04), Vec3::new(0.1, 0.07, 0.09), lin(BONE_D), SHADE_BONE);
            for i in 0..7 {
                let x = -0.3 + i as f32 * 0.08;
                bx(out, pig + Vec3::new(x, 0.04, (rnd(11.0, i) - 0.5) * 0.05), Vec3::new(0.05, 0.04, 0.05), lin(BONE), SHADE_BONE);
            }
        }
        // The fowl.
        let fowl = Vec3::new(x0 + 1.8, c.y, c.z);
        drum(f, fowl + Vec3::new(0.0, 0.012, 0.0), 0.22, 0.025, up, pewter, Vec3::ZERO, SHADE_IRON);
        if k >= 0.6 {
            ball(f, fowl + Vec3::new(0.0, 0.1, 0.0), Vec3::new(0.15, 0.09, 0.11), up, lin(ROAST_L), Vec3::ZERO, SHADE_LEATHER);
            for s in [-1.0f32, 1.0] {
                ball(
                    f,
                    fowl + Vec3::new(0.12, 0.08, s * 0.08),
                    Vec3::new(0.07, 0.04, 0.04),
                    Quat::from_rotation_y(s * 0.5),
                    lin(ROAST_L) * 0.9,
                    Vec3::ZERO,
                    SHADE_LEATHER,
                );
            }
        } else {
            let out = &mut f.opaque[MeshKind::Cube as usize];
            for i in 0..(if k >= 0.3 { 2 } else { 4 }) {
                let a = rnd(13.0, i) * PI;
                let p = fowl + Vec3::new((rnd(15.0, i) - 0.5) * 0.25, 0.03, (rnd(17.0, i) - 0.5) * 0.2);
                rod(
                    out,
                    p - Vec3::new(a.cos(), 0.0, a.sin()) * 0.08,
                    p + Vec3::new(a.cos(), 0.0, a.sin()) * 0.08,
                    0.025,
                    lin(BONE),
                    Vec3::ZERO,
                    SHADE_BONE,
                );
            }
        }
        // The cake, in tiers with cherries; then half; then a slice; then crumbs.
        let cake = Vec3::new(x1 - 1.8, c.y, c.z);
        drum(f, cake + Vec3::new(0.0, 0.012, 0.0), 0.26, 0.025, up, lin(BRASS), Vec3::ZERO, SHADE_BRASS);
        if k >= 0.7 {
            for (i, (r, h)) in [(0.22f32, 0.12f32), (0.16, 0.1), (0.1, 0.08)].into_iter().enumerate() {
                let yb = 0.025 + [0.0, 0.12, 0.22][i];
                drum(
                    f,
                    cake + Vec3::new(0.0, yb + h / 2.0, 0.0),
                    r,
                    h,
                    up,
                    lin(if i % 2 == 0 { CAKE } else { CREAM }),
                    Vec3::ZERO,
                    SHADE_LIT,
                );
            }
            for i in 0..6 {
                let a = i as f32 * TAU / 6.0;
                ball(f, cake + Vec3::new(a.cos() * 0.17, 0.27, a.sin() * 0.17), Vec3::splat(0.025), up, lin(CHERRY), Vec3::ZERO, SHADE_WET);
            }
            ball(f, cake + Vec3::new(0.0, 0.35, 0.0), Vec3::splat(0.035), up, lin(CHERRY), Vec3::ZERO, SHADE_WET);
        } else if k >= 0.35 {
            let out = &mut f.opaque[MeshKind::Cube as usize];
            bx(out, cake + Vec3::new(-0.06, 0.09, 0.0), Vec3::new(0.2, 0.13, 0.36), lin(CAKE), SHADE_LIT);
            bx(out, cake + Vec3::new(-0.06, 0.16, 0.0), Vec3::new(0.2, 0.02, 0.36), lin(CREAM), SHADE_LIT);
            ball(f, cake + Vec3::new(-0.06, 0.19, 0.08), Vec3::splat(0.025), up, lin(CHERRY), Vec3::ZERO, SHADE_WET);
        } else if k >= 0.15 {
            let out = &mut f.opaque[MeshKind::Cube as usize];
            bxr(
                out,
                cake + Vec3::new(0.05, 0.06, 0.02),
                Vec3::new(0.14, 0.08, 0.07),
                Quat::from_rotation_y(0.6),
                lin(CAKE),
                Vec3::ZERO,
                SHADE_LIT,
            );
        } else {
            let out = &mut f.opaque[MeshKind::Cube as usize];
            for i in 0..5 {
                bx(
                    out,
                    cake + Vec3::new((rnd(19.0, i) - 0.5) * 0.3, 0.035, (rnd(21.0, i) - 0.5) * 0.3),
                    Vec3::splat(0.025),
                    lin(CAKE) * 0.85,
                    SHADE_LIT,
                );
            }
        }
        // Loaves at either end: three, then fewer, then crusts.
        let loaves = (k * 3.0).ceil() as u32;
        for (j, x) in [x0 + 0.5, x1 - 0.5].into_iter().enumerate() {
            for i in 0..loaves.min(3) {
                let p = Vec3::new(x + (i as f32 - 1.0) * 0.12, c.y + 0.06, c.z + (i as f32 - 1.0) * 0.18 * if j == 0 { 1.0 } else { -1.0 });
                ball(f, p, Vec3::new(0.11, 0.06, 0.07), Quat::from_rotation_y(0.4 + i as f32), lin(BREAD), Vec3::ZERO, SHADE_LIT);
            }
            if loaves == 0 {
                let out = &mut f.opaque[MeshKind::Cube as usize];
                bx(out, Vec3::new(x, c.y + 0.02, c.z), Vec3::new(0.08, 0.03, 0.05), lin(BREAD) * 0.7, SHADE_LIT);
            }
        }
        // Cheese: a wheel with a wedge cut, then the rind.
        let cheese = Vec3::new(x1 - 0.9, c.y, far + 0.05);
        if k >= 0.5 {
            drum(f, cheese + Vec3::new(0.0, 0.06, 0.0), 0.13, 0.11, up, lin(CHEESE), Vec3::ZERO, SHADE_LIT);
            let out = &mut f.opaque[MeshKind::Cube as usize];
            bxr(
                out,
                cheese + Vec3::new(0.2, 0.04, 0.05),
                Vec3::new(0.1, 0.07, 0.06),
                Quat::from_rotation_y(0.5),
                lin(CHEESE),
                Vec3::ZERO,
                SHADE_LIT,
            );
        } else {
            let out = &mut f.opaque[MeshKind::Cube as usize];
            bxr(
                out,
                cheese + Vec3::new(0.0, 0.02, 0.0),
                Vec3::new(0.16, 0.03, 0.04),
                Quat::from_rotation_y(0.9),
                lin(RIND),
                Vec3::ZERO,
                SHADE_LIT,
            );
        }
        // Bowls of fruit: apples, pears, a heap of grapes.
        for (j, x) in [c.x - 1.15, c.x + 1.15].into_iter().enumerate() {
            let bowl = Vec3::new(x, c.y, far + 0.02);
            drum(f, bowl + Vec3::new(0.0, 0.04, 0.0), 0.15, 0.08, up, lin(BRASS) * 0.9, Vec3::ZERO, SHADE_BRASS);
            let fruit = (k * 5.0).round() as u32;
            for i in 0..fruit {
                let a = i as f32 * 2.4 + j as f32;
                let r = if i == 0 { 0.0 } else { 0.08 };
                let p = bowl + Vec3::new(a.cos() * r, 0.11 + if i == 0 { 0.05 } else { 0.0 }, a.sin() * r);
                let col = match (i + j as u32) % 3 {
                    0 => APPLE,
                    1 => PEAR,
                    _ => GRAPE,
                };
                ball(f, p, Vec3::splat(0.05), up, lin(col), Vec3::ZERO, SHADE_WET);
            }
            if fruit == 0 {
                let out = &mut f.opaque[MeshKind::Cube as usize];
                bx(out, bowl + Vec3::new(0.03, 0.09, 0.0), Vec3::new(0.015, 0.05, 0.015), lin(0x4a3a20), SHADE_WOOD);
            }
        }
        // The wine jug: upright and full, then on its side.
        let jug = Vec3::new(x0 + 1.2, c.y, far + 0.05);
        if k >= 0.35 {
            drum(f, jug + Vec3::new(0.0, 0.11, 0.0), 0.09, 0.22, up, lin(0x6a4a3a), Vec3::ZERO, SHADE_LIT);
            drum(f, jug + Vec3::new(0.0, 0.24, 0.0), 0.05, 0.05, up, lin(0x6a4a3a), Vec3::ZERO, SHADE_LIT);
        } else {
            let lie = Quat::from_rotation_z(PI / 2.0) * Quat::from_rotation_x(0.3);
            drum(f, jug + Vec3::new(0.0, 0.09, 0.0), 0.09, 0.22, lie, lin(0x6a4a3a), Vec3::ZERO, SHADE_LIT);
        }
        // Goblets along the near edge, full while the wine lasts; the last one overturned.
        let goblets = [x0 + 1.2, c.x - 0.75, c.x + 0.75, x1 - 1.2];
        for (i, x) in goblets.into_iter().enumerate() {
            let g = Vec3::new(x, c.y, near - 0.02);
            let full = k > 0.2 + i as f32 * 0.15;
            if level >= plan::LEVELS && i == 2 {
                // The overturned cup, its wine run across the cloth and off the edge.
                let lie = Quat::from_rotation_z(PI / 2.0 - 0.15) * Quat::from_rotation_y(0.4);
                drum(f, g + Vec3::new(0.0, 0.055, 0.0), 0.06, 0.12, lie, lin(BRASS), Vec3::ZERO, SHADE_BRASS);
                drum(f, g + Vec3::new(-0.11, 0.03, -0.03), 0.04, 0.03, lie, lin(BRASS) * 0.9, Vec3::ZERO, SHADE_BRASS);
                let out = &mut f.opaque[MeshKind::Cube as usize];
                bxr(
                    out,
                    g + Vec3::new(0.18, 0.005, 0.12),
                    Vec3::new(0.34, 0.006, 0.12),
                    Quat::from_rotation_y(-0.5),
                    lin(WINE),
                    Vec3::ZERO,
                    SHADE_WET,
                );
                bx(out, g + Vec3::new(0.3, -0.12, 0.31), Vec3::new(0.05, 0.26, 0.012), lin(WINE), SHADE_WET);
                // Its drop falling, now and then.
                let t = (time * 0.7).fract();
                bx(out, g + Vec3::new(0.3, -0.27 - t * (TABLE_Y - 0.3), 0.32), Vec3::splat(0.022), lin(WINE_L), SHADE_WET);
                bxr(
                    out,
                    Vec3::new(g.x + 0.3, floor + 0.004, g.z + 0.45),
                    Vec3::new(0.3, 0.005, 0.22),
                    Quat::from_rotation_y(0.3),
                    lin(WINE),
                    Vec3::ZERO,
                    SHADE_WET,
                );
                continue;
            }
            drum(f, g + Vec3::new(0.0, 0.005, 0.0), 0.05, 0.012, up, lin(BRASS), Vec3::ZERO, SHADE_BRASS);
            let out = &mut f.opaque[MeshKind::Cube as usize];
            bx(out, g + Vec3::new(0.0, 0.06, 0.0), Vec3::new(0.018, 0.1, 0.018), lin(BRASS), SHADE_BRASS);
            drum(f, g + Vec3::new(0.0, 0.15, 0.0), 0.06, 0.1, up, lin(BRASS), Vec3::ZERO, SHADE_BRASS);
            if full {
                drum(f, g + Vec3::new(0.0, 0.195, 0.0), 0.05, 0.01, up, lin(WINE_L), Vec3::ZERO, SHADE_WET);
            }
        }
        // The candelabra: three candles each, burning down level by level, out by the last two.
        let lit = k > 0.2;
        for x in [c.x - 2.3, c.x + 2.3] {
            let base = Vec3::new(x, c.y, c.z);
            drum(f, base + Vec3::new(0.0, 0.02, 0.0), 0.1, 0.04, up, lin(BRASS), Vec3::ZERO, SHADE_BRASS);
            let out = &mut f.opaque[MeshKind::Cube as usize];
            bx(out, base + Vec3::new(0.0, 0.2, 0.0), Vec3::new(0.03, 0.36, 0.03), lin(BRASS), SHADE_BRASS);
            bx(out, base + Vec3::new(0.0, 0.38, 0.0), Vec3::new(0.34, 0.025, 0.03), lin(BRASS), SHADE_BRASS);
            let tall = 0.05 + 0.3 * k;
            for s in [-0.15f32, 0.0, 0.15] {
                let cup = base + Vec3::new(s, 0.4, 0.0);
                let out = &mut f.opaque[MeshKind::Cube as usize];
                bx(out, cup + Vec3::new(0.0, tall / 2.0, 0.0), Vec3::new(0.035, tall, 0.035), lin(WAX), SHADE_LIT);
                bx(out, cup + Vec3::new(0.012, tall * 0.4, 0.012), Vec3::new(0.02, tall * 0.5, 0.02), lin(WAX) * 0.9, SHADE_LIT);
                let tip = cup + Vec3::new(0.0, tall + 0.04, 0.0);
                if lit {
                    let flick = 0.85 + (time * 11.0 + x + s * 9.0).sin() * 0.1 + (time * 27.0 + s).sin() * 0.05;
                    ball(f, tip, Vec3::new(0.018, 0.04, 0.018), up, lin(FLAME), lin(FLAME) * 3.0 * flick, SHADE_UNLIT);
                    glow_orb(f, tip, 0.18, lin(FLAME), 0.25 * flick);
                } else {
                    // A thread of smoke from the wick.
                    let t = (time * 0.5 + s * 3.0).fract();
                    let m = Mat4::from_translation(tip + Vec3::new((t * 6.0).sin() * 0.03, t * 0.4, 0.0))
                        * Mat4::from_scale(Vec3::splat(0.02 + t * 0.03));
                    f.transparent[MeshKind::Sphere as usize].push(Instance::new(m, lin(0x8a8680), 0.25 * (1.0 - t), Vec3::ZERO, SHADE_LIT));
                }
            }
            if lit {
                let flick = 0.9 + (time * 9.0 + x).sin() * 0.06;
                f.lights.push(Light { pos: base + Vec3::new(0.0, 0.5 + 0.3 * k, 0.0), color: lin(FLAME) * 2.6 * flick, distance: 4.0 });
            }
        }
        // Bones the eaters dropped round the table, more at every level.
        let bones = ((1.0 - k) * 12.0).round() as u32;
        let out = &mut f.opaque[MeshKind::Cube as usize];
        for i in 0..bones {
            let side = if rnd(31.0, i) > 0.5 { 1.0 } else { -1.0 };
            let p = Vec3::new(x0 + rnd(33.0, i) * (x1 - x0), floor + 0.025, c.z + side * (0.8 + rnd(35.0, i) * 1.6));
            let a = rnd(37.0, i) * PI;
            let d = Vec3::new(a.cos(), 0.0, a.sin()) * (0.08 + rnd(39.0, i) * 0.08);
            rod(out, p - d, p + d, 0.03, lin(BONE) * (0.85 + rnd(41.0, i) * 0.15), Vec3::ZERO, SHADE_BONE);
            bx(out, p + d, Vec3::splat(0.05), lin(BONE), SHADE_BONE);
        }
        // Flies over what is left, once it has begun to go.
        if k < 0.6 {
            let n = ((0.6 - k) * 20.0) as u32;
            for i in 0..n {
                let a = time * (2.0 + rnd(43.0, i) * 2.0) + i as f32;
                let r = 0.15 + rnd(45.0, i) * 0.3;
                let around = if i % 2 == 0 { c } else { Vec3::new(c.x + 1.15 * if i % 4 == 1 { 1.0 } else { -1.0 }, c.y, far) };
                let p = around + Vec3::new(a.cos() * r, 0.25 + (a * 1.7).sin() * 0.08, (a * 1.3).sin() * r * 0.6);
                bx(out, p, Vec3::splat(0.018), lin(FLY), SHADE_LIT);
            }
        }
    }

    /// Dust drifting down through the light that falls through the holes.
    fn emit_pit_motes(&self, f: &mut Frame, time: f32, hero: Vec3) {
        let r = plan::platform();
        let (lo, w) = (r.x0 as f32 - 0.5, (r.x1 - r.x0 + 1) as f32);
        for i in 0..70u32 {
            let fall = (time * (0.05 + rnd(51.0, i) * 0.06) + rnd(53.0, i)).fract();
            let y = hero.y - 1.0 + (1.0 - fall) * 12.0;
            let p = Vec3::new(
                lo + rnd(55.0, i) * w + (time * 0.3 + i as f32).sin() * 0.2,
                y,
                lo + rnd(57.0, i) * w + (time * 0.23 + i as f32 * 1.3).cos() * 0.2,
            );
            let a = 0.35 * (1.0 - (fall * 2.0 - 1.0).abs());
            glow_orb(f, p, 0.03, lin(0xf0eee8), a);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shell_carves_a_door_and_cells_into_every_wall_and_fades_its_wall_tiles() {
        let mut out = Vec::new();
        let ranges = shell(&mut out, 0.0);
        assert_eq!(ranges.len(), (SIZE * 4 - 4) as usize, "every wall tile, corners and all");
        assert!(ranges.iter().all(|r| r.2 > 0));
        // Nothing of the level stands in the hole, where the platform goes.
        let r = plan::platform();
        for c in &out {
            let m = Mat4::from_cols_array(&c.m);
            let p = m.w_axis.truncate();
            let inside = p.x > r.x0 as f32 - 0.4 && p.x < r.x1 as f32 + 0.4 && p.z > r.y0 as f32 - 0.4 && p.z < r.y1 as f32 + 0.4;
            assert!(!inside || p.y < -LEVEL_DEPTH || p.y > 0.5, "a cube in the hole at {p:?}");
        }
        // Each level's number can be carved.
        for level in 1..=plan::LEVELS {
            assert!(DIGITS[level as usize].iter().all(|r| r.len() == 3));
        }
        assert!(feature(plan::DOOR_AT[0], 0) == Feature::Door);
        assert!(feature(0, plan::CELLS[1]) == Feature::Cell);
    }
}
