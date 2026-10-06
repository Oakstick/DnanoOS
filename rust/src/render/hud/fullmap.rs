//! The full map (UI.md §7), a page over the pause (`game::fullmap`): the explored level large in a
//! panel, north up, what the map knows drawn on it (`minimap::draw_marker`) with the hero's arrow;
//! the place's name and what he must do there (the old man's guide's first word on it) above; a
//! legend of what is marked here beside it; and the controls below, in the glyphs of the device
//! used last.

use super::minimap::{clip_to_rect, draw_marker, image_polygon, objective, platform, pointer};
use super::*;
use crate::logic::types::V;
use crate::render::minimap::{markers, FoeRank, MarkerKind};

/// The page's parts on screen.
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    /// The map's panel, and the area the level is drawn in inside it.
    pub panel: [f32; 4],
    pub inner: [f32; 4],
    /// The legend's panel, at the right.
    pub legend: [f32; 4],
    pub title_y: f32,
    pub objective_y: f32,
    pub prompt_y: f32,
    pub t: f32,
}

/// Where the page's parts sit on a screen `w` by `h`.
pub fn layout(w: f32, h: f32) -> Layout {
    let t = tile_for(w, h);
    let safe = style::safe(w, h);
    let top = safe + t * 3.1;
    let bottom = h - safe - t * 1.4;
    let legend_w = (t * 10.0).min(w * 0.26);
    let gap = t * 0.5;
    let panel = [safe, top, w - 2.0 * safe - legend_w - gap, bottom - top];
    let pad = t * 0.3;
    Layout {
        panel,
        inner: [panel[0] + pad, panel[1] + pad, panel[2] - 2.0 * pad, panel[3] - 2.0 * pad],
        legend: [w - safe - legend_w, top, legend_w, bottom - top],
        title_y: safe + t * 0.75,
        objective_y: safe + t * 2.0,
        prompt_y: h - safe - t * 0.45,
        t,
    }
}

/// Pixels a tile at zoom 1: the whole level of `tiles` fits the map's area.
pub fn fit(lay: &Layout, tiles: (i32, i32)) -> f32 {
    (lay.inner[2] / tiles.0.max(1) as f32).min(lay.inner[3] / tiles.1.max(1) as f32)
}

/// The closest zoom: three units a tile.
pub fn max_zoom(lay: &Layout, fit: f32) -> f32 {
    (lay.t * 3.0 / fit.max(1e-3)).max(1.0)
}

/// What the legend calls each kind of mark, in the order it lists them; a kind of no row is none.
fn legend_row(kind: MarkerKind) -> Option<(usize, &'static str)> {
    Some(match kind {
        MarkerKind::Hero { .. } => (0, "You"),
        MarkerKind::Lamp => (1, "The old man's lamp"),
        MarkerKind::Gate { .. } => (2, "The gate"),
        MarkerKind::Entry => (3, "The way in"),
        MarkerKind::Portal { .. } => (4, "A portal"),
        MarkerKind::HouseDoor { .. } => (5, "The House's door"),
        MarkerKind::PitDoor { .. } => (6, "A door of the Pit"),
        MarkerKind::Platform { .. } => (7, "The platform"),
        MarkerKind::Cage { .. } => (8, "The cage"),
        MarkerKind::Shrine { .. } => (9, "An archangel's shrine"),
        MarkerKind::Key => (10, "A golden key"),
        MarkerKind::RedKey => (11, "The red key"),
        MarkerKind::Chest => (12, "A chest"),
        MarkerKind::Loot => (13, "Gold and gems"),
        MarkerKind::Remedy => (14, "A remedy"),
        MarkerKind::Potion => (15, "A potion"),
        MarkerKind::ItemShrine => (16, "An item shrine"),
        MarkerKind::Instrument { .. } => (17, "The general's instrument"),
        MarkerKind::Post => (18, "His command post"),
        MarkerKind::Foe(FoeRank::Plain) => (19, "A foe"),
        MarkerKind::Foe(FoeRank::Elite) => (20, "A pack's leader"),
        MarkerKind::Foe(FoeRank::Rival) => (21, "A rival"),
        MarkerKind::Foe(FoeRank::General) => (22, "The general"),
        MarkerKind::Foe(FoeRank::Boss) => (23, "The boss"),
    })
}
const LEGEND_ROWS: usize = 24;

pub fn draw(o: &mut Overlay, g: &Game, t: f32) {
    let Some(m) = &g.map_ui.full else { return };
    o.map.sync(g);
    let Some((iw, ih, ppt)) = o.map.image.as_ref().map(|i| (i.w as f32, i.h as f32, i.px_per_tile as f32)) else { return };
    let (w, h) = (o.width, o.height);
    let lay = layout(w, h);
    let tiles = (g.level.grid.width, g.level.grid.height);
    let s = fit(&lay, tiles) * m.zoom;
    let fade = (m.age / 0.15).clamp(0.0, 1.0);
    let before = o.alpha;
    o.alpha = before * fade;

    // Over the pause: a dim, the place's name and what he must do there.
    o.rect(0.0, 0.0, w, h, [INK[0] * 0.6, INK[1] * 0.6, INK[2] * 0.6, 0.94]);
    let (guide, _) = g.guide_now();
    let title = style::name(t, 1.15, rgba(GOLD, 1.0)).centered();
    o.text_styled(&guide.title, w / 2.0, lay.title_y, &title);
    if let Some(task) = guide.task.first() {
        let body = style::body(t, style::size::BODY, rgba(SAND, 1.0)).centered();
        let lines = o.wrap_lines(task, (w - 2.0 * style::safe(w, h)).min(t * 40.0), &body);
        let line_h = t * 0.75;
        let y0 = lay.objective_y - (lines.len().min(2) as f32 - 1.0) * line_h / 2.0;
        for (k, line) in lines.iter().take(2).enumerate() {
            let text = if k == 1 && lines.len() > 2 { format!("{line}...") } else { line.clone() };
            o.text_styled(&text, w / 2.0, y0 + k as f32 * line_h, &body);
        }
    }

    // The map's panel, and the level in it.
    let [px, py, pw, ph] = lay.panel;
    style::panel_glass(o, [px, py, pw, ph], t, 1.0);
    let [ix, iy, iw2, ih2] = lay.inner;
    o.rect(ix, iy, iw2, ih2, [0.0, 0.0, 0.0, 0.45]);
    let pc = (ix + iw2 / 2.0, iy + ih2 / 2.0);
    let place = |at: V| (pc.0 + (at.x as f32 - m.centre.0) * s, pc.1 + (at.y as f32 - m.centre.1) * s);
    // A faint grid every eight tiles, so its scale reads.
    let line = [1.0, 1.0, 1.0, 0.035];
    let (gx0, gy0) = place(V::new(-0.5, -0.5));
    let step = 8.0 * s;
    let mut k = ((ix - gx0) / step).ceil();
    while gx0 + k * step < ix + iw2 {
        o.rect(gx0 + k * step, iy, 1.0, ih2, line);
        k += 1.0;
    }
    let mut k = ((iy - gy0) / step).ceil();
    while gy0 + k * step < iy + ih2 {
        o.rect(ix, gy0 + k * step, iw2, 1.0, line);
        k += 1.0;
    }
    let to_image = |p: [f32; 2]| {
        let (x, y) = ((p[0] - pc.0) / s + m.centre.0, (p[1] - pc.1) / s + m.centre.1);
        [(x + 0.5) * ppt, (y + 0.5) * ppt]
    };
    let area = [[ix, iy], [ix + iw2, iy], [ix + iw2, iy + ih2], [ix, iy + ih2]];
    image_polygon(o, (iw, ih), &area, to_image, |_| [0.0, 0.0], 0, [1.0; 4]);

    // What the map knows, the hero on top; the hub's portals named.
    let ms = markers(g);
    let u = (s * 0.42).clamp(t * 0.13, t * 0.3);
    let inside = |(x, y): (f32, f32), r: f32| x >= ix + r && x <= ix + iw2 - r && y >= iy + r && y <= iy + ih2 - r;
    let mut present = [false; LEGEND_ROWS];
    let mut samples: [Option<(MarkerKind, u32)>; LEGEND_ROWS] = [None; LEGEND_ROWS];
    let label = style::label(t, style::size::MARK, rgba(TEXT, 1.0)).centered();
    for mk in ms.iter().filter(|m| m.known) {
        if let Some((row, _)) = legend_row(mk.kind) {
            present[row] = true;
            samples[row].get_or_insert((mk.kind, mk.color()));
        }
        let p = place(mk.at);
        if let MarkerKind::Platform { half } = mk.kind {
            platform(o, mk.at, half, place, |a, b| clip_to_rect(a, b, lay.inner), (u * 0.35).max(1.5), mk.color());
            continue;
        }
        if !inside(p, u * 1.5) {
            // An objective off the panel: an arrow at its edge, pointing its way.
            if objective(mk.kind) {
                let (dx, dy) = (p.0 - pc.0, p.1 - pc.1);
                let edge = (iw2 / 2.0 - t * 0.6) / dx.abs().max(1e-3);
                let k = edge.min((ih2 / 2.0 - t * 0.6) / dy.abs().max(1e-3));
                let lamp = mk.kind == MarkerKind::Lamp;
                pointer(o, pc.0 + dx * k, pc.1 + dy * k, dy.atan2(dx), mk.color(), t * 0.55, g.time as f32, lamp);
            }
            continue;
        }
        let u = if matches!(mk.kind, MarkerKind::Hero { .. }) { u * 1.35 } else { u };
        draw_marker(o, mk.kind, mk.color(), p.0, p.1, u, s, 0.0, g.time as f32);
        if let MarkerKind::Portal { sin, .. } = mk.kind {
            o.text_styled(sin.title(), p.0, p.1 + u * 2.6, &label.color(rgba(sin.color(), 1.0)));
        }
    }
    // The north mark, in the panel's corner.
    let (nx, ny, nr) = (ix + iw2 - t * 0.75, iy + t * 0.75, t * 0.42);
    style::badge(o, "N", nx, ny, nr, GOLD, 1.0);

    // The legend: what is marked here.
    let [lx, ly, _, lh] = lay.legend;
    style::panel_glass(o, lay.legend, t, 1.0);
    let below = style::panel_header(o, lay.legend, "LEGEND", t, GOLD, 1.0);
    let row_h = (t * 0.95).min((ly + lh - below - t * 0.5) / present.iter().filter(|&&p| p).count().max(1) as f32);
    let text = style::body(t, style::size::LABEL, rgba(TEXT, 1.0));
    let mut y = below + row_h * 0.6;
    let lu = (row_h * 0.22).min(t * 0.24);
    for row in 0..LEGEND_ROWS {
        let (Some((kind, color)), true) = (samples[row], present[row]) else { continue };
        let Some((_, name)) = legend_row(kind) else { continue };
        let kind = match kind {
            MarkerKind::Hero { .. } => MarkerKind::Hero { heading: -std::f32::consts::FRAC_PI_2 },
            MarkerKind::Platform { .. } => MarkerKind::Platform { half: 1.6 },
            k => k,
        };
        draw_marker(o, kind, color, lx + t * 0.9, y, lu, lu * 0.9, 0.0, g.time as f32);
        o.text_styled(name, lx + t * 1.8, y, &text);
        y += row_h;
    }

    // The controls, in the glyphs of the device used last.
    let prompts = if g.input.last_device().is_pad() {
        "{Move} move    {Look} or {TabPrev} {TabNext} zoom    {Secondary} back to you    {Map} {Back} close"
    } else {
        "{Move} or drag to move    the wheel or {TabPrev} {TabNext} to zoom    {Secondary} back to you    {Map} or {Back} to close"
    };
    glyphs::prompt_line(o, &g.input, prompts, w / 2.0, lay.prompt_y, t * 0.5, rgba(TEXT_DIM, 1.0), Align::Center);
    o.alpha = before;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Stage;

    #[test]
    fn the_page_fits_the_screen_and_draws_quickly() {
        for (w, h) in [(1280.0, 720.0), (1920.0, 1080.0), (3840.0, 2160.0)] {
            let lay = layout(w, h);
            for r in [lay.panel, lay.legend] {
                assert!(r[0] >= 0.0 && r[1] >= 0.0 && r[0] + r[2] <= w && r[1] + r[3] <= h, "{r:?} on {w}x{h}");
            }
            assert!(lay.panel[0] + lay.panel[2] < lay.legend[0], "side by side");
            assert!(lay.title_y < lay.objective_y && lay.objective_y < lay.panel[1] && lay.panel[1] + lay.panel[3] < lay.prompt_y);
        }
        let mut g = Game::new();
        g.map_jump(Stage::Realm(crate::logic::sins::Sin::Greed));
        g.finish_loading();
        g.screen = Screen::Playing;
        g.open_full_map();
        let mut o = Overlay::new();
        o.begin(1920.0, 1080.0);
        let t = tile_for(1920.0, 1080.0);
        draw(&mut o, &g, t);
        let n = 50;
        let at = web_time::Instant::now();
        for _ in 0..n {
            o.begin(1920.0, 1080.0);
            draw(&mut o, &g, t);
        }
        let ms = at.elapsed().as_secs_f64() * 1e3 / n as f64;
        println!("full map: the vaults drawn in {ms:.3} ms, {} vertices", o.verts.len());
        assert!(!o.verts.is_empty());
    }
}
