//! The settings page (UI.md §8), from SETTINGS on the title or the pause menu: five tabs of rows,
//! each a toggle, a slider, a choice or a binding, changed with the arrows, the D-pad or the
//! stick, LB and RB (Q and E) between the tabs, A or Enter to choose, and B or Esc to go back; the
//! mouse hovers, clicks and drags as it would expect to. Every change is applied at once and saved.
//!
//! The Controls tab rebinds play's actions, one row each, on the keyboard and mouse or on the
//! controller (a row of its own chooses which): a slot is chosen and the next control pressed is
//! bound into it (Esc or the Menu button cancels, and so does waiting five seconds). A control
//! already on another action asks first: swap the two, take it from the other, or cancel. The
//! fixed keys (F1 to F11, 0) are refused. `{Secondary}` clears a slot, the slot after the second
//! puts the row back to its defaults, and the last row resets them all (asked twice).
//!
//! Nothing here touches the simulation: the settings change what is drawn and heard and how the
//! controls are read, and the page only opens over the title and the pause.

use super::{Game, MenuItem};
use crate::audio::{Sfx, Volumes};
use crate::input::bindings::PLAY;
use crate::input::{Action, Control, PadButton, Rebound, Resolve, RumbleLevel};
use crate::logic::types::Dir;
use crate::render::hud;
use crate::render::view::CameraMode;
use crate::settings::{self, ColourBlind, Range, TextSize};

/// Seconds the page waits for a control to bind before it gives up.
pub const LISTEN_SECONDS: f64 = 5.0;
/// Seconds a note under the list stays.
pub const NOTE_SECONDS: f64 = 3.0;

/// The page's tabs.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Tab {
    #[default]
    Gameplay,
    Controls,
    Audio,
    Display,
    Accessibility,
}

impl Tab {
    pub const ALL: [Tab; 5] = [Tab::Gameplay, Tab::Controls, Tab::Audio, Tab::Display, Tab::Accessibility];

    pub fn name(self) -> &'static str {
        match self {
            Tab::Gameplay => "GAMEPLAY",
            Tab::Controls => "CONTROLS",
            Tab::Audio => "AUDIO",
            Tab::Display => "DISPLAY",
            Tab::Accessibility => "ACCESSIBILITY",
        }
    }

    fn index(self) -> usize {
        Tab::ALL.iter().position(|&t| t == self).unwrap_or(0)
    }
}

/// One row of a tab.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Row {
    Camera,
    FreeMove,
    SlowMotion,
    Shake,
    MinimapTurn,
    ShieldMode,
    Sensitivity,
    InvertMouse,
    InvertStick,
    MoveDeadzone,
    LookDeadzone,
    Rumble,
    /// Which side the binding rows show: the keyboard and mouse's, or the controller's.
    Device,
    Bind(Action),
    ResetAll,
    Master,
    Music,
    Effects,
    Voices,
    Mute,
    Fullscreen,
    Quality,
    UiScale,
    TextSize,
    ColourBlind,
    Backing,
    ReduceFlashing,
}

/// How a row is changed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Toggle,
    Slider(Range),
    /// One of a few, stepped through.
    Choice,
    /// Two slots and a reset.
    Binding,
    /// Does something when chosen.
    Button,
}

/// The rows of a tab, top to bottom.
pub fn rows(tab: Tab) -> Vec<Row> {
    match tab {
        Tab::Gameplay => vec![Row::Camera, Row::FreeMove, Row::SlowMotion, Row::Shake, Row::MinimapTurn, Row::ShieldMode],
        Tab::Controls => {
            let mut v =
                vec![Row::Sensitivity, Row::InvertMouse, Row::InvertStick, Row::MoveDeadzone, Row::LookDeadzone, Row::Rumble, Row::Device];
            v.extend(PLAY.iter().map(|&a| Row::Bind(a)));
            v.push(Row::ResetAll);
            v
        }
        Tab::Audio => vec![Row::Master, Row::Music, Row::Effects, Row::Voices, Row::Mute],
        // (On the web the full screen is the browser's, and the quality is the web's own preset.)
        Tab::Display if cfg!(target_arch = "wasm32") => vec![Row::UiScale, Row::TextSize],
        Tab::Display => vec![Row::Fullscreen, Row::Quality, Row::UiScale, Row::TextSize],
        Tab::Accessibility => vec![Row::ColourBlind, Row::Backing, Row::ReduceFlashing, Row::Shake],
    }
}

impl Row {
    pub fn kind(self) -> Kind {
        match self {
            Row::FreeMove | Row::SlowMotion | Row::MinimapTurn | Row::InvertMouse | Row::InvertStick => Kind::Toggle,
            Row::Mute | Row::Fullscreen | Row::ReduceFlashing => Kind::Toggle,
            Row::Sensitivity => Kind::Slider(settings::SENSITIVITY),
            Row::MoveDeadzone => Kind::Slider(settings::MOVE_DEADZONE),
            Row::LookDeadzone => Kind::Slider(settings::LOOK_DEADZONE),
            Row::Master | Row::Music | Row::Effects | Row::Voices => Kind::Slider(settings::VOLUME),
            Row::UiScale => Kind::Slider(settings::UI_SCALE),
            Row::Backing => Kind::Slider(settings::BACKING),
            Row::Shake => Kind::Slider(Range { min: 0, max: 100, step: 25 }),
            Row::Bind(_) => Kind::Binding,
            Row::ResetAll => Kind::Button,
            Row::Camera | Row::ShieldMode | Row::Rumble | Row::Device | Row::Quality | Row::TextSize | Row::ColourBlind => Kind::Choice,
        }
    }

    pub fn label(self) -> String {
        match self {
            Row::Camera => "Camera".into(),
            Row::FreeMove => "Free movement".into(),
            Row::SlowMotion => "Slow motion on kills".into(),
            Row::Shake => "Screen shake".into(),
            Row::MinimapTurn => "Minimap turns with the camera".into(),
            Row::ShieldMode => "Shield".into(),
            Row::Sensitivity => "Mouse sensitivity".into(),
            Row::InvertMouse => "Invert mouse up and down".into(),
            Row::InvertStick => "Invert right stick up and down".into(),
            Row::MoveDeadzone => "Left stick deadzone".into(),
            Row::LookDeadzone => "Right stick deadzone".into(),
            Row::Rumble => "Rumble".into(),
            Row::Device => "Bindings for".into(),
            Row::Bind(a) => crate::input::action_name(a).into(),
            Row::ResetAll => "Reset every binding".into(),
            Row::Master => "Master volume".into(),
            Row::Music => "Music".into(),
            Row::Effects => "Effects".into(),
            Row::Voices => "Voices".into(),
            Row::Mute => "Mute".into(),
            Row::Fullscreen => "Full screen".into(),
            Row::Quality => "Quality".into(),
            Row::UiScale => "UI scale".into(),
            Row::TextSize => "Text size".into(),
            Row::ColourBlind => "Colour-blind mode".into(),
            Row::Backing => "Subtitle and dialogue backing".into(),
            Row::ReduceFlashing => "Reduce flashing".into(),
        }
    }

    /// A line on what it does, under the list while it is highlighted.
    pub fn help(self) -> &'static str {
        match self {
            Row::Camera => "Over his shoulder, or from above. {Camera} switches it in play.",
            Row::FreeMove => "Run any way at all, or keep to the corridors' grid.",
            Row::SlowMotion => "Time slows for a beat on the kills that end something.",
            Row::Shake => "How hard the screen shakes when blows land.",
            Row::MinimapTurn => "The minimap turns as the camera does; off, north is always up.",
            Row::ShieldMode => "Hold {Block} to keep the shield up, or press once to raise it and again to lower it.",
            Row::Sensitivity => "How far the camera turns for the mouse's movement.",
            Row::InvertMouse | Row::InvertStick => "Up looks down, and down looks up.",
            Row::MoveDeadzone => "How far the left stick must be pushed before he runs.",
            Row::LookDeadzone => "How far the right stick must be pushed before the camera turns.",
            Row::Rumble => "How strongly the controller answers blows, parries and quakes.",
            Row::Device => "Which bindings the rows below change.",
            Row::Bind(_) => "{Confirm} then press a control to bind it.  {Secondary} clears a slot.  The last column resets the row.",
            Row::ResetAll => "Every action back to its first binding, on both the keyboard and the controller.",
            Row::Master => "Everything you hear.",
            Row::Music => "The music, and the drone of danger under it.",
            Row::Effects => "Blows, steps, the world.",
            Row::Voices => "Cries, and the words as they are spoken.",
            Row::Mute => "Silence, keeping the volumes as they are (Ctrl + M too).",
            Row::Fullscreen => "The whole screen, or a window (F11 or Alt + Enter too).",
            Row::Quality => "High for the full look; Low gives up distant shadow detail first.",
            Row::UiScale => "The size of everything on screen, from 80% to 150%.",
            Row::TextSize => "The size of the old man's words, the guide and the tips.",
            Row::ColourBlind => "Tells, markers and health in colours told apart: red and green become orange and blue.",
            Row::Backing => "How dark the band behind subtitles and dialogue is.",
            Row::ReduceFlashing => "Flashes and white-outs dimmed, nothing strobing.",
        }
    }
}

/// Waiting for a control to bind.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Listen {
    pub action: Action,
    pub slot: usize,
    /// The game's clock when it began.
    pub since: f64,
    /// Begun this update: the presses of this update are not its answer.
    pub fresh: bool,
}

/// A control asked for that another action has: how to settle it.
#[derive(Clone, PartialEq, Debug)]
pub struct Conflict {
    pub action: Action,
    pub slot: usize,
    pub control: Control,
    pub others: Vec<Action>,
    /// 0 swap, 1 take it from them, 2 cancel.
    pub choice: usize,
}

/// The conflict's three answers, in order.
pub const CONFLICT_CHOICES: [&str; 3] = ["SWAP", "TAKE IT", "CANCEL"];

/// The page's state.
#[derive(Clone, Debug, Default)]
pub struct SettingsPage {
    pub open: bool,
    pub tab: Tab,
    /// The highlighted row, and on a binding row the highlighted column (0 and 1 the slots, 2 the
    /// reset).
    pub row: usize,
    pub column: usize,
    /// The first row shown (a long tab scrolls).
    pub scroll: usize,
    /// The binding rows show the controller's side.
    pub pad: bool,
    pub listen: Option<Listen>,
    pub conflict: Option<Conflict>,
    /// A line under the list, and when it was set.
    pub note: Option<(String, f64)>,
    /// Reset every binding was chosen once: once more does it.
    pub reset_armed: bool,
    /// Where the page was opened from: the pause menu (else the title).
    pub from_pause: bool,
}

impl Game {
    /// The settings into everything they change, at once (the input's reading, the mixer, the
    /// HUD's scale and palette, the view's tells).
    pub fn apply_settings(&mut self) {
        self.input.apply(&self.settings);
        self.audio.set_volumes(Volumes::of(&self.settings));
        hud::set_prefs(self.settings.ui());
        self.view.colour_blind = self.settings.colour_blind;
        self.view.reduce_flashing = self.settings.reduce_flashing;
    }

    /// The settings changed: applied, and saved.
    fn settings_changed(&mut self) {
        self.apply_settings();
        self.settings.save();
    }

    pub fn open_settings(&mut self) {
        let from_pause = self.screen == super::Screen::Paused;
        self.settings_page = SettingsPage { open: true, pad: self.input.last_device().is_pad(), from_pause, ..Default::default() };
        self.audio.play(Sfx::Whoosh);
        for a in [Action::Confirm, Action::Jump, Action::Tap, Action::Attack] {
            self.input.consume(a);
        }
        self.input.click = None;
    }

    /// Opens on a tab (captures, and the Map's stop).
    pub fn open_settings_on(&mut self, tab: Tab) {
        self.open_settings();
        self.settings_page.tab = tab;
    }

    fn close_settings(&mut self) {
        self.settings_page.open = false;
        self.audio.play(Sfx::Nodule);
        if let Some(i) = self.menu_items().iter().position(|&m| m == MenuItem::Settings) {
            self.menu_index = i;
        }
    }

    /// The highlighted row.
    pub fn settings_row(&self) -> Option<Row> {
        rows(self.settings_page.tab).get(self.settings_page.row).copied()
    }

    /// A row's value, as the page writes it (a binding row's: its slots, by `slot_names`).
    pub fn settings_value(&self, row: Row) -> String {
        let s = &self.settings;
        let on = |b: bool| if b { "On" } else { "Off" }.to_string();
        match row {
            Row::Camera => if self.view.camera_mode == CameraMode::Top { "Top-down" } else { "Follow" }.into(),
            Row::FreeMove => on(self.hero.free_move),
            Row::SlowMotion => on(self.feel.slow_motion),
            Row::Shake if self.feel.shake == 0 => "Off".into(),
            Row::Shake => format!("{}%", self.feel.shake),
            Row::MinimapTurn => on(s.minimap_turn),
            Row::ShieldMode => if s.shield_toggle { "Toggle" } else { "Hold" }.into(),
            Row::Sensitivity => format!("{}%", s.mouse_sensitivity),
            Row::InvertMouse => on(s.invert_mouse_y),
            Row::InvertStick => on(s.invert_stick_y),
            Row::MoveDeadzone => format!("{}%", s.move_deadzone),
            Row::LookDeadzone => format!("{}%", s.look_deadzone),
            Row::Rumble => match self.input.rumble() {
                RumbleLevel::Off => "Off",
                RumbleLevel::Low => "Low",
                RumbleLevel::High => "High",
            }
            .into(),
            Row::Device => if self.settings_page.pad { "Controller" } else { "Keyboard and mouse" }.into(),
            Row::Bind(_) | Row::ResetAll => String::new(),
            Row::Master => format!("{}%", s.master),
            Row::Music => format!("{}%", s.music),
            Row::Effects => format!("{}%", s.effects),
            Row::Voices => format!("{}%", s.voices),
            Row::Mute => on(self.audio.muted),
            Row::Fullscreen => on(self.fullscreen),
            Row::Quality => {
                let mut n = s.preset.to_string();
                n[..1].make_ascii_uppercase();
                n
            }
            Row::UiScale => format!("{}%", s.ui_scale),
            Row::TextSize => s.text_size.label().into(),
            Row::ColourBlind => s.colour_blind.label().into(),
            Row::Backing => format!("{}%", s.backing),
            Row::ReduceFlashing => on(s.reduce_flashing),
        }
    }

    /// A slider's value, for its fill.
    pub fn settings_slider(&self, row: Row) -> Option<(Range, u8)> {
        let Kind::Slider(r) = row.kind() else { return None };
        let s = &self.settings;
        let v = match row {
            Row::Sensitivity => s.mouse_sensitivity,
            Row::MoveDeadzone => s.move_deadzone,
            Row::LookDeadzone => s.look_deadzone,
            Row::Master => s.master,
            Row::Music => s.music,
            Row::Effects => s.effects,
            Row::Voices => s.voices,
            Row::UiScale => s.ui_scale,
            Row::Backing => s.backing,
            Row::Shake => self.feel.shake,
            _ => return None,
        };
        Some((r, v))
    }

    /// Sets a slider to `v` (in its range).
    fn set_slider(&mut self, row: Row, v: u8) {
        let s = &mut self.settings;
        match row {
            Row::Sensitivity => s.mouse_sensitivity = v,
            Row::MoveDeadzone => s.move_deadzone = v,
            Row::LookDeadzone => s.look_deadzone = v,
            Row::Master => s.master = v,
            Row::Music => s.music = v,
            Row::Effects => s.effects = v,
            Row::Voices => s.voices = v,
            Row::UiScale => s.ui_scale = v,
            Row::Backing => s.backing = v,
            Row::Shake => {
                self.set_shake(v);
                return;
            }
            _ => return,
        }
        self.settings_changed();
    }

    /// Changes a row by a step either way (left and right; a choice goes round).
    pub fn settings_adjust(&mut self, row: Row, by: i32) {
        if let Some((r, v)) = self.settings_slider(row) {
            let to = r.stepped(v, by);
            if to != v {
                self.set_slider(row, to);
                self.audio.play(Sfx::Nodule);
            }
            return;
        }
        let step = |n: usize, k: usize| ((k as i32 + by).rem_euclid(n as i32)) as usize;
        match row {
            Row::Rumble => {
                let k = RumbleLevel::ALL.iter().position(|&l| l == self.input.rumble()).unwrap_or(2);
                self.input.set_rumble(RumbleLevel::ALL[step(3, k)]);
            }
            Row::Device => self.settings_page.pad = !self.settings_page.pad,
            Row::Quality => {
                let all = crate::render::preset::ALL;
                let k = all.iter().position(|p| p.name == self.settings.preset).unwrap_or(0);
                self.settings.preset = all[step(all.len(), k)].name;
                self.settings_changed();
            }
            Row::TextSize => {
                let k = TextSize::ALL.iter().position(|&t| t == self.settings.text_size).unwrap_or(1);
                self.settings.text_size = TextSize::ALL[step(3, k)];
                self.settings_changed();
            }
            Row::ColourBlind => {
                let k = ColourBlind::ALL.iter().position(|&c| c == self.settings.colour_blind).unwrap_or(0);
                self.settings.colour_blind = ColourBlind::ALL[step(4, k)];
                self.settings_changed();
            }
            // Two ways, so either step is the other.
            _ => self.settings_toggle(row),
        }
        self.audio.play(Sfx::Nodule);
    }

    /// A toggle (or a two-way choice) turned over.
    fn settings_toggle(&mut self, row: Row) {
        match row {
            Row::Camera => self.switch_camera(),
            Row::FreeMove => {
                self.hero.free_move = !self.hero.free_move;
                if !cfg!(test) {
                    crate::platform::save("vh.free_move", if self.hero.free_move { "1" } else { "0" });
                }
            }
            Row::SlowMotion => self.toggle_slow_motion(),
            Row::MinimapTurn => {
                self.settings.minimap_turn = !self.settings.minimap_turn;
                self.settings_changed();
            }
            Row::ShieldMode => {
                self.settings.shield_toggle = !self.settings.shield_toggle;
                self.settings_changed();
            }
            Row::InvertMouse => {
                self.settings.invert_mouse_y = !self.settings.invert_mouse_y;
                self.settings_changed();
            }
            Row::InvertStick => {
                self.settings.invert_stick_y = !self.settings.invert_stick_y;
                self.settings_changed();
            }
            Row::Mute => {
                self.audio.unlock();
                self.audio.toggle_mute();
            }
            Row::Fullscreen => self.want_fullscreen_toggle = true,
            Row::ReduceFlashing => {
                self.settings.reduce_flashing = !self.settings.reduce_flashing;
                self.settings_changed();
            }
            _ => {}
        }
    }

    /// A row chosen (A, Enter, a click on it).
    fn settings_activate(&mut self, row: Row) {
        match row.kind() {
            Kind::Toggle => {
                self.settings_toggle(row);
                self.audio.play(Sfx::Nodule);
            }
            Kind::Choice | Kind::Slider(_) => self.settings_adjust(row, 1),
            Kind::Binding => {
                let Row::Bind(action) = row else { return };
                if self.settings_page.column >= 2 {
                    self.input.bindings.reset_action(action);
                    self.save_bindings();
                    self.settings_note(format!("{}: as it was at first", crate::input::action_name(action)));
                    self.audio.play(Sfx::Nodule);
                } else {
                    self.listen(action, self.settings_page.column);
                }
            }
            Kind::Button => {
                if self.settings_page.reset_armed {
                    self.input.bindings.reset();
                    self.save_bindings();
                    self.settings_page.reset_armed = false;
                    self.settings_note("Every binding is as it was at first".into());
                    self.audio.play(Sfx::Anvil);
                } else {
                    self.settings_page.reset_armed = true;
                    self.settings_note("Choose it again to reset every binding".into());
                    self.audio.play(Sfx::Nodule);
                }
            }
        }
    }

    /// Waits for the next control pressed, to bind into `slot` of `action`.
    pub fn listen(&mut self, action: Action, slot: usize) {
        self.settings_page.listen = Some(Listen { action, slot, since: self.time, fresh: true });
        self.settings_page.note = None;
        self.audio.play(Sfx::Tick);
    }

    fn settings_note(&mut self, text: String) {
        self.settings_page.note = Some((text, self.time));
    }

    fn save_bindings(&self) {
        self.input.bindings.save();
    }

    /// The shake set to `v` percent (one of its steps), and remembered.
    fn set_shake(&mut self, v: u8) {
        while self.feel.shake != v {
            let before = self.feel.shake;
            self.step_shake();
            if self.feel.shake == before {
                break;
            }
        }
    }

    /// A control pressed while the page listens: bound, refused, or asked about.
    fn hear(&mut self, control: Control) {
        let Some(l) = self.settings_page.listen else { return };
        // Esc and the Menu button cancel.
        if matches!(control, Control::Key(winit::keyboard::KeyCode::Escape) | Control::Pad(PadButton::Menu)) {
            self.settings_page.listen = None;
            self.settings_note("Cancelled: nothing changed".into());
            self.audio.play(Sfx::Nodule);
            return;
        }
        let pad = self.settings_page.pad;
        if control.is_pad() != pad {
            // The other side's control: still listening (a hint, once).
            let side = if pad { "a controller button" } else { "a key or a mouse button" };
            self.settings_note(format!("Press {side} (Esc or Menu cancels)"));
            return;
        }
        let device = if pad { self.names_device() } else { crate::input::Device::KeyboardMouse };
        let name = crate::input::control_name(control, device).into_owned();
        match self.input.bindings.rebind(l.action, pad, l.slot, control, Resolve::Ask) {
            Rebound::Done => {
                self.settings_page.listen = None;
                self.save_bindings();
                self.settings_note(format!("{}: {name}", crate::input::action_name(l.action)));
                self.audio.play(Sfx::Buy);
            }
            Rebound::Conflict(others) => {
                self.settings_page.listen = None;
                self.settings_page.conflict = Some(Conflict { action: l.action, slot: l.slot, control, others, choice: 0 });
                self.audio.play(Sfx::Denied);
            }
            Rebound::Fixed(what) => {
                let what = what.map(|a| format!(" ({})", crate::input::action_name(a))).unwrap_or_default();
                self.settings_note(format!("{name} is kept for the game itself{what}: choose another"));
                self.audio.play(Sfx::Denied);
            }
            Rebound::Resolved(_) | Rebound::WrongSide => {}
        }
    }

    /// The controller's family the page names its buttons by (Xbox's for one we do not know).
    pub fn names_device(&self) -> crate::input::Device {
        match self.input.last_device() {
            crate::input::Device::PlayStation => crate::input::Device::PlayStation,
            _ => crate::input::Device::Xbox,
        }
    }

    /// The conflict answered: 0 swap, 1 take it from the others, 2 cancel.
    fn settle_conflict(&mut self, choice: usize) {
        let Some(c) = self.settings_page.conflict.take() else { return };
        let resolve = match choice {
            0 => Resolve::Swap,
            1 => Resolve::Clear,
            _ => {
                self.settings_note("Cancelled: nothing changed".into());
                self.audio.play(Sfx::Nodule);
                return;
            }
        };
        self.input.bindings.rebind(c.action, self.settings_page.pad, c.slot, c.control, resolve);
        self.save_bindings();
        let others: Vec<&str> = c.others.iter().map(|&a| crate::input::action_name(a)).collect();
        self.settings_note(if choice == 0 {
            format!("Swapped with {}", others.join(", "))
        } else {
            format!("Taken from {}", others.join(", "))
        });
        self.audio.play(Sfx::Buy);
    }

    /// One update of the page.
    pub(super) fn update_settings(&mut self) {
        let (w, h) = self.viewport;
        if self.settings_page.note.as_ref().is_some_and(|(_, at)| self.time - at > NOTE_SECONDS) {
            self.settings_page.note = None;
        }
        // ---- the conflict's question: left and right (or up and down) choose, A answers, B cancels
        if let Some(c) = self.settings_page.conflict.as_mut() {
            let mut answer = None;
            match self.input.nav {
                Some(Dir::Left | Dir::Up) => c.choice = (c.choice + 2) % 3,
                Some(Dir::Right | Dir::Down) => c.choice = (c.choice + 1) % 3,
                _ => {}
            }
            let rects = hud::settings::conflict_rects(w, h);
            let hit = |p: (f32, f32)| rects.iter().position(|r| p.0 >= r[0] && p.0 <= r[0] + r[2] && p.1 >= r[1] && p.1 <= r[1] + r[3]);
            if self.input.cursor_moved && !self.input.mouse_captured {
                if let Some(i) = self.input.cursor.and_then(hit) {
                    c.choice = i;
                }
            }
            if let Some(p) = self.input.click {
                answer = hit(p);
            } else if self.input.was_pressed(&[Action::Confirm, Action::Jump]) {
                answer = Some(c.choice);
            } else if self.input.was_pressed(&[Action::Back, Action::Pause]) {
                answer = Some(2);
            }
            if let Some(k) = answer {
                self.settle_conflict(k);
            }
            self.swallow();
            return;
        }
        // ---- listening for the control to bind
        if let Some(l) = self.settings_page.listen {
            if l.fresh {
                if let Some(l) = self.settings_page.listen.as_mut() {
                    l.fresh = false;
                }
            } else if self.time - l.since > LISTEN_SECONDS {
                self.settings_page.listen = None;
                self.settings_note("Nothing was pressed: nothing changed".into());
            } else {
                let raw: Vec<Control> = self.input.raw_presses().to_vec();
                for c in raw {
                    if self.settings_page.listen.is_none() {
                        break;
                    }
                    self.hear(c);
                }
            }
            self.swallow();
            return;
        }

        // ---- the page itself
        let tab = self.settings_page.tab;
        let list = rows(tab);
        let n = list.len();
        let layout = hud::settings::layout(w, h, n, self.settings_page.scroll);
        let in_rect = |p: (f32, f32), r: &[f32; 4]| p.0 >= r[0] && p.0 <= r[0] + r[2] && p.1 >= r[1] && p.1 <= r[1] + r[3];
        // Back: B, Esc, Menu (or the Back button).
        let back_clicked = self.input.click.is_some_and(|p| in_rect(p, &layout.back));
        if self.input.was_pressed(&[Action::Back, Action::Pause]) || back_clicked {
            self.swallow();
            self.close_settings();
            return;
        }
        // The tabs: LB and RB (Q and E), or a click on one.
        let mut to_tab = None;
        if self.input.was_pressed(&[Action::TabPrev]) {
            to_tab = Some((tab.index() + Tab::ALL.len() - 1) % Tab::ALL.len());
        }
        if self.input.was_pressed(&[Action::TabNext]) {
            to_tab = Some((tab.index() + 1) % Tab::ALL.len());
        }
        if let Some(p) = self.input.click {
            if let Some(k) = layout.tabs.iter().position(|r| in_rect(p, r)) {
                to_tab = Some(k);
            }
        }
        if let Some(k) = to_tab {
            if Tab::ALL[k] != tab {
                let pad = self.settings_page.pad;
                self.settings_page =
                    SettingsPage { open: true, tab: Tab::ALL[k], pad, from_pause: self.settings_page.from_pause, ..Default::default() };
                self.audio.play(Sfx::Nodule);
            }
            self.swallow();
            return;
        }
        // Up and down move the highlight (the wheel too); left and right change it.
        let page = &mut self.settings_page;
        match self.input.nav {
            Some(Dir::Up) => page.row = (page.row + n - 1) % n,
            Some(Dir::Down) => page.row = (page.row + 1) % n,
            _ => {}
        }
        if self.input.nav.is_some() {
            page.reset_armed = page.reset_armed && list.get(page.row) == Some(&Row::ResetAll);
        }
        // The pointer: hovering a row highlights it (and a binding row's slot under it).
        if self.input.cursor_moved && !self.input.mouse_captured {
            if let Some(p) = self.input.cursor {
                if let Some((i, cells)) = layout.rows.iter().find(|(_, r)| in_rect(p, r)).map(|(i, _)| (*i, layout.cells(*i))) {
                    page.row = i;
                    if let Some(k) = cells.iter().position(|r| in_rect(p, r)) {
                        page.column = k;
                    }
                }
            }
        }
        let row = list[page.row.min(n - 1)];
        // Kept in view: a long list scrolls to keep the highlight a row from its edges.
        let shown = hud::settings::visible_rows(w, h).max(1);
        if page.row < page.scroll + 1 {
            page.scroll = page.row.saturating_sub(1);
        } else if page.row + 2 > page.scroll + shown {
            page.scroll = (page.row + 2).saturating_sub(shown);
        }
        page.scroll = page.scroll.min(n.saturating_sub(shown));
        match self.input.nav {
            Some(Dir::Left) | Some(Dir::Right) => {
                let by = if self.input.nav == Some(Dir::Left) { -1 } else { 1 };
                if row.kind() == Kind::Binding {
                    self.settings_page.column = (self.settings_page.column as i32 + by).clamp(0, 2) as usize;
                } else if row.kind() != Kind::Button {
                    self.settings_adjust(row, by);
                }
            }
            _ => {}
        }
        // A slot cleared: Y (R on the keyboard), or Delete... the menus' secondary.
        if row.kind() == Kind::Binding && self.input.was_pressed(&[Action::Secondary]) {
            if let Row::Bind(a) = row {
                let slot = self.settings_page.column.min(1);
                if self.input.bindings.clear_slot(a, self.settings_page.pad, slot).is_some() {
                    self.save_bindings();
                    self.settings_note(format!("{}: slot {} cleared", crate::input::action_name(a), slot + 1));
                    self.audio.play(Sfx::Nodule);
                }
            }
        }
        // A click: on a slider, the value where it landed; on a slot, listening; elsewhere on a row, as A.
        if let Some(p) = self.input.click {
            if let Some(&(i, _)) = layout.rows.iter().find(|(_, r)| in_rect(p, r)) {
                let row = list[i];
                self.settings_page.row = i;
                let cells = layout.cells(i);
                match row.kind() {
                    Kind::Slider(_) if in_rect(p, &cells[0]) || in_rect(p, &cells[1]) => {
                        let track = layout.track(i);
                        let frac = (p.0 - track[0]) / track[2];
                        if let Some((r, v)) = self.settings_slider(row) {
                            let to = r.at(frac);
                            if to != v {
                                self.set_slider(row, to);
                                self.audio.play(Sfx::Nodule);
                            }
                        }
                    }
                    Kind::Slider(_) => {}
                    Kind::Binding => {
                        if let Some(k) = cells.iter().position(|r| in_rect(p, r)) {
                            self.settings_page.column = k;
                            self.settings_activate(row);
                        }
                    }
                    _ => self.settings_activate(row),
                }
            }
            self.swallow();
            return;
        }
        if self.input.was_pressed(&[Action::Confirm, Action::Jump]) {
            self.settings_activate(row);
        }
        self.swallow();
    }

    /// Nothing the page heard this update goes on to anything else.
    fn swallow(&mut self) {
        for a in [Action::Confirm, Action::Jump, Action::Tap, Action::Attack, Action::Back, Action::Pause, Action::Guide, Action::Map] {
            self.input.consume(a);
        }
        self.input.click = None;
    }

    /// The in-game loading card shown over the level as it stands, held `hold` seconds (the Map's
    /// stop, and the captures' `loading`): as a load that took long enough to need it.
    pub fn show_loading_card(&mut self, hold: f64) {
        self.loading = Some(super::Loading {
            label: self.level.data.name.clone(),
            started: web_time::Instant::now(),
            load_ms: 0.0,
            age: 0.0,
            cold: false,
            hold,
        });
        self.juice.fade = 1.0;
    }

    /// The device used last, remembered as it changes (for the engine's start screen's legend).
    pub(super) fn note_device(&mut self) {
        let Some(d) = self.input.device_used() else { return };
        if Some(d) != self.device_saved {
            self.device_saved = Some(d);
            if !cfg!(test) {
                crate::platform::save(crate::settings::LAST_DEVICE_KEY, crate::settings::device_name(d));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Screen;
    use crate::input::{Device, MouseButton};
    use winit::keyboard::KeyCode;

    /// A save folder of this test's own (the mute and the camera write theirs as they change).
    fn own_saves() {
        crate::platform::use_dir_on_this_thread(std::env::temp_dir().join(format!("vh-test-settings-{}", std::process::id())));
    }

    fn paused() -> Game {
        own_saves();
        let mut g = Game::new();
        g.load_level(0);
        g.enter(Screen::Paused);
        g
    }

    fn step(g: &mut Game) {
        g.update(crate::app::STEP);
    }

    fn key(g: &mut Game, k: KeyCode) {
        g.input.key_down(k, false);
        step(g);
        g.input.key_up(k);
        step(g);
    }

    /// The settings page on `tab`, `row` highlighted.
    fn on(g: &mut Game, tab: Tab, row: Row) {
        g.open_settings_on(tab);
        step(g);
        g.settings_page.row = rows(tab).iter().position(|&r| r == row).expect("the row is on its tab");
    }

    #[test]
    fn the_pause_menu_opens_settings_and_back_closes_it() {
        let mut g = paused();
        let k = g.menu_items().iter().position(|&m| m == MenuItem::Settings).expect("SETTINGS in the pause menu");
        g.menu_index = k;
        g.input.inject(Action::Confirm);
        step(&mut g);
        assert!(g.settings_page.open);
        // RB (E) to the next tab, LB (Q) back, around the end.
        key(&mut g, KeyCode::KeyE);
        assert_eq!(g.settings_page.tab, Tab::Controls);
        key(&mut g, KeyCode::KeyQ);
        key(&mut g, KeyCode::KeyQ);
        assert_eq!(g.settings_page.tab, Tab::Accessibility);
        // Esc: back to the pause menu, SETTINGS highlighted, still paused.
        key(&mut g, KeyCode::Escape);
        assert!(!g.settings_page.open);
        assert_eq!(g.screen, Screen::Paused);
        assert_eq!(g.menu_items()[g.menu_index], MenuItem::Settings);
        // From the title too.
        own_saves();
        let mut g = Game::new();
        let k = g.menu_items().iter().position(|&m| m == MenuItem::Settings).expect("SETTINGS on the title");
        g.menu_index = k;
        g.input.inject(Action::Confirm);
        step(&mut g);
        assert!(g.settings_page.open);
        assert_eq!(g.screen, Screen::Title);
    }

    #[test]
    fn every_row_changes_and_applies_live() {
        let mut g = paused();
        // UI scale: the HUD's unit grows with it.
        let t0 = hud::tile_for(1280.0, 720.0);
        on(&mut g, Tab::Display, Row::UiScale);
        key(&mut g, KeyCode::ArrowRight);
        key(&mut g, KeyCode::ArrowRight);
        assert_eq!(g.settings.ui_scale, 120);
        let t1 = hud::tile_for(1280.0, 720.0);
        assert!((t1 / t0 - 1.2).abs() < 1e-4, "{t0} -> {t1}");
        // The volumes reach the mixer's settings.
        on(&mut g, Tab::Audio, Row::Music);
        key(&mut g, KeyCode::ArrowLeft);
        assert_eq!(g.settings.music, 95);
        assert!((g.audio.volumes().music - 0.95).abs() < 1e-9);
        // Colour-blind: the view's tells take the palette.
        on(&mut g, Tab::Accessibility, Row::ColourBlind);
        key(&mut g, KeyCode::Enter);
        assert_eq!(g.settings.colour_blind, ColourBlind::Deuteranopia);
        assert_eq!(g.view.colour_blind, ColourBlind::Deuteranopia);
        assert_eq!(hud::prefs().colour_blind, ColourBlind::Deuteranopia);
        // The shield by toggle reaches the input.
        on(&mut g, Tab::Gameplay, Row::ShieldMode);
        key(&mut g, KeyCode::Enter);
        assert!(g.settings.shield_toggle);
        // Every row of every tab answers left, right and A without a panic, and comes back.
        for tab in Tab::ALL {
            for (i, row) in rows(tab).into_iter().enumerate() {
                if matches!(row, Row::Bind(_) | Row::ResetAll | Row::Fullscreen) {
                    continue;
                }
                g.open_settings_on(tab);
                g.settings_page.row = i;
                let before = g.settings_value(row);
                // (A slider at the top of its range is stepped down and back up.)
                let (there, back) = match g.settings_slider(row) {
                    Some((r, v)) if v == r.max => (KeyCode::ArrowLeft, KeyCode::ArrowRight),
                    _ => (KeyCode::ArrowRight, KeyCode::ArrowLeft),
                };
                key(&mut g, there);
                let changed = g.settings_value(row);
                key(&mut g, back);
                assert_eq!(g.settings_value(row), before, "{row:?} comes back");
                assert_ne!(changed, before, "{row:?} changes");
            }
        }
        // Back to the defaults for the other tests on this thread.
        hud::set_prefs(crate::settings::Settings::default().ui());
    }

    #[test]
    fn a_click_on_a_slider_sets_it_where_it_lands() {
        let mut g = paused();
        on(&mut g, Tab::Audio, Row::Master);
        step(&mut g);
        let (w, h) = g.viewport;
        let layout = hud::settings::layout(w, h, rows(Tab::Audio).len(), 0);
        let track = layout.track(0);
        g.input.cursor_moved_to(track[0] + track[2] * 0.3, track[1] + track[3] / 2.0);
        g.input.mouse_button(winit::event::MouseButton::Left, true);
        step(&mut g);
        g.input.mouse_button(winit::event::MouseButton::Left, false);
        assert_eq!(g.settings.master, 30);
    }

    /// The attack's row on the keyboard's side, its first slot chosen and listening.
    fn listening_for_attack(g: &mut Game) {
        on(g, Tab::Controls, Row::Bind(Action::Attack));
        g.settings_page.pad = false;
        g.settings_page.column = 0;
        key(g, KeyCode::Enter);
        assert!(g.settings_page.listen.is_some(), "listening");
    }

    #[test]
    fn rebinding_binds_asks_on_a_conflict_swaps_cancels_and_times_out() {
        let mut g = paused();
        // Bind: F for the attack.
        listening_for_attack(&mut g);
        key(&mut g, KeyCode::KeyF);
        assert!(g.settings_page.listen.is_none());
        assert_eq!(g.input.bindings.side(Action::Attack, false), vec![Control::Key(KeyCode::KeyF)]);
        // Conflict: Q is the drink's. Asked; swapped: the drink takes F.
        listening_for_attack(&mut g);
        key(&mut g, KeyCode::KeyQ);
        let c = g.settings_page.conflict.clone().expect("asked");
        assert_eq!(c.others, vec![Action::Drink]);
        assert_eq!(g.input.bindings.side(Action::Attack, false), vec![Control::Key(KeyCode::KeyF)], "nothing yet");
        key(&mut g, KeyCode::Enter);
        assert!(g.settings_page.conflict.is_none());
        assert_eq!(g.input.bindings.side(Action::Attack, false), vec![Control::Key(KeyCode::KeyQ)]);
        assert_eq!(g.input.bindings.side(Action::Drink, false), vec![Control::Key(KeyCode::KeyF)]);
        // Conflict, cancelled with Esc: nothing changes, and the page stays open.
        let before = g.input.bindings.clone();
        listening_for_attack(&mut g);
        key(&mut g, KeyCode::KeyE);
        assert!(g.settings_page.conflict.is_some());
        key(&mut g, KeyCode::Escape);
        assert!(g.settings_page.conflict.is_none() && g.settings_page.open);
        assert_eq!(g.input.bindings, before);
        // Take it: the call loses E, the attack has it.
        listening_for_attack(&mut g);
        key(&mut g, KeyCode::KeyE);
        key(&mut g, KeyCode::ArrowRight);
        key(&mut g, KeyCode::Enter);
        assert!(g.input.bindings.side(Action::Call, false).is_empty());
        // Listening, cancelled by Esc; a fixed key refused (still listening); then the timeout.
        let before = g.input.bindings.clone();
        listening_for_attack(&mut g);
        key(&mut g, KeyCode::Escape);
        assert!(g.settings_page.listen.is_none() && g.settings_page.open, "Esc cancels, the page stays");
        listening_for_attack(&mut g);
        key(&mut g, KeyCode::F1);
        assert!(g.settings_page.listen.is_some(), "F1 is the game's own");
        // (The pad's buttons are not the keyboard's side's.)
        g.input.set_pad(Some(crate::input::PadState {
            buttons: [PadButton::South].into_iter().collect(),
            kind: Some(Device::Xbox),
            ..Default::default()
        }));
        step(&mut g);
        g.input.set_pad(None);
        assert!(g.settings_page.listen.is_some());
        for _ in 0..(LISTEN_SECONDS / crate::app::STEP) as usize + 4 {
            step(&mut g);
        }
        assert!(g.settings_page.listen.is_none(), "it gives up");
        assert_eq!(g.input.bindings, before);
        // A mouse button binds on the keyboard's side; the second slot.
        on(&mut g, Tab::Controls, Row::Bind(Action::Attack));
        g.settings_page.pad = false;
        g.settings_page.column = 1;
        key(&mut g, KeyCode::Enter);
        g.input.mouse_button(winit::event::MouseButton::Back, true);
        step(&mut g);
        g.input.mouse_button(winit::event::MouseButton::Back, false);
        assert_eq!(g.input.bindings.side(Action::Attack, false), vec![Control::Key(KeyCode::KeyE), Control::Mouse(MouseButton::Back)]);
        // The reset column: the row back to its defaults; then every row, asked twice.
        g.settings_page.column = 2;
        key(&mut g, KeyCode::Enter);
        assert_eq!(g.input.bindings.controls(Action::Attack), crate::input::bindings::default_controls(Action::Attack).to_vec());
        on(&mut g, Tab::Controls, Row::ResetAll);
        key(&mut g, KeyCode::Enter);
        assert_ne!(g.input.bindings, crate::input::Bindings::default(), "asked first");
        key(&mut g, KeyCode::Enter);
        assert_eq!(g.input.bindings, crate::input::Bindings::default());
    }

    #[test]
    fn the_controllers_side_rebinds_with_its_buttons() {
        let mut g = paused();
        on(&mut g, Tab::Controls, Row::Bind(Action::Angel));
        g.settings_page.pad = true;
        g.settings_page.column = 0;
        key(&mut g, KeyCode::Enter);
        let press = |g: &mut Game, b: Option<PadButton>| {
            g.input.set_pad(Some(crate::input::PadState {
                buttons: b.into_iter().collect(),
                kind: Some(Device::PlayStation),
                ..Default::default()
            }));
            step(g);
        };
        press(&mut g, Some(PadButton::LeftStick));
        press(&mut g, None);
        assert!(g.settings_page.conflict.is_some(), "L3 is the minimap's zoom: asked");
        assert_eq!(g.input.bindings.side(Action::Angel, true), vec![Control::Pad(PadButton::RightTrigger)], "nothing yet");
        // A (Cross) answers with the highlighted SWAP: the zoom takes RT (R2).
        press(&mut g, Some(PadButton::South));
        press(&mut g, None);
        assert!(g.settings_page.conflict.is_none());
        assert_eq!(g.input.bindings.side(Action::Angel, true), vec![Control::Pad(PadButton::LeftStick)]);
        assert_eq!(g.input.bindings.side(Action::MinimapZoom, true), vec![Control::Pad(PadButton::RightTrigger)]);
        // The Menu button cancels listening on the controller.
        key(&mut g, KeyCode::Enter);
        assert!(g.settings_page.listen.is_some());
        press(&mut g, Some(PadButton::Menu));
        press(&mut g, None);
        assert!(g.settings_page.listen.is_none() && g.settings_page.open);
        assert_eq!(g.screen, Screen::Paused);
    }
}
