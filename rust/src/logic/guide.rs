//! The old man's guide (B): what he tells the hero about the place he stands in. The tale or the
//! myth behind it, what he must do there, and what to watch for. He always leads to good: every
//! word is true of the game (see the tests).
//!
//! Pure data. The game adds what it knows now (the realm's general, keys held) under these.

use crate::logic::heroes::HeroKind;
use crate::logic::sins::Sin;

/// Where the hero stands, as the guide tells it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum GuidePlace {
    /// Dungeon I, the Garden of Krampus.
    Garden,
    /// Dungeon II, the Cisterns.
    Cisterns,
    /// Dungeon III, the Bone Vaults.
    BoneVaults,
    /// The Cyclops' meadow after dungeon `n` (0, 1, 2).
    Cyclops(usize),
    /// The Portals of Hell, the hub over the lava.
    Hub,
    /// A sin's realm.
    Realm(Sin),
    /// A sin's prince in his court.
    Court(Sin),
    /// The gym and its Sparring Ring.
    Gym,
}

/// What he says of a place.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Guide {
    /// The place's name, as a heading.
    pub title: String,
    /// The tale, the myth or the sin behind it.
    pub tale: Vec<String>,
    /// What the hero must do here.
    pub task: Vec<String>,
    /// What to watch for: its foes, its ground, its tricks, and how to meet them.
    pub watch: Vec<String>,
}

/// The guide to `place`, in the old man's words to `hero`.
pub fn guide(place: GuidePlace, hero: HeroKind) -> Guide {
    let _ = hero;
    Guide { title: format!("{place:?}").to_uppercase(), ..Guide::default() }
}
