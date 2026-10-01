//! The controls page, from CONTROLLER on the title or the pause menu: it lies
//! over whatever was on screen (drawn by `render::hud::controls`) until B, A,
//! Esc, Start or a click puts it away. It shows the controller or the
//! keyboard and mouse: left and right, the bumpers' tab keys, Tab or a click
//! on their tabs switch between the two (it opens on the controller when one
//! is plugged in).

use super::{Game, MenuItem};
use crate::audio::Sfx;
use crate::input::Action;
use crate::logic::types::Dir;
use crate::render::hud;

impl Game {
    pub fn open_controls(&mut self) {
        self.show_controls = true;
        self.controls_keyboard = !self.input.pad_connected;
        self.audio.play(Sfx::Whoosh);
        self.input.consume(Action::Confirm);
        self.input.consume(Action::Jump);
        self.input.consume(Action::Tap);
    }

    pub(super) fn update_controls(&mut self) {
        // Controller or keyboard: left and right, LT and RT, Tab or Y, or a click on a tab.
        let tabs = hud::controls_tab_rects(self.viewport.0, self.viewport.1);
        let on_tab = |p: (f32, f32)| tabs.iter().position(|r| p.0 >= r[0] && p.0 <= r[0] + r[2] && p.1 >= r[1] && p.1 <= r[1] + r[3]);
        let mut want = None;
        if matches!(self.input.nav, Some(Dir::Left) | Some(Dir::Right))
            || self.input.was_pressed(&[Action::TabPrev, Action::TabNext, Action::NextPotion])
        {
            want = Some(!self.controls_keyboard);
        }
        if let Some(k) = self.input.click.and_then(on_tab) {
            want = Some(k == 1);
            self.input.click = None;
            self.input.consume(Action::Tap);
            self.input.consume(Action::Attack);
        }
        if let Some(keyboard) = want {
            if keyboard != self.controls_keyboard {
                self.controls_keyboard = keyboard;
                self.audio.play(Sfx::Nodule);
            }
            self.input.consume(Action::TabNext);
            self.input.consume(Action::Attack);
            return;
        }
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
