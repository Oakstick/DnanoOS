//! The six realms after the vaults as the view draws them (`logic::realms` lays them out). Each has
//! its own ground, props, sky and light in its module. This one hands them the build, the frame and
//! the atmosphere, and keeps what they share: the portals in and out (Greed's are the vaults'
//! own), and a plain hall for a realm standing in for one not yet built.

mod feast;
mod field;
mod grotto;
mod mire;
mod mirrors;
mod thorns;

use super::*;
use crate::logic::realms::Run;

/// What the view keeps of the realm being played: its own module's state.
pub(super) enum RunView {
    Pride(mirrors::MirrorsView),
    Envy(grotto::GrottoView),
    Wrath(field::FieldView),
    Sloth(mire::MireView),
    Gluttony(feast::FeastView),
    Lust(thorns::ThornsView),
}

impl WorldView {
    /// The realm's static geometry, and its portals.
    pub(super) fn build_run(&mut self, grid: &Grid, run: &Run) {
        let view = match run {
            Run::Pride(r) => RunView::Pride(self.build_mirrors(grid, r)),
            Run::Envy(r) => RunView::Envy(self.build_grotto(grid, r)),
            Run::Wrath(r) => RunView::Wrath(self.build_field(grid, r)),
            Run::Sloth(r) => RunView::Sloth(self.build_mire(grid, r)),
            Run::Gluttony(r) => RunView::Gluttony(self.build_feast(grid, r)),
            Run::Lust(r) => RunView::Lust(self.build_thorns(grid, r)),
        };
        // The portals in and out, in the sin's colours, as in the Portals of Hell.
        let b = run.base();
        self.build_portal_arch(&b.entry);
        self.build_portal_arch(&b.exit);
        self.run_view = Some(view);
    }

    /// The realm's own animation, one step.
    pub(super) fn update_run_view(&mut self, s: &SceneState, dt: f32) {
        let Some(run) = s.run else { return };
        let Some(mut v) = self.run_view.take() else { return };
        match (&mut v, run) {
            (RunView::Pride(v), Run::Pride(r)) => self.update_mirrors_view(v, r, s, dt),
            (RunView::Envy(v), Run::Envy(r)) => self.update_grotto_view(v, r, s, dt),
            (RunView::Wrath(v), Run::Wrath(r)) => self.update_field_view(v, r, s, dt),
            (RunView::Sloth(v), Run::Sloth(r)) => self.update_mire_view(v, r, s, dt),
            (RunView::Gluttony(v), Run::Gluttony(r)) => self.update_feast_view(v, r, s, dt),
            (RunView::Lust(v), Run::Lust(r)) => self.update_thorns_view(v, r, s, dt),
            _ => {}
        }
        self.run_view = Some(v);
    }

    /// Everything of the realm that moves, glows or lights, and its portals.
    pub(super) fn emit_run(&self, s: &SceneState, time: f32, hero: Vec3, f: &mut Frame) {
        let (Some(v), Some(run)) = (self.run_view.as_ref(), s.run) else { return };
        match (v, run) {
            (RunView::Pride(v), Run::Pride(r)) => self.emit_mirrors(v, r, s, time, hero, f),
            (RunView::Envy(v), Run::Envy(r)) => self.emit_grotto(v, r, s, time, hero, f),
            (RunView::Wrath(v), Run::Wrath(r)) => self.emit_field(v, r, s, time, hero, f),
            (RunView::Sloth(v), Run::Sloth(r)) => self.emit_mire(v, r, s, time, hero, f),
            (RunView::Gluttony(v), Run::Gluttony(r)) => self.emit_feast(v, r, s, time, hero, f),
            (RunView::Lust(v), Run::Lust(r)) => self.emit_thorns(v, r, s, time, hero, f),
            _ => {}
        }
        // The portals: the one he came through shut behind him, the far one opening once its seal breaks.
        let door = s.gate.map(|g| g.door).unwrap_or(0.0);
        let b = run.base();
        self.emit_portal(&b.entry, 0.0, false, time, hero, f);
        self.emit_portal(&b.exit, door, false, time, hero, f);
    }

    /// The realm's sky, fog and light.
    pub(super) fn run_atmosphere(&self, s: &SceneState) -> Option<Atmosphere> {
        let (v, run) = (self.run_view.as_ref()?, s.run?);
        Some(match (v, run) {
            (RunView::Pride(v), Run::Pride(r)) => mirrors::atmosphere(v, r, s),
            (RunView::Envy(v), Run::Envy(r)) => grotto::atmosphere(v, r, s),
            (RunView::Wrath(v), Run::Wrath(r)) => field::atmosphere(v, r, s),
            (RunView::Sloth(v), Run::Sloth(r)) => mire::atmosphere(v, r, s),
            (RunView::Gluttony(v), Run::Gluttony(r)) => feast::atmosphere(v, r, s),
            (RunView::Lust(v), Run::Lust(r)) => thorns::atmosphere(v, r, s),
            _ => return None,
        })
    }

    /// How brightly his own lantern burns in the realm (each realm's light is mostly its own).
    pub(super) fn run_lantern(&self) -> Option<f32> {
        Some(match self.run_view.as_ref()? {
            RunView::Pride(_) => mirrors::LANTERN,
            RunView::Envy(_) => grotto::LANTERN,
            RunView::Wrath(_) => field::LANTERN,
            RunView::Sloth(_) => mire::LANTERN,
            RunView::Gluttony(_) => feast::LANTERN,
            RunView::Lust(_) => thorns::LANTERN,
        })
    }

    /// A plain hall standing in for a realm not yet built: a floor of `floor` slabs, walls of `wall`
    /// blocks round it.
    pub(super) fn build_plain_hall(&mut self, grid: &Grid, floor: u32, wall: u32) {
        for (x, y, t) in grid.cells() {
            let (xf, yf) = (x as f32, y as f32);
            let h = hash(x as f64 * 1.3, y as f64 * 2.1) as f32;
            if t == Tile::Rock {
                let by_floor = CARDINALS.iter().any(|d| {
                    let (dx, dy) = d.vec();
                    grid.is_walkable(x + dx, y + dy, false)
                });
                if by_floor {
                    let m = Mat4::from_scale_rotation_translation(Vec3::new(1.0, 1.4, 1.0), Quat::IDENTITY, Vec3::new(xf, 0.7, yf));
                    self.static_cubes.push(Instance::new(m, lin(wall) * (0.9 + 0.2 * h), 1.0, Vec3::ZERO, SHADE_LIT));
                }
                continue;
            }
            let m = Mat4::from_scale_rotation_translation(Vec3::new(0.97, 0.12, 0.97), Quat::IDENTITY, Vec3::new(xf, -0.06, yf));
            self.static_cubes.push(Instance::new(m, lin(floor) * (0.9 + 0.2 * h), 1.0, Vec3::ZERO, SHADE_LIT));
        }
    }
}

/// A dark hall's air under a sin's colour (for the stand-ins).
pub(super) fn plain_atmosphere(sin_color: u32) -> Atmosphere {
    let mut a = vault::vault_atmosphere();
    a.background = 0x0a0808;
    a.tinted(sin_color)
}
