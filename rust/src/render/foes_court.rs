//! The princes' courts' own (ROADMAP.md, phase 3): Mammon, the prince of Greed, and the gold
//! golems he raises from his heaps. Each faces +z, stands in the middle of its grid and is
//! sculpted in the parts its rig swings.
//!
//! Mammon is a corpulent demon-king two and a half times the hero's height: a robe of umber wool
//! trimmed in gold, open over a great belly of dark red hide with coins sewn into it, a charcoal
//! belt with a copper buckle and a money sack hung from it, a mantle of gold chain over his
//! shoulders, a jowled head with burning amber eyes and gold teeth, ram's horns curling from his
//! temples, a crown of stacked coins, and a heavy gold sceptre in his right fist, its head a
//! spiked disc of gold. His left hand is an open, ringed claw for grasping and for coins.
//!
//! The golems are lumps of fused coins with gold bars for arms, embers for eyes.

use std::f32::consts::TAU;

use glam::Vec3;

use super::carve::{ball, limb, nz, v};
use super::characters::{mat, Sculpt};
use super::voxels::{Collected, PartDef};

mod pal {
    use super::mat;
    pub const HIDE: u32 = 0x9a3e2a | mat::SKIN;
    pub const HIDE_L: u32 = 0xb85a3a | mat::SKIN;
    pub const HIDE_D: u32 = 0x6e2a1c | mat::SKIN;
    pub const ROBE: u32 = 0x6e4a30 | mat::CLOTH;
    pub const ROBE_L: u32 = 0x8a6442 | mat::CLOTH;
    pub const ROBE_D: u32 = 0x4e3220 | mat::CLOTH;
    pub const LINING: u32 = 0x6a1a1c | mat::CLOTH;
    pub const GOLD: u32 = 0xe2b040 | mat::GOLD;
    pub const GOLD_L: u32 = 0xfbe08a | mat::GOLD;
    pub const GOLD_D: u32 = 0xa47828 | mat::GOLD;
    pub const COPPER: u32 = 0xb06a44 | mat::METAL;
    pub const BELT: u32 = 0x2a2420 | mat::LEATHER;
    pub const BOOT: u32 = 0x221c1a | mat::LEATHER;
    pub const SACK: u32 = 0x8a6a3e | mat::LEATHER;
    pub const SACK_D: u32 = 0x6a4e2a | mat::LEATHER;
    pub const HORN: u32 = 0x5a4a3c | mat::IVORY;
    pub const HORN_L: u32 = 0x8e7a62 | mat::IVORY;
    pub const HORN_TIP: u32 = 0xe0d0aa | mat::IVORY;
    pub const EYE: u32 = 0xffb030 | mat::GLOW;
    pub const MAW: u32 = 0x1a0a08 | mat::SKIN;
    pub const RUBY: u32 = 0xc01a2a | mat::GOLD;
    pub const NAIL: u32 = 0x2a2018 | mat::IVORY;
    pub const EMBER: u32 = 0xff9a2a | mat::GLOW;
}
use pal::*;

/// Mammon's grid and voxel: about three tiles tall, two and a half times the hero.
pub const MAMMON_SIZE: (i32, i32, i32) = (100, 124, 80);
pub const MAMMON_VOXEL: f32 = 0.025;
const MX: f32 = 50.0;
const MZ: f32 = 38.0;

/// Hips, shoulders and neck; the sceptre goes with his right arm, the sack with his body.
pub const MAMMON_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [MX - 11.0, 34.0, MZ] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [MX + 11.0, 34.0, MZ] },
    PartDef { name: "leftArm", boxes: &[], pivot: [MX - 30.0, 84.0, MZ] },
    PartDef { name: "rightArm", boxes: &[], pivot: [MX + 30.0, 84.0, MZ] },
    PartDef { name: "head", boxes: &[], pivot: [MX, 92.0, MZ + 4.0] },
];

/// A coin set into his hide, facing out along `n` at `c`.
fn coin_at(s: &mut Sculpt, c: Vec3, n: Vec3, r: f32) {
    let n = n.normalize_or_zero();
    let a = if n.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    let u = n.cross(a).normalize();
    let w = n.cross(u);
    let steps = (r * 2.0).ceil() as i32;
    for i in -steps..=steps {
        for j in -steps..=steps {
            let (fi, fj) = (i as f32 * 0.5, j as f32 * 0.5);
            let d = (fi * fi + fj * fj).sqrt();
            if d > r {
                continue;
            }
            let p = c + u * fi + w * fj;
            let col = if d > r - 0.7 {
                GOLD_D
            } else if (fi + fj) < -0.5 {
                GOLD_L
            } else {
                GOLD
            };
            s.set(p.x.round() as i32, p.y.round() as i32, p.z.round() as i32, col);
        }
    }
}

/// The robe's half-widths (x across, z deep) at height `y`: a heavy bell to the floor.
fn robe(y: f32) -> (f32, f32) {
    let k = ((56.0 - y) / 50.0).clamp(0.0, 1.0);
    (25.0 + k.powf(1.3) * 9.0, 22.0 + k.powf(1.3) * 7.0)
}

pub fn mammon() -> Collected {
    let (w, h, d) = MAMMON_SIZE;
    let mut s = Sculpt::new(w, h, d);

    // ---- legs and feet: thick, under the robe; black boots with clawed toes at the hem
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        let x = MX + side * 11.0;
        s.capsule((x, 34.0, MZ), (x, 8.0, MZ + 2.0), 8.0, 7.0, HIDE_D);
        ball(&mut s, v(x, 4.0, MZ + 8.0), v(7.0, 4.5, 10.0), BOOT);
        for t in [-3.5f32, 0.0, 3.5] {
            limb(&mut s, &[(v(x + t, 3.0, MZ + 16.0), 1.4), (v(x + t * 1.2, 1.2, MZ + 20.0), 0.7)], NAIL);
        }
        // A gold band at the ankle.
        s.capsule((x, 9.0, MZ + 2.0), (x, 11.0, MZ + 2.0), 7.6, 7.6, GOLD_D);
    }

    // ---- the belly: dark red hide, a great swell (the robe hangs over all but its front)
    s.part(0);
    ball(&mut s, v(MX, 50.0, MZ + 8.0), v(24.0, 22.0, 24.0), HIDE);
    ball(&mut s, v(MX, 70.0, MZ + 6.0), v(20.0, 13.0, 18.0), HIDE);
    s.paint(|x, y, z, c| {
        if c != HIDE {
            return None;
        }
        let n = nz(x, y, z);
        Some(if n < 0.18 {
            HIDE_L
        } else if n > 0.85 {
            HIDE_D
        } else {
            HIDE
        })
    });
    // ---- the robe: an umber bell, open down the front over his belly
    s.part(0);
    for y in 7..=78 {
        let yf = y as f32;
        let (rx, rz) = robe(yf);
        // Above the waist the robe follows the swell of his body to the shoulders.
        let (rx, rz) = if yf > 56.0 {
            let t = ((yf - 56.0) / 22.0).clamp(0.0, 1.0);
            (rx + 6.0 * (t * std::f32::consts::PI).sin() + t * 3.0, rz - t * 4.0)
        } else {
            (rx, rz)
        };
        for z in 0..d {
            for x in 0..w {
                let (xf, zf) = (x as f32, z as f32);
                let (dx, dz) = ((xf - MX) / rx, (zf - MZ) / rz);
                let q = dx * dx + dz * dz;
                if q > 1.0 {
                    continue;
                }
                // The opening down the front, wider at the belly.
                let open = (xf - MX).abs() < 10.0 + ((yf - 30.0) / 30.0).clamp(0.0, 1.0) * 6.0 && dz > 0.55 && y > 12;
                if open && q > 0.55 {
                    continue;
                }
                let a = dz.atan2(dx);
                let fold = (a * 9.0 + (yf * 0.09).sin()).sin();
                let c = if y <= 10 || (open && q > 0.5) {
                    GOLD
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
    // Gold trim down both edges of the opening (a band two voxels wide), and round the hem.
    for y in 8..=78 {
        let yf = y as f32;
        let half = 10.0 + ((yf - 30.0) / 30.0).clamp(0.0, 1.0) * 6.0;
        for z in 0..d {
            for x in 0..w {
                let is_robe = matches!(s.get(x, y, z), Some(ROBE | ROBE_L | ROBE_D));
                if !is_robe {
                    continue;
                }
                let off = (x as f32 - MX).abs();
                let edge = y > 12 && off >= half - 0.5 && off <= half + 2.0 && z as f32 > MZ + 12.0;
                if edge || y <= 11 {
                    s.recolor(x, y, z, if edge && off > half + 1.0 { GOLD_D } else { GOLD });
                }
            }
        }
    }

    // ---- coins sewn in rows across the belly and the chest
    for (row, y) in [(0, 40.0f32), (1, 50.0), (2, 60.0), (3, 70.0)] {
        let n = 5 + row % 2;
        for k in 0..n {
            let a = -0.9 + 1.8 * (k as f32 + 0.5) / n as f32;
            let (rx, rz, cy) = if y < 64.0 { (24.0, 24.0, 50.0) } else { (20.0, 18.0, 70.0) };
            let ry = if y < 64.0 { 22.0 } else { 13.0 };
            let t = ((y - cy) / ry).clamp(-0.95, 0.95);
            let ring = (1.0 - t * t).sqrt();
            let c = v(MX + a.sin() * rx * ring, y, MZ + 8.0 + a.cos() * rz * ring - if y >= 64.0 { 2.0 } else { 0.0 });
            let nrm = Vec3::new(a.sin(), t * 0.5, a.cos());
            coin_at(&mut s, c + nrm * 0.6, nrm, 2.2);
        }
    }
    // The belt under the belly: charcoal leather, copper studs, a gold-and-copper buckle.
    for y in 33..=37 {
        for z in 0..d {
            for x in 0..w {
                let (xf, zf) = (x as f32, z as f32);
                let (rx, rz) = robe(y as f32);
                let q = ((xf - MX) / (rx + 1.0)).powi(2) + ((zf - MZ - 1.0) / (rz + 1.0)).powi(2);
                if (0.82..=1.0).contains(&q) || (q < 1.0 && zf > MZ + 18.0) {
                    let stud = (x % 5 == 0) && y == 35;
                    s.set(x, y, z, if stud { COPPER } else { BELT });
                }
            }
        }
    }
    let buckle = v(MX, 35.0, MZ + 31.0);
    for y in 31..=39 {
        for x in (MX - 5.0) as i32..=(MX + 5.0) as i32 {
            let ring = (x as f32 - MX).abs() > 3.5 || !(33..=37).contains(&y);
            s.set(x, y, buckle.z as i32, if ring { GOLD } else { COPPER });
            s.set(x, y, buckle.z as i32 - 1, BELT);
        }
    }
    // The money sack at his left hip, tied with gold cord, coins spilling at its mouth.
    let sack = v(MX - 26.0, 24.0, MZ + 14.0);
    ball(&mut s, sack, v(8.0, 10.0, 8.0), SACK);
    ball(&mut s, sack + v(0.0, -3.0, 0.0), v(8.6, 7.0, 8.6), SACK);
    s.paint(|x, y, z, c| {
        if c != SACK {
            return None;
        }
        Some(if nz(x, y, z) < 0.3 { SACK_D } else { SACK })
    });
    limb(&mut s, &[(sack + v(0.0, 9.0, 0.0), 3.0), (sack + v(0.0, 12.0, 0.0), 2.2)], SACK_D);
    s.capsule((sack.x - 3.5, sack.y + 9.0, sack.z), (sack.x + 3.5, sack.y + 9.0, sack.z), 1.2, 1.2, GOLD);
    limb(&mut s, &[(sack + v(0.0, 12.0, 0.0), 1.0), (v(MX - 22.0, 34.0, MZ + 18.0), 1.0)], GOLD_D);
    for k in 0..4 {
        coin_at(&mut s, sack + v(-2.0 + k as f32 * 1.6, 13.0 + (k % 2) as f32, 2.0), Vec3::new(0.2, 1.0, 0.3), 1.6);
    }

    // ---- shoulders and the mantle of gold chain
    ball(&mut s, v(MX, 80.0, MZ + 1.0), v(30.0, 8.0, 20.0), ROBE);
    for k in 0..60 {
        let a = k as f32 / 60.0 * TAU;
        for (r, y) in [(26.0f32, 82.0f32), (22.0, 86.0)] {
            let c = v(MX + a.cos() * r, y - (a.sin().max(0.0)) * 4.0, MZ + 1.0 + a.sin() * r * 0.66);
            let col = if k % 2 == 0 { GOLD } else { GOLD_D };
            ball(&mut s, c, v(1.3, 1.3, 1.3), col);
        }
    }
    // A great medallion on the chest.
    coin_at(&mut s, v(MX, 78.0, MZ + 22.0), Vec3::new(0.0, 0.2, 1.0), 4.4);
    ball(&mut s, v(MX, 78.0, MZ + 23.0), Vec3::splat(1.4), RUBY);

    // ---- arms: wide sleeves to the elbow, bare hide below sewn with coins; gold cuffs
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(3 + i as u8);
        let sh = v(MX + side * 30.0, 82.0, MZ);
        let elbow = v(MX + side * 37.0, 62.0, MZ + 6.0);
        let wrist = v(MX + side * 36.0, 46.0, MZ + 16.0);
        ball(&mut s, sh, v(10.0, 10.0, 10.0), ROBE);
        limb(&mut s, &[(sh, 9.5), (elbow, 10.5)], ROBE);
        // The sleeve's open cuff, trimmed in gold and lined in red.
        s.capsule((elbow.x, elbow.y - 1.0, elbow.z), (elbow.x, elbow.y - 4.0, elbow.z + 1.0), 11.0, 11.5, LINING);
        s.capsule((elbow.x, elbow.y - 4.0, elbow.z + 1.0), (elbow.x, elbow.y - 5.0, elbow.z + 1.0), 11.6, 11.6, GOLD);
        limb(&mut s, &[(elbow, 7.0), (wrist, 6.0)], HIDE);
        for k in 0..3 {
            let t = 0.3 + k as f32 * 0.25;
            let p = elbow.lerp(wrist, t);
            let out = Vec3::new(side, 0.0, 0.6).normalize();
            coin_at(&mut s, p + out * 6.2, out, 1.8);
        }
        s.capsule((wrist.x, wrist.y + 2.0, wrist.z), (wrist.x, wrist.y - 1.0, wrist.z), 6.4, 6.4, GOLD_D);
        let hand = wrist + v(side * 0.5, -6.0, 2.5);
        ball(&mut s, hand, v(6.0, 6.5, 6.0), HIDE);
        if side < 0.0 {
            // His left: an open claw, ringed in gold, for grasping and for coins.
            for (k, a) in [-0.9f32, -0.3, 0.3, 0.9].into_iter().enumerate() {
                let base = hand + v(a * 4.0, -4.0, 3.0);
                let tip = base + v(a * 2.5, -6.0, 4.0);
                limb(&mut s, &[(base, 2.0), (tip, 1.3)], HIDE);
                limb(&mut s, &[(tip, 1.1), (tip + v(0.0, -2.0, 1.5), 0.5)], NAIL);
                if k % 2 == 0 {
                    s.capsule((base.x, base.y - 1.0, base.z), (base.x, base.y - 2.0, base.z), 2.3, 2.3, GOLD);
                }
            }
            limb(&mut s, &[(hand + v(5.0, -1.0, 2.0), 2.0), (hand + v(8.0, -5.0, 6.0), 1.4)], HIDE);
        } else {
            // His right: a fist round the sceptre's shaft.
            ball(&mut s, hand + v(0.0, -2.0, 3.0), v(6.0, 5.0, 5.0), HIDE_D);
            s.capsule((hand.x - 4.0, hand.y + 1.0, hand.z + 6.0), (hand.x + 4.0, hand.y + 1.0, hand.z + 6.0), 1.6, 1.6, GOLD);
            // The sceptre: a heavy gold shaft held upright, a spiked disc of gold at its head.
            let grip = hand + v(0.0, 0.0, 4.0);
            let foot = grip + v(0.0, -26.0, 1.0);
            let top = grip + v(0.0, 44.0, -3.0);
            limb(&mut s, &[(foot, 2.2), (grip, 2.6), (top, 2.6)], GOLD);
            for y in [12.0f32, 24.0, 34.0] {
                let c = grip + v(0.0, y, -y / 44.0 * 3.0);
                s.capsule((c.x, c.y, c.z), (c.x, c.y + 1.5, c.z), 3.6, 3.6, GOLD_D);
            }
            ball(&mut s, foot, Vec3::splat(3.0), GOLD_D);
            let head = top + v(0.0, 7.0, 0.0);
            // The disc faces forward, its rim spiked.
            for i in -9..=9 {
                for j in -9..=9 {
                    let r = ((i * i + j * j) as f32).sqrt();
                    if r > 8.5 {
                        continue;
                    }
                    let c = if r > 7.0 {
                        GOLD_D
                    } else if r < 2.5 {
                        RUBY
                    } else if (i + j) % 3 == 0 {
                        GOLD_L
                    } else {
                        GOLD
                    };
                    for dz in -1..=1 {
                        s.set(head.x as i32 + i, head.y as i32 + j, head.z as i32 + dz, c);
                    }
                }
            }
            for k in 0..10 {
                let a = k as f32 / 10.0 * TAU;
                let o = v(a.cos(), a.sin(), 0.0);
                limb(&mut s, &[(head + o * 8.0, 1.4), (head + o * 12.5, 0.4)], GOLD_L);
            }
        }
    }

    // ---- the head: jowled, burning eyes, a wide maw of gold teeth, horns and a crown of coins
    s.part(5);
    let hc = v(MX, 98.0, MZ + 7.0);
    ball(&mut s, v(MX, 89.0, MZ + 6.0), v(13.0, 6.0, 12.0), HIDE);
    ball(&mut s, hc, v(12.0, 12.0, 11.0), HIDE);
    for side in [-1.0f32, 1.0] {
        // The jowls.
        ball(&mut s, hc + v(side * 7.5, -7.0, 5.0), v(6.5, 6.0, 6.0), HIDE);
    }
    // The brow, heavy over the eyes.
    ball(&mut s, hc + v(0.0, 4.0, 8.0), v(11.0, 3.0, 4.0), HIDE_D);
    let front = |s: &Sculpt, x: i32, y: i32| (0..d).rev().find(|&z| s.get(x, y, z).is_some());
    for side in [-1i32, 1] {
        let ex = MX as i32 + side * 5;
        for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let (x, y) = (ex + dx * side, 100 + dy);
            if let Some(z) = front(&s, x, y) {
                s.recolor(x, y, z, EYE);
            }
        }
    }
    // The nose: a broad snout.
    ball(&mut s, hc + v(0.0, -1.0, 11.5), v(3.5, 3.0, 2.5), HIDE_L);
    // The maw: dark, rimmed with gold teeth.
    for x in (MX - 7.0) as i32..=(MX + 7.0) as i32 {
        let curve = (((x as f32 - MX) / 7.0).powi(2) * 2.0) as i32;
        for y in 90 + curve..=93 + curve {
            if let Some(z) = front(&s, x, y) {
                let tooth = (y == 93 + curve || y == 90 + curve) && x % 2 == 0;
                s.recolor(x, y, z, if tooth { GOLD_L } else { MAW });
            }
        }
    }
    // Horns: ram's horns from the temples, curling out and back, pale at the tips.
    for side in [-1.0f32, 1.0] {
        let mut knots = Vec::new();
        for k in 0..=12 {
            let t = k as f32 / 12.0;
            let a = t * 4.2;
            let r = 11.0 * (1.0 - t * 0.35);
            let p = hc + v(side * (10.0 + t * 8.0 + a.sin() * r * 0.5), 6.0 + a.sin() * r * 0.9, -2.0 - (1.0 - a.cos()) * r * 0.6);
            knots.push((p, 5.0 - 4.0 * t));
        }
        limb(&mut s, &knots, HORN);
        let tip = knots.last().unwrap().0;
        ball(&mut s, tip, Vec3::splat(1.0), HORN_TIP);
    }
    s.paint(|x, y, _, c| {
        if c != HORN {
            return None;
        }
        Some(if (y + x) % 4 == 0 { HORN_L } else { HORN })
    });
    // The crown of coins: stacks round his head, taller at the front, a ruby in each.
    for k in 0..14 {
        let a = k as f32 / 14.0 * TAU;
        let front_k = a.sin().max(0.0);
        let base = hc + v(a.cos() * 10.5, 9.0, a.sin() * 9.5);
        let tall = 5 + (front_k * 4.0) as i32 + (k % 2) * 2;
        for j in 0..tall {
            let c = base + v(0.0, j as f32 * 1.0, 0.0);
            let col = if j % 2 == 0 { GOLD } else { GOLD_D };
            ball(&mut s, c, v(2.4, 0.6, 2.4), col);
        }
        if k % 2 == 0 {
            ball(&mut s, base + v(a.cos() * 1.5, tall as f32 * 0.5, a.sin() * 1.5), Vec3::splat(1.1), RUBY);
        }
    }
    s.capsule((hc.x, hc.y + 9.0, hc.z), (hc.x, hc.y + 10.0, hc.z), 11.0, 11.0, GOLD_D);

    s.finish(0.03)
}

/// The golem's grid and voxel: half again the hero's height.
pub const GOLEM_SIZE: (i32, i32, i32) = (60, 84, 48);
pub const GOLEM_VOXEL: f32 = 0.02;
const GX: f32 = 30.0;
const GZ: f32 = 24.0;

pub const GOLEM_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [GX - 8.0, 26.0, GZ] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [GX + 8.0, 26.0, GZ] },
    PartDef { name: "leftArm", boxes: &[], pivot: [GX - 18.0, 60.0, GZ] },
    PartDef { name: "rightArm", boxes: &[], pivot: [GX + 18.0, 60.0, GZ] },
    PartDef { name: "head", boxes: &[], pivot: [GX, 66.0, GZ + 3.0] },
];

/// A lump of fused coins: an ellipsoid whose voxels read as stacked coin rims.
fn coin_lump(s: &mut Sculpt, c: Vec3, r: Vec3) {
    ball(s, c, r, GOLD);
    let (lo, hi) = (c - r - Vec3::ONE, c + r + Vec3::ONE);
    for y in lo.y as i32..=hi.y as i32 {
        for z in lo.z as i32..=hi.z as i32 {
            for x in lo.x as i32..=hi.x as i32 {
                if s.get(x, y, z) != Some(GOLD) {
                    continue;
                }
                let n = nz(x, y, z);
                let rim = (y % 2 == 0) ^ (n < 0.3);
                s.recolor(
                    x,
                    y,
                    z,
                    if n > 0.9 {
                        GOLD_L
                    } else if rim {
                        GOLD_D
                    } else {
                        GOLD
                    },
                );
            }
        }
    }
}

/// A gold bar from `a` to `b`, square in section.
fn bar(s: &mut Sculpt, a: Vec3, b: Vec3, half: f32) {
    let n = ((b - a).length() * 2.0).ceil() as i32;
    for k in 0..=n {
        let p = a.lerp(b, k as f32 / n.max(1) as f32);
        for dy in -(half as i32)..=half as i32 {
            for dz in -(half as i32)..=half as i32 {
                for dx in -(half as i32)..=half as i32 {
                    let edge = dx.abs() == half as i32 || dy.abs() == half as i32;
                    s.set(p.x as i32 + dx, p.y as i32 + dy, p.z as i32 + dz, if edge { GOLD_D } else { GOLD_L });
                }
            }
        }
    }
}

pub fn gold_golem() -> Collected {
    let (w, h, d) = GOLEM_SIZE;
    let mut s = Sculpt::new(w, h, d);
    // Legs: stumps of fused coin.
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        coin_lump(&mut s, v(GX + side * 8.0, 16.0, GZ), v(7.0, 11.0, 7.5));
        coin_lump(&mut s, v(GX + side * 8.5, 5.0, GZ + 2.0), v(7.5, 5.0, 9.0));
    }
    // The body: a hunched heap, a lump on a lump, a few bars jutting from it.
    s.part(0);
    coin_lump(&mut s, v(GX, 36.0, GZ), v(17.0, 13.0, 13.0));
    coin_lump(&mut s, v(GX, 52.0, GZ - 1.0), v(19.0, 11.0, 13.0));
    coin_lump(&mut s, v(GX - 6.0, 44.0, GZ + 7.0), v(8.0, 8.0, 7.0));
    bar(&mut s, v(GX + 6.0, 58.0, GZ - 6.0), v(GX + 12.0, 70.0, GZ - 12.0), 1.0);
    bar(&mut s, v(GX - 10.0, 56.0, GZ - 8.0), v(GX - 14.0, 66.0, GZ - 14.0), 1.0);
    // A seam of ember through the chest where the gold is fused.
    for k in 0..14 {
        let t = k as f32 / 13.0;
        let p = v(GX - 8.0 + t * 16.0, 46.0 + (t * 9.0).sin() * 2.0, GZ + 12.5);
        if let Some(z) = (0..d).rev().find(|&z| s.get(p.x as i32, p.y as i32, z).is_some()) {
            s.recolor(p.x as i32, p.y as i32, z, EMBER);
        }
    }
    // Arms: gold bars stacked into heavy limbs, fists of coin.
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(3 + i as u8);
        let sh = v(GX + side * 18.0, 58.0, GZ);
        coin_lump(&mut s, sh, v(7.0, 7.0, 7.0));
        bar(&mut s, sh + v(side * 2.0, -4.0, 1.0), sh + v(side * 4.0, -22.0, 4.0), 2.0);
        bar(&mut s, sh + v(side * 1.0, -6.0, -2.0), sh + v(side * 3.0, -24.0, 1.0), 1.0);
        coin_lump(&mut s, sh + v(side * 4.0, -30.0, 5.0), v(6.5, 6.5, 6.5));
    }
    // The head: a small lump sunk between the shoulders, two embers for eyes.
    s.part(5);
    let hc = v(GX, 68.0, GZ + 4.0);
    coin_lump(&mut s, hc, v(8.0, 7.0, 7.0));
    for side in [-1i32, 1] {
        let x = GX as i32 + side * 3;
        for y in 68..=69 {
            if let Some(z) = (0..d).rev().find(|&z| s.get(x, y, z).is_some()) {
                s.recolor(x, y, z, EYE);
                s.set(x, y, z + 1, EMBER);
            }
        }
    }
    s.finish(0.03)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mammon_towers_over_the_hero_and_the_golems_are_heaps_of_gold() {
        let m = mammon();
        let top = m.voxels.iter().map(|v| v.y).max().unwrap() as f32 * MAMMON_VOXEL;
        assert!(top > 2.7 && top < 3.4, "{top} tiles tall");
        assert!(m.voxels.len() > 8000, "{} voxels", m.voxels.len());
        // Every part of his rig has its voxels.
        for part in 0..=MAMMON_RIG.len() as u8 {
            assert!(m.voxels.iter().any(|v| v.part == part), "part {part}");
        }
        let gold = |c: u32| c & 0xff00_0000 == mat::GOLD;
        assert!(m.voxels.iter().filter(|v| gold(v.c)).count() > 1500, "gold-trimmed and crowned");
        let g = gold_golem();
        let top = g.voxels.iter().map(|v| v.y).max().unwrap() as f32 * GOLEM_VOXEL;
        assert!(top > 1.4 && top < 2.0, "{top}");
        assert!(g.voxels.iter().filter(|v| gold(v.c)).count() * 10 > g.voxels.len() * 8, "nearly all gold");
    }
}
