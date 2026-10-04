//! Lust's court: **The Rose Bower**, where Asmodeus waits. Not yet built: until `BUILT` is
//! true the court is the stand-in (the Cyclops' meadow under Lust's sky, `super::stand_in`) and
//! Asmodeus is a Cyclops in its colour. (Its art will be `render::view::court_lust`; its fight,
//! `game::court::lust`.)

use super::{stand_in, STAND_IN_PRINCE};
use crate::logic::level::LevelData;
use crate::logic::sins::Sin;

/// Is the court built to its own plan, Asmodeus fought as himself?
pub const BUILT: bool = false;
/// The court's name.
pub const NAME: &str = "The Rose Bower";
/// What Asmodeus raises against the hero.
pub const ADDS: &str = "elf-maids";
/// Where Asmodeus stands as the fight begins.
pub const PRINCE: (i32, i32) = STAND_IN_PRINCE;

/// The court as a level, as hard as dungeon `tier` (for now the stand-in).
pub fn level(tier: usize) -> LevelData {
    stand_in(Sin::Lust, tier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lusts_court_parses_and_every_tile_of_it_can_be_reached() {
        super::super::tests::court_is_sound(Sin::Lust);
    }
}
