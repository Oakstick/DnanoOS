//! Pure rules: combat reach, explosions, the frenzy window, slime puddles and score.

use std::collections::{BTreeMap, HashSet};

use super::enemy::Enemy;
use super::grid::Grid;
use super::types::{dist_sq, Tile, P, V};
use crate::platform;

// ------------------------------------------------------------------- combat

/// How far in front of the hero the sword reaches, in tiles.
pub const ATTACK_REACH: f64 = 1.2;
/// Half-width of the swing to either side, in tiles.
pub const ATTACK_HALF_WIDTH: f64 = 0.7;
pub const POINTS_KILL: u32 = 100;
/// How far the trail of a swing sweeps to either side of the aim, in radians: as far as the slash drawn.
pub const SWEEP_HALF_ANGLE: f64 = 1.15;
/// A foe's body to a blade: the blow lands if any of it is in the arc.
pub const FOE_RADIUS: f64 = 0.4;

/// The radius of the arc a swing's trail sweeps (and is drawn at), for a blade reaching `reach`.
pub fn sweep_radius(reach: f64) -> f64 {
    1.08 + (reach - 1.2) * 0.54
}

/// `aim` below is the unit vector a blow goes along (`Hero::aim`): any way in free movement.
fn in_swing(hero: V, aim: V, p: V) -> bool {
    let f = aim;
    let dx = p.x - hero.x;
    let dy = p.y - hero.y;
    let along = dx * f.x + dy * f.y; // distance in front of the hero
    let across = (dx * f.y - dy * f.x).abs(); // lateral offset
    (-0.25..=ATTACK_REACH).contains(&along) && across <= ATTACK_HALF_WIDTH
}

/// Does a swing reach a big body of `radius` tiles centered at `p`?
pub fn in_swing_reach(hero: V, aim: V, p: V, radius: f64) -> bool {
    let f = aim;
    let dx = p.x - hero.x;
    let dy = p.y - hero.y;
    let along = dx * f.x + dy * f.y;
    let across = (dx * f.y - dy * f.x).abs();
    along >= -0.25 - radius * 0.5 && along <= ATTACK_REACH + radius && across <= ATTACK_HALF_WIDTH + radius * 0.8
}

/// Objects on a tile the sword can reach: used to strike shrines.
pub fn tile_in_reach(hero: V, aim: V, tile: P) -> bool {
    in_swing(hero, aim, tile.v())
}

/**
 * Is `p` (of `radius`) caught by a swing: inside its reach (`reach` ahead,
 * `half_width` to each side, `behind` back), or anywhere in the arc its trail
 * sweeps round him (`sweep_radius`, `SWEEP_HALF_ANGLE` either side of the aim)?
 */
pub fn in_blade_arc(hero: V, aim: V, p: V, reach: f64, half_width: f64, behind: f64, radius: f64) -> bool {
    let f = aim;
    let dx = p.x - hero.x;
    let dy = p.y - hero.y;
    let along = dx * f.x + dy * f.y;
    let across = (dx * f.y - dy * f.x).abs();
    if along >= -behind - radius * 0.5 && along <= reach + radius && across <= half_width + radius * 0.8 {
        return true;
    }
    let dist = (dx * dx + dy * dy).sqrt();
    if dist > sweep_radius(reach) + radius {
        return false;
    }
    if dist <= radius {
        return true;
    }
    // Off the aim by no more than the sweep, give or take the body's own width.
    let off = (along / dist).clamp(-1.0, 1.0).acos();
    off <= SWEEP_HALF_ANGLE + (radius / dist).min(1.0).asin()
}

/// Indices of targetable enemies caught by such a swing.
pub fn find_blade_targets(hero: V, aim: V, enemies: &[Enemy], reach: f64, half_width: f64, behind: f64) -> Vec<usize> {
    enemies
        .iter()
        .enumerate()
        .filter(|(_, e)| e.is_targetable() && in_blade_arc(hero, aim, e.pos(), reach, half_width, behind, FOE_RADIUS))
        .map(|(i, _)| i)
        .collect()
}

/// Indices of targetable enemies caught by a sword swing.
pub fn find_attack_targets(hero: V, aim: V, enemies: &[Enemy]) -> Vec<usize> {
    enemies
        .iter()
        .enumerate()
        .filter(|(_, e)| e.is_targetable() && in_swing(hero, aim, e.pos()))
        .map(|(i, _)| i)
        .collect()
}

/// Targetable enemies within `radius` of a point, nearest first, skipping any in `exclude`.
pub fn find_projectile_hits(point: V, radius: f64, enemies: &[Enemy], exclude: &HashSet<usize>) -> Vec<usize> {
    let r2 = radius * radius;
    let mut hits: Vec<(usize, f64)> = enemies
        .iter()
        .enumerate()
        .filter(|(i, e)| e.is_targetable() && !exclude.contains(i))
        .map(|(i, e)| (i, dist_sq(e.pos(), point)))
        .filter(|&(_, d)| d <= r2)
        .collect();
    hits.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    hits.into_iter().map(|(i, _)| i).collect()
}

// ---------------------------------------------------------------- explosion

/// Entities within this distance of the blast center are hit.
pub const BLAST_RADIUS: f64 = 3.0;
/// Rock within this distance is destroyed.
pub const BREAK_RADIUS: f64 = 3.2;
/// Chain-ignition delay range for bombs caught in a blast, seconds.
pub const CHAIN_DELAY_MIN: f64 = 0.15;
pub const CHAIN_DELAY_MAX: f64 = 0.5;

/// Damage to the hero by distance from the blast center.
pub fn blast_damage(distance: f64) -> i32 {
    if distance <= BLAST_RADIUS / 2.0 {
        50
    } else if distance <= BLAST_RADIUS {
        25
    } else {
        0
    }
}

pub fn in_blast(center: P, p: V) -> bool {
    dist_sq(center.v(), p) <= BLAST_RADIUS * BLAST_RADIUS
}

/// Rock that must survive: the outer border and any rock touching the enemy crypt.
pub fn is_protected(grid: &Grid, x: i32, y: i32) -> bool {
    if x <= 0 || y <= 0 || x >= grid.width - 1 || y >= grid.height - 1 {
        return true;
    }
    for dy in -1..=1 {
        for dx in -1..=1 {
            if grid.get(x + dx, y + dy) == Tile::Nest {
                return true;
            }
        }
    }
    false
}

/// Turns unprotected rock near the center into open floor. Returns the tiles changed.
pub fn break_rocks(grid: &mut Grid, center: P, radius: f64) -> Vec<P> {
    let mut changed = Vec::new();
    let r = radius.ceil() as i32;
    for dy in -r..=r {
        for dx in -r..=r {
            if (dx * dx + dy * dy) as f64 > radius * radius {
                continue;
            }
            let x = center.x + dx;
            let y = center.y + dy;
            if grid.get(x, y) != Tile::Rock || is_protected(grid, x, y) || grid.is_landmark(x, y) {
                continue;
            }
            grid.set(x, y, Tile::Open);
            changed.push(P::new(x, y));
        }
    }
    changed
}

// ------------------------------------------------------------------- frenzy

/// The state inversion: the stun window and the escalating value of enemies taken within it.
#[derive(Clone, Debug, Default)]
pub struct Frenzy {
    pub timer: f64,
    pub duration: f64,
    pub chain: u32,
}

impl Frenzy {
    pub fn active(&self) -> bool {
        self.timer > 0.0
    }
    /// Last two seconds: enemies flash as a warning.
    pub fn warning(&self) -> bool {
        self.active() && self.timer <= 2.0
    }
    /// 0..1 remaining fraction for the HUD bar.
    pub fn fraction(&self) -> f64 {
        if self.duration > 0.0 {
            (self.timer / self.duration).max(0.0)
        } else {
            0.0
        }
    }
    pub fn start(&mut self, seconds: f64) {
        self.timer = seconds;
        self.duration = seconds;
        self.chain = 0;
    }
    /// Returns true on the tick the window closes.
    pub fn update(&mut self, dt: f64) -> bool {
        if self.timer <= 0.0 {
            return false;
        }
        self.timer -= dt;
        if self.timer <= 0.0 {
            self.timer = 0.0;
            return true;
        }
        false
    }
    /// Score for the next enemy taken in this window: 200, 400, 800, 1600.
    pub fn consume_value(&mut self) -> u32 {
        let value = 200 * 2u32.pow(self.chain.min(20));
        self.chain += 1;
        value
    }
    pub fn cancel(&mut self) {
        self.timer = 0.0;
    }
}

// ------------------------------------------------------------------ puddles

pub const PUDDLE_SECONDS: f64 = 8.0;

/// Temporary slime marks left on tiles; each fades out on its own timer.
#[derive(Clone, Debug, Default)]
pub struct PuddleField {
    /// Ordered, so the goo is always drawn in the same order.
    map: BTreeMap<P, f64>,
}

impl PuddleField {
    /// Drops (or refreshes) a puddle on a tile.
    pub fn add(&mut self, tile: P, seconds: f64) {
        let life = self.map.entry(tile).or_insert(0.0);
        *life = life.max(seconds);
    }
    pub fn has(&self, tile: P) -> bool {
        self.map.contains_key(&tile)
    }
    pub fn update(&mut self, dt: f64) {
        self.map.retain(|_, life| {
            *life -= dt;
            *life > 0.0
        });
    }
    pub fn clear(&mut self) {
        self.map.clear();
    }
    pub fn len(&self) -> usize {
        self.map.len()
    }
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = (P, f64)> + '_ {
        self.map.iter().map(|(&p, &l)| (p, l))
    }
}

// -------------------------------------------------------------------- score

/// A golden key is worth four kills.
pub const POINTS_NODULE: u32 = 400;
pub const POINTS_CHARGE: u32 = 50;
/// Ragnar starts with three lives and can never hold more.
pub const MAX_LIVES: u32 = 3;
const HIGH_SCORE_KEY: &str = "vh.highscore";

#[derive(Clone, Debug)]
pub struct Score {
    pub score: u32,
    pub high: u32,
    pub lives: u32,
    pub level: usize,
    next_life_at: u32,
}

impl Default for Score {
    fn default() -> Self {
        Self::new()
    }
}

impl Score {
    pub fn new() -> Self {
        let high = platform::load(HIGH_SCORE_KEY).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
        Self { score: 0, high, lives: MAX_LIVES, level: 0, next_life_at: 10_000 }
    }

    /// Returns true if an extra life was awarded.
    pub fn add(&mut self, points: u32) -> bool {
        self.score += points;
        if self.score > self.high {
            self.high = self.score;
            platform::save(HIGH_SCORE_KEY, &self.high.to_string());
        }
        if self.score >= self.next_life_at {
            self.next_life_at += 10_000;
            return self.gain_life();
        }
        false
    }

    /// Returns true if a life was actually added (not already at the cap).
    pub fn gain_life(&mut self) -> bool {
        if self.lives >= MAX_LIVES {
            return false;
        }
        self.lives += 1;
        true
    }

    /// Returns true when that was the last life.
    pub fn lose_life(&mut self) -> bool {
        self.lives = self.lives.saturating_sub(1);
        self.lives == 0
    }

    /// Picks up a saved adventure: its score and lives, and the next extra life above that score.
    pub fn resume(&mut self, score: u32, lives: u32) {
        self.score = score;
        self.lives = lives.clamp(1, MAX_LIVES);
        self.next_life_at = (score / 10_000 + 1) * 10_000;
    }

    pub fn reset(&mut self) {
        self.score = 0;
        self.lives = MAX_LIVES;
        self.level = 0;
        self.next_life_at = 10_000;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_swing_catches_every_foe_its_trail_sweeps_through() {
        let hero = V::new(5.0, 5.0);
        let up = V::new(0.0, -1.0);
        let (reach, half, behind) = (1.2, 0.7, 0.25);
        let caught = |p: V| in_blade_arc(hero, up, p, reach, half, behind, FOE_RADIUS);
        // Side by side in a corridor two tiles wide, both ahead of him: one blow takes both.
        assert!(caught(V::new(5.0, 4.0)) && caught(V::new(6.0, 4.0)));
        // Beside him and a little ahead, where the slash begins.
        assert!(caught(V::new(6.0, 4.6)));
        // Not straight beside him, behind him, or out of reach ahead.
        assert!(!caught(V::new(6.2, 5.0)) && !caught(V::new(5.0, 6.2)) && !caught(V::new(5.0, 2.8)));
        // A spear's long thrust is no wider for it.
        assert!(!in_blade_arc(hero, up, V::new(6.8, 3.0), 2.3, 0.55, 0.2, FOE_RADIUS));
    }

    #[test]
    fn a_blow_reaches_whichever_way_it_is_aimed() {
        let hero = V::new(5.0, 5.0);
        // A foe off on the diagonal: a blow aimed at it lands; one straight to the side does not.
        let foe = V::new(5.8, 5.8);
        let diagonal = V::new(std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2);
        assert!(in_blade_arc(hero, diagonal, foe, 1.2, 0.7, 0.25, 0.0));
        assert!(!in_blade_arc(hero, super::super::types::Dir::Right.vecf(), foe, 1.2, 0.7, 0.25, 0.0));
    }

    #[test]
    fn frenzy_chain_doubles() {
        let mut f = Frenzy::default();
        f.start(5.0);
        assert_eq!(f.consume_value(), 200);
        assert_eq!(f.consume_value(), 400);
        assert_eq!(f.consume_value(), 800);
    }

    #[test]
    fn blast_falls_off() {
        assert_eq!(blast_damage(1.0), 50);
        assert_eq!(blast_damage(2.5), 25);
        assert_eq!(blast_damage(3.5), 0);
    }

    #[test]
    fn puddles_fade() {
        let mut p = PuddleField::default();
        p.add(P::new(2, 2), PUDDLE_SECONDS);
        p.update(PUDDLE_SECONDS - 0.1);
        assert!(p.has(P::new(2, 2)));
        p.update(0.2);
        assert!(!p.has(P::new(2, 2)));
    }
}
