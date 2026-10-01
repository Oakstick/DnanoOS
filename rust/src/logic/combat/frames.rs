//! Frame data: every move broken into startup, active and recovery frames (60 Hz), with what
//! it deals and how it strikes. The table is COMBAT.md §1; the tests below hold it to the
//! design limits of §0 (the reaction floor, the punish window, the parry by reaction).

use std::f64::consts::PI;

/// Human visual reaction, in frames (about 250 ms).
pub const REACTION: u16 = 15;

/// Every move in the game.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub enum MoveId {
    /// The hero's training sword: a string of three.
    L1,
    L2,
    L3,
    /// The Brute's overhead slam: slow, far, narrow.
    Slam,
    /// The Brute's sweep, for a hero at his flank.
    Sweep,
    /// The Skirmisher's lunge: a gap-closing bite.
    Lunge,
    /// The Hexer's bolt.
    Hex,
}

/// Where in a move a frame falls.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Startup,
    Active,
    Recovery,
}

/// One move's frame data and hitbox.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoveData {
    pub name: &'static str,
    pub startup: u16,
    pub active: u16,
    pub recovery: u16,
    /// Base damage (before power, state, and proration).
    pub damage: f64,
    /// Base poise damage.
    pub poise: f64,
    /// Frames the struck reel (before decay).
    pub hitstun: u16,
    /// The hitbox: a cone of `reach` tiles and `half_angle` radians either side of the aim.
    pub reach: f64,
    pub half_angle: f64,
    /// Poise damage a blow needs to interrupt this move in its startup and active frames.
    pub armor: f64,
    /// Startup frames in which the attacker still turns toward its target; after them the aim is committed.
    pub track: u16,
    /// Tiles the attacker travels along its aim over the active frames (a lunge).
    pub travel: f64,
    /// The next move of the string, for an attack buffered in the cancel window.
    pub chain: Option<MoveId>,
    /// Recovery frame from which the move may be cancelled (into its chain, an evade or the guard).
    pub cancel: u16,
    /// Freeze frames when it lands.
    pub hitstop: u16,
    /// It looses a projectile on its first active frame instead of striking.
    pub projectile: bool,
}

const fn deg(d: f64) -> f64 {
    d * PI / 180.0
}

const BASE: MoveData = MoveData {
    name: "",
    startup: 1,
    active: 1,
    recovery: 1,
    damage: 0.0,
    poise: 0.0,
    hitstun: 0,
    reach: 1.0,
    half_angle: deg(45.0),
    armor: 0.0,
    track: 0,
    travel: 0.0,
    chain: None,
    cancel: u16::MAX,
    hitstop: 0,
    projectile: false,
};

/// The Hexer's bolt: tiles per second, frames it flies, and its radius.
pub const BOLT_SPEED: f64 = 7.0;
pub const BOLT_LIFE: u16 = 90;
pub const BOLT_RADIUS: f64 = 0.3;
/// A parried bolt flies back this much faster, and deals this many times its damage to foes.
pub const REFLECT_SPEED: f64 = 1.6;
pub const REFLECT_DAMAGE: f64 = 2.0;

impl MoveId {
    pub const HERO: [MoveId; 3] = [MoveId::L1, MoveId::L2, MoveId::L3];
    pub const FOE: [MoveId; 4] = [MoveId::Slam, MoveId::Sweep, MoveId::Lunge, MoveId::Hex];

    pub const fn data(self) -> MoveData {
        match self {
            MoveId::L1 => MoveData {
                name: "L1",
                startup: 6,
                active: 4,
                recovery: 14,
                damage: 10.0,
                poise: 20.0,
                hitstun: 14,
                reach: 1.5,
                half_angle: deg(60.0),
                track: 4,
                chain: Some(MoveId::L2),
                cancel: 4,
                hitstop: 3,
                ..BASE
            },
            MoveId::L2 => MoveData {
                name: "L2",
                startup: 5,
                active: 4,
                recovery: 16,
                damage: 11.0,
                poise: 20.0,
                hitstun: 14,
                reach: 1.5,
                half_angle: deg(60.0),
                track: 3,
                chain: Some(MoveId::L3),
                cancel: 4,
                hitstop: 3,
                ..BASE
            },
            MoveId::L3 => MoveData {
                name: "L3",
                startup: 9,
                active: 5,
                recovery: 22,
                damage: 16.0,
                poise: 45.0,
                hitstun: 24,
                reach: 1.7,
                half_angle: deg(75.0),
                track: 5,
                cancel: 10,
                hitstop: 6,
                ..BASE
            },
            MoveId::Slam => MoveData {
                name: "SLAM",
                startup: 42,
                active: 6,
                recovery: 48,
                damage: 30.0,
                reach: 1.8,
                half_angle: deg(35.0),
                armor: 35.0,
                track: 30,
                hitstop: 6,
                ..BASE
            },
            MoveId::Sweep => MoveData {
                name: "SWEEP",
                startup: 30,
                active: 8,
                recovery: 36,
                damage: 20.0,
                reach: 1.6,
                half_angle: deg(110.0),
                armor: 35.0,
                track: 16,
                hitstop: 5,
                ..BASE
            },
            MoveId::Lunge => MoveData {
                name: "LUNGE",
                startup: 18,
                active: 8,
                recovery: 22,
                damage: 15.0,
                reach: 0.9,
                half_angle: deg(70.0),
                track: 12,
                travel: 2.6,
                hitstop: 4,
                ..BASE
            },
            MoveId::Hex => MoveData {
                name: "HEX",
                startup: 36,
                active: 1,
                recovery: 30,
                damage: 12.0,
                reach: BOLT_SPEED * BOLT_LIFE as f64 / 60.0,
                half_angle: 0.0,
                track: 30,
                hitstop: 3,
                projectile: true,
                ..BASE
            },
        }
    }

    /// One of the hero's.
    pub fn is_hero(self) -> bool {
        Self::HERO.contains(&self)
    }
}

impl MoveData {
    pub const fn total(&self) -> u16 {
        self.startup + self.active + self.recovery
    }

    /// The phase of frame `frame` (0 = the first startup frame).
    pub fn phase(&self, frame: u16) -> Phase {
        if frame < self.startup {
            Phase::Startup
        } else if frame < self.startup + self.active {
            Phase::Active
        } else {
            Phase::Recovery
        }
    }

    /// The first frame from which the move may be cancelled.
    pub fn cancel_frame(&self) -> u16 {
        (self.startup + self.active).saturating_add(self.cancel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::combat::fsm::{GUARD_RAISE, PARRY_FROM, PARRY_TO};

    #[test]
    fn every_foe_attack_can_be_read_before_it_lands() {
        for m in MoveId::FOE {
            let d = m.data();
            assert!(d.startup >= 18, "{}: {} f of startup is under the 300 ms reaction floor", d.name, d.startup);
        }
    }

    #[test]
    fn every_foe_attack_can_be_punished_when_it_misses() {
        let l1 = MoveId::L1.data();
        for m in MoveId::FOE {
            let d = m.data();
            assert!(
                d.recovery >= l1.startup + l1.active,
                "{}: recovery {} f is shorter than L1 ({} f to land)",
                d.name,
                d.recovery,
                l1.startup + l1.active
            );
        }
        // The slam's recovery holds a whole string if the punish starts by frame 12.
        let slam = MoveId::Slam.data();
        let l3_lands = 14 + 13 + MoveId::L3.data().startup;
        assert!(slam.recovery >= 12 + l3_lands, "slam {} f vs {} f", slam.recovery, 12 + l3_lands);
    }

    #[test]
    fn every_melee_foe_attack_can_be_parried_on_reaction() {
        const { assert!(PARRY_TO - PARRY_FROM + 1 == 8 && GUARD_RAISE == PARRY_FROM) };
        for m in MoveId::FOE.into_iter().filter(|m| !m.data().projectile) {
            let d = m.data();
            // Guard pressed REACTION frames after the tell begins is up before the hit, and a
            // later press can still put the hit inside the window.
            assert!(d.startup >= REACTION + PARRY_FROM, "{}: {} f is too quick to parry on sight", d.name, d.startup);
        }
    }

    #[test]
    fn the_strings_frames_and_damage_add_up() {
        let (l1, l2, l3) = (MoveId::L1.data(), MoveId::L2.data(), MoveId::L3.data());
        assert_eq!((l1.total(), l2.total(), l3.total()), (24, 25, 36));
        assert_eq!((l1.cancel_frame(), l2.cancel_frame()), (14, 13), "the chain comes out at f14, then f27");
        let string = l1.cancel_frame() + l2.cancel_frame() + l3.total();
        assert_eq!(string, 63, "1.05 s");
        let dps = (l1.damage + l2.damage + l3.damage) / (string as f64 / 60.0);
        assert!((dps - 35.24).abs() < 0.01, "{dps}");
        assert_eq!(l1.phase(5), Phase::Startup);
        assert_eq!(l1.phase(6), Phase::Active);
        assert_eq!(l1.phase(10), Phase::Recovery);
    }
}
