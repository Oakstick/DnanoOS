//! Envy's court in play, once it is built (`logic::court::envy` lays it out): Leviathan's own doings,
//! through the hooks `game::court` calls for his court. Empty until his fight is built.

use crate::game::Game;
use crate::logic::types::V;

impl Game {
    /// As Leviathan is set down where his fight begins.
    pub(super) fn envy_enter(&mut self, _prince: u32) {}

    /// Each tick of the fight in his court.
    pub(super) fn envy_tick(&mut self, _dt: f64) {}

    /// As his health falls into `phase` (`self.adds_unsummoned(prince)` says whether his
    /// phase-two adds have yet to come).
    pub(super) fn envy_phase(&mut self, _prince: u32, _at: V, _phase: u8) {}

    /// After a fall (his adds are already gone).
    pub(super) fn envy_reset(&mut self) {}

    /// As he falls.
    pub(super) fn envy_slain(&mut self, _at: V) {}

    /// The phase banner's line under his name, and its colour.
    pub(super) fn envy_banner(phase: u8) -> (&'static str, u32) {
        match phase {
            2 => ("HE GROWS ANGRY   WATCH FOR HIS NEW MOVES", 0xd8a040),
            _ => ("HE FIGHTS FOR HIS LIFE   STRIKE IN HIS RECOVERIES", 0xff8a3a),
        }
    }

    /// His adds at once (his general's decree done): his brood.
    pub(super) fn envy_raise(&mut self, _prince: u32, _at: V) {}
}
