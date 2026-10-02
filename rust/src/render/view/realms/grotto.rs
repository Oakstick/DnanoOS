//! The Serpent Grotto as the view draws it (Envy's realm; laid out in `logic::realms::grotto`). For now a
//! plain hall stands in for it.

use super::super::*;
use crate::logic::realms::grotto::Grotto;

/// How brightly his lantern burns here.
pub(super) const LANTERN: f32 = 0.7;

/// What the view keeps of the realm.
pub(in crate::render::view) struct GrottoView;

impl WorldView {
    /// The realm's static geometry.
    pub(super) fn build_grotto(&mut self, grid: &Grid, _r: &Grotto) -> GrottoView {
        self.build_plain_hall(grid, 0x1a2a24, 0x2a4a3a);
        GrottoView
    }

    /// The realm's own animation, one step.
    pub(super) fn update_grotto_view(&mut self, _v: &mut GrottoView, _r: &Grotto, _s: &SceneState, _dt: f32) {}

    /// What moves, glows and lights in the realm.
    pub(super) fn emit_grotto(&self, _v: &GrottoView, _r: &Grotto, _s: &SceneState, _time: f32, _hero: Vec3, _f: &mut Frame) {}
}

/// The realm's sky, fog and light.
pub(super) fn atmosphere(_v: &GrottoView, _r: &Grotto, _s: &SceneState) -> Atmosphere {
    super::plain_atmosphere(Sin::Envy.color())
}
