//! The map as the player has it (UI.md §7): the minimap's settings (its zoom, which L3 steps, and
//! whether it turns with the camera, `vh.minimap_turn`), and the full map, a page over the pause
//! that the map action opens from play or the pause menu (View, the D-pad's up, M or Tab): the
//! explored level large, panned with the left stick, the keys or a drag of the mouse, zoomed with
//! the wheel, the right stick or the bumpers (Q and E), and put away by the map action or Back.
//! (The developer's list of stops is `map`, on F12.)
//!
//! Presentation only: nothing here is read by the game.

use super::{Game, MenuItem, Screen};
use crate::audio::Sfx;
use crate::input::Action;
use crate::platform;
use crate::render::hud;

/// The settings' keys: the minimap turning with the camera ("1") or north up, and its zoom.
pub const TURN_KEY: &str = "vh.minimap_turn";
pub const ZOOM_KEY: &str = "vh.minimap_zoom";
/// The minimap's zooms: near, mid and far.
pub const MINIMAP_ZOOMS: usize = 3;
/// The full map's zoom steps: a notch of the wheel, a press of a bumper.
const WHEEL_STEP: f32 = 1.25;
const TAB_STEP: f32 = 1.5;
/// The right stick pushed all the way zooms this many times over in a second.
const STICK_ZOOM: f32 = 2.5;
/// The left stick (or the keys) pans this much of the map's width in a second.
const PAN_RATE: f32 = 0.8;

/// The minimap's settings and the full map, while it is open.
#[derive(Clone, Debug, Default)]
pub struct MapUi {
    /// The minimap's zoom: 0 near, 1 mid, 2 far.
    pub zoom: usize,
    /// The minimap turns with the camera (the camera's heading up) rather than keeping north up.
    pub turn: bool,
    pub full: Option<FullMap>,
}

/// The full map while it is open.
#[derive(Clone, Debug, PartialEq)]
pub struct FullMap {
    /// Opened from play (putting it away goes back to play) or from the pause menu.
    pub from_play: bool,
    /// The point of the level at the middle of the map's panel, in tiles (their centres whole).
    pub centre: (f32, f32),
    /// 1: the whole level fits the panel; more is closer.
    pub zoom: f32,
    /// Seconds since it opened (it fades in), and the game's clock when it did.
    pub age: f32,
    opened: f64,
    /// Where the pointer was while the left button drags the map.
    drag: Option<(f32, f32)>,
}

impl MapUi {
    /// As the player left them (tests keep to the defaults: north up, the near zoom).
    pub fn load() -> MapUi {
        if cfg!(test) {
            return MapUi::default();
        }
        MapUi {
            zoom: platform::load(ZOOM_KEY).and_then(|v| v.parse().ok()).unwrap_or(0).min(MINIMAP_ZOOMS - 1),
            turn: platform::load(TURN_KEY).is_some_and(|v| v == "1"),
            full: None,
        }
    }
}

impl Game {
    /// The minimap turning with the camera, or north up; saved (the Settings page's toggle).
    pub fn set_minimap_turn(&mut self, on: bool) {
        self.map_ui.turn = on;
        platform::save(TURN_KEY, if on { "1" } else { "0" });
    }

    /// L3: the minimap's next zoom (near, mid, far, and round again); saved.
    pub fn cycle_minimap_zoom(&mut self) {
        self.map_ui.zoom = (self.map_ui.zoom + 1) % MINIMAP_ZOOMS;
        platform::save(ZOOM_KEY, &self.map_ui.zoom.to_string());
        self.audio.play(Sfx::Tick);
    }

    /// The full map is open.
    pub fn full_map_open(&self) -> bool {
        self.map_ui.full.is_some()
    }

    /**
     * Opens the full map, from play (which pauses; the countdown counts as play) or the pause
     * menu: framed on what he has explored, as close as that allows.
     */
    pub fn open_full_map(&mut self) {
        let from_play = matches!(self.screen, Screen::Playing | Screen::Ready);
        if from_play {
            self.enter(Screen::Paused);
        }
        let grid = &self.level.grid;
        let (tw, th) = (grid.width, grid.height);
        let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for y in 0..th {
            for x in 0..tw {
                if self.explore.seen(x, y) {
                    (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
                }
            }
        }
        if x0 > x1 {
            (x0, y0, x1, y1) = (0, 0, tw - 1, th - 1);
        }
        let (bw, bh) = ((x1 - x0 + 5) as f32, (y1 - y0 + 5) as f32);
        let lay = hud::fullmap_layout(self.viewport.0, self.viewport.1);
        let whole = hud::fullmap_fit(&lay, (tw, th));
        let zoom = ((lay.inner[2] / bw).min(lay.inner[3] / bh) / whole).clamp(1.0, hud::fullmap_max_zoom(&lay, whole));
        let centre = ((x0 + x1) as f32 / 2.0, (y0 + y1) as f32 / 2.0);
        self.map_ui.full = Some(FullMap { from_play, centre, zoom, age: 0.0, opened: self.time, drag: None });
        self.clamp_full_map();
        self.audio.play(Sfx::Whoosh);
        for a in [Action::Map, Action::Confirm, Action::Jump, Action::Tap, Action::Back, Action::Pause] {
            self.input.consume(a);
        }
    }

    /// Puts the full map away: back to play if it was opened from play, else to the pause menu
    /// with MAP highlighted.
    pub fn close_full_map(&mut self) {
        let Some(m) = self.map_ui.full.take() else { return };
        self.audio.play(Sfx::Nodule);
        self.input.consume(Action::Pause);
        self.input.consume(Action::Back);
        if m.from_play && self.screen == Screen::Paused {
            self.enter(Screen::Playing);
        } else if let Some(i) = self.menu_items().iter().position(|&m| m == MenuItem::Map) {
            self.menu_index = i;
        }
    }

    /// The full map's input this update: pan, zoom, back to him, or put away.
    pub(super) fn update_full_map(&mut self, dt: f64) {
        // Whatever opened it puts it away (the map action as bound, M or View in a menu), and Back;
        // not the press that opened it, this same update.
        let fresh = self.map_ui.full.as_ref().is_some_and(|m| m.opened == self.time);
        let bound = self.input.bindings.controls(Action::Map).into_iter().any(|c| self.input.went_down(c));
        if !fresh && (bound || self.input.was_pressed(&[Action::Map, Action::Back, Action::Pause, Action::Guide])) {
            self.close_full_map();
            return;
        }
        let lay = hud::fullmap_layout(self.viewport.0, self.viewport.1);
        let tiles = (self.level.grid.width, self.level.grid.height);
        let fit = hud::fullmap_fit(&lay, tiles);
        let max = hud::fullmap_max_zoom(&lay, fit);
        let hero = self.hero.pos();
        let (mv, (_, (_, ry))) = (self.input.move_vector(), self.input.sticks());
        let cursor = self.input.cursor;
        let dragging = self.input.is_down(Action::Tap) && !self.input.mouse_captured;
        let (wheel, zin, zout, home) = (
            self.input.wheel_turned(),
            self.input.was_pressed(&[Action::TabNext]),
            self.input.was_pressed(&[Action::TabPrev]),
            self.input.was_pressed(&[Action::Secondary]),
        );
        let Some(m) = self.map_ui.full.as_mut() else { return };
        let dt = dt as f32;
        m.age += dt;
        let pc = (lay.inner[0] + lay.inner[2] / 2.0, lay.inner[1] + lay.inner[3] / 2.0);
        let inside = |p: (f32, f32)| {
            p.0 >= lay.inner[0] && p.0 <= lay.inner[0] + lay.inner[2] && p.1 >= lay.inner[1] && p.1 <= lay.inner[1] + lay.inner[3]
        };
        // Zoom about a point of the screen (the pointer over the map, else the middle).
        let zoom_by = |m: &mut FullMap, k: f32, about: (f32, f32)| {
            let s = fit * m.zoom;
            let at = (m.centre.0 + (about.0 - pc.0) / s, m.centre.1 + (about.1 - pc.1) / s);
            m.zoom = (m.zoom * k).clamp(1.0, max);
            let s = fit * m.zoom;
            m.centre = (at.0 - (about.0 - pc.0) / s, at.1 - (about.1 - pc.1) / s);
        };
        if wheel != 0 {
            let about = cursor.filter(|&p| inside(p)).unwrap_or(pc);
            zoom_by(m, WHEEL_STEP.powi(wheel), about);
        }
        if zin {
            zoom_by(m, TAB_STEP, pc);
        }
        if zout {
            zoom_by(m, 1.0 / TAB_STEP, pc);
        }
        if ry != 0.0 {
            zoom_by(m, (-ry * STICK_ZOOM.ln() * dt).exp(), pc);
        }
        // Pan: the left stick or the keys, at a steady share of the panel a second; a drag.
        let s = fit * m.zoom;
        let rate = lay.inner[2] * PAN_RATE / s * dt;
        m.centre.0 += mv.0 as f32 * rate;
        m.centre.1 += mv.1 as f32 * rate;
        match (dragging, cursor) {
            (true, Some(c)) => {
                if let Some(prev) = m.drag {
                    m.centre.0 -= (c.0 - prev.0) / s;
                    m.centre.1 -= (c.1 - prev.1) / s;
                }
                m.drag = Some(c);
            }
            _ => m.drag = None,
        }
        if home {
            m.centre = (hero.x as f32, hero.y as f32);
        }
        self.clamp_full_map();
        for a in [Action::Attack, Action::Confirm, Action::Jump, Action::Tap, Action::Call] {
            self.input.consume(a);
        }
    }

    /// Keeps the full map's middle on the level and its zoom in bounds.
    fn clamp_full_map(&mut self) {
        let (tw, th) = (self.level.grid.width as f32, self.level.grid.height as f32);
        let lay = hud::fullmap_layout(self.viewport.0, self.viewport.1);
        let fit = hud::fullmap_fit(&lay, (tw as i32, th as i32));
        let max = hud::fullmap_max_zoom(&lay, fit);
        if let Some(m) = self.map_ui.full.as_mut() {
            m.zoom = m.zoom.clamp(1.0, max);
            m.centre = (m.centre.0.clamp(-0.5, tw - 0.5), m.centre.1.clamp(-0.5, th - 0.5));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::tests::playing;
    use crate::game::Stage;
    use crate::input::Control;
    use crate::render::overlay::Overlay;
    use winit::keyboard::KeyCode;

    fn step(g: &mut Game) {
        g.update(crate::app::STEP);
    }

    #[test]
    fn the_map_action_opens_and_closes_the_full_map_from_play_and_the_pause() {
        let mut g = playing();
        // From play: it pauses under the map, and the map action puts it away, back to play.
        g.input.inject(Action::Map);
        step(&mut g);
        assert_eq!((g.full_map_open(), g.screen), (true, Screen::Paused));
        assert!(g.map_ui.full.as_ref().unwrap().from_play);
        assert_eq!(g.input_context(), crate::input::Context::Menu);
        g.input.inject(Action::Map);
        step(&mut g);
        assert_eq!((g.full_map_open(), g.screen), (false, Screen::Playing));
        // From the pause menu's MAP: Back puts it away, back to the menu with MAP highlighted.
        g.input.inject(Action::Pause);
        step(&mut g);
        assert_eq!(g.screen, Screen::Paused);
        g.menu_index = g.menu_items().iter().position(|&m| m == MenuItem::Map).unwrap();
        g.input.inject(Action::Confirm);
        step(&mut g);
        assert!(g.full_map_open() && !g.map_ui.full.as_ref().unwrap().from_play);
        g.input.inject(Action::Back);
        step(&mut g);
        assert_eq!((g.full_map_open(), g.screen), (false, Screen::Paused));
        assert_eq!(g.menu_items()[g.menu_index], MenuItem::Map);
        // And the map action from the pause menu opens it too; Esc puts it away, staying paused.
        g.input.inject(Action::Map);
        step(&mut g);
        assert!(g.full_map_open());
        g.input.key_down(KeyCode::Escape, false);
        step(&mut g);
        g.input.key_up(KeyCode::Escape);
        assert_eq!((g.full_map_open(), g.screen), (false, Screen::Paused));
        // Rebound, whatever opens it puts it away.
        g.enter(Screen::Playing);
        step(&mut g);
        g.input.bindings.bind(Action::Map, Control::Key(KeyCode::KeyN));
        for open in [true, false] {
            g.input.key_down(KeyCode::KeyN, false);
            step(&mut g);
            g.input.key_up(KeyCode::KeyN);
            step(&mut g);
            assert_eq!(g.full_map_open(), open);
        }
    }

    #[test]
    fn the_full_map_pans_and_zooms_and_comes_back_to_him() {
        let mut g = playing();
        g.open_full_map();
        let m = g.map_ui.full.clone().unwrap();
        let hp = g.hero.pos();
        // Framed on what he has seen: he is on it.
        let lay = hud::fullmap_layout(g.viewport.0, g.viewport.1);
        let fit = hud::fullmap_fit(&lay, (g.level.grid.width, g.level.grid.height));
        let s = fit * m.zoom;
        assert!(m.zoom > 1.0, "the garden's start is small: closer than the whole");
        assert!(((hp.x as f32 - m.centre.0) * s).abs() < lay.inner[2] / 2.0 && ((hp.y as f32 - m.centre.1) * s).abs() < lay.inner[3] / 2.0);
        // The wheel zooms in, a bumper out; the zoom stays between the whole and the closest.
        g.input.wheel(1.0);
        step(&mut g);
        let z = g.map_ui.full.as_ref().unwrap().zoom;
        assert!(z > m.zoom);
        for _ in 0..30 {
            g.input.inject(Action::TabPrev);
            step(&mut g);
        }
        assert_eq!(g.map_ui.full.as_ref().unwrap().zoom, 1.0);
        for _ in 0..30 {
            g.input.inject(Action::TabNext);
            step(&mut g);
        }
        assert_eq!(g.map_ui.full.as_ref().unwrap().zoom, hud::fullmap_max_zoom(&lay, fit));
        // Held, the keys pan it (D: east); a drag of the mouse moves it with the pointer.
        let c0 = g.map_ui.full.as_ref().unwrap().centre;
        g.input.key_down(KeyCode::KeyD, false);
        for _ in 0..30 {
            step(&mut g);
        }
        g.input.key_up(KeyCode::KeyD);
        let c1 = g.map_ui.full.as_ref().unwrap().centre;
        assert!(c1.0 > c0.0 && (c1.1 - c0.1).abs() < 1e-4, "{c0:?} to {c1:?}");
        let (x, y) = (lay.inner[0] + lay.inner[2] / 2.0, lay.inner[1] + lay.inner[3] / 2.0);
        g.input.cursor_moved_to(x, y);
        g.input.mouse_button(winit::event::MouseButton::Left, true);
        step(&mut g);
        g.input.cursor_moved_to(x + 100.0, y);
        step(&mut g);
        g.input.mouse_button(winit::event::MouseButton::Left, false);
        step(&mut g);
        let c2 = g.map_ui.full.as_ref().unwrap().centre;
        let s = fit * g.map_ui.full.as_ref().unwrap().zoom;
        assert!((c1.0 - c2.0 - 100.0 / s).abs() < 1e-3, "dragged right, the map follows: {c1:?} to {c2:?}");
        assert!(g.full_map_open(), "a click on the map does not close it");
        // Y (R): back to him. And never off the level.
        g.input.inject(Action::Secondary);
        step(&mut g);
        assert_eq!(g.map_ui.full.as_ref().unwrap().centre, (hp.x as f32, hp.y as f32));
        g.map_ui.full.as_mut().unwrap().centre = (-500.0, 9000.0);
        step(&mut g);
        let c = g.map_ui.full.as_ref().unwrap().centre;
        assert!(c.0 >= -0.5 && c.1 <= g.level.grid.height as f32);
    }

    #[test]
    fn the_developers_map_stays_one_key_away() {
        // F12 from the title, from play and from the pause menu: the list of stops.
        let mut g = Game::new();
        assert!(!g.dev && !g.menu_items().contains(&MenuItem::DevMap));
        g.input.key_down(KeyCode::F12, false);
        step(&mut g);
        g.input.key_up(KeyCode::F12);
        assert!(g.show_map && g.dev);
        g.input.inject(Action::Pause);
        step(&mut g);
        assert!(!g.show_map);
        // Once opened, DEV MAP stands on the title and in the pause menu.
        assert!(g.menu_items().contains(&MenuItem::DevMap));
        let mut g = playing();
        g.input.inject(Action::DevMap);
        step(&mut g);
        assert!(g.show_map && g.screen == Screen::Paused);
        g.input.inject(Action::Pause);
        step(&mut g);
        assert!(!g.show_map && g.screen == Screen::Paused);
        let i = g.menu_items().iter().position(|&m| m == MenuItem::DevMap).expect("on the pause menu");
        assert_eq!(g.menu_index, i, "highlighted, back on the menu");
        g.input.inject(Action::Confirm);
        step(&mut g);
        assert!(g.show_map);
        // F12 over the full map trades it for the stops.
        g.input.inject(Action::Pause);
        step(&mut g);
        g.open_full_map();
        g.input.inject(Action::DevMap);
        step(&mut g);
        assert!(g.show_map && !g.full_map_open());
        // The stops include the full map of the garden and of the vaults.
        let stops = crate::game::map_stops();
        let full: Vec<Stage> = stops.iter().filter(|s| s.full_map).map(|s| s.stage).collect();
        assert_eq!(full, vec![Stage::Dungeon(0), Stage::Realm(crate::logic::sins::Sin::Greed)]);
        g.map_index = stops.iter().position(|s| s.full_map).unwrap();
        g.input.inject(Action::Confirm);
        step(&mut g);
        g.finish_loading();
        assert!(g.full_map_open() && g.screen == Screen::Paused && g.dungeon == 0);
    }

    #[test]
    fn l3_steps_the_minimaps_zoom_and_its_turning_is_a_setting() {
        let mut g = playing();
        assert_eq!((g.map_ui.zoom, g.map_ui.turn), (0, false), "near, north up");
        for want in [1, 2, 0] {
            g.input.inject(Action::MinimapZoom);
            step(&mut g);
            assert_eq!(g.map_ui.zoom, want);
        }
        g.set_minimap_turn(true);
        assert_eq!(platform::load(TURN_KEY).as_deref(), Some("1"));
        g.set_minimap_turn(false);
        assert_eq!(platform::load(TURN_KEY).as_deref(), Some("0"));
    }

    #[test]
    fn the_map_never_changes_the_game() {
        use crate::capture::parse_script;
        use crate::driver::Driver;
        // The expert bot through the vaults, once with the minimap drawn every few frames, its zoom
        // stepped and its turning switched, and once without: the same game.
        let play = |map: bool| {
            let mut g = Game::new();
            g.reseed(1);
            g.tour_pending = false;
            let mut d = Driver::new(parse_script("0.1:realm:4,0.5:bot:2"));
            let mut o = Overlay::new();
            let mut k = 0;
            while d.t < 20.0 {
                if map && k % 6 == 0 {
                    o.begin(1280.0, 720.0);
                    crate::render::hud::draw(&mut o, &g, 60);
                    o.map.upload(|_, _| {});
                    if k % 240 == 0 {
                        g.input.inject(Action::MinimapZoom);
                        g.map_ui.turn = !g.map_ui.turn;
                    }
                }
                d.step(&mut g);
                k += 1;
            }
            let foes: Vec<(i64, i64)> = g.enemies.iter().map(|e| ((e.pos().x * 1e6) as i64, (e.pos().y * 1e6) as i64)).collect();
            (d.deeds(&g), g.hero.pos(), g.hero.hp, g.score.score, g.hoard.gold, foes, g.screen, o.map.uploads)
        };
        let (with, without) = (play(true), play(false));
        assert!(with.7 > 1, "the map was drawn and sent up");
        assert_eq!((with.0, with.1, with.2, with.3, with.4), (without.0, without.1, without.2, without.3, without.4));
        assert_eq!((&with.5, with.6), (&without.5, without.6));
    }
}
