//! The controller legend drawn with the overlay (UI.md §9): the gamepad in the game's colours
//! (shared with the controls page), each part called out by a line to what it does from the
//! bindings, the keyboard and mouse's column beside it, and the in-game loading card that shows
//! them with the level's name and a tip from the old man. The legend's data (the pad's shape, the
//! callouts, the rows, the tips) is `render::legend`'s, which the engine's start screen draws too.

use super::*;
use crate::game::Loading;
use crate::input::{Control, Device, PadButton};
use crate::render::legend::{self as data, Part};
use crate::render::overlay::Style;

pub(super) const BODY: u32 = 0x2a2e38;
pub(super) const EDGE_GREY: u32 = 0x626a7c;
pub(super) const DARK: u32 = 0x15171d;
pub(super) const CAP: u32 = 0x3e4452;
pub(super) const SHINE: u32 = 0x8a93a6;
pub(super) const LINE: u32 = 0x9aa3b8;
/// The old man's amber (his frame, his name).
const AMBER: u32 = 0xffa83a;

/// A filled circle with a rim a little lighter than it, the way the pad's plastic catches the light.
pub(super) fn disc(o: &mut Overlay, x: f32, y: f32, r: f32, fill: u32, rim: u32, lw: f32) {
    o.circle(x, y, r + lw, rgba(rim, 1.0));
    o.circle(x, y, r, rgba(fill, 1.0));
}

/// A rectangle with rounded ends (a bumper, a trigger, the d-pad's arms).
pub(super) fn pill(o: &mut Overlay, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
    let r = h.min(w) / 2.0;
    if w >= h {
        o.rect(x + r, y, w - 2.0 * r, h, color);
        o.circle(x + r, y + r, r, color);
        o.circle(x + w - r, y + r, r, color);
    } else {
        o.rect(x, y + r, w, h - 2.0 * r, color);
        o.circle(x + r, y + r, r, color);
        o.circle(x + r, y + h - r, r, color);
    }
}

/// A point of the pad (pad units about its centre) on screen.
fn at(cx: f32, cy: f32, u: f32, p: [f32; 2]) -> [f32; 2] {
    [cx + p[0] * u, cy + p[1] * u]
}

/// The pad's body, centred on (`cx`, `cy`), `u` pixels a pad unit: the triggers and bumpers behind
/// it (named as `device` names them), the lobes and grips, the slab. Its buttons come after
/// (`draw_pad_buttons`), so callout lines drawn between meet their edges.
pub(super) fn draw_pad_body(o: &mut Overlay, cx: f32, cy: f32, u: f32, device: Device) {
    let lw = (u * 0.22).max(1.5);
    for s in [-1.0f32, 1.0] {
        for (r, label_y, b) in [(data::TRIGGER, -5.55, s < 0.0), (data::BUMPER, -4.25, s < 0.0)] {
            let x = if s < 0.0 { r[0] } else { -r[0] - r[2] };
            let [px, py] = at(cx, cy, u, [x, r[1]]);
            pill(o, px - lw, py - lw, r[2] * u + 2.0 * lw, r[3] * u + 2.0 * lw, rgba(EDGE_GREY, 1.0));
            pill(o, px, py, r[2] * u, r[3] * u, rgba(CAP, 1.0));
            let button = match (r == data::TRIGGER, b) {
                (true, true) => PadButton::LeftTrigger,
                (true, false) => PadButton::RightTrigger,
                (false, true) => PadButton::LeftBumper,
                (false, false) => PadButton::RightBumper,
            };
            let name = crate::input::control_name(Control::Pad(button), device);
            let lx = cx + (x + r[2] / 2.0) * u;
            o.text(&name, lx, cy + label_y * u, u * 0.62, rgba(SHINE, 1.0), Align::Center, false);
        }
    }
    for pass in 0..2 {
        let (grow, color) = if pass == 0 { (lw, EDGE_GREY) } else { (0.0, BODY) };
        for &(x, y, r) in &data::BODY_CIRCLES {
            let [sx, sy] = at(cx, cy, u, [x, y]);
            o.circle(sx, sy, r * u + grow, rgba(color, 1.0));
        }
        let s = data::BODY_SLAB;
        let [rx, ry] = at(cx, cy, u, [s[0], s[1]]);
        o.rect(rx - grow, ry - grow, s[2] * u + 2.0 * grow, s[3] * u + 2.0 * grow, rgba(color, 1.0));
    }
    // A soft highlight along the top of the body.
    let [hx, hy] = at(cx, cy, u, [-4.0, -4.0]);
    o.rect(hx, hy, 8.0 * u, 0.18 * u, rgba(SHINE, 0.35));
}

/// The pad's sticks, D-pad, face buttons (marked as `device` marks them) and the three in the middle.
pub(super) fn draw_pad_buttons(o: &mut Overlay, cx: f32, cy: f32, u: f32, device: Device) {
    let lw = (u * 0.22).max(1.5);
    let p = |q: [f32; 2]| at(cx, cy, u, q);
    for s in [p(data::LEFT_STICK), p(data::RIGHT_STICK)] {
        disc(o, s[0], s[1], 1.55 * u, DARK, EDGE_GREY, lw * 0.6);
        disc(o, s[0], s[1], 1.1 * u, CAP, SHINE, lw * 0.4);
        o.circle(s[0], s[1], 0.35 * u, rgba(DARK, 1.0));
    }
    let dpad = p(data::DPAD);
    disc(o, dpad[0], dpad[1], 1.75 * u, DARK, EDGE_GREY, lw * 0.4);
    pill(o, dpad[0] - 1.35 * u, dpad[1] - 0.45 * u, 2.7 * u, 0.9 * u, rgba(CAP, 1.0));
    pill(o, dpad[0] - 0.45 * u, dpad[1] - 1.35 * u, 0.9 * u, 2.7 * u, rgba(CAP, 1.0));
    o.circle(dpad[0], dpad[1], 0.25 * u, rgba(DARK, 1.0));
    for b in [PadButton::North, PadButton::East, PadButton::South, PadButton::West] {
        let [bx, by] = p(Part::Button(b).anchor());
        glyphs::draw_control(o, Control::Pad(b), device, bx - 0.68 * u, by, 1.36 * u);
    }
    let (view, menu, home) = (p(data::VIEW), p(data::MENU), p(data::HOME));
    disc(o, home[0], home[1], 0.7 * u, DARK, SHINE, lw * 0.5);
    for b in [view, menu] {
        disc(o, b[0], b[1], 0.42 * u, DARK, EDGE_GREY, lw * 0.5);
    }
    // The View button's two panes, the Menu button's three lines.
    o.rect_outline(view[0] - 0.2 * u, view[1] - 0.14 * u, 0.3 * u, 0.22 * u, (u * 0.06).max(1.0), rgba(SHINE, 1.0));
    for k in 0..3 {
        o.rect(menu[0] - 0.2 * u, menu[1] - 0.15 * u + k as f32 * 0.14 * u, 0.4 * u, (u * 0.06).max(1.0), rgba(SHINE, 1.0));
    }
}

/// A part's glyph (its button as `device` draws it; the D-pad's name) `size` tall, its left edge at
/// `x`: its width. Measured only, with `draw` false.
fn part_glyph(o: &mut Overlay, part: Part, device: Device, x: f32, y: f32, size: f32, draw: bool) -> f32 {
    let c = match part {
        Part::Button(b) => Control::Pad(b),
        Part::LeftStick => Control::Pad(PadButton::LeftStick),
        Part::RightStick => Control::Pad(PadButton::RightStick),
        Part::DPad => {
            let s = Style::body(size * 0.62, rgba(SHINE, 1.0)).bold();
            if draw {
                return o.text_styled("D-pad", x, y, &s);
            }
            return o.measure_styled("D-pad", &s);
        }
    };
    if draw {
        glyphs::draw_control(o, c, device, x, y, size)
    } else {
        glyphs::control_width(o, c, device, size)
    }
}

/// The pad, centred on (`cx`, `cy`) and `u` pixels a pad unit, with every part called out to what
/// it does in play (from `input`'s bindings), its glyph as `device` draws it, in text `size` tall,
/// the labels out to `reach` pad units either side. The callouts' widest reach, in pixels from
/// the centre.
pub fn draw_pad_legend(o: &mut Overlay, input: &crate::input::Input, device: Device, area: [f32; 4], size: f32) -> [f32; 2] {
    let device = if device.is_pad() { device } else { Device::Xbox };
    let callouts = data::callouts(&input.bindings);
    let style = Style::body(size, rgba(TEXT, 1.0));
    let glyph = size * 1.35;
    let gap = size * 0.5;
    // The widest label each side (its words, a gap, its glyph, a gap), so the pad and its labels
    // fit `area` together: the pad as large as the room left allows, and the area's height.
    let label = |o: &mut Overlay, parts: &[Part]| {
        parts
            .iter()
            .filter_map(|&p| callouts.iter().find(|c| c.part == p).map(|c| (p, c.does.clone())))
            .map(|(p, does)| o.measure_styled(&does, &style) + 2.0 * gap + part_glyph(o, p, device, 0.0, 0.0, glyph, false))
            .fold(0.0f32, f32::max)
    };
    let (lw, rw) = (label(o, &Part::LEFT), label(o, &Part::RIGHT));
    let [ax, ay, aw, ah] = area;
    let u = ((aw - lw - rw) / 22.4).min(ah / 13.6).max(4.0);
    let cx = ax + lw + 11.2 * u + ((aw - lw - rw - 22.4 * u) / 2.0).max(0.0);
    let cy = ay + ah / 2.0 + 0.4 * u;
    draw_pad_body(o, cx, cy, u, device);
    let (top, bottom) = (cy - 6.4 * u, cy + 5.6 * u);
    for (side, parts) in [(-1.0f32, &Part::LEFT[..]), (1.0, &Part::RIGHT[..])] {
        let edge = cx + side * 11.2 * u;
        let n = parts.len();
        for (i, &part) in parts.iter().enumerate() {
            let Some(c) = callouts.iter().find(|c| c.part == part) else { continue };
            let y = top + (bottom - top) * (i as f32 + 0.5) / n as f32;
            let a = at(cx, cy, u, part.anchor());
            o.circle(a[0], a[1], (u * 0.13).max(2.0), rgba(LINE, 0.95));
            let elbow = [edge - side * 0.8 * u, y];
            o.line(a, elbow, (u * 0.06).max(1.0), rgba(LINE, 0.6));
            o.line(elbow, [edge, y], (u * 0.06).max(1.0), rgba(LINE, 0.6));
            let gw = part_glyph(o, part, device, 0.0, y, glyph, false);
            let dim = c.actions.is_empty() && !matches!(part, Part::LeftStick | Part::RightStick);
            let style = style.color(rgba(if dim { TEXT_DIM } else { TEXT }, 1.0));
            let tw = o.measure_styled(&c.does, &style);
            if side < 0.0 {
                let gx = edge - gap - gw;
                part_glyph(o, part, device, gx, y, glyph, true);
                o.text_styled(&c.does, gx - gap - tw, y, &style);
            } else {
                let gx = edge + gap;
                part_glyph(o, part, device, gx, y, glyph, true);
                o.text_styled(&c.does, gx + gw + gap, y, &style);
            }
        }
    }
    draw_pad_buttons(o, cx, cy, u, device);
    // (Where the pad's top sits, over its triggers: for a heading.)
    [cx, cy - 6.6 * u]
}

/// The keyboard and mouse column in `area` (its rows spread down it): each row's keys as keycaps and
/// what they do, text `size` tall. Where it ends.
pub fn draw_key_column(o: &mut Overlay, input: &crate::input::Input, area: [f32; 4], size: f32) -> f32 {
    let [x, y, width, height] = area;
    let rows = data::key_rows(&input.bindings);
    let kbm = Device::KeyboardMouse;
    let glyph = size * 1.35;
    // (Spread down the room there is, up to half as much again as a glyph.)
    let row_h = (height / (rows.len() + 1) as f32).clamp(glyph * 1.15, glyph * 1.6);
    let style = Style::body(size, rgba(TEXT, 1.0));
    let gap = size * 0.3;
    let label_x = x + width * 0.44;
    let mut y = y + row_h * 0.5;
    // The mouse's own row: it turns the camera.
    glyphs::prompt_text_on(o, input, kbm, "{Look}", x, y, &style);
    o.text_styled("Camera", label_x, y, &style);
    y += row_h;
    for row in rows {
        let mut pen = x;
        let range = row.actions.len() == 5 && row.controls.len() == 2;
        for (k, &c) in row.controls.iter().enumerate() {
            if k > 0 && range {
                pen += gap + o.text_styled("-", pen + gap, y, &style) + gap;
            }
            if pen + glyphs::control_width(o, c, kbm, glyph) > label_x - gap {
                break;
            }
            pen += glyphs::draw_control(o, c, kbm, pen, y, glyph) + gap;
        }
        o.text_styled(&row.does, label_x, y, &style);
        y += row_h;
    }
    y - row_h * 0.5
}

/// The in-game loading card (`draw_loading`, once a load takes long enough to need it): over the
/// black, the level's name, the legend (the pad and its callouts, the keyboard and mouse's column,
/// the device used last lit), and a tip from the old man for this load, all faded in by `show`.
pub fn draw_loading_card(o: &mut Overlay, g: &Game, l: &Loading, t: f32, show: f32) {
    let (w, h) = (o.width, o.height);
    o.rect(0.0, 0.0, w, h, [0.0, 0.0, 0.0, 1.0]);
    // The level's name in the display type, and the wait's pulse.
    let title = Style::display(t * 1.15, rgba(GOLD, 0.95 * show)).centered().tracking(0.06).shadow(2.0, 3.0);
    o.text_styled(&l.label.to_uppercase(), w / 2.0, t * 2.0, &title);
    let pulse = 0.45 + 0.35 * ((l.age as f32) * 5.0).sin().abs();
    let small = Style::body(t * 0.5, rgba(SAND, pulse * show)).centered().tracking(0.2);
    o.text_styled("LOADING", w / 2.0, t * 3.1, &small);
    style::divider(o, w / 2.0 - t * 4.0, t * 3.6, t * 8.0, t, rgba(COPPER, 0.8 * show));

    // The legend: smaller than the start screen's, the pad left of the middle and the keys right.
    let device = g.input.last_device();
    let size = (t * 0.6).max(11.0);
    let tip_h = t * 3.4;
    let (top, bottom) = (t * 4.9, h - tip_h - t * 1.4);
    let key_w = (w * 0.25).min(t * 14.0);
    let kx = w - key_w - t * 1.2;
    let used = |pad: bool| if device.is_pad() == pad { rgba(GOLD, show) } else { rgba(TEXT_DIM, 0.8 * show) };
    let head = |o: &mut Overlay, text: &str, x: f32, y: f32, color: [f32; 4], align: Align| {
        let s = Style::body(t * 0.5, color).bold().tracking(0.12).align(align);
        o.text_styled(text, x, y, &s);
    };
    let area = [t * 1.2, top + t * 0.6, kx - t * 2.4, bottom - top - t * 0.6];
    let [cx, pad_top] = draw_pad_legend(o, &g.input, device, area, size);
    head(o, if device.is_pad() { "CONTROLLER  ·  USED LAST" } else { "CONTROLLER" }, cx, pad_top - t * 0.6, used(true), Align::Center);
    head(o, if device.is_pad() { "KEYBOARD AND MOUSE" } else { "KEYBOARD AND MOUSE  ·  USED LAST" }, kx, top, used(false), Align::Left);
    style::divider(o, kx, top + t * 0.45, key_w, t, rgba(COPPER, 0.7 * show));
    draw_key_column(o, &g.input, [kx, top + t * 0.8, key_w, bottom - top - t * 0.8], size);

    // The old man's tip, in his amber frame, turning with each load.
    let prose = prefs().prose;
    let tw = (w * 0.62).min(t * 34.0);
    let (tx, ty) = ((w - tw) / 2.0, h - tip_h - t * 0.6);
    // (In his amber, as his dialogue is.)
    style::panel_ruled(o, [tx, ty, tw, tip_h], t, show, AMBER);
    let name = Style::display(t * 0.55, rgba(AMBER, show)).tracking(0.08);
    o.text_styled("THE OLD MAN SAYS", tx + t * 0.6, ty + t * 0.65, &name);
    let tip = glyphs::words(&g.input, data::tip(g.transitions.len()));
    let words = Style::body(t * 0.62 * prose, rgba(TEXT, show));
    o.wrap_styled(&tip, tx + t * 0.6, ty + t * 1.55, tw - t * 1.2, t * 0.8 * prose, &words);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_loading_card_draws_the_legend_and_a_tip_at_each_size() {
        let mut g = Game::new();
        g.loading =
            Some(Loading { label: "The Garden".into(), started: web_time::Instant::now(), load_ms: 0.0, age: 2.0, cold: true, hold: 0.0 });
        for (w, h) in [(1280.0, 720.0), (1920.0, 1080.0), (3840.0, 2160.0)] {
            for device in [Device::KeyboardMouse, Device::Xbox, Device::PlayStation] {
                g.input.set_last_device(device);
                let mut o = Overlay::new();
                o.begin(w, h);
                let t = tile_for(w, h);
                draw_loading_card(&mut o, &g, g.loading.as_ref().unwrap(), t, 1.0);
                assert!(o.verts.len() > 2000, "{w}x{h} {device:?}: {} vertices", o.verts.len());
            }
        }
    }
}
