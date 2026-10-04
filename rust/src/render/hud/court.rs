//! A prince's court on the HUD (ROADMAP.md phase 3): his health across the top middle in the
//! panels' charcoal and copper, his name and the title of his phase over it, notches where his
//! fight changes (the thirds, or Krampus's own) and a pip lit for each phase reached, the pale
//! trail of what the last blows cost, and a gold edge while his hide is open
//! (`game::court::PrinceBar`); his health is in his sin's colour (Krampus's red). Under the score,
//! what the fight wants of him: the fight, the key, the gate (in the lair, the fight and the friend).

use super::*;

/// The bar's colours: a flash as a blow lands, its edge bright while his hide is open.
const HEALTH_HIT: u32 = 0xfff0c0;
const OPEN: u32 = 0xffe070;
const TRAIL: u32 = 0xe8dcc0;

/// The prince's bar: his name and phase, his health, the thirds, his hide.
pub(super) fn draw_prince_bar(o: &mut Overlay, g: &Game, t: f32) {
    let Some(b) = g.prince_bar() else { return };
    let w = o.width;
    let margin = t * 0.4;
    let bw = (w * 0.36).min(t * 20.0);
    let (x0, y0) = (w / 2.0 - bw / 2.0, margin);
    let ph = t * 2.25;
    // The panel, as the HUD's others: charcoal ink, a faint edge, a copper rule along its top.
    o.rect(x0 - t * 0.5, y0, bw + t, ph, [INK[0], INK[1], INK[2], 0.84]);
    o.rect_outline(x0 - t * 0.5, y0, bw + t, ph, (t * 0.06).max(1.0), [EDGE[0], EDGE[1], EDGE[2], 0.26]);
    o.rect(x0 - t * 0.2, y0, bw + t * 0.4, (t * 0.06).max(1.0), rgba(COPPER, 0.75));
    let time = g.time as f32;
    let ty = y0 + t * 0.6;
    o.text(&b.name, x0, ty, t * 0.66, rgba(SAND, 1.0), Align::Left, true);
    let phase_color = match b.phase {
        1 => COPPER,
        2 => 0xd8a040,
        _ => 0xff8a3a,
    };
    o.text(b.title, x0 + bw, ty, t * 0.48, rgba(phase_color, 1.0), Align::Right, true);
    // A pip for each phase, lit as it is reached.
    let ps = t * 0.13;
    let px = x0 + bw - o.measure(b.title, t * 0.48) - t * 0.5;
    for k in 0..3u8 {
        let x = px - (2 - k) as f32 * ps * 2.8;
        let lit = b.phase > k;
        o.circle(x, ty, ps, rgba(if lit { phase_color } else { 0x3a3430 }, if lit { 1.0 } else { 0.9 }));
    }
    // The bar.
    let by = y0 + t * 1.15;
    let bh = t * 0.55;
    let frac = (b.hp / b.max.max(1.0)).clamp(0.0, 1.0);
    let trail = (b.trail / b.max.max(1.0)).clamp(0.0, 1.0);
    o.rect(x0 - 2.0, by - 2.0, bw + 4.0, bh + 4.0, [0.0, 0.0, 0.0, 0.75]);
    o.rect(x0, by, bw, bh, [EDGE[0], EDGE[1], EDGE[2], 0.1]);
    o.rect(x0, by, bw * trail, bh, rgba(TRAIL, 0.7));
    o.rect(x0, by, bw * frac, bh, rgba(if b.flash { HEALTH_HIT } else { b.color }, 1.0));
    o.rect(x0, by, bw * frac, bh * 0.3, [1.0, 1.0, 1.0, 0.16]);
    for k in b.notches {
        o.rect(x0 + bw * k - 1.0, by - t * 0.12, 2.0, bh + t * 0.24, rgba(COPPER, 0.95));
    }
    if b.open {
        // His hide is open (or Amon, who has none, is open): strike now.
        let a = 0.55 + (time * 9.0).sin() * 0.35;
        o.rect_outline(x0 - 3.0, by - 3.0, bw + 6.0, bh + 6.0, (t * 0.08).max(2.0), rgba(OPEN, a));
        let open = if b.hide { "HIS HIDE IS OPEN" } else { "HE IS OPEN" };
        o.text(open, w / 2.0, by + bh + t * 0.5, t * 0.42, rgba(OPEN, a), Align::Center, true);
    }
}

/// Under the score: the fight, then the key, then the gate.
pub(super) fn draw_status(o: &mut Overlay, g: &Game, rc: f32, ty: f32, small: f32) {
    if g.in_lair() {
        // Krampus's lair: no key and no gate, only the fight and the friend in the cage.
        let held = g.lair.as_ref().is_some_and(|l| l.child.state == crate::game::lair::ChildState::Held);
        let (text, color) = if g.prince().is_some() {
            ("THE LAST FIGHT", 0xff8a2a)
        } else if held {
            ("YOUR FRIEND IS FREE", 0xfff4d8)
        } else {
            ("KRAMPUS IS SLAIN", 0xe8c060)
        };
        o.text(text, rc, ty, small, rgba(color, 1.0), Align::Right, true);
        return;
    }
    let key = g.key_name();
    let (text, color) = if g.prince().is_some() {
        ("PRINCE FIGHT".to_string(), 0xff8a2a)
    } else if g.red_key.is_some() {
        (format!("TAKE {key}"), g.key_color())
    } else {
        (format!("{key}: TO THE GATE"), g.key_color())
    };
    o.text(&text, rc, ty, small, rgba(color, 1.0), Align::Right, true);
}
