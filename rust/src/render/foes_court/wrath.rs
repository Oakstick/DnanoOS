//! Wrath's court's own (PRINCES.md §3): Amon, the prince of Wrath, and his ravens. Each faces +z,
//! stands in the middle of its grid and is sculpted in the parts its rig swings.
//!
//! Amon is a raven-headed demon warrior two and a half times the hero's height, huge in the
//! shoulder: a hide of dark red skin, scarred, cracked down the throat and chest where the fire in
//! him shows through; a wolf's pelt over his shoulders and down his back, its head on his left
//! shoulder and its forelegs knotted on his chest; an iron pauldron spiked on his right; a war
//! kilt of studded leather straps over a red loincloth, a belt with skulls hung from it; wolf's
//! legs in iron greaves, clawed paws. His head is a raven's: black feathers, a ragged crest and
//! hackles, two short black horns, ember eyes, and a great hooked beak lined with a dog's teeth,
//! the fire glowing between its halves. In his right fist a great iron axe, its crescent blade's
//! edge red with heat; his left hand an open claw. A serpent's tail coils behind him.
//!
//! His ravens are great black birds with their wings spread, their feathers smouldering at the
//! tips, ember-eyed, talons hanging.

use std::f32::consts::TAU;

use crate::render::carve::{ball, limb, nz, v};
use crate::render::characters::{mat, Sculpt};
use crate::render::voxels::{Collected, PartDef};

mod pal {
    use super::mat;
    pub const SKIN: u32 = 0x6e3024 | mat::SKIN;
    pub const SKIN_L: u32 = 0x8c4634 | mat::SKIN;
    pub const SKIN_D: u32 = 0x4a1e16 | mat::SKIN;
    pub const SCAR: u32 = 0xa8705a | mat::SKIN;
    pub const FUR: u32 = 0x4e4a46 | mat::FUR;
    pub const FUR_L: u32 = 0x77706a | mat::FUR;
    pub const FUR_D: u32 = 0x2a2624 | mat::FUR;
    pub const FEATHER: u32 = 0x16151c | mat::FUR;
    pub const GLOSS: u32 = 0x1c2032 | mat::FUR;
    pub const GLOSS_L: u32 = 0x283048 | mat::FUR;
    pub const BEAK: u32 = 0x1a1716 | mat::LEATHER;
    pub const BEAK_L: u32 = 0x36302c | mat::LEATHER;
    pub const HORN: u32 = 0x1e1a1a | mat::IVORY;
    pub const TOOTH: u32 = 0xe6dabe | mat::IVORY;
    pub const BONE: u32 = 0xd2c6a8 | mat::IVORY;
    pub const BONE_D: u32 = 0x9a8c70 | mat::IVORY;
    pub const CLAW: u32 = 0x141010 | mat::IVORY;
    pub const EYE: u32 = 0xffa828 | mat::GLOW;
    pub const FIRE: u32 = 0xff5a18 | mat::GLOW;
    pub const FIRE_HOT: u32 = 0xffc060 | mat::GLOW;
    pub const EMBER: u32 = 0xc83a10 | mat::GLOW;
    pub const IRON: u32 = 0x3c3c42 | mat::METAL;
    pub const IRON_L: u32 = 0x60626a | mat::METAL;
    pub const IRON_D: u32 = 0x26262a | mat::METAL;
    pub const RUST: u32 = 0x6a3a22 | mat::METAL;
    pub const LEATHER: u32 = 0x3a2418 | mat::LEATHER;
    pub const LEATHER_L: u32 = 0x5a3a24 | mat::LEATHER;
    pub const HAFT: u32 = 0x3a2416 | mat::WOOD;
    pub const RED: u32 = 0x7a1410 | mat::CLOTH;
    pub const RED_D: u32 = 0x520c0a | mat::CLOTH;
    pub const SCALE: u32 = 0x22301e | mat::LEATHER;
    pub const SCALE_L: u32 = 0x34462a | mat::LEATHER;
    pub const BELLY: u32 = 0x8a7a4a | mat::LEATHER;
}
use pal::*;

/// Amon's grid and voxel: about three tiles tall, as Mammon (100 x 124 x 80 at 0.025).
pub const AMON_SIZE: (i32, i32, i32) = (100, 124, 80);
pub const AMON_VOXEL: f32 = 0.025;
const MX: f32 = 50.0;
const MZ: f32 = 42.0;

/// Hips, shoulders and neck, and his tail at the small of his back; the axe goes with his right
/// arm, the pelt with his body.
pub const AMON_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [MX - 12.0, 40.0, MZ] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [MX + 12.0, 40.0, MZ] },
    PartDef { name: "leftArm", boxes: &[], pivot: [MX - 32.0, 84.0, MZ] },
    PartDef { name: "rightArm", boxes: &[], pivot: [MX + 32.0, 84.0, MZ] },
    PartDef { name: "head", boxes: &[], pivot: [MX, 92.0, MZ + 4.0] },
    PartDef { name: "tail", boxes: &[], pivot: [MX, 34.0, MZ - 12.0] },
];

/// Shades hide by a voxel's own noise: lighter on the swell, darker in the folds.
fn skin_paint(s: &mut Sculpt) {
    s.paint(|x, y, z, c| {
        if c != SKIN {
            return None;
        }
        let n = nz(x, y, z);
        Some(if n < 0.15 {
            SKIN_L
        } else if n > 0.86 {
            SKIN_D
        } else {
            SKIN
        })
    });
}

/// Paints a line of `c` across the frontmost voxels from (x0, y0) to (x1, y1) (a scar, a crack).
fn front_line(s: &mut Sculpt, d: i32, a: (f32, f32), b: (f32, f32), c: u32, wide: bool) {
    let n = ((b.0 - a.0).abs().max((b.1 - a.1).abs()) * 1.5).ceil().max(1.0) as i32;
    for k in 0..=n {
        let t = k as f32 / n as f32;
        let (x, y) = ((a.0 + (b.0 - a.0) * t).round() as i32, (a.1 + (b.1 - a.1) * t).round() as i32);
        for dx in if wide { -1..=0 } else { 0..=0 } {
            if let Some(z) = (0..d).rev().find(|&z| s.get(x + dx, y, z).is_some()) {
                s.recolor(x + dx, y, z, c);
            }
        }
    }
}

pub fn amon() -> Collected {
    let (w, h, d) = AMON_SIZE;
    let mut s = Sculpt::new(w, h, d);

    // ---- legs: a wolf's, thick-thighed, in iron greaves, on clawed paws
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        let x = MX + side * 12.0;
        let knee = v(x + side * 1.5, 23.0, MZ + 5.0);
        let ankle = v(x + side * 1.0, 9.0, MZ - 2.0);
        limb(&mut s, &[(v(x, 40.0, MZ), 9.5), (knee, 7.2)], SKIN);
        limb(&mut s, &[(knee, 6.6), (ankle, 4.6)], SKIN_D);
        // The greave over the shin and knee, riveted, a spike at the knee.
        limb(&mut s, &[(knee + v(0.0, 1.0, 0.6), 7.2), (ankle + v(0.0, 2.0, 0.8), 5.4)], IRON);
        for k in 0..4 {
            let p = knee.lerp(ankle, 0.15 + k as f32 * 0.22) + v(0.0, 0.0, 6.0);
            s.set(p.x as i32, p.y as i32, p.z as i32 + 1, IRON_L);
        }
        limb(&mut s, &[(knee + v(0.0, 2.0, 6.5), 2.2), (knee + v(0.0, 5.0, 11.0), 0.6)], IRON_L);
        // The paw: a pad and three long toes, black claws.
        ball(&mut s, v(x, 4.5, MZ + 3.0), v(6.5, 4.5, 8.0), SKIN_D);
        ball(&mut s, v(x, 6.5, MZ - 3.0), v(4.0, 4.0, 4.0), SKIN_D);
        for t in [-3.6f32, 0.0, 3.6] {
            let toe = v(x + t, 3.0, MZ + 10.0);
            limb(&mut s, &[(v(x + t * 0.7, 4.0, MZ + 6.0), 2.4), (toe, 1.8)], SKIN_D);
            limb(&mut s, &[(toe + v(0.0, 0.0, 1.0), 1.4), (toe + v(t * 0.15, -2.4, 5.0), 0.4)], CLAW);
        }
        // A shaggy fringe of fur at the top of the greave.
        for k in 0..14 {
            let a = k as f32 / 14.0 * TAU;
            let p = knee + v(a.cos() * 7.5, 4.0, a.sin() * 7.5);
            limb(
                &mut s,
                &[(p, 1.4), (p + v(a.cos() * 1.5, -3.0 - (k % 3) as f32, a.sin() * 1.5), 0.6)],
                if k % 2 == 0 { FUR } else { FUR_D },
            );
        }
    }

    // ---- the body: hips, a great slab of chest, shoulders like a bull's
    s.part(0);
    ball(&mut s, v(MX, 41.0, MZ), v(20.0, 9.0, 13.0), SKIN_D);
    ball(&mut s, v(MX, 54.0, MZ + 2.0), v(18.0, 12.0, 13.0), SKIN);
    ball(&mut s, v(MX, 69.0, MZ + 3.0), v(25.0, 13.0, 15.0), SKIN);
    ball(&mut s, v(MX, 81.0, MZ + 0.5), v(31.0, 9.0, 15.0), SKIN);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(MX + side * 10.5, 72.0, MZ + 12.0), v(11.0, 8.0, 6.0), SKIN);
        // The trapezius, heaped up to the neck.
        ball(&mut s, v(MX + side * 13.0, 86.0, MZ - 1.0), v(12.0, 6.0, 10.0), SKIN);
    }
    // The neck, thick as a bull's.
    s.capsule((MX, 84.0, MZ + 1.0), (MX, 95.0, MZ + 4.0), 10.0, 9.0, SKIN);
    skin_paint(&mut s);
    // The muscle's lines: the cleft of the chest, the belly's rows.
    front_line(&mut s, d, (MX, 78.0), (MX, 48.0), SKIN_D, false);
    for y in [60.0f32, 54.0, 48.0] {
        front_line(&mut s, d, (MX - 9.0, y), (MX + 9.0, y + 0.5), SKIN_D, false);
    }
    for side in [-1.0f32, 1.0] {
        front_line(&mut s, d, (MX + side * 2.0, 65.0), (MX + side * 20.0, 66.0), SKIN_D, false);
    }
    // Scars across his chest and belly, pale and puckered.
    for (a, b) in [((MX - 20.0, 76.0), (MX - 4.0, 58.0)), ((MX + 6.0, 74.0), (MX + 18.0, 64.0)), ((MX - 12.0, 52.0), (MX + 2.0, 46.0))] {
        front_line(&mut s, d, a, b, SCAR, true);
    }
    // The fire in him: a crack down his throat and his breastbone, branching, glowing.
    let mut p = (MX + 0.5, 90.0);
    for k in 0..9 {
        let q = (MX + ((k as f32 * 1.7).sin() * 2.5), p.1 - 3.6);
        front_line(&mut s, d, p, q, if k < 5 { FIRE_HOT } else { FIRE }, k < 4);
        if k % 3 == 1 {
            let side = if k % 2 == 0 { 1.0 } else { -1.0 };
            front_line(&mut s, d, q, (q.0 + side * 6.0, q.1 - 3.0), FIRE, false);
        }
        p = q;
    }
    // ---- the war kilt: studded leather straps over a red loincloth, a belt hung with skulls
    for k in 0..18 {
        let a = k as f32 / 18.0 * TAU;
        let (ca, sa) = (a.cos(), a.sin());
        let top = v(MX + ca * 19.5, 44.0, MZ + sa * 13.5);
        let bottom = v(MX + ca * 23.0, 25.0 + (k % 3) as f32, MZ + sa * 16.0);
        limb(&mut s, &[(top, 3.2), (bottom, 2.9)], if k % 2 == 0 { LEATHER } else { LEATHER_L });
        for j in 0..3 {
            let q = top.lerp(bottom, 0.25 + j as f32 * 0.3) + v(ca * 3.0, 0.0, sa * 3.0);
            s.set(q.x.round() as i32, q.y.round() as i32, q.z.round() as i32, IRON_L);
        }
    }
    for x in (MX - 7.0) as i32..=(MX + 7.0) as i32 {
        for y in 14..=44 {
            let ragged = y < 18 && nz(x, 3, 5) < 0.5;
            if ragged {
                continue;
            }
            let z = (MZ + 15.5 + (44 - y) as f32 * 0.08) as i32;
            let edge = (x as f32 - MX).abs() > 6.0;
            s.set(x, y, z, if edge { RED_D } else { RED });
        }
    }
    s.capsule((MX, 45.0, MZ), (MX, 49.0, MZ), 20.5, 20.5, LEATHER_L);
    s.paint(|x, y, z, c| {
        if c != LEATHER_L || !(45..=49).contains(&y) {
            return None;
        }
        let q = ((x as f32 - MX) / 20.5).powi(2) + ((z as f32 - MZ) / 20.5).powi(2);
        (q > 0.8 && x % 5 == 0 && y == 47).then_some(IRON_L)
    });
    // The buckle: a skull of iron and bone.
    ball(&mut s, v(MX, 47.0, MZ + 20.0), v(4.0, 4.5, 2.5), BONE);
    for side in [-1.0f32, 1.0] {
        s.set((MX + side * 1.6).round() as i32, 48, (MZ + 22.0) as i32, CLAW);
    }
    for x in (MX - 2.0) as i32..=(MX + 2.0) as i32 {
        s.set(x, 44, (MZ + 22.0) as i32, if x % 2 == 0 { BONE_D } else { CLAW });
    }
    // Two skulls hung at his left hip by their jaws.
    for (k, dz) in [(0, 6.0f32), (1, -2.0)] {
        let c = v(MX - 22.0 - k as f32, 34.0 - k as f32 * 3.0, MZ + dz);
        limb(&mut s, &[(c + v(0.0, 9.0, 0.0), 0.8), (c + v(0.0, 3.5, 0.0), 0.8)], LEATHER);
        ball(&mut s, c, v(3.0, 3.4, 3.4), BONE);
        s.set(c.x as i32 - 1, c.y as i32, c.z as i32 + 3, CLAW);
        s.set(c.x as i32 + 1, c.y as i32, c.z as i32 + 3, CLAW);
    }

    // ---- the wolf's pelt: over his shoulders and down his back, its head on his left shoulder,
    // its forelegs knotted on his chest
    for y in 56..=92 {
        let yf = y as f32;
        let t = ((yf - 56.0) / 36.0).clamp(0.0, 1.0);
        let (rx, rz) = (21.0 + t * 13.0, 15.0 + t * 3.0);
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = ((x as f32 - MX) / rx, (z as f32 - MZ + 1.0) / rz);
                let q = dx * dx + dz * dz;
                let behind = dz < 0.15 + t * 0.35 || (yf > 84.0 && dz < 0.55);
                if (0.82..=1.1).contains(&q) && behind {
                    let n = nz(x, y, z);
                    s.set(
                        x,
                        y,
                        z,
                        if n < 0.2 {
                            FUR_L
                        } else if n > 0.8 {
                            FUR_D
                        } else {
                            FUR
                        },
                    );
                }
            }
        }
    }
    // Its ragged hem, hanging in tufts down his back.
    for k in 0..24 {
        let a = std::f32::consts::PI * (0.95 + k as f32 / 24.0 * 1.1);
        let p = v(MX + a.cos() * 22.0, 57.0, MZ + a.sin() * 16.0);
        let len = 6.0 + nz(k, 4, 9) * 10.0;
        limb(&mut s, &[(p, 2.4), (p + v(0.0, -len, -1.0), 0.8)], if k % 3 == 0 { FUR_D } else { FUR });
    }
    // Its forelegs over his shoulders, paws knotted on his chest.
    for side in [-1.0f32, 1.0] {
        let knot = v(MX + side * 3.0, 80.0, MZ + 16.0);
        limb(&mut s, &[(v(MX + side * 20.0, 90.0, MZ + 6.0), 3.6), (v(MX + side * 11.0, 84.0, MZ + 14.0), 3.0), (knot, 2.6)], FUR);
        ball(&mut s, knot + v(side * 1.0, -3.0, 1.0), v(2.6, 2.4, 2.4), FUR_D);
        for c in [-1.0f32, 0.0, 1.0] {
            limb(&mut s, &[(knot + v(side + c * 1.2, -4.5, 2.0), 0.7), (knot + v(side + c * 1.4, -6.5, 3.0), 0.3)], CLAW);
        }
    }
    ball(&mut s, v(MX, 80.0, MZ + 17.0), v(3.2, 2.6, 2.4), LEATHER_L);
    // Its head on his left shoulder, snarling forward, a dead eye glowing low.
    let wolf = v(MX - 26.0, 93.0, MZ + 4.0);
    ball(&mut s, wolf, v(8.0, 6.5, 8.5), FUR);
    limb(&mut s, &[(wolf + v(0.0, -1.0, 6.0), 5.0), (wolf + v(0.0, -2.5, 14.0), 3.0)], FUR);
    ball(&mut s, wolf + v(0.0, -1.0, 14.5), v(2.0, 1.6, 1.6), CLAW);
    limb(&mut s, &[(wolf + v(0.0, -5.0, 6.0), 3.6), (wolf + v(0.0, -6.0, 12.0), 2.2)], FUR_D);
    for k in 0..4 {
        let z = wolf.z + 7.0 + k as f32 * 2.0;
        s.set(wolf.x as i32 - 2, (wolf.y - 3.5) as i32, z as i32, TOOTH);
        s.set(wolf.x as i32 + 2, (wolf.y - 3.5) as i32, z as i32, TOOTH);
    }
    for sx in [-1.0f32, 1.0] {
        limb(&mut s, &[(wolf + v(sx * 4.0, 4.5, -1.0), 2.4), (wolf + v(sx * 5.0, 10.0, -3.0), 0.6)], FUR_D);
        s.set((wolf.x + sx * 3.5) as i32, (wolf.y + 1.0) as i32, (wolf.z + 7.5) as i32, EMBER);
    }
    // The iron pauldron on his right shoulder: three plates and their spikes.
    for k in 0..3 {
        let c = v(MX + 30.0 + k as f32 * 1.5, 88.0 - k as f32 * 4.0, MZ);
        ball(&mut s, c, v(12.0 - k as f32, 6.0, 12.5 - k as f32), if k == 0 { IRON } else { IRON_D });
    }
    for k in 0..4 {
        let a = -0.9 + k as f32 * 0.6;
        let base = v(MX + 31.0 + a.sin() * 4.0, 93.0, MZ + a.cos() * 6.0);
        limb(&mut s, &[(base, 2.6), (base + v(3.0, 9.0 - (k % 2) as f32 * 3.0, a.cos() * 2.0), 0.4)], IRON_L);
    }
    s.paint(|x, y, z, c| (c == IRON && y > 84 && (x + z) % 7 == 0).then_some(RUST));

    // ---- arms: huge, bare and scarred, iron bracers, black claws
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(3 + i as u8);
        let sh = v(MX + side * 32.0, 84.0, MZ);
        let elbow = v(MX + side * 39.0, 64.0, MZ + 4.0);
        let wrist = v(MX + side * 38.0, 47.0, MZ + 10.0);
        ball(&mut s, sh, v(11.0, 10.0, 11.0), SKIN);
        limb(&mut s, &[(sh, 10.5), (elbow, 8.0)], SKIN);
        limb(&mut s, &[(elbow, 7.6), (wrist, 6.2)], SKIN);
        // (The biceps' swell, the forearm's.)
        ball(&mut s, sh.lerp(elbow, 0.45) + v(0.0, 0.0, 4.0), v(8.0, 9.0, 6.0), SKIN);
        ball(&mut s, elbow.lerp(wrist, 0.3) + v(side * 1.5, 0.0, 1.5), v(6.5, 7.0, 6.0), SKIN);
        skin_paint(&mut s);
        // The bracer, studded, and a band of iron above the elbow.
        limb(&mut s, &[(elbow.lerp(wrist, 0.35), 7.0), (wrist + v(0.0, 1.0, 0.0), 6.8)], IRON);
        for k in 0..6 {
            let a = k as f32 / 6.0 * TAU;
            let c = elbow.lerp(wrist, 0.65) + v(a.cos() * 7.2, 0.0, a.sin() * 7.2);
            s.set(c.x.round() as i32, c.y.round() as i32, c.z.round() as i32, IRON_L);
        }
        let band = sh.lerp(elbow, 0.78);
        s.capsule((band.x, band.y + 1.0, band.z), (band.x, band.y - 1.0, band.z), 8.6, 8.4, IRON_D);
        // A scar down the upper arm.
        let sc = sh.lerp(elbow, 0.3) + v(side * 3.0, 0.0, 8.0);
        limb(&mut s, &[(sc, 0.8), (sc + v(side * 1.0, -9.0, 0.5), 0.7)], SCAR);
        let hand = wrist + v(side * 0.5, -6.0, 2.5);
        ball(&mut s, hand, v(6.5, 6.5, 6.5), SKIN_D);
        if side < 0.0 {
            // His left: an open claw.
            for a in [-0.9f32, -0.3, 0.3, 0.9] {
                let base = hand + v(a * 4.0, -4.0, 3.0);
                let tip = base + v(a * 2.0, -6.0, 4.5);
                limb(&mut s, &[(base, 2.2), (tip, 1.5)], SKIN_D);
                limb(&mut s, &[(tip, 1.3), (tip + v(0.0, -3.0, 2.0), 0.4)], CLAW);
            }
            limb(&mut s, &[(hand + v(5.5, -1.0, 2.0), 2.2), (hand + v(8.0, -5.0, 6.5), 1.5)], SKIN_D);
            limb(&mut s, &[(hand + v(8.0, -5.0, 6.5), 1.2), (hand + v(8.5, -7.0, 8.5), 0.4)], CLAW);
        } else {
            // His right: a fist round the axe's haft, its knuckles clawed.
            ball(&mut s, hand + v(0.0, -1.5, 3.5), v(6.4, 5.4, 5.4), SKIN_D);
            for k in 0..4 {
                let c = hand + v(-4.0 + k as f32 * 2.6, 1.0, 8.0);
                s.set(c.x as i32, c.y as i32, c.z as i32, CLAW);
            }
            // The axe: a long haft bound in iron, a great crescent blade, a spike behind.
            let grip = hand + v(1.0, 0.0, 5.0);
            let foot = grip + v(0.0, -26.0, 1.5);
            let top = grip + v(0.0, 58.0, -2.0);
            limb(&mut s, &[(foot, 2.0), (grip, 2.3), (top, 2.3)], HAFT);
            for y in [-20.0f32, 12.0, 30.0, 46.0] {
                let c = grip + v(0.0, y, -y / 58.0 * 2.0);
                s.capsule((c.x, c.y, c.z), (c.x, c.y + 1.5, c.z), 2.9, 2.9, IRON_D);
            }
            ball(&mut s, foot, v(2.6, 3.4, 2.6), IRON);
            ball(&mut s, top + v(0.0, 2.0, 0.0), v(2.8, 3.0, 2.8), IRON);
            limb(&mut s, &[(top + v(0.0, 4.0, 0.0), 2.2), (top + v(0.0, 10.0, 0.5), 0.4)], IRON_L);
            // The blade: in the plane of the haft, forward of it, its edge a crescent.
            let hub = top + v(0.0, -10.0, 0.5);
            for j in -18..=12 {
                for k in 0..=22 {
                    let (jf, kf) = (j as f32, k as f32);
                    // The crescent: its edge bulging forward, its cheeks narrowing to the haft.
                    let reach = 14.0 + 8.0 * (1.0 - (jf + 3.0).powi(2) / 240.0).max(0.0);
                    let near = (jf + 3.0).abs() * 0.25;
                    if kf > reach || kf < 1.0 || (kf < 8.0 && jf.abs() > 7.0 + near + kf * 0.6) {
                        continue;
                    }
                    let edge = kf > reach - 2.2;
                    let col = if edge {
                        if kf > reach - 1.0 {
                            FIRE
                        } else {
                            IRON_L
                        }
                    } else if nz(j, k, 7) > 0.82 {
                        RUST
                    } else if (j + k) % 9 == 0 {
                        IRON_D
                    } else {
                        IRON
                    };
                    for dx in -1..=1 {
                        if dx != 0 && (edge || kf > reach - 4.0) {
                            continue;
                        }
                        s.set(hub.x as i32 + dx, hub.y as i32 + j, hub.z as i32 + k, col);
                    }
                }
            }
            // The spike behind the haft.
            limb(&mut s, &[(hub + v(0.0, -2.0, -2.0), 3.0), (hub + v(0.0, -4.0, -12.0), 0.5)], IRON_D);
        }
    }

    // ---- the head: a raven's, black-feathered, ragged-crested, horned, ember-eyed, its great
    // beak lined with a dog's teeth and the fire between its halves
    s.part(5);
    let hc = v(MX, 104.0, MZ + 6.0);
    ball(&mut s, hc, v(11.0, 11.0, 12.0), FEATHER);
    ball(&mut s, hc + v(0.0, -5.0, 3.0), v(10.0, 7.0, 10.0), FEATHER);
    // The hackles: a ruff of long feathers about the neck, hanging down over the shoulders.
    for k in 0..26 {
        let a = k as f32 / 26.0 * TAU;
        let (ca, sa) = (a.cos(), a.sin());
        if sa > 0.75 {
            continue; // (not over the throat: the fire shows there)
        }
        let p = v(MX + ca * 10.5, 97.0, MZ + 5.0 + sa * 10.0);
        let len = 6.0 + nz(k, 2, 3) * 5.0;
        limb(&mut s, &[(p, 2.2), (p + v(ca * 4.0, -len, sa * 3.5), 0.6)], if k % 3 == 0 { GLOSS } else { FEATHER });
    }
    // The crest: ragged feathers swept back off the crown.
    for k in 0..7 {
        let x = MX - 6.0 + k as f32 * 2.0;
        let lift = 4.0 + (3.0 - (k as f32 - 3.0).abs()) * 1.5;
        let p = v(x, 112.0, MZ + 8.0 - (k as f32 - 3.0).abs());
        limb(
            &mut s,
            &[(p, 1.8), (p + v((x - MX) * 0.3, lift, -9.0), 1.0), (p + v((x - MX) * 0.5, lift + 1.0, -16.0), 0.4)],
            if k % 2 == 0 { FEATHER } else { GLOSS },
        );
    }
    // The gloss of the feathers on the crown and down the back of the head.
    s.paint(|x, y, z, c| {
        if c != FEATHER || y < 100 {
            return None;
        }
        let row = (z as f32 / 2.5).floor() as i32;
        let fx = ((x as f32 + if row % 2 == 0 { 0.0 } else { 1.3 }) / 2.6).fract();
        Some(if fx < 0.18 {
            FEATHER
        } else if nz(x, y, z) > 0.7 {
            GLOSS_L
        } else if y > 108 {
            GLOSS
        } else {
            FEATHER
        })
    });
    // Two short black horns sweeping back from the brow.
    for side in [-1.0f32, 1.0] {
        let base = hc + v(side * 7.0, 7.0, 3.0);
        limb(&mut s, &[(base, 2.8), (base + v(side * 4.0, 5.0, -5.0), 1.8), (base + v(side * 6.0, 7.0, -12.0), 0.5)], HORN);
    }
    // The eyes: embers under a heavy brow.
    for side in [-1.0f32, 1.0] {
        let e = hc + v(side * 8.5, 2.0, 7.5);
        ball(&mut s, e, v(1.3, 1.1, 1.2), EYE);
        limb(&mut s, &[(e + v(-side * 3.0, 3.0, 1.0), 1.6), (e + v(side * 2.0, 2.4, 1.5), 1.2)], FEATHER);
    }
    // The fire in his throat, showing between the beak's halves.
    ball(&mut s, hc + v(0.0, -4.5, 13.0), v(3.6, 2.6, 7.0), FIRE_HOT);
    // The beak: great and hooked, the upper half over the lower, a gape between them.
    let root = hc + v(0.0, -1.0, 10.0);
    limb(&mut s, &[(root + v(0.0, 1.0, 0.0), 6.0), (root + v(0.0, -1.0, 10.0), 4.0), (root + v(0.0, -4.0, 20.0), 1.8)], BEAK);
    limb(&mut s, &[(root + v(0.0, -4.0, 20.0), 1.8), (root + v(0.0, -8.0, 23.0), 0.6)], BEAK_L);
    limb(&mut s, &[(root + v(0.0, -7.5, 0.0), 4.4), (root + v(0.0, -8.5, 10.0), 3.0), (root + v(0.0, -9.5, 17.0), 1.0)], BEAK);
    s.paint(|x, y, z, c| {
        if c != BEAK || z < (root.z + 4.0) as i32 {
            return None;
        }
        if (x as f32 - MX).abs() < 0.6 && y as f32 > root.y + 2.0 {
            Some(BEAK_L)
        } else {
            None
        }
    });
    // A dog's teeth along the upper half's edge, and up from the lower.
    for k in 0..6 {
        let z = root.z + 3.0 + k as f32 * 2.4;
        let y = root.y - 3.5 - k as f32 * 0.45;
        for sx in [-2.0f32, 2.0] {
            let w = (1.0 - k as f32 / 8.0) * sx;
            s.set((MX + w).round() as i32, y.round() as i32, z.round() as i32, TOOTH);
            s.set((MX + w).round() as i32, (y - 1.0).round() as i32, z.round() as i32, TOOTH);
            if k % 2 == 1 {
                s.set((MX + w * 0.8).round() as i32, (y - 2.6).round() as i32, (z + 0.5).round() as i32, TOOTH);
            }
        }
    }

    // ---- the serpent's tail, coiling behind him on the floor, scaled, a bone spike at its tip
    s.part(6);
    let knots = [
        (v(MX, 34.0, MZ - 12.0), 7.0),
        (v(MX + 3.0, 20.0, MZ - 23.0), 6.2),
        (v(MX + 9.0, 7.0, MZ - 31.0), 5.4),
        (v(MX + 19.0, 4.5, MZ - 36.0), 4.6),
        (v(MX + 30.0, 4.0, MZ - 31.0), 3.8),
        (v(MX + 35.0, 5.0, MZ - 21.0), 2.8),
        (v(MX + 32.0, 7.0, MZ - 13.0), 1.8),
        (v(MX + 26.0, 9.0, MZ - 9.0), 1.0),
    ];
    limb(&mut s, &knots, SCALE);
    s.paint(|x, y, z, c| {
        if c != SCALE {
            return None;
        }
        let row = ((x + z) as f32 / 2.2).floor() as i32;
        let n = nz(x, y, z);
        // The pale belly underneath, the scales in rows above, a gloss here and there.
        Some(if y <= 3 || (y < 6 && n < 0.4) {
            BELLY
        } else if (row + y / 2) % 3 == 0 {
            SCALE_L
        } else if n > 0.9 {
            GLOSS
        } else {
            SCALE
        })
    });
    let tip = knots[knots.len() - 1].0;
    limb(&mut s, &[(tip, 1.4), (tip + v(-5.0, 3.0, 3.0), 0.4)], BONE);
    // A ridge of dark spines down its back.
    for (k, w) in knots.windows(2).enumerate() {
        let m = w[0].0.lerp(w[1].0, 0.5) + v(0.0, w[0].1 * 0.9, 0.0);
        limb(&mut s, &[(m, 1.2), (m + v(0.0, 2.5 - k as f32 * 0.2, -1.0), 0.3)], HORN);
    }

    s.finish(0.03)
}

/// A raven's grid and voxel: a great bird with its wings spread, hovering at about a hero's
/// height (its grid's floor is the floor it flies over).
pub const RAVEN_SIZE: (i32, i32, i32) = (80, 64, 54);
pub const RAVEN_VOXEL: f32 = 0.025;
const RX: f32 = 40.0;
const RY: f32 = 44.0;
const RZ: f32 = 26.0;

/// Its wings hinge where they meet its body.
pub const RAVEN_RIG: &[PartDef] = &[
    PartDef { name: "leftWing", boxes: &[], pivot: [RX - 6.0, RY + 2.0, RZ] },
    PartDef { name: "rightWing", boxes: &[], pivot: [RX + 6.0, RY + 2.0, RZ] },
];

/// A raven of Amon: a great black bird, wings spread, smouldering at the tips, ember-eyed, its
/// talons hanging; it hovers in its grid at about the hero's height.
pub fn raven() -> Collected {
    let (w, h, d) = RAVEN_SIZE;
    let mut s = Sculpt::new(w, h, d);
    // ---- the wings: coverts over the arm, long fingered primaries at the tip, their ends smouldering
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        let span = 32.0;
        for k in 0..=(span as i32 * 2) {
            let u = k as f32 / (span * 2.0);
            let x = RX + side * (6.0 + u * span);
            let y = RY + 2.0 + u * 7.0 - (u - 0.8).max(0.0) * 14.0;
            let lead = RZ + 6.0 - u * 3.0;
            let trail = RZ - 9.0 + u * 2.0 + (x * 0.9).sin().abs() * 1.6;
            if u > 0.62 {
                continue; // (the primaries, below)
            }
            for z in trail.floor() as i32..=lead.ceil() as i32 {
                let t = (lead - z as f32) / (lead - trail).max(1.0);
                let c = if t < 0.12 {
                    FEATHER
                } else if t < 0.45 {
                    if (z + k) % 3 == 0 {
                        GLOSS_L
                    } else {
                        GLOSS
                    }
                } else if t > 0.92 && nz(k, z, 3) < 0.5 {
                    EMBER
                } else {
                    FEATHER
                };
                s.set(x.round() as i32, y.round() as i32, z, c);
                if t < 0.5 {
                    s.set(x.round() as i32, y.round() as i32 - 1, z, FEATHER);
                }
            }
        }
        // The primaries, fingered, fanning out from the wrist, their tips smouldering.
        let wrist = v(RX + side * (6.0 + 0.6 * span), RY + 2.0 + 0.6 * 7.0, RZ + 3.0);
        for k in 0..6 {
            let a = k as f32 / 5.0;
            let tip = v(RX + side * (6.0 + span * (1.0 - a * 0.12)), RY + 7.0 - a * 3.0, RZ + 5.0 - a * 14.0);
            let base = wrist + v(-side * a * 2.0, 0.0, -a * 6.0);
            limb(&mut s, &[(base, 1.6), (base.lerp(tip, 0.8), 1.2), (tip, 0.6)], if k % 2 == 0 { FEATHER } else { GLOSS });
            s.set(tip.x.round() as i32, tip.y.round() as i32, tip.z.round() as i32, EMBER);
        }
    }
    // ---- the body, the tail, the head
    s.part(0);
    ball(&mut s, v(RX, RY, RZ - 1.0), v(7.0, 6.5, 12.0), FEATHER);
    s.capsule((RX, RY + 1.5, RZ + 7.0), (RX, RY + 4.0, RZ + 11.0), 5.8, 5.2, FEATHER);
    for k in 0..7 {
        let a = (k as f32 - 3.0) / 3.0;
        limb(
            &mut s,
            &[(v(RX + a * 2.0, RY - 1.0, RZ - 10.0), 1.8), (v(RX + a * 7.0, RY - 2.0 - a.abs() * 2.0, RZ - 22.0 + a.abs() * 3.0), 1.5)],
            if k % 2 == 0 { FEATHER } else { GLOSS },
        );
        s.set((RX + a * 7.0).round() as i32, (RY - 2.0 - a.abs() * 2.0).round() as i32, (RZ - 22.0 + a.abs() * 3.0).round() as i32, EMBER);
    }
    let head = v(RX, RY + 5.0, RZ + 13.0);
    ball(&mut s, head, v(5.4, 5.2, 5.8), FEATHER);
    // Hackles under its throat.
    for k in 0..7 {
        let x = RX - 3.0 + k as f32;
        limb(&mut s, &[(v(x, RY + 1.0, RZ + 12.0), 1.0), (v(x, RY - 3.0 - (k % 3) as f32, RZ + 12.5), 0.4)], FEATHER);
    }
    s.paint(|x, y, z, c| {
        if c != FEATHER {
            return None;
        }
        let row = (z as f32 / 2.2).floor() as i32;
        let fx = ((x as f32 + if row % 2 == 0 { 0.0 } else { 1.2 }) / 2.4).fract();
        Some(if fx < 0.16 {
            FEATHER
        } else if y as f32 > RY + 3.0 && nz(x, y, z) > 0.6 {
            GLOSS_L
        } else if y as f32 > RY + 1.0 {
            GLOSS
        } else {
            FEATHER
        })
    });
    // The beak, heavy and hooked, the fire in its throat; the eyes, embers.
    limb(&mut s, &[(head + v(0.0, 0.5, 4.0), 2.8), (head + v(0.0, -0.5, 9.5), 1.2), (head + v(0.0, -2.2, 11.0), 0.5)], BEAK);
    limb(&mut s, &[(head + v(0.0, -2.2, 4.0), 1.9), (head + v(0.0, -2.6, 8.0), 0.6)], BEAK_L);
    ball(&mut s, head + v(0.0, -1.3, 5.0), v(1.2, 0.8, 2.0), FIRE);
    for side in [-1.0f32, 1.0] {
        s.set((RX + side * 4.0).round() as i32, (head.y + 1.0) as i32, (head.z + 2.5) as i32, EYE);
        s.set((RX + side * 4.0).round() as i32, (head.y + 1.0) as i32, (head.z + 3.5) as i32, EYE);
    }
    // The talons, hanging.
    for side in [-1.0f32, 1.0] {
        let hip = v(RX + side * 3.0, RY - 5.0, RZ + 1.0);
        let foot = hip + v(side * 0.5, -8.0, 2.0);
        limb(&mut s, &[(hip, 1.6), (foot, 1.0)], BEAK);
        for t in [-1.2f32, 0.0, 1.2] {
            limb(&mut s, &[(foot, 0.8), (foot + v(t * 1.5, -2.5, 2.5), 0.4)], CLAW);
        }
    }
    s.finish(0.03)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amon_towers_over_the_hero_and_his_ravens_fly_at_his_height() {
        let a = amon();
        let top = a.voxels.iter().map(|v| v.y).max().unwrap() as f32 * AMON_VOXEL;
        assert!(top > 2.7 && top < 3.4, "{top} tiles tall");
        assert!(a.voxels.len() > 8000, "{} voxels", a.voxels.len());
        for part in 0..=AMON_RIG.len() as u8 {
            assert!(a.voxels.iter().any(|v| v.part == part), "part {part}");
        }
        let glow = |c: u32| c & 0xff00_0000 == mat::GLOW;
        assert!(a.voxels.iter().filter(|v| glow(v.c)).count() > 60, "the fire in him shows");
        assert!(a.voxels.iter().all(|v| v.x >= 0 && v.x < AMON_SIZE.0 && v.z >= 0 && v.z < AMON_SIZE.2));
        let r = raven();
        let low = r.voxels.iter().map(|v| v.y).min().unwrap() as f32 * RAVEN_VOXEL;
        let top = r.voxels.iter().map(|v| v.y).max().unwrap() as f32 * RAVEN_VOXEL;
        assert!(low > 0.6 && top > 1.0 && top < 2.2, "{low}..{top}: aloft");
        for part in 0..=RAVEN_RIG.len() as u8 {
            assert!(r.voxels.iter().any(|v| v.part == part), "raven part {part}");
        }
    }
}
