//! Wrath's court in play, once it is built (`logic::court::wrath` lays it out): Amon's own doings,
//! through the hooks `game::court` calls for his court. Empty until his fight is built.

use crate::game::Game;
use crate::logic::types::V;

impl Game {
    /// As Amon is set down where his fight begins.
    pub(super) fn wrath_enter(&mut self, _prince: u32) {}

    /// Each tick of the fight in his court.
    pub(super) fn wrath_tick(&mut self, _dt: f64) {}

    /// As his health falls into `phase` (`self.adds_unsummoned(prince)` says whether his
    /// phase-two adds have yet to come).
    pub(super) fn wrath_phase(&mut self, _prince: u32, _at: V, _phase: u8) {}

    /// After a fall (his adds are already gone).
    pub(super) fn wrath_reset(&mut self) {}

    /// As he falls.
    pub(super) fn wrath_slain(&mut self, _at: V) {}

    /// The phase banner's line under his name, and its colour.
    pub(super) fn wrath_banner(phase: u8) -> (&'static str, u32) {
        match phase {
            2 => ("HE GROWS ANGRY   WATCH FOR HIS NEW MOVES", 0xd8a040),
            _ => ("HE FIGHTS FOR HIS LIFE   STRIKE IN HIS RECOVERIES", 0xff8a3a),
        }
    }

    /// His adds at once (his general's decree done): his ravens.
    pub(super) fn wrath_raise(&mut self, _prince: u32, _at: V) {}
}
