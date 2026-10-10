//! The cover in the Cyclops' meadow (B57): the stone pillars and the low walls inside the arena.

use super::Game;
use crate::logic::levels::ARENA;
use crate::logic::types::Tile;

impl Game {
    /// Tiles of cover still standing in the Cyclops' meadow (0 anywhere else).
    pub fn meadow_cover_standing(&self) -> u32 {
        if self.boss.is_none() {
            return 0;
        }
        let (x0, y0, x1, y1) = ARENA;
        let g = &self.level.grid;
        (y0..=y1).flat_map(|y| (x0..=x1).map(move |x| (x, y))).filter(|&(x, y)| g.get(x, y) == Tile::Rock).count() as u32
    }
}
