//! The character screen, after Play on the title: Mehmed, Ragnar and
//! Miyamoto stand side by side in a cavern of lava and crystal. Left and
//! right (or the pointer) pick one, Enter or a click begins with him, Esc
//! goes back to the title. Only Ragnar can be chosen for now; picking one of
//! the others is refused with a buzz and a shake of his name plate.

use super::{Game, Screen};
use crate::audio::Sfx;
use crate::input::Action;
use crate::logic::heroes::{HeroKind, ROSTER};
use crate::logic::types::Dir;
use crate::render::hud;

/// Seconds a refused pick shakes the locked hero's name plate.
pub const DENIED_SECONDS: f64 = 0.45;

impl Game {
    /// Opens the character screen with the current hero highlighted.
    pub(super) fn open_select(&mut self) {
        self.enter(Screen::Select);
        self.select_index = self.hero_kind.slot();
        self.select_age = 0.0;
        self.select_denied = 0.0;
    }

    /// The hero under the highlight.
    pub fn select_hero(&self) -> HeroKind {
        ROSTER[self.select_index.min(ROSTER.len() - 1)]
    }

    pub(super) fn update_select(&mut self, dt: f64) {
        self.select_age += dt;
        self.select_denied = (self.select_denied - dt).max(0.0);
        // Esc goes back; a controller's Menu button (Pause and Confirm at once) begins instead.
        if self.input.was_pressed(&[Action::Pause]) && !self.input.was_pressed(&[Action::Confirm]) {
            self.enter(Screen::Title);
            return;
        }
        let n = ROSTER.len();
        let before = self.select_index;
        match self.input.nav {
            Some(Dir::Left) => self.select_index = (self.select_index + n - 1) % n,
            Some(Dir::Right) => self.select_index = (self.select_index + 1) % n,
            _ => {}
        }
        // The pointer highlights the hero it rests on; the buttons begin or go back.
        let layout = hud::select_layout(self.viewport.0, self.viewport.1);
        if self.input.cursor_moved && !self.input.mouse_captured {
            if let Some(i) = self.input.cursor.and_then(|p| layout.hero_at(p)) {
                self.select_index = i;
            }
        }
        let mut begin = self.input.was_pressed(&[Action::Confirm, Action::Jump]);
        if let Some(p) = self.input.click {
            if layout.back.contains(p) {
                self.enter(Screen::Title);
                return;
            }
            if layout.begin.contains(p) {
                begin = true;
            } else if let Some(i) = layout.hero_at(p) {
                // A click on the highlighted hero begins with him; on another, it highlights him.
                if i == self.select_index {
                    begin = true;
                } else {
                    self.select_index = i;
                }
            }
        }
        if self.select_index != before {
            self.select_age = 0.0;
            self.select_denied = 0.0;
            self.audio.play(Sfx::Whoosh);
        }
        if begin {
            let hero = self.select_hero();
            if hero.unlocked() {
                self.hero_kind = hero;
                self.audio.unlock();
                self.audio.play(Sfx::Start);
                self.start_game();
                // The click that begins is not also a swing of the axe.
                self.input.consume(Action::Attack);
            } else {
                self.select_denied = DENIED_SECONDS;
                self.audio.play(Sfx::Denied);
            }
        }
    }
}
