//! The Ledger of Grudges on screen (WARBAND.md §8.4, §8.5, §13): a rival's name card beside him
//! when he is met (name and epithet, rank pips, breed, his lesson, the traits found and "?" for the
//! rest, his bark), and the Ledger board: a pinned note for each rival, his portrait drawn from his
//! breed's rig, his name and rank, the traits as found, his lesson and his last encounter. The
//! board opens from the pause menu (THE LEDGER), and the workshop's War Table hosts it through
//! `draw_ledger_board`.

use std::collections::HashMap;
use std::sync::Mutex;

use super::*;
use crate::game::ledger::CARD_SECONDS;
use crate::logic::war::ledger::{Card, Ledger, Rival};
use crate::render::voxels::Collected;

/// The lesson's colour: a warning, warmer than the copper.
const LESSON: u32 = 0xff7a4a;
/// The notes: old paper, the ink on it, its quieter ink, and the blood of a slain one.
const PAPER: u32 = 0xd9ccb0;
const PAPER_EDGE: u32 = 0x8a7458;
const INK_DARK: u32 = 0x2a2018;
const INK_SOFT: u32 = 0x5e5040;
const INK_RED: u32 = 0x9a2a1a;
/// The rank pips, lit and unlit.
const PIP: u32 = 0xeab55e;
const PIP_OFF: u32 = 0x4a4038;

/// The breed's rig, as `sprite_from` draws it.
fn rig_of(kind: EnemyKind) -> Collected {
    use crate::render::{foes, foes2, foes3, foes4, foes5, foes6, foes7, foes_court, foes_feast, foes_lair, foes_mire, foes_thorns};
    use EnemyKind as K;
    match kind {
        K::Skeleton => foes::draugr(),
        K::Bat => foes::raven(false),
        K::Vampire => foes::raven(true),
        K::Slime => foes::bog_slime(),
        K::Witch => foes2::witch(),
        K::Minion => foes2::minion(),
        K::Helmeted | K::Peerless => foes2::helmeted_skeleton(),
        K::Werewolf => foes2::loup_garou(),
        K::Blemmyes => foes2::blemmyes(),
        K::LargeBat => foes2::large_bat(),
        K::Tengu => foes3::tengu(),
        K::Rasetsu => foes3::rasetsu(),
        K::DireWolf => foes3::dire_wolf(),
        K::Jorogumo => foes3::jorogumo(),
        K::Cobra => foes3::cobra(),
        K::Kappa => foes4::kappa(),
        K::Rokurokubi => foes4::rokurokubi(),
        K::Oni => foes4::oni(),
        K::Kitsune => foes4::kitsune(),
        K::Tulukus => foes5::tulukus(),
        K::Albasti => foes5::albasti(),
        K::Cin => foes5::cin(),
        K::Arcura => foes5::arcura(),
        K::Karakoncolos => foes5::karakoncolos(),
        K::Tepegoz => foes6::tepegoz(),
        K::Gashadokuro => foes6::gashadokuro(),
        K::Ejder => foes6::ejder(),
        K::Goryo => foes6::goryo(),
        K::FrostGiant => foes7::frost_giant(),
        K::Surtr => foes7::surtr(),
        K::Nidhogg => foes7::nidhogg(),
        K::Hel => foes7::hel(),
        K::Ganglot => foes_mire::ganglot(),
        K::Gaki => foes_feast::gaki(),
        K::Ellepige => foes_thorns::ellepige(),
        K::Mammon => foes_court::mammon(),
        K::GoldGolem => foes_court::gold_golem(),
        K::Lucifer => foes_court::pride::lucifer(),
        K::Reflection => foes_court::pride::reflection(),
        K::Leviathan => foes_court::envy::leviathan(),
        K::Serpentling => foes_court::envy::serpentling(),
        K::Amon => foes_court::wrath::amon(),
        K::Raven => foes_court::wrath::raven(),
        K::Belphegor => foes_court::sloth::belphegor(),
        K::Nightmare => foes_court::sloth::nightmare(),
        K::Beelzebub => foes_court::gluttony::beelzebub(),
        K::FlySwarm => foes_court::gluttony::fly_swarm(),
        K::Asmodeus => foes_court::lust::asmodeus(),
        K::Krampus => foes_lair::krampus(),
        K::Percht => foes_lair::percht(),
    }
}

/// A sprite boxed down so its longer side is at most `max` pixels (each cell takes the colour
/// most of its pixels have), so a portrait costs a few hundred rects, not thousands.
fn shrink(s: Sprite, max: i32) -> Sprite {
    let k = ((s.width.max(s.height) + max - 1) / max).max(1);
    if k == 1 {
        return s;
    }
    let (w, h) = ((s.width + k - 1) / k, (s.height + k - 1) / k);
    let pixels = (0..h)
        .map(|y| {
            (0..w)
                .map(|x| {
                    let mut seen: Vec<(u32, u32)> = Vec::new();
                    let mut empty = 0;
                    for yy in y * k..((y + 1) * k).min(s.height) {
                        for xx in x * k..((x + 1) * k).min(s.width) {
                            match s.pixels[yy as usize][xx as usize] {
                                Some(c) => match seen.iter_mut().find(|e| e.0 == c) {
                                    Some(e) => e.1 += 1,
                                    None => seen.push((c, 1)),
                                },
                                None => empty += 1,
                            }
                        }
                    }
                    let filled: u32 = seen.iter().map(|e| e.1).sum();
                    (filled >= empty).then(|| seen.iter().max_by_key(|e| e.1).map(|e| e.0)).flatten()
                })
                .collect()
        })
        .collect();
    Sprite { width: w, height: h, pixels }
}

/// The portraits made so far, one per breed.
static PORTRAITS: Mutex<Option<HashMap<EnemyKind, Sprite>>> = Mutex::new(None);

/// Makes the portraits of `kinds` now (as a level loads), not the first time a card shows one.
pub(super) fn warm(kinds: &[EnemyKind]) {
    let mut guard = PORTRAITS.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    for &kind in kinds {
        map.entry(kind).or_insert_with(|| shrink(cropped(sprite_from(&rig_of(kind))), 44));
    }
}

/// Draws the portrait of `kind` fitted into the square at (x, y), made once per breed.
fn draw_portrait(o: &mut Overlay, kind: EnemyKind, x: f32, y: f32, size: f32, alpha: f32) {
    let mut guard = PORTRAITS.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    let s = map.entry(kind).or_insert_with(|| shrink(cropped(sprite_from(&rig_of(kind))), 44));
    let scale = (size * 0.92) / s.width.max(s.height) as f32;
    let (sw, sh) = (s.width as f32 * scale, s.height as f32 * scale);
    draw_sprite(o, s, x + (size - sw) / 2.0, y + (size - sh) / 2.0, scale, None, alpha);
}

/// `text` at `size`, or smaller until it fits in `max` (never under 60%).
#[allow(clippy::too_many_arguments)]
fn fit(o: &mut Overlay, text: &str, x: f32, y: f32, size: f32, max: f32, color: [f32; 4], align: Align, bold: bool) {
    let mut s = size;
    while s > size * 0.6 && o.measure(text, s) > max {
        s *= 0.92;
    }
    o.text(text, x, y, s, color, align, bold);
}

/// Five pips from (x, y) rightward, lit up to `rank`.
fn pips(o: &mut Overlay, rank: u8, x: f32, y: f32, r: f32, alpha: f32) {
    for k in 0..5u8 {
        let lit = k < rank;
        let cx = x + k as f32 * r * 2.7;
        o.circle(cx, y, r * 1.15, [0.0, 0.0, 0.0, 0.5 * alpha]);
        o.circle(cx, y, r, rgba(if lit { PIP } else { PIP_OFF }, alpha));
    }
}

/// `text` broken into lines no wider than `max` at `size`.
fn wrap_lines(o: &Overlay, text: &str, size: f32, max: f32) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split(' ') {
        match lines.last_mut() {
            Some(l) if o.measure(&format!("{l} {word}"), size) <= max => {
                l.push(' ');
                l.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
}

/// The rival met: his card beside him for 2.5 s, fading in and out, a line to his head.
pub(super) fn draw_rival_card(o: &mut Overlay, g: &Game, t: f32) {
    if !matches!(g.screen, Screen::Playing | Screen::Ready) {
        return;
    }
    let Some((r, card, at, age)) = g.rival_card() else { return };
    let (w, h) = (o.width, o.height);
    let (fx, fy) = (at.x as f32, at.y as f32);
    let head = Vec3::new(fx, floor_at(&g.level.grid, fx, fy) + 2.3, fy);
    let Some((sx, sy)) = g.view.project(head, w, h) else { return };
    let age = age as f32;
    let alpha = (age / 0.18).min(1.0) * ((CARD_SECONDS as f32 - age) / 0.4).clamp(0.0, 1.0);
    let c = r.card();
    let cw = (t * 14.0).min(w - t);
    let pad = t * 0.45;
    let bark = wrap_lines(o, &format!("\"{}\"", card.bark), t * 0.48, cw - pad * 2.0);
    let rows = 2.0 + c.lesson.is_some() as u8 as f32 + 3.0 + bark.len() as f32 * 0.85;
    let ch = t * (0.75 + rows * 0.68);
    // Beside him, on the side with room; the slide in from that side.
    let right = sx + t * 1.4 + cw < w - t * 0.3;
    let slide = (1.0 - (age / 0.18).min(1.0)) * t * 0.8;
    let x0 = if right { sx + t * 1.4 + slide } else { sx - t * 1.4 - cw - slide };
    let y0 = (sy - ch * 0.6).clamp(t * 0.3, h - ch - t * 2.0);
    o.alpha = alpha;
    o.line([sx, sy], [if right { x0 } else { x0 + cw }, y0 + t * 0.9], (t * 0.06).max(1.0), rgba(COPPER, 0.8));
    o.circle(sx, sy, t * 0.12, rgba(COPPER, 0.9));
    draw_card(o, &c, &bark, x0, y0, cw, ch, t);
    o.alpha = 1.0;
}

/// The card's face: name, breed and rank, the lesson, the traits ("?" for those not yet found),
/// and what he said.
#[allow(clippy::too_many_arguments)]
fn draw_card(o: &mut Overlay, c: &Card, bark: &[String], x0: f32, y0: f32, cw: f32, ch: f32, t: f32) {
    panel(o, x0, y0, cw, ch, t);
    let pad = t * 0.45;
    let step = t * 0.68;
    let mut y = y0 + t * 0.62;
    fit(o, &c.title.to_uppercase(), x0 + pad, y, t * 0.6, cw - pad * 2.0, rgba(GOLD, 1.0), Align::Left, true);
    y += step;
    o.text(c.breed, x0 + pad, y, t * 0.48, rgba(SAND, 1.0), Align::Left, false);
    let pr = t * 0.16;
    pips(o, c.rank, x0 + cw - pad - pr * 2.7 * 4.0 - pr, y, pr, 1.0);
    o.text("RANK", x0 + cw - pad - pr * 2.7 * 4.0 - pr * 3.0, y, t * 0.38, rgba(TEXT_DIM, 1.0), Align::Right, true);
    if let Some(lesson) = &c.lesson {
        y += step;
        fit(o, lesson, x0 + pad, y, t * 0.52, cw - pad * 2.0, rgba(LESSON, 1.0), Align::Left, true);
    }
    // The traits: what is found, and "?" for the rest.
    let label_w = t * 3.4;
    for (label, value) in &c.traits {
        y += step;
        o.text(label, x0 + pad, y, t * 0.42, rgba(TEXT_DIM, 1.0), Align::Left, true);
        let (text, color) = match value {
            Some(v) => (v.as_str(), TEXT),
            None => ("?", COPPER),
        };
        fit(o, text, x0 + pad + label_w, y, t * 0.48, cw - pad * 2.0 - label_w, rgba(color, 1.0), Align::Left, false);
    }
    // What he said, under a rule.
    y += step * 0.6;
    o.rect(x0 + pad, y, cw - pad * 2.0, (t * 0.04).max(1.0), rgba(COPPER, 0.5));
    for line in bark {
        y += step * 0.85;
        o.text(line, x0 + pad, y, t * 0.48, rgba(TEXT, 1.0), Align::Left, false);
    }
}

/// The board from the pause menu: dimmed game, the board in the middle, how to close it.
pub(super) fn draw_ledger_screen(o: &mut Overlay, g: &Game, t: f32) {
    let (w, h) = (o.width, o.height);
    o.rect(0.0, 0.0, w, h, [INK[0], INK[1], INK[2], 0.94]);
    let bw = (w - t * 2.0).min(t * 54.0);
    let bh = h - t * 3.2;
    let rect = [w / 2.0 - bw / 2.0, t * 0.8, bw, bh];
    draw_ledger_board(o, g.ledger(), rect, t, g.time as f32);
    super::glyphs::prompt_line(o, &g.input, "{Back} or a click to close", w / 2.0, h - t * 1.1, t * 0.5, rgba(SAND, 0.9), Align::Center);
}

/**
 * The Ledger board in `rect` ([x, y, w, h]): its title, then a pinned note for each rival, the
 * living first (highest rank first), then the slain, up to nine. The workshop's War Table draws
 * it in its own frame.
 */
pub fn draw_ledger_board(o: &mut Overlay, ledger: &Ledger, rect: [f32; 4], t: f32, time: f32) {
    let [x0, y0, bw, bh] = rect;
    panel(o, x0, y0, bw, bh, t);
    // Cork under the notes: a warm dark ground.
    let inner = [x0 + t * 0.4, y0 + t * 2.2, bw - t * 0.8, bh - t * 2.6];
    o.rect(inner[0], inner[1], inner[2], inner[3], rgba(0x2a2018, 0.9));
    o.rect_outline(inner[0], inner[1], inner[2], inner[3], (t * 0.05).max(1.0), rgba(COPPER, 0.4));
    o.text("THE LEDGER OF GRUDGES", x0 + bw / 2.0, y0 + t * 0.8, t * 0.9, rgba(GOLD, 1.0), Align::Center, true);
    let living = ledger.live().count();
    let sub = format!(
        "{living} {} remember{} you   {} slain",
        if living == 1 { "rival" } else { "rivals" },
        if living == 1 { "s" } else { "" },
        ledger.slain
    );
    o.text(&sub, x0 + bw / 2.0, y0 + t * 1.6, t * 0.46, rgba(SAND, 0.95), Align::Center, false);
    let mut shown: Vec<&Rival> = ledger.live().collect();
    shown.sort_by_key(|r| (std::cmp::Reverse(r.rank), r.id));
    shown.extend(ledger.rivals.iter().rev().filter(|r| !r.alive));
    shown.truncate(9);
    if shown.is_empty() {
        o.text(
            "No one down there knows your name. Yet.",
            x0 + bw / 2.0,
            inner[1] + inner[3] / 2.0,
            t * 0.6,
            rgba(TEXT_DIM, 1.0),
            Align::Center,
            false,
        );
        return;
    }
    let cols = if shown.len() <= 4 { 2 } else { 3 };
    let rows = shown.len().div_ceil(cols).max(1);
    let gap = t * 0.5;
    let nw = (inner[2] - gap * (cols as f32 + 1.0)) / cols as f32;
    let nh = ((inner[3] - gap * (rows as f32 + 1.0)) / rows as f32).min(nw * 0.62);
    // The notes in the middle of the cork, top to bottom.
    let top = inner[1] + (inner[3] - (nh * rows as f32 + gap * (rows as f32 + 1.0))) / 2.0;
    let shadow = o.shadow;
    for (i, r) in shown.iter().enumerate() {
        let (c, row) = (i % cols, i / cols);
        // Pinned a little askew: each note sits a hair off the grid by its id.
        let jitter = ((r.id.0 as f32 * 2.399).sin()) * t * 0.12;
        let x = inner[0] + gap + c as f32 * (nw + gap) + jitter;
        let y = top + gap + row as f32 * (nh + gap) - jitter * 0.6;
        draw_note(o, r, x, y, nw, nh, t, time);
    }
    o.shadow = shadow;
}

/// One rival's note: portrait, name, epithet, breed and rank, the traits as found, the lesson,
/// his last encounter; a slain one crossed through in red, his trophy named.
fn draw_note(o: &mut Overlay, r: &Rival, x: f32, y: f32, nw: f32, nh: f32, t: f32, time: f32) {
    o.shadow = 0.0;
    // The note's shadow, the paper, its edge.
    o.rect(x + t * 0.12, y + t * 0.15, nw, nh, [0.0, 0.0, 0.0, 0.45]);
    let paper = if r.alive { PAPER } else { 0xb8ab90 };
    o.rect(x, y, nw, nh, rgba(paper, 1.0));
    o.rect_outline(x, y, nw, nh, (t * 0.05).max(1.0), rgba(PAPER_EDGE, 1.0));
    // The portrait, in a dark frame on the left.
    let pad = t * 0.35;
    let ps = (nh - pad * 2.0).min(nw * 0.36);
    o.rect(x + pad, y + pad, ps, ps, rgba(0x1c1814, 1.0));
    o.rect_outline(x + pad, y + pad, ps, ps, (t * 0.05).max(1.0), rgba(COPPER, 0.8));
    draw_portrait(o, r.kind, x + pad, y + pad, ps, if r.alive { 1.0 } else { 0.55 });
    // The pin.
    let (px, py) = (x + nw / 2.0, y + t * 0.05);
    o.circle(px + t * 0.05, py + t * 0.08, t * 0.2, [0.0, 0.0, 0.0, 0.4]);
    o.circle(px, py, t * 0.2, rgba(COPPER, 1.0));
    o.circle(px - t * 0.06, py - t * 0.06, t * 0.07, rgba(0xffd0a0, 0.9));
    // The words, right of the portrait.
    let tx = x + pad * 2.0 + ps;
    let tw = x + nw - pad - tx;
    let line = nh / 9.2;
    let size = (line * 0.82).min(t * 0.5);
    let mut ty = y + pad + line * 0.55;
    fit(o, &r.name.to_uppercase(), tx, ty, size * 1.25, tw, rgba(INK_DARK, 1.0), Align::Left, true);
    ty += line * 1.15;
    fit(o, &r.epithet, tx, ty, size, tw, rgba(INK_RED, 1.0), Align::Left, false);
    ty += line;
    let c = r.card();
    fit(o, c.breed, tx, ty, size * 0.92, tw * 0.55, rgba(INK_SOFT, 1.0), Align::Left, false);
    pips(o, r.rank, x + nw - pad - size * 0.32 * 2.7 * 4.0 - size * 0.32, ty, size * 0.32, 1.0);
    ty += line * 1.1;
    for (label, value) in &c.traits {
        let text = match value {
            Some(v) => format!("{}: {v}", title_case(label)),
            None => format!("{}: ?", title_case(label)),
        };
        fit(o, &text, tx, ty, size * 0.9, tw, rgba(if value.is_some() { INK_DARK } else { INK_SOFT }, 1.0), Align::Left, false);
        ty += line * 0.95;
    }
    if let Some(lesson) = &c.lesson {
        fit(o, lesson, tx, ty, size * 0.9, tw, rgba(INK_RED, 1.0), Align::Left, true);
    }
    // His last encounter, along the foot of the note.
    let foot = y + nh - pad * 0.9;
    if let Some(e) = r.last() {
        fit(o, &e.line(), x + pad, foot, size * 0.82, nw - pad * 2.0, rgba(INK_SOFT, 1.0), Align::Left, false);
    }
    if !r.alive {
        // Crossed through, and the trophy taken.
        let a = 0.75 + (time * 2.0).sin() * 0.05;
        o.line([x + pad, y + pad], [x + pad + ps, y + pad + ps], (t * 0.14).max(2.0), rgba(INK_RED, a));
        o.line([x + pad + ps, y + pad], [x + pad, y + pad + ps], (t * 0.14).max(2.0), rgba(INK_RED, a));
        o.text("SLAIN", x + pad + ps / 2.0, y + pad + ps / 2.0, size * 1.3, rgba(0xf0e0c8, 1.0), Align::Center, true);
        fit(o, &format!("Trophy: {}", r.trophy.name()), tx, foot - line, size * 0.82, tw, rgba(INK_DARK, 1.0), Align::Left, false);
    }
}

/// "STRENGTH" as "Strength".
fn title_case(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut c = lower.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
