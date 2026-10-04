//! The end's overlay (FINALE.md §4): the white the platform rises into, the words on it one line
//! at a time (dark ink on white turning to a soft gold, held long enough to read), the white
//! going down to the dark, and the credits rolling up out of it to THE END. And on the title,
//! once the game has been finished, a small mark of it.

use super::*;
use crate::logic::story::{ascent, credits, Credit, Shot, STANZA_BREAKS, WORDS};

/// The ink of the words on the white, and the soft gold the white turns to under them.
const INK_WORDS: u32 = 0x3a2c1e;
const CREAM: u32 = 0xf6ecd6;
/// The dark the credits roll on: not quite black, warm.
const DUSK: u32 = 0x0d0b09;

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// `a` mixed `k` of the way to `b`, as a colour with alpha.
fn blend(a: u32, b: u32, k: f32, alpha: f32) -> [f32; 4] {
    let (x, y) = (rgba(a, alpha), rgba(b, alpha));
    let k = k.clamp(0.0, 1.0);
    [x[0] + (y[0] - x[0]) * k, x[1] + (y[1] - x[1]) * k, x[2] + (y[2] - x[2]) * k, alpha]
}

/// Over Scene II's picture: the white as the platform nears the top, then the words and the
/// credits on their own ground. `bt` is seconds into the beat.
pub(super) fn draw_ending(o: &mut Overlay, g: &Game, t: f32, shot: Shot, bt: f32) {
    let (w, h) = (o.width, o.height);
    match shot {
        Shot::Rise => {
            let white = smooth(ascent::WHITEOUT, crate::logic::story::SCENE_TWO[2].hold as f32, bt);
            if white > 0.0 {
                o.rect(0.0, 0.0, w, h, [1.0, 0.985, 0.96, white]);
            }
        }
        Shot::White => o.rect(0.0, 0.0, w, h, [1.0, 0.985, 0.96, 1.0]),
        Shot::Words => draw_words(o, t, bt),
        Shot::Credits => draw_credits(o, g, t, bt),
        _ => {}
    }
}

/// The poem, gathering line by line, centred, on the white turning to cream; then all of it
/// fading and the white going down into the dark.
fn draw_words(o: &mut Overlay, t: f32, bt: f32) {
    let (w, h) = (o.width, o.height);
    let warm = smooth(0.0, 14.0, bt);
    let dark = smooth(ascent::WORDS_FADE + 1.0, ascent::WORDS_FADE + 3.8, bt);
    let ground = blend(0xfffbf4, CREAM, warm, 1.0);
    let ground =
        [ground[0] * (1.0 - dark) + 0.051 * dark, ground[1] * (1.0 - dark) + 0.043 * dark, ground[2] * (1.0 - dark) + 0.035 * dark, 1.0];
    o.rect(0.0, 0.0, w, h, ground);
    let shadow = std::mem::replace(&mut o.shadow, 0.0);
    let out = 1.0 - smooth(ascent::WORDS_FADE, ascent::WORDS_FADE + 1.6, bt);
    let size = (t * 1.15).min(w / 36.0);
    let lh = size * 1.75;
    let gap = size * 1.1;
    let first_size = size * 1.3;
    let total = lh * (WORDS.len() as f32 - 1.0) + gap * STANZA_BREAKS.len() as f32 + first_size * 0.4;
    let mut y = h * 0.5 - total / 2.0;
    for (i, line) in WORDS.iter().enumerate() {
        if STANZA_BREAKS.contains(&i) {
            y += gap;
        }
        let at = ascent::WORDS_AT[i];
        let a = smooth(at, at + ascent::LINE_IN, bt) * out;
        let drift = (1.0 - smooth(at, at + ascent::LINE_IN * 1.3, bt)) * size * 0.35;
        let s = if i == 0 { first_size } else { size };
        if a > 0.0 {
            o.text(line, w / 2.0, y + drift, s, blend(INK_WORDS, 0x5a4428, warm, a), Align::Center, i == 0);
        }
        y += if i == 0 { lh + first_size * 0.4 } else { lh };
    }
    // When the last line has settled, a thin gold rule under the poem.
    let rule = smooth(ascent::WORDS_AT[5] + 2.5, ascent::WORDS_AT[5] + 4.5, bt) * out;
    if rule > 0.0 {
        let half = size * 3.5 * rule;
        let c = blend(0xb08a48, 0xc8a060, warm, 0.8 * rule);
        o.line([w / 2.0 - half, y + gap * 0.3], [w / 2.0 + half, y + gap * 0.3], (t * 0.06).max(1.2), c);
        let r = t * 0.18;
        let (cx, cy) = (w / 2.0, y + gap * 0.3);
        for (a, b) in
            [([cx - r, cy], [cx, cy - r]), ([cx, cy - r], [cx + r, cy]), ([cx + r, cy], [cx, cy + r]), ([cx, cy + r], [cx - r, cy])]
        {
            o.line(a, b, (t * 0.06).max(1.2), c);
        }
    }
    o.shadow = shadow;
}

/// How tall one credit stands on the roll.
fn credit_height(c: &Credit, t: f32) -> f32 {
    match c {
        Credit::Title => t * 5.6,
        Credit::Heading(_) => t * 1.6,
        Credit::Name(..) => t * 2.0,
        Credit::Line(_) => t * 1.5,
        Credit::Gap => t * 2.2,
    }
}

/// The credits rolling up out of the dark, then THE END.
fn draw_credits(o: &mut Overlay, g: &Game, t: f32, bt: f32) {
    let (w, h) = (o.width, o.height);
    let time = g.time as f32;
    o.rect(0.0, 0.0, w, h, rgba(DUSK, 1.0));
    let list = credits();
    let total: f32 = list.iter().map(|c| credit_height(c, t)).sum();
    let u = ((bt - ascent::ROLL_FROM) / ascent::ROLL_SECONDS).clamp(0.0, 1.0);
    // From just below the screen to just above it.
    let mut y = h + t * 1.0 - (h + total + t * 2.0) * u;
    let edge = |y: f32| smooth(0.0, h * 0.14, y) * smooth(0.0, h * 0.14, h - y);
    for c in &list {
        let ch = credit_height(c, t);
        let cy = y + ch / 2.0;
        y += ch;
        if cy < -t * 4.0 || cy > h + t * 4.0 {
            continue;
        }
        let a = edge(cy);
        if a <= 0.0 {
            continue;
        }
        match c {
            Credit::Title => {
                o.text("ADVENTURES OF THE", w / 2.0, cy - t * 1.9, t * 0.8, rgba(SAND, a), Align::Center, true);
                draw_logo_word(o, "ANCIENT BLADES", w / 2.0, cy, t * 2.2, time);
                // The logo draws at full strength: veil it near the edges as the rest fades.
                o.rect(0.0, cy - t * 1.6, w, t * 3.2, rgba(DUSK, 1.0 - a));
                draw_ornament(o, w / 2.0, cy + t * 1.9, t * 11.0, t, time);
            }
            Credit::Heading(text) => {
                o.text(text, w / 2.0, cy, t * 0.6, rgba(GOLD, a * 0.95), Align::Center, true);
            }
            Credit::Name(name, role) => {
                o.text(name, w / 2.0, cy - t * 0.38, t * 0.95, rgba(TEXT, a), Align::Center, true);
                o.text(role, w / 2.0, cy + t * 0.55, t * 0.52, rgba(TEXT_DIM, a * 0.9), Align::Center, false);
            }
            Credit::Line(text) => {
                o.text(text, w / 2.0, cy, t * 0.75, rgba(SAND, a), Align::Center, false);
            }
            Credit::Gap => {}
        }
    }
    // THE END, alone in the middle once the roll has gone by.
    let end = smooth(ascent::THE_END, ascent::THE_END + 2.0, bt);
    if end > 0.0 {
        let cy = h * 0.47;
        o.text("THE END", w / 2.0, cy, t * 1.6, rgba(GOLD, end), Align::Center, true);
        draw_ornament(o, w / 2.0, cy + t * 1.5, t * 9.0 * end, t, time);
    }
}

/// On the title, once the game has been finished: a small gold diamond and a line, top left.
pub(super) fn draw_finished_mark(o: &mut Overlay, g: &Game, t: f32) {
    if !g.finished || g.confirm_reset {
        return;
    }
    let (x, y) = (t * 0.9, o.height * 0.035);
    let r = t * 0.2;
    let lw = (t * 0.07).max(1.2);
    let c = rgba(GOLD, 0.9);
    o.line([x - r, y], [x, y - r], lw, c);
    o.line([x, y - r], [x + r, y], lw, c);
    o.line([x + r, y], [x, y + r], lw, c);
    o.line([x, y + r], [x - r, y], lw, c);
    o.text("THE TALE IS TOLD", x + t * 0.55, y, t * 0.5, rgba(SAND, 0.85), Align::Left, false);
}
