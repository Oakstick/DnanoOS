//! Scene II on screen: the shaft of the Pit, on a set of its own far from everything else (so it
//! stands apart from the Pit and the lair as they are played). At its bottom a frozen hollow:
//! black stone dusted with snow, the cage fallen and broken open, chains, a sack, birch switches
//! and broken toys. Above it the seven levels: each a square cell of rough grey stone round the
//! hole, its ledge, its barred cells and iron door, its number carved huge on the north wall; and
//! the stone platform, the feast on it picked to the bones, fitting the hole exactly.
//!
//! The freed boy runs to the hero, who kneels, and they hold each other. A white light falls
//! down the shaft; the platform rises with them through the levels, every cell empty now and lit
//! warm, into the white. (The words and the credits are drawn over it: `hud::ending`.)

use super::story::{kid_root, Act};
use super::*;
use crate::logic::heroes::HeroKind;
use crate::logic::story::{ascent, rise, Shot};

/// Where the shaft stands: the middle of its bottom.
pub(super) const PIT: Vec3 = Vec3::new(-240.0, 0.0, -40.0);
/// Each level's height, floor to floor, and how many there are.
const LEVEL_H: f32 = 6.0;
const LEVELS: usize = 7;
/// Half the hole's width, the cells' inner face (the pillars), and their back walls.
const HOLE: f32 = 5.6;
const CELL: f32 = 12.5;
const BACK: f32 = 13.7;
/// The ledge's thickness under each level's floor.
const LEDGE: f32 = 0.6;
/// Half the platform's width, its thickness, its top at rest on the bottom, and at the end.
const PLATE: f32 = 5.5;
const SLAB: f32 = 0.8;
const REST: f32 = 0.4;
const SUMMIT: f32 = 53.0;
/// The top of the shaft.
const RIM: f32 = LEVEL_H * 8.0;

/// Level `n` (1 at the top, 7 at the bottom): the top of its floor.
fn floor_of(n: usize) -> f32 {
    (8 - n) as f32 * LEVEL_H
}

/// Material shade modes for plain blocks (5 + the material's number), as the story's sets use.
const SHADE_METAL: f32 = 6.0;
const SHADE_CLOTH: f32 = 8.0;
const SHADE_WOOD: f32 = 12.0;

const STONE: u32 = 0x6c6a64;
const STONE_D: u32 = 0x4e4c48;
const IRON: u32 = 0x2a2b2e;
const WARM: u32 = 0xffb070;
const SNOW: u32 = 0xe4eaf2;
const BLACK_STONE: u32 = 0x1c1c22;

/// Where the hero stands on the platform (facing north, toward the fallen cage), where the boy
/// runs from and where he comes to hold him, and where he stands beside him after.
const HERO_AT: Vec3 = Vec3::new(0.0, 0.0, 1.0);
const BOY_FROM: Vec3 = Vec3::new(0.45, 0.0, -8.0);
const BOY_HUG: Vec3 = Vec3::new(0.0, 0.0, 0.66);
const BOY_SIDE: Vec3 = Vec3::new(0.46, 0.0, 0.92);
/// The fallen cage, and the long table of the feast (its middle, on the platform).
const CAGE: Vec3 = Vec3::new(0.5, 0.0, -8.6);
const TABLE: Vec3 = Vec3::new(0.0, 0.0, 4.2);

fn h(a: f64, b: f64) -> f32 {
    hash(a, b) as f32
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn block(out: &mut Vec<Instance>, p: Vec3, s: Vec3, rot: Quat, c: Vec3, shade: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(s, rot, p), c, 1.0, Vec3::ZERO, shade));
}

fn glowing(out: &mut Vec<Instance>, p: Vec3, s: Vec3, c: Vec3, glow: Vec3) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(s, Quat::IDENTITY, p), c, 1.0, glow, SHADE_LIT));
}

/// A block in one of the four walls: `along` the wall from its middle, `out` from the shaft's
/// middle, `y` up; side 0 north, 1 east, 2 south, 3 west. Sizes are given along, up, out.
fn wall_block(out: &mut Vec<Instance>, side: usize, along: f32, outward: f32, y: f32, size: Vec3, c: Vec3, shade: f32) {
    let (p, s) = match side {
        0 => (Vec3::new(along, y, -outward), Vec3::new(size.x, size.y, size.z)),
        1 => (Vec3::new(outward, y, along), Vec3::new(size.z, size.y, size.x)),
        2 => (Vec3::new(-along, y, outward), Vec3::new(size.x, size.y, size.z)),
        _ => (Vec3::new(-outward, y, -along), Vec3::new(size.z, size.y, size.x)),
    };
    block(out, PIT + p, s, Quat::IDENTITY, c, shade);
}

/// The seven segments of a carved figure, as (x, y, horizontal) in a figure one wide and two tall.
const SEGMENTS: [(f32, f32, bool); 7] =
    [(0.5, 2.0, true), (1.0, 1.5, false), (1.0, 0.5, false), (0.5, 0.0, true), (0.0, 0.5, false), (0.0, 1.5, false), (0.5, 1.0, true)];

/// Which segments light for each figure 0 to 9 (a b c d e f g).
const DIGITS: [u8; 10] = [0x3f, 0x06, 0x5b, 0x4f, 0x66, 0x6d, 0x7d, 0x07, 0x7f, 0x6f];

// ---------------------------------------------------------------- the set

/// The shaft, all of it that stays put: the hollow at the bottom and the seven levels above.
pub(super) fn pit_set() -> Vec<Instance> {
    let mut out = Vec::new();
    bottom(&mut out);
    for n in 1..=LEVELS {
        level(&mut out, n);
    }
    // The rim at the top: a lip of dressed stone round the opening, into the light.
    for side in 0..4 {
        wall_block(&mut out, side, 0.0, HOLE + 0.35, RIM + 0.25, Vec3::new(2.0 * HOLE + 1.4, 0.5, 0.7), lin(0x8a867c), SHADE_LIT);
    }
    out
}

/// The frozen hollow at the bottom of the world.
fn bottom(out: &mut Vec<Instance>) {
    let flat = Quat::IDENTITY;
    let at = |x: f32, y: f32, z: f32| PIT + Vec3::new(x, y, z);
    // Black stone underfoot, snow drifted over it (deeper against the walls), the platform's
    // well in the middle.
    for gx in -13..=13 {
        for gz in -13..=13 {
            let (x, z) = (gx as f32, gz as f32);
            if x.abs() < PLATE + 0.2 && z.abs() < PLATE + 0.2 {
                continue;
            }
            let k = h(gx as f64 * 1.7, gz as f64 * 2.3);
            block(out, at(x, -0.15, z), Vec3::new(1.0, 0.3, 1.0), flat, lin(BLACK_STONE) * (0.7 + 0.5 * k), SHADE_LIT);
            let edge = x.abs().max(z.abs());
            let drift = ((edge - 6.0) / 7.0).clamp(0.0, 1.0) * 0.5 + h(gx as f64, gz as f64 * 5.0) * 0.5;
            if drift > 0.35 {
                let depth = 0.03 + (drift - 0.35) * 0.25;
                block(
                    out,
                    at(x + (k - 0.5) * 0.3, depth / 2.0, z + (h(gz as f64, gx as f64) - 0.5) * 0.3),
                    Vec3::new(0.7 + k * 0.6, depth, 0.7 + k * 0.5),
                    Quat::from_rotation_y(k * 3.0),
                    lin(SNOW) * (0.9 + 0.1 * k),
                    SHADE_LIT,
                );
            }
        }
    }
    // The hollow's walls: rough black rock, rimed with frost, rising to the first ledge.
    for side in 0..4 {
        for i in 0..14 {
            let along = -13.0 + i as f32 * 2.0;
            for row in 0..3 {
                let k = h(i as f64 * 3.1 + side as f64, row as f64 * 1.9);
                let c = if k > 0.7 { lin(0x5a6472) } else { lin(0x26262c) * (0.7 + 0.6 * k) };
                wall_block(out, side, along, CELL + 0.5 + k * 0.6, 0.9 + row as f32 * 1.8, Vec3::new(2.1, 1.9, 1.2), c, SHADE_LIT);
            }
            // Icicles under the first ledge.
            for j in 0..3 {
                let k = h(i as f64 * 7.0 + j as f64, side as f64 * 11.0);
                if k < 0.35 {
                    continue;
                }
                let len = 0.3 + k * 0.9;
                let (along, outward) = (along + j as f32 * 0.6 - 0.6, CELL - 0.4 - k * 5.5);
                wall_block(out, side, along, outward, LEVEL_H - LEDGE - len / 2.0, Vec3::new(0.08, len, 0.08), lin(0xcfe4f4), SHADE_LIT);
            }
        }
    }
    // The cage, fallen and burst open: its floor plate, bars bent and broken, its door torn off
    // and lying in the snow, the chain it hung by coiled beside it.
    let cage = |x: f32, y: f32, z: f32| at(CAGE.x + x, CAGE.y + y, CAGE.z + z);
    let tilt = Quat::from_rotation_z(0.08) * Quat::from_rotation_y(0.25);
    block(out, cage(0.0, 0.06, 0.0), Vec3::new(1.7, 0.12, 1.7), tilt, lin(IRON), SHADE_METAL);
    block(out, cage(0.0, 2.15, 0.0), Vec3::new(1.7, 0.1, 1.7), tilt, lin(IRON), SHADE_METAL);
    for side in 0..4 {
        for k in 0..6 {
            let along = -0.75 + k as f32 * 0.3;
            let p = match side {
                0 => Vec3::new(along, 1.1, -0.82),
                1 => Vec3::new(0.82, 1.1, along),
                2 => Vec3::new(along, 1.1, 0.82),
                _ => Vec3::new(-0.82, 1.1, along),
            };
            // The front (south) is where the door was: two bars bent wide, the rest gone.
            if side == 2 && (1..5).contains(&k) {
                if k == 1 || k == 4 {
                    let bend = Quat::from_rotation_z(if k == 1 { 0.5 } else { -0.5 });
                    block(out, cage(p.x, p.y, p.z), Vec3::new(0.05, 2.1, 0.05), tilt * bend, lin(IRON), SHADE_METAL);
                }
                continue;
            }
            block(out, cage(p.x, p.y, p.z), Vec3::new(0.05, 2.1, 0.05), tilt, lin(IRON), SHADE_METAL);
        }
    }
    // The door, flat in the snow before it.
    let door = Quat::from_rotation_y(0.4) * Quat::from_rotation_x(-PI / 2.0 + 0.06);
    for k in 0..5 {
        let off = door * Vec3::new(-0.6 + k as f32 * 0.3, 0.0, 0.0);
        block(out, cage(0.2, 0.06, 1.8) + off, Vec3::new(0.05, 1.9, 0.05), door, lin(IRON), SHADE_METAL);
    }
    for y in [-0.85f32, 0.85] {
        block(out, cage(0.2, 0.06, 1.8) + door * Vec3::new(0.0, y, 0.0), Vec3::new(1.3, 0.06, 0.06), door, lin(IRON), SHADE_METAL);
    }
    // The great chain, coiled where it fell.
    for i in 0..60 {
        let a = i as f32 * 0.42;
        let r = 0.35 + i as f32 * 0.012;
        let p = cage(-1.7 + a.cos() * r, 0.05 + (i / 20) as f32 * 0.06, 0.2 + a.sin() * r);
        let q = Quat::from_rotation_y(-a) * if i % 2 == 0 { Quat::IDENTITY } else { Quat::from_rotation_z(PI / 2.0) };
        block(out, p, Vec3::new(0.05, 0.022, 0.1), q, lin(0x3a3c40), SHADE_METAL);
    }
    // Chains hanging out of the dark, bells on them, gone still.
    for (n, &(x, z, len)) in [(-9.0f32, -8.5f32, 4.0f32), (8.5, -9.4, 3.2), (-10.2, 6.5, 3.6), (9.6, 7.8, 4.4)].iter().enumerate() {
        let top = LEVEL_H - LEDGE;
        let links = (len / 0.1) as usize;
        for l in 0..links {
            let y = top - l as f32 * 0.1;
            let q = if l % 2 == 0 { Quat::IDENTITY } else { Quat::from_rotation_y(PI / 2.0) };
            block(out, at(x, y, z), Vec3::new(0.06, 0.11, 0.022), q, lin(0x34363a), SHADE_METAL);
        }
        let k = h(n as f64, 3.0);
        block(out, at(x, top - len - 0.12, z), Vec3::new(0.2, 0.2, 0.2), Quat::from_rotation_y(k), lin(0x8a6a2a), SHADE_METAL);
    }
    // A sack slumped against the west wall, his basket on its side, bundles of birch switches.
    block(out, at(-11.0, 0.5, -2.0), Vec3::new(1.2, 1.0, 1.0), Quat::from_rotation_y(0.3), lin(0x5a4a34), SHADE_CLOTH);
    block(out, at(-11.1, 1.05, -2.0), Vec3::new(0.5, 0.25, 0.4), Quat::from_rotation_z(0.4), lin(0x5a4a34), SHADE_CLOTH);
    for k in 0..8 {
        let a = k as f32 / 8.0 * TAU;
        block(
            out,
            at(10.6 + a.cos() * 0.45, 0.45, 1.8 + a.sin() * 0.45),
            Vec3::new(0.12, 0.9, 0.12),
            Quat::from_rotation_y(-a) * Quat::from_rotation_x(PI / 2.0 - 0.1),
            lin(0x7a5a32),
            SHADE_WOOD,
        );
    }
    for (b, &(x, z)) in [(-7.5f32, -6.5f32), (7.0, -6.8), (-8.4, 3.4)].iter().enumerate() {
        for k in 0..9 {
            let a = (k as f32 - 4.0) * 0.06;
            block(
                out,
                at(x + (k as f32 - 4.0) * 0.03, 0.06, z),
                Vec3::new(0.025, 0.025, 1.4),
                Quat::from_rotation_y(b as f32 * 1.3 + a),
                lin(0x8a7a62),
                SHADE_WOOD,
            );
        }
        block(out, at(x, 0.07, z), Vec3::new(0.2, 0.1, 0.1), Quat::from_rotation_y(b as f32 * 1.3), lin(0xb02a24), SHADE_CLOTH);
    }
    // Broken toys in the snow: a wooden horse on its side, a drum with its skin split, a doll.
    let horse = |x: f32, y: f32, z: f32| at(5.2 + x, y, -8.4 + z);
    let q = Quat::from_rotation_y(0.7) * Quat::from_rotation_z(PI / 2.0 - 0.1);
    for (off, s, c) in [
        (Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.3, 0.2, 0.55), 0xc8a060),
        (Vec3::new(0.0, 0.18, 0.3), Vec3::new(0.16, 0.3, 0.14), 0xc8a060),
        (Vec3::new(0.0, 0.32, 0.4), Vec3::new(0.12, 0.12, 0.24), 0xc8a060),
        (Vec3::new(0.0, -0.22, 0.2), Vec3::new(0.06, 0.25, 0.06), 0x8a6a3a),
        (Vec3::new(0.0, -0.22, -0.2), Vec3::new(0.06, 0.25, 0.06), 0x8a6a3a),
        (Vec3::new(0.0, 0.2, 0.1), Vec3::new(0.05, 0.1, 0.3), 0xb02a24),
    ] {
        block(out, horse(0.0, 0.16, 0.0) + q * off, s, q, lin(c), SHADE_WOOD);
    }
    block(out, at(-5.4, 0.18, 7.8), Vec3::new(0.42, 0.36, 0.42), Quat::from_rotation_z(1.2), lin(0xb03a2a), SHADE_WOOD);
    block(out, at(-5.25, 0.2, 7.8), Vec3::new(0.06, 0.44, 0.44), Quat::from_rotation_z(1.2), lin(0xe8dcc4), SHADE_CLOTH);
    block(out, at(6.8, 0.06, 7.2), Vec3::new(0.18, 0.1, 0.42), Quat::from_rotation_y(2.1), lin(0x6a8ab0), SHADE_CLOTH);
    block(out, at(6.8, 0.08, 7.0), Vec3::new(0.14, 0.12, 0.14), Quat::from_rotation_y(2.1), lin(0xe8c0a0), SHADE_LIT);
}

/// One level of the Pit: its ledge round the hole, its walls of cells, its iron door, its number.
fn level(out: &mut Vec<Instance>, n: usize) {
    let y0 = floor_of(n);
    let seed = n as f64 * 13.0;
    let flat = Quat::IDENTITY;
    // The ledge: four slabs round the hole, and a worn lip at its edge.
    let width = CELL - HOLE;
    let mid = (CELL + HOLE) / 2.0;
    for side in 0..4 {
        let long = if side % 2 == 0 { 2.0 * BACK } else { 2.0 * HOLE };
        wall_block(out, side, 0.0, mid, y0 - LEDGE / 2.0, Vec3::new(long, LEDGE, width), lin(0x5a5852), SHADE_LIT);
        wall_block(out, side, 0.0, HOLE + 0.2, y0 + 0.02, Vec3::new(2.0 * HOLE + 0.4, 0.06, 0.4), lin(0x77746c), SHADE_LIT);
        // Flagstones: a few set darker and lighter, so the floor is not one slab.
        for k in 0..10 {
            let a = h(seed + k as f64, side as f64 * 3.0);
            let b = h(seed + side as f64, k as f64 * 5.0);
            let along = (a - 0.5) * 2.0 * (if side % 2 == 0 { CELL } else { HOLE });
            let o = HOLE + 0.6 + b * (width - 1.2);
            let c = lin(0x5a5852) * (0.75 + 0.5 * h(a as f64, b as f64));
            wall_block(out, side, along, o, y0 + 0.005, Vec3::new(1.0 + a * 0.8, 0.02, 0.8 + b * 0.6), c, SHADE_LIT);
        }
    }
    // The walls: pillars on the cells' face, a lintel over them, the back wall behind; the
    // middle of the north wall solid, for the level's number; an iron door in the south wall.
    let cells = 7;
    let span = 2.0 * CELL / cells as f32;
    for side in 0..4 {
        // The back wall and the wall above the cells.
        wall_block(
            out,
            side,
            0.0,
            BACK + 0.3,
            y0 + LEVEL_H / 2.0 - LEDGE / 2.0,
            Vec3::new(2.0 * BACK + 0.6, LEVEL_H, 0.6),
            lin(STONE_D),
            SHADE_LIT,
        );
        for k in 0..=cells {
            let along = -CELL + k as f32 * span;
            for row in 0..3 {
                let v = h(seed + k as f64 * 2.0 + side as f64, row as f64);
                wall_block(
                    out,
                    side,
                    along,
                    CELL + 0.6,
                    y0 + 0.55 + row as f32 * 1.05,
                    Vec3::new(0.85 + v * 0.06, 1.04, 1.2),
                    lin(STONE) * (0.78 + 0.3 * v),
                    SHADE_LIT,
                );
            }
        }
        // The lintel band, in courses of rough stone.
        for k in 0..12 {
            let along = -CELL + (k as f32 + 0.5) * (2.0 * CELL / 12.0);
            for row in 0..2 {
                let v = h(seed + k as f64 * 1.3 + side as f64 * 7.0, row as f64 * 3.0 + 1.0);
                wall_block(
                    out,
                    side,
                    along + (row as f32 - 0.5) * 0.4,
                    CELL + 0.6,
                    y0 + 3.55 + row as f32 * 0.95,
                    Vec3::new(2.0 * CELL / 12.0 - 0.04, 0.92, 1.2),
                    lin(STONE) * (0.72 + 0.32 * v),
                    SHADE_LIT,
                );
            }
        }
        // The cells between the pillars: bars, a straw pallet, an empty bowl, and a lamp lit
        // warm inside each now that no one is in them.
        for k in 0..cells {
            let along = -CELL + (k as f32 + 0.5) * span;
            if side == 0 && (2..5).contains(&k) {
                // The north wall's middle: solid, the number carved into it.
                wall_block(out, side, along, CELL + 0.55, y0 + 1.55, Vec3::new(span, 3.1, 1.1), lin(STONE) * 0.92, SHADE_LIT);
                continue;
            }
            if side == 2 && k == 3 {
                // The iron door: plates, straps and rivets, shut.
                wall_block(out, side, along, CELL + 0.3, y0 + 1.5, Vec3::new(span - 0.8, 3.0, 0.2), lin(IRON), SHADE_METAL);
                for y in [0.5f32, 1.5, 2.5] {
                    wall_block(out, side, along, CELL + 0.18, y0 + y, Vec3::new(span - 0.8, 0.14, 0.06), lin(0x4a4640), SHADE_METAL);
                }
                continue;
            }
            for b in 0..7 {
                let x = along - span / 2.0 + 0.62 + b as f32 * (span - 1.24) / 6.0;
                wall_block(out, side, x, CELL + 0.05, y0 + 1.55, Vec3::new(0.06, 3.1, 0.06), lin(IRON), SHADE_METAL);
            }
            for y in [0.3f32, 2.9] {
                wall_block(out, side, along, CELL + 0.05, y0 + y, Vec3::new(span - 0.85, 0.08, 0.08), lin(IRON), SHADE_METAL);
            }
            let v = h(seed + k as f64, side as f64 + 40.0);
            wall_block(
                out,
                side,
                along - 0.4,
                BACK - 0.55,
                y0 + 0.1,
                Vec3::new(1.4, 0.2, 0.7),
                lin(0xa08a5a) * (0.8 + 0.3 * v),
                SHADE_CLOTH,
            );
            if v > 0.4 {
                wall_block(out, side, along + 0.7, CELL + 0.8, y0 + 0.06, Vec3::new(0.24, 0.1, 0.24), lin(0x8a7a62), SHADE_WOOD);
            }
            let lamp = match side {
                0 => Vec3::new(along, y0 + 2.6, -(BACK - 0.35)),
                1 => Vec3::new(BACK - 0.35, y0 + 2.6, along),
                2 => Vec3::new(-along, y0 + 2.6, BACK - 0.35),
                _ => Vec3::new(-(BACK - 0.35), y0 + 2.6, -along),
            };
            glowing(out, PIT + lamp, Vec3::new(0.2, 0.26, 0.2), lin(0xffd49a), lin(WARM) * 2.4);
        }
    }
    // The level's number, carved huge into the north wall's middle: raised strokes of paler stone.
    let (fw, fh, stroke) = (2.0f32, 1.9f32, 0.42f32);
    let digit = DIGITS[n % 10];
    for (i, &(sx, sy, horizontal)) in SEGMENTS.iter().enumerate() {
        if digit & (1 << i) == 0 {
            continue;
        }
        let (x, y) = (-fw / 2.0 + sx * fw, y0 + 0.2 + sy * fh * 0.72 + 0.35);
        let size = if horizontal { Vec3::new(fw + stroke, stroke, 0.16) } else { Vec3::new(stroke, fh * 0.72 + stroke * 0.5, 0.16) };
        block(out, PIT + Vec3::new(x, y, -(CELL - 0.06)), size, flat, lin(0xa8a296), SHADE_LIT);
    }
}

// ------------------------------------------------------------ the light

/// The shaft's air: cold and dark at the bottom; white far above once the light comes (`dawn`),
/// the levels fading up into it; warm on the two of them as it reaches them (`light`); and
/// whiter the higher they rise (`up`).
fn pit_air(dawn: f32, light: f32, up: f32) -> Atmosphere {
    let white = dawn.max(up).clamp(0.0, 1.0);
    let bg = mix_hex(0x06080e, 0xfff4e2, white.powf(1.2));
    Atmosphere {
        background: bg,
        fog_near: 9.0 - 3.0 * up,
        fog_far: 46.0 - 18.0 * up,
        hemi_sky: lin(0x5a6a8a).lerp(lin(0xfff2dc), light) * (0.5 + 0.45 * light),
        hemi_ground: lin(0x1e222a).lerp(lin(0x8a7a62), light) * 0.45,
        ambient: lin(0x222838).lerp(lin(0x6a6050), light) * (0.6 + 0.25 * light),
        sun_dir: Vec3::new(0.06, 1.0, 0.1).normalize(),
        sun: lin(0x8aa0d0).lerp(lin(0xfff4e0), light) * (0.3 + 0.5 * dawn + 2.2 * light),
        exposure: 1.25 + 0.1 * up,
        night_sky: 0.0,
        bloom: 1.25 + 0.6 * light,
        moon_view: Vec3::Y,
        cloud_tint: lin(0x161b24),
    }
}

impl WorldView {
    /// Scene II, each frame: the shaft, the platform where it is, the two on it, and the light.
    pub(super) fn emit_ascent(&self, sv: StoryView, s: &SceneState, f: &mut Frame) {
        let time = s.time as f32;
        let shot = sv.shot();
        let top = platform_top(sv);
        let light = light_level(sv);
        let up = ((top - REST) / (SUMMIT - REST)).clamp(0.0, 1.0);
        let dawn = match shot {
            Shot::Embrace => 0.0,
            Shot::Light => smooth(ascent::LIGHT_FAR, ascent::LIGHT_TOUCH + 0.5, sv.t),
            _ => 1.0,
        };
        f.atmosphere = pit_air(dawn, light, up);
        f.bounds = Vec4::new(PIT.x - 14.0, PIT.z - 14.0, PIT.x + 14.0, PIT.z + 14.0);
        if matches!(shot, Shot::Words | Shot::Credits) {
            // Under the words and the credits there is only the white.
            return;
        }
        f.sets.push(self.ascent_set.clone());
        let on = |p: Vec3| PIT + Vec3::new(p.x, top + p.y, p.z);
        self.emit_platform(f, top, up, time);

        // ---- the two of them
        let embrace = sv.since(Shot::Embrace).min(100.0);
        let lit = sv.since(Shot::Light);
        let kneel = smooth(ascent::KNEEL, ascent::KNEEL + 0.7, embrace) * (1.0 - smooth(ascent::STAND, ascent::STAND + 1.1, lit));
        let gaze = smooth(ascent::LOOK_UP, ascent::LOOK_UP + 1.0, lit);
        let (hug, hand) = {
            let close = smooth(ascent::REACH - 0.15, ascent::REACH + 0.5, embrace);
            let part = smooth(ascent::STAND - 0.3, ascent::STAND + 0.5, lit);
            (close * (1.0 - part), part)
        };
        let hero_yaw = PI;
        self.emit_ascent_hero(f, s.hero_kind, on(HERO_AT), hero_yaw, kneel, hug, gaze, hand, top, time);

        // The boy: across the snow and onto the platform, into his arms; then at his side.
        let run = (embrace - ascent::RUN) / (ascent::REACH - ascent::RUN);
        let (feet, yaw, act) = if embrace < ascent::RUN {
            (PIT + BOY_FROM, PI + 0.1, Act::Gaze(0.15))
        } else if run < 1.0 {
            let u = run.clamp(0.0, 1.0);
            let e = 1.0 - (1.0 - u) * (1.0 - u) * 0.6 - (1.0 - u) * 0.4;
            let mut p = BOY_FROM.lerp(BOY_HUG, e);
            // Up the step onto the platform as he crosses its edge.
            p.y = if p.z > -PLATE { top } else { 0.0 } + (u * 21.0).sin().abs() * 0.035;
            // (His run's cycle by the ground he covers, so his feet stay where they land.)
            let path = BOY_FROM.distance(BOY_HUG);
            let pace = 1.0 - smooth(0.85, 1.0, u) * 0.6;
            (PIT + p, 0.0, Act::Dash { phase: embrace * 15.0, pace, dist: e * path, speed: path / (ascent::REACH - ascent::RUN) })
        } else if hand <= 0.0 {
            (on(BOY_HUG), 0.0, Act::Hug(smooth(ascent::REACH - 0.1, ascent::REACH + 0.5, embrace)))
        } else {
            let p = BOY_HUG.lerp(BOY_SIDE, hand);
            let yaw = lerp_angle(0.0, PI, smooth(0.0, 0.7, hand));
            (on(p), yaw, Act::HandUp(hand, gaze))
        };
        let act = if lit >= 0.0 && hand <= 0.0 && hug < 0.05 { Act::Gaze(gaze) } else { act };
        let glow = Vec3::ZERO;
        self.emit_kid(f, 0, kid_root(feet, yaw), act, time, 0.0, glow, None);

        // ---- the light falling down the shaft
        self.emit_shaft_light(f, sv, top, light, time);

        // ---- the cells' lamps near the platform, and the cold or the white on the two of them
        for n in 1..=LEVELS {
            let y = floor_of(n) + 2.6;
            if (y - top).abs() > LEVEL_H * 1.6 {
                continue;
            }
            for side in 0..4 {
                let p = match side {
                    0 => Vec3::new(0.0, y, -(CELL - 1.0)),
                    1 => Vec3::new(CELL - 1.0, y, 0.0),
                    2 => Vec3::new(0.0, y, CELL - 1.0),
                    _ => Vec3::new(-(CELL - 1.0), y, 0.0),
                };
                f.lights.push(Light { pos: PIT + p, color: lin(WARM) * 7.0, distance: 11.0 });
            }
        }
        let cold = 1.0 - light;
        if cold > 0.01 {
            // The bottom's cold: a blue fill from the snow, a faint moonish light from the cage side.
            f.lights.push(Light { pos: on(Vec3::new(2.0, 3.0, -1.2)), color: lin(0xa8c0f0) * 9.0 * cold, distance: 7.5 });
            f.lights.push(Light { pos: on(Vec3::new(2.6, 1.1, 1.2)), color: lin(0x8a9cc8) * 3.0 * cold, distance: 4.5 });
            f.lights.push(Light { pos: on(Vec3::new(-2.0, 1.6, 2.4)), color: lin(0x7a90c0) * 3.0 * cold, distance: 5.0 });
        }
        // A warm key on their faces, rising with the light.
        f.lights.push(Light { pos: on(Vec3::new(0.6, 2.4, -2.6)), color: lin(0xffe8c8) * (1.2 + 4.0 * light), distance: 6.0 });
        if light > 0.01 {
            f.lights.push(Light { pos: on(Vec3::new(0.0, 3.4, 0.9)), color: lin(0xfff6e8) * 9.0 * light, distance: 6.5 });
        }

        // ---- snow sifting down at the bottom; motes of light once it comes
        let bottomish = 1.0 - smooth(4.0, 14.0, top);
        for k in 0..70 {
            let fk = k as f32;
            let fall = (time * (0.06 + (fk * 0.37).fract() * 0.05) + fk * 0.618).fract();
            let p = PIT
                + Vec3::new(
                    ((fk * 0.731).fract() - 0.5) * 16.0 + (time * 0.3 + fk).sin() * 0.3,
                    top + 7.0 - fall * 7.5,
                    ((fk * 0.413).fract() - 0.5) * 16.0 + (time * 0.25 + fk * 1.7).cos() * 0.3,
                );
            if bottomish > 0.01 {
                f.additive.push(Instance::new(
                    Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.022)),
                    lin(0xdce8ff),
                    0.55 * bottomish * (1.0 - light * 0.6),
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
            if light > 0.01 && k % 2 == 0 {
                let rise = (time * (0.05 + (fk * 0.29).fract() * 0.05) + fk * 0.37).fract();
                let q = PIT + Vec3::new(((fk * 0.53).fract() - 0.5) * 7.0, top + rise * 6.0, ((fk * 0.71).fract() - 0.5) * 7.0);
                glow_orb(f, q, 0.035, lin(0xfff2d8), 0.6 * light * (1.0 - rise) * (0.6 + 0.4 * (time * 2.0 + fk).sin()));
            }
        }
    }

    /// The stone platform at `top`, its edge faintly lit, and the feast on it picked to the bones.
    fn emit_platform(&self, f: &mut Frame, top: f32, up: f32, time: f32) {
        let cube = MeshKind::Cube as usize;
        let at = |x: f32, y: f32, z: f32| PIT + Vec3::new(x, top + y, z);
        let flat = Quat::IDENTITY;
        let put = |f: &mut Frame, p: Vec3, s: Vec3, q: Quat, c: u32, shade: f32| {
            f.opaque[cube].push(Instance::new(Mat4::from_scale_rotation_translation(s, q, p), lin(c), 1.0, Vec3::ZERO, shade));
        };
        // The slab, in big dressed stones, a groove round its edge.
        for gx in 0..4 {
            for gz in 0..4 {
                let k = h(gx as f64 * 3.0 + 1.0, gz as f64 * 7.0);
                let size = 2.0 * PLATE / 4.0;
                let (x, z) = (-PLATE + (gx as f32 + 0.5) * size, -PLATE + (gz as f32 + 0.5) * size);
                f.opaque[cube].push(Instance::new(
                    Mat4::from_scale_rotation_translation(Vec3::new(size - 0.04, SLAB, size - 0.04), flat, at(x, -SLAB / 2.0, z)),
                    lin(0x6e6a62) * (0.82 + 0.25 * k),
                    1.0,
                    Vec3::ZERO,
                    SHADE_LIT,
                ));
            }
        }
        put(f, at(0.0, -SLAB / 2.0 - 0.01, 0.0), Vec3::new(2.0 * PLATE - 0.02, SLAB - 0.04, 2.0 * PLATE - 0.02), flat, 0x3a3834, SHADE_LIT);
        // Its edge: a faint warm line, brighter as it rises into the light.
        let glow = lin(0xffd8a0) * (0.15 + 0.8 * up) * (0.85 + 0.15 * (time * 1.3).sin());
        for side in 0..4 {
            let (p, s) = match side {
                0 => (Vec3::new(0.0, 0.0, -PLATE), Vec3::new(2.0 * PLATE, 0.05, 0.06)),
                1 => (Vec3::new(PLATE, 0.0, 0.0), Vec3::new(0.06, 0.05, 2.0 * PLATE)),
                2 => (Vec3::new(0.0, 0.0, PLATE), Vec3::new(2.0 * PLATE, 0.05, 0.06)),
                _ => (Vec3::new(-PLATE, 0.0, 0.0), Vec3::new(0.06, 0.05, 2.0 * PLATE)),
            };
            f.opaque[cube].push(Instance::new(
                Mat4::from_scale_rotation_translation(s, flat, at(p.x, 0.0, p.z)),
                lin(0xc8b494),
                1.0,
                glow,
                SHADE_LIT,
            ));
        }
        // The long table, its benches, and what is left of the feast: bones, a plate licked clean,
        // a cup on its side.
        let tb = |x: f32, y: f32, z: f32| at(TABLE.x + x, y, TABLE.z + z);
        put(f, tb(0.0, 0.92, 0.0), Vec3::new(5.2, 0.14, 1.3), flat, 0x5a4a3a, SHADE_WOOD);
        for (x, z) in [(-2.3f32, -0.5f32), (2.3, -0.5), (-2.3, 0.5), (2.3, 0.5)] {
            put(f, tb(x, 0.43, z), Vec3::new(0.16, 0.86, 0.16), flat, 0x4a3a2a, SHADE_WOOD);
        }
        for z in [-1.15f32, 1.15] {
            put(f, tb(0.0, 0.45, z), Vec3::new(4.6, 0.1, 0.4), flat, 0x4e3e2e, SHADE_WOOD);
            put(f, tb(-2.0, 0.22, z), Vec3::new(0.12, 0.44, 0.3), flat, 0x4a3a2a, SHADE_WOOD);
            put(f, tb(2.0, 0.22, z), Vec3::new(0.12, 0.44, 0.3), flat, 0x4a3a2a, SHADE_WOOD);
        }
        let bone = 0xe2d8c0;
        for (i, &(x, z, a)) in
            [(-1.6f32, -0.2f32, 0.4f32), (-1.3, 0.25, 2.0), (0.4, -0.1, 1.1), (1.5, 0.3, -0.6), (1.9, -0.25, 2.6)].iter().enumerate()
        {
            let q = Quat::from_rotation_y(a);
            put(f, tb(x, 1.02, z), Vec3::new(0.05, 0.05, 0.34), q, bone, SHADE_LIT);
            for e in [-0.17f32, 0.17] {
                put(f, tb(x, 1.02, z) + q * Vec3::new(0.0, 0.0, e), Vec3::new(0.09, 0.07, 0.06), q, bone, SHADE_LIT);
            }
            if i == 2 {
                // A plate, licked clean.
                put(f, tb(x - 0.5, 1.0, z + 0.2), Vec3::new(0.46, 0.03, 0.46), Quat::from_rotation_y(0.3), 0xb8b0a0, SHADE_LIT);
            }
        }
        let cup = Quat::from_rotation_z(PI / 2.0) * Quat::from_rotation_y(0.4);
        put(f, tb(0.9, 1.05, -0.35), Vec3::new(0.16, 0.22, 0.16), cup, 0x8a6a3a, SHADE_METAL);
    }

    /**
     * The hero on the platform: kneeling (`kneel`), holding the boy (`hug`), looking up into the
     * light (`gaze`), his hand holding the boy's (`hand`). His weapon lies on the stone beside him.
     */
    #[allow(clippy::too_many_arguments)]
    fn emit_ascent_hero(
        &self,
        f: &mut Frame,
        kind: HeroKind,
        feet: Vec3,
        yaw: f32,
        kneel: f32,
        hug: f32,
        gaze: f32,
        hand: f32,
        top: f32,
        time: f32,
    ) {
        let rig = match kind {
            HeroKind::Ragnar => &self.models.ragnar,
            HeroKind::Mehmed => &self.models.mehmed,
            HeroKind::Miyamoto => &self.models.miyamoto,
        };
        let mut pose = Pose::rest(rig);
        let idx = |n: &str| rig.index(n);
        let breathe = (time * 1.9).sin();
        let hip = idx("leftLeg").map(|i| pose.pos[i].y).unwrap_or(0.35);
        // Down on both knees: the legs folded back under him.
        for leg in ["leftLeg", "rightLeg"] {
            if let Some(i) = idx(leg) {
                pose.rot[i].x = 1.15 * kneel;
            }
        }
        let drop = hip * 0.62 * kneel;
        // His arms round the boy, or the shield arm's hand down to the boy's beside him.
        for (name, side) in [("shieldArm", 1.0f32), ("swordArm", -1.0), ("shield", 1.0)] {
            if let Some(i) = idx(name) {
                let sob = (time * 5.5 + side).sin().max(0.0) * 0.02 * hug;
                pose.rot[i].x = -1.15 * hug + sob + breathe * 0.02;
                pose.rot[i].z = side * -0.55 * hug;
                pose.rot[i].y = side * 0.25 * hug;
                if side < 0.0 && hand > 0.0 {
                    pose.rot[i].x += -0.25 * hand;
                    pose.rot[i].z += side * 0.22 * hand;
                }
            }
        }
        if let Some(i) = idx("head") {
            pose.rot[i].x = 0.35 * hug - 0.75 * gaze + breathe * 0.015;
            pose.rot[i].y = -0.35 * hug;
        }
        // Whatever he fought with, set down on the stone at his side.
        for (name, side) in [("blade", -1.0f32), ("offhand", 1.0)] {
            if let Some(i) = idx(name) {
                let rest = pose.pos[i];
                pose.pos[i] = Vec3::new(rest.x + side * 0.38, drop + 0.04, rest.z + 0.25);
                pose.rot[i] = Vec3::new(-PI / 2.0, side * 0.35, PI / 2.0);
            }
        }
        let lean = 0.14 * hug - 0.1 * gaze;
        let root = Mat4::from_translation(feet - Vec3::new(0.0, drop, 0.0) + Vec3::new(0.0, breathe * 0.004, 0.0))
            * Mat4::from_rotation_y(yaw)
            * Mat4::from_rotation_x(lean);
        let light = (top / SUMMIT).clamp(0.0, 1.0);
        emit_rig(&mut f.rigs, rig, &pose, root, Vec3::ONE, 1.0, lin(0xfff0d8) * 0.05 * light);
    }

    /// The light coming down the shaft from far above: a soft column of glow reaching down to
    /// the platform (`light` how far it has come), rays in it, and the brightness at the top.
    fn emit_shaft_light(&self, f: &mut Frame, sv: StoryView, top: f32, light: f32, time: f32) {
        let lit = sv.since(Shot::Light);
        if lit < ascent::LIGHT_FAR {
            return;
        }
        // Its lowest point: from far up the shaft, down to them.
        let reach = smooth(ascent::LIGHT_FAR, ascent::LIGHT_TOUCH, lit);
        let low = RIM + 12.0 - (RIM + 12.0 - top) * reach;
        let thin = smooth(ascent::LIGHT_FAR, ascent::LIGHT_TOUCH + 1.0, lit);
        let width = 0.6 + 1.9 * thin;
        let mut y = low;
        let mut k = 0;
        while y < RIM + 14.0 && k < 60 {
            let along = ((y - low) / 6.0).min(1.0);
            let a = (0.05 + 0.07 * light) * (0.4 + 0.6 * along) * (0.85 + 0.15 * (time * 1.7 + k as f32).sin());
            glow_orb(f, PIT + Vec3::new(0.0, y, 0.6), width * (1.0 + 0.15 * (k as f32 * 0.7).sin()), lin(0xfff4e2), a);
            y += 1.1;
            k += 1;
        }
        // Rays: thin bright threads in the column, drifting.
        for r in 0..9 {
            let fr = r as f32;
            let a = fr * 2.39 + time * 0.05;
            let rad = width * 0.55 * (0.3 + (fr * 0.37).fract() * 0.7);
            let x = a.cos() * rad;
            let z = 0.6 + a.sin() * rad;
            let from = low.max(top + 2.8);
            let len = (RIM + 14.0 - from).max(0.1);
            f.additive.push(Instance::new(
                Mat4::from_translation(PIT + Vec3::new(x, from + len / 2.0, z)) * Mat4::from_scale(Vec3::new(0.012, len, 0.012)),
                lin(0xfff6ea) * 1.6,
                0.05 * thin * (0.6 + 0.4 * (time * 0.9 + fr * 1.3).sin()),
                Vec3::ZERO,
                SHADE_UNLIT,
            ));
        }
        // Where it falls on the stone: a pool of light round their feet.
        if reach > 0.98 {
            let pool = Mat4::from_translation(PIT + Vec3::new(0.0, top + 0.015, 0.8))
                * Mat4::from_rotation_x(-PI / 2.0)
                * Mat4::from_scale(Vec3::new(2.2, 2.2, 1.0));
            f.glow.push(Instance::new(pool, lin(0xfff0d0), 0.35 * light, Vec3::ZERO, SHADE_GLOW));
        }
        glow_orb(f, PIT + Vec3::new(0.0, RIM + 6.0, 0.0), 9.0, lin(0xfff8ee), 0.5 * thin);
    }

    /// Where Scene II's camera stands, what it looks at, and what is in focus.
    pub(super) fn ascent_camera(&self, sv: StoryView) -> (Vec3, Vec3, Vec3) {
        let t = sv.t;
        let top = platform_top(sv);
        let v = |x: f32, y: f32, z: f32| PIT + Vec3::new(x, top + y, z);
        let faces = v(0.2, 0.7, 0.85);
        match sv.shot() {
            Shot::Embrace => {
                if t < ascent::REACH + 0.25 {
                    // Over his shoulder, the boy running to him across the snow.
                    let u = smooth(0.0, ascent::REACH, t);
                    (v(3.4, 1.4, 3.5).lerp(v(2.9, 1.15, 2.9), u), v(0.1, 0.55, -2.6).lerp(v(0.1, 0.5, -0.5), u), v(0.2, 0.5, -1.0))
                } else {
                    // Close, side on: the two of them holding each other.
                    let u = smooth(ascent::REACH, 8.0, t);
                    (v(1.75, 0.82, 0.55).lerp(v(1.35, 0.78, 0.62), u), v(0.0, 0.58, 0.85), faces)
                }
            }
            Shot::Light => {
                if t < ascent::LIGHT_TOUCH + 0.3 {
                    // Low, looking up the shaft: the levels stacked into the dark, the light coming.
                    let u = smooth(0.0, ascent::LIGHT_TOUCH, t);
                    (v(1.6, 0.45, 2.9), v(-0.2, 9.0 + 8.0 * u, -3.0), v(0.0, 6.0, -2.0))
                } else {
                    // In front of them, low: their faces turned up into it.
                    let u = smooth(ascent::LIGHT_TOUCH, 7.0, t);
                    (v(0.35, 0.42, -1.55).lerp(v(0.3, 0.5, -1.9), u), v(0.12, 0.95, 0.9), faces)
                }
            }
            Shot::Rise => {
                if t < 7.0 {
                    // High above them, looking down, as the levels fall away beneath.
                    let u = smooth(0.0, 7.0, t);
                    (v(2.6, 5.6, -3.4).lerp(v(2.2, 4.4, -3.0), u), v(0.2, 0.3, 0.9), faces)
                } else {
                    // Low before them, looking up past them into the white.
                    let u = smooth(7.0, 15.0, t);
                    (v(1.3, 0.5, -3.2).lerp(v(0.9, 0.6, -2.6), u), v(-0.1, 1.15, 1.0).lerp(v(0.0, 2.0, 1.1), u), faces)
                }
            }
            _ => (v(0.9, 0.6, -2.6), v(0.0, 4.6, 1.2), faces),
        }
    }

    /// The part of the shaft the shadow map covers: round the platform, wherever it is.
    pub(super) fn ascent_shadow_box(&self, sv: StoryView) -> (Vec3, f32) {
        (PIT + Vec3::new(0.0, platform_top(sv) + 1.0, 0.0), 7.5)
    }
}

/// The platform's top, for this moment of Scene II.
fn platform_top(sv: StoryView) -> f32 {
    match sv.shot() {
        Shot::Embrace | Shot::Light => REST,
        Shot::Rise => REST + rise(sv.t) * (SUMMIT - REST),
        _ => SUMMIT,
    }
}

/// How far the white light has come (0 none, 1 on them), for this moment.
fn light_level(sv: StoryView) -> f32 {
    match sv.shot() {
        Shot::Embrace => 0.0,
        Shot::Light => smooth(ascent::LIGHT_TOUCH - 0.6, ascent::LIGHT_TOUCH + 0.8, sv.t),
        _ => 1.0,
    }
}
