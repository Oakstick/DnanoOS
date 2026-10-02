//! The Serpent Grotto in play (Envy's realm, REALMS.md §2; laid out in `logic::realms::grotto`). For now
//! a plain hall stands in for it, its far portal open.

use super::super::Game;
use super::Meter;

impl Game {
    /// As the realm is entered (its packs raised, the camera above).
    pub(in crate::game) fn enter_grotto(&mut self) {
        // (The stand-in has no trap: its far portal stands open.)
        self.gate_open = true;
    }

    /// The realm's own rules each tick.
    pub(in crate::game) fn update_grotto(&mut self, _dt: f64) {}

    /// After a fall, as he rises at the checkpoint (and as the realm is first entered).
    pub(in crate::game) fn reset_grotto(&mut self) {}

    /// F2: straight to the last trap.
    pub(in crate::game) fn cheat_grotto(&mut self) {
        self.cheat_to_far_portal();
    }

    /// What the HUD shows of the realm's rules.
    pub(in crate::game) fn grotto_meters(&self) -> Vec<Meter> {
        Vec::new()
    }
}
