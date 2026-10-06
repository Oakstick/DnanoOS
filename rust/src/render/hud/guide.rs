//! The old man's guide on screen (B): over the paused game, a glass page ruled in his amber, his
//! live portrait 9 units square on a warm ground beside the place's name, and his words in four
//! parts: the tale, what you must do, what to watch for, and what is so here now.

use super::style::{self, size};
use super::*;
use crate::game::counsel::AMBER;

/// The page's parts: (heading, its lines, the heading's colour).
type Part<'a> = (&'a str, &'a [String], u32);

/// The portrait's size, in units (UI.md §6).
pub const FACE: f32 = 9.0;

pub(super) fn draw_guide(o: &mut Overlay, g: &Game, t: f32) {
    let (w, h) = (o.width, o.height);
    let (guide, now) = g.guide_now();
    o.rect(0.0, 0.0, w, h, [INK[0], INK[1], INK[2], 0.8]);
    let safe = style::safe(w, h);
    let pw = (t * 46.0).min(w - safe * 2.0);
    let px = (w - pw) / 2.0;
    let room = h - safe * 2.0;
    let pad = t * 0.8;

    // His words, part by part, in a column beside his face; smaller only if they would overflow.
    let parts: [Part; 4] = [
        ("THE TALE", &guide.tale, AMBER),
        ("WHAT YOU MUST DO", &guide.task, GOLD),
        ("WHAT TO WATCH FOR", &guide.watch, 0x9ad1ff),
        ("HERE, NOW", &now, 0x7dff9a),
    ];
    let face = t * FACE;
    let col_x = px + pad + face + t * 0.9;
    let col_w = px + pw - pad - col_x;
    let lines: Vec<(&str, Vec<String>, u32)> = parts
        .iter()
        .filter(|p| !p.1.is_empty())
        .map(|(head, ls, c)| (*head, ls.iter().map(|l| glyphs::words(&g.input, l)).collect(), *c))
        .collect();
    // (The guide is prose: at the size the player chose for it, smaller only to fit.)
    let mut units = size::BODY * 0.92 * prefs().prose;
    let height = |o: &Overlay, units: f32| {
        let st = style::body(t, units, [1.0; 4]);
        let line = t * units * 1.36;
        let mut y = t * 3.1;
        for (_, ls, _) in &lines {
            y += t * 1.25;
            for l in ls {
                y += o.wrap_lines(l, col_w - t * 0.6, &st).len() as f32 * line + line * 0.2;
            }
            y += t * 0.35;
        }
        y + t * 1.6
    };
    while units > 0.5 && height(o, units) > room {
        units *= 0.94;
    }
    let ph = height(o, units).max(face + pad * 2.0 + t * 2.4).min(room);
    let py = (h - ph) / 2.0;
    style::panel_ruled(o, [px, py, pw, ph], t, 1.0, AMBER);

    // His face on a warm ground, framed in gold, and who is speaking under it.
    let (fx, fy) = (px + pad, py + pad);
    o.rect(fx, fy, face, face, [0.03, 0.02, 0.01, 0.85]);
    style::glow(o, fx + face * 0.42, fy + face * 0.4, face * 0.75, rgba(0xd08a3a, 0.42));
    let pose = crate::render::portrait::Pose::live(g.time as f32, 1.0);
    o.portrait(crate::render::portrait::PortraitId::OldMan, pose, fx, fy, face, face, false, [1.0; 4]);
    o.rect_outline(fx, fy, face, face, (t * 0.06).max(1.0), rgba(0xf2c46a, 0.9));
    for (cx, cy) in [(fx, fy), (fx + face, fy), (fx, fy + face), (fx + face, fy + face)] {
        style::diamond(o, cx, cy, t * 0.13, rgba(0xf2c46a, 1.0));
    }
    let under_face = style::label(t, 0.58, rgba(AMBER, 1.0)).centered();
    o.text_styled("THE OLD MAN SPEAKS", fx + face / 2.0, fy + face + t * 0.7, &under_face);
    let dog = style::body(t, 0.56, rgba(TEXT_DIM, 1.0)).centered();
    o.text_styled("Quitmer lies at his feet while he talks.", fx + face / 2.0, fy + face + t * 1.45, &dog);

    // The place's name, then the parts.
    let title = style::name(t, 1.05, rgba(GOLD, 1.0));
    o.text_styled(&guide.title, col_x, py + pad + t * 0.75, &title);
    style::divider(o, col_x, py + pad + t * 1.65, col_w, t, rgba(COPPER, 0.7));
    let st = style::body(t, units, rgba(TEXT, 1.0));
    let line = t * units * 1.36;
    let mut y = py + t * 3.1;
    let bottom = py + ph - t * 1.4;
    for (head, ls, color) in &lines {
        if y > bottom {
            break;
        }
        let hs = style::label(t, size::LABEL, rgba(*color, 1.0)).tracking(0.04);
        let hw = o.text_styled(head, col_x, y, &hs);
        o.rect(col_x, y + t * 0.42, hw, (t * 0.04).max(1.0), rgba(*color, 0.5));
        y += t * 1.25;
        for l in ls {
            if y > bottom {
                break;
            }
            style::diamond(o, col_x + t * 0.12, y, t * 0.09, rgba(*color, 0.85));
            y = o.wrap_styled(l, col_x + t * 0.5, y, col_w - t * 0.6, line, &st) + line * 0.2;
        }
        y += t * 0.35;
    }
    let back = style::label(t, 0.58, rgba(TEXT_DIM, 1.0)).centered();
    glyphs::prompt_text(o, &g.input, "{Back} BACK TO THE ROAD", px + pw / 2.0, py + ph - t * 0.65, &back);
}
