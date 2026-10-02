//! The combat core, as COMBAT.md specifies it: frame data for every move (`frames`), the
//! formulas (`math`), the one state machine the hero and every foe run (`fsm`), the threat
//! manager that hands out attack tokens and places the waiting foes (`threat`), the three
//! archetypes (`archetype`), a foe's mind (`duelist`), and the hero's gear as numbers (`kit`).
//!
//! Pure logic on a fixed 60 Hz frame. The simulation steps at 120 Hz; `FrameClock` turns its
//! steps into whole frames, so every number here is in the frames of a fighting-game table.

pub mod archetype;
pub mod duelist;
pub mod frames;
pub mod fsm;
pub mod kit;
pub mod math;
pub mod threat;

pub use archetype::{Archetype, Breed};
pub use duelist::Duelist;
pub use frames::{MoveData, MoveId, Phase};
pub use fsm::{Event, Fighter, Input, Reel, State};
pub use kit::Kit;
pub use threat::ThreatManager;

/// Frames per second of all frame data.
pub const FPS: f64 = 60.0;

/// Seconds in `frames`.
pub fn secs(frames: u16) -> f64 {
    frames as f64 / FPS
}

/// Milliseconds in `frames`.
pub fn ms(frames: u16) -> f64 {
    frames as f64 * 1000.0 / FPS
}

/// Turns the simulation's steps into whole 60 Hz frames.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameClock {
    carry: f64,
}

impl FrameClock {
    /// Advances by `dt` seconds; returns how many whole frames passed.
    pub fn advance(&mut self, dt: f64) -> u32 {
        self.carry += dt * FPS;
        let n = (self.carry + 1e-9).floor().max(0.0);
        self.carry -= n;
        n as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_simulation_steps_make_one_combat_frame() {
        let mut c = FrameClock::default();
        let frames: u32 = (0..240).map(|_| c.advance(crate::app::STEP)).sum();
        assert_eq!(frames, 120, "two seconds");
        assert_eq!(ms(6), 100.0);
        assert!((secs(90) - 1.5).abs() < 1e-12);
    }
}
