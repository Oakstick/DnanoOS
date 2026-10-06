//! Scene I's overlay: letterbox bars, the scene's title card over black, a
//! card naming each place as its scene opens, the dialogue box (the speaker's
//! name and stage direction on a tab, the line typing itself out, a marker
//! when it has all shown), the narrator's closing words across the bottom,
//! and the flashes and fades between the two worlds.

use super::*;
use crate::game::TYPE_DELAY;
use crate::logic::story::{ascent, stage, Scene, Shot, Speaker, SCENE_TITLE};

/// The colour each speaker's name (and the box's edge) is drawn in.
fn voice_color(s: Speaker) -> u32 {
    match s {
        Speaker::Boys => 0xf2c46a,
        Speaker::OldMan => 0xa8c4e8,
        Speaker::Dog => 0xe0604a,
        Speaker::Narrator => 0xe8dcc0,
    }
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Splits `text` into lines no wider than `max` at `size`, breaking between words.
fn wrap(o: &Overlay, text: &str, max: f32, size: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split(' ') {
        let trial = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if o.measure(&trial, size) > max && !line.is_empty() {
            lines.push(std::mem::take(&mut line));
            line = word.to_string();
        } else {
            line = trial;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/**
 * Draws the first `shown` letters of `text`, wrapped as the whole of it will be (so a word
 * never jumps to the next line as it types), each line placed by `align` at `x`. `shake`
 * jostles the letters (a scream, a snarl).
 */
#[allow(clippy::too_many_arguments)]
fn typed(
    o: &mut Overlay,
    text: &str,
    shown: usize,
    x: f32,
    y: f32,
    max: f32,
    size: f32,
    lh: f32,
    color: [f32; 4],
    align: Align,
    shake: f32,
    time: f32,
) {
    let mut left = shown;
    for (i, line) in wrap(o, text, max, size).iter().enumerate() {
        if left == 0 {
            break;
        }
        let n = line.chars().count().min(left);
        left = left.saturating_sub(line.chars().count() + 1);
        let part: String = line.chars().take(n).collect();
        let ly = y + i as f32 * lh;
        if shake > 0.0 {
            // Letter by letter, each jittering on its own.
            let style = o.legacy_style(size, color, Align::Left, true);
            let (pens, full) = o.pens(line, &style);
            let pen = match align {
                Align::Left => x,
                Align::Center => x - full / 2.0,
                Align::Right => x - full,
            };
            for (k, (ch, at)) in part.chars().zip(pens).enumerate() {
                let s = ch.to_string();
                let jx = (time * 47.0 + k as f32 * 1.7).sin() * shake;
                let jy = (time * 53.0 + k as f32 * 2.3).cos() * shake;
                o.text_styled(&s, pen + at + jx, ly + jy, &style);
            }
        } else {
            let full = o.measure(line, size);
            let px = match align {
                Align::Left => x,
                Align::Center => x - full / 2.0,
                Align::Right => x - full,
            };
            o.text(&part, px, ly, size, color, Align::Left, false);
        }
    }
}

pub(super) fn draw_story(o: &mut Overlay, g: &Game, t: f32) {
    let Some(st) = &g.story else { return };
    let (w, h) = (o.width, o.height);
    let time = g.time as f32;
    let beat = st.beat();
    let bt = st.t as f32;
    let age = st.age as f32;

    // ---- flashes and fades between the worlds
    let mut black = 0.0f32;
    let mut white = 0.0f32;
    match beat.shot {
        Shot::Title => black = 1.0 - smooth(2.7, 3.4, bt),
        Shot::Pursuit => white = smooth(stage::FLASH, stage::FLASH + 0.7, bt),
        Shot::Arrive => white = 1.0 - smooth(0.0, 0.9, bt),
        // Scene II comes up out of the black the fight left.
        Shot::Embrace => black = 1.0 - smooth(0.0, ascent::FADE_IN, bt),
        _ => {}
    }
    if black > 0.0 {
        o.rect(0.0, 0.0, w, h, [0.0, 0.0, 0.0, black]);
    }

    // ---- letterbox bars, sliding in as the scene opens (and away into the white at the end)
    let title = st.scene.title();
    let open = if st.scene == Scene::Two { 1.0 - smooth(0.0, 1.5, st.scene_time() as f32 - 24.5) } else { 1.0 };
    let k = smooth(0.0, 0.6, age) * open;
    let bar = h * 0.1 * k;
    o.rect(0.0, 0.0, w, bar, [0.0, 0.0, 0.0, 1.0]);
    o.rect(0.0, h - bar, w, bar, [0.0, 0.0, 0.0, 1.0]);
    super::glyphs::prompt_line(o, &g.input, "{Pause} SKIP", w - t * 0.9, bar / 2.0, t * 0.5, rgba(TEXT_DIM, k * 0.8), Align::Right);
    o.text(&format!("{} - {}", title.sub, title.title), t * 0.9, bar / 2.0, t * 0.5, rgba(SAND, k * 0.6), Align::Left, false);

    // ---- the scene's title over black
    if beat.shot == Shot::Title {
        let a = smooth(0.35, 1.1, bt) * (1.0 - smooth(2.4, 3.1, bt));
        let cy = h * 0.46;
        o.text(SCENE_TITLE.sub, w / 2.0, cy - t * 2.3, t * 0.75, rgba(GOLD, a), Align::Center, true);
        if a > 0.01 {
            let shadow = o.shadow;
            draw_logo_word(o, SCENE_TITLE.title, w / 2.0, cy, t * 2.4, time);
            o.shadow = shadow;
            // The logo draws at full strength: veil it while the card fades.
            o.rect(0.0, cy - t * 2.0, w, t * 4.0, [0.0, 0.0, 0.0, (1.0 - a) * black]);
        }
        draw_ornament(o, w / 2.0, cy + t * 1.7, t * 12.0 * a.max(0.01), t, time);
        o.text("ADVENTURES OF THE ANCIENT BLADES", w / 2.0, cy + t * 2.7, t * 0.5, rgba(SAND, a * 0.8), Align::Center, false);
    }

    // ---- the card naming a place
    if let Some(card) = beat.card {
        let start = if beat.shot == Shot::Arrive { 1.0 } else { 0.9 };
        let a = smooth(start, start + 0.6, bt) * (1.0 - smooth(start + 3.4, start + 4.1, bt));
        if a > 0.0 {
            let (x, y) = (t * 1.6, h - bar - t * 2.6);
            o.rect(x - t * 0.5, y - t * 0.2, t * 0.12, t * 1.7, rgba(GOLD, 0.85 * a));
            o.text(card.title, x, y + t * 0.2, t * 1.0, rgba(0xf2e3c4, a), Align::Left, true);
            o.text(card.sub, x, y + t * 1.15, t * 0.55, rgba(SAND, a * 0.85), Align::Left, false);
        }
    }

    // ---- the words
    if !beat.line.is_empty() && bt > TYPE_DELAY as f32 * 0.5 {
        let shown = st.shown as usize;
        let appear = smooth(0.0, 0.25, bt);
        let shake = if matches!(beat.shot, Shot::Scream | Shot::Snarl) { t * 0.06 * (1.0 - smooth(1.5, 3.0, bt)) } else { 0.0 };
        if beat.speaker == Speaker::Narrator {
            // The narrator speaks across the foot of the picture, over a soft dark band.
            let size = t * 0.72;
            let lh = t * 1.1;
            let max = (w * 0.66).min(t * 40.0);
            let lines = wrap(o, beat.line, max, size).len() as f32;
            let top = h - bar - t * 1.1 - lines * lh;
            let bands = 10;
            for i in 0..bands {
                let v = i as f32 / (bands - 1) as f32;
                let y0 = top - t * 1.2 + v * (h - top + t * 1.2);
                o.rect(0.0, y0, w, (h - top + t * 1.2) / bands as f32 + 1.0, [0.0, 0.0, 0.0, 0.55 * v * appear]);
            }
            typed(
                o,
                beat.line,
                shown,
                w / 2.0,
                top + lh * 0.5,
                max,
                size,
                lh,
                rgba(voice_color(beat.speaker), appear),
                Align::Center,
                0.0,
                time,
            );
            if st.line_done() {
                let pulse = 0.5 + 0.5 * (time * 4.0).sin();
                super::glyphs::prompt_line(
                    o,
                    &g.input,
                    "{Confirm} CONTINUE",
                    w / 2.0,
                    h - bar / 2.0,
                    t * 0.45,
                    rgba(TEXT_DIM, pulse),
                    Align::Center,
                );
            }
        } else {
            let color = voice_color(beat.speaker);
            let bw = (w * 0.7).min(t * 36.0);
            let bh = t * 3.7;
            let bx = (w - bw) / 2.0;
            let by = h - bar * 0.55 - bh;
            let rise = (1.0 - appear) * t * 0.6;
            let by = by + rise;
            o.rect(bx, by, bw, bh, [4.0 / 255.0, 6.0 / 255.0, 10.0 / 255.0, 0.86 * appear]);
            o.rect_outline(bx, by, bw, bh, 1.0, [1.0, 1.0, 1.0, 0.16 * appear]);
            o.rect(bx, by, t * 0.14, bh, rgba(color, 0.9 * appear));
            // The name on a tab above the box's corner, the stage direction after it.
            let name = beat.speaker.name();
            let size = t * 0.62;
            let tab_w = o.measure(name, size) + t * 1.2;
            o.rect(bx, by - t * 1.05, tab_w, t * 1.05, [4.0 / 255.0, 6.0 / 255.0, 10.0 / 255.0, 0.86 * appear]);
            o.rect(bx, by - t * 1.05, tab_w, t * 0.08, rgba(color, 0.9 * appear));
            o.text(name, bx + t * 0.6, by - t * 0.5, size, rgba(color, appear), Align::Left, true);
            if !beat.cue.is_empty() {
                o.text(&format!("({})", beat.cue), bx + tab_w + t * 0.4, by - t * 0.5, t * 0.5, rgba(TEXT_DIM, appear), Align::Left, false);
            }
            let tx = bx + t * 0.8;
            typed(o, beat.line, shown, tx, by + t * 0.85, bw - t * 1.6, t * 0.68, t * 0.95, rgba(TEXT, appear), Align::Left, shake, time);
            if st.line_done() {
                // A little arrow bobbing in the corner: the rest is yours to move on.
                let (ax, ay) = (bx + bw - t * 0.8, by + bh - t * 0.55 + (time * 5.0).sin() * t * 0.1);
                let c = rgba(color, 0.9 * appear);
                let s = t * 0.28;
                o.line([ax - s, ay - s * 0.6], [ax, ay + s * 0.4], (t * 0.08).max(1.5), c);
                o.line([ax + s, ay - s * 0.6], [ax, ay + s * 0.4], (t * 0.08).max(1.5), c);
            }
        }
    }

    if white > 0.0 {
        o.rect(0.0, 0.0, w, h, [1.0, 1.0, 1.0, white]);
    }
    if st.scene == Scene::Two {
        super::ending::draw_ending(o, g, t, beat.shot, bt);
        if beat.shot == Shot::Credits && bt > 2.0 && st.ending.is_none() {
            let pulse = 0.35 + 0.25 * (time * 2.0).sin();
            super::glyphs::prompt_line(
                o,
                &g.input,
                "{Confirm} CONTINUE",
                w - t * 0.9,
                h - t * 0.9,
                t * 0.45,
                rgba(TEXT_DIM, pulse),
                Align::Right,
            );
        }
    }
    let fade = st.fade_out();
    if fade > 0.0 {
        o.rect(0.0, 0.0, w, h, [0.0, 0.0, 0.0, fade]);
    }
}
