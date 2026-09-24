use std::collections::HashSet;

use super::grid::Grid;
use super::items::{weapon, RangedKind, WeaponDef};
use super::types::{dist_sq, js_round, Dir, P, V};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShotState {
    Flying,
    Wall,
    Spent,
    Caught,
}

const CATCH_RADIUS: f64 = 0.5;

/**
 * A ranged shot. Flies straight along a cardinal direction, passes through
 * tunnels and over the crypt floor, and breaks on rock or at max range.
 * Piercing shots keep going after a hit; returning shots fly back to be caught.
 */
#[derive(Clone, Debug)]
pub struct Projectile {
    pub pos: V,
    pub dir: Dir,
    pub kind: RangedKind,
    pub def: WeaponDef,
    pub traveled: f64,
    pub alive: bool,
    pub returning: bool,
    /// Enemies already struck (by index), so piercing shots hit each one once.
    pub struck: HashSet<usize>,
    /// Render-only spin of the model, in radians.
    pub spin: f32,
}

impl Projectile {
    pub fn new(start: V, dir: Dir, kind: RangedKind) -> Self {
        Self {
            pos: start,
            dir: dir.or_down(),
            kind,
            def: weapon(kind),
            traveled: 0.0,
            alive: true,
            returning: false,
            struck: HashSet::new(),
            spin: 0.0,
        }
    }

    /// Tile the shot is currently over.
    pub fn tile(&self) -> P {
        P::new(js_round(self.pos.x) as i32, js_round(self.pos.y) as i32)
    }

    pub fn update(&mut self, dt: f64, grid: &Grid, hero_pos: V) -> ShotState {
        if !self.alive {
            return ShotState::Spent;
        }
        let mut remaining = self.def.speed * dt;
        // Sub-step so a fast shot cannot skip over a wall block.
        while remaining > 0.0 {
            let s = remaining.min(0.25);
            remaining -= s;
            let v = self.dir.vecf();
            // Tile centers are integers, so wrap x within [-0.5, width - 0.5).
            let w = grid.width as f64;
            let raw_x = self.pos.x + v.x * s + 0.5;
            self.pos = V::new(raw_x.rem_euclid(w) - 0.5, self.pos.y + v.y * s);
            self.traveled += s;

            let t = self.tile();
            if !grid.is_walkable(t.x, t.y, true) {
                if self.def.returns && !self.returning {
                    self.turn_back();
                    continue;
                }
                self.alive = false;
                return ShotState::Wall;
            }
            if self.returning && dist_sq(self.pos, hero_pos) <= CATCH_RADIUS * CATCH_RADIUS {
                self.alive = false;
                return ShotState::Caught;
            }
            if self.traveled >= self.def.range {
                if self.def.returns && !self.returning {
                    self.turn_back();
                    continue;
                }
                self.alive = false;
                return ShotState::Spent;
            }
        }
        ShotState::Flying
    }

    /// Boomerang reversal: fly back the way it came for a little more than it went out.
    fn turn_back(&mut self) {
        self.returning = true;
        self.dir = self.dir.opposite();
        let back = self.traveled + 2.0;
        self.traveled = self.def.range - back;
        self.struck.clear();
    }
}

/// Spawns the projectiles a weapon fires from a position and facing.
pub fn spawn_shots(kind: RangedKind, from: V, facing: Dir) -> Vec<Projectile> {
    let def = weapon(kind);
    let dir = facing.or_down();
    let f = dir.vecf();
    let right = V::new(-f.y, f.x);
    let mut shots = Vec::new();
    for &off in def.lateral {
        shots.push(Projectile::new(V::new(from.x + right.x * off, from.y + right.y * off), dir, kind));
    }
    if def.spread {
        let sides = if dir == Dir::Up || dir == Dir::Down { [Dir::Left, Dir::Right] } else { [Dir::Up, Dir::Down] };
        for side in sides {
            shots.push(Projectile::new(from, side, kind));
        }
    }
    shots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trishot_fires_three_ways() {
        let shots = spawn_shots(RangedKind::Trishot, V::new(3.0, 3.0), Dir::Up);
        let dirs: Vec<Dir> = shots.iter().map(|s| s.dir).collect();
        assert_eq!(dirs, vec![Dir::Up, Dir::Left, Dir::Right]);
    }

    #[test]
    fn double_arrow_is_side_by_side() {
        let shots = spawn_shots(RangedKind::Double, V::new(3.0, 3.0), Dir::Right);
        assert_eq!(shots.len(), 2);
        assert!((shots[0].pos.y - shots[1].pos.y).abs() > 0.5);
    }
}
