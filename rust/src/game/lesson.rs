//! Hızır's lesson in the Garden (B04).

use super::Game;

impl Game {
    /// The lesson holds the Garden still (no hunters come while Hızır teaches).
    pub(super) fn lesson_holds(&self) -> bool {
        false
    }

    /// No lesson (the harnesses, and a player who skips it).
    pub fn skip_lesson(&mut self) {}
}
