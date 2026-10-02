//! The third stage's cast, sculpted as finely as the second (see `foes2`): the tengu, a
//! red-faced mountain goblin with a nose as long as a hand and a white mane, in a pilgrim's
//! jacket hung with pompoms, his katana held low; the oni, a horned red demon with a curved
//! blade in each fist and a tufted tail; the dire wolf, grey and bigger than the loup-garou, a
//! mane of spikes on its shoulders; the jorogumo, a pale spider-woman on eight long legs with a
//! sickle in her hand; and the cobra, coiled, its hood spread over its open fangs. Each faces
//! +z, stands in the middle of its grid (where its rig sets it down), and is sculpted in the
//! parts its rig swings.
//!
//! Between them they fight all three ways (COMBAT.md §10): the tengu and the oni with their
//! blades, the dire wolf and the cobra with their whole bodies, the jorogumo from afar.

use std::f32::consts::{PI, TAU};

use glam::Vec3;

use super::carve::*;
use super::characters::{mat, Sculpt};
use super::voxels::{Collected, PartDef};

mod pal {
    use super::mat;
    pub const CAVITY: u32 = 0x170f0e;
    pub const TOOTH: u32 = 0xf0eadc | mat::IVORY;
    pub const STEEL: u32 = 0xc4ccd6 | mat::METAL;
    pub const STEEL_L: u32 = 0xeef2f6 | mat::METAL;
    pub const STEEL_D: u32 = 0x868e98 | mat::METAL;
    // The tengu.
    pub const T_RED: u32 = 0xc8322a | mat::SKIN;
    pub const T_RED_D: u32 = 0x962420 | mat::SKIN;
    pub const T_RED_L: u32 = 0xe2503e | mat::SKIN;
    pub const WHITE: u32 = 0xeeeae2 | mat::FUR;
    pub const WHITE_S: u32 = 0xc9c4ba | mat::FUR;
    pub const TOKIN: u32 = 0x1a1818 | mat::LEATHER;
    pub const TOKIN_L: u32 = 0x3a3634 | mat::LEATHER;
    pub const JACKET: u32 = 0xb4a078 | mat::CLOTH;
    pub const JACKET_D: u32 = 0x8e7c56 | mat::CLOTH;
    pub const JACKET_L: u32 = 0xcab894 | mat::CLOTH;
    pub const KIMONO: u32 = 0x464a52 | mat::CLOTH;
    pub const KIMONO_L: u32 = 0x8e949e | mat::CLOTH;
    pub const HAKAMA: u32 = 0x2a3046 | mat::CLOTH;
    pub const HAKAMA_D: u32 = 0x1b1f30 | mat::CLOTH;
    pub const OBI: u32 = 0x22283a | mat::CLOTH;
    pub const WRAP: u32 = 0xa6a8ae | mat::CLOTH;
    pub const WRAP_D: u32 = 0x84868e | mat::CLOTH;
    pub const TABI: u32 = 0xf0eee8 | mat::CLOTH;
    pub const TABI_D: u32 = 0xc8c6c0 | mat::CLOTH;
    pub const GETA: u32 = 0xc4a272 | mat::WOOD;
    pub const GETA_D: u32 = 0x8e6e44 | mat::WOOD;
    pub const THONG: u32 = 0xa82a2a | mat::CLOTH;
    pub const CORD: u32 = 0xeae6dc | mat::CLOTH;
    pub const POM: u32 = 0xec7a2c | mat::FUR;
    pub const POM_D: u32 = 0xc05c1c | mat::FUR;
    pub const POUCH: u32 = 0x5e5442 | mat::CLOTH;
    pub const POUCH_D: u32 = 0x40382a | mat::CLOTH;
    pub const TSUKA: u32 = 0x18181c | mat::CLOTH;
    pub const SAME: u32 = 0xeae4d4 | mat::IVORY;
    pub const TSUBA: u32 = 0xc89c3c | mat::GOLD;
    pub const TSUBA_D: u32 = 0x8a6a26 | mat::GOLD;
    pub const SAYA: u32 = 0x1e1816 | mat::WOOD;
    // The oni.
    pub const O_SKIN: u32 = 0xb43a30 | mat::SKIN;
    pub const O_SKIN_D: u32 = 0x862a24 | mat::SKIN;
    pub const O_CLOTH: u32 = 0x962c26 | mat::CLOTH;
    pub const O_CLOTH_D: u32 = 0x6a1c18 | mat::CLOTH;
    pub const O_CLOTH_L: u32 = 0xb4463a | mat::CLOTH;
    pub const TROUSER: u32 = 0x4a1e1c | mat::CLOTH;
    pub const TROUSER_D: u32 = 0x341412 | mat::CLOTH;
    pub const LEATHER: u32 = 0x3a3634 | mat::LEATHER;
    pub const LEATHER_L: u32 = 0x5a5550 | mat::LEATHER;
    pub const FOOT: u32 = 0x34302e | mat::LEATHER;
    pub const CLAW: u32 = 0x1e1a18 | mat::IVORY;
    pub const HORN: u32 = 0xd6caae | mat::IVORY;
    pub const HORN_D: u32 = 0xa49676 | mat::IVORY;
    pub const HORN_TIP: u32 = 0x5e5240 | mat::IVORY;
    pub const O_HAIR: u32 = 0x151316 | mat::FUR;
    pub const O_HAIR_L: u32 = 0x2e2a30 | mat::FUR;
    pub const BONE: u32 = 0xccc4ac | mat::IVORY;
    pub const EYE_Y: u32 = 0xf2c232;
    pub const GRIP: u32 = 0x2a2220 | mat::LEATHER;
    pub const TUFT: u32 = 0x1a1414 | mat::FUR;
    // The dire wolf.
    pub const GREY: u32 = 0x4c5056 | mat::FUR;
    pub const GREY_D: u32 = 0x2e3136 | mat::FUR;
    pub const GREY_L: u32 = 0x70757c | mat::FUR;
    pub const GREY_TIP: u32 = 0x9ca2aa | mat::FUR;
    pub const MUZZLE: u32 = 0x5c6066 | mat::FUR;
    pub const NOSE: u32 = 0x101010;
    pub const GUM: u32 = 0x5a2228 | mat::SKIN;
    pub const MOUTH: u32 = 0x2a0e10;
    pub const CLAW_D: u32 = 0x121212 | mat::IVORY;
    pub const EAR_IN: u32 = 0x3a2a2e | mat::SKIN;
    pub const WOLF_EYE: u32 = 0xf0b022;
    // The jorogumo.
    pub const PALE: u32 = 0xc6c8d0 | mat::SKIN;
    pub const PALE_D: u32 = 0x9a9ca8 | mat::SKIN;
    pub const J_HAIR: u32 = 0x26222c | mat::FUR;
    pub const J_HAIR_L: u32 = 0x463f4e | mat::FUR;
    pub const LIP: u32 = 0x5a3446 | mat::SKIN;
    pub const J_EYE: u32 = 0xb02a3a;
    pub const TOP: u32 = 0x38323e | mat::LEATHER;
    pub const TOP_L: u32 = 0x5a5262 | mat::LEATHER;
    pub const NAIL: u32 = 0x2a2228 | mat::IVORY;
    pub const SPIKE: u32 = 0xd8d0c0 | mat::IVORY;
    pub const SHELL: u32 = 0x5a3430 | mat::LEATHER;
    pub const SHELL_D: u32 = 0x3a2220 | mat::LEATHER;
    pub const SHELL_L: u32 = 0x7c4c42 | mat::LEATHER;
    pub const LEG: u32 = 0x6e3c34 | mat::LEATHER;
    pub const LEG_D: u32 = 0x4a2622 | mat::LEATHER;
    pub const JOINT: u32 = 0xb89c8c | mat::IVORY;
    pub const LEG_TIP: u32 = 0x1e1616 | mat::IVORY;
    pub const BULB: u32 = 0x6a6058 | mat::LEATHER;
    pub const BULB_D: u32 = 0x463e38 | mat::LEATHER;
    pub const BULB_L: u32 = 0x8c8074 | mat::LEATHER;
    pub const BRISTLE: u32 = 0xa69686 | mat::FUR;
    pub const HANDLE: u32 = 0x5a3c22 | mat::WOOD;
    // The cobra.
    pub const SCALE: u32 = 0x6a7448 | mat::LEATHER;
    pub const SCALE_D: u32 = 0x3c4228 | mat::LEATHER;
    pub const SCALE_L: u32 = 0x8c9662 | mat::LEATHER;
    pub const BELLY: u32 = 0xe4d6a6 | mat::IVORY;
    pub const BELLY_D: u32 = 0xc4b486 | mat::IVORY;
    pub const MARK: u32 = 0x2c3020 | mat::LEATHER;
    pub const PINK: u32 = 0xd47a88 | mat::SKIN;
    pub const TONGUE: u32 = 0xb8304e | mat::SKIN;
    pub const SNAKE_EYE: u32 = 0xf0c020;
}
use pal::*;

/// Sets the voxel nearest `p`.
fn put(s: &mut Sculpt, p: Vec3, c: u32) {
    s.set(p.x.round() as i32, p.y.round() as i32, p.z.round() as i32, c);
}

/**
 * A body swept along `knots` (centre, radius), each voxel coloured by `paint(voxel, its
 * nearest point on the axis, how far along the whole body that is, 0 to 1)`: a snake's coils,
 * a spider's legs.
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

fn bezier(p: [Vec3; 3], t: f32) -> Vec3 {
    let u = 1.0 - t;
    p[0] * (u * u) + p[1] * (2.0 * u * t) + p[2] * (t * t)
}

fn bezier_tangent(p: [Vec3; 3], t: f32) -> Vec3 {
    ((p[1] - p[0]) * (2.0 * (1.0 - t)) + (p[2] - p[1]) * (2.0 * t)).normalize_or_zero()
}

/**
 * A flat curved blade along the curve `p`, up to `width` across, widening toward `side` (the
 * outside of its curve) and two voxels thick along `through`: its spine dark, its edge bright,
 * its point sharp.
 */
fn crescent(s: &mut Sculpt, p: [Vec3; 3], width: f32, side: Vec3, through: Vec3) {
    let n = 90;
    for k in 0..=n {
        let t = k as f32 / n as f32;
        let c = bezier(p, t);
        let tan = bezier_tangent(p, t);
        let nrm = (side - tan * side.dot(tan)).normalize_or_zero();
        // (sin(pi) is a hair below zero in f32: clamped, or the point would be NaN.)
        let wide = width * (t * PI).sin().max(0.0).powf(0.55) * (1.0 - 0.35 * t) + 0.7;
        let steps = (wide * 2.0).ceil().max(1.0) as i32;
        for j in 0..=steps {
            let u = j as f32 / steps as f32;
            let at = c + nrm * (u - 0.3) * wide;
            let col = if u > 0.78 {
                STEEL_L
            } else if u < 0.18 {
                STEEL_D
            } else {
                STEEL
            };
            put(s, at, col);
            put(s, at + through, col);
        }
    }
}

/// A flat disc of radius `r` about `c`, facing along `n` (a sword's guard).
fn disc(s: &mut Sculpt, c: Vec3, n: Vec3, r: f32, color: u32) {
    let n = n.normalize();
    let u = n.cross(if n.y.abs() < 0.9 { Vec3::Y } else { Vec3::X }).normalize();
    let w = n.cross(u);
    let steps = (r * 6.0) as i32;
    for i in -steps..=steps {
        for j in -steps..=steps {
            let (a, b) = (i as f32 / steps as f32, j as f32 / steps as f32);
            if a * a + b * b <= 1.0 {
                put(s, c + u * a * r + w * b * r, color);
            }
        }
    }
}

/// Stiff hairs standing out of the `on` voxels, about `density` of them (a spider's bristles).
fn bristles(s: &mut Sculpt, dims: (i32, i32, i32), on: &[u32], density: f32, c: u32) {
    let (w, h, d) = dims;
    let mut add = vec![];
    for y in 1..h {
        for z in 0..d {
            for x in 0..w {
                let Some(col) = s.get(x, y, z) else { continue };
                if !on.contains(&col) || nz(x * 5, y * 3, z * 7) > density {
                    continue;
                }
                if let Some((dx, dy, dz)) = [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, 0, 1), (0, 0, -1)]
                    .into_iter()
                    .find(|&(dx, dy, dz)| s.get(x + dx, y + dy, z + dz).is_none())
                {
                    add.push((x + dx, y + dy, z + dz, s.tag_at(x, y, z)));
                }
            }
        }
    }
    for (x, y, z, tag) in add {
        s.part(tag);
        s.set(x, y, z, c);
    }
    s.part(0);
}

// ---------------------------------------------------------------------- tengu

/// The tengu's grid: 48 wide, 92 tall, 56 deep (his katana reaches far before him).
pub const TENGU_SIZE: (i32, i32, i32) = (48, 92, 56);
const TENGU_Z: f32 = 27.5;

/// Hips, shoulders and neck; the katana goes with the right hand.
pub const TENGU_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [18.5, 34.0, TENGU_Z] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [28.5, 34.0, TENGU_Z] },
    PartDef { name: "leftArm", boxes: &[], pivot: [12.0, 62.0, TENGU_Z] },
    PartDef { name: "rightArm", boxes: &[], pivot: [35.0, 62.0, TENGU_Z] },
    PartDef { name: "head", boxes: &[], pivot: [23.5, 67.0, TENGU_Z] },
];

/// An orange pompom of the pilgrim's sash, darker underneath.
fn pompom(s: &mut Sculpt, c: Vec3, r: f32) {
    for y in (c.y - r).floor() as i32..=(c.y + r).ceil() as i32 {
        for z in (c.z - r).floor() as i32..=(c.z + r).ceil() as i32 {
            for x in (c.x - r).floor() as i32..=(c.x + r).ceil() as i32 {
                let p = Vec3::new(x as f32, y as f32, z as f32) - c;
                if p.length() <= r {
                    s.set(x, y, z, if p.y < -r * 0.35 || nz(x, y, z) < 0.2 { POM_D } else { POM });
                }
            }
        }
    }
}

/// The tengu's katana in the fist at `hand`, held low: the grip through the fist, a gilt guard,
/// and the blade down and forward past his foot, curving gently back, its edge leading.
fn katana(s: &mut Sculpt, hand: Vec3, out: f32) {
    let dir = Vec3::new(0.14 * out, -0.84, 0.52).normalize();
    for k in -5..=4 {
        let p = hand + dir * (k as f32 * 0.95);
        ball(s, p, Vec3::splat(1.1), if k % 2 == 0 { TSUKA } else { SAME });
    }
    ball(s, hand - dir * 5.4, Vec3::splat(1.25), TSUBA_D);
    disc(s, hand + dir * 4.9, dir, 2.5, TSUBA);
    let fwd = (Vec3::Z - dir * dir.dot(Vec3::Z)).normalize();
    let len = 34.0;
    for k in 0..=(len as i32 * 2) {
        let a = k as f32 * 0.5;
        let t = a / len;
        let c = hand + dir * (5.6 + a) - fwd * (2.4 * t * t);
        put(s, c, STEEL);
        put(s, c + fwd * 0.9, STEEL_L);
        if t < 0.9 {
            put(s, c - fwd * 0.9, STEEL_D);
        }
    }
}

/**
 * The tengu: a red face with a nose as long as a hand, white brows bristling over glaring
 * eyes, a white mane and a long pointed beard, a little black cap on his brow; a pilgrim's
 * jacket of sand-coloured hemp over a grey kimono, orange pompoms down its front, a white
 * cord knotted at the sash over a woven apron; wide navy hakama, grey leg wraps, white tabi
 * on wooden geta; a black scabbard through his sash and the katana held low in his right fist.
 */
pub fn tengu() -> Collected {
    let (w, h, d) = TENGU_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (23.5f32, TENGU_Z);

    // ---- legs: geta on two teeth, split-toed tabi, leg wraps, the hakama's wide legs
    for (i, lx) in [18.5f32, 28.5].into_iter().enumerate() {
        s.part(1 + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        let fx = lx + out * 0.4;
        rounded(&mut s, v(fx, 3.4, cz + 1.6), v(3.4, 0.9, 6.6), 3.0, 0, GETA);
        for tz in [cz - 2.6, cz + 5.2] {
            let z = tz.round() as i32;
            s.cuboid((fx - 2.8).round() as i32, 0, z, (fx + 2.8).round() as i32, 2, z + 1, GETA_D);
        }
        s.ellipsoid((fx, 6.2, cz + 2.0), (2.9, 2.3, 5.0), 5, TABI);
        let split = (fx - out * 0.9).round() as i32;
        for z in (cz + 4.6) as i32..=(cz + 7.2) as i32 {
            for y in 5..=8 {
                if s.get(split, y, z) == Some(TABI) {
                    s.recolor(split, y, z, TABI_D);
                }
            }
        }
        for side in [-1.0f32, 1.0] {
            s.capsule((split as f32, 7.8, cz + 5.0), (fx + side * 2.5, 6.4, cz + 1.6), 0.6, 0.5, THONG);
        }
        s.capsule((lx, 7.6, cz), (lx, 21.0, cz), 2.5, 2.9, WRAP);
        // The hakama's leg, pleated, open at its hem round the wrap.
        for y in 17..=34 {
            let t = (34 - y) as f32 / 17.0;
            let (rx, rz) = (4.9 + t * 0.9, 5.1 + t * 0.7);
            let lxx = lx + out * 0.6 * t;
            for z in 0..d {
                for x in 0..w {
                    let (dx, dz) = ((x as f32 - lxx) / rx, (z as f32 - cz) / rz);
                    let q = dx * dx + dz * dz;
                    if q > 1.0 || (y <= 19 && q < 0.55) {
                        continue;
                    }
                    s.set(x, y, z, if x % 3 == 0 { HAKAMA_D } else { HAKAMA });
                }
            }
        }
    }
    s.paint(|x, y, z, c| (c == WRAP && (y + (x + z) / 3) % 4 == 0).then_some(WRAP_D));

    // ---- the hakama's seat, up to the sash
    s.part(0);
    for y in 33..=45 {
        let t = (y - 33) as f32 / 12.0;
        slab(&mut s, cx, cz, 11.4 - t * 1.6, 6.9 - t * 0.6, 2.4, y, y, HAKAMA);
    }
    s.paint(|x, y, _z, c| (c == HAKAMA && y >= 33 && x % 3 == 0).then_some(HAKAMA_D));

    // ---- the kimono: grey, its pale collar crossing at the chest
    for y in 45..=63 {
        let t = (y - 45) as f32 / 18.0;
        slab(&mut s, cx, cz, 8.4 + t * 1.2, 5.5 + t * 0.4, 2.6, y, y, KIMONO);
    }
    for y in 50..=63 {
        let k = (y - 50) as f32 * 0.42;
        for side in [-1.0f32, 1.0] {
            for j in 0..2 {
                face(&mut s, (cx + side * (k + j as f32)).round() as i32, y, d, 0, KIMONO_L);
            }
        }
    }

    // ---- the sash, a white cord knotted at its left, a woven apron hanging before him
    slab(&mut s, cx, cz, 10.0, 6.7, 2.4, 44, 47, OBI);
    let fz = front(&s, (cx - 4.0) as i32, 45, d).unwrap_or(34) as f32;
    let knot = v(cx - 4.0, 45.6, fz + 1.0);
    for side in [-1.0f32, 1.0] {
        s.capsule((knot.x, knot.y, knot.z), (knot.x + side * 2.6, 47.4, knot.z + 0.4), 0.7, 0.7, CORD);
        s.capsule((knot.x, knot.y, knot.z), (knot.x + side * 2.2, 44.0, knot.z + 0.4), 0.7, 0.7, CORD);
    }
    for (end, len) in [(-1.2f32, 9.4f32), (1.4, 8.4)] {
        s.capsule((knot.x, knot.y - 0.6, knot.z + 0.2), (knot.x + end, knot.y - len, knot.z + 0.6), 0.6, 0.6, CORD);
        ball(&mut s, v(knot.x + end, knot.y - len - 0.6, knot.z + 0.6), Vec3::splat(0.95), CORD);
    }
    let az = front(&s, cx as i32, 40, d).unwrap_or(34);
    let (ax0, ax1) = ((cx - 3.0).round() as i32, (cx + 3.0).round() as i32);
    for y in 33..=43 {
        for x in ax0..=ax1 {
            let edge = y == 33 || y == 43 || x == ax0 || x == ax1;
            s.set(x, y, az + 1, if edge || (x + y) % 2 == 0 { POUCH_D } else { POUCH });
        }
    }

    // ---- the jacket: sand-coloured hemp, open down the front, flaring below the sash, ragged at the hem
    let open = |y: i32| 2.6 + (63 - y).max(0) as f32 * 0.1;
    for y in 31..=65 {
        let flare = ((45 - y).max(0) as f32 / 14.0) * 2.6;
        let (rx, rz) = (10.6 + flare, 7.3 + flare * 0.6);
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = ((x as f32 - cx) / rx, (z as f32 - cz) / rz);
                if dx.abs().powf(2.3) + dz.abs().powf(2.3) > 1.0 {
                    continue;
                }
                let (ix, iz) = ((x as f32 - cx) / (rx - 1.6), (z as f32 - cz) / (rz - 1.6));
                let inner = ix.abs().powf(2.3) + iz.abs().powf(2.3) < 1.0;
                let neck = (x as f32 - cx).powi(2) + (z as f32 - cz - 0.5).powi(2) < 7.5;
                let hollow = if y >= 63 { neck } else { inner };
                if hollow {
                    continue;
                }
                if z as f32 > cz + 2.0 && (x as f32 - cx).abs() < open(y) {
                    continue;
                }
                if y <= 32 && nz(x, 1, z) < 0.4 {
                    continue;
                }
                let fold = ((z as f32 - cz).atan2(x as f32 - cx) * 6.0 + y as f32 * 0.12).sin();
                s.set(
                    x,
                    y,
                    z,
                    if fold > 0.78 {
                        JACKET_D
                    } else if fold < -0.85 {
                        JACKET_L
                    } else {
                        JACKET
                    },
                );
            }
        }
    }
    // Pompoms down the jacket's front edges, the biggest at the top.
    for (y, r) in [(59.0f32, 1.9f32), (55.0, 1.6), (51.0, 1.3)] {
        for side in [-1.0f32, 1.0] {
            let x = cx + side * (open(y as i32) + 1.5);
            let z = front(&s, x.round() as i32, y as i32, d).unwrap_or(34) as f32 + r * 0.5;
            pompom(&mut s, v(x, y, z), r);
        }
    }
    // The scabbard, thrust through the sash at his left hip, pointing back and down.
    s.capsule((cx - 6.0, 46.4, cz + 6.4), (cx - 13.6, 30.0, cz - 9.6), 1.3, 1.2, SAYA);
    ball(&mut s, v(cx - 6.0, 46.4, cz + 6.4), Vec3::splat(1.5), TSUBA_D);
    ball(&mut s, v(cx - 13.8, 29.6, cz - 10.0), Vec3::splat(1.4), TSUBA_D);
    s.capsule((cx, 61.5, cz), (cx, 67.5, cz + 0.6), 2.5, 2.3, T_RED_D);

    // ---- arms: wide jacket sleeves to the elbow, the kimono's to the wrist, red fists
    for (i, ax) in [12.0f32, 35.0].into_iter().enumerate() {
        s.part(3 + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        ball(&mut s, v(ax, 61.5, cz), Vec3::splat(3.8), JACKET);
        sleeve(
            &mut s,
            &[(v(ax, 61.5, cz), 3.6), (v(ax + out * 1.3, 54.0, cz + 1.2), 4.0), (v(ax + out * 1.1, 48.5, cz + 2.8), 4.7)],
            JACKET,
            JACKET_D,
        );
        s.capsule((ax + out * 1.1, 51.0, cz + 2.2), (ax + out * 0.7, 43.0, cz + 5.0), 2.3, 2.0, KIMONO);
        let hand = v(ax + out * 0.7, 41.0, cz + 6.0);
        ball(&mut s, hand, v(2.2, 2.4, 2.3), T_RED);
        for k in -1..=1 {
            s.set((hand.x + k as f32).round() as i32, (hand.y + 0.6).round() as i32, (hand.z + 2.4).round() as i32, T_RED_D);
        }
        if i == 1 {
            katana(&mut s, hand, out);
        }
    }

    // ---- the head: red, the long nose, glaring eyes under white brows, a grimace, the beard and mane
    s.part(5);
    let hz = cz + 0.5;
    ball(&mut s, v(cx, 76.0, hz), v(6.0, 7.0, 6.2), T_RED);
    rounded(&mut s, v(cx, 70.6, hz + 1.8), v(4.3, 2.8, 4.5), 2.6, 0, T_RED);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 3.5, 73.6, hz + 3.6), Vec3::splat(2.3), T_RED);
    }
    s.capsule((cx - 4.8, 79.8, hz + 4.5), (cx + 4.8, 79.8, hz + 4.5), 1.6, 1.6, T_RED_D);
    s.capsule((cx, 77.4, hz + 5.2), (cx, 74.8, hz + 15.0), 2.1, 1.25, T_RED);
    ball(&mut s, v(cx, 74.7, hz + 15.0), Vec3::splat(1.35), T_RED_L);
    let nostril_z = (hz + 14.0).round() as i32;
    for x in [cx.floor() as i32, cx.ceil() as i32] {
        if let Some(y) = (70..=76).find(|&y| s.get(x, y, nostril_z).is_some()) {
            s.recolor(x, y, nostril_z, CAVITY);
        }
    }
    for (x0, inward) in [(19, 1), (28, -1)] {
        for y in 77..=78 {
            face(&mut s, x0, y, d, 0, EYE_WHITE);
            face(&mut s, x0 + inward, y, d, 0, PUPIL);
        }
        face(&mut s, x0 + inward, 78, d, 0, GLINT);
    }
    for x in 20..=27 {
        face(&mut s, x, 71, d, 0, CAVITY);
        face(&mut s, x, 72, d, 0, if x % 2 == 0 { TOOTH } else { CAVITY });
    }
    // White brows, bristling up and out.
    for side in [-1.0f32, 1.0] {
        for k in 0..6 {
            let x = cx + side * (1.4 + k as f32 * 0.9);
            let y = 79.6 + k as f32 * 0.35;
            let z = front(&s, x.round() as i32, y.round() as i32, d).unwrap_or(30) as f32;
            ball(&mut s, v(x, y, z + 0.4), v(0.9, 1.0, 0.9), WHITE);
        }
        s.capsule((cx + side * 6.0, 81.6, hz + 3.0), (cx + side * 8.4, 83.6, hz + 1.8), 0.9, 0.4, WHITE);
        let z = front(&s, (cx + side * 1.5).round() as i32, 73, d).unwrap_or(30) as f32;
        s.capsule((cx + side * 1.2, 73.0, z + 0.5), (cx + side * 5.6, 70.8, hz + 4.6), 0.9, 0.6, WHITE);
    }
    // The beard: long and pointed, down over his chest.
    for y in 54..=70 {
        let k = (70 - y) as f32 / 16.0;
        let (rx, rz) = (4.4 * (1.0 - k * 0.82) + 0.5, 1.9 + (1.0 - k) * 0.7);
        let zc = hz + 6.4 + k * 1.4;
        for z in (zc - rz).floor() as i32..=(zc + rz).ceil() as i32 {
            for x in (cx - rx).floor() as i32..=(cx + rx).ceil() as i32 {
                if ((x as f32 - cx) / rx).powi(2) + ((z as f32 - zc) / rz).powi(2) <= 1.0 {
                    s.set(x, y, z, if (x + y / 3) % 3 == 0 { WHITE_S } else { WHITE });
                }
            }
        }
    }
    // The mane: over the crown and the back of the head and down his back, a lock either side of his face.
    for y in 70..=85 {
        for z in 0..d {
            for x in 0..w {
                let p = v(x as f32 - cx, y as f32 - 78.6, z as f32 - (hz - 0.9));
                if (p.x / 6.6).powi(2) + (p.y / 5.6).powi(2) + (p.z / 6.6).powi(2) > 1.0 {
                    continue;
                }
                if z as f32 > hz + 2.2 && y < 82 {
                    continue; // not over his face
                }
                s.set(x, y, z, if nz(x, y, z) < 0.3 { WHITE_S } else { WHITE });
            }
        }
    }
    for k in 0..44 {
        let a = k as f32 / 44.0 * TAU;
        let (sa, ca) = (a.sin(), a.cos());
        if sa > 0.3 {
            continue;
        }
        let lock = ca.abs() > 0.8 && sa > -0.4;
        let (hx, hzz) = (cx + ca * 6.3, hz - 0.9 + sa * 6.4);
        let end = if lock { 57 } else { 52 + (nz(k, 3, 1) * 7.0) as i32 };
        for y in end..=80 {
            let drop = (80 - y) as f32;
            let sway = (y as f32 * 0.35 + k as f32).sin() * 0.5;
            let z = if lock { (hzz + drop * 0.36).min(hz + 8.4) } else { hzz - (drop * 0.06).min(1.2) };
            let x = hx + ca * (drop * 0.05).min(0.8) + sway;
            s.set(x.round() as i32, y, z.round() as i32, if nz(k, y, 2) < 0.3 { WHITE_S } else { WHITE });
        }
    }
    for side in [-1.0f32, 1.0] {
        s.capsule((cx + side * 5.6, 77.0, hz + 0.4), (cx + side * 9.6, 80.8, hz - 0.8), 1.7, 0.5, T_RED);
        s.set((cx + side * 6.4).round() as i32, 78, (hz + 1.2).round() as i32, T_RED_D);
    }
    // The tokin: a little black pillbox on his brow.
    for y in 83..=86 {
        let r = 2.5 - (y - 83) as f32 * 0.25;
        column(&mut s, cx, hz + 2.4, r, r, y, y, TOKIN);
    }
    s.paint(|x, y, z, c| (c == TOKIN && y < 86 && (turn(x, z, cx, hz + 2.4) * 12.0).fract() < 0.3).then_some(TOKIN_L));
    s.finish(0.04)
}

// ------------------------------------------------------------------------ oni

/// The oni's grid: 64 wide (his blades), 116 tall (his horns), 40 deep.
pub const ONI_SIZE: (i32, i32, i32) = (64, 116, 40);
const ONI_Z: f32 = 19.5;

/// Hips, shoulders and neck, and the tail at the small of his back; a blade in each fist.
pub const ONI_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [25.0, 40.0, ONI_Z] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [38.0, 40.0, ONI_Z] },
    PartDef { name: "leftArm", boxes: &[], pivot: [16.0, 78.0, ONI_Z] },
    PartDef { name: "rightArm", boxes: &[], pivot: [47.0, 78.0, ONI_Z] },
    PartDef { name: "head", boxes: &[], pivot: [31.5, 84.0, ONI_Z] },
    PartDef { name: "tail", boxes: &[], pivot: [31.5, 44.0, 12.5] },
];

/**
 * The oni: a red demon a head taller than a man, horns ringed like a ram's sweeping up and out
 * from his brow, pointed ears, deep-set yellow eyes under a scowl, fangs and a black goatee, his
 * black hair long down his back; a ragged red tunic open at the chest, a broad belt with a
 * skull for its buckle over a torn red kilt; leather bracers and shin straps, clawed feet, a
 * thin tail ending in a black tuft, and a curved blade in each fist.
 */
pub fn oni() -> Collected {
    let (w, h, d) = ONI_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (31.5f32, ONI_Z);

    // ---- legs: clawed feet, red cloth at the ankle, leather straps up the shin, baggy trousers
    for (i, lx) in [25.0f32, 38.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        s.ellipsoid((lx, 2.8, cz + 2.4), (3.9, 2.9, 5.6), 0, FOOT);
        for f in [-2.3f32, 0.0, 2.3] {
            s.capsule((lx + f, 2.0, cz + 6.6), (lx + f * 1.15, 0.6, cz + 10.4), 1.1, 0.45, CLAW);
        }
        column(&mut s, lx, cz, 3.8, 4.0, 5, 7, O_CLOTH_D);
        s.capsule((lx, 7.0, cz), (lx, 22.0, cz), 3.3, 3.9, LEATHER);
        for y in 20..=40 {
            let t = (y - 20) as f32 / 20.0;
            let r = 4.5 + (t * PI * 0.85).sin() * 1.9 + t * 0.5;
            column(&mut s, lx + out * 0.5 * t, cz, r, r * 1.04, y, y, TROUSER);
        }
    }
    s.paint(|x, y, z, c| (c == LEATHER && (y * 2 + x + z).rem_euclid(7) == 0).then_some(LEATHER_L));
    s.paint(|x, y, z, c| (c == TROUSER && nz(x, y / 2, z) < 0.25).then_some(TROUSER_D));

    // ---- hips, a barrel of a belly, a broad chest, shoulders like boulders
    s.part(0);
    for y in 38..=50 {
        slab(&mut s, cx, cz, 12.6 - (y - 38) as f32 * 0.12, 7.6, 2.4, y, y, TROUSER);
    }
    rounded(&mut s, v(cx, 61.0, cz + 0.4), v(10.4, 7.6, 7.2), 2.4, 0, O_SKIN);
    rounded(&mut s, v(cx, 71.5, cz), v(13.2, 9.0, 8.4), 2.6, 0, O_SKIN);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 5.2, 73.6, cz + 5.4), v(5.0, 3.8, 3.0), O_SKIN);
        ball(&mut s, v(cx + side * 11.6, 78.4, cz), v(5.8, 4.6, 5.8), O_SKIN);
    }
    // The tunic over it: open in a V at the neck, ragged where it crosses the shoulders.
    s.paint(|x, y, z, c| {
        if c != O_SKIN || y < 53 {
            return None;
        }
        let open = y as f32 > 64.0 && (x as f32 - cx).abs() < (y as f32 - 64.0) * 0.55 + 0.5 && z as f32 > cz + 1.0;
        if open || (y > 80 && nz(x, y, z) < 0.35) {
            return None;
        }
        Some(if nz(x / 2, y / 3, z / 2) < 0.22 {
            O_CLOTH_D
        } else if nz(x, y, z) > 0.88 {
            O_CLOTH_L
        } else {
            O_CLOTH
        })
    });
    for y in 66..=74 {
        face(&mut s, cx.floor() as i32, y, d, 0, O_SKIN_D);
    }
    // A red sash, the broad belt over it, and its buckle: a skull.
    slab(&mut s, cx, cz, 12.9, 8.0, 2.4, 48, 50, O_CLOTH);
    slab(&mut s, cx, cz, 13.3, 8.4, 2.4, 51, 55, LEATHER);
    s.paint(|_x, y, _z, c| (c == LEATHER && (y == 51 || y == 55)).then_some(LEATHER_L));
    let fz = front(&s, cx as i32, 53, d).unwrap_or(28) as f32;
    ball(&mut s, v(cx, 53.6, fz + 1.0), v(3.0, 3.0, 1.9), BONE);
    rounded(&mut s, v(cx, 50.8, fz + 1.2), v(2.1, 1.4, 1.4), 2.4, 0, BONE);
    for (x, y) in [(30, 54), (33, 54), (31, 52), (32, 52)] {
        face(&mut s, x, y, d, 0, CAVITY);
    }
    for x in 30..=33 {
        face(&mut s, x, 50, d, 0, if x % 2 == 0 { CAVITY } else { BONE });
    }
    // The kilt: torn red cloth hanging before and behind (the front longest), open at the sides.
    for y in 22..=49 {
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = ((x as f32 - cx) / 13.6, (z as f32 - cz) / 8.8);
                let (ix, iz) = ((x as f32 - cx) / 12.0, (z as f32 - cz) / 7.2);
                if dx.abs().powf(2.4) + dz.abs().powf(2.4) > 1.0 || ix.abs().powf(2.4) + iz.abs().powf(2.4) < 1.0 {
                    continue;
                }
                let (fore, aft) = (z as f32 > cz + 2.0, (z as f32) < cz - 2.0);
                if !fore && !aft {
                    continue;
                }
                let middle = (x as f32 - cx).abs() < 6.5;
                let hem = match (fore, middle) {
                    (true, true) => 22,
                    (false, true) => 27,
                    _ => 33,
                };
                if y < hem + (nz(x, 1, z) * 4.0) as i32 || nz(x, y / 5, z) < 0.07 {
                    continue;
                }
                let c = if (x + z).rem_euclid(4) == 0 {
                    O_CLOTH_D
                } else if nz(x, y, z) > 0.9 {
                    O_CLOTH_L
                } else {
                    O_CLOTH
                };
                s.set(x, y, z, c);
            }
        }
    }
    // A cord round his neck with a fang on it.
    for k in 0..=24 {
        let phi = k as f32 / 24.0 * PI;
        let (x, y) = ((cx + 4.8 * phi.cos()).round() as i32, (80.0 - 8.4 * phi.sin()).round() as i32);
        if let Some(z) = front(&s, x, y, d) {
            s.set(x, y, z + 1, LEATHER_L);
        }
    }
    let nz0 = front(&s, cx as i32, 71, d).unwrap_or(28) as f32;
    s.capsule((cx, 71.6, nz0 + 1.0), (cx, 68.6, nz0 + 1.6), 1.0, 0.4, BONE);
    s.capsule((cx, 78.0, cz), (cx, 85.0, cz + 0.6), 4.4, 4.0, O_SKIN);

    // ---- arms: thick and bare, leather bracers, a curved blade in each fist
    for (i, ax) in [16.0f32, 47.0].into_iter().enumerate() {
        s.part(3 + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        ball(&mut s, v(ax, 77.6, cz), Vec3::splat(5.0), O_SKIN);
        s.capsule((ax, 77.6, cz), (ax + out * 1.6, 65.0, cz + 1.0), 4.5, 3.9, O_SKIN);
        ball(&mut s, v(ax + out * 0.4, 71.0, cz + 2.6), v(3.4, 4.2, 3.0), O_SKIN);
        s.capsule((ax + out * 1.6, 65.0, cz + 1.0), (ax + out * 2.4, 51.5, cz + 3.0), 3.9, 3.2, O_SKIN);
        // The bracer, wrist to elbow, and the ragged cap of the tunic over the shoulder.
        s.paint(|x, y, z, c| {
            let mine = (x as f32 - cx) * out > 11.0;
            if c != O_SKIN || !mine {
                return None;
            }
            if (51..=62).contains(&y) {
                Some(if (y + (x + z) / 2) % 4 == 0 { LEATHER_L } else { LEATHER })
            } else if y >= 79 && nz(x, y, z) > 0.3 {
                Some(O_CLOTH)
            } else {
                None
            }
        });
        let fist = v(ax + out * 2.4, 48.6, cz + 3.6);
        ball(&mut s, fist, v(3.1, 3.3, 3.3), O_SKIN);
        for k in -1..=1 {
            s.set((fist.x + k as f32 * 1.2).round() as i32, (fist.y + 1.0).round() as i32, (fist.z + 3.2).round() as i32, O_SKIN_D);
        }
        s.capsule((fist.x, fist.y + 4.0, fist.z), (fist.x, fist.y - 4.2, fist.z), 1.1, 1.1, GRIP);
        ball(&mut s, v(fist.x, fist.y + 4.6, fist.z), Vec3::splat(1.3), LEATHER_L);
        let p0 = v(fist.x, fist.y - 4.4, fist.z + 0.4);
        crescent(&mut s, [p0, p0 + v(out * 8.6, -2.0, 1.0), p0 + v(out * 10.8, -14.5, 1.6)], 3.8, v(out, 0.6, 0.0), Vec3::Z);
    }

    // ---- the head: a scowl, yellow eyes, fangs, pointed ears, the horns and the long black hair
    s.part(5);
    let hz = cz + 1.0;
    ball(&mut s, v(cx, 91.0, hz), v(6.8, 7.4, 6.6), O_SKIN);
    rounded(&mut s, v(cx, 85.4, hz + 2.2), v(5.8, 3.4, 4.8), 2.4, 0, O_SKIN);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 4.2, 88.2, hz + 3.6), Vec3::splat(2.6), O_SKIN);
    }
    s.capsule((cx - 5.6, 93.6, hz + 5.2), (cx + 5.6, 93.6, hz + 5.2), 1.9, 1.9, O_SKIN_D);
    ball(&mut s, v(cx, 92.8, hz + 5.9), Vec3::splat(1.7), O_SKIN_D);
    ball(&mut s, v(cx, 89.3, hz + 6.8), v(2.3, 2.0, 1.7), O_SKIN);
    for (x0, inward) in [(27, 1), (36, -1)] {
        for k in -1..=2 {
            for y in 89..=92 {
                face(&mut s, x0 + k * inward, y, d, 0, O_SKIN_D);
            }
        }
        face(&mut s, x0, 90, d, 0, EYE_Y);
        face(&mut s, x0, 91, d, 0, EYE_Y);
        face(&mut s, x0 + inward, 90, d, 0, EYE_Y);
        face(&mut s, x0 + inward, 91, d, 0, PUPIL);
    }
    for x in 28..=35 {
        face(&mut s, x, 86, d, 0, CAVITY);
        face(&mut s, x, 87, d, 0, if x % 2 == 1 { TOOTH } else { CAVITY });
    }
    for x in [29, 34] {
        face(&mut s, x, 86, d, 0, TOOTH);
        face(&mut s, x, 85, d, 1, TOOTH);
    }
    face(&mut s, 30, 88, d, 0, CAVITY);
    face(&mut s, 33, 88, d, 0, CAVITY);
    ball(&mut s, v(cx, 81.8, hz + 5.0), v(1.6, 2.2, 1.2), O_HAIR);
    for side in [-1.0f32, 1.0] {
        s.capsule((cx + side * 6.4, 90.6, hz + 0.4), (cx + side * 12.8, 94.2, hz - 0.8), 2.0, 0.4, O_SKIN);
        s.set((cx + side * 7.6).round() as i32, 91, (hz + 1.4).round() as i32, O_SKIN_D);
    }
    // The hair: over the crown and down his back.
    s.paint(|_x, y, z, c| (c == O_SKIN && y > 85 && ((z as f32) < hz - 0.5 || y > 96)).then_some(O_HAIR));
    for k in 0..40 {
        let a = k as f32 / 40.0 * TAU;
        let (sa, ca) = (a.sin(), a.cos());
        if sa > 0.1 {
            continue;
        }
        let (hx, hzz) = (cx + ca * 6.8, hz + sa * 6.6);
        let end = 66 + (nz(k, 5, 2) * 7.0) as i32;
        for y in end..=95 {
            let drop = (95 - y) as f32;
            let z = hzz - (drop * 0.18).min(3.8);
            let x = hx + ca * (drop * 0.08).min(2.5);
            s.set(x.round() as i32, y, z.round() as i32, if nz(k, y, 4) < 0.25 { O_HAIR_L } else { O_HAIR });
        }
    }
    // The horns: ringed like a ram's, sweeping up and out, darkening to their points.
    for side in [-1.0f32, 1.0] {
        let n = 80;
        for k in 0..=n {
            let t = k as f32 / n as f32;
            // Out to the side first, then up, the points turning in.
            let reach = 4.4 + 10.0 * (1.0 - (1.0 - t) * (1.0 - t)) - 3.4 * ((t - 0.6).max(0.0) / 0.4).powi(2);
            let x = cx + side * reach;
            let y = 95.5 + 16.5 * t.powf(1.3);
            let z = hz + 2.0 - 4.4 * t + 3.0 * t * t;
            let r = 3.0 - 2.4 * t;
            let c = if t > 0.86 {
                HORN_TIP
            } else if (t * 15.0).fract() < 0.22 {
                HORN_D
            } else {
                HORN
            };
            s.ellipsoid((x, y, z), (r, r, r), 0, c);
        }
    }

    // ---- the tail, curling out behind him to its tuft
    s.part(6);
    limb(
        &mut s,
        &[
            (v(cx, 44.0, cz - 7.2), 1.7),
            (v(cx + 3.0, 37.0, cz - 11.6), 1.4),
            (v(cx + 8.0, 31.0, cz - 13.6), 1.15),
            (v(cx + 13.0, 32.6, cz - 13.0), 0.95),
        ],
        O_SKIN,
    );
    ball(&mut s, v(cx + 15.0, 34.2, cz - 12.0), v(2.6, 2.2, 2.6), TUFT);
    s.finish(0.04)
}

// ------------------------------------------------------------------ dire wolf

/// The dire wolf's grid: 40 wide, 62 tall, 88 long (it faces +z).
pub const DIRE_SIZE: (i32, i32, i32) = (40, 62, 88);

/// Four legs at the shoulders and hips, the head at the neck, the tail at the rump.
pub const DIRE_RIG: &[PartDef] = &[
    PartDef { name: "frontLeftLeg", boxes: &[], pivot: [12.5, 35.0, 61.0] },
    PartDef { name: "frontRightLeg", boxes: &[], pivot: [26.5, 35.0, 61.0] },
    PartDef { name: "backLeftLeg", boxes: &[], pivot: [12.5, 34.0, 23.0] },
    PartDef { name: "backRightLeg", boxes: &[], pivot: [26.5, 34.0, 23.0] },
    PartDef { name: "head", boxes: &[], pivot: [19.5, 46.0, 67.0] },
    PartDef { name: "tail", boxes: &[], pivot: [19.5, 38.0, 13.0] },
];

/// A spike of fur from `base` along `dir`: the coat at its root, pale at its point.
fn spike(s: &mut Sculpt, base: Vec3, dir: Vec3, len: f32, r: f32) {
    let (mid, tip) = (base + dir * len * 0.55, base + dir * len);
    s.capsule((base.x, base.y, base.z), (mid.x, mid.y, mid.z), r, r * 0.65, GREY);
    s.capsule((mid.x, mid.y, mid.z), (tip.x, tip.y, tip.z), r * 0.65, 0.3, GREY_TIP);
}

/**
 * The dire wolf: a wolf bigger than the loup-garou, charcoal grey, its fur paling at the
 * tips; a great mane of spikes over its neck and shoulders and hackles standing down its
 * spine; heavy legs on black claws; a broad head with its lips drawn back off its fangs in a
 * snarl, yellow eyes, ears laid forward, and a bushy tail hanging behind.
 */
pub fn dire_wolf() -> Collected {
    let (w, h, d) = DIRE_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let cx = 19.5f32;

    // ---- legs: heavy shoulders and thighs, the hind hocks bent back, paws with black claws
    for (i, (lx, lz)) in [(12.5f32, 61.0f32), (26.5, 61.0), (12.5, 23.0), (26.5, 23.0)].into_iter().enumerate() {
        s.part(1 + i as u8);
        let out = if i.is_multiple_of(2) { -1.0 } else { 1.0 };
        let pz = if i >= 2 {
            ball(&mut s, v(lx, 33.0, lz), v(5.6, 6.8, 7.4), GREY);
            s.capsule((lx, 31.0, lz - 1.0), (lx + out * 0.4, 16.0, lz - 7.0), 4.8, 2.9, GREY);
            ball(&mut s, v(lx + out * 0.4, 16.0, lz - 7.0), Vec3::splat(2.9), GREY_D);
            s.capsule((lx + out * 0.4, 16.0, lz - 7.0), (lx + out * 0.2, 4.6, lz - 2.4), 2.7, 2.3, GREY_D);
            lz - 1.4
        } else {
            ball(&mut s, v(lx, 34.0, lz), v(5.4, 6.8, 6.4), GREY);
            s.capsule((lx, 32.0, lz), (lx, 17.0, lz + 1.6), 4.6, 3.1, GREY);
            s.capsule((lx, 17.0, lz + 1.6), (lx, 4.6, lz + 1.2), 3.0, 2.5, GREY_D);
            lz + 2.6
        };
        ball(&mut s, v(lx, 2.5, pz), v(3.4, 2.6, 4.0), GREY_D);
        for f in [-1.9f32, -0.65, 0.65, 1.9] {
            s.capsule((lx + f, 1.9, pz + 2.9), (lx + f * 1.15, 0.6, pz + 5.8), 0.8, 0.35, CLAW_D);
        }
    }

    // ---- the body: a deep chest, the belly paler, the great ruff at the neck
    s.part(0);
    s.capsule((cx, 35.0, 20.0), (cx, 40.0, 58.0), 8.8, 10.4, GREY);
    ball(&mut s, v(cx, 37.0, 57.0), v(10.8, 11.2, 9.8), GREY);
    ball(&mut s, v(cx, 30.6, 38.0), v(7.8, 5.8, 14.5), GREY_L);
    s.paint(|_, y, _, c| (c == GREY_L && y > 31).then_some(GREY));
    ball(&mut s, v(cx, 46.0, 61.0), v(11.8, 9.8, 7.8), GREY);
    s.paint(|x, y, z, c| (c == GREY && y > 44 && (x as f32 - cx).abs() < 4.0 && nz(x, y, z) < 0.6).then_some(GREY_D));
    // The mane: spikes standing off the ruff and sweeping back; hackles down the spine.
    for k in 0..120 {
        let u = nz(k, 7, 3) * TAU;
        let up = nz(k, 11, 5) * 0.9 + 0.1;
        let round = (1.0 - up * up).sqrt();
        let n = Vec3::new(u.cos() * round, up, u.sin() * round);
        let base = v(cx, 46.0, 61.0) + n * Vec3::new(11.0, 9.2, 7.2);
        spike(&mut s, base, (n + Vec3::new(0.0, 0.5, -1.1)).normalize(), 4.0 + nz(k, 13, 7) * 4.5, 1.4);
    }
    for k in 0..16 {
        let z = 56.0 - k as f32 * 2.3;
        let f = (z - 20.0) / 38.0;
        let top = 35.0 + 5.0 * f + 8.8 + 1.6 * f;
        for side in [-1.0f32, 0.0, 1.0] {
            let len = 3.0 + nz(k, side as i32 + 2, 9) * 2.5;
            spike(&mut s, v(cx + side * 2.0, top - 0.5, z), Vec3::new(side * 0.3, 1.0, -1.2).normalize(), len, 1.2);
        }
    }

    // ---- the tail, hanging bushy behind
    s.part(6);
    limb(&mut s, &[(v(cx, 38.0, 15.0), 3.1), (v(cx, 30.0, 7.0), 3.9), (v(cx, 18.0, 3.8), 3.5), (v(cx, 9.0, 4.6), 2.3)], GREY);
    ball(&mut s, v(cx, 8.0, 5.0), Vec3::splat(2.3), GREY_TIP);
    shag(&mut s, (w, h, d), GREY, GREY_L, GREY_TIP, 0.18, 2);

    // ---- the head: broad, the muzzle open in a snarl, yellow eyes, the ears up
    s.part(5);
    ball(&mut s, v(cx, 48.0, 69.0), v(7.8, 7.4, 7.2), GREY);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 5.8, 45.4, 67.6), v(3.4, 4.2, 4.2), GREY_L);
    }
    // The mouth between the jaws: dark, its sides gum-red.
    s.cuboid(17, 38, 72, 22, 41, 82, MOUTH);
    for z in 72..=82 {
        for y in 38..=41 {
            for x in [17, 22] {
                s.recolor(x, y, z, GUM);
            }
        }
    }
    s.capsule((cx, 46.4, 73.0), (cx, 45.2, 83.2), 4.7, 3.2, MUZZLE);
    ball(&mut s, v(cx, 46.0, 84.6), v(2.1, 1.7, 1.5), NOSE);
    s.capsule((cx, 38.4, 71.0), (cx, 35.6, 80.0), 3.0, 2.0, MUZZLE);
    for z in (73..=81).step_by(2) {
        let low = (41.4 - (z as f32 - 71.0) / 9.0 * 3.8 + 0.6).round() as i32;
        for x in [17, 22] {
            s.set(x, 41, z, TOOTH);
            s.set(x, low, z + 1, TOOTH);
        }
    }
    for x in [18, 21] {
        for y in 39..=41 {
            s.set(x, y, 81, TOOTH);
        }
        s.set(x, 38, 79, TOOTH);
        s.set(x, 39, 79, TOOTH);
    }
    for z in [75, 77, 79] {
        for x in 17..=22 {
            if let Some(y) = (44..=52).rev().find(|&y| s.get(x, y, z).is_some()) {
                s.recolor(x, y, z, GREY_D);
            }
        }
    }
    for side in [-1.0f32, 1.0] {
        let ex = cx + side * 4.3;
        ball(&mut s, v(ex, 50.2, 74.4), v(1.7, 1.2, 1.2), WOLF_EYE);
        s.set(ex.round() as i32, 50, 76, PUPIL);
        for k in -2..=2 {
            s.set((ex + k as f32).round() as i32, 52, 74, GREY_D);
        }
        s.capsule((cx + side * 4.8, 54.0, 66.0), (cx + side * 6.6, 60.6, 64.0), 2.6, 0.6, GREY);
        s.set((cx + side * 5.2).round() as i32, 56, 67, EAR_IN);
        s.set((cx + side * 5.6).round() as i32, 58, 66, EAR_IN);
    }
    s.finish(0.05)
}

// ------------------------------------------------------------------- jorogumo

/// The jorogumo's grid: 80 wide and 80 long (her legs), 84 tall.
pub const JORO_SIZE: (i32, i32, i32) = (80, 84, 80);
const JORO_X: f32 = 39.5;
/// Her legs sweep out from her front at these angles (degrees), front pair to back.
const JORO_ANGLES: [f32; 4] = [28.0, 62.0, 108.0, 148.0];
/// Her legs' parts: left (lower x) then right, front to back.
pub const JORO_LEGS: [&str; 8] = ["leg0", "leg1", "leg2", "leg3", "leg4", "leg5", "leg6", "leg7"];

/// Eight legs where they leave her carapace, her shoulders and her neck.
pub const JORO_RIG: &[PartDef] = &[
    PartDef { name: "leg0", boxes: &[], pivot: [35.98, 31.0, 51.5] },
    PartDef { name: "leg1", boxes: &[], pivot: [43.02, 31.0, 51.5] },
    PartDef { name: "leg2", boxes: &[], pivot: [32.88, 31.0, 47.99] },
    PartDef { name: "leg3", boxes: &[], pivot: [46.12, 31.0, 47.99] },
    PartDef { name: "leg4", boxes: &[], pivot: [32.37, 31.0, 41.37] },
    PartDef { name: "leg5", boxes: &[], pivot: [46.63, 31.0, 41.37] },
    PartDef { name: "leg6", boxes: &[], pivot: [35.53, 31.0, 36.79] },
    PartDef { name: "leg7", boxes: &[], pivot: [43.47, 31.0, 36.79] },
    PartDef { name: "leftArm", boxes: &[], pivot: [32.0, 58.6, 49.0] },
    PartDef { name: "rightArm", boxes: &[], pivot: [47.0, 58.6, 49.0] },
    PartDef { name: "head", boxes: &[], pivot: [39.5, 62.0, 49.0] },
];

/// Where leg `i` leaves her carapace, and the way it reaches (level, unit).
fn joro_leg(i: usize) -> (Vec3, Vec3) {
    let side = if i.is_multiple_of(2) { -1.0 } else { 1.0 };
    let a = JORO_ANGLES[i / 2].to_radians();
    (Vec3::new(JORO_X + side * a.sin() * 7.5, 31.0, 44.0 + a.cos() * 8.5), Vec3::new(side * a.sin(), 0.0, a.cos()))
}

/**
 * The jorogumo: a woman from the waist up, pale as ash, with dark hair drawn back into a long
 * tail, long pointed ears, dark eyes with a red light in them and dark lips, a top of dark
 * scales, little horns of bone on her shoulders, clawed hands and a sickle; from the waist down
 * a spider, her hips plated into its carapace, eight long legs banded at the joints and black
 * at the points, and behind them a great mottled bulb of an abdomen, chevroned and bristling.
 */
pub fn jorogumo() -> Collected {
    let (w, h, d) = JORO_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let cx = JORO_X;

    // ---- eight legs: up from the carapace to a high knee, down a long shin to a black point
    for i in 0..8 {
        s.part(1 + i as u8);
        let (root, dir) = joro_leg(i);
        let knee = root + dir * 12.0 + Vec3::Y * 14.0;
        let ankle = root + dir * 23.5 - Vec3::Y * 14.0;
        let tip = root + dir * 28.5 - Vec3::Y * 30.6;
        sweep(&mut s, &[(root, 2.0), (knee, 1.7), (ankle, 1.3), (tip, 0.45)], |p, c, along| {
            if along > 0.9 {
                LEG_TIP
            } else if (along - 0.28).abs() < 0.025 || (along - 0.74).abs() < 0.025 {
                JOINT
            } else if p.y < c.y - 0.6 {
                LEG_D
            } else {
                LEG
            }
        });
    }

    // ---- the abdomen behind, the waist, the carapace, her hips plated into it
    s.part(0);
    ball(&mut s, v(cx, 33.0, 19.5), v(14.0, 12.5, 15.5), BULB);
    s.paint(|x, y, z, c| {
        if c != BULB {
            return None;
        }
        let chevron = y > 36 && (z as f32 * 0.4 + (x as f32 - cx).abs() * 0.55).rem_euclid(4.2) < 0.9;
        if chevron || nz(x / 2, y / 2, z / 2) < 0.2 {
            Some(BULB_D)
        } else if nz(x, y, z) > 0.86 {
            Some(BULB_L)
        } else {
            None
        }
    });
    s.capsule((cx, 31.0, 31.0), (cx, 31.0, 37.0), 3.6, 3.6, SHELL_D);
    ball(&mut s, v(cx, 30.0, 44.0), v(9.6, 6.6, 10.4), SHELL);
    s.paint(|_x, y, z, c| (c == SHELL && (y < 26 || (z % 4 == 0 && y > 31))).then_some(if y < 26 { SHELL_D } else { SHELL_L }));
    for y in 32..=40 {
        let t = (y - 32) as f32 / 8.0;
        let c = if (y - 32) % 3 == 2 { SHELL_D } else { SHELL_L };
        column(&mut s, cx, 47.5 + t, 6.8 - t * 2.0, 5.8 - t * 1.8, y, y, c);
    }
    bristles(&mut s, (w, h, d), &[BULB, BULB_D, LEG, LEG_D], 0.05, BRISTLE);

    // ---- her waist, belly and chest; a top of dark scales; little horns on her shoulders
    for y in 40..=60 {
        let t = (y - 40) as f32 / 20.0;
        let rx = 4.7 + (t * PI).sin() * 1.5 + (t - 0.75).max(0.0) * 12.0;
        let rz = 3.5 + (t * PI).sin() * 0.8;
        column(&mut s, cx, 48.5 + t * 0.6, rx, rz, y, y, PALE);
    }
    s.paint(|x, y, z, c| (c == PALE && y < 61 && nz(x, y, z) < 0.15).then_some(PALE_D));
    face(&mut s, cx.ceil() as i32, 45, d, 0, PALE_D);
    for side in [-1.0f32, 1.0] {
        let x = cx + side * 2.4;
        let z = front(&s, x.round() as i32, 54, d).unwrap_or(52) as f32;
        ball(&mut s, v(x, 53.6, z - 0.6), v(2.4, 2.1, 1.6), TOP);
    }
    s.paint(|x, y, z, c| (c == TOP && (x + y + z) % 3 == 0).then_some(TOP_L));
    for side in [-1.0f32, 1.0] {
        s.capsule((cx + side * 6.2, 59.6, 48.6), (cx + side * 7.8, 63.8, 47.6), 0.9, 0.25, SPIKE);
        s.capsule((cx + side * 5.0, 60.0, 47.8), (cx + side * 5.6, 63.0, 46.4), 0.7, 0.2, SPIKE);
    }
    s.capsule((cx, 59.0, 49.0), (cx, 64.0, 49.4), 1.9, 1.7, PALE);

    // ---- arms: slender, reaching forward, clawed; a sickle in her left hand
    for (i, ax) in [32.0f32, 47.0].into_iter().enumerate() {
        s.part(9 + i as u8);
        let out = if i == 0 { -1.0 } else { 1.0 };
        ball(&mut s, v(ax, 58.6, 49.0), Vec3::splat(2.1), PALE);
        let (elbow, wrist) = (v(ax + out * 2.2, 50.4, 50.6), v(ax + out * 3.2, 48.0, 57.2));
        limb(&mut s, &[(v(ax, 58.4, 49.0), 1.75), (elbow, 1.35), (wrist, 1.1)], PALE);
        ball(&mut s, wrist + v(0.0, 0.2, 1.3), v(1.3, 1.1, 1.5), PALE);
        for f in [-1.0f32, 0.0, 1.0] {
            s.capsule(
                (wrist.x + f * 0.8, wrist.y + 0.2, wrist.z + 2.2),
                (wrist.x + f * 1.4, wrist.y + 1.2, wrist.z + 4.8),
                0.45,
                0.3,
                NAIL,
            );
        }
        if i == 0 {
            let top = wrist + v(0.0, 7.0, 1.6);
            s.capsule((wrist.x, wrist.y - 1.4, wrist.z + 1.2), (top.x, top.y, top.z), 0.8, 0.8, HANDLE);
            crescent(&mut s, [top, top + v(0.0, 3.6, 4.4), top + v(0.0, -1.6, 8.4)], 2.2, v(0.0, 1.0, 0.7), Vec3::X);
        }
    }

    // ---- the head: dark eyes with red in them, dark lips, pointed ears, the hair drawn back
    s.part(11);
    let hz = 49.6;
    ball(&mut s, v(cx, 68.6, hz), v(4.2, 5.0, 4.4), PALE);
    rounded(&mut s, v(cx, 65.2, hz + 1.2), v(2.7, 1.9, 2.8), 2.4, 0, PALE);
    for (x0, inward) in [(37, 1), (42, -1)] {
        face(&mut s, x0, 69, d, 0, PUPIL);
        face(&mut s, x0 + inward, 69, d, 0, J_EYE);
        face(&mut s, x0 - inward, 71, d, 0, J_HAIR);
        face(&mut s, x0, 71, d, 0, J_HAIR);
        face(&mut s, x0 + inward, 70, d, 0, J_HAIR);
    }
    for x in 38..=41 {
        face(&mut s, x, 65, d, 0, LIP);
    }
    face(&mut s, cx.ceil() as i32, 67, d, 1, PALE_D);
    for side in [-1.0f32, 1.0] {
        s.capsule((cx + side * 3.9, 68.8, hz), (cx + side * 8.6, 72.8, hz - 2.4), 1.1, 0.25, PALE);
    }
    for y in 64..=75 {
        for z in 0..d {
            for x in 0..w {
                let p = v(x as f32 - cx, y as f32 - 70.4, z as f32 - (hz - 0.9));
                if (p.x / 4.6).powi(2) + (p.y / 4.4).powi(2) + (p.z / 4.8).powi(2) > 1.0 || (z as f32 > hz + 1.8 && y < 73) {
                    continue;
                }
                s.set(x, y, z, J_HAIR);
            }
        }
    }
    limb(
        &mut s,
        &[
            (v(cx, 72.0, hz - 4.0), 1.8),
            (v(cx, 69.0, hz - 6.2), 1.6),
            (v(cx, 63.0, hz - 6.6), 1.4),
            (v(cx, 56.0, hz - 6.8), 1.0),
            (v(cx, 50.0, hz - 6.2), 0.6),
        ],
        J_HAIR,
    );
    s.paint(|x, y, z, c| (c == J_HAIR && nz(x, y, z) < 0.25).then_some(J_HAIR_L));
    s.finish(0.04)
}

// ---------------------------------------------------------------------- cobra

/// The cobra's grid: 60 wide, 84 tall, 60 deep.
pub const COBRA_SIZE: (i32, i32, i32) = (60, 84, 60);
const COBRA: (f32, f32) = (29.5, 29.5);

/// The neck where it rises off the coil, the hood and head on top of it, the tail's last coil.
pub const COBRA_RIG: &[PartDef] = &[
    PartDef { name: "neck", boxes: &[], pivot: [29.5, 9.0, 35.0] },
    PartDef { name: "head", boxes: &[], pivot: [29.5, 52.0, 34.0] },
    PartDef { name: "tail", boxes: &[], pivot: [15.5, 2.6, 21.5] },
];

/// A snake's skin about its axis: the pale belly plates on the side toward `belly`, a dark
/// seam beside them, and the olive back mottled darker and lighter.
fn snake_skin(belly: Vec3) -> impl Fn(Vec3, Vec3, f32) -> u32 {
    move |p, c, _| {
        let b = (p - c).normalize_or_zero().dot(belly);
        let (x, y, z) = (p.x as i32, p.y as i32, p.z as i32);
        if b > 0.5 {
            if y % 2 == 0 {
                BELLY_D
            } else {
                BELLY
            }
        } else if b > 0.3 || nz(x / 2, y / 2, z / 2) < 0.22 {
            SCALE_D
        } else if nz(x, y, z) > 0.84 {
            SCALE_L
        } else {
            SCALE
        }
    }
}

/// The z of a body's axis at height `y`, along knots that climb.
fn axis_z(knots: &[(Vec3, f32)], y: f32) -> f32 {
    for w in knots.windows(2) {
        let (a, b) = (w[0].0, w[1].0);
        if (a.y..=b.y).contains(&y) {
            return a.z + (b.z - a.z) * (y - a.y) / (b.y - a.y).max(1e-4);
        }
    }
    knots.last().map(|k| k.0.z).unwrap_or(0.0)
}

/**
 * The cobra: a great olive snake mottled dark along its back, coiled once round on the ground
 * with the tip of its tail curled up, the front of it reared up as tall as a man, pale belly
 * plates down its front; its hood spread wide, dark bands at the throat and the spectacle mark
 * behind; its mouth wide open, pink inside, on two fangs, yellow eyes and a forked tongue out.
 */
pub fn cobra() -> Collected {
    let (w, h, d) = COBRA_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cc) = COBRA;
    let ground = |x: f32, z: f32, r: f32| (v(x, r + 0.1, z), r);

    // ---- the coil on the ground, from the tail round and inward to where the neck rises
    let coil = [
        ground(cx - 14.0, cc - 8.0, 3.0),
        ground(cx - 16.4, cc + 1.0, 3.5),
        ground(cx - 12.4, cc + 10.6, 4.0),
        ground(cx - 2.4, cc + 15.0, 4.4),
        ground(cx + 9.0, cc + 12.0, 4.8),
        ground(cx + 15.6, cc + 2.0, 5.2),
        ground(cx + 12.4, cc - 9.4, 5.5),
        ground(cx + 2.0, cc - 13.6, 5.7),
        ground(cx - 7.4, cc - 8.2, 5.7),
        ground(cx - 7.6, cc + 1.0, 5.6),
        (v(cx - 2.6, 6.2, cc + 5.4), 5.3),
    ];
    s.part(0);
    sweep(&mut s, &coil, snake_skin(-Vec3::Y));
    s.part(3);
    let tail = [
        (v(cx - 14.0, 3.1, cc - 8.0), 3.0),
        (v(cx - 15.4, 3.0, cc - 13.0), 2.1),
        (v(cx - 13.2, 5.4, cc - 17.0), 1.4),
        (v(cx - 9.6, 7.8, cc - 17.6), 0.95),
        (v(cx - 7.4, 9.4, cc - 15.6), 0.6),
    ];
    sweep(&mut s, &tail, snake_skin(-Vec3::Y));

    // ---- the neck, reared up and leaning into the strike
    let neck = [
        (v(cx - 2.6, 6.2, cc + 5.4), 5.1),
        (v(cx - 0.6, 12.0, cc + 5.8), 5.0),
        (v(cx, 21.0, cc + 4.8), 4.8),
        (v(cx, 31.0, cc + 3.8), 4.6),
        (v(cx, 41.0, cc + 3.8), 4.4),
        (v(cx, 50.0, cc + 5.2), 4.1),
        (v(cx, 57.0, cc + 7.0), 3.9),
        (v(cx, 64.0, cc + 9.0), 3.7),
        (v(cx, 69.5, cc + 10.6), 3.5),
    ];
    s.part(1);
    sweep(&mut s, &neck[..6], snake_skin(Vec3::Z));
    s.part(2);
    sweep(&mut s, &neck[5..], snake_skin(Vec3::Z));

    // ---- the hood: spread wide either side, pale in front with dark bands at the throat,
    //      olive behind with the spectacle mark
    for y in 49..=70 {
        let k = (y - 49) as f32 / 21.0;
        let hw = 4.2 + 6.4 * (k * PI).sin().max(0.0).powf(0.7);
        let zc = axis_z(&neck, y as f32) - 0.6;
        let hd = 2.5;
        for z in (zc - hd).floor() as i32..=(zc + hd).ceil() as i32 {
            for x in (cx - hw).floor() as i32..=(cx + hw).ceil() as i32 {
                if ((x as f32 - cx) / hw).powi(2) + ((z as f32 - zc) / hd).powi(2) > 1.0 {
                    continue;
                }
                let dx = (x as f32 - cx).abs();
                let c = if z as f32 > zc {
                    if (51..=54).contains(&y) && dx < hw - 1.0 {
                        MARK
                    } else if y % 2 == 0 {
                        BELLY_D
                    } else {
                        BELLY
                    }
                } else {
                    let ring = ((dx - 3.6).powi(2) + (y as f32 - 61.0).powi(2)).sqrt();
                    if (1.4..2.4).contains(&ring) {
                        MARK
                    } else if nz(x / 2, y / 2, z) < 0.22 {
                        SCALE_D
                    } else {
                        SCALE
                    }
                };
                s.set(x, y, z, c);
            }
        }
    }

    // ---- the head: the mouth wide open on its fangs, yellow eyes, the tongue forked out
    s.ellipsoid((cx, 70.6, cc + 15.0), (3.8, 2.4, 4.8), 0, PINK);
    s.ellipsoid((cx, 74.6, cc + 13.6), (5.6, 3.0, 7.0), 0, SCALE);
    s.capsule((cx, 70.0, cc + 9.6), (cx, 66.0, cc + 18.8), 3.4, 2.0, BELLY);
    s.paint(|x, y, z, c| (c == SCALE && y >= 74 && z as f32 > cc + 8.0 && nz(x / 2, y, z / 2) < 0.25).then_some(SCALE_D));
    for side in [-1.0f32, 1.0] {
        s.capsule((cx + side * 2.1, 72.2, cc + 19.6), (cx + side * 2.1, 69.2, cc + 19.8), 0.6, 0.3, TOOTH);
        ball(&mut s, v(cx + side * 4.8, 75.6, cc + 15.6), Vec3::splat(1.2), SNAKE_EYE);
        s.set((cx + side * 6.0).round() as i32, 76, (cc + 15.6).round() as i32, PUPIL);
    }
    s.capsule((cx, 69.6, cc + 17.0), (cx, 68.4, cc + 23.0), 0.6, 0.5, TONGUE);
    for side in [-1.0f32, 1.0] {
        s.capsule((cx, 68.4, cc + 23.0), (cx + side * 1.1, 68.0, cc + 25.0), 0.45, 0.3, TONGUE);
    }
    s.finish(0.04)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spider_womans_legs_hinge_where_they_leave_her_carapace() {
        for (i, def) in JORO_RIG.iter().take(8).enumerate() {
            let (root, _) = joro_leg(i);
            assert_eq!(def.name, JORO_LEGS[i]);
            assert!((Vec3::from(def.pivot) - root).length() < 0.05, "{}: {:?} vs {root:?}", def.name, def.pivot);
        }
    }

    #[test]
    fn every_third_stage_foe_stands_in_the_middle_of_its_grid_and_inside_it() {
        for (name, c, size) in [
            ("tengu", tengu(), TENGU_SIZE),
            ("oni", oni(), ONI_SIZE),
            ("dire wolf", dire_wolf(), DIRE_SIZE),
            ("jorogumo", jorogumo(), JORO_SIZE),
            ("cobra", cobra(), COBRA_SIZE),
        ] {
            assert!(c.voxels.len() > 2000, "{name}: {} voxels", c.voxels.len());
            assert_eq!((c.width, c.height, c.depth), size, "{name}");
            // Its feet on the floor, and its footprint about the grid's middle (where the rig puts its place).
            assert!(c.voxels.iter().map(|v| v.y).min().is_some_and(|y| y <= 1), "{name} stands on the floor");
            assert!(!c.voxels.iter().any(|v| v.x == 0), "{name}: a stray voxel on the grid's edge (a NaN)");
            let floor: Vec<_> = c.voxels.iter().filter(|v| v.y <= 2).collect();
            let mx = floor.iter().map(|v| v.x as f32).sum::<f32>() / floor.len() as f32;
            assert!((mx - (size.0 - 1) as f32 / 2.0).abs() < 3.0, "{name}: footprint centred at x {mx}");
        }
    }
}
