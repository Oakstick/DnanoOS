use super::grid::Grid;
use super::types::{Dir, Tile, P, V};

/**
 * Grid-locked mover. `tile` is the integer tile we are leaving/standing on and
 * `offset` (0..1) is how far we have traveled toward the next tile along `dir`.
 * Decisions happen only when we arrive at a tile center (offset wraps past 1).
 */
#[derive(Clone, Debug)]
pub struct Mover {
    pub tile: P,
    pub offset: f64,
    pub dir: Dir,
    /// Tiles per second.
    pub speed: f64,
    /// Flyers ignore elevation and pits.
    pub flying: bool,
    /// Width of the world last stepped through, so `occupied` can wrap in a tunnel.
    pub world_width: i32,
}

impl Mover {
    pub fn new(start: P, speed: f64) -> Self {
        Self { tile: start, offset: 0.0, dir: Dir::None, speed, flying: false, world_width: 0 }
    }

    /// Continuous position in tile units (tile centers are integers).
    pub fn pos(&self) -> V {
        let v = self.dir.vecf();
        V::new(self.tile.x as f64 + v.x * self.offset, self.tile.y as f64 + v.y * self.offset)
    }

    /// Floor elevation under the mover, blended between the tiles it is crossing.
    pub fn elevation_on(&mut self, grid: &Grid) -> f64 {
        self.world_width = grid.width;
        let a = grid.elevation(self.tile.x, self.tile.y) as f64;
        if self.dir == Dir::None || self.offset == 0.0 {
            return a;
        }
        let n = grid.wrap(self.next_tile(self.tile));
        let b = if grid.get(n.x, n.y) == Tile::Rock { a } else { grid.elevation(n.x, n.y) as f64 };
        a + (b - a) * self.offset
    }

    /**
     * The tile this mover counts as standing on. Half a tile past the centre it
     * becomes the tile ahead, wrapped, so a mover crossing a tunnel edge reports
     * the tile it is really entering rather than one off the end of the map.
     */
    pub fn occupied(&self) -> P {
        let p = if self.offset >= 0.5 { self.next_tile(self.tile) } else { self.tile };
        if self.world_width <= 0 {
            return p;
        }
        P::new(p.x.rem_euclid(self.world_width), p.y)
    }

    pub fn next_tile(&self, from: P) -> P {
        from.step(self.dir)
    }

    pub fn teleport(&mut self, to: P) {
        self.tile = to;
        self.offset = 0.0;
        self.dir = Dir::None;
    }

    /// Immediately flip direction, preserving position.
    pub fn reverse(&mut self, grid: &Grid) {
        self.world_width = grid.width;
        if self.dir == Dir::None || self.offset == 0.0 {
            self.dir = self.dir.opposite();
            return;
        }
        self.tile = grid.wrap(self.next_tile(self.tile));
        self.offset = 1.0 - self.offset;
        self.dir = self.dir.opposite();
    }

    /**
     * Advance along the grid. `decide` is asked for a direction at every tile
     * center (and once when idle). Returns true if the mover crossed a center.
     */
    pub fn step(
        &mut self,
        dt: f64,
        grid: &Grid,
        mut decide: impl FnMut(P, Dir) -> Dir,
        allow_nest: bool,
        allow_mid_reverse: bool,
        stop_when_idle: bool,
    ) -> bool {
        self.world_width = grid.width;
        let mut crossed = false;

        if self.dir == Dir::None {
            let want = decide(self.tile, self.dir);
            if grid.can_move(self.tile, want, allow_nest, self.flying) {
                self.dir = want;
            } else {
                return false;
            }
        } else if allow_mid_reverse {
            let want = decide(self.tile, self.dir);
            if want == self.dir.opposite() {
                self.reverse(grid);
            }
        }

        let mut remaining = self.speed * dt;
        while remaining > 0.0 {
            let to_center = 1.0 - self.offset;
            if remaining < to_center {
                self.offset += remaining;
                break;
            }
            // Arrive at the next tile center.
            remaining -= to_center;
            self.tile = grid.wrap(self.next_tile(self.tile));
            self.offset = 0.0;
            crossed = true;

            let want = decide(self.tile, self.dir);
            if want != Dir::None && grid.can_move(self.tile, want, allow_nest, self.flying) {
                self.dir = want;
            } else if (stop_when_idle && want == Dir::None)
                || !grid.can_move(self.tile, self.dir, allow_nest, self.flying)
            {
                // Either nothing is asked for (hold-to-move release) or the way is blocked: rest on this center.
                self.dir = Dir::None;
                break;
            }
        }
        crossed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::grid::Grid;

    pub fn corridor(len: i32) -> Grid {
        let mut tiles = vec![Tile::Rock; (len * 3) as usize];
        for x in 1..len - 1 {
            tiles[(len + x) as usize] = Tile::Open;
        }
        Grid::new(len, 3, tiles, vec![0; (len * 3) as usize], vec![false; (len * 3) as usize])
    }

    #[test]
    fn stops_at_walls() {
        let g = corridor(6);
        let mut m = Mover::new(P::new(1, 1), 10.0);
        for _ in 0..100 {
            m.step(0.05, &g, |_, _| Dir::Right, false, false, false);
        }
        assert_eq!(m.tile, P::new(4, 1));
        assert_eq!(m.dir, Dir::None);
    }
}
