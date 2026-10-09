//! The Krampus fountain in the middle of Dungeon I's garden, the first thing a player sees in play,
//! in the crypt's dressed stone (RENDERING.md, "The Garden, at the Cisterns' bar": the fountain): an
//! octagonal basin of chunky charcoal and umber blocks on a pale footing, dark and wet low down, under
//! a coping of pale slabs with a copper lip gone green, verdigris run down the stone below it; four
//! copper nozzles on the coping; in the middle a pedestal of drums carrying a shallow bowl with a
//! copper rim, brimming over all round in a veil of water; and on a pale plinth in the bowl a
//! Krampus cast in bronze and weathered green. He has cloven hooves, a shaggy coat, a wicker basket
//! of switches on his back, a bundle of birch rods raised in one fist and a chain hanging from the
//! other, ram's horns and ember eyes, and water runs off his long lolling tongue; the rain has left
//! him verdigris, pale where it runs and dark in the hollows, the bronze rubbed bright on his horns,
//! hooves and knuckles. Jets arc in from the coping, a lamp under the water lights him from below,
//! and four spouts under the bridges feed the channels. The stone and bronze are static; the water
//! is drawn each frame, rings spreading where it falls in (`water` in the shader).

use super::*;
use std::f32::consts::TAU;

use super::crypt::pal::{CHARCOAL, COPPER, PALE_STONE, UMBER, VERDIGRIS};
use super::crypt::{block, block_m, jit, SHADE_BRONZE, SHADE_FLAG, SHADE_STONE};
use crate::logic::garden::{GardenPlan, RiverTile};
use crate::render::characters::{mat, Sculpt};

mod pal {
    use super::mat;
    pub const PAVER_D: u32 = 0x7c7666;
    // The statue as it was sculpted (the casting's parts, told apart by colour), weathered after.
    pub const BRONZE: u32 = 0x9a7446 | mat::METAL;
    pub const BRONZE_D: u32 = 0x76583a | mat::METAL;
    pub const COAT: u32 = 0x6a5232 | mat::METAL;
    pub const COAT_L: u32 = 0x86683e | mat::METAL;
    pub const GILT: u32 = 0xecbc4c | mat::GOLD;
    pub const GILT_D: u32 = 0xbc8c32 | mat::GOLD;
    pub const TONGUE: u32 = 0xa8302c | mat::LEATHER;
    pub const FANG: u32 = 0xeae2cc | mat::IVORY;
    pub const MAW: u32 = 0x1a120e;
    pub const EYE: u32 = 0xff8a30;
    pub const WICKER: u32 = 0x8e6c3c | mat::WOOD;
    pub const WICKER_D: u32 = 0x6e5230 | mat::WOOD;
    pub const CHAIN: u32 = 0x8a9098 | mat::METAL;
    pub const STRAP: u32 = 0x5a3e24 | mat::LEATHER;
    /// Verdigris, the crust the rain leaves on bronze (shaded as the vaults' stone, `voxel_surface`
    /// kind 13: matte, mottled, pitted): its body, pale where the rain runs and on what faces up,
    /// dark in the hollows; and the bronze rubbed bright on what stands out.
    const CRUST: u32 = 13 << 24;
    pub const VERD: u32 = 0x3c584f | CRUST;
    pub const VERD_L: u32 = 0x5c7e70 | CRUST;
    pub const VERD_D: u32 = 0x1f2f2b | CRUST;
    pub const BRONZE_DARK: u32 = 0x4a3826 | mat::GOLD;
    pub const WORN: u32 = 0x8a6a40 | mat::GOLD;
    pub const WORN_D: u32 = 0x5e4630 | mat::GOLD;
    pub const IRON_D: u32 = 0x3a3634 | mat::METAL;
}
use crate::render::voxels::Collected;
use pal::*;

/// Bronze voxels of the statue.
const STATUE_VOXEL: f32 = 0.02;
/// Heights above the floor the fountain stands on: the basin's water (a wet band of its wall over
/// it), the top of its coping, the bowl's rim and water, the plinth's top in the bowl.
const BASIN_WATER: f32 = 0.235;
const COPING: f32 = 0.33;
const BOWL_RIM: f32 = 0.775;
const BOWL_WATER: f32 = 0.748;
const BOWL_R: f32 = 0.262;
const PLINTH: f32 = 0.82;
/// The basin's outer face from its middle (an octagon's apothem).
const BASIN_R: f32 = 0.465;
/// The statue's foot: his hooves (3 voxels up his grid) on the plinth.
const STATUE_BASE: f32 = PLINTH - 3.0 * STATUE_VOXEL;
/// Where the water leaves the tip of his tongue, from the statue's foot (he faces -z).
const TONGUE_TIP: Vec3 = Vec3::new(0.0, 0.8, -0.33);

fn nz(x: i32, y: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9e37_79b1) ^ (y as u32).wrapping_mul(0x85eb_ca77) ^ (z as u32).wrapping_mul(0xc2b2_ae3d);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    (h & 0xffff) as f32 / 65535.0
}

/// Fills a tapered capsule from `a` (radius `ra`) to `b` (radius `rb`), in voxel units.
fn capsule(s: &mut Sculpt, a: Vec3, b: Vec3, ra: f32, rb: f32, color: impl Fn(f32) -> u32) {
    let r = ra.max(rb) + 1.0;
    let (lo, hi) = (a.min(b) - Vec3::splat(r), a.max(b) + Vec3::splat(r));
    let ab = b - a;
    let len2 = ab.length_squared().max(1e-6);
    for y in lo.y.floor() as i32..=hi.y.ceil() as i32 {
        for z in lo.z.floor() as i32..=hi.z.ceil() as i32 {
            for x in lo.x.floor() as i32..=hi.x.ceil() as i32 {
                let p = Vec3::new(x as f32, y as f32, z as f32);
                let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
                if p.distance(a + ab * t) <= ra + (rb - ra) * t {
                    s.set(x, y, z, color(t));
                }
            }
        }
    }
}

/// A limb, tail or horn along a path of (point, radius) knots.
fn limb(s: &mut Sculpt, knots: &[(Vec3, f32)], c: u32) {
    for w in knots.windows(2) {
        capsule(s, w[0].0, w[1].0, w[0].1, w[1].1, |_| c);
    }
}

/// A ram's horn: like a limb, but ringed with ridges along its length.
fn horn(s: &mut Sculpt, knots: &[(Vec3, f32)]) {
    let mut run = 0.0;
    for w in knots.windows(2) {
        let len = w[0].0.distance(w[1].0);
        capsule(s, w[0].0, w[1].0, w[0].1, w[1].1, |t| if ((run + t * len) / 1.3) as i32 % 2 == 0 { GILT } else { GILT_D });
        run += len;
    }
}

fn ball(s: &mut Sculpt, c: Vec3, r: Vec3, color: u32) {
    s.ellipsoid((c.x, c.y, c.z), (r.x, r.y, r.z), 0, color);
}

/// Shaggy coat: clumps of fur standing out of the surface and hanging down from it.
fn shag(s: &mut Sculpt, dims: (i32, i32, i32), coat: u32, tuft: u32) {
    let (w, h, d) = dims;
    let mut add = vec![];
    for y in 1..h {
        for z in 0..d {
            for x in 0..w {
                if s.get(x, y, z) != Some(coat) {
                    continue;
                }
                for (dx, dy, dz) in [(1, 0, 0), (-1, 0, 0), (0, 0, 1), (0, 0, -1), (0, -1, 0)] {
                    if s.get(x + dx, y + dy, z + dz).is_none() && nz(x * 3 + dx, y * 5 + dy, z * 7 + dz) > 0.76 {
                        add.push((x + dx, y + dy, z + dz));
                        if nz(x, y - 1, z + 11) > 0.45 {
                            add.push((x + dx, y + dy - 1, z + dz));
                        }
                    }
                }
            }
        }
    }
    for (x, y, z) in add {
        if s.get(x, y, z).is_none() {
            s.set(x, y, z, tuft);
        }
    }
}

/**
 * The Krampus statue, facing +z: 44 x 64 x 34 voxels of bronze weathered green (`weather`).
 * Body coordinates below run from his hooves, 3 voxels up the grid (on the plinth's top).
 */
pub fn krampus() -> Collected {
    const W: i32 = 44;
    const H: i32 = 64;
    const D: i32 = 34;
    const G: i32 = 3;
    let mut s = Sculpt::new(W, H, D);
    let cx = 21.5f32;
    let v = |x: f32, y: f32, z: f32| Vec3::new(x, y + G as f32, z);

    // ---- legs: gilt cloven hooves, fetlock tufts, thin cannons back to the hock, shaggy thighs
    for side in [-1.0f32, 1.0] {
        let x = cx + side * 4.5;
        ball(&mut s, v(x, 1.2, 20.0), Vec3::new(2.3, 1.8, 3.0), GILT_D);
        for y in G..G + 3 {
            for z in 21..25 {
                s.clear(x as i32, y, z); // the cleft
            }
        }
        ball(&mut s, v(x, 3.7, 19.0), Vec3::new(2.3, 1.3, 2.3), COAT);
        limb(&mut s, &[(v(x, 3.0, 19.5), 1.5), (v(x, 11.0, 15.5), 1.3)], BRONZE);
        ball(&mut s, v(x, 11.5, 15.2), Vec3::splat(2.0), BRONZE);
        limb(&mut s, &[(v(x, 12.0, 15.0), 2.6), (v(x + side * 0.6, 18.0, 17.5), 3.8), (v(x + side * 0.4, 23.0, 18.2), 4.2)], COAT);
    }

    // ---- hips, belly, chest and shoulders, hunched forward
    ball(&mut s, v(cx, 24.0, 16.5), Vec3::new(8.0, 4.2, 5.8), COAT);
    ball(&mut s, v(cx, 29.0, 17.2), Vec3::new(7.4, 6.0, 6.0), COAT);
    ball(&mut s, v(cx, 34.5, 18.8), Vec3::new(8.6, 5.5, 6.0), COAT);
    ball(&mut s, v(cx, 38.3, 19.6), Vec3::new(10.2, 3.4, 5.2), COAT);
    // A tail with a tuft.
    limb(
        &mut s,
        &[(v(cx, 23.5, 11.5), 1.2), (v(cx + 2.0, 20.5, 8.5), 1.0), (v(cx + 5.0, 21.0, 5.5), 0.8), (v(cx + 6.5, 24.5, 4.5), 0.6)],
        COAT,
    );
    ball(&mut s, v(cx + 6.8, 26.0, 4.5), Vec3::splat(1.8), COAT_L);

    // ---- a wicker basket of switches strapped to his back
    let (bx0, bx1) = ((cx - 6.5) as i32, (cx + 6.5) as i32);
    for y in 27..=41 {
        for z in 4..=11 {
            for x in bx0..=bx1 {
                let wall = z == 4 || z == 11 || x == bx0 || x == bx1 || y == 27;
                if wall {
                    let band = y == 27 || y == 34 || y == 41;
                    s.set(
                        x,
                        y + G,
                        z,
                        if band {
                            BRONZE_D
                        } else if (x + y + z) % 2 == 0 {
                            WICKER
                        } else {
                            WICKER_D
                        },
                    );
                }
            }
        }
    }
    for i in 0..5 {
        let fi = i as f32;
        let base = v(cx - 4.0 + fi * 2.0, 38.0, 7.5);
        let tip = v(cx - 4.6 + fi * 2.3, 47.5 + (i % 2) as f32, 5.0 + (i % 3) as f32);
        capsule(&mut s, base, tip, 0.55, 0.35, |_| if i % 2 == 0 { GILT_D } else { GILT });
    }
    for side in [-1.0f32, 1.0] {
        limb(
            &mut s,
            &[(v(cx + side * 4.5, 41.0, 11.0), 0.8), (v(cx + side * 5.5, 41.8, 17.0), 0.8), (v(cx + side * 5.5, 36.0, 25.2), 0.8)],
            STRAP,
        );
    }

    // ---- a chain slung round his waist, with bells
    for i in 0..56 {
        let a = i as f32 / 56.0 * TAU;
        if i % 3 == 2 {
            continue;
        }
        let (x, z) = (cx + a.cos() * 8.4, 17.0 + a.sin() * 6.4);
        let y = 26.0 + (a * 2.0).sin() * 0.6;
        s.set(x.round() as i32, (y + G as f32).round() as i32, z.round() as i32, CHAIN);
    }
    for (dx, dz) in [(-3.5f32, 6.1f32), (3.5, 6.1), (0.0, 6.6)] {
        ball(&mut s, v(cx + dx, 23.6, 17.0 + dz), Vec3::new(1.3, 1.5, 1.3), GILT);
        s.set((cx + dx).round() as i32, 21 + G, (17.0 + dz).round() as i32, GILT_D);
    }

    // ---- neck and head, thrust forward
    limb(&mut s, &[(v(cx, 38.0, 20.0), 3.6), (v(cx, 41.5, 22.5), 3.2)], COAT);
    ball(&mut s, v(cx, 44.0, 19.8), Vec3::new(5.2, 5.5, 3.8), COAT); // the mane behind his head
    ball(&mut s, v(cx, 45.5, 23.2), Vec3::new(4.6, 4.2, 4.4), BRONZE); // skull
    ball(&mut s, v(cx, 43.6, 27.0), Vec3::new(3.0, 2.5, 3.3), BRONZE); // muzzle
    ball(&mut s, v(cx, 41.2, 26.4), Vec3::new(2.8, 1.3, 3.1), BRONZE_D); // the jaw, hanging open
    for z in 25..=30 {
        for x in (cx - 1.5) as i32..=(cx + 1.5) as i32 {
            s.set(x, 42 + G, z, MAW);
        }
    }
    for side in [-1.0f32, 1.0] {
        s.set((cx + side * 1.5) as i32, 42 + G, 30, FANG);
        s.set((cx + side * 1.5) as i32, 42 + G, 29, FANG);
        s.set((cx + side * 0.5) as i32, 44 + G, 30, MAW); // nostrils
    }
    // The long tongue lolling out and down; the water runs off its tip.
    limb(&mut s, &[(v(cx, 42.0, 28.5), 1.3), (v(cx, 41.0, 31.0), 1.1), (v(cx + 0.5, 38.5, 32.0), 0.9), (v(cx, 36.0, 32.4), 0.7)], TONGUE);
    ball(&mut s, v(cx, 47.6, 26.0), Vec3::new(4.3, 1.0, 1.5), BRONZE_D); // a heavy brow
    for side in [-1.0f32, 1.0] {
        let x = (cx + side * 2.5) as i32;
        for z in 25..=27 {
            s.set(x, 46 + G, z, EYE);
        }
        // Goat's ears, sticking out.
        limb(
            &mut s,
            &[(v(cx + side * 4.0, 46.5, 22.0), 1.4), (v(cx + side * 6.5, 47.8, 21.0), 0.9), (v(cx + side * 8.5, 49.0, 20.0), 0.4)],
            BRONZE,
        );
        // Ram's horns: up, back and out, curling down and forward at the tips.
        let k = |dx: f32, y: f32, z: f32, r: f32| (v(cx + side * dx, y, z), r);
        horn(
            &mut s,
            &[
                k(2.5, 49.0, 23.0, 1.9),
                k(4.5, 52.5, 21.0, 1.7),
                k(7.5, 55.5, 18.5, 1.45),
                k(10.5, 57.0, 16.8, 1.2),
                k(13.3, 56.2, 16.5, 1.0),
                k(15.2, 53.5, 17.8, 0.8),
                k(15.4, 50.8, 20.0, 0.6),
                k(14.2, 49.4, 21.8, 0.4),
            ],
        );
    }
    limb(&mut s, &[(v(cx, 40.2, 27.0), 1.8), (v(cx, 37.5, 26.6), 1.2), (v(cx, 35.5, 25.8), 0.6)], COAT_L); // goatee

    // ---- one arm raised with a bundle of birch rods
    limb(&mut s, &[(v(cx + 8.5, 38.0, 19.5), 2.8), (v(cx + 12.0, 43.0, 21.5), 2.4)], COAT);
    ball(&mut s, v(cx + 12.0, 43.0, 21.5), Vec3::splat(2.6), COAT_L);
    limb(&mut s, &[(v(cx + 12.0, 43.0, 21.5), 2.1), (v(cx + 12.5, 48.5, 24.5), 1.7)], BRONZE);
    limb(&mut s, &[(v(cx + 12.3, 46.8, 23.6), 2.2), (v(cx + 12.4, 47.8, 24.1), 2.2)], CHAIN); // a shackle
    ball(&mut s, v(cx + 12.6, 50.0, 25.2), Vec3::splat(1.9), BRONZE);
    let fist = v(cx + 12.6, 50.3, 25.2);
    for (i, d) in [
        (0.15f32, 1.0f32, -0.35f32),
        (0.35, 1.0, -0.2),
        (-0.05, 1.0, -0.15),
        (0.25, 1.0, 0.05),
        (0.5, 0.95, -0.4),
        (0.05, 1.0, -0.5),
        (0.4, 1.0, 0.15),
    ]
    .iter()
    .enumerate()
    {
        let dir = Vec3::new(d.0, d.1, d.2).normalize();
        let base = fist + Vec3::new((i as f32 - 3.0) * 0.3, -1.2, 0.0);
        let len = 6.5 + (i % 3) as f32;
        capsule(&mut s, base, base + dir * len, 0.55, 0.35, |_| if i % 2 == 0 { GILT } else { GILT_D });
    }
    for k in 0..3 {
        // Claws round the rods.
        let fk = k as f32;
        s.set((cx + 11.5 + fk) as i32, 49 + G, 27, GILT_D);
    }

    // ---- the other hanging, a chain dangling from its fist
    limb(&mut s, &[(v(cx - 8.5, 38.0, 19.5), 2.8), (v(cx - 11.5, 32.5, 21.0), 2.4)], COAT);
    ball(&mut s, v(cx - 11.5, 32.5, 21.0), Vec3::splat(2.6), COAT_L);
    limb(&mut s, &[(v(cx - 11.5, 32.5, 21.0), 2.1), (v(cx - 12.0, 27.0, 23.5), 1.7)], BRONZE);
    limb(&mut s, &[(v(cx - 11.8, 28.6, 22.8), 2.2), (v(cx - 11.9, 27.8, 23.2), 2.2)], CHAIN);
    ball(&mut s, v(cx - 12.0, 25.6, 24.0), Vec3::splat(1.9), BRONZE);
    for k in 0..3 {
        let fk = k as f32;
        s.set((cx - 13.5 + fk) as i32, 24 + G, 26, GILT_D);
    }
    for k in 0..22 {
        let fk = k as f32;
        let y = 23.5 - fk;
        if y < 0.5 {
            break;
        }
        let x = cx - 12.0 + (fk * 0.2).sin() * 0.9;
        let z = 24.4 + fk * 0.14;
        let (xi, yi, zi) = (x.round() as i32, (y + G as f32).round() as i32, z.round() as i32);
        // Links at right angles to each other.
        if k % 2 == 0 {
            for dx in -1..=1 {
                s.set(xi + dx, yi, zi, CHAIN);
            }
        } else {
            for dz in -1..=1 {
                s.set(xi, yi, zi + dz, CHAIN);
            }
        }
    }

    shag(&mut s, (W, H, D), COAT, COAT_L);
    weather(&mut s, (W, H, D));
    s.finish(0.02)
}

/// A block of `size` (along, up, out) on side `a` of an octagon round `c` (its middle `r` out),
/// `u` along the side from its middle, `y` up (its middle), `askew` turned off square.
#[allow(clippy::too_many_arguments)]
fn on_side(out: &mut Vec<Instance>, c: Vec3, a: f32, r: f32, u: f32, y: f32, size: Vec3, askew: f32, col: Vec3, shade: f32) {
    let (n, t) = (Vec3::new(a.cos(), 0.0, a.sin()), Vec3::new(-a.sin(), 0.0, a.cos()));
    let at = c + n * r + t * u + Vec3::Y * y;
    block_m(out, Mat4::from_scale_rotation_translation(size, Quat::from_rotation_y(-a - PI / 2.0 + askew), at), col, shade);
}

/**
 * The fountain's stonework round its middle `c` (on the floor): the basin, its coping and copper
 * lip, the nozzles of the jets, the pedestal, the bowl and the plinth in it.
 */
fn stonework(out: &mut Vec<Instance>, c: Vec3, (hx, hy): (f64, f64)) {
    let side = 2.0 * BASIN_R * (PI / 8.0).tan();
    let pale = |k: f64| jit(PALE_STONE[(k * 7.0) as usize % 2], k.fract(), 0.06) * 0.8;
    // The basin's floor under the water, dark, and its pale footing.
    for turn in [0.0, PI / 4.0] {
        block(out, c + Vec3::Y * 0.05, Vec3::new(BASIN_R * 1.6, 0.1, BASIN_R * 1.6), turn, jit(CHARCOAL[1], 0.5, 0.0) * 0.45, SHADE_STONE);
    }
    for k in 0..8 {
        let a = k as f32 * PI / 4.0 + PI / 8.0;
        let h = hash(hx + k as f64 * 1.7, hy);
        on_side(
            out,
            c,
            a,
            BASIN_R + 0.005,
            0.0,
            0.025,
            Vec3::new(side + 0.06, 0.05, 0.1),
            (h as f32 - 0.5) * 0.02,
            pale(h) * 0.85,
            SHADE_STONE,
        );
    }
    // The wall: two courses of chunky blocks each its own depth out, a touch askew, now and then umber
    // or green with verdigris, the lower course dark and slick with the wet.
    let courses = [(0.05f32, 0.155f32), (0.155, 0.27)];
    for k in 0..8 {
        let a = k as f32 * PI / 4.0 + PI / 8.0;
        for (ci, &(y0, y1)) in courses.iter().enumerate() {
            let mut u = -side / 2.0 - 0.02 - if ci == 1 { 0.07 } else { 0.0 };
            let mut j = 0;
            while u < side / 2.0 + 0.02 {
                let hk = hash(hx * 7.0 + k as f64 * 3.1 + j as f64 * 1.3, hy * 3.0 + ci as f64 * 5.0);
                let len = 0.13 + hk as f32 * 0.09;
                let (u0, u1) = (u.max(-side / 2.0 - 0.02), (u + len).min(side / 2.0 + 0.02));
                let roll = hash(hk * 31.0, hx + k as f64);
                let col = if roll < 0.08 {
                    jit(VERDIGRIS, hk, 0.1)
                } else if roll < 0.22 {
                    jit(UMBER[(hk * 2.0) as usize % 2], hk, 0.1)
                } else {
                    jit(CHARCOAL[(hk * 3.0) as usize % 3], hk, 0.12)
                };
                let col = if ci == 0 { col * 0.55 + lin(0x1c2a24) * 0.3 } else { col };
                let depth = 0.1 + hash(hk * 13.0, hy) as f32 * 0.03;
                let chip = if (hk * 29.0).fract() < 0.25 { 0.012 } else { 0.0 };
                if u1 - u0 > 0.03 {
                    on_side(
                        out,
                        c,
                        a,
                        BASIN_R - depth / 2.0,
                        (u0 + u1) / 2.0,
                        (y0 + y1) / 2.0 - chip / 2.0,
                        Vec3::new(u1 - u0 - 0.012, y1 - y0 - 0.01 - chip, depth),
                        ((hk * 41.0).fract() as f32 - 0.5) * 0.05,
                        col,
                        SHADE_STONE,
                    );
                }
                u += len;
                j += 1;
            }
        }
        // Green run down the stone under the lip, inside and out, here and there.
        for m in 0..3 {
            let hm = hash(hx + k as f64 * 5.3 + m as f64, hy * 1.9);
            if hm < 0.55 {
                let along = (hm as f32 / 0.55 - 0.5) * side * 0.8;
                let len = 0.05 + hm as f32 * 0.18;
                let r = if m == 2 { BASIN_R + 0.004 } else { BASIN_R - 0.13 };
                on_side(
                    out,
                    c,
                    a,
                    r,
                    along,
                    COPING - 0.065 - len / 2.0,
                    Vec3::new(0.022, len, 0.008),
                    0.0,
                    jit(VERDIGRIS, hm, 0.12) * 1.15,
                    SHADE_STONE,
                );
            }
        }
        // The coping: two pale slabs a side, overhanging the wall, a touch askew, wet on top; its
        // copper lip along the inner edge, green in its seams.
        for (m, u) in [-side / 4.0, side / 4.0].into_iter().enumerate() {
            let h = hash(hx * 3.3 + k as f64 * 2.0 + m as f64, hy * 1.7);
            on_side(
                out,
                c,
                a,
                BASIN_R - 0.035,
                u,
                COPING - 0.03,
                Vec3::new(side / 2.0 + 0.012, 0.06, 0.15),
                (h as f32 - 0.5) * 0.035,
                pale(h) * (0.95 + h as f32 * 0.1),
                SHADE_FLAG,
            );
            let copper = if hash(h * 9.0, hx) < 0.45 { jit(VERDIGRIS, h, 0.1) * 1.15 } else { jit(COPPER, h, 0.08) * 0.85 };
            on_side(out, c, a, BASIN_R - 0.105, u, COPING + 0.004, Vec3::new(side / 2.0 + 0.01, 0.018, 0.026), 0.0, copper, SHADE_BRONZE);
        }
    }
    // The jets' nozzles on the coping: a pale boss each, and a copper pipe out of it.
    for k in 0..4 {
        let a = k as f32 * PI / 2.0 + PI / 2.0;
        let n = Vec3::new(a.cos(), 0.0, a.sin());
        block(out, c + n * 0.415 + Vec3::Y * (COPING + 0.025), Vec3::new(0.075, 0.05, 0.075), -a, pale(k as f64 * 0.37 + 0.2), SHADE_STONE);
        block(
            out,
            c + n * 0.39 + Vec3::Y * (COPING + 0.055),
            Vec3::new(0.03, 0.03, 0.03),
            -a,
            jit(VERDIGRIS, 0.5, 0.0) * 1.2,
            SHADE_BRONZE,
        );
    }
    // The pedestal: a pale foot out of the water, drums of dark stone and umber, a pale capital.
    block(out, c + Vec3::Y * 0.19, Vec3::new(0.21, 0.1, 0.21), PI / 4.0, pale(0.31) * 0.85, SHADE_STONE);
    block(out, c + Vec3::Y * 0.27, Vec3::new(0.17, 0.06, 0.17), 0.0, pale(0.62) * 0.8, SHADE_STONE);
    for (k, (y0, y1)) in [(0.3f32, 0.4f32), (0.4, 0.5), (0.5, 0.58)].into_iter().enumerate() {
        let col = if k == 1 { jit(UMBER[0], 0.4, 0.06) } else { jit(CHARCOAL[k], 0.3 + k as f64 * 0.2, 0.08) };
        block(out, c + Vec3::Y * ((y0 + y1) / 2.0), Vec3::new(0.12, y1 - y0 - 0.008, 0.12), k as f32 * 0.05, col, SHADE_STONE);
    }
    block(out, c + Vec3::Y * 0.6, Vec3::new(0.17, 0.04, 0.17), 0.0, pale(0.83) * 0.82, SHADE_STONE);
    // The bowl: its underside in widening tiers, a rim of pale blocks, a copper lip on it.
    for (k, (y, r)) in [(0.635f32, 0.1f32), (0.67, 0.155), (0.705, 0.205), (0.73, 0.235)].into_iter().enumerate() {
        for turn in [0.0, PI / 4.0] {
            block(
                out,
                c + Vec3::Y * y,
                Vec3::new(r * 1.62, 0.036, r * 1.62),
                turn + k as f32 * 0.2,
                jit(CHARCOAL[k % 3], 0.5, 0.0) * 0.8,
                SHADE_STONE,
            );
        }
    }
    for k in 0..16 {
        let a = k as f32 / 16.0 * TAU;
        let h = hash(hx + k as f64 * 0.7, hy * 2.0);
        let along = TAU * BOWL_R / 16.0 + 0.012;
        on_side(out, c, a, BOWL_R - 0.02, 0.0, BOWL_RIM - 0.03, Vec3::new(along, 0.05, 0.045), 0.0, pale(h), SHADE_STONE);
        let copper = if h < 0.4 { jit(VERDIGRIS, h, 0.1) * 1.2 } else { jit(COPPER, h, 0.08) * 0.9 };
        on_side(out, c, a, BOWL_R - 0.02, 0.0, BOWL_RIM + 0.004, Vec3::new(along, 0.012, 0.03), 0.0, copper, SHADE_BRONZE);
    }
    // The plinth in the bowl: a pale block on a moulded foot under a cornice, green run down it.
    block(out, c + Vec3::Y * (BOWL_WATER - 0.01), Vec3::new(0.24, 0.04, 0.22), 0.0, pale(0.15) * 0.8, SHADE_STONE);
    block(out, c + Vec3::Y * (PLINTH - 0.045), Vec3::new(0.2, 0.07, 0.18), 0.0, pale(0.45) * 0.9, SHADE_STONE);
    block(out, c + Vec3::Y * (PLINTH - 0.006), Vec3::new(0.235, 0.022, 0.215), 0.0, pale(0.75), SHADE_FLAG);
    block(out, c + Vec3::new(0.0, PLINTH - 0.05, -0.091), Vec3::new(0.08, 0.06, 0.004), 0.0, jit(VERDIGRIS, 0.5, 0.1), SHADE_STONE);
}

/**
 * The bronze weathered, as rain and hands leave a statue out of doors: verdigris all over, paler
 * on what faces the sky and in streaks where the water runs down him, dark in the hollows of his
 * coat and between his limbs; the metal rubbed back to bronze on what stands out (his horns'
 * ridges, his hooves, his knuckles, the rods' ends, his brow and muzzle). His tongue, fangs and
 * maw are cast too; his ember eyes stay lit; the chain is iron.
 */
fn weather(s: &mut Sculpt, (w, h, d): (i32, i32, i32)) {
    let mut out = vec![];
    for y in 0..h {
        for z in 0..d {
            for x in 0..w {
                let Some(c) = s.get(x, y, z) else { continue };
                if c == EYE || c == MAW {
                    continue;
                }
                if c == CHAIN {
                    out.push((x, y, z, IRON_D));
                    continue;
                }
                // How shut in it is (its 26 neighbours filled), and whether rain falls on it.
                let mut near = 0;
                for (dx, dy, dz) in (-1..=1).flat_map(|a| (-1..=1).flat_map(move |b| (-1..=1).map(move |c| (a, b, c)))) {
                    near += s.get(x + dx, y + dy, z + dz).is_some() as i32;
                }
                let sky = s.get(x, y + 1, z).is_none() && s.get(x, y + 2, z).is_none();
                // (Patches a few voxels across, not a speckle: the crust grows in blooms.)
                let n = nz(x / 3, y / 3, z / 3) * 0.75 + nz(x, y, z) * 0.25;
                let run = nz(x / 2, 7, z / 2) > 0.7 && z > 18;
                let flat = near >= 14;
                let col = if matches!(c, GILT | GILT_D) {
                    // His horns, hooves, rods and bells rubbed back to the bronze, the horns' rings
                    // kept, green only in a hollow or where the rain sits on them.
                    if near >= 20 || (sky && flat && n > 0.7) {
                        VERD
                    } else if c == GILT {
                        WORN
                    } else {
                        WORN_D
                    }
                } else if matches!(c, TONGUE | FANG) || (matches!(c, BRONZE | BRONZE_D) && near <= 14) {
                    // His tongue and fangs, his knuckles, brow and muzzle: bronze, darker.
                    if n > 0.5 {
                        WORN_D
                    } else {
                        BRONZE_DARK
                    }
                } else if near >= 21 || n < 0.24 {
                    VERD_D
                } else if (sky && flat && n > 0.35) || (run && n > 0.45) {
                    VERD_L
                } else {
                    VERD
                };
                out.push((x, y, z, col));
            }
        }
    }
    for (x, y, z, c) in out {
        s.set(x, y, z, c);
    }
}

/// Pushes a sculpt's voxels as static cubes, each shaded by its own material; eyes glow.
fn push_statue(out: &mut Vec<Instance>, c: &Collected, pos: Vec3, rot_y: f32) {
    let q = Quat::from_rotation_y(rot_y);
    let (ox, oz) = ((c.width - 1) as f32 / 2.0, (c.depth - 1) as f32 / 2.0);
    for v in &c.voxels {
        let off = Vec3::new((v.x as f32 - ox) * STATUE_VOXEL, (v.y as f32 + 0.5) * STATUE_VOXEL, (v.z as f32 - oz) * STATUE_VOXEL);
        let m = Mat4::from_scale_rotation_translation(Vec3::splat(STATUE_VOXEL), q, pos + q * off);
        let shade = if v.c >> 24 == 0 { SHADE_LIT } else { 5.0 + (v.c >> 24) as f32 };
        // Ember eyes (plain, bright orange: the color jitter leaves them near EYE).
        let (r, g, b) = ((v.c >> 16) & 255, (v.c >> 8) & 255, v.c & 255);
        let eye = v.c >> 24 == 0 && r > 220 && (100..170).contains(&g) && b < 80;
        let glow = if eye { lin(0xff6a1a) * 3.0 } else { Vec3::ZERO };
        out.push(Instance::new(m, lin(v.c), 1.0, glow, shade));
    }
}

/// The fountain as it stands in the level, for the water drawn each frame.
#[derive(Clone, Debug)]
pub(super) struct FountainView {
    /// The middle of its tile, at the floor it stands on.
    pub base: Vec3,
    /// Where each channel is fed: the spout's lip, the water it pours into, and the way it runs.
    pub outlets: Vec<(Vec3, f32, Vec2)>,
    /// The glass of each lantern at the plaza's corners.
    pub lanterns: Vec<Vec3>,
}

impl WorldView {
    /// The fountain's stone and bronze, the paving round it, and the lamp under its water.
    pub(super) fn build_fountain(&mut self, grid: &Grid, f: RiverTile, plan: &GardenPlan) {
        let (xf, zf) = (f.p.x as f32, f.p.y as f32);
        let floor = f.level as f32 * STEP;
        let base = Vec3::new(xf, floor, zf);
        let out = &mut self.static_cubes;
        // Solid ground under it, paved.
        out.push(Instance::new(
            Mat4::from_scale_rotation_translation(Vec3::new(1.0, floor + 1.2, 1.0), Quat::IDENTITY, Vec3::new(xf, (floor - 1.2) / 2.0, zf)),
            lin(PAVER_D),
            1.0,
            Vec3::ZERO,
            SHADE_LIT,
        ));
        let start = out.len();
        stonework(out, base, (xf as f64, zf as f64));
        // Krampus faces north, up the channel toward the gate.
        let statue = krampus();
        push_statue(out, &statue, base + Vec3::new(0.0, STATUE_BASE, 0.0), PI);
        let len = out.len() - start;
        let shades: Vec<f32> = out[start..].iter().map(|i| i.emissive[3]).collect();
        self.authored.push((start, len));
        // Like a temple, it fades when it stands between the camera and Ragnar.
        self.trees.push(Tree { range: (start, len), base, radius: 0.5, height: 2.1, fade: 1.0, alive: true, shades: Some(shades) });

        // Flagstones on the path tiles at its corners, laid in rings round it, and at each outer
        // corner of the plaza an amber lantern on an iron post.
        let mut lanterns = vec![];
        for (dx, dz) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
            let (x, y) = (f.p.x + dx, f.p.y + dz);
            if !grid.is_walkable(x, y, false) || grid.is_bridge(x, y) || grid.is_ramp(x, y) {
                continue;
            }
            let top = grid.elevation(x, y) as f32 * STEP;
            let post = Vec3::new(x as f32 + dx as f32 * 0.38, top, y as f32 + dz as f32 * 0.38);
            let start = self.static_cubes.len();
            lanterns.push(garden_beds::lamp_post(&mut self.static_cubes, post, Vec3::new(dx as f32, 0.0, dz as f32), 0.95));
            for i in 0..3 {
                for j in 0..3 {
                    let (u, w) = (x as f32 + (i as f32 - 1.0) / 3.0, y as f32 + (j as f32 - 1.0) / 3.0);
                    let ring = ((u - xf).powi(2) + (w - zf).powi(2)).sqrt();
                    let h = hash(u as f64 * 7.0, w as f64 * 5.0);
                    // Pale dressed stone in rings round the basin, a charcoal course between.
                    let color = if (ring * 3.0) as i32 % 3 == 0 {
                        crypt::jit(crypt::pal::CHARCOAL[(h * 3.0) as usize % 3], h, 0.08)
                    } else {
                        crypt::jit(crypt::pal::PALE_STONE[(h * 2.0) as usize % 2], h, 0.06) * 0.8
                    };
                    let m = Mat4::from_scale_rotation_translation(
                        Vec3::new(0.31, 0.03, 0.31),
                        Quat::from_rotation_y((h as f32 - 0.5) * 0.06),
                        Vec3::new(u, top + 0.012, w),
                    );
                    self.static_cubes.push(Instance::new(m, color, 1.0, Vec3::ZERO, crypt::SHADE_FLAG));
                }
            }
            self.authored.push((start, self.static_cubes.len() - start));
        }

        // The spouts: one at the foot of the basin on each side a channel leaves from.
        let mut outlets = vec![];
        for channel in &plan.channels {
            let Some(first) = channel.first() else { continue };
            let dir = Vec2::new((first.p.x - f.p.x) as f32, (first.p.y - f.p.y) as f32);
            if dir.length_squared() != 1.0 {
                continue;
            }
            let lip = base + Vec3::new(dir.x * 0.5, 0.03, dir.y * 0.5);
            let water = first.level as f32 * STEP - garden::WATER_DROP;
            for (k, (along, hgt, across)) in [(0.47f32, 0.05f32, 0.1f32), (0.5, 0.03, 0.08)].iter().enumerate() {
                let size = if dir.x != 0.0 { Vec3::new(0.08, *hgt, *across * 2.0) } else { Vec3::new(*across * 2.0, *hgt, 0.08) };
                let at = base + Vec3::new(dir.x * along, 0.01 + k as f32 * 0.03, dir.y * along);
                let col = if k == 0 { jit(PALE_STONE[1], 0.4, 0.0) * 0.75 } else { jit(VERDIGRIS, 0.6, 0.0) * 1.1 };
                block(&mut self.static_cubes, at, size, 0.0, col, if k == 0 { SHADE_STONE } else { SHADE_BRONZE });
            }
            outlets.push((lip, water, dir));
        }
        self.fountain = Some(FountainView { base, outlets, lanterns });
    }

    /// The fountain's water: basin and bowl, the veil over the bowl's rim, the jets, the tongue's stream, splashes, spray and glints.
    pub(super) fn emit_fountain(&self, time: f32, f: &mut Frame) {
        let Some(fv) = &self.fountain else { return };
        let o = fv.base;
        let (disc, sphere, tube, annulus, cube) = (
            MeshKind::Disc as usize,
            MeshKind::Sphere as usize,
            MeshKind::Tube as usize,
            MeshKind::Annulus as usize,
            MeshKind::Cube as usize,
        );
        let flat = Mat4::from_rotation_x(-PI / 2.0);
        // The board's mist teal: the still water, and the jets paler.
        let water_tint = lin(0x24453f);
        let jet_tint = lin(0x8fb2aa);
        let basin_y = o.y + BASIN_WATER;
        let bowl_y = o.y + BOWL_WATER;

        // Still water in the basin and the bowl, churned where the water falls in.
        // (Rings spread from where the veil falls in, and in the bowl from where the jets land: the
        // shader's `water` takes the circle as a share of the disc, and the disc's size.)
        let (basin_r, bowl_r) = (0.425, BOWL_R - 0.02);
        let m = Mat4::from_translation(Vec3::new(o.x, basin_y, o.z)) * flat * Mat4::from_scale(Vec3::new(basin_r, basin_r, 1.0));
        f.transparent[disc].push(Instance::new(m, water_tint, 0.9, Vec3::new(BOWL_R / basin_r, basin_r, 3.0), SHADE_WATER));
        let m = Mat4::from_translation(Vec3::new(o.x, bowl_y, o.z)) * flat * Mat4::from_scale(Vec3::new(bowl_r, bowl_r, 1.0));
        f.transparent[disc].push(Instance::new(m, water_tint, 0.9, Vec3::new(0.17 / bowl_r, bowl_r, 2.7), SHADE_WATER));

        // The veil: the bowl brims over all round, a thin sheet flaring as it falls.
        let top = o.y + BOWL_RIM;
        let fall = top - basin_y;
        let m = Mat4::from_translation(Vec3::new(o.x, (top + basin_y) / 2.0, o.z))
            * Mat4::from_scale(Vec3::new(BOWL_R + 0.004, fall, BOWL_R + 0.004));
        f.transparent[tube].push(Instance::new(m, lin(0x9cbcb4), 0.38, Vec3::ZERO, SHADE_WATER));

        // Four jets arc in from the basin's sides and land in the bowl at his hooves.
        let mut splashes: Vec<(Vec3, f32)> = vec![];
        for k in 0..4 {
            let a = k as f32 * PI / 2.0 + PI / 2.0;
            let (ca, sa) = (a.cos(), a.sin());
            let start = Vec3::new(o.x + ca * 0.385, o.y + COPING + 0.055, o.z + sa * 0.385);
            let land = Vec3::new(o.x + ca * 0.17, bowl_y, o.z + sa * 0.17);
            let lift = 0.62 + (time * 1.3 + k as f32 * 1.7).sin() * 0.04;
            let seed = k as f32 * 3.7;
            water_rope(
                f,
                sphere,
                |t| start.lerp(land, t) + Vec3::new(0.0, 4.0 * lift * t * (1.0 - t), 0.0),
                16,
                0.011,
                0.014,
                jet_tint,
                0.5,
                seed,
            );
            splashes.push((land, 0.7));
            // Glints running down the jet.
            for g in 0..3 {
                let t = (time * 0.9 + g as f32 / 3.0 + k as f32 * 0.21).fract();
                let p = start.lerp(land, t) + Vec3::new(0.0, 4.0 * lift * t * (1.0 - t), 0.0);
                f.additive.push(Instance::new(
                    Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.012)),
                    lin(0xc8e4dc),
                    0.35,
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
        }

        // The stream off the tip of his tongue, down past the bowl into the basin.
        let tip = o + Vec3::new(0.0, STATUE_BASE, 0.0) + TONGUE_TIP;
        let hit = Vec3::new(o.x, basin_y, o.z - 0.37);
        let wobble = (time * 7.0).sin() * 0.004;
        water_rope(
            f,
            sphere,
            |t| tip.lerp(hit, t * t) + Vec3::new(wobble * t, 0.0, -0.02 * (t * PI).sin()),
            12,
            0.006,
            0.011,
            jet_tint * 0.7,
            0.42,
            11.0,
        );
        splashes.push((hit, 1.0));

        // Rings spreading where the water lands, droplets thrown up, spray drifting off.
        for (i, &(p, strength)) in splashes.iter().enumerate() {
            for j in 0..2 {
                let ph = (time * 1.4 + j as f32 * 0.5 + i as f32 * 0.31).fract();
                let r = 0.02 + ph * 0.09 * strength;
                let m = Mat4::from_translation(p + Vec3::new(0.0, 0.004, 0.0)) * flat * Mat4::from_scale(Vec3::new(r, r, 1.0));
                f.transparent[annulus].push(Instance::new(m, lin(0xe4f4f8), 0.45 * (1.0 - ph), Vec3::ZERO, SHADE_UNLIT));
            }
            for d in 0..4 {
                let seed = i as f32 * 7.0 + d as f32 * 1.9;
                let ph = (time * 2.3 + seed * 0.37).fract();
                let a = seed * 2.4 + (time * 0.5).floor();
                let up = 0.09 * strength;
                let q = p + Vec3::new(a.cos() * ph * 0.06, 4.0 * up * ph * (1.0 - ph), a.sin() * ph * 0.06);
                f.transparent[sphere].push(Instance::new(
                    Mat4::from_translation(q) * Mat4::from_scale(Vec3::splat(0.007)),
                    jet_tint,
                    0.7 * (1.0 - ph),
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
        }
        for k in 0..10 {
            let fk = k as f32;
            let ph = (time * 0.25 + fk * 0.1).fract();
            let a = fk * 2.39 + time * 0.2;
            let r = 0.3 + ph * 0.25;
            let p = Vec3::new(o.x + a.cos() * r, basin_y + 0.05 + ph * 0.5, o.z + a.sin() * r);
            let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.07 + ph * 0.12));
            f.transparent[sphere].push(Instance::new(m, lin(0x9ab8b0), 0.035 * (PI * ph).sin(), Vec3::ZERO, SHADE_LIT));
        }

        // The spouts under the bridges pour into the channels.
        for (i, &(lip, water, dir)) in fv.outlets.iter().enumerate() {
            let drop = (lip.y - water).max(0.02);
            let at = Vec3::new(lip.x + dir.x * 0.03, water + drop / 2.0, lip.z + dir.y * 0.03);
            let size = if dir.x != 0.0 { Vec3::new(0.002, drop, 0.16) } else { Vec3::new(0.16, drop, 0.002) };
            f.transparent[cube].push(Instance::new(
                Mat4::from_translation(at) * Mat4::from_scale(size),
                lin(0xc8dce4),
                0.8,
                Vec3::ZERO,
                SHADE_WATER,
            ));
            // (Rings where it lands, not foam: under the bridge, foam would rise through its deck.)
            for k in 0..2 {
                let t = (time * 0.7 + k as f32 * 0.5 + i as f32 * 0.37).fract();
                let r = 0.05 + t * 0.2;
                let m = Mat4::from_translation(Vec3::new(at.x + dir.x * 0.08, water + 0.005, at.z + dir.y * 0.08))
                    * flat
                    * Mat4::from_scale(Vec3::new(r, r, 1.0));
                f.transparent[annulus].push(Instance::new(m, lin(0x9fb8b4), 0.25 * (1.0 - t), Vec3::ZERO, SHADE_UNLIT));
            }
        }

        // The plaza's lanterns: a soft amber pool each, and a halo in the damp air.
        for (i, &p) in fv.lanterns.iter().enumerate() {
            let flick = 1.0 + (time * 7.0 + i as f32 * 1.9).sin() * 0.06;
            glow_orb(f, p, 0.32, lin(0xffb050), 0.18 * flick);
            // (Warm pools on the wet paving and on the bronze: the plaza is the first thing seen.)
            f.lights.push(Light { pos: p, color: lin(0xffb050) * 2.3 * flick, distance: 3.4 });
        }
        // A lamp under the water lights him from below, shimmering as the surface moves.
        let shimmer = (time * 2.7).sin() * 0.25 + (time * 5.3 + 1.0).sin() * 0.15;
        if o.distance_squared(self.focus_point) > 20.0 * 20.0 {
            return;
        }
        f.lights.push(Light { pos: Vec3::new(o.x, o.y + 0.36, o.z), color: lin(0x6ea89c) * (0.45 + shimmer * 0.25), distance: 2.6 });
    }
}

/**
 * A rope of water along the path `at(t)`, t from 0 to 1: overlapping
 * stretched drops, each turned along the path, narrowing from `r0` to `r1`.
 */
#[allow(clippy::too_many_arguments)]
fn water_rope(
    f: &mut Frame,
    sphere: usize,
    at: impl Fn(f32) -> Vec3,
    segments: usize,
    r0: f32,
    r1: f32,
    color: Vec3,
    alpha: f32,
    seed: f32,
) {
    for i in 0..segments {
        let (t0, t1) = (i as f32 / segments as f32, (i + 1) as f32 / segments as f32);
        let (a, b) = (at(t0), at(t1));
        let d = b - a;
        let len = d.length();
        if len < 1e-4 {
            continue;
        }
        let r = r0 + (r1 - r0) * (t0 + t1) / 2.0;
        let m =
            Mat4::from_scale_rotation_translation(Vec3::new(r, len * 0.85, r), Quat::from_rotation_arc(Vec3::Y, d / len), (a + b) / 2.0);
        f.transparent[sphere].push(Instance::new(m, color, alpha, Vec3::new(seed + i as f32 * 0.37, 0.0, 0.0), SHADE_JET));
    }
}
