//! The fourth stage's cast, sculpted as finely as the third (see `foes3`): four yokai of the old
//! tales. The kappa, a warty green river imp with a turtle's shell on its back and a dish of water
//! on its crown, crouched to charge like a sumo wrestler; the rokurokubi, a woman kneeling small
//! in a dark kimono, whose neck rises in long pale curves to her white face and wild black hair;
//! the oni, the ogre of the old tales and not the demon of the stage before, huge and red, horned
//! and bearded, in a tiger-skin loincloth, his iron club in his fist; and the kitsune, a
//! nogitsune, a red fox with narrow burning eyes and one great white-tipped tail, foxfire floating
//! over it. Each faces +z, stands in the middle of its grid (where its rig sets it down), and is
//! sculpted in the parts its rig swings.
//!
//! Between them they fight all three ways (COMBAT.md §10): the oni with his club, which shakes the
//! ground where it lands; the kappa and the rokurokubi with their bodies, the kappa throwing all
//! of itself at you in a sumo charge (bow to it and it bows back, spills the water from its dish
//! and grows weak), the rokurokubi lashing her neck out across the room; the kitsune from afar,
//! casting foxfire and hiding among its illusions.

use std::f32::consts::{PI, TAU};

use glam::Vec3;

use super::carve::*;
use super::characters::{mat, Sculpt};
use super::voxels::{Collected, PartDef};

mod pal {
    use super::mat;
    pub const CAVITY: u32 = 0x170f0e | mat::SKIN;
    // Eyes: polished, so they catch the light.
    pub const SCLERA: u32 = 0xf2eee6 | mat::IVORY;
    pub const GLEAM: u32 = 0xffffff | mat::IVORY;
    pub const DARK_PUPIL: u32 = 0x120e0c | mat::IVORY;
    // The kappa.
    pub const K_SKIN: u32 = 0x4c8c66 | mat::SKIN;
    pub const K_SKIN_D: u32 = 0x346a4e | mat::SKIN;
    pub const K_SKIN_L: u32 = 0x6aa87c | mat::SKIN;
    pub const K_SPOT: u32 = 0x2a5640 | mat::SKIN;
    pub const K_WART: u32 = 0x86ba86 | mat::SKIN;
    pub const K_BELLY: u32 = 0x7eae74 | mat::SKIN;
    pub const K_WEB: u32 = 0xa8c48c | mat::SKIN;
    pub const K_LID: u32 = 0x2c5a42 | mat::SKIN;
    pub const PLASTRON: u32 = 0xe2d286 | mat::IVORY;
    pub const PLASTRON_L: u32 = 0xf0e4a6 | mat::IVORY;
    pub const PLASTRON_D: u32 = 0x9c8644 | mat::IVORY;
    pub const SHELL: u32 = 0x6e6434 | mat::LEATHER;
    pub const SHELL_L: u32 = 0x8c7e46 | mat::LEATHER;
    pub const SHELL_D: u32 = 0x2c2814 | mat::LEATHER;
    pub const SHELL_RIM: u32 = 0x5a5028 | mat::LEATHER;
    pub const BEAK: u32 = 0xc2b252 | mat::IVORY;
    pub const BEAK_L: u32 = 0xdcce74 | mat::IVORY;
    pub const BEAK_D: u32 = 0x8a7c34 | mat::IVORY;
    pub const K_MOUTH: u32 = 0x3a1416 | mat::SKIN;
    pub const K_TONGUE: u32 = 0xc2565e | mat::SKIN;
    pub const K_IRIS: u32 = 0xf2d24a | mat::IVORY;
    pub const K_CLAW: u32 = 0xd8cfa8 | mat::IVORY;
    pub const DISH: u32 = 0xd8dabe | mat::IVORY;
    pub const DISH_D: u32 = 0xa4a88e | mat::IVORY;
    pub const DISH_IN: u32 = 0x8e947c | mat::IVORY;
    pub const WATER: u32 = 0x7ec8ec | mat::SLIME;
    pub const WATER_L: u32 = 0xc2ecff | mat::SLIME;
    pub const WATER_D: u32 = 0x4e9cd0 | mat::SLIME;
    pub const K_HAIR: u32 = 0x23302a | mat::FUR;
    pub const K_HAIR_L: u32 = 0x3f5246 | mat::FUR;
    // The rokurokubi.
    pub const R_KIMONO: u32 = 0x1e2236 | mat::CLOTH;
    pub const R_KIMONO_D: u32 = 0x131626 | mat::CLOTH;
    pub const R_KIMONO_L: u32 = 0x2c3250 | mat::CLOTH;
    pub const R_PETAL: u32 = 0x4e5880 | mat::CLOTH;
    pub const R_PETAL_C: u32 = 0x8a90b4 | mat::CLOTH;
    pub const R_LINING: u32 = 0x9a1e2a | mat::CLOTH;
    pub const R_COLLAR: u32 = 0xeae6de | mat::CLOTH;
    pub const R_UNDER: u32 = 0xd8d2ca | mat::SKIN;
    pub const R_OBI: u32 = 0x8a1e28 | mat::CLOTH;
    pub const R_OBI_D: u32 = 0x5c1219 | mat::CLOTH;
    pub const R_OBI_GOLD: u32 = 0xd0a84c | mat::GOLD;
    pub const R_CORD: u32 = 0xd8c27a | mat::CLOTH;
    pub const R_TABI: u32 = 0xeeeae2 | mat::CLOTH;
    pub const R_SKIN: u32 = 0xf0ebe4 | mat::SKIN;
    pub const R_NAIL: u32 = 0x9a1a26 | mat::IVORY;
    pub const NECK: u32 = 0xc6b8aa | mat::SKIN;
    pub const NECK_L: u32 = 0xdacfc4 | mat::SKIN;
    pub const NECK_D: u32 = 0x9c8c7e | mat::SKIN;
    pub const R_FACE: u32 = 0xf6f2ec | mat::SKIN;
    pub const R_FACE_D: u32 = 0xd2c9c0 | mat::SKIN;
    pub const R_EYE: u32 = 0x1a1216 | mat::IVORY;
    pub const R_GLINT: u32 = 0xff3a2e | mat::GLOW;
    pub const R_LASH: u32 = 0x121014 | mat::FUR;
    pub const R_LIP: u32 = 0xc41e30 | mat::SKIN;
    pub const R_LIP_D: u32 = 0x961624 | mat::SKIN;
    pub const R_TEETH: u32 = 0xf2eee4 | mat::IVORY;
    pub const R_HAIR: u32 = 0x15151b | mat::FUR;
    pub const R_HAIR_L: u32 = 0x2c2e3e | mat::FUR;
    pub const R_COMB: u32 = 0x6a3418 | mat::IVORY;
    pub const R_BEAD: u32 = 0xb81c2a | mat::IVORY;
    // The oni.
    pub const O_SKIN: u32 = 0xc03a2e | mat::SKIN;
    pub const O_SKIN_D: u32 = 0x8a2620 | mat::SKIN;
    pub const O_SKIN_L: u32 = 0xd04a3a | mat::SKIN;
    pub const O_SKIN_M: u32 = 0xa8322a | mat::SKIN;
    pub const O_SOCKET: u32 = 0x5e1a16 | mat::SKIN;
    pub const O_LIP: u32 = 0x5a1a1e | mat::SKIN;
    pub const O_EYE: u32 = 0xf2c030 | mat::IVORY;
    pub const O_EYE_L: u32 = 0xfff0a0 | mat::IVORY;
    pub const O_CLAW: u32 = 0x221c18 | mat::IVORY;
    pub const TOOTH: u32 = 0xf0e8d4 | mat::IVORY;
    pub const HORN: u32 = 0xe2d4ac | mat::IVORY;
    pub const HORN_D: u32 = 0xb4a47c | mat::IVORY;
    pub const HORN_TIP: u32 = 0x4e4232 | mat::IVORY;
    pub const O_HAIR: u32 = 0x141214 | mat::FUR;
    pub const O_HAIR_L: u32 = 0x2e2a2e | mat::FUR;
    pub const O_GOLD: u32 = 0xe8b840 | mat::GOLD;
    pub const O_STRAP: u32 = 0x5a3a22 | mat::LEATHER;
    pub const O_STRAP_D: u32 = 0x3c2616 | mat::LEATHER;
    pub const ROPE: u32 = 0xd2b47a | mat::CLOTH;
    pub const ROPE_D: u32 = 0x9c8050 | mat::CLOTH;
    pub const TIGER: u32 = 0xe08a2c | mat::FUR;
    pub const TIGER_L: u32 = 0xf2a448 | mat::FUR;
    pub const TIGER_STRIPE: u32 = 0x1c1612 | mat::FUR;
    pub const TIGER_PALE: u32 = 0xf2e8d6 | mat::FUR;
    pub const IRON: u32 = 0x8a8e96 | mat::METAL;
    pub const IRON_L: u32 = 0xaeb2ba | mat::METAL;
    pub const IRON_D: u32 = 0x5e626a | mat::METAL;
    pub const IRON_BAND: u32 = 0x6a6e76 | mat::METAL;
    pub const STUD: u32 = 0xd4d8e0 | mat::METAL;
    pub const O_GRIP: u32 = 0x4a2e1c | mat::LEATHER;
    pub const O_GRIP_L: u32 = 0x6e4a30 | mat::LEATHER;
    // The kitsune.
    pub const F_FUR: u32 = 0xd8642a | mat::FUR;
    pub const F_FUR_D: u32 = 0xa4441c | mat::FUR;
    pub const F_FUR_L: u32 = 0xee8c46 | mat::FUR;
    pub const F_CREAM: u32 = 0xf4ece0 | mat::FUR;
    pub const F_CREAM_D: u32 = 0xd8cdbc | mat::FUR;
    pub const F_CHEEK: u32 = 0xf2eadc | mat::FUR;
    pub const F_BLACK: u32 = 0x1e1816 | mat::FUR;
    pub const F_EAR_IN: u32 = 0xe8d6c4 | mat::FUR;
    pub const F_CLAW: u32 = 0x141010 | mat::IVORY;
    pub const F_NOSE: u32 = 0x0e0c0c | mat::LEATHER;
    pub const F_LINE: u32 = 0x161010 | mat::FUR;
    pub const F_TOOTH: u32 = 0xf4f0e6 | mat::IVORY;
    pub const F_EYE: u32 = 0xffb424 | mat::GLOW;
    pub const FIRE_C: u32 = 0xf0ffff | mat::GLOW;
    pub const FIRE: u32 = 0xa6eeff | mat::GLOW;
    pub const FIRE_D: u32 = 0x6ccff0 | mat::GLOW;
}
use pal::*;

/// Sets the voxel nearest `p`.
fn put(s: &mut Sculpt, p: Vec3, c: u32) {
    s.set(p.x.round() as i32, p.y.round() as i32, p.z.round() as i32, c);
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

/// A smooth curve through `points` (a Catmull-Rom spline), `per` samples to a span.
fn spline(points: &[Vec3], per: usize) -> Vec<Vec3> {
    let n = points.len();
    let mut out = Vec::with_capacity(n * per);
    for i in 0..n - 1 {
        let (p0, p1, p2, p3) = (points[i.saturating_sub(1)], points[i], points[i + 1], points[(i + 2).min(n - 1)]);
        for k in 0..per {
            let t = k as f32 / per as f32;
            let (t2, t3) = (t * t, t * t * t);
            out.push(0.5 * (2.0 * p1 + (p2 - p0) * t + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2 + (3.0 * p1 - p0 - 3.0 * p2 + p3) * t3));
        }
    }
    out.push(points[n - 1]);
    out
}

/// Stiff hairs (or warts) standing out of the `on` voxels, about `density` of them.
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

/**
 * Shades by the shape, as light pools in a sculpture's hollows: every open voxel of the `on`
 * colours is recoloured `f(colour, depth)`, where depth is how much more of the ball of radius
 * `r` round it is filled than on a flat face (weighted toward its middle, so the steps of a
 * smooth swell do not band): above 0 in a crease or a cleft, below 0 on a ridge or a swell.
 */
fn hollows(s: &mut Sculpt, dims: (i32, i32, i32), r: i32, on: &[u32], f: impl Fn(u32, f32) -> u32) {
    let (w, h, d) = dims;
    let spread = 2.0 * (0.6 * r as f32).powi(2);
    let mut offsets = vec![];
    for dz in -r..=r {
        for dy in -r..=r {
            for dx in -r..=r {
                let q = dx * dx + dy * dy + dz * dz;
                if q <= r * r {
                    offsets.push((dx, dy, dz, (-(q as f32) / spread).exp()));
                }
            }
        }
    }
    let total: f32 = offsets.iter().map(|o| o.3).sum();
    let flat = offsets.iter().filter(|o| o.1 <= 0).map(|o| o.3).sum::<f32>() / total;
    let mut out = vec![];
    for y in 0..h {
        for z in 0..d {
            for x in 0..w {
                let Some(c) = s.get(x, y, z) else { continue };
                if !on.contains(&c) || !open(s, x, y, z) {
                    continue;
                }
                let filled: f32 = offsets.iter().filter(|o| s.get(x + o.0, y + o.1, z + o.2).is_some()).map(|o| o.3).sum();
                // (Dithered a little, so a gradual turn shades in a stipple rather than a line.)
                let depth = filled / total - flat + (nz(x * 7, y * 7, z * 7) - 0.5) * 0.04;
                out.push((x, y, z, f(c, depth)));
            }
        }
    }
    for (x, y, z, c) in out {
        s.recolor(x, y, z, c);
    }
}

/// A web of skin between two fingers or toes, from `root` out to `a` and `b`, its free edge
/// scalloped in toward the root.
fn web(s: &mut Sculpt, root: Vec3, a: Vec3, b: Vec3, c: u32) {
    let n = 28;
    for i in 0..=n {
        for j in 0..=(n - i) {
            let (u, w) = (i as f32 / n as f32, j as f32 / n as f32);
            let sum = u + w;
            if sum > 1e-4 {
                let k = u / sum;
                if sum > 1.0 - 1.4 * k * (1.0 - k) {
                    continue;
                }
            }
            put(s, root + (a - root) * u + (b - root) * w, c);
        }
    }
}

// ---------------------------------------------------------------------- kappa

/// The kappa's grid: 55 wide, 80 tall, 56 deep.
pub const KAPPA_SIZE: (i32, i32, i32) = (55, 80, 56);
const KAPPA_X: f32 = 27.0;
/// Its hips' depth: a little behind the grid's middle, so that its feet, which reach forward,
/// stand about the middle.
const KAPPA_Z: f32 = 25.0;
/// The centre of the water's surface in the dish on its head (the "water" part's pivot).
pub const KAPPA_DISH: [f32; 3] = [KAPPA_X, 74.0, KAPPA_Z + 5.5];

/// Hips, shoulders and neck, and the water in its dish.
pub const KAPPA_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [KAPPA_X - 8.5, 29.0, KAPPA_Z - 1.5] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [KAPPA_X + 8.5, 29.0, KAPPA_Z - 1.5] },
    PartDef { name: "leftArm", boxes: &[], pivot: [KAPPA_X - 12.5, 46.5, KAPPA_Z + 2.0] },
    PartDef { name: "rightArm", boxes: &[], pivot: [KAPPA_X + 12.5, 46.5, KAPPA_Z + 2.0] },
    PartDef { name: "head", boxes: &[], pivot: [KAPPA_X, 47.0, KAPPA_Z + 2.0] },
    PartDef { name: "water", boxes: &[], pivot: KAPPA_DISH },
];

/// One of the kappa's big round eyes, bulging from its face about `c`: a yellow iris round a
/// black pupil with a glint, a heavy lid over its top, lowest toward the nose (`inward`, +1 or
/// -1, is the way to it), so it glares.
fn goggle(s: &mut Sculpt, c: Vec3, r: f32, inward: f32, d: i32) {
    ball(s, c, v(r, r, r * 0.8), K_LID);
    for y in (c.y - r).floor() as i32..=(c.y + r).ceil() as i32 {
        for x in (c.x - r).floor() as i32..=(c.x + r).ceil() as i32 {
            let (dx, dy) = (x as f32 - c.x, y as f32 - c.y);
            let q = (dx * dx + dy * dy).sqrt();
            if q > r - 0.5 {
                continue;
            }
            let col = if dy > r * 0.5 - dx * inward * 0.5 {
                K_LID
            } else if q < r * 0.34 {
                DARK_PUPIL
            } else {
                K_IRIS
            };
            face(s, x, y, d, 0, col);
        }
    }
    face(s, (c.x - r * 0.3).round() as i32, (c.y + r * 0.2).round() as i32, d, 0, GLEAM);
}

/**
 * The kappa: a river imp the size of a child, green as pond weed and warty, crouched to charge
 * like a sumo wrestler; a round head with big round yellow eyes under heavy brows and a hooked
 * yellow beak, and on its crown the dish of water that is its strength, ringed with lank black
 * hair; a turtle's shell on its back, olive scutes seamed in black, a pale plastron down its pot
 * belly; webbed hands thrust forward, short stout legs bent out over flat webbed feet, every
 * finger and toe clawed.
 */
pub fn kappa() -> Collected {
    let (w, h, d) = KAPPA_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (KAPPA_X, KAPPA_Z);

    // ---- legs: thick thighs bent out over the knees, stout shins, flat webbed feet turned out
    for (i, out) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        let hip = v(cx + out * 8.5, 29.0, cz - 1.5);
        let knee = v(cx + out * 17.0, 20.0, cz + 3.5);
        let ankle = v(cx + out * 16.0, 5.0, cz - 0.5);
        limb(&mut s, &[(hip, 7.0), (knee, 5.5)], K_SKIN);
        ball(&mut s, knee, Vec3::splat(5.4), K_SKIN);
        limb(&mut s, &[(knee, 4.8), (ankle, 3.6)], K_SKIN);
        ball(&mut s, v(knee.x + out * 0.3, 13.0, cz - 0.5), v(4.2, 5.0, 4.0), K_SKIN);
        ball(&mut s, v(ankle.x, 2.4, cz - 1.0), v(4.2, 2.5, 4.4), K_SKIN);
        let root = v(ankle.x, 1.5, cz + 1.5);
        let mut tips = vec![];
        for (k, a) in [-24.0f32, -2.0, 20.0, 42.0].into_iter().enumerate() {
            let ang = (a * out).to_radians();
            let dir = v(ang.sin(), 0.0, ang.cos());
            let len = [7.0, 8.6, 8.6, 7.0][k];
            let tip = root + dir * len - Vec3::Y * 0.4;
            s.capsule((root.x, root.y, root.z), (tip.x, tip.y, tip.z), 1.7, 1.05, K_SKIN);
            let point = tip + dir * 1.9 - Vec3::Y * 0.9;
            s.capsule((tip.x, tip.y, tip.z), (point.x, point.y, point.z), 0.8, 0.3, K_CLAW);
            tips.push(root + dir * (len * 0.86) - Vec3::Y * 0.5);
        }
        for pair in tips.windows(2) {
            web(&mut s, root - Vec3::Y * 0.4, pair[0], pair[1], K_WEB);
        }
    }

    // ---- the body: a pot belly, a broad chest, a seat under the shell
    s.part(0);
    ball(&mut s, v(cx, 32.5, cz + 1.0), v(13.8, 12.5, 12.0), K_SKIN);
    ball(&mut s, v(cx, 43.5, cz + 2.5), v(13.0, 8.8, 9.8), K_SKIN);
    ball(&mut s, v(cx, 27.5, cz - 2.5), v(12.0, 7.0, 9.6), K_SKIN);

    // The shell on its back: a dome of olive scutes seamed in black, its rim of marginal scutes.
    let (sc, sr) = (v(cx, 37.0, cz - 2.5), v(15.8, 16.6, 13.0));
    let cut = cz - 3.0;
    let centres: Vec<Vec3> = [
        (0.0f32, 0.0f32),
        (28.0, 90.0),
        (52.0, 90.0),
        (28.0, -90.0),
        (52.0, -90.0),
        (44.0, 52.0),
        (44.0, 14.0),
        (44.0, -24.0),
        (44.0, -60.0),
        (44.0, 128.0),
        (44.0, 166.0),
        (44.0, 204.0),
        (44.0, 240.0),
    ]
    .iter()
    .map(|&(th, ph): &(f32, f32)| {
        let (th, ph) = (th.to_radians(), ph.to_radians());
        sc + sr * v(th.sin() * ph.cos(), th.sin() * ph.sin(), -th.cos())
    })
    .collect();
    for y in 0..h {
        for z in 0..d {
            for x in 0..w {
                let p = v(x as f32, y as f32, z as f32);
                let q = (p - sc) / sr;
                if q.length() > 1.0 || p.z > cut || ((p - sc) / (sr - Vec3::splat(2.6))).length() < 1.0 {
                    continue;
                }
                // Its angle from the middle of the back, and round it.
                let qn = q.normalize_or_zero();
                let theta = (-qn.z).clamp(-1.0, 1.0).acos().to_degrees();
                let phi = qn.y.atan2(qn.x).to_degrees();
                let c = if theta > 66.0 {
                    let seam = (theta - 66.0).abs() < 3.0 || ((phi + 7.5) / 15.0).rem_euclid(1.0) < 0.12 || theta > 81.0;
                    if seam {
                        SHELL_D
                    } else {
                        SHELL_RIM
                    }
                } else {
                    // The nearest two scutes' middles: a seam where they are about as near, rings of
                    // growth round each.
                    let mut ds: Vec<f32> = centres.iter().map(|c| (*c - p).length()).collect();
                    ds.sort_by(|a, b| a.total_cmp(b));
                    if ds[1] - ds[0] < 1.3 {
                        SHELL_D
                    } else if (ds[0] * 0.8) as i32 % 2 == 0 && nz(x, y, z) > 0.3 {
                        SHELL_L
                    } else {
                        SHELL
                    }
                };
                s.set(x, y, z, c);
            }
        }
    }

    // The plastron down its belly: pale plates, seamed across and down the middle.
    for y in 20..=50 {
        for x in 0..w {
            let (dx, dy) = ((x as f32 - cx) / 9.8, (y as f32 - 35.0) / 13.6);
            if dx * dx + dy * dy > 1.0 {
                continue;
            }
            let Some(z) = front(&s, x, y, d) else { continue };
            let edge = dx * dx + dy * dy > 0.8;
            let seam = x as f32 == cx || [26, 31, 36, 42].contains(&y);
            let c = if edge || seam {
                PLASTRON_D
            } else if nz(x, y, z) > 0.8 {
                PLASTRON_L
            } else {
                PLASTRON
            };
            s.set(x, y, z + 1, c);
        }
    }

    // ---- arms: short, thrust forward and out, the palms open, webbed and clawed
    for (i, out) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(3 + i as u8);
        let sh = v(cx + out * 12.5, 46.5, cz + 2.0);
        let el = v(cx + out * 18.0, 39.5, cz + 9.0);
        let wr = v(cx + out * 16.5, 40.5, cz + 17.0);
        ball(&mut s, sh, v(5.2, 5.0, 5.2), K_SKIN);
        limb(&mut s, &[(sh, 4.4), (el, 3.5)], K_SKIN);
        ball(&mut s, el, Vec3::splat(3.4), K_SKIN);
        limb(&mut s, &[(el, 3.4), (wr, 2.6)], K_SKIN);
        let palm = wr + v(0.0, 2.0, 1.3);
        ball(&mut s, palm, v(3.4, 3.6, 1.8), K_SKIN);
        let mut tips = vec![];
        for (k, a) in [-70.0f32, -16.0, 10.0, 36.0].into_iter().enumerate() {
            let ang = a.to_radians();
            let dir = v(out * ang.sin(), ang.cos(), 0.3).normalize();
            let base = palm + v(out * ang.sin() * 2.0, ang.cos() * 2.6, 0.4);
            let len = if k == 0 { 3.8 } else { 5.4 };
            let knuckle = base + dir * len;
            let tip = knuckle + (dir + v(0.0, -0.2, 0.9)).normalize() * 2.4;
            limb(&mut s, &[(base, 1.25), (knuckle, 1.0), (tip, 0.8)], K_SKIN);
            let point = tip + v(0.0, -0.5, 1.6);
            s.capsule((tip.x, tip.y, tip.z), (point.x, point.y, point.z), 0.7, 0.25, K_CLAW);
            tips.push(base + dir * (len * 0.85));
        }
        for pair in tips.windows(2) {
            web(&mut s, palm + v(0.0, 1.2, 0.4), pair[0], pair[1], K_WEB);
        }
    }

    // ---- the head: big and round, bulging eyes under heavy brows, the hooked open beak
    s.part(5);
    let hz = cz + 6.0;
    let hc = v(cx, 63.0, hz);
    limb(&mut s, &[(v(cx, 45.0, cz + 2.0), 6.8), (v(cx, 54.0, hz - 1.0), 6.4)], K_SKIN);
    ball(&mut s, hc, v(11.6, 11.5, 11.0), K_SKIN);
    ball(&mut s, v(cx, 57.5, hz + 3.0), v(10.6, 6.6, 9.0), K_SKIN);
    // Eyes, bulging; the brows over them, low and angry at the middle.
    for side in [-1.0f32, 1.0] {
        let ex = cx + side * 5.4;
        let fz = front(&s, ex.round() as i32, 64, d).unwrap_or(40) as f32;
        goggle(&mut s, v(ex, 64.5, fz - 1.6), 3.8, -side, d);
        let (a, b) = (v(cx + side * 1.8, 68.6, fz + 0.6), v(cx + side * 9.6, 70.6, fz - 3.0));
        s.capsule((a.x, a.y, a.z), (b.x, b.y, b.z), 1.5, 1.1, K_SKIN_D);
    }
    // The beak: a snapping turtle's, broad at the corners of the mouth and hooked at its point,
    // open on a pink tongue, the mouth running back along the cheeks.
    let (z0, z1) = (hz + 5.0, hz + 17.0);
    for z in z0 as i32..=z1 as i32 {
        let t = ((z as f32 - z0) / (z1 - z0)).clamp(0.0, 1.0);
        // The upper bill: domed, narrowing to the hook, which drops over the lower bill.
        let hw = 7.8 * (1.0 - 0.72 * t.powf(1.5));
        let top = 61.6 - 2.8 * t.powf(1.2);
        let bot = 56.4 - if t > 0.72 { (t - 0.72) / 0.28 * 2.8 } else { 0.0 };
        for y in bot.floor() as i32..=top.ceil() as i32 {
            let k = ((y as f32 - bot) / (top - bot).max(0.5)).clamp(0.0, 1.0);
            let half = hw * (1.0 - k * k * 0.5);
            for x in (cx - half).floor() as i32..=(cx + half).ceil() as i32 {
                if (x as f32 - cx).abs() <= half {
                    let c = if y as f32 >= top - 0.5 && t < 0.85 {
                        BEAK_L
                    } else if y as f32 <= bot + 0.5 {
                        BEAK_D
                    } else {
                        BEAK
                    };
                    s.set(x, y, z, c);
                }
            }
        }
        // The lower bill, its chin rising to the point, and the mouth between the two.
        if t < 0.86 {
            let lw = 6.8 * (1.0 - 0.7 * t.powf(1.5));
            let (ltop, lbot) = (52.6, 49.4 + 2.2 * t);
            for y in lbot.floor() as i32..=ltop as i32 {
                for x in (cx - lw).floor() as i32..=(cx + lw).ceil() as i32 {
                    if (x as f32 - cx).abs() > lw {
                        continue;
                    }
                    let c = if y == ltop as i32 {
                        BEAK_L
                    } else if y as f32 <= lbot + 0.5 {
                        BEAK_D
                    } else {
                        BEAK
                    };
                    s.set(x, y, z, c);
                }
            }
            if t < 0.7 {
                let mw = lw - 0.8;
                for y in 53..=55 {
                    for x in (cx - mw).floor() as i32..=(cx + mw).ceil() as i32 {
                        if (x as f32 - cx).abs() <= mw {
                            let tongue = y == 53 && (x as f32 - cx).abs() < 3.0 && t > 0.15;
                            s.set(x, y, z, if tongue { K_TONGUE } else { K_MOUTH });
                        }
                    }
                }
            }
        }
    }
    // Nostrils on the bill, and little teeth of horn along the upper bill's edge.
    for x in [cx - 1.5, cx + 1.5] {
        let (x, z) = (x.round() as i32, (z0 + 8.0) as i32);
        if let Some(y) = (57..=64).rev().find(|&y| s.get(x, y, z).is_some()) {
            s.recolor(x, y, z, CAVITY);
        }
    }
    for side in [-1.0f32, 1.0] {
        for k in 0..3 {
            let t = 0.2 + k as f32 * 0.17;
            let z = z0 + t * (z1 - z0);
            let x = cx + side * (7.8 * (1.0 - 0.72 * t.powf(1.5)) - 0.8);
            put(&mut s, v(x, 55.6, z), K_CLAW);
        }
    }
    s.paint(|x, y, z, c| {
        let dx = (x as f32 - cx).abs();
        (c == K_SKIN && y == 55 && (6.5..11.5).contains(&dx) && (z as f32) > hz + 1.0).then_some(K_MOUTH)
    });

    // ---- the dish on its crown: a pale rim round a shallow bowl, ringed with lank black hair
    let [dx, dy, dz] = KAPPA_DISH;
    // The hair first: a bowl cut round the dish, clear of its face, lank and ragged.
    for y in 54..=75 {
        for z in 0..d {
            for x in 0..w {
                let p = v(x as f32, y as f32, z as f32);
                let r = ((p.x - dx).powi(2) + (p.z - dz).powi(2)).sqrt();
                let (o, i) = (((p - hc) / v(12.8, 12.7, 12.2)).length(), ((p - hc) / v(11.0, 10.9, 10.4)).length());
                let collar = r < 7.8 && p.y <= dy - 1.0 && p.y > dy - 4.0;
                if (o > 1.0 && !collar) || i < 1.0 || r < 6.6 {
                    continue;
                }
                let fore = p.z - hz;
                let line = if fore > 4.0 {
                    71.0
                } else if fore > 0.0 {
                    65.0 + fore * 1.5
                } else {
                    63.0 + fore * 0.35 + nz(x, 1, z) * 2.4
                };
                if p.y < line {
                    continue;
                }
                s.set(x, y, z, if nz(x, y / 2, z) < 0.3 { K_HAIR_L } else { K_HAIR });
            }
        }
    }
    shag(&mut s, (w, h, d), K_HAIR, K_HAIR, K_HAIR_L, 0.22, 6);
    // (Shag leaves the body's tag set: the rest of the crown is the head's.)
    s.part(5);
    // Tufts standing out round the dish, and lank locks falling from the hem.
    for k in 0..26 {
        let a = k as f32 / 26.0 * TAU + nz(k, 2, 9) * 0.2;
        let (ca, sa) = (a.cos(), a.sin());
        let base = v(dx + ca * 7.4, dy - 1.8, dz + sa * 7.4);
        let tip = base + v(ca * (1.6 + nz(k, 4, 1) * 1.6), 0.4 + nz(k, 5, 3) * 1.2, sa * (1.6 + nz(k, 6, 2) * 1.6));
        s.capsule((base.x, base.y, base.z), (tip.x, tip.y, tip.z), 0.9, 0.4, if k % 3 == 0 { K_HAIR_L } else { K_HAIR });
    }
    for k in 0..34 {
        let a = k as f32 / 34.0 * TAU;
        let (ca, sa) = (a.cos(), a.sin());
        if sa > 0.3 {
            continue; // not over its face
        }
        let top = v(hc.x + ca * 12.0, 64.0 + nz(k, 1, 7) * 2.0 + sa.max(0.0) * 4.0, hc.z + sa * 11.6);
        let len = 3.0 + nz(k, 8, 1) * 5.0;
        let end = top + v(ca * 0.8, -len, sa * 0.8);
        s.capsule((top.x, top.y, top.z), (end.x, end.y, end.z), 0.8, 0.45, if k % 4 == 0 { K_HAIR_L } else { K_HAIR });
    }
    column(&mut s, dx, dz, 6.8, 6.8, 66, (dy - 1.0) as i32, K_SKIN_D);
    let rim_y = dy - 0.2;
    for y in (dy - 4.0) as i32..=(dy + 3.0) as i32 {
        for z in (dz - 8.0) as i32..=(dz + 8.0) as i32 {
            for x in (dx - 8.0) as i32..=(dx + 8.0) as i32 {
                let r = ((x as f32 - dx).powi(2) + (z as f32 - dz).powi(2)).sqrt();
                let ring = (r - 5.9).powi(2) + (y as f32 - rim_y).powi(2);
                if ring < 1.5 * 1.5 {
                    s.set(x, y, z, if y as f32 > rim_y { DISH } else { DISH_D });
                } else if r < 5.1 && y as f32 > dy - 2.5 {
                    s.clear(x, y, z);
                } else if r < 5.1 && y as f32 > dy - 3.5 {
                    s.set(x, y, z, DISH_IN);
                }
            }
        }
    }

    // ---- the water in the dish, a part of its own (it spills when the kappa bows)
    s.part(6);
    for y in (dy - 2.0) as i32..=dy as i32 {
        for z in (dz - 6.0) as i32..=(dz + 6.0) as i32 {
            for x in (dx - 6.0) as i32..=(dx + 6.0) as i32 {
                let r = ((x as f32 - dx).powi(2) + (z as f32 - dz).powi(2)).sqrt();
                if r < 5.1 {
                    let c = if y < dy as i32 {
                        WATER_D
                    } else if r < 1.6 || ((r - 3.2).abs() < 0.5 && nz(x, y, z) > 0.4) {
                        WATER_L
                    } else {
                        WATER
                    };
                    s.set(x, y, z, c);
                }
            }
        }
    }
    s.part(0);

    // ---- the skin: shadowed in its creases, mottled, warty, paler down the belly
    hollows(&mut s, (w, h, d), 2, &[K_SKIN], |c, depth| {
        if depth > 0.07 {
            K_SKIN_D
        } else if depth < -0.07 {
            K_SKIN_L
        } else {
            c
        }
    });
    s.paint(|x, y, z, c| {
        if c != K_SKIN && c != K_SKIN_L {
            return None;
        }
        if nz(x / 2, y / 2, z / 2) < 0.14 {
            Some(K_SPOT)
        } else if z as f32 > cz + 6.0 && y < 48 && (x as f32 - cx).abs() < 12.0 && y > 19 {
            Some(K_BELLY)
        } else {
            None
        }
    });
    bristles(&mut s, (w, h, d), &[K_SKIN, K_SKIN_L, K_SPOT], 0.04, K_WART);
    s.finish(0.04)
}

// ----------------------------------------------------------------- rokurokubi

/// The rokurokubi's grid: 49 wide, 142 tall (her neck), 45 deep.
pub const ROKURO_SIZE: (i32, i32, i32) = (49, 142, 45);
const ROKURO_X: f32 = 24.0;
const ROKURO_Z: f32 = 22.0;
/// Her neck's axis, from inside her collar up through its curves to under her jaw.
pub const ROKURO_NECK: [[f32; 3]; 9] = [
    [24.0, 37.0, 18.0],
    [25.0, 50.0, 16.0],
    [31.0, 62.0, 16.5],
    [33.0, 74.0, 20.0],
    [27.0, 86.0, 24.0],
    [18.0, 97.0, 24.0],
    [16.0, 107.0, 22.5],
    [21.0, 115.0, 25.0],
    [24.0, 121.0, 28.0],
];
/// Her neck's radius at the collar and under her jaw, in voxels.
pub const ROKURO_NECK_RADII: (f32, f32) = (4.8, 4.2);

/// Her shoulders (her arms in their long sleeves), her neck at the collar, her head atop it.
pub const ROKURO_RIG: &[PartDef] = &[
    PartDef { name: "leftArm", boxes: &[], pivot: [16.0, 36.5, 18.5] },
    PartDef { name: "rightArm", boxes: &[], pivot: [32.0, 36.5, 18.5] },
    PartDef { name: "neck", boxes: &[], pivot: [24.0, 40.0, 18.0] },
    PartDef { name: "head", boxes: &[], pivot: [24.0, 121.0, 28.0] },
];

/// Sets the voxel nearest `p` only if it is empty (hair that must not steal a neck's voxels).
fn put_empty(s: &mut Sculpt, p: Vec3, c: u32) {
    let (x, y, z) = (p.x.round() as i32, p.y.round() as i32, p.z.round() as i32);
    if s.get(x, y, z).is_none() {
        s.set(x, y, z, c);
    }
}

/**
 * The rokurokubi: by day a woman like any other, by night her neck stretches on and on. She
 * kneels small and still in a kimono of indigo so dark it is nearly black, scattered with faint
 * blossoms, its collar white and its lining crimson, a red brocade obi tied in a drum knot at
 * her back, her long sleeves draped to the floor at her sides and her pale hands on her knees;
 * and out of her collar her neck rises in long curves, pale and grey and creased in rings, to
 * her head far above: a face powdered white, thin black brows drawn down, dark eyes with a red
 * light in them, red lips parted on her teeth, and wild black hair hanging long about her.
 */
pub fn rokurokubi() -> Collected {
    let (w, h, d) = ROKURO_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (ROKURO_X, ROKURO_Z);
    let tz = cz - 3.5;

    // ---- kneeling: the kimono over her folded legs, its hem pooled round her, white tabi behind
    s.part(0);
    for z in 0..d {
        for x in 0..w {
            let (ex, ez) = ((x as f32 - cx) / 13.0, (z as f32 - cz - 1.0) / 15.0);
            if ex * ex + ez * ez <= 1.0 {
                s.cuboid(x, 0, z, x, 1, z, R_KIMONO);
            }
        }
    }
    rounded(&mut s, v(cx, 8.0, tz - 1.0), v(10.4, 8.0, 8.4), 2.4, 0, R_KIMONO);
    rounded(&mut s, v(cx, 7.5, cz + 4.5), v(10.2, 6.6, 9.6), 2.4, 0, R_KIMONO);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 5.0, 6.0, cz + 11.5), v(5.2, 5.8, 4.8), R_KIMONO);
        rounded(&mut s, v(cx + side * 3.2, 2.4, tz - 9.5), v(2.4, 2.2, 1.4), 2.0, 0, R_TABI);
    }

    // ---- her body upright from the waist, slender, the shoulders sloping
    for y in 12..=40 {
        let t = (y - 12) as f32 / 28.0;
        let rx = 7.6 + (t * PI).sin() * 0.5 + t * 1.0 - (t - 0.82).max(0.0) * 18.0;
        let rz = 5.2 + t * 0.6;
        slab(&mut s, cx, tz, rx.max(4.0), rz, 2.4, y, y, R_KIMONO);
    }
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 7.0, 36.0, tz), v(3.4, 2.8, 5.0), R_KIMONO);
    }
    // The collar: crossed at her breast, white at the neck, a line of crimson under it.
    s.paint(|x, y, z, c| {
        if c != R_KIMONO || y < 29 || (z as f32) < tz + 1.0 {
            return None;
        }
        let hw = (y as f32 - 29.0) * 0.55;
        let dx = (x as f32 - cx).abs();
        if dx < hw - 2.6 {
            Some(R_UNDER)
        } else if dx < hw - 1.0 {
            Some(R_COLLAR)
        } else if dx < hw {
            Some(R_LINING)
        } else {
            None
        }
    });
    // Behind, the collar drawn down off her nape, as it is worn.
    for z in 0..d {
        for x in 0..w {
            let (dx, dz) = (x as f32 - cx, z as f32 - tz);
            if dz < -1.0 && dx.abs() < 5.0 && s.get(x, 38, z).is_some() {
                s.recolor(x, 38, z, R_COLLAR);
                s.clear(x, 39, z);
                s.clear(x, 40, z);
            }
        }
    }

    // ---- the obi: red brocade figured in gold, a cord round it, the drum knot behind
    slab(&mut s, cx, tz, 8.9, 6.4, 2.4, 17, 27, R_OBI);
    s.paint(|x, y, z, c| {
        if c != R_OBI {
            return None;
        }
        if y == 17 || y == 27 {
            return Some(R_OBI_D);
        }
        let k = (x + z) as f32 * 0.5;
        let lattice = ((k + y as f32 * 0.5).rem_euclid(4.0) < 0.6) || ((k - y as f32 * 0.5).rem_euclid(4.0) < 0.6);
        lattice.then_some(R_OBI_GOLD)
    });
    slab(&mut s, cx, tz, 9.2, 6.7, 2.4, 22, 22, R_CORD);
    let fz = front(&s, cx as i32, 22, d).unwrap_or(26) as f32;
    ball(&mut s, v(cx, 22.0, fz + 0.6), v(1.6, 1.2, 1.0), R_CORD);
    rounded(&mut s, v(cx, 27.5, tz - 8.0), v(7.0, 5.4, 2.6), 2.6, 0, R_OBI);
    rounded(&mut s, v(cx, 20.5, tz - 7.8), v(6.0, 2.4, 2.2), 2.6, 0, R_OBI_D);
    s.paint(|x, y, z, c| {
        (c == R_OBI && (z as f32) < tz - 6.0 && ((x + y).rem_euclid(5) == 0 || (x - y).rem_euclid(5) == 0)).then_some(R_OBI_GOLD)
    });

    // ---- arms: in long sleeves draped to the floor at her sides, her pale hands on her knees
    for (i, out) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        let sh = v(cx + out * 8.0, 36.5, tz);
        let el = v(cx + out * 10.6, 25.5, tz + 1.5);
        let wr = v(cx + out * 6.4, 15.8, cz + 6.0);
        ball(&mut s, sh, Vec3::splat(3.3), R_KIMONO);
        limb(&mut s, &[(sh, 3.2), (el, 3.1)], R_KIMONO);
        sleeve(&mut s, &[(el, 3.3), (wr, 3.5)], R_KIMONO, R_LINING);
        // The sleeve's long drop, hanging from her forearm and draped over her thigh to the floor.
        let (z0, z1, sx) = (tz - 4.0, wr.z - 1.0, cx + out * 11.6);
        for z in z0 as i32..=z1 as i32 {
            let zf = z as f32;
            let t = ((zf - el.z) / (wr.z - el.z)).clamp(0.0, 1.0);
            let (top, ax) = (el.y + (wr.y - el.y) * t, el.x + (wr.x - el.x) * t);
            for y in 1..=top.round() as i32 {
                let corner = 4.5;
                let u = (zf - (z1 - corner)).max(z0 + corner - zf);
                let k = corner + 1.0 - y as f32;
                if u > 0.0 && k > 0.0 && u * u + k * k > corner * corner {
                    continue;
                }
                let drape = ((top - y as f32) / 7.0).clamp(0.0, 1.0);
                let x = ax + (sx - ax) * drape * drape * (3.0 - 2.0 * drape);
                let c = if y <= 2 { R_LINING } else { R_KIMONO };
                put(&mut s, v(x, y as f32, zf), c);
                put(&mut s, v(x + out, y as f32, zf), c);
            }
        }
        // The hand: slender and pale, out of the cuff and over her knee, the nails lacquered red.
        let dir = (wr - el).normalize();
        let palm = wr + dir * 2.6;
        ball(&mut s, palm, v(2.2, 1.1, 2.6), R_SKIN);
        for (k, f) in [-1.5f32, -0.5, 0.5, 1.5].into_iter().enumerate() {
            let base = palm + v(f * 0.95, 0.1, 1.8);
            let tip = base + v(f * 0.3, -1.3 - (k % 2) as f32 * 0.3, 3.0 - (f.abs() - 0.5) * 0.4);
            s.capsule((base.x, base.y, base.z), (tip.x, tip.y, tip.z), 0.65, 0.5, R_SKIN);
            put(&mut s, tip + v(0.0, -0.1, 0.7), R_NAIL);
        }
        let tb = palm + v(-out * 1.8, 0.3, 0.6);
        s.capsule((tb.x, tb.y, tb.z), (tb.x - out * 0.9, tb.y - 0.4, tb.z + 2.2), 0.65, 0.5, R_SKIN);
    }

    // ---- the kimono's folds and its faint blossoms
    s.paint(|x, y, z, c| {
        if c != R_KIMONO {
            return None;
        }
        // Blossoms, scattered thinly: each a pale heart and its four petals.
        let cell = (x.div_euclid(7), y.div_euclid(7), z.div_euclid(7));
        let at = (cell.0 * 7 + 3, cell.1 * 7 + 3, cell.2 * 7 + 3);
        if nz(cell.0, cell.1, cell.2) > 0.55 {
            let (dx, dy, dz) = ((x - at.0).abs(), (y - at.1).abs(), (z - at.2).abs());
            if dx + dy + dz <= 1 {
                return Some(if dx + dy + dz == 0 { R_PETAL_C } else { R_PETAL });
            }
        }
        // Folds: round her body, and running along her lap toward her knees.
        let lap = (z as f32) > tz + 6.0 && y < 15;
        let fold = if lap {
            (x as f32 * 1.05 + (z as f32 * 0.3).sin()).sin()
        } else {
            ((z as f32 - tz).atan2(x as f32 - cx) * 7.0 + y as f32 * 0.05).sin()
        };
        if fold > 0.8 {
            Some(R_KIMONO_D)
        } else if fold < -0.88 {
            Some(R_KIMONO_L)
        } else {
            None
        }
    });

    // ---- the neck: rising in long curves from her collar, pale grey, creased in rings
    s.part(3);
    let path = spline(&ROKURO_NECK.map(Vec3::from), 10);
    let (r0, r1) = ROKURO_NECK_RADII;
    let n = path.len() as f32 - 1.0;
    let knots: Vec<(Vec3, f32)> = path.iter().enumerate().map(|(k, p)| (*p, r0 + (r1 - r0) * k as f32 / n)).collect();
    let total: f32 = path.windows(2).map(|w| (w[1] - w[0]).length()).sum();
    sweep(&mut s, &knots, |p, c, along| {
        let ring = (along * total / 3.6).fract();
        let up = (p - c).normalize_or_zero();
        if ring < 0.16 {
            NECK_D
        } else if ring < 0.34 || up.y > 0.7 {
            NECK_L
        } else {
            NECK
        }
    });
    // Below the collar its root is her body's, so that when the game hides the neck to draw it
    // stretching, her collar is not left open.
    s.part(0);
    let [nx, ny, nz0] = ROKURO_NECK[0];
    for y in (ny - 6.0) as i32..=40 {
        for z in (nz0 - 6.0) as i32..=(nz0 + 6.0) as i32 {
            for x in (nx - 6.0) as i32..=(nx + 6.0) as i32 {
                if s.tag_at(x, y, z) == 3 && s.get(x, y, z).is_some() {
                    s.set(x, y, z, NECK_D);
                }
            }
        }
    }

    // ---- the head: an oval face powdered white, the glare, red lips parted on her teeth
    s.part(4);
    let hc = v(cx, 127.4, cz + 7.5);
    ball(&mut s, hc, v(7.2, 8.8, 7.6), R_FACE);
    rounded(&mut s, v(cx, 121.8, hc.z + 1.8), v(4.8, 3.6, 5.0), 2.2, 0, R_FACE);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 3.8, 125.8, hc.z + 4.2), v(2.5, 2.3, 2.5), R_FACE);
    }
    hollows(&mut s, (w, h, d), 2, &[R_FACE], |c, depth| if depth > 0.08 { R_FACE_D } else { c });
    // Eyes: dark, the whites at their corners, a red light in them, lined black and winged.
    for side in [-1.0f32, 1.0] {
        let x = |k: i32| (cx + side * k as f32) as i32;
        for y in 128..=129 {
            face(&mut s, x(2), y, d, 0, SCLERA);
            face(&mut s, x(3), y, d, 0, R_EYE);
            face(&mut s, x(4), y, d, 0, R_EYE);
            face(&mut s, x(5), y, d, 0, if y == 128 { SCLERA } else { R_LASH });
        }
        face(&mut s, x(3), 129, d, 0, R_GLINT);
        for k in 2..=6 {
            face(&mut s, x(k), 130, d, 0, R_LASH);
        }
        face(&mut s, x(7), 130, d, 0, R_LASH);
        face(&mut s, x(6), 129, d, 0, R_LIP);
        for k in 2..=5 {
            face(&mut s, x(k), 127, d, 0, R_FACE_D);
        }
        face(&mut s, x(1), 128, d, 0, R_FACE_D);
        // Thin brows, drawn down hard toward her nose.
        for (k, y) in [(1, 131), (2, 131), (3, 132), (4, 132), (5, 133), (6, 133)] {
            face(&mut s, x(k), y, d, 0, R_LASH);
        }
    }
    // A small nose, a shadow under it.
    face(&mut s, cx as i32, 126, d, 1, R_FACE);
    for k in [-1, 1] {
        face(&mut s, cx as i32 + k, 125, d, 0, R_FACE_D);
    }
    // The lips, red, parted on white teeth; dark at the corners of her mouth.
    for k in -2..=2 {
        let x = cx as i32 + k;
        face(&mut s, x, 123, d, 0, if k.abs() == 2 { R_LIP } else { R_LIP_D });
        face(&mut s, x, 122, d, 0, if k.abs() == 2 { CAVITY } else { R_TEETH });
        face(&mut s, x, 121, d, 0, R_LIP);
    }
    for k in [-1, 1] {
        face(&mut s, cx as i32 + k, 124, d, 0, R_LIP);
        face(&mut s, cx as i32 + k * 3, 122, d, 0, R_FACE_D);
    }
    face(&mut s, cx as i32, 120, d, 0, R_FACE_D);

    // ---- the hair: parted over her crown, wild and long, hanging about her in ragged locks
    for y in 112..=139 {
        for z in 0..d {
            for x in 0..w {
                let p = v(x as f32, y as f32, z as f32);
                let (o, i) = (((p - hc) / v(8.6, 10.2, 9.2)).length(), ((p - hc) / v(7.0, 8.6, 7.4)).length());
                if o > 1.0 || i < 1.0 {
                    continue;
                }
                let fore = p.z - hc.z;
                if fore > 1.0 && (p.x - cx).abs() < 6.4 && p.y < 133.5 + (p.x - cx).abs() * 0.2 {
                    continue; // her face
                }
                if p.y < 120.0 && fore > -2.0 {
                    continue;
                }
                let part = (p.x - cx).abs() < 0.6 && fore > -4.0 && p.y > 134.0;
                put_empty(&mut s, p, if part || nz(x, y / 3, z) < 0.25 { R_HAIR_L } else { R_HAIR });
            }
        }
    }
    for k in 0..64 {
        let a = k as f32 / 64.0 * TAU + nz(k, 3, 3) * 0.15;
        let (ca, sa) = (a.cos(), a.sin());
        if sa > 0.5 {
            continue; // not over her face
        }
        let start = v(hc.x + ca * 7.8, 124.0 + nz(k, 1, 5) * 7.0 + (-sa).max(0.0) * 5.0, hc.z + sa * 8.2);
        let wild = if nz(k, 9, 9) > 0.86 { 0.24 } else { 0.03 + nz(k, 8, 2) * 0.06 };
        let end = 82.0 + nz(k, 2, 7) * 24.0 + sa.max(0.0) * 10.0;
        let (phase, thick, len) = (nz(k, 4, 4) * TAU, 0.7 + nz(k, 6, 1) * 0.6, start.y - end);
        let mut y = start.y;
        while y > end {
            let drop = start.y - y;
            let sway = (drop * 0.17 + phase).sin() * (0.4 + drop * 0.03);
            let p = v(start.x + ca * drop * wild - sway * sa, y, start.z + sa * drop * wild * 0.8 + sway * ca);
            let r = thick * (1.0 - 0.7 * drop / len);
            let c = if nz(k, y as i32 / 2, 6) < 0.2 { R_HAIR_L } else { R_HAIR };
            for dz in -1..=1 {
                for dx in -1..=1 {
                    if (dx * dx + dz * dz) as f32 <= r * r + 0.2 {
                        put_empty(&mut s, p + v(dx as f32, 0.0, dz as f32), c);
                    }
                }
            }
            y -= 0.5;
        }
    }
    shag(&mut s, (w, h, d), R_HAIR, R_HAIR, R_HAIR_L, 0.12, 3);
    // What is left of how she wears it by day: a tortoiseshell comb, rimmed in gold, caught in the
    // back of it, and a gold pin with a red bead.
    s.part(4);
    for k in 0..=30 {
        let a = (20.0 + k as f32 * 140.0 / 30.0).to_radians();
        let (x, y) = (cx + a.cos() * 4.6, 131.0 + a.sin() * 2.6);
        for (dy, c) in [(0.0, R_OBI_GOLD), (-1.0, R_COMB), (-2.0, R_COMB)] {
            put(&mut s, v(x, y + dy, hc.z - 8.9), c);
        }
    }
    let (pin, bead) = (v(cx + 2.0, 133.0, hc.z - 4.0), v(cx + 10.4, 135.4, hc.z - 6.4));
    s.capsule((pin.x, pin.y, pin.z), (bead.x, bead.y, bead.z), 0.5, 0.5, R_OBI_GOLD);
    ball(&mut s, bead, Vec3::splat(1.3), R_BEAD);
    s.finish(0.04)
}

// ------------------------------------------------------------------------ oni

/// The oni's grid: 91 wide (his club), 138 tall, 57 deep.
pub const ONI_SIZE: (i32, i32, i32) = (91, 138, 57);
const ONI_X: f32 = 45.0;
const ONI_Z: f32 = 28.0;
/// The kanabo's axis from his right fist, down and a little out and forward, its head near the floor.
const KANABO_DIR: Vec3 = Vec3::new(0.06, -1.0, 0.18);
/// How far the kanabo reaches past the fist, in voxels.
const KANABO_LEN: f32 = 46.0;

/// Hips, shoulders and neck; the kanabo goes with the right hand.
pub const ONI_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [34.0, 48.0, ONI_Z] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [56.0, 48.0, ONI_Z] },
    PartDef { name: "leftArm", boxes: &[], pivot: [20.0, 94.0, ONI_Z] },
    PartDef { name: "rightArm", boxes: &[], pivot: [70.0, 94.0, ONI_Z] },
    PartDef { name: "head", boxes: &[], pivot: [45.0, 98.0, 29.5] },
];

/**
 * The kanabo in the fist at `fist`: a club of iron, eight-sided and swelling toward its head,
 * studded in rings of spikes, bound in iron below the fist, at its middle and at its end, its
 * leather-wrapped grip inside the fist.
 */
fn kanabo(s: &mut Sculpt, fist: Vec3) {
    let dir = KANABO_DIR.normalize();
    let e1 = dir.cross(Vec3::Z).normalize();
    let e2 = dir.cross(e1);
    let radius = |t: f32| -> f32 {
        if t < 6.5 {
            2.2
        } else {
            2.7 + 2.7 * ((t - 6.5) / (KANABO_LEN - 6.5)).clamp(0.0, 1.0).sqrt()
        }
    };
    let (lo, hi) = (fist - dir * 5.0, fist + dir * KANABO_LEN);
    let (a, b) = (lo.min(hi) - Vec3::splat(7.0), lo.max(hi) + Vec3::splat(7.0));
    for y in a.y.floor() as i32..=b.y.ceil() as i32 {
        for z in a.z.floor() as i32..=b.z.ceil() as i32 {
            for x in a.x.floor() as i32..=b.x.ceil() as i32 {
                let p = v(x as f32, y as f32, z as f32) - fist;
                let t = p.dot(dir);
                if !(-4.5..=KANABO_LEN).contains(&t) {
                    continue;
                }
                let (u, w) = (p.dot(e1), p.dot(e2));
                // An octagon: a square with its corners cut.
                let oct = u.abs().max(w.abs()).max((u.abs() + w.abs()) * std::f32::consts::FRAC_1_SQRT_2);
                if oct > radius(t) {
                    continue;
                }
                let facet = ((w.atan2(u) / (TAU / 8.0)).round() as i32).rem_euclid(8);
                let c = if t < 6.5 {
                    if (t * 0.7).rem_euclid(2.0) < 1.0 {
                        O_GRIP
                    } else {
                        O_GRIP_L
                    }
                } else if t < 8.5 || t > KANABO_LEN - 2.5 || (t - 28.0).abs() < 1.0 {
                    IRON_BAND
                } else if facet == 1 || facet == 2 {
                    IRON_L
                } else if facet >= 5 {
                    IRON_D
                } else {
                    IRON
                };
                s.set(x, y, z, c);
            }
        }
    }
    // Rings of spikes down the swelling head, each ring turned half a step from the last.
    let (mut ring, mut t) = (0, 13.0);
    while t < KANABO_LEN - 3.0 {
        let r = radius(t);
        for k in 0..8 {
            let a = (k as f32 + if ring % 2 == 0 { 0.0 } else { 0.5 }) * TAU / 8.0;
            let out = e1 * a.cos() + e2 * a.sin();
            let base = fist + dir * t + out * (r - 0.4);
            let tip = base + out * (2.0 + r * 0.25);
            s.capsule((base.x, base.y, base.z), (tip.x, tip.y, tip.z), 1.1, 0.3, STUD);
        }
        t += 4.6;
        ring += 1;
    }
}

/**
 * The oni: the ogre of the old tales, half again a man's height and red as a forge, slabs of
 * muscle on him; two short horns on his brow, a wild black mane and beard, a scowl under a brow
 * like a cliff, gold eyes, tusks jutting up from his jaw; his bare chest crossed with studded
 * leather straps, a rope belt knotted over a tiger-skin loincloth, iron cuffs on his wrists, gold
 * rings in his ears, bare clawed feet; and in his right fist the kanabo, the iron club of the
 * saying, hanging at his side with its spiked head near the floor.
 */
pub fn oni() -> Collected {
    let (w, h, d) = ONI_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (ONI_X, ONI_Z);

    // ---- legs: great thighs, heavy calves, broad bare feet with black claws
    for (i, out) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        let hip = v(cx + out * 11.0, 48.0, cz);
        let knee = v(cx + out * 15.0, 27.0, cz + 1.0);
        let ankle = v(cx + out * 16.0, 7.5, cz - 2.0);
        limb(&mut s, &[(hip, 10.0), (knee, 7.4)], O_SKIN);
        ball(&mut s, v(hip.x + out * 1.0, 38.0, cz + 3.6), v(8.2, 9.5, 6.8), O_SKIN);
        ball(&mut s, v(hip.x - out * 2.6, 40.0, cz + 1.0), v(5.8, 7.5, 6.2), O_SKIN);
        ball(&mut s, knee, Vec3::splat(7.0), O_SKIN);
        limb(&mut s, &[(knee, 6.8), (ankle, 5.0)], O_SKIN);
        ball(&mut s, v(knee.x, 19.0, cz - 3.6), v(6.2, 7.2, 5.6), O_SKIN);
        let fx = ankle.x + out * 0.3;
        ball(&mut s, v(fx, 3.9, cz + 1.4), v(6.0, 3.9, 8.4), O_SKIN);
        for k in 0..4 {
            let tx = fx + (k as f32 - 1.5) * 2.5;
            let tz = cz + 8.8 - (k as f32 - 1.5 - out * 0.5).abs() * 0.9;
            ball(&mut s, v(tx, 2.2, tz), v(1.6, 2.1, 2.3), O_SKIN);
            s.capsule((tx, 2.8, tz + 1.7), (tx, 0.6, tz + 3.5), 1.0, 0.35, O_CLAW);
        }
    }

    // ---- the trunk: a barrel of a gut, a slab of a chest, a back like a wall
    s.part(0);
    rounded(&mut s, v(cx, 49.0, cz), v(17.0, 8.0, 12.0), 2.4, 0, O_SKIN);
    ball(&mut s, v(cx, 62.0, cz + 4.0), v(18.0, 13.5, 13.5), O_SKIN);
    rounded(&mut s, v(cx, 80.0, cz + 1.0), v(20.5, 15.0, 12.0), 2.2, 0, O_SKIN);
    ball(&mut s, v(cx, 84.0, cz - 3.0), v(23.5, 13.0, 10.0), O_SKIN);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 8.8, 88.0, cz + 9.0), v(10.0, 7.4, 5.8), O_SKIN);
        ball(&mut s, v(cx + side * 10.5, 99.5, cz - 1.0), v(11.0, 7.0, 7.4), O_SKIN);
        ball(&mut s, v(cx + side * 16.0, 70.0, cz + 4.0), v(4.5, 9.0, 7.5), O_SKIN);
    }
    // The belly's muscle, three rows either side of the line down its middle.
    for row in 0..3 {
        for side in [-1.0f32, 1.0] {
            let (x, y) = (cx + side * 4.0, 78.0 - row as f32 * 5.6);
            let z = front(&s, x.round() as i32, y.round() as i32, d).unwrap_or(40) as f32;
            ball(&mut s, v(x, y, z - 1.5), v(3.6, 2.6, 2.2), O_SKIN);
        }
    }
    limb(&mut s, &[(v(cx, 95.0, cz + 1.0), 8.4), (v(cx, 104.0, cz + 3.0), 7.6)], O_SKIN);
    // The clefts of his muscle cut in: under each pec, down the middle of his belly, between the
    // rows of it; his navel.
    for x in (cx - 17.0) as i32..=(cx + 17.0) as i32 {
        let k = (x as f32 - cx).abs();
        let under = 80.4 + (k - 8.8).abs() * 0.22 + (k < 1.5) as i32 as f32 * 1.6;
        face(&mut s, x, under.round() as i32, d, 0, O_SKIN_D);
        if k < 7.5 && k > 0.5 {
            for y in [75, 69] {
                face(&mut s, x, y, d, 0, O_SKIN_D);
            }
        }
    }
    for y in 61..=81 {
        face(&mut s, cx as i32, y, d, 0, O_SKIN_D);
    }
    for (dx, dy) in [(0, 0), (0, -1), (-1, 0), (1, 0)] {
        face(&mut s, cx as i32 + dx, 62 + dy, d, 0, CAVITY);
    }

    // ---- the tiger-skin loincloth: wrapped round his hips, a flap before and behind, ragged
    for y in 31..=57 {
        let t = (57 - y) as f32 / 26.0;
        let (rx, rz) = (18.4 + t * 2.2, 13.4 + t * 1.6);
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = (x as f32 - cx, z as f32 - cz);
                let q = (dx / rx).abs().powf(2.4) + (dz / rz).abs().powf(2.4);
                let qi = (dx / (rx - 2.2)).abs().powf(2.4) + (dz / (rz - 2.2)).abs().powf(2.4);
                if q > 1.0 || (qi < 1.0 && y < 50) {
                    continue;
                }
                let flap = dx.abs() < 7.4;
                let hem = if flap && dz > 0.0 {
                    31.0
                } else if flap {
                    35.0
                } else {
                    44.0 + dx.abs().min(16.0) * 0.1
                };
                if (y as f32) < hem + nz(x, 2, z) * 3.0 {
                    continue;
                }
                let a = dz.atan2(dx);
                let stripe = (a * 9.0 + (y as f32 * 0.28).sin() * 1.4 + nz(x / 3, y / 4, z / 3) * 1.2).sin();
                let taper = (y as f32 - hem) / (57.0 - hem).max(1.0);
                let c = if stripe > 0.62 + taper * 0.25 {
                    TIGER_STRIPE
                } else if (y as f32) < hem + 4.0 && flap && nz(x, y, z) < 0.5 {
                    TIGER_PALE
                } else if nz(x, y, z) > 0.85 {
                    TIGER_L
                } else {
                    TIGER
                };
                s.set(x, y, z, c);
            }
        }
    }
    // The tiger's tail, hanging behind from under the belt, ringed and tipped black.
    let tail: Vec<(Vec3, f32)> = spline(
        &[v(cx + 7.0, 55.0, cz - 14.0), v(cx + 9.0, 47.0, cz - 16.5), v(cx + 8.0, 37.0, cz - 16.0), v(cx + 10.5, 28.0, cz - 14.5)],
        6,
    )
    .into_iter()
    .map(|p| (p, 1.7))
    .collect();
    sweep(&mut s, &tail, |p, _c, along| if along > 0.88 || (p.y * 0.55).rem_euclid(2.0) < 0.8 { TIGER_STRIPE } else { TIGER });
    // The rope belt: twisted straw, knotted at the front, its ends hanging.
    for k in 0..220 {
        let a = k as f32 / 220.0 * TAU;
        let c = v(cx + a.cos() * 19.0, 57.5, cz + a.sin() * 14.4);
        ball(&mut s, c, Vec3::splat(1.7), if ((a * 32.0) as i32) % 2 == 0 { ROPE } else { ROPE_D });
    }
    let kz = front(&s, cx as i32, 57, d).unwrap_or(44) as f32;
    ball(&mut s, v(cx, 57.5, kz + 0.4), v(2.8, 2.4, 1.8), ROPE_D);
    for side in [-1.0f32, 1.0] {
        s.capsule((cx + side * 0.8, 56.5, kz + 0.6), (cx + side * 2.4, 46.5, kz + 1.4), 1.0, 0.9, ROPE);
        ball(&mut s, v(cx + side * 2.6, 45.5, kz + 1.4), v(1.4, 1.8, 1.4), ROPE_D);
    }

    // ---- the straps: crossed over his chest and back, studded with iron, a boss where they cross
    let line = |x: f32, y: f32, side: f32| -> (f32, f32) {
        // From the shoulder on `side` down across to the other hip: how far from it, how far along.
        let (a, b) = (v(cx + side * 16.0, 99.0, 0.0), v(cx - side * 16.5, 59.0, 0.0));
        let ab = b - a;
        let t = ((x - a.x) * ab.x + (y - a.y) * ab.y) / ab.length_squared();
        let c = a + ab * t.clamp(0.0, 1.0);
        (((x - c.x).powi(2) + (y - c.y).powi(2)).sqrt(), t * ab.length())
    };
    let mut straps = vec![];
    for y in 56..=102 {
        for x in (cx - 21.0) as i32..=(cx + 21.0) as i32 {
            for side in [-1.0f32, 1.0] {
                let (dist, along) = line(x as f32, y as f32, side);
                if dist <= 2.0 {
                    let c = if along.rem_euclid(6.0) < 1.0 && dist < 0.8 {
                        STUD
                    } else if dist > 1.4 {
                        O_STRAP_D
                    } else {
                        O_STRAP
                    };
                    straps.push((x, y, c));
                }
            }
        }
    }
    for (x, y, c) in straps {
        if let Some(z) = front(&s, x, y, d) {
            s.set(x, y, z + 1, c);
        }
        if let Some(z) = (0..d).find(|&z| s.get(x, y, z).is_some()) {
            s.set(x, y, z - 1, c);
        }
    }
    let (sy, bz) = (79.0, front(&s, cx as i32, 79, d).unwrap_or(44) as f32);
    ball(&mut s, v(cx, sy, bz + 0.2), v(3.0, 3.0, 1.2), IRON_BAND);
    ball(&mut s, v(cx, sy, bz + 0.9), v(1.2, 1.2, 0.8), STUD);

    // ---- arms: thick as a man's waist, cuffed in iron at the wrist; the kanabo in his right fist
    for (i, out) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(3 + i as u8);
        let sh = v(cx + out * 25.0, 94.0, cz);
        let el = v(cx + out * 30.5, 72.0, cz + 1.0);
        let wr = v(cx + out * 31.5, 56.0, cz + 4.0);
        ball(&mut s, sh, v(9.4, 8.6, 9.0), O_SKIN);
        limb(&mut s, &[(sh, 8.0), (el, 6.6)], O_SKIN);
        ball(&mut s, v(sh.x + out * 0.6, 82.0, cz + 4.0), v(6.4, 8.4, 5.8), O_SKIN);
        ball(&mut s, v(sh.x + out * 1.6, 82.0, cz - 3.4), v(6.0, 8.8, 5.6), O_SKIN);
        ball(&mut s, el, Vec3::splat(6.8), O_SKIN);
        limb(&mut s, &[(el, 7.0), (wr, 5.7)], O_SKIN);
        ball(&mut s, v(el.x + out * 0.6, 66.0, cz + 2.6), v(6.8, 7.4, 6.2), O_SKIN);
        // The iron cuff, studded.
        for y in 54..=60 {
            let t = (y as f32 - wr.y) / (el.y - wr.y);
            let c = wr + (el - wr) * t;
            column(&mut s, c.x, c.z, 6.5, 6.5, y, y, if y == 54 || y == 60 { IRON_BAND } else { IRON });
        }
        for k in 0..12 {
            let a = k as f32 / 12.0 * TAU;
            put(&mut s, v(wr.x + a.cos() * 6.9, 57.0, wr.z + a.sin() * 6.9), STUD);
        }
        // The fist, the knuckles before it.
        let fist = v(wr.x, 50.0, cz + 5.2);
        ball(&mut s, fist, v(5.7, 6.0, 5.8), O_SKIN);
        for k in 0..4 {
            let y = fist.y + 2.8 - k as f32 * 2.0;
            let z = front(&s, fist.x.round() as i32, y.round() as i32, d).unwrap_or(40) as f32;
            ball(&mut s, v(fist.x - out * 0.6, y, z - 0.2), v(3.6, 1.15, 1.2), O_SKIN_D);
        }
        ball(&mut s, v(fist.x - out * 3.6, fist.y + 2.4, fist.z + 2.2), v(1.9, 1.7, 2.1), O_SKIN);
        if i == 1 {
            kanabo(&mut s, fist);
        }
    }

    // ---- the head: a scowl under a brow like a cliff, gold eyes, tusks, the horns, ears ringed in gold
    s.part(5);
    let hz = cz + 4.5;
    ball(&mut s, v(cx, 114.0, hz), v(11.0, 12.0, 11.0), O_SKIN);
    rounded(&mut s, v(cx, 105.5, hz + 3.8), v(10.2, 5.6, 8.0), 2.3, 0, O_SKIN);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 6.8, 110.5, hz + 7.8), v(3.8, 3.3, 3.3), O_SKIN);
        s.capsule((cx + side * 1.2, 115.2, hz + 11.2), (cx + side * 9.4, 118.2, hz + 7.8), 2.6, 2.1, O_SKIN);
    }
    ball(&mut s, v(cx, 110.6, hz + 11.6), v(3.6, 2.9, 2.7), O_SKIN);
    ball(&mut s, v(cx, 113.4, hz + 11.0), v(1.7, 2.0, 1.5), O_SKIN);
    hollows(&mut s, (w, h, d), 2, &[O_SKIN], |c, depth| if depth > 0.08 { O_SKIN_D } else { c });
    // Gold eyes deep under the brow, slit-pupilled, in dark sockets.
    for side in [-1.0f32, 1.0] {
        let x = |k: i32| (cx + side * k as f32) as i32;
        for k in 2..=7 {
            for y in 112..=115 {
                face(&mut s, x(k), y, d, 0, O_SOCKET);
            }
        }
        for k in 3..=5 {
            for y in 113..=114 {
                face(&mut s, x(k), y, d, 0, if k == 4 { DARK_PUPIL } else { O_EYE });
            }
        }
        face(&mut s, x(5), 114, d, 0, O_EYE_L);
    }
    // Nostrils flared, a furrow between the brows.
    for k in [-1, 1] {
        face(&mut s, cx as i32 + k * 2, 110, d, 0, CAVITY);
    }
    for y in 115..=117 {
        face(&mut s, cx as i32, y, d, 0, O_SKIN_D);
    }
    // The mouth: a scowl, the lips dark, teeth set in it, the tusks jutting up from the jaw.
    for x in (cx - 5.0) as i32..=(cx + 5.0) as i32 {
        let k = (x as f32 - cx).abs();
        face(&mut s, x, 107, d, 0, O_LIP);
        face(&mut s, x, 106, d, 0, if k < 4.5 && x % 2 == 0 { TOOTH } else { CAVITY });
        face(&mut s, x, 105, d, 0, if k < 4.5 { CAVITY } else { O_LIP });
        face(&mut s, x, 104, d, 0, O_LIP);
    }
    for side in [-1.0f32, 1.0] {
        let x = cx + side * 4.6;
        let z = front(&s, x.round() as i32, 105, d).unwrap_or(40) as f32;
        s.capsule((x, 103.0, z - 0.4), (x + side * 0.9, 110.0, z + 0.8), 1.4, 0.45, TOOTH);
        let x2 = cx + side * 2.0;
        let z2 = front(&s, x2.round() as i32, 106, d).unwrap_or(40) as f32;
        s.capsule((x2, 106.4, z2), (x2, 104.4, z2 + 0.2), 0.7, 0.3, TOOTH);
    }
    // Pointed ears, a gold ring in each.
    for side in [-1.0f32, 1.0] {
        s.capsule((cx + side * 10.5, 113.0, hz + 1.0), (cx + side * 15.8, 116.6, hz - 1.4), 2.6, 0.4, O_SKIN);
        s.set((cx + side * 12.0).round() as i32, 113, (hz + 0.8).round() as i32, O_SKIN_D);
        for k in 0..16 {
            let a = k as f32 / 16.0 * TAU;
            put(&mut s, v(cx + side * 11.6, 107.6 + a.sin() * 2.2, hz + 1.0 + a.cos() * 2.2), O_GOLD);
        }
    }
    // The beard: black and wild, from his cheeks down over his chest, clear of his mouth.
    for y in 90..=110 {
        for z in 0..d {
            for x in 0..w {
                let p = v(x as f32, y as f32, z as f32);
                let q = (p - v(cx, 101.0, hz + 6.4)) / v(11.6, 11.0, 7.0);
                if q.length() > 1.0 || s.get(x, y, z).is_some_and(|c| c != O_SKIN && c != O_SKIN_D) {
                    continue;
                }
                let dx = (p.x - cx).abs();
                if (p.y > 103.0 && dx < 6.6) || (p.y > 108.0 && dx < 9.0) || p.z < hz + 1.0 {
                    continue; // his mouth and cheeks
                }
                s.set(x, y, z, if nz(x, y / 2, z) < 0.3 { O_HAIR_L } else { O_HAIR });
            }
        }
    }
    // The mane: over his crown and down his nape, standing out in wild locks.
    let mane = v(cx, 116.0, hz - 1.0);
    for y in 102..=131 {
        for z in 0..d {
            for x in 0..w {
                let p = v(x as f32, y as f32, z as f32);
                let (o, i) = (((p - mane) / v(13.8, 13.4, 13.2)).length(), ((p - v(cx, 114.0, hz)) / v(10.6, 11.6, 10.6)).length());
                if o > 1.0 || i < 1.0 {
                    continue;
                }
                let fore = p.z - hz;
                if (fore > 3.0 && p.y < 121.0 + (p.x - cx).abs() * 0.15) || (fore > -4.0 && p.y < 110.0) {
                    continue; // his face and neck
                }
                s.set(x, y, z, if nz(x, y / 2, z) < 0.3 { O_HAIR_L } else { O_HAIR });
            }
        }
    }
    for k in 0..46 {
        let a = nz(k, 3, 1) * TAU;
        let up = nz(k, 5, 2) * 0.85 - 0.2;
        let n = v(a.cos() * (1.0 - up * up).sqrt(), up, a.sin() * (1.0 - up * up).sqrt());
        if n.z > 0.45 && n.y < 0.5 {
            continue; // not over his face
        }
        let base = mane + n * v(13.0, 12.6, 12.4);
        let mut tip = base + (n + v(0.0, 0.3, -0.35)).normalize() * (4.0 + nz(k, 7, 3) * 4.0);
        tip.y = tip.y.min(131.0);
        s.capsule((base.x, base.y, base.z), (tip.x, tip.y, tip.z), 2.0, 0.4, if k % 3 == 0 { O_HAIR_L } else { O_HAIR });
    }
    // The horns: short and thick on his brow, ringed, darkening to their points.
    for side in [-1.0f32, 1.0] {
        let n = 40;
        for k in 0..=n {
            let t = k as f32 / n as f32;
            let c = v(cx + side * (4.8 + 1.6 * t), 122.0 + 8.4 * t, hz + 7.0 + 2.0 * t - 1.4 * t * t);
            let r = 3.0 - 2.5 * t;
            let col = if t > 0.8 {
                HORN_TIP
            } else if (t * 9.0).fract() < 0.25 {
                HORN_D
            } else {
                HORN
            };
            s.ellipsoid((c.x, c.y, c.z), (r, r, r), 0, col);
        }
    }
    s.part(0);

    // ---- the skin: shadowed in the clefts of his muscle, catching the light on its swells
    hollows(&mut s, (w, h, d), 3, &[O_SKIN], |c, depth| {
        if depth > 0.045 {
            O_SKIN_D
        } else if depth < -0.085 {
            O_SKIN_L
        } else {
            c
        }
    });
    s.paint(|x, y, z, c| (c == O_SKIN && nz(x / 2, y / 2, z / 2) < 0.12).then_some(O_SKIN_M));
    shag(&mut s, (w, h, d), O_HAIR, O_HAIR, O_HAIR_L, 0.3, 4);
    s.finish(0.04)
}

// -------------------------------------------------------------------- kitsune

/// The kitsune's grid: 45 wide, 102 tall (its tail and the foxfire over it), 117 long.
pub const KITSUNE_SIZE: (i32, i32, i32) = (45, 102, 117);
const KITSUNE_X: f32 = 22.0;
/// The tail's axis, from its root at the rump back and up through an S to its white tip.
const KITSUNE_TAIL: [[f32; 3]; 8] = [
    [22.0, 43.0, 31.0],
    [22.5, 40.0, 23.0],
    [23.5, 45.0, 15.0],
    [23.0, 56.0, 11.0],
    [21.0, 68.0, 13.5],
    [20.5, 78.0, 19.0],
    [21.5, 84.0, 26.0],
    [22.0, 85.5, 31.0],
];
/// The foxfire floating over the tail's tip: the middle of its flame.
pub const KITSUNE_FOXFIRE: [f32; 3] = [22.0, 93.5, 31.0];

/// Shoulders and hips, the neck where it leaves the chest, the tail's root; the foxfire goes with the tail.
pub const KITSUNE_RIG: &[PartDef] = &[
    PartDef { name: "frontLeftLeg", boxes: &[], pivot: [16.5, 41.0, 74.0] },
    PartDef { name: "frontRightLeg", boxes: &[], pivot: [27.5, 41.0, 74.0] },
    PartDef { name: "backLeftLeg", boxes: &[], pivot: [16.0, 40.0, 39.0] },
    PartDef { name: "backRightLeg", boxes: &[], pivot: [28.0, 40.0, 39.0] },
    PartDef { name: "head", boxes: &[], pivot: [22.0, 45.0, 81.0] },
    PartDef { name: "tail", boxes: &[], pivot: [22.0, 43.0, 31.0] },
];

/// A fox's paw at (x, z): small and neat, its toes and black claws.
fn fox_paw(s: &mut Sculpt, x: f32, z: f32) {
    ball(s, v(x, 1.8, z), v(2.4, 1.8, 3.0), F_BLACK);
    for k in 0..4 {
        let tx = x - 1.5 + k as f32;
        put(s, v(tx, 0.0, z + 3.0), F_CLAW);
    }
}

/**
 * The kitsune: a nogitsune, a fox grown old and wicked enough to cast fire and wear the shapes
 * of men; a little larger than a living fox, slender and long-legged, red-orange and darker
 * down its back, a cream bib and belly and cheeks, black stockings and black backs to its tall
 * ears; narrow slanting eyes of burning amber, a sly mouth and the points of its teeth; and its
 * one great tail raised behind it in an S, bushy, its tip white, a little flame of pale foxfire
 * floating over it.
 */
pub fn kitsune() -> Collected {
    let (w, h, d) = KITSUNE_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let cx = KITSUNE_X;

    // ---- legs: slender, the forelegs straight, the hind legs bent at the hock, black below
    for (i, out) in [-1.0f32, 1.0].into_iter().enumerate() {
        let x = cx + out * 5.5;
        s.part(1 + i as u8);
        ball(&mut s, v(x + out * 0.4, 39.0, 74.0), v(3.6, 5.6, 5.0), F_FUR);
        limb(&mut s, &[(v(x, 40.0, 74.0), 4.0), (v(x + out * 0.4, 28.0, 71.0), 3.0)], F_FUR);
        limb(&mut s, &[(v(x + out * 0.4, 28.0, 71.0), 2.7), (v(x + out * 0.3, 8.0, 73.0), 1.9), (v(x + out * 0.3, 3.0, 74.4), 1.8)], F_FUR);
        fox_paw(&mut s, x + out * 0.3, 75.4);
        let x = cx + out * 6.0;
        s.part(3 + i as u8);
        ball(&mut s, v(x, 38.0, 40.0), v(4.6, 7.4, 7.2), F_FUR);
        limb(&mut s, &[(v(x, 37.0, 40.5), 5.4), (v(x + out * 0.6, 25.0, 45.0), 3.4)], F_FUR);
        limb(&mut s, &[(v(x + out * 0.6, 25.0, 45.0), 3.0), (v(x + out * 0.6, 13.0, 34.0), 2.0)], F_FUR);
        limb(&mut s, &[(v(x + out * 0.6, 13.0, 34.0), 1.9), (v(x + out * 0.5, 3.0, 36.4), 1.8)], F_FUR);
        fox_paw(&mut s, x + out * 0.5, 37.4);
    }

    // ---- the body: a deep narrow chest, a tucked belly, the haunches
    s.part(0);
    ball(&mut s, v(cx, 36.0, 72.0), v(8.0, 9.4, 12.0), F_FUR);
    ball(&mut s, v(cx, 38.6, 55.0), v(7.0, 6.8, 14.0), F_FUR);
    ball(&mut s, v(cx, 39.5, 38.5), v(8.0, 8.4, 10.0), F_FUR);

    // ---- the tail: raised behind in an S, bushiest in the middle, streaked, its tip white and
    //      drawn to a brush; the foxfire over it
    s.part(6);
    let path = spline(&KITSUNE_TAIL.map(Vec3::from), 8);
    let n = path.len() as f32 - 1.0;
    let knots: Vec<(Vec3, f32)> = path
        .iter()
        .enumerate()
        .map(|(k, p)| {
            let t = k as f32 / n;
            let r = 3.4 + 5.4 * (t * PI * 0.9).sin().max(0.0).powf(0.6) - t * 1.4;
            (*p, r * (1.0 - ((t - 0.85) / 0.15).clamp(0.0, 1.0) * 0.55))
        })
        .collect();
    sweep(&mut s, &knots, |p, c, along| {
        let out = (p - c).normalize_or_zero();
        let (x, y, z) = (p.x as i32, p.y as i32, p.z as i32);
        let streak = (out.x * 2.6 + nz(x, (along * 30.0) as i32, 3) * 0.25).rem_euclid(1.0) < 0.16;
        if along > 0.84 {
            F_CREAM
        } else if along > 0.8 && nz(x, y, z) < 0.5 {
            F_CREAM_D
        } else if (out.dot(v(0.0, -0.3, -1.0).normalize()) > 0.45 || streak) && along > 0.2 {
            F_FUR_D
        } else if nz(x, y / 2, z) > 0.85 {
            F_FUR_L
        } else {
            F_FUR
        }
    });
    // Foxfire: a pale teardrop of flame, burning white at its heart.
    let [fx, fy, fz] = KITSUNE_FOXFIRE;
    for y in (fy - 3.0) as i32..=(fy + 6.0) as i32 {
        for z in (fz - 3.0) as i32..=(fz + 3.0) as i32 {
            for x in (fx - 3.0) as i32..=(fx + 3.0) as i32 {
                let (dx, dy, dz) = (x as f32 - fx, y as f32 - fy, z as f32 - fz);
                // A ball below, drawn up into a point.
                let r = if dy < 0.0 { 2.6 } else { 2.6 * (1.0 - dy / 5.5).max(0.0) };
                let q = (dx * dx + dz * dz).sqrt();
                if q > r || (dy < 0.0 && q * q + dy * dy > 2.6 * 2.6) {
                    continue;
                }
                let c = if q < 1.2 && dy < 2.0 {
                    FIRE_C
                } else if dy > 2.4 {
                    FIRE_D
                } else {
                    FIRE
                };
                s.set(x, y, z, c);
            }
        }
    }

    // ---- the head: a fine wedge, white-cheeked, the long muzzle, the tall ears, the narrow eyes
    s.part(5);
    limb(&mut s, &[(v(cx, 41.0, 79.0), 6.6), (v(cx, 47.0, 85.0), 6.0), (v(cx, 52.0, 89.5), 5.4)], F_FUR);
    ball(&mut s, v(cx, 56.5, 93.0), v(7.2, 6.2, 7.2), F_FUR);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 5.0, 53.6, 95.5), v(3.9, 3.6, 4.0), F_CHEEK);
    }
    limb(&mut s, &[(v(cx, 54.8, 97.0), 4.2), (v(cx, 53.4, 103.0), 3.0), (v(cx, 52.4, 108.5), 1.8)], F_FUR);
    limb(&mut s, &[(v(cx, 50.8, 96.5), 3.2), (v(cx, 50.3, 102.0), 2.2), (v(cx, 50.5, 107.0), 1.3)], F_CHEEK);
    ball(&mut s, v(cx, 52.8, 109.6), v(1.6, 1.3, 1.3), F_NOSE);
    // The lips: white below, a black line where they part, curling up at the corner in a smile.
    s.paint(|x, y, z, c| {
        let dx = (x as f32 - cx).abs();
        if c == F_FUR && z > 97 && y <= 52 && dx > 0.5 {
            return Some(F_CHEEK);
        }
        let line = (y == 51 && dx > 1.0 && dx < 3.0) || (y == 52 && (2.6..4.2).contains(&dx));
        (c != F_NOSE && (z as f32) > 97.5 && (z as f32) < 108.0 && line).then_some(F_LINE)
    });
    for side in [-1.0f32, 1.0] {
        put(&mut s, v(cx + side * 1.3, 50.8, 106.4), F_TOOTH);
        put(&mut s, v(cx + side * 1.9, 50.8, 105.0), F_TOOTH);
    }
    // Narrow eyes slanting up at their outer corners, burning amber, lined in black, the dark
    // tear-line running from each down the muzzle.
    for side in [-1.0f32, 1.0] {
        let x = |k: i32| (cx + side * k as f32) as i32;
        for (k, y) in [(2, 57), (3, 57), (4, 58), (5, 58)] {
            face(&mut s, x(k), y, d, 0, F_EYE);
        }
        for (k, y) in [(1, 57), (2, 58), (3, 58), (4, 59), (5, 59), (6, 59), (6, 58), (2, 56), (3, 56), (4, 57), (5, 57)] {
            face(&mut s, x(k), y, d, 0, F_LINE);
        }
        for (k, y) in [(1, 56), (1, 55), (2, 55)] {
            face(&mut s, x(k), y, d, 0, F_FUR_D);
        }
    }
    // The ears: tall and pointed, cream within, black behind and at the rims.
    for side in [-1.0f32, 1.0] {
        let (base, tip) = (v(cx + side * 4.0, 60.5, 92.5), v(cx + side * 7.0, 72.0, 90.6));
        let steps = 44;
        for k in 0..=steps {
            let t = k as f32 / steps as f32;
            let c = base + (tip - base) * t;
            let (rw, rd) = (3.4 * (1.0 - t) + 0.3, 1.4 * (1.0 - t) + 0.5);
            for z in (c.z - rd).floor() as i32..=(c.z + rd).ceil() as i32 {
                for x in (c.x - rw).floor() as i32..=(c.x + rw).ceil() as i32 {
                    let (dx, dz) = ((x as f32 - c.x) / rw, (z as f32 - c.z) / rd);
                    if dx * dx + dz * dz > 1.0 {
                        continue;
                    }
                    let inner = dz > 0.2 && dx.abs() < 0.62 && t < 0.82;
                    s.set(x, c.y.round() as i32, z, if inner { F_EAR_IN } else { F_BLACK });
                }
            }
        }
    }

    // ---- the coat: dark down the spine, cream at the throat, chest and belly, black stockings
    s.paint(|x, y, z, c| {
        if c != F_FUR {
            return None;
        }
        let (dx, fz, fy) = ((x as f32 - cx).abs(), z as f32, y as f32);
        // Black stockings: the forelegs from the elbow down, the hind legs from the hock.
        let fore = fz > 66.0 && fz < 80.0 && fy < 20.0 + nz(x, 1, z) * 3.0;
        let hind = fz > 30.0 && fz < 48.0 && (fy < 13.0 || (fy < 20.0 && fz < 38.0));
        if fore || hind {
            return Some(F_BLACK);
        }
        // The cream bib down the throat and chest, between the forelegs, and the belly.
        let bib = fz > 76.0 && fy < 50.0 && fy > 24.0 && dx < 4.6 && fz + fy * 0.4 > 96.0;
        if bib || (fy < 31.0 && fz > 44.0 && fz < 70.0 && dx < 5.0) {
            return Some(F_CREAM);
        }
        if fy > 42.0 && fz < 86.0 && dx < 4.0 + nz(x, y, z) * 1.5 {
            return Some(F_FUR_D);
        }
        (nz(x, y / 2, z) > 0.86).then_some(F_FUR_L)
    });
    s.paint(|x, y, z, c| (c == F_FUR && z > 88 && y > 57 && nz(x, y, z) < 0.35).then_some(F_FUR_D));
    shag(&mut s, (w, h, d), F_CREAM, F_CREAM, F_CREAM_D, 0.28, 3);
    shag(&mut s, (w, h, d), F_FUR, F_FUR, F_FUR_L, 0.12, 2);
    shag(&mut s, (w, h, d), F_FUR_D, F_FUR_D, F_FUR, 0.18, 3);
    s.finish(0.04)
}

#[cfg(test)]
mod tests {
    use super::super::voxels::Voxel;
    use super::*;

    /// A foe: its name, its model, its grid and its rig.
    type Foe = (&'static str, Collected, (i32, i32, i32), &'static [PartDef]);

    /// The four, each with its grid and its rig.
    fn cast() -> [Foe; 4] {
        [
            ("kappa", kappa(), KAPPA_SIZE, KAPPA_RIG),
            ("rokurokubi", rokurokubi(), ROKURO_SIZE, ROKURO_RIG),
            ("oni", oni(), ONI_SIZE, ONI_RIG),
            ("kitsune", kitsune(), KITSUNE_SIZE, KITSUNE_RIG),
        ]
    }

    /// The tag of the part called `name` in `rig`.
    fn tag(rig: &[PartDef], name: &str) -> u8 {
        1 + rig.iter().position(|p| p.name == name).unwrap_or_else(|| panic!("no part {name}")) as u8
    }

    #[test]
    fn every_fourth_stage_foe_stands_in_the_middle_of_its_grid_and_inside_it() {
        for (name, c, size, _) in cast() {
            assert!(c.voxels.len() > 2000, "{name}: {} voxels", c.voxels.len());
            assert_eq!((c.width, c.height, c.depth), size, "{name}");
            // Its feet on the floor, and its footprint about the grid's middle (where the rig puts its place).
            assert!(c.voxels.iter().map(|v| v.y).min().is_some_and(|y| y <= 1), "{name} stands on the floor");
            assert!(!c.voxels.iter().any(|v| v.x == 0), "{name}: a stray voxel on the grid's edge (a NaN)");
            assert!(c.voxels.iter().all(|v| v.c >> 24 != 0), "{name}: a voxel with no material");
            let floor: Vec<_> = c.voxels.iter().filter(|v| v.y <= 2).collect();
            let mx = floor.iter().map(|v| v.x as f32).sum::<f32>() / floor.len() as f32;
            assert!((mx - (size.0 - 1) as f32 / 2.0).abs() < 3.0, "{name}: footprint centred at x {mx}");
        }
    }

    #[test]
    fn every_fourth_stage_joint_lies_inside_its_grid_at_its_part() {
        for (name, c, (w, h, d), rig) in cast() {
            for def in rig {
                let [x, y, z] = def.pivot;
                let inside =
                    (0.0..=(w - 1) as f32).contains(&x) && (0.0..=(h - 1) as f32).contains(&y) && (0.0..=(d - 1) as f32).contains(&z);
                assert!(inside, "{name} {}: its pivot {:?} is outside its grid", def.name, def.pivot);
                // Its part is sculpted, and the joint is at it (within its bounds, a voxel or two to spare).
                let part: Vec<&Voxel> = c.voxels.iter().filter(|v| v.part == tag(rig, def.name)).collect();
                assert!(!part.is_empty(), "{name} {}: no voxels", def.name);
                let near = |p: f32, at: fn(&Voxel) -> i32| {
                    let (lo, hi) = (part.iter().map(|v| at(v)).min().unwrap_or(0), part.iter().map(|v| at(v)).max().unwrap_or(0));
                    (lo as f32 - 2.0..=hi as f32 + 2.0).contains(&p)
                };
                assert!(
                    near(x, |v| v.x) && near(y, |v| v.y) && near(z, |v| v.z),
                    "{name} {}: its pivot {:?} is away from it",
                    def.name,
                    def.pivot
                );
            }
        }
    }

    #[test]
    fn the_kappas_water_is_only_the_water_in_its_dish_and_the_dish_holds_without_it() {
        let c = kappa();
        let [dx, dy, dz] = KAPPA_DISH;
        let water: Vec<_> = c.voxels.iter().filter(|v| v.part == tag(KAPPA_RIG, "water")).collect();
        assert!(water.len() > 40, "{} voxels of water", water.len());
        for v in water {
            assert_eq!(v.c >> 24, mat::SLIME >> 24, "the water at {:?} is not wet", (v.x, v.y, v.z));
            let r = ((v.x as f32 - dx).powi(2) + (v.z as f32 - dz).powi(2)).sqrt();
            assert!(r < 5.5 && (v.y as f32 - dy).abs() <= 2.0, "water outside the dish at {:?}", (v.x, v.y, v.z));
        }
        // The pale rim stands above the water's surface, on the head (which keeps it when the water spills).
        let rim = c.voxels.iter().filter(|v| v.part == tag(KAPPA_RIG, "head") && v.c >> 24 == mat::IVORY >> 24 && v.y as f32 > dy);
        assert!(rim.count() > 20, "no rim round the dish");
    }

    #[test]
    fn the_rokurokubis_neck_runs_from_her_collar_to_her_jaw() {
        let c = rokurokubi();
        let ys: Vec<i32> = c.voxels.iter().filter(|v| v.part == tag(ROKURO_RIG, "neck")).map(|v| v.y).collect();
        let (lo, hi) = (ys.iter().min().copied().unwrap_or(0) as f32, ys.iter().max().copied().unwrap_or(0) as f32);
        let (collar, jaw) = (ROKURO_RIG[2].pivot[1], ROKURO_RIG[3].pivot[1]);
        assert!((lo - collar).abs() <= 2.0, "her neck starts at {lo}, not at her collar ({collar})");
        assert!((hi - jaw).abs() <= 2.0, "her neck ends at {hi}, not under her jaw ({jaw})");
    }

    #[test]
    fn the_foxfire_floats_over_the_kitsunes_tail_and_goes_with_it() {
        let c = kitsune();
        let tail = tag(KITSUNE_RIG, "tail");
        let fire: Vec<_> = c.voxels.iter().filter(|v| v.c >> 24 == mat::GLOW >> 24 && v.y as f32 > KITSUNE_FOXFIRE[1] - 4.0).collect();
        assert!(fire.len() > 10 && fire.iter().all(|v| v.part == tail), "the foxfire is not the tail's");
        let tip = c.voxels.iter().filter(|v| v.part == tail && v.c >> 24 != mat::GLOW >> 24).map(|v| v.y).max().unwrap_or(0);
        assert!(fire.iter().all(|v| v.y > tip), "the foxfire touches the tail");
    }

    /// The voxels of `c` inside the box from `lo` to `hi`, moved to its corner: a close-up for a sheet.
    fn slice(c: &Collected, lo: [i32; 3], hi: [i32; 3]) -> Collected {
        let inside = |v: &Voxel| (lo[0]..=hi[0]).contains(&v.x) && (lo[1]..=hi[1]).contains(&v.y) && (lo[2]..=hi[2]).contains(&v.z);
        let voxels =
            c.voxels.iter().filter(|v| inside(v)).map(|v| Voxel { x: v.x - lo[0], y: v.y - lo[1], z: v.z - lo[2], ..*v }).collect();
        Collected { voxels, width: hi[0] - lo[0] + 1, depth: hi[2] - lo[2] + 1, height: hi[1] - lo[1] + 1 }
    }

    /// The kappa with the water gone from its dish, as the game shows it once it has bowed.
    fn spilled(c: &Collected) -> Collected {
        let water = tag(KAPPA_RIG, "water");
        Collected {
            voxels: c.voxels.iter().filter(|v| v.part != water).copied().collect(),
            width: c.width,
            depth: c.depth,
            height: c.height,
        }
    }

    /// Every model's sheet (front, side, back / top, two three-quarter views), close-ups of their
    /// heads, the rokurokubi's body and the kappa's dish spilled, into VH_PREVIEW_DIR.
    #[test]
    #[ignore]
    fn preview_stage_four() {
        let [(_, k, ..), (_, r, ..), (_, o, ..), (_, f, ..)] = cast();
        let close = [
            ("kappa_head", slice(&k, [0, 46, 0], [54, 79, 55])),
            ("kappa_spilled", spilled(&slice(&k, [0, 46, 0], [54, 79, 55]))),
            ("rokurokubi_head", slice(&r, [0, 108, 0], [48, 141, 44])),
            ("rokurokubi_body", slice(&r, [0, 0, 0], [48, 48, 44])),
            ("oni_head", slice(&o, [10, 88, 0], [80, 137, 56])),
            ("kitsune_head", slice(&f, [0, 36, 76], [44, 75, 116])),
        ];
        for (name, c) in [("kappa", k), ("rokurokubi", r), ("oni", o), ("kitsune", f)].into_iter().chain(close) {
            println!("{}", super::super::preview::sheet(&c, name));
        }
    }
}
