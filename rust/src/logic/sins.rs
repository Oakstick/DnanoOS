//! The seven deadly sins. In the Portals of Hell each has a platform and a
//! portal gate, its name carved over the door and the door showing the sin;
//! behind it lies the sin's realm, where its prince keeps the key. They stand
//! clockwise from the entrance in the order of the terraces of Dante's
//! Purgatory.

use super::enemy::EnemyKind;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Sin {
    Pride,
    Envy,
    Wrath,
    Sloth,
    Greed,
    Gluttony,
    Lust,
}

/// The seven, clockwise round the hub from the entrance.
pub const SINS: [Sin; 7] = [Sin::Pride, Sin::Envy, Sin::Wrath, Sin::Sloth, Sin::Greed, Sin::Gluttony, Sin::Lust];

/// Every sin's key held: the seal breaks.
pub const ALL_SINS: u8 = 0x7f;

impl Sin {
    pub fn index(self) -> usize {
        SINS.iter().position(|&s| s == self).unwrap_or(0)
    }

    /// This sin's bit in the set of those cleared.
    pub fn bit(self) -> u8 {
        1 << self.index()
    }

    pub fn name(self) -> &'static str {
        match self {
            Sin::Pride => "Pride",
            Sin::Envy => "Envy",
            Sin::Wrath => "Wrath",
            Sin::Sloth => "Sloth",
            Sin::Greed => "Greed",
            Sin::Gluttony => "Gluttony",
            Sin::Lust => "Lust",
        }
    }

    /// The sin of `name` (any case): "greed", "Pride"...
    pub fn from_name(name: &str) -> Option<Sin> {
        SINS.into_iter().find(|s| s.name().eq_ignore_ascii_case(name))
    }

    /// Its name in capitals, for titles.
    pub fn title(self) -> &'static str {
        match self {
            Sin::Pride => "PRIDE",
            Sin::Envy => "ENVY",
            Sin::Wrath => "WRATH",
            Sin::Sloth => "SLOTH",
            Sin::Greed => "GREED",
            Sin::Gluttony => "GLUTTONY",
            Sin::Lust => "LUST",
        }
    }

    /// Its colour, on the door, in the fires by it, and on its key.
    pub fn color(self) -> u32 {
        match self {
            Sin::Pride => 0x9a5aff,
            Sin::Envy => 0x3ad06a,
            Sin::Wrath => 0xff3a2a,
            Sin::Sloth => 0x7a9ac8,
            Sin::Greed => 0xf2c53d,
            Sin::Gluttony => 0xff8a2a,
            Sin::Lust => 0xff4a9a,
        }
    }

    /// The prince of this sin, who keeps its key.
    pub fn prince(self) -> &'static str {
        match self {
            Sin::Pride => "Lucifer",
            Sin::Envy => "Leviathan",
            Sin::Wrath => "Amon",
            Sin::Sloth => "Belphegor",
            Sin::Greed => "Mammon",
            Sin::Gluttony => "Beelzebub",
            Sin::Lust => "Asmodeus",
        }
    }

    /// The prince of this sin as a foe: fought as himself in his court once it is built.
    pub fn prince_kind(self) -> EnemyKind {
        match self {
            Sin::Pride => EnemyKind::Lucifer,
            Sin::Envy => EnemyKind::Leviathan,
            Sin::Wrath => EnemyKind::Amon,
            Sin::Sloth => EnemyKind::Belphegor,
            Sin::Greed => EnemyKind::Mammon,
            Sin::Gluttony => EnemyKind::Beelzebub,
            Sin::Lust => EnemyKind::Asmodeus,
        }
    }

    /// The sin whose prince `kind` is, if he is one.
    pub fn of_prince(kind: EnemyKind) -> Option<Sin> {
        SINS.into_iter().find(|s| s.prince_kind() == kind)
    }

    /// The realm behind the door.
    pub fn realm(self) -> &'static str {
        match self {
            Sin::Pride => "The Hall of Mirrors",
            Sin::Envy => "The Serpent Grotto",
            Sin::Wrath => "The Burning Field",
            Sin::Sloth => "The Grey Mire",
            Sin::Greed => "The Vaults of Mammon",
            Sin::Gluttony => "The Rotting Feast",
            Sin::Lust => "The Garden of Thorns",
        }
    }

    /// What the door shows, for the tour.
    pub fn door(self) -> &'static str {
        match self {
            Sin::Pride => "a peacock fanning its tail beneath a crown",
            Sin::Envy => "a great slit-pupilled eye in the coils of a serpent",
            Sin::Wrath => "a snarling horned face in flames over crossed axes",
            Sin::Sloth => "a snail on a stopped hourglass, hung with cobwebs",
            Sin::Greed => "gold spilling from a chest into a grasping hand",
            Sin::Gluttony => "a fanged maw swallowing a feast",
            Sin::Lust => "two serpents twined into a heart among thorned roses",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seven_sins_seven_bits_seven_colours() {
        let bits: u8 = SINS.iter().map(|s| s.bit()).fold(0, |a, b| a | b);
        assert_eq!(bits, ALL_SINS);
        let mut colours: Vec<u32> = SINS.iter().map(|s| s.color()).collect();
        colours.sort();
        colours.dedup();
        assert_eq!(colours.len(), 7, "each sin its own colour");
        assert!(SINS.iter().all(|s| s.name().is_ascii() && s.door().is_ascii()));
        assert!(SINS.iter().all(|&s| Sin::from_name(&s.name().to_lowercase()) == Some(s)));
        assert_eq!(Sin::from_name("avarice"), None);
    }

    #[test]
    fn every_sins_prince_is_a_prince_by_frame_data() {
        use crate::logic::combat::archetype::{Archetype, Breed};
        use crate::logic::enemy::realm_foe_spec;
        for sin in SINS {
            let kind = sin.prince_kind();
            assert_eq!(Breed::of(kind).arch, Archetype::Prince, "{sin:?}");
            assert!(kind.court_own(), "{sin:?}");
            assert_eq!(realm_foe_spec(kind).map(|s| s.name), Some(sin.prince().to_string()), "{sin:?}");
        }
        let kinds: std::collections::HashSet<EnemyKind> = SINS.iter().map(|s| s.prince_kind()).collect();
        assert_eq!(kinds.len(), 7, "seven princes");
        assert!(SINS.iter().all(|&s| Sin::of_prince(s.prince_kind()) == Some(s)));
        assert_eq!(Sin::of_prince(EnemyKind::GoldGolem), None);
    }
}
