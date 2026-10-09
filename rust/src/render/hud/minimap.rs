//! The minimap (UI.md §7): top right inside the safe area, round, seven units across. The level's
//! image (`render::minimap::MapImage`, in the UI atlas's `MAP_REGION`) seen through a round window
//! centred on the hero, north up or turning with the camera (`vh.minimap_turn`), at one of three
//! zooms (L3). A copper rim with the north mark and the zoom's pips, a soft shadow inside it; the
//! hero an arrow in the middle with the camera's view as a faint wedge; what the map knows drawn
//! on top as small shapes in their colours (`draw_marker`, shared with the full map), and the
//! objectives out of range (the exit, the old man's lamp, unspent shrines, the boss) as arrows on
//! the rim pointing their way.
//!
//! Shown in play, the countdown and the pause (under its dim); hidden on the title, in the story
//! and the cutscenes, the old man's wares, the menus' own screens and behind the loading card.

use std::f32::consts::{FRAC_PI_2, TAU};

use super::*;
use crate::logic::types::V;
use crate::render::minimap::{markers, FoeRank, MapLayer, Marker, MarkerKind};
use crate::render::overlay::{OVertex, FLAG_CIRCLE};

/// Tiles from the minimap's middle to its rim at each zoom: near, mid, far.
pub const ZOOM_TILES: [f32; 3] = [9.0, 15.0, 26.0];
/// The dark under every marker, so it reads on any floor.
const SHADOW: u32 = 0x0a0c0c;

/// The minimap's square on screen: the one play's HUD keeps for it, top right inside the safe
/// area, seven units across (`layout::minimap_rect`).
pub fn rect(w: f32, h: f32) -> [f32; 4] {
    super::layout::minimap_rect(w, h)
}

/// Whether the minimap is up: in play, the countdown, the fall and the pause, never in a composed
/// shot or behind the loading screen.
pub fn shown(g: &Game) -> bool {
    matches!(g.screen, Screen::Playing | Screen::Ready | Screen::Paused | Screen::Dying | Screen::Cleared)
        && g.loading.is_none()
        && !g.view.cinematic()
        && !g.full_map_open()
}

/**
 * How far the minimap turns (radians; screen = R(angle) · world, y down): 0 keeps north up;
 * turning, the camera's heading is up. The heading is the camera's forward along the ground, or
 * for a camera looking straight down, its screen's up.
 */
pub fn turn(g: &Game) -> f32 {
    if !g.map_ui.turn {
        return 0.0;
    }
    let (_, fwd, _, up, _) = g.view.camera_basis();
    let flat = |v: glam::Vec3| glam::Vec2::new(v.x, v.z);
    let (f, u) = (flat(fwd), flat(up).normalize_or_zero());
    let d = if f.length() > 0.25 { f } else { u };
    if d.length_squared() < 1e-8 {
        return 0.0;
    }
    -FRAC_PI_2 - d.y.atan2(d.x)
}

/// The camera's heading along the ground (radians in the grid's plane, y down), for the wedge.
fn camera_heading(g: &Game) -> Option<f32> {
    let (_, fwd, _, up, _) = g.view.camera_basis();
    let (f, u) = (glam::Vec2::new(fwd.x, fwd.z), glam::Vec2::new(up.x, up.z).normalize_or_zero());
    let d = if f.length() > 0.25 { f } else { u };
    (d.length_squared() > 1e-8).then(|| d.y.atan2(d.x))
}

/// A round window on the level: where its middle is (tiles), its radius on screen, the pixels a
/// tile, and the turn.
#[derive(Clone, Copy, Debug)]
pub struct Window {
    pub cx: f32,
    pub cy: f32,
    pub radius: f32,
    pub scale: f32,
    pub centre: (f32, f32),
    pub angle: f32,
}

impl Window {
    /// Where a point of the level (tiles) falls on screen.
    pub fn place(&self, at: V) -> (f32, f32) {
        let (s, c) = self.angle.sin_cos();
        let (dx, dy) = ((at.x as f32 - self.centre.0) * self.scale, (at.y as f32 - self.centre.1) * self.scale);
        (self.cx + dx * c - dy * s, self.cy + dx * s + dy * c)
    }

    /// The point of the level (tiles) under a point of the screen.
    pub fn tile_at(&self, p: [f32; 2]) -> (f32, f32) {
        let (s, c) = self.angle.sin_cos();
        let (dx, dy) = ((p[0] - self.cx) / self.scale, (p[1] - self.cy) / self.scale);
        (self.centre.0 + dx * c + dy * s, self.centre.1 - dx * s + dy * c)
    }
}

/// The minimap's window for the game as it is (its middle on the hero).
pub fn window(g: &Game, w: f32, h: f32) -> Window {
    let lay = super::layout::HudLayout::of(w, h);
    let r = lay.minimap;
    let radius = r[2] / 2.0 - rim(lay.t);
    let hp = g.hero.pos();
    Window {
        cx: r[0] + r[2] / 2.0,
        cy: r[1] + r[3] / 2.0,
        radius,
        scale: radius / ZOOM_TILES[g.map_ui.zoom.min(ZOOM_TILES.len() - 1)],
        centre: (hp.x as f32, hp.y as f32),
        angle: turn(g),
    }
}

/// The copper rim's width.
fn rim(t: f32) -> f32 {
    (t * 0.2).max(3.0)
}

/// The objectives an arrow on the rim points to when they are out of range.
pub fn objective(kind: MarkerKind) -> bool {
    matches!(kind, MarkerKind::Gate { .. } | MarkerKind::Lamp | MarkerKind::Shrine { spent: false, .. } | MarkerKind::Foe(FoeRank::Boss))
}

/**
 * What the minimap draws for `markers`: each known one inside the window, and the objectives out
 * of range as arrows on the rim (`Some(angle)`, the way to them on screen). The gate is left to
 * the lamp where both stand on the same tile (a realm's far portal).
 */
pub fn plan(win: &Window, markers: &[Marker], reach: f32) -> Vec<(usize, (f32, f32), Option<f32>)> {
    let lamp = markers.iter().find(|m| m.kind == MarkerKind::Lamp).map(|m| m.at);
    let mut out = Vec::new();
    for (i, m) in markers.iter().enumerate() {
        if !m.known || matches!(m.kind, MarkerKind::Hero { .. } | MarkerKind::Platform { .. }) {
            continue;
        }
        let (x, y) = win.place(m.at);
        let (dx, dy) = (x - win.cx, y - win.cy);
        let d = (dx * dx + dy * dy).sqrt();
        if d <= reach {
            out.push((i, (x, y), None));
        } else if objective(m.kind) && !(matches!(m.kind, MarkerKind::Gate { .. }) && lamp == Some(m.at)) {
            out.push((i, (x, y), Some(dy.atan2(dx))));
        }
    }
    out
}

/// Keeps the minimap's image up to date (the HUD draws; behind the loading screen, so a new
/// level's image is built there).
pub fn prepare(o: &mut Overlay, g: &Game) {
    o.map.sync(g);
}

pub fn draw(o: &mut Overlay, g: &Game, t: f32) {
    if !shown(g) {
        return;
    }
    o.map.sync(g);
    let Some((iw, ih, ppt)) = o.map.image.as_ref().map(|i| (i.w as f32, i.h as f32, i.px_per_tile as f32)) else { return };
    let win = window(g, o.width, o.height);
    let rw = rim(t);
    let outer = win.radius + rw;
    let (cx, cy) = (win.cx, win.cy);

    // The frame, in the HUD's glass (`style`): a soft shadow, a ring of the dark glass with its
    // copper rule outside and a fainter one in, lit a little from above; the dark ground the level
    // lies on.
    let t = super::layout::HudLayout::of(o.width, o.height).t;
    style::glow(o, cx, cy + t * 0.12, outer + t * 0.7, [0.0, 0.0, 0.0, 0.55]);
    disc(o, cx, cy, outer, [INK[0], INK[1], INK[2], 0.9]);
    gradient_ring(o, cx, cy, outer, outer - rw, [1.0, 1.0, 1.0, 0.1], [1.0, 1.0, 1.0, 0.0], Some(-FRAC_PI_2));
    let rule = (t * 0.07).max(1.5);
    style::ring(o, cx, cy, outer - rule / 2.0, rule, 0.0, TAU, rgba(COPPER, 0.85));
    style::ring(o, cx, cy, win.radius + rule * 0.5, rule * 0.7, 0.0, TAU, rgba(COPPER, 0.45));
    disc(o, cx, cy, win.radius, [INK[0] * 0.8, INK[1] * 0.8, INK[2] * 0.8, 0.94]);

    // The level, through the round window.
    let r = win.radius;
    let square = [[cx - r, cy - r], [cx + r, cy - r], [cx + r, cy + r], [cx - r, cy + r]];
    let to_image = |p: [f32; 2]| {
        let (x, y) = win.tile_at(p);
        [(x + 0.5) * ppt, (y + 0.5) * ppt]
    };
    let local = |p: [f32; 2]| [(p[0] - cx) / r, (p[1] - cy) / r];
    image_polygon(o, (iw, ih), &square, to_image, local, FLAG_CIRCLE, [1.0; 4]);
    // A soft shadow inside the rim.
    gradient_ring(o, cx, cy, r, r - t * 0.6, [0.0, 0.0, 0.0, 0.55], [0.0, 0.0, 0.0, 0.0], None);

    // The camera's view, a faint wedge from him.
    if let Some(h) = camera_heading(g) {
        wedge(o, cx, cy, h + win.angle, 0.62, r * 0.62, rgba(0xfff4dc, 0.16));
    }

    // What the map knows, then the objectives out of range on the rim.
    let ms = markers(g);
    let u = (r / 24.0).max(2.2);
    let plan = plan(&win, &ms, r - u * 1.2);
    for m in ms.iter().filter(|m| m.known) {
        if let MarkerKind::Platform { half } = m.kind {
            platform(o, m.at, half, |v| win.place(v), |a, b| clip_to_circle(a, b, [cx, cy], r - 1.0), (u * 0.35).max(1.5), m.color());
        }
    }
    for &(i, (x, y), arrow) in &plan {
        if arrow.is_none() {
            draw_marker(o, ms[i].kind, ms[i].color(), x, y, u, win.scale, win.angle, g.time as f32);
        }
    }
    if let Some(hero) = ms.iter().rev().find(|m| matches!(m.kind, MarkerKind::Hero { .. })) {
        draw_marker(o, hero.kind, hero.color(), cx, cy, u * 1.25, win.scale, win.angle, g.time as f32);
    }

    // The north mark, on the rim where north is.
    let (ns, nc) = (win.angle - FRAC_PI_2).sin_cos();
    let (nx, ny) = (cx + nc * (outer - rw * 0.5), cy + ns * (outer - rw * 0.5));
    let nr = (t * 0.36).max(7.0);
    style::badge(o, "N", nx, ny, nr, GOLD, 1.0);

    for &(i, _, arrow) in &plan {
        if let Some(a) = arrow {
            let size = (rw * 1.1).max(u * 1.7);
            // (Past the north mark, where it would sit on it.)
            let off = (a - (win.angle - FRAC_PI_2) + std::f32::consts::PI).rem_euclid(TAU) - std::f32::consts::PI;
            let clear = off.abs() * outer > nr + size;
            let at = if clear { outer + size * 0.3 } else { outer - rw * 0.5 + nr + size * 0.75 };
            let lamp = ms[i].kind == MarkerKind::Lamp;
            pointer(o, cx + a.cos() * at, cy + a.sin() * at, a, ms[i].color(), size, g.time as f32, lamp);
        }
    }

    // The zoom: three pips on the rim below, as many lit as the zoom is far; L3's glyph beside
    // them on a controller.
    let zoom = g.map_ui.zoom.min(ZOOM_TILES.len() - 1);
    let py = cy + outer - rw * 0.5;
    let pr = (rw * 0.32).max(1.6);
    for k in 0..ZOOM_TILES.len() {
        let px = cx + (k as f32 - 1.0) * pr * 3.4;
        disc(o, px, py, pr + 1.0, [0.0, 0.0, 0.0, 0.8]);
        disc(o, px, py, pr, if k <= zoom { rgba(GOLD, 1.0) } else { rgba(COPPER, 0.45) });
    }
    if g.input.last_device().is_pad() && g.input.bindings.shown(Action::MinimapZoom, g.input.last_device()).is_some() {
        let size = t * 0.62;
        let (ax, ay) = (cx - outer * 0.86 - size * 0.4, cy + outer * 0.78);
        glyphs::draw_prompt(o, &g.input, Action::MinimapZoom, ax, ay, size);
    }
}

// ------------------------------------------------------------------------- shapes

/// A filled disc, smooth-edged (a quad masked round).
pub(super) fn disc(o: &mut Overlay, x: f32, y: f32, r: f32, color: [f32; 4]) {
    if r <= 0.0 {
        return;
    }
    let uv = Overlay::solid_uv();
    o.textured_quad([[x - r, y - r], [x + r, y - r], [x + r, y + r], [x - r, y + r]], [uv; 4], color, FLAG_CIRCLE);
}

fn vert(p: [f32; 2], color: [f32; 4]) -> OVertex {
    OVertex { pos: p, uv: Overlay::solid_uv(), color, extra: [0.0; 4] }
}

/// A ring from radius `r0` (colour `c0`) to `r1` (colour `c1`), the colours blended across it: a
/// soft shadow, a bevel. With `lit`, only the half facing that angle (fading off its sides).
#[allow(clippy::too_many_arguments)]
fn gradient_ring(o: &mut Overlay, x: f32, y: f32, r0: f32, r1: f32, c0: [f32; 4], c1: [f32; 4], lit: Option<f32>) {
    let n = 48;
    let fade = |a: f32, c: [f32; 4]| match lit {
        Some(l) => {
            let k = ((a - l).cos()).max(0.0);
            [c[0], c[1], c[2], c[3] * k]
        }
        None => c,
    };
    for i in 0..n {
        let (a0, a1) = (i as f32 / n as f32 * TAU, (i + 1) as f32 / n as f32 * TAU);
        let at = |a: f32, r: f32| [x + r * a.cos(), y + r * a.sin()];
        let (p0, p1, q0, q1) = (at(a0, r0), at(a1, r0), at(a0, r1), at(a1, r1));
        let (c00, c01, c10, c11) = (fade(a0, c0), fade(a1, c0), fade(a0, c1), fade(a1, c1));
        o.raw_triangle([vert(p0, c00), vert(p1, c01), vert(q1, c11)]);
        o.raw_triangle([vert(p0, c00), vert(q1, c11), vert(q0, c10)]);
    }
}

/// A wedge from `x, y` toward `angle`, `spread` radians either side, `len` long, fading out.
#[allow(clippy::too_many_arguments)]
fn wedge(o: &mut Overlay, x: f32, y: f32, angle: f32, spread: f32, len: f32, color: [f32; 4]) {
    let n = 8;
    let clear = [color[0], color[1], color[2], 0.0];
    for i in 0..n {
        let a0 = angle - spread + 2.0 * spread * i as f32 / n as f32;
        let a1 = angle - spread + 2.0 * spread * (i + 1) as f32 / n as f32;
        o.raw_triangle([
            vert([x, y], color),
            vert([x + len * a0.cos(), y + len * a0.sin()], clear),
            vert([x + len * a1.cos(), y + len * a1.sin()], clear),
        ]);
    }
}

/// A ring `width` wide round `r`, as a strip (for marks small enough that its facets never show).
fn ring(o: &mut Overlay, x: f32, y: f32, r: f32, width: f32, color: [f32; 4]) {
    let n = 20;
    let (ri, ro) = (r - width / 2.0, r + width / 2.0);
    for i in 0..n {
        let (a0, a1) = (i as f32 / n as f32 * TAU, (i + 1) as f32 / n as f32 * TAU);
        let at = |a: f32, r: f32| [x + r * a.cos(), y + r * a.sin()];
        o.triangle(at(a0, ri), at(a0, ro), at(a1, ro), color);
        o.triangle(at(a0, ri), at(a1, ro), at(a1, ri), color);
    }
}

fn diamond(o: &mut Overlay, x: f32, y: f32, r: f32, color: [f32; 4]) {
    o.triangle([x, y - r], [x + r, y], [x, y + r], color);
    o.triangle([x, y - r], [x, y + r], [x - r, y], color);
}

fn square(o: &mut Overlay, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
    o.rect(x - w / 2.0, y - h / 2.0, w, h, color);
}

/// An arrow at `x, y` pointing toward `angle` (screen radians): the way to an objective out of
/// range, on the minimap's rim or the full map's edge. The lamp's breathes.
#[allow(clippy::too_many_arguments)]
pub(super) fn pointer(o: &mut Overlay, x: f32, y: f32, angle: f32, col: u32, size: f32, time: f32, pulse: bool) {
    let (s, c) = angle.sin_cos();
    let (px, py) = (-s, c);
    let at = |along: f32, across: f32| [x + c * along + px * across, y + s * along + py * across];
    let k = 1.0 + if pulse { 0.12 * (time * 4.0).sin() } else { 0.0 };
    let (a, b) = (size * 0.75 * k, size * 0.62 * k);
    let grow = 1.5;
    o.triangle(at(a + grow, 0.0), at(-a * 0.55 - grow, b + grow), at(-a * 0.55 - grow, -b - grow), rgba(SHADOW, 0.9));
    o.triangle(at(a, 0.0), at(-a * 0.55, b), at(-a * 0.55, -b), rgba(col, 1.0));
}

/// The part of the segment `a`-`b` inside the circle round `c` of radius `r`, if any.
pub(super) fn clip_to_circle(a: [f32; 2], b: [f32; 2], c: [f32; 2], r: f32) -> Option<([f32; 2], [f32; 2])> {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let (fx, fy) = (a[0] - c[0], a[1] - c[1]);
    let qa = dx * dx + dy * dy;
    if qa < 1e-9 {
        return None;
    }
    let qb = 2.0 * (fx * dx + fy * dy);
    let qc = fx * fx + fy * fy - r * r;
    let disc = qb * qb - 4.0 * qa * qc;
    if disc <= 0.0 {
        return None;
    }
    let root = disc.sqrt();
    let (t0, t1) = (((-qb - root) / (2.0 * qa)).max(0.0), ((-qb + root) / (2.0 * qa)).min(1.0));
    (t0 < t1).then(|| ([a[0] + dx * t0, a[1] + dy * t0], [a[0] + dx * t1, a[1] + dy * t1]))
}

/// The part of the segment `a`-`b` inside the rectangle `r` (x, y, w, h), if any.
pub(super) fn clip_to_rect(a: [f32; 2], b: [f32; 2], r: [f32; 4]) -> Option<([f32; 2], [f32; 2])> {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for (p, q) in [(-dx, a[0] - r[0]), (dx, r[0] + r[2] - a[0]), (-dy, a[1] - r[1]), (dy, r[1] + r[3] - a[1])] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
        }
    }
    (t0 < t1).then(|| ([a[0] + dx * t0, a[1] + dy * t0], [a[0] + dx * t1, a[1] + dy * t1]))
}

/// The Pit's platform drawn to the map's scale: its four edges as `place` puts its corners on
/// screen, each cut to what `clip` keeps.
pub(super) fn platform(
    o: &mut Overlay,
    at: V,
    half: f32,
    place: impl Fn(V) -> (f32, f32),
    clip: impl Fn([f32; 2], [f32; 2]) -> Option<([f32; 2], [f32; 2])>,
    width: f32,
    color: u32,
) {
    let h = half as f64;
    let corners =
        [V::new(at.x - h, at.y - h), V::new(at.x + h, at.y - h), V::new(at.x + h, at.y + h), V::new(at.x - h, at.y + h)].map(|c| {
            let p = place(c);
            [p.0, p.1]
        });
    for k in 0..4 {
        if let Some((a, b)) = clip(corners[k], corners[(k + 1) % 4]) {
            o.line(a, b, width, rgba(color, 0.75));
        }
    }
}

/**
 * One marker at `x, y` on screen, `u` pixels its unit (a plain foe's dot is a unit across), for
 * the minimap and the full map alike: the hero an arrow (his heading turned by `angle`), foes red
 * dots with their rank's shape (a leader's gold ring, a rival's gold diamond, a general's crown,
 * a boss's skull), shrines diamonds in their angel's colour with a halo, keys, chests, gates,
 * portals in their sin's colour, the Pit's doors and its platform (drawn to the map's `scale`, in
 * pixels a tile), the old man's lamp in amber, the cage.
 */
#[allow(clippy::too_many_arguments)]
pub fn draw_marker(o: &mut Overlay, kind: MarkerKind, color: u32, x: f32, y: f32, u: f32, scale: f32, angle: f32, time: f32) {
    let col = rgba(color, 1.0);
    let dark = rgba(SHADOW, 0.85);
    match kind {
        MarkerKind::Hero { heading } => {
            let a = heading + angle;
            let (ux, uy) = (a.cos(), a.sin());
            let (px, py) = (-uy, ux);
            let r = u * 2.1;
            let tip = [x + ux * r, y + uy * r];
            let back = [x - ux * r * 0.45, y - uy * r * 0.45];
            let l = [x - ux * r * 0.8 + px * r * 0.72, y - uy * r * 0.8 + py * r * 0.72];
            let rr = [x - ux * r * 0.8 - px * r * 0.72, y - uy * r * 0.8 - py * r * 0.72];
            let grow = |p: [f32; 2], k: f32| [x + (p[0] - x) * k, y + (p[1] - y) * k];
            disc(o, x, y, r * 1.25, rgba(0xfff4dc, 0.14));
            let g = 1.0 + 2.2 / r.max(1.0);
            o.triangle(grow(tip, g), grow(l, g), grow(back, g), rgba(SHADOW, 0.95));
            o.triangle(grow(tip, g), grow(back, g), grow(rr, g), rgba(SHADOW, 0.95));
            o.triangle(tip, back, rr, col);
            o.triangle(tip, l, back, rgba(GOLD, 1.0));
        }
        MarkerKind::Foe(rank) => match rank {
            FoeRank::General => {
                // A crown: a band and three points.
                let (w, h) = (u * 2.6, u * 1.9);
                let (x0, y0) = (x - w / 2.0, y - h / 2.0);
                disc(o, x, y, u * 1.75, dark);
                o.rect(x0, y0 + h * 0.45, w, h * 0.55, col);
                o.triangle([x0, y0], [x0 + w * 0.3, y0 + h * 0.5], [x0, y0 + h * 0.5], col);
                o.triangle([x0 + w * 0.22, y0 + h * 0.5], [x, y0 - h * 0.08], [x0 + w * 0.78, y0 + h * 0.5], col);
                o.triangle([x0 + w, y0], [x0 + w, y0 + h * 0.5], [x0 + w * 0.7, y0 + h * 0.5], col);
            }
            FoeRank::Boss => {
                // A skull: the dome, the jaw, two eyes.
                let r = u * 1.5;
                disc(o, x, y, r * 1.3, dark);
                disc(o, x, y - r * 0.15, r * 0.85, col);
                o.rect(x - r * 0.45, y + r * 0.3, r * 0.9, r * 0.5, col);
                disc(o, x - r * 0.33, y - r * 0.12, r * 0.24, rgba(SHADOW, 1.0));
                disc(o, x + r * 0.33, y - r * 0.12, r * 0.24, rgba(SHADOW, 1.0));
                o.rect(x - r * 0.07, y + r * 0.3, r * 0.14, r * 0.5, rgba(SHADOW, 0.8));
            }
            FoeRank::Rival => {
                diamond(o, x, y, u * 1.75, dark);
                diamond(o, x, y, u * 1.3, col);
                disc(o, x, y, u * 0.42, rgba(0xc8202a, 1.0));
            }
            FoeRank::Elite => {
                disc(o, x, y, u * 1.5, dark);
                ring(o, x, y, u * 1.1, (u * 0.38).max(1.2), col);
                disc(o, x, y, u * 0.62, rgba(0xe8382a, 1.0));
            }
            FoeRank::Plain => {
                disc(o, x, y, u * 0.82, dark);
                disc(o, x, y, u * 0.58, col);
            }
        },
        MarkerKind::Shrine { spent, .. } => {
            let r = u * 1.35;
            if !spent {
                let k = 1.0 + 0.1 * (time * 2.5).sin();
                disc(o, x, y, r * 2.0 * k, rgba(color, 0.18));
                ring(o, x, y, r * 1.5 * k, (u * 0.26).max(1.0), rgba(color, 0.75));
            }
            diamond(o, x, y, r * 1.25, dark);
            diamond(o, x, y, r, col);
            diamond(o, x, y, r * 0.4, [1.0, 1.0, 1.0, if spent { 0.2 } else { 0.85 }]);
        }
        MarkerKind::Key | MarkerKind::RedKey => {
            diamond(o, x, y, u * 1.05, dark);
            diamond(o, x, y, u * 0.75, col);
        }
        MarkerKind::Remedy => {
            let (a, b) = (u * 0.75, u * 0.26);
            square(o, x, y, 2.0 * a + 2.0, 2.0 * b + 2.0, dark);
            square(o, x, y, 2.0 * b + 2.0, 2.0 * a + 2.0, dark);
            square(o, x, y, 2.0 * a, 2.0 * b, col);
            square(o, x, y, 2.0 * b, 2.0 * a, col);
        }
        MarkerKind::Potion | MarkerKind::Loot => {
            disc(o, x, y, u * 0.55, dark);
            disc(o, x, y, u * 0.36, col);
        }
        MarkerKind::ItemShrine => {
            diamond(o, x, y, u * 1.0, dark);
            diamond(o, x, y, u * 0.72, col);
        }
        MarkerKind::Chest => {
            let (w, h) = (u * 1.7, u * 1.15);
            square(o, x, y, w + 3.0, h + 3.0, dark);
            square(o, x, y, w, h, col);
            square(o, x, y, w, h * 0.14, rgba(0x5a3a1a, 1.0));
        }
        MarkerKind::Gate { open } => {
            let (w, h) = (u * 2.4, u * 2.0);
            square(o, x, y, w + 3.0, h + 3.0, dark);
            square(o, x, y, w, h, col);
            if open {
                square(o, x, y, w * 0.55, h * 0.6, rgba(0x0a1a10, 0.85));
            } else {
                square(o, x, y, (u * 0.22).max(1.0), h, rgba(SHADOW, 0.85));
            }
        }
        MarkerKind::Entry => {
            ring(o, x, y, u * 0.95, (u * 0.5).max(2.0), dark);
            ring(o, x, y, u * 0.95, (u * 0.26).max(1.2), col);
        }
        MarkerKind::Portal { conquered, sin } => {
            disc(o, x, y, u * 1.85, dark);
            disc(o, x, y, u * 1.5, col);
            disc(o, x, y, u * 0.9, rgba(if conquered { 0x3a3632 } else { 0x140c0a }, 1.0));
            if conquered {
                ring(o, x, y, u * 1.5, (u * 0.26).max(1.0), rgba(sin.color(), 1.0));
            }
        }
        MarkerKind::HouseDoor { open } => {
            let (w, h) = (u * 2.8, u * 2.5);
            square(o, x, y, w + 3.0, h + 3.0, dark);
            square(o, x, y, w, h, col);
            o.rect(x - w * 0.25, y - h * 0.2, w * 0.5, h * 0.7, rgba(if open { 0x1a0806 } else { 0x2a1a14 }, 1.0));
        }
        MarkerKind::PitDoor { open } => {
            let r = u * 1.0;
            if open {
                disc(o, x, y, r * 2.0, rgba(color, 0.22));
            }
            square(o, x, y, 2.0 * r + 3.0, 2.0 * r + 3.0, dark);
            square(o, x, y, 2.0 * r, 2.0 * r, col);
        }
        MarkerKind::Platform { half } => {
            let r = half * scale;
            let w = (u * 0.35).max(1.5);
            let c = rgba(color, 0.75);
            o.rect(x - r, y - r - w / 2.0, 2.0 * r, w, c);
            o.rect(x - r, y + r - w / 2.0, 2.0 * r, w, c);
            o.rect(x - r - w / 2.0, y - r, w, 2.0 * r, c);
            o.rect(x + r - w / 2.0, y - r, w, 2.0 * r, c);
        }
        MarkerKind::Lamp => {
            // The old man's lamp: an amber diamond's outline round a flame, and a soft glow.
            let s = u * 1.6;
            let k = 1.0 + 0.12 * (time * 4.0).sin();
            disc(o, x, y, s * 1.7 * k, rgba(color, 0.18));
            diamond(o, x, y, s + 2.0, dark);
            diamond(o, x, y, s, col);
            diamond(o, x, y, s * 0.62, rgba(0x2a1606, 1.0));
            disc(o, x, y + s * 0.08, s * 0.3, rgba(0xffe6a0, 1.0));
            o.triangle([x - s * 0.22, y], [x + s * 0.22, y], [x, y - s * 0.45], rgba(0xffe6a0, 1.0));
        }
        MarkerKind::Instrument { order, broken } => {
            let r = u * 1.0;
            disc(o, x, y, r * 1.3, dark);
            disc(o, x, y, r, col);
            disc(o, x, y, r * 0.6, rgba(0x14100c, 1.0));
            for k in 0..=order {
                disc(o, x - r * 0.4 + k as f32 * r * 0.4, y, r * 0.15, col);
            }
            if broken {
                o.line([x - r, y - r], [x + r, y + r], (u * 0.25).max(1.0), rgba(0xc8382a, 1.0));
            }
        }
        MarkerKind::Post => {
            let (w, h) = (u * 1.8, u * 2.4);
            square(o, x, y, w + 2.0, h + 2.0, dark);
            o.rect(x - w / 2.0, y - h / 2.0, w, h * 0.7, col);
            o.triangle([x - w / 2.0, y + h * 0.2], [x, y + h * 0.2], [x - w / 2.0, y + h / 2.0], col);
            o.triangle([x, y + h * 0.2], [x + w / 2.0, y + h * 0.2], [x + w / 2.0, y + h / 2.0], col);
        }
        MarkerKind::Cage { open } => {
            let (w, h) = (u * 4.0, u * 4.0);
            square(o, x, y, w, h, rgba(SHADOW, 0.55));
            for k in 0..5 {
                let bx = x - w / 2.0 + w * k as f32 / 4.0;
                let a = if open && k == 2 { 0.2 } else { 1.0 };
                o.rect(bx - u * 0.13, y - h / 2.0, (u * 0.26).max(1.0), h, rgba(color, a));
            }
        }
    }
}

/**
 * The part of a convex polygon on screen (`poly`) that falls on the map's image, drawn from the
 * atlas: `to_image` gives each screen point's place on the image (its pixels, affine), `local` its
 * place in the circle's square for the round mask (with `FLAG_CIRCLE`). Clipped to the image's
 * texel centres, so nothing past its edge is read (the atlas holds other things there).
 */
#[allow(clippy::too_many_arguments)]
pub(super) fn image_polygon(
    o: &mut Overlay,
    size: (f32, f32),
    poly: &[[f32; 2]],
    to_image: impl Fn([f32; 2]) -> [f32; 2],
    local: impl Fn([f32; 2]) -> [f32; 2],
    flags: u32,
    tint: [f32; 4],
) {
    let mut pts: Vec<[f32; 2]> = poly.to_vec();
    let planes: [&dyn Fn([f32; 2]) -> f32; 4] =
        [&|p| to_image(p)[0] - 0.5, &|p| size.0 - 0.5 - to_image(p)[0], &|p| to_image(p)[1] - 0.5, &|p| size.1 - 0.5 - to_image(p)[1]];
    for f in planes {
        if pts.len() < 3 {
            return;
        }
        let mut out = Vec::with_capacity(pts.len() + 2);
        for i in 0..pts.len() {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            let (fa, fb) = (f(a), f(b));
            if fa >= 0.0 {
                out.push(a);
            }
            if (fa >= 0.0) != (fb >= 0.0) {
                let k = fa / (fa - fb);
                out.push([a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k]);
            }
        }
        pts = out;
    }
    if pts.len() < 3 {
        return;
    }
    let v = |p: [f32; 2]| {
        let i = to_image(p);
        let l = local(p);
        OVertex { pos: p, uv: MapLayer::atlas_uv(i[0], i[1]), color: tint, extra: [flags as f32, l[0], l[1], 0.0] }
    };
    let first = v(pts[0]);
    for k in 1..pts.len() - 1 {
        o.raw_triangle([first, v(pts[k]), v(pts[k + 1])]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Stage;
    use crate::logic::sins::Sin;

    fn at(stage: Stage) -> Game {
        let mut g = Game::new();
        g.map_jump(stage);
        g.finish_loading();
        g.screen = Screen::Playing;
        g
    }

    #[test]
    fn the_minimap_keeps_him_in_the_middle_and_turns_with_the_camera_when_asked() {
        let mut g = at(Stage::Dungeon(0));
        let (w, h) = (1920.0, 1080.0);
        let r = rect(w, h);
        // Top right, inside the safe area, seven units across.
        assert_eq!(r[2], tile_for(w, h) * 7.0);
        let safe = style::safe(w, h);
        assert!((r[0] + r[2] - (w - safe)).abs() < 1e-3 && (r[1] - safe).abs() < 1e-3);
        let win = window(&g, w, h);
        let hp = g.hero.pos();
        let (x, y) = win.place(hp);
        assert!((x - (r[0] + r[2] / 2.0)).abs() < 1e-3 && (y - (r[1] + r[3] / 2.0)).abs() < 1e-3, "he is in the middle");
        // North up: what is north of him is straight above him, east to the right.
        let north = win.place(V::new(hp.x, hp.y - 3.0));
        let east = win.place(V::new(hp.x + 3.0, hp.y));
        assert!((north.0 - x).abs() < 1e-3 && north.1 < y);
        assert!(east.0 > x && (east.1 - y).abs() < 1e-3);
        // His arrow is drawn there: the frame and its shapes round the middle.
        let mut o = Overlay::new();
        o.begin(w, h);
        draw(&mut o, &g, tile_for(w, h));
        assert!(o.map.image.is_some() && o.map.dirty(), "the image built, waiting to go up");
        assert!(o.verts.iter().any(|v| (v.pos[0] - x).abs() < 1.0 && (v.pos[1] - y).abs() < 30.0));
        // Turning: the way the camera looks is up. Aim it east, then south.
        g.set_minimap_turn(true);
        // (The camera behind him looking east, then south: from `angle` round the point it looks at.)
        let target = glam::Vec3::new(hp.x as f32, 0.6, hp.y as f32);
        for (yaw, ahead) in [(-FRAC_PI_2, V::new(hp.x + 4.0, hp.y)), (std::f32::consts::PI, V::new(hp.x, hp.y + 4.0))] {
            g.view.set_closeup(target, yaw, 3.4);
            let win = window(&g, w, h);
            let p = win.place(ahead);
            assert!((p.0 - win.cx).abs() < win.scale * 1.5 && p.1 < win.cy - win.scale * 2.0, "{yaw}: ahead is up ({p:?})");
            // The image under a point and the point of a tile agree.
            let back = win.tile_at([p.0, p.1]);
            assert!((back.0 - ahead.x as f32).abs() < 1e-3 && (back.1 - ahead.y as f32).abs() < 1e-3);
        }
    }

    #[test]
    fn only_objectives_out_of_range_go_to_the_rim_as_arrows() {
        let g = at(Stage::Dungeon(0));
        let win = window(&g, 1280.0, 720.0);
        let hp = g.hero.pos();
        let far = V::new(hp.x + 80.0, hp.y);
        let near = V::new(hp.x + 2.0, hp.y);
        let mk = |kind, at| Marker { kind, at, known: true };
        let ms = vec![
            mk(MarkerKind::Foe(FoeRank::Plain), far),
            mk(MarkerKind::Key, far),
            mk(MarkerKind::Chest, far),
            mk(MarkerKind::Lamp, far),
            mk(MarkerKind::Foe(FoeRank::Boss), far),
            mk(MarkerKind::Shrine { angel: crate::logic::angels::ARCHANGELS[0], spent: false }, far),
            mk(MarkerKind::Shrine { angel: crate::logic::angels::ARCHANGELS[1], spent: true }, far),
            mk(MarkerKind::Gate { open: false }, V::new(hp.x, hp.y + 90.0)),
            mk(MarkerKind::Key, near),
            Marker { kind: MarkerKind::Lamp, at: far, known: false },
            mk(MarkerKind::Hero { heading: 0.0 }, hp),
        ];
        let p = plan(&win, &ms, win.radius - 4.0);
        let arrows: Vec<usize> = p.iter().filter(|e| e.2.is_some()).map(|e| e.0).collect();
        let inside: Vec<usize> = p.iter().filter(|e| e.2.is_none()).map(|e| e.0).collect();
        // The lamp, the boss, the unspent shrine and the gate: arrows; the foe, the key, the chest
        // and the spent shrine out of range: nothing. The near key inside; the unknown never.
        assert_eq!(arrows, vec![3, 4, 5, 7]);
        assert_eq!(inside, vec![8]);
        // The arrows point their way: east for those east of him, south for the gate.
        let a = |i: usize| p.iter().find(|e| e.0 == i).unwrap().2.unwrap();
        assert!(a(3).abs() < 1e-3 && (a(7) - std::f32::consts::FRAC_PI_2).abs() < 1e-3);
        // Where the lamp stands on the gate (a realm's far portal), only the lamp points there.
        let r = at(Stage::Realm(Sin::Envy));
        let win = window(&r, 1280.0, 720.0);
        let ms = markers(&r);
        let gate = ms.iter().position(|m| matches!(m.kind, MarkerKind::Gate { .. })).unwrap();
        let mut ms2 = ms.clone();
        ms2[gate].known = true;
        let p = plan(&win, &ms2, 1.0);
        assert!(p.iter().any(|e| ms2[e.0].kind == MarkerKind::Lamp && e.2.is_some()));
        assert!(!p.iter().any(|e| e.0 == gate));
    }

    #[test]
    fn the_minimap_shows_in_play_and_pause_and_hides_elsewhere() {
        let mut g = at(Stage::Dungeon(0));
        assert!(shown(&g));
        g.screen = Screen::Paused;
        assert!(shown(&g), "under the pause's dim");
        g.screen = Screen::Ready;
        assert!(shown(&g));
        g.open_full_map();
        assert!(!shown(&g), "the full map stands in for it");
        g.close_full_map();
        for s in [Screen::Title, Screen::Story, Screen::Shop, Screen::Select, Screen::Won, Screen::GameOver, Screen::Toll] {
            g.screen = s;
            g.view.showcase = s == Screen::Title;
            assert!(!shown(&g), "{s:?}");
        }
        g.screen = Screen::Playing;
        g.view.showcase = false;
        g.await_renderer = true;
        g.map_jump(Stage::Hub);
        assert!(g.loading.is_some() && !shown(&g), "behind the loading card");
    }
    #[test]
    fn the_minimap_is_quick_and_sends_up_only_what_changed() {
        use crate::logic::types::P;
        // Drawn in the vaults (their 1390 marks of gold the most of any level), every frame's cost.
        let g = at(Stage::Realm(Sin::Greed));
        let (w, h) = (1920.0, 1080.0);
        let t = tile_for(w, h);
        let mut o = Overlay::new();
        o.begin(w, h);
        draw(&mut o, &g, t);
        let whole = o.map.upload(|_, _| {});
        assert_eq!(whole, (1024 * 1024 * 4) as usize, "a new level's image goes up whole, the region cleared");
        // The fastest of 20 batches of 10 draws: a batch the scheduler took the core from (the
        // whole suite running beside it on every core) does not count, only the draw's own cost.
        let (batches, n) = (20, 10);
        let mut ms = f64::MAX;
        for _ in 0..batches {
            let at0 = web_time::Instant::now();
            for _ in 0..n {
                o.begin(w, h);
                draw(&mut o, &g, t);
            }
            ms = ms.min(at0.elapsed().as_secs_f64() * 1e3 / n as f64);
        }
        let verts = o.verts.len();
        // A walk in the garden: each new tile's look sends a small patch.
        let mut g = at(Stage::Dungeon(0));
        o.map = MapLayer::default();
        o.map.sync(&g);
        o.map.upload(|_, _| {});
        let hp = g.hero.pos();
        let mut p = P::new(hp.x.round() as i32, hp.y.round() as i32);
        let mut sent = Vec::new();
        for _ in 0..12 {
            let next = [P::new(p.x, p.y - 1), P::new(p.x + 1, p.y), P::new(p.x - 1, p.y), P::new(p.x, p.y + 1)]
                .into_iter()
                .find(|q| g.level.grid.is_walkable(q.x, q.y, false) && !g.explore.visible(q.x, q.y - 3))
                .unwrap_or(P::new(p.x, p.y - 1));
            p = next;
            g.explore.observe(&g.level.grid, p.v());
            o.map.sync(&g);
            let mut rect = None;
            let bytes = o.map.upload(|r, px| {
                assert_eq!(px.len(), (r.w * r.h * 4) as usize);
                rect = Some(r);
            });
            if bytes > 0 {
                let r = rect.unwrap();
                assert!(r.x >= crate::render::overlay::MAP_REGION.x && r.y + r.h <= crate::render::overlay::MAP_REGION.h);
                sent.push(bytes);
            }
        }
        let (most, mean) = (sent.iter().copied().max().unwrap_or(0), sent.iter().sum::<usize>() / sent.len().max(1));
        println!(
            "minimap: drawn in the vaults in {ms:.4} ms ({verts} vertices); a new level sends {whole} bytes, a step in the garden {mean} bytes on average, {most} at most ({} of 12 steps sent)",
            sent.len()
        );
        assert!(!sent.is_empty() && most < whole / 8, "a patch, not the image");
        // (Alone, about 0.02 ms. The best batch is seldom slowed by the load beside it, so the same
        // limit holds under the whole suite, and a debug build; only the pathological fails.)
        assert!(ms < 5.0, "the minimap took {ms} ms (its best batch of {n})");
    }
}
