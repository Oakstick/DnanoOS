//! The heroes you can play: Ragnar the viking, Mehmed the janissary and
//! Miyamoto the samurai. For now only Ragnar can be chosen; the other two
//! stand greyed out on the character screen until their abilities are set.

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

    /// Can he be chosen yet?
    pub fn unlocked(self) -> bool {
        self == HeroKind::Ragnar
    }

    /// His place in the line-up.
    pub fn slot(self) -> usize {
        ROSTER.iter().position(|&h| h == self).unwrap_or(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_ragnar_is_unlocked_and_he_stands_in_the_middle() {
        assert_eq!(ROSTER.iter().filter(|h| h.unlocked()).count(), 1);
        assert!(HeroKind::Ragnar.unlocked());
        assert_eq!(HeroKind::Ragnar.slot(), 1);
        assert_eq!(HeroKind::default(), HeroKind::Ragnar);
    }
}
