//! The map, from MAP on the title or the pause menu: every stop on the road
//! down (the Garden and its Cyclops, the Portals of Hell, then each sin's
//! realm and its prince's court), and a jump straight to any of them. For testing for now. It lies over the screen (drawn by
//! `render::hud::map`) until B, Esc or Start puts it away.

use super::save::Stage;
use super::{Game, MenuItem, Screen};
use crate::audio::Sfx;
use crate::input::Action;
use crate::logic::gym;
use crate::logic::levels::level_for;
use crate::logic::sins::SINS;
use crate::logic::types::Dir;
use crate::render::hud;

/// One stop on the map: where it goes and what it is called.
pub struct MapStop {
    pub stage: Stage,
    pub label: String,
}

/// The road down: the Garden and its Cyclops, the Portals of Hell, then each sin's realm and
/// its prince's court (in any order in play; here in the order of the doors).
pub fn map_stops() -> Vec<MapStop> {
    let mut stops = vec![
        MapStop { stage: Stage::Dungeon(0), label: format!("DEPTH I - {}", level_for(0).name.to_uppercase()) },
        MapStop { stage: Stage::Arena(0), label: "DEPTH I - THE CYCLOPS".to_string() },
        MapStop { stage: Stage::Hub, label: "DEPTH II - THE PORTALS OF HELL".to_string() },
    ];
    for sin in SINS {
        stops.push(MapStop { stage: Stage::Realm(sin), label: format!("{} - {}", sin.title(), sin.realm().to_uppercase()) });
        stops.push(MapStop { stage: Stage::Court(sin), label: format!("{} - {}, ITS PRINCE", sin.title(), sin.prince().to_uppercase()) });
    }
    stops.push(MapStop { stage: Stage::Gym, label: "THE GYM - ONE OF EVERY FOE".to_string() });
    // Off the road: dungeons II and III, the levels built to the art direction.
    stops.push(MapStop { stage: Stage::Dungeon(1), label: format!("OFF THE ROAD - {}", level_for(1).name.to_uppercase()) });
    stops.push(MapStop { stage: Stage::Dungeon(2), label: format!("OFF THE ROAD - {}", level_for(2).name.to_uppercase()) });
    stops
}

impl Game {
    /// The gym: one of every foe, stage by stage, on platforms over the lava.
    pub fn load_gym(&mut self) {
        self.load(gym::gym(), 1);
    }
}

impl Game {
    pub fn open_map(&mut self) {
        self.show_map = true;
        // The stop he is at now is highlighted (the first one from the title).
        let here = if self.screen == Screen::Paused {
            Some(if self.hub.is_some() {
                Stage::Hub
            } else if self.theme().is_gym() {
                Stage::Gym
            } else if let Some(sin) = self.realm {
                if self.arena() {
                    Stage::Court(sin)
                } else {
                    Stage::Realm(sin)
                }
            } else if self.boss.is_some() {
                Stage::Arena(self.dungeon)
            } else {
                Stage::Dungeon(self.dungeon)
            })
        } else {
            None
        };
        self.map_index = here.and_then(|s| map_stops().iter().position(|m| m.stage == s)).unwrap_or(0);
        self.audio.play(Sfx::Whoosh);
        self.input.consume(Action::Confirm);
        self.input.consume(Action::Jump);
        self.input.consume(Action::Tap);
    }

    fn close_map(&mut self) {
        self.show_map = false;
        self.audio.play(Sfx::Nodule);
        // Back on the menu, MAP stays highlighted.
        if let Some(i) = self.menu_items().iter().position(|&m| m == MenuItem::Map) {
            self.menu_index = i;
        }
    }

    pub(super) fn update_map(&mut self) {
        let stops = map_stops();
        let n = stops.len();
        let rects = hud::map_rects(n, self.viewport.0, self.viewport.1);
        let hit = |p: (f32, f32)| rects.iter().position(|r| p.0 >= r[0] && p.0 <= r[0] + r[2] && p.1 >= r[1] && p.1 <= r[1] + r[3]);
        // Two columns past ten stops: left and right step a column over.
        let rows = if n > 10 { n.div_ceil(2) } else { n };
        match self.input.nav {
            Some(Dir::Up) => self.map_index = (self.map_index + n - 1) % n,
            Some(Dir::Down) => self.map_index = (self.map_index + 1) % n,
            Some(Dir::Left) => self.map_index = (self.map_index + n - rows) % n,
            Some(Dir::Right) => self.map_index = (self.map_index + rows) % n,
            _ => {}
        }
        if self.input.was_pressed(&[Action::TabPrev]) {
            self.map_index = (self.map_index + n - 1) % n;
        }
        if self.input.was_pressed(&[Action::TabNext]) {
            self.map_index = (self.map_index + 1) % n;
        }
        if self.input.cursor_moved && !self.input.mouse_captured {
            if let Some(i) = self.input.cursor.and_then(hit) {
                self.map_index = i;
            }
        }
        self.input.consume(Action::Attack);
        if self.input.was_pressed(&[Action::Bomb, Action::Pause]) {
            self.input.consume(Action::Pause);
            self.close_map();
            return;
        }
        let clicked = self.input.click.and_then(hit);
        if let Some(i) = clicked {
            self.map_index = i;
        }
        if clicked.is_some() || self.input.was_pressed(&[Action::Confirm, Action::Jump]) {
            self.map_jump(stops[self.map_index.min(n - 1)].stage);
        }
    }

    /**
     * Straight to a stop on the map. From the title he goes as the saved hero
     * (or Ragnar) with a fresh kit and no story or tour; from the pause menu he
     * keeps his score, belt and blessing. Either way the adventure is saved
     * there, so Continue comes back to it.
     */
    pub fn map_jump(&mut self, stage: Stage) {
        self.show_map = false;
        self.show_controls = false;
        self.audio.unlock();
        self.audio.play(Sfx::Whoosh);
        self.tour = None;
        self.tour_pending = false;
        if matches!(self.screen, Screen::Title | Screen::GameOver) {
            if let Some(run) = &self.saved_run {
                self.hero_kind = run.hero;
            }
            self.score.reset();
            self.juice.score_shown = 0.0;
            self.new_kit();
        }
        if stage == Stage::Hub {
            // The first time in the Portals of Hell, the tour shows him round the seven doors.
            self.arrive_in_hub();
        } else {
            match stage {
                Stage::Dungeon(n) => self.load_level(n),
                Stage::Arena(n) | Stage::Shop(n) => self.load_boss(n),
                Stage::Realm(s) => self.load_realm(s),
                Stage::Court(s) | Stage::SinShop(s) => self.load_court(s),
                Stage::Gym => self.load_gym(),
                Stage::Hub => unreachable!(),
            }
            self.checkpoint(stage);
            self.juice.fade = 1.0;
            self.enter(Screen::Ready);
        }
        for a in [Action::Confirm, Action::Jump, Action::Attack, Action::Tap] {
            self.input.consume(a);
        }
    }
}
