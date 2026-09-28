//! The foes, sculpted as finely as the heroes they fight (voxels a quarter
//! the size of their first models): the draugr, an undead viking in rusted
//! mail with a cold light in his ribs; the raven and its night kin, feathered
//! wing by wing; and the bog slime, glossy and bubbling, with a lily pad for
//! a hat. Each faces +z and is sculpted in the parts its rig swings (see
//! `Sculpt::part`). Far from the camera they are drawn from a coarser copy
//! (`Rig::coarse`), so a dungeon full of them costs little more than before.

use std::f32::consts::{PI, TAU};

use glam::Vec3;

use super::carve::*;
use super::characters::{mat, Sculpt};
use super::voxels::{Collected, PartDef};

/**
 * A socket or pit pressed into the front of a surface: an elliptical bowl,
 * deepest (`depth`) at its middle, its floor painted `wall`. Returns the
 * floor's z at the middle.
 */
#[allow(clippy::too_many_arguments)]
fn socket(s: &mut Sculpt, x0: f32, y0: f32, rx: f32, ry: f32, depth: i32, d: i32, wall: u32) -> i32 {
    let mut middle = 0;
    for y in (y0 - ry).floor() as i32..=(y0 + ry).ceil() as i32 {
        for x in (x0 - rx).floor() as i32..=(x0 + rx).ceil() as i32 {
            let q = ((x as f32 - x0) / rx).powi(2) + ((y as f32 - y0) / ry).powi(2);
            if q > 1.0 {
                continue;
            }
            let Some(z) = front(s, x, y, d) else { continue };
            let deep = ((1.0 - q).sqrt() * depth as f32).round() as i32;
            for k in 0..deep {
                s.clear(x, y, z - k);
            }
            s.recolor(x, y, z - deep, wall);
            if x == x0.round() as i32 && y == y0.round() as i32 {
                middle = z - deep;
            }
        }
    }
    middle
}

/// A thin ring round an upright axis at (cx, cz), radii `rx`, `rz`, at height `y`.
fn hoop(s: &mut Sculpt, cx: f32, cz: f32, rx: f32, rz: f32, y: i32, c: u32) {
    let n = ((rx + rz) * 4.0) as i32;
    for k in 0..n {
        let a = k as f32 / n as f32 * TAU;
        s.set((cx + a.cos() * rx).round() as i32, y, (cz + a.sin() * rz).round() as i32, c);
    }
}

// --------------------------------------------------------------------- draugr

/// Rusted mail: rows of rings, patches of heavier rust and of bright orange.
fn mail(x: i32, y: i32, z: i32) -> u32 {
    use dead::*;
    let patch = nz(x / 3, y / 3, z / 3);
    let ring = (x + z + (y % 2) * 2).rem_euclid(4) == 0;
    match (patch, ring) {
        (p, true) if p > 0.8 => RUST,
        (p, false) if p > 0.8 => RUST_L,
        (p, true) if p < 0.18 => RUST_DD,
        (p, false) if p < 0.18 => RUST_D,
        (_, true) => RUST_D,
        _ => RUST,
    }
}

mod dead {
    use super::mat;
    pub const BONE: u32 = 0xe2d6bc | mat::IVORY;
    pub const BONE_S: u32 = 0xc2b292 | mat::IVORY;
    pub const BONE_D: u32 = 0x8e7e62 | mat::IVORY;
    pub const TOOTH: u32 = 0xeee4c8 | mat::IVORY;
    pub const CAVITY: u32 = 0x17120f;
    pub const GLOW: u32 = 0x6ff0ff;
    pub const GLOW_C: u32 = 0xe8ffff;
    pub const HALO: u32 = 0x2c6c78;
    pub const RUST: u32 = 0x6e4e3a | mat::METAL;
    pub const RUST_L: u32 = 0x9c6a48 | mat::METAL;
    pub const RUST_D: u32 = 0x5c4232 | mat::METAL;
    pub const RUST_DD: u32 = 0x3a2a22 | mat::METAL;
    pub const IRON: u32 = 0x5a5856 | mat::METAL;
    pub const IRON_D: u32 = 0x3c3a3a | mat::METAL;
    pub const BLADE: u32 = 0x8d8a86 | mat::METAL;
    pub const EDGE: u32 = 0xb8b4ac | mat::METAL;
    pub const RAG: u32 = 0x3b3744 | mat::CLOTH;
    pub const RAG_D: u32 = 0x27242e | mat::CLOTH;
    pub const RAG_L: u32 = 0x544e62 | mat::CLOTH;
    pub const LEATHER: u32 = 0x5a4030 | mat::LEATHER;
    pub const LEATHER_D: u32 = 0x3a281c | mat::LEATHER;
    pub const VERDIGRIS: u32 = 0x5f9a82 | mat::METAL;
    pub const BRONZE: u32 = 0x9a7038 | mat::GOLD;
    pub const HORN: u32 = 0xcdbf98 | mat::IVORY;
    pub const HORN_D: u32 = 0x9c8c68 | mat::IVORY;
    pub const HORN_L: u32 = 0xe8dcc0 | mat::IVORY;
}

/// The draugr's grid: 52 wide (a horn and a claw), 84 tall, 28 deep.
pub const DRAUGR_SIZE: (i32, i32, i32) = (52, 84, 28);
const D_LEFT_LEG: u8 = 1;
const D_LEFT_ARM: u8 = 3;
const D_HEAD: u8 = 5;

/// Hips, shoulders and neck; the right hand holds the sword.
pub const DRAUGR_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [17.5, 28.0, 13.0] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [33.5, 28.0, 13.0] },
    PartDef { name: "leftArm", boxes: &[], pivot: [8.5, 55.0, 13.5] },
    PartDef { name: "rightArm", boxes: &[], pivot: [42.5, 55.0, 13.5] },
    PartDef { name: "head", boxes: &[], pivot: [25.5, 60.0, 13.5] },
];

/**
 * The draugr: a viking dead and risen. Bare bones below the knee (one foot
 * still in its rotten shoe), a byrnie of rusted mail torn open over the
 * ribs and the cold light burning in them, a ragged cloak, a notched sword
 * in his right fist. His skull grins under a rusted helmet with one horn
 * whole and one snapped off, blue fire in its sockets, the last wisps of
 * his beard hanging from the jaw.
 */
pub fn draugr() -> Collected {
    use dead::*;
    let (w, h, d) = DRAUGR_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (25.5f32, 13.0f32);

    // ---- legs: thigh, knee and shin bones, a shoe on the left foot and bare toes on the right
    for (i, lx) in [17.5f32, 33.5].into_iter().enumerate() {
        s.part(D_LEFT_LEG + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        ball(&mut s, v(lx - out * 1.0, 27.4, 13.0), Vec3::splat(2.6), BONE_S);
        limb(&mut s, &[(v(lx - out * 0.6, 27.0, 13.0), 2.2), (v(lx, 22.0, 13.2), 1.6), (v(lx, 18.6, 13.4), 2.1)], BONE);
        ball(&mut s, v(lx, 17.4, 13.2), v(2.8, 2.0, 2.4), BONE_S);
        ball(&mut s, v(lx, 17.8, 15.6), v(1.9, 1.8, 1.2), BONE);
        limb(&mut s, &[(v(lx, 16.4, 13.3), 2.0), (v(lx, 10.0, 13.0), 1.4), (v(lx, 5.0, 12.6), 1.7)], BONE);
        s.capsule((lx + out * 2.0, 15.4, 12.2), (lx + out * 1.8, 5.6, 12.0), 0.8, 0.7, BONE_S);
        ball(&mut s, v(lx, 3.8, 12.5), v(2.3, 1.8, 2.4), BONE_S);
        if i == 0 {
            s.ellipsoid((lx, 1.9, 15.2), (3.6, 2.9, 6.2), 0, LEATHER);
            s.paint(|x, y, z, c| (c == LEATHER && (y == 0 || nz(x, y, z) < 0.16)).then_some(LEATHER_D));
            s.capsule((lx + 1.0, 1.2, 20.6), (lx + 1.4, 0.8, 22.6), 0.8, 0.6, BONE);
            for y in [3, 5] {
                hoop(&mut s, lx, 12.6, 2.5, 2.6, y, LEATHER_D);
            }
        } else {
            ball(&mut s, v(lx, 1.6, 10.8), v(1.9, 1.6, 2.0), BONE_S);
            for (k, f) in [-2.2f32, -0.8, 0.6, 2.0].into_iter().enumerate() {
                let tip = 21.0 - (k as f32 - 1.5).abs() * 0.9;
                s.capsule((lx + f * 0.5, 2.3, 12.6), (lx + f, 1.0, tip - 2.6), 0.75, 0.6, BONE);
                s.capsule((lx + f, 1.0, tip - 2.6), (lx + f * 1.05, 0.6, tip), 0.6, 0.45, BONE_S);
                s.set((lx + f * 1.05).round() as i32, 0, tip.round() as i32 + 1, BONE_D);
            }
        }
        // Scraps of his leg wraps, hanging loose round the shin.
        for y in [7, 8, 11, 12] {
            for z in 8..=18 {
                for x in (lx - 4.0) as i32..=(lx + 4.0) as i32 {
                    let r = ((x as f32 - lx).powi(2) + (z as f32 - 12.9).powi(2)).sqrt();
                    if (2.3..3.3).contains(&r) && nz(x, y / 2, z) > 0.32 {
                        s.set(x, y, z, if nz(x, y, z) < 0.4 { RAG_D } else { RAG });
                    }
                }
            }
        }
    }

    // ---- inside the byrnie: the spine, the ribs round a hollow, the cold light in it
    s.part(0);
    ball(&mut s, v(cx, 29.6, 12.0), v(7.0, 3.0, 4.0), BONE_S);
    for y in 32..=57 {
        ball(&mut s, v(cx, y as f32, 8.4), v(1.8, 0.6, 1.7), if y % 2 == 0 { BONE } else { BONE_D });
        if y % 3 == 0 {
            s.set(cx.round() as i32 - 1, y, 6, BONE_S);
            s.set(cx.round() as i32, y, 6, BONE_S);
        }
    }
    // A dark wall behind the ribs, and the soul-fire before it.
    for y in 40..=54 {
        for x in (cx - 6.0) as i32..=(cx + 6.0) as i32 {
            s.set(x, y, 9, CAVITY);
        }
    }
    ball(&mut s, v(cx - 0.5, 46.5, 12.6), v(2.4, 3.2, 2.2), GLOW);
    ball(&mut s, v(cx - 0.5, 46.5, 13.6), v(1.2, 1.8, 1.4), GLOW_C);
    for k in 0..6 {
        let back = 54.0 - k as f32 * 2.9;
        let (rx, rz) = (7.0 + (k as f32 * 0.6).sin() * 1.4, 5.0);
        let n = 64;
        for j in 0..n {
            let a = j as f32 / n as f32 * TAU;
            let (sa, ca) = (a.sin(), a.cos());
            // The two lowest ribs float, short of the breastbone.
            if k >= 4 && sa > 0.45 {
                continue;
            }
            let y = (back - 2.6 * (1.0 + sa) / 2.0).round() as i32;
            for (r, c) in [(1.0, BONE), (0.82, BONE_S)] {
                s.set((cx + ca * rx * r).round() as i32, y, (cz + sa * rz * r).round() as i32, c);
            }
        }
    }
    s.capsule((cx, 52.0, 18.2), (cx, 41.5, 18.0), 1.2, 1.0, BONE);
    for side in [-1.0f32, 1.0] {
        s.capsule((cx + side * 1.2, 54.2, 17.4), (cx + side * 12.5, 55.6, 13.0), 1.0, 1.0, BONE_S);
    }

    // ---- the byrnie: rusted mail torn open over his left ribs, holed, ragged at the hem
    for y in 28..=57 {
        let (rx, rz) = if y <= 36 {
            (11.4 - (y - 28) as f32 * 0.22, 7.0 - (y - 28) as f32 * 0.05)
        } else if y <= 53 {
            (9.6 + ((y - 36) as f32 / 17.0 * PI * 0.5).sin() * 1.2, 6.8 + ((y - 36) as f32 / 17.0 * PI).sin() * 0.5)
        } else {
            (10.8 + (y - 53) as f32 * 0.9, 7.0 - (y - 53) as f32 * 0.4)
        };
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = ((x as f32 - cx) / rx, (z as f32 - cz) / rz);
                let q = dx.abs().powf(2.4) + dz.abs().powf(2.4);
                let (ix, iz) = ((x as f32 - cx) / (rx - 1.15), (z as f32 - cz) / (rz - 1.15));
                let inner = ix.abs().powf(2.4) + iz.abs().powf(2.4);
                // Over the shoulders it closes into a yoke round the neck.
                let neck = ((x as f32 - cx) / 4.6).powi(2) + ((z as f32 - 13.5) / 4.2).powi(2) < 1.0;
                let shell = if y >= 55 { q <= 1.0 && !neck } else { q <= 1.0 && inner > 1.0 };
                if !shell {
                    continue;
                }
                let tear = z as f32 > cz && (37..=54).contains(&y) && (x as f32) < cx + 3.0 + (y as f32 * 1.3).sin() * 1.6 - ((y - 45) as f32).powi(2) * 0.06;
                let hole = nz(x, y, z) < 0.07;
                let hem = y <= 29 && nz(x, 0, z) < 0.4;
                if tear || hole || hem {
                    continue;
                }
                s.set(x, y, z, mail(x, y, z));
            }
        }
    }
    // Rags of the tunic hanging below the mail at the back, clear of the legs.
    for x in (cx - 11.0) as i32..=(cx + 11.0) as i32 {
        if nz(x, 5, 1) < 0.45 {
            continue;
        }
        let len = 3 + (nz(x, 9, 2) * 6.0) as i32;
        let z = if (x as f32 - cx).abs() > 8.0 { 8 } else { 6 };
        for y in 28 - len..=28 {
            s.set(x, y, z, if y == 28 - len { RAG_D } else { RAG });
        }
    }
    // The belt, cracked, and its buckle green with age.
    for y in 31..=33 {
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = ((x as f32 - cx) / 10.6, (z as f32 - cz) / 7.3);
                let q = dx.abs().powf(2.4) + dz.abs().powf(2.4);
                if q <= 1.0 && q > 0.72 {
                    s.set(x, y, z, if nz(x, y, z) < 0.2 { LEATHER_D } else { LEATHER });
                }
            }
        }
    }
    for y in 30..=34 {
        for x in 23..=28 {
            if y == 30 || y == 34 || x == 23 || x == 28 {
                face(&mut s, x, y, d, 1, VERDIGRIS);
            }
        }
    }
    face(&mut s, 25, 32, d, 1, BRONZE);
    // The cloak down his back, holed and ragged, pinned at his left shoulder.
    for y in 18..=56 {
        let spread = (56 - y) as f32 * 0.08;
        for x in (cx - 11.0 - spread) as i32..=(cx + 11.0 + spread) as i32 {
            let fold = (x as f32 * 0.9).sin();
            let z = (3.6 + fold * 0.8).round() as i32;
            let hem = 18 + (nz(x, 0, 7) * 8.0) as i32;
            if y < hem || nz(x / 2, y / 2, 3) < 0.13 {
                continue;
            }
            let c = if y < hem + 2 || fold < -0.5 { RAG_D } else if fold > 0.7 { RAG_L } else { RAG };
            s.set(x, y, z, c);
        }
    }
    for x in 13..=19 {
        for y in 52..=58 {
            let r = ((x as f32 - 16.0).powi(2) + (y as f32 - 55.0).powi(2)).sqrt();
            if (1.6..2.8).contains(&r) {
                face(&mut s, x, y, d, 1, BRONZE);
            } else if r < 1.6 {
                face(&mut s, x, y, d, 1, VERDIGRIS);
            }
        }
    }

    // ---- arms: a ragged mail sleeve, bare bones below, a claw on the left and the sword in the right
    for (i, ax) in [8.5f32, 42.5].into_iter().enumerate() {
        s.part(D_LEFT_ARM + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        ball(&mut s, v(ax, 54.0, 13.5), Vec3::splat(3.0), BONE_S);
        limb(&mut s, &[(v(ax, 53.5, 13.5), 1.9), (v(ax + out * 0.3, 48.0, 13.7), 1.5), (v(ax + out * 0.6, 44.0, 14.0), 1.7)], BONE);
        s.capsule((ax, 55.4, 13.5), (ax + out * 0.3, 49.0, 13.8), 3.1, 2.8, RUST);
        s.paint(|x, y, z, c| (c == RUST && (x as f32 - ax).abs() <= 5.0 && y <= 57).then(|| mail(x, y, z)));
        for y in 47..=51 {
            for z in 8..=19 {
                for x in (ax - 5.0) as i32..=(ax + 5.0) as i32 {
                    if y < 49 + (nz(x, 3, z) * 3.0) as i32 && s.get(x, y, z).is_some_and(|c| c & 0xff00_0000 == mat::METAL) {
                        s.clear(x, y, z);
                    }
                }
            }
        }
        ball(&mut s, v(ax + out * 0.6, 43.2, 14.2), v(2.3, 2.2, 2.3), BONE_S);
        for (k, dx) in [-0.9f32, 0.9].into_iter().enumerate() {
            s.capsule((ax + out * 0.6 + dx, 42.4, 14.2), (ax + dx * 0.9, 33.6, 15.4), 1.1, 0.9, if k == 0 { BONE } else { BONE_S });
        }
        ball(&mut s, v(ax, 32.4, 15.5), v(2.0, 1.4, 2.0), BONE_S);
        if i == 0 {
            // A bronze arm ring he was buried with, and a claw reaching for the living.
            hoop(&mut s, ax, 15.0, 2.2, 2.2, 37, BRONZE);
            hoop(&mut s, ax, 15.0, 2.2, 2.2, 38, BRONZE);
            ball(&mut s, v(ax, 30.8, 16.4), v(2.2, 1.6, 2.0), BONE);
            for (k, f) in [-1.8f32, -0.6, 0.6, 1.8].into_iter().enumerate() {
                let reach = 20.6 - (k as f32 - 1.5).abs() * 0.7;
                s.capsule((ax + f * 0.8, 30.2, 17.4), (ax + f * 1.2, 28.0, reach - 1.6), 0.6, 0.5, BONE);
                s.capsule((ax + f * 1.2, 28.0, reach - 1.6), (ax + f * 1.3, 26.4, reach), 0.5, 0.35, BONE_S);
                s.set((ax + f * 1.3).round() as i32, 26, reach.round() as i32 + 1, BONE_D);
            }
            s.capsule((ax + 2.2, 31.0, 16.0), (ax + 2.8, 28.8, 18.4), 0.6, 0.45, BONE);
        } else {
            // The sword: a leather-bound grip in the bony fist, a rusted guard, a notched blade.
            s.capsule((ax, 35.6, 16.6), (ax, 26.4, 16.8), 1.0, 1.0, LEATHER_D);
            ball(&mut s, v(ax, 36.4, 16.6), Vec3::splat(1.5), IRON);
            ball(&mut s, v(ax, 30.8, 16.4), v(2.2, 2.4, 1.8), BONE_S);
            for k in 0..4 {
                let y = 33.0 - k as f32 * 1.4;
                ball(&mut s, v(ax, y, 18.0), v(1.9, 0.7, 0.9), if k % 2 == 0 { BONE } else { BONE_S });
            }
            s.cuboid(ax as i32 - 4, 24, 16, ax as i32 + 4, 25, 17, RUST_D);
            s.set(ax as i32 - 4, 26, 16, RUST);
            s.set(ax as i32 + 4, 26, 17, RUST);
            for y in 2..=23 {
                let lean = ((24 - y) as f32 * 0.3).round() as i32;
                let half = if y <= 5 { (y - 1).clamp(0, 2) } else { 2 };
                for x in ax as i32 - half..=ax as i32 + half - 1 {
                    let edge = x == ax as i32 - half || x == ax as i32 + half - 1;
                    // Notches bitten out of the edges.
                    if edge && nz(x, y, 5) < 0.18 {
                        continue;
                    }
                    let c = if nz(x, y / 2, 11) < 0.3 {
                        RUST
                    } else if edge {
                        EDGE
                    } else if x == ax as i32 || x == ax as i32 - 1 {
                        IRON_D
                    } else {
                        BLADE
                    };
                    s.set(x, y, 16 + lean, c);
                    s.set(x, y, 17 + lean, if c == EDGE { EDGE } else { BLADE });
                }
            }
        }
    }

    // ---- the head: a skull on its neck bones, grinning, blue fire in the sockets
    s.part(D_HEAD);
    for y in 56..=61 {
        ball(&mut s, v(cx, y as f32, 12.0), v(1.7, 0.6, 1.7), if y % 2 == 0 { BONE } else { BONE_D });
    }
    ball(&mut s, v(cx, 70.6, 14.4), v(7.4, 8.0, 7.6), BONE);
    rounded(&mut s, v(cx, 64.2, 16.8), v(5.2, 3.2, 4.8), 2.6, 0, BONE);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 5.4, 66.6, 18.2), v(1.8, 1.2, 1.9), BONE);
        s.capsule((cx + side * 5.6, 66.0, 13.0), (cx + side * 5.0, 60.8, 14.6), 1.3, 1.3, BONE_S);
        s.capsule((cx + side * 5.0, 60.4, 14.8), (cx + side * 2.2, 59.4, 19.6), 1.4, 1.4, BONE);
    }
    ball(&mut s, v(cx, 59.6, 20.0), v(2.6, 1.4, 1.4), BONE);
    // Sockets, burning, with a haze of the light round them; the nose a black hollow.
    // Hollow cheeks under the cheekbones.
    for side in [-1.0f32, 1.0] {
        for y in 61..=64 {
            let x = (cx + side * 4.6).round() as i32;
            if let Some(z) = front(&s, x, y, d) {
                s.recolor(x, y, z, BONE_D);
            }
        }
    }
    for ex in [cx - 3.3, cx + 3.3] {
        let z = socket(&mut s, ex, 69.0, 2.7, 2.4, 4, d, CAVITY);
        let (x0, y0) = (ex.round() as i32, 69);
        for (dx, dy) in [(-1, 0), (0, 0), (-1, 1), (0, 1)] {
            s.recolor(x0 + dx, y0 + dy, z, GLOW);
            s.set(x0 + dx, y0 + dy, z + 1, GLOW);
        }
        for (dx, dy) in [(-2, 0), (1, 0), (-2, 1), (1, 1), (-1, -1), (0, -1), (-1, 2), (0, 2)] {
            s.recolor(x0 + dx, y0 + dy, z, HALO);
        }
        s.set(x0 - 1, y0 + 1, z + 2, GLOW_C);
        s.set(x0, y0, z + 2, GLOW_C);
    }
    // Bone darkened round the sockets and the nose.
    let mut rim = Vec::new();
    for y in 63..=71 {
        for z in 10..d {
            for x in 16..36 {
                let near = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)]
                    .iter()
                    .any(|&(dx, dy)| [0, -1].iter().any(|&dz| s.get(x + dx, y + dy, z + dz) == Some(CAVITY)));
                if s.get(x, y, z) == Some(BONE) && near {
                    rim.push((x, y, z));
                }
            }
        }
    }
    for (x, y, z) in rim {
        s.recolor(x, y, z, BONE_S);
    }
    socket(&mut s, cx, 65.6, 1.5, 1.9, 4, d, CAVITY);
    for x in [cx - 0.8, cx + 0.8] {
        face(&mut s, x.round() as i32, 67, d, 0, CAVITY);
    }
    // A gap-toothed grin, the jaw hanging a little open.
    for x in (cx - 4.0) as i32..=(cx + 4.0) as i32 {
        let gap = nz(x, 5, 5) < 0.2;
        face(&mut s, x, 62, d, 0, if gap { CAVITY } else if x % 2 == 0 { TOOTH } else { BONE_S });
        face(&mut s, x, 61, d, 0, CAVITY);
        if (x as f32 - cx).abs() < 3.5 {
            face(&mut s, x, 60, d, 0, if nz(x, 6, 6) < 0.25 { CAVITY } else if x % 2 == 1 { TOOTH } else { BONE_S });
        }
    }
    // The helmet: rusted iron over the crown, a riveted brow band, a ridge, a nose guard.
    let (hx, hy, hz) = (cx, 70.8f32, 14.4f32);
    let rim = |z: i32| 71.0 - ((hz - z as f32) / 7.0).max(0.0) * 4.0;
    for y in 62..h {
        for z in 0..d {
            for x in 0..w {
                let q = ((x as f32 - hx) / 8.5).powi(2) + ((y as f32 - hy) / 9.0).powi(2) + ((z as f32 - hz) / 8.7).powi(2);
                if q > 1.0 || (y as f32) < rim(z) {
                    continue;
                }
                let band = (y as f32) < rim(z) + 2.0;
                let rivet = band && (turn(x, z, hx, hz) * 24.0).fract() < 0.18 && (y as f32) > rim(z) + 0.5;
                let n = nz(x, y, z);
                let c = if rivet {
                    RUST_L
                } else if band {
                    IRON_D
                } else if n < 0.15 {
                    RUST_D
                } else if n > 0.86 {
                    RUST_L
                } else {
                    RUST
                };
                s.set(x, y, z, c);
            }
        }
    }
    for z in 6..=23 {
        if let Some(top) = (0..h).rev().find(|&y| s.get(25, y, z).is_some() && s.tag_at(25, y, z) == D_HEAD) {
            if top > 74 {
                s.set(25, top + 1, z, IRON);
                s.set(26, top + 1, z, IRON_D);
            }
        }
    }
    for y in 64..=71 {
        for x in [25, 26] {
            face(&mut s, x, y, d, 1, if y == 64 { IRON_D } else { IRON });
        }
    }
    // One horn whole, curling up; the other snapped off short.
    limb(&mut s, &[(v(cx - 7.4, 74.6, 14.0), 2.4), (v(cx - 11.4, 76.6, 14.6), 2.0), (v(cx - 14.4, 79.6, 15.6), 1.5), (v(cx - 15.4, 82.4, 17.0), 0.8)], HORN);
    limb(&mut s, &[(v(cx + 7.4, 74.6, 14.0), 2.4), (v(cx + 11.0, 76.2, 14.4), 2.0)], HORN);
    s.paint(|x, y, z, c| {
        if c != HORN {
            return None;
        }
        let dx = (x as f32 - cx).abs();
        if x as f32 > cx + 11.2 {
            return Some(HORN_L);
        }
        ((dx * 1.3 + y as f32 * 0.7).sin() > 0.75 || nz(x, y, z) < 0.1).then_some(HORN_D)
    });
    s.finish(0.05)
}

// --------------------------------------------------------------------- ravens

/// A raven's colours: the day raven's are black with a blue gloss, the night raven's purple.
struct Plumage {
    black: u32,
    gloss: u32,
    gloss_l: u32,
    edge: u32,
    under: u32,
    beak: u32,
    beak_l: u32,
    beak_d: u32,
    iris: u32,
    leg: u32,
}

const DAY: Plumage = Plumage {
    black: 0x1c1e26 | mat::FUR,
    gloss: 0x2e3a58 | mat::FUR,
    gloss_l: 0x4a5c88 | mat::FUR,
    edge: 0x12131a | mat::FUR,
    under: 0x0e0f14 | mat::FUR,
    beak: 0x4a4844 | mat::IVORY,
    beak_l: 0x6e6b64 | mat::IVORY,
    beak_d: 0x2a2926 | mat::IVORY,
    iris: 0xffd23a,
    leg: 0x2c2c30 | mat::LEATHER,
};

const NIGHT: Plumage = Plumage {
    black: 0x2a1838 | mat::FUR,
    gloss: 0x4c2870 | mat::FUR,
    gloss_l: 0x7440a8 | mat::FUR,
    edge: 0x1a0f24 | mat::FUR,
    under: 0x140b1c | mat::FUR,
    beak: 0xd8ccb0 | mat::IVORY,
    beak_l: 0xf0e6d0 | mat::IVORY,
    beak_d: 0x9a8e74 | mat::IVORY,
    iris: 0xff3a3a,
    leg: 0x3a2a40 | mat::LEATHER,
};

/// The raven's grid: 76 wide (wings spread), 28 tall, 44 long.
pub const RAVEN_SIZE: (i32, i32, i32) = (76, 28, 44);
const RAVEN_X: f32 = 37.5;

/// Wings hinge where they meet the body.
pub const RAVEN_RIG: &[PartDef] = &[
    PartDef { name: "leftWing", boxes: &[], pivot: [29.5, 14.0, 21.5] },
    PartDef { name: "rightWing", boxes: &[], pivot: [45.5, 14.0, 21.5] },
];

/**
 * A flat feather in the plane of a wing or tail, from `a` to `b` (x, z),
 * half as wide as `wa` at its root and `wb` near its rounded tip, at the
 * height `y(x)`: a shaft down its middle, its vane and its edge.
 */
#[allow(clippy::too_many_arguments)]
fn feather(s: &mut Sculpt, a: (f32, f32), b: (f32, f32), wa: f32, wb: f32, y: impl Fn(f32) -> f32, vane: u32, shaft: u32, edge: u32) {
    let r = wa.max(wb) + 1.0;
    let ab = (b.0 - a.0, b.1 - a.1);
    let len2 = (ab.0 * ab.0 + ab.1 * ab.1).max(1e-6);
    for z in (a.1.min(b.1) - r).floor() as i32..=(a.1.max(b.1) + r).ceil() as i32 {
        for x in (a.0.min(b.0) - r).floor() as i32..=(a.0.max(b.0) + r).ceil() as i32 {
            let p = (x as f32 - a.0, z as f32 - a.1);
            let t = ((p.0 * ab.0 + p.1 * ab.1) / len2).clamp(0.0, 1.0);
            let dist = ((p.0 - ab.0 * t).powi(2) + (p.1 - ab.1 * t).powi(2)).sqrt();
            let mut half = wa + (wb - wa) * t;
            if t > 0.8 {
                half *= ((1.0 - t) / 0.2).sqrt().max(0.3);
            }
            if dist > half {
                continue;
            }
            let c = if dist < 0.5 && t < 0.9 { shaft } else if dist > half - 0.8 { edge } else { vane };
            s.set(x, y(x as f32).round() as i32, z, c);
        }
    }
}

/// One wing, spread: coverts over the arm, secondaries behind them, primaries fanned from the wrist.
fn wing(s: &mut Sculpt, p: &Plumage, side: f32, night: bool) {
    let (root, span) = (7.0f32, 29.6f32);
    let u_of = move |x: f32| ((x - RAVEN_X) * side - root) / span;
    // Raised toward the tips, drooping at the very end.
    let wy = move |x: f32| {
        let u = u_of(x).clamp(0.0, 1.0);
        13.6 + u * 5.0 - (u - 0.82).max(0.0) * 9.0
    };
    let (w, _, d) = RAVEN_SIZE;
    for x in 0..w {
        let u = u_of(x as f32);
        if !(0.0..=0.5).contains(&u) {
            continue;
        }
        let le = 28.4 - u * 3.0;
        let te = 10.6 + u * 6.0;
        let y = wy(x as f32).round() as i32;
        for z in 0..d {
            let zf = z as f32;
            // The trailing edge of the secondaries: one rounded tip per feather.
            let tip = (x as f32 * side * 0.52).sin().abs().sqrt() * 1.6;
            let torn = night && nz(x, 7, z) < 0.12 && zf < te + 5.0;
            if zf > le || zf < te + 1.6 - tip || torn {
                continue;
            }
            let t = (le - zf) / (le - te);
            let c = if t < 0.1 {
                p.edge
            } else if t < 0.42 {
                // Rows of small coverts, glossy, each row's edge in shadow.
                let row = ((le - zf) / 2.2) as i32;
                let fx = ((x as f32 + row as f32 * 1.5) / 3.0).fract();
                if fx < 0.22 || (le - zf) % 2.2 < 0.5 {
                    p.edge
                } else if nz(x, row, 3) > 0.7 {
                    p.gloss_l
                } else {
                    p.gloss
                }
            } else if (x as f32 * side * 0.52 / PI).fract() < 0.12 {
                p.edge
            } else {
                p.black
            };
            s.set(x, y, z, c);
            if t < 0.55 {
                s.set(x, y - 1, z, p.under);
            }
            if t < 0.16 && u < 0.4 {
                s.set(x, y + 1, z, p.black);
            }
        }
    }
    // The primaries, fingered, fanning out from the wrist.
    let wrist = RAVEN_X + side * (root + 0.44 * span);
    for k in 0..7 {
        let a = k as f32 / 6.0;
        let reach = 1.0 - a * 0.14;
        let tip_x = RAVEN_X + side * (root + reach * span);
        let tip_z = 25.6 - a * 17.0;
        let base = (wrist - side * a * 2.0, 24.0 - a * 7.0);
        let short = if night && nz(k, 3, 3) < 0.45 { 0.78 } else { 1.0 };
        let tip = (base.0 + (tip_x - base.0) * short, base.1 + (tip_z - base.1) * short);
        feather(s, base, tip, 1.9, 1.3, wy, p.black, p.edge, if a < 0.2 { p.gloss } else { p.edge });
    }
    if night {
        // A hooked bone claw at the wrist.
        let y = wy(wrist) + 1.0;
        s.capsule((wrist, y, 26.4), (wrist - side * 1.0, y + 0.6, 29.4), 0.9, 0.4, p.beak);
    }
}

/**
 * A raven with its wings spread: glossy feathers row on row, fingered
 * primaries, a wedge of a tail, a heavy hooked beak, shaggy throat hackles
 * and talons tucked under. `night` is its larger kin: purple-black, the
 * wings ragged, a beak and wrist claws of bone, eyes burning red.
 */
pub fn raven(night: bool) -> Collected {
    let p = if night { &NIGHT } else { &DAY };
    let (w, h, d) = RAVEN_SIZE;
    let cx = RAVEN_X;
    let mut s = Sculpt::new(w, h, d);
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        wing(&mut s, p, side, night);
    }

    // ---- the tail: a wedge of long feathers, the middle ones longest
    s.part(0);
    for k in 0..7 {
        let a = (k as f32 - 3.0) / 3.0;
        let tip = (cx + a * 8.0, 0.6 + a.abs() * 3.4);
        feather(&mut s, (cx + a * 2.4, 10.0), tip, 1.6, 1.7, |_| 11.0, p.black, p.edge, if a.abs() > 0.9 { p.gloss } else { p.edge });
    }
    for x in (cx - 3.0) as i32..=(cx + 3.0) as i32 {
        for z in 5..=10 {
            s.set(x, 10, z, p.under);
        }
    }

    // ---- body, neck and head
    ball(&mut s, v(cx, 12.4, 20.0), v(8.2, 7.2, 12.4), p.black);
    s.capsule((cx, 13.8, 26.0), (cx, 16.0, 30.0), 6.2, 5.6, p.black);
    ball(&mut s, v(cx, 16.6, 31.8), v(5.8, 5.6, 6.0), p.black);
    // Feathers laid in overlapping rows, the gloss on the back and crown.
    s.paint(|x, y, z, c| {
        if c != p.black {
            return None;
        }
        let row = (z as f32 / 2.4).floor() as i32;
        let fx = ((x as f32 + if row % 2 == 0 { 0.0 } else { 1.5 }) / 3.0).fract();
        let fz = (z as f32 / 2.4).fract();
        Some(if y <= 7 {
            p.under
        } else if fz < 0.3 || fx < 0.16 {
            p.edge
        } else if y >= 19 && nz(x, y, z) > 0.55 {
            p.gloss_l
        } else if y >= 16 {
            p.gloss
        } else {
            p.black
        })
    });
    // Shaggy hackles under the throat.
    for z in 30..=37 {
        for x in (cx - 3.0) as i32..=(cx + 3.0) as i32 {
            let Some(y0) = (0..h).find(|&y| s.get(x, y, z).is_some()) else { continue };
            if nz(x, 2, z) < 0.3 {
                continue;
            }
            let n = 1 + (nz(x, 9, z) * 2.6) as i32;
            for k in 1..=n {
                s.set(x, y0 - k, z, if k == n { p.edge } else { p.black });
            }
        }
    }

    // ---- the beak: heavy, hooked, its ridge catching the light, bristles over its root
    s.capsule((cx, 16.6, 35.6), (cx, 15.8, 41.2), 2.7, 0.9, p.beak);
    s.capsule((cx, 15.8, 41.2), (cx, 14.4, 42.6), 0.9, 0.5, p.beak);
    s.capsule((cx, 13.9, 35.6), (cx, 14.0, 39.8), 1.9, 0.6, p.beak_d);
    s.paint(|x, y, z, c| {
        if c != p.beak || z < 36 {
            return None;
        }
        if (x as f32 - cx).abs() < 0.6 && y >= 17 {
            Some(p.beak_l)
        } else if y <= 14 {
            Some(p.beak_d)
        } else {
            None
        }
    });
    for z in 35..=38 {
        for x in (cx - 1.5) as i32..=(cx + 1.5) as i32 {
            if let Some(top) = (0..h).rev().find(|&y| s.get(x, y, z).is_some()) {
                s.set(x, top + 1, z, if (x + z) % 2 == 0 { p.black } else { p.edge });
            }
        }
    }

    // ---- eyes on the sides of the head: iris, pupil, a glint; the night raven's burn
    for side in [-1i32, 1] {
        let (ey, ez) = (18, 34);
        for dy in -2i32..=2 {
            for dz in -2i32..=2 {
                let r2 = dy * dy + dz * dz;
                if r2 > 5 {
                    continue;
                }
                let (y, z) = (ey + dy, ez + dz);
                let found = if side < 0 { (0..w).find(|&x| s.get(x, y, z).is_some()) } else { (0..w).rev().find(|&x| s.get(x, y, z).is_some()) };
                let Some(x) = found else { continue };
                if s.tag_at(x, y, z) != 0 {
                    continue;
                }
                let c = if dy == 1 && dz == 1 {
                    GLINT
                } else if r2 <= 1 {
                    PUPIL
                } else if r2 <= 4 {
                    p.iris
                } else {
                    p.edge
                };
                s.recolor(x, y, z, c);
                if night && r2 <= 4 && c != GLINT {
                    s.set(x + side, y, z, if r2 <= 1 { 0xffd0c0 } else { p.iris });
                }
            }
        }
    }

    // ---- talons tucked under the belly
    for side in [-1.0f32, 1.0] {
        let lx = cx + side * 3.0;
        s.capsule((lx, 6.4, 18.0), (lx, 3.6, 14.2), 1.3, 0.9, p.leg);
        for dx in [-1.2f32, 0.0, 1.2] {
            s.capsule((lx, 3.6, 14.2), (lx + dx, 2.2, 11.8), 0.6, 0.5, p.leg);
            s.set((lx + dx).round() as i32, 1, 11, p.beak_d);
        }
    }
    s.finish(0.05)
}

// ----------------------------------------------------------------- bog slime

mod goo {
    use super::mat;
    pub const GREEN: u32 = 0x3fe05a | mat::SLIME;
    pub const DEEP: u32 = 0x1f8a33;
    pub const DEEPER: u32 = 0x166a28;
    /// The lower flanks, matte: gloss there only catches every ledge of the voxels.
    pub const FLANK: u32 = 0x38cc50;
    pub const LIGHT: u32 = 0x86f59a | mat::SLIME;
    pub const GLOSS: u32 = 0xeefff0 | mat::SLIME;
    pub const BUBBLE: u32 = 0xa8ffb8 | mat::SLIME;
    pub const SPECK: u32 = 0x9ad84a | mat::SLIME;
    pub const WHITE: u32 = 0xf4f4f8 | mat::IVORY;
    pub const MOUTH: u32 = 0x2c0e14;
    pub const THROAT: u32 = 0x160609;
    pub const TONGUE: u32 = 0xe0607a | mat::SKIN;
    pub const TOOTH: u32 = 0xf6f2e0 | mat::IVORY;
    pub const PAD: u32 = 0x3a8c2c | mat::LEATHER;
    pub const PAD_L: u32 = 0x58ac3c | mat::LEATHER;
    pub const PAD_D: u32 = 0x2a6a20 | mat::LEATHER;
    pub const PETAL: u32 = 0xffb8d0 | mat::CLOTH;
    pub const PETAL_L: u32 = 0xfff0f4 | mat::CLOTH;
    pub const POLLEN: u32 = 0xffd23a;
    pub const BONE: u32 = 0xd8ccb0 | mat::IVORY;
}

/// The bog slime's grid: 44 across, 32 tall.
pub const SLIME_SIZE: (i32, i32, i32) = (44, 32, 44);

/**
 * A bog slime: a glossy dome of green goo sagging into a puddle at its
 * foot, bubbles rising in it and a drowned bone showing through its side,
 * two big eyes (one bigger than the other), a gummy grin with a single
 * tooth, and a lily pad in flower on its head.
 */
pub fn bog_slime() -> Collected {
    use goo::*;
    let (w, h, d) = SLIME_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (21.5f32, 21.5f32);
    // The dome, sagging wide at its foot.
    for y in 0..h {
        let k = y as f32 / 29.5;
        if k > 1.0 {
            break;
        }
        let r = 20.2 * (1.0 - k * k).max(0.0).sqrt() * (1.0 + (1.0 - k).powi(6) * 0.06);
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = (x as f32 - cx, z as f32 - cz);
                let wobble = ((dz.atan2(dx) * 5.0).sin() * 0.5 + nz(x, 0, z) * 0.6) * (1.0 - k);
                if (dx * dx + dz * dz).sqrt() <= r + wobble {
                    s.set(x, y, z, GREEN);
                }
            }
        }
    }
    // A thin puddle round its foot.
    for z in 0..d {
        for x in 0..w {
            let r = ((x as f32 - cx).powi(2) + (z as f32 - cz).powi(2)).sqrt();
            if r <= 21.0 + nz(x / 2, 1, z / 2) * 1.6 && s.get(x, 0, z).is_none() {
                s.set(x, 0, z, DEEPER);
            }
        }
    }
    // Deep at the foot, lighter toward the crown and where the light falls, bubbles inside.
    s.paint(|x, y, z, _| {
        let n = nz(x, y, z);
        Some(if y == 0 {
            DEEPER
        } else if y <= 3 {
            DEEP
        } else if n < 0.035 {
            BUBBLE
        } else if y >= 17 && (x as f32) < cx - 2.0 && (z as f32) > cz - 6.0 && n > 0.25 {
            LIGHT
        } else if y >= 25 && nz(x / 2, y, z / 2) < 0.12 {
            SPECK
        } else if y < 9 {
            FLANK
        } else {
            GREEN
        })
    });
    // Drips running down its sides.
    for k in 0..7 {
        let a = k as f32 / 7.0 * TAU + 0.4;
        let (dx, dz) = (a.cos(), a.sin());
        if dz > 0.55 {
            continue; // not over its face
        }
        let top = 14 + (nz(k, 1, 1) * 8.0) as i32;
        for y in 2..=top {
            let k2 = y as f32 / 29.5;
            let r = 20.2 * (1.0 - k2 * k2).sqrt() + 0.6;
            s.set((cx + dx * r).round() as i32, y, (cz + dz * r).round() as i32, if y < 4 { GREEN } else { DEEP });
        }
        s.set((cx + dx * 21.6).round() as i32, 1, (cz + dz * 21.6).round() as i32, GREEN);
    }
    // Bubbles on the surface, each with its glint.
    for k in 0..11 {
        let a = nz(k, 3, 7) * TAU;
        let y = 5.0 + nz(k, 5, 2) * 21.0;
        let kk = y / 29.5;
        let r = 20.2 * (1.0 - kk * kk).sqrt();
        let (bx, bz) = (cx + a.cos() * r, cz + a.sin() * r);
        if bz > cz + 12.0 && y > 6.0 {
            continue;
        }
        let br = 1.0 + nz(k, 9, 9) * 1.3;
        ball(&mut s, v(bx, y, bz), Vec3::splat(br), BUBBLE);
        s.set((bx - a.cos() * 0.3).round() as i32, (y + br).round() as i32, bz.round() as i32, GLOSS);
    }
    // The shine where the light strikes the crown.
    for (x, y, z) in [(14, 25, 24), (13, 24, 25), (15, 26, 23), (12, 23, 27), (16, 26, 21), (14, 26, 22), (12, 24, 26)] {
        if let Some(top) = (0..h).rev().find(|&yy| s.get(x, yy, z).is_some()) {
            s.recolor(x, top.min(y + 3), z, GLOSS);
        }
    }
    // A drowned bone, half out of its side.
    s.capsule((33.0, 9.0, 15.0), (41.5, 11.0, 12.0), 1.1, 1.1, BONE);
    ball(&mut s, v(41.8, 11.8, 12.6), Vec3::splat(1.5), BONE);
    ball(&mut s, v(42.0, 10.2, 11.4), Vec3::splat(1.4), BONE);

    // ---- the face: big eyes, a grin with one tooth
    for (ex, r, look) in [(14.0f32, 4.8f32, 0.8f32), (29.0, 4.1, -0.4)] {
        let fz = front(&s, ex.round() as i32, 19, d).unwrap_or(38) as f32;
        ball(&mut s, v(ex, 19.5, fz - 0.4), v(r, r * 1.08, 3.4), WHITE);
        let pz = front(&s, ex.round() as i32, 19, d).unwrap_or(40);
        let (px, py) = (ex + look, 19.0);
        for y in (py - 2.0) as i32..=(py + 2.0) as i32 {
            for x in (px - 2.0) as i32..=(px + 2.0) as i32 {
                let q = (x as f32 - px).powi(2) + (y as f32 - py).powi(2);
                if q <= 4.6 {
                    let z = front(&s, x, y, d).unwrap_or(pz);
                    s.recolor(x, y, z, PUPIL);
                }
            }
        }
        let gx = (px - 1.0).round() as i32;
        let gz = front(&s, gx, 21, d).unwrap_or(pz);
        s.recolor(gx, 21, gz, GLINT);
        s.set(gx, 21, gz + 1, GLINT);
    }
    // The grin: a curved slot pressed deep into the goo.
    for x in 11..=32 {
        let t = (x as f32 - 21.5) / 10.5;
        let mid = 10.0 + t * t * 2.6;
        let half = 2.6 * (1.0 - t * t).max(0.0).sqrt();
        for y in (mid - half).round() as i32..=(mid + half).round() as i32 {
            let Some(z) = front(&s, x, y, d) else { continue };
            let deep = 2 + (half * 0.8) as i32;
            for k in 0..deep {
                s.clear(x, y, z - k);
            }
            s.recolor(x, y, z - deep, if (y as f32) < mid - half + 1.2 { TONGUE } else if deep > 3 { THROAT } else { MOUTH });
        }
    }
    if let Some(z) = front(&s, 24, 13, d) {
        for y in [11, 12] {
            s.set(24, y, z - 1, TOOTH);
            s.set(25, y, z - 1, TOOTH);
        }
    }

    // ---- a lily pad on its head, notched, veined, in flower
    // It drapes over the crown, following the goo beneath.
    let (lx, lz) = (24.0f32, 18.0f32);
    let mut ly = 0;
    for z in 0..d {
        for x in 0..w {
            let (dx, dz) = (x as f32 - lx, z as f32 - lz);
            let r = (dx * dx + dz * dz).sqrt();
            let ang = dz.atan2(dx);
            if r > 7.4 || (ang > 2.2 && ang < 2.7) {
                continue;
            }
            let Some(top) = (0..h - 3).rev().find(|&y| s.get(x, y, z).is_some()) else { continue };
            let vein = r > 1.5 && ((ang * 9.0 / TAU).fract() < 0.12);
            let c = if r > 6.6 { PAD_D } else if vein { PAD_L } else { PAD };
            s.set(x, top + 1, z, c);
            if x == lx as i32 && z == lz as i32 {
                ly = top + 1;
            }
        }
    }
    for k in 0..8 {
        let a = k as f32 / 8.0 * TAU;
        for t in 1..=3 {
            let (x, z) = ((lx + 0.5 + a.cos() * t as f32).round() as i32, (lz + 0.5 + a.sin() * t as f32).round() as i32);
            s.set(x, ly + 1 + i32::from(t == 3), z, if t == 3 { PETAL_L } else { PETAL });
        }
    }
    s.set(lx.round() as i32, ly + 1, lz.round() as i32, POLLEN);
    s.set(lx.round() as i32 + 1, ly + 1, lz.round() as i32, POLLEN);
    s.finish_grounded(0.04)
}

// -------------------------------------------------------------------- cyclops

mod cyc {
    use super::mat;
    pub const SKIN: u32 = 0x9a4f86 | mat::SKIN;
    pub const SKIN_M: u32 = 0x87417a | mat::SKIN;
    pub const SKIN_D: u32 = 0x6e3160 | mat::SKIN;
    pub const SKIN_L: u32 = 0xc987b0 | mat::SKIN;
    pub const WART: u32 = 0x5a2450 | mat::SKIN;
    pub const WART_L: u32 = 0xa45e90 | mat::SKIN;
    pub const SCLERA: u32 = 0xf4eedc | mat::SLIME;
    pub const SCLERA_D: u32 = 0xdcd0b4 | mat::SLIME;
    pub const VEIN: u32 = 0xd23a3a | mat::SLIME;
    pub const VEIN_D: u32 = 0x9a2030 | mat::SLIME;
    pub const IRIS: u32 = 0xffb21e | mat::SLIME;
    pub const IRIS_L: u32 = 0xffe07a | mat::SLIME;
    pub const IRIS_D: u32 = 0xd8641a | mat::SLIME;
    pub const LIMBUS: u32 = 0x8a3410 | mat::SLIME;
    pub const PUPIL: u32 = 0x160810 | mat::SLIME;
    pub const SHINE: u32 = 0xffffff;
    pub const MAW: u32 = 0x2a0a16;
    pub const THROAT: u32 = 0x14050a;
    pub const GUM: u32 = 0x8a2a4a | mat::SKIN;
    pub const TONGUE: u32 = 0xc04a6a | mat::SKIN;
    pub const TOOTH: u32 = 0xf2ead0 | mat::IVORY;
    pub const TOOTH_D: u32 = 0xc8b890 | mat::IVORY;
    pub const HORN: u32 = 0xeadfc2 | mat::IVORY;
    pub const HORN_R: u32 = 0xc4b48e | mat::IVORY;
    pub const HORN_D: u32 = 0x9c8a66 | mat::IVORY;
    pub const MEMBRANE: u32 = 0x4e1f47 | mat::LEATHER;
    pub const MEMBRANE_L: u32 = 0x6a2c60 | mat::LEATHER;
    pub const MEMBRANE_V: u32 = 0x36122f | mat::LEATHER;
    pub const BONE: u32 = 0x8c5a7e | mat::SKIN;
    pub const CLAW: u32 = 0x2a1a22 | mat::IVORY;
    pub const SUCKER: u32 = 0xd89ac0 | mat::SKIN;
    pub const SUCKER_D: u32 = 0x7a3a66 | mat::SKIN;
}

/// The Cyclops' grid: 90 wide (wings spread), 68 tall (horns), 58 deep.
pub const CYCLOPS_SIZE: (i32, i32, i32) = (90, 68, 58);
/// Its voxels' size: twice as fine as its first model, the same size in the world.
pub const CYCLOPS_VOXEL: f32 = 0.035;
/// Center of the great eye in its voxel grid; the iris and lids pivot here.
pub const CYCLOPS_EYE: [f32; 3] = [44.5, 36.0, 31.5];
const EYE_R: f32 = 18.6;
const TENTACLES: [(f32, f32); 4] = [(32.5, 18.5), (56.5, 18.5), (36.5, 34.5), (52.5, 34.5)];

/// Wings hinge at the body; tentacles hang from their roots.
pub const CYCLOPS_RIG: &[PartDef] = &[
    PartDef { name: "leftWing", boxes: &[], pivot: [21.5, 44.0, 21.5] },
    PartDef { name: "rightWing", boxes: &[], pivot: [67.5, 44.0, 21.5] },
    PartDef { name: "tentacle0", boxes: &[], pivot: [32.5, 12.0, 18.5] },
    PartDef { name: "tentacle1", boxes: &[], pivot: [56.5, 12.0, 18.5] },
    PartDef { name: "tentacle2", boxes: &[], pivot: [36.5, 12.0, 34.5] },
    PartDef { name: "tentacle3", boxes: &[], pivot: [52.5, 12.0, 34.5] },
];

/// Is (x, y) inside the polygon `poly`?
fn inside(poly: &[(f32, f32)], x: f32, y: f32) -> bool {
    let mut odd = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[j]);
        if (a.1 > y) != (b.1 > y) && x < (b.0 - a.0) * (y - a.1) / (b.1 - a.1) + a.0 {
            odd = !odd;
        }
        j = i;
    }
    odd
}

/// Distance from (x, y) to the segment a-b.
fn to_segment(a: (f32, f32), b: (f32, f32), x: f32, y: f32) -> f32 {
    let ab = (b.0 - a.0, b.1 - a.1);
    let t = (((x - a.0) * ab.0 + (y - a.1) * ab.1) / (ab.0 * ab.0 + ab.1 * ab.1).max(1e-6)).clamp(0.0, 1.0);
    ((x - a.0 - ab.0 * t).powi(2) + (y - a.1 - ab.1 * t).powi(2)).sqrt()
}

/**
 * The Cyclops: a round, warty, mauve body that is mostly one enormous wet
 * eye, veined red toward its rim; a grin of crooked fangs under it; horns
 * ringed with growth, curling up and out; bat wings webbed between long
 * finger bones, a claw at each wrist; and four tentacles dangling beneath,
 * suckers down their inner sides. The iris and the lids are separate layers
 * (`cyclops_iris`, `cyclops_lid`) so it can look around and blink.
 */
pub fn cyclops() -> Collected {
    use cyc::*;
    let (w, h, d) = CYCLOPS_SIZE;
    let [ex, ey, ez] = CYCLOPS_EYE;
    let mut s = Sculpt::new(w, h, d);

    // ---- wings: long finger bones, the membrane webbed between them, scalloped
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        // Points in (distance out from the middle, height).
        let (root, wrist) = ((21.0f32, 44.0f32), (33.0f32, 52.0f32));
        let tips = [(43.5f32, 50.0f32), (42.5, 38.0), (36.0, 29.0)];
        let body = (21.0f32, 31.0f32);
        let poly = [root, wrist, tips[0], tips[1], tips[2], body];
        for y in 20..=56 {
            for u10 in 190..=440 {
                let u = u10 as f32 / 10.0;
                if !inside(&poly, u, y as f32) {
                    continue;
                }
                // Scallops bitten out of the trailing edge between the tips.
                let edges = [(tips[0], tips[1]), (tips[1], tips[2]), (tips[2], body)];
                let bitten = edges.iter().any(|&(a, b)| {
                    let m = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
                    let len = ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
                    // The bite's centre sits out past the edge, away from the wrist.
                    let away = ((m.0 - wrist.0), (m.1 - wrist.1));
                    let n = (away.0 * away.0 + away.1 * away.1).sqrt();
                    let c = (m.0 + away.0 / n * len * 0.42, m.1 + away.1 / n * len * 0.42);
                    ((u - c.0).powi(2) + (y as f32 - c.1).powi(2)).sqrt() < len * 0.56
                });
                if bitten {
                    continue;
                }
                let x = (ex + side * u).round() as i32;
                let veined = tips.iter().any(|&t| to_segment(wrist, t, u, y as f32) < 0.6)
                    || [0.3f32, 0.6].iter().any(|&k| to_segment(wrist, (body.0 + (tips[2].0 - body.0) * k, body.1 + (tips[2].1 - body.1) * k), u, y as f32) < 0.5);
                let c = if veined { MEMBRANE_V } else if nz(x / 3, y / 3, 1) > 0.6 { MEMBRANE_L } else { MEMBRANE };
                s.set(x, y, 20, c);
                s.set(x, y, 21, if veined { MEMBRANE_V } else { MEMBRANE });
            }
        }
        let at = |p: (f32, f32), z: f32| (ex + side * p.0, p.1, z);
        s.capsule(at(root, 21.0), at(wrist, 21.0), 2.2, 1.6, BONE);
        for (k, &t) in tips.iter().enumerate() {
            s.capsule(at(wrist, 21.0), at(t, 21.0), 1.3 - k as f32 * 0.1, 0.6, BONE);
        }
        s.capsule(at(wrist, 21.0), at((wrist.0 + 1.5, wrist.1 + 3.5), 22.5), 1.3, 0.5, CLAW);
    }

    // ---- the body, mottled and warty, pale beneath
    s.part(0);
    s.ellipsoid((ex, ey, 26.5), (24.0, 23.0, 23.0), 0, SKIN);
    s.paint(|x, y, z, _| {
        let n = nz(x, y, z);
        let patch = nz(x / 4, y / 4, z / 4);
        Some(if (y as f32) < ey - 14.0 + (nz(x / 2, 0, z / 2) - 0.5) * 6.0 {
            if n < 0.3 { SKIN } else { SKIN_L }
        } else if (y as f32) > ey + 12.0 && (z as f32) < 24.0 {
            if patch > 0.6 { SKIN_M } else { SKIN_D }
        } else if patch > 0.7 {
            SKIN_M
        } else {
            SKIN
        })
    });
    // Warts, each catching the light on top.
    for k in 0..46 {
        let a = nz(k, 11, 3) * TAU;
        let b = (nz(k, 5, 17) * 2.0 - 1.0).asin();
        let dir = Vec3::new(b.cos() * a.cos(), b.sin(), b.cos() * a.sin());
        // Not on the eye or the grin.
        if dir.z > 0.35 && dir.y > -0.75 {
            continue;
        }
        let p = Vec3::new(ex, ey, 26.5) + dir * Vec3::new(24.0, 23.0, 23.0);
        let r = 1.0 + nz(k, 2, 2) * 1.2;
        ball(&mut s, p, Vec3::splat(r), WART);
        s.recolor(p.x.round() as i32, (p.y + r).round() as i32, p.z.round() as i32, WART_L);
    }

    // ---- the eye bulges out of the front, bloodshot toward its rim
    s.ellipsoid((ex, ey, ez), (EYE_R, EYE_R, EYE_R), 0, SCLERA);
    s.paint(|x, y, z, c| {
        if c != SCLERA {
            return None;
        }
        let (dx, dy, dz) = (x as f32 - ex, y as f32 - ey, z as f32 - ez);
        let front = dz / (dx * dx + dy * dy + dz * dz).sqrt().max(0.01);
        (front < 0.45).then_some(SCLERA_D)
    });
    for k in 0..11 {
        let mut phi = k as f32 / 11.0 * TAU + nz(k, 1, 1) * 0.3;
        let reach = 0.62 + nz(k, 2, 1) * 0.25;
        let mut theta = 1.25f32;
        let mut step = 0;
        while theta > reach {
            let r = EYE_R + 0.4;
            let p = Vec3::new(ex + theta.sin() * phi.cos() * r, ey + theta.sin() * phi.sin() * r, ez + theta.cos() * r);
            let (x, y, z) = (p.x.round() as i32, p.y.round() as i32, p.z.round() as i32);
            for zz in [z, z - 1] {
                if matches!(s.get(x, y, zz), Some(SCLERA) | Some(SCLERA_D)) {
                    s.recolor(x, y, zz, if theta > 0.9 { VEIN_D } else { VEIN });
                    break;
                }
            }
            // A branch now and then.
            if step % 7 == 3 {
                let bp = phi + if nz(k, step, 4) > 0.5 { 0.12 } else { -0.12 };
                for j in 0..4 {
                    let t2 = theta - j as f32 * 0.05;
                    let q = Vec3::new(ex + t2.sin() * (bp + (bp - phi) * j as f32).cos() * r, ey + t2.sin() * (bp + (bp - phi) * j as f32).sin() * r, ez + t2.cos() * r);
                    let (x, y, z) = (q.x.round() as i32, q.y.round() as i32, q.z.round() as i32);
                    if matches!(s.get(x, y, z), Some(SCLERA) | Some(SCLERA_D)) {
                        s.recolor(x, y, z, VEIN);
                    }
                }
            }
            phi += (nz(k, step, 9) - 0.5) * 0.14;
            theta -= 0.035;
            step += 1;
        }
    }

    // ---- a grin of crooked fangs under the eye
    let mouth = |x: i32| {
        let t = (x as f32 - ex) / 14.0;
        (ey - 20.0 + t * t * 6.0, ey - 12.5 + t * t * 3.0)
    };
    let mut lips = Vec::new();
    for x in (ex - 14.0) as i32..=(ex + 14.0) as i32 {
        let (lo, hi) = mouth(x);
        for y in lo.round() as i32..=hi.round() as i32 {
            let Some(z) = front(&s, x, y, d) else { continue };
            if (z as f32) < 36.0 {
                continue;
            }
            let deep = 3 + ((y as f32 - lo) * (hi - y as f32)).max(0.0).sqrt() as i32;
            for k in 0..deep {
                s.clear(x, y, z - k);
            }
            let tongue = (y as f32) < lo + 2.0 && (x as f32 - ex).abs() < 8.0;
            s.recolor(x, y, z - deep, if tongue { TONGUE } else if deep > 4 { THROAT } else { MAW });
            lips.push((x, y, z));
        }
    }
    // Gums round the rim, fangs hanging from the top and jutting from below.
    for &(x, y, _) in &lips {
        let (lo, hi) = mouth(x);
        if y == lo.round() as i32 || y == hi.round() as i32 {
            if let Some(z) = front(&s, x, y, d) {
                s.recolor(x, y, z, GUM);
            }
        }
    }
    for (k, x0) in ((ex - 12.0) as i32..=(ex + 12.0) as i32).step_by(4).enumerate() {
        let (lo, hi) = mouth(x0);
        let long = 3 + (nz(k as i32, 4, 4) * 3.0) as i32;
        for (top, len) in [(true, long), (false, long - 1)] {
            for j in 0..len {
                let y = if top { hi.round() as i32 - 1 - j } else { lo.round() as i32 + 1 + j };
                let half = ((len - j) as f32 / len as f32 * 1.7).round() as i32;
                for x in x0 - half..=x0 + half {
                    if let Some(&(_, _, z)) = lips.iter().find(|&&(lx, ly, _)| lx == x && ly == y) {
                        s.set(x, y, z - 1, if j == len - 1 || x != x0 { TOOTH_D } else { TOOTH });
                    }
                }
            }
        }
    }

    // ---- horns, ringed with growth, curling up and out
    for side in [-1.0f32, 1.0] {
        for i in 0..24 {
            let k = i as f32 / 23.0;
            let c = (ex + side * (12.0 + k * 10.0 + (k * 2.6).sin() * 3.0), 54.0 + k * 12.0 - k * k * 3.0, 23.5 + k * 5.0);
            let r = 3.9 - k * 2.7;
            let ring = (k * 9.0).fract() < 0.22;
            s.ellipsoid(c, (r, r, r), 0, if k > 0.74 { HORN_D } else if ring { HORN_R } else { HORN });
        }
    }

    // ---- four tentacles hanging beneath, suckers down their inner sides, curling at the tips
    for (k, &(tx, tz)) in TENTACLES.iter().enumerate() {
        s.part(3 + k as u8);
        let lean = if k % 2 == 0 { -1.0 } else { 1.0 };
        // From inside the body down to a curled tip, banded dark and light.
        let mut prev: Option<Vec3> = None;
        for j in 0..=34 {
            let y = 19.0 - j as f32 * 0.54;
            let tip = (1.0 - y / 6.0).max(0.0);
            let p = v(tx + lean * tip * tip * 4.4, y + tip * tip * 1.6, tz + tip * 1.6);
            let r = 1.6 + (y.min(14.0) / 14.0) * 2.6;
            if let Some(q) = prev {
                let c = if (y as i32 / 2) % 2 == 0 { SKIN_D } else { SKIN };
                s.capsule((q.x, q.y, q.z), (p.x, p.y, p.z), r + 0.1, r, c);
            }
            prev = Some(p);
        }
        // Suckers facing the middle of the body.
        let inward = Vec3::new(ex - tx, 0.0, 26.5 - tz).normalize();
        for y in (2..=12).step_by(2) {
            let tip = (1.0 - y as f32 / 6.0).max(0.0);
            let r = 1.6 + (y as f32 / 14.0) * 2.6;
            let p = v(tx + lean * tip * tip * 4.4, y as f32 + tip * tip * 1.6, tz + tip * 1.6) + inward * (r + 0.2);
            s.set(p.x.round() as i32, y, p.z.round() as i32, SUCKER);
            let o = p + inward * 0.9;
            s.set(o.x.round() as i32, y, o.z.round() as i32, SUCKER_D);
        }
    }
    s.finish(0.05)
}

/// The iris and slit pupil: a cap sitting just proud of the eye's front, streaked, ringed dark.
pub fn cyclops_iris() -> Collected {
    use cyc::*;
    let (w, h, d) = CYCLOPS_SIZE;
    let [ex, ey, ez] = CYCLOPS_EYE;
    let mut s = Sculpt::new(w, h, d);
    for y in 0..h {
        for z in 0..d {
            for x in 0..w {
                let (dx, dy, dz) = (x as f32 - ex, y as f32 - ey, z as f32 - ez);
                let r = (dx * dx + dy * dy + dz * dz).sqrt();
                if r <= EYE_R || r > EYE_R + 1.6 || dz <= 0.0 {
                    continue;
                }
                let off = (dx * dx + dy * dy).sqrt();
                if off > 9.4 {
                    continue;
                }
                let streak = ((dy.atan2(dx) * 11.0).sin() + nz(x, y, 3) - 0.5) > 0.7;
                let c = if dx.abs() < 1.6 * (1.0 - (dy / 7.6).powi(2)).max(0.0).sqrt() + 0.2 && dy.abs() < 7.6 {
                    PUPIL
                } else if off > 8.4 {
                    LIMBUS
                } else if off > 6.8 {
                    if streak { IRIS } else { IRIS_D }
                } else if streak {
                    IRIS_L
                } else {
                    IRIS
                };
                s.set(x, y, z, c);
            }
        }
    }
    let fz = (ez + EYE_R + 1.2) as i32;
    for (x, y) in [(-5, 4), (-4, 4), (-5, 5), (-4, 5), (-3, 6), (4, -5)] {
        s.set(ex as i32 + x, ey as i32 + y, fz, SHINE);
    }
    s.finish(0.04)
}

/**
 * The eyelid: a shell over the top of the eye, folded back at rest; it swings
 * down over the whole eye to blink. (There is no lower lid: it would hide the
 * grin, which sits right under the eye.)
 */
pub fn cyclops_lid() -> Collected {
    use cyc::*;
    let (w, h, d) = CYCLOPS_SIZE;
    let [ex, ey, ez] = CYCLOPS_EYE;
    let mut s = Sculpt::new(w, h, d);
    for y in 0..h {
        for z in 0..d {
            for x in 0..w {
                let (dx, dy, dz) = (x as f32 - ex, y as f32 - ey, z as f32 - ez);
                let r = (dx * dx + dy * dy + dz * dz).sqrt();
                if r <= EYE_R + 1.6 || r > EYE_R + 3.4 || dz <= -4.0 {
                    continue;
                }
                // Its edge arches, lower at the corners.
                let elev = (dy / r).asin();
                let edge = 0.56 + (dx / r).powi(2) * 0.3;
                if elev <= edge {
                    continue;
                }
                let rim = (elev - edge).abs() < 0.08;
                let crease = (elev - edge).abs() > 0.2 && (elev - edge).abs() < 0.26;
                s.set(x, y, z, if rim { SKIN_L } else if crease { SKIN_D } else if nz(x, y, z) < 0.06 { WART } else { SKIN });
            }
        }
    }
    s.finish(0.05)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::voxels::{build_layered_rig, build_rig};

    fn within(c: &Collected, (w, h, d): (i32, i32, i32)) -> bool {
        c.voxels.iter().all(|v| (0..w).contains(&v.x) && (0..h).contains(&v.y) && (0..d).contains(&v.z))
    }

    #[test]
    fn every_foe_is_sculpted_into_its_parts_within_a_budget() {
        for (name, model, rig, size, budget) in [
            ("draugr", draugr(), DRAUGR_RIG, DRAUGR_SIZE, 9_000),
            ("raven", raven(false), RAVEN_RIG, RAVEN_SIZE, 6_000),
            ("night raven", raven(true), RAVEN_RIG, RAVEN_SIZE, 6_000),
            ("bog slime", bog_slime(), &[] as &[PartDef], SLIME_SIZE, 6_000),
        ] {
            let n = model.voxels.len();
            eprintln!("{name}: {n} voxels");
            assert!(within(&model, size), "{name} spills out of its grid");
            assert!(n > 1_500 && n < budget, "{name} has {n} voxels");
            let r = build_rig(&model, rig, 0.0125);
            for part in &r.parts {
                assert!(part.cubes.len() > 40, "{name}'s {} has {} cubes", part.name, part.cubes.len());
            }
            let far = r.coarse();
            let count = |r: &crate::render::voxels::Rig| r.parts.iter().map(|p| p.cubes.len()).sum::<usize>();
            assert!(count(&far) * 5 < count(&r) * 2, "{name}'s far copy has {} cubes against {}", count(&far), count(&r));
        }
    }

    #[test]
    fn the_cyclops_has_an_eye_that_moves_and_lids_that_blink() {
        let body = cyclops();
        eprintln!("cyclops: {} voxels", body.voxels.len());
        assert!(within(&body, CYCLOPS_SIZE));
        assert!(body.voxels.len() < 30_000, "{} voxels", body.voxels.len());
        let r = build_layered_rig(
            &body,
            CYCLOPS_RIG,
            &[("iris", &cyclops_iris(), CYCLOPS_EYE), ("lid", &cyclops_lid(), CYCLOPS_EYE)],
            CYCLOPS_VOXEL,
        );
        for name in ["body", "iris", "lid", "leftWing", "rightWing", "tentacle0", "tentacle3"] {
            let i = r.index(name).unwrap_or_else(|| panic!("no {name}"));
            assert!(r.parts[i].cubes.len() > 40, "{name} has {} cubes", r.parts[i].cubes.len());
        }
        // Its fangs, the veins in its eye and its suckers are there to be seen (colours jittered a little).
        let close = |a: u32, b: u32| [16, 8, 0].iter().all(|&k| (((a >> k) & 255) as i32 - ((b >> k) & 255) as i32).abs() <= 14);
        let has = |c: u32| body.voxels.iter().any(|v| v.c >> 24 == c >> 24 && close(v.c, c));
        assert!(has(cyc::TOOTH) && has(cyc::VEIN) && has(cyc::SUCKER));
    }
}
