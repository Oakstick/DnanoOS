//! The sixth stage's cast, the legends, sculpted as finely as the third's (see `foes3`), the
//! giants at a coarser voxel so that they cost no more: Tepegöz, the one-eyed giant of the Book
//! of Dede Korkut, bald and black-bearded, his single eye high on his brow, a wolf's pelt about
//! his loins and a tree trunk bound with iron for a club; the gashadokuro, a starving skeleton
//! fifteen times the height of a man, stooped, its huge skull hung forward and its long arms
//! reaching, the skulls of the dead it is made of fused into its bones; the yedi başlı ejder, a
//! dragon of green-bronze scales with seven horned heads on long necks fanned from its shoulders
//! and its tail curled round its side; and the goryō, the vengeful ghost of a court noble in a
//! black robe and a tall cap, its hair afloat, its hands crackling with lightning, its robe
//! fading into wisps. Each faces +z, stands (or floats) in the middle of its grid, where its rig
//! sets it down, and is sculpted in the parts its rig swings.
//!
//! Between them they fight all three ways (COMBAT.md §10). Tepegöz's body is proof against blade
//! and arrow, so only his one eye can be hurt; he swings his club (melee). The gashadokuro is
//! heard before it is seen, a ringing in the ears, then its hands close on you (melee: a grab).
//! The ejder's seven heads strike one after another (body). The goryō calls lightning down on
//! you and cannot be touched except while its wrath is upon it (distance).

use std::f32::consts::{PI, TAU};

use glam::Vec3;

use super::carve::*;
use super::characters::{mat, Sculpt};
use super::voxels::{Collected, PartDef};

mod pal {
    use super::mat;
    pub const CAVITY: u32 = 0x170f0e | mat::SKIN;
    pub const IRON: u32 = 0x4c5058 | mat::METAL;
    pub const IRON_L: u32 = 0x868c96 | mat::METAL;
    pub const IRON_D: u32 = 0x2c2e34 | mat::METAL;
    // Tepegöz.
    pub const TP_SKIN: u32 = 0xb47a54 | mat::SKIN;
    pub const TP_SKIN_M: u32 = 0xa6724e | mat::SKIN;
    pub const TP_SKIN_D: u32 = 0x8c5a3c | mat::SKIN;
    pub const TP_SKIN_DD: u32 = 0x5e3a26 | mat::SKIN;
    pub const TP_RUDDY: u32 = 0xae6c50 | mat::SKIN;
    pub const TP_SCAR: u32 = 0xdcae94 | mat::SKIN;
    pub const TP_LIP: u32 = 0x7a3e34 | mat::SKIN;
    pub const TP_NAIL: u32 = 0xd2bc98 | mat::IVORY;
    pub const TP_BEARD: u32 = 0x1c1715 | mat::FUR;
    pub const TP_BEARD_L: u32 = 0x3a332e | mat::FUR;
    pub const TP_WHITE: u32 = 0xf0ece0 | mat::SLIME;
    pub const TP_WHITE_D: u32 = 0xcfc5b4 | mat::SLIME;
    pub const TP_VEIN: u32 = 0xc04a40 | mat::SLIME;
    pub const TP_IRIS: u32 = 0x4e2c12 | mat::SLIME;
    pub const TP_IRIS_L: u32 = 0x8a5a24 | mat::SLIME;
    pub const TP_PUPIL: u32 = 0x0e0a08 | mat::SLIME;
    pub const TP_GLINT: u32 = 0xffffff | mat::SLIME;
    pub const TP_TOOTH: u32 = 0xe2d6b4 | mat::IVORY;
    pub const TP_TOOTH_D: u32 = 0xa89664 | mat::IVORY;
    pub const TP_FUR: u32 = 0x6c6256 | mat::FUR;
    pub const TP_FUR_D: u32 = 0x443e38 | mat::FUR;
    pub const TP_FUR_L: u32 = 0x928878 | mat::FUR;
    pub const TP_FUR_TIP: u32 = 0xb8ae9a | mat::FUR;
    pub const TP_ROPE: u32 = 0xb49c68 | mat::CLOTH;
    pub const TP_ROPE_D: u32 = 0x86703e | mat::CLOTH;
    pub const TP_STRAP: u32 = 0x5e3e24 | mat::LEATHER;
    pub const TP_STRAP_D: u32 = 0x3e2816 | mat::LEATHER;
    pub const TP_BARK: u32 = 0x5c4028 | mat::WOOD;
    pub const TP_BARK_D: u32 = 0x3a2817 | mat::WOOD;
    pub const TP_BARK_L: u32 = 0x7e5a36 | mat::WOOD;
    pub const TP_HEART: u32 = 0xb08c5c | mat::WOOD;
    pub const TP_HEART_D: u32 = 0x8a6a40 | mat::WOOD;
    pub const RUST: u32 = 0x7a4a2c | mat::METAL;
    pub const RING: u32 = 0xf4c444 | mat::GOLD;
    pub const RING_D: u32 = 0xb88a2c | mat::GOLD;
    pub const TURQUOISE: u32 = 0x3cc4b4 | mat::METAL;
    // The gashadokuro.
    pub const GK_BONE: u32 = 0xdccb96 | mat::IVORY;
    pub const GK_BONE_L: u32 = 0xefe2b6 | mat::IVORY;
    pub const GK_BONE_D: u32 = 0xb4a274 | mat::IVORY;
    pub const GK_JOINT: u32 = 0x8e7a56 | mat::IVORY;
    pub const GK_CARTILAGE: u32 = 0xa49a84 | mat::IVORY;
    pub const GK_GRIME: u32 = 0x7a6a4c | mat::IVORY;
    pub const GK_CRACK: u32 = 0x2e2418 | mat::IVORY;
    pub const GK_SOCKET: u32 = 0x0b0907 | mat::IVORY;
    pub const GK_SKULL: u32 = 0xccb88c | mat::IVORY;
    pub const GK_TOOTH: u32 = 0xe8e0c8 | mat::IVORY;
    pub const GK_TOOTH_D: u32 = 0xa08e6a | mat::IVORY;
    pub const GK_EYE: u32 = 0xe2fff0 | mat::GLOW;
    pub const GK_EYE_G: u32 = 0x6ef0a0 | mat::GLOW;
    pub const GK_SHROUD: u32 = 0xb4ae9e | mat::CLOTH;
    pub const GK_SHROUD_D: u32 = 0x86806e | mat::CLOTH;
    pub const GK_SHROUD_L: u32 = 0xcec8b6 | mat::CLOTH;
    // The ejder. (Its hide is laid on as `EJ_RAW` and patterned into scales at the end.)
    pub const EJ_RAW: u32 = 0x5b6f35 | mat::LEATHER;
    pub const EJ_SCALE: u32 = 0x5a6e34 | mat::LEATHER;
    pub const EJ_SCALE_D: u32 = 0x34421f | mat::LEATHER;
    pub const EJ_BACK: u32 = 0x45552a | mat::LEATHER;
    pub const EJ_SCALE_L: u32 = 0x7a8c44 | mat::LEATHER;
    pub const EJ_BRONZE: u32 = 0xa08a3a | mat::LEATHER;
    pub const EJ_BELLY: u32 = 0xe4d28c | mat::IVORY;
    pub const EJ_BELLY_D: u32 = 0xbea868 | mat::IVORY;
    pub const EJ_HORN: u32 = 0xdccfae | mat::IVORY;
    pub const EJ_HORN_D: u32 = 0x9a8a66 | mat::IVORY;
    pub const EJ_HORN_TIP: u32 = 0x3a3226 | mat::IVORY;
    pub const EJ_SPINE: u32 = 0x6e5e30 | mat::IVORY;
    pub const EJ_FRILL: u32 = 0xa83c26 | mat::LEATHER;
    pub const EJ_FRILL_D: u32 = 0x6e2216 | mat::LEATHER;
    pub const EJ_MOUTH: u32 = 0x5a1818 | mat::SKIN;
    pub const EJ_TONGUE: u32 = 0xb43448 | mat::SKIN;
    pub const EJ_TOOTH: u32 = 0xf2ead2 | mat::IVORY;
    pub const EJ_EYE: u32 = 0xffcc22 | mat::GLOW;
    pub const EJ_NOSTRIL: u32 = 0x140e0a | mat::LEATHER;
    pub const EJ_CLAW: u32 = 0xd4c8a8 | mat::IVORY;
    pub const EJ_CLAW_D: u32 = 0x2c2620 | mat::IVORY;
    // The goryō.
    pub const GR_ROBE: u32 = 0x212844 | mat::CLOTH;
    pub const GR_ROBE_D: u32 = 0x121524 | mat::CLOTH;
    pub const GR_ROBE_L: u32 = 0x363e64 | mat::CLOTH;
    pub const GR_ROBE_W: u32 = 0x2c3354 | mat::CLOTH;
    pub const GR_WISP: u32 = 0x3a4670 | mat::CLOTH;
    pub const GR_WISP_L: u32 = 0x6474a4 | mat::CLOTH;
    pub const GR_WISP_T: u32 = 0x9cacd4 | mat::CLOTH;
    pub const GR_WISP_G: u32 = 0x8cb4f0 | mat::GLOW;
    pub const GR_LAYER_W: u32 = 0xe6e2da | mat::CLOTH;
    pub const GR_LAYER_R: u32 = 0x8c1c26 | mat::CLOTH;
    pub const GR_BELT: u32 = 0x161418 | mat::LEATHER;
    pub const GR_JADE: u32 = 0xd2e2d0 | mat::IVORY;
    pub const GR_UME: u32 = 0xb8a466 | mat::GOLD;
    pub const GR_UME_D: u32 = 0x7a6a3a | mat::GOLD;
    pub const GR_SKIN: u32 = 0xd6dce6 | mat::SKIN;
    pub const GR_SKIN_D: u32 = 0xa6aebe | mat::SKIN;
    pub const GR_SKIN_DD: u32 = 0x7c8498 | mat::SKIN;
    pub const GR_LIP: u32 = 0x5c5468 | mat::SKIN;
    pub const GR_NAIL: u32 = 0x9a9480 | mat::IVORY;
    pub const GR_SOCKET: u32 = 0x0a0a12 | mat::SKIN;
    pub const GR_EYE: u32 = 0xe0f4ff | mat::GLOW;
    pub const GR_EYE_B: u32 = 0x5aa4ff | mat::GLOW;
    pub const GR_SPARK: u32 = 0xeef8ff | mat::GLOW;
    pub const GR_SPARK_B: u32 = 0x78b6ff | mat::GLOW;
    pub const GR_HAIR: u32 = 0x0e0d12 | mat::FUR;
    pub const GR_HAIR_L: u32 = 0x272532 | mat::FUR;
    pub const GR_CAP: u32 = 0x1a1a22 | mat::LEATHER;
    pub const GR_CAP_L: u32 = 0x3c3c4a | mat::LEATHER;
    pub const GR_CORD: u32 = 0xd8d2c8 | mat::CLOTH;
    pub const GR_PAPER: u32 = 0xe6dcc0 | mat::CLOTH;
    pub const GR_INK: u32 = 0x1c1616 | mat::CLOTH;
    pub const GR_SEAL: u32 = 0xb02222 | mat::CLOTH;
}
use pal::*;

// ------------------------------------------------------------------- helpers

/// Sets the voxel nearest `p`.
fn put(s: &mut Sculpt, p: Vec3, c: u32) {
    s.set(p.x.round() as i32, p.y.round() as i32, p.z.round() as i32, c);
}

/// A tapered capsule from `a` (radius `ra`) to `b` (radius `rb`).
fn tube(s: &mut Sculpt, a: Vec3, b: Vec3, ra: f32, rb: f32, c: u32) {
    s.capsule((a.x, a.y, a.z), (b.x, b.y, b.z), ra, rb, c);
}

/// A rounded box about `c` turned to the axes `ax` (unit vectors), its half-sizes along them
/// `r`, the higher `p` the squarer.
fn block(s: &mut Sculpt, c: Vec3, ax: [Vec3; 3], r: Vec3, p: f32, col: u32) {
    let reach = r.max_element() + 1.0;
    for y in (c.y - reach).floor() as i32..=(c.y + reach).ceil() as i32 {
        for z in (c.z - reach).floor() as i32..=(c.z + reach).ceil() as i32 {
            for x in (c.x - reach).floor() as i32..=(c.x + reach).ceil() as i32 {
                let d = v(x as f32, y as f32, z as f32) - c;
                let q = v(d.dot(ax[0]) / r.x, d.dot(ax[1]) / r.y, d.dot(ax[2]) / r.z);
                if q.x.abs().powf(p) + q.y.abs().powf(p) + q.z.abs().powf(p) <= 1.0 {
                    s.set(x, y, z, col);
                }
            }
        }
    }
}

/**
 * A body swept along `knots` (centre, radius), each voxel coloured by `paint(voxel, its
 * nearest point on the axis, how far along the whole body that is, 0 to 1)`.
 */
fn sweep(s: &mut Sculpt, knots: &[(Vec3, f32)], paint: impl Fn(Vec3, Vec3, f32) -> u32) {
    let total: f32 = knots.windows(2).map(|w| (w[1].0 - w[0].0).length()).sum::<f32>().max(1e-4);
    let mut run = 0.0;
    for w in knots.windows(2) {
        let ((a, ra), (b, rb)) = (w[0], w[1]);
        let ab = b - a;
        let len = ab.length().max(1e-4);
        let r = ra.max(rb) + 1.0;
        let (lo, hi) = (a.min(b) - Vec3::splat(r), a.max(b) + Vec3::splat(r));
        for y in lo.y.floor() as i32..=hi.y.ceil() as i32 {
            for z in lo.z.floor() as i32..=hi.z.ceil() as i32 {
                for x in lo.x.floor() as i32..=hi.x.ceil() as i32 {
                    let p = Vec3::new(x as f32, y as f32, z as f32);
                    let t = ((p - a).dot(ab) / (len * len)).clamp(0.0, 1.0);
                    let c = a + ab * t;
                    let rad = ra + (rb - ra) * t;
                    if (p - c).length_squared() <= rad * rad {
                        s.set(x, y, z, paint(p, c, (run + t * len) / total));
                    }
                }
            }
        }
        run += len;
    }
}

/// Recolours the voxels of colour `from` within `r` of `c` (a knuckle or a kneecap reddened).
fn tint(s: &mut Sculpt, c: Vec3, r: f32, from: u32, to: u32) {
    for y in (c.y - r).floor() as i32..=(c.y + r).ceil() as i32 {
        for z in (c.z - r).floor() as i32..=(c.z + r).ceil() as i32 {
            for x in (c.x - r).floor() as i32..=(c.x + r).ceil() as i32 {
                if s.get(x, y, z) == Some(from) && (v(x as f32, y as f32, z as f32) - c).length() <= r {
                    s.recolor(x, y, z, to);
                }
            }
        }
    }
}

/// A box blur `2r + 1` wide of a grid of `dims` along one axis (0 x, 1 y, 2 z).
fn blur(f: &[f32], dims: (i32, i32, i32), axis: usize, r: i32) -> Vec<f32> {
    let (w, h, d) = dims;
    let (len, stride) = [(w, 1), (h, w * d), (d, w)][axis];
    let ends = [w, h, d].map(|n| n.max(1));
    let mut g = vec![0.0; f.len()];
    for y in 0..if axis == 1 { 1 } else { ends[1] } {
        for z in 0..if axis == 2 { 1 } else { ends[2] } {
            for x in 0..if axis == 0 { 1 } else { ends[0] } {
                let base = (y * d + z) * w + x;
                let at = |i: i32| (base + i * stride) as usize;
                let mut sum: f32 = (0..=r.min(len - 1)).map(|i| f[at(i)]).sum();
                for i in 0..len {
                    g[at(i)] = sum / (2 * r + 1) as f32;
                    if i + r + 1 < len {
                        sum += f[at(i + r + 1)];
                    }
                    if i - r >= 0 {
                        sum -= f[at(i - r)];
                    }
                }
            }
        }
    }
    g
}

/**
 * Shades a model as if sculpted: each open voxel that `on` picks darkens in a crease or a
 * hollow (under the pecs, between the fingers, in the folds of a robe) and lightens on a swell,
 * by how much of the space about it is filled, `depth` saying how strongly. Last of all, since
 * it shifts the colours off the palette.
 */
fn hollows(s: &mut Sculpt, dims: (i32, i32, i32), on: impl Fn(u32) -> bool, depth: f32) {
    // The bulk at half the resolution (each cell the share of its eight voxels that are filled),
    // blurred three times along each axis: about as soft as twice seven voxels wide at full.
    let (w, h, d) = dims;
    let half = ((w + 1) / 2, (h + 1) / 2, (d + 1) / 2);
    let at = |x: i32, y: i32, z: i32| ((y * half.2 + z) * half.0 + x) as usize;
    let mut f = vec![0.0f32; (half.0 * half.1 * half.2) as usize];
    for y in 0..h {
        for z in 0..d {
            for x in 0..w {
                if s.get(x, y, z).is_some() {
                    f[at(x / 2, y / 2, z / 2)] += 0.125;
                }
            }
        }
    }
    for _ in 0..3 {
        for axis in 0..3 {
            f = blur(&f, half, axis, 1);
        }
    }
    // Read back between the cells' centres.
    let sample = |x: i32, y: i32, z: i32| {
        let p = Vec3::new(x as f32, y as f32, z as f32) * 0.5 - Vec3::splat(0.25);
        let (i, t) = (p.floor(), p - p.floor());
        let mut sum = 0.0;
        for k in 0..8 {
            let o = Vec3::new((k & 1) as f32, (k >> 1 & 1) as f32, (k >> 2 & 1) as f32);
            let c = (i + o).as_ivec3().clamp(glam::IVec3::ZERO, glam::IVec3::new(half.0 - 1, half.1 - 1, half.2 - 1));
            let wgt = (Vec3::ONE - o + (o * 2.0 - Vec3::ONE) * t).element_product();
            sum += f[at(c.x, c.y, c.z)] * wgt;
        }
        sum
    };
    let mut out = vec![];
    for y in 0..h {
        for z in 0..d {
            for x in 0..w {
                let Some(c) = s.get(x, y, z) else { continue };
                if on(c) && open(s, x, y, z) {
                    out.push((x, y, z, c, sample(x, y, z)));
                }
            }
        }
    }
    // (Half filled is about a gently rounded surface: left as it is.)
    for (x, y, z, c, k) in out {
        s.recolor(x, y, z, scale(c, (1.0 + (0.5 - k) * depth).clamp(0.55, 1.2)));
    }
}

/// A point on a Catmull-Rom spline through `p`, `t` running from 0 at the first to 1 at the last.
fn spline(p: &[Vec3], t: f32) -> Vec3 {
    let n = p.len() - 1;
    let f = (t.clamp(0.0, 1.0) * n as f32).min(n as f32 - 1e-4);
    let i = f as usize;
    let u = f - i as f32;
    let (p0, p1, p2, p3) = (p[i.saturating_sub(1)], p[i], p[(i + 1).min(n)], p[(i + 2).min(n)]);
    (p1 * 2.0 + (p2 - p0) * u + (p0 * 2.0 - p1 * 5.0 + p2 * 4.0 - p3) * (u * u) + (p1 * 3.0 - p0 - p2 * 3.0 + p3) * (u * u * u)) * 0.5
}

/// `n + 1` points evenly along the spline through `p`, by length, each with its tangent.
fn along(p: &[Vec3], n: usize) -> Vec<(Vec3, Vec3)> {
    let fine: Vec<Vec3> = (0..=400).map(|k| spline(p, k as f32 / 400.0)).collect();
    let mut run = vec![0.0f32];
    for w in fine.windows(2) {
        run.push(run[run.len() - 1] + (w[1] - w[0]).length());
    }
    let total = run[run.len() - 1].max(1e-4);
    (0..=n)
        .map(|k| {
            let want = total * k as f32 / n as f32;
            let j = run.iter().position(|&r| r >= want).unwrap_or(400).clamp(1, 400);
            let t = (want - run[j - 1]) / (run[j] - run[j - 1]).max(1e-4);
            (fine[j - 1].lerp(fine[j], t), (fine[j] - fine[j - 1]).normalize_or_zero())
        })
        .collect()
}

/// A thin plate filling the triangle `a`, `b`, `c`, `th` thick (a shoulder blade).
fn plate(s: &mut Sculpt, a: Vec3, b: Vec3, c: Vec3, th: f32, col: u32) {
    let n = (b - a).cross(c - a).normalize_or_zero();
    let (lo, hi) = (a.min(b).min(c) - Vec3::splat(th + 1.0), a.max(b).max(c) + Vec3::splat(th + 1.0));
    let (v0, v1) = (b - a, c - a);
    let (d00, d01, d11) = (v0.dot(v0), v0.dot(v1), v1.dot(v1));
    let den = (d00 * d11 - d01 * d01).max(1e-6);
    for y in lo.y.floor() as i32..=hi.y.ceil() as i32 {
        for z in lo.z.floor() as i32..=hi.z.ceil() as i32 {
            for x in lo.x.floor() as i32..=hi.x.ceil() as i32 {
                let p = v(x as f32, y as f32, z as f32) - a;
                if p.dot(n).abs() > th * 0.5 {
                    continue;
                }
                let (d20, d21) = (p.dot(v0), p.dot(v1));
                let (bv, bw) = ((d11 * d20 - d01 * d21) / den, (d00 * d21 - d01 * d20) / den);
                if bv >= -0.02 && bw >= -0.02 && bv + bw <= 1.02 {
                    s.set(x, y, z, col);
                }
            }
        }
    }
}

/// A frame for something facing `f`: (right, up, forward), the up as near to +y as it can be.
fn frame(f: Vec3) -> (Vec3, Vec3, Vec3) {
    let f = f.normalize();
    let r = Vec3::Y.cross(f).normalize_or(Vec3::X);
    (r, f.cross(r), f)
}

// ------------------------------------------------------------------- Tepegöz

/// Tepegöz's grid: 100 wide, 138 tall, 100 deep (his club reaches out and down before his right
/// foot). Built at 0.02 world units a voxel, he stands about 2.7 tall.
pub const TEPEGOZ_SIZE: (i32, i32, i32) = (100, 138, 100);
const TP_X: f32 = 49.5;
const TP_Z: f32 = 49.5;
/// The way his club points from his fist, down and forward and a little out.
const TP_CLUB: Vec3 = Vec3::new(0.18, -0.76, 0.62);
/// The middle of his one eye, high on his brow (the game lights it when it comes within reach).
pub const TEPEGOZ_EYE: [f32; 3] = [49.5, 123.6, 70.6];

/// Hips, shoulders and neck; the club goes with the right hand.
pub const TEPEGOZ_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [TP_X - 10.5, 57.0, TP_Z] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [TP_X + 10.5, 57.0, TP_Z] },
    PartDef { name: "leftArm", boxes: &[], pivot: [TP_X - 23.5, 97.0, TP_Z + 4.0] },
    PartDef { name: "rightArm", boxes: &[], pivot: [TP_X + 23.5, 97.0, TP_Z + 4.0] },
    PartDef { name: "head", boxes: &[], pivot: [TP_X, 107.0, TP_Z + 8.5] },
];

/// The club's trunk, how thick it is `a` voxels along from the fist.
fn tp_club_radius(a: f32) -> f32 {
    let knots = [(-8.0f32, 2.7f32), (0.0, 2.9), (8.0, 3.3), (18.0, 4.3), (30.0, 5.8), (40.0, 6.9), (46.0, 7.4), (50.0, 7.0)];
    for w in knots.windows(2) {
        if a <= w[1].0 {
            let t = ((a - w[0].0) / (w[1].0 - w[0].0)).clamp(0.0, 1.0);
            return w[0].1 + (w[1].1 - w[0].1) * t;
        }
    }
    knots[knots.len() - 1].1
}

/**
 * Tepegöz's club: a tree trunk torn up whole, from the fist at `grip` along `dir`, its handle
 * worn smooth, its bark ridged and knotted, the stubs of its branches broken off pale, and its
 * root end bound with iron bands and studded with nails.
 */
fn tp_club(s: &mut Sculpt, grip: Vec3, dir: Vec3) {
    let dir = dir.normalize();
    let side = dir.cross(Vec3::X).normalize();
    let up = side.cross(dir);
    let len = 50.0;
    let bend = |a: f32| side * (1.8 * (a / len * PI).sin()) + up * (1.0 * (a / len * TAU).sin());
    let lo = grip.min(grip + dir * len) - Vec3::splat(11.0);
    let hi = grip.max(grip + dir * len) + Vec3::splat(11.0);
    let bands = [35.0f32, 44.5];
    for y in lo.y.floor() as i32..=hi.y.ceil() as i32 {
        for z in lo.z.floor() as i32..=hi.z.ceil() as i32 {
            for x in lo.x.floor() as i32..=hi.x.ceil() as i32 {
                let p = Vec3::new(x as f32, y as f32, z as f32) - grip;
                let a = p.dot(dir);
                if !(-9.0..=len + 1.0).contains(&a) {
                    continue;
                }
                let q = p - dir * a - bend(a.clamp(0.0, len));
                let th = q.dot(up).atan2(q.dot(side));
                let gnarl = 1.0 + 0.09 * (th * 3.0 + a * 0.23).sin() + 0.06 * (th * 5.0 - a * 0.11).sin();
                let mut r = tp_club_radius(a) * if a > 4.0 { gnarl } else { 1.0 };
                // Rounded ends: the worn knob of the handle, the sawn root.
                if a < -6.0 {
                    r *= (1.0 - ((-6.0 - a) / 3.0).powi(2)).max(0.0).sqrt();
                }
                if a > len - 1.5 {
                    r *= (1.0 - ((a - len + 1.5) / 2.5).powi(2)).max(0.0).sqrt();
                }
                let rr = q.length();
                if let Some(b) = bands.iter().find(|b| (a - **b).abs() < 1.6) {
                    if rr <= tp_club_radius(*b) * 1.1 + 0.8 {
                        let rivet = (th / TAU * 9.0).rem_euclid(1.0) < 0.2 && (a - b).abs() < 0.6;
                        let edge = (a - b).abs() > 1.0;
                        s.set(
                            x,
                            y,
                            z,
                            if rivet {
                                IRON_L
                            } else if edge {
                                IRON_D
                            } else if nz(x, y, z) < 0.2 {
                                RUST
                            } else {
                                IRON
                            },
                        );
                    }
                    continue;
                }
                if rr > r {
                    continue;
                }
                let c = if a < 6.0 && a > -6.0 {
                    // The handle, worn smooth by his grip.
                    if (a * 1.3 + th * 2.0).sin() > 0.75 {
                        TP_BARK
                    } else {
                        TP_HEART_D
                    }
                } else if a <= -6.0 || a >= len - 0.8 {
                    // The cut ends: pale wood, ringed.
                    if (rr * 1.7).sin() > 0.3 {
                        TP_HEART_D
                    } else {
                        TP_HEART
                    }
                } else {
                    let ridge = (th * 7.0 + a * 0.09 + (a * 0.4).sin() * 0.5).sin();
                    if ridge > 0.6 || nz(x / 2, y / 2, z / 2) < 0.12 {
                        TP_BARK_D
                    } else if ridge < -0.8 || nz(x, y, z) > 0.9 {
                        TP_BARK_L
                    } else {
                        TP_BARK
                    }
                };
                s.set(x, y, z, c);
            }
        }
    }
    // Knots in the bark, and two branches broken off short.
    for (a, th, r) in [(15.0f32, 0.8f32, 1.8f32), (24.0, 3.6, 2.3), (29.0, 5.4, 2.0), (20.0, 2.0, 1.6)] {
        let n = side * th.cos() + up * th.sin();
        let c = grip + dir * a + bend(a) + n * tp_club_radius(a) * 0.95;
        ball(s, c, Vec3::splat(r), TP_BARK_L);
        put(s, c + n * (r * 0.8), TP_BARK_D);
    }
    for (a, th, l) in [(26.0f32, 4.4f32, 6.0f32), (12.0, 1.2, 4.5)] {
        let n = side * th.cos() + up * th.sin();
        let base = grip + dir * a + bend(a) + n * tp_club_radius(a) * 0.6;
        let tip = base + (n * 1.0 + dir * 0.6).normalize() * l;
        tube(s, base, tip, 2.1, 1.6, TP_BARK);
        ball(s, tip, Vec3::splat(1.5), TP_HEART);
    }
    // Nails driven through the root end, their points standing out.
    for k in 0..12 {
        let a = [39.5f32, 48.0][k % 2];
        let th = k as f32 * TAU / 12.0 + 0.2;
        let n = side * th.cos() + up * th.sin();
        let base = grip + dir * a + bend(a) + n * (tp_club_radius(a) * 0.95);
        tube(s, base, base + n * 2.8, 1.1, 0.35, IRON_D);
        put(s, base + n * 0.6, IRON_L);
    }
}

/**
 * One of Tepegöz's hands at the wrist `wr` of an arm hanging at his `out` side: the left open,
 * its fingers curled toward his thigh; the right a fist round the haft of his club, his mother's
 * ring of gold on its forefinger.
 */
fn tp_hand(s: &mut Sculpt, wr: Vec3, out: f32) {
    if out < 0.0 {
        let palm = wr + v(out * 0.4, -6.0, 0.8);
        rounded(s, palm, v(3.6, 6.0, 6.0), 2.2, 0, TP_SKIN);
        for k in 0..4 {
            let r = [1.95, 2.0, 1.9, 1.7][k];
            let kn = v(palm.x + out * 0.6, palm.y - 5.0, palm.z + 4.2 - k as f32 * 2.8);
            ball(s, kn + v(out * 1.6, 0.6, 0.0), Vec3::splat(r * 0.9), TP_SKIN);
            tint(s, kn + v(out * 2.4, 0.6, 0.0), 1.6, TP_SKIN, TP_RUDDY);
            let mid = kn + v(-out * 0.8, -4.8, 0.4 * (1.5 - k as f32));
            let tip = mid + v(-out * 2.8, -2.8, 0.0);
            tube(s, kn, mid, r, r * 0.92, TP_SKIN);
            tube(s, mid, tip, r * 0.92, r * 0.8, TP_SKIN);
            put(s, tip + v(-out * 0.4, -r * 0.8, 0.0), TP_NAIL);
        }
        let t0 = palm + v(-out * 1.4, 2.4, 5.0);
        let t1 = t0 + v(-out * 1.2, -4.6, 2.6);
        let t2 = t1 + v(-out * 2.2, -3.2, 0.2);
        tube(s, t0, t1, 2.2, 1.9, TP_SKIN);
        tube(s, t1, t2, 1.9, 1.5, TP_SKIN);
        put(s, t2 + v(0.0, -1.2, 0.4), TP_NAIL);
        return;
    }
    // The fist, built about the haft: `a` along it, `u` out of the back of the hand, `n` toward the wrist.
    let a = TP_CLUB.normalize();
    let u = (Vec3::X - a * a.x).normalize() * out;
    let n0 = Vec3::Y - a * a.y - u * u.y;
    let n = n0.normalize();
    let grip = wr + v(out * 0.5, -9.0, -1.0);
    tp_club(s, grip, a);
    block(s, grip + n * 3.4 + u * 0.9, [u, n, a], v(3.9, 4.9, 6.9), 2.2, TP_SKIN);
    for k in 0..4 {
        let at = grip + a * (4.5 - k as f32 * 3.0);
        let r = [2.25, 2.35, 2.25, 2.0][k];
        let knuckle = at + u * 3.9 + n * 2.5;
        let p1 = at + u * 3.8 - n * 1.7;
        let p2 = at + u * 0.2 - n * 3.9;
        let p3 = at - u * 3.3 - n * 2.1;
        let tip = at - u * 3.8 + n * 1.0;
        tube(s, knuckle, p1, r, r, TP_SKIN);
        tube(s, p1, p2, r, r * 0.95, TP_SKIN);
        tube(s, p2, p3, r * 0.95, r * 0.9, TP_SKIN);
        tube(s, p3, tip, r * 0.9, r * 0.8, TP_SKIN);
        ball(s, knuckle + u * 0.6, Vec3::splat(r * 0.95), TP_SKIN);
        tint(s, knuckle + u * 1.6, 1.9, TP_SKIN, TP_RUDDY);
        put(s, tip - u * 0.5 + n * 0.8, TP_NAIL);
        if k == 0 {
            // The peri's ring: a gold band round the forefinger, set with a turquoise.
            let m = (knuckle + p1) * 0.5;
            let along = (p1 - knuckle).normalize();
            for dy in -4..=4 {
                for dz in -4..=4 {
                    for dx in -4..=4 {
                        let p = v((m.x).round() + dx as f32, (m.y).round() + dy as f32, (m.z).round() + dz as f32);
                        let rel = p - m;
                        let ax = rel.dot(along);
                        let rad = (rel - along * ax).length();
                        if ax.abs() < 0.75 && rad < r + 0.9 {
                            put(s, p, if rad > r + 0.2 || rel.dot(u) > 0.5 { RING } else { RING_D });
                        }
                    }
                }
            }
            put(s, m + u * (r + 1.0), TURQUOISE);
            put(s, m + u * (r + 1.0) + a * 0.9, TURQUOISE);
        }
    }
    let t0 = grip + a * 5.4 - u * 2.6 + n * 3.6;
    let t1 = grip + a * 7.0 - u * 0.4 + n * 0.6;
    let t2 = grip + a * 6.6 + u * 2.6 - n * 0.6;
    tube(s, t0, t1, 2.6, 2.3, TP_SKIN);
    tube(s, t1, t2, 2.3, 1.9, TP_SKIN);
    put(s, t2 + u * 0.6 + a * 1.5, TP_NAIL);
}

/**
 * Tepegöz, the one-eyed giant of the Book of Dede Korkut, born of a shepherd and a peri: bald
 * and heavily muscled, stooped under his own weight, his one great eye in the middle of his
 * brow under a heavy lid and a heavier ridge of bone, a broad broken nose, a black beard thick
 * from ear to ear and down over his chest, broken teeth in a scowl; a bare chest scored pale
 * where blades have glanced off it, a wolf's pelt round his loins tied with a hemp rope, its
 * tail hanging behind, bare feet, a strap of hide at each wrist; on the forefinger of his right
 * hand his mother's ring of gold, and in that fist a tree trunk for a club, held low, its root
 * end bound with iron and bristling with nails.
 */
pub fn tepegoz() -> Collected {
    let (w, h, d) = TEPEGOZ_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (TP_X, TP_Z);

    // ---- legs: broad bare feet, heavy calves, thighs like tree trunks
    for (i, lx) in [cx - 10.5, cx + 10.5].into_iter().enumerate() {
        s.part(1 + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        let fx = lx + out * 1.6;
        rounded(&mut s, v(fx, 3.0, cz + 2.0), v(5.6, 3.0, 9.8), 2.3, 0, TP_SKIN);
        ball(&mut s, v(fx, 4.4, cz - 4.8), v(4.9, 4.4, 4.7), TP_SKIN);
        ball(&mut s, v(fx + out * 0.4, 4.8, cz + 5.0), v(5.8, 4.0, 5.6), TP_SKIN);
        for k in 0..5 {
            let kf = k as f32;
            let tx = fx - out * (4.0 - kf * 2.0);
            let r = [2.2, 1.7, 1.6, 1.5, 1.4][k];
            let len = [4.6, 4.4, 4.0, 3.5, 2.9][k];
            let z0 = cz + 9.8 - kf * 0.7;
            let a = v(tx, r + 0.4, z0);
            let b = v(tx + out * kf * 0.25, r * 0.95, z0 + len);
            tube(&mut s, a, b, r, r * 0.92, TP_SKIN);
            put(&mut s, b + v(0.0, r * 0.75, 0.3), TP_NAIL);
            if k == 0 {
                put(&mut s, b + v(-out * 0.9, r * 0.75, 0.3), TP_NAIL);
            }
        }
        let ankle = v(lx + out * 1.2, 8.5, cz - 1.5);
        let knee = v(lx + out * 0.8, 32.0, cz + 1.0);
        let hip = v(lx, 57.0, cz);
        tube(&mut s, ankle, knee, 4.6, 6.2, TP_SKIN);
        ball(&mut s, v(lx + out * 1.3, 19.5, cz - 2.8), v(5.4, 8.8, 5.2), TP_SKIN);
        ball(&mut s, v(lx - out * 0.9, 21.0, cz - 2.0), v(4.8, 7.6, 4.8), TP_SKIN);
        ball(&mut s, ankle + v(out * 3.5, -0.8, 0.4), Vec3::splat(1.7), TP_SKIN);
        ball(&mut s, ankle + v(-out * 3.3, 0.0, 0.4), Vec3::splat(1.7), TP_SKIN);
        ball(&mut s, knee, v(6.2, 5.6, 6.4), TP_SKIN);
        ball(&mut s, knee + v(0.0, 0.4, 5.2), v(3.3, 3.5, 2.0), TP_SKIN);
        tube(&mut s, knee + v(0.0, 1.0, 0.0), hip, 6.9, 10.0, TP_SKIN);
        ball(&mut s, v(lx + out * 1.0, 43.0, cz + 3.6), v(8.2, 10.8, 6.8), TP_SKIN);
        ball(&mut s, v(lx - out * 2.6, 38.5, cz + 3.0), v(4.6, 6.2, 5.2), TP_SKIN);
        ball(&mut s, v(lx, 45.0, cz - 3.2), v(7.8, 10.2, 6.8), TP_SKIN);
        tint(&mut s, knee + v(0.0, 0.4, 7.0), 3.4, TP_SKIN, TP_RUDDY);
        tint(&mut s, v(fx, 3.0, cz + 15.0), 4.0, TP_SKIN, TP_RUDDY);
    }

    // ---- the trunk: hips, a hard belly, a barrel of a chest stooped forward, the shoulders heaped up
    s.part(0);
    rounded(&mut s, v(cx, 60.0, cz), v(18.2, 8.5, 12.0), 2.4, 0, TP_SKIN);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 7.5, 57.0, cz - 6.5), v(8.5, 8.0, 6.5), TP_SKIN);
    }
    ball(&mut s, v(cx, 73.0, cz + 2.0), v(16.8, 12.0, 12.4), TP_SKIN);
    ball(&mut s, v(cx, 88.0, cz + 3.5), v(19.4, 14.5, 12.8), TP_SKIN);
    ball(&mut s, v(cx, 92.0, cz - 1.2), v(18.6, 14.0, 11.2), TP_SKIN);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 12.0, 85.0, cz - 0.5), v(7.0, 13.0, 9.6), TP_SKIN);
    }
    ball(&mut s, v(cx, 99.0, cz + 2.5), v(14.0, 6.6, 9.6), TP_SKIN);
    for side in [-1.0f32, 1.0] {
        tube(&mut s, v(cx + side * 4.0, 105.5, cz + 5.0), v(cx + side * 17.0, 100.5, cz + 3.5), 5.2, 4.2, TP_SKIN);
    }
    tube(&mut s, v(cx, 98.0, cz + 5.0), v(cx, 112.0, cz + 11.0), 8.0, 7.2, TP_SKIN);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 8.4, 92.0, cz + 12.4), v(9.4, 6.8, 4.9), TP_SKIN);
        tube(&mut s, v(cx + side * 2.5, 101.6, cz + 14.0), v(cx + side * 18.0, 101.0, cz + 9.0), 1.5, 1.4, TP_SKIN);
    }
    for y in [82.5f32, 77.0, 71.5] {
        for side in [-1.0f32, 1.0] {
            let x = cx + side * 3.7;
            let z = front(&s, x.round() as i32, y.round() as i32, d).unwrap_or(60) as f32;
            ball(&mut s, v(x, y, z - 0.6), v(3.3, 2.5, 1.7), TP_SKIN);
        }
    }
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 13.0, 71.0, cz + 4.5), v(5.0, 8.0, 7.0), TP_SKIN);
        for k in 0..3 {
            let k = k as f32;
            ball(&mut s, v(cx + side * 16.0, 84.5 - k * 3.2, cz + 8.0 - k * 0.5), v(2.8, 1.5, 3.4), TP_SKIN);
        }
    }
    // The furrow down his back, the navel, the nipples.
    s.paint(|x, y, z, c| {
        (c == TP_SKIN && (x as f32 - cx).abs() < 1.0 && (z as f32) < cz - 6.0 && (64..96).contains(&y)).then_some(TP_SKIN_D)
    });
    face(&mut s, cx.floor() as i32, 67, d, 0, TP_SKIN_DD);
    for side in [-1.0f32, 1.0] {
        face(&mut s, (cx + side * 9.0).round() as i32, 89, d, 0, TP_LIP);
    }

    // ---- the pelt round his loins, long before and behind, short at the hips, and the rope belt
    for y in 33..=65 {
        let flare = ((50 - y).max(0) as f32) * 0.07;
        let (rx, rz) = (20.0 + flare, 13.6 + flare);
        for z in (cz - rz - 1.0) as i32..=(cz + rz + 2.0) as i32 {
            for x in (cx - rx - 1.0) as i32..=(cx + rx + 1.0) as i32 {
                let (dx, dz) = (x as f32 - cx, z as f32 - cz - 0.6);
                let o = (dx / rx).abs().powf(2.4) + (dz / rz).abs().powf(2.4);
                let i = (dx / (rx - 2.8)).abs().powf(2.4) + (dz / (rz - 2.8)).abs().powf(2.4);
                if o > 1.0 || i < 1.0 {
                    continue;
                }
                let hem = if dx.abs() < 9.5 && dz > 0.0 {
                    34.0
                } else if dx.abs() < 10.5 && dz < 0.0 {
                    37.0
                } else {
                    51.0
                } + nz(x, 3, z) * 4.0;
                if (y as f32) < hem {
                    continue;
                }
                // A wolf's pelt: dark bands across it, paler patches, its ragged edge pale.
                let band = (dx * 0.3 + (y as f32 * 0.3).sin() * 1.2 + nz(x / 4, y / 4, z / 4) * 2.0).sin();
                let c = if (y as f32) < hem + 1.5 && nz(x, y, z) < 0.5 {
                    TP_FUR_TIP
                } else if band > 0.7 {
                    TP_FUR_D
                } else if nz(x / 3, y / 3, z / 3) > 0.72 {
                    TP_FUR_L
                } else {
                    TP_FUR
                };
                s.set(x, y, z, c);
            }
        }
    }
    // The wolf's own tail, hanging from the belt behind.
    let back = (0..d).find(|&z| s.get((cx + 4.0) as i32, 62, z).is_some()).unwrap_or(36) as f32;
    sweep(
        &mut s,
        &[
            (v(cx + 4.0, 63.0, back - 0.5), 2.6),
            (v(cx + 5.0, 54.0, back - 2.6), 3.6),
            (v(cx + 5.6, 44.0, back - 2.2), 3.4),
            (v(cx + 5.0, 36.0, back - 1.0), 2.0),
        ],
        |p, _, along| {
            if along > 0.8 {
                TP_FUR_TIP
            } else if nz(p.x as i32, p.y as i32, p.z as i32) < 0.3 {
                TP_FUR_D
            } else {
                TP_FUR
            }
        },
    );
    shag(&mut s, (w, h, d), TP_FUR, TP_FUR, TP_FUR_L, 0.24, 4);
    shag(&mut s, (w, h, d), TP_FUR_D, TP_FUR_D, TP_FUR, 0.2, 3);
    for k in 0..170 {
        let a = k as f32 / 170.0 * TAU;
        let p = v(cx + a.cos() * 20.2, 64.6 + (a * 3.0).sin() * 0.3, cz + 0.6 + a.sin() * 13.8);
        ball(&mut s, p, Vec3::splat(1.5), if (k / 3) % 2 == 0 { TP_ROPE } else { TP_ROPE_D });
    }
    let knot = v(cx - 7.0, 64.0, front(&s, (cx - 7.0) as i32, 64, d).unwrap_or(66) as f32 + 0.6);
    ball(&mut s, knot, v(2.3, 2.1, 1.9), TP_ROPE_D);
    for (dx, len) in [(-1.6f32, 12.0f32), (1.2, 9.0)] {
        tube(&mut s, knot, knot + v(dx, -len, 0.8), 1.25, 1.05, TP_ROPE);
        ball(&mut s, knot + v(dx, -len - 0.9, 0.8), Vec3::splat(1.5), TP_ROPE_D);
    }

    // ---- arms: shoulders like boulders, arms thicker than a man's waist, huge hands
    for (i, sx) in [cx - 23.5, cx + 23.5].into_iter().enumerate() {
        s.part(3 + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        let sh = v(sx, 97.0, cz + 4.0);
        let el = v(sx + out * 4.0, 74.0, cz + 3.0);
        let wr = v(sx + out * 6.0, 54.5, cz + 7.0);
        ball(&mut s, sh + v(out * 1.6, -4.2, 0.2), v(7.4, 10.2, 8.4), TP_SKIN);
        tube(&mut s, sh, el, 7.2, 6.0, TP_SKIN);
        let mid = (sh + el) * 0.5;
        ball(&mut s, mid + v(0.0, -2.0, 3.6), v(5.8, 9.0, 5.2), TP_SKIN);
        ball(&mut s, mid + v(out * 1.2, 1.0, -3.4), v(6.0, 9.6, 5.4), TP_SKIN);
        ball(&mut s, el, Vec3::splat(6.0), TP_SKIN);
        tube(&mut s, el, wr, 6.6, 4.4, TP_SKIN);
        ball(&mut s, el.lerp(wr, 0.3) + v(out * 1.6, 0.0, 1.0), v(5.8, 8.0, 5.8), TP_SKIN);
        ball(&mut s, el.lerp(wr, 0.32) + v(-out * 1.2, 0.0, -0.6), v(5.2, 7.6, 5.2), TP_SKIN);
        for k in 0..16 {
            let t = k as f32 / 15.0;
            let p = el.lerp(wr, 0.12 + t * 0.8) + v(out * (0.8 + (t * 9.0).sin() * 0.9), 0.0, 4.8 - t * 0.6);
            put(&mut s, p, TP_SKIN_D);
        }
        // A strap of hide wound round the wrist.
        let up = (el - wr).normalize();
        sweep(&mut s, &[(wr + up * 0.6, 5.0), (wr + up * 5.6, 5.4)], |p, c, _| {
            let turn = (p - c).normalize_or_zero();
            if ((p - wr).dot(up) * 1.6 + turn.x.atan2(turn.z) * 0.8).sin() > 0.6 {
                TP_STRAP_D
            } else {
                TP_STRAP
            }
        });
        tp_hand(&mut s, wr, out);
    }

    // ---- the head: bald, the one great eye, a beard like a black fleece, broken teeth
    s.part(5);
    let hz = cz + 12.0;
    let (sc, sr) = (v(cx, 123.2, hz), v(11.2, 12.4, 11.6));
    ball(&mut s, sc, sr, TP_SKIN);
    rounded(&mut s, v(cx, 113.4, hz + 3.0), v(10.0, 6.0, 9.0), 2.4, 0, TP_SKIN);
    let fz = front(&s, cx as i32, 123, d).unwrap_or(70) as f32;
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 6.4, 117.0, hz + 8.6), v(4.0, 2.8, 3.2), TP_SKIN);
        // The ears, red, their hollows dark.
        ball(&mut s, v(cx + side * 11.0, 118.0, hz - 0.6), v(1.8, 4.6, 2.9), TP_SKIN);
        tint(&mut s, v(cx + side * 12.4, 118.0, hz - 0.6), 2.6, TP_SKIN, TP_RUDDY);
        for y in 117..=119 {
            for z in (hz - 1.4) as i32..=(hz + 0.2) as i32 {
                let x = (cx + side * 14.0).round() as i32;
                if let Some(xx) = (0..4).map(|k| x - side as i32 * k).find(|&xx| s.get(xx, y, z).is_some()) {
                    s.recolor(xx, y, z, TP_SKIN_DD);
                }
            }
        }
    }
    // The eye: the socket hollowed out under the brow, the eyeball deep in it, the lids heavy on it.
    let e = v(cx, 123.6, fz - 2.4);
    for y in 116..=131 {
        for z in (e.z - 3.0) as i32..=(e.z + 9.0) as i32 {
            for x in (cx - 8.0) as i32..=(cx + 8.0) as i32 {
                let q = (v(x as f32, y as f32, z as f32) - (e + v(0.0, 0.0, 2.6))) / v(7.0, 5.6, 4.4);
                if q.length_squared() <= 1.0 {
                    s.clear(x, y, z);
                }
            }
        }
    }
    for y in 117..=130 {
        for z in (e.z - 5.0) as i32..=(e.z + 5.0) as i32 {
            for x in (cx - 7.0) as i32..=(cx + 7.0) as i32 {
                let p = v(x as f32, y as f32, z as f32) - e;
                if (p / v(6.2, 4.8, 4.0)).length_squared() > 1.0 {
                    continue;
                }
                let r = (p.x * p.x + p.y * p.y).sqrt();
                let c = if p.z > 2.0 && r < 1.4 {
                    TP_PUPIL
                } else if p.z > 1.3 && r < 2.9 {
                    if r > 2.3 || (p.y > 0.4 && p.x < 0.0) {
                        TP_IRIS
                    } else {
                        TP_IRIS_L
                    }
                } else if nz(x, y, z) < 0.15 && r > 3.3 {
                    TP_VEIN
                } else if p.y < -2.4 {
                    TP_WHITE_D
                } else {
                    TP_WHITE
                };
                s.set(x, y, z, c);
            }
        }
    }
    put(&mut s, e + v(-1.2, 1.2, 3.9), TP_GLINT);
    // The heavy upper lid, its rim dark, and the line of the lower lid.
    for y in 124..=131 {
        for z in (e.z - 3.0) as i32..=(e.z + 6.0) as i32 {
            for x in (cx - 8.0) as i32..=(cx + 8.0) as i32 {
                let p = v(x as f32, y as f32, z as f32) - e;
                if (p / v(7.3, 5.8, 5.1)).length_squared() <= 1.0 && p.y > 2.3 {
                    s.set(x, y, z, if p.y < 3.4 { TP_SKIN_DD } else { TP_SKIN_D });
                }
            }
        }
    }
    for x in (cx - 6.0) as i32..=(cx + 6.0) as i32 {
        let dx = x as f32 - cx;
        let y = (e.y - 4.9 - dx * dx * 0.02).round() as i32;
        if let Some(z) = front(&s, x, y, d) {
            s.set(x, y, z + 1, TP_SKIN_D);
        }
    }
    // The brow ridge: a shelf of bone jutting over the eye, the forehead furrowed above it.
    for k in 0..=32 {
        let u = k as f32 / 16.0 - 1.0;
        let (x, y) = (cx + u * 10.6, 129.4 + 1.6 * (1.0 - u * u));
        let q = ((x - sc.x) / sr.x).powi(2) + ((y - sc.y) / sr.y).powi(2);
        let z = sc.z + sr.z * (1.0 - q).max(0.0).sqrt();
        ball(&mut s, v(x, y, z + 1.2 * (1.0 - u * u * 0.5)), v(2.1, 2.1, 2.3), TP_SKIN);
    }
    for y in [133.4f32, 135.0] {
        for x in (cx - 6.0) as i32..=(cx + 6.0) as i32 {
            let yy = (y - ((x as f32 - cx) * 0.2).powi(2)).round() as i32;
            face(&mut s, x, yy, d, 0, TP_SKIN_D);
        }
    }
    // The nose: broad and flat, broken once, red at its end.
    tube(&mut s, v(cx, 119.0, hz + 11.0), v(cx + 0.6, 115.0, hz + 15.2), 2.1, 2.7, TP_SKIN);
    ball(&mut s, v(cx + 0.6, 114.5, hz + 14.8), v(3.1, 2.3, 2.3), TP_SKIN);
    tint(&mut s, v(cx + 0.6, 114.5, hz + 16.6), 2.4, TP_SKIN, TP_RUDDY);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + 0.6 + side * 3.0, 113.8, hz + 13.2), v(2.0, 1.8, 2.0), TP_SKIN);
        put(&mut s, v(cx + 0.6 + side * 1.5, 112.6, hz + 14.4), CAVITY);
    }
    // The beard: round the jaw from ear to ear and hanging down over his chest.
    for y in 86..=120 {
        for z in (hz - 8.0) as i32..=(hz + 16.0) as i32 {
            for x in (cx - 13.0) as i32..=(cx + 13.0) as i32 {
                let p = v(x as f32, y as f32, z as f32);
                let ax = (p.x - cx).abs();
                let jaw = ((p - v(cx, 112.4, hz + 3.4)) / v(12.0, 8.2, 10.8)).length_squared() <= 1.0;
                let k = ((110.0 - p.y) / 23.0).clamp(0.0, 1.0);
                let zc = hz + 7.4 + k * 1.8;
                let (rx, rz) = (11.0 * (1.0 - k * 0.45) * (1.0 - k.powi(4) * 0.6), 6.2 * (1.0 - k * 0.35));
                let hang = p.y < 111.0 && ((p.x - cx) / rx).powi(2) + ((p.z - zc) / rz).powi(2) <= 1.0;
                if !(jaw || hang) {
                    continue;
                }
                // Not over the cheeks: the beard grows from under the cheekbones, sideburns before the ears.
                let line = if ax > 8.0 { 120.0 } else { 112.6 + (ax - 3.5).max(0.0) * 0.6 };
                if p.y > line || (p.z < hz - 3.0 && p.y > 108.0) {
                    continue;
                }
                if k > 0.8 && p.y < 86.0 + nz(x, 1, z) * 6.0 {
                    continue;
                }
                let curl = ((p.x * 0.9 + (p.y * 0.5).sin() * 1.5 + p.z * 0.3) as i32).rem_euclid(3) == 0;
                s.set(x, y, z, if curl || nz(x, y, z) < 0.16 { TP_BEARD_L } else { TP_BEARD });
            }
        }
    }
    // The mouth in the beard: a scowl, the teeth broken and gapped.
    let mz = front(&s, cx as i32, 110, d).unwrap_or(70);
    let mx = cx.round() as i32;
    for x in mx - 5..=mx + 4 {
        for y in 109..=111 {
            s.set(x, y, mz, CAVITY);
            s.clear(x, y, mz + 1);
        }
        let dx = x - mx;
        if let Some(c) = match dx {
            -3 | 1 => None,
            2 | -5 => Some(TP_TOOTH_D),
            _ => Some(TP_TOOTH),
        } {
            s.set(x, 111, mz, c);
            if dx != 0 && dx != -4 {
                s.set(x, 110, mz, c);
            }
        }
        if matches!(dx, -2 | 0 | 3) {
            s.set(x, 109, mz, TP_TOOTH_D);
        }
        s.set(x, 112, mz + 1, TP_LIP);
        s.set(x, 108, mz + 1, TP_LIP);
    }
    // The moustache over it, drooping into the beard.
    for side in [-1.0f32, 1.0] {
        let m = mz as f32;
        tube(&mut s, v(cx + 0.6 + side * 0.8, 112.2, m + 2.4), v(cx + side * 6.0, 110.8, m + 1.4), 1.6, 1.5, TP_BEARD);
        tube(&mut s, v(cx + side * 6.0, 110.8, m + 1.4), v(cx + side * 7.0, 105.0, m + 0.8), 1.5, 1.0, TP_BEARD_L);
    }
    shag(&mut s, (w, h, d), TP_BEARD, TP_BEARD, TP_BEARD_L, 0.3, 3);

    // ---- the skin: mottled, pale scars where blades have glanced off, his chest hair; then shaded
    s.paint(|x, y, z, c| (c == TP_SKIN && nz(x / 4, y / 4, z / 4) < 0.3).then_some(TP_SKIN_M));
    for (a, b) in [
        (v(cx - 15.0, 96.0, 0.0), v(cx - 3.0, 86.0, 0.0)),
        (v(cx + 4.0, 80.0, 0.0), v(cx + 14.0, 73.0, 0.0)),
        (v(cx - 10.0, 75.0, 0.0), v(cx - 2.0, 79.0, 0.0)),
    ] {
        for k in 0..=24 {
            let p = a.lerp(b, k as f32 / 24.0);
            face(&mut s, p.x.round() as i32, p.y.round() as i32, d, 0, TP_SCAR);
        }
    }
    for y in 76..=97 {
        for x in (cx - 6.0) as i32..=(cx + 6.0) as i32 {
            let dx = (x as f32 - cx).abs();
            let reach = if y > 86 { 6.0 - (97 - y) as f32 * 0.3 } else { 1.2 };
            if dx < reach && nz(x, y, 7) < 0.35 {
                if let Some(z) = front(&s, x, y, d) {
                    if s.get(x, y, z).is_some_and(|c| matches!(c, TP_SKIN | TP_SKIN_M)) {
                        s.recolor(x, y, z, TP_BEARD_L);
                    }
                }
            }
        }
    }
    // Black hair on his forearms, and old scars on his arm and thigh.
    s.paint(|x, y, z, c| {
        let ax = (x as f32 - cx).abs();
        let hairy = (56..74).contains(&y) && ax > 24.0;
        (hairy && matches!(c, TP_SKIN | TP_SKIN_M) && nz(x, y, z) < 0.05).then_some(TP_BEARD_L)
    });
    for (a, b) in [(v(cx - 30.0, 88.0, 0.0), v(cx - 25.0, 80.0, 0.0)), (v(cx + 6.0, 48.0, 0.0), v(cx + 14.0, 41.0, 0.0))] {
        for k in 0..=16 {
            let p = a.lerp(b, k as f32 / 16.0);
            face(&mut s, p.x.round() as i32, p.y.round() as i32, d, 0, TP_SCAR);
        }
    }
    hollows(
        &mut s,
        (w, h, d),
        |c| !matches!(c & 0xff00_0000, mat::GOLD | mat::SLIME | mat::GLOW) && !matches!(c, TP_TOOTH | TP_TOOTH_D | TP_NAIL | TURQUOISE),
        2.5,
    );
    s.finish(0.04)
}

// --------------------------------------------------------------- gashadokuro

/// The gashadokuro's grid: 72 wide, 124 tall, 120 deep (its arms reach far before it). Built at
/// 0.025 world units a voxel, it stoops about 3.0 tall.
pub const GASHADOKURO_SIZE: (i32, i32, i32) = (72, 124, 120);
const GK_X: f32 = 35.5;
/// The depth its body is built about: its feet, four voxels before it, stand at the grid's middle.
const GK_Z: f32 = 55.5;

/// Hips, shoulders, the neck at the top of the spine, and the jaw at its hinges (a part of its
/// own, so it must turn with the head as well as open).
pub const GASHADOKURO_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [GK_X - 8.5, 54.5, GK_Z - 2.0] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [GK_X + 8.5, 54.5, GK_Z - 2.0] },
    PartDef { name: "leftArm", boxes: &[], pivot: [GK_X - 15.5, 92.0, GK_Z + 4.5] },
    PartDef { name: "rightArm", boxes: &[], pivot: [GK_X + 15.5, 92.0, GK_Z + 4.5] },
    PartDef { name: "head", boxes: &[], pivot: [GK_X, 100.0, GK_Z + 17.0] },
    PartDef { name: "jaw", boxes: &[], pivot: [GK_X, 100.8, GK_Z + 26.4] },
];

/// The spine's line, from the tail bone up through the small of the back, hunched over, to the skull.
const GK_SPINE: [Vec3; 9] = [
    Vec3::new(GK_X, 51.0, GK_Z - 7.5),
    Vec3::new(GK_X, 58.0, GK_Z - 9.5),
    Vec3::new(GK_X, 64.0, GK_Z - 9.0),
    Vec3::new(GK_X, 74.0, GK_Z - 8.6),
    Vec3::new(GK_X, 84.0, GK_Z - 6.0),
    Vec3::new(GK_X, 92.0, GK_Z - 0.8),
    Vec3::new(GK_X, 97.0, GK_Z + 6.0),
    Vec3::new(GK_X, 99.4, GK_Z + 13.0),
    Vec3::new(GK_X, 100.0, GK_Z + 17.0),
];

/**
 * A man's skull, one of the many dead the gashadokuro is made of, fused into its bones at `c`
 * facing `f`, `r` its half-width: the brow, black sockets, the hole of the nose, a row of teeth.
 */
fn little_skull(s: &mut Sculpt, c: Vec3, f: Vec3, r: f32) {
    let (rt, up, fw) = frame(f);
    let ax = [rt, up, fw];
    block(s, c, ax, v(r, r * 1.05, r * 1.15), 2.0, GK_SKULL);
    block(s, c + fw * (r * 0.55) - up * (r * 0.75), ax, v(r * 0.62, r * 0.45, r * 0.55), 2.2, GK_SKULL);
    for side in [-1.0f32, 1.0] {
        let eye = c + rt * (side * r * 0.42) + up * (r * 0.05) + fw * (r * 1.0);
        ball(s, eye, Vec3::splat(r * 0.36), GK_SOCKET);
        put(s, eye + fw * (r * 0.2), GK_SOCKET);
    }
    put(s, c - up * (r * 0.42) + fw * (r * 1.12), GK_SOCKET);
    for k in -2..=2 {
        let p = c + rt * (k as f32 * r * 0.24) - up * (r * 0.95) + fw * (r * 1.02);
        put(s, p, if k % 2 == 0 { GK_TOOTH } else { GK_SOCKET });
    }
}

/// A dark crack wandering over the bone from `start` along `dir`, `n` steps, kept to the surface about `c`.
fn crack(s: &mut Sculpt, c: Vec3, start: Vec3, dir: Vec3, n: usize) {
    let mut p = start;
    let mut d = dir.normalize();
    for k in 0..n {
        let wob = nz(k as i32, 3, 9) - 0.5;
        d = (d + Vec3::new(wob, nz(k as i32, 5, 1) - 0.5, wob * 0.5) * 0.45).normalize();
        p += d * 0.6;
        let out = (p - c).normalize_or_zero();
        let filled = |q: Vec3| s.get(q.x.round() as i32, q.y.round() as i32, q.z.round() as i32).is_some();
        // In along the radius to the bone, then out to the last of it: the surface.
        let mut q = p;
        for _ in 0..12 {
            if filled(q) {
                break;
            }
            q -= out;
        }
        for _ in 0..12 {
            if !filled(q + out) {
                break;
            }
            q += out;
        }
        let (x, y, z) = (q.x.round() as i32, q.y.round() as i32, q.z.round() as i32);
        if s.get(x, y, z).is_some() {
            s.recolor(x, y, z, GK_CRACK);
        }
    }
}

/// A long bone from `a` to `b`, `r` thick in the shaft, swelling to knobbed, darker ends.
fn bone(s: &mut Sculpt, a: Vec3, b: Vec3, r: f32, knob: f32) {
    let len = (b - a).length().max(1e-3);
    sweep(s, &[(a, r * 1.25), (a.lerp(b, 0.2), r), (a.lerp(b, 0.8), r), (b, r * 1.25)], |p, _, t| {
        let end = (t - 0.5).abs() * len;
        if end > len * 0.5 - 1.6 {
            GK_JOINT
        } else if nz(p.x as i32 / 2, p.y as i32 / 2, p.z as i32 / 2) < 0.1 {
            GK_BONE_D
        } else {
            GK_BONE
        }
    });
    for e in [a, b] {
        ball(s, e, Vec3::splat(knob), GK_BONE);
    }
}

/// The z of a spine's line at height `y`, from its points along it (it climbs all the way).
fn axis_z(spine: &[(Vec3, Vec3)], y: f32) -> f32 {
    for w in spine.windows(2) {
        let (a, b) = (w[0].0, w[1].0);
        if (a.y..=b.y).contains(&y) {
            return a.z + (b.z - a.z) * (y - a.y) / (b.y - a.y).max(1e-4);
        }
    }
    spine.last().map(|k| k.0.z).unwrap_or(0.0)
}

/**
 * The gashadokuro: a skeleton fifteen times the height of a man, made of the bones of the
 * starved and the unburied dead, stooped and stalking on bent knees, its spine bowed so that
 * its huge skull hangs forward; black sockets with a small green-white fire deep in each, a
 * cracked brow, a jaw hanging open on broken teeth; ribs, breastbone and shoulder blades, a
 * pelvis like a bowl, long shins of two bones each; its long arms reaching out before it, the
 * bony fingers spread to seize; the skulls of men fused into its spine, breastbone, shoulders
 * and pelvis, and the rags of a burial shroud hanging from its back. Its bones are yellowed
 * ivory, darker at the joints and grimed with earth.
 */
pub fn gashadokuro() -> Collected {
    let (w, h, d) = GASHADOKURO_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, bz) = (GK_X, GK_Z);

    // ---- legs: knees bent, the thigh bones forward, two bones in each shin, the bones of the feet
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        let hip = v(cx + side * 8.5, 54.5, bz - 2.0);
        let knee = v(cx + side * 9.6, 35.0, bz + 8.6);
        let ankle = v(cx + side * 9.8, 7.0, bz - 2.0);
        ball(&mut s, hip, Vec3::splat(2.9), GK_BONE);
        let troch = hip + v(side * 3.4, -0.4, -0.4);
        tube(&mut s, hip, troch, 2.0, 2.0, GK_BONE);
        ball(&mut s, troch, v(2.2, 2.6, 2.4), GK_BONE_D);
        bone(&mut s, troch + v(-side * 0.6, -2.4, 0.4), knee + v(0.0, 1.6, 0.0), 2.2, 2.2);
        for k in [-1.0f32, 1.0] {
            ball(&mut s, knee + v(k * 1.9, 0.0, -0.4), v(2.2, 2.4, 2.8), GK_JOINT);
        }
        ball(&mut s, knee + v(0.0, 0.2, 3.4), v(1.9, 2.4, 1.3), GK_BONE_L);
        rounded(&mut s, knee + v(0.0, -2.6, -0.6), v(3.6, 1.4, 3.2), 2.2, 0, GK_JOINT);
        bone(&mut s, knee + v(0.0, -3.6, -0.6), ankle + v(0.0, 1.2, 0.0), 1.9, 1.9);
        // The fibula, thin, bowed out from the shin bone so that the light shows between them.
        let (f0, f1) = (knee + v(side * 3.2, -3.8, -1.8), ankle + v(side * 2.8, 0.8, -0.9));
        let fm = f0.lerp(f1, 0.5) + v(side * 1.2, 0.0, -0.4);
        sweep(&mut s, &[(f0, 1.3), (fm, 0.85), (f1, 1.2)], |_, _, t| if !(0.12..=0.88).contains(&t) { GK_JOINT } else { GK_BONE });
        ball(&mut s, ankle + v(-side * 1.8, 0.0, 0.2), Vec3::splat(1.5), GK_BONE_D);
        // The foot: the heel, the ankle bone, the small bones of the arch, five long toes.
        ball(&mut s, ankle, v(2.4, 2.0, 2.6), GK_JOINT);
        ball(&mut s, ankle + v(0.0, -3.4, -3.6), v(2.2, 2.4, 2.8), GK_BONE);
        rounded(&mut s, ankle + v(side * 0.4, -3.4, 2.4), v(3.2, 1.6, 2.8), 2.2, 0, GK_BONE_D);
        for k in 0..5 {
            let kf = k as f32 - 2.0;
            let r = if k == (if side < 0.0 { 4 } else { 0 }) { 1.15 } else { 0.85 };
            let base = ankle + v(side * 0.4 + kf * 1.3, -3.6, 4.4);
            let joint = base + v(kf * 0.6, -1.4, 5.0);
            let tip = joint + v(kf * 0.3, -1.2, 3.4);
            bone(&mut s, base, joint, r, r + 0.35);
            tube(&mut s, joint, tip, r * 0.9, 0.5, GK_BONE);
        }
    }

    // ---- the pelvis: the hip bones a bowl, the sacrum behind, the ring of the pubic arch below
    s.part(0);
    let pc = v(cx, 57.0, bz - 3.0);
    for y in 53..=68 {
        for z in (bz - 14.0) as i32..=(bz + 8.0) as i32 {
            for x in (cx - 14.0) as i32..=(cx + 14.0) as i32 {
                let p = v(x as f32, y as f32, z as f32) - pc;
                let q = (p / v(12.6, 10.6, 9.6)).length();
                if !(0.84..=1.0).contains(&q) || p.y < 1.0 {
                    continue;
                }
                let ax = p.x.abs();
                if (p.z > 0.5 && ax < 8.0) || (p.z < -5.0 && ax < 4.0) {
                    continue;
                }
                let crest = 9.6 - ((p.z + 2.5) / 6.5).powi(2) * 4.2;
                if p.y > crest {
                    continue;
                }
                s.set(
                    x,
                    y,
                    z,
                    if p.y > crest - 1.2 {
                        GK_BONE_L
                    } else if nz(x / 2, y / 2, z / 2) < 0.12 {
                        GK_BONE_D
                    } else {
                        GK_BONE
                    },
                );
            }
        }
    }
    plate(&mut s, v(cx - 4.4, 64.0, bz - 10.0), v(cx + 4.4, 64.0, bz - 10.0), v(cx, 50.5, bz - 8.4), 2.2, GK_BONE);
    for k in 0..4 {
        for side in [-1.0f32, 1.0] {
            put(&mut s, v(cx + side * 1.6, 61.5 - k as f32 * 2.6, bz - 11.0 + k as f32 * 0.3), GK_SOCKET);
        }
    }
    for side in [-1.0f32, 1.0] {
        let socket = v(cx + side * 8.5, 54.5, bz - 2.0);
        ball(&mut s, socket + v(side * 0.6, 0.6, 0.0), v(3.0, 3.0, 3.2), GK_JOINT);
        tube(&mut s, socket + v(-side * 1.0, -1.8, 1.8), v(cx + side * 1.4, 50.0, bz + 4.4), 1.5, 1.3, GK_BONE);
        tube(&mut s, socket + v(-side * 0.6, -2.4, -1.6), v(cx + side * 5.4, 48.6, bz - 4.0), 1.7, 1.9, GK_BONE);
        tube(&mut s, v(cx + side * 5.4, 48.6, bz - 4.0), v(cx + side * 1.4, 49.4, bz + 4.0), 1.2, 1.2, GK_BONE_D);
    }

    // ---- the spine: a column of vertebrae from the sacrum, bowed over, up to the skull
    let spine = along(&GK_SPINE, 24);
    sweep(&mut s, &spine.iter().skip(3).map(|(p, _)| (*p, 1.9)).collect::<Vec<_>>(), |_, _, _| GK_JOINT);
    let mut thoracic = vec![];
    for (p, t) in spine.iter().skip(3) {
        let back = Vec3::new(0.0, t.z, -t.y).normalize_or(Vec3::NEG_Z);
        let y = p.y;
        let r = if y < 73.0 {
            3.1
        } else if y < 96.0 {
            2.6
        } else {
            2.0
        };
        block(&mut s, *p, [Vec3::X, *t, back], v(r, 0.85, r * 0.85), 2.4, GK_BONE);
        tube(&mut s, *p + back * (r * 0.8), *p + back * (r + 3.4) - *t * 1.2, 0.95, 0.5, GK_BONE_D);
        for side in [-1.0f32, 1.0] {
            tube(&mut s, *p + back * (r * 0.4), *p + back * (r * 0.6) + Vec3::X * (side * (r + 2.0)), 0.8, 0.5, GK_BONE);
        }
        if (74.0..96.0).contains(&y) {
            thoracic.push((*p, *t, back));
        }
    }

    // ---- the ribs: nine pairs bowed round from the spine, the breastbone before them, the collarbones
    thoracic.reverse();
    let ribs = thoracic.len().min(9);
    let mut fronts = vec![];
    for (i, &(vp, t, back)) in thoracic.iter().take(ribs).enumerate() {
        let fi = i as f32 / 8.0;
        let wid = 8.5 + 7.0 * (fi * PI * 0.75).sin();
        let dep = 6.5 + 4.8 * (fi * PI * 0.7).sin();
        let fwd = (-back - t * (0.35 + fi * 0.5)).normalize();
        let end = [0.9f32, 0.9, 0.9, 0.9, 0.9, 0.9, 0.8, 0.8, 0.6][i];
        for side in [-1.0f32, 1.0] {
            let at = |phi: f32| {
                vp + Vec3::X * (side * wid * phi.sin()) + fwd * (dep * (1.0 - phi.cos())) + back * (2.4 * phi.sin() * phi.cos().max(0.0))
            };
            let pts: Vec<(Vec3, f32)> = (0..=24).map(|k| (at(end * PI * k as f32 / 24.0), 1.0)).collect();
            sweep(&mut s, &pts, |p, c, _| if p.y < c.y - 0.4 { GK_BONE_D } else { GK_BONE });
            if i < 8 {
                let to = if i < 6 { at(PI) } else { at(0.93 * PI) + t * 4.0 };
                tube(&mut s, at(end * PI), to, 0.8, 0.7, GK_CARTILAGE);
            }
            if side > 0.0 && i < 6 {
                fronts.push(at(PI));
            }
        }
    }
    if fronts.len() >= 2 {
        let top = fronts[0];
        let bottom = fronts[fronts.len() - 1];
        let along_st = (bottom - top).normalize();
        let out = Vec3::X.cross(along_st).normalize() * if Vec3::X.cross(along_st).z < 0.0 { -1.0 } else { 1.0 };
        for k in 0..=12 {
            let t = k as f32 / 12.0;
            let p = top.lerp(bottom, t) + out * 0.4;
            let wide = if t < 0.25 { 3.2 } else { 2.2 - t * 0.6 };
            block(&mut s, p, [Vec3::X, along_st, out], v(wide, 1.3, 1.0), 2.2, GK_BONE_L);
        }
        tube(&mut s, bottom, bottom + along_st * 3.4 + out * 0.6, 1.0, 0.4, GK_BONE);
        for side in [-1.0f32, 1.0] {
            let sh = v(cx + side * 15.0, 94.0, bz + 4.0);
            let mid = top.lerp(sh, 0.5) + v(0.0, 1.2, 1.0);
            tube(&mut s, top + v(side * 2.0, 0.6, 0.0), mid, 1.3, 1.2, GK_BONE);
            tube(&mut s, mid, sh, 1.2, 1.4, GK_BONE);
        }
        little_skull(&mut s, top.lerp(bottom, 0.5) + out * 2.4, out + Vec3::Y * 0.2, 3.0);
    }
    // The shoulder blades on its back, their spines ridged.
    for side in [-1.0f32, 1.0] {
        let a = v(cx + side * 14.0, 91.5, bz + 1.0);
        let b = v(cx + side * 5.5, 93.0, bz - 5.6);
        let c = v(cx + side * 8.0, 79.5, bz - 9.4);
        plate(&mut s, a, b, c, 1.4, GK_BONE);
        tube(&mut s, b.lerp(a, 0.08) + v(0.0, -2.0, -0.6), v(cx + side * 15.6, 93.6, bz + 2.6), 1.2, 1.0, GK_BONE_L);
        // A dead man's skull fused over each shoulder.
        little_skull(&mut s, v(cx + side * 13.0, 97.4, bz + 1.0), v(side * 0.5, 0.6, 0.6), 3.0);
    }
    // More of them grown into its spine, staring backward, and one in the sacrum.
    for (k, r) in [(11usize, 2.8f32), (16, 3.0)] {
        let (p, t) = spine[k];
        let back = Vec3::new(0.0, t.z, -t.y).normalize_or(Vec3::NEG_Z);
        little_skull(&mut s, p + back * 4.6, back, r);
    }
    little_skull(&mut s, v(cx, 59.0, bz - 12.2), v(0.0, 0.1, -1.0), 2.8);

    // ---- arms: long, reaching out before it, the hands open and the fingers spread to seize
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(3 + i as u8);
        let sh = v(cx + side * 15.5, 92.0, bz + 4.5);
        let el = v(cx + side * 19.5, 78.0, bz + 24.0);
        let wr = v(cx + side * 18.0, 75.0, bz + 43.0);
        ball(&mut s, sh, Vec3::splat(3.1), GK_BONE);
        bone(&mut s, sh + v(side * 0.4, -2.0, 1.4), el + v(0.0, 0.6, -1.4), 1.9, 2.0);
        rounded(&mut s, el, v(3.2, 2.2, 2.4), 2.2, 0, GK_JOINT);
        ball(&mut s, el + v(0.0, -0.4, -2.6), v(1.8, 1.8, 1.8), GK_BONE_D);
        for k in [-1.0f32, 1.0] {
            bone(&mut s, el + v(k * 1.5, -0.4, 1.6), wr + v(k * 1.3, 0.0, -1.0), 1.15, 1.45);
        }
        for k in 0..6 {
            let kf = k as f32;
            put(&mut s, wr + v((kf % 3.0 - 1.0) * 1.4, (kf / 3.0).floor() * 1.2 - 0.6, 0.6), GK_JOINT);
            ball(&mut s, wr + v((kf % 3.0 - 1.0) * 1.3, (kf / 3.0).floor() * 1.1 - 0.5, 0.8), Vec3::splat(1.0), GK_BONE_D);
        }
        // Four fingers fanned out and hooked, the thumb inward.
        for f in 0..4 {
            let ff = f as f32 - 1.5;
            let spread = ff * 0.32;
            let dir = v(side * 0.15 + spread.sin(), -0.1, spread.cos()).normalize();
            let k0 = wr + v(ff * 1.4, 0.0, 2.0);
            let k1 = k0 + dir * 7.0;
            let k2 = k1 + (dir + v(0.0, -0.45, 0.0)).normalize() * 4.4;
            let k3 = k2 + (dir + v(0.0, -1.1, 0.0)).normalize() * 3.6;
            let k4 = k3 + (dir * 0.4 + v(0.0, -1.0, 0.0)).normalize() * 2.8;
            bone(&mut s, k0, k1, 0.9, 1.2);
            bone(&mut s, k1, k2, 0.85, 1.1);
            bone(&mut s, k2, k3, 0.75, 1.0);
            tube(&mut s, k3, k4, 0.7, 0.35, GK_BONE_L);
        }
        let t0 = wr + v(-side * 2.0, -0.6, 1.4);
        let t1 = t0 + v(-side * 3.4, -1.0, 4.0);
        let t2 = t1 + v(-side * 1.8, -1.6, 3.4);
        let t3 = t2 + v(-side * 0.4, -1.8, 2.0);
        bone(&mut s, t0, t1, 1.0, 1.2);
        bone(&mut s, t1, t2, 0.9, 1.1);
        tube(&mut s, t2, t3, 0.8, 0.35, GK_BONE_L);
    }

    // ---- the skull: huge, hung forward, black sockets with a fire in each, a cracked brow
    s.part(5);
    let kc = v(cx, 110.5, bz + 25.0);
    let kr = v(11.4, 11.0, 13.6);
    ball(&mut s, kc, kr, GK_BONE);
    // Flat at the temples, where the jaw's muscles were.
    for y in 96..112 {
        for z in (bz + 10.0) as i32..=(bz + 40.0) as i32 {
            for x in (cx - 13.0) as i32..=(cx + 13.0) as i32 {
                if (x as f32 - cx).abs() > 10.2 + (y - 96) as f32 * 0.04 {
                    s.clear(x, y, z);
                }
            }
        }
    }
    // The face: cheekbones and their arches back to the ears, the upper jaw, the knobs behind the ears.
    rounded(&mut s, v(cx, 102.6, bz + 33.4), v(8.4, 6.2, 5.2), 2.2, 0, GK_BONE);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 7.8, 103.0, bz + 35.6), v(2.6, 2.2, 2.6), GK_BONE_L);
        tube(&mut s, v(cx + side * 9.2, 103.2, bz + 34.0), v(cx + side * 10.8, 104.0, bz + 25.5), 1.3, 1.2, GK_BONE);
        ball(&mut s, v(cx + side * 9.6, 100.6, bz + 21.6), v(1.6, 2.0, 1.8), GK_BONE_D);
    }
    // The brow ridge, arched over each socket.
    for k in 0..=24 {
        let u = k as f32 / 12.0 - 1.0;
        let x = cx + u * 9.6;
        let y = 112.2 + (1.0 - (u.abs() * 2.0 - 1.0).powi(2)) * 0.7;
        let q = ((x - kc.x) / kr.x).powi(2) + ((y - kc.y) / kr.y).powi(2);
        let z = kc.z + kr.z * (1.0 - q).max(0.0).sqrt();
        ball(&mut s, v(x, y, z - 0.2), v(1.6, 1.4, 1.6), GK_BONE_L);
    }
    // The sockets, hollowed deep and black, a small fire burning at the back of each.
    for side in [-1.0f32, 1.0] {
        let ex = cx + side * 5.0;
        let e = v(ex, 107.4, front(&s, ex.round() as i32, 107, d).unwrap_or(90) as f32 + 0.5);
        for y in (e.y - 5.0) as i32..=(e.y + 5.0) as i32 {
            for z in (e.z - 9.0) as i32..=(e.z + 3.0) as i32 {
                for x in (e.x - 5.0) as i32..=(e.x + 5.0) as i32 {
                    let p = v(x as f32, y as f32, z as f32) - e;
                    let r = 3.6 - (-p.z).max(0.0) * 0.14;
                    if p.z > -8.0 && (p.x / r).powi(2) + (p.y / (r * 0.95)).powi(2) < 1.0 {
                        s.clear(x, y, z);
                    }
                }
            }
        }
        for y in (e.y - 5.0) as i32..=(e.y + 5.0) as i32 {
            for z in (e.z - 10.0) as i32..=(e.z - 1.4) as i32 {
                for x in (e.x - 5.0) as i32..=(e.x + 5.0) as i32 {
                    let p = v(x as f32, y as f32, z as f32) - e;
                    if p.x * p.x + p.y * p.y < 22.0 && open(&s, x, y, z) {
                        s.recolor(x, y, z, GK_SOCKET);
                    }
                }
            }
        }
        // (A point of white fire, a green flicker over it and a glow behind.)
        let fire = e + v(-side * 0.3, -0.6, -4.8);
        put(&mut s, fire, GK_EYE);
        put(&mut s, fire + v(0.0, 1.0, -0.6), GK_EYE_G);
        put(&mut s, fire + v(0.0, 0.0, -1.0), GK_EYE_G);
    }
    // The hole of the nose: an upturned heart, the little spine of bone at its foot.
    let nf = front(&s, cx as i32, 102, d).unwrap_or(90);
    for (y, half) in [(105, 0.5f32), (104, 1.0), (103, 1.6), (102, 1.9), (101, 1.7), (100, 1.2)] {
        for x in (cx - half).round() as i32..=(cx + half).round() as i32 {
            if y == 100 && x == cx.round() as i32 {
                continue;
            }
            for z in nf - 4..=nf + 2 {
                s.clear(x, y, z);
            }
            s.set(x, y, nf - 5, GK_SOCKET);
        }
    }
    // The holes of the ears.
    for side in [-1.0f32, 1.0] {
        for (dy, dz) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let (y, z) = (104 + dy, (bz + 23.0) as i32 + dz);
            if let Some(x) = (0..8).map(|k| (cx + side * (13.0 - k as f32)).round() as i32).find(|&x| s.get(x, y, z).is_some()) {
                s.recolor(x, y, z, GK_SOCKET);
            }
        }
    }
    // The upper teeth along the curve of the jaw: long, cracked, two of them gone.
    for k in 0..12 {
        let u = (k as f32 - 5.5) / 5.5;
        let x = cx + u * 6.2;
        let z = bz + 38.0 - u * u * 4.0;
        let c = match k {
            3 | 9 => continue,
            1 | 6 => GK_TOOTH_D,
            _ => GK_TOOTH,
        };
        let long = if k == 4 {
            1.0
        } else if k == 2 || k == 10 {
            2.8
        } else {
            2.2
        };
        tube(&mut s, v(x, 96.8, z), v(x, 96.8 - long, z + 0.2), 0.75, 0.6, c);
    }
    // Cracks over the brow and the crown.
    crack(&mut s, kc, v(cx + 3.0, 120.0, bz + 32.0), v(0.3, -0.6, 0.6), 30);
    crack(&mut s, kc, v(cx + 5.0, 116.0, bz + 34.5), v(1.0, -0.3, 0.0), 9);
    crack(&mut s, kc, v(cx - 6.0, 120.5, bz + 24.0), v(-0.5, -0.3, -0.4), 24);

    // ---- the jaw: hanging open from its hinges, its teeth broken
    s.part(6);
    for side in [-1.0f32, 1.0] {
        let hinge = v(cx + side * 9.6, 100.8, bz + 26.4);
        let angle = v(cx + side * 8.8, 92.2, bz + 25.4);
        tube(&mut s, hinge, angle, 1.5, 1.9, GK_BONE);
        ball(&mut s, hinge, v(1.6, 1.5, 1.8), GK_JOINT);
        tube(&mut s, angle + v(0.0, 1.6, 2.6), v(cx + side * 8.2, 99.6, bz + 30.6), 1.0, 0.6, GK_BONE_D);
    }
    let arch_at = |u: f32| v(cx + u * 8.8, 92.2 - (1.0 - u * u) * 2.6, bz + 25.4 + (1.0 - u * u) * 11.0);
    let arch: Vec<(Vec3, f32)> = (0..=16).map(|k| (arch_at(k as f32 / 8.0 - 1.0), 1.9)).collect();
    sweep(&mut s, &arch, |p, c, _| if p.y < c.y - 0.6 { GK_BONE_D } else { GK_BONE });
    ball(&mut s, arch_at(0.0) + v(0.0, -0.4, 0.4), v(2.6, 2.0, 1.6), GK_BONE);
    for k in 0..11 {
        let u = (k as f32 - 5.0) / 8.0;
        let p = arch_at(u) + v(0.0, 1.6, -0.2);
        let c = match k {
            2 | 7 => continue,
            5 => GK_TOOTH_D,
            _ => GK_TOOTH,
        };
        tube(&mut s, p, p + v(0.0, if k == 8 { 0.8 } else { 1.8 }, 0.0), 0.7, 0.55, c);
    }

    // ---- the shroud: rags of a burial cloth hung over its back from the shoulders
    s.part(0);
    for y in 58..=100 {
        for z in (bz - 18.0) as i32..=(bz + 4.0) as i32 {
            for x in (cx - 18.0) as i32..=(cx + 18.0) as i32 {
                let p = v(x as f32, y as f32, z as f32);
                let sz = axis_z(&spine, p.y) - 3.6 - (p.x - cx).powi(2) * 0.012;
                if (p.z - sz).abs() > 0.7 || (p.x - cx).abs() > 15.0 - (100.0 - p.y) * 0.05 {
                    continue;
                }
                let strip = ((p.x - cx) * 0.55 + nz(x / 3, 1, 2) * 2.0).sin();
                let hem = 80.0 - strip * 14.0 - nz(x, 3, 5) * 5.0;
                if p.y < hem || nz(x / 2, y / 3, z) < 0.12 || p.y > 98.0 - (p.x - cx).abs() * 0.15 {
                    continue;
                }
                s.set(
                    x,
                    y,
                    z,
                    if strip > 0.6 {
                        GK_SHROUD_D
                    } else if nz(x, y, z) > 0.85 {
                        GK_SHROUD_L
                    } else {
                        GK_SHROUD
                    },
                );
            }
        }
    }

    // ---- grime: earth on its feet and shins, darker in every hollow
    s.paint(|x, y, z, c| {
        (matches!(c, GK_BONE | GK_BONE_L | GK_BONE_D) && nz(x / 2, y / 2, z / 2) < 0.55 - y as f32 / 30.0).then_some(GK_GRIME)
    });
    hollows(
        &mut s,
        (w, h, d),
        |c| !matches!(c & 0xff00_0000, mat::GLOW) && !matches!(c, GK_SOCKET | GK_CRACK | GK_TOOTH | GK_TOOTH_D),
        1.6,
    );
    s.finish(0.04)
}

// ---------------------------------------------------------------------- ejder

/// The ejder's grid: 132 wide (its necks fan out), 132 tall (its highest head), 102 deep. Built
/// at 0.018 world units a voxel, its middle head rises about 2.3.
pub const EJDER_SIZE: (i32, i32, i32) = (132, 132, 102);
const EJ_X: f32 = 65.5;
const EJ_Z: f32 = 50.5;
/// Its necks fan out from its shoulders at these angles from upright (degrees), left to right.
const EJ_FAN: [f32; 7] = [-60.0, -40.0, -20.0, 0.0, 20.0, 40.0, 60.0];
/// Where each neck leaves its shoulders (its joint), left to right.
const EJ_ROOTS: [[f32; 3]; 7] = [
    [EJ_X - 26.0, 41.5, EJ_Z + 13.0],
    [EJ_X - 19.0, 49.5, EJ_Z + 14.0],
    [EJ_X - 10.0, 54.0, EJ_Z + 13.0],
    [EJ_X, 55.5, EJ_Z + 12.0],
    [EJ_X + 10.0, 54.0, EJ_Z + 13.0],
    [EJ_X + 19.0, 49.5, EJ_Z + 14.0],
    [EJ_X + 26.0, 41.5, EJ_Z + 13.0],
];
/// Where each head sits on its neck (the back of its skull), left to right.
const EJ_HEADS: [[f32; 3]; 7] = [
    [EJ_X - 52.0, 79.0, EJ_Z + 20.0],
    [EJ_X - 41.0, 96.0, EJ_Z + 22.0],
    [EJ_X - 23.0, 107.5, EJ_Z + 22.0],
    [EJ_X, 113.5, EJ_Z + 19.0],
    [EJ_X + 23.0, 107.5, EJ_Z + 22.0],
    [EJ_X + 41.0, 96.0, EJ_Z + 22.0],
    [EJ_X + 52.0, 79.0, EJ_Z + 20.0],
];
/// The tail's root, under the rump.
const EJ_TAIL: [f32; 3] = [EJ_X, 29.0, EJ_Z - 29.0];
/// Its legs' shoulders and hips: front left, front right, back left, back right.
const EJ_LEGS: [[f32; 3]; 4] = [
    [EJ_X - 23.0, 29.0, EJ_Z + 12.0],
    [EJ_X + 23.0, 29.0, EJ_Z + 12.0],
    [EJ_X - 18.5, 28.0, EJ_Z - 22.0],
    [EJ_X + 18.5, 28.0, EJ_Z - 22.0],
];

/// Seven necks at their roots on the shoulders (each with its head), the tail at its root, and the four legs.
pub const EJDER_RIG: &[PartDef] = &[
    PartDef { name: "neck0", boxes: &[], pivot: EJ_ROOTS[0] },
    PartDef { name: "neck1", boxes: &[], pivot: EJ_ROOTS[1] },
    PartDef { name: "neck2", boxes: &[], pivot: EJ_ROOTS[2] },
    PartDef { name: "neck3", boxes: &[], pivot: EJ_ROOTS[3] },
    PartDef { name: "neck4", boxes: &[], pivot: EJ_ROOTS[4] },
    PartDef { name: "neck5", boxes: &[], pivot: EJ_ROOTS[5] },
    PartDef { name: "neck6", boxes: &[], pivot: EJ_ROOTS[6] },
    PartDef { name: "tail", boxes: &[], pivot: EJ_TAIL },
    PartDef { name: "frontLeftLeg", boxes: &[], pivot: EJ_LEGS[0] },
    PartDef { name: "frontRightLeg", boxes: &[], pivot: EJ_LEGS[1] },
    PartDef { name: "backLeftLeg", boxes: &[], pivot: EJ_LEGS[2] },
    PartDef { name: "backRightLeg", boxes: &[], pivot: EJ_LEGS[3] },
];

/// The ejder's scales, in rows `row` along it and columns `col` across, each row staggered half
/// a scale: every scale's back edge and side dark, here and there one catching the light.
fn ej_scale(row: f32, col: f32, seed: i32) -> u32 {
    let r = row.floor();
    let cc = col + r * 0.5;
    let (fu, fv) = (row.fract(), cc.fract());
    let n = nz(r as i32, cc.floor() as i32, seed);
    if fu < 0.26 || fv < 0.2 {
        EJ_SCALE_D
    } else if n > 0.85 {
        EJ_BRONZE
    } else if n > 0.6 {
        EJ_SCALE_L
    } else {
        EJ_SCALE
    }
}

/**
 * One of the ejder's heads, the back of its skull at `b`, facing `fwd`, `k` its size, its jaw
 * dropped `gape` radians: a long horned head, small yellow eyes under scaled brows, the jaws open
 * on rows of teeth and a forked tongue, and behind each cheek a frill of red webbing on spines.
 */
fn ej_head(s: &mut Sculpt, b: Vec3, fwd: Vec3, k: f32, gape: f32) {
    let (r, u, f) = frame(fwd);
    let ax = [r, u, f];
    let at = |x: f32, y: f32, z: f32| b + (r * x + u * y + f * z) * k;
    block(s, at(0.0, 0.4, 2.6), ax, v(4.2, 3.6, 4.2) * k, 2.2, EJ_RAW);
    for j in 0..=10 {
        let t = j as f32 / 10.0;
        block(s, at(0.0, 0.9 - t * 0.5, 5.0 + t * 8.4), ax, v(3.5 - t * 1.4, 2.0 - t * 0.6, 1.2) * k, 2.2, EJ_RAW);
    }
    for j in 0..=8 {
        let t = j as f32 / 8.0;
        block(s, at(0.0, -1.1 + t * 0.3, 5.0 + t * 7.6), ax, v(2.6 - t, 0.5, 1.0) * k, 2.0, EJ_MOUTH);
    }
    for side in [-1.0f32, 1.0] {
        ball(s, at(side * 2.6, 2.6, 5.6), v(1.7, 1.1, 2.2) * k, EJ_SCALE_D);
        put(s, at(side * 3.3, 1.5, 5.4), EJ_EYE);
        put(s, at(side * 3.3, 1.5, 6.4), EJ_EYE);
        put(s, at(side * 1.1, 1.6, 13.0), EJ_NOSTRIL);
        let horn = [
            (at(side * 2.2, 3.0, 2.0), 1.4 * k),
            (at(side * 3.2, 5.4, -0.8), 1.15 * k),
            (at(side * 3.8, 6.8, -4.0), 0.8 * k),
            (at(side * 3.4, 7.2, -6.8), 0.45 * k),
            (at(side * 2.6, 6.6, -8.4), 0.25 * k),
        ];
        sweep(s, &horn, |_, _, t| {
            if t > 0.78 {
                EJ_HORN_TIP
            } else if (t * 9.0).fract() < 0.25 {
                EJ_HORN_D
            } else {
                EJ_HORN
            }
        });
        // The frill: spines fanned out behind the cheek, red webbing stretched between them.
        let root = at(side * 3.6, -0.6, 1.4);
        let tips: Vec<Vec3> = (0..5)
            .map(|j| {
                let th = (25.0 + j as f32 * 32.0).to_radians();
                let dir = (r * (side * 0.75) + u * th.cos() - f * 0.7).normalize();
                root + dir * ((7.0 - (j as f32 - 1.5).abs() * 0.8) * k)
            })
            .collect();
        for w in tips.windows(2) {
            for q in 0..=10 {
                let f = q as f32 / 10.0;
                let e = root + (w[0].lerp(w[1], f) - root) * (1.0 - 0.2 * (f * PI).sin());
                let steps = ((e - root).length() * 2.5) as i32;
                for st in 0..=steps {
                    let t = st as f32 / steps.max(1) as f32;
                    put(s, root.lerp(e, t), if t > 0.82 { EJ_FRILL_D } else { EJ_FRILL });
                }
            }
        }
        for t in &tips {
            tube(s, root, *t, 0.55 * k, 0.25 * k, EJ_HORN_D);
        }
        // The upper teeth along the jaw's edge, the fangs at the front longest.
        for j in 0..6 {
            let z = 6.0 + j as f32 * 1.4;
            let half = 3.5 - (z - 5.0) / 8.4 * 1.4;
            let a = at(side * (half - 0.7), -1.1, z);
            tube(s, a, a - u * (if j == 5 { 2.2 } else { 1.2 } * k), 0.55 * k, 0.3 * k, EJ_TOOTH);
        }
    }
    // A short horn on the snout, a crest of spikes over the crown.
    tube(s, at(0.0, 1.8, 11.0), at(0.0, 3.6, 12.4), 0.9 * k, 0.2 * k, EJ_HORN);
    for (z, tall) in [(4.4f32, 1.6f32), (2.4, 2.2), (0.4, 2.0)] {
        tube(s, at(0.0, 3.6, z), at(0.0, 3.6 + tall, z - 1.4), 0.7 * k, 0.2 * k, EJ_HORN_D);
    }
    // The lower jaw, dropped open, pale beneath, its teeth standing up, the tongue lolling in it.
    let hinge = at(0.0, -1.7, 2.0);
    let jd = (f * gape.cos() - u * gape.sin()).normalize();
    let ju = jd.cross(r);
    for j in 0..=10 {
        let t = j as f32 / 10.0;
        let c = hinge + jd * (t * 11.0 * k);
        block(s, c, [r, ju, jd], v(3.0 - t * 1.2, 1.0, 0.9) * k, 2.2, EJ_RAW);
        block(s, c - ju * (0.7 * k), [r, ju, jd], v(2.6 - t * 1.1, 0.5, 0.9) * k, 2.0, EJ_BELLY);
    }
    for side in [-1.0f32, 1.0] {
        for j in 0..5 {
            let a = hinge + jd * ((4.4 + j as f32 * 1.5) * k) + r * (side * (2.2 - j as f32 * 0.2) * k) + ju * (0.8 * k);
            tube(s, a, a + ju * (if j == 4 { 1.8 } else { 1.0 } * k), 0.5 * k, 0.3 * k, EJ_TOOTH);
        }
    }
    let t0 = hinge + ju * (1.0 * k) + jd * (2.0 * k);
    let t1 = hinge + ju * (1.3 * k) + jd * (11.5 * k);
    tube(s, t0, t1, 0.9 * k, 0.6 * k, EJ_TONGUE);
    for side in [-1.0f32, 1.0] {
        tube(s, t1, t1 + jd * (2.0 * k) + r * (side * 1.0 * k) + ju * (0.6 * k), 0.5 * k, 0.3 * k, EJ_TONGUE);
    }
}

/// Neck `i` of the ejder (0 its far left, 3 the middle and tallest, 6 its far right) and its head.
fn ej_neck(s: &mut Sculpt, i: usize) {
    let a = EJ_FAN[i].to_radians();
    let root = Vec3::from(EJ_ROOTS[i]);
    let head = Vec3::from(EJ_HEADS[i]);
    let fan = v(a.sin(), a.cos(), 0.0);
    // Each looks its own way, a little outward, some lower, some higher.
    let (pitch, yaw) = ([0.08f32, -0.1, 0.12, -0.04, 0.16, -0.08, 0.02][i], [0.1f32, -0.06, 0.08, 0.0, -0.1, 0.07, -0.08][i]);
    let fwd = v(a.sin() * 0.35 + yaw, -0.16 - pitch, 1.0).normalize();
    let len = (head - root).length();
    let n = 36;
    let sway = v(a.cos(), -a.sin(), 0.0) * if i.is_multiple_of(2) { 2.6 } else { -2.6 };
    let path = along(&[root, root + fan * (len * 0.4) - Vec3::Z * 3.0 + sway, head - fwd * 8.0 - Vec3::Y * 3.0 - sway * 0.6, head], n);
    let knots: Vec<(Vec3, f32)> = path
        .iter()
        .enumerate()
        .map(|(k, (p, _))| {
            let t = k as f32 / n as f32;
            (*p, 4.3 - 1.3 * t)
        })
        .collect();
    let ventral = |t: f32| {
        let tan = path[((t * n as f32) as usize).min(n)].1;
        let want = v(0.0, -0.6, 1.0);
        (tan, (want - tan * want.dot(tan)).normalize_or(Vec3::Z))
    };
    sweep(s, &knots, |p, c, t| {
        let (tan, ven) = ventral(t);
        let nrm = (p - c).normalize_or_zero();
        let vn = nrm.dot(ven);
        let run = t * len;
        if vn > 0.8 {
            if (run * 0.8) as i32 % 2 == 0 {
                EJ_BELLY_D
            } else {
                EJ_BELLY
            }
        } else if vn > 0.64 {
            EJ_SCALE_D
        } else {
            let ang = nrm.dot(tan.cross(ven)).atan2(vn);
            ej_scale(run / 2.4, ang * 4.0 / 2.6, i as i32)
        }
    });
    // Spines down the back of the neck, leaning toward the shoulders.
    for k in (n * 3 / 10..n * 9 / 10).step_by(2) {
        let t = k as f32 / n as f32;
        let (c, rad) = knots[k];
        let (tan, ven) = ventral(t);
        let base = c - ven * (rad - 0.6);
        let tip = base - ven * (2.6 - t * 0.8) - tan * 1.4;
        tube(s, base, tip, 0.9, 0.2, if k % 4 == 0 { EJ_SPINE } else { EJ_HORN_D });
    }
    ej_head(s, head, fwd, if i == 3 { 1.5 } else { 1.35 }, [0.5f32, 0.36, 0.62, 0.56, 0.42, 0.64, 0.46][i]);
}

/**
 * The yedi başlı ejder, the seven-headed dragon of the Turkish tales that sat on a town's only
 * spring: a great body of green-bronze scales on four short, heavy legs, crouched, its belly
 * pale and ridged, its back spined; seven long necks rising from its shoulders in a fan, the
 * middle one highest, each with a horned head on it, its jaws open on rows of teeth, small
 * yellow eyes burning, a red frill fanned behind its cheeks; a thick tail curled round its left
 * side along the ground, spined to its tip.
 */
pub fn ejder() -> Collected {
    let (w, h, d) = EJDER_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (EJ_X, EJ_Z);
    // Its scales, darker along its back and paler down its flanks.
    let body_paint = |x: i32, y: i32, z: i32| match ej_scale((y as f32 + (x as f32 - cx).abs() * 0.7) / 3.2, z as f32 / 3.4, 9) {
        EJ_SCALE if y as f32 > 44.0 + nz(x / 3, 2, z / 3) * 8.0 => EJ_BACK,
        EJ_SCALE if (y as f32) < 22.0 + nz(x / 3, 4, z / 3) * 6.0 => EJ_SCALE_L,
        c => c,
    };

    // ---- the body: a deep chest heaped up under the necks, a barrel of a belly, the haunches
    s.part(0);
    ball(&mut s, v(cx, 37.0, cz + 8.0), v(28.0, 21.0, 18.0), EJ_RAW);
    ball(&mut s, v(cx, 32.0, cz - 6.0), v(25.0, 19.0, 22.0), EJ_RAW);
    ball(&mut s, v(cx, 32.0, cz - 21.0), v(20.5, 17.0, 14.0), EJ_RAW);
    s.paint(|x, y, z, c| {
        if c != EJ_RAW {
            return None;
        }
        let dx = (x as f32 - cx).abs();
        // The pale ridged belly: across its breast in bands, then under it from the forelegs back.
        if z as f32 > cz + 16.0 && dx < 5.5 + (y as f32 - 16.0).max(0.0) * 0.14 - ((y as f32 - 40.0).max(0.0) * 0.5).powi(2) && y < 47 {
            return Some(if (y / 2) % 2 == 0 { EJ_BELLY_D } else { EJ_BELLY });
        }
        if y < 20 && dx < 12.0 {
            return Some(if (z / 2) % 2 == 0 { EJ_BELLY_D } else { EJ_BELLY });
        }
        Some(body_paint(x, y, z))
    });
    // Spines down the ridge of its back to the tail.
    for k in 0..9 {
        let z = cz + 4.0 - k as f32 * 4.2;
        let top = (0..h).rev().find(|&y| s.get(cx.round() as i32, y, z.round() as i32).is_some()).unwrap_or(48) as f32;
        let tall = 5.2 - k as f32 * 0.25;
        for x in [cx.floor(), cx.ceil()] {
            plate(
                &mut s,
                v(x, top - 1.0, z + 2.0),
                v(x, top - 1.0, z - 2.0),
                v(x, top + tall, z - 2.6),
                1.0,
                if k % 2 == 0 { EJ_SPINE } else { EJ_HORN_D },
            );
        }
    }

    // ---- legs: short and heavy, the elbows out, broad clawed feet
    for (i, j) in EJ_LEGS.iter().enumerate() {
        s.part(9 + i as u8);
        let side = if i.is_multiple_of(2) { -1.0 } else { 1.0 };
        let joint = Vec3::from(*j);
        let front = i < 2;
        let (knee, ankle, foot) = if front {
            (joint + v(side * 7.5, -12.0, 1.0), joint + v(side * 6.5, -22.0, 3.5), joint + v(side * 7.0, -26.0, 6.0))
        } else {
            (joint + v(side * 7.5, -12.0, 6.0), joint + v(side * 7.5, -21.0, -1.0), joint + v(side * 7.5, -25.0, 3.0))
        };
        ball(&mut s, joint, v(9.0, 10.0, 10.0), EJ_RAW);
        tube(&mut s, joint, knee, 8.0, 6.0, EJ_RAW);
        ball(&mut s, knee, Vec3::splat(5.6), EJ_RAW);
        tube(&mut s, knee, ankle, 5.6, 4.2, EJ_RAW);
        rounded(&mut s, foot, v(6.0, 3.0, 6.8), 2.2, 0, EJ_RAW);
        // Toes: three spread forward, one inward, each ending in a hooked claw.
        for (t, ang) in [(0, -32.0f32), (1, 0.0), (2, 32.0), (3, -side * 80.0)] {
            let a = ang.to_radians() * if t < 3 { side } else { 1.0 };
            let dir = v(a.sin(), 0.0, a.cos());
            let base = foot + dir * 3.8;
            let tip = foot + dir * (if t == 3 { 6.6 } else { 9.2 }) + v(0.0, -0.6, 0.0);
            tube(&mut s, base, tip, 2.1, 1.7, EJ_RAW);
            let claw =
                [(tip + v(0.0, 0.5, 0.0), 1.4), (tip + dir * 2.4 + v(0.0, 0.3, 0.0), 0.9), (tip + dir * 3.8 + v(0.0, -1.4, 0.0), 0.3)];
            sweep(&mut s, &claw, |_, _, t| if t > 0.6 { EJ_CLAW_D } else { EJ_CLAW });
        }
        // Spikes at the elbow.
        tube(&mut s, knee + v(side * 3.6, 1.0, -2.0), knee + v(side * 6.4, 2.0, -5.0), 1.1, 0.2, EJ_HORN_D);
    }

    // ---- the tail: thick at the rump, curled round its left side along the ground, spined to its tip
    //      (held a hair above the floor, so that the feet alone make the footprint)
    s.part(8);
    let tail_pts = [
        Vec3::from(EJ_TAIL),
        v(cx - 3.0, 20.0, cz - 39.0),
        v(cx - 16.0, 12.0, cz - 43.0),
        v(cx - 30.0, 10.0, cz - 35.0),
        v(cx - 38.0, 8.0, cz - 19.0),
        v(cx - 41.0, 7.0, cz - 3.0),
        v(cx - 42.5, 6.0, cz + 8.0),
        v(cx - 46.5, 5.5, cz + 14.0),
        v(cx - 43.0, 5.4, cz + 18.5),
        v(cx - 39.5, 5.5, cz + 15.0),
    ];
    let tn = 60;
    let tail = along(&tail_pts, tn);
    let tknots: Vec<(Vec3, f32)> = tail.iter().enumerate().map(|(k, (p, _))| (*p, 10.0 - 8.2 * (k as f32 / tn as f32).powf(0.8))).collect();
    let tknots: Vec<(Vec3, f32)> =
        tknots.iter().enumerate().map(|(k, &(p, r))| (v(p.x, if k > 14 { r + 3.0 } else { p.y.max(r + 3.0) }, p.z), r)).collect();
    sweep(&mut s, &tknots, |p, c, t| {
        let nrm = (p - c).normalize_or_zero();
        if nrm.y < -0.5 {
            if (t * 120.0) as i32 % 2 == 0 {
                EJ_BELLY_D
            } else {
                EJ_BELLY
            }
        } else if nrm.y < -0.3 {
            EJ_SCALE_D
        } else {
            ej_scale(t * 44.0, nrm.x.atan2(nrm.y) * 2.4, 8)
        }
    });
    for k in (2..tn - 2).step_by(3) {
        let (c, r) = tknots[k];
        let tan = tail[k].1;
        let tall = 4.6 * (1.0 - k as f32 / tn as f32) + 1.2;
        plate(
            &mut s,
            c + v(0.0, r - 0.8, 0.0) + tan * 1.6,
            c + v(0.0, r - 0.8, 0.0) - tan * 1.6,
            c + v(0.0, r + tall, 0.0) - tan * 2.4,
            1.0,
            if k % 2 == 0 { EJ_SPINE } else { EJ_HORN_D },
        );
    }

    // ---- the seven necks, each with its head
    for i in 0..7 {
        s.part(1 + i as u8);
        ej_neck(&mut s, i);
    }
    s.paint(|x, y, z, c| (c == EJ_RAW).then_some(body_paint(x, y, z)));
    hollows(&mut s, (w, h, d), |c| !matches!(c & 0xff00_0000, mat::GLOW) && c != EJ_TOOTH, 1.4);
    s.finish(0.04)
}

// ----------------------------------------------------------------------- goryo

/// The goryō's grid: 60 wide, 124 tall, 92 deep (its train trails far behind it). Built at 0.0125
/// world units a voxel, like the third stage's foes, its cap rises about 1.5.
pub const GORYO_SIZE: (i32, i32, i32) = (60, 124, 92);
const GR_X: f32 = 29.5;
const GR_Z: f32 = 45.5;

/// Shoulders in the great sleeves, and the neck; it has no legs, only the trailing robe.
pub const GORYO_RIG: &[PartDef] = &[
    PartDef { name: "leftArm", boxes: &[], pivot: [GR_X - 11.0, 85.0, GR_Z + 1.0] },
    PartDef { name: "rightArm", boxes: &[], pivot: [GR_X + 11.0, 85.0, GR_Z + 1.0] },
    PartDef { name: "head", boxes: &[], pivot: [GR_X, 88.0, GR_Z + 1.5] },
];

/// The robe's dark cloth at `p`, folded about the axis at `c`: deep in its folds, catching
/// the light on them, woven faintly with a lozenge figure.
fn gr_cloth(p: Vec3, c: Vec3, folds: f32) -> u32 {
    let a = (p.z - c.z).atan2(p.x - c.x);
    let f = (a * folds + p.y * 0.05 + nz(p.x as i32 / 3, 1, p.z as i32 / 3) * 1.4).sin();
    let lozenge = ((p.x + p.y) as i32).rem_euclid(6) == 0 && ((p.x - p.y) as i32).rem_euclid(6) == 0;
    if f > 0.72 {
        GR_ROBE_D
    } else if f < -0.86 {
        GR_ROBE_L
    } else if lozenge {
        GR_ROBE_W
    } else {
        GR_ROBE
    }
}

/// A bolt of lightning from `a` to `b`, jagged by `jag`, white at its heart and blue about it.
fn bolt(s: &mut Sculpt, a: Vec3, b: Vec3, jag: f32, seed: i32) {
    let n = ((b - a).length() / 2.2).ceil().max(2.0) as i32;
    let (rt, up, _) = frame(b - a);
    let mut prev = a;
    for k in 1..=n {
        let t = k as f32 / n as f32;
        let off = if k == n { Vec3::ZERO } else { (rt * (nz(seed, k, 1) - 0.5) + up * (nz(seed, k, 2) - 0.5)) * (2.0 * jag) };
        let next = a.lerp(b, t) + off;
        let steps = ((next - prev).length() * 1.6).ceil() as i32;
        for j in 0..=steps {
            put(s, prev.lerp(next, j as f32 / steps.max(1) as f32), if k % 3 == 0 { GR_SPARK_B } else { GR_SPARK });
        }
        prev = next;
    }
}

/**
 * The goryō: the vengeful ghost of a wronged noble of the court, as Sugawara no Michizane's
 * struck the palace with lightning: gaunt, deathly pale, its eyes hollow and burning blue-white,
 * a thin black moustache and beard; a tall black eboshi tied under its chin, its long black
 * hair loose and floating about it; a court robe of black indigo with great sleeves, red and
 * white at their openings, the plum crest of Tenjin on its back, a paper talisman stuck to its
 * breast and another hanging from its sleeve; thin hands held out before it crackling with
 * lightning. Below the knee its robe frays into wisps that taper down to the floor, and its long
 * train trails behind it, fading.
 */
pub fn goryo() -> Collected {
    let (w, h, d) = GORYO_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (GR_X, GR_Z);

    // ---- the robe: broad at the shoulders, falling straight and widening, fraying below the knee
    s.part(0);
    for y in 18..=88 {
        let t = (y - 18) as f32 / 70.0;
        let round = ((y - 83).max(0) as f32 / 5.6).min(1.0);
        let rx = (10.9 + 7.0 * (1.0 - t).powi(2)) * (1.0 - round * round * 0.6);
        let rz = (7.5 + 4.6 * (1.0 - t).powi(2)) * (1.0 - round * round * 0.35);
        let zc = cz - 0.6 + t * 1.2;
        for z in (zc - rz - 1.0) as i32..=(zc + rz + 1.0) as i32 {
            for x in (cx - rx - 1.0) as i32..=(cx + rx + 1.0) as i32 {
                let (dx, dz) = ((x as f32 - cx) / rx, (z as f32 - zc) / rz);
                if dx.abs().powf(2.6) + dz.abs().powf(2.6) > 1.0 {
                    continue;
                }
                // Below the knee it comes apart: torn, and paling as it goes.
                let fray = (34 - y).max(0) as f32 / 16.0;
                if fray > 0.0 && nz(x, y / 2, z) < fray * 0.75 {
                    continue;
                }
                let p = v(x as f32, y as f32, z as f32);
                let c = if fray > 0.55 {
                    GR_WISP
                } else if (30..=33).contains(&y) {
                    GR_ROBE_W
                } else {
                    gr_cloth(p, v(cx, 0.0, zc), 9.0)
                };
                s.set(x, y, z, c);
            }
        }
    }
    // The round collar standing about the neck, white beneath; the belt of black leather and jade.
    for k in 0..48 {
        let a = k as f32 / 48.0 * TAU;
        let p = v(cx + a.cos() * 4.4, 88.4, cz + 1.4 + a.sin() * 4.2);
        ball(&mut s, p, v(1.3, 1.6, 1.3), GR_ROBE_L);
        put(&mut s, p + v(0.0, 1.4, 0.0) - (p - v(cx, 88.4, cz + 1.4)) * 0.2, GR_LAYER_W);
    }
    for y in 62..=64 {
        for z in 0..d {
            for x in 0..w {
                if s.get(x, y, z).is_some() && open(&s, x, y, z) && (z as f32) < cz + 3.0 {
                    let plaque = (x / 3) % 2 == 0 && y == 63;
                    s.recolor(x, y, z, if plaque { GR_JADE } else { GR_BELT });
                }
            }
        }
    }
    // The plum crest of Tenjin between its shoulders: five round petals about a heart.
    let back = (0..d).find(|&z| s.get(cx as i32, 77, z).is_some()).unwrap_or(36);
    for k in 0..5 {
        let a = k as f32 / 5.0 * TAU + PI / 2.0;
        for (dx, dy) in [(0, 0), (1, 0), (0, 1), (-1, 0), (0, -1)] {
            let x = (cx + a.cos() * 2.6).round() as i32 + dx;
            let y = (77.0 + a.sin() * 2.6).round() as i32 + dy;
            if let Some(z) = (0..d).find(|&z| s.get(x, y, z).is_some()) {
                s.recolor(x, y, z, GR_UME);
            }
        }
    }
    s.recolor(cx.round() as i32, 77, back, GR_UME_D);

    // ---- the train: hung from the back of its waist, trailing behind and fading into wisps
    let train = along(
        &[
            v(cx, 62.0, cz - 8.6),
            v(cx, 40.0, cz - 10.8),
            v(cx, 22.0, cz - 13.6),
            v(cx, 8.0, cz - 19.0),
            v(cx, 2.0, cz - 30.0),
            v(cx, 1.0, cz - 40.0),
        ],
        40,
    );
    for (k, &(c, tan)) in train.iter().enumerate() {
        let t = k as f32 / 40.0;
        let side = Vec3::X;
        let out = tan.cross(side).normalize_or(Vec3::NEG_Z);
        let half = 7.0 + t * 5.0;
        for j in -24i32..=24 {
            let u = j as f32 / 24.0;
            let p = c + side * (u * half) + out * ((u.abs() * 5.0 + t * 3.0).sin() * 1.2 - u * u * 4.0 + (t * 9.0).sin() * 2.0 * t);
            // (Torn alike on both sides, so that it trails behind the middle of it.)
            if t > 0.55 && nz(j.abs() * 2, k as i32 / 3, 4) < (t - 0.55) * 1.6 {
                continue;
            }
            // Its folds run down its length.
            let fold = (u.abs() * 6.0 + nz(j.abs(), 1, 7) * 0.8).sin();
            let col = if t > 0.8 {
                GR_WISP_L
            } else if t > 0.62 {
                GR_WISP
            } else if fold > 0.6 {
                GR_ROBE_D
            } else if fold < -0.75 {
                GR_ROBE_L
            } else {
                GR_ROBE
            };
            for k in 0..3 {
                put(&mut s, p + out * (k as f32 * 0.9), col);
            }
        }
    }
    // ---- wisps: the frayed hem drawn out into tendrils tapering down to the floor
    for k in 0..14 {
        for side in [-1.0f32, 1.0] {
            let a = (k as f32 + 0.5) / 14.0 * PI;
            let (ca, sa) = (a.cos(), a.sin());
            let start = v(cx + side * sa * 13.0, 22.0 + nz(k, 1, 3) * 8.0, cz + ca * 9.0);
            let curl = v(side * ca * 2.0, 0.0, -sa * 2.0);
            let end_y = if k % 3 == 0 { 0.0 } else { 2.0 + nz(k, 2, 5) * 10.0 };
            let pts = [
                start,
                start + v(side * sa * 1.5, -8.0, ca * 1.0) + curl,
                v(start.x - side * sa * 2.0, end_y + 4.0, start.z) - curl,
                v(start.x - side * sa * 3.0, end_y, start.z - 1.0),
            ];
            let path = along(&pts, 16);
            let knots: Vec<(Vec3, f32)> = path.iter().enumerate().map(|(j, (p, _))| (*p, 1.7 - 1.4 * j as f32 / 16.0)).collect();
            // (The tips of those that reach the floor faintly alight with ghost-light.)
            let lit = end_y < 1.0;
            sweep(&mut s, &knots, |_, _, t| {
                if lit && t > 0.9 {
                    GR_WISP_G
                } else if t > 0.7 {
                    GR_WISP_T
                } else if t > 0.3 {
                    GR_WISP_L
                } else {
                    GR_WISP
                }
            });
        }
    }

    // ---- arms: in the great sleeves, held out before it, the hands thin and crackling
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        let sh = v(cx + side * 11.0, 85.0, cz + 1.0);
        let el = v(cx + side * 14.5, 75.0, cz + 8.0);
        let wr = v(cx + side * 12.0, 73.0, cz + 21.0);
        // The sleeve: a great bag of cloth hanging from the whole arm, open down its front edge.
        let arm = along(&[sh, el, wr], 30);
        let fwd = (wr - el).normalize();
        let bag = |k: usize, shrink: f32| {
            let t = k as f32 / 30.0;
            let a = arm[k].0;
            let drop = 3.0 + 24.0 * (t * 2.2).min(1.0).powf(0.7);
            (a, a + v(side * (2.0 + 2.6 * t), -drop + shrink, -4.2 * t), 4.4 - t * 0.6 - shrink)
        };
        for k in 0..=30 {
            let (a, b, r) = bag(k, 0.0);
            tube(&mut s, a, b, r, r - 0.6, GR_ROBE);
        }
        let mid = v(cx + side * 14.0, 60.0, cz + 6.0);
        s.paint(|x, y, z, c| {
            (c == GR_ROBE && (x as f32 - cx) * side > 7.0 && y > 40).then_some(gr_cloth(v(x as f32, y as f32, z as f32), mid, 5.0))
        });
        // Hollow inside, cut open in front: the white and the red of the robes beneath at its lip.
        let mine = 1 + i as u8;
        let ahead = |p: Vec3| (p - wr).dot(fwd) + (wr.y - p.y) * 0.12;
        for y in 38..=82 {
            for z in (cz - 4.0) as i32..=(wr.z + 6.0) as i32 {
                for x in (wr.x - 9.0) as i32..=(wr.x + 9.0) as i32 {
                    if s.get(x, y, z).is_none() || s.tag_at(x, y, z) != mine {
                        continue;
                    }
                    let p = v(x as f32, y as f32, z as f32);
                    let inner = (15..=30).any(|k| {
                        let (a, b, r) = bag(k, 1.3);
                        let ab = b - a;
                        let t = ((p - a).dot(ab) / ab.length_squared().max(1e-4)).clamp(0.0, 1.0);
                        (p - a - ab * t).length() < r - 0.6 * t
                    });
                    let f = ahead(p);
                    if f > -0.4 || (inner && f > -9.0) {
                        s.clear(x, y, z);
                    }
                }
            }
        }
        for y in 38..=82 {
            for z in (cz - 4.0) as i32..=(wr.z + 6.0) as i32 {
                for x in (wr.x - 9.0) as i32..=(wr.x + 9.0) as i32 {
                    if s.get(x, y, z).is_none() || s.tag_at(x, y, z) != mine {
                        continue;
                    }
                    let f = ahead(v(x as f32, y as f32, z as f32));
                    if f > -1.3 {
                        s.recolor(x, y, z, GR_LAYER_W);
                    } else if f > -2.3 {
                        s.recolor(x, y, z, GR_LAYER_R);
                    } else if open(&s, x, y, z)
                        && f > -9.0
                        && NEIGHBORS.iter().any(|&(dx, dy, dz)| {
                            s.get(x + dx, y + dy, z + dz).is_none()
                                && ahead(v((x + dx) as f32, (y + dy) as f32, (z + dz) as f32)) < -0.4
                                && (x + dx - wr.x as i32).abs() < 4
                        })
                    {
                        s.recolor(x, y, z, GR_ROBE_D);
                    }
                }
            }
        }
        // The hand: bony, held out palm forward, the long fingers splayed and hooked.
        let palm = wr + v(-side * 0.4, 1.4, 2.0);
        rounded(&mut s, palm, v(1.9, 2.2, 0.9), 2.0, 0, GR_SKIN);
        tube(&mut s, wr - fwd * 2.0, palm, 1.2, 1.3, GR_SKIN);
        for f in 0..4 {
            let ff = f as f32 - 1.5;
            let base = palm + v(ff * 1.1 * side, 1.8, 0.2);
            let k1 = base + v(ff * 0.9 * side, 3.0, 0.8);
            let k2 = k1 + v(ff * 0.4 * side, 2.0, 1.6);
            tube(&mut s, base, k1, 0.62, 0.55, GR_SKIN);
            tube(&mut s, k1, k2, 0.55, 0.4, GR_SKIN);
            put(&mut s, k2 + v(0.0, 0.4, 0.6), GR_NAIL);
            ball(&mut s, k1, Vec3::splat(0.7), GR_SKIN_D);
        }
        let t1 = palm + v(-side * 2.6, 0.4, 0.6);
        tube(&mut s, palm + v(-side * 1.2, -0.6, 0.0), t1, 0.7, 0.6, GR_SKIN);
        tube(&mut s, t1, t1 + v(-side * 1.4, 1.8, 1.0), 0.6, 0.4, GR_SKIN);
        // Lightning crackling about it, a few sparks loose.
        for b in 0..3 {
            let to = palm + v(side * (2.0 + b as f32 * 2.4), 3.0 + b as f32 * 2.8 - 2.0, 6.0 + b as f32);
            bolt(&mut s, palm + v(0.0, 2.0, 1.0), to, 1.1, 10 + b + i as i32 * 7);
        }
        bolt(&mut s, palm + v(-side * 1.4, 4.0, 1.6), palm + v(-side * 6.0, 9.0, 4.0), 1.0, 31 + i as i32);
        for k in 0..6 {
            let sp = palm + v((nz(k, 1, i as i32) - 0.5) * 9.0, (nz(k, 2, i as i32) - 0.3) * 10.0, nz(k, 3, i as i32) * 6.0 + 1.0);
            put(&mut s, sp, if k % 2 == 0 { GR_SPARK } else { GR_SPARK_B });
        }
        // A talisman hung from the left sleeve, torn at its foot.
        if i == 0 {
            let top = v(cx - 17.4, 56.0, cz + 10.0);
            for y in 0..10 {
                for x in 0..3 {
                    if y < 2 && (x + y) % 2 == 0 && nz(x, y, 7) < 0.7 {
                        continue;
                    }
                    let c = if y == 8 && x == 1 {
                        GR_SEAL
                    } else if x == 1 && y > 1 && y < 8 && y % 2 == 0 {
                        GR_INK
                    } else {
                        GR_PAPER
                    };
                    s.set(top.x as i32 - x, top.y as i32 - 9 + y, top.z as i32, c);
                }
            }
            tube(&mut s, top + v(-1.0, 1.0, 0.0), top + v(-1.0, 4.0, -1.0), 0.4, 0.4, GR_LAYER_W);
        }
    }
    // A talisman stuck to its breast.
    s.part(0);
    let bz = front(&s, (cx + 3.0) as i32, 74, d).unwrap_or(56) + 1;
    for y in 0..11 {
        for x in 0..4 {
            if y < 2 && nz(x, y, 3) < 0.4 {
                continue;
            }
            let c = if y == 9 && (1..=2).contains(&x) {
                GR_SEAL
            } else if (1..=2).contains(&x) && y > 1 && y < 8 && (y + x) % 3 != 0 {
                GR_INK
            } else {
                GR_PAPER
            };
            s.set((cx + 1.0) as i32 + x, 67 + y, bz, c);
        }
    }

    // ---- the head: gaunt and deathly pale, the sockets hollow and burning, the eboshi, the hair afloat
    s.part(3);
    let hc = v(cx, 98.0, cz + 2.4);
    tube(&mut s, v(cx, 86.5, cz + 1.4), v(cx, 93.0, cz + 2.6), 2.2, 2.0, GR_SKIN_D);
    ball(&mut s, hc, v(5.2, 6.8, 5.8), GR_SKIN);
    rounded(&mut s, v(cx, 93.0, cz + 4.4), v(3.6, 2.6, 3.8), 2.2, 0, GR_SKIN);
    let fz = front(&s, cx as i32, 98, d).unwrap_or(56) as f32;
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 3.4, 98.0, fz - 1.8), v(1.5, 1.1, 1.3), GR_SKIN);
        // The cheeks sunken under the bones, the temples hollow.
        for y in 94..=97 {
            if let Some(z) = front(&s, (cx + side * 3.2).round() as i32, y, d) {
                s.recolor((cx + side * 3.2).round() as i32, y, z, GR_SKIN_DD);
            }
        }
        // The hollow of the eye, black, a cold blue-white fire burning in it.
        let ex = (cx + side * 2.3).round() as i32;
        for (dx, dy) in [(0, 0), (1, 0), (-1, 0), (0, 1), (side as i32, 1)] {
            if let Some(z) = front(&s, ex + dx, 99 + dy, d) {
                s.recolor(ex + dx, 99 + dy, z, GR_SOCKET);
            }
        }
        if let Some(z) = front(&s, ex, 100, d) {
            s.set(ex, 100, z, GR_EYE);
        }
        if let Some(z) = front(&s, ex - side as i32, 99, d) {
            s.set(ex - side as i32, 99, z, GR_EYE_B);
        }
        // The brows drawn down in wrath toward the nose.
        for k in 0..4 {
            let (x, y) = ((cx + side * (1.0 + k as f32)).round() as i32, 101 + (k + 1) / 2);
            if let Some(z) = front(&s, x, y, d) {
                s.recolor(x, y, z, GR_HAIR);
            }
        }
    }
    // The nose, long and thin; the grey mouth turned down; the moustache drooping and the little beard.
    tube(&mut s, v(cx, 99.4, fz), v(cx, 96.6, fz + 1.6), 0.8, 0.9, GR_SKIN);
    for (x, y) in [(-2, 93), (-1, 94), (0, 94), (1, 94), (2, 93)] {
        let x = cx.round() as i32 + x;
        if let Some(z) = front(&s, x, y, d) {
            s.recolor(x, y, z, GR_SOCKET);
        }
        if let Some(z) = front(&s, x, y - 1, d) {
            s.recolor(x, y - 1, z, GR_LIP);
        }
    }
    for side in [-1.0f32, 1.0] {
        tube(&mut s, v(cx + side * 0.6, 95.6, fz + 0.8), v(cx + side * 3.0, 94.4, fz - 0.4), 0.5, 0.45, GR_HAIR);
        tube(&mut s, v(cx + side * 3.0, 94.4, fz - 0.4), v(cx + side * 3.6, 90.6, fz - 1.4), 0.45, 0.3, GR_HAIR);
    }
    tube(&mut s, v(cx, 91.6, fz - 0.4), v(cx, 87.6, fz - 0.2), 0.9, 0.3, GR_HAIR);
    // The hair: black over the back of the head and long down its back, and loose locks falling
    // from under the cap and drifting out behind it, their ends lifting as if in deep water.
    ball(&mut s, hc + v(0.0, 0.6, -1.2), v(5.6, 6.6, 5.6), GR_HAIR);
    s.paint(|x, y, z, c| {
        let p = v(x as f32, y as f32, z as f32) - hc;
        (c == GR_HAIR && p.z > 0.5 && p.y < 3.5).then_some(GR_SKIN)
    });
    ball(&mut s, v(cx, 84.0, cz - 6.4), v(6.8, 16.0, 2.8), GR_HAIR);
    let lock = |s: &mut Sculpt, pts: &[Vec3], r: f32, k: i32| {
        let path = along(pts, 24);
        let knots: Vec<(Vec3, f32)> = path.iter().enumerate().map(|(j, (p, _))| (*p, r - (r - 0.5) * j as f32 / 24.0)).collect();
        sweep(s, &knots, |p, _, t| if nz(p.x as i32, p.y as i32 / 2, k) < 0.18 + t * 0.25 { GR_HAIR_L } else { GR_HAIR });
    };
    for k in 0..20 {
        let a = (k as f32 / 19.0 - 0.5) * 1.24 * PI;
        let (sa, ca) = (a.sin(), a.cos());
        let back = ca.max(0.0);
        let root = hc + v(sa * 5.0, 1.0 + nz(k, 1, 1) * 2.0, -ca * 4.6 - 1.4);
        let (rise1, rise2) = (nz(k, 2, 1) * 4.0, nz(k, 3, 1) * 4.0);
        let wob = (nz(k, 4, 1) - 0.5) * 4.0;
        let p1 = root + v(sa * 1.8, -6.0 - nz(k, 5, 1) * 4.0, -2.0 - back * 1.5);
        let p2 = p1 + v(sa * 5.0 + wob, -7.0 + rise1, -5.0 - back * 2.0);
        let p3 = p2 + v(sa * 5.0 - wob, rise2, -6.0 - back * 2.0);
        lock(&mut s, &[root, p1, p2, p3], 2.3, k);
    }
    for side in [-1.0f32, 1.0] {
        let root = v(cx + side * 5.0, 98.0, cz + 1.8);
        lock(
            &mut s,
            &[root, v(cx + side * 6.8, 90.0, cz + 5.6), v(cx + side * 7.8, 80.0, cz + 9.0), v(cx + side * 8.6, 70.0, cz + 9.6)],
            1.7,
            40 + side as i32,
        );
    }
    // The eboshi: a tall black cap of lacquered gauze, creased, its crown bent forward, tied under the chin.
    for y in 101..=121 {
        let t = (y - 101) as f32 / 20.0;
        let lean = t * t * 4.6;
        let top = if t > 0.8 { (1.0 - ((t - 0.8) / 0.2).powi(2)).max(0.0).sqrt() } else { 1.0 };
        let (rx, rz) = ((5.0 - t * 1.0) * top, (5.6 - t * 0.8) * top);
        let zc = cz + 2.0 + lean;
        for z in (zc - rz - 1.0) as i32..=(zc + rz + 1.0) as i32 {
            for x in (cx - rx - 1.0) as i32..=(cx + rx + 1.0) as i32 {
                let (dx, dz) = ((x as f32 - cx) / rx.max(0.3), (z as f32 - zc) / rz.max(0.3));
                if dx.abs().powf(2.2) + dz.abs().powf(2.2) > 1.0 {
                    continue;
                }
                let crease = (x as f32 - cx + (y as f32 * 0.4).sin()).abs() < 0.6 && z as f32 > zc;
                s.set(x, y, z, if crease || (y == 104) { GR_CAP_L } else { GR_CAP });
            }
        }
    }
    for side in [-1.0f32, 1.0] {
        tube(&mut s, v(cx + side * 4.6, 103.0, cz + 3.2), v(cx + side * 4.0, 95.0, cz + 4.4), 0.4, 0.4, GR_CORD);
        tube(&mut s, v(cx + side * 4.0, 95.0, cz + 4.4), v(cx + side * 0.8, 90.6, cz + 5.2), 0.4, 0.4, GR_CORD);
    }
    hollows(&mut s, (w, h, d), |c| !matches!(c & 0xff00_0000, mat::GLOW) && !matches!(c, GR_PAPER | GR_SEAL | GR_INK), 1.3);
    s.finish(0.04)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sixth_stage_foe_stands_in_the_middle_of_its_grid_and_inside_it() {
        for (name, c, size) in [
            ("Tepegöz", tepegoz(), TEPEGOZ_SIZE),
            ("gashadokuro", gashadokuro(), GASHADOKURO_SIZE),
            ("ejder", ejder(), EJDER_SIZE),
            ("goryō", goryo(), GORYO_SIZE),
        ] {
            assert!(c.voxels.len() > 2000, "{name}: {} voxels", c.voxels.len());
            assert_eq!((c.width, c.height, c.depth), size, "{name}");
            // On the floor (the goryō by the wisps of its robe), and its footprint about the grid's middle.
            assert!(c.voxels.iter().map(|v| v.y).min().is_some_and(|y| y <= 1), "{name} stands on the floor");
            assert!(!c.voxels.iter().any(|v| v.x == 0), "{name}: a stray voxel on the grid's edge (a NaN)");
            let floor: Vec<_> = c.voxels.iter().filter(|v| v.y <= 2).collect();
            let mx = floor.iter().map(|v| v.x as f32).sum::<f32>() / floor.len() as f32;
            assert!((mx - (size.0 - 1) as f32 / 2.0).abs() < 3.0, "{name}: footprint centred at x {mx}");
            // Nothing cut off: a voxel to spare at every side but the floor.
            let edge = c.voxels.iter().find(|v| v.x >= size.0 - 1 || v.y >= size.1 - 1 || v.z <= 0 || v.z >= size.2 - 1);
            assert!(edge.is_none(), "{name}: a voxel against the grid's side at {:?}", edge.map(|v| (v.x, v.y, v.z)));
        }
    }

    #[test]
    fn every_sixth_stage_joint_lies_inside_its_grid_on_the_part_it_swings() {
        for (name, c, size, rig) in [
            ("Tepegöz", tepegoz(), TEPEGOZ_SIZE, TEPEGOZ_RIG),
            ("gashadokuro", gashadokuro(), GASHADOKURO_SIZE, GASHADOKURO_RIG),
            ("ejder", ejder(), EJDER_SIZE, EJDER_RIG),
            ("goryō", goryo(), GORYO_SIZE, GORYO_RIG),
        ] {
            for (i, def) in rig.iter().enumerate() {
                let p = Vec3::from(def.pivot);
                let top = Vec3::new(size.0 as f32 - 1.0, size.1 as f32 - 1.0, size.2 as f32 - 1.0);
                assert!(p.cmpge(Vec3::ZERO).all() && p.cmple(top).all(), "{name} {}: {p} outside the grid", def.name);
                let mine: Vec<_> = c.voxels.iter().filter(|v| v.part as usize == i + 1).collect();
                assert!(mine.len() > 40, "{name} {}: {} voxels", def.name, mine.len());
                // The joint within the part's bounds (a jaw hinges on the line between its two knuckles).
                let (lo, hi) = mine.iter().fold((Vec3::MAX, Vec3::MIN), |(lo, hi), v| {
                    let q = Vec3::new(v.x as f32, v.y as f32, v.z as f32);
                    (lo.min(q), hi.max(q))
                });
                assert!(p.cmpge(lo - 2.0).all() && p.cmple(hi + 2.0).all(), "{name} {}: its joint {p} is off it ({lo} to {hi})", def.name);
            }
        }
    }

    #[test]
    fn the_ejders_seven_necks_never_share_a_voxel_and_hinge_on_its_shoulders() {
        let (w, h, d) = EJDER_SIZE;
        let necks: Vec<Vec<bool>> = (0..7)
            .map(|i| {
                let mut s = Sculpt::new(w, h, d);
                ej_neck(&mut s, i);
                let mut cells = vec![false; (w * h * d) as usize];
                for y in 0..h {
                    for z in 0..d {
                        for x in 0..w {
                            cells[((y * d + z) * w + x) as usize] = s.get(x, y, z).is_some();
                        }
                    }
                }
                cells
            })
            .collect();
        for i in 0..7 {
            for j in i + 1..7 {
                let shared = necks[i].iter().zip(&necks[j]).filter(|(a, b)| **a && **b).count();
                assert_eq!(shared, 0, "neck{i} and neck{j} share {shared} voxels");
            }
        }
        for (i, def) in EJDER_RIG.iter().take(7).enumerate() {
            assert_eq!(def.name, format!("neck{i}"));
            assert_eq!(def.pivot, EJ_ROOTS[i], "{}", def.name);
        }
    }

    #[test]
    fn tepegoz_has_one_eye_and_it_is_in_the_middle_of_his_brow() {
        let c = tepegoz();
        let eye: Vec<_> = c.voxels.iter().filter(|v| v.c & 0xff00_0000 == mat::SLIME).collect();
        assert!(eye.len() > 40, "{} voxels of eye", eye.len());
        assert!(eye.iter().all(|v| v.part == 5), "his eye moves with his head");
        let m = eye.iter().fold(Vec3::ZERO, |a, v| a + Vec3::new(v.x as f32, v.y as f32, v.z as f32)) / eye.len() as f32;
        assert!((m.x - TP_X).abs() < 0.6, "one eye, in the middle: {m}");
        assert!(m.y > 119.0 && m.y < 128.0, "high on his brow: {m}");
    }

    /// Every model's sheet (front, side, back / top, two three-quarter views), into VH_PREVIEW_DIR.
    #[test]
    #[ignore]
    fn preview_stage_six() {
        for (name, c) in [("tepegoz", tepegoz()), ("gashadokuro", gashadokuro()), ("ejder", ejder()), ("goryo", goryo())] {
            println!("{}", super::super::preview::sheet(&c, name));
        }
    }
}
