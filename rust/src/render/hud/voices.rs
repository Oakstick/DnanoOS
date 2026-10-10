//! The two voices' signs on screen (WARBAND.md §10, §13; their words are in `dialogue`): in a
//! realm, about the hero, the realm's compass needle (pale, or turned toward a den, flickering in
//! the sin's colour) and the old man's amber lamp, always true, both shown only while he stands
//! idle.

use super::*;
use crate::game::counsel::AMBER;
use crate::logic::types::V;

/// Where a world point lands on screen.
fn screen(g: &Game, p: V, up: f32, w: f32, h: f32) -> Option<(f32, f32)> {
    let (x, y) = (p.x as f32, p.y as f32);
    g.view.project(Vec3::new(x, floor_at(&g.level.grid, x, y) + up, y), w, h)
}

/// In a realm, while he stands idle: the compass about him (the realm's needle, the old man's lamp).
pub(super) fn draw_signs(o: &mut Overlay, g: &Game, t: f32) {
    if g.screen != Screen::Playing || !g.hero.alive || g.realm.is_none() || !(g.in_vault() || g.in_run()) {
        return;
    }
    // Only when he stands idle: the compass is for the moment he stops to look.
    let shown = g.compass_shown();
    if shown <= 0.0 {
        return;
    }
    let (w, h) = (o.width, o.height);
    let hp = g.hero.pos();
    let time = g.time as f32;
    // A unit step on screen toward a world point, from him.
    let bearing = |to: V| -> Option<(f32, f32)> {
        let (dx, dy) = (to.x - hp.x, to.y - hp.y);
        let l = (dx * dx + dy * dy).sqrt();
        if l < 0.5 {
            return None;
        }
        let a = screen(g, hp, 0.2, w, h)?;
        let b = screen(g, V::new(hp.x + dx / l, hp.y + dy / l), 0.2, w, h)?;
        let (sx, sy) = (b.0 - a.0, b.1 - a.1);
        let sl = (sx * sx + sy * sy).sqrt().max(1e-3);
        Some((sx / sl, sy / sl))
    };
    if let Some(c) = screen(g, hp, 0.2, w, h) {
        // The old man's lamp: a small amber diamond toward the far portal, always true.
        if let Some((ux, uy)) = g.gate.and_then(|p| bearing(p.v())) {
            let r = t * 4.5;
            let (lx, ly) = (c.0 + ux * r, c.1 + uy * r);
            let s = t * 0.34;
            let a = (0.85 + (time * 2.0).sin() * 0.1) * shown;
            o.line([lx, ly - s], [lx + s, ly], 2.5, rgba(AMBER, a));
            o.line([lx + s, ly], [lx, ly + s], 2.5, rgba(AMBER, a));
            o.line([lx, ly + s], [lx - s, ly], 2.5, rgba(AMBER, a));
            o.line([lx - s, ly], [lx, ly - s], 2.5, rgba(AMBER, a));
            o.circle(lx, ly, s * 0.35, rgba(AMBER, a));
        }
        // The realm's needle: pale stone toward the portal, or turned (the sin's colour, flickering).
        if let Some((to, turned)) = g.compass() {
            if let Some((ux, uy)) = bearing(to) {
                let sin = g.realm.unwrap_or(crate::logic::sins::Sin::Greed);
                let (color, alpha) = if turned {
                    let flicker = if ((time * 13.0).sin() + (time * 7.3).sin()) > 0.4 { 0.45 } else { 1.0 };
                    // (The prince's colour, never one that could pass for the old man's lamp.)
                    (crate::game::counsel::whisper_color(sin), flicker)
                } else {
                    (0xd7ccb8, 0.8)
                };
                let alpha = alpha * shown;
                let (r0, r1) = (t * 2.6, t * 3.7);
                let tip = (c.0 + ux * r1, c.1 + uy * r1);
                let (px, py) = (-uy, ux);
                let base = (c.0 + ux * r0, c.1 + uy * r0);
                let half = t * 0.32;
                let edges = [
                    ([base.0 + px * half, base.1 + py * half], [tip.0, tip.1]),
                    ([base.0 - px * half, base.1 - py * half], [tip.0, tip.1]),
                    ([base.0 + px * half, base.1 + py * half], [base.0 - px * half, base.1 - py * half]),
                ];
                // A dark rim under it, so it reads on gold floors too.
                for (a, b) in edges {
                    o.line(a, b, 6.5, [0.0, 0.0, 0.0, 0.6]);
                }
                for (a, b) in edges {
                    o.line(a, b, 3.0, rgba(color, alpha));
                }
            }
        }
    }
}
