//! The Warband (WARBAND.md): what decides who fights, where, when and why, above the frame data
//! and the threat manager, and what the game shows and sounds of it.
//!
//! This module holds what every layer shares: the war log's events (§3), which every outcome of a
//! fight goes out through, and the presentation's own rng, which the feel and the sound draw from
//! so that nothing they do can touch the simulation (rule I5).

pub mod drum;
pub mod ledger;
pub mod names;
pub mod pack;
pub mod save;

use crate::logic::combat::fsm::{Defense, Effect};
use crate::logic::combat::MoveId;
use crate::logic::enemy::EnemyKind;
use crate::logic::rng::Rng;
use crate::logic::types::{PotionKind, V};

/// One thing that happened in a fight, as the layers above and around it hear of it. The brawl
/// writes them in the order they happen; `game::war` reads them once a frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WarEvent {
    /// A foe began a move: its startup, its tell (`foe` is its duelist id).
    Tell { foe: u32, kind: EnemyKind, mv: MoveId, at: V },
    /// A foe's strike reached the hero, however he met it. `dmg` is what he lost; `dir` the way it
    /// came (unit, from the foe toward him); `lands` whether it counts as a landing (a volley's later
    /// bolts and a flurry's later bites do not); `bolt` whether it flew.
    Landed { foe: u32, kind: Option<EnemyKind>, how: Defense, dmg: f64, dir: V, lands: bool, bolt: bool },
    /// A foe's blow ran out its active frames without meeting him.
    Whiffed { foe: u32, kind: EnemyKind, mv: MoveId },
    /// The hero struck a foe: what the blow did to its fighter, the damage, whether it was his
    /// finisher, and the way it went (unit, from him toward the foe). `warded` is the share its
    /// legend turned (0 when none was).
    HeroHit { foe: u32, kind: EnemyKind, effect: Effect, dmg: f64, finisher: bool, dir: V, at: V, warded: f64 },
    /// A foe was slain. `last_of_pack`: no foe of its den (or, in the ring, of the bout) stands.
    Slain { foe: u32, kind: EnemyKind, at: V, dir: V, last_of_pack: bool },
    /// He drank a potion.
    Drank { potion: PotionKind },
}

impl WarEvent {
    /// The foe it is about, if any.
    pub fn foe(&self) -> Option<u32> {
        match *self {
            WarEvent::Tell { foe, .. }
            | WarEvent::Landed { foe, .. }
            | WarEvent::Whiffed { foe, .. }
            | WarEvent::HeroHit { foe, .. }
            | WarEvent::Slain { foe, .. } => Some(foe),
            WarEvent::Drank { .. } => None,
        }
    }
}

/// The presentation's rng: shakes, sparks, debris, the voices' variants, barks. Seeded apart from
/// the game's, and never drawn by the simulation, so a harness prints the same numbers with the
/// feel on or off.
#[derive(Clone, Debug)]
pub struct PresRng(Rng);

impl PresRng {
    pub fn new(seed: u32) -> Self {
        Self(Rng::new(seed ^ 0x5eed_f00d))
    }
    /// The next number in [0, 1). (Not an `Iterator`, as `Rng` is not.)
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> f64 {
        self.0.next()
    }
    /// Uniform in [a, b).
    pub fn range(&mut self, a: f64, b: f64) -> f64 {
        self.0.range(a, b)
    }
    pub fn index(&mut self, len: usize) -> usize {
        self.0.index(len)
    }
}

impl Default for PresRng {
    fn default() -> Self {
        Self::new(1)
    }
}
