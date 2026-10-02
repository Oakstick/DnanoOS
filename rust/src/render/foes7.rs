//! The seventh stage's cast, the last of the bestiary: Ragnarök, four beings of the old north,
//! sculpted as finely as the third's (see `foes3`). The frost giant, a hrímþurs of ice-blue
//! muscle, his long white beard plaited and hung with icicles, crystals of ice jutting from his
//! shoulders and back, his fists clenched; Surtr, the fire giant of Muspelheim, black basalt
//! cracked with glowing magma under a mane and beard of fire, a flaming greatsword held low in his
//! right hand; Níðhöggr, the serpent-dragon that gnaws at the roots of the World Tree, coiled among
//! them, its horned head reared up on an S of a neck and its fangs dripping venom; and Hel, ruler of
//! the dead, tall in layered black under a crown of thorns, her face half living and half rotted to
//! the skull, a spiked crescent staff in her right hand. Each faces +z, stands in the middle of its
//! grid (where its rig sets it down), and is sculpted in the parts its rig swings.
//!
//! Between them they fight all three ways (COMBAT.md §10): the frost giant's blows chill you, and
//! chilled twice you freeze solid; Surtr's flaming sword leaves the ground burning behind it (both
//! melee); Níðhöggr burrows under the ground and bursts up beneath you, and its bite is venom
//! (body); Hel's bolts drain your life into hers, and she raises the dead (distance).
//!
//! The giants tower over Ragnar at no greater cost than a man-sized foe: they are built at a
//! coarser voxel with the same detail to a voxel, 0.02 world units for the two giants, 0.0175 for
//! Níðhöggr and 0.0135 for Hel (the third stage's foes are 0.0125).

use std::f32::consts::{PI, TAU};

use glam::Vec3;

use super::carve::*;
use super::characters::{mat, Sculpt};
use super::voxels::{Collected, PartDef};

mod pal {
    use super::mat;
    pub const CAVITY: u32 = 0x120e10 | mat::SKIN;
    pub const TOOTH: u32 = 0xeee6d4 | mat::IVORY;
    // The frost giant.
    pub const RIME: u32 = 0x88b0d6 | mat::SKIN;
    pub const RIME_L: u32 = 0xbcd8f0 | mat::SKIN;
    pub const RIME_D: u32 = 0x5e86b6 | mat::SKIN;
    pub const RIME_DD: u32 = 0x3e5e90 | mat::SKIN;
    pub const BLOOM: u32 = 0xd8eaf6 | mat::SKIN;
    pub const SNOW: u32 = 0xeef4f8 | mat::FUR;
    pub const SNOW_M: u32 = 0xd6e2ec | mat::FUR;
    pub const SNOW_S: u32 = 0xb4c6d6 | mat::FUR;
    pub const SNOW_D: u32 = 0x8ea4b8 | mat::FUR;
    pub const ICE: u32 = 0x9cd8ee | mat::SLIME;
    pub const ICE_L: u32 = 0xd8f4fc | mat::SLIME;
    pub const ICE_D: u32 = 0x5aa6cc | mat::SLIME;
    pub const ICE_GLOW: u32 = 0x3a92c0 | mat::GLOW;
    pub const ICE_CORE: u32 = 0x74cfee | mat::GLOW;
    pub const FROST_EYE: u32 = 0xc8f6ff | mat::GLOW;
    pub const PELT: u32 = 0x5e4e40 | mat::FUR;
    pub const PELT_D: u32 = 0x3e332a | mat::FUR;
    pub const PELT_L: u32 = 0x8c7a66 | mat::FUR;
    pub const HIDE: u32 = 0x8e6e4c | mat::LEATHER;
    pub const HIDE_D: u32 = 0x664c34 | mat::LEATHER;
    pub const HIDE_L: u32 = 0xa88a66 | mat::LEATHER;
    pub const BELT: u32 = 0x3a2c24 | mat::LEATHER;
    pub const BELT_L: u32 = 0x5a483c | mat::LEATHER;
    pub const BONE: u32 = 0xdcd2b8 | mat::IVORY;
    pub const BONE_D: u32 = 0xa89c80 | mat::IVORY;
    pub const NAIL: u32 = 0xcadcea | mat::IVORY;
    // Surtr.
    pub const BASALT: u32 = 0x342b28 | mat::SKIN;
    pub const BASALT_L: u32 = 0x5e4c42 | mat::SKIN;
    pub const BASALT_D: u32 = 0x2a1814 | mat::SKIN;
    pub const BASALT_DD: u32 = 0x1c0e0b | mat::SKIN;
    pub const SCORCH: u32 = 0x60261a | mat::SKIN;
    pub const MAGMA_R: u32 = 0xc83614 | mat::GLOW;
    pub const MAGMA: u32 = 0xff7418 | mat::GLOW;
    pub const MAGMA_Y: u32 = 0xffb63a | mat::GLOW;
    pub const MAGMA_W: u32 = 0xfff0b4 | mat::GLOW;
    pub const FIRE_R: u32 = 0xd8301a | mat::GLOW;
    pub const FIRE_O: u32 = 0xff6c16 | mat::GLOW;
    pub const FIRE_Y: u32 = 0xffb830 | mat::GLOW;
    pub const FIRE_W: u32 = 0xffeaa0 | mat::GLOW;
    pub const IRON: u32 = 0x3c3e44 | mat::METAL;
    pub const IRON_L: u32 = 0x6c7078 | mat::METAL;
    pub const IRON_D: u32 = 0x232428 | mat::METAL;
    pub const RIVET: u32 = 0x9a9ea6 | mat::METAL;
    pub const HORN_ROCK: u32 = 0x29211f | mat::IVORY;
    pub const HORN_ROCK_L: u32 = 0x524640 | mat::IVORY;
    pub const GRIP: u32 = 0x2c1d16 | mat::LEATHER;
    pub const GRIP_L: u32 = 0x4e3628 | mat::LEATHER;
    pub const FANG: u32 = 0xcab896 | mat::IVORY;
    // Níðhöggr.
    pub const SCALE: u32 = 0x27361f | mat::LEATHER;
    pub const SCALE_D: u32 = 0x131b10 | mat::LEATHER;
    pub const SCALE_L: u32 = 0x42592f | mat::LEATHER;
    pub const BELLY: u32 = 0xa4aa44 | mat::IVORY;
    pub const BELLY_D: u32 = 0x6e7428 | mat::IVORY;
    pub const BELLY_L: u32 = 0xc2c45c | mat::IVORY;
    pub const SPINE: u32 = 0xd2c6a2 | mat::IVORY;
    pub const SPINE_D: u32 = 0x8a7e62 | mat::IVORY;
    pub const FRILL: u32 = 0x5e2826 | mat::LEATHER;
    pub const FRILL_D: u32 = 0x3e1818 | mat::LEATHER;
    pub const FRILL_L: u32 = 0x86443a | mat::LEATHER;
    pub const DRAKE_EYE: u32 = 0xd6ff3c | mat::GLOW;
    pub const DRAKE_EYE_D: u32 = 0x8ad01e | mat::GLOW;
    pub const GULLET: u32 = 0x3c1018 | mat::SKIN;
    pub const GUM: u32 = 0x6e2632 | mat::SKIN;
    pub const VENOM: u32 = 0x9ee83a | mat::SLIME;
    pub const VENOM_D: u32 = 0x5aa828 | mat::SLIME;
    pub const ROOT: u32 = 0x4c3c2e | mat::WOOD;
    pub const ROOT_D: u32 = 0x2e241c | mat::WOOD;
    pub const ROOT_L: u32 = 0x6e5842 | mat::WOOD;
    pub const GNAWED: u32 = 0xccac7c | mat::WOOD;
    pub const GNAWED_D: u32 = 0x94784e | mat::WOOD;
    // Hel.
    pub const ROBE: u32 = 0x1e1c26 | mat::CLOTH;
    pub const ROBE_D: u32 = 0x111016 | mat::CLOTH;
    pub const ROBE_L: u32 = 0x3e3a4c | mat::CLOTH;
    pub const GOWN: u32 = 0x2e2338 | mat::CLOTH;
    pub const GOWN_D: u32 = 0x1a1420 | mat::CLOTH;
    pub const GOWN_L: u32 = 0x4a3a5e | mat::CLOTH;
    pub const LINING: u32 = 0x4c1e50 | mat::CLOTH;
    pub const LINING_D: u32 = 0x28122a | mat::CLOTH;
    pub const LIVING: u32 = 0xe6dcd8 | mat::SKIN;
    pub const LIVING_D: u32 = 0xbcaeb0 | mat::SKIN;
    pub const LIP: u32 = 0x4a2436 | mat::SKIN;
    pub const DEAD: u32 = 0x313c56 | mat::SKIN;
    pub const DEAD_D: u32 = 0x1c2438 | mat::SKIN;
    pub const DEAD_L: u32 = 0x4a5874 | mat::SKIN;
    pub const SKULL: u32 = 0xd6ccb2 | mat::IVORY;
    pub const SKULL_D: u32 = 0x9c9076 | mat::IVORY;
    pub const HEL_EYE: u32 = 0x86e2ff | mat::GLOW;
    pub const HEL_EYE_W: u32 = 0xdaf8ff | mat::GLOW;
    pub const HAIR: u32 = 0x131117 | mat::FUR;
    pub const HAIR_L: u32 = 0x2c2834 | mat::FUR;
    pub const THORN: u32 = 0x1e1c24 | mat::METAL;
    pub const THORN_L: u32 = 0x46424e | mat::METAL;
    pub const SHAFT: u32 = 0x2a2220 | mat::WOOD;
    pub const SHAFT_L: u32 = 0x3e3430 | mat::WOOD;
    pub const DARK_IRON: u32 = 0x2e3038 | mat::METAL;
    pub const DARK_IRON_L: u32 = 0x5c606c | mat::METAL;
    pub const GEM: u32 = 0x5ccaf4 | mat::GLOW;
    pub const GEM_CORE: u32 = 0xdcf8ff | mat::GLOW;
    pub const GEM_D: u32 = 0x2a84b8 | mat::GLOW;
    pub const SHOE: u32 = 0x18161c | mat::LEATHER;
}
use pal::*;

// ------------------------------------------------------------------- helpers

/// Sets the voxel nearest `p`.
fn put(s: &mut Sculpt, p: Vec3, c: u32) {
    s.set(p.x.round() as i32, p.y.round() as i32, p.z.round() as i32, c);
}

/// Sets the voxel nearest `p` if it is empty: hair and beards lie over a body, never into it.
fn lay(s: &mut Sculpt, p: Vec3, c: u32) {
    let (x, y, z) = (p.x.round() as i32, p.y.round() as i32, p.z.round() as i32);
    if s.get(x, y, z).is_none() {
        s.set(x, y, z, c);
    }
}

/// A tapered capsule between two points (see `Sculpt::capsule`).
fn cap(s: &mut Sculpt, a: Vec3, b: Vec3, ra: f32, rb: f32, c: u32) {
    s.capsule((a.x, a.y, a.z), (b.x, b.y, b.z), ra, rb, c);
}

/// A tapered capsule like `cap`, filling only empty voxels: a lock of hair lying over what is there.
fn lay_cap(s: &mut Sculpt, a: Vec3, b: Vec3, ra: f32, rb: f32, c: u32) {
    let ab = b - a;
    let len2 = ab.length_squared().max(1e-6);
    let reach = Vec3::splat(ra.max(rb) + 1.0);
    for (x, y, z, q) in cells(a.min(b) - reach, a.max(b) + reach) {
        let t = ((q - a).dot(ab) / len2).clamp(0.0, 1.0);
        let r = ra + (rb - ra) * t;
        if (q - a - ab * t).length_squared() <= r * r && s.get(x, y, z).is_none() {
            s.set(x, y, z, c);
        }
    }
}

/// The backmost filled z at (x, y), if any: where hair lies down a back.
fn rear(s: &Sculpt, x: i32, y: i32, d: i32) -> Option<i32> {
    (0..d).find(|&z| s.get(x, y, z).is_some())
}

/// Two unit vectors at right angles to `dir` and to each other, the first level if it can be.
fn frame(dir: Vec3) -> (Vec3, Vec3) {
    let d = dir.normalize();
    let u = d.cross(if d.y.abs() > 0.95 { Vec3::X } else { Vec3::Y }).normalize();
    (u, u.cross(d))
}

/// Every voxel within the box `lo`..`hi`, as integer coordinates and as a point.
fn cells(lo: Vec3, hi: Vec3) -> impl Iterator<Item = (i32, i32, i32, Vec3)> {
    let (x0, y0, z0) = (lo.x.floor() as i32, lo.y.floor().max(0.0) as i32, lo.z.floor() as i32);
    let (x1, y1, z1) = (hi.x.ceil() as i32, hi.y.ceil() as i32, hi.z.ceil() as i32);
    (y0..=y1).flat_map(move |y| (z0..=z1).flat_map(move |z| (x0..=x1).map(move |x| (x, y, z, Vec3::new(x as f32, y as f32, z as f32)))))
}

/// A rounded box about `c` along the axes `ax` (unit, at right angles) with radii `r`: the higher
/// `p`, the squarer.
fn block(s: &mut Sculpt, c: Vec3, ax: [Vec3; 3], r: Vec3, p: f32, colour: u32) {
    let reach = Vec3::splat(r.max_element() + 1.0);
    for (x, y, z, q) in cells(c - reach, c + reach) {
        let d = q - c;
        let l = Vec3::new(d.dot(ax[0]) / r.x, d.dot(ax[1]) / r.y, d.dot(ax[2]) / r.z).abs();
        if l.x.powf(p) + l.y.powf(p) + l.z.powf(p) <= 1.0 {
            s.set(x, y, z, colour);
        }
    }
}

/// A muscle: a spheroid about `c`, `ra` long along `dir` and `rb` across it.
fn lump(s: &mut Sculpt, c: Vec3, dir: Vec3, ra: f32, rb: f32, colour: u32) {
    let d = dir.normalize();
    let reach = Vec3::splat(ra.max(rb) + 1.0);
    for (x, y, z, q) in cells(c - reach, c + reach) {
        let p = q - c;
        let a = p.dot(d);
        let b = (p - d * a).length();
        if (a / ra).powi(2) + (b / rb).powi(2) <= 1.0 {
            s.set(x, y, z, colour);
        }
    }
}

/**
 * An ice crystal from `base` along `dir`: a six-sided prism, each face `r` from its axis, `len` long, the last
 * third of it tapering to a point. Each face is painted lit, mid or shadowed by which way it
 * turns, the ridges between them bright; a glowing crystal is lit from within.
 */
fn crystal(s: &mut Sculpt, base: Vec3, dir: Vec3, len: f32, r: f32, glow: bool) {
    let d = dir.normalize();
    let (u, w) = frame(d);
    let tip = base + d * len;
    let reach = Vec3::splat(r + 1.0);
    let light = Vec3::new(-0.45, 0.8, 0.55).normalize();
    for (x, y, z, q) in cells(base.min(tip) - reach, base.max(tip) + reach) {
        let p = q - base;
        let a = p.dot(d);
        if !(-0.5..=len).contains(&a) {
            continue;
        }
        let rad = r * ((len - a) / (len * 0.34)).clamp(0.0, 1.0) + 0.35;
        let (pu, pw) = (p.dot(u), p.dot(w));
        let ang = pw.atan2(pu);
        let sector = (ang / (PI / 3.0)).floor();
        let mid = (sector + 0.5) * PI / 3.0;
        if pu * mid.cos() + pw * mid.sin() > rad {
            continue;
        }
        let n = u * mid.cos() + w * mid.sin();
        let ridge = (ang / (PI / 3.0) - sector - 0.5).abs() > 0.38;
        let lit = n.dot(light);
        let c = if glow {
            if a > len * 0.72 || ridge {
                ICE_CORE
            } else {
                ICE_GLOW
            }
        } else if ridge || a > len * 0.8 || lit > 0.25 {
            ICE_L
        } else if lit < -0.3 {
            ICE_D
        } else {
            ICE
        };
        s.set(x, y, z, c);
    }
}

/// An icicle hanging `len` from `top`, `r` thick at its root.
fn icicle(s: &mut Sculpt, top: Vec3, len: f32, r: f32) {
    let end = top - Vec3::Y * len;
    cap(s, top, end, r, 0.3, ICE);
    put(s, end, ICE_L);
    put(s, top - Vec3::Y * (len * 0.35), ICE_L);
}

/**
 * A crust of ice chunks round a limb from `a` to `b`, `ra` out from the axis at `a` and `rb` at
 * `b`: rings of nine facets, each ring turned half a facet from the last, each chunk standing out
 * its own way and painted lit, mid or shadowed by how it faces, bright along its edges.
 */
fn ice_crust(s: &mut Sculpt, a: Vec3, b: Vec3, ra: f32, rb: f32, seed: i32) {
    let axis = (b - a).normalize();
    let (u, w) = frame(axis);
    let len = (b - a).length();
    let step = TAU / 9.0;
    let light = Vec3::new(-0.45, 0.8, 0.55).normalize();
    let reach = Vec3::splat(ra.max(rb) + 2.0);
    for (x, y, z, q) in cells(a.min(b) - reach, a.max(b) + reach) {
        let p = q - a;
        let t = p.dot(axis) / len;
        if !(0.0..=1.0).contains(&t) {
            continue;
        }
        let radial = p - axis * (t * len);
        let ang = radial.dot(w).atan2(radial.dot(u)) + PI;
        let band = (t * len / 3.4).floor() as i32;
        let twist = if band % 2 == 0 { 0.0 } else { 0.5 };
        let sector = (ang / step + twist).floor();
        let mid = (sector - twist + 0.5) * step - PI;
        let chunk = nz(sector as i32 + seed, band, 7);
        let face_n = u * mid.cos() + w * mid.sin();
        if radial.dot(face_n) > ra + (rb - ra) * t + (chunk - 0.5) * 1.6 {
            continue;
        }
        let lit = face_n.dot(light) + (chunk - 0.5) * 0.5;
        let edge = (ang / step + twist).fract();
        let c = if !(0.12..=0.88).contains(&edge) || lit > 0.45 {
            ICE_L
        } else if lit < -0.2 {
            ICE_D
        } else {
            ICE
        };
        s.set(x, y, z, c);
    }
}

/**
 * Shades by the shape, as light pools in a sculpture's hollows: every open voxel of the `on`
 * colours is recoloured `f(voxel, colour, fill)`, where fill is how much of the ball of radius
 * `r` round it is filled: about 0.58 on a flat face (for r = 5), more in a crease between two
 * muscles, less on a ridge.
 */
fn hollows(s: &mut Sculpt, dims: (i32, i32, i32), r: i32, on: &[u32], f: impl Fn(Vec3, u32, f32) -> u32) {
    let (w, h, d) = dims;
    let mut offsets = vec![];
    for dz in -r..=r {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy + dz * dz <= r * r {
                    offsets.push((dx, dy, dz));
                }
            }
        }
    }
    let mut out = vec![];
    for y in 0..h {
        for z in 0..d {
            for x in 0..w {
                let Some(c) = s.get(x, y, z) else { continue };
                if !on.contains(&c) || !open(s, x, y, z) {
                    continue;
                }
                let filled = offsets.iter().filter(|&&(dx, dy, dz)| s.get(x + dx, y + dy, z + dz).is_some()).count();
                out.push((x, y, z, f(v(x as f32, y as f32, z as f32), c, filled as f32 / offsets.len() as f32)));
            }
        }
    }
    for (x, y, z, c) in out {
        s.recolor(x, y, z, c);
    }
}

/// The colour `t` of the way along `stops` (0 the first, 1 the last), blended channel by channel
/// and keeping the first stop's material.
fn ramp(stops: &[u32], t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
    let i = (t.floor() as usize).min(stops.len() - 2);
    let f = t - i as f32;
    let (a, b) = (stops[i], stops[i + 1]);
    let ch = |s: u32| {
        let (p, q) = (((a >> s) & 255) as f32, ((b >> s) & 255) as f32);
        ((p + (q - p) * f).round() as u32) << s
    };
    ch(16) | ch(8) | ch(0) | (a & 0xff00_0000)
}

/// Dusts the upturned faces of the `on` voxels above `above` with `c`, about `density` of them:
/// frost on a giant's shoulders.
fn frost(s: &mut Sculpt, dims: (i32, i32, i32), on: &[u32], above: i32, density: f32, c: u32) {
    let (w, h, d) = dims;
    let mut out = vec![];
    for y in above.max(0)..h {
        for z in 0..d {
            for x in 0..w {
                if s.get(x, y, z).is_some_and(|col| on.contains(&col)) && s.get(x, y + 1, z).is_none() && nz(x, y, z) < density {
                    out.push((x, y, z));
                }
            }
        }
    }
    for (x, y, z) in out {
        s.recolor(x, y, z, c);
    }
}

/// Smooth noise from 0 to 1, wandering over about `cell` voxels.
fn smooth(p: Vec3, cell: f32) -> f32 {
    let q = p / cell;
    let b = q.floor();
    let f = q - b;
    let f = f * f * (Vec3::splat(3.0) - 2.0 * f);
    let n = |dx: i32, dy: i32, dz: i32| nz(b.x as i32 + dx, b.y as i32 + dy, b.z as i32 + dz);
    let mix = |a: f32, c: f32, t: f32| a + (c - a) * t;
    let x0 = mix(mix(n(0, 0, 0), n(1, 0, 0), f.x), mix(n(0, 1, 0), n(1, 1, 0), f.x), f.y);
    let x1 = mix(mix(n(0, 0, 1), n(1, 0, 1), f.x), mix(n(0, 1, 1), n(1, 1, 1), f.x), f.y);
    mix(x0, x1, f.z)
}

/// How far `p` lies from a seam of a crust cracked into cells about `cell` voxels across: the gap
/// between its nearest and next nearest of the seeds jittered about a lattice (0 on a seam).
fn seam(p: Vec3, cell: f32) -> f32 {
    let base = (p / cell).floor();
    let (mut d1, mut d2) = (f32::MAX, f32::MAX);
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let c = base + Vec3::new(dx as f32, dy as f32, dz as f32);
                let (ix, iy, iz) = (c.x as i32, c.y as i32, c.z as i32);
                let jitter = Vec3::new(nz(ix, iy, iz), nz(ix + 31, iy + 17, iz + 7), nz(ix + 5, iy + 41, iz + 23));
                let dist = (p - (c + jitter) * cell).length();
                if dist < d1 {
                    (d1, d2) = (dist, d1);
                } else if dist < d2 {
                    d2 = dist;
                }
            }
        }
    }
    d2 - d1
}

/// Whether `c` is a flame's colour, and how hot (0 white-hot to 3 red).
fn heat(c: u32) -> Option<usize> {
    [FIRE_W, FIRE_Y, FIRE_O, FIRE_R].iter().position(|&f| f == c)
}

/**
 * A tongue of fire from `base`, rising `h` and leaning along `lean` the higher it goes, `r` across
 * at its root and narrowing to a wavering point: gold at its root, then orange, red at its tip. It
 * fills only empty voxels (or cooler flame), so it licks round what it rises from.
 */
fn flame(s: &mut Sculpt, base: Vec3, h: f32, r: f32, lean: Vec3, phase: f32) {
    let (side, _) = frame(Vec3::Y + lean);
    let n = (h * 1.6).ceil() as i32;
    for k in (0..=n).rev() {
        let t = k as f32 / n as f32;
        let wave = (t * 5.5 + phase).sin() * r * 0.5 * t;
        let c = base + Vec3::Y * (h * t) + lean * (h * t * t) + side * wave;
        let rad = r * (1.0 - t).max(0.0).powf(1.1) + 0.45;
        let hot = (1.0 - t) - 0.12 * nz(k, phase as i32, 3);
        let col = if hot > 0.95 {
            FIRE_W
        } else if hot > 0.72 {
            FIRE_Y
        } else if hot > 0.3 {
            FIRE_O
        } else {
            FIRE_R
        };
        let rank = heat(col).unwrap_or(3);
        let reach = Vec3::splat(rad + 1.0);
        for (x, y, z, q) in cells(c - reach, c + reach) {
            if (q - c).length_squared() > rad * rad {
                continue;
            }
            match s.get(x, y, z) {
                None => s.set(x, y, z, col),
                Some(old) if heat(old).is_some_and(|o| o > rank) => s.set(x, y, z, col),
                _ => {}
            }
        }
    }
}

// ------------------------------------------------------------- giants' bodies

/// A giant's proportions in voxels: his middle, his hips and shoulders, how thickset he is, where
/// his neck carries his head, and his skin.
struct Giant {
    cx: f32,
    cz: f32,
    /// The hip joints' height and half their spread.
    hip: (f32, f32),
    /// The shoulder joints' height and half their spread.
    shoulder: (f32, f32),
    /// Girth: 1 for the frost giant.
    g: f32,
    /// The top of his neck, under his skull.
    neck: Vec3,
    skin: u32,
}

/**
 * A giant's bare leg from his hip (see `Giant`) by `knee` to `ankle`: the thigh's quadriceps with the teardrop
 * above the knee, the hamstrings and the inner thigh, the kneecap, the shin and the calf's two
 * heads, the knobs of the ankle, the heel, the ball of the foot and five toes turned a little out,
 * nailed in `nail`. `out` is -1 for the left leg (lower x), +1 for the right.
 */
fn giant_leg(s: &mut Sculpt, b: &Giant, knee: Vec3, ankle: Vec3, out: f32, nail: u32) {
    let (g, skin) = (b.g, b.skin);
    let hip = v(b.cx + out * b.hip.1, b.hip.0, b.cz);
    let at = |a: Vec3, c: Vec3, t: f32| a + (c - a) * t;
    let (o, f) = (Vec3::X * out, Vec3::Z);
    let thigh = knee - hip;
    cap(s, hip, knee, 10.0 * g, 6.8 * g, skin);
    lump(s, at(hip, knee, 0.42) + f * 4.6 * g, thigh, 12.0 * g, 5.2 * g, skin);
    lump(s, at(hip, knee, 0.5) + (o * 4.8 + f * 1.0) * g, thigh, 13.0 * g, 5.4 * g, skin);
    lump(s, knee + (-o * 3.0 + Vec3::Y * 6.4 + f * 3.2) * g, thigh, 6.0 * g, 4.6 * g, skin);
    lump(s, at(hip, knee, 0.42) - f * 5.0 * g, thigh, 12.0 * g, 6.0 * g, skin);
    lump(s, at(hip, knee, 0.25) - o * 4.0 * g, thigh, 9.5 * g, 5.8 * g, skin);
    ball(s, knee + (Vec3::Y * 0.6 + f * 4.8) * g, v(3.6, 3.8, 2.6) * g, skin);
    let shin = ankle - knee;
    cap(s, knee, ankle, 6.6 * g, 4.0 * g, skin);
    lump(s, at(knee, ankle, 0.3) + (-o * 2.1 - f * 3.8) * g, shin, 9.6 * g, 4.3 * g, skin);
    lump(s, at(knee, ankle, 0.25) + (o * 2.1 - f * 3.6) * g, shin, 8.6 * g, 3.9 * g, skin);
    lump(s, at(knee, ankle, 0.32) + (o * 1.5 + f * 2.8) * g, shin, 9.5 * g, 2.8 * g, skin);
    for side in [-1.0f32, 1.0] {
        ball(s, ankle + v(side * 3.5, -0.6, 0.0) * g, Vec3::splat(1.8 * g), skin);
    }
    let k = 1.12 * g;
    ball(s, v(ankle.x, 4.0 * k, ankle.z - 2.6 * k), v(4.0, 4.0, 4.4) * k, skin);
    rounded(s, v(ankle.x + out * 0.4 * k, 3.1 * k, ankle.z + 4.4 * k), v(5.0, 3.2, 6.6) * k, 2.2, 0, skin);
    for i in 0..5 {
        let fi = i as f32;
        let len = [4.4f32, 3.8, 3.4, 3.0, 2.5][i] * k;
        let r = [1.9f32, 1.45, 1.35, 1.25, 1.1][i] * k;
        let base = v(ankle.x + out * (-3.3 + fi * 1.75) * k, 2.0 * k, ankle.z + (9.4 - fi * 0.55) * k);
        let tip = base + v(out * (0.15 + fi * 0.1) * len, -0.5 * k, len);
        cap(s, base, tip, r, r * 0.92, skin);
        put(s, tip + v(0.0, r * 0.6, 0.2 * k), nail);
    }
}

/**
 * A giant's bare trunk: the pelvis and the buttocks, six blocks of belly with the obliques beside
 * them, a barrel of ribs with the serratus on its flanks, the two slabs of the chest, the lats'
 * wings, the trapezius heaped up behind his neck, the shoulder blades and the two ridges either
 * side of the spine, and a neck like a tree stump.
 */
fn giant_trunk(s: &mut Sculpt, b: &Giant, d: i32) {
    let (cx, cz, g, skin) = (b.cx, b.cz, b.g, b.skin);
    let (hy, sy, span) = (b.hip.0, b.shoulder.0, b.shoulder.1);
    let p = |x: f32, y: f32, z: f32| v(cx + x * g, y, cz + z * g);
    let r = |x: f32, y: f32, z: f32| v(x * g, y, z * g);
    ball(s, p(0.0, hy, -0.5), r(16.5, 9.5, 11.0), skin);
    let (y0, y1) = ((hy - 2.0) as i32, (hy + 26.0) as i32);
    for y in y0..=y1 {
        let t = (y - y0) as f32 / (y1 - y0) as f32;
        slab(s, cx, cz + 0.3 * g, (15.0 + t * 2.5) * g, (10.8 + t * 1.2) * g, 2.2, y, y, skin);
    }
    ball(s, p(0.0, sy - 13.0, -0.4), r(21.0, 17.0, 13.4), skin);
    ball(s, p(0.0, sy - 2.0, -1.5), v(span - 2.0, 8.0, 11.0 * g), skin);
    ball(s, p(0.0, sy + 1.0, -8.0), r(13.0, 11.0, 6.5), skin);
    for side in [-1.0f32, 1.0] {
        ball(s, p(side * 7.5, hy - 2.5, -7.4), r(8.4, 9.0, 6.2), skin);
        cap(s, p(side * 2.6, sy - 8.0, 10.2), p(side * 16.0, sy - 4.0, 6.8), 7.0 * g, 5.8 * g, skin);
        ball(s, p(side * 9.2, sy - 7.0, 9.4), r(9.0, 6.6, 5.4), skin);
        ball(s, p(side * 14.5, hy + 18.0, 2.0), r(4.6, 9.0, 7.8), skin);
        for i in 0..3 {
            ball(s, p(side * 17.0, sy - 15.0 - i as f32 * 3.4, 5.0), r(2.6, 1.7, 2.8), skin);
        }
        ball(s, p(side * 14.5, sy - 13.0, -5.0), r(8.5, 14.0, 7.5), skin);
        cap(s, p(side * 3.5, sy + 9.0, -1.0), v(cx + side * (span - 6.0), sy + 2.0, cz - 2.5 * g), 6.6 * g, 4.6 * g, skin);
        ball(s, p(side * 10.0, sy - 8.0, -11.5), r(6.6, 8.0, 3.2), skin);
        cap(s, p(side * 3.4, hy + 4.0, -10.5), p(side * 3.4, sy - 14.0, -11.2), 3.2 * g, 3.2 * g, skin);
    }
    // The belly's six blocks, standing out of it, and the V below them.
    for dy in [17.0f32, 23.5, 30.0] {
        for side in [-1.0f32, 1.0] {
            let (x, y) = (cx + side * 3.6 * g, sy - dy);
            let z = front(s, x.round() as i32, y.round() as i32, d).unwrap_or(0) as f32;
            ball(s, v(x, y, z - 1.2), r(3.5, 3.0, 2.5), skin);
        }
    }
    let z = front(s, cx as i32, (hy + 9.0) as i32, d).unwrap_or(0) as f32;
    ball(s, v(cx, hy + 9.0, z - 1.4), r(5.6, 4.4, 2.5), skin);
    cap(s, p(0.0, sy - 2.0, 0.0), b.neck, 7.4 * g, 6.8 * g, skin);
}

/**
 * A giant's bare arm from `shoulder` by `elbow` to `wrist`: the deltoid capping the shoulder, the
 * biceps before and the triceps behind, the knob of the elbow, the forearm thick with muscle below
 * the elbow and narrowing to the wrist. `out` is -1 for the left arm (lower x), +1 for the right.
 */
fn giant_arm(s: &mut Sculpt, b: &Giant, shoulder: Vec3, elbow: Vec3, wrist: Vec3, out: f32) {
    let (g, skin) = (b.g, b.skin);
    let at = |a: Vec3, c: Vec3, t: f32| a + (c - a) * t;
    let (o, f) = (Vec3::X * out, Vec3::Z);
    let (upper, fore) = (elbow - shoulder, wrist - elbow);
    ball(s, shoulder + (o * 2.2 + Vec3::Y * 0.8) * g, v(8.6, 9.6, 9.2) * g, skin);
    cap(s, shoulder, elbow, 7.4 * g, 5.8 * g, skin);
    lump(s, at(shoulder, elbow, 0.55) + (-o * 0.4 + f * 3.4) * g, upper, 8.6 * g, 5.3 * g, skin);
    lump(s, at(shoulder, elbow, 0.42) + (o * 0.9 - f * 3.6) * g, upper, 10.0 * g, 5.6 * g, skin);
    lump(s, at(shoulder, elbow, 0.78) + (o * 3.0 + f * 0.6) * g, upper, 5.0 * g, 3.3 * g, skin);
    let fd = fore.normalize();
    let fz = (Vec3::Z - fd * fd.z).normalize();
    ball(s, elbow - fz * 3.9 * g, Vec3::splat(3.0 * g), skin);
    cap(s, elbow, wrist, 6.4 * g, 4.4 * g, skin);
    lump(s, at(elbow, wrist, 0.22) + (o * 1.9 + fz * 0.9) * g, fore, 7.4 * g, 5.4 * g, skin);
    lump(s, at(elbow, wrist, 0.25) + (-o * 1.9 + fz * 0.5) * g, fore, 7.0 * g, 4.8 * g, skin);
}

/**
 * A giant's clenched fist below `wrist`, the forearm coming down from `elbow`, built to scale
 * `k`: the broad back of the hand turned forward, four knuckles standing out in a row across its
 * foot, each finger curled down and back under the palm, the thumb along the inside and across
 * them. Returns the knuckles, from the inside out, and the fist's axes (outward, down the arm,
 * forward).
 */
fn giant_fist(s: &mut Sculpt, elbow: Vec3, wrist: Vec3, out: f32, k: f32, skin: u32) -> ([Vec3; 4], [Vec3; 3]) {
    let dn = (wrist - elbow).normalize();
    let f = (Vec3::Z - dn * dn.z).normalize();
    let o = f.cross(dn) * out;
    let c = wrist + dn * 3.6 * k;
    block(s, c, [o, dn, f], v(4.3, 3.6, 3.0) * k, 2.4, skin);
    let mut knuckles = [Vec3::ZERO; 4];
    for (i, kn) in knuckles.iter_mut().enumerate() {
        *kn = c + (o * (-3.0 + i as f32 * 2.0) + dn * 3.3 + f * 2.2) * k;
        ball(s, *kn, v(1.05, 1.2, 1.25) * k, skin);
        let bend = *kn + (dn * 2.3 - f * 0.5) * k;
        cap(s, *kn, bend, 0.95 * k, 0.9 * k, skin);
        cap(s, bend, *kn + (dn * 2.0 - f * 3.9) * k, 0.9 * k, 0.85 * k, skin);
    }
    cap(s, c + (-o * 4.3 - dn * 1.2 - f * 0.6) * k, c + (-o * 3.4 + dn * 3.9 - f * 1.6) * k, 1.3 * k, 1.1 * k, skin);
    (knuckles, [o, dn, f])
}

/// Shades a giant's `skin` along `tones` (light, base, dark, deep) by the hollows of his muscles,
/// darkening it toward his feet as if the light fell from above (full at `top`).
fn shade_skin(s: &mut Sculpt, dims: (i32, i32, i32), skin: u32, tones: [u32; 4], top: f32) {
    let [light, base, dark, deep] = tones;
    hollows(s, dims, 5, &[skin], |p, _, fill| {
        let c = ramp(&[light, base, base, dark, deep], (fill - 0.46) / 0.27);
        scale(c, 0.8 + 0.2 * (p.y / top).min(1.0))
    });
}

// ---------------------------------------------------------------- frost giant

/// The frost giant's grid: 96 wide (his fists and crystals), 144 tall, 60 deep.
pub const FROST_GIANT_SIZE: (i32, i32, i32) = (96, 144, 60);
const FG: (f32, f32) = (47.5, 29.5);

/// Hips, shoulders and the base of the neck (his head carried forward of it).
pub const FROST_GIANT_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [FG.0 - 10.5, 60.0, FG.1] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [FG.0 + 10.5, 60.0, FG.1] },
    PartDef { name: "leftArm", boxes: &[], pivot: [FG.0 - 26.0, 107.0, FG.1 - 1.0] },
    PartDef { name: "rightArm", boxes: &[], pivot: [FG.0 + 26.0, 107.0, FG.1 - 1.0] },
    PartDef { name: "head", boxes: &[], pivot: [FG.0, 112.0, FG.1 + 2.5] },
];

/// A plait of hair along `path`, `r` thick: lobes crossing left and right, ringed with ice at
/// each of `rings` (fractions along it), and an icicle hanging from its end.
fn braid(s: &mut Sculpt, path: &[Vec3], r: f32, rings: &[f32]) {
    let total: f32 = path.windows(2).map(|w| (w[1] - w[0]).length()).sum::<f32>().max(1e-4);
    let mut run = 0.0;
    for w in path.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = (b - a).length().max(1e-4);
        let (u, _) = frame(b - a);
        let n = (len * 2.0).ceil() as i32;
        for i in 0..=n {
            let t = i as f32 / n as f32;
            let along = run + t * len;
            let c = a + (b - a) * t;
            let lobe = (along / (r * 1.3)).floor() as i32 % 2;
            if rings.iter().any(|&f| (along - f * total).abs() < 1.2) {
                ball(s, c, Vec3::splat(r + 0.6), ICE);
            } else {
                let side = if lobe == 0 { 1.0 } else { -1.0 };
                ball(s, c + u * side * r * 0.35, Vec3::splat(r), if lobe == 0 { SNOW } else { SNOW_M });
            }
        }
        run += len;
    }
    let end = path[path.len() - 1];
    ball(s, end, Vec3::splat(r + 0.4), ICE);
    icicle(s, end - Vec3::Y * r, 4.0 + r, r * 0.7);
}

/**
 * The frost giant, a hrímþurs: a rime-giant two and a half times as tall as a man, pale ice-blue
 * and heavy with muscle, the hollows of it shaded deep blue and frost on his shoulders, his head
 * thrust forward on a bull's neck; a long white beard in locks over his chest, ending in two plaits
 * ringed with ice, icicles hanging from it; white hair combed back into a great plait down his
 * spine and two thin ones at his temples, small crystals of frost growing back through it;
 * deep-set eyes glowing pale blue under a frosted brow; clusters of ice crystals jutting from his
 * shoulders, his forearms and his back, a few of them lit from within; bracers of ice, ice on his
 * knuckles, his fists clenched; a shaggy bearskin wrapped round his hips on a broad belt with a
 * bone clasp, icicles hanging from its edge, hide flaps before and behind, the front one stitched
 * and painted with the rune of the elk; bare feet, ice grown round the ankles.
 */
pub fn frost_giant() -> Collected {
    let (w, h, d) = FROST_GIANT_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = FG;
    let hc = v(cx, 125.0, cz + 7.0);
    let b = Giant { cx, cz, hip: (60.0, 10.5), shoulder: (107.0, 26.0), g: 1.0, neck: v(cx, 118.0, cz + 4.5), skin: RIME };

    // ---- legs, ice grown round the ankles
    for (i, out) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        let ankle = v(cx + out * 13.6, 8.5, cz - 2.4);
        giant_leg(&mut s, &b, v(cx + out * 12.5, 34.0, cz + 2.0), ankle, out, NAIL);
        ice_crust(&mut s, ankle + v(0.0, 0.6, 0.4), ankle + v(0.0, 5.2, 0.4), 6.0, 5.6, 40 + 9 * i as i32);
    }

    // ---- the trunk
    s.part(0);
    giant_trunk(&mut s, &b, d);

    // ---- the bearskin round his hips (filled to the body and round the legs, so that nothing of its
    //      inside is drawn), the belt and its clasp, the hide flaps
    for y in 50..=67 {
        let drop = (67 - y) as f32;
        let (rx, rz) = (19.0 + drop * 0.22, 13.4 + drop * 0.12);
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = ((x as f32 - cx) / rx, (z as f32 - cz) / rz);
                if dx.abs().powf(2.3) + dz.abs().powf(2.3) > 1.0 || y < 50 + (nz(x, 3, z) * 5.0) as i32 {
                    continue;
                }
                let c = if nz(x / 2, y / 3, z / 2) < 0.25 {
                    PELT_D
                } else if nz(x, y, z) > 0.85 {
                    PELT_L
                } else {
                    PELT
                };
                lay(&mut s, v(x as f32, y as f32, z as f32), c);
            }
        }
    }
    slab(&mut s, cx, cz + 0.3, 17.6, 12.8, 2.3, 65, 70, BELT);
    s.paint(|_x, y, _z, c| (c == BELT && (y == 65 || y == 70)).then_some(BELT_L));
    let bz = front(&s, cx as i32, 67, d).unwrap_or(42) as f32;
    for (x, y, z, q) in cells(v(cx - 4.0, 63.0, bz), v(cx + 4.0, 72.0, bz + 1.0)) {
        let r = ((q.x - cx).powi(2) + (q.y - 67.5).powi(2)).sqrt();
        if r < 3.6 {
            s.set(
                x,
                y,
                z,
                if r < 1.4 {
                    BELT
                } else if r > 2.7 {
                    BONE_D
                } else {
                    BONE
                },
            );
        }
    }
    // Icicles hanging from the bearskin's ragged edge.
    for k in 0..22 {
        let a = k as f32 / 22.0 * TAU + 0.1;
        let (x, z) = ((cx + a.cos() * 20.4).round() as i32, (cz + a.sin() * 14.4).round() as i32);
        if let Some(y) = (44..60).find(|&y| matches!(s.get(x, y, z), Some(PELT) | Some(PELT_D) | Some(PELT_L))) {
            icicle(&mut s, v(x as f32, y as f32 - 0.6, z as f32), 2.0 + nz(k, 5, 5) * 3.5, 0.8);
        }
    }
    // The hide flaps: before him to his knees' tops, behind him longer, their hems fringed; the
    // front one stitched round and painted with the rune of the elk.
    let seg = |p: (f32, f32), a: (f32, f32), b: (f32, f32)| {
        let (ab, ap) = ((b.0 - a.0, b.1 - a.1), (p.0 - a.0, p.1 - a.1));
        let t = ((ap.0 * ab.0 + ap.1 * ab.1) / (ab.0 * ab.0 + ab.1 * ab.1)).clamp(0.0, 1.0);
        ((ap.0 - ab.0 * t).powi(2) + (ap.1 - ab.1 * t).powi(2)).sqrt()
    };
    let rune = |x: f32, y: f32| {
        seg((x, y), (cx, 45.5), (cx, 58.0)) < 0.6 || [-1.0f32, 1.0].iter().any(|&sd| seg((x, y), (cx, 52.5), (cx + sd * 3.6, 57.5)) < 0.6)
    };
    for (fore, half, bottom) in [(true, 9.0f32, 41.0f32), (false, 11.0, 44.0)] {
        let z_at = |dx: f32, y: i32| if fore { cz + 15.4 - dx * dx * 2.2 + (64 - y) as f32 * 0.04 } else { cz - 14.6 + dx * dx * 2.0 };
        for y in bottom as i32 - 3..=64 {
            for x in (cx - half - 1.0) as i32..=(cx + half + 1.0) as i32 {
                let dx = (x as f32 - cx) / half;
                if dx.abs() > 1.0 {
                    continue;
                }
                let hem = bottom + dx * dx * 5.0;
                let fringe = y as f32 <= hem && x % 2 == 0 && y as f32 > hem - 3.0;
                if (y as f32) < hem && !fringe {
                    continue;
                }
                let edge = dx.abs() > 0.86 || (y as f32) < hem + 1.5;
                let stitch = fore && y % 2 == 0 && ((dx.abs() - 0.74).abs() < 0.06 || ((y as f32) - (hem + 2.4)).abs() < 0.5);
                let c = if fringe || edge {
                    HIDE_D
                } else if fore && rune(x as f32, y as f32) {
                    BONE
                } else if stitch || nz(x / 2, y / 2, 1) > 0.8 {
                    HIDE_L
                } else {
                    HIDE
                };
                for t in 0..2 {
                    let z = z_at(dx, y) + if fore { -(t as f32) } else { t as f32 };
                    put(&mut s, v(x as f32, y as f32, z), c);
                }
            }
        }
    }

    // ---- crystals of ice growing out of his back, either side of the plait
    for (k, (dx, y)) in
        [(-9.0f32, 104.0f32), (-5.0, 97.0), (5.4, 99.0), (10.0, 105.0), (12.6, 95.0), (-13.0, 94.0), (8.0, 90.0)].into_iter().enumerate()
    {
        let x = cx + dx;
        let z = rear(&s, x as i32, y as i32, d).unwrap_or(10) as f32;
        let n = |j: i32| nz(k as i32, j, 1);
        let dir = v(dx * 0.05 + (n(1) - 0.5) * 0.5, 0.75 + n(2) * 0.4, -1.0);
        crystal(&mut s, v(x, y, z + 3.0), dir, 9.0 + n(3) * 7.0, 1.8 + n(4) * 1.2, k % 3 == 1);
    }

    // ---- arms: the shoulders crusted with crystals, bracers of ice, fists with ice on the knuckles
    for (i, out) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(3 + i as u8);
        let shoulder = Vec3::from(FROST_GIANT_RIG[2 + i].pivot);
        let elbow = v(cx + out * 33.0, 82.0, cz - 3.5);
        let wrist = v(cx + out * 35.0, 61.0, cz + 6.5);
        giant_arm(&mut s, &b, shoulder, elbow, wrist, out);
        let (knuckles, [o, dn, f]) = giant_fist(&mut s, elbow, wrist, out, 1.45, RIME);
        // The bracer: a crust of ice chunks from mid-forearm to the wrist.
        let at = |t: f32| elbow + (wrist - elbow) * t;
        ice_crust(&mut s, at(0.46), at(0.98), 6.68, 5.64, 20 * i as i32);
        // Crystals growing out of the bracer, back and out.
        for (j, (t, dir, l, r)) in
            [(0.6f32, v(0.7, 0.6, -0.8), 9.0f32, 2.0f32), (0.82, v(0.6, 0.2, -0.7), 6.5, 1.6), (0.7, v(0.2, 0.7, -1.0), 7.5, 1.7)]
                .into_iter()
                .enumerate()
        {
            let base = elbow + (wrist - elbow) * t + o * 4.0 - f * 2.5;
            crystal(&mut s, base, o * dir.x + Vec3::Y * dir.y + f * dir.z, l, r, j == 2);
        }
        // Ice on the knuckles: a crust over each, a short spike standing out of it.
        for (j, kn) in knuckles.iter().enumerate() {
            for (x, y, z, q) in cells(*kn - Vec3::splat(2.5), *kn + Vec3::splat(2.5)) {
                if s.get(x, y, z).is_some() && (q - *kn).dot(f) > 0.6 && (q - *kn).length() < 2.4 {
                    s.set(x, y, z, if (q - *kn).dot(dn) < -0.5 { ICE_L } else { ICE });
                }
            }
            crystal(&mut s, *kn + f * 0.8, f * 1.2 + dn * 0.5 + o * (j as f32 - 1.5) * 0.15, 3.4 + (j % 2) as f32 * 1.2, 1.0, false);
        }
        // The shoulder's cluster.
        let top = shoulder + o * 2.6 + Vec3::Y * 7.5;
        for (j, (dir, l, r, off)) in [
            (v(0.35, 1.0, -0.15), 15.0f32, 3.0f32, v(0.0, 0.0, 0.0)),
            (v(0.8, 0.75, 0.15), 11.0, 2.4, v(2.6, -1.6, 1.4)),
            (v(0.1, 1.0, -0.6), 12.0, 2.2, v(-1.4, 0.0, -2.6)),
            (v(0.6, 0.6, -0.65), 9.0, 2.0, v(2.2, -1.0, -2.8)),
            (v(0.9, 0.35, 0.4), 7.0, 1.8, v(3.6, -3.4, 1.6)),
            (v(-0.2, 0.9, 0.35), 8.0, 1.6, v(-2.4, 0.6, 1.8)),
        ]
        .into_iter()
        .enumerate()
        {
            let base = top + o * off.x + Vec3::Y * off.y + Vec3::Z * off.z;
            let dir = o * dir.x + Vec3::Y * dir.y + Vec3::Z * dir.z;
            crystal(&mut s, base - dir.normalize() * 3.0, dir, l + 3.0, r, j == 5);
        }
    }

    // ---- the head: a heavy brow over glowing eyes, a broad nose, the beard and the hair
    s.part(5);
    let zf = hc.z + 9.0;
    ball(&mut s, hc, v(8.4, 9.8, 9.0), RIME);
    rounded(&mut s, v(cx, hc.y - 7.6, hc.z + 2.4), v(7.6, 4.2, 6.6), 2.4, 0, RIME);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 5.6, hc.y - 3.4, zf - 2.2), Vec3::splat(2.7), RIME);
        cap(&mut s, v(cx + side * 8.0, hc.y - 1.0, hc.z - 0.5), v(cx + side * 9.8, hc.y + 1.6, hc.z - 2.2), 1.7, 0.8, RIME);
    }
    cap(&mut s, v(cx - 6.2, hc.y + 1.2, zf - 0.8), v(cx + 6.2, hc.y + 1.2, zf - 0.8), 2.3, 2.3, RIME);
    ball(&mut s, v(cx, hc.y + 0.6, zf + 0.4), Vec3::splat(2.0), RIME);
    cap(&mut s, v(cx, hc.y - 0.2, zf + 0.8), v(cx, hc.y - 4.6, zf + 3.0), 1.5, 2.1, RIME);
    ball(&mut s, v(cx, hc.y - 4.8, zf + 1.8), v(2.8, 1.6, 1.8), RIME);
    // Eyes: deep in shadow under the brow, glowing pale blue.
    let ey = (hc.y - 1.6).round() as i32;
    for side in [-1.0f32, 1.0] {
        for k in 0..4 {
            let x = (cx + side * (1.5 + k as f32)).round() as i32;
            for y in ey..=ey + 1 {
                face(&mut s, x, y, d, 0, RIME_DD);
            }
        }
        for k in 0..2 {
            face(&mut s, (cx + side * (2.5 + k as f32)).round() as i32, ey, d, 0, FROST_EYE);
        }
    }
    for x in [cx.floor() as i32 - 1, cx.ceil() as i32 + 1] {
        face(&mut s, x, (hc.y - 5.0).round() as i32, d, 0, CAVITY);
    }
    // The mouth: a grim line, teeth showing.
    let my = (hc.y - 7.4).round() as i32;
    for x in (cx - 3.0) as i32..=(cx + 3.0).ceil() as i32 {
        face(&mut s, x, my, d, 0, CAVITY);
        face(&mut s, x, my + 1, d, 0, if x % 2 == 0 { TOOTH } else { CAVITY });
    }
    // Bushy brows, frosted, low at the middle; the moustache drooping into the beard.
    for side in [-1.0f32, 1.0] {
        cap(&mut s, v(cx + side * 1.4, hc.y + 2.0, zf + 1.8), v(cx + side * 7.4, hc.y + 3.6, zf - 0.6), 1.4, 1.0, SNOW);
        cap(&mut s, v(cx + side * 1.2, hc.y - 5.8, zf + 2.6), v(cx + side * 5.8, hc.y - 8.4, zf + 1.0), 1.5, 1.4, SNOW);
        cap(&mut s, v(cx + side * 5.8, hc.y - 8.4, zf + 1.0), v(cx + side * 6.6, hc.y - 13.5, zf - 0.2), 1.4, 1.1, SNOW_M);
    }
    // The beard: a short growth over the jaw and cheeks, then locks hanging from it over his
    // chest (the outer laid first, the middle over them), two plaits ringed with ice at its foot.
    let jaw = v(cx, hc.y - 6.4, hc.z + 3.0);
    for (x, y, z, q) in cells(jaw - v(10.0, 7.5, 9.5), jaw + v(10.0, 7.5, 9.5)) {
        let e = (q - jaw) / v(9.4, 6.8, 8.6);
        let line = hc.y - 5.0 + ((q.x - cx).abs() - 6.0).max(0.0) * 1.2;
        if e.length() > 1.0 || q.y > line || (q.y > hc.y - 9.0 && (q.x - cx).abs() < 4.2 && q.z > hc.z + 4.5) {
            continue;
        }
        lay(&mut s, q, if nz(x, y / 3, z) < 0.3 { SNOW_M } else { SNOW });
    }
    let beard = [SNOW, SNOW_M, SNOW, SNOW_S, SNOW_M];
    for (k, a) in
        [-96.0f32, 96.0, -82.0, 82.0, -68.0, 68.0, -55.0, 55.0, -42.0, 42.0, -30.0, 30.0, -18.0, 18.0, -6.0, 6.0].into_iter().enumerate()
    {
        let r = a.to_radians();
        let start = v(cx + 8.4 * r.sin(), hc.y - 9.6 + 3.8 * (1.0 - r.cos()), hc.z + 3.0 + 7.6 * r.cos());
        let len = 29.0 - a.abs() * 0.12 + nz(k as i32, 3, 3) * 5.0;
        let thick = 2.4 - a.abs() * 0.007;
        let n = (len / 2.5).ceil() as i32;
        let (mut prev, mut pr) = (start, thick);
        for j in 1..=n {
            let t = j as f32 / n as f32;
            let y = start.y - t * len;
            let x = cx + (start.x - cx) * (1.0 - 0.4 * t) + (t * 6.0 + k as f32 * 1.7).sin() * 0.6;
            let rr = thick * (1.0 - 0.5 * t);
            let under = front(&s, x.round() as i32, y.round() as i32, d).unwrap_or(0) as f32;
            let p = v(x, y, (start.z - t * 2.0).max(under + rr * 0.2));
            s.part(if y < 104.0 { 0 } else { 5 });
            lay_cap(&mut s, prev, p, pr, rr, beard[k % beard.len()]);
            (prev, pr) = (p, rr);
        }
        if k % 3 == 1 {
            icicle(&mut s, prev - Vec3::Y * pr, 3.0 + nz(k as i32, 4, 4) * 4.0, 0.9);
        }
    }
    s.part(0);
    for side in [-1.0f32, 1.0] {
        let x = cx + side * 2.8;
        let z = front(&s, x as i32, 91, d).unwrap_or(40) as f32;
        braid(&mut s, &[v(x, 91.0, z - 1.0), v(x + side * 0.4, 84.0, z - 1.0), v(x + side * 0.2, 78.0, z - 1.2)], 1.7, &[0.45]);
    }
    s.part(5);
    // The hair: combed back over the crown in locks, two thin plaits at the temples, a great plait
    // down his spine.
    let hair = v(cx, hc.y + 0.8, hc.z - 0.6);
    let hr = v(9.4, 10.6, 9.9);
    for (x, y, z, q) in cells(hair - hr - Vec3::ONE, hair + hr + Vec3::ONE) {
        let hairline = hc.y + 5.4 + (q.x - cx).abs() * 0.12 + nz(x, 1, 1) * 1.2;
        if ((q - hair) / hr).length() > 1.0 || (q.z > hc.z + 3.0 && q.y < hairline) || (q.y < hc.y - 4.0 && q.z > hc.z - 3.0) {
            continue;
        }
        lay(&mut s, q, if nz(x, y / 4, z / 4) < 0.5 { SNOW_D } else { SNOW_S });
    }
    for (k, xo) in [-7.5f32, -5.0, -2.5, 0.0, 2.5, 5.0, 7.5].into_iter().enumerate() {
        let squeeze = (1.0 - (xo / hr.x).powi(2)).max(0.0).sqrt();
        let (ry, rz) = (hr.y * squeeze + 0.5, hr.z * squeeze + 0.5);
        let mut prev: Option<Vec3> = None;
        for j in 0..=14 {
            let th = (36.0 + j as f32 * 11.0).to_radians();
            let p = v(cx + xo * (1.0 - j as f32 * 0.02), hair.y + ry * th.sin(), hair.z + rz * th.cos());
            if let Some(a) = prev {
                lay_cap(&mut s, a, p, 1.25, 1.25, [SNOW, SNOW_M, SNOW][k % 3]);
            }
            prev = Some(p);
        }
    }
    // Frost growing out of his crown: small crystals raked back through the hair.
    for (k, (xo, th, l)) in
        [(0.0f32, 92.0f32, 6.0f32), (-3.6, 104.0, 5.0), (3.6, 100.0, 5.5), (-6.2, 120.0, 4.0), (6.4, 116.0, 4.5)].into_iter().enumerate()
    {
        let r = th.to_radians();
        let base = v(cx + xo, hair.y + hr.y * r.sin() * 0.9, hair.z + hr.z * r.cos() * 0.9);
        crystal(&mut s, base, v(xo * 0.1, 0.75, -1.0 - k as f32 * 0.05), l + 2.0, 1.3, false);
    }
    for side in [-1.0f32, 1.0] {
        let zat = |s: &Sculpt, y: i32, x: f32| front(s, x as i32, y, d).unwrap_or(40) as f32 + 1.2;
        let (x1, x2) = (cx + side * 11.0, cx + side * 12.2);
        let path = [
            v(cx + side * 8.4, hc.y - 1.0, hc.z + 1.0),
            v(cx + side * 9.8, 113.0, zat(&s, 113, cx + side * 9.8)),
            v(x1, 104.0, zat(&s, 104, x1)),
            v(x2, 96.0, zat(&s, 96, x2)),
        ];
        braid(&mut s, &path, 1.3, &[0.55]);
    }
    let mut plait = vec![v(cx, hc.y - 3.0, rear(&s, cx as i32, (hc.y - 3.0) as i32, d).unwrap_or(16) as f32 + 0.6)];
    for y in [112, 104, 96, 88] {
        plait.push(v(cx, y as f32, rear(&s, cx as i32, y, d).unwrap_or(14) as f32 - 1.4));
    }
    braid(&mut s, &plait[..3], 2.6, &[]);
    s.part(0);
    braid(&mut s, &plait[2..], 2.4, &[0.3, 0.7]);

    // ---- shaggy fur on the bearskin, stray hairs hanging from the beard
    shag(&mut s, (w, h, d), PELT, PELT, PELT_L, 0.35, 3);
    shag(&mut s, (w, h, d), SNOW, SNOW_M, SNOW, 0.1, 2);

    // ---- the skin: frost on top, deep blue in its hollows, pale on the crowns of the muscles
    frost(&mut s, (w, h, d), &[RIME], 104, 0.22, BLOOM);
    shade_skin(&mut s, (w, h, d), RIME, [RIME_L, RIME, RIME_D, RIME_DD], 120.0);
    hollows(&mut s, (w, h, d), 3, &[SNOW, SNOW_M, SNOW_S, SNOW_D], |_, c, fill| if fill > 0.7 { scale(c, 0.8) } else { c });
    s.finish(0.04)
}

// ---------------------------------------------------------------------- Surtr

/// Surtr's grid: 104 wide, 172 tall (his mane of fire), 110 deep (his sword reaches far before him).
pub const SURTR_SIZE: (i32, i32, i32) = (104, 172, 110);
const SU: (f32, f32) = (51.5, 54.5);

/// Hips, shoulders and the base of the neck; the sword goes with the right arm.
pub const SURTR_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [SU.0 - 11.0, 66.0, SU.1] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [SU.0 + 11.0, 66.0, SU.1] },
    PartDef { name: "leftArm", boxes: &[], pivot: [SU.0 - 27.5, 116.0, SU.1 - 1.0] },
    PartDef { name: "rightArm", boxes: &[], pivot: [SU.0 + 27.5, 116.0, SU.1 - 1.0] },
    PartDef { name: "head", boxes: &[], pivot: [SU.0, 123.0, SU.1 + 1.0] },
];

/// Surtr's sword hand (the centre of his right fist) and the way his blade points from it.
const SURTR_GRIP: ([f32; 3], [f32; 3]) = ([SU.0 + 32.6, 75.0, SU.1 + 13.0], [0.1, -0.87, 0.48]);
/// How long the blade is, from the guard to the point.
const SURTR_BLADE: f32 = 66.0;

/**
 * A bracer of blackened iron round a forearm from `elbow` to `wrist` (the arm `r0` thick at the
 * elbow, `r1` at the wrist): rimmed at both ends, a ring of rivets inside each rim, three spikes
 * along its outside. `out` is -1 for the left arm, +1 for the right.
 */
fn iron_bracer(s: &mut Sculpt, elbow: Vec3, wrist: Vec3, out: f32, r0: f32, r1: f32) {
    let axis = (wrist - elbow).normalize();
    let len = (wrist - elbow).length();
    let (u, wv) = frame(axis);
    let (a0, a1) = (0.42, 0.96);
    let reach = Vec3::splat(r0 + 3.0);
    for (x, y, z, q) in cells(elbow.min(wrist) - reach, elbow.max(wrist) + reach) {
        let p = q - elbow;
        let t = p.dot(axis) / len;
        if !(a0..=a1).contains(&t) {
            continue;
        }
        let radial = p - axis * (t * len);
        if radial.length() > r0 + (r1 - r0) * t + 1.6 {
            continue;
        }
        let (from0, from1) = ((t - a0) * len, (a1 - t) * len);
        let ang = radial.dot(wv).atan2(radial.dot(u));
        let rivet = ((from0 - 3.0).abs() < 0.6 || (from1 - 3.0).abs() < 0.6) && (ang * 10.0 / TAU).rem_euclid(1.0) < 0.3;
        let c = if rivet {
            RIVET
        } else if from0 < 1.6 || from1 < 1.6 {
            IRON_L
        } else if nz(x, y, z) < 0.15 {
            IRON_D
        } else {
            IRON
        };
        s.set(x, y, z, c);
    }
    let o = Vec3::X * out;
    for k in 0..3 {
        let t = 0.56 + k as f32 * 0.15;
        let base = elbow + (wrist - elbow) * t;
        let r = r0 + (r1 - r0) * t + 1.0;
        cap(s, base + o * r, base + o * (r + 4.0) - axis * 1.0, 1.3, 0.3, IRON_L);
    }
}

/**
 * Surtr's sword, gripped in the fist at `hand` and pointing along `dir`, its flat turned toward
 * `flat`: a long grip bound in leather, an iron pommel, a cross-guard of blackened iron whose arms
 * bend toward the blade, a coal burning at its heart, and the blade, `len` long from just past the
 * guard (its point `len + 9` along `dir` from `hand`), burning brighter than the sun: white-hot
 * along its middle, then gold, orange, and red at its waving edges, flames licking up off it.
 */
fn flaming_sword(s: &mut Sculpt, hand: Vec3, dir: Vec3, flat: Vec3, len: f32) {
    let dir = dir.normalize();
    let flat = (flat - dir * flat.dot(dir)).normalize();
    let edge = dir.cross(flat);
    for k in -9i32..=5 {
        ball(s, hand + dir * k as f32, Vec3::splat(1.5), if k.rem_euclid(3) == 0 { GRIP_L } else { GRIP });
    }
    ball(s, hand - dir * 10.8, Vec3::splat(2.6), IRON);
    ball(s, hand - dir * 12.8, Vec3::splat(1.4), IRON_L);
    let g = hand + dir * 7.0;
    for k in -28..=28 {
        let u = k as f32 / 28.0;
        let p = g + edge * (u * 13.0) + dir * (u * u * 3.6);
        ball(s, p, Vec3::splat(2.3 - 0.6 * u.abs()), if (u.abs() * 7.0).fract() < 0.2 { IRON_L } else { IRON });
    }
    for side in [-1.0f32, 1.0] {
        ball(s, g + edge * (side * 13.4) + dir * 4.2, Vec3::splat(2.3), IRON_L);
    }
    block(s, g + dir * 1.0, [edge, dir, flat], v(4.2, 2.6, 2.6), 2.5, IRON_L);
    ball(s, g + dir * 1.0 + flat * 2.4, Vec3::splat(1.5), MAGMA_Y);
    let b0 = g + dir * 2.0;
    let tip = b0 + dir * len;
    let half = |t: f32| {
        let wave = 5.2 * (1.0 - 0.32 * t) * (1.0 + 0.1 * (t * TAU * 5.0).sin());
        if t < 0.86 {
            wave
        } else {
            wave * ((1.0 - t) / 0.14).max(0.0).powf(0.8)
        }
    };
    let reach = Vec3::splat(8.0);
    for (x, y, z, q) in cells(b0.min(tip) - reach, b0.max(tip) + reach) {
        let p = q - b0;
        let a = p.dot(dir);
        if !(0.0..=len).contains(&a) {
            continue;
        }
        let hw = half(a / len);
        let e = p.dot(edge).abs();
        if e > hw {
            continue;
        }
        let across = e / hw.max(0.01);
        if p.dot(flat).abs() > 1.6 * (1.0 - across) + 0.55 {
            continue;
        }
        let c = if across < 0.22 {
            MAGMA_W
        } else if across < 0.5 {
            MAGMA_Y
        } else if across < 0.8 {
            MAGMA
        } else {
            MAGMA_R
        };
        s.set(x, y, z, c);
    }
    for k in 0..16 {
        let t = 0.06 + k as f32 * 0.055;
        let across = (nz(k, 9, 2) - 0.5) * 1.4 * half(t);
        let base = b0 + dir * (t * len) + edge * across + flat * 1.2;
        flame(s, base, 5.0 + nz(k, 9, 3) * 7.0 * (1.0 - t * 0.5), 1.5 + nz(k, 8, 3) * 0.9, dir * 0.15, nz(k, 7, 3) * TAU);
    }
}

/**
 * Surtr, the fire giant of Muspelheim: a colossus of black basalt cracked all over with seams of
 * glowing magma, brightest in the hollows of his muscles; a mane of fire streaming up and back
 * from his scalp and a beard of fire rising from his jaw, short horns of black rock at his
 * temples, burning eyes, his mouth open on the furnace of his throat; a kilt of blackened iron
 * plates, red-hot along their lower edges, on a broad belt with a rune of fire for its buckle; iron
 * bracers spiked along their outsides; flames rising off his shoulders and his smouldering left
 * fist; and in his right hand, held low and forward, a greatsword burning brighter than the sun,
 * waved like a flame along its edges.
 */
pub fn surtr() -> Collected {
    let (w, h, d) = SURTR_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = SU;
    let hc = v(cx, 136.0, cz + 4.0);
    let b = Giant { cx, cz, hip: (66.0, 11.0), shoulder: (116.0, 27.5), g: 1.07, neck: v(cx, 128.0, cz + 2.0), skin: BASALT };

    // ---- legs, planted wide
    for (i, out) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        giant_leg(&mut s, &b, v(cx + out * 13.5, 37.0, cz + 2.5), v(cx + out * 15.0, 9.0, cz - 2.6), out, HORN_ROCK);
    }

    // ---- the trunk
    s.part(0);
    giant_trunk(&mut s, &b, d);

    // ---- the kilt: plates of blackened iron hanging from a broad belt, longest before and behind,
    //      filled behind them to the body and round the legs (so that nothing of its inside is drawn)
    let plates = 20.0;
    for y in 44..=76 {
        let drop = (76 - y) as f32;
        let (rx, rz) = (19.8 + drop * 0.22, 14.6 + drop * 0.14);
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = ((x as f32 - cx) / rx, (z as f32 - cz) / rz);
                if dx.abs().powf(2.3) + dz.abs().powf(2.3) > 1.0 {
                    continue;
                }
                let a = (z as f32 - cz).atan2(x as f32 - cx);
                let (ix, iz) = ((x as f32 - cx) / (rx - 2.2), (z as f32 - cz) / (rz - 2.2));
                if ix.abs().powf(2.3) + iz.abs().powf(2.3) < 1.0 {
                    if y as f32 >= 60.0 - 11.0 * a.sin().abs() {
                        lay(&mut s, v(x as f32, y as f32, z as f32), IRON_D);
                    }
                    continue;
                }
                let u = (a / TAU * plates + 0.5).rem_euclid(1.0);
                if u < 0.08 {
                    continue;
                }
                let hem = 57.0 - 11.0 * a.sin().abs() + (u - 0.54).powi(2) * 9.0;
                if (y as f32) < hem {
                    continue;
                }
                let rivet = y == 71 && ((u - 0.3).abs() < 0.07 || (u - 0.78).abs() < 0.07);
                let c = if rivet {
                    RIVET
                } else if (y as f32) < hem + 0.9 {
                    MAGMA_R
                } else if (y as f32) < hem + 2.2 {
                    IRON_D
                } else if (u - 0.54).abs() < 0.07 || y >= 74 {
                    IRON_L
                } else {
                    IRON
                };
                lay(&mut s, v(x as f32, y as f32, z as f32), c);
            }
        }
    }
    slab(&mut s, cx, cz + 0.3, 19.0, 14.0, 2.3, 73, 79, IRON_D);
    s.paint(|x, y, z, c| {
        if c != IRON_D || !(73..=79).contains(&y) {
            return None;
        }
        let a = (z as f32 - cz).atan2(x as f32 - cx);
        if y == 73 || y == 79 {
            Some(IRON_L)
        } else if y == 76 && (a / TAU * 24.0).rem_euclid(1.0) < 0.25 {
            Some(RIVET)
        } else {
            None
        }
    });
    // The buckle: a square of iron with a rune of fire burning in it.
    let bz = front(&s, cx as i32, 76, d).unwrap_or(70) as f32;
    block(&mut s, v(cx, 76.0, bz + 0.5), [Vec3::X, Vec3::Y, Vec3::Z], v(5.0, 5.0, 1.3), 4.0, IRON);
    for (dx, dy) in [(1, 3), (0, 2), (-1, 1), (-2, 0), (-1, -1), (0, -2), (1, -3), (-2, 1), (-2, -1), (-2, 2), (-2, -2), (-2, 3), (-2, -3)]
    {
        let (x, y) = ((cx + dx as f32).round() as i32, 76 + dy);
        if let Some(z) = front(&s, x, y, d) {
            s.recolor(x, y, z, if dx == -2 { MAGMA } else { MAGMA_Y });
        }
    }

    // ---- flames rising off his shoulders
    for side in [-1.0f32, 1.0] {
        for (k, (dx, dz, hgt)) in
            [(10.0f32, -3.0f32, 17.0f32), (15.0, -1.0, 13.0), (6.0, -6.0, 12.0), (12.0, -8.0, 10.0)].into_iter().enumerate()
        {
            let x = cx + side * dx;
            let y = (b.shoulder.0 + 14.0).min(h as f32 - 1.0);
            let top = (0..y as i32).rev().find(|&yy| s.get(x as i32, yy, (cz + dz) as i32).is_some()).unwrap_or(120) as f32;
            flame(&mut s, v(x, top - 1.0, cz + dz), hgt, 2.8, v(side * 0.2, 0.0, -0.45), k as f32 * 2.1 + side);
        }
    }

    // ---- arms: bare and cracked, bracers of iron; the left fist smoulders, the right holds the sword
    let (hand, blade) = (Vec3::from(SURTR_GRIP.0), Vec3::from(SURTR_GRIP.1));
    for (i, out) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(3 + i as u8);
        let shoulder = Vec3::from(SURTR_RIG[2 + i].pivot);
        let (elbow, wrist) = if i == 0 {
            (v(cx - 34.5, 91.0, cz - 3.5), v(cx - 37.0, 68.5, cz + 5.0))
        } else {
            (v(cx + 33.5, 94.0, cz + 2.0), hand - (hand - v(cx + 33.5, 94.0, cz + 2.0)).normalize() * 5.4)
        };
        giant_arm(&mut s, &b, shoulder, elbow, wrist, out);
        let (knuckles, [_, _, f]) = giant_fist(&mut s, elbow, wrist, out, 1.5, BASALT);
        iron_bracer(&mut s, elbow, wrist, out, 6.4 * b.g, 4.4 * b.g);
        for side in [8.0f32, 4.0] {
            let p = shoulder + Vec3::X * out * side + Vec3::Y * 8.0;
            let top = (0..h).rev().find(|&yy| s.get(p.x as i32, yy, p.z as i32).is_some()).unwrap_or(120) as f32;
            flame(&mut s, v(p.x, top, p.z), 9.0 + side * 0.6, 2.4, v(out * 0.3, 0.0, -0.25), side + out);
        }
        if i == 0 {
            for (k, kn) in knuckles.iter().enumerate() {
                flame(&mut s, *kn + f * 1.2, 5.0 + (k % 2) as f32 * 3.0, 1.4, f * 0.3, k as f32 * 1.7);
            }
        } else {
            flaming_sword(&mut s, hand, blade, v(0.0, 0.53, 0.85), SURTR_BLADE);
        }
    }

    // ---- the head: a heavy scowl, burning eyes, the mouth a furnace, horns of black rock
    s.part(5);
    let zf = hc.z + 9.4;
    ball(&mut s, hc, v(8.8, 10.4, 9.4), BASALT);
    rounded(&mut s, v(cx, hc.y - 8.0, hc.z + 2.8), v(8.2, 4.6, 7.0), 2.4, 0, BASALT);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 6.0, hc.y - 3.6, zf - 2.2), Vec3::splat(2.9), BASALT);
        cap(&mut s, v(cx + side * 1.4, hc.y + 0.6, zf + 0.8), v(cx + side * 7.4, hc.y + 2.8, zf - 1.4), 2.4, 2.0, BASALT);
    }
    ball(&mut s, v(cx, hc.y + 0.4, zf + 0.6), Vec3::splat(2.1), BASALT);
    cap(&mut s, v(cx, hc.y - 0.6, zf + 0.8), v(cx, hc.y - 4.6, zf + 2.6), 1.6, 2.4, BASALT);
    ball(&mut s, v(cx, hc.y - 4.9, zf + 1.4), v(3.3, 1.7, 1.9), BASALT);
    // Highlights where the light catches the rock of his face: brow, nose, cheekbones.
    for (x, y, z, q) in cells(v(cx - 9.0, hc.y - 6.0, zf - 4.0), v(cx + 9.0, hc.y + 4.0, zf + 4.0)) {
        let brow = (q.y - (hc.y + 1.4 + (q.x - cx).abs() * 0.3)).abs() < 0.8 && (q.x - cx).abs() < 7.5;
        let nose = (q.x - cx).abs() < 1.0 && q.y > hc.y - 4.0 && q.y < hc.y + 0.5;
        let cheek = ((q.x - cx).abs() - 6.0).abs() < 1.4 && (q.y - (hc.y - 2.6)).abs() < 0.8;
        if (brow || nose || cheek) && front(&s, x, y, d) == Some(z) {
            s.recolor(x, y, z, BASALT_L);
        }
    }
    let ey = (hc.y - 1.8).round() as i32;
    for side in [-1.0f32, 1.0] {
        for k in 0..4 {
            for y in ey - 1..=ey + 1 {
                face(&mut s, (cx + side * (1.5 + k as f32)).round() as i32, y, d, 0, if y == ey + 1 { BASALT_DD } else { MAGMA_R });
            }
        }
        face(&mut s, (cx + side * 2.5).round() as i32, ey, d, 0, MAGMA_W);
        face(&mut s, (cx + side * 3.5).round() as i32, ey, d, 0, MAGMA_W);
        face(&mut s, (cx + side * 4.5).round() as i32, ey, d, 0, MAGMA_Y);
    }
    for x in [cx.floor() as i32 - 1, cx.ceil() as i32 + 1] {
        face(&mut s, x, (hc.y - 5.2).round() as i32, d, 0, MAGMA_R);
    }
    // The mouth: open in a snarl, the furnace of his throat glowing between his teeth.
    let my = (hc.y - 8.6).round() as i32;
    for x in (cx - 4.0).round() as i32..=(cx + 4.0).round() as i32 {
        for y in my - 1..=my + 1 {
            if let Some(z) = front(&s, x, y, d) {
                s.clear(x, y, z);
                s.recolor(x, y, z - 1, if y == my { MAGMA_Y } else { MAGMA });
                let edge = x == (cx - 4.0).round() as i32 || x == (cx + 4.0).round() as i32;
                if (y == my + 1 && x % 2 == 0 && !edge) || (y == my - 1 && ((x as f32 - cx).abs() - 2.5).abs() < 0.6) {
                    s.set(x, y, z, FANG);
                }
            }
        }
    }
    // Horns of black rock, out from his temples, forward and up like a bull's, glowing where they
    // break from the skull.
    for side in [-1.0f32, 1.0] {
        let n = 44;
        for k in 0..=n {
            let t = k as f32 / n as f32;
            let reach = 6.6 + 5.6 * (1.0 - (1.0 - t) * (1.0 - t)) - 1.4 * t * t;
            let p = v(cx + side * reach, hc.y + 3.6 + 9.5 * t.powf(1.2), hc.z + 3.2 + 6.4 * t - 1.6 * t * t);
            let c = if t < 0.1 {
                MAGMA_R
            } else if t > 0.86 || (t * 8.0).fract() < 0.22 {
                HORN_ROCK_L
            } else {
                HORN_ROCK
            };
            ball(&mut s, p, Vec3::splat(3.0 * (1.0 - t) + 0.5), c);
        }
    }
    // The mane: flames rooted over his crown and the back of his head, rising and streaming back.
    let hr = v(8.6, 10.2, 9.2);
    for k in 0..26 {
        let a = (nz(k, 1, 7) * 2.0 - 1.0) * PI;
        let e = 0.25 + nz(k, 2, 7) * 1.3;
        if (a.abs() < 1.7 && e < 0.85) || (a.abs() < 1.0 && e < 1.15) {
            continue;
        }
        let dir = v(a.sin() * e.cos(), e.sin(), a.cos() * e.cos());
        let base = hc + dir * hr * 0.96;
        let hgt = 10.0 + nz(k, 3, 7) * 9.0 + e.sin() * 6.0;
        flame(&mut s, base, hgt, 1.9 + nz(k, 4, 7) * 0.9, v(a.sin() * 0.25, 0.0, -0.55), nz(k, 5, 7) * TAU);
    }
    // The beard: flames off the edge of his jaw, out past his cheeks and jutting before his chin.
    let jaw = v(cx, hc.y - 8.0, hc.z + 2.8);
    for k in 0..10 {
        let a = (-1.0 + k as f32 / 4.5) * 1.8;
        if a.abs() < 0.35 {
            continue;
        }
        let base = jaw + v(a.sin() * 8.2, -3.6 + a.sin().abs() * 2.6, a.cos() * 6.4);
        let lean = v(a.sin() * 0.9, 0.0, a.cos() * 0.5);
        flame(&mut s, base, 6.0 + a.abs() * 2.8 + nz(k, 6, 7) * 2.5, 1.5 + nz(k, 7, 7) * 0.5, lean, nz(k, 8, 7) * TAU);
    }
    // Under his chin the fire hangs a moment before it rises: a short fork of flame jutting forward.
    for side in [-1.0f32, 1.0] {
        flame(&mut s, jaw + v(side * 1.8, -4.4, 6.0), 5.0, 1.6, v(side * 0.3, 0.0, 1.3), side * 2.0);
    }

    // ---- the skin: basalt, dark in its hollows, cracked with magma brightest in the creases
    hollows(&mut s, (w, h, d), 5, &[BASALT], |p, _, fill| {
        let g = seam(p, 15.0);
        let crease = ((fill - 0.6) * 5.0).clamp(0.0, 1.0);
        let shown = smooth(p, 12.0) > 0.46 || crease > 0.5;
        let width = 0.8 + crease * 1.5;
        if shown && g < width * 0.4 && crease > 0.35 {
            MAGMA_Y
        } else if shown && g < width * 0.75 && crease > 0.1 {
            MAGMA
        } else if shown && g < width {
            MAGMA_R
        } else if shown && g < width + 0.8 {
            SCORCH
        } else {
            scale(ramp(&[BASALT_L, BASALT, BASALT, BASALT_D, BASALT_DD], (fill - 0.44) / 0.32), 0.85 + 0.15 * (p.y / 140.0).min(1.0))
        }
    });
    s.finish(0.04)
}

// ------------------------------------------------------------------- Níðhöggr

/// Níðhöggr's grid: 86 wide and 88 deep (its coils and the roots round them), 124 tall.
pub const NIDHOGG_SIZE: (i32, i32, i32) = (86, 124, 88);
const NI: (f32, f32) = (42.5, 43.5);

/// The neck where it rises off the coil, the head at the top of the neck, the tail where it
/// leaves the coil.
pub const NIDHOGG_RIG: &[PartDef] = &[
    PartDef { name: "neck", boxes: &[], pivot: [NI.0 - 2.0, 12.0, NI.1 - 10.0] },
    PartDef { name: "head", boxes: &[], pivot: [NI.0 + 1.0, 86.5, NI.1 + 1.0] },
    PartDef { name: "tail", boxes: &[], pivot: [NI.0 - 22.0, 4.8, NI.1 - 17.0] },
];

/// One voxel of a swept body: where it is, its nearest point on the axis, how far along the body
/// that point lies (in voxels), the way the axis runs there, and the body's radius there.
struct Along {
    p: Vec3,
    c: Vec3,
    dist: f32,
    dir: Vec3,
    r: f32,
}

/**
 * A body swept along `knots` (centre, radius) like `sweep`, each voxel coloured by `paint`, over
 * what is there if `over` (else only into empty voxels); its length is counted on from `start` and
 * the end returned, so one body can be swept in pieces.
 */
fn body(s: &mut Sculpt, knots: &[(Vec3, f32)], start: f32, over: bool, paint: impl Fn(&Along) -> u32) -> f32 {
    let mut run = start;
    for w in knots.windows(2) {
        let ((a, ra), (b, rb)) = (w[0], w[1]);
        let ab = b - a;
        let len = ab.length().max(1e-4);
        let reach = Vec3::splat(ra.max(rb) + 1.0);
        for (x, y, z, p) in cells(a.min(b) - reach, a.max(b) + reach) {
            let t = ((p - a).dot(ab) / (len * len)).clamp(0.0, 1.0);
            let c = a + ab * t;
            let r = ra + (rb - ra) * t;
            if (p - c).length_squared() <= r * r && (over || s.get(x, y, z).is_none()) {
                s.set(x, y, z, paint(&Along { p, c, dist: run + t * len, dir: ab / len, r }));
            }
        }
        run += len;
    }
    run
}

/// How long the path through `knots` is.
fn span(knots: &[(Vec3, f32)]) -> f32 {
    knots.windows(2).map(|w| (w[1].0 - w[0].0).length()).sum()
}

/// The point, direction and radius `dist` voxels along `knots`.
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

/**
 * A dragon's hide about its axis: toward `belly` the ridged belly, broad yellow-green plates each
 * with a dark seam; everywhere else black-green diamond scales, every one edged in shadow and a
 * lighter one here and there, the ridge of the back darkest.
 */
fn dragon_hide(belly: Vec3) -> impl Fn(&Along) -> u32 {
    move |a| {
        let radial = (a.p - a.c).normalize_or_zero();
        let down = (belly - a.dir * belly.dot(a.dir)).normalize_or_zero();
        let b = radial.dot(down);
        if b > 0.62 {
            let k = (a.dist / 2.6).rem_euclid(1.0);
            return if k < 0.28 {
                BELLY_D
            } else if k > 0.78 {
                BELLY_L
            } else {
                BELLY
            };
        }
        if b > 0.45 {
            return SCALE_D;
        }
        let around = radial.dot(a.dir.cross(down)).atan2(b) * a.r / 3.0;
        let along = a.dist / 3.0;
        let (d1, d2) = ((along + around).rem_euclid(1.0), (along - around).rem_euclid(1.0));
        let (x, y, z) = (a.p.x as i32, a.p.y as i32, a.p.z as i32);
        // Pale bands across the back every so often, ragged where they meet the flanks.
        let band = (a.dist / 15.0).rem_euclid(1.0) < 0.15 && b < 0.1 + nz(x, y, z) * 0.25;
        if d1 < 0.2 || d2 < 0.2 || (b < -0.8 && nz(x, y, z) < 0.5) {
            SCALE_D
        } else if band || ((d1 - 0.6).abs() < 0.16 && (d2 - 0.6).abs() < 0.16 && nz(x, y, z) < 0.6) {
            SCALE_L
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
    cap(s, base, mid, r, r * 0.55, SPINE);
    cap(s, mid, tip, r * 0.55, 0.3, SPINE_D);
    tip
}

/// A tattered web of skin between two spines (`a` and `b`, each from its root to its point), its
/// free edge sagging between their points, veined, and torn through here and there.
fn frill(s: &mut Sculpt, a: (Vec3, Vec3), b: (Vec3, Vec3)) {
    let m = ((b.0 - a.0).length() * 3.0) as i32 + 4;
    let n = ((a.1 - a.0).length().max((b.1 - b.0).length()) * 3.0) as i32 + 4;
    for i in 0..=m {
        let u = i as f32 / m as f32;
        let (root, top) = (a.0 + (b.0 - a.0) * u, a.1 + (b.1 - a.1) * u);
        let sag = 1.0 - 0.42 * (u * PI).sin();
        for j in 0..=n {
            let w = j as f32 / n as f32;
            if w > sag {
                break;
            }
            let p = root + (top - root) * w;
            if w > 0.35 && nz(p.x.round() as i32 * 3, p.y.round() as i32, p.z.round() as i32 * 5) < 0.12 {
                continue;
            }
            let c = if (u * 3.0).fract() < 0.18 {
                FRILL_L
            } else if w > sag - 0.14 {
                FRILL_D
            } else {
                FRILL
            };
            lay(s, p, c);
        }
    }
}

/**
 * A crest down the back of a body along `knots`: a spine every `every` voxels from `from` to `to`
 * along it, raked toward the body's start and `len(f)` long at fraction `f` of the way, webbed
 * between with frill; `back` is the way the back faces, `part(f)` the part each belongs to.
 */
#[allow(clippy::too_many_arguments)]
fn crest(
    s: &mut Sculpt,
    knots: &[(Vec3, f32)],
    from: f32,
    to: f32,
    every: f32,
    back: Vec3,
    len: impl Fn(f32) -> f32,
    part: impl Fn(f32) -> u8,
) {
    let mut prev: Option<(Vec3, Vec3)> = None;
    let mut dist = from;
    while dist <= to {
        let (p, dir, r) = at_length(knots, dist);
        let up = (back - dir * back.dot(dir)).normalize_or_zero();
        let base = p + up * (r - 0.9);
        let f = (dist - from) / (to - from).max(1e-4);
        s.part(part(f));
        let l = len(f);
        let tip = spine(s, base, up - dir * 0.55, l, 0.75 + l * 0.08);
        if let Some(a) = prev {
            frill(s, a, (base, tip));
        }
        prev = Some((base, tip));
        dist += every;
    }
    s.part(0);
}

/// The bark of a root of the World Tree, ridged in a slow twist and paler on top; past `gnawed`
/// voxels along it, bitten wood, pale and scored with tooth marks.
fn bark(gnawed: f32) -> impl Fn(&Along) -> u32 {
    move |a| {
        let radial = (a.p - a.c).normalize_or_zero();
        let (u, w) = frame(a.dir);
        let grain = (radial.dot(w).atan2(radial.dot(u)) * 3.0 / TAU + a.dist * 0.12).rem_euclid(1.0);
        if a.dist > gnawed {
            if (a.p.x + a.p.y * 2.0) as i32 % 3 == 0 {
                GNAWED_D
            } else {
                GNAWED
            }
        } else if grain < 0.22 {
            ROOT_D
        } else if radial.y > 0.55 {
            ROOT_L
        } else {
            ROOT
        }
    }
}

/**
 * A root wound round the body along `knots`, from `from` to `to` voxels along it and `turns` times
 * round, starting at angle `phase`, `r` thick: it comes up out of the ground before and goes back
 * down into it after, lying only where nothing else is.
 */
#[allow(clippy::too_many_arguments)]
fn vine(s: &mut Sculpt, knots: &[(Vec3, f32)], from: f32, to: f32, turns: f32, phase: f32, r: f32, centre: Vec3) {
    let mut path = vec![];
    let n = ((to - from) / 2.5).ceil() as i32;
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let (p, dir, br) = at_length(knots, from + (to - from) * t);
        let (u, w) = frame(dir);
        let a = phase + t * turns * TAU;
        let mut q = p + (u * a.cos() + w * a.sin()) * (br + r * 0.4);
        q.y = q.y.max(r * 0.5);
        path.push((q, r * (1.0 - 0.25 * t)));
    }
    let out = |q: Vec3| {
        let o = v(q.x - centre.x, 0.0, q.z - centre.z).normalize_or_zero();
        v(q.x, -1.0, q.z) + o * 4.0
    };
    let (first, last) = (path[0], path[path.len() - 1]);
    path.insert(0, (out(first.0), r * 1.15));
    path.push((out(last.0), last.1));
    body(s, &path, 0.0, false, bark(f32::MAX));
}

/// A strand of venom hanging `len` from `top`, a drop gathering at its end.
fn drip(s: &mut Sculpt, top: Vec3, len: f32) {
    cap(s, top, top - Vec3::Y * len, 0.6, 0.45, VENOM_D);
    ball(s, top - Vec3::Y * (len + 0.6), Vec3::splat(1.05), VENOM);
}

/**
 * Níðhöggr, the Malice Striker: a great wingless serpent-dragon, black-green and scaled, its belly
 * ridged with sickly yellow-green plates; its body coiled twice round on the ground, the inner
 * coil heaped on the outer, roots of the World Tree wound round the coils and two of them gnawed
 * off; its neck rearing up out of the coils in an S, a crest of bone spines webbed with tattered
 * frill down its back from its crown to its tail's barbed tip; a long horned head thrust forward,
 * the jaws gaping on rows of teeth, the long fangs dripping green venom, the eyes glowing
 * yellow-green under heavy brows.
 */
pub fn nidhogg() -> Collected {
    let (w, h, d) = NIDHOGG_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = NI;
    let at = |x: f32, y: f32, z: f32, r: f32| (v(cx + x, y, cz + z), r);
    let ground = |x: f32, z: f32, r: f32| (v(cx + x, r + 0.2, cz + z), r);

    // ---- the coils: the outer round on the ground from the tail's root, the inner heaped on it,
    //      ending at the back where the neck rises
    let coil = [
        ground(-22.0, -17.0, 4.6),
        ground(-29.0, -3.0, 5.8),
        ground(-25.5, 13.0, 6.8),
        ground(-12.0, 24.0, 7.6),
        ground(6.0, 26.0, 8.2),
        ground(21.5, 17.0, 8.6),
        ground(28.0, 0.0, 8.8),
        ground(21.0, -18.0, 8.8),
        ground(5.0, -27.0, 8.8),
        ground(-10.5, -24.0, 8.6),
        at(-18.0, 12.0, -8.0, 8.4),
        at(-20.5, 14.5, 4.0, 8.2),
        at(-12.0, 15.5, 14.0, 8.0),
        at(0.0, 15.5, 16.5, 7.8),
        at(12.0, 15.5, 12.0, 7.8),
        at(16.0, 14.5, 0.0, 7.8),
        at(10.0, 13.5, -10.0, 7.8),
        at(-2.0, 12.0, -10.0, 7.6),
    ];
    s.part(0);
    let coil_len = body(&mut s, &coil, 0.0, true, dragon_hide(-Vec3::Y));
    let outer = span(&coil[..10]);
    crest(&mut s, &coil, 6.0, outer - 4.0, 5.0, Vec3::Y, |f| 4.5 + f * 2.0, |_| 0);
    crest(&mut s, &coil, outer + 6.0, coil_len - 4.0, 4.6, Vec3::Y, |f| 6.0 + f * 2.5, |_| 0);

    // ---- the tail, out from the coil and curling, its tip barbed with bone
    s.part(3);
    let tail = [
        at(-22.0, 4.8, -17.0, 4.6),
        at(-28.5, 4.0, -25.0, 3.4),
        at(-26.0, 4.6, -33.5, 2.5),
        at(-18.5, 6.0, -37.5, 1.8),
        at(-11.0, 8.0, -36.0, 1.2),
        at(-7.5, 10.0, -32.0, 0.7),
    ];
    let tail_len = span(&tail);
    body(&mut s, &tail, -tail_len, true, dragon_hide(-Vec3::Y));
    let (tip, tdir, _) = at_length(&tail, tail_len);
    let (tu, _) = frame(tdir);
    for (side, up) in [(-1.0f32, 0.4f32), (1.0, 0.4), (0.0, 1.0)] {
        spine(&mut s, tip - tdir * 1.5, tdir + tu * side * 0.8 + Vec3::Y * up, 4.0, 0.9);
    }
    crest(&mut s, &tail, 3.0, tail_len - 4.0, 4.0, Vec3::Y, |f| 3.2 - f * 1.6, |_| 3);

    // ---- the neck, rearing up out of the coils in an S
    let neck = [
        at(-2.0, 12.0, -10.0, 8.4),
        at(2.5, 25.0, -6.0, 8.2),
        at(5.5, 38.0, 2.0, 7.8),
        at(2.5, 51.0, 7.0, 7.4),
        at(-3.5, 63.0, 4.0, 7.0),
        at(-4.5, 75.0, -2.0, 6.6),
        at(-1.0, 86.5, 1.0, 6.2),
        at(0.0, 94.0, 6.0, 5.8),
    ];
    s.part(1);
    let mid = body(&mut s, &neck[..7], coil_len, true, dragon_hide(Vec3::Z));
    s.part(2);
    body(&mut s, &neck[6..], mid, true, dragon_hide(Vec3::Z));
    let (neck_len, joint) = (span(&neck), span(&neck[..7]));
    crest(&mut s, &neck, 5.0, neck_len - 1.0, 4.4, -Vec3::Z, |f| 6.0 + f * 5.5, |f| if 5.0 + f * (neck_len - 6.0) > joint { 2 } else { 1 });

    // ---- the head: long, horned, the jaws gaping on their teeth, venom dripping
    s.part(2);
    let k = 1.2;
    let ho = v(cx, 94.0, cz + 6.0);
    let hp = |x: f32, y: f32, z: f32| ho + v(x, y, z) * k;
    let hide = dragon_hide(-Vec3::Y);
    body(&mut s, &[(hp(0.0, 4.0, 0.5), 6.6 * k), (hp(0.0, 4.5, 5.5), 6.8 * k)], 0.0, true, &hide);
    ball(&mut s, hp(0.0, 4.6, 3.5), v(8.0, 6.4, 6.6) * k, SCALE);
    // The jaws, each a taper along z, broader than deep: the upper from z0 for 21, the lower from z1
    // for 20 (times k).
    let (z0, z1) = (ho.z + 6.5 * k, ho.z + 1.5 * k);
    let su = |z: f32| ((z - z0) / (21.0 * k)).clamp(0.0, 1.0);
    let sl = |z: f32| ((z - z1) / (20.0 * k)).clamp(0.0, 1.0);
    let up_y = |z: f32| ho.y + (4.5 - 2.5 * su(z)) * k;
    let up_r = |z: f32| (5.6 - 2.6 * su(z)) * k;
    let lo_y = |z: f32| ho.y + (0.5 - 11.5 * sl(z)) * k;
    let lo_r = |z: f32| (4.6 - 2.2 * sl(z)) * k;
    for (yf, rf, za, len) in [(&up_y as &dyn Fn(f32) -> f32, &up_r as &dyn Fn(f32) -> f32, z0, 21.0 * k), (&lo_y, &lo_r, z1, 20.0 * k)] {
        let reach = 8.0 * k;
        for (x, y, z, q) in cells(v(cx - reach, yf(za + len) - reach, za - reach), v(cx + reach, yf(za) + reach, za + len + 3.0)) {
            let zz = q.z.clamp(za, za + len);
            let (ay, r) = (yf(zz), rf(zz));
            let e = v((q.x - cx) / 1.18, (q.y - ay) / 0.9, q.z - zz);
            if e.length() <= r {
                s.set(x, y, z, hide(&Along { p: q, c: v(cx, ay, zz), dist: zz, dir: Vec3::Z, r }));
            }
        }
    }
    // The mouth between them: the gullet dark at the back, the gums red, the lips left round it.
    for (x, y, z, q) in cells(v(cx - 6.0, lo_y(z1 + 20.0 * k) - 2.0, z1), v(cx + 6.0, up_y(z0), z1 + 20.0 * k)) {
        let (top, bottom) = (up_y(q.z) - up_r(q.z) * 0.72, lo_y(q.z) + lo_r(q.z) * 0.72);
        if q.y < top && q.y > bottom && (q.x - cx).abs() < up_r(q.z).min(lo_r(q.z)) * 0.78 {
            s.set(x, y, z, if q.z < ho.z + 9.0 * k { GULLET } else { GUM });
        }
    }
    let mut z = ho.z + 8.5 * k;
    while z < ho.z + 25.0 * k {
        for side in [-1.0f32, 1.0] {
            let base = v(cx + side * up_r(z) * 0.8, up_y(z) - up_r(z) * 0.62, z);
            cap(&mut s, base, base - Vec3::Y * 2.6, 0.75, 0.3, TOOTH);
        }
        z += 2.2;
    }
    let mut z = ho.z + 4.5 * k;
    while z < ho.z + 19.0 * k {
        for side in [-1.0f32, 1.0] {
            let base = v(cx + side * lo_r(z) * 0.8, lo_y(z) + lo_r(z) * 0.62, z);
            cap(&mut s, base, base + Vec3::Y * 2.2, 0.7, 0.3, TOOTH);
        }
        z += 2.2;
    }
    for side in [-1.0f32, 1.0] {
        let zf = ho.z + 25.1 * k;
        let base = v(cx + side * 2.2, up_y(zf) - up_r(zf) * 0.7, zf);
        let fang = base - Vec3::Y * 6.4 + Vec3::Z * 0.5;
        cap(&mut s, base, fang, 1.3, 0.3, TOOTH);
        drip(&mut s, fang - Vec3::Y * 0.8, 3.0 + (side + 1.0) * 1.6);
        let zl = ho.z + 20.1 * k;
        let base = v(cx + side * 1.8, lo_y(zl) + lo_r(zl) * 0.7, zl);
        cap(&mut s, base, base + Vec3::Y * 4.2, 1.1, 0.3, TOOTH);
    }
    let chin = ho.z + 21.5 * k;
    drip(&mut s, v(cx, lo_y(chin) - lo_r(chin) - 0.5, chin), 6.5);
    // The ridge of the nose, a little horn on it, nostrils at the tip of the snout.
    let (n0, n1) = (ho.z + 8.5 * k, ho.z + 24.5 * k);
    cap(&mut s, v(cx, up_y(n0) + up_r(n0) - 0.4, n0), v(cx, up_y(n1) + up_r(n1) - 0.2, n1), 1.7, 1.2, SCALE_D);
    spine(&mut s, v(cx, up_y(n1) + up_r(n1) - 0.2, n1), v(0.0, 1.0, 0.45), 4.0, 1.4);
    let tipz = ho.z + 26.5 * k;
    for side in [-1.0f32, 1.0] {
        for dz in [0.0f32, -1.0] {
            put(&mut s, v(cx + side * 1.8, up_y(tipz) + up_r(tipz) * 0.4, tipz + dz), CAVITY);
        }
    }
    // Eyes glowing yellow-green, slit, under heavy brows.
    for side in [-1.0f32, 1.0] {
        let eye = hp(side * 5.0, 7.0, 8.5);
        ball(&mut s, eye, v(1.5, 1.3, 1.8) * k, DRAKE_EYE);
        put(&mut s, eye + v(side * 1.5, -0.8, 0.0), DRAKE_EYE_D);
        for dy in [-1.0f32, 0.0, 1.0] {
            put(&mut s, eye + v(side * 1.7, dy, 0.0), CAVITY);
        }
        cap(&mut s, hp(side * 2.4, 9.9, 11.5), hp(side * 6.8, 10.3, 4.9), 1.8 * k, 1.4 * k, SCALE_D);
    }
    // Horns: a great pair swept back from the crown, a lesser pair from the cheeks, a frill at each
    // hinge of the jaw, spikes under the chin.
    for side in [-1.0f32, 1.0] {
        let path = [hp(side * 4.0, 9.5, 0.5), hp(side * 7.0, 12.5, -6.5), hp(side * 8.6, 16.0, -13.5), hp(side * 8.0, 19.0, -19.5)];
        let radii = [2.7f32, 2.0, 1.2, 0.4];
        for seg in 0..3 {
            for j in 0..10 {
                let t = j as f32 / 10.0;
                let p = path[seg] + (path[seg + 1] - path[seg]) * t;
                let rr = (radii[seg] + (radii[seg + 1] - radii[seg]) * t) * k;
                ball(&mut s, p, Vec3::splat(rr), if (seg * 10 + j) % 4 == 0 || seg == 2 { SPINE_D } else { SPINE });
            }
        }
        cap(&mut s, hp(side * 6.0, 4.5, -0.5), hp(side * 10.5, 3.5, -7.0), 1.8 * k, 1.1 * k, SPINE);
        cap(&mut s, hp(side * 10.5, 3.5, -7.0), hp(side * 12.5, 5.6, -11.5), 1.1 * k, 0.3, SPINE_D);
        let hinge = hp(side * 5.6, 0.6, 0.1);
        let mut prev: Option<(Vec3, Vec3)> = None;
        for (j, dir) in [v(side * 0.9, 0.5, -0.7), v(side * 1.0, -0.1, -0.8), v(side * 0.8, -0.7, -0.6)].into_iter().enumerate() {
            let tip = spine(&mut s, hinge, dir, (7.0 - j as f32 * 0.6) * k, 0.9);
            if let Some(a) = prev {
                frill(&mut s, a, (hinge, tip));
            }
            prev = Some((hinge, tip));
        }
    }
    for j in 0..3 {
        let z = ho.z + (10.0 + j as f32 * 4.0) * k;
        spine(&mut s, v(cx, lo_y(z) - lo_r(z) + 0.6, z), v(0.0, -1.0, -0.6), 3.0 - j as f32 * 0.5, 0.9);
    }

    // ---- the roots it gnaws: a great one arching out of the ground over the back of the coils and
    //      down again, lesser ones wound round the outer coil, one bitten off where it lies
    s.part(0);
    let mid_c = v(cx, 0.0, cz);
    let r = |x: f32, y: f32, z: f32, rad: f32| (v(cx + x, y, cz + z), rad);
    let great = [
        r(-36.5, -2.0, -16.0, 4.8),
        r(-31.0, 12.0, -19.0, 4.5),
        r(-22.0, 24.0, -21.0, 4.3),
        r(-8.0, 28.0, -23.0, 4.0),
        r(8.0, 27.0, -23.0, 3.9),
        r(22.0, 22.0, -21.0, 4.0),
        r(32.0, 10.0, -17.0, 4.3),
        r(35.5, -2.0, -13.0, 4.5),
    ];
    body(&mut s, &great, 0.0, false, bark(f32::MAX));
    vine(&mut s, &coil, 14.0, 34.0, 1.25, 0.6, 2.0, mid_c);
    vine(&mut s, &coil, 86.0, 108.0, 1.0, 2.2, 2.3, mid_c);
    vine(&mut s, &great, 30.0, 52.0, 1.5, 1.0, 1.6, mid_c);
    let bitten = [
        r(-34.0, -1.0, 25.0, 3.2),
        r(-28.0, 9.0, 22.0, 3.0),
        r(-20.0, 15.5, 18.0, 2.8),
        r(-13.0, 17.5, 25.0, 2.6),
        r(-9.0, 15.0, 31.0, 2.4),
    ];
    let len = span(&bitten);
    body(&mut s, &bitten, 0.0, false, bark(len - 1.8));

    // ---- shadow where the coils lie against each other and the ground
    hollows(&mut s, (w, h, d), 3, &[SCALE, SCALE_L, BELLY, BELLY_L], |_, c, fill| if fill > 0.72 { scale(c, 0.72) } else { c });
    s.finish(0.04)
}

// ------------------------------------------------------------------------ Hel

/// Hel's grid: 60 wide, 158 tall (her staff), 60 deep (the train of her robes behind her).
pub const HEL_SIZE: (i32, i32, i32) = (60, 158, 60);
const HE: (f32, f32) = (29.5, 29.5);

/// Her shoulders, the base of her neck and her hips; the staff goes with her right arm.
pub const HEL_RIG: &[PartDef] = &[
    PartDef { name: "leftArm", boxes: &[], pivot: [HE.0 - 10.5, 100.0, HE.1] },
    PartDef { name: "rightArm", boxes: &[], pivot: [HE.0 + 10.5, 100.0, HE.1] },
    PartDef { name: "head", boxes: &[], pivot: [HE.0, 106.0, HE.1 + 0.6] },
    PartDef { name: "leftLeg", boxes: &[], pivot: [HE.0 - 4.0, 64.0, HE.1] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [HE.0 + 4.0, 64.0, HE.1] },
];

/// The centre of Hel's right fist, where the shaft of her staff passes up through it.
const HEL_HAND: [f32; 3] = [HE.0 + 15.0, 82.0, HE.1 + 10.0];

/**
 * A wide sleeve of black cloth from `shoulder` by `elbow`, close round the upper arm, then opening
 * below the forearm into a bell whose mouth hangs below and beyond `wrist`, lined in dark purple
 * and torn into tatters at its edge.
 */
fn bell_sleeve(s: &mut Sculpt, shoulder: Vec3, elbow: Vec3, wrist: Vec3) {
    cap(s, shoulder, elbow, 3.7, 4.0, ROBE);
    let mouth = wrist + v(0.0, -4.5, -1.0);
    let axis = mouth - elbow;
    let len = axis.length();
    let dir = axis / len;
    let (u, w) = frame(dir);
    let reach = Vec3::splat(9.0);
    for (x, y, z, q) in cells(elbow.min(mouth) - reach, elbow.max(mouth) + reach) {
        let p = q - elbow;
        let a = p.dot(dir);
        if !(0.0..=len).contains(&a) {
            continue;
        }
        let t = a / len;
        let r = 4.0 + 3.6 * t * t;
        let radial = p - dir * a;
        let rl = radial.length();
        if rl > r || (t > 0.5 && rl < r - 1.4) {
            continue;
        }
        let ang = radial.dot(w).atan2(radial.dot(u));
        let strip = ((ang + PI) / TAU * 13.0).floor() as i32;
        if a > len - nz(strip, 7, 3) * 3.6 {
            continue;
        }
        let c = if t > 0.5 && rl < r - 1.1 {
            LINING_D
        } else if t > 0.5 && rl < r - 0.7 {
            LINING
        } else if (ang * 5.0 + t * 2.0).sin() > 0.75 {
            ROBE_L
        } else {
            ROBE
        };
        s.set(x, y, z, c);
    }
}

/**
 * A hand of bare bone at `wrist`, reaching along `fwd` palm down: the knot of small bones at the
 * wrist, the thumb crooked in from the inside, then four long fingers splayed and hooked, each a
 * long bone and three short ones, its knuckles darker. `out` is the way the little finger lies
 * (-1 toward lower x, +1 higher).
 */
fn bone_hand(s: &mut Sculpt, wrist: Vec3, fwd: Vec3, out: f32) {
    let f = fwd.normalize();
    let lat = Vec3::X * out;
    let down = -Vec3::Y;
    ball(s, wrist + f * 0.9, v(1.7, 0.9, 1.4), SKULL);
    let thumb = wrist + f * 0.8 - lat * 1.5;
    let t1 = thumb + f * 1.4 - lat * 1.5;
    let t2 = t1 + f * 1.8 - lat * 0.6 + down * 0.4;
    cap(s, thumb, t1, 0.55, 0.5, SKULL);
    cap(s, t1, t2, 0.5, 0.4, SKULL);
    put(s, t1, SKULL_D);
    for i in 0..4 {
        let across = -0.9 + i as f32 * 0.75;
        let base = wrist + f * 1.6 + lat * across;
        let dir = (f + lat * across * 0.2).normalize();
        let k1 = base + dir * 3.0;
        let k2 = k1 + (dir + down * 0.35).normalize() * (2.2 - (i as f32 - 1.0).abs() * 0.2);
        let k3 = k2 + (dir + down * 0.9).normalize() * 1.6;
        let tip = k3 + (dir * 0.4 + down).normalize() * 1.2;
        cap(s, base, k1, 0.55, 0.5, SKULL);
        cap(s, k1, k2, 0.5, 0.45, SKULL);
        cap(s, k2, k3, 0.45, 0.4, SKULL);
        cap(s, k3, tip, 0.4, 0.3, SKULL_D);
        put(s, k1, SKULL_D);
        put(s, k2, SKULL_D);
    }
}

/**
 * Hel's staff, its shaft passing up through the fist at `hand` from `foot` to `top`: blackened
 * wood bound with iron rings, an iron spike at its foot; at its head a crescent of dark iron, horns
 * up, spiked all round its outer edge, and cupped in it a gem of cold blue light held in three
 * prongs, its centre 10.2 above `top`.
 */
fn hel_staff(s: &mut Sculpt, hand: Vec3, foot: f32, top: f32) {
    let (sx, sz) = (hand.x, hand.z);
    for y in foot as i32 + 3..=top as i32 {
        let ring = (y - hand.y as i32).rem_euclid(13) == 0 || y == top as i32 || y == top as i32 - 1;
        let r = if ring { 1.6 } else { 1.15 };
        for (x, yy, z, q) in cells(v(sx - 2.0, y as f32, sz - 2.0), v(sx + 2.0, y as f32, sz + 2.0)) {
            let (dx, dz) = (q.x - sx, q.z - sz);
            if dx * dx + dz * dz > r * r {
                continue;
            }
            let grain = ((dz.atan2(dx) * 2.0 / TAU + y as f32 * 0.07).rem_euclid(1.0)) < 0.3;
            s.set(
                x,
                yy,
                z,
                if ring {
                    DARK_IRON_L
                } else if grain {
                    SHAFT_L
                } else {
                    SHAFT
                },
            );
        }
    }
    cap(s, v(sx, foot + 3.5, sz), v(sx, foot, sz), 1.3, 0.3, DARK_IRON);
    // The crescent, in the plane facing forward: an outer circle less an inner one set higher.
    let c = v(sx, top + 9.0, sz);
    let (ro, ri, lift) = (9.6f32, 6.8f32, 3.4f32);
    for (x, y, z, q) in cells(c - v(ro + 1.0, ro + 1.0, 2.0), c + v(ro + 1.0, ro + 1.0, 2.0)) {
        let p = q - c;
        let (out_r, in_r) = ((p.x * p.x + p.y * p.y).sqrt(), (p.x * p.x + (p.y - lift) * (p.y - lift)).sqrt());
        if out_r > ro || in_r < ri || p.z.abs() > 1.3 - (out_r - ri).max(0.0) * 0.02 {
            continue;
        }
        let edge = out_r > ro - 1.0 || in_r < ri + 0.9;
        s.set(x, y, z, if edge { DARK_IRON_L } else { DARK_IRON });
    }
    // Spikes all round its outer edge, longest at the bottom and the horns' outsides.
    for k in 0..11 {
        let a = (-170.0 + k as f32 * 22.0).to_radians() + PI * 0.0;
        let dir = v(a.cos(), a.sin(), 0.0);
        if (c.y + dir.y * ro) > c.y + 7.5 {
            continue;
        }
        let len = 3.0 + (1.0 - (a.sin() + 1.0) * 0.5) * 2.2;
        cap(s, c + dir * (ro - 0.6), c + dir * (ro + len), 1.0, 0.25, DARK_IRON);
    }
    // The gem in the cup, its prongs.
    let gem = c + v(0.0, 1.2, 0.0);
    for (x, y, z, q) in cells(gem - Vec3::splat(3.5), gem + Vec3::splat(3.5)) {
        let p = q - gem;
        let rr = (p / v(2.6, 3.2, 2.4)).length();
        if rr <= 1.0 {
            s.set(
                x,
                y,
                z,
                if rr < 0.55 {
                    GEM_CORE
                } else if p.y > 1.2 || p.x < -1.0 {
                    GEM
                } else {
                    GEM_D
                },
            );
        }
    }
    for a in [-0.75f32, 0.0, 0.75] {
        let from = c + v(a.sin() * ri * 0.9, lift - a.cos() * ri, 0.0);
        cap(s, from, gem + v(a.sin() * 2.2, -1.6 * a.cos() + 0.6, 0.6), 0.6, 0.45, DARK_IRON_L);
    }
}

/**
 * Hel, ruler of the dead, Loki's daughter: a tall, gaunt woman in layered black, her face split
 * down the middle, the right half pale living flesh and the left blue-black and rotting, the
 * skull showing through at cheek, brow and jaw, her teeth bare on that side; both eyes burning a
 * cold blue; long black hair, and a crown of black thorny spikes; a high stiff collar standing
 * round her neck and flaring up behind her head, lined in purple; a fitted bodice with ribs of
 * bone on its dead side and a girdle of bones; a purple-black gown flaring to the floor and
 * trailing behind her, an open over-robe and a mantle over it, a border of runes embroidered round
 * them above their torn hems; wide sleeves torn into tatters; her left hand bare bone, reaching;
 * in her right a tall staff of black wood crowned with a spiked iron crescent cupping a cold blue
 * gem; the toes of pointed shoes under her hem.
 */
pub fn hel() -> Collected {
    let (w, h, d) = HEL_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = HE;

    // ---- legs, thin and hosed in black under the gown, the toes of pointed shoes peeping out
    for (i, out) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(4 + i as u8);
        let hip = Vec3::from(HEL_RIG[3 + i].pivot);
        let knee = v(cx + out * 4.6, 34.0, cz + 1.0);
        let ankle = v(cx + out * 4.8, 5.0, cz + 0.8);
        cap(&mut s, hip, knee, 3.4, 2.5, ROBE_D);
        cap(&mut s, knee, ankle, 2.5, 1.8, ROBE_D);
        rounded(&mut s, v(ankle.x, 2.0, ankle.z + 3.0), v(2.0, 2.0, 4.6), 2.2, 0, SHOE);
        cap(&mut s, v(ankle.x, 1.6, ankle.z + 6.5), v(ankle.x + out * 0.4, 1.0, ankle.z + 11.0), 1.4, 0.4, SHOE);
    }

    // ---- the body: a narrow waist, the bodice, the shoulders, a long neck
    s.part(0);
    ball(&mut s, v(cx, 72.0, cz), v(7.0, 9.0, 5.4), ROBE);
    ball(&mut s, v(cx, 89.0, cz), v(8.0, 12.0, 5.8), ROBE);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 3.0, 92.0, cz + 3.6), v(3.0, 2.8, 2.4), ROBE);
    }
    cap(&mut s, v(cx - 9.5, 99.0, cz - 0.4), v(cx + 9.5, 99.0, cz - 0.4), 3.4, 3.4, ROBE);
    cap(&mut s, v(cx, 100.0, cz + 0.2), v(cx, 111.0, cz + 1.2), 2.6, 2.3, LIVING);
    // Ribs of bone across the dead side of the bodice, a seam of lacing up its middle.
    for y in 82..=96 {
        for x in (cx - 8.0) as i32..=(cx + 8.0) as i32 {
            let Some(z) = front(&s, x, y, d) else { continue };
            if s.get(x, y, z) != Some(ROBE) {
                continue;
            }
            let dx = x as f32 - cx;
            if dx < -1.0 && (y - 82) % 3 != 2 && dx > -7.0 + (y - 82) as f32 * 0.1 {
                s.recolor(x, y, z, if (y - 82) % 3 == 0 { SKULL } else { SKULL_D });
            } else if dx.abs() < 0.6 && y % 2 == 0 {
                s.recolor(x, y, z, ROBE_L);
            }
        }
    }
    // The girdle of bones, a little skull for its clasp.
    for y in 75..=77 {
        for x in 0..w {
            for z in 0..d {
                if s.get(x, y, z) == Some(ROBE) && open(&s, x, y, z) {
                    s.recolor(x, y, z, if (x + z) % 3 == 0 { SKULL_D } else { SKULL });
                }
            }
        }
    }
    let gz = front(&s, cx as i32, 76, d).unwrap_or(35) as f32;
    ball(&mut s, v(cx, 76.2, gz + 0.6), v(1.8, 2.0, 1.4), SKULL);
    for x in [cx - 0.8, cx + 0.8] {
        put(&mut s, v(x, 76.6, gz + 1.8), CAVITY);
    }

    // ---- the gown: flaring from the waist to the floor in deep folds, trailing behind her; it and
    //      the layers over it are filled within, round her legs, so that nothing of their insides is
    //      drawn
    let hem = |ang: f32, long: f32| {
        let strip = ((ang + PI) / TAU * 26.0).floor() as i32;
        (nz(strip, 3, 9) * long) as i32
    };
    for y in 0..=78 {
        let t = (78 - y) as f32 / 78.0;
        let rx = 6.8 + 10.2 * t.powf(1.3);
        let (rf, rb) = (5.4 + 4.4 * t.powf(1.3), 5.4 + 8.6 * t.powf(1.2) + 12.0 * (t - 0.55).max(0.0).powi(2) * 4.0);
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = (x as f32 - cx, z as f32 - cz);
                let ang = dz.atan2(dx);
                let fold = (ang * 9.0 + y as f32 * 0.035).sin();
                let k = 1.0 + 0.05 * fold * t.max(0.25);
                let rz = if dz > 0.0 { rf } else { rb };
                let (ox, oz) = (rx * k, rz * k);
                if (dx / ox).powi(2) + (dz / oz).powi(2) > 1.0 {
                    continue;
                }
                if y < hem(dz.atan2(dx.abs()), 3.0) {
                    continue;
                }
                let c = if fold > 0.45 {
                    GOWN_L
                } else if fold < -0.55 {
                    GOWN_D
                } else {
                    GOWN
                };
                lay(&mut s, v(x as f32, y as f32, z as f32), c);
            }
        }
    }
    // The over-robe: open down the front over the bodice and the gown, fuller, torn at the hem.
    for y in 0..=100 {
        let t = ((78 - y) as f32 / 78.0).max(0.0);
        let above = (y - 78).max(0) as f32;
        let rx = if y > 78 { 8.6 + above * 0.08 } else { 8.4 + 11.2 * t.powf(1.25) };
        let (rf, rb) = if y > 78 {
            (6.6, 6.8)
        } else {
            (6.4 + 5.4 * t.powf(1.25), 6.6 + 10.6 * t.powf(1.15) + 12.0 * (t - 0.55).max(0.0).powi(2) * 4.0)
        };
        let opening = if y > 78 { 2.2 + above * 0.2 } else { 2.6 + (78 - y) as f32 * 0.17 };
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = (x as f32 - cx, z as f32 - cz);
                let ang = dz.atan2(dx);
                let fold = (ang * 7.0 - y as f32 * 0.03).sin();
                let k = 1.0 + 0.06 * fold * t.max(0.15);
                let rz = if dz > 0.0 { rf } else { rb };
                let (ox, oz) = (rx * k, rz * k);
                if (dx / ox).powi(2) + (dz / oz).powi(2) > 1.0 {
                    continue;
                }
                if dz > 1.0 && dx.abs() < opening {
                    continue;
                }
                if y < hem(dz.atan2(dx.abs()) + 0.13, 9.0)
                    || (y < 18 && nz(((dz.atan2(dx.abs()) + PI) / TAU * 40.0) as i32, y / 6, 4) < 0.12)
                {
                    continue;
                }
                let lapel = dz > 1.0 && dx.abs() < opening + 1.3;
                let c = if lapel {
                    LINING
                } else if fold > 0.5 {
                    ROBE_L
                } else if fold < -0.6 {
                    ROBE_D
                } else {
                    ROBE
                };
                lay(&mut s, v(x as f32, y as f32, z as f32), c);
            }
        }
    }
    // The mantle: from her shoulders down her back over the over-robe, its hem in long tatters.
    for y in 0..=101 {
        let t = (101 - y) as f32 / 101.0;
        let (rx, rb) = (10.4 + 11.0 * t.powf(1.1), 7.4 + 12.0 * t.powf(1.1) + 10.0 * (t - 0.6).max(0.0).powi(2) * 5.0);
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = (x as f32 - cx, z as f32 - cz);
                if dz > 1.5 {
                    continue;
                }
                let ang = dz.atan2(dx);
                let fold = (ang * 6.0 + y as f32 * 0.02).sin();
                let k = 1.0 + 0.07 * fold * t.max(0.1);
                let (ox, oz) = (rx * k, rb * k);
                if (dx / ox).powi(2) + (dz / oz).powi(2) > 1.0 {
                    continue;
                }
                if y < hem(dz.atan2(dx.abs()) + 0.31, 16.0) {
                    continue;
                }
                let c = if fold > 0.55 {
                    ROBE_L
                } else if fold < -0.5 {
                    ROBE_D
                } else {
                    ROBE
                };
                lay(&mut s, v(x as f32, y as f32, z as f32), c);
            }
        }
    }

    // A border embroidered round the robes above their torn hems: a purple band stitched with runes
    // in bone-white thread.
    let mut border = vec![];
    for y in 15..=19 {
        for z in 0..d {
            for x in 0..w {
                if matches!(s.get(x, y, z), Some(ROBE) | Some(ROBE_L) | Some(ROBE_D)) && open(&s, x, y, z) {
                    let ang = (z as f32 - cz).atan2(x as f32 - cx);
                    let mark = (ang * 30.0 / TAU).rem_euclid(1.0);
                    let rune = (16..=18).contains(&y)
                        && (mark < 0.12 || (y == 17 && (mark - 0.5).abs() < 0.1) || (y == 18 && (mark - 0.3).abs() < 0.08));
                    border.push((
                        x,
                        y,
                        z,
                        if y == 15 || y == 19 {
                            LINING_D
                        } else if rune {
                            SKULL_D
                        } else {
                            LINING
                        },
                    ));
                }
            }
        }
    }
    for (x, y, z, c) in border {
        s.recolor(x, y, z, c);
    }

    // ---- the hair and the head (her head and the hair above her shoulders move with it)
    s.part(3);
    let hc = v(cx, 119.0, cz + 1.6);
    ball(&mut s, hc, v(5.2, 7.4, 6.0), LIVING);
    ball(&mut s, v(cx, 112.6, cz + 4.0), v(2.5, 2.2, 2.4), LIVING);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 3.4, 118.4, cz + 5.6), Vec3::splat(1.5), LIVING);
    }
    cap(&mut s, v(cx, 119.6, cz + 7.2), v(cx, 116.6, cz + 8.3), 0.9, 1.1, LIVING);
    // The split: her left half dead, the line between ragged.
    s.paint(|x, y, _z, c| (c == LIVING && y > 98 && (x as f32) < cx - 0.2 + (nz(1, y, 3) - 0.5) * 1.4).then_some(DEAD));
    let face_at = |s: &Sculpt, x: f32, y: f32| front(s, x.round() as i32, y.round() as i32, d);
    // The dead half: the cheek sunken under a bare cheekbone, the jaw bare, the teeth lipless, the
    // eye deep in a socket of bone.
    for (x, y) in [(-3.0f32, 115.0f32), (-2.0, 115.0), (-3.0, 116.0), (-2.0, 116.0), (-4.0, 116.0)] {
        if let Some(z) = face_at(&s, cx + x, y) {
            s.clear((cx + x).round() as i32, y as i32, z);
            s.recolor((cx + x).round() as i32, y as i32, z - 1, DEAD_D);
        }
    }
    for (x, y, c) in [
        (-3.0f32, 118.0f32, SKULL),
        (-4.0, 118.0, SKULL),
        (-2.0, 118.0, SKULL_D),
        (-4.0, 113.0, SKULL),
        (-3.0, 112.0, SKULL),
        (-2.0, 112.0, SKULL_D),
        (-5.0, 121.0, SKULL),
        (-5.0, 122.0, SKULL_D),
        (-1.0, 122.0, SKULL),
        (-2.0, 122.0, SKULL),
        (-3.0, 122.0, SKULL),
    ] {
        face(&mut s, (cx + x).round() as i32, y as i32, d, 0, c);
    }
    for (x, c) in [(-3.0f32, TOOTH), (-2.0, CAVITY), (-1.0, TOOTH), (0.0, TOOTH)] {
        face(&mut s, (cx + x).round() as i32, 114, d, 0, c);
    }
    face(&mut s, (cx - 2.0).round() as i32, 113, d, 0, TOOTH);
    for (x, y) in [(-4.0f32, 117.0f32), (-5.0, 119.0), (-1.0, 116.0), (-4.0, 123.0), (-1.0, 110.0), (-2.0, 108.0)] {
        face(&mut s, (cx + x).round() as i32, y as i32, d, 0, DEAD_L);
    }
    for (x, y) in [(-4.0f32, 119.0f32), (-4.0, 120.0), (-3.0, 121.0), (-2.0, 121.0), (-1.0, 120.0), (-1.0, 119.0)] {
        face(&mut s, (cx + x).round() as i32, y as i32, d, 0, DEAD_D);
    }
    face(&mut s, (cx - 3.0).round() as i32, 120, d, 0, HEL_EYE);
    face(&mut s, (cx - 2.0).round() as i32, 120, d, 0, HEL_EYE_W);
    face(&mut s, (cx - 3.0).round() as i32, 119, d, 0, HEL_EYE);
    face(&mut s, (cx - 2.0).round() as i32, 119, d, 0, CAVITY);
    // The living half: a pale cheek shaded under its bone, a dark lip, a glowing eye under a thin
    // dark brow.
    for (x, y, c) in [
        (2.0f32, 120.0f32, HEL_EYE_W),
        (3.0, 120.0, HEL_EYE),
        (1.0, 120.0, LIVING_D),
        (2.0, 121.0, HAIR),
        (3.0, 121.0, HAIR),
        (4.0, 121.0, HAIR),
        (2.0, 122.0, HAIR_L),
        (3.0, 122.0, HAIR),
        (4.0, 117.0, LIVING_D),
        (3.0, 116.0, LIVING_D),
        (1.0, 114.0, LIP),
        (2.0, 114.0, LIP),
        (1.0, 113.0, LIP),
    ] {
        face(&mut s, (cx + x).round() as i32, y as i32, d, 0, c);
    }
    face(&mut s, cx.round() as i32, 117, d, 0, LIVING_D);
    // Long black hair, parted in the middle, falling behind her and over her shoulders before.
    let hair = v(cx, 120.4, cz + 0.6);
    let hr = v(6.0, 8.0, 6.9);
    for (x, y, z, q) in cells(hair - hr - Vec3::ONE, hair + hr + Vec3::ONE) {
        if ((q - hair) / hr).length() > 1.0 || (q.z > cz + 3.4 && q.y < 124.5) || (q.y < 116.0 && q.z > cz + 0.5) {
            continue;
        }
        let parting = (x as f32 - cx).abs() < 0.6 && q.y > 124.0;
        lay(&mut s, q, if parting || nz(x, y / 3, z) < 0.25 { HAIR_L } else { HAIR });
    }
    for k in 0..34 {
        let a = k as f32 / 34.0 * TAU;
        let (sa, ca) = (a.sin(), a.cos());
        if sa > 0.45 {
            continue;
        }
        let front_lock = ca.abs() > 0.8 && sa > -0.35;
        let (hx, hz) = (cx + ca * 6.2, cz + 0.6 + sa * 6.6);
        let end = if front_lock { 88 } else { 96 + (nz(k, 3, 1) * 5.0) as i32 };
        for y in end..=121 {
            let drop = (121 - y) as f32;
            let z = if front_lock { (hz + drop * 0.32).min(cz + 5.5) } else { hz - (drop * 0.12).min(2.5) };
            let x = hx + ca * (drop * 0.07).min(1.8) + (y as f32 * 0.4 + k as f32).sin() * 0.3;
            s.part(if y < 104 { 0 } else { 3 });
            lay(&mut s, v(x, y as f32, z), if nz(k, y, 2) < 0.25 { HAIR_L } else { HAIR });
        }
    }
    s.part(3);
    // The crown: a band of black iron round her brow, thorny spikes standing up out of it.
    for k in 0..40 {
        let a = (k as f32 / 40.0 * 1.5 - 0.75) * PI;
        let p = v(cx + a.sin() * 6.0, 124.6, cz + 1.4 + a.cos() * 6.6);
        ball(&mut s, p, v(0.9, 0.9, 0.9), if k % 5 == 0 { THORN_L } else { THORN });
    }
    for k in 0..9 {
        let a = (k as f32 / 8.0 * 1.4 - 0.7) * PI;
        let base = v(cx + a.sin() * 6.0, 125.0, cz + 1.4 + a.cos() * 6.6);
        let dir = v(a.sin() * 0.38, 1.0, a.cos() * 0.22);
        let len = 14.0 - a.abs() * 3.0 + if k == 4 { 2.0 } else { 0.0 };
        let tip = base + dir.normalize() * len;
        cap(&mut s, base, tip, 1.55, 0.3, THORN);
        for (j, f) in [0.38f32, 0.66].into_iter().enumerate() {
            let at = base + (tip - base) * f;
            let side = if (k + j) % 2 == 0 { 1.0 } else { -1.0 };
            let (u, _) = frame(dir);
            cap(&mut s, at, at + (u * side * 1.0 + Vec3::Y * 0.9).normalize() * (2.6 - f * 1.4), 0.6, 0.2, THORN_L);
        }
    }

    // ---- the high collar, flaring up behind her head, lined in purple, its edge in points
    s.part(0);
    for y in 98..=124 {
        // Close round her neck at first, a standing collar, then opening and flaring behind her head.
        let r = 4.4 + (y - 98) as f32 * 0.21;
        let gap = if y < 105 { -1.0 } else { 0.3 + ((y - 106) as f32 / 6.0).clamp(0.0, 1.0) * 0.85 };
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = (x as f32 - cx, z as f32 - (cz + 0.4));
                let ang = dx.atan2(dz);
                if ang.abs() < gap {
                    continue;
                }
                let rr = (dx * dx + dz * dz).sqrt();
                if rr > r || rr < r - 1.6 {
                    continue;
                }
                let top = 124.0 - (ang * 3.5).sin().abs() * 5.0;
                if y as f32 > top {
                    continue;
                }
                lay(
                    &mut s,
                    v(x as f32, y as f32, z as f32),
                    if rr < r - 0.8 {
                        LINING
                    } else if (ang * 3.5).sin().abs() < 0.2 {
                        ROBE_L
                    } else {
                        ROBE
                    },
                );
            }
        }
    }

    // ---- arms: thin under wide torn sleeves; the right hand on the staff, the left bare bone
    for (i, out) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        let shoulder = Vec3::from(HEL_RIG[i].pivot);
        let (elbow, wrist) = if i == 0 {
            (v(cx - 13.0, 86.0, cz + 2.5), v(cx - 12.2, 82.5, cz + 11.0))
        } else {
            (v(cx + 13.2, 86.0, cz + 1.5), v(cx + 14.6, 82.5, cz + 7.6))
        };
        let skin = if i == 0 { DEAD } else { LIVING };
        cap(&mut s, shoulder, elbow, 2.4, 2.0, skin);
        cap(&mut s, elbow, wrist, 2.0, 1.6, skin);
        bell_sleeve(&mut s, shoulder, elbow, wrist);
        if i == 0 {
            let fore = (wrist - elbow).normalize();
            for side in [-0.6f32, 0.6] {
                cap(&mut s, wrist - fore * 3.0 + Vec3::X * side, wrist + Vec3::X * side * 0.7, 0.5, 0.5, SKULL_D);
            }
            bone_hand(&mut s, wrist, v(0.15, -0.25, 1.0), out);
        } else {
            let hand = Vec3::from(HEL_HAND);
            ball(&mut s, hand, v(1.9, 2.3, 1.9), LIVING);
            for k in 0..3 {
                let y = hand.y - 1.4 + k as f32 * 1.3;
                cap(
                    &mut s,
                    v(hand.x - 1.6, y, hand.z + 1.0),
                    v(hand.x + 0.4, y, hand.z + 2.0),
                    0.7,
                    0.7,
                    if k == 1 { LIVING_D } else { LIVING },
                );
            }
            hel_staff(&mut s, hand, 0.0, 134.0);
        }
    }

    // ---- the cloth: shadowed in its folds and hollows
    hollows(&mut s, (w, h, d), 3, &[ROBE, ROBE_L, GOWN, GOWN_L, LINING], |_, c, fill| if fill > 0.7 { scale(c, 0.7) } else { c });
    s.finish(0.04)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_seventh_stage_foe_stands_in_the_middle_of_its_grid_and_inside_it() {
        for (name, c, size) in [
            ("frost giant", frost_giant(), FROST_GIANT_SIZE),
            ("Surtr", surtr(), SURTR_SIZE),
            ("Nidhogg", nidhogg(), NIDHOGG_SIZE),
            ("Hel", hel(), HEL_SIZE),
        ] {
            assert!(c.voxels.len() > 2000, "{name}: {} voxels", c.voxels.len());
            assert_eq!((c.width, c.height, c.depth), size, "{name}");
            // Its feet on the floor, and its footprint about the grid's middle (where the rig puts its place).
            assert!(c.voxels.iter().map(|v| v.y).min().is_some_and(|y| y <= 1), "{name} stands on the floor");
            assert!(!c.voxels.iter().any(|v| v.x == 0), "{name}: a stray voxel on the grid's edge (a NaN)");
            let floor: Vec<_> = c.voxels.iter().filter(|v| v.y <= 2).collect();
            let mx = floor.iter().map(|v| v.x as f32).sum::<f32>() / floor.len() as f32;
            assert!((mx - (size.0 - 1) as f32 / 2.0).abs() < 3.0, "{name}: footprint centred at x {mx}");
            // Nothing cut off: a voxel to spare at every side but the floor.
            let (w, h, d) = size;
            assert!(c.voxels.iter().all(|v| v.x < w - 1 && v.y < h - 1 && v.z > 0 && v.z < d - 1), "{name} touches its grid's edge");
        }
    }

    #[test]
    fn every_seventh_stage_pivot_lies_inside_its_grid() {
        for (name, rig, size) in [
            ("frost giant", FROST_GIANT_RIG, FROST_GIANT_SIZE),
            ("Surtr", SURTR_RIG, SURTR_SIZE),
            ("Nidhogg", NIDHOGG_RIG, NIDHOGG_SIZE),
            ("Hel", HEL_RIG, HEL_SIZE),
        ] {
            let (w, h, d) = (size.0 as f32, size.1 as f32, size.2 as f32);
            for p in rig {
                let [x, y, z] = p.pivot;
                assert!(
                    (0.0..w).contains(&x) && (0.0..h).contains(&y) && (0.0..d).contains(&z),
                    "{name}'s {} pivot {:?} outside {size:?}",
                    p.name,
                    p.pivot
                );
            }
        }
    }

    #[test]
    fn surtrs_sword_and_hels_staff_swing_with_their_right_arms() {
        // The voxels nearest the sword's point and the staff's gem belong to the "rightArm" part.
        let near = |c: &Collected, p: Vec3| {
            c.voxels
                .iter()
                .min_by(|a, b| {
                    (v(a.x as f32, a.y as f32, a.z as f32) - p).length().total_cmp(&(v(b.x as f32, b.y as f32, b.z as f32) - p).length())
                })
                .map(|v| v.part)
        };
        let (hand, dir) = (Vec3::from(SURTR_GRIP.0), Vec3::from(SURTR_GRIP.1).normalize());
        let arm = SURTR_RIG.iter().position(|p| p.name == "rightArm").map(|i| i as u8 + 1);
        assert_eq!(near(&surtr(), hand + dir * (SURTR_BLADE + 8.0)), arm, "Surtr's sword point");
        let arm = HEL_RIG.iter().position(|p| p.name == "rightArm").map(|i| i as u8 + 1);
        assert_eq!(near(&hel(), Vec3::from(HEL_HAND) + v(0.0, 62.0, 0.0)), arm, "Hel's staff gem");
    }

    /// Every model's sheet (front, side, back / top, two three-quarter views), into VH_PREVIEW_DIR.
    #[test]
    #[ignore]
    fn preview_stage_seven() {
        for (name, c) in [("frost_giant", frost_giant()), ("surtr", surtr()), ("nidhogg", nidhogg()), ("hel", hel())] {
            println!("{}", super::super::preview::sheet(&c, name));
        }
    }
}
