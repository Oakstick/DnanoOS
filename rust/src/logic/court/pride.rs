//! Pride's court: **The Throne of the Morning Star**, where Lucifer waits. Not yet built: until `BUILT` is
//! true the court is the stand-in (the Cyclops' meadow under Pride's sky, `super::stand_in`) and
//! Lucifer is a Cyclops in its colour. (Its art will be `render::view::court_pride`; its fight,
//! `game::court::pride`.)

use super::{stand_in, STAND_IN_PRINCE};
use crate::logic::level::LevelData;
use crate::logic::sins::Sin;

/// Is the court built to its own plan, Lucifer fought as himself?
pub const BUILT: bool = false;
/// The court's name.
pub const NAME: &str = "The Throne of the Morning Star";
/// What Lucifer raises against the hero.
pub const ADDS: &str = "reflections";
/// Where Lucifer stands as the fight begins.
pub const PRINCE: (i32, i32) = STAND_IN_PRINCE;

/// The court as a level, as hard as dungeon `tier` (for now the stand-in).
pub fn level(tier: usize) -> LevelData {
    stand_in(Sin::Pride, tier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prides_court_parses_and_every_tile_of_it_can_be_reached() {
        super::super::tests::court_is_sound(Sin::Pride);
    }
}
