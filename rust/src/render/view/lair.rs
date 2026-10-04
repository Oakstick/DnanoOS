//! Krampus's lair (`logic::lair` lays it out): the bottom of the Pit, under cold blue-white light
//! falling from the shaft far above.
//!
//! A frozen hollow of black stone, its floor slabs dusted with snow and drifted deep against the
//! wall, frost in the cracks. The wall is black rock glazed with ice, icicles hanging from its
//! lip, and over it all the dark. Four columns of ice stand frozen from the roof to the floor.
//! In the south the platform has come to rest, a heavy slab of grey stone with an iron rim, the
//! picked bones of the feast and an overturned cup still on it, a shaft of pale light falling on
//! it and the snow drifting down through the light. About the floor lie bundles of birch
//! switches tied with red ribbon, his sacks and his great basket, broken toys half buried in the
//! snow (a rocking horse, a doll, a drum, a top, a soldier, a ball), and chains hang out of the
//! dark with a bell on each, swaying. Either side of the cage two braziers of his coal burn red.
//!
//! At the north end the iron cage hangs from a great chain over a ring of black iron in the floor,
//! the friend inside it: frightened, at the bars, crying when Krampus rattles them. When Krampus
//! falls his body sinks into the snow while his bells fall silent one by one; the cage's lock
//! breaks, the cage comes down, its door swings open, the boy climbs out and runs to the hero.
//!
//! Krampus's fight is drawn here too: his poses (`pose_krampus`, over the duel's), his coal in
//! its sack glowing, his chain cracking out along its line, the coals he flings (`emit_coals`),
//! and his slow death.

use super::animation::EnemyView;
use super::story::{kid_root, Act};
use super::*;
use crate::game::lair::{ChildState, Lair, BELL_EVERY, LOCK_BREAKS};
use crate::logic::combat::prince::{Trap, COIN_FLIGHT};
use crate::logic::combat::{Duelist, MoveId, Phase, Reel, State};
use crate::logic::lair::{self as plan, BASKET, BIRCH, BRAZIERS, CAGE_MIDDLE, CENTRE, CHAINS, COLUMNS, PLATFORM, RADIUS, SACKS, TOYS};
use crate::render::foes_lair::{KRAMPUS_BELLS, KRAMPUS_RIG, KRAMPUS_SIZE};

/// Where his right fist closes (on the birch, or the sack's neck), in his grid.
const RIGHT_FIST: [f32; 3] = [23.0, 48.4, 62.0];

pub(super) mod pal {
    pub const STONE: u32 = 0x1e2026;
    pub const STONE_L: u32 = 0x2a2c34;
    pub const STONE_D: u32 = 0x14151a;
    pub const ROCK: u32 = 0x18191e;
    pub const ROCK_L: u32 = 0x262a32;
    pub const SNOW: u32 = 0xdfe8f2;
    pub const SNOW_D: u32 = 0xa8b8cc;
    pub const FROST: u32 = 0x8aa8c8;
    pub const ICE: u32 = 0x9cc4e4;
    pub const ICE_D: u32 = 0x5a86b0;
    pub const ICE_L: u32 = 0xd4ecff;
    pub const SLAB: u32 = 0x6a6862;
    pub const SLAB_L: u32 = 0x7e7c74;
    pub const SLAB_D: u32 = 0x4e4c48;
    pub const IRON: u32 = 0x34363c;
    pub const IRON_L: u32 = 0x5a5e66;
    pub const IRON_D: u32 = 0x202226;
    pub const RUST: u32 = 0x5a3a26;
    pub const BRASS: u32 = 0xa8823a;
    pub const BRASS_L: u32 = 0xe8c060;
    pub const WICKER: u32 = 0x4a3622;
    pub const WICKER_D: u32 = 0x2a1e14;
    pub const BARK: u32 = 0xc8c0b0;
    pub const BARK_D: u32 = 0x2a2624;
    pub const RIBBON: u32 = 0xb01822;
    pub const SACK: u32 = 0x5a4630;
    pub const SACK_D: u32 = 0x3e3020;
    pub const ROPE: u32 = 0x8a7450;
    pub const COAL: u32 = 0x18120f;
    pub const EMBER: u32 = 0xff4a14;
    pub const EMBER_HOT: u32 = 0xffa040;
    pub const FLAME: u32 = 0xffb050;
    pub const FLAME_TIP: u32 = 0xff4a18;
    pub const WOOD: u32 = 0x6a4a2c;
    pub const WOOD_D: u32 = 0x3e2a18;
    pub const PAINT_RED: u32 = 0xa02020;
    pub const PAINT_BLUE: u32 = 0x284a8a;
    pub const PAINT_WHITE: u32 = 0xd8d0c0;
    pub const PAINT_GOLD: u32 = 0xc8a040;
    pub const FACE: u32 = 0xe0c8b0;
    pub const BONE: u32 = 0xd8ccb0;
    pub const PEWTER: u32 = 0x8a8c90;
    pub const SHAFT: u32 = 0xc8dcff;
    pub const COLD: u32 = 0x9ab8e8;
}
use pal::*;

/// The voxel materials' shading modes, for plain boxes.
const SHADE_IRON: f32 = 6.0;
const SHADE_FUR: f32 = 7.0;
const SHADE_CLOTH: f32 = 8.0;
const SHADE_WOOD: f32 = 12.0;
const SHADE_BRASS: f32 = 13.0;
const SHADE_ICE: f32 = 15.0;
const SHADE_STONE: f32 = 18.0;
/// The wall stands this tall; over it, the dark.
const WALL_H: f32 = 3.4;
/// The columns of ice rise this high, into the dark.
const COLUMN_H: f32 = 7.5;
/// The cage: its radius and height, how far over the floor it hangs, and how high the great chain
/// is hung from.
const CAGE_R: f32 = 1.15;
const CAGE_H: f32 = 2.1;
const CAGE_HANG: f32 = 0.75;
const CHAIN_TOP: f32 = 7.0;
/// The hanging chains: how far down out of the dark they reach.
const CHAIN_FROM: f32 = 5.5;

/// Cold and dark: the light falls from the shaft far above, blue-white; the coal glows red.
pub fn lair_atmosphere() -> Atmosphere {
    Atmosphere {
        background: 0x04060b,
        fog_near: 15.0,
        fog_far: 42.0,
        hemi_sky: lin(0x5a7aa8) * 0.55,
        hemi_ground: lin(0x2a3242) * 0.45,
        ambient: lin(0x40506c) * 0.6,
        sun_dir: Vec3::new(0.12, 1.0, 0.3).normalize(),
        sun: lin(0xd0e2ff) * 0.95,
        exposure: 1.3,
        night_sky: 0.0,
        bloom: 0.85,
        moon_view: Vec3::new(0.0, 0.3, -1.0).normalize(),
        cloud_tint: lin(0x0a0e18),
    }
}

/// Its ground haze: a cold mist off the snow, low.
pub const HAZE: (f32, f32) = (0.02, 1.3);

/// What the view keeps of the lair for its moving parts.
pub struct LairView {
    /// Where the braziers' coals burn.
    pub braziers: Vec<Vec3>,
    /// The cage's middle on the floor, and the platform's.
    pub cage: Vec3,
    pub platform: Vec3,
}

fn cube(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot_y: f32, color: Vec3, shade: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(rot_y), pos), color, 1.0, Vec3::ZERO, shade));
}

/// A box turned by `rot`, lit from within by `glow`.
fn glowing(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot: Quat, color: Vec3, glow: Vec3, shade: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(scale, rot, pos), color, 1.0, glow, shade));
}

/// A rod from `a` to `b`, `w` thick.
fn rod(out: &mut Vec<Instance>, a: Vec3, b: Vec3, w: f32, color: Vec3, glow: Vec3, shade: f32) {
    let d = b - a;
    let len = d.length().max(1e-4);
    let rot = Quat::from_rotation_arc(Vec3::Y, d / len);
    out.push(Instance::new(Mat4::from_scale_rotation_translation(Vec3::new(w, len, w), rot, (a + b) * 0.5), color, 1.0, glow, shade));
}

/// The `k`-th draw, 0 to 1, from `seed`.
fn rnd(seed: f64, k: u32) -> f32 {
    hash(seed * 1.31 + k as f64 * 7.77 + 0.5, seed * 0.67 - k as f64 * 3.19 + 11.0) as f32
}

fn stone(h: f64) -> Vec3 {
    let set = [STONE, STONE, STONE_L, STONE_D];
    lin(set[(h * 4.0) as usize % 4]) * (0.9 + h as f32 * 0.16)
}

/// How much snow lies at (x, y): drifted deep against the wall, thin in the middle of the floor,
/// none where the fighting has trodden it (about the middle).
fn snow_at(x: f32, y: f32) -> f32 {
    let d = Vec2::new(x - CENTRE.0 as f32, y - CENTRE.1 as f32).length();
    let wall = ((d - (RADIUS as f32 - 3.5)) / 3.5).clamp(0.0, 1.0);
    let n = hash(x as f64 * 0.37 + 3.0, y as f64 * 0.41 + 7.0) as f32;
    (0.25 + 0.75 * wall) * (0.6 + 0.4 * n)
}

/// A chain of iron links from `a` to `b`, `size` a link's length, swinging links turned about.
fn chain(out: &mut Vec<Instance>, a: Vec3, b: Vec3, size: f32, color: Vec3) {
    let d = b - a;
    let len = d.length();
    let n = (len / size).ceil().max(1.0) as i32;
    let dir = d / len.max(1e-4);
    let rot = Quat::from_rotation_arc(Vec3::Y, dir);
    for k in 0..n {
        let p = a + dir * ((k as f32 + 0.5) * len / n as f32);
        let turn = if k % 2 == 0 { 0.0 } else { PI / 2.0 };
        let m =
            Mat4::from_scale_rotation_translation(Vec3::new(size * 0.55, size * 1.05, size * 0.16), rot * Quat::from_rotation_y(turn), p);
        out.push(Instance::new(m, color, 1.0, Vec3::ZERO, SHADE_IRON));
    }
}

/// A bell hanging at `at` (its crown), `r` across, rung by `swing`.
fn hung_bell(out: &mut Vec<Instance>, at: Vec3, r: f32, swing: f32, glint: f32) {
    let rot = Quat::from_rotation_z(swing);
    let body = at + rot * Vec3::new(0.0, -r * 0.9, 0.0);
    out.push(Instance::new(
        Mat4::from_scale_rotation_translation(Vec3::new(r * 1.4, r * 1.6, r * 1.4), rot, body),
        lin(BRASS),
        1.0,
        lin(BRASS_L) * glint,
        SHADE_BRASS,
    ));
    out.push(Instance::new(
        Mat4::from_scale_rotation_translation(Vec3::new(r * 1.9, r * 0.35, r * 1.9), rot, at + rot * Vec3::new(0.0, -r * 1.7, 0.0)),
        lin(BRASS) * 0.8,
        1.0,
        Vec3::ZERO,
        SHADE_BRASS,
    ));
}

impl WorldView {
    /// The lair's static scene.
    pub(super) fn build_lair(&mut self, grid: &Grid) {
        let c = Vec3::new(CENTRE.0 as f32, 0.0, CENTRE.1 as f32);
        let floor_tile = |x: i32, y: i32| plan::in_hollow(x, y);

        // ---- the floor: slabs of black stone, snow dusted over them and drifted against the wall
        for y in 0..grid.height {
            for x in 0..grid.width {
                if !floor_tile(x, y) || plan::on_platform(x, y) {
                    continue;
                }
                let (xf, yf) = (x as f32, y as f32);
                let h = hash(x as f64 * 1.7 + 3.0, y as f64 * 2.9 + 5.0);
                let out = &mut self.static_cubes;
                let tilt = (h as f32 - 0.5) * 0.04;
                out.push(Instance::new(
                    Mat4::from_scale_rotation_translation(
                        Vec3::new(0.97, 0.14, 0.97),
                        Quat::from_rotation_x(tilt) * Quat::from_rotation_z(-tilt * 0.7),
                        Vec3::new(xf, -0.07 + (h as f32 - 0.5) * 0.025, yf),
                    ),
                    stone(h).lerp(lin(SNOW_D) * 0.5, snow_at(xf, yf) * 0.35),
                    1.0,
                    Vec3::ZERO,
                    SHADE_STONE,
                ));
                // Snow over it: a powder of fine flakes, thicker toward the wall, and against the
                // wall drifts heaped in soft mounds.
                let snow = snow_at(xf, yf);
                let chips = (1.0 + snow * 6.0) as u32;
                for k in 0..chips {
                    let p = Vec3::new(xf + (rnd(h, k + 4) - 0.5) * 0.9, 0.0, yf + (rnd(h, k + 40) - 0.5) * 0.9);
                    let s = 0.05 + rnd(h, k + 12) * (0.06 + snow * 0.12);
                    let col = lin(SNOW_D).lerp(lin(FROST), rnd(h, k + 20) * 0.6) * 0.85;
                    cube(
                        out,
                        p + Vec3::new(0.0, 0.006, 0.0),
                        Vec3::new(s, 0.012, s * (0.6 + rnd(h, k + 60) * 0.5)),
                        rnd(h, k + 24) * PI,
                        col,
                        SHADE_LIT,
                    );
                }
                if snow > 0.5 {
                    // A drift: soft steps of snow heaped one on another, each smaller and turned.
                    let deep = (snow - 0.5) * 2.0;
                    let p = Vec3::new(xf + (rnd(h, 90) - 0.5) * 0.3, 0.0, yf + (rnd(h, 91) - 0.5) * 0.3);
                    let mut lift = 0.0;
                    for j in 0..4 {
                        let step = 0.03 + deep * 0.06;
                        let s = (1.15 - j as f32 * 0.24) * (0.85 + rnd(h, 92 + j) * 0.3);
                        if s < 0.2 || j as f32 > 1.0 + deep * 3.0 {
                            break;
                        }
                        let col = lin(SNOW).lerp(lin(SNOW_D), 0.35 - 0.1 * j as f32);
                        cube(
                            out,
                            p + Vec3::new(0.0, lift + step * 0.5, 0.0),
                            Vec3::new(s, step, s * 0.8),
                            j as f32 * 0.7 + rnd(h, 95) * 0.6,
                            col,
                            SHADE_LIT,
                        );
                        lift += step;
                    }
                }
                if h > 0.88 {
                    // A crack across the slab, frost in it.
                    let a = hash(x as f64 * 5.0, y as f64 * 9.0) as f32 * PI;
                    cube(out, Vec3::new(xf, 0.006, yf), Vec3::new(0.85, 0.008, 0.035), a, lin(FROST), SHADE_LIT);
                }
            }
        }
        // A ring of iron set in the floor under the cage, black and rimed.
        let cage = Vec3::new(CAGE_MIDDLE.0 as f32, 0.0, CAGE_MIDDLE.1 as f32);
        for k in 0..32 {
            let a = k as f32 * TAU / 32.0;
            let p = cage + Vec3::new(a.cos() * 1.35, 0.03, a.sin() * 1.35);
            cube(&mut self.static_cubes, p, Vec3::new(0.3, 0.06, 0.12), -a + PI / 2.0, lin(IRON_D), SHADE_IRON);
        }
        cube(
            &mut self.static_cubes,
            cage + Vec3::new(0.0, 0.01, 0.0),
            Vec3::new(2.4, 0.02, 2.4),
            PI / 4.0,
            lin(STONE_D) * 0.6,
            SHADE_STONE,
        );
        cube(&mut self.static_cubes, cage + Vec3::new(0.0, 0.012, 0.0), Vec3::new(2.4, 0.02, 2.4), 0.0, lin(STONE_D) * 0.6, SHADE_STONE);

        // ---- the platform, come to rest: a heavy slab of grey stone with an iron rim
        let (px0, py0, px1, py1) = PLATFORM;
        let platform = Vec3::new((px0 + px1) as f32 / 2.0, 0.0, (py0 + py1) as f32 / 2.0);
        let (w, d) = ((px1 - px0 + 1) as f32, (py1 - py0 + 1) as f32);
        {
            let out = &mut self.static_cubes;
            // Its body, the top a little above the floor, and great flagstones over it.
            cube(out, platform + Vec3::new(0.0, -0.3, 0.0), Vec3::new(w + 0.1, 0.66, d + 0.1), 0.0, lin(SLAB_D), SHADE_STONE);
            for y in py0..=py1 {
                for x in px0..=px1 {
                    if (x - px0) % 2 == 1 || (y - py0) % 2 == 1 {
                        continue;
                    }
                    let h = hash(x as f64 * 0.9 + 1.0, y as f64 * 1.3 + 2.0);
                    let col = [SLAB, SLAB_L, SLAB, SLAB_D][(h * 4.0) as usize % 4];
                    let sw = if x + 1 > px1 { 0.96 } else { 1.96 };
                    let sd = if y + 1 > py1 { 0.96 } else { 1.96 };
                    let mid = Vec3::new(x as f32 + (sw - 0.96) * 0.5, 0.02, y as f32 + (sd - 0.96) * 0.5);
                    cube(out, mid, Vec3::new(sw, 0.06, sd), 0.0, lin(col) * (0.92 + h as f32 * 0.12), SHADE_STONE);
                }
            }
            // The iron rim, riveted, round its four edges, and the iron rings it was lowered by.
            let edge = |out: &mut Vec<Instance>, a: Vec3, b: Vec3| {
                let mid = (a + b) * 0.5;
                let len = (b - a).length();
                let yaw = if (b - a).x.abs() > (b - a).z.abs() { 0.0 } else { PI / 2.0 };
                cube(out, mid + Vec3::new(0.0, 0.0, 0.0), Vec3::new(len + 0.3, 0.2, 0.22), yaw, lin(IRON), SHADE_IRON);
                let n = (len / 0.7).ceil() as i32;
                for k in 0..=n {
                    let p = a.lerp(b, k as f32 / n as f32) + Vec3::new(0.0, 0.11, 0.0);
                    cube(out, p, Vec3::splat(0.07), 0.0, lin(IRON_L), SHADE_IRON);
                }
            };
            let (x0, x1, z0, z1) = (px0 as f32 - 0.55, px1 as f32 + 0.55, py0 as f32 - 0.55, py1 as f32 + 0.55);
            for (a, b) in [
                (Vec3::new(x0, 0.06, z0), Vec3::new(x1, 0.06, z0)),
                (Vec3::new(x0, 0.06, z1), Vec3::new(x1, 0.06, z1)),
                (Vec3::new(x0, 0.06, z0), Vec3::new(x0, 0.06, z1)),
                (Vec3::new(x1, 0.06, z0), Vec3::new(x1, 0.06, z1)),
            ] {
                edge(out, a, b);
            }
            for (x, z) in [(x0, z0), (x1, z0), (x0, z1), (x1, z1)] {
                cube(out, Vec3::new(x, 0.14, z), Vec3::new(0.5, 0.3, 0.5), PI / 4.0, lin(IRON_D), SHADE_IRON);
                for k in 0..10 {
                    let a = k as f32 * TAU / 10.0;
                    cube(
                        out,
                        Vec3::new(x + a.cos() * 0.22, 0.38, z + a.sin() * 0.22),
                        Vec3::new(0.12, 0.05, 0.05),
                        -a,
                        lin(IRON_L),
                        SHADE_IRON,
                    );
                }
            }
            // What is left of the feast: picked bones and an overturned cup.
            let corner = Vec3::new(x1 - 1.4, 0.06, z0 + 1.2);
            for k in 0..5 {
                let p = corner + Vec3::new((rnd(3.0, k) - 0.5) * 1.2, 0.03, (rnd(5.0, k) - 0.5) * 0.9);
                cube(out, p, Vec3::new(0.34, 0.05, 0.06), rnd(7.0, k) * PI, lin(BONE), SHADE_LIT);
                cube(out, p + Vec3::new(0.15, 0.0, 0.0), Vec3::new(0.08, 0.07, 0.09), 0.0, lin(BONE), SHADE_LIT);
            }
            let cup = corner + Vec3::new(-0.9, 0.09, 0.6);
            out.push(Instance::new(
                Mat4::from_scale_rotation_translation(Vec3::new(0.14, 0.22, 0.14), Quat::from_rotation_z(PI / 2.0), cup),
                lin(PEWTER),
                1.0,
                Vec3::ZERO,
                SHADE_IRON,
            ));
        }

        // ---- the wall: black rock glazed with ice, icicles hanging from its lip
        for (x, y, t) in grid.cells() {
            if t != Tile::Rock || floor_tile(x, y) {
                continue;
            }
            let near =
                [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)].iter().any(|&(dx, dy)| floor_tile(x + dx, y + dy));
            if !near {
                continue;
            }
            let start = self.static_cubes.len();
            let (xf, yf) = (x as f32, y as f32);
            let h = hash(x as f64 * 3.1, y as f64 * 4.7);
            let top = WALL_H + (h as f32 - 0.5) * 0.9;
            let toward = Vec2::new(c.x - xf, c.z - yf).normalize_or_zero();
            let out = &mut self.static_cubes;
            let mut yb = 0.0;
            let mut course = 0;
            while yb < top - 0.05 {
                let hh = (0.5 + rnd(h, course) * 0.4).min(top - yb);
                let off = (rnd(h, course + 10) - 0.5) * 0.16;
                let col = lin(if course % 3 == 1 { ROCK_L } else { ROCK }) * (0.85 + rnd(h, course + 20) * 0.3);
                cube(out, Vec3::new(xf + off, yb + hh / 2.0, yf - off), Vec3::new(1.04, hh - 0.02, 1.04), off, col, SHADE_STONE);
                yb += hh;
                course += 1;
            }
            // Ice glazing the rock's face toward the hollow, and the lip's icicles.
            let face = Vec3::new(xf + toward.x * 0.5, 0.0, yf + toward.y * 0.5);
            let yaw = -toward.y.atan2(toward.x);
            for k in 0..2 {
                if rnd(h, 28 + k) < 0.45 {
                    continue;
                }
                // (A run of ice down the face, thin and dark: meltwater frozen where it ran.)
                let tall = 0.6 + rnd(h, 34 + k) * 1.2;
                let y0 = tall * 0.5 + rnd(h, 30 + k) * (top - tall).max(0.0);
                out.push(Instance::new(
                    Mat4::from_scale_rotation_translation(
                        Vec3::new(0.035, tall, 0.18 + rnd(h, 38 + k) * 0.22),
                        Quat::from_rotation_y(yaw),
                        face + Vec3::new(0.0, y0, 0.0) + Vec3::new(-toward.y, 0.0, toward.x) * (rnd(h, 42 + k) - 0.5) * 0.6,
                    ),
                    lin(ICE_D).lerp(lin(ICE), rnd(h, 46 + k)),
                    1.0,
                    lin(ICE_L) * 0.02,
                    SHADE_ICE,
                ));
            }
            for k in 0..4 {
                if rnd(h, 50 + k) < 0.35 {
                    continue;
                }
                let along = (rnd(h, 54 + k) - 0.5) * 0.9;
                let long = 0.25 + rnd(h, 58 + k) * 0.9;
                let p = face + Vec3::new(-toward.y, 0.0, toward.x) * along + Vec3::new(0.0, top - long * 0.5, 0.0);
                cube(out, p, Vec3::new(0.07, long, 0.07), rnd(h, 62 + k) * PI, lin(ICE_L), SHADE_ICE);
                cube(out, p - Vec3::new(0.0, long * 0.42, 0.0), Vec3::new(0.035, long * 0.25, 0.035), 0.0, lin(ICE_L), SHADE_ICE);
            }
            // Snow lying on the wall's top.
            cube(out, Vec3::new(xf, top + 0.04, yf), Vec3::new(0.98, 0.08, 0.98), h as f32, lin(SNOW_D), SHADE_LIT);
            let len = self.static_cubes.len() - start;
            self.rock_index.insert(tile_key(x, y), (start, len));
            let shades: Vec<f32> = self.static_cubes[start..start + len].iter().map(|i| i.emissive[3]).collect();
            self.trees.push(Tree {
                range: (start, len),
                base: Vec3::new(xf, 0.0, yf),
                radius: 0.72,
                height: top + 0.3,
                fade: 1.0,
                alive: true,
                shades: Some(shades),
            });
        }

        // ---- the columns of ice, frozen from the roof to the floor
        for (i, &p) in COLUMNS.iter().enumerate() {
            let m = plan::column_middle(p);
            let base = Vec3::new(m.0 as f32, 0.0, m.1 as f32);
            let start = self.static_cubes.len();
            self.build_ice_column(base, i as f64);
            let len = self.static_cubes.len() - start;
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                self.rock_index.insert(tile_key(p.0 + dx, p.1 + dy), (start, len));
            }
            let shades: Vec<f32> = self.static_cubes[start..start + len].iter().map(|i| i.emissive[3]).collect();
            self.trees.push(Tree {
                range: (start, len),
                base,
                radius: 1.1,
                height: COLUMN_H,
                fade: 1.0,
                alive: true,
                shades: Some(shades),
            });
        }

        // ---- the braziers of his coal
        let mut braziers = Vec::new();
        for &(x, y) in &BRAZIERS {
            let at = Vec3::new(x as f32, 0.0, y as f32);
            let start = self.static_cubes.len();
            braziers.push(self.build_coal_brazier(at));
            self.solid_prop(x, y, at, start, 1.5);
        }
        // ---- his basket and his sacks against the wall
        let start = self.static_cubes.len();
        let basket = Vec3::new(BASKET.0 as f32, 0.0, BASKET.1 as f32);
        self.build_basket(basket);
        self.solid_prop(BASKET.0, BASKET.1, basket, start, 1.9);
        for (k, &(x, y)) in SACKS.iter().enumerate() {
            let at = Vec3::new(x as f32, 0.0, y as f32);
            let start = self.static_cubes.len();
            self.build_sack(at, k as f64);
            self.solid_prop(x, y, at, start, 1.2);
        }
        // ---- bundles of birch switches and broken toys in the snow
        for (k, &(x, y)) in BIRCH.iter().enumerate() {
            self.build_birch_bundle(Vec3::new(x as f32, 0.0, y as f32), k as f64);
        }
        for (k, &(x, y)) in TOYS.iter().enumerate() {
            self.build_toy(Vec3::new(x as f32, 0.0, y as f32), k);
        }
        self.lair = Some(LairView { braziers, cage, platform });
    }

    /// Registers a solid prop on the floor at its tile, for the camera's fading.
    fn solid_prop(&mut self, x: i32, y: i32, base: Vec3, start: usize, height: f32) {
        let len = self.static_cubes.len() - start;
        self.rock_index.insert(tile_key(x, y), (start, len));
        let shades: Vec<f32> = self.static_cubes[start..start + len].iter().map(|i| i.emissive[3]).collect();
        self.trees.push(Tree { range: (start, len), base, radius: 0.6, height, fade: 1.0, alive: true, shades: Some(shades) });
    }

    /// A column of ice frozen from the dark down to the floor: knotted blocks of pale blue, wider at
    /// its foot where it has run and frozen, icicles about it, snow heaped at its foot.
    fn build_ice_column(&mut self, base: Vec3, seed: f64) {
        let out = &mut self.static_cubes;
        let mut y = 0.0;
        let mut k = 0;
        while y < COLUMN_H {
            let tall = 0.5 + rnd(seed, k) * 0.6;
            let foot = (1.0 - y / 2.0).max(0.0);
            let r = 0.78 + 0.25 * (y * 1.3 + seed as f32).sin() + 0.5 * foot + rnd(seed, k + 40) * 0.12;
            let off = Vec3::new((rnd(seed, k + 80) - 0.5) * 0.18, 0.0, (rnd(seed, k + 90) - 0.5) * 0.18);
            let col = lin(ICE).lerp(lin(ICE_D), rnd(seed, k + 100) * 0.6);
            for turn in [0.0, PI / 4.0] {
                out.push(Instance::new(
                    Mat4::from_scale_rotation_translation(
                        Vec3::new(r * 1.7, tall + 0.05, r * 1.6),
                        Quat::from_rotation_y(turn + rnd(seed, k + 120) * 0.4),
                        base + off + Vec3::new(0.0, y + tall * 0.5, 0.0),
                    ),
                    col,
                    1.0,
                    lin(ICE_L) * 0.05,
                    SHADE_ICE,
                ));
            }
            y += tall;
            k += 1;
        }
        // Runs of meltwater frozen down its face, paler.
        for j in 0..6 {
            let a = j as f32 * TAU / 6.0 + seed as f32;
            let r = 1.05;
            let top = 2.0 + rnd(seed, 200 + j) * 4.0;
            rod(
                out,
                base + Vec3::new(a.cos() * r, top, a.sin() * r),
                base + Vec3::new(a.cos() * (r + 0.25), 0.05, a.sin() * (r + 0.25)),
                0.14,
                lin(ICE_L),
                lin(ICE_L) * 0.06,
                SHADE_ICE,
            );
        }
        // Snow heaped round its foot.
        for j in 0..8 {
            let a = j as f32 * TAU / 8.0 + 0.3;
            let p = base + Vec3::new(a.cos() * 1.35, 0.06, a.sin() * 1.35);
            cube(out, p, Vec3::new(0.7, 0.14 + rnd(seed, 300 + j) * 0.1, 0.45), -a, lin(SNOW), SHADE_LIT);
        }
    }

    /// An iron brazier on three legs, its bowl heaped with his coal. Returns where the coal burns.
    fn build_coal_brazier(&mut self, at: Vec3) -> Vec3 {
        let out = &mut self.static_cubes;
        for k in 0..3 {
            let a = k as f32 * TAU / 3.0 + 0.3;
            rod(
                out,
                at + Vec3::new(a.cos() * 0.45, 0.0, a.sin() * 0.45),
                at + Vec3::new(a.cos() * 0.2, 0.75, a.sin() * 0.2),
                0.07,
                lin(IRON_D),
                Vec3::ZERO,
                SHADE_IRON,
            );
        }
        for turn in [0.0, PI / 4.0] {
            cube(out, at + Vec3::new(0.0, 0.85, 0.0), Vec3::new(0.85, 0.24, 0.85), turn, lin(IRON), SHADE_IRON);
        }
        cube(out, at + Vec3::new(0.0, 0.98, 0.0), Vec3::new(0.95, 0.05, 0.95), PI / 8.0, lin(RUST), SHADE_IRON);
        for k in 0..9 {
            let a = k as f32 * 2.4;
            let r = 0.12 + (k % 3) as f32 * 0.1;
            let p = at + Vec3::new(a.cos() * r, 1.02 + (k % 2) as f32 * 0.05, a.sin() * r);
            let hot = if k % 3 == 0 { lin(EMBER_HOT) * 1.6 } else { lin(EMBER) * 1.1 };
            glowing(out, p, Vec3::new(0.16, 0.12, 0.14), Quat::from_rotation_y(a), lin(COAL), hot, SHADE_LIT);
        }
        at + Vec3::new(0.0, 1.08, 0.0)
    }

    /// His great wicker basket, set down against the wall, bristling with birch switches.
    fn build_basket(&mut self, at: Vec3) {
        let out = &mut self.static_cubes;
        for k in 0..7 {
            let y = 0.12 + k as f32 * 0.22;
            let col = if k % 2 == 0 { lin(WICKER) } else { lin(WICKER_D) };
            cube(out, at + Vec3::new(0.0, y, 0.0), Vec3::new(1.0 + k as f32 * 0.03, 0.2, 0.75 + k as f32 * 0.02), 0.2, col, SHADE_WOOD);
        }
        cube(out, at + Vec3::new(0.0, 1.62, 0.0), Vec3::new(1.26, 0.1, 0.95), 0.2, lin(WICKER_D), SHADE_WOOD);
        for k in 0..9 {
            let a = k as f32 * 0.7;
            let base = at + Vec3::new(a.cos() * 0.3, 1.3, a.sin() * 0.22);
            let tip = base + Vec3::new(a.cos() * 0.3 + (rnd(9.0, k) - 0.5) * 0.4, 1.0 + rnd(11.0, k) * 0.5, a.sin() * 0.3);
            rod(out, base, tip, 0.04, lin(BARK), Vec3::ZERO, SHADE_WOOD);
        }
        cube(out, at + Vec3::new(0.0, 0.03, 0.0), Vec3::new(1.6, 0.06, 1.3), 0.2, lin(SNOW_D), SHADE_LIT);
    }

    /// One of his sacks, slumped against the wall, tied at the neck; one spilling coal.
    fn build_sack(&mut self, at: Vec3, seed: f64) {
        let out = &mut self.static_cubes;
        let lean = (rnd(seed, 1) - 0.5) * 0.5;
        for (k, (y, s)) in [(0.25f32, 0.85f32), (0.6, 0.78), (0.88, 0.55), (1.05, 0.25)].into_iter().enumerate() {
            let col = if k % 2 == 0 { lin(SACK) } else { lin(SACK_D) };
            out.push(Instance::new(
                Mat4::from_scale_rotation_translation(
                    Vec3::new(s, 0.38, s * 0.85),
                    Quat::from_rotation_z(lean * y),
                    at + Vec3::new(lean * y * 0.6, y, 0.0),
                ),
                col,
                1.0,
                Vec3::ZERO,
                SHADE_CLOTH,
            ));
        }
        cube(out, at + Vec3::new(lean * 0.7, 1.0, 0.0), Vec3::new(0.32, 0.06, 0.32), 0.0, lin(ROPE), SHADE_CLOTH);
        if seed as i32 == 0 {
            for k in 0..6 {
                let p = at + Vec3::new(0.5 + rnd(seed, k + 4) * 0.6, 0.06, (rnd(seed, k + 9) - 0.5) * 0.6);
                cube(out, p, Vec3::new(0.14, 0.1, 0.12), rnd(seed, k) * PI, lin(COAL), SHADE_STONE);
            }
        }
    }

    /// A bundle of birch switches lying in the snow, tied with a red ribbon.
    fn build_birch_bundle(&mut self, at: Vec3, seed: f64) {
        let out = &mut self.static_cubes;
        let yaw = rnd(seed, 1) * TAU;
        let dir = Vec3::new(yaw.cos(), 0.0, yaw.sin());
        let side = Vec3::new(-dir.z, 0.0, dir.x);
        for k in 0..7 {
            let fan = (k as f32 - 3.0) * 0.05;
            let a = at - dir * 0.5 + side * (k as f32 - 3.0) * 0.03 + Vec3::new(0.0, 0.04, 0.0);
            let b = at + (dir + side * fan * 4.0) * 0.55 + Vec3::new(0.0, 0.03, 0.0);
            rod(out, a, b, 0.035, lin(if k % 3 == 1 { BARK_D } else { BARK }), Vec3::ZERO, SHADE_WOOD);
        }
        cube(out, at - dir * 0.22 + Vec3::new(0.0, 0.06, 0.0), Vec3::new(0.08, 0.08, 0.26), -yaw + PI / 2.0, lin(RIBBON), SHADE_CLOTH);
        cube(out, at + Vec3::new(0.0, 0.015, 0.0), Vec3::new(1.2, 0.03, 0.5), -yaw, lin(SNOW), SHADE_LIT);
    }

    /// A broken toy half buried in the snow: a rocking horse, a doll, a drum, a top, a tin soldier,
    /// a ball.
    fn build_toy(&mut self, at: Vec3, which: usize) {
        let out = &mut self.static_cubes;
        let yaw = rnd(which as f64 + 0.5, 3) * TAU;
        let r = Quat::from_rotation_y(yaw);
        let put = |out: &mut Vec<Instance>, p: Vec3, s: Vec3, rot: Quat, col: u32, shade: f32| {
            out.push(Instance::new(Mat4::from_scale_rotation_translation(s, r * rot, at + r * p), lin(col), 1.0, Vec3::ZERO, shade));
        };
        match which % 6 {
            0 => {
                // A rocking horse on its side, one rocker snapped.
                let tip = Quat::from_rotation_z(1.3);
                put(out, Vec3::new(0.0, 0.2, 0.0), Vec3::new(0.18, 0.5, 0.22), tip, PAINT_WHITE, SHADE_WOOD);
                put(out, Vec3::new(-0.3, 0.24, 0.0), Vec3::new(0.14, 0.22, 0.2), tip, PAINT_WHITE, SHADE_WOOD);
                put(out, Vec3::new(-0.42, 0.3, 0.0), Vec3::new(0.06, 0.12, 0.22), tip, PAINT_RED, SHADE_CLOTH);
                put(out, Vec3::new(0.1, 0.06, 0.18), Vec3::new(0.7, 0.05, 0.05), Quat::from_rotation_y(0.2), WOOD, SHADE_WOOD);
                put(out, Vec3::new(0.3, 0.05, -0.2), Vec3::new(0.35, 0.05, 0.05), Quat::from_rotation_y(-0.5), WOOD_D, SHADE_WOOD);
            }
            1 => {
                // A doll face up in the snow, her dress red.
                put(out, Vec3::new(0.0, 0.05, 0.0), Vec3::new(0.18, 0.08, 0.32), Quat::IDENTITY, PAINT_RED, SHADE_CLOTH);
                put(out, Vec3::new(0.0, 0.07, -0.24), Vec3::new(0.13, 0.12, 0.13), Quat::IDENTITY, FACE, SHADE_LIT);
                put(out, Vec3::new(0.0, 0.1, -0.29), Vec3::new(0.15, 0.06, 0.08), Quat::IDENTITY, WOOD_D, SHADE_FUR);
                put(out, Vec3::new(0.12, 0.04, 0.05), Vec3::new(0.2, 0.04, 0.05), Quat::from_rotation_y(0.6), FACE, SHADE_LIT);
            }
            2 => {
                // A drum, split, its sticks beside it.
                put(out, Vec3::new(0.0, 0.14, 0.0), Vec3::new(0.36, 0.26, 0.36), Quat::from_rotation_x(0.3), PAINT_BLUE, SHADE_WOOD);
                put(out, Vec3::new(0.0, 0.29, 0.04), Vec3::new(0.38, 0.03, 0.38), Quat::from_rotation_x(0.3), PAINT_WHITE, SHADE_CLOTH);
                put(out, Vec3::new(0.0, 0.14, 0.0), Vec3::new(0.38, 0.04, 0.38), Quat::from_rotation_x(0.3), PAINT_GOLD, SHADE_BRASS);
                put(out, Vec3::new(0.32, 0.03, 0.1), Vec3::new(0.3, 0.025, 0.025), Quat::from_rotation_y(0.4), WOOD, SHADE_WOOD);
                put(out, Vec3::new(0.3, 0.03, -0.12), Vec3::new(0.3, 0.025, 0.025), Quat::from_rotation_y(-0.3), WOOD, SHADE_WOOD);
            }
            3 => {
                // A spinning top, fallen over.
                put(out, Vec3::new(0.0, 0.1, 0.0), Vec3::new(0.24, 0.16, 0.24), Quat::from_rotation_z(1.2), PAINT_RED, SHADE_WOOD);
                put(out, Vec3::new(0.06, 0.1, 0.0), Vec3::new(0.26, 0.04, 0.26), Quat::from_rotation_z(1.2), PAINT_GOLD, SHADE_WOOD);
                put(out, Vec3::new(-0.16, 0.06, 0.0), Vec3::new(0.04, 0.12, 0.04), Quat::from_rotation_z(1.2), WOOD_D, SHADE_WOOD);
            }
            4 => {
                // A tin soldier, face down, his red coat and tall black hat.
                put(out, Vec3::new(0.0, 0.05, 0.0), Vec3::new(0.12, 0.1, 0.26), Quat::IDENTITY, PAINT_RED, SHADE_IRON);
                put(out, Vec3::new(0.0, 0.05, -0.2), Vec3::new(0.1, 0.1, 0.16), Quat::IDENTITY, IRON_D, SHADE_IRON);
                put(out, Vec3::new(0.0, 0.05, 0.2), Vec3::new(0.11, 0.09, 0.16), Quat::IDENTITY, PAINT_BLUE, SHADE_IRON);
            }
            _ => {
                // A ball, half sunk in a drift.
                put(out, Vec3::new(0.0, 0.08, 0.0), Vec3::new(0.26, 0.24, 0.26), Quat::from_rotation_y(0.7), PAINT_RED, SHADE_CLOTH);
                put(out, Vec3::new(0.0, 0.08, 0.0), Vec3::new(0.27, 0.06, 0.27), Quat::from_rotation_x(0.4), PAINT_WHITE, SHADE_CLOTH);
            }
        }
        // The snow it lies in, heaped a little over it.
        put(out, Vec3::new(0.08, 0.025, 0.06), Vec3::new(0.9, 0.05, 0.7), Quat::from_rotation_y(0.4), SNOW, SHADE_LIT);
    }

    /// Each frame: the cold light falling from above and the snow in it, the braziers burning, the
    /// chains swaying with their bells, the cage and the boy in it, and Krampus's own: his coal,
    /// his chain cracking out, and his slow death.
    pub(super) fn emit_lair(&self, s: &SceneState, time: f32, hero: Vec3, f: &mut Frame) {
        let Some(lv) = &self.lair else { return };
        f.bounds = Vec4::new(-0.5, -0.5, s.grid.width as f32 - 0.5, s.grid.height as f32 - 0.5);
        let state = s.lair;
        // The cold light from the shaft above: on the platform, and over the cage.
        f.lights.push(Light { pos: lv.platform + Vec3::new(0.0, 6.0, 0.0), color: lin(SHAFT) * 7.0, distance: 13.0 });
        f.lights.push(Light { pos: lv.cage + Vec3::new(0.0, 5.0, 1.2), color: lin(COLD) * 5.0, distance: 9.0 });
        f.lights.push(Light { pos: hero + Vec3::new(0.0, 4.5, 0.0), color: lin(COLD) * 2.0, distance: 7.0 });
        self.emit_shaft(lv.platform, 4.4, time, f);
        self.emit_shaft(lv.cage, 1.6, time + 7.0, f);
        // The braziers.
        for (i, &p) in lv.braziers.iter().enumerate() {
            coal_flames(f, p, 0.9, time, i as f32);
            let flick = 4.0 + (time * 6.0 + i as f32 * 1.3).sin() * 0.6 + (time * 15.0 + i as f32).sin() * 0.3;
            f.lights.push(Light { pos: p + Vec3::new(0.0, 0.4, 0.0), color: lin(EMBER) * flick, distance: 6.0 });
            glow_orb(f, p + Vec3::new(0.0, 0.2, 0.0), 0.9, lin(EMBER), 0.2 + 0.04 * (time * 7.0 + i as f32).sin());
        }
        // The chains hanging out of the dark, swaying, a bell on each.
        let near = |p: Vec3| p.distance(hero) < 16.0;
        for (k, &(x, y)) in CHAINS.iter().enumerate() {
            let top = Vec3::new(x as f32, CHAIN_FROM + 2.0, y as f32);
            if !near(top) {
                continue;
            }
            let len = 2.6 + (k % 4) as f32 * 0.5;
            let sway = (time * (0.5 + k as f32 * 0.07) + k as f32).sin() * 0.06 + (time * 1.3 + k as f32 * 2.0).sin() * 0.015;
            let end = top + Vec3::new(sway.sin() * len, -len, (sway * 0.7).sin() * len * 0.5);
            chain(&mut f.opaque[MeshKind::Cube as usize], top, end, 0.18, lin(IRON));
            hung_bell(&mut f.opaque[MeshKind::Cube as usize], end, 0.11, sway * 2.0, 0.1);
        }
        // The cage and the boy.
        self.emit_cage(lv.cage, state, s, time, f);
        // Krampus's own.
        for (i, e) in s.enemies.iter().enumerate() {
            if let Some(d) = e.duel.as_deref().filter(|d| d.alive() && d.breed.kind == EnemyKind::Krampus) {
                self.emit_krampus_fight(i, d, time, f);
            }
        }
        if let Some(fall) = state.and_then(|l| l.fall) {
            self.emit_krampus_fall(fall, time, f);
        }
    }

    /// A shaft of pale light falling from far above onto `at`, `r` across, and the snow drifting down
    /// through it.
    fn emit_shaft(&self, at: Vec3, r: f32, time: f32, f: &mut Frame) {
        // The pool of light it lays on the floor.
        for (rr, a) in [(r * 1.1, 0.018f32), (r * 0.75, 0.022), (r * 0.45, 0.03)] {
            let m = Mat4::from_translation(at + Vec3::new(0.0, 0.08, 0.0))
                * Mat4::from_rotation_x(-PI / 2.0)
                * Mat4::from_scale(Vec3::new(rr, rr, 1.0));
            f.transparent[MeshKind::Disc as usize].push(Instance::new(m, lin(SHAFT) * 1.4, a, Vec3::ZERO, SHADE_UNLIT));
        }
        // Rays of it falling slant through the dark, faint, drifting.
        let n = (r * 3.0) as usize + 3;
        for k in 0..n {
            let seed = k as f64 * 3.7 + at.z as f64;
            let a = rnd(seed, 1) * TAU + time * 0.03;
            let rr = r * rnd(seed, 2).sqrt() * 0.85;
            let foot = at + Vec3::new(a.cos() * rr, 0.0, a.sin() * rr);
            let lean = Vec3::new(0.35, 0.0, -0.2);
            let top = foot + Vec3::new(0.0, 9.0, 0.0) + lean * 9.0 * 0.15;
            let w = 0.12 + rnd(seed, 3) * 0.3;
            let flick = 0.6 + 0.4 * (time * 0.4 + k as f32 * 1.7).sin();
            let d = top - foot;
            let m = Mat4::from_scale_rotation_translation(
                Vec3::new(w, d.length(), w * 0.3),
                Quat::from_rotation_arc(Vec3::Y, d.normalize()),
                (foot + top) * 0.5,
            );
            f.additive.push(Instance::new(m, lin(SHAFT) * 0.5, 0.035 * flick, Vec3::ZERO, SHADE_UNLIT));
        }
        // Snow drifting down through it.
        let n = (r * 18.0) as usize;
        for k in 0..n {
            let seed = k as f64 * 2.31 + at.x as f64;
            let fall = 0.35 + rnd(seed, 1) * 0.3;
            let t = ((time * fall / 8.0) + rnd(seed, 2)).fract();
            let a = rnd(seed, 3) * TAU + time * 0.2;
            let rr = r * rnd(seed, 4).sqrt() * 0.95;
            let p = at + Vec3::new(a.cos() * rr + (time * 0.7 + k as f32).sin() * 0.2, 8.0 * (1.0 - t), a.sin() * rr);
            let size = 0.018 + rnd(seed, 5) * 0.014;
            f.additive.push(Instance::new(
                Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(size)),
                lin(SNOW) * 1.6,
                0.8 * (1.0 - t * t),
                Vec3::ZERO,
                SHADE_UNLIT,
            ));
        }
    }

    /// The iron cage hanging from its great chain (swaying, rattled by Krampus, coming down when the
    /// lock breaks, its door swinging open), and the boy in it, frightened, or climbing out and
    /// running to the hero.
    fn emit_cage(&self, at: Vec3, lair: Option<&Lair>, s: &SceneState, time: f32, f: &mut Frame) {
        let cage = lair.map(|l| l.cage).unwrap_or_default();
        let rattled = lair.is_some_and(|l| l.taunt.is_some_and(|t| t.rattling));
        let fear = lair.map(|l| l.child.fear).unwrap_or(0.4);
        // Rattled, the cage shakes; hanging, it turns a little on its chain.
        let shake = if rattled { (time * 31.0).sin() * 0.05 + (time * 23.0).cos() * 0.03 } else { 0.0 };
        let hang = CAGE_HANG * (1.0 - cage.down as f32);
        let sway = if cage.down >= 1.0 { 0.0 } else { (time * 0.6).sin() * 0.025 + shake };
        let root = Mat4::from_translation(at + Vec3::new(0.0, hang, 0.0))
            * Mat4::from_rotation_z(sway)
            * Mat4::from_rotation_x(shake * 0.6)
            * Mat4::from_rotation_y((time * 0.25).sin() * 0.05 * (1.0 - cage.down as f32));
        let out = &mut f.opaque[MeshKind::Cube as usize];
        let iron = lin(IRON);
        let put = |out: &mut Vec<Instance>, m: Mat4, col: Vec3| out.push(Instance::new(root * m, col, 1.0, Vec3::ZERO, SHADE_IRON));
        // The floor of the cage and its hoops.
        put(out, Mat4::from_scale(Vec3::new(CAGE_R * 2.0, 0.08, CAGE_R * 2.0)) * Mat4::from_rotation_y(PI / 8.0), lin(IRON_D));
        let bars = 16;
        let door = (0..3).map(|k| bars / 4 + k).collect::<Vec<_>>();
        for hoop in [0.06f32, CAGE_H * 0.5, CAGE_H] {
            for k in 0..bars {
                let a = (k as f32 + 0.5) * TAU / bars as f32;
                let p = Vec3::new(a.cos() * CAGE_R, hoop, a.sin() * CAGE_R);
                let seg = TAU * CAGE_R / bars as f32;
                put(
                    out,
                    Mat4::from_translation(p) * Mat4::from_rotation_y(-a + PI / 2.0) * Mat4::from_scale(Vec3::new(seg * 1.05, 0.07, 0.07)),
                    iron,
                );
            }
        }
        // The bars, the door's three on their hinge, swung open as the door opens.
        let open = cage.door as f32 * 1.7;
        let hinge = {
            let a = door[0] as f32 * TAU / bars as f32;
            Vec3::new(a.cos() * CAGE_R, 0.0, a.sin() * CAGE_R)
        };
        for k in 0..bars {
            let a = k as f32 * TAU / bars as f32;
            let p = Vec3::new(a.cos() * CAGE_R, CAGE_H * 0.5, a.sin() * CAGE_R);
            let bar = Mat4::from_scale(Vec3::new(0.06, CAGE_H, 0.06));
            if door.contains(&k) {
                let swing = Mat4::from_translation(hinge) * Mat4::from_rotation_y(-open) * Mat4::from_translation(p - hinge);
                put(out, swing * bar, lin(IRON_L));
            } else {
                put(out, Mat4::from_translation(p) * bar, iron);
            }
        }
        // The dome: bars bending in to a ring, and the lock (broken off when it breaks).
        for k in 0..8 {
            let a = k as f32 * TAU / 8.0;
            let from = Vec3::new(a.cos() * CAGE_R, CAGE_H, a.sin() * CAGE_R);
            let to = Vec3::new(a.cos() * 0.18, CAGE_H + 0.7, a.sin() * 0.18);
            let d = to - from;
            let m = Mat4::from_translation((from + to) * 0.5)
                * Mat4::from_quat(Quat::from_rotation_arc(Vec3::Y, d.normalize()))
                * Mat4::from_scale(Vec3::new(0.06, d.length(), 0.06));
            put(out, m, iron);
        }
        put(out, Mat4::from_translation(Vec3::new(0.0, CAGE_H + 0.78, 0.0)) * Mat4::from_scale(Vec3::new(0.32, 0.14, 0.32)), lin(IRON_L));
        let lock = Vec3::new(0.0, CAGE_H * 0.5, CAGE_R + 0.05);
        if !cage.broken {
            put(out, Mat4::from_translation(lock) * Mat4::from_scale(Vec3::new(0.2, 0.26, 0.1)), lin(BRASS));
        }
        // The great chain up into the dark (slack once the cage is down).
        let top = root.transform_point3(Vec3::new(0.0, CAGE_H + 0.85, 0.0));
        if cage.broken && cage.down >= 1.0 {
            chain(out, top, top + Vec3::new(0.3, 0.35, 0.2), 0.32, lin(IRON_D));
        } else {
            chain(out, top, Vec3::new(at.x, CHAIN_TOP, at.z), 0.32, lin(IRON_D));
        }
        // The boy.
        let Some(l) = lair else {
            self.emit_kid(f, 0, kid_root(root.transform_point3(Vec3::new(0.0, 0.05, 0.0)), 0.0), Act::Scared(0.4), time, 0.0, Vec3::ZERO);
            return;
        };
        let c = l.child;
        let yaw = (c.facing.x as f32).atan2(c.facing.y as f32);
        match c.state {
            ChildState::Caged => {
                // At the bars toward the hero, frightened; crying while the cage is rattled.
                let hp = s.hero.pos();
                let look = Vec3::new(hp.x as f32 - at.x, 0.0, hp.y as f32 - at.z);
                let yaw = look.x.atan2(look.z);
                let near_bars = Vec3::new(yaw.sin(), 0.0, yaw.cos()) * (0.45 - 0.35 * fear);
                let act = if c.cry > 0.0 || rattled { Act::Scared(1.0) } else { Act::Scared(0.25 + 0.35 * fear) };
                let feet = root.transform_point3(Vec3::new(near_bars.x, 0.05, near_bars.z));
                self.emit_kid(f, 0, kid_root(feet, yaw), act, time, 0.0, Vec3::ZERO);
            }
            ChildState::Climbing => {
                let hop = (c.age as f32 / crate::game::lair::CLIMB as f32 * PI).sin() * 0.3;
                let feet = Vec3::new(c.pos.x as f32, hop, c.pos.y as f32);
                self.emit_kid(f, 0, kid_root(feet, yaw), Act::Run { phase: time * 6.0, pace: 0.5 }, time, 0.0, Vec3::ZERO);
            }
            ChildState::Running => {
                let feet = Vec3::new(c.pos.x as f32, 0.0, c.pos.y as f32);
                self.emit_kid(f, 0, kid_root(feet, yaw), Act::Run { phase: time * 9.0, pace: 1.0 }, time, 0.0, Vec3::ZERO);
            }
            ChildState::Held => {
                let feet = Vec3::new(c.pos.x as f32, 0.0, c.pos.y as f32);
                let glow = lin(0xfff4d8) * (0.15 * (c.age as f32).min(1.0));
                self.emit_kid(f, 0, kid_root(feet, yaw), Act::Scared(0.55), time, 0.0, glow);
            }
        }
    }

    /// Krampus as he fights: the coal glowing in his sack's mouth and his eyes' light, his bells
    /// glinting as he tolls, his chain cracking out along its line or sweeping round him, snow
    /// thrown up from his hooves as he charges.
    fn emit_krampus_fight(&self, i: usize, d: &Duelist, time: f32, f: &mut Frame) {
        let Some(v) = self.enemy_views.get(i) else { return };
        let rig = self.models.rig(v.rig);
        let root = enemy_root(v);
        let p = Vec3::new(d.pos.x as f32, 0.0, d.pos.y as f32);
        let ahead = Vec3::new(d.facing.x as f32, 0.0, d.facing.y as f32).normalize_or_zero();
        // His key light (cold, before him) and the coal's red glow at his hip.
        f.lights.push(Light { pos: p + ahead * 2.4 + Vec3::new(0.0, 3.6, 0.0), color: lin(SHAFT) * 4.0, distance: 7.0 });
        let sack = rig.index("sack").unwrap_or(0);
        let coal = posed(rig, &v.pose, root, sack, pivot(KRAMPUS_RIG, "sack"), [32.0, 50.0, 42.0]);
        f.lights.push(Light { pos: coal, color: lin(EMBER) * 0.9, distance: 1.8 });
        glow_orb(f, coal, 0.1, lin(EMBER), 0.14);
        // His eyes.
        let head = rig.index("head").unwrap_or(0);
        for ex in [45.0f32, 55.0] {
            let e = posed(rig, &v.pose, root, head, pivot(KRAMPUS_RIG, "head"), [ex, 98.0, 66.0]);
            glow_orb(f, e, 0.1, lin(0xff3a1a), 0.6);
        }
        let Some((mv, phase, frame)) = d.f.attack() else { return };
        let data = d.data(mv);
        let aim = Vec3::new(d.aim.x as f32, 0.0, d.aim.y as f32).normalize_or_zero();
        let arm = rig.index("armL").unwrap_or(0);
        let hand = posed(rig, &v.pose, root, arm, pivot(KRAMPUS_RIG, "armL"), [77.0, 46.0, 63.0]);
        match (mv, phase) {
            (MoveId::TollOfBells, Phase::Startup) => {
                // His bells shaken: glints dancing on each.
                for b in &KRAMPUS_BELLS[..6] {
                    let at = posed(rig, &v.pose, root, 0, body_pivot(), *b);
                    let k = (time * 40.0 + b[0]).sin().abs();
                    glow_orb(f, at, 0.12, lin(BRASS_L), 0.25 + 0.4 * k);
                }
            }
            (MoveId::ChainLash, Phase::Active) => {
                // The chain cracking out along the line, link by link, to its end.
                let span = d.lore.span as f32;
                let u = ((frame - data.startup) as f32 / 3.0).min(1.0);
                let end = p + aim * span * u + Vec3::new(0.0, 0.25, 0.0);
                chain(&mut f.opaque[MeshKind::Cube as usize], hand, end, 0.3, lin(IRON_L));
                hung_bell(&mut f.opaque[MeshKind::Cube as usize], end + Vec3::new(0.0, 0.2, 0.0), 0.16, 0.0, 0.3);
                glow_orb(f, end, 0.5, lin(ICE_L), 0.35);
            }
            (MoveId::ChainSweep, Phase::Active) => {
                // The chain swung round him at arm's length: an arc of links trailing the hand.
                let reach = data.reach as f32;
                let u = (frame - data.startup) as f32 / data.active as f32;
                let base = aim.z.atan2(aim.x);
                let half = data.half_angle as f32;
                let now = base + half - 2.0 * half * u;
                for k in 0..6 {
                    let a = now + k as f32 * 0.12;
                    let q = p + Vec3::new(a.cos(), 0.0, a.sin()) * reach + Vec3::new(0.0, 0.6, 0.0);
                    chain(&mut f.opaque[MeshKind::Cube as usize], hand, q, 0.3, lin(IRON_L) * (1.0 - k as f32 * 0.12));
                    if k > 0 {
                        break;
                    }
                }
            }
            (MoveId::HoofCharge, Phase::Active) => {
                // Snow kicked up behind his hooves.
                for k in 0..10 {
                    let t = ((time * 3.0 + k as f32 * 0.1) % 1.0 + 1.0) % 1.0;
                    let q =
                        p - aim * (0.6 + t * 1.6) + Vec3::new(((k * 7) % 5) as f32 * 0.1 - 0.2, t * 0.8, ((k * 3) % 5) as f32 * 0.1 - 0.2);
                    f.additive.push(Instance::new(
                        Mat4::from_translation(q) * Mat4::from_scale(Vec3::splat(0.06 * (1.0 - t) + 0.02)),
                        lin(SNOW) * 1.2,
                        0.7 * (1.0 - t),
                        Vec3::ZERO,
                        SHADE_UNLIT,
                    ));
                }
            }
            _ => {}
        }
    }

    /// His slow death: he sinks to his knees in the snow, his head bowed, his bells falling silent
    /// one by one (each still ringing glints), then slumps forward and is gone into the snow in a
    /// breath of black smoke.
    fn emit_krampus_fall(&self, fall: crate::game::lair::Fall, time: f32, f: &mut Frame) {
        let rig = &self.models.krampus_fight;
        let mut pose = Pose::rest(rig);
        let t = fall.age as f32;
        let kneel = (t / 0.8).min(1.0);
        let slump = ((t - BELL_EVERY as f32 * 7.0) / 1.2).clamp(0.0, 1.0);
        let gone = ((t - LOCK_BREAKS as f32) / 1.6).clamp(0.0, 1.0);
        let idx = |n: &str| rig.index(n).unwrap_or(0);
        pose.rot[idx("legR")].x = -1.3 * kneel;
        pose.rot[idx("legL")].x = -1.1 * kneel;
        pose.rot[idx("armR")].x = 0.3 * kneel + 0.5 * slump;
        pose.rot[idx("armL")].x = 0.2 * kneel + 0.6 * slump;
        pose.rot[idx("head")].x = 0.5 * kneel + 0.4 * slump + (time * 2.0).sin() * 0.02 * (1.0 - slump);
        pose.rot[idx("sack")].x = 0.4 * kneel;
        let yaw = (fall.facing.x as f32).atan2(fall.facing.y as f32);
        let at = Vec3::new(fall.at.x as f32, 0.0, fall.at.y as f32);
        let sink = 0.55 * kneel + 0.5 * gone;
        let root = Mat4::from_translation(at - Vec3::new(0.0, sink, 0.0))
            * Mat4::from_rotation_y(yaw)
            * Mat4::from_rotation_x(0.25 * kneel + 0.9 * slump);
        let alpha = 1.0 - gone;
        if alpha > 0.01 {
            let dark = Vec3::splat(1.0 - 0.4 * slump);
            f.rigs.push(rig, &pose, root, dark, alpha, Vec3::ZERO, rig_look::FADED, 0.0);
        }
        // His bells: those still ringing glint, one by one going dark.
        for (k, b) in KRAMPUS_BELLS[..6].iter().enumerate() {
            if k as u8 >= fall.bells || alpha < 0.05 {
                continue;
            }
            let p = posed(rig, &pose, root, 0, body_pivot(), *b);
            let ring = (time * 30.0 + k as f32).sin().abs();
            glow_orb(f, p, 0.14, lin(BRASS_L), (0.3 + 0.4 * ring) * alpha);
        }
        // Black smoke and the last embers of his coal, as he goes into the snow.
        if t > LOCK_BREAKS as f32 - 0.6 {
            for k in 0..24 {
                let seed = k as f64 * 1.7;
                let u = ((time * 0.4 + rnd(seed, 1)) % 1.0 + 1.0) % 1.0;
                let a = rnd(seed, 2) * TAU;
                let q = at + Vec3::new(a.cos() * 0.8 * u, 0.3 + u * 3.0, a.sin() * 0.8 * u);
                let sz = 0.18 + u * 0.4;
                let fade = (1.0 - u) * (1.0 - ((t - LOCK_BREAKS as f32 - 2.0) / 2.0).clamp(0.0, 1.0));
                f.transparent[MeshKind::Sphere as usize].push(Instance::new(
                    Mat4::from_translation(q) * Mat4::from_scale(Vec3::splat(sz)),
                    lin(0x0a080a),
                    0.35 * fade,
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
                if k % 3 == 0 {
                    f.additive.push(Instance::new(
                        Mat4::from_translation(q * 0.6 + at * 0.4) * Mat4::from_scale(Vec3::splat(0.04)),
                        lin(EMBER_HOT) * 2.0,
                        0.8 * fade,
                        Vec3::ZERO,
                        SHADE_UNLIT,
                    ));
                }
            }
        }
    }

    /// His coals (his traps): flung from his hand in an arc, glowing hotter on the floor as their
    /// fuse burns, bursting into flame, and burning on in a bed of embers while they linger.
    pub(super) fn emit_coals(&self, traps: &[Trap], grid: &Grid, time: f32, f: &mut Frame) {
        for t in traps {
            let ground = floor_at(grid, t.pos.x as f32, t.pos.y as f32);
            let at = Vec3::new(t.pos.x as f32, ground, t.pos.y as f32);
            let seed = (t.pos.x * 3.7 + t.pos.y * 1.9) as f32;
            let r = t.radius as f32;
            if t.flight > 0 {
                // In the air: a coal arcing from his hand, trailing sparks.
                let from = Vec3::new(t.from.x as f32, ground + 1.6, t.from.y as f32);
                let k = 1.0 - t.flight as f32 / (COIN_FLIGHT as f32 + 1.0);
                let q = from.lerp(at, k) + Vec3::new(0.0, (k * PI).sin() * 1.2, 0.0);
                glowing(
                    &mut f.opaque[MeshKind::Cube as usize],
                    q,
                    Vec3::splat(0.14),
                    Quat::from_rotation_y(time * 9.0 + seed),
                    lin(COAL),
                    lin(EMBER) * 1.6,
                    SHADE_LIT,
                );
                glow_orb(f, q, 0.25, lin(EMBER), 0.5);
                continue;
            }
            match t.burst {
                None => {
                    // On the floor: the coal glowing hotter as its fuse burns, the snow melting
                    // round it in a dark ring that shows how far it will burn.
                    let u = t.tell();
                    let pulse = 0.6 + 0.4 * (time * (6.0 + 10.0 * u)).sin().abs();
                    glowing(
                        &mut f.opaque[MeshKind::Cube as usize],
                        at + Vec3::new(0.0, 0.07, 0.0),
                        Vec3::new(0.22, 0.12, 0.2),
                        Quat::from_rotation_y(seed),
                        lin(COAL),
                        lin(EMBER).lerp(lin(EMBER_HOT), u) * (0.8 + 1.6 * u) * pulse,
                        SHADE_LIT,
                    );
                    let m = Mat4::from_translation(at + Vec3::new(0.0, 0.02, 0.0))
                        * Mat4::from_rotation_x(-PI / 2.0)
                        * Mat4::from_scale(Vec3::new(r, r, 1.0));
                    f.transparent[MeshKind::Disc as usize].push(Instance::new(
                        m,
                        lin(EMBER) * (0.4 + 1.2 * u),
                        0.25 + 0.35 * u,
                        Vec3::ZERO,
                        SHADE_UNLIT,
                    ));
                    let m = Mat4::from_translation(at + Vec3::new(0.0, 0.025, 0.0))
                        * Mat4::from_rotation_x(-PI / 2.0)
                        * Mat4::from_scale(Vec3::new(r, r, 1.0));
                    f.transparent[MeshKind::Annulus as usize].push(Instance::new(
                        m,
                        lin(EMBER_HOT) * 1.6,
                        0.5 + 0.4 * u,
                        Vec3::ZERO,
                        SHADE_UNLIT,
                    ));
                    f.lights.push(Light { pos: at + Vec3::new(0.0, 0.4, 0.0), color: lin(EMBER) * (0.6 + 2.0 * u), distance: 2.0 });
                }
                Some(b) => {
                    let left = if t.linger > 0 { 1.0 - b as f32 / t.linger as f32 } else { 1.0 - b as f32 / 26.0 };
                    if b < 20 {
                        // The burst: flame thrown up.
                        let u = b as f32 / 20.0;
                        for k in 0..10 {
                            let a = k as f32 * 2.39 + seed;
                            let q = at
                                + Vec3::new(
                                    a.cos() * r * 0.5 * u,
                                    0.2 + u * 1.4 * (0.6 + 0.4 * ((k * 7 % 5) as f32 / 5.0)),
                                    a.sin() * r * 0.5 * u,
                                );
                            let s = 0.18 * (1.0 - u) + 0.05;
                            f.additive.push(Instance::new(
                                Mat4::from_translation(q) * Mat4::from_scale(Vec3::new(s, s * 1.8, s)),
                                lin(FLAME).lerp(lin(FLAME_TIP), u) * 2.2,
                                0.9 * (1.0 - u),
                                Vec3::ZERO,
                                SHADE_UNLIT,
                            ));
                        }
                    }
                    if t.lingering() || b < 4 {
                        // The bed of embers burning on, its flames low, dying as it ends.
                        let m = Mat4::from_translation(at + Vec3::new(0.0, 0.02, 0.0))
                            * Mat4::from_rotation_x(-PI / 2.0)
                            * Mat4::from_scale(Vec3::new(r, r, 1.0));
                        f.transparent[MeshKind::Disc as usize].push(Instance::new(
                            m,
                            lin(EMBER) * 1.3,
                            0.6 * left.max(0.2),
                            Vec3::ZERO,
                            SHADE_UNLIT,
                        ));
                        coal_flames(f, at + Vec3::new(0.0, 0.05, 0.0), 0.7 * left.max(0.3), time, seed);
                        for k in 0..4 {
                            let a = k as f32 * 1.7 + seed;
                            let q = at + Vec3::new(a.cos() * r * 0.5, 0.05, a.sin() * r * 0.5);
                            glowing(
                                &mut f.opaque[MeshKind::Cube as usize],
                                q,
                                Vec3::new(0.14, 0.08, 0.12),
                                Quat::from_rotation_y(a),
                                lin(COAL),
                                lin(EMBER) * left * 1.6,
                                SHADE_LIT,
                            );
                        }
                        f.lights.push(Light { pos: at + Vec3::new(0.0, 0.5, 0.0), color: lin(EMBER) * 2.6 * left.max(0.2), distance: 2.6 });
                    }
                }
            }
        }
    }

    /**
     * Krampus's poses over the duel's (`duel::update_duelist`): his birch lashed across, his chain
     * swept round and cracked out along its line, his bells shaken high and a hoof stamped, his
     * horns lowered for the charge, coal flung from his sack, the sack swung up over his head and
     * brought down, the bars of the cage gripped and shaken, and dazed, his head lolling.
     */
    pub(super) fn pose_krampus(&mut self, i: usize, d: &Duelist, time: f32) {
        let rig = self.models.rig(self.enemy_views[i].rig);
        let v = &mut self.enemy_views[i];
        let ease = |u: f32| {
            let u = u.clamp(0.0, 1.0);
            u * u * (3.0 - 2.0 * u)
        };
        if let Some(sk) = rig.index("sack") {
            // (The sack hangs at his hip but while he swings it.)
            v.pose.pos[sk] = rig.parts[sk].rest;
        }
        let set_x = |v: &mut EnemyView, name: &str, x: f32| v.pose.rot_x(rig, name, x);
        let set_z = |v: &mut EnemyView, name: &str, z: f32| {
            if let Some(k) = rig.index(name) {
                v.pose.rot[k].z = z;
            }
        };
        match d.f.state {
            State::Attack { mv, frame } => {
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
                v.root_rot = Vec3::ZERO;
                v.root_scale = Vec3::ONE;
                match mv {
                    MoveId::BirchLash => {
                        // The birch raised high and back over his shoulder, then lashed down across him.
                        let arm = if strike == 0.0 { -2.6 * wind } else { (-2.6 + 3.0 * snap) * out };
                        set_x(v, "armR", arm);
                        set_z(v, "armR", if strike == 0.0 { -0.5 * wind } else { (-0.5 + 1.1 * snap) * out });
                        v.root_rot.y = if strike == 0.0 { -0.45 * wind } else { (-0.45 + 1.0 * snap) * out };
                        v.root_rot.x = if strike == 0.0 { -0.08 * wind } else { 0.2 * snap * out };
                        set_x(v, "head", 0.15 * snap * out - 0.2 * wind * (1.0 - snap));
                    }
                    MoveId::ChainSweep => {
                        // Wound round, the chain arm drawn back; then swung right round at arm's length.
                        set_z(v, "armL", (1.2 * wind + 0.2 * snap) * out);
                        set_x(v, "armL", -0.6 * wind * out);
                        v.root_rot.y = if strike == 0.0 { 0.9 * wind } else { (0.9 - 2.3 * snap) * out };
                    }
                    MoveId::TollOfBells => {
                        // Both arms flung up, the bells shaken hard; then a hoof stamped and the floor shakes.
                        let up = if strike == 0.0 { -2.7 * wind } else { (-2.7 + 2.2 * snap) * out };
                        let jitter = if strike == 0.0 { (time * 38.0).sin() * 0.12 * wind } else { 0.0 };
                        set_x(v, "armR", up + jitter);
                        set_x(v, "armL", up - jitter);
                        set_x(v, "head", -0.45 * wind * (1.0 - snap) + 0.2 * snap * out);
                        set_x(v, "legR", if strike == 0.0 { -0.6 * wind } else { -0.6 * (1.0 - snap) * out });
                        v.root_rot.z = jitter * 0.3;
                        if strike > 0.0 {
                            v.root_scale = Vec3::new(1.0 + 0.06 * snap * out, 1.0 - 0.08 * snap * out, 1.0 + 0.06 * snap * out);
                        }
                    }
                    MoveId::ChainLash => {
                        // The chain arm raised overhead and back, leaning back; then whipped forward.
                        let arm = if strike == 0.0 { -2.9 * wind } else { (-2.9 + 1.6 * snap) * out };
                        set_x(v, "armL", arm);
                        set_x(v, "armR", 0.3 * wind * out);
                        v.root_rot.x = if strike == 0.0 { -0.18 * wind } else { (-0.18 + 0.4 * snap) * out };
                        set_x(v, "head", -0.25 * wind * (1.0 - snap) + 0.25 * snap * out);
                    }
                    MoveId::HoofCharge => {
                        // Head down, horns forward, a hoof pawing the snow; then the gallop.
                        let lower = if rec > 0.0 { 0.7 * out } else { 0.7 * wind };
                        set_x(v, "head", lower);
                        v.root_rot.x = 0.32 * wind * out;
                        if strike == 0.0 {
                            set_x(v, "legR", -0.4 * (time * 9.0).sin().abs() * wind);
                        } else if rec == 0.0 {
                            let g = (time * 16.0).sin();
                            set_x(v, "legR", g * 0.9);
                            set_x(v, "legL", -g * 0.9);
                            set_x(v, "armR", -g * 0.5 + 0.3);
                            set_x(v, "armL", g * 0.5 + 0.3);
                        }
                    }
                    MoveId::CoalFire => {
                        // His hand in the sack, the sack lifted; then the coals flung out before him.
                        if strike == 0.0 {
                            set_x(v, "armR", 0.35 * wind);
                            set_z(v, "armR", 0.35 * wind);
                            set_x(v, "sack", -0.5 * wind);
                            v.root_rot.y = -0.35 * wind;
                        } else {
                            set_x(v, "armR", -1.7 * snap * out);
                            set_z(v, "armR", 0.35 * (1.0 - snap) * out);
                            set_x(v, "sack", -0.5 * (1.0 - snap) * out);
                            v.root_rot.y = (-0.35 + 0.6 * snap) * out;
                        }
                    }
                    MoveId::SackSwing => {
                        // He takes the sack off his hip by its neck, swings it up over his head,
                        // and brings it down before him.
                        let arm = if strike == 0.0 { -2.8 * wind } else { (-2.8 + 2.4 * snap) * out };
                        set_x(v, "armR", arm);
                        if let (Some(r), Some(sk)) = (rig.index("armR"), rig.index("sack")) {
                            let fist = (Vec3::from(RIGHT_FIST) - Vec3::from(pivot(KRAMPUS_RIG, "armR"))) * rig.size;
                            let held = v.pose.pos[r] + euler(v.pose.rot[r]).transform_vector3(fist);
                            let k = ease(wind * 1.6).min(ease(out * 1.6));
                            v.pose.pos[sk] = rig.parts[sk].rest.lerp(held, k);
                            v.pose.rot[sk].x = arm * k - 0.4 * snap * out * k;
                        }
                        v.root_rot.x = if strike == 0.0 { -0.15 * wind } else { (-0.15 + 0.4 * snap) * out };
                        if strike > 0.0 {
                            v.root_scale = Vec3::new(1.0 + 0.05 * snap * out, 1.0 - 0.07 * snap * out, 1.0 + 0.05 * snap * out);
                        }
                    }
                    MoveId::CageRattle => {
                        // Both fists on the bars, shaking them; his head thrown back, laughing.
                        let grip = wind;
                        let shake = if f >= s { (time * 30.0).sin() * 0.12 } else { 0.0 };
                        set_x(v, "armR", -1.45 * grip + shake);
                        set_x(v, "armL", -1.45 * grip - shake);
                        set_x(v, "head", -0.35 * grip + (time * 9.0).sin().abs() * -0.15 * (f >= s) as i32 as f32);
                        v.root_rot.z = shake * 0.15;
                        v.root_rot.x = 0.12 * grip;
                    }
                    _ => {}
                }
            }
            State::Reel { kind: Reel::Staggered | Reel::GuardBroken, .. } => {
                // Dazed: his head lolling, his arms hanging slack, swaying on his hooves.
                set_x(v, "head", 0.55 + (time * 3.0).sin() * 0.15);
                set_z(v, "head", (time * 2.3).sin() * 0.2);
                set_x(v, "armR", 0.25);
                set_x(v, "armL", 0.25);
                v.root_rot.z = (time * 4.0).sin() * 0.08;
            }
            State::Reel { kind: Reel::Deflected, left, total } => {
                // Parried: thrown back, his arms flung wide.
                let k = left as f32 / total.max(1) as f32;
                set_z(v, "armR", -0.9 * k);
                set_z(v, "armL", 0.9 * k);
                v.root_rot.x = -0.3 * k;
            }
            _ => {}
        }
    }
}

/// A brazier's (or his coals') flames: tongues of fire flickering up from `p`.
fn coal_flames(f: &mut Frame, p: Vec3, size: f32, time: f32, seed: f32) {
    for k in 0..7 {
        let t = (time * 2.0 + seed * 0.37 + k as f32 * 0.143).fract();
        let a = seed * 1.9 + k as f32 * 1.3;
        let spread = 0.18 * size * (1.0 - t);
        let q = p + Vec3::new(a.cos() * spread, (0.05 + t * 0.6) * size, a.sin() * spread);
        let s = (0.18 * (1.0 - t) + 0.04) * size;
        let c = lin(FLAME).lerp(lin(FLAME_TIP), t);
        f.additive.push(Instance::new(
            Mat4::from_translation(q) * Mat4::from_scale(Vec3::new(s, s * 1.8, s)),
            c * 1.4,
            0.65 * (1.0 - t),
            Vec3::ZERO,
            SHADE_UNLIT,
        ));
    }
}

/// An enemy's root transform as its view draws it.
fn enemy_root(v: &EnemyView) -> Mat4 {
    Mat4::from_translation(v.group_pos) * Mat4::from_rotation_y(v.yaw) * euler(v.root_rot) * Mat4::from_scale(v.root_scale)
}

/// Where Krampus's body (part 0) turns about in his grid.
fn body_pivot() -> [f32; 3] {
    let (w, _, d) = KRAMPUS_SIZE;
    [(w - 1) as f32 / 2.0, 0.0, (d - 1) as f32 / 2.0]
}

/// The joint of `name` in `defs`.
fn pivot(defs: &[crate::render::voxels::PartDef], name: &str) -> [f32; 3] {
    defs.iter().find(|d| d.name == name).map(|d| d.pivot).unwrap_or([0.0; 3])
}

/// A point of a model's voxel grid, posed: `p` lies in part `part`, whose joint is at grid point `pivot`.
fn posed(rig: &Rig, pose: &Pose, root: Mat4, part: usize, pivot: [f32; 3], p: [f32; 3]) -> Vec3 {
    let local = (Vec3::from(p) - Vec3::from(pivot)) * rig.size;
    (root * Mat4::from_translation(pose.pos[part]) * euler(pose.rot[part])).transform_point3(local)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_snow_lies_deeper_against_the_wall_than_in_the_middle() {
        let middle: f32 = (0..20).map(|k| snow_at(CENTRE.0 as f32 + k as f32 * 0.1, CENTRE.1 as f32)).sum();
        let wall: f32 = (0..20).map(|k| snow_at(CENTRE.0 as f32 + RADIUS as f32 - 0.5, CENTRE.1 as f32 + k as f32 * 0.1)).sum();
        assert!(wall > middle * 1.6, "{wall} against the wall, {middle} in the middle");
    }
}
