//! The Burning Field as the view draws it (Wrath's realm; laid out in `logic::realms::field`). For now a
//! plain hall stands in for it.

use super::super::*;
use crate::logic::realms::field::Battlefield;

/// How brightly his lantern burns here.
pub(super) const LANTERN: f32 = 0.5;

/// What the view keeps of the realm.
pub(in crate::render::view) struct FieldView;

impl WorldView {
    /// The realm's static geometry.
    pub(super) fn build_field(&mut self, grid: &Grid, _r: &Battlefield) -> FieldView {
        self.build_plain_hall(grid, 0x2a1c14, 0x4a2a1a);
        FieldView
    }

    /// The realm's own animation, one step.
    pub(super) fn update_field_view(&mut self, _v: &mut FieldView, _r: &Battlefield, _s: &SceneState, _dt: f32) {}

    /// What moves, glows and lights in the realm.
    pub(super) fn emit_field(&self, _v: &FieldView, _r: &Battlefield, _s: &SceneState, _time: f32, _hero: Vec3, _f: &mut Frame) {}
}

/// The realm's sky, fog and light.
pub(super) fn atmosphere(_v: &FieldView, _r: &Battlefield, _s: &SceneState) -> Atmosphere {
    super::plain_atmosphere(Sin::Wrath.color())
}
