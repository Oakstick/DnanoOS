//! The controller legend (UI.md §9), as data both loading screens draw: the engine's start screen
//! on its own thread with its own raster (`splash`), and the in-game loading card with the
//! overlay (`hud::legend`). Here: the gamepad's shape and where its buttons sit (in pad units, 20
//! across), what each control does from the bindings (the pad's callouts and the keyboard and
//! mouse's rows), and the old man's tips.

use crate::input::{action_name, Action, Bindings, Context, Control, Device, MouseButton, PadButton};

/// Pad units: the pad is drawn about 20 across and 13 tall about its centre, y down.
/// The body: round lobes and grips (centre, radius), and the slab between them (x, y, w, h).
pub const BODY_CIRCLES: [(f32, f32, f32); 6] =
    [(-5.6, -1.4, 3.4), (5.6, -1.4, 3.4), (-6.3, 2.4, 3.0), (6.3, 2.4, 3.0), (-7.0, 4.3, 2.3), (7.0, 4.3, 2.3)];
pub const BODY_SLAB: [f32; 4] = [-5.6, -4.3, 11.2, 6.3];
/// The triggers' and the bumpers' pills behind the body (x, y, w, h), the left's (the right's is mirrored).
pub const TRIGGER: [f32; 4] = [-7.9, -6.3, 3.0, 1.5];
pub const BUMPER: [f32; 4] = [-8.3, -4.8, 4.6, 1.1];

/// Where the sticks, the D-pad, the face buttons' middle and the three middle buttons sit.
pub const LEFT_STICK: [f32; 2] = [-5.4, -1.5];
pub const RIGHT_STICK: [f32; 2] = [2.9, 1.7];
pub const DPAD: [f32; 2] = [-2.9, 1.7];
pub const FACE: [f32; 2] = [5.4, -1.5];
pub const VIEW: [f32; 2] = [-1.6, -2.0];
pub const MENU: [f32; 2] = [1.6, -2.0];
pub const HOME: [f32; 2] = [0.0, -3.3];
/// How far each face button sits from the face's middle.
pub const FACE_SPREAD: f32 = 1.3;

/// A part of the pad called out: a button, a stick (its push and its click), or the D-pad's four.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Part {
    Button(PadButton),
    LeftStick,
    RightStick,
    DPad,
}

impl Part {
    /// The parts called out, the left side's first, top to bottom, then the right's.
    pub const LEFT: [Part; 5] = [
        Part::Button(PadButton::LeftTrigger),
        Part::Button(PadButton::LeftBumper),
        Part::Button(PadButton::View),
        Part::LeftStick,
        Part::DPad,
    ];
    pub const RIGHT: [Part; 8] = [
        Part::Button(PadButton::RightTrigger),
        Part::Button(PadButton::RightBumper),
        Part::Button(PadButton::Menu),
        Part::Button(PadButton::North),
        Part::Button(PadButton::East),
        Part::Button(PadButton::West),
        Part::Button(PadButton::South),
        Part::RightStick,
    ];

    /// The point on the pad its callout's line meets (pad units).
    pub fn anchor(self) -> [f32; 2] {
        let face = |dx: f32, dy: f32| [FACE[0] + dx * FACE_SPREAD, FACE[1] + dy * FACE_SPREAD];
        match self {
            Part::Button(b) => match b {
                PadButton::LeftTrigger => [-7.7, -5.55],
                PadButton::RightTrigger => [7.7, -5.55],
                PadButton::LeftBumper => [-8.1, -4.25],
                PadButton::RightBumper => [8.1, -4.25],
                PadButton::View => VIEW,
                PadButton::Menu => MENU,
                PadButton::North => face(0.0, -1.0),
                PadButton::East => face(1.0, 0.0),
                PadButton::West => face(-1.0, 0.0),
                PadButton::South => face(0.0, 1.0),
                PadButton::LeftStick => LEFT_STICK,
                PadButton::RightStick => RIGHT_STICK,
                _ => DPAD,
            },
            Part::LeftStick => LEFT_STICK,
            Part::RightStick => RIGHT_STICK,
            Part::DPad => DPAD,
        }
    }

    /// The buttons it stands for.
    pub fn buttons(self) -> Vec<PadButton> {
        match self {
            Part::Button(b) => vec![b],
            Part::LeftStick => vec![PadButton::LeftStick],
            Part::RightStick => vec![PadButton::RightStick],
            Part::DPad => vec![PadButton::DPadUp, PadButton::DPadDown, PadButton::DPadLeft, PadButton::DPadRight],
        }
    }

    /// Its name as the pad in use names it ("LT", "L2", "Left stick", "D-pad").
    pub fn name(self, device: Device) -> String {
        match self {
            Part::Button(b) => crate::input::control_name(Control::Pad(b), device).into_owned(),
            Part::LeftStick => "Left stick".into(),
            Part::RightStick => "Right stick".into(),
            Part::DPad => "D-pad".into(),
        }
    }
}

/// One part of the pad called out: what it does in play, from the bindings.
#[derive(Clone, Debug, PartialEq)]
pub struct Callout {
    pub part: Part,
    /// The play actions on its buttons (a stick's click, the D-pad's four ways).
    pub actions: Vec<Action>,
    /// What it does, in a few words.
    pub does: String,
}

/// A short name for what an action does on a legend.
pub fn short(a: Action) -> &'static str {
    match a {
        Action::Attack => "Attack",
        Action::Jump => "Jump / evade",
        Action::Block => "Shield",
        Action::Step => "Bifrost Step",
        Action::Angel => "Archangel",
        Action::Drink => "Drink potion",
        Action::PotionPrev => "Potion ←",
        Action::PotionNext => "Potion →",
        Action::Call => "Call the old man",
        Action::Guide => "His word",
        Action::Map => "Map",
        Action::Camera => "Camera",
        Action::CameraReset => "Recentre",
        Action::MinimapZoom => "Minimap zoom",
        Action::Pause => "Pause",
        Action::Confirm => "Select",
        _ => action_name(a),
    }
}

/// The pad's callouts, the left side's then the right's (`Part::LEFT`, `Part::RIGHT`), each with
/// the play actions on it.
pub fn callouts(b: &Bindings) -> Vec<Callout> {
    Part::LEFT
        .iter()
        .chain(Part::RIGHT.iter())
        .map(|&part| {
            let mut actions: Vec<Action> = Vec::new();
            for button in part.buttons() {
                for a in b.actions(Control::Pad(button), Context::Play) {
                    if !actions.contains(&a) {
                        actions.push(a);
                    }
                }
            }
            let words: Vec<String> = match part {
                Part::DPad => {
                    // Each way that does something, named by its arrow (left and right as one
                    // when they turn the belt, as they do at first).
                    let way = |w: PadButton| b.actions(Control::Pad(w), Context::Play);
                    let mut ways = vec![(PadButton::DPadUp, "↑"), (PadButton::DPadDown, "↓")];
                    let belt = way(PadButton::DPadLeft) == [Action::PotionPrev] && way(PadButton::DPadRight) == [Action::PotionNext];
                    if !belt {
                        ways.extend([(PadButton::DPadLeft, "←"), (PadButton::DPadRight, "→")]);
                    }
                    let mut words: Vec<String> = ways
                        .iter()
                        .filter_map(|&(w, arrow)| {
                            let acts = way(w);
                            (!acts.is_empty()).then(|| format!("{arrow} {}", acts.iter().map(|&a| short(a)).collect::<Vec<_>>().join(", ")))
                        })
                        .collect();
                    if belt {
                        words.push("←→ Potion".into());
                    }
                    words
                }
                _ => actions.iter().map(|&a| short(a).to_string()).collect(),
            };
            let does = match part {
                Part::LeftStick if words.is_empty() => "Run".to_string(),
                Part::LeftStick => format!("Run · click: {}", words.join(", ")),
                Part::RightStick if words.is_empty() => "Camera".to_string(),
                Part::RightStick => format!("Camera · click: {}", words.join(", ")),
                _ if words.is_empty() => "-".to_string(),
                Part::DPad => words.join("  ·  "),
                _ => words.join(", "),
            };
            Callout { part, actions, does }
        })
        .collect()
}

/// One row of the keyboard and mouse column: the controls (keys and mouse buttons, as bound) and
/// what they do.
#[derive(Clone, Debug, PartialEq)]
pub struct KeyRow {
    pub controls: Vec<Control>,
    pub actions: Vec<Action>,
    pub does: String,
}

/// The first keyboard or mouse control bound to `a`, if any.
fn key_of(b: &Bindings, a: Action) -> Option<Control> {
    b.controls(a).into_iter().find(|c| !c.is_pad())
}

/// The keyboard and mouse column, from the bindings: the four ways of running as one row, the
/// belt's choice and its five slots as one each, then every other play action with a key.
pub fn key_rows(b: &Bindings) -> Vec<KeyRow> {
    let mut rows = Vec::new();
    let group = |acts: &[Action], does: &str, rows: &mut Vec<KeyRow>| {
        let controls: Vec<Control> = acts.iter().filter_map(|&a| key_of(b, a)).collect();
        let actions: Vec<Action> = acts.iter().copied().filter(|&a| key_of(b, a).is_some()).collect();
        if !controls.is_empty() {
            rows.push(KeyRow { controls, actions, does: does.to_string() });
        }
    };
    group(&[Action::MoveUp, Action::MoveLeft, Action::MoveDown, Action::MoveRight], "Run", &mut rows);
    for a in [Action::Attack, Action::Block, Action::Jump, Action::Step, Action::Angel, Action::Drink] {
        group(&[a], short(a), &mut rows);
    }
    group(&[Action::PotionPrev, Action::PotionNext], "Choose a potion", &mut rows);
    let slots = [Action::Slot(0), Action::Slot(1), Action::Slot(2), Action::Slot(3), Action::Slot(4)];
    if slots.iter().all(|&a| key_of(b, a).is_some()) {
        // (As a range, first to last, when all five have a key.)
        let (first, last) = (key_of(b, slots[0]).unwrap(), key_of(b, slots[4]).unwrap());
        rows.push(KeyRow { controls: vec![first, last], actions: slots.to_vec(), does: "Drink from slot".into() });
    } else {
        for a in slots {
            group(&[a], action_name(a), &mut rows);
        }
    }
    for a in
        [Action::Call, Action::Guide, Action::Map, Action::Camera, Action::CameraReset, Action::MinimapZoom, Action::Pause, Action::Confirm]
    {
        group(&[a], short(a), &mut rows);
    }
    rows
}

/// The mouse's own row: it turns the camera (no binding: it is the mouse).
pub const MOUSE_LOOK: &str = "Camera (click the game to take the mouse)";

/// The bindings as saved, for the engine's start screen (its own thread, before the game is
/// made): the defaults if nothing is saved.
pub fn saved_bindings() -> Bindings {
    if cfg!(test) {
        return Bindings::default();
    }
    crate::platform::load(crate::input::bindings::BINDINGS_KEY).map(|s| Bindings::from_setting(&s)).unwrap_or_default()
}

/// The device used last as saved: a controller until one is known.
pub fn saved_device() -> Device {
    if cfg!(test) {
        return Device::Xbox;
    }
    crate::platform::load(crate::settings::LAST_DEVICE_KEY).and_then(|s| Device::from_name(s.trim())).unwrap_or(Device::Xbox)
}

/// The old man's tips, one a load (`{Token}`s name the controls as the device in use calls them):
/// short, true, and kind. He always leads to good.
pub const TIPS: [&str; 14] = [
    "Raise your shield with {Block} just as a blow comes: a parry costs you nothing and leaves the foe open.",
    "A blade that glints red is a blow no shield holds. Step out of its way with {Jump}.",
    "Every swing spends breath. Rest a moment and it comes back to you.",
    "Lost? Press {Guide} and I will tell you what this place asks of you.",
    "Call me with {Call} wherever you are, and my wares are open to you.",
    "{Drink} drinks the chosen potion; {PotionPrev} and {PotionNext} choose another.",
    "The archangel's blessing you carry answers {Angel}. Keep it for when you are hard pressed.",
    "Parry a witch's bolt and it flies back at her.",
    "{Camera} switches the camera: from above, or over your shoulder.",
    "Once you have learned the Bifrost Step, {Step} carries you clear in a blink.",
    "Your silver and rune fragments are kept between adventures. Nothing you earn is lost.",
    "Every control can be changed in Settings, from the pause menu or the title.",
    "{Map} opens the map of all you have walked.",
    "A guard held too long on too little breath will break. Strike back when the foe's blow is spent.",
];

/// The tip for the `n`th load.
pub fn tip(n: usize) -> &'static str {
    TIPS[n % TIPS.len()]
}

/// A mouse button's short name on a keycap-sized glyph ("LMB" would be the genre's; the game says
/// what it is).
pub fn mouse_short(b: MouseButton) -> &'static str {
    match b {
        MouseButton::Left => "Left click",
        MouseButton::Right => "Right click",
        MouseButton::Middle => "Middle click",
        MouseButton::Back => "Mouse 4",
        MouseButton::Forward => "Mouse 5",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::bindings::PLAY;
    use winit::keyboard::KeyCode;

    /// Every play action bound on a side appears on the legend of that side.
    fn listed(b: &Bindings) {
        let on_pad: Vec<Action> = callouts(b).into_iter().flat_map(|c| c.actions).collect();
        let on_keys: Vec<Action> = key_rows(b).into_iter().flat_map(|r| r.actions).collect();
        for a in PLAY {
            let controls = b.controls(a);
            if controls.iter().any(|c| c.is_pad()) {
                assert!(on_pad.contains(&a), "{a:?} is bound on the pad but not called out");
            }
            if controls.iter().any(|c| !c.is_pad()) {
                assert!(on_keys.contains(&a), "{a:?} has a key but no row");
            }
        }
    }

    #[test]
    fn the_legend_lists_every_action_bound_on_each_device() {
        let mut b = Bindings::default();
        listed(&b);
        // The defaults: X attacks, the D-pad's up is the map, the left stick's click the minimap.
        let c = callouts(&b);
        let of = |p: Part| c.iter().find(|x| x.part == p).unwrap().clone();
        assert_eq!(of(Part::Button(PadButton::West)).does, "Attack");
        assert!(of(Part::DPad).does.contains("↑ Map"), "{}", of(Part::DPad).does);
        assert!(of(Part::LeftStick).does.starts_with("Run · click: Minimap"), "{}", of(Part::LeftStick).does);
        assert_eq!(of(Part::Button(PadButton::Menu)).does, "Pause");
        // Rebound: the legend follows (Y attacks too; the jump to a mouse button; the slots apart).
        b.bind(Action::Attack, Control::Pad(PadButton::North));
        b.unbind(Action::Jump, Control::Key(KeyCode::Space));
        b.bind(Action::Jump, Control::Mouse(MouseButton::Back));
        b.unbind(Action::Slot(2), Control::Key(KeyCode::Digit3));
        listed(&b);
        let c = callouts(&b);
        assert!(c.iter().find(|x| x.part == Part::Button(PadButton::North)).unwrap().actions.contains(&Action::Attack));
        let rows = key_rows(&b);
        assert!(rows.iter().any(|r| r.controls == vec![Control::Mouse(MouseButton::Back)] && r.does == short(Action::Jump)));
        assert!(rows.iter().any(|r| r.actions == vec![Action::Slot(0)]), "the slots one by one when one has no key");
        // An action left with nothing on a side is not listed there.
        for c in b.controls(Action::Call) {
            b.unbind(Action::Call, c);
        }
        assert!(!callouts(&b).iter().any(|x| x.actions.contains(&Action::Call)));
        assert!(!key_rows(&b).iter().any(|r| r.actions.contains(&Action::Call)));
        listed(&b);
    }

    #[test]
    fn the_tips_name_only_real_controls_and_turn_with_each_load() {
        for t in TIPS {
            for piece in t.split('{').skip(1) {
                let token = piece.split('}').next().unwrap();
                assert!(crate::input::from_token(token).is_some(), "{token} in {t}");
            }
            assert!(!t.to_lowercase().contains("devil") && !t.to_lowercase().contains("satan"));
        }
        assert_ne!(tip(0), tip(1));
        assert_eq!(tip(TIPS.len()), tip(0));
    }
}
