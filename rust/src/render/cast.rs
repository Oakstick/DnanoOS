//! The cast of Scene I, sculpted voxel by voxel like the heroes: the four
//! friends, Krampus as a shadow out of the fog, the old gatekeeper and his
//! dog Quitmer. The story's cameras come close, so they are sculpted fine,
//! at about twice the heroes' resolution: eyes with whites, irises and a
//! glint, knitted caps with ribbed cuffs, a robe hanging in folds, a beard in
//! strands, a chain of real links. Each is sculpted in parts (see
//! `Sculpt::part`) that the story animates. Models face +z; coordinates are
//! voxels, y up.

use std::f32::consts::TAU;

use glam::Vec3;

use super::carve::*;
use super::characters::{mat, Sculpt};
use super::voxels::{Collected, PartDef};

// --------------------------------------------------------------- the boys

/// What a boy wears on his head.
#[derive(Clone, Copy, Debug)]
enum Hat {
    /// A knitted cap with a ribbed, turned-up cuff, and a bobble if `pom` is some colour.
    Knit { color: u32, band: u32, pom: Option<u32> },
    /// A flat cloth cap of eight panels with a short peak.
    Flat(u32),
    /// Nothing: just his hair.
    Bare,
}

/// How one of the four friends looks.
#[derive(Clone, Copy, Debug)]
struct Kid {
    skin: u32,
    hair: u32,
    eyes: u32,
    coat: u32,
    trim: u32,
    scarf: u32,
    scarf_band: u32,
    breeches: u32,
    socks: u32,
    shoes: u32,
    hat: Hat,
    freckles: bool,
    /// Knitted mittens or leather gloves, or bare hands.
    mitts: Option<u32>,
}

const KIDS: [Kid; 4] = [
    // The one Krampus takes: a red bobble cap and a red scarf over a blue coat.
    Kid {
        skin: 0xf0c8a0,
        hair: 0x6a4024,
        eyes: 0x6a4428,
        coat: 0x34588e,
        trim: 0x284470,
        scarf: 0xc8322a,
        scarf_band: 0xf0e6d0,
        breeches: 0x5e5248,
        socks: 0xb8b0a4,
        shoes: 0x3a2618,
        hat: Hat::Knit { color: 0xc8322a, band: 0xa0241e, pom: Some(0xf4f0e6) },
        freckles: false,
        mitts: None,
    },
    // A flat cap, a yellow scarf, an olive coat, brown leather gloves.
    Kid {
        skin: 0xe2b088,
        hair: 0x1e1612,
        eyes: 0x3a2618,
        coat: 0x5e7a34,
        trim: 0x485e26,
        scarf: 0xe8b83c,
        scarf_band: 0xc88e24,
        breeches: 0x3e3a36,
        socks: 0x7a2e2a,
        shoes: 0x2a1c12,
        hat: Hat::Flat(0x6a4a2c),
        freckles: false,
        mitts: Some(0x5a3a22 | mat::LEATHER),
    },
    // Bare-headed and fair, in a brick-red coat and a grey scarf.
    Kid {
        skin: 0xf6d6b8,
        hair: 0xe6be5e,
        eyes: 0x3a6aa8,
        coat: 0x923a2c,
        trim: 0x6e2a20,
        scarf: 0x9aa0a8,
        scarf_band: 0x72787e,
        breeches: 0x4e4a5e,
        socks: 0xd8d0c4,
        shoes: 0x4a2e1a,
        hat: Hat::Bare,
        freckles: false,
        mitts: None,
    },
    // Freckled and ginger, a teal cap, scarf and mittens, a plum coat.
    Kid {
        skin: 0xf0c4a2,
        hair: 0xc4602a,
        eyes: 0x4a7a3a,
        coat: 0x5a3e6e,
        trim: 0x442e54,
        scarf: 0x2e9a8c,
        scarf_band: 0x227266,
        breeches: 0x6a5a44,
        socks: 0x4a6a8a,
        shoes: 0x2e2014,
        hat: Hat::Knit { color: 0x2e9a8c, band: 0x227266, pom: None },
        freckles: true,
        mitts: Some(0x2e9a8c | mat::CLOTH),
    },
];

/// The friends' grid: 36 wide, 64 tall, 26 deep.
pub const KID_SIZE: (i32, i32, i32) = (36, 64, 26);
/// Where each leg stands across the grid.
const KID_LEGS: [f32; 2] = [12.0, 23.0];
/// The line of lashes over a child's eye.
const LASH: u32 = 0x2a1c16;

/**
 * One of the four friends (0 is the one Krampus takes): a big round head
 * with bright eyes under a cap or a mop of hair, a striped scarf wound twice
 * round the neck with a fringed end hanging, a coat with a belt, a buttoned
 * placket, pocket flaps and turned-back cuffs, mittens or bare hands, knee
 * breeches buttoned at the knee, ribbed socks and laced boots.
 */
pub fn kid(which: usize) -> Collected {
    let k = KIDS[which % KIDS.len()];
    let (w, h, d) = KID_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (17.5f32, 12.5f32);
    let cloth = |c: u32| c | mat::CLOTH;
    let coat = cloth(k.coat);
    let coat_d = cloth(scale(k.coat, 0.8));
    let coat_l = cloth(scale(k.coat, 1.15));
    let trim = cloth(k.trim);
    let trim_d = cloth(scale(k.trim, 0.78));
    let skin = k.skin | mat::SKIN;
    let skin_d = scale(k.skin, 0.86) | mat::SKIN;
    let hair = k.hair | mat::FUR;
    let hair_d = scale(k.hair, 0.74) | mat::FUR;
    let hair_l = scale(k.hair, 1.2) | mat::FUR;
    let shoe = k.shoes | mat::LEATHER;
    let shoe_d = scale(k.shoes, 0.6) | mat::LEATHER;
    let shoe_l = scale(k.shoes, 1.45) | mat::LEATHER;
    let brass = 0xb08c4c | mat::GOLD;
    let lace = 0xa89a7c | mat::CLOTH;
    let (socks, socks_d) = (cloth(k.socks), cloth(scale(k.socks, 0.84)));
    let (breeches, breeches_d) = (cloth(k.breeches), cloth(scale(k.breeches, 0.8)));

    // ---- legs: boots with a rounded toe and laces, ribbed socks, knee breeches
    for (i, lx) in KID_LEGS.into_iter().enumerate() {
        s.part(1 + i as u8);
        let outward = if i == 0 { -1.0 } else { 1.0 };
        s.ellipsoid((lx, 1.2, 14.6), (3.7, 3.4, 6.4), 0, shoe);
        column(&mut s, lx, 12.4, 3.4, 3.6, 3, 8, shoe);
        column(&mut s, lx, 12.4, 3.9, 4.1, 8, 8, shoe_l);
        for (y, xs) in [(5, &[-1.0f32, 1.0][..]), (6, &[0.0][..]), (7, &[-1.0, 1.0][..])] {
            for dx in xs {
                face(&mut s, (lx + dx).round() as i32, y, d, 0, lace);
            }
        }
        column(&mut s, lx, 12.4, 3.0, 3.1, 9, 12, socks);
        column(&mut s, lx, 12.4, 3.3, 3.4, 13, 13, cloth(scale(k.socks, 1.1)));
        column(&mut s, lx, 12.4, 3.8, 4.0, 14, 19, breeches);
        column(&mut s, lx, 12.4, 3.95, 4.1, 14, 15, breeches_d);
        s.set((lx + outward * 3.9).round() as i32, 15, 12, brass);
    }
    s.paint(|x, y, z, c| {
        if c == shoe && (y == 0 || (y <= 1 && z <= 10)) {
            Some(shoe_d) // the sole and heel
        } else if c == shoe && z >= 19 && y >= 1 {
            Some(shoe_l) // a scuffed, shiny toe cap
        } else if c == socks && x % 2 == 0 {
            Some(socks_d) // ribbing
        } else if c == breeches && nz(x, y / 2, z) < 0.12 {
            Some(breeches_d)
        } else {
            None
        }
    });

    // ---- the coat: a skirt flaring to mid-thigh, a belt, a buttoned placket, pocket flaps
    s.part(0);
    for y in 20..=35 {
        let (mut rx, mut rz) = if y <= 25 { (10.3 - (y - 20) as f32 * 0.1, 7.6 - (y - 20) as f32 * 0.2) } else { (9.9, 6.5) };
        if y >= 33 {
            rx -= (y - 32) as f32 * 0.9;
            rz -= (y - 33) as f32 * 0.4;
        }
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = ((x as f32 - cx) / rx, (z as f32 - cz) / rz);
                if dx.abs().powf(2.6) + dz.abs().powf(2.6) <= 1.0 {
                    s.set(x, y, z, coat);
                }
            }
        }
    }
    s.paint(|x, y, z, c| {
        if c != coat {
            return None;
        }
        let (dx, dz) = (x as f32 - cx, z as f32 - cz);
        let front = dz > 3.5;
        if (26..=27).contains(&y) {
            return Some(if front && (16..=19).contains(&x) { if (17..=18).contains(&x) { trim_d } else { brass } } else { trim_d });
        }
        if front && (16..=19).contains(&x) {
            return Some(if y <= 25 && (17..=18).contains(&x) { coat_d } else { trim });
        }
        if y <= 25 {
            if dz < -3.5 && (17..=18).contains(&x) {
                return Some(coat_d); // the vent up the back
            }
            if y == 20 {
                return Some(trim);
            }
            let fold = (dz.atan2(dx) * 7.0).sin();
            return Some(if fold > 0.8 { coat_d } else if fold < -0.85 { coat_l } else { coat });
        }
        if y == 33 && dx.abs() > 6.0 {
            return Some(coat_d); // the shoulder seams
        }
        let n = nz(x, y, z);
        Some(if n < 0.06 { coat_d } else if n > 0.95 { coat_l } else { coat })
    });
    for by in [29, 32] {
        for (x, y) in [(17, by), (18, by), (17, by + 1), (18, by + 1)] {
            face(&mut s, x, y, d, 0, brass);
        }
    }
    for (x0, x1) in [(9, 13), (22, 26)] {
        for x in x0..=x1 {
            face(&mut s, x, 24, d, 0, trim);
            face(&mut s, x, 23, d, 0, trim_d);
        }
        face(&mut s, (x0 + x1) / 2, 23, d, 0, brass);
    }

    // ---- arms: sleeves with turned-back cuffs, mittens or bare hands
    for (i, sx) in [4.5f32, 30.5].into_iter().enumerate() {
        s.part(3 + i as u8);
        let (wx, inward) = if i == 0 { (sx - 0.3, 1.0) } else { (sx + 0.3, -1.0) };
        ball(&mut s, v(sx, 32.0, 12.5), Vec3::splat(3.1), coat);
        s.capsule((sx, 32.0, 12.5), (wx, 22.0, 13.2), 3.1, 2.8, coat);
        column(&mut s, wx, 13.2, 3.1, 3.1, 20, 22, trim);
        column(&mut s, wx, 13.2, 3.15, 3.15, 22, 22, trim_d);
        match k.mitts {
            Some(m) => {
                ball(&mut s, v(wx, 17.2, 13.4), v(2.5, 2.9, 2.5), m);
                ball(&mut s, v(wx + inward * 1.6, 18.2, 15.2), Vec3::splat(1.1), m);
                column(&mut s, wx, 13.3, 2.6, 2.6, 19, 19, scale(m, 0.8));
            }
            None => {
                ball(&mut s, v(wx, 17.2, 13.4), v(2.2, 2.7, 2.3), skin);
                ball(&mut s, v(wx + inward * 1.4, 18.2, 15.0), Vec3::splat(0.95), skin);
            }
        }
    }
    s.paint(|x, y, z, c| if c == coat && (x <= 7 || x >= 28) && (25..=27).contains(&y) && nz(x, y, z) > 0.5 { Some(coat_d) } else { None });
    s.paint(|_x, y, _z, c| if c == skin && y <= 15 { Some(skin_d) } else { None });

    // ---- the scarf: twice round the neck, knotted, a striped end with a fringe hanging down the front
    s.part(0);
    let (scarf, band) = (cloth(k.scarf), cloth(k.scarf_band));
    for y in 32..=36 {
        let (rx, rz, oz) = if y <= 34 { (9.2, 7.3, 0.0) } else { (8.4, 6.7, 0.5) };
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = ((x as f32 - cx) / rx, (z as f32 - cz - oz) / rz);
                let (ix, iz) = ((x as f32 - cx) / 4.4, (z as f32 - cz) / 4.0);
                if dx * dx + dz * dz <= 1.0 && ix * ix + iz * iz > 1.0 {
                    s.set(x, y, z, scarf);
                }
            }
        }
    }
    ball(&mut s, v(13.0, 33.6, 19.4), v(2.6, 2.2, 1.9), scarf);
    s.paint(|x, y, z, c| {
        if c != scarf {
            return None;
        }
        let seg = (turn(x, z, cx, cz) * 16.0) as i32 + if y >= 35 { 2 } else { 0 };
        if seg % 4 == 0 {
            Some(band)
        } else {
            None
        }
    });
    for y in 20..=31 {
        let xo = 11 + ((31 - y) as f32 * 0.09) as i32;
        for x in xo..xo + 4 {
            let Some(fz) = front(&s, x, y, d) else { continue };
            let fringe = y <= 21;
            if fringe && (x + y) % 2 == 1 {
                continue;
            }
            let c = if fringe || y % 5 == 0 { band } else { scarf };
            s.set(x, y, fz + 1, c);
            if !fringe {
                s.set(x, y, fz + 2, c);
            }
        }
    }

    // ---- the head: big and round, a neck under it
    s.part(5);
    column(&mut s, cx, 13.0, 3.6, 3.3, 34, 37, skin_d);
    s.ellipsoid((cx, 46.5, 13.5), (10.6, 10.2, 9.8), 37, skin);
    // Hair round the back and over the top, sideburns, bangs over the forehead.
    let bangs = |x: i32| 50 + (nz(x, 3, 1) * 2.6) as i32;
    s.paint(|x, y, z, c| {
        if c != skin || y < 38 {
            return None;
        }
        let dx = (x as f32 - cx).abs();
        let back = z <= 10 && y >= 40;
        let nape = z <= 12 && y >= 43;
        let top = y >= 53;
        let side = dx >= 8.2 && y >= 46 && z <= 17;
        let bang = z >= 15 && y >= bangs(x);
        if back || nape || top || side || bang {
            Some(hair)
        } else {
            None
        }
    });
    // A layer of hair standing off the skull, so it has some body.
    let mut shell = vec![];
    for y in 40..h {
        for z in 0..d {
            for x in 0..w {
                if s.get(x, y, z).is_some() {
                    continue;
                }
                let touches = NEIGHBORS.iter().any(|&(a, b, c)| s.get(x + a, y + b, z + c) == Some(hair));
                if touches && (y >= 49 || z <= 9) {
                    shell.push((x, y, z));
                }
            }
        }
    }
    for (x, y, z) in shell {
        s.set(x, y, z, hair);
    }
    if matches!(k.hat, Hat::Bare) {
        // A tousled mop: tufts standing up and out of the crown.
        let mut tufts = vec![];
        for y in 52..h - 1 {
            for z in 0..d {
                for x in 0..w {
                    if s.get(x, y, z) == Some(hair) && s.get(x, y + 1, z).is_none() && nz(x, y, z) > 0.6 {
                        let lean = if nz(z, x, y) > 0.5 { 1 } else { -1 };
                        tufts.push((x, y + 1, z));
                        if nz(y, z, x) > 0.5 {
                            tufts.push((x + lean, y + 2, z));
                        }
                    }
                }
            }
        }
        for (x, y, z) in tufts {
            s.set(x, y, z, hair_l);
        }
    }
    s.paint(|x, y, z, c| {
        if c != hair {
            return None;
        }
        let n = nz(x, y / 3, z);
        Some(if n < 0.28 { hair_d } else if n > 0.8 { hair_l } else { hair })
    });
    // Ears.
    for (ex, outer) in [(6.9f32, 6), (28.1, 29)] {
        ball(&mut s, v(ex, 44.6, 13.6), v(1.4, 2.5, 1.9), skin);
        for y in 44..=45 {
            for z in 13..=14 {
                s.recolor(outer, y, z, skin_d);
            }
        }
    }
    // The face: eyes with a glint, brows, a button nose, a small mouth, rosy cheeks.
    eye(&mut s, 12, 43, 1, d, k.eyes, LASH);
    eye(&mut s, 23, 43, -1, d, k.eyes, LASH);
    for x in [12, 13, 14, 21, 22, 23] {
        face(&mut s, x, 47, d, 0, hair_d);
    }
    for x in [17, 18] {
        face(&mut s, x, 41, d, 1, skin);
        face(&mut s, x, 40, d, 1, skin_d);
    }
    for x in 16..=19 {
        face(&mut s, x, 38, d, 0, 0x8a3a30 | mat::SKIN);
    }
    for (x, y) in [(11, 40), (12, 40), (11, 41), (12, 41), (23, 40), (24, 40), (23, 41), (24, 41)] {
        face(&mut s, x, y, d, 0, blush(skin));
    }
    if k.freckles {
        for (x, y) in [(11, 42), (13, 41), (14, 42), (16, 42), (19, 42), (21, 42), (22, 41), (24, 42), (10, 41), (25, 41)] {
            face(&mut s, x, y, d, 0, 0xc88a5a | mat::SKIN);
        }
    }

    // ---- what he wears on his head
    match k.hat {
        Hat::Knit { color, band, pom } => {
            let (color, band) = (cloth(color), cloth(band));
            let (color_d, band_d) = (cloth(scale(color, 0.86)), cloth(scale(band, 0.76)));
            s.ellipsoid((cx, 50.5, 13.5), (11.5, 8.0, 10.7), 51, color);
            column(&mut s, cx, 13.5, 11.8, 11.0, 48, 51, band);
            s.paint(|x, y, z, c| {
                if c == band {
                    // The cuff's ribs.
                    return Some(if (turn(x, z, cx, 13.5) * 44.0) as i32 % 2 == 0 { band_d } else { band });
                }
                if c == color {
                    // Rows of knitted stitches.
                    let seg = (turn(x, z, cx, 13.5) * 28.0) as i32;
                    return Some(if (seg + y / 2) % 2 == 0 { color_d } else { color });
                }
                None
            });
            if let Some(p) = pom {
                let (p, p_d) = (p | mat::FUR, scale(p, 0.86) | mat::FUR);
                let c = v(cx, 60.0, 13.2);
                ball(&mut s, c, Vec3::splat(2.7), p);
                for y in 55..h {
                    for z in 8..19 {
                        for x in 12..24 {
                            let r = Vec3::new(x as f32, y as f32, z as f32).distance(c);
                            if r > 2.7 && r < 3.7 && nz(x, y, z) > 0.55 {
                                s.set(x, y, z, if nz(z, y, x) > 0.5 { p } else { p_d });
                            }
                        }
                    }
                }
            }
        }
        Hat::Flat(c) => {
            let c = cloth(c);
            let (c_d, c_dd) = (cloth(scale(c, 0.82)), cloth(scale(c, 0.66)));
            s.ellipsoid((cx, 53.0, 13.0), (11.9, 4.8, 11.3), 51, c);
            for (y, col) in [(51, c_dd), (52, c)] {
                for z in 20..d {
                    for x in 0..w {
                        let (dx, dz) = ((x as f32 - cx) / 9.4, (z as f32 - 20.0) / 5.4);
                        if dx * dx + dz * dz <= 1.0 {
                            s.set(x, y, z, col);
                        }
                    }
                }
            }
            s.paint(|x, y, z, col| {
                let a = turn(x, z, cx, 13.0) * TAU;
                if col == c && y >= 53 && (a * 4.0).sin().abs() < 0.14 {
                    Some(c_d) // the seams between the panels
                } else {
                    None
                }
            });
            s.cuboid(17, 58, 12, 18, 58, 13, c_d);
        }
        Hat::Bare => {}
    }
    s.finish(0.025)
}

/// A boy's joints: legs from the hips, arms from the shoulders, the head (with its cap).
pub const KID_RIG: &[PartDef] = &[
    PartDef { name: "legA", boxes: &[], pivot: [12.0, 20.0, 12.5] },
    PartDef { name: "legB", boxes: &[], pivot: [23.0, 20.0, 12.5] },
    PartDef { name: "armA", boxes: &[], pivot: [4.5, 32.0, 12.5] },
    PartDef { name: "armB", boxes: &[], pivot: [30.5, 32.0, 12.5] },
    PartDef { name: "head", boxes: &[], pivot: [17.5, 37.0, 13.0] },
];

/// A boy's middle, in his grid: where Krampus's claws close on him.
pub const KID_WAIST: [f32; 3] = [17.5, 24.0, 12.5];

// ----------------------------------------------------------- the old man

mod old {
    use super::mat;
    pub const ROBE: u32 = 0x3c4658 | mat::CLOTH;
    pub const ROBE_D: u32 = 0x2a3140 | mat::CLOTH;
    pub const ROBE_L: u32 = 0x4e5a70 | mat::CLOTH;
    pub const ROBE_DIRT: u32 = 0x3a3634 | mat::CLOTH;
    pub const LINING: u32 = 0x1a1e28 | mat::CLOTH;
    pub const PATCH: u32 = 0x6a5236 | mat::CLOTH;
    pub const PATCH_D: u32 = 0x54402a | mat::CLOTH;
    pub const STITCH: u32 = 0xb8a47a | mat::CLOTH;
    pub const HOOD: u32 = 0x323a4a | mat::CLOTH;
    pub const HOOD_D: u32 = 0x262c3a | mat::CLOTH;
    pub const HOOD_IN: u32 = 0x12151c | mat::CLOTH;
    pub const ROPE: u32 = 0x9a8660 | mat::CLOTH;
    pub const ROPE_D: u32 = 0x6e5e3a | mat::CLOTH;
    pub const ROPE_L: u32 = 0xb8a47c | mat::CLOTH;
    pub const SKIN: u32 = 0xdcae8e | mat::SKIN;
    pub const SKIN_D: u32 = 0xb4846a | mat::SKIN;
    pub const NOSE: u32 = 0xd68a72 | mat::SKIN;
    pub const WRINKLE: u32 = 0xc4967a | mat::SKIN;
    pub const CHEEK: u32 = 0xd8a086 | mat::SKIN;
    pub const NOSTRIL: u32 = 0x5a3028 | mat::SKIN;
    pub const BEARD: u32 = 0xe4e2da | mat::FUR;
    pub const BEARD_D: u32 = 0xbcbab2 | mat::FUR;
    pub const BEARD_L: u32 = 0xf8f6f0 | mat::FUR;
    pub const EYE_W: u32 = 0xe8e2d4;
    pub const IRIS: u32 = 0x6a88a4;
    pub const PUPIL: u32 = 0x16161a;
    pub const IRON: u32 = 0x4a4c50 | mat::METAL;
    pub const IRON_L: u32 = 0x7a7e84 | mat::METAL;
    pub const RUST: u32 = 0x7a4a2c | mat::METAL;
    pub const STAFF: u32 = 0x5a3e22 | mat::WOOD;
    pub const STAFF_D: u32 = 0x3e2a16 | mat::WOOD;
    pub const STAFF_L: u32 = 0x74522e | mat::WOOD;
    pub const LEATHER: u32 = 0x4a3020 | mat::LEATHER;
    pub const LEATHER_D: u32 = 0x33200f | mat::LEATHER;
    pub const SANDAL: u32 = 0x5a3a1e | mat::LEATHER;
    pub const SANDAL_D: u32 = 0x3a2412 | mat::LEATHER;
}

/// The old man's grid: 60 wide, 104 tall (his staff tops his head), 40 deep.
pub const OLD_MAN_SIZE: (i32, i32, i32) = (60, 104, 40);

/**
 * The old gatekeeper: a stooped figure in a long robe of slate-blue wool,
 * hanging in folds to a ragged, muddied hem, patched and stitched, a short
 * cape over his shoulders and its hood up round a long weathered face: a
 * big red nose, shaggy white brows over heavy-lidded eyes, a wrinkled brow,
 * a drooping moustache and a white beard in strands down to his rope belt,
 * which is knotted and hung with a ring of great iron keys. One knobbly hand
 * grips a gnarled staff taller than he is, bound with leather where he holds
 * it; the other hangs out of its wide sleeve.
 */
pub fn old_man() -> Collected {
    use old::*;
    let (w, h, d) = OLD_MAN_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (29.5f32, 19.5f32);
    let stoop = |y: f32| (y - 36.0).max(0.0) * 0.08;
    let fold = |a: f32, y: f32| (a * 9.0 + y * 0.035).sin();

    // ---- the robe: a long bell of wool hanging in folds, wider at the shoulders
    for y in 0..=62 {
        let t = y as f32 / 62.0;
        let (rx, rz) = (15.2 - 3.4 * t, 11.0 - 2.0 * t);
        let oz = cz + stoop(y as f32);
        let amp = 0.07 * (1.0 - t) + 0.02;
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = (x as f32 - cx, z as f32 - oz);
                let k = 1.0 + amp * fold(dz.atan2(dx), y as f32);
                if (dx / (rx * k)).powi(2) + (dz / (rz * k)).powi(2) <= 1.0 {
                    s.set(x, y, z, ROBE);
                }
            }
        }
    }
    ball(&mut s, v(cx, 59.0, cz + stoop(59.0)), v(15.0, 5.6, 9.8), ROBE);
    s.paint(|x, y, z, c| {
        if c != ROBE {
            return None;
        }
        let (dx, dz) = (x as f32 - cx, z as f32 - cz - stoop(y as f32));
        let f = fold(dz.atan2(dx), y as f32);
        let n = nz(x, y / 2, z);
        if y <= 4 && n < 0.25 + (4 - y) as f32 * 0.18 {
            return Some(ROBE_DIRT);
        }
        Some(if f > 0.6 { ROBE_L } else if f < -0.6 || n < 0.05 { ROBE_D } else { ROBE })
    });
    // A ragged hem.
    for z in 0..d {
        for x in 0..w {
            let n = nz(x, 0, z);
            if n > 0.6 {
                s.clear(x, 0, z);
            }
            if n > 0.85 {
                s.clear(x, 1, z);
            }
        }
    }
    // A square patch on the front, cross-stitched round.
    for y in 10..=19 {
        for x in 34..=41 {
            let Some(z) = front(&s, x, y, d) else { continue };
            let edge = y == 10 || y == 19 || x == 34 || x == 41;
            let c = if edge && (x + y) % 2 == 0 { STITCH } else if nz(x, y, 5) < 0.2 { PATCH_D } else { PATCH };
            s.set(x, y, z + 1, c);
        }
    }
    // Sandalled feet peeping out under the hem, toes and all.
    for fx in [23.0f32, 36.0] {
        s.ellipsoid((fx, 0.6, 30.5), (3.4, 2.4, 4.4), 0, SANDAL);
        for k in 0..4 {
            ball(&mut s, v(fx - 2.1 + k as f32 * 1.4, 1.3, 34.0), v(0.8, 0.9, 1.0), SKIN_D);
        }
    }
    // Their soles, and a strap over each instep.
    s.paint(|_x, y, z, c| if c == SANDAL && (y == 0 || (y == 2 && (30..=32).contains(&z))) { Some(SANDAL_D) } else { None });

    // ---- a short cape over his shoulders, its edge ragged and stitched
    s.ellipsoid((cx, 60.5, cz + 2.0), (16.6, 5.4, 11.6), 55, HOOD);
    for z in 0..d {
        for x in 0..w {
            if s.get(x, 55, z) == Some(HOOD) && nz(x, 55, z) > 0.55 {
                s.clear(x, 55, z);
            }
        }
    }
    s.paint(|x, y, z, c| {
        if c != HOOD {
            return None;
        }
        Some(if y == 56 && (x + z) % 3 == 0 { STITCH } else if nz(x, y, z) < 0.1 { HOOD_D } else { HOOD })
    });

    // ---- the rope belt: twisted, knotted at his side, its frayed ends hanging
    let mut rope = vec![];
    for y in 34..=37 {
        for z in 0..d {
            for x in 0..w {
                if s.get(x, y, z).is_none() && [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dz)| s.get(x + dx, y, z + dz) == Some(ROBE) || s.get(x + dx, y, z + dz) == Some(ROBE_D) || s.get(x + dx, y, z + dz) == Some(ROBE_L)) {
                    rope.push((x, y, z));
                }
            }
        }
    }
    for (x, y, z) in rope {
        let seg = (turn(x, z, cx, cz) * 60.0) as i32;
        s.set(x, y, z, if (seg + y) % 4 < 2 { ROPE } else { ROPE_D });
    }
    let (kx, ky) = (cx + 5.5, 35.5f32);
    let kz = front(&s, kx as i32, 35, d).unwrap_or(30) as f32 + 1.2;
    ball(&mut s, v(kx, ky, kz), v(2.2, 2.2, 1.8), ROPE_D);
    for (a, b) in [(v(kx - 0.8, 34.0, kz + 0.4), v(kx - 1.6, 23.0, kz)), (v(kx + 0.8, 34.0, kz + 0.4), v(kx + 1.8, 20.0, kz - 0.4))] {
        s.capsule((a.x, a.y, a.z), (b.x, b.y, b.z), 0.95, 0.95, ROPE);
        for dy in 0..3 {
            for dx in -1..=1 {
                if nz(dx, dy, b.x as i32) > 0.35 {
                    s.set(b.x.round() as i32 + dx, b.y as i32 - 1 - dy, b.z.round() as i32, ROPE_L);
                }
            }
        }
    }
    s.paint(|x, y, _z, c| if c == ROPE && x > (kx - 3.0) as i32 && x < (kx + 3.0) as i32 && y < 33 && y % 2 == 0 { Some(ROPE_D) } else { None });
    // The ring of keys at his left hip, hung from the belt by a thong.
    let (rx, ry, rz) = (42.5f32, 27.0f32, 26);
    s.capsule((rx, 34.0, rz as f32 + 0.5), (rx, 30.0, rz as f32 + 0.5), 0.6, 0.6, LEATHER);
    for k in 0..24 {
        let a = k as f32 / 24.0 * TAU;
        s.set((rx + a.cos() * 3.0).round() as i32, (ry + a.sin() * 3.0).round() as i32, rz, IRON);
    }
    for (i, kx) in [40, 42, 44].into_iter().enumerate() {
        let top = 23;
        let len = [8, 11, 9][i];
        let z = rz + 1;
        for k in 0..12 {
            let a = k as f32 / 12.0 * TAU;
            s.set((kx as f32 + a.cos() * 1.4).round() as i32, (top as f32 - 1.5 + a.sin() * 1.4).round() as i32, z, if i == 1 { RUST } else { IRON });
        }
        for y in top - 3 - len..=top - 3 {
            s.set(kx, y, z, IRON_L);
        }
        let bot = top - 3 - len;
        s.cuboid(kx + 1, bot, z, kx + 2, bot + 2, z, IRON);
        s.clear(kx + 2, bot + 1, z);
    }

    // ---- the staff arm: a wide sleeve, a knobbly fist round a gnarled staff
    s.part(2);
    sleeve(&mut s, &[(v(15.2, 59.5, 21.5), 5.0), (v(12.2, 51.0, 25.0), 5.2), (v(10.8, 46.5, 30.5), 5.9)], ROBE, LINING);
    let (sx, sz) = (10.2f32, 34.0f32);
    let knots = [18.0f32, 37.0, 61.0, 79.0];
    for y in 0..=96 {
        let fy = y as f32;
        let (ax, az) = (sx + (fy * 0.19).sin() * 0.5, sz + (fy * 0.13 + 1.0).cos() * 0.4);
        let knot = knots.iter().any(|&k| (fy - k).abs() < 1.6);
        let wrap = (38..=41).contains(&y) || (49..=53).contains(&y);
        let r = if knot { 2.3 } else if wrap { 2.0 } else { 1.55 };
        for z in (az - r).floor() as i32..=(az + r).ceil() as i32 {
            for x in (ax - r).floor() as i32..=(ax + r).ceil() as i32 {
                let (dx, dz) = (x as f32 - ax, z as f32 - az);
                if dx * dx + dz * dz > r * r {
                    continue;
                }
                let a = dz.atan2(dx);
                let c = if wrap {
                    if ((a / TAU * 8.0) as i32 + y) % 3 == 0 { LEATHER_D } else { LEATHER }
                } else if knot {
                    if (fy - knots.iter().fold(0.0, |b, &k| if (fy - k).abs() < 1.6 { k } else { b })).abs() < 0.5 { STAFF_L } else { STAFF_D }
                } else if (a * 3.0 + fy * 0.12).sin() > 0.55 {
                    STAFF_D
                } else {
                    STAFF
                };
                s.set(x, y, z, c);
            }
        }
    }
    ball(&mut s, v(sx, 98.0, sz), v(3.1, 2.9, 3.0), STAFF);
    s.capsule((sx, 99.6, sz), (sx + 2.6, 101.2, sz), 1.5, 1.4, STAFF_D);
    s.capsule((sx + 2.6, 101.2, sz), (sx + 4.2, 99.0, sz), 1.4, 1.1, STAFF);
    s.capsule((sx + 4.2, 99.0, sz), (sx + 3.8, 97.4, sz), 1.1, 0.8, STAFF_D);
    // The fist, the knuckles down its front, the thumb over the top.
    ball(&mut s, v(sx, 44.6, sz - 0.4), v(2.6, 3.4, 2.5), SKIN);
    for (i, y) in [42.2f32, 43.8, 45.4, 47.0].into_iter().enumerate() {
        ball(&mut s, v(sx + if i % 2 == 0 { -0.4 } else { 0.4 }, y, sz + 2.1), v(1.0, 0.9, 0.9), if i % 2 == 0 { SKIN_D } else { SKIN });
    }
    ball(&mut s, v(sx, 48.2, sz + 0.6), v(1.0, 1.1, 1.3), SKIN);

    // ---- the free arm, hanging: its sleeve, a gnarled old hand
    s.part(3);
    sleeve(&mut s, &[(v(44.0, 59.5, 21.5), 5.0), (v(46.8, 50.0, 22.4), 5.3), (v(47.6, 43.0, 23.0), 6.0)], ROBE, LINING);
    ball(&mut s, v(47.6, 39.8, 23.4), v(2.5, 2.9, 2.2), SKIN);
    for k in 0..4 {
        let fx = 46.1 + k as f32;
        let tip = if k == 0 || k == 3 { 36.2 } else { 35.2 };
        s.capsule((fx, 38.6, 23.8), (fx - 0.1, tip, 24.8), 0.75, 0.6, SKIN);
    }
    s.capsule((45.4, 40.0, 25.0), (45.0, 38.0, 26.2), 0.85, 0.7, SKIN);
    s.paint(|x, y, _z, c| if c == SKIN && (37..=38).contains(&y) && (45..=50).contains(&x) { Some(SKIN_D) } else { None });

    // ---- the head: the hood up, its point hanging down his back
    s.part(1);
    column(&mut s, cx, 21.0, 4.6, 4.0, 60, 67, SKIN_D);
    s.ellipsoid((cx, 79.0, 20.5), (12.4, 13.4, 11.4), 64, HOOD);
    limb(&mut s, &[(v(cx, 86.0, 11.0), 3.6), (v(cx, 79.0, 6.6), 2.6), (v(cx, 70.0, 5.4), 1.6)], HOOD);
    let hole = |x: i32, y: i32| ((x as f32 - cx) / 7.6).powi(2) + ((y as f32 - 76.5) / 11.6).powi(2);
    for y in 64..=92 {
        for z in 25..d {
            for x in 0..w {
                if hole(x, y) < 1.0 {
                    s.clear(x, y, z);
                }
            }
        }
    }
    // Deep shadow inside it, a rolled rim round the face.
    let mut rim = vec![];
    s.paint(|x, y, z, c| {
        if c != HOOD {
            return None;
        }
        let r = hole(x, y);
        if z >= 22 && (1.0..1.32).contains(&r) {
            Some(ROBE_L)
        } else if z >= 18 && r < 1.0 {
            Some(HOOD_IN)
        } else if nz(x, y / 2, z) < 0.12 {
            Some(HOOD_D)
        } else {
            None
        }
    });
    for y in 64..=92 {
        for x in 0..w {
            if let Some(z) = front(&s, x, y, d) {
                if s.get(x, y, z) == Some(ROBE_L) && z < d - 1 {
                    rim.push((x, y, z + 1));
                }
            }
        }
    }
    for (x, y, z) in rim {
        s.set(x, y, z, ROBE_L);
    }
    // The beard, from under his chin to his belt (the face fills in behind it).
    limb(&mut s, &[(v(cx, 66.0, 26.4), 7.0), (v(cx, 58.0, 28.6), 7.6), (v(cx, 48.0, 30.0), 5.8), (v(cx, 40.0, 30.8), 3.4), (v(cx + 0.6, 35.0, 31.2), 1.2)], BEARD);
    for y in 69..h {
        for z in 0..d {
            for x in 0..w {
                let side = (x as f32 - cx).abs() > 6.2 && y < 78;
                if s.get(x, y, z) == Some(BEARD) && !side {
                    s.clear(x, y, z);
                }
            }
        }
    }
    // Wispy at its edges low down.
    for y in 34..50 {
        for z in 0..d {
            for x in 0..w {
                if s.get(x, y, z) == Some(BEARD) && open(&s, x, y, z) && nz(x, y, z) > 0.78 {
                    s.clear(x, y, z);
                }
            }
        }
    }
    // The face, long and weathered, set into the hood.
    let (fc, fr) = (v(cx, 76.0, 23.4), v(7.4, 10.4, 8.0));
    for y in 64..=88 {
        for z in 14..d {
            for x in 18..42 {
                let p = (Vec3::new(x as f32, y as f32, z as f32) - fc) / fr;
                let jaw = y < 71 && (x as f32 - cx).abs() > fr.x * (0.6 + 0.4 * (y - 64) as f32 / 7.0);
                if p.length_squared() <= 1.0 && !jaw && matches!(s.get(x, y, z), None | Some(HOOD_IN)) {
                    s.set(x, y, z, SKIN);
                }
            }
        }
    }
    // White hair escaping at his temples.
    s.paint(|x, y, _z, c| if c == SKIN && (x as f32 - cx).abs() >= 5.6 && y >= 73 { Some(BEARD) } else { None });
    // A furrowed brow; tired eyes under heavy lids, bags beneath, crow's feet.
    for y in [83, 85] {
        for x in 26..=33 {
            if (x + y) % 3 != 0 {
                face(&mut s, x, y, d, 0, WRINKLE);
            }
        }
    }
    for (x0, dir) in [(25, 1), (34, -1)] {
        let at = |k: i32| x0 + k * dir;
        for (k, c) in [EYE_W, IRIS, PUPIL, EYE_W].into_iter().enumerate() {
            face(&mut s, at(k as i32), 77, d, 0, c);
        }
        for k in 0..4 {
            face(&mut s, at(k), 78, d, 0, SKIN_D);
        }
        for k in [0, 2] {
            face(&mut s, at(k), 76, d, 0, SKIN_D);
        }
        face(&mut s, at(-2), 77, d, 0, SKIN_D);
        face(&mut s, at(-1), 76, d, 0, SKIN_D);
    }
    // Shaggy white brows jutting out over them, a few long hairs astray.
    for x in (23..=28).chain(31..=36) {
        face(&mut s, x, 79, d, 1, BEARD_L);
    }
    for x in (24..=27).chain(32..=35) {
        face(&mut s, x, 80, d, 1, BEARD);
    }
    for (x, y) in [(22, 78), (37, 78), (23, 81), (36, 81)] {
        face(&mut s, x, y, d, 1, BEARD_L);
    }
    // Ruddy cheeks, and the nose: a long bridge down to a big red bulb.
    for y in 72..=74 {
        for x in (23..=25).chain(34..=36) {
            face(&mut s, x, y, d, 0, CHEEK);
        }
    }
    s.capsule((cx, 77.5, 30.4), (cx, 73.8, 31.6), 1.2, 1.3, SKIN);
    ball(&mut s, v(cx, 72.2, 32.0), v(2.3, 2.1, 2.0), NOSE);
    for x in [28, 31] {
        face(&mut s, x, 71, d, 0, NOSTRIL);
    }
    // His upper lip, and a drooping moustache over it and over the beard.
    ball(&mut s, v(cx, 69.6, 30.4), v(4.4, 1.8, 2.2), SKIN);
    for side in [-1.0f32, 1.0] {
        s.capsule((cx + side * 0.5, 70.2, 33.0), (cx + side * 4.0, 68.6, 33.2), 1.6, 1.4, BEARD_L);
        s.capsule((cx + side * 4.0, 68.6, 33.2), (cx + side * 6.8, 65.2, 32.0), 1.4, 0.9, BEARD_L);
    }
    for y in 64..=66 {
        for x in 23..=36 {
            if let Some(z) = front(&s, x, y, d) {
                if s.get(x, y, z) == Some(BEARD) {
                    s.recolor(x, y, z, BEARD_D);
                }
            }
        }
    }
    // The beard in strands, white and silver.
    s.paint(|x, y, z, c| {
        if c != BEARD && c != BEARD_L {
            return None;
        }
        // Long wavy strands, each its own shade from top to bottom, greyer toward the tip.
        let strand = x + ((y as f32 * 0.22 + z as f32 * 0.3).sin() * 1.3).round() as i32;
        let n = nz(strand, z / 3, y / 16) - if y < 46 { 0.12 } else { 0.0 };
        Some(if c == BEARD_L || n > 0.72 { BEARD_L } else if n < 0.24 { BEARD_D } else { BEARD })
    });
    s.finish(0.025)
}

/// The old man's joints: the head (with its hood and beard), the staff arm, the free arm.
pub const OLD_MAN_RIG: &[PartDef] = &[
    PartDef { name: "head", boxes: &[], pivot: [29.5, 64.0, 22.0] },
    PartDef { name: "staffArm", boxes: &[], pivot: [15.5, 59.0, 21.0] },
    PartDef { name: "freeArm", boxes: &[], pivot: [43.5, 59.0, 21.0] },
];

// ---------------------------------------------------------------- Quitmer

mod hound {
    use super::mat;
    pub const COAT: u32 = 0x4a3a2e | mat::FUR;
    pub const COAT_D: u32 = 0x2a2019 | mat::FUR;
    pub const COAT_L: u32 = 0x6a5644 | mat::FUR;
    pub const WRINKLE: u32 = 0x3a2e24 | mat::FUR;
    pub const CHEST: u32 = 0x7a6a5a | mat::FUR;
    pub const MASK: u32 = 0x1c1614 | mat::FUR;
    pub const NOSE: u32 = 0x0e0c0c | mat::LEATHER;
    pub const NOSE_L: u32 = 0x3a3434 | mat::LEATHER;
    pub const GUM: u32 = 0x6a2228 | mat::SKIN;
    pub const TONGUE: u32 = 0xa04050 | mat::SKIN;
    pub const EAR_IN: u32 = 0x4a2a2a | mat::SKIN;
    pub const TOOTH: u32 = 0xeae2cc | mat::IVORY;
    pub const EYE: u32 = 0xffb030;
    pub const PUPIL: u32 = 0x1a0a04;
    pub const SCAR: u32 = 0x8a6a64 | mat::SKIN;
    pub const COLLAR: u32 = 0x1e1614 | mat::LEATHER;
    pub const SPIKE: u32 = 0xa8acb2 | mat::METAL;
    pub const RIVET: u32 = 0xc8ccd2 | mat::METAL;
    pub const RING: u32 = 0x5a5c62 | mat::METAL;
    pub const CLAW: u32 = 0x3a342e | mat::IVORY;
}

/// Quitmer's grid: 48 wide, 72 tall, 116 long (tail at z = 0, nose at the far end).
pub const DOG_SIZE: (i32, i32, i32) = (48, 72, 116);
/// Where his chain fastens (the ring under his collar), in his grid.
pub const DOG_COLLAR_RING: [f32; 3] = [23.5, 34.0, 96.0];
/// His eyes, in his grid (they glow).
pub const DOG_EYES: [[f32; 3]; 2] = [[16.0, 55.0, 104.5], [31.0, 55.0, 104.5]];

/// A dog's paw at (x, z): broad, four toes and their black claws.
fn paw(s: &mut Sculpt, x: f32, z: f32, coat: u32, claw: u32) {
    ball(s, v(x, 2.6, z), v(4.6, 2.8, 5.8), coat);
    for k in 0..4 {
        let tx = x - 2.7 + k as f32 * 1.8;
        ball(s, v(tx, 2.2, z + 5.0), v(1.1, 1.8, 1.3), coat);
        s.set(tx.round() as i32, 0, (z + 6.4).round() as i32, claw);
        s.set(tx.round() as i32, 1, (z + 6.4).round() as i32, claw);
    }
}

/**
 * Quitmer, the gatekeeper's dog: a mastiff the size of a pony, dark brindle
 * with a black mask and a pale blaze on his chest, a scarred and wrinkled
 * brow, cropped ears standing up, glowing amber eyes, heavy jowls over his
 * fangs, broad paws with black claws, and a collar of black leather set with
 * iron spikes and rivets, its ring waiting for the chain.
 */
pub fn dog() -> Collected {
    use hound::*;
    let (w, h, d) = DOG_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let cx = 23.5f32;

    // ---- the body: a deep chest, a long barrel, the rump, hams and shoulders
    ball(&mut s, v(cx, 34.0, 52.0), v(14.6, 13.2, 30.0), COAT);
    ball(&mut s, v(cx, 33.0, 73.0), v(16.2, 16.6, 14.0), COAT);
    ball(&mut s, v(cx, 41.0, 77.0), v(15.0, 9.4, 12.0), COAT);
    ball(&mut s, v(cx, 36.0, 31.0), v(15.0, 12.6, 12.0), COAT);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 9.6, 29.0, 30.0), v(6.4, 10.8, 9.6), COAT);
        ball(&mut s, v(cx + side * 10.4, 34.0, 77.0), v(5.6, 10.4, 7.8), COAT);
        ball(&mut s, v(cx + side * 10.0, 22.0, 75.0), Vec3::splat(3.4), COAT);
    }
    // ---- the legs: hind legs angled at the hock, forelegs straight columns, broad paws
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        let x = cx + side * 9.6;
        s.part(5 + i as u8);
        limb(&mut s, &[(v(x, 21.0, 26.5), 4.8), (v(x, 12.0, 23.5), 3.8), (v(x, 4.6, 28.4), 3.6)], COAT);
        paw(&mut s, x, 30.4, COAT_D, CLAW);
        s.part(3 + i as u8);
        limb(&mut s, &[(v(x, 24.0, 79.5), 5.4), (v(x, 14.0, 80.0), 4.8), (v(x, 5.0, 82.0), 4.2)], COAT);
        paw(&mut s, x, 84.0, COAT_D, CLAW);
    }
    // ---- the neck, thick and short, loose skin under it
    s.part(0);
    limb(&mut s, &[(v(cx, 42.0, 83.0), 11.4), (v(cx, 49.0, 92.0), 10.2)], COAT);
    ball(&mut s, v(cx, 38.5, 88.0), v(6.4, 4.6, 5.6), COAT);
    // The collar round it: leather, riveted, spiked, a ring at the bottom.
    let (cc, axis) = (v(cx, 45.85, 87.95), v(0.0, 7.0, 9.0).normalize());
    for y in 28..64 {
        for z in 76..100 {
            for x in 6..42 {
                let p = Vec3::new(x as f32, y as f32, z as f32) - cc;
                let along = p.dot(axis);
                let radial = (p - axis * along).length();
                if along.abs() <= 1.7 && (10.3..=12.0).contains(&radial) {
                    let a = (p - axis * along).normalize();
                    let ang = a.y.atan2(a.x);
                    let rivet = along.abs() < 0.8 && radial > 11.2 && ((ang / TAU * 24.0).rem_euclid(1.0) < 0.18);
                    s.set(x, y, z, if rivet { RIVET } else { COLLAR });
                }
            }
        }
    }
    let side_a = v(1.0, 0.0, 0.0);
    let side_b = axis.cross(side_a).normalize();
    for k in 0..12 {
        let a = k as f32 / 12.0 * TAU + 0.26;
        let out = side_a * a.cos() + side_b * a.sin();
        let (p0, p1) = (cc + out * 11.6, cc + out * 14.8);
        s.capsule((p0.x, p0.y, p0.z), (p1.x, p1.y, p1.z), 1.1, 0.3, SPIKE);
    }
    let [rx, ry, rz] = DOG_COLLAR_RING;
    for k in 0..16 {
        let a = k as f32 / 16.0 * TAU;
        s.set((rx + a.cos() * 2.2).round() as i32, (ry + a.sin() * 2.2).round() as i32, rz as i32, RING);
    }

    // ---- the head: a broad skull, a heavy brow, jowls, a short square muzzle
    s.part(2);
    ball(&mut s, v(cx, 55.0, 96.0), v(12.2, 11.0, 11.0), COAT);
    ball(&mut s, v(cx, 48.6, 102.0), v(10.8, 6.8, 8.4), COAT);
    ball(&mut s, v(cx, 49.2, 109.4), v(7.8, 5.6, 5.4), MASK);
    ball(&mut s, v(cx, 58.6, 104.0), v(9.4, 2.6, 3.6), COAT);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 5.4, 45.2, 108.4), v(3.4, 3.4, 4.8), MASK);
    }
    // The black mask over the face, frayed at its edge.
    s.paint(|x, y, z, c| if c == COAT && y < 60 && (z >= 104 || (z >= 101 && nz(x, y, z) > 0.5)) { Some(MASK) } else { None });
    // A broad wet nose, its nostrils.
    ball(&mut s, v(cx, 52.2, 114.0), v(4.2, 2.8, 1.8), NOSE);
    s.paint(|x, y, z, c| if c == NOSE && y >= 54 && z >= 114 && (x as f32 - cx).abs() < 1.5 && (x + y) % 2 == 0 { Some(NOSE_L) } else { None });
    for x in [21, 26] {
        face(&mut s, x, 51, d, 0, 0x050404);
    }
    // Teeth under the lip: fangs at the corners, a row between.
    for x in [20, 27] {
        s.cuboid(x, 42, 112, x, 44, 112, TOOTH);
    }
    for x in 21..=26 {
        s.set(x, 43, 112, TOOTH);
        s.set(x, 44, 112, GUM);
    }
    // Amber eyes under a heavy brow, a lid over each.
    for e in DOG_EYES {
        let ex = e[0] as i32;
        for y in 54..=55 {
            for x in ex - 1..=ex + 1 {
                face(&mut s, x, y, d, 0, if x == ex { PUPIL } else { EYE });
            }
        }
        for x in ex - 2..=ex + 2 {
            face(&mut s, x, 56, d, 0, COAT_D);
        }
    }
    // Wrinkles across the brow, and a pale scar over the left eye.
    for (y, x0, x1) in [(61, 19, 28), (63, 20, 27)] {
        for x in x0..=x1 {
            if (x + y) % 4 != 0 {
                face(&mut s, x, y, d, 0, WRINKLE);
            }
        }
    }
    for (x, y) in [(12, 63), (13, 62), (13, 61), (14, 60), (14, 59), (15, 58), (15, 53), (16, 52), (16, 51)] {
        face(&mut s, x, y, d, 0, SCAR);
    }
    // Cropped ears standing up.
    for side in [-1.0f32, 1.0] {
        limb(&mut s, &[(v(cx + side * 8.8, 63.0, 94.0), 3.4), (v(cx + side * 10.0, 68.0, 92.4), 1.9), (v(cx + side * 10.4, 70.8, 91.6), 0.8)], COAT_D);
    }
    let mut inner = vec![];
    for y in 63..=68 {
        for z in 92..100 {
            for x in 0..w {
                if s.get(x, y, z) == Some(COAT_D) && (x as f32 - cx).abs() > 7.5 && s.get(x, y, z + 1).is_none() && (x as f32 - cx).abs() < 10.5 {
                    inner.push((x, y, z));
                }
            }
        }
    }
    for (x, y, z) in inner {
        s.recolor(x, y, z, EAR_IN);
    }
    // ---- the lower jaw (a part of its own, so it can snap), a tongue inside it
    s.part(1);
    ball(&mut s, v(cx, 40.4, 106.0), v(6.6, 2.8, 7.0), MASK);
    ball(&mut s, v(cx, 42.2, 106.4), v(3.8, 1.0, 4.6), TONGUE);
    for x in [21, 26] {
        s.cuboid(x, 42, 111, x, 43, 111, TOOTH);
    }
    for x in 22..=25 {
        s.set(x, 42, 112, GUM);
    }
    // ---- a tail curling up
    s.part(7);
    limb(&mut s, &[(v(cx, 40.0, 21.0), 4.0), (v(cx, 45.0, 11.0), 3.2), (v(cx, 52.0, 5.0), 2.6), (v(cx, 58.0, 4.0), 1.8)], COAT);
    s.part(0);

    // ---- brindle stripes over it all, dark down the spine, a pale belly and chest
    s.paint(|x, y, z, c| {
        if c != COAT {
            return None;
        }
        let (fx, fy, fz) = (x as f32, y as f32, z as f32);
        let stripe = ((fz * 0.45 + fy * 0.28 + (fx * 0.35).sin() * 1.6) * 0.9).sin();
        let n = nz(x, y, z);
        if fy > 44.0 && (fx - cx).abs() < 3.0 && fz > 22.0 && fz < 88.0 {
            return Some(COAT_D);
        }
        if fz > 84.0 && fy < 40.0 && (fx - cx).abs() < 5.5 {
            return Some(CHEST);
        }
        if fz > 89.0 && fy > 44.0 {
            return Some(if n < 0.35 { COAT_D } else { COAT }); // his head dark and solid, no stripes
        }
        if fy < 24.0 && fz > 40.0 && fz < 72.0 {
            return Some(COAT_L);
        }
        Some(if stripe > 0.62 { COAT_D } else if n > 0.92 { COAT_L } else { COAT })
    });
    shag(&mut s, (w, h, d), COAT, COAT_D, COAT_L, 0.07, 2);
    s.finish(0.03)
}

/// Quitmer's joints.
pub const DOG_RIG: &[PartDef] = &[
    PartDef { name: "jaw", boxes: &[], pivot: [23.5, 44.0, 98.0] },
    PartDef { name: "head", boxes: &[], pivot: [23.5, 48.0, 90.0] },
    PartDef { name: "foreA", boxes: &[], pivot: [13.9, 17.0, 81.0] },
    PartDef { name: "foreB", boxes: &[], pivot: [33.1, 17.0, 81.0] },
    PartDef { name: "hindA", boxes: &[], pivot: [13.9, 17.0, 27.0] },
    PartDef { name: "hindB", boxes: &[], pivot: [33.1, 17.0, 27.0] },
    PartDef { name: "tail", boxes: &[], pivot: [23.5, 41.0, 20.0] },
];

// ---------------------------------------------------------------- Krampus

mod shade {
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
    pub const TONGUE: u32 = 0x9a1a2a | mat::SKIN;
    pub const TONGUE_D: u32 = 0x6a0e1a | mat::SKIN;
    pub const GUM: u32 = 0x4a0e16 | mat::SKIN;
    pub const MAW: u32 = 0x0a0606;
    pub const FANG: u32 = 0xe4d8bc | mat::IVORY;
    pub const EYE: u32 = 0xff3a1a;
    pub const EYE_L: u32 = 0xff8a4a;
    pub const SLIT: u32 = 0x1a0402;
    pub const WICKER: u32 = 0x4a3622 | mat::WOOD;
    pub const WICKER_M: u32 = 0x3c2c1c | mat::WOOD;
    pub const WICKER_D: u32 = 0x2a1e14 | mat::WOOD;
    pub const BIRCH: u32 = 0xb8b0a0 | mat::WOOD;
    pub const BIRCH_D: u32 = 0x2a2624 | mat::WOOD;
    pub const CHAIN: u32 = 0x3c3e44 | mat::METAL;
    pub const CHAIN_L: u32 = 0x5a5e66 | mat::METAL;
    pub const BELL: u32 = 0x8e7034 | mat::GOLD;
    pub const BELL_D: u32 = 0x5e4a22 | mat::GOLD;
    pub const STRAP: u32 = 0x2a1e16 | mat::LEATHER;
}

/// Krampus's grid: 92 wide, 128 tall (horns and all), 84 deep.
pub const KRAMPUS_SIZE: (i32, i32, i32) = (92, 128, 84);
/// His eyes, in his grid.
pub const KRAMPUS_EYES: [[f32; 3]; 2] = [[40.5, 98.0, 65.5], [50.5, 98.0, 65.5]];
/// The claws of his right hand (on the viewer's left as he faces you), where he holds what he takes.
pub const KRAMPUS_HAND: [f32; 3] = [17.5, 44.0, 61.0];

/// A link of chain at `p` along `t`: a bar seen side-on, or a ring seen face-on, turn about.
fn link(s: &mut Sculpt, p: Vec3, t: Vec3, bar: bool, c: u32, hi: u32) {
    if bar {
        let (a, b) = (p - t * 1.6, p + t * 1.6);
        s.capsule((a.x, a.y, a.z), (b.x, b.y, b.z), 0.8, 0.8, c);
    } else {
        for k in 0..14 {
            let a = k as f32 / 14.0 * TAU;
            let q = p + t * a.cos() * 1.9 + Vec3::Y * a.sin() * 1.3;
            s.set(q.x.round() as i32, q.y.round() as i32, q.z.round() as i32, if k < 7 { hi } else { c });
        }
    }
}

/// A bell hanging from `top`: its loop, the dome, the flared lip, the dark mouth, the clapper.
fn bell(s: &mut Sculpt, top: Vec3, d: i32, c: u32, dark: u32, mouth: u32) {
    let (tx, tz) = (top.x.round() as i32, top.z.round() as i32);
    s.set(tx, top.y.round() as i32, tz, dark);
    let o = top - Vec3::new(0.0, 3.4, 0.0);
    s.ellipsoid((o.x, o.y, o.z), (2.4, 2.8, 2.4), (o.y - 1.0).round() as i32, c);
    column(s, o.x, o.z, 3.1, 3.1, (o.y - 2.0).round() as i32, (o.y - 1.0).round() as i32, dark);
    for dx in -1..=1 {
        for dz in -1..=1 {
            s.set(o.x.round() as i32 + dx, (o.y - 2.0).round() as i32, o.z.round() as i32 + dz, mouth);
        }
    }
    s.set(tx, (o.y - 3.0).round() as i32, tz, dark);
    // The slot up its front.
    if let Some(z) = front(s, tx, o.y.round() as i32, d) {
        s.recolor(tx, o.y.round() as i32, z, mouth);
        s.recolor(tx, o.y.round() as i32 + 1, z, mouth);
    }
}

/// A birch switch from `base` to `tip`: pale bark ringed with black, a twig off its side.
fn birch(s: &mut Sculpt, base: Vec3, tip: Vec3, seed: i32) {
    use shade::{BIRCH, BIRCH_D};
    let len = base.distance(tip);
    let steps = (len * 2.0) as i32;
    for k in 0..=steps {
        let t = k as f32 / steps as f32;
        let p = base.lerp(tip, t);
        let r = 0.95 - 0.45 * t;
        let c = if (k / 3 + seed) % 3 == 1 { BIRCH_D } else { BIRCH };
        s.ellipsoid((p.x, p.y, p.z), (r, r, r), 0, c);
    }
    let m = base.lerp(tip, 0.6);
    let out = Vec3::new(if seed % 2 == 0 { 1.0 } else { -1.0 }, 0.9, -0.3).normalize() * 4.0;
    s.capsule((m.x, m.y, m.z), (m.x + out.x, m.y + out.y, m.z + out.z), 0.5, 0.3, BIRCH);
}

/// A horn along `knots`, ridged with rings along its length, its tip paler.
fn horn(s: &mut Sculpt, knots: &[(Vec3, f32)], base: u32, ring: u32, tip: u32) {
    let total: f32 = knots.windows(2).map(|w| w[0].0.distance(w[1].0)).sum();
    let mut run = 0.0;
    for w in knots.windows(2) {
        let ((a, ra), (b, rb)) = (w[0], w[1]);
        let len = a.distance(b);
        let steps = (len * 2.5).ceil() as i32;
        for k in 0..=steps {
            let t = k as f32 / steps as f32;
            let p = a.lerp(b, t);
            let along = run + len * t;
            let u = along / total;
            let ridge = (along / 2.4).fract() < 0.32 && u < 0.8;
            let r = ra + (rb - ra) * t + if ridge { 0.45 } else { 0.0 };
            let c = if u > 0.8 { tip } else if ridge { ring } else { base };
            s.ellipsoid((p.x, p.y, p.z), (r, r, r), 0, c);
        }
        run += len;
    }
}

/**
 * Krampus as the boys see him: a shadow come out of the fog, long black
 * shaggy fur on a hunched man-goat, long sinewy arms with jointed fingers and
 * hooked claws hanging almost to his knees, cloven hooves, a goat's face
 * thrust forward with its jaw hanging open, fangs and a long red tongue
 * lolling out, ember eyes with a goat's slit pupils, great ridged horns
 * curling back and round, chains of real links wound round him hung with
 * bells, and a woven wicker basket on his back for children, bristling with
 * birch switches.
 */
pub fn krampus_shade() -> Collected {
    use shade::*;
    let (w, h, d) = KRAMPUS_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let cx = 45.5f32;

    // ---- goat legs: cloven hooves, thin shanks back up to the hock, shaggy thighs forward to the hip
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        let x = cx + side * 10.0;
        s.part(4 + i as u8);
        s.ellipsoid((x, 3.2, 45.0), (4.8, 3.6, 6.0), 0, HOOF);
        s.paint(|_x, y, _z, c| if c == HOOF && y >= 5 { Some(HOOF_L) } else { None });
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
    // ---- hips, belly, chest, and heavy shoulders, all hunched forward
    s.part(0);
    ball(&mut s, v(cx, 51.0, 37.0), v(16.8, 9.2, 12.4), FUR);
    ball(&mut s, v(cx, 62.0, 39.0), v(15.6, 12.4, 12.8), FUR);
    ball(&mut s, v(cx, 74.0, 43.0), v(18.4, 12.0, 13.2), FUR);
    ball(&mut s, v(cx, 80.0, 45.0), v(22.4, 7.2, 11.2), FUR);
    ball(&mut s, v(cx, 82.0, 38.0), v(14.0, 8.0, 9.0), FUR);
    limb(&mut s, &[(v(cx, 79.0, 46.0), 8.4), (v(cx, 87.0, 53.0), 7.2)], FUR);

    // ---- the basket on his back: stakes and weavers over and under, a braided rim, leather bands
    let (bx0, bx1, bz0, bz1, by0, by1) = (31, 60, 14, 28, 58, 90);
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
            let rim = x <= bx0 || x >= bx1 || z <= bz0 || z >= bz1;
            if rim {
                for y in by1 + 1..=by1 + 2 {
                    s.set(x, y, z, if (x + z + y) % 3 == 0 { WICKER_D } else { WICKER });
                }
            }
        }
    }
    for i in 0..9 {
        let fi = i as f32;
        let base = v(35.0 + fi * 2.8, 80.0, 18.0 + (i % 3) as f32 * 2.5);
        let tip = v(33.0 + fi * 3.4 + (fi * 1.7).sin() * 2.0, 104.0 + (i % 4) as f32 * 2.2, 15.0 + (i % 3) as f32 * 2.0);
        birch(&mut s, base, tip, i);
    }
    for side in [-1.0f32, 1.0] {
        limb(&mut s, &[(v(cx + side * 10.0, 88.0, 27.0), 1.3), (v(cx + side * 12.0, 87.0, 40.0), 1.3), (v(cx + side * 12.4, 72.0, 53.0), 1.3)], STRAP);
        s.cuboid((cx + side * 12.4) as i32 - 1, 70, 54, (cx + side * 12.4) as i32 + 1, 72, 54, BELL);
    }

    // ---- chains round his waist and across his chest, link by link, hung with bells
    // (height, half-width, half-depth, middle's depth, tilt)
    let chains = [(54.0f32, 16.8f32, 12.6f32, 37.5f32, 0.0f32), (72.0, 19.4, 14.2, 43.0, 0.35)];
    let on_chain = |(cy, crx, crz, oz, tilt): (f32, f32, f32, f32, f32), a: f32| v(cx + a.cos() * crx, cy + (a * 2.0).sin() * 1.2 + a.cos() * tilt * 12.0, oz + a.sin() * crz);
    for ch in chains {
        let n = 28;
        for i in 0..n {
            let a = i as f32 / n as f32 * TAU;
            let t = (on_chain(ch, a + 0.01) - on_chain(ch, a - 0.01)).normalize();
            link(&mut s, on_chain(ch, a), t, i % 2 == 0, CHAIN, CHAIN_L);
        }
    }
    for (dx, which) in [(-8.0f32, 0), (7.0, 0), (0.0, 0), (-13.0, 1), (12.0, 1)] {
        let ch = chains[which];
        let a = (dx / ch.1).clamp(-1.0, 1.0).acos();
        bell(&mut s, on_chain(ch, a) + v(0.0, -1.2, 1.4), d, BELL, BELL_D, MAW);
    }

    // ---- arms: long and hanging forward, shaggy to the elbow, sinewy below, clawed hands
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(2 + i as u8);
        let sh = v(cx + side * 21.0, 81.0, 44.0);
        let elbow = v(cx + side * 26.0, 66.0, 50.0);
        let wrist = v(cx + side * 28.0, 51.0, 57.0);
        limb(&mut s, &[(sh, 6.4), (elbow, 5.4)], FUR);
        ball(&mut s, elbow, Vec3::splat(5.8), FUR_L);
        limb(&mut s, &[(elbow, 4.6), (wrist, 3.8)], HIDE_D);
        ball(&mut s, wrist + v(0.0, -2.4, 1.2), v(4.0, 4.4, 3.6), HIDE);
        for (k, (ox, oz)) in [(-2.4f32, 0.4f32), (-0.8, 1.2), (0.8, 1.2), (2.4, 0.4)].into_iter().enumerate() {
            let base = wrist + v(ox, -5.4, 1.6 + oz);
            let mid = base + v(ox * 0.25, -4.0, 2.2);
            let tip = mid + v(ox * 0.1, -2.6, 2.8 - k as f32 * 0.2);
            limb(&mut s, &[(base, 1.35), (mid, 1.1), (tip, 0.8)], HIDE);
            ball(&mut s, mid, Vec3::splat(1.3), HIDE_D);
            let claw = tip + v(0.0, -1.3, 2.6);
            s.capsule((tip.x, tip.y, tip.z), (claw.x, claw.y, claw.z), 0.75, 0.2, CLAW);
        }
        let tb = wrist + v(-side * 3.0, -3.0, 2.0);
        let tt = tb + v(-side * 1.0, -3.4, 2.4);
        limb(&mut s, &[(tb, 1.3), (tt, 0.9)], HIDE);
        let tc = tt + v(0.0, -1.0, 2.2);
        s.capsule((tt.x, tt.y, tt.z), (tc.x, tc.y, tc.z), 0.7, 0.2, CLAW);
    }

    // ---- the head, thrust forward and low: mane, skull, a goat's snout, the jaw hanging open
    s.part(1);
    ball(&mut s, v(cx, 93.0, 48.0), v(12.4, 12.4, 9.2), FUR);
    ball(&mut s, v(cx, 95.0, 57.0), v(9.6, 9.2, 9.6), HIDE);
    ball(&mut s, v(cx, 90.4, 66.4), v(6.2, 5.4, 6.8), HIDE);
    ball(&mut s, v(cx, 91.2, 72.4), v(4.6, 3.4, 1.8), HIDE_D);
    ball(&mut s, v(cx, 84.8, 64.8), v(5.8, 2.6, 6.4), HIDE_D);
    // Wrinkles up the snout.
    s.paint(|_x, y, z, c| if c == HIDE && z > 62 && y > 92 && y % 2 == 0 { Some(HIDE_D) } else { None });
    // The maw, gums round it, fangs above and below.
    for z in 62..=72 {
        for x in (cx - 3.5) as i32..=(cx + 3.5) as i32 {
            s.set(x, 86, z, MAW);
            s.set(x, 87, z, if z >= 71 { GUM } else { MAW });
        }
    }
    for x in [42, 49] {
        s.cuboid(x, 84, 71, x, 86, 71, FANG);
        s.set(x, 85, 72, FANG);
    }
    for x in [43, 48] {
        s.cuboid(x, 87, 70, x, 88, 70, FANG);
    }
    for x in [44, 47] {
        face(&mut s, x, 91, d, 0, MAW);
    }
    // The long tongue lolling out and down over his chest, a groove down its middle.
    limb(&mut s, &[(v(cx, 86.0, 68.0), 2.6), (v(cx, 83.0, 73.0), 2.2), (v(cx + 1.2, 77.0, 75.6), 1.8), (v(cx, 70.0, 76.4), 1.4), (v(cx - 0.6, 65.0, 75.4), 1.0)], TONGUE);
    s.paint(|x, y, z, c| if c == TONGUE && (x == 45 || x == 46) && y < 84 && z > 74 && nz(x, y, z) > 0.3 { Some(TONGUE_D) } else { None });
    // A heavy brow over ember eyes with a goat's slit pupils.
    ball(&mut s, v(cx, 99.6, 63.6), v(8.8, 2.2, 3.2), HIDE_D);
    for e in KRAMPUS_EYES {
        let ex = e[0];
        for y in 97..=99 {
            for x in (ex - 1.5) as i32..=(ex + 1.5) as i32 {
                for z in 66..d {
                    s.clear(x, y, z);
                }
                s.set(x, y, 64, HIDE_D);
                let mid = (x as f32 - ex).abs() < 1.0;
                s.set(x, y, 65, if y == 98 && mid { SLIT } else if y == 99 { EYE_L } else { EYE });
            }
        }
    }
    for side in [-1.0f32, 1.0] {
        // Goat's ears sticking out.
        limb(&mut s, &[(v(cx + side * 8.4, 97.0, 54.0), 2.8), (v(cx + side * 13.6, 99.6, 52.0), 1.8), (v(cx + side * 17.6, 101.6, 50.0), 0.8)], HIDE_D);
        // Great horns: up and back, curling round and forward at the tips, ridged along their length.
        let k = |dx: f32, y: f32, z: f32, r: f32| (v(cx + side * dx, y, z), r);
        let knots = [k(5.2, 102.0, 57.0, 4.2), k(9.6, 110.0, 53.0, 3.8), k(15.6, 117.0, 47.0, 3.2), k(22.0, 121.0, 43.0, 2.7), k(28.0, 120.0, 42.0, 2.2), k(32.4, 114.0, 44.8, 1.8), k(33.0, 108.0, 49.6, 1.3), k(30.4, 104.8, 53.6, 0.9)];
        horn(&mut s, &knots, HORN, HORN_D, HORN_TIP);
    }
    limb(&mut s, &[(v(cx, 82.0, 62.0), 3.6), (v(cx, 76.0, 61.0), 2.2), (v(cx, 71.0, 59.6), 1.0)], FUR_L); // a goatee
    s.part(0);

    // ---- long shaggy fur over it all, blacker in the hollows, paler at the tips
    s.paint(|x, y, z, c| {
        if c != FUR {
            return None;
        }
        let n = nz(x, y / 3, z);
        Some(if n < 0.25 { FUR_D } else if n > 0.86 { FUR_L } else { FUR })
    });
    shag(&mut s, (w, h, d), FUR, FUR_D, FUR_TIP, 0.3, 3);
    shag(&mut s, (w, h, d), FUR_D, FUR, FUR_TIP, 0.16, 2);
    s.finish(0.04)
}

/// Krampus's joints: head and horns, both arms from the shoulder, both legs from the hip.
pub const KRAMPUS_RIG: &[PartDef] = &[
    PartDef { name: "head", boxes: &[], pivot: [45.5, 82.0, 48.0] },
    PartDef { name: "armR", boxes: &[], pivot: [24.5, 81.0, 44.0] },
    PartDef { name: "armL", boxes: &[], pivot: [66.5, 81.0, 44.0] },
    PartDef { name: "legR", boxes: &[], pivot: [35.5, 44.0, 38.0] },
    PartDef { name: "legL", boxes: &[], pivot: [55.5, 44.0, 38.0] },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::voxels::build_rig;

    #[test]
    fn the_cast_is_cut_into_its_parts() {
        let models: [(&str, Collected, &[PartDef]); 7] = [
            ("kid 0", kid(0), KID_RIG),
            ("kid 1", kid(1), KID_RIG),
            ("kid 2", kid(2), KID_RIG),
            ("kid 3", kid(3), KID_RIG),
            ("old man", old_man(), OLD_MAN_RIG),
            ("dog", dog(), DOG_RIG),
            ("krampus", krampus_shade(), KRAMPUS_RIG),
        ];
        for (name, m, defs) in models {
            let rig = build_rig(&m, defs, 0.025);
            assert_eq!(rig.parts.len(), defs.len() + 1, "{name}");
            let total: usize = rig.parts.iter().map(|p| p.cubes.len()).sum();
            assert_eq!(total, m.voxels.len(), "{name}: every voxel in exactly one part");
            for p in &rig.parts {
                assert!(!p.cubes.is_empty(), "{name}: part {} is empty", p.name);
            }
        }
    }

    #[test]
    fn the_cast_stays_within_its_grids() {
        for (name, m, (w, h, d)) in [("kid", kid(0), KID_SIZE), ("old man", old_man(), OLD_MAN_SIZE), ("dog", dog(), DOG_SIZE), ("krampus", krampus_shade(), KRAMPUS_SIZE)] {
            assert_eq!((m.width, m.height, m.depth), (w, h, d), "{name}");
            eprintln!("{name}: {} voxels", m.voxels.len());
        }
    }
}
