//! Pride's court, **the Throne of the Morning Star** (`logic::court::pride` lays it out), where
//! Lucifer waits: its art, and his own on the floor (his wings, his light, his lance, his stars).
//!
//! A round hall of white marble veined with gold, polished to a mirror: a morning star of gold
//! inlaid at its heart in rings of gold and lapis, a lapis way from the gate to the steps. Its
//! wall is white marble between gilt pilasters under a gold cornice, and six tall mirrors in gilt
//! frames stand round it, each crowned with a sunburst. In the north, white steps edged in gold
//! rise to an empty throne whose back is the seventh mirror, before a high apse in which a great
//! window shaped like a star burns with the light outside; shafts of it fall across the hall to
//! pools on the floor, motes turning in them. Gold sconces of white fire hang between the mirrors.
//! The light is white and gold, cold blue in the glass. The gate he came in by is Pride's portal
//! in the south wall.
//!
//! Lucifer himself is dressed here too (`dress_lucifer`, from the duel view): his six wings, white
//! or burnt, how he moves for each of his moves, his light; and drawn here: his halo's glow and
//! his likeness in the polished floor, the Lance of Dawn blazing along its line, the mirror he will
//! step out of shining, the shadow of his fall, the smoke of his burnt wings, his falling stars.

use super::vault::{SHADE_GOLD, SHADE_MARBLE};
use super::*;
use crate::logic::combat::prince::pride::FALL_RISE;
use crate::logic::combat::prince::{Trap, COIN_FLIGHT};
use crate::logic::combat::{Duelist, MoveId, Phase, State};
use crate::logic::court::pride::{self as plan, Piece, CENTRE, MIRRORS, SHAFTS, STEPS, THRONE, WINDOW};
use crate::logic::hub::Gate;
use crate::render::foes_court::pride::{BURNT_WINGS, LUCIFER_VOXEL, REFLECTION_VOXEL, WHITE_WINGS};

pub(super) mod pal {
    pub const MARBLE: [u32; 4] = [0xe9e4da, 0xf1ede5, 0xdfd9ce, 0xf5f2ec];
    pub const WALL: u32 = 0xe6e0d4;
    pub const WALL_D: u32 = 0xccc4b6;
    pub const GOLD: u32 = 0xe8c060;
    pub const GOLD_L: u32 = 0xffe8a0;
    pub const GOLD_D: u32 = 0xb08a38;
    pub const LAPIS: u32 = 0x26386e;
    pub const GLASS: u32 = 0xc4d2e2;
    pub const GLASS_D: u32 = 0x8a9cb4;
    pub const LIGHT: u32 = 0xfff2c8;
    pub const SKY: u32 = 0xcfe0ff;
    pub const SILVER: u32 = 0xd8e4ff;
    pub const EMBER: u32 = 0xff7a2a;
    pub const SMOKE: u32 = 0x3a3634;
}
use pal::*;

/// The wall stands this tall; the apse behind the throne, and the window's middle, higher.
const WALL_H: f32 = 3.6;
const APSE_H: f32 = 8.6;
const WINDOW_Y: f32 = 6.0;
/// The height of the steps' top, where the throne stands.
const STEP: f32 = 0.3;
/// The sconces' height, and how far from the middle they hang (on the wall's face).
const SCONCE_Y: f32 = 2.6;
const SCONCE_R: f32 = 14.55;
/// Where round the wall (degrees from north, clockwise) the sconces hang: between the mirrors.
const SCONCES: [f32; 8] = [28.0, -28.0, 72.0, -72.0, 117.0, -117.0, 160.0, -160.0];

/// White, gold and the cold of the glass: the window's light falls from the north, high, and the
/// marble gives it back.
pub fn pride_atmosphere() -> Atmosphere {
    Atmosphere {
        background: 0x0b0a12,
        fog_near: 20.0,
        fog_far: 54.0,
        hemi_sky: lin(SKY) * 0.55,
        hemi_ground: lin(0xe0d0b0) * 0.45,
        ambient: lin(0xb8b0a0) * 0.5,
        sun_dir: Vec3::new(0.12, 1.0, -0.6).normalize(),
        sun: lin(0xfff0d0) * 1.15,
        exposure: 1.15,
        night_sky: 0.0,
        bloom: 0.8,
        moon_view: Vec3::new(0.0, 0.3, -1.0).normalize(),
        cloud_tint: lin(0x14121c),
    }
}

/// Its ground haze: density and falloff (a little, for the shafts of light to stand in).
pub const HAZE: (f32, f32) = (0.014, 2.6);

/// What the view keeps of the court for its moving parts: each mirror (where its glass is and
/// which way it faces; the throne's last), the sconces, the gate, the throne and the window.
#[derive(Clone, Debug)]
pub struct ThroneView {
    pub mirrors: Vec<(Vec3, Vec3)>,
    pub sconces: Vec<Vec3>,
    pub gate: Gate,
    pub throne: Vec3,
    pub window: Vec3,
}

fn cube(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot_y: f32, color: Vec3, shade: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(rot_y), pos), color, 1.0, Vec3::ZERO, shade));
}

/// A box set in a frame at `origin` turned `yaw` (its +z out of the frame's face): at `at` in the
/// frame, turned `roll` in the face's plane.
#[allow(clippy::too_many_arguments)]
fn framed(out: &mut Vec<Instance>, origin: Vec3, yaw: f32, at: Vec3, scale: Vec3, roll: f32, color: Vec3, shade: f32) {
    let m = Mat4::from_translation(origin)
        * Mat4::from_rotation_y(yaw)
        * Mat4::from_translation(at)
        * Mat4::from_rotation_z(roll)
        * Mat4::from_scale(scale);
    out.push(Instance::new(m, color, 1.0, Vec3::ZERO, shade));
}

fn marble(h: f64) -> Vec3 {
    lin(MARBLE[(h * 4.0) as usize % 4]) * (0.96 + h as f32 * 0.06)
}

/// A ring of gold (or lapis) inlay on the floor about the middle: `r` out, `wide` across.
fn inlay_ring(out: &mut Vec<Instance>, c: Vec3, r: f32, wide: f32, color: Vec3, lift: f32) {
    let n = (TAU * r / 0.3).round().max(8.0) as i32;
    for k in 0..n {
        let a = k as f32 * TAU / n as f32;
        let p = c + Vec3::new(a.cos() * r, lift, a.sin() * r);
        cube(
            out,
            p,
            Vec3::new(TAU * r / n as f32 + 0.02, 0.012, wide),
            -a + PI / 2.0,
            color,
            if color.x > color.z { SHADE_GOLD } else { SHADE_MARBLE },
        );
    }
}

/// The world point of a tile.
fn at(p: (i32, i32)) -> Vec3 {
    Vec3::new(p.0 as f32, 0.0, p.1 as f32)
}

impl WorldView {
    /// The court's static scene, as the level loads.
    pub(super) fn build_pride_court(&mut self, grid: &Grid) {
        let c = at(CENTRE);
        let gate_tile = (plan::GATE.0, plan::GATE.1);
        let floor_tile = |x: i32, y: i32| plan::in_hall(x, y);

        // ---- the floor: white marble polished to a mirror, veined with gold
        for y in 0..grid.height {
            for x in 0..grid.width {
                if !floor_tile(x, y) && (x, y) != gate_tile {
                    continue;
                }
                let (xf, yf) = (x as f32, y as f32);
                let h = hash(x as f64 * 1.7 + 3.0, y as f64 * 2.3);
                let out = &mut self.static_cubes;
                cube(out, Vec3::new(xf, -0.06, yf), Vec3::new(0.985, 0.12, 0.985), 0.0, marble(h), SHADE_MARBLE);
                let d = Vec2::new(xf - c.x, yf - c.z).length();
                if h > 0.82 && d > 4.0 {
                    // A vein of gold wandering across the slab.
                    let a = hash(x as f64 * 5.0, y as f64 * 7.0) as f32 * PI;
                    let o = Vec3::new(xf + (h as f32 - 0.86) * 0.6, 0.003, yf + (hash(y as f64, x as f64) as f32 - 0.5) * 0.5);
                    let dir = Vec3::new(a.cos(), 0.0, -a.sin());
                    for (k, bend) in [(-0.5f32, 0.3f32), (0.5, -0.25)] {
                        cube(out, o + dir * (k * 0.22), Vec3::new(0.22, 0.006, 0.01), a + bend, lin(GOLD), SHADE_GOLD);
                    }
                }
            }
        }
        // The inlay: rings of gold and lapis about the middle, sixteen rays of gold between them,
        // and the morning star at the heart.
        {
            let out = &mut self.static_cubes;
            for (r, wide, col) in
                [(3.2f32, 0.07f32, GOLD), (7.2, 0.5, LAPIS), (6.9, 0.06, GOLD), (7.5, 0.06, GOLD), (11.6, 0.08, GOLD), (12.0, 0.05, GOLD_D)]
            {
                inlay_ring(out, c, r, wide, lin(col), if col == LAPIS { 0.004 } else { 0.006 });
            }
            for k in 0..16 {
                let a = k as f32 * TAU / 16.0 + TAU / 32.0;
                let dir = Vec3::new(a.cos(), 0.0, a.sin());
                for (r0, r1) in [(3.3f32, 6.85f32), (7.55, 11.55)] {
                    let mid = c + dir * ((r0 + r1) / 2.0) + Vec3::new(0.0, 0.005, 0.0);
                    cube(out, mid, Vec3::new(r1 - r0, 0.01, 0.05), -a, lin(GOLD_D), SHADE_GOLD);
                }
            }
            // The star: eight rays, the long four to the quarters, tapering, white gold at the heart.
            for k in 0..8 {
                let a = k as f32 * TAU / 8.0 - PI / 2.0;
                let long = if k % 2 == 0 { 3.0 } else { 1.8 };
                let dir = Vec3::new(a.cos(), 0.0, a.sin());
                for s in 0..14 {
                    let t = (s as f32 + 0.5) / 14.0;
                    let w = 0.55 * (1.0 - t) + 0.04;
                    let col = if k % 2 == 0 { lin(GOLD) } else { lin(GOLD_D) };
                    cube(
                        out,
                        c + dir * (0.4 + t * long) + Vec3::new(0.0, 0.007, 0.0),
                        Vec3::new(long / 14.0 + 0.02, 0.012, w),
                        -a,
                        col,
                        SHADE_GOLD,
                    );
                }
            }
            for (r, col) in [(0.75f32, GOLD_L), (0.45, GOLD), (0.2, 0xfffbe8)] {
                let m = Mat4::from_translation(c + Vec3::new(0.0, 0.008 + (0.75 - r) * 0.01, 0.0))
                    * Mat4::from_scale(Vec3::new(r * 2.0, 0.014, r * 2.0));
                out.push(Instance::new(m, lin(col), 1.0, Vec3::ZERO, SHADE_GOLD));
            }
            // The way of lapis from the gate to the star, and on from the star to the steps.
            for (z0, z1) in [(c.z + 3.4, plan::GATE.1 as f32 - 0.5), (STEPS.3 as f32 + 0.5, c.z - 3.4)] {
                let n = ((z1 - z0) / 1.0).ceil() as i32;
                for k in 0..n {
                    let z = z0 + (k as f32 + 0.5) * (z1 - z0) / n as f32;
                    let len = (z1 - z0) / n as f32 + 0.01;
                    cube(out, Vec3::new(c.x, 0.004, z), Vec3::new(1.5, 0.012, len), 0.0, lin(LAPIS), SHADE_MARBLE);
                    for s in [-1.0f32, 1.0] {
                        cube(out, Vec3::new(c.x + s * 0.8, 0.006, z), Vec3::new(0.08, 0.012, len), 0.0, lin(GOLD), SHADE_GOLD);
                    }
                    if k % 2 == 0 {
                        let m = Mat4::from_translation(Vec3::new(c.x, 0.008, z))
                            * Mat4::from_rotation_y(PI / 4.0)
                            * Mat4::from_scale(Vec3::new(0.32, 0.012, 0.32));
                        out.push(Instance::new(m, lin(GOLD_L), 1.0, Vec3::ZERO, SHADE_GOLD));
                    }
                }
            }
        }

        // ---- the wall: white marble between gilt pilasters, a gold cornice
        for (x, y, t) in grid.cells() {
            if t != Tile::Rock || floor_tile(x, y) || (x, y) == gate_tile {
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
            let near = !open.is_empty() || [(-1, -1), (1, -1), (-1, 1), (1, 1)].iter().any(|&(dx, dy)| floor_tile(x + dx, y + dy));
            if !near || (y == plan::GATE.1 && (x - plan::GATE.0).abs() <= 1) {
                continue;
            }
            let start = self.static_cubes.len();
            let (xf, yf) = (x as f32, y as f32);
            let h = hash(x as f64 * 3.0, y as f64 * 5.0) as f32;
            let out = &mut self.static_cubes;
            cube(out, Vec3::new(xf, 0.15, yf), Vec3::new(1.0, 0.3, 1.0), 0.0, lin(WALL_D), SHADE_MARBLE);
            cube(
                out,
                Vec3::new(xf, WALL_H / 2.0 + 0.15, yf),
                Vec3::new(1.0, WALL_H - 0.3, 1.0),
                0.0,
                lin(WALL) * (0.95 + h * 0.08),
                SHADE_MARBLE,
            );
            cube(out, Vec3::new(xf, WALL_H - 0.05, yf), Vec3::new(1.08, 0.14, 1.08), 0.0, lin(GOLD), SHADE_GOLD);
            cube(out, Vec3::new(xf, WALL_H + 0.08, yf), Vec3::new(1.12, 0.12, 1.12), 0.0, lin(WALL_D), SHADE_MARBLE);
            cube(out, Vec3::new(xf, 0.32, yf), Vec3::new(1.04, 0.05, 1.04), 0.0, lin(GOLD_D), SHADE_GOLD);
            for &d in &open {
                let (dx, dy) = d.vec();
                let n = Vec3::new(dx as f32, 0.0, dy as f32);
                let along = if dx == 0 { Vec3::X } else { Vec3::Z };
                let face = Vec3::new(xf, 0.0, yf) + n * 0.5;
                let size = |a: f32, up: f32, out: f32| along * a + Vec3::Y * up + Vec3::new(n.x.abs(), 0.0, n.z.abs()) * out;
                // A tall sunk panel framed in gold, and a frieze of gold under the cornice.
                cube(out, face + Vec3::new(0.0, 1.75, 0.0) + n * 0.005, size(0.72, 2.3, 0.01), 0.0, lin(WALL_D), SHADE_MARBLE);
                for (yy, hh) in [(0.6f32, 0.035f32), (2.9, 0.035), (3.25, 0.08)] {
                    cube(
                        out,
                        face + Vec3::new(0.0, yy, 0.0) + n * 0.012,
                        size(if yy > 3.0 { 1.0 } else { 0.76 }, hh, 0.02),
                        0.0,
                        lin(GOLD_D),
                        SHADE_GOLD,
                    );
                }
                for s in [-1.0f32, 1.0] {
                    cube(
                        out,
                        face + along * (s * 0.37) + Vec3::new(0.0, 1.75, 0.0) + n * 0.012,
                        size(0.03, 2.33, 0.02),
                        0.0,
                        lin(GOLD_D),
                        SHADE_GOLD,
                    );
                }
                if (x + y) % 2 == 0 {
                    // A pilaster, its capital and base gilt.
                    cube(
                        out,
                        face + along * 0.5 + Vec3::new(0.0, WALL_H / 2.0, 0.0) + n * 0.07,
                        size(0.2, WALL_H, 0.14),
                        0.0,
                        lin(MARBLE[3]),
                        SHADE_MARBLE,
                    );
                    for (yy, hh) in [(0.36f32, 0.12f32), (WALL_H - 0.35, 0.18)] {
                        cube(
                            out,
                            face + along * 0.5 + Vec3::new(0.0, yy, 0.0) + n * 0.08,
                            size(0.27, hh, 0.17),
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

        // ---- the mirrors round the wall, and the steps
        let mut mirrors = Vec::new();
        for (x, y, _) in grid.cells() {
            let Some(piece) = plan::piece(x, y) else { continue };
            match piece {
                Piece::Mirror => {
                    let start = self.static_cubes.len();
                    let face = (c - at((x, y))).normalize();
                    let glass = self.build_pride_mirror(at((x, y)) - face * 0.22, face);
                    mirrors.push((glass, face));
                    let len = self.static_cubes.len() - start;
                    self.rock_index.insert(tile_key(x, y), (start, len));
                    let shades: Vec<f32> = self.static_cubes[start..start + len].iter().map(|i| i.emissive[3]).collect();
                    self.trees.push(Tree {
                        range: (start, len),
                        base: at((x, y)),
                        radius: 0.6,
                        height: 3.4,
                        fade: 1.0,
                        alive: true,
                        shades: Some(shades),
                    });
                }
                Piece::Steps => self.build_pride_step(x, y),
            }
        }
        // (In the order of the plan's mirrors, as the doors before them are.)
        mirrors.sort_by(|a, b| {
            let k = |m: &(Vec3, Vec3)| MIRRORS.iter().position(|&p| (at(p) - m.0).length() < 1.0).unwrap_or(99);
            k(a).cmp(&k(b))
        });

        // ---- the apse, the star window in it, and the empty throne before it
        let window = self.build_pride_apse();
        let throne = self.build_pride_throne();
        mirrors.push((throne + Vec3::new(0.0, 1.95, -0.46), Vec3::Z));

        // ---- the sconces between the mirrors
        let mut sconces = Vec::new();
        for b in SCONCES {
            let a = b.to_radians();
            let dir = Vec3::new(a.sin(), 0.0, -a.cos());
            let p = c + dir * SCONCE_R + Vec3::new(0.0, SCONCE_Y, 0.0);
            let yaw = (-dir.x).atan2(-dir.z);
            let out = &mut self.static_cubes;
            framed(out, p, yaw, Vec3::new(0.0, -0.2, -0.2), Vec3::new(0.1, 0.1, 0.45), 0.0, lin(GOLD_D), SHADE_GOLD);
            framed(out, p, yaw, Vec3::new(0.0, -0.08, 0.0), Vec3::new(0.32, 0.08, 0.32), 0.0, lin(GOLD), SHADE_GOLD);
            framed(out, p, yaw, Vec3::new(0.0, -0.32, -0.42), Vec3::new(0.24, 0.5, 0.06), 0.0, lin(GOLD_D), SHADE_GOLD);
            sconces.push(p + Vec3::new(0.0, 0.1, 0.0));
        }

        // ---- the gate: Pride's portal in the south wall, looking into the hall
        let gate = Gate { tile: P::new(plan::GATE.0, plan::GATE.1), facing: Dir::Up, sin: Sin::Pride };
        self.build_court_portal(&gate);
        self.throne = Some(ThroneView { mirrors, sconces, gate, throne, window });
    }

    /// A tall mirror in a gilt frame on a plinth, crowned with a sunburst, its back to the wall at
    /// `base` and its glass toward `face`. Returns the glass's middle.
    fn build_pride_mirror(&mut self, base: Vec3, face: Vec3) -> Vec3 {
        let yaw = face.x.atan2(face.z);
        let out = &mut self.static_cubes;
        let f = |out: &mut Vec<Instance>, at: Vec3, s: Vec3, roll: f32, col: u32, shade: f32| {
            framed(out, base, yaw, at, s, roll, lin(col), shade)
        };
        f(out, Vec3::new(0.0, 0.14, 0.0), Vec3::new(1.7, 0.28, 0.75), 0.0, GOLD_D, SHADE_GOLD);
        f(out, Vec3::new(0.0, 0.3, 0.05), Vec3::new(1.55, 0.06, 0.62), 0.0, GOLD, SHADE_GOLD);
        f(out, Vec3::new(0.0, 1.75, -0.16), Vec3::new(1.5, 3.0, 0.2), 0.0, WALL_D, SHADE_MARBLE);
        // The glass, and a darker rim of its silvering.
        f(out, Vec3::new(0.0, 1.75, 0.0), Vec3::new(1.22, 2.66, 0.06), 0.0, GLASS, SHADE_MARBLE);
        f(out, Vec3::new(0.0, 1.75, -0.01), Vec3::new(1.3, 2.74, 0.06), 0.0, GLASS_D, SHADE_MARBLE);
        // The frame: its sides, head and foot, a bead of pale gold inside it.
        for s in [-1.0f32, 1.0] {
            f(out, Vec3::new(s * 0.7, 1.75, 0.02), Vec3::new(0.16, 3.0, 0.16), 0.0, GOLD, SHADE_GOLD);
            f(out, Vec3::new(s * 0.6, 1.75, 0.05), Vec3::new(0.035, 2.66, 0.04), 0.0, GOLD_L, SHADE_GOLD);
            f(out, Vec3::new(s * 0.7, 3.32, 0.04), Vec3::new(0.24, 0.24, 0.24), PI / 4.0, GOLD_L, SHADE_GOLD);
        }
        for (y, h) in [(0.4f32, 0.18f32), (3.12, 0.2)] {
            f(out, Vec3::new(0.0, y, 0.02), Vec3::new(1.56, h, 0.16), 0.0, GOLD, SHADE_GOLD);
        }
        // The sunburst over it: rays of gold about a disc.
        for k in 0..9 {
            let a = PI * (k as f32 / 8.0);
            let long = if k % 2 == 0 { 0.6 } else { 0.4 };
            let o = Vec3::new(a.cos() * (0.2 + long / 2.0), 3.3 + a.sin() * (0.2 + long / 2.0), 0.0);
            f(out, o, Vec3::new(long, 0.06, 0.05), a, if k % 2 == 0 { GOLD_L } else { GOLD }, SHADE_GOLD);
        }
        f(out, Vec3::new(0.0, 3.3, 0.02), Vec3::new(0.3, 0.3, 0.08), PI / 4.0, GOLD_L, SHADE_GOLD);
        let m = Mat4::from_translation(base) * Mat4::from_rotation_y(yaw);
        m.transform_point3(Vec3::new(0.0, 1.75, 0.05))
    }

    /// A tile of the steps up to the throne: white marble, higher toward the throne, each step's
    /// nose edged in gold, a gold post at each end of the lowest.
    fn build_pride_step(&mut self, x: i32, y: i32) {
        let level = (STEPS.3 - y + 1) as f32;
        let h = level * STEP;
        let (xf, yf) = (x as f32, y as f32);
        let out = &mut self.static_cubes;
        let hm = hash(x as f64 * 2.0, y as f64 * 3.0);
        cube(out, Vec3::new(xf, h / 2.0, yf), Vec3::new(1.0, h, 1.0), 0.0, marble(hm), SHADE_MARBLE);
        cube(out, Vec3::new(xf, h - 0.025, yf + 0.48), Vec3::new(1.0, 0.05, 0.06), 0.0, lin(GOLD), SHADE_GOLD);
        let edge = |xx: i32| plan::piece(xx, y) != Some(Piece::Steps);
        for s in [-1i32, 1] {
            if edge(x + s) {
                cube(out, Vec3::new(xf + s as f32 * 0.48, h / 2.0, yf), Vec3::new(0.06, h, 1.0), 0.0, lin(GOLD_D), SHADE_GOLD);
                if y == STEPS.3 {
                    // A post of gold at the foot of the steps, a ball of it on top.
                    let p = Vec3::new(xf + s as f32 * 0.3, 0.0, yf + 0.3);
                    cube(out, p + Vec3::new(0.0, 0.6, 0.0), Vec3::new(0.16, 1.2, 0.16), 0.0, lin(GOLD), SHADE_GOLD);
                    cube(out, p + Vec3::new(0.0, 1.25, 0.0), Vec3::new(0.24, 0.24, 0.24), PI / 4.0, lin(GOLD_L), SHADE_GOLD);
                }
            }
        }
    }

    /// The apse behind the throne, rising over the wall, and the great window shaped like a star in
    /// it. Returns the window's middle.
    fn build_pride_apse(&mut self) -> Vec3 {
        let w = Vec3::new(WINDOW.0 as f32, WINDOW_Y, WINDOW.1 as f32);
        let out = &mut self.static_cubes;
        let back = WINDOW.1 as f32 - 0.25;
        cube(out, Vec3::new(w.x, APSE_H / 2.0, back), Vec3::new(9.0, APSE_H, 0.9), 0.0, lin(WALL), SHADE_MARBLE);
        cube(out, Vec3::new(w.x, APSE_H + 0.1, back), Vec3::new(9.4, 0.22, 1.1), 0.0, lin(GOLD), SHADE_GOLD);
        for s in [-1.0f32, 1.0] {
            let x = w.x + s * 4.1;
            cube(out, Vec3::new(x, APSE_H / 2.0, back + 0.5), Vec3::new(0.5, APSE_H, 0.25), 0.0, lin(MARBLE[3]), SHADE_MARBLE);
            for y in [0.4f32, APSE_H - 0.4, WALL_H] {
                cube(out, Vec3::new(x, y, back + 0.55), Vec3::new(0.6, 0.18, 0.3), 0.0, lin(GOLD), SHADE_GOLD);
            }
        }
        // The star of glass: eight rays tapering from the heart, the long four to the quarters,
        // leaded in gold, in a ring of gold.
        let face = back + 0.47;
        for k in 0..8 {
            let a = k as f32 * TAU / 8.0 + PI / 2.0;
            let long = if k % 2 == 0 { 2.5 } else { 1.5 };
            for s in 0..5 {
                let t = (s as f32 + 0.5) / 5.0;
                let wide = 0.9 * (1.0 - t) + 0.08;
                let p = w + Vec3::new(a.cos() * (0.3 + t * long), a.sin() * (0.3 + t * long), 0.0);
                let m = Mat4::from_translation(Vec3::new(p.x, p.y, face))
                    * Mat4::from_rotation_z(a)
                    * Mat4::from_scale(Vec3::new(long / 5.0 + 0.04, wide, 0.04));
                out.push(Instance::new(m, lin(LIGHT) * 1.7, 1.0, lin(LIGHT) * 1.1, SHADE_UNLIT));
            }
            let tip = w + Vec3::new(a.cos() * (0.3 + long), a.sin() * (0.3 + long), 0.0);
            out.push(Instance::new(
                forge::segment(Vec3::new(w.x, w.y, face + 0.02), Vec3::new(tip.x, tip.y, face + 0.02), 0.05),
                lin(GOLD_D),
                1.0,
                Vec3::ZERO,
                SHADE_GOLD,
            ));
        }
        let m = Mat4::from_translation(Vec3::new(w.x, w.y, face + 0.01))
            * Mat4::from_rotation_z(PI / 4.0)
            * Mat4::from_scale(Vec3::new(0.75, 0.75, 0.05));
        out.push(Instance::new(m, Vec3::splat(2.2), 1.0, Vec3::splat(1.4), SHADE_UNLIT));
        let n = 40;
        for k in 0..n {
            let a = k as f32 * TAU / n as f32;
            let p = Vec3::new(w.x + a.cos() * 2.95, w.y + a.sin() * 2.95, face + 0.03);
            let m = Mat4::from_translation(p) * Mat4::from_rotation_z(a) * Mat4::from_scale(Vec3::new(0.12, 0.5, 0.1));
            out.push(Instance::new(m, lin(GOLD), 1.0, Vec3::ZERO, SHADE_GOLD));
        }
        Vec3::new(w.x, w.y, face)
    }

    /// The empty throne at the top of the steps: white marble and gold, its high back the seventh
    /// mirror. Returns its seat.
    fn build_pride_throne(&mut self) -> Vec3 {
        let top = (STEPS.3 - STEPS.1 + 1) as f32 * STEP;
        let seat = Vec3::new(THRONE.0 as f32, top, THRONE.1 as f32);
        let out = &mut self.static_cubes;
        cube(out, seat + Vec3::new(0.0, 0.28, 0.0), Vec3::new(1.8, 0.56, 1.2), 0.0, lin(MARBLE[1]), SHADE_MARBLE);
        cube(out, seat + Vec3::new(0.0, 0.57, 0.05), Vec3::new(1.6, 0.06, 1.05), 0.0, lin(GOLD), SHADE_GOLD);
        cube(out, seat + Vec3::new(0.0, 0.64, 0.1), Vec3::new(1.4, 0.1, 0.9), 0.0, lin(LAPIS), SHADE_MARBLE);
        for s in [-1.0f32, 1.0] {
            cube(out, seat + Vec3::new(s * 0.9, 0.85, 0.0), Vec3::new(0.22, 0.42, 1.2), 0.0, lin(GOLD), SHADE_GOLD);
            cube(out, seat + Vec3::new(s * 0.9, 1.1, 0.55), Vec3::new(0.3, 0.2, 0.3), PI / 4.0, lin(GOLD_L), SHADE_GOLD);
        }
        // Its back: a tall pane of glass in gold, a star over it.
        let b = seat + Vec3::new(0.0, 0.0, -0.5);
        cube(out, b + Vec3::new(0.0, 1.9, -0.06), Vec3::new(1.8, 3.2, 0.14), 0.0, lin(GOLD_D), SHADE_GOLD);
        cube(out, b + Vec3::new(0.0, 1.95, 0.02), Vec3::new(1.36, 2.6, 0.05), 0.0, lin(GLASS), SHADE_MARBLE);
        for s in [-1.0f32, 1.0] {
            cube(out, b + Vec3::new(s * 0.8, 1.9, 0.04), Vec3::new(0.16, 3.2, 0.16), 0.0, lin(GOLD), SHADE_GOLD);
        }
        cube(out, b + Vec3::new(0.0, 3.5, 0.04), Vec3::new(1.8, 0.18, 0.18), 0.0, lin(GOLD), SHADE_GOLD);
        for k in 0..8 {
            let a = k as f32 * TAU / 8.0;
            let long = if k % 2 == 0 { 0.55 } else { 0.32 };
            let m = Mat4::from_translation(b + Vec3::new(a.cos() * long / 2.0, 3.95 + a.sin() * long / 2.0, 0.05))
                * Mat4::from_rotation_z(a)
                * Mat4::from_scale(Vec3::new(long, 0.08, 0.06));
            out.push(Instance::new(m, lin(GOLD_L), 1.0, Vec3::ZERO, SHADE_GOLD));
        }
        seat
    }

    /// Each frame: the window's light and its shafts, the sconces, the glass's sheen (and the mirror
    /// he will step out of, shining), his light and his likeness in the floor, his lance, his fall,
    /// his burnt wings smoking, and the gate.
    pub(super) fn emit_pride_court(&self, s: &SceneState, time: f32, hero: Vec3, f: &mut Frame) {
        let Some(tv) = &self.throne else { return };
        f.bounds = Vec4::new(-0.5, -0.5, s.grid.width as f32 - 0.5, s.grid.height as f32 - 0.5);
        // ---- the window, burning with the light outside, and its shafts across the hall
        let breathe = 0.92 + 0.08 * (time * 0.7).sin();
        glow_orb(f, tv.window + Vec3::new(0.0, 0.0, 0.3), 2.6, lin(LIGHT), 0.3 * breathe);
        f.lights.push(Light { pos: tv.window + Vec3::new(0.0, -1.2, 2.2), color: lin(0xfff0d0) * 10.0 * breathe, distance: 15.0 });
        for (k, &(x, z, r)) in SHAFTS.iter().enumerate() {
            let foot = Vec3::new(x as f32, 0.0, z as f32);
            let top = tv.window + Vec3::new((foot.x - tv.window.x) * 0.12, 0.0, 0.4);
            let r = r as f32;
            for (wide, alpha) in [(r * 0.9, 0.018f32), (r * 0.5, 0.026)] {
                let mut m = forge::segment(top, foot + Vec3::new(0.0, 0.02, 0.0), 1.0);
                m *= Mat4::from_scale(Vec3::new(wide, 1.0, wide * 0.6));
                f.additive.push(Instance::new(m, lin(LIGHT) * 1.2, alpha * breathe, Vec3::ZERO, SHADE_UNLIT));
            }
            if k < 3 {
                // Where it falls, the marble lit.
                f.lights.push(Light { pos: foot + Vec3::new(0.0, 0.9, 0.0), color: lin(LIGHT) * 3.5 * breathe, distance: 3.2 + r });
            }
            // Motes turning in the light.
            for j in 0..6 {
                let u = (time * 0.05 + j as f32 * 0.167 + k as f32 * 0.31).fract();
                let p = foot.lerp(top, u)
                    + Vec3::new((time * 0.6 + j as f32 * 2.1).sin() * r * 0.3, 0.0, (time * 0.5 + j as f32).cos() * r * 0.2);
                let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.03));
                f.additive.push(Instance::new(m, lin(LIGHT) * 2.0, 0.6 * (u * PI).sin(), Vec3::ZERO, SHADE_UNLIT));
            }
        }
        // The empty throne in its own gold light.
        f.lights.push(Light { pos: tv.throne + Vec3::new(0.0, 2.2, 1.6), color: lin(GOLD_L) * 3.0, distance: 6.0 });
        // ---- the sconces' white fire; the nearest light the hall
        for (i, p) in tv.sconces.iter().enumerate() {
            let fi = i as f32;
            for k in 0..4 {
                let t = (time * 1.8 + fi * 0.37 + k as f32 * 0.25).fract();
                let q = *p + Vec3::new((fi + k as f32).cos() * 0.05 * (1.0 - t), t * 0.35, (fi * 1.3 + k as f32).sin() * 0.05 * (1.0 - t));
                let size = 0.1 * (1.0 - t) + 0.025;
                f.additive.push(Instance::new(
                    Mat4::from_translation(q) * Mat4::from_scale(Vec3::new(size, size * 1.6, size)),
                    lin(0xfffaf0).lerp(lin(GOLD_L), t),
                    0.7 * (1.0 - t),
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
            glow_orb(f, *p + Vec3::new(0.0, 0.12, 0.0), 0.45, lin(LIGHT), 0.25 + 0.03 * (time * 5.0 + fi).sin());
        }
        for (i, p) in nearest(&tv.sconces, hero, 6).into_iter().enumerate() {
            let flick = 4.5 + (time * 5.0 + i as f32).sin() * 0.3;
            f.lights.push(Light { pos: p + Vec3::new(0.0, 0.2, 0.0), color: lin(0xfff4e0) * flick, distance: 8.0 });
        }
        // ---- the glass: a sheen drifting across each mirror
        for (k, (glass, face)) in tv.mirrors.iter().enumerate() {
            let yaw = face.x.atan2(face.z);
            let u = ((time * 0.13 + k as f32 * 0.29).fract() - 0.5) * 1.6;
            let m = Mat4::from_translation(*glass + *face * 0.02)
                * Mat4::from_rotation_y(yaw)
                * Mat4::from_translation(Vec3::new(u * 0.5, u * 0.6, 0.0))
                * Mat4::from_rotation_z(0.6)
                * Mat4::from_scale(Vec3::new(0.12, 2.6, 0.01));
            f.additive.push(Instance::new(m, lin(0xe8f0ff), 0.25 * (1.0 - u.abs() / 0.8).max(0.0), Vec3::ZERO, SHADE_UNLIT));
            // The silvering giving back the hall's light.
            let m =
                Mat4::from_translation(*glass + *face * 0.015) * Mat4::from_rotation_y(yaw) * Mat4::from_scale(Vec3::new(1.2, 2.62, 0.01));
            f.additive.push(Instance::new(m, lin(0xc8d8f0), 0.12, Vec3::ZERO, SHADE_UNLIT));
        }
        // ---- Lucifer and his Reflections
        for (i, e) in s.enemies.iter().enumerate() {
            let Some(d) = e.duel.as_deref().filter(|d| d.alive()) else { continue };
            let Some(v) = self.enemy_views.get(i) else { continue };
            match d.breed.kind {
                EnemyKind::Lucifer => self.emit_lucifer(tv, d, v, time, f),
                EnemyKind::Reflection if v.visible => {
                    // A cold copy: a silver ring for a halo, a pale likeness in the floor.
                    let back = Vec3::new(v.yaw.sin(), 0.0, v.yaw.cos()) * -0.12;
                    glow_orb(f, v.group_pos + back + Vec3::new(0.0, 101.0 * REFLECTION_VOXEL, 0.0), 0.45, lin(SILVER), 0.3);
                    floor_likeness(f, v.group_pos, 0.7, lin(SILVER), 0.1);
                }
                _ => {}
            }
        }
        // ---- the gate he came in by
        let door = s.gate.map(|g| g.door).unwrap_or(0.0);
        self.emit_portal(&tv.gate, door, false, time, hero, f);
    }

    /// Lucifer's own light each frame: his halo and crown glowing, his likeness in the polished
    /// floor, the lance gathering and blazing, the mirror he will step out of, the shadow of his
    /// fall, his burnt wings' smoke and embers.
    fn emit_lucifer(&self, tv: &ThroneView, d: &Duelist, v: &EnemyView, time: f32, f: &mut Frame) {
        let base = v.group_pos;
        let ahead = Vec3::new(v.yaw.sin(), 0.0, v.yaw.cos());
        let fallen = d.breed.phase >= 3;
        let gold = if fallen { lin(0xffb060) } else { lin(LIGHT) };
        if v.visible {
            glow_orb(f, base - ahead * 0.4 + Vec3::new(0.0, 101.0 * LUCIFER_VOXEL, 0.0), 0.7, gold, 0.09 + 0.02 * (time * 2.0).sin());
            f.lights.push(Light { pos: base + ahead * 2.2 + Vec3::new(0.0, 3.2, 0.0), color: lin(0xfff0d8) * 3.0, distance: 6.5 });
            f.lights.push(Light { pos: base - ahead * 0.6 + Vec3::new(0.0, 2.8, 0.0), color: gold * 2.2, distance: 3.5 });
            floor_likeness(f, Vec3::new(base.x, 0.0, base.z), 1.3, gold, 0.16);
        }
        if fallen && v.visible {
            // His burnt wings smoking, embers lifting off them.
            let side = Vec3::new(ahead.z, 0.0, -ahead.x);
            for k in 0..10 {
                let t = (time * 0.45 + k as f32 * 0.1).fract();
                let s = if k % 2 == 0 { 1.0 } else { -1.0 };
                let p = base - ahead * 0.5
                    + side * s * (0.5 + 0.6 * ((k * 7 % 5) as f32 / 5.0))
                    + Vec3::new(0.0, 1.6 + 1.4 * ((k * 3 % 4) as f32 / 4.0) + t * 1.4, 0.0);
                let m = Mat4::from_translation(p + Vec3::new((time + k as f32).sin() * 0.15 * t, 0.0, 0.0))
                    * Mat4::from_scale(Vec3::splat(0.18 + t * 0.35));
                f.transparent[MeshKind::Sphere as usize].push(Instance::new(m, lin(SMOKE), 0.3 * (1.0 - t), Vec3::ZERO, SHADE_UNLIT));
                let e = p + Vec3::new(0.0, t * 0.6, 0.0) + side * s * t * 0.3;
                let m = Mat4::from_translation(e) * Mat4::from_scale(Vec3::splat(0.035));
                f.additive.push(Instance::new(m, lin(EMBER) * 2.5, 0.9 * (1.0 - t), Vec3::ZERO, SHADE_UNLIT));
            }
            f.lights.push(Light { pos: base - ahead * 0.6 + Vec3::new(0.0, 2.2, 0.0), color: lin(EMBER) * 2.5, distance: 3.5 });
        }
        let State::Attack { mv, frame } = d.f.state else { return };
        let data = mv.data();
        let phase = data.phase(frame);
        let aim = Vec3::new(d.aim.x as f32, 0.0, d.aim.y as f32).normalize_or_zero();
        let foot = Vec3::new(d.pos.x as f32, 0.0, d.pos.y as f32);
        match mv {
            MoveId::LanceOfDawn => {
                let span = d.lore.span as f32;
                let hand = foot + aim * 1.1 + Vec3::new(0.0, 1.7, 0.0);
                match phase {
                    Phase::Startup => {
                        // The light gathering at his sword, the line burning brighter on the floor.
                        let u = frame as f32 / data.startup as f32;
                        glow_orb(f, hand, 0.3 + 0.9 * u, lin(LIGHT), 0.4 + 0.6 * u);
                        let mid = foot + aim * (span / 2.0) + Vec3::new(0.0, 0.025, 0.0);
                        let m = Mat4::from_translation(mid)
                            * Mat4::from_rotation_y(aim.x.atan2(aim.z))
                            * Mat4::from_scale(Vec3::new(0.08 + 0.1 * u, 0.01, span));
                        f.additive.push(Instance::new(m, lin(LIGHT) * 2.0, 0.25 + 0.5 * u, Vec3::ZERO, SHADE_UNLIT));
                        f.lights.push(Light { pos: hand, color: lin(LIGHT) * 4.0 * u, distance: 4.0 });
                    }
                    _ => {
                        // The lance: a column of white fire along the floor to where the rock stops it.
                        let after = frame - data.startup;
                        let k = if phase == Phase::Active { 1.0 } else { (1.0 - (after - data.active) as f32 / 14.0).max(0.0) };
                        if k <= 0.0 {
                            return;
                        }
                        let yaw = aim.x.atan2(aim.z);
                        let from = foot + aim * 0.6;
                        let mid = from + aim * (span - 0.6) / 2.0;
                        let len = (span - 0.6).max(0.1);
                        let flick = 1.0 + 0.12 * (time * 50.0).sin();
                        for (y, wide, tall, col, alpha) in [
                            (0.9f32, 0.2f32, 0.2f32, Vec3::splat(3.4), 0.95f32),
                            (0.9, 0.55, 0.5, lin(LIGHT) * 1.8, 0.28),
                            (0.9, 1.0, 0.9, lin(LIGHT) * 1.2, 0.1),
                            (0.02, 1.1, 0.01, lin(GOLD_L) * 2.2, 0.5),
                        ] {
                            let m = Mat4::from_translation(Vec3::new(mid.x, y, mid.z))
                                * Mat4::from_rotation_y(yaw)
                                * Mat4::from_scale(Vec3::new(wide * flick, tall, len));
                            f.additive.push(Instance::new(m, col, alpha * k, Vec3::ZERO, SHADE_UNLIT));
                        }
                        glow_orb(f, hand, 1.4, lin(LIGHT), 0.9 * k);
                        let n = ((span / 3.5).ceil() as i32).clamp(1, 3);
                        for j in 0..n {
                            let p = from + aim * (len * (j as f32 + 0.5) / n as f32) + Vec3::new(0.0, 1.0, 0.0);
                            f.lights.push(Light { pos: p, color: lin(0xfff4dc) * 7.0 * k, distance: 4.5 });
                        }
                    }
                }
            }
            MoveId::MirrorStep if phase == Phase::Startup => {
                // The mirror he will come out of shines, brighter as he goes into the light.
                let mark = Vec3::new(d.lore.mark.x as f32, 0.0, d.lore.mark.y as f32);
                let doors = plan::mirror_doors();
                let k = doors.iter().enumerate().min_by(|a, b| {
                    let da = (Vec3::new(a.1.x as f32, 0.0, a.1.y as f32) - mark).length();
                    let db = (Vec3::new(b.1.x as f32, 0.0, b.1.y as f32) - mark).length();
                    da.total_cmp(&db)
                });
                if let Some((glass, face)) = k.and_then(|(k, _)| tv.mirrors.get(k)) {
                    let u = frame as f32 / data.startup as f32;
                    let yaw = face.x.atan2(face.z);
                    let m = Mat4::from_translation(*glass + *face * 0.03)
                        * Mat4::from_rotation_y(yaw)
                        * Mat4::from_scale(Vec3::new(1.2, 2.6, 0.02));
                    f.additive.push(Instance::new(m, lin(LIGHT) * 2.0, 0.2 + 0.7 * u, Vec3::ZERO, SHADE_UNLIT));
                    glow_orb(f, *glass + *face * 0.4, 1.6, lin(LIGHT), 0.3 + 0.6 * u);
                    f.lights.push(Light { pos: *glass + *face * 1.2, color: lin(LIGHT) * 6.0 * u, distance: 5.0 });
                }
            }
            MoveId::PridesFall if phase == Phase::Startup && frame >= FALL_RISE => {
                // His shadow on his mark, darker as he comes down.
                let mark = Vec3::new(d.lore.mark.x as f32, 0.03, d.lore.mark.y as f32);
                let u = (frame - FALL_RISE) as f32 / (data.startup - FALL_RISE) as f32;
                let r = 1.6 - 0.5 * u;
                let m = Mat4::from_translation(mark) * Mat4::from_rotation_x(-PI / 2.0) * Mat4::from_scale(Vec3::new(r, r, 1.0));
                f.transparent[MeshKind::Disc as usize].push(Instance::new(m, Vec3::ZERO, 0.2 + 0.4 * u, Vec3::ZERO, SHADE_UNLIT));
            }
            _ => {}
        }
    }

    /// The traps on the floor of Pride's court: Lucifer's falling stars (any other's as they are
    /// everywhere, `emit_traps`).
    pub(super) fn emit_pride_traps(&self, s: &SceneState, traps: &[Trap], time: f32, f: &mut Frame) {
        let his =
            |t: &Trap| s.enemies.iter().filter_map(|e| e.duel.as_deref()).any(|d| d.id == t.owner && d.breed.kind == EnemyKind::Lucifer);
        let others: Vec<Trap> = traps.iter().filter(|t| !his(t)).copied().collect();
        if !others.is_empty() {
            self.emit_traps(&others, s.grid, time, f);
        }
        for t in traps.iter().filter(|t| his(t)) {
            let at = Vec3::new(t.pos.x as f32, 0.0, t.pos.y as f32);
            let seed = (t.pos.x * 3.7 + t.pos.y * 1.9) as f32;
            let r = t.radius as f32;
            if let Some(b) = t.burst {
                // It bursts: a pillar of white fire and rays of it thrown up, gone in a moment.
                let u = b as f32 / 20.0;
                if u >= 1.0 {
                    continue;
                }
                let k = 1.0 - u;
                let m = Mat4::from_translation(at + Vec3::new(0.0, 1.5, 0.0)) * Mat4::from_scale(Vec3::new(r * 0.5 * k, 3.0, r * 0.5 * k));
                f.additive.push(Instance::new(m, lin(LIGHT) * 2.5, 0.8 * k, Vec3::ZERO, SHADE_UNLIT));
                for j in 0..6 {
                    let a = j as f32 * TAU / 6.0 + seed;
                    let p = at + Vec3::new(a.cos() * r * u, 0.2 + u * 1.2, a.sin() * r * u);
                    let m = Mat4::from_translation(p)
                        * Mat4::from_rotation_y(a)
                        * Mat4::from_rotation_z(0.8)
                        * Mat4::from_scale(Vec3::new(0.05, 0.4 * k, 0.05));
                    f.additive.push(Instance::new(m, lin(GOLD_L) * 2.4, 0.9 * k, Vec3::ZERO, SHADE_UNLIT));
                }
                let m = Mat4::from_translation(at + Vec3::new(0.0, 0.03, 0.0))
                    * Mat4::from_rotation_x(-PI / 2.0)
                    * Mat4::from_scale(Vec3::new(r * (0.6 + u), r * (0.6 + u), 1.0));
                f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, lin(LIGHT) * 2.0, 0.7 * k, Vec3::ZERO, SHADE_UNLIT));
                f.lights.push(Light { pos: at + Vec3::new(0.0, 0.8, 0.0), color: lin(LIGHT) * 6.0 * k, distance: 3.5 });
                continue;
            }
            let star = |f: &mut Frame, p: Vec3, size: f32, glow: f32| {
                for (k, roll) in [0.0f32, PI / 4.0].into_iter().enumerate() {
                    let m = Mat4::from_translation(p)
                        * Mat4::from_rotation_y(time * 3.0 + seed)
                        * Mat4::from_rotation_z(roll)
                        * Mat4::from_scale(Vec3::new(size * 0.25, size, size * 0.25));
                    f.additive.push(Instance::new(
                        m,
                        if k == 0 { Vec3::splat(3.0) } else { lin(GOLD_L) * 2.6 },
                        0.95,
                        Vec3::ZERO,
                        SHADE_UNLIT,
                    ));
                    let m = Mat4::from_translation(p)
                        * Mat4::from_rotation_y(time * 3.0 + seed)
                        * Mat4::from_rotation_x(PI / 2.0)
                        * Mat4::from_rotation_z(roll)
                        * Mat4::from_scale(Vec3::new(size * 0.25, size, size * 0.25));
                    f.additive.push(Instance::new(m, lin(GOLD_L) * 2.2, 0.8, Vec3::ZERO, SHADE_UNLIT));
                }
                glow_orb(f, p, size * 1.4, lin(LIGHT), glow);
            };
            if t.flight > 0 {
                // Falling from the height of the hall, a tail of light behind it.
                let total = COIN_FLIGHT as f32 + 1.0;
                let k = 1.0 - t.flight as f32 / total;
                let p = at + Vec3::new(-0.8 * (1.0 - k), 7.0 * (1.0 - k * k) + 0.3, 0.0);
                star(f, p, 0.35, 0.7);
                let m = forge::segment(p, p + Vec3::new(-0.5, 2.2, 0.0), 0.06);
                f.additive.push(Instance::new(m, lin(LIGHT) * 2.0, 0.5, Vec3::ZERO, SHADE_UNLIT));
                f.lights.push(Light { pos: p, color: lin(LIGHT) * 2.5, distance: 3.0 });
            } else {
                // Fallen: it lies burning on the floor while a ring of light closes on it (the tell),
                // flashing as it comes due.
                let fill = t.tell();
                let late = t.fuse <= 8;
                let blink = if late { 0.6 + (time * 40.0).sin().abs() * 0.8 } else { 0.0 };
                star(f, at + Vec3::new(0.0, 0.3 + 0.05 * (time * 6.0 + seed).sin(), 0.0), 0.28 + 0.1 * fill, 0.5 + blink * 0.3);
                let n = ((TAU * r) / 0.22).ceil() as i32;
                for k in 0..n {
                    let a = k as f32 * TAU / n as f32;
                    let m = Mat4::from_translation(at + Vec3::new(a.cos() * r, 0.03, a.sin() * r))
                        * Mat4::from_scale(Vec3::new(0.08, 0.012, 0.08));
                    f.additive.push(Instance::new(m, lin(LIGHT) * (1.0 + blink), 0.7, Vec3::ZERO, SHADE_UNLIT));
                }
                let inner = r * (1.0 - fill);
                let m = Mat4::from_translation(at + Vec3::new(0.0, 0.025, 0.0))
                    * Mat4::from_rotation_x(-PI / 2.0)
                    * Mat4::from_scale(Vec3::new(r, r, 1.0));
                f.transparent[MeshKind::Disc as usize].push(Instance::new(
                    m,
                    lin(LIGHT) * (0.6 + blink * 0.6),
                    0.08 + 0.2 * fill,
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
                let m = Mat4::from_translation(at + Vec3::new(0.0, 0.03, 0.0))
                    * Mat4::from_rotation_x(-PI / 2.0)
                    * Mat4::from_scale(Vec3::new(inner + 0.05, inner + 0.05, 1.0));
                f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, lin(GOLD_L) * 1.4, 0.5, Vec3::ZERO, SHADE_UNLIT));
                f.lights.push(Light { pos: at + Vec3::new(0.0, 0.5, 0.0), color: lin(LIGHT) * (1.5 + blink * 2.0), distance: 2.5 });
            }
        }
    }
}

/// A likeness in the polished floor: a soft pool of a body's light under it.
fn floor_likeness(f: &mut Frame, at: Vec3, r: f32, color: Vec3, alpha: f32) {
    let m =
        Mat4::from_translation(Vec3::new(at.x, 0.015, at.z)) * Mat4::from_rotation_x(-PI / 2.0) * Mat4::from_scale(Vec3::new(r, r, 1.0));
    f.transparent[MeshKind::Disc as usize].push(Instance::new(m, color, alpha, Vec3::ZERO, SHADE_UNLIT));
}

fn ease(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/**
 * Lucifer (or a Reflection of him, as in a mirror: its sword in its left hand) dressed for this
 * frame, after his gait and the duel's colours: his white wings, or in his third phase the burnt
 * ones (the others put away under the floor), breathing, spread or folded; and his pose for each
 * of his moves: the cut swept wide, the lance raised to the sky and levelled, the mirror step
 * folded in his wings as he goes into the light (and the light leaving him as he comes out), the
 * stars called down, the fall up on his wings and down on his mark.
 */
pub(super) fn dress_lucifer(rig: &Rig, v: &mut EnemyView, d: &Duelist, time: f32, clips: bool) {
    // By his clips (`view::foe_clips`) his turns, leans and crouches are in them, and his wings too:
    // only his flight over his mark (and his light, his glow, his Reflections' cold) stay the code's.
    let (rot, scale) = (v.root_rot, v.root_scale);
    dress(rig, v, d, time);
    if clips {
        v.root_rot = rot;
        v.root_scale = scale;
    }
}

fn dress(rig: &Rig, v: &mut EnemyView, d: &Duelist, time: f32) {
    let m = if v.kind == EnemyKind::Reflection { -1.0f32 } else { 1.0 };
    let (sword, other) = if m > 0.0 { ("rightArm", "leftArm") } else { ("leftArm", "rightArm") };
    let burnt = d.breed.phase >= 3 && v.kind == EnemyKind::Lucifer;
    // (Wings put away sink far under the floor.)
    for (names, shown) in [(WHITE_WINGS, !burnt), (BURNT_WINGS, burnt)] {
        for name in names {
            if let Some(k) = rig.index(name) {
                v.pose.pos[k] = rig.parts[k].rest - if shown { Vec3::ZERO } else { Vec3::new(0.0, 40.0, 0.0) };
            }
        }
    }
    // The wings: (lift, fold) — lifted up and out, or folded forward round him.
    let mut lift = 0.04 * (time * 1.3 + d.id as f32).sin();
    let mut fold = 0.0;
    if let State::Attack { mv, frame } = d.f.state {
        let data = mv.data();
        let (s, a, f) = (data.startup as f32, data.active as f32, frame as f32);
        let (wind, strike, rec) = if f < s {
            (ease(f / s), 0.0, 0.0)
        } else if f < s + a {
            (1.0, ((f - s + 1.0) / a).min(1.0), 0.0)
        } else {
            (1.0, 1.0, ease((f - s - a) / data.recovery as f32))
        };
        let snap = 1.0 - (1.0 - strike).powi(3);
        let out = 1.0 - rec;
        match mv {
            MoveId::RadiantCut => {
                // The sword drawn back across his body as he winds round, then swept wide before him.
                v.root_rot.y += m * if strike == 0.0 {
                    -0.7 * wind
                } else if rec == 0.0 {
                    -0.7 + 1.8 * snap
                } else {
                    1.1 * out
                };
                let arm = if strike == 0.0 { -2.2 * wind } else { -2.2 + 1.2 * snap };
                v.pose.rot_x(rig, sword, arm * out);
                if let Some(r) = rig.index(sword) {
                    v.pose.rot[r].z = m * 0.6 * wind * out;
                }
                v.pose.rot_x(rig, other, 0.3 * wind * out);
                lift += 0.25 * wind * out;
            }
            MoveId::LanceOfDawn => {
                // The sword raised to the sky as the dawn gathers in it, then levelled at him.
                let arm = if strike == 0.0 { -2.9 * wind } else { -2.9 + 1.4 * snap };
                v.pose.rot_x(rig, sword, arm * out);
                v.pose.rot_x(rig, other, -0.9 * wind * out);
                v.pose.rot_x(rig, "head", -0.25 * wind * (1.0 - snap) * out);
                v.root_rot.x += if strike == 0.0 { -0.08 * wind } else { 0.1 * out };
                lift += 0.45 * wind * out;
            }
            MoveId::MirrorStep => {
                // Folded in his wings, arms crossed, he goes into the light; he comes out of it.
                fold = 1.0 * wind * out;
                v.pose.rot_x(rig, "leftArm", -0.7 * wind * out);
                v.pose.rot_x(rig, "rightArm", -0.7 * wind * out);
                for (name, side) in [("leftArm", 1.0f32), ("rightArm", -1.0)] {
                    if let Some(k) = rig.index(name) {
                        v.pose.rot[k].z = side * 0.5 * wind * out;
                    }
                }
                let light = if f < s { ((f / s - 0.45) / 0.55).clamp(0.0, 1.0) } else { (1.0 - (f - s) / 24.0).max(0.0) };
                v.emissive += lin(0xfff4d8) * 1.6 * light;
                v.mat_color *= 1.0 + 0.6 * light;
            }
            MoveId::Starfall => {
                // Both hands raised to the vault, head thrown back, wings high: the stars come down.
                let up = if strike == 0.0 { -2.9 * wind } else { -2.9 * out };
                v.pose.rot_x(rig, "leftArm", up);
                v.pose.rot_x(rig, "rightArm", up);
                for (name, side) in [("leftArm", -1.0f32), ("rightArm", 1.0)] {
                    if let Some(k) = rig.index(name) {
                        v.pose.rot[k].z = side * 0.35 * snap * out;
                    }
                }
                v.pose.rot_x(rig, "head", -0.4 * wind * out);
                lift += 0.6 * wind * out;
            }
            MoveId::PridesFall => {
                // Down in a crouch, up on beating wings, hanging over his mark; then down on it,
                // the sword first, and up again slowly.
                let rise = FALL_RISE as f32;
                let dive = s - 8.0;
                let high = if f < rise {
                    v.root_scale.y *= 1.0 - 0.1 * (f / rise);
                    0.0
                } else if f < dive {
                    1.9 * ease((f - rise) / (data.track as f32 - rise).max(1.0))
                } else if f < s {
                    1.9 * (1.0 - ((f - dive) / 8.0).powi(2))
                } else {
                    0.0
                };
                v.group_pos.y += high;
                let flap = if f >= rise && f < dive { (time * 14.0).sin() * 0.4 } else { 0.0 };
                lift += if f < s { 0.6 * ease(f / rise) + flap } else { 0.3 * out };
                let arm = if f < s { -2.9 * wind } else { -2.9 + 3.2 * snap };
                v.pose.rot_x(rig, sword, arm * out);
                v.pose.rot_x(rig, other, -1.2 * wind * out);
                if f >= s {
                    v.root_rot.x += 0.3 * out;
                    v.root_scale.y *= 1.0 - 0.06 * out;
                }
            }
            _ => {}
        }
    }
    for (names, side) in [([WHITE_WINGS[0], BURNT_WINGS[0]], -1.0f32), ([WHITE_WINGS[1], BURNT_WINGS[1]], 1.0)] {
        for name in names {
            if let Some(k) = rig.index(name) {
                v.pose.rot[k] = Vec3::new(-0.05 * fold, -side * 1.1 * fold, side * lift);
            }
        }
    }
    if burnt {
        // The fallen: a dull ember glow in him.
        v.emissive += lin(0xff6a2a) * 0.04 * (1.0 + (time * 3.0).sin() * 0.3);
    }
    if v.kind == EnemyKind::Reflection {
        // Cold glass: no warmth in it, and it wavers a little, as a reflection does.
        v.mat_color *= Vec3::new(0.92, 0.97, 1.08) * (0.96 + 0.04 * (time * 7.0 + d.id as f32).sin());
        v.emissive += lin(0x8aa8ff) * 0.06;
        v.light_color = lin(SILVER);
        v.light_intensity = 1.2;
    }
}
