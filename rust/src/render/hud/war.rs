//! The general on screen (WARBAND.md §9, §13): his banner at the top centre (his sigil, his name,
//! his three orders, each ready, counting down, under way, broken or done), the order cards that
//! slide in under it ("THE ASSESSOR MUSTERS - 4"), a ping over each instrument still whole (and at
//! the screen's edge for the one whose order is under way), the gate glowing for a muster with how
//! far he has barred it, and the general's name over his head. All in the charcoal-and-copper
//! panels of the rest of the HUD.

use super::*;
use crate::game::general::{GeneralRun, CARD_SECONDS};
use crate::logic::war::general::{InstrumentKind, OrderStage, BAR_FRAMES};

/// The colours of an order's states.
const READY: u32 = 0x8a8478;
const COUNTING: u32 = 0xffc040;
const BROKEN: u32 = 0xc8382a;
const DONE: u32 = 0x6a6a72;

/// Everything of the general's on screen, while he is in a realm with one.
pub(super) fn draw_war(o: &mut Overlay, g: &Game, t: f32) {
    if !matches!(g.screen, Screen::Playing | Screen::Ready | Screen::Paused) {
        return;
    }
    let Some(r) = g.general.as_ref() else { return };
    draw_pings(o, g, r, t);
    draw_banner(o, g, r, t);
    draw_cards(o, g, r, t);
}

/// An order's state as a colour, a pulse (0..1) and the seconds left on its count, if counting.
fn state_look(stage: OrderStage, time: f32, color: u32) -> (u32, f32, Option<u32>) {
    match stage {
        OrderStage::Ready => (READY, 0.0, None),
        OrderStage::Announced { left } => (COUNTING, 0.5 + 0.5 * (time * 9.0).sin(), Some(left.div_ceil(60))),
        OrderStage::Live { .. } => (color, 0.3 + 0.3 * (time * 4.0).sin(), None),
        OrderStage::Broken => (BROKEN, 0.0, None),
        OrderStage::Done => (DONE, 0.0, None),
    }
}

/// His banner: a charcoal panel at the top centre, his colour hanging from its left as a pennant
/// with his sin's sigil on it, his name, and his three orders as discs.
fn draw_banner(o: &mut Overlay, g: &Game, r: &GeneralRun, t: f32) {
    let w = o.width;
    let time = g.time as f32;
    let gen = r.general();
    let margin = t * 0.4;
    let bw = (t * 15.0).min(w - t * 22.0).max(t * 11.0);
    let bh = t * 2.3;
    let x0 = w / 2.0 - bw / 2.0;
    let y0 = margin;
    panel(o, x0, y0, bw, bh, t);
    // The pennant: his cloak's colour hanging from the panel's rule, his sin's sigil on it.
    let pw = t * 1.5;
    let px = x0 + t * 0.35;
    o.rect(px, y0 + t * 0.2, pw, bh - t * 0.4, rgba(gen.cloak, 0.95));
    o.rect(px, y0 + t * 0.2, pw, t * 0.12, rgba(COPPER, 1.0));
    let sc = (px + pw / 2.0, y0 + t * 0.95);
    o.circle(sc.0, sc.1, t * 0.48, [0.0, 0.0, 0.0, 0.55]);
    o.circle(sc.0, sc.1, t * 0.4, rgba(r.sin.color(), 1.0));
    o.text(&r.sin.title()[..1], sc.0, sc.1, t * 0.5, rgba(0x14100c, 1.0), Align::Center, true);
    // His name, and what he holds.
    let tx = px + pw + t * 0.45;
    let slain = r.command.slain;
    let name_w = bw - (tx - x0) - t * 4.4;
    fit_text(o, &r.name, tx, y0 + t * 0.72, t * 0.6, name_w, rgba(if slain { TEXT_DIM } else { GOLD }, 1.0), true);
    let line = if slain {
        "SLAIN - HIS ORDERS ARE ENDED".to_string()
    } else if r.usurper.is_some() {
        format!("USURPER OF {}'S POST", gen.name)
    } else {
        format!("GENERAL OF {}", r.sin.prince().to_uppercase())
    };
    fit_text(o, &line, tx, y0 + t * 1.45, t * 0.42, name_w, rgba(if slain { BROKEN } else { SAND }, 1.0), false);
    if slain {
        let y = y0 + t * 0.72;
        o.line([tx, y], [tx + o.measure(&r.name, t * 0.6).min(name_w), y], (t * 0.08).max(1.5), rgba(BROKEN, 0.9));
    }
    // The three orders.
    let rr = t * 0.42;
    for k in 0..3 {
        let cx = x0 + bw - t * 0.75 - (2 - k) as f32 * t * 1.2;
        let cy = y0 + t * 0.95;
        let (color, pulse, count) = state_look(r.command.stages[k], time, gen.cloak);
        if pulse > 0.0 {
            o.circle(cx, cy, rr * (1.25 + 0.2 * pulse), rgba(color, 0.25 * pulse));
        }
        o.circle(cx, cy, rr * 1.1, [0.0, 0.0, 0.0, 0.6]);
        o.circle(cx, cy, rr, rgba(color, 1.0));
        o.circle(cx, cy, rr * 0.72, [INK[0], INK[1], INK[2], 0.9]);
        o.text(gen.orders[k].letter(), cx, cy, t * 0.46, rgba(color, 1.0), Align::Center, true);
        match r.command.stages[k] {
            OrderStage::Broken => {
                let d = rr * 0.8;
                o.line([cx - d, cy - d], [cx + d, cy + d], (t * 0.09).max(1.5), rgba(BROKEN, 1.0));
                o.line([cx - d, cy + d], [cx + d, cy - d], (t * 0.09).max(1.5), rgba(BROKEN, 1.0));
            }
            OrderStage::Done => o.rect(cx - rr * 0.8, cy - t * 0.04, rr * 1.6, (t * 0.08).max(1.5), rgba(DONE, 1.0)),
            _ => {}
        }
        if let Some(n) = count {
            o.text(&n.to_string(), cx, cy + rr + t * 0.32, t * 0.4, rgba(COUNTING, 1.0), Align::Center, true);
        }
    }
}

/// `text` at `size`, or smaller until it fits in `max` (never under 60%), left-aligned.
fn fit_text(o: &mut Overlay, text: &str, x: f32, y: f32, size: f32, max: f32, color: [f32; 4], bold: bool) {
    let mut s = size;
    while s > size * 0.6 && o.measure(text, s) > max {
        s *= 0.92;
    }
    o.text(text, x, y, s, color, Align::Left, bold);
}

/// The order cards, under the banner: each slides down into place and fades as it goes; an order's
/// card counts down to its bite.
fn draw_cards(o: &mut Overlay, g: &Game, r: &GeneralRun, t: f32) {
    let w = o.width;
    let mut y = t * 0.4 + t * 2.3 + t * 0.25;
    for c in r.cards.iter().rev() {
        let age = (g.time - c.since) as f32;
        let slide = 1.0 - (age / 0.22).min(1.0);
        let alpha = (age / 0.15).min(1.0) * ((CARD_SECONDS as f32 - age) / 0.45).clamp(0.0, 1.0);
        if alpha <= 0.0 {
            continue;
        }
        let text = match c.order.map(|k| r.command.stages[k]) {
            Some(OrderStage::Announced { left }) => format!("{} - {}", c.text, left.div_ceil(60)),
            _ => c.text.clone(),
        };
        let cw = (o.measure(&text, t * 0.62).max(o.measure(&c.sub, t * 0.42)) + t * 1.6).min(w - t);
        let ch = t * 1.55;
        let x0 = w / 2.0 - cw / 2.0;
        let yy = y - slide * t * 0.9;
        o.alpha = alpha;
        panel(o, x0, yy, cw, ch, t);
        o.rect(x0, yy, t * 0.22, ch, rgba(c.color, 1.0));
        o.text(&text, w / 2.0, yy + t * 0.55, t * 0.62, rgba(GOLD, 1.0), Align::Center, true);
        o.text(&c.sub, w / 2.0, yy + t * 1.13, t * 0.42, rgba(SAND, 1.0), Align::Center, false);
        o.alpha = 1.0;
        y += ch + t * 0.15;
    }
}

/// Where a point of the world falls on screen (its floor and `lift` above).
fn on_screen(g: &Game, p: crate::logic::types::V, lift: f32, w: f32, h: f32) -> Option<(f32, f32)> {
    let (x, z) = (p.x as f32, p.y as f32);
    let at = Vec3::new(x, floor_at(&g.level.grid, x, z) + lift, z);
    g.view.project(at, w, h)
}

/// A ping over each instrument still whole (a pulsing one for the order under way, and an arrow
/// at the screen's edge if that one is off it), the glowing gate with its barring, and the
/// general's name over his head.
fn draw_pings(o: &mut Overlay, g: &Game, r: &GeneralRun, t: f32) {
    let (w, h) = (o.width, o.height);
    let time = g.time as f32;
    let gen = r.general();
    let hp = g.hero.pos();
    for k in 0..3 {
        let i = r.command.instruments[k];
        if i.broken {
            continue;
        }
        let at = i.tile.v();
        let busy = matches!(r.command.stages[k], OrderStage::Announced { .. } | OrderStage::Live { .. });
        let near = dist(at, hp) < 22.0;
        if !busy && !near {
            continue;
        }
        let pulse = if busy { 0.5 + 0.5 * (time * 8.0).sin() } else { 0.0 };
        let color = if busy { COUNTING } else { gen.cloak };
        let inside = on_screen(g, at, 1.6, w, h).filter(|&(x, y)| x > t && y > t * 3.0 && x < w - t && y < h - t * 2.5);
        match inside {
            Some((x, y)) => {
                let s = t * (0.32 + 0.12 * pulse);
                o.circle(x, y, s * 1.5, [0.0, 0.0, 0.0, 0.45]);
                o.circle(x, y, s * 1.2, rgba(color, 0.9));
                o.circle(x, y, s * 0.85, [INK[0], INK[1], INK[2], 0.95]);
                o.text(gen.orders[k].letter(), x, y, t * 0.38, rgba(color, 1.0), Align::Center, true);
                let label = match i.kind {
                    InstrumentKind::Bellows if i.turned => "TURNED".to_string(),
                    InstrumentKind::Bellows if dist(at, hp) < 2.5 => format!("{} BLOWS - F TO TURN", i.blows),
                    _ if dist(at, hp) < 6.0 => format!("{} BLOWS", i.blows),
                    _ => String::new(),
                };
                if !label.is_empty() {
                    o.text(&label, x, y - t * 0.75, t * 0.38, rgba(SAND, 0.95), Align::Center, true);
                }
            }
            None if busy => edge_arrow(o, g, at, color, t),
            None => {}
        }
    }
    // The gate glowing for the muster: a ring, and how far he has barred it.
    if let Some((at, barred)) = r.glowing() {
        match on_screen(g, at, 0.2, w, h).filter(|&(x, y)| x > 0.0 && y > 0.0 && x < w && y < h) {
            Some((x, y)) => {
                let pulse = 0.5 + 0.5 * (time * 6.0).sin();
                o.circle(x, y, t * (0.9 + 0.2 * pulse), rgba(0xff8a3a, 0.18));
                o.arc(x, y, t * 0.8, 0.0, std::f32::consts::TAU, (t * 0.08).max(1.5), rgba(0xff8a3a, 0.9));
                if barred > 0.0 {
                    o.arc(
                        x,
                        y,
                        t * 0.8,
                        -std::f32::consts::FRAC_PI_2,
                        -std::f32::consts::FRAC_PI_2 + barred * std::f32::consts::TAU,
                        (t * 0.16).max(2.5),
                        rgba(0x9ad1ff, 1.0),
                    );
                }
                let label = if barred > 0.0 {
                    format!("BARRING {}%", (barred * 100.0).round())
                } else {
                    "THE GATE - STAND HERE TO BAR IT".to_string()
                };
                o.text(&label, x, y - t * 1.2, t * 0.42, rgba(0xffc08a, 1.0), Align::Center, true);
                let _ = BAR_FRAMES;
            }
            None => edge_arrow(o, g, at, 0xff8a3a, t),
        }
    }
    // A caught herald's map: his post marked, wherever the hero is.
    if r.post_known && !r.command.slain {
        let at = r.posts.post.v();
        match on_screen(g, at, 1.8, w, h).filter(|&(x, y)| x > t && y > t * 3.0 && x < w - t && y < h - t * 2.5) {
            Some((x, y)) => {
                o.circle(x, y, t * 0.42, rgba(gen.cloak, 0.9));
                o.text("POST", x, y - t * 0.8, t * 0.38, rgba(GOLD, 1.0), Align::Center, true);
            }
            None => edge_arrow(o, g, at, gen.cloak, t),
        }
    }
    // His name over his head, when he is near.
    if let Some(d) = r.id.and_then(|id| g.enemies.iter().filter_map(|e| e.duel.as_deref()).find(|d| d.id == id)) {
        if d.alive() && dist(d.pos, hp) < 16.0 {
            let top = crate::render::view::crown(d.breed.kind).unwrap_or(1.6) + 0.9;
            if let Some((x, y)) = on_screen(g, d.pos, top, w, h) {
                let s = t * 0.5;
                let tw = o.measure(&r.name, s) + t * 0.6;
                o.rect(x - tw / 2.0, y - t * 0.38, tw, t * 0.72, [INK[0], INK[1], INK[2], 0.7]);
                o.rect(x - tw / 2.0, y + t * 0.3, tw, (t * 0.05).max(1.0), rgba(gen.cloak, 1.0));
                o.text(&r.name, x, y, s, rgba(GOLD, 1.0), Align::Center, true);
            }
        }
    }
}

fn dist(a: crate::logic::types::V, b: crate::logic::types::V) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

/// An arrow at the screen's edge, on the way from him to `at`.
fn edge_arrow(o: &mut Overlay, g: &Game, at: crate::logic::types::V, color: u32, t: f32) {
    let (w, h) = (o.width, o.height);
    let hp = g.hero.pos();
    // His own place on screen, and the target's (or a point toward it) seen from there.
    let Some((hx, hy)) = on_screen(g, hp, 1.0, w, h) else { return };
    let toward = {
        let dx = at.x - hp.x;
        let dy = at.y - hp.y;
        let l = (dx * dx + dy * dy).sqrt().max(1e-6);
        crate::logic::types::V::new(hp.x + dx / l * 4.0, hp.y + dy / l * 4.0)
    };
    let Some((tx, ty)) = on_screen(g, toward, 1.0, w, h) else { return };
    let (dx, dy) = (tx - hx, ty - hy);
    let l = (dx * dx + dy * dy).sqrt().max(1e-3);
    let (ux, uy) = (dx / l, dy / l);
    // Out from the middle until the edge (kept clear of the HUD's top band).
    let (cx, cy) = (w / 2.0, h / 2.0);
    let (ex, ey) = ((w / 2.0 - t * 1.2) / ux.abs().max(1e-3), (h / 2.0 - t * 4.4) / uy.abs().max(1e-3));
    let k = ex.min(ey);
    let (x, y) = (cx + ux * k, cy + uy * k);
    let s = t * 0.55;
    let tip = [x + ux * s, y + uy * s];
    let left = [x - uy * s * 0.6, y + ux * s * 0.6];
    let right = [x + uy * s * 0.6, y - ux * s * 0.6];
    o.line(left, tip, (t * 0.14).max(2.0), rgba(color, 1.0));
    o.line(right, tip, (t * 0.14).max(2.0), rgba(color, 1.0));
    o.circle(x - ux * s * 0.4, y - uy * s * 0.4, t * 0.16, rgba(color, 0.8));
}
