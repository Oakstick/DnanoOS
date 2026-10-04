//! Lust's court in play, once it is built (`logic::court::lust` lays it out): Asmodeus's own doings,
//! through the hooks `game::court` calls for his court. Empty until his fight is built.

use crate::game::Game;
use crate::logic::types::V;

impl Game {
    /// As Asmodeus is set down where his fight begins.
    pub(super) fn lust_enter(&mut self, _prince: u32) {}

    /// Each tick of the fight in his court.
    pub(super) fn lust_tick(&mut self, _dt: f64) {}

    /// As his health falls into `phase` (`self.adds_unsummoned(prince)` says whether his
    /// phase-two adds have yet to come).
    pub(super) fn lust_phase(&mut self, _prince: u32, _at: V, _phase: u8) {}

    /// After a fall (his adds are already gone).
    pub(super) fn lust_reset(&mut self) {}

    /// As he falls.
    pub(super) fn lust_slain(&mut self, _at: V) {}

    /// The phase banner's line under his name, and its colour.
    pub(super) fn lust_banner(phase: u8) -> (&'static str, u32) {
        match phase {
            2 => ("HE GROWS ANGRY   WATCH FOR HIS NEW MOVES", 0xd8a040),
            _ => ("HE FIGHTS FOR HIS LIFE   STRIKE IN HIS RECOVERIES", 0xff8a3a),
        }
    }

    /// His adds at once (his general's decree done): his elf-maids.
    pub(super) fn lust_raise(&mut self, _prince: u32, _at: V) {}
}
