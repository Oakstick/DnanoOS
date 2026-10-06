//! The controls page, opened from the title or the pause menu: two tabs, the
//! controller and the keyboard. The controller is a gamepad drawn in the
//! game's colours, every button called out by a line to what it does (by
//! their Xbox names; a line under the pad gives their PlayStation twins). The
//! keyboard is every key as a keycap beside what it does.

use super::*;
use crate::input::{Control, PadButton};

const SHINE: u32 = 0x8a93a6;
const LINE: u32 = 0x9aa3b8;

/// The two tabs at the top of the page: the controller, then the keyboard and mouse.
pub fn controls_tab_rects(w: f32, h: f32) -> [[f32; 4]; 2] {
    let t = tile_for(w, h);
    let (bw, bh, gap) = (t * 8.0, t * 1.1, t * 0.4);
    let y = t * 2.55;
    [[w / 2.0 - bw - gap / 2.0, y, bw, bh], [w / 2.0 + gap / 2.0, y, bw, bh]]
}

/// What an action does, in a few words, for the page.
fn does(a: Action) -> &'static str {
    match a {
        Action::Attack => "Attack",
        Action::Jump => "Jump; evade in a fight",
        Action::Block => "Hold: raise your shield",
        Action::Step => "Bifrost Step, once learned",
        Action::Angel => "Archangel's power",
        Action::Drink => "Drink the chosen potion",
        Action::PotionPrev => "Previous potion",
        Action::PotionNext => "Next potion",
        Action::Call => "Call the old man (his wares)",
        Action::Guide => "The old man's word on this place",
        Action::Map => "The map",
        Action::Camera => "Switch the camera",
        Action::CameraReset => "Recentre the camera",
        Action::MinimapZoom => "Minimap: zoom",
        Action::Pause => "Pause and resume",
        Action::Confirm => "Select",
        Action::Back => "Back",
        Action::TabPrev => "Previous tab",
        Action::TabNext => "Next tab",
        Action::Secondary => "Secondary",
        _ => crate::input::action_name(a),
    }
}

/// The keyboard and mouse rows: each action's keys (as bound) beside what it does.
const KEY_ROWS: [Action; 14] = [
    Action::Attack,
    Action::Jump,
    Action::Block,
    Action::Step,
    Action::Angel,
    Action::Drink,
    Action::PotionNext,
    Action::Call,
    Action::Guide,
    Action::Map,
    Action::Camera,
    Action::Pause,
    Action::Confirm,
    Action::Slot(0),
];

/// The keyboard and mouse, as keycaps beside what they do, in two columns, from the bindings.
fn draw_keyboard(o: &mut Overlay, g: &Game, t: f32) {
    use winit::keyboard::KeyCode;
    let (w, h) = (o.width, o.height);
    let kbm = crate::input::Device::KeyboardMouse;
    // Each row: its glyphs (`[Control]` tokens and `{Action}` ones) and what it does.
    let mut rows: Vec<(String, String)> = vec![
        ("{Move}".into(), "Run, relative to the camera (arrows too)".into()),
        ("{Look}".into(), "Look (click the game to capture the mouse)".into()),
    ];
    for a in KEY_ROWS {
        let mut controls: Vec<Control> = g.input.bindings.controls(a).into_iter().filter(|c| !c.is_pad()).collect();
        // (Two keys of one name, Enter and the keypad's, show once.)
        controls.dedup_by_key(|c| crate::input::control_name(*c, kbm));
        controls.truncate(2);
        let (glyph, what) = match a {
            Action::PotionNext => ("{PotionPrev} {PotionNext}".to_string(), "Choose a potion on the belt".to_string()),
            Action::Slot(_) => ("[Digit1] - [Digit5]".to_string(), "Drink from that belt slot".to_string()),
            _ if controls.is_empty() => continue,
            _ => (controls.iter().map(|c| format!("[{}]", c.token())).collect::<Vec<_>>().join(" "), does(a).to_string()),
        };
        rows.push((glyph, what));
    }
    rows.push(("[F4]".into(), "Free or grid movement".into()));
    rows.push(("[ControlLeft] + [KeyM]".into(), "Sound on or off".into()));
    rows.push(("[F11]".into(), "Fullscreen (Alt + Enter too)".into()));
    let per_col = rows.len().div_ceil(2);
    let size = (t * 0.52).max(11.0);
    let (top, row) = (t * 5.0, ((h - t * 9.5) / per_col as f32).min(t * 1.35));
    let col_w = (w * 0.42).min(t * 22.0);
    let x0 = w / 2.0 - col_w - t * 0.5;
    let gh = row * 0.62;
    let body = crate::render::overlay::Style::body(size, rgba(TEXT, 1.0));
    for (i, (glyph, what)) in rows.iter().enumerate() {
        let (cx, cy) = (x0 + (i / per_col) as f32 * (col_w + t * 1.0), top + (i % per_col) as f32 * row + row * 0.4);
        let mut pen = cx;
        for part in glyph.split(' ') {
            let control = part.strip_prefix('[').and_then(|p| p.strip_suffix(']')).and_then(|tok| {
                Control::from_token(tok).or(match tok {
                    "F4" => Some(Control::Key(KeyCode::F4)),
                    "F11" => Some(Control::Key(KeyCode::F11)),
                    "ControlLeft" => Some(Control::Key(KeyCode::ControlLeft)),
                    "Digit1" => Some(Control::Key(KeyCode::Digit1)),
                    "Digit5" => Some(Control::Key(KeyCode::Digit5)),
                    _ => None,
                })
            });
            pen += match control {
                Some(c) => glyphs::draw_control(o, c, kbm, pen, cy, gh),
                None => glyphs::prompt_text_on(o, &g.input, kbm, part, pen, cy, &body),
            } + t * 0.15;
        }
        o.text(what, cx + t * 4.4, cy, size * 0.95, rgba(TEXT, 1.0), Align::Left, false);
    }
}

/// Where the Back button sits on the controller page.
fn controls_back_rect(w: f32, h: f32) -> [f32; 4] {
    let t = tile_for(w, h);
    let (bw, bh) = (t * 6.0, t * 1.3);
    [w / 2.0 - bw / 2.0, h - bh - t * 0.9, bw, bh]
}

/// One button called out: where it is on the pad, its name, its colour and what it does.
struct Callout {
    at: [f32; 2],
    name: String,
    color: u32,
    does: String,
}

/// What a controller button does, from the bindings: its play action, then what it does in menus.
fn pad_does(g: &Game, b: PadButton) -> String {
    use crate::input::Context;
    let c = Control::Pad(b);
    let play: Vec<&str> = g.input.bindings.actions(c, Context::Play).into_iter().map(does).collect();
    let menu: Vec<String> = g
        .input
        .bindings
        .actions(c, Context::Menu)
        .into_iter()
        .filter(|a| !matches!(a, Action::NavUp | Action::NavDown | Action::NavLeft | Action::NavRight))
        .map(|a| does(a).to_lowercase())
        .collect();
    let mut out = if play.is_empty() { "-".to_string() } else { play.join(", ") };
    if !menu.is_empty() {
        out += &format!("; menus: {}", menu.join(", "));
    }
    out
}

/// The D-pad's four ways in play, and in menus.
fn dpad_does(g: &Game) -> String {
    use crate::input::Context;
    let way =
        |b: PadButton| g.input.bindings.actions(Control::Pad(b), Context::Play).first().map(|&a| does(a)).unwrap_or("-").to_lowercase();
    format!("Left, right: potion; up: {}; down: {}; menus: move", way(PadButton::DPadUp), way(PadButton::DPadDown))
}

pub fn draw_controls(o: &mut Overlay, g: &Game, t: f32) {
    let (w, h) = (o.width, o.height);
    o.rect(0.0, 0.0, w, h, [2.0 / 255.0, 6.0 / 255.0, 12.0 / 255.0, 0.9]);
    o.text("CONTROLS", w / 2.0, t * 1.5, t * 1.3, rgba(GOLD, 1.0), Align::Center, true);
    // The two tabs, the open one lit.
    let pulse = 0.75 + (g.time as f32 * 4.0).sin() * 0.25;
    for (k, (r, name)) in controls_tab_rects(w, h).iter().zip(["CONTROLLER", "KEYBOARD AND MOUSE"]).enumerate() {
        let open = g.controls_keyboard == (k == 1);
        o.rect(r[0], r[1], r[2], r[3], if open { [0.25, 0.32, 0.5, 0.95] } else { [0.02, 0.02, 0.03, 0.85] });
        o.rect_outline(r[0], r[1], r[2], r[3], (t * 0.06).max(1.0), if open { rgba(GOLD, pulse) } else { [1.0, 1.0, 1.0, 0.16] });
        o.text(name, r[0] + r[2] / 2.0, r[1] + r[3] / 2.0, t * 0.5, rgba(if open { 0xffffff } else { TEXT_DIM }, 1.0), Align::Center, true);
    }
    glyphs::prompt_line(
        o,
        &g.input,
        "{NavLR} or {TabPrev} {TabNext} switch between them",
        w / 2.0,
        t * 4.2,
        t * 0.42,
        rgba(TEXT_DIM, 1.0),
        Align::Center,
    );
    if g.controls_keyboard {
        draw_keyboard(o, g, t);
        draw_back(o, g, t);
        return;
    }

    // The pad's buttons are named as the controller in use names them (Xbox's for one we do not know).
    let names_device = match g.input.last_device() {
        crate::input::Device::PlayStation => crate::input::Device::PlayStation,
        _ => crate::input::Device::Xbox,
    };
    let names = |b: PadButton| crate::input::control_name(Control::Pad(b), names_device).into_owned();
    // ---- the pad, about 20 units wide and 13 tall, centred a little above the middle
    let u = (w * 0.33).min(h * 0.58) / 20.0;
    let (cx, cy) = (w / 2.0, h * 0.5);
    let p = |x: f32, y: f32| [cx + x * u, cy + y * u];

    // Triggers and bumpers behind the body, then the body (`legend`, as the loading card has it).
    super::legend::draw_pad_body(o, cx, cy, u, names_device);

    // Where the sticks, the d-pad, the face buttons and the three in the middle sit.
    let (ls, rs, dpad, face) = (p(-5.4, -1.5), p(2.9, 1.7), p(-2.9, 1.7), p(5.4, -1.5));
    let (view, menu) = (p(-1.6, -2.0), p(1.6, -2.0));
    // The triggers and bumpers are called out from their outer ends, clear of their names.
    let (lt_at, rt_at, lb_at, rb_at) = (p(-7.7, -5.55), p(7.7, -5.55), p(-8.1, -4.25), p(8.1, -4.25));

    // ---- what each does, called out to either side (drawn under the buttons, so each line meets its button's edge)
    let face_at = |dx: f32, dy: f32| [face[0] + dx * u, face[1] + dy * u];
    let say = |b: PadButton| pad_does(g, b);
    let left = [
        Callout { at: lt_at, name: names(PadButton::LeftTrigger), color: SHINE, does: say(PadButton::LeftTrigger) },
        Callout { at: lb_at, name: names(PadButton::LeftBumper), color: SHINE, does: say(PadButton::LeftBumper) },
        Callout { at: view, name: names(PadButton::View), color: SHINE, does: say(PadButton::View) },
        Callout {
            at: ls,
            name: "Left stick".into(),
            color: SHINE,
            does: format!("Run; press: {}", say(PadButton::LeftStick).to_lowercase()),
        },
        Callout { at: dpad, name: "D-pad".into(), color: SHINE, does: dpad_does(g) },
    ];
    let right = [
        Callout { at: rt_at, name: names(PadButton::RightTrigger), color: SHINE, does: say(PadButton::RightTrigger) },
        Callout { at: rb_at, name: names(PadButton::RightBumper), color: SHINE, does: say(PadButton::RightBumper) },
        Callout { at: menu, name: names(PadButton::Menu), color: SHINE, does: say(PadButton::Menu) },
        Callout {
            at: face_at(0.0, -1.3),
            name: names(PadButton::North),
            color: glyphs::face_color(PadButton::North, names_device),
            does: say(PadButton::North),
        },
        Callout {
            at: face_at(1.3, 0.0),
            name: names(PadButton::East),
            color: glyphs::face_color(PadButton::East, names_device),
            does: say(PadButton::East),
        },
        Callout {
            at: face_at(-1.3, 0.0),
            name: names(PadButton::West),
            color: glyphs::face_color(PadButton::West, names_device),
            does: say(PadButton::West),
        },
        Callout {
            at: face_at(0.0, 1.3),
            name: names(PadButton::South),
            color: glyphs::face_color(PadButton::South, names_device),
            does: say(PadButton::South),
        },
        Callout {
            at: rs,
            name: "Right stick".into(),
            color: SHINE,
            does: format!("Look; press: {}", say(PadButton::RightStick).to_lowercase()),
        },
    ];
    // Top to bottom by where they sit on the pad, so the lines out to them cross as little as they can.
    let mut left = left;
    let mut right = right;
    left.sort_by(|a, b| a.at[1].total_cmp(&b.at[1]));
    right.sort_by(|a, b| a.at[1].total_cmp(&b.at[1]));
    let (top, bottom) = (cy - 6.4 * u, cy + 5.6 * u);
    let size = (t * 0.5).max(u * 0.55);
    let gap = size * 0.9;
    for (side, list) in [(-1.0f32, &left[..]), (1.0, &right[..])] {
        let edge = cx + side * 11.6 * u;
        let n = list.len();
        for (i, c) in list.iter().enumerate() {
            let y = top + (bottom - top) * (i as f32 + 0.5) / n as f32;
            // A dot on the button, a line out to the label.
            o.circle(c.at[0], c.at[1], (u * 0.13).max(2.0), rgba(LINE, 0.95));
            let elbow = [edge - side * 0.8 * u, y];
            o.line(c.at, elbow, (u * 0.06).max(1.0), rgba(LINE, 0.6));
            o.line(elbow, [edge, y], (u * 0.06).max(1.0), rgba(LINE, 0.6));
            let name_w = o.measure_as(&c.name, size, true);
            if side < 0.0 {
                o.text(&c.name, edge - gap * 0.4, y, size, rgba(c.color, 1.0), Align::Right, true);
                o.text(&c.does, edge - gap * 0.4 - name_w - gap, y, size * 0.92, rgba(TEXT, 1.0), Align::Right, false);
            } else {
                o.text(&c.name, edge + gap * 0.4, y, size, rgba(c.color, 1.0), Align::Left, true);
                o.text(&c.does, edge + gap * 0.4 + name_w + gap, y, size * 0.92, rgba(TEXT, 1.0), Align::Left, false);
            }
        }
    }

    // Sticks, d-pad, face buttons (as the controller in use marks them) and the three in the middle.
    super::legend::draw_pad_buttons(o, cx, cy, u, names_device);

    // ---- notes, and the way back
    let notes_y = cy + 7.4 * u;
    glyphs::prompt_line_on(
        o,
        &g.input,
        names_device,
        "In menus: {Nav} or {Move} move, {Confirm} select, {Back} back",
        w / 2.0,
        notes_y,
        t * 0.5,
        rgba(SAND, 1.0),
        Align::Center,
    );
    draw_back(o, g, t);
}

/// The Back button at the foot of the page.
fn draw_back(o: &mut Overlay, g: &Game, t: f32) {
    let (w, h) = (o.width, o.height);
    let r = controls_back_rect(w, h);
    let pulse = 0.75 + (g.time as f32 * 4.0).sin() * 0.25;
    o.rect(r[0], r[1], r[2], r[3], [2.0 / 255.0, 6.0 / 255.0, 12.0 / 255.0, 0.85]);
    o.rect(r[0], r[1], r[2], r[3], [1.0, 1.0, 1.0, 0.14]);
    o.rect_outline(r[0], r[1], r[2], r[3], (t * 0.08).max(1.0), rgba(GOLD, pulse));
    o.text("BACK", r[0] + r[2] / 2.0, r[1] + r[3] / 2.0, t * 0.6, rgba(GOLD, 1.0), Align::Center, true);
    glyphs::prompt_line(
        o,
        &g.input,
        "{Back} or a click to go back",
        w / 2.0,
        r[1] + r[3] + t * 0.45,
        t * 0.42,
        rgba(TEXT_DIM, 1.0),
        Align::Center,
    );
}
