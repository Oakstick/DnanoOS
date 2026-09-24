//! Everything drawn flat over the 3D view: the HUD, world labels, and the
//! start, pause, win and game-over screens. `tile` is the layout unit; it
//! scales with the shorter window side so text stays legible from a Steam
//! Deck to 4K.
//!
//! HUD layout: health (portrait, lives, HP, weapon, boons) in the top-left
//! corner, score (gold, nuggets, best) in the top-right corner, bombs and
//! the dungeon name tucked into the bottom-right.

use std::sync::OnceLock;

use glam::Vec3;

use super::overlay::{rgba, Align, Overlay};
use super::view::STEP;
use super::voxels::{sprite_from_model, Sprite, BAT, POPOLON, SKELETON, SLIME};
use crate::game::{Game, MenuItem, Screen};
use crate::logic::items::{weapon_label, RangedKind, WeaponKind};
use crate::logic::systems::MAX_LIVES;
use crate::render::view::floor_at;

const LAYOUT_COLS: f32 = 64.0;
const LAYOUT_ROWS: f32 = 36.0;
const MIN_TILE: f32 = 10.0;
const MAX_TILE: f32 = 28.0;
/// Panel height in tiles.
const PANEL_ROWS: f32 = 3.0;

const TEXT: u32 = 0xe6e0d2;
const TEXT_DIM: u32 = 0x8f8a80;
const SAND: u32 = 0xd7ccb8;
const GOLD: u32 = 0xf2c53d;
const FRENZIED: u32 = 0x3b5bff;
const FRENZIED_WARN: u32 = 0xe8f0ff;

struct Sprites {
    portrait: Sprite,
    cast: Vec<(Sprite, &'static str, u32, &'static str)>,
}

fn sprites() -> &'static Sprites {
    static S: OnceLock<Sprites> = OnceLock::new();
    S.get_or_init(|| Sprites {
        portrait: sprite_from_model(&POPOLON),
        cast: vec![
            (sprite_from_model(&SKELETON), "Skeleton", 0xe9e9f2, "Follows your footsteps. Falls apart, gets up again."),
            (sprite_from_model(&BAT), "Bat", 0x9a5cff, "Fast, and flies over pits and ledges."),
            (sprite_from_model(&SLIME), "Slime", 0x5cff6a, "Slow, but its trail slows you too."),
        ],
    })
}

const RULES: [&str; 5] = [
    "Collect all 25 gold nuggets to win. Three lives; dying loses everything you carry.",
    "The dungeon rises in levels. Jump with Space to clear pits, leap over foes, and climb a ledge; ramps let you walk up. Bats fly over everything.",
    "Enemies hunt you, and more keep arriving from the hollow the longer you take. Every five nuggets the music quickens.",
    "You start with a sword. Strike a shrine to cycle its gift, step on it to take it: a weapon replaces your sword until its ammo runs out.",
    "Green crosses restore 50 health. Potions: red is an extra life, blue a shield, purple invisibility. Bombs blast through walls. Slime trails slow you.",
];

pub struct OverlayLine {
    pub text: String,
    pub size: f32,
    pub color: u32,
}

fn line(text: impl Into<String>, size: f32, color: u32) -> OverlayLine {
    OverlayLine { text: text.into(), size, color }
}

pub fn tile_for(w: f32, h: f32) -> f32 {
    (w / LAYOUT_COLS).min(h / LAYOUT_ROWS).clamp(MIN_TILE, MAX_TILE)
}

/// Builds the whole overlay for this frame.
pub fn draw(o: &mut Overlay, g: &Game, fps: u32) {
    let (w, h) = (o.width, o.height);
    let t = tile_for(w, h);
    if g.screen != Screen::Title {
        draw_world_overlays(o, g, t);
        draw_hud(o, g, t, if g.show_fps { Some(fps) } else { None });
    }
    match g.screen {
        Screen::Title => draw_title(o, g, t),
        Screen::Ready => draw_screen(
            o,
            t,
            &[line(g.level.data.name.to_uppercase(), 1.1, GOLD), line("DESCENDING...", 0.8, SAND)],
            0.35,
        ),
        Screen::Paused => {
            o.rect(0.0, 0.0, o.width, o.height, [2.0 / 255.0, 6.0 / 255.0, 12.0 / 255.0, 0.65]);
            let top = menu_rects(Screen::Paused, 1, o.width, o.height)[0][1];
            o.text("PAUSED", o.width / 2.0, top - t * 2.6, t * 1.45, rgba(TEXT, 1.0), Align::Center, true);
            o.text("Esc or Start to resume", o.width / 2.0, top - t * 1.1, t * 0.6, rgba(SAND, 1.0), Align::Center, false);
            draw_menu(o, g, t);
        }
        Screen::Won => draw_screen(
            o,
            t,
            &[
                line("YOU WON", 2.2, GOLD),
                line("Every nugget in the dungeon is yours.", 0.9, SAND),
                line(format!("Gold {}   Lives left {}", g.score.score, g.score.lives), 0.9, TEXT),
                line(format!("Best {}", g.score.high), 0.8, SAND),
                line("Enter, click or tap to play again", 0.8, TEXT_DIM),
            ],
            0.7,
        ),
        Screen::GameOver => draw_screen(
            o,
            t,
            &[
                line("POPOLON HAS FALLEN", 1.6, 0xd8262e),
                line(format!("Gold {}", g.score.score), 0.9, TEXT),
                line(format!("Best {}", g.score.high), 0.8, SAND),
                line("Enter, click or tap to try again", 0.8, TEXT_DIM),
            ],
            0.65,
        ),
        _ => {}
    }
}

fn bar(o: &mut Overlay, x: f32, y: f32, w: f32, h: f32, frac: f32, color: u32) {
    o.rect(x - 1.0, y - 1.0, w + 2.0, h + 2.0, [0.0, 0.0, 0.0, 0.6]);
    o.rect(x, y, w, h, [1.0, 1.0, 1.0, 0.12]);
    o.rect(x, y, w * frac.clamp(0.0, 1.0), h, rgba(color, 1.0));
}

fn hp_color(frac: f32) -> u32 {
    if frac > 0.5 {
        0x5cff6a
    } else if frac > 0.25 {
        GOLD
    } else {
        0xd8262e
    }
}

fn draw_sprite(o: &mut Overlay, s: &Sprite, x: f32, y: f32, scale: f32, tint: Option<u32>, alpha: f32) {
    for (py, row) in s.pixels.iter().enumerate() {
        for (px, c) in row.iter().enumerate() {
            if let Some(c) = c {
                let color = rgba(tint.unwrap_or(*c), alpha);
                o.rect(x + px as f32 * scale, y + py as f32 * scale, scale + 0.5, scale + 0.5, color);
            }
        }
    }
}

// ---------------------------------------------------------------------- HUD

fn draw_hud(o: &mut Overlay, g: &Game, t: f32, fps: Option<u32>) {
    let w = o.width;
    let h = o.height;
    let hero = &g.hero;
    let fs = (t * 0.6).max(10.0);
    let small = (t * 0.45).max(9.0);
    let margin = t * 0.4;
    let band = PANEL_ROWS * t;
    let panel_bg = [2.0 / 255.0, 6.0 / 255.0, 12.0 / 255.0, 0.72];
    let sp = sprites();
    let time = g.time as f32;

    // ---- top left: health
    let portrait_h = band - t * 0.4;
    let scale = portrait_h / sp.portrait.height as f32;
    let portrait_w = sp.portrait.width as f32 * scale;
    let px0 = margin + t * 0.35;
    let py0 = margin + t * 0.2;
    let cx0 = px0 + portrait_w + t * 0.7;
    let left_w = (cx0 - margin) + t * 9.6;
    o.rect(margin, margin, left_w, band, panel_bg);

    o.rect(px0 - t * 0.15, py0 - t * 0.1, portrait_w + t * 0.3, portrait_h + t * 0.2, [1.0, 1.0, 1.0, 0.05]);
    let ghost = hero.invisible();
    let golden = hero.invincible() && ((time * 6.0).floor() as i64).rem_euclid(2) == 0;
    draw_sprite(o, &sp.portrait, px0, py0, scale, if golden { Some(0xffd84a) } else { None }, if ghost { 0.4 } else { 1.0 });
    if hero.shield > 0 {
        o.rect_outline(px0 - t * 0.15, py0 - t * 0.1, portrait_w + t * 0.3, portrait_h + t * 0.2, (t * 0.08).max(1.0), [127.0 / 255.0, 215.0 / 255.0, 1.0, 0.9]);
    }

    let row_a = margin + t * 0.5;
    let row_b = margin + t * 1.2;
    let row_c = margin + t * 1.95;
    let row_d = margin + t * 2.65;

    o.text("POPOLON", cx0, row_a, small, rgba(TEXT_DIM, 1.0), Align::Left, true);
    let mut hx = cx0 + o.measure("POPOLON", small) + t * 0.5;
    for i in 0..MAX_LIVES {
        let color = if i < g.score.lives { rgba(0xff4d6d, 1.0) } else { [1.0, 1.0, 1.0, 0.18] };
        o.text("♥", hx, row_a, fs, color, Align::Left, false);
        hx += t * 0.7;
    }

    let frac = hero.hp as f32 / hero.max_hp as f32;
    bar(o, cx0, row_b - t * 0.18, t * 6.0, t * 0.36, frac, hp_color(frac));
    o.text(&format!("{}/{}", hero.hp, hero.max_hp), cx0 + t * 6.3, row_b, small, rgba(TEXT, 1.0), Align::Left, true);

    let (wname, wcolor) = weapon_label(hero.weapon);
    draw_weapon_icon(o, hero.weapon, cx0 + t * 0.35, row_c, t * 0.5, wcolor);
    let weapon_text = match hero.weapon {
        WeaponKind::Sword => "SWORD".to_string(),
        WeaponKind::Ranged(_) => format!("{} x{}", wname.to_uppercase(), hero.ammo),
    };
    o.text(&weapon_text, cx0 + t * 0.9, row_c, small, rgba(wcolor, 1.0), Align::Left, true);

    let mut boons: Vec<(String, u32)> = Vec::new();
    if hero.shield > 0 {
        boons.push((format!("SHIELD {}", hero.shield), 0x7fd7ff));
    }
    if hero.invincible() {
        boons.push((format!("INVINCIBLE {}s", hero.invincible_timer.ceil()), 0xffd84a));
    }
    if hero.invisible() {
        boons.push((format!("INVISIBLE {}s", hero.invisible_timer.ceil()), 0xb06cff));
    }
    if hero.hasted() {
        boons.push((format!("SPEED {}s", hero.speed_timer.ceil()), 0x7dff9a));
    }
    if hero.slowed() {
        boons.push(("SLOWED".to_string(), 0x5cff6a));
    }
    let mut bx = cx0;
    if boons.is_empty() {
        o.text("NO POTION", bx, row_d, small, rgba(TEXT_DIM, 1.0), Align::Left, true);
    }
    for (text, color) in &boons {
        o.rect(bx, row_d - t * 0.18, t * 0.3, t * 0.36, rgba(*color, 1.0));
        o.text(text, bx + t * 0.45, row_d, small, rgba(*color, 1.0), Align::Left, true);
        bx += o.measure(text, small) + t * 1.1;
    }

    // ---- top right: score
    let right_w = t * 8.2;
    let rx = w - margin - right_w;
    o.rect(rx, margin, right_w, band, panel_bg);
    let rc = rx + right_w - t * 0.5;
    o.text("GOLD", rc, row_a, small, rgba(TEXT_DIM, 1.0), Align::Right, true);
    o.text(&format!("{:06}", g.score.score), rc, row_b, fs * 1.25, rgba(TEXT, 1.0), Align::Right, true);
    o.text(&format!("NUGGETS {}/{}", g.nuggets_got, g.nuggets_total), rc, row_c, small, rgba(GOLD, 1.0), Align::Right, true);
    o.text(&format!("BEST {:06}", g.score.high), rc, row_d, small, rgba(TEXT_DIM, 1.0), Align::Right, true);

    // ---- bottom right: bombs and where you are
    let bh = t * 1.9;
    let by0 = h - margin - bh;
    let name = g.level.data.name.to_uppercase();
    let bw = (o.measure(&name, small) + t * 1.0).max(t * 6.0);
    o.rect(w - margin - bw, by0, bw, bh, panel_bg);
    let br = w - margin - t * 0.5;
    o.text(&name, br, by0 + t * 0.5, small, rgba(TEXT_DIM, 1.0), Align::Right, true);
    let mut x = br;
    let brow = by0 + t * 1.3;
    for _ in 0..g.bombs_left {
        o.rect(x - t * 0.44, brow - t * 0.22, t * 0.44, t * 0.44, rgba(0xc8262e, 1.0));
        o.rect(x - t * 0.44, brow - t * 0.06, t * 0.44, t * 0.12, rgba(0xf4f4f8, 1.0));
        x -= t * 0.6;
    }
    let label = if g.bombs_left > 0 { "BOMBS" } else { "NO BOMBS" };
    o.text(label, x - t * 0.2, brow, small, rgba(TEXT_DIM, 1.0), Align::Right, true);

    if let Some(fps) = fps {
        o.text(&format!("{fps} fps"), w / 2.0, h - margin - t * 0.4, small, rgba(0x99ffff, 1.0), Align::Center, true);
    }

    // Frenzy window, centered along the top edge.
    if g.frenzy.active() {
        let bw = w * 0.3;
        let bxx = w / 2.0 - bw / 2.0;
        let byy = margin + t * 0.1;
        o.rect(bxx, byy, bw, t * 0.1, [1.0, 1.0, 1.0, 0.1]);
        let color = if g.frenzy.warning() { FRENZIED_WARN } else { FRENZIED };
        o.rect(bxx, byy, bw * g.frenzy.fraction() as f32, t * 0.1, rgba(color, 1.0));
    }
}

/// Small glyph for the held weapon, centered at (cx, cy).
fn draw_weapon_icon(o: &mut Overlay, kind: WeaponKind, cx: f32, cy: f32, size: f32, color: u32) {
    let c = rgba(color, 1.0);
    let lw = (size * 0.18).max(1.0);
    let h = size / 2.0;
    let pi = std::f32::consts::PI;
    match kind {
        WeaponKind::Sword => {
            o.line([cx, cy + h], [cx, cy - h], lw, c);
            o.line([cx - h * 0.6, cy + h * 0.35], [cx + h * 0.6, cy + h * 0.35], lw, c);
        }
        WeaponKind::Ranged(RangedKind::Arrow) => {
            o.line([cx, cy + h], [cx, cy - h], lw, c);
            o.line([cx - h * 0.5, cy - h * 0.4], [cx, cy - h], lw, c);
            o.line([cx, cy - h], [cx + h * 0.5, cy - h * 0.4], lw, c);
        }
        WeaponKind::Ranged(RangedKind::Double) => {
            for dx in [-h * 0.4, h * 0.4] {
                o.line([cx + dx, cy + h], [cx + dx, cy - h], lw, c);
            }
        }
        WeaponKind::Ranged(RangedKind::Trishot) => {
            o.line([cx, cy + h * 0.6], [cx, cy - h], lw, c);
            o.line([cx, cy + h * 0.6], [cx - h, cy + h * 0.2], lw, c);
            o.line([cx, cy + h * 0.6], [cx + h, cy + h * 0.2], lw, c);
        }
        WeaponKind::Ranged(RangedKind::Boomerang) => {
            o.line([cx - h, cy - h * 0.6], [cx - h * 0.2, cy + h * 0.7], lw, c);
            o.line([cx - h * 0.2, cy + h * 0.7], [cx + h, cy - h * 0.2], lw, c);
        }
        WeaponKind::Ranged(RangedKind::Fireball) => o.circle(cx, cy, h * 0.7, c),
        WeaponKind::Ranged(RangedKind::Crescent) => o.arc(cx, cy, h * 0.9, pi * 1.1, pi * 1.9, lw, c),
    }
}

// ------------------------------------------------------------- world labels

/// Health bar over Popolon, shrine labels, and floating text projected onto the overlay.
fn draw_world_overlays(o: &mut Overlay, g: &Game, t: f32) {
    let (w, h) = (o.width, o.height);
    let grid = &g.level.grid;
    let view = &g.view;
    if g.hero.alive && g.screen == Screen::Playing {
        let hp = g.hero.pos();
        let p = Vec3::new(hp.x as f32, g.hero.altitude as f32 * STEP + 1.45, hp.y as f32);
        if let Some((x, y)) = view.project(p, w, h) {
            let bw = t * 1.6;
            let bh = (t * 0.18).max(4.0);
            let frac = g.hero.hp as f32 / g.hero.max_hp as f32;
            bar(o, x - bw / 2.0, y, bw, bh, frac, hp_color(frac));
        }
    }
    for d in &g.diamonds {
        if !d.active {
            continue;
        }
        let (tx, ty) = (d.tile.x as f32, d.tile.y as f32);
        if let Some((x, y)) = view.project(Vec3::new(tx, floor_at(grid, tx, ty) + 1.25, ty), w, h) {
            let offer = d.current();
            o.text(&offer.name().to_uppercase(), x, y, t * 0.45, rgba(offer.color(), 0.95), Align::Center, true);
        }
    }
    for f in &g.floaters {
        let (fx, fy) = (f.x as f32, f.y as f32);
        let lift = floor_at(grid, fx, fy) + 0.9 + (1.0 - f.life as f32) * 0.6;
        if let Some((x, y)) = view.project(Vec3::new(fx, lift, fy), w, h) {
            let alpha = (f.life as f32 * 2.0).min(1.0);
            o.text(&f.text, x, y, t * 0.55, rgba(f.color.unwrap_or(GOLD), alpha), Align::Center, true);
        }
    }
}

// ------------------------------------------------------------------ screens

/// Button rectangles `[x, y, w, h]` for a menu, in logical pixels. The title
/// menu is a row along the bottom; the pause menu is a centered column.
pub fn menu_rects(screen: Screen, count: usize, w: f32, h: f32) -> Vec<[f32; 4]> {
    let t = tile_for(w, h);
    let bh = t * 1.3;
    match screen {
        Screen::Title => {
            let bw = t * 6.0;
            let gap = t * 0.8;
            let total = bw * count as f32 + gap * (count.saturating_sub(1)) as f32;
            let x0 = w / 2.0 - total / 2.0;
            (0..count).map(|i| [x0 + i as f32 * (bw + gap), h * 0.95 - bh / 2.0, bw, bh]).collect()
        }
        _ => {
            let bw = t * 9.0;
            let gap = t * 0.45;
            let y0 = h / 2.0 - t * 0.6;
            (0..count).map(|i| [w / 2.0 - bw / 2.0, y0 + i as f32 * (bh + gap), bw, bh]).collect()
        }
    }
}

/// The current screen's buttons, with the highlighted one outlined in gold.
fn draw_menu(o: &mut Overlay, g: &Game, t: f32) {
    let items = g.menu_items();
    let rects = menu_rects(g.screen, items.len(), o.width, o.height);
    let pulse = 0.75 + (g.time as f32 * 4.0).sin() * 0.25;
    for (i, (item, r)) in items.iter().zip(rects).enumerate() {
        let selected = i == g.menu_index;
        let quit = matches!(item, MenuItem::Quit | MenuItem::QuitToTitle);
        let bg = if selected { [1.0, 1.0, 1.0, 0.14] } else { [1.0, 1.0, 1.0, 0.05] };
        o.rect(r[0], r[1], r[2], r[3], [2.0 / 255.0, 6.0 / 255.0, 12.0 / 255.0, 0.85]);
        o.rect(r[0], r[1], r[2], r[3], bg);
        let edge = if selected { rgba(GOLD, pulse) } else { [1.0, 1.0, 1.0, 0.18] };
        o.rect_outline(r[0], r[1], r[2], r[3], (t * 0.08).max(1.0), edge);
        let color = if selected { if quit { 0xff8a8a } else { GOLD } } else { TEXT_DIM };
        o.text(item.label(), r[0] + r[2] / 2.0, r[1] + r[3] / 2.0, t * 0.7, rgba(color, 1.0), Align::Center, true);
    }
}

/// Centered overlay for ready, pause, win, and game over screens.
fn draw_screen(o: &mut Overlay, t: f32, lines: &[OverlayLine], dim: f32) {
    let (w, h) = (o.width, o.height);
    o.rect(0.0, 0.0, w, h, [2.0 / 255.0, 6.0 / 255.0, 12.0 / 255.0, dim]);
    let total: f32 = lines.iter().map(|l| l.size * t * 1.4).sum();
    let mut y = h / 2.0 - total / 2.0;
    for l in lines {
        let size = l.size * t * 0.9;
        y += size * 0.7;
        o.text(&l.text, w / 2.0, y, size, rgba(l.color, 1.0), Align::Center, true);
        y += size * 0.85;
    }
}

/// Start screen: title, hero shot, the cast, the rules and the controls.
fn draw_title(o: &mut Overlay, g: &Game, t: f32) {
    let (w, h) = (o.width, o.height);
    let time = g.time as f32;
    let sp = sprites();
    o.rect(0.0, 0.0, w, h, [2.0 / 255.0, 6.0 / 255.0, 12.0 / 255.0, 0.88]);

    o.text("VOID HARVEST", w / 2.0, h * 0.09, t * 2.0, rgba(GOLD, 1.0), Align::Center, true);
    o.text("Popolon and the Sunken Dungeon", w / 2.0, h * 0.09 + t * 1.6, t * 0.7, rgba(SAND, 1.0), Align::Center, true);

    let hero_h = (h * 0.34).min(t * 11.0);
    let hero_scale = hero_h / sp.portrait.height as f32;
    let hero_x = w * 0.14 - sp.portrait.width as f32 * hero_scale / 2.0;
    let hero_y = h * 0.22 + (time * 2.0).sin() * t * 0.1;
    draw_sprite(o, &sp.portrait, hero_x, hero_y, hero_scale, None, 1.0);
    o.text("POPOLON", w * 0.14, hero_y + hero_h + t * 0.9, t * 0.8, rgba(0xf2e3c4, 1.0), Align::Center, true);
    o.text("the knight", w * 0.14, hero_y + hero_h + t * 1.7, t * 0.55, rgba(TEXT_DIM, 1.0), Align::Center, false);

    let cast_h = (h * 0.17).min(t * 5.5);
    let cast_y = h * 0.24;
    for (i, (sprite, name, color, blurb)) in sp.cast.iter().enumerate() {
        let cx = w * (0.42 + i as f32 * 0.2);
        let scale = cast_h / sprite.height as f32;
        let x = cx - sprite.width as f32 * scale / 2.0;
        let bob = (time * 3.0 + i as f32).sin() * t * 0.08;
        draw_sprite(o, sprite, x, cast_y + (cast_h - sprite.height as f32 * scale) + bob, scale, None, 1.0);
        o.text(&name.to_uppercase(), cx, cast_y + cast_h + t * 0.9, t * 0.7, rgba(*color, 1.0), Align::Center, true);
        o.wrap_text(blurb, cx, cast_y + cast_h + t * 1.6, w * 0.19, t * 0.62, t * 0.5, rgba(TEXT_DIM, 1.0));
    }

    let mut y = h * 0.6;
    for rule in RULES {
        y = o.wrap_text(rule, w / 2.0, y, w * 0.86, t * 0.75, t * 0.6, rgba(SAND, 1.0)) + t * 0.25;
    }

    let hint = rgba(TEXT_DIM, 1.0);
    o.text("Click to aim with the mouse   Hold W to run, A / D sidestep, S back", w / 2.0, h * 0.81, t * 0.55, hint, Align::Center, false);
    o.text(
        "Left click attack   Right click bomb   Space jump   E attack   B bomb   C camera   Esc pause   M mute   F11 fullscreen",
        w / 2.0,
        h * 0.855,
        t * 0.5,
        hint,
        Align::Center,
        false,
    );
    o.text(
        "Controller: left stick runs, right stick aims, A jump, X attack, B bomb, Start begins or pauses",
        w / 2.0,
        h * 0.9,
        t * 0.55,
        rgba(0x7fd7ff, 1.0),
        Align::Center,
        false,
    );
    draw_menu(o, g, t);
    o.text(&format!("Best {}", g.score.high), w - t * 0.6, h * 0.04, t * 0.5, rgba(TEXT_DIM, 1.0), Align::Right, false);
}
