//! The Burning Field in play (Wrath's realm, REALMS.md §3; laid out in `logic::realms::field`). For now
//! a plain hall stands in for it, its far portal open.

use super::super::Game;
use super::Meter;
use crate::logic::types::Dir;

impl Game {
    /// As the realm is entered (its packs raised, the camera above).
    pub(in crate::game) fn enter_field(&mut self) {
        // (The stand-in has no trap: its far portal stands open.)
        self.gate_open = true;
    }

    /// The realm's own rules each tick.
    pub(in crate::game) fn update_field(&mut self, _dt: f64) {}

    /// The controls as the realm leaves them (`Game::steer_run`): untouched.
    pub(in crate::game) fn steer_field(&mut self, wanted: Dir) -> Dir {
        wanted
    }

    /// After a fall, as he rises at the checkpoint (and as the realm is first entered).
    pub(in crate::game) fn reset_field(&mut self) {}

    /// F2: straight to the last trap.
    pub(in crate::game) fn cheat_field(&mut self) {
        self.cheat_to_far_portal();
    }

    /// What the HUD shows of the realm's rules.
    pub(in crate::game) fn field_meters(&self) -> Vec<Meter> {
        Vec::new()
    }
}
