//! What he picks up and drinks: golden keys and green crosses, the shrines, bottles lying about,
//! the hidden relic, and the potions on his belt.

use super::{Banner, FlyingCoin, Game, GameEvent, KEYS_PER_TEMPO_STEP};
use crate::audio::Sfx;
use crate::logic::gear::{BELT_SLOTS, COINS_KEY, RUNES_SHRINE};
use crate::logic::items::{ItemKind, Offer};
use crate::logic::systems::{POINTS_CHARGE, POINTS_NODULE};
use crate::logic::types::{PotionKind, Tile, P, V};
use crate::logic::{gear, progression};
use crate::render::view::floor_at;

/// Health a green cross gives back, capped at Ragnar's maximum.
const REMEDY_HEAL: i32 = 50;

impl Game {
    pub(super) fn collect(&mut self) {
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
                if self.keys_got.is_multiple_of(KEYS_PER_TEMPO_STEP) {
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
                if !self.arena() && self.keys_got < self.keys_total {
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

    /// Stepping onto an active shrine takes whatever it is showing.
    pub(super) fn visit_shrines(&mut self) {
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
    pub(super) fn visit_potions(&mut self) {
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
            self.take_potion(kind, o);
        }
    }

    /// A bottle picked up at `at`: onto the belt if there is room; an extra life (or a full belt)
    /// is drunk at once.
    pub(super) fn take_potion(&mut self, kind: PotionKind, at: P) {
        let def = gear::potion(self.hero_kind, kind);
        self.view.spark(&self.level.grid, at.x as f64, at.y as f64, def.color);
        if kind != PotionKind::Life && self.belt.len() < BELT_SLOTS {
            self.belt.push(kind);
            self.audio.play(Sfx::Start);
            self.floater(at.x as f64, at.y as f64, def.name.to_uppercase(), 1.4, Some(def.color));
        } else {
            self.quaff(kind);
        }
    }

    /// Stepping onto the hidden shrine takes its relic and a rune fragment.
    pub(super) fn visit_relic_shrine(&mut self) {
        let Some(t) = self.relic_shrine else { return };
        if self.hero.airborne() || self.hero.occupied() != t {
            return;
        }
        self.relic_shrine = None;
        self.progress.runes += RUNES_SHRINE;
        self.audio.play(Sfx::Milestone);
        self.view.sparkle(&self.level.grid, t.x as f64, t.y as f64, 1.0, 0x9ad1ff, 50);
        self.floater(
            t.x as f64,
            t.y as f64 + 0.6,
            format!("+{RUNES_SHRINE} {}", progression::gems(self.hero_kind, RUNES_SHRINE)),
            1.8,
            Some(0x9ad1ff),
        );
        self.take_relic(t.v());
        self.save_progress();
    }

    /**
     * A relic of the old heroes, from a hidden shrine or a fallen Cyclops: it
     * unseals the next sealed tier of the tree that has the most still sealed.
     */
    pub(super) fn take_relic(&mut self, at: V) {
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
                format!(
                    "{} {} OF {} UNSEALED   IT IS YOURS ONCE THE TIERS BEFORE IT ARE",
                    tree.step(),
                    tier,
                    tree.title(hero).to_uppercase()
                )
            },
            color: n.color,
            life: 3.2,
            max: 3.2,
        });
        self.view.sparkle(&self.level.grid, at.x, at.y, 1.2, n.color, 40);
    }

    /// Drinks the potion in belt slot `slot`, if there is one.
    pub(super) fn drink(&mut self, slot: usize) {
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
    pub(super) fn quaff(&mut self, kind: PotionKind) {
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

    /// Nuggets shown in the HUD: the ones still flying there are not counted yet.
    pub fn keys_shown(&self) -> u32 {
        self.keys_got.saturating_sub(self.juice.coins.len() as u32)
    }
}
