//! The voxel dungeon as a scene: everything the old Three.js renderer did,
//! minus the GPU. Grid x maps to world x, grid y to world z, and floor
//! elevation to world y in steps of STEP. Each frame it emits instanced
//! draws and a light list; `gpu.rs` turns those into pixels.

use std::collections::HashMap;
use std::f32::consts::{PI, TAU};

use bytemuck::{Pod, Zeroable};
use glam::{EulerRot, Mat4, Quat, Vec2, Vec3, Vec4};

use super::voxels::*;
use crate::logic::enemy::{Enemy, EnemyKind, EnemyMode};
use crate::logic::grid::Grid;
use crate::logic::hero::Hero;
use crate::logic::items::{Bomb, Diamond, DiamondKind, Potion, RangedKind};
use crate::logic::projectile::Projectile;
use crate::logic::rng::Rng;
use crate::logic::systems::{Frenzy, PuddleField, PUDDLE_SECONDS};
use crate::logic::types::{js_round, Dir, Tile, CARDINALS, P};

mod forest;
mod angels;
mod forge;
mod fountain;
mod garden;
mod gate;
mod hell;
mod meadow;
mod select;
mod story;
mod vault;
pub use fountain::krampus;
pub use select::SelectView;
pub use story::StoryView;
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
/// Ragnar's voxel size: 23 voxels make him about as tall as a tile is wide.
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
/// Past this distance from the camera a foe is drawn from its coarse copy: its fine voxels
/// would be a pixel or two across there, and a dungeon can hold a dozen foes.
const FOE_DETAIL_DISTANCE: f32 = 6.0;
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

/// Lit, and on its top face a field of grass blades drawn with parallax occlusion mapping.
pub const SHADE_GRASS: f32 = 4.0;

/// A layer of the cloud sea the dungeon floats on (or, with a seed of 20 and up, ground mist).
pub const SHADE_CLOUD: f32 = 5.0;
/// Moving water: rippled, mirroring the night sky; sheets that stand upright fall as cascades.
pub const SHADE_WATER: f32 = 20.0;
/// A jet of water: a glassy rope with bubbles rushing along it.
pub const SHADE_JET: f32 = 21.0;
/// A soft glowing orb: bright in the middle, fading to nothing at its rim (additive, on a sphere).
pub const SHADE_GLOW: f32 = 22.0;

/// A halo of light round a lamp or a wisp: `radius` across, `strength` bright.
pub fn glow_orb(f: &mut Frame, at: Vec3, radius: f32, color: Vec3, strength: f32) {
    let m = Mat4::from_scale_rotation_translation(Vec3::splat(radius), f.facing, at);
    f.glow.push(Instance::new(m, color, strength, Vec3::ZERO, SHADE_GLOW));
}

/// How see-through a tree gets while it blocks the view of Ragnar.

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
    /// An open tube along y, flaring a little toward the bottom (a veil of falling water).
    Tube = 5,
    /// A thin full ring in the XY plane (ripples).
    Annulus = 6,
}
pub const MESH_KINDS: usize = 7;

#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub pos: Vec3,
    /// Linear color times intensity.
    pub color: Vec3,
    pub distance: f32,
}

/// The sky, fog and ambient light a level is drawn under.
/// `a` mixed `k` of the way to `b`, both sRGB hex.
pub(crate) fn mix_hex(a: u32, b: u32, k: f32) -> u32 {
    let ch = |s: u32| {
        let x = ((a >> s) & 255) as f32;
        let y = ((b >> s) & 255) as f32;
        ((x + (y - x) * k.clamp(0.0, 1.0)) as u32) << s
    };
    ch(16) | ch(8) | ch(0)
}

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
    /// 1 for the starry night sky with its full moon, 0 for a plain background.
    pub night_sky: f32,
    /// How strongly bright things glow (bloom).
    pub bloom: f32,
    /// Where the moon (or at dusk the setting sun) hangs in the sky.
    pub moon_view: Vec3,
    /// The cloud sea's color.
    pub cloud_tint: Vec3,
}

impl Atmosphere {
    /// The dungeons: a moonlit forest at night.
    /// The same sky under a sin's colour, for its realm and its prince's court.
    pub fn tinted(mut self, hex: u32) -> Self {
        let c = lin(hex);
        let toward = |v: Vec3, k: f32| v.lerp(c * (v.max_element().max(0.02) / c.max_element().max(1e-3)), k);
        self.background = mix_hex(self.background, hex, 0.35);
        self.hemi_sky = toward(self.hemi_sky, 0.5);
        self.hemi_ground = toward(self.hemi_ground, 0.3);
        self.ambient = toward(self.ambient, 0.45);
        self.sun = toward(self.sun, 0.3);
        self.cloud_tint = toward(self.cloud_tint, 0.5);
        self
    }

    pub fn forest() -> Self {
        Self {
            background: BACKGROUND,
            fog_near: 14.0,
            fog_far: 34.0,
            hemi_sky: lin(0x6f8fa8) * 0.85,
            hemi_ground: lin(0x14260f),
            ambient: lin(0x2c3a2a) * 0.7,
            sun_dir: Vec3::new(-6.0, 14.0, 8.0).normalize(),
            sun: lin(MOON) * 1.15,
            exposure: 1.2,
            night_sky: 1.0,
            bloom: 1.0,
            moon_view: Vec3::new(-1.103, 0.534, -0.311).normalize(),
            cloud_tint: lin(0x161b24),
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
    /// See-through cubes drawn before every other see-through thing (the smoke wall far behind it all).
    pub transparent_early: Vec<Instance>,
    pub transparent: [Vec<Instance>; MESH_KINDS],
    pub additive: Vec<Instance>,
    /// Soft glowing orbs, drawn additively on discs facing the camera (`SHADE_GLOW`): halos round lamps, wisps.
    pub glow: Vec<Instance>,
    /// Turns a disc in the XY plane to face the camera.
    pub facing: Quat,
    pub lights: Vec<Light>,
    pub coin_pulse: f32,
    /// Seconds since start, for the twinkling stars.
    pub time: f32,
    /// The level's footprint (min x, min z, max x, max z), for the cloud sea around it.
    pub bounds: Vec4,
    /// The title diorama draws on its own; the level's static geometry sits this frame out.
    pub skip_static: bool,
    pub atmosphere: Atmosphere,
}

impl Frame {
    pub fn clear(&mut self) {
        for v in self.opaque.iter_mut().chain(self.transparent.iter_mut()) {
            v.clear();
        }
        self.transparent_early.clear();
        self.additive.clear();
        self.glow.clear();
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
    pub gate: Option<GateView>,
    /// This dungeon's hidden shrine, until its relic is taken.
    pub relic: Option<P>,
    /// A decoy of Loki's trick.
    pub decoy: Option<crate::logic::types::V>,
    /// Who is being played, for his weapon and shield models and his brews' colours.
    pub hero_kind: crate::logic::heroes::HeroKind,
    /// The ancestral spirits at his side.
    pub spirits: &'a [crate::game::Spirit],
    /// The archangels who cannot be had now (each gives once a level; Uriel also rests):
    /// a shrine showing one of them stands grey.
    pub angels_spent: [bool; 4],
    /// Holy fire burning on tiles, seconds left each.
    pub fires: &'a [(P, f64)],
    /// The red diamond key the Cyclops dropped, while it lies on the grass.
    pub red_key: Option<P>,
    /// The sin whose realm or court this is: its colour on the key he drops.
    pub realm: Option<crate::logic::sins::Sin>,
    /// The Portals of Hell, while he is there.
    pub hub: Option<HubView>,
    /// The gold of the Vaults of Mammon: where each piece lies, and whether he has taken it.
    pub loot: &'a [crate::logic::vault::Loot],
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
    Witch,
    Minion,
    Helmeted,
    Werewolf,
    Blemmyes,
    LargeBat,
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
    /// Each instance's own shade, for models with voxel materials (None: all plain lit).
    shades: Option<Vec<f32>>,
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
        // Stop a little short of Ragnar so a tree right behind him never counts.
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
    ragnar: Rig,
    skeleton: Rig,
    bat: Rig,
    vampire: Rig,
    slime: Rig,
    /// The same four at half the resolution, for foes far from the camera.
    witch: Rig,
    minion: Rig,
    helmeted: Rig,
    werewolf: Rig,
    blemmyes: Rig,
    large_bat: Rig,
    /// Every foe's coarse copy, in `RigId` order.
    foes_far: Vec<Rig>,
    medkit: Mesh,
    bomb: Mesh,
    arrow: Mesh,
    boomerang: Mesh,
    fireball: Mesh,
    crescent: Mesh,
    diamond: Mesh,
    /// The vaults' gold: a stack of coins, a heap of them, a chest (body and lid).
    coin: Mesh,
    pile: Mesh,
    chest: (Mesh, Mesh),
    tree: Mesh,
    key: Mesh,
    /// The Cyclops' red diamond key.
    /// The diamond keys the bosses drop: the Cyclops' red one, then one in each sin's colour.
    keys: Vec<Mesh>,
    /// The meadow's hedge ring, its bushes, and the forest's darker bushes: four shapes of each.
    hedges: Vec<Mesh>,
    bushes: Vec<Mesh>,
    forest_bushes: Vec<Mesh>,
    /// The seven portal doors of the Portals of Hell, each a pair of leaves carrying its sin in relief,
    /// and the sin's name in letters for the lintel.
    portal_doors: Vec<[Mesh; 2]>,
    portal_names: Vec<Mesh>,
    /// Shrine pedestals: weapon (blue runes) and item (gold runes).
    pedestals: [(Mesh, Mesh); 2],
    bottles: HashMap<u32, Mesh>,
    cyclops: Rig,
    boulder: Rig,
    /// The hero as he is equipped now (weapon in hand, shield on his back), what that is,
    /// and his weapon on its own for when he throws it. Rebuilt when his gear changes.
    hero: Rig,
    hero_key: (crate::logic::heroes::HeroKind, u8, Option<u8>),
    thrown: Rig,
    /// The other two heroes as they are shown on the title and character screens, as
    /// souls in Scene I and as ancestral spirits (Ragnar is `ragnar`).
    mehmed: Rig,
    miyamoto: Rig,
    /// Mehmed's and Miyamoto's ancestral spirits: the same at half the resolution, see-through.
    spirits: [Rig; 2],
    /// An archangel's shrine as it honours each of the four (plinth and statue).
    angel_shrines: [Rig; 4],
    /// Scene I's cast: the four friends, the old gatekeeper, his dog, and Krampus as a shadow.
    kids: [Rig; 4],
    old_man: Rig,
    dog: Rig,
    krampus: Rig,
}

impl Models {
    fn new() -> Self {
        let mut bottles = HashMap::new();
        for hero in crate::logic::heroes::ROSTER {
            for kind in crate::logic::gear::ALL_POTIONS {
                let hex = crate::logic::gear::potion(hero, kind).color;
                bottles.entry(hex).or_insert_with(|| super::props::bottle(hex));
            }
        }
        let ragnar = crate::logic::heroes::HeroKind::Ragnar;
        use super::foes::{bog_slime, draugr, raven, DRAUGR_RIG, RAVEN_RIG};
        let skeleton = build_rig(&draugr(), DRAUGR_RIG, 0.0125);
        let bat = build_rig(&raven(false), RAVEN_RIG, 0.0105);
        let vampire = build_rig(&raven(true), RAVEN_RIG, 0.0125);
        let slime = build_rig(&bog_slime(), &[], 0.01875);
        use super::foes2::{blemmyes, helmeted_skeleton, large_bat, loup_garou, minion, witch, BAT_RIG, BLEMMYES_RIG, HELMET_RIG, WITCH_RIG, WOLF_RIG};
        let witch = build_rig(&witch(), WITCH_RIG, 0.0125);
        let minion = build_rig(&minion(), &[], 0.014);
        let helmeted = build_rig(&helmeted_skeleton(), HELMET_RIG, 0.0125);
        let werewolf = build_rig(&loup_garou(), WOLF_RIG, 0.0135);
        let blemmyes = build_rig(&blemmyes(), BLEMMYES_RIG, 0.0145);
        let large_bat = build_rig(&large_bat(), BAT_RIG, 0.011);
        let foes_far = vec![
            skeleton.coarse(),
            bat.coarse(),
            vampire.coarse(),
            slime.coarse(),
            witch.coarse(),
            minion.coarse(),
            helmeted.coarse(),
            werewolf.coarse(),
            blemmyes.coarse(),
            large_bat.coarse(),
        ];
        Self {
            ragnar: forge::hero_model(ragnar, 1, Some(1)),
            hero: forge::hero_model(ragnar, 1, Some(1)),
            hero_key: (ragnar, 1, Some(1)),
            thrown: forge::thrown_model(ragnar, 1),
            skeleton,
            bat,
            vampire,
            slime,
            witch,
            minion,
            helmeted,
            werewolf,
            blemmyes,
            large_bat,
            foes_far,
            medkit: super::props::medkit(),
            bomb: super::props::bomb(),
            arrow: super::props::arrow(),
            boomerang: super::props::boomerang(),
            fireball: super::props::fireball(),
            crescent: super::props::crescent(),
            diamond: super::props::gem(),
            coin: vault::coin(),
            pile: vault::pile(),
            chest: vault::chest(),
            tree: super::props::pine(),
            key: gate::golden_key(),
            keys: std::iter::once(gate::RED_GEM).chain(crate::logic::sins::SINS.iter().map(|s| s.color())).map(gate::diamond_key).collect(),
            hedges: (0..4).map(crate::render::foliage::hedge_block).collect(),
            bushes: (0..4).map(|v| crate::render::foliage::bush(v, crate::render::foliage::MEADOW)).collect(),
            forest_bushes: (0..4).map(|v| crate::render::foliage::small_bush(v, crate::render::foliage::FOREST)).collect(),
            portal_doors: crate::logic::sins::SINS.iter().map(|&s| hell::portal_doors(s)).collect(),
            portal_names: crate::logic::sins::SINS.iter().map(|&s| hell::carved_name(s.title())).collect(),
            pedestals: [super::props::pedestal(0x6ab8ff), super::props::pedestal(0xffc84a)],
            bottles,
            cyclops: {
                use super::foes::{cyclops, cyclops_iris, cyclops_lid, CYCLOPS_EYE, CYCLOPS_RIG, CYCLOPS_VOXEL};
                build_layered_rig(
                    &cyclops(),
                    CYCLOPS_RIG,
                    &[("iris", &cyclops_iris(), CYCLOPS_EYE), ("lid", &cyclops_lid(), CYCLOPS_EYE)],
                    CYCLOPS_VOXEL,
                )
            },
            boulder: build_rig(&meadow::boulder(), &[], 0.09),
            mehmed: forge::hero_model(crate::logic::heroes::HeroKind::Mehmed, 1, Some(1)),
            miyamoto: forge::hero_model(crate::logic::heroes::HeroKind::Miyamoto, 1, Some(1)),
            spirits: [crate::logic::heroes::HeroKind::Mehmed, crate::logic::heroes::HeroKind::Miyamoto].map(|h| forge::hero_model(h, 1, Some(1)).coarse()),
            angel_shrines: crate::logic::angels::ARCHANGELS.map(|a| build_rig(&angels::angel_shrine(a), &[], angels::SHRINE_VOXEL)),
            kids: [0, 1, 2, 3].map(|i| build_rig(&super::cast::kid(i), super::cast::KID_RIG, story::KID_VOXEL)),
            old_man: build_rig(&super::cast::old_man(), super::cast::OLD_MAN_RIG, story::OLD_MAN_VOXEL),
            dog: build_rig(&super::cast::dog(), super::cast::DOG_RIG, story::DOG_VOXEL),
            krampus: build_rig(&super::cast::krampus_shade(), super::cast::KRAMPUS_RIG, story::KRAMPUS_VOXEL),
        }
    }

    fn rig(&self, id: RigId) -> &Rig {
        match id {
            RigId::Skeleton => &self.skeleton,
            RigId::Bat => &self.bat,
            RigId::Vampire => &self.vampire,
            RigId::Slime => &self.slime,
            RigId::Witch => &self.witch,
            RigId::Minion => &self.minion,
            RigId::Helmeted => &self.helmeted,
            RigId::Werewolf => &self.werewolf,
            RigId::Blemmyes => &self.blemmyes,
            RigId::LargeBat => &self.large_bat,
        }
    }

    /// A foe as it is drawn from `distance` away: in full detail near the camera, coarser far off.
    fn foe_at(&self, id: RigId, distance: f32) -> &Rig {
        if distance < FOE_DETAIL_DISTANCE {
            return self.rig(id);
        }
        &self.foes_far[id as usize]
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
    /// Stream tiles (bridges included) and whether the water runs along x.
    water: Vec<(P, bool)>,
    /// Dungeon I's landscape garden, when this level is it.
    garden: Option<crate::logic::garden::GardenPlan>,
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

    // ---------------------------------------------------------------- build

    #[allow(clippy::too_many_arguments)]
    pub fn build_level(&mut self, grid: &Grid, enemies: &[Enemy], diamonds: &[Diamond], theme: crate::logic::level::Theme, garden: Option<&crate::logic::garden::GardenPlan>, gate: Option<P>, vault: Option<&crate::logic::vault::Vault>) {
        let meadow = theme.is_meadow();
        self.hell = theme.is_hell();
        self.gym = theme == crate::logic::level::Theme::Gym;
        // A sin's colour over the sky, but for the vaults, which have their own gold light.
        self.tint = theme.sin().filter(|_| vault.is_none()).map(|s| s.color());
        self.vault = None;
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
        } else if self.hell {
            self.build_hell(grid);
        } else if meadow {
            self.build_meadow(grid);
        } else {
            self.build_forest(grid);
        }
        if let Some(g) = gate.filter(|_| vault.is_none()) {
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

    /// Drops every enemy's view (the capture's line-up of foes replaces them).
    pub fn clear_enemies(&mut self) {
        self.enemy_views.clear();
    }

    /// Adds a view for an enemy; call again for enemies that arrive mid-level. Order must match the game's list.
    pub fn add_enemy(&mut self, e: &Enemy) {
        let (rig, hover) = match e.kind() {
            EnemyKind::Skeleton => (RigId::Skeleton, 0.0),
            EnemyKind::Bat => (RigId::Bat, 0.55),
            EnemyKind::Vampire => (RigId::Vampire, 0.65),
            EnemyKind::Slime => (RigId::Slime, 0.0),
            EnemyKind::Witch => (RigId::Witch, 0.0),
            EnemyKind::Minion => (RigId::Minion, 0.0),
            EnemyKind::Helmeted => (RigId::Helmeted, 0.0),
            EnemyKind::Werewolf => (RigId::Werewolf, 0.0),
            EnemyKind::Blemmyes => (RigId::Blemmyes, 0.0),
            EnemyKind::LargeBat => (RigId::LargeBat, 0.6),
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
            if self.slash_t > SLASH_SECONDS {
                self.slash_t = -1.0;
            }
        }
        self.hero_phase += dtf * hero.motion as f32 * if hero.hasted() { 17.0 } else { 13.0 };
        let turn_to = if hero.free_move { (hero.heading.cos() as f32).atan2(hero.heading.sin() as f32) } else { dir_to_rotation(hero.facing) };
        self.hero_yaw = lerp_angle(self.hero_yaw, turn_to, 1.0 - (-TURN_RATE_HERO * dtf).exp());
        self.animate_hero(hero);

        for (i, e) in s.enemies.iter().enumerate() {
            if i >= self.enemy_views.len() {
                break;
            }
            self.update_enemy(i, e, s, dtf, time);
        }
        // The vaults' chests stand open once taken.
        if let Some(vv) = &mut self.vault {
            for (i, l) in s.loot.iter().enumerate() {
                if l.taken {
                    if let Some(a) = vv.opened.get_mut(i) {
                        *a += dtf;
                    }
                }
            }
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
        self.update_angel_fx(dtf);
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
     * Trees standing between the camera and Ragnar dissolve so he is never
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
            // What the tour is showing never fades out of its own shot.
            let shown = self.tour.is_some() && Vec2::new(t.base.x - feet.x, t.base.z - feet.z).length() < t.radius + 0.3;
            let blocking = (hero.alive || self.tour.is_some()) && !shown && targets.iter().any(|&to| t.blocks(eye, to));
            let goal = if blocking { TREE_FADE_MIN } else { 1.0 };
            let fade = if t.fade < goal { (t.fade + step).min(goal) } else { (t.fade - step).max(goal) };
            if fade == t.fade {
                continue;
            }
            let (start, len) = t.range;
            self.trees[i].fade = fade;
            let solid = fade >= 0.999;
            let shades = self.trees[i].shades.as_deref();
            for (k, inst) in self.static_cubes[start..start + len].iter_mut().enumerate() {
                inst.color[3] = fade;
                inst.emissive[3] = if !solid { SHADE_DITHER } else { shades.map_or(SHADE_LIT, |s| s[k]) };
            }
            self.static_patches.push((start, len));
        }
    }

    /// How faded each standing tree is (1 = solid), for tests and tools.
    pub fn tree_fades(&self) -> impl Iterator<Item = f32> + '_ {
        self.trees.iter().filter(|t| t.alive).map(|t| t.fade)
    }

    /// Tiles with a tree standing on them (for tests).
    #[cfg(test)]
    pub fn tree_tiles(&self) -> Vec<P> {
        self.tree_index.keys().copied().collect()
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
        if e.is_ally() {
            // Turned by Michael's fire: washed in blue light.
            let blue = lin(crate::logic::angels::Archangel::Michael.color());
            v.mat_color = Vec3::new(0.72, 0.84, 1.3);
            v.emissive = blue * (0.3 + (time * 4.0).sin() * 0.08);
            v.light_color = blue;
        } else if e.is_stunned() {
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
        let rig = &self.models.hero;
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
            self.sync_hands();
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
        self.sync_hands();
    }

    /// The weapons (layered parts of his rig) move with the arms that hold them.
    fn sync_hands(&mut self) {
        let rig = &self.models.hero;
        for (arm, held) in [("swordArm", "blade"), ("shieldArm", "offhand")] {
            if let (Some(a), Some(b)) = (rig.index(arm), rig.index(held)) {
                self.hero_pose.rot[b] = self.hero_pose.rot[a];
            }
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
        let rig = &self.models.hero;
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
            EnemyKind::Skeleton | EnemyKind::Helmeted | EnemyKind::Witch | EnemyKind::Blemmyes => {
                // A walk: the blemmyes lumbers, the witch strides, the skeletons stalk.
                let (stride, swing) = match v.kind {
                    EnemyKind::Blemmyes => (0.6, 0.35),
                    EnemyKind::Witch => (0.7, 0.3),
                    _ => (0.85, 0.5),
                };
                let s = v.phase.sin();
                v.pose.rot_x(rig, "leftLeg", s * stride);
                v.pose.rot_x(rig, "rightLeg", -s * stride);
                v.pose.rot_x(rig, "leftArm", -s * swing);
                v.pose.rot_x(rig, "rightArm", s * swing);
                v.pose.rot_x(rig, "head", (v.phase * 2.0).sin() * 0.1);
                v.root_rot.z = v.phase.sin() * if v.kind == EnemyKind::Blemmyes { 0.1 } else { 0.06 };
            }
            EnemyKind::Minion => {
                // Scurries in hops, its hat bobbing.
                let t = if moving { v.phase * 1.6 } else { time * 3.0 + i as f32 };
                let hop = t.sin();
                v.root_scale = Vec3::new(1.0 - hop * 0.06, 1.0 + hop * 0.1, 1.0 - hop * 0.06);
                v.root_rot.z = (t * 0.5).sin() * 0.12;
                if moving {
                    v.group_pos.y += hop.max(0.0) * 0.12;
                }
            }
            EnemyKind::Werewolf => {
                // A gallop: the front legs together, the back legs half a beat behind, the head low, the tail streaming.
                let t = v.phase * 1.2;
                let (front, back) = (t.sin(), (t - 1.2).sin());
                v.pose.rot_x(rig, "frontLeftLeg", front * 0.7);
                v.pose.rot_x(rig, "frontRightLeg", (t + 0.4).sin() * 0.7);
                v.pose.rot_x(rig, "backLeftLeg", -back * 0.7);
                v.pose.rot_x(rig, "backRightLeg", -(t - 0.8).sin() * 0.7);
                v.pose.rot_x(rig, "head", if moving { 0.15 + front * 0.08 } else { (time * 1.5).sin() * 0.05 });
                if let Some(k) = rig.index("tail") {
                    v.pose.rot[k].y = (time * 6.0 + i as f32).sin() * 0.3;
                }
                v.root_rot.x = if moving { front * 0.05 } else { 0.0 };
                if moving {
                    v.group_pos.y += front.max(0.0) * 0.08;
                }
            }
            EnemyKind::Bat | EnemyKind::Vampire | EnemyKind::LargeBat => {
                let beat = time * if v.kind == EnemyKind::LargeBat { 9.0 } else { 15.0 } + i as f32 * 2.0;
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
        self.level_dims = Vec2::new(grid.width as f32, grid.height as f32);
        if self.showcase != self.was_showcase || self.forge != self.was_forge || self.select.is_some() != self.was_select || self.story.is_some() != self.was_story {
            self.was_showcase = self.showcase;
            self.was_forge = self.forge;
            self.was_select = self.select.is_some();
            self.was_story = self.story.is_some();
            self.initialized = false;
        }
        if self.tour.is_none() {
            self.tour_index = None;
        }
        let story_cam = self.story.map(|sv| self.story_camera(sv));
        let (desired, look, pos_lerp) = if let Some((eye, look, _)) = story_cam {
            // The story's shots are placed exactly, cut to cut.
            self.initialized = false;
            (eye, look, 1.0)
        } else if let Some((target, t, index)) = self.tour {
            // A dramatic swoop: in from high and wide, then a slow close orbit.
            if self.tour_index != Some(index) {
                self.tour_index = Some(index);
                self.tour_angle = open_angle(grid, target);
            }
            let e = 1.0 - (1.0 - (t / 0.7).min(1.0)).powi(3);
            let a = self.tour_angle + t * 0.22;
            let r = 2.5 + 3.6 * (1.0 - e);
            let h = 2.15 + 3.0 * (1.0 - e);
            (target + Vec3::new(a.cos() * r, h, a.sin() * r), target + Vec3::new(0.0, 0.35, 0.0), 7.0 + 6.0 * e)
        } else if let Some(sel) = self.select {
            // The character screen: a low, heroic view of the line-up, drifting toward whoever is picked.
            let goal = select::slot_feet(sel.index, 1.0).x - select::SELECT_STAGE.x;
            self.select_cam += (goal * 0.32 - self.select_cam) * (1.0 - (-3.0 * dt).exp());
            self.showcase_angle += dt;
            let sway = (self.showcase_angle * 0.31).sin() * 0.07;
            let base = select::SELECT_STAGE;
            (base + Vec3::new(self.select_cam + sway, 1.0, 3.35), base + Vec3::new(self.select_cam * 1.15, 0.78, 0.0), 6.0)
        } else if self.forge {
            // The old man's crypt fills the left of the screen; his wares sit on the right.
            self.showcase_angle += dt;
            let (eye, look) = self.wares_camera(self.showcase_angle);
            (eye, look, 6.0)
        } else if self.showcase {
            // The title diorama: the camera sways gently in front of Ragnar.
            self.showcase_angle += dt;
            let a = (self.showcase_angle * 0.23).sin() * 0.5;
            (STAGE + Vec3::new(a * 1.1, 1.2, 3.9), STAGE + Vec3::new(a * 0.3, 0.72, -0.2), 6.0)
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
                    None if hero.free_move => {
                        // Free movement: trail where he runs, unless he is backing toward the camera.
                        let goal = hero.heading as f32;
                        let mut off = (goal - self.cam_yaw).rem_euclid(TAU);
                        if off > PI {
                            off = TAU - off;
                        }
                        if hero.vel > 0.3 && off < 1.75 && self.follow_heading {
                            self.cam_yaw = lerp_angle(self.cam_yaw, goal, 1.0 - (-chase::FOLLOW_LERP * 0.6 * dt).exp());
                        }
                    }
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
        self.focus_point = if let Some((_, _, focus)) = story_cam {
            focus
        } else if let Some((target, _, _)) = self.tour {
            target
        } else if let Some(sel) = self.select {
            select::slot_feet(sel.index, 1.0) + Vec3::new(0.0, 0.7, 0.0)
        } else if self.forge {
            story::GATEWAY + Vec3::new(-0.4, 0.8, -0.6)
        } else if self.showcase {
            STAGE + Vec3::new(0.0, 0.6, 0.3)
        } else {
            focus + Vec3::new(0.0, 0.5, 0.0)
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

    /// Swings the chase camera back behind Ragnar (R3); it follows him again afterwards.
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

    /// Freezes the camera on a close portrait of a point, from `angle` radians around it.
    pub fn set_closeup(&mut self, target: Vec3, angle: f32, distance: f32) {
        self.focus_point = target;
        self.cam_look = target;
        self.cam_pos = target + Vec3::new(angle.sin() * distance, distance * 0.28, angle.cos() * distance);
        self.shake_offset = Vec3::ZERO;
    }

    /// A close view like `set_closeup`, from `rise` times the distance above the point.
    pub fn set_closeup_rise(&mut self, target: Vec3, angle: f32, distance: f32, rise: f32) {
        self.set_closeup(target, angle, distance);
        self.cam_pos = target + Vec3::new(angle.sin() * distance, distance * rise, angle.cos() * distance);
    }

    /// Freezes the camera into a classic isometric view of the whole grid.
    pub fn set_map_view(&mut self, grid: &Grid) {
        let (w, h) = (grid.width as f32, grid.height as f32);
        let center = Vec3::new(w / 2.0 - 0.5, 0.0, h / 2.0 - 0.5);
        // 45 degrees around, 35.26 degrees down: the true isometric angle.
        let dir = Vec3::ONE.normalize();
        self.cam_look = center;
        self.cam_pos = center + dir * 80.0;
        self.shake_offset = Vec3::ZERO;
        // The diamond of the map is (w + h) / sqrt(2) wide and about half that tall on screen.
        let across = (w + h) / 2f32.sqrt();
        self.map_view = Some((across / self.aspect.max(0.01)).max(across * 0.5 + 2.0) * 0.53);
    }

    /// The sky's view rays: forward, and right and up scaled to the screen edges. The pitch is
    /// held a little above the horizon, so the stars stay up there even under a map camera.
    pub fn sky_basis(&self) -> (Vec3, Vec3, Vec3) {
        let d = (self.cam_look - self.cam_pos).normalize_or(Vec3::NEG_Z);
        let yaw = d.z.atan2(d.x);
        let pitch = d.y.clamp(-1.0, 1.0).asin().max(0.15);
        let fwd = Vec3::new(pitch.cos() * yaw.cos(), pitch.sin(), pitch.cos() * yaw.sin());
        let right = fwd.cross(Vec3::Y).normalize();
        let up = right.cross(fwd);
        let t = (FOV_DEGREES.to_radians() * 0.5).tan();
        (fwd, right * t * self.aspect.max(0.01), up * t)
    }

    /// The camera for reconstructing world positions from depth: position, forward, and right
    /// and up scaled to the screen edges (per unit of depth, or in world units when orthographic).
    pub fn camera_basis(&self) -> (Vec3, Vec3, Vec3, Vec3, bool) {
        let pos = self.cam_pos + self.shake_offset;
        let fwd = (self.cam_look + self.shake_offset * 0.6 - pos).normalize_or(Vec3::NEG_Z);
        let right = fwd.cross(Vec3::Y).normalize_or(Vec3::X);
        let up = right.cross(fwd);
        let aspect = self.aspect.max(0.01);
        match self.map_view {
            Some(half) => (pos, fwd, right * half * aspect, up * half, true),
            None => {
                let t = (FOV_DEGREES.to_radians() * 0.5).tan();
                (pos, fwd, right * t * aspect, up * t, false)
            }
        }
    }

    /// How far in front of the camera the in-focus plane lies; None turns depth of field off.
    pub fn focus_depth(&self) -> Option<f32> {
        if self.map_view.is_some() {
            return None;
        }
        let (pos, fwd, ..) = self.camera_basis();
        Some((self.focus_point - pos).dot(fwd).max(0.5))
    }

    /// Where hero `i` of the character screen's line-up stands, his feet in the world.
    pub fn select_feet(&self, i: usize) -> Vec3 {
        select::slot_feet(i, self.select_step[i.min(2)])
    }

    /// The region the shadow map covers: its center and radius.
    /// Where the fine shadow cascade centers: on the ground a little past where the
    /// camera looks. None where the coarse one is enough (the map, the title, the shop).
    pub fn shadow_focus(&self) -> Option<Vec3> {
        if self.forge || self.showcase || self.select.is_some() || self.story.is_some() || self.map_view.is_some() {
            return None;
        }
        let fwd = self.cam_look - self.cam_pos;
        let flat = Vec3::new(fwd.x, 0.0, fwd.z).normalize_or_zero();
        Some(Vec3::new(self.cam_look.x, self.focus_point.y, self.cam_look.z) + flat * 3.0)
    }

    pub fn shadow_box(&self) -> (Vec3, f32) {
        if let Some(sv) = self.story {
            self.story_shadow_box(sv)
        } else if self.select.is_some() {
            (select::SELECT_STAGE + Vec3::new(0.0, 0.6, -0.6), 6.5)
        } else if self.forge {
            (story::GATEWAY + Vec3::new(-0.5, 1.0, -0.5), 5.5)
        } else if self.showcase {
            (STAGE, 5.0)
        } else {
            let d = self.level_dims;
            (Vec3::new(d.x / 2.0 - 0.5, 0.5, d.y / 2.0 - 0.5), d.length() / 2.0 + 2.5)
        }
    }

    pub fn projection(&self) -> Mat4 {
        if let Some(half) = self.map_view {
            let hw = half * self.aspect.max(0.01);
            return Mat4::orthographic_rh(-hw, hw, -half, half, 1.0, 200.0);
        }
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
        {
            let fwd = (self.cam_look - self.cam_pos).normalize_or(Vec3::NEG_Z);
            let right = fwd.cross(Vec3::Y).normalize_or(Vec3::X);
            let up = right.cross(fwd);
            f.facing = Quat::from_mat3(&glam::Mat3::from_cols(right, up, -fwd));
        }
        f.skip_static = self.showcase || self.forge || self.select.is_some() || self.story.is_some();
        f.time = s.time as f32;
        if let Some(sv) = self.story {
            self.emit_story(sv, s, f);
            return;
        }
        if self.forge {
            self.emit_wares(s, f);
            return;
        }
        if let Some(sel) = self.select {
            self.emit_select(sel, s, f);
            return;
        }
        f.atmosphere = if self.showcase {
            Atmosphere::forest()
        } else if self.vault.is_some() {
            vault::vault_atmosphere()
        } else if self.hell {
            hell::hell_atmosphere()
        } else if self.meadow {
            meadow_atmosphere()
        } else {
            Atmosphere::forest()
        };
        if let Some(c) = self.tint.filter(|_| !self.showcase) {
            f.atmosphere = f.atmosphere.tinted(c);
        }
        // Seer's Sight (path 3) sees through the dark.
        let nv = if s.hero.sees() { 0.6 } else { 0.0 };
        if nv > 0.0 && !self.meadow && !self.hell && !self.showcase && self.vault.is_none() {
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
        // A shield potion's bark greys his skin a little.
        let tint = if hero.bark > 0 { Vec3::new(0.82, 0.84, 0.88) } else { Vec3::ONE };
        if hero.fx.ragnarok > 0.0 {
            emissive += lin(0x9a2aff) * (0.1 + (time * 5.0).sin().abs() * 0.1);
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
        if self.hell {
            hero_light *= 0.45; // the lava lights him from below
        }
        if self.vault.is_some() {
            hero_light *= 0.5; // candlelight everywhere
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
            // The trail grows with the blade's reach and takes its colour.
            // (The ring is drawn at 1.08 across; a blade's swing lands wherever the trail sweeps, see `in_blade_arc`.)
            let reach = crate::logic::progression::blade(s.hero_kind, hero.blade).swing.map(|s| s.0).unwrap_or(1.2);
            let size = (crate::logic::systems::sweep_radius(reach) / 1.08) as f32;
            let axe_color = if hero.blade <= 1 {
                lin(SLASH)
            } else {
                lin(crate::logic::progression::node(s.hero_kind, crate::logic::progression::Tree::Armory, hero.blade).color).lerp(Vec3::ONE, 0.35)
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

        // --- enemies (those lost in the fog are not drawn at all: the vaults hold dozens)
        for (i, v) in self.enemy_views.iter().enumerate() {
            if i >= s.enemies.len() {
                break;
            }
            if v.group_pos.distance(self.cam_pos) > f.atmosphere.fog_far + 4.0 {
                continue;
            }
            let group = Mat4::from_translation(v.group_pos) * Mat4::from_rotation_y(v.yaw);
            if v.visible {
                let root = group * euler(v.root_rot) * Mat4::from_scale(v.root_scale);
                let rig = self.models.foe_at(v.rig, v.group_pos.distance(self.cam_pos));
                emit_rig(&mut f.opaque[cube], rig, &v.pose, root, v.mat_color, 1.0, v.emissive);
            }
            if v.light_intensity > 0.0 {
                // Inside the body, so it lights the ground round the foe and not the foe; a draugr's is low,
                // under his open ribs.
                let lift = match v.kind {
                    EnemyKind::Skeleton | EnemyKind::Helmeted | EnemyKind::Minion => 0.2,
                    EnemyKind::Bat | EnemyKind::LargeBat => 0.13,
                    EnemyKind::Vampire => 0.16,
                    EnemyKind::Slime | EnemyKind::Werewolf => 0.3,
                    EnemyKind::Witch => 0.4,
                    EnemyKind::Blemmyes => 0.5,
                };
                f.lights.push(Light { pos: v.group_pos + Vec3::new(0.0, lift, 0.0), color: v.light_color * v.light_intensity, distance: 4.0 });
            }
            if s.enemies[i].is_ally() && v.visible {
                // A halo over the turned.
                let top = match v.kind {
                    EnemyKind::Skeleton => 1.18,
                    EnemyKind::Witch => 1.3,
                    EnemyKind::Helmeted | EnemyKind::Blemmyes => 1.05,
                    EnemyKind::Slime => 0.78,
                    EnemyKind::Werewolf | EnemyKind::Minion => 0.68,
                    _ => 0.32,
                };
                let blue = lin(crate::logic::angels::Archangel::Michael.color());
                let m = Mat4::from_translation(v.group_pos + Vec3::new(0.0, top, 0.0)) * Mat4::from_rotation_x(-PI / 2.0) * Mat4::from_scale(Vec3::new(0.2, 0.2, 1.0));
                f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, blue * 2.4, 0.85, Vec3::ZERO, SHADE_UNLIT));
            }
            if s.enemies[i].is_confused() && v.visible {
                for k in 0..3 {
                    let a = time * 5.0 + k as f32 * TAU / 3.0;
                    let p = v.group_pos + Vec3::new(a.cos() * 0.28, 1.0 + (time * 7.0 + k as f32).sin() * 0.04, a.sin() * 0.28);
                    f.additive.push(Instance::new(Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.05)), lin(0xc89aff) * 1.6, 0.9, Vec3::ZERO, SHADE_UNLIT));
                }
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
            if d.kind == DiamondKind::Angel {
                if let Some(lit) = self.emit_angel_shrine(d, grid, &s.angels_spent, f) {
                    shrine_points.push(lit);
                }
                continue;
            }
            let base = Vec3::new(d.tile.x as f32, floor_at(grid, d.tile.x as f32, d.tile.y as f32), d.tile.y as f32);
            let (stone, runes) = &self.models.pedestals[1];
            let rune = lin(0xffc84a);
            let rune_glow = if d.active { 0.6 + (time * 2.5 + d.tile.x as f32).sin().abs() * 0.6 } else { 0.1 };
            emit_mesh(&mut f.opaque[cube], stone, Mat4::from_translation(base), Vec3::ONE, 1.0, Vec3::ZERO);
            emit_mesh(&mut f.opaque[cube], runes, Mat4::from_translation(base), Vec3::ONE, 1.0, rune * rune_glow);
            if !d.active {
                continue;
            }
            let hex = d.current().color();
            let spin = self.diamond_spin.get(i).copied().unwrap_or(0.0);
            let m = Mat4::from_translation(base + Vec3::new(0.0, 0.45 + (time * 2.0 + d.tile.x as f32).sin() * 0.06, 0.0))
                * Mat4::from_rotation_y(spin);
            emit_mesh(&mut f.opaque[cube], &self.models.diamond, m, lin(hex), 1.0, lin(hex) * 0.3);
            shrine_points.push((base, lin(hex)));
        }

        // --- the hidden shrine: a pedestal with a rune stone turning over it, and with Seer's Sight a beam through the walls
        if let Some(t) = s.relic {
            let base = Vec3::new(t.x as f32, floor_at(grid, t.x as f32, t.y as f32), t.y as f32);
            let (stone, runes) = &self.models.pedestals[0];
            let rune = lin(0x9ad1ff);
            emit_mesh(&mut f.opaque[cube], stone, Mat4::from_translation(base), Vec3::ONE, 1.0, Vec3::ZERO);
            emit_mesh(&mut f.opaque[cube], runes, Mat4::from_translation(base), Vec3::ONE, 1.0, rune * (0.8 + (time * 3.0).sin().abs() * 0.8));
            let m = Mat4::from_translation(base + Vec3::new(0.0, 0.55 + (time * 2.0).sin() * 0.07, 0.0)) * Mat4::from_rotation_y(time * 1.4) * Mat4::from_scale(Vec3::new(0.2, 0.26, 0.1));
            f.opaque[cube].push(Instance::new(m, lin(0x8a8e96), 1.0, rune * 0.5, SHADE_LIT));
            glow_orb(f, base + Vec3::new(0.0, 0.6, 0.0), 0.7, rune, 0.35);
            f.lights.push(Light { pos: base + Vec3::new(0.0, 0.9, 0.0), color: rune * 5.0, distance: 3.5 });
            if s.hero.sees() {
                let beam = Mat4::from_translation(base + Vec3::new(0.0, 3.0, 0.0)) * Mat4::from_scale(Vec3::new(0.12, 6.0, 0.12));
                f.additive.push(Instance::new(beam, rune, 0.35 + 0.1 * (time * 4.0).sin(), Vec3::ZERO, SHADE_UNLIT));
            }
        }
        if let Some(d) = s.decoy {
            let at = Vec3::new(d.x as f32, floor_at(grid, d.x as f32, d.y as f32), d.y as f32);
            self.emit_decoy(at, time, f);
        }
        self.emit_spirits(s, time, f);
        self.emit_fires(s, time, f);
        self.emit_angel_fx(time, f);

        // --- potions
        for p in s.potions {
            if p.taken {
                continue;
            }
            let hex = p.def(s.hero_kind).color;
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
            let pos = Vec3::new(b.at.x as f32, floor_at(grid, b.tile.x as f32, b.tile.y as f32), b.at.y as f32);
            let tint = if on { Vec3::ONE } else { lin(0x9a9a9a) };
            let glow = if on { Vec3::splat(0.9) } else { Vec3::ZERO };
            emit_mesh(&mut f.opaque[cube], &self.models.bomb, Mat4::from_translation(pos), tint, 1.0, glow);
            // The fuse sputters: a few sparks jumping off its tip.
            let tip = pos + Vec3::new(0.07, 0.53, 0.0);
            for k in 0..4 {
                let ph = (s.time as f32 * 9.0 + k as f32 * 0.25).fract();
                let a = k as f32 * 1.9 + (s.time as f32 * 3.0).floor();
                let p = tip + Vec3::new(a.cos() * ph * 0.12, ph * 0.12, a.sin() * ph * 0.12);
                f.additive.push(Instance::new(Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.03)), lin(0xffc040) * 1.8, 1.0 - ph, Vec3::ZERO, SHADE_UNLIT));
            }
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

        // --- slime trails: glossy splats joined tile to tile, bubbling, drying away
        let trail: std::collections::HashSet<P> = s.puddles.iter().map(|(p, _)| p).collect();
        for (tile, life) in s.puddles.iter() {
            emit_goo(f, tile, floor_at(grid, tile.x as f32, tile.y as f32), life as f32 / PUDDLE_SECONDS as f32, time, &trail);
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
            f.lights.push(Light { pos: target + Vec3::new(0.4, 3.0, 0.6), color: lin(0xffe6b8) * 5.0 * k, distance: 6.0 });
        }

        // --- the boss and what it throws
        if let Some(b) = s.boss {
            self.emit_boss(b, hero_pos, time, f);
        }
        self.emit_boss_shots(s.boss_shots, hero_pos, time, f);

        let focus_hint = Vec3::new(hx, 0.0, hz);
        // --- fireflies by night; water, butterflies and pollen by day
        if self.meadow {
            self.emit_meadow(time, f);
        }
        if self.hell {
            // Lava below instead of the sea of cloud.
            f.bounds = Vec4::new(-0.5, -0.5, grid.width as f32 - 0.5, grid.height as f32 - 0.5);
            self.emit_hell(s, time, hero_pos, f);
        } else if self.vault.is_some() {
            // Underground: no sea of cloud, and no fireflies.
            f.bounds = Vec4::new(-0.5, -0.5, grid.width as f32 - 0.5, grid.height as f32 - 0.5);
            self.emit_vault(s, time, hero_pos, f);
        } else {
            emit_cloud_sea(grid, f, self.cam_pos);
        }
        self.emit_garden(time, f);
        self.emit_fountain(time, f);
        self.emit_gate_and_keys(s, time, hero_pos, f);
        self.emit_smoke(time, f);
        let mote_color = lin(MOTE);
        for (i, b) in self.mote_base.iter().enumerate() {
            if self.meadow || self.hell || self.vault.is_some() {
                break;
            }
            let fi = i as f32;
            let p = Vec3::new(
                b.x + (time * 0.5 + fi).sin() * 0.4,
                b.y + (time * 0.9 + fi * 0.7).sin() * 0.2,
                b.z + (time * 0.4 + fi * 1.3).cos() * 0.4,
            );
            // Each firefly flashes in its own rhythm: a quick glow, then a long dim spell.
            let phase = (time * (0.35 + (fi * 0.618).fract() * 0.3) + fi * 0.37).fract();
            let flash = (1.0 - (phase * 3.0 - 0.5).abs() * 2.0).max(0.0);
            glow_orb(f, p, 0.04 + 0.015 * flash, mote_color, 0.35 + 0.65 * flash * flash);
            if flash > 0.3 && p.distance_squared(focus_hint) < 144.0 {
                glow_orb(f, p, 0.16 + 0.1 * flash, mote_color, 0.28 * flash);
            }
        }

        // --- light pools, moved to the sources nearest the hero
        let focus = focus_hint;
        let torch_points: Vec<Vec3> = self.torches.values().map(|(p, _)| *p).collect();
        for (i, p) in nearest(&torch_points, focus, LIGHT_POOL_TORCHES).into_iter().enumerate() {
            let fi = i as f32;
            let intensity = 4.5 + (time * 3.0 + fi * 2.3).sin() * 0.5 + (time * 7.0 + fi).sin() * 0.3;
            f.lights.push(Light { pos: p + Vec3::new(0.0, 0.5, 0.2), color: lin(TORCH_LIGHT) * intensity, distance: 7.0 });
        }
        for (i, p) in nearest(&self.candles, focus, 5).into_iter().enumerate() {
            let flicker = 1.6 + (time * 9.0 + i as f32 * 1.7).sin() * 0.25 + (time * 23.0 + i as f32).sin() * 0.12;
            f.lights.push(Light { pos: p, color: lin(0xffb060) * flicker, distance: 3.2 });
        }
        // Halos round the lamps and candles nearby, as if lighting the damp air.
        for (i, p) in nearest(&torch_points, focus, 10).into_iter().enumerate() {
            let flick = 0.9 + (time * 3.0 + i as f32 * 2.3).sin() * 0.06;
            glow_orb(f, p + Vec3::new(0.0, 0.42, 0.0), 0.42, lin(TORCH_LIGHT), 0.16 * flick);
        }
        for (i, p) in nearest(&self.candles, focus, 14).into_iter().enumerate() {
            let flick = 0.85 + (time * 9.0 + i as f32 * 1.7).sin() * 0.1;
            glow_orb(f, p, 0.24, lin(0xffb060), 0.2 * flick);
        }
        for p in nearest(&remedy_points, focus, LIGHT_POOL_REMEDIES) {
            f.lights.push(Light { pos: p + Vec3::new(0.0, 0.45, 0.0), color: lin(REMEDY) * (4.0 + (time * 5.0).sin() * 1.5), distance: 5.0 });
        }
        let shrine_pos: Vec<Vec3> = shrine_points.iter().map(|(p, _)| *p).collect();
        for p in nearest(&shrine_pos, focus, LIGHT_POOL_SHRINES) {
            let color = shrine_points.iter().find(|(q, _)| *q == p).map(|(_, c)| *c).unwrap_or(Vec3::ONE);
            f.lights.push(Light { pos: p + Vec3::new(0.0, 0.9, 0.0), color: color * 4.0, distance: 5.0 });
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
     * The title screen's stage: the three friends still free stand in front,
     * and behind each on a stone dais towers the warrior whose soul he will
     * carry (Mehmed behind the left, Ragnar behind the middle, Miyamoto behind
     * the right), each in a glow of his soul's colour. A ring of trees,
     * braziers and fireflies round them, a warm key light, a cool rim.
     */
    fn emit_stage(&self, s: &SceneState, f: &mut Frame) {
        use crate::logic::heroes::HeroKind;
        let time = s.time as f32;
        let cube = MeshKind::Cube as usize;
        let cyl = MeshKind::Cylinder as usize;
        let at = |x: f32, y: f32, z: f32| STAGE + Vec3::new(x, y, z);

        // Ground: a wide mossy platform; behind the boys a dais of dressed stone, two steps up.
        let platform = Mat4::from_translation(at(0.0, -0.09, 0.0)) * Mat4::from_scale(Vec3::new(14.0, 1.5, 14.0));
        f.opaque[cyl].push(Instance::new(platform, lin(0x2a4a28), 1.0, Vec3::ZERO, SHADE_LIT));
        for (k, (w, d, y)) in [(4.9f32, 1.9f32, 0.07f32), (4.5, 1.5, 0.2)].into_iter().enumerate() {
            for gx in 0..8 {
                let x = -w / 2.0 + (gx as f32 + 0.5) * w / 8.0;
                let v = hash(gx as f64 + k as f64 * 9.0, 3.0) as f32;
                let m = Mat4::from_translation(at(x, y, -0.75 + k as f32 * 0.0)) * Mat4::from_scale(Vec3::new(w / 8.0 - 0.02, 0.13, d));
                f.opaque[cube].push(Instance::new(m, lin(0x6a6e64) * (0.8 + 0.3 * v), 1.0, Vec3::ZERO, SHADE_LIT));
            }
        }
        // A ring of trees, and two braziers either side of the line-up.
        for i in 0..14 {
            let a = i as f32 / 14.0 * TAU + 0.2;
            // Leave the camera's side of the ring open (the camera sits toward +z).
            if a.sin() > 0.35 {
                continue;
            }
            let r = 4.6 + (i % 2) as f32 * 0.9;
            push_field(&mut f.opaque[cube], &self.models.tree, at(a.cos() * r, 0.0, a.sin() * r), a * 3.0, 1.9 + (i % 3) as f32 * 0.3, Vec3::ONE, Vec3::ZERO, SHADE_LIT);
        }
        let braziers = [at(-2.9, 0.0, 0.2), at(2.9, 0.0, 0.2)];
        for p in braziers {
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
            glow_orb(f, p + Vec3::new(0.0, 1.1, 0.0), 0.6, lin(0xffa050), 0.25);
        }

        // The warriors on the dais, and in front of each the boy who will carry his soul.
        let heroes = [(HeroKind::Mehmed, &self.models.mehmed), (HeroKind::Ragnar, &self.models.ragnar), (HeroKind::Miyamoto, &self.models.miyamoto)];
        for (i, (_, rig)) in heroes.iter().enumerate() {
            let x = (i as f32 - 1.0) * 1.3;
            let tint = lin(story::SOUL_TINT[i]);
            let feet = at(x, 0.27, -0.75);
            let breathe = 1.0 + (time * 2.0 + i as f32 * 1.7).sin() * 0.006;
            let root = Mat4::from_translation(feet) * Mat4::from_rotation_y(-x * 0.12) * Mat4::from_scale(Vec3::new(1.12, 1.12 * breathe, 1.12));
            let mut pose = Pose::rest(rig);
            if let Some(hd) = rig.index("head") {
                pose.rot[hd].y = (time * 0.5 + i as f32 * 2.0).sin() * 0.12;
            }
            let aura = tint * (0.035 + 0.02 * (time * 1.5 + i as f32).sin());
            emit_rig(&mut f.opaque[cube], rig, &pose, root, Vec3::ONE, 1.0, aura);
            // His soul's light: a halo behind him, a ring at his feet, motes rising.
            glow_orb(f, feet + Vec3::new(0.0, 0.75, -0.35), 1.25, tint, 0.2 + 0.05 * (time * 1.3 + i as f32).sin());
            let ring = Mat4::from_translation(feet + Vec3::new(0.0, 0.01, 0.0)) * Mat4::from_rotation_x(-PI / 2.0) * Mat4::from_scale(Vec3::new(0.5, 0.5, 1.0));
            f.glow.push(Instance::new(ring, tint, 0.3, Vec3::ZERO, SHADE_GLOW));
            for k in 0..8 {
                let fk = k as f32;
                let u = (time * 0.4 + fk * 0.125 + i as f32 * 0.3).fract();
                let a = fk * 2.4 + time * 0.6;
                glow_orb(f, feet + Vec3::new(a.cos() * 0.4, u * 1.6, a.sin() * 0.3), 0.03, tint, 0.6 * (1.0 - u));
            }
            f.lights.push(Light { pos: feet + Vec3::new(0.0, 1.0, -0.6), color: tint * 4.0, distance: 3.0 });

            // The boy, a step in front, looking about; now and then one of them waves.
            let boy = at(x * 0.95, 0.0, 0.95);
            let cycle = (time * 0.25 + i as f32 * 0.37).fract();
            let act = if cycle < 0.18 { story::Act::Talk(cycle * 4.0 / 0.18) } else { story::Act::Stand };
            let look = (time * 0.6 + i as f32 * 2.1).sin() * 0.35;
            self.emit_kid(f, i + 1, story::kid_root(boy, -x * 0.1), act, time, look, Vec3::ZERO);
        }

        // Fireflies.
        for i in 0..50 {
            let fi = i as f32;
            let a = fi * 2.399;
            let r = 1.5 + (fi * 0.37) % 3.2;
            let p = at(a.cos() * r + (time * 0.5 + fi).sin() * 0.3, 0.5 + (fi * 0.61) % 2.0 + (time * 0.9 + fi).sin() * 0.15, a.sin() * r);
            let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.05));
            f.additive.push(Instance::new(m, lin(MOTE), 0.55 + (time * 2.2 + fi).sin() * 0.3, Vec3::ZERO, SHADE_UNLIT));
        }

        // Light: warm from the front for the boys' faces, a cool moon rim from behind, the braziers.
        let flicker = 1.0 + (time * 7.0).sin() * 0.05 + (time * 3.1).sin() * 0.04;
        f.lights.push(Light { pos: at(-1.2, 2.2, 3.0), color: lin(0xffc27a) * 22.0 * flicker, distance: 10.0 });
        f.lights.push(Light { pos: at(2.6, 2.8, -2.6), color: lin(0x8fb4ff) * 22.0, distance: 10.0 });
        f.lights.push(Light { pos: at(0.8, 2.0, 2.2), color: lin(0xfff0e0) * 5.0, distance: 6.0 });
        for p in braziers {
            f.lights.push(Light { pos: p + Vec3::new(0.0, 1.3, 0.0), color: lin(TORCH_LIGHT) * 4.0 * flicker, distance: 5.0 });
        }
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
    for (i, part) in rig.parts.iter().enumerate() {
        let pm = root * Mat4::from_translation(pose.pos[i]) * euler(pose.rot[i]);
        for (j, (off, c)) in part.cubes.iter().enumerate() {
            let shade = part.shade.get(j).copied().filter(|&m| m > 0.0).unwrap_or(SHADE_LIT);
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

/**
 * How far back the camera can sit before undergrowth comes between it and
 * Ragnar. Walls are whole tiles, so the ray is walked against the grid.
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

/// The dungeon floats on a sea of dark clouds: stacked mist layers under and around it. The
/// upper layers only show outside the level and swallow its sides; the lowest hides the void.
/// How far outside the island each rank of the smoke wall stands.
const SMOKE_RANKS: [f32; 5] = [0.35, 1.3, 2.6, 4.2, 6.5];
/// The smoke wall's sheets reach from down in the cloud sea to above its highest crest.
const SMOKE_BOTTOM: f32 = -2.6;
const SMOKE_TOP: f32 = 7.4;

/**
 * The cloud sea the island floats on, and the smoke wall round its edges:
 * ranks of billowing curtains that rise to about five times Ragnar's
 * height. A rank thins away as the camera comes up to it, so the view out
 * from near the edge stays clear, and from far outside (the map) the near
 * ranks are left out altogether.
 */
fn emit_cloud_sea(grid: &Grid, f: &mut Frame, eye: Vec3) {
    let tint = f.atmosphere.cloud_tint;
    let (w, h) = (grid.width as f32, grid.height as f32);
    f.bounds = Vec4::new(-0.5, -0.5, w - 0.5, h - 0.5);
    let center = Vec3::new(w / 2.0 - 0.5, 0.0, h / 2.0 - 0.5);
    let size = (w.max(h) + 70.0) * 2.0;
    // (height, opacity, seed), lowest first so they blend back to front.
    let layers = [(-1.6, 1.0, 0.0), (-1.2, 0.9, 1.0), (-0.8, 0.85, 2.0), (-0.45, 0.8, 3.0), (-0.1, 0.8, 4.0), (0.15, 0.75, 5.0), (0.4, 0.65, 6.0), (0.7, 0.5, 7.0), (1.0, 0.35, 8.0)];
    for (y, alpha, seed) in layers {
        let m = Mat4::from_translation(center + Vec3::new(0.0, y, 0.0)) * Mat4::from_scale(Vec3::new(size, 0.002, size));
        f.transparent[MeshKind::Cube as usize].push(Instance::new(m, tint, alpha, Vec3::new(seed, y, 0.0), SHADE_CLOUD));
    }

    // The smoke wall: north, south, west and east, each rank a curtain the length of the side and then some.
    let b = f.bounds;
    let mid_y = (SMOKE_BOTTOM + SMOKE_TOP) / 2.0;
    let tall = SMOKE_TOP - SMOKE_BOTTOM;
    let mut curtains: Vec<(f32, Instance)> = vec![];
    for (rank, &out) in SMOKE_RANKS.iter().enumerate() {
        for side in 0..4 {
            let along_x = side < 2;
            let len = if along_x { w } else { h } + 2.0 * out + 6.0;
            let (pos, eye_out) = match side {
                0 => (Vec3::new(center.x, mid_y, b.y - out), b.y - eye.z),
                1 => (Vec3::new(center.x, mid_y, b.w + out), eye.z - b.w),
                2 => (Vec3::new(b.x - out, mid_y, center.z), b.x - eye.x),
                _ => (Vec3::new(b.z + out, mid_y, center.z), eye.x - b.z),
            };
            let k = ((out - eye_out) / 1.5).clamp(0.0, 1.0);
            if k <= 0.0 {
                continue;
            }
            let scale = if along_x { Vec3::new(len, tall, 0.002) } else { Vec3::new(0.002, tall, len) };
            let m = Mat4::from_translation(pos) * Mat4::from_scale(scale);
            curtains.push((out - eye_out, Instance::new(m, tint, k, Vec3::new(30.0 + rank as f32, out, side as f32), SHADE_CLOUD)));
        }
    }
    // Farthest first, so the nearer ranks blend over the ones behind them.
    curtains.sort_by(|a, b| b.0.total_cmp(&a.0));
    f.transparent_early.extend(curtains.into_iter().map(|(_, i)| i));
}

/// Slime's wet material: glossy, a sharp highlight (voxel material 9).
const SHADE_GOO: f32 = 14.0;

/**
 * One tile of slime trail: a lumpy splat of glossy goo with droplets flung
 * around it, a smear joining it to the next trail tile, and bubbles that swell
 * and pop. `life` runs 1 (fresh) to 0 (dried up); the goo shrinks as it dries.
 */
fn emit_goo(f: &mut Frame, tile: P, floor: f32, life: f32, time: f32, trail: &std::collections::HashSet<P>) {
    let disc = MeshKind::Disc as usize;
    let k = (life / 0.4).min(1.0).sqrt();
    if k <= 0.02 {
        return;
    }
    let (tx, tz) = (tile.x as f32, tile.y as f32);
    let (hx, hy) = (tile.x as f64, tile.y as f64);
    let goo = lin(0x46d23c);
    let goo_l = lin(0x9cf06a);
    let glow = lin(0x5cff6a) * 0.14;
    let mut lift = 0.012;
    let mut blob = |f: &mut Frame, x: f32, z: f32, rx: f32, rz: f32, rot: f32, color: Vec3| {
        let m = Mat4::from_translation(Vec3::new(x, floor + lift, z)) * Mat4::from_rotation_y(rot) * Mat4::from_rotation_x(-PI / 2.0) * Mat4::from_scale(Vec3::new(rx, rz, 1.0));
        f.opaque[disc].push(Instance::new(m, color, 1.0, glow, SHADE_GOO));
        lift += 0.0015;
    };
    let wob = 1.0 + (time * 2.1 + tx * 1.3 + tz * 0.7).sin() * 0.05;
    let jx = (hash(hx * 3.0, hy * 5.0) as f32 - 0.5) * 0.12;
    let jz = (hash(hy * 3.0, hx * 5.0) as f32 - 0.5) * 0.12;
    let spin = hash(hx, hy) as f32 * TAU;
    // The main splat: two overlapping lobes.
    blob(f, tx + jx, tz + jz, 0.25 * k * wob, 0.19 * k / wob, spin, goo);
    blob(f, tx + jx + spin.cos() * 0.08, tz + jz + spin.sin() * 0.08, 0.15 * k / wob, 0.12 * k * wob, spin + 1.3, goo_l);
    // Droplets flung around it.
    for i in 0..5 {
        let hi = hash(hx * 7.0 + i as f64, hy * 11.0) as f32;
        let a = spin + i as f32 * 1.3 + hi;
        let d = 0.24 + hi * 0.14;
        let r = (0.035 + hi * 0.045) * k;
        blob(f, tx + jx + a.cos() * d, tz + jz + a.sin() * d, r, r * 0.85, a, if i % 2 == 0 { goo } else { goo_l });
    }
    // A smear toward the next tile of the trail, so it reads as one track.
    for (dx, dz) in [(1, 0), (0, 1)] {
        if trail.contains(&P::new(tile.x + dx, tile.y + dz)) {
            let (rx, rz) = if dx != 0 { (0.36, 0.1) } else { (0.1, 0.36) };
            blob(f, tx + dx as f32 * 0.5, tz + dz as f32 * 0.5, rx * k, rz * k, 0.0, goo);
        }
    }
    // Bubbles: swell, then pop.
    for b in 0..2 {
        let hb = hash(hx * 13.0 + b as f64, hy * 17.0) as f32;
        let phase = (time * (0.35 + hb * 0.3) + hb * 5.0).fract();
        let r = phase.powf(0.7) * 0.04 * k;
        if phase < 0.95 && r > 0.004 {
            let p = Vec3::new(tx + jx + (hb - 0.5) * 0.24, floor + 0.012 + r * 0.6, tz + jz + (0.5 - hb) * 0.2);
            let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(r));
            f.opaque[MeshKind::Sphere as usize].push(Instance::new(m, goo_l, 1.0, glow, SHADE_GOO));
        }
    }
}
