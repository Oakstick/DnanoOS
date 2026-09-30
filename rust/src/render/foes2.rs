//! The second stage's cast, the foes of the seven realms, sculpted as finely
//! as the first (see `foes`): the witch in her purple robe and bent hat with
//! her gnarled staff; her minion, a knee-high witchling; the helmeted
//! skeleton, red fire in its sockets under a dented iron cap; the loup-garou,
//! a shaggy wolf the size of a pony with red eyes and a mouth full of teeth;
//! the blemmyes, a headless brute with his face in his chest; and the large
//! bat, wings of torn membrane on long bone fingers. Each faces +z and is
//! sculpted in the parts its rig swings.

use std::f32::consts::{PI, TAU};

use glam::Vec3;

use super::carve::*;
use super::characters::{mat, Sculpt};
use super::voxels::{Collected, PartDef};

mod pal {
    use super::mat;
    pub const BONE: u32 = 0xe2d6bc | mat::IVORY;
    pub const BONE_S: u32 = 0xc2b292 | mat::IVORY;
    pub const BONE_D: u32 = 0x8e7e62 | mat::IVORY;
    pub const RED_FIRE: u32 = 0xff3a2a;
    pub const RED_CORE: u32 = 0xffb0a0;
    pub const CAVITY: u32 = 0x17120f;
    pub const HELM: u32 = 0x8a919b | mat::METAL;
    pub const HELM_L: u32 = 0xb2b9c3 | mat::METAL;
    pub const HELM_D: u32 = 0x5a6069 | mat::METAL;
    pub const IRON: u32 = 0x4a4846 | mat::METAL;
    pub const IRON_D: u32 = 0x2e2c2a | mat::METAL;
    pub const ROBE: u32 = 0x6a2aa8 | mat::CLOTH;
    pub const ROBE_D: u32 = 0x47186f | mat::CLOTH;
    pub const ROBE_L: u32 = 0x8a48c8 | mat::CLOTH;
    pub const HAT: u32 = 0x2c1a44 | mat::CLOTH;
    pub const HAT_L: u32 = 0x3f2860 | mat::CLOTH;
    pub const BAND: u32 = 0x9a2a60 | mat::CLOTH;
    pub const BUCKLE: u32 = 0xf2c53d | mat::GOLD;
    pub const BELT: u32 = 0x4a3020 | mat::LEATHER;
    pub const BOOT: u32 = 0x2a2026 | mat::LEATHER;
    pub const WITCH_SKIN: u32 = 0x93c56c | mat::SKIN;
    pub const WITCH_SKIN_D: u32 = 0x6f9a4e | mat::SKIN;
    pub const WART: u32 = 0x5a7a3a | mat::SKIN;
    pub const HAIR: u32 = 0x1a1420 | mat::FUR;
    pub const HAIR_L: u32 = 0x3a2e48 | mat::FUR;
    pub const STAFF: u32 = 0x6a4426 | mat::WOOD;
    pub const STAFF_D: u32 = 0x4a2e18 | mat::WOOD;
    pub const GEM: u32 = 0x8fff8a;
    pub const GEM_C: u32 = 0xe8ffe0;
    pub const MINION_SKIN: u32 = 0xdcc8ac | mat::SKIN;
    pub const MINION_ROBE: u32 = 0x5a2494 | mat::CLOTH;
    pub const MINION_ROBE_D: u32 = 0x3d1866 | mat::CLOTH;
    pub const MINION_HAIR: u32 = 0x3a2418 | mat::FUR;
    pub const FUR: u32 = 0x5a4232 | mat::FUR;
    pub const FUR_D: u32 = 0x33241a | mat::FUR;
    pub const FUR_L: u32 = 0x7a5e48 | mat::FUR;
    pub const FUR_TIP: u32 = 0x8c7058 | mat::FUR;
    pub const BELLY: u32 = 0x9c7e60 | mat::FUR;
    pub const NOSE: u32 = 0x121010;
    pub const MOUTH: u32 = 0x5a1414;
    pub const TOOTH: u32 = 0xeee4c8 | mat::IVORY;
    pub const CLAW: u32 = 0xd8ccb0 | mat::IVORY;
    pub const EAR_IN: u32 = 0x8a4a58 | mat::SKIN;
    pub const SKIN: u32 = 0xe8b494 | mat::SKIN;
    pub const SKIN_D: u32 = 0xc48c6c | mat::SKIN;
    pub const SKIN_L: u32 = 0xf4cbb0 | mat::SKIN;
    pub const LIP: u32 = 0xb0685a | mat::SKIN;
    pub const CLOTH: u32 = 0x6a4a2e | mat::CLOTH;
    pub const CLOTH_D: u32 = 0x4a3220 | mat::CLOTH;
    pub const ROPE: u32 = 0xa88a5a | mat::LEATHER;
    pub const CLUB: u32 = 0x5a3a20 | mat::WOOD;
    pub const CLUB_D: u32 = 0x3c2614 | mat::WOOD;
    pub const BAT_FUR: u32 = 0x3a2430 | mat::FUR;
    pub const BAT_FUR_L: u32 = 0x5a3a4a | mat::FUR;
    pub const BAT_BONE: u32 = 0x2a1a22 | mat::LEATHER;
    pub const MEMBRANE: u32 = 0x5e2a3c | mat::LEATHER;
    pub const MEMBRANE_D: u32 = 0x3e1a28 | mat::LEATHER;
    pub const VEIN: u32 = 0x2a1018 | mat::LEATHER;
}
use pal::*;

/// Is (x, y) inside the polygon?
fn inside(poly: &[(f32, f32)], x: f32, y: f32) -> bool {
    let mut hit = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.1 > y) != (b.1 > y) && x < a.0 + (b.0 - a.0) * (y - a.1) / (b.1 - a.1) {
            hit = !hit;
        }
    }
    hit
}

/// A thin ring round an upright axis at (cx, cz) at height `y`.
fn ring(s: &mut Sculpt, cx: f32, cz: f32, rx: f32, rz: f32, y: i32, c: u32) {
    let n = ((rx + rz) * 4.0) as i32;
    for k in 0..n {
        let a = k as f32 / n as f32 * TAU;
        s.set((cx + a.cos() * rx).round() as i32, y, (cz + a.sin() * rz).round() as i32, c);
    }
}

/// An eye socket sunk into the front of a skull at (x, y), fire burning in it.
fn fire_socket(s: &mut Sculpt, x: i32, y: i32, d: i32, fire: u32) {
    for dy in -1..=1 {
        for dx in -1..=1 {
            if let Some(z) = front(s, x + dx, y + dy, d) {
                s.clear(x + dx, y + dy, z);
                s.recolor(x + dx, y + dy, z - 1, CAVITY);
            }
        }
    }
    if let Some(z) = front(s, x, y, d) {
        s.set(x, y, z, fire);
        s.set(x, y, z + 1, RED_CORE);
    }
}

// ---------------------------------------------------------------------- witch

/// The witch's grid: 40 wide, 96 tall with her hat, 26 deep.
pub const WITCH_SIZE: (i32, i32, i32) = (40, 96, 26);

/// Hips, shoulders and neck; the hat goes with the head, the staff with the right hand.
pub const WITCH_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [15.5, 22.0, 13.0] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [23.5, 22.0, 13.0] },
    PartDef { name: "leftArm", boxes: &[], pivot: [12.5, 56.0, 13.0] },
    PartDef { name: "rightArm", boxes: &[], pivot: [26.5, 56.0, 13.0] },
    PartDef { name: "head", boxes: &[], pivot: [19.5, 61.0, 13.0] },
];

/**
 * The witch: a long purple robe belted at the waist, black boots under its
 * hem, green skin, a hooked nose with a wart, yellow eyes under a wide-brimmed
 * hat whose point bends over, long black hair, and a gnarled staff with a
 * green stone glowing at its head.
 */
pub fn witch() -> Collected {
    let (w, h, d) = WITCH_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (19.5f32, 13.0f32);

    // ---- legs: boots, under the hem
    for (i, lx) in [15.5f32, 23.5].into_iter().enumerate() {
        s.part(1 + i as u8);
        s.capsule((lx, 22.0, 13.0), (lx, 6.0, 13.0), 2.4, 2.0, ROBE_D);
        s.ellipsoid((lx, 2.4, 14.5), (3.0, 2.6, 5.4), 0, BOOT);
        s.capsule((lx, 5.0, 13.0), (lx, 2.0, 13.0), 2.4, 2.4, BOOT);
    }

    // ---- the robe: flared to the hem, gathered at the waist, folds down its length
    s.part(0);
    for y in 4..=58 {
        let (rx, rz) = if y <= 42 {
            let k = (42 - y) as f32 / 38.0;
            (6.0 + k * 6.5, 4.2 + k * 3.8)
        } else {
            let k = (y - 42) as f32 / 16.0;
            (6.0 + (k * PI).sin() * 1.6, 4.4 + (k * PI).sin() * 0.8)
        };
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = ((x as f32 - cx) / rx, (z as f32 - cz) / rz);
                let q = dx.abs().powf(2.4) + dz.abs().powf(2.4);
                if q > 1.0 || (y < 22 && q < 0.55) {
                    continue; // hollow over the boots
                }
                let fold = ((dz.atan2(dx) * 7.0).sin() * 0.5 + 0.5) * (1.0 - (y as f32 / 58.0) * 0.6);
                let hem = y <= 6 && nz(x, 1, z) < 0.35;
                if hem {
                    continue;
                }
                s.set(
                    x,
                    y,
                    z,
                    if fold > 0.72 {
                        ROBE_D
                    } else if fold < 0.2 {
                        ROBE_L
                    } else {
                        ROBE
                    },
                );
            }
        }
    }
    // The belt and its buckle.
    for y in 40..=42 {
        ring(&mut s, cx, cz, 6.6, 4.9, y, BELT);
        ring(&mut s, cx, cz, 6.2, 4.5, y, BELT);
    }
    for (x, y) in [(18, 40), (19, 40), (20, 40), (21, 40), (18, 42), (19, 42), (20, 42), (21, 42), (18, 41), (21, 41)] {
        face(&mut s, x, y, d, 1, BUCKLE);
    }
    // The neck.
    s.capsule((cx, 58.0, 13.0), (cx, 62.0, 13.0), 2.2, 2.0, WITCH_SKIN_D);

    // ---- arms: wide sleeves, green hands; the right one grips the staff
    for (i, ax) in [12.5f32, 26.5].into_iter().enumerate() {
        s.part(3 + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        ball(&mut s, v(ax, 56.0, 13.0), Vec3::splat(3.4), ROBE);
        sleeve(
            &mut s,
            &[(v(ax, 56.0, 13.0), 3.2), (v(ax + out * 1.5, 46.0, 15.0), 2.9), (v(ax + out * 1.2, 40.0, 17.5), 3.4)],
            ROBE,
            ROBE_D,
        );
        ball(&mut s, v(ax + out * 1.2, 38.6, 18.5), v(2.0, 1.9, 2.1), WITCH_SKIN);
        for (k, f) in [-1.2f32, -0.4, 0.4, 1.2].into_iter().enumerate() {
            let len = 22.2 - (k as f32 - 1.5).abs() * 0.5;
            s.capsule((ax + out * 1.2 + f, 38.2, 19.5), (ax + out * 1.2 + f * 1.1, 36.8, len), 0.6, 0.45, WITCH_SKIN_D);
        }
        if i == 1 {
            // The staff: gnarled wood, a knot at its head holding the green stone.
            let sx = ax + out * 1.8;
            s.capsule((sx, 10.0, 21.0), (sx + 0.6, 42.0, 21.0), 1.2, 1.1, STAFF);
            s.capsule((sx + 0.6, 42.0, 21.0), (sx - 0.4, 70.0, 20.6), 1.1, 1.0, STAFF);
            s.paint(|x, y, z, c| (c == STAFF && nz(x, y / 3, z) < 0.3).then_some(STAFF_D));
            ball(&mut s, v(sx - 0.4, 72.0, 20.6), v(2.8, 2.6, 2.8), STAFF_D);
            ball(&mut s, v(sx - 0.4, 74.6, 20.6), Vec3::splat(1.7), GEM);
            s.set((sx - 0.4).round() as i32, 75, 21, GEM_C);
        }
    }

    // ---- the head: a long green face, the nose hooked, yellow eyes, a grin, hair down the back
    s.part(5);
    ball(&mut s, v(cx, 67.0, 13.0), v(5.4, 6.4, 5.2), WITCH_SKIN);
    rounded(&mut s, v(cx, 62.5, 13.4), v(3.4, 2.4, 3.6), 2.6, 0, WITCH_SKIN_D); // the chin
    s.capsule((cx, 66.4, 17.6), (cx + 0.8, 63.4, 21.6), 1.3, 0.9, WITCH_SKIN);
    s.set((cx + 0.8).round() as i32, 63, 22, WITCH_SKIN_D);
    s.set(cx.round() as i32 + 2, 64, 20, WART);
    for (x, inward) in [(15, 1), (24, -1)] {
        eye(&mut s, x, 67, inward, d, 0xe0b020, HAIR);
    }
    for x in 17..=22 {
        face(&mut s, x, 61, d, 0, if x == 17 || x == 22 { WITCH_SKIN_D } else { CAVITY });
    }
    face(&mut s, 19, 61, d, 0, TOOTH);
    // Hair: long strands from the crown down over the shoulders and back.
    for k in 0..26 {
        let a = k as f32 / 26.0 * TAU;
        if a.sin() > 0.55 {
            continue; // not over her face
        }
        let (hx, hz) = (cx + a.cos() * 5.8, cz + a.sin() * 5.6);
        let len = 14 + (nz(k, 2, 5) * 12.0) as i32;
        for y in (71 - len)..=71 {
            let sway = ((y as f32) * 0.3 + k as f32).sin() * 0.6;
            s.set((hx + sway).round() as i32, y, hz.round() as i32, if nz(k, y, 1) < 0.3 { HAIR_L } else { HAIR });
        }
    }
    ball(&mut s, v(cx, 71.0, 12.0), v(5.8, 2.6, 5.6), HAIR);
    // The hat: a wide brim, a band, and a tall cone bending over to the right.
    for y in 72..=73 {
        for z in 0..d {
            for x in 0..w {
                let r = ((x as f32 - cx).powi(2) / (12.0f32).powi(2) + (z as f32 - cz).powi(2) / (11.0f32).powi(2)).sqrt();
                let wave = (x as f32 * 0.5).sin() * 0.05;
                if r <= 1.0 + wave {
                    s.set(x, y, z, if r > 0.92 { HAT_L } else { HAT });
                }
            }
        }
    }
    for y in 74..=94 {
        let k = (y - 74) as f32 / 20.0;
        let r = 7.4 * (1.0 - k).powf(0.85) + 0.6;
        let bend = (k * k) * 7.0;
        let droop = (k - 0.7).max(0.0) * 6.0;
        let (hx, hy) = (cx + bend, y as f32 - droop);
        for z in 0..d {
            for x in 0..w {
                let q = ((x as f32 - hx) / r).powi(2) + ((z as f32 - cz) / (r * 0.95)).powi(2);
                if q <= 1.0 {
                    let c = if y <= 76 {
                        BAND
                    } else if (x as f32) < hx - r * 0.4 {
                        HAT_L
                    } else {
                        HAT
                    };
                    s.set(x, hy.round() as i32, z, c);
                }
            }
        }
    }
    s.finish(0.04)
}

// --------------------------------------------------------------------- minion

/// The witch minion's grid: 26 wide, 44 tall, 22 deep.
pub const MINION_SIZE: (i32, i32, i32) = (26, 44, 22);

/**
 * A witch minion: a knee-high witchling, all robe and hat, two big dark eyes
 * in a pale little face under a hat too big for it, and two small hands.
 */
pub fn minion() -> Collected {
    let (w, h, d) = MINION_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (12.5f32, 10.5f32);
    // The robe, a cone to the floor, ragged at the hem.
    for y in 0..=25 {
        let k = y as f32 / 25.0;
        let (rx, rz) = (9.4 - k * 4.4, 7.6 - k * 3.6);
        for z in 0..d {
            for x in 0..w {
                let q = ((x as f32 - cx) / rx).powi(2) + ((z as f32 - cz) / rz).powi(2);
                if q > 1.0 || (y <= 2 && nz(x, 1, z) < 0.4) {
                    continue;
                }
                let fold = ((z as f32 - cz).atan2(x as f32 - cx) * 5.0).sin();
                s.set(x, y, z, if fold > 0.6 { MINION_ROBE_D } else { MINION_ROBE });
            }
        }
    }
    // Arms: two little sleeves with hands.
    for side in [-1.0f32, 1.0] {
        let ax = cx + side * 5.5;
        s.capsule((ax, 22.0, 11.0), (ax + side * 2.0, 15.0, 14.5), 2.0, 1.7, MINION_ROBE);
        ball(&mut s, v(ax + side * 2.0, 13.8, 15.4), Vec3::splat(1.5), MINION_SKIN);
    }
    // The head, the hair, the face.
    ball(&mut s, v(cx, 30.0, 10.5), v(5.6, 5.0, 5.4), MINION_SKIN);
    ball(&mut s, v(cx, 31.5, 9.0), v(6.0, 4.6, 5.6), MINION_HAIR);
    s.paint(|_, y, z, c| (c == MINION_HAIR && z as f32 > cz + 2.5 && y < 33).then_some(MINION_SKIN));
    for side in [-1i32, 1] {
        for y in 28..=31 {
            s.set(cx as i32 + side * 6, y, 10, MINION_HAIR);
            s.set(cx as i32 + side * 6, y, 11, MINION_HAIR);
        }
    }
    for x in [9, 10, 14, 15] {
        for y in [29, 30] {
            face(&mut s, x, y, d, 0, PUPIL);
        }
    }
    face(&mut s, 10, 30, d, 0, GLINT);
    face(&mut s, 15, 30, d, 0, GLINT);
    face(&mut s, 12, 27, d, 0, MOUTH);
    // The hat: brim and a bent cone.
    for y in 34..=35 {
        s.ellipsoid((cx, y as f32, cz), (10.4, 0.6, 9.6), y, if y == 35 { HAT_L } else { HAT });
    }
    for y in 36..=43 {
        let k = (y - 36) as f32 / 7.0;
        let r = 5.2 * (1.0 - k) + 0.8;
        let hx = cx + k * k * 5.0;
        for z in 0..d {
            for x in 0..w {
                if ((x as f32 - hx) / r).powi(2) + ((z as f32 - cz) / r).powi(2) <= 1.0 {
                    s.set(x, y, z, if y == 36 { BAND } else { HAT });
                }
            }
        }
    }
    s.finish_grounded(0.04)
}

// ------------------------------------------------------------ helmeted skeleton

/// The helmeted skeleton's grid: 40 wide, 80 tall, 22 deep.
pub const HELMET_SIZE: (i32, i32, i32) = (40, 80, 22);

/// Hips, shoulders and neck; the right hand swings a mace.
pub const HELMET_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [15.5, 28.0, 11.0] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [23.5, 28.0, 11.0] },
    PartDef { name: "leftArm", boxes: &[], pivot: [7.5, 54.0, 11.0] },
    PartDef { name: "rightArm", boxes: &[], pivot: [31.5, 54.0, 11.0] },
    PartDef { name: "head", boxes: &[], pivot: [19.5, 58.0, 11.0] },
];

/**
 * The helmeted skeleton: clean old bones, a ribcage round a dark hollow, a
 * rounded iron helmet with a nasal bar and cheek guards, dented, over a skull
 * with red fire in its sockets, a claw of a left hand and a spiked mace in
 * the right.
 */
pub fn helmeted_skeleton() -> Collected {
    let (w, h, d) = HELMET_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (19.5f32, 11.0f32);

    // ---- legs
    for (i, lx) in [15.5f32, 23.5].into_iter().enumerate() {
        s.part(1 + i as u8);
        ball(&mut s, v(lx, 27.4, 11.0), Vec3::splat(2.5), BONE_S);
        limb(&mut s, &[(v(lx, 27.0, 11.0), 1.9), (v(lx, 21.0, 11.2), 1.5), (v(lx, 16.6, 11.4), 1.9)], BONE);
        ball(&mut s, v(lx, 16.0, 11.2), v(2.6, 2.0, 2.3), BONE_S);
        limb(&mut s, &[(v(lx, 15.0, 11.3), 1.8), (v(lx, 9.0, 11.0), 1.3), (v(lx, 4.0, 10.8), 1.6)], BONE);
        s.capsule((lx + if i == 0 { -1.8 } else { 1.8 }, 14.0, 10.2), (lx + if i == 0 { -1.6 } else { 1.6 }, 4.6, 10.0), 0.7, 0.6, BONE_S);
        ball(&mut s, v(lx, 2.6, 10.6), v(2.2, 1.7, 2.3), BONE_S);
        for (k, f) in [-1.8f32, -0.6, 0.6, 1.8].into_iter().enumerate() {
            let tip = 17.5 - (k as f32 - 1.5).abs() * 0.8;
            s.capsule((lx + f * 0.5, 2.0, 11.0), (lx + f, 0.8, tip), 0.7, 0.5, BONE);
        }
    }

    // ---- the pelvis, spine, ribs and shoulders
    s.part(0);
    ball(&mut s, v(cx, 29.6, 10.5), v(6.4, 2.8, 3.6), BONE_S);
    for y in 31..=55 {
        ball(&mut s, v(cx, y as f32, 7.6), v(1.7, 0.6, 1.6), if y % 2 == 0 { BONE } else { BONE_D });
    }
    for y in 38..=52 {
        for x in (cx - 5.0) as i32..=(cx + 5.0) as i32 {
            s.set(x, y, 8, CAVITY);
        }
    }
    for k in 0..6 {
        let back = 52.0 - k as f32 * 2.7;
        let (rx, rz) = (6.4 + (k as f32 * 0.7).sin() * 1.2, 4.4);
        for j in 0..56 {
            let a = j as f32 / 56.0 * TAU;
            let (sa, ca) = (a.sin(), a.cos());
            if k >= 4 && sa > 0.5 {
                continue;
            }
            let y = (back - 2.4 * (1.0 + sa) / 2.0).round() as i32;
            s.set((cx + ca * rx).round() as i32, y, (cz + sa * rz).round() as i32, if j % 5 == 0 { BONE_S } else { BONE });
        }
    }
    s.capsule((cx, 51.0, 15.6), (cx, 41.0, 15.4), 1.1, 0.9, BONE);
    for side in [-1.0f32, 1.0] {
        s.capsule((cx + side * 1.2, 53.4, 14.8), (cx + side * 11.5, 54.6, 11.0), 0.9, 0.9, BONE_S);
    }
    s.capsule((cx, 55.0, 11.0), (cx, 58.4, 11.0), 1.4, 1.2, BONE);

    // ---- arms: a claw on the left, the mace in the right fist
    for (i, ax) in [7.5f32, 31.5].into_iter().enumerate() {
        s.part(3 + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        ball(&mut s, v(ax, 53.6, 11.0), Vec3::splat(2.8), BONE_S);
        limb(&mut s, &[(v(ax, 53.0, 11.0), 1.7), (v(ax + out * 0.4, 47.0, 11.4), 1.3), (v(ax + out * 0.6, 43.0, 11.8), 1.6)], BONE);
        ball(&mut s, v(ax + out * 0.6, 42.4, 12.0), v(2.1, 2.0, 2.1), BONE_S);
        for (k, dx) in [-0.9f32, 0.9].into_iter().enumerate() {
            s.capsule((ax + out * 0.6 + dx, 41.6, 12.0), (ax + dx * 0.9, 33.0, 13.4), 1.0, 0.8, if k == 0 { BONE } else { BONE_S });
        }
        ball(&mut s, v(ax, 31.8, 13.6), v(2.0, 1.5, 2.0), BONE_S);
        if i == 0 {
            for (k, f) in [-1.8f32, -0.6, 0.6, 1.8].into_iter().enumerate() {
                let reach = 18.4 - (k as f32 - 1.5).abs() * 0.7;
                s.capsule((ax + f * 0.8, 30.6, 15.0), (ax + f * 1.2, 27.6, reach), 0.6, 0.4, BONE);
            }
        } else {
            // The mace: an iron haft and a spiked ball.
            s.capsule((ax, 34.0, 15.0), (ax + 0.6, 18.0, 16.0), 0.9, 0.9, IRON);
            ball(&mut s, v(ax + 0.6, 15.6, 16.0), Vec3::splat(3.0), IRON_D);
            for k in 0..8 {
                let a = k as f32 / 8.0 * TAU;
                let n = Vec3::new(a.cos() * 0.8, (k as f32 * 1.7).sin() * 0.5, a.sin() * 0.8).normalize();
                let base = v(ax + 0.6, 15.6, 16.0) + n * 2.6;
                s.capsule((base.x, base.y, base.z), (base.x + n.x * 2.2, base.y + n.y * 2.2, base.z + n.z * 2.2), 0.7, 0.3, IRON);
            }
        }
    }

    // ---- the skull and the helmet
    s.part(5);
    ball(&mut s, v(cx, 63.5, 11.0), v(5.2, 5.6, 5.0), BONE);
    rounded(&mut s, v(cx, 58.6, 11.6), v(3.6, 2.2, 3.6), 2.4, 0, BONE_S); // the jaw
    for x in 16..=23 {
        if x % 2 == 0 {
            face(&mut s, x, 60, d, 0, TOOTH);
            face(&mut s, x, 59, d, 0, TOOTH);
        } else {
            face(&mut s, x, 60, d, 0, CAVITY);
        }
    }
    face(&mut s, 19, 62, d, 0, CAVITY);
    face(&mut s, 20, 62, d, 0, CAVITY);
    fire_socket(&mut s, 17, 65, d, RED_FIRE);
    fire_socket(&mut s, 22, 65, d, RED_FIRE);
    // The helmet: a rounded cap down to the brow, a nasal bar, cheek guards, a dent on the left.
    for y in 63..=72 {
        let k = (y - 63) as f32 / 9.0;
        let r = 6.6 * (1.0 - k * k * 0.85).sqrt() + 0.4;
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = ((x as f32 - cx) / r, (z as f32 - cz) / (r * 0.96));
                let q = dx * dx + dz * dz;
                let inner = ((x as f32 - cx) / (r - 1.6)).powi(2) + ((z as f32 - cz) / ((r - 1.6) * 0.96)).powi(2);
                if q > 1.0 || (inner <= 1.0 && y < 71) {
                    continue;
                }
                let dent = ((x as f32 - 14.0).powi(2) + (y as f32 - 68.0).powi(2) + (z as f32 - 12.0).powi(2)).sqrt() < 2.6;
                let c = if y <= 64 || dent {
                    HELM_D
                } else if nz(x, y, z) > 0.8 {
                    HELM_L
                } else {
                    HELM
                };
                s.set(x, y, z, c);
            }
        }
    }
    s.cuboid(19, 58, 16, 20, 64, 17, HELM_D); // the nasal bar
    for x in [13, 26] {
        s.cuboid(x, 57, 10, x + 1, 63, 15, HELM);
    }
    s.set(19, 73, 11, HELM_L); // a stud at the crown
    s.finish(0.04)
}

// ----------------------------------------------------------------- loup-garou

/// The loup-garou's grid: 30 wide, 46 tall, 66 long (it faces +z).
pub const WOLF_SIZE: (i32, i32, i32) = (30, 46, 66);

/// Four legs at the shoulders and hips, the head at the neck, the tail at the rump.
pub const WOLF_RIG: &[PartDef] = &[
    PartDef { name: "frontLeftLeg", boxes: &[], pivot: [9.5, 26.0, 46.0] },
    PartDef { name: "frontRightLeg", boxes: &[], pivot: [19.5, 26.0, 46.0] },
    PartDef { name: "backLeftLeg", boxes: &[], pivot: [9.5, 26.0, 16.0] },
    PartDef { name: "backRightLeg", boxes: &[], pivot: [19.5, 26.0, 16.0] },
    PartDef { name: "head", boxes: &[], pivot: [14.5, 34.0, 50.0] },
    PartDef { name: "tail", boxes: &[], pivot: [14.5, 30.0, 8.0] },
];

/**
 * The loup-garou: a wolf the size of a pony, shaggy dark fur with a ruff at
 * the neck, a long muzzle open on a mouth of teeth, red eyes, ears pricked,
 * claws on every paw and a brush of a tail.
 */
pub fn loup_garou() -> Collected {
    let (w, h, d) = WOLF_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let cx = 14.5f32;

    // ---- legs, from the shoulders and hips down to the paws
    for (i, (lx, lz)) in [(9.5f32, 46.0f32), (19.5, 46.0), (9.5, 16.0), (19.5, 16.0)].into_iter().enumerate() {
        s.part(1 + i as u8);
        let back = i >= 2;
        ball(&mut s, v(lx, 26.0, lz), v(4.2, 4.6, 5.0), FUR);
        s.capsule((lx, 26.0, lz), (lx, 15.0, lz + if back { -2.0 } else { 1.5 }), 3.6, 2.7, FUR);
        s.capsule((lx, 15.0, lz + if back { -2.0 } else { 1.5 }), (lx, 5.0, lz + if back { 1.0 } else { 0.5 }), 2.6, 2.2, FUR_D);
        let pz = lz + 2.5;
        ball(&mut s, v(lx, 2.4, pz), v(3.0, 2.3, 3.6), FUR_D);
        for f in [-1.6f32, 0.0, 1.6] {
            s.capsule((lx + f, 1.6, pz + 2.6), (lx + f * 1.2, 0.8, pz + 5.2), 0.7, 0.35, CLAW);
        }
    }

    // ---- the body: chest deep, the back rising to the shoulders, a ruff at the neck, a paler belly
    s.part(0);
    s.capsule((cx, 29.0, 14.0), (cx, 32.0, 48.0), 7.6, 8.6, FUR);
    ball(&mut s, v(cx, 30.0, 42.0), v(9.2, 9.4, 9.0), FUR);
    ball(&mut s, v(cx, 27.0, 26.0), v(7.0, 5.6, 12.0), BELLY);
    s.paint(|_, y, _, c| (c == BELLY && y > 27).then_some(FUR));
    ball(&mut s, v(cx, 34.0, 48.0), v(9.6, 8.4, 4.4), FUR_D);
    ball(&mut s, v(cx, 36.0, 34.0), v(5.0, 3.6, 10.0), FUR_D); // the ridge of fur down the back

    // ---- the tail
    s.part(6);
    s.capsule((cx, 30.0, 10.0), (cx, 36.0, 0.0), 2.5, 1.7, FUR);
    ball(&mut s, v(cx, 36.4, 0.8), Vec3::splat(2.0), FUR_TIP);

    // Shaggy along the back and flanks (the head stays clean, so it reads).
    shag(&mut s, (w, h, d), FUR, FUR_L, FUR_TIP, 0.14, 1);

    // ---- the head: a broad skull, a pale muzzle open on its teeth, red eyes under dark brows, ears pricked
    s.part(5);
    ball(&mut s, v(cx, 37.0, 53.0), v(7.2, 6.6, 6.6), FUR);
    ball(&mut s, v(cx, 36.0, 56.5), v(5.2, 4.4, 4.0), BELLY);
    s.capsule((cx, 35.0, 56.0), (cx, 34.2, 63.5), 4.2, 3.2, BELLY);
    s.paint(|_, y, z, c| (c == BELLY && z > 50 && y > 37).then_some(FUR));
    ball(&mut s, v(cx, 35.0, 65.0), v(1.8, 1.5, 1.4), NOSE);
    s.capsule((cx, 31.2, 55.0), (cx, 30.4, 62.5), 3.0, 2.3, BELLY);
    s.cuboid(12, 32, 57, 17, 33, 64, MOUTH);
    for z in (57..=63).step_by(2) {
        for x in [12, 17] {
            s.set(x, 33, z, TOOTH);
            s.set(x, 32, z + 1, TOOTH);
        }
    }
    for x in [13, 16] {
        s.set(x, 32, 63, TOOTH);
        s.set(x, 34, 63, TOOTH);
    }
    for side in [-1.0f32, 1.0] {
        let ex = cx + side * 3.6;
        ball(&mut s, v(ex, 38.8, 58.8), v(1.7, 1.4, 1.4), RED_FIRE);
        s.set(ex.round() as i32, 39, 60, RED_CORE);
        for k in -2..=2 {
            s.set((ex + k as f32).round() as i32, 41, 59, FUR_D); // the brow
        }
        s.capsule((cx + side * 4.0, 41.0, 51.0), (cx + side * 6.4, 47.0, 49.5), 2.2, 0.7, FUR);
        s.set((cx + side * 4.6).round() as i32, 43, 52, EAR_IN);
        s.set((cx + side * 5.2).round() as i32, 44, 51, EAR_IN);
        s.set((cx + side * 5.0).round() as i32, 45, 51, EAR_IN);
    }
    s.finish(0.05)
}

// ------------------------------------------------------------------- blemmyes

/// The blemmyes' grid: 48 wide, 70 tall, 28 deep.
pub const BLEMMYES_SIZE: (i32, i32, i32) = (48, 70, 28);

/// Hips and shoulders; there is no neck, and no head to swing.
pub const BLEMMYES_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [16.5, 24.0, 13.0] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [30.5, 24.0, 13.0] },
    PartDef { name: "leftArm", boxes: &[], pivot: [5.5, 54.0, 13.0] },
    PartDef { name: "rightArm", boxes: &[], pivot: [41.5, 54.0, 13.0] },
];

/**
 * The blemmyes: a headless brute, broad as a door, with his face in his
 * chest: small eyes under a heavy brow, a squashed nose, a wide grin of
 * teeth. Bare feet, a ragged loincloth on a rope, and a knotted club.
 */
pub fn blemmyes() -> Collected {
    let (w, h, d) = BLEMMYES_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (23.5f32, 13.0f32);

    // ---- legs, thick, bare feet with toes
    for (i, lx) in [16.5f32, 30.5].into_iter().enumerate() {
        s.part(1 + i as u8);
        s.capsule((lx, 24.0, 13.0), (lx, 12.0, 13.2), 5.0, 4.2, SKIN);
        ball(&mut s, v(lx, 12.0, 13.4), Vec3::splat(4.4), SKIN_D);
        s.capsule((lx, 12.0, 13.2), (lx, 4.0, 13.0), 4.2, 3.8, SKIN);
        s.ellipsoid((lx, 2.2, 15.5), (4.6, 2.3, 6.6), 0, SKIN);
        for (k, f) in [-2.8f32, -1.4, 0.0, 1.4, 2.8].into_iter().enumerate() {
            let tip = 22.4 - (k as f32 - 2.0).abs() * 0.6;
            s.capsule((lx + f, 1.6, 20.0), (lx + f * 1.1, 1.2, tip), 1.0, 0.8, SKIN_D);
        }
    }

    // ---- the body: the belly, the barrel chest, the shoulders, the loincloth
    s.part(0);
    rounded(&mut s, v(cx, 44.0, 13.0), v(15.5, 15.0, 8.8), 2.6, 0, SKIN);
    ball(&mut s, v(cx, 33.0, 14.5), v(12.0, 8.0, 8.0), SKIN);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 15.0, 56.0, 13.0), v(6.4, 5.4, 6.4), SKIN);
    }
    s.paint(|x, y, z, c| (c == SKIN && nz(x / 2, y / 2, z / 2) < 0.12).then_some(SKIN_D));
    s.paint(|x, y, z, c| (c == SKIN && z as f32 > cz + 4.0 && y > 40 && nz(x, y, z) > 0.86).then_some(SKIN_L));
    for y in 20..=30 {
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = ((x as f32 - cx) / 13.2, (z as f32 - cz) / 9.4);
                let q = dx.abs().powf(2.6) + dz.abs().powf(2.6);
                let inner = ((x as f32 - cx) / 11.4).abs().powf(2.6) + ((z as f32 - cz) / 7.6).abs().powf(2.6);
                if q > 1.0 || inner <= 1.0 && y > 20 {
                    continue;
                }
                let hem = y <= 22 && nz(x, 3, z) < 0.45;
                if hem {
                    continue;
                }
                s.set(
                    x,
                    y,
                    z,
                    if y >= 29 {
                        ROPE
                    } else if nz(x, y, z) < 0.25 {
                        CLOTH_D
                    } else {
                        CLOTH
                    },
                );
            }
        }
    }
    // The face, in the chest: brow, eyes, nose, and a grin.
    for x in 13..=34 {
        face(&mut s, x, 53, d, 0, SKIN_D);
        face(&mut s, x, 54, d, 0, SKIN_D);
    }
    // Big pale eyes with small dark pupils, glaring from under the brow.
    for (x0, inward) in [(14, 1), (33, -1)] {
        for k in 0..5 {
            for y in 47..=50 {
                face(&mut s, x0 + k * inward, y, d, 0, EYE_WHITE);
            }
        }
        for k in 2..=3 {
            for y in 48..=49 {
                face(&mut s, x0 + k * inward, y, d, 0, PUPIL);
            }
        }
        face(&mut s, x0 + 2 * inward, 49, d, 0, GLINT);
        for k in 0..5 {
            face(&mut s, x0 + k * inward, 51, d, 0, SKIN_D);
        }
    }
    s.capsule((cx, 46.0, 21.0), (cx, 42.0, 24.0), 2.4, 2.8, SKIN_D);
    s.set(22, 42, 24, CAVITY);
    s.set(25, 42, 24, CAVITY);
    // A wide grin: lips round a dark mouth, a row of teeth above and below.
    for x in 14..=33 {
        face(&mut s, x, 33, d, 0, LIP);
        face(&mut s, x, 39, d, 0, LIP);
        for y in 34..=38 {
            face(&mut s, x, y, d, 0, MOUTH);
        }
        if x % 2 == 0 && x > 14 && x < 33 {
            face(&mut s, x, 38, d, 0, TOOTH);
            face(&mut s, x, 37, d, 0, TOOTH);
            face(&mut s, x, 34, d, 0, TOOTH);
        }
    }
    for y in 34..=38 {
        face(&mut s, 13, y, d, 0, LIP);
        face(&mut s, 34, y, d, 0, LIP);
    }

    // ---- arms: heavy, a fist on the left, the club in the right
    for (i, ax) in [5.5f32, 41.5].into_iter().enumerate() {
        s.part(3 + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        s.capsule((ax, 54.0, 13.0), (ax + out * 1.0, 42.0, 14.0), 5.0, 4.4, SKIN);
        ball(&mut s, v(ax + out * 1.0, 41.4, 14.2), Vec3::splat(4.5), SKIN_D);
        s.capsule((ax + out * 1.0, 41.4, 14.2), (ax + out * 1.6, 30.0, 16.5), 4.4, 4.0, SKIN);
        ball(&mut s, v(ax + out * 1.6, 27.0, 17.5), v(4.4, 3.8, 4.2), SKIN);
        for k in 0..4 {
            s.set((ax + out * 1.6 - 2.4 + k as f32 * 1.6).round() as i32, 25, 21, SKIN_D);
        }
        if i == 1 {
            // The club, knotted, dragged behind his fist.
            s.capsule((ax + out * 1.6, 28.0, 18.0), (ax + out * 3.0, 6.0, 24.0), 2.0, 3.6, CLUB);
            s.paint(|x, y, z, c| (c == CLUB && nz(x, y / 2, z) < 0.3).then_some(CLUB_D));
            for k in 0..5 {
                let a = k as f32 * 1.3;
                let p = v(ax + out * 3.0 + a.cos() * 3.2, 8.0 + k as f32 * 2.2, 24.0 + a.sin() * 3.2);
                ball(&mut s, p, Vec3::splat(1.3), CLUB_D);
            }
        }
    }
    s.finish(0.04)
}

// ------------------------------------------------------------------ large bat

/// The large bat's grid: 84 wide (wings spread), 30 tall, 44 long.
pub const BAT_SIZE: (i32, i32, i32) = (84, 30, 44);
const BAT_X: f32 = 41.5;

/// Wings hinge where they meet the body.
pub const BAT_RIG: &[PartDef] = &[
    PartDef { name: "leftWing", boxes: &[], pivot: [34.5, 15.0, 22.0] },
    PartDef { name: "rightWing", boxes: &[], pivot: [48.5, 15.0, 22.0] },
];

/**
 * The large bat: a furry body and a snub face with red eyes, fangs and big
 * ears, hung between wings of torn membrane stretched on long bone fingers,
 * hooked claws at the wrists and feet tucked under.
 */
pub fn large_bat() -> Collected {
    let (w, h, d) = BAT_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let cx = BAT_X;

    // ---- wings: the arm, the fingers fanning from the wrist, the membrane between them
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        let wrist = (cx + side * 20.0, 21.0f32, 27.0f32);
        s.capsule((cx + side * 6.0, 16.0, 24.0), wrist, 1.7, 1.3, BAT_BONE);
        let tips = [(cx + side * 40.0, 19.0, 30.0), (cx + side * 39.0, 18.0, 15.0), (cx + side * 31.0, 17.0, 3.0)];
        for (k, tip) in tips.into_iter().enumerate() {
            s.capsule(wrist, tip, 1.2 - k as f32 * 0.1, 0.5, BAT_BONE);
        }
        // The thumb claw, hooked forward from the wrist.
        s.capsule(wrist, (wrist.0 + side * 1.0, wrist.1 + 0.8, wrist.2 + 4.0), 0.9, 0.35, CLAW);
        let poly = [
            (cx + side * 5.0, 30.0),
            (cx + side * 20.0, 29.0),
            (tips[0].0, tips[0].2),
            (cx + side * 42.0, 22.0),
            (tips[1].0, tips[1].2),
            (cx + side * 36.0, 9.0),
            (tips[2].0, tips[2].2),
            (cx + side * 5.0, 8.0),
        ];
        let (x0, x1) = if side < 0.0 { (0, cx as i32) } else { (cx as i32, w - 1) };
        for z in 0..d {
            for x in x0..=x1 {
                if !inside(&poly, x as f32, z as f32) {
                    continue;
                }
                let u = ((x as f32 - cx).abs() - 5.0) / 37.0;
                // The membrane sags between the fingers.
                let along = ((z as f32 - 8.0) / 22.0 * PI).sin();
                let y = 17.0 + u * 3.5 - along * 1.8 * u.max(0.2);
                let near_edge = [(tips[0].0, tips[0].2), (tips[1].0, tips[1].2), (tips[2].0, tips[2].2)]
                    .iter()
                    .any(|t| ((x as f32 - t.0).powi(2) + (z as f32 - t.1).powi(2)).sqrt() < 4.0);
                if near_edge && nz(x, 1, z) < 0.5 {
                    continue; // frayed at the tips
                }
                if nz(x / 2, 5, z / 2) < 0.05 {
                    continue; // a hole
                }
                let vein = ((x as f32 * 0.35 + z as f32 * 0.2).sin() * (z as f32 * 0.5).cos()).abs() < 0.08;
                let c = if vein {
                    VEIN
                } else if nz(x, 2, z) < 0.3 {
                    MEMBRANE_D
                } else {
                    MEMBRANE
                };
                s.set(x, y.round() as i32, z, c);
            }
        }
    }

    // ---- the body, head, ears, eyes, fangs and feet
    s.part(0);
    ball(&mut s, v(cx, 15.0, 22.0), v(7.2, 6.6, 9.4), BAT_FUR);
    ball(&mut s, v(cx, 18.0, 30.0), v(5.6, 5.2, 5.4), BAT_FUR);
    ball(&mut s, v(cx, 16.4, 34.4), v(2.6, 2.0, 2.2), BAT_FUR_L);
    ball(&mut s, v(cx, 16.6, 36.2), v(1.3, 1.0, 1.0), NOSE);
    for side in [-1.0f32, 1.0] {
        s.capsule((cx + side * 3.5, 21.0, 30.0), (cx + side * 5.8, 29.0, 29.0), 2.2, 0.7, BAT_FUR);
        s.set((cx + side * 4.4).round() as i32, 24, 31, EAR_IN);
        s.set((cx + side * 4.8).round() as i32, 26, 31, EAR_IN);
        ball(&mut s, v(cx + side * 2.6, 18.8, 34.6), v(1.3, 1.2, 1.1), RED_FIRE);
        s.set((cx + side * 2.6).round() as i32, 19, 36, RED_CORE);
        s.set((cx + side * 1.4).round() as i32, 14, 35, TOOTH);
        s.set((cx + side * 1.4).round() as i32, 13, 35, TOOTH);
        s.capsule((cx + side * 3.0, 10.0, 18.0), (cx + side * 4.0, 5.0, 15.0), 1.2, 0.8, BAT_BONE);
        for f in [-1.0f32, 0.0, 1.0] {
            s.capsule((cx + side * 4.0 + f, 5.0, 15.0), (cx + side * 4.0 + f * 1.4, 3.6, 13.0), 0.5, 0.3, CLAW);
        }
    }
    shag(&mut s, (w, h, d), BAT_FUR, BAT_FUR_L, BAT_FUR_L, 0.25, 1);
    s.finish(0.04)
}
