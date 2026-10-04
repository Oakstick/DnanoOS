//! Wrath's court, **The Anvil of Wrath** (`logic::court::wrath` lays it out), where Amon waits: its art.
//! A placeholder until the court is built: while `logic::court::wrath::BUILT` is false the court is
//! drawn as the stand-in meadow and none of this is called. Its entry points are the Counting
//! House's (`court_greed`): its light, its haze, its static scene as the level loads, and its
//! moving parts each frame (chosen by sin in `courts`).

use super::*;

/// The court's light (for now the Counting House's).
pub fn wrath_atmosphere() -> Atmosphere {
    super::court_greed::counting_atmosphere()
}

/// Its ground haze: density and falloff (for now the Counting House's).
pub const HAZE: (f32, f32) = (0.012, 2.0);

/// What the view keeps of the court for its moving parts.
#[derive(Clone, Debug, Default)]
pub struct AnvilView {}

impl WorldView {
    /// The court's static scene, as the level loads (for now none).
    pub(super) fn build_wrath_court(&mut self, _grid: &Grid) {
        self.anvil = Some(AnvilView::default());
    }

    /// The court's moving parts and lights, each frame (for now none).
    pub(super) fn emit_wrath_court(&self, _s: &SceneState, _time: f32, _hero: Vec3, _f: &mut Frame) {
        let Some(_view) = &self.anvil else { return };
    }
}
