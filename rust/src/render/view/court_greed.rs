//! Greed's court, the Counting House (`logic::court` lays it out), at the Cisterns' bar and in the
//! Vaults of Mammon's materials (`vault_art`): their dressed stone, their gold as metal.
//!
//! An octagonal hall floored in charcoal marble veined with gold, a ring of coin mosaic round its
//! middle and Mammon's seal at its heart; walls of the crypt's dressed stone, warmed to umber,
//! under a gilt band and a pale cornice, black marble pilasters banded in gilt along them, tall
//! cases of ledgers to east and west, Greed's banners to the north and strongroom doors either
//! side of the gate; eight black-and-gilt pillars in a ring; coin heaped in the corners and
//! drifted against the walls, chests spilling theirs; counting tables piled with ledgers, a
//! candle burning on each, east and west. At the north end Mammon's throne of gold and red velvet
//! stands on a stepped dais between gilt candelabra, under his giant brass scales, a heap of
//! coins in each pan. Copper braziers hang on chains from the dark above, burning amber, each a
//! pool on the polished floor; the far end glows faintly teal where the cold light falls on the
//! scales. The gate he came in by is Greed's portal in the south wall. His coins on the floor
//! (`prince::Trap`) are drawn here too, wherever they fall: gold that glints as it flies.

use super::crypt::{jit, masonry, Wall, SHADE_BRONZE, SHADE_CLOTH, SHADE_IRON, SHADE_STONE, SHADE_WOOD};
use super::vault::{glint, SHADE_MARBLE};
use super::vault_art::{self, art, coin_stack, gold, GOLD_GLOW, SHADE_GILT};
use super::vault_decor::{self, push_merged};
use super::*;
use crate::logic::combat::prince::{Trap, SPIKES};
use crate::logic::combat::MoveId;
use crate::logic::court::{self, Piece, CENTRE, DAIS, MOSAIC, SCALES, TABLES};
use crate::logic::hub::Gate;

pub(super) mod pal {
    pub const MARBLE: u32 = 0x2e2a28;
    pub const MARBLE_L: u32 = 0x3c3634;
    pub const MARBLE_D: u32 = 0x221f1e;
    pub const BLACK: u32 = 0x1a1818;
    pub const COPPER: u32 = 0xb06a44;
    pub const COPPER_D: u32 = 0x7a4630;
    /// Brass by its reflectance (the scales).
    pub const BRASS: u32 = 0xf0d690;
    pub const BRASS_D: u32 = 0xc8a860;
    pub const GOLD: u32 = 0xe2b040;
    pub const GOLD_L: u32 = 0xfbe08a;
    pub const STONE: u32 = 0xb8ae98;
    pub const VELVET: u32 = 0x6a1a1c;
    pub const VELVET_D: u32 = 0x4a1014;
    pub const WOOD: u32 = 0x3e2a1c;
    pub const WOOD_L: u32 = 0x5a3e28;
    pub const PAGE: u32 = 0xd8ccb0;
    pub const LEDGER: [u32; 4] = [0x4a3426, 0x2e4a46, 0x5a2a20, 0x2a2622];
    pub const FLAME: u32 = 0xffb050;
    pub const FLAME_TIP: u32 = 0xff6a20;
    pub const AMBER: u32 = 0xffb060;
    pub const CANDLE: u32 = 0xffc080;
    pub const MIST_TEAL: u32 = 0x6e9a90;
}
use pal::*;

/// The hall's walls and pillars stand this tall; the masonry rises to `FACE_H`.
const WALL_H: f32 = 2.4;
const FACE_H: f32 = 2.16;
const PILLAR_H: f32 = 3.0;
/// How high the braziers hang.
const BRAZIER_Y: f32 = 2.5;

/// Dark and warm: the braziers' amber pooling on the polished floor and the gold, a dim light
/// from shafts high above, a cold teal fill on the scales, the far dark cool.
pub fn counting_atmosphere() -> Atmosphere {
    Atmosphere {
        background: 0x121515,
        fog_near: 16.0,
        fog_far: 44.0,
        hemi_sky: lin(0x8a6440) * 0.3,
        hemi_ground: lin(0x4a3422) * 0.16,
        ambient: lin(0x2e2924) * 0.75,
        sun_dir: Vec3::new(-0.35, 1.0, 0.5).normalize(),
        sun: lin(0xd8c8a8) * 0.32,
        exposure: 1.3,
        night_sky: 0.0,
        bloom: 0.9,
        moon_view: Vec3::new(0.0, 0.3, -1.0).normalize(),
        cloud_tint: lin(0x1a1008),
    }
}

/// The Counting House's ground haze (density at the floor, falloff).
pub const HAZE: (f32, f32) = (0.024, 1.4);

/// What the view keeps of the Counting House: its braziers and candles, its gate, the scales and
/// the throne.
pub struct CountingView {
    pub braziers: Vec<Vec3>,
    /// The giant statues' feet (each lit from below).
    pub statues: Vec<Vec3>,
    pub candles: Vec<Vec3>,
    pub gate: Gate,
    pub scales: Vec3,
    pub throne: Vec3,
}

fn cube(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot_y: f32, color: Vec3, shade: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(rot_y), pos), color, 1.0, Vec3::ZERO, shade));
}

fn marble(h: f64) -> Vec3 {
    let set = [MARBLE, MARBLE, MARBLE_L, MARBLE_D];
    lin(set[(h * 4.0) as usize % 4]) * (0.94 + h as f32 * 0.1)
}

/// What dresses a straight wall's face between the pilasters.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Dress {
    Plain,
    Sconce,
    Ledgers,
    Banner,
    Door,
    Drift,
}

impl WorldView {
    /// The Counting House's static scene.
    pub(super) fn build_counting_house(&mut self, grid: &Grid) {
        let c = Vec3::new(CENTRE.0 as f32, 0.0, CENTRE.1 as f32);
        let floor_tile = |x: i32, y: i32| court::in_hall(x, y);
        let built = self.static_cubes.len();

        // ---- the floor: charcoal marble veined with gold, the mosaic ring, the seal at its heart
        for y in 0..grid.height {
            for x in 0..grid.width {
                if !floor_tile(x, y) && !(x == court::GATE.0 && y == court::GATE.1) {
                    continue;
                }
                let (xf, yf) = (x as f32, y as f32);
                let h = hash(x as f64 * 1.7 + 3.0, y as f64 * 2.3);
                let out = &mut self.static_cubes;
                cube(out, Vec3::new(xf, -0.06, yf), Vec3::new(0.98, 0.12, 0.98), 0.0, marble(h), SHADE_MARBLE);
                let d = Vec2::new(xf - c.x, yf - c.z).length();
                if h > 0.8 && (d - MOSAIC as f32).abs() > 1.2 && d > 2.6 {
                    // A hairline of gold wandering across the slab, dimmed into the stone.
                    let a = hash(x as f64 * 5.0, y as f64 * 7.0) as f32 * PI;
                    let o = Vec3::new(xf + (h as f32 - 0.9) * 0.5, 0.003, yf + (hash(y as f64, x as f64) as f32 - 0.5) * 0.4);
                    let dir = Vec3::new(a.cos(), 0.0, -a.sin());
                    for (k, bend) in [(-1.0f32, 0.25f32), (0.0, 0.0), (1.0, -0.2)] {
                        cube(out, o + dir * (k * 0.26), Vec3::new(0.28, 0.008, 0.012), a + bend, gold(h) * 0.5, SHADE_GILT);
                    }
                }
            }
        }
        // The coin mosaic: a band of gold and copper coins set edge to edge, rimmed in pale stone.
        let ring_n = (TAU * MOSAIC as f32 / 0.34).round() as i32;
        for band in 0..3 {
            let r = MOSAIC as f32 - 0.34 + band as f32 * 0.34;
            let n = (TAU * r / 0.34).round() as i32;
            for k in 0..n {
                let a = (k as f32 + band as f32 * 0.5) * TAU / n as f32;
                let p = c + Vec3::new(a.cos() * r, 0.004, a.sin() * r);
                let (col, shade) =
                    if (k + band) % 3 == 0 { (lin(COPPER), SHADE_BRONZE) } else { (gold(hash(k as f64, band as f64)) * 0.85, SHADE_GILT) };
                cube(&mut self.static_cubes, p, Vec3::new(0.24, 0.016, 0.24), a + PI / 4.0, col, shade);
            }
        }
        for r in [MOSAIC as f32 - 0.62, MOSAIC as f32 + 0.62] {
            for k in 0..ring_n {
                let a = k as f32 * TAU / ring_n as f32;
                let p = c + Vec3::new(a.cos() * r, 0.003, a.sin() * r);
                cube(&mut self.static_cubes, p, Vec3::new(0.33, 0.012, 0.07), -a + PI / 2.0, lin(STONE) * 0.75, SHADE_MARBLE);
            }
        }
        // The seal at the heart: a great coin, rings of gold and copper, an eight-rayed star in it.
        for (r, col) in [(1.9f32, art::GOLD_DARK), (1.6, COPPER), (1.25, art::GOLD), (0.9, BLACK), (0.55, art::GOLD_PALE)] {
            let n = (TAU * r / 0.22).round().max(8.0) as i32;
            let shade = match col {
                COPPER => SHADE_BRONZE,
                BLACK => SHADE_MARBLE,
                _ => SHADE_GILT,
            };
            for k in 0..n {
                let a = k as f32 * TAU / n as f32;
                cube(
                    &mut self.static_cubes,
                    c + Vec3::new(a.cos() * r, 0.005, a.sin() * r),
                    Vec3::new(0.24, 0.014, 0.24),
                    a,
                    lin(col),
                    shade,
                );
            }
        }
        for k in 0..8 {
            let a = k as f32 * TAU / 8.0;
            cube(
                &mut self.static_cubes,
                c + Vec3::new(a.cos() * 0.45, 0.008, a.sin() * 0.45),
                Vec3::new(0.9, 0.012, 0.08),
                -a,
                lin(art::GOLD_PALE),
                SHADE_GILT,
            );
        }

        // ---- the walls: dressed stone under a gilt band and a pale cornice, black pilasters
        let pile = super::vault::pile();
        let mut candles = Vec::new();
        for (x, y, t) in grid.cells() {
            if t != Tile::Rock || floor_tile(x, y) || (x == court::GATE.0 && y == court::GATE.1) {
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
            if !diagonal {
                continue;
            }
            // (Either side of the gate the portal's own towers stand.)
            if y == court::GATE.1 && (x - court::GATE.0).abs() <= 1 {
                continue;
            }
            let start = self.static_cubes.len();
            candles.extend(self.counting_wall(P::new(x, y), &open, &pile));
            let len = self.static_cubes.len() - start;
            self.rock_index.insert(tile_key(x, y), (start, len));
            let shades: Vec<f32> = self.static_cubes[start..start + len].iter().map(|i| i.emissive[3]).collect();
            self.trees.push(Tree {
                range: (start, len),
                base: Vec3::new(x as f32, 0.0, y as f32),
                radius: 0.72,
                height: WALL_H + 0.3,
                fade: 1.0,
                alive: true,
                shades: Some(shades),
            });
        }

        // ---- what stands on the floor
        let (chest, lid) = super::vault::chest();
        let treasure = vault_decor::treasure();
        for (x, y, _) in grid.cells() {
            let Some(piece) = court::piece(x, y) else { continue };
            let base = Vec3::new(x as f32, 0.0, y as f32);
            let h = hash(x as f64 * 2.1, y as f64 * 1.3) as f32;
            let start = self.static_cubes.len();
            match piece {
                Piece::Pillar => self.build_court_pillar(base, h),
                Piece::Heap => {
                    // A great heap of coins, treasure half buried in it, coins strewn round its foot.
                    let out = &mut self.static_cubes;
                    push_merged(
                        out,
                        &pile,
                        Mat4::from_translation(base) * Mat4::from_rotation_y(h * TAU) * Mat4::from_scale(Vec3::new(2.4, 2.0, 2.4)),
                        GOLD_GLOW,
                    );
                    push_merged(
                        out,
                        &pile,
                        Mat4::from_translation(base + Vec3::new(0.45, 0.0, -0.3))
                            * Mat4::from_rotation_y(h * 3.0)
                            * Mat4::from_scale(Vec3::splat(1.6)),
                        GOLD_GLOW,
                    );
                    push_merged(
                        out,
                        &treasure,
                        Mat4::from_translation(base + Vec3::new(-0.35, 0.2, 0.25)) * Mat4::from_rotation_y(h * 5.0),
                        GOLD_GLOW,
                    );
                    for k in 0..7 {
                        let a = h * 9.0 + k as f32 * 0.9;
                        let r = 0.75 + (k % 3) as f32 * 0.18;
                        coin_stack(
                            out,
                            Mat4::from_translation(base + Vec3::new(a.cos() * r, 0.0, a.sin() * r)) * Mat4::from_rotation_y(a),
                            a,
                        );
                    }
                }
                Piece::Chest => {
                    // A chest against the wall, its lid thrown back, its coins spilling onto the floor.
                    let toward = if x < CENTRE.0 { -PI / 2.0 + PI } else { -PI / 2.0 };
                    let yaw = toward + (h - 0.5) * 0.3;
                    let m = Mat4::from_translation(base) * Mat4::from_rotation_y(yaw) * Mat4::from_scale(Vec3::splat(1.5));
                    let out = &mut self.static_cubes;
                    push_merged(out, &chest, m, 0.0);
                    push_merged(out, &lid, super::vault::lid_matrix(m, -1.9), 0.0);
                    let front = Vec3::new(yaw.sin(), 0.0, yaw.cos());
                    push_merged(
                        out,
                        &pile,
                        Mat4::from_translation(base + front * 0.55) * Mat4::from_scale(Vec3::new(1.2, 0.5, 1.2)),
                        GOLD_GLOW,
                    );
                    for k in 0..4 {
                        let side = Vec3::new(front.z, 0.0, -front.x);
                        let p = base + front * (0.75 + k as f32 * 0.12) + side * ((k as f32 - 1.5) * 0.25);
                        coin_stack(out, Mat4::from_translation(p) * Mat4::from_rotation_y(k as f32), h * 31.0 + k as f32);
                    }
                }
                Piece::Table => {
                    if let Some(candle) = self.build_counting_table(base, x, h) {
                        candles.push(candle);
                    }
                }
                Piece::Dais => {}
            }
            if piece != Piece::Dais {
                let len = self.static_cubes.len() - start;
                self.rock_index.insert(tile_key(x, y), (start, len));
                if piece == Piece::Pillar {
                    let shades: Vec<f32> = self.static_cubes[start..start + len].iter().map(|i| i.emissive[3]).collect();
                    self.trees.push(Tree {
                        range: (start, len),
                        base,
                        radius: 0.5,
                        height: PILLAR_H,
                        fade: 1.0,
                        alive: true,
                        shades: Some(shades),
                    });
                }
            }
        }

        // ---- the dais, the throne between its candelabra, and the great scales behind it
        let throne = self.build_throne();
        for s in [-1.0f32, 1.0] {
            let base = Vec3::new(CENTRE.0 as f32 + s * 3.9, 0.0, DAIS.3 as f32 + 0.1);
            self.vault_candelabrum(base, 0.3 + 0.2 * s as f64);
            candles.push(base + Vec3::new(0.0, 1.32, 0.0));
        }
        let scales = self.build_scales(&pile);
        // Two giant gilt statues of Mammon stand on the north wall either side of the scales,
        // looming over the hall (they fade out of the way of the view of him, as the walls do).
        let mammon = &vault_decor::statues()[4];
        let mut statues = Vec::new();
        for s in [-1.0f32, 1.0] {
            let base = Vec3::new(CENTRE.0 as f32 + s * 4.6, WALL_H, 1.9);
            statues.push(base);
            let start = self.static_cubes.len();
            let out = &mut self.static_cubes;
            cube(out, base + Vec3::Y * 0.16, Vec3::new(1.5, 0.32, 1.1), 0.0, lin(art::PALE[0]) * 0.55, SHADE_STONE);
            cube(out, base + Vec3::Y * 0.34, Vec3::new(1.56, 0.05, 1.16), 0.0, gold(0.4 + s as f64 * 0.1) * 0.85, SHADE_GILT);
            push_merged(out, mammon, Mat4::from_translation(base + Vec3::Y * 0.36) * Mat4::from_scale(Vec3::splat(2.3)), 0.0);
            let len = self.static_cubes.len() - start;
            let shades: Vec<f32> = self.static_cubes[start..start + len].iter().map(|i| i.emissive[3]).collect();
            self.trees.push(Tree {
                range: (start, len),
                base: Vec3::new(base.x, 0.0, base.z),
                radius: 0.9,
                height: WALL_H + 3.3,
                fade: 1.0,
                alive: true,
                shades: Some(shades),
            });
        }

        // ---- the hanging braziers: a ring of eight between the pillars, and two by the throne
        let mut braziers = Vec::new();
        for k in 0..8 {
            // (In line with the pillars, inside them: none hangs over the way between him and the throne.)
            let a = (k as f32 + 0.5) * TAU / 8.0;
            braziers.push(c + Vec3::new(a.cos() * 7.8, BRAZIER_Y, a.sin() * 7.8));
        }
        braziers.push(Vec3::new(DAIS.0 as f32 - 0.6, BRAZIER_Y + 0.2, DAIS.1 as f32 + 1.4));
        braziers.push(Vec3::new(DAIS.2 as f32 + 0.6, BRAZIER_Y + 0.2, DAIS.1 as f32 + 1.4));
        for &b in &braziers {
            self.build_brazier(b);
        }
        // (Built in its own materials and colours: the grade leaves it be.)
        self.authored.push((built, self.static_cubes.len() - built));

        // ---- the gate: Greed's portal in the south wall, looking into the hall
        let gate = Gate { tile: P::new(court::GATE.0, court::GATE.1), facing: Dir::Up, sin: crate::logic::sins::Sin::Greed };
        self.build_court_portal(&gate);
        self.counting = Some(CountingView { braziers, statues, candles, gate, scales, throne });
    }

    /// A wall tile of the Counting House at `p`, its faces toward `open`: dressed stone under a gilt
    /// band and a pale cornice, a black pilaster banded in gilt on every other block, and on the
    /// straight walls ledger cases (east and west), Greed's banners (north), strongroom doors
    /// (south) and coin drifted against the foot.
    fn counting_wall(&mut self, p: P, open: &[Dir], pile: &vault_decor::Merged) -> Option<Vec3> {
        let (x, y) = (p.x, p.y);
        let mut candle = None;
        let (xf, yf) = (x as f32, y as f32);
        let (hx, hy) = (x as f64, y as f64);
        let out = &mut self.static_cubes;
        let cc = Vec3::new(xf, 0.0, yf);
        // Down the straight walls, every other block between the pilasters is dressed.
        let dress_of = |d: Dir| {
            let (dx, _) = d.vec();
            let u = if dx == 0 { x } else { y };
            let (wx, wy) = (CENTRE.0 + court::HALF + 1, CENTRE.1 + court::HALF + 1);
            let straight = open.len() == 1 && (x == CENTRE.0 * 2 - wx || x == wx || y == CENTRE.1 * 2 - wy || y == wy);
            let near_gate = d == Dir::Up && (x - court::GATE.0).abs() <= 4;
            if (x + y) % 2 == 0 || !straight {
                Dress::Plain
            } else if near_gate {
                Dress::Door
            } else if dx != 0 && (u - CENTRE.1).abs() >= 3 && (u - CENTRE.1).abs() <= 7 {
                Dress::Ledgers
            } else if d == Dir::Down && (u - CENTRE.0).abs() >= 5 {
                Dress::Banner
            } else if hash(hx * 1.7, hy * 2.3) < 0.45 {
                Dress::Drift
            } else {
                Dress::Sconce
            }
        };
        // The core, cut back where a case or a door is sunk into a face.
        let (mut lo, mut hi) = (Vec3::new(-0.46, 0.0, -0.46), Vec3::new(0.46, 0.0, 0.46));
        for &d in open {
            let cut = match dress_of(d) {
                Dress::Ledgers => 0.3,
                Dress::Door => 0.2,
                _ => 0.0,
            };
            let (dx, dy) = d.vec();
            if dx > 0 {
                hi.x -= cut;
            } else if dx < 0 {
                lo.x += cut;
            } else if dy > 0 {
                hi.z -= cut;
            } else {
                lo.z += cut;
            }
        }
        let mid = (lo + hi) / 2.0;
        cube(
            out,
            Vec3::new(xf + mid.x, (FACE_H + 0.06) / 2.0, yf + mid.z),
            Vec3::new(hi.x - lo.x, FACE_H + 0.06, hi.z - lo.z),
            0.0,
            jit(art::CORE, hash(hx, hy), 0.06),
            SHADE_STONE,
        );
        cube(out, Vec3::new(xf, WALL_H - 0.06, yf), Vec3::new(1.06, 0.12, 1.06), 0.0, jit(art::CAP, hash(hy, hx), 0.08), SHADE_STONE);
        for &d in open {
            let (dx, dy) = d.vec();
            let n = Vec3::new(dx as f32, 0.0, dy as f32);
            let along = Vec3::new(dy.abs() as f32, 0.0, dx.abs() as f32);
            let dress = dress_of(d);
            // (The pilaster at the block's one end stands in a cut of the courses; nothing on a
            // face reaches past `vault_art::LIP` from the wall's middle.)
            // (Only where the wall runs on along the face: not over a step of the octagon's sides.)
            let (ax, az) = (dy.abs(), dx.abs());
            let pilaster = (x + y) % 2 == 0 && !court::in_hall(x + ax, y + az);
            let edge = if (x + y) % 2 == 0 { 0.5 } else { -0.5 };
            let edge = if pilaster || ((x + y) % 2 == 1 && !court::in_hall(x - ax, y - az)) { edge } else { 9.0 };
            let openings: &[(f32, f32, f32, f32, f32)] = match dress {
                Dress::Door => &[(0.0, 0.34, 0.0, 1.35, 0.0), (edge, 0.11, 0.0, FACE_H, 0.0)],
                Dress::Ledgers => &[(0.0, 0.42, 0.0, 1.95, 0.0), (edge, 0.11, 0.0, FACE_H, 0.0)],
                Dress::Sconce => &[(0.0, 0.15, 1.1, 1.52, 0.0), (edge, 0.11, 0.0, FACE_H, 0.0)],
                _ => &[(edge, 0.11, 0.0, FACE_H, 0.0)],
            };
            let start = out.len();
            masonry(
                out,
                &Wall {
                    centre: cc,
                    dir: (dx, dy),
                    half: 0.5,
                    base: 0.0,
                    height: FACE_H,
                    courses: 6,
                    seed: (hx, hy),
                    len: (0.32, 0.26),
                    depth: vault_art::STONE_DEPTH,
                    face: vault_art::STONE_FACE,
                    wet: false,
                    ruined: false,
                    foot: None,
                    openings,
                },
            );
            vault_art::warm(out, start);
            let pale = jit(art::PALE[(hash(hx + dx as f64, hy) * 2.0) as usize % 2], hash(hy, hx), 0.05) * 0.6;
            cube(
                out,
                cc + n * 0.52 + Vec3::Y * (FACE_H + 0.03),
                along * 1.0 + n * 0.04 + Vec3::Y * 0.045,
                0.0,
                gold(hash(hx, hy)) * 0.9,
                SHADE_GILT,
            );
            cube(out, cc + n * 0.47 + Vec3::Y * (FACE_H + 0.13), along * 1.02 + n * 0.14 + Vec3::Y * 0.16, 0.0, pale, SHADE_STONE);
            cube(out, cc + n * 0.5 + Vec3::Y * (FACE_H + 0.22), along * 1.04 + n * 0.08 + Vec3::Y * 0.05, 0.0, pale * 1.08, SHADE_STONE);
            if pilaster {
                // A black marble pilaster banded in gilt, up to the cornice.
                let f = cc + n * 0.47 + along * 0.5;
                cube(out, f + Vec3::Y * (FACE_H / 2.0), along * 0.2 + n * 0.12 + Vec3::Y * FACE_H, 0.0, lin(BLACK), SHADE_MARBLE);
                for yy in [0.25f32, 1.1, 1.95] {
                    cube(out, f + Vec3::Y * yy, along * 0.23 + n * 0.14 + Vec3::Y * 0.06, 0.0, gold(hash(hx + yy as f64, hy)), SHADE_GILT);
                }
            }
            let g = gold(hash(hx * 2.3, hy * 1.9));
            match dress {
                Dress::Plain => {}
                Dress::Sconce => {
                    // A gilt sconce of two candles, a practical lighting the wall.
                    let f = cc + n * 0.46;
                    cube(out, f + n * 0.01 + Vec3::Y * 1.32, along * 0.16 + n * 0.02 + Vec3::Y * 0.34, 0.0, g * 0.85, SHADE_GILT);
                    cube(out, f + n * 0.04 + Vec3::Y * 1.24, along * 0.04 + n * 0.06 + Vec3::Y * 0.04, 0.0, g * 0.9, SHADE_GILT);
                    cube(out, f + n * 0.05 + Vec3::Y * 1.275, along * 0.28 + n * 0.06 + Vec3::Y * 0.03, 0.0, g, SHADE_GILT);
                    for s in [-1.0f32, 1.0] {
                        cube(
                            out,
                            f + n * 0.05 + along * (s * 0.09) + Vec3::Y * 1.34,
                            Vec3::new(0.045, 0.1, 0.045),
                            0.0,
                            lin(art::WAX),
                            SHADE_LIT,
                        );
                    }
                    candle = Some(f + n * 0.05 + Vec3::Y * 1.39);
                }
                Dress::Drift => {
                    let k = hash(hx * 3.7, hy * 2.9) as f32;
                    let m = Mat4::from_translation(cc + n * (0.44 + 0.04 * k))
                        * Mat4::from_rotation_y(if dx != 0 { PI / 2.0 } else { 0.0 })
                        * Mat4::from_scale(Vec3::new(1.7, 0.7 + 0.3 * k, 0.8));
                    push_merged(out, pile, m, GOLD_GLOW);
                }
                Dress::Banner => {
                    // Greed's banner: red velvet from a gilt rod under the cornice, a gold coin on it.
                    let f = cc + n * 0.5;
                    cube(out, f + n * 0.02 + Vec3::Y * 2.06, along * 0.7 + n * 0.04 + Vec3::Y * 0.04, 0.0, g, SHADE_GILT);
                    cube(out, f + n * 0.03 + Vec3::Y * 1.38, along * 0.56 + n * 0.02 + Vec3::Y * 1.32, 0.0, lin(VELVET), SHADE_CLOTH);
                    for s in [-1.0f32, 1.0] {
                        cube(
                            out,
                            f + n * 0.03 + along * (s * 0.17) + Vec3::Y * 0.66,
                            along * 0.22 + n * 0.02 + Vec3::Y * 0.16,
                            0.0,
                            lin(VELVET_D),
                            SHADE_CLOTH,
                        );
                    }
                    for k in 0..3 {
                        let rot = Quat::from_rotation_arc(Vec3::Z, n) * Quat::from_rotation_z(k as f32 * PI / 6.0);
                        out.push(Instance::new(
                            Mat4::from_scale_rotation_translation(Vec3::new(0.3, 0.3, 0.02), rot, f + n * 0.03 + Vec3::Y * 1.45),
                            g,
                            1.0,
                            Vec3::ZERO,
                            SHADE_GILT,
                        ));
                    }
                    cube(out, f + n * 0.04 + Vec3::Y * 1.45, along * 0.1 + n * 0.01 + Vec3::Y * 0.1, PI / 4.0, lin(VELVET_D), SHADE_CLOTH);
                }
                Dress::Door => {
                    // A strongroom door: pale jambs and lintel, riveted iron, a gilt lock.
                    let f = cc + n * 0.5;
                    for s in [-1.0f32, 1.0] {
                        cube(
                            out,
                            f - n * 0.03 + along * (s * 0.39) + Vec3::Y * 0.78,
                            along * 0.1 + n * 0.14 + Vec3::Y * 1.16,
                            0.0,
                            pale * 1.2,
                            SHADE_STONE,
                        );
                    }
                    cube(out, f - n * 0.03 + Vec3::Y * 1.45, along * 0.9 + n * 0.14 + Vec3::Y * 0.22, 0.0, pale * 1.25, SHADE_STONE);
                    let door = f - n * 0.1;
                    cube(
                        out,
                        door + Vec3::Y * 0.78,
                        along * 0.68 + n * 0.06 + Vec3::Y * 1.14,
                        0.0,
                        jit(art::IRON, hash(hx, hy * 2.0), 0.08),
                        SHADE_IRON,
                    );
                    for hh in [0.3f32, 0.75, 1.2] {
                        cube(
                            out,
                            door + n * 0.04 + Vec3::Y * hh,
                            along * 0.68 + n * 0.025 + Vec3::Y * 0.08,
                            0.0,
                            lin(art::IRON_L),
                            SHADE_IRON,
                        );
                        for s in [-1.0f32, -0.33, 0.33, 1.0] {
                            cube(
                                out,
                                door + n * 0.056 + along * (s * 0.3) + Vec3::Y * hh,
                                Vec3::splat(0.028),
                                0.0,
                                lin(art::IRON_L) * 1.2,
                                SHADE_IRON,
                            );
                        }
                    }
                    cube(out, door + n * 0.05 + along * 0.17 + Vec3::Y * 0.9, along * 0.16 + n * 0.03 + Vec3::Y * 0.2, 0.0, g, SHADE_GILT);
                    cube(
                        out,
                        door + n * 0.068 + along * 0.17 + Vec3::Y * 0.91,
                        along * 0.03 + n * 0.01 + Vec3::Y * 0.07,
                        0.0,
                        lin(art::GROUT),
                        SHADE_LIT,
                    );
                }
                Dress::Ledgers => {
                    // A tall case of ledgers sunk in the wall: oak shelves, the books' spines in
                    // the board's colours, the odd one lying flat, gilt titles on their spines.
                    let back = cc + n * 0.18;
                    let wood = jit(art::OAK, hash(hx, hy), 0.08);
                    cube(out, back + Vec3::Y * 0.98, along * 0.84 + n * 0.04 + Vec3::Y * 1.95, 0.0, wood * 0.6, SHADE_WOOD);
                    for s in [-1.0f32, 1.0] {
                        cube(
                            out,
                            cc + n * 0.35 + along * (s * 0.44) + Vec3::Y * 0.98,
                            along * 0.06 + n * 0.38 + Vec3::Y * 1.95,
                            0.0,
                            wood,
                            SHADE_WOOD,
                        );
                    }
                    for (k, hh) in [0.05f32, 0.5, 0.95, 1.4, 1.85].into_iter().enumerate() {
                        cube(out, cc + n * 0.35 + Vec3::Y * hh, along * 0.84 + n * 0.38 + Vec3::Y * 0.05, 0.0, wood * 1.1, SHADE_WOOD);
                        if k == 4 {
                            break;
                        }
                        let mut uu = -0.38;
                        let mut j = 0;
                        while uu < 0.36 {
                            let r = hash(hx * 3.1 + j as f64 * 1.7 + k as f64 * 5.0, hy * 2.3 + j as f64) as f32;
                            let w = 0.06 + r * 0.05;
                            let tall = 0.28 + r * 0.1;
                            let col = lin(LEDGER[(j + k) % LEDGER.len()]) * (0.8 + 0.4 * r);
                            if r > 0.9 {
                                // One lying flat.
                                cube(
                                    out,
                                    cc + n * 0.34 + along * (uu + 0.1) + Vec3::Y * (hh + 0.06),
                                    along * 0.2 + n * 0.26 + Vec3::Y * 0.05,
                                    0.0,
                                    col,
                                    SHADE_CLOTH,
                                );
                                uu += 0.22;
                            } else {
                                cube(
                                    out,
                                    cc + n * 0.34 + along * (uu + w / 2.0) + Vec3::Y * (hh + 0.025 + tall / 2.0),
                                    along * w + n * 0.28 + Vec3::Y * tall,
                                    0.0,
                                    col,
                                    SHADE_CLOTH,
                                );
                                cube(
                                    out,
                                    cc + n * 0.485 + along * (uu + w / 2.0) + Vec3::Y * (hh + 0.025 + tall * 0.75),
                                    along * w * 0.9 + n * 0.01 + Vec3::Y * 0.02,
                                    0.0,
                                    g,
                                    SHADE_GILT,
                                );
                                uu += w + 0.012;
                            }
                            j += 1;
                        }
                    }
                }
            }
        }
        candle
    }

    /// A pillar of black marble banded in gilt, on a stepped base under a gilt capital.
    fn build_court_pillar(&mut self, base: Vec3, h: f32) {
        let out = &mut self.static_cubes;
        cube(out, base + Vec3::new(0.0, 0.08, 0.0), Vec3::new(0.95, 0.16, 0.95), 0.0, lin(art::PALE[1]) * 0.6, SHADE_STONE);
        cube(out, base + Vec3::new(0.0, 0.22, 0.0), Vec3::new(0.8, 0.14, 0.8), PI / 4.0, gold(h as f64) * 0.8, SHADE_GILT);
        for k in 0..12 {
            let y = 0.3 + k as f32 * 0.21;
            let band = k % 4 == 3;
            let col =
                if band { gold(h as f64 + k as f64 * 0.1) } else { lin(BLACK) * (0.95 + 0.1 * ((k + (h * 7.0) as usize) % 2) as f32) };
            let shade = if band { SHADE_GILT } else { SHADE_MARBLE };
            cube(out, base + Vec3::new(0.0, y + 0.105, 0.0), Vec3::new(0.56, 0.21, 0.56), k as f32 * PI / 12.0, col, shade);
        }
        cube(out, base + Vec3::new(0.0, PILLAR_H - 0.22, 0.0), Vec3::new(0.78, 0.14, 0.78), PI / 4.0, gold(h as f64 * 3.0), SHADE_GILT);
        cube(out, base + Vec3::new(0.0, PILLAR_H - 0.08, 0.0), Vec3::new(0.95, 0.16, 0.95), 0.0, lin(art::PALE[0]) * 0.65, SHADE_STONE);
    }

    /// A counting table: dark wood, ledgers stacked and one lying open, coin stacks, and at the
    /// run's outer end a candle in a gilt stick. Returns where that candle burns.
    fn build_counting_table(&mut self, base: Vec3, x: i32, h: f32) -> Option<Vec3> {
        let out = &mut self.static_cubes;
        cube(out, base + Vec3::new(0.0, 0.72, 0.0), Vec3::new(1.02, 0.08, 0.86), 0.0, lin(WOOD_L), SHADE_WOOD);
        cube(out, base + Vec3::new(0.0, 0.66, 0.0), Vec3::new(0.96, 0.06, 0.8), 0.0, lin(WOOD), SHADE_WOOD);
        let left_end = TABLES.iter().any(|t| t.0 == x);
        for (sx, sz) in [(-0.42f32, -0.36f32), (0.42, -0.36), (-0.42, 0.36), (0.42, 0.36)] {
            let end = if left_end { sx < 0.0 } else { sx > 0.0 };
            if end {
                cube(out, base + Vec3::new(sx, 0.33, sz), Vec3::new(0.09, 0.66, 0.09), 0.0, lin(WOOD), SHADE_WOOD);
            }
        }
        let top = 0.76;
        // Ledgers in a stack, their covers in the board's colours, page edges pale.
        for k in 0..(3 + (h * 3.0) as usize) {
            let p = base + Vec3::new(-0.22, top + 0.04 + k as f32 * 0.075, -0.12);
            let col = lin(LEDGER[(k + x as usize) % LEDGER.len()]);
            cube(out, p, Vec3::new(0.36, 0.07, 0.26), (k as f32 * 0.37).sin() * 0.15, col, SHADE_CLOTH);
            cube(out, p + Vec3::new(0.012, 0.0, 0.0), Vec3::new(0.34, 0.05, 0.27), (k as f32 * 0.37).sin() * 0.15, lin(PAGE), SHADE_LIT);
        }
        // An open ledger, its columns of figures inked dark.
        let open = base + Vec3::new(0.18, top + 0.02, 0.14);
        cube(out, open, Vec3::new(0.5, 0.03, 0.34), 0.2, lin(LEDGER[1]), SHADE_CLOTH);
        cube(out, open + Vec3::new(0.0, 0.02, 0.0), Vec3::new(0.46, 0.02, 0.3), 0.2, lin(PAGE), SHADE_LIT);
        for k in 0..5 {
            let q = open + Vec3::new(-0.15 + k as f32 * 0.07, 0.032, 0.0);
            cube(out, q, Vec3::new(0.012, 0.004, 0.22), 0.2, lin(0x2a2420), SHADE_LIT);
        }
        // Stacks of coin, counted out.
        for k in 0..3 {
            let p = base + Vec3::new(0.3 - k as f32 * 0.12, top, -0.24);
            coin_stack(out, Mat4::from_translation(p) * Mat4::from_rotation_y(k as f32 * 1.3), h * 17.0 + k as f32);
        }
        // A candle in a gilt stick at the run's outer end.
        let outer = TABLES.iter().any(|t| (t.0 == x && t.0 < CENTRE.0) || (t.1 == x && t.1 > CENTRE.0));
        if !outer {
            return None;
        }
        let side = if x < CENTRE.0 { -0.36 } else { 0.36 };
        let at = base + Vec3::new(side, top, 0.24);
        let g = gold(h as f64);
        cube(out, at + Vec3::Y * 0.015, Vec3::new(0.1, 0.03, 0.1), PI / 4.0, g, SHADE_GILT);
        cube(out, at + Vec3::Y * 0.07, Vec3::new(0.03, 0.09, 0.03), 0.0, g, SHADE_GILT);
        cube(out, at + Vec3::Y * 0.17, Vec3::new(0.045, 0.12, 0.045), 0.0, lin(art::WAX), SHADE_LIT);
        Some(at + Vec3::Y * 0.25)
    }

    /// The dais of three steps against the north wall and the throne on it: gold, its back high
    /// and spiked, cushioned in red velvet. Returns the throne's seat.
    fn build_throne(&mut self) -> Vec3 {
        let (x0, y0, x1, y1) = DAIS;
        let out = &mut self.static_cubes;
        let mid = Vec3::new((x0 + x1) as f32 / 2.0, 0.0, (y0 + y1) as f32 / 2.0);
        let (w, d) = ((x1 - x0 + 1) as f32, (y1 - y0 + 1) as f32);
        for k in 0..3 {
            let k = k as f32;
            let p = mid + Vec3::new(0.0, 0.12 + k * 0.24, -k * 0.25);
            cube(out, p, Vec3::new(w - k * 1.2, 0.24, d - k * 0.5), 0.0, lin(MARBLE_D), SHADE_MARBLE);
            cube(
                out,
                p + Vec3::new(0.0, 0.125, (d - k * 0.5) / 2.0 - 0.02),
                Vec3::new(w - k * 1.2, 0.025, 0.05),
                0.0,
                gold(k as f64 * 0.3) * 0.85,
                SHADE_GILT,
            );
        }
        let seat = mid + Vec3::new(0.0, 0.72, -0.6);
        let (g, gd, gl) = (lin(art::GOLD), lin(art::GOLD_DARK), lin(art::GOLD_PALE));
        // The seat and its velvet cushion.
        cube(out, seat + Vec3::new(0.0, 0.3, 0.0), Vec3::new(2.4, 0.6, 1.6), 0.0, g, SHADE_GILT);
        cube(out, seat + Vec3::new(0.0, 0.66, 0.1), Vec3::new(2.1, 0.14, 1.3), 0.0, lin(VELVET), SHADE_CLOTH);
        // Arms ending in gold lion's paws (blocks), the back rising high with spikes over it.
        for s in [-1.0f32, 1.0] {
            cube(out, seat + Vec3::new(s * 1.2, 0.9, 0.0), Vec3::new(0.3, 0.5, 1.5), 0.0, gd, SHADE_GILT);
            cube(out, seat + Vec3::new(s * 1.2, 1.2, 0.7), Vec3::new(0.38, 0.22, 0.3), 0.0, gl, SHADE_GILT);
        }
        cube(out, seat + Vec3::new(0.0, 1.9, -0.7), Vec3::new(2.4, 2.6, 0.3), 0.0, gd, SHADE_GILT);
        cube(out, seat + Vec3::new(0.0, 1.8, -0.53), Vec3::new(1.9, 2.1, 0.06), 0.0, lin(VELVET_D), SHADE_CLOTH);
        for k in 0..7 {
            let x = -1.05 + k as f32 * 0.35;
            let tall = 0.5 + (1.0 - (x / 1.2).abs()) * 0.7;
            cube(out, seat + Vec3::new(x, 3.2 + tall / 2.0, -0.7), Vec3::new(0.14, tall, 0.14), PI / 4.0, g, SHADE_GILT);
        }
        // A great coin over the back: his seal.
        for (r, col, shade) in [(0.55f32, gl, SHADE_GILT), (0.4, lin(COPPER), SHADE_BRONZE), (0.18, g, SHADE_GILT)] {
            cube(out, seat + Vec3::new(0.0, 2.6, -0.5), Vec3::new(r * 2.0, r * 2.0, 0.06), 0.0, col, shade);
        }
        seat
    }

    /// The giant brass scales behind the throne: a column, a beam across, and two pans hanging on
    /// chains, each heaped with coins (the beam tipped, the greedier pan lower). Returns the beam's middle.
    fn build_scales(&mut self, pile: &vault_decor::Merged) -> Vec3 {
        let base = Vec3::new(SCALES.0 as f32, 0.0, SCALES.1 as f32);
        let out = &mut self.static_cubes;
        let (brass, brass_d) = (lin(BRASS), lin(BRASS_D));
        cube(out, base + Vec3::new(0.0, 0.2, 0.0), Vec3::new(1.6, 0.4, 1.2), 0.0, brass_d, SHADE_GILT);
        cube(out, base + Vec3::new(0.0, 0.5, 0.0), Vec3::new(1.1, 0.2, 0.9), PI / 4.0, brass, SHADE_GILT);
        for k in 0..10 {
            let y = 0.6 + k as f32 * 0.5;
            cube(
                out,
                base + Vec3::new(0.0, y + 0.25, 0.0),
                Vec3::new(0.36, 0.5, 0.36),
                k as f32 * 0.3,
                if k % 3 == 2 { brass_d } else { brass },
                SHADE_GILT,
            );
        }
        let top = base + Vec3::new(0.0, 5.7, 0.0);
        cube(out, top, Vec3::new(0.6, 0.6, 0.6), PI / 4.0, lin(art::GOLD), SHADE_GILT);
        let tilt = 0.12f32;
        let half = 3.6;
        let beam = Mat4::from_translation(top) * Mat4::from_rotation_z(tilt);
        out.push(Instance::new(beam * Mat4::from_scale(Vec3::new(half * 2.0, 0.22, 0.26)), brass, 1.0, Vec3::ZERO, SHADE_GILT));
        for s in [-1.0f32, 1.0] {
            let end = beam.transform_point3(Vec3::new(s * half, 0.0, 0.0));
            cube(out, end, Vec3::new(0.36, 0.36, 0.36), PI / 4.0, lin(art::GOLD_DARK), SHADE_GILT);
            let pan = end - Vec3::new(0.0, 2.9, 0.0);
            // Three chains to the pan.
            for k in 0..3 {
                let a = k as f32 * TAU / 3.0;
                let foot = pan + Vec3::new(a.cos() * 0.8, 0.12, a.sin() * 0.5);
                let mid = (end + foot) * 0.5;
                let dir = (foot - end).normalize();
                let len = (foot - end).length();
                let rot = Quat::from_rotation_arc(Vec3::Y, dir);
                out.push(Instance::new(
                    Mat4::from_scale_rotation_translation(Vec3::new(0.04, len, 0.04), rot, mid),
                    brass_d,
                    1.0,
                    Vec3::ZERO,
                    SHADE_GILT,
                ));
            }
            cube(out, pan, Vec3::new(1.9, 0.12, 1.2), 0.0, brass, SHADE_GILT);
            cube(out, pan - Vec3::new(0.0, 0.1, 0.0), Vec3::new(1.5, 0.1, 0.9), 0.0, brass_d, SHADE_GILT);
            push_merged(
                out,
                pile,
                Mat4::from_translation(pan + Vec3::new(0.0, 0.05, 0.0))
                    * Mat4::from_scale(Vec3::new(1.4, if s > 0.0 { 1.3 } else { 0.8 }, 0.9)),
                GOLD_GLOW,
            );
        }
        top
    }

    /// A copper brazier hanging on chains from the dark above (its fire is drawn each frame).
    fn build_brazier(&mut self, at: Vec3) {
        let out = &mut self.static_cubes;
        cube(out, at - Vec3::new(0.0, 0.1, 0.0), Vec3::new(0.5, 0.18, 0.5), PI / 4.0, lin(COPPER), SHADE_BRONZE);
        cube(out, at - Vec3::new(0.0, 0.22, 0.0), Vec3::new(0.32, 0.1, 0.32), PI / 4.0, lin(COPPER_D), SHADE_BRONZE);
        cube(out, at, Vec3::new(0.56, 0.05, 0.56), 0.0, lin(COPPER_D), SHADE_BRONZE);
        super::crypt::glowing_block(out, at + Vec3::new(0.0, 0.03, 0.0), Vec3::new(0.42, 0.04, 0.42), lin(0x2a1408), lin(0xff7a28) * 0.4);
        for k in 0..3 {
            let a = k as f32 * TAU / 3.0;
            let foot = at + Vec3::new(a.cos() * 0.26, 0.02, a.sin() * 0.26);
            let hook = at + Vec3::new(0.0, 1.3, 0.0);
            let mid = (foot + hook) * 0.5;
            let rot = Quat::from_rotation_arc(Vec3::Y, (hook - foot).normalize());
            out.push(Instance::new(
                Mat4::from_scale_rotation_translation(Vec3::new(0.025, (hook - foot).length(), 0.025), rot, mid),
                lin(art::IRON),
                1.0,
                Vec3::ZERO,
                SHADE_IRON,
            ));
        }
        cube(out, at + Vec3::new(0.0, 3.0, 0.0), Vec3::new(0.035, 3.4, 0.035), 0.0, lin(art::IRON), SHADE_IRON);
    }

    /// Each frame: the braziers' and candles' fire and light, the cold light on the scales, the
    /// prince's own light, and the gate.
    pub(super) fn emit_counting_house(&self, s: &SceneState, time: f32, hero: Vec3, f: &mut Frame) {
        let Some(cv) = &self.counting else { return };
        f.bounds = Vec4::new(-0.5, -0.5, s.grid.width as f32 - 0.5, s.grid.height as f32 - 0.5);
        for (i, b) in cv.braziers.iter().enumerate() {
            let fi = i as f32;
            for k in 0..5 {
                let t = (time * 2.2 + fi * 0.37 + k as f32 * 0.2).fract();
                let a = fi * 1.9 + k as f32 * 1.3;
                let p = *b + Vec3::new(a.cos() * 0.12 * (1.0 - t), 0.08 + t * 0.42, a.sin() * 0.12 * (1.0 - t));
                let size = 0.11 * (1.0 - t) + 0.03;
                let c = lin(FLAME).lerp(lin(FLAME_TIP), t);
                f.additive.push(Instance::new(
                    Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(size, size * 1.8, size)),
                    c,
                    0.45 * (1.0 - t),
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
            glow_orb(f, *b + Vec3::new(0.0, 0.2, 0.0), 0.55, lin(AMBER), 0.14 + 0.03 * (time * 7.0 + fi).sin());
        }
        for (i, c) in cv.candles.iter().enumerate() {
            let fi = i as f32 * 1.7;
            let tall = 0.07 + 0.012 * (time * 11.0 + fi * 5.0).sin();
            // (A candelabrum's three candles stand in a row; a table's one.)
            let three = (c.y - 1.32).abs() < 0.01;
            let row: &[f32] = if three { &[-0.25, 0.0, 0.25] } else { &[0.0] };
            for dx in row {
                let p = *c + Vec3::new(*dx, 0.04 + tall * 0.5, 0.0);
                f.additive.push(Instance::new(
                    Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(0.03, tall, 0.03)),
                    lin(FLAME),
                    0.85,
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
                f.additive.push(Instance::new(
                    Mat4::from_translation(p + Vec3::Y * 0.01) * Mat4::from_scale(Vec3::new(0.013, tall * 0.55, 0.013)),
                    lin(0xfff0c0),
                    0.9,
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
            glow_orb(f, *c + Vec3::new(0.0, 0.07, 0.0), 0.3, lin(FLAME), 0.12 + 0.02 * (time * 9.0 + fi).sin());
            let flick = 1.0 + (time * 9.0 + fi).sin() * 0.06;
            f.lights.push(Light { pos: *c + Vec3::new(0.0, 0.25, 0.0), color: lin(CANDLE) * 3.5 * flick, distance: 6.0 });
        }
        // The braziers light the hall, every one of them: the hall's pools of light.
        for (i, p) in cv.braziers.iter().enumerate() {
            let flick = 5.0 + (time * 6.0 + i as f32 * 1.3).sin() * 0.4 + (time * 15.0 + i as f32).sin() * 0.2;
            f.lights.push(Light { pos: *p + Vec3::new(0.0, 0.35, 0.0), color: lin(AMBER) * flick, distance: 9.0 });
        }
        f.lights.push(Light { pos: cv.throne + Vec3::new(0.0, 2.0, 2.2), color: lin(GOLD) * 3.0, distance: 7.0 });
        // The giant statues lit from below, by the fire on the wall's top before each.
        for (i, p) in cv.statues.iter().enumerate() {
            let flick = 1.0 + (time * 5.0 + i as f32 * 2.1).sin() * 0.05;
            f.lights.push(Light { pos: *p + Vec3::new(0.0, 0.4, 1.0), color: lin(AMBER) * 4.0 * flick, distance: 4.5 });
        }
        // The prince stands in his own light: a warm key before him and above, a cold rim behind.
        for e in s.enemies {
            let Some(d) = e.duel.as_deref().filter(|d| d.alive() && d.breed.kind == EnemyKind::Mammon) else { continue };
            let p = Vec3::new(d.pos.x as f32, 0.0, d.pos.y as f32);
            let ahead = Vec3::new(d.facing.x as f32, 0.0, d.facing.y as f32);
            f.lights.push(Light { pos: p + ahead * 2.4 + Vec3::new(0.0, 3.6, 0.0), color: lin(0xffd8a0) * 6.0, distance: 7.5 });
            f.lights.push(Light { pos: p - ahead * 1.8 + Vec3::new(0.0, 3.2, 0.0), color: lin(MIST_TEAL) * 5.0, distance: 5.5 });
            // The hoard slam: where the sceptre lands, coin sprays up out of the floor in a ring,
            // glinting, and a flash of gold (the quake's ring on the floor is the duel's tell).
            if let Some((MoveId::HoardSlam, _, frame)) = d.f.attack() {
                let landed = MoveId::HoardSlam.data().startup;
                let t = frame.saturating_sub(landed) as f32 / 40.0;
                if frame >= landed && t < 1.0 {
                    let hit = p + ahead * 1.3;
                    for k in 0..18 {
                        let a = k as f32 * TAU / 18.0 + (k as f32 * 1.7).sin() * 0.2;
                        let r = (1.4 + (k * 7 % 5) as f32 * 0.25) * t * 1.6;
                        let y = 0.1 + t * (2.6 + (k % 3) as f32 * 0.5) - t * t * 3.4;
                        if y < 0.0 {
                            continue;
                        }
                        let at = hit + Vec3::new(a.cos() * r, y, a.sin() * r);
                        let spin = Quat::from_rotation_x(time * 17.0 + k as f32) * Quat::from_rotation_y(a);
                        let g = gold(k as f64 * 0.07);
                        let m = Mat4::from_scale_rotation_translation(Vec3::new(0.12, 0.025, 0.12), spin, at);
                        f.opaque[MeshKind::Cylinder as usize].push(Instance::new(m, g, 1.0, g * 0.2, SHADE_GILT));
                        if k % 4 == 0 {
                            glint(f, at, 0.14 * (1.0 - t), 1.0 - t, time + a);
                        }
                    }
                    f.lights.push(Light { pos: hit + Vec3::Y * 0.6, color: lin(GOLD_L) * 6.0 * (1.0 - t), distance: 4.0 });
                }
            }
        }
        // A cold shaft from above on the scales: the board's mist teal against the amber.
        f.lights.push(Light { pos: cv.scales + Vec3::new(0.0, 1.5, 2.5), color: lin(MIST_TEAL) * 5.0, distance: 9.0 });
        // The gate he came in by.
        let door = s.gate.map(|g| g.door).unwrap_or(0.0);
        self.emit_portal(&cv.gate, door, false, time, hero, f);
    }

    /// A prince's coins on the floor: flying from his hand (or falling from above), glinting as
    /// they turn, then lying there as a ring of gold grows round each (the tell), and bursting
    /// into gold spikes.
    pub(super) fn emit_traps(&self, traps: &[Trap], grid: &Grid, time: f32, f: &mut Frame) {
        for t in traps {
            let ground = floor_at(grid, t.pos.x as f32, t.pos.y as f32);
            let at = Vec3::new(t.pos.x as f32, ground, t.pos.y as f32);
            let seed = (t.pos.x * 3.7 + t.pos.y * 1.9) as f32;
            if let Some(b) = t.burst {
                if t.lingering() {
                    // Its ground still hurting: a pulsing pool on the floor, fading as it ends.
                    let left = 1.0 - b as f32 / t.linger.max(1) as f32;
                    let pulse = 0.7 + 0.3 * (time * 6.0 + seed).sin();
                    let r = t.radius as f32;
                    let m = Mat4::from_translation(at + Vec3::new(0.0, 0.02, 0.0))
                        * Mat4::from_rotation_x(-PI / 2.0)
                        * Mat4::from_scale(Vec3::new(r, r, 1.0));
                    f.transparent[MeshKind::Disc as usize].push(Instance::new(
                        m,
                        lin(0xff9a3a) * pulse,
                        0.35 * left,
                        Vec3::ZERO,
                        SHADE_UNLIT,
                    ));
                }
                if b >= SPIKES {
                    continue;
                }
                // Spikes of gold thrust up out of the floor, and sink back; a spray of glints.
                let u = b as f32 / SPIKES as f32;
                let rise = if u < 0.15 { u / 0.15 } else { 1.0 - ((u - 0.15) / 0.85).powi(2) };
                for k in 0..9 {
                    let a = k as f32 * 2.39 + seed;
                    let r = if k == 0 { 0.0 } else { t.radius as f32 * (0.3 + 0.6 * ((k * 37 % 10) as f32 / 10.0)) };
                    let tall = (0.55 + 0.4 * ((k * 13 % 7) as f32 / 7.0)) * rise;
                    let p = at + Vec3::new(a.cos() * r, tall * 0.5, a.sin() * r);
                    let m = Mat4::from_translation(p)
                        * Mat4::from_rotation_y(a)
                        * Mat4::from_rotation_z(0.15 * (a * 2.0).sin())
                        * Mat4::from_scale(Vec3::new(0.09, tall.max(0.01), 0.09));
                    let g = gold(k as f64 * 0.13);
                    f.opaque[MeshKind::Cube as usize].push(Instance::new(m, g, 1.0, g * 0.12, SHADE_GILT));
                    if u < 0.4 && k % 3 == 0 {
                        glint(f, p + Vec3::Y * tall * 0.5, 0.14 * (1.0 - u * 2.0), 1.0 - u * 2.5, time + a);
                    }
                }
                f.lights.push(Light { pos: at + Vec3::new(0.0, 0.6, 0.0), color: lin(GOLD_L) * 4.0 * (1.0 - u), distance: 3.0 });
                continue;
            }
            let pos = if t.flight > 0 {
                // In the air: an arc from his hand, or a fall from high above.
                let total = crate::logic::combat::prince::COIN_FLIGHT as f32 + 1.0;
                let k = 1.0 - t.flight as f32 / total;
                if t.fell {
                    at + Vec3::new(0.0, 6.0 * (1.0 - k * k), 0.0)
                } else {
                    let from = Vec3::new(t.from.x as f32, ground, t.from.y as f32);
                    from.lerp(at, k) + Vec3::new(0.0, 2.0 + 2.2 * k * (1.0 - k) * 2.0 - 2.0 * k, 0.0)
                }
            } else {
                at + Vec3::new(0.0, 0.04, 0.0)
            };
            let spin = if t.flight > 0 { Quat::from_rotation_x(time * 20.0 + seed) } else { Quat::from_rotation_y(seed) };
            let m = Mat4::from_scale_rotation_translation(Vec3::new(0.2, 0.04, 0.2), spin, pos);
            let g = lin(art::GOLD);
            f.opaque[MeshKind::Cylinder as usize].push(Instance::new(m, g, 1.0, g * 0.18, SHADE_GILT));
            if t.flight > 0 {
                // A glint off it each time it turns its face to the eye, and a falling one's streak.
                let flash = (time * 20.0 + seed).sin().abs().powi(12);
                if flash > 0.1 {
                    glint(f, pos, 0.16, flash, time + seed);
                }
                if t.fell {
                    let m = Mat4::from_translation(pos + Vec3::Y * 0.35) * Mat4::from_scale(Vec3::new(0.025, 0.6, 0.025));
                    f.additive.push(Instance::new(m, lin(GOLD_L), 0.35, Vec3::ZERO, SHADE_UNLIT));
                }
            }
            if t.flight == 0 {
                // The tell: a ring of gold on the floor, growing in from its edge and flashing as it comes due.
                let fill = t.tell();
                let late = t.fuse <= 8;
                let blink = if late { 0.6 + (time * 40.0).sin().abs() * 0.8 } else { 0.0 };
                let r = t.radius as f32;
                let n = ((TAU * r) / 0.22).ceil() as i32;
                for k in 0..n {
                    let a = k as f32 * TAU / n as f32;
                    let p = at + Vec3::new(a.cos() * r, 0.03, a.sin() * r);
                    let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(0.08, 0.012, 0.08));
                    f.additive.push(Instance::new(m, lin(0xffd860) * (0.9 + blink), 0.7, Vec3::ZERO, SHADE_UNLIT));
                }
                let inner = r * (1.0 - fill);
                let m = Mat4::from_translation(at + Vec3::new(0.0, 0.025, 0.0))
                    * Mat4::from_rotation_x(-PI / 2.0)
                    * Mat4::from_scale(Vec3::new(r.max(inner), r.max(inner), 1.0));
                f.transparent[MeshKind::Disc as usize].push(Instance::new(
                    m,
                    lin(0xb87a28) * (0.6 + blink * 0.6),
                    0.08 + 0.2 * fill,
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
                let m = Mat4::from_translation(at + Vec3::new(0.0, 0.03, 0.0))
                    * Mat4::from_rotation_x(-PI / 2.0)
                    * Mat4::from_scale(Vec3::new(inner + 0.05, inner + 0.05, 1.0));
                f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, lin(GOLD_L) * 1.2, 0.5, Vec3::ZERO, SHADE_UNLIT));
            }
        }
    }
}
