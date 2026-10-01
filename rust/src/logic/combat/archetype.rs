//! The three archetypes (COMBAT.md §4.3), each asking the player one question: the Brute
//! (*can you wait?*), the Skirmisher (*can you read the timing?*) and the Hexer (*can you
//! close the distance?*). Each wears the body of a foe already in the game.

use super::frames::MoveId;
use super::fsm::Fighter;
use crate::logic::enemy::EnemyKind;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Archetype {
    Brute,
    Skirmisher,
    Hexer,
}

/// An archetype's numbers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stats {
    pub hp: f64,
    pub poise: f64,
    pub armor: f64,
    pub stagger: u16,
    pub resolve: u16,
    /// Tiles per second walking to its target, strafing on its ring, backing off.
    pub walk: f64,
    pub strafe: f64,
    pub retreat: f64,
    /// Radians per second it turns.
    pub turn: f64,
    /// The radius of its ring while it waits for a token.
    pub ring: f64,
    /// Threat points its token costs.
    pub cost: u8,
    /// Its body's radius.
    pub radius: f64,
    /// Frames before it may ask for a token again after its attack.
    pub rest: u16,
}

impl Archetype {
    pub const ALL: [Archetype; 3] = [Archetype::Brute, Archetype::Skirmisher, Archetype::Hexer];

    pub const fn stats(self) -> Stats {
        match self {
            Archetype::Brute => Stats {
                hp: 240.0,
                poise: 100.0,
                armor: 35.0,
                stagger: 90,
                resolve: 120,
                walk: 1.8,
                strafe: 1.2,
                retreat: 1.2,
                turn: 150.0 * std::f64::consts::PI / 180.0,
                ring: 3.6,
                cost: 2,
                radius: 0.5,
                rest: 90,
            },
            Archetype::Skirmisher => Stats {
                hp: 70.0,
                poise: 25.0,
                armor: 0.0,
                stagger: 50,
                resolve: 60,
                walk: 4.6,
                strafe: 2.8,
                retreat: 4.0,
                turn: 540.0 * std::f64::consts::PI / 180.0,
                ring: 4.2,
                cost: 1,
                radius: 0.38,
                rest: 60,
            },
            Archetype::Hexer => Stats {
                hp: 45.0,
                poise: 20.0,
                armor: 0.0,
                stagger: 60,
                resolve: 60,
                walk: 2.2,
                strafe: 2.0,
                retreat: 3.2,
                turn: 360.0 * std::f64::consts::PI / 180.0,
                ring: 6.0,
                cost: 1,
                radius: 0.35,
                rest: 75,
            },
        }
    }

    /// The body it wears.
    pub fn kind(self) -> EnemyKind {
        match self {
            Archetype::Brute => EnemyKind::Blemmyes,
            Archetype::Skirmisher => EnemyKind::Werewolf,
            Archetype::Hexer => EnemyKind::Witch,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Archetype::Brute => "BRUTE",
            Archetype::Skirmisher => "SKIRMISHER",
            Archetype::Hexer => "HEXER",
        }
    }

    /// Its moves.
    pub fn moves(self) -> &'static [MoveId] {
        match self {
            Archetype::Brute => &[MoveId::Slam, MoveId::Sweep],
            Archetype::Skirmisher => &[MoveId::Lunge],
            Archetype::Hexer => &[MoveId::Hex],
        }
    }

    /// The farthest it attacks from (centre to centre), to plan its approach.
    pub fn range(self) -> f64 {
        match self {
            Archetype::Brute => 1.7,
            Archetype::Skirmisher => LUNGE_FAR,
            Archetype::Hexer => HEX_FAR,
        }
    }

    /// A fresh fighter for it.
    pub fn fighter(self) -> Fighter {
        let s = self.stats();
        Fighter::new(s.poise, s.armor, s.stagger, s.resolve)
    }
}

/// The Brute sweeps a hero this close outside its slam's cone.
pub const SWEEP_NEAR: f64 = 1.3;
/// The Skirmisher lunges from between these distances, and steps back when nearer.
pub const LUNGE_NEAR: f64 = 1.6;
pub const LUNGE_FAR: f64 = 3.2;
/// The Hexer casts from no farther than this, and backs off from a hero nearer than `FLEE`.
pub const HEX_FAR: f64 = 7.5;
pub const HEX_NEAR: f64 = 2.5;
pub const FLEE: f64 = 3.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_archetypes_three_questions() {
        let (b, s, h) = (Archetype::Brute.stats(), Archetype::Skirmisher.stats(), Archetype::Hexer.stats());
        // The Brute can't be interrupted by light blows; the others flinch from everything.
        assert!(b.armor > MoveId::L1.data().poise && b.armor < MoveId::L3.data().poise);
        assert!(s.armor == 0.0 && h.armor == 0.0);
        // The Skirmisher's poise breaks inside one string; the Brute's takes two.
        let string = MoveId::L1.data().poise + MoveId::L2.data().poise + MoveId::L3.data().poise;
        assert!(s.poise <= MoveId::L1.data().poise + MoveId::L2.data().poise);
        assert!(b.poise > string && b.poise <= 2.0 * string);
        // The Skirmisher is a little slower than the hero (6.5 x 0.8 = 5.2 t/s), so he can always get away,
        // and more than twice as quick as the Brute.
        assert!(s.walk < 6.5 * 0.8 && s.walk > b.walk * 2.0);
        // The Hexer waits beyond the others, outside the hero's reach.
        assert!(h.ring > s.ring && h.ring > b.ring && FLEE > MoveId::L3.data().reach);
        // A Brute's token is worth two of the others.
        assert_eq!((b.cost, s.cost, h.cost), (2, 1, 1));
    }
}
