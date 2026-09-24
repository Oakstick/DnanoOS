//! The voxel dungeon as a scene: everything the old Three.js renderer did,
//! minus the GPU. Grid x maps to world x, grid y to world z, and floor
//! elevation to world y in steps of STEP. Each frame it emits instanced
//! draws and a light list; `gpu.rs` turns those into pixels.

use std::collections::HashMap;
use std::f32::consts::{PI, TAU};

use bytemuck::{Pod, Zeroable};
use glam::{EulerRot, Mat4, Quat, Vec3, Vec4};

use super::voxels::*;
use crate::logic::enemy::{Enemy, EnemyKind, EnemyMode};
use crate::logic::grid::Grid;
use crate::logic::hero::Hero;
use crate::logic::items::{Bomb, Diamond, DiamondKind, Potion, RangedKind};
use crate::logic::projectile::Projectile;
use crate::logic::rng::Rng;
use crate::logic::systems::{Frenzy, PuddleField, PUDDLE_SECONDS};
use crate::logic::types::{js_round, Dir, Tile, CARDINALS, P};

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
const VOXEL: f32 = 0.075;
const BLAST_SECONDS: f32 = 0.45;
const DEBRIS_SECONDS: f32 = 1.4;
const GRAVITY: f32 = 14.0;
const REASSEMBLE_START: f64 = 0.7;
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

#[derive(Default)]
pub struct Frame {
    pub opaque: [Vec<Instance>; MESH_KINDS],
    pub transparent: [Vec<Instance>; MESH_KINDS],
    pub additive: Vec<Instance>,
    pub lights: Vec<Light>,
    pub coin_pulse: f32,
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
    bottles: HashMap<u32, Mesh>,
}

impl Models {
    fn new() -> Self {
        let mut bottles = HashMap::new();
        for kind in [crate::logic::types::PotionKind::Life, crate::logic::types::PotionKind::Shield, crate::logic::types::PotionKind::Invisible] {
            let hex = crate::logic::items::potion_def(kind).color;
            bottles.insert(hex, build_mesh_recolored(&BOTTLE, 0.1, BOTTLE_LIQUID, hex));
        }
        Self {
            popolon: build_rig(&POPOLON, POPOLON_RIG, VOXEL),
            skeleton: build_rig(&SKELETON, SKELETON_RIG, 0.07),
            bat: build_rig(&BAT, BAT_RIG, 0.07),
            vampire: build_rig(&BAT, BAT_RIG, 0.08),
            slime: build_rig(&SLIME, &[], 0.1),
            medkit: build_mesh(&MEDKIT, 0.1),
            bomb: build_mesh(&BOMB, 0.09),
            arrow: build_mesh(&ARROW, 0.08),
            boomerang: build_mesh(&BOOMERANG, 0.07),
            fireball: build_mesh(&FIREBALL, 0.09),
            crescent: build_mesh(&CRESCENT, 0.08),
            diamond: build_mesh(&DIAMOND, 0.1),
            bottles,
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

    models: Models,
    /// Level geometry that only changes when a bomb or a pickup changes it.
    pub static_cubes: Vec<Instance>,
    pub static_dirty: bool,
    rock_index: HashMap<P, usize>,
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
    enemy_views: Vec<EnemyView>,
    diamond_spin: Vec<f32>,
    cam_pos: Vec3,
    cam_look: Vec3,
    initialized: bool,
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
            models,
            static_cubes: Vec::new(),
            static_dirty: true,
            rock_index: HashMap::new(),
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
            enemy_views: Vec::new(),
            diamond_spin: Vec::new(),
            cam_pos: Vec3::ZERO,
            cam_look: Vec3::ZERO,
            initialized: false,
            rng: Rng::from_time(),
        }
    }

    // ---------------------------------------------------------------- build

    pub fn build_level(&mut self, grid: &Grid, enemies: &[Enemy], diamonds: &[Diamond]) {
        self.static_cubes.clear();
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
        for l in self.effect_lights.iter_mut() {
            l.intensity = 0.0;
        }
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
            self.rock_index.insert(tile_key(x, y), self.static_cubes.len());
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
        if let Some(i) = self.rock_index.remove(&k) {
            self.hide_range((i, 1));
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
        self.static_cubes.push(Instance::new(m, lin(DIRT), 1.0, Vec3::ZERO, SHADE_LIT));
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
        self.update_camera(dtf, s.grid, hero);
        self.update_tree_fade(dtf, hero);
    }

    /**
     * Trees standing between the camera and Popolon dissolve so he is never
     * lost behind the woods, then grow back once the view is clear. The fade
     * is a screen-door dither in the opaque pass, so depth stays correct.
     */
    fn update_tree_fade(&mut self, dt: f32, hero: &Hero) {
        let hp = hero.pos();
        let feet = Vec3::new(hp.x as f32, hero.altitude as f32 * STEP, hp.y as f32);
        let targets = [feet + Vec3::new(0.0, 0.25, 0.0), feet + Vec3::new(0.0, 0.6, 0.0), feet + Vec3::new(0.0, 1.0, 0.0)];
        let eye = self.cam_pos;
        let step = TREE_FADE_RATE * dt;
        for i in 0..self.trees.len() {
            let t = &self.trees[i];
            if !t.alive {
                continue;
            }
            let blocking = hero.alive && targets.iter().any(|&to| t.blocks(eye, to));
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
        let hidden = matches!(e.mode(), EnemyMode::Eaten | EnemyMode::Nest);
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
        let (desired, look, pos_lerp) = if self.camera_mode == CameraMode::Third {
            if !self.mouse_look {
                self.cam_yaw = lerp_angle(self.cam_yaw, dir_to_yaw(hero.facing), 1.0 - (-chase::FOLLOW_LERP * dt).exp());
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
        self.cam_yaw += dx as f32 * chase::SENSITIVITY;
        let p = self.cam_pitch - dy as f32 * chase::SENSITIVITY;
        self.cam_pitch = p.clamp(chase::MIN_PITCH, chase::MAX_PITCH);
    }

    /// Points the camera along a grid direction, for a fresh life or dungeon.
    pub fn aim_along(&mut self, dir: Dir) {
        self.cam_yaw = dir_to_yaw(dir);
        self.cam_pitch = chase::REST_PITCH;
    }

    // ------------------------------------------------------------- camera math

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.cam_pos, self.cam_look, Vec3::Y)
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
        let (emissive, light_color) = if hero.invincible() {
            (lin(INVINCIBLE) * (0.45 + (time * 12.0).sin() * 0.25), lin(INVINCIBLE))
        } else {
            (Vec3::ZERO, lin(HERO_LIGHT))
        };
        if hero.alive && !blink {
            let alpha = if ghost { 0.3 + (time * 6.0).sin() * 0.08 } else { 1.0 };
            let root = hero_group * euler(self.hero_root_rot);
            let target = if ghost { &mut f.transparent[cube] } else { &mut f.opaque[cube] };
            emit_rig(target, &self.models.popolon, &self.hero_pose, root, Vec3::ONE, alpha, emissive);
        }
        let mut hero_light = if hero.alive { 8.0 } else { 0.0 };
        if ghost {
            hero_light = 3.0;
        }
        f.lights.push(Light { pos: hero_pos + Vec3::new(0.0, 1.2, 0.0), color: light_color * hero_light, distance: 6.0 });

        if hero.swinging() {
            let opacity = (hero.swing_progress() as f32 * PI).sin() * 0.85;
            let m = hero_group * Mat4::from_translation(Vec3::new(0.0, 0.35, 0.2)) * euler(Vec3::new(-PI / 2.0, 0.0, PI));
            f.transparent[MeshKind::Ring as usize].push(Instance::new(m, lin(SLASH), opacity, Vec3::ZERO, SHADE_UNLIT));
        }
        if hero.alive && hero.shield > 0 {
            let opacity = 0.16 + (time * 5.0).sin() * 0.06 + hero.shield as f32 * 0.03;
            let m = hero_group * Mat4::from_translation(Vec3::new(0.0, 0.5, 0.0)) * Mat4::from_scale(Vec3::splat(0.58));
            f.transparent[MeshKind::Sphere as usize].push(Instance::new(m, lin(SHIELD), opacity, Vec3::ZERO, SHADE_UNLIT));
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
            let group = Mat4::from_translation(pos) * Mat4::from_rotation_y(dir_to_rotation(p.dir));
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

        // --- fireflies
        let mote_color = lin(MOTE);
        let mote_alpha = 0.6 + (time * 2.2).sin() * 0.25;
        for (i, b) in self.mote_base.iter().enumerate() {
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
        // At spawn the chase camera sits behind a wall tree; it must fade, and not much else.
        let mut g = crate::game::Game::new();
        g.input.inject(crate::input::Action::Confirm);
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
