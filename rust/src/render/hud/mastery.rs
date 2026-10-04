//! The heroes' mastery on screen (WARBAND.md §11.1, §12): the toast when a rank's deeds are met,
//! the trial's line under the ring's banner and its card when it is over, and the training
//! yard's card: each rank, what it gives, its deeds and its trial.

use super::*;
use crate::logic::story::roman;
use crate::logic::war::mastery::{self as m, Rule};

/// "MASTERY RISES", low in the middle, while the toast lasts.
pub(super) fn draw_mastery_toast(o: &mut Overlay, g: &Game, t: f32) {
    let left = g.mastery.toast as f32;
    if left <= 0.0 || !matches!(g.screen, Screen::Playing | Screen::Ready) {
        return;
    }
    let shown = crate::game::mastery::TOAST_SECONDS as f32 - left;
    let alpha = (shown / 0.25).min(1.0) * (left / 0.6).min(1.0);
    let way = m::way(g.hero_kind);
    let (w, h) = (t * 15.0, t * 2.7);
    // Low over the belt, clear of the banners and the floaters over him.
    let slide = (1.0 - (shown / 0.25).min(1.0)) * t * 0.8;
    let (x, y) = ((o.width - w) / 2.0, o.height - t * 7.2 + slide);
    o.alpha = alpha;
    panel(o, x, y, w, h, t);
    let cx = o.width / 2.0;
    let glow = 0.5 + 0.5 * (g.time as f32 * 5.0).sin();
    o.text("MASTERY RISES", cx, y + t * 0.8, t * 0.8, rgba(mix_rgb(GOLD, 0xffffff, glow * 0.4), 1.0), Align::Center, true);
    let rank = roman(g.mastery.toast_rank.max(1) as usize);
    o.text(&format!("{}  -  RANK {rank}", way.name), cx, y + t * 1.55, t * 0.46, rgba(SAND, 1.0), Align::Center, true);
    o.text("\"The old man would test you.\"", cx, y + t * 2.2, t * 0.44, rgba(TEXT, 0.95), Align::Center, false);
    o.alpha = 1.0;
}

/// Under the ring's banner during a trial: its deeds so far, its rule, its time.
pub(super) fn trial_line(g: &Game) -> Option<String> {
    let run = g.mastery.trial.as_ref()?;
    let tr = run.trial;
    let secs = g.brawl.as_ref().map(|b| b.stats.frames as f64 / 60.0).unwrap_or(0.0);
    let rule = if tr.rule == Rule::Free { String::new() } else { format!("   {}", tr.rule.name()) };
    Some(format!("{} {}/{}{rule}   {:.0}/{:.0}s", tr.feat.name(), run.count.min(tr.count), tr.count, secs, tr.limit))
}

/// The trial's card through its aftermath: passed or failed, the deeds, why, the rank.
pub(super) fn draw_trial_results(o: &mut Overlay, g: &Game, t: f32) {
    let (Some(run), Some(card)) = (g.mastery.trial.as_ref(), g.mastery.last_trial.as_ref()) else { return };
    let tr = run.trial;
    let done = if card.count >= card.of { 0x7ad87a } else { 0xffb050 };
    let mut rows: Vec<(&str, String, u32)> = vec![
        ("DEEDS", format!("{} {} of {}", card.count, tr.feat.name(), card.of), done),
        ("TIME", format!("{:.1} s   (allowed {:.0} s)", card.secs, tr.limit), TEXT),
    ];
    if tr.rule != Rule::Free {
        let kept = card.passed || !card.why.contains("YOUR") && !card.why.contains("WOUNDED");
        rows.push((
            "RULE",
            format!("{}   {}", tr.rule.name(), if kept { "KEPT" } else { "BROKEN" }),
            if kept { 0x7ad87a } else { 0xff4a3a },
        ));
    }
    if card.granted {
        let way = m::way(card.hero);
        rows.push(("RANK", format!("{} - {}", roman(card.rank as usize), way.ranks[(card.rank - 1) as usize].to_uppercase()), GOLD));
    } else if card.passed {
        rows.push(("RANK", "NOT YET: ITS DEEDS ARE NOT ALL DONE".to_string(), TEXT_DIM));
    } else {
        rows.push(("WHY", card.why.to_string(), 0xffb050));
    }
    let (w, h) = (t * 17.0, t * (3.4 + rows.len() as f32 * 0.82));
    let (x, y) = ((o.width - w) / 2.0, o.height * 0.7 - h / 2.0);
    panel(o, x, y, w, h, t);
    let cx = o.width / 2.0;
    let (head, color) = if card.passed { ("TRIAL PASSED", GOLD) } else { ("TRIAL FAILED", 0xff6a4a) };
    o.text(head, cx, y + t * 1.0, t * 0.85, rgba(color, 1.0), Align::Center, true);
    o.text(&format!("TRIAL {}: {}", roman(tr.rank as usize), tr.name), cx, y + t * 1.75, t * 0.45, rgba(SAND, 0.95), Align::Center, false);
    let size = t * 0.46;
    let mut ry = y + t * 2.9;
    for (label, value, c) in rows {
        o.text(label, x + t * 0.8, ry, size, rgba(GOLD, 0.85), Align::Left, true);
        o.text(&value, x + t * 3.6, ry, size, rgba(c, 0.95), Align::Left, false);
        ry += t * 0.82;
    }
}

/// The training yard's card for the trials' row: the way, each rank with what it gives, its deeds
/// and its trial, and the last trial's card.
pub(super) fn draw_mastery_card(o: &mut Overlay, g: &Game, t: f32, c: [f32; 4]) {
    let hero = g.hero_kind;
    let way = m::way(hero);
    let mst = g.progress.hero(hero).mastery;
    let pad = t * 0.5;
    let (x, w) = (c[0] + pad, c[2] - pad * 2.0);
    let mut y = c[1] + pad + t * 0.35;
    o.text(way.name, x, y, t * 0.66, rgba(GOLD, 1.0), Align::Left, true);
    o.text(&format!("RANK {} / {}", mst.rank, m::RANKS), x + w, y, t * 0.5, rgba(SAND, 1.0), Align::Right, true);
    y += t * 0.7;
    let deeds: Vec<String> = way.feats.iter().map(|&f| format!("{} {}", mst.of(hero, f), f.name())).collect();
    o.text(&deeds.join("   "), x, y, t * 0.4, rgba(TEXT_DIM, 1.0), Align::Left, false);
    y += t * 0.55;
    let line = ((c[1] + c[3] - pad - t * 0.9 - y) / m::RANKS as f32).clamp(t * 0.9, t * 1.3);
    let size = t * 0.4;
    let bw = t * 6.0;
    for r in 1..=m::RANKS {
        let k = (r - 1) as usize;
        let tr = m::trial(hero, r);
        let held = mst.rank >= r;
        let next = mst.rank + 1 == r;
        let ry = y + line * k as f32;
        if next {
            o.rect(x - t * 0.2, ry - t * 0.05, w + t * 0.4, line - t * 0.1, [0.43, 0.6, 0.56, 0.12]);
        }
        let color = if held {
            GOLD
        } else if next {
            TEXT
        } else {
            0x8a857a
        };
        o.text(&roman(r as usize), x + t * 0.35, ry + t * 0.3, size * 1.1, rgba(color, 1.0), Align::Center, true);
        o.text(&way.ranks[k].to_uppercase(), x + t * 0.95, ry + t * 0.3, size, rgba(color, 1.0), Align::Left, true);
        let ty = ry + t * 0.3 + line * 0.42;
        let tried = if mst.tries[k] > 0 { format!("   tried {}, best {}", mst.tries[k], mst.best[k]) } else { String::new() };
        o.text(&format!("TRIAL: {}{tried}", tr.name), x + t * 0.95, ty, size * 0.88, rgba(TEXT_DIM, 1.0), Align::Left, false);
        // Its standing at the right: held, the deeds' bar, or the trial set.
        let rx = x + w - bw;
        if held {
            o.text("MASTERED", x + w, ty, size, rgba(0x7ad87a, 1.0), Align::Right, true);
        } else if next {
            let (have, need) = mst.progress().unwrap_or((0, 1));
            if have >= need {
                let pulse = 0.7 + 0.3 * (g.time as f32 * 4.0).sin();
                o.text("THE TRIAL IS SET", x + w, ty, size, rgba(GOLD, pulse), Align::Right, true);
            } else {
                bar(o, rx, ty - t * 0.12, bw - t * 2.2, t * 0.24, have as f32 / need as f32, 0x9ad1ff);
                o.text(&format!("{have}/{need}"), x + w, ty, size * 0.9, rgba(SAND, 1.0), Align::Right, false);
            }
        } else {
            o.text(&format!("{} DEEDS", m::DEEDS_FOR[k]), x + w, ty, size * 0.9, rgba(0x6a665e, 1.0), Align::Right, false);
        }
    }
    // The last trial, at the foot.
    let fy = c[1] + c[3] - pad - t * 0.2;
    match g.mastery.last_trial.as_ref().filter(|r| r.hero == hero) {
        Some(r) => {
            let (word, color) = if r.passed { ("PASSED", 0x7ad87a) } else { ("FAILED", 0xff8a6a) };
            let why = if r.passed || r.why.is_empty() { String::new() } else { format!(" - {}", r.why) };
            let text = format!("LAST TRIAL ({}): {word}, {}/{}{why}", roman(r.rank as usize), r.count, r.of);
            o.text(&text, x, fy, size * 0.95, rgba(color, 1.0), Align::Left, true);
        }
        None => {
            let hint = match g.trial_ready() {
                Some(tr) => format!("\"{}\"", tr.line),
                None => "Deeds in any fight count. When a rank's are done, the old man sets its trial.".to_string(),
            };
            o.text(&hint, x, fy, size * 0.9, rgba(TEXT_DIM, 1.0), Align::Left, false);
        }
    }
}
