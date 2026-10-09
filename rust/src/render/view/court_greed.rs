//! Greed's court, the Counting House (`logic::court` lays it out), to the art direction: the
//! board's palette (charcoal, umber, copper, amber, mist teal, pale stone) with Greed's gold as its
//! accent, and the vaults' materials (polished gold, marble).
//!
//! An octagonal hall floored in charcoal marble veined with gold, a ring of coin mosaic round its
//! middle and Mammon's seal at its heart; walls of umber panels between black marble pilasters
//! banded in gold under a copper cornice; eight black-and-gold pillars in a ring; heaps of coins
//! in the corners and chests against the walls spilling theirs; counting tables piled with
//! ledgers and coin stacks east and west. At the north end Mammon's throne of gold and red
//! velvet stands on a stepped dais under his giant brass scales, a heap of coins in each pan.
//! Copper braziers hang on chains from the dark above, burning amber; the far end glows faintly
//! teal where the cold light falls on the scales. The gate he came in by is Greed's portal in the
//! south wall. His coins on the floor (`prince::Trap`) are drawn here too, wherever they fall.

use super::vault::{SHADE_GOLD, SHADE_MARBLE};
use super::vault_decor::{self, push_merged};
use super::*;
use crate::logic::combat::prince::{Trap, SPIKES};
use crate::logic::court::{self, Piece, CENTRE, DAIS, MOSAIC, SCALES, TABLES};
use crate::logic::hub::Gate;

pub(super) mod pal {
    pub const MARBLE: u32 = 0x2e2a28;
    pub const MARBLE_L: u32 = 0x3c3634;
    pub const MARBLE_D: u32 = 0x221f1e;
    pub const BLACK: u32 = 0x1a1818;
    pub const UMBER: u32 = 0x4a3426;
    pub const UMBER_D: u32 = 0x36261c;
    pub const COPPER: u32 = 0xb06a44;
    pub const COPPER_D: u32 = 0x7a4630;
    pub const BRASS: u32 = 0xb8903a;
    pub const BRASS_D: u32 = 0x86662a;
    pub const GOLD: u32 = 0xe2b040;
    pub const GOLD_D: u32 = 0xa47828;
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
    pub const MIST_TEAL: u32 = 0x6e9a90;
}
use pal::*;

/// The hall's walls and pillars stand this tall.
const WALL_H: f32 = 2.4;
const PILLAR_H: f32 = 3.0;
/// How high the braziers hang.
const BRAZIER_Y: f32 = 2.5;

/// Dark and warm: the braziers' amber is the light, a cold teal fill from above, the gold the accent.
pub fn counting_atmosphere() -> Atmosphere {
    Atmosphere {
        background: 0x0c0a09,
        fog_near: 18.0,
        fog_far: 46.0,
        hemi_sky: lin(MIST_TEAL) * 0.42,
        hemi_ground: lin(0x5a4028) * 0.42,
        ambient: lin(0x5e4c38) * 0.46,
        sun_dir: Vec3::new(-0.35, 1.0, 0.5).normalize(),
        sun: lin(0xf2dcb8) * 1.05,
        exposure: 1.2,
        night_sky: 0.0,
        bloom: 0.75,
        moon_view: Vec3::new(0.0, 0.3, -1.0).normalize(),
        cloud_tint: lin(0x1a1008),
    }
}

/// What the view keeps of the Counting House: its braziers, its gate, the scales and the throne.
pub struct CountingView {
    pub braziers: Vec<Vec3>,
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

impl WorldView {
    /// The Counting House's static scene.
    pub(super) fn build_counting_house(&mut self, grid: &Grid) {
        let rock = |x: i32, y: i32| grid.get(x, y) == Tile::Rock;
        let c = Vec3::new(CENTRE.0 as f32, 0.0, CENTRE.1 as f32);
        let floor_tile = |x: i32, y: i32| court::in_hall(x, y);

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
                        cube(out, o + dir * (k * 0.26), Vec3::new(0.28, 0.008, 0.012), a + bend, lin(GOLD_D) * 0.7, SHADE_GOLD);
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
                let col = if (k + band) % 3 == 0 { lin(COPPER) } else { lin(GOLD) * (0.9 + 0.1 * (k % 2) as f32) };
                cube(&mut self.static_cubes, p, Vec3::new(0.24, 0.016, 0.24), a + PI / 4.0, col, SHADE_GOLD);
            }
        }
        for r in [MOSAIC as f32 - 0.62, MOSAIC as f32 + 0.62] {
            for k in 0..ring_n {
                let a = k as f32 * TAU / ring_n as f32;
                let p = c + Vec3::new(a.cos() * r, 0.003, a.sin() * r);
                cube(&mut self.static_cubes, p, Vec3::new(0.33, 0.012, 0.07), -a + PI / 2.0, lin(STONE), SHADE_MARBLE);
            }
        }
        // The seal at the heart: a great coin, rings of gold and copper, an eight-rayed star in it.
        for (r, col) in [(1.9f32, GOLD_D), (1.6, COPPER), (1.25, GOLD), (0.9, BLACK), (0.55, GOLD)] {
            let n = (TAU * r / 0.22).round().max(8.0) as i32;
            for k in 0..n {
                let a = k as f32 * TAU / n as f32;
                cube(
                    &mut self.static_cubes,
                    c + Vec3::new(a.cos() * r, 0.005, a.sin() * r),
                    Vec3::new(0.24, 0.014, 0.24),
                    a,
                    lin(col),
                    SHADE_GOLD,
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
                lin(GOLD_L),
                SHADE_GOLD,
            );
        }

        // ---- the walls: umber panels between black pilasters banded in gold, a copper cornice
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
            let (xf, yf) = (x as f32, y as f32);
            let h = hash(x as f64 * 3.0, y as f64 * 5.0);
            let out = &mut self.static_cubes;
            cube(out, Vec3::new(xf, 0.12, yf), Vec3::new(1.0, 0.24, 1.0), 0.0, lin(BLACK), SHADE_MARBLE);
            cube(
                out,
                Vec3::new(xf, WALL_H / 2.0 + 0.1, yf),
                Vec3::new(1.0, WALL_H - 0.2, 1.0),
                0.0,
                lin(UMBER) * (0.92 + h as f32 * 0.1),
                SHADE_LIT,
            );
            cube(out, Vec3::new(xf, WALL_H - 0.04, yf), Vec3::new(1.08, 0.12, 1.08), 0.0, lin(COPPER_D), SHADE_GOLD);
            cube(out, Vec3::new(xf, WALL_H + 0.06, yf), Vec3::new(1.12, 0.08, 1.12), 0.0, lin(MARBLE_D), SHADE_MARBLE);
            // On each face toward the hall: a sunk panel, its gold moulding, a pilaster every other block.
            for &d in &open {
                let (dx, dy) = d.vec();
                let n = Vec3::new(dx as f32, 0.0, dy as f32);
                let along = if dx == 0 { Vec3::X } else { Vec3::Z };
                let face = Vec3::new(xf, 0.0, yf) + n * 0.5;
                let size = |a: f32, up: f32, out: f32| along * a + Vec3::Y * up + Vec3::new(n.x.abs(), 0.0, n.z.abs()) * out;
                cube(out, face + Vec3::new(0.0, 1.15, 0.0) + n * 0.005, size(0.7, 1.4, 0.01), 0.0, lin(UMBER_D), SHADE_LIT);
                for (yy, hh) in [(0.44f32, 0.03f32), (1.86, 0.03)] {
                    cube(out, face + Vec3::new(0.0, yy, 0.0) + n * 0.012, size(0.74, hh, 0.02), 0.0, lin(GOLD_D), SHADE_GOLD);
                }
                for s in [-1.0f32, 1.0] {
                    cube(
                        out,
                        face + along * (s * 0.37) + Vec3::new(0.0, 1.15, 0.0) + n * 0.012,
                        size(0.03, 1.45, 0.02),
                        0.0,
                        lin(GOLD_D),
                        SHADE_GOLD,
                    );
                }
                if (x + y) % 2 == 0 {
                    cube(
                        out,
                        face + along * 0.5 + Vec3::new(0.0, WALL_H / 2.0, 0.0) + n * 0.06,
                        size(0.18, WALL_H, 0.12),
                        0.0,
                        lin(BLACK),
                        SHADE_MARBLE,
                    );
                    for yy in [0.3f32, 1.2, 2.1] {
                        cube(
                            out,
                            face + along * 0.5 + Vec3::new(0.0, yy, 0.0) + n * 0.07,
                            size(0.2, 0.05, 0.14),
                            0.0,
                            lin(GOLD),
                            SHADE_GOLD,
                        );
                    }
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

        // ---- what stands on the floor
        let pile = super::vault::pile();
        let coin = super::vault::coin();
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
                    // A great heap of coins, a chest half buried in it, coins strewn round its foot.
                    let out = &mut self.static_cubes;
                    push_merged(
                        out,
                        &pile,
                        Mat4::from_translation(base) * Mat4::from_rotation_y(h * TAU) * Mat4::from_scale(Vec3::new(2.4, 2.0, 2.4)),
                        0.05,
                    );
                    push_merged(
                        out,
                        &pile,
                        Mat4::from_translation(base + Vec3::new(0.45, 0.0, -0.3))
                            * Mat4::from_rotation_y(h * 3.0)
                            * Mat4::from_scale(Vec3::splat(1.6)),
                        0.05,
                    );
                    push_merged(
                        out,
                        &treasure,
                        Mat4::from_translation(base + Vec3::new(-0.35, 0.2, 0.25)) * Mat4::from_rotation_y(h * 5.0),
                        0.04,
                    );
                    for k in 0..7 {
                        let a = h * 9.0 + k as f32 * 0.9;
                        let r = 0.75 + (k % 3) as f32 * 0.18;
                        push_merged(
                            out,
                            &coin,
                            Mat4::from_translation(base + Vec3::new(a.cos() * r, 0.0, a.sin() * r)) * Mat4::from_rotation_y(a),
                            0.04,
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
                    push_merged(out, &pile, Mat4::from_translation(base + front * 0.55) * Mat4::from_scale(Vec3::new(1.2, 0.5, 1.2)), 0.05);
                    for k in 0..4 {
                        let side = Vec3::new(front.z, 0.0, -front.x);
                        let p = base + front * (0.75 + k as f32 * 0.12) + side * ((k as f32 - 1.5) * 0.25);
                        push_merged(out, &coin, Mat4::from_translation(p) * Mat4::from_rotation_y(k as f32), 0.04);
                    }
                }
                Piece::Table => self.build_counting_table(base, x, h, &coin),
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

        // ---- the dais, the throne, and the great scales behind it
        let throne = self.build_throne();
        let scales = self.build_scales();

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

        // ---- the gate: Greed's portal in the south wall, looking into the hall
        let gate = Gate { tile: P::new(court::GATE.0, court::GATE.1), facing: Dir::Up, sin: crate::logic::sins::Sin::Greed };
        self.build_court_portal(&gate);
        let _ = rock;
        self.counting = Some(CountingView { braziers, gate, scales, throne });
    }

    /// A pillar of black marble banded in gold, on a stepped base under a gold capital.
    fn build_court_pillar(&mut self, base: Vec3, h: f32) {
        let out = &mut self.static_cubes;
        cube(out, base + Vec3::new(0.0, 0.08, 0.0), Vec3::new(0.95, 0.16, 0.95), 0.0, lin(MARBLE_D), SHADE_MARBLE);
        cube(out, base + Vec3::new(0.0, 0.22, 0.0), Vec3::new(0.8, 0.14, 0.8), PI / 4.0, lin(GOLD_D), SHADE_GOLD);
        for k in 0..12 {
            let y = 0.3 + k as f32 * 0.21;
            let col = if k % 4 == 3 { lin(GOLD) } else { lin(BLACK) * (0.95 + 0.1 * ((k + (h * 7.0) as usize) % 2) as f32) };
            let shade = if k % 4 == 3 { SHADE_GOLD } else { SHADE_MARBLE };
            cube(out, base + Vec3::new(0.0, y + 0.105, 0.0), Vec3::new(0.56, 0.21, 0.56), k as f32 * PI / 12.0, col, shade);
        }
        cube(out, base + Vec3::new(0.0, PILLAR_H - 0.22, 0.0), Vec3::new(0.78, 0.14, 0.78), PI / 4.0, lin(GOLD), SHADE_GOLD);
        cube(out, base + Vec3::new(0.0, PILLAR_H - 0.08, 0.0), Vec3::new(0.95, 0.16, 0.95), 0.0, lin(COPPER_D), SHADE_GOLD);
    }

    /// A counting table: dark wood, ledgers stacked and one lying open, coin stacks and a scale-pan.
    fn build_counting_table(&mut self, base: Vec3, x: i32, h: f32, coin: &vault_decor::Merged) {
        let out = &mut self.static_cubes;
        cube(out, base + Vec3::new(0.0, 0.72, 0.0), Vec3::new(1.02, 0.08, 0.86), 0.0, lin(WOOD_L), SHADE_LIT);
        cube(out, base + Vec3::new(0.0, 0.66, 0.0), Vec3::new(0.96, 0.06, 0.8), 0.0, lin(WOOD), SHADE_LIT);
        let left_end = TABLES.iter().any(|t| t.0 == x);
        for (sx, sz) in [(-0.42f32, -0.36f32), (0.42, -0.36), (-0.42, 0.36), (0.42, 0.36)] {
            let end = if left_end { sx < 0.0 } else { sx > 0.0 };
            if end {
                cube(out, base + Vec3::new(sx, 0.33, sz), Vec3::new(0.09, 0.66, 0.09), 0.0, lin(WOOD), SHADE_LIT);
            }
        }
        let top = 0.76;
        // Ledgers in a stack, their covers in the board's colours, page edges pale.
        for k in 0..(3 + (h * 3.0) as usize) {
            let p = base + Vec3::new(-0.22, top + 0.04 + k as f32 * 0.075, -0.12);
            let col = lin(LEDGER[(k + x as usize) % LEDGER.len()]);
            cube(out, p, Vec3::new(0.36, 0.07, 0.26), (k as f32 * 0.37).sin() * 0.15, col, SHADE_LIT);
            cube(out, p + Vec3::new(0.012, 0.0, 0.0), Vec3::new(0.34, 0.05, 0.27), (k as f32 * 0.37).sin() * 0.15, lin(PAGE), SHADE_LIT);
        }
        // An open ledger, its columns of figures inked dark.
        let open = base + Vec3::new(0.18, top + 0.02, 0.14);
        cube(out, open, Vec3::new(0.5, 0.03, 0.34), 0.2, lin(LEDGER[1]), SHADE_LIT);
        cube(out, open + Vec3::new(0.0, 0.02, 0.0), Vec3::new(0.46, 0.02, 0.3), 0.2, lin(PAGE), SHADE_LIT);
        for k in 0..5 {
            let q = open + Vec3::new(-0.15 + k as f32 * 0.07, 0.032, 0.0);
            cube(out, q, Vec3::new(0.012, 0.004, 0.22), 0.2, lin(0x2a2420), SHADE_LIT);
        }
        // Stacks of coin, counted out.
        for k in 0..3 {
            let p = base + Vec3::new(0.3 - k as f32 * 0.12, top, -0.24);
            push_merged(out, coin, Mat4::from_translation(p) * Mat4::from_rotation_y(k as f32 * 1.3), 0.04);
        }
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
                lin(GOLD_D),
                SHADE_GOLD,
            );
        }
        let seat = mid + Vec3::new(0.0, 0.72, -0.6);
        // The seat and its velvet cushion.
        cube(out, seat + Vec3::new(0.0, 0.3, 0.0), Vec3::new(2.4, 0.6, 1.6), 0.0, lin(GOLD), SHADE_GOLD);
        cube(out, seat + Vec3::new(0.0, 0.66, 0.1), Vec3::new(2.1, 0.14, 1.3), 0.0, lin(VELVET), SHADE_LIT);
        // Arms ending in gold lion's paws (blocks), the back rising high with spikes over it.
        for s in [-1.0f32, 1.0] {
            cube(out, seat + Vec3::new(s * 1.2, 0.9, 0.0), Vec3::new(0.3, 0.5, 1.5), 0.0, lin(GOLD_D), SHADE_GOLD);
            cube(out, seat + Vec3::new(s * 1.2, 1.2, 0.7), Vec3::new(0.38, 0.22, 0.3), 0.0, lin(GOLD_L), SHADE_GOLD);
        }
        cube(out, seat + Vec3::new(0.0, 1.9, -0.7), Vec3::new(2.4, 2.6, 0.3), 0.0, lin(GOLD_D), SHADE_GOLD);
        cube(out, seat + Vec3::new(0.0, 1.8, -0.53), Vec3::new(1.9, 2.1, 0.06), 0.0, lin(VELVET_D), SHADE_LIT);
        for k in 0..7 {
            let x = -1.05 + k as f32 * 0.35;
            let tall = 0.5 + (1.0 - (x / 1.2).abs()) * 0.7;
            cube(out, seat + Vec3::new(x, 3.2 + tall / 2.0, -0.7), Vec3::new(0.14, tall, 0.14), PI / 4.0, lin(GOLD), SHADE_GOLD);
        }
        // A great coin over the back: his seal.
        for (r, col) in [(0.55f32, GOLD_L), (0.4, COPPER), (0.18, GOLD)] {
            cube(out, seat + Vec3::new(0.0, 2.6, -0.5), Vec3::new(r * 2.0, r * 2.0, 0.06), 0.0, lin(col), SHADE_GOLD);
        }
        seat
    }

    /// The giant brass scales behind the throne: a column, a beam across, and two pans hanging on
    /// chains, each heaped with coins (the beam tipped, the greedier pan lower). Returns the beam's middle.
    fn build_scales(&mut self) -> Vec3 {
        let base = Vec3::new(SCALES.0 as f32, 0.0, SCALES.1 as f32);
        let pile = super::vault::pile();
        let out = &mut self.static_cubes;
        cube(out, base + Vec3::new(0.0, 0.2, 0.0), Vec3::new(1.6, 0.4, 1.2), 0.0, lin(BRASS_D), SHADE_GOLD);
        cube(out, base + Vec3::new(0.0, 0.5, 0.0), Vec3::new(1.1, 0.2, 0.9), PI / 4.0, lin(BRASS), SHADE_GOLD);
        for k in 0..10 {
            let y = 0.6 + k as f32 * 0.5;
            cube(
                out,
                base + Vec3::new(0.0, y + 0.25, 0.0),
                Vec3::new(0.36, 0.5, 0.36),
                k as f32 * 0.3,
                lin(if k % 3 == 2 { BRASS_D } else { BRASS }),
                SHADE_GOLD,
            );
        }
        let top = base + Vec3::new(0.0, 5.7, 0.0);
        cube(out, top, Vec3::new(0.6, 0.6, 0.6), PI / 4.0, lin(GOLD), SHADE_GOLD);
        let tilt = 0.12f32;
        let half = 3.6;
        let beam = Mat4::from_translation(top) * Mat4::from_rotation_z(tilt);
        out.push(Instance::new(beam * Mat4::from_scale(Vec3::new(half * 2.0, 0.22, 0.26)), lin(BRASS), 1.0, Vec3::ZERO, SHADE_GOLD));
        for s in [-1.0f32, 1.0] {
            let end = beam.transform_point3(Vec3::new(s * half, 0.0, 0.0));
            cube(out, end, Vec3::new(0.36, 0.36, 0.36), PI / 4.0, lin(GOLD_D), SHADE_GOLD);
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
                    lin(BRASS_D),
                    1.0,
                    Vec3::ZERO,
                    SHADE_GOLD,
                ));
            }
            cube(out, pan, Vec3::new(1.9, 0.12, 1.2), 0.0, lin(BRASS), SHADE_GOLD);
            cube(out, pan - Vec3::new(0.0, 0.1, 0.0), Vec3::new(1.5, 0.1, 0.9), 0.0, lin(BRASS_D), SHADE_GOLD);
            push_merged(
                out,
                &pile,
                Mat4::from_translation(pan + Vec3::new(0.0, 0.05, 0.0))
                    * Mat4::from_scale(Vec3::new(1.4, if s > 0.0 { 1.3 } else { 0.8 }, 0.9)),
                0.05,
            );
        }
        top
    }

    /// A copper brazier hanging on chains from the dark above (its fire is drawn each frame).
    fn build_brazier(&mut self, at: Vec3) {
        let out = &mut self.static_cubes;
        cube(out, at - Vec3::new(0.0, 0.1, 0.0), Vec3::new(0.5, 0.18, 0.5), PI / 4.0, lin(COPPER), SHADE_GOLD);
        cube(out, at - Vec3::new(0.0, 0.22, 0.0), Vec3::new(0.32, 0.1, 0.32), PI / 4.0, lin(COPPER_D), SHADE_GOLD);
        cube(out, at, Vec3::new(0.56, 0.05, 0.56), 0.0, lin(COPPER_D), SHADE_GOLD);
        cube(out, at + Vec3::new(0.0, 0.03, 0.0), Vec3::new(0.42, 0.04, 0.42), 0.0, lin(0x3a1c0c), SHADE_LIT);
        for k in 0..3 {
            let a = k as f32 * TAU / 3.0;
            let foot = at + Vec3::new(a.cos() * 0.26, 0.02, a.sin() * 0.26);
            let hook = at + Vec3::new(0.0, 1.3, 0.0);
            let mid = (foot + hook) * 0.5;
            let rot = Quat::from_rotation_arc(Vec3::Y, (hook - foot).normalize());
            out.push(Instance::new(
                Mat4::from_scale_rotation_translation(Vec3::new(0.025, (hook - foot).length(), 0.025), rot, mid),
                lin(0x2a2420),
                1.0,
                Vec3::ZERO,
                SHADE_LIT,
            ));
        }
        cube(out, at + Vec3::new(0.0, 3.0, 0.0), Vec3::new(0.035, 3.4, 0.035), 0.0, lin(0x2a2420), SHADE_LIT);
    }

    /// Each frame: the braziers' fire and light, the gold's glints, the cold light on the scales,
    /// and the gate.
    pub(super) fn emit_counting_house(&self, s: &SceneState, time: f32, hero: Vec3, f: &mut Frame) {
        let Some(cv) = &self.counting else { return };
        f.bounds = Vec4::new(-0.5, -0.5, s.grid.width as f32 - 0.5, s.grid.height as f32 - 0.5);
        for (i, b) in cv.braziers.iter().enumerate() {
            let fi = i as f32;
            for k in 0..5 {
                let t = (time * 2.2 + fi * 0.37 + k as f32 * 0.2).fract();
                let a = fi * 1.9 + k as f32 * 1.3;
                let p = *b + Vec3::new(a.cos() * 0.12 * (1.0 - t), 0.08 + t * 0.42, a.sin() * 0.12 * (1.0 - t));
                let size = 0.14 * (1.0 - t) + 0.03;
                let c = lin(FLAME).lerp(lin(FLAME_TIP), t);
                f.additive.push(Instance::new(
                    Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(size, size * 1.7, size)),
                    c,
                    0.6 * (1.0 - t),
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
            glow_orb(f, *b + Vec3::new(0.0, 0.2, 0.0), 0.55, lin(AMBER), 0.16 + 0.03 * (time * 7.0 + fi).sin());
        }
        // The nearest braziers light the hall; one hangs over the throne whatever.
        for (i, p) in nearest(&cv.braziers, hero, 5).into_iter().enumerate() {
            let flick = 5.5 + (time * 6.0 + i as f32 * 1.3).sin() * 0.5 + (time * 15.0 + i as f32).sin() * 0.25;
            f.lights.push(Light { pos: p + Vec3::new(0.0, 0.25, 0.0), color: lin(AMBER) * flick, distance: 9.0 });
        }
        f.lights.push(Light { pos: cv.throne + Vec3::new(0.0, 2.0, 2.2), color: lin(GOLD) * 3.5, distance: 7.0 });
        // The prince stands in his own light: a warm key before him and above, a cold rim behind.
        for e in s.enemies {
            let Some(d) = e.duel.as_deref().filter(|d| d.alive() && d.breed.kind == EnemyKind::Mammon) else { continue };
            let p = Vec3::new(d.pos.x as f32, 0.0, d.pos.y as f32);
            let ahead = Vec3::new(d.facing.x as f32, 0.0, d.facing.y as f32);
            f.lights.push(Light { pos: p + ahead * 2.4 + Vec3::new(0.0, 3.6, 0.0), color: lin(0xffd8a0) * 6.0, distance: 7.5 });
            f.lights.push(Light { pos: p - ahead * 1.8 + Vec3::new(0.0, 3.2, 0.0), color: lin(MIST_TEAL) * 4.0, distance: 5.5 });
        }
        // A cold shaft from above on the scales: the board's mist teal against the amber.
        f.lights.push(Light { pos: cv.scales + Vec3::new(0.0, 1.5, 2.5), color: lin(MIST_TEAL) * 5.0, distance: 9.0 });
        // The gate he came in by.
        let door = s.gate.map(|g| g.door).unwrap_or(0.0);
        self.emit_portal(&cv.gate, door, false, time, hero, f);
    }

    /// A prince's coins on the floor: flying from his hand (or falling from above), then lying
    /// there glinting as a ring of gold grows round each (the tell), and bursting into gold spikes.
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
                // Spikes of gold thrust up out of the floor, and sink back.
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
                    f.opaque[MeshKind::Cube as usize].push(Instance::new(m, lin(GOLD), 1.0, lin(GOLD) * 0.25, SHADE_GOLD));
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
            f.opaque[MeshKind::Cylinder as usize].push(Instance::new(m, lin(GOLD), 1.0, lin(GOLD) * 0.3, SHADE_GOLD));
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
