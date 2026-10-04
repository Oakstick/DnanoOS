//! The gate in the outer wall: locked until every key is found, then its doors swing open and
//! stepping through wins the level.

use super::{Banner, Game, GameEvent, Screen, CLEARED_SECONDS, DOOR_SECONDS};
use crate::audio::Sfx;
use crate::logic::types::Tile;

impl Game {
    /// The last key: every lock on the gate is lit. Now Ragnar must reach it.
    pub(super) fn unlock_gate(&mut self) {
        self.gate_open = true;
        self.audio.play(Sfx::Win);
        let hp = self.hero.pos();
        self.view.sparkle(&self.level.grid, hp.x, hp.y, 0.6, 0xf2c53d, 40);
        let arena = self.arena();
        self.juice.banner = Some(Banner {
            text: "THE GATE IS UNLOCKED".into(),
            sub: if self.in_court() {
                "IN THE SOUTH WALL: GO BACK THROUGH IT".into()
            } else if arena {
                "IN THE NORTH HEDGE: GO THROUGH AND ON DOWN".into()
            } else {
                "FIND IT IN THE NORTH WALL AND GO THROUGH".into()
            },
            color: if arena { self.key_color() } else { 0xf2c53d },
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
    pub(super) fn update_gate(&mut self, dt: f64) {
        let Some(g) = self.gate else { return };
        self.gate_hint = (self.gate_hint - dt).max(0.0);
        let hp = self.hero.pos();
        let near = (hp.x - g.x as f64).abs() < 1.2 && (hp.y - g.y as f64).abs() < 1.7;
        if near && !self.gate_open && self.vault.is_some() {
            // The far portal of the vaults asks Mammon's toll instead of keys.
            if !self.toll_asked {
                self.ask_toll();
            }
            return;
        }
        if near && !self.gate_open && self.run.is_some() {
            // The realms after the vaults seal theirs until their last trap has been survived.
            if self.gate_hint <= 0.0 {
                // (Not every few seconds: a realm's last trap may be sprung right before it.)
                self.gate_hint = 12.0;
                self.audio.play(Sfx::Denied);
                let text = self.run_seal_hint();
                let color = self.run.as_ref().map(|r| r.sin().color()).unwrap_or(0xffffff);
                self.floater(g.x as f64, g.y as f64 + 0.6, text, 1.6, Some(color));
            }
            return;
        }
        if near && !self.gate_open && self.gate_hint <= 0.0 {
            self.gate_hint = 3.0;
            self.audio.play(Sfx::Denied);
            let standing = self.boss.as_ref().map(|b| b.alive()).or(self.court.as_ref().map(|c| !c.slain));
            let (text, color) = match standing {
                Some(true) => (format!("LOCKED: {} HOLDS THE KEY", self.boss_name()), self.key_color()),
                Some(false) => (format!("LOCKED: TAKE {}", self.key_name()), self.key_color()),
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
            if self.arena() {
                // Out of the Cyclops' meadow (or a prince's court): the victory, and then the old man.
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
    pub(super) fn win(&mut self) {
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
            sub: match self.realm {
                Some(sin) => format!("{} WAITS IN HIS COURT...", sin.prince().to_uppercase()),
                None => "SOMETHING VAST STIRS IN THE MEADOW...".into(),
            },
            color: 0xf2c53d,
            life: CLEARED_SECONDS as f32,
            max: CLEARED_SECONDS as f32,
        });
        self.enter(Screen::Cleared);
    }
}
