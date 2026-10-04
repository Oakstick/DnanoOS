//! Sloth's court, **the Hall of Stopped Clocks** (`logic::court::sloth` lays it out), to the art
//! direction: the board's palette (charcoal, pale stone, mist teal) gone grey and drowsy, with
//! Sloth's slate blue as its accent and the dim amber of guttering candles against cold moonlight.
//!
//! A grey bedchamber the size of a hall: a floor of worn grey boards under a fog that lies on it,
//! a great faded rug in the middle; walls hung with grey-blue hangings over a wooden wainscot,
//! clocks of every size hung on them, all stopped, and candles guttering in iron sconces; tall
//! narrow windows in the north wall letting in grey moonlight in shafts. Great beams cross
//! overhead on four timber posts, hung with cobwebs. Tall longcase clocks stand round the walls;
//! at the north end Belphegor's great wing chair stands against the wall, upholstered in slate
//! velvet, and before it the giant hourglass, its sand stopped in mid-fall; a canopied bed stands
//! unslept-in in the north-east corner, and a drift of sand has run out across the south-west.
//! When Belphegor falls the clocks start again: their pendulums swing, their hands go round, and
//! the hourglass's sand runs.
//!
//! Also drawn here: Belphegor asleep and his moves' poses (`pose_belphegor`, over his gait), the
//! Zs and dream-mist over him while he sleeps, and the sand of hours on the floor (`emit_sand`).

use super::forge::segment;
use super::*;
use crate::logic::combat::prince::sloth::Slumber;
use crate::logic::combat::prince::{Trap, SPIKES};
use crate::logic::combat::{Duelist, MoveId};
use crate::logic::court::sloth::{self as plan, Piece, BED, CHAIR, DRIFT, HOURGLASS, POSTS};
use crate::logic::hub::Gate;

mod pal {
    pub const BOARD: [u32; 4] = [0x5a5650, 0x524e49, 0x625e57, 0x4a4743];
    pub const BOARD_GAP: u32 = 0x24221f;
    pub const RUG: u32 = 0x3e4862;
    pub const RUG_D: u32 = 0x313a50;
    pub const RUG_BORDER: u32 = 0x8a8470;
    pub const RUG_FIGURE: u32 = 0x6a7088;
    pub const WAINSCOT: u32 = 0x3e3934;
    pub const WAINSCOT_L: u32 = 0x4e4842;
    pub const HANGING: u32 = 0x485066;
    pub const HANGING_D: u32 = 0x394054;
    pub const HANGING_FIGURE: u32 = 0x5c6680;
    pub const TIMBER: u32 = 0x3a342e;
    pub const TIMBER_L: u32 = 0x4a433b;
    pub const TIMBER_D: u32 = 0x2a2622;
    pub const STONE: u32 = 0x6a6862;
    pub const CLOCK_WOOD: u32 = 0x3c2e26;
    pub const CLOCK_WOOD_L: u32 = 0x4e3c30;
    pub const DIAL: u32 = 0xd8d0b8;
    pub const HAND: u32 = 0x1c1a1a;
    pub const BRASS: u32 = 0xb08a40;
    pub const BRASS_D: u32 = 0x7a6028;
    pub const IRON: u32 = 0x2e2e32;
    pub const VELVET: u32 = 0x4a5878;
    pub const VELVET_D: u32 = 0x384462;
    pub const LINEN: u32 = 0xb8b6ae;
    pub const LINEN_D: u32 = 0x8e8c86;
    pub const COVER: u32 = 0x5a6278;
    pub const SAND: u32 = 0xc8aa70;
    pub const SAND_D: u32 = 0xa88a54;
    pub const SAND_L: u32 = 0xdcc290;
    pub const GLASS: u32 = 0xb8d0dc;
    pub const WEB: u32 = 0xc8c8c4;
    pub const WAX: u32 = 0xd8d0b8;
    pub const FLAME: u32 = 0xffd090;
    pub const FLAME_TIP: u32 = 0xff9040;
    pub const CANDLE: u32 = 0xffc070;
    pub const MOONLIGHT: u32 = 0x9ab0e0;
    pub const MIST: u32 = 0x8a92a8;
    pub const SLATE: u32 = 0x8a96b0;
    pub const DREAM: u32 = 0xa070ff;
    pub const ZZZ: u32 = 0xb8c8ff;
}
use pal::*;

/// Shade modes: cloth, wood, iron and brass (`voxel_surface`), and stone.
const SHADE_CLOTH: f32 = 8.0;
const SHADE_WOOD: f32 = 12.0;
const SHADE_IRON: f32 = 6.0;
const SHADE_BRASS: f32 = 13.0;
const SHADE_STONE: f32 = 18.0;

/// The hall's walls stand this tall, the beams cross this high, the candles burn at this height.
const WALL_H: f32 = 3.0;
const BEAM_Y: f32 = 4.3;
const SCONCE_Y: f32 = 1.75;
/// The giant hourglass's height over the floor, and its bulbs' radius.
const GLASS_H: f32 = 3.2;
const BULB: f32 = 0.62;

/// Drowsy and grey: guttering candles the warm light, cold moonlight through the north windows,
/// a slate-blue fill; the fog lies thick on the floor.
pub fn sloth_atmosphere() -> Atmosphere {
    Atmosphere {
        background: 0x090a0e,
        fog_near: 16.0,
        fog_far: 44.0,
        hemi_sky: lin(SLATE) * 0.46,
        hemi_ground: lin(0x4a4640) * 0.42,
        ambient: lin(0x50566a) * 0.62,
        sun_dir: Vec3::new(0.25, 1.0, -0.6).normalize(),
        sun: lin(0xb8c4e0) * 0.85,
        exposure: 1.25,
        night_sky: 0.0,
        bloom: 0.7,
        moon_view: Vec3::new(0.0, 0.3, -1.0).normalize(),
        cloud_tint: lin(0x10121a),
    }
}

/// Its ground haze: density and falloff (the fog lying on the floor: thick, and low).
pub const HAZE: (f32, f32) = (0.05, 1.0);

/// What the view keeps of the Hall of Stopped Clocks for its moving parts.
#[derive(Clone, Debug)]
pub struct ClocksView {
    /// The candles' flames in their sconces (some have guttered out: `lit`).
    pub candles: Vec<(Vec3, bool)>,
    /// Each clock's face: its middle, the way it faces, its radius, its pendulum's pivot (if it has
    /// one) and the hour it stopped at.
    pub clocks: Vec<ClockFace>,
    /// The north windows' panes (their middles; they face +z).
    pub windows: Vec<Vec3>,
    pub gate: Gate,
    /// The giant hourglass's foot.
    pub hourglass: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub struct ClockFace {
    pub at: Vec3,
    pub normal: Vec3,
    pub radius: f32,
    pub pendulum: Option<Vec3>,
    pub stopped: f32,
}

fn cube(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot_y: f32, color: Vec3, shade: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(rot_y), pos), color, 1.0, Vec3::ZERO, shade));
}

fn bar(out: &mut Vec<Instance>, a: Vec3, b: Vec3, thick: f32, color: Vec3, shade: f32) {
    out.push(Instance::new(segment(a, b, thick), color, 1.0, Vec3::ZERO, shade));
}

/// A cobweb spun across a corner: threads radiating from `at` in the plane of `u` and `v`, and
/// rings round them.
fn cobweb(out: &mut Vec<Instance>, at: Vec3, u: Vec3, v: Vec3, size: f32) {
    let spokes = 6;
    let web = lin(WEB) * 0.8;
    for k in 0..spokes {
        let a = k as f32 / (spokes - 1) as f32 * PI * 0.5;
        bar(out, at, at + (u * a.cos() + v * a.sin()) * size, 0.005, web, SHADE_LIT);
    }
    for ring in 1..4 {
        let r = size * ring as f32 / 4.0;
        for k in 0..spokes - 1 {
            let (a0, a1) = (k as f32 / (spokes - 1) as f32 * PI * 0.5, (k + 1) as f32 / (spokes - 1) as f32 * PI * 0.5);
            let p0 = at + (u * a0.cos() + v * a0.sin()) * r;
            let p1 = at + (u * a1.cos() + v * a1.sin()) * r * 0.95;
            bar(out, p0, p1, 0.004, web, SHADE_LIT);
        }
    }
}

fn h2(x: f32, y: f32) -> f32 {
    hash(x as f64 * 1.31 + 0.7, y as f64 * 2.17 + 0.3) as f32
}

/// The way from the wall a piece at (x, y) stands against, into the hall (+z if none).
fn into_hall(x: i32, y: i32) -> Vec3 {
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        if !plan::in_hall(x - dx, y - dy) {
            return Vec3::new(dx as f32, 0.0, dy as f32);
        }
    }
    Vec3::Z
}

impl WorldView {
    /// The Hall of Stopped Clocks' static scene.
    pub(super) fn build_sloth_court(&mut self, grid: &Grid) {
        let floor_tile = |x: i32, y: i32| plan::in_hall(x, y) || (x, y) == plan::GATE;
        let c = Vec3::new(plan::CENTRE.0 as f32, 0.0, plan::CENTRE.1 as f32);

        // ---- the floor: worn grey boards running east to west, the rug in the middle, fog on it
        for y in 0..grid.height {
            for x in 0..grid.width {
                if !floor_tile(x, y) {
                    continue;
                }
                let (xf, yf) = (x as f32, y as f32);
                let out = &mut self.static_cubes;
                cube(out, Vec3::new(xf, -0.1, yf), Vec3::new(1.0, 0.08, 1.0), 0.0, lin(BOARD_GAP), SHADE_LIT);
                for k in 0..3 {
                    // Boards of odd lengths, their ends staggered row to row.
                    let row = y * 3 + k;
                    let h = h2(row as f32, (x + row * 2) as f32 / 3.0);
                    let col = lin(BOARD[(h * 4.0) as usize % 4]) * (0.92 + h * 0.14);
                    let z = yf - 0.333 + k as f32 * 0.333;
                    cube(out, Vec3::new(xf, -0.04, z), Vec3::new(0.995, 0.08, 0.31), 0.0, col, SHADE_WOOD);
                    if (x + row * 5) % 7 == 0 {
                        // A board's end, and its nail heads.
                        cube(out, Vec3::new(xf + 0.3, 0.001, z), Vec3::new(0.02, 0.004, 0.3), 0.0, lin(BOARD_GAP), SHADE_LIT);
                        cube(out, Vec3::new(xf + 0.24, 0.002, z), Vec3::new(0.035, 0.006, 0.035), 0.0, lin(IRON), SHADE_IRON);
                    }
                }
                // Fog lying on the floor: thin sheets, thicker toward the walls.
                let edge = (((x - plan::CENTRE.0).abs().max((y - plan::CENTRE.1).abs()) as f32) / plan::HALF as f32).clamp(0.0, 1.0);
                for (k, &(lift, alpha)) in [(0.1f32, 0.34f32), (0.3, 0.26), (0.58, 0.14)].iter().enumerate() {
                    let a = alpha * (0.75 + 0.5 * edge);
                    let m = Mat4::from_translation(Vec3::new(xf, lift, yf)) * Mat4::from_scale(Vec3::new(1.0, 0.002, 1.0));
                    self.mist.push(Instance::new(m, lin(MIST), a, Vec3::new(20.0 + k as f32, lift, 0.0), SHADE_CLOUD));
                }
            }
        }
        self.build_rug(c);

        // ---- the walls: hangings over a wainscot, clocks hung on them, candles in sconces
        let mut candles = Vec::new();
        let mut clocks = Vec::new();
        let mut windows = Vec::new();
        for (x, y, t) in grid.cells() {
            if t != Tile::Rock || floor_tile(x, y) {
                continue;
            }
            let open: Vec<Dir> =
                CARDINALS.iter().copied().filter(|d| plan::in_hall(P::new(x, y).step(*d).x, P::new(x, y).step(*d).y)).collect();
            let near = !open.is_empty() || [(-1, -1), (1, -1), (-1, 1), (1, 1)].iter().any(|&(dx, dy)| plan::in_hall(x + dx, y + dy));
            if !near || (y == plan::GATE.1 && (x - plan::GATE.0).abs() <= 1) {
                continue;
            }
            let start = self.static_cubes.len();
            let (xf, yf) = (x as f32, y as f32);
            let h = h2(xf * 3.0, yf * 5.0);
            let out = &mut self.static_cubes;
            cube(out, Vec3::new(xf, WALL_H / 2.0, yf), Vec3::new(1.0, WALL_H, 1.0), 0.0, lin(HANGING_D) * 0.8, SHADE_LIT);
            cube(out, Vec3::new(xf, WALL_H + 0.06, yf), Vec3::new(1.1, 0.12, 1.1), 0.0, lin(TIMBER_D), SHADE_WOOD);
            for &d in &open {
                let (dx, dy) = d.vec();
                let n = Vec3::new(dx as f32, 0.0, dy as f32);
                let along = if dx == 0 { Vec3::X } else { Vec3::Z };
                let face = Vec3::new(xf, 0.0, yf) + n * 0.5;
                let size = |a: f32, up: f32, depth: f32| along * a + Vec3::Y * up + Vec3::new(n.x.abs(), 0.0, n.z.abs()) * depth;
                // The wainscot: panels of dark wood, a rail on top.
                cube(out, face + Vec3::new(0.0, 0.42, 0.0) + n * 0.02, size(1.0, 0.84, 0.04), 0.0, lin(WAINSCOT), SHADE_WOOD);
                cube(out, face + Vec3::new(0.0, 0.42, 0.0) + n * 0.04, size(0.78, 0.6, 0.02), 0.0, lin(WAINSCOT_L), SHADE_WOOD);
                cube(out, face + Vec3::new(0.0, 0.86, 0.0) + n * 0.05, size(1.0, 0.06, 0.08), 0.0, lin(TIMBER), SHADE_WOOD);
                // The hanging: grey-blue, faded, its figure a lozenge, hanging in soft folds.
                let fold = ((x + y) % 2) as f32;
                cube(
                    out,
                    face + Vec3::new(0.0, 1.9, 0.0) + n * (0.025 + fold * 0.02),
                    size(1.0, 2.0, 0.03),
                    0.0,
                    lin(HANGING) * (0.94 + 0.1 * fold),
                    SHADE_CLOTH,
                );
                for (ux, uy) in [(0.0f32, 0.0f32), (0.18, 0.18), (-0.18, 0.18), (0.18, -0.18), (-0.18, -0.18)] {
                    cube(
                        out,
                        face + along * ux + Vec3::new(0.0, 1.95 + uy, 0.0) + n * (0.05 + fold * 0.02),
                        size(0.1, 0.1, 0.01),
                        0.0,
                        lin(HANGING_FIGURE),
                        SHADE_CLOTH,
                    );
                }
                // A cornice of timber under the beams.
                cube(out, face + Vec3::new(0.0, WALL_H - 0.08, 0.0) + n * 0.06, size(1.0, 0.16, 0.12), 0.0, lin(TIMBER_L), SHADE_WOOD);
                // Every few tiles a clock hung on the wall, of its own size; elsewhere a sconce.
                let k = (x * 7 + y * 13) % 5;
                if k == 0 || k == 3 {
                    let r = 0.16 + h * 0.2;
                    let at = face + along * ((h - 0.5) * 0.3) + Vec3::new(0.0, 1.45 + h * 0.6, 0.0) + n * 0.09;
                    let case = if k == 0 { CLOCK_WOOD } else { CLOCK_WOOD_L };
                    cube(out, at - n * 0.03, size(r * 2.3, r * 2.3, 0.08), 0.0, lin(case), SHADE_WOOD);
                    if k == 3 {
                        // A cuckoo clock's little roof, and its weights hanging on chains.
                        cube(out, at + Vec3::new(0.0, r * 1.35, 0.0), size(r * 2.6, r * 0.25, 0.14), 0.0, lin(TIMBER_D), SHADE_WOOD);
                        for s in [-1.0f32, 1.0] {
                            let top = at + along * (s * r * 0.4) - Vec3::new(0.0, r * 1.1, 0.0);
                            bar(out, top, top - Vec3::new(0.0, 0.45 + s * 0.1, 0.0), 0.01, lin(BRASS_D), SHADE_BRASS);
                            cube(out, top - Vec3::new(0.0, 0.5 + s * 0.1, 0.0), size(0.05, 0.14, 0.05), 0.0, lin(BRASS), SHADE_BRASS);
                        }
                    }
                    clocks.push(ClockFace { at, normal: n, radius: r, pendulum: None, stopped: h * TAU });
                } else if k == 1 && h > 0.25 {
                    // An iron sconce, a candle in it (some guttered out).
                    let at = face + Vec3::new(0.0, SCONCE_Y - 0.25, 0.0) + n * 0.12;
                    cube(out, at - n * 0.06, size(0.1, 0.2, 0.06), 0.0, lin(IRON), SHADE_IRON);
                    bar(out, at - n * 0.06, at + n * 0.1, 0.03, lin(IRON), SHADE_IRON);
                    cube(out, at + n * 0.12, size(0.16, 0.03, 0.16), 0.0, lin(IRON), SHADE_IRON);
                    let tall = 0.1 + h * 0.18;
                    cube(out, at + n * 0.12 + Vec3::new(0.0, tall / 2.0 + 0.015, 0.0), size(0.06, tall, 0.06), 0.0, lin(WAX), SHADE_LIT);
                    // A drip of wax down the candle's side.
                    cube(
                        out,
                        at + n * 0.15 + Vec3::new(0.0, tall * 0.4, 0.0),
                        size(0.02, tall * 0.5, 0.02),
                        0.0,
                        lin(WAX) * 0.9,
                        SHADE_LIT,
                    );
                    candles.push((at + n * 0.12 + Vec3::new(0.0, tall + 0.06, 0.0), h > 0.4));
                }
                // Cobwebs in the corners where the wall meets the cornice.
                if (x * 3 + y) % 6 == 0 {
                    cobweb(out, face + along * 0.5 + Vec3::new(0.0, WALL_H - 0.16, 0.0) + n * 0.08, -along, Vec3::NEG_Y + n * 0.2, 0.55);
                }
            }
            // Tall narrow windows high in the north wall, letting in the moon.
            if open.contains(&Dir::Down) && y == plan::CENTRE.1 - plan::HALF - 1 && [7, 11, 21, 25].contains(&x) {
                let face = Vec3::new(xf, 0.0, yf + 0.5);
                cube(out, face + Vec3::new(0.0, 2.25, 0.04), Vec3::new(0.62, 1.5, 0.08), 0.0, lin(TIMBER_D), SHADE_WOOD);
                cube(out, face + Vec3::new(0.0, 3.06, 0.04), Vec3::new(0.44, 0.14, 0.08), 0.0, lin(TIMBER_D), SHADE_WOOD);
                cube(out, face + Vec3::new(0.0, 1.5, 0.1), Vec3::new(0.7, 0.06, 0.16), 0.0, lin(STONE), SHADE_STONE);
                windows.push(face + Vec3::new(0.0, 2.3, 0.07));
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

        // ---- what stands on the floor: the tall clocks, the posts, the chair, the hourglass, the bed
        for (x, y, _) in grid.cells() {
            let Some(piece) = plan::piece(x, y) else { continue };
            let base = Vec3::new(x as f32, 0.0, y as f32);
            let h = h2(x as f32 * 2.1, y as f32 * 1.3);
            let start = self.static_cubes.len();
            let (radius, height) = match piece {
                Piece::Clock => {
                    clocks.push(self.build_longcase(base, into_hall(x, y), h));
                    (0.45, 2.9)
                }
                Piece::Post => {
                    self.build_post(base, h);
                    (0.4, BEAM_Y)
                }
                // (The chair, the hourglass and the bed are built once, from their first tile.)
                Piece::Chair if (x, y) == (CHAIR.0, CHAIR.1) => {
                    self.build_great_chair();
                    (1.6, 3.8)
                }
                Piece::Hourglass => {
                    self.build_hourglass_frame(base);
                    (0.7, GLASS_H)
                }
                Piece::Bed if (x, y) == (BED.0, BED.1) => {
                    self.build_bed();
                    (1.6, 2.8)
                }
                _ => continue,
            };
            let len = self.static_cubes.len() - start;
            self.rock_index.insert(tile_key(x, y), (start, len));
            let shades: Vec<f32> = self.static_cubes[start..start + len].iter().map(|i| i.emissive[3]).collect();
            let mid = match piece {
                Piece::Chair => Vec3::new((CHAIR.0 + CHAIR.2) as f32 / 2.0, 0.0, (CHAIR.1 + CHAIR.3) as f32 / 2.0),
                Piece::Bed => Vec3::new((BED.0 + BED.2) as f32 / 2.0, 0.0, (BED.1 + BED.3) as f32 / 2.0),
                _ => base,
            };
            self.trees.push(Tree { range: (start, len), base: mid, radius, height, fade: 1.0, alive: true, shades: Some(shades) });
        }

        // ---- the beams overhead, on the posts and the walls, hung with cobwebs
        self.build_beams();
        // ---- the drift of sand run out across the south-west floor
        self.build_drift();
        // ---- the gate: Sloth's portal in the south wall, looking into the hall
        let gate = Gate { tile: P::new(plan::GATE.0, plan::GATE.1), facing: Dir::Up, sin: crate::logic::sins::Sin::Sloth };
        self.build_portal_arch(&gate);
        let hourglass = Vec3::new(HOURGLASS.0 as f32, 0.0, HOURGLASS.1 as f32);
        self.clocks = Some(ClocksView { candles, clocks, windows, gate, hourglass });
    }

    /// The great faded rug in the middle of the floor: slate blue, a cream border, a figure of
    /// lozenges, its fringe frayed, one corner turned up.
    fn build_rug(&mut self, c: Vec3) {
        let out = &mut self.static_cubes;
        let (hw, hd) = (6.5f32, 5.0f32);
        let mid = c + Vec3::new(0.0, 0.004, 1.0);
        cube(out, mid, Vec3::new(hw * 2.0, 0.012, hd * 2.0), 0.0, lin(RUG), SHADE_CLOTH);
        for (w, d, col) in
            [(hw * 2.0 - 0.3, hd * 2.0 - 0.3, RUG_BORDER), (hw * 2.0 - 0.7, hd * 2.0 - 0.7, RUG_D), (hw * 2.0 - 0.9, hd * 2.0 - 0.9, RUG)]
        {
            cube(out, mid + Vec3::new(0.0, 0.002, 0.0), Vec3::new(w, 0.012, d), 0.0, lin(col), SHADE_CLOTH);
        }
        for i in -4..=4 {
            for j in -3..=3 {
                if (i + j) % 2 != 0 {
                    continue;
                }
                let p = mid + Vec3::new(i as f32 * 1.3, 0.006, j as f32 * 1.2);
                cube(out, p, Vec3::new(0.5, 0.012, 0.5), PI / 4.0, lin(RUG_FIGURE), SHADE_CLOTH);
                cube(out, p + Vec3::new(0.0, 0.002, 0.0), Vec3::new(0.22, 0.012, 0.22), PI / 4.0, lin(RUG_BORDER) * 0.8, SHADE_CLOTH);
            }
        }
        for k in 0..40 {
            let x = -hw + 0.15 + k as f32 * (hw * 2.0 - 0.3) / 39.0;
            for s in [-1.0f32, 1.0] {
                cube(out, mid + Vec3::new(x, 0.0, s * (hd + 0.08)), Vec3::new(0.04, 0.008, 0.16), 0.0, lin(RUG_BORDER) * 0.9, SHADE_CLOTH);
            }
        }
    }

    /// A tall longcase clock against the wall, facing `n` into the hall: plinth, trunk with its
    /// glass door and pendulum, hood with its dial and finials. Every one a different size.
    fn build_longcase(&mut self, base: Vec3, n: Vec3, h: f32) -> ClockFace {
        let out = &mut self.static_cubes;
        let yaw = n.x.atan2(n.z);
        let tall = 2.2 + h * 0.8;
        let wide = 0.55 + h * 0.2;
        let at = base - n * 0.15;
        let wood = lin(if h > 0.5 { CLOCK_WOOD } else { CLOCK_WOOD_L });
        cube(out, at + Vec3::new(0.0, 0.25, 0.0), Vec3::new(wide + 0.12, 0.5, 0.5), yaw, wood * 0.85, SHADE_WOOD);
        cube(out, at + Vec3::new(0.0, 0.52, 0.0), Vec3::new(wide + 0.16, 0.06, 0.54), yaw, wood, SHADE_WOOD);
        let trunk = tall * 0.55;
        cube(out, at + Vec3::new(0.0, 0.55 + trunk / 2.0, 0.0), Vec3::new(wide, trunk, 0.42), yaw, wood, SHADE_WOOD);
        // The trunk's door: a pane of dark glass, the pendulum's brass bob behind it.
        let door = at + n * 0.22 + Vec3::new(0.0, 0.55 + trunk / 2.0, 0.0);
        cube(out, door, Vec3::new(wide * 0.6, trunk * 0.75, 0.02), yaw, lin(0x1a1c22), SHADE_LIT);
        let hood_y = 0.55 + trunk;
        let hood = tall - hood_y;
        cube(out, at + Vec3::new(0.0, hood_y + hood / 2.0, 0.02), Vec3::new(wide + 0.1, hood, 0.48), yaw, wood, SHADE_WOOD);
        cube(out, at + Vec3::new(0.0, tall + 0.04, 0.02), Vec3::new(wide + 0.18, 0.08, 0.54), yaw, wood * 0.8, SHADE_WOOD);
        for s in [-1.0f32, 0.0, 1.0] {
            let side = Vec3::new(n.z, 0.0, -n.x);
            let top = at + side * (s * wide * 0.45) + Vec3::new(0.0, tall + 0.08, 0.02);
            cube(out, top + Vec3::new(0.0, 0.08, 0.0), Vec3::new(0.06, 0.16, 0.06), yaw, lin(BRASS), SHADE_BRASS);
            cube(out, top + Vec3::new(0.0, 0.2, 0.0), Vec3::new(0.08, 0.08, 0.08), yaw + PI / 4.0, lin(BRASS), SHADE_BRASS);
        }
        let r = (wide * 0.42).min(hood * 0.42);
        let face = at + n * 0.27 + Vec3::new(0.0, hood_y + hood / 2.0, 0.0);
        // A cobweb from its hood to the wall.
        let side = Vec3::new(n.z, 0.0, -n.x);
        cobweb(out, at + side * (wide * 0.5) + Vec3::new(0.0, tall - 0.05, 0.0) - n * 0.1, side, Vec3::NEG_Y, 0.35);
        ClockFace {
            at: face,
            normal: n,
            radius: r,
            pendulum: Some(door + n * 0.02 + Vec3::new(0.0, trunk * 0.32, 0.0)),
            stopped: h * TAU * 3.0,
        }
    }

    /// A timber post holding up the beams: a stone footing, a chamfered shaft, a bracket at the top.
    fn build_post(&mut self, base: Vec3, h: f32) {
        let out = &mut self.static_cubes;
        cube(out, base + Vec3::new(0.0, 0.12, 0.0), Vec3::new(0.72, 0.24, 0.72), 0.0, lin(STONE), SHADE_STONE);
        cube(out, base + Vec3::new(0.0, BEAM_Y / 2.0 + 0.1, 0.0), Vec3::new(0.46, BEAM_Y, 0.46), h * 0.1, lin(TIMBER), SHADE_WOOD);
        cube(
            out,
            base + Vec3::new(0.0, BEAM_Y / 2.0 + 0.1, 0.0),
            Vec3::new(0.5, BEAM_Y - 0.4, 0.34),
            h * 0.1 + PI / 4.0,
            lin(TIMBER_D),
            SHADE_WOOD,
        );
        for s in [-1.0f32, 1.0] {
            bar(out, base + Vec3::new(0.0, BEAM_Y - 0.9, 0.0), base + Vec3::new(s * 0.9, BEAM_Y, 0.0), 0.16, lin(TIMBER_L), SHADE_WOOD);
        }
        // An iron band, and a candle-stand bracket with a guttered stub.
        cube(out, base + Vec3::new(0.0, 1.2, 0.0), Vec3::new(0.5, 0.06, 0.5), 0.0, lin(IRON), SHADE_IRON);
    }

    /// The great beams across the hall on the posts, and cobwebs hung from them.
    fn build_beams(&mut self) {
        let out = &mut self.static_cubes;
        let (lo, hi) = ((plan::CENTRE.0 - plan::HALF) as f32 - 0.5, (plan::CENTRE.0 + plan::HALF) as f32 + 0.5);
        let rows: Vec<f32> = POSTS.iter().map(|p| p.1).collect::<std::collections::BTreeSet<i32>>().into_iter().map(|y| y as f32).collect();
        for &z in &rows {
            cube(out, Vec3::new((lo + hi) / 2.0, BEAM_Y + 0.15, z), Vec3::new(hi - lo, 0.3, 0.32), 0.0, lin(TIMBER_L), SHADE_WOOD);
        }
        for col in [POSTS[0].0, POSTS[1].0] {
            let x = col as f32;
            cube(
                out,
                Vec3::new(x, BEAM_Y + 0.45, plan::CENTRE.1 as f32),
                Vec3::new(0.34, 0.26, (plan::HALF * 2 + 1) as f32),
                0.0,
                lin(TIMBER_D),
                SHADE_WOOD,
            );
        }
        // Cobwebs sagging from the beams at the posts.
        for (i, &(px, py)) in POSTS.iter().enumerate() {
            let at = Vec3::new(px as f32, BEAM_Y - 0.02, py as f32);
            for (u, s) in [(Vec3::X, 0.8f32), (Vec3::NEG_X, 0.7), (Vec3::Z, 0.6)] {
                if (i + (s * 10.0) as usize).is_multiple_of(2) {
                    cobweb(out, at, u, Vec3::NEG_Y, s);
                }
            }
        }
        // Long strands of web hanging from the beams, tufted at their ends.
        for k in 0..18 {
            let x = lo + 1.5 + (h2(k as f32, 3.0) * (hi - lo - 3.0));
            let z = rows[k % rows.len()] + (h2(k as f32, 5.0) - 0.5) * 0.3;
            let len = 0.4 + h2(k as f32, 7.0) * 1.1;
            let top = Vec3::new(x, BEAM_Y - 0.02, z);
            bar(out, top, top - Vec3::new(0.0, len, 0.0), 0.006, lin(WEB) * 0.8, SHADE_LIT);
            cube(out, top - Vec3::new(0.0, len, 0.0), Vec3::new(0.06, 0.08, 0.06), k as f32, lin(WEB) * 0.6, SHADE_LIT);
        }
    }

    /// Belphegor's great wing chair against the north wall: dark carved wood, a deep seat in slate
    /// velvet, wings, rolled arms, a high back with a crescent moon carved at its crest.
    fn build_great_chair(&mut self) {
        let out = &mut self.static_cubes;
        let x = (CHAIR.0 + CHAIR.2) as f32 / 2.0;
        let back_z = CHAIR.1 as f32 + 0.1;
        let front_z = CHAIR.3 as f32 + 0.95;
        let depth = front_z - back_z;
        let mid_z = (back_z + front_z) / 2.0;
        let w = (CHAIR.2 - CHAIR.0 + 1) as f32 - 0.1;
        let wood = lin(TIMBER);
        // Claw feet, the seat's frame, its cushion.
        for (sx, sz) in [(-1.0f32, 0.0f32), (1.0, 0.0), (-1.0, 1.0), (1.0, 1.0)] {
            let p = Vec3::new(x + sx * (w / 2.0 - 0.2), 0.18, back_z + 0.3 + sz * (depth - 0.6));
            cube(out, p, Vec3::new(0.3, 0.36, 0.3), 0.0, lin(TIMBER_D), SHADE_WOOD);
            cube(out, p + Vec3::new(0.0, -0.12, 0.12), Vec3::new(0.34, 0.12, 0.2), 0.0, lin(BRASS_D), SHADE_BRASS);
        }
        cube(out, Vec3::new(x, 0.5, mid_z), Vec3::new(w, 0.34, depth), 0.0, wood, SHADE_WOOD);
        cube(out, Vec3::new(x, 0.74, mid_z + 0.1), Vec3::new(w - 0.5, 0.2, depth - 0.3), 0.0, lin(VELVET), SHADE_CLOTH);
        cube(out, Vec3::new(x, 0.62, front_z - 0.04), Vec3::new(w - 0.3, 0.3, 0.1), 0.0, lin(VELVET_D), SHADE_CLOTH);
        // The high back, its velvet tufted with brass studs, the crescent at its crest.
        let back_h = 3.4;
        cube(out, Vec3::new(x, back_h / 2.0 + 0.3, back_z + 0.15), Vec3::new(w, back_h, 0.34), 0.0, wood, SHADE_WOOD);
        cube(out, Vec3::new(x, back_h / 2.0 + 0.55, back_z + 0.34), Vec3::new(w - 0.55, back_h - 0.6, 0.06), 0.0, lin(VELVET), SHADE_CLOTH);
        for i in 0..4 {
            for j in 0..5 {
                let p = Vec3::new(x - 0.75 + i as f32 * 0.5, 1.1 + j as f32 * 0.5, back_z + 0.38);
                cube(out, p, Vec3::new(0.05, 0.05, 0.03), 0.0, lin(BRASS), SHADE_BRASS);
            }
        }
        let crest = Vec3::new(x, back_h + 0.6, back_z + 0.15);
        cube(out, crest, Vec3::new(w * 0.7, 0.34, 0.3), 0.0, wood, SHADE_WOOD);
        for k in 0..9 {
            let a = PI * 0.15 + k as f32 / 8.0 * PI * 1.1;
            let p = crest + Vec3::new(a.cos() * 0.42, 0.5 + a.sin() * 0.42, 0.0);
            cube(out, p, Vec3::new(0.12, 0.12, 0.2), a, lin(BRASS), SHADE_BRASS);
        }
        // The wings either side of the back, the rolled arms.
        for s in [-1.0f32, 1.0] {
            cube(out, Vec3::new(x + s * (w / 2.0 - 0.12), 2.1, back_z + 0.8), Vec3::new(0.24, 2.4, 1.2), s * 0.15, wood, SHADE_WOOD);
            cube(
                out,
                Vec3::new(x + s * (w / 2.0 - 0.25), 2.1, back_z + 0.85),
                Vec3::new(0.06, 2.1, 1.0),
                s * 0.15,
                lin(VELVET_D),
                SHADE_CLOTH,
            );
            let arm = Vec3::new(x + s * (w / 2.0 - 0.15), 1.15, mid_z + 0.2);
            cube(out, arm, Vec3::new(0.32, 0.6, depth - 0.7), 0.0, wood, SHADE_WOOD);
            cube(out, arm + Vec3::new(0.0, 0.34, 0.0), Vec3::new(0.4, 0.14, depth - 0.6), 0.0, lin(VELVET_D), SHADE_CLOTH);
            cube(out, arm + Vec3::new(0.0, 0.2, (depth - 0.6) / 2.0), Vec3::new(0.42, 0.42, 0.2), 0.0, lin(TIMBER_L), SHADE_WOOD);
        }
        // A cobweb from the crest to the wall, a slipper fallen by the foot.
        cobweb(out, crest + Vec3::new(w * 0.35, 0.1, -0.1), Vec3::X, Vec3::NEG_Y, 0.7);
        cube(out, Vec3::new(x + 1.6, 0.06, front_z + 0.35), Vec3::new(0.32, 0.12, 0.58), 0.5, lin(VELVET_D), SHADE_CLOTH);
    }

    /// The giant hourglass's frame on the floor before the chair: a stepped plinth, brass plates
    /// top and bottom, four turned posts. (Its glass and its stopped sand are drawn each frame.)
    fn build_hourglass_frame(&mut self, base: Vec3) {
        let out = &mut self.static_cubes;
        cube(out, base + Vec3::new(0.0, 0.1, 0.0), Vec3::new(1.5, 0.2, 1.5), 0.0, lin(TIMBER_D), SHADE_WOOD);
        cube(out, base + Vec3::new(0.0, 0.25, 0.0), Vec3::new(1.3, 0.1, 1.3), PI / 4.0, lin(TIMBER), SHADE_WOOD);
        let plate = |out: &mut Vec<Instance>, y: f32| {
            cube(out, base + Vec3::new(0.0, y, 0.0), Vec3::new(1.5, 0.12, 1.5), 0.0, lin(BRASS), SHADE_BRASS);
            cube(out, base + Vec3::new(0.0, y, 0.0), Vec3::new(1.5, 0.16, 1.5), PI / 4.0, lin(BRASS_D), SHADE_BRASS);
        };
        plate(out, 0.38);
        plate(out, GLASS_H + 0.38);
        for k in 0..4 {
            let a = PI / 4.0 + k as f32 * PI / 2.0;
            let p = base + Vec3::new(a.cos() * 0.62, 0.0, a.sin() * 0.62);
            bar(out, p + Vec3::new(0.0, 0.4, 0.0), p + Vec3::new(0.0, GLASS_H + 0.36, 0.0), 0.1, lin(TIMBER_L), SHADE_WOOD);
            for y in [0.8f32, 1.6, 2.4, 3.0] {
                cube(out, p + Vec3::new(0.0, y, 0.0), Vec3::new(0.15, 0.08, 0.15), a, lin(BRASS_D), SHADE_BRASS);
            }
        }
        // A finial on top: a brass ball and spike.
        cube(out, base + Vec3::new(0.0, GLASS_H + 0.62, 0.0), Vec3::new(0.26, 0.26, 0.26), PI / 4.0, lin(BRASS), SHADE_BRASS);
        cube(out, base + Vec3::new(0.0, GLASS_H + 0.9, 0.0), Vec3::new(0.06, 0.34, 0.06), 0.0, lin(BRASS), SHADE_BRASS);
    }

    /// The canopied bed in the north-east corner: dark posts, grey linen, a slate coverlet turned
    /// back, bolsters, a sagging canopy with its curtains, cobwebs on it (no one has slept in it).
    fn build_bed(&mut self) {
        let out = &mut self.static_cubes;
        let mid = Vec3::new((BED.0 + BED.2) as f32 / 2.0, 0.0, (BED.1 + BED.3) as f32 / 2.0);
        let (w, d) = ((BED.2 - BED.0 + 1) as f32 - 0.15, (BED.3 - BED.1 + 1) as f32 - 0.15);
        for (sx, sz) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let p = mid + Vec3::new(sx * (w / 2.0 - 0.1), 0.0, sz * (d / 2.0 - 0.1));
            cube(out, p + Vec3::new(0.0, 1.4, 0.0), Vec3::new(0.14, 2.8, 0.14), 0.0, lin(TIMBER_D), SHADE_WOOD);
            cube(out, p + Vec3::new(0.0, 2.85, 0.0), Vec3::new(0.2, 0.12, 0.2), PI / 4.0, lin(TIMBER_L), SHADE_WOOD);
        }
        cube(out, mid + Vec3::new(0.0, 0.42, 0.0), Vec3::new(w - 0.1, 0.24, d - 0.1), 0.0, lin(TIMBER), SHADE_WOOD);
        cube(out, mid + Vec3::new(0.0, 0.66, 0.0), Vec3::new(w - 0.3, 0.24, d - 0.3), 0.0, lin(LINEN_D), SHADE_CLOTH);
        cube(out, mid + Vec3::new(0.0, 1.0, -d / 2.0 + 0.1), Vec3::new(w - 0.2, 0.9, 0.12), 0.0, lin(TIMBER), SHADE_WOOD);
        for s in [-0.5f32, 0.5] {
            cube(out, mid + Vec3::new(s * (w - 0.8) * 0.5, 0.86, -d / 2.0 + 0.5), Vec3::new(0.8, 0.22, 0.42), 0.0, lin(LINEN), SHADE_CLOTH);
        }
        cube(out, mid + Vec3::new(0.0, 0.81, 0.35), Vec3::new(w - 0.25, 0.08, d - 1.1), 0.0, lin(COVER), SHADE_CLOTH);
        cube(out, mid + Vec3::new(0.0, 0.86, -0.25), Vec3::new(w - 0.25, 0.08, 0.3), 0.0, lin(LINEN), SHADE_CLOTH);
        // The canopy, sagging in the middle, and its curtains hanging in folds at the corners.
        cube(out, mid + Vec3::new(0.0, 2.84, 0.0), Vec3::new(w, 0.08, d), 0.0, lin(VELVET_D), SHADE_CLOTH);
        cube(out, mid + Vec3::new(0.0, 2.74, 0.0), Vec3::new(w * 0.6, 0.06, d * 0.6), 0.0, lin(VELVET_D) * 0.85, SHADE_CLOTH);
        for (sx, sz) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            for k in 0..3 {
                let p = mid + Vec3::new(sx * (w / 2.0 - 0.25 - k as f32 * 0.12), 1.6, sz * (d / 2.0 - 0.05));
                cube(out, p, Vec3::new(0.12, 2.3, 0.08), k as f32 * 0.3, lin(VELVET) * (0.9 + 0.08 * k as f32), SHADE_CLOTH);
            }
        }
        cobweb(out, mid + Vec3::new(w / 2.0 - 0.1, 2.78, d / 2.0 - 0.1), Vec3::NEG_X, Vec3::NEG_Y, 0.6);
        cobweb(out, mid + Vec3::new(-w / 2.0 + 0.1, 2.78, d / 2.0 - 0.1), Vec3::X, Vec3::NEG_Y, 0.5);
    }

    /// The drift of sand run out across the south-west floor: low mounds, ripples, and a small
    /// hourglass lying broken in it.
    fn build_drift(&mut self) {
        let out = &mut self.static_cubes;
        let (cx, cz, r) = (DRIFT.0 as f32, DRIFT.1 as f32, DRIFT.2 as f32);
        for k in 0..28 {
            let a = h2(k as f32, 11.0) * TAU;
            let d = h2(k as f32, 13.0).sqrt() * r;
            let p = Vec3::new(cx + a.cos() * d, 0.0, cz + a.sin() * d * 0.8);
            let big = (1.0 - d / r).max(0.15);
            let col = [SAND, SAND_D, SAND_L][k % 3];
            for layer in 0..3 {
                let s = big * (1.6 - layer as f32 * 0.45);
                let tall = big * 0.12;
                cube(
                    out,
                    p + Vec3::new(0.0, tall * (layer as f32 + 0.5) - 0.02, 0.0),
                    Vec3::new(s, tall, s * 0.8),
                    a + layer as f32 * 0.6,
                    lin(col),
                    SHADE_LIT,
                );
            }
        }
        // Ripples in the sand.
        for k in 0..10 {
            let p = Vec3::new(cx - 1.5 + k as f32 * 0.32, 0.1, cz - 0.6 + (k as f32 * 0.8).sin() * 0.4);
            cube(out, p, Vec3::new(0.05, 0.04, 1.0), 0.4, lin(SAND_D), SHADE_LIT);
        }
        // A small hourglass lying on its side, its glass broken, the sand that ran out of it.
        let at = Vec3::new(cx + 0.6, 0.2, cz - 0.4);
        let m = Mat4::from_translation(at) * Mat4::from_rotation_z(PI / 2.0) * Mat4::from_rotation_y(0.4);
        for y in [-0.3f32, 0.3] {
            out.push(Instance::new(
                m * Mat4::from_translation(Vec3::new(0.0, y, 0.0)) * Mat4::from_scale(Vec3::new(0.34, 0.05, 0.34)),
                lin(BRASS),
                1.0,
                Vec3::ZERO,
                SHADE_BRASS,
            ));
        }
        for k in 0..3 {
            let a = k as f32 * TAU / 3.0;
            out.push(Instance::new(
                m * Mat4::from_translation(Vec3::new(a.cos() * 0.13, 0.0, a.sin() * 0.13)) * Mat4::from_scale(Vec3::new(0.025, 0.6, 0.025)),
                lin(BRASS_D),
                1.0,
                Vec3::ZERO,
                SHADE_BRASS,
            ));
        }
    }

    /// Each frame: the candles and the moonlight, the fog drifting, the clocks (stopped while he
    /// lives, going again once he has fallen), the hourglass's glass and sand, Belphegor's light
    /// and his sleep over him, the nightmares' glow, and the gate.
    pub(super) fn emit_sloth_court(&self, s: &SceneState, time: f32, hero: Vec3, f: &mut Frame) {
        let Some(cv) = &self.clocks else { return };
        f.bounds = Vec4::new(-0.5, -0.5, s.grid.width as f32 - 0.5, s.grid.height as f32 - 0.5);
        let prince = s.enemies.iter().filter_map(|e| e.duel.as_deref()).find(|d| d.breed.kind == EnemyKind::Belphegor && d.alive());
        let going = prince.is_none();
        // The candles: small flames guttering, their glow; the nearest light the hall.
        let lit: Vec<Vec3> = cv.candles.iter().filter(|c| c.1).map(|c| c.0).collect();
        for (i, p) in lit.iter().enumerate() {
            let fi = i as f32;
            let flick = 0.8 + 0.2 * (time * 9.0 + fi * 2.1).sin() * (time * 3.3 + fi).cos();
            let m =
                Mat4::from_translation(*p + Vec3::new(0.0, 0.03 * flick, 0.0)) * Mat4::from_scale(Vec3::new(0.035, 0.08 * flick, 0.035));
            f.additive.push(Instance::new(m, lin(FLAME).lerp(lin(FLAME_TIP), 0.3), 0.9, Vec3::ZERO, SHADE_UNLIT));
            glow_orb(f, *p + Vec3::new(0.0, 0.03, 0.0), 0.32, lin(CANDLE), 0.12 + 0.03 * flick);
        }
        for (i, p) in nearest(&lit, hero, 6).into_iter().enumerate() {
            let flick = 3.4 + (time * 7.0 + i as f32 * 1.7).sin() * 0.35 + (time * 17.0 + i as f32).sin() * 0.2;
            f.lights.push(Light { pos: p + Vec3::new(0.0, 0.15, 0.0), color: lin(CANDLE) * flick, distance: 6.5 });
        }
        // Moonlight through the north windows: the panes, a shaft of dust, a cold light on the floor.
        for (i, w) in cv.windows.iter().enumerate() {
            let pane = Mat4::from_translation(*w) * Mat4::from_scale(Vec3::new(0.5, 1.3, 0.02));
            f.additive.push(Instance::new(pane, lin(MOONLIGHT) * 1.4, 0.8, Vec3::ZERO, SHADE_UNLIT));
            for k in 0..3 {
                let along = 1.2 + k as f32 * 1.6;
                let p = *w + Vec3::new(0.0, -along * 0.55, along);
                let m = Mat4::from_translation(p) * Mat4::from_rotation_x(-0.5) * Mat4::from_scale(Vec3::new(0.55, 1.6, 0.05));
                f.additive.push(Instance::new(
                    m,
                    lin(MOONLIGHT) * 0.5,
                    0.06 + 0.02 * ((time * 0.4 + i as f32 + k as f32).sin() + 1.0),
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
            if i % 2 == 0 {
                f.lights.push(Light { pos: *w + Vec3::new(0.0, -0.8, 3.0), color: lin(MOONLIGHT) * 4.0, distance: 9.0 });
            }
        }
        // The fog drifting low over the boards: slow pale glows.
        for k in 0..14 {
            let fk = k as f32;
            let base = Vec3::new(4.0 + h2(fk, 1.0) * 24.0, 0.0, 4.0 + h2(fk, 2.0) * 24.0);
            let drift = Vec3::new((time * 0.07 + fk).sin() * 1.6, 0.0, (time * 0.05 + fk * 1.3).cos() * 1.2);
            glow_orb(f, base + drift + Vec3::new(0.0, 0.2, 0.0), 1.6, lin(MIST), 0.035);
        }
        // The clocks: their hands stopped, every one at its own hour, until he falls.
        for c in &cv.clocks {
            self.emit_clock(c, going, time, f);
        }
        // The giant hourglass: its glass, its sand stopped in mid-fall (running, once he has fallen).
        self.emit_giant_hourglass(cv.hourglass, going, time, f);
        // Belphegor: his own light, and his sleep over him.
        if let Some(d) = prince {
            let p = Vec3::new(d.pos.x as f32, 0.0, d.pos.y as f32);
            let ahead = Vec3::new(d.facing.x as f32, 0.0, d.facing.y as f32);
            f.lights.push(Light { pos: p + ahead * 2.4 + Vec3::new(0.0, 3.4, 0.0), color: lin(0xd8d0c0) * 5.0, distance: 7.0 });
            f.lights.push(Light { pos: p - ahead * 1.6 + Vec3::new(0.0, 3.0, 0.0), color: lin(SLATE) * 4.0, distance: 5.5 });
            emit_sleep(d.lore.slumber, p, ahead, time, f);
        }
        // The nightmares' cold violet light.
        for d in s.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.alive() && d.breed.kind == EnemyKind::Nightmare) {
            let p = Vec3::new(d.pos.x as f32, 1.2, d.pos.y as f32);
            f.lights.push(Light { pos: p, color: lin(DREAM) * 2.2, distance: 3.5 });
            glow_orb(f, p + Vec3::new(0.0, 0.2, 0.0), 0.7, lin(DREAM), 0.08 + 0.03 * (time * 3.0 + d.id as f32).sin());
        }
        let door = s.gate.map(|g| g.door).unwrap_or(0.0);
        self.emit_portal(&cv.gate, door, false, time, hero, f);
    }

    /// A clock's dial and hands (and pendulum): still at the hour it stopped, or going.
    fn emit_clock(&self, c: &ClockFace, going: bool, time: f32, f: &mut Frame) {
        let n = c.normal;
        let side = Vec3::new(n.z, 0.0, -n.x);
        let yaw = n.x.atan2(n.z);
        let dial = Mat4::from_translation(c.at)
            * Mat4::from_rotation_y(yaw)
            * Mat4::from_rotation_x(PI / 2.0)
            * Mat4::from_scale(Vec3::new(c.radius * 2.7, 0.2, c.radius * 2.7));
        f.opaque[MeshKind::Cylinder as usize].push(Instance::new(dial, lin(DIAL), 1.0, lin(DIAL) * 0.06, SHADE_LIT));
        let rim = Mat4::from_translation(c.at - n * 0.004)
            * Mat4::from_rotation_y(yaw)
            * Mat4::from_rotation_x(PI / 2.0)
            * Mat4::from_scale(Vec3::new(c.radius * 3.05, 0.15, c.radius * 3.05));
        f.opaque[MeshKind::Cylinder as usize].push(Instance::new(rim, lin(BRASS), 1.0, Vec3::ZERO, SHADE_BRASS));
        // The twelve hours, ticks round the dial.
        for k in 0..12 {
            let a = k as f32 * TAU / 12.0;
            let p = c.at + n * 0.015 + side * (a.sin() * c.radius * 0.82) + Vec3::Y * (a.cos() * c.radius * 0.82);
            let m = Mat4::from_translation(p) * Mat4::from_rotation_y(yaw) * Mat4::from_scale(Vec3::splat(c.radius * 0.1));
            f.opaque[MeshKind::Cube as usize].push(Instance::new(m, lin(HAND), 1.0, Vec3::ZERO, SHADE_LIT));
        }
        let (hour, minute) = if going { (c.stopped + time * 0.08, c.stopped * 12.0 + time * 1.0) } else { (c.stopped, c.stopped * 12.0) };
        for (a, len, thick) in [(hour, 0.5, 0.09), (minute, 0.78, 0.06)] {
            let dir = side * a.sin() + Vec3::Y * a.cos();
            let p = c.at + n * 0.03 + dir * (c.radius * len * 0.5);
            let rot = Quat::from_rotation_arc(Vec3::Y, dir);
            let m = Mat4::from_scale_rotation_translation(Vec3::new(c.radius * thick, c.radius * len, c.radius * 0.05), rot, p);
            f.opaque[MeshKind::Cube as usize].push(Instance::new(m, lin(HAND), 1.0, Vec3::ZERO, SHADE_LIT));
        }
        if let Some(pivot) = c.pendulum {
            let swing = if going { (time * 3.1).sin() * 0.22 } else { 0.0 };
            let dir = -Vec3::Y * swing.cos() + side * swing.sin();
            let bob = pivot + dir * 0.55;
            f.opaque[MeshKind::Cube as usize].push(Instance::new(segment(pivot, bob, 0.02), lin(BRASS_D), 1.0, Vec3::ZERO, SHADE_BRASS));
            let m = Mat4::from_translation(bob)
                * Mat4::from_rotation_y(yaw)
                * Mat4::from_rotation_x(PI / 2.0)
                * Mat4::from_scale(Vec3::new(0.42, 0.25, 0.42));
            f.opaque[MeshKind::Cylinder as usize].push(Instance::new(m, lin(BRASS), 1.0, lin(BRASS) * 0.1, SHADE_BRASS));
        }
    }

    /// The giant hourglass's glass and sand: the lower bulb heaped, a little left in the upper,
    /// and a stream of grains hanging motionless between them, until he falls and it runs.
    fn emit_giant_hourglass(&self, base: Vec3, going: bool, time: f32, f: &mut Frame) {
        let mid = base + Vec3::new(0.0, 0.38 + GLASS_H / 2.0, 0.0);
        let upper = mid + Vec3::new(0.0, GLASS_H * 0.24, 0.0);
        let lower = mid - Vec3::new(0.0, GLASS_H * 0.24, 0.0);
        let bulb = Vec3::new(BULB, GLASS_H * 0.26, BULB);
        // The sand: a heap in the lower bulb, a cone left in the upper's neck.
        let heap = Mat4::from_translation(lower - Vec3::new(0.0, bulb.y * 0.5, 0.0))
            * Mat4::from_scale(Vec3::new(BULB * 0.86, bulb.y * 0.5, BULB * 0.86));
        f.opaque[MeshKind::Sphere as usize].push(Instance::new(heap, lin(SAND), 1.0, Vec3::ZERO, SHADE_LIT));
        let left = Mat4::from_translation(upper - Vec3::new(0.0, bulb.y * 0.7, 0.0))
            * Mat4::from_scale(Vec3::new(BULB * 0.45, bulb.y * 0.25, BULB * 0.45));
        f.opaque[MeshKind::Sphere as usize].push(Instance::new(left, lin(SAND_D), 1.0, Vec3::ZERO, SHADE_LIT));
        // The grains between: hanging still, or falling.
        let top = upper.y - bulb.y * 0.85;
        let bottom = lower.y + bulb.y * 0.1;
        for k in 0..14 {
            let u = (k as f32 / 14.0 + if going { time * 1.4 } else { 0.0 }).fract();
            let y = top - (top - bottom) * u;
            let wob = (k as f32 * 2.3).sin() * 0.015;
            let m = Mat4::from_translation(Vec3::new(mid.x + wob, y, mid.z + wob)) * Mat4::from_scale(Vec3::splat(0.035));
            f.opaque[MeshKind::Cube as usize].push(Instance::new(m, lin(SAND_L), 1.0, lin(SAND) * 0.15, SHADE_LIT));
        }
        // The glass over it all, catching the light.
        for c in [upper, lower] {
            let m = Mat4::from_translation(c) * Mat4::from_scale(bulb);
            f.transparent[MeshKind::Sphere as usize].push(Instance::new(m, lin(GLASS) * 0.6, 0.14, lin(GLASS) * 0.02, SHADE_LIT));
        }
        let neck = Mat4::from_translation(mid) * Mat4::from_scale(Vec3::new(0.4, 4.0, 0.4));
        f.transparent[MeshKind::Cylinder as usize].push(Instance::new(neck, lin(GLASS), 0.25, Vec3::ZERO, SHADE_LIT));
        glow_orb(f, upper + Vec3::new(0.3, 0.4, 0.4), 0.25, lin(MOONLIGHT), 0.05);
    }

    /// The sand of hours on the floor (`prince::Trap`): falling in a stream from high above, then
    /// lying in a heap as a ring of sand grows round it (the tell), and bursting in a blast of sand.
    pub(super) fn emit_sand(&self, traps: &[Trap], grid: &Grid, time: f32, f: &mut Frame) {
        for t in traps {
            let ground = floor_at(grid, t.pos.x as f32, t.pos.y as f32);
            let at = Vec3::new(t.pos.x as f32, ground, t.pos.y as f32);
            let seed = (t.pos.x * 3.7 + t.pos.y * 1.9) as f32;
            if let Some(b) = t.burst {
                if b >= SPIKES {
                    continue;
                }
                // A blast of sand thrown up out of the heap, and settling.
                let u = b as f32 / SPIKES as f32;
                for k in 0..12usize {
                    let a = k as f32 * 2.39 + seed;
                    let r = t.radius as f32 * (0.2 + 0.8 * u) * (0.5 + 0.5 * ((k * 37 % 10) as f32 / 10.0));
                    let up = (1.0 - u) * (0.4 + 0.9 * ((k * 13 % 7) as f32 / 7.0)) * (u * PI).sin().max(0.2);
                    let p = at + Vec3::new(a.cos() * r, 0.1 + up, a.sin() * r);
                    let m = Mat4::from_translation(p) * Mat4::from_rotation_y(a) * Mat4::from_scale(Vec3::splat(0.12 * (1.0 - u * 0.6)));
                    f.opaque[MeshKind::Cube as usize].push(Instance::new(
                        m,
                        lin([SAND, SAND_D, SAND_L][k % 3]),
                        1.0,
                        Vec3::ZERO,
                        SHADE_LIT,
                    ));
                }
                let m = Mat4::from_translation(at + Vec3::new(0.0, 0.3 * (1.0 - u), 0.0))
                    * Mat4::from_scale(Vec3::new(t.radius as f32 * 1.1, 0.5 * (1.0 - u) + 0.05, t.radius as f32 * 1.1));
                f.transparent[MeshKind::Sphere as usize].push(Instance::new(m, lin(SAND_L), 0.3 * (1.0 - u), Vec3::ZERO, SHADE_UNLIT));
                continue;
            }
            let total = crate::logic::combat::prince::COIN_FLIGHT as f32 + 1.0;
            let landed = 1.0 - t.flight as f32 / total;
            // The stream of sand drifting down onto it from high above.
            let stream = if t.flight > 0 { landed } else { (1.0 - t.tell() * 1.4).max(0.0) };
            if stream > 0.0 {
                let tall = 4.5;
                for k in 0..10 {
                    let y = (k as f32 / 10.0 + time * 1.7 + seed).fract() * tall;
                    let p = at + Vec3::new((seed + k as f32).sin() * 0.06, y, (seed * 1.3 + k as f32).cos() * 0.06);
                    let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.05));
                    f.opaque[MeshKind::Cube as usize].push(Instance::new(m, lin(SAND_L), 1.0, lin(SAND) * 0.1, SHADE_LIT));
                }
                let m = Mat4::from_translation(at + Vec3::new(0.0, tall / 2.0, 0.0)) * Mat4::from_scale(Vec3::new(0.08, tall, 0.08));
                f.additive.push(Instance::new(m, lin(SAND_L) * 0.5, 0.25 * stream, Vec3::ZERO, SHADE_UNLIT));
            }
            // The heap growing on the floor.
            let grown = if t.flight > 0 { landed * 0.4 } else { 0.4 + 0.6 * t.tell() };
            let m = Mat4::from_translation(at) * Mat4::from_scale(Vec3::new(0.32 * grown + 0.05, 0.2 * grown, 0.32 * grown + 0.05));
            f.opaque[MeshKind::Sphere as usize].push(Instance::new(m, lin(SAND), 1.0, Vec3::ZERO, SHADE_LIT));
            if t.flight == 0 {
                // The tell: a ring of sand on the floor where it will burst, filling in from its edge.
                let fill = t.tell();
                let late = t.fuse <= 8;
                let blink = if late { 0.6 + (time * 40.0).sin().abs() * 0.8 } else { 0.0 };
                let r = t.radius as f32;
                let n = ((TAU * r) / 0.22).ceil() as i32;
                for k in 0..n {
                    let a = k as f32 * TAU / n as f32;
                    let p = at + Vec3::new(a.cos() * r, 0.03, a.sin() * r);
                    let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(0.08, 0.012, 0.08));
                    f.additive.push(Instance::new(m, lin(SAND_L) * (0.9 + blink), 0.7, Vec3::ZERO, SHADE_UNLIT));
                }
                let inner = r * (1.0 - fill);
                let m = Mat4::from_translation(at + Vec3::new(0.0, 0.025, 0.0))
                    * Mat4::from_rotation_x(-PI / 2.0)
                    * Mat4::from_scale(Vec3::new(r, r, 1.0));
                f.transparent[MeshKind::Disc as usize].push(Instance::new(
                    m,
                    lin(SAND_D) * (0.6 + blink * 0.6),
                    0.1 + 0.25 * fill,
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
                let m = Mat4::from_translation(at + Vec3::new(0.0, 0.03, 0.0))
                    * Mat4::from_rotation_x(-PI / 2.0)
                    * Mat4::from_scale(Vec3::new(inner + 0.05, inner + 0.05, 1.0));
                f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, lin(SAND_L) * 1.2, 0.5, Vec3::ZERO, SHADE_UNLIT));
            }
        }
    }

    /// Belphegor's pose over his gait: asleep (slumped in his chair, or down on the floor in his
    /// dream), breathing slow, his head lolling; or his moves as they wind up, strike and hang open.
    pub(super) fn pose_belphegor(&mut self, i: usize, d: &Duelist, time: f32) {
        let rig = self.models.rig(self.enemy_views[i].rig);
        let v = &mut self.enemy_views[i];
        let set = |v: &mut super::animation::EnemyView, name: &str, rot: Vec3| {
            if let Some(k) = rig.index(name) {
                v.pose.rot[k] = rot;
            }
        };
        let s = d.lore.slumber;
        let k = ease(s.depth());
        if k > 0.0 {
            let face = Vec3::new(d.facing.x as f32, 0.0, d.facing.y as f32).normalize_or_zero();
            let breathe = (time * 1.25 + i as f32).sin() * k;
            if s.chair {
                // Slumped back in his great chair, his legs out over its seat, his hands on its arms.
                v.group_pos -= face * (0.85 * k) + Vec3::new(0.0, 0.2 * k, 0.0);
                set(v, "leftLeg", Vec3::new(-1.2 * k, 0.0, -0.1 * k));
                set(v, "rightLeg", Vec3::new(-1.25 * k, 0.0, 0.12 * k));
                set(v, "leftArm", Vec3::new(-0.5 * k, 0.0, -0.3 * k));
                set(v, "rightArm", Vec3::new(-0.45 * k, 0.0, 0.32 * k));
                v.root_rot.x = -0.2 * k;
            } else {
                // Sat down on the floor where he stood, legs out before him, hands on the boards.
                v.group_pos.y -= 0.74 * k;
                set(v, "leftLeg", Vec3::new(-1.5 * k, 0.0, -0.18 * k));
                set(v, "rightLeg", Vec3::new(-1.45 * k, 0.0, 0.2 * k));
                set(v, "leftArm", Vec3::new(0.2 * k, 0.0, -0.42 * k));
                set(v, "rightArm", Vec3::new(0.15 * k, 0.0, 0.45 * k));
                v.root_rot.x = -0.26 * k;
            }
            set(v, "head", Vec3::new((0.45 + 0.04 * breathe) * k, 0.0, 0.24 * k));
            v.root_rot.z = 0.03 * k * (time * 0.5 + i as f32).sin();
            v.root_scale = Vec3::new(1.0 + 0.012 * breathe, 1.0 + 0.022 * breathe, 1.0 + 0.018 * breathe);
            return;
        }
        let Some((mv, _, frame)) = d.f.attack() else { return };
        let data = mv.data();
        let (st, ac, fr) = (data.startup as f32, data.active as f32, frame as f32);
        let (wind, strike, rec) = if fr < st {
            (ease(fr / st), 0.0, 0.0)
        } else if fr < st + ac {
            (1.0, ((fr - st + 1.0) / ac).min(1.0), 0.0)
        } else {
            (1.0, 1.0, ease((fr - st - ac) / data.recovery as f32))
        };
        let snap = 1.0 - (1.0 - strike).powi(3);
        let out = 1.0 - rec;
        match mv {
            MoveId::Swat => {
                // His great right hand drawn back and out, then swept across before him.
                let (rx, rz) = if strike == 0.0 { (0.55 * wind, 0.95 * wind) } else { (0.55 - 1.95 * snap, 0.95 - 1.45 * snap) };
                set(v, "rightArm", Vec3::new(rx * out, 0.0, rz * out));
                v.root_rot.y = if strike == 0.0 { -0.55 * wind } else { (-0.55 + 1.3 * snap) * out };
                v.root_rot.x = 0.1 * snap * out;
            }
            MoveId::Yawn => {
                // A vast stretch: arms up and out, his head thrown back, his chest swelling; then the
                // breath let out over him, leaning in.
                let up = if strike == 0.0 { wind } else { out };
                set(v, "leftArm", Vec3::new(-2.5 * up, 0.0, -0.7 * up));
                set(v, "rightArm", Vec3::new(-2.5 * up, 0.0, 0.7 * up));
                let head = if strike == 0.0 { -0.55 * wind } else { (-0.55 + 0.75 * snap) * out };
                set(v, "head", Vec3::new(head, 0.0, 0.0));
                let swell = if strike == 0.0 { wind } else { 1.0 - snap * 0.8 } * out;
                v.root_scale = Vec3::new(1.0 + 0.04 * swell, 1.0 + 0.03 * swell, 1.0 + 0.06 * swell);
                v.root_rot.x = if strike == 0.0 { -0.12 * wind } else { 0.14 * out };
            }
            MoveId::SandOfHours => {
                // His hourglass raised high in his left hand and tipped, shaken out over the floor.
                let shake = if strike > 0.0 && rec < 0.5 { (time * 30.0).sin() * 0.15 } else { 0.0 };
                set(v, "leftArm", Vec3::new(-2.7 * wind * out, 0.0, (-0.3 + shake) * wind * out));
                set(v, "head", Vec3::new(-0.3 * wind * out, 0.0, 0.0));
            }
            MoveId::HourglassSlam => {
                // The hourglass in both hands high over his head, leaning back; down like a hammer.
                let arms = if strike == 0.0 {
                    -3.0 * wind
                } else if rec == 0.0 {
                    -3.0 + 3.6 * snap
                } else {
                    0.6 * out
                };
                set(v, "leftArm", Vec3::new(arms, 0.0, 0.25 * wind * out));
                set(v, "rightArm", Vec3::new(arms, 0.0, -0.25 * wind * out));
                v.root_rot.x = if strike == 0.0 { -0.18 * wind } else { 0.3 * out };
                v.root_scale = Vec3::new(1.0, 1.0 - 0.06 * snap * out, 1.0);
            }
            MoveId::ClockHands => {
                // Arms out wide like a clock's hands, wound back, then swept all the way round.
                let wide = if strike == 0.0 { wind } else { out };
                set(v, "leftArm", Vec3::new(-0.3 * wide, 0.0, -1.45 * wide));
                set(v, "rightArm", Vec3::new(-0.3 * wide, 0.0, 1.45 * wide));
                v.root_rot.y = if strike == 0.0 { -1.1 * wind } else { -1.1 + (TAU + 1.1) * snap };
            }
            _ => {}
        }
    }
}

fn ease(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/// What shows of his sleep over him: Zs drifting up from his head while he sleeps deep, and in
/// his dream (on the floor) a slow violet swirl of dream-mist round him.
fn emit_sleep(s: Slumber, at: Vec3, ahead: Vec3, time: f32, f: &mut Frame) {
    let depth = s.depth();
    if depth < 0.5 {
        return;
    }
    let head = if s.chair { at - ahead * 0.85 + Vec3::new(0.0, 2.75, 0.0) } else { at + Vec3::new(0.0, 2.2, 0.0) + ahead * 0.3 };
    for k in 0..3 {
        let u = (time * 0.35 + k as f32 / 3.0).fract();
        let size = 0.12 + 0.18 * u;
        let p = head + Vec3::new(0.3 + u * 0.5 + (u * 6.0).sin() * 0.1, u * 1.4, 0.0);
        let alpha = (1.0 - u) * depth * 0.9;
        let c = lin(ZZZ) * 1.8;
        // A Z: its top, its slash, its foot.
        for (dy, w, rot) in [(size, size, 0.0f32), (0.0, size * 1.35, -0.78), (-size, size, 0.0)] {
            let m = Mat4::from_translation(p + Vec3::new(0.0, dy * 0.5, 0.0))
                * Mat4::from_rotation_z(rot)
                * Mat4::from_scale(Vec3::new(w, size * 0.16, 0.03));
            f.additive.push(Instance::new(m, c, alpha, Vec3::ZERO, SHADE_UNLIT));
        }
    }
    if !s.chair {
        for k in 0..6 {
            let a = time * 0.6 + k as f32 * TAU / 6.0;
            let r = 1.3 + 0.2 * (time * 0.9 + k as f32).sin();
            let p = at + Vec3::new(a.cos() * r, 0.5 + 0.3 * (time * 0.7 + k as f32 * 1.7).sin(), a.sin() * r);
            glow_orb(f, p, 0.45, lin(DREAM), 0.07 * depth);
        }
        f.lights.push(Light { pos: at + Vec3::new(0.0, 1.0, 0.0), color: lin(DREAM) * 2.5 * depth, distance: 4.0 });
    }
}
