//! The heroes you can play: Ragnar the viking, Mehmed the janissary and
//! Miyamoto the samurai, each with his own armory and path (see `progression`).

/// A playable hero.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Default)]
pub enum HeroKind {
    #[default]
    Ragnar,
    Mehmed,
    Miyamoto,
}

/// The line-up on the character screen, left to right.
pub const ROSTER: [HeroKind; 3] = [HeroKind::Mehmed, HeroKind::Ragnar, HeroKind::Miyamoto];

impl HeroKind {
    pub fn name(self) -> &'static str {
        match self {
            HeroKind::Ragnar => "Ragnar",
            HeroKind::Mehmed => "Mehmed",
            HeroKind::Miyamoto => "Miyamoto",
        }
    }

    /// What he is, shown under his name.
    pub fn title(self) -> &'static str {
        match self {
            HeroKind::Ragnar => "The Viking",
            HeroKind::Mehmed => "The Janissary",
            HeroKind::Miyamoto => "The Samurai",
        }
    }

    /// Can he be chosen? All three can.
    pub fn unlocked(self) -> bool {
        true
    }

    /// His place in the line-up.
    pub fn slot(self) -> usize {
        ROSTER.iter().position(|&h| h == self).unwrap_or(1)
    }

    /// How fast he runs, Ragnar's pace being 1: the samurai is quicker, the janissary slower.
    pub fn speed(self) -> f64 {
        match self {
            HeroKind::Ragnar => 1.0,
            HeroKind::Mehmed => 0.9,
            HeroKind::Miyamoto => 1.1,
        }
    }

    /// How much he can take, Ragnar's health being 1: the janissary more, the samurai less.
    pub fn vigor(self) -> f64 {
        match self {
            HeroKind::Ragnar => 1.0,
            HeroKind::Mehmed => 1.2,
            HeroKind::Miyamoto => 0.8,
        }
    }

    /// How hard his blows land: the janissary's hit harder.
    pub fn might(self) -> f64 {
        match self {
            HeroKind::Mehmed => MEHMED_MIGHT,
            _ => 1.0,
        }
    }

    /// How long he waits between blows (his cooldowns are scaled by it): the samurai strikes quicker.
    pub fn tempo(self) -> f64 {
        match self {
            HeroKind::Miyamoto => MIYAMOTO_TEMPO,
            _ => 1.0,
        }
    }
}

/// Mehmed's blows land this much harder (rounded up, so always at least one more).
pub const MEHMED_MIGHT: f64 = 1.25;
/// Miyamoto's cooldowns are this much of the others': he strikes a quarter more often.
pub const MIYAMOTO_TEMPO: f64 = 0.8;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_three_can_be_played_and_ragnar_stands_in_the_middle() {
        assert!(ROSTER.iter().all(|h| h.unlocked()));
        assert_eq!(HeroKind::Ragnar.slot(), 1);
        assert_eq!(HeroKind::default(), HeroKind::Ragnar);
    }

    #[test]
    fn each_hero_has_his_own_nature_measured_against_ragnar() {
        use HeroKind::*;
        assert_eq!((Ragnar.speed(), Miyamoto.speed(), Mehmed.speed()), (1.0, 1.1, 0.9));
        assert_eq!((Ragnar.vigor(), Mehmed.vigor(), Miyamoto.vigor()), (1.0, 1.2, 0.8));
        assert!(Mehmed.might() > 1.0 && Ragnar.might() == 1.0 && Miyamoto.might() == 1.0, "the janissary hits hardest");
        assert!(Miyamoto.tempo() < 1.0 && Ragnar.tempo() == 1.0 && Mehmed.tempo() == 1.0, "the samurai strikes quickest");
    }
}
