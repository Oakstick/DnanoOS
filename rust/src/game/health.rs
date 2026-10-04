//! The hero's health: what his gifts do when he runs low, taking hits through shield and
//! invulnerability, and dying.

use super::{Banner, Game, Screen};
use crate::audio::Sfx;
use crate::logic::hero::DamageResult;
use crate::logic::heroes::HeroKind;
use crate::logic::progression;
use crate::logic::progression::{path, Tree};

impl Game {
    /**
     * The path's safety nets, watched every tick: the World Tree (path 10)
     * restores him once per dungeon when he falls to a quarter of his health.
     */
    pub(super) fn watch_health(&mut self) {
        if self.hero.path >= 10 && !self.restored && self.hero.alive && (self.hero.hp as f64) <= self.hero.max_hp as f64 * path::TREE_AT {
            self.restored = true;
            self.hero.restore();
            let hp = self.hero.pos();
            let n = progression::node(self.hero_kind, Tree::Path, 10);
            self.audio.play(Sfx::Heal);
            self.view.sparkle(&self.level.grid, hp.x, hp.y, 1.0, 0x6ae86a, 50);
            self.juice.hp_flash = 1.0;
            self.juice.hp_flash_color = 0x5cff6a;
            self.juice.banner =
                Some(Banner { text: n.ability.to_uppercase(), sub: "FULLY RESTORED".into(), color: 0x6ae86a, life: 2.0, max: 2.0 });
        }
    }

    /// Applies damage with shield and invincibility rules. Returns true if the hero died.
    pub(super) fn hurt_hero(&mut self, amount: i32) -> bool {
        let result = if amount > 0 { self.hero.take_damage(amount) } else { DamageResult::Immune };
        let hp = self.hero.pos();
        if self.hero.shield_broke {
            self.hero.shield_broke = false;
            self.shatter_shield();
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
                if self.immortal {
                    // The blow landed, and is forgotten at once.
                    self.keep_immortal();
                    return false;
                }
                if !self.hero.alive && self.hero.path >= 8 && self.hero_kind != HeroKind::Ragnar && !self.stood {
                    // The last stand (Seppuku Resolve, Blood of the Empire): death is held off, and he strikes triple.
                    self.stood = true;
                    self.hero.alive = true;
                    self.hero.hp = 1;
                    self.hero.last_stand = path::STAND_SECONDS;
                    self.hero.invuln = 0.5;
                    let n = progression::node(self.hero_kind, Tree::Path, 8);
                    self.audio.play(Sfx::Brutal);
                    self.view.sparkle(&self.level.grid, hp.x, hp.y, 1.2, 0xc8202a, 50);
                    self.view.shake(0.6);
                    self.juice.banner = Some(Banner {
                        text: n.ability.to_uppercase(),
                        sub: "YOU CANNOT DIE   TRIPLE DAMAGE".into(),
                        color: 0xff4a3a,
                        life: 2.2,
                        max: 2.2,
                    });
                    return false;
                }
                if !self.hero.alive && self.hero.path >= 9 && self.hero_kind == HeroKind::Ragnar && !self.revived {
                    // Valkyrie Call: once in each dungeon, a fatal blow is not the end.
                    self.revived = true;
                    self.hero.alive = true;
                    self.hero.hp = self.hero.max_hp / 2;
                    self.hero.invuln = 2.0;
                    let n = progression::node(self.hero_kind, Tree::Path, 9);
                    self.audio.play(Sfx::Heal);
                    self.view.sparkle(&self.level.grid, hp.x, hp.y, 1.4, 0xfff1a8, 60);
                    self.juice.banner = Some(Banner {
                        text: n.ability.to_uppercase(),
                        sub: "RAISED AGAIN WITH HALF YOUR HEALTH".into(),
                        color: 0xfff1a8,
                        life: 2.2,
                        max: 2.2,
                    });
                    return false;
                }
                if !self.hero.alive {
                    self.kill_hero();
                    return true;
                }
                false
            }
        }
    }

    /// His shield has broken: for good, struck off until the old man forges it anew.
    pub(super) fn shatter_shield(&mut self) {
        let hp = self.hero.pos();
        self.audio.play(Sfx::ShieldBreak);
        self.view.dust(&self.level.grid, hp.x, hp.y, 0x8a6a40, 16, 3.0);
        self.view.shake(0.3);
        let h = self.progress.hero_mut(self.hero_kind);
        h.broken |= 1 << h.carry;
        self.save_progress();
        self.floater(hp.x, hp.y - 0.6, "SHIELD DESTROYED", 1.6, Some(0xff8a6a));
        // (That he can reforge it, the old man says himself.)
        self.counsel_gear(crate::logic::war::counsel::GearWarning::ShieldBroken);
    }

    /// F5: immortal on or off.
    pub(super) fn toggle_immortal(&mut self) {
        self.immortal = !self.immortal;
        self.keep_immortal();
        let hp = self.hero.pos();
        self.audio.play(if self.immortal { Sfx::Heal } else { Sfx::Nodule });
        self.floater(hp.x, hp.y - 1.0, if self.immortal { "GOD MODE ON" } else { "GOD MODE OFF" }, 1.4, Some(0xffb050));
    }

    /// While immortal, his health is always whole again.
    pub(super) fn keep_immortal(&mut self) {
        if self.immortal {
            self.hero.alive = true;
            self.hero.hp = self.hero.max_hp;
        }
    }

    pub(super) fn kill_hero(&mut self) {
        self.counsel_fell();
        self.audio.play(Sfx::Death);
        self.audio.set_frenzy(false);
        self.audio.set_threat(0.0);
        self.frenzy.cancel();
        let hp = self.hero.pos();
        self.view.explode(&self.level.grid, hp.x, hp.y, 0);
        self.view.shake(0.8);
        self.enter(Screen::Dying);
    }
}
