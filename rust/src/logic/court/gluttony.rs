//! Gluttony's court: **The Banquet of Flies**, where Beelzebub waits. Not yet built: until `BUILT` is
//! true the court is the stand-in (the Cyclops' meadow under Gluttony's sky, `super::stand_in`) and
//! Beelzebub is a Cyclops in its colour. (Its art will be `render::view::court_gluttony`; its fight,
//! `game::court::gluttony`.)

use super::{stand_in, STAND_IN_PRINCE};
use crate::logic::level::LevelData;
use crate::logic::sins::Sin;

/// Is the court built to its own plan, Beelzebub fought as himself?
pub const BUILT: bool = false;
/// The court's name.
pub const NAME: &str = "The Banquet of Flies";
/// What Beelzebub raises against the hero.
pub const ADDS: &str = "swarms";
/// Where Beelzebub stands as the fight begins.
pub const PRINCE: (i32, i32) = STAND_IN_PRINCE;

/// The court as a level, as hard as dungeon `tier` (for now the stand-in).
pub fn level(tier: usize) -> LevelData {
    stand_in(Sin::Gluttony, tier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gluttonys_court_parses_and_every_tile_of_it_can_be_reached() {
        super::super::tests::court_is_sound(Sin::Gluttony);
    }
}
