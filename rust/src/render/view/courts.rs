//! The princes' courts' views, chosen by sin: Greed's Counting House (`court_greed`) and the six
//! courts still to be built (`court_pride` ... `court_lust`). Only a court built to its own plan
//! (`Theme::is_princes_court`) gets its view; the others are drawn as the stand-in meadow.

use super::*;

impl WorldView {
    /// Clears every court's view, as a level loads.
    pub(super) fn clear_courts(&mut self) {
        self.court = None;
        self.counting = None;
        self.throne = None;
        self.bell = None;
        self.anvil = None;
        self.clocks = None;
        self.banquet = None;
        self.bower = None;
    }

    /// Builds the court of `sin`'s static scene.
    pub(super) fn build_court(&mut self, sin: Sin, grid: &Grid) {
        self.court = Some(sin);
        match sin {
            Sin::Greed => self.build_counting_house(grid),
            Sin::Pride => self.build_pride_court(grid),
            Sin::Envy => self.build_envy_court(grid),
            Sin::Wrath => self.build_wrath_court(grid),
            Sin::Sloth => self.build_sloth_court(grid),
            Sin::Gluttony => self.build_gluttony_court(grid),
            Sin::Lust => self.build_lust_court(grid),
        }
    }

    /**
     * The portal he came in by, in the court's south wall, fading out of the camera's way as trees
     * do: it stands near four tall, and the top camera, looking north over it, lost him under it
     * two tiles inside the gate (QA.md Q9, `--camera-check court1`: hides him at 16,28).
     */
    pub(super) fn build_court_portal(&mut self, gate: &crate::logic::hub::Gate) {
        let start = self.static_cubes.len();
        self.build_portal_arch(gate);
        let range = (start, self.static_cubes.len() - start);
        let cubes = &self.static_cubes[range.0..range.0 + range.1];
        let shades: Vec<f32> = cubes.iter().map(|i| i.emissive[3]).collect();
        let low = cubes
            .iter()
            .map(|c| {
                let m = Mat4::from_cols_array(&c.m);
                m.w_axis.y - m.y_axis.truncate().length() * 0.5
            })
            .fold(f32::INFINITY, f32::min);
        let base = Vec3::new(gate.tile.x as f32, if low.is_finite() { low } else { 0.0 }, gate.tile.y as f32);
        // (Its crown is fitted to its cubes as the level's trees are: `fit_trees`.)
        self.trees.push(Tree { range, base, radius: 1.0, height: 1.0, fade: 1.0, alive: true, shades: Some(shades) });
    }

    /// The light of the court he is in, if he is in one.
    pub(super) fn court_atmosphere(&self) -> Option<Atmosphere> {
        Some(match self.court? {
            Sin::Greed => court_greed::counting_atmosphere(),
            Sin::Pride => court_pride::pride_atmosphere(),
            Sin::Envy => court_envy::envy_atmosphere(),
            Sin::Wrath => court_wrath::wrath_atmosphere(),
            Sin::Sloth => court_sloth::sloth_atmosphere(),
            Sin::Gluttony => court_gluttony::gluttony_atmosphere(),
            Sin::Lust => court_lust::lust_atmosphere(),
        })
    }

    /// The ground haze of the court he is in (density and falloff), if he is in one.
    pub(super) fn court_haze(&self) -> Option<(f32, f32)> {
        Some(match self.court? {
            Sin::Greed => (0.012, 2.0),
            Sin::Pride => court_pride::HAZE,
            Sin::Envy => court_envy::HAZE,
            Sin::Wrath => court_wrath::HAZE,
            Sin::Sloth => court_sloth::HAZE,
            Sin::Gluttony => court_gluttony::HAZE,
            Sin::Lust => court_lust::HAZE,
        })
    }

    /// The moving parts of the court he is in, each frame.
    pub(super) fn emit_court(&self, s: &SceneState, time: f32, hero: Vec3, f: &mut Frame) {
        let Some(sin) = self.court else { return };
        match sin {
            Sin::Greed => self.emit_counting_house(s, time, hero, f),
            Sin::Pride => self.emit_pride_court(s, time, hero, f),
            Sin::Envy => self.emit_envy_court(s, time, hero, f),
            Sin::Wrath => self.emit_wrath_court(s, time, hero, f),
            Sin::Sloth => self.emit_sloth_court(s, time, hero, f),
            Sin::Gluttony => self.emit_gluttony_court(s, time, hero, f),
            Sin::Lust => self.emit_lust_court(s, time, hero, f),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::game::{Game, Stage};
    use crate::logic::sins::SINS;
    use crate::logic::types::P;
    use crate::render::view::CameraMode;

    /// In every court the portal he came in by fades while he stands two tiles inside it under the
    /// top camera, which looks north over it (QA.md Q9: `--camera-check court1` lost him there).
    #[test]
    fn the_court_portal_fades_out_of_the_top_cameras_way() {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        for sin in SINS {
            let mut g = Game::new();
            g.map_jump(Stage::Court(sin));
            let gate = g.gate.expect("a court has its gate");
            g.view.camera_mode = CameraMode::Top;
            g.hero.place(P::new(gate.x, gate.y - 2));
            g.view.snap_camera();
            for _ in 0..60 {
                g.update(crate::app::STEP);
            }
            let arch = g.view.trees.iter().find(|t| (t.base.x - gate.x as f32).abs() < 0.01 && (t.base.z - gate.y as f32).abs() < 0.01);
            let arch = arch.unwrap_or_else(|| panic!("{sin:?}: the portal fades as a tree"));
            assert!(arch.fade < 0.5, "{sin:?}: the portal fades ({})", arch.fade);
        }
    }
}
