//! The old man's guide on screen (B): over the paused game, a charcoal page with his face in its
//! amber frame, the place's name, and his words in four parts: the tale, what you must do, what to
//! watch for, and what is so here now.

use super::*;

/// The page's parts: (heading, its lines, the heading's colour).
type Part<'a> = (&'a str, &'a [String], u32);

pub(super) fn draw_guide(o: &mut Overlay, g: &Game, t: f32) {
    let (w, h) = (o.width, o.height);
    let (guide, now) = g.guide_now();
    o.rect(0.0, 0.0, w, h, [INK[0], INK[1], INK[2], 0.82]);
    let pw = (t * 40.0).min(w - t * 2.0);
    let ph = (h - t * 3.0).min(t * 33.0);
    let (px, py) = ((w - pw) / 2.0, (h - ph) / 2.0);
    o.rect(px, py, pw, ph, [INK[0], INK[1], INK[2], 0.96]);
    o.rect_outline(px, py, pw, ph, (t * 0.07).max(1.5), rgba(COPPER, 0.8));
    o.rect(px + t * 0.4, py, pw - t * 0.8, (t * 0.08).max(1.5), rgba(0xffa83a, 0.9));

    // His face, and who is speaking.
    let fs = t * 3.6;
    let (fx, fy) = (px + t * 0.6, py + t * 0.6);
    o.rect(fx, fy, fs, fs, [0.0, 0.0, 0.0, 0.6]);
    let f = voices::face();
    let scale = (fs * 0.94) / f.width.max(f.height) as f32;
    let (sw, sh) = (f.width as f32 * scale, f.height as f32 * scale);
    draw_sprite(o, f, fx + (fs - sw) / 2.0, fy + (fs - sh) / 2.0, scale, None, 1.0);
    o.rect_outline(fx, fy, fs, fs, (t * 0.08).max(1.5), rgba(0xffa83a, 0.95));
    let tx = fx + fs + t * 0.6;
    o.text("THE OLD MAN SPEAKS", tx, fy + t * 0.45, t * 0.5, rgba(0xffa83a, 1.0), Align::Left, true);
    o.text(&guide.title, tx, fy + t * 1.45, t * 0.95, rgba(GOLD, 1.0), Align::Left, true);
    o.text("Quitmer lies at his feet while he talks.", tx, fy + t * 2.45, t * 0.42, rgba(TEXT_DIM, 1.0), Align::Left, false);

    // His words, part by part, down the page.
    let parts: [Part; 4] = [
        ("THE TALE", &guide.tale, 0xffa83a),
        ("WHAT YOU MUST DO", &guide.task, GOLD),
        ("WHAT TO WATCH FOR", &guide.watch, 0x9ad1ff),
        ("HERE, NOW", &now, 0x7dff9a),
    ];
    let x = px + t * 0.9;
    let width = pw - t * 1.8;
    let size = (t * 0.48).max(11.0);
    let line = size * 1.32;
    let mut y = fy + fs + t * 0.9;
    let bottom = py + ph - t * 1.4;
    for (head, lines, color) in parts {
        if lines.is_empty() || y > bottom {
            continue;
        }
        o.text(head, x, y, size * 1.05, rgba(color, 1.0), Align::Left, true);
        o.rect(x, y + size * 0.75, o.measure(head, size * 1.05), (t * 0.04).max(1.0), rgba(color, 0.5));
        y += line * 1.15;
        for l in lines.iter() {
            if y > bottom {
                break;
            }
            o.text("-", x, y, size, rgba(color, 0.8), Align::Left, false);
            y = wrap_left(o, l, x + t * 0.5, y, width - t * 0.5, line, size, rgba(TEXT, 1.0));
            y += line * 0.15;
        }
        y += line * 0.45;
    }
    super::glyphs::prompt_line(
        o,
        &g.input,
        "{Back} BACK TO THE ROAD",
        px + pw / 2.0,
        py + ph - t * 0.6,
        t * 0.42,
        rgba(TEXT_DIM, 1.0),
        Align::Center,
    );
}
