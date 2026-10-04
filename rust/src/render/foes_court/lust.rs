//! Lust's court's own (PRINCES.md §6): Asmodeus, the prince of Lust. (What he raises are the
//! elf-maids of the Garden of Thorns, `foes_thorns`.) He faces +z, stands in the middle of his grid
//! and is sculpted in the parts his rig swings.
//!
//! Asmodeus, as the Goetia draws him and the bower dresses him: tall and elegant, two and a half
//! times the hero's height, in a long coat of rose-red silk split at the front over dark hose,
//! embroidered in gold at the hem, the edges and the cuffs, a crimson doublet under it buttoned in
//! gold, a gold sash with a rose at its knot, and a high fan of a collar standing behind his heads.
//! Three heads on his shoulders: in the middle a handsome man's, pale, dark-haired, smiling, a
//! circlet of gold on his brow and his eyes burning rose; on his right a black bull's with pale
//! horns and a gold ring through its nose; on his left a ram's in cream wool, its horns curled.
//! Under the coat's hem, goose's feet; behind him a serpent's tail, crimson, banded, coiled on the
//! floor. In his right hand a long gold lance with a steel head and a swallow-tailed rose pennon,
//! held slanting up before him; his left hand open, raised to blow a kiss.

use std::f32::consts::{PI, TAU};

use glam::Vec3;

use crate::render::carve::{ball, limb, nz, v};
use crate::render::characters::{mat, Sculpt};
use crate::render::voxels::{Collected, PartDef};

mod pal {
    use super::mat;
    pub const SILK: u32 = 0xc4234a | mat::CLOTH;
    pub const SILK_L: u32 = 0xe04a6a | mat::CLOTH;
    pub const SILK_D: u32 = 0x8c1434 | mat::CLOTH;
    pub const DOUBLET: u32 = 0x6a0c22 | mat::CLOTH;
    pub const DOUBLET_L: u32 = 0x8e1a34 | mat::CLOTH;
    pub const HOSE: u32 = 0x2e0c18 | mat::CLOTH;
    pub const LACE: u32 = 0xf2e6dc | mat::CLOTH;
    pub const GOLD: u32 = 0xe2b040 | mat::GOLD;
    pub const GOLD_L: u32 = 0xfbe08a | mat::GOLD;
    pub const GOLD_D: u32 = 0xa47828 | mat::GOLD;
    pub const RUBY: u32 = 0xd01a40 | mat::GOLD;
    pub const STEEL: u32 = 0xc8ccd4 | mat::METAL;
    pub const STEEL_D: u32 = 0x7a808a | mat::METAL;
    pub const SKIN: u32 = 0xf0cbbc | mat::SKIN;
    pub const SKIN_D: u32 = 0xd8a898 | mat::SKIN;
    pub const LIPS: u32 = 0xb01838 | mat::SKIN;
    pub const HAIR: u32 = 0x24101a | mat::FUR;
    pub const HAIR_L: u32 = 0x3e1a2a | mat::FUR;
    pub const BULL: u32 = 0x2a1a16 | mat::FUR;
    pub const BULL_L: u32 = 0x4a3024 | mat::FUR;
    pub const MUZZLE: u32 = 0x5a4038 | mat::SKIN;
    pub const WOOL: u32 = 0xe2d6bc | mat::FUR;
    pub const WOOL_D: u32 = 0xbcae92 | mat::FUR;
    pub const RAM_FACE: u32 = 0xc8a888 | mat::SKIN;
    pub const HORN: u32 = 0xe8dcc0 | mat::IVORY;
    pub const HORN_D: u32 = 0xa89470 | mat::IVORY;
    pub const NOSTRIL: u32 = 0x140a08 | mat::SKIN;
    pub const EYE: u32 = 0xff5a8a | mat::GLOW;
    pub const EYE_BULL: u32 = 0xff3a2a | mat::GLOW;
    pub const EYE_RAM: u32 = 0xffb030 | mat::GLOW;
    pub const GOOSE: u32 = 0xe89a3a | mat::SKIN;
    pub const GOOSE_D: u32 = 0xb8701e | mat::SKIN;
    pub const SCALE: u32 = 0x7a1426 | mat::SKIN;
    pub const SCALE_L: u32 = 0xa82a3e | mat::SKIN;
    pub const SCALE_D: u32 = 0x4a0a18 | mat::SKIN;
    pub const BELLY: u32 = 0xe0b8a0 | mat::SKIN;
    pub const PENNON: u32 = 0xe02a5a | mat::CLOTH;
    pub const PENNON_D: u32 = 0xa81442 | mat::CLOTH;
}
use pal::*;

/// Asmodeus's grid and voxel: about three tiles tall, two and a half times the hero (as Mammon).
pub const ASMODEUS_SIZE: (i32, i32, i32) = (100, 124, 80);
pub const ASMODEUS_VOXEL: f32 = 0.025;
const MX: f32 = 50.0;
const MZ: f32 = 34.0;

/// Hips, shoulders and neck (his three heads swing together), and the root of his tail; the lance
/// goes with his right arm.
pub const ASMODEUS_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [MX - 8.0, 40.0, MZ] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [MX + 8.0, 40.0, MZ] },
    PartDef { name: "leftArm", boxes: &[], pivot: [MX - 21.0, 84.0, MZ] },
    PartDef { name: "rightArm", boxes: &[], pivot: [MX + 21.0, 84.0, MZ] },
    PartDef { name: "head", boxes: &[], pivot: [MX, 90.0, MZ] },
    PartDef { name: "tail", boxes: &[], pivot: [MX, 24.0, MZ - 12.0] },
];

/// The way his lance points at rest: forward and up, slanting.
fn lance_dir() -> Vec3 {
    let a = 55.0f32.to_radians();
    Vec3::new(0.0, a.sin(), a.cos())
}

/// The coat's half-widths (x across, z deep) at height `y`: a long skirt flaring to the hem, a
/// narrow waist, a full chest, and its middle's depth.
fn coat(y: f32) -> (f32, f32) {
    if y < 42.0 {
        let k = ((42.0 - y) / 38.0).clamp(0.0, 1.0);
        (15.0 + k.powf(1.2) * 11.0, 12.0 + k.powf(1.2) * 9.0)
    } else if y < 58.0 {
        (15.0 - (y - 42.0) / 16.0, 12.0 - (y - 42.0) / 16.0)
    } else {
        let k = ((y - 58.0) / 24.0).clamp(0.0, 1.0);
        (14.0 + (k * PI * 0.5).sin() * 5.0, 11.0 + (k * PI * 0.5).sin() * 2.5)
    }
}

/// A swallow-tailed pennon on the lance: hung from the shaft between `a` and `b`, streaming out
/// along `out` and drooping, its edge in gold.
fn pennon(s: &mut Sculpt, a: Vec3, b: Vec3, out: Vec3) {
    let along = b - a;
    let n = (along.length() * 1.5) as i32;
    for i in 0..=n {
        let t = i as f32 / n as f32;
        // Wider at the shaft, two tails at its end with a notch between.
        let reach = 20.0 - 4.0 * t;
        let steps = (reach * 1.5) as i32;
        for j in 0..=steps {
            let u = j as f32 / steps as f32;
            let notch = u > 0.7 && (t - 0.5).abs() < (u - 0.7) * 1.2;
            if notch {
                continue;
            }
            let droop = Vec3::new(0.0, -u * u * 6.0 + (u * 7.0).sin() * 0.8, 0.0);
            let p = a + along * t + out * (u * reach) + droop;
            let edge = t < 0.06 || t > 0.94 || u > 0.94 || (u > 0.7 && (t - 0.5).abs() < (u - 0.7) * 1.2 + 0.08);
            let c = if edge {
                GOLD
            } else if (u * 5.0 + t * 2.0).sin() > 0.6 {
                PENNON_D
            } else {
                PENNON
            };
            s.set(p.x.round() as i32, p.y.round() as i32, p.z.round() as i32, c);
        }
    }
}

/// A horn along `knots`, ridged.
fn horn(s: &mut Sculpt, knots: &[(Vec3, f32)]) {
    limb(s, knots, HORN);
    let (lo, hi) = knots.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), (p, r)| {
        (lo.min(*p - Vec3::splat(*r + 1.0)), hi.max(*p + Vec3::splat(*r + 1.0)))
    });
    for y in lo.y as i32..=hi.y as i32 {
        for z in lo.z as i32..=hi.z as i32 {
            for x in lo.x as i32..=hi.x as i32 {
                if s.get(x, y, z) == Some(HORN) && (x + y * 2 + z) % 5 == 0 {
                    s.recolor(x, y, z, HORN_D);
                }
            }
        }
    }
}

pub fn asmodeus() -> Collected {
    let (w, h, d) = ASMODEUS_SIZE;
    let mut s = Sculpt::new(w, h, d);

    // ---- legs in dark hose, and goose's feet: webbed, three-toed, orange
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        let x = MX + side * 8.0;
        s.capsule((x, 40.0, MZ), (x, 22.0, MZ + 1.0), 6.0, 4.6, HOSE);
        s.capsule((x, 22.0, MZ + 1.0), (x, 6.0, MZ), 4.6, 3.4, HOSE);
        // A gold garter below the knee.
        s.capsule((x, 20.0, MZ + 1.0), (x, 21.5, MZ + 1.0), 5.0, 5.0, GOLD);
        // The foot: a webbed fan of three toes, flat on the floor.
        ball(&mut s, v(x, 4.0, MZ + 1.0), v(3.6, 3.0, 3.6), GOOSE_D);
        for t in [-1.0f32, 0.0, 1.0] {
            let tip = v(x + t * 6.0 + side * 1.0, 1.0, MZ + 14.0 - t.abs() * 2.0);
            limb(&mut s, &[(v(x, 2.5, MZ + 3.0), 1.6), (tip, 1.0)], GOOSE);
            ball(&mut s, tip + v(0.0, 0.0, 1.0), Vec3::splat(0.9), GOOSE_D);
        }
        // The web between the toes.
        for z in (MZ + 3.0) as i32..=(MZ + 12.0) as i32 {
            let k = (z as f32 - MZ - 3.0) / 9.0;
            let half = 1.5 + k * 4.5;
            for xx in (x - half).round() as i32..=(x + half).round() as i32 {
                s.set(xx + side as i32, 1, z, GOOSE);
            }
        }
    }

    // ---- the serpent's tail, from under the coat's back, coiled on the floor to his left
    s.part(6);
    let tail = [
        (v(MX, 26.0, MZ - 10.0), 6.5),
        (v(MX - 1.0, 14.0, MZ - 19.0), 6.0),
        (v(MX - 6.0, 5.0, MZ - 25.0), 5.5),
        (v(MX - 17.0, 4.5, MZ - 27.0), 5.0),
        (v(MX - 28.0, 4.0, MZ - 20.0), 4.4),
        (v(MX - 34.0, 3.5, MZ - 8.0), 3.6),
        (v(MX - 33.0, 3.0, MZ + 3.0), 2.8),
        (v(MX - 27.0, 3.5, MZ + 8.0), 1.8),
        (v(MX - 22.0, 6.0, MZ + 7.0), 0.8),
    ];
    limb(&mut s, &tail, SCALE);
    s.paint(|x, y, z, c| {
        if c != SCALE {
            return None;
        }
        // Bands of darker scales along it, a pale belly underneath, a glint on the top of each.
        let band = ((x as f32 * 0.31 + z as f32 * 0.27).sin() * 0.5 + 0.5) > 0.78;
        Some(if y <= 1 {
            BELLY
        } else if band {
            SCALE_D
        } else if nz(x, y, z) > 0.82 {
            SCALE_L
        } else {
            SCALE
        })
    });

    // ---- the coat of rose silk: a long skirt split at the front over his legs, a narrow waist
    s.part(0);
    for y in 5..=84 {
        let yf = y as f32;
        let (rx, rz) = coat(yf);
        for z in 0..d {
            for x in 0..w {
                let (xf, zf) = (x as f32, z as f32);
                let (dx, dz) = ((xf - MX) / rx, (zf - MZ) / rz);
                let q = dx * dx + dz * dz;
                if q > 1.0 {
                    continue;
                }
                // The split down the front of the skirt, widening to the hem.
                let split = yf < 44.0 && dz > 0.1 && (xf - MX).abs() < 4.0 + (44.0 - yf) * 0.22;
                // (His legs stand inside it, their own: the coat goes round them.)
                if split || (yf < 44.0 && s.get(x, y, z).is_some()) {
                    continue;
                }
                let a = dz.atan2(dx);
                let fold = (a * 11.0 + (yf * 0.11).sin() * 1.5).sin();
                let hem = y <= 7;
                let c = if hem {
                    GOLD
                } else if yf > 58.0 && dz > 0.45 && (xf - MX).abs() < 7.0 + (yf - 58.0) * 0.12 {
                    // The doublet showing between the coat's lapels.
                    if (xf - MX).abs() < 1.0 && y % 4 == 0 {
                        GOLD_L
                    } else if ((xf - MX).abs() as i32 + y) % 6 == 0 {
                        DOUBLET_L
                    } else {
                        DOUBLET
                    }
                } else if fold > 0.6 {
                    SILK_L
                } else if fold < -0.55 {
                    SILK_D
                } else {
                    SILK
                };
                s.set(x, y, z, c);
            }
        }
    }
    // Gold embroidery down the skirt's split edges and up the lapels, and a vine of it above the hem.
    for y in 6..=84 {
        let yf = y as f32;
        for z in 0..d {
            for x in 0..w {
                if !matches!(s.get(x, y, z), Some(SILK | SILK_L | SILK_D)) {
                    continue;
                }
                let (xf, zf) = (x as f32, z as f32);
                let (rx, rz) = coat(yf);
                let dz = (zf - MZ) / rz;
                let off = (xf - MX).abs();
                let sw = 4.0 + (44.0 - yf) * 0.22;
                let split_edge = yf < 44.0 && dz > 0.1 && off >= sw && off <= sw + 1.8;
                let lapel = yf >= 44.0 && dz > 0.45 && (off - 7.0 - (yf - 58.0).max(0.0) * 0.12).abs() < 1.2;
                let vine = (9..=12).contains(&y) && ((xf + zf) * 0.7).sin() > 0.2;
                let _ = rx;
                if split_edge || lapel || vine {
                    s.recolor(x, y, z, if (x + y) % 3 == 0 { GOLD_L } else { GOLD });
                }
            }
        }
    }
    // The gold sash at the waist, a rose at its knot and its ends hanging down his left hip.
    for y in 46..=50 {
        let (rx, rz) = coat(y as f32);
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = ((x as f32 - MX) / (rx + 1.2), (z as f32 - MZ) / (rz + 1.2));
                if dx * dx + dz * dz <= 1.0 && s.get(x, y, z).is_none() || (dx * dx + dz * dz <= 1.0 && (dx * dx + dz * dz) > 0.75) {
                    s.set(x, y, z, if y == 48 { GOLD_L } else { GOLD });
                }
            }
        }
    }
    let knot = v(MX - 8.0, 48.0, MZ + 13.0);
    for k in 0..12 {
        let a = k as f32 / 12.0 * TAU;
        ball(&mut s, knot + v(a.cos() * 2.4, a.sin() * 2.4, 0.5), Vec3::splat(1.6), if k % 2 == 0 { RUBY } else { SILK_D });
    }
    ball(&mut s, knot + v(0.0, 0.0, 1.2), Vec3::splat(1.6), RUBY);
    for (dx, len) in [(-2.0f32, 16.0f32), (1.5, 12.0)] {
        limb(&mut s, &[(knot + v(dx, -2.0, 0.0), 1.5), (knot + v(dx - 1.5, -len, 1.5), 1.1)], GOLD_D);
    }

    // ---- shoulders, and the high fan of a collar behind his heads
    ball(&mut s, v(MX, 82.0, MZ - 1.0), v(22.0, 7.0, 13.0), SILK);
    for y in 82..=110 {
        let t = (y - 82) as f32 / 28.0;
        let r = 12.0 + t * 12.0;
        for k in 0..60 {
            let a = PI * (0.08 + 0.84 * k as f32 / 59.0);
            let x = MX + a.cos() * r;
            let z = MZ - 7.0 - a.sin() * (4.0 + t * 5.0);
            let rib = k % 6 == 0;
            let rim = y >= 108 || (a - PI * 0.08).abs() < 0.03 || (a - PI * 0.92).abs() < 0.03;
            let c = if rim || rib {
                GOLD
            } else if (y + k) % 7 == 0 {
                LACE
            } else {
                DOUBLET
            };
            // (Its top a scalloped edge.)
            if y > 104 && ((k as f32 * 0.52).sin() * 3.0 + 105.0) < y as f32 {
                continue;
            }
            s.set(x.round() as i32, y, z.round() as i32, c);
            s.set(x.round() as i32, y, z.round() as i32 - 1, DOUBLET);
        }
    }

    // ---- arms: puffed sleeves of silk slashed with crimson, gold cuffs frothing lace, pale hands
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(3 + i as u8);
        let sh = v(MX + side * 21.0, 83.0, MZ);
        ball(&mut s, sh + v(side * 1.0, -2.0, 0.0), v(7.5, 8.0, 7.5), SILK_L);
        for k in 0..6 {
            let a = k as f32 / 6.0 * TAU;
            limb(
                &mut s,
                &[(sh + v(a.cos() * 6.0 + side, 3.0, a.sin() * 6.0), 1.4), (sh + v(a.cos() * 7.0 + side, -8.0, a.sin() * 7.0), 1.4)],
                DOUBLET,
            );
        }
        let (elbow, wrist) = if side > 0.0 {
            // His right, low before him, the lance in his fist.
            (v(MX + 26.0, 64.0, MZ + 4.0), v(MX + 25.0, 50.0, MZ + 13.0))
        } else {
            // His left, bent and raised, the hand open at his lips' height to blow a kiss.
            (v(MX - 27.0, 66.0, MZ + 6.0), v(MX - 18.0, 76.0, MZ + 17.0))
        };
        limb(&mut s, &[(sh + v(side, -6.0, 0.0), 5.0), (elbow, 4.4)], SILK);
        limb(&mut s, &[(elbow, 4.4), (wrist, 3.6)], SILK);
        let cuff = wrist + (wrist - elbow).normalize() * -1.0;
        ball(&mut s, cuff, v(4.6, 4.6, 4.6), GOLD);
        ball(&mut s, wrist + (wrist - elbow).normalize() * 1.6, v(4.2, 4.2, 4.2), LACE);
        let hand = wrist + (wrist - elbow).normalize() * 4.0;
        if side > 0.0 {
            // A fist round the shaft, a ruby ring.
            ball(&mut s, hand, v(3.6, 4.0, 3.6), SKIN);
            ball(&mut s, hand + v(-2.4, 0.5, 1.5), Vec3::splat(1.0), RUBY);
            // The lance: a long gold shaft banded in rose, a vamplate before the hand, a steel head.
            let dir = lance_dir();
            let butt = hand - dir * 20.0;
            let tip = hand + dir * 56.0;
            limb(&mut s, &[(butt, 1.6), (hand, 1.8), (tip - dir * 10.0, 1.5)], GOLD);
            let n = 56;
            for k in 0..n {
                let p = butt + dir * (k as f32 * 76.0 / n as f32);
                if k % 7 == 3 {
                    ball(&mut s, p, Vec3::splat(2.0), SILK_D);
                }
            }
            ball(&mut s, butt, Vec3::splat(2.4), GOLD_D);
            // The vamplate: a cone of gold opening toward the hand.
            for k in 0..8 {
                let c = hand + dir * (3.0 + k as f32);
                let r = 6.0 - k as f32 * 0.6;
                let (u, w2) = (Vec3::X, dir.cross(Vec3::X).normalize());
                for j in 0..((r * 7.0) as i32) {
                    let a = j as f32 / (r * 7.0) * TAU;
                    let p = c + u * a.cos() * r + w2 * a.sin() * r;
                    s.set(p.x.round() as i32, p.y.round() as i32, p.z.round() as i32, if k == 0 { GOLD_L } else { GOLD_D });
                }
            }
            // The head: a long leaf of steel, its edge bright.
            for k in 0..14 {
                let t = k as f32 / 13.0;
                let c = tip - dir * 10.0 + dir * (k as f32 * 1.0);
                let r = 2.8 * (t * PI).sin().max(0.15) * (1.0 - t * 0.3);
                limb(&mut s, &[(c - Vec3::X * r, 0.7), (c + Vec3::X * r, 0.7)], if k % 4 == 0 { STEEL_D } else { STEEL });
            }
            ball(&mut s, tip - dir * 11.0, Vec3::splat(2.2), GOLD_L);
            pennon(&mut s, tip - dir * 24.0, tip - dir * 13.0, Vec3::new(1.0, -0.2, -0.15).normalize());
        } else {
            // An open hand, the fingers spread up and toward his lips.
            ball(&mut s, hand, v(3.4, 3.8, 2.6), SKIN);
            for (k, a) in [-0.7f32, -0.25, 0.2, 0.6].into_iter().enumerate() {
                let base = hand + v(a * 3.0, 2.6, 0.5);
                let tip = base + v(a * 2.2, 4.5 - (k as f32 - 1.5).abs() * 0.6, 1.0);
                limb(&mut s, &[(base, 1.1), (tip, 0.8)], SKIN);
            }
            limb(&mut s, &[(hand + v(3.0, 0.0, 0.5), 1.1), (hand + v(5.0, 2.0, 1.5), 0.8)], SKIN);
            ball(&mut s, hand + v(-0.5, 3.5, 1.4), Vec3::splat(0.9), GOLD);
        }
    }

    // ---- the three heads on their necks
    s.part(5);
    // The man's, in the middle: a long neck, the face pale and fine, smiling.
    s.capsule((MX, 84.0, MZ), (MX, 95.0, MZ + 1.0), 5.0, 4.4, SKIN);
    let hc = v(MX, 102.0, MZ + 2.0);
    ball(&mut s, hc, v(7.4, 9.4, 8.2), SKIN);
    ball(&mut s, hc + v(0.0, -6.5, 2.5), v(5.0, 4.0, 5.5), SKIN);
    // His hair: dark, swept back, long at the nape, a lock over the brow.
    for y in (hc.y - 4.0) as i32..=(hc.y + 11.0) as i32 {
        for z in (hc.z - 10.0) as i32..=(hc.z + 9.0) as i32 {
            for x in (hc.x - 9.0) as i32..=(hc.x + 9.0) as i32 {
                let p = v(x as f32, y as f32, z as f32) - hc;
                let q = (p.x / 8.4).powi(2) + (p.y / 10.4).powi(2) + (p.z / 9.2).powi(2);
                let back = p.z < 1.5 - (p.y + 4.0).max(0.0) * 0.6;
                if q <= 1.0 && (p.y > 4.5 || back) && s.get(x, y, z) != Some(SKIN) || (q <= 1.0 && back) {
                    s.set(x, y, z, if (x * 3 + y) % 5 == 0 { HAIR_L } else { HAIR });
                }
            }
        }
    }
    limb(&mut s, &[(hc + v(0.0, 3.0, -6.0), 5.0), (hc + v(0.0, -8.0, -7.0), 4.0), (hc + v(0.0, -13.0, -5.0), 2.6)], HAIR);
    limb(&mut s, &[(hc + v(2.0, 9.0, 6.0), 1.6), (hc + v(4.0, 5.0, 8.5), 1.0)], HAIR_L);
    // The circlet of gold, a ruby at the brow and points like a crown's.
    for k in 0..40 {
        let a = k as f32 / 40.0 * TAU;
        let p = hc + v(a.cos() * 8.0, 5.0 + a.sin() * 0.6, a.sin() * 8.8);
        ball(&mut s, p, Vec3::splat(0.9), GOLD);
        if k % 5 == 0 {
            limb(&mut s, &[(p, 0.9), (p + v(0.0, 3.0, 0.0), 0.4)], GOLD_L);
        }
    }
    ball(&mut s, hc + v(0.0, 5.0, 9.0), Vec3::splat(1.4), RUBY);
    // The face: a straight nose, rose-burning eyes under fine brows, a thin moustache, the smile.
    let front = |s: &Sculpt, x: i32, y: i32| (0..d).rev().find(|&z| s.get(x, y, z).is_some());
    limb(&mut s, &[(hc + v(0.0, 1.0, 8.0), 1.0), (hc + v(0.0, -2.5, 9.6), 1.4)], SKIN_D);
    for side in [-1i32, 1] {
        let ex = MX as i32 + side * 3;
        for (dx, dy) in [(0, 0), (1, 0)] {
            let (x, y) = (ex + dx * side, hc.y as i32 + 1 + dy);
            if let Some(z) = front(&s, x, y) {
                s.recolor(x, y, z, EYE);
            }
        }
        for dx in 0..3 {
            let (x, y) = (MX as i32 + side * (2 + dx), hc.y as i32 + 3 + (dx == 1) as i32);
            if let Some(z) = front(&s, x, y) {
                s.recolor(x, y, z, HAIR);
            }
        }
        // The moustache, curled up at its ends.
        for dx in 1..4 {
            let (x, y) = (MX as i32 + side * dx, hc.y as i32 - 4 + (dx == 3) as i32);
            if let Some(z) = front(&s, x, y) {
                s.recolor(x, y, z, HAIR);
            }
        }
    }
    // The smile.
    for x in (MX - 3.0) as i32..=(MX + 3.0) as i32 {
        let off = (x as f32 - MX).abs();
        let y = hc.y as i32 - 6 + (off > 2.0) as i32;
        if let Some(z) = front(&s, x, y) {
            s.recolor(x, y, z, LIPS);
        }
    }

    // The bull's, on his right: a black brow and muzzle, pale horns sweeping out and up, a gold ring.
    let bc = v(MX + 15.0, 96.0, MZ + 1.0);
    limb(&mut s, &[(v(MX + 9.0, 84.0, MZ - 1.0), 6.0), (bc + v(-1.0, -3.0, -1.0), 5.0)], BULL);
    ball(&mut s, bc, v(6.4, 6.6, 7.2), BULL);
    ball(&mut s, bc + v(1.0, -3.0, 7.0), v(4.6, 4.2, 4.4), MUZZLE);
    ball(&mut s, bc + v(0.0, 4.0, 1.0), v(5.8, 2.4, 5.0), BULL_L);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, bc + v(1.0 + side * 1.8, -3.0, 11.0), Vec3::splat(0.9), NOSTRIL);
        // The ears, out to the sides.
        limb(&mut s, &[(bc + v(side * 5.5, 1.0, -1.0), 1.8), (bc + v(side * 9.0, 0.0, -1.0), 1.0)], BULL_L);
        let root = bc + v(side * 4.5, 4.5, 0.0);
        horn(
            &mut s,
            &[
                (root, 2.2),
                (root + v(side * 5.0, 1.5, 1.0), 1.9),
                (root + v(side * 9.0, 5.0, 2.0), 1.5),
                (root + v(side * 10.0, 10.0, 1.0), 1.0),
                (root + v(side * 9.0, 13.5, 0.0), 0.5),
            ],
        );
    }
    for k in 0..14 {
        let a = k as f32 / 14.0 * TAU;
        s.set((bc.x + 1.0).round() as i32, (bc.y - 5.5 + a.sin() * 1.8).round() as i32, (bc.z + 11.5 + a.cos() * 1.0).round() as i32, GOLD);
    }
    for (dx, dy) in [(-3.0f32, 1.5f32), (4.0, 1.5)] {
        let (x, y) = ((bc.x + dx).round() as i32, (bc.y + dy).round() as i32);
        if let Some(z) = front(&s, x, y) {
            s.recolor(x, y, z, EYE_BULL);
        }
    }

    // The ram's, on his left: cream wool, a long tan face, horns curled round its ears.
    let rc = v(MX - 15.0, 96.0, MZ + 1.0);
    limb(&mut s, &[(v(MX - 9.0, 84.0, MZ - 1.0), 6.0), (rc + v(1.0, -3.0, -1.0), 5.0)], WOOL);
    ball(&mut s, rc, v(6.0, 6.4, 6.6), WOOL);
    limb(&mut s, &[(rc + v(0.0, 1.0, 3.0), 3.8), (rc + v(-0.5, -3.5, 9.0), 2.8)], RAM_FACE);
    ball(&mut s, rc + v(-0.5, -4.5, 10.0), v(2.4, 1.8, 1.4), NOSTRIL);
    s.paint(|x, y, z, c| {
        if c != WOOL {
            return None;
        }
        Some(if nz(x, y, z) > 0.6 { WOOL_D } else { WOOL })
    });
    for side in [-1.0f32, 1.0] {
        // A horn curling back, down and forward round the ear: a spiral.
        let mut knots = Vec::new();
        for k in 0..=16 {
            let t = k as f32 / 16.0;
            let a = t * 1.7 * TAU;
            let r = 6.0 - t * 2.5;
            let p = rc + v(side * (4.0 + t * 3.5), 4.0 + a.cos() * r - r, -a.sin() * r * 0.9);
            knots.push((p, 2.6 - t * 1.8));
        }
        horn(&mut s, &knots);
    }
    for (dx, dy) in [(-3.5f32, 1.0f32), (3.0, 1.0)] {
        let (x, y) = ((rc.x + dx).round() as i32, (rc.y + dy).round() as i32);
        if let Some(z) = front(&s, x, y) {
            s.recolor(x, y, z, EYE_RAM);
        }
    }

    s.finish(0.03)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asmodeus_towers_over_the_hero_in_rose_silk_with_three_heads_a_lance_and_a_tail() {
        let m = asmodeus();
        let top = m.voxels.iter().map(|v| v.y).max().unwrap() as f32 * ASMODEUS_VOXEL;
        assert!(top > 2.7 && top < 3.4, "{top} tiles tall");
        assert!(m.voxels.len() > 8000, "{} voxels", m.voxels.len());
        for part in 0..=ASMODEUS_RIG.len() as u8 {
            assert!(m.voxels.iter().any(|v| v.part == part), "part {part}");
        }
        let is = |c: u32, m: u32| c & 0xff00_0000 == m;
        assert!(m.voxels.iter().filter(|v| is(v.c, mat::CLOTH)).count() > 4000, "dressed in silk");
        assert!(m.voxels.iter().filter(|v| is(v.c, mat::GOLD)).count() > 600, "gold-trimmed");
        // Three faces' eyes burning, and the lance reaching out before him.
        let eyes: Vec<f32> = m.voxels.iter().filter(|v| is(v.c, mat::GLOW) && v.part == 5).map(|v| v.x as f32 - MX).collect();
        assert!(
            eyes.iter().any(|x| *x < -8.0) && eyes.iter().any(|x| x.abs() < 6.0) && eyes.iter().any(|x| *x > 8.0),
            "three heads' eyes: {eyes:?}"
        );
        let reach = m.voxels.iter().filter(|v| v.part == 4).map(|v| v.z).max().unwrap();
        assert!(reach as f32 > MZ + 30.0, "the lance before him");
        // The tail on the floor behind him.
        assert!(m.voxels.iter().any(|v| v.part == 6 && (v.z as f32) < MZ - 20.0 && v.y < 4));
    }
}
