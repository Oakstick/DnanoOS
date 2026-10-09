//! The game: screens, the round, and every rule that ties the entities together.
//!
//! `Game` holds the whole state of a run. This file defines it, builds it, and runs one tick of
//! play; each submodule adds the methods for one part of the rules:
//!
//! - `flow`: the frame loop, the screens and menus, the keys that work everywhere
//! - `level`: loading a level and starting a round
//! - `movement`: turning input into movement, footsteps
//! - `combat`, `boss`, `foes`, `health`: blows, the boss fight, the foes, the hero's health
//! - `brawl`, `sparring`: fighting by frame data (`logic::combat`, COMBAT.md) in the Vaults of
//!   Mammon and the gym's Sparring Ring, and the ring's bouts
//! - `pickups`, `gate`: what he picks up, the gate that ends a level
//! - `feedback`: floating text, points, streaks and the screen's juice
//! - `angels`, `hub`, `realm`, `vault`, `shop`, `map`, `story`, `tour`, `select`, `controls`,
//!   `save`: the archangels, the Portals of Hell, the realms, the Vaults of Mammon, the old man's
//!   wares, the map, Scene I, the first-run tour, the character screen, the controller page, saves;
//!   `settings_page`: the settings (UI.md §8) and the rebinding of the controls
//! - `pit`: the Pit under the House of the Devil, ridden down level by level (FINALE.md)
//! - `debug`: testing cheats and the capture tool's hooks
//! - `ending`: the end, Scene II, the words and the credits (FINALE.md §4)

mod angels;
pub mod bestiary;
mod boss;
pub mod bot;
pub mod brawl;
mod combat;
mod controls;
pub mod counsel;
mod court;
mod debug;
pub mod deceit;
pub mod ending;
mod feedback;
mod feel;
mod flow;
mod foes;
pub mod fullmap;
mod gate;
pub mod general;
mod guide;
mod health;
mod hub;
mod juice;
#[cfg(test)]
mod juice_tests;
pub mod lair;
pub mod ledger;
mod level;
mod map;
pub mod mastery;
mod movement;
mod packs;
mod persist;
mod pickups;
pub mod pit;
mod preload;
pub mod qa;
#[cfg(test)]
mod qa_tests;
mod realm;
mod realms;
#[cfg(test)]
mod ring_tests;
mod rivals;
mod save;
mod select;
pub mod settings_page;
mod shop;
pub mod sparring;
mod story;
#[cfg(test)]
mod tests;
mod tour;
mod vault;
mod viewpick;
pub mod war;
pub mod whole_run;
pub mod workshop;

pub use combat::Spirit;
pub use court::PrinceBar;
pub use feedback::{
    streak_tier, Banner, Floater, FlyingCoin, Juice, BLESSED_SECONDS, BRUTAL_STREAK, COIN_FLIGHT_SECONDS, DOUBLE_POINTS_SECONDS,
    STREAK_CALLS,
};
pub use juice::{Feel, JUICE_EVENTS, SHAKE_STEPS};
pub use level::center_spawn;
pub use map::map_stops;
pub use realms::{Meter, Offer};
pub use save::Stage;
pub use shop::{RowKind, ShopAction, ShopButton, ShopRow, ShopState};
pub use story::{Story, STORY_FADE, TYPE_DELAY};
pub use tour::{Tour, TourArt, TourStop, TOUR_STOP_SECONDS};
pub use vault::Hoard;
pub use workshop::{GeneralCard, QuitmerMood, Station, Trophy, WorkshopData, WorkshopView};

use crate::audio::{AudioManager, Sfx};
use crate::input::{Action, Input};
use crate::logic::boss::{BossEvent, BossShot, Cyclops};
use crate::logic::enemy::{Enemy, EnemyContext, EnemyKind, GlobalMode, HeroView};
use crate::logic::gear::BELT_SLOTS;
use crate::logic::hero::Hero;
use crate::logic::items::{Bomb, Diamond, Potion, BOMBS_PER_LIFE};
use crate::logic::level::{parse_level, ParsedLevel, Theme};
use crate::logic::levels::level_for;
use crate::logic::progression::Progress;
use crate::logic::projectile::Projectile;
use crate::logic::rng::Rng;
use crate::logic::systems::{Frenzy, PuddleField, Score};
use crate::logic::types::{Dir, PotionKind, P, V};
use crate::render::profile::{Lap, SimTimes};
use crate::render::view::{CameraMode, WorldView};
use std::collections::HashMap;

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
    /// Before the far portal of the Vaults of Mammon: drop everything, or keep it?
    Toll,
    /// Just before play: the camera from above, or over his shoulder?
    View,
    GameOver,
}

/// Reinforcements: the first arrives after this long, then one every interval, up to the cap.
const SPAWN_FIRST: f64 = 20.0;
/// Seconds a wounded foe staggers, harmless, before it comes on again.
const WOUND_STAGGER: f64 = 0.8;
/// The music quickens one step for every this many keys.
pub const KEYS_PER_TEMPO_STEP: u32 = 2;
/// Seconds the gate's doors take to swing open.
const DOOR_SECONDS: f64 = 1.6;

/// The foes' schedule: they hunt the hero from the start and never break off to
/// wander to their corners (as the arcade ghosts did). Frozen while they are stunned.
const MODE_SCHEDULE: [(GlobalMode, f64); 1] = [(GlobalMode::Chase, f64::INFINITY)];

/// The beat before play: READY, then GO!
pub const READY_SECONDS: f64 = 2.0;

/// A build for developing: a debug build, or a release build `tools/release.sh` makes for the
/// Desktop (`VH_DEV_MENU`). The distributables (`build-windows.ps1`, the web) are not: there the
/// testing keys do nothing (the user's call, 2026-10-09: F1 is the help key on many keyboards).
pub const DEV_BUILD: bool = cfg!(debug_assertions) || option_env!("VH_DEV_MENU").is_some();

/// The fanfare and the fade into the boss arena.
pub const CLEARED_SECONDS: f64 = 3.4;
/// The Cyclops' red diamond key, on the HUD and the gate's lock lamp.
const RED_KEY_COLOR: u32 = 0xff2a44;
/// The shop cheat tops the purse up to this, so there is something to spend.
const CHEAT_COINS: u32 = 1000;
/// Bleeding (a tier-8 weapon) and burning: seconds between their bites.
const BLEED_TICK: f64 = 0.5;
/// Venom: seconds between its bites.
const POISON_TICK: f64 = 1.0;
/// Flames on the Cyclops (Kusanagi's fragment, the Simurg bow) and a greatsword's holy fire.
const FIRE_COLOR: u32 = 0xff8a2a;

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
    /// The full map of the level (the pause menu's).
    Map,
    /// The developer's map: jump to any stop on the road (testing; on the title and the pause
    /// menu in developer builds, `--dev`, or once F12 has opened it).
    DevMap,
    /// The Ledger of Grudges: the run's rivals, on their board.
    Ledger,
    /// The bestiary: the old man's book of every foe met (`bestiary`).
    Bestiary,
    Resume,
    /// The gatekeeper's promise: call him, and his wares open wherever you are.
    CallOldMan,
    QuitToTitle,
    /// Leave the game entirely. Desktop only: a browser tab cannot close itself.
    Quit,
    /// Mammon's toll: keep the gold (he promises a chance; there is none, and the vaults begin
    /// again), or leave it and go through.
    KeepAll,
    DropAll,
    /// An answer to a prince's offer in one of the realms after the vaults, in its own words.
    Choice(&'static str),
    /// The camera: from above, or over his shoulder (the choice before play).
    ViewTop,
    ViewFollow,
    /// The settings page (UI.md §8), from the title and the pause menu: the camera, slow motion
    /// and the shake live there now, with everything else.
    Settings,
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
            MenuItem::DevMap => "LEVELS",
            MenuItem::Ledger => "THE LEDGER",
            MenuItem::Bestiary => "THE BESTIARY",
            MenuItem::Resume => "RESUME",
            MenuItem::CallOldMan => "CALL THE OLD MAN",
            MenuItem::QuitToTitle => "QUIT TO TITLE",
            MenuItem::Quit => "QUIT GAME",
            MenuItem::KeepAll => "KEEP THE GOLD",
            MenuItem::DropAll => "LEAVE IT",
            MenuItem::Choice(words) => words,
            MenuItem::ViewTop => "VIEW: TOP-DOWN",
            MenuItem::ViewFollow => "VIEW: FOLLOW",
            MenuItem::Settings => "SETTINGS",
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

/// A level being made ready behind the loading screen (RP03): the world waits, black, until the
/// renderer has its scenery up and its light worked out, and then fades in on a finished frame.
#[derive(Clone, Debug)]
pub struct Loading {
    /// The level's name, shown on the loading screen.
    pub label: String,
    pub started: web_time::Instant,
    /// The load itself (building the level and its view), in ms.
    pub load_ms: f64,
    /// Seconds held so far.
    pub age: f64,
    /// The first time this level was loaded in this run of the game (nothing of it cached yet).
    pub cold: bool,
    /// Held at least this many seconds, ready or not (the loading card's preview, from the Map).
    pub hold: f64,
}

/// One change of level, timed (RP03): from the load to the first frame shown, in ms.
#[derive(Clone, Debug)]
pub struct Transition {
    pub label: String,
    pub cold: bool,
    pub load_ms: f64,
    pub ms: f64,
}

/// The loading screen gives up waiting after this long (a renderer that never reports ready).
pub const LOADING_TIMEOUT: f64 = 10.0;

pub struct Game {
    /// Where the last step's time went (RP06).
    pub sim: SimTimes,
    /// A level being made ready (RP03), while the screen is held.
    pub loading: Option<Loading>,
    /// Hold each new level until the renderer has it ready (`finish_loading`). The game and the
    /// performance harness set it; tests and captures, with no renderer between steps, do not.
    pub await_renderer: bool,
    /// Every change of level and how long it took to show (RP03).
    pub transitions: Vec<Transition>,
    /// Levels the renderer has made ready so far in this run, for telling a cold load from a warm one.
    visited: std::collections::HashSet<String>,
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
    /// F5 (testing cheat): foes still strike him, but his health always comes back and he never dies.
    pub immortal: bool,
    /// The renderer's own readout under the frame rate (F3): CPU and GPU times, counts.
    pub perf_lines: Vec<String>,
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
    /// The rumbles the fight asks for since the app last played them (presentation only: nothing
    /// in the simulation reads them; a few at most are kept).
    pub rumbles: Vec<crate::input::Rumble>,
    /// Set when the player asks for fullscreen; the app layer applies it.
    pub want_fullscreen_toggle: bool,
    /// The highlighted menu button.
    pub menu_index: usize,
    /// The player chose Quit Game; the app layer closes the window.
    pub quit_requested: bool,
    /// Logical window size, for hit-testing menu buttons.
    pub viewport: (f32, f32),
    pub juice: Juice,
    /// The old man's guide is open over the pause (B).
    pub guide_open: bool,
    /// The war log and what reads it (WARBAND.md).
    pub war: war::War,
    /// The run's Warband as saved (the Ledger of Grudges), its name card and its board.
    pub warband: ledger::Warband,
    /// The bestiary: the book of every foe met, as saved, and its page while it is open.
    pub bestiary: bestiary::Bestiary,
    /// The two voices: the old man's counsel, the line in the panel, the prince's cheats.
    pub voices: counsel::Voices,
    /// The heroes' mastery in the fight, its toast and its trials (WARBAND.md §11.1).
    pub mastery: mastery::MasteryRun,
    /// Seconds of freeze-frame left.
    hitstop: f64,
    /// The freezes' budget, the slowing of time and the pace (`juice`); the feel's settings; and
    /// the juice switched off (`render::profile::fx` bits: the harnesses run without it).
    tempo: juice::Tempo,
    pub feel: juice::Feel,
    pub juice_off: u32,
    kill_streak: u32,
    /// The last streak call shouted, so the next is another.
    last_call: &'static str,
    streak_timer: f64,
    step_dist: f64,
    was_airborne: bool,
    /// The way Ragnar faces at spawn: down the longest corridor from the center.
    start_facing: Dir,
    /// The dungeon number this stretch of the game belongs to (its boss shares it).
    pub dungeon: usize,
    /// The song this level plays, drawn as it is loaded (and kept through its retries).
    level_track: Option<crate::audio::Soundtrack>,
    /// The boss of a boss arena, and what it has thrown.
    pub boss: Option<Cyclops>,
    pub boss_shots: Vec<BossShot>,
    boss_events: Vec<BossEvent>,
    /// Where the Cyclops' red diamond key lies, until he takes it.
    pub red_key: Option<P>,
    /// Silver, rune fragments and every hero's unlocked trees: kept between adventures.
    pub progress: Progress,
    /// Potions carried on his belt, and the one Q (or RB) drinks.
    pub belt: Vec<PotionKind>,
    pub belt_index: usize,
    pub shop: ShopState,
    /// What the old man's workshop shows of the war below (trophies, news, the generals known).
    pub workshop: WorkshopData,
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
    /// After the camera is chosen: the tour (or else the countdown).
    view_then_tour: bool,
    /// The camera in use when the choice opened (Back keeps it).
    view_before: crate::render::view::CameraMode,
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
    /// The run's save (the checkpoint, the purse, the Ledger) as one snapshot: marked, written at
    /// a calm moment (`persist`).
    pub persist: persist::Persist,
    /// The title is asking whether to wipe everything and start over.
    pub confirm_reset: bool,
    /// The controller page is open over the title or the pause menu.
    pub show_controls: bool,
    /// The map page is open over the title or the pause menu; which stop is highlighted.
    pub show_map: bool,
    pub map_index: usize,
    /// The minimap's settings and the full map while it is open (`fullmap`).
    pub map_ui: fullmap::MapUi,
    /// Developer builds, `--dev`, or F12 pressed once (where the testing keys work): the
    /// developer's map on the title and the pause menu.
    pub dev: bool,
    /// The testing keys work (F1, F2, F5-F10, F12, 0): in developer builds only (`DEV_BUILD`), or
    /// with `--dev` and the harnesses; inert in the distributables.
    pub testing_keys: bool,
    /// The archangel's blessing he carries (from dungeon to dungeon), if any.
    pub blessing: Option<crate::logic::angels::Blessing>,
    /// Seconds of play until Uriel grants his blessing again; when the last refusal was shown.
    pub uriel_rest: f64,
    refused_at: f64,
    /// Seconds until his patron archangel's power is his to call again (0: ready).
    pub patron_wait: f64,
    /// Wear carried between whole points of a blade's edge.
    edge_carry: f64,
    /// The controls page shows the keyboard rather than the controller.
    pub controls_keyboard: bool,
    /// Uriel's wave while it rolls out.
    pub holy_wave: Option<angels::HolyWave>,
    /// Which archangels have given their blessing in this level (each gives once).
    pub angels_used: [bool; 4],
    /// The sins whose keys he holds (one bit each, `Sin::bit`).
    pub sins_cleared: u8,
    /// The Portals of Hell, while he is there.
    pub hub: Option<hub::HubState>,
    /// The sin whose realm, or whose prince's court, he is in.
    pub realm: Option<crate::logic::sins::Sin>,
    /// The Vaults of Mammon, while he is in them: their rooms, packs and gold.
    pub vault: Option<crate::logic::vault::Vault>,
    /// One of the six realms after the vaults, while he is in it: its ground and its rules' state.
    pub run: Option<crate::logic::realms::Run>,
    /// Greed's court, the Counting House, while he is in it: its prince's fight (`court`).
    pub court: Option<court::Court>,
    /// Krampus's lair at the bottom of the Pit, while he is in it: its fight, the cage, the boy (`lair`).
    pub lair: Option<lair::Lair>,
    /// The general of the realm he is in, while he is in it (WARBAND.md §9, `general`).
    pub general: Option<general::GeneralRun>,
    /// The gold he carries out of the vaults, and whether Mammon has asked his toll for it.
    pub hoard: Hoard,
    toll_asked: bool,
    /// His belt and rune fragments as he came into the vaults (restored if the vaults close on him).
    vault_kit: (Vec<PotionKind>, u32),
    /// The camera as it was before the vaults turned it to the top view.
    camera_before_vault: Option<CameraMode>,
    /// The gym's Sparring Ring: its bouts (none elsewhere).
    pub spar: Option<sparring::Sparring>,
    /// Fighting by frame data: the hero's fighter, the threat manager, the bolts (in the gym and the vaults).
    pub brawl: Option<brawl::Brawl>,
    /// The game has been finished (the end has played, here or in an earlier run): the title shows it.
    pub finished: bool,
    /// The Pit, while he is in it: its level, its clock, its waves and what rode down (`pit`).
    pub pit: Option<pit::PitRun>,
    /// The fog of war over the level, for the minimap (UI.md §7): presentation only, nothing in
    /// the game reads it.
    pub explore: crate::logic::explore::Explore,
    /// The player's settings (UI.md §8: `crate::settings`), and the page that changes them.
    pub settings: crate::settings::Settings,
    pub settings_page: settings_page::SettingsPage,
    /// The window fills the screen (the app keeps it so, for the settings page to show).
    pub fullscreen: bool,
    /// The window has the player's attention: away (another window in front), play that comes
    /// on by itself (a countdown ending, a load done) waits on the pause menu (QA.md Q1).
    pub focused: bool,
    /// The device used last as last saved (`vh.last_device`, for the engine's start screen).
    device_saved: Option<crate::input::Device>,
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
            input: {
                let mut input = Input::new();
                input.load_settings();
                input
            },
            view: WorldView::new(),
            show_fps: false,
            immortal: false,
            sim: SimTimes::default(),
            loading: None,
            await_renderer: false,
            transitions: Vec::new(),
            visited: Default::default(),
            perf_lines: Vec::new(),
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
            rumbles: Vec::new(),
            war: war::War::default(),
            guide_open: false,
            warband: ledger::Warband::load(),
            bestiary: bestiary::Bestiary::load(),
            voices: counsel::Voices::default(),
            mastery: mastery::MasteryRun::default(),
            want_fullscreen_toggle: false,
            menu_index: 0,
            quit_requested: false,
            viewport: (1280.0, 720.0),
            juice: Juice::default(),
            hitstop: 0.0,
            tempo: juice::Tempo::default(),
            feel: juice::Feel::load(),
            juice_off: 0,
            kill_streak: 0,
            last_call: "",
            streak_timer: 0.0,
            step_dist: 0.0,
            was_airborne: false,
            start_facing: Dir::Down,
            dungeon: 0,
            level_track: None,
            boss: None,
            boss_shots: Vec::new(),
            boss_events: Vec::new(),
            red_key: None,
            // Tests start from nothing; the game from the saved progress.
            progress: if cfg!(test) { Progress::default() } else { persist::read_progress() },
            belt: Vec::new(),
            belt_index: 0,
            shop: ShopState::default(),
            workshop: WorkshopData::default(),
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
            view_then_tour: false,
            view_before: crate::render::view::CameraMode::Third,
            hero_kind: crate::logic::heroes::HeroKind::Ragnar,
            select_index: 1,
            select_age: 0.0,
            select_denied: 0.0,
            story: None,
            intro: !cfg!(test),
            saved_run: save::load_run(),
            persist: persist::Persist::default(),
            confirm_reset: false,
            show_controls: false,
            show_map: false,
            map_index: 0,
            map_ui: fullmap::MapUi::load(),
            // (Developing: debug builds, and the release builds `tools/release.sh` makes for the
            // Desktop (`VH_DEV_MENU`), not the distributables; F12 opens it where the testing keys
            // work.)
            dev: DEV_BUILD && !cfg!(test),
            // (The tests press them; `testing_keys_are_inert_outside_developer_builds` turns them off.)
            testing_keys: DEV_BUILD || cfg!(test),
            blessing: None,
            uriel_rest: 0.0,
            refused_at: -10.0,
            patron_wait: 0.0,
            edge_carry: 0.0,
            controls_keyboard: false,
            holy_wave: None,
            angels_used: [false; 4],
            sins_cleared: 0,
            hub: None,
            realm: None,
            vault: None,
            run: None,
            court: None,
            lair: None,
            general: None,
            hoard: Hoard::default(),
            toll_asked: false,
            vault_kit: (Vec::new(), 0),
            camera_before_vault: None,
            spar: None,
            brawl: None,
            finished: !cfg!(test) && ending::finished_saved(),
            pit: None,
            explore: crate::logic::explore::Explore::new(),
            settings: crate::settings::Settings::load(),
            settings_page: Default::default(),
            fullscreen: false,
            focused: true,
            device_saved: None,
        };
        // The settings into the input, the mixer, the HUD and the view.
        g.apply_settings();
        g.load_level(0);
        g.audio.set_soundtrack(Some(crate::audio::Soundtrack::Menu));
        // Free movement is on unless the player turned it off (tests keep to the grid).
        g.hero.free_move = !cfg!(test) && crate::platform::load("vh.free_move").map(|v| v != "0").unwrap_or(true);
        // The camera chosen last time.
        g.view.camera_mode = viewpick::saved_camera();
        g
    }

    /// How the level looks, and what lives there.
    /// A boss's arena: the Cyclops' meadow, or a prince's court.
    pub fn arena(&self) -> bool {
        self.level.data.boss
    }

    pub fn theme(&self) -> Theme {
        self.level.data.theme
    }

    /// Seeds the game's and the view's random generators, so a scripted run (a capture) plays the
    /// same every time.
    pub fn reseed(&mut self, seed: u32) {
        self.rng = Rng::new(seed);
        self.view.reseed(seed.wrapping_add(1));
    }

    /// The old man's guide (B) and the belt (turned, drunk from): in play, and while the Pit's
    /// platform goes down. True when the guide opened (the tick ends there).
    fn guide_and_belt(&mut self) -> bool {
        if self.input.was_pressed(&[Action::Guide]) && self.screen == Screen::Playing {
            self.open_guide();
            return true;
        }
        // The wheel or the D-pad's left and right: the potion before or after on the belt.
        let turn = self.input.was_pressed(&[Action::PotionNext]) as usize
            + self.belt.len().saturating_sub(1) * self.input.was_pressed(&[Action::PotionPrev]) as usize;
        if turn > 0 && !self.belt.is_empty() {
            self.belt_index = (self.belt_index + turn) % self.belt.len();
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
        false
    }

    fn tick(&mut self, dt: f64) {
        // The Pit's platform going down: nothing fights, nothing walks.
        if self.pit_holds(dt) {
            return;
        }
        let mut lap = Lap::start();
        self.update_mode_schedule(dt);

        let wanted = self.resolve_wanted_dir();
        self.hero.wish = self.free_wish();
        // A realm may take the controls (a charm mirrors them, a berserk rage aims them).
        let wanted = self.steer_run(wanted);
        // LB or Shift held: the shield is raised (if he has one, and is on his feet). Where fights go by
        // frame data (the Sparring Ring, the vaults) the guard is his fighter's, and so are his attack
        // and evade; a shrine weapon from a vault chest still fires as it always has.
        let guard = self.input.held(Action::Block) && self.hero.alive && !self.hero.airborne();
        let ring = self.brawling();
        let swinging_fsm = ring && !self.hero.holding_pickup();
        if ring {
            let attack = swinging_fsm && self.input.was_pressed(&[Action::Attack, Action::Tap]);
            let evade = self.input.was_pressed(&[Action::Jump]);
            self.brawl_input(attack, evade, guard);
        } else {
            self.hero.blocking = guard && self.hero.shield.is_some();
            self.hero.root = 1.0;
        }
        let before = self.hero.pos();
        self.hero.update(dt, &self.level.grid, wanted);
        self.footsteps(before);
        // (The minimap's fog: it only looks, and nothing reads it.)
        self.explore.observe(&self.level.grid, self.hero.pos());
        self.collect();
        self.update_gate(dt);
        self.update_hub(dt);
        if self.screen != Screen::Playing {
            return; // through a portal: the realm waits for its countdown
        }
        self.visit_shrines();
        self.visit_potions();
        self.visit_relic_shrine();
        self.update_decoy(dt);
        self.update_spirits(dt);
        self.update_fires(dt);
        if !ring {
            // (Under the frame data the kit's second wind stands in for the World Tree.)
            self.watch_health();
        }

        // (Under the frame data: the sword, the guard and the evade; in the vaults bombs and the
        // archangels too, as blows through the foes' fighters. COMBAT.md §4.3, §9.)
        let vault_brawl = ring && self.brawl.as_ref().is_some_and(|b| b.field == brawl::Field::Vault);
        if !swinging_fsm && self.input.was_pressed(&[Action::Attack, Action::Tap]) {
            self.attack();
        }
        if !ring && self.input.was_pressed(&[Action::Step]) {
            self.bifrost_step();
        }
        if (!ring || vault_brawl) && self.input.was_pressed(&[Action::Angel]) {
            self.invoke_blessing();
        }
        if self.hero.winded {
            self.hero.winded = false;
            let hp = self.hero.pos();
            if !self.floaters.iter().any(|f| f.text == "OUT OF BREATH") {
                self.floater(hp.x, hp.y - 0.6, "OUT OF BREATH", 0.9, Some(0x9ad1ff));
            }
        }
        if !ring && self.input.was_pressed(&[Action::Jump]) && self.hero.try_jump(&self.level.grid) {
            self.audio.play(Sfx::Jump);
        }
        if self.guide_and_belt() {
            return;
        }
        self.sim.hero += lap.lap();
        self.update_projectiles(dt);
        self.update_angels(dt);
        self.update_bombs(dt);
        for d in self.diamonds.iter_mut() {
            d.update(dt);
        }
        if self.screen != Screen::Playing {
            return;
        }
        self.sim.shots += lap.lap();
        if self.boss.is_some() {
            self.update_boss(dt);
            if self.screen != Screen::Playing {
                return;
            }
        }
        if self.court.is_some() {
            self.update_court(dt);
        }
        if self.lair.is_some() {
            self.update_lair(dt);
            if self.screen == Screen::Story {
                return; // the end has begun: the run is closed, nothing more of it is counted
            }
        }

        self.sim.boss += lap.lap();
        if self.frenzy.update(dt) {
            for e in self.enemies.iter_mut() {
                e.exit_frenzy(self.global_mode);
            }
            self.audio.set_frenzy(false);
        }

        let hero = HeroView { pos: self.hero.pos(), dir: self.hero.dir(), heard_pos: self.hero.heard_pos, heard_age: self.hero.heard_age };
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
        self.update_witches(dt);
        self.sim.foes += lap.lap();
        self.update_sparring(dt);
        self.update_brawl(dt);
        if self.screen != Screen::Playing {
            return; // slain in the vaults
        }
        self.update_gym(dt);
        self.update_vault(dt);
        self.update_run(dt);
        self.update_pit(dt);
        self.update_general(dt);
        self.sim.fights += lap.lap();
        self.update_spawns(dt);
        self.update_slime_trails(dt);
        self.sim.spawns += lap.lap();

        self.resolve_contact();
        self.sim.contact += lap.lap();
        if self.screen != Screen::Playing {
            return;
        }
        self.update_effects(dt);
        self.sim.effects += lap.lap();
    }
}

impl Game {
    /// A level has just been built: hold the screen until the renderer has it (RP03).
    pub(super) fn begin_loading(&mut self, label: String, started: web_time::Instant) {
        if !self.await_renderer {
            return;
        }
        let cold = !self.visited.contains(&label);
        let load_ms = started.elapsed().as_secs_f64() * 1e3;
        self.loading = Some(Loading { label, started, load_ms, age: 0.0, cold, hold: 0.0 });
        self.juice.fade = 1.0;
    }

    /// The renderer has the new level ready: the hold ends and the level fades in.
    pub fn finish_loading(&mut self) {
        if self.loading.as_ref().is_some_and(|l| l.age < l.hold) {
            return;
        }
        if let Some(l) = self.loading.take() {
            let ms = l.started.elapsed().as_secs_f64() * 1e3;
            self.visited.insert(l.label.clone());
            self.transitions.push(Transition { label: l.label, cold: l.cold, load_ms: l.load_ms, ms });
            self.juice.fade = 1.0;
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}
