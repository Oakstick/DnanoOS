//! The gaki, the hungry ghosts of the Rotting Feast (REALMS.md §5), as the gaki zōshi scrolls
//! paint them: starved to the bone and pale as ash, their bellies swollen tight as drums over legs
//! and arms like dry sticks; a throat as thin as a needle, so nothing they eat goes down; a mouth no
//! bigger than a pinhole, where food turns to fire (an ember glows in it); eyes sunk deep in a
//! skull of a head, burning faintly; a few long black hairs; a rag about the hips. They stoop,
//! head thrust forward, long-fingered hands reaching for whoever smells of food.
//!
//! Faces +z, stands in the middle of its grid, sculpted in the parts its rig swings.

use glam::Vec3;

use super::carve::*;
use super::characters::{mat, Sculpt};
use super::voxels::{Collected, PartDef};

/// The model's grid, and the size of one of its voxels in tiles: about one and a quarter tall.
pub const GAKI_SIZE: (i32, i32, i32) = (52, 100, 52);
pub const GAKI_VOXEL: f32 = 0.0125;
const CX: f32 = 25.5;
const CZ: f32 = 23.0;

/// The hips (the legs' joints), the shoulders, the root of the neck.
const HIP: f32 = 42.0;
const SHOULDER: (f32, f32, f32) = (7.0, 69.5, CZ + 4.0);
const NECK: (f32, f32) = (73.0, CZ + 6.0);

pub const GAKI_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [CX - 5.0, HIP, CZ] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [CX + 5.0, HIP, CZ] },
    PartDef { name: "leftArm", boxes: &[], pivot: [CX - SHOULDER.0, SHOULDER.1, SHOULDER.2] },
    PartDef { name: "rightArm", boxes: &[], pivot: [CX + SHOULDER.0, SHOULDER.1, SHOULDER.2] },
    PartDef { name: "head", boxes: &[], pivot: [CX, NECK.0, NECK.1] },
];

mod pal {
    use super::mat;
    pub const SKIN: u32 = 0xd2cab4 | mat::SKIN;
    pub const SKIN_L: u32 = 0xe4dcc8 | mat::SKIN;
    pub const SKIN_D: u32 = 0xa49a84 | mat::SKIN;
    pub const SKIN_DD: u32 = 0x7a705e | mat::SKIN;
    pub const BELLY: u32 = 0xe2d8c6 | mat::SLIME;
    pub const BELLY_D: u32 = 0xc4b8a6 | mat::SLIME;
    pub const VEIN: u32 = 0x8a7c9e | mat::SKIN;
    pub const RIB: u32 = 0xeae2d0 | mat::SKIN;
    pub const SOCKET: u32 = 0x161014 | mat::SKIN;
    pub const EYE: u32 = 0xeef4b0 | mat::GLOW;
    pub const EMBER: u32 = 0xff6a1e | mat::GLOW;
    pub const EMBER_L: u32 = 0xffd070 | mat::GLOW;
    pub const LIP: u32 = 0x5a3a3a | mat::SKIN;
    pub const HAIR: u32 = 0x161210 | mat::FUR;
    pub const HAIR_L: u32 = 0x2e2824 | mat::FUR;
    pub const NAIL: u32 = 0x4a3a28 | mat::IVORY;
    pub const RAG: u32 = 0x8c846e | mat::CLOTH;
    pub const RAG_D: u32 = 0x5e5848 | mat::CLOTH;
}
use pal::*;

/// A thin limb along knots, with a knob of bone at every inner knot.
fn sticks(s: &mut Sculpt, knots: &[(Vec3, f32)], knob: f32, c: u32) {
    limb(s, knots, c);
    for k in &knots[1..knots.len() - 1] {
        ball(s, k.0, Vec3::splat(k.1 + knob), c);
    }
}

/// A hand at `wrist`, its long fingers reaching along `dir` and curling down, dark nails at the tips.
fn hand(s: &mut Sculpt, wrist: Vec3, dir: Vec3, side: f32) {
    let across = Vec3::new(side, 0.0, 0.0);
    ball(s, wrist + dir * 1.5, Vec3::new(1.9, 1.3, 2.0), SKIN);
    for k in 0..4 {
        let spread = (k as f32 - 1.5) * 0.9;
        let base = wrist + dir * 2.6 + across * spread * 0.9;
        let mid = base + dir * 3.4 + across * spread * 0.6 + Vec3::new(0.0, -1.2, 0.0);
        let tip = mid + dir * 2.6 + Vec3::new(0.0, -2.6, 0.0) + across * spread * 0.3;
        limb(s, &[(base, 0.7), (mid, 0.6), (tip, 0.45)], SKIN);
        ball(s, tip, Vec3::splat(0.6), NAIL);
    }
    // The thumb, opposed.
    let base = wrist + dir * 1.8 - across * 1.6;
    limb(s, &[(base, 0.7), (base + dir * 2.4 - across * 0.6 + Vec3::new(0.0, -1.4, 0.0), 0.5)], SKIN);
}

/// A foot: long bones splayed, clawed toes.
fn foot(s: &mut Sculpt, ankle: Vec3, side: f32) {
    ball(s, ankle + Vec3::new(0.0, -1.0, 1.0), Vec3::new(1.8, 1.4, 2.4), SKIN);
    for k in 0..4 {
        let spread = (k as f32 - 1.5) * 0.9 + side * 0.2;
        let a = ankle + Vec3::new(spread * 0.6, -1.6, 2.0);
        let b = a + Vec3::new(spread * 0.5, -0.8, 3.6);
        limb(s, &[(a, 0.75), (b, 0.55)], SKIN);
        ball(s, b + Vec3::new(0.0, -0.2, 0.5), Vec3::splat(0.55), NAIL);
    }
}

pub fn gaki() -> Collected {
    let (w, h, d) = GAKI_SIZE;
    let mut s = Sculpt::new(w, h, d);

    // ---- the legs: dry sticks, knees like knots, splayed clawed feet
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        let hip = v(CX + side * 5.0, HIP, CZ);
        let knee = v(CX + side * 7.5, 23.0, CZ + 3.5);
        let ankle = v(CX + side * 6.5, 3.6, CZ + 0.5);
        sticks(&mut s, &[(hip, 2.6), (knee, 1.8), (ankle, 1.5)], 1.1, SKIN);
        foot(&mut s, ankle, side);
    }

    // ---- the body: a rag about the hips, the belly swollen tight, the ribs, the hunched back
    s.part(0);
    // Pelvis.
    ball(&mut s, v(CX, HIP + 1.0, CZ), v(6.6, 4.0, 4.6), SKIN_D);
    // The belly, a drum: huge, round, tight and glossy, veined.
    let belly = v(CX, 51.0, CZ + 7.0);
    ball(&mut s, belly, v(12.0, 11.5, 13.0), BELLY);
    // The ribcage above it: narrow, starved, every rib showing; the hunch of the back.
    for y in 56..=72 {
        let t = (y - 56) as f32 / 16.0;
        let lean = t * 3.0;
        let (rx, rz) = (6.4 - 1.2 * t, 5.2 - 0.6 * t);
        let zc = CZ + 2.0 + lean;
        for z in (zc - rz - 1.0) as i32..=(zc + rz + 1.0) as i32 {
            for x in (CX - rx - 1.0) as i32..=(CX + rx + 1.0) as i32 {
                let (dx, dz) = ((x as f32 - CX) / rx, (z as f32 - zc) / rz);
                if dx * dx + dz * dz <= 1.0 {
                    let rib = y % 3 == 0 && dz > -0.5 && dx.abs() > 0.15;
                    let groove = y % 3 == 1 && dz > -0.2;
                    s.set(
                        x,
                        y,
                        z,
                        if rib {
                            RIB
                        } else if groove {
                            SKIN_D
                        } else {
                            SKIN
                        },
                    );
                }
            }
        }
    }
    // The hunch, and the knuckles of the spine down the back.
    ball(&mut s, v(CX, 68.0, CZ - 1.0), v(6.0, 5.5, 4.0), SKIN);
    for k in 0..9 {
        let y = 48.0 + k as f32 * 2.8;
        let z = CZ - 3.6 - (k as f32 - 5.0).abs().mul_add(-0.35, 1.8);
        ball(&mut s, v(CX, y, z), v(1.1, 1.0, 1.1), SKIN_L);
    }
    // Shoulder blades standing out, and collarbones.
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(CX + side * 4.5, 66.0, CZ - 2.4), v(2.6, 3.6, 1.2), SKIN_L);
        limb(&mut s, &[(v(CX + side * 1.0, 72.5, CZ + 6.0), 0.9), (v(CX + side * 6.5, 70.8, CZ + 4.5), 0.9)], RIB);
        ball(&mut s, v(CX + side * 6.8, 69.6, CZ + 4.0), v(2.4, 2.4, 2.5), SKIN);
    }
    // The rag: a tattered strip about the hips, its ends hanging.
    for z in 0..d {
        for x in 0..w {
            for y in 37..=44 {
                let (dx, dz) = ((x as f32 - CX) / 7.6, (z as f32 - CZ - 0.6) / 5.6);
                let r = dx * dx + dz * dz;
                if (0.72..=1.0).contains(&r) && !(y < 39 && nz(x, 3, z) < 0.45) {
                    s.set(x, y, z, if (x + y) % 4 == 0 { RAG_D } else { RAG });
                }
            }
        }
    }
    for (dx, len) in [(-2.0, 8.0), (1.5, 11.0), (4.0, 6.0)] {
        let top = v(CX + dx, 38.0, CZ + 6.2);
        limb(&mut s, &[(top, 1.3), (top + v(0.4, -len, 0.8), 0.9)], RAG);
    }

    // ---- the arms: long, thin, bony, reaching forward with clawed fingers
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(3 + i as u8);
        let sh = v(CX + side * SHOULDER.0, SHOULDER.1, SHOULDER.2);
        let elbow = v(CX + side * 12.5, 56.0, CZ + 1.0);
        let wrist = v(CX + side * 9.5, 50.0, CZ + 17.5);
        sticks(&mut s, &[(sh, 2.0), (elbow, 1.4), (wrist, 1.2)], 1.0, SKIN);
        let dir = (v(CX + side * 6.5, 44.0, CZ + 26.0) - wrist).normalize();
        hand(&mut s, wrist, dir, side);
    }

    // ---- the head: the needle neck thrust forward, a skull of a face, the pinhole mouth aglow
    s.part(5);
    let neck_top = v(CX, 80.0, CZ + 12.0);
    limb(&mut s, &[(v(CX, NECK.0 - 1.0, NECK.1), 1.6), (v(CX, 77.0, CZ + 9.0), 1.15), (neck_top, 1.25)], SKIN);
    // A few rings on the neck, the windpipe's knuckles.
    for k in 0..3 {
        let p = v(CX, 75.0 + k as f32 * 1.6, CZ + 8.0 + k as f32 * 1.3);
        ball(&mut s, p + v(0.0, 0.0, 0.6), v(1.3, 0.5, 1.2), SKIN_D);
    }
    let head = v(CX, 86.0, CZ + 15.0);
    // The cranium, high and narrow; the jaw beneath, sharp-chinned; the cheekbones jutting.
    ball(&mut s, head + v(0.0, 1.5, -0.8), v(5.4, 6.2, 5.8), SKIN);
    ball(&mut s, head + v(0.0, -3.4, 1.8), v(3.6, 3.4, 3.6), SKIN);
    ball(&mut s, head + v(0.0, -6.0, 3.2), v(1.5, 1.5, 1.6), SKIN);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, head + v(side * 3.6, -1.0, 3.4), v(1.6, 1.3, 1.4), SKIN_L);
        // Hollow cheeks.
        ball(&mut s, head + v(side * 3.4, -3.4, 3.0), v(0.9, 1.4, 0.9), SKIN_DD);
        // Ears, long and thin.
        limb(&mut s, &[(head + v(side * 5.2, 0.5, -0.5), 0.9), (head + v(side * 6.6, 3.0, -1.2), 0.6)], SKIN_D);
    }
    // The brow ridge, and the deep sockets under it with a faint light in each.
    for x in -4..=4 {
        let p = head + v(x as f32, 1.6, 0.0);
        if let Some(z) = front(&s, p.x.round() as i32, p.y.round() as i32, d) {
            s.set(p.x.round() as i32, p.y.round() as i32, z + 1, SKIN_L);
        }
    }
    for side in [-1.0f32, 1.0] {
        let eye = head + v(side * 2.1, -0.2, 0.0);
        let (x, y) = (eye.x.round() as i32, eye.y.round() as i32);
        for (dx, dy) in [(0, 0), (1, 0), (-1, 0), (0, -1), (1, -1), (-1, -1), (0, 1)] {
            if let Some(z) = front(&s, x + dx, y + dy, d) {
                s.set(x + dx, y + dy, z, SOCKET);
                s.clear(x + dx, y + dy, z);
                if let Some(z2) = front(&s, x + dx, y + dy, d) {
                    s.recolor(x + dx, y + dy, z2, SOCKET);
                }
            }
        }
        if let Some(z) = front(&s, x, y, d) {
            s.recolor(x, y, z, EYE);
        }
    }
    // The nose: two slits. The mouth: a puckered ring no bigger than a pinhole, an ember in it.
    let nose = head + v(0.0, -2.4, 0.0);
    for dx in [-1, 1] {
        face(&mut s, nose.x.round() as i32 + dx, nose.y.round() as i32, d, 0, SKIN_DD);
    }
    let mouth = head + v(0.0, -5.0, 0.0);
    let (mx, my) = (mouth.x.round() as i32, mouth.y.round() as i32);
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        face(&mut s, mx + dx, my + dy, d, 1, LIP);
    }
    face(&mut s, mx, my, d, 0, EMBER);
    face(&mut s, mx, my, d, 1, EMBER_L);
    // A few long black hairs from the crown, hanging lank down the back and the sides.
    for k in 0..11 {
        let a = -1.2 + k as f32 * 0.24;
        let root = head + v(a.sin() * 4.6, 6.2 - a.abs() * 0.8, -1.5 - a.cos() * 2.4);
        let len = 9.0 + nz(k, 7, 3) * 8.0;
        let swing = v(a.sin() * 2.2, 0.0, -2.0 - nz(k, 2, 9) * 2.0);
        let mid = root + swing * 0.6 + v(0.0, -len * 0.5, 0.0);
        let tip = root + swing + v(0.0, -len, 0.0);
        limb(&mut s, &[(root, 0.55), (mid, 0.5), (tip, 0.4)], if k % 3 == 0 { HAIR_L } else { HAIR });
    }

    // ---- shading: the undersides in shadow, veins on the belly
    let mut under = Vec::new();
    for y in 3..h {
        for z in 0..d {
            for x in 0..w {
                if s.get(x, y, z) == Some(SKIN) && s.get(x, y - 1, z).is_none() {
                    under.push((x, y, z));
                }
            }
        }
    }
    for (x, y, z) in under {
        s.recolor(x, y, z, SKIN_D);
    }
    s.paint(|x, y, z, c| {
        if c != BELLY {
            return None;
        }
        let q = v(x as f32, y as f32, z as f32) - belly;
        let vein = ((q.x * 0.55 + (q.y * 0.4).sin() * 2.0).sin() * (q.z * 0.3 + q.y * 0.2).cos()).abs() < 0.06;
        if vein && q.z > 0.0 {
            Some(VEIN)
        } else if q.y < -6.0 {
            Some(BELLY_D)
        } else if q.x.abs() < 0.8 && q.y.abs() < 0.8 {
            Some(SKIN_DD)
        } else {
            None
        }
    });
    s.finish(0.03)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gaki_stands_in_the_middle_of_its_grid_its_joints_on_the_parts_they_swing() {
        let c = gaki();
        let size = GAKI_SIZE;
        assert!(c.voxels.len() > 2000, "{} voxels", c.voxels.len());
        assert!(c.voxels.iter().map(|v| v.y).min().is_some_and(|y| y <= 1), "on its feet");
        let edge = c.voxels.iter().find(|v| v.x <= 0 || v.x >= size.0 - 1 || v.y >= size.1 - 1 || v.z <= 0 || v.z >= size.2 - 1);
        assert!(edge.is_none(), "a voxel against the grid's side at {:?}", edge.map(|v| (v.x, v.y, v.z)));
        let floor: Vec<_> = c.voxels.iter().filter(|v| v.y <= 3).collect();
        let mx = floor.iter().map(|v| v.x as f32).sum::<f32>() / floor.len() as f32;
        assert!((mx - (size.0 - 1) as f32 / 2.0).abs() < 2.0, "footprint centred at x {mx}");
        for (i, def) in GAKI_RIG.iter().enumerate() {
            let mine: Vec<_> = c.voxels.iter().filter(|v| v.part as usize == i + 1).collect();
            assert!(mine.len() > 40, "{}: {} voxels", def.name, mine.len());
            let p = Vec3::from(def.pivot);
            let (lo, hi) = mine.iter().fold((Vec3::MAX, Vec3::MIN), |(lo, hi), v| {
                let q = Vec3::new(v.x as f32, v.y as f32, v.z as f32);
                (lo.min(q), hi.max(q))
            });
            assert!(p.cmpge(lo - 3.0).all() && p.cmple(hi + 3.0).all(), "{}: its joint {p} is off it ({lo} to {hi})", def.name);
        }
        // The ember in its pinhole mouth, and its eyes, burn.
        let glow = c.voxels.iter().filter(|v| v.c & 0xff00_0000 == mat::GLOW).count();
        assert!(glow >= 3, "{glow} glowing voxels");
    }

    /// The model's sheet (front, side, back / top, two three-quarter views), into VH_PREVIEW_DIR.
    #[test]
    #[ignore]
    fn preview_gaki() {
        println!("{}", super::super::preview::sheet(&gaki(), "gaki"));
    }
}
