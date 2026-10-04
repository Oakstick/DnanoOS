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
