//! Envy's court's own (PRINCES.md §2): Leviathan, the prince of Envy, and his brood, the
//! serpentlings. Each faces +z, stands in the middle of its grid and is sculpted in the parts its
//! rig swings.
//!
//! Leviathan is the front third of a sea serpent risen out of the deep, two and a half times the
//! hero's height: a body as thick as a man, green-black scales edged in shadow with a sheen of
//! the deep on them, a pale ridged belly; it rises in an S off a great coil lying behind him on
//! the floor that arches up once as if out of the water and runs out into a finned tail curling
//! round his left. His head is long and finned: a crest of bone spines webbed with dark membrane
//! from his brow down his back, fans of gill fins either side of the jaw hinge, slit-pupilled eyes
//! burning envious yellow-green under heavy brows, barbels trailing from the snout, a jaw of
//! ivory fangs hanging a little open on a red maw. Pectoral fins spread from his flanks;
//! barnacles crust the old coil.
//!
//! The serpentlings are banded sea snakes reared up off their coils: teal and black bands, a
//! paddle of a tail, a small finned head, the same eyes.

use std::f32::consts::{PI, TAU};

use glam::Vec3;

use crate::render::carve::{ball, limb, nz, v};
use crate::render::characters::{mat, Sculpt};
use crate::render::voxels::{Collected, PartDef};

mod pal {
    use super::mat;
    pub const SCALE: u32 = 0x24443a | mat::SKIN;
    pub const SCALE_L: u32 = 0x386a58 | mat::SKIN;
    pub const SCALE_D: u32 = 0x14261f | mat::SKIN;
    pub const SHEEN: u32 = 0x34807a | mat::SKIN;
    pub const RIDGE: u32 = 0x0c1814 | mat::SKIN;
    pub const BELLY: u32 = 0x8e9a76 | mat::IVORY;
    pub const BELLY_L: u32 = 0xa8b28a | mat::IVORY;
    pub const BELLY_D: u32 = 0x5e6a4e | mat::IVORY;
    pub const FIN: u32 = 0x1e5a52 | mat::SKIN;
    pub const FIN_L: u32 = 0x2f7a6a | mat::SKIN;
    pub const FIN_D: u32 = 0x123a36 | mat::SKIN;
    pub const SPINE: u32 = 0xd8ceaa | mat::IVORY;
    pub const SPINE_D: u32 = 0x9a9070 | mat::IVORY;
    pub const FANG: u32 = 0xeee6c8 | mat::IVORY;
    pub const MAW: u32 = 0x5a1418 | mat::SKIN;
    pub const MAW_D: u32 = 0x2a0a0c | mat::SKIN;
    pub const TONGUE: u32 = 0x8a2a30 | mat::SKIN;
    pub const EYE: u32 = 0xd8ff40 | mat::GLOW;
    pub const EYE_RIM: u32 = 0x8adf20 | mat::GLOW;
    pub const PUPIL: u32 = 0x060806 | mat::SKIN;
    pub const BARNACLE: u32 = 0xa8a294 | mat::IVORY;
    pub const BARNACLE_D: u32 = 0x6a665a | mat::IVORY;
    pub const KRAIT: u32 = 0x3aa898 | mat::SKIN;
    pub const KRAIT_L: u32 = 0x5ac8b0 | mat::SKIN;
    pub const KRAIT_BAND: u32 = 0x10161a | mat::SKIN;
    pub const KRAIT_BELLY: u32 = 0xc8d0a0 | mat::IVORY;
}
use pal::*;

/// One voxel of a swept body: where it is, its nearest point on the axis, how far along the body
/// that point lies (in voxels), the way the axis runs there, and the body's radius there.
struct Along {
    p: Vec3,
    c: Vec3,
    dist: f32,
    dir: Vec3,
    r: f32,
}

/// A smooth path through `ctrl` (centre, radius): a Catmull-Rom curve sampled `per` times between
/// each two points.
fn spline(ctrl: &[(Vec3, f32)], per: usize) -> Vec<(Vec3, f32)> {
    let n = ctrl.len();
    let mut out = Vec::new();
    for i in 0..n - 1 {
        let p0 = ctrl[i.saturating_sub(1)];
        let (p1, p2) = (ctrl[i], ctrl[i + 1]);
        let p3 = ctrl[(i + 2).min(n - 1)];
        for k in 0..per {
            let t = k as f32 / per as f32;
            let (t2, t3) = (t * t, t * t * t);
            let c = |a: f32, b: f32, c: f32, d: f32| {
                0.5 * (2.0 * b + (c - a) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2 + (3.0 * b - a - 3.0 * c + d) * t3)
            };
            let p = Vec3::new(c(p0.0.x, p1.0.x, p2.0.x, p3.0.x), c(p0.0.y, p1.0.y, p2.0.y, p3.0.y), c(p0.0.z, p1.0.z, p2.0.z, p3.0.z));
            out.push((p, p1.1 + (p2.1 - p1.1) * t));
        }
    }
    out.push(ctrl[n - 1]);
    out
}

/// A body swept along `knots`, each voxel coloured by `paint` (into empty voxels only); its length
/// is counted on from `start` and the end returned, so one body can be swept in pieces.
fn sweep(s: &mut Sculpt, knots: &[(Vec3, f32)], start: f32, paint: &dyn Fn(&Along) -> u32) -> f32 {
    let mut run = start;
    for w in knots.windows(2) {
        let ((a, ra), (b, rb)) = (w[0], w[1]);
        let ab = b - a;
        let len = ab.length().max(1e-4);
        let reach = ra.max(rb) + 1.0;
        let (lo, hi) = (a.min(b) - Vec3::splat(reach), a.max(b) + Vec3::splat(reach));
        for y in lo.y.floor() as i32..=hi.y.ceil() as i32 {
            for z in lo.z.floor() as i32..=hi.z.ceil() as i32 {
                for x in lo.x.floor() as i32..=hi.x.ceil() as i32 {
                    let p = Vec3::new(x as f32, y as f32, z as f32);
                    let t = ((p - a).dot(ab) / (len * len)).clamp(0.0, 1.0);
                    let c = a + ab * t;
                    let r = ra + (rb - ra) * t;
                    if (p - c).length_squared() <= r * r && s.get(x, y, z).is_none() {
                        s.set(x, y, z, paint(&Along { p, c, dist: run + t * len, dir: ab / len, r }));
                    }
                }
            }
        }
        run += len;
    }
    run
}

/// The point and direction `dist` voxels along `knots`.
fn at_length(knots: &[(Vec3, f32)], dist: f32) -> (Vec3, Vec3, f32) {
    let mut run = 0.0;
    for w in knots.windows(2) {
        let ((a, ra), (b, rb)) = (w[0], w[1]);
        let len = (b - a).length().max(1e-4);
        if dist <= run + len {
            let t = ((dist - run) / len).clamp(0.0, 1.0);
            return (a + (b - a) * t, (b - a) / len, ra + (rb - ra) * t);
        }
        run += len;
    }
    let n = knots.len();
    (knots[n - 1].0, (knots[n - 1].0 - knots[n - 2].0).normalize_or_zero(), knots[n - 1].1)
}

/// How long the path through `knots` is.
fn span(knots: &[(Vec3, f32)]) -> f32 {
    knots.windows(2).map(|w| (w[1].0 - w[0].0).length()).sum()
}

/**
 * Leviathan's hide about its axis: toward `belly` the pale belly, broad plates each with a dark
 * seam; beside them a dark seam; elsewhere green-black scales in a diamond lattice, each edged in
 * shadow, here and there one catching the deep's sheen, the ridge of the back darkest.
 */
fn serpent_hide(belly: Vec3) -> impl Fn(&Along) -> u32 {
    move |a| {
        let radial = (a.p - a.c).normalize_or_zero();
        let down = (belly - a.dir * belly.dot(a.dir)).normalize_or_zero();
        let b = radial.dot(down);
        let (x, y, z) = (a.p.x as i32, a.p.y as i32, a.p.z as i32);
        if b > 0.7 {
            let k = (a.dist / 3.2).rem_euclid(1.0);
            return if k < 0.22 {
                BELLY_D
            } else if k > 0.8 {
                BELLY_L
            } else {
                BELLY
            };
        }
        if b > 0.55 {
            return SCALE_D;
        }
        if b < -0.88 {
            return RIDGE;
        }
        let around = radial.dot(a.dir.cross(down)).atan2(b) * a.r / 3.4;
        let along = a.dist / 3.4;
        let (d1, d2) = ((along + around).rem_euclid(1.0), (along - around).rem_euclid(1.0));
        if d1 < 0.18 || d2 < 0.18 {
            SCALE_D
        } else if (d1 - 0.6).abs() < 0.15 && (d2 - 0.6).abs() < 0.15 && nz(x, y, z) < 0.5 {
            if nz(x / 3, y / 3, z / 3) < 0.35 {
                SHEEN
            } else {
                SCALE_L
            }
        } else {
            SCALE
        }
    }
}

/// A spine of bone from `base` along `dir`, `len` long and `r` thick at its root, yellowing toward
/// its point; returns the point.
fn spine(s: &mut Sculpt, base: Vec3, dir: Vec3, len: f32, r: f32) -> Vec3 {
    let tip = base + dir.normalize() * len;
    let mid = base + (tip - base) * 0.6;
    limb(s, &[(base, r), (mid, r * 0.55)], SPINE);
    limb(s, &[(mid, r * 0.55), (tip, 0.35)], SPINE_D);
    tip
}

/// A web of fin between two spines (`a` and `b`, each from its root to its point), its free edge
/// scalloped between their points, ribbed light and dark.
fn web(s: &mut Sculpt, a: (Vec3, Vec3), b: (Vec3, Vec3)) {
    let m = ((b.0 - a.0).length().max((b.1 - a.1).length()) * 2.5) as i32 + 4;
    let n = ((a.1 - a.0).length().max((b.1 - b.0).length()) * 2.5) as i32 + 4;
    for i in 0..=m {
        let u = i as f32 / m as f32;
        let (root, top) = (a.0 + (b.0 - a.0) * u, a.1 + (b.1 - a.1) * u);
        let sag = 1.0 - 0.3 * (u * PI).sin();
        for j in 0..=n {
            let w = j as f32 / n as f32;
            if w > sag {
                break;
            }
            let p = root + (top - root) * w;
            let (x, y, z) = (p.x.round() as i32, p.y.round() as i32, p.z.round() as i32);
            if s.get(x, y, z).is_some() {
                continue;
            }
            let rib = (u * 6.0).fract() < 0.22;
            s.set(
                x,
                y,
                z,
                if rib {
                    FIN_D
                } else if w > sag - 0.12 {
                    FIN_L
                } else {
                    FIN
                },
            );
        }
    }
}

/// A fan of fin from `root`: spines spread from `from` to `to` (directions), `len` long, webbed.
fn fan(s: &mut Sculpt, root: Vec3, from: Vec3, to: Vec3, len: f32, n: usize, r: f32) {
    let mut prev: Option<(Vec3, Vec3)> = None;
    for k in 0..n {
        let u = k as f32 / (n - 1).max(1) as f32;
        let dir = from.lerp(to, u).normalize();
        let l = len * (0.75 + 0.25 * (u * PI).sin());
        let base = root + (to - from) * (u - 0.5) * 2.0;
        let tip = spine(s, base, dir, l, r);
        if let Some(p) = prev {
            web(s, p, (base, tip));
        }
        prev = Some((base, tip));
    }
}

/// A cluster of barnacles crusting the hide at `c`.
fn barnacles(s: &mut Sculpt, c: Vec3, n: usize, seed: f32) {
    for k in 0..n {
        let a = seed + k as f32 * 2.4;
        let p = c + Vec3::new(a.cos() * (1.0 + (k % 3) as f32), (k % 2) as f32, a.sin() * (1.0 + (k % 3) as f32));
        ball(s, p, Vec3::new(1.3, 1.0, 1.3), BARNACLE);
        s.set(p.x.round() as i32, (p.y + 1.0).round() as i32, p.z.round() as i32, BARNACLE_D);
    }
}

/// Leviathan's grid and voxel: about three tiles tall, as Mammon is (and no larger than his).
pub const LEVIATHAN_SIZE: (i32, i32, i32) = (100, 124, 80);
pub const LEVIATHAN_VOXEL: f32 = 0.025;
const LX: f32 = 50.0;
const LZ: f32 = 40.0;
/// His head is sculpted at this size of its numbers.
const HK: f32 = 1.25;

/// His neck where it rises off the body, his head on top of it, his jaw at its hinge, his tail
/// where it leaves the coil, his pectoral fins at his flanks.
pub const LEVIATHAN_RIG: &[PartDef] = &[
    PartDef { name: "neck", boxes: &[], pivot: [LX, 47.0, LZ - 3.0] },
    PartDef { name: "head", boxes: &[], pivot: [LX, 92.0, LZ + 4.0] },
    PartDef { name: "jaw", boxes: &[], pivot: [LX, 92.8, LZ + 5.6] },
    PartDef { name: "tail", boxes: &[], pivot: [30.0, 8.5, LZ + 14.0] },
    PartDef { name: "leftFin", boxes: &[], pivot: [LX - 11.0, 36.0, LZ + 2.0] },
    PartDef { name: "rightFin", boxes: &[], pivot: [LX + 11.0, 36.0, LZ + 2.0] },
];

pub fn leviathan() -> Collected {
    let (w, h, d) = LEVIATHAN_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let forward = Vec3::Z;

    // ---- the rising front third: an S off the floor, leaning forward to the head
    let rise = spline(
        &[
            (v(LX, 12.0, LZ + 6.0), 14.0),
            (v(LX, 28.0, LZ + 1.0), 12.8),
            (v(LX, 47.0, LZ - 5.0), 11.2),
            (v(LX, 64.0, LZ - 8.0), 9.8),
            (v(LX, 78.0, LZ - 6.0), 9.0),
            (v(LX, 87.0, LZ - 2.0), 8.6),
            (v(LX, 94.0, LZ + 2.0), 8.6),
        ],
        6,
    );
    // (The lower half is his body; from the neck's joint up, his neck.)
    let joint = rise.iter().position(|k| k.0.y >= 47.0).unwrap_or(rise.len() / 2);
    s.part(0);
    let run = sweep(&mut s, &rise[..=joint], 0.0, &serpent_hide(forward));
    s.part(1);
    sweep(&mut s, &rise[joint..], run, &serpent_hide(forward));

    // ---- the coil: back round his right, arching up once as if out of the water, round behind
    // and along his left, out into the tail
    let coil = spline(
        &[
            (v(LX, 12.0, LZ + 6.0), 14.0),
            (v(LX + 11.0, 12.0, LZ), 13.5),
            (v(LX + 19.0, 12.0, LZ - 9.0), 13.0),
            (v(LX + 20.0, 13.0, LZ - 18.0), 12.5),
            (v(LX + 13.0, 30.0, LZ - 25.0), 12.0),
            (v(LX + 2.0, 37.0, LZ - 27.0), 11.5),
            (v(LX - 9.0, 29.0, LZ - 25.0), 11.0),
            (v(LX - 18.0, 13.0, LZ - 18.0), 10.5),
            (v(LX - 23.0, 10.0, LZ - 7.0), 9.5),
            (v(LX - 23.0, 9.0, LZ + 5.0), 8.5),
            (v(30.0, 8.5, LZ + 14.0), 7.5),
        ],
        6,
    );
    s.part(0);
    let coil_len = sweep(&mut s, &coil, 0.0, &serpent_hide(-Vec3::Y));
    // The tail: on round his left and curling up, a fin along its last length.
    let tail = spline(
        &[
            (v(30.0, 8.5, LZ + 14.0), 7.5),
            (v(34.0, 9.0, LZ + 23.0), 6.0),
            (v(40.0, 11.0, LZ + 30.0), 4.5),
            (v(46.0, 16.0, LZ + 33.0), 2.6),
            (v(50.0, 22.0, LZ + 32.0), 1.4),
        ],
        5,
    );
    s.part(4);
    sweep(&mut s, &tail, coil_len, &serpent_hide(-Vec3::Y));
    let tail_len = span(&tail);
    for k in 0..7 {
        let at = tail_len * (0.25 + 0.1 * k as f32);
        let (p, dir, r) = at_length(&tail, at);
        let up = (Vec3::Y - dir * dir.y).normalize_or_zero();
        let base = p + up * (r - 0.5);
        let tip = base + (up * 1.0 - dir * 0.35).normalize() * (4.0 + 3.0 * (k as f32 * 0.6).sin().abs());
        limb(&mut s, &[(base, 0.9), (tip, 0.4)], SPINE_D);
        if k > 0 {
            let (pp, pdir, pr) = at_length(&tail, at - tail_len * 0.1);
            let pup = (Vec3::Y - pdir * pdir.y).normalize_or_zero();
            let pbase = pp + pup * (pr - 0.5);
            let ptip = pbase + (pup - pdir * 0.35).normalize() * (4.0 + 3.0 * ((k - 1) as f32 * 0.6).sin().abs());
            web(&mut s, (pbase, ptip), (base, tip));
        }
    }
    // Barnacles crusting the old coil, where the water never reaches.
    s.part(0);
    for (k, at) in [0.18f32, 0.33, 0.47, 0.62, 0.8].into_iter().enumerate() {
        let (p, dir, r) = at_length(&coil, coil_len * at);
        let side = dir.cross(Vec3::Y).normalize_or_zero() * if k % 2 == 0 { 1.0 } else { -1.0 };
        let out = (Vec3::Y * 0.8 + side * 0.6).normalize();
        barnacles(&mut s, p + out * (r - 0.6), 4 + k % 3, k as f32 * 1.7);
    }

    // ---- the dorsal crest: spines webbed down the back of his neck and body
    let total = span(&rise);
    let mut prev: Option<(Vec3, Vec3)> = None;
    for k in 0..10 {
        let at = total * (0.3 + 0.065 * k as f32);
        let (p, dir, r) = at_length(&rise, at);
        let back = (-forward - dir * (-forward).dot(dir)).normalize_or_zero();
        let base = p + back * (r - 1.0);
        let len = 4.5 + 4.0 * (k as f32 / 9.0 * PI).sin();
        s.part(if p.y >= 47.0 { 1 } else { 0 });
        let tip = spine(&mut s, base, back * 1.0 + Vec3::Y * 0.3 - dir * 0.45, len, 1.1);
        if let Some(q) = prev {
            web(&mut s, q, (base, tip));
        }
        prev = Some((base, tip));
    }

    // ---- the pectoral fins, spread from his flanks
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(5 + i as u8);
        let root = v(LX + side * 11.0, 36.0, LZ + 2.0);
        fan(&mut s, root, v(side, 0.35, 0.35), v(side, -0.5, -0.4), 16.0, 5, 1.1);
    }

    // ---- the head: a long finned skull, heavy brows, the snout and its barbels (sculpted at
    // `HK` the size of its numbers)
    s.part(2);
    let hc = v(LX, 100.0, LZ + 1.0);
    let at = |x: f32, y: f32, z: f32| hc + v(x, y, z) * HK;
    ball(&mut s, hc, v(12.5, 9.5, 13.0) * HK, SCALE);
    ball(&mut s, at(0.0, -3.0, -6.0), v(11.0, 9.0, 9.0) * HK, SCALE);
    // The snout: long, flattened, tapering to the nostrils.
    limb(&mut s, &[(at(0.0, -1.0, 8.0), 9.0 * HK), (at(0.0, -3.0, 16.0), 7.0 * HK), (at(0.0, -4.5, 24.0), 5.0 * HK)], SCALE);
    let lip = (hc.z + 8.0 * HK) as i32;
    s.paint(|x, y, z, c| {
        if c != SCALE {
            return None;
        }
        // A tight lattice of small scales over the skull, darker on top, lighter at the lip.
        let n = nz(x, y, z);
        Some(if (x + z) % 3 == 0 && n < 0.6 {
            SCALE_D
        } else if (y as f32) < hc.y - 3.0 && z > lip {
            SCALE_L
        } else if n > 0.93 {
            SHEEN
        } else {
            SCALE
        })
    });
    // Heavy brow ridges over the eyes, sweeping back into the crest.
    for side in [-1.0f32, 1.0] {
        limb(
            &mut s,
            &[(at(side * 7.5, 6.0, 9.0), 3.2 * HK), (at(side * 9.0, 7.5, 1.0), 3.0 * HK), (at(side * 8.0, 8.0, -7.0), 2.2 * HK)],
            SCALE_D,
        );
        // The barbels: trailing from the snout's sides down and back.
        let b0 = at(side * 4.5, -6.0, 21.0);
        limb(&mut s, &[(b0, 1.4), (b0 + v(side * 3.0, -7.0, -2.0), 1.1), (b0 + v(side * 5.0, -15.0, -7.0), 0.6)], FIN_L);
        // Nostrils.
        let n = at(side * 2.5, -2.0, 27.0);
        s.set(n.x.round() as i32, n.y.round() as i32, n.z.round() as i32, MAW_D);
        s.set(n.x.round() as i32, n.y.round() as i32, n.z.round() as i32 - 1, MAW_D);
    }
    // The eyes: set into the sides of the skull under the brows, burning yellow-green, slit black.
    let eye = at(0.0, 3.0, 8.0);
    for side in [-1i32, 1] {
        let (ey, ez) = (eye.y.round() as i32, eye.z.round() as i32);
        for dy in -2..=2i32 {
            for dz in -3..=3i32 {
                if dy.abs() == 2 && dz.abs() >= 2 {
                    continue;
                }
                let (y, z) = (ey + dy, ez + dz);
                let x =
                    if side < 0 { (0..w).find(|&x| s.get(x, y, z).is_some()) } else { (0..w).rev().find(|&x| s.get(x, y, z).is_some()) };
                let Some(x) = x else { continue };
                let c = if dz == 0 && dy.abs() <= 1 {
                    PUPIL
                } else if dy.abs() == 2 || dz.abs() == 3 {
                    EYE_RIM
                } else {
                    EYE
                };
                s.recolor(x, y, z, c);
            }
        }
    }
    // The upper jaw's fangs, a row down each side, the longest at the front.
    for side in [-1.0f32, 1.0] {
        for k in 0..8 {
            let t = k as f32 / 7.0;
            let base = at(side * (6.5 - 2.8 * t), -6.5 + 1.5 * t, 6.0 + 17.0 * t);
            let long = if k == 6 { 6.0 } else { 2.6 + t * 1.4 };
            limb(&mut s, &[(base, 1.0), (base + v(0.0, -long, 0.4), 0.3)], FANG);
        }
    }
    // The crest: a tall fan of spines from the brow back, the longest over the crown.
    let mut prev: Option<(Vec3, Vec3)> = None;
    for k in 0..8 {
        let u = k as f32 / 7.0;
        let base = at(0.0, 8.5 - u * 4.0, 8.0 - u * 18.0);
        let len = 9.0 + 8.0 * (u * PI).sin();
        let tip = spine(&mut s, base, v(0.0, 1.0 - u * 0.4, -0.35 - u * 0.8), len, 1.7);
        if let Some(q) = prev {
            web(&mut s, q, (base, tip));
        }
        prev = Some((base, tip));
    }
    // The gill fins: great fans either side of the jaw hinge, sweeping back and out.
    for side in [-1.0f32, 1.0] {
        let root = at(side * 11.0, -2.0, -3.0);
        fan(&mut s, root, v(side * 0.55, 0.95, -0.4), v(side * 0.9, -0.5, -0.55), 19.0, 6, 1.3);
    }

    // ---- the jaw: hanging open, ivory fangs up from it, the red maw and tongue within
    s.part(3);
    let hinge = at(0.0, -6.0, 3.0);
    let jt = |x: f32, y: f32, z: f32| hinge + v(x, y, z) * HK;
    limb(&mut s, &[(jt(0.0, -1.0, -2.0), 7.5 * HK), (jt(0.0, -5.0, 9.0), 6.0 * HK), (jt(0.0, -9.0, 19.0), 3.8 * HK)], SCALE_D);
    let jaw = |s: &Sculpt, x: i32, y: i32, z: i32| s.tag_at(x, y, z) == 3;
    let mut paints = Vec::new();
    s.paint(|x, y, z, c| {
        paints.push((x, y, z, c));
        None
    });
    for (x, y, z, c) in paints {
        if c != SCALE_D || !jaw(&s, x, y, z) {
            continue;
        }
        // Its underside pale, as his belly.
        let under = (y as f32) < hinge.y - 6.0 - (z as f32 - hinge.z) / 2.6;
        let c = if under {
            if (z / 3) % 2 == 0 {
                BELLY
            } else {
                BELLY_D
            }
        } else if nz(x, y, z) < 0.3 {
            SCALE
        } else {
            SCALE_D
        };
        s.recolor(x, y, z, c);
    }
    // The maw: the jaw's top hollowed into dark red, a forked tongue lying in it.
    let reach = 21.0 * HK;
    for z in hinge.z as i32..=(hinge.z + reach) as i32 {
        let t = (z as f32 - hinge.z) / reach;
        let half = ((5.0 - t * 3.0) * HK).max(1.0) as i32;
        let top = (hinge.y - 1.0 - t * 8.5 * HK).round() as i32;
        for x in LX as i32 - half..=LX as i32 + half {
            for y in top - 1..=top + 1 {
                if s.get(x, y, z).is_some() && jaw(&s, x, y, z) {
                    s.recolor(x, y, z, if y == top + 1 { MAW_D } else { MAW });
                }
            }
        }
    }
    limb(&mut s, &[(jt(0.0, -1.5, 4.0), 1.6), (jt(0.0, -6.0, 16.0), 1.1)], TONGUE);
    for side in [-1.0f32, 1.0] {
        limb(&mut s, &[(jt(0.0, -6.0, 16.0), 0.9), (jt(side * 2.0, -7.0, 21.0), 0.4)], TONGUE);
        for k in 0..6 {
            let t = k as f32 / 5.0;
            let base = jt(side * (5.0 - 2.5 * t), -2.5 - 6.0 * t, 4.0 + 14.0 * t);
            let long = if k == 5 { 5.0 } else { 2.0 + t * 1.5 };
            limb(&mut s, &[(base, 0.9), (base + v(0.0, long, 0.3), 0.3)], FANG);
        }
    }

    s.finish(0.03)
}

/// The serpentling's grid and voxel: a sea snake reared up a tile and a half.
pub const SERPENTLING_SIZE: (i32, i32, i32) = (44, 62, 44);
pub const SERPENTLING_VOXEL: f32 = 0.025;
const SX: f32 = 22.0;
const SZ: f32 = 22.0;

/// The neck where it rears off the coil, the head on it, the paddle of a tail.
pub const SERPENTLING_RIG: &[PartDef] = &[
    PartDef { name: "neck", boxes: &[], pivot: [SX, 8.0, SZ - 1.0] },
    PartDef { name: "head", boxes: &[], pivot: [SX, 50.0, SZ + 3.0] },
    PartDef { name: "tail", boxes: &[], pivot: [SX - 10.0, 3.0, SZ + 8.0] },
];

/// A sea krait's hide: teal with black bands round it, the belly pale.
fn krait(a: &Along) -> u32 {
    let radial = (a.p - a.c).normalize_or_zero();
    let down = (Vec3::Z - a.dir * Vec3::Z.dot(a.dir)).normalize_or_zero();
    let band = (a.dist / 6.5).rem_euclid(1.0) < 0.38;
    if band {
        KRAIT_BAND
    } else if radial.dot(down) > 0.65 {
        KRAIT_BELLY
    } else if nz(a.p.x as i32, a.p.y as i32, a.p.z as i32) < 0.2 {
        KRAIT_L
    } else {
        KRAIT
    }
}

pub fn serpentling() -> Collected {
    let (w, h, d) = SERPENTLING_SIZE;
    let mut s = Sculpt::new(w, h, d);
    // The coil on the floor, round once.
    s.part(0);
    let ring: Vec<(Vec3, f32)> = (0..=24)
        .map(|k| {
            let a = k as f32 / 24.0 * TAU * 0.9 + PI * 0.6;
            (v(SX + a.cos() * 11.0, 4.0, SZ + a.sin() * 11.0), 4.2)
        })
        .collect();
    let run = sweep(&mut s, &ring, 0.0, &krait);
    // Reared up off it: the neck, leaning forward to the head.
    s.part(1);
    let neck =
        spline(&[(v(SX, 6.0, SZ - 2.0), 4.4), (v(SX, 22.0, SZ - 4.0), 4.0), (v(SX, 38.0, SZ - 2.0), 3.6), (v(SX, 48.0, SZ + 2.0), 3.4)], 5);
    sweep(&mut s, &neck, run, &krait);
    // The tail: a flat paddle trailing off the coil.
    s.part(3);
    let root = ring[0].0;
    limb(&mut s, &[(root, 4.0), (root + v(-3.0, 0.0, 6.0), 3.0)], KRAIT);
    for k in 0..10 {
        let t = k as f32 / 9.0;
        let p = root + v(-3.0 - t * 3.0, 0.0, 6.0 + t * 6.0);
        for dy in -1..=(4.0 * (1.0 - t * 0.4)) as i32 {
            let c = if k % 3 == 0 { KRAIT_BAND } else { KRAIT_L };
            s.set(p.x.round() as i32, (p.y + dy as f32).round() as i32, p.z.round() as i32, c);
            s.set(p.x.round() as i32 + 1, (p.y + dy as f32).round() as i32, p.z.round() as i32, c);
        }
    }
    // The head: small and flat, a little fin over it, the eyes burning as his.
    s.part(2);
    let hc = v(SX, 51.0, SZ + 6.0);
    ball(&mut s, hc, v(4.6, 3.6, 6.0), KRAIT);
    limb(&mut s, &[(hc + v(0.0, -0.5, 4.0), 3.6), (hc + v(0.0, -1.5, 8.5), 2.2)], KRAIT);
    limb(&mut s, &[(hc + v(0.0, -2.5, 2.0), 3.0), (hc + v(0.0, -3.0, 8.0), 1.8)], KRAIT_BELLY);
    for k in 0..5 {
        let u = k as f32 / 4.0;
        let base = hc + v(0.0, 3.0 - u, 3.0 - u * 9.0);
        limb(&mut s, &[(base, 0.6), (base + v(0.0, 3.5 + 2.0 * (u * PI).sin(), -1.5), 0.3)], FIN_L);
    }
    for side in [-1i32, 1] {
        let x = SX as i32 + side * 4;
        for (y, z) in [(52, (SZ + 8.0) as i32), (52, (SZ + 9.0) as i32), (53, (SZ + 8.0) as i32)] {
            let x = if side < 0 {
                (0..w).find(|&x| s.get(x, y, z).is_some()).unwrap_or(x)
            } else {
                (0..w).rev().find(|&x| s.get(x, y, z).is_some()).unwrap_or(x)
            };
            s.recolor(x, y, z, EYE);
        }
    }
    let _ = (h, d);
    s.finish(0.03)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::voxels::Voxel;

    fn m_voxels(c: &Collected) -> Vec<Voxel> {
        c.voxels.clone()
    }

    fn m_count(v: &[Voxel], m: u32) -> usize {
        v.iter().filter(|v| v.c & 0xff00_0000 == m).count()
    }

    #[test]
    fn leviathan_rises_over_the_hero_and_his_serpentlings_are_small_banded_snakes() {
        let m = leviathan();
        let top = m.voxels.iter().map(|v| v.y).max().unwrap() as f32 * LEVIATHAN_VOXEL;
        assert!(top > 2.6 && top < 3.4, "{top} tiles tall");
        assert!(m.voxels.len() > 8000, "{} voxels", m.voxels.len());
        for part in 0..=LEVIATHAN_RIG.len() as u8 {
            assert!(m.voxels.iter().any(|v| v.part == part), "part {part}");
        }
        // Burning eyes; ivory fangs, spines and belly on green-black scales.
        let of = |m: u32| m_count(&m_voxels(&leviathan()), m);
        assert!(of(mat::GLOW) >= 12, "his eyes");
        assert!(of(mat::IVORY) > 600, "fangs, spines and his belly");
        assert!(of(mat::SKIN) > of(mat::IVORY) * 2, "most of him scaled");
        let s = serpentling();
        let top = s.voxels.iter().map(|v| v.y).max().unwrap() as f32 * SERPENTLING_VOXEL;
        assert!(top > 1.1 && top < 1.8, "{top}");
        assert!(m_count(&s.voxels, mat::GLOW) >= 4, "its eyes");
        for part in 0..=SERPENTLING_RIG.len() as u8 {
            assert!(s.voxels.iter().any(|v| v.part == part), "part {part}");
        }
    }
}
