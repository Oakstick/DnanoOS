//! Play's HUD (UI.md §5), in the layout's corners (`layout::HudLayout`) and the glass of `style`:
//!
//! - **Top left**: the hero's live portrait, his name and lives, health with its trail and flash,
//!   stamina; under it his running effects, each an icon with its time.
//! - **Top right**: the minimap's square (the map builder's), and under it the objective (the
//!   first thing the old man's guide says must be done here) and the purse: the keys or the sin
//!   keys or what the fight wants, silver and runes, the realm's meters. (The score is the arcade
//!   game's: it is shown in the pause menu and when he falls.)
//! - **Top centre**: the boss's bar (the prince's, the Pit's clock and the general's banner are
//!   drawn by their own modules in the same place).
//! - **Bottom centre**: the weapon, the shield and the belt, each with its control's glyph, the
//!   chosen potion lit, cooldowns as radial fills.
//! - **Bottom right**: where he is, small, with the guide's glyph.

use std::cell::Cell;

use super::layout::HudLayout;
use super::style::{self, size};
use super::*;
use crate::logic::boss::BossState;
use crate::render::overlay::Style;

thread_local! {
    /// Where the purse's key row was drawn this frame: the flying nuggets' target.
    static KEY_ROW: Cell<Option<(f32, f32)>> = const { Cell::new(None) };
}

/// Where flying keys land: the keys' row in the purse (as drawn this frame, else where it would be).
pub fn key_target(w: f32, h: f32) -> (f32, f32) {
    KEY_ROW.with(|k| k.get()).unwrap_or_else(|| {
        let l = HudLayout::of(w, h);
        (l.purse[0] + l.purse[2] - l.t * 2.6, l.purse[1] + l.t * 2.2)
    })
}

/// Everything of play's HUD but the old man's dialogue and the archangel (their own modules).
pub(super) fn draw(o: &mut Overlay, g: &Game, fps: Option<u32>) {
    let l = HudLayout::of(o.width, o.height);
    draw_health(o, g, &l);
    draw_effects(o, g, &l);
    draw_purse(o, g, &l);
    draw_loadout(o, g, &l);
    draw_location(o, g, &l);
    draw_boss_bar(o, g, &l);
    if g.frenzy.active() {
        // The frenzy's window, a thin bar along the top centre's foot.
        let [x, y, w, h] = l.top;
        let bw = w * 0.8;
        let color = if g.frenzy.warning() { FRENZIED_WARN } else { FRENZIED };
        style::bar(o, [x + (w - bw) / 2.0, y + h - l.t * 0.35, bw, l.t * 0.18], g.frenzy.fraction() as f32, 0.0, color, 1.0);
    }
    if let Some(fps) = fps {
        // The debug readout, over the archangel's corner.
        let (t, x) = (l.t, l.safe);
        let y = l.angel[1] - t * 0.5;
        let small = t * 0.5;
        o.text(&format!("{fps} fps"), x, y, small, rgba(0x99ffff, 1.0), Align::Left, true);
        let n = g.perf_lines.len();
        for (i, line) in g.perf_lines.iter().enumerate() {
            let y = y - (n - i) as f32 * small * 1.25;
            o.text(line, x, y, small * 0.8, rgba(0x99ffff, 0.9), Align::Left, true);
        }
    }
    #[cfg(debug_assertions)]
    if std::env::var_os("VH_HUD_LAYOUT").is_some() {
        // The layout's rectangles, outlined (a debug aid).
        for (_, r) in l.parts() {
            o.rect_outline(r[0], r[1], r[2], r[3], 1.0, [1.0, 0.0, 1.0, 0.6]);
        }
    }
}

// ------------------------------------------------------------------ top left

/// The hero: his live portrait, name, lives, health (its trail, its flash) and stamina.
fn draw_health(o: &mut Overlay, g: &Game, l: &HudLayout) {
    let t = l.t;
    let [x, y, w, h] = l.health;
    let hero = &g.hero;
    let j = &g.juice;
    let time = g.time as f32;
    style::panel_glass(o, [x, y, w, h], t, 1.0);

    // His portrait, made by the renderer once a second (his gear, his breath), on a warm ground.
    let ps = t * 3.2;
    let (px, py) = (x + (h - ps) / 2.0, y + (h - ps) / 2.0);
    o.rect(px, py, ps, ps, [0.0, 0.0, 0.0, 0.45]);
    style::glow(o, px + ps * 0.45, py + ps * 0.42, ps * 0.62, rgba(0xc0884a, 0.28));
    let golden = hero.invincible() && ((time * 6.0).floor() as i64).rem_euclid(2) == 0;
    let hurt_red = j.hurt > 0.5 && ((time * 20.0).floor() as i64).rem_euclid(2) == 0;
    let tint = if golden {
        rgba(0xffd84a, 1.0)
    } else if hurt_red {
        rgba(0xff5a5a, 1.0)
    } else {
        [1.0; 4]
    };
    let tint = [tint[0], tint[1], tint[2], if hero.invisible() { 0.4 } else { 1.0 }];
    let pose = crate::render::portrait::Pose::live(time, 1.0);
    o.portrait(crate::render::portrait::PortraitId::Hero(g.hero_kind), pose, px, py, ps, ps, false, tint);
    let rim = if hero.shield.is_some() && hero.blocking { rgba(0x9ad1ff, 0.95) } else { rgba(COPPER, 0.85) };
    o.rect_outline(px, py, ps, ps, (t * 0.06).max(1.0), rim);

    let cx = px + ps + t * 0.55;
    let right = x + w - t * 0.45;
    // Name and lives.
    let row_a = y + t * 0.78;
    let name = g.hero_kind.name().to_uppercase();
    let nw = o.text_styled(&name, cx, row_a, &style::name(t, 0.66, rgba(SAND, 1.0)));
    let heart = Style::body(t * 0.78, rgba(0xff4d6d, 1.0)).shadow((t * 0.05).max(1.0), (t * 0.05).max(1.0));
    let mut hx = cx + nw + t * 0.5;
    for i in 0..MAX_LIVES {
        let c = if i < g.score.lives { rgba(0xff4d6d, 1.0) } else { [1.0, 1.0, 1.0, 0.18] };
        hx += o.text_styled("♥", hx, row_a, &heart.color(c)) + t * 0.18;
    }
    // Health: the trail of what the last blow cost, a flash round it, the numbers beside.
    let nums = format!("{} / {}", hero.hp, hero.max_hp);
    let ns = style::label(t, size::LABEL, rgba(TEXT, 1.0)).tabular().right();
    let num_w = o.measure_styled(&format!("{} / {}", hero.max_hp, hero.max_hp), &ns);
    let bar_w = (right - cx - num_w - t * 0.35).max(t * 4.0);
    let (bh, by) = (t * 0.62, y + t * 1.55);
    let frac = hero.hp as f32 / hero.max_hp.max(1) as f32;
    let trail = j.hp_trail / hero.max_hp.max(1) as f32;
    let low = frac <= 0.25 && (time * 4.0).sin() > 0.0;
    style::bar(o, [cx, by, bar_w, bh], frac, trail, hp_color(frac), if low { 0.75 } else { 1.0 });
    if j.hp_flash > 0.0 {
        o.rect_outline(cx - 2.0, by - 2.0, bar_w + 4.0, bh + 4.0, (t * 0.1).max(2.0), rgba(j.hp_flash_color, j.hp_flash));
    }
    o.text_styled(&nums, right, by + bh / 2.0, &ns);
    // Stamina: every blow spends it; orange while Forge-Heart makes blows free.
    let sfrac = (hero.stamina / progression::STAMINA_MAX) as f32;
    let scolor = if hero.tireless() {
        0xff9a3a
    } else if sfrac < 0.2 && (time * 6.0).sin() > 0.0 {
        0x4a7aa8
    } else {
        0x7ad8ff
    };
    let sy = by + bh + t * 0.36;
    style::bar(o, [cx, sy, bar_w, (t * 0.24).max(4.0)], if hero.tireless() { 1.0 } else { sfrac }, 0.0, scolor, 1.0);
    let stamina = style::label(t, 0.5, rgba(TEXT_DIM, 0.9));
    o.text_styled("STAMINA", cx + bar_w + t * 0.35, sy + t * 0.12, &stamina);
}

/// His running effects under the health panel: each an icon in a small glass, a bar draining
/// under it and its seconds; then a slowing, and god mode (F5).
fn draw_effects(o: &mut Overlay, g: &Game, l: &HudLayout) {
    let t = l.t;
    let hero = &g.hero;
    let hk = g.hero_kind;
    let fx = &hero.fx;
    // (Icon, time left, its whole, colour, the figure under it.)
    let mut effects: Vec<(Icon, f64, f64, u32, u32)> = Vec::new();
    let mut add = |kind: PotionKind, left: f64| {
        if left > 0.0 {
            let d = gear::potion(hk, kind);
            effects.push((Icon::Potion(hk, kind), left, d.seconds.max(left), d.color, left.ceil() as u32));
        }
    };
    use PotionKind as K;
    add(K::Invulnerability, hero.invincible_timer);
    add(K::Haste, fx.haste.max(hero.speed_timer));
    add(K::Strength, fx.strength);
    add(K::Fury, fx.fury);
    add(K::Fire, fx.fire);
    add(K::Ragnarok, fx.ragnarok);
    add(K::Spirit, fx.spirit);
    if hero.heart > 0.0 {
        let n = progression::node(hk, Tree::Path, 7);
        effects.push((Icon::Node(hk, Tree::Path, 7), hero.heart, progression::path::HEART_SECONDS, n.color, hero.heart.ceil() as u32));
    }
    if hero.bark > 0 {
        // (Bark counts the blows it has left.)
        let d = gear::potion(hk, K::Bark);
        effects.push((Icon::Potion(hk, K::Bark), hero.bark as f64, 50.0, d.color, hero.bark as u32));
    }
    let [x0, y0, w, _] = l.effects;
    let s = t * 1.3;
    let mut x = x0;
    let mark = style::label(t, size::MARK, rgba(TEXT, 1.0)).tabular().centered();
    for (ic, left, total, color, figure) in effects {
        if x + s > x0 + w {
            break;
        }
        style::panel_ruled(o, [x, y0, s, s], t * 0.6, 0.95, color);
        draw_sprite(o, icon(ic), x + s * 0.12, y0 + s * 0.12, s * 0.76 / 16.0, None, 1.0);
        let fr = (left / total.max(0.01)).clamp(0.0, 1.0) as f32;
        style::bar(o, [x + t * 0.08, y0 + s + t * 0.12, s - t * 0.16, (t * 0.12).max(2.0)], fr, 0.0, color, 1.0);
        o.text_styled(&figure.to_string(), x + s / 2.0, y0 + s + t * 0.48, &mark);
        x += s + t * 0.3;
    }
    let pill = style::label(t, size::MARK + 0.04, rgba(0x9dff8a, 1.0));
    let row = y0 + s * 0.5;
    if hero.slowed() {
        x += style::pill(o, "SLOWED", x, row, &pill, [0.0, 0.0, 0.0, 0.6], Some(rgba(0x5cff6a, 0.8))) + t * 0.25;
    }
    if g.immortal {
        style::pill(o, "GOD MODE (F5)", x, row, &pill.color(rgba(0xffb050, 1.0)), [0.0, 0.0, 0.0, 0.6], Some(rgba(0xffb050, 0.8)));
    }
}

// ------------------------------------------------------------------ top right

/// What must be done here, in a line or two: the first thing the old man's guide says, short.
pub fn objective(g: &Game) -> String {
    thread_local! {
        static LAST: std::cell::RefCell<Option<(crate::logic::guide::GuidePlace, HeroKind, String)>> = const { std::cell::RefCell::new(None) };
    }
    let place = g.guide_place();
    LAST.with(|last| {
        let mut last = last.borrow_mut();
        if let Some((p, h, s)) = last.as_ref() {
            if *p == place && *h == g.hero_kind {
                return s.clone();
            }
        }
        let guide = crate::logic::guide::guide(place, g.hero_kind);
        let s = guide.task.first().map(|l| short(&glyphs::words(&g.input, l))).unwrap_or_default();
        *last = Some((place, g.hero_kind, s.clone()));
        s
    })
}

/// A task's first sentence, its lead-in ("As in the garden: ...") dropped and its afterthoughts
/// (after a colon or a semicolon) cut.
pub fn short(task: &str) -> String {
    let first = task.split(". ").next().unwrap_or(task).trim_end_matches('.');
    let first = match first.split_once(": ") {
        Some((lead, rest)) if lead.starts_with("As ") => {
            let mut c = rest.chars();
            c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
        }
        Some((head, _)) => head.to_string(),
        None => first.to_string(),
    };
    first.split("; ").next().unwrap_or(&first).to_string()
}

/// Under the minimap: the objective, then the purse (the keys, or what the fight wants; silver and
/// runes), the realm's meters, and double points while they last.
fn draw_purse(o: &mut Overlay, g: &Game, l: &HudLayout) {
    let t = l.t;
    let [px, py, pw, _] = l.purse;
    let pad = t * 0.5;
    let rc = px + pw - pad;
    let small = t * size::LABEL;
    style::under(
        o,
        |o| {
            // The objective: a gold diamond and a line or two, right-aligned.
            let s = style::body(t, 0.66, rgba(SAND, 1.0)).right();
            let text = objective(g);
            let mut lines = o.wrap_lines(&text, pw - pad * 2.0 - t * 0.5, &s);
            if lines.len() > 2 {
                lines.truncate(2);
                lines[1].push('…');
            }
            let mut y = py + t * 0.7;
            for (i, line) in lines.iter().enumerate() {
                if i == 0 {
                    let lw = o.measure_styled(line, &s);
                    style::diamond(o, rc - lw - t * 0.35, y, t * 0.14, rgba(GOLD, 1.0));
                }
                o.text_styled(line, rc, y, &s);
                y += t * 0.78;
            }
            y += t * -0.05;
            style::divider(o, px + pad, y, pw - pad * 2.0, t, rgba(COPPER, 0.65));
            y += t * 0.62;
            // The keys, or what the fight or the place wants.
            KEY_ROW.with(|k| k.set(Some((rc - t * 1.6, y))));
            y = draw_status(o, g, t, rc, y, small);
            // The purse: silver and rune fragments to spend at the old man's.
            y += t * 0.86;
            draw_coins(o, g, t, rc, y, small);
            y += t * 0.55;
            if g.in_run() {
                y += t * 0.4;
                let n = g.run_meters().len();
                draw_run_meters(o, g, t, rc, y, small);
                y += n.saturating_sub(1) as f32 * small * 1.9 + t * 0.4;
            }
            if g.double_points > 0.0 && g.screen == Screen::Playing {
                y += t * 0.55;
                let pulse = 0.8 + (g.time as f32 * 8.0).sin() * 0.2;
                let secs = g.double_points.ceil() as u32;
                let st = style::label(t, size::LABEL, rgba(0xffd0a0, 1.0)).right();
                o.text_styled(&format!("x2 POINTS  {secs}s"), rc, y, &st);
                let fr = (g.double_points / DOUBLE_POINTS_SECONDS) as f32;
                y += t * 0.45;
                style::bar(o, [px + pad, y, pw - pad * 2.0, t * 0.12], fr, 0.0, 0xff5a3a, pulse);
                y += t * 0.1;
            }
            y + t * 0.35
        },
        |o, bottom| style::panel_glass(o, [px, py, pw, bottom - py], t, 1.0),
    );
}

/// The purse's first row: the sin keys, the fight's state, the hoard, the Pit's riders, or the
/// golden keys found. The y below what it drew.
fn draw_status(o: &mut Overlay, g: &Game, t: f32, rc: f32, ty: f32, small: f32) -> f32 {
    let j = &g.juice;
    let punch = j.key_punch;
    let ncolor = if punch > 0.0 { mix_rgb(GOLD, 0xffffff, punch * 0.7) } else { GOLD };
    let label = |o: &mut Overlay, text: &str, x: f32, color: u32| {
        o.text_styled(text, x, ty, &style::label(t, size::LABEL, rgba(color, 1.0)).right());
    };
    let mut below = ty;
    if g.hub.is_some() {
        // The seven sins' keys: a gem in each one's colour, lit for those he holds.
        let held = SINS.iter().filter(|s| g.sins_cleared & s.bit() != 0).count();
        let gs = t * 0.24;
        let gap = gs * 2.7;
        let x0 = rc - gap * 6.0 - gs;
        label(o, &format!("SIN KEYS {held}/7"), x0 - t * 0.45, 0xff8a5a);
        for (i, sin) in SINS.iter().enumerate() {
            let got = g.sins_cleared & sin.bit() != 0;
            let x = x0 + i as f32 * gap;
            if got {
                style::glow(o, x + gs, ty, gs * 2.6, rgba(sin.color(), 0.45));
            }
            style::disc(o, x + gs, ty, gs, rgba(if got { sin.color() } else { 0x3a3440 }, if got { 1.0 } else { 0.85 }));
        }
    } else if let Some(b) = &g.boss {
        let key = g.realm.map(|s| format!("THE KEY OF {}", s.title())).unwrap_or_else(|| "THE RED KEY".to_string());
        let (text, color) = if b.alive() {
            ("BOSS FIGHT".to_string(), 0xff8a2a)
        } else if g.red_key.is_some() {
            (format!("TAKE {key}"), g.key_color())
        } else {
            (format!("{key}: TO THE GATE"), g.key_color())
        };
        label(o, &text, rc, color);
    } else if g.in_court() || g.in_lair() {
        court::draw_status(o, g, rc, ty, small);
    } else if g.in_vault() {
        // The hoard: what he carries out of the vaults, for Mammon to ask his toll on.
        let cp = j.coin_punch;
        let gold = format!("{}", g.hoard.gold);
        let gs = style::label(t, size::LABEL * 1.2 * (1.0 + 0.3 * cp), rgba(mix_rgb(GOLD, 0xffffff, cp * 0.6), 1.0))
            .weight(0.5)
            .tabular()
            .right();
        let gw = o.text_styled(&gold, rc, ty, &gs);
        label(o, "HOARD", rc - gw - t * 0.3, 0xffc85a);
    } else if g.in_pit() {
        below = ty + small * 1.1;
        pit::draw_status(o, g, rc, ty, below, small);
    } else if !g.in_run() {
        // A row of keys: gold for each one found, shadows for the rest.
        let n = g.keys_total.max(1);
        let ks = t * 0.82;
        let gap = ks * 0.9;
        let x0 = rc - gap * n as f32 + gap - ks;
        label(o, "KEYS", x0 - t * 0.25, ncolor);
        for i in 0..n {
            let got = i < g.keys_shown();
            let pop = if got && i + 1 == g.keys_shown() { 1.0 + 0.5 * punch } else { 1.0 };
            let s = ks * pop;
            let kx = x0 + i as f32 * gap - (s - ks) / 2.0;
            if got {
                style::glow(o, kx + s / 2.0, ty, s * 0.8, rgba(GOLD, 0.25));
            }
            draw_sprite(
                o,
                icon(Icon::Key),
                kx,
                ty - s / 2.0,
                s / 16.0,
                if got { None } else { Some(0x3a3a44) },
                if got { 1.0 } else { 0.8 },
            );
        }
    } else {
        // A realm: its name's sin, its meters below.
        if let Some(sin) = g.realm {
            label(o, &format!("THE REALM OF {}", sin.title()), rc, sin.color());
        }
    }
    if punch > 0.0 {
        let (tx, _) = key_target(o.width, o.height);
        o.circle(tx, ty, t * 0.9 * (1.0 - punch) + t * 0.2, rgba(GOLD, punch * 0.25));
    }
    below
}

/// Silver and runes, right-aligned at `rc`.
fn draw_coins(o: &mut Overlay, g: &Game, t: f32, rc: f32, y: f32, small: f32) {
    let cp = g.juice.coin_punch;
    let figures = style::label(t, size::LABEL * (1.0 + 0.3 * cp), rgba(mix_rgb(GOLD, 0xffffff, cp * 0.6), 1.0)).tabular().right();
    let coins = format!("{}", g.progress.silver);
    let cw = o.text_styled(&coins, rc, y, &figures);
    let ciw = small * 1.25;
    let cx = rc - cw - ciw - t * 0.15;
    draw_sprite(o, icon(Icon::Money(g.hero_kind)), cx, y - ciw / 2.0, ciw / 16.0, None, 1.0);
    let runes = format!("{}", g.progress.runes);
    let rx = cx - t * 0.55;
    let rw = o.text_styled(&runes, rx, y, &style::label(t, size::LABEL, rgba(0x9ad1ff, 1.0)).tabular().right());
    draw_sprite(o, icon(Icon::Gem(g.hero_kind)), rx - rw - ciw - t * 0.12, y - ciw / 2.0, ciw / 16.0, None, 1.0);
}

// ------------------------------------------------------------------ top centre

/**
 * The boss's health across the top centre: its name, a long bar that flashes when struck and
 * leaves a pale trail of what the blow cost, notches at the thirds where it grows angrier.
 */
fn draw_boss_bar(o: &mut Overlay, g: &Game, l: &HudLayout) {
    let Some(b) = &g.boss else { return };
    if matches!(b.state, BossState::Hidden | BossState::Dead) {
        return;
    }
    let t = l.t;
    let [x, y, w, _] = l.top;
    let time = g.time as f32;
    let ph = t * 2.5;
    style::panel_glass(o, [x, y, w, ph], t, 1.0);
    let pad = t * 0.6;
    let who = g.boss_name();
    let (state, state_color) = match b.phase() {
        1 => ("", SAND),
        2 => ("ENRAGED", 0xff8a2a),
        _ => ("FURIOUS", 0xff4d4d),
    };
    o.text_styled(&who, x + pad, y + t * 0.72, &style::name(t, 0.7, rgba(SAND, 1.0)));
    if !state.is_empty() {
        o.text_styled(state, x + w - pad, y + t * 0.72, &style::label(t, size::LABEL, rgba(state_color, 1.0)).right());
    }
    let r = [x + pad, y + t * 1.35, w - pad * 2.0, t * 0.6];
    let frac = b.hp as f32 / b.max_hp.max(1) as f32;
    let trail = g.juice.boss_trail / b.max_hp.max(1) as f32;
    let color = if b.flash > 0.3 {
        0xffffff
    } else if b.dazed() {
        mix_rgb(0xc24bd8, 0xffe066, (time * 8.0).sin() * 0.5 + 0.5)
    } else {
        0xc24bd8
    };
    style::bar(o, r, frac, trail, color, 1.0);
    style::notches(o, r, &[1.0 / 3.0, 2.0 / 3.0], t, [0.0, 0.0, 0.0, 0.85]);
    if b.dazed() {
        let a = 0.7 + (time * 10.0).sin() * 0.3;
        let s = style::label(t, size::LABEL, rgba(0xffe066, a)).centered();
        o.text_styled("DAZED - STRIKE NOW!", x + w / 2.0, y + ph + t * 0.55, &s);
    }
}

// ------------------------------------------------------------------ bottom centre

/// A slot's square: a darker glass, its rule in `rule`.
fn slot(o: &mut Overlay, x: f32, y: f32, s: f32, t: f32, rule: [f32; 4]) {
    o.rect(x, y, s, s, [0.0, 0.0, 0.0, 0.42]);
    style::glow(o, x + s / 2.0, y + s * 0.4, s * 0.55, [1.0, 1.0, 1.0, 0.05]);
    o.rect_outline(x, y, s, s, (t * 0.05).max(1.0), rule);
}

/// A control's glyph tucked over a slot's corner (bottom right), if one is bound on the device.
fn corner_glyph(o: &mut Overlay, g: &Game, action: Action, x: f32, y: f32, s: f32, t: f32) {
    let size = t * 0.58;
    glyphs::draw_prompt_right(o, &g.input, action, x + s + t * 0.12, y + s - size * 0.25, size);
}

/**
 * The bottom-centre panel: the weapon (its tier, its edge, a cooldown sweeping over its icon), the
 * shield (its strength, a rim while raised) and the belt (the chosen potion lit, each slot with
 * its key where one is bound), each with its control's glyph; the chosen potion's name over it.
 */
fn draw_loadout(o: &mut Overlay, g: &Game, l: &HudLayout) {
    let t = l.t;
    let hero = &g.hero;
    let hk = g.hero_kind;
    let time = g.time as f32;
    let [x0, y_top, pw, h] = l.loadout;
    let ph = t * 2.9;
    let y0 = y_top + h - ph;
    style::panel_glass(o, [x0, y0, pw, ph], t, 1.0);
    let label = |units: f32, color: u32| style::label(t, units, rgba(color, 1.0));

    // ---- the weapon
    let (wname, wcolor) = weapon_label(hk, hero.weapon, hero.blade);
    let ic = match hero.weapon {
        WeaponKind::Ranged(r) if hero.holding_pickup() => Icon::Pickup(r),
        _ => Icon::Node(hk, Tree::Armory, hero.blade),
    };
    let s = t * 2.3;
    let (sx, sy) = (x0 + t * 0.3, y0 + (ph - s) / 2.0);
    slot(o, sx, sy, s, t, rgba(COPPER, 0.7));
    let iw = s * 0.82 * if hero.swinging() { 1.1 } else { 1.0 };
    draw_sprite(o, icon(ic), sx + (s - iw) / 2.0, sy + (s - iw) / 2.0, iw / 16.0, None, 1.0);
    let sword = hero.weapon == WeaponKind::Sword;
    let wait = if sword {
        let full = progression::blade(hk, hero.blade).cooldown.max(1e-3);
        (hero.attack_cooldown / full) as f32
    } else {
        let full = match hero.weapon {
            WeaponKind::Ranged(r) => weapon(r).cooldown,
            _ => 1.0,
        }
        .max(1e-3);
        (hero.fire_cooldown / full) as f32
    };
    style::cooldown(o, sx, sy, s, wait.clamp(0.0, 1.0));
    corner_glyph(o, g, Action::Attack, sx, sy, s, t);
    let tx = sx + s + t * 0.45;
    let name_w = t * 6.2;
    let wname = wname.to_uppercase();
    let ws = label(size::LABEL, wcolor);
    let fit = (name_w / o.measure_styled(&wname, &ws).max(1.0)).min(1.0);
    o.text_styled(&wname, tx, y0 + t * 0.72, &ws.size(ws.size * fit.max(0.72)));
    let row2 = y0 + t * 1.5;
    if hero.holding_pickup() {
        let WeaponKind::Ranged(r) = hero.weapon else { unreachable!() };
        let max = weapon(r).ammo;
        let pip = t * 0.28;
        for i in 0..max {
            let c = if i < hero.ammo { rgba(wcolor, 1.0) } else { [1.0, 1.0, 1.0, 0.15] };
            style::diamond(o, tx + pip * 0.6 + i as f32 * (pip + t * 0.16), row2, pip * 0.6, c);
        }
    } else {
        let tier = format!("TIER {}", hero.blade);
        let tw = o.text_styled(&tier, tx, row2, &label(size::LABEL, GOLD));
        let ready = wait <= 0.0;
        let (word, color) = if ready { ("READY", 0x7dff9a) } else { ("...", TEXT_DIM) };
        o.text_styled(word, tx + tw + t * 0.45, row2, &label(size::LABEL, color));
        // The blade's edge: keen gold, dulling amber, rusted red.
        let ec = if hero.edge >= crate::logic::gear::DULL_EDGE {
            0xe8c860
        } else if hero.edge >= crate::logic::gear::RUSTED_EDGE {
            0xd88a3a
        } else {
            0xc84a2a
        };
        style::bar(o, [tx, y0 + t * 2.2, name_w, t * 0.16], hero.edge as f32, 0.0, ec, 1.0);
    }

    // ---- the shield
    let sx0 = x0 + t * 9.7;
    o.rect(sx0, y0 + t * 0.4, (t * 0.05).max(1.0), ph - t * 0.8, rgba(COPPER, 0.35));
    let ss = t * 2.0;
    let (shx, shy) = (sx0 + t * 0.4, y0 + (ph - ss) / 2.0);
    match hero.shield {
        Some(sh) => {
            let def = progression::node(hk, Tree::Shield, sh.tier);
            slot(o, shx, shy, ss, t, if hero.blocking { rgba(0x9ad1ff, 0.95) } else { rgba(COPPER, 0.6) });
            draw_sprite(o, icon(Icon::Node(hk, Tree::Shield, sh.tier)), shx + ss * 0.1, shy + ss * 0.1, ss * 0.8 / 16.0, None, 1.0);
            corner_glyph(o, g, Action::Block, shx, shy, ss, t);
            let bx = shx + ss + t * 0.45;
            let name = def.name.to_uppercase();
            let ns = label(size::LABEL, def.color);
            let fit = (t * 4.6 / o.measure_styled(&name, &ns).max(1.0)).min(1.0);
            o.text_styled(&name, bx, y0 + t * 0.72, &ns.size(ns.size * fit.max(0.7)));
            let frac = sh.fraction();
            let low = frac < 0.3 && (time * 5.0).sin() > 0.0;
            let color = if frac > 0.3 {
                0x9ad1ff
            } else if low {
                0xff4a3a
            } else {
                0xff8a6a
            };
            style::bar(o, [bx, y0 + t * 1.4, t * 4.6, t * 0.34], frac, 0.0, color, 1.0);
            if hero.blocking {
                o.text_styled("RAISED", bx, y0 + t * 2.2, &label(size::MARK + 0.04, 0x9ad1ff));
            }
        }
        None => {
            let p = g.progress.hero(hk);
            if p.is_broken(p.carry) {
                o.text_styled("SHIELD DESTROYED", shx, y0 + t * 1.0, &label(size::LABEL, 0xff8a6a));
                o.text_styled("THE OLD MAN CAN REFORGE IT", shx, y0 + t * 1.8, &label(size::MARK + 0.04, TEXT_DIM));
            } else {
                o.text_styled("NO SHIELD", shx, y0 + ph / 2.0, &label(size::LABEL, TEXT_DIM));
            }
        }
    }

    // ---- the belt
    let bx0 = x0 + t * 17.6;
    o.rect(bx0, y0 + t * 0.4, (t * 0.05).max(1.0), ph - t * 0.8, rgba(COPPER, 0.35));
    let bs = t * 1.55;
    let gap = t * 0.2;
    let by = y0 + (ph - bs) / 2.0;
    let chosen = (g.belt_index < g.belt.len()).then_some(g.belt_index);
    for i in 0..BELT_SLOTS {
        let x = bx0 + t * 0.4 + i as f32 * (bs + gap);
        let sel = chosen == Some(i);
        let lift = if sel { t * 0.08 } else { 0.0 };
        if sel {
            style::glow(o, x + bs / 2.0, by + bs / 2.0, bs * 0.95, rgba(GOLD, 0.35));
        }
        let rule = if sel { rgba(GOLD, 0.95) } else { [EDGE[0], EDGE[1], EDGE[2], 0.32] };
        slot(o, x, by - lift, bs, t, rule);
        if sel {
            o.rect_outline(x, by - lift, bs, bs, (t * 0.08).max(1.5), rgba(GOLD, 0.95));
        }
        if let Some(&k) = g.belt.get(i) {
            let a = if sel || chosen.is_none() { 1.0 } else { 0.8 };
            draw_sprite(o, icon(Icon::Potion(hk, k)), x + bs * 0.1, by - lift + bs * 0.1, bs * 0.8 / 16.0, None, a);
        }
        // Its own key, where the device has one for it (the keyboard's 1 to 5).
        glyphs::draw_prompt(o, &g.input, Action::Slot(i as u8), x - t * 0.08, by - lift + t * 0.08, t * 0.46);
    }
    // The chosen potion's name over the belt, with the control that drinks it; the belt's turn.
    if let Some(&k) = chosen.and_then(|i| g.belt.get(i)) {
        let def = gear::potion(hk, k);
        let st = label(size::LABEL, def.color);
        glyphs::prompt_text(o, &g.input, &format!("{{Drink}} {}", def.name.to_uppercase()), bx0 + t * 0.4, y_top + t * 0.45, &st);
    }
    if g.input.last_device().is_pad() && g.belt.len() > 1 {
        let ry = by + bs / 2.0;
        let end = bx0 + t * 0.4 + BELT_SLOTS as f32 * (bs + gap) - gap;
        glyphs::draw_prompt_right(o, &g.input, Action::PotionPrev, bx0 + t * 0.35, ry, t * 0.5);
        glyphs::draw_prompt(o, &g.input, Action::PotionNext, end.min(x0 + pw - t * 0.5), ry, t * 0.5);
    }
}

// ------------------------------------------------------------------ bottom right

/// Where he is, small, with the guide's glyph: the old man's word on this place.
fn draw_location(o: &mut Overlay, g: &Game, l: &HudLayout) {
    let t = l.t;
    let [x, y, w, h] = l.location;
    let name = g.level.data.name.to_uppercase();
    let base = style::label(t, 0.6, rgba(SAND, 1.0)).right();
    let gs = t * 0.62;
    let glyph = glyphs::measure_prompt_text(o, &g.input, "{Guide}", &base);
    let room = w - glyph - t * 1.3;
    let fit = (room / o.measure_styled(&name, &base).max(1.0)).clamp(0.7, 1.0);
    let st = base.size(base.size * fit);
    let pw = (o.measure_styled(&name, &st) + glyph + t * 1.3).min(w);
    let px = x + w - pw;
    style::panel_glass(o, [px, y, pw, h], t, 0.92);
    let cy = y + h / 2.0;
    o.text_styled(&name, x + w - t * 0.5, cy, &st);
    glyphs::draw_prompt(o, &g.input, Action::Guide, px + t * 0.45, cy, gs);
}

/// The pause screen's summary line: the score (the arcade game's) and the best.
pub(super) fn draw_pause_summary(o: &mut Overlay, g: &Game, t: f32, y: f32) {
    let s = style::label(t, size::LABEL, rgba(TEXT_DIM, 1.0)).tabular().centered();
    o.text_styled(&format!("SCORE {:06}     BEST {:06}", g.score.score, g.score.high), o.width / 2.0, y, &s);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_objective_is_the_guides_first_task_made_short() {
        assert_eq!(
            short("Find the eight golden keys hidden across the garden. Each one lights a lock on the gate in the north wall."),
            "Find the eight golden keys hidden across the garden"
        );
        assert_eq!(
            short("As in the garden: find the eight golden keys, and the gate in the north wall opens."),
            "Find the eight golden keys, and the gate in the north wall opens"
        );
        assert_eq!(short("Go down the gullet: the Hall of Courses, the Kitchens, the Larder, the Stomach."), "Go down the gullet");
        assert_eq!(
            short("Walk to the Black Thorn, the one dead tree that never moves; the court's gate stands at its foot. Walk on."),
            "Walk to the Black Thorn, the one dead tree that never moves"
        );
        assert_eq!(short("Bring Lucifer down. He drops the Key of Pride."), "Bring Lucifer down");
    }

    #[test]
    fn every_place_has_an_objective_of_a_line_or_two() {
        use crate::logic::guide::{guide, GuidePlace};
        use crate::logic::sins::SINS;
        let mut places =
            vec![GuidePlace::Garden, GuidePlace::Cisterns, GuidePlace::BoneVaults, GuidePlace::Hub, GuidePlace::Gym, GuidePlace::Lair];
        places.extend((0..3).map(GuidePlace::Cyclops));
        places.extend(SINS.iter().map(|&s| GuidePlace::Realm(s)));
        places.extend(SINS.iter().map(|&s| GuidePlace::Court(s)));
        places.extend((1..=7).map(GuidePlace::Pit));
        for p in places {
            let g = guide(p, HeroKind::Ragnar);
            let s = short(g.task.first().expect("a task"));
            assert!(s.len() >= 8 && s.len() <= 110, "{p:?}: {s:?}");
        }
    }
}
