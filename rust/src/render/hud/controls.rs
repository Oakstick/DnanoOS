//! The controls page, opened from the title or the pause menu: two tabs, the
//! controller and the keyboard. The controller is a gamepad drawn in the
//! game's colours, every button called out by a line to what it does (by
//! their Xbox names; a line under the pad gives their PlayStation twins). The
//! keyboard is every key as a keycap beside what it does.

use super::*;

const BODY: u32 = 0x2a2e38;
const EDGE: u32 = 0x626a7c;
const DARK: u32 = 0x15171d;
const CAP: u32 = 0x3e4452;
const SHINE: u32 = 0x8a93a6;
const A_GREEN: u32 = 0x5cc84a;
const B_RED: u32 = 0xe0544a;
const X_BLUE: u32 = 0x4a8ee8;
const Y_YELLOW: u32 = 0xf2c53d;
const LINE: u32 = 0x9aa3b8;

/// The two tabs at the top of the page: the controller, then the keyboard and mouse.
pub fn controls_tab_rects(w: f32, h: f32) -> [[f32; 4]; 2] {
    let t = tile_for(w, h);
    let (bw, bh, gap) = (t * 8.0, t * 1.1, t * 0.4);
    let y = t * 2.55;
    [[w / 2.0 - bw - gap / 2.0, y, bw, bh], [w / 2.0 + gap / 2.0, y, bw, bh]]
}

/// The keyboard and mouse, as keycaps beside what they do, in two columns.
fn draw_keyboard(o: &mut Overlay, t: f32) {
    let (w, h) = (o.width, o.height);
    let keys: [(&str, &str); 18] = [
        ("W A S D", "Run (arrows too), relative to the camera"),
        ("SPACE", "Jump; pick in menus"),
        ("LEFT CLICK", "Attack (E too)"),
        ("B", "The old man's guide to this place"),
        ("SHIFT", "Hold: raise your shield"),
        ("F", "Bifrost Step, once learned"),
        ("R", "Archangel's power"),
        ("Q", "Drink the chosen potion"),
        ("TAB", "Next potion; shop: next tab"),
        ("1 - 5", "Drink from that belt slot"),
        ("MOUSE", "Look (click the game to capture it)"),
        ("C", "Switch the camera"),
        ("H", "Call the old man"),
        ("ESC", "Pause and resume"),
        ("ENTER", "Pick in menus"),
        ("F4", "Free or grid movement"),
        ("M", "Sound on or off"),
        ("F11", "Fullscreen"),
    ];
    let size = (t * 0.52).max(11.0);
    let (top, row) = (t * 5.0, ((h - t * 9.5) / 9.0).min(t * 1.35));
    let col_w = (w * 0.42).min(t * 22.0);
    let x0 = w / 2.0 - col_w - t * 0.5;
    for (i, (key, does)) in keys.iter().enumerate() {
        let (cx, cy) = (x0 + (i / 9) as f32 * (col_w + t * 1.0), top + (i % 9) as f32 * row);
        // The keycap: a dark rounded key with a lighter rim and its name on it.
        let kw = o.measure(key, size) + t * 0.9;
        let (kx, ky, kh) = (cx, cy, row * 0.78);
        o.rect(kx - 1.0, ky - 1.0, kw + 2.0, kh + 2.0, rgba(EDGE, 1.0));
        o.rect(kx, ky, kw, kh, rgba(CAP, 1.0));
        o.rect(kx, ky + kh * 0.82, kw, kh * 0.18, rgba(DARK, 0.6));
        o.text(key, kx + kw / 2.0, ky + kh * 0.45, size, rgba(0xf0ebe0, 1.0), Align::Center, true);
        o.text(does, kx + kw + t * 0.45, ky + kh * 0.45, size * 0.95, rgba(TEXT, 1.0), Align::Left, false);
    }
}

/// Where the Back button sits on the controller page.
fn controls_back_rect(w: f32, h: f32) -> [f32; 4] {
    let t = tile_for(w, h);
    let (bw, bh) = (t * 6.0, t * 1.3);
    [w / 2.0 - bw / 2.0, h - bh - t * 0.9, bw, bh]
}

/// A filled circle with a rim a little lighter than it, the way the pad's plastic catches the light.
fn disc(o: &mut Overlay, x: f32, y: f32, r: f32, fill: u32, rim: u32, lw: f32) {
    o.circle(x, y, r + lw, rgba(rim, 1.0));
    o.circle(x, y, r, rgba(fill, 1.0));
}

/// A rectangle with rounded ends (a bumper, a trigger, the d-pad's arms).
fn pill(o: &mut Overlay, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
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

/// One button called out: where it is on the pad, its name, its colour and what it does.
struct Callout {
    at: [f32; 2],
    name: &'static str,
    color: u32,
    does: &'static str,
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
    o.text("Left and right, LT and RT, or Tab switch between them", w / 2.0, t * 4.2, t * 0.42, rgba(TEXT_DIM, 1.0), Align::Center, false);
    if g.controls_keyboard {
        draw_keyboard(o, t);
        draw_back(o, g, t);
        return;
    }

    // ---- the pad, about 20 units wide and 13 tall, centred a little above the middle
    let u = (w * 0.33).min(h * 0.58) / 20.0;
    let (cx, cy) = (w / 2.0, h * 0.5);
    let p = |x: f32, y: f32| [cx + x * u, cy + y * u];
    let lw = (u * 0.22).max(1.5);

    // Triggers and bumpers behind the body.
    for s in [-1.0f32, 1.0] {
        let [tx, ty] = p(s * 6.4 - 1.5, -6.3);
        pill(o, tx - lw, ty - lw, 3.0 * u + 2.0 * lw, 1.5 * u + 2.0 * lw, rgba(EDGE, 1.0));
        pill(o, tx, ty, 3.0 * u, 1.5 * u, rgba(CAP, 1.0));
        let [bx, by] = p(s * 6.0 - 2.3, -4.8);
        pill(o, bx - lw, by - lw, 4.6 * u + 2.0 * lw, 1.1 * u + 2.0 * lw, rgba(EDGE, 1.0));
        pill(o, bx, by, 4.6 * u, 1.1 * u, rgba(CAP, 1.0));
    }
    let (lt, rt, lb, rb) = (p(-6.4, -5.55), p(6.4, -5.55), p(-6.0, -4.25), p(6.0, -4.25));
    for (at, name) in [(lt, "LT"), (rt, "RT"), (lb, "LB"), (rb, "RB")] {
        o.text(name, at[0], at[1], u * 0.62, rgba(SHINE, 1.0), Align::Center, false);
    }

    // The body: a rounded slab, two lobes and the grips, rimmed first and then filled.
    let shapes: [(f32, f32, f32); 6] =
        [(-5.6, -1.4, 3.4), (5.6, -1.4, 3.4), (-6.3, 2.4, 3.0), (6.3, 2.4, 3.0), (-7.0, 4.3, 2.3), (7.0, 4.3, 2.3)];
    for pass in 0..2 {
        let (grow, color) = if pass == 0 { (lw, EDGE) } else { (0.0, BODY) };
        for &(x, y, r) in &shapes {
            let [sx, sy] = p(x, y);
            o.circle(sx, sy, r * u + grow, rgba(color, 1.0));
        }
        let [rx, ry] = p(-5.6, -4.3);
        o.rect(rx - grow, ry - grow, 11.2 * u + 2.0 * grow, 6.3 * u + 2.0 * grow, rgba(color, 1.0));
    }
    // A soft highlight along the top of the body.
    let [hx, hy] = p(-4.0, -4.0);
    o.rect(hx, hy, 8.0 * u, 0.18 * u, rgba(SHINE, 0.35));

    // Where the sticks, the d-pad, the face buttons and the three in the middle sit.
    let (ls, rs, dpad, face) = (p(-5.4, -1.5), p(2.9, 1.7), p(-2.9, 1.7), p(5.4, -1.5));
    let (view, menu, guide) = (p(-1.6, -2.0), p(1.6, -2.0), p(0.0, -3.3));
    // The triggers and bumpers are called out from their outer ends, clear of their names.
    let (lt_at, rt_at, lb_at, rb_at) = (p(-7.7, -5.55), p(7.7, -5.55), p(-8.1, -4.25), p(8.1, -4.25));

    // ---- what each does, called out to either side (drawn under the buttons, so each line meets its button's edge)
    let face_at = |dx: f32, dy: f32| [face[0] + dx * u, face[1] + dy * u];
    let left = [
        Callout { at: lt_at, name: "LT", color: SHINE, does: "Archangel's power; shop: tab back" },
        Callout { at: lb_at, name: "LB", color: SHINE, does: "Hold: raise your shield" },
        Callout { at: view, name: "View", color: SHINE, does: "Call the old man" },
        Callout { at: ls, name: "Left stick", color: SHINE, does: "Run; press: Bifrost Step" },
        Callout { at: dpad, name: "D-pad", color: SHINE, does: "Run; choose in menus" },
    ];
    let right = [
        Callout { at: rt_at, name: "RT", color: SHINE, does: "Attack; shop: next tab" },
        Callout { at: rb_at, name: "RB", color: SHINE, does: "Drink the chosen potion" },
        Callout { at: menu, name: "Menu", color: SHINE, does: "Pause and resume; pick in menus" },
        Callout { at: face_at(0.0, -1.3), name: "Y", color: Y_YELLOW, does: "Next potion on the belt" },
        Callout { at: face_at(1.3, 0.0), name: "B", color: B_RED, does: "The old man's guide; back in menus" },
        Callout { at: face_at(-1.3, 0.0), name: "X", color: X_BLUE, does: "Attack" },
        Callout { at: face_at(0.0, 1.3), name: "A", color: A_GREEN, does: "Jump; pick in menus" },
        Callout { at: rs, name: "Right stick", color: SHINE, does: "Look; press: recentre, or switch camera" },
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
            let name_w = o.measure(c.name, size);
            if side < 0.0 {
                o.text(c.name, edge - gap * 0.4, y, size, rgba(c.color, 1.0), Align::Right, true);
                o.text(c.does, edge - gap * 0.4 - name_w - gap, y, size * 0.92, rgba(TEXT, 1.0), Align::Right, false);
            } else {
                o.text(c.name, edge + gap * 0.4, y, size, rgba(c.color, 1.0), Align::Left, true);
                o.text(c.does, edge + gap * 0.4 + name_w + gap, y, size * 0.92, rgba(TEXT, 1.0), Align::Left, false);
            }
        }
    }

    // Sticks, d-pad, face buttons and the three in the middle.
    for s in [ls, rs] {
        disc(o, s[0], s[1], 1.55 * u, DARK, EDGE, lw * 0.6);
        disc(o, s[0], s[1], 1.1 * u, CAP, SHINE, lw * 0.4);
        o.circle(s[0], s[1], 0.35 * u, rgba(DARK, 1.0));
    }
    disc(o, dpad[0], dpad[1], 1.75 * u, DARK, EDGE, lw * 0.4);
    pill(o, dpad[0] - 1.35 * u, dpad[1] - 0.45 * u, 2.7 * u, 0.9 * u, rgba(CAP, 1.0));
    pill(o, dpad[0] - 0.45 * u, dpad[1] - 1.35 * u, 0.9 * u, 2.7 * u, rgba(CAP, 1.0));
    o.circle(dpad[0], dpad[1], 0.25 * u, rgba(DARK, 1.0));
    let buttons = [("Y", 0.0, -1.3, Y_YELLOW), ("B", 1.3, 0.0, B_RED), ("A", 0.0, 1.3, A_GREEN), ("X", -1.3, 0.0, X_BLUE)];
    for &(name, dx, dy, color) in &buttons {
        let (bx, by) = (face[0] + dx * u, face[1] + dy * u);
        disc(o, bx, by, 0.62 * u, color, DARK, lw * 0.6);
        o.text(name, bx, by, u * 0.62, rgba(DARK, 1.0), Align::Center, true);
    }
    disc(o, guide[0], guide[1], 0.7 * u, DARK, SHINE, lw * 0.5);
    for b in [view, menu] {
        disc(o, b[0], b[1], 0.42 * u, DARK, EDGE, lw * 0.5);
    }
    // The View button's two panes, the Menu button's three lines.
    o.rect_outline(view[0] - 0.2 * u, view[1] - 0.14 * u, 0.3 * u, 0.22 * u, (u * 0.06).max(1.0), rgba(SHINE, 1.0));
    for k in 0..3 {
        o.rect(menu[0] - 0.2 * u, menu[1] - 0.15 * u + k as f32 * 0.14 * u, 0.4 * u, (u * 0.06).max(1.0), rgba(SHINE, 1.0));
    }

    // ---- notes, and the way back
    let notes_y = cy + 7.4 * u;
    o.text("In menus: the d-pad or left stick chooses, A or Menu picks.", w / 2.0, notes_y, t * 0.5, rgba(SAND, 1.0), Align::Center, false);
    o.text(
        "On a PlayStation pad: A is Cross, B Circle, X Square, Y Triangle, View is Share and Menu is Options.",
        w / 2.0,
        notes_y + t * 0.9,
        t * 0.45,
        rgba(TEXT_DIM, 1.0),
        Align::Center,
        false,
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
    o.text("B, Esc or a click to go back", w / 2.0, r[1] + r[3] + t * 0.45, t * 0.42, rgba(TEXT_DIM, 1.0), Align::Center, false);
}
