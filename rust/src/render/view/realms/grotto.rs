//! The Serpent Grotto as the view draws it (Envy's realm; laid out in `logic::realms::grotto`): a
//! flooded sea-cave cathedral of wet black basalt, bleached serpent bone, verdigris bronze and
//! nacre, its light coming from below.
//!
//! - **The Waterline** (where he arrives) is cold and dim: shelves of wet basalt standing in jade
//!   water, rib-bridges of dead sea-serpents, bone rafts adrift, algae glowing along the edges of
//!   the deep water.
//! - **The Drowned Floor** is drawn twice. Beneath the Waterline it shows through the water, two
//!   storeys down, warm and gold-green with lanterns, shallower and shifted toward the camera by
//!   the refraction (so what he dives for lies beyond where it showed). Where it really is, off to
//!   the east of the map, he walks it, the Waterline's surface rippling overhead, its rafts and
//!   ribs dark against it.
//! - The other side always looks richer: phantom chests and beads lie under the water and phantom
//!   gold is heaped on the rafts seen from below. A phantom is drawn at the wrong depth, just
//!   under the surface, on the line from where the camera stood a moment ago: standing still, it
//!   sits on the floor below; moving, it slides with the surface. It casts no shadow in the caustics.
//! - Leviathan's coils hump through the deep water and slide across the passages below; the Eye
//!   opens in the water and sweeps its cone; Kiyohime's bell hangs over the black pool, and drops,
//!   coiled, heating from bronze to white.

mod build;
mod decor;
mod emit;

use super::super::*;
use crate::logic::realms::grotto::*;
use decor::GrottoArt;

/// How brightly his lantern burns here (the grotto's light is mostly its own, from below).
pub(super) const LANTERN: f32 = 0.4;

/// The Waterline's surface, the top of its shelves, how far down a shelf goes, and its walls' height.
const WATER_Y: f32 = -0.03;
const SHELF_Y: f32 = -0.1;
const SHELF_DEPTH: f32 = 1.3;
const WALL_H: f32 = 1.5;
/// The Drowned Floor's walls.
const DROWNED_WALL: f32 = 1.4;
/// How far beneath the Waterline its drowned twin shows (it lies deeper; water makes it look shallower).
const DEPTH: f32 = 2.6;
/// Per-frame scenery is drawn only this near him (tiles).
const RANGE: f32 = 22.0;
/// Voxel materials as shade modes (kind + 5): wet and glossy, ivory, bronze, nacre, lit from within.
const SHADE_WET: f32 = 14.0;
const SHADE_BONE: f32 = 11.0;
const SHADE_BRONZE: f32 = 13.0;
const SHADE_NACRE: f32 = 15.0;
const SHADE_LIGHT: f32 = 16.0;

mod pal {
    pub const BASALT: u32 = 0x283236;
    pub const BASALT_L: u32 = 0x384448;
    pub const BASALT_D: u32 = 0x192024;
    pub const STRATA: u32 = 0x45524e;
    pub const SHELF: u32 = 0x323c40;
    pub const SHELF_L: u32 = 0x445054;
    pub const ALGAE: u32 = 0x3ad06a;
    pub const ALGAE_D: u32 = 0x1c6a3c;
    /// The algae along the waterline: a dim verdigris, so the deep's edge reads without glaring.
    pub const FRINGE: u32 = 0x3a6a5a;
    pub const VERDIGRIS: u32 = 0x4aa088;
    pub const VERDIGRIS_D: u32 = 0x2e6a5a;
    pub const BRONZE: u32 = 0xa8743a;
    pub const BONE: u32 = 0xe6dcc0;
    pub const BONE_D: u32 = 0xb8ac8a;
    pub const NACRE: u32 = 0xeae2f2;
    pub const NACRE_P: u32 = 0xf0c8dc;
    pub const FLOOR_GOLD: u32 = 0x806a44;
    pub const FLOOR_JADE: u32 = 0x46604f;
    pub const FLOOR_SAND: u32 = 0x665a46;
    pub const GOLD: u32 = 0xf2c53d;
    pub const CORAL: u32 = 0xd8705a;
    pub const CORAL_L: u32 = 0xf0a080;
    pub const KELP: u32 = 0x2c6e36;
    pub const WATER: u32 = 0x1c4a44;
    pub const WATER_BELOW: u32 = 0x2e6a5e;
    pub const LAMP: u32 = 0xf2d68a;
    pub const EYE: u32 = 0xc8f05a;
    pub const NAZAR: u32 = 0x1e48c8;
    pub const NAZAR_L: u32 = 0x7ab8ff;
}
use pal::*;

/// What the view keeps of the realm: the models, the lanterns and algae that light it (where they
/// are, and where they show through the water), the rib-bridges and rafts to draw over the Drowned
/// Floor, where the camera stood a moment ago (for the phantoms), how faded the hanging bell is,
/// and where he was last frame.
pub(in crate::render::view) struct GrottoView {
    art: Box<GrottoArt>,
    lamps: Vec<Vec3>,
    copy_lamps: Vec<Vec3>,
    algae: Vec<Vec3>,
    ribs: Vec<(Vec3, bool)>,
    anchor: Vec3,
    bell_fade: f32,
    last_hero: Vec2,
}

/// Where the Drowned Floor's copy beneath the Waterline is drawn, from where it really is.
fn copy_offset() -> Vec3 {
    Vec3::new(-OX as f32, -DEPTH, REFRACTION as f32)
}

/// Where the Waterline's copy over the Drowned Floor is drawn, from where it really is.
fn above_offset() -> Vec3 {
    Vec3::new(OX as f32, DEPTH - WATER_Y, 0.0)
}

fn cube(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot_y: f32, color: Vec3, shade: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(rot_y), pos), color, 1.0, Vec3::ZERO, shade));
}

/// A box that glows a little of its own colour.
fn cube_glow(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot: Quat, color: Vec3, glow: f32, shade: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(scale, rot, pos), color, 1.0, color * glow, shade));
}

fn basalt(h: f64) -> Vec3 {
    let set = [BASALT, BASALT, BASALT_L, BASALT_D];
    lin(set[(h * 4.0) as usize % 4]) * (0.92 + h as f32 * 0.16)
}

impl WorldView {
    /// The realm's static geometry.
    pub(super) fn build_grotto(&mut self, grid: &Grid, r: &Grotto) -> GrottoView {
        let art = Box::new(GrottoArt::new());
        let built = self.build_grotto_static(grid, r, &art);
        GrottoView {
            art,
            lamps: built.lamps,
            copy_lamps: built.copy_lamps,
            algae: built.algae,
            ribs: built.ribs,
            anchor: Vec3::new(r.base.arrival.x as f32, 8.0, r.base.arrival.y as f32 + 5.0),
            bell_fade: 1.0,
            last_hero: Vec2::new(r.base.arrival.x as f32, r.base.arrival.y as f32),
        }
    }

    /// The realm's own animation, one step: the camera cuts across a crossing (rather than flying
    /// over the map), the phantoms' anchor trails the camera, the hanging bell fades out of his way.
    pub(super) fn update_grotto_view(&mut self, v: &mut GrottoView, r: &Grotto, s: &SceneState, dt: f32) {
        let hp = s.hero.pos();
        let here = Vec2::new(hp.x as f32, hp.y as f32);
        if (here - v.last_hero).length() > 6.0 {
            self.initialized = false;
            v.anchor = self.cam_pos;
        }
        v.last_hero = here;
        v.anchor = v.anchor.lerp(self.cam_pos, 1.0 - (-1.1 * dt).exp());
        let c = r.bell.center;
        let near = (here - Vec2::new(c.x as f32, c.y as f32)).length() < 5.5 && layer_of(s.hero.occupied()) == Layer::Waterline;
        let goal = if near { 0.4 } else { 1.0 };
        v.bell_fade += (goal - v.bell_fade).clamp(-dt * 3.0, dt * 3.0);
    }

    /// What moves, glows and lights in the realm.
    pub(super) fn emit_grotto(&self, v: &GrottoView, r: &Grotto, s: &SceneState, time: f32, hero: Vec3, f: &mut Frame) {
        self.emit_grotto_frame(v, r, s, time, hero, f);
    }
}

/// The realm's sky, fog and light: cold and dim on the Waterline, the light from below; warmer on
/// the Drowned Floor, under the green of the water.
pub(super) fn atmosphere(_v: &GrottoView, _r: &Grotto, s: &SceneState) -> Atmosphere {
    let drowned = layer_of(s.hero.occupied()) == Layer::Drowned;
    if drowned {
        Atmosphere {
            background: 0x06140f,
            fog_near: 13.0,
            fog_far: 36.0,
            hemi_sky: lin(0x5a9a80) * 0.45,
            hemi_ground: lin(0xb0905a) * 0.55,
            ambient: lin(0x2c4438) * 0.8,
            sun_dir: Vec3::new(-0.3, 1.0, 0.3).normalize(),
            sun: lin(0xd0e8c0) * 0.55,
            exposure: 1.2,
            night_sky: 0.0,
            bloom: 1.1,
            moon_view: Vec3::new(0.0, 0.3, -1.0).normalize(),
            cloud_tint: lin(0x061410),
        }
    } else {
        Atmosphere {
            background: 0x030a0a,
            fog_near: 15.0,
            fog_far: 42.0,
            // Cold mist teal from above, a little amber off the drowned floor below.
            hemi_sky: lin(0x7ea49c) * 0.62,
            hemi_ground: lin(0x8a7a50) * 0.45,
            ambient: lin(0x2c3a3a) * 0.95,
            sun_dir: Vec3::new(-0.35, 1.0, 0.25).normalize(),
            sun: lin(0xb8d0cc) * 0.85,
            exposure: 1.25,
            night_sky: 0.0,
            bloom: 0.95,
            moon_view: Vec3::new(0.0, 0.3, -1.0).normalize(),
            cloud_tint: lin(0x030a0a),
        }
    }
}
