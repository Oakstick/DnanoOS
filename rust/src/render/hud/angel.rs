//! The archangels on screen (UI.md §6): at the bottom left, once he has met one, an emblem 4 units
//! across in the angel's colour (wings round a halo, from the atlas's shapes), the charges as pips
//! round it, a radial fill while the patron's power comes back, a pulse and the control's glyph
//! when it is ready, the angel's name in the display serif; dim when none is held. A full-width
//! banner when a blessing is given (the angel's name and what the power does, 2.5 s). And an arrow
//! at the screen's edge toward each unspent shrine within 25 tiles that is out of sight.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use super::layout::HudLayout;
use super::style::{self, art, size};
use super::*;
use crate::game::BLESSED_SECONDS;
use crate::logic::items::Offer;
use crate::logic::types::V;

/// What the emblem shows.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Emblem {
    pub angel: Archangel,
    /// Uses left of a shrine's blessing, and how many it gave.
    pub charges: u8,
    pub max: u8,
    /// How far the power has come back (1: ready to call).
    pub fill: f32,
    pub ready: bool,
    /// A blessing or a patron's power is his: otherwise the emblem is the last angel met, dim.
    pub held: bool,
}

/// The emblem's state, once he has met an archangel (a shrine's blessing, or his patron).
pub fn emblem(g: &Game) -> Option<Emblem> {
    if let Some(b) = g.blessing {
        let max = b.angel.charges();
        return Some(Emblem { angel: b.angel, charges: b.charges, max, fill: 1.0, ready: b.charges > 0, held: true });
    }
    if let Some(a) = g.progress.hero(g.hero_kind).patron {
        let fill = if g.patron_wait <= 0.0 { 1.0 } else { (1.0 - g.patron_wait / a.recharge()).clamp(0.0, 1.0) as f32 };
        return Some(Emblem { angel: a, charges: 0, max: 0, fill, ready: g.patron_wait <= 0.0, held: true });
    }
    g.juice.angel_met.map(|a| Emblem { angel: a, charges: 0, max: 0, fill: 0.0, ready: false, held: false })
}

/// The emblem's art: a dark disc, the recharge ring, the wings in the angel's colour, the halo,
/// the charges as pips round its foot. `dim` 0..1 takes the colour out.
#[allow(clippy::too_many_arguments)]
pub fn draw_emblem_art(o: &mut Overlay, e: &Emblem, cx: f32, cy: f32, r: f32, time: f32, alpha: f32) {
    let color = if e.held { e.angel.color() } else { mix_rgb(e.angel.color(), 0x7a7a80, 0.6) };
    let a = alpha * if e.held { 1.0 } else { 0.7 };
    let pulse = if e.ready { 0.5 + 0.5 * (time * 3.2).sin() } else { 0.0 };
    style::glow(o, cx, cy + r * 0.1, r * 1.55, [0.0, 0.0, 0.0, 0.55 * alpha]);
    if e.ready {
        style::glow(o, cx, cy, r * (1.45 + 0.12 * pulse), rgba(color, (0.35 + 0.25 * pulse) * a));
    }
    style::disc(o, cx, cy, r * 0.98, [INK[0], INK[1], INK[2], 0.92 * alpha]);
    style::glow(o, cx, cy - r * 0.1, r * 0.95, rgba(color, 0.28 * a));
    // The ring: its track, and the power coming back from the top, clockwise.
    let rw = (r * 0.09).max(2.0);
    style::ring(o, cx, cy, r * 0.9, rw, 0.0, TAU, [1.0, 1.0, 1.0, 0.1 * alpha]);
    if e.fill > 0.0 {
        style::ring(o, cx, cy, r * 0.9, rw, -FRAC_PI_2, -FRAC_PI_2 + e.fill * TAU, rgba(color, 0.95 * a));
    }
    // The wings, and the halo over them.
    let (ww, wh) = (r * 1.7, r * 1.7 * 160.0 / 256.0);
    o.region(art::WINGS, cx - ww / 2.0, cy - wh * 0.58, ww, wh, rgba(mix_rgb(color, 0xffffff, 0.3), a));
    // The light between the wings, and the halo over it.
    style::glow(o, cx, cy + r * 0.05, r * 0.5, rgba(mix_rgb(color, 0xffffff, 0.5), 0.8 * a));
    style::disc(o, cx, cy + r * 0.08, r * 0.13, rgba(mix_rgb(color, 0xffffff, 0.75), a));
    let hs = r * 0.62;
    o.region(art::HALO, cx - hs / 2.0, cy - r * 0.36 - hs / 2.0, hs, hs * 0.55, rgba(if e.held { 0xffe07a } else { 0xb0a890 }, a));
    // The charges, round the foot of the ring.
    if e.max > 0 {
        let n = e.max as f32;
        let spread = (0.16 * (n - 1.0)).min(0.9);
        for i in 0..e.max {
            let ang = FRAC_PI_2 + if e.max > 1 { spread * (i as f32 / (n - 1.0) - 0.5) * PI } else { 0.0 };
            let (px, py) = (cx + ang.cos() * r * 1.08, cy + ang.sin() * r * 1.08);
            let lit = i < e.charges;
            let s = r * 0.13;
            style::diamond(o, px, py, s * 1.35, [0.0, 0.0, 0.0, 0.7 * alpha]);
            style::diamond(o, px, py, s, if lit { rgba(mix_rgb(color, 0xffffff, 0.3), a) } else { [1.0, 1.0, 1.0, 0.16 * alpha] });
        }
    }
}

/// The emblem at the bottom left, its name and state beside it, Uriel's rest over it.
pub(super) fn draw(o: &mut Overlay, g: &Game) {
    let l = HudLayout::of(o.width, o.height);
    let t = l.t;
    let time = g.time as f32;
    let [x, y, _, h] = l.angel;
    if g.uriel_rest > 0.0 {
        let left = g.uriel_rest.ceil() as u32;
        let s = style::label(t, size::MARK + 0.06, rgba(Archangel::Uriel.color(), 0.95));
        o.text_styled(&format!("URIEL RESTS {}:{:02}", left / 60, left % 60), x + t * 0.2, y + t * 0.1, &s);
    }
    let Some(e) = emblem(g) else { return };
    let r = t * 2.0;
    let (cx, cy) = (x + r + t * 0.15, y + h - r - t * 0.35);
    let tx = cx + r + t * 0.45;
    let color = if e.held { mix_rgb(e.angel.color(), 0xffffff, 0.2) } else { 0x8a8a90 };
    let (state, sc) = if !e.held {
        ("NONE HELD".to_string(), TEXT_DIM)
    } else if e.max > 0 {
        (format!("{} OF {}", e.charges, e.max), SAND)
    } else if e.ready {
        ("READY".to_string(), 0x7dff9a)
    } else {
        (format!("{}s", g.patron_wait.ceil() as u32), TEXT_DIM)
    };
    let name = e.angel.name().to_uppercase();
    let ns = style::name(t, size::NAME, rgba(color, 1.0));
    let ss = style::label(t, size::LABEL, rgba(sc, 1.0));
    // A glass plate for the name, the emblem over its end.
    let tw = o.measure_styled(&name, &ns).max(o.measure_styled(&state, &ss));
    style::panel_glass(o, [cx, cy - t * 1.1, tx - cx + tw + t * 0.7, t * 2.2], t, if e.held { 0.95 } else { 0.75 });
    draw_emblem_art(o, &e, cx, cy, r, time, 1.0);
    if e.ready {
        // Its control, at the emblem's shoulder.
        glyphs::draw_prompt(o, &g.input, Action::Angel, cx + r * 0.62, cy - r * 0.78, t * 0.7);
    }
    // The name, and what stands: uses left, ready, or seconds to wait.
    o.text_styled(&name, tx, cy - t * 0.4, &ns);
    o.text_styled(&state, tx, cy + t * 0.48, &ss);
}

/// The blessing's banner across the screen while it lasts: the angel's name and what it does.
pub(super) fn draw_banner(o: &mut Overlay, g: &Game, t: f32) {
    let Some((angel, left)) = g.juice.blessed else { return };
    if !matches!(g.screen, Screen::Playing | Screen::Ready) {
        return;
    }
    let (w, h) = (o.width, o.height);
    let age = BLESSED_SECONDS - left;
    let (fade, slide) = style::appear(age, left);
    if fade <= 0.0 {
        return;
    }
    let color = angel.color();
    let y = h * 0.3 + slide * t * 0.6;
    let bh = t * 3.4;
    // A band of glass across the screen, fading at its edges, a rule of the angel's light.
    style::band(o, y, bh, t, color, fade);
    style::glow(o, w / 2.0, y, bh * 1.2, rgba(color, 0.22 * fade));
    // The emblem, the name, what it does.
    let e = Emblem { angel, charges: angel.charges(), max: angel.charges(), fill: 1.0, ready: true, held: true };
    let title = format!("{}'S BLESSING", angel.name().to_uppercase());
    let ts = style::name(t, 1.25, rgba(mix_rgb(color, 0xffffff, 0.3), fade)).centered().glow(t * 0.35, rgba(color, 0.45 * fade));
    let tw = o.measure_styled(&title, &ts);
    let r = t * 1.25;
    draw_emblem_art(o, &e, w / 2.0 - tw / 2.0 - r - t * 0.6, y, r, g.time as f32, fade);
    o.text_styled(&title, w / 2.0, y - t * 0.55, &ts);
    let sub = style::label(t, size::LABEL, rgba(SAND, fade)).centered();
    let line = format!("{}   {{Angel}}  x{}", angel.power().to_uppercase(), angel.charges());
    glyphs::prompt_text(o, &g.input, &line, w / 2.0, y + t * 0.85, &sub);
}

/// Shrines out of sight within this many tiles get an arrow at the screen's edge.
pub const ARROW_REACH: f64 = 25.0;

/// The unspent shrines within reach of him: where each stands and whose it is now.
pub fn shrines_near(g: &Game) -> Vec<(V, Archangel)> {
    let hp = g.hero.pos();
    let spent = g.angels_spent();
    g.diamonds
        .iter()
        .filter(|d| d.active)
        .filter_map(|d| match d.current() {
            Offer::Angel(a) if !spent[a.index()] => Some((d.tile.v(), a)),
            _ => None,
        })
        .filter(|(p, _)| ((p.x - hp.x).powi(2) + (p.y - hp.y).powi(2)).sqrt() <= ARROW_REACH)
        .collect()
}

/// An arrow at the edge for each unspent shrine within reach and out of sight, in its angel's
/// colour with the angel's sign; kept clear of the HUD's corners.
pub(super) fn draw_arrows(o: &mut Overlay, g: &Game) {
    if g.screen != Screen::Playing || !g.hero.alive {
        return;
    }
    let (w, h) = (o.width, o.height);
    let l = HudLayout::of(w, h);
    let t = l.t;
    let time = g.time as f32;
    let hp = g.hero.pos();
    let view = g.view.view_matrix();
    let at = |p: V, up: f32| {
        let (x, z) = (p.x as f32, p.y as f32);
        g.view.project(Vec3::new(x, floor_at(&g.level.grid, x, z) + up, z), w, h)
    };
    // Inside this, a shrine is in sight; the arrows ride its edge.
    let (x0, x1) = (l.safe + t * 0.9, w - l.safe - t * 0.9);
    let (y0, y1) = (l.effects[1] + l.effects[3] + t * 0.9, l.loadout[1] - t * 0.9);
    for (p, angel) in shrines_near(g) {
        if let Some((sx, sy)) = at(p, 1.0) {
            if sx > x0 && sx < x1 && sy > l.safe && sy < h - l.safe {
                continue;
            }
        }
        // Which way on screen: the way to it on the ground, as the camera turns it (its right is
        // the screen's right; ahead of it, and up toward it, is the screen's up). Sound for a
        // shrine behind the camera, where a projection is not.
        let (dx, dy) = (p.x - hp.x, p.y - hp.y);
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        let v = view.transform_vector3(Vec3::new(dx as f32, 0.0, dy as f32));
        let (ux, uy) = (v.x, v.z - v.y);
        let ul = (ux * ux + uy * uy).sqrt();
        if ul < 1e-4 {
            continue;
        }
        let (ux, uy) = (ux / ul, uy / ul);
        let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        let k = ((x1 - x0) / 2.0 / ux.abs().max(1e-3)).min((y1 - y0) / 2.0 / uy.abs().max(1e-3));
        let (ax, ay) = (cx + ux * k, cy + uy * k);
        let color = angel.color();
        let bob = (time * 3.0).sin() * t * 0.08;
        let (ax, ay) = (ax - ux * bob, ay - uy * bob);
        // The chevron, pointing out.
        let s = t * 0.62;
        let tip = [ax + ux * s * 1.25, ay + uy * s * 1.25];
        let side = |k: f32| [ax + ux * s * 0.35 - uy * s * 0.75 * k, ay + uy * s * 0.35 + ux * s * 0.75 * k];
        let (l_, r_) = (side(1.0), side(-1.0));
        o.triangle(tip, l_, [ax + ux * s * 0.6, ay + uy * s * 0.6], rgba(color, 0.95));
        o.triangle(tip, [ax + ux * s * 0.6, ay + uy * s * 0.6], r_, rgba(color, 0.95));
        // The angel's sign: a small emblem behind the chevron, and how far.
        let r = t * 0.62;
        let e = Emblem { angel, charges: 0, max: 0, fill: 1.0, ready: false, held: true };
        let (ex, ey) = (ax - ux * r * 1.1, ay - uy * r * 1.1);
        draw_emblem_art(o, &e, ex, ey, r, time, 0.95);
        let dist = (len.round() as u32).to_string();
        let ds = style::label(t, size::MARK, rgba(mix_rgb(color, 0xffffff, 0.5), 0.95)).centered();
        o.text_styled(&dist, ex - ux * r * 1.45, ey - uy * r * 1.45, &ds);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::angels::{Blessing, ARCHANGELS};

    #[test]
    fn the_emblem_shows_charges_and_the_power_coming_back() {
        let mut g = Game::new();
        assert_eq!(emblem(&g), None, "no angel met, no emblem");
        // A shrine's blessing: its charges, ready.
        g.blessing = Some(Blessing { angel: Archangel::Gabriel, charges: 2 });
        let e = emblem(&g).unwrap();
        assert_eq!((e.angel, e.charges, e.max, e.ready, e.held), (Archangel::Gabriel, 2, 3, true, true));
        // A patron's power coming back: the ring fills.
        g.blessing = None;
        g.progress.hero_mut(g.hero_kind).patron = Some(Archangel::Raphael);
        g.patron_wait = Archangel::Raphael.recharge() * 0.25;
        let e = emblem(&g).unwrap();
        assert!(!e.ready && (e.fill - 0.75).abs() < 1e-4, "{e:?}");
        g.patron_wait = 0.0;
        assert!(emblem(&g).unwrap().ready);
        // Met and none held: dim.
        g.progress.hero_mut(g.hero_kind).patron = None;
        g.juice.angel_met = Some(Archangel::Michael);
        let e = emblem(&g).unwrap();
        assert!(!e.held && !e.ready && e.charges == 0);
        let _ = ARCHANGELS;
    }

    #[test]
    fn a_blessing_given_raises_its_banner_and_the_emblem_stays() {
        let mut g = Game::new();
        g.blessing = Some(Blessing::of(Archangel::Uriel));
        g.debug_tick_juice(0.1);
        assert_eq!(g.juice.blessed.map(|b| b.0), Some(Archangel::Uriel));
        assert_eq!(g.juice.angel_met, Some(Archangel::Uriel));
        // A use is not a new blessing; the banner runs out.
        g.blessing = None;
        g.debug_tick_juice(BLESSED_SECONDS as f64);
        assert_eq!(g.juice.blessed, None);
        assert_eq!(g.juice.angel_met, Some(Archangel::Uriel), "met stays met");
        assert!(emblem(&g).is_some_and(|e| !e.held));
    }
}
