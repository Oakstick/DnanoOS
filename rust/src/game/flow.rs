//! The frame loop and the screens: which screen is up, what each one does with input, the menus,
//! the keys that work everywhere, and the scene handed to the renderer.

use super::{save, Game, MenuItem, Screen, CLEARED_SECONDS, READY_SECONDS};
use crate::audio::{Sfx, Soundtrack};
use crate::input::{Action, Context};
use crate::logic::boss::BossState;
use crate::logic::types::Dir;
use crate::platform;
use crate::render::hud;
use crate::render::view::{CameraMode, GateView, HubView, SceneState, SelectView};

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
                red: $g.arena(),
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
            traps: $g.brawl.as_ref().map(|b| b.traps.as_slice()).unwrap_or(&[]),
            pace: $g.pace() as f32,
            time_scale: $g.time_scale() as f32,
            hero_fighter: $g.brawl.as_ref().filter(|_| $g.brawling()).map(|b| &b.hero),
            parry_edge: $g.parry_edge(),
            general: $g.general.as_ref(),
            pit: $g.pit_view(),
            lair: $g.lair.as_ref(),
        }
    };
}

impl Game {
    pub(super) fn enter(&mut self, screen: Screen) {
        // The run's snapshot (and a new high score) is written here, between fights, not on the
        // kill that set it.
        self.flush_saves();
        // Play never begins by itself while he is away from the window (a countdown that ends, the
        // old man's door): it waits on the pause menu for him.
        let screen = if screen == Screen::Playing && !self.focused { Screen::Paused } else { screen };
        self.screen = screen;
        self.menu_index = 0;
        self.confirm_reset = false;
        self.show_controls = false;
        self.show_map = false;
        self.settings_page = Default::default();
        self.map_ui.full = None;
        self.warband.board_open = false;
        (self.bestiary.open, self.bestiary.review) = (false, false);
        // The backing track steps back when the game does, and under the fanfare.
        self.audio
            .duck_music(matches!(screen, Screen::Paused | Screen::Dying | Screen::GameOver | Screen::Won | Screen::Cleared | Screen::Toll));
        if screen == Screen::Title {
            self.audio.set_tempo_step(0);
        }
        // The title and the shop have their own recording, and every level the song it drew as it loaded.
        match screen {
            Screen::Title | Screen::Select | Screen::Shop => self.audio.set_soundtrack(Some(Soundtrack::Menu)),
            Screen::Ready | Screen::Tour | Screen::View => self.audio.set_soundtrack(self.level_track),
            _ => {}
        }
        if screen == Screen::Ready {
            self.audio.play(Sfx::Tick); // "READY"
        }
        self.screen_timer = match screen {
            Screen::Ready => READY_SECONDS,
            Screen::Dying => DYING_SECONDS,
            Screen::Cleared => CLEARED_SECONDS,
            _ => 0.0,
        };
    }

    pub fn update(&mut self, dt: f64) {
        self.sim = Default::default();
        // A new level is being made ready (RP03): the world waits behind the loading screen. Only
        // the camera moves, cut to where the level starts, so the frame the renderer prepares is
        // the one that will be shown.
        if let Some(l) = self.loading.as_mut() {
            l.age += dt;
            self.juice.fade = 1.0;
            if l.age > crate::game::LOADING_TIMEOUT {
                self.finish_loading();
            } else {
                let state = scene_state!(self);
                self.view.update(0.0, &state);
                self.input.flush();
                return;
            }
        }
        self.time += dt;
        self.update_juice(dt);
        self.view.showcase = self.screen == Screen::Title;
        self.view.select = (self.screen == Screen::Select).then_some(SelectView {
            index: self.select_index,
            age: self.select_age as f32,
            denied: self.select_denied as f32,
        });
        self.view.story = self.story.as_ref().filter(|_| self.screen == Screen::Story).map(|s| s.view());
        self.view.forge = self.screen == Screen::Shop;
        self.view.workshop = self.view.forge.then(|| self.workshop_view());
        self.view.tour = self.tour.as_ref().filter(|_| self.screen == Screen::Tour).map(|t| (t.stop().target, t.t as f32, t.index));
        self.view.focus = self
            .boss
            .as_ref()
            .filter(|b| b.alive() && matches!(self.screen, Screen::Playing | Screen::Paused))
            .filter(|b| !matches!(b.state, BossState::Hidden))
            .map(|b| glam::Vec3::new(b.pos.x as f32, 0.0, b.pos.y as f32))
            .or_else(|| self.prince_focus())
            .or_else(|| self.hub_focus());
        self.input.set_context(self.input_context());
        self.input.poll(dt);
        // A press of any kind (a controller's reaches the game without the window's focus): he is here.
        if self.input.presses().next().is_some() {
            self.focused = true;
        }
        self.note_device();
        self.handle_global_keys();
        self.settle_pace();

        match self.screen {
            Screen::Title | Screen::Paused if self.bestiary.open => self.update_bestiary(dt),
            Screen::Title | Screen::Paused if self.settings_page.open => self.update_settings(),
            Screen::Paused if self.full_map_open() => self.update_full_map(dt),
            Screen::Title | Screen::Paused if self.show_controls => self.update_controls(),
            Screen::Title | Screen::Paused if self.show_map => self.update_map(),
            Screen::Title | Screen::Paused if self.warband.board_open => self.update_ledger_board(),
            Screen::Paused if self.guide_open => self.update_guide(),
            Screen::Title => match self.menu_choice() {
                Some(MenuItem::Quit) => self.quit_requested = true,
                Some(MenuItem::Controls) => self.open_controls(),
                Some(MenuItem::Map | MenuItem::DevMap) => self.open_map(),
                Some(MenuItem::Settings) => self.open_settings(),
                Some(MenuItem::Bestiary) => self.open_bestiary(),
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
                self.screen_timer -= dt;
                if self.screen_timer <= 0.0 {
                    self.enter(Screen::Playing);
                    self.audio.play(Sfx::Go);
                    self.juice.go = GO_SECONDS;
                    if let Some(b) = &mut self.boss {
                        b.wake();
                    }
                }
            }
            Screen::Playing => {
                self.keep_immortal();
                // A freeze-frame on a hit: the world holds for a beat, effects keep moving; and
                // while time is slowed the world runs slower, evenly (`juice`).
                self.play_step(dt);
                self.update_war(dt);
                // (The bestiary only watches: the foes met and felled, and what felled him.)
                self.observe_bestiary(dt);
            }
            Screen::Toll => {
                if let Some(choice) = self.menu_choice() {
                    if self.voices.deceit.asking.is_some() {
                        self.answer_deceit(choice);
                    } else if self.vault.is_some() {
                        self.pay_toll(choice);
                    } else {
                        self.answer_run_offer(choice);
                    }
                }
            }
            Screen::Paused => match self.menu_choice() {
                Some(MenuItem::Resume) => self.enter(Screen::Playing),
                Some(MenuItem::Settings) => self.open_settings(),
                Some(MenuItem::CallOldMan) => self.call_old_man(),
                Some(MenuItem::Controls) => self.open_controls(),
                Some(MenuItem::Map) => self.open_full_map(),
                Some(MenuItem::DevMap) => self.open_map(),
                Some(MenuItem::Ledger) => self.open_ledger(),
                Some(MenuItem::Bestiary) => self.open_bestiary(),
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
            Screen::View => self.update_view_choice(),
            Screen::GameOver => {
                if self.input.was_pressed(&[Action::Confirm, Action::Jump, Action::Tap]) {
                    self.enter(Screen::Title);
                }
            }
        }
        // The old man's counsel and the prince's cheats (WARBAND.md §10).
        self.update_voices(dt);
        let mut lap = crate::render::profile::Lap::start();
        self.sync_feel();
        let state = scene_state!(self);
        self.view.update(dt, &state);
        self.sim.view += lap.lap();
        // A marked save is written once the fight gives it a calm moment.
        self.tick_saves(dt);
        // (Set again for the keys and clicks that come before the next update: the screen may
        // have changed.)
        self.input.set_context(self.input_context());
        self.input.flush();
    }

    /// The layer the controls read through now: play's while the hero moves with nothing open
    /// over him, and the menus' otherwise (presentation only).
    pub fn input_context(&self) -> Context {
        let over = self.show_controls
            || self.bestiary.open
            || self.show_map
            || self.guide_open
            || self.warband.board_open
            || self.full_map_open()
            || self.settings_page.open;
        if matches!(self.screen, Screen::Playing | Screen::Ready | Screen::Dying | Screen::Cleared) && !over {
            Context::Play
        } else {
            Context::Menu
        }
    }

    /// The fight asks for a rumble in the hands (UI.md §3): queued for the app, which plays it
    /// scaled by the setting; a few at most wait (no app drains them under the harnesses).
    pub(super) fn rumble(&mut self, r: crate::input::Rumble) {
        if self.rumbles.len() < 8 {
            self.rumbles.push(r);
        }
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
            // (The view, slow motion and the shake are on the settings page now.)
            Screen::Paused => {
                let mut items = vec![
                    MenuItem::Resume,
                    MenuItem::Settings,
                    MenuItem::Controls,
                    MenuItem::Map,
                    MenuItem::Ledger,
                    MenuItem::Bestiary,
                    MenuItem::CallOldMan,
                    MenuItem::QuitToTitle,
                ];
                if self.dev {
                    items.insert(4, MenuItem::DevMap);
                }
                items
            }
            Screen::View => return super::viewpick::VIEWS.iter().map(|v| v.0).collect(),
            // Keep first, and highlighted: Mammon would have it so. (A prince's offer in the realms
            // after the vaults puts the answer he would have first, too.)
            Screen::Toll if self.voices.deceit.asking.is_some() => return self.deceit_offer().map(|o| o.choices).unwrap_or_default(),
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
        let horizontal = matches!(self.screen, Screen::Title | Screen::View);
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

    /// Captures: the foes the camera was moved to see posed by their clips where they stand
    /// (`WorldView::pose_foes_in_view`).
    pub fn pose_foes_in_view(&mut self) {
        let state = scene_state!(self);
        self.view.pose_foes_in_view(&state);
    }

    /// The testing keys (developer builds only: `testing_keys`): F1 the Cyclops, F10 a rival, F5
    /// god mode, F8 the ring's readout, F6 gold, F9 and F7 the ring's bouts, F2 the gate, 0 the old
    /// man's wares. (F12, the developer's map, is read with the pages.)
    fn handle_testing_keys(&mut self) {
        if self.input.was_pressed(&[Action::Cheat]) {
            self.cheat_to_boss();
        }
        if self.input.was_pressed(&[Action::MakeRival]) && self.screen == Screen::Playing {
            self.cheat_rival();
        }
        if self.input.was_pressed(&[Action::Immortal]) {
            self.toggle_immortal();
        }
        if self.input.was_pressed(&[Action::Readout]) {
            if let Some(b) = self.brawl.as_mut() {
                b.readout = !b.readout;
            }
        }
        if self.input.was_pressed(&[Action::Gold]) && matches!(self.screen, Screen::Playing | Screen::Paused | Screen::Shop) {
            self.cheat_gold();
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
        if self.input.was_pressed(&[Action::ShopCheat]) && self.screen != Screen::Shop {
            self.cheat_to_shop();
        }
    }

    pub(super) fn handle_global_keys(&mut self) {
        // While the settings page waits for a control to bind (or asks about one), every press
        // is its own: even the testing keys do nothing (it tells the player they are kept).
        if self.settings_page.listen.is_some() || self.settings_page.conflict.is_some() {
            let _ = self.input.take_look();
            return;
        }
        if self.input.was_pressed(&[Action::Fps]) {
            self.show_fps = !self.show_fps;
        }
        if self.testing_keys {
            self.handle_testing_keys();
        }
        if self.input.was_pressed(&[Action::FreeMove]) {
            self.hero.free_move = !self.hero.free_move;
            platform::save("vh.free_move", if self.hero.free_move { "1" } else { "0" });
            let p = self.hero.pos();
            let text = if self.hero.free_move { "FREE MOVEMENT" } else { "GRID MOVEMENT" };
            self.floater(p.x, p.y - 0.8, text, 1.4, Some(0xf2e3c4));
        }
        if self.input.was_pressed(&[Action::Call])
            && matches!(self.screen, Screen::Playing | Screen::Paused)
            && !self.bestiary.open
            && !self.show_controls
            && !self.show_map
            && !self.full_map_open()
            && !self.settings_page.open
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
        // M, Tab, View or the D-pad's up: the full map, from play or the pause menu. Pressed again,
        // it is put away (`update_full_map`).
        let pages = self.show_controls
            || self.bestiary.open
            || self.show_map
            || self.warband.board_open
            || self.guide_open
            || self.full_map_open()
            || self.settings_page.open;
        if self.input.was_pressed(&[Action::Map]) && matches!(self.screen, Screen::Playing | Screen::Paused) && !pages {
            self.open_full_map();
        }
        // F12: the developer's map (its stops to jump to), from the title, play or the pause menu;
        // its buttons stay on the title and the pause menu for the rest of the session (where the
        // testing keys work).
        if self.testing_keys
            && self.input.was_pressed(&[Action::DevMap])
            && matches!(self.screen, Screen::Title | Screen::Playing | Screen::Paused)
            && !self.show_map
        {
            self.dev = true;
            if self.screen == Screen::Playing {
                self.enter(Screen::Paused);
            }
            (self.show_controls, self.guide_open, self.warband.board_open) = (false, false, false);
            self.settings_page.open = false;
            self.bestiary.open = false;
            self.map_ui.full = None;
            self.open_map();
            self.input.consume(Action::DevMap);
        }
        // L3: the minimap's zoom (near, mid, far).
        if self.input.was_pressed(&[Action::MinimapZoom]) && matches!(self.screen, Screen::Playing | Screen::Ready) {
            self.cycle_minimap_zoom();
        }
        // With the controller page, a map or the settings open, Esc and Start close it instead.
        if self.input.was_pressed(&[Action::Pause])
            && !self.bestiary.open
            && !self.show_controls
            && !self.show_map
            && !self.warband.board_open
            && !self.guide_open
            && !self.full_map_open()
            && !self.settings_page.open
        {
            if self.screen == Screen::Playing {
                self.enter(Screen::Paused);
                // Nothing pressed with it may pick Resume at once.
                self.input.consume(Action::Confirm);
                self.input.consume(Action::Jump);
            } else if self.screen == Screen::Paused {
                self.enter(Screen::Playing);
            }
        }
        if self.input.was_pressed(&[Action::Camera]) && self.screen != Screen::View {
            self.switch_camera();
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
            } else if self.screen != Screen::View {
                self.switch_camera();
            }
        }
        // Looking around is allowed during the countdown too.
        if aiming && matches!(self.screen, Screen::Playing | Screen::Ready) {
            self.view.look(dx, dy);
        }
    }

    /// Pauses the game when the window loses focus.
    pub fn focus_lost(&mut self) {
        self.focused = false;
        self.input.blur();
        if self.screen == Screen::Playing {
            self.enter(Screen::Paused);
        }
    }

    /// The controller in his hands was unplugged (or its battery died): as if he had looked away,
    /// the game pauses, and play that would begin by itself waits for him (QA.md Q5). A controller
    /// unplugged while he plays on the keyboard changes nothing.
    pub fn controller_lost(&mut self) {
        if !self.input.last_device().is_pad() {
            return;
        }
        self.focused = false;
        if self.screen == Screen::Playing {
            self.enter(Screen::Paused);
        }
    }

    /**
     * The GPU was lost (a driver reset, sleep and resume, a GPU switch) and the renderer starts
     * again (QA.md Q2). Nothing of the run is lost: whatever save is marked is written now (as when
     * the game closes), the game itself goes on in memory, play waits on the pause menu (and play
     * that would begin by itself waits for him, as when he looks away), the level goes up whole to
     * the new renderer, and a line tells him what happened when he resumes.
     */
    pub fn renderer_lost(&mut self) {
        self.flush_saves();
        self.focused = false;
        self.input.blur();
        if self.screen == Screen::Playing {
            self.enter(Screen::Paused);
        }
        self.view.static_dirty = true;
        self.juice.banner = Some(super::Banner {
            text: "THE SCREEN WAS LOST, AND IS BACK".into(),
            sub: "NOTHING OF YOUR RUN WAS LOST".into(),
            color: 0xf2e3c4,
            life: 4.0,
            max: 4.0,
        });
    }

    /// The window has the player's attention again (the pause menu he came back to stays up).
    pub fn focus_gained(&mut self) {
        self.focused = true;
    }
}
