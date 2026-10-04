//! Wrath's court: **The Anvil of Wrath**, where Amon waits. Not yet built: until `BUILT` is
//! true the court is the stand-in (the Cyclops' meadow under Wrath's sky, `super::stand_in`) and
//! Amon is a Cyclops in its colour. (Its art will be `render::view::court_wrath`; its fight,
//! `game::court::wrath`.)

use super::{stand_in, STAND_IN_PRINCE};
use crate::logic::level::LevelData;
use crate::logic::sins::Sin;

/// Is the court built to its own plan, Amon fought as himself?
pub const BUILT: bool = false;
/// The court's name.
pub const NAME: &str = "The Anvil of Wrath";
/// What Amon raises against the hero.
pub const ADDS: &str = "ravens";
/// Where Amon stands as the fight begins.
pub const PRINCE: (i32, i32) = STAND_IN_PRINCE;

/// The court as a level, as hard as dungeon `tier` (for now the stand-in).
pub fn level(tier: usize) -> LevelData {
    stand_in(Sin::Wrath, tier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraths_court_parses_and_every_tile_of_it_can_be_reached() {
        super::super::tests::court_is_sound(Sin::Wrath);
    }
}
