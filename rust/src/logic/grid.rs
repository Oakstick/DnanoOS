use super::types::{Dir, Tile, CARDINALS, P};

/**
 * Tile grid with elevation. Layout never changes except by bombs; nodules
 * and charges are consumed in place. Every tile has an integer floor height
 * (0 = ground). Walking may step down any drop but climb only via a ramp;
 * jumping climbs one level. Flyers ignore height and pits.
 */
#[derive(Clone, Debug)]
pub struct Grid {
    pub width: i32,
    pub height: i32,
    tiles: Vec<Tile>,
    heights: Vec<i32>,
    ramps: Vec<bool>,
}

impl Grid {
    pub fn new(width: i32, height: i32, tiles: Vec<Tile>, heights: Vec<i32>, ramps: Vec<bool>) -> Self {
        assert_eq!(tiles.len(), (width * height) as usize, "grid size mismatch");
        Self { width, height, tiles, heights, ramps }
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.width && y < self.height
    }

    fn idx(&self, x: i32, y: i32) -> usize {
        (y * self.width + x) as usize
    }

    /// Wraps horizontally so tunnels connect across the edges.
    pub fn wrap(&self, p: P) -> P {
        P::new(p.x.rem_euclid(self.width), p.y)
    }

    pub fn get(&self, x: i32, y: i32) -> Tile {
        if !self.in_bounds(x, y) {
            return Tile::Rock;
        }
        self.tiles[self.idx(x, y)]
    }

    pub fn set(&mut self, x: i32, y: i32, t: Tile) {
        if self.in_bounds(x, y) {
            let i = self.idx(x, y);
            self.tiles[i] = t;
        }
    }

    /// Floor elevation in levels; rock reports the height of its tallest walkable neighbor's floor.
    pub fn elevation(&self, x: i32, y: i32) -> i32 {
        if !self.in_bounds(x, y) {
            return 0;
        }
        self.heights[self.idx(x, y)]
    }

    pub fn set_elevation(&mut self, x: i32, y: i32, h: i32) {
        if self.in_bounds(x, y) {
            let i = self.idx(x, y);
            self.heights[i] = h;
        }
    }

    pub fn is_ramp(&self, x: i32, y: i32) -> bool {
        self.in_bounds(x, y) && self.ramps[self.idx(x, y)]
    }

    /// Standable floor: not rock, not a pit, and the nest only when allowed.
    pub fn is_walkable(&self, x: i32, y: i32, allow_nest: bool) -> bool {
        match self.get(x, y) {
            Tile::Rock | Tile::Pit => false,
            Tile::Nest => allow_nest,
            _ => true,
        }
    }

    /// Air: anything but rock. Flyers cross pits and hover over the nest.
    pub fn is_flyable(&self, x: i32, y: i32, allow_nest: bool) -> bool {
        match self.get(x, y) {
            Tile::Rock => false,
            Tile::Nest => allow_nest,
            _ => true,
        }
    }

    pub fn neighbor(&self, p: P, dir: Dir) -> P {
        self.wrap(p.step(dir))
    }

    /**
     * Can a mover standing on `p` step to the next tile in `dir`? On foot that
     * means solid floor and no climb, unless a ramp joins the two tiles. In the
     * air only rock stops you.
     */
    pub fn can_move(&self, p: P, dir: Dir, allow_nest: bool, flying: bool) -> bool {
        if dir == Dir::None {
            return false;
        }
        let n = self.neighbor(p, dir);
        if flying {
            return self.is_flyable(n.x, n.y, allow_nest);
        }
        if !self.is_walkable(n.x, n.y, allow_nest) {
            return false;
        }
        let climb = self.elevation(n.x, n.y) - self.elevation(p.x, p.y);
        if climb <= 0 {
            return true;
        }
        climb == 1 && (self.is_ramp(p.x, p.y) || self.is_ramp(n.x, n.y))
    }

    pub fn open_directions(&self, p: P, allow_nest: bool, flying: bool) -> Vec<Dir> {
        CARDINALS.iter().copied().filter(|&d| self.can_move(p, d, allow_nest, flying)).collect()
    }

    /**
     * Where a jump from `p` in `dir` lands: two tiles ahead if that floor exists
     * and is at most one level up and the tile in between is not rock; else one
     * tile ahead under the same rules; else None (a hop in place).
     */
    pub fn jump_landing(&self, p: P, dir: Dir) -> Option<P> {
        if dir == Dir::None {
            return None;
        }
        let from = self.elevation(p.x, p.y);
        let one = self.neighbor(p, dir);
        let two = self.neighbor(one, dir);
        let landable = |t: P| self.is_walkable(t.x, t.y, false) && self.elevation(t.x, t.y) - from <= 1;
        if self.get(one.x, one.y) != Tile::Rock && landable(two) {
            return Some(two);
        }
        if landable(one) {
            return Some(one);
        }
        None
    }

    /// Removes a collectible and returns what was there.
    pub fn consume(&mut self, x: i32, y: i32) -> Option<Tile> {
        let t = self.get(x, y);
        if t == Tile::Nodule || t == Tile::Charge {
            self.set(x, y, Tile::Open);
            return Some(t);
        }
        None
    }

    pub fn count(&self, tile: Tile) -> usize {
        self.tiles.iter().filter(|&&t| t == tile).count()
    }

    /// Every tile in row order.
    pub fn cells(&self) -> impl Iterator<Item = (i32, i32, Tile)> + '_ {
        let w = self.width;
        self.tiles.iter().enumerate().map(move |(i, &t)| (i as i32 % w, i as i32 / w, t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny() -> Grid {
        // # . . #   heights 0 0 1 0 with a ramp on x=1
        let tiles = vec![Tile::Rock, Tile::Open, Tile::Open, Tile::Rock];
        Grid::new(4, 1, tiles, vec![0, 0, 1, 0], vec![false, true, false, false])
    }

    #[test]
    fn ramps_allow_one_level_climbs() {
        let g = tiny();
        assert!(g.can_move(P::new(1, 0), Dir::Right, false, false));
        assert!(g.can_move(P::new(2, 0), Dir::Left, false, false));
        assert!(!g.can_move(P::new(2, 0), Dir::Right, false, false));
    }

    #[test]
    fn wraps_horizontally() {
        let g = tiny();
        assert_eq!(g.wrap(P::new(-1, 0)), P::new(3, 0));
        assert_eq!(g.wrap(P::new(4, 0)), P::new(0, 0));
    }
}
