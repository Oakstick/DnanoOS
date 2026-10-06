//! Keyboard, mouse, touch and gamepad, reduced to game actions through the action map
//! (`bindings`: UI.md §3). Movement is hold-to-move: `direction()` is the most recently pressed
//! direction still held. Touch works as a drag joystick. The game never needs to know the device,
//! except to draw its prompts in the glyphs of the one used last (`last_device`).
//!
//! The controls are read through one of two layers, play's or the menus', as the game says each
//! update (`set_context`): in play the D-pad chooses potions, opens the map and switches the
//! camera; in a menu it moves the highlight. A key, a button or the wheel reads through the layer
//! of the moment it is pressed, and lets go of what it pressed when it is released.

pub mod bindings;
pub mod rumble;

use std::collections::{HashMap, HashSet};

pub use bindings::{action_name, control_name, from_token, key_name, pad_kind, token, trigger_latch, Bindings, Context, Control, Device};
pub use bindings::{MouseButton, PadButton};
pub use rumble::{Pulse, Rumble, RumbleLevel, Rumbler};

use crate::logic::types::Dir;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Action {
    /// Enter, or A in a menu: begin, continue, choose.
    Confirm,
    /// A tap, or a left click in a menu: picks what it lands on; a tap attacks in play.
    Tap,
    /// Left click / X.
    Attack,
    /// Space / A: jump, or evade where fights go by frame data.
    Jump,
    /// B / pad B: the old man's guide to the place (a pause with his words).
    Guide,
    /// C / D-pad down: the camera, chase or top.
    Camera,
    /// Esc / Menu.
    Pause,
    /// Ctrl+M (the settings' Audio page to come).
    Mute,
    /// F3.
    Fps,
    /// F11 (and Alt+Enter, which the app reads).
    Fullscreen,
    /// R3: swing the camera back behind Ragnar.
    CameraReset,
    /// F1: straight to the first boss fight (testing cheat).
    Cheat,
    /// F2: every key, and Ragnar at the gate (testing cheat).
    KeyCheat,
    /// F4: free movement on or off.
    FreeMove,
    /// Q / RB: drink the selected potion on the belt.
    Drink,
    /// The wheel, or the D-pad's left and right: the potion before, the potion after.
    PotionPrev,
    PotionNext,
    /// 1..5: drink the potion in that belt slot.
    Slot(u8),
    /// 0: straight to the old man's wares (testing cheat).
    ShopCheat,
    /// Left Shift / LT: Bifrost Step, the path's blink (once it is learned).
    Step,
    /// Right click / LB, held: raise the shield.
    Block,
    /// E / Y: call the old gatekeeper, and his wares open.
    Call,
    /// R / RT: one use of the archangel's blessing he carries.
    Angel,
    /// LB and RB, Q and E (or Tab) in a menu: the tab before, the tab after.
    TabPrev,
    TabNext,
    /// M, Tab, View or the D-pad's up: the full map.
    Map,
    /// L3: the minimap's zoom (the minimap's builder gives it its work).
    MinimapZoom,
    /// B, Esc or Menu in a menu: back (it presses Pause too, which the screens read as back).
    Back,
    /// Y in a menu (R on the keyboard): the page's second choice.
    Secondary,
    /// The run's four ways (W A S D, the arrows): never pressed as actions, they steer `direction`.
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    /// A menu's four ways: never pressed as actions, they set `nav`.
    NavUp,
    NavDown,
    NavLeft,
    NavRight,
    /// F8: the Sparring Ring's frame readout on or off.
    Readout,
    /// F5: immortal on or off (testing cheat): blows still land, but his health always comes back.
    Immortal,
    /// F10: the nearest foe becomes a rival of the Ledger, there and then (testing cheat).
    MakeRival,
    /// F9: the Sparring Ring's next bout; F7: the first of the next stage's.
    NextBout,
    /// F6: a thousand coins into his purse, every press (testing cheat).
    Gold,
    NextStageBout,
    /// F12: the developer's map, the list of stops to jump to (testing).
    DevMap,
}

impl Action {
    /// The way a run action steers.
    pub fn run_dir(self) -> Option<Dir> {
        Some(match self {
            Action::MoveUp => Dir::Up,
            Action::MoveDown => Dir::Down,
            Action::MoveLeft => Dir::Left,
            Action::MoveRight => Dir::Right,
            _ => return None,
        })
    }

    /// The way a menu's move goes.
    pub fn nav_dir(self) -> Option<Dir> {
        Some(match self {
            Action::NavUp => Dir::Up,
            Action::NavDown => Dir::Down,
            Action::NavLeft => Dir::Left,
            Action::NavRight => Dir::Right,
            _ => return None,
        })
    }

    /// What a press of this action puts down: Back is Pause as well (the screens read Pause as
    /// back, and in the pause menu it resumes).
    fn put_down(self) -> impl Iterator<Item = Action> {
        std::iter::once(self).chain((self == Action::Back).then_some(Action::Pause))
    }
}

/// Past this much push a stick counts as held; a grid needs a decisive shove.
const MOVE_DEADZONE: f32 = 0.4;
const LOOK_DEADZONE: f32 = 0.16;
/// Right stick at full tilt turns the camera this fast, in mouse pixels per second.
const PAD_LOOK_SPEED: f64 = 950.0;
/// The mouse or stick counts as steering the camera for this long after its
/// last movement; with the stick, `pad_aiming` also holds on until recentered.
const PAD_AIM_MEMORY: f64 = 1.2;
/// The mouse moved this many pixels in an update: the keyboard and mouse are in use.
const MOUSE_MEANINGFUL: f64 = 6.0;

/// One controller reading, in the web Gamepad API's conventions (y down).
#[derive(Clone, Default, Debug)]
pub struct PadState {
    /// Actions held, as a bot presses them (they count whatever the context).
    pub actions: HashSet<Action>,
    /// The D-pad, as a button (`buttons` has it too from a real controller).
    pub dpad: Option<Dir>,
    pub left: (f32, f32),
    pub right: (f32, f32),
    /// A real controller's buttons held (the triggers past their threshold), read through the
    /// bindings in the context of the moment. The bots leave it empty and press `actions`.
    pub buttons: HashSet<PadButton>,
    /// The controller's family, from a real one (none from a bot: its play is no device used).
    pub kind: Option<Device>,
}

#[derive(Default)]
pub struct Input {
    held: Vec<Dir>,
    pressed: HashSet<Action>,
    just: HashSet<Action>,
    touch: Option<(u64, f64, f64)>,
    touch_dir: Dir,
    look: (f64, f64),
    pad: Option<PadState>,
    pad_dir: Dir,
    pad_held: HashSet<Action>,
    pad_aim_age: f64,
    /// The controller's buttons down at the last poll: a press is a button not among them.
    pad_buttons: HashSet<PadButton>,
    /// A stick was past its deadzone at the last poll.
    stick_out: bool,
    /// What each key or mouse button held put down, lifted when it is let go.
    by_control: HashMap<Control, Vec<Action>>,
    ctrl: bool,
    /// The wheel's turn not yet a whole notch, and the mouse's movement this update.
    wheel: f64,
    motion: f64,
    context: Context,
    device: Device,
    /// The controls that went down this update (keys, buttons, the wheel's notches), and the
    /// wheel's notches turned (up positive): for a page that reads them as they are, the map.
    went_down: HashSet<Control>,
    wheel_turned: i32,
    rumble: RumbleLevel,
    /// The action map: play's layer as the player has bound it (the menus' is fixed).
    pub bindings: Bindings,
    /// Once the right stick has moved the camera, it stays where it was put
    /// (no drifting back behind him) until R3 recenters it.
    pub pad_aiming: bool,
    /// True once a controller has been seen.
    pub pad_connected: bool,
    /// The mouse is captured for aiming.
    pub mouse_captured: bool,
    /// Pointer position in logical pixels, for menus.
    pub cursor: Option<(f32, f32)>,
    /// The pointer moved since the last update (menus follow the hover).
    pub cursor_moved: bool,
    /// Where the last left click or tap landed this update, in logical pixels.
    pub click: Option<(f32, f32)>,
    /// A direction pressed this update (menu navigation), from keys, d-pad or stick.
    pub nav: Option<Dir>,
}

impl Input {
    pub fn new() -> Self {
        Self { pad_dir: Dir::None, touch_dir: Dir::None, pad_aim_age: PAD_AIM_MEMORY, ..Default::default() }
    }

    /// The player's bindings and rumble, as saved (tests keep to the defaults).
    pub fn load_settings(&mut self) {
        self.bindings = Bindings::load();
        self.rumble = RumbleLevel::load();
    }

    /// The layer the controls read through from now on (the game sets it each update).
    pub fn set_context(&mut self, context: Context) {
        self.context = context;
    }

    pub fn context(&self) -> Context {
        self.context
    }

    /// The kind of device used last: keyboard and mouse (touch too), or a controller's family.
    pub fn last_device(&self) -> Device {
        self.device
    }

    /// Sets the device used last (captures, to draw the prompts in each device's glyphs).
    pub fn set_last_device(&mut self, device: Device) {
        self.device = device;
    }

    /// The rumble setting.
    pub fn rumble(&self) -> RumbleLevel {
        self.rumble
    }

    /// Sets the rumble, and saves it.
    pub fn set_rumble(&mut self, level: RumbleLevel) {
        self.rumble = level;
        level.save();
    }

    /// The name of the control that does `a` on the device used last ("RT", "R2", "R",
    /// "Left click"), for text that names it; none if nothing is bound.
    pub fn prompt(&self, a: Action) -> Option<String> {
        self.bindings.shown(a, self.device).map(|c| control_name(c, self.device).into_owned())
    }

    fn press(&mut self, a: Action) {
        self.pressed.insert(a);
        self.just.insert(a);
    }

    /// A key, a mouse button or a notch of the wheel goes down: the run's ways by play's layer and
    /// a menu's moves by the menus' (whatever the context, so a key held through a menu still runs
    /// after it), and the rest by the context's layer.
    fn control_down(&mut self, c: Control) {
        let play = self.bindings.actions(c, Context::Play);
        for d in play.iter().filter_map(|a| a.run_dir()) {
            self.held.retain(|&h| h != d);
            self.held.push(d);
        }
        self.went_down.insert(c);
        let menu = self.bindings.actions(c, Context::Menu);
        if let Some(d) = menu.iter().find_map(|a| a.nav_dir()) {
            self.nav = Some(d);
        }
        let layer = if self.context == Context::Play { play } else { menu };
        let acts: Vec<Action> =
            layer.into_iter().filter(|a| a.run_dir().is_none() && a.nav_dir().is_none()).flat_map(Action::put_down).collect();
        for &a in &acts {
            self.press(a);
        }
        self.by_control.entry(c).or_default().extend(acts);
    }

    fn control_up(&mut self, c: Control) {
        for d in self.bindings.actions(c, Context::Play).iter().filter_map(|a| a.run_dir()) {
            self.held.retain(|&h| h != d);
        }
        for a in self.by_control.remove(&c).unwrap_or_default() {
            self.pressed.remove(&a);
        }
    }

    pub fn key_down(&mut self, code: winit::keyboard::KeyCode, repeat: bool) {
        use winit::keyboard::KeyCode;
        if repeat {
            return;
        }
        self.device = Device::KeyboardMouse;
        if matches!(code, KeyCode::ControlLeft | KeyCode::ControlRight) {
            self.ctrl = true;
        }
        // Ctrl+M mutes (M alone is the map), and the testing keys never move.
        let fixed = if self.ctrl && code == KeyCode::KeyM { Some(Action::Mute) } else { bindings::fixed_key(code) };
        if let Some(a) = fixed {
            self.press(a);
            self.by_control.entry(Control::Key(code)).or_default().push(a);
            return;
        }
        self.control_down(Control::Key(code));
    }

    pub fn key_up(&mut self, code: winit::keyboard::KeyCode) {
        use winit::keyboard::KeyCode;
        if matches!(code, KeyCode::ControlLeft | KeyCode::ControlRight) {
            self.ctrl = false;
        }
        self.control_up(Control::Key(code));
    }

    pub fn cursor_moved_to(&mut self, x: f32, y: f32) {
        if let Some((px, py)) = self.cursor {
            if (x - px).abs() + (y - py).abs() > MOUSE_MEANINGFUL as f32 {
                self.device = Device::KeyboardMouse;
            }
        }
        if self.cursor != Some((x, y)) {
            self.cursor = Some((x, y));
            self.cursor_moved = true;
        }
    }

    /// A mouse button, through the bindings (left click attacks and right click shields in play);
    /// in a menu the left click is a tap too, picking what it lands on.
    pub fn mouse_button(&mut self, button: winit::event::MouseButton, down: bool) {
        self.device = Device::KeyboardMouse;
        let Some(b) = MouseButton::from_winit(button) else { return };
        let c = Control::Mouse(b);
        if !down {
            self.control_up(c);
            return;
        }
        if b == MouseButton::Left {
            self.click = self.cursor;
        }
        self.control_down(c);
        if b == MouseButton::Left && self.context == Context::Menu {
            self.press(Action::Tap);
            self.by_control.entry(c).or_default().push(Action::Tap);
        }
    }

    /// The wheel turned by `lines` (up positive): each whole notch is a press of its control.
    pub fn wheel(&mut self, lines: f64) {
        if lines == 0.0 || !lines.is_finite() {
            return;
        }
        self.device = Device::KeyboardMouse;
        if self.wheel.signum() != lines.signum() {
            self.wheel = 0.0;
        }
        self.wheel = (self.wheel + lines).clamp(-4.0, 4.0);
        while self.wheel.abs() >= 1.0 {
            let c = if self.wheel > 0.0 { Control::WheelUp } else { Control::WheelDown };
            self.wheel_turned += if self.wheel > 0.0 { 1 } else { -1 };
            self.wheel -= self.wheel.signum();
            self.control_down(c);
            self.control_up(c);
        }
    }

    pub fn mouse_motion(&mut self, dx: f64, dy: f64) {
        self.motion += dx.abs() + dy.abs();
        if self.motion > MOUSE_MEANINGFUL {
            self.device = Device::KeyboardMouse;
        }
        if self.mouse_captured {
            self.look.0 += dx;
            self.look.1 += dy;
        }
    }

    pub fn touch(&mut self, id: u64, phase: winit::event::TouchPhase, x: f64, y: f64) {
        use winit::event::TouchPhase::*;
        self.device = Device::KeyboardMouse;
        match phase {
            Started => {
                if self.touch.is_none() {
                    self.touch = Some((id, x, y));
                    self.just.insert(Action::Tap);
                    self.click = Some((x as f32, y as f32));
                }
            }
            Moved => {
                if let Some((tid, sx, sy)) = self.touch {
                    if tid != id {
                        return;
                    }
                    let (dx, dy) = (x - sx, y - sy);
                    if dx.abs().max(dy.abs()) > 18.0 {
                        self.touch_dir = if dx.abs() > dy.abs() {
                            if dx > 0.0 {
                                Dir::Right
                            } else {
                                Dir::Left
                            }
                        } else if dy > 0.0 {
                            Dir::Down
                        } else {
                            Dir::Up
                        };
                    }
                }
            }
            Ended | Cancelled => {
                if matches!(self.touch, Some((tid, _, _)) if tid == id) {
                    self.touch = None;
                    self.touch_dir = Dir::None;
                }
            }
        }
    }

    /// Window lost focus: nothing is held any more.
    pub fn blur(&mut self) {
        self.held.clear();
        self.pressed.clear();
        self.by_control.clear();
        self.ctrl = false;
        self.touch = None;
        self.touch_dir = Dir::None;
    }

    pub fn set_pad(&mut self, pad: Option<PadState>) {
        self.pad = pad;
    }

    /// Reads the controller; called once per fixed update so stick aiming is frame-rate independent.
    pub fn poll(&mut self, dt: f64) {
        self.pad_aim_age += dt;
        self.motion = 0.0;
        let Some(pad) = self.pad.clone() else {
            for a in self.pad_held.drain() {
                self.pressed.remove(&a);
            }
            self.pad_buttons.clear();
            self.stick_out = false;
            self.pad_dir = Dir::None;
            return;
        };
        self.pad_connected = true;

        // A bot's actions, pressed as they come; then the buttons through the bindings, in this
        // update's context (a button held from before a change of context presses nothing new).
        let mut buttons = pad.buttons.clone();
        buttons.extend(pad.dpad.and_then(PadButton::dpad));
        let mut held = pad.actions.clone();
        let mut fresh: Vec<Action> = pad.actions.iter().filter(|a| !self.pad_held.contains(a)).copied().collect();
        let mut button_dir = Dir::None;
        let mut order: Vec<PadButton> = buttons.iter().copied().collect();
        order.sort_by_key(|b| PadButton::ALL.iter().position(|x| x == b));
        for &b in &order {
            let new = !self.pad_buttons.contains(&b);
            if new {
                self.went_down.insert(Control::Pad(b));
            }
            if let Some(d) = self.bindings.actions(Control::Pad(b), Context::Play).iter().find_map(|a| a.run_dir()) {
                if button_dir == Dir::None {
                    button_dir = d;
                }
            }
            for a in self.bindings.actions(Control::Pad(b), self.context) {
                if a.run_dir().is_some() {
                    continue;
                }
                if let Some(d) = a.nav_dir() {
                    if new {
                        self.nav = Some(d);
                    }
                    continue;
                }
                for p in a.put_down() {
                    if new {
                        fresh.push(p);
                    }
                    held.insert(p);
                }
            }
        }
        for a in fresh {
            self.just.insert(a);
        }
        for &a in &held {
            self.pressed.insert(a);
        }
        for a in self.pad_held.iter() {
            if !held.contains(a) {
                self.pressed.remove(a);
            }
        }
        self.pad_held = held;

        // A real controller's button pressed, or a stick pushed out past its deadzone: the
        // controller is the device in use.
        let out = [pad.left, pad.right].iter().any(|&(x, y)| x.abs().max(y.abs()) >= MOVE_DEADZONE);
        if let Some(kind) = pad.kind {
            if buttons.iter().any(|b| !self.pad_buttons.contains(b)) || (out && !self.stick_out) {
                self.device = kind;
            }
        }
        self.stick_out = out;
        self.pad_buttons = buttons;

        // The left stick chooses the corridor (or a button bound to running; the D-pad does not run).
        let mut dir = button_dir;
        let (x, y) = pad.left;
        if x.abs().max(y.abs()) >= MOVE_DEADZONE {
            dir = if x.abs() > y.abs() {
                if x > 0.0 {
                    Dir::Right
                } else {
                    Dir::Left
                }
            } else if y > 0.0 {
                Dir::Down
            } else {
                Dir::Up
            };
        }
        if dir != self.pad_dir && dir != Dir::None {
            self.nav = Some(dir);
        }
        self.pad_dir = dir;

        // Right stick aims, squared so small pushes stay fine and full tilt is quick.
        let lx = curve(pad.right.0);
        let ly = curve(pad.right.1);
        if lx != 0.0 || ly != 0.0 {
            self.look.0 += lx as f64 * PAD_LOOK_SPEED * dt;
            self.look.1 += ly as f64 * PAD_LOOK_SPEED * dt;
            self.pad_aim_age = 0.0;
            self.pad_aiming = true;
        }
    }

    /// True while the mouse or the right stick is aiming the camera.
    pub fn aiming(&self) -> bool {
        self.mouse_captured || self.pad_aiming || self.pad_aim_age < PAD_AIM_MEMORY
    }

    /// Look movement in mouse pixels since the last call, then cleared.
    pub fn take_look(&mut self) -> (f64, f64) {
        std::mem::take(&mut self.look)
    }

    /// Held movement direction right now, or none.
    pub fn direction(&self) -> Dir {
        if self.pad_dir != Dir::None {
            return self.pad_dir;
        }
        if self.touch_dir != Dir::None {
            return self.touch_dir;
        }
        self.held.last().copied().unwrap_or(Dir::None)
    }

    /**
     * Free movement's direction (or none), unit length: held keys add up, so
     * W and D together run diagonally and opposite keys cancel (eight ways);
     * the left stick points any way at all.
     */
    pub fn move_vector(&self) -> (f64, f64) {
        let mut v = (0.0, 0.0);
        let mut add = |d: Dir| {
            let (x, y) = d.vec();
            v.0 += x as f64;
            v.1 += y as f64;
        };
        let stick = self.pad.as_ref().map(|p| p.left).filter(|&(x, y)| x.abs().max(y.abs()) >= MOVE_DEADZONE);
        if let Some((x, y)) = stick {
            let a = (y as f64).atan2(x as f64);
            return (a.cos(), a.sin());
        }
        if self.pad_dir != Dir::None {
            add(self.pad_dir);
        } else if self.touch_dir != Dir::None {
            add(self.touch_dir);
        } else {
            let mut seen: Vec<Dir> = vec![];
            for &d in &self.held {
                if !seen.contains(&d) {
                    seen.push(d);
                    add(d);
                }
            }
        }
        let len = (v.0 * v.0 + v.1 * v.1).sqrt();
        if len > 0.0 {
            (v.0 / len, v.1 / len)
        } else {
            (0.0, 0.0)
        }
    }

    pub fn clear_direction(&mut self) {
        self.held.clear();
        self.touch_dir = Dir::None;
        self.pad_dir = Dir::None;
    }

    pub fn is_down(&self, a: Action) -> bool {
        self.pressed.contains(&a)
    }

    /// True once per press; cleared after each update by `flush`.
    pub fn was_pressed(&self, actions: &[Action]) -> bool {
        actions.iter().any(|a| self.just.contains(a))
    }

    /// This update's presses, in no order (a freeze keeps them for the step that ends it).
    pub fn presses(&self) -> impl Iterator<Item = Action> + '_ {
        self.just.iter().copied()
    }

    /// A press kept from an earlier update, made new again until `flush` (and not held: the
    /// button's state is its own). Whether it was not already pressed this update.
    pub fn repress(&mut self, a: Action) -> bool {
        self.just.insert(a)
    }

    /// Removes a press so nothing else reacts to it this update.
    pub fn consume(&mut self, a: Action) {
        self.just.remove(&a);
    }

    pub fn flush(&mut self) {
        self.just.clear();
        self.went_down.clear();
        self.wheel_turned = 0;
        self.click = None;
        self.nav = None;
        self.cursor_moved = false;
    }

    /// Whether `c` went down this update (whatever it stands for in the layer of the moment).
    pub fn went_down(&self, c: Control) -> bool {
        self.went_down.contains(&c)
    }

    /// The wheel's whole notches turned this update, up positive.
    pub fn wheel_turned(&self) -> i32 {
        self.wheel_turned
    }

    /// The controller's two sticks as they are now (y down), each past its deadzone or nothing,
    /// for a page that reads them as they are (the map pans and zooms by them).
    pub fn sticks(&self) -> ((f32, f32), (f32, f32)) {
        let Some(p) = &self.pad else { return ((0.0, 0.0), (0.0, 0.0)) };
        let past = |(x, y): (f32, f32)| if x.abs().max(y.abs()) >= LOOK_DEADZONE { (x, y) } else { (0.0, 0.0) };
        (past(p.left), past(p.right))
    }

    /// Is `a` held down now (a key, a mouse button or a controller button)?
    pub fn held(&self, a: Action) -> bool {
        self.pressed.contains(&a)
    }

    /// Feeds a scripted press (for captures and tests).
    pub fn inject(&mut self, a: Action) {
        self.press(a);
    }

    pub fn inject_release(&mut self, a: Action) {
        self.pressed.remove(&a);
    }

    pub fn hold_dir(&mut self, d: Dir, down: bool) {
        self.held.retain(|&h| h != d);
        if down {
            self.held.push(d);
        }
    }
}

/// Stick response: squared past the deadzone, so a nudge is a nudge.
fn curve(v: f32) -> f32 {
    if v.abs() < LOOK_DEADZONE {
        return 0.0;
    }
    let t = (v.abs() - LOOK_DEADZONE) / (1.0 - LOOK_DEADZONE);
    v.signum() * t * t
}

#[cfg(test)]
mod tests;
