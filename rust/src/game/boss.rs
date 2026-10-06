//! The boss fight: the Cyclops (or a prince standing in for one), what it throws, how it is
//! wounded, burned, bled and poisoned, and the key it drops.

use super::{save, Banner, Game, GameEvent, Screen, BLEED_TICK, FIRE_COLOR, POISON_TICK};
use crate::audio::Sfx;
use crate::logic::boss::{BossEvent, ShotKind, CONTACT_RADIUS, POINTS_CYCLOPS};
use crate::logic::gear::{COINS_BOSS, RUNES_BOSS};
use crate::logic::hero::CONTACT_DAMAGE;
use crate::logic::heroes::HeroKind;
use crate::logic::juice::FPS;
use crate::logic::levels::ARENA;
use crate::logic::progression;
use crate::logic::progression::path;
use crate::logic::types::{dist_sq, Tile, P, V};

/// How close he must come to the Cyclops' red diamond key to take it.
const RED_KEY_REACH: f64 = 0.7;

/// From the second dungeon on the Cyclops' hide turns this much of every blow.
const BOSS_ARMOR: i32 = 1;

/// Missiles a mirror-shield turns back strike the Cyclops this hard.
const REFLECT_DAMAGE: i32 = 2;

/// Dragonfire burns the Cyclops this hard when it touches him.
const DRAGONFIRE_DAMAGE: i32 = 3;

/// Freeze-frame on a blow to the boss.
const HITSTOP_BOSS: f64 = 0.035;

impl Game {
    pub(super) fn update_boss(&mut self, dt: f64) {
        let hero_pos = self.hero.pos();
        let mut events = std::mem::take(&mut self.boss_events);
        let was_active = self.boss.as_ref().map(|b| b.active()).unwrap_or(false);
        if let Some(b) = &mut self.boss {
            b.update(dt, hero_pos, &mut self.boss_shots, &mut events);
        }
        let now_active = self.boss.as_ref().map(|b| b.active()).unwrap_or(false);
        if now_active && !was_active {
            self.audio.set_tempo_step(1);
        }
        for e in events.drain(..) {
            self.boss_event(e);
        }
        self.boss_events = events;
        self.update_boss_shots(dt);
        if self.screen != Screen::Playing {
            return;
        }
        if self.boss_bleed > 0.0 {
            self.boss_bleed -= dt;
            self.bleed_tick -= dt;
            if let Some(bp) = self.boss.as_ref().filter(|b| b.active()).map(|b| b.pos) {
                if self.bleed_tick <= 0.0 {
                    self.bleed_tick = BLEED_TICK;
                    self.wound_boss(1, bp);
                }
                if self.rng.next() < dt * 20.0 {
                    self.view.sparkle(&self.level.grid, bp.x, bp.y, 0.6 + self.rng.next() as f32, self.bleed_color, 2);
                }
            } else {
                self.boss_bleed = 0.0;
            }
        }
        if self.poison.0 > 0 {
            self.poison.1 -= dt;
            self.poison.2 -= dt;
            if let Some(bp) = self.boss.as_ref().filter(|b| b.active()).map(|b| b.pos) {
                if self.poison.2 <= 0.0 {
                    self.poison.2 = POISON_TICK;
                    self.wound_boss(self.poison.0, bp);
                }
                if self.rng.next() < dt * 14.0 {
                    self.view.sparkle(&self.level.grid, bp.x, bp.y, 0.8, 0x6ae86a, 2);
                }
            }
            if self.poison.1 <= 0.0 {
                self.poison = (0, 0.0, 0.0);
            }
        }
        // Touching it hurts, unless he sails over it; a thorned shield or dragonfire bites back.
        if let Some(b) = &self.boss {
            let near = dist_sq(b.pos, hero_pos) < CONTACT_RADIUS * CONTACT_RADIUS;
            if b.harmful() && b.hover < 1.3 && near && !self.hero.airborne() && !self.hero.invisible() {
                let bp = b.pos;
                let open = !self.hero.invulnerable() && !self.hero.invincible();
                let thorns = self.hero.shield.map(|s| progression::ward(s.tier).thorns).unwrap_or(0);
                let fire = if self.hero.fx.fire > 0.0 { DRAGONFIRE_DAMAGE } else { 0 };
                self.hurt_hero(CONTACT_DAMAGE);
                if open && thorns + fire > 0 {
                    self.wound_boss(thorns + fire, bp);
                }
            }
        }
        self.take_red_key();
    }

    /// Walking over the red diamond key the Cyclops dropped: the arena's gate is unlocked.
    pub(super) fn take_red_key(&mut self) {
        let Some(k) = self.red_key else { return };
        if !self.hero.alive || dist_sq(self.hero.pos(), k.v()) > RED_KEY_REACH * RED_KEY_REACH {
            return;
        }
        self.red_key = None;
        self.keys_got = self.keys_total;
        let (x, y) = (k.x as f64, k.y as f64);
        let color = self.key_color();
        self.view.sparkle(&self.level.grid, x, y, 1.0, color, 50);
        self.view.flash_light(&self.level.grid, k.v(), color);
        self.audio.play(Sfx::Milestone);
        self.floater(x, y - 0.8, self.key_name(), 2.0, Some(color));
        self.unlock_gate();
    }

    /**
     * The Cyclops' red diamond key falls where it died, or on the nearest
     * open grass if it died over stone or water: a tile he can walk onto.
     */
    pub(super) fn drop_red_key(&mut self, at: V) {
        // (In a prince's court, anywhere on its floor.)
        let (x0, y0, x1, y1) = if self.in_court() { (0, 0, self.level.grid.width - 1, self.level.grid.height - 1) } else { ARENA };
        let g = &self.level.grid;
        let spot = g
            .cells()
            .filter(|&(x, y, t)| {
                x >= x0 && x <= x1 && y >= y0 && y <= y1 && matches!(t, Tile::Open | Tile::Charge) && g.is_walkable(x, y, false)
            })
            .map(|(x, y, _)| P::new(x, y))
            .min_by(|a, b| dist_sq(a.v(), at).total_cmp(&dist_sq(b.v(), at)));
        self.red_key = spot;
        if let Some(k) = spot {
            let color = self.key_color();
            self.view.sparkle(&self.level.grid, k.x as f64, k.y as f64, 1.2, color, 40);
        }
    }

    pub(super) fn boss_event(&mut self, e: BossEvent) {
        let grid = &self.level.grid;
        match e {
            BossEvent::Roar => {
                self.audio.play(Sfx::BossRoar);
                self.view.shake(0.35);
            }
            BossEvent::Charge => self.audio.play(Sfx::BossCharge),
            BossEvent::Fire(kind) => {
                self.audio.play(if kind == ShotKind::Boulder { Sfx::BossHurl } else { Sfx::BossBolt });
                if let Some(b) = &self.boss {
                    let p = b.pos;
                    if kind == ShotKind::Boulder {
                        self.view.dust(grid, p.x, p.y, 0x8a6a40, 10, 2.0);
                    }
                }
            }
            BossEvent::Slam(p) => {
                self.audio.play(Sfx::BossSlam);
                self.view.shockwave(grid, p.x, p.y);
                self.view.dust(grid, p.x, p.y, 0x8a6a40, 34, 4.2);
                self.view.dust(grid, p.x, p.y, 0x5aa834, 16, 3.0);
                let d = dist_sq(p, self.hero.pos()).sqrt() as f32;
                self.view.shake(0.7 * (1.0 - d / 14.0).max(0.2));
                self.rumble(crate::input::Rumble::Quake((1.0 - d / 14.0).max(0.2)));
            }
            BossEvent::Enraged(phase) => {
                self.audio.set_tempo_step(phase as u32);
                self.juice.banner = Some(Banner {
                    text: format!("{} IS {}", self.boss_name(), if phase == 2 { "ENRAGED" } else { "FURIOUS" }),
                    sub: if phase == 2 { "IT DIVES NOW   DODGE, THEN STRIKE WHILE IT IS DAZED".into() } else { "FINISH IT!".into() },
                    color: if phase == 2 { 0xff8a2a } else { 0xff4d4d },
                    life: 2.2,
                    max: 2.2,
                });
                self.drop_remedy();
            }
            BossEvent::Spasm(p) => {
                self.view.spark(grid, p.x, p.y, 0xffb060);
                self.view.dust(grid, p.x, p.y, 0x8a2a6a, 8, 3.0);
                self.view.shake(0.12);
                self.audio.play(Sfx::ArrowHit);
            }
            BossEvent::Died(p) => self.boss_slain(p),
        }
    }

    /// The boss falls (the Cyclops, or a prince in his court): the blast and the bounty, a relic,
    /// his key on the floor, and the checkpoint at the old man's (the sin's key his, in a court).
    pub(super) fn boss_slain(&mut self, p: V) {
        let grid = &self.level.grid;
        self.view.explode(grid, p.x, p.y, 0);
        self.view.sparkle(grid, p.x, p.y, 1.2, 0xf2c53d, 60);
        self.view.sparkle(grid, p.x, p.y, 1.0, 0xffffff, 30);
        self.view.dust(grid, p.x, p.y, 0x8a2a6a, 40, 5.0);
        self.view.shockwave(grid, p.x, p.y);
        self.view.shake(1.0);
        self.audio.play(Sfx::Explode);
        self.audio.play(Sfx::BossDeath);
        self.audio.set_threat(0.0);
        self.award(POINTS_CYCLOPS);
        let bounty = COINS_BOSS + 100 * self.dungeon as u32;
        self.earn(bounty);
        self.floater(p.x, p.y - 0.8, format!("+{bounty} {}", progression::coin(self.hero_kind)), 2.2, Some(0xffd84a));
        self.floater(p.x, p.y, format!("+{POINTS_CYCLOPS}"), 2.0, Some(0xf2c53d));
        let runes = RUNES_BOSS + self.dungeon as u32;
        self.progress.runes += runes;
        self.floater(p.x, p.y + 0.8, format!("+{runes} {}", progression::gems(self.hero_kind, runes)), 2.4, Some(0x9ad1ff));
        // Its downfall gives up a relic of the old heroes.
        self.take_relic(p);
        self.events.push(GameEvent::BossSlain);
        // It drops its key to the gate in the north hedge: the red diamond, or the sin's.
        self.drop_red_key(p);
        self.juice.banner = Some(Banner {
            text: format!("{} IS SLAIN", self.boss_name()),
            sub: format!(
                "TAKE {} TO THE GATE {}",
                self.key_name(),
                if self.in_court() { "IN THE SOUTH WALL" } else { "IN THE NORTH HEDGE" }
            ),
            color: self.key_color(),
            life: 3.5,
            max: 3.5,
        });
        self.save_progress();
        if let Some(sin) = self.realm {
            // The sin is conquered: its key is his, for the seal.
            self.sins_cleared |= sin.bit();
            self.checkpoint(save::Stage::SinShop(sin));
        } else {
            self.checkpoint(save::Stage::Shop(self.dungeon));
        }
    }

    /// A green cross appears on open grass away from the Cyclops: a breather between phases.
    pub(super) fn drop_remedy(&mut self) {
        let (x0, y0, x1, y1) = ARENA;
        let boss = self.boss.as_ref().map(|b| b.pos).unwrap_or_default();
        let hero = self.hero.pos();
        let mut best: Option<(f64, P)> = None;
        for _ in 0..40 {
            let p = P::new(x0 + self.rng.index((x1 - x0 + 1) as usize) as i32, y0 + self.rng.index((y1 - y0 + 1) as usize) as i32);
            if self.level.grid.get(p.x, p.y) != Tile::Open {
                continue;
            }
            // Not on top of him, not under the eye.
            let score = dist_sq(p.v(), boss).sqrt().min(6.0) - (dist_sq(p.v(), hero).sqrt() - 4.0).abs();
            if best.map(|(s, _)| score > s).unwrap_or(true) {
                best = Some((score, p));
            }
        }
        if let Some((_, p)) = best {
            self.level.grid.set(p.x, p.y, Tile::Charge);
            self.view.add_charge(&self.level.grid, p.x, p.y);
            self.view.sparkle(&self.level.grid, p.x as f64, p.y as f64, 0.4, 0x5cff6a, 20);
            self.floater(p.x as f64, p.y as f64, "HEALTH", 1.6, Some(0x5cff6a));
        }
    }

    /// Bolts and boulders: they fly until they hit rock, Ragnar, or run out.
    pub(super) fn update_boss_shots(&mut self, dt: f64) {
        let grid = &self.level.grid;
        let hero_pos = self.hero.pos();
        let hero_open = self.hero.alive && !self.hero.airborne();
        let mut hurt = 0;
        for s in self.boss_shots.iter_mut() {
            let mut remaining = dt;
            while remaining > 0.0 && s.alive {
                let step = remaining.min(0.02);
                remaining -= step;
                s.pos = V::new(s.pos.x + s.vel.x * step, s.pos.y + s.vel.y * step);
                s.rolled += (s.vel.x.hypot(s.vel.y)) * step;
                s.life -= step;
                let t = P::new(s.pos.x.round() as i32, s.pos.y.round() as i32);
                if s.life <= 0.0 || !grid.in_bounds(t.x, t.y) {
                    s.alive = false;
                } else if grid.get(t.x, t.y) == Tile::Rock {
                    s.alive = false;
                    let color = if s.kind == ShotKind::Boulder { 0x8a867c } else { 0xff7a3a };
                    self.view.spark(grid, s.pos.x, s.pos.y, color);
                    self.view.dust(grid, s.pos.x, s.pos.y, if s.kind == ShotKind::Boulder { 0x8a867c } else { 0x4a3620 }, 8, 2.0);
                } else if hero_open && dist_sq(s.pos, hero_pos).sqrt() < s.kind.radius() + 0.3 {
                    s.alive = false;
                    hurt = hurt.max(s.kind.damage());
                    self.view.spark(grid, s.pos.x, s.pos.y, 0xff7a3a);
                }
            }
        }
        self.boss_shots.retain(|s| s.alive);
        if hurt > 0 {
            let ward = self.hero.shield.map(|s| progression::ward(s.tier));
            let open = !self.hero.invulnerable() && !self.hero.invincible();
            if let (Some(w), true) = (ward, open) {
                if w.missile_proof {
                    // The missile smashes on the shield: it wears, but nothing gets through.
                    if let Some(s) = &mut self.hero.shield {
                        let floor = if w.unbreakable { 1 } else { 0 };
                        s.durability = (s.durability - (hurt as f64 * w.block).round() as i32).max(floor);
                    }
                    self.hero.invuln = 0.5;
                    self.hero.heal(w.aegis);
                    let hp = self.hero.pos();
                    self.audio.play(Sfx::Nodule);
                    self.view.sparkle(&self.level.grid, hp.x, hp.y, 0.5, 0x7fd7ff, 12);
                    self.floater(hp.x, hp.y, if w.reflect { "TURNED BACK" } else { "WARDED" }, 0.8, Some(0x7fd7ff));
                    if w.reflect {
                        if let Some(bp) = self.boss_target() {
                            self.view.zap(&self.level.grid, hp, bp);
                            self.wound_boss(REFLECT_DAMAGE, bp);
                        }
                    }
                    if self.hero.shield.map(|s| s.durability <= 0).unwrap_or(false) {
                        self.hero.shield = None;
                        self.hero.shield_broke = true;
                        self.hurt_hero(0);
                    }
                    return;
                }
            }
            self.hurt_hero(hurt);
        }
    }

    /// A blow to the Cyclops from any weapon, with all the feedback it deserves.
    pub(super) fn wound_boss(&mut self, amount: i32, at: V) -> bool {
        self.damage_boss(amount, at, false)
    }

    /// An archangel's blow: exactly `amount`, dazed or not.
    pub(super) fn smite_boss(&mut self, amount: i32, at: V) -> bool {
        let landed = self.damage_boss(amount, at, true);
        if landed {
            self.floater(at.x, at.y - 1.1, format!("-{amount}"), 1.2, Some(0xfff1a8));
        }
        landed
    }

    pub(super) fn damage_boss(&mut self, amount: i32, at: V, exact: bool) -> bool {
        let mut events = std::mem::take(&mut self.boss_events);
        let landed =
            self.boss.as_mut().map(|b| if exact { b.smite(amount, &mut events) } else { b.hit(amount, &mut events) }).unwrap_or(false);
        let dying = self.boss.as_ref().map(|b| !b.alive()).unwrap_or(false);
        if landed {
            let grid = &self.level.grid;
            self.audio.play(Sfx::BossHit);
            self.view.spark(grid, at.x, at.y, 0xffe0e0);
            self.view.dust(grid, at.x, at.y, 0x8a2a6a, 10, 2.6);
            self.view.sparkle(grid, at.x, at.y, 1.0, 0xfff1c0, 6);
            self.view.shake(0.16);
            if dying {
                // Its last scream pops everything still in the air.
                for s in self.boss_shots.drain(..) {
                    self.view.spark(grid, s.pos.x, s.pos.y, 0xffb060);
                }
            }
            // The world holds on the blow, and longer on its last.
            self.freeze_frames(if dying { 0.12 } else { HITSTOP_BOSS } * FPS);
            self.award(10 * amount as u32);
        }
        for e in events.drain(..) {
            self.boss_event(e);
        }
        self.boss_events = events;
        landed
    }

    /// Where the Cyclops is, if it can be struck at all.
    pub(super) fn boss_target(&self) -> Option<V> {
        self.boss.as_ref().filter(|b| b.vulnerable()).map(|b| b.pos)
    }

    /**
     * A blow of `base` damage to the Cyclops, as everything he has makes it:
     * his elixirs, the Cyclops' armour (from the second dungeon on, unless the
     * blade ignores it), a critical hit (always, for a blade that never misses;
     * sometimes, with Seer's Sight), and triple from behind (Wolf's Fang).
     */
    pub(super) fn strike_boss(&mut self, base: i32, at: V) -> bool {
        let blade = progression::blade(self.hero_kind, self.hero.blade);
        let mut dmg = self.hero.blow(base);
        // A dull blade bites less.
        dmg = ((dmg as f64) * crate::logic::gear::edge_power(self.hero.edge)).round().max(1.0) as i32;
        if self.dungeon >= 1 && !blade.pierce_armor {
            dmg = (dmg - BOSS_ARMOR).max(1);
        }
        let mut tag = None;
        if blade.crit || (self.hero.sees() && self.rng.next() < path::SIGHT_CRIT) {
            dmg *= 2;
            tag = Some(("CRITICAL!", 0xffd84a));
        }
        let from = self.hero.pos();
        if self.hero.path >= 6 && self.boss.as_ref().map(|b| b.behind(from)).unwrap_or(false) {
            dmg *= path::FANG_MULT;
            tag = Some(("BACKSTAB!", 0xff5a3a));
        } else if self.hero.path >= 6 && self.hero_kind != HeroKind::Ragnar && self.boss.as_ref().map(|b| b.flank(from)).unwrap_or(false) {
            dmg *= path::FLANK_MULT;
            tag = Some(("FLANKED!", 0xffa050));
        }
        let landed = self.wound_boss(dmg, at);
        if let (true, Some((text, color))) = (landed, tag) {
            self.floater(at.x, at.y - 1.1, text, 1.0, Some(color));
        }
        landed
    }

    /// Sets the Cyclops burning for `seconds`.
    pub(super) fn set_boss_alight(&mut self, seconds: f64) {
        if self.boss_bleed <= 0.0 {
            self.bleed_tick = BLEED_TICK;
        }
        self.boss_bleed = self.boss_bleed.max(seconds);
        self.bleed_color = FIRE_COLOR;
    }
}
