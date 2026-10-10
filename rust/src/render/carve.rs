//! Carving helpers shared by the sculpted characters (see `cast` and
//! `heroes`): balls, limbs along knots, upright columns, features set into the
//! front of a face, eyes, sleeves open at the cuff, and shaggy fur.

use std::f32::consts::{PI, TAU};

use glam::Vec3;

use super::characters::Sculpt;

pub(super) fn nz(x: i32, y: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9e37_79b1) ^ (y as u32).wrapping_mul(0x85eb_ca77) ^ (z as u32).wrapping_mul(0xc2b2_ae3d);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    (h & 0xffff) as f32 / 65535.0
}

pub(super) const NEIGHBORS: [(i32, i32, i32); 6] = [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)];

pub(super) fn ball(s: &mut Sculpt, c: Vec3, r: Vec3, color: u32) {
    s.ellipsoid((c.x, c.y, c.z), (r.x, r.y, r.z), 0, color);
}

/// A limb, tail or horn along a path of (point, radius) knots.
pub(super) fn limb(s: &mut Sculpt, knots: &[(Vec3, f32)], c: u32) {
    for w in knots.windows(2) {
        let (a, b) = (w[0].0, w[1].0);
        s.capsule((a.x, a.y, a.z), (b.x, b.y, b.z), w[0].1, w[1].1, c);
    }
}

pub(super) fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

/// An upright elliptical column from `y0` to `y1`.
#[allow(clippy::too_many_arguments)]
pub(super) fn column(s: &mut Sculpt, cx: f32, cz: f32, rx: f32, rz: f32, y0: i32, y1: i32, c: u32) {
    for y in y0..=y1 {
        for z in (cz - rz).floor() as i32..=(cz + rz).ceil() as i32 {
            for x in (cx - rx).floor() as i32..=(cx + rx).ceil() as i32 {
                let (dx, dz) = ((x as f32 - cx) / rx, (z as f32 - cz) / rz);
                if dx * dx + dz * dz <= 1.0 {
                    s.set(x, y, z, c);
                }
            }
        }
    }
}

/// The frontmost filled z at (x, y), if any: where a face's features go on a round head.
pub(super) fn front(s: &Sculpt, x: i32, y: i32, d: i32) -> Option<i32> {
    (0..d).rev().find(|&z| s.get(x, y, z).is_some())
}

/// Paints the frontmost voxel at (x, y), or sets one `out` voxels in front of it.
pub(super) fn face(s: &mut Sculpt, x: i32, y: i32, d: i32, out: i32, c: u32) {
    if let Some(z) = front(s, x, y, d) {
        if out == 0 {
            s.recolor(x, y, z, c);
        } else {
            s.set(x, y, z + out, c);
        }
    }
}

/// Whether a filled voxel shows: one of its faces is open.
pub(super) fn open(s: &Sculpt, x: i32, y: i32, z: i32) -> bool {
    s.get(x, y, z).is_some() && NEIGHBORS.iter().any(|&(dx, dy, dz)| s.get(x + dx, y + dy, z + dz).is_none())
}

/// Angle round an upright axis at (cx, cz), from 0 to 1.
pub(super) fn turn(x: i32, z: i32, cx: f32, cz: f32) -> f32 {
    ((z as f32 - cz).atan2(x as f32 - cx) + PI) / TAU
}

/// Scales a colour's brightness (keeping its material byte).
pub(super) fn scale(c: u32, k: f32) -> u32 {
    let ch = |s: u32| ((((c >> s) & 255) as f32 * k).clamp(0.0, 255.0) as u32) << s;
    ch(16) | ch(8) | ch(0) | (c & 0xff00_0000)
}

/// A rosier version of a skin tone, for cheeks.
pub(super) fn blush(c: u32) -> u32 {
    let (r, g, b) = ((c >> 16) & 255, (c >> 8) & 255, c & 255);
    ((r + 10).min(255) << 16) | ((g * 84 / 100) << 8) | (b * 80 / 100) | (c & 0xff00_0000)
}

/**
 * Shaggy fur: strands standing out of the open faces of `coat` and hanging
 * down, up to `len` voxels long, the longest ending in `tip`. Each strand
 * belongs to the part of the voxel it grows from.
 */
#[allow(clippy::too_many_arguments)]
pub(super) fn shag(s: &mut Sculpt, dims: (i32, i32, i32), coat: u32, tuft: u32, tip: u32, density: f32, len: i32) {
    let (w, h, d) = dims;
    let mut add = vec![];
    for y in 1..h {
        for z in 0..d {
            for x in 0..w {
                if s.get(x, y, z) != Some(coat) {
                    continue;
                }
                let tag = s.tag_at(x, y, z);
                for (i, (dx, dy, dz)) in [(1, 0, 0), (-1, 0, 0), (0, 0, 1), (0, 0, -1), (0, -1, 0)].into_iter().enumerate() {
                    if s.get(x + dx, y + dy, z + dz).is_some() || nz(x * 3 + dx, y * 5 + dy, z * 7 + dz) <= 1.0 - density {
                        continue;
                    }
                    let n = (1 + (nz(x + 17, y * 3, z + i as i32) * len as f32) as i32).min(len);
                    for k in 0..n {
                        let c = if n > 1 && k == n - 1 { tip } else { tuft };
                        add.push((x + dx, y + dy - k, z + dz, c, tag));
                    }
                }
            }
        }
    }
    for (x, y, z, c, tag) in add {
        if s.get(x, y, z).is_none() {
            s.part(tag);
            s.set(x, y, z, c);
        }
    }
    s.part(0);
}

/**
 * A wide sleeve along `knots`, open at its end: the cloth hollowed out at the
 * cuff and its inside in the shadow of `lining`, so a hand can come out of it.
 */
pub(super) fn sleeve(s: &mut Sculpt, knots: &[(Vec3, f32)], cloth: u32, lining: u32) {
    limb(s, knots, cloth);
    let n = knots.len();
    let (a, b, r) = (knots[n - 2].0, knots[n - 1].0, knots[n - 1].1);
    let dir = (b - a).normalize();
    let reach = r + 1.5;
    for y in (b.y - reach).floor() as i32..=(b.y + reach).ceil() as i32 {
        for z in (b.z - reach).floor() as i32..=(b.z + reach).ceil() as i32 {
            for x in (b.x - reach).floor() as i32..=(b.x + reach).ceil() as i32 {
                let p = Vec3::new(x as f32, y as f32, z as f32) - b;
                let along = p.dot(dir);
                let radial = (p - dir * along).length();
                if radial < r - 1.5 {
                    if along > 0.6 {
                        s.clear(x, y, z);
                    } else if along > -1.2 {
                        s.recolor(x, y, z, lining);
                    }
                }
            }
        }
    }
}

pub(super) const EYE_WHITE: u32 = 0xf2eee6;
pub(super) const GLINT: u32 = 0xffffff;
pub(super) const PUPIL: u32 = 0x120e0c;

/**
 * An eye, its outer corner at column `outer` and its lower row at `y`: the
 * white at the corner, the iris inward (`inward` is +1 or -1) with its pupil
 * and a glint of light, and over it a line of `lid` (lashes, or a heavy lid).
 */
pub(super) fn eye(s: &mut Sculpt, outer: i32, y: i32, inward: i32, d: i32, iris: u32, lid: u32) {
    let at = |k: i32| outer + k * inward;
    face(s, at(0), y, d, 0, EYE_WHITE);
    face(s, at(0), y + 1, d, 0, EYE_WHITE);
    face(s, at(1), y, d, 0, iris);
    face(s, at(2), y, d, 0, iris);
    face(s, at(1), y + 1, d, 0, GLINT);
    face(s, at(2), y + 1, d, 0, PUPIL);
    for k in 0..3 {
        face(s, at(k), y + 2, d, 0, lid);
    }
}

/**
 * A rounded box: every voxel within a superellipsoid about `c` with radii `r`
 * (the higher `p`, the squarer), clipped to `y >= ymin`.
 */
pub(super) fn rounded(s: &mut Sculpt, c: Vec3, r: Vec3, p: f32, ymin: i32, color: u32) {
    for y in ((c.y - r.y).floor() as i32).max(ymin)..=(c.y + r.y).ceil() as i32 {
        for z in (c.z - r.z).floor() as i32..=(c.z + r.z).ceil() as i32 {
            for x in (c.x - r.x).floor() as i32..=(c.x + r.x).ceil() as i32 {
                let q = (Vec3::new(x as f32, y as f32, z as f32) - c) / r;
                if q.x.abs().powf(p) + q.y.abs().powf(p) + q.z.abs().powf(p) <= 1.0 {
                    s.set(x, y, z, color);
                }
            }
        }
    }
}

/// An upright column with a rounded-square section (a superellipse of power `p`) from `y0` to `y1`.
#[allow(clippy::too_many_arguments)]
pub(super) fn slab(s: &mut Sculpt, cx: f32, cz: f32, rx: f32, rz: f32, p: f32, y0: i32, y1: i32, c: u32) {
    for y in y0..=y1 {
        for z in (cz - rz).floor() as i32..=(cz + rz).ceil() as i32 {
            for x in (cx - rx).floor() as i32..=(cx + rx).ceil() as i32 {
                let (dx, dz) = ((x as f32 - cx) / rx, (z as f32 - cz) / rz);
                if dx.abs().powf(p) + dz.abs().powf(p) <= 1.0 {
                    s.set(x, y, z, c);
                }
            }
        }
    }
}
