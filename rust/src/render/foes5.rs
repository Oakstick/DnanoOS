//! The fifth stage's cast, out of the nights of Anatolia and sculpted as finely as the third's
//! (see `foes3`): the tülükuş, a great horned owl gone to the bad, its eyes burning orange in a
//! pale disc rimmed with black; the albastı, a gaunt night hag hunched under tangled black hair,
//! her arms so long that her claws all but drag; the cin, a djinn of smokeless fire in the shape
//! of a glaring old man in a hood, a crooked staff burning in his hand; the arçura, the spirit of
//! the forest, lanky, of bark and leaves, with a beard of grass and horns like branches; and the
//! karakoncolos, the frost-pale demon of the zemheri, ape-armed and shaggy under coiled ram's
//! horns. Each faces +z, stands in the middle of its grid (where its rig sets it down), and is
//! sculpted in the parts its rig swings.
//!
//! Between them they fight all three ways (COMBAT.md §10). The karakoncolos comes to blows: it
//! roars your name in the voice of someone you love, dooming you, then strikes with its claws.
//! The tülükuş and the albastı come at you with their bodies: the owl drops its talons without a
//! sound, and its hoot foretells a death; the hag falls on you with the weight of a nightmare.
//! The cin and the arçura fight from afar: the cin hurls smokeless fire out of nowhere, and
//! vanishes to appear elsewhere; the arçura throws darts of root and thorn that burst into roots
//! where they strike, and changes its size.

use std::f32::consts::{PI, TAU};

use glam::Vec3;

use super::carve::*;
use super::characters::{mat, Sculpt};
use super::voxels::{Collected, PartDef};

mod pal {
    use super::mat;
    pub const PUPIL_G: u32 = 0x0a0807 | mat::SLIME;
    pub const GLINT_G: u32 = 0xfff4e0 | mat::SLIME;
    // The tülükuş.
    pub const OWL: u32 = 0x56381f | mat::FUR;
    pub const OWL_D: u32 = 0x3a2515 | mat::FUR;
    pub const OWL_K: u32 = 0x18100a | mat::FUR;
    pub const OWL_L: u32 = 0x7a5530 | mat::FUR;
    pub const TAWNY: u32 = 0x9c7446 | mat::FUR;
    pub const TAWNY_L: u32 = 0xbf9862 | mat::FUR;
    pub const TAWNY_D: u32 = 0x76552f | mat::FUR;
    pub const BREAST: u32 = 0xa47c4c | mat::FUR;
    pub const LEGF: u32 = 0x8e6a40 | mat::FUR;
    pub const FLUFF: u32 = 0x86643c | mat::FUR;
    pub const DISC: u32 = 0xb3a083 | mat::FUR;
    pub const DISC_D: u32 = 0x857256 | mat::FUR;
    pub const DISC_L: u32 = 0xcdbd9f | mat::FUR;
    pub const RIM: u32 = 0x1a110a | mat::FUR;
    pub const BEAK: u32 = 0x262222 | mat::IVORY;
    pub const BEAK_L: u32 = 0x4c4644 | mat::IVORY;
    pub const TOE: u32 = 0x3e3630 | mat::SKIN;
    pub const TOE_D: u32 = 0x2c2622 | mat::SKIN;
    pub const TALON: u32 = 0x100e0e | mat::IVORY;
    pub const IRIS: u32 = 0xff7414 | mat::GLOW;
    pub const IRIS_L: u32 = 0xffb238 | mat::GLOW;
    pub const IRIS_D: u32 = 0xc4380a | mat::GLOW;
    // The albastı.
    pub const AL_SKIN: u32 = 0x2e3644 | mat::SKIN;
    pub const AL_SKIN_D: u32 = 0x1e242e | mat::SKIN;
    pub const AL_SKIN_DD: u32 = 0x12161c | mat::SKIN;
    pub const AL_SKIN_L: u32 = 0x56647e | mat::SKIN;
    pub const AL_HAIR: u32 = 0x0e0e12 | mat::FUR;
    pub const AL_HAIR_L: u32 = 0x23242e | mat::FUR;
    pub const AL_HAIR_B: u32 = 0x2c3446 | mat::FUR;
    pub const AL_CLAW: u32 = 0x0c0a0b | mat::IVORY;
    pub const AL_CLAW_L: u32 = 0x2a2428 | mat::IVORY;
    pub const AL_EYE: u32 = 0xff2c18 | mat::GLOW;
    pub const AL_EYE_L: u32 = 0xffa860 | mat::GLOW;
    pub const AL_MOUTH: u32 = 0x2a080c | mat::SKIN;
    pub const AL_LIP: u32 = 0x3c1c26 | mat::SKIN;
    pub const AL_TOOTH: u32 = 0xe4dccb | mat::IVORY;
    pub const AL_RAG: u32 = 0x1f1b1d | mat::CLOTH;
    pub const AL_RAG_D: u32 = 0x121011 | mat::CLOTH;
    pub const AL_RAG_L: u32 = 0x36302f | mat::CLOTH;
    pub const AL_RED: u32 = 0x5e1418 | mat::CLOTH;
    pub const AL_RED_D: u32 = 0x3a0b0f | mat::CLOTH;
    pub const AL_RED_L: u32 = 0x7e2226 | mat::CLOTH;
    // The cin.
    pub const CIN_ROBE: u32 = 0x4a3324 | mat::CLOTH;
    pub const CIN_ROBE_D: u32 = 0x2e2018 | mat::CLOTH;
    pub const CIN_ROBE_L: u32 = 0x664834 | mat::CLOTH;
    pub const CIN_UNDER: u32 = 0x5e5a56 | mat::CLOTH;
    pub const CIN_UNDER_D: u32 = 0x403c3a | mat::CLOTH;
    pub const CIN_SHAWL: u32 = 0x55514d | mat::CLOTH;
    pub const CIN_SHAWL_D: u32 = 0x37342f | mat::CLOTH;
    pub const CIN_SHAWL_L: u32 = 0x76716b | mat::CLOTH;
    pub const CIN_SASH: u32 = 0x261b16 | mat::CLOTH;
    pub const CIN_SKIN: u32 = 0xa48e7c | mat::SKIN;
    pub const CIN_SKIN_D: u32 = 0x6e5a4a | mat::SKIN;
    pub const CIN_SKIN_L: u32 = 0xc2ac98 | mat::SKIN;
    pub const CIN_HAIR: u32 = 0xe6e2da | mat::FUR;
    pub const CIN_HAIR_S: u32 = 0xb8b4ac | mat::FUR;
    pub const CIN_HAIR_D: u32 = 0x8c8882 | mat::FUR;
    pub const CIN_EYE: u32 = 0xfff2b8 | mat::GLOW;
    pub const CIN_EYE_D: u32 = 0xd8a040 | mat::GLOW;
    pub const CIN_WOOD: u32 = 0x5c4028 | mat::WOOD;
    pub const CIN_WOOD_D: u32 = 0x3c2a1a | mat::WOOD;
    pub const CIN_WOOD_L: u32 = 0x7c5a3a | mat::WOOD;
    pub const CIN_SLIPPER: u32 = 0x3a2a20 | mat::LEATHER;
    pub const FLAME_CORE: u32 = 0xfffbe6 | mat::GLOW;
    pub const FLAME_GOLD: u32 = 0xffe48a | mat::GLOW;
    pub const FLAME_EDGE: u32 = 0xffb25a | mat::GLOW;
    pub const SMOKE: u32 = 0x9e9a96 | mat::FUR;
    pub const SMOKE_D: u32 = 0x7a7672 | mat::FUR;
    pub const SMOKE_L: u32 = 0xbcb8b4 | mat::FUR;
    // The arçura.
    pub const AR_BARK: u32 = 0x5a4630 | mat::WOOD;
    pub const AR_BARK_D: u32 = 0x36291b | mat::WOOD;
    pub const AR_BARK_L: u32 = 0x7c654a | mat::WOOD;
    pub const AR_BARK_G: u32 = 0x6c675a | mat::WOOD;
    pub const AR_HOLE: u32 = 0x120c07 | mat::WOOD;
    pub const AR_MOSS: u32 = 0x4c7028 | mat::FUR;
    pub const AR_MOSS_D: u32 = 0x33501c | mat::FUR;
    pub const AR_MOSS_L: u32 = 0x6e923a | mat::FUR;
    pub const AR_LICHEN: u32 = 0xa4ac8a | mat::FUR;
    pub const AR_LEAF: u32 = 0x3c782c | mat::LEATHER;
    pub const AR_LEAF_D: u32 = 0x285620 | mat::LEATHER;
    pub const AR_LEAF_L: u32 = 0x5c983a | mat::LEATHER;
    pub const AR_LEAF_Y: u32 = 0x8ca83a | mat::LEATHER;
    pub const AR_VINE: u32 = 0x4a5e24 | mat::WOOD;
    pub const AR_GRASS: u32 = 0x6cb83c | mat::FUR;
    pub const AR_GRASS_Y: u32 = 0xa8c842 | mat::FUR;
    pub const AR_GRASS_D: u32 = 0x3e8a2c | mat::FUR;
    pub const AR_FLOWER: u32 = 0xf0ecd0 | mat::FUR;
    pub const AR_EYE: u32 = 0x78ff52 | mat::GLOW;
    pub const AR_EYE_L: u32 = 0xd8ffb4 | mat::GLOW;
    pub const AR_HOOF: u32 = 0x2a2420 | mat::IVORY;
    pub const AR_HOOF_L: u32 = 0x4a4038 | mat::IVORY;
    // The karakoncolos.
    pub const KK_SKIN: u32 = 0x8696aa | mat::SKIN;
    pub const KK_SKIN_D: u32 = 0x5e6c82 | mat::SKIN;
    pub const KK_SKIN_DD: u32 = 0x3c4658 | mat::SKIN;
    pub const KK_SKIN_L: u32 = 0xacbccf | mat::SKIN;
    pub const KK_FUR: u32 = 0x1a2030 | mat::FUR;
    pub const KK_FUR_D: u32 = 0x0f131d | mat::FUR;
    pub const KK_FUR_L: u32 = 0x2c3850 | mat::FUR;
    pub const KK_FUR_T: u32 = 0x4c5a76 | mat::FUR;
    pub const KK_RIME: u32 = 0xc4d4ea | mat::FUR;
    pub const KK_HORN: u32 = 0xc6c0ae | mat::IVORY;
    pub const KK_HORN_D: u32 = 0x948c78 | mat::IVORY;
    pub const KK_HORN_T: u32 = 0x5a5446 | mat::IVORY;
    pub const KK_FROST: u32 = 0xe8f4ff | mat::IVORY;
    pub const KK_EYE: u32 = 0x86d6ff | mat::GLOW;
    pub const KK_EYE_L: u32 = 0xeafaff | mat::GLOW;
    pub const KK_MOUTH: u32 = 0x1a1222 | mat::SKIN;
    pub const KK_GUM: u32 = 0x4a3048 | mat::SKIN;
    pub const KK_TONGUE: u32 = 0x5e3a5c | mat::SKIN;
    pub const KK_TOOTH: u32 = 0xeeeadc | mat::IVORY;
    pub const KK_CLAW: u32 = 0x0e0e12 | mat::IVORY;
    pub const KK_CLAW_L: u32 = 0x2a2a34 | mat::IVORY;
    pub const ICE: u32 = 0xa4e4f2 | mat::SLIME;
    pub const ICE_L: u32 = 0xdaf8ff | mat::SLIME;
    pub const ICE_D: u32 = 0x6cbcd6 | mat::SLIME;
}
use pal::*;

/// Sets the voxel nearest `p`.
fn put(s: &mut Sculpt, p: Vec3, c: u32) {
    s.set(p.x.round() as i32, p.y.round() as i32, p.z.round() as i32, c);
}

fn bezier(p: [Vec3; 3], t: f32) -> Vec3 {
    let u = 1.0 - t;
    p[0] * (u * u) + p[1] * (2.0 * u * t) + p[2] * (t * t)
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

/// A curved claw from `root` toward `tip`, bowed toward `bow`, tapering from `r` to a point.
fn claw(s: &mut Sculpt, root: Vec3, tip: Vec3, bow: Vec3, r: f32, c: u32) {
    let mid = (root + tip) * 0.5 + bow;
    let knots: Vec<(Vec3, f32)> =
        (0..=6).map(|k| k as f32 / 6.0).map(|t| (bezier([root, mid, tip], t), (r * (1.0 - t * 0.8)).max(0.7))).collect();
    sweep(s, &knots, |_, _, _| c);
}

/// A feather's outline along it (0 to 1): narrow at the quill, broad along its length, rounded at its tip.
fn quill(t: f32) -> f32 {
    let tip = 1.0 - ((t - 0.7) / 0.3).max(0.0).powi(2);
    (0.55 + 0.45 * (t / 0.25).min(1.0)) * tip.max(0.0).sqrt()
}

/**
 * A flat vane from `root` to `tip` (a feather, a leaf), its face toward `normal`, `width` across
 * at its broadest: `shape(t)` is its half-width along it (t from the root, 0 to 1) and
 * `paint(t, u)` its colour (u across it, -1 to 1). It is a voxel and a half thick.
 */
fn vane(s: &mut Sculpt, root: Vec3, tip: Vec3, normal: Vec3, width: f32, shape: impl Fn(f32) -> f32, paint: impl Fn(f32, f32) -> u32) {
    let along = tip - root;
    let len = along.length().max(1e-4);
    let across = normal.cross(along / len).normalize_or_zero();
    let n = (along / len).cross(across);
    let steps = (len * 2.5).ceil() as i32;
    for k in 0..=steps {
        let t = k as f32 / steps as f32;
        let half = width * 0.5 * shape(t);
        if half <= 0.0 {
            continue;
        }
        let m = (half * 2.5).ceil() as i32;
        for j in -m..=m {
            let u = j as f32 / m as f32;
            let p = root + along * t + across * (u * half);
            let c = paint(t, u);
            put(s, p, c);
            put(s, p + n * 0.6, c);
        }
    }
}

/**
 * A lock of hair from `root`, setting out along `dir` and falling under its own weight for `len`
 * voxels; where it meets anything solid (anything but hair and fur) it slides off toward `off`.
 * `seed` sets how it wanders, and `paint(t)` colours it along its length (t from 0 to 1).
 */
fn drape(s: &mut Sculpt, root: Vec3, dir: Vec3, len: f32, off: Vec3, seed: i32, paint: impl Fn(f32) -> u32) {
    let solid = |s: &Sculpt, p: Vec3| s.get(p.x.round() as i32, p.y.round() as i32, p.z.round() as i32).is_some_and(|c| c >> 24 != 2);
    let (mut p, mut d) = (root, dir.normalize_or_zero());
    let steps = (len * 2.0) as i32;
    for k in 0..=steps {
        let t = k as f32 / steps.max(1) as f32;
        let wander = v(nz(seed, k / 5, 1) - 0.5, 0.0, nz(seed, k / 5, 2) - 0.5) * 0.22;
        // (Its weight pulls it straight down: whatever way it set out or wandered fades as it falls.)
        d = (v(d.x * 0.9, d.y, d.z * 0.9) + v(0.0, -0.16, 0.0) + wander).normalize_or_zero();
        let mut next = p + d * 0.5;
        for _ in 0..14 {
            if !solid(s, next) {
                break;
            }
            next += off * 0.5;
        }
        // (A lock pushed far round something is drawn all the way, not left with a gap.)
        let gap = next - p;
        let n = (gap.length() / 0.5).ceil().max(1.0) as i32;
        for j in 1..=n {
            put(s, p + gap * (j as f32 / n as f32), paint(t));
        }
        p = next;
    }
}

// ------------------------------------------------------------------ tülükuş

/// The tülükuş's grid: 48 wide, 88 tall (its ear tufts), 46 deep.
pub const TULUKUS_SIZE: (i32, i32, i32) = (48, 88, 46);
const TULUKUS_Z: f32 = 22.5;
const OWL_X: f32 = 23.5;
/// The owl's body: a belly and a chest, (centre, radii), so its folded wings can lie along its sides.
const OWL_BODY: [(Vec3, Vec3); 2] = [
    (Vec3::new(OWL_X, 28.5, TULUKUS_Z), Vec3::new(13.8, 16.0, 11.6)),
    (Vec3::new(OWL_X, 44.0, TULUKUS_Z - 0.5), Vec3::new(13.0, 13.0, 11.0)),
];

/// Wings at the shoulders, the head at the neck, legs at the hips.
pub const TULUKUS_RIG: &[PartDef] = &[
    PartDef { name: "leftWing", boxes: &[], pivot: [11.5, 53.0, 21.5] },
    PartDef { name: "rightWing", boxes: &[], pivot: [35.5, 53.0, 21.5] },
    PartDef { name: "head", boxes: &[], pivot: [23.5, 54.0, 23.0] },
    PartDef { name: "leftLeg", boxes: &[], pivot: [17.5, 16.0, 23.5] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [29.5, 16.0, 23.5] },
];

/// How far the owl's body reaches out to either side at (y, z), from its middle.
fn owl_side(y: f32, z: f32) -> f32 {
    OWL_BODY
        .iter()
        .map(|(c, r)| {
            let q = 1.0 - ((y - c.y) / r.y).powi(2) - ((z - c.z) / r.z).powi(2);
            r.x * q.max(0.0).sqrt()
        })
        .fold(0.0, f32::max)
}

/**
 * One of the owl's wing feathers: drawn flat in side view from `root` to `tip` (each (z, y), z
 * from the owl's middle), then laid on its body on `side`, `lift.0` voxels out from it; toward
 * its tip it bends `lift.1` voxels in, past the middle where no body is under it (the primaries
 * crossing over the tail). `paint(t, u)` colours it as `vane` does, u = 1 its front edge.
 */
fn owl_feather(
    s: &mut Sculpt,
    side: f32,
    root: (f32, f32),
    tip: (f32, f32),
    width: f32,
    lift: (f32, f32),
    paint: impl Fn(f32, f32) -> u32,
) {
    let (dz, dy) = (tip.0 - root.0, tip.1 - root.1);
    let len = (dz * dz + dy * dy).sqrt().max(1e-4);
    let (az, ay) = (-dy / len, dz / len);
    let steps = (len * 2.5).ceil() as i32;
    for k in 0..=steps {
        let t = k as f32 / steps as f32;
        let half = width * 0.5 * quill(t);
        if half <= 0.0 {
            continue;
        }
        let m = (half * 2.5).ceil() as i32;
        for j in -m..=m {
            let u = j as f32 / m as f32;
            let (z, y) = (root.0 + dz * t + az * u * half, root.1 + dy * t + ay * u * half);
            let body = owl_side(y, TULUKUS_Z + z);
            let mut off = body + lift.0 - lift.1 * ((t - 0.55) / 0.45).clamp(0.0, 1.0).powi(2);
            if body > 0.0 {
                off = off.max(body + 0.6);
            }
            let p = v(OWL_X + side * off, y, TULUKUS_Z + z);
            let c = paint(t, u);
            put(s, p, c);
            put(s, p - v(side * 0.8, 0.0, 0.0), c);
        }
    }
}

/**
 * The tülükuş: a great horned owl gone to the bad, deep brown barred with black, its breast
 * paler and streaked; ear tufts standing up like horns over a scowl, a pale facial disc rimmed in
 * black round eyes that burn orange, a hooked black beak; its wings folded down its sides, their
 * long primaries crossed over its tail, and heavy feathered legs gripping the ground with black
 * talons.
 */
pub fn tulukus() -> Collected {
    let (w, h, d) = TULUKUS_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (OWL_X, TULUKUS_Z);

    // ---- legs: feathered trousers down to the toes, three toes forward and one back, black talons
    for (i, lx) in [cx - 6.0, cx + 6.0].into_iter().enumerate() {
        s.part(4 + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        ball(&mut s, v(lx, 13.5, cz + 1.2), v(4.8, 5.0, 5.2), LEGF);
        s.capsule((lx, 13.0, cz + 1.2), (lx + out * 0.6, 6.0, cz + 2.2), 4.2, 3.5, LEGF);
        let foot = v(lx + out * 0.8, 3.4, cz + 2.8);
        ball(&mut s, foot, v(3.7, 2.8, 3.7), LEGF);
        for a in [-0.6f32, 0.0, 0.6] {
            let dir = v((a + out * 0.15).sin(), 0.0, (a + out * 0.15).cos());
            let tip = foot + dir * 7.2 + v(0.0, -1.5, 0.0);
            s.capsule((foot.x, foot.y, foot.z), (tip.x, tip.y, tip.z), 1.8, 1.25, TOE);
            s.capsule((foot.x, foot.y + 0.5, foot.z), (foot.x + dir.x * 3.8, foot.y + 0.2, foot.z + dir.z * 3.8), 2.1, 1.7, LEGF);
            claw(
                &mut s,
                tip + dir * 0.6 + v(0.0, 0.4, 0.0),
                tip + dir * 3.4 + v(0.0, -2.0, 0.0),
                dir * 0.9 + v(0.0, 1.1, 0.0),
                1.05,
                TALON,
            );
        }
        let back = foot + v(-out * 0.6, -1.1, -5.2);
        s.capsule((foot.x, foot.y, foot.z), (back.x, back.y, back.z), 1.7, 1.2, TOE);
        claw(&mut s, back + v(0.0, 0.3, -0.6), back + v(0.0, -2.1, -3.0), v(0.0, 1.0, -0.6), 0.95, TALON);
    }
    s.paint(|x, y, z, c| (c == TOE && (y + z + x) % 2 == 0).then_some(TOE_D));
    shag(&mut s, (w, h, d), LEGF, LEGF, TAWNY_L, 0.35, 2);
    s.paint(|x, y, z, c| (c == LEGF && (y + (x + z) % 2) % 3 == 0 && nz(x, y, z) > 0.35).then_some(TAWNY_D));

    // ---- the body: a barrel of feathers, the breast paler and streaked, the back barred black
    s.part(0);
    for (c, r) in OWL_BODY {
        ball(&mut s, c, r, OWL);
    }
    s.paint(|x, y, z, c| {
        if c != OWL {
            return None;
        }
        let (fx, fz) = (x as f32 - cx, z as f32 - cz);
        // The breast: whatever faces forward, between the wings; and the flanks under them.
        let breast = fz > 2.0 + fx.abs() * 0.5 && y < 55;
        if breast || (fz > -7.0 && y < 31) {
            // Black streaks down the breast, longest and thickest under the face; fine wavy bars below.
            let col = (x + 70) / 3;
            let lx = (x + 70) % 3;
            let run = ((y as f32 + nz(col, 2, 7) * 9.0) / (5.0 + nz(col, 3, 7) * 3.0)) as i32;
            let on = nz(col, run, 3) > if y > 45 { 0.15 } else { 0.4 };
            let wide = y > 45 && lx != 2;
            let streak = breast && y > 33 && on && (lx == 1 || wide) && ((y as f32 + nz(col, 2, 7) * 9.0) % 6.0) > 1.4;
            let wave = (x as f32 * 0.9 + z as f32 * 0.5).sin() * 0.8;
            let bar = y <= 36 && ((y as f32 + wave).rem_euclid(3.0) < 0.8) && nz(x, y, z) > 0.3;
            return Some(if streak || bar {
                OWL_K
            } else if nz(x, y, z) > 0.86 {
                TAWNY_L
            } else if y < 18 || !breast || fx.abs() > 6.5 || nz(x, y, z) < 0.15 {
                TAWNY_D
            } else {
                BREAST
            });
        }
        // The back: streaked and mottled, not barred like the wings over it.
        Some(if (x + 40 + (nz(x, y / 4, 2) * 2.0) as i32) % 3 == 0 && nz(x, y / 3, z) > 0.45 {
            OWL_K
        } else if nz(x / 2, y / 2, z / 2) < 0.25 {
            OWL_D
        } else if nz(x, y, z) > 0.85 {
            OWL_L
        } else {
            OWL
        })
    });
    // Ragged down hanging off the belly over the legs.
    s.paint(|x, y, z, c| (y < 19 && (c == TAWNY_D || c == BREAST) && nz(x, y, z) < 0.7).then_some(FLUFF));
    shag(&mut s, (w, h, d), FLUFF, TAWNY_D, TAWNY_L, 0.5, 3);
    // A paler throat under the face.
    s.paint(|x, y, z, c| {
        (y >= 49 && (z as f32) > cz + 6.0 && (x as f32 - cx).abs() < 5.0 && c != OWL_K && nz(x, y, z) > 0.35).then_some(DISC_D)
    });
    // The tail, under the crossed wings: a short fan of barred feathers.
    for k in 0..7 {
        let f = k as f32 / 6.0 - 0.5;
        let root = v(cx + f * 5.0, 22.0, cz - 8.0);
        let tip = v(cx + f * 10.5, 8.0 + f.abs() * 2.5, cz - 14.8 + f.abs() * 1.6);
        let normal = Vec3::X.cross((tip - root).normalize());
        let len = (tip - root).length();
        vane(&mut s, root, tip, normal, 3.8, quill, |t, u| {
            if t > 0.95 {
                TAWNY_L
            } else if u.abs() < 0.15 {
                OWL_D
            } else if (t * len / 3.0).fract() < 0.45 {
                TAWNY
            } else {
                TAWNY_D
            }
        });
    }

    // ---- the wings, folded down its sides: the long primaries under the rest and crossed over the
    //      tail, the barred secondaries, the coverts in rows, the shoulder over them all
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        let outside = i as f32;
        // Each feather a shade of its own, so they part from each other.
        let tone = |k: i32, row: i32| 0.86 + nz(k, row, 5 + i as i32) * 0.28;
        for k in 0..6 {
            let f = k as f32 / 5.0;
            let (root, tip) = ((1.0 - f * 7.0, 37.0 - f * 3.0), (-16.0 + f * 3.0 - outside * 1.0, 14.0 + f * 4.5));
            let len = ((tip.0 - root.0).powi(2) + (tip.1 - root.1).powi(2)).sqrt();
            let k2 = tone(k, 0);
            owl_feather(&mut s, side, root, tip, 5.0, (1.0 + outside * 0.4, 4.6), |t, u| {
                let c = if t > 0.92 || u > 0.75 || u.abs() < 0.12 {
                    OWL_K
                } else if (t * len / 3.6 + f * 0.45).fract() < 0.33 {
                    TAWNY
                } else {
                    OWL_D
                };
                scale(c, k2)
            });
        }
        for k in 0..7 {
            let f = k as f32 / 6.0;
            let (root, tip) = ((4.8 - f * 13.6, 46.0 + f * 1.0), (4.6 - f * 15.0, 32.0 - f * 7.0));
            let len = ((tip.0 - root.0).powi(2) + (tip.1 - root.1).powi(2)).sqrt();
            let k2 = tone(k, 1);
            owl_feather(&mut s, side, root, tip, 4.8, (2.0 + (k % 2) as f32 * 0.7, 0.0), |t, u| {
                let c = if t > 0.93 {
                    TAWNY_L
                } else if u > 0.72 {
                    OWL_K
                } else if (t * len / 3.2 + f * 0.5).fract() < 0.38 {
                    TAWNY
                } else {
                    OWL_D
                };
                scale(c, k2)
            });
        }
        for k in 0..8 {
            let f = k as f32 / 7.0;
            let (root, tip) = ((6.6 - f * 15.0, 51.0), (7.2 - f * 16.6, 40.0 - f * 2.5));
            let k2 = tone(k, 2);
            owl_feather(&mut s, side, root, tip, 3.9, (3.4 + (k % 2) as f32 * 0.6, 0.0), |t, u| {
                let c = if t > 0.9 {
                    TAWNY_L
                } else if u > 0.75 || (u.abs() < 0.3 && t < 0.8) {
                    OWL_K
                } else if t > 0.6 && u < -0.2 {
                    TAWNY
                } else {
                    OWL
                };
                scale(c, k2)
            });
        }
        for (row, (n, ytop, ytip, lift)) in [(7, 54.0f32, 44.0f32, 4.4f32), (6, 57.0, 49.0, 5.0)].into_iter().enumerate() {
            for k in 0..n {
                let f = k as f32 / (n - 1) as f32;
                let z0 = if row == 0 { 6.0 - f * 14.0 } else { 4.5 - f * 12.0 };
                let k2 = tone(k, 3 + row as i32);
                owl_feather(&mut s, side, (z0, ytop), (z0 - 0.5, ytip + f * 0.8), 3.2 - row as f32 * 0.3, (lift, 0.0), |t, u| {
                    let c = if t > 0.86 {
                        TAWNY_L
                    } else if u.abs() < 0.35 || u > 0.8 {
                        OWL_K
                    } else if nz(k, row as i32, (t * 9.0) as i32) < 0.3 {
                        TAWNY_D
                    } else {
                        OWL_D
                    };
                    scale(c, k2)
                });
            }
        }
        // The leading edge: small dark feathers down the bend of the wing.
        for k in 0..9 {
            let f = k as f32 / 8.0;
            let (z0, y0) = (2.6 + (f * PI).sin() * 6.4 - f * 1.4, 55.5 - f * 21.0);
            owl_feather(&mut s, side, (z0, y0 + 2.0), (z0 - 0.6, y0 - 2.6), 3.0, (4.0, 0.0), |t, _| {
                if t > 0.85 {
                    TAWNY_D
                } else if nz(k, 7, (t * 6.0) as i32) < 0.4 {
                    OWL_K
                } else {
                    OWL_D
                }
            });
        }
    }

    // ---- the head: round, the pale facial disc rimmed in black, deep-set burning eyes, the hooked
    //      beak, a scowl of dark brows running up into the ear tufts
    s.part(3);
    let (hy, hz) = (64.0f32, cz + 1.0);
    ball(&mut s, v(cx, hy, hz), v(12.8, 10.8, 10.6), OWL);
    s.paint(|x, y, z, c| {
        (c == OWL && y >= 52).then(|| {
            if (x + y / 2) % 3 == 0 && nz(x, y, z) > 0.45 {
                OWL_K
            } else if nz(x / 2, y / 2, z) < 0.3 {
                OWL_D
            } else if nz(x, y, z) > 0.8 {
                TAWNY_D
            } else {
                OWL
            }
        })
    });
    let face_z = (hz + 8.2).round() as i32;
    for y in 50..=78 {
        for z in face_z + 1..d {
            for x in 0..w {
                if s.tag_at(x, y, z) == 3 {
                    s.clear(x, y, z);
                }
            }
        }
    }
    // The disc: two lobes round the eyes, dished in toward them, its rim raised and black.
    let ey = hy + 0.4;
    let eyes = [cx - 5.0, cx + 5.0];
    let lobe = |x: f32, y: f32| eyes.iter().map(|&ex| ((x - ex) / 6.6).powi(2) + ((y - ey + 1.0) / 7.2).powi(2)).fold(f32::MAX, f32::min);
    for y in 50..=76 {
        for x in 6..=41 {
            let q = lobe(x as f32, y as f32);
            let Some(zf) = front(&s, x, y, d) else { continue };
            if s.tag_at(x, y, zf) != 3 || q > 1.32 {
                continue;
            }
            if q > 1.0 {
                s.recolor(x, y, zf, RIM);
                s.set(x, y, zf + 1, if q > 1.2 { OWL_K } else { RIM });
                continue;
            }
            let dig = ((1.0 - q) * 2.4).round() as i32;
            for z in zf - dig + 1..=zf {
                s.clear(x, y, z);
            }
            let a = (y as f32 - ey).atan2(x as f32 - if x < cx as i32 { eyes[0] } else { eyes[1] });
            let ray = (a * 7.0 / PI).rem_euclid(2.0) < 0.35;
            s.recolor(
                x,
                y,
                zf - dig,
                if ray || nz(x, y, 1) < 0.2 {
                    DISC_D
                } else if nz(x, y, 2) > 0.8 {
                    DISC_L
                } else {
                    DISC
                },
            );
        }
    }
    // The eyes: huge, burning orange round black pupils, ringed in shadow, the brow low over them.
    for &ex in &eyes {
        for y in (ey - 5.0) as i32..=(ey + 5.0) as i32 {
            for x in (ex - 5.0) as i32..=(ex + 5.0) as i32 {
                let r = ((x as f32 - ex).powi(2) + (y as f32 - ey).powi(2)).sqrt();
                let Some(zf) = front(&s, x, y, d) else { continue };
                if r <= 3.7 {
                    let pupil = ((x as f32 - ex) / 1.35).powi(2) + ((y as f32 - ey) / 1.9).powi(2) <= 1.0;
                    let col = if pupil {
                        PUPIL_G
                    } else if r < 2.4 {
                        IRIS_L
                    } else if r < 3.2 {
                        IRIS
                    } else {
                        IRIS_D
                    };
                    s.recolor(x, y, zf, col);
                } else if r <= 4.7 {
                    s.recolor(x, y, zf, OWL_K);
                }
            }
        }
        let z = front(&s, (ex - 1.0) as i32, (ey + 1.0) as i32, d).unwrap_or(0);
        s.recolor((ex - 1.0) as i32, (ey + 1.0) as i32, z, GLINT_G);
    }
    // The brows: a black scowl from over the beak up and out into the ear tufts.
    for side in [-1.0f32, 1.0] {
        // (Every point found on the face first: a ball set down would be found in its place.)
        let brow: Vec<(Vec3, f32)> = (0..=30)
            .map(|k| k as f32 / 30.0)
            .filter_map(|t| {
                let (x, y) = (cx + side * (0.6 + t * 9.4), ey + 2.6 + t * 3.0 + t * t * 2.6);
                front(&s, x.round() as i32, y.round() as i32, d).map(|z| (v(x, y, z as f32 + 0.3), t))
            })
            .collect();
        for (k, &(p, t)) in brow.iter().enumerate() {
            ball(&mut s, p, v(1.2, 1.0 + (1.0 - t) * 0.5, 1.0), if k % 6 == 0 { OWL_D } else { OWL_K });
        }
        // A pale edge over the black, so the scowl reads.
        for &(p, t) in brow.iter().filter(|(_, t)| *t < 0.75) {
            put(&mut s, p + v(0.0, 1.6 + (1.0 - t) * 0.4, -0.4), if t < 0.4 { DISC_L } else { TAWNY_L });
        }
        // The ear tufts: a bundle of black feathers each side, standing up and out like horns.
        for (j, spread) in [-1.0f32, 0.0, 1.0].into_iter().enumerate() {
            let root = v(cx + side * (7.4 + spread * 1.6), hy + 7.0 - spread.abs() * 0.6, hz + 1.8 - spread * 0.8);
            let ctrl = v(cx + side * (9.0 + spread * 1.5), hy + 14.0, hz + 1.2 - spread * 0.9);
            let tip = v(cx + side * (12.6 + spread * 1.1), hy + 20.0 - spread.abs() * 2.4 + spread * side * 0.0, hz - 1.4 - spread * 1.0);
            let knots: Vec<(Vec3, f32)> =
                (0..=10).map(|k| k as f32 / 10.0).map(|t| (bezier([root, ctrl, tip], t), 1.9 * (1.0 - t) + 0.35)).collect();
            sweep(&mut s, &knots, |p, c, along| {
                if along > 0.78 || (p.x - c.x) * side > 0.7 {
                    OWL_K
                } else if j == 0 && (p.x - c.x) * side < -0.5 {
                    TAWNY_D
                } else if nz(p.x as i32, p.y as i32, p.z as i32) < 0.3 {
                    OWL_D
                } else {
                    RIM
                }
            });
        }
    }
    // The beak: hooked and black out of the bristles between the eyes, the cere paler.
    let bz = front(&s, cx as i32, (ey - 2.0) as i32, d).unwrap_or(30) as f32;
    let curve = [v(cx, ey - 1.4, bz - 0.5), v(cx, ey - 2.4, bz + 4.8), v(cx, ey - 7.4, bz + 2.8)];
    for k in 0..=40 {
        let t = k as f32 / 40.0;
        let c = bezier(curve, t);
        let (rx, ry) = (1.6 - t * 1.0, 2.1 - t * 1.5);
        for y in (c.y - ry).floor() as i32..=(c.y + ry).ceil() as i32 {
            for x in (c.x - rx).floor() as i32..=(c.x + rx).ceil() as i32 {
                if ((x as f32 - c.x) / rx).powi(2) + ((y as f32 - c.y) / ry).powi(2) <= 1.0 {
                    s.set(x, y, c.z.round() as i32, if t < 0.22 { BEAK_L } else { BEAK });
                }
            }
        }
    }
    for side in [-1.0f32, 1.0] {
        for k in 0..5 {
            let a = v(cx + side * 1.3, ey - 0.6 - k as f32 * 1.3, bz + 0.6);
            let b = a + v(side * (2.8 - k as f32 * 0.2), -1.0 - k as f32 * 0.3, 0.8);
            s.capsule((a.x, a.y, a.z), (b.x, b.y, b.z), 0.6, 0.35, if k % 2 == 0 { DISC_L } else { DISC_D });
        }
    }
    s.finish(0.04)
}

// ------------------------------------------------------------------- albastı

/// The albastı's grid: 48 wide, 116 tall, 54 deep (she hunches far forward over her feet).
pub const ALBASTI_SIZE: (i32, i32, i32) = (48, 116, 54);
const ALBASTI_Z: f32 = 26.5;

/// Hips, shoulders and the base of her neck.
pub const ALBASTI_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [18.0, 53.0, 23.5] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [29.0, 53.0, 23.5] },
    PartDef { name: "leftArm", boxes: &[], pivot: [12.0, 92.0, 30.5] },
    PartDef { name: "rightArm", boxes: &[], pivot: [35.0, 92.0, 30.5] },
    PartDef { name: "head", boxes: &[], pivot: [23.5, 96.0, 33.0] },
];

/// Her trunk from the hips up: (y, its middle's z from the grid's, half its width, its depth before and behind it).
const ALBASTI_TRUNK: [(f32, f32, f32, f32, f32); 12] = [
    (46.0, -2.4, 7.4, 4.6, 5.2),
    (51.0, -3.0, 8.6, 5.6, 5.8),
    (56.0, -3.6, 8.0, 4.8, 5.6),
    (62.0, -4.0, 6.2, 3.4, 5.0),
    (68.0, -4.0, 7.0, 4.4, 5.4),
    (74.0, -3.6, 8.8, 6.2, 6.0),
    (80.0, -2.6, 9.6, 6.6, 6.9),
    (86.0, -0.6, 10.0, 6.2, 8.0),
    (91.0, 2.0, 10.6, 5.4, 8.6),
    (95.0, 4.6, 9.0, 4.6, 7.6),
    (98.0, 6.8, 6.2, 3.6, 5.8),
    (100.0, 8.2, 3.4, 2.4, 3.2),
];

/// Her trunk's slice at height `y`, between the rows of `ALBASTI_TRUNK`: (z offset, half width, front, back).
fn albasti_trunk(y: f32) -> (f32, f32, f32, f32) {
    let rows = &ALBASTI_TRUNK;
    let at = rows.windows(2).position(|w| y <= w[1].0).unwrap_or(rows.len() - 2);
    let (a, b) = (rows[at], rows[at + 1]);
    let k = ((y - a.0) / (b.0 - a.0)).clamp(0.0, 1.0);
    (a.1 + (b.1 - a.1) * k, a.2 + (b.2 - a.2) * k, a.3 + (b.3 - a.3) * k, a.4 + (b.4 - a.4) * k)
}

/// Her skin about a limb's axis: darker underneath and in mottles, the bone catching the light on top.
fn albasti_skin(p: Vec3, c: Vec3) -> u32 {
    let n = nz(p.x as i32, p.y as i32, p.z as i32);
    if p.y < c.y - 0.8 || n < 0.14 {
        AL_SKIN_D
    } else if n > 0.9 {
        AL_SKIN_L
    } else {
        AL_SKIN
    }
}

/**
 * The albastı: a night hag, gaunt and hunched, her skin the dark blue-grey of a drowned thing and
 * her ribs standing out of it; her head thrust out before her, long black hair hanging from it in
 * tangled locks down her back and over her face, eyes burning red in deep sockets, a mouth too
 * wide for her face full of needle teeth, pointed ears; arms so long their black claws reach
 * almost to the ground, bony knees bent, bare clawed feet, and rags about her hips, dark, with the
 * red of her name in them.
 */
pub fn albasti() -> Collected {
    let (w, h, d) = ALBASTI_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (23.5f32, ALBASTI_Z);

    // ---- legs: thin and crooked, the knees bent forward and bony, long clawed feet splayed
    for (i, lx) in [cx - 5.5, cx + 5.5].into_iter().enumerate() {
        s.part(1 + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        let hip = v(lx, 53.0, cz - 3.0);
        let knee = v(lx + out * 1.6, 31.0, cz + 6.0);
        let ankle = v(lx + out * 1.8, 5.5, cz - 1.0);
        sweep(&mut s, &[(hip, 4.4), (v(lx + out * 1.0, 42.0, cz + 2.0), 3.8), (knee, 2.9)], |p, c, _| albasti_skin(p, c));
        sweep(&mut s, &[(knee, 2.8), (v(lx + out * 1.8, 21.0, cz + 3.4), 2.7), (ankle, 1.9)], |p, c, _| albasti_skin(p, c));
        ball(&mut s, knee + v(0.0, 0.2, 1.7), v(2.3, 2.8, 1.7), AL_SKIN_L);
        ball(&mut s, knee + v(0.0, -2.6, 1.6), v(1.2, 1.2, 1.1), AL_SKIN);
        // The shin bone's edge down the front.
        s.capsule((knee.x, knee.y - 3.0, knee.z + 1.8), (ankle.x, ankle.y + 3.0, ankle.z + 1.5), 0.7, 0.6, AL_SKIN_L);
        for side in [-1.0f32, 1.0] {
            ball(&mut s, ankle + v(side * 1.7, 0.6, 0.0), Vec3::splat(1.1), AL_SKIN_L);
        }
        // The foot: a bony heel, a long instep, four long toes splayed out, each with its black claw.
        let heel = v(ankle.x, 2.2, ankle.z - 2.0);
        let toes = v(ankle.x + out * 0.8, 2.0, ankle.z + 5.2);
        s.capsule((heel.x, heel.y, heel.z), (toes.x, toes.y, toes.z), 2.3, 2.0, AL_SKIN);
        s.capsule((ankle.x, ankle.y, ankle.z), (toes.x, toes.y + 0.6, toes.z - 1.0), 1.8, 1.6, AL_SKIN);
        for (k, a) in [-0.5f32, -0.16, 0.18, 0.52].into_iter().enumerate() {
            let ang = a + out * 0.3;
            let dir = v(ang.sin(), 0.0, ang.cos());
            let len = 5.2 - (k as f32 - 1.4).abs() * 0.8;
            let tip = toes + dir * len + v(0.0, -0.7, 0.0);
            s.capsule((toes.x, toes.y, toes.z), (tip.x, tip.y, tip.z), 1.15, 0.8, AL_SKIN);
            put(&mut s, toes + dir * (len * 0.5) + v(0.0, 0.9, 0.0), AL_SKIN_L);
            claw(
                &mut s,
                tip + dir * 0.5 + v(0.0, 0.3, 0.0),
                tip + dir * 3.0 + v(0.0, -1.5, 0.0),
                dir * 0.6 + v(0.0, 0.8, 0.0),
                0.8,
                AL_CLAW,
            );
        }
    }

    // ---- the trunk: hips, a sunken belly, the ribcage, the hump of her shoulders
    s.part(0);
    for y in 46..=100 {
        let (zo, rx, rf, rb) = albasti_trunk(y as f32);
        let zc = cz + zo;
        for z in (zc - rb - 1.0) as i32..=(zc + rf + 1.0) as i32 {
            for x in (cx - rx - 1.0) as i32..=(cx + rx + 1.0) as i32 {
                let dz = z as f32 - zc;
                let r = if dz > 0.0 { rf } else { rb };
                if ((x as f32 - cx) / rx).powi(2) + (dz / r).powi(2) <= 1.0 {
                    s.set(x, y, z, if nz(x, y, z) < 0.12 { AL_SKIN_D } else { AL_SKIN });
                }
            }
        }
    }
    // The hollow of her belly under the ribs.
    s.paint(|x, y, z, c| {
        ((58..=70).contains(&y) && (z as f32) > cz - 2.0 && (x as f32 - cx).abs() < 4.5 && c == AL_SKIN).then_some(AL_SKIN_D)
    });
    // The ribs, each curving down from her spine round to the breastbone, standing out of her skin.
    for k in 0..6 {
        let base = 72.0 + k as f32 * 3.0;
        for j in -40..=40 {
            let th = j as f32 / 40.0 * 1.95;
            if th.abs() < if k < 2 { 0.55 } else { 0.22 } {
                continue;
            }
            let y = base + 2.6 * (1.0 - th.cos());
            let (zo, rx, rf, rb) = albasti_trunk(y);
            let r = if th.cos() > 0.0 { rf } else { rb };
            let p = v(cx + rx * th.sin(), y, cz + zo + r * th.cos());
            ball(&mut s, p, v(0.8, 0.75, 0.8), if j % 7 == 0 { AL_SKIN } else { AL_SKIN_L });
        }
    }
    // The breastbone, the collarbones, the knobs of her spine and the blades of her shoulders.
    for y in 75..=90 {
        let (zo, _, rf, _) = albasti_trunk(y as f32);
        put(&mut s, v(cx, y as f32, cz + zo + rf + 0.2), AL_SKIN_L);
    }
    for side in [-1.0f32, 1.0] {
        let (zo, _, rf, _) = albasti_trunk(93.0);
        s.capsule((cx + side * 1.2, 92.6, cz + zo + rf - 0.2), (cx + side * 9.4, 94.0, cz + zo + 1.0), 0.9, 0.8, AL_SKIN_L);
        let (zo, _, _, rb) = albasti_trunk(88.0);
        ball(&mut s, v(cx + side * 5.0, 88.0, cz + zo - rb + 0.6), v(3.0, 4.6, 1.4), AL_SKIN);
        s.capsule((cx + side * 2.6, 91.5, cz + zo - rb - 0.4), (cx + side * 7.4, 85.0, cz + zo - rb + 0.2), 0.6, 0.6, AL_SKIN_L);
    }
    for y in (58..=98).step_by(3) {
        let (zo, _, _, rb) = albasti_trunk(y as f32);
        ball(&mut s, v(cx, y as f32, cz + zo - rb - 0.1), v(1.1, 0.9, 1.0), AL_SKIN_L);
    }

    // ---- the rags: a deep red cloth wrapped about her hips under a dark one torn into strips,
    //      the red showing between them and below, a knot of it at her left hip
    let (hzo, hrx, hrf, hrb) = albasti_trunk(52.0);
    for (layer, (grow, top, low)) in [(0.9f32, 55.0f32, 36.0f32), (2.2, 57.0, 39.0)].into_iter().enumerate() {
        for y in (low - 4.0) as i32..=top as i32 + 2 {
            for z in 0..d {
                for x in 0..w {
                    let flare = (52.0 - y as f32).max(0.0) * 0.16;
                    let (dx, dz) = (x as f32 - cx, z as f32 - (cz + hzo));
                    let (ox, of, ob) = (hrx + grow + flare, hrf + grow + 0.4 + flare, hrb + grow + flare);
                    let q = (dx / ox).powi(2) + (dz / if dz > 0.0 { of } else { ob }).powi(2);
                    let qi = (dx / (ox - 1.5)).powi(2) + (dz / if dz > 0.0 { of - 1.5 } else { ob - 1.5 }).powi(2);
                    if q > 1.0 || qi < 1.0 {
                        continue;
                    }
                    let th = dx.atan2(dz);
                    let a = (th + PI) / TAU * if layer == 0 { 11.0 } else { 17.0 };
                    let (strip, within) = (a.floor() as i32, a.fract());
                    // The hem: lowest before and behind, ragged, every strip its own length.
                    let hem = low - th.cos().abs() * 5.0 + nz(strip, 1 + layer as i32, 7) * 6.0 - 1.0;
                    if (y as f32) < hem || y as f32 > top + nz(x, 2, z) * 2.0 {
                        continue;
                    }
                    if layer == 1 && ((y as f32) < 50.0 && within < 0.16 || nz(strip, 5, 3) < 0.12 && (y as f32) < 49.0) {
                        continue;
                    }
                    if nz(x, y / 2, z) < 0.06 && (y as f32) < top - 3.0 {
                        continue;
                    }
                    let fold = (th * if layer == 0 { 7.0 } else { 11.0 } + y as f32 * 0.12).sin();
                    let c = match (layer, (y as f32) < hem + 1.5) {
                        (0, true) => AL_RED_D,
                        (0, false) if fold > 0.6 => AL_RED_D,
                        (0, false) if nz(x, y, z) > 0.86 => AL_RED_L,
                        (0, false) => AL_RED,
                        (_, true) => AL_RAG_L,
                        _ if fold > 0.55 || within < 0.25 => AL_RAG_D,
                        _ if nz(x, y, z) > 0.88 => AL_RAG_L,
                        _ => AL_RAG,
                    };
                    s.set(x, y, z, c);
                }
            }
        }
    }
    let knot = v(cx - hrx - 2.4, 54.0, cz + hzo + 2.6);
    ball(&mut s, knot, v(1.9, 2.1, 2.1), AL_RED);
    for (dx, dz, len) in [(-0.9f32, 1.0f32, 15.0f32), (0.4, -0.6, 11.0)] {
        for k in 0..(len as i32) {
            let y = knot.y - 1.0 - k as f32;
            let sway = (k as f32 * 0.5).sin() * 0.5;
            let (x, z) = ((knot.x + dx * (k as f32 * 0.12) + sway).round() as i32, (knot.z + dz * (k as f32 * 0.15)).round() as i32);
            s.cuboid(x, y as i32, z - 1, x, y as i32, z, if k % 4 == 3 || k + 1 == len as i32 { AL_RED_D } else { AL_RED });
        }
    }

    // ---- arms: long past all reason, thin and sinewy, bony at elbow and wrist, the hands hanging
    //      before her knees, long fingers curling, black claws all but scraping the ground
    for (i, ax) in [cx - 11.5, cx + 11.5].into_iter().enumerate() {
        s.part(3 + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        let sh = v(ax, 92.0, cz + 4.0);
        let elbow = v(ax + out * 3.8, 64.0, cz + 3.6);
        let wrist = v(ax + out * 3.0, 33.0, cz + 12.0);
        ball(&mut s, sh + v(out * 0.4, 0.4, 0.0), v(3.6, 3.6, 3.8), AL_SKIN);
        sweep(&mut s, &[(sh, 3.2), (v(ax + out * 1.6, 78.0, cz + 4.8), 2.8), (elbow, 2.3)], |p, c, _| albasti_skin(p, c));
        ball(&mut s, elbow + v(0.0, 0.0, -1.3), v(1.9, 2.2, 1.8), AL_SKIN_L);
        sweep(&mut s, &[(elbow, 2.4), (v(ax + out * 3.0, 50.0, cz + 8.2), 2.4), (wrist, 1.7)], |p, c, _| albasti_skin(p, c));
        ball(&mut s, wrist + v(out * 0.9, 0.0, -0.3), Vec3::splat(1.2), AL_SKIN_L);
        // The hand, its back to the front, the long fingers spread and curling forward, so that
        // the claws reach out at whatever is before her.
        let knuckles = wrist + v(0.0, -6.2, 1.2);
        rounded(&mut s, (wrist + knuckles) * 0.5, v(2.6, 3.6, 1.4), 2.4, 0, AL_SKIN);
        for f in 0..4 {
            let fx = (f as f32 - 1.5) * 1.3 * out;
            let long = 1.0 - (f as f32 - 1.6).abs() * 0.12;
            let k0 = knuckles + v(fx, 0.0, 0.0);
            let j1 = k0 + v(fx * 0.2, -4.4 * long, 0.9);
            let j2 = j1 + v(fx * 0.1, -3.4 * long, 1.5);
            let tip = j2 + v(0.0, -2.2 * long, 1.8);
            limb(&mut s, &[(k0, 0.95), (j1, 0.8), (j2, 0.7), (tip, 0.6)], AL_SKIN);
            put(&mut s, k0 + v(0.0, 0.0, 0.9), AL_SKIN_L);
            put(&mut s, j1 + v(0.0, 0.0, 0.8), AL_SKIN_L);
            put(&mut s, j2 + v(0.0, 0.0, 0.8), AL_SKIN_L);
            claw(&mut s, tip + v(0.0, -0.3, 0.4), tip + v(fx * 0.15, -5.6, 3.2), v(0.0, 0.0, 1.4), 0.95, AL_CLAW);
        }
        let th0 = wrist + v(-out * 2.2, -3.0, 1.6);
        let th1 = th0 + v(-out * 0.9, -3.0, 1.4);
        let th2 = th1 + v(-out * 0.3, -2.6, 1.2);
        limb(&mut s, &[(th0, 1.0), (th1, 0.85), (th2, 0.65)], AL_SKIN);
        claw(&mut s, th2 + v(0.0, -0.3, 0.4), th2 + v(0.0, -4.4, 2.4), v(0.0, 0.0, 1.0), 0.9, AL_CLAW);
    }
    s.paint(|x, y, z, c| (c == AL_CLAW && (x + y + z) % 5 == 0).then_some(AL_CLAW_L));

    // ---- the head, thrust out before her on a scrawny neck: gaunt, a heavy brow over deep sockets
    //      and burning eyes, sunken cheeks, the mouth wide with needle teeth, pointed ears
    s.part(5);
    let hd = v(cx, 101.0, cz + 17.0);
    sweep(&mut s, &[(v(cx, 95.0, cz + 6.0), 3.2), (v(cx, 98.0, cz + 10.5), 2.7), (hd + v(0.0, -2.6, -3.6), 2.7)], |p, c, _| {
        albasti_skin(p, c)
    });
    for side in [-1.0f32, 1.0] {
        s.capsule((cx + side * 1.4, 95.6, cz + 8.4), (cx + side * 1.2, 98.6, cz + 13.6), 0.7, 0.6, AL_SKIN_L);
    }
    ball(&mut s, hd + v(0.0, 1.6, -1.0), v(6.0, 6.8, 7.2), AL_SKIN);
    rounded(&mut s, hd + v(0.0, -3.2, 2.6), v(4.7, 4.0, 4.6), 2.4, 0, AL_SKIN);
    ball(&mut s, hd + v(0.0, -7.0, 5.0), v(2.0, 1.8, 1.9), AL_SKIN);
    put(&mut s, hd + v(0.0, -7.6, 6.6), AL_SKIN_L);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, hd + v(side * 3.9, -0.3, 4.6), v(1.7, 1.3, 1.7), AL_SKIN_L);
        // The hollow under each cheekbone.
        for y in -5..=-2 {
            for z in 3..=7 {
                let (x, yy, zz) = ((hd.x + side * 4.4).round() as i32, (hd.y + y as f32).round() as i32, (hd.z + z as f32).round() as i32);
                if open(&s, x, yy, zz) {
                    s.recolor(x, yy, zz, AL_SKIN_DD);
                }
            }
        }
    }
    s.capsule((hd.x - 4.5, hd.y + 2.8, hd.z + 5.4), (hd.x + 4.5, hd.y + 2.8, hd.z + 5.4), 1.3, 1.3, AL_SKIN_D);
    for x in [hd.x - 3.0, hd.x + 3.0] {
        put(&mut s, v(x, hd.y + 3.6, hd.z + 6.2), AL_SKIN_L);
    }
    // The sockets, deep under the brow, and the eyes burning at the bottom of them.
    let fz = (hd.z + 6.8).round() as i32;
    for side in [-1.0f32, 1.0] {
        let ex = hd.x + side * 2.5;
        for y in 0..=2 {
            for x in -1..=1 {
                let (xx, yy) = ((ex + x as f32).round() as i32, (hd.y + y as f32).round() as i32);
                for z in fz - 2..=fz + 2 {
                    s.clear(xx, yy, z);
                }
                s.set(xx, yy, fz - 3, AL_SKIN_DD);
            }
        }
        let (xi, xo) = ((ex - side * 0.5).round() as i32, (ex + side * 0.5).round() as i32);
        let y = (hd.y + 1.0).round() as i32;
        s.set(xi, y, fz - 2, AL_EYE_L);
        s.set(xo, y, fz - 2, AL_EYE);
        s.set(xi, y - 1, fz - 2, AL_EYE);
        s.set(xo, y + 1, fz - 3, AL_EYE);
    }
    // A thin hooked nose and its nostrils.
    s.capsule((hd.x, hd.y + 1.6, hd.z + 6.6), (hd.x, hd.y - 2.0, hd.z + 8.0), 0.9, 1.0, AL_SKIN);
    put(&mut s, v(hd.x, hd.y + 0.4, hd.z + 7.8), AL_SKIN_L);
    for x in [hd.x - 0.5, hd.x + 0.5] {
        put(&mut s, v(x, hd.y - 2.4, hd.z + 7.6), AL_SKIN_DD);
    }
    // The mouth: a slit from cheek to cheek, turned up at its ends, needle teeth meeting across it.
    for j in -9i32..=9 {
        let x = (hd.x + j as f32 * 0.5).round() as i32;
        let lift = (j as f32 / 9.0).powi(4) * 2.2;
        let y = (hd.y - 4.6 + lift).round() as i32;
        let Some(zf) = front(&s, x, y, d) else { continue };
        if (zf as f32) < hd.z + 3.0 {
            continue;
        }
        for yy in y - 2..=y {
            for z in zf - 2..=zf {
                s.clear(x, yy, z);
            }
            s.set(x, yy, zf - 2, AL_MOUTH);
        }
        if j.abs() < 9 {
            if x % 2 == 0 {
                s.set(x, y, zf - 1, AL_TOOTH);
                s.set(x, y - 1, zf - 1, AL_TOOTH);
            } else {
                s.set(x, y - 2, zf - 1, AL_TOOTH);
            }
            s.recolor(x, y + 1, zf, AL_LIP);
            s.recolor(x, y - 3, zf, AL_LIP);
        }
    }
    // Pointed ears, swept back, poking out of her hair.
    for side in [-1.0f32, 1.0] {
        s.capsule((hd.x + side * 5.4, hd.y + 0.4, hd.z - 0.4), (hd.x + side * 9.6, hd.y + 3.8, hd.z - 3.2), 1.4, 0.3, AL_SKIN);
        put(&mut s, v(hd.x + side * 6.6, hd.y + 0.9, hd.z - 0.4), AL_SKIN_DD);
    }
    // The hair: a cap over her crown, hanging in heavy tangled locks down off her head (which is
    // to say before her, she leans so far), over her face, and down her back onto the hump.
    let crown = hd + v(0.0, 1.8, -1.6);
    for y in (hd.y - 6.0) as i32..=(hd.y + 10.0) as i32 {
        for z in (crown.z - 9.0) as i32..=(crown.z + 9.0) as i32 {
            for x in (crown.x - 8.0) as i32..=(crown.x + 8.0) as i32 {
                let p = v(x as f32, y as f32, z as f32) - crown;
                if (p.x / 6.8).powi(2) + (p.y / 7.6).powi(2) + (p.z / 7.9).powi(2) > 1.0 || s.get(x, y, z).is_some() {
                    continue;
                }
                if z as f32 - hd.z > 2.4 + 0.55 * (y as f32 - hd.y) {
                    continue;
                }
                s.set(x, y, z, if nz(x, y, z) < 0.25 { AL_HAIR_L } else { AL_HAIR });
            }
        }
    }
    let hair = |k: i32| {
        move |t: f32| {
            if nz(k, (t * 7.0) as i32, 3) < 0.18 {
                AL_HAIR_L
            } else if nz(k, 1, 4) < 0.1 {
                AL_HAIR_B
            } else {
                AL_HAIR
            }
        }
    };
    // Locks of four or five strands each, all the way round from one temple behind to the other.
    for lock in 0..18 {
        let a = PI * (0.03 + 0.94 * lock as f32 / 17.0);
        let (ca, sa) = (a.cos(), a.sin());
        let len = 30.0 + nz(lock, 3, 3) * 18.0 - (1.0 - sa) * 6.0;
        for k in 0..5 {
            let jit = v(nz(lock, k, 8) - 0.5, nz(lock, k, 9) - 0.5, nz(lock, k, 10) - 0.5) * 1.0;
            let root = crown + v(ca * 6.6, 2.0 - sa * 2.0, -sa * 7.4) + jit;
            let off = v(ca, 0.1, -sa).normalize_or_zero();
            let seed = lock * 8 + if k == 4 { 7 } else { 0 };
            drape(&mut s, root, v(ca * 0.5, 0.1, -sa * 0.7), len - k as f32 * 1.5, off, seed, hair(lock * 8 + k));
        }
    }
    // Locks from her brow, falling past her face and over it.
    for (lock, (x, len)) in [(-4.0f32, 28.0f32), (-4.8, 24.0), (4.0, 30.0), (4.9, 22.0)].into_iter().enumerate() {
        for k in 0..4 {
            let root = hd + v(x + (k as f32 - 1.5) * 0.5, 6.2 - (k % 2) as f32 * 0.6, 4.8);
            drape(
                &mut s,
                root,
                v(x * 0.05, -0.6, 0.45),
                len - k as f32 * 2.0,
                v(0.0, 0.0, 1.0),
                300 + lock as i32 * 8,
                hair(300 + lock as i32 * 8 + k),
            );
        }
    }
    s.finish(0.04)
}

// ------------------------------------------------------------------------ cin

/// The cin's grid: 44 wide, 122 tall (the fire on his staff), 40 deep.
pub const CIN_SIZE: (i32, i32, i32) = (44, 122, 40);
const CIN_Z: f32 = 19.5;
/// The heart of the smokeless fire at his staff's tip, in his grid's voxels (x, y, z), at rest.
pub const CIN_FLAME: [f32; 3] = [8.0, 111.0, 28.0];

/// Hips (his feet under the robe), shoulders and neck; the staff goes with the left hand.
pub const CIN_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [17.5, 44.0, 19.5] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [25.5, 44.0, 19.5] },
    PartDef { name: "leftArm", boxes: &[], pivot: [10.5, 82.0, 19.5] },
    PartDef { name: "rightArm", boxes: &[], pivot: [32.5, 82.0, 19.5] },
    PartDef { name: "head", boxes: &[], pivot: [21.5, 84.0, 20.5] },
];

/// The robe's slice at height `y`: (its middle's z from the grid's, half its width, half its depth).
fn cin_robe(y: f32) -> (f32, f32, f32) {
    const ROWS: [(f32, f32, f32, f32); 7] = [
        (0.0, -0.6, 12.6, 10.6),
        (20.0, -0.4, 11.0, 9.0),
        (44.0, 0.0, 9.4, 7.4),
        (56.0, 0.2, 8.4, 6.6),
        (70.0, 0.6, 9.6, 7.0),
        (80.0, 1.0, 11.0, 7.4),
        (86.0, 1.2, 6.6, 5.4),
    ];
    let at = ROWS.windows(2).position(|w| y <= w[1].0).unwrap_or(ROWS.len() - 2);
    let (a, b) = (ROWS[at], ROWS[at + 1]);
    let k = ((y - a.0) / (b.0 - a.0)).clamp(0.0, 1.0);
    (a.1 + (b.1 - a.1) * k, a.2 + (b.2 - a.2) * k, a.3 + (b.3 - a.3) * k)
}

/**
 * A flame from `base` up: a teardrop `height` tall and `r` across at its widest, licking to one
 * side near its tip; white-gold at its heart, gold, then pale orange at its edges.
 */
fn flame(s: &mut Sculpt, base: Vec3, height: f32, r: f32) {
    for y in base.y.floor() as i32..=(base.y + height).ceil() as i32 {
        let t = (y as f32 - base.y) / height;
        if !(0.0..=1.0).contains(&t) {
            continue;
        }
        // (Clamped: a hair below zero at the tip would make the powers NaN.)
        let rad = r * (t / 0.3).min(1.0).sqrt() * (1.0 - t).max(0.0).powf(0.75) + 0.3;
        let lean = (t * PI * 1.5).sin() * 0.8 * t;
        let c = base + v(lean, 0.0, -lean * 0.5);
        for z in (c.z - rad - 1.0).floor() as i32..=(c.z + rad + 1.0).ceil() as i32 {
            for x in (c.x - rad - 1.0).floor() as i32..=(c.x + rad + 1.0).ceil() as i32 {
                let q = ((x as f32 - c.x).powi(2) + (z as f32 - c.z).powi(2)).sqrt() / rad;
                if q > 1.0 {
                    continue;
                }
                let col = if q < 0.45 && t < 0.7 {
                    FLAME_CORE
                } else if q < 0.8 && t < 0.85 {
                    FLAME_GOLD
                } else {
                    FLAME_EDGE
                };
                s.set(x, y, z, col);
            }
        }
    }
}

/**
 * The cin: a djinn of smokeless fire in the shape of a tall, gaunt old man, glaring; a hood of
 * deep umber over wild white hair, bushy white brows over eyes that glow faintly white-gold, a
 * long hooked nose and a long white beard down to his sash; layered robes, umber over smoke-grey,
 * a grey shawl, wide sleeves; a crooked staff in his left hand with smokeless fire cradled at
 * its tip, his right hand open to cast; wisps of pale smoke curling round his ragged hem.
 */
pub fn cin() -> Collected {
    let (w, h, d) = CIN_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (21.5f32, CIN_Z);

    // ---- feet (the legs, swung from the hip, are lost in the robe): pointed slippers with curled
    //      toes peeping from under his hem
    for (i, lx) in [cx - 4.0, cx + 4.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        s.capsule((lx, 9.0, cz), (lx, 2.5, cz - 0.2), 1.8, 1.9, CIN_SLIPPER);
        s.capsule((lx, 1.6, cz - 1.2), (lx + out * 0.6, 1.4, cz + 8.0), 2.3, 1.3, CIN_SLIPPER);
        s.capsule((lx + out * 0.6, 1.4, cz + 8.0), (lx + out * 0.7, 3.6, cz + 9.6), 1.1, 0.5, CIN_SLIPPER);
        s.paint(|x, y, z, c| (c == CIN_SLIPPER && y <= 1 && (x + z) % 3 == 0).then_some(CIN_ROBE_D));
    }

    // ---- the robe: umber over smoke-grey, open below the sash, folded, ragged at the hem
    s.part(0);
    for y in 0..=86 {
        let (zo, rx, rz) = cin_robe(y as f32);
        let zc = cz + zo;
        for z in (zc - rz - 2.0) as i32..=(zc + rz + 2.0) as i32 {
            for x in (cx - rx - 2.0) as i32..=(cx + rx + 2.0) as i32 {
                let (dx, dz) = (x as f32 - cx, z as f32 - zc);
                let th = dz.atan2(dx);
                let fold = (th * 9.0 + y as f32 * 0.05).sin();
                let swell = if y < 54 { 1.0 + 0.05 * fold } else { 1.0 };
                if (dx / (rx * swell)).powi(2) + (dz / (rz * swell)).powi(2) > 1.0 {
                    continue;
                }
                if y < 5 && nz(x, 1, z) < 0.18 + (5 - y) as f32 * 0.1 {
                    continue;
                }
                let open = 1.4 + (54 - y).max(0) as f32 * 0.09;
                let c = if y < 54 && dz > rz * 0.55 && dx.abs() < open {
                    if dx.abs() > open - 1.0 {
                        CIN_ROBE_D
                    } else if fold > 0.5 {
                        CIN_UNDER_D
                    } else {
                        CIN_UNDER
                    }
                } else if y < 3 || fold > 0.62 {
                    CIN_ROBE_D
                } else if fold < -0.8 || nz(x, y, z) > 0.9 {
                    CIN_ROBE_L
                } else {
                    CIN_ROBE
                };
                s.set(x, y, z, c);
            }
        }
    }
    // The sash, knotted at his right with its end hanging.
    for y in 54..=57 {
        let (zo, rx, rz) = cin_robe(y as f32);
        column(&mut s, cx, cz + zo, rx + 0.8, rz + 0.8, y, y, CIN_SASH);
    }
    let (zo, rx, rz) = cin_robe(55.0);
    let knot = v(cx + rx * 0.55, 55.5, cz + zo + rz * 0.85 + 0.6);
    ball(&mut s, knot, v(1.6, 1.8, 1.4), CIN_SASH);
    s.capsule((knot.x, knot.y - 1.0, knot.z + 0.3), (knot.x + 1.0, 42.0, knot.z + 1.2), 1.0, 0.8, CIN_SASH);
    s.capsule((knot.x - 0.6, knot.y - 1.0, knot.z + 0.2), (knot.x - 1.4, 45.0, knot.z + 1.0), 0.9, 0.7, CIN_SASH);
    // The shawl: smoke-grey over his shoulders, hanging in a ragged edge before and behind.
    for y in 56..=88 {
        for z in 0..d {
            for x in 0..w {
                let (zo, rx, rz) = cin_robe((y as f32).min(85.0));
                let (dx, dz) = (x as f32 - cx, z as f32 - (cz + zo));
                let q = (dx / (rx + 1.6)).powi(2) + (dz / (rz + 1.6)).powi(2);
                let qi = (dx / (rx - 0.2)).powi(2) + (dz / (rz - 0.2)).powi(2);
                if q > 1.0 || (qi < 1.0 && y < 86) || s.get(x, y, z).is_some_and(|c| c == CIN_SASH) {
                    continue;
                }
                let th = dz.atan2(dx);
                let (front, side) = (th.sin(), th.cos().abs());
                if front > 0.5 && dx.abs() < 3.4 {
                    continue;
                }
                let bottom = 62.0 + side * 8.0 - front.min(0.0) * 2.0 + nz((th * 12.0) as i32, 3, 4) * 3.0;
                if (y as f32) < bottom {
                    continue;
                }
                let c = if (y as f32) < bottom + 1.0 || (th * 7.0 + y as f32 * 0.1).sin() > 0.65 {
                    CIN_SHAWL_D
                } else if nz(x, y, z) > 0.88 {
                    CIN_SHAWL_L
                } else {
                    CIN_SHAWL
                };
                s.set(x, y, z, c);
            }
        }
    }
    // Wisps of smoke curling round his hem: short curls in puffs, rising and thinning.
    for k in 0..7 {
        let a0 = nz(k, 1, 9) * TAU;
        let y0 = 0.8 + nz(k, 2, 9) * 14.0;
        let n = 60;
        for j in 0..=n {
            let t = j as f32 / n as f32;
            if nz(k, j / 6, 3) < 0.3 {
                continue;
            }
            let y = y0 + t * 11.0 + (t * PI).sin() * 2.0;
            let (zo, rx, rz) = cin_robe(y);
            let a = a0 + t * TAU * 0.55 * if k % 2 == 0 { 1.0 } else { -1.0 };
            // Out from the robe and curling back in at the end, like a lock of smoke.
            let swirl = 2.4 + (t * PI).sin() * 3.0 - t * t * 1.6;
            let p = v(cx + (rx + swirl) * a.cos(), y, cz + zo + (rz + swirl) * a.sin());
            let r = 1.9 * (1.0 - t * 0.7) * (0.55 + 0.45 * (t * 13.0 + k as f32).sin().abs());
            for z in (p.z - r).floor() as i32..=(p.z + r).ceil() as i32 {
                for yy in (p.y - r).floor() as i32..=(p.y + r).ceil() as i32 {
                    for x in (p.x - r).floor() as i32..=(p.x + r).ceil() as i32 {
                        let q = v(x as f32, yy as f32, z as f32) - p;
                        if q.length() <= r && s.get(x, yy, z).is_none() {
                            s.set(
                                x,
                                yy,
                                z,
                                if q.y > r * 0.3 {
                                    SMOKE_L
                                } else if q.y < -r * 0.4 {
                                    SMOKE_D
                                } else {
                                    SMOKE
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    // ---- arms: wide umber sleeves; the left hand gripping the crooked staff, the right open
    for (i, ax) in [cx - 11.0, cx + 11.0].into_iter().enumerate() {
        s.part(3 + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        let sh = v(ax, 82.0, cz);
        let (elbow, wrist) = if i == 0 {
            (v(ax + out * 2.6, 67.0, cz + 0.6), v(ax + out * 2.8, 61.0, cz + 6.6))
        } else {
            (v(ax + out * 2.6, 66.0, cz + 1.0), v(ax + out * 3.4, 56.0, cz + 5.0))
        };
        ball(&mut s, sh, v(3.9, 3.7, 3.9), CIN_ROBE);
        sleeve(&mut s, &[(sh, 3.6), (elbow, 3.7), (wrist, 4.4)], CIN_ROBE, CIN_ROBE_D);
        s.paint(|x, y, z, c| {
            let mine = (x as f32 - cx) * out > 8.0 && y > 52;
            (mine && c == CIN_ROBE && (y + (x + z) / 2) % 5 == 0).then_some(CIN_ROBE_D)
        });
        let dir = (wrist - elbow).normalize();
        if i == 0 {
            // A bony fist round the staff.
            let fist = wrist + dir * 2.2 + v(0.0, -0.6, 0.0);
            ball(&mut s, fist, v(2.1, 2.5, 2.1), CIN_SKIN);
            for k in 0..4 {
                put(&mut s, fist + v(out * 0.3, 1.2 - k as f32 * 1.0, 2.0), CIN_SKIN_L);
            }
            // The staff: crooked, knotted, planted by his left foot, rising past his head to three
            // gnarled prongs that cradle the fire.
            let path = [
                v(-0.6, 1.0, 0.8),
                v(0.4, 20.0, 0.2),
                v(-0.5, 38.0, 0.9),
                v(0.0, 59.0, 0.0),
                v(0.6, 78.0, -0.5),
                v(-0.3, 95.0, 0.3),
                v(0.4, 106.0, -0.4),
            ];
            let at = |p: Vec3| v(fist.x + p.x, p.y, fist.z + 0.6 + p.z);
            let knots: Vec<(Vec3, f32)> = path.iter().map(|&p| (at(p), 1.35)).collect();
            sweep(&mut s, &knots, |p, c, _| {
                let n = nz(p.x as i32, p.y as i32 / 2, p.z as i32);
                if n < 0.2 || p.x > c.x + 0.6 {
                    CIN_WOOD_D
                } else if n > 0.85 {
                    CIN_WOOD_L
                } else {
                    CIN_WOOD
                }
            });
            for y in [24.0f32, 47.0, 86.0] {
                let k = path.windows(2).find(|w| w[1].y >= y).map(|w| w[0].lerp(w[1], (y - w[0].y) / (w[1].y - w[0].y))).unwrap_or(path[0]);
                ball(&mut s, at(k) + v(0.5, 0.0, -0.3), v(2.0, 1.6, 1.8), CIN_WOOD_D);
            }
            let top = at(path[6]);
            ball(&mut s, top, v(2.4, 2.0, 2.3), CIN_WOOD);
            for k in 0..3 {
                let a = k as f32 / 3.0 * TAU + 0.5;
                let (ca, sa) = (a.cos(), a.sin());
                let p0 = top + v(ca * 1.2, 1.0, sa * 1.2);
                let p1 = top + v(ca * 3.4, 4.6, sa * 3.4);
                let p2 = top + v(ca * 1.6, 8.6, sa * 1.6);
                let knots: Vec<(Vec3, f32)> = (0..=8).map(|j| j as f32 / 8.0).map(|t| (bezier([p0, p1, p2], t), 1.0 - t * 0.6)).collect();
                sweep(&mut s, &knots, |_, _, along| if along > 0.7 { CIN_WOOD_D } else { CIN_WOOD });
            }
            // The fire: smokeless, white-gold at its heart.
            flame(&mut s, top + v(0.0, 1.6, 0.0), 9.0, 2.5);
        } else {
            // The open hand: palm forward, long bony fingers spread and curling, long nails.
            let palm = wrist + dir * 2.6 + v(0.0, -1.4, 0.4);
            rounded(&mut s, palm, v(2.2, 2.6, 1.0), 2.2, 0, CIN_SKIN);
            for f in 0..4 {
                let fx = (f as f32 - 1.5) * 1.15;
                let len = 1.0 - (f as f32 - 1.6).abs() * 0.1;
                let k0 = palm + v(fx, -2.2, 0.2);
                let spread = fx * 0.22;
                let j1 = k0 + v(spread * 3.0, -3.2 * len, 0.8);
                let j2 = j1 + v(spread * 2.0, -2.4 * len, 1.3);
                let tip = j2 + v(spread, -1.6 * len, 1.4);
                limb(&mut s, &[(k0, 0.75), (j1, 0.65), (j2, 0.55), (tip, 0.5)], CIN_SKIN);
                put(&mut s, j1 + v(0.0, 0.0, 0.6), CIN_SKIN_L);
                put(&mut s, tip + v(0.0, -0.3, 0.6), CIN_HAIR_D);
            }
            let t0 = palm + v(-out * 2.0, -0.4, 0.4);
            let t1 = t0 + v(-out * 1.8, -1.8, 1.0);
            let t2 = t1 + v(-out * 0.6, -1.8, 1.0);
            limb(&mut s, &[(t0, 0.8), (t1, 0.65), (t2, 0.5)], CIN_SKIN);
        }
    }

    // ---- the head: gaunt and glaring under his hood, the hooked nose, bushy brows, the eyes'
    //      faint fire, the long beard and the wild hair escaping the hood
    s.part(5);
    let hd = v(cx, 94.0, cz + 2.0);
    s.capsule((cx, 82.0, cz + 0.6), (cx, 89.0, cz + 1.6), 2.6, 2.4, CIN_SKIN_D);
    ball(&mut s, hd + v(0.0, 1.4, -0.4), v(5.4, 6.4, 6.0), CIN_SKIN);
    rounded(&mut s, hd + v(0.0, -3.4, 1.6), v(4.3, 3.8, 4.2), 2.4, 0, CIN_SKIN);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, hd + v(side * 3.5, -0.8, 4.0), v(1.4, 1.2, 1.4), CIN_SKIN_L);
        for y in -4..=-1 {
            if let Some(z) = front(&s, (hd.x + side * 3.8).round() as i32, (hd.y + y as f32).round() as i32, d) {
                s.recolor((hd.x + side * 3.8).round() as i32, (hd.y + y as f32).round() as i32, z, CIN_SKIN_D);
            }
        }
    }
    s.capsule((hd.x - 4.2, hd.y + 2.8, hd.z + 4.6), (hd.x + 4.2, hd.y + 2.8, hd.z + 4.6), 1.2, 1.2, CIN_SKIN);
    // Furrows across his brow.
    for (y, half) in [(hd.y + 4.6, 3.0f32), (hd.y + 6.0, 2.4)] {
        for x in (hd.x - half).round() as i32..=(hd.x + half).round() as i32 {
            if let Some(z) = front(&s, x, y.round() as i32, d) {
                s.recolor(x, y.round() as i32, z, CIN_SKIN_D);
            }
        }
    }
    // The sockets and the eyes in them: a point of white-gold fire in each.
    for side in [-1.0f32, 1.0] {
        let ex = hd.x + side * 2.2;
        for x in [ex - 0.5, ex + 0.5] {
            let (xx, y) = (x.round() as i32, (hd.y + 1.0).round() as i32);
            if let Some(z) = front(&s, xx, y, d) {
                s.clear(xx, y, z);
                s.set(xx, y, z - 1, if (x - hd.x).abs() < 2.2 { CIN_EYE } else { CIN_EYE_D });
            }
        }
        put(&mut s, v(ex + side * 1.4, hd.y + 0.4, hd.z + 4.6), CIN_SKIN_D);
    }
    // The nose: long and hooked.
    s.capsule((hd.x, hd.y + 1.8, hd.z + 5.2), (hd.x, hd.y - 2.4, hd.z + 8.0), 0.9, 1.2, CIN_SKIN);
    ball(&mut s, v(hd.x, hd.y - 2.8, hd.z + 7.6), v(1.2, 1.1, 1.1), CIN_SKIN);
    put(&mut s, v(hd.x, hd.y - 0.2, hd.z + 7.4), CIN_SKIN_L);
    for x in [hd.x - 0.9, hd.x + 0.9] {
        put(&mut s, v(x, hd.y - 3.4, hd.z + 7.0), CIN_SKIN_D);
    }
    // Bushy white brows, bristling up and out from over his nose, drawn down in a glare.
    for side in [-1.0f32, 1.0] {
        for k in 0..5 {
            let x = hd.x + side * (1.5 + k as f32 * 0.8);
            let y = hd.y + 2.6 + k as f32 * 0.45;
            let c = if k % 2 == 0 { CIN_HAIR } else { CIN_HAIR_S };
            s.capsule((x, y, hd.z + 5.9), (x + side * 1.1, y + 1.0 + (k % 2) as f32 * 0.8, hd.z + 6.6 - k as f32 * 0.3), 0.75, 0.4, c);
        }
    }
    // The beard: from his jaw down over his chest to a point at the sash, in wavy white strands,
    // the moustache drooping into it.
    for y in 50..=90 {
        let k = (90 - y) as f32 / 40.0;
        // Narrow at the chin, fullest under it, then a long taper to its point.
        let rx = (3.4 + (k * 3.0).min(1.0) * 2.0) * (1.0 - k.powf(1.6)) + 0.6 + (y as f32 * 0.7).sin() * 0.35;
        let zc = hd.z + 5.4 + (k * 2.4).min(1.0) * 1.4 - k * 0.6;
        let rz = 2.0 + (1.0 - k) * 1.4;
        for z in (zc - rz).floor() as i32..=(zc + rz).ceil() as i32 {
            for x in (cx - rx).floor() as i32..=(cx + rx).ceil() as i32 {
                if ((x as f32 - cx) / rx).powi(2) + ((z as f32 - zc) / rz).powi(2) > 1.0 {
                    continue;
                }
                let col = if (x + y / 4) % 3 == 0 {
                    CIN_HAIR_S
                } else if nz(x, y / 3, z) > 0.92 {
                    CIN_HAIR_D
                } else {
                    CIN_HAIR
                };
                s.set(x, y, z, col);
            }
        }
    }
    let beard = |k: i32| move |t: f32| if nz(k, (t * 6.0) as i32, 5) < 0.25 { CIN_HAIR_S } else { CIN_HAIR };
    for k in 0..7 {
        let x = cx + (k as f32 / 6.0 - 0.5) * 3.0;
        drape(
            &mut s,
            v(x, 56.0, hd.z + 5.4),
            v((x - cx) * 0.1, -1.0, 0.1),
            5.0 + nz(k, 2, 2) * 5.0,
            v(0.0, 0.0, 1.0),
            500 + k,
            beard(500 + k),
        );
    }
    for side in [-1.0f32, 1.0] {
        s.capsule((hd.x + side * 0.6, hd.y - 3.6, hd.z + 7.4), (hd.x + side * 3.2, hd.y - 5.6, hd.z + 6.8), 1.1, 0.8, CIN_HAIR);
        s.capsule((hd.x + side * 3.2, hd.y - 5.6, hd.z + 6.8), (hd.x + side * 3.6, hd.y - 10.0, hd.z + 7.0), 0.8, 0.45, CIN_HAIR_S);
    }
    for x in (hd.x - 1.5).round() as i32..=(hd.x + 1.5).round() as i32 {
        put(&mut s, v(x as f32, hd.y - 4.8, hd.z + 7.4), CIN_SKIN_D);
    }
    // The hood: deep umber, its brim out over his brow so that his eyes glow from its shadow,
    // dark within, falling to his shoulders, its peak dropping behind.
    let hc = hd + v(0.0, 1.0, -0.8);
    for y in (hd.y - 10.0) as i32..=(hd.y + 12.0) as i32 {
        for z in (hc.z - 11.0) as i32..=(hc.z + 11.0) as i32 {
            for x in (hc.x - 10.0) as i32..=(hc.x + 10.0) as i32 {
                if s.get(x, y, z).is_some() {
                    continue;
                }
                let (p, f) = (v(x as f32, y as f32, z as f32) - hc, v(x as f32, y as f32, z as f32) - hd);
                let q = (p.x / 7.8).powi(2) + (p.y / 9.0).powi(2) + (p.z / 8.6).powi(2);
                let peak = (p.x / 3.4).powi(2) + ((p.y - 7.6) / 3.2).powi(2) + ((p.z + 3.6) / 4.2).powi(2) <= 1.0;
                if q > 1.0 && !peak {
                    continue;
                }
                let face = f.y < 3.8 && f.x.abs() < 5.4 && f.z > 0.5;
                let throat = f.y < -6.0 && f.z > -1.5;
                if face || throat {
                    continue;
                }
                let inner = q < 0.72 && !peak;
                let rim = f.z > 4.0 || (f.y < 4.8 && f.x.abs() < 6.4 && f.z > 1.0);
                let fold = (p.x.atan2(p.z) * 5.0 + p.y * 0.2).sin();
                let c = if inner || rim || fold > 0.7 {
                    CIN_ROBE_D
                } else if fold < -0.8 {
                    CIN_ROBE_L
                } else {
                    CIN_ROBE
                };
                s.set(x, y, z, c);
            }
        }
    }
    // Wild white hair escaping the hood down past his cheeks.
    let hair = |k: i32| move |t: f32| if nz(k, (t * 6.0) as i32, 6) < 0.3 { CIN_HAIR_S } else { CIN_HAIR };
    for side in [-1.0f32, 1.0] {
        for k in 0..7 {
            let root = hd + v(side * (4.4 + nz(k, 1, 1) * 0.6), 2.6 - k as f32 * 0.8, 3.6 + (k % 3) as f32 * 0.6);
            let seed = 600 + k + if side > 0.0 { 50 } else { 0 };
            drape(&mut s, root, v(side * 0.2, -0.4, 0.5), 6.0 + nz(seed, 3, 3) * 6.0, v(0.0, 0.0, 1.0), seed, hair(seed));
        }
    }
    s.finish(0.04)
}

// --------------------------------------------------------------------- arçura

/// The arçura's grid: 46 wide, 132 tall (its horns), 36 deep.
pub const ARCURA_SIZE: (i32, i32, i32) = (46, 132, 36);
const ARCURA_Z: f32 = 17.5;

/// Hips, shoulders, neck, and the tail at the small of its back.
pub const ARCURA_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [17.0, 61.0, 16.5] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [28.0, 61.0, 16.5] },
    PartDef { name: "leftArm", boxes: &[], pivot: [14.0, 96.0, 18.0] },
    PartDef { name: "rightArm", boxes: &[], pivot: [31.0, 96.0, 18.0] },
    PartDef { name: "head", boxes: &[], pivot: [22.5, 100.0, 18.3] },
    PartDef { name: "tail", boxes: &[], pivot: [22.5, 59.0, 12.5] },
];

/// The trunk's slice at height `y`: (its middle's z from the grid's, half its width, half its depth).
fn arcura_trunk(y: f32) -> (f32, f32, f32) {
    const ROWS: [(f32, f32, f32, f32); 7] = [
        (57.0, -0.8, 6.8, 5.0),
        (63.0, -0.8, 7.6, 5.4),
        (70.0, -0.4, 6.0, 4.3),
        (80.0, 0.0, 7.0, 4.9),
        (90.0, 0.4, 7.8, 5.4),
        (96.0, 0.6, 8.2, 5.1),
        (100.0, 0.8, 5.0, 4.0),
    ];
    let at = ROWS.windows(2).position(|w| y <= w[1].0).unwrap_or(ROWS.len() - 2);
    let (a, b) = (ROWS[at], ROWS[at + 1]);
    let k = ((y - a.0) / (b.0 - a.0)).clamp(0.0, 1.0);
    (a.1 + (b.1 - a.1) * k, a.2 + (b.2 - a.2) * k, a.3 + (b.3 - a.3) * k)
}

/// Bark about a limb's axis: its grain running along it in dark cracks and pale ridges, patched with moss.
fn bark(p: Vec3, c: Vec3) -> u32 {
    let (x, y, z) = (p.x as i32, p.y as i32, p.z as i32);
    let crack = nz(x, y / 4, z);
    if nz(x / 3, y / 3, z / 3) < 0.16 && p.y > c.y - 0.5 {
        if crack < 0.4 {
            AR_MOSS
        } else {
            AR_MOSS_L
        }
    } else if crack < 0.24 {
        AR_BARK_D
    } else if crack > 0.86 {
        AR_BARK_L
    } else if nz(x, y, z) > 0.95 {
        AR_LICHEN
    } else {
        AR_BARK
    }
}

/// A leaf's outline along it (0 to 1): pointed at both ends, broadest a little past its middle.
fn leaf_shape(t: f32) -> f32 {
    (t.powf(0.7) * (1.0 - t)).max(0.0).sqrt() * 2.1
}

/// A leaf's colour: `shade` picks its green; its midrib and edges darker.
fn leaf_paint(shade: f32) -> impl Fn(f32, f32) -> u32 {
    let base = if shade < 0.3 {
        AR_LEAF_D
    } else if shade < 0.62 {
        AR_LEAF
    } else if shade < 0.86 {
        AR_LEAF_L
    } else {
        AR_LEAF_Y
    };
    move |t, u| {
        let c = if u.abs() < 0.16 && t < 0.85 {
            scale(base, 1.18)
        } else if u.abs() > 0.75 {
            scale(base, 0.8)
        } else {
            base
        };
        scale(c, 0.72 + t * 0.45)
    }
}

/**
 * The arçura: the spirit of the forest, tall and lanky, of bark and moss, its trunk wrapped in
 * leaves of many greens with vines about its waist; a long face of bark with a knot of a nose,
 * green fire deep in its eyes and a crack of a mouth; hair and a long beard of living grass, two
 * short horns like branches; long arms ending in fingers of twigs, the legs of a goat on cloven
 * hooves, and a short tail tufted with grass.
 */
pub fn arcura() -> Collected {
    let (w, h, d) = ARCURA_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (22.5f32, ARCURA_Z);

    // ---- legs: a goat's, the knee high and forward, the hock back, mossy thighs, cloven hooves
    for (i, lx) in [cx - 5.5, cx + 5.5].into_iter().enumerate() {
        s.part(1 + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        let hip = v(lx, 61.0, cz - 1.0);
        let knee = v(lx + out * 0.6, 46.0, cz + 4.6);
        let hock = v(lx + out * 0.9, 25.0, cz - 5.0);
        let fetlock = v(lx + out * 0.9, 6.5, cz - 1.6);
        let hoof = v(lx + out * 0.9, 2.2, cz + 1.4);
        sweep(&mut s, &[(hip, 4.8), (knee, 3.6)], |p, c, _| bark(p, c));
        ball(&mut s, knee, Vec3::splat(3.4), AR_BARK);
        sweep(&mut s, &[(knee, 3.4), (hock, 2.3)], |p, c, _| bark(p, c));
        ball(&mut s, hock, v(2.4, 2.6, 2.6), AR_BARK_L);
        sweep(&mut s, &[(hock, 2.1), (fetlock, 1.8), (hoof + v(0.0, 1.6, -0.8), 1.9)], |p, c, _| bark(p, c));
        ball(&mut s, fetlock + v(0.0, 0.0, -0.8), Vec3::splat(2.0), AR_BARK_D);
        // The hoof: two toes of dark horn with a cleft between.
        for side in [-1.0f32, 1.0] {
            rounded(&mut s, hoof + v(side * 1.15, 0.0, 0.4), v(1.15, 2.0, 2.6), 2.2, 0, if side < 0.0 { AR_HOOF } else { AR_HOOF_L });
        }
        s.paint(|x, y, z, c| ((c == AR_HOOF || c == AR_HOOF_L) && y >= 3 && (x + z) % 2 == 0).then_some(AR_BARK_D));
        // Moss shaggy on the thighs.
        s.paint(|x, y, z, c| (c == AR_BARK && y > 44 && nz(x / 2, y / 2, z / 2) < 0.5).then_some(AR_MOSS));
    }
    shag(&mut s, (w, h, d), AR_MOSS, AR_MOSS_D, AR_MOSS_L, 0.3, 2);

    // ---- the trunk: bark, wrapped in leaves at the chest and hips, vines about its waist
    s.part(0);
    for y in 56..=100 {
        let (zo, rx, rz) = arcura_trunk(y as f32);
        for z in (cz + zo - rz - 1.0) as i32..=(cz + zo + rz + 1.0) as i32 {
            for x in (cx - rx - 1.0) as i32..=(cx + rx + 1.0) as i32 {
                let (dx, dz) = (x as f32 - cx, z as f32 - cz - zo);
                if (dx / rx).powi(2) + (dz / rz).powi(2) <= 1.0 {
                    let p = v(x as f32, y as f32, z as f32);
                    s.set(x, y, z, bark(p, p));
                }
            }
        }
    }
    // Vines winding round the waist.
    for k in 0..2 {
        let n = 90;
        for j in 0..=n {
            let t = j as f32 / n as f32;
            let y = 64.0 + t * 15.0;
            let (zo, rx, rz) = arcura_trunk(y);
            let a = t * TAU * 1.6 + k as f32 * PI;
            put(&mut s, v(cx + (rx + 0.4) * a.cos(), y, cz + zo + (rz + 0.4) * a.sin()), if j % 9 == 0 { AR_LEAF_L } else { AR_VINE });
        }
    }
    // The leaves: shingled in rows from the shoulders down the chest, and a skirt of longer ones from the hips.
    let mut row = 0;
    let mut y = 95.0f32;
    while y > 50.0 {
        if (64.0..80.0).contains(&y) {
            y -= 3.0;
            continue;
        }
        let skirt = y < 64.0;
        let (zo, rx, rz) = arcura_trunk(y.max(57.0));
        let n = ((rx + rz) * PI / 4.2) as i32;
        for k in 0..n {
            let a = (k as f32 + 0.5 * (row % 2) as f32) / n as f32 * TAU;
            let radial = v(a.cos(), 0.0, a.sin());
            let root = v(cx + (rx + 0.2) * a.cos(), y, cz + zo + (rz + 0.2) * a.sin());
            let (len, flare) = if skirt { (10.0 + nz(k, row, 3) * 3.0, 3.0) } else { (7.4 + nz(k, row, 3) * 2.6, 2.2) };
            let tip = root + radial * flare + v(0.0, -len, 0.0);
            vane(&mut s, root, tip, radial, 5.0, leaf_shape, leaf_paint(nz(k, row, 7)));
        }
        row += 1;
        y -= if skirt { 5.0 } else { 5.2 };
    }

    // ---- arms: long and lean, mossy bark, a vine down each; fingers of twigs, knotted and long
    for (i, ax) in [cx - 8.5, cx + 8.5].into_iter().enumerate() {
        s.part(3 + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        let sh = v(ax, 96.0, cz + 0.5);
        let elbow = v(ax + out * 3.0, 76.0, cz + 0.4);
        let wrist = v(ax + out * 4.0, 57.0, cz + 3.6);
        ball(&mut s, sh, Vec3::splat(3.6), AR_BARK);
        sweep(&mut s, &[(sh, 3.1), (elbow, 2.5)], |p, c, _| bark(p, c));
        ball(&mut s, elbow, Vec3::splat(2.6), AR_BARK_L);
        sweep(&mut s, &[(elbow, 2.4), (wrist, 1.8)], |p, c, _| bark(p, c));
        // Leaves over the shoulder like a pauldron.
        for k in 0..4 {
            let a = (k as f32 / 3.0 - 0.5) * 2.2;
            let radial = v(out * a.cos(), 0.35, a.sin()).normalize();
            let root = sh + v(out * 0.8, 2.6, 0.0) + radial * 1.2;
            vane(&mut s, root, root + radial * 1.4 + v(out * 0.8, -6.0, 0.0), radial, 3.2, leaf_shape, leaf_paint(nz(k, i as i32, 11)));
        }
        let n = 60;
        for j in 0..=n {
            let t = j as f32 / n as f32;
            let c = if t < 0.5 { sh.lerp(elbow, t * 2.0) } else { elbow.lerp(wrist, t * 2.0 - 1.0) };
            let a = t * TAU * 2.2 + i as f32;
            let r = if t < 0.5 { 3.0 - t } else { 2.6 - t * 0.8 };
            put(&mut s, c + v(a.cos() * r, 0.0, a.sin() * r), if j % 8 == 0 { AR_LEAF_L } else { AR_VINE });
        }
        // The hand: a knot of a palm, its four twig fingers in a row from back to front, a thumb.
        let palm = wrist + v(out * 0.2, -2.8, 0.8);
        ball(&mut s, palm, v(1.6, 2.6, 2.2), AR_BARK);
        for f in 0..4 {
            let fz = (f as f32 - 1.5) * 1.25;
            let long = 1.0 - (f as f32 - 1.6).abs() * 0.12;
            let k0 = palm + v(0.0, -2.0, fz);
            let j1 = k0 + v(out * 0.5, -6.0 * long, 0.6 + fz * 0.3);
            let j2 = j1 + v(out * 0.3, -5.0 * long, 1.2 + fz * 0.15);
            let tip = j2 + v(0.0, -4.0 * long, 1.8);
            limb(&mut s, &[(k0, 1.0), (j1, 0.8), (j2, 0.6), (tip, 0.3)], AR_BARK_D);
            ball(&mut s, j1, Vec3::splat(1.05), AR_BARK_L);
            ball(&mut s, j2, Vec3::splat(0.85), AR_BARK_L);
            if f % 2 == 1 {
                let twig = j1 + v(0.0, -1.6, 0.0);
                s.capsule((twig.x, twig.y, twig.z), (twig.x + out * 1.8, twig.y - 1.6, twig.z + 0.8), 0.4, 0.2, AR_BARK_L);
            }
        }
        let t0 = palm + v(-out * 1.2, -1.0, 1.6);
        let t1 = t0 + v(-out * 0.8, -3.6, 1.6);
        let t2 = t1 + v(-out * 0.2, -3.0, 1.2);
        limb(&mut s, &[(t0, 0.75), (t1, 0.55), (t2, 0.25)], AR_BARK);
        ball(&mut s, t1, Vec3::splat(0.75), AR_BARK_D);
    }

    // ---- the head: a long face of bark, a heavy brow over green fire, the knot of a nose, a crack
    //      of a mouth; horns like short branches; hair and beard of grass
    s.part(5);
    let hd = v(cx, 111.0, cz + 1.2);
    sweep(&mut s, &[(v(cx, 99.0, cz + 0.8), 2.8), (v(cx, 104.0, cz + 1.2), 2.6)], |p, c, _| bark(p, c));
    ball(&mut s, hd + v(0.0, 1.0, -0.4), v(4.8, 7.0, 5.4), AR_BARK);
    rounded(&mut s, hd + v(0.0, -5.0, 1.0), v(3.8, 3.6, 4.0), 2.4, 0, AR_BARK);
    s.paint(|x, y, z, c| {
        (c == AR_BARK && y > 100).then(|| {
            let crack = nz(x, y / 4, z);
            if crack < 0.26 {
                AR_BARK_D
            } else if crack > 0.84 {
                AR_BARK_L
            } else if nz(x, y, z) > 0.94 {
                AR_BARK_G
            } else {
                AR_BARK
            }
        })
    });
    s.capsule((hd.x - 4.2, hd.y + 2.0, hd.z + 4.6), (hd.x + 4.2, hd.y + 2.0, hd.z + 4.6), 1.4, 1.4, AR_BARK_L);
    // The eyes: deep hollows under the brow, green fire at the bottom of each.
    let fz = (hd.z + 5.4).round() as i32;
    for side in [-1.0f32, 1.0] {
        let ex = hd.x + side * 2.2;
        for (x, y) in [(ex - 0.5, 0.0f32), (ex + 0.5, 0.0), (ex - 0.5, 1.0), (ex + 0.5, 1.0)] {
            let (xx, yy) = (x.round() as i32, (hd.y + y).round() as i32);
            for z in fz - 2..=fz + 2 {
                s.clear(xx, yy, z);
            }
            s.set(xx, yy, fz - 3, AR_HOLE);
        }
        let y = hd.y.round() as i32;
        for dy in 0..=1 {
            s.set((ex - side * 0.5).round() as i32, y + dy, fz - 2, if dy == 0 { AR_EYE_L } else { AR_EYE });
            s.set((ex + side * 0.5).round() as i32, y + dy, fz - 2, AR_EYE);
        }
    }
    // The nose: a long gnarled knot.
    s.capsule((hd.x, hd.y + 1.4, hd.z + 5.0), (hd.x, hd.y - 3.6, hd.z + 7.4), 1.0, 1.3, AR_BARK);
    ball(&mut s, v(hd.x, hd.y - 3.8, hd.z + 7.0), v(1.4, 1.2, 1.3), AR_BARK_L);
    put(&mut s, v(hd.x, hd.y - 1.2, hd.z + 6.6), AR_BARK_D);
    // The mouth: a crack across the face like a split in a trunk.
    for j in -4i32..=4 {
        let x = (hd.x + j as f32 * 0.6).round() as i32;
        let y = (hd.y - 6.0 + (j as f32 * 0.3).abs() * 0.5).round() as i32;
        if let Some(z) = front(&s, x, y, d) {
            s.clear(x, y, z);
            s.recolor(x, y, z - 1, AR_HOLE);
            if j.abs() < 3 {
                s.clear(x, y - 1, z);
                s.recolor(x, y - 1, z - 1, AR_HOLE);
            }
        }
    }
    // The horns: short branches up and out from the brow, each forked, pale at the tips.
    let horn = |q: Vec3, c: Vec3, along: f32| {
        if along > 0.82 {
            AR_LICHEN
        } else if nz(q.x as i32, q.y as i32 / 2, q.z as i32) < 0.3 || q.y < c.y - 0.6 {
            AR_BARK_G
        } else {
            AR_BARK_L
        }
    };
    for side in [-1.0f32, 1.0] {
        let root = hd + v(side * 2.8, 5.4, 2.2);
        let p = [root, root + v(side * 2.6, 6.0, -0.4), root + v(side * 6.0, 10.6, -2.6)];
        let knots: Vec<(Vec3, f32)> = (0..=10).map(|k| k as f32 / 10.0).map(|t| (bezier(p, t), 1.9 - t * 1.4)).collect();
        sweep(&mut s, &knots, horn);
        for (t, out, up, fwd) in [(0.42f32, 3.4f32, 2.6f32, 1.2f32), (0.7, 0.6, 3.6, -1.4)] {
            let a = bezier(p, t);
            let b = a + v(side * out, up, fwd);
            let knots = [(a, 1.0), ((a + b) * 0.5 + v(0.0, 0.4, 0.0), 0.7), (b, 0.35)];
            sweep(&mut s, &knots, horn);
        }
        let tip = bezier(p, 0.42) + v(side * 3.6, 2.9, 1.3);
        vane(&mut s, tip, tip + v(side * 1.6, 2.2, 1.2), v(0.0, 0.3, 1.0), 2.2, leaf_shape, leaf_paint(0.7));
    }
    // Hair of grass over the crown and down its back; a beard of it from the jaw down the chest.
    let grass = |k: i32| {
        move |t: f32| {
            if t > 0.75 && nz(k, 1, 3) < 0.5 {
                AR_GRASS_Y
            } else if nz(k, (t * 5.0) as i32, 4) < 0.3 {
                AR_GRASS_D
            } else {
                AR_GRASS
            }
        }
    };
    let crown = hd + v(0.0, 1.6, -1.0);
    for k in 0..40 {
        let a = PI * (k as f32 / 39.0) * 1.2 - 0.1 * PI;
        let (ca, sa) = (a.cos(), a.sin());
        let up = nz(k, 2, 2);
        let root = crown + v(ca * 4.8, 4.0 + up * 3.0, -sa * 5.0);
        let len = 8.0 + nz(k, 3, 3) * 9.0;
        drape(&mut s, root, v(ca * 0.6, 0.6, -sa * 0.6 - 0.2), len, v(ca, 0.0, -sa - 0.2).normalize(), 700 + k, grass(700 + k));
    }
    for k in 0..10 {
        let a = nz(k, 4, 4) * TAU;
        let root = crown + v(a.cos() * 3.0, 6.8, a.sin() * 3.0);
        drape(&mut s, root, v(a.cos() * 0.5, 1.0, a.sin() * 0.5), 4.0 + nz(k, 5, 5) * 4.0, Vec3::Y, 800 + k, grass(800 + k));
    }
    // (The beard paler than the hair, yellowing like hay toward its ends.)
    let straw = |k: i32| move |t: f32| if nz(k, (t * 5.0) as i32, 4) < 0.25 + t * 0.5 { AR_GRASS_Y } else { AR_GRASS };
    for k in 0..16 {
        let f = k as f32 / 15.0 - 0.5;
        let root = hd + v(f * 7.0, -5.0 - (f * 2.4).powi(2) * 1.5 - nz(k, 6, 6) * 1.2, 3.4 + (1.0 - (f * 2.0).abs()) * 1.6);
        let len = 9.0 + (1.0 - (f * 2.0).abs()) * 9.0 + nz(k, 7, 7) * 3.0;
        drape(&mut s, root, v(f * 0.5, -1.0, 0.5), len, v(0.0, 0.0, 1.0), 900 + k, straw(900 + k));
    }
    // A few small white flowers in the grass.
    for k in 0..7 {
        let a = nz(k, 8, 8) * PI * 1.2 - 0.1 * PI;
        put(&mut s, crown + v(a.cos() * 5.4, 3.0 + nz(k, 9, 9) * 3.0, -a.sin() * 5.6), AR_FLOWER);
    }

    // ---- the tail: short, out from the small of its back, tufted with grass
    s.part(6);
    let tail = [(v(cx, 59.0, cz - 4.6), 1.7), (v(cx, 56.6, cz - 8.0), 1.4), (v(cx + 0.4, 55.0, cz - 10.4), 1.1)];
    sweep(&mut s, &tail, |p, c, _| bark(p, c));
    for k in 0..12 {
        let a = nz(k, 10, 10) * TAU;
        let root = v(cx + 0.4, 55.0, cz - 10.6) + v(a.cos(), a.sin(), 0.0) * 0.8;
        drape(&mut s, root, v(a.cos() * 0.6, 0.3, -1.0), 5.0 + nz(k, 11, 11) * 4.0, v(0.0, 0.0, -1.0), 950 + k, grass(950 + k));
    }
    s.finish(0.04)
}

// --------------------------------------------------------------- karakoncolos

/// The karakoncolos's grid: 72 wide (its arms), 138 tall (its horns), 58 deep.
pub const KARAKONCOLOS_SIZE: (i32, i32, i32) = (72, 138, 58);
const KK_Z: f32 = 28.5;

/// Hips, shoulders and the neck down in its hunched shoulders.
pub const KARAKONCOLOS_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [26.0, 57.0, 26.5] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [45.0, 57.0, 26.5] },
    PartDef { name: "leftArm", boxes: &[], pivot: [14.5, 109.0, 31.0] },
    PartDef { name: "rightArm", boxes: &[], pivot: [56.5, 109.0, 31.0] },
    PartDef { name: "head", boxes: &[], pivot: [35.5, 109.0, 36.5] },
];

/// Its trunk from the hips up: (y, its middle's z from the grid's, half its width, its depth before and behind it).
const KK_TRUNK: [(f32, f32, f32, f32, f32); 11] = [
    (47.0, -2.0, 11.4, 8.2, 8.8),
    (57.0, -2.4, 13.0, 9.2, 9.6),
    (66.0, -2.0, 13.2, 9.6, 9.4),
    (74.0, -1.0, 14.0, 10.6, 9.8),
    (84.0, 0.4, 16.0, 11.8, 10.8),
    (95.0, 1.6, 17.4, 12.4, 12.0),
    (103.0, 2.6, 19.4, 11.8, 13.2),
    (110.0, 3.4, 20.0, 10.6, 13.8),
    (116.0, 4.0, 15.4, 8.6, 13.2),
    (121.0, 5.0, 10.2, 6.0, 10.6),
    (124.0, 6.0, 5.0, 3.0, 5.0),
];

/// Its trunk's slice at height `y`, between the rows of `KK_TRUNK`: (z offset, half width, front, back).
fn kk_trunk(y: f32) -> (f32, f32, f32, f32) {
    let rows = &KK_TRUNK;
    let at = rows.windows(2).position(|w| y <= w[1].0).unwrap_or(rows.len() - 2);
    let (a, b) = (rows[at], rows[at + 1]);
    let k = ((y - a.0) / (b.0 - a.0)).clamp(0.0, 1.0);
    (a.1 + (b.1 - a.1) * k, a.2 + (b.2 - a.2) * k, a.3 + (b.3 - a.3) * k, a.4 + (b.4 - a.4) * k)
}

/// Its skin about a limb's axis: frost-pale, darker underneath and in mottles, pale on the muscle.
fn kk_skin(p: Vec3, c: Vec3) -> u32 {
    let n = nz(p.x as i32, p.y as i32, p.z as i32);
    if p.y < c.y - 1.5 || n < 0.12 {
        KK_SKIN_D
    } else if n > 0.9 || p.y > c.y + 2.5 {
        KK_SKIN_L
    } else {
        KK_SKIN
    }
}

/// A smooth path through `pts` (Catmull-Rom), `per` points between each pair of them.
fn smooth(pts: &[Vec3], per: usize) -> Vec<Vec3> {
    let n = pts.len();
    let mut out = Vec::with_capacity(n * per);
    for i in 0..n - 1 {
        let (p0, p1, p2, p3) = (pts[i.saturating_sub(1)], pts[i], pts[i + 1], pts[(i + 2).min(n - 1)]);
        for k in 0..per {
            let t = k as f32 / per as f32;
            let (t2, t3) = (t * t, t * t * t);
            out.push((p1 * 2.0 + (p2 - p0) * t + (p0 * 2.0 - p1 * 5.0 + p2 * 4.0 - p3) * t2 + (p1 * 3.0 - p0 - p2 * 3.0 + p3) * t3) * 0.5);
        }
    }
    out.push(pts[n - 1]);
    out
}

/// An icicle hanging from `top`, `len` long and `r` thick at its root: glassy, its core and point pale.
fn icicle(s: &mut Sculpt, top: Vec3, len: f32, r: f32) {
    let steps = (len * 2.0).ceil().max(1.0) as i32;
    for k in 0..=steps {
        let t = k as f32 / steps as f32;
        let rad = r * (1.0 - t).max(0.0).powf(0.8) + 0.35;
        let c = top + v(0.0, -len * t, 0.0);
        for z in (c.z - rad).floor() as i32..=(c.z + rad).ceil() as i32 {
            for x in (c.x - rad).floor() as i32..=(c.x + rad).ceil() as i32 {
                let q = (x as f32 - c.x).powi(2) + (z as f32 - c.z).powi(2);
                if q <= rad * rad {
                    let col = if t > 0.8 || q < (rad * 0.4).powi(2) {
                        ICE_L
                    } else if nz(x, c.y as i32, z) < 0.3 {
                        ICE_D
                    } else {
                        ICE
                    };
                    s.set(x, c.y.round() as i32, z, col);
                }
            }
        }
    }
}

/**
 * The karakoncolos: the demon of the zemheri, the coldest nights; taller than a door and stooped
 * under the hunch of its shoulders, its skin the pale blue-grey of frost; a shaggy coat of
 * black-blue fur over its shoulders and back, its forearms and shins, rime lying on its shoulders
 * and icicles hanging from it; arms like an ape's, long enough to drag its black claws; ram's
 * horns coiled either side of its head, pale and rimed with frost; pale blue fire in its eyes
 * under a heavy brow, and a wide maw of fangs, icicles hanging from its jaw.
 */
pub fn karakoncolos() -> Collected {
    let (w, h, d) = KARAKONCOLOS_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (35.5f32, KK_Z);
    let skin = |c: u32| c == KK_SKIN || c == KK_SKIN_D || c == KK_SKIN_L;

    // ---- the trunk: hips, a hard belly, a barrel of a chest, the great hunch of its shoulders
    //      (first, so that its fur is painted before the limbs are there to catch the paint)
    s.part(0);
    for y in 46..=124 {
        let (zo, rx, rf, rb) = kk_trunk(y as f32);
        let zc = cz + zo;
        for z in (zc - rb - 1.0) as i32..=(zc + rf + 1.0) as i32 {
            for x in (cx - rx - 1.0) as i32..=(cx + rx + 1.0) as i32 {
                let dz = z as f32 - zc;
                let r = if dz > 0.0 { rf } else { rb };
                if ((x as f32 - cx) / rx).powi(2) + (dz / r).powi(2) <= 1.0 {
                    s.set(x, y, z, if nz(x, y, z) < 0.12 { KK_SKIN_D } else { KK_SKIN });
                }
            }
        }
    }
    // The chest muscles, shadowed beneath, and the knots of the belly.
    for side in [-1.0f32, 1.0] {
        let (zo, _, rf, _) = kk_trunk(93.0);
        ball(&mut s, v(cx + side * 7.2, 93.0, cz + zo + rf - 2.4), v(7.6, 5.6, 3.6), KK_SKIN);
        for (k, y) in [64.0f32, 70.0, 76.0].into_iter().enumerate() {
            let (zo, _, rf, _) = kk_trunk(y);
            ball(&mut s, v(cx + side * 3.0, y, cz + zo + rf - 0.9), v(2.5, 2.4, 1.6), if k == 2 { KK_SKIN } else { KK_SKIN_L });
        }
    }
    s.paint(|x, y, z, c| {
        let (zo, _, rf, _) = kk_trunk(y as f32);
        let front = z as f32 > cz + zo + rf - 2.0;
        let fx = (x as f32 - cx).abs();
        let under = (87..=88).contains(&y) && fx > 1.5 && fx < 13.0;
        let groove = fx < 0.8 && ((89..=97).contains(&y) || (61..=79).contains(&y));
        (c == KK_SKIN && front && (under || groove)).then_some(KK_SKIN_D)
    });
    // Fur down its back, over its shoulders and hunch, a pelt about its hips with a flap before and behind.
    s.paint(|x, y, z, c| {
        let (zo, rx, _, rb) = kk_trunk(y as f32);
        let back = (z as f32) < cz + zo - rb * 0.15 + (x as f32 - cx).abs() / rx * 4.0;
        (skin(c) && (back || y <= 58 || y >= 107)).then_some(KK_FUR)
    });
    rounded(&mut s, v(cx, 46.0, cz + 6.6), v(7.0, 7.0, 2.6), 2.2, 0, KK_FUR);
    rounded(&mut s, v(cx, 47.0, cz - 9.2), v(8.4, 7.0, 2.6), 2.2, 0, KK_FUR);

    // ---- legs: thick and bowed, the knees bent, the shins shaggy, broad feet with clawed toes
    for (i, lx) in [cx - 9.5, cx + 9.5].into_iter().enumerate() {
        s.part(1 + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        let hip = v(lx, 57.0, cz - 2.0);
        let knee = v(lx + out * 1.6, 31.0, cz + 4.0);
        let ankle = v(lx + out * 2.2, 8.0, cz - 1.0);
        sweep(&mut s, &[(hip, 8.0), (v(lx + out * 0.8, 44.0, cz + 2.0), 7.0), (knee, 5.6)], |p, c, _| kk_skin(p, c));
        ball(&mut s, knee + v(0.0, 0.0, 1.6), v(4.4, 4.6, 4.0), KK_SKIN_L);
        sweep(&mut s, &[(knee, 5.4), (v(lx + out * 2.0, 19.0, cz + 1.4), 5.2), (ankle, 4.0)], |p, c, _| kk_skin(p, c));
        ball(&mut s, v(lx + out * 1.9, 21.0, cz - 1.2), v(4.4, 6.0, 4.0), KK_SKIN);
        let heel = v(ankle.x, 3.4, cz - 4.0);
        let toes = v(ankle.x + out * 0.6, 3.0, cz + 6.0);
        s.capsule((heel.x, heel.y, heel.z), (toes.x, toes.y, toes.z), 4.0, 3.4, KK_SKIN);
        s.capsule((ankle.x, ankle.y, ankle.z), (toes.x, toes.y + 1.0, toes.z - 2.0), 3.6, 3.0, KK_SKIN);
        for (k, a) in [-0.5f32, -0.17, 0.17, 0.5].into_iter().enumerate() {
            let dir = v((a + out * 0.15).sin(), 0.0, (a + out * 0.15).cos());
            let len = 4.6 - (k as f32 - 1.5).abs() * 0.7;
            let tip = toes + dir * (len + 1.6) + v(0.0, -0.8, 0.0);
            s.capsule((toes.x + dir.x * 1.6, toes.y, toes.z + dir.z * 1.6), (tip.x, tip.y, tip.z), 1.8, 1.4, KK_SKIN);
            put(&mut s, toes + dir * 2.6 + v(0.0, 1.6, 0.0), KK_SKIN_L);
            claw(
                &mut s,
                tip + dir * 0.8 + v(0.0, 0.5, 0.0),
                tip + dir * 4.0 + v(0.0, -2.2, 0.0),
                dir * 0.8 + v(0.0, 1.2, 0.0),
                1.2,
                KK_CLAW,
            );
        }
    }
    // The shins under shaggy fur (the trunk reaches no lower than the hips).
    s.paint(|_, y, _, c| ((8..=29).contains(&y) && skin(c)).then_some(KK_FUR));

    // ---- arms: an ape's, long and thick, the shoulders and forearms shaggy, huge clawed hands
    //      hanging below its knees
    for (i, ax) in [cx - 21.0, cx + 21.0].into_iter().enumerate() {
        s.part(3 + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        let sh = v(ax, 109.0, cz + 2.5);
        let elbow = v(ax + out * 4.0, 76.0, cz + 3.0);
        let wrist = v(ax + out * 4.6, 41.0, cz + 9.0);
        ball(&mut s, sh + v(out * 0.6, 0.6, 0.0), v(8.2, 7.6, 8.0), KK_SKIN);
        sweep(&mut s, &[(sh, 7.0), (v(ax + out * 2.4, 93.0, cz + 3.2), 6.4), (elbow, 5.2)], |p, c, _| kk_skin(p, c));
        ball(&mut s, v(ax + out * 2.2, 91.0, cz + 6.4), v(4.6, 7.2, 4.0), KK_SKIN_L);
        ball(&mut s, elbow + v(0.0, 0.0, -2.6), v(3.4, 3.8, 3.0), KK_SKIN_L);
        sweep(&mut s, &[(elbow, 5.6), (v(ax + out * 4.8, 63.0, cz + 5.2), 6.2), (wrist, 4.4)], |p, c, _| kk_skin(p, c));
        // The hand: the palm turned in, four thick fingers curling, black claws, a thumb before them.
        let palm = wrist + v(out * 0.4, -5.2, 1.8);
        rounded(&mut s, palm, v(2.9, 5.0, 4.6), 2.2, 0, KK_SKIN);
        for f in 0..4 {
            let fz = (f as f32 - 1.5) * 2.2;
            let long = 1.0 - (f as f32 - 1.6).abs() * 0.1;
            let k0 = palm + v(0.0, -4.0, fz);
            let j1 = k0 + v(0.0, -5.0 * long, 1.0);
            let j2 = j1 + v(0.0, -4.0 * long, 1.8);
            let tip = j2 + v(0.0, -2.6 * long, 2.0);
            limb(&mut s, &[(k0, 1.8), (j1, 1.55), (j2, 1.35), (tip, 1.15)], KK_SKIN);
            ball(&mut s, j1 + v(0.0, 0.0, 0.9), Vec3::splat(1.3), KK_SKIN_L);
            ball(&mut s, j2 + v(0.0, 0.0, 0.9), Vec3::splat(1.1), KK_SKIN_L);
            claw(&mut s, tip + v(0.0, -0.3, 0.6), tip + v(0.0, -5.6, 2.6), v(0.0, 0.0, 1.3), 1.25, KK_CLAW);
        }
        let t0 = palm + v(-out * 2.4, -1.4, 3.2);
        let t1 = t0 + v(-out * 1.4, -3.6, 1.8);
        let t2 = t1 + v(-out * 0.4, -3.0, 1.2);
        limb(&mut s, &[(t0, 1.8), (t1, 1.5), (t2, 1.2)], KK_SKIN);
        claw(&mut s, t2 + v(0.0, -0.3, 0.6), t2 + v(0.0, -4.4, 2.0), v(0.0, 0.0, 1.0), 1.1, KK_CLAW);
    }
    // Fur on the forearms and over the shoulders (out past the trunk and legs, so only the arms).
    s.paint(|x, y, _, c| {
        let far = (x as f32 - cx).abs();
        (skin(c) && ((far > 20.0 && (42..=70).contains(&y)) || (far > 15.0 && y >= 103))).then_some(KK_FUR)
    });
    s.paint(|x, y, z, c| (c == KK_CLAW && (x + y + z) % 4 == 0).then_some(KK_CLAW_L));

    // ---- the head, set low and forward in its shoulders: a heavy scowling brow over pale fire,
    //      a flat nose, the maw wide open on its fangs, pointed ears, the coiled horns
    s.part(5);
    let hd = v(cx, 117.0, cz + 15.5);
    sweep(&mut s, &[(v(cx, 108.0, cz + 7.0), 6.4), (hd + v(0.0, -4.0, -5.0), 6.0)], |p, c, _| kk_skin(p, c));
    ball(&mut s, hd + v(0.0, 2.5, -2.0), v(8.0, 7.5, 8.0), KK_SKIN);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, hd + v(side * 5.8, 0.6, 4.4), v(2.7, 2.2, 2.6), KK_SKIN_L);
    }
    rounded(&mut s, hd + v(0.0, -1.4, 5.4), v(6.4, 2.8, 5.0), 2.4, 0, KK_SKIN);
    rounded(&mut s, hd + v(0.0, -11.2, 4.2), v(5.8, 2.4, 5.0), 2.4, 0, KK_SKIN);
    for side in [-1.0f32, 1.0] {
        s.capsule((hd.x + side * 6.0, hd.y - 2.6, hd.z + 2.4), (hd.x + side * 5.6, hd.y - 10.6, hd.z + 2.4), 1.9, 1.9, KK_SKIN_D);
    }
    // The maw: dark within, a tongue, gums, a row of teeth above and below and four great fangs.
    let (top, low) = ((hd.y - 4.0).round() as i32, (hd.y - 9.0).round() as i32);
    let lip = |x: i32| (hd.z + 9.6 - ((x as f32 - hd.x) / 5.0).powi(2) * 2.6).round() as i32;
    for x in (hd.x - 5.0).round() as i32..=(hd.x + 5.0).round() as i32 {
        let zf = lip(x);
        for y in low..=top {
            for z in (hd.z + 1.0) as i32..=zf + 1 {
                s.clear(x, y, z);
            }
            let tongue = y <= low + 1 && (x as f32 - hd.x).abs() < 3.0;
            s.set(x, y, zf - 3, if tongue { KK_TONGUE } else { KK_MOUTH });
        }
        s.set(x, top + 1, zf, KK_GUM);
        s.set(x, low - 1, zf, KK_GUM);
        s.set(x, top, zf, KK_TOOTH);
        s.set(x, low, zf, KK_TOOTH);
        if x % 2 == 0 {
            s.set(x, top - 1, zf, KK_TOOTH);
        }
    }
    for side in [-1.0f32, 1.0] {
        let (xu, xl) = (hd.x + side * 3.4, hd.x + side * 2.0);
        let (zu, zl) = (lip(xu.round() as i32) as f32, lip(xl.round() as i32) as f32);
        s.capsule((xu, top as f32 + 0.6, zu), (xu + side * 0.2, low as f32 + 0.2, zu + 0.4), 1.0, 0.3, KK_TOOTH);
        s.capsule((xl, low as f32 - 0.6, zl), (xl - side * 0.2, top as f32 - 1.2, zl + 0.4), 0.9, 0.3, KK_TOOTH);
    }
    // The nose: flat and broad, the nostrils flared.
    ball(&mut s, hd + v(0.0, 1.2, 9.2), v(2.6, 1.7, 1.5), KK_SKIN_D);
    for x in [hd.x - 1.1, hd.x + 1.1] {
        put(&mut s, v(x, hd.y + 0.8, hd.z + 10.4), KK_SKIN_DD);
    }
    // The brow: heavy, drawn down to the middle in a scowl; the eyes burning pale blue under it.
    for side in [-1.0f32, 1.0] {
        s.capsule((hd.x + side * 0.6, hd.y + 3.6, hd.z + 7.4), (hd.x + side * 7.4, hd.y + 6.2, hd.z + 5.0), 2.2, 1.8, KK_SKIN_D);
    }
    let fz = (hd.z + 7.0).round() as i32;
    for side in [-1.0f32, 1.0] {
        let ex = hd.x + side * 3.6;
        for (dx, dy) in [(-1.0f32, 0.0f32), (0.0, 0.0), (1.0, 0.0), (-1.0, 1.0), (0.0, 1.0), (1.0, 1.0)] {
            let (x, y) = ((ex + dx).round() as i32, (hd.y + 2.0 + dy).round() as i32);
            for z in fz - 1..=fz + 3 {
                s.clear(x, y, z);
            }
            s.set(x, y, fz - 2, KK_SKIN_DD);
        }
        let y = (hd.y + 2.0).round() as i32;
        s.set(ex.round() as i32, y, fz - 1, KK_EYE_L);
        s.set((ex + side).round() as i32, y, fz - 1, KK_EYE);
        s.set((ex - side).round() as i32, y, fz - 1, KK_EYE);
        s.set(ex.round() as i32, y + 1, fz - 1, KK_EYE);
    }
    // Pointed ears, back under the horns.
    for side in [-1.0f32, 1.0] {
        s.capsule((hd.x + side * 7.4, hd.y + 0.4, hd.z - 0.6), (hd.x + side * 11.6, hd.y + 2.2, hd.z - 3.6), 1.8, 0.4, KK_SKIN);
    }
    // The mane: over the back of its head and down its cheeks into the fur of its hunch.
    s.paint(|x, y, z, c| {
        let back = (z as f32) < hd.z - 1.0 - (y as f32 - hd.y).max(0.0) * 0.2;
        let cheek = (x as f32 - hd.x).abs() > 5.8 && (y as f32) < hd.y + 1.0 && (z as f32) < hd.z + 4.0;
        (y > 104 && skin(c) && (back || cheek)).then_some(KK_FUR)
    });
    // The horns: a ram's, ringed, up from its brow and coiled back, down and forward round its
    // ears to points beside its cheeks, rimed with frost on top.
    for side in [-1.0f32, 1.0] {
        let pts: Vec<Vec3> = [
            (4.4f32, 7.0f32, 2.4f32),
            (7.4, 12.6, 0.4),
            (11.0, 14.2, -4.6),
            (14.2, 10.2, -9.2),
            (15.6, 3.0, -8.6),
            (15.8, -1.8, -3.6),
            (14.8, -0.8, 2.0),
            (13.8, 2.8, 5.0),
            (13.2, 5.6, 6.4),
        ]
        .into_iter()
        .map(|(x, y, z)| hd + v(side * x, y, z))
        .collect();
        let path = smooth(&pts, 18);
        let total: f32 = path.windows(2).map(|w| (w[1] - w[0]).length()).sum();
        let n = path.len() as f32;
        let knots: Vec<(Vec3, f32)> = path
            .iter()
            .enumerate()
            .map(|(k, &p)| {
                let t = k as f32 / (n - 1.0);
                (p, (3.7 - t * 3.0) * (1.0 + 0.07 * (t * total * 3.6).sin()))
            })
            .collect();
        sweep(&mut s, &knots, |p, c, along| {
            let up = (p - c).normalize_or_zero().y;
            if up > 0.5 && nz(p.x as i32, p.y as i32, p.z as i32) > 0.3 {
                KK_FROST
            } else if along > 0.9 {
                KK_HORN_T
            } else if (along * total / 1.7).fract() < 0.3 {
                KK_HORN_D
            } else {
                KK_HORN
            }
        });
        // Icicles under the coil's lowest turn.
        for (t, len) in [(0.6f32, 4.0f32), (0.68, 2.6)] {
            let p = path[((n - 1.0) * t) as usize];
            icicle(&mut s, p + v(0.0, -2.4, 0.0), len, 0.9);
        }
    }
    // Icicles from its jaw.
    for (dx, len) in [(-3.0f32, 4.0f32), (-0.6, 5.4), (2.2, 3.6), (4.2, 2.6)] {
        icicle(&mut s, hd + v(dx, -13.4, 5.4 - dx.abs() * 0.4), len, 0.9);
    }

    // ---- the coat: shaggy, frosted at the tips, icicles hanging from its edges
    shag(&mut s, (w, h, d), KK_FUR, KK_FUR, KK_FUR_T, 0.55, 4);
    s.paint(|x, y, z, c| {
        (c == KK_FUR).then(|| {
            let n = nz(x, y / 2, z);
            if n < 0.25 {
                KK_FUR_D
            } else if n > 0.93 {
                KK_FUR_L
            } else {
                KK_FUR
            }
        })
    });
    // Rime settled in patches on the fur of its shoulders and hunch, where it faces the sky.
    let mut rime = vec![];
    for y in 100..h - 1 {
        for z in 0..d {
            for x in 0..w {
                let fur = s.get(x, y, z).is_some_and(|c| c == KK_FUR || c == KK_FUR_D || c == KK_FUR_L);
                if fur && s.get(x, y + 1, z).is_none() && nz(x / 3, y / 3, z / 3) < 0.45 {
                    rime.push((x, y, z));
                }
            }
        }
    }
    for (x, y, z) in rime {
        s.recolor(x, y, z, KK_RIME);
    }
    for (i, ax) in [cx - 21.0, cx + 21.0].into_iter().enumerate() {
        let out = if i == 0 { -1.0 } else { 1.0 };
        s.part(3 + i as u8);
        for k in 0..5 {
            let a = k as f32 / 5.0 * TAU + 0.3;
            icicle(&mut s, v(ax + out * 4.6 + a.cos() * 5.6, 40.0, cz + 9.0 + a.sin() * 5.6), 2.5 + nz(k, i as i32, 3) * 3.5, 0.9);
        }
        for k in 0..4 {
            let a = k as f32 / 4.0 * TAU + 0.8;
            icicle(&mut s, v(ax + out * 1.0 + a.cos() * 8.0, 100.0, cz + 2.5 + a.sin() * 8.0), 3.0 + nz(k, i as i32, 4) * 3.0, 1.0);
        }
    }
    s.part(0);
    for k in 0..9 {
        let a = k as f32 / 9.0 * TAU + 0.2;
        let (zo, rx, rf, rb) = kk_trunk(48.0);
        let r = if a.sin() > 0.0 { rf } else { rb };
        icicle(&mut s, v(cx + (rx + 1.6) * a.cos(), 42.0, cz + zo + (r + 1.6) * a.sin()), 2.5 + nz(k, 5, 5) * 4.0, 1.0);
    }
    for i in 0..2 {
        s.part(1 + i as u8);
        let lx = if i == 0 { cx - 11.7 } else { cx + 11.7 };
        for k in 0..4 {
            let a = k as f32 / 4.0 * TAU + 0.5;
            icicle(&mut s, v(lx + a.cos() * 5.2, 5.0 + nz(k, i, 6) * 2.0, cz - 1.0 + a.sin() * 5.2), 1.6 + nz(k, i, 7) * 1.6, 0.7);
        }
    }
    s.finish(0.04)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A model with its name, grid and rig.
    type Foe = (&'static str, Collected, (i32, i32, i32), &'static [PartDef]);

    /// Every model with its name, grid and rig.
    fn cast() -> Vec<Foe> {
        vec![
            ("tülükuş", tulukus(), TULUKUS_SIZE, TULUKUS_RIG),
            ("albastı", albasti(), ALBASTI_SIZE, ALBASTI_RIG),
            ("cin", cin(), CIN_SIZE, CIN_RIG),
            ("arçura", arcura(), ARCURA_SIZE, ARCURA_RIG),
            ("karakoncolos", karakoncolos(), KARAKONCOLOS_SIZE, KARAKONCOLOS_RIG),
        ]
    }

    #[test]
    fn every_fifth_stage_foe_stands_in_the_middle_of_its_grid_and_inside_it() {
        for (name, c, size, _) in cast() {
            assert!(c.voxels.len() > 2000, "{name}: {} voxels", c.voxels.len());
            assert_eq!((c.width, c.height, c.depth), size, "{name}");
            // Its feet on the floor, and its footprint about the grid's middle (where the rig puts its place).
            assert!(c.voxels.iter().map(|v| v.y).min().is_some_and(|y| y <= 1), "{name} stands on the floor");
            assert!(!c.voxels.iter().any(|v| v.x == 0), "{name}: a stray voxel on the grid's edge (a NaN)");
            let floor: Vec<_> = c.voxels.iter().filter(|v| v.y <= 2).collect();
            let mx = floor.iter().map(|v| v.x as f32).sum::<f32>() / floor.len() as f32;
            assert!((mx - (size.0 - 1) as f32 / 2.0).abs() < 3.0, "{name}: footprint centred at x {mx}");
            // Nothing cut off at the grid's edges: a voxel or two to spare all round, and above.
            let (x1, y1, z1) = (size.0 - 1, size.1 - 1, size.2 - 1);
            assert!(!c.voxels.iter().any(|v| v.x >= x1 || v.z <= 0 || v.z >= z1 || v.y >= y1), "{name}: something touches the grid's edge");
        }
    }

    #[test]
    fn every_fifth_stage_joint_lies_inside_its_grid_with_its_part_hanging_from_it() {
        for (name, c, size, rig) in cast() {
            for (i, def) in rig.iter().enumerate() {
                let [x, y, z] = def.pivot;
                assert!(x > 0.0 && y > 0.0 && z > 0.0, "{name} {}: {:?}", def.name, def.pivot);
                assert!(
                    x < (size.0 - 1) as f32 && y < (size.1 - 1) as f32 && z < (size.2 - 1) as f32,
                    "{name} {}: {:?}",
                    def.name,
                    def.pivot
                );
                // The part has voxels of its own, its joint over them (or among them) and not beside them.
                let part: Vec<_> = c.voxels.iter().filter(|v| v.part as usize == i + 1).collect();
                assert!(part.len() > 50, "{name} {}: {} voxels", def.name, part.len());
                let span = |k: usize| {
                    let at = part.iter().map(|v| [v.x, v.y, v.z][k] as f32);
                    (at.clone().fold(f32::MAX, f32::min), at.fold(f32::MIN, f32::max))
                };
                let ((x0, x1), (y0, _), (z0, z1)) = (span(0), span(1), span(2));
                assert!(x > x0 - 6.0 && x < x1 + 6.0 && z > z0 - 6.0 && z < z1 + 6.0, "{name} {}: its joint is beside it", def.name);
                assert!(y > y0 - 6.0, "{name} {}: its joint is under it", def.name);
            }
            // Left is the lower x, as in the third stage.
            for (left, right) in [("leftLeg", "rightLeg"), ("leftArm", "rightArm"), ("leftWing", "rightWing")] {
                let at = |n: &str| rig.iter().find(|p| p.name == n).map(|p| p.pivot[0]);
                if let (Some(l), Some(r)) = (at(left), at(right)) {
                    assert!(l < (size.0 - 1) as f32 / 2.0 && r > (size.0 - 1) as f32 / 2.0, "{name}: {left} at x {l}, {right} at x {r}");
                }
            }
        }
    }

    #[test]
    fn the_cins_fire_burns_at_the_tip_of_the_staff_in_his_left_hand() {
        let c = cin();
        let left = CIN_RIG.iter().position(|p| p.name == "leftArm").unwrap() as u8 + 1;
        let fire: Vec<_> = c
            .voxels
            .iter()
            .filter(|v| v.c >> 24 == 11 && (Vec3::new(v.x as f32, v.y as f32, v.z as f32) - Vec3::from(CIN_FLAME)).length() < 4.0)
            .collect();
        assert!(fire.len() > 20, "{} glowing voxels about CIN_FLAME", fire.len());
        assert!(fire.iter().all(|v| v.part == left), "the fire goes with the staff, in his left hand");
        assert!(CIN_FLAME[0] < (CIN_SIZE.0 - 1) as f32 / 2.0, "the staff is on his left (the lower x)");
    }

    /// The voxels of `c` inside the box `lo..=hi`, moved to the origin: a close-up for the sheet.
    fn crop(c: &Collected, lo: [i32; 3], hi: [i32; 3]) -> Collected {
        let voxels = c
            .voxels
            .iter()
            .filter(|v| (lo[0]..=hi[0]).contains(&v.x) && (lo[1]..=hi[1]).contains(&v.y) && (lo[2]..=hi[2]).contains(&v.z))
            .map(|v| {
                let mut v = *v;
                (v.x, v.y, v.z) = (v.x - lo[0], v.y - lo[1], v.z - lo[2]);
                v
            })
            .collect();
        Collected { voxels, width: hi[0] - lo[0] + 1, height: hi[1] - lo[1] + 1, depth: hi[2] - lo[2] + 1 }
    }

    /// Every model's sheet (front, side, back / top, two three-quarter views), into VH_PREVIEW_DIR,
    /// with a close-up of its head beside it.
    #[test]
    #[ignore]
    fn preview_stage_five() {
        let heads = [
            ([6, 48, 8], [41, 87, 45]),
            ([6, 80, 20], [41, 113, 53]),
            ([8, 76, 6], [36, 106, 34]),
            ([6, 92, 2], [39, 131, 35]),
            ([14, 96, 14], [57, 137, 57]),
        ];
        let names = ["tulukus", "albasti", "cin", "arcura", "karakoncolos"];
        for (((_, c, _, _), (lo, hi)), name) in cast().into_iter().zip(heads).zip(names) {
            println!("{}", super::super::preview::sheet(&c, name));
            println!("{}", super::super::preview::sheet(&crop(&c, lo, hi), &format!("{name}_head")));
        }
    }
}
