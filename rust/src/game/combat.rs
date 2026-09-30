//! The hero's fighting: the swing and what his blade carries, arrows and thrown weapons, bombs,
//! holy fire, the spirits at his side, Loki's trick, lightning and the Bifrost step.

use super::{Game, BLEED_TICK, FIRE_COLOR, POISON_TICK};
use crate::audio::Sfx;
use crate::logic::boss::BODY_RADIUS;
use crate::logic::heroes::HeroKind;
use crate::logic::items::{Bomb, RangedKind, WeaponKind};
use crate::logic::progression::{path, Tree};
use crate::logic::projectile::{spawn_shots, ShotState};
use crate::logic::systems::{
    blast_damage, break_rocks, find_blade_targets, find_projectile_hits, in_blade_arc, in_blast, tile_in_reach, BREAK_RADIUS,
    CHAIN_DELAY_MAX, CHAIN_DELAY_MIN,
};
use crate::logic::types::{dist_sq, P, V};
use crate::logic::{items, progression};

/// Damage the Cyclops takes from a bomb (the weapons carry their own).
const BOMB_DAMAGE: i32 = 6;

/// Chain lightning (a tier-10 weapon): how far each arc leaps from where it struck, and what it deals the Cyclops.
const ZAP_RANGE: f64 = 4.5;

/// What each arc of chain lightning deals the Cyclops.
const ZAP_DAMAGE: i32 = 2;

/// Venom (Miyamoto's serpent spear): the most stacks, how long they last after a hit, seconds between bites.
const POISON_MAX: i32 = 5;

/// How long venom lasts after the last hit that stacked it.
const POISON_SECONDS: f64 = 6.0;

/// How long flames burn on the Cyclops, and holy fire on a tile.
const FIRE_SECONDS: f64 = 3.0;

/// Loki's trick: how long foes stay befuddled, how long a decoy lasts and how often one appears.
const CONFUSE_SECONDS: f64 = 4.0;

/// How long a decoy of Loki's trick lasts.
const DECOY_SECONDS: f64 = 5.0;

/// Seconds between Loki's decoys.
const DECOY_EVERY: f64 = 10.0;

/// An ancestral spirit fighting beside the hero: where it floats, and seconds until its next blow.
#[derive(Clone, Copy, Debug)]
pub struct Spirit {
    pub pos: V,
    pub cooldown: f64,
    /// 1 when it has just struck, fading: its swing.
    pub strike: f64,
}

impl Game {
    /**
     * The one attack button (E, left click, X). Facing a shrine, it cycles the
     * shrine. Otherwise it swings the sword, or fires the ranged weapon that
     * has replaced it.
     */
    pub(super) fn attack(&mut self) {
        if !self.hero.alive {
            return;
        }
        let hp = self.hero.pos();
        let mut cycled = false;
        for i in 0..self.diamonds.len() {
            let d = &mut self.diamonds[i];
            if d.active && tile_in_reach(hp, self.hero.aim(), d.tile) {
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

    /**
     * A swing of his armory blade: its reach and arc slay every foe caught
     * and strike the Cyclops; then whatever rides on the blade: a piercing
     * wave, a stunning oath, a trick, a bleeding cut.
     */
    pub(super) fn perform_attack(&mut self) {
        let blade = progression::blade(self.hero_kind, self.hero.blade);
        if blade.arrows > 0 {
            self.rain_arrows(blade);
            return;
        }
        self.audio.play(Sfx::Slash);
        self.view.sword_slash();
        if blade.lunge > 0.0 {
            // Gale Dash: a gust carries him forward into the cut.
            if let Some((from, to)) = self.hero.dash(&self.level.grid, blade.lunge as i32, 0.0) {
                self.view.zap(&self.level.grid, from, to);
                self.view.dust(&self.level.grid, from.x, from.y, 0xd8f0e8, 10, 2.0);
                self.audio.play(Sfx::Whoosh);
            }
        }
        if blade.flash > 0.0 {
            self.solar_flash(blade);
        }
        let Some((reach, half_width, behind)) = blade.swing else { return };
        let hp = self.hero.pos();
        let aim = self.hero.aim();
        let mut hits = find_blade_targets(hp, aim, &self.enemies, reach, half_width, behind);
        if blade.burn > 0.0 {
            // Sacred flames lick beyond the blade's reach.
            for i in find_blade_targets(hp, aim, &self.enemies, reach + 0.9, half_width + 0.8, behind) {
                if !hits.contains(&i) {
                    let p = self.enemies[i].pos();
                    self.view.sparkle(&self.level.grid, p.x, p.y, 0.8, FIRE_COLOR, 16);
                    hits.push(i);
                }
            }
        }
        let landed_any = !hits.is_empty();
        if landed_any {
            self.view.shake(0.14 + 0.02 * self.hero.blade as f32);
        }
        for i in hits {
            self.strike_enemy(i);
        }
        let mut struck_boss = false;
        if let Some(bp) = self.boss_target() {
            if in_blade_arc(hp, aim, bp, reach, half_width, behind, BODY_RADIUS) {
                struck_boss = self.strike_boss(blade.damage, bp);
            }
        }
        if blade.burn > 0.0 && struck_boss {
            self.set_boss_alight(blade.burn);
        }
        if blade.poison && struck_boss {
            self.poison =
                ((self.poison.0 + 1).min(POISON_MAX), POISON_SECONDS, if self.poison.0 == 0 { POISON_TICK } else { self.poison.2 });
        }
        if blade.blood > 0 && (landed_any || struck_boss) {
            // Muramasa drinks from the hand that holds it.
            self.hero.hp = (self.hero.hp - blade.blood).max(1);
            self.floater(hp.x, hp.y - 0.4, format!("-{}", blade.blood), 0.8, Some(0xc83a5a));
        }
        if blade.stealth > 0.0 {
            self.hero.grant_invisibility(self.hero.invisible_timer.max(blade.stealth));
        }
        if blade.fire_trail {
            self.lay_fire(hp, aim);
        }
        if blade.wave {
            // Angurvadel's stream of anguish: a wave of force down the line.
            self.projectiles.extend(spawn_shots(RangedKind::Wave, hp, aim));
        }
        if blade.stun > 0.0 {
            self.stun_around(hp, progression::STUN_RADIUS, blade.stun);
            if struck_boss {
                let staggered = self.boss.as_mut().and_then(|b| b.stagger().then_some(b.pos));
                if let Some(p) = staggered {
                    self.floater(p.x, p.y - 0.9, "STAGGERED", 1.0, Some(0xe0c070));
                }
            }
        }
        if blade.trick {
            self.loki_trick(hp);
        }
        if blade.bleed > 0.0 && struck_boss {
            if self.boss_bleed <= 0.0 {
                self.bleed_tick = BLEED_TICK;
            }
            self.boss_bleed = blade.bleed;
        }
    }

    /**
     * A bow of the heavens (Yumi, Simurg): arrows rain down out of the sky on
     * the nearest foes in range, one each, and two on the Cyclops if it is near.
     */
    pub(super) fn rain_arrows(&mut self, blade: progression::Blade) {
        let hp = self.hero.pos();
        let color = progression::node(self.hero_kind, Tree::Armory, self.hero.blade).color;
        self.audio.play(Sfx::Arrow);
        let mut left = blade.arrows;
        if let Some(bp) = self.boss_target() {
            if dist_sq(bp, hp) <= (progression::ARROW_RANGE + 1.0).powi(2) {
                for k in 0..2 {
                    let at = V::new(bp.x + (k as f64 - 0.5) * 0.6, bp.y);
                    self.view.rain(&self.level.grid, at, color);
                    self.strike_boss(blade.damage, bp);
                }
                if blade.burn > 0.0 {
                    self.set_boss_alight(blade.burn);
                }
                left -= 2;
            }
        }
        let mut near: Vec<(f64, usize)> = self
            .enemies
            .iter()
            .enumerate()
            .filter(|(_, e)| e.is_targetable())
            .map(|(i, e)| (dist_sq(e.pos(), hp), i))
            .filter(|&(d, _)| d <= progression::ARROW_RANGE * progression::ARROW_RANGE)
            .collect();
        near.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        for &(_, i) in near.iter().take(left) {
            let p = self.enemies[i].pos();
            self.view.rain(&self.level.grid, p, color);
            self.strike_enemy(i);
        }
        if near.is_empty() && left == blade.arrows {
            // Nothing in range: the volley falls ahead of him.
            let f = self.hero.aim();
            self.view.rain(&self.level.grid, V::new(hp.x + f.x * 3.0, hp.y + f.y * 3.0), color);
        }
    }

    /**
     * A sun-blade's flash (Amaterasu, Kaf Dagi): blinding light bursts from
     * him, every foe within reach is obliterated and the Cyclops, if near, seared.
     */
    pub(super) fn solar_flash(&mut self, blade: progression::Blade) {
        let hp = self.hero.pos();
        let r = blade.flash;
        self.view.shockwave(&self.level.grid, hp.x, hp.y);
        self.view.sparkle(&self.level.grid, hp.x, hp.y, 1.4, 0xfff1a8, 60);
        self.view.flash_light(&self.level.grid, hp, 0xfff1c0);
        self.view.shake(0.4);
        self.audio.play(Sfx::Zap);
        let doomed: Vec<usize> =
            self.enemies.iter().enumerate().filter(|(_, e)| e.is_targetable() && dist_sq(e.pos(), hp) <= r * r).map(|(i, _)| i).collect();
        for i in doomed {
            self.slay_enemy(i);
        }
        if let Some(bp) = self.boss_target() {
            if dist_sq(bp, hp) <= (r + 1.0) * (r + 1.0) {
                self.strike_boss(blade.damage, bp);
            }
        }
    }

    /// A greatsword's holy fire: the tiles ahead of him along `aim` burn for a few seconds.
    pub(super) fn lay_fire(&mut self, from: V, aim: V) {
        let g = &self.level.grid;
        let o = g.wrap(P::new(from.x.round() as i32, from.y.round() as i32));
        let mut last = o;
        for k in 1..=6 {
            let d = k as f64 * 0.5;
            let t = g.wrap(P::new((from.x + aim.x * d).round() as i32, (from.y + aim.y * d).round() as i32));
            if t == last {
                continue;
            }
            last = t;
            if !g.is_walkable(t.x, t.y, false) {
                break;
            }
            match self.fires.iter_mut().find(|(p, _)| *p == t) {
                Some(f) => f.1 = FIRE_SECONDS,
                None => self.fires.push((t, FIRE_SECONDS)),
            }
        }
        self.audio.play(Sfx::Fuse);
    }

    /// The holy fire burns out; while it lasts, foes on it burn to ash and the Cyclops over it catches.
    pub(super) fn update_fires(&mut self, dt: f64) {
        for f in self.fires.iter_mut() {
            f.1 -= dt;
        }
        self.fires.retain(|f| f.1 > 0.0);
        if self.fires.is_empty() {
            return;
        }
        let burning: Vec<usize> = self
            .enemies
            .iter()
            .enumerate()
            .filter(|(_, e)| e.is_targetable() && self.fires.iter().any(|(p, _)| *p == e.occupied()))
            .map(|(i, _)| i)
            .collect();
        for i in burning {
            let p = self.enemies[i].pos();
            self.view.sparkle(&self.level.grid, p.x, p.y, 0.8, FIRE_COLOR, 16);
            self.slay_enemy(i);
        }
        if let Some(bp) = self.boss_target() {
            if self.fires.iter().any(|(p, _)| dist_sq(p.v(), bp) < 1.6 * 1.6) {
                self.set_boss_alight(1.0);
            }
        }
    }

    /**
     * The ancestral spirits (path 9 of Mehmed and Miyamoto): they float round
     * him and strike down any foe that comes near, or the Cyclops.
     */
    pub(super) fn update_spirits(&mut self, dt: f64) {
        let (count, every) = progression::spirits(self.hero_kind);
        let want = if self.hero.path >= 9 && self.hero.alive { count } else { 0 };
        let hp = self.hero.pos();
        while self.spirits.len() < want {
            let k = self.spirits.len() as f64;
            self.spirits.push(Spirit { pos: hp, cooldown: every * (0.5 + k / count.max(1) as f64), strike: 0.0 });
        }
        self.spirits.truncate(want);
        let n = self.spirits.len().max(1) as f64;
        for i in 0..self.spirits.len() {
            let a = self.time * 0.9 + i as f64 / n * std::f64::consts::TAU;
            let goal = V::new(hp.x + a.cos() * 1.1, hp.y + a.sin() * 1.1);
            let k = 1.0 - (-4.0 * dt).exp();
            let sp = &mut self.spirits[i];
            sp.pos = V::new(sp.pos.x + (goal.x - sp.pos.x) * k, sp.pos.y + (goal.y - sp.pos.y) * k);
            sp.cooldown -= dt;
            sp.strike = (sp.strike - dt * 3.0).max(0.0);
            if sp.cooldown > 0.0 {
                continue;
            }
            let at = sp.pos;
            let reach = path::SPIRIT_REACH;
            let foe = self
                .enemies
                .iter()
                .enumerate()
                .filter(|(_, e)| e.is_dangerous() || e.is_stunned())
                .map(|(j, e)| (dist_sq(e.pos(), at), j))
                .filter(|&(d, _)| d <= reach * reach)
                .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            if let Some((_, j)) = foe {
                let p = self.enemies[j].pos();
                self.view.zap(&self.level.grid, at, p);
                self.slay_enemy(j);
            } else if let Some(bp) = self.boss_target().filter(|bp| dist_sq(*bp, at) <= (reach + 1.0).powi(2)) {
                self.view.zap(&self.level.grid, at, bp);
                self.wound_boss(1, bp);
            } else {
                continue;
            }
            self.spirits[i].cooldown = every;
            self.spirits[i].strike = 1.0;
        }
    }

    /// A stunning blow or a shield's clash: every foe within `radius` reels, harmless, for `seconds`.
    pub(super) fn stun_around(&mut self, at: V, radius: f64, seconds: f64) {
        let grid = &self.level.grid;
        let mut any = false;
        for e in self.enemies.iter_mut() {
            if (e.is_dangerous() || e.is_stunned()) && dist_sq(e.pos(), at) <= radius * radius {
                e.scare(grid, seconds);
                any = true;
            }
        }
        if any {
            self.view.shockwave(grid, at.x, at.y);
            self.audio.play(Sfx::Zap);
        }
    }

    /// Loki's trick: the foes around him lose their way, and now and then a decoy of him draws them off.
    pub(super) fn loki_trick(&mut self, at: V) {
        for e in self.enemies.iter_mut() {
            if dist_sq(e.pos(), at) <= progression::TRICK_RADIUS * progression::TRICK_RADIUS {
                e.confuse(CONFUSE_SECONDS);
            }
        }
        if self.decoy_cooldown <= 0.0 {
            self.decoy = Some((at, DECOY_SECONDS));
            self.decoy_cooldown = DECOY_EVERY;
            self.view.sparkle(&self.level.grid, at.x, at.y, 0.8, 0x9a4aff, 30);
            self.floater(at.x, at.y - 0.8, "DECOY", 1.2, Some(0xc89aff));
        }
    }

    pub(super) fn update_decoy(&mut self, dt: f64) {
        self.decoy_cooldown = (self.decoy_cooldown - dt).max(0.0);
        if let Some((p, t)) = &mut self.decoy {
            *t -= dt;
            if *t <= 0.0 {
                let at = *p;
                self.view.sparkle(&self.level.grid, at.x, at.y, 0.6, 0x9a4aff, 16);
                self.decoy = None;
            }
        }
    }

    /**
     * Chain lightning (a tier-10 weapon) from where it struck: it leaps to
     * the nearest foes in turn, up to `count`, and to the Cyclops if it is near.
     */
    pub(super) fn chain_lightning(&mut self, from: V, count: usize) {
        let mut at = from;
        let mut struck = 0;
        let mut done: Vec<usize> = Vec::new();
        while struck < count {
            let next = self
                .enemies
                .iter()
                .enumerate()
                .filter(|(i, e)| e.is_targetable() && !done.contains(i))
                .map(|(i, e)| (dist_sq(e.pos(), at), i))
                .filter(|&(d, _)| d <= ZAP_RANGE * ZAP_RANGE)
                .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            let Some((_, i)) = next else { break };
            let p = self.enemies[i].pos();
            self.view.zap(&self.level.grid, at, p);
            self.slay_enemy(i);
            done.push(i);
            at = p;
            struck += 1;
        }
        if let Some(bp) = self.boss_target() {
            if dist_sq(bp, from) <= (ZAP_RANGE + 1.0).powi(2) {
                self.view.zap(&self.level.grid, from, bp);
                self.strike_boss(ZAP_DAMAGE, bp);
                struck += 1;
            }
        }
        if struck > 0 {
            self.audio.play(Sfx::Zap);
            self.view.shake(0.2);
        }
    }

    /// Bifrost Step (path 5): a flash of spectral light carries him a few tiles ahead.
    pub(super) fn bifrost_step(&mut self) {
        if self.hero.path < 5 {
            return;
        }
        let Some((from, to)) = self.hero.try_step(&self.level.grid) else { return };
        let grid = &self.level.grid;
        // A flash of Bifrost's light, a cloud of cherry blossom, a gust of desert sand.
        let (trail, burst) = match self.hero_kind {
            HeroKind::Ragnar => (0x9af0ff, 0xfff1ff),
            HeroKind::Miyamoto => (0xffb0d0, 0xffe0f0),
            HeroKind::Mehmed => (0xe8d09a, 0xfff0c8),
        };
        self.audio.play(Sfx::Whoosh);
        self.view.zap(grid, from, to);
        self.view.sparkle(grid, from.x, from.y, 0.6, trail, 24);
        self.view.sparkle(grid, to.x, to.y, 0.8, burst, 30);
        if self.hero_kind != HeroKind::Ragnar {
            self.view.dust(grid, from.x, from.y, trail, 14, 2.2);
        }
    }

    /// Fires the held ranged weapon in the facing direction; the last shot hands the sword back.
    pub(super) fn fire(&mut self) {
        let Some(kind) = self.hero.try_fire() else { return };
        let shots = spawn_shots(kind, self.hero.pos(), self.hero.aim());
        self.projectiles.extend(shots);
        self.audio.play(Sfx::Arrow);
        if let RangedKind::Hurled(h, t) = kind {
            if progression::node(h, Tree::Armory, t).form == progression::Form::Gun {
                // A crack and a puff of powder smoke.
                let hp = self.hero.pos();
                self.audio.play(Sfx::ArrowHit);
                self.view.dust(&self.level.grid, hp.x, hp.y, 0xb8b8c0, 12, 1.8);
                self.view.shake(0.12);
            }
        }
        if !self.hero.holding_pickup() && self.hero.weapon != WeaponKind::Ranged(kind) {
            // The shrine weapon ran dry; his own is back in hand.
            let hp = self.hero.pos();
            let (name, color) = items::weapon_label(self.hero_kind, self.hero.weapon, self.hero.blade);
            self.floater(hp.x, hp.y, name.to_uppercase(), 1.2, Some(color));
        }
    }

    pub(super) fn update_projectiles(&mut self, dt: f64) {
        let hero_pos = self.hero.pos();
        // What seeking knives can chase.
        let mut prey: Vec<V> = self.enemies.iter().filter(|e| e.is_targetable()).map(|e| e.pos()).collect();
        prey.extend(self.boss_target());
        let mut i = 0;
        while i < self.projectiles.len() {
            let state = self.projectiles[i].update(dt, &self.level.grid, hero_pos, &prey);
            if self.projectiles[i].alive {
                let (pos, radius) = (self.projectiles[i].pos, self.projectiles[i].def.hit_radius);
                let hits = find_projectile_hits(pos, radius, &self.enemies, &self.projectiles[i].struck);
                let mut hit_at = hits.first().map(|_| pos);
                let grace = self.projectiles[i].kind == RangedKind::Grace;
                for e in hits {
                    if grace {
                        self.turn_foe(e);
                    } else {
                        self.strike_enemy(e);
                    }
                    self.projectiles[i].struck.insert(e);
                    self.audio.play(Sfx::ArrowHit);
                    if !self.projectiles[i].def.pierce {
                        self.projectiles[i].alive = false;
                        break;
                    }
                }
                // The boss counts as one more target, struck once per shot.
                if let Some(bp) = self.boss_target() {
                    let p = &self.projectiles[i];
                    if p.alive && !p.struck.contains(&usize::MAX) && dist_sq(p.pos, bp).sqrt() <= radius + BODY_RADIUS {
                        let base = p.def.damage;
                        let at = p.pos;
                        self.projectiles[i].struck.insert(usize::MAX);
                        if !self.projectiles[i].def.pierce {
                            self.projectiles[i].alive = false;
                        }
                        if grace {
                            self.smite_boss(base, at);
                        } else {
                            self.strike_boss(base, at);
                        }
                        hit_at = Some(at);
                        if let RangedKind::Hurled(h, t) = self.projectiles[i].kind {
                            if progression::blade(h, t).throw.map(|t| t.stagger).unwrap_or(false) {
                                let staggered = self.boss.as_mut().and_then(|b| b.stagger().then_some(b.pos));
                                if let Some(p) = staggered {
                                    self.floater(p.x, p.y - 0.9, "STAGGERED", 1.0, Some(0xe0c070));
                                }
                            }
                        }
                    }
                }
                // Mjolnir: its first strike of each throw calls down chain lightning.
                let chain = match self.projectiles[i].kind {
                    RangedKind::Hurled(h, t) => progression::blade(h, t).chain,
                    _ => 0,
                };
                if chain > 0 && !self.projectiles[i].struck.contains(&(usize::MAX - 1)) {
                    if let Some(at) = hit_at {
                        self.projectiles[i].struck.insert(usize::MAX - 1);
                        self.chain_lightning(at, chain);
                    }
                }
            } else if state == ShotState::Wall {
                let p = self.projectiles[i].pos;
                self.view.spark(&self.level.grid, p.x, p.y, 0xfff1c0);
                self.view.dust(&self.level.grid, p.x, p.y, 0x4a3620, 6, 1.6);
                self.view.shake(0.03);
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
    pub(super) fn place_bomb(&mut self) {
        if self.bombs_left == 0 || !self.hero.alive || self.hero.airborne() {
            return;
        }
        let at = self.hero.occupied();
        if self.bombs.iter().any(|b| b.tile == at) {
            return;
        }
        self.bombs.push(Bomb::dropped(at, if self.hero.free_move { self.hero.pos() } else { at.v() }));
        self.bombs_left -= 1;
        self.audio.play(Sfx::Fuse);
    }

    pub(super) fn update_bombs(&mut self, dt: f64) {
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
    pub(super) fn explode(&mut self, bomb: Bomb) {
        let c = bomb.tile;
        let broken = break_rocks(&mut self.level.grid, c, BREAK_RADIUS);
        let floor = self.level.grid.elevation(c.x, c.y);
        for t in &broken {
            // A blasted wall becomes floor at the bomb's level.
            self.level.grid.set_elevation(t.x, t.y, floor);
            self.view.remove_rock(&self.level.grid, t.x, t.y);
        }
        self.view.explode(&self.level.grid, c.x as f64, c.y as f64, broken.len());
        self.view.dust(&self.level.grid, c.x as f64, c.y as f64, 0x4a3620, 30, 4.0);
        // The closer the blast, the harder the kick.
        let d = dist_sq(c.v(), self.hero.pos()).sqrt() as f32;
        self.view.shake(0.75 * (1.0 - d / 12.0).max(0.15));
        self.audio.play(Sfx::Explode);

        for i in 0..self.enemies.len() {
            if self.enemies[i].is_targetable() && in_blast(c, self.enemies[i].pos()) {
                self.slay_enemy(i);
            }
        }
        if let Some(bp) = self.boss_target() {
            if in_blast(c, bp) {
                self.wound_boss(BOMB_DAMAGE, bp);
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
}
