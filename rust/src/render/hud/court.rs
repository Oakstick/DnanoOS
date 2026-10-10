//! A prince's court on the HUD (ROADMAP.md phase 3): his health across the top middle in the
//! panels' charcoal and copper, his name and the title of his phase over it, notches where his
//! fight changes (the thirds, or Krampus's own) and a pip lit for each phase reached, the pale
//! trail of what the last blows cost, and a gold edge while his hide is open
//! (`game::court::PrinceBar`); his health is in his sin's colour (Krampus's red). In the purse
//! under the minimap, what the fight wants of him: the fight, the key, the gate (in the lair, the
//! fight and the friend).

use super::style;
use super::*;

/// The bar's colours: a flash as a blow lands, its edge bright while his hide is open.
const HEALTH_HIT: u32 = 0xfff0c0;
const OPEN: u32 = 0xffe070;

/// The prince's bar: his name and phase, his health, the thirds, his hide.
pub(super) fn draw_prince_bar(o: &mut Overlay, g: &Game) {
    let Some(b) = g.prince_bar() else { return };
    let w = o.width;
    let l = super::layout::HudLayout::of(w, o.height);
    let t = l.t;
    let [px, y0, pw, _] = l.top;
    let ph = t * 2.5;
    let pad = t * 0.6;
    let (x0, bw) = (px + pad, pw - pad * 2.0);
    style::panel_glass(o, [px, y0, pw, ph], t, 1.0);
    let time = g.time as f32;
    let ty = y0 + t * 0.72;
    let nw = o.text_styled(&b.name, x0, ty, &style::name(t, 0.7, rgba(SAND, 1.0)));
    // A New Game+ prince (or Krampus) says so beside his name, small in gold: he is the heavier for it.
    if g.ng_plus > 0 {
        let st = style::label(t, style::size::LABEL, rgba(GOLD, 0.95));
        o.text_styled(&format!("NEW GAME+{}", g.ng_plus), x0 + nw + t * 0.6, ty, &st);
    }
    let phase_color = match b.phase {
        1 => COPPER,
        2 => 0xd8a040,
        _ => 0xff8a3a,
    };
    let title = style::label(t, style::size::LABEL, rgba(phase_color, 1.0)).right();
    let tw = o.text_styled(b.title, x0 + bw, ty, &title);
    // A pip for each phase, lit as it is reached.
    let ps = t * 0.14;
    let pxx = x0 + bw - tw - t * 0.5;
    for k in 0..3u8 {
        let x = pxx - (2 - k) as f32 * ps * 2.8;
        let lit = b.phase > k;
        if lit {
            style::glow(o, x, ty, ps * 3.0, rgba(phase_color, 0.4));
        }
        style::diamond(o, x, ty, ps * 1.2, rgba(if lit { phase_color } else { 0x3a3430 }, if lit { 1.0 } else { 0.9 }));
    }
    // The bar: the trail of the last blows, the notches where his fight changes.
    let r = [x0, y0 + t * 1.35, bw, t * 0.6];
    let frac = (b.hp / b.max.max(1.0)).clamp(0.0, 1.0);
    let trail = (b.trail / b.max.max(1.0)).clamp(0.0, 1.0);
    style::bar(o, r, frac, trail, if b.flash { HEALTH_HIT } else { b.color }, 1.0);
    style::notches(o, r, &b.notches, t, rgba(COPPER, 0.95));
    if b.open {
        // His hide is open (or Amon, who has none, is open): strike now.
        let a = 0.55 + (time * 9.0).sin() * 0.35;
        o.rect_outline(r[0] - 3.0, r[1] - 3.0, r[2] + 6.0, r[3] + 6.0, (t * 0.08).max(2.0), rgba(OPEN, a));
        let open = if b.hide { "HIS HIDE IS OPEN" } else { "HE IS OPEN" };
        o.text_styled(open, w / 2.0, y0 + ph + t * 0.55, &style::label(t, style::size::LABEL, rgba(OPEN, a)).centered());
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
        o.text_styled(
            text,
            rc,
            ty,
            &super::style::label(small / super::style::size::LABEL, super::style::size::LABEL, rgba(color, 1.0)).right(),
        );
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
    o.text_styled(
        &text,
        rc,
        ty,
        &super::style::label(small / super::style::size::LABEL, super::style::size::LABEL, rgba(color, 1.0)).right(),
    );
}
