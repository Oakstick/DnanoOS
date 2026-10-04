//! The Pit on the HUD (FINALE.md §2), in the panels' charcoal and copper: across the top middle the
//! level's clock, large, with "LEVEL n" and the name of its stage over it in the level's colour and
//! a thin bar of the two minutes running out under it; in the last fifteen seconds the clock burns
//! red and pulses with the bell, and the warning sits under the panel. On the way down the panel
//! says where the platform is going. Under the score: how many foes stand on the platform (they
//! will ride down), how many are still behind the doors, how many rode down into this level.

use super::*;
use crate::logic::pit;

/// The clock's red in the last seconds, and the warning's.
const BELL: u32 = 0xff5a3a;

/// The clock panel, top middle.
pub(super) fn draw_clock(o: &mut Overlay, g: &Game, t: f32) {
    let Some(v) = g.pit_view() else { return };
    let Some((level, left, on, _, _)) = g.pit_status() else { return };
    let w = o.width;
    let margin = t * 0.4;
    let pw = (t * 11.0).min(w * 0.4);
    let ph = t * 3.35;
    let (x0, y0) = (w / 2.0 - pw / 2.0, margin);
    o.rect(x0, y0, pw, ph, [INK[0], INK[1], INK[2], 0.86]);
    o.rect_outline(x0, y0, pw, ph, (t * 0.06).max(1.0), [EDGE[0], EDGE[1], EDGE[2], 0.26]);
    o.rect(x0 + t * 0.3, y0, pw - t * 0.6, (t * 0.07).max(1.0), rgba(COPPER, 0.9));
    let time = g.time as f32;
    let cx = w / 2.0;
    let color = pit::light(level);
    if let Some(d) = v.descent {
        // Going down: the next level's name coming up, the clock gone quiet.
        let next = level + 1;
        let going = if level >= pit::LEVELS { "THE BOTTOM".to_string() } else { pit::level_label(next) };
        o.text("THE PLATFORM DESCENDS", cx, y0 + t * 0.6, t * 0.44, rgba(SAND, 1.0), Align::Center, true);
        let k = (d / pit::DESCENT_SECONDS as f32).clamp(0.0, 1.0);
        let c = if level >= pit::LEVELS { 0xe8dcc8 } else { pit::light(next) };
        o.text(&going, cx, y0 + t * 1.6, t * 1.05, rgba(c, 0.4 + 0.6 * k), Align::Center, true);
        let by = y0 + ph - t * 0.55;
        o.rect(x0 + t * 0.5, by, pw - t, t * 0.14, [1.0, 1.0, 1.0, 0.1]);
        o.rect(x0 + t * 0.5, by, (pw - t) * k, t * 0.14, rgba(c, 0.9));
        return;
    }
    if v.bottom {
        o.text("THE BOTTOM OF THE PIT", cx, y0 + t * 1.5, t * 0.62, rgba(SAND, 1.0), Align::Center, true);
        return;
    }
    // LEVEL n and the stage's name.
    let label = pit::level_label(level);
    o.text(&label, cx, y0 + t * 0.55, t * 0.5, rgba(color, 1.0), Align::Center, true);
    o.text(pit::stage_name(level), cx, y0 + t * 1.0, t * 0.36, rgba(TEXT_DIM, 1.0), Align::Center, true);
    // The clock: large, red and beating with the bell at the end.
    let bell = left <= pit::BELL_SECONDS && left > 0.0;
    let beat = if bell { 1.0 - (left as f32).fract() } else { 0.0 };
    let size = t * 1.35 * (1.0 + 0.12 * beat.powi(3));
    let clock_color = if bell { mix_rgb(BELL, 0xffffff, beat.powi(4) * 0.5) } else { TEXT };
    o.text(&pit::mmss(pit::LEVEL_SECONDS - left), cx, y0 + t * 2.0, size, rgba(clock_color, 1.0), Align::Center, true);
    // The two minutes running out.
    let by = y0 + ph - t * 0.45;
    let frac = (left / pit::LEVEL_SECONDS) as f32;
    o.rect(x0 + t * 0.5, by, pw - t, t * 0.12, [1.0, 1.0, 1.0, 0.1]);
    o.rect(x0 + t * 0.5, by, (pw - t) * frac, t * 0.12, rgba(if bell { BELL } else { color }, 0.9));
    if bell {
        let a = 0.6 + 0.4 * beat;
        let words = match on {
            0 => "THE PLATFORM DESCENDS. IT IS CLEAR".to_string(),
            1 => "THE PLATFORM DESCENDS. CLEAR IT: 1 STANDS ON IT".to_string(),
            n => format!("THE PLATFORM DESCENDS. CLEAR IT: {n} STAND ON IT"),
        };
        o.text(&words, cx, y0 + ph + t * 0.55, t * 0.5, rgba(BELL, a), Align::Center, true);
        // The screen's edge reddens with each toll.
        let edge = (t * 0.25).max(3.0);
        let c = [1.0, 0.3, 0.15, 0.35 * beat.powi(2)];
        o.rect(0.0, 0.0, w, edge, c);
        o.rect(0.0, o.height - edge, w, edge, c);
        o.rect(0.0, 0.0, edge, o.height, c);
        o.rect(w - edge, 0.0, edge, o.height, c);
    }
    let _ = time;
}

/// Under the score: who will ride down, who is still to come, who came down.
pub(super) fn draw_status(o: &mut Overlay, g: &Game, rc: f32, ty: f32, below: f32, small: f32) {
    let Some((level, left, on, waiting, came)) = g.pit_status() else { return };
    let warn = left <= pit::BELL_SECONDS && on > 0;
    let text = format!("ON THE PLATFORM {on}");
    o.text(&text, rc, ty, small, rgba(if warn { BELL } else { pit::light(level) }, 1.0), Align::Right, true);
    let more = if waiting > 0 { format!("{waiting} BEHIND THE DOORS") } else { "THE DOORS ARE SHUT".to_string() };
    let more = if came > 0 { format!("{more}   {came} RODE DOWN") } else { more };
    o.text(&more, rc, below, small * 0.85, rgba(TEXT_DIM, 1.0), Align::Right, true);
}
