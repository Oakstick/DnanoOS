//! The archangels in play (see `logic::angels` for what each does): all four
//! brought by the first shrine and kept for the adventure, each shrine after
//! readying them, the chosen one called with R or RT and the next chosen with
//! F or the D-pad's up, each back 25 seconds after his call. Michael's fire
//! turns foes to the hero's side, Gabriel's thunder strikes down every foe
//! near him, Raphael's beam cuts down all in its path, and Uriel's wave rolls
//! out over the quarter. Before the first shrine only his patron is with him.

use super::{Game, Screen};
use crate::audio::Sfx;
use crate::input::Action;
use crate::logic::angels::*;
use crate::logic::boss::BODY_RADIUS;
use crate::logic::items::{DiamondKind, RangedKind};
use crate::logic::projectile::Projectile;
use crate::logic::systems::FOE_RADIUS;
use crate::logic::types::{dist_sq, Tile, P, V};

/// Uriel's wave rolling out from where it was sent: how far it has come, whom it has struck (a
/// duelist it only wounds is struck once), and whether the boss has felt it.
#[derive(Clone, Debug)]
pub struct HolyWave {
    pub origin: V,
    pub radius: f64,
    pub struck: Vec<usize>,
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
    /// The archangel he swore to at Hızır's, if any: with him before the shrines, and chosen first.
    pub fn patron(&self) -> Option<Archangel> {
        self.progress.hero(self.hero_kind).patron
    }

    /// The archangel the angel button calls now, if any is with him.
    pub fn chosen_angel(&self) -> Option<Archangel> {
        self.angels.current(self.patron())
    }

    /**
     * Stepping onto an archangel's shrine: the first brings all four to him for the rest of the
     * adventure (his patron chosen first); any after readies all four at once. Each shrine gives
     * once a level, then stands grey.
     */
    pub(super) fn visit_angel_shrine(&mut self, shrine: usize) {
        let at = self.diamonds[shrine].tile;
        let first = !self.angels.granted;
        if first {
            self.angels.granted = true;
            if let Some(p) = self.patron() {
                self.angels.chosen = p;
            }
        }
        self.angels.refresh();
        let d = &mut self.diamonds[shrine];
        d.active = false;
        d.respawn_timer = f64::INFINITY;
        self.audio.play(Sfx::Milestone);
        self.boy_glow(false);
        let (x, y) = (at.x as f64, at.y as f64);
        for a in ARCHANGELS {
            self.view.sparkle(&self.level.grid, x, y, 1.2, a.color(), 12);
        }
        self.view.flash_light(&self.level.grid, V::new(x, y), 0xfff1c0);
        self.juice.blessed = Some((first, crate::game::BLESSED_SECONDS));
        let text = if first { "THE FOUR ARCHANGELS ARE WITH YOU" } else { "THE ARCHANGELS ARE READY" };
        self.floater(x, y - 0.9, text, 1.8, Some(0xfff1c0));
    }

    /// The next button: the next of the four with him is chosen.
    pub(super) fn choose_next_angel(&mut self) {
        let patron = self.patron();
        if self.angels.choose_next(patron) {
            self.audio.play(Sfx::Nodule);
            self.juice.angel_chosen = 1.0;
        }
    }

    /// RT or R: the chosen archangel's power, if he has come back; otherwise how long he will be.
    pub(super) fn invoke_angel(&mut self) {
        if !self.hero.alive {
            return;
        }
        let patron = self.patron();
        let Some(angel) = self.angels.current(patron) else { return };
        if !self.angels.ready(angel, patron) {
            if self.time - self.refused_at > 1.0 {
                self.refused_at = self.time;
                self.audio.play(Sfx::Denied);
                let hp = self.hero.pos();
                let left = self.angels.wait[angel.index()].ceil() as u32;
                self.floater(hp.x, hp.y - 0.9, format!("{} IN {left}s", angel.name().to_uppercase()), 1.0, Some(0x9a8a8a));
            }
            return;
        }
        self.angels.call(angel);
        self.call_angel(angel);
    }

    /// The archangel's power itself.
    fn call_angel(&mut self, angel: Archangel) {
        match angel {
            Archangel::Michael => self.michaels_fire(),
            Archangel::Gabriel => self.gabriels_thunder(),
            Archangel::Raphael => self.raphaels_beam(),
            Archangel::Uriel => self.uriels_wave(),
        }
    }

    /// An archangel's power reaches foe `i`: a lesser foe dies; a duelist takes the power's blow.
    fn smite_foe(&mut self, i: usize, angel: Archangel) {
        if self.enemies[i].duel.is_some() {
            self.angel_blow(i, angel);
        } else {
            self.slay_enemy(i);
        }
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
        // A duelist (under the frame data) is not turned but awed: all its poise goes, and it staggers
        // (a prince, or Krampus, loses `PRINCE_SHARE` of his: shaken, not felled).
        let awed = self.enemies[i].duel.as_deref_mut().map(|d| {
            let prince = d.arch == crate::logic::combat::Archetype::Prince;
            let poise = d.f.max_poise * if prince { PRINCE_SHARE } else { 1.0 };
            d.f.shake(poise);
        });
        let p = self.enemies[i].pos();
        let color = Archangel::Michael.color();
        self.audio.play(Sfx::MichaelTurn);
        self.view.sparkle(&self.level.grid, p.x, p.y, 1.0, color, 30);
        self.floater(p.x, p.y - 0.8, if awed.is_some() { "AWED" } else { "TURNED" }, 1.2, Some(color));
    }

    /// Gabriel: thunder from a clear sky; every foe within reach is struck, a boss wounded.
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
            self.smite_foe(i, Archangel::Gabriel);
        }
        if let Some(bp) = self.boss_target() {
            if dist_sq(bp, hp).sqrt() - BODY_RADIUS <= THUNDER_RADIUS {
                self.view.thunder(&self.level.grid, bp, color);
                self.smite_boss(Archangel::Gabriel.boss_damage(), bp);
            }
        }
        self.floater(hp.x, hp.y - 1.0, "GABRIEL'S THUNDER", 1.4, Some(color));
    }

    /// Raphael: a beam along his aim, as far as solid rock lets it; all in its path is struck.
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
            .filter(|&i| {
                self.enemies[i].is_targetable() && from_segment(hp, end, self.enemies[i].pos()) <= BEAM_HALF_WIDTH + FOE_RADIUS * 0.5
            })
            .collect();
        for i in doomed {
            self.smite_foe(i, Archangel::Raphael);
        }
        if let Some(bp) = self.boss_target() {
            if from_segment(hp, end, bp) <= BEAM_HALF_WIDTH + BODY_RADIUS {
                self.smite_boss(Archangel::Raphael.boss_damage(), bp);
            }
        }
    }

    /// Uriel: a great wave rolls out from him (see `update_angels`).
    fn uriels_wave(&mut self) {
        let hp = self.hero.pos();
        self.holy_wave = Some(HolyWave { origin: hp, radius: 0.0, struck: Vec::new(), boss_struck: false });
        let color = Archangel::Uriel.color();
        self.view.holy_wave(&self.level.grid, hp, color);
        self.view.shake(0.8);
        self.audio.play(Sfx::UrielWave);
        self.floater(hp.x, hp.y - 1.0, "URIEL'S WAVE", 1.6, Some(color));
    }

    /// Each tick of play: the archangels come back, Uriel's wave rolls on, and the turned foes hunt
    /// until their time is up.
    pub(super) fn update_angels(&mut self, dt: f64) {
        let patron = self.patron();
        for angel in self.angels.tick(dt) {
            if !self.angels.has(angel, patron) {
                continue;
            }
            // His power is back: a soft chime, and the emblem flares on the HUD.
            self.juice.angel_back[angel.index()] = 1.0;
            self.audio.play(Sfx::Nodule);
        }
        if let Some(mut w) = self.holy_wave.take() {
            w.radius += WAVE_SPEED * dt;
            let doomed: Vec<usize> = (0..self.enemies.len())
                .filter(|&i| {
                    !w.struck.contains(&i)
                        && self.enemies[i].is_targetable()
                        && dist_sq(self.enemies[i].pos(), w.origin) <= w.radius * w.radius
                })
                .collect();
            for i in doomed {
                w.struck.push(i);
                let p = self.enemies[i].pos();
                self.view.sparkle(&self.level.grid, p.x, p.y, 0.8, Archangel::Uriel.color(), 16);
                self.smite_foe(i, Archangel::Uriel);
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

    /// The archangels' two buttons in play (where they may be called: not in the gym's ring).
    pub(super) fn angel_buttons(&mut self, may_call: bool) {
        if self.input.was_pressed(&[Action::AngelNext]) {
            self.choose_next_angel();
        }
        if may_call && self.input.was_pressed(&[Action::Angel]) {
            self.invoke_angel();
        }
    }

    /// The archangels' shrines still to give in this level: (shrine, where it stands).
    pub fn shrines_to_give(&self) -> impl Iterator<Item = (usize, V)> + '_ {
        self.diamonds.iter().enumerate().filter(|(_, d)| d.kind == DiamondKind::Angel && d.active).map(|(i, d)| (i, d.tile.v()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_a_fight_by_frame_data_uriels_wave_is_one_blow_on_each_duelist_and_a_prince_takes_half() {
        use crate::game::lair::testing::{krampus, lair};
        let mut g = lair();
        g.angels.granted = true;
        g.angels.chosen = Archangel::Uriel;
        let hp = g.hero.pos();
        let d = krampus(&mut g);
        d.pos = V::new(hp.x, hp.y - 4.0);
        let before = d.hp;
        g.invoke_angel();
        for _ in 0..(1.5 / crate::app::STEP) as usize {
            g.update_angels(crate::app::STEP);
        }
        let lost = before - krampus(&mut g).hp;
        let (damage, _) = Archangel::Uriel.brawl_blow();
        assert!(lost > 0.0 && lost <= damage * PRINCE_SHARE + 1e-6, "one blow at half: {lost}");
    }

    #[test]
    fn a_beam_catches_what_lies_along_it() {
        let (a, b) = (V::new(0.0, 0.0), V::new(10.0, 0.0));
        assert!(from_segment(a, b, V::new(5.0, 0.5)) < 0.6);
        assert!(from_segment(a, b, V::new(12.0, 0.0)) > 1.9, "not past its end");
        assert!(from_segment(a, b, V::new(-1.0, 0.0)) > 0.9, "not behind him");
    }
}
