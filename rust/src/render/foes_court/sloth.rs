//! Sloth's court's own (PRINCES.md §4): Belphegor, the prince of Sloth, and the nightmares that
//! come out of his dream. Each faces +z, stands in the middle of its grid and is sculpted in the
//! parts its rig swings.
//!
//! Belphegor is a huge, bloated demon two and a half times the hero's height, in a grey night-robe
//! striped in slate blue, tied with a sash under a belly like a feather bed. His skin is ashen
//! lilac; his face jowled and heavy-lidded, its eyes two dim slits of light; a long grey beard
//! falls over his chest and belly; ram's horns curl from his temples, and a slate-blue nightcap
//! sits between them, flopping over to one side, its tassel swinging. His feet are bare, and every
//! nail of his hands and feet is long and yellowed. His right hand hangs open, clawed, for his
//! swat; in his left he carries an hourglass of brass and glass, its sand stopped.
//!
//! A nightmare is a dream-wisp: a hooded shroud of violet dark with a mare's pale skull in the
//! hood and pink light in its sockets, thin bony arms out of tattered sleeves, and no legs, only
//! two trailing wisps that fade into a glow.

use std::f32::consts::PI;

use glam::Vec3;

use crate::render::carve::{ball, limb, nz, v};
use crate::render::characters::{mat, Sculpt};
use crate::render::voxels::{Collected, PartDef};

mod pal {
    use super::mat;
    pub const SKIN: u32 = 0x8e8aa4 | mat::SKIN;
    pub const SKIN_L: u32 = 0xa8a4bc | mat::SKIN;
    pub const SKIN_D: u32 = 0x6a6680 | mat::SKIN;
    pub const LID: u32 = 0x55516a | mat::SKIN;
    pub const ROBE: u32 = 0x737882 | mat::CLOTH;
    pub const ROBE_L: u32 = 0x8c919c | mat::CLOTH;
    pub const ROBE_D: u32 = 0x50555f | mat::CLOTH;
    pub const STRIPE: u32 = 0x56607e | mat::CLOTH;
    pub const STRIPE_D: u32 = 0x404862 | mat::CLOTH;
    pub const TRIM: u32 = 0xb8b0a0 | mat::CLOTH;
    pub const SASH: u32 = 0x3e4460 | mat::CLOTH;
    pub const CAP: u32 = 0x5a6890 | mat::CLOTH;
    pub const CAP_D: u32 = 0x46527a | mat::CLOTH;
    pub const BRIM: u32 = 0xc8c0b0 | mat::CLOTH;
    pub const TASSEL: u32 = 0xd8c890 | mat::CLOTH;
    pub const BEARD: u32 = 0xb4b2ac | mat::FUR;
    pub const BEARD_L: u32 = 0xd2d0ca | mat::FUR;
    pub const BEARD_D: u32 = 0x86847e | mat::FUR;
    pub const HORN: u32 = 0x6a5e50 | mat::IVORY;
    pub const HORN_L: u32 = 0x948470 | mat::IVORY;
    pub const HORN_TIP: u32 = 0xd8ccb0 | mat::IVORY;
    pub const NAIL: u32 = 0x9a8a58 | mat::IVORY;
    pub const NAIL_TIP: u32 = 0x3a3220 | mat::IVORY;
    pub const EYE: u32 = 0xe8d870 | mat::GLOW;
    pub const NOSTRIL: u32 = 0x3a3044 | mat::SKIN;
    pub const BRASS: u32 = 0xb08a40 | mat::GOLD;
    pub const BRASS_L: u32 = 0xd8b060 | mat::GOLD;
    pub const BRASS_D: u32 = 0x7a6028 | mat::GOLD;
    pub const GLASS: u32 = 0xa8c4cc | mat::METAL;
    pub const SAND: u32 = 0xd2b478 | mat::CLOTH;
    pub const SAND_D: u32 = 0xb0945a | mat::CLOTH;
    // The nightmare's.
    pub const SHROUD: u32 = 0x2c2440 | mat::CLOTH;
    pub const SHROUD_L: u32 = 0x463a64 | mat::CLOTH;
    pub const SHROUD_D: u32 = 0x1c1628 | mat::CLOTH;
    pub const WISP: u32 = 0x6a50a8 | mat::GLOW;
    pub const WISP_D: u32 = 0x3e3070 | mat::GLOW;
    pub const BONE: u32 = 0xd8d0c0 | mat::IVORY;
    pub const BONE_D: u32 = 0xa8a090 | mat::IVORY;
    pub const SOCKET: u32 = 0x120c18 | mat::IVORY;
    pub const DREAM_EYE: u32 = 0xff70d8 | mat::GLOW;
}
use pal::*;

/// Belphegor's grid and voxel: about three tiles tall (to the tip of his nightcap), as Mammon.
pub const BELPHEGOR_SIZE: (i32, i32, i32) = (100, 124, 80);
pub const BELPHEGOR_VOXEL: f32 = 0.025;
const BX: f32 = 50.0;
const BZ: f32 = 34.0;

/// Hips, shoulders and neck; the hourglass goes with his left arm, the beard's fall with his body.
pub const BELPHEGOR_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [BX - 12.0, 30.0, BZ] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [BX + 12.0, 30.0, BZ] },
    PartDef { name: "leftArm", boxes: &[], pivot: [BX - 31.0, 84.0, BZ] },
    PartDef { name: "rightArm", boxes: &[], pivot: [BX + 31.0, 84.0, BZ] },
    PartDef { name: "head", boxes: &[], pivot: [BX, 90.0, BZ + 6.0] },
];

/// The night-robe's half-widths (x across, z deep) at height `y`: a heavy bell to the ankles.
fn robe(y: f32) -> (f32, f32) {
    let k = ((52.0 - y) / 42.0).clamp(0.0, 1.0);
    (29.0 + k.powf(1.2) * 7.0, 25.0 + k.powf(1.2) * 5.0)
}

/// Is `c` one of the robe's own colours (not its trim)?
fn robed(c: u32) -> bool {
    matches!(c, ROBE | ROBE_L | ROBE_D)
}

/// A long curved nail from `at`, `len` voxels along `dir`, curling down at its tip.
fn nail(s: &mut Sculpt, at: Vec3, dir: Vec3, len: f32) {
    let d = dir.normalize_or_zero();
    let mid = at + d * (len * 0.6);
    let tip = at + d * len + v(0.0, -len * 0.25, 0.0);
    limb(s, &[(at, 1.3), (mid, 1.0)], NAIL);
    limb(s, &[(mid, 1.0), (tip, 0.45)], NAIL);
    s.set(tip.x.round() as i32, tip.y.round() as i32, tip.z.round() as i32, NAIL_TIP);
}

/// The hourglass he carries: brass plates top and bottom joined by three posts, two glass bulbs,
/// the sand run nearly all into the lower and stopped there. `top` is the middle of its top plate.
fn hourglass(s: &mut Sculpt, top: Vec3, tall: f32) {
    let bottom = top - v(0.0, tall, 0.0);
    let r = tall * 0.32;
    for (c, rr) in [(top, r), (bottom, r)] {
        s.capsule((c.x, c.y - 1.0, c.z), (c.x, c.y + 1.0, c.z), rr, rr, BRASS);
        s.capsule((c.x, c.y - 1.6, c.z), (c.x, c.y - 1.4, c.z), rr - 1.2, rr - 1.2, BRASS_D);
    }
    for k in 0..3 {
        let a = k as f32 * 2.0 * PI / 3.0 + 0.4;
        let o = v(a.cos() * (r - 1.2), 0.0, a.sin() * (r - 1.2));
        limb(s, &[(top + o, 1.0), (bottom + o, 1.0)], BRASS_D);
        ball(s, top + o - v(0.0, 2.0, 0.0), Vec3::splat(1.3), BRASS_L);
        ball(s, bottom + o + v(0.0, 2.0, 0.0), Vec3::splat(1.3), BRASS_L);
    }
    let mid = (top + bottom) * 0.5;
    let bulb = v(r - 2.2, tall * 0.24, r - 2.2);
    let upper = mid + v(0.0, tall * 0.23, 0.0);
    let lower = mid - v(0.0, tall * 0.23, 0.0);
    ball(s, upper, bulb, GLASS);
    ball(s, lower, bulb, GLASS);
    s.capsule((mid.x, mid.y - 2.0, mid.z), (mid.x, mid.y + 2.0, mid.z), 1.4, 1.4, GLASS);
    // The sand: a heap in the lower bulb, a little caught in the neck of the upper.
    let level = lower.y + 1.0;
    let lo = (lower - bulb - Vec3::ONE).as_ivec3();
    let hi = (lower + bulb + Vec3::ONE).as_ivec3();
    for y in lo.y..=hi.y {
        for z in lo.z..=hi.z {
            for x in lo.x..=hi.x {
                if s.get(x, y, z) != Some(GLASS) {
                    continue;
                }
                let heap = level - ((x as f32 - lower.x).powi(2) + (z as f32 - lower.z).powi(2)).sqrt() * 0.35;
                if (y as f32) <= heap {
                    s.recolor(x, y, z, if (x + y + z) % 3 == 0 { SAND_D } else { SAND });
                }
            }
        }
    }
    for y in (upper.y - bulb.y) as i32..=(upper.y - bulb.y + 2.0) as i32 {
        s.recolor(upper.x as i32, y, upper.z as i32, SAND);
    }
}

pub fn belphegor() -> Collected {
    let (w, h, d) = BELPHEGOR_SIZE;
    let mut s = Sculpt::new(w, h, d);

    // ---- legs and feet: thick, under the robe; bare grey feet with long yellowed toenails
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        let x = BX + side * 12.0;
        s.capsule((x, 30.0, BZ), (x, 7.0, BZ + 2.0), 9.0, 7.5, SKIN_D);
        ball(&mut s, v(x + side * 1.0, 4.0, BZ + 9.0), v(8.0, 4.5, 11.0), SKIN);
        ball(&mut s, v(x + side * 1.0, 6.0, BZ + 4.0), v(6.5, 4.0, 6.0), SKIN_D);
        for (k, t) in [-4.5f32, -1.5, 1.5, 4.5].into_iter().enumerate() {
            let toe = v(x + side * 1.0 + t, 2.5, BZ + 18.0 - (k as f32 - 1.5).abs());
            ball(&mut s, toe, v(1.8, 1.8, 2.4), SKIN_L);
            nail(&mut s, toe + v(0.0, 0.5, 2.0), v(t * 0.06, 0.0, 1.0), 4.5);
        }
    }

    // ---- the belly, the robe over it, and the chest: a grey night-robe in slate-blue stripes
    s.part(0);
    ball(&mut s, v(BX, 44.0, BZ + 8.0), v(29.0, 26.0, 28.0), ROBE);
    ball(&mut s, v(BX, 74.0, BZ + 2.0), v(31.0, 13.0, 21.0), ROBE);
    for y in 10..=62 {
        let yf = y as f32;
        let (rx, rz) = robe(yf);
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = ((x as f32 - BX) / rx, (z as f32 - BZ - 2.0) / rz);
                if dx * dx + dz * dz <= 1.0 {
                    s.set(x, y, z, if y <= 13 { TRIM } else { ROBE });
                }
            }
        }
    }
    // Folds and stripes: vertical bands round him, shaded by the fall of the cloth.
    s.paint(|x, y, z, c| {
        if !robed(c) {
            return None;
        }
        let a = (z as f32 - BZ).atan2(x as f32 - BX);
        let band = ((a / PI * 9.0).floor() as i32).rem_euclid(2) == 0;
        let fold = (a * 11.0 + (y as f32 * 0.08).sin() * 1.5).sin();
        Some(match (band, fold > 0.5, fold < -0.5) {
            (true, _, true) => STRIPE_D,
            (true, _, _) => STRIPE,
            (false, true, _) => ROBE_L,
            (false, _, true) => ROBE_D,
            _ => ROBE,
        })
    });
    // The sash under the belly's swell, its knot and two hanging ends at his left.
    for y in 56..=60 {
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = ((x as f32 - BX) / 31.0, (z as f32 - BZ - 6.0) / 29.0);
                let q = dx * dx + dz * dz;
                if (0.78..=1.0).contains(&q) {
                    s.set(x, y, z, SASH);
                }
            }
        }
    }
    let knot = v(BX - 14.0, 58.0, BZ + 32.0);
    ball(&mut s, knot, v(3.5, 3.0, 2.5), SASH);
    limb(&mut s, &[(knot, 2.0), (knot + v(-2.0, -14.0, 2.0), 1.6), (knot + v(-1.0, -22.0, 3.0), 1.4)], SASH);
    limb(&mut s, &[(knot, 2.0), (knot + v(2.5, -11.0, 2.5), 1.5), (knot + v(3.0, -17.0, 3.5), 1.3)], SASH);
    // The collar: the robe's cream trim round his neck and down the front to the sash.
    for k in 0..40 {
        let a = k as f32 / 40.0 * 2.0 * PI;
        ball(&mut s, v(BX + a.cos() * 16.0, 85.0, BZ + 4.0 + a.sin() * 12.0), Vec3::splat(2.0), TRIM);
    }
    // The beard's long fall over his chest and belly (its top goes with his head).
    limb(&mut s, &[(v(BX, 82.0, BZ + 22.0), 10.0), (v(BX, 68.0, BZ + 29.0), 9.0), (v(BX + 1.0, 52.0, BZ + 35.0), 5.5)], BEARD);
    ball(&mut s, v(BX + 1.5, 47.0, BZ + 36.0), v(3.5, 5.0, 2.5), BEARD);

    // ---- arms: wide striped sleeves to the elbow, bare grey forearms, great long-nailed hands
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(3 + i as u8);
        let sh = v(BX + side * 31.0, 82.0, BZ);
        let elbow = v(BX + side * 39.0, 62.0, BZ + 6.0);
        let wrist = if side < 0.0 { v(BX - 40.0, 50.0, BZ + 18.0) } else { v(BX + 38.0, 46.0, BZ + 14.0) };
        ball(&mut s, sh, v(11.0, 11.0, 11.0), ROBE);
        limb(&mut s, &[(sh, 10.0), (elbow, 11.5)], ROBE);
        s.capsule((elbow.x, elbow.y - 1.0, elbow.z), (elbow.x, elbow.y - 4.0, elbow.z + 1.0), 12.0, 12.4, STRIPE_D);
        s.capsule((elbow.x, elbow.y - 4.0, elbow.z + 1.0), (elbow.x, elbow.y - 5.5, elbow.z + 1.0), 12.6, 12.6, TRIM);
        limb(&mut s, &[(elbow, 7.5), (wrist, 6.0)], SKIN);
        let hand = wrist + v(side * 0.5, -6.0, 2.0);
        ball(&mut s, hand, v(6.5, 7.0, 6.0), SKIN);
        if side > 0.0 {
            // His right: open, hanging, four long fingers and a thumb, every nail long and curled.
            for (k, a) in [-0.9f32, -0.3, 0.3, 0.9].into_iter().enumerate() {
                let base = hand + v(a * 4.2, -4.5, 2.5);
                let tip = base + v(a * 1.5, -6.5 - (k % 2) as f32, 3.0);
                limb(&mut s, &[(base, 2.2), (tip, 1.6)], SKIN);
                nail(&mut s, tip, v(a * 0.2, -0.6, 1.0), 7.0);
            }
            let thumb = hand + v(-5.0, -1.0, 4.0);
            limb(&mut s, &[(thumb, 2.0), (thumb + v(-2.0, -4.0, 4.0), 1.5)], SKIN);
            nail(&mut s, thumb + v(-2.0, -4.0, 4.0), v(-0.3, -0.4, 1.0), 5.0);
        } else {
            // His left: round the top of his hourglass, the nails curled over its plate.
            let top = hand + v(0.0, -6.0, 1.0);
            for a in [-0.8f32, 0.0, 0.8] {
                let base = hand + v(a * 4.0, -3.0, 3.5);
                limb(&mut s, &[(base, 2.0), (base + v(a, -3.0, 1.5), 1.6)], SKIN);
                nail(&mut s, base + v(a, -3.0, 1.5), v(a * 0.3, -1.0, 0.4), 4.5);
            }
            hourglass(&mut s, top, 28.0);
        }
    }

    // ---- the head: jowled and heavy-lidded, a great nose, the beard, horns, and his nightcap
    s.part(5);
    let hc = v(BX, 101.0, BZ + 9.0);
    ball(&mut s, v(BX, 90.0, BZ + 8.0), v(15.0, 7.0, 13.0), SKIN);
    ball(&mut s, hc, v(13.0, 13.0, 12.0), SKIN);
    for side in [-1.0f32, 1.0] {
        // The jowls, and pointed ears under the cap.
        ball(&mut s, hc + v(side * 8.5, -7.0, 5.0), v(7.0, 6.5, 6.0), SKIN);
        limb(&mut s, &[(hc + v(side * 12.0, 1.0, 0.0), 2.5), (hc + v(side * 19.0, 5.0, -3.0), 0.8)], SKIN_D);
    }
    // The brow, heavy over the eyes, and the lids sagging over them.
    ball(&mut s, hc + v(0.0, 4.5, 9.0), v(11.5, 2.8, 4.0), SKIN_D);
    let front = |s: &Sculpt, x: i32, y: i32| (0..d).rev().find(|&z| s.get(x, y, z).is_some());
    for side in [-1i32, 1] {
        for k in 0..4 {
            let x = BX as i32 + side * (3 + k);
            // A slit of dim light under each heavy lid.
            if let Some(z) = front(&s, x, 101) {
                s.recolor(x, 101, z, if k == 0 || k == 3 { LID } else { EYE });
            }
            for y in 102..=103 {
                if let Some(z) = front(&s, x, y) {
                    s.set(x, y, z + 1, LID);
                }
            }
        }
    }
    // The nose: broad and bulbous, its nostrils dark.
    ball(&mut s, hc + v(0.0, -1.5, 12.5), v(4.0, 4.5, 3.2), SKIN_L);
    for side in [-1i32, 1] {
        s.set(BX as i32 + side * 2, 96, (hc.z + 14.5) as i32, NOSTRIL);
    }
    // The mustache drooping either side, and the beard's top from the chin.
    for side in [-1.0f32, 1.0] {
        limb(
            &mut s,
            &[(hc + v(side * 2.0, -5.0, 13.0), 2.4), (hc + v(side * 7.0, -7.0, 11.0), 2.0), (hc + v(side * 9.0, -14.0, 9.0), 1.2)],
            BEARD_L,
        );
    }
    ball(&mut s, hc + v(0.0, -13.0, 8.0), v(11.5, 8.5, 7.5), BEARD);
    ball(&mut s, hc + v(0.0, -19.0, 10.0), v(9.5, 5.0, 6.0), BEARD);
    // Ram's horns from the temples, curling out and back, pale at the tips.
    for side in [-1.0f32, 1.0] {
        let mut knots = Vec::new();
        for k in 0..=14 {
            let t = k as f32 / 14.0;
            let a = t * 4.4;
            let r = 12.0 * (1.0 - t * 0.35);
            let p = hc + v(side * (11.0 + t * 8.0 + a.sin() * r * 0.5), 5.0 + a.sin() * r * 0.85, -1.0 - (1.0 - a.cos()) * r * 0.6);
            knots.push((p, 5.5 - 4.4 * t));
        }
        limb(&mut s, &knots, HORN);
        ball(&mut s, knots.last().unwrap().0, Vec3::splat(1.1), HORN_TIP);
    }
    s.paint(|x, y, z, c| {
        if c != HORN {
            return None;
        }
        Some(if (y + x + z / 3) % 4 == 0 { HORN_L } else { HORN })
    });
    // The nightcap: snug over his crown between the horns, its brim pale, flopping over to his
    // right and down, a tassel at its end.
    s.ellipsoid((hc.x, hc.y + 6.0, hc.z - 1.0), (13.5, 9.0, 12.5), (hc.y + 4.0) as i32, CAP);
    for k in 0..48 {
        let a = k as f32 / 48.0 * 2.0 * PI;
        ball(&mut s, v(hc.x + a.cos() * 13.0, hc.y + 5.0, hc.z - 1.0 + a.sin() * 12.0), Vec3::splat(1.8), BRIM);
    }
    let cone = [
        (hc + v(0.0, 11.0, -1.0), 9.5),
        (hc + v(2.5, 15.5, -2.0), 7.5),
        (hc + v(8.0, 18.5, -3.0), 5.2),
        (hc + v(15.0, 17.5, -4.0), 3.6),
        (hc + v(21.0, 12.5, -4.0), 2.6),
        (hc + v(23.5, 7.0, -3.0), 1.8),
    ];
    limb(&mut s, &cone, CAP);
    s.paint(|_, y, _, c| {
        if c != CAP {
            return None;
        }
        Some(if (y / 3) % 2 == 0 { CAP } else { CAP_D })
    });
    ball(&mut s, hc + v(24.0, 4.0, -3.0), Vec3::splat(2.8), TASSEL);
    limb(&mut s, &[(hc + v(24.0, 2.0, -3.0), 1.4), (hc + v(24.5, -1.5, -3.0), 0.8)], TASSEL);
    // The beard made wiry: strands lighter and darker down its fall (on the body's part too).
    s.paint(|x, y, z, c| {
        if c != BEARD {
            return None;
        }
        let n = nz(x, y / 3, z);
        Some(if x % 3 == 0 && n > 0.4 {
            BEARD_D
        } else if n > 0.85 {
            BEARD_L
        } else {
            BEARD
        })
    });
    s.paint(|x, y, z, c| {
        if c != SKIN {
            return None;
        }
        let n = nz(x, y, z);
        Some(if n < 0.12 {
            SKIN_L
        } else if n > 0.9 {
            SKIN_D
        } else {
            SKIN
        })
    });

    s.finish(0.03)
}

/// The nightmare's grid and voxel: about a hero's height and a half, floating.
pub const NIGHTMARE_SIZE: (i32, i32, i32) = (52, 76, 44);
pub const NIGHTMARE_VOXEL: f32 = 0.025;
const NX: f32 = 26.0;
const NZ: f32 = 20.0;

/// Its two trailing wisps (where legs would be), its arms, and its hooded head.
pub const NIGHTMARE_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [NX - 5.0, 32.0, NZ] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [NX + 5.0, 32.0, NZ] },
    PartDef { name: "leftArm", boxes: &[], pivot: [NX - 11.0, 54.0, NZ] },
    PartDef { name: "rightArm", boxes: &[], pivot: [NX + 11.0, 54.0, NZ] },
    PartDef { name: "head", boxes: &[], pivot: [NX, 58.0, NZ + 1.0] },
];

pub fn nightmare() -> Collected {
    let (w, h, d) = NIGHTMARE_SIZE;
    let mut s = Sculpt::new(w, h, d);

    // ---- the shroud: hunched, its hem in tatters
    s.part(0);
    ball(&mut s, v(NX, 46.0, NZ - 1.0), v(11.0, 13.0, 9.0), SHROUD);
    ball(&mut s, v(NX, 54.0, NZ - 2.0), v(12.0, 5.0, 8.0), SHROUD);
    for k in 0..14 {
        let a = k as f32 / 14.0 * 2.0 * PI;
        let len = 6.0 + nz(k, 3, 7) * 7.0;
        let top = v(NX + a.cos() * 9.5, 36.0, NZ - 1.0 + a.sin() * 7.5);
        limb(&mut s, &[(top, 2.4), (top + v(a.cos() * 1.5, -len, a.sin() * 1.5), 0.7)], SHROUD_D);
    }
    s.paint(|x, y, z, c| {
        if c != SHROUD {
            return None;
        }
        let fold = ((x as f32 - NX).atan2(z as f32 - NZ) * 7.0 + y as f32 * 0.15).sin();
        Some(if fold > 0.6 {
            SHROUD_L
        } else if fold < -0.6 {
            SHROUD_D
        } else {
            SHROUD
        })
    });

    // ---- two wisps trailing below it, fading into a glow
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        let knots = [
            (v(NX + side * 5.0, 34.0, NZ - 1.0), 6.0),
            (v(NX + side * 6.5, 24.0, NZ - 3.0), 4.5),
            (v(NX + side * 5.0, 14.0, NZ - 6.0), 3.0),
            (v(NX + side * 7.5, 6.0, NZ - 9.0), 1.6),
            (v(NX + side * 9.0, 1.0, NZ - 11.0), 0.6),
        ];
        limb(&mut s, &knots, SHROUD_D);
        s.paint(|_, y, _, c| {
            if c != SHROUD_D || y > 26 {
                return None;
            }
            Some(if y < 12 { WISP } else { WISP_D })
        });
    }

    // ---- thin bony arms out of tattered sleeves, long grasping fingers
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(3 + i as u8);
        let sh = v(NX + side * 11.0, 53.0, NZ - 1.0);
        let elbow = v(NX + side * 15.0, 42.0, NZ + 5.0);
        let wrist = v(NX + side * 15.5, 34.0, NZ + 11.0);
        limb(&mut s, &[(sh, 4.6), (elbow, 4.2)], SHROUD);
        limb(&mut s, &[(elbow, 4.2), (elbow + v(side * 0.5, -3.0, 1.5), 5.0)], SHROUD_D);
        limb(&mut s, &[(elbow, 1.8), (wrist, 1.5)], BONE);
        ball(&mut s, wrist, v(2.2, 2.0, 2.2), BONE);
        for a in [-1.0f32, 0.0, 1.0] {
            let tip = wrist + v(a * 1.6 + side * 0.5, -6.0, 3.0 + a.abs());
            limb(&mut s, &[(wrist, 0.9), (tip, 0.6)], BONE_D);
        }
    }

    // ---- the hood, and the mare's pale skull in it
    s.part(5);
    let hc = v(NX, 61.0, NZ + 1.0);
    ball(&mut s, hc, v(10.0, 11.0, 10.0), SHROUD);
    // The hood's point, drooping back.
    limb(&mut s, &[(hc + v(0.0, 8.0, -3.0), 5.0), (hc + v(0.0, 13.0, -8.0), 2.0), (hc + v(0.0, 12.0, -13.0), 0.8)], SHROUD);
    // Its opening: hollowed out in front.
    for y in (hc.y - 9.0) as i32..=(hc.y + 6.0) as i32 {
        for z in (hc.z + 3.0) as i32..d {
            for x in (hc.x - 6.5) as i32..=(hc.x + 6.5) as i32 {
                let (dx, dy) = ((x as f32 - hc.x) / 6.5, (y as f32 - hc.y + 1.0) / 8.0);
                if dx * dx + dy * dy <= 1.0 {
                    s.clear(x, y, z);
                }
            }
        }
    }
    // The skull: a horse's long head, looking out of the dark.
    ball(&mut s, hc + v(0.0, 1.0, 2.0), v(5.5, 6.0, 6.0), BONE);
    limb(&mut s, &[(hc + v(0.0, -1.0, 5.0), 4.5), (hc + v(0.0, -4.0, 12.0), 3.4), (hc + v(0.0, -5.5, 16.0), 3.0)], BONE);
    for side in [-1.0f32, 1.0] {
        // Its sockets, burning pink; its nostrils; its ears pricked through the hood.
        ball(&mut s, hc + v(side * 3.6, 2.0, 6.0), v(1.8, 1.8, 1.4), SOCKET);
        ball(&mut s, hc + v(side * 3.4, 2.0, 6.6), Vec3::splat(0.9), DREAM_EYE);
        s.set((hc.x + side * 1.5) as i32, (hc.y - 5.0) as i32, (hc.z + 19.0) as i32, SOCKET);
        limb(&mut s, &[(hc + v(side * 5.0, 9.0, -1.0), 1.8), (hc + v(side * 7.0, 15.0, -2.0), 0.6)], SHROUD_L);
    }
    for k in 0..5 {
        s.set((hc.x - 2.0 + k as f32) as i32, (hc.y - 8.0) as i32, (hc.z + 15.0) as i32, BONE_D);
    }
    s.paint(|x, y, z, c| {
        if c != BONE {
            return None;
        }
        Some(if nz(x, y, z) > 0.8 { BONE_D } else { BONE })
    });

    s.finish(0.03)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn belphegor_towers_over_the_hero_in_his_nightcap_and_a_nightmare_floats_smaller() {
        let b = belphegor();
        let top = b.voxels.iter().map(|v| v.y).max().unwrap() as f32 * BELPHEGOR_VOXEL;
        assert!(top > 2.7 && top < 3.4, "{top} tiles tall");
        assert!(b.voxels.len() > 8000, "{} voxels", b.voxels.len());
        for part in 0..=BELPHEGOR_RIG.len() as u8 {
            assert!(b.voxels.iter().any(|v| v.part == part), "part {part}");
        }
        let cloth = |c: u32| c & 0xff00_0000 == mat::CLOTH;
        let fur = |c: u32| c & 0xff00_0000 == mat::FUR;
        assert!(b.voxels.iter().filter(|v| cloth(v.c)).count() > b.voxels.len() / 3, "robed in his night-robe");
        assert!(b.voxels.iter().filter(|v| fur(v.c)).count() > 400, "bearded");
        let glow = |c: u32| c & 0xff00_0000 == mat::GLOW;
        assert!(b.voxels.iter().filter(|v| glow(v.c)).count() >= 4, "his eyes, two slits of light");
        let n = nightmare();
        let top = n.voxels.iter().map(|v| v.y).max().unwrap() as f32 * NIGHTMARE_VOXEL;
        assert!(top > 1.4 && top < 2.0, "{top}");
        for part in 0..=NIGHTMARE_RIG.len() as u8 {
            assert!(n.voxels.iter().any(|v| v.part == part), "nightmare part {part}");
        }
        assert!(n.voxels.iter().any(|v| glow(v.c)), "its eyes burn, its wisps glow");
    }
}
