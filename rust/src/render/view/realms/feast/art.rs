//! The Rotting Feast's models, sculpted in voxels and merged into boxes (`vault_decor::merge`): the
//! food (a glossy roast fowl, a scored ham, a lattice pie, a tiered cake, a bowl of fruit, a loaf,
//! a pudding, sausages, a wheel of cheese, a gingerbread house), the silver dome, the seams' teeth
//! (carved marble, copper-sheathed, and at last real, wet and rooted in gum), a copper cauldron, an
//! oven like a mouth, a high-backed chair, hanging hams and sausages, a salt barrel, a rib, a pile
//! of digested armour, a chopping block with its cleaver, a copper pan and a purge font. Each is
//! centred on x/z with its base at y = 0 and faces +z.

use super::super::super::vault_decor::{merge, Merged};
use crate::logic::realms::feast::FoodKind;
use crate::render::carve::{ball, limb, nz, v};
use crate::render::characters::{mat, Sculpt};
use glam::Vec3;
use std::f32::consts::{PI, TAU};

/// Polished marble as a voxel material (shader kind 10).
pub const MARBLE: u32 = 10 << 24;

pub mod pal {
    use super::{mat, MARBLE};
    pub const SILVER: u32 = 0xd8dce4 | mat::METAL;
    pub const SILVER_L: u32 = 0xf4f6fa | mat::METAL;
    pub const SILVER_D: u32 = 0x9aa0aa | mat::METAL;
    pub const COPPER: u32 = 0xc8743a | mat::METAL;
    pub const COPPER_L: u32 = 0xeaa064 | mat::METAL;
    pub const COPPER_D: u32 = 0x8a4a22 | mat::METAL;
    pub const VERDIGRIS: u32 = 0x5aa08a | mat::METAL;
    pub const IRON: u32 = 0x3a3c42 | mat::METAL;
    pub const GOLD: u32 = 0xf2c53d | mat::GOLD;
    pub const MARBLE_W: u32 = 0xf0e8da | MARBLE;
    pub const MARBLE_D: u32 = 0xc8bca8 | MARBLE;
    pub const VEIN_RED: u32 = 0xa02a2a | MARBLE;
    pub const GUM: u32 = 0xc04a5a | mat::SLIME;
    pub const GUM_D: u32 = 0x8a2a3a | mat::SLIME;
    pub const TOOTH: u32 = 0xf0e6c8 | mat::IVORY;
    pub const TOOTH_D: u32 = 0xc8b88e | mat::IVORY;
    pub const TOOTH_ROOT: u32 = 0xa88e62 | mat::IVORY;
    pub const ROAST: u32 = 0xa8561e | mat::SLIME;
    pub const ROAST_L: u32 = 0xd8862e | mat::SLIME;
    pub const ROAST_D: u32 = 0x6a2e10 | mat::SLIME;
    pub const FRILL: u32 = 0xf4f0e6 | mat::CLOTH;
    pub const HAM: u32 = 0xc0604a | mat::SLIME;
    pub const HAM_D: u32 = 0x8a3a24 | mat::SLIME;
    pub const HAM_FAT: u32 = 0xf0d8c0 | mat::SKIN;
    pub const BONE: u32 = 0xf0e6d0 | mat::IVORY;
    pub const CRUST: u32 = 0xd8a04a | mat::WOOD;
    pub const CRUST_L: u32 = 0xf0c870 | mat::WOOD;
    pub const CRUST_D: u32 = 0xa06a26 | mat::WOOD;
    pub const JAM: u32 = 0xa01a2a | mat::SLIME;
    pub const ICING: u32 = 0xfaf4f0 | mat::SKIN;
    pub const PINK: u32 = 0xf0a0b8 | mat::SKIN;
    pub const CHERRY: u32 = 0xd0102a | mat::SLIME;
    pub const APPLE: u32 = 0xc82020 | mat::SLIME;
    pub const ORANGE: u32 = 0xf08a1a | mat::SLIME;
    pub const GRAPE: u32 = 0x5a2a6a | mat::SLIME;
    pub const PEAR: u32 = 0xb8c84a | mat::SLIME;
    pub const LEAF: u32 = 0x3a7a2a | mat::CLOTH;
    pub const LOAF: u32 = 0xc88a3a | mat::WOOD;
    pub const LOAF_L: u32 = 0xe8b860 | mat::WOOD;
    pub const PUDDING: u32 = 0x4a2214 | mat::SLIME;
    pub const CUSTARD: u32 = 0xf8e8b0 | mat::SLIME;
    pub const HOLLY: u32 = 0x2a6a2a | mat::CLOTH;
    pub const SAUSAGE: u32 = 0x9a4a2a | mat::SLIME;
    pub const SAUSAGE_D: u32 = 0x6a2e1a | mat::SLIME;
    pub const CHEESE: u32 = 0xf0c840 | mat::SKIN;
    pub const CHEESE_D: u32 = 0xc89a28 | mat::SKIN;
    pub const RIND: u32 = 0xd88a2a | mat::LEATHER;
    pub const GINGER: u32 = 0xa8622a | mat::WOOD;
    pub const GINGER_D: u32 = 0x7a4218 | mat::WOOD;
    pub const CANDY_R: u32 = 0xe02a3a | mat::SLIME;
    pub const CANDY_G: u32 = 0x3ab04a | mat::SLIME;
    pub const BOARD: u32 = 0x7a5232 | mat::WOOD;
    pub const WOOD: u32 = 0x5a3a22 | mat::WOOD;
    pub const WOOD_D: u32 = 0x3a2414 | mat::WOOD;
    pub const VELVET: u32 = 0x8a1a22 | mat::CLOTH;
    pub const STONE: u32 = 0x8a8478 | MARBLE;
    pub const STONE_D: u32 = 0x5a564e | MARBLE;
    pub const BRICK: u32 = 0x9a4a2e;
    pub const BRICK_D: u32 = 0x6a2e1c;
    pub const LIP: u32 = 0xb03040 | mat::SKIN;
    pub const LIP_D: u32 = 0x701a26 | mat::SKIN;
    pub const FIRE: u32 = 0xffa030 | mat::GLOW;
    pub const FIRE_L: u32 = 0xffe080 | mat::GLOW;
    pub const SOOT: u32 = 0x1a1412;
    pub const STEW: u32 = 0x7a4a1a | mat::SLIME;
    pub const SALT: u32 = 0xe8e4dc | mat::IVORY;
    pub const RUST: u32 = 0x6a3a22 | mat::METAL;
    pub const ACID: u32 = 0xb0e040 | mat::GLOW;
    pub const FLESH: u32 = 0xa83a42 | mat::SLIME;
    pub const FLESH_D: u32 = 0x6a1a24 | mat::SLIME;
}
use pal::*;

/// Food is sculpted at this voxel: about half a tile across.
pub const FOOD_VOXEL: f32 = 0.03;
/// The teeth's voxel.
pub const TOOTH_VOXEL: f32 = 0.034;

/// Fills an upright cylinder (or a cone, with `r1` at the top) from `y0` to `y1` round (cx, cz).
#[allow(clippy::too_many_arguments)]
fn drum(s: &mut Sculpt, cx: f32, cz: f32, y0: i32, y1: i32, r0: f32, r1: f32, c: u32) {
    for y in y0..=y1 {
        let t = if y1 > y0 { (y - y0) as f32 / (y1 - y0) as f32 } else { 0.0 };
        let r = r0 + (r1 - r0) * t;
        let n = r.ceil() as i32 + 1;
        for z in (cz as i32 - n)..=(cz as i32 + n) {
            for x in (cx as i32 - n)..=(cx as i32 + n) {
                if (x as f32 - cx).powi(2) + (z as f32 - cz).powi(2) <= r * r {
                    s.set(x, y, z, c);
                }
            }
        }
    }
}

// ------------------------------------------------------------------ food

/// A dish of `kind`, without its platter.
pub fn food(kind: FoodKind) -> Merged {
    let (w, h, d) = (18, 16, 18);
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (8.5, 8.5);
    match kind {
        FoodKind::Roast => {
            // A fowl on its back, legs up, glazed to a mirror.
            ball(&mut s, v(cx, 4.0, cz), v(5.6, 3.8, 4.4), ROAST);
            ball(&mut s, v(cx, 6.2, cz - 0.5), v(4.2, 2.2, 3.4), ROAST_L);
            for side in [-1.0f32, 1.0] {
                limb(&mut s, &[(v(cx + side * 3.4, 4.6, cz + 2.4), 1.8), (v(cx + side * 4.4, 7.4, cz + 5.4), 1.2)], ROAST);
                limb(&mut s, &[(v(cx + side * 4.4, 7.4, cz + 5.4), 0.6), (v(cx + side * 4.8, 8.6, cz + 6.8), 0.6)], BONE);
                ball(&mut s, v(cx + side * 4.8, 9.0, cz + 7.2), v(1.0, 0.9, 1.0), FRILL);
                // A wing tucked in.
                ball(&mut s, v(cx + side * 4.8, 3.6, cz - 1.0), v(1.4, 1.6, 2.4), ROAST_D);
            }
            // Parsley and a slice of orange beside it.
            for k in 0..5 {
                let a = k as f32 * 1.3;
                s.set((cx + a.cos() * 6.8) as i32, 1, (cz + a.sin() * 6.8) as i32, LEAF);
            }
            s.cuboid(1, 1, 7, 2, 1, 9, ORANGE);
            s.paint(|x, y, z, c| (c == ROAST && (x * 3 + z * 5 + y) % 9 == 0).then_some(ROAST_D));
        }
        FoodKind::Ham => {
            ball(&mut s, v(cx, 4.0, cz - 1.0), v(5.4, 4.0, 5.8), HAM);
            limb(&mut s, &[(v(cx, 3.6, cz + 4.0), 2.4), (v(cx, 3.8, cz + 7.6), 1.2)], HAM_D);
            limb(&mut s, &[(v(cx, 3.8, cz + 7.6), 0.9), (v(cx, 4.0, cz + 9.0), 0.9)], BONE);
            ball(&mut s, v(cx, 4.0, cz + 9.2), v(1.3, 1.3, 0.8), BONE);
            // The scored glaze, studded with cloves.
            s.paint(|x, y, z, c| {
                if c != HAM || y < 4 {
                    return None;
                }
                if (x + z) % 4 == 0 || (x - z).rem_euclid(4) == 0 {
                    Some(HAM_D)
                } else if (x * 7 + z * 3) % 13 == 0 {
                    Some(WOOD_D)
                } else {
                    None
                }
            });
            ball(&mut s, v(cx, 1.6, cz - 1.0), v(5.6, 1.0, 6.0), HAM_FAT);
        }
        FoodKind::Pie => {
            drum(&mut s, cx, cz, 0, 3, 6.4, 6.6, CRUST_D);
            drum(&mut s, cx, cz, 4, 4, 6.4, 6.4, JAM);
            for y in 4..=5 {
                for z in 0..d {
                    for x in 0..w {
                        let r = ((x as f32 - cx).powi(2) + (z as f32 - cz).powi(2)).sqrt();
                        let lattice = (x % 3 == 0 && y == 5) || (z % 3 == 0 && y == 4);
                        if (r <= 6.4 && lattice) || (r > 5.6 && r <= 6.8 && y == 4) {
                            s.set(x, y + 1, z, if r > 5.6 { CRUST_L } else { CRUST });
                        }
                    }
                }
            }
            // The crimped rim.
            for k in 0..24 {
                let a = k as f32 / 24.0 * TAU;
                s.set((cx + a.cos() * 6.6).round() as i32, 6, (cz + a.sin() * 6.6).round() as i32, CRUST_L);
            }
        }
        FoodKind::Cake => {
            drum(&mut s, cx, cz, 0, 3, 6.6, 6.6, ICING);
            drum(&mut s, cx, cz, 4, 7, 4.8, 4.8, PINK);
            drum(&mut s, cx, cz, 8, 10, 3.2, 3.2, ICING);
            // Icing dripping down the tiers, a cherry crown.
            for k in 0..20 {
                let a = k as f32 / 20.0 * TAU;
                let len = 1 + (nz(k, 3, 1) * 2.5) as i32;
                for y in (4 - len)..=3 {
                    s.set((cx + a.cos() * 6.9).round() as i32, y, (cz + a.sin() * 6.9).round() as i32, PINK);
                }
                if k % 2 == 0 {
                    s.set((cx + a.cos() * 5.1).round() as i32, 7, (cz + a.sin() * 5.1).round() as i32, ICING);
                }
            }
            ball(&mut s, v(cx, 12.0, cz), v(1.4, 1.4, 1.4), CHERRY);
            for k in 0..5 {
                let a = k as f32 / 5.0 * TAU;
                s.set((cx + a.cos() * 4.0).round() as i32, 8, (cz + a.sin() * 4.0).round() as i32, CHERRY);
            }
        }
        FoodKind::Fruit => {
            // A low silver bowl heaped with fruit, a bunch of grapes hanging over its lip.
            for y in 0..=3 {
                let r = 4.0 + y as f32 * 0.9;
                for k in 0..36 {
                    let a = k as f32 / 36.0 * TAU;
                    s.set((cx + a.cos() * r).round() as i32, y, (cz + a.sin() * r).round() as i32, if y == 3 { SILVER_L } else { SILVER });
                }
            }
            drum(&mut s, cx, cz, 0, 0, 4.0, 4.0, SILVER_D);
            for (p, r, c) in [
                (v(cx - 2.4, 4.6, cz - 1.0), 2.4, APPLE),
                (v(cx + 2.2, 4.4, cz - 1.8), 2.3, ORANGE),
                (v(cx + 0.4, 5.0, cz + 2.0), 2.2, PEAR),
                (v(cx - 0.4, 7.4, cz - 0.6), 2.1, APPLE),
                (v(cx + 2.6, 6.8, cz + 1.0), 1.9, ORANGE),
            ] {
                ball(&mut s, p, Vec3::splat(r), c);
            }
            for k in 0..9 {
                let p = v(cx - 4.0 - (k % 3) as f32 * 0.9, 5.0 - (k / 3) as f32 * 1.4, cz + 3.0 + (k % 2) as f32);
                ball(&mut s, p, Vec3::splat(0.9), GRAPE);
            }
            s.set(cx as i32 - 1, 10, cz as i32 - 1, LEAF);
        }
        FoodKind::Bread => {
            ball(&mut s, v(cx, 2.4, cz), v(4.0, 3.6, 7.0), LOAF);
            for k in -2..=2 {
                let z = (cz + k as f32 * 2.4).round() as i32;
                for x in (cx - 2.0) as i32..=(cx + 2.0) as i32 {
                    if let Some(y) = (0..h).rev().find(|&y| s.get(x, y, z).is_some()) {
                        s.recolor(x, y, z, LOAF_L);
                    }
                }
            }
        }
        FoodKind::Pudding => {
            for y in 0..=8 {
                let r = 5.6 - (y as f32 / 8.0).powi(2) * 3.8;
                drum(&mut s, cx, cz, y, y, r, r, PUDDING);
            }
            // Custard poured over its crown and running down its flutes.
            s.paint(|x, y, z, c| {
                let a = (z as f32 - cz).atan2(x as f32 - cx);
                let drip = 6 - ((a * 4.0).sin() * 2.0 + 1.0) as i32;
                if c == PUDDING && y >= drip {
                    Some(CUSTARD)
                } else {
                    None
                }
            });
            for (dx, dz) in [(-1, 0), (1, 0), (0, 1)] {
                s.set(cx as i32 + dx, 10, cz as i32 + dz, HOLLY);
            }
            s.set(cx as i32, 10, cz as i32, CHERRY);
        }
        FoodKind::Sausages => {
            for k in 0..6 {
                let z = 2.5 + k as f32 * 2.4;
                let bend = (k as f32 * 1.7).sin() * 1.2;
                limb(&mut s, &[(v(2.5, 1.4, z), 1.1), (v(cx + bend, 1.8, z + 0.6), 1.2), (v(w as f32 - 3.0, 1.4, z), 1.1)], SAUSAGE);
            }
            for k in 0..3 {
                let z = 4.0 + k as f32 * 4.4;
                ball(&mut s, v(cx + 1.0, 3.4, z), v(3.0, 1.0, 1.0), SAUSAGE_D);
            }
        }
        FoodKind::Cheese => {
            for y in 0..=5 {
                for z in 0..d {
                    for x in 0..w {
                        let (dx, dz) = (x as f32 - cx, z as f32 - cz);
                        let r = (dx * dx + dz * dz).sqrt();
                        let a = dz.atan2(dx);
                        let cut = a > 0.2 && a < 1.2;
                        if r <= 6.6 && !cut {
                            let rind = r > 5.8 || y == 0 || y == 5;
                            let hole = !rind && nz(x, y, z) < 0.08;
                            if !hole {
                                s.set(x, y, z, if rind { RIND } else { CHEESE });
                            }
                        }
                    }
                }
            }
            s.paint(|x, y, z, c| (c == RIND && y == 5 && (x + z) % 5 == 0).then_some(CHEESE_D));
        }
        FoodKind::Gingerbread => {
            // The witch's bait: a little gingerbread house, iced, with candy at its eaves.
            s.cuboid(4, 0, 4, 13, 6, 12, GINGER);
            for y in 7..=12 {
                let k = y - 7;
                s.cuboid(3 + k, y, 3, 14 - k, y, 13, if k % 2 == 0 { ICING } else { GINGER_D });
            }
            s.cuboid(7, 0, 12, 10, 4, 12, GINGER_D);
            s.cuboid(5, 3, 12, 6, 4, 12, ICING);
            s.cuboid(11, 3, 12, 12, 4, 12, ICING);
            for x in 3..=14 {
                s.set(x, 6, 13, if x % 2 == 0 { CANDY_R } else { CANDY_G });
            }
            s.set(12, 13, 6, GINGER_D);
            s.set(12, 14, 6, ICING);
        }
    }
    merge(&s.finish_grounded(0.02), FOOD_VOXEL)
}

/// A silver dome over a dish, its knob on top: the realm's emblem.
pub fn cloche() -> Merged {
    let (w, h, d) = (22, 15, 22);
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (10.5, 10.5);
    for y in 0..=11 {
        let t = y as f32 / 11.0;
        let r = 9.6 * (1.0 - t * t).max(0.0).sqrt();
        let n = (r * 7.0) as i32 + 8;
        for k in 0..n {
            let a = k as f32 / n as f32 * TAU;
            let c = if y == 0 {
                SILVER_D
            } else if (a - 2.2).abs() < 0.35 && y > 3 {
                SILVER_L
            } else {
                SILVER
            };
            for rr in [r, r - 0.7] {
                s.set((cx + a.cos() * rr).round() as i32, y, (cz + a.sin() * rr).round() as i32, c);
            }
        }
    }
    drum(&mut s, cx, cz, 11, 11, 3.0, 3.0, SILVER);
    drum(&mut s, cx, cz, 12, 12, 1.2, 1.2, SILVER_D);
    ball(&mut s, v(cx, 13.4, cz), v(1.8, 1.4, 1.8), SILVER_L);
    merge(&s.finish(0.0), 0.028)
}

// ------------------------------------------------------------------ the seams' teeth

/// What a seam's teeth are dressed as.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fang {
    /// The Hall's: carved white marble, fluted like a column, red veins already in it.
    Marble,
    /// The Kitchens': sheathed in riveted copper, sweating.
    Copper,
    /// The Larder's: real teeth, wet enamel rooted in gum.
    Ivory,
}

/// A tooth that stands as a column at a seam's arch: gum at its root, tapering to a point,
/// `height` tiles tall.
pub fn tooth(fang: Fang, height: f32) -> Merged {
    let hv = (height / TOOTH_VOXEL).round() as i32;
    let (w, d) = (16, 16);
    let mut s = Sculpt::new(w, hv + 2, d);
    let (cx, cz) = (7.5, 7.5);
    let gum = (hv as f32 * 0.12) as i32;
    for y in 0..=hv {
        let t = y as f32 / hv as f32;
        // A canine's taper: full through its body, curving to a point, leaning a little.
        let u = ((y - gum) as f32 / (hv - gum) as f32).max(0.0);
        let r = if y < gum { 7.2 - (y as f32 / gum as f32) * 0.6 } else { 6.6 * (1.0 - u.powf(1.25) * 0.93) };
        let lean = t * t * 1.6;
        let n = (r * 7.0) as i32 + 6;
        for rr in 0..=(r.ceil() as i32) {
            let rr = (rr as f32).min(r);
            for k in 0..n {
                let a = k as f32 / n as f32 * TAU;
                let (x, z) = (cx + a.cos() * rr, cz + a.sin() * rr + lean);
                let edge = rr > r - 1.1;
                let c = if y < gum {
                    if edge && nz(k, y, 1) < 0.4 {
                        GUM_D
                    } else {
                        GUM
                    }
                } else {
                    match fang {
                        Fang::Marble => {
                            let flute = edge && (a * 8.0).sin() > 0.55;
                            let band = (y - gum) % 22 == 0 || (y - gum) % 22 == 1;
                            let vein = edge && ((a * 3.0 + y as f32 * 0.21).sin() * (y as f32 * 0.13).cos()).abs() < 0.022;
                            if vein {
                                VEIN_RED
                            } else if band {
                                GOLD
                            } else if flute {
                                MARBLE_D
                            } else {
                                MARBLE_W
                            }
                        }
                        Fang::Copper => {
                            let seam = (y - gum) % 9 == 0;
                            let rivet = seam && k % 4 == 0;
                            if rivet {
                                COPPER_L
                            } else if seam {
                                COPPER_D
                            } else if edge && nz(k, y / 3, 7) < 0.12 {
                                VERDIGRIS
                            } else {
                                COPPER
                            }
                        }
                        Fang::Ivory => {
                            if t > 0.85 {
                                TOOTH_D
                            } else if (y - gum) < 5 {
                                TOOTH_ROOT
                            } else if edge && (a * 5.0 + t * 3.0).sin() > 0.93 {
                                TOOTH_D
                            } else {
                                TOOTH
                            }
                        }
                    }
                };
                s.set(x.round() as i32, y, z.round() as i32, c);
            }
        }
    }
    merge(&s.finish(0.0), TOOTH_VOXEL)
}

// ------------------------------------------------------------------ the kitchens

/// A copper cauldron of stew, on three stubby legs.
pub fn cauldron() -> Merged {
    let (w, h, d) = (26, 20, 26);
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (12.5, 12.5);
    for y in 3..=17 {
        let t = (y - 3) as f32 / 14.0;
        let r = 11.0 * (1.0 - (t - 0.45).powi(2) * 1.4).max(0.2).sqrt();
        drum(
            &mut s,
            cx,
            cz,
            y,
            y,
            r,
            r,
            if y == 17 {
                COPPER_L
            } else if y % 5 == 0 {
                COPPER_D
            } else {
                COPPER
            },
        );
    }
    drum(&mut s, cx, cz, 16, 17, 8.6, 8.6, STEW);
    drum(&mut s, cx, cz, 18, 18, 9.4, 9.4, COPPER_L);
    for y in 18..=18 {
        drum(&mut s, cx, cz, y, y, 8.4, 8.4, STEW);
    }
    for k in 0..3 {
        let a = k as f32 / 3.0 * TAU + 0.4;
        limb(&mut s, &[(v(cx + a.cos() * 7.0, 4.0, cz + a.sin() * 7.0), 1.4), (v(cx + a.cos() * 8.0, 0.5, cz + a.sin() * 8.0), 1.1)], IRON);
    }
    // Handles, and a ladle leaning in the stew.
    for side in [-1.0f32, 1.0] {
        limb(&mut s, &[(v(cx + side * 10.5, 15.0, cz), 0.8), (v(cx + side * 12.0, 17.0, cz), 0.8)], COPPER_D);
    }
    limb(&mut s, &[(v(cx + 2.0, 17.0, cz + 1.0), 0.6), (v(cx + 7.0, 19.0, cz + 5.0), 0.6)], WOOD);
    s.paint(|x, y, z, c| (c == STEW && nz(x, y, z) < 0.15).then_some(ROAST_D));
    merge(&s.finish(0.0), 0.034)
}

/// An oven like a mouth, built into a wall and facing +z: a brick dome, lips round its opening,
/// teeth along them, the fire behind.
pub fn oven() -> Merged {
    let (w, h, d) = (32, 44, 16);
    let mut s = Sculpt::new(w, h, d);
    let cx = 15.5;
    // The brick face and dome.
    for y in 0..40 {
        let t = y as f32 / 40.0;
        let half = if t < 0.6 { 15.5 } else { 15.5 * (1.0 - ((t - 0.6) / 0.4).powi(2)).max(0.0).sqrt() };
        for x in 0..w {
            if (x as f32 - cx).abs() > half {
                continue;
            }
            for z in 0..8 {
                let mortar = y % 4 == 0 || (x + if (y / 4) % 2 == 0 { 0 } else { 3 }) % 6 == 0;
                s.set(x, y, z, if mortar { BRICK_D } else { BRICK });
            }
        }
    }
    // The mouth: an arched opening, the fire deep in it.
    let (my, mw, mh) = (8.0, 9.0, 9.5);
    for y in 0..h {
        for x in 0..w {
            let (dx, dy) = ((x as f32 - cx) / mw, (y as f32 - my - mh * 0.5) / (mh * 0.62));
            let inside = dx * dx + dy * dy <= 1.0 && y as f32 >= my;
            if inside {
                for z in 2..8 {
                    s.clear(x, y, z);
                }
                s.set(x, y, 2, if y < my as i32 + 3 { FIRE_L } else { FIRE });
                s.set(x, y, 1, SOOT);
            }
            // The lips, round the opening, puffed out.
            let ring = (dx * dx + dy * dy).sqrt();
            if (1.0..1.45).contains(&ring) && y as f32 >= my - 2.5 {
                for z in 8..(10 + ((1.45 - ring) * 6.0) as i32) {
                    s.set(x, y, z, if ring > 1.3 { LIP_D } else { LIP });
                }
                // Teeth along the inner edge.
                if ring < 1.12 && x % 2 == 0 {
                    s.set(x, y, 10, TOOTH);
                    s.set(x, y, 11, TOOTH);
                }
            }
        }
    }
    // Soot over the mouth, and a chimney stub.
    for x in (cx as i32 - 5)..=(cx as i32 + 5) {
        for y in 26..32 {
            if nz(x, y, 2) < 0.6 {
                s.recolor(x, y, 7, SOOT);
            }
        }
    }
    s.cuboid(cx as i32 - 3, 38, 1, cx as i32 + 3, 43, 6, BRICK_D);
    merge(&s.finish(0.0), 0.033)
}

/// A copper pan hanging by its handle (the hook at the top).
pub fn pan() -> Merged {
    let mut s = Sculpt::new(14, 26, 5);
    let (cx, cy) = (6.5, 7.0);
    for y in 0..14 {
        for x in 0..14 {
            let r = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
            if r <= 6.4 {
                s.set(x, y, 1, if r > 5.4 { COPPER_L } else { COPPER });
                s.set(x, y, 2, if r > 5.4 { COPPER_D } else { COPPER });
            }
        }
    }
    s.cuboid(6, 13, 2, 7, 23, 2, COPPER_D);
    s.cuboid(6, 24, 2, 7, 25, 3, IRON);
    merge(&s.finish(0.0), 0.03)
}

/// A chopping block: a sawn trunk, a slab of meat on it, a cleaver bitten into the wood.
pub fn chopping_block() -> Merged {
    let (w, h, d) = (24, 26, 24);
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (11.5, 11.5);
    drum(&mut s, cx, cz, 0, 17, 10.4, 10.0, WOOD);
    s.paint(|x, y, z, c| {
        let r = ((x as f32 - cx).powi(2) + (z as f32 - cz).powi(2)).sqrt();
        if y == 17 {
            Some(if (r as i32) % 3 == 0 { WOOD_D } else { BOARD })
        } else if (x * 5 + z * 3) % 7 == 0 && c == WOOD {
            Some(WOOD_D)
        } else {
            None
        }
    });
    ball(&mut s, v(cx - 2.0, 19.0, cz + 1.0), v(5.6, 2.0, 4.2), HAM);
    ball(&mut s, v(cx - 2.0, 18.6, cz + 1.0), v(5.8, 1.2, 4.4), HAM_FAT);
    // The cleaver: its blade bitten into the block, its handle up.
    s.cuboid(cx as i32 + 3, 15, cz as i32 - 4, cx as i32 + 3, 22, cz as i32 + 3, SILVER);
    s.cuboid(cx as i32 + 3, 22, cz as i32 - 4, cx as i32 + 3, 22, cz as i32 + 3, SILVER_L);
    limb(&mut s, &[(v(cx + 3.0, 21.0, cz + 4.0), 0.9), (v(cx + 3.0, 25.0, cz + 8.0), 0.9)], WOOD_D);
    merge(&s.finish_grounded(0.0), 0.034)
}

/// A hanging ham, the hook at its top.
pub fn ham() -> Merged {
    let mut s = Sculpt::new(14, 30, 14);
    let (cx, cz) = (6.5, 6.5);
    ball(&mut s, v(cx, 8.0, cz), v(5.2, 7.6, 4.6), HAM);
    limb(&mut s, &[(v(cx, 14.0, cz), 2.4), (v(cx, 20.0, cz), 1.2)], HAM_D);
    limb(&mut s, &[(v(cx, 20.0, cz), 0.8), (v(cx, 23.0, cz), 0.8)], BONE);
    limb(&mut s, &[(v(cx, 23.0, cz), 0.5), (v(cx, 29.0, cz), 0.5)], IRON);
    s.paint(|x, y, z, c| (c == HAM && (x + y + z) % 5 == 0).then_some(HAM_D));
    merge(&s.finish(0.0), 0.034)
}

/// A string of sausages, hung in loops.
pub fn sausages() -> Merged {
    let mut s = Sculpt::new(20, 30, 6);
    for k in 0..5 {
        let y = 26.0 - k as f32 * 5.0;
        let x0 = if k % 2 == 0 { 3.0 } else { 16.0 };
        let x1 = if k % 2 == 0 { 16.0 } else { 3.0 };
        limb(
            &mut s,
            &[(v(x0, y, 3.0), 1.5), ((v(x0, y, 3.0) + v(x1, y - 5.0, 3.0)) * 0.5 + v(0.0, -2.0, 0.0), 1.7), (v(x1, y - 5.0, 3.0), 1.5)],
            SAUSAGE,
        );
        s.set(x1 as i32, (y - 5.0) as i32, 3, SAUSAGE_D);
    }
    s.cuboid(2, 28, 2, 4, 29, 4, IRON);
    merge(&s.finish(0.0), 0.034)
}

// ------------------------------------------------------------------ the hall

/// A high-backed chair in dark wood with a red velvet seat, facing +z.
pub fn chair() -> Merged {
    let mut s = Sculpt::new(14, 32, 14);
    for (x, z) in [(1, 1), (12, 1), (1, 12), (12, 12)] {
        s.cuboid(x, 0, z, x, 11, z, WOOD_D);
    }
    s.cuboid(0, 12, 0, 13, 13, 13, WOOD);
    s.cuboid(1, 14, 1, 12, 14, 12, VELVET);
    s.cuboid(0, 12, 0, 13, 31, 1, WOOD);
    s.cuboid(2, 17, 2, 11, 28, 2, VELVET);
    s.cuboid(4, 30, 0, 9, 31, 1, GOLD);
    merge(&s.finish_grounded(0.0), 0.032)
}

// ------------------------------------------------------------------ the larder

/// A salt barrel, hooped in iron, salt heaped over its rim.
pub fn barrel() -> Merged {
    let (w, h, d) = (24, 28, 24);
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (11.5, 11.5);
    for y in 0..=24 {
        let t = y as f32 / 24.0;
        let r = 9.0 + (PI * t).sin() * 1.6;
        let hoop = y == 2 || y == 3 || y == 21 || y == 22 || y == 12;
        drum(&mut s, cx, cz, y, y, r, r, if hoop { IRON } else { WOOD });
    }
    s.paint(|x, _, z, c| {
        let a = (z as f32 - cz).atan2(x as f32 - cx);
        (c == WOOD && ((a * 12.0 / TAU).fract() < 0.1)).then_some(WOOD_D)
    });
    for y in 25..=27 {
        drum(&mut s, cx, cz, y, y, 8.6 - (y - 25) as f32 * 3.0, 8.6 - (y - 25) as f32 * 3.0, SALT);
    }
    merge(&s.finish_grounded(0.0), 0.034)
}

/// A rib, rising from the floor and arching over toward +z.
pub fn rib() -> Merged {
    let (w, h, d) = (10, 64, 34);
    let mut s = Sculpt::new(w, h, d);
    let mut knots = Vec::new();
    for k in 0..=8 {
        let t = k as f32 / 8.0;
        let a = t * PI * 0.62;
        knots.push((v(4.5, 2.0 + a.sin() * 58.0, 3.0 + (1.0 - a.cos()) * 30.0), 3.0 - t * 1.8));
    }
    limb(&mut s, &knots, TOOTH);
    ball(&mut s, v(4.5, 1.5, 3.0), v(4.2, 2.6, 4.2), FLESH);
    s.paint(|x, y, z, c| (c == TOOTH && nz(x, y / 3, z) < 0.25).then_some(TOOTH_D));
    merge(&s.finish(0.0), 0.034)
}

// ------------------------------------------------------------------ the stomach

/// A heap of what the Stomach has not finished: a dented helmet, a breastplate, bones, a plate.
pub fn digested() -> Merged {
    let (w, h, d) = (26, 14, 26);
    let mut s = Sculpt::new(w, h, d);
    ball(&mut s, v(12.5, 0.0, 12.5), v(11.0, 3.0, 10.0), FLESH_D);
    // The helmet, tipped on its side.
    ball(&mut s, v(8.0, 6.0, 10.0), v(4.6, 4.4, 4.6), RUST);
    s.cuboid(4, 5, 13, 12, 6, 14, IRON);
    // The breastplate.
    ball(&mut s, v(17.0, 4.0, 14.0), v(5.6, 3.0, 4.0), SILVER_D);
    // Bones, and a plate.
    limb(&mut s, &[(v(3.0, 3.0, 19.0), 1.0), (v(14.0, 4.0, 22.0), 1.0)], BONE);
    ball(&mut s, v(3.0, 3.0, 19.0), v(1.5, 1.5, 1.5), BONE);
    ball(&mut s, v(14.0, 4.0, 22.0), v(1.5, 1.5, 1.5), BONE);
    drum(&mut s, 19.0, 6.0, 3, 3, 4.0, 4.0, SILVER);
    s.paint(|x, y, z, c| (c == RUST && nz(x, y, z) < 0.3).then_some(ACID));
    merge(&s.finish_grounded(0.03), 0.034)
}

/// A purge font: a stone basin sunk in the floor, a carved mouth spewing into it. The liquid is
/// apart (`font_liquid`), so it can be drained once used.
pub fn font() -> Merged {
    let (w, h, d) = (28, 26, 28);
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (13.5, 13.5);
    for y in 0..=4 {
        let r = 12.4 + y as f32 * 0.2;
        for rr in [r, r - 1.0, r - 2.0] {
            for k in 0..80 {
                let a = k as f32 / 80.0 * TAU;
                s.set((cx + a.cos() * rr).round() as i32, y, (cz + a.sin() * rr).round() as i32, if y == 4 { STONE } else { STONE_D });
            }
        }
    }
    // The spout behind: a pillar with a gaping carved face.
    s.cuboid(cx as i32 - 3, 0, 1, cx as i32 + 3, 22, 5, STONE);
    ball(&mut s, v(cx, 18.0, 5.5), v(4.0, 4.0, 2.4), STONE);
    s.cuboid(cx as i32 - 1, 15, 6, cx as i32 + 1, 17, 8, LIP_D);
    s.set(cx as i32 - 2, 20, 7, ACID);
    s.set(cx as i32 + 2, 20, 7, ACID);
    merge(&s.finish_grounded(0.0), 0.034)
}
