//! Device-aware prompts (UI.md §2.2, §4): the control bound to an action on the device used last,
//! drawn as a shape: the face buttons in their colours (A B X Y, or Cross Circle Square Triangle),
//! bumpers, triggers, sticks, the D-pad with its arm lit, View and Menu; keycaps with the key's
//! name; the mouse with its button lit. Prompts change the moment the player switches devices.
//!
//! Text with prompts in it is written with tokens: `"{Attack} strike"`, `"{Back} close"`; each
//! token is an action's (`input::token`), or one of the pairs that read as one control: `{Move}`
//! (the left stick, or W A S D), `{Look}` (the right stick, or the mouse), `{Nav}` (the D-pad, or
//! the arrows) and `{NavLR}` (left and right on either). A token we do not know is written as it is.
//!
//! ```ignore
//! // A line of prompts, centred under a panel, in the HUD's dim sans:
//! let s = Style::body(t * 0.42, rgba(TEXT_DIM, 1.0)).centered();
//! glyphs::prompt_text(o, &g.input, "{Confirm} take     {Back} leave", w / 2.0, y, &s);
//! // One glyph, 22 px tall, its left edge at x:
//! let wide = glyphs::draw_prompt(o, &g.input, Action::Angel, x, y, 22.0);
//! ```

use crate::input::{from_token, Action, Control, Device, Input, MouseButton, PadButton};
use crate::render::overlay::{rgba, Align, Overlay, Style};

/// The glyphs' colours: a keycap's face and edge, its label, a lit part, and a controller's dark plastic.
const CAP: u32 = 0xe8e0cc;
const CAP_EDGE: u32 = 0x8a7a5a;
const LABEL: u32 = 0x1a1612;
const LIT: u32 = 0xffc35a;
const PLASTIC: u32 = 0x2a2c33;
const PLASTIC_EDGE: u32 = 0x9aa0ad;

/// A piece of a prompt line: words, an action's control, or one of the pairs.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Piece {
    Text(String),
    Action(Action),
    /// The left stick or W A S D.
    Move,
    /// The right stick or the mouse.
    Look,
    /// The D-pad or the arrow keys, all four ways.
    Nav,
    /// Left and right on the D-pad or the arrow keys.
    NavLR,
}

/// Splits `text` at its `{Token}`s.
pub fn parse(text: &str) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}').map(|c| open + c) else { break };
        let name = &rest[open + 1..close];
        let piece = match name {
            "Move" => Some(Piece::Move),
            "Look" => Some(Piece::Look),
            "Nav" => Some(Piece::Nav),
            "NavLR" => Some(Piece::NavLR),
            _ => from_token(name).map(Piece::Action),
        };
        match piece {
            Some(p) => {
                if open > 0 {
                    out.push(Piece::Text(rest[..open].to_string()));
                }
                out.push(p);
            }
            None => out.push(Piece::Text(rest[..=close].to_string())),
        }
        rest = &rest[close + 1..];
    }
    if !rest.is_empty() {
        out.push(Piece::Text(rest.to_string()));
    }
    // Neighbouring words joined again (an unknown token splits them).
    let mut merged: Vec<Piece> = Vec::with_capacity(out.len());
    for p in out {
        match (merged.last_mut(), p) {
            (Some(Piece::Text(a)), Piece::Text(b)) => a.push_str(&b),
            (_, p) => merged.push(p),
        }
    }
    merged
}

/// The words a piece stands for, for a reader or a log ("[RT] use the blessing").
pub fn plain(input: &Input, text: &str) -> String {
    let d = input.last_device();
    parse(text)
        .into_iter()
        .map(|p| match p {
            Piece::Text(t) => t,
            Piece::Action(a) => input.prompt(a).map(|n| format!("[{n}]")).unwrap_or_default(),
            Piece::Move => if d.is_pad() { "[Left stick]" } else { "[W A S D]" }.into(),
            Piece::Look => if d.is_pad() { "[Right stick]" } else { "[Mouse]" }.into(),
            Piece::Nav => if d.is_pad() { "[D-pad]" } else { "[Arrows]" }.into(),
            Piece::NavLR => if d.is_pad() { "[D-pad ← →]" } else { "[← →]" }.into(),
        })
        .collect()
}

/// A glyph's height for text of `size` (a little over its capitals).
fn glyph_height(size: f32) -> f32 {
    size * 1.3
}

/// What one piece draws as on `device`: its controls (one, or a pair shown together).
fn controls_of(input: &Input, piece: &Piece, device: Device) -> Vec<Shape> {
    use winit::keyboard::KeyCode::*;
    let pad = device.is_pad();
    match piece {
        Piece::Text(_) => vec![],
        Piece::Action(a) => input.bindings.shown(*a, device).map(|c| vec![Shape::Control(c)]).unwrap_or_default(),
        Piece::Move if pad => vec![Shape::Stick { right: false, click: false }],
        Piece::Move => vec![Shape::Keys(&["W", "A", "S", "D"])],
        Piece::Look if pad => vec![Shape::Stick { right: true, click: false }],
        Piece::Look => vec![Shape::Mouse(None)],
        Piece::Nav if pad => vec![Shape::Dpad(None)],
        Piece::Nav => vec![Shape::Control(Control::Key(ArrowUp)), Shape::Control(Control::Key(ArrowDown))],
        Piece::NavLR if pad => vec![Shape::Control(Control::Pad(PadButton::DPadLeft)), Shape::Control(Control::Pad(PadButton::DPadRight))],
        Piece::NavLR => vec![Shape::Control(Control::Key(ArrowLeft)), Shape::Control(Control::Key(ArrowRight))],
    }
}

/// A glyph to draw.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Shape {
    Control(Control),
    /// A stick (left or right), clicked or pushed.
    Stick {
        right: bool,
        click: bool,
    },
    /// The D-pad, one arm lit (or none).
    Dpad(Option<PadButton>),
    /// The mouse, a button lit (or none: it moves).
    Mouse(Option<MouseButton>),
    /// Several keys as one cluster (W A S D).
    Keys(&'static [&'static str]),
}

/// Draws the control bound to `action` on the device used last, `size` pixels tall, its left edge at
/// `x` and centred on `y`. Its width (0 when nothing is bound there).
pub fn draw_prompt(o: &mut Overlay, input: &Input, action: Action, x: f32, y: f32, size: f32) -> f32 {
    let device = input.last_device();
    match input.bindings.shown(action, device) {
        Some(c) => draw_control(o, c, device, x, y, size),
        None => 0.0,
    }
}

/// `draw_prompt` with its right edge at `x` (a glyph in a panel's corner). Its width.
pub fn draw_prompt_right(o: &mut Overlay, input: &Input, action: Action, x: f32, y: f32, size: f32) -> f32 {
    let device = input.last_device();
    match input.bindings.shown(action, device) {
        Some(c) => {
            let w = control_width(o, c, device, size);
            draw_control(o, c, device, x - w, y, size)
        }
        None => 0.0,
    }
}

/// Draws one control as its glyph on `device`'s layout (see `draw_prompt`). Its width.
pub fn draw_control(o: &mut Overlay, c: Control, device: Device, x: f32, y: f32, size: f32) -> f32 {
    draw_shape(o, Shape::Control(c), device, x, y, size, true)
}

/// The width `draw_control` would take.
pub fn control_width(o: &Overlay, c: Control, device: Device, size: f32) -> f32 {
    shape_width(o, Shape::Control(c), device, size)
}

/// The width of `text` with its tokens drawn as glyphs, in `style`.
pub fn measure_prompt_text(o: &Overlay, input: &Input, text: &str, style: &Style) -> f32 {
    measure_prompt_text_on(o, input, input.last_device(), text, style)
}

/// `measure_prompt_text` for `device`, whichever was used last.
pub fn measure_prompt_text_on(o: &Overlay, input: &Input, device: Device, text: &str, style: &Style) -> f32 {
    let g = glyph_height(style.size);
    let gap = style.size * 0.25;
    parse(text)
        .iter()
        .map(|p| match p {
            Piece::Text(t) => o.measure_styled(t, style),
            p => {
                let shapes = controls_of(input, p, device);
                let w: f32 =
                    shapes.iter().map(|&s| shape_width(o, s, device, g)).sum::<f32>() + gap * shapes.len().saturating_sub(1) as f32;
                if shapes.is_empty() {
                    0.0
                } else {
                    w + gap
                }
            }
        })
        .sum()
}

/**
 * Draws `text` in `style` with each `{Token}` as its control's glyph on the device used last,
 * aligned on `x` by `style.align` and centred on `y` as text is. Its width.
 */
pub fn prompt_text(o: &mut Overlay, input: &Input, text: &str, x: f32, y: f32, style: &Style) -> f32 {
    prompt_text_on(o, input, input.last_device(), text, x, y, style)
}

/// `prompt_text` in `device`'s glyphs, whichever was used last (a page about one device).
pub fn prompt_text_on(o: &mut Overlay, input: &Input, device: Device, text: &str, x: f32, y: f32, style: &Style) -> f32 {
    let width = measure_prompt_text_on(o, input, device, text, style);
    let mut pen = match style.align {
        Align::Left => x,
        Align::Center => x - width / 2.0,
        Align::Right => x - width,
    };
    let left = Style { align: Align::Left, ..*style };
    let g = glyph_height(style.size);
    let gap = style.size * 0.25;
    let alpha = o.alpha;
    for p in parse(text) {
        match p {
            Piece::Text(t) => pen += o.text_styled(&t, pen, y, &left),
            p => {
                let shapes = controls_of(input, &p, device);
                if shapes.is_empty() {
                    continue;
                }
                o.alpha = alpha * style.color[3];
                for (k, s) in shapes.into_iter().enumerate() {
                    if k > 0 {
                        pen += gap;
                    }
                    pen += draw_shape(o, s, device, pen, y, g, true);
                }
                o.alpha = alpha;
                pen += gap;
            }
        }
    }
    width
}

/// `prompt_text` in the look the old `text` call gave (the body sans, the HUD's drop shadow).
#[allow(clippy::too_many_arguments)]
pub fn prompt_line(o: &mut Overlay, input: &Input, text: &str, x: f32, y: f32, size: f32, color: [f32; 4], align: Align) -> f32 {
    prompt_line_on(o, input, input.last_device(), text, x, y, size, color, align)
}

/// `prompt_line` in `device`'s glyphs.
#[allow(clippy::too_many_arguments)]
pub fn prompt_line_on(
    o: &mut Overlay,
    input: &Input,
    device: Device,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    color: [f32; 4],
    align: Align,
) -> f32 {
    let mut s = Style::body(size, color).align(align);
    if o.shadow > 0.0 {
        let d = o.shadow.max(size * 0.07).min((size * 0.12).max(1.0));
        s = Style {
            shadow: Some(crate::render::overlay::Shadow { dx: d * 0.8, dy: d, blur: d * 0.3, color: [0.0, 0.0, 0.0, color[3] * 0.7] }),
            ..s
        };
    }
    prompt_text_on(o, input, device, text, x, y, &s)
}

fn shape_width(o: &Overlay, s: Shape, device: Device, h: f32) -> f32 {
    match s {
        Shape::Control(Control::Key(k)) => keycap_width(o, &crate::input::key_name(k), h),
        Shape::Control(Control::Mouse(_) | Control::WheelUp | Control::WheelDown) | Shape::Mouse(_) => h * 0.72,
        Shape::Control(Control::Pad(b)) => match b {
            PadButton::LeftBumper | PadButton::RightBumper => h * 1.5,
            PadButton::LeftTrigger | PadButton::RightTrigger => h * 1.05,
            PadButton::View | PadButton::Menu if device == Device::PlayStation => h * 1.0,
            PadButton::View | PadButton::Menu => h * 1.0,
            _ => h,
        },
        Shape::Stick { .. } | Shape::Dpad(_) => h,
        Shape::Keys(keys) => keys.iter().map(|k| keycap_width(o, k, h)).sum::<f32>() + h * 0.1 * (keys.len() as f32 - 1.0),
    }
}

fn keycap_width(o: &Overlay, name: &str, h: f32) -> f32 {
    let label = Style::body(h * 0.62, [1.0; 4]).bold();
    (o.measure_styled(name, &label) + h * 0.45).max(h)
}

/// A rounded rectangle, filled.
pub fn rounded_rect(o: &mut Overlay, x: f32, y: f32, w: f32, h: f32, r: f32, color: [f32; 4]) {
    let r = r.min(w / 2.0).min(h / 2.0);
    o.rect(x + r, y, w - 2.0 * r, h, color);
    o.rect(x, y + r, r, h - 2.0 * r, color);
    o.rect(x + w - r, y + r, r, h - 2.0 * r, color);
    for (cx, cy) in [(x + r, y + r), (x + w - r, y + r), (x + r, y + h - r), (x + w - r, y + h - r)] {
        quarter(o, cx, cy, r, cx < x + w / 2.0, cy < y + h / 2.0, color);
    }
}

/// A quarter disc at a rounded corner (left or right, top or bottom).
fn quarter(o: &mut Overlay, cx: f32, cy: f32, r: f32, left: bool, top: bool, color: [f32; 4]) {
    let start = match (left, top) {
        (true, true) => std::f32::consts::PI,
        (false, true) => 1.5 * std::f32::consts::PI,
        (false, false) => 0.0,
        (true, false) => 0.5 * std::f32::consts::PI,
    };
    let n = 5;
    for i in 0..n {
        let a0 = start + i as f32 / n as f32 * std::f32::consts::FRAC_PI_2;
        let a1 = start + (i + 1) as f32 / n as f32 * std::f32::consts::FRAC_PI_2;
        o.triangle([cx, cy], [cx + r * a0.cos(), cy + r * a0.sin()], [cx + r * a1.cos(), cy + r * a1.sin()], color);
    }
}

/// A rounded rectangle with an edge: the edge's colour, then the face inset by `lw`.
#[allow(clippy::too_many_arguments)]
fn framed(o: &mut Overlay, x: f32, y: f32, w: f32, h: f32, r: f32, lw: f32, face: [f32; 4], edge: [f32; 4]) {
    rounded_rect(o, x, y, w, h, r, edge);
    rounded_rect(o, x + lw, y + lw, w - 2.0 * lw, h - 2.0 * lw, (r - lw).max(0.0), face);
}

/// Draws a shape `h` tall at `x` (left), centred on `y`. Its width.
fn draw_shape(o: &mut Overlay, s: Shape, device: Device, x: f32, y: f32, h: f32, _lit: bool) -> f32 {
    let w = shape_width(o, s, device, h);
    let top = y - h / 2.0;
    let lw = (h * 0.07).max(1.0);
    let label = |o: &mut Overlay, text: &str, cx: f32, size: f32, color: u32| {
        o.text_styled(text, cx, y, &Style::body(size, rgba(color, 1.0)).bold().centered());
    };
    match s {
        Shape::Control(Control::Key(k)) => {
            keycap(o, &crate::input::key_name(k), x, top, w, h, lw);
        }
        Shape::Keys(keys) => {
            let mut pen = x;
            for k in keys {
                let kw = keycap_width(o, k, h);
                keycap(o, k, pen, top, kw, h, lw);
                pen += kw + h * 0.1;
            }
        }
        Shape::Control(Control::Mouse(b)) => mouse(o, x, top, w, h, lw, Some(b), false),
        Shape::Mouse(b) => mouse(o, x, top, w, h, lw, b, false),
        Shape::Control(Control::WheelUp | Control::WheelDown) => {
            mouse(o, x, top, w, h, lw, None, true);
            // The wheel's way: an arrow beside it.
            let up = matches!(s, Shape::Control(Control::WheelUp));
            let (ax, ay, d) = (x + w * 0.5, y + if up { -h * 0.62 } else { h * 0.62 }, h * 0.16);
            let tip = if up { ay - d } else { ay + d };
            let base = if up { ay + d * 0.4 } else { ay - d * 0.4 };
            o.triangle([ax, tip], [ax - d, base], [ax + d, base], rgba(LIT, 1.0));
        }
        Shape::Control(Control::Pad(b)) => pad_button(o, b, device, x, top, w, h, lw, &label),
        Shape::Stick { right, click } => stick(o, x, y, h, lw, if right { "R" } else { "L" }, click),
        Shape::Dpad(lit) => dpad(o, x, y, h, lit),
    }
    w
}

fn keycap(o: &mut Overlay, name: &str, x: f32, top: f32, w: f32, h: f32, lw: f32) {
    // A cap with a darker lip below, so it reads as a key.
    rounded_rect(o, x, top + h * 0.08, w, h * 0.92, h * 0.18, rgba(CAP_EDGE, 1.0));
    framed(o, x, top, w, h * 0.9, h * 0.18, lw * 0.8, rgba(CAP, 1.0), rgba(CAP_EDGE, 1.0));
    let size = if name.chars().count() > 1 { h * 0.52 } else { h * 0.62 };
    o.text_styled(name, x + w / 2.0, top + h * 0.45, &Style::body(size, rgba(LABEL, 1.0)).bold().centered());
}

#[allow(clippy::too_many_arguments)]
fn mouse(o: &mut Overlay, x: f32, top: f32, w: f32, h: f32, lw: f32, lit: Option<MouseButton>, wheel: bool) {
    framed(o, x, top, w, h, w * 0.48, lw, rgba(CAP, 1.0), rgba(CAP_EDGE, 1.0));
    let split = top + h * 0.42;
    let inner = |o: &mut Overlay, left: bool| {
        let (bx, bw) = if left { (x + lw, w / 2.0 - lw) } else { (x + w / 2.0, w / 2.0 - lw) };
        o.rect(bx, top + lw + w * 0.2, bw, split - top - lw - w * 0.2, rgba(LIT, 1.0));
        let r = w * 0.48 - lw;
        let cx = if left { x + lw + r } else { x + w - lw - r };
        quarter(o, cx, top + lw + r, r, left, true, rgba(LIT, 1.0));
    };
    match lit {
        Some(MouseButton::Left) => inner(o, true),
        Some(MouseButton::Right) => inner(o, false),
        _ => {}
    }
    o.rect(x + lw, split, w - 2.0 * lw, lw, rgba(CAP_EDGE, 1.0));
    o.rect(x + w / 2.0 - lw / 2.0, top, lw, split - top, rgba(CAP_EDGE, 1.0));
    let lit_wheel = wheel || lit == Some(MouseButton::Middle);
    rounded_rect(o, x + w / 2.0 - w * 0.1, top + h * 0.14, w * 0.2, h * 0.2, w * 0.1, rgba(if lit_wheel { LIT } else { CAP_EDGE }, 1.0));
}

/// A face button's colour on `device` (its letter's, or its symbol's on a PlayStation pad).
pub fn face_color(b: PadButton, device: Device) -> u32 {
    face(b, device).0
}

/// The face buttons' colours and letters: Xbox's, PlayStation's symbols, or plain for one we do not know.
fn face(b: PadButton, device: Device) -> (u32, &'static str) {
    match (device, b) {
        (Device::PlayStation, PadButton::South) => (0x7aa8f0, "x"),
        (Device::PlayStation, PadButton::East) => (0xf0707a, "o"),
        (Device::PlayStation, PadButton::West) => (0xe08ae0, "s"),
        (Device::PlayStation, _) => (0x4ad0a8, "t"),
        (Device::Generic, PadButton::South) => (0xb8bcc6, "A"),
        (Device::Generic, PadButton::East) => (0xb8bcc6, "B"),
        (Device::Generic, PadButton::West) => (0xb8bcc6, "X"),
        (Device::Generic, _) => (0xb8bcc6, "Y"),
        (_, PadButton::South) => (0x5cb83a, "A"),
        (_, PadButton::East) => (0xe0463c, "B"),
        (_, PadButton::West) => (0x3a7fd8, "X"),
        (_, _) => (0xf0c020, "Y"),
    }
}

#[allow(clippy::too_many_arguments)]
fn pad_button(
    o: &mut Overlay,
    b: PadButton,
    device: Device,
    x: f32,
    top: f32,
    w: f32,
    h: f32,
    lw: f32,
    label: &dyn Fn(&mut Overlay, &str, f32, f32, u32),
) {
    let ps = device == Device::PlayStation;
    let (cx, cy) = (x + w / 2.0, top + h / 2.0);
    match b {
        PadButton::South | PadButton::East | PadButton::West | PadButton::North => {
            let (color, mark) = face(b, device);
            if ps {
                // A dark button with its coloured symbol.
                o.circle(cx, cy, h / 2.0, rgba(PLASTIC_EDGE, 1.0));
                o.circle(cx, cy, h / 2.0 - lw, rgba(PLASTIC, 1.0));
                let c = rgba(color, 1.0);
                let r = h * 0.24;
                let sw = (h * 0.09).max(1.2);
                match mark {
                    "x" => {
                        o.line([cx - r, cy - r], [cx + r, cy + r], sw, c);
                        o.line([cx - r, cy + r], [cx + r, cy - r], sw, c);
                    }
                    "o" => o.arc(cx, cy, r, 0.0, std::f32::consts::TAU, sw, c),
                    "s" => o.rect_outline(cx - r * 0.85, cy - r * 0.85, r * 1.7, r * 1.7, sw, c),
                    _ => {
                        let (a, b2, c2) = ([cx, cy - r * 1.05], [cx - r * 1.05, cy + r * 0.75], [cx + r * 1.05, cy + r * 0.75]);
                        o.line(a, b2, sw, c);
                        o.line(b2, c2, sw, c);
                        o.line(c2, a, sw, c);
                    }
                }
            } else {
                // A coloured button with its letter, as Xbox's are lettered.
                o.circle(cx, cy, h / 2.0, rgba(0x101114, 1.0));
                o.circle(cx, cy, h / 2.0 - lw, rgba(color, 1.0));
                label(o, mark, cx, h * 0.62, 0x101114);
            }
        }
        PadButton::LeftBumper | PadButton::RightBumper => {
            framed(o, x, top + h * 0.12, w, h * 0.76, h * 0.3, lw, rgba(PLASTIC, 1.0), rgba(PLASTIC_EDGE, 1.0));
            let name = match (b, ps) {
                (PadButton::LeftBumper, true) => "L1",
                (PadButton::RightBumper, true) => "R1",
                (PadButton::LeftBumper, false) => "LB",
                _ => "RB",
            };
            label(o, name, cx, h * 0.5, 0xf0ece0);
        }
        PadButton::LeftTrigger | PadButton::RightTrigger => {
            // Taller, rounded at the top like a trigger.
            rounded_rect(o, x, top, w, h, w * 0.42, rgba(PLASTIC_EDGE, 1.0));
            rounded_rect(o, x + lw, top + lw, w - 2.0 * lw, h - 2.0 * lw, w * 0.42 - lw, rgba(PLASTIC, 1.0));
            o.rect(x + lw, top + h * 0.6, w - 2.0 * lw, h * 0.4 - lw, rgba(PLASTIC, 1.0));
            let name = match (b, ps) {
                (PadButton::LeftTrigger, true) => "L2",
                (PadButton::RightTrigger, true) => "R2",
                (PadButton::LeftTrigger, false) => "LT",
                _ => "RT",
            };
            label(o, name, cx, h * 0.48, 0xf0ece0);
        }
        PadButton::LeftStick | PadButton::RightStick => {
            stick(o, x, cy, h, lw, if b == PadButton::LeftStick { "L" } else { "R" }, true);
        }
        PadButton::DPadUp | PadButton::DPadDown | PadButton::DPadLeft | PadButton::DPadRight => dpad(o, x, cy, h, Some(b)),
        PadButton::View | PadButton::Menu => {
            o.circle(cx, cy, h / 2.0, rgba(PLASTIC_EDGE, 1.0));
            o.circle(cx, cy, h / 2.0 - lw, rgba(PLASTIC, 1.0));
            let c = rgba(0xf0ece0, 1.0);
            let s = h * 0.17;
            if b == PadButton::Menu {
                // Three lines (Options: the same, as its button is marked).
                for k in -1..=1 {
                    o.rect(cx - s * 1.2, cy + k as f32 * s * 0.85 - lw * 0.45, s * 2.4, lw * 0.9, c);
                }
            } else if ps {
                // Share: an arrow up out of a tray.
                o.line([cx, cy + s * 0.6], [cx, cy - s * 1.1], lw, c);
                o.triangle([cx, cy - s * 1.5], [cx - s * 0.7, cy - s * 0.7], [cx + s * 0.7, cy - s * 0.7], c);
                o.rect_outline(cx - s, cy - s * 0.1, s * 2.0, s * 1.2, lw * 0.8, c);
            } else {
                // View: two windows, one over the other.
                o.rect_outline(cx - s * 1.1, cy - s * 0.9, s * 1.5, s * 1.1, lw * 0.8, c);
                o.rect(cx - s * 0.4, cy - s * 0.2, s * 1.5, s * 1.1, c);
            }
        }
    }
}

/// A stick from above: its ring and cap, its letter; clicked shows the press ring lit.
fn stick(o: &mut Overlay, x: f32, cy: f32, h: f32, lw: f32, name: &str, click: bool) {
    let cx = x + h / 2.0;
    o.circle(cx, cy, h / 2.0, rgba(PLASTIC_EDGE, 1.0));
    o.circle(cx, cy, h / 2.0 - lw, rgba(PLASTIC, 1.0));
    o.circle(cx, cy, h * 0.3, rgba(if click { LIT } else { 0x4a4e58 }, 1.0));
    o.circle(cx, cy, h * 0.3 - lw, rgba(PLASTIC, 1.0));
    let text = if click { format!("{name}3") } else { name.to_string() };
    o.text_styled(&text, cx, cy, &Style::body(h * if click { 0.36 } else { 0.42 }, rgba(0xf0ece0, 1.0)).bold().centered());
}

/// The D-pad: a cross, the arm `lit` gold (none: all four alike).
fn dpad(o: &mut Overlay, x: f32, cy: f32, h: f32, lit: Option<PadButton>) {
    let cx = x + h / 2.0;
    let (arm, thick) = (h / 2.0, h * 0.34);
    let edge = rgba(PLASTIC_EDGE, 1.0);
    let lw = (h * 0.06).max(1.0);
    o.rect(cx - thick / 2.0 - lw, cy - arm, thick + 2.0 * lw, 2.0 * arm, edge);
    o.rect(cx - arm, cy - thick / 2.0 - lw, 2.0 * arm, thick + 2.0 * lw, edge);
    let fill = rgba(PLASTIC, 1.0);
    o.rect(cx - thick / 2.0, cy - arm + lw, thick, 2.0 * arm - 2.0 * lw, fill);
    o.rect(cx - arm + lw, cy - thick / 2.0, 2.0 * arm - 2.0 * lw, thick, fill);
    let gold = rgba(LIT, 1.0);
    let part = arm - thick / 2.0 - lw;
    match lit {
        Some(PadButton::DPadUp) => o.rect(cx - thick / 2.0, cy - arm + lw, thick, part, gold),
        Some(PadButton::DPadDown) => o.rect(cx - thick / 2.0, cy + thick / 2.0, thick, part, gold),
        Some(PadButton::DPadLeft) => o.rect(cx - arm + lw, cy - thick / 2.0, part, thick, gold),
        Some(PadButton::DPadRight) => o.rect(cx + thick / 2.0, cy - thick / 2.0, part, thick, gold),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_expand_into_glyphs_and_words() {
        assert_eq!(parse("{Attack} strike"), vec![Piece::Action(Action::Attack), Piece::Text(" strike".into())]);
        assert_eq!(
            parse("{Confirm} take   {Back} leave"),
            vec![Piece::Action(Action::Confirm), Piece::Text(" take   ".into()), Piece::Action(Action::Back), Piece::Text(" leave".into())]
        );
        assert_eq!(parse("{Move} run, {NavLR}"), vec![Piece::Move, Piece::Text(" run, ".into()), Piece::NavLR]);
        // What is not a token stays as written.
        assert_eq!(parse("a {Nonsense} b {"), vec![Piece::Text("a {Nonsense} b {".into())]);
        assert_eq!(parse(""), vec![]);
    }

    #[test]
    fn prompts_follow_the_device_used_last() {
        let mut input = Input::new();
        input.set_last_device(Device::KeyboardMouse);
        assert_eq!(plain(&input, "{Angel} bless"), "[R] bless");
        assert_eq!(plain(&input, "{Attack}"), "[Left click]");
        input.set_last_device(Device::Xbox);
        assert_eq!(plain(&input, "{Angel} bless, {Back}"), "[RT] bless, [B]");
        input.set_last_device(Device::PlayStation);
        assert_eq!(plain(&input, "{Angel} bless, {Confirm}"), "[R2] bless, [Cross]");
    }

    #[test]
    fn a_prompt_line_is_as_wide_as_it_measures_and_glyphs_take_room() {
        let mut o = Overlay::new();
        o.begin(1280.0, 720.0);
        let mut input = Input::new();
        let s = Style::body(16.0, [1.0; 4]);
        for d in [Device::KeyboardMouse, Device::Xbox, Device::PlayStation, Device::Generic] {
            input.set_last_device(d);
            let text = "{Confirm} take   {Back} leave   {Move} run   {Nav} {NavLR} {Look} {PotionPrev}";
            let w = measure_prompt_text(&o, &input, text, &s);
            let words = o.measure_styled(" take    leave    run    ", &s);
            assert!(w > words + 5.0 * 16.0, "{d:?}: glyphs take room ({w} against {words})");
            o.verts.clear();
            assert_eq!(prompt_text(&mut o, &input, text, 0.0, 50.0, &s), w);
            let hi = o.verts.iter().map(|v| v.pos[0]).fold(f32::MIN, f32::max);
            assert!(hi <= w + 4.0, "{d:?}: drawn within its width: {hi} > {w}");
        }
        // Every control of every play action draws as something.
        input.set_last_device(Device::Xbox);
        for a in crate::input::bindings::PLAY {
            for c in input.bindings.controls(a) {
                for d in [Device::KeyboardMouse, Device::Xbox, Device::PlayStation] {
                    o.verts.clear();
                    let w = draw_control(&mut o, c, d, 0.0, 20.0, 20.0);
                    assert!(w > 0.0 && !o.verts.is_empty(), "{a:?} {c:?} on {d:?}");
                }
            }
        }
    }
}
