//! The action map (UI.md §3): which controls stand for which actions, as data. Two layers: play,
//! which the player may rebind and which is saved (`vh.bindings`), and the menus', fixed. The
//! testing keys (F1-F10, F12, 0) and F11 sit outside both and never move. Besides the map: the devices
//! a prompt can be drawn for, the names of keys and buttons, and the tokens prompt strings use.

use std::borrow::Cow;

use winit::keyboard::KeyCode;

use super::Action;
use crate::platform;

/// The kind of device used last, for the prompts' glyphs.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Device {
    /// Keyboard and mouse (and touch).
    #[default]
    KeyboardMouse,
    Xbox,
    PlayStation,
    /// A controller of no family we know: drawn with the Xbox layout's positions.
    Generic,
}

impl Device {
    /// A controller of any family.
    pub fn is_pad(self) -> bool {
        self != Device::KeyboardMouse
    }

    /// Its short name for a capture script (`device:kbm|xbox|ps|pad`).
    pub fn from_name(s: &str) -> Option<Device> {
        Some(match s {
            "kbm" | "keyboard" | "mouse" => Device::KeyboardMouse,
            "xbox" => Device::Xbox,
            "ps" | "playstation" => Device::PlayStation,
            "pad" | "generic" => Device::Generic,
            _ => return None,
        })
    }
}

/// A controller's buttons, by position (Xbox names: South is A, Cross on a PlayStation pad).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PadButton {
    South,
    East,
    West,
    North,
    LeftBumper,
    RightBumper,
    /// The analog triggers read as buttons past a threshold (`trigger_latch`).
    LeftTrigger,
    RightTrigger,
    /// The sticks clicked (L3, R3).
    LeftStick,
    RightStick,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    View,
    Menu,
}

impl PadButton {
    pub const ALL: [PadButton; 16] = [
        PadButton::South,
        PadButton::East,
        PadButton::West,
        PadButton::North,
        PadButton::LeftBumper,
        PadButton::RightBumper,
        PadButton::LeftTrigger,
        PadButton::RightTrigger,
        PadButton::LeftStick,
        PadButton::RightStick,
        PadButton::DPadUp,
        PadButton::DPadDown,
        PadButton::DPadLeft,
        PadButton::DPadRight,
        PadButton::View,
        PadButton::Menu,
    ];

    /// The D-pad's button for a direction.
    pub fn dpad(d: crate::logic::types::Dir) -> Option<PadButton> {
        use crate::logic::types::Dir;
        Some(match d {
            Dir::Up => PadButton::DPadUp,
            Dir::Down => PadButton::DPadDown,
            Dir::Left => PadButton::DPadLeft,
            Dir::Right => PadButton::DPadRight,
            Dir::None => return None,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

impl MouseButton {
    pub fn from_winit(b: winit::event::MouseButton) -> Option<MouseButton> {
        use winit::event::MouseButton as W;
        Some(match b {
            W::Left => MouseButton::Left,
            W::Right => MouseButton::Right,
            W::Middle => MouseButton::Middle,
            W::Back => MouseButton::Back,
            W::Forward => MouseButton::Forward,
            W::Other(_) => return None,
        })
    }
}

/// One control: a key, a mouse button, a notch of the wheel, or a controller button.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Control {
    Key(KeyCode),
    Mouse(MouseButton),
    WheelUp,
    WheelDown,
    Pad(PadButton),
}

impl Control {
    /// On a controller (and not the keyboard or the mouse).
    pub fn is_pad(self) -> bool {
        matches!(self, Control::Pad(_))
    }

    /// Its name in the saved bindings: the key's own name ("KeyW", "Space"), "MouseLeft",
    /// "WheelUp", "PadSouth".
    pub fn token(self) -> String {
        match self {
            Control::Key(k) => format!("{k:?}"),
            Control::Mouse(b) => format!("Mouse{b:?}"),
            Control::WheelUp => "WheelUp".into(),
            Control::WheelDown => "WheelDown".into(),
            Control::Pad(b) => format!("Pad{b:?}"),
        }
    }

    /// The control a saved name stands for (keys from the keys that can be bound, `KEYS`).
    pub fn from_token(s: &str) -> Option<Control> {
        match s {
            "WheelUp" => return Some(Control::WheelUp),
            "WheelDown" => return Some(Control::WheelDown),
            _ => {}
        }
        if let Some(b) = s.strip_prefix("Mouse") {
            return [MouseButton::Left, MouseButton::Right, MouseButton::Middle, MouseButton::Back, MouseButton::Forward]
                .into_iter()
                .find(|m| format!("{m:?}") == b)
                .map(Control::Mouse);
        }
        if let Some(b) = s.strip_prefix("Pad") {
            return PadButton::ALL.into_iter().find(|p| format!("{p:?}") == b).map(Control::Pad);
        }
        KEYS.iter().find(|k| format!("{k:?}") == s).map(|&k| Control::Key(k))
    }
}

/// Which layer the controls are read through: play's, or the menus' (the game sets it each
/// update from the screen and the pages open over it; presentation only).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Context {
    Play,
    /// The title, the pause menu and the pages over it, the old man's wares, the story, the
    /// choices: everything that is not the hero moving.
    #[default]
    Menu,
}

/// Keys the player may bind (the testing keys and F11 are not among them).
pub const KEYS: &[KeyCode] = {
    use KeyCode::*;
    &[
        KeyA,
        KeyB,
        KeyC,
        KeyD,
        KeyE,
        KeyF,
        KeyG,
        KeyH,
        KeyI,
        KeyJ,
        KeyK,
        KeyL,
        KeyM,
        KeyN,
        KeyO,
        KeyP,
        KeyQ,
        KeyR,
        KeyS,
        KeyT,
        KeyU,
        KeyV,
        KeyW,
        KeyX,
        KeyY,
        KeyZ,
        Digit1,
        Digit2,
        Digit3,
        Digit4,
        Digit5,
        Digit6,
        Digit7,
        Digit8,
        Digit9,
        Space,
        Enter,
        Tab,
        Escape,
        Backspace,
        ShiftLeft,
        ShiftRight,
        ControlLeft,
        ControlRight,
        AltLeft,
        AltRight,
        CapsLock,
        ArrowUp,
        ArrowDown,
        ArrowLeft,
        ArrowRight,
        Comma,
        Period,
        Slash,
        Semicolon,
        Quote,
        BracketLeft,
        BracketRight,
        Backslash,
        Minus,
        Equal,
        Backquote,
        Insert,
        Delete,
        Home,
        End,
        PageUp,
        PageDown,
        Numpad1,
        Numpad2,
        Numpad3,
        Numpad4,
        Numpad5,
        Numpad6,
        Numpad7,
        Numpad8,
        Numpad9,
        NumpadEnter,
        NumpadAdd,
        NumpadSubtract,
        NumpadMultiply,
        NumpadDivide,
        NumpadDecimal,
    ]
};

/// The keys that never move: the testing keys (F1-F10, 0; F12 the developer's map) and F11 for the
/// full screen. (Ctrl+M,
/// mute, is read before the layers too: see `Input::key_down`.)
pub fn fixed_key(code: KeyCode) -> Option<Action> {
    use KeyCode::*;
    Some(match code {
        F1 => Action::Cheat,
        F2 => Action::KeyCheat,
        F3 => Action::Fps,
        F4 => Action::FreeMove,
        F5 => Action::Immortal,
        F6 => Action::Gold,
        F7 => Action::NextStageBout,
        F8 => Action::Readout,
        F9 => Action::NextBout,
        F10 => Action::MakeRival,
        F11 => Action::Fullscreen,
        F12 => Action::DevMap,
        Digit0 | Numpad0 => Action::ShopCheat,
        _ => return None,
    })
}

/// A control the player may not take: one of the fixed keys.
pub fn reserved(c: Control) -> bool {
    matches!(c, Control::Key(k) if fixed_key(k).is_some())
}

/// The play layer's actions, in the order a controls page lists them.
pub const PLAY: [Action; 25] = [
    Action::MoveUp,
    Action::MoveDown,
    Action::MoveLeft,
    Action::MoveRight,
    Action::Attack,
    Action::Jump,
    Action::Block,
    Action::Step,
    Action::Angel,
    Action::Drink,
    Action::PotionPrev,
    Action::PotionNext,
    Action::Slot(0),
    Action::Slot(1),
    Action::Slot(2),
    Action::Slot(3),
    Action::Slot(4),
    Action::Call,
    Action::Guide,
    Action::Map,
    Action::Camera,
    Action::CameraReset,
    Action::MinimapZoom,
    Action::Pause,
    // (Confirm in play: Enter, for whatever asks for it while he moves; nothing does yet.)
    Action::Confirm,
];

/// The play layer's defaults (UI.md §3): keyboard and mouse first, then the controller. The left
/// stick runs and the right stick turns the camera: they are not buttons, and are not bound here.
pub fn default_controls(a: Action) -> &'static [Control] {
    use Control::{Key, Mouse, Pad};
    use KeyCode::*;
    match a {
        Action::MoveUp => &[Key(KeyW), Key(ArrowUp)],
        Action::MoveDown => &[Key(KeyS), Key(ArrowDown)],
        Action::MoveLeft => &[Key(KeyA), Key(ArrowLeft)],
        Action::MoveRight => &[Key(KeyD), Key(ArrowRight)],
        Action::Attack => &[Mouse(MouseButton::Left), Pad(PadButton::West)],
        Action::Jump => &[Key(Space), Pad(PadButton::South)],
        Action::Block => &[Mouse(MouseButton::Right), Pad(PadButton::LeftBumper)],
        Action::Step => &[Key(ShiftLeft), Pad(PadButton::LeftTrigger)],
        Action::Angel => &[Key(KeyR), Pad(PadButton::RightTrigger)],
        Action::Drink => &[Key(KeyQ), Pad(PadButton::RightBumper)],
        Action::PotionPrev => &[Control::WheelUp, Pad(PadButton::DPadLeft)],
        Action::PotionNext => &[Control::WheelDown, Pad(PadButton::DPadRight)],
        Action::Slot(0) => &[Key(Digit1)],
        Action::Slot(1) => &[Key(Digit2)],
        Action::Slot(2) => &[Key(Digit3)],
        Action::Slot(3) => &[Key(Digit4)],
        Action::Slot(4) => &[Key(Digit5)],
        Action::Call => &[Key(KeyE), Pad(PadButton::North)],
        Action::Guide => &[Key(KeyB), Pad(PadButton::East)],
        Action::Map => &[Key(KeyM), Key(Tab), Pad(PadButton::View), Pad(PadButton::DPadUp)],
        Action::Camera => &[Key(KeyC), Pad(PadButton::DPadDown)],
        Action::CameraReset => &[Pad(PadButton::RightStick)],
        Action::MinimapZoom => &[Pad(PadButton::LeftStick)],
        Action::Pause => &[Key(Escape), Pad(PadButton::Menu)],
        Action::Confirm => &[Key(Enter), Key(NumpadEnter)],
        _ => &[],
    }
}

/// The menus' layer (fixed): the D-pad, the arrows and WASD move the highlight (the left stick
/// too, read as a stick); A, Enter or Space selects; B, Menu or Esc goes back; LB and RB, Q and E
/// (or Tab) change tabs; Y is the secondary; B and M (and View) put away the page they opened.
pub const MENU: &[(Action, &[Control])] = {
    use Control::{Key, Pad};
    use KeyCode::*;
    &[
        (Action::NavUp, &[Key(ArrowUp), Key(KeyW), Pad(PadButton::DPadUp), Control::WheelUp]),
        (Action::NavDown, &[Key(ArrowDown), Key(KeyS), Pad(PadButton::DPadDown), Control::WheelDown]),
        (Action::NavLeft, &[Key(ArrowLeft), Key(KeyA), Pad(PadButton::DPadLeft)]),
        (Action::NavRight, &[Key(ArrowRight), Key(KeyD), Pad(PadButton::DPadRight)]),
        (Action::Confirm, &[Key(Enter), Key(NumpadEnter), Key(Space), Pad(PadButton::South)]),
        (Action::Back, &[Key(Escape), Pad(PadButton::East), Pad(PadButton::Menu)]),
        (Action::TabPrev, &[Key(KeyQ), Pad(PadButton::LeftBumper)]),
        (Action::TabNext, &[Key(KeyE), Key(Tab), Pad(PadButton::RightBumper)]),
        (Action::Secondary, &[Key(KeyR), Pad(PadButton::North)]),
        (Action::Guide, &[Key(KeyB)]),
        (Action::Map, &[Key(KeyM), Pad(PadButton::View)]),
    ]
};

/// The play layer as the player has it: each play action's controls, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bindings {
    play: Vec<(Action, Vec<Control>)>,
}

impl Default for Bindings {
    fn default() -> Self {
        Self { play: PLAY.iter().map(|&a| (a, default_controls(a).to_vec())).collect() }
    }
}

/// How a rebind settles a control already on other actions.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Resolve {
    /// Change nothing: report the conflict (the page asks the player).
    Ask,
    /// The others get the control the slot had, in its place.
    Swap,
    /// The others lose it.
    Clear,
}

/// What a rebind did.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Rebound {
    Done,
    /// It is on these other actions: nothing changed (`Resolve::Ask`).
    Conflict(Vec<Action>),
    /// It was on these other actions, and they were settled as asked.
    Resolved(Vec<Action>),
    /// A fixed key, and what it does (if anything).
    Fixed(Option<Action>),
    /// A control of the other side, or not a play action.
    WrongSide,
}

/// The settings key, and the version of the saved form (`v1;Attack=MouseLeft,PadWest;...`).
pub const BINDINGS_KEY: &str = "vh.bindings";
const VERSION: &str = "v1";

impl Bindings {
    fn entry(&self, a: Action) -> Option<&Vec<Control>> {
        self.play.iter().find(|(b, _)| *b == a).map(|(_, c)| c)
    }

    fn entry_mut(&mut self, a: Action) -> Option<&mut Vec<Control>> {
        self.play.iter_mut().find(|(b, _)| *b == a).map(|(_, c)| c)
    }

    /// Every control bound to `action` (play layer), in order.
    pub fn controls(&self, action: Action) -> Vec<Control> {
        self.entry(action).cloned().unwrap_or_default()
    }

    /// The control to show for `action` on `device` (the first bound of that kind), if any: the
    /// play layer's for a play action, the menus' for a menu action (Confirm on a controller,
    /// Back, the tabs, the secondary).
    pub fn shown(&self, action: Action, device: Device) -> Option<Control> {
        let fits = |c: &&Control| c.is_pad() == device.is_pad();
        if let Some(c) = self.entry(action).and_then(|cs| cs.iter().find(fits)) {
            return Some(*c);
        }
        MENU.iter().filter(|(a, _)| *a == action).flat_map(|(_, cs)| cs.iter()).find(fits).copied()
    }

    /// The actions `control` stands for in `context`.
    pub fn actions(&self, control: Control, context: Context) -> Vec<Action> {
        match context {
            Context::Play => self.play.iter().filter(|(_, cs)| cs.contains(&control)).map(|(a, _)| *a).collect(),
            Context::Menu => MENU.iter().filter(|(_, cs)| cs.contains(&control)).map(|(a, _)| *a).collect(),
        }
    }

    /// The other play actions already on `control` (a rebind would share it with them).
    pub fn conflicts(&self, action: Action, control: Control) -> Vec<Action> {
        self.play.iter().filter(|(a, cs)| *a != action && cs.contains(&control)).map(|(a, _)| *a).collect()
    }

    /**
     * Adds `control` to `action`, after its others, and reports the conflicts: the other play
     * actions on it, which keep it (the caller unbinds them, or lets them share). A fixed key
     * (`reserved`) is not taken: what it stands for is reported instead. Not a play action:
     * nothing changes.
     */
    pub fn bind(&mut self, action: Action, control: Control) -> Vec<Action> {
        if reserved(control) {
            return match control {
                Control::Key(k) => fixed_key(k).into_iter().collect(),
                _ => Vec::new(),
            };
        }
        let conflicts = self.conflicts(action, control);
        if let Some(cs) = self.entry_mut(action) {
            if !cs.contains(&control) {
                cs.push(control);
            }
        }
        conflicts
    }

    /// Takes `control` off `action`.
    pub fn unbind(&mut self, action: Action, control: Control) {
        if let Some(cs) = self.entry_mut(action) {
            cs.retain(|&c| c != control);
        }
    }

    /// The controls of `action` on one side, in order: the controller's (`pad`), or the keyboard
    /// and mouse's.
    pub fn side(&self, action: Action, pad: bool) -> Vec<Control> {
        self.controls(action).into_iter().filter(|c| c.is_pad() == pad).collect()
    }

    /// Takes the control in slot `slot` of `action`'s side off it, and says what it was.
    pub fn clear_slot(&mut self, action: Action, pad: bool, slot: usize) -> Option<Control> {
        let c = *self.side(action, pad).get(slot)?;
        self.unbind(action, c);
        Some(c)
    }

    /**
     * The rebinding page's bind: `control` into slot `slot` of `action`'s side (in place of what
     * was there, or after the others). A fixed key is refused (naming what it does), and so is a
     * control of the other side. A control already on another play action is a conflict: `Ask`
     * changes nothing and names them; `Swap` gives each of them the control this slot had in its
     * place (or nothing, when the slot was empty); `Clear` takes it from them. A control already
     * in another of the action's own slots changes places with this one.
     */
    pub fn rebind(&mut self, action: Action, pad: bool, slot: usize, control: Control, resolve: Resolve) -> Rebound {
        if reserved(control) {
            return Rebound::Fixed(match control {
                Control::Key(k) => fixed_key(k),
                _ => None,
            });
        }
        if control.is_pad() != pad || !PLAY.contains(&action) {
            return Rebound::WrongSide;
        }
        let side = self.side(action, pad);
        let old = side.get(slot).copied();
        if old == Some(control) {
            return Rebound::Done;
        }
        let others = self.conflicts(action, control);
        if !others.is_empty() {
            match resolve {
                Resolve::Ask => return Rebound::Conflict(others),
                Resolve::Swap => {
                    for &o in &others {
                        if let Some(cs) = self.entry_mut(o) {
                            match old {
                                // (Where the other has `old` already, it simply loses `control`.)
                                Some(prev) if !cs.contains(&prev) => {
                                    for c in cs.iter_mut().filter(|c| **c == control) {
                                        *c = prev;
                                    }
                                }
                                _ => cs.retain(|&c| c != control),
                            }
                        }
                    }
                }
                Resolve::Clear => {
                    for &o in &others {
                        self.unbind(o, control);
                    }
                }
            }
        }
        let Some(cs) = self.entry_mut(action) else { return Rebound::WrongSide };
        let at = |cs: &Vec<Control>, c: Control| cs.iter().position(|&x| x == c);
        match (old, at(cs, control)) {
            // Already in another of its own slots: the two change places.
            (Some(prev), Some(j)) => {
                let i = at(cs, prev).unwrap_or(j);
                cs.swap(i, j);
            }
            (None, Some(_)) => {}
            (Some(prev), None) => {
                if let Some(i) = at(cs, prev) {
                    cs[i] = control;
                }
            }
            (None, None) => cs.push(control),
        }
        if others.is_empty() {
            Rebound::Done
        } else {
            Rebound::Resolved(others)
        }
    }

    /// Every play action back to its defaults.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// One play action back to its defaults.
    pub fn reset_action(&mut self, action: Action) {
        if let Some(cs) = self.entry_mut(action) {
            *cs = default_controls(action).to_vec();
        }
    }

    /// The saved form: the version, then one `action=control,control` entry each.
    pub fn to_setting(&self) -> String {
        let mut s = String::from(VERSION);
        for (a, cs) in &self.play {
            s.push(';');
            s.push_str(token(*a));
            s.push('=');
            s.push_str(&cs.iter().map(|c| c.token()).collect::<Vec<_>>().join(","));
        }
        s
    }

    /**
     * Read back from the saved form, forgiving: another version, or garbage, is the defaults; an
     * action not saved, or saved with nothing that can still be read, keeps its defaults; an
     * action saved empty stays unbound; unknown actions and controls, fixed keys and repeats are
     * passed over (the first entry for an action counts).
     */
    pub fn from_setting(s: &str) -> Self {
        let mut b = Self::default();
        let mut parts = s.trim().split(';');
        if parts.next() != Some(VERSION) {
            return b;
        }
        let mut seen = Vec::new();
        for part in parts {
            let Some((name, list)) = part.split_once('=') else { continue };
            let Some(a) = from_token(name.trim()).filter(|a| PLAY.contains(a)) else { continue };
            if seen.contains(&a) {
                continue;
            }
            seen.push(a);
            let names: Vec<&str> = list.split(',').map(str::trim).filter(|n| !n.is_empty()).collect();
            let mut cs: Vec<Control> = Vec::new();
            for c in names.iter().filter_map(|n| Control::from_token(n)).filter(|&c| !reserved(c)) {
                if !cs.contains(&c) {
                    cs.push(c);
                }
            }
            if names.is_empty() || !cs.is_empty() {
                if let Some(slot) = b.entry_mut(a) {
                    *slot = cs;
                }
            }
        }
        b
    }

    /// As saved (tests keep to the defaults).
    pub fn load() -> Self {
        if cfg!(test) {
            return Self::default();
        }
        platform::load(BINDINGS_KEY).map(|s| Self::from_setting(&s)).unwrap_or_default()
    }

    /// Saved, for the rebinding page (never from tests; captures save into their own folder).
    pub fn save(&self) {
        if cfg!(test) {
            return;
        }
        platform::save(BINDINGS_KEY, &self.to_setting());
    }
}

/// Past this the analog trigger reads as pressed; it lets go only under `TRIGGER_RELEASE`, so a
/// finger resting near the threshold does not chatter.
pub const TRIGGER_PRESS: f32 = 0.35;
pub const TRIGGER_RELEASE: f32 = 0.25;

/// A trigger as a button: pressed now, given whether it was and how far it is pulled (0 to 1).
pub fn trigger_latch(was: bool, value: f32) -> bool {
    if was {
        value > TRIGGER_RELEASE
    } else {
        value >= TRIGGER_PRESS
    }
}

/// A controller's family, from its USB vendor and its name.
pub fn pad_kind(vendor: Option<u16>, name: &str) -> Device {
    let n = name.to_ascii_lowercase();
    if vendor == Some(0x054c) || ["playstation", "dualsense", "dualshock", "ps4", "ps5"].iter().any(|k| n.contains(k)) {
        Device::PlayStation
    } else if vendor == Some(0x045e) || n.contains("xbox") || n.contains("xinput") {
        Device::Xbox
    } else {
        Device::Generic
    }
}

/// A key's short name for a keycap: "W", "Space", "Shift", "Esc", "Tab", "1", "F11", "↑"...
pub fn key_name(code: KeyCode) -> Cow<'static, str> {
    use KeyCode::*;
    let name: &'static str = match code {
        Space => "Space",
        Enter | NumpadEnter => "Enter",
        Tab => "Tab",
        Escape => "Esc",
        Backspace => "Backspace",
        ShiftLeft | ShiftRight => "Shift",
        ControlLeft | ControlRight => "Ctrl",
        AltLeft | AltRight => "Alt",
        CapsLock => "Caps",
        ArrowUp => "↑",
        ArrowDown => "↓",
        ArrowLeft => "←",
        ArrowRight => "→",
        Comma => ",",
        Period => ".",
        Slash => "/",
        Semicolon => ";",
        Quote => "'",
        BracketLeft => "[",
        BracketRight => "]",
        Backslash => "\\",
        Minus => "-",
        Equal => "=",
        Backquote => "`",
        Insert => "Ins",
        Delete => "Del",
        Home => "Home",
        End => "End",
        PageUp => "PgUp",
        PageDown => "PgDn",
        NumpadAdd => "Num +",
        NumpadSubtract => "Num -",
        NumpadMultiply => "Num *",
        NumpadDivide => "Num /",
        NumpadDecimal => "Num .",
        _ => {
            let raw = format!("{code:?}");
            for prefix in ["Key", "Digit"] {
                if let Some(rest) = raw.strip_prefix(prefix) {
                    return Cow::Owned(rest.to_string());
                }
            }
            if let Some(rest) = raw.strip_prefix("Numpad") {
                return Cow::Owned(format!("Num {rest}"));
            }
            return Cow::Owned(raw);
        }
    };
    Cow::Borrowed(name)
}

/// A control's name in words, for text that names it ("Left click", "RT", "R2", "Shift"); a
/// controller's buttons in the family's own names (the Xbox names for one we do not know).
pub fn control_name(c: Control, device: Device) -> Cow<'static, str> {
    let ps = device == Device::PlayStation;
    let name: &'static str = match c {
        Control::Key(k) => return key_name(k),
        Control::Mouse(MouseButton::Left) => "Left click",
        Control::Mouse(MouseButton::Right) => "Right click",
        Control::Mouse(MouseButton::Middle) => "Middle click",
        Control::Mouse(MouseButton::Back) => "Mouse 4",
        Control::Mouse(MouseButton::Forward) => "Mouse 5",
        Control::WheelUp => "Wheel up",
        Control::WheelDown => "Wheel down",
        Control::Pad(b) => match b {
            PadButton::South if ps => "Cross",
            PadButton::East if ps => "Circle",
            PadButton::West if ps => "Square",
            PadButton::North if ps => "Triangle",
            PadButton::LeftBumper if ps => "L1",
            PadButton::RightBumper if ps => "R1",
            PadButton::LeftTrigger if ps => "L2",
            PadButton::RightTrigger if ps => "R2",
            PadButton::View if ps => "Share",
            PadButton::Menu if ps => "Options",
            PadButton::South => "A",
            PadButton::East => "B",
            PadButton::West => "X",
            PadButton::North => "Y",
            PadButton::LeftBumper => "LB",
            PadButton::RightBumper => "RB",
            PadButton::LeftTrigger => "LT",
            PadButton::RightTrigger => "RT",
            PadButton::View => "View",
            PadButton::Menu => "Menu",
            PadButton::LeftStick => "L3",
            PadButton::RightStick => "R3",
            PadButton::DPadUp => "D-pad ↑",
            PadButton::DPadDown => "D-pad ↓",
            PadButton::DPadLeft => "D-pad ←",
            PadButton::DPadRight => "D-pad →",
        },
    };
    Cow::Borrowed(name)
}

/// A short English label for an action, for the controls and settings pages.
pub fn action_name(a: Action) -> &'static str {
    match a {
        Action::MoveUp => "Run up",
        Action::MoveDown => "Run down",
        Action::MoveLeft => "Run left",
        Action::MoveRight => "Run right",
        Action::Attack => "Attack",
        Action::Jump => "Jump",
        Action::Block => "Shield",
        Action::Step => "Bifrost Step",
        Action::Angel => "Archangel",
        Action::Drink => "Drink",
        Action::PotionPrev => "Previous potion",
        Action::PotionNext => "Next potion",
        Action::Slot(0) => "Potion 1",
        Action::Slot(1) => "Potion 2",
        Action::Slot(2) => "Potion 3",
        Action::Slot(3) => "Potion 4",
        Action::Slot(_) => "Potion 5",
        Action::Call => "Call the old man",
        Action::Guide => "Guide",
        Action::Map => "Map",
        Action::Camera => "Camera",
        Action::CameraReset => "Recentre",
        Action::MinimapZoom => "Minimap zoom",
        Action::Pause => "Pause",
        Action::Confirm => "Select",
        Action::Tap => "Tap",
        Action::Back => "Back",
        Action::TabPrev => "Previous tab",
        Action::TabNext => "Next tab",
        Action::Secondary => "Secondary",
        Action::NavUp => "Up",
        Action::NavDown => "Down",
        Action::NavLeft => "Left",
        Action::NavRight => "Right",
        Action::Mute => "Mute",
        Action::Fullscreen => "Full screen",
        Action::Fps => "Frame rate",
        Action::Cheat => "To the boss",
        Action::KeyCheat => "Every key",
        Action::FreeMove => "Free movement",
        Action::ShopCheat => "To the old man",
        Action::Readout => "Frame readout",
        Action::Immortal => "Immortal",
        Action::MakeRival => "Make a rival",
        Action::NextBout => "Next bout",
        Action::Gold => "A thousand coins",
        Action::NextStageBout => "Next stage's bout",
        Action::DevMap => "Developer's map",
    }
}

/// Every action and its token, for prompt strings and the saved bindings (stable: never renamed).
const TOKENS: [(Action, &str); 48] = [
    (Action::MoveUp, "MoveUp"),
    (Action::MoveDown, "MoveDown"),
    (Action::MoveLeft, "MoveLeft"),
    (Action::MoveRight, "MoveRight"),
    (Action::Attack, "Attack"),
    (Action::Jump, "Jump"),
    (Action::Block, "Block"),
    (Action::Step, "Step"),
    (Action::Angel, "Angel"),
    (Action::Drink, "Drink"),
    (Action::PotionPrev, "PotionPrev"),
    (Action::PotionNext, "PotionNext"),
    (Action::Slot(0), "Slot1"),
    (Action::Slot(1), "Slot2"),
    (Action::Slot(2), "Slot3"),
    (Action::Slot(3), "Slot4"),
    (Action::Slot(4), "Slot5"),
    (Action::Call, "Call"),
    (Action::Guide, "Guide"),
    (Action::Map, "Map"),
    (Action::Camera, "Camera"),
    (Action::CameraReset, "CameraReset"),
    (Action::MinimapZoom, "MinimapZoom"),
    (Action::Pause, "Pause"),
    (Action::Confirm, "Confirm"),
    (Action::Tap, "Tap"),
    (Action::Back, "Back"),
    (Action::TabPrev, "TabPrev"),
    (Action::TabNext, "TabNext"),
    (Action::Secondary, "Secondary"),
    (Action::NavUp, "NavUp"),
    (Action::NavDown, "NavDown"),
    (Action::NavLeft, "NavLeft"),
    (Action::NavRight, "NavRight"),
    (Action::Mute, "Mute"),
    (Action::Fullscreen, "Fullscreen"),
    (Action::Fps, "Fps"),
    (Action::Cheat, "Cheat"),
    (Action::KeyCheat, "KeyCheat"),
    (Action::FreeMove, "FreeMove"),
    (Action::ShopCheat, "ShopCheat"),
    (Action::Readout, "Readout"),
    (Action::Immortal, "Immortal"),
    (Action::MakeRival, "MakeRival"),
    (Action::NextBout, "NextBout"),
    (Action::Gold, "Gold"),
    (Action::NextStageBout, "NextStageBout"),
    (Action::DevMap, "DevMap"),
];

/// An action's stable token ("Attack", "Jump", "Block", "Step", "Angel", "Confirm", "Back"...).
pub fn token(a: Action) -> &'static str {
    TOKENS.iter().find(|(b, _)| *b == a).map(|(_, t)| *t).unwrap_or("Unknown")
}

/// The action a token names.
pub fn from_token(s: &str) -> Option<Action> {
    TOKENS.iter().find(|(_, t)| *t == s).map(|(a, _)| *a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn play_controls(b: &Bindings, pad: bool) -> Vec<(Action, Control)> {
        PLAY.iter().flat_map(|&a| b.controls(a).into_iter().filter(move |c| c.is_pad() == pad).map(move |c| (a, c))).collect()
    }

    #[test]
    fn every_play_action_has_a_key_or_button_and_none_is_shared() {
        let b = Bindings::default();
        // The camera's recentre and the minimap's zoom are the sticks' clicks: on the keyboard
        // the mouse aims, so neither needs a key. The belt's slots are the number keys only, and
        // Confirm in play is Enter only.
        let pad_only = [Action::CameraReset, Action::MinimapZoom];
        let keys_only = [Action::Slot(0), Action::Slot(1), Action::Slot(2), Action::Slot(3), Action::Slot(4), Action::Confirm];
        for a in PLAY {
            let kbm = b.shown(a, Device::KeyboardMouse);
            let pad = b.shown(a, Device::Xbox);
            assert!(kbm.is_some() || pad_only.contains(&a), "{a:?} has no key");
            let moves = matches!(a, Action::MoveUp | Action::MoveDown | Action::MoveLeft | Action::MoveRight);
            assert!(pad.is_some() || keys_only.contains(&a) || moves, "{a:?} has no button (the stick runs)");
        }
        for pad in [false, true] {
            let all = play_controls(&b, pad);
            for (i, (a, c)) in all.iter().enumerate() {
                for (b2, c2) in &all[i + 1..] {
                    assert!(c != c2, "{c:?} is both {a:?} and {b2:?}");
                }
            }
        }
        for (a, c) in play_controls(&b, false).into_iter().chain(play_controls(&b, true)) {
            assert!(!reserved(c), "{a:?} on a fixed key");
            assert!(b.conflicts(a, c).is_empty());
        }
    }

    #[test]
    fn the_defaults_are_ui_md_section_three() {
        let b = Bindings::default();
        let pad = |a| b.shown(a, Device::Xbox);
        let key = |a| b.shown(a, Device::KeyboardMouse);
        use Control::{Key, Mouse, Pad};
        assert_eq!(pad(Action::Attack), Some(Pad(PadButton::West)));
        assert_eq!(pad(Action::Jump), Some(Pad(PadButton::South)));
        assert_eq!(pad(Action::Guide), Some(Pad(PadButton::East)));
        assert_eq!(pad(Action::Call), Some(Pad(PadButton::North)));
        assert_eq!(pad(Action::Block), Some(Pad(PadButton::LeftBumper)));
        assert_eq!(pad(Action::Drink), Some(Pad(PadButton::RightBumper)));
        assert_eq!(pad(Action::Step), Some(Pad(PadButton::LeftTrigger)));
        assert_eq!(pad(Action::Angel), Some(Pad(PadButton::RightTrigger)));
        assert_eq!(pad(Action::PotionPrev), Some(Pad(PadButton::DPadLeft)));
        assert_eq!(pad(Action::PotionNext), Some(Pad(PadButton::DPadRight)));
        assert_eq!(pad(Action::Map), Some(Pad(PadButton::View)));
        assert_eq!(pad(Action::Camera), Some(Pad(PadButton::DPadDown)));
        assert_eq!(pad(Action::CameraReset), Some(Pad(PadButton::RightStick)));
        assert_eq!(pad(Action::MinimapZoom), Some(Pad(PadButton::LeftStick)));
        assert_eq!(pad(Action::Pause), Some(Pad(PadButton::Menu)));
        assert_eq!(key(Action::Attack), Some(Mouse(MouseButton::Left)));
        assert_eq!(key(Action::Block), Some(Mouse(MouseButton::Right)));
        assert_eq!(key(Action::Jump), Some(Key(KeyCode::Space)));
        assert_eq!(key(Action::Step), Some(Key(KeyCode::ShiftLeft)));
        assert_eq!(key(Action::Angel), Some(Key(KeyCode::KeyR)));
        assert_eq!(key(Action::Drink), Some(Key(KeyCode::KeyQ)));
        assert_eq!(key(Action::PotionPrev), Some(Control::WheelUp));
        assert_eq!(key(Action::Call), Some(Key(KeyCode::KeyE)));
        assert_eq!(key(Action::Guide), Some(Key(KeyCode::KeyB)));
        assert_eq!(b.controls(Action::Map), vec![Key(KeyCode::KeyM), Key(KeyCode::Tab), Pad(PadButton::View), Pad(PadButton::DPadUp)]);
        assert_eq!(key(Action::Camera), Some(Key(KeyCode::KeyC)));
        assert_eq!(key(Action::Pause), Some(Key(KeyCode::Escape)));
        // The menus' own: A selects, B goes back, LB and RB page, Y the secondary.
        assert_eq!(pad(Action::Confirm), Some(Pad(PadButton::South)));
        assert_eq!(pad(Action::Back), Some(Pad(PadButton::East)));
        assert_eq!(key(Action::Back), Some(Key(KeyCode::Escape)));
        assert_eq!(pad(Action::TabPrev), Some(Pad(PadButton::LeftBumper)));
        assert_eq!(pad(Action::TabNext), Some(Pad(PadButton::RightBumper)));
        assert_eq!(pad(Action::Secondary), Some(Pad(PadButton::North)));
        assert_eq!(key(Action::Confirm), Some(Key(KeyCode::Enter)));
    }

    #[test]
    fn the_d_pad_is_the_belt_the_map_and_the_camera_in_play_and_moves_in_menus() {
        let b = Bindings::default();
        let play = |p| b.actions(Control::Pad(p), Context::Play);
        let menu = |p| b.actions(Control::Pad(p), Context::Menu);
        assert_eq!(play(PadButton::DPadLeft), vec![Action::PotionPrev]);
        assert_eq!(play(PadButton::DPadRight), vec![Action::PotionNext]);
        assert_eq!(play(PadButton::DPadUp), vec![Action::Map]);
        assert_eq!(play(PadButton::DPadDown), vec![Action::Camera]);
        assert_eq!(menu(PadButton::DPadLeft), vec![Action::NavLeft]);
        assert_eq!(menu(PadButton::DPadRight), vec![Action::NavRight]);
        assert_eq!(menu(PadButton::DPadUp), vec![Action::NavUp]);
        assert_eq!(menu(PadButton::DPadDown), vec![Action::NavDown]);
        assert_eq!(menu(PadButton::South), vec![Action::Confirm]);
        assert_eq!(menu(PadButton::East), vec![Action::Back]);
        assert_eq!(menu(PadButton::Menu), vec![Action::Back]);
        assert_eq!(play(PadButton::Menu), vec![Action::Pause]);
    }

    #[test]
    fn the_bindings_round_trip_through_the_settings() {
        let mut b = Bindings::default();
        assert_eq!(Bindings::from_setting(&b.to_setting()), b);
        b.unbind(Action::Attack, Control::Mouse(MouseButton::Left));
        b.bind(Action::Attack, Control::Key(KeyCode::KeyF));
        b.bind(Action::Jump, Control::Mouse(MouseButton::Back));
        b.bind(Action::Map, Control::Key(KeyCode::Backslash));
        for c in b.controls(Action::Camera) {
            b.unbind(Action::Camera, c);
        }
        let s = b.to_setting();
        assert!(s.starts_with("v1;"), "{s}");
        let back = Bindings::from_setting(&s);
        assert_eq!(back, b);
        assert!(back.controls(Action::Camera).is_empty(), "unbound stays unbound");
        assert_eq!(back.controls(Action::Attack), vec![Control::Pad(PadButton::West), Control::Key(KeyCode::KeyF)]);
    }

    #[test]
    fn old_or_broken_settings_fall_back_to_the_defaults_action_by_action() {
        let d = Bindings::default();
        for junk in ["", "garbage", "v0;Attack=KeyF", "v2;Attack=KeyF", ";;;", "v1", "v1;", "\u{1F600}=;", "v1;=;=,=;Attack"] {
            assert_eq!(Bindings::from_setting(junk), d, "{junk:?}");
        }
        // An unknown action is passed over; the rest are read.
        let b = Bindings::from_setting("v1;Fly=KeyF;Jump=KeyJ");
        assert_eq!(b.controls(Action::Jump), vec![Control::Key(KeyCode::KeyJ)]);
        assert_eq!(b.controls(Action::Attack), d.controls(Action::Attack));
        // Unknown controls are dropped; with none left that can be read, the defaults.
        let b = Bindings::from_setting("v1;Jump=KeyJ,Laser,PadZ;Attack=Laser");
        assert_eq!(b.controls(Action::Jump), vec![Control::Key(KeyCode::KeyJ)]);
        assert_eq!(b.controls(Action::Attack), d.controls(Action::Attack));
        // Repeats: the first entry for an action, and each control once.
        let b = Bindings::from_setting("v1;Jump=KeyJ,KeyJ,PadSouth;Jump=KeyK");
        assert_eq!(b.controls(Action::Jump), vec![Control::Key(KeyCode::KeyJ), Control::Pad(PadButton::South)]);
        // The fixed keys can never be taken, saved or not; menu and testing actions are not saved.
        let b = Bindings::from_setting("v1;Jump=F1,Digit0;Cheat=KeyJ;Confirm=KeyY");
        assert_eq!(b.controls(Action::Jump), d.controls(Action::Jump));
        assert!(b.controls(Action::Cheat).is_empty());
        assert_eq!(b.controls(Action::Confirm), vec![Control::Key(KeyCode::KeyY)]);
        // Whitespace about the entries is forgiven.
        let b = Bindings::from_setting(" v1; Jump = KeyJ , PadSouth ");
        assert_eq!(b.controls(Action::Jump), vec![Control::Key(KeyCode::KeyJ), Control::Pad(PadButton::South)]);
    }

    #[test]
    fn rebinding_reports_conflicts_and_resets() {
        let mut b = Bindings::default();
        assert_eq!(b.conflicts(Action::Attack, Control::Key(KeyCode::KeyQ)), vec![Action::Drink]);
        assert_eq!(b.bind(Action::Attack, Control::Key(KeyCode::KeyQ)), vec![Action::Drink]);
        assert!(b.controls(Action::Attack).contains(&Control::Key(KeyCode::KeyQ)));
        assert!(b.controls(Action::Drink).contains(&Control::Key(KeyCode::KeyQ)), "the other keeps it until unbound");
        assert_eq!(b.actions(Control::Key(KeyCode::KeyQ), Context::Play), vec![Action::Attack, Action::Drink]);
        b.unbind(Action::Drink, Control::Key(KeyCode::KeyQ));
        assert!(b.conflicts(Action::Attack, Control::Key(KeyCode::KeyQ)).is_empty());
        assert_eq!(b.shown(Action::Drink, Device::KeyboardMouse), None);
        // Binding twice adds once; a fixed key is refused and names its owner.
        b.bind(Action::Attack, Control::Key(KeyCode::KeyQ));
        assert_eq!(b.controls(Action::Attack).iter().filter(|&&c| c == Control::Key(KeyCode::KeyQ)).count(), 1);
        assert_eq!(b.bind(Action::Jump, Control::Key(KeyCode::F1)), vec![Action::Cheat]);
        assert!(!b.controls(Action::Jump).contains(&Control::Key(KeyCode::F1)));
        b.reset_action(Action::Drink);
        assert_eq!(b.controls(Action::Drink), default_controls(Action::Drink).to_vec());
        assert_ne!(b, Bindings::default());
        b.reset();
        assert_eq!(b, Bindings::default());
    }

    #[test]
    fn the_rebinding_page_binds_asks_swaps_clears_and_saves() {
        use Control::{Key, Mouse, Pad};
        let mut b = Bindings::default();
        // Bind: F into the attack's first keyboard slot, in place of the left click.
        assert_eq!(b.rebind(Action::Attack, false, 0, Key(KeyCode::KeyF), Resolve::Ask), Rebound::Done);
        assert_eq!(b.side(Action::Attack, false), vec![Key(KeyCode::KeyF)]);
        assert_eq!(b.side(Action::Attack, true), vec![Pad(PadButton::West)], "the pad's side untouched");
        // A second slot, empty until now: added after.
        assert_eq!(b.rebind(Action::Attack, false, 1, Mouse(MouseButton::Left), Resolve::Ask), Rebound::Done);
        assert_eq!(b.side(Action::Attack, false), vec![Key(KeyCode::KeyF), Mouse(MouseButton::Left)]);
        // A conflict: Q is the drink's. Asked, nothing changes (the page's cancel leaves it so).
        let before = b.clone();
        assert_eq!(b.rebind(Action::Attack, false, 0, Key(KeyCode::KeyQ), Resolve::Ask), Rebound::Conflict(vec![Action::Drink]));
        assert_eq!(b, before);
        // Swapped: the drink takes F, the attack Q.
        assert_eq!(b.rebind(Action::Attack, false, 0, Key(KeyCode::KeyQ), Resolve::Swap), Rebound::Resolved(vec![Action::Drink]));
        assert_eq!(b.side(Action::Attack, false), vec![Key(KeyCode::KeyQ), Mouse(MouseButton::Left)]);
        assert_eq!(b.side(Action::Drink, false), vec![Key(KeyCode::KeyF)]);
        assert!(b.conflicts(Action::Attack, Key(KeyCode::KeyQ)).is_empty());
        // Cleared: B is the guide's; the guide loses it and is left with only its button.
        assert_eq!(b.rebind(Action::Call, false, 0, Key(KeyCode::KeyB), Resolve::Clear), Rebound::Resolved(vec![Action::Guide]));
        assert_eq!(b.side(Action::Call, false), vec![Key(KeyCode::KeyB)]);
        assert!(b.side(Action::Guide, false).is_empty());
        assert_eq!(b.side(Action::Guide, true), vec![Pad(PadButton::East)]);
        // On the pad: Y onto the shield swaps with LB for the call.
        assert_eq!(b.rebind(Action::Block, true, 0, Pad(PadButton::North), Resolve::Swap), Rebound::Resolved(vec![Action::Call]));
        assert_eq!(b.side(Action::Block, true), vec![Pad(PadButton::North)]);
        assert_eq!(b.side(Action::Call, true), vec![Pad(PadButton::LeftBumper)]);
        // A swap into an empty slot leaves the other without it.
        assert_eq!(b.rebind(Action::Slot(0), true, 0, Pad(PadButton::RightBumper), Resolve::Swap), Rebound::Resolved(vec![Action::Drink]));
        assert!(b.side(Action::Drink, true).is_empty());
        // Its own other slot: the two change places.
        b.rebind(Action::Map, false, 1, Key(KeyCode::KeyM), Resolve::Ask);
        assert_eq!(b.side(Action::Map, false), vec![Key(KeyCode::Tab), Key(KeyCode::KeyM)]);
        // Refused: a fixed key, and a control of the other side.
        assert_eq!(b.rebind(Action::Jump, false, 0, Key(KeyCode::F2), Resolve::Swap), Rebound::Fixed(Some(Action::KeyCheat)));
        assert_eq!(b.rebind(Action::Jump, false, 0, Pad(PadButton::South), Resolve::Swap), Rebound::WrongSide);
        assert_eq!(b.rebind(Action::Back, false, 0, Key(KeyCode::KeyJ), Resolve::Swap), Rebound::WrongSide);
        // Cleared slot by slot.
        assert_eq!(b.clear_slot(Action::Attack, false, 1), Some(Mouse(MouseButton::Left)));
        assert_eq!(b.clear_slot(Action::Attack, false, 5), None);
        // Saved and read back, through the settings form and through the settings file itself.
        let back = Bindings::from_setting(&b.to_setting());
        assert_eq!(back, b);
        let dir = std::env::temp_dir().join(format!("vh-rebind-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        platform::use_dir_on_this_thread(dir.clone());
        platform::save(BINDINGS_KEY, &b.to_setting());
        assert_eq!(platform::load(BINDINGS_KEY).map(|s| Bindings::from_setting(&s)), Some(b.clone()));
        let _ = std::fs::remove_dir_all(&dir);
        // Reset one, then all.
        b.reset_action(Action::Attack);
        assert_eq!(b.controls(Action::Attack), default_controls(Action::Attack).to_vec());
        b.reset();
        assert_eq!(b, Bindings::default());
    }

    #[test]
    fn the_triggers_read_as_buttons_with_hysteresis() {
        let mut held = false;
        let mut seen = Vec::new();
        for v in [0.0, 0.2, 0.34, 0.35, 0.3, 0.26, 0.25, 0.3, 0.34, 0.9, 0.0] {
            held = trigger_latch(held, v);
            seen.push(held);
        }
        assert_eq!(seen, [false, false, false, true, true, true, false, false, false, true, false]);
    }

    #[test]
    fn pads_are_told_apart_by_vendor_and_name() {
        assert_eq!(pad_kind(Some(0x054c), "Wireless Controller"), Device::PlayStation);
        assert_eq!(pad_kind(None, "DualSense Wireless Controller"), Device::PlayStation);
        assert_eq!(pad_kind(None, "PS4 Controller"), Device::PlayStation);
        assert_eq!(pad_kind(Some(0x045e), "Controller"), Device::Xbox);
        assert_eq!(pad_kind(None, "Xbox Series X Controller"), Device::Xbox);
        assert_eq!(pad_kind(None, "XInput Controller #1"), Device::Xbox);
        assert_eq!(pad_kind(Some(0x057e), "Pro Controller"), Device::Generic);
        assert!(Device::Generic.is_pad() && !Device::KeyboardMouse.is_pad());
    }

    #[test]
    fn names_and_tokens() {
        assert_eq!(key_name(KeyCode::KeyW), "W");
        assert_eq!(key_name(KeyCode::Space), "Space");
        assert_eq!(key_name(KeyCode::ShiftLeft), "Shift");
        assert_eq!(key_name(KeyCode::Escape), "Esc");
        assert_eq!(key_name(KeyCode::Digit1), "1");
        assert_eq!(key_name(KeyCode::F11), "F11");
        assert_eq!(key_name(KeyCode::ArrowUp), "↑");
        assert_eq!(control_name(Control::Pad(PadButton::RightTrigger), Device::Xbox), "RT");
        assert_eq!(control_name(Control::Pad(PadButton::RightTrigger), Device::PlayStation), "R2");
        assert_eq!(control_name(Control::Pad(PadButton::South), Device::PlayStation), "Cross");
        for (a, t) in TOKENS {
            assert_eq!(from_token(t), Some(a));
            assert_eq!(token(a), t);
            assert!(!action_name(a).is_empty());
        }
        for &k in KEYS {
            assert_eq!(Control::from_token(&Control::Key(k).token()), Some(Control::Key(k)), "{k:?}");
            assert!(fixed_key(k).is_none(), "{k:?} is fixed");
        }
        for p in PadButton::ALL {
            assert_eq!(Control::from_token(&Control::Pad(p).token()), Some(Control::Pad(p)));
        }
        assert_eq!(Control::from_token("MouseRight"), Some(Control::Mouse(MouseButton::Right)));
        assert_eq!(Control::from_token("WheelDown"), Some(Control::WheelDown));
        assert_eq!(Control::from_token("F1"), None, "the fixed keys are not bindable");
    }
}
