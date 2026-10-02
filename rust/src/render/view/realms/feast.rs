//! The Rotting Feast as the view draws it (Gluttony's realm; laid out in `logic::realms::feast`). For now a
//! plain hall stands in for it.

use super::super::*;
use crate::logic::realms::feast::Feast;

/// How brightly his lantern burns here.
pub(super) const LANTERN: f32 = 0.6;

/// What the view keeps of the realm.
pub(in crate::render::view) struct FeastView;

impl WorldView {
    /// The realm's static geometry.
    pub(super) fn build_feast(&mut self, grid: &Grid, _r: &Feast) -> FeastView {
        self.build_plain_hall(grid, 0x3a2a1a, 0x5a3a24);
        FeastView
    }

    /// The realm's own animation, one step.
    pub(super) fn update_feast_view(&mut self, _v: &mut FeastView, _r: &Feast, _s: &SceneState, _dt: f32) {}

    /// What moves, glows and lights in the realm.
    pub(super) fn emit_feast(&self, _v: &FeastView, _r: &Feast, _s: &SceneState, _time: f32, _hero: Vec3, _f: &mut Frame) {}
}

/// The realm's sky, fog and light.
pub(super) fn atmosphere(_v: &FeastView, _r: &Feast, _s: &SceneState) -> Atmosphere {
    super::plain_atmosphere(Sin::Gluttony.color())
}
