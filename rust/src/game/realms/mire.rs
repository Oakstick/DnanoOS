//! The Grey Mire in play (Sloth's realm, REALMS.md §4; laid out in `logic::realms::mire`). For now
//! a plain hall stands in for it, its far portal open.

use super::super::Game;
use super::super::MenuItem;
use super::Meter;
use super::Offer;

impl Game {
    /// As the realm is entered (its packs raised, the camera above).
    pub(in crate::game) fn enter_mire(&mut self) {
        // (The stand-in has no trap: its far portal stands open.)
        self.gate_open = true;
    }

    /// The realm's own rules each tick.
    pub(in crate::game) fn update_mire(&mut self, _dt: f64) {}

    /// After a fall, as he rises at the checkpoint (and as the realm is first entered).
    pub(in crate::game) fn reset_mire(&mut self) {}

    /// F2: straight to the last trap.
    pub(in crate::game) fn cheat_mire(&mut self) {
        self.cheat_to_far_portal();
    }

    /// What the HUD shows of the realm's rules.
    pub(in crate::game) fn mire_meters(&self) -> Vec<Meter> {
        Vec::new()
    }

    /// The offer standing before him (the bed at the hourglass's foot), if one is.
    pub(in crate::game) fn mire_offer(&self) -> Option<Offer> {
        None
    }

    /// His answer to it.
    pub(in crate::game) fn answer_mire(&mut self, _choice: MenuItem) {
        self.enter(super::super::Screen::Playing);
    }
}
