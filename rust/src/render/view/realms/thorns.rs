//! The Garden of Thorns as the view draws it (Lust's realm; laid out in `logic::realms::thorns`). For now a
//! plain hall stands in for it.

use super::super::*;
use crate::logic::realms::thorns::Thorns;

/// How brightly his lantern burns here.
pub(super) const LANTERN: f32 = 0.6;

/// What the view keeps of the realm.
pub(in crate::render::view) struct ThornsView;

impl WorldView {
    /// The realm's static geometry.
    pub(super) fn build_thorns(&mut self, grid: &Grid, _r: &Thorns) -> ThornsView {
        self.build_plain_hall(grid, 0x2a1a20, 0x4a2030);
        ThornsView
    }

    /// The realm's own animation, one step.
    pub(super) fn update_thorns_view(&mut self, _v: &mut ThornsView, _r: &Thorns, _s: &SceneState, _dt: f32) {}

    /// What moves, glows and lights in the realm.
    pub(super) fn emit_thorns(&self, _v: &ThornsView, _r: &Thorns, _s: &SceneState, _time: f32, _hero: Vec3, _f: &mut Frame) {}
}

/// The realm's sky, fog and light.
pub(super) fn atmosphere(_v: &ThornsView, _r: &Thorns, _s: &SceneState) -> Atmosphere {
    super::plain_atmosphere(Sin::Lust.color())
}
