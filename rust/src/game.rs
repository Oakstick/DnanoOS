//! The game: screens, the round, and every rule that ties the entities together.

use std::collections::HashMap;
use std::f64::consts::PI;

mod angels;
mod controls;
mod map;
mod save;

pub use map::map_stops;
pub use save::Stage;
mod select;
mod shop;
mod story;
mod tour;
pub use shop::{RowKind, ShopAction, ShopButton, ShopRow, ShopState};
pub use story::{Story, STORY_FADE, TYPE_DELAY};
pub use tour::{Tour, TourArt, TourStop, TOUR_STOP_SECONDS};

use crate::audio::{AudioManager, Sfx};
use crate::input::{Action, Input};
use crate::logic::boss::{BossEvent, BossShot, Cyclops, ShotKind, BODY_RADIUS, CONTACT_RADIUS, CYCLOPS_HP, CYCLOPS_HP_STEP, POINTS_CYCLOPS};
use crate::logic::enemy::{default_enemy_specs, Enemy, EnemyContext, EnemyKind, EnemyMode, GlobalMode, HeroView};
use crate::logic::gear::{self, ShieldGear, BELT_SLOTS, COINS_BOSS, COINS_KEY, COINS_KILL, RUNES_BOSS, RUNES_SHRINE, RUNE_DROP};
use crate::logic::heroes::HeroKind;
use crate::logic::progression::{self, path, Progress, Tree};
use crate::logic::hero::{DamageResult, Hero, CONTACT_DAMAGE};
use crate::logic::items::{
    Bomb, Diamond, DiamondKind, ItemKind, Offer, Potion, BOMBS_PER_LIFE,
};
use crate::logic::level::{parse_level, ParsedLevel};
use crate::logic::levels::{boss_arena, level_for, ARENA};
use crate::logic::projectile::{spawn_shots, Projectile, ShotState};
use crate::logic::rng::Rng;
use crate::logic::systems::*;
use crate::logic::types::{dist_sq, manhattan, relative_dir, Dir, PotionKind, Tile, CARDINALS, P, V};
use crate::render::view::{dir_to_yaw, floor_at, CameraMode, SceneState, WorldView};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Title,
    /// Scene I, "The Threshold": after Play on the title, before the heroes are chosen.
    Story,
    /// Choosing a hero, after the story.
    Select,
    Ready,
    Playing,
    Paused,
    Dying,
    /// Every nugget taken: the fanfare plays, then the way to the boss opens.
    Cleared,
    Won,
    /// The old gatekeeper's wares: after a boss, or whenever he is called.
    Shop,
    /// The first-run tour of the dungeon, before the first countdown.
    Tour,
    GameOver,
}

/// Health a green cross gives back, capped at Ragnar's maximum.
const REMEDY_HEAL: i32 = 50;
/// Reinforcements: the first arrives after this long, then one every interval, up to the cap.
const SPAWN_FIRST: f64 = 20.0;
const SPAWN_EVERY: f64 = 25.0;
/// With every key taken the wait between reinforcements shrinks, down to this share at the last.
const SPAWN_EVERY_ENRAGED: f64 = 0.25;
const MAX_ENEMIES: usize = 12;
/// Every key raises the cap on foes by this many.
const MAX_ENEMIES_PER_KEY: usize = 2;
const SPAWN_CYCLE: [EnemyKind; 4] = [EnemyKind::Bat, EnemyKind::Slime, EnemyKind::Vampire, EnemyKind::Skeleton];
/// The music quickens one step for every this many keys.
pub const KEYS_PER_TEMPO_STEP: u32 = 2;
/// Seconds the gate's doors take to swing open.
const DOOR_SECONDS: f64 = 1.6;

/// The foes' schedule: they hunt the hero from the start and never break off to
/// wander to their corners (as the arcade ghosts did). Frozen while they are stunned.
const MODE_SCHEDULE: [(GlobalMode, f64); 1] = [(GlobalMode::Chase, f64::INFINITY)];

/// The countdown before play: 3, 2, 1, then GO.
pub const READY_SECONDS: f64 = 3.0;
/// How long "GO!" hangs on screen once play begins.
const GO_SECONDS: f32 = 0.8;
/// Freeze-frame on a kill, the classic "impact" beat.
const HITSTOP_KILL: f64 = 0.05;
/// Kills closer together than this count as a streak.
const STREAK_SECONDS: f64 = 2.5;
/// Every third kill in a streak is BRUTAL: a big shake and double points for a while.
pub const BRUTAL_STREAK: u32 = 3;
pub const DOUBLE_POINTS_SECONDS: f64 = 10.0;
/// Tiles walked between footsteps.
const STRIDE_TILES: f64 = 0.5;
/// A collected nugget takes this long to fly into the HUD counter.
pub const COIN_FLIGHT_SECONDS: f32 = 0.55;
const DYING_SECONDS: f64 = 1.8;
/// The fanfare and the fade into the boss arena.
pub const CLEARED_SECONDS: f64 = 3.4;
/// After the Cyclops bursts, a beat to take it in before the victory screen.
/// How close he must come to the Cyclops' red diamond key to take it, and its colour.
const RED_KEY_REACH: f64 = 0.7;
const RED_KEY_COLOR: u32 = 0xff2a44;
/// The shop cheat tops the purse up to this, so there is something to spend.
const CHEAT_COINS: u32 = 1000;
/// ...and the runes up to this.
const CHEAT_RUNES: u32 = 10;
/// Damage the Cyclops takes from a bomb (the weapons carry their own).
const BOMB_DAMAGE: i32 = 6;
/// Chain lightning (a tier-10 weapon): how far each arc leaps from where it struck, and what it deals the Cyclops.
const ZAP_RANGE: f64 = 4.5;
const ZAP_DAMAGE: i32 = 2;
/// Bleeding (a tier-8 weapon) and burning: seconds between their bites.
const BLEED_TICK: f64 = 0.5;
/// Venom (Miyamoto's serpent spear): the most stacks, how long they last after a hit, seconds between bites.
const POISON_MAX: i32 = 5;
const POISON_SECONDS: f64 = 6.0;
const POISON_TICK: f64 = 1.0;
/// Flames on the Cyclops (Kusanagi's fragment, the Simurg bow) and a greatsword's holy fire.
const FIRE_COLOR: u32 = 0xff8a2a;
const FIRE_SECONDS: f64 = 3.0;
/// Loki's trick: how long foes stay befuddled, how long a decoy lasts and how often one appears.
const CONFUSE_SECONDS: f64 = 4.0;
const DECOY_SECONDS: f64 = 5.0;
const DECOY_EVERY: f64 = 10.0;
/// From the second dungeon on the Cyclops' hide turns this much of every blow.
const BOSS_ARMOR: i32 = 1;
/// A shield's stagger (tier 7): how far and how long the foes around him reel.
const STAGGER_RADIUS: f64 = 2.5;
const STAGGER_SECONDS: f64 = 2.0;
/// Missiles a mirror-shield turns back strike the Cyclops this hard.
const REFLECT_DAMAGE: i32 = 2;
/// Dragonfire burns the Cyclops this hard when it touches him.
const DRAGONFIRE_DAMAGE: i32 = 3;
/// Freeze-frame on a blow to the boss.
const HITSTOP_BOSS: f64 = 0.035;
const COLLISION_RADIUS_SQ: f64 = 0.45 * 0.45;

/// An ancestral spirit fighting beside the hero: where it floats, and seconds until its next blow.
#[derive(Clone, Copy, Debug)]
pub struct Spirit {
    pub pos: V,
    pub cooldown: f64,
    /// 1 when it has just struck, fading: its swing.
    pub strike: f64,
}

pub struct Floater {
    pub x: f64,
    pub y: f64,
    pub text: String,
    pub life: f64,
    pub max_life: f64,
    pub color: Option<u32>,
}

/// A nugget on its way from the world to the HUD counter, in screen space.
pub struct FlyingCoin {
    pub from: (f32, f32),
    pub t: f32,
}

/// A big centered announcement ("10 NUGGETS").
pub struct Banner {
    pub text: String,
    pub sub: String,
    pub color: u32,
    pub life: f32,
    pub max: f32,
}

/// Presentation state for rewards and feedback; none of it affects the rules.
#[derive(Default)]
pub struct Juice {
    /// The score as displayed, rolling up toward the real one.
    pub score_shown: f64,
    /// 1 when a nugget lands in the counter, fading to 0.
    pub key_punch: f32,
    /// The health bar's trailing chunk after a hit, in HP.
    pub hp_trail: f32,
    /// Flash on the health bar: color and 1..0 strength.
    pub hp_flash: f32,
    pub hp_flash_color: u32,
    /// Red screen-edge flash when hurt.
    pub hurt: f32,
    pub coins: Vec<FlyingCoin>,
    pub banner: Option<Banner>,
    /// Seconds "GO!" has left on screen.
    pub go: f32,
    /// Black over everything, 1..0, fading in after a level change.
    pub fade: f32,
    /// The boss health bar's trailing chunk, in HP.
    pub boss_trail: f32,
    /// 1 when coins come in, fading: the purse counter jumps.
    pub coin_punch: f32,
    /// 1 on a BRUTAL streak, fading: a hot flash over the screen.
    pub brutal: f32,
}

/// A button on the title or pause menu.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuItem {
    /// A new adventure (on the title; with something saved, it asks first).
    Play,
    /// Back into the saved adventure.
    Continue,
    /// The answers to "start over?".
    StartOver,
    KeepProgress,
    /// The controller page: what every button does.
    Controls,
    /// The map: jump to any dungeon or Cyclops (for testing, for now).
    Map,
    Resume,
    /// The gatekeeper's promise: call him, and his wares open wherever you are.
    CallOldMan,
    QuitToTitle,
    /// Leave the game entirely. Desktop only: a browser tab cannot close itself.
    Quit,
}

impl MenuItem {
    pub fn label(self) -> &'static str {
        match self {
            MenuItem::Play => "START",
            MenuItem::Continue => "CONTINUE",
            MenuItem::StartOver => "START OVER",
            MenuItem::KeepProgress => "KEEP MY PROGRESS",
            MenuItem::Controls => "CONTROLLER",
            MenuItem::Map => "MAP",
            MenuItem::Resume => "RESUME",
            MenuItem::CallOldMan => "CALL THE OLD MAN",
            MenuItem::QuitToTitle => "QUIT TO TITLE",
            MenuItem::Quit => "QUIT GAME",
        }
    }
}

/// Things the platform layer may want to hear about (Steam achievements).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameEvent {
    /// Every nugget in a dungeon.
    Won,
    FirstKey,
    BossSlain,
}

pub struct Game {
    pub screen: Screen,
    pub level: ParsedLevel,
    pub hero: Hero,
    pub enemies: Vec<Enemy>,
    pub bombs: Vec<Bomb>,
    pub bombs_left: u32,
    pub projectiles: Vec<Projectile>,
    pub diamonds: Vec<Diamond>,
    pub potions: Vec<Potion>,
    pub keys_total: u32,
    pub keys_got: u32,
    /// The gate to the boss, and whether every key has been found.
    pub gate: Option<P>,
    pub gate_open: bool,
    /// How far its doors have swung (0 shut, 1 open); they start when Ragnar comes to them.
    pub door: f64,
    door_opening: bool,
    gate_hint: f64,
    pub puddles: PuddleField,
    /// Dungeon I's garden plan (its river, bridges and scenery), when this is Dungeon I.
    pub garden: Option<crate::logic::garden::GardenPlan>,
    trail_tile: HashMap<usize, P>,
    spawn_timer: f64,
    spawned: usize,
    pub frenzy: Frenzy,
    pub score: Score,
    pub audio: AudioManager,
    pub input: Input,
    pub view: WorldView,
    pub show_fps: bool,
    rng: Rng,
    /// The corridor the chase camera currently points down.
    cam_dir: Dir,
    /// A held key and the corridor it was resolved to when pressed (camera-follow mode).
    latch: Option<(Dir, Dir)>,
    global_mode: GlobalMode,
    mode_index: usize,
    mode_timer: f64,
    /// Seconds left on a timed screen (the countdown reads it).
    pub screen_timer: f64,
    pub time: f64,
    pub floaters: Vec<Floater>,
    nest_slots: Vec<P>,
    pub events: Vec<GameEvent>,
    /// Set when the player asks for fullscreen; the app layer applies it.
    pub want_fullscreen_toggle: bool,
    /// The highlighted menu button.
    pub menu_index: usize,
    /// The player chose Quit Game; the app layer closes the window.
    pub quit_requested: bool,
    /// Logical window size, for hit-testing menu buttons.
    pub viewport: (f32, f32),
    pub juice: Juice,
    /// Seconds of freeze-frame left.
    hitstop: f64,
    kill_streak: u32,
    streak_timer: f64,
    step_dist: f64,
    was_airborne: bool,
    /// The way Ragnar faces at spawn: down the longest corridor from the center.
    start_facing: Dir,
    /// The dungeon number this stretch of the game belongs to (its boss shares it).
    pub dungeon: usize,
    /// The boss of a boss arena, and what it has thrown.
    pub boss: Option<Cyclops>,
    pub boss_shots: Vec<BossShot>,
    boss_events: Vec<BossEvent>,
    /// Counts down to the victory screen once the boss is dead.
    /// Where the Cyclops' red diamond key lies, until he takes it.
    pub red_key: Option<P>,
    /// Silver, rune fragments and every hero's unlocked trees: kept between adventures.
    pub progress: Progress,
    /// Potions carried on his belt, and the one Q (or RB) drinks.
    pub belt: Vec<PotionKind>,
    pub belt_index: usize,
    pub shop: ShopState,
    /// Seconds the boss has left bleeding, and until its next bite.
    boss_bleed: f64,
    bleed_tick: f64,
    /// A decoy of Loki's trick the foes chase (where, seconds left), and seconds until the next.
    pub decoy: Option<(V, f64)>,
    decoy_cooldown: f64,
    /// Once per dungeon: the Valkyrie's revival, and the World Tree's restoration, spent.
    revived: bool,
    restored: bool,
    /// This dungeon's hidden shrine and its relic, until taken.
    pub relic_shrine: Option<P>,
    /// Colour of what the boss is losing blood to (a bleeding cut, or flames).
    bleed_color: u32,
    /// Venom stacked on the Cyclops (a serpent spear): stacks, seconds left, until its next bite.
    poison: (i32, f64, f64),
    /// Holy fire left burning on tiles (a greatsword's trail), seconds left each.
    pub fires: Vec<(P, f64)>,
    /// The spirits fighting at the hero's side (path 9 of Mehmed and Miyamoto).
    pub spirits: Vec<Spirit>,
    /// Once per dungeon: the last stand, spent.
    stood: bool,
    /// Seconds of double points left (from a BRUTAL streak).
    pub double_points: f64,
    /// The first-run tour, while it plays, and whether it is still owed.
    pub tour: Option<Tour>,
    pub tour_pending: bool,
    /// The hero being played (chosen on the character screen).
    pub hero_kind: crate::logic::heroes::HeroKind,
    /// The character screen: which place in the line-up is highlighted, seconds since it
    /// moved, and seconds left of the shake after a locked hero was picked.
    pub select_index: usize,
    pub select_age: f64,
    pub select_denied: f64,
    /// Scene I while it plays, and whether Play on the title shows it (tests go straight to the heroes).
    pub story: Option<Story>,
    pub intro: bool,
    /// The adventure saved at the start of the last stage reached, for Continue.
    pub saved_run: Option<save::Run>,
    /// The title is asking whether to wipe everything and start over.
    pub confirm_reset: bool,
    /// The controller page is open over the title or the pause menu.
    pub show_controls: bool,
    /// The map page is open over the title or the pause menu; which stop is highlighted.
    pub show_map: bool,
    pub map_index: usize,
    /// The archangel's blessing he carries (from dungeon to dungeon), if any.
    pub blessing: Option<crate::logic::angels::Blessing>,
    /// Seconds of play until Uriel grants his blessing again; when the last refusal was shown.
    pub uriel_rest: f64,
    refused_at: f64,
    /// Uriel's wave while it rolls out.
    pub holy_wave: Option<angels::HolyWave>,
    /// Which archangels have given their blessing in this level (each gives once).
    pub angels_used: [bool; 4],
}

impl Game {
    pub fn new() -> Self {
        let level = parse_level(&level_for(0));
        let hero = Hero::new(level.hero_spawn, level.data.hero_speed);
        let mut g = Self {
            screen: Screen::Title,
            level,
            hero,
            enemies: Vec::new(),
            bombs: Vec::new(),
            bombs_left: BOMBS_PER_LIFE,
            projectiles: Vec::new(),
            diamonds: Vec::new(),
            potions: Vec::new(),
            keys_total: 0,
            keys_got: 0,
            gate: None,
            gate_open: false,
            door: 0.0,
            door_opening: false,
            gate_hint: 0.0,
            puddles: PuddleField::default(),
            garden: None,
            trail_tile: HashMap::new(),
            spawn_timer: SPAWN_FIRST,
            spawned: 0,
            frenzy: Frenzy::default(),
            score: Score::new(),
            audio: AudioManager::new(),
            input: Input::new(),
            view: WorldView::new(),
            show_fps: false,
            rng: Rng::from_time(),
            cam_dir: Dir::None,
            latch: None,
            global_mode: GlobalMode::Chase,
            mode_index: 0,
            mode_timer: MODE_SCHEDULE[0].1,
            screen_timer: 0.0,
            time: 0.0,
            floaters: Vec::new(),
            nest_slots: Vec::new(),
            events: Vec::new(),
            want_fullscreen_toggle: false,
            menu_index: 0,
            quit_requested: false,
            viewport: (1280.0, 720.0),
            juice: Juice::default(),
            hitstop: 0.0,
            kill_streak: 0,
            streak_timer: 0.0,
            step_dist: 0.0,
            was_airborne: false,
            start_facing: Dir::Down,
            dungeon: 0,
            boss: None,
            boss_shots: Vec::new(),
            boss_events: Vec::new(),
            red_key: None,
            // Tests start from nothing; the game from the saved progress.
            progress: if cfg!(test) { Progress::default() } else { crate::platform::load("vh.progress").and_then(|s| Progress::decode(&s)).unwrap_or_default() },
            belt: Vec::new(),
            belt_index: 0,
            shop: ShopState::default(),
            boss_bleed: 0.0,
            bleed_tick: 0.0,
            decoy: None,
            decoy_cooldown: 0.0,
            revived: false,
            restored: false,
            relic_shrine: None,
            bleed_color: 0xc8202a,
            poison: (0, 0.0, 0.0),
            fires: Vec::new(),
            spirits: Vec::new(),
            stood: false,
            double_points: 0.0,
            tour: None,
            // Tests start straight into play; they switch the tour on when they want it.
            tour_pending: !cfg!(test) && !tour::tour_seen(),
            hero_kind: crate::logic::heroes::HeroKind::Ragnar,
            select_index: 1,
            select_age: 0.0,
            select_denied: 0.0,
            story: None,
            intro: !cfg!(test),
            saved_run: save::load_run(),
            confirm_reset: false,
            show_controls: false,
            show_map: false,
            map_index: 0,
            blessing: None,
            uriel_rest: 0.0,
            refused_at: -10.0,
            holy_wave: None,
            angels_used: [false; 4],
        };
        g.load_level(0);
        g.audio.set_soundtrack(Some(crate::audio::Soundtrack::Menu));
        // Free movement is on unless the player turned it off (tests keep to the grid).
        g.hero.free_move = !cfg!(test) && crate::platform::load("vh.free_move").map(|v| v != "0").unwrap_or(true);
        g
    }

    // ---------------------------------------------------------------- setup

    /// Dungeon `index`: the labyrinth with its 25 nuggets.
    pub fn load_level(&mut self, index: usize) {
        self.load(level_for(index), index);
    }

    /// The Cyclops' arena that follows dungeon `index`.
    pub fn load_boss(&mut self, index: usize) {
        self.load(boss_arena(index), index);
    }

    fn load(&mut self, data: crate::logic::level::LevelData, index: usize) {
        self.score.level = index;
        self.dungeon = index;
        self.level = parse_level(&data);
        if self.level.data.boss {
            // In the arena he starts at the south edge, facing the Cyclops.
            self.start_facing = Dir::Up;
        } else {
            // Ragnar starts in the heart of the dungeon, facing its longest corridor.
            let (spawn, facing) = center_spawn(&self.level);
            self.level.hero_spawn = spawn;
            self.start_facing = facing;
        }
        // Dungeon I is a landscape garden: a fountain and its bridged channels, a churchyard, temples and flower beds.
        self.garden = None;
        if self.level.data.boss {
            // The arena's stream runs in a stone channel under stone bridges too.
            self.garden = crate::logic::garden::stream_plan(&self.level.grid);
            if let Some(plan) = &self.garden {
                for b in &plan.bridges {
                    self.level.grid.set_bridge(b.x, b.y, true);
                }
            }
        } else if index == 0 {
            let l = &self.level;
            let mut keep = vec![l.hero_spawn, l.enemy_spawn];
            keep.extend(l.angel_shrines.iter().chain(&l.item_shrines).copied());
            keep.extend(l.potions.iter().map(|(p, _)| *p));
            keep.extend(l.gate);
            self.garden = crate::logic::garden::plan(&l.grid, &keep);
            if let Some(plan) = &self.garden {
                crate::logic::garden::apply(&mut self.level.grid, plan);
            }
        }
        // The estate's gate takes its golden keys; the arena's, the one red key the Cyclops drops.
        self.keys_total = if self.level.data.boss { 1 } else { self.level.grid.count(Tile::Nodule) as u32 };
        self.keys_got = 0;
        self.gate = self.level.gate;
        self.gate_open = false;
        self.door = 0.0;
        self.door_opening = false;
        self.gate_hint = 0.0;
        self.audio.set_tempo_step(0);

        // Those waiting in the hollow stand along its middle row: at either end and in the middle.
        let c = self.level.nest_center;
        let g = &self.level.grid;
        let row: Vec<P> = (0..g.width).map(|x| P::new(x, c.y)).filter(|p| g.get(p.x, p.y) == Tile::Nest && (p.x - c.x).abs() <= 3).collect();
        let (first, last) = (row.first().copied().unwrap_or(c), row.last().copied().unwrap_or(c));
        self.nest_slots = vec![self.level.enemy_spawn, first, c, last];

        // Clearing a dungeon keeps the held weapon and its ammo; only death takes them.
        let carried = if self.hero.alive && self.hero.holding_pickup() { Some((self.hero.weapon, self.hero.ammo)) } else { None };
        // His shield, dents and all, goes with him, and so does the way he moves (free or on the grid).
        let (shield, free_move) = (self.hero.shield, self.hero.free_move);
        // Each hero runs at his own pace: Miyamoto quicker than Ragnar, Mehmed slower.
        self.hero = Hero::new(self.level.hero_spawn, self.level.data.hero_speed * self.hero_kind.speed());
        self.hero.shield = shield;
        self.hero.free_move = free_move;
        self.equip_from_progress();
        let specs = if self.level.data.boss { Vec::new() } else { default_enemy_specs(&self.level.grid) };
        self.enemies = specs
            .iter()
            .enumerate()
            .map(|(i, s)| Enemy::new(s, self.nest_slots.get(i).copied().unwrap_or(c), self.level.data.enemy_speed))
            .collect();
        self.boss = if self.level.data.boss {
            let (x0, y0, x1, y1) = ARENA;
            let hp = CYCLOPS_HP + CYCLOPS_HP_STEP * index as i32;
            let min = V::new(x0 as f64 + 1.0, y0 as f64 + 0.5);
            let max = V::new(x1 as f64 - 1.0, y1 as f64 - 0.5);
            Some(Cyclops::new(self.level.enemy_spawn.v(), min, max, hp, self.rng.index(1 << 30) as u32))
        } else {
            None
        };
        self.boss_shots.clear();
        self.red_key = None;
        self.angels_used = [false; 4];
        self.diamonds = self
            .level
            .angel_shrines
            .iter()
            .enumerate()
            .map(|(i, &t)| Diamond::new(DiamondKind::Angel, t, i))
            .chain(self.level.item_shrines.iter().enumerate().map(|(i, &t)| Diamond::new(DiamondKind::Item, t, i)))
            .collect();
        self.potions = self.level.potions.iter().map(|&(t, k)| Potion::new(t, k)).collect();
        if self.level.data.boss {
            // No bottles in a boss arena: bring your own.
            self.potions.clear();
        }
        if index >= 1 {
            // From the second dungeon on, gear only comes from the old gatekeeper;
            // the archangels' shrines stand in every dungeon.
            self.potions.clear();
            self.diamonds.retain(|d| d.kind == DiamondKind::Angel);
        }
        self.boss_bleed = 0.0;
        self.decoy = None;
        self.decoy_cooldown = 0.0;
        if !self.level.data.boss {
            // The Valkyrie and the World Tree answer once in each dungeon (its Cyclops included).
            self.revived = false;
            self.restored = false;
            self.stood = false;
        }
        self.poison = (0, 0.0, 0.0);
        self.fires.clear();
        self.spirits.clear();
        self.relic_shrine = if self.level.data.boss { None } else { self.hidden_shrine_spot() };
        self.spawn_timer = SPAWN_FIRST;
        self.spawned = 0;
        if let Some(g) = self.level.gate {
            for dx in -1..=1 {
                self.level.grid.set_landmark(g.x + dx, g.y, true);
            }
        }
        self.view.build_level(&self.level.grid, &self.enemies, &self.diamonds, self.level.data.boss, self.garden.as_ref(), self.level.gate);
        self.juice.boss_trail = self.boss.as_ref().map(|b| b.hp as f32).unwrap_or(0.0);
        self.reset_round();
        if let Some((crate::logic::items::WeaponKind::Ranged(w), ammo)) = carried.filter(|_| index == 0) {
            self.hero.equip(w);
            self.hero.ammo = ammo;
        }
    }

    /// Puts everyone back at spawn without touching collected gold.
    fn reset_round(&mut self) {
        self.hero.respawn(self.level.hero_spawn);
        // On his tile's own height from the first frame: the start can be raised (the fountain's terrace),
        // and until he moves nothing else lifts him out of the stone.
        let spawn = self.level.hero_spawn;
        self.hero.altitude = self.level.grid.elevation(spawn.x, spawn.y) as f64;
        self.hero.face(self.start_facing);
        self.view.aim_along(self.hero.facing);
        self.juice.coins.clear();
        self.juice.banner = None;
        self.juice.hp_trail = self.hero.hp as f32;
        self.hitstop = 0.0;
        self.kill_streak = 0;
        self.double_points = 0.0;
        self.was_airborne = false;
        let center = self.level.nest_center;
        let graves = self.grave_spawns();
        for (i, e) in self.enemies.iter_mut().enumerate() {
            match graves.get(i % graves.len().max(1)) {
                // Draugr climb out of the churchyard rather than the hollow.
                Some(&g) if e.kind() == EnemyKind::Skeleton => e.rise_at(g, crate::logic::enemy::REASSEMBLE_SECONDS),
                _ => e.reset(self.nest_slots.get(i).copied().unwrap_or(center)),
            }
        }
        self.mode_index = 0;
        self.mode_timer = MODE_SCHEDULE[0].1;
        self.global_mode = MODE_SCHEDULE[0].0;
        self.frenzy.cancel();
        self.audio.set_frenzy(false);
        self.floaters.clear();
        self.puddles.clear();
        self.trail_tile.clear();
        self.input.clear_direction();
        self.cam_dir = Dir::None;
        self.bombs.clear();
        self.bombs_left = BOMBS_PER_LIFE;
        self.projectiles.clear();
        // The Cyclops flies back up; it keeps the wounds it has taken.
        let home = self.level.enemy_spawn.v();
        if let Some(b) = &mut self.boss {
            b.reset(home);
        }
        self.boss_shots.clear();
    }

    fn start_game(&mut self) {
        self.score.reset();
        self.blessing = None;
        self.uriel_rest = 0.0;
        self.holy_wave = None;
        self.new_kit();
        self.hero.respawn(self.level.hero_spawn); // a new game starts with the bare axe
        self.load_level(0);
        self.juice.score_shown = 0.0;
        self.checkpoint(save::Stage::Dungeon(0));
        if self.tour_pending {
            self.begin_tour();
        } else {
            self.enter(Screen::Ready);
        }
    }

    /// A fresh adventure: the weapon and shield he chose at the old man's, one health potion.
    /// Silver, runes and what is unlocked are his from adventure to adventure.
    fn new_kit(&mut self) {
        self.belt = vec![PotionKind::Health];
        self.belt_index = 0;
        let carry = self.progress.hero(self.hero_kind).carry;
        self.hero.shield = Some(ShieldGear::new(carry));
        self.equip_from_progress();
    }

    /// Puts the chosen weapon in his hand and his path's skills in his heart.
    pub fn equip_from_progress(&mut self) {
        let h = *self.progress.hero(self.hero_kind);
        self.hero.kind = self.hero_kind;
        self.hero.arm(h.wield);
        self.hero.path = h.path();
        self.hero.max_hp = self.hero.health_cap();
    }

    /// Writes the silver, the runes and the unlocks to the save (not in tests).
    fn save_progress(&self) {
        if !cfg!(test) {
            crate::platform::save("vh.progress", &self.progress.encode());
        }
    }

    /**
     * Where this dungeon hides its shrine: the dead end farthest (by walking)
     * from where he starts, clear of keys, shrines, bottles and the gate.
     */
    fn hidden_shrine_spot(&self) -> Option<P> {
        let g = &self.level.grid;
        let start = self.level.hero_spawn;
        let mut dist: HashMap<P, u32> = HashMap::new();
        let mut queue = std::collections::VecDeque::new();
        dist.insert(start, 0);
        queue.push_back(start);
        while let Some(t) = queue.pop_front() {
            let d = dist[&t];
            for dir in CARDINALS {
                if !g.can_move(t, dir, false, false) {
                    continue;
                }
                let n = g.neighbor(t, dir);
                if let std::collections::hash_map::Entry::Vacant(e) = dist.entry(n) {
                    e.insert(d + 1);
                    queue.push_back(n);
                }
            }
        }
        let taken: Vec<P> = self
            .level
            .angel_shrines
            .iter()
            .chain(&self.level.item_shrines)
            .copied()
            .chain(self.level.potions.iter().map(|(p, _)| *p))
            .chain(self.level.gate)
            .chain([self.level.enemy_spawn, self.level.nest_center])
            .collect();
        let mut best: Option<(u32, P)> = None;
        for (&t, &d) in &dist {
            if g.get(t.x, t.y) != Tile::Open || g.is_bridge(t.x, t.y) || taken.iter().any(|q| manhattan(q.v(), t.v()) < 2.0) {
                continue;
            }
            let exits = CARDINALS.iter().filter(|&&dir| g.can_move(t, dir, false, false)).count();
            // Dead ends first; ties go to the far one, then the lower-left, so it is always the same spot.
            let score = d + if exits == 1 { 1000 } else { 0 };
            if best.map(|(s, b)| score > s || (score == s && (t.y, t.x) > (b.y, b.x))).unwrap_or(true) {
                best = Some((score, t));
            }
        }
        best.map(|(_, t)| t)
    }

    fn enter(&mut self, screen: Screen) {
        self.screen = screen;
        self.menu_index = 0;
        self.confirm_reset = false;
        self.show_controls = false;
        self.show_map = false;
        // The backing track steps back when the game does, and under the fanfare.
        self.audio.duck_music(matches!(screen, Screen::Paused | Screen::Dying | Screen::GameOver | Screen::Won | Screen::Cleared));
        if screen == Screen::Title {
            self.audio.set_tempo_step(0);
        }
        // Dungeon I has its own recorded soundtrack; the title, the shop and the rest keep the synth tune.
        use crate::audio::Soundtrack;
        match screen {
            Screen::Title | Screen::Select | Screen::Shop => self.audio.set_soundtrack(Some(Soundtrack::Menu)),
            Screen::Ready | Screen::Tour => {
                let track = if self.level.data.boss {
                    Some(Soundtrack::Boss)
                } else if self.dungeon == 0 {
                    Some(Soundtrack::Level1)
                } else {
                    None
                };
                self.audio.set_soundtrack(track);
            }
            _ => {}
        }
        if screen == Screen::Ready {
            self.audio.play(Sfx::Tick); // "3"
        }
        self.screen_timer = match screen {
            Screen::Ready => READY_SECONDS,
            Screen::Dying => DYING_SECONDS,
            Screen::Cleared => CLEARED_SECONDS,
            _ => 0.0,
        };
    }

    // --------------------------------------------------------------- update

    pub fn update(&mut self, dt: f64) {
        self.time += dt;
        self.update_juice(dt);
        self.view.showcase = self.screen == Screen::Title;
        self.view.select = (self.screen == Screen::Select).then(|| crate::render::view::SelectView {
            index: self.select_index,
            age: self.select_age as f32,
            denied: self.select_denied as f32,
        });
        self.view.story = self.story.as_ref().filter(|_| self.screen == Screen::Story).map(|s| crate::render::view::StoryView {
            beat: s.beat,
            t: s.t as f32,
            age: s.age as f32,
            shown: s.shown as f32,
            souls: s.souls,
        });
        self.view.forge = self.screen == Screen::Shop;
        self.view.tour = self.tour.as_ref().filter(|_| self.screen == Screen::Tour).map(|t| (t.stop().target, t.t as f32, t.index));
        self.view.focus = self
            .boss
            .as_ref()
            .filter(|b| b.alive() && matches!(self.screen, Screen::Playing | Screen::Paused))
            .filter(|b| !matches!(b.state, crate::logic::boss::BossState::Hidden))
            .map(|b| glam::Vec3::new(b.pos.x as f32, 0.0, b.pos.y as f32));
        self.input.poll(dt);
        self.handle_global_keys();

        match self.screen {
            Screen::Title | Screen::Paused if self.show_controls => self.update_controls(),
            Screen::Title | Screen::Paused if self.show_map => self.update_map(),
            Screen::Title => match self.menu_choice() {
                Some(MenuItem::Quit) => self.quit_requested = true,
                Some(MenuItem::Controls) => self.open_controls(),
                Some(MenuItem::Map) => self.open_map(),
                Some(MenuItem::Continue) => self.continue_game(),
                Some(MenuItem::StartOver) => self.answer_reset(true),
                Some(MenuItem::KeepProgress) => self.answer_reset(false),
                // Start begins a new adventure: the story, then the character screen.
                Some(_) => self.start_from_title(),
                None if self.confirm_reset && self.input.was_pressed(&[Action::Pause]) => self.answer_reset(false),
                // With nothing saved, a tap anywhere on the title starts too.
                None if !self.can_continue() && self.input.was_pressed(&[Action::Tap]) => self.play_from_title(),
                None => {}
            },
            Screen::Story => self.update_story(dt),
            Screen::Select => self.update_select(dt),
            Screen::Ready => {
                let before = self.screen_timer.ceil();
                self.screen_timer -= dt;
                if self.screen_timer <= 0.0 {
                    self.enter(Screen::Playing);
                    self.audio.play(Sfx::Go);
                    self.juice.go = GO_SECONDS;
                    if let Some(b) = &mut self.boss {
                        b.wake();
                    }
                } else if self.screen_timer.ceil() < before {
                    self.audio.play(Sfx::Tick); // "2", "1"
                }
            }
            Screen::Playing => {
                // A freeze-frame on a hit: the world holds for a beat, effects keep moving.
                if self.hitstop > 0.0 {
                    self.hitstop -= dt;
                } else {
                    self.tick(dt);
                }
            }
            Screen::Paused => match self.menu_choice() {
                Some(MenuItem::Resume) => self.enter(Screen::Playing),
                Some(MenuItem::CallOldMan) => self.call_old_man(),
                Some(MenuItem::Controls) => self.open_controls(),
                Some(MenuItem::Map) => self.open_map(),
                Some(MenuItem::QuitToTitle) => self.enter(Screen::Title),
                Some(MenuItem::Quit) => self.quit_requested = true,
                _ => {}
            },
            Screen::Dying => {
                self.screen_timer -= dt;
                if self.screen_timer <= 0.0 {
                    if self.score.lose_life() {
                        self.enter(Screen::GameOver);
                    } else {
                        self.reset_round();
                        self.enter(Screen::Ready);
                    }
                }
            }
            Screen::Cleared => {
                // The world keeps moving (sparkles, the camera) while the fanfare plays.
                self.screen_timer -= dt;
                self.update_effects(dt);
                if self.screen_timer <= 0.0 {
                    let next = self.dungeon;
                    self.load_boss(next);
                    self.checkpoint(save::Stage::Arena(next));
                    self.juice.fade = 1.0;
                    self.enter(Screen::Ready);
                }
            }
            Screen::Won => {
                // The boss is down: off to the old gatekeeper.
                if self.input.was_pressed(&[Action::Confirm, Action::Jump, Action::Tap]) {
                    self.open_shop();
                    self.juice.fade = 1.0;
                    self.input.consume(Action::Attack);
                    self.input.consume(Action::Confirm);
                }
            }
            Screen::Shop => self.update_shop(dt),
            Screen::Tour => self.update_tour(dt),
            Screen::GameOver => {
                if self.input.was_pressed(&[Action::Confirm, Action::Jump, Action::Tap]) {
                    self.enter(Screen::Title);
                }
            }
        }
        let state = SceneState {
            grid: &self.level.grid,
            hero: &self.hero,
            enemies: &self.enemies,
            frenzy: &self.frenzy,
            time: self.time,
            bombs: &self.bombs,
            projectiles: &self.projectiles,
            diamonds: &self.diamonds,
            potions: &self.potions,
            puddles: &self.puddles,
            boss: self.boss.as_ref(),
            boss_shots: &self.boss_shots,
            gate: self.gate.map(|tile| crate::render::view::GateView { tile, keys: self.keys_got, total: self.keys_total, door: self.door as f32, red: self.boss.is_some() }),
            relic: self.relic_shrine,
            decoy: self.decoy.map(|(p, _)| p),
            hero_kind: self.hero_kind,
            spirits: &self.spirits,
            fires: &self.fires,
            angels_spent: self.angels_spent(),
            red_key: self.red_key,
        };
        self.view.update(dt, &state);
        self.input.flush();
    }

    /// Play on the title: Scene I first (unless it is switched off), then the heroes.
    fn play_from_title(&mut self) {
        self.audio.unlock();
        if self.intro {
            self.begin_story();
        } else {
            self.open_select();
        }
        self.input.consume(Action::Attack);
        self.input.consume(Action::Confirm);
    }

    /// The buttons on the current screen, top to bottom (or left to right).
    pub fn menu_items(&self) -> Vec<MenuItem> {
        let quit = cfg!(not(target_arch = "wasm32"));
        let mut items = match self.screen {
            Screen::Title if self.confirm_reset => return self.title_items(),
            Screen::Title => self.title_items(),
            Screen::Paused => vec![MenuItem::Resume, MenuItem::CallOldMan, MenuItem::Controls, MenuItem::Map, MenuItem::QuitToTitle],
            _ => return Vec::new(),
        };
        if quit {
            items.push(MenuItem::Quit);
        }
        items
    }

    /**
     * Menu input for this update: arrows, W/S, the d-pad or the stick move the
     * highlight, the pointer highlights what it hovers, and Enter, Space, A,
     * Start or a click on a button picks. Returns the button chosen, if any.
     */
    fn menu_choice(&mut self) -> Option<MenuItem> {
        let items = self.menu_items();
        if items.is_empty() {
            return None;
        }
        let rects = crate::render::hud::menu_rects(self.screen, &items, self.viewport.0, self.viewport.1);
        let hit = |p: (f32, f32)| rects.iter().position(|r| p.0 >= r[0] && p.0 <= r[0] + r[2] && p.1 >= r[1] && p.1 <= r[1] + r[3]);
        let n = items.len();
        let horizontal = self.screen == Screen::Title;
        match self.input.nav {
            Some(Dir::Up) if !horizontal => self.menu_index = (self.menu_index + n - 1) % n,
            Some(Dir::Down) if !horizontal => self.menu_index = (self.menu_index + 1) % n,
            Some(Dir::Left) if horizontal => self.menu_index = (self.menu_index + n - 1) % n,
            Some(Dir::Right) if horizontal => self.menu_index = (self.menu_index + 1) % n,
            _ => {}
        }
        if self.input.cursor_moved && !self.input.mouse_captured {
            if let Some(i) = self.input.cursor.and_then(hit) {
                self.menu_index = i;
            }
        }
        self.menu_index = self.menu_index.min(n - 1);
        if let Some(p) = self.input.click {
            return hit(p).map(|i| {
                self.menu_index = i;
                items[i]
            });
        }
        if self.input.was_pressed(&[Action::Confirm, Action::Jump]) {
            return Some(items[self.menu_index]);
        }
        None
    }

    pub fn scene(&self) -> SceneState<'_> {
        SceneState {
            grid: &self.level.grid,
            hero: &self.hero,
            enemies: &self.enemies,
            frenzy: &self.frenzy,
            time: self.time,
            bombs: &self.bombs,
            projectiles: &self.projectiles,
            diamonds: &self.diamonds,
            potions: &self.potions,
            puddles: &self.puddles,
            boss: self.boss.as_ref(),
            boss_shots: &self.boss_shots,
            gate: self.gate.map(|tile| crate::render::view::GateView { tile, keys: self.keys_got, total: self.keys_total, door: self.door as f32, red: self.boss.is_some() }),
            relic: self.relic_shrine,
            decoy: self.decoy.map(|(p, _)| p),
            hero_kind: self.hero_kind,
            spirits: &self.spirits,
            fires: &self.fires,
            angels_spent: self.angels_spent(),
            red_key: self.red_key,
        }
    }

    fn handle_global_keys(&mut self) {
        if self.input.was_pressed(&[Action::Fps]) {
            self.show_fps = !self.show_fps;
        }
        if self.input.was_pressed(&[Action::Cheat]) {
            self.cheat_to_boss();
        }
        if self.input.was_pressed(&[Action::KeyCheat]) && self.screen == Screen::Playing {
            self.cheat_to_gate();
        }
        if self.input.was_pressed(&[Action::FreeMove]) {
            self.hero.free_move = !self.hero.free_move;
            crate::platform::save("vh.free_move", if self.hero.free_move { "1" } else { "0" });
            let p = self.hero.pos();
            let text = if self.hero.free_move { "FREE MOVEMENT" } else { "GRID MOVEMENT" };
            self.floater(p.x, p.y - 0.8, text, 1.4, Some(0xf2e3c4));
        }
        if self.input.was_pressed(&[Action::ShopCheat]) && self.screen != Screen::Shop {
            self.cheat_to_shop();
        }
        if self.input.was_pressed(&[Action::Call]) && matches!(self.screen, Screen::Playing | Screen::Paused) && !self.show_controls && !self.show_map {
            self.call_old_man();
        }
        if self.input.was_pressed(&[Action::Fullscreen]) {
            self.want_fullscreen_toggle = true;
        }
        if self.input.was_pressed(&[Action::Mute]) {
            self.audio.unlock();
            self.audio.toggle_mute();
        }
        // With the controller page or the map open, Esc and Start close it instead.
        if self.input.was_pressed(&[Action::Pause]) && !self.show_controls && !self.show_map {
            if self.screen == Screen::Playing {
                self.enter(Screen::Paused);
                // A controller's Menu button is Confirm as well: it must not pick Resume at once.
                self.input.consume(Action::Confirm);
                self.input.consume(Action::Jump);
            } else if self.screen == Screen::Paused {
                self.enter(Screen::Playing);
            }
        }
        if self.input.was_pressed(&[Action::Camera]) {
            self.view.camera_mode = if self.view.camera_mode == CameraMode::Third { CameraMode::Top } else { CameraMode::Third };
        }

        // Aiming, by mouse or right stick, only steers the chase camera.
        let aiming = self.input.aiming() && self.view.camera_mode == CameraMode::Third;
        self.view.mouse_look = aiming;
        let (dx, dy) = self.input.take_look();
        if self.input.was_pressed(&[Action::CameraReset]) {
            // R3: swing the view back behind him and let it follow again.
            self.input.pad_aiming = false;
            self.view.recenter();
        }
        // Looking around is allowed during the countdown too.
        if aiming && matches!(self.screen, Screen::Playing | Screen::Ready) {
            self.view.look(dx, dy);
        }
    }

    /// Pauses the game when the window loses focus.
    pub fn focus_lost(&mut self) {
        self.input.blur();
        if self.screen == Screen::Playing {
            self.enter(Screen::Paused);
        }
    }

    /**
     * Movement input. In third person a key means "relative to where the camera
     * looks": W runs into the view, A and D step to either side, S backs out.
     */
    fn resolve_wanted_dir(&mut self) -> Dir {
        let raw = self.input.direction();
        if self.view.camera_mode != CameraMode::Third || raw == Dir::None {
            self.latch = None;
            return raw;
        }
        // When the camera trails his heading, a turn on the spot swings the view,
        // which would re-map the still-held key and spin him again. So a held key
        // keeps the corridor it meant when pressed. With the mouse steering, the
        // camera does not follow him, and running where it looks stays live.
        if !self.view.mouse_look {
            if let Some((key, abs)) = self.latch {
                if key == raw {
                    return abs;
                }
            }
        }
        let abs = relative_dir(raw, self.camera_facing());
        self.latch = Some((raw, abs));
        abs
    }

    /// Free movement's direction in grid units: the held keys, turned to match the camera behind him.
    fn free_wish(&mut self) -> V {
        let (rx, ry) = self.input.move_vector();
        // The chase camera swings round behind him only while he runs (nearly) straight ahead;
        // running diagonally, sideways or back it holds still, so a held direction stays put.
        self.view.follow_heading = ry < -0.97;
        if self.view.camera_mode != CameraMode::Third {
            return V::new(rx, ry);
        }
        let yaw = self.view.cam_yaw as f64;
        let (fx, fy) = (yaw.cos(), yaw.sin());
        let (sx, sy) = (-fy, fx);
        V::new(fx * -ry + sx * rx, fy * -ry + sy * rx)
    }

    /// The corridor the camera is pointing down, with hysteresis near the diagonals.
    fn camera_facing(&mut self) -> Dir {
        let yaw = self.view.cam_yaw as f64;
        let off = |d: Dir| {
            let mut a = (yaw - dir_to_yaw(d) as f64) % (PI * 2.0);
            if a > PI {
                a -= PI * 2.0;
            }
            if a < -PI {
                a += PI * 2.0;
            }
            a.abs()
        };
        if self.cam_dir != Dir::None && off(self.cam_dir) < PI * 0.32 {
            return self.cam_dir;
        }
        let mut best = Dir::Down;
        let mut best_off = f64::INFINITY;
        for d in CARDINALS {
            let a = off(d);
            if a < best_off {
                best_off = a;
                best = d;
            }
        }
        self.cam_dir = best;
        best
    }

    fn tick(&mut self, dt: f64) {
        self.update_mode_schedule(dt);

        let wanted = self.resolve_wanted_dir();
        self.hero.wish = self.free_wish();
        let before = self.hero.pos();
        self.hero.update(dt, &self.level.grid, wanted);
        self.footsteps(before);
        self.collect();
        self.update_gate(dt);
        self.visit_shrines();
        self.visit_potions();
        self.visit_relic_shrine();
        self.update_decoy(dt);
        self.update_spirits(dt);
        self.update_fires(dt);
        self.watch_health();

        if self.input.was_pressed(&[Action::Attack, Action::Tap]) {
            self.attack();
        }
        if self.input.was_pressed(&[Action::Step]) {
            self.bifrost_step();
        }
        if self.input.was_pressed(&[Action::Angel]) {
            self.invoke_blessing();
        }
        if self.hero.winded {
            self.hero.winded = false;
            let hp = self.hero.pos();
            if !self.floaters.iter().any(|f| f.text == "OUT OF BREATH") {
                self.floater(hp.x, hp.y - 0.6, "OUT OF BREATH", 0.9, Some(0x9ad1ff));
            }
        }
        if self.input.was_pressed(&[Action::Jump]) && self.hero.try_jump(&self.level.grid) {
            self.audio.play(Sfx::Jump);
        }
        if self.input.was_pressed(&[Action::Bomb]) {
            self.place_bomb();
        }
        if self.input.was_pressed(&[Action::NextPotion]) && !self.belt.is_empty() {
            self.belt_index = (self.belt_index + 1) % self.belt.len();
            self.audio.play(Sfx::Nodule);
        }
        if self.input.was_pressed(&[Action::Drink]) {
            self.drink(self.belt_index);
        }
        for slot in 0..BELT_SLOTS {
            if self.input.was_pressed(&[Action::Slot(slot as u8)]) {
                self.drink(slot);
            }
        }
        self.update_projectiles(dt);
        self.update_angels(dt);
        self.update_bombs(dt);
        for d in self.diamonds.iter_mut() {
            d.update(dt);
        }
        if self.screen != Screen::Playing {
            return;
        }
        if self.boss.is_some() {
            self.update_boss(dt);
            if self.screen != Screen::Playing {
                return;
            }
        }

        if self.frenzy.update(dt) {
            for e in self.enemies.iter_mut() {
                e.exit_frenzy(self.global_mode);
            }
            self.audio.set_frenzy(false);
        }

        let hero = HeroView {
            pos: self.hero.pos(),
            dir: self.hero.dir(),
            heard_pos: self.hero.heard_pos,
            heard_age: self.hero.heard_age,
        };
        for i in 0..self.enemies.len() {
            let skeleton_pos = self.enemies.iter().find(|e| e.kind() == EnemyKind::Skeleton).map(|e| e.pos());
            let quarry = if self.enemies[i].is_ally() { self.quarry_for(self.enemies[i].pos()) } else { None };
            let mut ctx = EnemyContext {
                grid: &self.level.grid,
                hero,
                skeleton_pos,
                nest_center: self.level.nest_center,
                nest_exit: self.level.enemy_spawn,
                global_mode: self.global_mode,
                frenzy_active: self.frenzy.active(),
                hero_hidden: self.hero.invisible(),
                rage: self.rage(),
                decoy: self.decoy.map(|(p, _)| p),
                quarry,
                rng: &mut self.rng,
            };
            self.enemies[i].update(dt, &mut ctx);
        }
        self.update_spawns(dt);
        self.update_slime_trails(dt);

        self.resolve_contact();
        if self.screen != Screen::Playing {
            return;
        }
        self.update_effects(dt);
    }

    /// Reinforcements crawl out of the hollow on a schedule, cycling through the cast.
    fn update_spawns(&mut self, dt: f64) {
        if self.boss.is_some() {
            return; // the arena belongs to the Cyclops
        }
        self.spawn_timer -= dt;
        if self.spawn_timer > 0.0 || self.enemies.len() >= self.max_enemies() {
            return;
        }
        self.spawn_timer = self.spawn_every();
        let kind = SPAWN_CYCLE[self.spawned % SPAWN_CYCLE.len()];
        let mut spec = default_enemy_specs(&self.level.grid).into_iter().find(|s| s.kind == kind).unwrap();
        spec.exit_delay = 0.5;
        spec.name = format!("{} {}", spec.name, self.spawned / SPAWN_CYCLE.len() + 2);
        let graves = self.grave_spawns();
        let mut enemy = Enemy::new(&spec, self.level.nest_center, self.level.data.enemy_speed);
        let risen = if kind == EnemyKind::Skeleton && !graves.is_empty() {
            // A draugr claws its way up out of a grave.
            let g = graves[((self.rng.next() * graves.len() as f64) as usize).min(graves.len() - 1)];
            enemy.rise_at(g, crate::logic::enemy::REASSEMBLE_SECONDS);
            Some(g)
        } else {
            None
        };
        self.view.add_enemy(&enemy);
        self.enemies.push(enemy);
        self.spawned += 1;
        self.audio.play(Sfx::Fuse);
        match risen {
            Some(g) => self.floater(g.x as f64, g.y as f64 - 0.6, "THE DEAD RISE", 1.6, Some(0xe9e9f2)),
            None => {
                let c = self.level.nest_center;
                self.floater(c.x as f64, c.y as f64 - 1.0, "SOMETHING STIRS", 1.6, Some(0xb06cff));
            }
        }
    }

    /// 0..1 how enraged the dungeon is: the share of its keys taken.
    pub fn rage(&self) -> f64 {
        if self.boss.is_some() || self.keys_total == 0 {
            0.0
        } else {
            self.keys_got as f64 / self.keys_total as f64
        }
    }

    /// Seconds between reinforcements, shrinking with every key.
    fn spawn_every(&self) -> f64 {
        SPAWN_EVERY * (1.0 - (1.0 - SPAWN_EVERY_ENRAGED) * self.rage())
    }

    /// How many foes may roam at once, more with every key.
    fn max_enemies(&self) -> usize {
        MAX_ENEMIES + MAX_ENEMIES_PER_KEY * self.keys_got as usize
    }

    /// Walkable tiles beside the churchyard's graves, where draugr rise (none without a churchyard).
    fn grave_spawns(&self) -> Vec<P> {
        let Some(plan) = &self.garden else { return vec![] };
        let g = &self.level.grid;
        let mut out: Vec<P> = vec![];
        for p in &plan.graveyard {
            for d in crate::logic::types::CARDINALS {
                let n = g.neighbor(*p, d);
                if g.is_walkable(n.x, n.y, false) && !g.is_bridge(n.x, n.y) && g.get(n.x, n.y) == Tile::Open && !out.contains(&n) {
                    out.push(n);
                }
            }
        }
        out.sort_by_key(|p| (p.y, p.x));
        out
    }

    /// Moving slimes mark every tile they cross; standing in the slime slows the hero.
    fn update_slime_trails(&mut self, dt: f64) {
        self.puddles.update(dt);
        for (i, e) in self.enemies.iter().enumerate() {
            if e.kind() != EnemyKind::Slime || matches!(e.mode(), EnemyMode::Nest | EnemyMode::Eaten | EnemyMode::Leaving) {
                continue;
            }
            let o = e.occupied();
            if self.trail_tile.get(&i) != Some(&o) {
                self.trail_tile.insert(i, o);
                if self.level.grid.get(o.x, o.y) != Tile::Nest {
                    self.puddles.add(o, PUDDLE_SECONDS);
                }
            }
        }
        if self.hero.alive && !self.hero.airborne() && self.puddles.has(self.hero.occupied()) {
            self.hero.slow();
        }
    }

    fn update_mode_schedule(&mut self, dt: f64) {
        if self.frenzy.active() {
            return;
        }
        self.mode_timer -= dt;
        if self.global_mode != GlobalMode::Chase {
            // They hunt without a break: anything that set them scattering gives way to the hunt.
            self.global_mode = GlobalMode::Chase;
        }
        if self.mode_timer <= 0.0 && self.mode_index < MODE_SCHEDULE.len() - 1 {
            self.mode_index += 1;
            let (mode, secs) = MODE_SCHEDULE[self.mode_index];
            // Scatter spells shorten as the rage grows; hunts lengthen.
            self.mode_timer = if mode == GlobalMode::Scatter { secs * (1.0 - self.rage()) } else { secs * (1.0 + self.rage()) };
            self.global_mode = mode;
            for e in self.enemies.iter_mut() {
                if e.is_dangerous() {
                    e.reverse(&self.level.grid);
                }
            }
        }
    }

    fn floater(&mut self, x: f64, y: f64, text: impl Into<String>, life: f64, color: Option<u32>) {
        self.floaters.push(Floater { x, y, text: text.into(), life, max_life: life, color });
    }

    fn collect(&mut self) {
        if self.hero.airborne() {
            return; // pickups happen on the ground
        }
        let o = self.hero.occupied();
        let got = self.level.grid.consume(o.x, o.y);
        if got.is_some() {
            self.view.remove_collectible(o.x, o.y);
        }
        let (ox, oy) = (o.x as f64, o.y as f64);
        match got {
            Some(Tile::Nodule) => {
                self.keys_got += 1;
                if self.keys_got == 1 {
                    self.events.push(GameEvent::FirstKey);
                }
                self.award(POINTS_NODULE);
                self.earn(COINS_KEY);
                self.audio.play_reward(self.keys_got * 3);
                self.audio.play(Sfx::Anvil);
                self.view.shake(0.25);
                // Every other key the music presses on a little faster.
                if self.keys_got % KEYS_PER_TEMPO_STEP == 0 {
                    self.audio.set_tempo_step(self.keys_got / KEYS_PER_TEMPO_STEP);
                }
                self.view.spark(&self.level.grid, ox, oy, 0xf2c53d);
                self.view.sparkle(&self.level.grid, ox, oy, 0.6, 0xf2c53d, 40);
                self.view.dust(&self.level.grid, ox, oy, 0xb8902a, 10, 1.6);
                // The key flies from where it lay into the HUD counter.
                let (w, h) = self.viewport;
                let at = glam::Vec3::new(o.x as f32, floor_at(&self.level.grid, o.x as f32, o.y as f32) + 0.3, o.y as f32);
                let from = self.view.project(at, w, h).unwrap_or((w / 2.0, h / 2.0));
                self.juice.coins.push(FlyingCoin { from, t: 0.0 });
                let text = format!("KEY {}/{}", self.keys_got, self.keys_total);
                self.floater(ox, oy, text, 1.2, Some(0xf2c53d));
                // The dungeon takes it badly: someone new comes for him at once, and sooner after.
                if self.boss.is_none() && self.keys_got < self.keys_total {
                    self.spawn_timer = self.spawn_timer.min(1.5);
                }
                if self.keys_got < self.keys_total {
                    self.audio.play(Sfx::Milestone);
                    let left = self.keys_total - self.keys_got;
                    self.juice.banner = Some(Banner {
                        text: "A GOLDEN KEY".to_string(),
                        sub: format!("{left} MORE TO FIND   THE DEAD GROW RESTLESS"),
                        color: 0xf2c53d,
                        life: 1.8,
                        max: 1.8,
                    });
                } else {
                    self.unlock_gate();
                }
            }
            Some(Tile::Charge) => {
                let before = self.hero.hp;
                self.hero.heal(REMEDY_HEAL);
                let gained = self.hero.hp - before;
                self.award(POINTS_CHARGE);
                self.audio.play(Sfx::Heal);
                self.view.spark(&self.level.grid, ox, oy, 0x4cf06a);
                self.view.sparkle(&self.level.grid, ox, oy, 0.4, 0x5cff6a, 16);
                self.juice.hp_flash = 1.0;
                self.juice.hp_flash_color = 0x5cff6a;
                let text = if gained > 0 { format!("+{gained} HP") } else { "FULL HEALTH".to_string() };
                self.floater(ox, oy, text, 0.9, Some(0x5cff6a));
            }
            _ => {}
        }
    }

    /// The last key: every lock on the gate is lit. Now Ragnar must reach it.
    fn unlock_gate(&mut self) {
        self.gate_open = true;
        self.audio.play(Sfx::Win);
        let hp = self.hero.pos();
        self.view.sparkle(&self.level.grid, hp.x, hp.y, 0.6, 0xf2c53d, 40);
        let arena = self.boss.is_some();
        self.juice.banner = Some(Banner {
            text: "THE GATE IS UNLOCKED".into(),
            sub: if arena { "IN THE NORTH HEDGE: GO THROUGH AND ON DOWN".into() } else { "FIND IT IN THE NORTH WALL AND GO THROUGH".into() },
            color: if arena { RED_KEY_COLOR } else { 0xf2c53d },
            life: 3.0,
            max: 3.0,
        });
        if self.gate.is_none() {
            self.win(); // an estate without a gate: straight on to the boss
        }
    }

    /**
     * The gate in the outer wall. Locked, it tells Ragnar how many keys he
     * still lacks. Unlocked, its doors swing open as he comes up to them, and
     * stepping through leads on to the boss.
     */
    fn update_gate(&mut self, dt: f64) {
        let Some(g) = self.gate else { return };
        self.gate_hint = (self.gate_hint - dt).max(0.0);
        let hp = self.hero.pos();
        let near = (hp.x - g.x as f64).abs() < 1.2 && (hp.y - g.y as f64).abs() < 1.7;
        if near && !self.gate_open && self.gate_hint <= 0.0 {
            self.gate_hint = 3.0;
            self.audio.play(Sfx::Denied);
            let (text, color) = match &self.boss {
                Some(b) if b.alive() => ("LOCKED: THE CYCLOPS HOLDS THE KEY".to_string(), RED_KEY_COLOR),
                Some(_) => ("LOCKED: TAKE THE RED DIAMOND KEY".to_string(), RED_KEY_COLOR),
                None => (format!("LOCKED  {}/{} KEYS", self.keys_got, self.keys_total), 0xf2c53d),
            };
            self.floater(g.x as f64, g.y as f64 + 0.6, text, 1.6, Some(color));
        }
        if near && self.gate_open && !self.door_opening {
            self.door_opening = true;
            self.audio.play(Sfx::BossCharge);
            self.view.shake(0.35);
        }
        if self.door_opening && self.door < 1.0 {
            self.door = (self.door + dt / DOOR_SECONDS).min(1.0);
            if self.door >= 1.0 {
                self.level.grid.set(g.x, g.y, Tile::Open);
                self.level.grid.set_elevation(g.x, g.y, self.level.grid.elevation(g.x, g.y + 1));
                self.audio.play(Sfx::Whoosh);
            }
        }
        if self.door >= 1.0 && self.hero.occupied() == g {
            if self.boss.is_some() {
                // Out of the Cyclops' meadow: the victory, and then the old man.
                self.gate_open = false;
                self.audio.play(Sfx::Win);
                self.enter(Screen::Won);
            } else {
                self.win();
            }
        }
    }

    /**
     * Through the gate: the dungeon is behind him. Congratulations are in
     * order, and then the ground shakes: the way to the Cyclops opens.
     */
    fn win(&mut self) {
        self.audio.play(Sfx::Win);
        self.audio.set_frenzy(false);
        self.audio.set_threat(0.0);
        self.frenzy.cancel();
        let hp = self.hero.pos();
        self.view.spark(&self.level.grid, hp.x, hp.y, 0xf2c53d);
        self.view.sparkle(&self.level.grid, hp.x, hp.y, 0.6, 0xf2c53d, 40);
        self.events.push(GameEvent::Won);
        self.juice.banner = Some(Banner {
            text: "THROUGH THE GATE".into(),
            sub: "SOMETHING VAST STIRS IN THE MEADOW...".into(),
            color: 0xf2c53d,
            life: CLEARED_SECONDS as f32,
            max: CLEARED_SECONDS as f32,
        });
        self.enter(Screen::Cleared);
    }

    /// F2: every key found at once, and Ragnar at the gate (testing cheat).
    pub fn cheat_to_gate(&mut self) {
        let Some(g) = self.gate else { return };
        for (x, y, t) in self.level.grid.cells().collect::<Vec<_>>() {
            if t == Tile::Nodule {
                self.level.grid.set(x, y, Tile::Open);
                self.view.remove_collectible(x, y);
            }
        }
        self.keys_got = self.keys_total;
        self.unlock_gate();
        self.hero.m.teleport(P::new(g.x, g.y + 2));
        self.hero.face(Dir::Up);
        self.view.aim_along(Dir::Up);
    }

    /// F1: skip straight to the first Cyclops, from anywhere.
    pub fn cheat_to_boss(&mut self) {
        self.audio.unlock();
        self.tour = None;
        if matches!(self.screen, Screen::Title | Screen::GameOver) {
            self.score.reset();
            self.juice.score_shown = 0.0;
            self.new_kit();
        }
        self.load_boss(0);
        self.juice.fade = 1.0;
        self.enter(Screen::Ready);
        self.input.consume(Action::Attack);
    }

    /// 0: straight to the old man's wares, from anywhere, with silver and runes to test with.
    pub fn cheat_to_shop(&mut self) {
        self.audio.unlock();
        self.tour = None;
        if matches!(self.screen, Screen::Title | Screen::GameOver) {
            self.score.reset();
            self.juice.score_shown = 0.0;
            self.new_kit();
            self.load_level(0);
        }
        self.progress.silver = self.progress.silver.max(CHEAT_COINS);
        self.progress.runes = self.progress.runes.max(CHEAT_RUNES);
        self.open_shop();
        self.juice.fade = 1.0;
        self.input.consume(Action::Attack);
    }

    // ------------------------------------------------------------------ boss

    fn update_boss(&mut self, dt: f64) {
        let hero_pos = self.hero.pos();
        let mut events = std::mem::take(&mut self.boss_events);
        let was_active = self.boss.as_ref().map(|b| b.active()).unwrap_or(false);
        if let Some(b) = &mut self.boss {
            b.update(dt, hero_pos, &mut self.boss_shots, &mut events);
        }
        let now_active = self.boss.as_ref().map(|b| b.active()).unwrap_or(false);
        if now_active && !was_active {
            self.audio.set_tempo_step(1);
        }
        for e in events.drain(..) {
            self.boss_event(e);
        }
        self.boss_events = events;
        self.update_boss_shots(dt);
        if self.screen != Screen::Playing {
            return;
        }
        if self.boss_bleed > 0.0 {
            self.boss_bleed -= dt;
            self.bleed_tick -= dt;
            if let Some(bp) = self.boss.as_ref().filter(|b| b.active()).map(|b| b.pos) {
                if self.bleed_tick <= 0.0 {
                    self.bleed_tick = BLEED_TICK;
                    self.wound_boss(1, bp);
                }
                if self.rng.next() < dt * 20.0 {
                    self.view.sparkle(&self.level.grid, bp.x, bp.y, 0.6 + self.rng.next() as f32, self.bleed_color, 2);
                }
            } else {
                self.boss_bleed = 0.0;
            }
        }
        if self.poison.0 > 0 {
            self.poison.1 -= dt;
            self.poison.2 -= dt;
            if let Some(bp) = self.boss.as_ref().filter(|b| b.active()).map(|b| b.pos) {
                if self.poison.2 <= 0.0 {
                    self.poison.2 = POISON_TICK;
                    self.wound_boss(self.poison.0, bp);
                }
                if self.rng.next() < dt * 14.0 {
                    self.view.sparkle(&self.level.grid, bp.x, bp.y, 0.8, 0x6ae86a, 2);
                }
            }
            if self.poison.1 <= 0.0 {
                self.poison = (0, 0.0, 0.0);
            }
        }
        // Touching it hurts, unless he sails over it; a thorned shield or dragonfire bites back.
        if let Some(b) = &self.boss {
            let near = dist_sq(b.pos, hero_pos) < CONTACT_RADIUS * CONTACT_RADIUS;
            if b.harmful() && b.hover < 1.3 && near && !self.hero.airborne() && !self.hero.invisible() {
                let bp = b.pos;
                let open = !self.hero.invulnerable() && !self.hero.invincible();
                let thorns = self.hero.shield.map(|s| progression::ward(s.tier).thorns).unwrap_or(0);
                let fire = if self.hero.fx.fire > 0.0 { DRAGONFIRE_DAMAGE } else { 0 };
                self.hurt_hero(CONTACT_DAMAGE);
                if open && thorns + fire > 0 {
                    self.wound_boss(thorns + fire, bp);
                }
            }
        }
        self.take_red_key();
    }

    /// Walking over the red diamond key the Cyclops dropped: the arena's gate is unlocked.
    fn take_red_key(&mut self) {
        let Some(k) = self.red_key else { return };
        if !self.hero.alive || dist_sq(self.hero.pos(), k.v()) > RED_KEY_REACH * RED_KEY_REACH {
            return;
        }
        self.red_key = None;
        self.keys_got = self.keys_total;
        let (x, y) = (k.x as f64, k.y as f64);
        self.view.sparkle(&self.level.grid, x, y, 1.0, RED_KEY_COLOR, 50);
        self.view.flash_light(&self.level.grid, k.v(), RED_KEY_COLOR);
        self.audio.play(Sfx::Milestone);
        self.floater(x, y - 0.8, "RED DIAMOND KEY", 2.0, Some(RED_KEY_COLOR));
        self.unlock_gate();
    }

    /**
     * The Cyclops' red diamond key falls where it died, or on the nearest
     * open grass if it died over stone or water: a tile he can walk onto.
     */
    fn drop_red_key(&mut self, at: V) {
        let (x0, y0, x1, y1) = ARENA;
        let g = &self.level.grid;
        let spot = g
            .cells()
            .filter(|&(x, y, t)| x >= x0 && x <= x1 && y >= y0 && y <= y1 && matches!(t, Tile::Open | Tile::Charge) && g.is_walkable(x, y, false))
            .map(|(x, y, _)| P::new(x, y))
            .min_by(|a, b| dist_sq(a.v(), at).total_cmp(&dist_sq(b.v(), at)));
        self.red_key = spot;
        if let Some(k) = spot {
            self.view.sparkle(&self.level.grid, k.x as f64, k.y as f64, 1.2, RED_KEY_COLOR, 40);
        }
    }

    fn boss_event(&mut self, e: BossEvent) {
        let grid = &self.level.grid;
        match e {
            BossEvent::Roar => {
                self.audio.play(Sfx::BossRoar);
                self.view.shake(0.35);
            }
            BossEvent::Charge => self.audio.play(Sfx::BossCharge),
            BossEvent::Fire(kind) => {
                self.audio.play(if kind == ShotKind::Boulder { Sfx::BossHurl } else { Sfx::BossBolt });
                if let Some(b) = &self.boss {
                    let p = b.pos;
                    if kind == ShotKind::Boulder {
                        self.view.dust(grid, p.x, p.y, 0x8a6a40, 10, 2.0);
                    }
                }
            }
            BossEvent::Slam(p) => {
                self.audio.play(Sfx::BossSlam);
                self.view.shockwave(grid, p.x, p.y);
                self.view.dust(grid, p.x, p.y, 0x8a6a40, 34, 4.2);
                self.view.dust(grid, p.x, p.y, 0x5aa834, 16, 3.0);
                let d = dist_sq(p, self.hero.pos()).sqrt() as f32;
                self.view.shake(0.7 * (1.0 - d / 14.0).max(0.2));
            }
            BossEvent::Enraged(phase) => {
                self.audio.set_tempo_step(phase as u32);
                self.juice.banner = Some(Banner {
                    text: if phase == 2 { "THE CYCLOPS IS ENRAGED".into() } else { "THE CYCLOPS IS FURIOUS".into() },
                    sub: if phase == 2 { "IT DIVES NOW   DODGE, THEN STRIKE WHILE IT IS DAZED".into() } else { "FINISH IT!".into() },
                    color: if phase == 2 { 0xff8a2a } else { 0xff4d4d },
                    life: 2.2,
                    max: 2.2,
                });
                self.drop_remedy();
            }
            BossEvent::Spasm(p) => {
                self.view.spark(grid, p.x, p.y, 0xffb060);
                self.view.dust(grid, p.x, p.y, 0x8a2a6a, 8, 3.0);
                self.view.shake(0.12);
                self.audio.play(Sfx::ArrowHit);
            }
            BossEvent::Died(p) => {
                self.view.explode(grid, p.x, p.y, 0);
                self.view.sparkle(grid, p.x, p.y, 1.2, 0xf2c53d, 60);
                self.view.sparkle(grid, p.x, p.y, 1.0, 0xffffff, 30);
                self.view.dust(grid, p.x, p.y, 0x8a2a6a, 40, 5.0);
                self.view.shockwave(grid, p.x, p.y);
                self.view.shake(1.0);
                self.audio.play(Sfx::Explode);
                self.audio.play(Sfx::BossDeath);
                self.audio.set_threat(0.0);
                self.award(POINTS_CYCLOPS);
                let bounty = COINS_BOSS + 100 * self.dungeon as u32;
                self.earn(bounty);
                self.floater(p.x, p.y - 0.8, format!("+{bounty} {}", progression::coin(self.hero_kind)), 2.2, Some(0xffd84a));
                self.floater(p.x, p.y, format!("+{POINTS_CYCLOPS}"), 2.0, Some(0xf2c53d));
                let runes = RUNES_BOSS + self.dungeon as u32;
                self.progress.runes += runes;
                self.floater(p.x, p.y + 0.8, format!("+{runes} {}", progression::gems(self.hero_kind, runes)), 2.4, Some(0x9ad1ff));
                // Its downfall gives up a relic of the old heroes.
                self.take_relic(p);
                self.events.push(GameEvent::BossSlain);
                // It drops the red diamond key to the gate in the north hedge.
                self.drop_red_key(p);
                self.juice.banner = Some(Banner {
                    text: "THE CYCLOPS IS SLAIN".into(),
                    sub: "TAKE ITS RED DIAMOND KEY TO THE GATE IN THE NORTH HEDGE".into(),
                    color: RED_KEY_COLOR,
                    life: 3.5,
                    max: 3.5,
                });
                self.save_progress();
                self.checkpoint(save::Stage::Shop(self.dungeon));
            }
        }
    }

    /// A green cross appears on open grass away from the Cyclops: a breather between phases.
    fn drop_remedy(&mut self) {
        let (x0, y0, x1, y1) = ARENA;
        let boss = self.boss.as_ref().map(|b| b.pos).unwrap_or_default();
        let hero = self.hero.pos();
        let mut best: Option<(f64, P)> = None;
        for _ in 0..40 {
            let p = P::new(x0 + self.rng.index((x1 - x0 + 1) as usize) as i32, y0 + self.rng.index((y1 - y0 + 1) as usize) as i32);
            if self.level.grid.get(p.x, p.y) != Tile::Open {
                continue;
            }
            // Not on top of him, not under the eye.
            let score = dist_sq(p.v(), boss).sqrt().min(6.0) - (dist_sq(p.v(), hero).sqrt() - 4.0).abs();
            if best.map(|(s, _)| score > s).unwrap_or(true) {
                best = Some((score, p));
            }
        }
        if let Some((_, p)) = best {
            self.level.grid.set(p.x, p.y, Tile::Charge);
            self.view.add_charge(&self.level.grid, p.x, p.y);
            self.view.sparkle(&self.level.grid, p.x as f64, p.y as f64, 0.4, 0x5cff6a, 20);
            self.floater(p.x as f64, p.y as f64, "HEALTH", 1.6, Some(0x5cff6a));
        }
    }

    /// Bolts and boulders: they fly until they hit rock, Ragnar, or run out.
    fn update_boss_shots(&mut self, dt: f64) {
        let grid = &self.level.grid;
        let hero_pos = self.hero.pos();
        let hero_open = self.hero.alive && !self.hero.airborne();
        let mut hurt = 0;
        for s in self.boss_shots.iter_mut() {
            let mut remaining = dt;
            while remaining > 0.0 && s.alive {
                let step = remaining.min(0.02);
                remaining -= step;
                s.pos = V::new(s.pos.x + s.vel.x * step, s.pos.y + s.vel.y * step);
                s.rolled += (s.vel.x.hypot(s.vel.y)) * step;
                s.life -= step;
                let t = P::new(s.pos.x.round() as i32, s.pos.y.round() as i32);
                if s.life <= 0.0 || !grid.in_bounds(t.x, t.y) {
                    s.alive = false;
                } else if grid.get(t.x, t.y) == Tile::Rock {
                    s.alive = false;
                    let color = if s.kind == ShotKind::Boulder { 0x8a867c } else { 0xff7a3a };
                    self.view.spark(grid, s.pos.x, s.pos.y, color);
                    self.view.dust(grid, s.pos.x, s.pos.y, if s.kind == ShotKind::Boulder { 0x8a867c } else { 0x4a3620 }, 8, 2.0);
                } else if hero_open && dist_sq(s.pos, hero_pos).sqrt() < s.kind.radius() + 0.3 {
                    s.alive = false;
                    hurt = hurt.max(s.kind.damage());
                    self.view.spark(grid, s.pos.x, s.pos.y, 0xff7a3a);
                }
            }
        }
        self.boss_shots.retain(|s| s.alive);
        if hurt > 0 {
            let ward = self.hero.shield.map(|s| progression::ward(s.tier));
            let open = !self.hero.invulnerable() && !self.hero.invincible();
            if let (Some(w), true) = (ward, open) {
                if w.missile_proof {
                    // The missile smashes on the shield: it wears, but nothing gets through.
                    if let Some(s) = &mut self.hero.shield {
                        let floor = if w.unbreakable { 1 } else { 0 };
                        s.durability = (s.durability - (hurt as f64 * w.block).round() as i32).max(floor);
                    }
                    self.hero.invuln = 0.5;
                    self.hero.heal(w.aegis);
                    let hp = self.hero.pos();
                    self.audio.play(Sfx::Nodule);
                    self.view.sparkle(&self.level.grid, hp.x, hp.y, 0.5, 0x7fd7ff, 12);
                    self.floater(hp.x, hp.y, if w.reflect { "TURNED BACK" } else { "WARDED" }, 0.8, Some(0x7fd7ff));
                    if w.reflect {
                        if let Some(bp) = self.boss_target() {
                            self.view.zap(&self.level.grid, hp, bp);
                            self.wound_boss(REFLECT_DAMAGE, bp);
                        }
                    }
                    if self.hero.shield.map(|s| s.durability <= 0).unwrap_or(false) {
                        self.hero.shield = None;
                        self.hero.shield_broke = true;
                        self.hurt_hero(0);
                    }
                    return;
                }
            }
            self.hurt_hero(hurt);
        }
    }

    /// A blow to the Cyclops from any weapon, with all the feedback it deserves.
    fn wound_boss(&mut self, amount: i32, at: V) -> bool {
        self.damage_boss(amount, at, false)
    }

    /// An archangel's blow: exactly `amount`, dazed or not.
    fn smite_boss(&mut self, amount: i32, at: V) -> bool {
        let landed = self.damage_boss(amount, at, true);
        if landed {
            self.floater(at.x, at.y - 1.1, format!("-{amount}"), 1.2, Some(0xfff1a8));
        }
        landed
    }

    fn damage_boss(&mut self, amount: i32, at: V, exact: bool) -> bool {
        let mut events = std::mem::take(&mut self.boss_events);
        let landed = self.boss.as_mut().map(|b| if exact { b.smite(amount, &mut events) } else { b.hit(amount, &mut events) }).unwrap_or(false);
        let dying = self.boss.as_ref().map(|b| !b.alive()).unwrap_or(false);
        if landed {
            let grid = &self.level.grid;
            self.audio.play(Sfx::BossHit);
            self.view.spark(grid, at.x, at.y, 0xffe0e0);
            self.view.dust(grid, at.x, at.y, 0x8a2a6a, 10, 2.6);
            self.view.sparkle(grid, at.x, at.y, 1.0, 0xfff1c0, 6);
            self.view.shake(0.16);
            self.hitstop = HITSTOP_BOSS;
            if dying {
                // Its last scream pops everything still in the air.
                for s in self.boss_shots.drain(..) {
                    self.view.spark(grid, s.pos.x, s.pos.y, 0xffb060);
                }
                self.hitstop = 0.12;
            }
            self.award(10 * amount as u32);
        }
        for e in events.drain(..) {
            self.boss_event(e);
        }
        self.boss_events = events;
        landed
    }

    /// Where the Cyclops is, if it can be struck at all.
    fn boss_target(&self) -> Option<V> {
        self.boss.as_ref().filter(|b| b.vulnerable()).map(|b| b.pos)
    }

    /// Stepping onto an active shrine takes whatever it is showing.
    fn visit_shrines(&mut self) {
        if self.hero.airborne() {
            return;
        }
        let o = self.hero.occupied();
        for i in 0..self.diamonds.len() {
            let d = &self.diamonds[i];
            if !d.active || d.tile != o {
                continue;
            }
            if let Offer::Angel(angel) = d.current() {
                self.receive_blessing(i, angel);
                continue;
            }
            let chosen = self.diamonds[i].take();
            match chosen {
                Offer::Angel(_) => {}
                Offer::Item(ItemKind::Shield) => self.hero.grant_shield(),
                Offer::Item(ItemKind::Invincible) => self.hero.grant_invincibility(),
                Offer::Item(ItemKind::Speed) => self.hero.grant_speed(),
            }
            self.audio.play(Sfx::Start);
            self.floater(o.x as f64, o.y as f64, chosen.name().to_uppercase(), 1.4, Some(chosen.color()));
        }
    }

    /// Walking over a bottle drinks it. Bottles do not come back within a dungeon.
    fn visit_potions(&mut self) {
        if self.hero.airborne() {
            return;
        }
        let o = self.hero.occupied();
        for i in 0..self.potions.len() {
            if self.potions[i].taken || self.potions[i].tile != o {
                continue;
            }
            self.potions[i].taken = true;
            let kind = self.potions[i].kind;
            let def = self.potions[i].def(self.hero_kind);
            self.view.spark(&self.level.grid, o.x as f64, o.y as f64, def.color);
            // Onto the belt if there is room; an extra life (or a full belt) is drunk at once.
            if kind != PotionKind::Life && self.belt.len() < BELT_SLOTS {
                self.belt.push(kind);
                self.audio.play(Sfx::Start);
                self.floater(o.x as f64, o.y as f64, format!("{} POTION", def.name.to_uppercase()), 1.4, Some(def.color));
            } else {
                self.quaff(kind);
            }
        }
    }

    /**
     * The one attack button (E, left click, X). Facing a shrine, it cycles the
     * shrine. Otherwise it swings the sword, or fires the ranged weapon that
     * has replaced it.
     */
    fn attack(&mut self) {
        if !self.hero.alive {
            return;
        }
        let hp = self.hero.pos();
        let mut cycled = false;
        for i in 0..self.diamonds.len() {
            let d = &mut self.diamonds[i];
            if d.active && tile_in_reach(hp, self.hero.aim(), d.tile) {
                d.cycle();
                cycled = true;
                let t = d.tile;
                self.audio.play(Sfx::Nodule);
                self.view.spark(&self.level.grid, t.x as f64, t.y as f64, 0xffffff);
            }
        }
        if cycled {
            return;
        }
        if !self.hero.has_ranged_weapon() {
            if self.hero.try_attack() {
                self.perform_attack();
            }
            return;
        }
        self.fire();
    }

    /**
     * A swing of his armory blade: its reach and arc slay every foe caught
     * and strike the Cyclops; then whatever rides on the blade: a piercing
     * wave, a stunning oath, a trick, a bleeding cut.
     */
    fn perform_attack(&mut self) {
        let blade = progression::blade(self.hero_kind, self.hero.blade);
        if blade.arrows > 0 {
            self.rain_arrows(blade);
            return;
        }
        self.audio.play(Sfx::Slash);
        self.view.sword_slash();
        if blade.lunge > 0.0 {
            // Gale Dash: a gust carries him forward into the cut.
            if let Some((from, to)) = self.hero.dash(&self.level.grid, blade.lunge as i32, 0.0) {
                self.view.zap(&self.level.grid, from, to);
                self.view.dust(&self.level.grid, from.x, from.y, 0xd8f0e8, 10, 2.0);
                self.audio.play(Sfx::Whoosh);
            }
        }
        if blade.flash > 0.0 {
            self.solar_flash(blade);
        }
        let Some((reach, half_width, behind)) = blade.swing else { return };
        let hp = self.hero.pos();
        let aim = self.hero.aim();
        let mut hits = find_blade_targets(hp, aim, &self.enemies, reach, half_width, behind);
        if blade.burn > 0.0 {
            // Sacred flames lick beyond the blade's reach.
            for i in find_blade_targets(hp, aim, &self.enemies, reach + 0.9, half_width + 0.8, behind) {
                if !hits.contains(&i) {
                    let p = self.enemies[i].pos();
                    self.view.sparkle(&self.level.grid, p.x, p.y, 0.8, FIRE_COLOR, 16);
                    hits.push(i);
                }
            }
        }
        let landed_any = !hits.is_empty();
        if landed_any {
            self.view.shake(0.14 + 0.02 * self.hero.blade as f32);
        }
        for i in hits {
            self.slay_enemy(i);
        }
        let mut struck_boss = false;
        if let Some(bp) = self.boss_target() {
            if in_blade_arc(hp, aim, bp, reach, half_width, behind, BODY_RADIUS) {
                struck_boss = self.strike_boss(blade.damage, bp);
            }
        }
        if blade.burn > 0.0 && struck_boss {
            self.set_boss_alight(blade.burn);
        }
        if blade.poison && struck_boss {
            self.poison = ((self.poison.0 + 1).min(POISON_MAX), POISON_SECONDS, if self.poison.0 == 0 { POISON_TICK } else { self.poison.2 });
        }
        if blade.blood > 0 && (landed_any || struck_boss) {
            // Muramasa drinks from the hand that holds it.
            self.hero.hp = (self.hero.hp - blade.blood).max(1);
            self.floater(hp.x, hp.y - 0.4, format!("-{}", blade.blood), 0.8, Some(0xc83a5a));
        }
        if blade.stealth > 0.0 {
            self.hero.grant_invisibility(self.hero.invisible_timer.max(blade.stealth));
        }
        if blade.fire_trail {
            self.lay_fire(hp, aim);
        }
        if blade.wave {
            // Angurvadel's stream of anguish: a wave of force down the line.
            self.projectiles.extend(spawn_shots(crate::logic::items::RangedKind::Wave, hp, aim));
        }
        if blade.stun > 0.0 {
            self.stun_around(hp, progression::STUN_RADIUS, blade.stun);
            if struck_boss {
                let staggered = self.boss.as_mut().and_then(|b| b.stagger().then_some(b.pos));
                if let Some(p) = staggered {
                    self.floater(p.x, p.y - 0.9, "STAGGERED", 1.0, Some(0xe0c070));
                }
            }
        }
        if blade.trick {
            self.loki_trick(hp);
        }
        if blade.bleed > 0.0 && struck_boss {
            if self.boss_bleed <= 0.0 {
                self.bleed_tick = BLEED_TICK;
            }
            self.boss_bleed = blade.bleed;
        }
    }

    /**
     * A blow of `base` damage to the Cyclops, as everything he has makes it:
     * his elixirs, the Cyclops' armour (from the second dungeon on, unless the
     * blade ignores it), a critical hit (always, for a blade that never misses;
     * sometimes, with Seer's Sight), and triple from behind (Wolf's Fang).
     */
    fn strike_boss(&mut self, base: i32, at: V) -> bool {
        let blade = progression::blade(self.hero_kind, self.hero.blade);
        let mut dmg = self.hero.blow(base);
        if self.dungeon >= 1 && !blade.pierce_armor {
            dmg = (dmg - BOSS_ARMOR).max(1);
        }
        let mut tag = None;
        if blade.crit || (self.hero.sees() && self.rng.next() < path::SIGHT_CRIT) {
            dmg *= 2;
            tag = Some(("CRITICAL!", 0xffd84a));
        }
        let from = self.hero.pos();
        if self.hero.path >= 6 && self.boss.as_ref().map(|b| b.behind(from)).unwrap_or(false) {
            dmg *= path::FANG_MULT;
            tag = Some(("BACKSTAB!", 0xff5a3a));
        } else if self.hero.path >= 6 && self.hero_kind != HeroKind::Ragnar && self.boss.as_ref().map(|b| b.flank(from)).unwrap_or(false) {
            dmg *= path::FLANK_MULT;
            tag = Some(("FLANKED!", 0xffa050));
        }
        let landed = self.wound_boss(dmg, at);
        if let (true, Some((text, color))) = (landed, tag) {
            self.floater(at.x, at.y - 1.1, text, 1.0, Some(color));
        }
        landed
    }

    /// Sets the Cyclops burning for `seconds`.
    fn set_boss_alight(&mut self, seconds: f64) {
        if self.boss_bleed <= 0.0 {
            self.bleed_tick = BLEED_TICK;
        }
        self.boss_bleed = self.boss_bleed.max(seconds);
        self.bleed_color = FIRE_COLOR;
    }

    /**
     * A bow of the heavens (Yumi, Simurg): arrows rain down out of the sky on
     * the nearest foes in range, one each, and two on the Cyclops if it is near.
     */
    fn rain_arrows(&mut self, blade: progression::Blade) {
        let hp = self.hero.pos();
        let color = progression::node(self.hero_kind, Tree::Armory, self.hero.blade).color;
        self.audio.play(Sfx::Arrow);
        let mut left = blade.arrows;
        if let Some(bp) = self.boss_target() {
            if dist_sq(bp, hp) <= (progression::ARROW_RANGE + 1.0).powi(2) {
                for k in 0..2 {
                    let at = V::new(bp.x + (k as f64 - 0.5) * 0.6, bp.y);
                    self.view.rain(&self.level.grid, at, color);
                    self.strike_boss(blade.damage, bp);
                }
                if blade.burn > 0.0 {
                    self.set_boss_alight(blade.burn);
                }
                left -= 2;
            }
        }
        let mut near: Vec<(f64, usize)> = self
            .enemies
            .iter()
            .enumerate()
            .filter(|(_, e)| e.is_targetable())
            .map(|(i, e)| (dist_sq(e.pos(), hp), i))
            .filter(|&(d, _)| d <= progression::ARROW_RANGE * progression::ARROW_RANGE)
            .collect();
        near.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        for &(_, i) in near.iter().take(left) {
            let p = self.enemies[i].pos();
            self.view.rain(&self.level.grid, p, color);
            self.slay_enemy(i);
        }
        if near.is_empty() && left == blade.arrows {
            // Nothing in range: the volley falls ahead of him.
            let f = self.hero.aim();
            self.view.rain(&self.level.grid, V::new(hp.x + f.x * 3.0, hp.y + f.y * 3.0), color);
        }
    }

    /**
     * A sun-blade's flash (Amaterasu, Kaf Dagi): blinding light bursts from
     * him, every foe within reach is obliterated and the Cyclops, if near, seared.
     */
    fn solar_flash(&mut self, blade: progression::Blade) {
        let hp = self.hero.pos();
        let r = blade.flash;
        self.view.shockwave(&self.level.grid, hp.x, hp.y);
        self.view.sparkle(&self.level.grid, hp.x, hp.y, 1.4, 0xfff1a8, 60);
        self.view.flash_light(&self.level.grid, hp, 0xfff1c0);
        self.view.shake(0.4);
        self.audio.play(Sfx::Zap);
        let doomed: Vec<usize> = self.enemies.iter().enumerate().filter(|(_, e)| e.is_targetable() && dist_sq(e.pos(), hp) <= r * r).map(|(i, _)| i).collect();
        for i in doomed {
            self.slay_enemy(i);
        }
        if let Some(bp) = self.boss_target() {
            if dist_sq(bp, hp) <= (r + 1.0) * (r + 1.0) {
                self.strike_boss(blade.damage, bp);
            }
        }
    }

    /// A greatsword's holy fire: the tiles ahead of him along `aim` burn for a few seconds.
    fn lay_fire(&mut self, from: V, aim: V) {
        let g = &self.level.grid;
        let o = g.wrap(P::new(from.x.round() as i32, from.y.round() as i32));
        let mut last = o;
        for k in 1..=6 {
            let d = k as f64 * 0.5;
            let t = g.wrap(P::new((from.x + aim.x * d).round() as i32, (from.y + aim.y * d).round() as i32));
            if t == last {
                continue;
            }
            last = t;
            if !g.is_walkable(t.x, t.y, false) {
                break;
            }
            match self.fires.iter_mut().find(|(p, _)| *p == t) {
                Some(f) => f.1 = FIRE_SECONDS,
                None => self.fires.push((t, FIRE_SECONDS)),
            }
        }
        self.audio.play(Sfx::Fuse);
    }

    /// The holy fire burns out; while it lasts, foes on it burn to ash and the Cyclops over it catches.
    fn update_fires(&mut self, dt: f64) {
        for f in self.fires.iter_mut() {
            f.1 -= dt;
        }
        self.fires.retain(|f| f.1 > 0.0);
        if self.fires.is_empty() {
            return;
        }
        let burning: Vec<usize> = self
            .enemies
            .iter()
            .enumerate()
            .filter(|(_, e)| e.is_targetable() && self.fires.iter().any(|(p, _)| *p == e.occupied()))
            .map(|(i, _)| i)
            .collect();
        for i in burning {
            let p = self.enemies[i].pos();
            self.view.sparkle(&self.level.grid, p.x, p.y, 0.8, FIRE_COLOR, 16);
            self.slay_enemy(i);
        }
        if let Some(bp) = self.boss_target() {
            if self.fires.iter().any(|(p, _)| dist_sq(p.v(), bp) < 1.6 * 1.6) {
                self.set_boss_alight(1.0);
            }
        }
    }

    /**
     * The ancestral spirits (path 9 of Mehmed and Miyamoto): they float round
     * him and strike down any foe that comes near, or the Cyclops.
     */
    fn update_spirits(&mut self, dt: f64) {
        let (count, every) = progression::spirits(self.hero_kind);
        let want = if self.hero.path >= 9 && self.hero.alive { count } else { 0 };
        let hp = self.hero.pos();
        while self.spirits.len() < want {
            let k = self.spirits.len() as f64;
            self.spirits.push(Spirit { pos: hp, cooldown: every * (0.5 + k / count.max(1) as f64), strike: 0.0 });
        }
        self.spirits.truncate(want);
        let n = self.spirits.len().max(1) as f64;
        for i in 0..self.spirits.len() {
            let a = self.time * 0.9 + i as f64 / n * std::f64::consts::TAU;
            let goal = V::new(hp.x + a.cos() * 1.1, hp.y + a.sin() * 1.1);
            let k = 1.0 - (-4.0 * dt).exp();
            let sp = &mut self.spirits[i];
            sp.pos = V::new(sp.pos.x + (goal.x - sp.pos.x) * k, sp.pos.y + (goal.y - sp.pos.y) * k);
            sp.cooldown -= dt;
            sp.strike = (sp.strike - dt * 3.0).max(0.0);
            if sp.cooldown > 0.0 {
                continue;
            }
            let at = sp.pos;
            let reach = path::SPIRIT_REACH;
            let foe = self
                .enemies
                .iter()
                .enumerate()
                .filter(|(_, e)| e.is_dangerous() || e.is_stunned())
                .map(|(j, e)| (dist_sq(e.pos(), at), j))
                .filter(|&(d, _)| d <= reach * reach)
                .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            if let Some((_, j)) = foe {
                let p = self.enemies[j].pos();
                self.view.zap(&self.level.grid, at, p);
                self.slay_enemy(j);
            } else if let Some(bp) = self.boss_target().filter(|bp| dist_sq(*bp, at) <= (reach + 1.0).powi(2)) {
                self.view.zap(&self.level.grid, at, bp);
                self.wound_boss(1, bp);
            } else {
                continue;
            }
            self.spirits[i].cooldown = every;
            self.spirits[i].strike = 1.0;
        }
    }

    /// A stunning blow or a shield's clash: every foe within `radius` reels, harmless, for `seconds`.
    fn stun_around(&mut self, at: V, radius: f64, seconds: f64) {
        let grid = &self.level.grid;
        let mut any = false;
        for e in self.enemies.iter_mut() {
            if (e.is_dangerous() || e.is_stunned()) && dist_sq(e.pos(), at) <= radius * radius {
                e.scare(grid, seconds);
                any = true;
            }
        }
        if any {
            self.view.shockwave(grid, at.x, at.y);
            self.audio.play(Sfx::Zap);
        }
    }

    /// Loki's trick: the foes around him lose their way, and now and then a decoy of him draws them off.
    fn loki_trick(&mut self, at: V) {
        for e in self.enemies.iter_mut() {
            if dist_sq(e.pos(), at) <= progression::TRICK_RADIUS * progression::TRICK_RADIUS {
                e.confuse(CONFUSE_SECONDS);
            }
        }
        if self.decoy_cooldown <= 0.0 {
            self.decoy = Some((at, DECOY_SECONDS));
            self.decoy_cooldown = DECOY_EVERY;
            self.view.sparkle(&self.level.grid, at.x, at.y, 0.8, 0x9a4aff, 30);
            self.floater(at.x, at.y - 0.8, "DECOY", 1.2, Some(0xc89aff));
        }
    }

    fn update_decoy(&mut self, dt: f64) {
        self.decoy_cooldown = (self.decoy_cooldown - dt).max(0.0);
        if let Some((p, t)) = &mut self.decoy {
            *t -= dt;
            if *t <= 0.0 {
                let at = *p;
                self.view.sparkle(&self.level.grid, at.x, at.y, 0.6, 0x9a4aff, 16);
                self.decoy = None;
            }
        }
    }

    /**
     * Chain lightning (a tier-10 weapon) from where it struck: it leaps to
     * the nearest foes in turn, up to `count`, and to the Cyclops if it is near.
     */
    fn chain_lightning(&mut self, from: V, count: usize) {
        let mut at = from;
        let mut struck = 0;
        let mut done: Vec<usize> = Vec::new();
        while struck < count {
            let next = self
                .enemies
                .iter()
                .enumerate()
                .filter(|(i, e)| e.is_targetable() && !done.contains(i))
                .map(|(i, e)| (dist_sq(e.pos(), at), i))
                .filter(|&(d, _)| d <= ZAP_RANGE * ZAP_RANGE)
                .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            let Some((_, i)) = next else { break };
            let p = self.enemies[i].pos();
            self.view.zap(&self.level.grid, at, p);
            self.slay_enemy(i);
            done.push(i);
            at = p;
            struck += 1;
        }
        if let Some(bp) = self.boss_target() {
            if dist_sq(bp, from) <= (ZAP_RANGE + 1.0).powi(2) {
                self.view.zap(&self.level.grid, from, bp);
                self.strike_boss(ZAP_DAMAGE, bp);
                struck += 1;
            }
        }
        if struck > 0 {
            self.audio.play(Sfx::Zap);
            self.view.shake(0.2);
        }
    }

    /// Bifrost Step (path 5): a flash of spectral light carries him a few tiles ahead.
    fn bifrost_step(&mut self) {
        if self.hero.path < 5 {
            return;
        }
        let Some((from, to)) = self.hero.try_step(&self.level.grid) else { return };
        let grid = &self.level.grid;
        // A flash of Bifrost's light, a cloud of cherry blossom, a gust of desert sand.
        let (trail, burst) = match self.hero_kind {
            HeroKind::Ragnar => (0x9af0ff, 0xfff1ff),
            HeroKind::Miyamoto => (0xffb0d0, 0xffe0f0),
            HeroKind::Mehmed => (0xe8d09a, 0xfff0c8),
        };
        self.audio.play(Sfx::Whoosh);
        self.view.zap(grid, from, to);
        self.view.sparkle(grid, from.x, from.y, 0.6, trail, 24);
        self.view.sparkle(grid, to.x, to.y, 0.8, burst, 30);
        if self.hero_kind != HeroKind::Ragnar {
            self.view.dust(grid, from.x, from.y, trail, 14, 2.2);
        }
    }

    /**
     * The path's safety nets, watched every tick: the World Tree (path 10)
     * restores him once per dungeon when he falls to a quarter of his health.
     */
    fn watch_health(&mut self) {
        if self.hero.path >= 10 && !self.restored && self.hero.alive && (self.hero.hp as f64) <= self.hero.max_hp as f64 * path::TREE_AT {
            self.restored = true;
            self.hero.restore();
            let hp = self.hero.pos();
            let n = progression::node(self.hero_kind, Tree::Path, 10);
            self.audio.play(Sfx::Heal);
            self.view.sparkle(&self.level.grid, hp.x, hp.y, 1.0, 0x6ae86a, 50);
            self.juice.hp_flash = 1.0;
            self.juice.hp_flash_color = 0x5cff6a;
            self.juice.banner = Some(Banner { text: n.ability.to_uppercase(), sub: "FULLY RESTORED".into(), color: 0x6ae86a, life: 2.0, max: 2.0 });
        }
    }

    /// Stepping onto the hidden shrine takes its relic and a rune fragment.
    fn visit_relic_shrine(&mut self) {
        let Some(t) = self.relic_shrine else { return };
        if self.hero.airborne() || self.hero.occupied() != t {
            return;
        }
        self.relic_shrine = None;
        self.progress.runes += RUNES_SHRINE;
        self.audio.play(Sfx::Milestone);
        self.view.sparkle(&self.level.grid, t.x as f64, t.y as f64, 1.0, 0x9ad1ff, 50);
        self.floater(t.x as f64, t.y as f64 + 0.6, format!("+{RUNES_SHRINE} {}", progression::gems(self.hero_kind, RUNES_SHRINE)), 1.8, Some(0x9ad1ff));
        self.take_relic(t.v());
        self.save_progress();
    }

    /**
     * A relic of the old heroes, from a hidden shrine or a fallen Cyclops: it
     * unseals the next sealed tier of the tree that has the most still sealed.
     */
    fn take_relic(&mut self, at: V) {
        let hero = self.hero_kind;
        let Some(tree) = self.progress.relic_tree(hero) else {
            // Every tree is open: the relic is worth its weight in silver.
            self.earn(300);
            self.floater(at.x, at.y - 1.4, format!("+300 {}", progression::coin(self.hero_kind)), 1.8, Some(0xffd84a));
            return;
        };
        let Some(tier) = self.progress.find_relic(hero, tree) else { return };
        let n = progression::node(hero, tree, tier);
        let now = self.progress.hero(hero).has(tree, tier);
        self.equip_from_progress();
        self.juice.banner = Some(Banner {
            text: format!("RELIC: {}", n.name.to_uppercase()),
            sub: if now {
                format!("{} {} OF {} IS YOURS", tree.step(), tier, tree.title(hero).to_uppercase())
            } else {
                format!("{} {} OF {} UNSEALED   IT IS YOURS ONCE THE TIERS BEFORE IT ARE", tree.step(), tier, tree.title(hero).to_uppercase())
            },
            color: n.color,
            life: 3.2,
            max: 3.2,
        });
        self.view.sparkle(&self.level.grid, at.x, at.y, 1.2, n.color, 40);
    }

    /**
     * The gatekeeper's promise: wherever he is, the hero can call for help and
     * the old man's wares open (the dungeon waits).
     */
    pub fn call_old_man(&mut self) {
        self.audio.play(Sfx::Whoosh);
        self.shop = ShopState { called: true, ..ShopState::default() };
        self.enter(Screen::Shop);
        self.juice.fade = 1.0;
        self.input.consume(Action::Attack);
        self.input.consume(Action::Confirm);
    }

    /// Adds points, doubled while a BRUTAL streak's reward runs. Returns what was added.
    fn award(&mut self, points: u32) -> u32 {
        let points = if self.double_points > 0.0 { points * 2 } else { points };
        self.score.add(points);
        points
    }

    /**
     * A third kill in quick succession: BRUTAL. The screen jolts and flashes,
     * the world holds for a beat, and every point is doubled for a while.
     */
    fn brutal(&mut self) {
        self.double_points = DOUBLE_POINTS_SECONDS;
        self.view.shake(0.85);
        self.hitstop = 0.14;
        self.juice.brutal = 1.0;
        self.audio.play(Sfx::Brutal);
        self.juice.banner = Some(Banner {
            text: "BRUTAL!".into(),
            sub: format!("x{} KILL STREAK   DOUBLE POINTS FOR {} SECONDS", self.kill_streak, DOUBLE_POINTS_SECONDS as u32),
            color: 0xff3a2a,
            life: 2.2,
            max: 2.2,
        });
        let hp = self.hero.pos();
        self.view.sparkle(&self.level.grid, hp.x, hp.y, 0.8, 0xff5a3a, 30);
    }

    /// Silver in the purse, and the counter jumps.
    fn earn(&mut self, coins: u32) {
        self.progress.silver += coins;
        self.juice.coin_punch = 1.0;
    }

    /// Drinks the potion in belt slot `slot`, if there is one.
    fn drink(&mut self, slot: usize) {
        if slot >= self.belt.len() || !self.hero.alive {
            return;
        }
        let kind = self.belt.remove(slot);
        if self.belt_index >= self.belt.len() {
            self.belt_index = self.belt.len().saturating_sub(1);
        }
        self.quaff(kind);
    }

    /// A potion's effect, from the belt or picked up.
    fn quaff(&mut self, kind: PotionKind) {
        let def = gear::potion(self.hero_kind, kind);
        let hp = self.hero.pos();
        let mut label = format!("{}!", def.name.to_uppercase());
        if kind == PotionKind::Life {
            if !self.score.gain_life() {
                self.award(500);
                label = "FULL LIVES +500".into();
            }
        } else {
            let before = self.hero.hp;
            self.hero.drink(kind);
            if kind == PotionKind::Health {
                label = format!("+{} HP", self.hero.hp - before);
                self.juice.hp_flash = 1.0;
                self.juice.hp_flash_color = 0x5cff6a;
            }
        }
        self.audio.play(Sfx::Drink);
        self.view.sparkle(&self.level.grid, hp.x, hp.y, 0.6, def.color, 24);
        self.floater(hp.x, hp.y, label, 1.3, Some(def.color));
    }

    fn slay_enemy(&mut self, i: usize) {
        let base = if self.enemies[i].is_stunned() { self.frenzy.consume_value() } else { POINTS_KILL };
        self.earn(COINS_KILL);
        self.enemies[i].kill();
        let at = self.enemies[i].pos();
        if self.hero.path >= 8 && self.hero_kind == HeroKind::Ragnar {
            // Terrify: the lesser foes around the kill flee in blind panic.
            let grid = &self.level.grid;
            for (k, e) in self.enemies.iter_mut().enumerate() {
                if k != i && e.is_dangerous() && dist_sq(e.pos(), at) <= path::TERROR_RADIUS * path::TERROR_RADIUS {
                    e.scare(grid, path::TERROR_SECONDS);
                }
            }
        }
        if self.rng.next() < RUNE_DROP {
            self.progress.runes += 1;
            self.audio.play(Sfx::Milestone);
            self.view.sparkle(&self.level.grid, at.x, at.y, 0.8, 0x9ad1ff, 20);
            self.floater(at.x, at.y - 1.2, progression::gems(self.hero_kind, 1), 1.6, Some(0x9ad1ff));
        }
        self.kill_streak = if self.streak_timer > 0.0 { self.kill_streak + 1 } else { 1 };
        self.streak_timer = STREAK_SECONDS;
        self.audio.play_kill(self.kill_streak - 1);
        if self.kill_streak % BRUTAL_STREAK == 0 {
            self.brutal();
        }
        let value = self.award(base);
        let p = self.enemies[i].pos();
        let color = self.enemies[i].b.color;
        self.view.spark(&self.level.grid, p.x, p.y, 0xffb0b0);
        self.view.dust(&self.level.grid, p.x, p.y, color, 14, 2.4);
        self.view.sparkle(&self.level.grid, p.x, p.y, 0.5, 0xfff1c0, 8);
        self.view.shake(0.1);
        self.hitstop = HITSTOP_KILL;
        let label = if self.double_points > 0.0 { format!("{value} x2") } else { value.to_string() };
        self.floater(p.x, p.y, label, 1.0, if self.double_points > 0.0 { Some(0xff5a3a) } else { None });
        if self.kill_streak >= 2 {
            let text = format!("x{} STREAK", self.kill_streak);
            self.floater(p.x, p.y - 0.6, text, 1.2, Some(0xff8a2a));
        }
    }

    /// Fires the held ranged weapon in the facing direction; the last shot hands the sword back.
    fn fire(&mut self) {
        let Some(kind) = self.hero.try_fire() else { return };
        let shots = spawn_shots(kind, self.hero.pos(), self.hero.aim());
        self.projectiles.extend(shots);
        self.audio.play(Sfx::Arrow);
        if let crate::logic::items::RangedKind::Hurled(h, t) = kind {
            if progression::node(h, Tree::Armory, t).form == progression::Form::Gun {
                // A crack and a puff of powder smoke.
                let hp = self.hero.pos();
                self.audio.play(Sfx::ArrowHit);
                self.view.dust(&self.level.grid, hp.x, hp.y, 0xb8b8c0, 12, 1.8);
                self.view.shake(0.12);
            }
        }
        if !self.hero.holding_pickup() && self.hero.weapon != crate::logic::items::WeaponKind::Ranged(kind) {
            // The shrine weapon ran dry; his own is back in hand.
            let hp = self.hero.pos();
            let (name, color) = crate::logic::items::weapon_label(self.hero_kind, self.hero.weapon, self.hero.blade);
            self.floater(hp.x, hp.y, name.to_uppercase(), 1.2, Some(color));
        }
    }

    fn update_projectiles(&mut self, dt: f64) {
        let hero_pos = self.hero.pos();
        // What seeking knives can chase.
        let mut prey: Vec<V> = self.enemies.iter().filter(|e| e.is_targetable()).map(|e| e.pos()).collect();
        prey.extend(self.boss_target());
        let mut i = 0;
        while i < self.projectiles.len() {
            let state = self.projectiles[i].update(dt, &self.level.grid, hero_pos, &prey);
            if self.projectiles[i].alive {
                let (pos, radius) = (self.projectiles[i].pos, self.projectiles[i].def.hit_radius);
                let hits = find_projectile_hits(pos, radius, &self.enemies, &self.projectiles[i].struck);
                let mut hit_at = hits.first().map(|_| pos);
                let grace = self.projectiles[i].kind == crate::logic::items::RangedKind::Grace;
                for e in hits {
                    if grace {
                        self.turn_foe(e);
                    } else {
                        self.slay_enemy(e);
                    }
                    self.projectiles[i].struck.insert(e);
                    self.audio.play(Sfx::ArrowHit);
                    if !self.projectiles[i].def.pierce {
                        self.projectiles[i].alive = false;
                        break;
                    }
                }
                // The boss counts as one more target, struck once per shot.
                if let Some(bp) = self.boss_target() {
                    let p = &self.projectiles[i];
                    if p.alive && !p.struck.contains(&usize::MAX) && dist_sq(p.pos, bp).sqrt() <= radius + BODY_RADIUS {
                        let base = p.def.damage;
                        let at = p.pos;
                        self.projectiles[i].struck.insert(usize::MAX);
                        if !self.projectiles[i].def.pierce {
                            self.projectiles[i].alive = false;
                        }
                        if grace {
                            self.smite_boss(base, at);
                        } else {
                            self.strike_boss(base, at);
                        }
                        hit_at = Some(at);
                        if let crate::logic::items::RangedKind::Hurled(h, t) = self.projectiles[i].kind {
                            if progression::blade(h, t).throw.map(|t| t.stagger).unwrap_or(false) {
                                let staggered = self.boss.as_mut().and_then(|b| b.stagger().then_some(b.pos));
                                if let Some(p) = staggered {
                                    self.floater(p.x, p.y - 0.9, "STAGGERED", 1.0, Some(0xe0c070));
                                }
                            }
                        }
                    }
                }
                // Mjolnir: its first strike of each throw calls down chain lightning.
                let chain = match self.projectiles[i].kind {
                    crate::logic::items::RangedKind::Hurled(h, t) => progression::blade(h, t).chain,
                    _ => 0,
                };
                if chain > 0 && !self.projectiles[i].struck.contains(&(usize::MAX - 1)) {
                    if let Some(at) = hit_at {
                        self.projectiles[i].struck.insert(usize::MAX - 1);
                        self.chain_lightning(at, chain);
                    }
                }
            } else if state == ShotState::Wall {
                let p = self.projectiles[i].pos;
                self.view.spark(&self.level.grid, p.x, p.y, 0xfff1c0);
                self.view.dust(&self.level.grid, p.x, p.y, 0x4a3620, 6, 1.6);
                self.view.shake(0.03);
            } else if state == ShotState::Caught {
                self.hero.refund_shot();
                self.audio.play(Sfx::Nodule);
            }
            if !self.projectiles[i].alive {
                self.projectiles.remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// Drops a bomb on the hero's current tile (B, Shift, right click), one per tile, from a stock of three.
    fn place_bomb(&mut self) {
        if self.bombs_left == 0 || !self.hero.alive || self.hero.airborne() {
            return;
        }
        let at = self.hero.occupied();
        if self.bombs.iter().any(|b| b.tile == at) {
            return;
        }
        self.bombs.push(Bomb::dropped(at, if self.hero.free_move { self.hero.pos() } else { at.v() }));
        self.bombs_left -= 1;
        self.audio.play(Sfx::Fuse);
    }

    fn update_bombs(&mut self, dt: f64) {
        // Each bomb burns once per tick, in order; a blast only shortens other fuses.
        let mut idx = 0;
        for _ in 0..self.bombs.len() {
            if idx >= self.bombs.len() {
                break;
            }
            if self.bombs[idx].update(dt) {
                let bomb = self.bombs.remove(idx);
                self.explode(bomb);
            } else {
                idx += 1;
            }
        }
    }

    /// TNT-style blast: breaks nearby walls, hurts everyone in range, chains to other bombs.
    fn explode(&mut self, bomb: Bomb) {
        let c = bomb.tile;
        let broken = break_rocks(&mut self.level.grid, c, BREAK_RADIUS);
        let floor = self.level.grid.elevation(c.x, c.y);
        for t in &broken {
            // A blasted wall becomes floor at the bomb's level.
            self.level.grid.set_elevation(t.x, t.y, floor);
            self.view.remove_rock(&self.level.grid, t.x, t.y);
        }
        self.view.explode(&self.level.grid, c.x as f64, c.y as f64, broken.len());
        self.view.dust(&self.level.grid, c.x as f64, c.y as f64, 0x4a3620, 30, 4.0);
        // The closer the blast, the harder the kick.
        let d = dist_sq(c.v(), self.hero.pos()).sqrt() as f32;
        self.view.shake(0.75 * (1.0 - d / 12.0).max(0.15));
        self.audio.play(Sfx::Explode);

        for i in 0..self.enemies.len() {
            if self.enemies[i].is_targetable() && in_blast(c, self.enemies[i].pos()) {
                self.slay_enemy(i);
            }
        }
        if let Some(bp) = self.boss_target() {
            if in_blast(c, bp) {
                self.wound_boss(BOMB_DAMAGE, bp);
            }
        }
        for other in self.bombs.iter_mut() {
            if in_blast(c, other.tile.v()) {
                other.ignite(self.rng.range(CHAIN_DELAY_MIN, CHAIN_DELAY_MAX));
            }
        }
        let dmg = blast_damage(dist_sq(c.v(), self.hero.pos()).sqrt());
        if dmg > 0 {
            self.hurt_hero(dmg);
        }
    }

    /// Enemy contact damages the hero; a dead hero starts the dying sequence.
    fn resolve_contact(&mut self) {
        if self.hero.invisible() || self.hero.airborne() {
            return; // they cannot find what they cannot see, or reach what sails overhead
        }
        let hp = self.hero.pos();
        let ho = self.hero.occupied();
        for i in 0..self.enemies.len() {
            let e = &self.enemies[i];
            if !e.is_dangerous() {
                continue;
            }
            let same_tile = e.occupied() == ho;
            if !same_tile && dist_sq(e.pos(), hp) > COLLISION_RADIUS_SQ {
                continue;
            }
            if self.hero.fx.fire > 0.0 {
                // Dragonfire: whatever lays a claw on him burns to ash.
                let p = self.enemies[i].pos();
                self.view.sparkle(&self.level.grid, p.x, p.y, 0.8, 0xff7a2a, 24);
                self.slay_enemy(i);
                continue;
            }
            let died = self.hurt_hero(CONTACT_DAMAGE);
            if self.hero.blocked_hit {
                self.hero.blocked_hit = false;
                let ward = self.hero.shield.map(|s| progression::ward(s.tier));
                if let Some(w) = ward {
                    if w.bash && self.enemies[i].is_dangerous() {
                        // The rim strikes back: the foe whose blow it caught goes down.
                        self.slay_enemy(i);
                    }
                    if w.stagger {
                        self.stun_around(hp, STAGGER_RADIUS, STAGGER_SECONDS);
                    }
                }
            }
            if died {
                return;
            }
        }
    }

    /// Applies damage with shield and invincibility rules. Returns true if the hero died.
    fn hurt_hero(&mut self, amount: i32) -> bool {
        let result = if amount > 0 { self.hero.take_damage(amount) } else { DamageResult::Immune };
        let hp = self.hero.pos();
        if self.hero.shield_broke {
            self.hero.shield_broke = false;
            self.audio.play(Sfx::ShieldBreak);
            self.view.dust(&self.level.grid, hp.x, hp.y, 0x8a6a40, 16, 3.0);
            self.view.shake(0.3);
            self.floater(hp.x, hp.y - 0.6, "SHIELD BROKEN", 1.4, Some(0xff8a6a));
        }
        match result {
            DamageResult::Blocked => {
                self.audio.play(Sfx::Nodule);
                self.view.spark(&self.level.grid, hp.x, hp.y, 0x7fd7ff);
                self.view.sparkle(&self.level.grid, hp.x, hp.y, 0.5, 0x7fd7ff, 12);
                self.view.shake(0.15);
                self.floater(hp.x, hp.y, "BLOCKED", 0.8, Some(0x7fd7ff));
                false
            }
            DamageResult::Immune => false,
            DamageResult::Damaged => {
                self.audio.play(Sfx::Hurt);
                self.view.spark(&self.level.grid, hp.x, hp.y, 0xd8262e);
                self.view.dust(&self.level.grid, hp.x, hp.y, 0xd8262e, 10, 2.2);
                self.view.shake(0.42);
                self.juice.hurt = 1.0;
                self.juice.hp_flash = 1.0;
                self.juice.hp_flash_color = 0xff4d4d;
                if !self.hero.alive && self.hero.path >= 8 && self.hero_kind != HeroKind::Ragnar && !self.stood {
                    // The last stand (Seppuku Resolve, Blood of the Empire): death is held off, and he strikes triple.
                    self.stood = true;
                    self.hero.alive = true;
                    self.hero.hp = 1;
                    self.hero.last_stand = path::STAND_SECONDS;
                    self.hero.invuln = 0.5;
                    let n = progression::node(self.hero_kind, Tree::Path, 8);
                    self.audio.play(Sfx::Brutal);
                    self.view.sparkle(&self.level.grid, hp.x, hp.y, 1.2, 0xc8202a, 50);
                    self.view.shake(0.6);
                    self.juice.banner = Some(Banner { text: n.ability.to_uppercase(), sub: "YOU CANNOT DIE   TRIPLE DAMAGE".into(), color: 0xff4a3a, life: 2.2, max: 2.2 });
                    return false;
                }
                if !self.hero.alive && self.hero.path >= 9 && self.hero_kind == HeroKind::Ragnar && !self.revived {
                    // Valkyrie Call: once in each dungeon, a fatal blow is not the end.
                    self.revived = true;
                    self.hero.alive = true;
                    self.hero.hp = self.hero.max_hp / 2;
                    self.hero.invuln = 2.0;
                    let n = progression::node(self.hero_kind, Tree::Path, 9);
                    self.audio.play(Sfx::Heal);
                    self.view.sparkle(&self.level.grid, hp.x, hp.y, 1.4, 0xfff1a8, 60);
                    self.juice.banner = Some(Banner { text: n.ability.to_uppercase(), sub: "RAISED AGAIN WITH HALF YOUR HEALTH".into(), color: 0xfff1a8, life: 2.2, max: 2.2 });
                    return false;
                }
                if !self.hero.alive {
                    self.kill_hero();
                    return true;
                }
                false
            }
        }
    }

    fn kill_hero(&mut self) {
        self.audio.play(Sfx::Death);
        self.audio.set_frenzy(false);
        self.audio.set_threat(0.0);
        self.frenzy.cancel();
        let hp = self.hero.pos();
        self.view.explode(&self.level.grid, hp.x, hp.y, 0);
        self.view.shake(0.8);
        self.enter(Screen::Dying);
    }

    fn update_effects(&mut self, dt: f64) {
        self.floaters.retain_mut(|f| {
            f.life -= dt;
            f.y -= dt * 0.8;
            f.life > 0.0
        });
        let hp = self.hero.pos();
        let nearest = self.enemies.iter().filter(|e| e.is_dangerous()).map(|e| manhattan(e.pos(), hp)).fold(f64::INFINITY, f64::min);
        let mut threat = if nearest.is_infinite() { 0.0 } else { (1.0 - nearest / 10.0).max(0.0) };
        if let Some(b) = self.boss.as_ref().filter(|b| b.active()) {
            // A boss fight is never calm; the closer the eye, the harder the drums.
            threat = 0.55 + 0.45 * (1.0 - dist_sq(b.pos, hp).sqrt() / 10.0).max(0.0);
        }
        self.audio.set_threat(threat);
        let frac = self.hero.hp as f64 / self.hero.max_hp as f64;
        self.audio.set_danger(if frac < 0.35 { 1.0 - frac / 0.35 } else { 0.0 });
        self.streak_timer -= dt;
        self.double_points = (self.double_points - dt).max(0.0);
    }

    /// Footsteps while running, a thud and a puff of dust on landing.
    fn footsteps(&mut self, before: crate::logic::types::V) {
        let airborne = self.hero.airborne();
        if self.was_airborne && !airborne {
            let hp = self.hero.pos();
            self.audio.play(Sfx::Land);
            self.view.dust(&self.level.grid, hp.x, hp.y, 0x4a3620, 12, 1.8);
            self.view.shake(0.07);
            self.step_dist = 0.0;
        }
        self.was_airborne = airborne;
        if airborne || !self.hero.alive {
            return;
        }
        let after = self.hero.pos();
        let moved = dist_sq(before, after).sqrt();
        if moved <= 0.0 || moved > 1.5 {
            return; // standing, or wrapping through a tunnel
        }
        self.step_dist += moved;
        if self.step_dist >= STRIDE_TILES {
            self.step_dist -= STRIDE_TILES;
            let speed = if self.hero.m.speed > 0.0 { (self.hero.vel / self.hero.m.speed).clamp(0.3, 1.0) } else { 0.5 };
            let slimy = self.hero.slowed();
            self.audio.play_step(speed, slimy);
            let color = if slimy { 0x5cff6a } else { 0x4a3620 };
            self.view.dust(&self.level.grid, after.x, after.y, color, 2, 0.7);
        }
    }

    /// Rewards and feedback that live only on screen: rolling score, flying coins, flashes.
    fn update_juice(&mut self, dt: f64) {
        let dtf = dt as f32;
        let j = &mut self.juice;
        let target = self.score.score as f64;
        if j.score_shown < target {
            j.score_shown = (j.score_shown + (target - j.score_shown) * (dt * 9.0).min(1.0) + dt * 40.0).min(target);
        } else {
            j.score_shown = target;
        }
        let hp = self.hero.hp as f32;
        if j.hp_trail > hp {
            j.hp_trail = (j.hp_trail - dtf * 60.0).max(hp);
        } else {
            j.hp_trail = hp;
        }
        j.key_punch = (j.key_punch - dtf * 3.5).max(0.0);
        j.hp_flash = (j.hp_flash - dtf * 3.0).max(0.0);
        j.hurt = (j.hurt - dtf * 2.2).max(0.0);
        j.go = (j.go - dtf).max(0.0);
        j.fade = (j.fade - dtf * 1.6).max(0.0);
        j.coin_punch = (j.coin_punch - dtf * 3.0).max(0.0);
        j.brutal = (j.brutal - dtf * 1.8).max(0.0);
        let boss_hp = self.boss.as_ref().map(|b| b.hp as f32).unwrap_or(0.0);
        if j.boss_trail > boss_hp {
            j.boss_trail = (j.boss_trail - dtf * 12.0).max(boss_hp);
        } else {
            j.boss_trail = boss_hp;
        }
        let mut landed = 0;
        j.coins.retain_mut(|c| {
            c.t += dtf / COIN_FLIGHT_SECONDS;
            if c.t >= 1.0 {
                landed += 1;
                false
            } else {
                true
            }
        });
        if landed > 0 {
            j.key_punch = 1.0;
        }
        if let Some(b) = &mut j.banner {
            b.life -= dtf;
            if b.life <= 0.0 {
                j.banner = None;
            }
        }
    }

    /// Test and capture hook: drop a nugget under Ragnar and pick it up.
    pub fn debug_collect_key(&mut self) {
        let o = self.hero.occupied();
        self.level.grid.set(o.x, o.y, Tile::Nodule);
        self.collect();
    }

    /// Test and capture hook: `amount` damage straight to the boss.
    pub fn debug_wound_boss(&mut self, amount: i32) {
        if let Some(b) = self.boss.as_ref().filter(|b| b.active()) {
            let at = b.pos;
            self.wound_boss(amount, at);
        }
    }

    /// Capture hook: three quick kills, for a BRUTAL streak.
    pub fn debug_streak(&mut self) {
        for i in 0..self.enemies.len().min(3) {
            self.slay_enemy(i);
        }
    }

    /// Capture hook: own everything up to `tier` in every tree, and take tier `tier` weapon and shield.
    pub fn debug_arm(&mut self, tier: u8) {
        let tier = tier.clamp(1, progression::TIERS);
        let h = self.progress.hero_mut(self.hero_kind);
        h.owned = [tier; 4];
        h.wield = tier;
        h.carry = tier;
        self.hero.shield = Some(ShieldGear::new(tier));
        self.equip_from_progress();
    }

    /// Capture hook: drink a potion without owning it.
    pub fn debug_drink(&mut self, kind: PotionKind) {
        self.quaff(kind);
    }

    /// Capture hook: knock the boss down as if a dive had missed.
    pub fn debug_daze_boss(&mut self) {
        if let Some(b) = self.boss.as_mut().filter(|b| b.active()) {
            b.state = crate::logic::boss::BossState::Dazed(0.0);
            b.hover = 0.05;
        }
    }

    /// Test and capture hook: an enemy's touch.
    /// Capture hook: straight into dungeon `index` (0 is the first) with a fresh kit, after a countdown.
    pub fn debug_dungeon(&mut self, index: usize) {
        self.score.reset();
        self.new_kit();
        self.load_level(index);
        self.enter(Screen::Ready);
    }

    /// Capture hook: `rows` rows of one of each foe standing a few tiles before the hero, facing him.
    pub fn debug_foes(&mut self, rows: usize) {
        let specs = default_enemy_specs(&self.level.grid);
        let p = self.hero.occupied();
        self.enemies.clear();
        self.view.clear_enemies();
        for (i, kind) in [EnemyKind::Skeleton, EnemyKind::Bat, EnemyKind::Vampire, EnemyKind::Slime].repeat(rows.max(1)).into_iter().enumerate() {
            let Some(spec) = specs.iter().find(|s| s.kind == kind) else { continue };
            let (col, row) = ((i % 4) as i32, (i / 4) as i32);
            let mut e = Enemy::new(spec, P::new(p.x - 3 + col * 2, p.y - 2 - row * 2), self.level.data.enemy_speed);
            e.b.mode = EnemyMode::Chase;
            e.m.dir = Dir::Down;
            self.view.add_enemy(&e);
            self.enemies.push(e);
        }
    }

    /// A fresh slime trail along the tiles around Ragnar, for screenshots.
    pub fn debug_goo(&mut self) {
        let p = self.hero.occupied();
        let g = &self.level.grid;
        let mut tiles: Vec<P> = vec![];
        for (dx, dy) in [(0, 0), (1, 0), (2, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1)] {
            let t = P::new(p.x + dx, p.y + dy);
            if g.is_walkable(t.x, t.y, false) {
                tiles.push(t);
            }
        }
        for (i, t) in tiles.into_iter().enumerate() {
            self.puddles.add(t, PUDDLE_SECONDS * (1.0 - i as f64 * 0.08));
        }
    }

    pub fn debug_hurt(&mut self) {
        self.hero.invuln = 0.0;
        self.hurt_hero(CONTACT_DAMAGE);
    }

    /// Nuggets shown in the HUD: the ones still flying there are not counted yet.
    pub fn keys_shown(&self) -> u32 {
        self.keys_got.saturating_sub(self.juice.coins.len() as u32)
    }
}

/**
 * The open floor tile nearest the dungeon's center (never a shrine, bottle,
 * ramp or the hollow, nor the middle row or column, where the fountain's
 * channels run), and the direction with the longest clear run from it.
 */
pub fn center_spawn(level: &ParsedLevel) -> (P, Dir) {
    let g = &level.grid;
    let (cx, cy) = ((g.width - 1) as f64 / 2.0, (g.height - 1) as f64 / 2.0);
    let taken: Vec<P> = level
        .angel_shrines
        .iter()
        .chain(level.item_shrines.iter())
        .copied()
        .chain(level.potions.iter().map(|(p, _)| *p))
        .collect();
    let mut best = level.hero_spawn;
    let mut best_d = f64::INFINITY;
    for (x, y, t) in g.cells() {
        let p = P::new(x, y);
        if t != Tile::Open || g.is_ramp(x, y) || taken.contains(&p) || y == (g.height - 1) / 2 || x == (g.width - 1) / 2 {
            continue;
        }
        let d = (x as f64 - cx).powi(2) + (y as f64 - cy).powi(2);
        if d < best_d {
            best_d = d;
            best = p;
        }
    }
    let run = |d: Dir| {
        let mut n = 0;
        let mut p = best;
        while n < 12 && g.can_move(p, d, false, false) {
            p = g.neighbor(p, d);
            n += 1;
        }
        n
    };
    let facing = [Dir::Down, Dir::Up, Dir::Left, Dir::Right].into_iter().max_by_key(|&d| (run(d), d == Dir::Down)).unwrap();
    (best, facing)
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Play on the title, then begin with the hero on the character screen (Ragnar).
    fn start_from_title(g: &mut Game) {
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Select);
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
    }

    fn playing() -> Game {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        let mut g = Game::new();
        start_from_title(&mut g);
        for _ in 0..(READY_SECONDS / crate::app::STEP) as usize + 2 {
            g.update(crate::app::STEP);
        }
        assert_eq!(g.screen, Screen::Playing);
        g
    }

    #[test]
    fn the_character_screen_offers_three_heroes_and_each_can_begin() {
        let mut g = Game::new();
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Select);
        assert_eq!(g.select_hero(), HeroKind::Ragnar, "Ragnar is highlighted first");
        // Left of Ragnar stands Mehmed, right Miyamoto; each of the three can begin.
        g.input.nav = Some(Dir::Left);
        g.update(crate::app::STEP);
        assert_eq!(g.select_hero(), HeroKind::Mehmed);
        for hero in crate::logic::heroes::ROSTER {
            g.enter(Screen::Title);
            g.open_select();
            g.select_index = hero.slot();
            g.input.inject(Action::Confirm);
            g.update(crate::app::STEP);
            assert_eq!(g.screen, Screen::Ready, "{hero:?} begins");
            assert_eq!(g.hero_kind, hero);
            assert_eq!(g.hero.kind, hero, "and plays with his own armory");
        }
        g.enter(Screen::Title);
        g.open_select();
        // Esc goes back to the title.
        g.input.inject(Action::Pause);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Title);
    }

    #[test]
    fn play_opens_scene_one_which_plays_every_beat_then_the_heroes() {
        use crate::logic::story::SCENE_ONE;
        let mut g = Game::new();
        g.intro = true;
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Story, "Play starts the story");
        let mut seen: Vec<usize> = Vec::new();
        for _ in 0..(300.0 / crate::app::STEP) as usize {
            if g.screen != Screen::Story {
                break;
            }
            let st = g.story.as_ref().unwrap();
            if seen.last() != Some(&st.beat) {
                seen.push(st.beat);
            }
            // Wait out the action; press on through the lines.
            if st.beat().waits() && st.t > 0.5 {
                g.input.inject(Action::Confirm);
            }
            g.update(crate::app::STEP);
        }
        assert_eq!(g.screen, Screen::Select, "the heroes are chosen after the story");
        assert_eq!(seen, (0..SCENE_ONE.len()).collect::<Vec<_>>(), "every beat plays, in order");
        assert!(g.story.is_none());
    }

    #[test]
    fn a_press_shows_the_whole_line_before_moving_on_and_esc_skips_the_story() {
        use crate::logic::story::{first, Shot};
        let mut g = Game::new();
        g.story_jump(first(Shot::Souls).unwrap(), 0.0);
        let st = g.story.as_mut().unwrap();
        st.shown = 0.0;
        for _ in 0..40 {
            g.update(crate::app::STEP);
        }
        let st = g.story.as_ref().unwrap();
        assert!(st.shown > 0.0 && !st.line_done(), "the line types itself out");
        let beat = st.beat;
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        let st = g.story.as_ref().unwrap();
        assert!(st.line_done() && st.beat == beat, "the first press shows the rest of the line");
        assert_eq!(st.souls, 0b111, "all three warriors' souls rise once they are named");
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        assert_eq!(g.story.as_ref().unwrap().beat, beat + 1, "the next press moves on");
        // Esc skips what is left, straight to the character screen.
        g.input.inject(Action::Pause);
        g.update(crate::app::STEP);
        for _ in 0..(1.0 / crate::app::STEP) as usize {
            g.update(crate::app::STEP);
        }
        assert_eq!(g.screen, Screen::Select);
    }

    #[test]
    fn every_key_enrages_the_dungeon() {
        let mut g = playing();
        assert_eq!(g.rage(), 0.0);
        let (calm_wait, calm_cap) = (g.spawn_every(), g.max_enemies());
        let before = g.enemies.len();
        // Take a key: a reinforcement is on its way within moments.
        let o = g.hero.occupied();
        g.level.grid.set(o.x, o.y, Tile::Nodule);
        g.collect();
        assert!(g.rage() > 0.0);
        assert!(g.spawn_timer <= 1.5, "someone comes at once");
        for _ in 0..(2.0 / crate::app::STEP) as usize {
            g.update(crate::app::STEP);
        }
        assert!(g.enemies.len() > before, "a new foe arrived");
        // Half the keys: shorter waits, more foes allowed; and they never scatter.
        g.keys_got = g.keys_total / 2;
        assert!(g.spawn_every() < calm_wait && g.max_enemies() > calm_cap);
        g.global_mode = GlobalMode::Scatter;
        g.mode_index = 0;
        g.update(crate::app::STEP);
        assert_eq!(g.global_mode, GlobalMode::Chase, "they hunt without a break");
    }

    #[test]
    fn every_second_key_quickens_the_music() {
        let mut g = playing();
        for n in 1..=7u32 {
            g.keys_got = n - 1;
            // Put a nugget under him and let him pick it up.
            let o = g.hero.occupied();
            g.level.grid.set(o.x, o.y, Tile::Nodule);
            g.collect();
            assert_eq!(g.audio.tempo_step(), n / KEYS_PER_TEMPO_STEP, "after {n} keys");
        }
    }

    #[test]
    fn ragnar_starts_in_the_middle_after_a_countdown() {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        let mut g = Game::new();
        let spawn = g.level.hero_spawn;
        let (w, h) = (g.level.grid.width, g.level.grid.height);
        assert!((spawn.x - (w - 1) / 2).abs() <= 2 && (spawn.y - (h - 1) / 2).abs() <= 2, "spawn {spawn:?}");
        // Every pickup is still reachable from the new start.
        let seen = crate::logic::pathing::reachable(&g.level.grid, spawn, true, false);
        for (x, y, t) in g.level.grid.cells() {
            if t == Tile::Nodule || t == Tile::Charge {
                assert!(seen.contains(&P::new(x, y)), "pickup at {x},{y} unreachable from the center");
            }
        }
        start_from_title(&mut g);
        assert_eq!(g.screen, Screen::Ready);
        for _ in 0..(2.9 / crate::app::STEP) as usize {
            g.update(crate::app::STEP);
        }
        assert_eq!(g.screen, Screen::Ready, "still counting down at 2.9 s");
        for _ in 0..(0.2 / crate::app::STEP) as usize {
            g.update(crate::app::STEP);
        }
        assert_eq!(g.screen, Screen::Playing);
        assert_eq!(g.hero.occupied(), spawn);
    }

    #[test]
    fn the_right_stick_turns_the_camera_and_it_stays_put() {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        let mut g = playing();
        let start = g.view.cam_yaw;
        let pad = |x: f32, y: f32| crate::input::PadState { right: (x, y), ..Default::default() };
        g.input.set_pad(Some(pad(1.0, 0.0)));
        for _ in 0..60 {
            g.update(crate::app::STEP);
        }
        let turned = g.view.cam_yaw;
        assert!(turned - start > 0.5, "stick barely turned the camera: {}", turned - start);
        // Let go and wait: the camera keeps the angle instead of drifting back behind him.
        g.input.set_pad(Some(pad(0.0, 0.0)));
        for _ in 0..(3.0 / crate::app::STEP) as usize {
            g.update(crate::app::STEP);
        }
        assert!((g.view.cam_yaw - turned).abs() < 1e-3, "camera drifted");
        // Tilting the stick up and down pitches it.
        let pitch = g.view.cam_pitch;
        g.input.set_pad(Some(pad(0.0, -1.0)));
        for _ in 0..20 {
            g.update(crate::app::STEP);
        }
        assert!(g.view.cam_pitch != pitch);
        // R3 swings it back behind him.
        let mut click = pad(0.0, 0.0);
        click.actions.insert(Action::CameraReset);
        g.input.set_pad(Some(click));
        g.update(crate::app::STEP);
        g.input.set_pad(Some(pad(0.0, 0.0)));
        for _ in 0..120 {
            g.update(crate::app::STEP);
        }
        let behind = crate::render::view::dir_to_yaw(g.hero.facing);
        let off = (g.view.cam_yaw - behind).rem_euclid(std::f32::consts::TAU);
        assert!(off < 0.05 || off > std::f32::consts::TAU - 0.05, "not recentered: {off}");
    }

    #[test]
    fn the_menus_can_quit() {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        let mut g = Game::new();
        assert_eq!(g.menu_items(), vec![MenuItem::Play, MenuItem::Controls, MenuItem::Map, MenuItem::Quit]);
        // Right three times, then Enter, on the title picks Quit.
        for _ in 0..3 {
            g.input.key_down(winit::keyboard::KeyCode::ArrowRight, false);
            g.update(crate::app::STEP);
            g.input.key_up(winit::keyboard::KeyCode::ArrowRight);
            g.update(crate::app::STEP);
        }
        g.input.key_down(winit::keyboard::KeyCode::Enter, false);
        g.update(crate::app::STEP);
        assert!(g.quit_requested);
        assert_eq!(g.screen, Screen::Title);

        // Pause offers Resume / Call the Old Man / Controller / Map / Quit to Title / Quit Game; clicking a button picks it.
        let mut g = playing();
        g.input.inject(Action::Pause);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Paused);
        assert_eq!(g.menu_items(), vec![MenuItem::Resume, MenuItem::CallOldMan, MenuItem::Controls, MenuItem::Map, MenuItem::QuitToTitle, MenuItem::Quit]);
        let r = crate::render::hud::menu_rects(Screen::Paused, &g.menu_items(), g.viewport.0, g.viewport.1)[4];
        g.input.cursor_moved_to(r[0] + r[2] / 2.0, r[1] + r[3] / 2.0);
        g.input.mouse_button(winit::event::MouseButton::Left, true);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Title);
    }

    #[test]
    fn he_stands_on_his_raised_start_through_the_tour_and_the_countdown() {
        let mut g = Game::new();
        start_from_title(&mut g);
        assert_eq!(g.screen, Screen::Ready);
        let spawn = g.level.hero_spawn;
        let floor = g.level.grid.elevation(spawn.x, spawn.y) as f64;
        assert!(floor > 0.0, "the first start is on the fountain's terrace");
        assert_eq!(g.hero.altitude, floor, "not sunk into the stone before he moves");
    }

    #[test]
    fn the_map_jumps_to_any_dungeon_or_cyclops() {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        let mut g = Game::new();
        let stops = map_stops();
        assert_eq!(stops.len(), 2 * crate::logic::levels::levels().len(), "each dungeon and its Cyclops");
        assert!(stops[0].label.contains("THE GARDEN OF KRAMPUS"));
        // From the title: MAP, down to the third stop (Dungeon II), go.
        g.menu_index = g.menu_items().iter().position(|&m| m == MenuItem::Map).unwrap();
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        assert!(g.show_map);
        for _ in 0..2 {
            g.input.nav = Some(Dir::Down);
            g.update(crate::app::STEP);
        }
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        assert!(!g.show_map);
        assert_eq!((g.screen, g.dungeon, g.boss.is_some()), (Screen::Ready, 1, false));
        assert_eq!(g.saved_run.as_ref().map(|r| r.stage), Some(save::Stage::Dungeon(1)), "Continue comes back here");
        // From the pause menu, to a Cyclops; Esc closes the map without going anywhere.
        g.enter(Screen::Paused);
        g.open_map();
        assert_eq!(stops[g.map_index].stage, save::Stage::Dungeon(1), "where he is, highlighted");
        g.input.inject(Action::Pause);
        g.update(crate::app::STEP);
        assert_eq!((g.show_map, g.screen), (false, Screen::Paused));
        g.map_jump(save::Stage::Arena(0));
        assert_eq!((g.screen, g.dungeon, g.boss.is_some()), (Screen::Ready, 0, true));
    }

    #[test]
    fn mehmed_is_slow_and_sturdy_and_hits_hard_miyamoto_quick_and_frail_and_strikes_fast() {
        let mut g = Game::new();
        // Bare heroes: no path skills to add health or quicken the blade.
        g.progress = Progress::default();
        let mut seen = Vec::new();
        for hero in [HeroKind::Ragnar, HeroKind::Mehmed, HeroKind::Miyamoto] {
            g.hero_kind = hero;
            g.load_level(0);
            assert!(g.hero.try_attack());
            seen.push((hero, g.hero.max_hp, g.hero.hp, g.hero.m.speed, g.hero.blow(3), g.hero.attack_cooldown));
        }
        let [(_, r_max, r_hp, r_speed, r_blow, _), (_, m_max, m_hp, m_speed, m_blow, _), (_, y_max, _, y_speed, y_blow, y_cool)] = seen[..] else { unreachable!() };
        assert_eq!((r_max, m_max, y_max), (100, 120, 80), "health 1 : 1.2 : 0.8");
        assert_eq!((r_hp, m_hp), (100, 120), "and each begins with it full");
        assert!((m_speed / r_speed - 0.9).abs() < 1e-9 && (y_speed / r_speed - 1.1).abs() < 1e-9, "speed 1 : 0.9 : 1.1");
        assert!(m_blow > r_blow && y_blow == r_blow, "Mehmed hits harder: {m_blow} against {r_blow}");
        // Miyamoto's blade waits a fifth less than it would for anyone else.
        let own = crate::logic::progression::blade(HeroKind::Miyamoto, g.hero.blade).cooldown;
        assert!((y_cool - own * crate::logic::heroes::MIYAMOTO_TEMPO).abs() < 1e-9, "{y_cool} against {own}");
    }

    /// A foe of `kind` hunting at `at`, in the game and in the view.
    fn foe_at(g: &mut Game, kind: EnemyKind, at: P) -> usize {
        let spec = default_enemy_specs(&g.level.grid).into_iter().find(|s| s.kind == kind).unwrap();
        let mut e = Enemy::new(&spec, at, g.level.data.enemy_speed);
        e.b.mode = EnemyMode::Chase;
        g.view.add_enemy(&e);
        g.enemies.push(e);
        g.enemies.len() - 1
    }

    fn fell(g: &Game, i: usize) -> bool {
        matches!(g.enemies[i].mode(), EnemyMode::Eaten | EnemyMode::Scattered)
    }

    /// Into the Cyclops' open meadow, with no foes but those placed; he stands at the south edge facing north.
    fn meadow() -> Game {
        let mut g = playing();
        g.load_boss(0);
        g.enemies.clear();
        g.view.clear_enemies();
        g.enter(Screen::Playing);
        g
    }

    #[test]
    fn an_archangels_shrine_switches_when_struck_and_blesses_whoever_steps_on_it() {
        use crate::logic::angels::{Archangel, Blessing};
        let mut g = playing();
        let i = g.diamonds.iter().position(|d| d.kind == DiamondKind::Angel).expect("an archangel's shrine");
        assert_eq!(g.diamonds[i].current(), Offer::Angel(Archangel::Michael));
        // It holds still until struck.
        for _ in 0..600 {
            g.diamonds[i].update(crate::app::STEP);
        }
        assert_eq!(g.diamonds[i].current(), Offer::Angel(Archangel::Michael));
        g.diamonds[i].cycle();
        assert_eq!(g.diamonds[i].current(), Offer::Angel(Archangel::Gabriel));
        g.hero.m.teleport(g.diamonds[i].tile);
        g.visit_shrines();
        assert_eq!(g.blessing, Some(Blessing { angel: Archangel::Gabriel, charges: 3 }));
        // Each archangel gives once a level: Gabriel again, at this shrine or another, is refused.
        g.blessing = None;
        let other = g.diamonds.iter().position(|d| d.kind == DiamondKind::Angel && d.tile != g.diamonds[i].tile).unwrap();
        while g.diamonds[other].current() != Offer::Angel(Archangel::Gabriel) {
            g.diamonds[other].cycle();
        }
        g.hero.m.teleport(g.diamonds[other].tile);
        g.visit_shrines();
        assert_eq!(g.blessing, None, "Gabriel has given this level");
        // The other three, then every shrine stands grey.
        for angel in [Archangel::Michael, Archangel::Raphael, Archangel::Uriel] {
            while g.diamonds[other].current() != Offer::Angel(angel) {
                g.diamonds[other].cycle();
            }
            g.visit_shrines();
            assert_eq!(g.blessing.map(|b| b.angel), Some(angel));
        }
        assert!(g.diamonds.iter().filter(|d| d.kind == DiamondKind::Angel).all(|d| !d.active), "all four have given: grey");
        g.blessing = Some(Blessing::of(Archangel::Gabriel));
        // The archangels stand in every dungeon.
        g.load_level(1);
        assert_eq!(g.diamonds.iter().filter(|d| d.kind == DiamondKind::Angel && d.active).count(), 4, "fresh in the next level");
        assert_eq!(g.angels_used, [false; 4]);
        assert_eq!(g.blessing.map(|b| b.charges), Some(3), "the blessing goes with him");
    }

    #[test]
    fn gabriels_thunder_strikes_every_foe_within_ten_tiles() {
        use crate::logic::angels::{Archangel, Blessing};
        let mut g = meadow();
        let o = g.hero.occupied();
        let near = foe_at(&mut g, EnemyKind::Bat, P::new(o.x + 3, o.y - 4));
        let far = foe_at(&mut g, EnemyKind::Bat, P::new(o.x - 7, o.y - 11));
        g.blessing = Some(Blessing::of(Archangel::Gabriel));
        g.invoke_blessing();
        assert!(fell(&g, near) && !fell(&g, far));
        assert_eq!(g.blessing.map(|b| b.charges), Some(2));
    }

    #[test]
    fn raphaels_beam_fells_all_in_its_path_and_nothing_beside_it() {
        use crate::logic::angels::{Archangel, Blessing};
        let mut g = meadow();
        let o = g.hero.occupied();
        g.hero.face(Dir::Up);
        let ahead = [foe_at(&mut g, EnemyKind::Slime, P::new(o.x, o.y - 2)), foe_at(&mut g, EnemyKind::Bat, P::new(o.x, o.y - 4))];
        let beside = foe_at(&mut g, EnemyKind::Bat, P::new(o.x + 3, o.y - 3));
        g.blessing = Some(Blessing::of(Archangel::Raphael));
        g.invoke_blessing();
        assert!(ahead.iter().all(|&i| fell(&g, i)) && !fell(&g, beside));
        g.invoke_blessing();
        assert_eq!(g.blessing, None, "two uses, then it is spent");
    }

    #[test]
    fn uriels_wave_rolls_out_over_everything_and_uriel_then_rests() {
        use crate::logic::angels::{Archangel, Blessing, URIEL_REST};
        let mut g = meadow();
        let o = g.hero.occupied();
        let foes = [foe_at(&mut g, EnemyKind::Bat, P::new(o.x + 1, o.y - 1)), foe_at(&mut g, EnemyKind::Bat, P::new(o.x - 8, o.y - 12))];
        g.blessing = Some(Blessing::of(Archangel::Uriel));
        g.invoke_blessing();
        assert!(!fell(&g, foes[1]), "the far one only when the wave reaches it");
        for _ in 0..(1.6 / crate::app::STEP) as usize {
            g.update_angels(crate::app::STEP);
        }
        assert!(foes.iter().all(|&i| fell(&g, i)));
        assert!((g.uriel_rest - URIEL_REST).abs() < 2.0, "ten minutes' rest");
        // Meanwhile his shrines turn the hero away.
        let mut g = playing();
        g.uriel_rest = 100.0;
        let i = g.diamonds.iter().position(|d| d.kind == DiamondKind::Angel).unwrap();
        for _ in 0..3 {
            g.diamonds[i].cycle();
        }
        g.hero.m.teleport(g.diamonds[i].tile);
        g.visit_shrines();
        assert_eq!(g.blessing, None);
        assert!(g.diamonds[i].active);
    }

    #[test]
    fn michaels_fire_turns_a_foe_that_then_hunts_the_others() {
        use crate::logic::angels::{Archangel, Blessing};
        let mut g = meadow();
        let o = g.hero.occupied();
        g.hero.face(Dir::Up);
        let turned = foe_at(&mut g, EnemyKind::Slime, P::new(o.x, o.y - 3));
        g.blessing = Some(Blessing::of(Archangel::Michael));
        g.invoke_blessing();
        for _ in 0..(1.0 / crate::app::STEP) as usize {
            g.update_projectiles(crate::app::STEP);
        }
        assert!(g.enemies[turned].is_ally() && !g.enemies[turned].is_dangerous(), "turned to his side");
        assert_eq!(g.blessing.map(|b| b.charges), Some(3));
        // A foe that comes within its reach dies.
        let at = g.enemies[turned].occupied();
        let foe = foe_at(&mut g, EnemyKind::Bat, at);
        g.update_angels(crate::app::STEP);
        assert!(fell(&g, foe));
        assert!(!fell(&g, turned));
    }

    #[test]
    fn the_first_cyclops_takes_fifty_and_the_archangels_wound_it_exactly() {
        use crate::logic::angels::{Archangel, Blessing};
        let mut g = playing();
        g.input.key_down(winit::keyboard::KeyCode::F1, false);
        g.update(crate::app::STEP);
        for _ in 0..((READY_SECONDS + 2.2) / crate::app::STEP) as usize {
            g.update(crate::app::STEP);
            g.hero.invuln = 1.0;
        }
        let hp0 = g.boss.as_ref().unwrap().hp;
        assert_eq!(g.boss.as_ref().unwrap().max_hp, 50);
        let bp = g.boss.as_ref().unwrap().pos;
        g.hero.m.teleport(P::new(bp.x.round() as i32, bp.y.round() as i32 + 4));
        g.blessing = Some(Blessing::of(Archangel::Gabriel));
        g.invoke_blessing();
        assert_eq!(g.boss.as_ref().unwrap().hp, hp0 - 5);
        // Dazed, it would take double from a blade, but an archangel's blow is what it is.
        g.debug_daze_boss();
        for _ in 0..20 {
            g.update(crate::app::STEP);
            g.hero.invuln = 1.0;
        }
        let hp1 = g.boss.as_ref().unwrap().hp;
        g.invoke_blessing();
        assert_eq!(g.boss.as_ref().unwrap().hp, hp1 - 5);
    }

    #[test]
    fn one_swing_fells_every_foe_it_sweeps_through() {
        let mut g = playing();
        // Two foes side by side just ahead of him, as in a corridor two tiles wide, and one behind.
        let o = g.hero.occupied();
        g.hero.face(Dir::Up);
        let spots = [P::new(o.x, o.y - 1), P::new(o.x + 1, o.y - 1), P::new(o.x, o.y + 2)];
        for (e, &at) in g.enemies.iter_mut().zip(&spots) {
            e.m.teleport(at);
            e.b.mode = EnemyMode::Chase;
        }
        g.hitstop = 0.0;
        g.perform_attack();
        let fell = |g: &Game, i: usize| matches!(g.enemies[i].mode(), EnemyMode::Eaten | EnemyMode::Scattered);
        assert!(fell(&g, 0) && fell(&g, 1), "both foes ahead fall to one blow");
        assert!(!fell(&g, 2), "the one behind him is untouched");
    }

    #[test]
    fn those_waiting_in_the_hollow_stand_in_it() {
        let g = playing();
        for p in &g.nest_slots[1..] {
            assert_eq!(g.level.grid.get(p.x, p.y), Tile::Nest, "{p:?} is not in the hollow");
        }
    }

    #[test]
    fn draugr_rise_from_the_churchyard() {
        let g = playing();
        let graves = g.grave_spawns();
        assert!(!graves.is_empty(), "Dungeon I has a churchyard");
        let draugr = g.enemies.iter().find(|e| e.kind() == EnemyKind::Skeleton).expect("a draugr");
        assert!(graves.contains(&draugr.m.tile), "it starts by the graves, not in the hollow: {:?}", draugr.m.tile);
    }

    #[test]
    fn the_last_key_unlocks_the_gate_and_going_through_wins() {
        let mut g = playing();
        assert_eq!(g.keys_total, 8);
        g.keys_got = g.keys_total - 1;
        let o = g.hero.occupied();
        g.level.grid.set(o.x, o.y, Tile::Nodule);
        g.collect();
        assert!(g.gate_open);
        assert_eq!(g.screen, Screen::Playing, "the gate must still be reached");
        // Up to the gate: the doors swing open, and he steps through.
        let gate = g.gate.unwrap();
        g.hero.m.teleport(P::new(gate.x, gate.y + 1));
        for _ in 0..((DOOR_SECONDS + 0.2) / crate::app::STEP) as usize {
            g.update(crate::app::STEP);
        }
        assert!(g.door >= 1.0);
        assert!(g.level.grid.is_walkable(gate.x, gate.y, false));
        g.hero.m.teleport(gate);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Cleared);
        assert!(g.events.contains(&GameEvent::Won));
        // After the fanfare, the Cyclops' meadow, and a countdown.
        for _ in 0..((CLEARED_SECONDS + 0.1) / crate::app::STEP) as usize {
            g.update(crate::app::STEP);
        }
        assert_eq!(g.screen, Screen::Ready);
        assert!(g.level.data.boss);
        assert!(g.boss.is_some());
        assert!(g.enemies.is_empty());
        assert_eq!(g.saved_run.as_ref().map(|r| r.stage), Some(save::Stage::Arena(0)), "the arena is a checkpoint");
    }

    #[test]
    fn f1_jumps_to_the_boss_and_felling_it_leads_on() {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        let mut g = Game::new();
        g.input.key_down(winit::keyboard::KeyCode::F1, false);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Ready);
        assert!(g.level.data.boss, "F1 goes straight to the boss arena");
        for _ in 0..((READY_SECONDS + 2.2) / crate::app::STEP) as usize {
            g.update(crate::app::STEP);
            g.hero.invuln = 1.0; // keep him alive while we watch
        }
        assert_eq!(g.screen, Screen::Playing);
        assert!(g.boss.as_ref().unwrap().active(), "the Cyclops has landed");
        // An axe blow in reach lands.
        let bp = g.boss.as_ref().unwrap().pos;
        g.hero.m.teleport(P::new(bp.x.round() as i32, bp.y.round() as i32 + 1));
        g.hero.facing = Dir::Up;
        let hp = g.boss.as_ref().unwrap().hp;
        g.hitstop = 0.0;
        g.perform_attack();
        assert!(g.boss.as_ref().unwrap().hp < hp, "axe missed the Cyclops");
        // Beat it and the victory screen leads to dungeon II.
        while g.boss.as_ref().unwrap().alive() {
            g.debug_wound_boss(10);
            for _ in 0..20 {
                g.update(crate::app::STEP);
                g.hero.invuln = 1.0;
            }
        }
        // Back at the south edge while it falls (so he does not pick the key up at once).
        g.hero.m.teleport(P::new(crate::logic::levels::ARENA_HERO.0, crate::logic::levels::ARENA_HERO.1));
        for _ in 0..(6.0 / crate::app::STEP) as usize {
            g.update(crate::app::STEP);
            g.hero.invuln = 1.0;
        }
        // It drops a red diamond key; the gate in the north hedge stays shut without it.
        assert_eq!(g.screen, Screen::Playing, "no victory yet: the key and the gate first");
        let key = g.red_key.expect("the Cyclops drops a red diamond key");
        assert!(g.level.grid.is_walkable(key.x, key.y, false), "on grass he can reach");
        let gate = g.gate.expect("a gate in the arena");
        assert_eq!((gate.x, gate.y), crate::logic::levels::ARENA_GATE);
        let front = P::new(gate.x, gate.y + 1);
        assert!(g.level.grid.is_walkable(front.x, front.y, false), "the way to the gate is clear");
        g.hero.m.teleport(front);
        for _ in 0..(2.0 / crate::app::STEP) as usize {
            g.update(crate::app::STEP);
        }
        assert!(!g.gate_open && g.door == 0.0, "locked without the key");
        // He takes the key: the gate unlocks, and its doors open as he comes to it.
        g.hero.m.teleport(key);
        g.update(crate::app::STEP);
        assert!(g.red_key.is_none() && g.gate_open, "the red key opens the gate");
        g.hero.m.teleport(front);
        for _ in 0..(3.0 / crate::app::STEP) as usize {
            g.update(crate::app::STEP);
        }
        assert!(g.door >= 1.0, "the doors swing open");
        g.hero.m.teleport(gate);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Won);
        assert!(g.events.contains(&GameEvent::BossSlain));
        assert!(g.progress.silver >= COINS_BOSS, "the boss pays out");
        assert!(g.progress.runes >= RUNES_BOSS, "and leaves rune fragments");
        assert!((0..4).any(|t| g.progress.hero(g.hero_kind).relics[t] != 0), "and a relic");
        assert_eq!(g.saved_run.as_ref().map(|r| r.stage), Some(save::Stage::Shop(0)), "Continue would bring back the old man");
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Shop, "the old man comes after the boss");
        // Leave him: on to dungeon II, where only the old man sells gear.
        g.input.inject(Action::Pause);
        g.update(crate::app::STEP);
        g.input.inject(Action::Pause);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Ready);
        assert!(!g.level.data.boss);
        assert_eq!(g.dungeon, 1);
        assert!(g.potions.is_empty() && g.diamonds.iter().all(|d| d.kind == DiamondKind::Angel), "no free gear after the first dungeon, only the archangels");
        assert_eq!(g.saved_run.as_ref().map(|r| r.stage), Some(save::Stage::Dungeon(1)));
    }

    #[test]
    fn free_movement_lasts_from_dungeon_to_dungeon() {
        let mut g = Game::new();
        g.hero.free_move = true;
        start_from_title(&mut g);
        assert!(g.hero.free_move, "beginning a game keeps it");
        g.load_boss(0);
        g.load_level(1);
        assert!(g.hero.free_move, "and so does every level after");
    }

    #[test]
    fn continue_picks_up_the_saved_stage_and_start_asks_before_wiping_everything() {
        let mut g = Game::new();
        assert_eq!(g.menu_items(), vec![MenuItem::Play, MenuItem::Controls, MenuItem::Map, MenuItem::Quit], "nothing to continue yet");
        start_from_title(&mut g);
        assert_eq!(g.saved_run.as_ref().map(|r| r.stage), Some(save::Stage::Dungeon(0)), "an adventure is saved from its start");

        // Mehmed reaches the Cyclops with silver, a score, two lives and two bottles.
        g.hero_kind = HeroKind::Mehmed;
        g.progress.silver = 500;
        g.score.resume(7000, 2);
        g.belt = vec![PotionKind::Health, PotionKind::Life];
        g.checkpoint(save::Stage::Arena(0));
        g.enter(Screen::Title);
        assert_eq!(g.menu_items(), vec![MenuItem::Continue, MenuItem::Play, MenuItem::Controls, MenuItem::Map, MenuItem::Quit]);
        g.hero_kind = HeroKind::Ragnar;
        g.score.reset();
        g.belt.clear();
        g.load_level(0);
        // Continue is highlighted first.
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Ready, "no story and no character screen");
        assert!(g.level.data.boss && g.boss.is_some());
        assert_eq!((g.hero_kind, g.hero.kind), (HeroKind::Mehmed, HeroKind::Mehmed));
        assert_eq!((g.score.score, g.score.lives), (7000, 2));
        assert_eq!(g.belt, vec![PotionKind::Health, PotionKind::Life]);

        // Start asks first, with "keep my progress" highlighted: Enter loses nothing.
        g.enter(Screen::Title);
        g.menu_index = 1;
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        assert!(g.confirm_reset);
        assert_eq!(g.menu_items(), vec![MenuItem::StartOver, MenuItem::KeepProgress]);
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        assert!(!g.confirm_reset);
        assert_eq!((g.screen, g.progress.silver), (Screen::Title, 500));
        assert!(g.saved_run.is_some());
        // Esc backs out too.
        g.menu_index = 1;
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        g.input.inject(Action::Pause);
        g.update(crate::app::STEP);
        assert!(!g.confirm_reset && g.saved_run.is_some());
        // Start over: everything goes, and the new adventure begins (with the heroes, the story being off in tests).
        g.menu_index = 1;
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        g.menu_index = 0;
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Select);
        assert_eq!(g.progress, Progress::default());
        assert!(g.saved_run.is_none());
        g.enter(Screen::Title);
        assert_eq!(g.menu_items(), vec![MenuItem::Play, MenuItem::Controls, MenuItem::Map, MenuItem::Quit]);
    }

    #[test]
    fn a_foe_across_the_maze_finds_the_hero_rather_than_circling() {
        let mut g = playing();
        g.enemies.clear();
        g.view.clear_enemies();
        let hero = g.hero.occupied();
        // Somewhere well away by the paths.
        let far = crate::logic::pathing::reachable(&g.level.grid, hero, false, false)
            .into_iter()
            .filter(|p| (p.x - hero.x).abs() + (p.y - hero.y).abs() >= 14 && g.level.grid.get(p.x, p.y) == Tile::Open)
            .min_by_key(|p| ((p.x - hero.x).abs() + (p.y - hero.y).abs(), p.y, p.x))
            .unwrap();
        let i = foe_at(&mut g, EnemyKind::Skeleton, far);
        let mut closest = f64::INFINITY;
        for _ in 0..(15.0 / crate::app::STEP) as usize {
            g.hero.invuln = 1.0;
            g.update(crate::app::STEP);
            closest = closest.min(dist_sq(g.enemies[i].pos(), g.hero.pos()).sqrt());
        }
        assert!(closest < 1.5, "it came no closer than {closest}");
    }

    #[test]
    fn lt_and_rt_page_through_the_old_mans_wares() {
        let mut g = playing();
        g.open_shop();
        let first = g.shop.tab;
        g.input.inject(Action::TabNext);
        g.input.inject(Action::Attack); // RT is Attack as well; it must not buy
        g.update(crate::app::STEP);
        let second = g.shop.tab;
        assert_ne!(first, second);
        g.input.inject(Action::TabPrev);
        g.update(crate::app::STEP);
        assert_eq!(g.shop.tab, first);
    }

    #[test]
    fn the_controllers_menu_button_opens_the_pause_menu_and_it_stays_open() {
        let mut g = playing();
        // Menu is Pause and Confirm at once.
        g.input.inject(Action::Pause);
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Paused);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Paused, "not resumed straight away");
        assert!(g.menu_items().contains(&MenuItem::CallOldMan), "the old man's wares from the menu");
        // Menu again resumes.
        g.input.inject(Action::Pause);
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Playing);
    }

    #[test]
    fn the_controller_page_opens_from_the_title_and_the_pause_menu() {
        let mut g = Game::new();
        g.menu_index = g.menu_items().iter().position(|&i| i == MenuItem::Controls).unwrap();
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        assert!(g.show_controls && g.screen == Screen::Title);
        // B closes it, back on the title with the button still highlighted.
        g.input.inject(Action::Bomb);
        g.update(crate::app::STEP);
        assert!(!g.show_controls && g.screen == Screen::Title);
        assert_eq!(g.menu_items()[g.menu_index], MenuItem::Controls);

        // From the pause menu, Esc closes the page and leaves the game paused.
        let mut g = playing();
        g.input.inject(Action::Pause);
        g.update(crate::app::STEP);
        g.menu_index = g.menu_items().iter().position(|&i| i == MenuItem::Controls).unwrap();
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        assert!(g.show_controls && g.screen == Screen::Paused);
        g.input.inject(Action::Pause);
        g.update(crate::app::STEP);
        assert!(!g.show_controls);
        assert_eq!(g.screen, Screen::Paused, "still paused");
    }

    #[test]
    fn the_old_man_sells_each_tree_in_order_and_equips() {
        let mut g = playing();
        g.open_shop();
        g.progress.silver = 5000;
        let press = |g: &mut Game, action: ShopAction| {
            let rows = g.shop_rows();
            let (r, c) = rows
                .iter()
                .enumerate()
                .find_map(|(r, row)| row.buttons.iter().position(|b| b.action == action).map(|c| (r, c)))
                .expect("no such button");
            g.shop.row = r;
            g.shop.col = c;
            g.input.inject(Action::Confirm);
            g.update(crate::app::STEP);
        };
        // Tier 3 cannot be had before tier 2.
        press(&mut g, ShopAction::Unlock(Tree::Armory, 3));
        assert_eq!(g.progress.hero(g.hero_kind).owned(Tree::Armory), 1);
        press(&mut g, ShopAction::Unlock(Tree::Armory, 2));
        assert_eq!(g.hero.blade, 2, "a new weapon goes straight to hand");
        assert_eq!(g.hero.main, crate::logic::items::WeaponKind::Ranged(crate::logic::items::RangedKind::Hurled(HeroKind::Ragnar, 2)), "Skofnung is thrown");
        press(&mut g, ShopAction::Wield(1));
        assert_eq!(g.hero.blade, 1);
        assert_eq!(g.hero.main, crate::logic::items::WeaponKind::Sword);
        assert_eq!(g.progress.silver, 5000 - progression::price(Tree::Armory, 2).silver);
        g.shop.tab = Tree::Path;
        press(&mut g, ShopAction::Unlock(Tree::Path, 1));
        assert_eq!(g.hero.path, 1, "the skill is his at once");
        g.shop.tab = Tree::Shield;
        press(&mut g, ShopAction::Unlock(Tree::Shield, 2));
        assert_eq!(g.hero.shield.unwrap().tier, 2);
        g.shop.tab = Tree::Elixir;
        let before = g.belt.len();
        press(&mut g, ShopAction::Bottle(PotionKind::Health));
        assert_eq!(g.belt.len(), before + 1);
        // An elixir must be unlocked before its bottles are sold.
        assert!(g.shop_rows().iter().flat_map(|r| &r.buttons).all(|b| b.action != ShopAction::Bottle(PotionKind::Strength)));
        // Too poor: nothing changes hands.
        g.progress.silver = 10;
        press(&mut g, ShopAction::Unlock(Tree::Elixir, 2));
        assert_eq!(g.progress.silver, 10);
        assert_eq!(g.progress.hero(g.hero_kind).owned(Tree::Elixir), 1);
    }

    #[test]
    fn the_old_man_can_be_called_from_the_dungeon_and_it_waits() {
        let mut g = playing();
        let at = g.hero.pos();
        g.input.key_down(winit::keyboard::KeyCode::KeyH, false);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Shop);
        assert!(g.shop.called);
        g.input.inject(Action::Pause);
        g.update(crate::app::STEP);
        g.input.inject(Action::Pause);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Ready, "back to the same dungeon after a countdown");
        assert_eq!(g.dungeon, 0);
        assert_eq!(g.hero.pos(), at, "where he was");
    }

    #[test]
    fn each_dungeon_hides_a_shrine_with_a_relic() {
        let mut g = playing();
        let t = g.relic_shrine.expect("a hidden shrine");
        assert_eq!(g.level.grid.get(t.x, t.y), Tile::Open);
        let runes = g.progress.runes;
        g.hero.m.teleport(t);
        g.update(crate::app::STEP);
        assert!(g.relic_shrine.is_none());
        assert_eq!(g.progress.runes, runes + RUNES_SHRINE);
        assert!((0..4).any(|i| g.progress.hero(g.hero_kind).relics[i] != 0));
    }

    #[test]
    fn the_path_revives_and_restores_once_per_dungeon() {
        let mut g = playing();
        g.progress.hero_mut(g.hero_kind).owned[1] = 10;
        g.equip_from_progress();
        g.hero.shield = None;
        g.hero.hp = 20;
        g.update(crate::app::STEP);
        assert_eq!(g.hero.hp, g.hero.max_hp, "the World Tree restores him at a quarter health");
        g.hero.hp = 20;
        g.update(crate::app::STEP);
        assert_eq!(g.hero.hp, 20, "only once");
        g.hero.invuln = 0.0;
        g.hurt_hero(500);
        assert!(g.hero.alive, "the Valkyrie raises him");
        assert_eq!(g.hero.hp, g.hero.max_hp / 2);
        g.hero.invuln = 0.0;
        g.hurt_hero(500);
        assert!(!g.hero.alive, "but only once");
    }

    #[test]
    fn the_blades_carry_their_gifts() {
        let mut g = playing();
        // Tier 4 stuns the foes around the blow.
        g.debug_arm(4);
        let hp = g.hero.pos();
        for e in g.enemies.iter_mut() {
            e.b.mode = EnemyMode::Chase;
            e.m.teleport(P::new(hp.x as i32 - 2, hp.y as i32));
        }
        g.hero.facing = Dir::Right;
        g.hero.attack_cooldown = 0.0;
        assert!(g.hero.try_attack());
        g.perform_attack();
        assert!(g.enemies.iter().all(|e| e.is_stunned()), "the oath binds them");
        // Tier 6 leaves a decoy.
        g.debug_arm(6);
        g.hero.attack_cooldown = 0.0;
        g.perform_attack();
        assert!(g.decoy.is_some());
    }

    /// A game in play as `hero`, with his trees owned up to `tier`.
    fn playing_as(hero: HeroKind, tier: u8) -> Game {
        let mut g = playing();
        g.hero_kind = hero;
        g.debug_arm(tier);
        g.hero.hp = g.hero.max_hp;
        g
    }

    #[test]
    fn a_samurai_stands_his_last_and_a_spirit_fights_beside_him() {
        let mut g = playing_as(HeroKind::Miyamoto, 9);
        g.hero.shield = None;
        g.hero.invuln = 0.0;
        g.hurt_hero(500);
        assert!(g.hero.alive, "Seppuku Resolve holds death off");
        assert!(g.hero.last_stand > 0.0);
        assert_eq!(g.hero.blow(2), 6, "and triples his blows");
        g.hero.invuln = 0.0;
        g.hurt_hero(500);
        assert!(g.hero.alive && g.hero.hp == 1, "he cannot die while it lasts");
        // His ancestor strikes down a foe that comes near.
        let hp = g.hero.pos();
        g.enemies[0].b.mode = EnemyMode::Chase;
        g.enemies[0].m.teleport(P::new(hp.x as i32, hp.y as i32));
        for _ in 0..(2.0 / crate::app::STEP) as usize {
            g.update_spirits(crate::app::STEP);
        }
        assert_eq!(g.spirits.len(), 1);
        assert!(!g.enemies[0].is_dangerous(), "the spirit slew it");
    }

    #[test]
    fn the_janissary_has_a_squad_a_pistol_and_a_bow() {
        let mut g = playing_as(HeroKind::Mehmed, 9);
        g.update_spirits(crate::app::STEP);
        assert_eq!(g.spirits.len(), 3, "a squad of three");
        g.debug_arm(5);
        assert_eq!(g.hero.main, crate::logic::items::WeaponKind::Ranged(crate::logic::items::RangedKind::Hurled(HeroKind::Mehmed, 5)), "Koroglu's flintlock is shot");
        // The Simurg bow rains arrows on the nearest foes.
        g.debug_arm(8);
        let hp = g.hero.pos();
        for e in g.enemies.iter_mut().take(3) {
            e.b.mode = EnemyMode::Chase;
            e.m.teleport(P::new(hp.x as i32 + 2, hp.y as i32));
        }
        g.perform_attack();
        assert!(g.enemies.iter().take(3).all(|e| !e.is_dangerous()), "every one of them struck");
    }

    #[test]
    fn a_sun_blade_obliterates_the_foes_around_him() {
        for hero in [HeroKind::Miyamoto, HeroKind::Mehmed] {
            let mut g = playing_as(hero, 10);
            let hp = g.hero.pos();
            for (k, e) in g.enemies.iter_mut().enumerate() {
                e.b.mode = EnemyMode::Chase;
                e.m.teleport(P::new(hp.x as i32 + if k % 2 == 0 { 2 } else { -2 }, hp.y as i32));
            }
            g.perform_attack();
            assert!(g.enemies.iter().all(|e| !e.is_dangerous()), "{hero:?}: all gone in the flash");
        }
    }

    #[test]
    fn zero_opens_the_shop_from_anywhere() {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        let mut g = Game::new();
        g.input.key_down(winit::keyboard::KeyCode::Digit0, false);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Shop);
        assert!(g.progress.silver >= CHEAT_COINS);
        let mut g = playing();
        g.input.key_down(winit::keyboard::KeyCode::Digit0, false);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Shop);
    }

    #[test]
    fn a_third_quick_kill_is_brutal_and_doubles_points() {
        let mut g = playing();
        let before = g.score.score;
        g.slay_enemy(0);
        g.slay_enemy(1);
        assert_eq!(g.double_points, 0.0);
        g.slay_enemy(2);
        assert!(g.double_points > 0.0, "no BRUTAL on the third kill");
        assert_eq!(g.juice.banner.as_ref().map(|b| b.text.as_str()), Some("BRUTAL!"));
        // The third kill already counts double: 100 + 100 + 200.
        assert_eq!(g.score.score - before, 400);
        let mid = g.score.score;
        g.debug_collect_key();
        assert_eq!(g.score.score - mid, POINTS_NODULE * 2, "nuggets count double too");
    }

    #[test]
    fn the_first_game_opens_with_a_tour() {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test-tour"));
        let mut g = Game::new();
        g.tour_pending = true;
        start_from_title(&mut g);
        assert_eq!(g.screen, Screen::Tour);
        let stops = g.tour.as_ref().unwrap().stops.len();
        assert!(stops >= 8, "only {stops} stops");
        assert!(g.tour.as_ref().unwrap().stops.iter().any(|s| matches!(s.art, TourArt::Enemy(_))));
        // The camera flies to the first thing.
        for _ in 0..60 {
            g.update(crate::app::STEP);
        }
        assert!(g.view.tour.is_some());
        // Click through every stop; then the countdown.
        for _ in 0..stops {
            for _ in 0..40 {
                g.update(crate::app::STEP);
            }
            g.input.inject(Action::Confirm);
            g.update(crate::app::STEP);
        }
        assert_eq!(g.screen, Screen::Ready);
        assert!(!g.tour_pending);
        assert!(tour::tour_seen(), "the tour is remembered");
    }

    #[test]
    fn shields_wear_down_and_break() {
        let mut g = playing();
        g.hero.shield = Some(ShieldGear::new(1));
        let hp = g.hero.hp;
        g.debug_hurt(); // 25: the first shield takes 40%
        assert!(g.hero.hp < hp && g.hero.hp > hp - 25, "shield took no share: {}", g.hero.hp);
        let left = g.hero.shield.unwrap().durability;
        assert!(left < ShieldGear::new(1).max());
        for _ in 0..10 {
            g.debug_hurt();
            g.hero.hp = 100;
        }
        assert!(g.hero.shield.is_none(), "it should have broken");
    }

    #[test]
    fn potions_ride_on_the_belt_until_drunk() {
        let mut g = playing();
        g.belt = vec![PotionKind::Strength, PotionKind::Health];
        g.hero.hp = 40;
        g.input.inject(Action::Slot(1));
        g.update(crate::app::STEP);
        assert_eq!(g.hero.hp, 40 + crate::logic::hero::HEALTH_POTION);
        assert_eq!(g.belt, vec![PotionKind::Strength]);
        g.input.inject(Action::Drink);
        g.update(crate::app::STEP);
        assert!(g.belt.is_empty());
        assert!(g.hero.fx.strength > 0.0);
        assert_eq!(g.hero.blow(1), 3);
    }

    #[test]
    fn right_click_drops_a_bomb_and_left_click_swings() {
        let mut g = playing();
        g.input.mouse_button(winit::event::MouseButton::Right, true);
        g.update(crate::app::STEP);
        assert_eq!(g.bombs.len(), 1);
        g.input.mouse_button(winit::event::MouseButton::Left, true);
        g.update(crate::app::STEP);
        assert!(g.hero.swinging());
    }
}
