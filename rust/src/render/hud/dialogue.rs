//! The old man's dialogue (UI.md §6): at the bottom centre over the loadout, a glass panel ruled in
//! his amber, his live portrait 7 units square on a warm ground (the renderer's, made every frame
//! while his words type out), his name plate (THE OLD MAN, and Quitmer his dog when the line has
//! him), his words in the body sans at a comfortable size and measure, typed out as they come, a
//! glyph to skip and one to open his guide.
//!
//! The prince's whisper keeps a frame of its own, so the two can never be taken for each other:
//! no portrait (smoke drifting where a face would be), the frame, the name and the words in the
//! sin's colour (`counsel::whisper_color`, never amber or gold).

use super::layout::HudLayout;
use super::style::{self, art};
use super::*;
use crate::game::counsel::{whisper_color, Speaker, Spoken, AMBER, HOLD, TYPE_RATE};
use crate::render::portrait::{PortraitId, Pose};

/// The old man's gold, for his name plate (his words are amber).
const PLATE: u32 = 0xf2c46a;

/// How a line is framed: its colour, whether a face is shown, the name over it.
#[derive(Clone, PartialEq, Debug)]
pub struct Look {
    /// The rule, the name and the words: the old man's amber, or the whisper's sin colour.
    pub accent: u32,
    /// The words' own colour (the accent, lifted to read).
    pub words: u32,
    /// His live portrait (the old man), or smoke and no face (a whisper).
    pub portrait: bool,
    pub name: String,
    /// "& QUITMER" beside his name, when the line speaks of the dog.
    pub quitmer: bool,
}

/// The frame for `s`.
pub fn look(s: &Spoken) -> Look {
    match s.who {
        Speaker::OldMan => Look {
            accent: AMBER,
            words: mix_rgb(AMBER, 0xfff4e0, 0.35),
            portrait: true,
            name: "THE OLD MAN".to_string(),
            quitmer: s.text.to_lowercase().contains("quitmer"),
        },
        Speaker::Whisper(sin) => {
            let c = whisper_color(sin);
            Look { accent: c, words: c, portrait: false, name: format!("{} WHISPERS", sin.prince().to_uppercase()), quitmer: false }
        }
    }
}

/// The words' size and the most letters a line holds (a comfortable measure).
const WORDS: f32 = 0.72;
const MEASURE: usize = 60;
/// Beside his name when the line speaks of the dog.
const QUITMER: &str = "& QUITMER, HIS DOG";

/// His face's size in units (UI.md §6), within the panel's height.
pub const FACE: f32 = layout::DIALOGUE_H - 1.0;

/// The panel's rectangle, `width` wide for `lines` lines of words: centred in the layout's room,
/// its foot on the room's (the loadout's top).
fn panel_rect(l: &HudLayout, portrait: bool, width: f32, lines: usize, words_line: f32) -> [f32; 4] {
    let t = l.t;
    let [x, y, w, h] = l.dialogue;
    let text_h = t * 1.9 + lines as f32 * words_line + t * 1.05;
    let ph = if portrait { h } else { text_h.clamp(t * 3.8, h) };
    let pw = width.min(w);
    [x + (w - pw) / 2.0, y + h - ph, pw, ph]
}

/// The line in the panel, typed out so far.
pub(super) fn draw(o: &mut Overlay, g: &Game) {
    let Some(s) = g.voices_line() else { return };
    let l = HudLayout::of(o.width, o.height);
    let t = l.t;
    let look = look(s);
    let time = g.time as f32;
    let age = (g.time - s.since) as f32;
    let n = s.text.chars().count();
    let left = (s.since + n as f64 / TYPE_RATE + HOLD - g.time) as f32;
    let (fade, slide) = style::appear(age, left);
    if fade <= 0.0 {
        return;
    }
    // The words wrapped as a whole (so a line never jumps as letters come), at a measure of about
    // sixty letters, smaller only if they would overflow the panel.
    let face = if look.portrait { t * FACE } else { 0.0 };
    let pad = t * 0.5;
    let text_x0 = if look.portrait { pad + face + t * 0.65 } else { t * 3.4 };
    let room = l.dialogue[2] - text_x0 - pad - t * 0.2;
    let mut ws = style::body(t, WORDS, rgba(look.words, fade));
    let per_letter = o.measure_styled("the old man walks with his dog, quitmer", &ws) / 39.0;
    let measure = (per_letter * MEASURE as f32).min(room);
    let words = glyphs::words(&g.input, &s.text);
    let mut lines = o.wrap_lines(&words, measure, &ws);
    let max_lines = ((l.dialogue[3] - t * 2.95) / (t * WORDS * 1.42)).floor().max(1.0) as usize;
    while lines.len() > max_lines && ws.size > t * 0.56 {
        ws = ws.size(ws.size * 0.94);
        lines = o.wrap_lines(&words, room, &ws);
    }
    let line_h = ws.size * 1.42;
    // As wide as the words and the name need (never narrower than the prompts at its foot).
    let ps = style::label(t, 0.56, rgba(TEXT_DIM, fade)).right();
    let device = g.input.last_device();
    let skip = g.input.bindings.shown(Action::Confirm, device).is_some();
    let prompt = match (look.portrait, skip) {
        (true, true) => "{Confirm} skip     {Guide} his guide",
        (true, false) => "{Guide} his guide",
        (false, true) => "{Confirm} skip",
        (false, false) => "",
    };
    let name_st = style::name(t, 0.72, rgba(if look.portrait { PLATE } else { look.accent }, fade));
    let q_st = style::label(t, 0.56, rgba(TEXT_DIM, 0.9 * fade));
    let name_w = o.measure_styled(&look.name, &name_st) + if look.quitmer { t * 0.4 + o.measure_styled(QUITMER, &q_st) } else { 0.0 };
    let words_w = lines.iter().map(|l| o.measure_styled(l, &ws)).fold(0.0, f32::max);
    let prompt_w = glyphs::measure_prompt_text(o, &g.input, prompt, &ps);
    let width = text_x0 + words_w.max(name_w).max(prompt_w).max(t * 12.0) + pad * 1.6;
    let [x, y, pw, ph] = panel_rect(&l, look.portrait, width, lines.len(), line_h);
    let y = y + slide * t * 0.8;
    o.alpha = fade;
    if look.portrait {
        style::panel_ruled(o, [x, y, pw, ph], t, 1.0, AMBER);
    } else {
        // The whisper: darker glass, its rule in the sin's colour.
        style::shadow(o, [x, y, pw, ph], t, 1.0);
        style::glass(o, [x, y, pw, ph], t, 1.0);
        o.rect(x + t * 0.2, y + t * 0.2, pw - t * 0.4, ph - t * 0.4, [0.0, 0.0, 0.0, 0.25]);
        style::frame(o, [x, y, pw, ph], t, rgba(look.accent, 0.85));
    }
    o.alpha = 1.0;

    if look.portrait {
        // His live portrait on a warm ground, framed in gold: every frame while his words type out
        // (his head and his hand move with them), once a second while they stand.
        let (fx, fy) = (x + pad, y + (ph - face) / 2.0);
        o.rect(fx, fy, face, face, [0.03, 0.02, 0.01, 0.8 * fade]);
        style::glow(o, fx + face * 0.42, fy + face * 0.38, face * 0.75, rgba(0xd08a3a, 0.42 * fade));
        style::glow(o, fx + face * 0.5, fy + face * 0.95, face * 0.6, rgba(0x3a2410, 0.5 * fade));
        let talking = s.shown(g.time) < n;
        let pose = if talking { Pose::talking(time, age) } else { Pose::live(time, 1.0) };
        o.portrait(PortraitId::OldMan, pose, fx, fy, face, face, false, [1.0, 1.0, 1.0, fade]);
        // A soft vignette inside the frame, so the face sits in it.
        for (k, a) in [(0.0f32, 0.35f32), (1.0, 0.18)] {
            let inset = k * t * 0.12;
            o.rect_outline(fx + inset, fy + inset, face - inset * 2.0, face - inset * 2.0, t * 0.12, [0.0, 0.0, 0.0, a * fade]);
        }
        o.rect_outline(fx, fy, face, face, (t * 0.06).max(1.0), rgba(PLATE, 0.9 * fade));
        for (cx, cy) in [(fx, fy), (fx + face, fy), (fx, fy + face), (fx + face, fy + face)] {
            style::diamond(o, cx, cy, t * 0.12, rgba(PLATE, fade));
        }
    } else {
        // No face: smoke in the sin's colour, drifting where one would be.
        let (cx, cy) = (x + t * 1.7, y + ph / 2.0);
        style::glow(o, cx, cy, t * 1.9, rgba(look.accent, 0.22 * fade));
        for k in 0..4 {
            let k = k as f32;
            let drift = (time * (0.35 + k * 0.17) + k * 2.1).sin() * t * 0.45;
            let r = t * (1.25 + 0.3 * (time * 0.6 + k).sin());
            let c = rgba(mix_rgb(look.accent, 0xffffff, 0.15), (0.75 - k * 0.12) * fade);
            let at = (cx + drift, cy + (k - 1.5) * t * 0.3 - (time * 0.4 + k * 0.7).fract() * t * 0.3);
            o.region_rotated(art::SMOKE, at.0, at.1, r * 2.0, r * 2.0, time * (0.2 + k * 0.1) + k, c);
        }
    }

    // The name plate.
    let tx = x + text_x0;
    let ny = y + t * 0.82;
    let plate = if look.portrait { PLATE } else { look.accent };
    let nw = o.text_styled(&look.name, tx, ny, &name_st.color(rgba(plate, fade)));
    if look.quitmer {
        o.text_styled(QUITMER, tx + nw + t * 0.4, ny, &q_st);
    }
    style::divider(o, tx, ny + t * 0.58, (pw - text_x0 - pad).min(measure), t, rgba(look.accent, 0.55 * fade));

    // The words, typed out so far, and a caret while they come. (The words may name a control by
    // its token: the share typed is kept.)
    let typed = s.shown(g.time);
    let total = words.chars().count();
    let mut shown = if typed >= n { total } else { typed * total / n.max(1) };
    let mut wy = y + t * 1.95;
    for line in &lines {
        if shown == 0 {
            break;
        }
        let take = line.chars().count().min(shown);
        let part: String = line.chars().take(take).collect();
        let w = o.text_styled(&part, tx, wy, &ws);
        shown = shown.saturating_sub(take + 1);
        if take < line.chars().count() || (shown == 0 && typed < n) {
            let blink = ((time * 3.0).fract() < 0.6) as u8 as f32;
            o.rect(tx + w + t * 0.08, wy - ws.size * 0.36, (t * 0.06).max(1.5), ws.size * 0.72, rgba(look.words, 0.8 * fade * blink));
            break;
        }
        wy += line_h;
    }

    // Skip, and his guide, at the panel's foot.
    glyphs::prompt_text(o, &g.input, prompt, x + pw - pad, y + ph - t * 0.55, &ps);
}

/// The text style the dialogue's words are set in (for the tests).
#[cfg(test)]
fn words_style(t: f32) -> Style {
    style::body(t, WORDS, rgba(AMBER, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::sins::SINS;

    fn spoken(who: Speaker, text: &str) -> Spoken {
        Spoken::new(who, text.to_string(), 0.0)
    }

    fn rgb(c: u32) -> [f32; 3] {
        [((c >> 16) & 255) as f32, ((c >> 8) & 255) as f32, (c & 255) as f32]
    }

    fn far(a: u32, b: u32) -> f32 {
        let (a, b) = (rgb(a), rgb(b));
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    }

    #[test]
    fn the_old_man_never_looks_like_a_whisper_nor_a_whisper_like_him() {
        let him = look(&spoken(Speaker::OldMan, "Up, Ragnar. Quitmer has your scent."));
        assert!(him.portrait && him.accent == AMBER && him.name == "THE OLD MAN" && him.quitmer);
        assert!(!look(&spoken(Speaker::OldMan, "Strike its back.")).quitmer, "the dog only when the line has him");
        for sin in SINS {
            let w = look(&spoken(Speaker::Whisper(sin), "The way is through the fire."));
            assert!(!w.portrait, "{sin:?}: a whisper shows no face");
            assert!(!w.name.contains("OLD MAN") && w.name.contains(&sin.prince().to_uppercase()), "{sin:?}: {}", w.name);
            for c in [w.accent, w.words] {
                // (As `counsel::whisper_color` keeps it from his amber; his plate's gold and the HUD's too.)
                assert!(far(c, AMBER) > 90.0 && far(c, PLATE) > 80.0 && far(c, GOLD) > 80.0, "{sin:?}: {c:06x} near the old man's colours");
            }
            assert_ne!(w.accent, him.accent);
        }
    }

    #[test]
    fn the_words_are_set_at_a_size_that_reads() {
        // At 1080p the words are at least 18 px, the face 196 px.
        let l = HudLayout::at(1920.0, 1080.0, 28.0);
        assert!(words_style(l.t).size >= 18.0);
        assert_eq!(l.t * FACE, 196.0);
    }
}
