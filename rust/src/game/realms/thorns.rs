//! The Garden of Thorns in play (Lust's realm, REALMS.md §6; laid out in `logic::realms::thorns`). For now
//! a plain hall stands in for it, its far portal open.

use super::super::Game;
use super::Meter;

impl Game {
    /// As the realm is entered (its packs raised, the camera above).
    pub(in crate::game) fn enter_thorns(&mut self) {
        // (The stand-in has no trap: its far portal stands open.)
        self.gate_open = true;
    }

    /// The realm's own rules each tick.
    pub(in crate::game) fn update_thorns(&mut self, _dt: f64) {}

    /// After a fall, as he rises at the checkpoint (and as the realm is first entered).
    pub(in crate::game) fn reset_thorns(&mut self) {}

    /// F2: straight to the last trap.
    pub(in crate::game) fn cheat_thorns(&mut self) {
        self.cheat_to_far_portal();
    }

    /// What the HUD shows of the realm's rules.
    pub(in crate::game) fn thorns_meters(&self) -> Vec<Meter> {
        Vec::new()
    }
}
