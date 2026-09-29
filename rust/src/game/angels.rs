//! The archangels' blessings in play (see `logic::angels` for what each
//! does): taken at their shrines, carried from dungeon to dungeon and into
//! the boss fights, spent with LT or R. Michael's fire turns foes to the
//! hero's side, Gabriel's thunder strikes down every foe near him, Raphael's
//! beam cuts down all in its path, and Uriel's wave rolls out over the whole
//! quarter.

use super::{Game, Screen};
use crate::audio::Sfx;
use crate::logic::angels::*;
use crate::logic::boss::BODY_RADIUS;
use crate::logic::items::RangedKind;
use crate::logic::projectile::Projectile;
use crate::logic::systems::FOE_RADIUS;
use crate::logic::types::{dist_sq, Tile, P, V};

/// Uriel's wave rolling out from where it was sent: how far it has come, and whether the boss has felt it.
#[derive(Clone, Copy, Debug)]
pub struct HolyWave {
    pub origin: V,
    pub radius: f64,
    pub boss_struck: bool,
}

/// How far `p` lies from the segment `a`-`b`.
fn from_segment(a: V, b: V, p: V) -> f64 {
    let ab = V::new(b.x - a.x, b.y - a.y);
    let len2 = (ab.x * ab.x + ab.y * ab.y).max(1e-9);
    let t = (((p.x - a.x) * ab.x + (p.y - a.y) * ab.y) / len2).clamp(0.0, 1.0);
    dist_sq(p, V::new(a.x + ab.x * t, a.y + ab.y * t)).sqrt()
}

impl Game {
    /// The archangels who cannot be had now: those who have given this level, and Uriel while he rests.
    pub fn angels_spent(&self) -> [bool; 4] {
        let mut spent = self.angels_used;
        spent[Archangel::Uriel.index()] |= self.uriel_rest > 0.0;
        spent
    }

    /**
     * Stepping onto an archangel's shrine: its blessing (in place of any other
     * he carried). Each archangel gives once a level, and Uriel only once in
     * ten minutes; a shrine showing one who has given turns the hero away,
     * and when all four have, every shrine stands grey.
     */
    pub(super) fn receive_blessing(&mut self, shrine: usize, angel: Archangel) {
        let at = self.diamonds[shrine].tile;
        if self.angels_spent()[angel.index()] {
            if self.time - self.refused_at > 2.0 {
                self.refused_at = self.time;
                self.audio.play(Sfx::Denied);
                let text = if self.angels_used[angel.index()] {
                    format!("{} HAS GIVEN HERE", angel.name().to_uppercase())
                } else {
                    let left = self.uriel_rest.ceil() as u32;
                    format!("URIEL RESTS {}:{:02}", left / 60, left % 60)
                };
                self.floater(at.x as f64, at.y as f64 - 0.8, text, 1.4, Some(0x9a8a8a));
            }
            return;
        }
        self.angels_used[angel.index()] = true;
        if self.angels_used.iter().all(|&u| u) {
            // All four have given: the shrines stand grey for the rest of the level.
            for d in self.diamonds.iter_mut().filter(|d| d.kind == crate::logic::items::DiamondKind::Angel) {
                d.active = false;
                d.respawn_timer = f64::INFINITY;
            }
        }
        self.blessing = Some(Blessing::of(angel));
        self.audio.play(Sfx::Milestone);
        let (x, y) = (at.x as f64, at.y as f64);
        self.view.sparkle(&self.level.grid, x, y, 1.2, angel.color(), 40);
        self.view.flash_light(&self.level.grid, V::new(x, y), angel.color());
        self.floater(x, y - 0.9, format!("{}'S BLESSING", angel.name().to_uppercase()), 1.8, Some(angel.color()));
        self.floater(x, y - 0.3, format!("{}   LT / R  x{}", angel.power().to_uppercase(), angel.charges()), 2.2, Some(0xe8dcc0));
    }

    /// LT or R: one use of the blessing he carries.
    pub(super) fn invoke_blessing(&mut self) {
        let Some(mut b) = self.blessing else { return };
        if !self.hero.alive || b.charges == 0 {
            return;
        }
        match b.angel {
            Archangel::Michael => self.michaels_fire(),
            Archangel::Gabriel => self.gabriels_thunder(),
            Archangel::Raphael => self.raphaels_beam(),
            Archangel::Uriel => self.uriels_wave(),
        }
        b.charges -= 1;
        self.blessing = (b.charges > 0).then_some(b);
    }

    /// Michael: a bolt of blue fire along his aim, seeking a foe.
    fn michaels_fire(&mut self) {
        let hp = self.hero.pos();
        self.projectiles.push(Projectile::new(hp, self.hero.aim(), RangedKind::Grace));
        self.audio.play(Sfx::MichaelFire);
        self.view.sparkle(&self.level.grid, hp.x, hp.y, 0.6, Archangel::Michael.color(), 14);
    }

    /// Michael's fire struck foe `i`: for a while it fights on the hero's side.
    pub(super) fn turn_foe(&mut self, i: usize) {
        self.enemies[i].turn(ALLY_SECONDS);
        let p = self.enemies[i].pos();
        let color = Archangel::Michael.color();
        self.audio.play(Sfx::MichaelTurn);
        self.view.sparkle(&self.level.grid, p.x, p.y, 1.0, color, 30);
        self.floater(p.x, p.y - 0.8, "TURNED", 1.2, Some(color));
    }

    /// Gabriel: thunder from a clear sky; every foe within reach is struck dead, a boss wounded.
    fn gabriels_thunder(&mut self) {
        let hp = self.hero.pos();
        let color = Archangel::Gabriel.color();
        let grid = &self.level.grid;
        self.view.shake(1.0);
        self.audio.play(Sfx::GabrielThunder);
        // A few bolts about him for the look of it, even with nothing near.
        for k in 0..4 {
            let a = k as f64 * std::f64::consts::FRAC_PI_2 + self.rng.next();
            self.view.thunder(grid, V::new(hp.x + a.cos() * 2.2, hp.y + a.sin() * 2.2), color);
        }
        let doomed: Vec<usize> = (0..self.enemies.len())
            .filter(|&i| self.enemies[i].is_targetable() && dist_sq(self.enemies[i].pos(), hp) <= THUNDER_RADIUS * THUNDER_RADIUS)
            .collect();
        for i in doomed {
            let p = self.enemies[i].pos();
            self.view.thunder(&self.level.grid, p, color);
            self.slay_enemy(i);
        }
        if let Some(bp) = self.boss_target() {
            if dist_sq(bp, hp).sqrt() - BODY_RADIUS <= THUNDER_RADIUS {
                self.view.thunder(&self.level.grid, bp, color);
                self.smite_boss(Archangel::Gabriel.boss_damage(), bp);
            }
        }
        self.floater(hp.x, hp.y - 1.0, "GABRIEL'S THUNDER", 1.4, Some(color));
    }

    /// Raphael: a beam along his aim, as far as solid rock lets it; all in its path dies.
    fn raphaels_beam(&mut self) {
        let hp = self.hero.pos();
        let aim = self.hero.aim();
        let grid = &self.level.grid;
        let mut reach = 0.5;
        while reach < BEAM_REACH {
            let d = reach + 0.25;
            let t = grid.wrap(P::new((hp.x + aim.x * d).round() as i32, (hp.y + aim.y * d).round() as i32));
            if grid.get(t.x, t.y) == Tile::Rock {
                break;
            }
            reach = d;
        }
        let end = V::new(hp.x + aim.x * reach, hp.y + aim.y * reach);
        let color = Archangel::Raphael.color();
        self.view.beam(grid, hp, end, color);
        self.view.shake(0.5);
        self.audio.play(Sfx::RaphaelBeam);
        let doomed: Vec<usize> = (0..self.enemies.len())
            .filter(|&i| self.enemies[i].is_targetable() && from_segment(hp, end, self.enemies[i].pos()) <= BEAM_HALF_WIDTH + FOE_RADIUS * 0.5)
            .collect();
        for i in doomed {
            self.slay_enemy(i);
        }
        if let Some(bp) = self.boss_target() {
            if from_segment(hp, end, bp) <= BEAM_HALF_WIDTH + BODY_RADIUS {
                self.smite_boss(Archangel::Raphael.boss_damage(), bp);
            }
        }
    }

    /// Uriel: a great wave rolls out from him (see `update_angels`), and Uriel rests ten minutes.
    fn uriels_wave(&mut self) {
        let hp = self.hero.pos();
        self.holy_wave = Some(HolyWave { origin: hp, radius: 0.0, boss_struck: false });
        self.uriel_rest = URIEL_REST;
        let color = Archangel::Uriel.color();
        self.view.holy_wave(&self.level.grid, hp, color);
        self.view.shake(0.8);
        self.audio.play(Sfx::UrielWave);
        self.floater(hp.x, hp.y - 1.0, "URIEL'S WAVE", 1.6, Some(color));
    }

    /// Each tick of play: Uriel rests, his wave rolls on, and the turned foes hunt until their time is up.
    pub(super) fn update_angels(&mut self, dt: f64) {
        self.uriel_rest = (self.uriel_rest - dt).max(0.0);
        if let Some(mut w) = self.holy_wave.take() {
            w.radius += WAVE_SPEED * dt;
            let doomed: Vec<usize> = (0..self.enemies.len())
                .filter(|&i| self.enemies[i].is_targetable() && dist_sq(self.enemies[i].pos(), w.origin) <= w.radius * w.radius)
                .collect();
            for i in doomed {
                let p = self.enemies[i].pos();
                self.view.sparkle(&self.level.grid, p.x, p.y, 0.8, Archangel::Uriel.color(), 16);
                self.slay_enemy(i);
                if self.screen != Screen::Playing {
                    return;
                }
            }
            if !w.boss_struck {
                if let Some(bp) = self.boss_target() {
                    if dist_sq(bp, w.origin).sqrt() - BODY_RADIUS <= w.radius && self.smite_boss(Archangel::Uriel.boss_damage(), bp) {
                        w.boss_struck = true;
                    }
                }
            }
            if w.radius < WAVE_RADIUS {
                self.holy_wave = Some(w);
            }
        }
        // The turned: a foe they touch dies; when their time runs out they go back to the dark.
        let foes: Vec<(usize, V)> = self.enemies.iter().enumerate().filter(|(_, e)| e.is_targetable()).map(|(i, e)| (i, e.pos())).collect();
        for i in 0..self.enemies.len() {
            if !self.enemies[i].is_ally() {
                continue;
            }
            self.enemies[i].b.ally -= dt;
            let at = self.enemies[i].pos();
            if self.enemies[i].b.ally <= 0.0 {
                self.enemies[i].b.ally = 0.0;
                self.enemies[i].kill();
                self.view.sparkle(&self.level.grid, at.x, at.y, 0.8, Archangel::Michael.color(), 20);
                continue;
            }
            for &(j, p) in &foes {
                if self.enemies[j].is_targetable() && dist_sq(p, at) <= ALLY_REACH * ALLY_REACH {
                    self.view.spark(&self.level.grid, p.x, p.y, Archangel::Michael.color());
                    self.slay_enemy(j);
                }
            }
        }
    }

    /// The nearest foe for a turned one at `from` to hunt, if any.
    pub(super) fn quarry_for(&self, from: V) -> Option<V> {
        self.enemies.iter().filter(|e| e.is_targetable()).map(|e| e.pos()).min_by(|a, b| dist_sq(*a, from).total_cmp(&dist_sq(*b, from)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_beam_catches_what_lies_along_it() {
        let (a, b) = (V::new(0.0, 0.0), V::new(10.0, 0.0));
        assert!(from_segment(a, b, V::new(5.0, 0.5)) < 0.6);
        assert!(from_segment(a, b, V::new(12.0, 0.0)) > 1.9, "not past its end");
        assert!(from_segment(a, b, V::new(-1.0, 0.0)) > 0.9, "not behind him");
    }
}
