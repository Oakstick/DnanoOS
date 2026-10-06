//! The two voices on screen (WARBAND.md §10, §13): the panel in the bottom-left corner where the
//! old man speaks (his live portrait in an amber frame, his name, his words typed out in amber) and where
//! the prince's whisper speaks under his own name (an empty frame, the name in the sin's colour, the
//! words in the sin's colour); and in a realm, the signs about the hero: the realm's compass needle
//! (pale, or turned toward a den, flickering in the sin's colour) and the old man's amber lamp,
//! always true, both shown only while he stands idle.

use super::*;
use crate::game::counsel::{Speaker, AMBER};
use crate::logic::types::V;

/// The panel's rectangle (x, y, width, height) for `lines` lines of words: over the bottom bar, on
/// the left.
pub fn panel_rect(w: f32, h: f32, lines: usize) -> [f32; 4] {
    let t = tile_for(w, h);
    let pw = (w * 0.4).min(t * 22.0);
    let ph = (t * 1.75 + lines as f32 * t * 0.56).max(t * 3.6);
    [t * 0.4, h - t * 3.9 - ph, pw, ph]
}

/// How many lines `text` takes at `size` within `max`.
fn lines_of(o: &Overlay, text: &str, size: f32, max: f32) -> usize {
    let mut lines = 1;
    let mut line = String::new();
    for word in text.split_whitespace() {
        let next = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if o.measure(&next, size) > max && !line.is_empty() {
            lines += 1;
            line = word.to_string();
        } else {
            line = next;
        }
    }
    lines
}

/// The line in the panel, typed out so far.
pub(super) fn draw_voices(o: &mut Overlay, g: &Game, t: f32) {
    let Some(s) = g.voices_line() else { return };
    let small = (t * 0.5).max(11.0);
    let probe = panel_rect(o.width, o.height, 1);
    let words_w = probe[2] - (probe[3] - t * 0.6) - t * 1.1;
    let [x, y, pw, ph] = panel_rect(o.width, o.height, lines_of(o, &s.text, small * 0.92, words_w));
    let age = (g.time - s.since) as f32;
    let fade = (age * 4.0).min(1.0);
    let color = s.color();
    let whisper = matches!(s.who, Speaker::Whisper(_));
    o.rect(x, y, pw, ph, [INK[0], INK[1], INK[2], 0.88 * fade]);
    o.rect_outline(x, y, pw, ph, (t * 0.06).max(1.0), rgba(if whisper { color } else { COPPER }, 0.6 * fade));
    o.rect(x + t * 0.3, y, pw - t * 0.6, (t * 0.07).max(1.0), rgba(color, 0.9 * fade));
    // The portrait: his face in an amber frame; for the whisper, an empty frame and a shadow.
    let fs = t * 3.0;
    let (fx, fy) = (x + t * 0.3, y + t * 0.3);
    o.rect(fx, fy, fs, fs, [0.0, 0.0, 0.0, 0.6 * fade]);
    if whisper {
        // No face: a smudge of the sin's colour, drifting.
        let drift = (g.time as f32 * 0.7).sin() * fs * 0.05;
        o.circle(fx + fs / 2.0 + drift, fy + fs * 0.45, fs * 0.22, rgba(color, 0.18 * fade));
        o.circle(fx + fs / 2.0 - drift, fy + fs * 0.5, fs * 0.12, rgba(color, 0.12 * fade));
    } else {
        // His live portrait, made by the renderer: every frame while his words type out (his head
        // and his hand move with them), once a second while they stand.
        use crate::render::portrait::{PortraitId, Pose};
        let talking = s.shown(g.time) < s.text.chars().count();
        let pose = if talking { Pose::talking(g.time as f32, age) } else { Pose::live(g.time as f32, 1.0) };
        o.portrait(PortraitId::OldMan, pose, fx, fy, fs, fs, false, [1.0, 1.0, 1.0, fade]);
    }
    o.rect_outline(fx, fy, fs, fs, (t * 0.08).max(1.5), rgba(color, 0.95 * fade));
    // The name over the words: the old man's, or the prince's own (he never takes the old man's).
    let tx = fx + fs + t * 0.45;
    let tw = x + pw - tx - t * 0.35;
    let name = match s.who {
        Speaker::Whisper(sin) => format!("{} WHISPERS", sin.prince().to_uppercase()),
        Speaker::OldMan => "THE OLD MAN".to_string(),
    };
    o.text(&name, tx, y + t * 0.62, small, rgba(color, fade), Align::Left, true);
    if !whisper {
        // Quitmer's mark beside his name.
        o.text(
            "& QUITMER",
            tx + o.measure_as("THE OLD MAN ", small, true),
            y + t * 0.62,
            small * 0.8,
            rgba(TEXT_DIM, 0.8 * fade),
            Align::Left,
            false,
        );
    }
    let shown: String = s.text.chars().take(s.shown(g.time)).collect();
    wrap_left(o, &shown, tx, y + t * 1.35, tw, t * 0.56, small * 0.92, rgba(color, fade));
}

/// Where a world point lands on screen.
fn screen(g: &Game, p: V, up: f32, w: f32, h: f32) -> Option<(f32, f32)> {
    let (x, y) = (p.x as f32, p.y as f32);
    g.view.project(Vec3::new(x, floor_at(&g.level.grid, x, y) + up, y), w, h)
}

/// In a realm, while he stands idle: the compass about him (the realm's needle, the old man's lamp).
pub(super) fn draw_signs(o: &mut Overlay, g: &Game, t: f32) {
    if g.screen != Screen::Playing || !g.hero.alive || g.realm.is_none() || !(g.in_vault() || g.in_run()) {
        return;
    }
    // Only when he stands idle: the compass is for the moment he stops to look.
    let shown = g.compass_shown();
    if shown <= 0.0 {
        return;
    }
    let (w, h) = (o.width, o.height);
    let hp = g.hero.pos();
    let time = g.time as f32;
    // A unit step on screen toward a world point, from him.
    let bearing = |to: V| -> Option<(f32, f32)> {
        let (dx, dy) = (to.x - hp.x, to.y - hp.y);
        let l = (dx * dx + dy * dy).sqrt();
        if l < 0.5 {
            return None;
        }
        let a = screen(g, hp, 0.2, w, h)?;
        let b = screen(g, V::new(hp.x + dx / l, hp.y + dy / l), 0.2, w, h)?;
        let (sx, sy) = (b.0 - a.0, b.1 - a.1);
        let sl = (sx * sx + sy * sy).sqrt().max(1e-3);
        Some((sx / sl, sy / sl))
    };
    if let Some(c) = screen(g, hp, 0.2, w, h) {
        // The old man's lamp: a small amber diamond toward the far portal, always true.
        if let Some((ux, uy)) = g.gate.and_then(|p| bearing(p.v())) {
            let r = t * 4.5;
            let (lx, ly) = (c.0 + ux * r, c.1 + uy * r);
            let s = t * 0.34;
            let a = (0.85 + (time * 2.0).sin() * 0.1) * shown;
            o.line([lx, ly - s], [lx + s, ly], 2.5, rgba(AMBER, a));
            o.line([lx + s, ly], [lx, ly + s], 2.5, rgba(AMBER, a));
            o.line([lx, ly + s], [lx - s, ly], 2.5, rgba(AMBER, a));
            o.line([lx - s, ly], [lx, ly - s], 2.5, rgba(AMBER, a));
            o.circle(lx, ly, s * 0.35, rgba(AMBER, a));
        }
        // The realm's needle: pale stone toward the portal, or turned (the sin's colour, flickering).
        if let Some((to, turned)) = g.compass() {
            if let Some((ux, uy)) = bearing(to) {
                let sin = g.realm.unwrap_or(crate::logic::sins::Sin::Greed);
                let (color, alpha) = if turned {
                    let flicker = if ((time * 13.0).sin() + (time * 7.3).sin()) > 0.4 { 0.45 } else { 1.0 };
                    // (The prince's colour, never one that could pass for the old man's lamp.)
                    (crate::game::counsel::whisper_color(sin), flicker)
                } else {
                    (0xd7ccb8, 0.8)
                };
                let alpha = alpha * shown;
                let (r0, r1) = (t * 2.6, t * 3.7);
                let tip = (c.0 + ux * r1, c.1 + uy * r1);
                let (px, py) = (-uy, ux);
                let base = (c.0 + ux * r0, c.1 + uy * r0);
                let half = t * 0.32;
                let edges = [
                    ([base.0 + px * half, base.1 + py * half], [tip.0, tip.1]),
                    ([base.0 - px * half, base.1 - py * half], [tip.0, tip.1]),
                    ([base.0 + px * half, base.1 + py * half], [base.0 - px * half, base.1 - py * half]),
                ];
                // A dark rim under it, so it reads on gold floors too.
                for (a, b) in edges {
                    o.line(a, b, 6.5, [0.0, 0.0, 0.0, 0.6]);
                }
                for (a, b) in edges {
                    o.line(a, b, 3.0, rgba(color, alpha));
                }
            }
        }
    }
}
