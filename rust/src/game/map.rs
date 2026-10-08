//! The developer's map, from F12 (in any build) or LEVELS on the title and the pause menu (in
//! developer builds and the Desktop builds `tools/release.sh` makes, with `--dev`, or once F12 has
//! opened it): every stop on the road down (the
//! Garden and its Cyclops, the Portals of Hell, then each sin's realm and its prince's court, the
//! Pit, the scenes, the full map), and a jump straight to any of them, for testing. It lies over
//! the screen (drawn by `render::hud::map`) until B, Esc, Start or the map action puts it away.
//! (The player's map is the full map, `fullmap`.)

use super::save::Stage;
use super::{Game, MenuItem, Screen};
use crate::audio::Sfx;
use crate::input::Action;
use crate::logic::gym;
use crate::logic::levels::level_for;
use crate::logic::sins::{Sin, SINS};
use crate::logic::story::{self, Scene, Shot};
use crate::logic::types::Dir;
use crate::render::hud;

/// One stop on the map: where it goes and what it is called. A scene's stop plays that scene from
/// its `scene` beat instead (for Scene I its stage is where the scene leads).
pub struct MapStop {
    pub stage: Stage,
    pub label: String,
    pub scene: Option<(Scene, usize)>,
    /// The stop is a general's command post in the stage's realm (WARBAND.md §9).
    pub post: bool,
    /// The stop opens the full map of the stage's level, over the pause (UI.md §7).
    pub full_map: bool,
    /// The stop is a screen of the UI rather than a place (UI.md §8, §9).
    pub page: Option<MapPage>,
}

/// The UI's screens on the Map, to look at without playing to them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MapPage {
    /// The settings page (over the pause menu, or the title).
    Settings,
    /// The in-game loading card, over where he stands, for a few seconds.
    LoadingCard,
}

/// The road down: the Garden and its Cyclops, the Portals of Hell, then each sin's realm and
/// its prince's court (in any order in play; here in the order of the doors).
pub fn map_stops() -> Vec<MapStop> {
    let mut stops = vec![
        stop(Stage::Dungeon(0), format!("DEPTH I - {}", level_for(0).name.to_uppercase())),
        stop(Stage::Arena(0), "DEPTH I - THE CYCLOPS".to_string()),
        stop(Stage::Hub, "DEPTH II - THE PORTALS OF HELL".to_string()),
    ];
    for sin in SINS {
        stops.push(stop(Stage::Realm(sin), format!("{} - {}", sin.title(), sin.realm().to_uppercase())));
        stops.push(stop(Stage::Court(sin), format!("{} - {}, ITS PRINCE", sin.title(), sin.prince().to_uppercase())));
    }
    // The bottom of the Pit: Krampus in his lair, the friend in the cage (FINALE.md §3).
    stops.push(stop(Stage::Lair, "THE BOTTOM OF THE PIT - KRAMPUS".to_string()));
    stops.push(stop(Stage::Gym, "THE GYM - ONE OF EVERY FOE".to_string()));
    // The Pit under the House of the Devil, level by level.
    for n in 1..=crate::logic::pit::LEVELS {
        stops.push(stop(Stage::Pit(n), format!("THE PIT - LEVEL {n}")));
    }
    // Off the road: dungeons II and III, the levels built to the art direction.
    stops.push(stop(Stage::Dungeon(1), format!("OFF THE ROAD - {}", level_for(1).name.to_uppercase())));
    stops.push(stop(Stage::Dungeon(2), format!("OFF THE ROAD - {}", level_for(2).name.to_uppercase())));
    // The old man's workshop, as after the first Cyclops.
    stops.push(stop(Stage::Shop(0), "THE OLD MAN'S WORKSHOP".to_string()));
    // Each realm's general at his command post.
    for sin in SINS {
        // (The Steward of the Last Course is the Steward on the Map's button.)
        let full = crate::logic::war::general::General::of(sin).name;
        let name = if full.len() > 26 { full.split(" OF ").next().unwrap_or(full) } else { full };
        stops.push(MapStop {
            stage: Stage::Realm(sin),
            label: format!("{} - {name}'S POST", sin.title()),
            scene: None,
            post: true,
            page: None,
            full_map: false,
        });
    }
    // The story's scenes, from the start of each place: the graveyard at dusk, and the gateway below.
    for (beat, label) in [(0, "SCENE I - THE GRAVEYARD"), (story::first(Shot::Arrive).unwrap_or(0), "SCENE I - THE DUNGEON GATEWAY")] {
        stops.push(MapStop {
            stage: Stage::Dungeon(0),
            label: label.to_string(),
            scene: Some((Scene::One, beat)),
            post: false,
            page: None,
            full_map: false,
        });
    }
    // The end: Scene II, from the bottom of the Pit to the credits (seen from here, it does not
    // count as finishing the game).
    stops.push(MapStop {
        stage: Stage::Dungeon(0),
        label: "SCENE II - THE WAY UP".to_string(),
        scene: Some((Scene::Two, 0)),
        post: false,
        full_map: false,
        page: None,
    });
    // The full map (UI.md §7), of the garden and of the vaults.
    for (stage, name) in
        [(Stage::Dungeon(0), level_for(0).name.to_uppercase()), (Stage::Realm(Sin::Greed), Sin::Greed.realm().to_uppercase())]
    {
        stops.push(MapStop { stage, label: format!("THE FULL MAP - {name}"), scene: None, post: false, full_map: true, page: None });
    }
    // The UI's screens: the settings, and the loading card with its legend.
    for (page, label) in [(MapPage::Settings, "THE UI - SETTINGS"), (MapPage::LoadingCard, "THE UI - THE LOADING CARD")] {
        stops.push(MapStop {
            stage: Stage::Dungeon(0),
            label: label.to_string(),
            scene: None,
            post: false,
            full_map: false,
            page: Some(page),
        });
    }
    stops
}

fn stop(stage: Stage, label: String) -> MapStop {
    MapStop { stage, label, scene: None, post: false, full_map: false, page: None }
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
            Some(if let Some(p) = &self.pit {
                Stage::Pit(p.level)
            } else if self.lair.is_some() {
                Stage::Lair
            } else if self.hub.is_some() {
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
        // Back on the menu, LEVELS stays highlighted.
        if let Some(i) = self.menu_items().iter().position(|&m| m == MenuItem::DevMap) {
            self.menu_index = i;
        }
    }

    pub(super) fn update_map(&mut self) {
        let stops = map_stops();
        let n = stops.len();
        let rects = hud::map_rects(n, self.viewport.0, self.viewport.1);
        let hit = |p: (f32, f32)| rects.iter().position(|r| p.0 >= r[0] && p.0 <= r[0] + r[2] && p.1 >= r[1] && p.1 <= r[1] + r[3]);
        // Past ten stops, in columns: left and right step a column over.
        let rows = n.div_ceil(hud::map_cols(n));
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
        if self.input.was_pressed(&[Action::Guide, Action::Pause, Action::Map, Action::DevMap]) {
            self.input.consume(Action::Pause);
            self.close_map();
            return;
        }
        let clicked = self.input.click.and_then(hit);
        if let Some(i) = clicked {
            self.map_index = i;
        }
        if clicked.is_some() || self.input.was_pressed(&[Action::Confirm, Action::Jump]) {
            let stop = &stops[self.map_index.min(n - 1)];
            if let Some(page) = stop.page {
                self.show_map = false;
                match page {
                    MapPage::Settings => self.open_settings(),
                    MapPage::LoadingCard => self.show_loading_card(6.0),
                }
                return;
            }
            match stop.scene {
                Some((scene, beat)) => self.map_scene(scene, beat),
                None => {
                    let (post, full_map) = (stop.post, stop.full_map);
                    self.map_jump(stop.stage);
                    if post {
                        self.jump_to_post();
                    }
                    if full_map {
                        self.enter(Screen::Paused);
                        self.open_full_map();
                    }
                }
            }
        }
    }

    /// Straight into a scene at `beat`, from the title or the pause menu: Scene I ends, as ever,
    /// at the choice of hero, and Scene II at the title.
    pub fn map_scene(&mut self, scene: Scene, beat: usize) {
        self.show_map = false;
        self.show_controls = false;
        self.audio.unlock();
        self.audio.play(Sfx::Whoosh);
        self.tour = None;
        self.tour_pending = false;
        self.story = None;
        match scene {
            Scene::One => self.begin_story(),
            Scene::Two => self.play_ending(),
        }
        if beat > 0 {
            self.story_jump(beat, 0.0);
        }
        self.juice.fade = 1.0;
        for a in [Action::Confirm, Action::Jump, Action::Attack, Action::Tap] {
            self.input.consume(a);
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
                Stage::Arena(n) => self.load_boss(n),
                // At the old man's: the dungeon after it waits behind his door (as Continue does).
                Stage::Shop(n) => self.load_level(n),
                Stage::Realm(s) => self.load_realm(s),
                Stage::Court(s) | Stage::SinShop(s) => self.load_court(s),
                Stage::Gym => self.load_gym(),
                Stage::Pit(n) => self.load_pit(n),
                Stage::Lair => self.load_lair(),
                Stage::Hub => unreachable!(),
            }
            self.checkpoint(stage);
            self.juice.fade = 1.0;
            if matches!(stage, Stage::Shop(_)) {
                self.open_shop();
            } else {
                self.enter(Screen::Ready);
            }
        }
        for a in [Action::Confirm, Action::Jump, Action::Attack, Action::Tap] {
            self.input.consume(a);
        }
    }
}
