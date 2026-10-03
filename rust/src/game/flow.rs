//! The frame loop and the screens: which screen is up, what each one does with input, the menus,
//! the keys that work everywhere, and the scene handed to the renderer.

use super::{save, Game, MenuItem, Screen, CLEARED_SECONDS, READY_SECONDS};
use crate::audio::{Sfx, Soundtrack};
use crate::input::Action;
use crate::logic::boss::BossState;
use crate::logic::types::Dir;
use crate::platform;
use crate::render::hud;
use crate::render::view::{CameraMode, GateView, HubView, SceneState, SelectView, StoryView};

/// How long "GO!" hangs on screen once play begins.
const GO_SECONDS: f32 = 0.8;

/// How long the hero's fall plays before the round restarts (or the game is over).
const DYING_SECONDS: f64 = 1.8;

/// What the renderer is handed each frame, built from the game's fields. A macro rather than a
/// method, so `update` can hand it to `self.view` (borrowed mutably) while it borrows the rest.
macro_rules! scene_state {
    ($g:expr) => {
        SceneState {
            grid: &$g.level.grid,
            hero: &$g.hero,
            enemies: &$g.enemies,
            frenzy: &$g.frenzy,
            time: $g.time,
            bombs: &$g.bombs,
            projectiles: &$g.projectiles,
            diamonds: &$g.diamonds,
            potions: &$g.potions,
            puddles: &$g.puddles,
            boss: $g.boss.as_ref(),
            boss_shots: &$g.boss_shots,
            gate: $g.gate.map(|tile| GateView {
                tile,
                keys: $g.keys_got,
                total: $g.keys_total,
                door: $g.door as f32,
                red: $g.boss.is_some(),
                key: $g.key_color(),
            }),
            relic: $g.relic_shrine,
            decoy: $g.decoy.map(|(p, _)| p),
            hero_kind: $g.hero_kind,
            spirits: &$g.spirits,
            fires: &$g.fires,
            angels_spent: $g.angels_spent(),
            red_key: $g.red_key,
            realm: $g.realm,
            hub: $g.hub.as_ref().map(|h| HubView { doors: h.doors.map(|d| d as f32), sins: $g.sins_cleared, seal: h.seal as f32 }),
            loot: $g.vault.as_ref().map(|v| v.loot.as_slice()).unwrap_or(&[]),
            drops: $g.vault.as_ref().map(|v| v.drops.as_slice()).unwrap_or(&[]),
            run: $g.run.as_ref(),
            bolts: $g.brawl.as_ref().map(|b| b.bolts.as_slice()).unwrap_or(&[]),
            hazards: $g.brawl.as_ref().map(|b| b.hazards.as_slice()).unwrap_or(&[]),
            flashes: $g.brawl.as_ref().map(|b| b.flashes.as_slice()).unwrap_or(&[]),
            ails: $g.brawl.as_ref().filter(|_| $g.brawling()).map(|b| b.ails).unwrap_or_default(),
        }
    };
}

impl Game {
    pub(super) fn enter(&mut self, screen: Screen) {
        self.screen = screen;
        self.menu_index = 0;
        self.confirm_reset = false;
        self.show_controls = false;
        self.show_map = false;
        // The backing track steps back when the game does, and under the fanfare.
        self.audio
            .duck_music(matches!(screen, Screen::Paused | Screen::Dying | Screen::GameOver | Screen::Won | Screen::Cleared | Screen::Toll));
        if screen == Screen::Title {
            self.audio.set_tempo_step(0);
        }
        // The title and the shop have their own recording, and every level the song it drew as it loaded.
        match screen {
            Screen::Title | Screen::Select | Screen::Shop => self.audio.set_soundtrack(Some(Soundtrack::Menu)),
            Screen::Ready | Screen::Tour => self.audio.set_soundtrack(self.level_track),
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

    pub fn update(&mut self, dt: f64) {
        self.time += dt;
        self.update_juice(dt);
        self.view.showcase = self.screen == Screen::Title;
        self.view.select = (self.screen == Screen::Select).then_some(SelectView {
            index: self.select_index,
            age: self.select_age as f32,
            denied: self.select_denied as f32,
        });
        self.view.story = self.story.as_ref().filter(|_| self.screen == Screen::Story).map(|s| StoryView {
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
            .filter(|b| !matches!(b.state, BossState::Hidden))
            .map(|b| glam::Vec3::new(b.pos.x as f32, 0.0, b.pos.y as f32))
            .or_else(|| self.hub_focus());
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
            Screen::Toll => {
                if let Some(choice) = self.menu_choice() {
                    if self.vault.is_some() {
                        self.pay_toll(choice);
                    } else {
                        self.answer_run_offer(choice);
                    }
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
                    // The Cyclops' meadow, or the court of the realm's prince.
                    if let Some(sin) = self.realm {
                        self.load_court(sin);
                        self.checkpoint(save::Stage::Court(sin));
                    } else {
                        let next = self.dungeon;
                        self.load_boss(next);
                        self.checkpoint(save::Stage::Arena(next));
                    }
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
        let state = scene_state!(self);
        self.view.update(dt, &state);
        self.input.flush();
    }

    /// Play on the title: Scene I first (unless it is switched off), then the heroes.
    pub(super) fn play_from_title(&mut self) {
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
            // Keep first, and highlighted: Mammon would have it so. (A prince's offer in the realms
            // after the vaults puts the answer he would have first, too.)
            Screen::Toll if self.vault.is_some() => return vec![MenuItem::KeepAll, MenuItem::DropAll],
            Screen::Toll => return self.run_offer().map(|o| o.choices).unwrap_or_default(),
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
    pub(super) fn menu_choice(&mut self) -> Option<MenuItem> {
        let items = self.menu_items();
        if items.is_empty() {
            return None;
        }
        let rects = hud::menu_rects(self.screen, &items, self.viewport.0, self.viewport.1);
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

    /// Everything the renderer needs to draw this frame.
    pub fn scene(&self) -> SceneState<'_> {
        scene_state!(self)
    }

    pub(super) fn handle_global_keys(&mut self) {
        if self.input.was_pressed(&[Action::Fps]) {
            self.show_fps = !self.show_fps;
        }
        if self.input.was_pressed(&[Action::Cheat]) {
            self.cheat_to_boss();
        }
        if self.input.was_pressed(&[Action::Readout]) {
            if let Some(b) = self.brawl.as_mut() {
                b.readout = !b.readout;
            }
        }
        if self.input.was_pressed(&[Action::NextBout]) && self.screen == Screen::Playing {
            self.next_bout();
        }
        if self.input.was_pressed(&[Action::NextStageBout]) && self.screen == Screen::Playing {
            self.next_stage_bout();
        }
        if self.input.was_pressed(&[Action::KeyCheat]) && self.screen == Screen::Playing {
            self.cheat_to_gate();
        }
        if self.input.was_pressed(&[Action::FreeMove]) {
            self.hero.free_move = !self.hero.free_move;
            platform::save("vh.free_move", if self.hero.free_move { "1" } else { "0" });
            let p = self.hero.pos();
            let text = if self.hero.free_move { "FREE MOVEMENT" } else { "GRID MOVEMENT" };
            self.floater(p.x, p.y - 0.8, text, 1.4, Some(0xf2e3c4));
        }
        if self.input.was_pressed(&[Action::ShopCheat]) && self.screen != Screen::Shop {
            self.cheat_to_shop();
        }
        if self.input.was_pressed(&[Action::Call])
            && matches!(self.screen, Screen::Playing | Screen::Paused)
            && !self.show_controls
            && !self.show_map
        {
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
            // R3: swing the view back behind him if the stick moved it; otherwise switch the camera.
            if self.input.pad_aiming {
                self.input.pad_aiming = false;
                self.view.recenter();
            } else {
                self.view.camera_mode = if self.view.camera_mode == CameraMode::Third { CameraMode::Top } else { CameraMode::Third };
            }
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
}
