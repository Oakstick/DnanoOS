//! The map, opened from the title or the pause menu (see `game::map`): the
//! road down as a column of stops, dungeon then Cyclops, depth by depth, with
//! the highlighted one ready to jump to.

use super::*;
use crate::game::{map_stops, Stage};

/// Where each stop's row sits on the map page.
pub fn map_rects(count: usize, w: f32, h: f32) -> Vec<[f32; 4]> {
    let t = tile_for(w, h);
    let (bw, bh, gap) = ((t * 22.0).min(w - t * 2.0), t * 1.25, t * 0.3);
    let total = count as f32 * bh + (count as f32 - 1.0).max(0.0) * gap;
    let y0 = (h - total) / 2.0 + t * 0.6;
    (0..count).map(|i| [(w - bw) / 2.0, y0 + i as f32 * (bh + gap), bw, bh]).collect()
}

pub fn draw_map(o: &mut Overlay, g: &Game, t: f32) {
    let (w, h) = (o.width, o.height);
    o.rect(0.0, 0.0, w, h, [2.0 / 255.0, 6.0 / 255.0, 12.0 / 255.0, 0.92]);
    let stops = map_stops();
    let rects = map_rects(stops.len(), w, h);
    let top = rects.first().map(|r| r[1]).unwrap_or(h / 2.0);
    o.text("MAP", w / 2.0, top - t * 2.6, t * 1.4, rgba(GOLD, 1.0), Align::Center, true);
    o.text("For testing: jump to any stop on the road down", w / 2.0, top - t * 1.3, t * 0.5, rgba(SAND, 1.0), Align::Center, false);
    let pulse = 0.75 + (g.time as f32 * 4.0).sin() * 0.25;
    for (i, (stop, r)) in stops.iter().zip(&rects).enumerate() {
        let selected = i == g.map_index;
        let arena = matches!(stop.stage, Stage::Arena(_));
        o.rect(r[0], r[1], r[2], r[3], [2.0 / 255.0, 6.0 / 255.0, 12.0 / 255.0, 0.85]);
        o.rect(r[0], r[1], r[2], r[3], if selected { [1.0, 1.0, 1.0, 0.14] } else { [1.0, 1.0, 1.0, 0.04] });
        let edge = if selected { rgba(GOLD, pulse) } else { [1.0, 1.0, 1.0, 0.16] };
        o.rect_outline(r[0], r[1], r[2], r[3], (t * 0.08).max(1.0), edge);
        // A mark down the left: gold for a dungeon, red for a Cyclops.
        o.rect(r[0], r[1], t * 0.14, r[3], rgba(if arena { 0xe0604a } else { GOLD }, if selected { 1.0 } else { 0.5 }));
        let color = if selected { GOLD } else if arena { 0xd8a898 } else { TEXT_DIM };
        o.text(&stop.label, r[0] + t * 0.8, r[1] + r[3] / 2.0, t * 0.58, rgba(color, 1.0), Align::Left, true);
    }
    let bottom = rects.last().map(|r| r[1] + r[3]).unwrap_or(h / 2.0);
    o.text("Up / Down or LT / RT: choose     Enter or A: go     Esc or B: back", w / 2.0, bottom + t * 1.2, t * 0.48, rgba(TEXT_DIM, 1.0), Align::Center, false);
}
