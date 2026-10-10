//! Gluttony's court, the Banquet of Flies (`logic::court::gluttony` lays it out), to the art
//! direction: the board's palette (charcoal, umber, ochre, a sickly olive) with Gluttony's orange
//! as its accent, and the Rotting Feast's own materials (its food, its cauldrons, its chairs: the
//! realm's `art`), gone to rot.
//!
//! A long hall floored in greasy grey-brown flags, stained with spilled wine and grease and strewn
//! with crumbs and gnawed bones, a moth-eaten red runner up its middle from the gate to the gap in
//! the head of the table. Walls of dark walnut wainscot under faded mustard damask blooming with
//! black mould, guttered candles in iron sconces. The great table runs round the floor in a U,
//! its linen yellowed and stained and hanging ragged, laid with tarnished platters of food furred
//! grey-green with mould (bare once he has eaten it, a blackened ruin once he has choked on it),
//! tipped goblets, plates of bones, candle stubs, rotten fruit and bottles; empty high-backed
//! chairs stand in the lanes behind it, some toppled. Cauldrons stand over cold fires of grey ash,
//! green scum on what is left in them. In the north, on a low dais, his throne of piled plates:
//! stacks of china for its seat, its arms and its high back, a fan of plates upright over it like a
//! crown. Two iron wheels of candles hang over the floor; tattered green banners with a fly in
//! faded gold hang either side of the throne. And everywhere the flies: a cloud over every dish,
//! slow vortices of them up in the dark, and a court of them about their lord. His bile on the
//! floor (`prince::Trap`) is drawn here too.

use super::realms::feast::art;
use super::vault::SHADE_MARBLE;
use super::vault_decor::{push_merged, MBox, Merged};
use super::*;
use crate::logic::combat::prince::{Trap, SPIKES};
use crate::logic::combat::{MoveId, State};
use crate::logic::court::gluttony::{self as plan, Piece, ARMS, CAULDRONS, CENTRE, DAIS, GATE, HEAD, PLATTERS, PLATTERS_SET};
use crate::logic::hub::Gate;
use crate::logic::realms::feast::FoodKind;

mod pal {
    pub const FLAG: u32 = 0x544a3c;
    pub const FLAG_L: u32 = 0x625846;
    pub const FLAG_D: u32 = 0x463e32;
    pub const GROUT: u32 = 0x2c261e;
    pub const STAIN: u32 = 0x403628;
    pub const GREASE: u32 = 0x423a24;
    pub const WINE: u32 = 0x42161a;
    pub const CRUMB: u32 = 0x9a7440;
    pub const BONE: u32 = 0xd8ccb0;
    pub const BONE_D: u32 = 0xa89a78;
    pub const RUNNER: u32 = 0x5a1a1c;
    pub const RUNNER_D: u32 = 0x42121a;
    pub const OCHRE: u32 = 0x8a6a2a;
    pub const WALNUT: u32 = 0x2e2016;
    pub const WALNUT_L: u32 = 0x40301e;
    pub const DAMASK: u32 = 0x76643a;
    pub const DAMASK_D: u32 = 0x5a4a28;
    pub const MOULD: u32 = 0x1a1c10;
    pub const MOULD_G: u32 = 0x2e3a16;
    pub const IRON: u32 = 0x2a2a2c;
    pub const PEWTER: u32 = 0x8a8a80;
    pub const SILVER_T: u32 = 0x8e8a78;
    pub const SILVER_D: u32 = 0x5e5a4c;
    pub const LINEN: u32 = 0xa8a088;
    pub const LINEN_D: u32 = 0x8a8268;
    pub const LINEN_STAIN: u32 = 0x6a5a3a;
    pub const WOOD: u32 = 0x3a2414;
    pub const WOOD_D: u32 = 0x24160c;
    pub const WAX: u32 = 0xd8ccaa;
    pub const BOTTLE: u32 = 0x1e3a22;
    pub const CHINA: u32 = 0xd8d4c4;
    pub const CHINA_D: u32 = 0xa8a494;
    pub const CHINA_RIM: u32 = 0x6a8aa0;
    pub const STONE: u32 = 0x3a342c;
    pub const STONE_D: u32 = 0x2a2620;
    pub const ASH: u32 = 0x5a5650;
    pub const CHAR: u32 = 0x1a1612;
    pub const BANNER: u32 = 0x2a3a1a;
    pub const BANNER_D: u32 = 0x1e2a12;
    pub const GOLD_F: u32 = 0x9a7a30;
    pub const FLAME: u32 = 0xffb050;
    pub const FLAME_TIP: u32 = 0xff7a20;
    pub const CANDLE_LIGHT: u32 = 0xffb868;
    pub const SICKLY: u32 = 0x9ab030;
    pub const FLY: u32 = 0x141010;
    pub const BILE: u32 = 0xa8b030;
    pub const BILE_D: u32 = 0x6a7018;
    pub const BILE_L: u32 = 0xd8e060;
    pub const SCUM: u32 = 0x4a5a1a;
}
use pal::*;

/// The hall's walls stand this tall; the table's top this high.
const WALL_H: f32 = 2.6;
const TABLE_Y: f32 = 0.78;
/// How high the candle wheels hang, and where.
const WHEEL_Y: f32 = 4.4;
const WHEELS: [(f32, f32); 2] = [(16.0, 13.0), (16.0, 21.0)];
/// Where candelabra stand on the table (none on a platter's tile).
const CANDELABRA: [(i32, i32); 8] = [(6, 9), (6, 16), (6, 22), (26, 9), (26, 16), (26, 22), (8, 8), (24, 8)];
/// The food on each platter, in the order of `PLATTERS`.
const FOODS: [FoodKind; 6] = [FoodKind::Roast, FoodKind::Ham, FoodKind::Pie, FoodKind::Pudding, FoodKind::Cake, FoodKind::Cheese];

/// The court's light: dim and sickly, candlelight in a dark of flies, an olive fill from above.
pub fn gluttony_atmosphere() -> Atmosphere {
    Atmosphere {
        background: 0x0c0a06,
        fog_near: 16.0,
        fog_far: 42.0,
        hemi_sky: lin(0x6e7450) * 0.5,
        hemi_ground: lin(0x5a4a30) * 0.5,
        ambient: lin(0x5a5440) * 0.6,
        sun_dir: Vec3::new(0.3, 1.0, 0.45).normalize(),
        sun: lin(0xe8dcb0) * 1.0,
        exposure: 1.35,
        night_sky: 0.0,
        bloom: 0.72,
        moon_view: Vec3::new(0.0, 0.3, -1.0).normalize(),
        cloud_tint: lin(0x141008),
    }
}

/// Its ground haze: density and falloff (the air thick over the food).
pub const HAZE: (f32, f32) = (0.018, 2.2);

/// One platter's static ranges: the dish itself, and the food as it lies laden, eaten bare and
/// spoiled (only one shows), with each range's own matrices to put back.
struct Platter {
    ranges: [(usize, usize); 4],
    mats: [Vec<[f32; 16]>; 4],
    at: Vec3,
}

/// What the view keeps of the Banquet: its candles, its wheels, its cauldrons, its throne, its
/// gate, and the platters with what lies on each (0 laden, 1 eaten, 2 spoiled; none: not laid).
pub struct BanquetView {
    candles: Vec<Vec3>,
    cauldrons: Vec<Vec3>,
    throne: Vec3,
    gate: Gate,
    platters: Vec<Platter>,
    dishes: Vec<Option<u8>>,
    /// The platters have been shown as `dishes` says at least once (as built, every state shows).
    shown: bool,
}

fn cube(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot_y: f32, color: Vec3, shade: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(rot_y), pos), color, 1.0, Vec3::ZERO, shade));
}

fn tilted(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot: Quat, color: Vec3, shade: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(scale, rot, pos), color, 1.0, Vec3::ZERO, shade));
}

/// A flat octagon (two squares turned) `r` across at `at`, `t` thick: a plate, a platter, a disc.
fn disc(out: &mut Vec<Instance>, at: Vec3, r: f32, t: f32, color: Vec3, shade: f32) {
    for a in [0.0, PI / 4.0] {
        cube(out, at, Vec3::new(r * 1.66, t, r * 1.66), a, color, shade);
    }
}

/// A plate standing on its rim, facing `yaw`, tipped back by `tip`.
fn upright_plate(out: &mut Vec<Instance>, at: Vec3, r: f32, yaw: f32, tip: f32, color: Vec3, rim: Vec3) {
    let rot = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(PI / 2.0 - tip);
    let normal = rot * Vec3::Y;
    for a in [0.0, PI / 4.0] {
        let q = rot * Quat::from_rotation_y(a);
        // Its blue rim behind, its white face before it, and the well of the plate in its middle.
        tilted(out, at, Vec3::new(r * 1.66, 0.02, r * 1.66), q, rim, SHADE_LIT);
        tilted(out, at + normal * 0.012, Vec3::new(r * 1.42, 0.02, r * 1.42), q, color, SHADE_LIT);
    }
    tilted(out, at + normal * 0.02, Vec3::new(r * 0.9, 0.01, r * 0.9), rot * Quat::from_rotation_y(PI / 8.0), color * 0.92, SHADE_LIT);
}

/// A model gone to mould: its colours greyed toward rot by `k`, furred pale in spots and black-green
/// in others.
fn mouldy(m: &Merged, k: f32) -> Merged {
    let grey = lin(0x6e7254);
    Merged {
        boxes: m
            .boxes
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let h = hash(i as f64 * 1.37 + 5.0, (b.center.y * 97.0 + b.center.x * 31.0) as f64) as f32;
                let rot = b.color.lerp(grey * (0.4 + b.color.max_element() * 0.5), k);
                let color = if h > 0.88 && b.center.y > 0.05 {
                    lin(0xb8c4a0)
                } else if h < 0.12 {
                    lin(MOULD_G)
                } else {
                    rot
                };
                MBox { color, ..*b }
            })
            .collect(),
    }
}

/// A model recoloured: every box near `from` becomes `to`, and the rest are pulled `k` of the way
/// toward `dark` (a cold copper cauldron, its stew gone to scum).
fn recolored(m: &Merged, from: Vec3, to: Vec3, dark: Vec3, k: f32) -> Merged {
    Merged {
        boxes: m
            .boxes
            .iter()
            .map(|b| {
                let color = if (b.color - from).length() < 0.02 { to } else { b.color.lerp(dark, k) };
                MBox { color, ..*b }
            })
            .collect(),
    }
}

/// A model squashed flat and blackened: the ruin of a spoiled dish.
fn ruined(m: &Merged) -> Merged {
    Merged {
        boxes: m
            .boxes
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let h = hash(i as f64 * 2.1, 3.0) as f32;
                let c = if h > 0.8 { lin(0x3a4a12) } else { lin(0x1a1a0e) * (0.8 + h * 0.4) };
                MBox {
                    center: Vec3::new(b.center.x * 1.15, b.center.y * 0.35, b.center.z * 1.15),
                    size: b.size * Vec3::new(1.15, 0.5, 1.15),
                    color: c,
                    shade: b.shade,
                }
            })
            .collect(),
    }
}

fn flag(h: f64) -> Vec3 {
    let set = [FLAG, FLAG, FLAG_L, FLAG_D, 0x4e4a3a];
    lin(set[(h * 5.0) as usize % 5]) * (0.9 + h as f32 * 0.16)
}

impl WorldView {
    /// The Banquet's static scene.
    pub(super) fn build_gluttony_court(&mut self, grid: &Grid) {
        let floor_tile = |x: i32, y: i32| plan::in_hall(x, y);
        let gate_tile = |x: i32, y: i32| x == GATE.0 && y == GATE.1;

        // ---- the floor: greasy flags in dark grout, stains, crumbs and bones; the runner up the middle
        for y in 0..grid.height {
            for x in 0..grid.width {
                if !floor_tile(x, y) && !gate_tile(x, y) {
                    continue;
                }
                let (xf, yf) = (x as f32, y as f32);
                let h = hash(x as f64 * 1.9 + 7.0, y as f64 * 2.7);
                let out = &mut self.static_cubes;
                cube(out, Vec3::new(xf, -0.07, yf), Vec3::new(1.0, 0.1, 1.0), 0.0, lin(GROUT), SHADE_LIT);
                // Two flags to a tile.
                for (k, dx) in [(0, -0.25f32), (1, 0.25)] {
                    let hh = hash(x as f64 * 3.1 + k as f64, y as f64 * 1.3 + 2.0);
                    cube(out, Vec3::new(xf + dx, -0.015, yf), Vec3::new(0.47, 0.06, 0.94), 0.0, flag(hh), SHADE_LIT);
                }
                let runner = (15..=17).contains(&x) && y >= HEAD[0].2 && !gate_tile(x, y);
                if runner {
                    // The runner, moth-eaten: a hole here and there shows the flags.
                    if h > 0.12 {
                        let edge = x != 16;
                        let col = if edge { lin(OCHRE) * 0.7 } else { lin(if h > 0.6 { RUNNER } else { RUNNER_D }) };
                        cube(out, Vec3::new(xf, 0.02, yf), Vec3::new(1.0, 0.012, 1.0), 0.0, col, SHADE_LIT);
                        if edge {
                            let inner = if x < 16 { 0.25 } else { -0.25 };
                            cube(out, Vec3::new(xf + inner, 0.027, yf), Vec3::new(0.5, 0.012, 1.0), 0.0, lin(RUNNER) * 0.9, SHADE_LIT);
                        }
                    }
                    if h > 0.82 {
                        cube(
                            out,
                            Vec3::new(xf + (h as f32 - 0.9) * 2.0, 0.031, yf + 0.2),
                            Vec3::new(0.35, 0.006, 0.28),
                            (h * 9.0) as f32,
                            lin(WINE),
                            SHADE_LIT,
                        );
                    }
                    continue;
                }
                // Stains: grease and old wine, spreading in blots.
                if h > 0.68 {
                    let n = 1 + (h * 10.0) as usize % 3;
                    for k in 0..n {
                        let a = hash(x as f64 + k as f64 * 3.0, y as f64 * 5.0) as f32 * TAU;
                        let r = 0.12 + 0.18 * hash(y as f64 + k as f64, x as f64) as f32;
                        let col = if h > 0.93 {
                            lin(WINE)
                        } else if k % 2 == 0 {
                            lin(GREASE)
                        } else {
                            lin(STAIN)
                        };
                        cube(
                            out,
                            Vec3::new(xf + a.cos() * 0.22, 0.017 + k as f32 * 0.001, yf + a.sin() * 0.22),
                            Vec3::new(r * 1.4, 0.006, r),
                            a,
                            col,
                            SHADE_LIT,
                        );
                    }
                }
                // Crumbs, and now and then a gnawed bone.
                if h < 0.3 {
                    for k in 0..5 {
                        let a = (h as f32 * 40.0 + k as f32 * 1.7) % TAU;
                        let r = 0.1 + 0.08 * k as f32;
                        cube(out, Vec3::new(xf + a.cos() * r, 0.025, yf + a.sin() * r), Vec3::splat(0.035), a, lin(CRUMB), SHADE_LIT);
                    }
                }
                if (0.3..0.36).contains(&h) {
                    let a = h as f32 * 50.0;
                    cube(out, Vec3::new(xf, 0.04, yf), Vec3::new(0.32, 0.045, 0.05), a, lin(BONE), 11.0);
                    for s in [-1.0f32, 1.0] {
                        let end = Vec3::new(xf + a.cos() * 0.16 * s, 0.045, yf - a.sin() * 0.16 * s);
                        cube(out, end, Vec3::new(0.07, 0.06, 0.08), a, lin(BONE_D), 11.0);
                    }
                }
            }
        }

        // ---- the walls: walnut wainscot, mouldering damask above, a dark cornice
        let mut candles = Vec::new();
        for (x, y, t) in grid.cells() {
            if t != Tile::Rock || floor_tile(x, y) || gate_tile(x, y) {
                continue;
            }
            let open: Vec<Dir> = CARDINALS
                .iter()
                .copied()
                .filter(|d| {
                    let n = P::new(x, y).step(*d);
                    floor_tile(n.x, n.y)
                })
                .collect();
            let diagonal = !open.is_empty() || [(-1, -1), (1, -1), (-1, 1), (1, 1)].iter().any(|&(dx, dy)| floor_tile(x + dx, y + dy));
            if !diagonal || (y == GATE.1 && (x - GATE.0).abs() <= 1) {
                continue;
            }
            let start = self.static_cubes.len();
            let (xf, yf) = (x as f32, y as f32);
            let h = hash(x as f64 * 4.0, y as f64 * 6.0);
            let out = &mut self.static_cubes;
            cube(out, Vec3::new(xf, 0.5, yf), Vec3::new(1.0, 1.0, 1.0), 0.0, lin(WALNUT), 12.0);
            cube(
                out,
                Vec3::new(xf, 1.0 + (WALL_H - 1.0) / 2.0, yf),
                Vec3::new(1.0, WALL_H - 1.0, 1.0),
                0.0,
                lin(DAMASK) * (0.9 + h as f32 * 0.12),
                SHADE_LIT,
            );
            cube(out, Vec3::new(xf, WALL_H, yf), Vec3::new(1.08, 0.14, 1.08), 0.0, lin(WALNUT_L), 12.0);
            cube(out, Vec3::new(xf, WALL_H + 0.1, yf), Vec3::new(1.12, 0.06, 1.12), 0.0, lin(STONE_D), SHADE_LIT);
            for &d in &open {
                let (dx, dy) = d.vec();
                let n = Vec3::new(dx as f32, 0.0, dy as f32);
                let along = if dx == 0 { Vec3::X } else { Vec3::Z };
                let face = Vec3::new(xf, 0.0, yf) + n * 0.5;
                let size = |a: f32, up: f32, out: f32| along * a + Vec3::Y * up + Vec3::new(n.x.abs(), 0.0, n.z.abs()) * out;
                // The wainscot's rail and its panel.
                cube(out, face + Vec3::new(0.0, 1.0, 0.0) + n * 0.03, size(1.0, 0.07, 0.06), 0.0, lin(WALNUT_L), 12.0);
                cube(out, face + Vec3::new(0.0, 0.5, 0.0) + n * 0.01, size(0.78, 0.7, 0.02), 0.0, lin(WALNUT_L) * 0.8, 12.0);
                // The damask's pattern: lozenges, darker.
                for (k, yy) in [1.35f32, 1.85, 2.3].into_iter().enumerate() {
                    let shift = if (x + y + k as i32) % 2 == 0 { 0.25 } else { -0.25 };
                    cube(
                        out,
                        face + along * shift + Vec3::new(0.0, yy, 0.0) + n * 0.008,
                        size(0.16, 0.16, 0.01),
                        0.0,
                        lin(DAMASK_D),
                        SHADE_LIT,
                    );
                }
                // Black mould blooming up from the wainscot, worst in the corners and by the food.
                let bloom = hash(x as f64 * 9.0 + 1.0, y as f64 * 3.0);
                if bloom > 0.45 {
                    for k in 0..6 {
                        let s = hash(x as f64 + k as f64 * 1.1, y as f64 + bloom) as f32 - 0.5;
                        let yy = 1.1 + k as f32 * 0.17 + s * 0.1;
                        let w = (0.5 - k as f32 * 0.07) * (0.6 + bloom as f32 * 0.6);
                        let col = if k % 3 == 2 { lin(MOULD_G) } else { lin(MOULD) };
                        cube(
                            out,
                            face + along * (s * 0.4) + Vec3::new(0.0, yy, 0.0) + n * 0.012,
                            size(w.max(0.08), 0.16, 0.012),
                            0.0,
                            col,
                            SHADE_LIT,
                        );
                    }
                }
                // A sconce every third block: an iron arm, a guttered candle.
                if (x + y) % 3 == 0 {
                    let at = face + n * 0.16 + Vec3::new(0.0, 1.55, 0.0);
                    cube(out, at - n * 0.08, size(0.05, 0.05, 0.16), 0.0, lin(IRON), 6.0);
                    cube(out, at, Vec3::new(0.14, 0.03, 0.14), 0.0, lin(IRON), 6.0);
                    let tall = 0.08 + 0.1 * hash(x as f64, y as f64 + 9.0) as f32;
                    cube(out, at + Vec3::new(0.0, tall / 2.0 + 0.015, 0.0), Vec3::new(0.06, tall, 0.06), 0.0, lin(WAX), 11.0);
                    cube(out, at + Vec3::new(0.035, 0.03, 0.0), Vec3::new(0.025, 0.06, 0.03), 0.0, lin(WAX) * 0.9, 11.0);
                    candles.push(at + Vec3::new(0.0, tall + 0.06, 0.0));
                }
            }
            let len = self.static_cubes.len() - start;
            self.rock_index.insert(tile_key(x, y), (start, len));
            let shades: Vec<f32> = self.static_cubes[start..start + len].iter().map(|i| i.emissive[3]).collect();
            self.trees.push(Tree {
                range: (start, len),
                base: Vec3::new(xf, 0.0, yf),
                radius: 0.72,
                height: WALL_H + 0.3,
                fade: 1.0,
                alive: true,
                shades: Some(shades),
            });
        }

        // ---- the table, its cloth, its clutter
        let fruit = mouldy(&art::food(FoodKind::Fruit), 0.55);
        let bread = mouldy(&art::food(FoodKind::Bread), 0.5);
        for (x, y, _) in grid.cells() {
            if plan::piece(x, y) != Some(Piece::Table) {
                continue;
            }
            let start = self.static_cubes.len();
            self.build_table_tile(x, y, &mut candles, &fruit, &bread);
            let len = self.static_cubes.len() - start;
            self.rock_index.insert(tile_key(x, y), (start, len));
        }
        // The platters, each with its food (laden, eaten, spoiled: one shows).
        let mut platters = Vec::new();
        for (k, &p) in PLATTERS.iter().enumerate() {
            platters.push(self.build_platter(p, FOODS[k]));
        }

        // ---- the chairs in the lanes and along the head, some toppled
        let chair = art::chair();
        let chair = recolored(&chair, lin(0x8a1a22), lin(0x4a1a1a), lin(0x1a1410), 0.25);
        for &(ax, y0, y1) in &ARMS {
            let side = if ax < CENTRE.0 { -1.0f32 } else { 1.0 };
            for y in (y0 + 1..=y1).step_by(2) {
                let h = hash(ax as f64 * 1.1, y as f64 * 0.7) as f32;
                let at = Vec3::new(ax as f32 + side * 0.72, 0.0, y as f32);
                let yaw = if side < 0.0 { PI / 2.0 } else { -PI / 2.0 };
                self.place_chair(&chair, at, yaw, h);
            }
        }
        for &(x0, x1, hy) in &HEAD {
            for x in (x0 + 1..=x1).step_by(2) {
                let h = hash(x as f64 * 0.9, hy as f64 * 1.3) as f32;
                self.place_chair(&chair, Vec3::new(x as f32, 0.0, hy as f32 - 0.72), 0.0, h);
            }
        }

        // ---- the cauldrons over their cold fires
        let pot = recolored(&art::cauldron(), lin(0x7a4a1a), lin(SCUM), lin(0x2a3226), 0.45);
        let mut cauldrons = Vec::new();
        for &(x, y) in &CAULDRONS {
            let base = Vec3::new(x as f32, 0.0, y as f32);
            let start = self.static_cubes.len();
            let out = &mut self.static_cubes;
            disc(out, base + Vec3::new(0.0, 0.01, 0.0), 0.5, 0.02, lin(ASH), SHADE_LIT);
            for k in 0..9 {
                let a = k as f32 * TAU / 9.0;
                let r = 0.48;
                cube(
                    out,
                    base + Vec3::new(a.cos() * r, 0.07, a.sin() * r),
                    Vec3::new(0.16, 0.13, 0.14),
                    a,
                    lin(if k % 3 == 0 { STONE } else { STONE_D }),
                    SHADE_MARBLE,
                );
            }
            for k in 0..3 {
                let a = k as f32 * 1.05 + x as f32;
                tilted(
                    out,
                    base + Vec3::new(0.0, 0.06, 0.0),
                    Vec3::new(0.62, 0.07, 0.08),
                    Quat::from_rotation_y(a) * Quat::from_rotation_z(0.12),
                    lin(CHAR),
                    SHADE_LIT,
                );
            }
            push_merged(out, &pot, Mat4::from_translation(base + Vec3::new(0.0, 0.05, 0.0)) * Mat4::from_rotation_y(x as f32), 0.0);
            let len = self.static_cubes.len() - start;
            self.rock_index.insert(tile_key(x, y), (start, len));
            cauldrons.push(base + Vec3::new(0.0, 0.72, 0.0));
        }

        // ---- the dais and his throne of plates; the banners either side
        let throne = self.build_plate_throne();
        for bx in [11.0f32, 21.0] {
            self.build_banner(Vec3::new(bx, 0.0, 2.5));
        }

        // ---- the candle wheels hung over the floor (each fading out of the camera's way, as the
        // trees do: under the top camera his marks fell behind its ring and struts, QA.md Q9)
        for &(wx, wz) in &WHEELS {
            let at = Vec3::new(wx, WHEEL_Y, wz);
            let start = self.static_cubes.len();
            let out = &mut self.static_cubes;
            for k in 0..16 {
                let a = k as f32 * TAU / 16.0;
                cube(out, at + Vec3::new(a.cos() * 0.9, 0.0, a.sin() * 0.9), Vec3::new(0.38, 0.06, 0.06), -a + PI / 2.0, lin(IRON), 6.0);
            }
            for k in 0..4 {
                let a = k as f32 * TAU / 4.0;
                let foot = at + Vec3::new(a.cos() * 0.88, 0.0, a.sin() * 0.88);
                let hook = at + Vec3::new(0.0, 1.4, 0.0);
                let mid = (foot + hook) * 0.5;
                let rot = Quat::from_rotation_arc(Vec3::Y, (hook - foot).normalize());
                tilted(out, mid, Vec3::new(0.02, (hook - foot).length(), 0.02), rot, lin(IRON), 6.0);
            }
            cube(out, at + Vec3::new(0.0, 3.0, 0.0), Vec3::new(0.03, 3.2, 0.03), 0.0, lin(IRON), 6.0);
            for k in 0..8 {
                let a = (k as f32 + 0.5) * TAU / 8.0;
                let c = at + Vec3::new(a.cos() * 0.9, 0.04, a.sin() * 0.9);
                let tall = 0.1 + 0.12 * ((k * 5 % 7) as f32 / 7.0);
                cube(out, c + Vec3::new(0.0, tall / 2.0, 0.0), Vec3::new(0.06, tall, 0.06), 0.0, lin(WAX), 11.0);
                // Wax hanging from the ring in icicles.
                cube(out, c + Vec3::new(0.03, -0.07, 0.0), Vec3::new(0.025, 0.12, 0.025), 0.0, lin(WAX) * 0.85, 11.0);
                candles.push(c + Vec3::new(0.0, tall + 0.05, 0.0));
            }
            let len = self.static_cubes.len() - start;
            let shades: Vec<f32> = self.static_cubes[start..start + len].iter().map(|i| i.emissive[3]).collect();
            self.trees.push(Tree {
                range: (start, len),
                base: at - Vec3::new(0.0, 0.2, 0.0),
                radius: 1.0,
                height: 4.8,
                fade: 1.0,
                alive: true,
                shades: Some(shades),
            });
        }

        // ---- the gate: Gluttony's portal in the south wall, looking into the hall
        let gate = Gate { tile: P::new(GATE.0, GATE.1), facing: Dir::Up, sin: Sin::Gluttony };
        self.build_court_portal(&gate);
        let mut view = BanquetView { candles, cauldrons, throne, gate, platters, dishes: vec![None; PLATTERS.len()], shown: false };
        // (Until the court says otherwise: the four platters laid, laden.)
        let laid: Vec<Option<u8>> = (0..PLATTERS.len()).map(|k| (k < PLATTERS_SET).then_some(0)).collect();
        self.show_platters(&mut view, &laid);
        self.banquet = Some(view);
    }

    /// One tile of the great table: trestle and top, the yellowed cloth hanging ragged where the
    /// table ends, and what lies on it (unless a platter stands there).
    fn build_table_tile(&mut self, x: i32, y: i32, candles: &mut Vec<Vec3>, fruit: &Merged, bread: &Merged) {
        let table = |x: i32, y: i32| plan::piece(x, y) == Some(Piece::Table);
        let base = Vec3::new(x as f32, 0.0, y as f32);
        let h = hash(x as f64 * 2.3 + 1.0, y as f64 * 1.7);
        let out = &mut self.static_cubes;
        // The top, its trestles where the table turns or ends.
        cube(out, base + Vec3::new(0.0, TABLE_Y - 0.04, 0.0), Vec3::new(0.98, 0.07, 0.98), 0.0, lin(WOOD), 12.0);
        let ends = CARDINALS.iter().filter(|d| {
            let n = P::new(x, y).step(**d);
            !table(n.x, n.y)
        });
        if ends.count() >= 3 || (x + y) % 3 == 0 {
            for (sx, sz) in [(-0.38f32, -0.38f32), (0.38, -0.38), (-0.38, 0.38), (0.38, 0.38)] {
                cube(out, base + Vec3::new(sx, (TABLE_Y - 0.08) / 2.0, sz), Vec3::new(0.08, TABLE_Y - 0.08, 0.08), 0.0, lin(WOOD_D), 12.0);
            }
        }
        // The cloth: over the top, stained, and down every side that has no table beyond it.
        let cloth = if h > 0.7 { lin(LINEN_D) } else { lin(LINEN) };
        cube(out, base + Vec3::new(0.0, TABLE_Y + 0.005, 0.0), Vec3::new(1.0, 0.02, 1.0), 0.0, cloth, 8.0);
        if h > 0.4 {
            let a = (h * 30.0) as f32;
            cube(
                out,
                base + Vec3::new((h as f32 - 0.7) * 0.6, TABLE_Y + 0.018, 0.1),
                Vec3::new(0.34, 0.006, 0.26),
                a,
                lin(LINEN_STAIN),
                8.0,
            );
        }
        for d in CARDINALS {
            let n = P::new(x, y).step(d);
            if table(n.x, n.y) {
                continue;
            }
            let (dx, dy) = d.vec();
            let nrm = Vec3::new(dx as f32, 0.0, dy as f32);
            let along = if dx == 0 { Vec3::X } else { Vec3::Z };
            for k in 0..5 {
                let u = -0.4 + k as f32 * 0.2;
                let drop = 0.32 + 0.22 * hash(x as f64 * 5.0 + k as f64, y as f64 * 3.0 + dx as f64) as f32;
                let p = base + nrm * 0.51 + along * u + Vec3::new(0.0, TABLE_Y - drop / 2.0, 0.0);
                let size = along * 0.21 + Vec3::Y * drop + Vec3::new(nrm.x.abs(), 0.0, nrm.z.abs()) * 0.02;
                cube(out, p, size, 0.0, cloth * (0.9 + 0.05 * (k % 2) as f32), 8.0);
            }
        }
        if PLATTERS.contains(&(x, y)) {
            return;
        }
        if CANDELABRA.contains(&(x, y)) {
            // A tall candelabrum of tarnished pewter, three candles guttering in it.
            let top = base + Vec3::new(0.0, TABLE_Y + 0.02, 0.0);
            cube(out, top + Vec3::new(0.0, 0.03, 0.0), Vec3::new(0.24, 0.05, 0.24), PI / 4.0, lin(PEWTER), 6.0);
            cube(out, top + Vec3::new(0.0, 0.3, 0.0), Vec3::new(0.05, 0.55, 0.05), 0.0, lin(PEWTER), 6.0);
            let along = if ARMS.iter().any(|a| a.0 == x) { Vec3::Z } else { Vec3::X };
            cube(
                out,
                top + Vec3::new(0.0, 0.58, 0.0),
                along * 0.5 + Vec3::new(0.0, 0.04, 0.0) + (Vec3::ONE - along) * 0.04,
                0.0,
                lin(PEWTER),
                6.0,
            );
            for s in [-1.0f32, 0.0, 1.0] {
                let c = top + along * (s * 0.24) + Vec3::new(0.0, 0.6 + if s == 0.0 { 0.08 } else { 0.0 }, 0.0);
                let tall = 0.14 + 0.06 * s.abs();
                cube(out, c + Vec3::new(0.0, tall / 2.0, 0.0), Vec3::new(0.055, tall, 0.055), 0.0, lin(WAX), 11.0);
                cube(out, c + Vec3::new(0.03, -0.03, 0.0), Vec3::new(0.02, 0.08, 0.02), 0.0, lin(WAX) * 0.9, 11.0);
                candles.push(c + Vec3::new(0.0, tall + 0.05, 0.0));
            }
            return;
        }
        // What lies on it.
        let top = base + Vec3::new(0.0, TABLE_Y + 0.02, 0.0);
        let a = (h * 47.0) as f32;
        match (h * 8.0) as usize {
            0 | 1 => {
                // A goblet tipped over in its own spilled wine.
                cube(out, top + Vec3::new(0.05, 0.002, 0.08), Vec3::new(0.4, 0.006, 0.3), a, lin(WINE), 8.0);
                let rot = Quat::from_rotation_y(a) * Quat::from_rotation_z(PI / 2.0);
                tilted(out, top + Vec3::new(0.0, 0.05, 0.0), Vec3::new(0.1, 0.14, 0.1), rot, lin(PEWTER), 6.0);
                tilted(
                    out,
                    top + Vec3::new(0.0, 0.05, 0.0) + rot * Vec3::new(0.0, -0.12, 0.0),
                    Vec3::new(0.03, 0.1, 0.03),
                    rot,
                    lin(PEWTER),
                    6.0,
                );
            }
            2 | 3 => {
                // A plate of gnawed bones.
                disc(out, top + Vec3::new(0.0, 0.01, 0.0), 0.2, 0.02, lin(CHINA), SHADE_LIT);
                for k in 0..3 {
                    let b = a + k as f32 * 1.2;
                    cube(out, top + Vec3::new(b.cos() * 0.06, 0.04, b.sin() * 0.06), Vec3::new(0.2, 0.035, 0.035), b, lin(BONE), 11.0);
                }
            }
            4 => {
                // A candle stub in a pewter stick, its wax run down.
                cube(out, top + Vec3::new(0.0, 0.02, 0.0), Vec3::new(0.16, 0.03, 0.16), 0.0, lin(PEWTER), 6.0);
                cube(out, top + Vec3::new(0.0, 0.12, 0.0), Vec3::new(0.07, 0.2, 0.07), 0.0, lin(WAX), 11.0);
                cube(out, top + Vec3::new(0.06, 0.01, 0.02), Vec3::new(0.12, 0.012, 0.09), 0.3, lin(WAX) * 0.9, 11.0);
                candles.push(top + Vec3::new(0.0, 0.27, 0.0));
            }
            5 => push_merged(out, fruit, Mat4::from_translation(top) * Mat4::from_rotation_y(a) * Mat4::from_scale(Vec3::splat(0.9)), 0.0),
            6 => {
                // A loaf furred with mould, a knife beside it.
                push_merged(out, bread, Mat4::from_translation(top) * Mat4::from_rotation_y(a), 0.0);
                cube(out, top + Vec3::new(0.25, 0.01, -0.1), Vec3::new(0.26, 0.01, 0.04), a + 0.4, lin(PEWTER), 6.0);
            }
            _ => {
                // A bottle, standing or lying, and crumbs.
                if h > 0.94 {
                    let rot = Quat::from_rotation_y(a) * Quat::from_rotation_z(PI / 2.0);
                    tilted(out, top + Vec3::new(0.0, 0.06, 0.0), Vec3::new(0.11, 0.26, 0.11), rot, lin(BOTTLE), 6.0);
                } else {
                    cube(out, top + Vec3::new(0.1, 0.14, 0.0), Vec3::new(0.11, 0.26, 0.11), a, lin(BOTTLE), 6.0);
                    cube(out, top + Vec3::new(0.1, 0.32, 0.0), Vec3::new(0.04, 0.1, 0.04), a, lin(BOTTLE), 6.0);
                }
                for k in 0..4 {
                    let b = a + k as f32 * 1.9;
                    cube(out, top + Vec3::new(b.cos() * 0.22, 0.01, b.sin() * 0.22), Vec3::splat(0.03), b, lin(CRUMB), SHADE_LIT);
                }
            }
        }
    }

    /// A platter on the table at `p`, and its food in its three states.
    fn build_platter(&mut self, p: (i32, i32), kind: FoodKind) -> Platter {
        let base = Vec3::new(p.0 as f32, TABLE_Y + 0.02, p.1 as f32);
        let mut ranges = [(0, 0); 4];
        let yaw = hash(p.0 as f64, p.1 as f64) as f32 * TAU;
        // The dish: a great tarnished platter on a foot.
        let start = self.static_cubes.len();
        let out = &mut self.static_cubes;
        disc(out, base + Vec3::new(0.0, 0.02, 0.0), 0.42, 0.03, lin(SILVER_D), 6.0);
        disc(out, base + Vec3::new(0.0, 0.035, 0.0), 0.36, 0.02, lin(SILVER_T), 6.0);
        ranges[0] = (start, self.static_cubes.len() - start);
        // Laden: the food, furred with mould, a silver dome tipped off beside it.
        let food = mouldy(&art::food(kind), 0.5);
        let start = self.static_cubes.len();
        let out = &mut self.static_cubes;
        push_merged(
            out,
            &food,
            Mat4::from_translation(base + Vec3::new(0.0, 0.045, 0.0)) * Mat4::from_rotation_y(yaw) * Mat4::from_scale(Vec3::splat(1.3)),
            0.0,
        );
        ranges[1] = (start, self.static_cubes.len() - start);
        // Eaten: bare, a few bones and crumbs left, the grease.
        let start = self.static_cubes.len();
        let out = &mut self.static_cubes;
        disc(out, base + Vec3::new(0.0, 0.048, 0.0), 0.24, 0.006, lin(GREASE), SHADE_LIT);
        for k in 0..4 {
            let a = yaw + k as f32 * 1.6;
            cube(out, base + Vec3::new(a.cos() * 0.14, 0.07, a.sin() * 0.14), Vec3::new(0.22, 0.035, 0.035), a, lin(BONE), 11.0);
        }
        for k in 0..6 {
            let a = yaw * 2.0 + k as f32 * 1.1;
            cube(out, base + Vec3::new(a.cos() * 0.25, 0.055, a.sin() * 0.25), Vec3::splat(0.03), a, lin(CRUMB), SHADE_LIT);
        }
        ranges[2] = (start, self.static_cubes.len() - start);
        // Spoiled: what he choked on, a blackened ruin oozing over the rim.
        let start = self.static_cubes.len();
        let out = &mut self.static_cubes;
        push_merged(
            out,
            &ruined(&art::food(kind)),
            Mat4::from_translation(base + Vec3::new(0.0, 0.045, 0.0)) * Mat4::from_rotation_y(yaw) * Mat4::from_scale(Vec3::splat(1.3)),
            0.0,
        );
        disc(out, base + Vec3::new(0.12, 0.05, 0.1), 0.3, 0.008, lin(0x2a3010), SHADE_LIT);
        ranges[3] = (start, self.static_cubes.len() - start);
        let mats = ranges.map(|(s, n)| self.static_cubes[s..s + n].iter().map(|i| i.m).collect::<Vec<_>>());
        Platter { ranges, mats, at: base }
    }

    /// A chair at `at` facing `yaw`; one in four lies toppled.
    fn place_chair(&mut self, chair: &Merged, at: Vec3, yaw: f32, h: f32) {
        let out = &mut self.static_cubes;
        if h > 0.75 {
            let m = Mat4::from_translation(at + Vec3::new(0.0, 0.22, 0.0))
                * Mat4::from_rotation_y(yaw + h * 2.0)
                * Mat4::from_rotation_x(-PI / 2.0 + 0.1);
            push_merged(out, chair, m * Mat4::from_translation(Vec3::new(0.0, -0.2, -0.2)), 0.0);
        } else {
            push_merged(out, chair, Mat4::from_translation(at) * Mat4::from_rotation_y(yaw + (h - 0.4) * 0.3), 0.0);
        }
    }

    /// The low dais against the north wall and his throne of piled plates on it: stacks of china
    /// for its seat, its arms and its back, and a fan of plates upright over it. Returns the seat.
    fn build_plate_throne(&mut self) -> Vec3 {
        let (x0, y0, x1, y1) = DAIS;
        let mid = Vec3::new((x0 + x1) as f32 / 2.0, 0.0, (y0 + y1) as f32 / 2.0);
        let (w, d) = ((x1 - x0 + 1) as f32, (y1 - y0 + 1) as f32);
        let out = &mut self.static_cubes;
        for k in 0..2 {
            let k = k as f32;
            let p = mid + Vec3::new(0.0, 0.11 + k * 0.22, -k * 0.3);
            cube(out, p, Vec3::new(w - k * 1.0, 0.22, d - k * 0.6), 0.0, lin(STONE), SHADE_MARBLE);
            cube(
                out,
                p + Vec3::new(0.0, 0.112, (d - k * 0.6) / 2.0 - 0.03),
                Vec3::new(w - k * 1.0, 0.012, 0.06),
                0.0,
                lin(OCHRE) * 0.6,
                SHADE_LIT,
            );
        }
        cube(out, mid + Vec3::new(0.0, 0.452, 0.35), Vec3::new(1.2, 0.012, d - 0.2), 0.0, lin(RUNNER_D), SHADE_LIT);
        let seat = mid + Vec3::new(0.0, 0.44, -0.35);
        let china = |k: usize, j: usize| {
            let h = ((k * 7 + j * 3) % 5) as f32 / 5.0;
            if (k + j).is_multiple_of(4) {
                lin(CHINA_D)
            } else {
                lin(CHINA) * (0.88 + h * 0.12)
            }
        };
        // A stack of `n` plates at `at`, each a little askew.
        let stack = |out: &mut Vec<Instance>, at: Vec3, n: usize, r: f32, k: usize| {
            for j in 0..n {
                let wob = Vec3::new(((j * 13 + k * 5) % 7) as f32 / 7.0 - 0.5, 0.0, ((j * 11 + k * 3) % 5) as f32 / 5.0 - 0.5) * 0.06;
                let p = at + wob + Vec3::new(0.0, 0.02 + j as f32 * 0.05, 0.0);
                // Each plate: its blue-banded rim, its white face a little smaller above it.
                let rim = if j % 4 == 2 { lin(0x8a5a3a) } else { lin(CHINA_RIM) };
                disc(out, p, r, 0.026, rim, SHADE_LIT);
                disc(out, p + Vec3::new(0.0, 0.016, 0.0), r * 0.86, 0.02, china(k, j), SHADE_LIT);
            }
        };
        // The seat: a broad drum of plates, a rotten cushion on it.
        for (k, off) in [(-0.3f32, 0.0f32), (0.3, 0.0), (0.0, -0.25), (0.0, 0.2)].iter().enumerate() {
            stack(out, seat + Vec3::new(off.0, 0.0, off.1), 12, 0.36, k);
        }
        cube(out, seat + Vec3::new(0.0, 0.66, 0.0), Vec3::new(1.0, 0.1, 0.8), 0.0, lin(RUNNER_D), 8.0);
        // The arms: two stacks either side, a goblet on each.
        for (k, s) in [-1.0f32, 1.0].into_iter().enumerate() {
            let at = seat + Vec3::new(s * 0.78, 0.0, 0.05);
            stack(out, at, 19, 0.24, 10 + k);
            cube(out, at + Vec3::new(0.0, 1.06, 0.0), Vec3::new(0.12, 0.16, 0.12), 0.0, lin(PEWTER), 6.0);
        }
        // The back: columns of plates rising in an arc, the middle the tallest.
        for k in 0..7 {
            let u = k as f32 / 6.0 - 0.5;
            let at = seat + Vec3::new(u * 1.7, 0.0, -0.55 - (u * u) * 0.5);
            let n = 30 + ((1.0 - u.abs() * 2.0) * 22.0) as usize;
            stack(out, at, n, 0.22, 20 + k);
        }
        // The crown: a fan of plates upright over the back.
        for k in 0..9 {
            let u = k as f32 / 8.0 - 0.5;
            let at = seat + Vec3::new(u * 2.0, 2.65 + (1.0 - (u * 2.0).powi(2)) * 0.55, -0.75);
            upright_plate(out, at, 0.26, u * 0.5, 0.35, china(k, 2), lin(CHINA_RIM));
        }
        // At its foot: a heap of plates, cracked and spilled, and bones.
        for k in 0..10 {
            let a = k as f32 * 0.9;
            let at = mid + Vec3::new(-1.6 + (k as f32 * 0.37) % 3.2, 0.46, 0.55 + (a.sin() * 0.2));
            let rot = Quat::from_rotation_y(a) * Quat::from_rotation_x(0.5 + (k % 3) as f32 * 0.3);
            tilted(out, at, Vec3::new(0.4, 0.03, 0.4), rot, china(k, 1), SHADE_LIT);
        }
        for k in 0..6 {
            let a = k as f32 * 1.4;
            cube(out, mid + Vec3::new(1.4 - k as f32 * 0.5, 0.48, 0.7 + a.cos() * 0.15), Vec3::new(0.24, 0.04, 0.04), a, lin(BONE), 11.0);
        }
        seat
    }

    /// A tattered banner of green cloth hung from the north wall at `at`: a fly in faded gold on it.
    fn build_banner(&mut self, at: Vec3) {
        let out = &mut self.static_cubes;
        cube(out, at + Vec3::new(0.0, 2.45, 0.0), Vec3::new(1.1, 0.05, 0.05), 0.0, lin(IRON), 6.0);
        for k in 0..6 {
            let u = -0.42 + k as f32 * 0.168;
            let drop = 1.3 + 0.35 * ((k * 7 % 5) as f32 / 5.0);
            cube(
                out,
                at + Vec3::new(u, 2.42 - drop / 2.0, 0.02),
                Vec3::new(0.17, drop, 0.02),
                0.0,
                lin(if k % 2 == 0 { BANNER } else { BANNER_D }),
                8.0,
            );
        }
        // The fly: a body, two wings, a crown over it.
        let f = at + Vec3::new(0.0, 1.75, 0.04);
        cube(out, f, Vec3::new(0.12, 0.34, 0.01), 0.0, lin(GOLD_F), 13.0);
        cube(out, f + Vec3::new(0.0, 0.22, 0.0), Vec3::new(0.14, 0.12, 0.01), 0.0, lin(GOLD_F), 13.0);
        for s in [-1.0f32, 1.0] {
            tilted(
                out,
                f + Vec3::new(s * 0.17, 0.08, 0.0),
                Vec3::new(0.26, 0.11, 0.01),
                Quat::from_rotation_z(s * 0.5),
                lin(GOLD_F) * 0.8,
                13.0,
            );
        }
        for k in 0..3 {
            cube(out, f + Vec3::new(-0.06 + k as f32 * 0.06, 0.34, 0.0), Vec3::new(0.035, 0.08, 0.01), 0.0, lin(GOLD_F), 13.0);
        }
    }

    /// Shows each platter as `dishes` says (index by `PLATTERS`; none: not laid).
    fn show_platters(&mut self, view: &mut BanquetView, dishes: &[Option<u8>]) {
        for (k, p) in view.platters.iter().enumerate() {
            let want = dishes.get(k).copied().flatten();
            if view.shown && view.dishes.get(k).copied().flatten() == want {
                continue;
            }
            for (part, &(start, len)) in p.ranges.iter().enumerate() {
                let show = match part {
                    0 => want.is_some(),
                    n => want == Some(n as u8 - 1),
                };
                if show {
                    for (j, m) in p.mats[part].iter().enumerate() {
                        self.static_cubes[start + j].m = *m;
                    }
                    self.static_patches.push((start, len));
                } else {
                    self.hide_range((start, len));
                }
            }
        }
        view.dishes = (0..view.platters.len()).map(|k| dishes.get(k).copied().flatten()).collect();
        view.shown = true;
    }

    /// The court tells the view what lies on the table: the platters laid (`laid`, in the order of
    /// `PLATTERS`), each laden (0), eaten bare (1) or spoiled (2).
    pub fn set_banquet(&mut self, laid: &[(i32, i32)], states: &[u8]) {
        let Some(mut view) = self.banquet.take() else { return };
        let dishes: Vec<Option<u8>> = (0..PLATTERS.len()).map(|k| if k < laid.len() { states.get(k).copied() } else { None }).collect();
        self.show_platters(&mut view, &dishes);
        self.banquet = Some(view);
    }

    /// Each frame: the candles and their light, the flies over the food and up in the dark and
    /// about their lord, his scatter as a cloud, the cauldrons' last breath, and the gate.
    pub(super) fn emit_gluttony_court(&self, s: &SceneState, time: f32, hero: Vec3, f: &mut Frame) {
        let Some(bv) = &self.banquet else { return };
        f.bounds = Vec4::new(-0.5, -0.5, s.grid.width as f32 - 0.5, s.grid.height as f32 - 0.5);
        // Guttering flames, leaning.
        for (i, c) in bv.candles.iter().enumerate() {
            if (c.x - hero.x).abs() > 16.0 || (c.z - hero.z).abs() > 16.0 {
                continue;
            }
            let fi = i as f32;
            let flick = (time * 9.0 + fi * 1.7).sin() * 0.5 + 0.5;
            let lean = Vec3::new((time * 2.3 + fi).sin() * 0.015, 0.0, (time * 1.9 + fi * 0.5).cos() * 0.015);
            let m = Mat4::from_translation(*c + lean + Vec3::new(0.0, 0.035, 0.0))
                * Mat4::from_scale(Vec3::new(0.035, 0.07 + flick * 0.03, 0.035));
            f.additive.push(Instance::new(m, lin(FLAME) * 2.4, 0.85, Vec3::ZERO, SHADE_UNLIT));
            let m = Mat4::from_translation(*c + lean * 2.0 + Vec3::new(0.0, 0.08, 0.0)) * Mat4::from_scale(Vec3::new(0.02, 0.04, 0.02));
            f.additive.push(Instance::new(m, lin(FLAME_TIP) * 2.0, 0.7, Vec3::ZERO, SHADE_UNLIT));
            glow_orb(f, *c + Vec3::new(0.0, 0.04, 0.0), 0.22, lin(CANDLE_LIGHT), 0.14 + 0.04 * flick);
        }
        for (i, p) in nearest(&bv.candles, hero, 6).into_iter().enumerate() {
            let flick = 3.6 + (time * 7.0 + i as f32 * 1.3).sin() * 0.5 + (time * 17.0 + i as f32).sin() * 0.25;
            f.lights.push(Light { pos: p + Vec3::new(0.0, 0.15, 0.0), color: lin(CANDLE_LIGHT) * flick, distance: 6.0 });
        }
        for &(wx, wz) in &WHEELS {
            let at = Vec3::new(wx, WHEEL_Y + 0.2, wz);
            f.lights.push(Light { pos: at, color: lin(CANDLE_LIGHT) * 10.0, distance: 12.0 });
        }
        f.lights.push(Light { pos: bv.throne + Vec3::new(0.0, 2.2, 1.8), color: lin(0xd8c890) * 3.0, distance: 6.5 });

        // The flies over each dish: a few over the bare ones, a swarm over the laden, a cloud over
        // the spoiled.
        for (k, p) in bv.platters.iter().enumerate() {
            let Some(state) = bv.dishes.get(k).copied().flatten() else { continue };
            let (n, r) = match state {
                0 => (12, 0.42),
                1 => (4, 0.3),
                _ => (20, 0.5),
            };
            flies(f, p.at + Vec3::new(0.0, 0.35, 0.0), n, r, 0.25, time, k as f32 * 3.7);
            if state == 0 {
                // A sickly breath of light over the rotting food.
                glow_orb(f, p.at + Vec3::new(0.0, 0.3, 0.0), 0.6, lin(SICKLY), 0.06);
                f.lights.push(Light { pos: p.at + Vec3::new(0.0, 0.7, 0.0), color: lin(SICKLY) * 1.6, distance: 3.2 });
            }
        }
        // The living dark: slow vortices of flies up in the hall's air.
        for (k, c) in [(10.0f32, 12.0f32), (22.0, 12.0), (12.0, 22.0), (21.0, 23.0), (16.0, 17.0)].iter().enumerate() {
            let at = Vec3::new(
                c.0 + (time * 0.13 + k as f32).sin() * 1.5,
                2.6 + (time * 0.2 + k as f32).sin() * 0.4,
                c.1 + (time * 0.11 + k as f32 * 2.0).cos() * 1.5,
            );
            flies(f, at, 36, 2.2, 0.9, time * 0.35, k as f32 * 11.0);
        }
        // The cauldrons: green scum bubbling now and then, a thread of foul vapour.
        for (k, c) in bv.cauldrons.iter().enumerate() {
            for j in 0..3 {
                let t = (time * 0.6 + k as f32 * 0.37 + j as f32 * 0.33).fract();
                let p = *c + Vec3::new(((k + j) as f32 * 1.3).sin() * 0.15, 0.02 + t * 0.9, ((k * 3 + j) as f32).cos() * 0.15);
                let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.08 + t * 0.22));
                f.additive.push(Instance::new(m, lin(0x4a5a2a) * 0.5, 0.18 * (1.0 - t), Vec3::ZERO, SHADE_UNLIT));
            }
            flies(f, *c + Vec3::new(0.0, 0.3, 0.0), 6, 0.35, 0.2, time, k as f32 * 5.3);
        }
        // Their lord, his court of flies about him, his own light; his scatter a cloud of them.
        for e in s.enemies {
            let Some(d) = e.duel.as_deref().filter(|d| d.alive() && d.breed.kind == EnemyKind::Beelzebub) else { continue };
            let p = Vec3::new(d.pos.x as f32, 0.0, d.pos.y as f32);
            let ahead = Vec3::new(d.facing.x as f32, 0.0, d.facing.y as f32);
            match d.f.state {
                State::Attack { mv: MoveId::Scatter, frame } => {
                    let data = MoveId::Scatter.data();
                    let gone = d.lore.vanish > 0;
                    // Gathering into a cloud as he bursts, a dense swarm as he drifts, gathering again.
                    let dense = if gone {
                        1.0
                    } else if frame < data.startup {
                        (frame as f32 / 12.0).min(1.0)
                    } else {
                        0.3
                    };
                    let n = (40.0 + 90.0 * dense) as usize;
                    flies(f, p + Vec3::new(0.0, 1.4, 0.0), n, 0.6 + 0.6 * dense, 1.0, time * 1.6, 7.0);
                    if gone {
                        glow_orb(f, p + Vec3::new(0.0, 1.3, 0.0), 1.4, lin(0x2a3010), 0.18);
                    }
                }
                State::Attack { mv: MoveId::Gorge, .. } => {
                    // Eating: the flies thick about his face.
                    flies(f, p + ahead * 0.6 + Vec3::new(0.0, 2.2, 0.0), 30, 0.55, 0.35, time * 1.4, 3.0);
                }
                _ => flies(f, p + Vec3::new(0.0, 2.4, 0.0), 16, 1.2, 0.7, time, 1.0),
            }
            if d.lore.vanish == 0 {
                f.lights.push(Light { pos: p + ahead * 2.2 + Vec3::new(0.0, 3.4, 0.0), color: lin(0xf0d8a0) * 5.0, distance: 7.0 });
                f.lights.push(Light { pos: p - ahead * 1.6 + Vec3::new(0.0, 3.0, 0.0), color: lin(SICKLY) * 3.5, distance: 5.5 });
            }
        }
        // The gate he came in by.
        let door = s.gate.map(|g| g.door).unwrap_or(0.0);
        self.emit_portal(&bv.gate, door, false, time, hero, f);
    }

    /// His bile on the floor: a glob arcing from his mouth, then a puddle growing yellow-green as
    /// its fuse burns (the tell), then a splash, and the pool lying there pulsing and bubbling while
    /// it burns.
    pub(super) fn emit_bile(&self, traps: &[Trap], grid: &Grid, time: f32, f: &mut Frame) {
        for t in traps {
            let ground = floor_at(grid, t.pos.x as f32, t.pos.y as f32);
            let at = Vec3::new(t.pos.x as f32, ground, t.pos.y as f32);
            let seed = (t.pos.x * 3.7 + t.pos.y * 1.9) as f32;
            let r = t.radius as f32;
            if let Some(b) = t.burst {
                if t.lingering() {
                    let left = 1.0 - b as f32 / t.linger.max(1) as f32;
                    let pulse = 0.75 + 0.25 * (time * 5.0 + seed).sin();
                    let m = Mat4::from_translation(at + Vec3::new(0.0, 0.02, 0.0))
                        * Mat4::from_rotation_x(-PI / 2.0)
                        * Mat4::from_scale(Vec3::new(r, r, 1.0));
                    f.transparent[MeshKind::Disc as usize].push(Instance::new(
                        m,
                        lin(BILE) * pulse,
                        0.55 * left.max(0.25),
                        lin(BILE_D) * 0.4,
                        SHADE_UNLIT,
                    ));
                    for k in 0..4 {
                        let u = (time * 1.3 + k as f32 * 0.29 + seed).fract();
                        let a = seed + k as f32 * 2.1;
                        let p = at + Vec3::new(a.cos() * r * 0.55, 0.03 + u * 0.08, a.sin() * r * 0.55);
                        let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.05 + 0.05 * u));
                        f.transparent[MeshKind::Sphere as usize].push(Instance::new(
                            m,
                            lin(BILE_L),
                            0.6 * (1.0 - u) * left,
                            lin(BILE) * 0.3,
                            SHADE_UNLIT,
                        ));
                    }
                    if b % 30 < 6 {
                        // (Each sting: a hiss of vapour.)
                        glow_orb(f, at + Vec3::new(0.0, 0.2, 0.0), r, lin(BILE), 0.12);
                    }
                }
                if b >= SPIKES {
                    continue;
                }
                // The splash: gobbets thrown up out of the floor, falling back.
                let u = b as f32 / SPIKES as f32;
                for k in 0..10 {
                    let a = k as f32 * 2.39 + seed;
                    let out = r * (0.2 + 0.8 * u) * (0.4 + 0.6 * ((k * 37 % 10) as f32 / 10.0));
                    let up = (1.0 - (2.0 * u - 0.6).powi(2)).max(0.0) * (0.4 + 0.3 * ((k * 13 % 7) as f32 / 7.0));
                    let p = at + Vec3::new(a.cos() * out, up, a.sin() * out);
                    let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.07 * (1.0 - u * 0.5)));
                    f.transparent[MeshKind::Sphere as usize].push(Instance::new(m, lin(BILE_L), 0.85, lin(BILE) * 0.5, SHADE_UNLIT));
                }
                f.lights.push(Light { pos: at + Vec3::new(0.0, 0.5, 0.0), color: lin(BILE_L) * 3.0 * (1.0 - u), distance: 2.5 });
                continue;
            }
            if t.flight > 0 {
                // In the air: an arc from his mouth.
                let total = crate::logic::combat::prince::COIN_FLIGHT as f32 + 1.0;
                let k = 1.0 - t.flight as f32 / total;
                let from = Vec3::new(t.from.x as f32, ground, t.from.y as f32);
                let p = from.lerp(at, k) + Vec3::new(0.0, 2.4 * (1.0 - k) + 1.6 * k * (1.0 - k) * 2.0, 0.0);
                let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(0.16, 0.13, 0.16));
                f.transparent[MeshKind::Sphere as usize].push(Instance::new(m, lin(BILE), 0.9, lin(BILE_D) * 0.4, SHADE_UNLIT));
                continue;
            }
            // On the floor: the puddle spreading as its fuse burns, its rim a ring of dots, blinking
            // as it comes due.
            let fill = t.tell();
            let late = t.fuse <= 8;
            let blink = if late { 0.6 + (time * 40.0).sin().abs() * 0.8 } else { 0.0 };
            let n = ((TAU * r) / 0.22).ceil() as i32;
            for k in 0..n {
                let a = k as f32 * TAU / n as f32;
                let p = at + Vec3::new(a.cos() * r, 0.03, a.sin() * r);
                let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(0.08, 0.012, 0.08));
                f.additive.push(Instance::new(m, lin(BILE_L) * (0.8 + blink), 0.7, Vec3::ZERO, SHADE_UNLIT));
            }
            let spread = 0.25 + 0.75 * fill;
            let m = Mat4::from_translation(at + Vec3::new(0.0, 0.025, 0.0))
                * Mat4::from_rotation_x(-PI / 2.0)
                * Mat4::from_scale(Vec3::new(r * spread, r * spread, 1.0));
            f.transparent[MeshKind::Disc as usize].push(Instance::new(
                m,
                lin(BILE_D) * (0.7 + blink * 0.5),
                0.35 + 0.35 * fill,
                Vec3::ZERO,
                SHADE_UNLIT,
            ));
        }
    }
}

/// `n` flies about `at`: each on its own darting loop within `r` across and `h` up and down, black
/// bodies and pale blurred wings.
fn flies(f: &mut Frame, at: Vec3, n: usize, r: f32, h: f32, time: f32, seed: f32) {
    for k in 0..n {
        let kf = k as f32;
        let s = seed + kf * 1.618;
        let speed = 1.3 + (s * 7.3).sin().abs() * 1.6;
        let a = time * speed + s * 2.7;
        let rr = r * (0.35 + 0.65 * ((s * 3.1).sin() * 0.5 + 0.5));
        let dart = (time * 6.0 + s).sin() * 0.06;
        let p = at + Vec3::new((a).cos() * rr + dart, (a * 1.7 + s).sin() * h * 0.5, (a * 1.3).sin() * rr - dart);
        let body = Mat4::from_scale_rotation_translation(Vec3::new(0.032, 0.026, 0.042), Quat::from_rotation_y(a), p);
        f.opaque[MeshKind::Cube as usize].push(Instance::new(body, lin(FLY), 1.0, lin(0x3a4a1a) * 0.12, SHADE_LIT));
        if k % 2 == 0 {
            let wing = Mat4::from_scale_rotation_translation(
                Vec3::new(0.08, 0.005, 0.035),
                Quat::from_rotation_y(a + 1.2),
                p + Vec3::new(0.0, 0.02, 0.0),
            );
            f.additive.push(Instance::new(wing, lin(0x8a9078) * 0.35, 0.16, Vec3::ZERO, SHADE_UNLIT));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mould_spoils_and_cold_copper_keep_the_food_its_shape() {
        let food = art::food(FoodKind::Roast);
        let m = mouldy(&food, 0.5);
        assert_eq!(m.boxes.len(), food.boxes.len());
        assert!(m.boxes.iter().zip(&food.boxes).all(|(a, b)| a.center == b.center));
        let r = ruined(&food);
        let top = |m: &Merged| m.boxes.iter().map(|b| b.center.y).fold(0.0, f32::max);
        assert!(top(&r) < top(&food) * 0.5, "a ruin lies low");
        let c = art::cauldron();
        let cold = recolored(&c, lin(0x7a4a1a), lin(SCUM), Vec3::ZERO, 0.45);
        assert_eq!(cold.boxes.len(), c.boxes.len());
    }
}
