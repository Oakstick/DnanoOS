//! The model of the ellepiger, the elf-maids of the Garden of Thorns (REALMS.md §6), facing +z, in
//! the parts its rig swings. The Bride is drawn from it, much larger.
//!
//! From the front she is lovely: pale skin and rosy lips, grey-blue eyes, long fair hair falling
//! over her shoulders under a crown of white and pink blossoms, and a white gown flowing to the
//! floor, sashed in rose at the waist, its bell sleeves open at the wrist. From behind she is hollow
//! as a kneading trough: from the hem to the shoulder blades her back is an open trough of rotten
//! wood, bark at its lips, moss and pale fungus in the dark of it.

use std::f32::consts::{PI, TAU};

use glam::Vec3;

use super::carve::{ball, limb, nz, v};
use super::characters::{mat, Sculpt};
use super::voxels::{Collected, PartDef};

/// The model's grid, and the size of one of its voxels in tiles: about a hero's height.
pub const ELLEPIGE_SIZE: (i32, i32, i32) = (38, 96, 32);
pub const ELLEPIGE_VOXEL: f32 = 0.0125;

/// Her middle (x, z).
const CX: f32 = 18.5;
const CZ: f32 = 16.0;

/// Hips, shoulders and neck.
pub const ELLEPIGE_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [15.5, 30.0, CZ] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [21.5, 30.0, CZ] },
    PartDef { name: "leftArm", boxes: &[], pivot: [12.0, 66.0, CZ] },
    PartDef { name: "rightArm", boxes: &[], pivot: [25.0, 66.0, CZ] },
    PartDef { name: "head", boxes: &[], pivot: [CX, 71.0, CZ] },
];

const SKIN: u32 = 0xecd2c8 | mat::SKIN;
const SKIN_D: u32 = 0xd8b4aa | mat::SKIN;
const BLUSH: u32 = 0xf2b4b8 | mat::SKIN;
const LIPS: u32 = 0xd2547a;
const EYE: u32 = 0x3e6a8c;
const EYE_D: u32 = 0x1a2a3a;
const BROW: u32 = 0xc8a060;
const GOWN: u32 = 0xe6dee6 | mat::CLOTH;
const GOWN_L: u32 = 0xf4eef2 | mat::CLOTH;
const GOWN_D: u32 = 0xc4b8cc | mat::CLOTH;
const LACE: u32 = 0xd8cce0 | mat::CLOTH;
const SASH: u32 = 0xf2a6c4 | mat::CLOTH;
const SASH_D: u32 = 0xd8809e | mat::CLOTH;
const HAIR: u32 = 0xf0d898 | mat::FUR;
const HAIR_L: u32 = 0xfff0c4 | mat::FUR;
const HAIR_D: u32 = 0xc8a866 | mat::FUR;
const PETAL: u32 = 0xfff8f2;
const PETAL_P: u32 = 0xffa8cc;
const PETAL_R: u32 = 0xff4a9a;
const POLLEN: u32 = 0xffd860;
const LEAF: u32 = 0x4a8a4a;
const SLIPPER: u32 = 0xf0e4ec | mat::LEATHER;
const BARK: u32 = 0x6a4a34 | mat::WOOD;
const BARK_D: u32 = 0x3a281c | mat::WOOD;
const ROT: u32 = 0x4a3222 | mat::WOOD;
const ROT_D: u32 = 0x24160e | mat::WOOD;
const MOSS: u32 = 0x4e6e34;
const FUNGUS: u32 = 0xc8bc90;
const FUNGUS_GLOW: u32 = 0xd8e4a8 | mat::GLOW;

/// The gown's half-widths at height `y` (x across, z deep): flared to the hem, gathered at the
/// waist, shaped through the bodice.
fn gown(y: f32) -> (f32, f32) {
    if y <= 52.0 {
        let k = ((52.0 - y) / 48.0).clamp(0.0, 1.0);
        let flare = k.powf(1.4);
        (5.4 + flare * 7.4, 4.2 + flare * 5.6)
    } else {
        let k = ((y - 52.0) / 16.0).clamp(0.0, 1.0);
        // The bodice: the waist, the bust, the shoulders.
        let bust = (-((k - 0.55) / 0.3).powi(2)).exp();
        (5.4 + k * 1.4 + bust * 0.4, 4.2 + bust * 1.0 + k * 0.2)
    }
}

/// How far into her shape (x, z) at height `y` lies: below 1 inside the gown.
fn shape(x: f32, y: f32, z: f32) -> f32 {
    let (rx, rz) = gown(y);
    ((x - CX) / rx).abs().powf(2.3) + ((z - CZ) / rz).abs().powf(2.3)
}

/// The trough runs from just above the hem to her shoulder blades.
const TROUGH: (f32, f32) = (9.0, 63.0);

/// Whether a voxel of her back falls in the trough: behind her middle, within the lips of it, and
/// between its ends (which round off).
fn in_trough(x: f32, y: f32, z: f32) -> bool {
    let (rx, rz) = gown(y);
    let (lo, hi) = TROUGH;
    if y < lo || y > hi || z > CZ - rz * 0.05 {
        return false;
    }
    // Narrower toward its ends, like a hollowed log.
    let t = ((y - lo) / (hi - lo)).clamp(0.0, 1.0);
    let ends = (t * PI).sin().powf(0.35);
    let half = rx * 0.6 * ends;
    (x - CX).abs() < half && (z - CZ) / rz > -1.3
}

pub fn ellepige() -> Collected {
    let (w, h, d) = ELLEPIGE_SIZE;
    let mut s = Sculpt::new(w, h, d);

    // ---- legs, under the gown: slender, pale slippers peeping out at the hem
    for (i, lx) in [15.5f32, 21.5].into_iter().enumerate() {
        s.part(1 + i as u8);
        s.capsule((lx, 30.0, CZ + 2.5), (lx, 5.0, CZ + 3.0), 2.0, 1.4, SKIN_D);
        ball(&mut s, v(lx, 2.2, CZ + 5.0), v(1.9, 1.7, 3.4), SLIPPER);
        s.capsule((lx, 4.0, CZ + 3.0), (lx, 2.0, CZ + 4.0), 1.6, 1.7, SLIPPER);
    }

    // ---- the gown: flowing folds down the skirt, lace at the hem, the bodice and the sash
    s.part(0);
    for y in 3..=68 {
        let yf = y as f32;
        for z in 0..d {
            for x in 0..w {
                let (xf, zf) = (x as f32, z as f32);
                let q = shape(xf, yf, zf);
                if q > 1.0 {
                    continue;
                }
                // The hem's scallops.
                let a = (zf - CZ).atan2(xf - CX);
                if y <= 4 && (a * 9.0).sin() < -0.2 {
                    continue;
                }
                let fold = (a * 8.0 + (yf * 0.07).sin() * 0.6).sin();
                let depth = (1.0 - yf / 60.0).clamp(0.15, 1.0);
                let c = if y <= 5 {
                    LACE
                } else if fold * depth > 0.45 {
                    GOWN_L
                } else if fold * depth < -0.45 {
                    GOWN_D
                } else {
                    GOWN
                };
                s.set(x, y, z, c);
            }
        }
    }
    // The décolletage: skin above the neckline in front, the gown's lace edge along it.
    for y in 60..=69 {
        let yf = y as f32;
        for z in 0..d {
            for x in 0..w {
                let (xf, zf) = (x as f32, z as f32);
                if s.get(x, y, z).is_none() || zf < CZ + 0.5 {
                    continue;
                }
                let neck = 63.5 + ((xf - CX).abs() / 6.0).powi(2) * 4.0;
                if yf > neck {
                    s.recolor(x, y, z, if (yf - neck) < 1.0 { LACE } else { SKIN });
                }
            }
        }
    }
    // Shoulders and the neck.
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(CX + side * 5.6, 66.0, CZ), v(2.6, 2.2, 2.6), SKIN);
    }
    s.capsule((CX, 66.0, CZ), (CX, 72.0, CZ + 0.3), 2.0, 1.8, SKIN);
    // The sash at the waist: a ribbon round her front and sides, its bow at her left hip.
    for y in 50..=53 {
        for z in 0..d {
            for x in 0..w {
                let (xf, zf) = (x as f32, z as f32);
                if s.get(x, y, z).is_none() {
                    continue;
                }
                let q = shape(xf, y as f32, zf);
                if q > 0.7 {
                    s.recolor(x, y, z, if y == 50 || y == 53 { SASH_D } else { SASH });
                }
            }
        }
    }
    let bow = v(CX - 5.0, 51.5, CZ + 3.2);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, bow + v(side * 1.8, 0.6, 0.6), v(1.6, 1.3, 0.9), SASH);
        limb(&mut s, &[(bow, 0.7), (bow + v(side * 1.0, -6.0, 1.0), 0.6)], SASH_D);
    }
    ball(&mut s, bow + v(0.0, 0.0, 0.8), Vec3::splat(0.9), SASH_D);

    // ---- the trough: her back, open from the hem to the shoulder blades, rotten wood inside
    let mut lips: Vec<(i32, i32, i32)> = Vec::new();
    for y in 0..h {
        for z in 0..d {
            for x in 0..w {
                if s.get(x, y, z).is_none() || s.tag_at(x, y, z) != 0 {
                    continue;
                }
                let (xf, yf, zf) = (x as f32, y as f32, z as f32);
                if in_trough(xf, yf, zf) {
                    s.clear(x, y, z);
                } else if in_trough(xf - 1.0, yf, zf)
                    || in_trough(xf + 1.0, yf, zf)
                    || in_trough(xf, yf - 1.0, zf)
                    || in_trough(xf, yf + 1.0, zf)
                    || in_trough(xf, yf, zf - 1.0)
                {
                    lips.push((x, y, z));
                }
            }
        }
    }
    // Inside: the near wall of the trough (what is left of her) is dark rot; its lips are bark.
    for y in 0..h {
        for z in 0..d {
            for x in 0..w {
                let Some(c) = s.get(x, y, z) else { continue };
                if s.tag_at(x, y, z) != 0 || c == SASH || c == SASH_D {
                    continue;
                }
                let (xf, yf, zf) = (x as f32, y as f32, z as f32);
                let faces_hollow = in_trough(xf, yf, zf - 1.0) || in_trough(xf - 1.0, yf, zf) || in_trough(xf + 1.0, yf, zf);
                if !faces_hollow {
                    continue;
                }
                let n = nz(x, y, z);
                let back = zf < CZ - gown(yf).1 * 0.55;
                let col = if back {
                    if n < 0.5 {
                        BARK
                    } else {
                        BARK_D
                    }
                } else if n < 0.04 {
                    FUNGUS
                } else if n < 0.2 {
                    MOSS
                } else if ((yf * 0.6 + xf * 0.3).sin() * 0.5 + 0.5) < 0.35 {
                    ROT_D
                } else {
                    ROT
                };
                s.recolor(x, y, z, col);
            }
        }
    }
    for (x, y, z) in lips {
        if s.get(x, y, z).is_some() && (z as f32) < CZ - 1.0 {
            s.recolor(x, y, z, if nz(x, y, z) < 0.5 { BARK } else { BARK_D });
        }
    }
    // Shelf fungus on the lips, and a few roots trailing from its foot.
    for (k, (fx, fy)) in [(-1.0f32, 22.0f32), (1.0, 38.0), (-1.0, 47.0)].into_iter().enumerate() {
        let (rx, rz) = gown(fy);
        let at = v(CX + fx * rx * 0.58, fy, CZ - rz * 0.75);
        ball(&mut s, at + v(fx * 0.8, 0.0, -0.6), v(1.6, 0.6, 1.2), if k % 2 == 0 { FUNGUS_GLOW } else { 0xb8a87a });
    }
    for k in 0..3 {
        let x0 = CX - 3.0 + k as f32 * 3.0;
        limb(&mut s, &[(v(x0, 10.0, CZ - 6.0), 0.6), (v(x0 + 0.6, 4.0, CZ - 8.5), 0.5), (v(x0 + 1.4, 0.5, CZ - 10.0), 0.4)], BARK_D);
    }

    // ---- arms: slender, bare above white bell sleeves, the hands folded a little forward
    for (i, ax) in [12.0f32, 25.0].into_iter().enumerate() {
        s.part(3 + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        let elbow = v(ax + out * 1.2, 55.0, CZ + 0.8);
        let wrist = v(ax + out * 0.4, 45.0, CZ + 3.4);
        limb(&mut s, &[(v(ax, 66.0, CZ), 1.9), (elbow, 1.5), (wrist, 1.1)], SKIN);
        // The sleeve: fitted to the elbow, then flaring open to a lace edge.
        limb(&mut s, &[(v(ax, 66.5, CZ), 2.3), (elbow + v(0.0, 1.0, 0.0), 1.9)], GOWN);
        for k in 0..=8 {
            let t = k as f32 / 8.0;
            let c = elbow.lerp(wrist + v(0.0, 1.5, -0.5), t);
            let r = 1.9 + t * 1.8;
            for a in 0..20 {
                let ang = a as f32 / 20.0 * TAU;
                let p = c + v(ang.cos() * r, 0.0, ang.sin() * r);
                s.set(
                    p.x.round() as i32,
                    p.y.round() as i32,
                    p.z.round() as i32,
                    if k == 8 {
                        LACE
                    } else if a % 5 == 0 {
                        GOWN_D
                    } else {
                        GOWN_L
                    },
                );
            }
        }
        ball(&mut s, wrist + v(0.0, -1.6, 0.4), v(1.1, 1.6, 0.9), SKIN);
        for f in [-0.6f32, 0.0, 0.6] {
            s.capsule((wrist.x + f, wrist.y - 2.4, wrist.z + 0.6), (wrist.x + f * 1.2, wrist.y - 4.0, wrist.z + 1.0), 0.45, 0.4, SKIN_D);
        }
    }

    // ---- the head: a fine pale face, rosy lips, grey-blue eyes; fair hair; the blossom crown
    s.part(5);
    let head = v(CX, 78.5, CZ + 0.4);
    ball(&mut s, head, v(4.5, 5.6, 4.7), SKIN);
    ball(&mut s, head + v(0.0, -3.6, 1.2), v(2.8, 2.0, 2.8), SKIN); // the chin
    let front = |s: &Sculpt, x: i32, y: i32| (0..d).rev().find(|&z| s.get(x, y, z).is_some());
    // Eyes: the grey-blue iris under a dark lash line, fair brows over them.
    for ex in [15i32, 21] {
        for (dx, dy, c) in [(0, 0, EYE), (1, 0, EYE), (0, 1, EYE_D), (1, 1, EYE_D), (0, 3, BROW), (1, 3, BROW)] {
            let (x, y) = (ex + dx, 78 + dy);
            if let Some(z) = front(&s, x, y) {
                s.recolor(x, y, z, c);
            }
        }
    }
    // A small nose, blushing cheeks, rosy lips.
    if let Some(z) = front(&s, 18, 76) {
        s.set(18, 76, z + 1, SKIN_D);
        s.set(19, 76, z + 1, SKIN_D);
    }
    for (x, y) in [(15, 76), (22, 76), (15, 75), (22, 75)] {
        if let Some(z) = front(&s, x, y) {
            s.recolor(x, y, z, BLUSH);
        }
    }
    for x in 17..=20 {
        if let Some(z) = front(&s, x, 74) {
            s.recolor(x, 74, z, LIPS);
        }
    }
    // Hair: a cap over the crown and the back of the head, parted in the middle.
    for y in 74..=86 {
        for z in 0..d {
            for x in 0..w {
                let p = v(x as f32, y as f32, z as f32) - (head + v(0.0, 0.6, -0.6));
                let q = (p.x / 5.2).powi(2) + (p.y / 6.3).powi(2) + (p.z / 5.4).powi(2);
                let face = p.z > 2.4 && y < 83;
                if q <= 1.0 && !face && s.get(x, y, z).is_none_or(|c| c == SKIN) {
                    let part = (x as f32 - CX).abs() < 0.6 && p.z > 0.0;
                    s.set(
                        x,
                        y,
                        z,
                        if part {
                            HAIR_D
                        } else if nz(x, y, z) < 0.25 {
                            HAIR_L
                        } else {
                            HAIR
                        },
                    );
                }
            }
        }
    }
    // Long locks: over her shoulders to her waist in front, and down her back to the trough's top.
    for k in 0..34 {
        let a = k as f32 / 34.0 * TAU;
        let (sa, ca) = (a.sin(), a.cos());
        // Not over the face.
        if sa > 0.45 && ca.abs() < 0.75 {
            continue;
        }
        let root = head + v(ca * 4.8, 1.0, sa * 4.8 - 0.4);
        let in_front = sa > -0.2;
        let len = if in_front { 26.0 + nz(k, 3, 1) * 6.0 } else { 14.0 + nz(k, 5, 2) * 5.0 };
        let steps = len as i32;
        for st in 0..steps {
            let t = st as f32 / len;
            // Out over the shoulders, then falling; a wave along each lock.
            let spread = if in_front { (t * 3.0).min(1.0) * 2.2 } else { (t * 3.0).min(1.0) * 1.2 };
            let wave = (st as f32 * 0.45 + k as f32).sin() * 0.35;
            let mut p = v(root.x + ca * spread + wave, root.y - st as f32, root.z + sa * spread * if in_front { 0.6 } else { 1.0 });
            if in_front && p.y < 66.0 {
                // Over the bust, the locks lie on the gown.
                p.z = p.z.max(CZ + 2.0 + gown(p.y).1 * 0.7);
            }
            let c = if nz(k, st, 7) < 0.25 {
                HAIR_L
            } else if nz(k, st, 9) > 0.8 {
                HAIR_D
            } else {
                HAIR
            };
            s.set(p.x.round() as i32, p.y.round() as i32, p.z.round() as i32, c);
            s.set((p.x + 0.5).round() as i32, p.y.round() as i32, p.z.round() as i32, c);
        }
    }
    // The blossom crown: white and pink flowers, a rose at the front, leaves between.
    for k in 0..14 {
        let a = k as f32 / 14.0 * TAU;
        let c = head + v(a.cos() * 5.0, 5.2 + (a * 2.0).sin() * 0.3, a.sin() * 5.2 - 0.4);
        let (x, y, z) = (c.x.round() as i32, c.y.round() as i32, c.z.round() as i32);
        if k % 2 == 0 {
            let petal = if a.sin() > 0.9 {
                PETAL_R
            } else if k % 4 == 0 {
                PETAL
            } else {
                PETAL_P
            };
            for (dx, dy, dz) in [(0, 0, 0), (1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)] {
                s.set(x + dx, y + dy, z + dz, petal);
            }
            s.set(x + (a.cos() * 1.5).round() as i32, y, z + (a.sin() * 1.5).round() as i32, POLLEN);
        } else {
            s.set(x, y, z, LEAF);
            s.set(x, y - 1, z, LEAF);
        }
    }
    s.finish(0.035)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lovely_from_the_front_and_hollow_as_a_trough_behind() {
        let m = ellepige();
        assert!(m.voxels.len() > 3000 && m.voxels.len() < 16000, "{} voxels", m.voxels.len());
        // (Colours are jittered a little as the model is finished.)
        let near = |a: u32, b: u32| (0..3).all(|k| (((a >> (8 * k)) & 255) as i32 - ((b >> (8 * k)) & 255) as i32).abs() <= 16);
        let is = |c: u32, set: &[u32]| set.iter().any(|s| near(*s, c));
        let rot = [ROT, ROT_D, MOSS, FUNGUS];
        // Looking at her from the front, at her middle: gown, never rot.
        let mid = |z_pick: fn(i32, i32) -> bool| {
            let mut seen: std::collections::HashMap<(i32, i32), (i32, u32)> = std::collections::HashMap::new();
            for vx in &m.voxels {
                if vx.y < 20 || vx.y > 56 || (vx.x as f32 - CX).abs() > 2.5 {
                    continue;
                }
                let e = seen.entry((vx.x, vx.y)).or_insert((vx.z, vx.c));
                if z_pick(vx.z, e.0) {
                    *e = (vx.z, vx.c);
                }
            }
            seen
        };
        let from_front = mid(|z, best| z > best);
        let from_back = mid(|z, best| z < best);
        let front_rot = from_front.values().filter(|(_, c)| is(*c, &rot)).count();
        let back_rot = from_back.values().filter(|(_, c)| is(*c, &rot)).count();
        assert_eq!(front_rot, 0, "her front shows no rot");
        assert!(back_rot * 2 > from_back.len(), "her back is a trough of rot: {back_rot} of {}", from_back.len());
    }
}
