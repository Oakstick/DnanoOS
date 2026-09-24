//! The game: screens, the round, and every rule that ties the entities together.

use std::collections::HashMap;
use std::f64::consts::PI;

use crate::audio::{AudioManager, Sfx};
use crate::input::{Action, Input};
use crate::logic::enemy::{default_enemy_specs, Enemy, EnemyContext, EnemyKind, EnemyMode, GlobalMode, HeroView};
use crate::logic::hero::{DamageResult, Hero, CONTACT_DAMAGE};
use crate::logic::items::{
    Bomb, Diamond, DiamondKind, ItemKind, Offer, Potion, BOMBS_PER_LIFE, INVISIBLE_SECONDS, SWORD_COLOR,
};
use crate::logic::level::{parse_level, ParsedLevel};
use crate::logic::levels::level_for;
use crate::logic::projectile::{spawn_shots, Projectile, ShotState};
use crate::logic::rng::Rng;
use crate::logic::systems::*;
use crate::logic::types::{dist_sq, manhattan, relative_dir, Dir, PotionKind, Tile, CARDINALS, P};
use crate::render::view::{dir_to_yaw, CameraMode, SceneState, WorldView};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Title,
    Ready,
    Playing,
    Paused,
    Dying,
    Won,
    GameOver,
}

/// Health a green cross gives back, capped at Popolon's maximum.
const REMEDY_HEAL: i32 = 50;
/// Reinforcements: the first arrives after this long, then one every interval, up to the cap.
const SPAWN_FIRST: f64 = 20.0;
const SPAWN_EVERY: f64 = 25.0;
const MAX_ENEMIES: usize = 12;
const SPAWN_CYCLE: [EnemyKind; 4] = [EnemyKind::Bat, EnemyKind::Slime, EnemyKind::Vampire, EnemyKind::Skeleton];
/// The music quickens one step for every this many nuggets.
pub const NUGGETS_PER_TEMPO_STEP: u32 = 5;

/// Alternating scatter/chase schedule in seconds. Frozen while enemies are stunned.
const MODE_SCHEDULE: [(GlobalMode, f64); 6] = [
    (GlobalMode::Scatter, 7.0),
    (GlobalMode::Chase, 20.0),
    (GlobalMode::Scatter, 7.0),
    (GlobalMode::Chase, 20.0),
    (GlobalMode::Scatter, 5.0),
    (GlobalMode::Chase, f64::INFINITY),
];

const READY_SECONDS: f64 = 2.2;
const DYING_SECONDS: f64 = 1.8;
const COLLISION_RADIUS_SQ: f64 = 0.45 * 0.45;

pub struct Floater {
    pub x: f64,
    pub y: f64,
    pub text: String,
    pub life: f64,
    pub color: Option<u32>,
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
            MenuItem::Play => "PLAY",
            MenuItem::Resume => "RESUME",
            MenuItem::QuitToTitle => "QUIT TO TITLE",
            MenuItem::Quit => "QUIT GAME",
        }
    }
}

/// Things the platform layer may want to hear about (Steam achievements).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameEvent {
    Won,
    FirstNugget,
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
    pub nuggets_total: u32,
    pub nuggets_got: u32,
    pub puddles: PuddleField,
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
    global_mode: GlobalMode,
    mode_index: usize,
    mode_timer: f64,
    screen_timer: f64,
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
            nuggets_total: 0,
            nuggets_got: 0,
            puddles: PuddleField::default(),
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
        };
        g.load_level(0);
        g
    }

    // ---------------------------------------------------------------- setup

    pub fn load_level(&mut self, index: usize) {
        self.score.level = index;
        self.level = parse_level(&level_for(index));
        self.nuggets_total = self.level.grid.count(Tile::Nodule) as u32;
        self.nuggets_got = 0;
        self.audio.set_tempo_step(0);

        let c = self.level.nest_center;
        self.nest_slots = vec![self.level.enemy_spawn, P::new(c.x - 2, c.y), P::new(c.x, c.y), P::new(c.x + 2, c.y)];

        // Clearing a dungeon keeps the held weapon and its ammo; only death takes them.
        let carried = if self.hero.alive && self.hero.has_ranged_weapon() { Some((self.hero.weapon, self.hero.ammo)) } else { None };
        self.hero = Hero::new(self.level.hero_spawn, self.level.data.hero_speed);
        let specs = default_enemy_specs(&self.level.grid);
        self.enemies = specs
            .iter()
            .enumerate()
            .map(|(i, s)| Enemy::new(s, self.nest_slots.get(i).copied().unwrap_or(c), self.level.data.enemy_speed))
            .collect();
        self.diamonds = self
            .level
            .weapon_shrines
            .iter()
            .enumerate()
            .map(|(i, &t)| Diamond::new(DiamondKind::Weapon, t, i))
            .chain(self.level.item_shrines.iter().enumerate().map(|(i, &t)| Diamond::new(DiamondKind::Item, t, i)))
            .collect();
        self.potions = self.level.potions.iter().map(|&(t, k)| Potion::new(t, k)).collect();
        self.spawn_timer = SPAWN_FIRST;
        self.spawned = 0;
        self.view.build_level(&self.level.grid, &self.enemies, &self.diamonds);
        self.reset_round();
        if let Some((crate::logic::items::WeaponKind::Ranged(w), ammo)) = carried {
            self.hero.equip(w);
            self.hero.ammo = ammo;
        }
    }

    /// Puts everyone back at spawn without touching collected gold.
    fn reset_round(&mut self) {
        self.hero.respawn(self.level.hero_spawn);
        self.view.aim_along(self.hero.facing);
        let center = self.level.nest_center;
        for (i, e) in self.enemies.iter_mut().enumerate() {
            e.reset(self.nest_slots.get(i).copied().unwrap_or(center));
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
    }

    fn start_game(&mut self) {
        self.score.reset();
        self.hero.respawn(self.level.hero_spawn); // a new game starts with the bare sword
        self.load_level(0);
        self.enter(Screen::Ready);
        self.audio.play(Sfx::Start);
    }

    fn enter(&mut self, screen: Screen) {
        self.screen = screen;
        self.menu_index = 0;
        // The backing track steps back when the game does, and under the fanfare.
        self.audio.duck_music(matches!(screen, Screen::Paused | Screen::Dying | Screen::GameOver | Screen::Won));
        if screen == Screen::Title {
            self.audio.set_tempo_step(0);
        }
        self.screen_timer = match screen {
            Screen::Ready => READY_SECONDS,
            Screen::Dying => DYING_SECONDS,
            _ => 0.0,
        };
    }

    // --------------------------------------------------------------- update

    pub fn update(&mut self, dt: f64) {
        self.time += dt;
        self.input.poll(dt);
        self.handle_global_keys();

        match self.screen {
            Screen::Title => match self.menu_choice() {
                Some(MenuItem::Quit) => self.quit_requested = true,
                Some(_) => {
                    self.audio.unlock();
                    self.start_game();
                    // The click that starts the game is not also a sword swing.
                    self.input.consume(Action::Attack);
                }
                // A tap or click anywhere else on the title still begins, as it always has.
                None if self.input.was_pressed(&[Action::Tap]) => {
                    self.audio.unlock();
                    self.start_game();
                    self.input.consume(Action::Attack);
                }
                None => {}
            },
            Screen::Ready => {
                self.screen_timer -= dt;
                if self.screen_timer <= 0.0 {
                    self.enter(Screen::Playing);
                }
            }
            Screen::Playing => self.tick(dt),
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
            Screen::Won | Screen::GameOver => {
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
        };
        self.view.update(dt, &state);
        self.input.flush();
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
        }
    }

    fn handle_global_keys(&mut self) {
        if self.input.was_pressed(&[Action::Fps]) {
            self.show_fps = !self.show_fps;
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
        if aiming && self.screen == Screen::Playing {
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
            return raw;
        }
        relative_dir(raw, self.camera_facing())
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
        self.hero.update(dt, &self.level.grid, wanted);
        self.collect();
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
        self.update_projectiles(dt);
        self.update_bombs(dt);
        for d in self.diamonds.iter_mut() {
            d.update(dt);
        }
        if self.screen != Screen::Playing {
            return;
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
        self.spawn_timer -= dt;
        if self.spawn_timer > 0.0 || self.enemies.len() >= MAX_ENEMIES {
            return;
        }
        self.spawn_timer = SPAWN_EVERY;
        let kind = SPAWN_CYCLE[self.spawned % SPAWN_CYCLE.len()];
        let mut spec = default_enemy_specs(&self.level.grid).into_iter().find(|s| s.kind == kind).unwrap();
        spec.exit_delay = 0.5;
        spec.name = format!("{} {}", spec.name, self.spawned / SPAWN_CYCLE.len() + 2);
        let enemy = Enemy::new(&spec, self.level.nest_center, self.level.data.enemy_speed);
        self.view.add_enemy(&enemy);
        self.enemies.push(enemy);
        self.spawned += 1;
        self.audio.play(Sfx::Fuse);
        let c = self.level.nest_center;
        self.floaters.push(Floater { x: c.x as f64, y: c.y as f64 - 1.0, text: "SOMETHING STIRS".into(), life: 1.6, color: Some(0xb06cff) });
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
        if self.mode_timer <= 0.0 && self.mode_index < MODE_SCHEDULE.len() - 1 {
            self.mode_index += 1;
            self.mode_timer = MODE_SCHEDULE[self.mode_index].1;
            self.global_mode = MODE_SCHEDULE[self.mode_index].0;
            for e in self.enemies.iter_mut() {
                if e.is_dangerous() {
                    e.reverse(&self.level.grid);
                }
            }
        }
    }

    fn floater(&mut self, x: f64, y: f64, text: impl Into<String>, life: f64, color: Option<u32>) {
        self.floaters.push(Floater { x, y, text: text.into(), life, color });
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
                self.nuggets_got += 1;
                if self.nuggets_got == 1 {
                    self.events.push(GameEvent::FirstNugget);
                }
                self.score.add(POINTS_NODULE);
                self.audio.play_reward(self.nuggets_got);
                // Every five nuggets the music presses on a little faster.
                if self.nuggets_got % NUGGETS_PER_TEMPO_STEP == 0 {
                    self.audio.set_tempo_step(self.nuggets_got / NUGGETS_PER_TEMPO_STEP);
                }
                self.view.spark(&self.level.grid, ox, oy, 0xf2c53d);
                let text = format!("{}/{}", self.nuggets_got, self.nuggets_total);
                self.floater(ox, oy, text, 0.9, Some(0xf2c53d));
                if self.nuggets_got >= self.nuggets_total {
                    self.win();
                }
            }
            Some(Tile::Charge) => {
                let before = self.hero.hp;
                self.hero.heal(REMEDY_HEAL);
                let gained = self.hero.hp - before;
                self.score.add(POINTS_CHARGE);
                self.audio.play(Sfx::Heal);
                self.view.spark(&self.level.grid, ox, oy, 0x4cf06a);
                let text = if gained > 0 { format!("+{gained} HP") } else { "FULL HEALTH".to_string() };
                self.floater(ox, oy, text, 0.9, Some(0x5cff6a));
            }
            _ => {}
        }
    }

    /// The last nugget: the dungeon is harvested. Congratulations are in order.
    fn win(&mut self) {
        self.audio.play(Sfx::Win);
        self.audio.set_frenzy(false);
        self.audio.set_threat(0.0);
        self.frenzy.cancel();
        let hp = self.hero.pos();
        self.view.spark(&self.level.grid, hp.x, hp.y, 0xf2c53d);
        self.events.push(GameEvent::Won);
        self.enter(Screen::Won);
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
            let def = self.potions[i].def();
            let mut label = def.name.to_uppercase();
            match self.potions[i].kind {
                PotionKind::Life => {
                    if !self.score.gain_life() {
                        self.score.add(500);
                        label = "FULL LIVES +500".into();
                    }
                }
                PotionKind::Shield => self.hero.grant_shield(),
                PotionKind::Invisible => self.hero.grant_invisibility(INVISIBLE_SECONDS),
            }
            self.audio.play(Sfx::Start);
            self.view.spark(&self.level.grid, o.x as f64, o.y as f64, def.color);
            self.floater(o.x as f64, o.y as f64, label, 1.4, Some(def.color));
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
    fn perform_attack(&mut self) {
        self.audio.play(Sfx::Slash);
        let hits = find_attack_targets(self.hero.pos(), self.hero.facing, &self.enemies);
        for i in hits {
            self.slay_enemy(i);
        }
    }

    fn slay_enemy(&mut self, i: usize) {
        let value = if self.enemies[i].is_stunned() { self.frenzy.consume_value() } else { POINTS_KILL };
        self.score.add(value);
        self.enemies[i].kill();
        self.audio.play(Sfx::Eat);
        let p = self.enemies[i].pos();
        self.view.spark(&self.level.grid, p.x, p.y, 0xffb0b0);
        self.floater(p.x, p.y, value.to_string(), 1.0, None);
    }

    /// Fires the held ranged weapon in the facing direction; the last shot hands the sword back.
    fn fire(&mut self) {
        let Some(kind) = self.hero.try_fire() else { return };
        let shots = spawn_shots(kind, self.hero.pos(), self.hero.facing);
        self.projectiles.extend(shots);
        self.audio.play(Sfx::Arrow);
        if !self.hero.has_ranged_weapon() {
            let hp = self.hero.pos();
            self.floater(hp.x, hp.y, "SWORD", 1.2, Some(SWORD_COLOR));
        }
    }

    fn update_projectiles(&mut self, dt: f64) {
        let hero_pos = self.hero.pos();
        let mut i = 0;
        while i < self.projectiles.len() {
            let state = self.projectiles[i].update(dt, &self.level.grid, hero_pos);
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
            } else if state == ShotState::Wall {
                let p = self.projectiles[i].pos;
                self.view.spark(&self.level.grid, p.x, p.y, 0xfff1c0);
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
        self.audio.play(Sfx::Explode);

        for i in 0..self.enemies.len() {
            if self.enemies[i].is_targetable() && in_blast(c, self.enemies[i].pos()) {
                self.slay_enemy(i);
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
        match result {
            DamageResult::Blocked => {
                self.audio.play(Sfx::Nodule);
                self.view.spark(&self.level.grid, hp.x, hp.y, 0x7fd7ff);
                self.floater(hp.x, hp.y, "BLOCKED", 0.8, Some(0x7fd7ff));
                false
            }
            DamageResult::Immune => false,
            DamageResult::Damaged => {
                self.audio.play(Sfx::Hurt);
                self.view.spark(&self.level.grid, hp.x, hp.y, 0xd8262e);
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
        self.audio.set_threat(if nearest.is_infinite() { 0.0 } else { (1.0 - nearest / 10.0).max(0.0) });
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playing() -> Game {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        let mut g = Game::new();
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        for _ in 0..(READY_SECONDS / crate::app::STEP) as usize + 2 {
            g.update(crate::app::STEP);
        }
        assert_eq!(g.screen, Screen::Playing);
        g
    }

    #[test]
    fn every_fifth_nugget_quickens_the_music() {
        let mut g = playing();
        for n in 1..=12u32 {
            g.nuggets_got = n - 1;
            // Put a nugget under him and let him pick it up.
            let o = g.hero.occupied();
            g.level.grid.set(o.x, o.y, Tile::Nodule);
            g.collect();
            assert_eq!(g.audio.tempo_step(), n / NUGGETS_PER_TEMPO_STEP, "after {n} nuggets");
        }
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
    fn the_last_nugget_wins_with_the_fanfare() {
        let mut g = playing();
        g.nuggets_got = g.nuggets_total - 1;
        let o = g.hero.occupied();
        g.level.grid.set(o.x, o.y, Tile::Nodule);
        g.collect();
        assert_eq!(g.screen, Screen::Won);
        assert!(g.events.contains(&GameEvent::Won));
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
