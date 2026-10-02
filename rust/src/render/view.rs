//! The voxel dungeon as a scene: everything the old Three.js renderer did,
//! minus the GPU. Grid x maps to world x, grid y to world z, and floor
//! elevation to world y in steps of STEP. Each frame it emits instanced
//! draws and a light list; `gpu.rs` turns those into pixels.

use std::collections::HashMap;
use std::f32::consts::{PI, TAU};

use bytemuck::{Pod, Zeroable};
use glam::{EulerRot, Mat4, Quat, Vec2, Vec3, Vec4};

use super::voxels::*;
use crate::game::Spirit;
use crate::logic::angels::{Archangel, ARCHANGELS};
use crate::logic::boss::{BossShot, Cyclops};
use crate::logic::enemy::{Enemy, EnemyKind, EnemyMode};
use crate::logic::garden::GardenPlan;
use crate::logic::gear::ALL_POTIONS;
use crate::logic::grid::Grid;
use crate::logic::hero::Hero;
use crate::logic::heroes::{HeroKind, ROSTER};
use crate::logic::items::{Bomb, Diamond, DiamondKind, Potion, RangedKind};
use crate::logic::level::Theme;
use crate::logic::projectile::Projectile;
use crate::logic::rng::Rng;
use crate::logic::sins::{Sin, SINS};
use crate::logic::systems::{Frenzy, PuddleField, PUDDLE_SECONDS};
use crate::logic::types::{js_round, Dir, Tile, CARDINALS, P, V};
use crate::logic::vault::{Loot, Vault};
use crate::logic::{gear, progression, systems};
use crate::render::foliage;
use crate::render::foliage::{FOREST, MEADOW};

mod angels;
mod animation;
mod camera;
mod duel;
mod effects;
mod emit;
mod fade;
mod forest;
mod forge;
mod fountain;
mod frame;
mod garden;
mod gate;
mod hell;
mod meadow;
mod models;
mod realms;
mod select;
mod showcase;
mod story;
mod vault;
mod vault_decor;

pub use animation::SLASH_SECONDS;
use animation::{EnemyView, Pose, TURN_RATE_HERO};
use camera::chase;
pub use camera::{CameraMode, FAR, FOV_DEGREES, NEAR};
pub use duel::crown;
use effects::{Blast, Debris, EffectLight, Particle, GRAVITY};
use fade::Tree;
pub use fade::TREE_FADE_MIN;
pub use fountain::krampus;
use frame::mix_hex;
pub use frame::{
    glow_orb, Atmosphere, Frame, Instance, Light, MeshKind, MESH_KINDS, SHADE_CLOUD, SHADE_DITHER, SHADE_GLOW, SHADE_GRASS, SHADE_JET,
    SHADE_LIT, SHADE_PULSE, SHADE_UNLIT, SHADE_WATER,
};
pub use meadow::meadow_atmosphere;
use models::{Models, RigId};
pub use select::SelectView;
use showcase::STAGE;
pub use story::StoryView;

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
const LIGHT_POOL_EFFECTS: usize = 2;

/// Ragnar's voxel size: 23 voxels make him about as tall as a tile is wide.
const VOXEL: f32 = 0.05;

// ------------------------------------------------------------ GPU records

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
    pub boss: Option<&'a Cyclops>,
    pub boss_shots: &'a [BossShot],
    pub gate: Option<GateView>,
    /// This dungeon's hidden shrine, until its relic is taken.
    pub relic: Option<P>,
    /// A decoy of Loki's trick.
    pub decoy: Option<V>,
    /// Who is being played, for his weapon and shield models and his brews' colours.
    pub hero_kind: HeroKind,
    /// The ancestral spirits at his side.
    pub spirits: &'a [Spirit],
    /// The archangels who cannot be had now (each gives once a level; Uriel also rests):
    /// a shrine showing one of them stands grey.
    pub angels_spent: [bool; 4],
    /// Holy fire burning on tiles, seconds left each.
    pub fires: &'a [(P, f64)],
    /// The red diamond key the Cyclops dropped, while it lies on the grass.
    pub red_key: Option<P>,
    /// The sin whose realm or court this is: its colour on the key he drops.
    pub realm: Option<Sin>,
    /// The Portals of Hell, while he is there.
    pub hub: Option<HubView>,
    /// The gold of the Vaults of Mammon: where each piece lies, and whether he has taken it.
    pub loot: &'a [Loot],
    /// The items the vaults' chests have thrown out.
    pub drops: &'a [crate::logic::vault::Drop],
    /// One of the six realms after the vaults, while he is in it.
    pub run: Option<&'a crate::logic::realms::Run>,
    /// The Sparring Ring's hex bolts in flight.
    pub bolts: &'a [crate::logic::combat::duelist::Bolt],
    /// The floor burning where Surtr's sword fell, the legends' marks on the scene, and what ails him.
    pub hazards: &'a [crate::logic::combat::ails::Hazard],
    pub flashes: &'a [crate::logic::combat::ails::Flash],
    pub ails: crate::logic::combat::ails::Ails,
}

/// The Portals of Hell as the view needs them: how far each portal's doors stand open (in the
/// order of `logic::sins::SINS`), the sins whose keys he holds, and how far the causeways have risen.
#[derive(Clone, Copy, Debug, Default)]
pub struct HubView {
    pub doors: [f32; 7],
    pub sins: u8,
    pub seal: f32,
}

/// The gate to the boss as the view needs it: where, how many locks are lit, how far the doors have swung.
#[derive(Clone, Copy, Debug)]
pub struct GateView {
    pub tile: P,
    pub keys: u32,
    pub total: u32,
    pub door: f32,
    /// The arena's gate: one lock, for the Cyclops' red diamond key.
    pub red: bool,
    /// That one key's colour: the red diamond's, or the sin's.
    pub key: u32,
}

// ---------------------------------------------------------------- views

pub struct WorldView {
    pub camera_mode: CameraMode,
    /// Where the chase camera looks, in world radians. The mouse drives these.
    pub cam_yaw: f32,
    pub cam_pitch: f32,
    /// True while the mouse is captured; otherwise the view trails his heading.
    pub mouse_look: bool,
    pub aspect: f32,
    /// Title screen: the camera circles Ragnar instead of chasing him.
    pub showcase: bool,
    /// Free movement: the chase camera turns after him only while he runs straight ahead.
    pub follow_heading: bool,
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
    /// The Portals of Hell: platforms over a sea of lava.
    pub hell: bool,
    /// A sin's realm or its prince's court: the sin's colour over the sky and the light.
    pub tint: Option<u32>,
    /// The gym: the hub's lava and platforms, but none of its landmarks.
    pub gym: bool,
    /// The Vaults of Mammon, while he is in them.
    vault: Option<vault::VaultView>,
    /// One of the six realms after the vaults, while he is in it.
    run_view: Option<realms::RunView>,
    /// Stream tiles (bridges included) and whether the water runs along x.
    water: Vec<(P, bool)>,
    /// Dungeon I's landscape garden, when this level is it.
    garden: Option<GardenPlan>,
    /// The Krampus fountain in the garden's middle.
    fountain: Option<fountain::FountainView>,
    /// Where the will-o'-the-wisps haunt: the churchyard's middle, at its turf.
    wisps: Option<Vec3>,
    /// Ground mist sheets, drawn every frame.
    mist: Vec<Instance>,
    /// Candle flames in shrines, temples and graves: the nearest few cast light.
    candles: Vec<Vec3>,
    /// Where the golden keys lie (each drawn while its tile still holds one).
    keys: Vec<P>,
    /// The gate to the boss, in the outer wall.
    gate_tile: Option<P>,
    /// Smoke sources: where, and how strongly.
    smoke: Vec<(Vec3, f32)>,
    /// Slam shockwaves: center and age.
    rings: Vec<(Vec3, f32)>,
    trees: Vec<Tree>,
    tree_index: HashMap<P, usize>,
    /// Static instance ranges rewritten since the last upload (tree fades).
    pub static_patches: Vec<(usize, usize)>,
    coin_ranges: HashMap<P, (usize, usize)>,
    /// Ordered, so the nearest lanterns always light in the same order.
    torches: std::collections::BTreeMap<P, (Vec3, (usize, usize))>,
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
    /// A slash keyed to frame data (startup, active, recovery seconds); none for the plain one.
    slash_timing: Option<[f32; 3]>,
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
    /// A fixed orthographic map view (half the visible height), for the isometric overview.
    map_view: Option<f32>,
    /// What the camera is looking at (kept in focus by the depth of field).
    focus_point: Vec3,
    /// The level's size in tiles, for the shadow map's coverage.
    level_dims: Vec2,
    initialized: bool,
    was_showcase: bool,
    /// R3 was pressed: the camera is swinging back behind him.
    recentering: bool,
    /// A boss the chase camera keeps in view (ground position), when there is one.
    pub focus: Option<Vec3>,
    /// The shop: the camera looks into the blacksmith's forge.
    pub forge: bool,
    /// The character screen, while it is up.
    pub select: Option<SelectView>,
    /// The character screen's cavern (built once), how far each hero has stepped forward,
    /// where the camera has drifted to, and whether the screen was up last frame.
    select_set: Vec<Instance>,
    select_step: [f32; 3],
    select_cam: f32,
    was_select: bool,
    /// Draw the heroes not yet unlocked in colour too (a capture option, to look at them).
    pub preview_heroes: bool,
    /// Scene I while it plays; its two sets and their mist (built once); how far each
    /// warrior's soul has risen; and whether the story was on last frame.
    pub story: Option<StoryView>,
    story_sets: [Vec<Instance>; 2],
    story_mist: [Vec<Instance>; 2],
    story_rise: [f32; 3],
    was_story: bool,
    /// The first-run tour: what the camera circles, seconds at this stop, and which stop.
    pub tour: Option<(Vec3, f32, usize)>,
    tour_index: Option<usize>,
    tour_angle: f32,
    was_forge: bool,
    zaps: Vec<forge::Zap>,
    /// Gabriel's thunderbolts, Raphael's beams, Uriel's waves.
    bolts: Vec<angels::Bolt>,
    beams: Vec<angels::Beam>,
    waves: Vec<angels::Wave>,
    /// Arrows of a heavenly bow falling, and sun-blade flashes fading (where, how much is left, colour).
    rains: Vec<forge::Falling>,
    flashes: Vec<(Vec3, f32, Vec3)>,
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
        let hero_pose = Pose::rest(&models.ragnar);
        Self {
            camera_mode: CameraMode::Third,
            cam_yaw: 0.0,
            cam_pitch: chase::REST_PITCH,
            mouse_look: false,
            aspect: 16.0 / 9.0,
            showcase: false,
            follow_heading: false,
            showcase_angle: 0.6,
            showcase_timer: 0.0,
            models,
            static_cubes: Vec::new(),
            static_dirty: true,
            rock_index: HashMap::new(),
            meadow: false,
            hell: false,
            tint: None,
            vault: None,
            run_view: None,
            gym: false,
            water: Vec::new(),
            garden: None,
            fountain: None,
            wisps: None,
            mist: Vec::new(),
            candles: Vec::new(),
            keys: Vec::new(),
            gate_tile: None,
            smoke: Vec::new(),
            rings: Vec::new(),
            trees: Vec::new(),
            tree_index: HashMap::new(),
            static_patches: Vec::new(),
            coin_ranges: HashMap::new(),
            torches: std::collections::BTreeMap::new(),
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
            slash_timing: None,
            hero_lunge: 0.0,
            particles: Vec::new(),
            trauma: 0.0,
            shake_clock: 0.0,
            shake_offset: Vec3::ZERO,
            enemy_views: Vec::new(),
            diamond_spin: Vec::new(),
            cam_pos: Vec3::ZERO,
            cam_look: Vec3::ZERO,
            map_view: None,
            focus_point: Vec3::ZERO,
            level_dims: Vec2::new(28.0, 24.0),
            initialized: false,
            was_showcase: false,
            recentering: false,
            focus: None,
            forge: false,
            select: None,
            select_set: select::cavern(),
            select_step: [0.0, 1.0, 0.0],
            select_cam: 0.0,
            was_select: false,
            preview_heroes: false,
            story: None,
            story_sets: [story::graveyard_set(), story::gateway_set()],
            story_mist: [story::graveyard_mist(), story::gateway_mist()],
            story_rise: [0.0; 3],
            was_story: false,
            tour: None,
            tour_index: None,
            tour_angle: 0.0,
            was_forge: false,
            zaps: Vec::new(),
            bolts: Vec::new(),
            beams: Vec::new(),
            waves: Vec::new(),
            rains: Vec::new(),
            flashes: Vec::new(),
            ghosts: Vec::new(),
            ghost_clock: 0.0,
            rng: Rng::from_time(),
        }
    }

    /// Seeds the view's random generator (particles, fireflies), for repeatable captures.
    pub fn reseed(&mut self, seed: u32) {
        self.rng = Rng::new(seed);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn build_level(
        &mut self,
        grid: &Grid,
        enemies: &[Enemy],
        diamonds: &[Diamond],
        theme: Theme,
        garden: Option<&GardenPlan>,
        gate: Option<P>,
        vault: Option<&Vault>,
        run: Option<&crate::logic::realms::Run>,
    ) {
        let meadow = theme.is_meadow();
        self.hell = theme.is_hell();
        self.gym = theme == Theme::Gym;
        // A sin's colour over the sky, but for the vaults and the realms after them, which have their own light.
        self.tint = theme.sin().filter(|_| vault.is_none() && run.is_none()).map(|s| s.color());
        self.vault = None;
        self.run_view = None;
        self.garden = garden.cloned();
        self.fountain = None;
        self.wisps = None;
        self.level_dims = Vec2::new(grid.width as f32, grid.height as f32);
        self.gate_tile = gate;
        self.keys.clear();
        self.smoke.clear();
        self.candles.clear();
        self.mist.clear();
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
        if let Some(v) = vault {
            self.build_vault(grid, v);
        } else if let Some(r) = run {
            self.build_run(grid, r);
        } else if self.hell {
            self.build_hell(grid);
        } else if meadow {
            self.build_meadow(grid);
        } else {
            self.build_forest(grid);
        }
        if let Some(g) = gate.filter(|_| vault.is_none() && run.is_none()) {
            // Eight lock lamps for the estate's keys; one for the Cyclops' red key (the vaults' far portal is a portal).
            self.build_gate(grid, g, if meadow { 1 } else { 8 });
        }
        // A thread of smoke above every candle.
        let wisps: Vec<(Vec3, f32)> = self.candles.iter().map(|c| (*c + Vec3::new(0.0, 0.12, 0.0), 0.35)).collect();
        self.smoke.extend(wisps);
        self.build_pickups(grid, enemies, diamonds);
    }

    /// Nuggets, green crosses, the cast and the fireflies: the same in every theme.
    fn build_pickups(&mut self, grid: &Grid, enemies: &[Enemy], diamonds: &[Diamond]) {
        let floor_y = |x: i32, y: i32| grid.elevation(x, y) as f32 * STEP;
        let white = Vec3::ONE;

        // Golden keys: drawn each frame, spinning, for as long as their tile holds one.
        for (x, y, t) in grid.cells() {
            if t == Tile::Nodule {
                self.keys.push(P::new(x, y));
            }
        }
        let nugget = super::props::nugget();
        let nugget_glow = lin(0xf2c53d);
        for (x, y, t) in grid.cells() {
            if t != Tile::Nodule || !self.keys.is_empty() {
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
            .map(|_| Vec3::new(self.rng.nextf() * grid.width as f32, 0.6 + self.rng.nextf() * 1.4, self.rng.nextf() * grid.height as f32))
            .collect();

        self.static_dirty = true;
        self.initialized = false;
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
        let patch = if self.meadow { lin(0x62b437) } else { lin(0x3c6c32) };
        self.static_cubes.push(Instance::new(m, patch, 1.0, Vec3::ZERO, if self.meadow { SHADE_LIT } else { SHADE_GRASS }));
        self.static_dirty = true;
    }

    pub fn remove_collectible(&mut self, x: i32, y: i32) {
        let k = tile_key(x, y);
        if let Some(range) = self.coin_ranges.remove(&k) {
            self.hide_range(range);
        }
        self.charges.retain(|(p, _)| *p != k);
    }

    /// Advances animation, effects and the camera by one fixed step.
    pub fn update(&mut self, dt: f64, s: &SceneState) {
        let dtf = dt as f32;
        let time = s.time as f32;
        self.refit_hero(s.hero_kind, s.hero);
        self.update_effects(dtf, s);

        if let Some(sv) = self.story {
            self.update_story(dtf, sv);
        }
        // The character screen: the picked hero steps forward, the others back.
        if let Some(sel) = self.select {
            let k = 1.0 - (-7.0 * dtf).exp();
            for (i, step) in self.select_step.iter_mut().enumerate() {
                let goal = if i == sel.index { 1.0 } else { 0.0 };
                *step += (goal - *step) * k;
            }
        }

        // Hero pose: stride cycle, facing, sword swing, jump tuck.
        let hero = s.hero;
        if self.showcase {
            // Ragnar shows off his axe now and then on the title screen.
            self.showcase_timer += dtf;
            if self.showcase_timer > 3.6 {
                self.showcase_timer = 0.0;
                self.sword_slash();
            }
        }
        if self.slash_t >= 0.0 {
            self.slash_t += dtf;
            if self.slash_t > self.slash_len() {
                self.slash_t = -1.0;
                self.slash_timing = None;
            }
        }
        self.hero_phase += dtf * hero.motion as f32 * if hero.hasted() { 17.0 } else { 13.0 };
        let turn_to =
            if hero.free_move { (hero.heading.cos() as f32).atan2(hero.heading.sin() as f32) } else { dir_to_rotation(hero.facing) };
        self.hero_yaw = lerp_angle(self.hero_yaw, turn_to, 1.0 - (-TURN_RATE_HERO * dtf).exp());
        self.animate_hero(hero);

        for (i, e) in s.enemies.iter().enumerate() {
            if i >= self.enemy_views.len() {
                break;
            }
            self.update_enemy(i, e, s, dtf, time);
        }
        // The vaults' gold leaves the scene as it is taken; their chests stand open.
        self.update_vault_view(s, dtf);
        self.update_run_view(s, dtf);
        for (i, d) in s.diamonds.iter().enumerate() {
            if d.active {
                if let Some(a) = self.diamond_spin.get_mut(i) {
                    *a += dtf * (2.0 + d.cycle_progress() as f32 * 8.0);
                }
            }
        }
        self.update_meadow(dtf);
        self.update_forge_fx(dtf, hero);
        self.update_angel_fx(dtf);
        self.update_camera(dtf, s.grid, hero);
        if !self.showcase {
            self.update_tree_fade(dtf, hero);
        }
        self.update_shake(dtf);
    }
}

impl Default for WorldView {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------- title diorama

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

/// A voxel cube `size` across at `off` in a posed part's frame `pm`: the same as
/// `pm * translation(off) * scale(size)`, without two matrix products per cube.
#[inline]
pub(super) fn voxel_matrix(pm: &Mat4, size: f32, off: Vec3) -> Mat4 {
    Mat4::from_cols(pm.x_axis * size, pm.y_axis * size, pm.z_axis * size, pm.transform_point3(off).extend(1.0))
}

fn emit_mesh(out: &mut Vec<Instance>, mesh: &Mesh, base: Mat4, tint: Vec3, alpha: f32, emissive: Vec3) {
    for (off, c) in &mesh.cubes {
        out.push(Instance::new(voxel_matrix(&base, mesh.size, *off), *c * tint, alpha, emissive, SHADE_LIT));
    }
}

fn emit_rig(out: &mut Vec<Instance>, rig: &Rig, pose: &Pose, root: Mat4, tint: Vec3, alpha: f32, emissive: Vec3) {
    // (A voxel lit from within, `mat::GLOW`, keeps its own light whatever tints the rest: eyes in
    // a shadow, fire on a giant dimmed in his recovery.)
    const GLOW: f32 = 16.0;
    for (i, part) in rig.parts.iter().enumerate() {
        let pm = root * Mat4::from_translation(pose.pos[i]) * euler(pose.rot[i]);
        for (j, (off, c)) in part.cubes.iter().enumerate() {
            let shade = part.shade.get(j).copied().filter(|&m| m > 0.0).unwrap_or(SHADE_LIT);
            let tint = if shade == GLOW { Vec3::ONE } else { tint };
            out.push(Instance::new(voxel_matrix(&pm, rig.size, *off), *c * tint, alpha, emissive, shade));
        }
    }
}

/// Like `emit_rig`, but drained of colour to grey and tinted (a hero not yet unlocked).
fn emit_rig_grey(out: &mut Vec<Instance>, rig: &Rig, pose: &Pose, root: Mat4, tint: Vec3) {
    for (i, part) in rig.parts.iter().enumerate() {
        let pm = root * Mat4::from_translation(pose.pos[i]) * euler(pose.rot[i]);
        for (j, (off, c)) in part.cubes.iter().enumerate() {
            let shade = part.shade.get(j).copied().filter(|&m| m > 0.0).unwrap_or(SHADE_LIT);
            // Like a statue in pale stone: the colours' lightness kept, squeezed into a narrow range.
            let grey = 0.1 + 0.3 * c.dot(Vec3::new(0.2126, 0.7152, 0.0722)).sqrt();
            out.push(Instance::new(voxel_matrix(&pm, rig.size, *off), Vec3::splat(grey) * tint, 1.0, Vec3::ZERO, shade));
        }
    }
}

/// Floor height in world units under a point, as the web build rounds it.
pub fn floor_at(grid: &Grid, x: f32, y: f32) -> f32 {
    grid.elevation(js_round(x as f64) as i32, js_round(y as f64) as i32) as f32 * STEP
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
    let mut v: Vec<(f32, Vec3)> = points.iter().map(|p| ((p.x - focus.x).powi(2) + (p.z - focus.z).powi(2), *p)).collect();
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
        let tree = Tree { range: (0, 0), base: Vec3::new(2.0, 0.7, 3.0), radius: 0.35, height: 1.1, fade: 1.0, alive: true, shades: None };
        let eye = Vec3::new(2.0, 3.1, 0.9);
        assert!(tree.blocks(eye, Vec3::new(2.0, 0.6, 4.0)));
        assert!(!tree.blocks(eye, Vec3::new(5.0, 0.6, 4.0)), "a tree off to the side does not block");
        assert!(!tree.blocks(Vec3::new(2.0, 3.1, 5.0), Vec3::new(2.0, 0.6, 4.0)), "nor one behind him");
    }

    #[test]
    fn trees_fade_out_of_the_way_at_spawn() {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        // Standing right against a wall tree, south of it and facing south, the chase camera can only
        // sit behind the tree (as at the web build's old spawn). It must fade, and not much else.
        let mut g = crate::game::Game::new();
        // Play on the title, then begin with Ragnar on the character screen.
        for _ in 0..2 {
            g.input.inject(crate::input::Action::Confirm);
            g.update(crate::app::STEP);
        }
        let grid = g.level.grid.clone();
        let mut spots: Vec<P> = g
            .view
            .tree_tiles()
            .into_iter()
            .map(|t| P::new(t.x, t.y + 1))
            .filter(|p| grid.is_walkable(p.x, p.y, false) && grid.is_walkable(p.x, p.y + 1, false))
            .collect();
        spots.sort_by_key(|p| (p.y, p.x));
        let spot = *spots.first().expect("a tree with room south of it");
        g.hero.m.teleport(spot);
        g.hero.altitude = grid.elevation(spot.x, spot.y) as f64;
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
