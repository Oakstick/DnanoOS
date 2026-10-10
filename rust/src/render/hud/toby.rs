//! Toby's trail on the pause screen (B08): a small row of his seven things under the road down,
//! each found one drawn in its colours on a warm ground, the others a dim socket, and how many.

use super::*;
use crate::logic::tale::{Token, TOBY_COLOR, TOKENS};

/// A small picture of `token`, `s` across, centred on (cx, cy).
pub(super) fn icon(o: &mut Overlay, token: Token, cx: f32, cy: f32, s: f32, a: f32) {
    let c = |hex: u32| rgba(hex, a);
    match token {
        Token::Ball => {
            o.circle(cx, cy, s * 0.36, c(0x9a5a2e));
            o.line([cx - s * 0.34, cy + s * 0.08], [cx + s * 0.34, cy - s * 0.08], s * 0.1, c(0x4a2a16));
        }
        Token::Drawing => {
            o.rect(cx - s * 0.4, cy - s * 0.3, s * 0.8, s * 0.6, c(0xf2ead8));
            for (k, hex) in [0x34588e, 0x5e7a34, 0x923a2c, 0x5a3e6e].into_iter().enumerate() {
                let x = cx - s * 0.24 + k as f32 * s * 0.16;
                o.line([x, cy - s * 0.05], [x, cy + s * 0.2], s * 0.05, c(hex));
                o.circle(x, cy - s * 0.1, s * 0.05, c(0xd8a880));
            }
            o.line([cx - s * 0.26, cy + s * 0.04], [cx + s * 0.26, cy + s * 0.04], s * 0.03, c(0x2a2a2a));
            o.circle(cx + s * 0.28, cy - s * 0.2, s * 0.06, c(0xf2c53d));
        }
        Token::Soldier => {
            o.rect(cx - s * 0.1, cy - s * 0.05, s * 0.2, s * 0.2, c(0xc8322a));
            o.rect(cx - s * 0.1, cy + s * 0.15, s * 0.2, s * 0.2, c(0x1e2a4a));
            o.circle(cx, cy - s * 0.12, s * 0.07, c(0xe8b890));
            o.rect(cx - s * 0.08, cy - s * 0.4, s * 0.16, s * 0.22, c(0x141418));
            o.line([cx + s * 0.16, cy - s * 0.3], [cx + s * 0.16, cy + s * 0.2], s * 0.04, c(0x5a3a22));
        }
        Token::Mitten => {
            o.circle(cx, cy - s * 0.12, s * 0.2, c(0xc8322a));
            o.rect(cx - s * 0.2, cy - s * 0.12, s * 0.4, s * 0.3, c(0xc8322a));
            o.circle(cx + s * 0.24, cy, s * 0.09, c(0xc8322a));
            o.rect(cx - s * 0.22, cy + s * 0.18, s * 0.44, s * 0.12, c(0xf0e6d0));
        }
        Token::Chalk => {
            o.line([cx - s * 0.25, cy + s * 0.25], [cx + s * 0.1, cy - s * 0.1], s * 0.14, c(0xf4f2ea));
            o.line([cx + s * 0.2, cy - s * 0.35], [cx + s * 0.2, cy + s * 0.05], s * 0.04, c(0xe8e6de));
            o.line([cx + s * 0.08, cy - s * 0.35], [cx + s * 0.32, cy - s * 0.35], s * 0.04, c(0xe8e6de));
        }
        Token::Bell => {
            o.triangle([cx, cy - s * 0.35], [cx - s * 0.3, cy + s * 0.22], [cx + s * 0.3, cy + s * 0.22], c(0xd8a840));
            o.circle(cx, cy - s * 0.22, s * 0.14, c(0xd8a840));
            o.circle(cx, cy + s * 0.28, s * 0.07, c(0x4a3a20));
        }
        Token::Scarf => {
            for k in 0..5 {
                let x = cx - s * 0.36 + k as f32 * s * 0.18;
                let hex = if k % 2 == 1 { 0xf0e6d0 } else { 0xc8322a };
                o.line(
                    [x, cy + (k as f32 * 1.3).sin() * s * 0.12],
                    [x + s * 0.18, cy + ((k + 1) as f32 * 1.3).sin() * s * 0.12],
                    s * 0.16,
                    c(hex),
                );
            }
        }
    }
}

/// The row on one line about `y` (its middle), centred: TOBY'S TRAIL and how many, then the seven.
pub(super) fn draw_row(o: &mut Overlay, g: &Game, t: f32, y: f32) {
    let held = g.tokens();
    let n = held.count();
    let head = format!("TOBY'S TRAIL  {n}/7");
    let st = style::label(t, 0.46, rgba(TOBY_COLOR, 0.95));
    let hw = o.measure_styled(&head, &st);
    let s = t * 1.05;
    let gap = t * 0.22;
    let icons = s * 7.0 + gap * 6.0;
    let total = hw + t * 0.6 + icons;
    let x0 = o.width / 2.0 - total / 2.0;
    o.text_styled(&head, x0, y, &st);
    let ix = x0 + hw + t * 0.6;
    for (k, token) in TOKENS.into_iter().enumerate() {
        let cx = ix + k as f32 * (s + gap) + s / 2.0;
        let found = held.has(token);
        let back = if found { rgba(0x3a1e16, 0.85) } else { [1.0, 1.0, 1.0, 0.05] };
        glyphs::rounded_rect(o, cx - s / 2.0, y - s / 2.0, s, s, s * 0.22, back);
        if found {
            style::glow(o, cx, y, s * 0.7, rgba(TOBY_COLOR, 0.25));
            icon(o, token, cx, y, s * 0.8, 1.0);
        } else {
            // A socket in the sin's colour, faint: the realm it waits in.
            o.circle(cx, y, s * 0.08, rgba(token.sin().color(), 0.45));
        }
    }
}
