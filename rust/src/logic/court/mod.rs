//! The princes' courts, built to their own plans as their fights come (ROADMAP.md, phases 3 to
//! 9). Each sin's court is a module of its own: `greed` (the Counting House, where Mammon sits,
//! the first built), `pride` (the Throne of the Morning Star), `envy` (the Drowned Bell), `wrath`
//! (the Anvil of Wrath), `sloth` (the Hall of Stopped Clocks), `gluttony` (the Banquet of Flies)
//! and `lust` (the Rose Bower).
//!
//! Every court module says the same few things: whether it is `BUILT` (until it is, its prince is
//! stood in by the Cyclops in the meadow, as before), its `NAME`, its `level` for a tier, where its
//! `PRINCE` stands as the fight begins, and what his `ADDS` are called. What is shared lives here:
//! the weight a court's tier puts on its prince, and the choice of court by sin.

pub mod envy;
pub mod gluttony;
pub mod greed;
pub mod lust;
pub mod pride;
pub mod sloth;
pub mod wrath;

pub use greed::{
    beside_heap, counting_house, in_hall, piece, Piece, CENTRE, CHESTS, CORNER, DAIS, GATE, HALF, HEAPS, HERO, MAMMON, MOSAIC, PILLARS,
    SCALES, SIZE, TABLES,
};

use super::level::{LevelData, Theme};
use super::levels::{boss_arena, ARENA_BOSS};
use super::sins::Sin;

/// What the prince (and what he raises) is multiplied by in a court of `tier`: his health and his
/// damage. The first court the hero comes to is the measure; every key held before it makes the
/// prince stronger, as it does the realms, and a little more with each (WARBAND.md W9: on a straight
/// line the kit a hero buys on the way outgrew the courts, the seventh prince falling sooner than
/// the first).
pub fn weight(tier: usize) -> (f64, f64) {
    let t = tier.max(1) as f64 - 1.0;
    (COURT_HEALTH * (1.0 + 0.3 * t + COURT_CURVE * t * (t - 1.0)), 0.85 + 0.1 * t)
}

/// The princes' health above their breed's (WARBAND.md W9: with the blade no longer rusting through
/// in a fight, the expert's first two courts fell in 78 to 103 seconds, under PRINCES.md's 90).
pub const COURT_HEALTH: f64 = 1.15;
/// The courts' health, curved upward by this much per key held squared (the second court as before).
pub const COURT_CURVE: f64 = 0.02;

/// Is the court of `sin` built to its own plan, its prince fought as himself? (If not, he is the
/// Cyclops in the meadow under the sin's sky.)
pub fn built(sin: Sin) -> bool {
    match sin {
        Sin::Greed => greed::BUILT,
        Sin::Pride => pride::BUILT,
        Sin::Envy => envy::BUILT,
        Sin::Wrath => wrath::BUILT,
        Sin::Sloth => sloth::BUILT,
        Sin::Gluttony => gluttony::BUILT,
        Sin::Lust => lust::BUILT,
    }
}

/// The court of `sin` as a level, as hard as dungeon `tier`.
pub fn court_level(sin: Sin, tier: usize) -> LevelData {
    match sin {
        Sin::Greed => greed::level(tier),
        Sin::Pride => pride::level(tier),
        Sin::Envy => envy::level(tier),
        Sin::Wrath => wrath::level(tier),
        Sin::Sloth => sloth::level(tier),
        Sin::Gluttony => gluttony::level(tier),
        Sin::Lust => lust::level(tier),
    }
}

/// Where the prince of `sin` stands as his fight begins.
pub fn prince_spot(sin: Sin) -> (i32, i32) {
    match sin {
        Sin::Greed => greed::PRINCE,
        Sin::Pride => pride::PRINCE,
        Sin::Envy => envy::PRINCE,
        Sin::Wrath => wrath::PRINCE,
        Sin::Sloth => sloth::PRINCE,
        Sin::Gluttony => gluttony::PRINCE,
        Sin::Lust => lust::PRINCE,
    }
}

/// The court's own name for what its prince raises against the hero ("golems", "ravens"...).
pub fn adds(sin: Sin) -> &'static str {
    match sin {
        Sin::Greed => greed::ADDS,
        Sin::Pride => pride::ADDS,
        Sin::Envy => envy::ADDS,
        Sin::Wrath => wrath::ADDS,
        Sin::Sloth => sloth::ADDS,
        Sin::Gluttony => gluttony::ADDS,
        Sin::Lust => lust::ADDS,
    }
}

/// The stand-in for a court not yet built: the Cyclops' meadow under the sin's sky, named for its
/// prince (the prince himself a Cyclops in the sin's colour).
pub fn stand_in(sin: Sin, tier: usize) -> LevelData {
    let mut data = boss_arena(tier);
    data.name = format!("The Court of {}", sin.prince());
    data.theme = Theme::Court(sin);
    data
}

/// Where the stand-in's Cyclops waits (`stand_in`).
pub const STAND_IN_PRINCE: (i32, i32) = ARENA_BOSS;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::parse_level;
    use crate::logic::pathing::reachable;
    use crate::logic::sins::SINS;
    use crate::logic::types::{Tile, P};

    /// Every court's level parses: its prince where its module says he stands, every open tile
    /// reachable from the hero's spawn, the court's gate in its wall, and it is a boss's arena under
    /// its sin's theme.
    pub(crate) fn court_is_sound(sin: Sin) {
        for tier in [1, 4, 7] {
            let level = parse_level(&court_level(sin, tier));
            assert!(level.data.boss && level.data.theme == Theme::Court(sin), "{sin:?}");
            let at = prince_spot(sin);
            assert_eq!(level.enemy_spawn, P::new(at.0, at.1), "{sin:?}: the prince where his module says");
            assert!(level.gate.is_some(), "{sin:?}: a gate");
            let g = &level.grid;
            let seen = reachable(g, level.hero_spawn, false, false);
            for (x, y, t) in g.cells() {
                if t != Tile::Rock && t != Tile::Pit {
                    assert!(seen.contains(&P::new(x, y)), "{sin:?}: {x},{y} cut off");
                }
            }
            assert!(seen.contains(&level.enemy_spawn), "{sin:?}: the way to the prince is open");
            assert!(!adds(sin).is_empty());
        }
    }

    #[test]
    fn every_court_built_or_not_is_a_sound_level_and_greeds_is_built() {
        for sin in SINS {
            court_is_sound(sin);
        }
        assert!(built(Sin::Greed));
        assert_eq!(court_level(Sin::Greed, 1).name, greed::NAME);
    }
}
