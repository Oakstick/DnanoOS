//! Keyboard, mouse, touch and gamepad, reduced to game actions. Movement is
//! hold-to-move: `direction()` is the most recently pressed direction still
//! held. Touch works as a drag joystick. Controller buttons report as the
//! actions they stand for, so the game never needs to know the device.

use std::collections::HashSet;

use crate::logic::types::Dir;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Action {
    /// Enter / Start: begin, continue.
    Confirm,
    /// A tap or left click: begins from a menu, attacks in play.
    Tap,
    Attack,
    Jump,
    Bomb,
    Camera,
    Pause,
    Mute,
    Fps,
    Fullscreen,
    /// Right-stick click: swing the camera back behind Sebastian.
    CameraReset,
    /// F1: straight to the first boss fight (testing cheat).
    Cheat,
    /// Q / RB: drink the selected potion on the belt.
    Drink,
    /// Tab / LT: select the next potion (in the shop: the next tab).
    NextPotion,
    /// 1..5: drink the potion in that belt slot.
    Slot(u8),
}

/// Past this much push a stick counts as held; a grid needs a decisive shove.
const MOVE_DEADZONE: f32 = 0.4;
const LOOK_DEADZONE: f32 = 0.16;
/// Right stick at full tilt turns the camera this fast, in mouse pixels per second.
const PAD_LOOK_SPEED: f64 = 950.0;
/// The mouse or stick counts as steering the camera for this long after its
/// last movement; with the stick, `pad_aiming` also holds on until recentered.
const PAD_AIM_MEMORY: f64 = 1.2;

/// One controller reading, in the web Gamepad API's conventions (y down).
#[derive(Clone, Default, Debug)]
pub struct PadState {
    pub actions: HashSet<Action>,
    pub dpad: Option<Dir>,
    pub left: (f32, f32),
    pub right: (f32, f32),
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

pub fn key_to_dir(code: winit::keyboard::KeyCode) -> Dir {
    use winit::keyboard::KeyCode::*;
    match code {
        ArrowUp | KeyW => Dir::Up,
        ArrowDown | KeyS => Dir::Down,
        ArrowLeft | KeyA => Dir::Left,
        ArrowRight | KeyD => Dir::Right,
        _ => Dir::None,
    }
}

pub fn key_to_action(code: winit::keyboard::KeyCode) -> Option<Action> {
    use winit::keyboard::KeyCode::*;
    Some(match code {
        Space => Action::Jump,
        Enter | NumpadEnter => Action::Confirm,
        Escape => Action::Pause,
        KeyM => Action::Mute,
        F1 => Action::Cheat,
        F3 => Action::Fps,
        KeyC => Action::Camera,
        KeyQ => Action::Drink,
        Tab => Action::NextPotion,
        Digit1 => Action::Slot(0),
        Digit2 => Action::Slot(1),
        Digit3 => Action::Slot(2),
        Digit4 => Action::Slot(3),
        Digit5 => Action::Slot(4),
        KeyE => Action::Attack,
        KeyB | KeyL | ShiftLeft | ShiftRight => Action::Bomb,
        F11 => Action::Fullscreen,
        _ => return None,
    })
}

impl Input {
    pub fn new() -> Self {
        Self { pad_dir: Dir::None, touch_dir: Dir::None, pad_aim_age: PAD_AIM_MEMORY, ..Default::default() }
    }

    fn press(&mut self, a: Action) {
        self.pressed.insert(a);
        self.just.insert(a);
    }

    pub fn key_down(&mut self, code: winit::keyboard::KeyCode, repeat: bool) {
        if repeat {
            return;
        }
        let d = key_to_dir(code);
        if d != Dir::None {
            self.held.retain(|&h| h != d);
            self.held.push(d);
            self.nav = Some(d);
        }
        if let Some(a) = key_to_action(code) {
            self.press(a);
        }
    }

    pub fn cursor_moved_to(&mut self, x: f32, y: f32) {
        if self.cursor != Some((x, y)) {
            self.cursor = Some((x, y));
            self.cursor_moved = true;
        }
    }

    pub fn key_up(&mut self, code: winit::keyboard::KeyCode) {
        let d = key_to_dir(code);
        if d != Dir::None {
            self.held.retain(|&h| h != d);
        }
        if let Some(a) = key_to_action(code) {
            self.pressed.remove(&a);
        }
    }

    /// Left click attacks (and begins from a menu); right click drops a bomb.
    pub fn mouse_button(&mut self, button: winit::event::MouseButton, down: bool) {
        use winit::event::MouseButton;
        let actions: &[Action] = match button {
            MouseButton::Left => &[Action::Attack, Action::Tap],
            MouseButton::Right => &[Action::Bomb],
            _ => &[],
        };
        if down && button == MouseButton::Left {
            self.click = self.cursor;
        }
        for &a in actions {
            if down {
                self.press(a);
            } else {
                self.pressed.remove(&a);
            }
        }
    }

    pub fn mouse_motion(&mut self, dx: f64, dy: f64) {
        if self.mouse_captured {
            self.look.0 += dx;
            self.look.1 += dy;
        }
    }

    pub fn touch(&mut self, id: u64, phase: winit::event::TouchPhase, x: f64, y: f64) {
        use winit::event::TouchPhase::*;
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
                            if dx > 0.0 { Dir::Right } else { Dir::Left }
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
        self.touch = None;
        self.touch_dir = Dir::None;
    }

    pub fn set_pad(&mut self, pad: Option<PadState>) {
        self.pad = pad;
    }

    /// Reads the controller; called once per fixed update so stick aiming is frame-rate independent.
    pub fn poll(&mut self, dt: f64) {
        self.pad_aim_age += dt;
        let Some(pad) = self.pad.clone() else {
            for a in self.pad_held.drain() {
                self.pressed.remove(&a);
            }
            self.pad_dir = Dir::None;
            return;
        };
        self.pad_connected = true;
        for &a in &pad.actions {
            if !self.pad_held.contains(&a) {
                self.just.insert(a);
            }
            self.pressed.insert(a);
        }
        for a in self.pad_held.iter() {
            if !pad.actions.contains(a) {
                self.pressed.remove(a);
            }
        }
        self.pad_held = pad.actions.clone();

        // Left stick, or the d-pad, chooses the corridor.
        let mut dir = pad.dpad.unwrap_or(Dir::None);
        if dir == Dir::None {
            let (x, y) = pad.left;
            if x.abs().max(y.abs()) >= MOVE_DEADZONE {
                dir = if x.abs() > y.abs() {
                    if x > 0.0 { Dir::Right } else { Dir::Left }
                } else if y > 0.0 {
                    Dir::Down
                } else {
                    Dir::Up
                };
            }
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

    /// Removes a press so nothing else reacts to it this update.
    pub fn consume(&mut self, a: Action) {
        self.just.remove(&a);
    }

    pub fn flush(&mut self) {
        self.just.clear();
        self.click = None;
        self.nav = None;
        self.cursor_moved = false;
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
