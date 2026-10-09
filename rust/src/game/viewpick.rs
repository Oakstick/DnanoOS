//! Choosing the camera just before play begins: from above (top-down) or over his shoulder
//! (follow), each shown in a picture of the Garden, and kept for the next time. C, or R3,
//! switches it at any time, and so does the pause menu's button.

use super::{Game, MenuItem, Screen};
use crate::input::Action;
use crate::platform;
use crate::render::view::CameraMode;

/// The settings key that remembers the camera chosen.
const KEY: &str = "vh.camera";

/// The camera the player chose last (follow, until they choose; tests keep to it).
pub(super) fn saved_camera() -> CameraMode {
    match platform::load(KEY).as_deref().map(str::trim) {
        Some("top") if !cfg!(test) => CameraMode::Top,
        _ => CameraMode::Third,
    }
}

/// The choice's button for each camera, in the order the cards stand.
pub const VIEWS: [(MenuItem, CameraMode); 2] = [(MenuItem::ViewTop, CameraMode::Top), (MenuItem::ViewFollow, CameraMode::Third)];

impl Game {
    /// Sets the camera, and remembers it.
    pub(super) fn choose_camera(&mut self, mode: CameraMode) {
        self.view.camera_mode = mode;
        platform::save(KEY, if mode == CameraMode::Top { "top" } else { "follow" });
    }

    /// The other camera (C, R3, or the pause menu's button).
    pub(super) fn switch_camera(&mut self) {
        let other = if self.view.camera_mode == CameraMode::Third { CameraMode::Top } else { CameraMode::Third };
        self.choose_camera(other);
    }

    /// The level is ready: the camera is chosen first, the one in use highlighted, and then comes
    /// the tour (if `tour`) or the countdown.
    pub(super) fn ask_view(&mut self, tour: bool) {
        self.view_then_tour = tour;
        self.view_before = self.view.camera_mode;
        self.enter(Screen::View);
        self.menu_index = VIEWS.iter().position(|v| v.1 == self.view.camera_mode).unwrap_or(1);
    }

    /// The choice: the world behind the cards is seen as the highlighted one shows it.
    pub(super) fn update_view_choice(&mut self) {
        // Back (Esc, B) closes the choice and keeps the camera that was in use.
        if self.input.was_pressed(&[Action::Back]) {
            self.input.consume(Action::Back);
            self.input.consume(Action::Pause);
            self.view.camera_mode = self.view_before;
            self.go_on_after_view();
            return;
        }
        let choice = self.menu_choice();
        self.view.camera_mode = VIEWS[self.menu_index.min(VIEWS.len() - 1)].1;
        let Some(item) = choice else { return };
        let mode = VIEWS.iter().find(|v| v.0 == item).map(|v| v.1).unwrap_or(CameraMode::Third);
        self.choose_camera(mode);
        self.go_on_after_view();
    }

    /// The camera is settled: on to the tour (if owed) or the countdown.
    fn go_on_after_view(&mut self) {
        self.input.consume(Action::Attack);
        self.input.consume(Action::Confirm);
        self.input.consume(Action::Jump);
        if self.view_then_tour {
            self.begin_tour();
        } else {
            self.enter(Screen::Ready);
        }
    }
}
