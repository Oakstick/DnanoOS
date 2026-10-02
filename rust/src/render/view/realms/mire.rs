//! The Grey Mire as the view draws it (Sloth's realm; laid out in `logic::realms::mire`). For now a
//! plain hall stands in for it.

use super::super::*;
use crate::logic::realms::mire::Mire;

/// How brightly his lantern burns here.
pub(super) const LANTERN: f32 = 0.8;

/// What the view keeps of the realm.
pub(in crate::render::view) struct MireView;

impl WorldView {
    /// The realm's static geometry.
    pub(super) fn build_mire(&mut self, grid: &Grid, _r: &Mire) -> MireView {
        self.build_plain_hall(grid, 0x2a2e30, 0x3a4248);
        MireView
    }

    /// The realm's own animation, one step.
    pub(super) fn update_mire_view(&mut self, _v: &mut MireView, _r: &Mire, _s: &SceneState, _dt: f32) {}

    /// What moves, glows and lights in the realm.
    pub(super) fn emit_mire(&self, _v: &MireView, _r: &Mire, _s: &SceneState, _time: f32, _hero: Vec3, _f: &mut Frame) {}
}

/// The realm's sky, fog and light.
pub(super) fn atmosphere(_v: &MireView, _r: &Mire, _s: &SceneState) -> Atmosphere {
    super::plain_atmosphere(Sin::Sloth.color())
}
