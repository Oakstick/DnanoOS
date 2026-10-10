//! The three heroes as they are played and shown: Ragnar the viking, Mehmed
//! the janissary and Miyamoto the samurai, sculpted finely (voxels 0.0125
//! across, half the size of their first detailed models, which they keep the
//! build and joints of): eyes with whites, irises and a glint, beards in
//! strands, riveted iron, lacquered lames laced with silk, damask worked with
//! gold. Each is sculpted in the parts his rig swings (legs, arms, head; see
//! `Sculpt::part`), faces +z, and has his weapon from the armory in his right
//! fist and a shield of his tree carved into his back.

use std::f32::consts::{PI, TAU};

use glam::Vec3;

use super::carve::*;
use super::characters::{mat, Sculpt};
use super::voxels::{Collected, PartDef, Voxel};
use crate::logic::heroes::HeroKind;
use crate::logic::progression::Form;

/// Every hero's rig parts, in the order of their `PartDef`s.
const LEFT_LEG: u8 = 1;
const SHIELD_ARM: u8 = 3;
const HEAD: u8 = 5;

/// Joints shared by the three rigs, from each hero's own pivots.
const fn rig(legs: [f32; 2], leg_y: f32, leg_z: f32, arms: [f32; 2], arm_y: f32, arm_z: f32, head: [f32; 3]) -> [PartDef; 5] {
    [
        PartDef { name: "leftLeg", boxes: &[], pivot: [legs[0], leg_y, leg_z] },
        PartDef { name: "rightLeg", boxes: &[], pivot: [legs[1], leg_y, leg_z] },
        PartDef { name: "shieldArm", boxes: &[], pivot: [arms[0], arm_y, arm_z] },
        PartDef { name: "swordArm", boxes: &[], pivot: [arms[1], arm_y, arm_z] },
        PartDef { name: "head", boxes: &[], pivot: head },
    ]
}

/// A fist closed round a grip that runs up through it: the fingers wrapped
/// round the front (their knuckles stacked down it), the thumb over the top.
fn fist(s: &mut Sculpt, c: Vec3, r: Vec3, skin: u32, knuckle: u32) {
    ball(s, c, r, skin);
    for k in 0..4 {
        let y = c.y + r.y * 0.62 - k as f32 * r.y * 0.42;
        ball(s, v(c.x, y, c.z + r.z * 0.72), v(r.x * 0.82, 0.8, 1.0), if k % 2 == 0 { knuckle } else { skin });
    }
    ball(s, v(c.x, c.y + r.y * 0.9, c.z + r.z * 0.2), v(r.x * 0.5, 0.9, r.z * 0.5), skin);
}

/// Strands down a beard or a mane: each column its own shade, a slow wave through them.
fn strands(s: &mut Sculpt, of: &[u32], dark: u32, mid: u32, light: u32) {
    s.paint(|x, y, z, c| {
        if !of.contains(&c) {
            return None;
        }
        let strand = x + ((y as f32 * 0.24 + z as f32 * 0.3).sin() * 1.3).round() as i32;
        let n = nz(strand, z / 3, y / 12);
        Some(if n < 0.26 {
            dark
        } else if n > 0.74 {
            light
        } else {
            mid
        })
    });
}

// ------------------------------------------------------------------ Ragnar

mod viking {
    use super::mat;
    pub const SKIN: u32 = 0xf0c49c | mat::SKIN;
    pub const SKIN_D: u32 = 0xd49e76 | mat::SKIN;
    pub const BLUSH: u32 = 0xe8a486 | mat::SKIN;
    pub const SCAR: u32 = 0xf4d8c4 | mat::SKIN;
    pub const LIP: u32 = 0x9a4a36 | mat::SKIN;
    pub const IRIS: u32 = 0x2f5c9c;
    pub const BEARD: u32 = 0xc4581c | mat::FUR;
    pub const BEARD_L: u32 = 0xe2823a | mat::FUR;
    pub const BEARD_D: u32 = 0x93400f | mat::FUR;
    pub const HAIR: u32 = 0x8a3c10 | mat::FUR;
    pub const HELM: u32 = 0x8d96a3 | mat::METAL;
    pub const HELM_D: u32 = 0x6a727e | mat::METAL;
    pub const HELM_L: u32 = 0xaab2bc | mat::METAL;
    pub const RIVET: u32 = 0xe4e9ee | mat::METAL;
    pub const BRONZE: u32 = 0xc8903c | mat::GOLD;
    pub const BRONZE_D: u32 = 0x9a6a28 | mat::GOLD;
    pub const GOLD: u32 = 0xf2c53d | mat::GOLD;
    pub const HORN: u32 = 0xf2e8d0 | mat::IVORY;
    pub const HORN_M: u32 = 0xdccca6 | mat::IVORY;
    pub const HORN_TIP: u32 = 0xb4a176 | mat::IVORY;
    pub const ANTLER: u32 = 0xd8c8a8 | mat::IVORY;
    pub const TUNIC: u32 = 0x2e5f96 | mat::CLOTH;
    pub const TUNIC_D: u32 = 0x244d7c | mat::CLOTH;
    pub const TUNIC_L: u32 = 0x3a70aa | mat::CLOTH;
    pub const TRIM: u32 = 0xd0a848 | mat::CLOTH;
    pub const TRIM_R: u32 = 0x8a2a24 | mat::CLOTH;
    pub const CLOAK: u32 = 0x7d2a24 | mat::CLOTH;
    pub const CLOAK_D: u32 = 0x5e1e1a | mat::CLOTH;
    pub const CLOAK_L: u32 = 0x94362e | mat::CLOTH;
    pub const TROUSER: u32 = 0x55593f | mat::CLOTH;
    pub const TROUSER_D: u32 = 0x464a33 | mat::CLOTH;
    pub const FUR: u32 = 0x8f6c49 | mat::FUR;
    pub const FUR_L: u32 = 0xc2a27c | mat::FUR;
    pub const FUR_D: u32 = 0x654a30 | mat::FUR;
    pub const BELT: u32 = 0x5c3a1c | mat::LEATHER;
    pub const LEATHER: u32 = 0x7a4a24 | mat::LEATHER;
    pub const LEATHER_D: u32 = 0x55331a | mat::LEATHER;
    pub const STITCH: u32 = 0xd8bc8e | mat::LEATHER;
    pub const WRAP: u32 = 0xb89a68 | mat::LEATHER;
    pub const BOOT: u32 = 0x4e321a | mat::LEATHER;
    pub const BOOT_D: u32 = 0x33200f | mat::LEATHER;
}

/// Ragnar's grid: 60 wide, 92 tall (his horns' tips), 36 deep.
pub const RAGNAR_SIZE: (i32, i32, i32) = (60, 92, 36);

/**
 * Ragnar: a riveted spangenhelm with a bronze brow band and nasal, ivory
 * horns ringed with growth; fierce blue eyes under bushy red brows, a broken
 * nose, a scar; a red beard in strands with two gold-ringed braids, a braid
 * down his back; a fur mantle with a gold brooch, a blue woollen tunic with
 * an embroidered hem; a stitched belt with a
 * gold buckle, a pouch and a seax across his front; laced leather bracers;
 * cross-gartered trousers and fur-cuffed, laced boots. His weapon is a model
 * of its own (see `armory`), held in his right fist; a shield of his tree
 * (form, colour, tier) is slung on his back, or none.
 */
pub fn ragnar_hd(shield: Option<(Form, u32, u8)>) -> Collected {
    use viking::*;
    let (w, h, d) = RAGNAR_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (29.5f32, 16.8f32);

    // ---- legs: laced boots with fur cuffs, cross-gartered trousers
    for (i, lx) in [21.5f32, 37.5].into_iter().enumerate() {
        s.part(LEFT_LEG + i as u8);
        s.ellipsoid((lx, 2.0, 20.5), (6.0, 6.0, 9.0), 0, BOOT);
        column(&mut s, lx, 19.0, 5.6, 6.0, 2, 11, BOOT);
        let c = lx.round() as i32;
        for y in 4..=10 {
            let xs = if y % 2 == 0 { [c - 2, c + 1] } else { [c - 1, c] };
            for x in xs {
                face(&mut s, x, y, d, 0, WRAP);
            }
        }
        column(&mut s, lx, 19.5, 5.6, 5.8, 14, 27, TROUSER);
        column(&mut s, lx, 19.0, 7.0, 7.4, 10, 14, FUR);
    }
    s.paint(|x, y, z, c| {
        if c == BOOT && y <= 1 {
            return Some(BOOT_D);
        }
        if c == TROUSER {
            // Leg wraps crossing over and under, bound with leather thongs.
            let lx = if x < 30 { 21.5 } else { 37.5 };
            let u = turn(x, z, lx, 19.5) * 12.0;
            let f = y as f32 * 0.55;
            if (u + f).rem_euclid(6.0) < 0.9 || (u - f).rem_euclid(6.0) < 0.9 {
                return Some(WRAP);
            }
            return Some(if nz(x, y / 2, z) < 0.1 { TROUSER_D } else { TROUSER });
        }
        None
    });

    // ---- the tunic: a flared skirt, a deep chest, an embroidered hem
    s.part(0);
    for y in 28..=55 {
        let (rx, rz) = if y <= 33 {
            (17.2 - (y - 28) as f32 * 0.15, 9.0 - (y - 28) as f32 * 0.15)
        } else {
            let chest = ((y as f32 - 34.0) / 21.0 * PI).sin();
            (15.4 + chest * 0.4, 6.8 + chest * 1.0)
        };
        slab(&mut s, cx, cz, rx, rz, 2.6, y, y, TUNIC);
    }
    s.paint(|x, y, z, c| {
        if c != TUNIC {
            return None;
        }
        let seg = (turn(x, z, cx, cz) * 96.0) as i32;
        if y <= 30 {
            // A band of gold embroidery between two red ones, a zigzag worked into it.
            return Some(if y == 28 || y == 30 || (seg + y) % 4 == 0 { TRIM_R } else { TRIM });
        }
        if y <= 33 && ((turn(x, z, cx, cz) * TAU) * 10.0).sin() > 0.8 {
            return Some(TUNIC_D); // folds in the skirt
        }
        let n = nz(x, y, z);
        Some(if n < 0.07 {
            TUNIC_D
        } else if n > 0.94 {
            TUNIC_L
        } else {
            TUNIC
        })
    });
    // ---- the belt: stitched leather, a gold buckle with its prong, the tongue hanging
    slab(&mut s, cx, cz, 16.8, 8.8, 2.6, 31, 35, BELT);
    s.paint(|x, y, z, c| (c == BELT && (y == 31 || y == 35) && (x + z) % 2 == 0).then_some(STITCH));
    for y in 30..=36 {
        for x in 26..=33 {
            let rim = y == 30 || y == 36 || x == 26 || x == 33;
            if rim {
                face(&mut s, x, y, d, 1, GOLD);
            } else if y == 33 {
                face(&mut s, x, y, d, 0, BRONZE_D);
            }
        }
    }
    for y in 24..=30 {
        for x in 35..=36 {
            face(&mut s, x, y, d, if y > 29 { 0 } else { 1 }, if y <= 25 { BRONZE } else { LEATHER });
        }
    }
    // A pouch at his right hip, its flap stitched, a gold toggle.
    s.ellipsoid((42.0, 31.0, 25.0), (3.8, 4.6, 2.4), 28, LEATHER);
    s.paint(|x, y, z, c| {
        if c != LEATHER || !(38..=46).contains(&x) || y < 28 || z < 23 {
            return None;
        }
        if y >= 32 {
            Some(if y == 32 && x % 2 == 0 { STITCH } else { LEATHER_D })
        } else {
            None
        }
    });
    face(&mut s, 42, 31, d, 1, GOLD);
    // A seax in its sheath across his front, bronze at the chape, an antler grip.
    s.capsule((17.0, 29.4, 26.0), (27.0, 28.8, 26.6), 1.3, 1.2, LEATHER_D);
    ball(&mut s, v(16.6, 29.4, 26.0), Vec3::splat(1.4), BRONZE);
    s.capsule((27.8, 28.8, 26.6), (32.0, 28.6, 26.8), 1.0, 0.9, ANTLER);
    ball(&mut s, v(32.6, 28.6, 26.8), Vec3::splat(1.2), BRONZE);
    ball(&mut s, v(27.6, 28.8, 26.6), v(0.8, 1.7, 1.7), BRONZE_D);

    // ---- arms: sleeves bunched at the elbow, laced bracers, fists
    for (i, ax) in [7.5f32, 51.5].into_iter().enumerate() {
        s.part(SHIELD_ARM + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        s.capsule((ax, 53.0, 16.5), (ax, 44.0, 17.0), 4.3, 4.2, TUNIC);
        ball(&mut s, v(ax, 45.0, 17.0), v(4.4, 1.6, 4.9), TUNIC_D);
        column(&mut s, ax, 17.0, 4.0, 4.6, 34, 43, LEATHER);
        column(&mut s, ax, 17.0, 4.3, 4.9, 34, 34, LEATHER_D);
        column(&mut s, ax, 17.0, 4.3, 4.9, 43, 43, LEATHER_D);
        let ox = (ax + out * 3.9).round() as i32;
        for y in 35..=42 {
            let k = (y - 35) % 4;
            for z in [15 + k.min(3 - k), 19 - k.min(3 - k)] {
                s.set(ox, y, z, STITCH);
            }
        }
        fist(&mut s, v(ax, 29.8, 17.4), v(4.1, 4.3, 4.6), SKIN, SKIN_D);
    }

    // ---- a fur mantle over the shoulders, shaggy at its edge, a gold brooch
    s.part(0);
    s.ellipsoid((cx, 57.0, 16.0), (23.6, 6.6, 12.4), 51, FUR);
    for y in 58..=66 {
        for z in 0..d {
            for x in 0..w {
                if ((x as f32 - cx) / 9.8).powi(2) + ((z as f32 - 16.5) / 7.6).powi(2) < 1.0 {
                    s.clear(x, y, z);
                }
            }
        }
    }
    s.paint(|x, y, z, c| {
        if c != FUR || y < 50 {
            return None;
        }
        let n = nz(x, y / 2, z);
        Some(if n < 0.28 {
            FUR_D
        } else if n > 0.74 {
            FUR_L
        } else {
            FUR
        })
    });
    let bz = front(&s, 41, 58, d).unwrap_or(26);
    for k in 0..14 {
        let a = k as f32 / 14.0 * TAU;
        s.set((41.0 + a.cos() * 2.2).round() as i32, (58.0 + a.sin() * 2.2).round() as i32, bz + 1, GOLD);
    }
    s.set(41, 58, bz + 1, BRONZE_D);

    // ---- the head: square-jawed, set on a thick neck
    s.part(HEAD);
    column(&mut s, cx, 16.5, 5.2, 4.6, 56, 61, SKIN);
    rounded(&mut s, v(cx, 70.0, 15.8), v(9.9, 10.2, 7.9), 3.2, 60, SKIN);
    // Red hair round the back and over the ears, under the helmet.
    s.paint(|x, y, z, c| {
        let dx = (x as f32 - cx).abs();
        (c == SKIN && ((z <= 12 && y >= 62) || (dx >= 8.4 && y >= 70))).then_some(HAIR)
    });
    for ex in [19.8f32, 39.2] {
        ball(&mut s, v(ex, 67.5, 15.2), v(1.3, 2.6, 2.0), SKIN_D);
    }
    // Fierce eyes, bushy brows knitted over them, a broken nose, a scar, weathered cheeks.
    eye(&mut s, 22, 68, 1, d, IRIS, SKIN_D);
    eye(&mut s, 37, 68, -1, d, IRIS, SKIN_D);
    for x in [25, 34] {
        face(&mut s, x, 68, d, 0, SKIN_D);
        face(&mut s, x, 69, d, 0, SKIN_D);
    }
    for x in (21..=26).chain(33..=38) {
        face(&mut s, x, 71, d, 1, BEARD_D);
    }
    for x in (22..=25).chain(34..=37) {
        face(&mut s, x, 72, d, 1, BEARD);
    }
    for x in [26, 33] {
        face(&mut s, x, 70, d, 1, BEARD_D);
    }
    s.capsule((cx, 70.5, 23.6), (cx + 0.4, 65.6, 25.0), 1.2, 1.3, SKIN);
    ball(&mut s, v(cx + 0.3, 64.9, 24.8), v(2.1, 1.6, 1.6), SKIN);
    for x in [28, 31] {
        face(&mut s, x, 63, d, 0, SKIN_D);
    }
    for (x, y) in [(22, 65), (23, 65), (22, 66), (36, 65), (37, 65), (37, 66)] {
        face(&mut s, x, y, d, 0, BLUSH);
    }
    for (x, y) in [(34, 73), (35, 72), (35, 71), (36, 67), (36, 66)] {
        face(&mut s, x, y, d, 0, SCAR);
    }

    // ---- the beard: full and red, from ear to ear, lying on his chest
    let bc = v(cx, 54.0, 24.2);
    let br = v(11.6, 12.8, 5.4);
    for y in 40..=67 {
        for z in 12..d {
            for x in 14..46 {
                let q = (Vec3::new(x as f32, y as f32, z as f32) - bc) / br;
                if q.length_squared() > 1.0 {
                    continue;
                }
                let side = (x as f32 - cx).abs() >= 7.6;
                let over_mouth = y > 63 && !side;
                if over_mouth || (y > 62 && s.get(x, y, z) == Some(SKIN) && !side) {
                    continue;
                }
                s.set(x, y, z, BEARD);
            }
        }
    }
    // A drooping moustache, and the mouth in the shadow under it.
    for x in 27..=32 {
        face(&mut s, x, 62, d, 0, LIP);
    }
    for side in [-1.0f32, 1.0] {
        s.capsule((cx + side * 0.6, 63.8, 25.4), (cx + side * 5.0, 62.6, 25.4), 1.5, 1.4, BEARD_L);
        s.capsule((cx + side * 5.0, 62.6, 25.4), (cx + side * 7.6, 58.4, 24.6), 1.4, 0.9, BEARD_L);
    }
    // Two braids from its tip, ringed with gold.
    for bx in [25.5f32, 33.5] {
        for y in 33..=44 {
            let fy = y as f32;
            ball(&mut s, v(bx, fy, 28.2 + (44.0 - fy) * 0.05), Vec3::splat(1.3), if y % 2 == 0 { BEARD } else { BEARD_D });
        }
        for y in [36.0f32, 42.0] {
            ball(&mut s, v(bx, y, 28.5), v(1.7, 0.8, 1.7), GOLD);
        }
        ball(&mut s, v(bx, 32.0, 28.6), v(1.6, 1.4, 1.4), BEARD_L);
    }
    strands(&mut s, &[BEARD], BEARD_D, BEARD, BEARD_L);
    // A braid down his back, a gold ring round it.
    for y in 44..=72 {
        let fy = y as f32;
        let z = 5.5 + (fy - 44.0) * 0.08;
        let c = if (48..=49).contains(&y) {
            GOLD
        } else if y % 3 == 0 {
            BEARD_D
        } else {
            HAIR
        };
        ball(&mut s, v(cx, fy, z), v(1.6, 0.8, 1.2), c);
    }

    // ---- the helmet: a hammered iron spangenhelm, a riveted bronze band and strips, a nasal
    s.ellipsoid((cx, 75.0, 15.8), (11.8, 14.0, 9.8), 74, HELM);
    s.paint(|x, y, z, c| {
        if c != HELM {
            return None;
        }
        let seg = (turn(x, z, cx, 15.8) * 72.0) as i32;
        if y <= 77 {
            return Some(if y == 76 && seg % 3 == 0 {
                RIVET
            } else if y == 74 {
                BRONZE_D
            } else {
                BRONZE
            });
        }
        let strip = (x as f32 - cx).abs() < 1.1 || (z as f32 - 15.8).abs() < 1.1;
        if strip {
            return Some(if y % 4 == 0 { RIVET } else { BRONZE_D });
        }
        let n = nz(x, y, z);
        Some(if n < 0.1 {
            HELM_D
        } else if n > 0.92 {
            HELM_L
        } else {
            HELM
        })
    });
    ball(&mut s, v(cx, 88.6, 15.8), Vec3::splat(1.6), BRONZE);
    for y in 69..=77 {
        for x in [29, 30] {
            face(&mut s, x, y, d, 1, if y == 69 { BRONZE } else { HELM_D });
        }
    }
    // ---- horns sweeping up and out from bronze collars, ringed with growth
    for side in [-1.0f32, 1.0] {
        let steps = 44;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = cx + side * (11.2 + 14.4 * t - 1.2 * t * t);
            let y = 78.4 + 3.2 * t + 8.4 * t * t;
            let z = 15.8 - 2.4 * t;
            let ring = (t * 9.0).fract() < 0.16 && t > 0.15 && t < 0.82;
            let r = 3.5 * (1.0 - t) + 0.9 + if ring { 0.35 } else { 0.0 };
            let color = if t < 0.08 {
                BRONZE
            } else if t > 0.82 {
                HORN_TIP
            } else if ring {
                HORN_M
            } else {
                HORN
            };
            ball(&mut s, v(x, y, z), Vec3::splat(r), color);
        }
    }

    // ---- fur standing out of the mantle and the boots' cuffs, the shield on his back
    shag(&mut s, (w, h, d), FUR, FUR_D, FUR_L, 0.22, 3);
    s.part(0);
    if let Some((form, color, tier)) = shield {
        super::armory::back_shield(&mut s, form, color, tier);
    }
    s.finish_solid(0.03)
}

/// Ragnar's joints.
pub const RAGNAR_HD_RIG: &[PartDef] = &rig([21.5, 37.5], 28.0, 19.5, [10.5, 48.5], 54.0, 16.5, [29.5, 58.0, 15.5]);

// ------------------------------------------------------------------ Mehmed

mod jan {
    use super::mat;
    pub const SKIN: u32 = 0xd9a47c | mat::SKIN;
    pub const SKIN_D: u32 = 0xb8825c | mat::SKIN;
    pub const LIP: u32 = 0x3a1c12 | mat::SKIN;
    pub const IRIS: u32 = 0x4a2c16;
    pub const HAIR: u32 = 0x221812 | mat::FUR;
    pub const BEARD: u32 = 0x1e1510 | mat::FUR;
    pub const BEARD_D: u32 = 0x120c08 | mat::FUR;
    pub const BEARD_L: u32 = 0x3a2a1e | mat::FUR;
    pub const BROW: u32 = 0x18110c | mat::FUR;
    pub const FELT: u32 = 0xe2dccc | mat::CLOTH;
    pub const FELT_D: u32 = 0xc6bead | mat::CLOTH;
    pub const GOLD: u32 = 0xf2c53d | mat::GOLD;
    pub const GOLD_D: u32 = 0xb88a2a | mat::GOLD;
    pub const GEM: u32 = 0x1fb8a4;
    pub const GEM_L: u32 = 0x8af0e0;
    pub const KAFTAN: u32 = 0x1d6a3a | mat::CLOTH;
    pub const KAFTAN_D: u32 = 0x145230 | mat::CLOTH;
    pub const THREAD: u32 = 0xae8c36 | mat::CLOTH;
    pub const ROBE: u32 = 0x8e1c26 | mat::CLOTH;
    pub const ROBE_D: u32 = 0x6e141c | mat::CLOTH;
    pub const SASH: u32 = 0xa01e2a | mat::CLOTH;
    pub const SASH_D: u32 = 0x7a1620 | mat::CLOTH;
    pub const BOOT: u32 = 0x8a2a1c | mat::LEATHER;
    pub const BOOT_D: u32 = 0x5e1c12 | mat::LEATHER;
}

/// Mehmed's grid: 96 wide (his body in the middle), 124 tall (his börk), 40 deep.
pub const MEHMED_SIZE: (i32, i32, i32) = (96, 124, 40);
/// Where Mehmed's shield sits, as an offset from where Ragnar's does.
pub const MEHMED_SHIELD_AT: (i32, i32, i32) = (18, 0, 6);

/**
 * Mehmed the janissary: a tall white börk of felt on a gold band, its flap
 * folded down his back, a gilt spoon-holder plate on its front set with a
 * green stone under a plume socket; dark eyes under heavy, knitted brows, a
 * strong nose, a black beard trimmed square and a heavy moustache curling up
 * at its ends; a green kaftan worked with a gold damask to the ankles,
 * frogged across the chest and open below over a crimson robe, a crimson
 * sash with a fringed end, wide sleeves with gold cuffs; red boots with
 * turned-up toes. His weapon comes from his armory; a shield of his tree
 * (form, colour, tier) is slung on his back, or none.
 */
pub fn mehmed_hd(shield: Option<(Form, u32, u8)>) -> Collected {
    use jan::*;
    let (w, h, d) = MEHMED_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (47.5f32, 19.5f32);

    // ---- red boots, the toes turned up
    for (i, lx) in [39.5f32, 55.5].into_iter().enumerate() {
        s.part(LEFT_LEG + i as u8);
        s.ellipsoid((lx, 3.0, 22.0), (5.8, 5.5, 8.6), 0, BOOT);
        column(&mut s, lx, 21.0, 5.2, 5.8, 3, 15, BOOT);
        s.capsule((lx, 2.4, 29.0), (lx, 5.8, 32.6), 1.9, 0.8, BOOT);
        column(&mut s, lx, 21.0, 5.5, 6.1, 14, 15, BOOT_D);
    }
    s.paint(|_x, y, _z, c| (c == BOOT && y == 0).then_some(BOOT_D));

    // ---- the kaftan's skirt, flaring to the ankles, open down the front over a crimson robe
    s.part(0);
    let skirt = |y: i32| {
        let t = (31 - y) as f32 / 23.0;
        (17.2 + t * 5.2, 9.4 + t * 3.2, 2.0 + t * 4.4)
    };
    for y in 8..=31 {
        let (rx, rz, _) = skirt(y);
        slab(&mut s, cx, cz, rx - 1.6, rz - 1.6, 2.4, y, y, if y % 5 == 0 { ROBE_D } else { ROBE });
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = ((x as f32 - cx) / rx, (z as f32 - cz) / rz);
                if dx.abs().powf(2.4) + dz.abs().powf(2.4) > 1.0 {
                    continue;
                }
                let (_, _, gap) = skirt(y);
                let off = (x as f32 - cx).abs();
                if dz > 0.4 && off < gap {
                    continue;
                }
                let edge = dz > 0.4 && off < gap + 1.3;
                s.set(x, y, z, if edge || y <= 9 { GOLD_D } else { KAFTAN });
            }
        }
    }
    // ---- the crimson sash, striped, a loose end hanging at the front with a gold fringe
    slab(&mut s, cx, cz, 19.4, 11.2, 2.4, 30, 35, SASH);
    s.paint(|_x, y, _z, c| (c == SASH && (y == 31 || y == 34)).then_some(SASH_D));
    for y in 14..=31 {
        for x in 36..=41 {
            let Some(fz) = front(&s, x, y, d) else { continue };
            let fringe = y <= 16;
            if fringe && x % 2 == 1 {
                continue;
            }
            let c = if fringe || y == 17 {
                GOLD_D
            } else if y % 4 == 0 {
                SASH_D
            } else {
                SASH
            };
            s.set(x, y, fz + 1, c);
        }
    }
    // ---- the kaftan's body, broad at the shoulders, frogged across the chest with gold
    for y in 36..=57 {
        let rx = if y >= 50 { 19.6 - (y - 55).max(0) as f32 * 1.2 } else { 17.2 };
        slab(&mut s, cx, cz, rx, 9.4, 2.6, y, y, KAFTAN);
    }
    // The damask: a lattice of gold thread over the green, a gold flower in every lozenge.
    s.paint(|x, y, z, c| {
        if c != KAFTAN {
            return None;
        }
        let u = if z <= 11 || z >= 27 { x } else { z + 80 };
        let (a, b) = ((u + y).rem_euclid(12), (u - y).rem_euclid(12));
        if a == 0 || b == 0 {
            Some(THREAD)
        } else if (a == 6 && (5..=7).contains(&b)) || (b == 6 && (5..=7).contains(&a)) {
            Some(GOLD)
        } else if nz(x, y, z) < 0.1 {
            Some(KAFTAN_D)
        } else {
            None
        }
    });
    for y in 36..=53 {
        for x in 46..=49 {
            face(&mut s, x, y, d, 0, GOLD_D);
        }
    }
    for fy in [38, 42, 46, 50] {
        for x in 38..=57 {
            face(&mut s, x, fy, d, 1, GOLD);
        }
        for x in [37, 58] {
            for y in fy - 1..=fy + 1 {
                face(&mut s, x, y, d, 1, GOLD_D);
            }
        }
        for (x, y) in [(47, fy), (48, fy), (47, fy + 1), (48, fy + 1)] {
            face(&mut s, x, y, d, 1, GOLD_D);
        }
    }
    // ---- collar and neck
    slab(&mut s, cx, 19.0, 9.8, 7.4, 2.6, 56, 59, KAFTAN_D);
    s.paint(|_x, y, _z, c| (c == KAFTAN_D && y == 59).then_some(GOLD_D));

    // ---- arms: wide sleeves worked like the kaftan, gold cuffs, fists
    for (i, ax) in [21.5f32, 73.5].into_iter().enumerate() {
        s.part(SHIELD_ARM + i as u8);
        s.capsule((ax, 54.0, 19.0), (ax, 34.0, 19.5), 4.6, 4.4, KAFTAN);
        column(&mut s, ax, 19.5, 5.4, 5.6, 28, 33, GOLD_D);
        column(&mut s, ax, 19.5, 5.5, 5.7, 30, 31, GOLD);
        fist(&mut s, v(ax, 24.5, 20.5), v(4.1, 4.2, 4.6), SKIN, SKIN_D);
    }
    s.paint(|x, y, z, c| {
        if c != KAFTAN || !(x <= 27 || x >= 68) {
            return None;
        }
        let (a, b) = ((z + y).rem_euclid(12), (z - y).rem_euclid(12));
        (a == 0 || b == 0).then_some(THREAD)
    });

    // ---- the head
    s.part(HEAD);
    column(&mut s, cx, 18.5, 4.8, 4.2, 56, 61, SKIN);
    rounded(&mut s, v(cx, 70.0, 18.0), v(9.9, 10.2, 7.9), 3.0, 60, SKIN);
    s.paint(|x, y, z, c| {
        let dx = (x as f32 - cx).abs();
        (c == SKIN && ((z <= 13 && y >= 62) || (dx >= 8.6 && y >= 70))).then_some(HAIR)
    });
    for ex in [37.6f32, 57.4] {
        ball(&mut s, v(ex, 67.5, 18.0), v(1.3, 2.6, 2.0), SKIN_D);
    }
    // Dark eyes under heavy, knitted brows; a strong, hooked nose.
    eye(&mut s, 40, 68, 1, d, IRIS, SKIN_D);
    eye(&mut s, 55, 68, -1, d, IRIS, SKIN_D);
    for x in (39..=45).chain(50..=56) {
        face(&mut s, x, 72, d, 1, BROW);
    }
    for x in (40..=44).chain(51..=55) {
        face(&mut s, x, 73, d, 1, BROW);
    }
    for x in [45, 50] {
        face(&mut s, x, 71, d, 1, BROW);
    }
    s.capsule((cx, 71.5, 26.0), (cx, 65.4, 27.8), 1.3, 1.4, SKIN);
    ball(&mut s, v(cx, 68.6, 26.8), v(1.4, 1.5, 1.5), SKIN);
    ball(&mut s, v(cx, 64.8, 27.4), v(2.0, 1.7, 1.6), SKIN);
    for x in [46, 49] {
        face(&mut s, x, 63, d, 0, SKIN_D);
    }
    // ---- a black beard trimmed square, a heavy moustache curling up at its ends
    let (bc, br) = (v(cx, 57.0, 24.6), v(10.8, 8.2, 5.2));
    for y in 49..=66 {
        for z in 14..d {
            for x in 34..62 {
                let q = (Vec3::new(x as f32, y as f32, z as f32) - bc) / br;
                if q.x.abs().powf(2.4) + q.y.abs().powf(2.4) + q.z.abs().powf(2.0) > 1.0 {
                    continue;
                }
                let side = (x as f32 - cx).abs() >= 7.2;
                if y > 62 && !side && (y > 63 || s.get(x, y, z) == Some(SKIN)) {
                    continue;
                }
                s.set(x, y, z, BEARD);
            }
        }
    }
    for x in 45..=50 {
        face(&mut s, x, 62, d, 0, LIP);
    }
    for side in [-1.0f32, 1.0] {
        s.capsule((cx + side * 0.5, 64.2, 28.2), (cx + side * 5.5, 63.4, 27.8), 1.8, 1.6, BEARD_L);
        s.capsule((cx + side * 5.5, 63.4, 27.8), (cx + side * 9.0, 64.8, 26.4), 1.6, 1.0, BEARD_L);
        s.capsule((cx + side * 9.0, 64.8, 26.4), (cx + side * 10.2, 66.8, 25.6), 1.0, 0.6, BEARD_L);
    }
    strands(&mut s, &[BEARD, BEARD_L], BEARD_D, BEARD, BEARD_L);

    // ---- the börk: white felt on a gold band, tapering; its flap folded down his back
    for y in 76..=109 {
        let taper = if y < 82 { 0.0 } else { (y - 82) as f32 * 0.07 };
        let dome = if y > 105 { (y - 105) as f32 * 0.9 } else { 0.0 };
        slab(&mut s, cx, 17.0, 10.6 - taper - dome, 7.8 - taper * 0.6 - dome, 3.0, y, y, if y <= 79 { GOLD } else { FELT });
    }
    s.paint(|x, y, z, c| {
        if c == GOLD && y <= 79 {
            return Some(if y == 76 || y == 79 { GOLD_D } else { GOLD });
        }
        (c == FELT && y > 82 && (x + z) % 8 == 0).then_some(FELT_D)
    });
    for y in 60..=109 {
        let half = if y >= 104 { 8.8 - (y - 104) as f32 * 0.6 } else { 8.8 };
        for x in (cx - half).round() as i32..=(cx + half).round() as i32 {
            let c = if y <= 61 {
                GOLD_D
            } else if x % 6 == 0 {
                FELT_D
            } else {
                FELT
            };
            s.set(x, y, 8, c);
            s.set(x, y, 9, FELT);
            if y >= 104 {
                s.set(x, y, 10, FELT);
                s.set(x, y, 11, FELT);
            }
        }
    }
    // The spoon-holder plate over the brow, set with a green stone, a plume socket above.
    for y in 80..=97 {
        let half = if y >= 96 { 3.6 } else { 5.2 };
        for x in (cx - half).round() as i32..=(cx + half).round() as i32 {
            let ring = y == 82 || y == 83 || y == 92 || y == 93;
            let Some(fz) = front(&s, x, y, d) else { continue };
            s.set(x, y, fz + 1, if ring { GOLD_D } else { GOLD });
        }
    }
    for y in 86..=89 {
        for x in 46..=49 {
            let Some(fz) = front(&s, x, y, d) else { continue };
            s.set(x, y, fz + 1, if y == 89 && x == 46 { GEM_L } else { GEM });
        }
    }
    for y in 98..=103 {
        for x in 46..=49 {
            for z in 25..=27 {
                s.set(x, y, z, GOLD_D);
            }
        }
    }

    s.part(0);
    if let Some((form, color, tier)) = shield {
        super::armory::back_shield_at(&mut s, form, color, tier, MEHMED_SHIELD_AT);
    }
    s.finish_solid(0.03)
}

/// Mehmed's joints.
pub const MEHMED_HD_RIG: &[PartDef] = &rig([39.5, 55.5], 16.0, 21.5, [21.5, 73.5], 54.0, 18.5, [47.5, 58.0, 18.5]);

// ---------------------------------------------------------------- Miyamoto

mod sam {
    use super::mat;
    pub const SKIN: u32 = 0xe2b48c | mat::SKIN;
    pub const SKIN_D: u32 = 0xa8785a | mat::SKIN;
    pub const EYE_W: u32 = 0xeeeae0;
    pub const PUPIL: u32 = 0x0e0a08;
    pub const LACQUER: u32 = 0xb8222a | mat::LEATHER;
    pub const LACQUER_D: u32 = 0x8a1820 | mat::LEATHER;
    pub const BLACK: u32 = 0x1c1a1e | mat::LEATHER;
    pub const IRON: u32 = 0x2c2a30 | mat::METAL;
    pub const GOLD: u32 = 0xf2c53d | mat::GOLD;
    pub const GOLD_D: u32 = 0xb88a2a | mat::GOLD;
    pub const LACING: u32 = 0x24367a | mat::CLOTH;
    pub const LACING_L: u32 = 0x3a52a8 | mat::CLOTH;
    pub const OBI: u32 = 0x2a4aa8 | mat::CLOTH;
    pub const OBI_D: u32 = 0x1e3a86 | mat::CLOTH;
    pub const HAKAMA: u32 = 0x2e2c36 | mat::CLOTH;
    pub const HAKAMA_D: u32 = 0x201e28 | mat::CLOTH;
    pub const KOTE: u32 = 0x26242e | mat::CLOTH;
    pub const MAIL: u32 = 0x4a4852 | mat::METAL;
    pub const TABI: u32 = 0xe8e2d4 | mat::CLOTH;
    pub const STRAW: u32 = 0xc8a868 | mat::WOOD;
    pub const STRAW_D: u32 = 0xa08450 | mat::WOOD;
    pub const MASK: u32 = 0xc0262c | mat::LEATHER;
    pub const MASK_D: u32 = 0x8e1a20 | mat::LEATHER;
    pub const TEETH: u32 = 0xf0ecdc | mat::IVORY;
    pub const WHISKER: u32 = 0x141010 | mat::FUR;
    pub const WRAP: u32 = 0x18161a | mat::CLOTH;
    pub const SAME: u32 = 0xeae4d4 | mat::IVORY;
}

/// Miyamoto's grid: 96 wide (his body in the middle), 120 tall (his crest), 44 deep.
pub const MIYAMOTO_SIZE: (i32, i32, i32) = (96, 120, 44);
/// Where Miyamoto's shield sits, as an offset from where Ragnar's does.
pub const MIYAMOTO_SHIELD_AT: (i32, i32, i32) = (18, 0, 8);

/// Lacquered lames laced with silk: `c` voxels of a lame are its lacquer, its lowest
/// row a gilt edge, and pairs of blue cords run down through it every `every` columns.
fn lame(y: i32, top: i32, u: i32, every: i32) -> u32 {
    use sam::*;
    let row = (top - y).rem_euclid(6);
    if row == 5 {
        GOLD_D
    } else if u.rem_euclid(every) < 2 && row != 0 {
        if row % 2 == 0 {
            LACING_L
        } else {
            LACING
        }
    } else {
        LACQUER
    }
}

/**
 * Miyamoto the samurai: a black iron kabuto riveted in rows, a gilt visor,
 * a neck guard of lacquered lames flaring out, gilt turnbacks and great gilt
 * horns rising from a disc on the brow; a red oni mask snarling white teeth,
 * black whiskers, fierce eyes in its shadow; a cuirass of red lames edged in
 * gold and laced in blue silk under a black breastplate with a gilt crest;
 * shoulder guards of lames; an armoured skirt in panels; a blue obi with a
 * short sword thrust through it; dark silk sleeves sewn with gilt plates over
 * mail, black gauntlets; pleated hakama, black shin guards splinted with gold,
 * white tabi and straw sandals. His weapon comes from his armory; a shield of
 * his tree (form, colour, tier) is slung on his back, or none.
 */
pub fn miyamoto_hd(shield: Option<(Form, u32, u8)>) -> Collected {
    use sam::*;
    let (w, h, d) = MIYAMOTO_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (47.5f32, 21.5f32);

    // ---- straw sandals over split-toed tabi, black shin guards splinted with gold
    for (i, lx) in [39.5f32, 55.5].into_iter().enumerate() {
        s.part(LEFT_LEG + i as u8);
        column(&mut s, lx, 25.5, 6.0, 9.6, 0, 1, STRAW);
        s.ellipsoid((lx, 2.0, 25.0), (5.6, 3.6, 9.0), 2, TABI);
        column(&mut s, lx, 24.0, 5.4, 6.4, 4, 19, HAKAMA);
        for y in 6..=19 {
            for x in (lx - 5.0).round() as i32..=(lx + 5.0).round() as i32 {
                let Some(fz) = front(&s, x, y, d) else { continue };
                let c = if y >= 18 {
                    GOLD
                } else if (x - lx.round() as i32).rem_euclid(4) == 0 {
                    GOLD_D
                } else {
                    BLACK
                };
                s.set(x, y, fz + 1, c);
            }
        }
        let t = lx.round() as i32 - 2;
        for y in 2..=5 {
            for z in 30..=34 {
                s.set(t, y, z, BLACK);
            }
        }
    }
    s.paint(|x, _y, z, c| (c == STRAW && (x + z) % 3 == 0).then_some(STRAW_D));

    // ---- hakama: wide pleated trousers
    s.part(0);
    for y in 16..=29 {
        let t = (y - 16) as f32 / 13.0;
        slab(&mut s, cx, cz, 14.8 + t * 2.4, 10.5, 2.6, y, y, HAKAMA);
    }
    s.paint(|x, y, _z, c| (c == HAKAMA && y >= 16 && x % 6 < 2).then_some(HAKAMA_D));

    // ---- the kusazuri: an armoured skirt of lames in panels, flaring from the waist
    for y in 20..=35 {
        let t = (35 - y) as f32 / 15.0;
        let (rx, rz) = (18.0 + t * 4.0, 11.5 + t * 3.2);
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = ((x as f32 - cx) / rx, (z as f32 - cz) / rz);
                if dx.abs().powf(2.6) + dz.abs().powf(2.6) > 1.0 {
                    continue;
                }
                // Gaps between the panels.
                if ((38..=39).contains(&x) || (56..=57).contains(&x)) && dz > 0.3 {
                    continue;
                }
                s.set(x, y, z, lame(y, 35, x + z, 7));
            }
        }
    }
    // ---- the obi, and a short sword thrust through it at the left hip
    slab(&mut s, cx, cz, 18.4, 11.2, 2.6, 34, 37, OBI);
    s.paint(|_x, y, _z, c| (c == OBI && y == 35).then_some(OBI_D));
    s.capsule((33.5, 36.0, 33.0), (24.5, 22.0, 7.0), 1.6, 1.4, BLACK);
    ball(&mut s, v(33.8, 36.4, 33.6), v(2.8, 2.8, 0.9), GOLD_D);
    for k in 0..8 {
        let t = k as f32 / 7.0;
        let p = v(34.2, 36.8, 34.4).lerp(v(36.4, 40.4, 40.0), t);
        ball(
            &mut s,
            p,
            Vec3::splat(1.3),
            if k == 7 {
                GOLD
            } else if k % 2 == 0 {
                SAME
            } else {
                WRAP
            },
        );
    }

    // ---- the dō: a cuirass of red lames edged in gold under a black breastplate, a gilt crest
    for y in 38..=59 {
        let rz = 9.6 + ((y - 38) as f32 / 21.0 * PI).sin() * 0.6;
        let c = if y >= 56 {
            if y == 59 {
                GOLD
            } else {
                BLACK
            }
        } else {
            LACQUER
        };
        slab(&mut s, cx, cz, 17.2, rz, 2.8, y, y, c);
    }
    s.paint(|x, y, z, c| (c == LACQUER && (38..=55).contains(&y)).then(|| lame(y, 55, x + z, 6)));
    let mz = front(&s, 47, 47, d).unwrap_or(32) as f32 + 0.6;
    ball(&mut s, v(cx, 47.0, mz), v(3.3, 3.3, 1.2), GOLD);
    for (x, y) in [(47, 49), (48, 49), (47, 45), (48, 45), (45, 47), (50, 47), (47, 47), (48, 47)] {
        face(&mut s, x, y, d, 0, GOLD_D);
    }

    // ---- arms: sleeves of dark silk over mail, sewn with gilt plates, black gauntlets;
    //      great shoulder guards of lames hang over them and swing with them
    for (i, ax) in [21.5f32, 73.5].into_iter().enumerate() {
        s.part(SHIELD_ARM + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        s.capsule((ax, 52.0, 19.5), (ax, 29.0, 20.5), 4.2, 4.0, KOTE);
        column(&mut s, ax, 20.5, 4.4, 5.0, 22, 28, BLACK);
        column(&mut s, ax, 20.5, 4.5, 5.1, 28, 28, GOLD_D);
        fist(&mut s, v(ax, 24.5, 21.5), v(4.1, 4.2, 4.6), BLACK, IRON);
        for y in 40..=59 {
            let flare = (59 - y) as f32 * 0.22;
            let x0 = ax - 4.6 + out * flare;
            for x in x0.round() as i32..=(x0 + 9.2).round() as i32 {
                for z in 14..=29 {
                    let c = if y >= 58 { BLACK } else { lame(y, 57, z, 5) };
                    s.set(x, y, z, c);
                }
            }
        }
    }
    s.paint(|x, y, z, c| {
        if c != KOTE {
            return None;
        }
        if (x + y * 2 + z) % 10 < 2 && y % 3 == 0 {
            Some(GOLD_D)
        } else if (x + y + z) % 2 == 0 {
            Some(MAIL)
        } else {
            None
        }
    });

    // ---- the throat guard, the head, and the red oni mask with its snarl
    s.part(HEAD);
    slab(&mut s, cx, 21.0, 9.8, 6.8, 2.6, 54, 59, BLACK);
    for y in 54..=59 {
        for x in 38..=57 {
            let Some(fz) = front(&s, x, y, d) else { continue };
            s.set(x, y, fz + 1, if y == 54 { GOLD_D } else { LACQUER_D });
        }
    }
    rounded(&mut s, v(cx, 70.0, 20.0), v(9.9, 10.2, 7.9), 3.0, 60, SKIN);
    rounded(&mut s, v(cx, 65.0, 23.4), v(10.0, 5.2, 6.0), 3.0, 60, MASK);
    s.capsule((cx, 71.0, 29.6), (cx, 66.4, 31.4), 1.6, 1.8, MASK);
    ball(&mut s, v(cx, 66.2, 31.2), v(2.4, 1.8, 1.8), MASK);
    for x in [46, 49] {
        face(&mut s, x, 65, d, 0, MASK_D);
    }
    // The snarl: a mouth of white teeth, longer fangs at its corners.
    for y in 61..=63 {
        for x in 41..=54 {
            let gap = x % 3 == 0 || (y == 62 && x % 3 != 0 && (x == 41 || x == 54));
            face(&mut s, x, y, d, 0, if gap { PUPIL } else { TEETH });
        }
    }
    for x in [41, 54] {
        face(&mut s, x, 60, d, 0, TEETH);
        face(&mut s, x, 64, d, 0, TEETH);
    }
    // Black whiskers: a moustache over the snarl, a tuft at the chin.
    for (x, y) in [(40, 65), (41, 65), (42, 66), (43, 66), (52, 66), (53, 66), (54, 65), (55, 65), (38, 66), (39, 67), (56, 67), (57, 66)] {
        face(&mut s, x, y, d, 1, WHISKER);
    }
    for y in 57..=60 {
        for x in 46..=49 {
            face(&mut s, x, y, d, 1, WHISKER);
        }
    }
    // Fierce eyes in the shadow of the visor, the mask's brows scowling over them.
    for y in 70..=73 {
        for x in 38..=57 {
            if let Some(z) = front(&s, x, y, d) {
                if s.get(x, y, z) == Some(SKIN) {
                    s.recolor(x, y, z, SKIN_D);
                }
            }
        }
    }
    for (x0, dir) in [(40, 1), (55, -1)] {
        for k in 0..4 {
            let x = x0 + k * dir;
            let c = if k < 2 { EYE_W } else { PUPIL };
            face(&mut s, x, 70, d, 0, c);
            face(&mut s, x, 71, d, 0, c);
        }
        for k in 0..6 {
            let x = x0 - dir + k * dir;
            face(&mut s, x, 72 + (k / 3), d, 1, MASK);
        }
    }

    // ---- the kabuto: a black iron bowl riveted in rows, a gilt visor, a knob on top
    s.ellipsoid((cx, 75.0, 20.0), (12.8, 12.0, 11.2), 74, IRON);
    s.paint(|x, y, z, c| {
        if c != IRON || y < 76 {
            return None;
        }
        let seg = turn(x, z, cx, 20.0) * 48.0;
        (seg.fract() < 0.18 && y % 3 == 0).then_some(GOLD_D)
    });
    ball(&mut s, v(cx, 86.6, 20.0), v(2.2, 1.2, 2.2), GOLD);
    for z in 24..=33 {
        for x in 36..=59 {
            let c = if z >= 32 { GOLD } else { BLACK };
            s.set(x, 74, z, c);
            s.set(x, 75, z, c);
        }
    }
    // The neck guard: four lames stepping down and out round the back and sides.
    for i in 0..4 {
        let top = 73 - i * 4;
        let (rx, rz) = (13.4 + 2.2 * i as f32, 11.8 + 2.2 * i as f32);
        for y in top - 3..=top {
            for z in 0..d {
                for x in 0..w {
                    let (dx, dz) = ((x as f32 - cx) / rx, (z as f32 - 20.0) / rz);
                    let r = (dx * dx + dz * dz).sqrt();
                    if !(0.9..=1.0).contains(&r) || z as f32 > 20.0 + 3.5 {
                        continue;
                    }
                    let seg = (turn(x, z, cx, 20.0) * 90.0) as i32;
                    let c = if y == top - 3 {
                        GOLD_D
                    } else if seg % 6 < 2 {
                        LACING
                    } else {
                        LACQUER
                    };
                    s.set(x, y, z, c);
                }
            }
        }
    }
    // Gilt turnbacks folded out beside the face.
    for (x0, x1) in [(32, 37), (58, 63)] {
        for y in 68..=75 {
            for z in 24..=29 {
                let c = if y == 75 {
                    GOLD
                } else if z >= 28 {
                    GOLD_D
                } else {
                    LACQUER
                };
                for x in x0..=x1 {
                    s.set(x, y, z, c);
                }
            }
        }
    }
    // ---- the crest: a gilt disc on the brow, and great gilt horns rising from it in a wide V
    s.ellipsoid((cx, 80.0, 33.0), (4.0, 4.0, 1.6), 0, GOLD);
    s.ellipsoid((cx, 80.0, 34.0), (2.0, 2.0, 1.0), 0, GOLD_D);
    for side in [-1.0f32, 1.0] {
        let steps = 56;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = cx + side * (3.6 + 14.0 * t + 3.0 * t * t);
            let y = 81.0 + 32.0 * t - 4.0 * t * t * t;
            let r = 2.2 - 1.1 * t;
            s.ellipsoid((x, y, 33.0), (r, r, r * 0.55), 0, if t > 0.88 { GOLD_D } else { GOLD });
        }
    }

    s.part(0);
    if let Some((form, color, tier)) = shield {
        super::armory::back_shield_at(&mut s, form, color, tier, MIYAMOTO_SHIELD_AT);
    }
    s.finish_solid(0.03)
}

/// Miyamoto's joints.
pub const MIYAMOTO_HD_RIG: &[PartDef] = &rig([39.5, 55.5], 20.0, 24.5, [21.5, 73.5], 52.0, 19.5, [47.5, 60.0, 20.5]);

// ------------------------------------------------------------------ outfits and ribbons

/// The outfits each hero can wear, chosen at the old man's: his own first.
pub fn outfit_names(hero: HeroKind) -> [&'static str; 4] {
    match hero {
        HeroKind::Ragnar => ["Berserker", "Raven", "Jarl", "Frost"],
        HeroKind::Mehmed => ["Janissary", "Sultan's Guard", "Midnight", "Desert"],
        HeroKind::Miyamoto => ["Crimson", "Shadow", "Jade", "Ivory"],
    }
}

/// What an outfit changes: each of the hero's own colours (with its material) and the colour it
/// becomes. His own outfit changes nothing.
fn outfit_swaps(hero: HeroKind, outfit: u8) -> Vec<(u32, u32)> {
    let pairs =
        |src: &[u32], dst: [u32; 10]| -> Vec<(u32, u32)> { src.iter().zip(dst).map(|(&s, d)| (s, d | (s & 0xff00_0000))).collect() };
    match (hero, outfit) {
        (HeroKind::Ragnar, 1..=3) => {
            use viking::*;
            let src = [TUNIC, TUNIC_D, TUNIC_L, TRIM, TRIM_R, CLOAK, CLOAK_D, CLOAK_L, TROUSER, TROUSER_D];
            let dst = match outfit {
                // Raven: black wool, a charcoal cloak, silver trim.
                1 => [0x2a2a30, 0x1e1e24, 0x3a3a44, 0xb8bcc4, 0x4a4c56, 0x3a3c44, 0x2a2c32, 0x4a4c56, 0x2e2e34, 0x24242a],
                // Jarl: crimson, a white cloak, gold trim.
                2 => [0x8e1c24, 0x701620, 0xa8262e, 0xf2c53d, 0x2a4a8a, 0xd8d4c8, 0xbab6aa, 0xeae6dc, 0x5a3a24, 0x48301e],
                // Frost: pale ice blue, a deep blue cloak, white trim.
                _ => [0x9ab8d0, 0x7e9cb4, 0xb4d0e4, 0xe8eef4, 0x2a3870, 0x1e2a5a, 0x16204a, 0x2a3870, 0x5e6a78, 0x4e5866],
            };
            pairs(&src, dst)
        }
        (HeroKind::Mehmed, 1..=3) => {
            use jan::*;
            let src = [KAFTAN, KAFTAN_D, THREAD, ROBE, ROBE_D, SASH, SASH_D, FELT, FELT_D, BOOT];
            let dst = match outfit {
                // The Sultan's Guard: a crimson kaftan over white, a gold sash.
                1 => [0x8e1c26, 0x6e141c, 0xf2c53d, 0xe2dccc, 0xc6bead, 0xc89a30, 0xa07a24, 0xf0ece0, 0xd6d0c0, 0x4a3a2a],
                // Midnight: navy and gold, silver thread.
                2 => [0x1a2650, 0x121c3e, 0xc8ccd4, 0xc8a040, 0xa08030, 0x2a3a7a, 0x1e2c60, 0x1e2440, 0x161a30, 0x1a1a22],
                // Desert: sand and teal, a rust sash.
                _ => [0xc8a874, 0xa88a5a, 0x8a5a2a, 0x1f8a8a, 0x166a6a, 0xb05a2a, 0x8a4420, 0xe8dcc0, 0xccbea0, 0x7a5a36],
            };
            pairs(&src, dst)
        }
        (HeroKind::Miyamoto, 1..=3) => {
            use sam::*;
            let src = [LACQUER, LACQUER_D, LACING, LACING_L, OBI, OBI_D, MASK, MASK_D, HAKAMA, HAKAMA_D];
            let dst = match outfit {
                // Shadow: black lacquer laced in red.
                1 => [0x1e1c22, 0x141218, 0x8a1a22, 0xb0262e, 0x9a1e26, 0x7a1620, 0x26242a, 0x18161c, 0x2a1a1e, 0x1e1216],
                // Jade: green lacquer laced in gold.
                2 => [0x1f7a4a, 0x165a36, 0xb8902a, 0xd8b048, 0xb8902a, 0x8a6a20, 0x2a8a52, 0x1f6a3e, 0x2a3a30, 0x1e2a22],
                // Ivory: white lacquer laced in black.
                _ => [0xe8e2d4, 0xc8c2b4, 0x1c1a1e, 0x2e2c32, 0x24222a, 0x18161c, 0xece6d8, 0xccc6b8, 0x3a3844, 0x2c2a34],
            };
            pairs(&src, dst)
        }
        _ => Vec::new(),
    }
}

/// Dresses a hero in `outfit`: every voxel of one of his own colours (shaded by the sculpt's
/// jitter, so matched by its hue and a brightness within a few hundredths) takes the outfit's.
pub fn dress(c: &mut Collected, hero: HeroKind, outfit: u8) {
    let swaps = outfit_swaps(hero, outfit);
    if swaps.is_empty() {
        return;
    }
    let ch = |c: u32| [((c >> 16) & 255) as f32, ((c >> 8) & 255) as f32, (c & 255) as f32];
    for v in c.voxels.iter_mut() {
        let rgb = ch(v.c);
        let mut best: Option<(f32, f32, u32)> = None;
        for &(src, dst) in &swaps {
            if src >> 24 != v.c >> 24 {
                continue;
            }
            let s = ch(src);
            let k = (rgb[0] + rgb[1] + rgb[2]) / (s[0] + s[1] + s[2]).max(1.0);
            if !(0.94..=1.06).contains(&k) {
                continue;
            }
            let err = (0..3).map(|i| (rgb[i] - s[i] * k).abs()).fold(0.0, f32::max);
            if err <= 3.5 && best.is_none_or(|b| err < b.0) {
                best = Some((err, k, dst));
            }
        }
        if let Some((_, k, dst)) = best {
            v.c = scale(dst, k);
        }
    }
}

/// Ribbons on the shield arm for the skills of his path he has learned: one band round the upper
/// arm for each, in that skill's colour, tied on its outer side with a tail hanging from the first.
/// `shoulder_y` is the arm's joint; the bands run down from just below it.
pub fn ribbons(c: &mut Collected, shoulder_y: f32, colors: &[u32]) {
    use std::collections::HashSet;
    if colors.is_empty() {
        return;
    }
    let taken: HashSet<(i32, i32, i32)> = c.voxels.iter().map(|v| (v.x, v.y, v.z)).collect();
    let arm: Vec<(i32, i32, i32)> = c.voxels.iter().filter(|v| v.part == SHIELD_ARM).map(|v| (v.x, v.y, v.z)).collect();
    let Some(low) = arm.iter().map(|p| p.1).min() else { return };
    let top = shoulder_y as i32 - 4;
    let mut added = Vec::new();
    let mut outer: Option<(i32, i32, i32)> = None;
    for (k, &color) in colors.iter().enumerate() {
        let y = top - 2 * k as i32;
        if y < low + 8 {
            break; // no further than the forearm
        }
        let ring: HashSet<(i32, i32)> = arm.iter().filter(|p| p.1 == y).map(|p| (p.0, p.2)).collect();
        for &(x, z) in &ring {
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let q = (x + dx, y, z + dz);
                if !ring.contains(&(q.0, q.2)) && !taken.contains(&q) && !added.iter().any(|v: &Voxel| (v.x, v.y, v.z) == q) {
                    added.push(Voxel { x: q.0, y: q.1, z: q.2, c: color | mat::CLOTH, part: SHIELD_ARM });
                    if k == 0 && outer.is_none_or(|o| q.0 < o.0) {
                        outer = Some(q);
                    }
                }
            }
        }
    }
    // The tail: the first ribbon's ends hanging down its outer side.
    if let Some((x, y, z)) = outer {
        for d in 1..=4 {
            let q = (x - 1, y - d, z + (d % 2));
            if !taken.contains(&q) {
                added.push(Voxel { x: q.0, y: q.1, z: q.2, c: colors[0] | mat::CLOTH, part: SHIELD_ARM });
            }
        }
    }
    c.voxels.extend(added);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::voxels::build_rig;

    #[test]
    fn every_hero_is_sculpted_into_his_parts() {
        for (name, model, defs, (w, h, d)) in [
            ("ragnar", ragnar_hd(Some((Form::Round, 0xb8864a, 1))), RAGNAR_HD_RIG, RAGNAR_SIZE),
            ("ragnar, kite", ragnar_hd(Some((Form::Kite, 0x8fa8c8, 8))), RAGNAR_HD_RIG, RAGNAR_SIZE),
            ("ragnar, bare", ragnar_hd(None), RAGNAR_HD_RIG, RAGNAR_SIZE),
            ("mehmed", mehmed_hd(Some((Form::Spiral, 0xc8a060, 1))), MEHMED_HD_RIG, MEHMED_SIZE),
            ("miyamoto", miyamoto_hd(Some((Form::Tate, 0xc8b870, 1))), MIYAMOTO_HD_RIG, MIYAMOTO_SIZE),
        ] {
            assert_eq!((model.width, model.height, model.depth), (w, h, d), "{name}");
            let rig = build_rig(&model, defs, 0.0125);
            let total: usize = rig.parts.iter().map(|p| p.cubes.len()).sum();
            assert_eq!(total, model.voxels.len(), "{name}: every voxel in one part");
            for p in &rig.parts {
                assert!(p.cubes.len() > 100, "{name}: part {} has {} cubes", p.name, p.cubes.len());
            }
            eprintln!("{name}: {} voxels", model.voxels.len());
        }
    }

    #[test]
    fn detailed_ragnar_carries_materials() {
        let model = ragnar_hd(Some((Form::Round, 0xb8864a, 1)));
        let rig = build_rig(&model, RAGNAR_HD_RIG, 0.0125);
        let shades: std::collections::HashSet<i32> = rig.parts.iter().flat_map(|p| p.shade.iter().map(|&s| s as i32)).collect();
        for m in 6..=13 {
            if m != 12 {
                assert!(shades.contains(&m), "no voxels of material {m}");
            }
        }
    }

    #[test]
    fn an_outfit_recolours_the_clothes_and_ribbons_band_the_shield_arm() {
        for hero in [HeroKind::Ragnar, HeroKind::Mehmed, HeroKind::Miyamoto] {
            let body = || match hero {
                HeroKind::Ragnar => ragnar_hd(None),
                HeroKind::Mehmed => mehmed_hd(None),
                HeroKind::Miyamoto => miyamoto_hd(None),
            };
            let own = body();
            let mut dressed = body();
            dress(&mut dressed, hero, 0);
            assert!(own.voxels.iter().zip(&dressed.voxels).all(|(a, b)| a.c == b.c), "his own outfit changes nothing");
            for outfit in 1..4 {
                let mut d = body();
                dress(&mut d, hero, outfit);
                let changed = own.voxels.iter().zip(&d.voxels).filter(|(a, b)| a.c != b.c).count();
                assert!(changed * 10 > own.voxels.len(), "{hero:?} outfit {outfit}: {changed} of {} voxels change", own.voxels.len());
                assert!(
                    own.voxels.iter().zip(&d.voxels).filter(|(a, _)| a.c >> 24 == mat::SKIN >> 24).all(|(a, b)| a.c == b.c),
                    "skin stays"
                );
            }
            let defs = match hero {
                HeroKind::Ragnar => RAGNAR_HD_RIG,
                HeroKind::Mehmed => MEHMED_HD_RIG,
                HeroKind::Miyamoto => MIYAMOTO_HD_RIG,
            };
            let shoulder = defs.iter().find(|d| d.name == "shieldArm").unwrap().pivot[1];
            let mut banded = body();
            ribbons(&mut banded, shoulder, &[0xff0000, 0x00ff00, 0x0000ff]);
            let added = banded.voxels.len() - own.voxels.len();
            assert!(added >= 12, "{hero:?}: {added} ribbon voxels");
            assert!(banded.voxels[own.voxels.len()..].iter().all(|v| v.part == SHIELD_ARM), "they move with the arm");
        }
    }
}
