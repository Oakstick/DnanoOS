//! The game: screens, the round, and every rule that ties the entities together.

use std::collections::HashMap;
use std::f64::consts::PI;

mod select;
mod shop;
mod story;
mod tour;
pub use shop::{RowKind, ShopAction, ShopButton, ShopRow, ShopState, ShopTab};
pub use story::{Story, STORY_FADE, TYPE_DELAY};
pub use tour::{Tour, TourArt, TourStop, TOUR_STOP_SECONDS};

use crate::audio::{AudioManager, Sfx};
use crate::input::{Action, Input};
use crate::logic::boss::{BossEvent, BossShot, Cyclops, ShotKind, BODY_RADIUS, CONTACT_RADIUS, CYCLOPS_HP, CYCLOPS_HP_STEP, POINTS_CYCLOPS};
use crate::logic::enemy::{default_enemy_specs, Enemy, EnemyContext, EnemyKind, EnemyMode, GlobalMode, HeroView};
use crate::logic::gear::{self, Arsenal, ShieldGear, ShieldKind, BELT_SLOTS, COINS_BOSS, COINS_KILL, COINS_KEY};
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
    /// The blacksmith's forge, after a boss.
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
/// Past this much rage the foes stop scattering and hunt without rest.
const RAGE_NO_SCATTER: f64 = 0.5;
const SPAWN_CYCLE: [EnemyKind; 4] = [EnemyKind::Bat, EnemyKind::Slime, EnemyKind::Vampire, EnemyKind::Skeleton];
/// The music quickens one step for every this many keys.
pub const KEYS_PER_TEMPO_STEP: u32 = 2;
/// Seconds the gate's doors take to swing open.
const DOOR_SECONDS: f64 = 1.6;

/// Alternating scatter/chase schedule in seconds. Frozen while enemies are stunned.
const MODE_SCHEDULE: [(GlobalMode, f64); 6] = [
    (GlobalMode::Scatter, 7.0),
    (GlobalMode::Chase, 20.0),
    (GlobalMode::Scatter, 7.0),
    (GlobalMode::Chase, 20.0),
    (GlobalMode::Scatter, 5.0),
    (GlobalMode::Chase, f64::INFINITY),
];

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
const VICTORY_DELAY: f64 = 1.6;
/// The shop cheat tops the purse up to this, so there is something to spend.
const CHEAT_COINS: u32 = 1000;
/// Damage the Cyclops takes from a bomb (the weapons carry their own).
const BOMB_DAMAGE: i32 = 6;
/// A lightning potion's zap: how far it jumps and how many foes it strikes.
const ZAP_RANGE: f64 = 4.5;
const ZAP_TARGETS: usize = 3;
/// Burning: seconds it lasts and seconds between its bites.
const BURN_SECONDS: f64 = 3.0;
const BURN_TICK: f64 = 0.75;
/// Freeze-frame on a blow to the boss.
const HITSTOP_BOSS: f64 = 0.035;
const COLLISION_RADIUS_SQ: f64 = 0.45 * 0.45;

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
    Play,
    Resume,
    QuitToTitle,
    /// Leave the game entirely. Desktop only: a browser tab cannot close itself.
    Quit,
}

impl MenuItem {
    pub fn label(self) -> &'static str {
        match self {
            MenuItem::Play => "START",
            MenuItem::Resume => "RESUME",
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
    victory_timer: Option<f64>,
    /// Coins to spend at the blacksmith.
    pub purse: u32,
    /// Weapons he owns and the one he has equipped.
    pub arsenal: Arsenal,
    /// Potions carried on his belt, and the one Q (or RB) drinks.
    pub belt: Vec<PotionKind>,
    pub belt_index: usize,
    pub shop: ShopState,
    /// Seconds the boss has left burning, and until its next bite.
    boss_burn: f64,
    burn_tick: f64,
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
            global_mode: GlobalMode::Scatter,
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
            victory_timer: None,
            purse: 0,
            arsenal: Arsenal::default(),
            belt: Vec::new(),
            belt_index: 0,
            shop: ShopState::default(),
            boss_burn: 0.0,
            burn_tick: 0.0,
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
            keep.extend(l.weapon_shrines.iter().chain(&l.item_shrines).copied());
            keep.extend(l.potions.iter().map(|(p, _)| *p));
            keep.extend(l.gate);
            self.garden = crate::logic::garden::plan(&l.grid, &keep);
            if let Some(plan) = &self.garden {
                crate::logic::garden::apply(&mut self.level.grid, plan);
            }
        }
        self.keys_total = self.level.grid.count(Tile::Nodule) as u32;
        self.keys_got = 0;
        self.gate = self.level.gate;
        self.gate_open = false;
        self.door = 0.0;
        self.door_opening = false;
        self.gate_hint = 0.0;
        self.audio.set_tempo_step(0);

        let c = self.level.nest_center;
        self.nest_slots = vec![self.level.enemy_spawn, P::new(c.x - 2, c.y), P::new(c.x, c.y), P::new(c.x + 2, c.y)];

        // Clearing a dungeon keeps the held weapon and its ammo; only death takes them.
        let carried = if self.hero.alive && self.hero.holding_pickup() { Some((self.hero.weapon, self.hero.ammo)) } else { None };
        // His shield, dents and all, goes with him.
        let shield = self.hero.shield;
        self.hero = Hero::new(self.level.hero_spawn, self.level.data.hero_speed);
        self.hero.shield = shield;
        self.hero.arm(&self.arsenal);
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
        self.victory_timer = None;
        self.diamonds = self
            .level
            .weapon_shrines
            .iter()
            .enumerate()
            .map(|(i, &t)| Diamond::new(DiamondKind::Weapon, t, i))
            .chain(self.level.item_shrines.iter().enumerate().map(|(i, &t)| Diamond::new(DiamondKind::Item, t, i)))
            .collect();
        self.potions = self.level.potions.iter().map(|&(t, k)| Potion::new(t, k)).collect();
        if self.level.data.boss {
            // No bottles in a boss arena: bring your own.
            self.potions.clear();
        }
        if index >= 1 {
            // From the second dungeon on, gear only comes from the blacksmith.
            self.potions.clear();
            self.diamonds.clear();
        }
        self.boss_burn = 0.0;
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
        self.hero.facing = self.start_facing;
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
        self.new_kit();
        self.hero.respawn(self.level.hero_spawn); // a new game starts with the bare axe
        self.load_level(0);
        self.juice.score_shown = 0.0;
        if self.tour_pending {
            self.begin_tour();
        } else {
            self.enter(Screen::Ready);
        }
    }

    /// A fresh adventure: the axe, a round shield, an empty purse, one health potion.
    fn new_kit(&mut self) {
        self.arsenal = Arsenal::default();
        self.purse = 0;
        self.belt = vec![PotionKind::Health];
        self.belt_index = 0;
        self.hero.shield = Some(ShieldGear::new(ShieldKind::Round));
        self.hero.arm(&self.arsenal);
    }

    fn enter(&mut self, screen: Screen) {
        self.screen = screen;
        self.menu_index = 0;
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
            Screen::Title => match self.menu_choice() {
                Some(MenuItem::Quit) => self.quit_requested = true,
                // Play (or a tap anywhere else on the title) starts the story, then the character screen.
                Some(_) => self.play_from_title(),
                None if self.input.was_pressed(&[Action::Tap]) => self.play_from_title(),
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
                    self.juice.fade = 1.0;
                    self.enter(Screen::Ready);
                }
            }
            Screen::Won => {
                // The boss is down: off to the blacksmith.
                if self.input.was_pressed(&[Action::Confirm, Action::Tap]) {
                    self.open_shop();
                    self.juice.fade = 1.0;
                    self.input.consume(Action::Attack);
                    self.input.consume(Action::Confirm);
                }
            }
            Screen::Shop => self.update_shop(dt),
            Screen::Tour => self.update_tour(dt),
            Screen::GameOver => {
                if self.input.was_pressed(&[Action::Confirm, Action::Tap]) {
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
            gate: self.gate.map(|tile| crate::render::view::GateView { tile, keys: self.keys_got, total: self.keys_total, door: self.door as f32 }),
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
            Screen::Title => vec![MenuItem::Play],
            Screen::Paused => vec![MenuItem::Resume, MenuItem::QuitToTitle],
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
        let rects = crate::render::hud::menu_rects(self.screen, items.len(), self.viewport.0, self.viewport.1);
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
            gate: self.gate.map(|tile| crate::render::view::GateView { tile, keys: self.keys_got, total: self.keys_total, door: self.door as f32 }),
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
        if self.input.was_pressed(&[Action::Fullscreen]) {
            self.want_fullscreen_toggle = true;
        }
        if self.input.was_pressed(&[Action::Mute]) {
            self.audio.unlock();
            self.audio.toggle_mute();
        }
        if self.input.was_pressed(&[Action::Pause]) {
            if self.screen == Screen::Playing {
                self.enter(Screen::Paused);
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
        // The chase camera swings round behind him only while he runs straight ahead; running
        // diagonally, sideways or back it holds still, so each of the eight directions stays put.
        self.view.follow_heading = ry < -0.99;
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

        if self.input.was_pressed(&[Action::Attack, Action::Tap]) {
            self.attack();
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
        if self.global_mode == GlobalMode::Scatter && self.rage() >= RAGE_NO_SCATTER {
            // Half the keys gone: no more breathers, they hunt from here on.
            self.mode_timer = 0.0;
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
        self.juice.banner = Some(Banner {
            text: "THE GATE IS UNLOCKED".into(),
            sub: "FIND IT IN THE NORTH WALL AND GO THROUGH".into(),
            color: 0xf2c53d,
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
            self.floater(g.x as f64, g.y as f64 + 0.6, format!("LOCKED  {}/{} KEYS", self.keys_got, self.keys_total), 1.6, Some(0xf2c53d));
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
            self.win();
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
        self.hero.facing = Dir::Up;
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

    /// 0: straight to the blacksmith, from anywhere, with coins to test with.
    pub fn cheat_to_shop(&mut self) {
        self.audio.unlock();
        self.tour = None;
        if matches!(self.screen, Screen::Title | Screen::GameOver) {
            self.score.reset();
            self.juice.score_shown = 0.0;
            self.new_kit();
            self.load_level(0);
        }
        self.purse = self.purse.max(CHEAT_COINS);
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
        if self.boss_burn > 0.0 {
            self.boss_burn -= dt;
            self.burn_tick -= dt;
            if let Some(bp) = self.boss.as_ref().filter(|b| b.active()).map(|b| b.pos) {
                if self.burn_tick <= 0.0 {
                    self.burn_tick = BURN_TICK;
                    self.wound_boss(1, bp);
                }
                if self.rng.next() < dt * 20.0 {
                    self.view.sparkle(&self.level.grid, bp.x, bp.y, 1.0 + self.rng.next() as f32, 0xff7a2a, 2);
                }
            } else {
                self.boss_burn = 0.0;
            }
        }
        // Touching it hurts, unless he sails over it.
        if let Some(b) = &self.boss {
            let near = dist_sq(b.pos, hero_pos) < CONTACT_RADIUS * CONTACT_RADIUS;
            if b.harmful() && b.hover < 1.3 && near && !self.hero.airborne() && !self.hero.invisible() {
                self.hurt_hero(CONTACT_DAMAGE);
            }
        }
        if let Some(t) = &mut self.victory_timer {
            *t -= dt;
            if *t <= 0.0 {
                self.victory_timer = None;
                self.audio.play(Sfx::Win);
                self.enter(Screen::Won);
            }
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
                self.floater(p.x, p.y - 0.8, format!("+{bounty} COINS"), 2.2, Some(0xffd84a));
                self.floater(p.x, p.y, format!("+{POINTS_CYCLOPS}"), 2.0, Some(0xf2c53d));
                self.events.push(GameEvent::BossSlain);
                self.victory_timer = Some(VICTORY_DELAY);
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
            self.hurt_hero(hurt);
        }
    }

    /// A blow to the Cyclops from any weapon, with all the feedback it deserves.
    fn wound_boss(&mut self, amount: i32, at: V) -> bool {
        let mut events = std::mem::take(&mut self.boss_events);
        let landed = self.boss.as_mut().map(|b| b.hit(amount, &mut events)).unwrap_or(false);
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
            let d = &mut self.diamonds[i];
            if !d.active || d.tile != o {
                continue;
            }
            let chosen = d.take();
            match chosen {
                Offer::Weapon(w) => self.hero.equip(w),
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
            let def = self.potions[i].def();
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
            if d.active && tile_in_reach(hp, self.hero.facing, d.tile) {
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

    /// Sword swing: slays every targetable enemy in front of the hero.
    /// An axe swing: its level sets the reach, the arc and the blow.
    fn perform_attack(&mut self) {
        self.audio.play(Sfx::Slash);
        self.view.sword_slash();
        let stats = gear::axe(self.hero.axe_level);
        let hits = find_axe_targets(self.hero.pos(), self.hero.facing, &self.enemies, &stats);
        if !hits.is_empty() {
            self.view.shake(0.14 + 0.04 * self.hero.axe_level as f32);
        }
        for i in hits {
            self.slay_enemy(i);
        }
        if let Some(bp) = self.boss_target() {
            if in_axe_arc(self.hero.pos(), self.hero.facing, bp, &stats, BODY_RADIUS) {
                self.wound_boss(stats.damage * self.hero.might(), bp);
            }
        }
        self.zap();
    }

    /**
     * A lightning potion at work: every attack also arcs to the nearest few
     * foes around him (and the boss, if it is close).
     */
    fn zap(&mut self) {
        if self.hero.fx.lightning <= 0.0 {
            return;
        }
        let hp = self.hero.pos();
        let mut near: Vec<(f64, usize)> = self
            .enemies
            .iter()
            .enumerate()
            .filter(|(_, e)| e.is_targetable())
            .map(|(i, e)| (dist_sq(e.pos(), hp), i))
            .filter(|&(d, _)| d <= ZAP_RANGE * ZAP_RANGE)
            .collect();
        near.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let mut struck = false;
        for &(_, i) in near.iter().take(ZAP_TARGETS) {
            let p = self.enemies[i].pos();
            self.view.zap(&self.level.grid, hp, p);
            self.slay_enemy(i);
            struck = true;
        }
        if let Some(bp) = self.boss_target() {
            if dist_sq(bp, hp) <= (ZAP_RANGE + 1.0).powi(2) {
                self.view.zap(&self.level.grid, hp, bp);
                self.wound_boss(self.hero.might(), bp);
                struck = true;
            }
        }
        if struck {
            self.audio.play(Sfx::Zap);
        }
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

    /// Coins in the purse, and the counter jumps.
    fn earn(&mut self, coins: u32) {
        self.purse += coins;
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
        let def = gear::potion(kind);
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
        let shots = spawn_shots(kind, self.hero.pos(), self.hero.facing);
        self.projectiles.extend(shots);
        self.audio.play(Sfx::Arrow);
        if !self.hero.holding_pickup() && self.hero.weapon != crate::logic::items::WeaponKind::Ranged(kind) {
            // The shrine weapon ran dry; his own is back in hand.
            let hp = self.hero.pos();
            let (name, color) = crate::logic::items::weapon_label(self.hero.weapon, self.hero.axe_level);
            self.floater(hp.x, hp.y, name.to_uppercase(), 1.2, Some(color));
        }
        self.zap();
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
                for e in hits {
                    self.slay_enemy(e);
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
                        let dmg = p.def.damage * self.hero.might();
                        let burns = p.def.burn;
                        let at = p.pos;
                        self.projectiles[i].struck.insert(usize::MAX);
                        if !self.projectiles[i].def.pierce {
                            self.projectiles[i].alive = false;
                        }
                        if self.wound_boss(dmg, at) && burns {
                            if self.boss_burn <= 0.0 {
                                self.burn_tick = BURN_TICK;
                            }
                            self.boss_burn = BURN_SECONDS;
                        }
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
        self.bombs.push(Bomb::new(at));
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
            if self.hurt_hero(CONTACT_DAMAGE) {
                return;
            }
        }
    }

    /// Applies damage with shield and invincibility rules. Returns true if the hero died.
    fn hurt_hero(&mut self, amount: i32) -> bool {
        let result = self.hero.take_damage(amount);
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

    /// Capture hook: own and equip weapon `line` at `level`.
    pub fn debug_arm(&mut self, line: gear::Line, level: u8) {
        self.arsenal.levels[line.index()] = level.clamp(1, 3);
        self.arsenal.equipped = line;
        self.hero.arm(&self.arsenal);
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
        .weapon_shrines
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
    fn the_character_screen_offers_three_heroes_but_only_ragnar_can_begin() {
        use crate::logic::heroes::HeroKind;
        let mut g = Game::new();
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Select);
        assert_eq!(g.select_hero(), HeroKind::Ragnar, "Ragnar is highlighted first");
        // Mehmed and Miyamoto are locked: picking them is refused.
        for (dir, hero) in [(Dir::Left, HeroKind::Mehmed), (Dir::Right, HeroKind::Ragnar), (Dir::Right, HeroKind::Miyamoto)] {
            g.input.nav = Some(dir);
            g.update(crate::app::STEP);
            assert_eq!(g.select_hero(), hero);
            g.input.inject(Action::Confirm);
            g.update(crate::app::STEP);
            if hero == HeroKind::Ragnar {
                assert_eq!(g.screen, Screen::Ready, "Ragnar begins");
                g.enter(Screen::Title);
                g.open_select();
                g.select_index = HeroKind::Ragnar.slot();
            } else {
                assert_eq!(g.screen, Screen::Select, "{hero:?} is locked");
                assert!(g.select_denied > 0.0);
            }
        }
        assert_eq!(g.hero_kind, HeroKind::Ragnar);
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
        // Half the keys: no more scattering, shorter waits, more foes allowed.
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
        assert_eq!(g.menu_items(), vec![MenuItem::Play, MenuItem::Quit]);
        // Right, then Enter, on the title picks Quit.
        g.input.key_down(winit::keyboard::KeyCode::ArrowRight, false);
        g.update(crate::app::STEP);
        g.input.key_up(winit::keyboard::KeyCode::ArrowRight);
        g.input.key_down(winit::keyboard::KeyCode::Enter, false);
        g.update(crate::app::STEP);
        assert!(g.quit_requested);
        assert_eq!(g.screen, Screen::Title);

        // Pause offers Resume / Quit to Title / Quit Game; clicking a button picks it.
        let mut g = playing();
        g.input.inject(Action::Pause);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Paused);
        let r = crate::render::hud::menu_rects(Screen::Paused, 3, g.viewport.0, g.viewport.1)[1];
        g.input.cursor_moved_to(r[0] + r[2] / 2.0, r[1] + r[3] / 2.0);
        g.input.mouse_button(winit::event::MouseButton::Left, true);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Title);
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
        for _ in 0..(6.0 / crate::app::STEP) as usize {
            g.update(crate::app::STEP);
            g.hero.invuln = 1.0;
        }
        assert_eq!(g.screen, Screen::Won);
        assert!(g.events.contains(&GameEvent::BossSlain));
        assert!(g.purse >= COINS_BOSS, "the boss pays out");
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Shop, "the blacksmith comes after the boss");
        // Leave the forge: on to dungeon II, where only the smith sells gear.
        g.input.inject(Action::Pause);
        g.update(crate::app::STEP);
        g.input.inject(Action::Pause);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Ready);
        assert!(!g.level.data.boss);
        assert_eq!(g.dungeon, 1);
        assert!(g.potions.is_empty() && g.diamonds.is_empty(), "no free gear after the first dungeon");
    }

    #[test]
    fn the_blacksmith_sells_upgrades_and_equips() {
        let mut g = playing();
        g.open_shop();
        g.purse = 2000;
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
        use crate::logic::gear::Line;
        press(&mut g, ShopAction::Buy(Line::Knife));
        assert_eq!(g.arsenal.level(Line::Knife), 1);
        assert_eq!(g.hero.main, crate::logic::items::WeaponKind::Ranged(crate::logic::items::RangedKind::Knife), "bought weapons go straight to hand");
        press(&mut g, ShopAction::Upgrade(Line::Knife));
        press(&mut g, ShopAction::Upgrade(Line::Knife));
        assert_eq!(g.arsenal.level(Line::Knife), 3);
        assert_eq!(g.hero.weapon, crate::logic::items::WeaponKind::Ranged(crate::logic::items::RangedKind::SeekingKnives));
        press(&mut g, ShopAction::Equip(Line::Axe));
        assert_eq!(g.hero.weapon, crate::logic::items::WeaponKind::Sword);
        press(&mut g, ShopAction::Upgrade(Line::Axe));
        assert_eq!(g.hero.axe_level, 2);
        let spent = 2000 - g.purse;
        assert_eq!(spent, Line::Knife.price() + Line::Knife.upgrade_price(1) + Line::Knife.upgrade_price(2) + Line::Axe.upgrade_price(1));
        g.shop.tab = ShopTab::Shields;
        press(&mut g, ShopAction::BuyShield(ShieldKind::Kite));
        assert_eq!(g.hero.shield.unwrap().kind, ShieldKind::Kite);
        g.shop.tab = ShopTab::Potions;
        let before = g.belt.len();
        press(&mut g, ShopAction::Potion(PotionKind::Strength));
        assert_eq!(g.belt.len(), before + 1);
        // Too poor: nothing changes hands.
        g.purse = 10;
        press(&mut g, ShopAction::Potion(PotionKind::Invulnerability));
        assert_eq!(g.purse, 10);
        assert_eq!(g.belt.len(), before + 1);
    }

    #[test]
    fn zero_opens_the_shop_from_anywhere() {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        let mut g = Game::new();
        g.input.key_down(winit::keyboard::KeyCode::Digit0, false);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Shop);
        assert!(g.purse >= CHEAT_COINS);
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
        g.hero.shield = Some(ShieldGear::new(ShieldKind::Round));
        let hp = g.hero.hp;
        g.debug_hurt(); // 25: the round shield takes half
        assert!(g.hero.hp < hp && g.hero.hp > hp - 25, "shield took no share: {}", g.hero.hp);
        let left = g.hero.shield.unwrap().durability;
        assert!(left < gear::shield(ShieldKind::Round).durability);
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
        assert_eq!(g.hero.hp, 90);
        assert_eq!(g.belt, vec![PotionKind::Strength]);
        g.input.inject(Action::Drink);
        g.update(crate::app::STEP);
        assert!(g.belt.is_empty());
        assert!(g.hero.fx.strength > 0.0);
        assert_eq!(g.hero.might(), 2);
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
