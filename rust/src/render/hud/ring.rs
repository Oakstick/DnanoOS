//! The brawl's HUD (COMBAT.md §5, §9): health and poise bars over every duelist in the fight (in
//! the ring and the vaults), the ring's bout banner and results card, and the frame readout (F8):
//! the hero's state and frame, his buffer, the threat points in use, each foe's state and frame,
//! his kit, and the fight's numbers.

use super::*;
use crate::game::brawl::{Brawl, Field};
use crate::game::sparring::{BoutPhase, Sparring};
use crate::logic::combat::fsm::PARRY_FROM;
use crate::logic::combat::{Fighter, Input, Phase, State};
use crate::logic::gym;

/// A fighter's state as the readout shows it: what, and how far through.
fn state_line(f: &Fighter) -> String {
    match f.state {
        State::Neutral => "NEUTRAL".to_string(),
        State::Attack { mv, frame } => {
            let d = f.data(mv);
            let (name, at, of) = match d.phase(frame) {
                Phase::Startup => ("STARTUP", frame + 1, d.startup),
                Phase::Active => ("ACTIVE", frame - d.startup + 1, d.active),
                Phase::Recovery => ("RECOVERY", frame - d.startup - d.active + 1, d.recovery),
            };
            format!("{} {name} {at}/{of}", d.name)
        }
        State::Guard { frame } if (PARRY_FROM..=f.parry_to).contains(&frame) => format!("GUARD {frame} PARRY"),
        State::Guard { frame } => format!("GUARD {frame}"),
        State::Evade { frame } => format!("EVADE {frame}"),
        State::Reel { kind, left, total } => format!("{kind:?} {}/{total}", total - left + 1).to_uppercase(),
        State::Dead => "DOWN".to_string(),
    }
}

/// Everything the brawl draws over the view.
pub(super) fn draw_ring(o: &mut Overlay, g: &Game, t: f32) {
    let Some(brawl) = g.brawl.as_ref() else { return };
    if g.screen != Screen::Playing {
        return;
    }
    draw_bars(o, g, brawl, t);
    if g.brawling() {
        draw_ails(o, brawl, t);
    }
    if let Some(spar) = g.spar.as_ref().filter(|_| g.in_ring()) {
        draw_banner(o, g, spar, brawl, t);
        if matches!(spar.phase, BoutPhase::Won(_) | BoutPhase::Lost(_)) {
            if g.in_trial() {
                super::mastery::draw_trial_results(o, g, t);
            } else {
                draw_results(o, spar, brawl, t);
            }
        }
    }
    if brawl.readout && g.brawling() {
        draw_readout(o, g, brawl, g.spar.as_ref(), t);
    }
}

/// The results card, through the aftermath of a bout: its time against the target, what he took,
/// and how he met what came at him.
fn draw_results(o: &mut Overlay, spar: &Sparring, brawl: &Brawl, t: f32) {
    let won = matches!(spar.phase, BoutPhase::Won(_));
    let s = &brawl.stats;
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
fn draw_bars(o: &mut Overlay, g: &Game, brawl: &Brawl, t: f32) {
    let (w, h) = (o.width, o.height);
    for e in &g.enemies {
        let Some(d) = e.duel.as_deref() else { continue };
        // In the vaults only those in the fight (or hurt) wear their bars.
        // (A prince in his court wears the boss bar at the top instead.)
        let prince = d.arch == crate::logic::combat::Archetype::Prince && g.in_court();
        if !d.alive() || prince || (brawl.field == Field::Vault && !d.engaged() && d.hp >= d.max_hp) {
            continue;
        }
        let top = crate::render::view::crown(d.breed.kind).map(|c| c + 0.3).unwrap_or(match d.arch {
            crate::logic::combat::Archetype::Brute => 1.55,
            crate::logic::combat::Archetype::Skirmisher => 1.2,
            crate::logic::combat::Archetype::Hexer => 1.85,
            crate::logic::combat::Archetype::Prince => 3.4,
        });
        let p = Vec3::new(d.pos.x as f32, top, d.pos.y as f32);
        let Some((x, y)) = g.view.project(p, w, h) else { continue };
        let st = d.stats();
        let bw = t * if st.hp > 100.0 { 3.0 } else { 2.0 };
        let bh = (t * 0.2).max(4.0);
        bar(o, x - bw / 2.0, y, bw, bh, (d.hp / d.max_hp) as f32, 0xe04a3a);
        // Poise: gold, white while its resolve holds it unbreakable.
        let pcolor = if d.f.resolve > 0 { 0xf0f0f0 } else { 0xf2c53d };
        bar(o, x - bw / 2.0, y + bh + 2.0, bw, (bh * 0.6).max(3.0), (d.f.poise / st.poise) as f32, pcolor);
        // A pack's leader (WARBAND.md §7): its title and affixes over its bars, and its pack's heart
        // as a thin bar under them.
        if let Some(pack) = g.war.packs.iter().find(|p| p.leader == d.id) {
            // (A rival of the Ledger leads under his own name.)
            let rival = g.rival_of(d.id).and_then(|r| g.ledger().get(r)).map(|r| r.full_name().to_uppercase());
            let mut title = rival.unwrap_or_else(|| String::from("LEADER"));
            for a in &pack.affixes {
                title.push_str("  ");
                title.push_str(a.name());
            }
            o.text(&title, x, y - t * 0.45, t * 0.36, rgba(0xffc040, 0.95), Align::Center, true);
            let heart = (pack.morale / crate::logic::war::pack::MORALE) as f32;
            bar(o, x - bw / 2.0, y + bh * 1.6 + 4.0, bw, 2.0, heart, 0x9ad1ff);
        }
        if brawl.readout {
            // How it attacks (melee, body or distance), and where it is in its frames.
            let line = format!("{}  {}", d.breed.attack().name(), state_line(&d.f));
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

/// What the legends have done to him, low across the middle of the screen: each ail and the
/// seconds left of it.
fn draw_ails(o: &mut Overlay, brawl: &Brawl, t: f32) {
    let a = &brawl.ails;
    let secs = |f: u16| format!("{:.1}S", f as f32 / 60.0);
    let mut parts: Vec<(String, u32)> = Vec::new();
    if a.frost > 0 {
        parts.push((format!("FROSTBITE {}", secs(a.frost)), 0x9ad8ff));
    }
    if a.venom > 0 {
        parts.push((format!("VENOM {}", secs(a.venom)), 0x9ae05a));
    }
    if a.doom > 0 {
        parts.push((format!("DOOMED {}", secs(a.doom)), 0xb08aff));
    }
    if parts.is_empty() {
        return;
    }
    let size = t * 0.5;
    let gap = t * 0.9;
    let width: f32 = parts.iter().map(|(p, _)| o.measure(p, size)).sum::<f32>() + gap * (parts.len() - 1) as f32;
    let (mut x, y) = ((o.width - width) / 2.0, o.height * 0.78);
    o.rect(x - t * 0.4, y - size * 0.7, width + t * 0.8, size * 1.4, [0.0, 0.0, 0.0, 0.5]);
    for (p, c) in parts {
        o.text(&p, x, y, size, rgba(c, 0.95), Align::Left, true);
        x += o.measure(&p, size) + gap;
    }
}

/// The bout's name and its phase across the top.
fn draw_banner(o: &mut Overlay, g: &Game, spar: &Sparring, brawl: &Brawl, t: f32) {
    if !g.in_ring() {
        return;
    }
    let w = o.width;
    let y = super::layout::HudLayout::of(w, o.height).top[1] + t * 0.8;
    let (sub, color) = match spar.phase {
        BoutPhase::Idle => return,
        BoutPhase::Countdown(left) => (format!("GET READY  {}", left.ceil() as u32), SAND),
        BoutPhase::Fight => match super::mastery::trial_line(g) {
            Some(line) => (line, SAND),
            None => {
                let alive = g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.alive()).count();
                (format!("{alive} LEFT  {:.1}s", brawl.stats.frames as f64 / 60.0), TEXT_DIM)
            }
        },
        BoutPhase::Won(left) | BoutPhase::Lost(left) if g.in_trial() => (format!("BACK TO THE OLD MAN IN {}", left.ceil() as u32), SAND),
        BoutPhase::Won(left) => (format!("THE NEXT BOUT IN {}", left.ceil() as u32), GOLD),
        BoutPhase::Lost(left) => (format!("AGAIN IN {}", left.ceil() as u32), 0xff6a4a),
    };
    o.text(&spar.title(), w / 2.0, y, t * 0.75, rgba(GOLD, 0.95), Align::Center, true);
    o.text(&sub, w / 2.0, y + t * 0.8, t * 0.48, rgba(color, 0.95), Align::Center, false);
}

/// The frame readout, down the left side.
fn draw_readout(o: &mut Overlay, g: &Game, spar: &Brawl, ring: Option<&Sparring>, t: f32) {
    let hud = super::layout::HudLayout::of(o.width, o.height);
    let x = hud.safe;
    let mut y = hud.effects[1] + hud.effects[3] + t * 0.6;
    let size = t * 0.46;
    let line = t * 0.62;
    let mut rows: Vec<(String, u32)> = vec![
        ("FRAME READOUT  (F8)".to_string(), GOLD),
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
    if let Some(ring) = ring.filter(|_| spar.field == Field::Ring) {
        if let Some((bout, last)) = ring.last {
            rows.push((format!("LAST WIN  {}  {:.1}s  -{:.0} HP", gym::BOUTS[bout].name, last.frames as f64 / 60.0, last.taken), GOLD));
        }
        rows.push(("F9  NEXT BOUT".to_string(), TEXT_DIM));
    } else {
        let awake = g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.engaged()).count();
        rows.push((format!("KILLS {}  HUNTING HIM {awake}", s.kills), TEXT_DIM));
        // The Drum (WARBAND.md §6): the level's breath and what it holds back.
        let d = &g.war.drum;
        let b = d.bounds();
        rows.push((format!("DRUM  {}  {:.0}s  INTENSITY {:.2}", d.phase.name(), d.age, d.intensity), 0xd8a050));
        rows.push((
            format!("  POOL {:+}  CADENCE +{}  ORDERS {}  VOICES {}", b.capacity, b.cadence, u8::from(b.orders), u8::from(b.voices)),
            TEXT_DIM,
        ));
    }
    // His kit (COMBAT.md §8): the string's frames, its scalars, the guard.
    let k = &spar.kit;
    let [l1, l2, l3] = spar.hero.table;
    rows.push((
        format!(
            "STRING {}/{}/{}  {}/{}/{}  {}/{}/{}",
            l1.startup, l1.active, l1.recovery, l2.startup, l2.active, l2.recovery, l3.startup, l3.active, l3.recovery
        ),
        TEXT_DIM,
    ));
    rows.push((
        format!("TEMPO {:.2}  POWER {:.2}  POISE {:.2}  REACH +{:.1}", k.strike.tempo, k.strike.power, k.strike.poise, k.strike.reach),
        TEXT_DIM,
    ));
    rows.push((
        format!("PARRY F{PARRY_FROM}-{}  BLOCK {:.2}/DMG  DEFLECT {}F", k.guard.parry_to(), k.guard.cost, k.guard.deflect),
        TEXT_DIM,
    ));
    let width = rows.iter().map(|r| o.measure(&r.0, size)).fold(0.0, f32::max) + t * 0.8;
    o.rect(x - t * 0.3, y - line * 0.75, width, line * rows.len() as f32 + t * 0.3, [0.0, 0.0, 0.0, 0.55]);
    for (text, color) in rows {
        o.text(&text, x, y, size, rgba(color, 0.95), Align::Left, false);
        y += line;
    }
}
