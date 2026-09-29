//! The controller page, from CONTROLLER on the title or the pause menu: it
//! lies over whatever was on screen (drawn by `render::hud::controls`) until
//! B, A, Esc, Start or a click puts it away.

use super::{Game, MenuItem};
use crate::audio::Sfx;
use crate::input::Action;

impl Game {
    pub(super) fn open_controls(&mut self) {
        self.show_controls = true;
        self.audio.play(Sfx::Whoosh);
        self.input.consume(Action::Confirm);
        self.input.consume(Action::Jump);
        self.input.consume(Action::Tap);
    }

    pub(super) fn update_controls(&mut self) {
        let back = [Action::Confirm, Action::Jump, Action::Bomb, Action::Pause, Action::Tap];
        if !self.input.was_pressed(&back) && self.input.click.is_none() {
            return;
        }
        for a in back {
            self.input.consume(a);
        }
        self.input.consume(Action::Attack);
        self.show_controls = false;
        self.audio.play(Sfx::Nodule);
        // Back on the menu, CONTROLLER stays highlighted.
        if let Some(i) = self.menu_items().iter().position(|&m| m == MenuItem::Controls) {
            self.menu_index = i;
        }
    }
}
