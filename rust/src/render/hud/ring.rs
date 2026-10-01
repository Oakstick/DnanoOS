//! The Sparring Ring's HUD (COMBAT.md §5): health and poise bars over every duelist, the bout's
//! banner, and the frame readout (F5): the hero's state and frame, his buffer, the threat
//! points in use, each foe's state and frame, and the bout's numbers.

use super::*;
use crate::game::sparring::{BoutPhase, Sparring};
use crate::logic::combat::fsm::{PARRY_FROM, PARRY_TO};
use crate::logic::combat::{Fighter, Input, MoveId, Phase, State};
use crate::logic::gym;

/// A fighter's state as the readout shows it: what, and how far through.
fn state_line(f: &Fighter) -> String {
    match f.state {
        State::Neutral => "NEUTRAL".to_string(),
        State::Attack { mv, frame } => {
            let d = mv.data();
            let (name, at, of) = match d.phase(frame) {
                Phase::Startup => ("STARTUP", frame + 1, d.startup),
                Phase::Active => ("ACTIVE", frame - d.startup + 1, d.active),
                Phase::Recovery => ("RECOVERY", frame - d.startup - d.active + 1, d.recovery),
            };
            format!("{} {name} {at}/{of}", d.name)
        }
        State::Guard { frame } if (PARRY_FROM..=PARRY_TO).contains(&frame) => format!("GUARD {frame} PARRY"),
        State::Guard { frame } => format!("GUARD {frame}"),
        State::Evade { frame } => format!("EVADE {frame}"),
        State::Reel { kind, left, total } => format!("{kind:?} {}/{total}", total - left + 1).to_uppercase(),
        State::Dead => "DOWN".to_string(),
    }
}

/// Everything the ring draws over the view.
pub(super) fn draw_ring(o: &mut Overlay, g: &Game, t: f32) {
    let Some(spar) = g.spar.as_ref() else { return };
    if g.screen != Screen::Playing {
        return;
    }
    draw_bars(o, g, spar, t);
    draw_banner(o, g, spar, t);
    if spar.readout && g.in_ring() {
        draw_readout(o, g, spar, t);
    }
    if matches!(spar.phase, BoutPhase::Won(_) | BoutPhase::Lost(_)) && g.in_ring() {
        draw_results(o, spar, t);
    }
}

/// The results card, through the aftermath of a bout: its time against the target, what he took,
/// and how he met what came at him.
fn draw_results(o: &mut Overlay, spar: &Sparring, t: f32) {
    let won = matches!(spar.phase, BoutPhase::Won(_));
    let s = &spar.stats;
    let bout = gym::BOUTS[spar.bout];
    let secs = s.frames as f64 / 60.0;
    let on_target = secs >= bout.target.0 && secs <= bout.target.1;
    let (w, h) = (t * 17.0, t * 8.6);
    let (x, y) = ((o.width - w) / 2.0, o.height * 0.7 - h / 2.0);
    panel(o, x, y, w, h, t);
    let cx = o.width / 2.0;
    let (head, color) = if won { ("BOUT WON", GOLD) } else { ("DEFEATED", 0xff6a4a) };
    o.text(head, cx, y + t * 1.0, t * 0.85, rgba(color, 1.0), Align::Center, true);
    o.text(&spar.title(), cx, y + t * 1.75, t * 0.45, rgba(SAND, 0.95), Align::Center, false);
    let time_color = if !won {
        TEXT_DIM
    } else if on_target {
        0x7ad87a
    } else {
        0xffb050
    };
    let rows: [(String, String, u32); 6] = [
        ("TIME".into(), format!("{secs:.1} s   (target {:.0}-{:.0} s)", bout.target.0, bout.target.1), time_color),
        ("HEALTH LOST".into(), format!("{:.0}   in {} blows", s.taken, s.hurt), if s.taken <= 30.0 { 0x7ad87a } else { TEXT }),
        ("DEALT".into(), format!("{:.0}   in {} blows,  {} poise breaks", s.dealt, s.hits, s.breaks), TEXT),
        ("MET".into(), format!("{} parried,  {} blocked,  {} evaded", s.parries, s.blocks, s.evades), TEXT),
        (
            "MISTAKES".into(),
            format!("{} caught mid-swing,  {} guards broken", s.caught, s.guard_breaks),
            if s.caught + s.guard_breaks == 0 { 0x7ad87a } else { 0xffb050 },
        ),
        ("OVERLAPS".into(), format!("{}", s.overlaps), if s.overlaps == 0 { 0x7ad87a } else { 0xff4a3a }),
    ];
    let size = t * 0.46;
    let mut ry = y + t * 2.9;
    for (label, value, c) in rows {
        o.text(&label, x + t * 0.8, ry, size, rgba(GOLD, 0.85), Align::Left, true);
        o.text(&value, x + t * 5.0, ry, size, rgba(c, 0.95), Align::Left, false);
        ry += t * 0.82;
    }
}

/// Health and poise over each duelist; its state under them with the readout on.
fn draw_bars(o: &mut Overlay, g: &Game, spar: &Sparring, t: f32) {
    let (w, h) = (o.width, o.height);
    for e in &g.enemies {
        let Some(d) = e.duel.as_deref() else { continue };
        if !d.alive() {
            continue;
        }
        let top = match d.arch {
            crate::logic::combat::Archetype::Brute => 1.55,
            crate::logic::combat::Archetype::Skirmisher => 1.2,
            crate::logic::combat::Archetype::Hexer => 1.85,
        };
        let p = Vec3::new(d.pos.x as f32, top, d.pos.y as f32);
        let Some((x, y)) = g.view.project(p, w, h) else { continue };
        let st = d.arch.stats();
        let bw = t * if st.hp > 100.0 { 3.0 } else { 2.0 };
        let bh = (t * 0.2).max(4.0);
        bar(o, x - bw / 2.0, y, bw, bh, (d.hp / st.hp) as f32, 0xe04a3a);
        // Poise: gold, white while its resolve holds it unbreakable.
        let pcolor = if d.f.resolve > 0 { 0xf0f0f0 } else { 0xf2c53d };
        bar(o, x - bw / 2.0, y + bh + 2.0, bw, (bh * 0.6).max(3.0), (d.f.poise / st.poise) as f32, pcolor);
        if spar.readout {
            let line = state_line(&d.f);
            let color = match d.f.attack() {
                Some((_, Phase::Startup, _)) => 0xff9a5a,
                Some((_, Phase::Active, _)) => 0xff4a3a,
                Some((_, Phase::Recovery, _)) => 0x7ad87a,
                None if d.f.vulnerable() => 0xffd040,
                None => TEXT_DIM,
            };
            o.text(&line, x, y + bh * 2.6 + t * 0.35, t * 0.42, rgba(color, 0.95), Align::Center, true);
        }
    }
}

/// The bout's name and its phase across the top.
fn draw_banner(o: &mut Overlay, g: &Game, spar: &Sparring, t: f32) {
    if !g.in_ring() {
        return;
    }
    let w = o.width;
    let y = PANEL_ROWS * t + t * 1.4;
    let (sub, color) = match spar.phase {
        BoutPhase::Idle => return,
        BoutPhase::Countdown(left) => (format!("GET READY  {}", left.ceil() as u32), SAND),
        BoutPhase::Fight => {
            let alive = g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.alive()).count();
            (format!("{alive} LEFT  {:.1}s", spar.stats.frames as f64 / 60.0), TEXT_DIM)
        }
        BoutPhase::Won(left) => (format!("THE NEXT BOUT IN {}", left.ceil() as u32), GOLD),
        BoutPhase::Lost(left) => (format!("AGAIN IN {}", left.ceil() as u32), 0xff6a4a),
    };
    o.text(&spar.title(), w / 2.0, y, t * 0.75, rgba(GOLD, 0.95), Align::Center, true);
    o.text(&sub, w / 2.0, y + t * 0.8, t * 0.48, rgba(color, 0.95), Align::Center, false);
}

/// The frame readout, down the left side.
fn draw_readout(o: &mut Overlay, g: &Game, spar: &Sparring, t: f32) {
    let x = t * 0.6;
    let mut y = PANEL_ROWS * t + t * 1.6;
    let size = t * 0.46;
    let line = t * 0.62;
    let mut rows: Vec<(String, u32)> = vec![
        ("FRAME READOUT  (F5)".to_string(), GOLD),
        (format!("HERO  {}", state_line(&spar.hero)), TEXT),
        (
            format!(
                "BUFFER  {}",
                match spar.hero.buffered() {
                    Some(Input::Attack) => "ATTACK",
                    Some(Input::Evade) => "EVADE",
                    None => "-",
                }
            ),
            TEXT_DIM,
        ),
        (format!("STAMINA  {:.0}   COMBO ON HIM  {}", g.hero.stamina, spar.hero.combo), TEXT_DIM),
        (format!("THREAT  {}/{}", spar.threat.used(), spar.threat.capacity), 0xff7a5a),
    ];
    let s = &spar.stats;
    rows.push((format!("DEALT {:.0}  TAKEN {:.0}  HITS {}", s.dealt, s.taken, s.hits), TEXT_DIM));
    rows.push((format!("PARRY {}  BLOCK {}  EVADE {}  BREAK {}", s.parries, s.blocks, s.evades, s.breaks), TEXT_DIM));
    rows.push((format!("OVERLAPS {}", s.overlaps), if s.overlaps == 0 { 0x7ad87a } else { 0xff4a3a }));
    if let Some((bout, last)) = spar.last {
        rows.push((format!("LAST WIN  {}  {:.1}s  -{:.0} HP", gym::BOUTS[bout].name, last.frames as f64 / 60.0, last.taken), GOLD));
    }
    rows.push(("F6  NEXT BOUT".to_string(), TEXT_DIM));
    let l1 = MoveId::L1.data();
    rows.push((format!("L1 {}/{}/{}  PARRY F{PARRY_FROM}-{PARRY_TO}", l1.startup, l1.active, l1.recovery), TEXT_DIM));
    let width = rows.iter().map(|r| o.measure(&r.0, size)).fold(0.0, f32::max) + t * 0.8;
    o.rect(x - t * 0.3, y - line * 0.75, width, line * rows.len() as f32 + t * 0.3, [0.0, 0.0, 0.0, 0.55]);
    for (text, color) in rows {
        o.text(&text, x, y, size, rgba(color, 0.95), Align::Left, false);
        y += line;
    }
}
