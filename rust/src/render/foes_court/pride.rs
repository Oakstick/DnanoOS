//! Pride's court's own (PRINCES.md §1): Lucifer, the prince of Pride, and the Reflections that
//! step out of his mirrors. Each faces +z, stands in the middle of its grid and is sculpted in the
//! parts its rig swings.
//!
//! Lucifer is the most beautiful of the angels, two and a half times the hero's height: plate of
//! white enamel edged in gold over a white tabard, the morning star on his breast, a face like a
//! statue's in pale marble under long pale-gold hair, a circlet from which a crown of light rises,
//! a gold halo behind his head, and a sword of light held point-down in his right fist. Six wings
//! (the seraph's: two over his head, two spread to fly, two down about his feet) stand from his
//! back, white; a second six, burnt black and smoking at their edges, are sculpted in the same
//! place in parts of their own, shown in his third phase in place of the white
//! (`render::view::court_pride` chooses).
//!
//! A Reflection is the same figure as in a mirror: left and right swapped (the sword in its left
//! hand), every colour turned to cold silver, its halo silver, smaller, and without the burnt wings.

use std::f32::consts::{PI, TAU};

use glam::Vec3;

use crate::render::carve::{ball, front, limb, nz, v};
use crate::render::characters::{mat, Sculpt};
use crate::render::voxels::{Collected, PartDef, Voxel};

mod pal {
    use super::mat;
    pub const PLATE: u32 = 0xf2ead8 | mat::METAL;
    pub const PLATE_L: u32 = 0xfffaf0 | mat::METAL;
    pub const PLATE_D: u32 = 0xc9bea4 | mat::METAL;
    pub const GOLD: u32 = 0xe8c060 | mat::GOLD;
    pub const GOLD_L: u32 = 0xffe8a0 | mat::GOLD;
    pub const GOLD_D: u32 = 0xb08a38 | mat::GOLD;
    pub const SKIN: u32 = 0xf2e6d6 | mat::SKIN;
    pub const SKIN_D: u32 = 0xd6c4ae | mat::SKIN;
    pub const LIP: u32 = 0xc8a49a | mat::SKIN;
    pub const HAIR: u32 = 0xf2d88e | mat::FUR;
    pub const HAIR_D: u32 = 0xd2b062 | mat::FUR;
    pub const LINING: u32 = 0x2c3c7a | mat::CLOTH;
    pub const LINING_D: u32 = 0x22305f | mat::CLOTH;
    pub const LIGHT: u32 = 0xfff0b8 | mat::GLOW;
    pub const LIGHT_W: u32 = 0xfffcf0 | mat::GLOW;
    pub const EYE: u32 = 0xffe6a0 | mat::GLOW;
    pub const GEM: u32 = 0x9ad0ff | mat::GLOW;
    pub const FEATHER: u32 = 0xfaf8f2 | mat::CLOTH;
    pub const FEATHER_S: u32 = 0xe8e4da | mat::CLOTH;
    pub const FEATHER_T: u32 = 0xcac6bc | mat::CLOTH;
    pub const CHAR: u32 = 0x1e1a18 | mat::LEATHER;
    pub const CHAR_L: u32 = 0x302824 | mat::LEATHER;
    pub const ASH: u32 = 0x55504a | mat::LEATHER;
    pub const EMBER: u32 = 0xff6a1a | mat::GLOW;
    pub const EMBER_Y: u32 = 0xffb040 | mat::GLOW;
}
use pal::*;

/// Lucifer's grid and voxel: about three tiles tall (his wings' tips), two and a half times the
/// hero to his crown.
pub const LUCIFER_SIZE: (i32, i32, i32) = (100, 124, 80);
pub const LUCIFER_VOXEL: f32 = 0.025;
const MX: f32 = 50.0;
const MZ: f32 = 46.0;

/// Hips, shoulders, neck, and the roots of his wings (the white six, then the burnt six); the sword
/// goes with his right arm, the halo and the crown of light with his head.
pub const LUCIFER_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [MX - 9.0, 52.0, MZ] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [MX + 9.0, 52.0, MZ] },
    PartDef { name: "leftArm", boxes: &[], pivot: [MX - 22.0, 85.0, MZ] },
    PartDef { name: "rightArm", boxes: &[], pivot: [MX + 22.0, 85.0, MZ] },
    PartDef { name: "head", boxes: &[], pivot: [MX, 91.0, MZ + 1.0] },
    PartDef { name: "leftWings", boxes: &[], pivot: [MX - 7.0, 82.0, MZ - 12.0] },
    PartDef { name: "rightWings", boxes: &[], pivot: [MX + 7.0, 82.0, MZ - 12.0] },
    PartDef { name: "leftBurnt", boxes: &[], pivot: [MX - 7.0, 82.0, MZ - 12.0] },
    PartDef { name: "rightBurnt", boxes: &[], pivot: [MX + 7.0, 82.0, MZ - 12.0] },
];

/// The rig parts of his white wings and of his burnt ones (indices into the rig).
pub const WHITE_WINGS: [&str; 2] = ["leftWings", "rightWings"];
pub const BURNT_WINGS: [&str; 2] = ["leftBurnt", "rightBurnt"];

/// A Reflection's grid, voxel and rig: his, as in a mirror, smaller, with no burnt wings.
pub const REFLECTION_SIZE: (i32, i32, i32) = LUCIFER_SIZE;
pub const REFLECTION_VOXEL: f32 = 0.0168;
pub const REFLECTION_RIG: &[PartDef] = LUCIFER_RIG.split_at(7).0;

/// The armour's half-widths (x across, z deep) of his fauld of plates at height `y`.
fn fauld(y: f32) -> (f32, f32) {
    let k = ((56.0 - y) / 20.0).clamp(0.0, 1.0);
    (14.5 + k * 4.5, 11.5 + k * 3.0)
}

/// One wing of six: a bone along a curve from `root` (bent toward `bend`) to `tip`, its feathers
/// hanging from it along `fall`, `long(t)` voxels long at `t` along the bone. `burnt`: black and
/// ragged, embers at its edges.
fn wing(s: &mut Sculpt, root: Vec3, bend: Vec3, tip: Vec3, fall: Vec3, long: impl Fn(f32) -> f32, burnt: bool) {
    let fall = fall.normalize();
    let n: usize = 72;
    let bone = |t: f32| root * (1.0 - t) * (1.0 - t) + bend * 2.0 * t * (1.0 - t) + tip * t * t;
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let b = bone(t);
        let len = long(t).max(1.0);
        let steps = len.ceil() as i32;
        for j in 0..=steps {
            let u = j as f32 / len;
            let p = b + fall * j as f32 + Vec3::new(0.0, 0.0, -1.5 * u);
            let (x, y, z) = (p.x.round() as i32, p.y.round() as i32, p.z.round() as i32);
            let seam = i.is_multiple_of(6) && u > 0.6;
            let c = if burnt {
                let h = nz(x, y, z);
                // Ragged: holes eaten through toward the tips, embers glowing at the burnt edges.
                if u > 0.55 && h < (u - 0.5) * 0.9 {
                    continue;
                }
                if u > 0.8 && h > 0.86 {
                    if h > 0.94 {
                        EMBER_Y
                    } else {
                        EMBER
                    }
                } else if seam || u > 0.9 {
                    ASH
                } else if (i / 2 + j as usize / 3).is_multiple_of(3) {
                    CHAR_L
                } else {
                    CHAR
                }
            } else if u > 0.92 {
                FEATHER_T
            } else if u > 0.4 || seam {
                FEATHER_S
            } else {
                FEATHER
            };
            s.set(x, y, z, c);
            s.set(x, y, z - 1, c);
        }
    }
    // The bone along its leading edge, thick at the root.
    let knots: Vec<(Vec3, f32)> = (0..=8).map(|k| k as f32 / 8.0).map(|t| (bone(t), 2.6 - 1.6 * t)).collect();
    limb(s, &knots, if burnt { CHAR_L } else { FEATHER });
}

/// The six wings on one side (`side` -1 his left, +1 his right), white or burnt.
fn wings(s: &mut Sculpt, side: f32, burnt: bool) {
    let x = |dx: f32| MX + side * dx;
    // Two over his head, rising.
    wing(
        s,
        v(x(8.0), 87.0, MZ - 13.0),
        v(x(19.0), 109.0, MZ - 20.0),
        v(x(23.0), 123.0, MZ - 15.0),
        Vec3::new(side * 0.3, -1.0, -0.05),
        |t| 4.0 + 17.0 * t.powf(0.9),
        burnt,
    );
    // Two spread to fly.
    wing(
        s,
        v(x(9.0), 82.0, MZ - 13.0),
        v(x(30.0), 97.0, MZ - 21.0),
        v(x(47.0), 93.0, MZ - 17.0),
        Vec3::new(side * 0.12, -1.0, -0.12),
        |t| 5.0 + 22.0 * t.powf(0.8),
        burnt,
    );
    // Two down about his feet.
    wing(
        s,
        v(x(8.0), 74.0, MZ - 12.0),
        v(x(27.0), 62.0, MZ - 19.0),
        v(x(30.0), 28.0, MZ - 15.0),
        Vec3::new(side * 0.7, -0.4, -0.1),
        |t| 4.0 + 12.0 * (PI * t).sin(),
        burnt,
    );
}

/// The morning star: eight rays of gold from a heart of light, facing +z at `c`.
fn star(s: &mut Sculpt, c: Vec3, r: f32) {
    for k in 0..8 {
        let a = k as f32 * TAU / 8.0;
        let long = if k % 2 == 0 { r } else { r * 0.55 };
        let tip = c + Vec3::new(a.cos() * long, a.sin() * long, 0.0);
        limb(s, &[(c, 1.3), (tip, 0.5)], if k % 2 == 0 { GOLD_L } else { GOLD });
    }
    ball(s, c + Vec3::new(0.0, 0.0, 0.5), Vec3::splat(1.6), LIGHT_W);
}

/// Lucifer's body: everything but the burnt wings (`lucifer` adds them).
fn body() -> Sculpt {
    let (w, h, d) = LUCIFER_SIZE;
    let mut s = Sculpt::new(w, h, d);

    // ---- legs: plate of white enamel, gold at the knee and down the shin, pointed sabatons
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        let x = MX + side * 9.0;
        let knee = v(x + side, 28.0, MZ + 2.0);
        let ankle = v(x + side, 8.0, MZ + 1.0);
        limb(&mut s, &[(v(x, 52.0, MZ), 7.5), (knee, 6.2)], PLATE);
        limb(&mut s, &[(knee, 6.0), (ankle, 4.4)], PLATE);
        ball(&mut s, knee + v(0.0, 0.0, 2.0), v(5.2, 5.0, 4.6), GOLD);
        ball(&mut s, knee + v(0.0, 0.5, 4.5), Vec3::splat(1.4), GEM);
        // A gold ridge down the front of the greave.
        limb(&mut s, &[(knee + v(0.0, -4.0, 5.0), 1.1), (ankle + v(0.0, 2.0, 4.0), 0.9)], GOLD_D);
        ball(&mut s, v(x + side, 3.5, MZ + 6.0), v(5.0, 3.6, 9.0), PLATE_D);
        limb(&mut s, &[(v(x + side, 2.5, MZ + 12.0), 2.6), (v(x + side, 1.2, MZ + 17.0), 1.0)], GOLD);
        s.capsule((ankle.x, 7.0, ankle.z), (ankle.x, 9.5, ankle.z), 5.0, 5.0, GOLD_D);
    }

    // ---- the body: the fauld of plates over his hips, the tabard hanging before and behind
    s.part(0);
    for y in 34..=57 {
        let yf = y as f32;
        let (rx, rz) = fauld(yf);
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = ((x as f32 - MX) / rx, (z as f32 - MZ) / rz);
                if dx * dx + dz * dz > 1.0 {
                    continue;
                }
                // Plates overlapping in bands, each edged in gold at its foot; open at the front.
                let band = (57 - y) % 6;
                let open = (x as f32 - MX).abs() < 6.0 && dz > 0.3 && y < 52;
                if open {
                    continue;
                }
                s.set(
                    x,
                    y,
                    z,
                    if band == 5 {
                        GOLD
                    } else if band == 0 {
                        PLATE_L
                    } else {
                        PLATE
                    },
                );
            }
        }
    }
    // The tabard: the morning sky's blue hanging from the belt to his shins, before and behind,
    // trimmed in gold.
    for (zf, sign) in [(MZ + 12.0, 1.0f32), (MZ - 12.0, -1.0)] {
        for y in 14..=56 {
            let half = 6.5 + (56 - y) as f32 * 0.06;
            for x in (MX - half).round() as i32..=(MX + half).round() as i32 {
                let edge = (x as f32 - MX).abs() > half - 1.2 || y <= 15;
                for k in 0..2 {
                    let z = (zf + sign * (k as f32 + (56 - y) as f32 * 0.05)).round() as i32;
                    s.set(
                        x,
                        y,
                        z,
                        if edge {
                            GOLD
                        } else if k == 1 && (x + y) % 9 == 0 {
                            LINING_D
                        } else {
                            LINING
                        },
                    );
                }
            }
        }
    }
    star(&mut s, v(MX, 36.0, MZ + 14.5), 4.5);
    // The belt of gold plates.
    for y in 55..=58 {
        for z in 0..d {
            for x in 0..w {
                let q = ((x as f32 - MX) / 14.5).powi(2) + ((z as f32 - MZ) / 11.8).powi(2);
                if (0.8..=1.0).contains(&q) {
                    s.set(x, y, z, if y == 55 || y == 58 { GOLD_D } else { GOLD });
                }
            }
        }
    }
    // The cuirass: a narrow waist, a broad breast with the morning star on it.
    ball(&mut s, v(MX, 63.0, MZ), v(13.0, 8.5, 10.0), PLATE);
    ball(&mut s, v(MX, 74.0, MZ + 0.5), v(18.0, 12.0, 11.5), PLATE);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(MX + side * 7.0, 76.0, MZ + 6.0), v(8.5, 7.0, 6.0), PLATE_L);
    }
    s.paint(|x, y, z, c| {
        if c != PLATE {
            return None;
        }
        // The plates of the belly, each edged in gold.
        (y > 57 && y < 68 && (y - 58) % 4 == 3 && z as f32 > MZ + 4.0 && (x as f32 - MX).abs() < 11.0).then_some(GOLD_D)
    });
    let breast = front(&s, MX as i32, 76, d).unwrap_or(MZ as i32 + 12) as f32;
    star(&mut s, v(MX, 76.0, breast + 0.6), 7.0);
    // The gorget, gold, and a lining of the morning sky's blue showing at the collar.
    s.capsule((MX, 84.0, MZ + 1.0), (MX, 87.0, MZ + 1.0), 9.5, 8.5, GOLD);
    s.capsule((MX, 86.5, MZ + 1.0), (MX, 87.5, MZ + 1.0), 8.0, 8.0, LINING);
    // Behind, a back plate where the wings grow.
    ball(&mut s, v(MX, 78.0, MZ - 7.0), v(14.0, 9.0, 5.0), PLATE_D);

    // ---- arms: great pauldrons, plate to the gauntlet, gold at elbow and wrist
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(3 + i as u8);
        let sh = v(MX + side * 23.0, 85.0, MZ);
        let elbow = v(MX + side * 27.0, 65.0, MZ + 2.0);
        let wrist = v(MX + side * 27.0, 50.0, MZ + 10.0);
        limb(&mut s, &[(sh, 5.6), (elbow, 5.0)], PLATE);
        limb(&mut s, &[(elbow, 5.0), (wrist, 4.4)], PLATE);
        ball(&mut s, elbow + v(0.0, 0.0, -1.0), Vec3::splat(5.4), GOLD);
        s.capsule((wrist.x, wrist.y + 1.5, wrist.z - 0.5), (wrist.x, wrist.y - 0.5, wrist.z), 5.0, 5.0, GOLD_D);
        // The pauldron: two plates, layered, edged in gold, a gem at the crown of each.
        ball(&mut s, sh + v(side * 1.5, 0.5, 0.0), v(9.5, 7.5, 9.5), PLATE);
        ball(&mut s, sh + v(side * 3.5, -5.0, 0.0), v(8.0, 4.0, 8.5), PLATE_L);
        s.paint(|x, y, z, c| {
            let edge = (c == PLATE || c == PLATE_L) && (y == 79 || y == 80 || y == 76) && (x as f32 - sh.x).abs() < 12.0;
            (edge && (z as f32 - sh.z).abs() < 10.0 && (x as f32 - MX) * side > 14.0).then_some(GOLD)
        });
        ball(&mut s, sh + v(side * 3.0, 6.0, 4.0), Vec3::splat(1.5), GEM);
        let hand = wrist + v(0.0, -5.0, 2.0);
        ball(&mut s, hand, v(4.4, 5.0, 4.4), PLATE_D);
        if side > 0.0 {
            // His right: a fist round the hilt of a sword of light held point-down before him.
            let down = Vec3::new(0.0, -1.0, 0.45).normalize();
            limb(&mut s, &[(hand - down * 8.0, 1.6), (hand + down * 3.0, 1.6)], GOLD_D);
            ball(&mut s, hand - down * 8.5, Vec3::splat(2.2), GOLD);
            ball(&mut s, hand - down * 8.5 + v(0.0, 0.0, 1.5), Vec3::splat(0.9), GEM);
            let guard = hand + down * 4.5;
            s.capsule((guard.x - 7.5, guard.y + 1.0, guard.z), (guard.x + 7.5, guard.y + 1.0, guard.z), 1.5, 1.5, GOLD);
            for sx in [-1.0f32, 1.0] {
                ball(&mut s, guard + v(sx * 8.0, 1.8, 0.0), Vec3::splat(1.6), GOLD_L);
            }
            ball(&mut s, guard + v(0.0, 0.5, 1.2), Vec3::splat(1.5), GEM);
            // The blade: light, white at its heart and gold at its edges, tapering to a point.
            let (a, b) = (guard + down * 1.5, guard + down * 42.0);
            s.capsule((a.x, a.y, a.z), (b.x, b.y, b.z), 2.8, 0.7, LIGHT);
            s.capsule((a.x, a.y, a.z), (b.x, b.y, b.z), 1.2, 0.4, LIGHT_W);
        } else {
            // His left: an open hand, the fingers long.
            for (k, a) in [-0.8f32, -0.25, 0.3, 0.85].into_iter().enumerate() {
                let base = hand + v(a * 2.6, -3.5, 2.5);
                let tip = base + v(a * 1.5, -5.5 + (k as f32 - 1.5).abs() * 0.8, 2.0);
                limb(&mut s, &[(base, 1.4), (tip, 1.0)], PLATE_D);
            }
            limb(&mut s, &[(hand + v(-3.5, -1.0, 2.0), 1.4), (hand + v(-5.5, -4.0, 4.5), 1.0)], PLATE_D);
        }
    }

    // ---- the head: a face like a statue's, long pale-gold hair, a circlet and a crown of light
    s.part(5);
    s.capsule((MX, 85.0, MZ + 1.0), (MX, 93.0, MZ + 2.5), 4.6, 4.2, SKIN);
    // Hair first: the face is cut out of its front.
    ball(&mut s, v(MX, 100.5, MZ + 1.5), v(7.6, 8.6, 7.8), HAIR);
    limb(&mut s, &[(v(MX, 99.0, MZ - 2.0), 6.8), (v(MX, 90.0, MZ - 4.5), 7.4), (v(MX, 84.0, MZ - 6.0), 6.4)], HAIR);
    for side in [-1.0f32, 1.0] {
        limb(
            &mut s,
            &[(v(MX + side * 6.0, 98.0, MZ + 2.0), 2.6), (v(MX + side * 7.0, 88.0, MZ), 2.8), (v(MX + side * 8.0, 83.0, MZ - 1.0), 2.0)],
            HAIR,
        );
    }
    ball(&mut s, v(MX, 98.5, MZ + 4.0), v(6.0, 7.4, 6.2), SKIN);
    ball(&mut s, v(MX, 94.5, MZ + 5.5), v(4.6, 3.6, 4.8), SKIN);
    s.paint(|x, y, z, c| {
        if c != HAIR {
            return None;
        }
        Some(if (x + z / 2) % 3 == 0 || nz(x, y, z) > 0.85 { HAIR_D } else { HAIR })
    });
    let hd = d;
    // The brow, the straight nose, the eyes of light, the cheekbones and a stern mouth.
    for x in (MX - 4.0) as i32..=(MX + 4.0) as i32 {
        if let Some(z) = front(&s, x, 101, hd) {
            s.set(x, 101, z + 1, SKIN_D);
        }
    }
    for y in 96..=100 {
        if let Some(z) = front(&s, MX as i32, y, hd) {
            s.set(MX as i32, y, z + 1, if y == 96 { SKIN_D } else { SKIN });
        }
    }
    for side in [-1i32, 1] {
        for dx in 2..=3 {
            let x = MX as i32 + side * dx;
            if let Some(z) = front(&s, x, 99, hd) {
                s.recolor(x, 99, z, EYE);
            }
        }
        let x = MX as i32 + side * 4;
        if let Some(z) = front(&s, x, 97, hd) {
            s.set(x, 97, z + 1, SKIN);
        }
    }
    for x in (MX - 2.0) as i32..=(MX + 2.0) as i32 {
        if let Some(z) = front(&s, x, 94, hd) {
            s.recolor(x, 94, z, LIP);
        }
    }
    // The circlet, and the crown of light standing from it.
    for k in 0..48 {
        let a = k as f32 * TAU / 48.0;
        ball(&mut s, v(MX + a.cos() * 7.6, 104.0, MZ + 2.0 + a.sin() * 7.6), Vec3::splat(1.0), GOLD);
    }
    for k in 0..7 {
        let a = PI * 0.5 + (k as f32 - 3.0) * 0.36;
        let base = v(MX + a.cos() * 7.0, 105.0, MZ + 2.0 + a.sin() * 7.0);
        let tall = 15.0 - (k as f32 - 3.0).abs() * 2.6;
        let tip = base + v(a.cos() * 3.0, tall, a.sin() * 3.0);
        limb(&mut s, &[(base, 1.3), (tip, 0.4)], if k == 3 { LIGHT_W } else { LIGHT });
    }
    // The halo: a ring of gold light behind his head.
    for k in 0..90 {
        let a = k as f32 * TAU / 90.0;
        let c = v(MX + a.cos() * 12.5, 101.0 + a.sin() * 12.5, MZ - 7.0);
        ball(&mut s, c, Vec3::splat(1.1), if k % 2 == 0 { LIGHT } else { GOLD_L });
    }

    // ---- the six white wings, either side
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(6 + i as u8);
        wings(&mut s, side, false);
    }
    s
}

pub fn lucifer() -> Collected {
    let mut out = body().finish(0.03);
    // The burnt six, in parts of their own over the white (its own grid: they share its space).
    let (w, h, d) = LUCIFER_SIZE;
    let mut burnt = Sculpt::new(w, h, d);
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        burnt.part(8 + i as u8);
        wings(&mut burnt, side, true);
    }
    out.voxels.extend(burnt.finish(0.05).voxels);
    out
}

/// A colour turned to the Reflections' cold silver: its brightness kept, its hue lost to a cold
/// blue-white, polished as a mirror's back; light stays light, cold.
fn silver(c: u32) -> u32 {
    let m = c & 0xff00_0000;
    let (r, g, b) = (((c >> 16) & 255) as f32, ((c >> 8) & 255) as f32, (c & 255) as f32);
    let l = (0.3 * r + 0.55 * g + 0.15 * b) / 255.0;
    let (k, out_m) = if m == mat::GLOW { (1.0, mat::GLOW) } else { (0.55 + 0.45 * l, mat::GOLD) };
    let ch = |base: f32| ((base * k).clamp(0.0, 255.0)) as u32;
    (ch(206.0) << 16) | (ch(222.0) << 8) | ch(255.0) | out_m
}

pub fn reflection() -> Collected {
    let src = body().finish(0.02);
    let swap = |p: u8| match p {
        1 => 2,
        2 => 1,
        3 => 4,
        4 => 3,
        6 => 7,
        7 => 6,
        p => p,
    };
    let voxels = src
        .voxels
        .iter()
        .map(|vx| Voxel { x: (2.0 * MX) as i32 - vx.x, y: vx.y, z: vx.z, c: silver(vx.c), part: swap(vx.part) })
        .filter(|vx| vx.x >= 0 && vx.x < src.width)
        .collect();
    Collected { voxels, ..src }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lucifer_towers_over_the_hero_on_six_wings_and_his_reflection_is_his_mirror_image() {
        let m = lucifer();
        let top = m.voxels.iter().map(|v| v.y).max().unwrap() as f32 * LUCIFER_VOXEL;
        assert!(top > 2.7 && top < 3.2, "{top} tiles tall");
        assert!(m.voxels.len() > 12000, "{} voxels", m.voxels.len());
        for part in 0..=LUCIFER_RIG.len() as u8 {
            assert!(m.voxels.iter().any(|v| v.part == part), "part {part}");
        }
        let glow = |c: u32| c & 0xff00_0000 == mat::GLOW;
        assert!(m.voxels.iter().filter(|v| v.part == 4 && glow(v.c)).count() > 100, "a sword of light in his right hand");
        assert!(m.voxels.iter().filter(|v| v.part == 5 && glow(v.c)).count() > 100, "a crown of light and a halo");
        // His wings spread wider than his shoulders; the burnt six lie where the white do.
        let span = |p: u8| m.voxels.iter().filter(|v| v.part == p).map(|v| v.x).fold((i32::MAX, i32::MIN), |a, x| (a.0.min(x), a.1.max(x)));
        assert!(span(6).0 < 10 && span(7).1 > 90, "wings {:?} {:?}", span(6), span(7));
        let (white, burnt) = (span(7), span(9));
        assert!((white.0 - burnt.0).abs() <= 3 && (white.1 - burnt.1).abs() <= 3);
        assert!(m.voxels.iter().filter(|v| v.part == 9).all(|v| !matches!(v.c & 0xffffff, 0xfaf8f2 | 0xe8e4da)), "black, not white");
        // The Reflection: as in a mirror (the sword in its left hand), silver, an add's height.
        let r = reflection();
        let top = r.voxels.iter().map(|v| v.y).max().unwrap() as f32 * REFLECTION_VOXEL;
        assert!(top > 1.8 && top < 2.2, "{top}");
        assert!(r.voxels.iter().filter(|v| v.part == 3 && glow(v.c)).count() > 100, "its sword in its left hand");
        assert!(r.voxels.iter().all(|v| v.part <= 7), "no burnt wings");
        assert!(
            r.voxels.iter().filter(|v| !glow(v.c)).all(|v| v.c & 0xff00_0000 == mat::GOLD && (v.c & 255) >= (v.c >> 16) & 255),
            "polished silver"
        );
    }
}
