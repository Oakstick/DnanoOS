//! Krampus's lair's own (FINALE.md §3): Krampus as he fights, and his Perchten. Each faces +z,
//! stands in the middle of its grid and is sculpted in the parts its rig swings.
//!
//! Krampus's look is fixed by Scene I (`cast::krampus_shade`, "Krampus as the boys see him"):
//! long black shaggy fur on a hunched man-goat, long sinewy arms with hooked claws, cloven
//! hooves, a goat's face thrust forward with its jaw hanging open, fangs and a long red tongue
//! lolling over his chest, ember eyes with a goat's slit pupils, great ridged horns curling back
//! and round, chains of real links wound round him hung with bells, and a wicker basket on his
//! back bristling with birch switches. Here he is the same beast at a boss's size and ready to
//! fight: in his right fist a bundle of birch switches bound with a red ribbon, from his left a
//! length of chain hanging to the floor with a great bell on its end, a sack of coals slung at
//! his hip (it swings on its own joint), and his eyes burning.
//!
//! A Percht is one of the masked companions who walk with him in the Alpine winter nights: a
//! shaggy pelt of long grey-brown sheepskin to the knees, a belt of great cowbells, and a carved
//! wooden mask, red-faced and grinning, with a tongue out and two pairs of horns; a birch switch
//! in its fist.

use std::f32::consts::TAU;

use glam::Vec3;

use crate::render::carve::{ball, limb, nz, shag, v};
use crate::render::cast::{bell, birch, horn, link};
use crate::render::characters::{mat, Sculpt};
use crate::render::voxels::{Collected, PartDef};

mod pal {
    use super::mat;
    pub const FUR: u32 = 0x1a151d | mat::FUR;
    pub const FUR_D: u32 = 0x0e0b10 | mat::FUR;
    pub const FUR_L: u32 = 0x2e2533 | mat::FUR;
    pub const FUR_TIP: u32 = 0x3e3440 | mat::FUR;
    pub const HIDE: u32 = 0x2c2428 | mat::SKIN;
    pub const HIDE_D: u32 = 0x1e181c | mat::SKIN;
    pub const HORN: u32 = 0x2e2820 | mat::IVORY;
    pub const HORN_D: u32 = 0x1e1a14 | mat::IVORY;
    pub const HORN_TIP: u32 = 0x5a4e3a | mat::IVORY;
    pub const HOOF: u32 = 0x141210 | mat::IVORY;
    pub const HOOF_L: u32 = 0x2a2620 | mat::IVORY;
    pub const CLAW: u32 = 0x8a7c66 | mat::IVORY;
    pub const TONGUE: u32 = 0xb01a2c | mat::SKIN;
    pub const TONGUE_D: u32 = 0x7a0e1a | mat::SKIN;
    pub const GUM: u32 = 0x5a0e16 | mat::SKIN;
    pub const MAW: u32 = 0x0a0606;
    pub const FANG: u32 = 0xe4d8bc | mat::IVORY;
    pub const EYE: u32 = 0xff3a1a | mat::GLOW;
    pub const EYE_L: u32 = 0xffa04a | mat::GLOW;
    pub const SLIT: u32 = 0x1a0402;
    pub const WICKER: u32 = 0x4a3622 | mat::WOOD;
    pub const WICKER_M: u32 = 0x3c2c1c | mat::WOOD;
    pub const WICKER_D: u32 = 0x2a1e14 | mat::WOOD;
    pub const RIBBON: u32 = 0xb01822 | mat::CLOTH;
    pub const RIBBON_D: u32 = 0x7a0e16 | mat::CLOTH;
    pub const CHAIN: u32 = 0x3c3e44 | mat::METAL;
    pub const CHAIN_L: u32 = 0x6a6e76 | mat::METAL;
    pub const BELL: u32 = 0x9e7c34 | mat::GOLD;
    pub const BELL_D: u32 = 0x5e4a22 | mat::GOLD;
    pub const STRAP: u32 = 0x2a1e16 | mat::LEATHER;
    pub const SACK: u32 = 0x5a4630 | mat::CLOTH;
    pub const SACK_D: u32 = 0x3e3020 | mat::CLOTH;
    pub const SACK_L: u32 = 0x6e5a40 | mat::CLOTH;
    pub const ROPE: u32 = 0x8a7450 | mat::CLOTH;
    pub const COAL: u32 = 0x1a1412;
    pub const EMBER: u32 = 0xff4a14 | mat::GLOW;
    // A Percht's
    pub const PELT: u32 = 0xb4a690 | mat::FUR;
    pub const PELT_D: u32 = 0x857866 | mat::FUR;
    pub const PELT_L: u32 = 0xd2c6ae | mat::FUR;
    pub const PELT_TIP: u32 = 0xe6dcc6 | mat::FUR;
    pub const MASK: u32 = 0x8a2a1c | mat::WOOD;
    pub const MASK_D: u32 = 0x5a1a12 | mat::WOOD;
    pub const MASK_L: u32 = 0xb0402a | mat::WOOD;
    pub const MASK_HAIR: u32 = 0x2a2420 | mat::FUR;
    pub const TEETH: u32 = 0xe8dcc0 | mat::IVORY;
    pub const MASK_EYE: u32 = 0xf0e6c8 | mat::IVORY;
    pub const PUPIL: u32 = 0x100c0a;
    pub const COWBELL: u32 = 0x6a5a3a | mat::METAL;
    pub const COWBELL_D: u32 = 0x3a3022 | mat::METAL;
    pub const BOOT: u32 = 0x2a1e16 | mat::LEATHER;
    pub const HAND: u32 = 0x3a2a22 | mat::LEATHER;
}
use pal::*;

/// Krampus's grid (horns, basket, birch and chain all in it) and voxel: as Scene I's, a little
/// wider for what he holds, about three and a quarter tiles tall.
pub const KRAMPUS_SIZE: (i32, i32, i32) = (100, 130, 96);
pub const KRAMPUS_VOXEL: f32 = 0.025;
/// His middle across, and how deep his body stands.
const CX: f32 = 50.0;
/// The bells on his chains (his death silences them one by one): where each hangs, in his grid.
pub const KRAMPUS_BELLS: [[f32; 3]; 7] = [
    [42.0, 51.0, 51.0],
    [57.0, 51.0, 51.0],
    [50.0, 50.0, 52.0],
    [37.0, 71.0, 57.0],
    [62.0, 74.0, 57.0],
    [50.0, 72.0, 59.0],
    [78.0, 18.0, 72.0],
];

/// His joints: head and horns, the birch arm, the chain arm, both legs from the hip, and the sack
/// at his hip.
pub const KRAMPUS_RIG: &[PartDef] = &[
    PartDef { name: "head", boxes: &[], pivot: [CX, 82.0, 48.0] },
    PartDef { name: "armR", boxes: &[], pivot: [CX - 21.0, 81.0, 44.0] },
    PartDef { name: "armL", boxes: &[], pivot: [CX + 21.0, 81.0, 44.0] },
    PartDef { name: "legR", boxes: &[], pivot: [CX - 10.0, 44.0, 38.0] },
    PartDef { name: "legL", boxes: &[], pivot: [CX + 10.0, 44.0, 38.0] },
    PartDef { name: "sack", boxes: &[], pivot: [CX - 17.0, 57.0, 38.0] },
];

/**
 * Krampus as he fights (`KRAMPUS_RIG`): Scene I's beast at a boss's size, the birch bundle in his
 * right fist, the chain and its bell hanging from his left, his sack of coals at his hip.
 */
pub fn krampus() -> Collected {
    let (w, h, d) = KRAMPUS_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let cx = CX;

    // ---- goat legs: cloven hooves, thin shanks back to the hock, shaggy thighs forward to the hip
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        let x = cx + side * 10.0;
        s.part(4 + i as u8);
        s.ellipsoid((x, 3.2, 45.0), (4.8, 3.6, 6.0), 0, HOOF);
        s.paint(|_x, y, _z, c| if c == HOOF && y >= 5 { Some(HOOF_L) } else { None });
        // The cleft between the two toes.
        for y in 0..=6 {
            for z in 45..=52 {
                s.clear(x.round() as i32, y, z);
            }
        }
        limb(&mut s, &[(v(x, 6.0, 43.0), 3.0), (v(x, 24.0, 33.0), 2.8)], HIDE_D);
        ball(&mut s, v(x, 9.0, 42.4), v(4.8, 2.8, 4.8), FUR);
        ball(&mut s, v(x, 25.0, 32.4), Vec3::splat(4.4), FUR);
        limb(&mut s, &[(v(x, 26.0, 32.0), 5.6), (v(x + side * 0.8, 37.0, 38.0), 7.8), (v(x + side * 0.4, 45.0, 40.0), 8.6)], FUR);
    }

    // ---- hips, belly, chest and heavy shoulders, hunched forward
    s.part(0);
    ball(&mut s, v(cx, 51.0, 37.0), v(16.8, 9.2, 12.4), FUR);
    ball(&mut s, v(cx, 62.0, 39.0), v(15.6, 12.4, 12.8), FUR);
    ball(&mut s, v(cx, 74.0, 43.0), v(18.4, 12.0, 13.2), FUR);
    ball(&mut s, v(cx, 80.0, 45.0), v(22.4, 7.2, 11.2), FUR);
    ball(&mut s, v(cx, 82.0, 38.0), v(14.0, 8.0, 9.0), FUR);
    limb(&mut s, &[(v(cx, 79.0, 46.0), 8.4), (v(cx, 87.0, 53.0), 7.2)], FUR);
    // A broad leather belt over the hips, the sack's rope tied to it.
    for k in 0..48 {
        let a = k as f32 / 48.0 * TAU;
        let p = v(cx + a.cos() * 17.6, 55.0, 37.5 + a.sin() * 13.2);
        s.capsule((p.x, p.y - 1.2, p.z), (p.x, p.y + 1.2, p.z), 1.1, 1.1, STRAP);
    }

    // ---- the basket on his back: stakes and weavers over and under, a braided rim, leather bands
    let (bx0, bx1, bz0, bz1, by0, by1) = (36, 65, 14, 28, 58, 90);
    for y in by0..=by1 {
        for z in bz0..=bz1 {
            for x in bx0..=bx1 {
                let sides = x <= bx0 + 1 || x >= bx1 - 1;
                let ends = z <= bz0 + 1 || z >= bz1 - 1;
                if !(sides || ends || y <= by0 + 1) {
                    continue;
                }
                let along = if ends { x } else { z };
                let band = y == 66 || y == 67 || y == 80 || y == 81;
                let outer = x == bx0 || x == bx1 || z == bz0 || z == bz1;
                let c = if band && outer {
                    STRAP
                } else if along % 5 == 0 {
                    WICKER_D
                } else if (along / 5 + y / 2) % 2 == 0 {
                    WICKER
                } else {
                    WICKER_M
                };
                s.set(x, y, z, c);
            }
        }
    }
    for z in bz0 - 1..=bz1 + 1 {
        for x in bx0 - 1..=bx1 + 1 {
            if x <= bx0 || x >= bx1 || z <= bz0 || z >= bz1 {
                for y in by1 + 1..=by1 + 2 {
                    s.set(x, y, z, if (x + z + y) % 3 == 0 { WICKER_D } else { WICKER });
                }
            }
        }
    }
    for i in 0..9 {
        let fi = i as f32;
        let base = v(40.0 + fi * 2.8, 80.0, 18.0 + (i % 3) as f32 * 2.5);
        let tip = v(38.0 + fi * 3.4 + (fi * 1.7).sin() * 2.0, 104.0 + (i % 4) as f32 * 2.2, 15.0 + (i % 3) as f32 * 2.0);
        birch(&mut s, base, tip, i);
    }
    for side in [-1.0f32, 1.0] {
        limb(
            &mut s,
            &[(v(cx + side * 10.0, 88.0, 27.0), 1.3), (v(cx + side * 12.0, 87.0, 40.0), 1.3), (v(cx + side * 12.4, 72.0, 53.0), 1.3)],
            STRAP,
        );
    }

    // ---- chains round his waist and across his chest, link by link, hung with bells
    let chains = [(54.0f32, 16.8f32, 12.6f32, 37.5f32, 0.0f32), (72.0, 19.4, 14.2, 43.0, 0.35)];
    let on_chain = |(cy, crx, crz, oz, tilt): (f32, f32, f32, f32, f32), a: f32| {
        v(cx + a.cos() * crx, cy + (a * 2.0).sin() * 1.2 + a.cos() * tilt * 12.0, oz + a.sin() * crz)
    };
    for ch in chains {
        let n = 28;
        for i in 0..n {
            let a = i as f32 / n as f32 * TAU;
            let t = (on_chain(ch, a + 0.01) - on_chain(ch, a - 0.01)).normalize();
            link(&mut s, on_chain(ch, a), t, i % 2 == 0, CHAIN, CHAIN_L);
        }
    }
    for b in &KRAMPUS_BELLS[..6] {
        bell(&mut s, Vec3::from(*b) + v(0.0, 2.2, 0.0), d, BELL, BELL_D, MAW);
    }

    // ---- the arms: long, shaggy to the elbow, sinewy below, clawed
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(2 + i as u8);
        let sh = v(cx + side * 21.0, 81.0, 44.0);
        let elbow = v(cx + side * 26.0, 66.0, 52.0);
        let wrist = v(cx + side * 27.0, 52.0, 60.0);
        limb(&mut s, &[(sh, 6.4), (elbow, 5.4)], FUR);
        ball(&mut s, elbow, Vec3::splat(5.8), FUR_L);
        limb(&mut s, &[(elbow, 4.6), (wrist, 3.8)], HIDE_D);
        // A fist closed round what it holds: knuckles, and the claws curled over.
        let fist = wrist + v(0.0, -3.6, 2.0);
        ball(&mut s, fist, v(4.2, 4.4, 4.0), HIDE);
        for k in 0..4 {
            let kx = fist.x + (k as f32 - 1.5) * 1.9;
            let knuckle = v(kx, fist.y + 0.6, fist.z + 3.4);
            ball(&mut s, knuckle, Vec3::splat(1.4), HIDE_D);
            let tip = knuckle + v(0.0, -2.6, 0.6);
            s.capsule((knuckle.x, knuckle.y - 1.0, knuckle.z + 0.8), (tip.x, tip.y, tip.z), 0.7, 0.2, CLAW);
        }
    }

    // ---- the birch bundle in his right fist: switches bound with a red ribbon, fanning out
    s.part(2);
    let grip = v(cx - 27.0, 48.4, 62.0);
    for k in 0..9 {
        let a = k as f32 / 9.0 * TAU;
        let spread = v(a.cos() * 3.4, a.sin() * 2.2, 0.0);
        let base = grip + v(a.cos() * 0.8, 3.0 + a.sin() * 0.6, -3.0);
        let tip = grip + v(0.0, -10.0, 22.0) + spread * (1.0 + (k % 3) as f32 * 0.3);
        birch(&mut s, base, tip, k);
    }
    for k in 0..3 {
        let y = grip.y - 1.6 + k as f32 * 0.9;
        let z = grip.z + 3.2 + k as f32 * 0.6;
        s.ellipsoid((grip.x, y, z), (2.4, 1.0, 1.0), 0, if k == 1 { RIBBON_D } else { RIBBON });
    }
    // The ribbon's two tails hanging from the knot.
    limb(&mut s, &[(grip + v(1.0, -1.0, 4.4), 0.6), (grip + v(2.2, -6.0, 5.0), 0.5), (grip + v(1.4, -10.0, 4.4), 0.4)], RIBBON);
    limb(&mut s, &[(grip + v(-1.0, -1.0, 4.4), 0.6), (grip + v(-2.4, -5.0, 6.0), 0.5), (grip + v(-3.0, -8.4, 5.6), 0.4)], RIBBON_D);

    // ---- the chain from his left fist, hanging to the floor, and its great bell
    s.part(3);
    let hold = v(cx + 27.0, 46.0, 63.0);
    let foot = v(cx + 28.0, 19.0, 71.0);
    let n = 14;
    for k in 0..=n {
        let t = k as f32 / n as f32;
        let sag = (t * std::f32::consts::PI).sin() * 2.6;
        let p = hold.lerp(foot, t) + v(0.0, 0.0, sag);
        let q = hold.lerp(foot, t + 0.01) + v(0.0, 0.0, ((t + 0.01) * std::f32::consts::PI).sin() * 2.6);
        link(&mut s, p, (q - p).normalize_or_zero(), k % 2 == 0, CHAIN, CHAIN_L);
    }
    let big = Vec3::from(KRAMPUS_BELLS[6]);
    s.ellipsoid((big.x, big.y + 1.0, big.z), (4.6, 5.4, 4.6), (big.y - 3.0) as i32, BELL);
    for y in (big.y - 4.0) as i32..=(big.y - 3.0) as i32 {
        for z in (big.z - 5.0) as i32..=(big.z + 5.0) as i32 {
            for x in (big.x - 5.0) as i32..=(big.x + 5.0) as i32 {
                let r = ((x as f32 - big.x).powi(2) + (z as f32 - big.z).powi(2)).sqrt();
                if r <= 5.6 {
                    s.set(x, y, z, if r > 4.4 { BELL_D } else { MAW });
                }
            }
        }
    }
    ball(&mut s, big + v(0.0, 6.6, 0.0), v(1.4, 1.4, 1.4), BELL_D);
    ball(&mut s, big + v(0.0, -4.6, 0.0), Vec3::splat(1.3), BELL_D);

    // ---- the sack of coals at his right hip, tied to the belt: lumpy burlap, a glow at its neck
    s.part(6);
    let neck = v(cx - 18.0, 52.0, 40.0);
    ball(&mut s, neck + v(-3.6, -12.0, 1.0), v(7.6, 9.4, 7.2), SACK);
    ball(&mut s, neck + v(-4.4, -17.0, 2.0), v(6.8, 5.4, 6.6), SACK_D);
    ball(&mut s, neck + v(-2.0, -7.0, 2.6), v(5.0, 5.0, 4.4), SACK_L);
    s.paint(|x, y, z, c| {
        if (c == SACK || c == SACK_L) && (x + y * 3 + z) % 7 == 0 {
            Some(SACK_D)
        } else if c == SACK && nz(x, y, z) > 0.85 {
            Some(SACK_L)
        } else {
            None
        }
    });
    limb(&mut s, &[(neck + v(-1.0, -3.0, 1.0), 2.2), (neck + v(0.0, 0.0, 0.0), 1.6)], SACK_D);
    for k in 0..10 {
        let a = k as f32 / 10.0 * TAU;
        s.set((neck.x - 1.0 + a.cos() * 2.4) as i32, (neck.y - 3.0) as i32, (neck.z + 1.0 + a.sin() * 2.4) as i32, ROPE);
    }
    // Coals showing at the sack's mouth, one still glowing.
    for (dx, dz, c) in [(-1.0f32, 2.0f32, COAL), (1.0, 2.6, EMBER), (0.0, 3.4, COAL)] {
        s.set((neck.x + dx) as i32, (neck.y - 2.0) as i32, (neck.z + dz) as i32, c);
    }

    // ---- the head, thrust forward and low: mane, skull, a goat's snout, the jaw hanging open
    s.part(1);
    ball(&mut s, v(cx, 93.0, 48.0), v(12.4, 12.4, 9.2), FUR);
    ball(&mut s, v(cx, 95.0, 57.0), v(9.6, 9.2, 9.6), HIDE);
    ball(&mut s, v(cx, 90.4, 66.4), v(6.2, 5.4, 6.8), HIDE);
    ball(&mut s, v(cx, 91.2, 72.4), v(4.6, 3.4, 1.8), HIDE_D);
    ball(&mut s, v(cx, 84.8, 64.8), v(5.8, 2.6, 6.4), HIDE_D);
    s.paint(|_x, y, z, c| if c == HIDE && z > 62 && y > 92 && y % 2 == 0 { Some(HIDE_D) } else { None });
    let (x0, x1) = ((cx - 3.5) as i32, (cx + 3.5) as i32);
    for z in 62..=72 {
        for x in x0..=x1 {
            s.set(x, 86, z, MAW);
            s.set(x, 87, z, if z >= 71 { GUM } else { MAW });
        }
    }
    for x in [x0 + 1, x1 - 1] {
        s.cuboid(x, 84, 71, x, 86, 71, FANG);
        s.set(x, 85, 72, FANG);
    }
    for x in [x0 + 2, x1 - 2] {
        s.cuboid(x, 87, 70, x, 88, 70, FANG);
    }
    // The long tongue lolling out and down over his chest, a groove down its middle.
    limb(
        &mut s,
        &[
            (v(cx, 86.0, 68.0), 2.8),
            (v(cx, 83.0, 73.0), 2.4),
            (v(cx + 1.2, 77.0, 76.0), 2.0),
            (v(cx, 70.0, 77.0), 1.6),
            (v(cx - 0.8, 64.0, 76.0), 1.2),
        ],
        TONGUE,
    );
    let mid = cx.round() as i32;
    s.paint(
        |x, y, z, c| if c == TONGUE && (x == mid || x == mid - 1) && y < 84 && z > 74 && nz(x, y, z) > 0.3 { Some(TONGUE_D) } else { None },
    );
    // A heavy brow over ember eyes with a goat's slit pupils.
    ball(&mut s, v(cx, 99.6, 63.6), v(8.8, 2.2, 3.2), HIDE_D);
    for ex in [cx - 5.0, cx + 5.0] {
        for y in 97..=99 {
            for x in (ex - 1.5) as i32..=(ex + 1.5) as i32 {
                for z in 66..d {
                    s.clear(x, y, z);
                }
                s.set(x, y, 64, HIDE_D);
                let middle = (x as f32 - ex).abs() < 1.0;
                s.set(
                    x,
                    y,
                    65,
                    if y == 98 && middle {
                        SLIT
                    } else if y == 99 {
                        EYE_L
                    } else {
                        EYE
                    },
                );
            }
        }
    }
    for side in [-1.0f32, 1.0] {
        // Goat's ears sticking out.
        limb(
            &mut s,
            &[(v(cx + side * 8.4, 97.0, 54.0), 2.8), (v(cx + side * 13.6, 99.6, 52.0), 1.8), (v(cx + side * 17.6, 101.6, 50.0), 0.8)],
            HIDE_D,
        );
        // Great horns: up and back, curling round and forward at the tips, ridged.
        let k = |dx: f32, y: f32, z: f32, r: f32| (v(cx + side * dx, y, z), r);
        let knots = [
            k(5.2, 102.0, 57.0, 4.4),
            k(9.6, 110.0, 53.0, 4.0),
            k(15.6, 117.0, 47.0, 3.4),
            k(22.0, 121.0, 43.0, 2.9),
            k(28.0, 120.0, 42.0, 2.4),
            k(32.4, 114.0, 44.8, 2.0),
            k(33.0, 108.0, 49.6, 1.5),
            k(30.4, 104.8, 53.6, 1.0),
        ];
        horn(&mut s, &knots, HORN, HORN_D, HORN_TIP);
    }
    limb(&mut s, &[(v(cx, 82.0, 62.0), 3.6), (v(cx, 76.0, 61.0), 2.2), (v(cx, 71.0, 59.6), 1.0)], FUR_L);
    s.part(0);

    // ---- long shaggy fur over it all, blacker in the hollows, paler at the tips
    s.paint(|x, y, z, c| {
        if c != FUR {
            return None;
        }
        let n = nz(x, y / 3, z);
        Some(if n < 0.25 {
            FUR_D
        } else if n > 0.86 {
            FUR_L
        } else {
            FUR
        })
    });
    shag(&mut s, (w, h, d), FUR, FUR_D, FUR_TIP, 0.3, 3);
    shag(&mut s, (w, h, d), FUR_D, FUR, FUR_TIP, 0.16, 2);
    s.finish(0.04)
}

/// A Percht's grid and voxel: about two tiles tall.
pub const PERCHT_SIZE: (i32, i32, i32) = (48, 82, 40);
pub const PERCHT_VOXEL: f32 = 0.025;
const PX: f32 = 24.0;
const PZ: f32 = 20.0;

/// A Percht's joints: both legs, both arms (the switch in the right), the masked head.
pub const PERCHT_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [PX + 5.0, 30.0, PZ] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [PX - 5.0, 30.0, PZ] },
    PartDef { name: "leftArm", boxes: &[], pivot: [PX + 11.0, 52.0, PZ] },
    PartDef { name: "rightArm", boxes: &[], pivot: [PX - 11.0, 52.0, PZ] },
    PartDef { name: "head", boxes: &[], pivot: [PX, 56.0, PZ + 1.0] },
];

/**
 * A Percht (`PERCHT_RIG`): a shaggy sheepskin pelt to the knees, a belt of cowbells, a carved red
 * mask grinning with its tongue out, two pairs of horns, and a birch switch in its right fist.
 */
pub fn percht() -> Collected {
    let (w, h, d) = PERCHT_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (PX, PZ);
    // Legs: fur to the knee, then boots.
    for (i, side) in [1.0f32, -1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        let x = cx + side * 5.0;
        limb(&mut s, &[(v(x, 30.0, cz), 4.4), (v(x, 14.0, cz + 0.6), 3.8)], PELT);
        limb(&mut s, &[(v(x, 13.0, cz + 0.6), 3.2), (v(x, 3.0, cz + 1.0), 3.0)], BOOT);
        ball(&mut s, v(x, 2.0, cz + 2.6), v(3.0, 2.0, 4.2), BOOT);
    }
    // The pelt: a great shaggy body to the knees, broad at the shoulders.
    s.part(0);
    ball(&mut s, v(cx, 40.0, cz), v(11.0, 14.0, 9.0), PELT);
    ball(&mut s, v(cx, 50.0, cz), v(12.4, 8.0, 9.6), PELT);
    ball(&mut s, v(cx, 26.0, cz), v(10.4, 9.0, 8.6), PELT);
    // The belt of cowbells round the waist.
    for k in 0..36 {
        let a = k as f32 / 36.0 * TAU;
        let p = v(cx + a.cos() * 11.4, 34.0, cz + a.sin() * 9.4);
        s.capsule((p.x, p.y - 0.8, p.z), (p.x, p.y + 0.8, p.z), 0.9, 0.9, BOOT);
    }
    for k in 0..6 {
        let a = (k as f32 + 0.5) / 6.0 * TAU;
        let c = v(cx + a.cos() * 12.6, 30.0, cz + a.sin() * 10.6);
        ball(&mut s, c, v(2.6, 3.4, 2.2), COWBELL);
        ball(&mut s, c + v(0.0, -3.0, 0.0), v(2.4, 0.8, 2.0), COWBELL_D);
    }
    // Arms: shaggy sleeves, dark leather hands.
    for (i, side) in [1.0f32, -1.0].into_iter().enumerate() {
        s.part(3 + i as u8);
        let sh = v(cx + side * 11.0, 52.0, cz);
        let elbow = v(cx + side * 13.6, 41.0, cz + 3.0);
        let hand = v(cx + side * 13.0, 31.0, cz + 7.0);
        limb(&mut s, &[(sh, 4.4), (elbow, 3.8), (hand + v(0.0, 2.0, -1.0), 3.2)], PELT);
        ball(&mut s, hand, v(2.6, 2.8, 2.6), HAND);
    }
    // The birch switch in the right fist, pointing forward and down.
    s.part(4);
    let grip = v(cx - 13.0, 30.0, cz + 8.0);
    for k in 0..4 {
        let a = k as f32 / 4.0 * TAU;
        birch(&mut s, grip + v(a.cos() * 0.4, 2.0, -1.0), grip + v(a.cos() * 2.0, -8.0 + a.sin(), 14.0), k);
    }
    // The mask: a carved red face, bulging eyes, a grinning mouth of teeth with a tongue out, and
    // a mane of black horsehair behind it.
    s.part(5);
    ball(&mut s, v(cx, 60.0, cz - 1.0), v(8.6, 9.0, 8.0), MASK_HAIR);
    ball(&mut s, v(cx, 61.0, cz + 4.0), v(7.4, 8.2, 5.6), MASK);
    ball(&mut s, v(cx, 57.0, cz + 7.0), v(5.6, 4.2, 3.6), MASK_L);
    // Brow ridges and a hooked nose.
    ball(&mut s, v(cx, 64.6, cz + 8.2), v(6.4, 1.4, 1.8), MASK_D);
    limb(&mut s, &[(v(cx, 63.0, cz + 9.4), 1.6), (v(cx, 59.4, cz + 11.2), 1.3)], MASK_L);
    // Bulging eyes, white with black pupils.
    for side in [-1.0f32, 1.0] {
        let e = v(cx + side * 3.2, 62.6, cz + 9.0);
        ball(&mut s, e, Vec3::splat(1.6), MASK_EYE);
        s.set(e.x.round() as i32, e.y.round() as i32, (e.z + 1.6).round() as i32, PUPIL);
    }
    // The grinning mouth: a dark slot, teeth above and below, the tongue hanging out of it.
    for x in (cx - 4.0) as i32..=(cx + 4.0) as i32 {
        let z = (cz + 9.6) as i32;
        s.set(x, 55, z, PUPIL);
        s.set(x, 56, z, if x % 2 == 0 { TEETH } else { PUPIL });
        s.set(x, 54, z, if x % 2 != 0 { TEETH } else { PUPIL });
    }
    limb(&mut s, &[(v(cx, 54.6, cz + 9.6), 1.5), (v(cx + 0.4, 50.6, cz + 10.8), 1.3), (v(cx, 47.6, cz + 10.4), 1.0)], TONGUE);
    // Two pairs of horns: ram's curling round the ears, goat's sweeping up and back.
    for side in [-1.0f32, 1.0] {
        let k = |dx: f32, y: f32, z: f32, r: f32| (v(cx + side * dx, y, z), r);
        horn(
            &mut s,
            &[
                k(5.6, 66.0, cz + 2.0, 2.2),
                k(9.0, 69.0, cz - 1.0, 1.9),
                k(11.6, 66.0, cz - 3.0, 1.6),
                k(11.4, 61.0, cz - 1.0, 1.3),
                k(9.6, 58.0, cz + 2.0, 1.0),
            ],
            HORN,
            HORN_D,
            HORN_TIP,
        );
        horn(
            &mut s,
            &[k(2.6, 68.0, cz + 3.0, 1.8), k(4.2, 73.0, cz + 0.0, 1.5), k(6.0, 76.0, cz - 3.0, 1.1), k(7.4, 77.0, cz - 6.0, 0.7)],
            HORN_D,
            HORN,
            HORN_TIP,
        );
    }
    s.part(0);
    shag(&mut s, (w, h, d), PELT, PELT_D, PELT_TIP, 0.35, 4);
    shag(&mut s, (w, h, d), PELT_D, PELT_L, PELT_TIP, 0.2, 2);
    s.paint(|x, y, z, c| if c == PELT && nz(x, y / 2, z) > 0.8 { Some(PELT_L) } else { None });
    s.finish(0.04)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::voxels::build_rig;

    #[test]
    fn krampus_is_a_bosss_size_and_his_perchten_smaller_and_every_part_has_its_voxels() {
        for (name, c, rig, voxel, lo, hi) in
            [("krampus", krampus(), KRAMPUS_RIG, KRAMPUS_VOXEL, 2.9, 3.6), ("percht", percht(), PERCHT_RIG, PERCHT_VOXEL, 1.4, 2.2)]
        {
            let top = c.voxels.iter().map(|v| v.y).max().unwrap() as f32 * voxel;
            assert!(top > lo && top < hi, "{name}: {top} tiles tall");
            for part in 0..=rig.len() as u8 {
                assert!(c.voxels.iter().any(|v| v.part == part), "{name}: part {part}");
            }
            let built = build_rig(&c, rig, voxel);
            assert_eq!(built.parts.len(), rig.len() + 1, "{name}");
        }
        assert_eq!((krampus().width, krampus().height, krampus().depth), KRAMPUS_SIZE);
    }
}
