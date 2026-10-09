//! The seven realms behind the portals (phase 2 of the roadmap). For now each
//! realm is an estate under its sin's sky, seeded to be its own, and its
//! prince a Cyclops in the sin's colour who carries the prince's name.
//! Through a portal he goes into the realm; its eight golden keys open the
//! gate to the prince's court; the prince drops the sin's key, which opens
//! the court's gate; the old man comes next; and then he is back in the
//! Portals of Hell before that portal, now sealed. The realms come in any
//! order, and each is as hard as the keys he holds: one dungeon deeper for
//! every one.

use super::save::Stage;
use super::{Banner, Game, Screen};
use crate::audio::Sfx;
use crate::logic::hub;
use crate::logic::levels::{realm_level, sin_arena};
use crate::logic::sins::{Sin, ALL_SINS};
use crate::logic::types::Tile;

impl Game {
    /// How hard the next realm is: as the first dungeon after the Garden, and one deeper for every
    /// sin's key held; New Game+ adds a tier for each time over (`newgame`), on top of all of it.
    pub fn realm_tier(&self) -> usize {
        1 + self.sins_cleared.count_ones() as usize + self.ng_plus as usize
    }

    /// The realm of `sin`.
    pub fn load_realm(&mut self, sin: Sin) {
        let tier = self.realm_tier();
        self.load(realm_level(sin, tier), tier);
    }

    /// The court of the prince of `sin`, beyond the realm's gate.
    pub fn load_court(&mut self, sin: Sin) {
        let tier = self.realm_tier();
        self.load(sin_arena(sin, tier), tier);
    }

    /// Through a portal in the Portals of Hell, into its realm.
    pub(super) fn enter_realm(&mut self, sin: Sin) {
        self.audio.play(Sfx::Portal);
        self.load_realm(sin);
        self.checkpoint(Stage::Realm(sin));
        self.juice.fade = 1.0;
        self.enter(Screen::Ready);
    }

    /// Down into the Portals of Hell (with its tour, the first time).
    pub(super) fn arrive_in_hub(&mut self) {
        self.load_hub();
        self.checkpoint(Stage::Hub);
        self.juice.fade = 1.0;
        if super::tour::hub_tour_seen() {
            self.enter(Screen::Ready);
        } else {
            self.begin_tour();
        }
    }

    /// Back in the Portals of Hell with the sin's key, before the portal he went through, now
    /// sealed. With the seventh key the seal breaks as he arrives.
    pub(super) fn return_to_hub(&mut self, sin: Sin) {
        self.load_hub();
        let gate = hub::gates()[sin.index()];
        self.level.hero_spawn = gate.front();
        self.start_facing = gate.facing;
        self.reset_round();
        if self.sins_cleared & ALL_SINS == ALL_SINS {
            // He is to watch the seal break: the causeways wait under the lava to rise before him.
            for c in hub::causeways() {
                for p in c.tiles() {
                    self.level.grid.set(p.x, p.y, Tile::Pit);
                }
            }
            if let Some(h) = &mut self.hub {
                h.seal = 0.0;
            }
        }
        self.checkpoint(Stage::Hub);
        self.juice.fade = 1.0;
        self.audio.play(Sfx::Milestone);
        let held = self.sins_cleared.count_ones();
        self.juice.banner = Some(Banner {
            text: format!("THE KEY OF {} IS SET IN THE SEAL", sin.title()),
            sub: if held >= 7 { "ALL SEVEN ARE HIS".to_string() } else { format!("{held} OF SEVEN   {} MORE TO TAKE", 7 - held) },
            color: sin.color(),
            life: 4.0,
            max: 4.0,
        });
        self.enter(Screen::Ready);
    }

    /// Leaving the old man's wares: back to the hub from a prince's court, down into the hub
    /// after the Garden's Cyclops, or (off the road, from the map) on to the next dungeon.
    pub(super) fn after_shop(&mut self) {
        if let Some(sin) = self.realm {
            self.return_to_hub(sin);
        } else if self.dungeon == 0 {
            self.arrive_in_hub();
        } else {
            let next = self.dungeon + 1;
            self.load_level(next);
            self.checkpoint(Stage::Dungeon(next));
            self.juice.fade = 1.0;
            self.enter(Screen::Ready);
        }
    }

    /// The boss before him, by name: the Cyclops, the prince of the realm, or Krampus.
    pub fn boss_name(&self) -> String {
        if self.in_lair() {
            return "KRAMPUS".to_string();
        }
        match self.realm {
            Some(sin) => sin.prince().to_uppercase(),
            None => "THE CYCLOPS".to_string(),
        }
    }

    /// The key the boss drops.
    pub fn key_name(&self) -> String {
        match self.realm {
            Some(sin) => format!("THE KEY OF {}", sin.title()),
            None => "THE RED DIAMOND KEY".to_string(),
        }
    }

    /// Its colour: the red diamond's, or the sin's.
    pub fn key_color(&self) -> u32 {
        self.realm.map(Sin::color).unwrap_or(super::RED_KEY_COLOR)
    }
}
