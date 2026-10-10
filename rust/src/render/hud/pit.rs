//! The Pit on the HUD (FINALE.md §2), in the HUD's glass: across the top centre the
//! level's clock, large, with "LEVEL n" and the name of its stage over it in the level's colour and
//! a thin bar of the two minutes running out under it; in the last fifteen seconds the clock burns
//! red and pulses with the bell, and the warning sits under the panel. On the way down the panel
//! says where the platform is going. In the purse: how many foes stand on the platform (they
//! will ride down), how many are still behind the doors, how many rode down into this level.

use super::*;
use crate::logic::pit;

/// The clock's red in the last seconds, and the warning's.
const BELL: u32 = 0xff5a3a;

/// The clock panel, top middle.
pub(super) fn draw_clock(o: &mut Overlay, g: &Game) {
    let Some(v) = g.pit_view() else { return };
    let Some((level, left, on, _, _)) = g.pit_status() else { return };
    let w = o.width;
    let l = super::layout::HudLayout::of(w, o.height);
    let t = l.t;
    let pw = (t * 12.0).min(l.top[2]);
    let ph = t * 3.35;
    let (x0, y0) = (w / 2.0 - pw / 2.0, l.top[1]);
    super::style::panel_glass(o, [x0, y0, pw, ph], t, 1.0);
    let time = g.time as f32;
    let cx = w / 2.0;
    let color = pit::light(level);
    if let Some(d) = v.descent {
        // Going down: the next level's name coming up, the clock gone quiet.
        let next = level + 1;
        let going = if level >= pit::LEVELS { "THE BOTTOM".to_string() } else { pit::level_label(next) };
        o.text_styled("THE PLATFORM DESCENDS", cx, y0 + t * 0.6, &super::style::label(t, 0.5, rgba(SAND, 1.0)).centered());
        let k = (d / pit::DESCENT_SECONDS as f32).clamp(0.0, 1.0);
        let c = if level >= pit::LEVELS { 0xe8dcc8 } else { pit::light(next) };
        o.text(&going, cx, y0 + t * 1.6, t * 1.05, rgba(c, 0.4 + 0.6 * k), Align::Center, true);
        let by = y0 + ph - t * 0.6;
        super::style::bar(o, [x0 + t * 0.6, by, pw - t * 1.2, t * 0.16], k, 0.0, c, 1.0);
        return;
    }
    if v.bottom {
        o.text("THE BOTTOM OF THE PIT", cx, y0 + t * 1.5, t * 0.62, rgba(SAND, 1.0), Align::Center, true);
        return;
    }
    // LEVEL n and the stage's name.
    let label = pit::level_label(level);
    o.text_styled(&label, cx, y0 + t * 0.55, &super::style::name(t, 0.56, rgba(color, 1.0)).centered());
    o.text_styled(pit::stage_name(level), cx, y0 + t * 1.08, &super::style::label(t, 0.5, rgba(TEXT_DIM, 1.0)).centered());
    // The clock: large, red and beating with the bell at the end.
    let bell = left <= pit::BELL_SECONDS && left > 0.0;
    let beat = if bell { 1.0 - (left as f32).fract() } else { 0.0 };
    let size = 1.3 * (1.0 + 0.12 * beat.powi(3));
    let clock_color = if bell { mix_rgb(BELL, 0xffffff, beat.powi(4) * 0.5) } else { TEXT };
    let clock = super::style::label(t, size, rgba(clock_color, 1.0)).weight(0.45).tabular().centered();
    o.text_styled(&pit::mmss(pit::LEVEL_SECONDS - left), cx, y0 + t * 2.08, &clock);
    // The two minutes running out.
    let by = y0 + ph - t * 0.5;
    let frac = (left / pit::LEVEL_SECONDS) as f32;
    super::style::bar(o, [x0 + t * 0.6, by, pw - t * 1.2, t * 0.16], frac, 0.0, if bell { BELL } else { color }, 1.0);
    if bell {
        let a = 0.6 + 0.4 * beat;
        let words = match on {
            0 => "THE PLATFORM DESCENDS. IT IS CLEAR".to_string(),
            1 => "THE PLATFORM DESCENDS. CLEAR IT: 1 STANDS ON IT".to_string(),
            n => format!("THE PLATFORM DESCENDS. CLEAR IT: {n} STAND ON IT"),
        };
        o.text_styled(&words, cx, y0 + ph + t * 0.55, &super::style::label(t, super::style::size::LABEL, rgba(BELL, a)).centered());
        // The screen's edge reddens with each toll.
        super::style::vignette(o, t * 2.2, [1.0, 0.3, 0.15, 0.5 * beat.powi(2)]);
    }
    let _ = time;
}

/// Under the score: who will ride down, who is still to come, who came down.
pub(super) fn draw_status(o: &mut Overlay, g: &Game, rc: f32, ty: f32, below: f32, small: f32) {
    let Some((level, left, on, waiting, came)) = g.pit_status() else { return };
    let warn = left <= pit::BELL_SECONDS && on > 0;
    let text = format!("ON THE PLATFORM {on}");
    o.text_styled(
        &text,
        rc,
        ty,
        &super::style::label(
            small / super::style::size::LABEL,
            super::style::size::LABEL,
            rgba(if warn { BELL } else { pit::light(level) }, 1.0),
        )
        .right(),
    );
    let more = if waiting > 0 { format!("{waiting} BEHIND THE DOORS") } else { "THE DOORS ARE SHUT".to_string() };
    let more = if came > 0 { format!("{more}   {came} RODE DOWN") } else { more };
    o.text_styled(
        &more,
        rc,
        below,
        &super::style::label(small / super::style::size::LABEL, super::style::size::LABEL * 0.85, rgba(TEXT_DIM, 1.0)).right(),
    );
}
