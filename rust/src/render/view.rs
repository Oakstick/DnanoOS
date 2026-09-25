//! The voxel dungeon as a scene: everything the old Three.js renderer did,
//! minus the GPU. Grid x maps to world x, grid y to world z, and floor
//! elevation to world y in steps of STEP. Each frame it emits instanced
//! draws and a light list; `gpu.rs` turns those into pixels.

use std::collections::HashMap;
use std::f32::consts::{PI, TAU};

use bytemuck::{Pod, Zeroable};
use glam::{EulerRot, Mat4, Quat, Vec2, Vec3, Vec4};

use super::characters;
use super::voxels::*;
use crate::logic::enemy::{Enemy, EnemyKind, EnemyMode};
use crate::logic::grid::Grid;
use crate::logic::hero::Hero;
use crate::logic::items::{Bomb, Diamond, DiamondKind, Potion, RangedKind};
use crate::logic::projectile::Projectile;
use crate::logic::rng::Rng;
use crate::logic::systems::{Frenzy, PuddleField, PUDDLE_SECONDS};
use crate::logic::types::{js_round, Dir, Tile, CARDINALS, P};

mod forge;
mod meadow;
pub use forge::SMITH_BEAT;
pub use meadow::meadow_atmosphere;

/// Moonlit forest palette. Walls are undergrowth and trees; higher floors read a little lighter.
pub mod colors {
    pub const BACKGROUND: u32 = 0x06100b;
    pub const STONE_A: u32 = 0x2b5a2a;
    pub const STONE_B: u32 = 0x234a22;
    pub const FLOOR_A: u32 = 0x3a6a32;
    pub const FLOOR_B: u32 = 0x33602c;
    pub const PIT: u32 = 0x050406;
    pub const PIT_RIM: u32 = 0x2a1e12;
    pub const RAMP: u32 = 0x5a4324;
    pub const HOLLOW: u32 = 0x3a2a18;
    pub const DIRT: u32 = 0x4a3620;
    pub const TORCH_LIGHT: u32 = 0xd6ff5a;
    pub const HERO_LIGHT: u32 = 0xffe6b0;
    pub const MOON: u32 = 0xbfd4ff;
    pub const STUN: u32 = 0x6a8cff;
    pub const STUN_WARN: u32 = 0xffffff;
    pub const SLASH: u32 = 0xbfefff;
    pub const SHIELD: u32 = 0x7fd7ff;
    pub const INVINCIBLE: u32 = 0xffd84a;
    pub const REMEDY: u32 = 0x4cf06a;
    pub const MOTE: u32 = 0xe4ff6a;
}
use colors::*;

/// World units per elevation level.
pub const STEP: f32 = 0.55;
const MOTE_COUNT: usize = 220;
/// Share of wall tiles facing a road that grow a tree.
const TREE_DENSITY: f64 = 0.5;
const LIGHT_POOL_TORCHES: usize = 3;
const LIGHT_POOL_REMEDIES: usize = 3;
const LIGHT_POOL_SHRINES: usize = 2;
const LIGHT_POOL_EFFECTS: usize = 2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CameraMode {
    Third,
    Top,
}

/// Over-the-shoulder chase camera tuning (see the web build for the reasoning).
mod chase {
    pub const DISTANCE: f32 = 3.4;
    pub const SHOULDER: f32 = 0.3;
    pub const PIVOT_HEIGHT: f32 = 1.7;
    pub const REST_PITCH: f32 = -0.44;
    pub const LOOK_AHEAD: f32 = 2.2;
    pub const MIN_PITCH: f32 = -1.2;
    pub const MAX_PITCH: f32 = 0.1;
    pub const MIN_DISTANCE: f32 = 0.9;
    pub const POS_LERP: f32 = 16.0;
    /// Radians of turn per pixel of mouse movement.
    pub const SENSITIVITY: f32 = 0.0026;
    /// Without the mouse captured the view drifts back behind him at this rate.
    pub const FOLLOW_LERP: f32 = 3.5;
    /// How quickly the view swings to keep a boss in frame.
    pub const FOCUS_LERP: f32 = 2.2;
}
/// Undergrowth is at least this tall, which is what the camera has to squeeze past.
const WALL_TOP: f32 = 0.7;
const TURN_RATE_HERO: f32 = 14.0;
const TURN_RATE_ENEMY: f32 = 10.0;
/// Top-down follow camera: high and slightly south of the hero, looking north.
const TOP_HEIGHT: f32 = 8.2;
const TOP_BACK: f32 = 5.0;
const TOP_LOOK_AHEAD: f32 = 0.4;
const TOP_LERP: f32 = 5.0;
/// Sebastian's voxel size: 23 voxels make him about as tall as a tile is wide.
const VOXEL: f32 = 0.05;
const BLAST_SECONDS: f32 = 0.45;
const DEBRIS_SECONDS: f32 = 1.4;
const GRAVITY: f32 = 14.0;
const REASSEMBLE_START: f64 = 0.7;
/// How long the sword's wind-up, strike and follow-through take on screen.
pub const SLASH_SECONDS: f32 = 0.3;
/// Particles alive at once; the oldest make way.
const MAX_PARTICLES: usize = 700;
/// Screen shake: trauma decays at this rate per second; offset scales with trauma squared.
const TRAUMA_DECAY: f32 = 1.9;
const SHAKE_DISTANCE: f32 = 0.13;
pub const FOV_DEGREES: f32 = 50.0;
pub const NEAR: f32 = 0.1;
pub const FAR: f32 = 80.0;

// ------------------------------------------------------------ GPU records

/// One instanced draw. `emissive.w` carries the shading mode.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct Instance {
    pub m: [f32; 16],
    /// Linear RGB and opacity.
    pub color: [f32; 4],
    /// Linear emitted radiance and the shading mode.
    pub emissive: [f32; 4],
}

pub const SHADE_LIT: f32 = 0.0;
pub const SHADE_UNLIT: f32 = 1.0;
/// Lit, with the emissive scaled by the shared nugget pulse.
pub const SHADE_PULSE: f32 = 2.0;
/// Lit, and screen-door dithered away by `1 - alpha` (trees fading out of the camera's way).
pub const SHADE_DITHER: f32 = 3.0;

/// How see-through a tree gets while it blocks the view of Popolon.
pub const TREE_FADE_MIN: f32 = 0.22;
/// Fade speed, in full fades per second.
const TREE_FADE_RATE: f32 = 6.0;

impl Instance {
    pub fn new(m: Mat4, color: Vec3, alpha: f32, emissive: Vec3, shade: f32) -> Self {
        Self {
            m: m.to_cols_array(),
            color: [color.x, color.y, color.z, alpha],
            emissive: [emissive.x, emissive.y, emissive.z, shade],
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MeshKind {
    Cube = 0,
    Sphere = 1,
    Disc = 2,
    Ring = 3,
    Cylinder = 4,
}
pub const MESH_KINDS: usize = 5;

#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub pos: Vec3,
    /// Linear color times intensity.
    pub color: Vec3,
    pub distance: f32,
}

/// The sky, fog and ambient light a level is drawn under.
#[derive(Clone, Copy, Debug)]
pub struct Atmosphere {
    /// Sky and fog color, sRGB hex.
    pub background: u32,
    pub fog_near: f32,
    pub fog_far: f32,
    pub hemi_sky: Vec3,
    pub hemi_ground: Vec3,
    pub ambient: Vec3,
    /// Direction toward the moon or sun, and its light.
    pub sun_dir: Vec3,
    pub sun: Vec3,
    pub exposure: f32,
}

impl Atmosphere {
    /// The dungeons: a moonlit forest at night.
    pub fn forest() -> Self {
        Self {
            background: BACKGROUND,
            fog_near: 14.0,
            fog_far: 34.0,
            hemi_sky: lin(0x6f8fa8),
            hemi_ground: lin(0x14260f),
            ambient: lin(0x2c3a2a) * 0.7,
            sun_dir: Vec3::new(-6.0, 14.0, 8.0).normalize(),
            sun: lin(MOON) * 0.9,
            exposure: 1.2,
        }
    }
}

impl Default for Atmosphere {
    fn default() -> Self {
        Self::forest()
    }
}

#[derive(Default)]
pub struct Frame {
    pub opaque: [Vec<Instance>; MESH_KINDS],
    pub transparent: [Vec<Instance>; MESH_KINDS],
    pub additive: Vec<Instance>,
    pub lights: Vec<Light>,
    pub coin_pulse: f32,
    /// The title diorama draws on its own; the level's static geometry sits this frame out.
    pub skip_static: bool,
    pub atmosphere: Atmosphere,
}

impl Frame {
    pub fn clear(&mut self) {
        for v in self.opaque.iter_mut().chain(self.transparent.iter_mut()) {
            v.clear();
        }
        self.additive.clear();
        self.lights.clear();
    }
}

pub struct SceneState<'a> {
    pub grid: &'a Grid,
    pub hero: &'a Hero,
    pub enemies: &'a [Enemy],
    pub frenzy: &'a Frenzy,
    pub time: f64,
    pub bombs: &'a [Bomb],
    pub projectiles: &'a [Projectile],
    pub diamonds: &'a [Diamond],
    pub potions: &'a [Potion],
    pub puddles: &'a PuddleField,
    pub boss: Option<&'a crate::logic::boss::Cyclops>,
    pub boss_shots: &'a [crate::logic::boss::BossShot],
}

// ---------------------------------------------------------------- views

#[derive(Clone)]
struct Pose {
    pos: Vec<Vec3>,
    rot: Vec<Vec3>,
}

impl Pose {
    fn rest(rig: &Rig) -> Self {
        Self { pos: rig.parts.iter().map(|p| p.rest).collect(), rot: vec![Vec3::ZERO; rig.parts.len()] }
    }
    fn rot_x(&mut self, rig: &Rig, name: &str, angle: f32) {
        if let Some(i) = rig.index(name) {
            self.rot[i].x = angle;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RigId {
    Skeleton,
    Bat,
    Vampire,
    Slime,
}

struct EnemyView {
    rig: RigId,
    pose: Pose,
    kind: EnemyKind,
    hover: f32,
    phase: f32,
    /// Smoothed model heading.
    yaw: f32,
    group_pos: Vec3,
    root_rot: Vec3,
    root_scale: Vec3,
    visible: bool,
    base_color: Vec3,
    mat_color: Vec3,
    emissive: Vec3,
    light_color: Vec3,
    light_intensity: f32,
    scatter: Option<Vec<(Vec3, Vec3)>>,
}

/// A tree on a wall tile, approximated as an upright cylinder for the camera test.
struct Tree {
    range: (usize, usize),
    base: Vec3,
    radius: f32,
    height: f32,
    fade: f32,
    alive: bool,
}

impl Tree {
    /// Does the sight line from `eye` to `to` pass through this tree?
    fn blocks(&self, eye: Vec3, to: Vec3) -> bool {
        // Cheap reject: the tree has to lie between the two ends in the ground plane.
        let r = self.radius;
        if self.base.x < eye.x.min(to.x) - r || self.base.x > eye.x.max(to.x) + r {
            return false;
        }
        if self.base.z < eye.z.min(to.z) - r || self.base.z > eye.z.max(to.z) + r {
            return false;
        }
        const SAMPLES: usize = 24;
        // Stop a little short of Popolon so a tree right behind him never counts.
        for i in 0..SAMPLES {
            let p = eye.lerp(to, i as f32 / SAMPLES as f32 * 0.95);
            if p.y < self.base.y || p.y > self.base.y + self.height {
                continue;
            }
            let (dx, dz) = (p.x - self.base.x, p.z - self.base.z);
            if dx * dx + dz * dz < r * r {
                return true;
            }
        }
        false
    }
}

/// One speck of dust or spark.
struct Particle {
    pos: Vec3,
    vel: Vec3,
    color: Vec3,
    life: f32,
    max: f32,
    size: f32,
    gravity: f32,
    floor: f32,
    glow: bool,
}

struct Debris {
    pos: Vec3,
    vel: Vec3,
    rot: Vec3,
    spin: Vec3,
    life: f32,
    floor_y: f32,
    color: Vec3,
}

struct Blast {
    id: u64,
    light: usize,
    pos: Vec3,
    color: Vec3,
    life: f32,
    seconds: f32,
    intensity: f32,
    scale: f32,
    radius: f32,
}

#[derive(Clone, Copy, Default)]
struct EffectLight {
    pos: Vec3,
    color: Vec3,
    intensity: f32,
    distance: f32,
    owner: u64,
}

struct Models {
    popolon: Rig,
    skeleton: Rig,
    bat: Rig,
    vampire: Rig,
    slime: Rig,
    medkit: Mesh,
    bomb: Mesh,
    arrow: Mesh,
    boomerang: Mesh,
    fireball: Mesh,
    crescent: Mesh,
    diamond: Mesh,
    tree: Mesh,
    nugget: Mesh,
    bottles: HashMap<u32, Mesh>,
    cyclops: Rig,
    boulder: Rig,
    /// Sebastian for every axe level and shield (or none).
    heroes: HashMap<(u8, Option<crate::logic::gear::ShieldKind>), Rig>,
    blacksmith: Rig,
    knife: Rig,
}

impl Models {
    fn new() -> Self {
        let mut bottles = HashMap::new();
        for kind in crate::logic::gear::POTIONS {
            let hex = crate::logic::gear::potion(kind).color;
            bottles.insert(hex, build_mesh_recolored(&BOTTLE, 0.1, BOTTLE_LIQUID, hex));
        }
        let mut heroes = HashMap::new();
        for level in 1..=3u8 {
            for shield in [None].into_iter().chain(crate::logic::gear::SHIELDS.into_iter().map(Some)) {
                heroes.insert((level, shield), build_rig(&characters::sebastian_geared(level, shield), characters::SEBASTIAN_RIG, VOXEL));
            }
        }
        Self {
            popolon: build_rig(&characters::sebastian(), characters::SEBASTIAN_RIG, VOXEL),
            skeleton: build_rig(&characters::draugr(), characters::DRAUGR_RIG, 0.05),
            bat: build_rig(&characters::raven(false), characters::RAVEN_RIG, 0.042),
            vampire: build_rig(&characters::raven(true), characters::RAVEN_RIG, 0.05),
            slime: build_rig(&characters::bog_slime(), &[], 0.075),
            medkit: build_mesh(&MEDKIT, 0.1),
            bomb: build_mesh(&BOMB, 0.09),
            arrow: build_mesh(&ARROW, 0.08),
            boomerang: build_mesh(&BOOMERANG, 0.07),
            fireball: build_mesh(&FIREBALL, 0.09),
            crescent: build_mesh(&CRESCENT, 0.08),
            diamond: build_mesh(&DIAMOND, 0.1),
            tree: build_mesh(&TREE_LOW, 0.17),
            nugget: build_mesh(&NUGGET, 0.11),
            bottles,
            cyclops: build_layered_rig(
                &characters::cyclops(),
                characters::CYCLOPS_RIG,
                &[
                    ("iris", &characters::cyclops_iris(), characters::CYCLOPS_EYE),
                    ("lidTop", &characters::cyclops_lid(true), characters::CYCLOPS_EYE),
                    ("lidBottom", &characters::cyclops_lid(false), characters::CYCLOPS_EYE),
                ],
                0.07,
            ),
            boulder: build_rig(&meadow::boulder(), &[], 0.09),
            heroes,
            blacksmith: build_rig(&characters::blacksmith(), characters::BLACKSMITH_RIG, 0.055),
            knife: build_rig(&forge::knife_model(), &[], 0.06),
        }
    }

    fn rig(&self, id: RigId) -> &Rig {
        match id {
            RigId::Skeleton => &self.skeleton,
            RigId::Bat => &self.bat,
            RigId::Vampire => &self.vampire,
            RigId::Slime => &self.slime,
        }
    }
}

pub struct WorldView {
    pub camera_mode: CameraMode,
    /// Where the chase camera looks, in world radians. The mouse drives these.
    pub cam_yaw: f32,
    pub cam_pitch: f32,
    /// True while the mouse is captured; otherwise the view trails his heading.
    pub mouse_look: bool,
    pub aspect: f32,
    /// Title screen: the camera circles Sebastian instead of chasing him.
    pub showcase: bool,
    showcase_angle: f32,
    showcase_timer: f32,

    models: Models,
    /// Level geometry that only changes when a bomb or a pickup changes it.
    pub static_cubes: Vec<Instance>,
    pub static_dirty: bool,
    /// Static instances that make up each wall tile, hidden when a bomb breaks it.
    rock_index: HashMap<P, (usize, usize)>,
    /// A boss arena: the sunny meadow instead of the night forest.
    pub meadow: bool,
    /// Stream tiles (bridges included) and whether the water runs along x.
    water: Vec<(P, bool)>,
    /// Slam shockwaves: center and age.
    rings: Vec<(Vec3, f32)>,
    trees: Vec<Tree>,
    tree_index: HashMap<P, usize>,
    /// Static instance ranges rewritten since the last upload (tree fades).
    pub static_patches: Vec<(usize, usize)>,
    coin_ranges: HashMap<P, (usize, usize)>,
    torches: HashMap<P, (Vec3, (usize, usize))>,
    charges: Vec<(P, Vec3)>,
    mote_base: Vec<Vec3>,
    effect_lights: [EffectLight; LIGHT_POOL_EFFECTS],
    effect_cursor: usize,
    next_blast_id: u64,
    blasts: Vec<Blast>,
    debris: Vec<Debris>,
    hero_pose: Pose,
    hero_phase: f32,
    hero_yaw: f32,
    hero_root_rot: Vec3,
    /// Seconds into the current sword slash, or negative when idle.
    slash_t: f32,
    /// +1 forehand, -1 backhand; alternates every swing.
    slash_side: f32,
    hero_lunge: f32,
    particles: Vec<Particle>,
    /// 0..1 accumulated impact, driving the screen shake.
    trauma: f32,
    shake_clock: f32,
    shake_offset: Vec3,
    enemy_views: Vec<EnemyView>,
    diamond_spin: Vec<f32>,
    cam_pos: Vec3,
    cam_look: Vec3,
    initialized: bool,
    was_showcase: bool,
    /// R3 was pressed: the camera is swinging back behind him.
    recentering: bool,
    /// A boss the chase camera keeps in view (ground position), when there is one.
    pub focus: Option<Vec3>,
    /// The shop: the camera looks into the blacksmith's forge.
    pub forge: bool,
    /// The first-run tour: what the camera circles, seconds at this stop, and which stop.
    pub tour: Option<(Vec3, f32, usize)>,
    tour_index: Option<usize>,
    tour_angle: f32,
    was_forge: bool,
    zaps: Vec<forge::Zap>,
    /// Swiftness afterimages: position, heading, fade.
    ghosts: Vec<(Vec3, f32, f32)>,
    ghost_clock: f32,
    rng: Rng,
}

fn tile_key(x: i32, y: i32) -> P {
    P::new(x, y)
}

impl WorldView {
    pub fn new() -> Self {
        let models = Models::new();
        let hero_pose = Pose::rest(&models.popolon);
        Self {
            camera_mode: CameraMode::Third,
            cam_yaw: 0.0,
            cam_pitch: chase::REST_PITCH,
            mouse_look: false,
            aspect: 16.0 / 9.0,
            showcase: false,
            showcase_angle: 0.6,
            showcase_timer: 0.0,
            models,
            static_cubes: Vec::new(),
            static_dirty: true,
            rock_index: HashMap::new(),
            meadow: false,
            water: Vec::new(),
            rings: Vec::new(),
            trees: Vec::new(),
            tree_index: HashMap::new(),
            static_patches: Vec::new(),
            coin_ranges: HashMap::new(),
            torches: HashMap::new(),
            charges: Vec::new(),
            mote_base: Vec::new(),
            effect_lights: [EffectLight::default(); LIGHT_POOL_EFFECTS],
            effect_cursor: 0,
            next_blast_id: 1,
            blasts: Vec::new(),
            debris: Vec::new(),
            hero_pose,
            hero_phase: 0.0,
            hero_yaw: 0.0,
            hero_root_rot: Vec3::ZERO,
            slash_t: -1.0,
            slash_side: 1.0,
            hero_lunge: 0.0,
            particles: Vec::new(),
            trauma: 0.0,
            shake_clock: 0.0,
            shake_offset: Vec3::ZERO,
            enemy_views: Vec::new(),
            diamond_spin: Vec::new(),
            cam_pos: Vec3::ZERO,
            cam_look: Vec3::ZERO,
            initialized: false,
            was_showcase: false,
            recentering: false,
            focus: None,
            forge: false,
            tour: None,
            tour_index: None,
            tour_angle: 0.0,
            was_forge: false,
            zaps: Vec::new(),
            ghosts: Vec::new(),
            ghost_clock: 0.0,
            rng: Rng::from_time(),
        }
    }

    // ---------------------------------------------------------------- build

    pub fn build_level(&mut self, grid: &Grid, enemies: &[Enemy], diamonds: &[Diamond], meadow: bool) {
        self.static_cubes.clear();
        self.rings.clear();
        self.water.clear();
        self.rock_index.clear();
        self.trees.clear();
        self.tree_index.clear();
        self.static_patches.clear();
        self.coin_ranges.clear();
        self.torches.clear();
        self.charges.clear();
        self.enemy_views.clear();
        self.blasts.clear();
        self.debris.clear();
        self.particles.clear();
        self.trauma = 0.0;
        for l in self.effect_lights.iter_mut() {
            l.intensity = 0.0;
        }
        self.meadow = meadow;
        if meadow {
            self.build_meadow(grid);
        } else {
            self.build_forest(grid);
        }
        self.build_pickups(grid, enemies, diamonds);
    }

    /// The dungeons' night forest: floors in levels, ramps, pits, undergrowth, trees, lanterns.
    fn build_forest(&mut self, grid: &Grid) {
        let floor_y = |x: i32, y: i32| grid.elevation(x, y) as f32 * STEP;
        let white = Vec3::ONE;

        // Floors: raised tiles are boxes standing on the ground, so their sides show as cliffs.
        for (x, y, t) in grid.cells() {
            if t == Tile::Rock || t == Tile::Pit || grid.is_ramp(x, y) {
                continue;
            }
            let h = floor_y(x, y);
            let thick = 0.1 + h;
            let m = Mat4::from_translation(Vec3::new(x as f32, h - thick / 2.0, y as f32))
                * Mat4::from_scale(Vec3::new(1.0, thick, 1.0));
            let checker = (x + y) % 2 == 0;
            let base = if t == Tile::Nest { HOLLOW } else if checker { FLOOR_A } else { FLOOR_B };
            let c = offset_hsl(
                lin(base),
                0.0,
                0.0,
                (hash(x as f64, y as f64) as f32 - 0.5) * 0.05 + grid.elevation(x, y) as f32 * 0.05,
            );
            self.static_cubes.push(Instance::new(m, c, 1.0, Vec3::ZERO, SHADE_LIT));
        }

        // Ramps: three ascending slabs toward the higher neighbor.
        for (x, y, _) in grid.cells() {
            if !grid.is_ramp(x, y) {
                continue;
            }
            let low = grid.elevation(x, y);
            let mut up: Option<Dir> = None;
            for d in CARDINALS {
                let n = grid.neighbor(P::new(x, y), d);
                if grid.get(n.x, n.y) != Tile::Rock && !grid.is_ramp(n.x, n.y) && grid.elevation(n.x, n.y) > low {
                    up = Some(d);
                }
            }
            let up = up.unwrap_or_else(|| {
                // Interior ramp tile: lean the way a neighboring ramp tile does, else default right.
                let mut u = Dir::Right;
                for d in CARDINALS {
                    let n = grid.neighbor(P::new(x, y), d);
                    if grid.is_ramp(n.x, n.y) && grid.elevation(n.x, n.y) > low {
                        u = d;
                    }
                }
                u
            });
            let (vx, vy) = up.vec();
            for i in 0..3 {
                let top = low as f32 * STEP + ((i + 1) as f32 / 3.0) * STEP;
                let thick = top + 0.1;
                let o = (i as f32 - 1.0) / 3.0;
                let m = Mat4::from_translation(Vec3::new(x as f32 + vx as f32 * o, (top - 0.1) / 2.0, y as f32 + vy as f32 * o))
                    * Mat4::from_scale(Vec3::new(if vx != 0 { 1.0 / 3.0 } else { 1.0 }, thick, if vy != 0 { 1.0 / 3.0 } else { 1.0 }));
                self.static_cubes.push(Instance::new(m, lin(RAMP), 1.0, Vec3::ZERO, SHADE_LIT));
            }
        }

        // Pits: a dark floor far below with a rim.
        for (x, y, t) in grid.cells() {
            if t != Tile::Pit {
                continue;
            }
            let bottom = Mat4::from_translation(Vec3::new(x as f32, -0.9, y as f32)) * Mat4::from_scale(Vec3::new(1.0, 0.1, 1.0));
            self.static_cubes.push(Instance::new(bottom, lin(PIT), 1.0, Vec3::ZERO, SHADE_LIT));
            let rim = Mat4::from_translation(Vec3::new(x as f32, -0.03, y as f32)) * Mat4::from_scale(Vec3::new(1.02, 0.06, 1.02));
            self.static_cubes.push(Instance::new(rim, lin(PIT_RIM), 1.0, Vec3::ZERO, SHADE_LIT));
        }

        // Walls: undergrowth blocks rising from the tallest neighboring floor, with trees on the road-facing edge.
        let faces_road = |x: i32, y: i32| {
            grid.get(x - 1, y) != Tile::Rock
                || grid.get(x + 1, y) != Tile::Rock
                || grid.get(x, y - 1) != Tile::Rock
                || grid.get(x, y + 1) != Tile::Rock
        };
        let tree = build_mesh(&TREE_LOW, 0.17);
        let lantern = build_mesh(&LANTERN, 0.09);
        let lantern_glow = lin(TORCH_LIGHT) * 0.8;
        let mut trees: Vec<(P, Vec3, f32, f32)> = Vec::new();
        let mut lanterns: Vec<P> = Vec::new();
        for (x, y, t) in grid.cells() {
            if t != Tile::Rock {
                continue;
            }
            let (xf, yf) = (x as f64, y as f64);
            let base = floor_y(x, y);
            let edge = faces_road(x, y);
            let tall = base + if edge { 0.7 + hash(xf, yf) as f32 * 0.25 } else { 0.9 + hash(xf, yf) as f32 * 0.3 };
            let m = Mat4::from_translation(Vec3::new(x as f32, tall / 2.0, y as f32)) * Mat4::from_scale(Vec3::new(1.0, tall, 1.0));
            let shade = hash(xf + 11.0, yf + 5.0) as f32;
            let c = offset_hsl(lin(if shade < 0.5 { STONE_A } else { STONE_B }), 0.0, 0.0, (shade - 0.5) * 0.08);
            self.rock_index.insert(tile_key(x, y), (self.static_cubes.len(), 1));
            self.static_cubes.push(Instance::new(m, c, 1.0, Vec3::ZERO, SHADE_LIT));
            if !edge {
                continue;
            }
            if grid.is_walkable(x, y + 1, false) && hash(xf * 3.0, yf * 7.0) < 0.06 {
                lanterns.push(P::new(x, y));
            } else if hash(xf * 5.0 + 1.0, yf * 3.0 + 2.0) < TREE_DENSITY {
                trees.push((
                    P::new(x, y),
                    Vec3::new(x as f32, tall - 0.05, y as f32),
                    hash(xf, yf) as f32 * TAU,
                    0.8 + hash(yf, xf) as f32 * 0.6,
                ));
            }
        }
        for (key, pos, rot, scale) in trees {
            let start = self.static_cubes.len();
            push_field(&mut self.static_cubes, &tree, pos, rot, scale, white, Vec3::ZERO, SHADE_LIT);
            self.tree_index.insert(key, self.trees.len());
            self.trees.push(Tree {
                range: (start, self.static_cubes.len() - start),
                base: pos,
                // TREE_LOW is 3 voxels across and 6 tall at 0.17 per voxel.
                radius: 0.26 * scale + 0.05,
                height: 1.02 * scale,
                fade: 1.0,
                alive: true,
            });
        }
        for spot in lanterns {
            let pos = Vec3::new(spot.x as f32, floor_y(spot.x, spot.y) + 0.5, spot.y as f32 + 0.5);
            let start = self.static_cubes.len();
            push_field(&mut self.static_cubes, &lantern, pos, 0.0, 1.0, white, lantern_glow, SHADE_LIT);
            self.torches.insert(spot, (pos, (start, self.static_cubes.len() - start)));
        }

    }

    /// Nuggets, green crosses, the cast and the fireflies: the same in every theme.
    fn build_pickups(&mut self, grid: &Grid, enemies: &[Enemy], diamonds: &[Diamond]) {
        let floor_y = |x: i32, y: i32| grid.elevation(x, y) as f32 * STEP;
        let white = Vec3::ONE;

        // Nuggets: hidden per tile as they are collected. Their glow pulses together.
        let nugget = build_mesh(&NUGGET, 0.11);
        let nugget_glow = lin(0xf2c53d);
        for (x, y, t) in grid.cells() {
            if t != Tile::Nodule {
                continue;
            }
            let (xf, yf) = (x as f64, y as f64);
            let start = self.static_cubes.len();
            push_field(
                &mut self.static_cubes,
                &nugget,
                Vec3::new(x as f32, floor_y(x, y), y as f32),
                hash(xf, yf) as f32 * TAU,
                0.9 + hash(yf, xf) as f32 * 0.4,
                white,
                nugget_glow,
                SHADE_PULSE,
            );
            self.coin_ranges.insert(P::new(x, y), (start, self.static_cubes.len() - start));
        }

        for (x, y, t) in grid.cells() {
            if t == Tile::Charge {
                self.charges.push((P::new(x, y), Vec3::new(x as f32, floor_y(x, y) + 0.3, y as f32)));
            }
        }

        for e in enemies {
            self.add_enemy(e);
        }
        self.diamond_spin = vec![0.0; diamonds.len()];

        // Fireflies drifting over the forest.
        self.mote_base = (0..MOTE_COUNT)
            .map(|_| {
                Vec3::new(
                    self.rng.nextf() * grid.width as f32,
                    0.6 + self.rng.nextf() * 1.4,
                    self.rng.nextf() * grid.height as f32,
                )
            })
            .collect();

        self.static_dirty = true;
        self.initialized = false;
    }

    /// Adds a view for an enemy; call again for enemies that arrive mid-level. Order must match the game's list.
    pub fn add_enemy(&mut self, e: &Enemy) {
        let (rig, hover) = match e.kind() {
            EnemyKind::Skeleton => (RigId::Skeleton, 0.0),
            EnemyKind::Bat => (RigId::Bat, 0.55),
            EnemyKind::Vampire => (RigId::Vampire, 0.65),
            EnemyKind::Slime => (RigId::Slime, 0.0),
        };
        let pose = Pose::rest(self.models.rig(rig));
        let base = lin(e.b.color);
        self.enemy_views.push(EnemyView {
            rig,
            pose,
            kind: e.kind(),
            hover,
            phase: self.rng.nextf() * 6.0,
            yaw: 0.0,
            group_pos: Vec3::ZERO,
            root_rot: Vec3::ZERO,
            root_scale: Vec3::ONE,
            visible: false,
            base_color: base,
            mat_color: Vec3::ONE,
            emissive: Vec3::ZERO,
            light_color: base,
            light_intensity: 0.0,
            scatter: None,
        });
    }

    fn hide_range(&mut self, range: (usize, usize)) {
        let hidden = Mat4::from_translation(Vec3::new(0.0, -50.0, 0.0)) * Mat4::from_scale(Vec3::ZERO);
        for i in range.0..range.0 + range.1 {
            self.static_cubes[i].m = hidden.to_cols_array();
        }
        self.static_dirty = true;
    }

    /// Collapse a wall block after a blast and patch the floor beneath it.
    pub fn remove_rock(&mut self, grid: &Grid, x: i32, y: i32) {
        let k = tile_key(x, y);
        if let Some(range) = self.rock_index.remove(&k) {
            self.hide_range(range);
        }
        if let Some((_, range)) = self.torches.remove(&k) {
            self.hide_range(range);
        }
        // A tree standing on the blasted block falls with it.
        if let Some(i) = self.tree_index.remove(&k) {
            self.trees[i].alive = false;
            let range = self.trees[i].range;
            self.hide_range(range);
        }
        let h = floor_at(grid, x as f32, y as f32);
        let thick = 0.1 + h;
        let m = Mat4::from_translation(Vec3::new(x as f32, h - thick / 2.0, y as f32)) * Mat4::from_scale(Vec3::new(1.0, thick, 1.0));
        let patch = if self.meadow { lin(0x62b437) } else { lin(DIRT) };
        self.static_cubes.push(Instance::new(m, patch, 1.0, Vec3::ZERO, SHADE_LIT));
        self.static_dirty = true;
    }

    pub fn remove_collectible(&mut self, x: i32, y: i32) {
        let k = tile_key(x, y);
        if let Some(range) = self.coin_ranges.remove(&k) {
            self.hide_range(range);
        }
        self.charges.retain(|(p, _)| *p != k);
    }

    /// Small light pop where a shot lands or connects.
    pub fn spark(&mut self, grid: &Grid, x: f64, y: f64, color: u32) {
        self.burst(grid, x as f32, y as f32, color, 14.0, 3.0, 0.18, 0.6, 0.12);
    }

    #[allow(clippy::too_many_arguments)]
    fn burst(&mut self, grid: &Grid, x: f32, y: f32, color: u32, intensity: f32, radius: f32, seconds: f32, scale: f32, start_radius: f32) {
        let h = floor_at(grid, x, y);
        let li = self.effect_cursor % LIGHT_POOL_EFFECTS;
        self.effect_cursor += 1;
        let id = self.next_blast_id;
        self.next_blast_id += 1;
        self.effect_lights[li] = EffectLight {
            pos: Vec3::new(x, h + 0.8, y),
            color: lin(color),
            intensity,
            distance: radius,
            owner: id,
        };
        self.blasts.push(Blast {
            id,
            light: li,
            pos: Vec3::new(x, h + 0.5, y),
            color: lin(color),
            life: seconds,
            seconds,
            intensity,
            scale,
            radius: start_radius,
        });
    }

    /// Flash, shockwave sphere, and flying chunks at a blast site.
    pub fn explode(&mut self, grid: &Grid, x: f64, y: f64, broken_tiles: usize) {
        let (x, y) = (x as f32, y as f32);
        self.burst(grid, x, y, 0xffb060, 120.0, 9.0, BLAST_SECONDS, 3.5, 0.5);
        let h = floor_at(grid, x, y);
        let count = 10 + broken_tiles * 4;
        for _ in 0..count {
            let r = &mut self.rng;
            let shade = r.nextf();
            // Torn undergrowth: leaves and clods of earth.
            let color = lin(if shade < 0.6 { STONE_A } else { DIRT });
            let pos = Vec3::new(x + (r.nextf() - 0.5) * 0.6, h + 0.4 + r.nextf() * 0.5, y + (r.nextf() - 0.5) * 0.6);
            let a = r.nextf() * TAU;
            let s = 2.0 + r.nextf() * 3.0;
            let vel = Vec3::new(a.cos() * s, 4.0 + r.nextf() * 5.0, a.sin() * s);
            let spin = Vec3::new(r.nextf() * 8.0, r.nextf() * 8.0, r.nextf() * 8.0);
            let life = DEBRIS_SECONDS * (0.7 + r.nextf() * 0.5);
            self.debris.push(Debris { pos, vel, rot: Vec3::ZERO, spin, life, floor_y: h + 0.08, color });
        }
    }

    // --------------------------------------------------------------- update

    /// Advances animation, effects and the camera by one fixed step.
    pub fn update(&mut self, dt: f64, s: &SceneState) {
        let dtf = dt as f32;
        let time = s.time as f32;
        self.update_effects(dtf, s);

        // Hero pose: stride cycle, facing, sword swing, jump tuck.
        let hero = s.hero;
        if self.showcase {
            // Sebastian shows off his axe now and then on the title screen.
            self.showcase_timer += dtf;
            if self.showcase_timer > 3.6 {
                self.showcase_timer = 0.0;
                self.sword_slash();
            }
        }
        if self.slash_t >= 0.0 {
            self.slash_t += dtf;
            if self.slash_t > SLASH_SECONDS {
                self.slash_t = -1.0;
            }
        }
        self.hero_phase += dtf * hero.motion as f32 * if hero.hasted() { 17.0 } else { 13.0 };
        self.hero_yaw = lerp_angle(self.hero_yaw, dir_to_rotation(hero.facing), 1.0 - (-TURN_RATE_HERO * dtf).exp());
        self.animate_hero(hero);

        for (i, e) in s.enemies.iter().enumerate() {
            if i >= self.enemy_views.len() {
                break;
            }
            self.update_enemy(i, e, s, dtf, time);
        }
        for (i, d) in s.diamonds.iter().enumerate() {
            if d.active {
                if let Some(a) = self.diamond_spin.get_mut(i) {
                    *a += dtf * (2.0 + d.cycle_progress() as f32 * 8.0);
                }
            }
        }
        self.update_meadow(dtf);
        self.update_forge_fx(dtf, hero);
        self.update_camera(dtf, s.grid, hero);
        if !self.showcase {
            self.update_tree_fade(dtf, hero);
        }
        self.update_shake(dtf);
    }

    /// Trauma-based shake: a few incommensurate sines, so it never loops visibly.
    fn update_shake(&mut self, dt: f32) {
        self.trauma = (self.trauma - TRAUMA_DECAY * dt).max(0.0);
        self.shake_clock += dt;
        let t = self.shake_clock;
        let s = self.trauma * self.trauma * SHAKE_DISTANCE;
        self.shake_offset = Vec3::new(
            (t * 47.0).sin() * 0.6 + (t * 83.0).sin() * 0.4,
            ((t * 59.0).sin() * 0.6 + (t * 97.0).cos() * 0.4) * 0.7,
            (t * 53.0).cos() * 0.6 + (t * 71.0).sin() * 0.4,
        ) * s;
    }

    /// Adds impact to the screen shake (0.05 a tap, 0.7 a bomb at your feet).
    pub fn shake(&mut self, amount: f32) {
        self.trauma = (self.trauma + amount).min(1.0);
    }

    /// Starts the sword's slash animation; swings alternate forehand and backhand.
    pub fn sword_slash(&mut self) {
        self.slash_t = 0.0;
        self.slash_side = -self.slash_side;
    }

    fn push_particle(&mut self, p: Particle) {
        if self.particles.len() >= MAX_PARTICLES {
            self.particles.remove(0);
        }
        self.particles.push(p);
    }

    /// Micro dust kicked up at a point on the floor: small lit cubes that tumble and settle.
    pub fn dust(&mut self, grid: &Grid, x: f64, y: f64, color: u32, count: usize, speed: f32) {
        let (x, z) = (x as f32, y as f32);
        let floor = floor_at(grid, x, z);
        let base = lin(color);
        for _ in 0..count {
            let r = &mut self.rng;
            let a = r.nextf() * TAU;
            let sp = speed * (0.4 + r.nextf() * 0.6);
            let shade = 0.75 + r.nextf() * 0.5;
            let p = Particle {
                pos: Vec3::new(x + (r.nextf() - 0.5) * 0.3, floor + 0.04 + r.nextf() * 0.15, z + (r.nextf() - 0.5) * 0.3),
                vel: Vec3::new(a.cos() * sp, speed * (0.4 + r.nextf() * 0.8), a.sin() * sp),
                color: base * shade,
                life: 0.3 + r.nextf() * 0.4,
                max: 0.7,
                size: 0.03 + r.nextf() * 0.035,
                gravity: 7.0,
                floor: floor + 0.02,
                glow: false,
            };
            self.push_particle(p);
        }
    }

    /// Glowing motes that burst upward from a point: pickups, kills, heals.
    pub fn sparkle(&mut self, grid: &Grid, x: f64, y: f64, height: f32, color: u32, count: usize) {
        let (x, z) = (x as f32, y as f32);
        let floor = floor_at(grid, x, z);
        let base = lin(color);
        for _ in 0..count {
            let r = &mut self.rng;
            let a = r.nextf() * TAU;
            let sp = 0.6 + r.nextf() * 1.6;
            let life = 0.4 + r.nextf() * 0.45;
            let p = Particle {
                pos: Vec3::new(x, floor + height, z),
                vel: Vec3::new(a.cos() * sp, 1.2 + r.nextf() * 2.6, a.sin() * sp),
                color: base * (1.2 + r.nextf() * 0.8),
                life,
                max: life,
                size: 0.035 + r.nextf() * 0.03,
                gravity: 4.0,
                floor: floor + 0.02,
                glow: true,
            };
            self.push_particle(p);
        }
    }

    /**
     * Trees standing between the camera and Popolon dissolve so he is never
     * lost behind the woods, then grow back once the view is clear. The fade
     * is a screen-door dither in the opaque pass, so depth stays correct.
     */
    fn update_tree_fade(&mut self, dt: f32, hero: &Hero) {
        let hp = hero.pos();
        // During the tour, clear the view of whatever the camera is showing.
        let feet = match self.tour {
            Some((target, _, _)) => target - Vec3::new(0.0, 0.3, 0.0),
            None => Vec3::new(hp.x as f32, hero.altitude as f32 * STEP, hp.y as f32),
        };
        let targets = [feet + Vec3::new(0.0, 0.25, 0.0), feet + Vec3::new(0.0, 0.6, 0.0), feet + Vec3::new(0.0, 1.0, 0.0)];
        let eye = self.cam_pos;
        let step = TREE_FADE_RATE * dt;
        for i in 0..self.trees.len() {
            let t = &self.trees[i];
            if !t.alive {
                continue;
            }
            let blocking = (hero.alive || self.tour.is_some()) && targets.iter().any(|&to| t.blocks(eye, to));
            let goal = if blocking { TREE_FADE_MIN } else { 1.0 };
            let fade = if t.fade < goal { (t.fade + step).min(goal) } else { (t.fade - step).max(goal) };
            if fade == t.fade {
                continue;
            }
            let (start, len) = t.range;
            self.trees[i].fade = fade;
            let shade = if fade < 0.999 { SHADE_DITHER } else { SHADE_LIT };
            for inst in &mut self.static_cubes[start..start + len] {
                inst.color[3] = fade;
                inst.emissive[3] = shade;
            }
            self.static_patches.push((start, len));
        }
    }

    /// How faded each standing tree is (1 = solid), for tests and tools.
    pub fn tree_fades(&self) -> impl Iterator<Item = f32> + '_ {
        self.trees.iter().filter(|t| t.alive).map(|t| t.fade)
    }

    fn update_effects(&mut self, dt: f32, _s: &SceneState) {
        let drag = 1.0 - (2.4 * dt).min(0.9);
        self.particles.retain_mut(|p| {
            p.life -= dt;
            if p.life <= 0.0 {
                return false;
            }
            p.vel.y -= p.gravity * dt;
            p.vel.x *= drag;
            p.vel.z *= drag;
            p.pos += p.vel * dt;
            if p.pos.y < p.floor {
                p.pos.y = p.floor;
                p.vel.y *= -0.3;
                p.vel.x *= 0.5;
                p.vel.z *= 0.5;
            }
            true
        });
        self.debris.retain_mut(|d| {
            d.life -= dt;
            if d.life <= 0.0 {
                return false;
            }
            d.vel.y -= GRAVITY * dt;
            d.pos += d.vel * dt;
            if d.pos.y < d.floor_y {
                d.pos.y = d.floor_y;
                d.vel.y *= -0.35;
                d.vel.x *= 0.6;
                d.vel.z *= 0.6;
            }
            d.rot += d.spin * dt;
            true
        });
        let lights = &mut self.effect_lights;
        self.blasts.retain_mut(|b| {
            b.life -= dt;
            let owns = lights[b.light].owner == b.id;
            if b.life <= 0.0 {
                if owns {
                    lights[b.light].intensity = 0.0;
                }
                return false;
            }
            let t = 1.0 - b.life / b.seconds;
            if owns {
                lights[b.light].intensity = b.intensity * (1.0 - t);
            }
            true
        });
    }

    fn update_enemy(&mut self, i: usize, e: &Enemy, s: &SceneState, dt: f32, time: f32) {
        let grid = s.grid;
        let frenzy = s.frenzy;
        let ep = e.pos();
        let ground = if e.m.flying {
            floor_at(grid, ep.x as f32, ep.y as f32)
        } else {
            let mut m = e.m.clone();
            m.elevation_on(grid) as f32 * STEP
        };
        {
            let v = &mut self.enemy_views[i];
            v.group_pos = Vec3::new(ep.x as f32, ground + v.hover + (time * 5.0 + i as f32 * 1.7).sin() * 0.05, ep.y as f32);
            if e.m.dir != Dir::None {
                v.yaw = lerp_angle(v.yaw, dir_to_rotation(e.m.dir), 1.0 - (-TURN_RATE_ENEMY * dt).exp());
            }
        }
        self.animate_enemy(i, e, dt, time);
        let v = &mut self.enemy_views[i];
        // Waiting in the hollow they are out of sight, except when the tour shows them off.
        let hidden = e.mode() == EnemyMode::Eaten || (e.mode() == EnemyMode::Nest && self.tour.is_none());
        v.visible = !hidden;
        if e.mode() == EnemyMode::Scattered {
            v.light_intensity = 0.6;
            v.light_color = v.base_color;
            v.mat_color = Vec3::ONE;
            v.emissive = Vec3::ZERO;
            return;
        }
        if e.is_stunned() {
            let warn = frenzy.warning() && ((time * 8.0).floor() as i64).rem_euclid(2) == 0;
            v.mat_color = lin(if warn { STUN_WARN } else { STUN });
            v.emissive = lin(STUN) * (0.5 + (time * 12.0).sin() * 0.3);
            v.light_color = lin(STUN);
        } else {
            v.mat_color = Vec3::ONE;
            v.emissive = Vec3::ZERO;
            v.light_color = v.base_color;
        }
        v.light_intensity = if hidden { 0.0 } else { 2.5 };
    }

    /// Legs and arms swing in opposition while moving; the sword arm does the slash; a tuck in the air.
    fn animate_hero(&mut self, hero: &Hero) {
        let rig = &self.models.popolon;
        let p = &mut self.hero_pose;
        let motion = hero.motion as f32;
        let stride = self.hero_phase.sin() * motion;
        if hero.airborne() {
            let tuck = (hero.jump_progress() as f32 * PI).sin();
            p.rot_x(rig, "leftLeg", -0.9 * tuck);
            p.rot_x(rig, "rightLeg", -0.9 * tuck);
            p.rot_x(rig, "shieldArm", -0.6 * tuck);
            p.rot_x(rig, "swordArm", 0.4 * tuck);
            p.rot_x(rig, "head", -0.15 * tuck);
            self.hero_root_rot.x = 0.2 * tuck;
            return;
        }
        p.rot_x(rig, "leftLeg", stride * 0.75);
        p.rot_x(rig, "rightLeg", -stride * 0.75);
        p.rot_x(rig, "shieldArm", -stride * 0.35);
        let swing = if hero.swinging() { (hero.swing_progress() as f32 * PI).sin() * 1.7 } else { stride * 0.35 };
        p.rot_x(rig, "swordArm", swing);
        p.rot_x(rig, "head", motion * 0.08 + if hero.swinging() { 0.15 } else { 0.0 });
        self.hero_root_rot.x = motion * if hero.hasted() { 0.14 } else { 0.07 };
        self.hero_root_rot.y = 0.0;
        self.hero_lunge = 0.0;
        if self.slash_t >= 0.0 {
            self.pose_slash();
        }
    }

    /**
     * The sword slash, keyed like a hand animation: a quick wind-up that lifts
     * the blade back over the shoulder, a fast strike that sweeps it across
     * and down with the hips turning into it and a small lunge, then an eased
     * follow-through back to guard. Mirrored on alternate swings.
     */
    fn pose_slash(&mut self) {
        // (time, arm pitch, arm roll, body twist, lunge); roll and twist mirror with the side.
        const KEYS: [(f32, f32, f32, f32, f32); 4] = [
            (0.0, 0.0, 0.0, 0.0, 0.0),
            (0.2, -2.3, 0.55, 0.38, -0.03),
            (0.47, 1.55, -0.75, -0.5, 0.1),
            (1.0, 0.0, 0.0, 0.0, 0.0),
        ];
        let p = (self.slash_t / SLASH_SECONDS).clamp(0.0, 1.0);
        let k = KEYS.iter().rposition(|k| k.0 <= p).unwrap_or(0).min(KEYS.len() - 2);
        let (a, b) = (KEYS[k], KEYS[k + 1]);
        let u = ((p - a.0) / (b.0 - a.0)).clamp(0.0, 1.0);
        // The strike snaps (ease-out cubic); wind-up and recovery ease in and out.
        let e = if k == 1 { 1.0 - (1.0 - u).powi(3) } else { u * u * (3.0 - 2.0 * u) };
        let mix = |x: f32, y: f32| x + (y - x) * e;
        let side = self.slash_side;
        let rig = &self.models.popolon;
        if let Some(i) = rig.index("swordArm") {
            self.hero_pose.rot[i].x = mix(a.1, b.1);
            self.hero_pose.rot[i].z = mix(a.2, b.2) * side;
        }
        let guard = (p * PI).sin();
        self.hero_pose.rot_x(rig, "shieldArm", -0.55 * guard);
        self.hero_pose.rot_x(rig, "head", 0.12 * guard);
        self.hero_root_rot.y = mix(a.3, b.3) * side;
        self.hero_lunge = mix(a.4, b.4);
    }

    fn animate_enemy(&mut self, i: usize, e: &Enemy, dt: f32, time: f32) {
        if e.mode() == EnemyMode::Scattered {
            self.animate_scatter(i, e, dt);
            return;
        }
        let models = &self.models;
        let v = &mut self.enemy_views[i];
        let rig = models.rig(v.rig);
        if v.scatter.is_some() {
            // Reassembled: snap every bone home.
            v.pose = Pose::rest(rig);
            v.scatter = None;
        }
        let moving = e.m.dir != Dir::None && e.mode() != EnemyMode::Nest;
        if moving {
            v.phase += dt * e.m.speed as f32 * 2.2;
        }
        match v.kind {
            EnemyKind::Skeleton => {
                let s = v.phase.sin();
                v.pose.rot_x(rig, "leftLeg", s * 0.85);
                v.pose.rot_x(rig, "rightLeg", -s * 0.85);
                v.pose.rot_x(rig, "leftArm", -s * 0.5);
                v.pose.rot_x(rig, "rightArm", s * 0.5);
                v.pose.rot_x(rig, "head", (v.phase * 2.0).sin() * 0.1);
                v.root_rot.z = v.phase.sin() * 0.06;
            }
            EnemyKind::Bat | EnemyKind::Vampire => {
                let beat = time * 15.0 + i as f32 * 2.0;
                let flap = beat.sin() * 0.8;
                if let Some(l) = rig.index("leftWing") {
                    v.pose.rot[l].z = -flap;
                }
                if let Some(r) = rig.index("rightWing") {
                    v.pose.rot[r].z = flap;
                }
                v.root_rot.x = if moving { 0.3 } else { 0.05 };
                v.group_pos.y += beat.sin() * 0.035;
            }
            EnemyKind::Slime => {
                let t = if moving { v.phase } else { time * 2.5 + i as f32 };
                let sq = t.sin();
                v.root_scale = Vec3::new(1.0 - sq * 0.14, 1.0 + sq * 0.3, 1.0 - sq * 0.14);
                if moving {
                    v.group_pos.y += sq.max(0.0) * 0.2;
                }
                v.root_rot.x = if moving { 0.12 } else { 0.0 };
            }
        }
    }

    /// Bones burst outward and tumble to the floor, lie still, then glide back to their rest pose.
    fn animate_scatter(&mut self, i: usize, e: &Enemy, dt: f32) {
        let models = &self.models;
        let rng = &mut self.rng;
        let v = &mut self.enemy_views[i];
        let rig = models.rig(v.rig);
        v.root_rot = Vec3::ZERO;
        v.root_scale = Vec3::ONE;
        if v.scatter.is_none() {
            v.scatter = Some(
                rig.parts
                    .iter()
                    .map(|p| {
                        let a = rng.nextf() * TAU;
                        let sp = if p.name == "body" { 0.3 } else { 0.8 + rng.nextf() * 1.1 };
                        (
                            Vec3::new(a.cos() * sp, 2.5 + rng.nextf() * 2.5, a.sin() * sp),
                            Vec3::new((rng.nextf() - 0.5) * 16.0, (rng.nextf() - 0.5) * 16.0, (rng.nextf() - 0.5) * 16.0),
                        )
                    })
                    .collect(),
            );
        }
        let progress = e.scatter_progress();
        let scatter = v.scatter.as_mut().unwrap();
        if progress < REASSEMBLE_START {
            for (pi, part) in rig.parts.iter().enumerate() {
                let (vel, spin) = &mut scatter[pi];
                let floor_y = -part.min_y + 0.02;
                vel.y -= GRAVITY * dt;
                v.pose.pos[pi] += *vel * dt;
                if v.pose.pos[pi].y < floor_y {
                    v.pose.pos[pi].y = floor_y;
                    vel.y *= -0.3;
                    vel.x *= 0.5;
                    vel.z *= 0.5;
                    *spin *= 0.4;
                }
                v.pose.rot[pi] += *spin * dt;
            }
        } else {
            let t = ((progress - REASSEMBLE_START) / (1.0 - REASSEMBLE_START)) as f32;
            let k = 1.0 - (-dt * (6.0 + t * 30.0)).exp();
            for (pi, part) in rig.parts.iter().enumerate() {
                v.pose.pos[pi] = v.pose.pos[pi].lerp(part.rest, k);
                v.pose.rot[pi] *= 1.0 - k;
            }
            v.root_rot.z = (t * 60.0).sin() * 0.04 * (1.0 - t);
        }
    }

    fn update_camera(&mut self, dt: f32, grid: &Grid, hero: &Hero) {
        let hp = hero.pos();
        let (hx, hz) = (hp.x as f32, hp.y as f32);
        // Follow the floor under him, not the jump arc, so the view does not bounce on every hop.
        let focus = Vec3::new(hx, floor_at(grid, hx, hz), hz);
        if self.showcase != self.was_showcase || self.forge != self.was_forge {
            self.was_showcase = self.showcase;
            self.was_forge = self.forge;
            self.initialized = false;
        }
        if self.tour.is_none() {
            self.tour_index = None;
        }
        let (desired, look, pos_lerp) = if let Some((target, t, index)) = self.tour {
            // A dramatic swoop: in from high and wide, then a slow close orbit.
            if self.tour_index != Some(index) {
                self.tour_index = Some(index);
                self.tour_angle = open_angle(grid, target);
            }
            let e = 1.0 - (1.0 - (t / 1.2).min(1.0)).powi(3);
            let a = self.tour_angle + t * 0.16;
            let r = 2.5 + 3.6 * (1.0 - e);
            let h = 2.15 + 3.0 * (1.0 - e);
            (target + Vec3::new(a.cos() * r, h, a.sin() * r), target + Vec3::new(0.0, 0.35, 0.0), 4.0 + 5.0 * e)
        } else if self.forge {
            // The forge fills the left of the screen; the shop panel sits on the right.
            self.showcase_angle += dt;
            let a = (self.showcase_angle * 0.2).sin() * 0.15;
            (forge::FORGE + Vec3::new(2.3 + a, 2.3, 5.5), forge::FORGE + Vec3::new(2.05 + a * 0.5, 0.75, -0.2), 6.0)
        } else if self.showcase {
            // The title diorama: the camera sways gently in front of Sebastian.
            self.showcase_angle += dt;
            let a = (self.showcase_angle * 0.23).sin() * 0.5;
            (STAGE + Vec3::new(a.sin() * 4.3, 1.45, a.cos() * 4.3), STAGE + Vec3::new(0.0, 0.62, 0.0), 6.0)
        } else if self.camera_mode == CameraMode::Third {
            if self.recentering {
                let k = 1.0 - (-10.0 * dt).exp();
                let goal = dir_to_yaw(hero.facing);
                self.cam_yaw = lerp_angle(self.cam_yaw, goal, k);
                self.cam_pitch += (chase::REST_PITCH - self.cam_pitch) * k;
                let mut off = (self.cam_yaw - goal).rem_euclid(TAU);
                if off > PI {
                    off = TAU - off;
                }
                if off < 0.01 && (self.cam_pitch - chase::REST_PITCH).abs() < 0.01 {
                    self.recentering = false;
                }
            } else if !self.mouse_look {
                // In a boss fight the view turns toward the boss rather than trailing his heading.
                let target = self.focus.map(|p| p - focus).filter(|d| d.x * d.x + d.z * d.z > 2.5);
                match target {
                    Some(d) => self.cam_yaw = lerp_angle(self.cam_yaw, d.z.atan2(d.x), 1.0 - (-chase::FOCUS_LERP * dt).exp()),
                    None => self.cam_yaw = lerp_angle(self.cam_yaw, dir_to_yaw(hero.facing), 1.0 - (-chase::FOLLOW_LERP * dt).exp()),
                }
            }
            let cp = self.cam_pitch.cos();
            let dir = Vec3::new(cp * self.cam_yaw.cos(), self.cam_pitch.sin(), cp * self.cam_yaw.sin());
            let right = Vec3::new(-self.cam_yaw.sin(), 0.0, self.cam_yaw.cos());
            let pivot = focus + Vec3::new(0.0, chase::PIVOT_HEIGHT, 0.0) + right * chase::SHOULDER;
            let desired = pivot - dir * clear_behind(grid, pivot, dir);
            let look = pivot + dir * chase::LOOK_AHEAD;
            (desired, look, chase::POS_LERP)
        } else {
            (focus + Vec3::new(0.0, TOP_HEIGHT, TOP_BACK), focus + Vec3::new(0.0, 0.0, -TOP_LOOK_AHEAD), TOP_LERP)
        };
        if !self.initialized {
            self.cam_pos = desired;
            self.cam_look = look;
            self.initialized = true;
        } else {
            let k = 1.0 - (-pos_lerp * dt).exp();
            self.cam_pos = self.cam_pos.lerp(desired, k);
            self.cam_look = self.cam_look.lerp(look, k);
        }
    }

    /// Turns the camera by a mouse movement in pixels.
    pub fn look(&mut self, dx: f64, dy: f64) {
        if dx != 0.0 || dy != 0.0 {
            self.recentering = false; // the player took the camera back
        }
        self.cam_yaw += dx as f32 * chase::SENSITIVITY;
        let p = self.cam_pitch - dy as f32 * chase::SENSITIVITY;
        self.cam_pitch = p.clamp(chase::MIN_PITCH, chase::MAX_PITCH);
    }

    /// Swings the chase camera back behind Sebastian (R3); it follows him again afterwards.
    pub fn recenter(&mut self) {
        self.recentering = true;
    }

    /// Points the camera along a grid direction, for a fresh life or dungeon.
    pub fn aim_along(&mut self, dir: Dir) {
        self.cam_yaw = dir_to_yaw(dir);
        self.cam_pitch = chase::REST_PITCH;
    }

    // ------------------------------------------------------------- camera math

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.cam_pos + self.shake_offset, self.cam_look + self.shake_offset * 0.6, Vec3::Y)
    }

    pub fn projection(&self) -> Mat4 {
        Mat4::perspective_rh(FOV_DEGREES.to_radians(), self.aspect.max(0.01), NEAR, FAR)
    }

    pub fn view_proj(&self) -> Mat4 {
        self.projection() * self.view_matrix()
    }

    pub fn camera_position(&self) -> Vec3 {
        self.cam_pos
    }

    /// World point to window coordinates (logical pixels), or None if behind the camera.
    pub fn project(&self, p: Vec3, width: f32, height: f32) -> Option<(f32, f32)> {
        let clip = self.view_proj() * Vec4::new(p.x, p.y, p.z, 1.0);
        if clip.w <= 1e-5 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        if ndc.z > 1.0 {
            return None;
        }
        Some(((ndc.x + 1.0) / 2.0 * width, (1.0 - ndc.y) / 2.0 * height))
    }

    // ----------------------------------------------------------------- emit

    /// Everything that moves or animates, plus the light list, for one rendered frame.
    pub fn emit(&self, s: &SceneState, f: &mut Frame) {
        f.clear();
        f.skip_static = self.showcase || self.forge;
        if self.forge {
            self.emit_forge(s, f);
            return;
        }
        f.atmosphere = if self.meadow && !self.showcase { meadow_atmosphere() } else { Atmosphere::forest() };
        let nv = (s.hero.fx.night_vision as f32).min(1.0);
        if nv > 0.0 && !self.meadow && !self.showcase {
            let a = &mut f.atmosphere;
            a.ambient *= 1.0 + 3.0 * nv;
            a.hemi_sky *= 1.0 + 1.4 * nv;
            a.hemi_ground *= 1.0 + 2.5 * nv;
            a.sun *= 1.0 + 1.0 * nv;
            a.fog_near += 10.0 * nv;
            a.fog_far += 20.0 * nv;
        }
        if self.showcase {
            self.emit_stage(s, f);
            return;
        }
        let time = s.time as f32;
        let grid = s.grid;
        let cube = MeshKind::Cube as usize;
        f.coin_pulse = 0.35 + (time * 3.0).sin() * 0.2;

        // --- hero
        let hero = s.hero;
        let hp = hero.pos();
        let (hx, hz) = (hp.x as f32, hp.y as f32);
        let bob = if hero.airborne() { 0.0 } else { hero.motion as f32 * self.hero_phase.cos().abs() * 0.05 };
        let hero_pos = Vec3::new(hx, hero.altitude as f32 * STEP + bob, hz);
        let hero_group = Mat4::from_translation(hero_pos) * Mat4::from_rotation_y(self.hero_yaw);
        let blink = hero.invulnerable() && !hero.invincible() && ((time * 16.0).floor() as i64).rem_euclid(2) == 0;
        let ghost = hero.invisible();
        let (mut emissive, light_color) = if hero.invincible() {
            (lin(INVINCIBLE) * (0.45 + (time * 12.0).sin() * 0.25), lin(INVINCIBLE))
        } else {
            (Vec3::ZERO, lin(HERO_LIGHT))
        };
        if hero.fx.strength > 0.0 {
            emissive += lin(0xff2a1a) * (0.12 + (time * 6.0).sin().abs() * 0.12);
        }
        // Iron skin turns him to grey steel.
        let tint = if hero.fx.iron_skin > 0.0 { Vec3::new(0.62, 0.68, 0.78) } else { Vec3::ONE };
        if hero.fx.iron_skin > 0.0 {
            emissive += Vec3::splat(0.05);
        }
        if hero.alive && !blink {
            let alpha = if ghost { 0.3 + (time * 6.0).sin() * 0.08 } else { 1.0 };
            let root = hero_group * Mat4::from_translation(Vec3::new(0.0, 0.0, self.hero_lunge)) * euler(self.hero_root_rot);
            let target = if ghost { &mut f.transparent[cube] } else { &mut f.opaque[cube] };
            emit_rig(target, self.hero_rig(hero), &self.hero_pose, root, tint, alpha, emissive);
        }
        self.emit_hero_fx(hero, hero_pos, time, f);
        let mut hero_light = if hero.alive { 8.0 } else { 0.0 };
        if ghost {
            hero_light = 3.0;
        }
        if self.meadow {
            hero_light *= 0.25; // broad daylight; the lantern barely shows
        }
        // His lantern rides well above the horned helmet, so it lights him without bleaching him.
        let to_cam = {
            let d = self.cam_pos - hero_pos;
            let flat = Vec3::new(d.x, 0.0, d.z);
            if flat.length_squared() > 1e-4 { flat.normalize() } else { Vec3::Z }
        };
        f.lights.push(Light { pos: hero_pos + Vec3::new(0.0, 2.3, 0.0) + to_cam * 0.9, color: light_color * hero_light * 2.2, distance: 8.5 });

        if self.slash_t >= 0.0 {
            // The blade's trail: a bright leading arc and fading echoes behind it, swept across the front.
            let p = self.slash_t / SLASH_SECONDS;
            let q = ((p - 0.17) / 0.33).clamp(0.0, 1.0);
            let env = if p < 0.15 { 0.0 } else { (1.0 - ((p - 0.15) / 0.55)).clamp(0.0, 1.0) };
            let side = self.slash_side;
            let size = 1.0 + 0.22 * (hero.axe_level.clamp(1, 3) - 1) as f32;
            let axe_color = match hero.axe_level {
                1 => lin(SLASH),
                2 => lin(0xffb0a0),
                _ => lin(0xffe28a),
            };
            for k in 0..5 {
                let qk = (q - k as f32 * 0.09).clamp(0.0, 1.0);
                let sweep = side * (1.15 - 2.3 * (1.0 - (1.0 - qk).powi(3)));
                let alpha = env * (0.85 - k as f32 * 0.16);
                if alpha <= 0.01 {
                    continue;
                }
                let m = hero_group
                    * Mat4::from_translation(Vec3::new(0.0, 0.45 - k as f32 * 0.02, 0.12))
                    * Mat4::from_rotation_y(sweep)
                    * Mat4::from_rotation_z(side * 0.32)
                    * euler(Vec3::new(-PI / 2.0, 0.0, PI))
                    * Mat4::from_scale(Vec3::splat((1.08 + k as f32 * 0.03) * size));
                let color = if k == 0 { lin(0xffffff) } else { axe_color };
                f.transparent[MeshKind::Ring as usize].push(Instance::new(m, color, alpha, Vec3::ZERO, SHADE_UNLIT));
            }
        }

        if hero.alive {
            // A drop shadow that stays on the floor while he jumps, so height reads clearly.
            let ground_h = floor_at(grid, hx, hz);
            let lift = (hero.altitude as f32 * STEP - ground_h).max(0.0);
            let sh = (1.0 - lift * 0.5).max(0.5);
            let m = Mat4::from_translation(Vec3::new(hx, ground_h + 0.015, hz))
                * Mat4::from_rotation_x(-PI / 2.0)
                * Mat4::from_scale(Vec3::new(0.32 * sh, 0.32 * sh, 1.0));
            f.transparent[MeshKind::Disc as usize].push(Instance::new(m, Vec3::ZERO, 0.45 * sh, Vec3::ZERO, SHADE_UNLIT));
        }

        // --- enemies
        for (i, v) in self.enemy_views.iter().enumerate() {
            if i >= s.enemies.len() {
                break;
            }
            let group = Mat4::from_translation(v.group_pos) * Mat4::from_rotation_y(v.yaw);
            if v.visible {
                let root = group * euler(v.root_rot) * Mat4::from_scale(v.root_scale);
                emit_rig(&mut f.opaque[cube], self.models.rig(v.rig), &v.pose, root, v.mat_color, 1.0, v.emissive);
            }
            if v.light_intensity > 0.0 {
                f.lights.push(Light { pos: v.group_pos + Vec3::new(0.0, 0.6, 0.0), color: v.light_color * v.light_intensity, distance: 4.0 });
            }
        }

        // --- remedies
        let mut remedy_points = Vec::new();
        let remedy_glow = lin(REMEDY) * 0.75;
        for (p, base) in &self.charges {
            let pos = Vec3::new(base.x, floor_at(grid, p.x as f32, p.y as f32) + 0.15 + (time * 3.0).sin() * 0.05, base.z);
            let m = Mat4::from_translation(pos) * Mat4::from_rotation_y(time * 1.5);
            emit_mesh(&mut f.opaque[cube], &self.models.medkit, m, Vec3::ONE, 1.0, remedy_glow);
            remedy_points.push(pos);
        }

        // --- shrines
        let mut shrine_points = Vec::new();
        for (i, d) in s.diamonds.iter().enumerate() {
            let base = Vec3::new(d.tile.x as f32, floor_at(grid, d.tile.x as f32, d.tile.y as f32), d.tile.y as f32);
            let ped = Mat4::from_translation(base + Vec3::new(0.0, 0.06, 0.0));
            let ped_color = lin(if d.kind == DiamondKind::Weapon { 0x3b4a6a } else { 0x5a4a2a });
            f.opaque[MeshKind::Cylinder as usize].push(Instance::new(ped, ped_color, 1.0, Vec3::ZERO, SHADE_LIT));
            if !d.active {
                continue;
            }
            let hex = d.current().color();
            let spin = self.diamond_spin.get(i).copied().unwrap_or(0.0);
            let m = Mat4::from_translation(base + Vec3::new(0.0, 0.45 + (time * 2.0 + d.tile.x as f32).sin() * 0.06, 0.0))
                * Mat4::from_rotation_y(spin);
            emit_mesh(&mut f.opaque[cube], &self.models.diamond, m, lin(hex), 1.0, lin(hex) * 0.8);
            shrine_points.push((base, lin(hex)));
        }

        // --- potions
        for p in s.potions {
            if p.taken {
                continue;
            }
            let hex = p.def().color;
            let pos = Vec3::new(
                p.tile.x as f32,
                floor_at(grid, p.tile.x as f32, p.tile.y as f32) + 0.05 + (time * 2.5 + p.tile.x as f32).sin() * 0.04,
                p.tile.y as f32,
            );
            let m = Mat4::from_translation(pos) * Mat4::from_rotation_y(time * 1.2);
            if let Some(mesh) = self.models.bottles.get(&hex) {
                emit_mesh(&mut f.opaque[cube], mesh, m, Vec3::ONE, 1.0, lin(hex) * 0.5);
            }
        }

        // --- bombs
        for b in s.bombs {
            let on = b.is_flash_on(s.time);
            let pos = Vec3::new(b.tile.x as f32, floor_at(grid, b.tile.x as f32, b.tile.y as f32), b.tile.y as f32);
            let tint = if on { Vec3::ONE } else { lin(0x9a9a9a) };
            let glow = if on { Vec3::splat(0.9) } else { Vec3::ZERO };
            emit_mesh(&mut f.opaque[cube], &self.models.bomb, Mat4::from_translation(pos), tint, 1.0, glow);
        }

        // --- projectiles
        for p in s.projectiles {
            let (px, pz) = (p.pos.x as f32, p.pos.y as f32);
            let pos = Vec3::new(px, floor_at(grid, px, pz) + 0.45, pz);
            let yaw = (p.heading.x as f32).atan2(p.heading.y as f32);
            if self.emit_forged_shot(p.kind, pos, yaw, time, f) {
                continue;
            }
            let group = Mat4::from_translation(pos) * Mat4::from_rotation_y(yaw);
            let color = lin(p.def.color);
            let (mesh, spin, glow) = match p.kind {
                RangedKind::Boomerang => (&self.models.boomerang, 14.0, Vec3::ZERO),
                RangedKind::Fireball => (&self.models.fireball, 6.0, color * 1.2),
                RangedKind::Crescent => (&self.models.crescent, 0.0, color * 0.9),
                _ => (&self.models.arrow, 0.0, Vec3::ZERO),
            };
            let inner = group * Mat4::from_rotation_y(time * spin);
            emit_mesh(&mut f.opaque[cube], mesh, inner, Vec3::ONE, 1.0, glow);
            if p.kind == RangedKind::Fireball {
                f.lights.push(Light { pos: pos + Vec3::new(0.0, 0.15, 0.0), color: color * 5.0, distance: 4.0 });
            }
        }

        // --- slime puddles
        for (tile, life) in s.puddles.iter() {
            let opacity = 0.55 * (life as f32 / (PUDDLE_SECONDS as f32 * 0.4)).min(1.0);
            let wobble = 1.0 + (time * 3.0 + tile.x as f32).sin() * 0.05;
            let m = Mat4::from_translation(Vec3::new(tile.x as f32, floor_at(grid, tile.x as f32, tile.y as f32) + 0.02, tile.y as f32))
                * Mat4::from_rotation_x(-PI / 2.0)
                * Mat4::from_scale(Vec3::new(0.42 * wobble, 0.42 / wobble, 1.0));
            f.transparent[MeshKind::Disc as usize].push(Instance::new(m, lin(0x5cff6a), opacity, Vec3::ZERO, SHADE_UNLIT));
        }

        // --- debris and blasts
        for d in &self.debris {
            let m = Mat4::from_translation(d.pos) * euler(d.rot) * Mat4::from_scale(Vec3::splat(0.16));
            f.opaque[cube].push(Instance::new(m, d.color, 1.0, Vec3::ZERO, SHADE_LIT));
        }
        for b in &self.blasts {
            let t = 1.0 - b.life / b.seconds;
            let m = Mat4::from_translation(b.pos) * Mat4::from_scale(Vec3::splat(b.radius * (1.0 + t * b.scale)));
            f.transparent[MeshKind::Sphere as usize].push(Instance::new(m, b.color, 0.9 * (1.0 - t), Vec3::ZERO, SHADE_UNLIT));
        }

        // --- impact particles
        for p in &self.particles {
            let k = (p.life / p.max).clamp(0.0, 1.0);
            let m = Mat4::from_translation(p.pos) * Mat4::from_scale(Vec3::splat(p.size * k.sqrt().max(0.2)));
            if p.glow {
                f.additive.push(Instance::new(m, p.color, k, Vec3::ZERO, SHADE_UNLIT));
            } else {
                f.opaque[cube].push(Instance::new(m, p.color, 1.0, Vec3::ZERO, SHADE_LIT));
            }
        }

        // --- a spotlight on whatever the tour is showing
        if let Some((target, t, _)) = self.tour {
            let k = (t / 0.6).min(1.0);
            f.lights.push(Light { pos: target + Vec3::new(0.3, 1.8, 0.4), color: lin(0xffe6b8) * 9.0 * k, distance: 5.0 });
        }

        // --- the boss and what it throws
        if let Some(b) = s.boss {
            self.emit_boss(b, hero_pos, time, f);
        }
        self.emit_boss_shots(s.boss_shots, hero_pos, time, f);

        // --- fireflies by night; water, butterflies and pollen by day
        if self.meadow {
            self.emit_meadow(time, f);
        }
        let mote_color = lin(MOTE);
        let mote_alpha = 0.6 + (time * 2.2).sin() * 0.25;
        for (i, b) in self.mote_base.iter().enumerate() {
            if self.meadow {
                break;
            }
            let fi = i as f32;
            let p = Vec3::new(
                b.x + (time * 0.5 + fi).sin() * 0.4,
                b.y + (time * 0.9 + fi * 0.7).sin() * 0.2,
                b.z + (time * 0.4 + fi * 1.3).cos() * 0.4,
            );
            let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.065));
            f.additive.push(Instance::new(m, mote_color, mote_alpha, Vec3::ZERO, SHADE_UNLIT));
        }

        // --- light pools, moved to the sources nearest the hero
        let focus = Vec3::new(hx, 0.0, hz);
        let torch_points: Vec<Vec3> = self.torches.values().map(|(p, _)| *p).collect();
        for (i, p) in nearest(&torch_points, focus, LIGHT_POOL_TORCHES).into_iter().enumerate() {
            let fi = i as f32;
            let intensity = 4.5 + (time * 3.0 + fi * 2.3).sin() * 0.5 + (time * 7.0 + fi).sin() * 0.3;
            f.lights.push(Light { pos: p + Vec3::new(0.0, 0.5, 0.2), color: lin(TORCH_LIGHT) * intensity, distance: 7.0 });
        }
        for p in nearest(&remedy_points, focus, LIGHT_POOL_REMEDIES) {
            f.lights.push(Light { pos: p + Vec3::new(0.0, 0.45, 0.0), color: lin(REMEDY) * (4.0 + (time * 5.0).sin() * 1.5), distance: 5.0 });
        }
        let shrine_pos: Vec<Vec3> = shrine_points.iter().map(|(p, _)| *p).collect();
        for p in nearest(&shrine_pos, focus, LIGHT_POOL_SHRINES) {
            let color = shrine_points.iter().find(|(q, _)| *q == p).map(|(_, c)| *c).unwrap_or(Vec3::ONE);
            f.lights.push(Light { pos: p + Vec3::new(0.0, 0.9, 0.0), color: color * (4.0 + (time * 4.0).sin()), distance: 5.0 });
        }
        for l in &self.effect_lights {
            if l.intensity > 0.0 {
                f.lights.push(Light { pos: l.pos, color: l.color * l.intensity, distance: l.distance });
            }
        }
    }
}

impl Default for WorldView {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------- title diorama

/// Where the title stage stands: well away from the dungeon, which is not drawn meanwhile.
const STAGE: Vec3 = Vec3::new(-40.0, 0.0, -40.0);

impl WorldView {
    /**
     * The title screen's stage: a mossy stone platform and dais in a ring of
     * trees and lanterns, Sebastian in the middle showing off his axe, his
     * foes idling around him, under a warm key light and a cool rim light.
     */
    fn emit_stage(&self, s: &SceneState, f: &mut Frame) {
        let time = s.time as f32;
        let cube = MeshKind::Cube as usize;
        let cyl = MeshKind::Cylinder as usize;
        let at = |x: f32, y: f32, z: f32| STAGE + Vec3::new(x, y, z);

        // Ground: a wide mossy platform, a stone dais, a ring of standing stones.
        let platform = Mat4::from_translation(at(0.0, -0.09, 0.0)) * Mat4::from_scale(Vec3::new(14.0, 1.5, 14.0));
        f.opaque[cyl].push(Instance::new(platform, lin(0x2a4a28), 1.0, Vec3::ZERO, SHADE_LIT));
        let dais = Mat4::from_translation(at(0.0, 0.095, 0.0)) * Mat4::from_scale(Vec3::new(4.3, 1.6, 4.3));
        f.opaque[cyl].push(Instance::new(dais, lin(0x6a6e64), 1.0, Vec3::ZERO, SHADE_LIT));
        for i in 0..14 {
            let a = i as f32 / 14.0 * TAU;
            let r = 1.85 + (i % 3) as f32 * 0.08;
            let hgt = 0.18 + (i % 4) as f32 * 0.07;
            let m = Mat4::from_translation(at(a.cos() * r, hgt / 2.0, a.sin() * r))
                * Mat4::from_rotation_y(a)
                * Mat4::from_scale(Vec3::new(0.26, hgt, 0.2));
            let shade = if i % 2 == 0 { 0x5a5e56 } else { 0x4a4e48 };
            f.opaque[cube].push(Instance::new(m, lin(shade), 1.0, Vec3::ZERO, SHADE_LIT));
        }
        // A ring of trees, and two lantern posts.
        for i in 0..14 {
            let a = i as f32 / 14.0 * TAU + 0.2;
            // Leave the camera's side of the ring open (the camera sits toward +z).
            if a.sin() > 0.55 {
                continue;
            }
            let r = 4.6 + (i % 2) as f32 * 0.9;
            push_field(&mut f.opaque[cube], &self.models.tree, at(a.cos() * r, 0.0, a.sin() * r), a * 3.0, 1.9 + (i % 3) as f32 * 0.3, Vec3::ONE, Vec3::ZERO, SHADE_LIT);
        }
        let lanterns = [at(-2.3, 0.0, 1.9), at(2.4, 0.0, -1.6)];
        for p in lanterns {
            // Iron braziers: a dark post and a glowing coal bed on top.
            let post = Mat4::from_translation(p + Vec3::new(0.0, 0.45, 0.0)) * Mat4::from_scale(Vec3::new(0.12, 0.9, 0.12));
            f.opaque[cube].push(Instance::new(post, lin(0x2e2a26), 1.0, Vec3::ZERO, SHADE_LIT));
            let bowl = Mat4::from_translation(p + Vec3::new(0.0, 0.93, 0.0)) * Mat4::from_scale(Vec3::new(1.2, 0.8, 1.2));
            f.opaque[cyl].push(Instance::new(bowl, lin(0x3a3530), 1.0, Vec3::ZERO, SHADE_LIT));
            for k in 0..5 {
                let a = time * 3.0 + k as f32 * 1.3;
                let fp = p + Vec3::new(a.cos() * 0.08, 1.02 + (time * 5.0 + k as f32).sin().abs() * 0.12, a.sin() * 0.08);
                let m = Mat4::from_translation(fp) * Mat4::from_scale(Vec3::splat(0.07));
                f.additive.push(Instance::new(m, lin(0xffa040) * 1.4, 0.8, Vec3::ZERO, SHADE_UNLIT));
            }
        }
        // A little hoard glinting on the dais.
        for (i, (x, z)) in [(-0.6, -0.35), (0.55, -0.45), (0.15, -0.7)].into_iter().enumerate() {
            let pulse = lin(0xf2c53d) * (0.4 + (time * 3.0 + i as f32).sin() * 0.2);
            push_field(&mut f.opaque[cube], &self.models.nugget, at(x, 0.19, z), i as f32 * 2.1, 0.75, Vec3::ONE, pulse, SHADE_LIT);
        }

        // Sebastian, turning a little as if looking around.
        let hero = s.hero;
        let yaw = (time * 0.4).sin() * 0.3;
        let group = Mat4::from_translation(at(0.0, 0.19, 0.0)) * Mat4::from_rotation_y(yaw);
        let root = group * Mat4::from_translation(Vec3::new(0.0, 0.0, self.hero_lunge)) * euler(self.hero_root_rot);
        let breathe = Vec3::new(0.0, (time * 2.0).sin() * 0.006, 0.0);
        let mut pose = self.hero_pose.clone();
        if let Some(h) = self.models.popolon.index("head") {
            pose.pos[h] += breathe;
            pose.rot[h].y = (time * 0.7).sin() * 0.25;
        }
        let emissive = if hero.invincible() { lin(INVINCIBLE) * 0.4 } else { Vec3::ZERO };
        emit_rig(&mut f.opaque[cube], &self.models.popolon, &pose, root, Vec3::ONE, 1.0, emissive);
        if self.slash_t >= 0.0 {
            let p = self.slash_t / SLASH_SECONDS;
            let env = if p < 0.15 { 0.0 } else { (1.0 - ((p - 0.15) / 0.55)).clamp(0.0, 1.0) };
            let q = ((p - 0.17) / 0.33).clamp(0.0, 1.0);
            let sweep = self.slash_side * (1.15 - 2.3 * (1.0 - (1.0 - q).powi(3)));
            let m = group
                * Mat4::from_translation(Vec3::new(0.0, 0.5, 0.12))
                * Mat4::from_rotation_y(sweep)
                * euler(Vec3::new(-PI / 2.0, 0.0, PI))
                * Mat4::from_scale(Vec3::splat(1.15));
            f.transparent[MeshKind::Ring as usize].push(Instance::new(m, lin(0xffffff), env * 0.8, Vec3::ZERO, SHADE_UNLIT));
        }

        // His foes, idling around the dais.
        let draugr = self.models.rig(RigId::Skeleton);
        let mut dp = Pose::rest(draugr);
        let sway = (time * 1.6).sin();
        dp.rot_x(draugr, "leftArm", -0.35 + sway * 0.15);
        dp.rot_x(draugr, "rightArm", -0.6 - sway * 0.2);
        dp.rot_x(draugr, "head", (time * 0.9).sin() * 0.2);
        let droot = Mat4::from_translation(at(-1.0, 0.0, -0.65)) * Mat4::from_rotation_y(0.7) * Mat4::from_rotation_z(sway * 0.05);
        emit_rig(&mut f.opaque[cube], draugr, &dp, droot, Vec3::ONE, 1.0, Vec3::ZERO);

        for (id, pos, phase, facing) in [
            (RigId::Bat, Vec3::new(1.05, 1.35, -0.7), 0.0, -0.7),
            (RigId::Vampire, Vec3::new(-0.95, 1.8, -1.2), 1.7, 0.6),
        ] {
            let rig = self.models.rig(id);
            let mut rp = Pose::rest(rig);
            let flap = (time * 13.0 + phase).sin() * 0.8;
            if let Some(l) = rig.index("leftWing") {
                rp.rot[l].z = -flap;
            }
            if let Some(r) = rig.index("rightWing") {
                rp.rot[r].z = flap;
            }
            let bob = (time * 1.8 + phase).sin() * 0.12 + (time * 13.0 + phase).sin() * 0.03;
            let m = Mat4::from_translation(at(pos.x, pos.y + bob, pos.z)) * Mat4::from_rotation_y(facing) * Mat4::from_rotation_x(0.25);
            emit_rig(&mut f.opaque[cube], rig, &rp, m, Vec3::ONE, 1.0, Vec3::ZERO);
        }

        let slime = self.models.rig(RigId::Slime);
        let sq = (time * 2.6).sin();
        let sm = Mat4::from_translation(at(0.95, 0.0, 0.25)) * Mat4::from_rotation_y(-0.8) * Mat4::from_scale(Vec3::new(1.0 - sq * 0.12, 1.0 + sq * 0.25, 1.0 - sq * 0.12));
        emit_rig(&mut f.opaque[cube], slime, &Pose::rest(slime), sm, Vec3::ONE, 1.0, Vec3::ZERO);
        // Its puddle.
        let pm = Mat4::from_translation(at(0.95, 0.2, 0.25)) * Mat4::from_rotation_x(-PI / 2.0) * Mat4::from_scale(Vec3::new(0.5, 0.45, 1.0));
        f.transparent[MeshKind::Disc as usize].push(Instance::new(pm, lin(0x5cff6a), 0.35, Vec3::ZERO, SHADE_UNLIT));

        // Fireflies.
        for i in 0..50 {
            let fi = i as f32;
            let a = fi * 2.399;
            let r = 1.5 + (fi * 0.37) % 3.2;
            let p = at(a.cos() * r + (time * 0.5 + fi).sin() * 0.3, 0.5 + (fi * 0.61) % 2.0 + (time * 0.9 + fi).sin() * 0.15, a.sin() * r);
            let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.05));
            f.additive.push(Instance::new(m, lin(MOTE), 0.55 + (time * 2.2 + fi).sin() * 0.3, Vec3::ZERO, SHADE_UNLIT));
        }

        // Light: warm lanterns as the key, a cool moon rim from behind, glows from the cast.
        let flicker = 1.0 + (time * 7.0).sin() * 0.05 + (time * 3.1).sin() * 0.04;
        f.lights.push(Light { pos: at(-1.8, 2.3, 2.6), color: lin(0xffc27a) * 26.0 * flicker, distance: 10.0 });
        f.lights.push(Light { pos: at(2.6, 2.8, -2.6), color: lin(0x8fb4ff) * 22.0, distance: 10.0 });
        f.lights.push(Light { pos: at(0.0, 3.2, 1.2), color: lin(HERO_LIGHT) * 7.0, distance: 8.0 });
        for p in lanterns {
            f.lights.push(Light { pos: p + Vec3::new(0.0, 1.3, 0.0), color: lin(TORCH_LIGHT) * 4.0 * flicker, distance: 5.0 });
        }
        f.lights.push(Light { pos: at(0.95, 0.5, 0.25), color: lin(0x5cff6a) * 1.5, distance: 2.5 });
        f.lights.push(Light { pos: at(-1.1, 1.0, -0.5), color: lin(0x6ff0ff) * 1.5, distance: 2.5 });
        for l in &self.effect_lights {
            if l.intensity > 0.0 {
                f.lights.push(Light { pos: l.pos, color: l.color * l.intensity, distance: l.distance });
            }
        }
    }
}

// ---------------------------------------------------------------- helpers

fn euler(r: Vec3) -> Mat4 {
    Mat4::from_quat(Quat::from_euler(EulerRot::XYZ, r.x, r.y, r.z))
}

#[allow(clippy::too_many_arguments)]
fn push_field(out: &mut Vec<Instance>, mesh: &Mesh, pos: Vec3, rot_y: f32, scale: f32, tint: Vec3, emissive: Vec3, shade: f32) {
    let q = Quat::from_rotation_y(rot_y);
    for (off, c) in &mesh.cubes {
        let p = pos + q * (*off * scale);
        let m = Mat4::from_scale_rotation_translation(Vec3::splat(mesh.size * scale), q, p);
        out.push(Instance::new(m, *c * tint, 1.0, emissive, shade));
    }
}

fn emit_mesh(out: &mut Vec<Instance>, mesh: &Mesh, base: Mat4, tint: Vec3, alpha: f32, emissive: Vec3) {
    let s = Mat4::from_scale(Vec3::splat(mesh.size));
    for (off, c) in &mesh.cubes {
        out.push(Instance::new(base * Mat4::from_translation(*off) * s, *c * tint, alpha, emissive, SHADE_LIT));
    }
}

fn emit_rig(out: &mut Vec<Instance>, rig: &Rig, pose: &Pose, root: Mat4, tint: Vec3, alpha: f32, emissive: Vec3) {
    let s = Mat4::from_scale(Vec3::splat(rig.size));
    for (i, part) in rig.parts.iter().enumerate() {
        let pm = root * Mat4::from_translation(pose.pos[i]) * euler(pose.rot[i]);
        for (off, c) in &part.cubes {
            out.push(Instance::new(pm * Mat4::from_translation(*off) * s, *c * tint, alpha, emissive, SHADE_LIT));
        }
    }
}

/// Floor height in world units under a point, as the web build rounds it.
pub fn floor_at(grid: &Grid, x: f32, y: f32) -> f32 {
    grid.elevation(js_round(x as f64) as i32, js_round(y as f64) as i32) as f32 * STEP
}

/**
 * How far back the camera can sit before undergrowth comes between it and
 * Popolon. Walls are whole tiles, so the ray is walked against the grid.
 */
fn clear_behind(grid: &Grid, pivot: Vec3, dir: Vec3) -> f32 {
    let steps = 14;
    for i in 1..=steps {
        let d = chase::DISTANCE * i as f32 / steps as f32;
        let x = js_round((pivot.x - dir.x * d) as f64) as i32;
        let z = js_round((pivot.z - dir.z * d) as f64) as i32;
        let y = pivot.y - dir.y * d;
        if grid.get(x, z) != Tile::Rock {
            continue;
        }
        if y > grid.elevation(x, z) as f32 * STEP + WALL_TOP {
            continue;
        }
        return chase::MIN_DISTANCE.max(d - chase::DISTANCE / steps as f32);
    }
    chase::DISTANCE
}

/// The direction from `target` with the most open ground, so a close camera sits over a road.
fn open_angle(grid: &Grid, target: Vec3) -> f32 {
    let mut best = (0, PI / 2.0);
    for k in 0..16 {
        let a = k as f32 / 16.0 * TAU;
        let mut open = 0;
        for d in [0.8f32, 1.6, 2.4, 3.2] {
            let x = js_round((target.x + a.cos() * d) as f64) as i32;
            let z = js_round((target.z + a.sin() * d) as f64) as i32;
            if grid.get(x, z) != Tile::Rock {
                open += 1;
            } else {
                break;
            }
        }
        // Prefer looking in from the south (the usual view) when it is a tie.
        let score = open * 10 + if a.sin() > 0.3 { 1 } else { 0 };
        if score > best.0 {
            best = (score, a);
        }
    }
    best.1
}

/// Camera yaw for a facing: forward = (cos yaw, 0, sin yaw) in world XZ.
pub fn dir_to_yaw(d: Dir) -> f32 {
    let (x, y) = d.or_down().vec();
    (y as f32).atan2(x as f32)
}

pub fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    let mut d = (b - a) % TAU;
    if d > PI {
        d -= TAU;
    }
    if d < -PI {
        d += TAU;
    }
    a + d * t
}

/// Models face +z (grid "down"); rotate so they face the movement direction.
pub fn dir_to_rotation(d: Dir) -> f32 {
    match d {
        Dir::Up => PI,
        Dir::Right => PI / 2.0,
        Dir::Left => -PI / 2.0,
        _ => 0.0,
    }
}

/// The `count` points closest to `focus` in the ground plane, nearest first.
fn nearest(points: &[Vec3], focus: Vec3, count: usize) -> Vec<Vec3> {
    let mut v: Vec<(f32, Vec3)> = points
        .iter()
        .map(|p| ((p.x - focus.x).powi(2) + (p.z - focus.z).powi(2), *p))
        .collect();
    if v.len() > count {
        v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        v.truncate(count);
    }
    v.into_iter().map(|(_, p)| p).collect()
}

/// ToInt32, the way JavaScript's `| 0` truncates.
fn to_int32(v: f64) -> i32 {
    (v.trunc().rem_euclid(4_294_967_296.0) as u64 as u32) as i32
}

/// The web build's tile hash, including its float rounding, so woods grow in the same places.
pub fn hash(x: f64, y: f64) -> f64 {
    let h = to_int32(x * 374_761_393.0 + y * 668_265_263.0);
    let a = (h ^ ((h as u32 >> 13) as i32)) as f64;
    let h2 = to_int32(a * 1_274_126_177.0) as u32;
    ((h2 ^ (h2 >> 16)) as f64) / 4_294_967_296.0
}

/// Three.js Color.offsetHSL on a linear color.
pub fn offset_hsl(c: Vec3, dh: f32, ds: f32, dl: f32) -> Vec3 {
    let (r, g, b) = (c.x, c.y, c.z);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (min + max) / 2.0;
    let (mut h, mut s) = (0.0, 0.0);
    if min != max {
        let delta = max - min;
        s = if l <= 0.5 { delta / (max + min) } else { delta / (2.0 - max - min) };
        h = if max == r {
            (g - b) / delta + if g < b { 6.0 } else { 0.0 }
        } else if max == g {
            (b - r) / delta + 2.0
        } else {
            (r - g) / delta + 4.0
        };
        h /= 6.0;
    }
    let h = (h + dh).rem_euclid(1.0);
    let s = (s + ds).clamp(0.0, 1.0);
    let l = (l + dl).clamp(0.0, 1.0);
    if s == 0.0 {
        return Vec3::splat(l);
    }
    let p = if l <= 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let q = 2.0 * l - p;
    let hue = |mut t: f32| {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            return q + (p - q) * 6.0 * t;
        }
        if t < 0.5 {
            return p;
        }
        if t < 2.0 / 3.0 {
            return q + (p - q) * 6.0 * (2.0 / 3.0 - t);
        }
        q
    };
    Vec3::new(hue(h + 1.0 / 3.0), hue(h), hue(h - 1.0 / 3.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_matches_the_web_build() {
        // Values printed by the TypeScript hash().
        assert!((hash(3.0, 7.0) - 0.060_723_933_158_442_38).abs() < 1e-12, "{}", hash(3.0, 7.0));
        assert!((hash(26.0, 22.0) - 0.367_933_403_700_590_13).abs() < 1e-12, "{}", hash(26.0, 22.0));
    }

    #[test]
    fn a_tree_on_the_sight_line_blocks_it() {
        let tree = Tree { range: (0, 0), base: Vec3::new(2.0, 0.7, 3.0), radius: 0.35, height: 1.1, fade: 1.0, alive: true };
        let eye = Vec3::new(2.0, 3.1, 0.9);
        assert!(tree.blocks(eye, Vec3::new(2.0, 0.6, 4.0)));
        assert!(!tree.blocks(eye, Vec3::new(5.0, 0.6, 4.0)), "a tree off to the side does not block");
        assert!(!tree.blocks(Vec3::new(2.0, 3.1, 5.0), Vec3::new(2.0, 0.6, 4.0)), "nor one behind him");
    }

    #[test]
    fn trees_fade_out_of_the_way_at_spawn() {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        // At the web build's old spawn (2,4) the chase camera sits behind a wall tree;
        // it must fade, and not much else.
        let mut g = crate::game::Game::new();
        g.input.inject(crate::input::Action::Confirm);
        g.update(crate::app::STEP);
        g.hero.m.teleport(P::new(2, 4));
        g.hero.facing = Dir::Down;
        g.view.aim_along(Dir::Down);
        for _ in 0..300 {
            g.update(crate::app::STEP);
        }
        assert!(g.view.tree_fades().filter(|&f| f < 0.5).count() >= 1, "no tree faded");
        assert!(g.view.tree_fades().filter(|&f| f < 1.0).count() < 12, "only the blockers fade");
    }

    #[test]
    fn hsl_offset_round_trips() {
        let c = lin(0x3a6a32);
        let d = offset_hsl(c, 0.0, 0.0, 0.0);
        assert!((c - d).length() < 1e-5);
    }
}

