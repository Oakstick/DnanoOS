//! The Hall of Mirrors as the view draws it (Pride's realm; laid out in `logic::realms::mirrors`). For now a
//! plain hall stands in for it.

use super::super::*;
use crate::logic::realms::mirrors::Mirrors;

/// How brightly his lantern burns here.
pub(super) const LANTERN: f32 = 0.6;

/// What the view keeps of the realm.
pub(in crate::render::view) struct MirrorsView;

impl WorldView {
    /// The realm's static geometry.
    pub(super) fn build_mirrors(&mut self, grid: &Grid, _r: &Mirrors) -> MirrorsView {
        self.build_plain_hall(grid, 0x2a2438, 0x4a3a6a);
        MirrorsView
    }

    /// The realm's own animation, one step.
    pub(super) fn update_mirrors_view(&mut self, _v: &mut MirrorsView, _r: &Mirrors, _s: &SceneState, _dt: f32) {}

    /// What moves, glows and lights in the realm.
    pub(super) fn emit_mirrors(&self, _v: &MirrorsView, _r: &Mirrors, _s: &SceneState, _time: f32, _hero: Vec3, _f: &mut Frame) {}
}

/// The realm's sky, fog and light.
pub(super) fn atmosphere(_v: &MirrorsView, _r: &Mirrors, _s: &SceneState) -> Atmosphere {
    super::plain_atmosphere(Sin::Pride.color())
}
