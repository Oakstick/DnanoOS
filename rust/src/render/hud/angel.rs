//! The archangels on screen (UI.md §6): at the bottom left, beside the belt, the archangels with
//! him as four emblems in a row, always in the same order (Michael, Gabriel, Raphael, Uriel), each
//! in his colour (wings round a halo, from the atlas's shapes) with his initial at its foot, so the
//! four are told apart without their colours. A ring fills as each power comes back, the seconds
//! left stand in its middle, and a ready emblem is bright and breathes. The chosen one is larger,
//! ringed in gold, with the angel button's glyph at his shoulder; his name and state stand over
//! the row, and the next button's glyph at its end. Before the first shrine only his patron shows.
//! A full-width banner when a shrine gives (the four, and the two buttons, 2.5 s). And an arrow at
//! the screen's edge toward each shrine still to give within 25 tiles that is out of sight.

use std::f32::consts::{FRAC_PI_2, TAU};

use super::layout::HudLayout;
use super::style::{self, art, size};
use super::*;
use crate::game::BLESSED_SECONDS;
use crate::logic::angels::{ARCHANGELS, RECHARGE};
use crate::logic::items::Offer;
use crate::logic::types::V;

/// What one emblem shows.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Emblem {
    pub angel: Archangel,
    /// How far the power has come back (1: ready to call).
    pub fill: f32,
    pub ready: bool,
    /// The one the angel button calls.
    pub chosen: bool,
    /// Seconds until he is back (0: ready).
    pub wait: f32,
}

/// The emblems of the archangels with him, in their order (none before a shrine, without a patron).
pub fn emblems(g: &Game) -> Vec<Emblem> {
    let patron = g.patron();
    let chosen = g.chosen_angel();
    ARCHANGELS
        .iter()
        .filter(|&&a| g.angels.has(a, patron))
        .map(|&a| Emblem {
            angel: a,
            fill: g.angels.fill(a) as f32,
            ready: g.angels.ready(a, patron),
            chosen: chosen == Some(a),
            wait: g.angels.wait[a.index()] as f32,
        })
        .collect()
}

/// The emblem's art: a dark disc, the recharge ring, the wings in the angel's colour (dimmed while
/// he is away), the halo. `alpha` fades it all.
pub fn draw_emblem_art(o: &mut Overlay, e: &Emblem, cx: f32, cy: f32, r: f32, time: f32, alpha: f32) {
    let color = if e.ready { e.angel.color() } else { mix_rgb(e.angel.color(), 0x60606a, 0.55) };
    let a = alpha * if e.ready { 1.0 } else { 0.85 };
    let pulse = if e.ready { 0.5 + 0.5 * (time * 3.2 + e.angel.index() as f32).sin() } else { 0.0 };
    style::glow(o, cx, cy + r * 0.1, r * 1.55, [0.0, 0.0, 0.0, 0.55 * alpha]);
    if e.ready {
        style::glow(o, cx, cy, r * (1.4 + 0.12 * pulse), rgba(e.angel.color(), (0.3 + 0.22 * pulse) * a));
    }
    style::disc(o, cx, cy, r * 0.98, [INK[0], INK[1], INK[2], 0.92 * alpha]);
    style::glow(o, cx, cy - r * 0.1, r * 0.95, rgba(color, 0.28 * a));
    // The ring: its track, and the power coming back from the top, clockwise.
    let rw = (r * 0.11).max(2.0);
    style::ring(o, cx, cy, r * 0.9, rw, 0.0, TAU, [1.0, 1.0, 1.0, 0.12 * alpha]);
    if e.fill > 0.0 {
        let ring = if e.ready { e.angel.color() } else { mix_rgb(e.angel.color(), 0xffffff, 0.15) };
        style::ring(o, cx, cy, r * 0.9, rw, -FRAC_PI_2, -FRAC_PI_2 + e.fill * TAU, rgba(ring, 0.95 * alpha));
    }
    // The wings, and the light and halo between them.
    let (ww, wh) = (r * 1.7, r * 1.7 * 160.0 / 256.0);
    o.region(art::WINGS, cx - ww / 2.0, cy - wh * 0.58, ww, wh, rgba(mix_rgb(color, 0xffffff, 0.3), a * if e.ready { 1.0 } else { 0.45 }));
    style::glow(o, cx, cy + r * 0.05, r * 0.5, rgba(mix_rgb(color, 0xffffff, 0.5), 0.8 * a));
    style::disc(o, cx, cy + r * 0.08, r * 0.13, rgba(mix_rgb(color, 0xffffff, 0.75), a));
    let hs = r * 0.62;
    o.region(art::HALO, cx - hs / 2.0, cy - r * 0.36 - hs / 2.0, hs, hs * 0.55, rgba(if e.ready { 0xffe07a } else { 0x9a9280 }, a));
}

/// The archangels at the bottom left: the row of emblems, the chosen one's name and state over it.
pub(super) fn draw(o: &mut Overlay, g: &Game) {
    let es = emblems(g);
    if es.is_empty() {
        return;
    }
    let l = HudLayout::of(o.width, o.height);
    let t = l.t;
    let time = g.time as f32;
    let [x, y, w, h] = l.angel;
    // The row: four slots across the corner, each archangel always in his own.
    let slot = w / 4.0;
    let row_y = y + h - t * 1.45;
    let top = y + t * 0.75;
    style::panel_glass(o, [x, y, w, h], t, 0.9);
    for e in &es {
        let i = if es.len() == 1 { 0 } else { e.angel.index() };
        let cx = x + slot * (i as f32 + 0.5);
        let pop = if e.chosen { g.juice.angel_chosen } else { 0.0 };
        let r = t * if e.chosen { 1.12 } else { 0.86 } * (1.0 + 0.12 * pop);
        let cy = row_y - if e.chosen { t * 0.12 } else { 0.0 };
        // A power just come back flares.
        let back = g.juice.angel_back[e.angel.index()];
        if back > 0.0 {
            style::glow(o, cx, cy, r * (1.6 + 0.8 * (1.0 - back)), rgba(e.angel.color(), 0.55 * back));
        }
        draw_emblem_art(o, e, cx, cy, r, time, 1.0);
        if e.chosen {
            style::ring(o, cx, cy, r * 1.1, (t * 0.07).max(1.5), 0.0, TAU, rgba(GOLD, 0.95));
        }
        // His initial at the foot, and the seconds left in the middle while he is away.
        let ink = if e.ready { mix_rgb(e.angel.color(), 0xffffff, 0.45) } else { 0xb8b4ac };
        let is = style::name(t, if e.chosen { 0.5 } else { 0.42 }, rgba(ink, 1.0)).centered();
        o.text_styled(e.angel.initial(), cx, cy + r * 0.62, &is);
        if !e.ready {
            let secs = (e.wait.ceil() as u32).to_string();
            let ss = style::label(t, if e.chosen { 0.62 } else { 0.5 }, rgba(0xffffff, 1.0)).centered();
            o.text_styled(&secs, cx, cy - r * 0.05, &ss);
        }
    }
    // Over the row: the chosen one's name, and ready or the seconds; the angel button's glyph before
    // it, the next button's at the far end (when there is another to choose).
    let Some(c) = es.iter().find(|e| e.chosen) else { return };
    let gs = t * 0.62;
    let mut tx = x + t * 0.35;
    tx += glyphs::draw_prompt(o, &g.input, Action::Angel, tx, top, gs) + t * 0.25;
    let ns = style::name(t, size::NAME * 0.82, rgba(mix_rgb(c.angel.color(), 0xffffff, 0.3), 1.0));
    let name = c.angel.name().to_uppercase();
    o.text_styled(&name, tx, top, &ns);
    tx += o.measure_styled(&name, &ns) + t * 0.3;
    let (state, sc) = if c.ready { ("READY".to_string(), 0x7dff9a) } else { (format!("{}s", c.wait.ceil() as u32), TEXT_DIM) };
    o.text_styled(&state, tx, top, &style::label(t, size::MARK, rgba(sc, 1.0)));
    if es.len() > 1 {
        glyphs::draw_prompt_right(o, &g.input, Action::AngelNext, x + w - t * 0.3, top, gs);
    }
}

/// A shrine's banner across the screen while it lasts: the four archangels, and their two buttons.
pub(super) fn draw_banner(o: &mut Overlay, g: &Game, t: f32) {
    let Some((first, left)) = g.juice.blessed else { return };
    if !matches!(g.screen, Screen::Playing | Screen::Ready) {
        return;
    }
    let (w, h) = (o.width, o.height);
    let age = BLESSED_SECONDS - left;
    let (fade, slide) = style::appear(age, left);
    if fade <= 0.0 {
        return;
    }
    let y = h * 0.3 + slide * t * 0.6;
    let bh = t * 4.6;
    // A band of glass across the screen, fading at its edges, a rule of heaven's light.
    let light = 0xfff1c0;
    style::band(o, y, bh, t, light, fade);
    style::glow(o, w / 2.0, y, bh * 1.2, rgba(light, 0.16 * fade));
    let title = if first { "THE FOUR ARCHANGELS ARE WITH YOU" } else { "THE ARCHANGELS ARE READY" };
    let ts = style::name(t, 1.15, rgba(0xfff6dc, fade)).centered().glow(t * 0.35, rgba(light, 0.4 * fade));
    o.text_styled(title, w / 2.0, y - t * 1.35, &ts);
    // The four, in their order and colours, each named.
    let r = t * 0.62;
    let gap = t * 3.4;
    for (i, &a) in ARCHANGELS.iter().enumerate() {
        let cx = w / 2.0 + (i as f32 - 1.5) * gap;
        let e = Emblem { angel: a, fill: 1.0, ready: true, chosen: false, wait: 0.0 };
        draw_emblem_art(o, &e, cx - t * 0.75, y + t * 0.05, r, g.time as f32, fade);
        let ns = style::name(t, 0.5, rgba(mix_rgb(a.color(), 0xffffff, 0.35), fade));
        o.text_styled(&a.name().to_uppercase(), cx - t * 0.05, y + t * 0.05, &ns);
    }
    let sub = style::label(t, size::LABEL * 0.9, rgba(SAND, fade)).centered();
    let line = format!("{{Angel}} CALLS THE CHOSEN   {{AngelNext}} CHOOSES   EACH IS BACK {} s AFTER HIS CALL", RECHARGE as u32);
    glyphs::prompt_text(o, &g.input, &line, w / 2.0, y + t * 1.45, &sub);
}

/// Shrines out of sight within this many tiles get an arrow at the screen's edge.
pub const ARROW_REACH: f64 = 25.0;

/// The shrines still to give within reach of him: where each stands and whose statue it shows now.
pub fn shrines_near(g: &Game) -> Vec<(V, Archangel)> {
    let hp = g.hero.pos();
    g.diamonds
        .iter()
        .filter(|d| d.active)
        .filter_map(|d| match d.current() {
            Offer::Angel(a) => Some((d.tile.v(), a)),
            _ => None,
        })
        .filter(|(p, _)| ((p.x - hp.x).powi(2) + (p.y - hp.y).powi(2)).sqrt() <= ARROW_REACH)
        .collect()
}

/// An arrow at the edge for each shrine still to give within reach and out of sight, in its angel's
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
        let e = Emblem { angel, fill: 1.0, ready: true, chosen: false, wait: 0.0 };
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

    #[test]
    fn the_row_shows_the_archangels_with_him_and_their_power_coming_back() {
        let mut g = Game::new();
        assert!(emblems(&g).is_empty(), "none before a shrine, without a patron");
        // A patron alone: his emblem, chosen.
        g.progress.hero_mut(g.hero_kind).patron = Some(Archangel::Raphael);
        let es = emblems(&g);
        assert_eq!(es.len(), 1);
        assert!(es[0].angel == Archangel::Raphael && es[0].chosen && es[0].ready);
        // All four, in their order; one away, its ring part filled and its seconds counted.
        g.angels.granted = true;
        g.angels.chosen = Archangel::Gabriel;
        g.angels.wait[Archangel::Raphael.index()] = RECHARGE * 0.25;
        let es = emblems(&g);
        assert_eq!(es.iter().map(|e| e.angel).collect::<Vec<_>>(), ARCHANGELS.to_vec());
        let r = es[Archangel::Raphael.index()];
        assert!(!r.ready && (r.fill - 0.75).abs() < 1e-4 && (r.wait - 6.25).abs() < 1e-4, "{r:?}");
        assert_eq!(es.iter().filter(|e| e.chosen).map(|e| e.angel).collect::<Vec<_>>(), vec![Archangel::Gabriel]);
    }

    #[test]
    fn a_shrine_raises_its_banner_and_it_runs_out() {
        let mut g = Game::new();
        g.juice.blessed = Some((true, BLESSED_SECONDS));
        g.debug_tick_juice(0.1);
        assert!(g.juice.blessed.is_some_and(|b| b.0));
        g.debug_tick_juice(BLESSED_SECONDS as f64);
        assert_eq!(g.juice.blessed, None);
    }
}
