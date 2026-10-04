//! Gluttony's court's own (PRINCES.md §5): Beelzebub, the prince of Gluttony, and the swarms of
//! flies that rise from his food. Each faces +z, stands in the middle of its grid and is sculpted
//! in the parts its rig swings.
//!
//! Beelzebub is a bloated lord two and a half times the hero's height with the head of a fly: a
//! rotting crimson robe trimmed in tarnished gold hangs open over a vast belly of sickly, veined
//! skin; a stained napkin is tied under his chin; a ruff of black bristles stands about his neck.
//! His head is a fly's: two great red compound eyes, faceted, bulging either side of a glossy black
//! face, a fleshy proboscis drooping over the napkin, short knobbed feelers, bristles, and a
//! tarnished gold crown set with rubies between the eyes. His forearms are a fly's legs, black and
//! bristled, ending in hooked claws: in his right a butcher's cleaver, notched and bloodied; in his
//! left a gnawed leg of fowl. Two great veined wings, pale and glassy, stand from his back, and
//! hooked black insect feet show under the ragged hem.
//!
//! A swarm is a knot of flies: a few big ones at its heart, a cloud of small ones about them,
//! drifting in four clouds its rig turns.

use std::f32::consts::{PI, TAU};

use glam::Vec3;

use crate::render::carve::{ball, limb, nz, v};
use crate::render::characters::{mat, Sculpt};
use crate::render::voxels::{Collected, PartDef};

mod pal {
    use super::mat;
    pub const SKIN: u32 = 0x8e8a52 | mat::SKIN;
    pub const SKIN_L: u32 = 0xa6a066 | mat::SKIN;
    pub const SKIN_D: u32 = 0x6a643a | mat::SKIN;
    pub const VEIN: u32 = 0x5e6e3a | mat::SKIN;
    pub const BRUISE: u32 = 0x6e5a5a | mat::SKIN;
    pub const ROBE: u32 = 0x5c2420 | mat::CLOTH;
    pub const ROBE_L: u32 = 0x76322a | mat::CLOTH;
    pub const ROBE_D: u32 = 0x3e1614 | mat::CLOTH;
    pub const GREASE: u32 = 0x4a3418 | mat::CLOTH;
    pub const WINE: u32 = 0x2c0a10 | mat::CLOTH;
    pub const TRIM: u32 = 0xa88a3a | mat::GOLD;
    pub const TRIM_D: u32 = 0x6e5a24 | mat::GOLD;
    pub const GOLD: u32 = 0xc8a040 | mat::GOLD;
    pub const GOLD_L: u32 = 0xe8c860 | mat::GOLD;
    pub const RUBY: u32 = 0xa01828 | mat::GOLD;
    pub const EMERALD: u32 = 0x2a7a3a | mat::GOLD;
    pub const NAPKIN: u32 = 0xc4bca4 | mat::CLOTH;
    pub const NAPKIN_D: u32 = 0xb0a688 | mat::CLOTH;
    pub const NAPKIN_STAIN: u32 = 0x8a6a3a | mat::CLOTH;
    pub const CHITIN: u32 = 0x1c1e16 | mat::SLIME;
    pub const CHITIN_L: u32 = 0x34402a | mat::SLIME;
    pub const CHITIN_SHEEN: u32 = 0x3a5a4a | mat::SLIME;
    pub const BRISTLE: u32 = 0x0e0e0a | mat::FUR;
    pub const EYE: u32 = 0x8e1410 | mat::SLIME;
    pub const EYE_L: u32 = 0xc42c1c | mat::SLIME;
    pub const EYE_D: u32 = 0x5a0c0c | mat::SLIME;
    pub const EYE_GLINT: u32 = 0xff5a30 | mat::GLOW;
    pub const PROBOSCIS: u32 = 0x4a3426 | mat::SLIME;
    pub const SPONGE: u32 = 0x8a6a42 | mat::SLIME;
    pub const CLAW: u32 = 0x2a2016 | mat::IVORY;
    pub const STEEL: u32 = 0x8a8e94 | mat::METAL;
    pub const STEEL_L: u32 = 0xc8ccd0 | mat::METAL;
    pub const RUST: u32 = 0x6a3a22 | mat::METAL;
    pub const BLOOD: u32 = 0x5a0a0e | mat::SLIME;
    pub const HANDLE: u32 = 0x3a2414 | mat::WOOD;
    pub const MEAT: u32 = 0x8a4a22 | mat::SLIME;
    pub const MEAT_L: u32 = 0xb06a32 | mat::SLIME;
    pub const BONE: u32 = 0xe0d6c0 | mat::IVORY;
    pub const WING: u32 = 0xb8bca6 | mat::SLIME;
    pub const WING_L: u32 = 0xccd4c8 | mat::SLIME;
    pub const WING_B: u32 = 0xa0aab8 | mat::SLIME;
    pub const WING_VEIN: u32 = 0x2e2a1a | mat::SLIME;
    pub const FLY: u32 = 0x161410 | mat::SLIME;
    pub const FLY_SHEEN: u32 = 0x2e4a2a | mat::SLIME;
    pub const FLY_EYE: u32 = 0x9a1a14 | mat::SLIME;
}
use pal::*;

/// Beelzebub's grid and voxel: about three tiles tall, Mammon's measure.
pub const BEELZEBUB_SIZE: (i32, i32, i32) = (100, 124, 80);
pub const BEELZEBUB_VOXEL: f32 = 0.025;
const MX: f32 = 50.0;
const MZ: f32 = 40.0;

/// Hips, shoulders and neck, and the roots of his wings. The cleaver and the drumstick go with
/// his arms, the napkin and the crown with his body and head.
pub const BEELZEBUB_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [MX - 12.0, 34.0, MZ] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [MX + 12.0, 34.0, MZ] },
    PartDef { name: "leftArm", boxes: &[], pivot: [MX - 31.0, 82.0, MZ] },
    PartDef { name: "rightArm", boxes: &[], pivot: [MX + 31.0, 82.0, MZ] },
    PartDef { name: "head", boxes: &[], pivot: [MX, 90.0, MZ + 4.0] },
    PartDef { name: "leftWing", boxes: &[], pivot: [MX - 9.0, 88.0, MZ - 14.0] },
    PartDef { name: "rightWing", boxes: &[], pivot: [MX + 9.0, 88.0, MZ - 14.0] },
];

/// The robe's half-widths (x across, z deep) at height `y`: a bell to the floor over his bulk.
fn robe(y: f32) -> (f32, f32) {
    let k = ((60.0 - y) / 54.0).clamp(0.0, 1.0);
    (27.0 + k.powf(1.2) * 8.0, 24.0 + k.powf(1.2) * 6.0)
}

/// A fly's leg for a forearm: a black, bristled limb along `knots`, its joints knobbed.
fn bristled(s: &mut Sculpt, knots: &[(Vec3, f32)]) {
    limb(s, knots, CHITIN);
    for w in knots.windows(2) {
        let (a, b) = (w[0].0, w[1].0);
        let n = ((b - a).length() / 2.5) as i32;
        for k in 0..n {
            let t = k as f32 / n.max(1) as f32;
            let p = a.lerp(b, t);
            let r = w[0].1 + (w[1].1 - w[0].1) * t;
            let ang = k as f32 * 2.4;
            let out = Vec3::new(ang.cos(), (k % 3) as f32 * 0.3 - 0.3, ang.sin());
            limb(s, &[(p + out * r, 0.6), (p + out * (r + 2.5) + Vec3::new(0.0, -0.8, 0.0), 0.4)], BRISTLE);
        }
    }
    for &(p, r) in &knots[1..knots.len() - 1] {
        ball(s, p, Vec3::splat(r + 0.8), CHITIN_L);
    }
}

/// A wing from `root`: a long glassy oval along `along`, its face across `across`, `len` long and
/// `wide` across at its widest, veined dark, its cells catching the light pale and bluish.
fn wing(s: &mut Sculpt, root: Vec3, along: Vec3, across: Vec3, len: f32, wide: f32) {
    let along = along.normalize();
    let across = (across - along * across.dot(along)).normalize();
    let normal = along.cross(across).normalize();
    let steps_u = (len * 2.0) as i32;
    let steps_v = (wide * 2.0) as i32;
    for i in 0..=steps_u {
        let u = i as f32 * 0.5;
        let t = u / len;
        // Narrow at the root, broadest past the middle, rounded at the tip.
        let half = wide * (t * (1.0 - t)).max(0.0).sqrt() * 2.0 * (0.55 + 0.45 * t.min(0.7) / 0.7);
        for j in -steps_v..=steps_v {
            let w = j as f32 * 0.5;
            if w.abs() > half {
                continue;
            }
            let p = root + along * u + across * (w - t * t * wide * 0.25);
            let vein = (w.abs() < 0.6)
                || ((w - half * 0.55).abs() < 0.45)
                || ((w + half * 0.5).abs() < 0.45)
                || (u % 9.0 < 0.6 && u > 4.0)
                || (half - w.abs() < 0.6);
            let cell = ((u / 9.0) as i32 + (w / 4.0).floor() as i32).rem_euclid(3);
            let c = if vein { WING_VEIN } else { [WING, WING_L, WING_B][cell as usize] };
            for k in 0..2 {
                let q = p + normal * k as f32 * 0.7;
                s.set(q.x.round() as i32, q.y.round() as i32, q.z.round() as i32, c);
            }
        }
    }
}

/// One of his compound eyes about `c`: a great red dome, faceted (each facet a little lighter at
/// its top, darker at its rim), a glint burning in it.
fn compound_eye(s: &mut Sculpt, c: Vec3, r: Vec3) {
    ball(s, c, r, EYE);
    let lo = c - r - Vec3::ONE;
    let hi = c + r + Vec3::ONE;
    for y in lo.y as i32..=hi.y as i32 {
        for z in lo.z as i32..=hi.z as i32 {
            for x in lo.x as i32..=hi.x as i32 {
                if s.get(x, y, z) != Some(EYE) {
                    continue;
                }
                // Facets in a staggered grid over the dome.
                let row = y.div_euclid(2);
                let col = (x + z + row).div_euclid(2);
                let rim = y.rem_euclid(2) == 0 && (x + z + row).rem_euclid(2) == 0;
                let c = if rim {
                    EYE_D
                } else if (row + col).rem_euclid(3) == 0 {
                    EYE_L
                } else {
                    EYE
                };
                s.recolor(x, y, z, c);
            }
        }
    }
}

pub fn beelzebub() -> Collected {
    let (w, h, d) = BEELZEBUB_SIZE;
    let mut s = Sculpt::new(w, h, d);

    // ---- legs under the robe, and a fly's hooked feet at the hem
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        let x = MX + side * 12.0;
        s.capsule((x, 34.0, MZ), (x, 9.0, MZ + 3.0), 8.5, 6.5, CHITIN);
        ball(&mut s, v(x, 6.0, MZ + 6.0), v(6.5, 4.5, 8.0), CHITIN_L);
        // Two hooked toes forward, a spur back.
        for t in [-2.5f32, 2.5] {
            limb(
                &mut s,
                &[(v(x + t, 4.0, MZ + 11.0), 2.0), (v(x + t * 1.4, 2.0, MZ + 18.0), 1.4), (v(x + t * 1.6, 0.5, MZ + 21.0), 0.8)],
                CHITIN,
            );
            ball(&mut s, v(x + t * 1.6, 0.8, MZ + 22.0), Vec3::splat(1.0), CLAW);
        }
        limb(&mut s, &[(v(x, 4.0, MZ + 1.0), 1.6), (v(x, 1.0, MZ - 5.0), 0.8)], CHITIN);
    }

    // ---- the belly: vast, pale, veined; the chest above it
    s.part(0);
    ball(&mut s, v(MX, 49.0, MZ + 9.0), v(28.0, 27.0, 27.0), SKIN);
    ball(&mut s, v(MX, 72.0, MZ + 4.0), v(23.0, 14.0, 20.0), SKIN);
    s.paint(|x, y, z, c| {
        if c != SKIN {
            return None;
        }
        let n = nz(x, y, z);
        // Veins wandering over the swell, bruised patches, light on its crown.
        let vein = ((x as f32 * 0.31 + (y as f32 * 0.17).sin() * 3.0).sin() * (y as f32 * 0.23 + (x as f32 * 0.11).cos() * 2.0).cos())
            .abs()
            < 0.06;
        Some(if vein {
            VEIN
        } else if n < 0.04 {
            BRUISE
        } else if y > 60 && n < 0.35 {
            SKIN_L
        } else if n > 0.9 || y < 30 {
            SKIN_D
        } else {
            SKIN
        })
    });
    // The navel, deep.
    for k in 0..4 {
        let z = (MZ + 35.0) as i32 - k;
        for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            s.clear(MX as i32 + dx, 47 + dy, z);
        }
    }
    for (dx, dy) in [(-1, 0), (2, 0), (0, -1), (1, 2), (-1, 1), (2, 1)] {
        let x = MX as i32 + dx;
        let y = 47 + dy;
        if let Some(z) = (0..d).rev().find(|&z| s.get(x, y, z).is_some()) {
            s.recolor(x, y, z, SKIN_D);
        }
    }

    // ---- the robe: crimson gone to rot, a bell to the floor, open down the front over the belly
    for y in 6..=80 {
        let yf = y as f32;
        let (rx, rz) = robe(yf);
        let (rx, rz) = if yf > 58.0 {
            let t = ((yf - 58.0) / 22.0).clamp(0.0, 1.0);
            (rx + 5.0 * (t * PI).sin() + t * 3.0, rz - t * 5.0)
        } else {
            (rx, rz)
        };
        for z in 0..d {
            for x in 0..w {
                let (xf, zf) = (x as f32, z as f32);
                let (dx, dz) = ((xf - MX) / rx, (zf - MZ) / rz);
                let q = dx * dx + dz * dz;
                if q > 1.0 || q < 0.72 {
                    continue;
                }
                let a = dz.atan2(dx);
                // The ragged hem: torn higher here and there.
                let rag = ((a * 7.0).sin() * 2.0 + (a * 17.0).cos() * 1.5).max(0.0);
                if yf < 6.0 + rag {
                    continue;
                }
                // The opening down the front, wide over the belly.
                let open = (xf - MX).abs() < 12.0 + ((yf - 24.0) / 26.0).clamp(0.0, 1.0) * 10.0 && dz > 0.45 && y > 14;
                if open {
                    continue;
                }
                let fold = (a * 10.0 + (yf * 0.08).sin()).sin();
                let n = nz(x, y, z);
                let c = if y <= 9 + rag as i32 {
                    TRIM_D
                } else if n < 0.05 {
                    GREASE
                } else if n > 0.97 {
                    WINE
                } else if fold > 0.55 {
                    ROBE_L
                } else if fold < -0.55 {
                    ROBE_D
                } else {
                    ROBE
                };
                s.set(x, y, z, c);
            }
        }
    }
    // Under the robe's opening, below the belly: his under-robe, dark and greasy, to the floor.
    for y in 6..=30 {
        let (rx, rz) = robe(y as f32);
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = ((x as f32 - MX) / (rx * 0.74), (z as f32 - MZ) / (rz * 0.74));
                if dx * dx + dz * dz <= 1.0 && s.get(x, y, z).is_none() {
                    s.set(
                        x,
                        y,
                        z,
                        if nz(x, y, z) < 0.08 {
                            GREASE
                        } else if (x + z) % 7 == 0 {
                            ROBE
                        } else {
                            ROBE_D
                        },
                    );
                }
            }
        }
    }
    // Tarnished gold down both edges of the opening.
    for y in 14..=80 {
        let yf = y as f32;
        let half = 12.0 + ((yf - 24.0) / 26.0).clamp(0.0, 1.0) * 10.0;
        for z in (MZ as i32)..d {
            for x in 0..w {
                if !matches!(s.get(x, y, z), Some(ROBE | ROBE_L | ROBE_D | GREASE | WINE)) {
                    continue;
                }
                let off = (x as f32 - MX).abs();
                if off >= half - 0.5 && off <= half + 2.5 && z as f32 > MZ + 10.0 {
                    s.recolor(x, y, z, if off > half + 1.5 { TRIM_D } else { TRIM });
                }
            }
        }
    }
    // A sash of grubby gold cloth under the belly, knotted, its ends hanging.
    for y in 24..=28 {
        for z in 0..d {
            for x in 0..w {
                let (xf, zf) = (x as f32, z as f32);
                let (rx, rz) = robe(y as f32);
                let q = ((xf - MX) / (rx + 1.0)).powi(2) + ((zf - MZ) / (rz + 1.0)).powi(2);
                let under = ((xf - MX) / 26.0).powi(2) + ((y as f32 - 49.0) / 27.0).powi(2) + ((zf - MZ - 9.0) / 27.0).powi(2);
                if (0.86..=1.0).contains(&q) && under > 0.98 {
                    s.set(x, y, z, if (x + y) % 4 == 0 { TRIM_D } else { TRIM });
                }
            }
        }
    }

    // ---- shoulders, the ruff of bristles, and the napkin tied under his chin
    ball(&mut s, v(MX, 82.0, MZ), v(31.0, 9.0, 20.0), ROBE);
    for k in 0..90 {
        let a = k as f32 / 90.0 * TAU;
        let r = 15.0 + (k % 3) as f32;
        let base = v(MX + a.cos() * r, 86.0, MZ + 3.0 + a.sin() * r * 0.8);
        let out = Vec3::new(a.cos(), 0.6 + (k % 4) as f32 * 0.15, a.sin() * 0.8);
        limb(&mut s, &[(base, 1.2), (base + out * 5.0, 0.5)], BRISTLE);
    }
    // The napkin: a stained cloth hanging over his chest, its knot at his throat.
    for y in 64..=88 {
        let t = (88 - y) as f32 / 24.0;
        let half = 7.0 + t * 11.0;
        for x in (MX - half) as i32..=(MX + half) as i32 {
            let xf = x as f32;
            // On the front of the chest and the belly's top, a voxel proud of it.
            let Some(z) = (MZ as i32..d).rev().find(|&z| s.get(x, y, z).is_some()) else { continue };
            let point = (xf - MX).abs() > half - (1.0 - t) * 2.0 - t * t * 6.0;
            if y < 66 && point {
                continue;
            }
            let n = nz(x, y, z);
            let c = if n < 0.12 {
                NAPKIN_STAIN
            } else if (x + y) % 5 == 0 {
                NAPKIN_D
            } else {
                NAPKIN
            };
            s.set(x, y, z + 1, c);
        }
    }
    ball(&mut s, v(MX, 88.0, MZ + 17.0), v(4.0, 3.0, 3.0), NAPKIN_D);
    for side in [-1.0f32, 1.0] {
        limb(&mut s, &[(v(MX + side * 3.0, 88.0, MZ + 18.0), 1.6), (v(MX + side * 6.0, 84.0, MZ + 20.0), 1.2)], NAPKIN);
    }
    // A stud of tarnished gold at each shoulder, holding the robe.
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(MX + side * 20.0, 86.0, MZ + 14.0), Vec3::splat(3.0), GOLD);
        ball(&mut s, v(MX + side * 20.0, 86.0, MZ + 16.5), Vec3::splat(1.3), RUBY);
    }

    // ---- arms: wide sleeves to the elbow, a fly's leg below it, hooked claws
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(3 + i as u8);
        let sh = v(MX + side * 31.0, 82.0, MZ);
        let elbow = v(MX + side * 38.0, 62.0, MZ + 7.0);
        let wrist = v(MX + side * 36.0, 47.0, MZ + 18.0);
        ball(&mut s, sh, v(10.0, 10.0, 10.0), ROBE);
        limb(&mut s, &[(sh, 9.5), (elbow, 11.0)], ROBE);
        s.capsule((elbow.x, elbow.y - 1.0, elbow.z), (elbow.x, elbow.y - 4.0, elbow.z + 1.0), 11.5, 12.0, ROBE_D);
        s.capsule((elbow.x, elbow.y - 4.0, elbow.z + 1.0), (elbow.x, elbow.y - 5.0, elbow.z + 1.0), 12.2, 12.2, TRIM);
        bristled(&mut s, &[(elbow, 4.5), (elbow.lerp(wrist, 0.5) + v(side * 1.0, 0.0, 0.0), 3.6), (wrist, 3.2)]);
        let hand = wrist + v(0.0, -4.0, 3.0);
        ball(&mut s, hand, v(4.5, 4.5, 4.5), CHITIN_L);
        // Three hooked claws closed round what it holds.
        for (k, a) in [-0.8f32, 0.0, 0.8].into_iter().enumerate() {
            let base = hand + v(a * 3.0, -2.0, 3.5);
            let tip = base + v(a * 1.5, -3.0 - k as f32 * 0.4, 3.0);
            limb(&mut s, &[(base, 1.4), (tip, 1.0)], CHITIN);
            limb(&mut s, &[(tip, 0.9), (tip + v(-side * 1.5, -1.5, -1.0), 0.4)], CLAW);
        }
        if side > 0.0 {
            // His right: the butcher's cleaver, its handle in his claws, its blade forward and down.
            let grip = hand + v(0.0, -1.0, 4.0);
            limb(&mut s, &[(grip + v(0.0, 0.0, -4.0), 1.6), (grip + v(0.0, 0.0, 7.0), 1.8)], HANDLE);
            for k in [-2.0f32, 1.0, 4.0] {
                ball(&mut s, grip + v(0.0, 0.0, k), Vec3::splat(1.9), TRIM_D);
            }
            let (z0, z1) = ((grip.z + 7.0) as i32, (grip.z + 27.0) as i32);
            let (y0, y1) = ((grip.y - 15.0) as i32, (grip.y + 2.0) as i32);
            for z in z0..=z1 {
                for y in y0..=y1 {
                    // The blade, its edge slanting a little, a hole through its heel.
                    let zf = (z - z0) as f32;
                    let edge = y0 as f32 + (20.0 - zf) * 0.12;
                    if (y as f32) < edge {
                        continue;
                    }
                    let hole = (zf - 3.0).powi(2) + (y as f32 - grip.y + 1.0).powi(2) < 3.0;
                    if hole {
                        continue;
                    }
                    let n = nz(grip.x as i32, y, z);
                    let c = if (y as f32) < edge + 1.6 {
                        if n < 0.35 {
                            BLOOD
                        } else {
                            STEEL_L
                        }
                    } else if y == y1 {
                        STEEL_L
                    } else if n < 0.1 {
                        RUST
                    } else if n > 0.92 {
                        BLOOD
                    } else {
                        STEEL
                    };
                    for dx in -1..=0 {
                        s.set(grip.x as i32 + dx, y, z, c);
                    }
                }
            }
        } else {
            // His left: a gnawed leg of fowl, the meat torn, the bone bare at its knuckle.
            let at = hand + v(0.0, 2.0, 5.0);
            ball(&mut s, at + v(0.0, 6.0, 2.0), v(5.0, 6.5, 5.0), MEAT);
            ball(&mut s, at + v(1.5, 8.0, 4.0), v(2.5, 3.0, 2.5), MEAT_L);
            limb(&mut s, &[(at + v(0.0, 1.0, 1.0), 1.4), (at + v(0.0, -5.0, -1.0), 1.2)], BONE);
            ball(&mut s, at + v(0.0, -6.0, -1.0), v(2.0, 1.6, 2.0), BONE);
            // Where he has bitten it: the bone shows.
            for k in 0..5 {
                s.set(at.x as i32 + 4, (at.y + 6.0) as i32 + k - 2, (at.z + 3.0) as i32, BONE);
            }
        }
    }

    // ---- the head: a fly's, glossy black, the great red eyes, the proboscis, the crown
    s.part(5);
    let hc = v(MX, 101.0, MZ + 7.0);
    ball(&mut s, v(MX, 91.0, MZ + 5.0), v(9.0, 4.0, 8.0), CHITIN);
    ball(&mut s, hc, v(12.0, 11.0, 11.0), CHITIN);
    // The face between the eyes: a plate of chitin with a sheen, ridged down its middle.
    ball(&mut s, hc + v(0.0, -2.0, 7.0), v(6.0, 8.0, 5.0), CHITIN_L);
    s.paint(|x, y, z, c| {
        if c != CHITIN_L || y < 90 {
            return None;
        }
        Some(if (x as f32 - MX).abs() < 1.0 {
            CHITIN
        } else if nz(x, y, z) < 0.25 {
            CHITIN_SHEEN
        } else {
            CHITIN_L
        })
    });
    for side in [-1.0f32, 1.0] {
        compound_eye(&mut s, hc + v(side * 9.5, 2.0, 3.0), v(8.0, 9.5, 8.5));
        // A glint burning in each.
        let gx = (MX + side * 11.0) as i32;
        for (dx, dy) in [(0, 0), (1, 0), (0, 1)] {
            let (x, y) = (gx + dx * side as i32, 106 + dy);
            if let Some(z) = (0..d).rev().find(|&z| s.get(x, y, z).is_some()) {
                s.recolor(x, y, z, EYE_GLINT);
            }
        }
    }
    // The proboscis, drooping over the napkin, its sponge of a tip wet.
    limb(&mut s, &[(hc + v(0.0, -6.0, 9.0), 3.0), (hc + v(0.0, -11.0, 12.5), 2.4), (hc + v(0.0, -16.0, 13.5), 2.0)], PROBOSCIS);
    ball(&mut s, hc + v(0.0, -18.0, 14.0), v(3.5, 2.2, 3.0), SPONGE);
    for side in [-1.0f32, 1.0] {
        // Palps either side of it.
        limb(&mut s, &[(hc + v(side * 3.0, -6.0, 8.0), 1.2), (hc + v(side * 5.0, -10.0, 10.0), 0.8)], CHITIN);
        // Short feelers, knobbed.
        let root = hc + v(side * 2.5, 6.0, 9.0);
        limb(&mut s, &[(root, 1.0), (root + v(side * 2.0, 3.0, 2.5), 0.8), (root + v(side * 4.5, 4.0, 4.5), 0.7)], CHITIN);
        ball(&mut s, root + v(side * 5.0, 4.2, 5.0), Vec3::splat(1.4), CHITIN_L);
    }
    // Bristles over the crown of his head and down its back.
    for k in 0..40 {
        let a = k as f32 * 2.4;
        let up = 0.2 + (k % 5) as f32 * 0.15;
        let dir = Vec3::new(a.cos() * 0.7, up + 0.4, -0.5 + a.sin() * 0.4).normalize();
        let base = hc + dir * 10.5;
        if base.z > hc.z + 4.0 {
            continue;
        }
        limb(&mut s, &[(base, 0.8), (base + dir * 4.0, 0.4)], BRISTLE);
    }
    // The crown: a band of tarnished gold between and over the eyes, its points set with rubies.
    for k in 0..48 {
        let a = k as f32 / 48.0 * TAU;
        let c = hc + v(a.cos() * 7.0, 9.0, a.sin() * 6.5 - 1.0);
        ball(&mut s, c, v(1.3, 1.6, 1.3), if k % 2 == 0 { GOLD } else { TRIM });
    }
    for k in 0..7 {
        let a = k as f32 / 7.0 * TAU + 0.2;
        let base = hc + v(a.cos() * 7.0, 10.0, a.sin() * 6.5 - 1.0);
        let tall = 6.0 + if a.sin() > 0.3 { 3.0 } else { 0.0 };
        limb(&mut s, &[(base, 1.4), (base + v(0.0, tall, 0.0), 0.6)], GOLD);
        ball(&mut s, base + v(0.0, tall, 0.0), Vec3::splat(1.0), GOLD_L);
        ball(&mut s, base + v(a.cos() * 1.0, 1.2, a.sin() * 1.0), Vec3::splat(1.1), if k % 3 == 1 { EMERALD } else { RUBY });
    }

    // ---- the wings: two great glassy veined wings from his back, up and out and back
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(6 + i as u8);
        let root = v(MX + side * 9.0, 88.0, MZ - 14.0);
        ball(&mut s, root, Vec3::splat(3.0), CHITIN);
        wing(&mut s, root, Vec3::new(side * 0.86, 0.46, -0.22), Vec3::new(0.0, 1.0, -0.3), 46.0, 15.0);
    }

    s.finish(0.03)
}

/// The swarm's grid and voxel: a cloud about a tile and a half tall and a tile across.
pub const FLY_SWARM_SIZE: (i32, i32, i32) = (44, 64, 44);
pub const FLY_SWARM_VOXEL: f32 = 0.025;
const SX: f32 = 22.0;
const SZ: f32 = 22.0;

/// Four clouds of flies the rig turns about the swarm's heart (the walk does not move them).
pub const FLY_SWARM_RIG: &[PartDef] = &[
    PartDef { name: "cloud1", boxes: &[], pivot: [SX, 36.0, SZ] },
    PartDef { name: "cloud2", boxes: &[], pivot: [SX, 36.0, SZ] },
    PartDef { name: "cloud3", boxes: &[], pivot: [SX, 36.0, SZ] },
    PartDef { name: "cloud4", boxes: &[], pivot: [SX, 36.0, SZ] },
];

/// A big fly at `at` heading `yaw`, `k` voxels to its body's span: head and red eyes, thorax,
/// banded abdomen, and two pale wings.
fn big_fly(s: &mut Sculpt, at: Vec3, yaw: f32, k: f32) {
    let fwd = Vec3::new(yaw.sin(), 0.0, yaw.cos());
    let side = Vec3::new(fwd.z, 0.0, -fwd.x);
    ball(s, at, Vec3::splat(1.6 * k), FLY);
    ball(s, at - fwd * 2.6 * k, v(1.5 * k, 1.3 * k, 1.5 * k), FLY_SHEEN);
    ball(s, at - fwd * 3.6 * k, v(1.3 * k, 1.1 * k, 1.3 * k), FLY);
    let head = at + fwd * 1.8 * k;
    ball(s, head, Vec3::splat(1.1 * k), FLY);
    for sd in [-1.0f32, 1.0] {
        ball(s, head + side * sd * 0.8 * k + fwd * 0.3 * k, Vec3::splat(0.7 * k), FLY_EYE);
        // A wing laid back over its body.
        let root = at + Vec3::new(0.0, 0.9 * k, 0.0) + side * sd * 0.6 * k;
        for u in 0..(4.0 * k) as i32 {
            let p = root - fwd * u as f32 + side * sd * (u as f32 * 0.35 + 0.5);
            s.set(p.x.round() as i32, p.y.round() as i32, p.z.round() as i32, if u % 3 == 0 { WING_VEIN } else { WING_L });
        }
    }
}

pub fn fly_swarm() -> Collected {
    let (w, h, d) = FLY_SWARM_SIZE;
    let mut s = Sculpt::new(w, h, d);
    // Its heart: three big flies about a dark knot.
    s.part(0);
    ball(&mut s, v(SX, 36.0, SZ), v(3.0, 4.0, 3.0), FLY);
    for (k, (dy, a)) in [(0.0f32, 0.0f32), (5.0, 2.1), (-4.0, 4.2)].into_iter().enumerate() {
        let at = v(SX + (a * 1.3).cos() * 3.5, 36.0 + dy, SZ + (a * 1.3).sin() * 3.5);
        big_fly(&mut s, at, a + 1.2 + k as f32, 1.6);
    }
    // The clouds: small flies scattered through a teardrop about the heart, each cloud its own part.
    for part in 1..=4u8 {
        s.part(part);
        let turn = part as f32 * TAU / 4.0;
        for k in 0..70 {
            let kf = k as f32 + part as f32 * 101.0;
            let a = turn + (nz(k, part as i32, 7) - 0.5) * 2.4;
            let y = 14.0 + nz(k, part as i32, 3) * 44.0;
            // Wider at the middle height, thinning above and below.
            let swell = 1.0 - ((y - 36.0) / 24.0).powi(2);
            let r = (4.0 + nz(k, part as i32, 5) * 13.0) * swell.max(0.25);
            let p = v(SX + a.cos() * r, y, SZ + a.sin() * r);
            if kf as i32 % 9 == 0 {
                big_fly(&mut s, p, a + kf, 0.9);
            } else {
                let c = if k % 4 == 0 { FLY_SHEEN } else { FLY };
                s.set(p.x as i32, p.y as i32, p.z as i32, c);
                s.set(p.x as i32 + 1, p.y as i32, p.z as i32, c);
                if k % 3 == 0 {
                    s.set(p.x as i32, p.y as i32 + 1, p.z as i32, WING_L);
                }
            }
        }
    }
    s.finish(0.04)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beelzebub_towers_over_the_hero_with_a_flys_head_and_his_swarms_are_clouds() {
        let b = beelzebub();
        let top = b.voxels.iter().map(|v| v.y).max().unwrap() as f32 * BEELZEBUB_VOXEL;
        assert!(top > 2.7 && top < 3.4, "{top} tiles tall");
        assert!(b.voxels.len() > 8000, "{} voxels", b.voxels.len());
        for part in 0..=BEELZEBUB_RIG.len() as u8 {
            assert!(b.voxels.iter().any(|v| v.part == part), "part {part}");
        }
        let rgb = |c: u32| (((c >> 16) & 255) as i32, ((c >> 8) & 255) as i32, (c & 255) as i32);
        let count = |f: &dyn Fn((i32, i32, i32)) -> bool| b.voxels.iter().filter(|v| f(rgb(v.c))).count();
        assert!(count(&|(r, g, bl)| r > 0x50 && g < 0x40 && bl < 0x40) > 300, "his great red eyes");
        assert!(count(&|(r, g, bl)| r > 0x70 && (r - g).abs() < 12 && (g - bl).abs() < 14 && bl > 0x70 && r < 0xb0) > 60, "his cleaver");
        assert!(count(&|(r, g, bl)| r > 0x96 && g > 0x9a && bl > 0x96) > 300, "his pale wings and napkin");
        assert!(b.voxels.iter().all(|v| v.x < BEELZEBUB_SIZE.0 && v.y < BEELZEBUB_SIZE.1 && v.z < BEELZEBUB_SIZE.2));
        let s = fly_swarm();
        let top = s.voxels.iter().map(|v| v.y).max().unwrap() as f32 * FLY_SWARM_VOXEL;
        assert!(top > 1.0 && top < 2.2, "{top}");
        for part in 0..=FLY_SWARM_RIG.len() as u8 {
            assert!(s.voxels.iter().any(|v| v.part == part), "swarm part {part}");
        }
    }
}
