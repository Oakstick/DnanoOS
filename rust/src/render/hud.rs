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
use super::foes::{bog_slime, draugr, raven};
use super::voxels::{sprite_from, Sprite};
use crate::game::{Game, MenuItem, Screen};
use crate::logic::progression::{self, Tree};
use crate::logic::items::{weapon, weapon_label, WeaponKind};
use crate::render::icons::{icon, Icon};

mod shop;
mod story;
pub use shop::{shop_row_at, shop_rects};
use crate::logic::systems::MAX_LIVES;
use crate::render::view::floor_at;

const LAYOUT_COLS: f32 = 64.0;
const LAYOUT_ROWS: f32 = 36.0;
const MIN_TILE: f32 = 12.0;
const MAX_TILE: f32 = 28.0;
/// Panel height in tiles.
const PANEL_ROWS: f32 = 3.0;

const TEXT: u32 = 0xe6e0d2;
const TEXT_DIM: u32 = 0xaaa293;
const SAND: u32 = 0xd7ccb8;
const GOLD: u32 = 0xf2c53d;
const FRENZIED: u32 = 0x3b5bff;
const FRENZIED_WARN: u32 = 0xe8f0ff;

struct Sprites {
    /// Each hero's face in the corner: Ragnar's pixel model, Mehmed and Miyamoto as they are played.
    portraits: [Sprite; 3],
    /// The fountain's statue, for its tour card.
    krampus: Sprite,
    cast: Vec<(Sprite, &'static str, u32, &'static str)>,
}

/// A sprite trimmed to what is drawn in it.
fn cropped(s: Sprite) -> Sprite {
    let filled = |x: i32, y: i32| s.pixels[y as usize][x as usize].is_some();
    let xs: Vec<i32> = (0..s.width).filter(|&x| (0..s.height).any(|y| filled(x, y))).collect();
    let ys: Vec<i32> = (0..s.height).filter(|&y| (0..s.width).any(|x| filled(x, y))).collect();
    let (Some(&x0), Some(&x1), Some(&y0), Some(&y1)) = (xs.first(), xs.last(), ys.first(), ys.last()) else { return s };
    let pixels = (y0..=y1).map(|y| (x0..=x1).map(|x| s.pixels[y as usize][x as usize]).collect()).collect();
    Sprite { width: x1 - x0 + 1, height: y1 - y0 + 1, pixels }
}

/// The hero's face for the HUD corner and the tour.
fn portrait_of(hero: crate::logic::heroes::HeroKind) -> &'static Sprite {
    use crate::logic::heroes::HeroKind;
    &sprites().portraits[match hero {
        HeroKind::Ragnar => 0,
        HeroKind::Mehmed => 1,
        HeroKind::Miyamoto => 2,
    }]
}

fn sprites() -> &'static Sprites {
    static S: OnceLock<Sprites> = OnceLock::new();
    S.get_or_init(|| Sprites {
        portraits: [
            cropped(sprite_from(&super::heroes::ragnar_hd(None))),
            cropped(sprite_from(&super::heroes::mehmed_hd(None))),
            cropped(sprite_from(&super::heroes::miyamoto_hd(None))),
        ],
        krampus: sprite_from(&crate::render::view::krampus()),
        cast: vec![
            (sprite_from(&draugr()), "Draugr", 0xe9e9f2, "An undead viking. Follows your footsteps, falls apart, rises again."),
            (sprite_from(&raven(false)), "Raven", 0xc9a0ff, "Fast, and flies over pits and ledges. Its dark kin hunts in pairs."),
            (sprite_from(&bog_slime()), "Bog Slime", 0x5cff6a, "Slow, but its trail slows you too."),
        ],
    })
}

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
    // Every piece of text gets a drop shadow so it reads over bright floors and lights.
    o.shadow = (t * 0.08).max(1.5);
    if g.screen == Screen::Tour {
        draw_tour(o, g, t);
        return;
    }
    if g.screen == Screen::Shop {
        shop::draw_shop(o, g, t);
        if g.juice.fade > 0.0 {
            o.rect(0.0, 0.0, o.width, o.height, [0.0, 0.0, 0.0, g.juice.fade.min(1.0)]);
        }
        return;
    }
    if g.screen == Screen::Story {
        story::draw_story(o, g, t);
        return;
    }
    if g.screen == Screen::Select {
        draw_select(o, g, t);
        if g.juice.fade > 0.0 {
            o.rect(0.0, 0.0, o.width, o.height, [0.0, 0.0, 0.0, g.juice.fade.min(1.0)]);
        }
        return;
    }
    if g.screen != Screen::Title {
        draw_world_overlays(o, g, t);
        draw_hud(o, g, t, if g.show_fps { Some(fps) } else { None });
        draw_feedback(o, g, t);
    }
    match g.screen {
        Screen::Title => draw_title(o, g, t),
        Screen::Ready => draw_countdown(o, g, t),
        Screen::Paused => {
            o.rect(0.0, 0.0, o.width, o.height, [2.0 / 255.0, 6.0 / 255.0, 12.0 / 255.0, 0.65]);
            let top = menu_rects(Screen::Paused, &[MenuItem::Resume], o.width, o.height)[0][1];
            o.text("PAUSED", o.width / 2.0, top - t * 2.6, t * 1.45, rgba(TEXT, 1.0), Align::Center, true);
            o.text("Esc or Start to resume", o.width / 2.0, top - t * 1.1, t * 0.6, rgba(SAND, 1.0), Align::Center, false);
            draw_menu(o, g, t);
            // Where the story stands: the road down after Krampus, below the last button.
            let rects = menu_rects(Screen::Paused, &g.menu_items(), o.width, o.height);
            let y = rects.last().map(|r| r[1] + r[3]).unwrap_or(o.height / 2.0) + t * 1.4;
            o.text("THE ROAD DOWN", o.width / 2.0, y, t * 0.55, rgba(GOLD, 0.9), Align::Center, true);
            o.text(&crate::logic::story::depth_line(g.dungeon, g.boss.is_some()), o.width / 2.0, y + t * 0.8, t * 0.5, rgba(SAND, 0.9), Align::Center, false);
            o.wrap_text(crate::logic::story::objective(), o.width / 2.0, y + t * 1.7, (t * 26.0).min(o.width - t * 2.0), t * 0.75, t * 0.5, rgba(TEXT_DIM, 1.0));
        }
        Screen::Won => draw_screen(
            o,
            t,
            &[
                line("THE CYCLOPS IS SLAIN", 2.0, GOLD),
                line(format!("The way down lies open. {} goes deeper.", g.hero_kind.name()), 0.9, SAND),
                line(format!("Gold {}   Lives left {}", g.score.score, g.score.lives), 0.9, TEXT),
                line(format!("Best {}", g.score.high), 0.8, SAND),
                line(
                    format!(
                        "+{} {}   +{} {}   and a relic of the old heroes",
                        crate::logic::gear::COINS_BOSS + 100 * g.dungeon as u32,
                        progression::coin(g.hero_kind).to_lowercase(),
                        crate::logic::gear::RUNES_BOSS + g.dungeon as u32,
                        progression::gems(g.hero_kind, 2).to_lowercase()
                    ),
                    0.9,
                    GOLD,
                ),
                line("Enter, click or tap: the old gatekeeper's wares", 0.8, TEXT_DIM),
            ],
            0.7,
        ),
        Screen::GameOver => draw_screen(
            o,
            t,
            &[
                line(format!("{} HAS FALLEN", g.hero_kind.name().to_uppercase()), 1.6, 0xd8262e),
                line(format!("Gold {}", g.score.score), 0.9, TEXT),
                line(format!("Best {}", g.score.high), 0.8, SAND),
                line("Enter, click or tap to try again", 0.8, TEXT_DIM),
            ],
            0.65,
        ),
        Screen::Cleared => {
            // The fanfare's last moments fade to black on the way to the boss.
            let k = (1.0 - g.screen_timer as f32 / 0.9).clamp(0.0, 1.0);
            o.rect(0.0, 0.0, o.width, o.height, [0.0, 0.0, 0.0, k]);
        }
        _ => {}
    }
    if g.juice.fade > 0.0 {
        o.rect(0.0, 0.0, o.width, o.height, [0.0, 0.0, 0.0, g.juice.fade.min(1.0)]);
    }
}

/**
 * The boss's health across the top middle: its name, a long bar that flashes
 * when struck and leaves a pale trail of what the blow cost, and notches at
 * the thirds where it grows angrier.
 */
fn draw_boss_bar(o: &mut Overlay, g: &Game, t: f32) {
    let Some(b) = &g.boss else { return };
    if matches!(b.state, crate::logic::boss::BossState::Hidden | crate::logic::boss::BossState::Dead) {
        return;
    }
    let w = o.width;
    let margin = t * 0.4;
    let bw = (w * 0.34).min(t * 19.0);
    let x0 = w / 2.0 - bw / 2.0;
    let y0 = margin + t * 0.35;
    let time = g.time as f32;
    let name = match b.phase() {
        1 => "THE CYCLOPS",
        2 => "THE CYCLOPS - ENRAGED",
        _ => "THE CYCLOPS - FURIOUS",
    };
    let name_color = match b.phase() {
        1 => SAND,
        2 => 0xff8a2a,
        _ => 0xff4d4d,
    };
    o.text(name, w / 2.0, y0 + t * 0.2, t * 0.62, rgba(name_color, 1.0), Align::Center, true);
    let by = y0 + t * 0.75;
    let bh = t * 0.55;
    let frac = b.hp as f32 / b.max_hp as f32;
    let trail = (g.juice.boss_trail / b.max_hp as f32).clamp(0.0, 1.0);
    o.rect(x0 - 3.0, by - 3.0, bw + 6.0, bh + 6.0, [0.0, 0.0, 0.0, 0.8]);
    o.rect(x0, by, bw, bh, [1.0, 1.0, 1.0, 0.1]);
    o.rect(x0, by, bw * trail, bh, [1.0, 0.95, 0.85, 0.8]);
    let color = if b.flash > 0.3 { 0xffffff } else if b.dazed() { mix_rgb(0xc24bd8, 0xffe066, (time * 8.0).sin() * 0.5 + 0.5) } else { 0xc24bd8 };
    o.rect(x0, by, bw * frac, bh, rgba(color, 1.0));
    o.rect(x0, by, bw * frac, bh * 0.3, [1.0, 1.0, 1.0, 0.2]);
    for k in [1.0f32 / 3.0, 2.0 / 3.0] {
        o.rect(x0 + bw * k - 1.0, by - t * 0.1, 2.0, bh + t * 0.2, [0.0, 0.0, 0.0, 0.85]);
    }
    if b.dazed() {
        o.text("DAZED - STRIKE NOW!", w / 2.0, by + bh + t * 0.55, t * 0.55, rgba(0xffe066, 0.7 + (time * 10.0).sin() * 0.3), Align::Center, true);
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

fn panel(o: &mut Overlay, x: f32, y: f32, w: f32, h: f32, t: f32) {
    o.rect(x, y, w, h, [2.0 / 255.0, 6.0 / 255.0, 12.0 / 255.0, 0.82]);
    o.rect_outline(x, y, w, h, (t * 0.06).max(1.0), [1.0, 1.0, 1.0, 0.14]);
    // A thin gold rule along the top edge ties the panels together.
    o.rect(x + t * 0.3, y, w - t * 0.6, (t * 0.06).max(1.0), rgba(GOLD, 0.35));
}

/// Where flying keys land: the row of keys in the top-right panel.
pub fn key_target(w: f32, h: f32) -> (f32, f32) {
    let t = tile_for(w, h);
    let margin = t * 0.4;
    (w - margin - t * 2.6, margin + t * 1.95)
}

fn draw_hud(o: &mut Overlay, g: &Game, t: f32, fps: Option<u32>) {
    let w = o.width;
    let h = o.height;
    let hero = &g.hero;
    let j = &g.juice;
    let fs = (t * 0.68).max(12.0);
    let small = (t * 0.52).max(11.0);
    let margin = t * 0.4;
    let band = PANEL_ROWS * t;
    let time = g.time as f32;

    // ---- top left: health
    let portrait = portrait_of(g.hero_kind);
    let portrait_h = band - t * 0.4;
    let scale = portrait_h / portrait.height as f32;
    let portrait_w = portrait.width as f32 * scale;
    let px0 = margin + t * 0.35;
    let py0 = margin + t * 0.2;
    let cx0 = px0 + portrait_w + t * 0.7;
    let bar_w = t * 7.0;
    let left_w = (cx0 - margin) + bar_w + t * 3.2;
    panel(o, margin, margin, left_w, band, t);
    o.rect(px0 - t * 0.15, py0 - t * 0.1, portrait_w + t * 0.3, portrait_h + t * 0.2, [1.0, 1.0, 1.0, 0.06]);
    let ghost = hero.invisible();
    let golden = hero.invincible() && ((time * 6.0).floor() as i64).rem_euclid(2) == 0;
    let hurt_red = j.hurt > 0.5 && ((time * 20.0).floor() as i64).rem_euclid(2) == 0;
    let tint = if golden { Some(0xffd84a) } else if hurt_red { Some(0xff5a5a) } else { None };
    draw_sprite(o, portrait, px0, py0, scale, tint, if ghost { 0.4 } else { 1.0 });
    if hero.shield.is_some() {
        o.rect_outline(px0 - t * 0.15, py0 - t * 0.1, portrait_w + t * 0.3, portrait_h + t * 0.2, (t * 0.08).max(1.0), [127.0 / 255.0, 215.0 / 255.0, 1.0, 0.9]);
    }

    let row_a = margin + t * 0.8;
    let row_b = margin + t * 1.85;
    let name = g.hero_kind.name().to_uppercase();
    o.text(&name, cx0, row_a, small, rgba(SAND, 1.0), Align::Left, true);
    let mut hx = cx0 + o.measure(&name, small) + t * 0.6;
    for i in 0..MAX_LIVES {
        let color = if i < g.score.lives { rgba(0xff4d6d, 1.0) } else { [1.0, 1.0, 1.0, 0.2] };
        o.text("♥", hx, row_a, fs * 1.1, color, Align::Left, false);
        hx += t * 0.85;
    }
    // Health bar: a white chunk trails behind a hit so you see how much it cost.
    let frac = hero.hp as f32 / hero.max_hp as f32;
    let trail = (j.hp_trail / hero.max_hp as f32).clamp(0.0, 1.0);
    let bh = t * 0.62;
    let by = row_b - bh / 2.0;
    o.rect(cx0 - 2.0, by - 2.0, bar_w + 4.0, bh + 4.0, [0.0, 0.0, 0.0, 0.75]);
    o.rect(cx0, by, bar_w, bh, [1.0, 1.0, 1.0, 0.1]);
    o.rect(cx0, by, bar_w * trail, bh, [1.0, 1.0, 1.0, 0.75]);
    let low = frac <= 0.25 && ((time * 4.0).sin() > 0.0);
    o.rect(cx0, by, bar_w * frac, bh, rgba(hp_color(frac), if low { 0.7 } else { 1.0 }));
    o.rect(cx0, by, bar_w * frac, bh * 0.3, [1.0, 1.0, 1.0, 0.18]);
    if j.hp_flash > 0.0 {
        o.rect_outline(cx0 - 2.0, by - 2.0, bar_w + 4.0, bh + 4.0, (t * 0.1).max(2.0), rgba(j.hp_flash_color, j.hp_flash));
    }
    o.text(&format!("{} / {}", hero.hp, hero.max_hp), cx0 + bar_w + t * 0.35, row_b, small, rgba(TEXT, 1.0), Align::Left, true);
    // Stamina: every blow spends it; orange while Forge-Heart makes blows free.
    let sfrac = (hero.stamina / progression::STAMINA_MAX) as f32;
    let sy = by + bh + t * 0.22;
    let sh = (t * 0.22).max(4.0);
    o.rect(cx0 - 1.0, sy - 1.0, bar_w + 2.0, sh + 2.0, [0.0, 0.0, 0.0, 0.7]);
    let scolor = if hero.tireless() { 0xff9a3a } else if sfrac < 0.2 && (time * 6.0).sin() > 0.0 { 0x4a7aa8 } else { 0x7ad8ff };
    o.rect(cx0, sy, bar_w * if hero.tireless() { 1.0 } else { sfrac }, sh, rgba(scolor, 1.0));

    // ---- top right: score
    let right_w = t * 8.6;
    let rx = w - margin - right_w;
    panel(o, rx, margin, right_w, band, t);
    let rc = rx + right_w - t * 0.5;
    o.text("SCORE", rc, margin + t * 0.5, small, rgba(SAND, 1.0), Align::Right, true);
    o.text(&format!("{:06}", j.score_shown.round() as u64), rc, margin + t * 1.2, fs * 1.3, rgba(TEXT, 1.0), Align::Right, true);
    // The counter jumps when a nugget lands in it.
    let punch = j.key_punch;
    let ncolor = if punch > 0.0 { mix_rgb(GOLD, 0xffffff, punch * 0.7) } else { GOLD };
    let (tx, ty) = key_target(w, h);
    if g.boss.is_some() {
        o.text("BOSS FIGHT", rc, ty, small, rgba(0xff8a2a, 1.0), Align::Right, true);
    } else {
        // A row of keys: gold for each one found, shadows for the rest.
        let n = g.keys_total.max(1);
        let ks = small * 1.3;
        let gap = ks * 0.92;
        let x0 = rc - gap * n as f32 + gap - ks;
        o.text("KEYS", x0 - t * 0.2, ty, small, rgba(ncolor, 1.0), Align::Right, true);
        for i in 0..n {
            let got = i < g.keys_shown();
            let pop = if got && i + 1 == g.keys_shown() { 1.0 + 0.5 * punch } else { 1.0 };
            let s = ks * pop;
            draw_sprite(o, icon(Icon::Key), x0 + i as f32 * gap - (s - ks) / 2.0, ty - s / 2.0, s / 16.0, if got { None } else { Some(0x3a3a44) }, if got { 1.0 } else { 0.75 });
        }
    }
    if punch > 0.0 {
        o.circle(tx + t * 0.2, ty, t * 0.9 * (1.0 - punch) + t * 0.2, rgba(GOLD, punch * 0.25));
    }
    // The purse: silver and rune fragments to spend at the old man's.
    let cp = j.coin_punch;
    let cs = small * (1.0 + 0.35 * cp);
    let coins = format!("{}", g.progress.silver);
    let cy = margin + t * 2.65;
    o.text(&coins, rc, cy, cs, rgba(mix_rgb(GOLD, 0xffffff, cp * 0.6), 1.0), Align::Right, true);
    let ciw = small * 1.2;
    let cx = rc - o.measure(&coins, cs) - ciw - t * 0.15;
    draw_sprite(o, icon(Icon::Money(g.hero_kind)), cx, cy - ciw / 2.0, ciw / 16.0, None, 1.0);
    let runes = format!("{}", g.progress.runes);
    let rx2 = cx - t * 0.5;
    o.text(&runes, rx2, cy, small, rgba(0x9ad1ff, 1.0), Align::Right, true);
    draw_sprite(o, icon(Icon::Gem(g.hero_kind)), rx2 - o.measure(&runes, small) - ciw - t * 0.1, cy - ciw / 2.0, ciw / 16.0, None, 1.0);

    // ---- bottom middle: weapon, shield and potions
    draw_loadout(o, g, t, fs, small);

    // ---- bottom right: bombs and where you are
    let bh = t * 2.0;
    let by0 = h - margin - bh;
    let name = g.level.data.name.to_uppercase();
    let bw = (o.measure(&name, small) + t * 1.0).max(t * 6.5);
    panel(o, w - margin - bw, by0, bw, bh, t);
    let br = w - margin - t * 0.5;
    o.text(&name, br, by0 + t * 0.55, small, rgba(SAND, 1.0), Align::Right, true);
    let mut x = br;
    let brow = by0 + t * 1.35;
    for _ in 0..g.bombs_left {
        let iw = t * 0.7;
        draw_sprite(o, icon(Icon::Bomb), x - iw, brow - iw / 2.0, iw / 16.0, None, 1.0);
        x -= t * 0.78;
    }
    let label = if g.bombs_left > 0 { "BOMBS" } else { "NO BOMBS" };
    o.text(label, x - t * 0.2, brow, small, rgba(TEXT_DIM, 1.0), Align::Right, true);

    if let Some(fps) = fps {
        o.text(&format!("{fps} fps"), margin + t * 0.3, h - margin - t * 0.4, small, rgba(0x99ffff, 1.0), Align::Left, true);
    }

    draw_boss_bar(o, g, t);

    // Frenzy window, centered along the top edge.
    if g.frenzy.active() {
        let bw = w * 0.3;
        let bxx = w / 2.0 - bw / 2.0;
        let byy = margin + t * 0.1;
        o.rect(bxx, byy, bw, t * 0.14, [1.0, 1.0, 1.0, 0.12]);
        let color = if g.frenzy.warning() { FRENZIED_WARN } else { FRENZIED };
        o.rect(bxx, byy, bw * g.frenzy.fraction() as f32, t * 0.14, rgba(color, 1.0));
    }
}

/**
 * The bottom-middle panel: what Ragnar holds and what protects him. Left,
 * the weapon (its level as pips, ammo for a shrine weapon); middle, the
 * shield and how much it has left; right, the potion belt with the one Q
 * drinks outlined. Running potion effects sit in a row just above it.
 */
fn draw_loadout(o: &mut Overlay, g: &Game, t: f32, fs: f32, small: f32) {
    let (w, h) = (o.width, o.height);
    let hero = &g.hero;
    let time = g.time as f32;
    let margin = t * 0.4;
    let pw = t * 27.0;
    let ph = t * 2.6;
    let x0 = w / 2.0 - pw / 2.0;
    let y0 = h - margin - ph;
    panel(o, x0, y0, pw, ph, t);

    // Weapon.
    let hk = g.hero_kind;
    let (wname, wcolor) = weapon_label(hk, hero.weapon, hero.blade);
    let ic = match hero.weapon {
        WeaponKind::Ranged(r) if hero.holding_pickup() => Icon::Pickup(r),
        _ => Icon::Node(hk, Tree::Armory, hero.blade),
    };
    let iw = t * 2.0 * if hero.swinging() { 1.12 } else { 1.0 };
    o.rect(x0 + t * 0.3, y0 + t * 0.3, t * 2.0, t * 2.0, [1.0, 1.0, 1.0, 0.06]);
    draw_sprite(o, icon(ic), x0 + t * 1.3 - iw / 2.0, y0 + t * 1.3 - iw / 2.0, iw / 16.0, None, 1.0);
    let tx = x0 + t * 2.6;
    let wname = wname.to_uppercase();
    let wsize = (fs * 0.9).min(fs * 0.9 * t * 6.7 / o.measure(&wname, fs * 0.9).max(1.0));
    o.text(&wname, tx, y0 + t * 0.75, wsize, rgba(wcolor, 1.0), Align::Left, true);
    let row2 = y0 + t * 1.65;
    if hero.holding_pickup() {
        let WeaponKind::Ranged(r) = hero.weapon else { unreachable!() };
        let max = weapon(r).ammo;
        let pip = t * 0.28;
        for i in 0..max {
            let c = if i < hero.ammo { rgba(wcolor, 1.0) } else { [1.0, 1.0, 1.0, 0.15] };
            o.rect(tx + i as f32 * (pip + t * 0.1), row2 - pip / 2.0, pip, pip, c);
        }
    } else {
        // Its tier, then whether it is ready.
        let tier = format!("TIER {}", hero.blade);
        o.text(&tier, tx, row2, small, rgba(GOLD, 1.0), Align::Left, true);
        let ready = if hero.weapon == WeaponKind::Sword { hero.attack_cooldown <= 0.0 } else { hero.fire_cooldown <= 0.0 };
        let (label, color) = if ready { ("READY", 0x7dff9a) } else { ("...", TEXT_DIM) };
        o.text(label, tx + o.measure(&tier, small) + t * 0.5, row2, small, rgba(color, 1.0), Align::Left, true);
    }

    // Shield.
    let sx0 = x0 + t * 9.6;
    o.rect(sx0, y0 + t * 0.35, (t * 0.05).max(1.0), ph - t * 0.7, [1.0, 1.0, 1.0, 0.12]);
    let sx = sx0 + t * 0.4;
    match hero.shield {
        Some(sh) => {
            let def = progression::node(hk, Tree::Shield, sh.tier);
            let siw = t * 1.7;
            draw_sprite(o, icon(Icon::Node(hk, Tree::Shield, sh.tier)), sx, y0 + (ph - siw) / 2.0, siw / 16.0, None, 1.0);
            let bx = sx + siw + t * 0.3;
            let name = def.name.to_uppercase();
            let size = if o.measure(&name, small) > t * 5.6 { small * 0.8 } else { small };
            o.text(&name, bx, y0 + t * 0.75, size, rgba(def.color, 1.0), Align::Left, true);
            let frac = sh.fraction();
            let low = frac < 0.3 && (time * 5.0).sin() > 0.0;
            bar(o, bx, y0 + t * 1.45, t * 4.2, t * 0.32, frac, if frac > 0.3 { 0x9ad1ff } else if low { 0xff4a3a } else { 0xff8a6a });
        }
        None => {
            o.text("NO SHIELD", sx, y0 + ph / 2.0, small, rgba(TEXT_DIM, 1.0), Align::Left, true);
        }
    }

    // The potion belt.
    let bx0 = x0 + t * 17.6;
    o.rect(bx0, y0 + t * 0.35, (t * 0.05).max(1.0), ph - t * 0.7, [1.0, 1.0, 1.0, 0.12]);
    let slot = t * 1.55;
    for i in 0..crate::logic::gear::BELT_SLOTS {
        let x = bx0 + t * 0.4 + i as f32 * (slot + t * 0.2);
        let y = y0 + t * 0.3;
        o.rect(x, y, slot, slot, [1.0, 1.0, 1.0, 0.06]);
        if let Some(&k) = g.belt.get(i) {
            draw_sprite(o, icon(Icon::Potion(hk, k)), x + slot * 0.08, y + slot * 0.08, slot * 0.84 / 16.0, None, 1.0);
        }
        let sel = i == g.belt_index && i < g.belt.len();
        o.rect_outline(x, y, slot, slot, if sel { (t * 0.08).max(1.5) } else { 1.0 }, if sel { rgba(GOLD, 0.95) } else { [1.0, 1.0, 1.0, 0.18] });
        o.text(&format!("{}", i + 1), x + slot / 2.0, y + slot + t * 0.35, t * 0.36, rgba(TEXT_DIM, 1.0), Align::Center, false);
    }
    if let Some(&k) = g.belt.get(g.belt_index) {
        let def = crate::logic::gear::potion(hk, k);
        o.text(&format!("Q  {}", def.name.to_uppercase()), bx0 + t * 0.4, y0 - t * 0.35, t * 0.42, rgba(def.color, 1.0), Align::Left, true);
    }

    // Running effects, each with a bar that drains as it wears off.
    let fx = &hero.fx;
    let mut effects: Vec<(Icon, f64, f64, u32)> = Vec::new();
    let mut add = |kind: crate::logic::types::PotionKind, left: f64| {
        if left > 0.0 {
            let d = crate::logic::gear::potion(hk, kind);
            effects.push((Icon::Potion(hk, kind), left, d.seconds.max(left), d.color));
        }
    };
    use crate::logic::types::PotionKind as K;
    add(K::Invulnerability, hero.invincible_timer);
    add(K::Haste, fx.haste.max(hero.speed_timer));
    add(K::Strength, fx.strength);
    add(K::Fury, fx.fury);
    add(K::Fire, fx.fire);
    add(K::Ragnarok, fx.ragnarok);
    add(K::Spirit, fx.spirit);
    if hero.heart > 0.0 {
        let n = progression::node(hk, Tree::Path, 7);
        effects.push((Icon::Node(hk, Tree::Path, 7), hero.heart, progression::path::HEART_SECONDS, n.color));
    }
    if hero.bark > 0 {
        let d = crate::logic::gear::potion(hk, K::Bark);
        effects.push((Icon::Potion(hk, K::Bark), hero.bark as f64, 50.0, d.color));
    }
    let ew = t * 1.1;
    let mut ex = x0 + t * 0.3;
    let ey = y0 - t * 1.55;
    for (ic, left, total, color) in effects {
        o.rect(ex - 2.0, ey - 2.0, ew + 4.0, ew + t * 0.35, [0.0, 0.0, 0.0, 0.55]);
        draw_sprite(o, icon(ic), ex, ey, ew / 16.0, None, 1.0);
        let fr = (left / total.max(0.01)).clamp(0.0, 1.0) as f32;
        o.rect(ex, ey + ew + t * 0.05, ew * fr, t * 0.14, rgba(color, 1.0));
        ex += ew + t * 0.3;
    }
    if hero.slowed() {
        o.text("SLOWED", ex, ey + ew / 2.0, small * 0.9, rgba(0x5cff6a, 1.0), Align::Left, true);
    }
}

fn mix_rgb(a: u32, b: u32, k: f32) -> u32 {
    let ch = |s: u32| {
        let x = ((a >> s) & 255) as f32;
        let y = ((b >> s) & 255) as f32;
        ((x + (y - x) * k.clamp(0.0, 1.0)) as u32) << s
    };
    ch(16) | ch(8) | ch(0)
}

/// Screen-wide feedback drawn over the HUD: flying nuggets, banners, GO!, the hurt vignette.
fn draw_feedback(o: &mut Overlay, g: &Game, t: f32) {
    let (w, h) = (o.width, o.height);
    let j = &g.juice;

    // BRUTAL: a hot flash that burns off from the middle out.
    if j.brutal > 0.0 {
        o.rect(0.0, 0.0, w, h, [1.0, 0.35, 0.15, 0.22 * j.brutal * j.brutal]);
        let bands = 8;
        let bw = t * 0.45;
        for i in 0..bands {
            let a = j.brutal * 0.5 * (1.0 - i as f32 / bands as f32);
            let inset = i as f32 * bw;
            o.rect_outline(inset + bw / 2.0, inset + bw / 2.0, w - 2.0 * inset - bw, h - 2.0 * inset - bw, bw, rgba(0xff5a1a, a));
        }
    }
    // Double points: a badge under the score while it lasts.
    if g.double_points > 0.0 && g.screen == Screen::Playing {
        let margin = t * 0.4;
        let bw = t * 8.6;
        let x = w - margin - bw;
        let y = margin + PANEL_ROWS * t + t * 0.25;
        let pulse = 0.8 + (g.time as f32 * 8.0).sin() * 0.2;
        o.rect(x, y, bw, t * 1.1, [0.35, 0.05, 0.02, 0.8]);
        o.rect_outline(x, y, bw, t * 1.1, (t * 0.07).max(1.0), rgba(0xff5a3a, pulse));
        let secs = g.double_points.ceil() as u32;
        o.text(&format!("x2 POINTS  {secs}s"), x + bw / 2.0, y + t * 0.45, t * 0.6, rgba(0xffd0a0, 1.0), Align::Center, true);
        let fr = (g.double_points / crate::game::DOUBLE_POINTS_SECONDS) as f32;
        o.rect(x + t * 0.3, y + t * 0.85, (bw - t * 0.6) * fr, t * 0.1, rgba(0xff5a3a, 1.0));
    }

    // Red at the screen edges when hurt.
    if j.hurt > 0.0 {
        let bands = 10;
        let bw = t * 0.3;
        for i in 0..bands {
            let a = j.hurt * 0.38 * (1.0 - i as f32 / bands as f32);
            let inset = i as f32 * bw;
            o.rect_outline(inset + bw / 2.0, inset + bw / 2.0, w - 2.0 * inset - bw, h - 2.0 * inset - bw, bw, rgba(0xd8262e, a));
        }
    }

    // Nuggets flying to the counter along an arc, with a short trail.
    let (tx, ty) = key_target(w, h);
    for c in &j.coins {
        for k in 0..4 {
            let u = (c.t - k as f32 * 0.05).clamp(0.0, 1.0);
            let e = u * u * (3.0 - 2.0 * u);
            let ctrl = (c.from.0 + (tx - c.from.0) * 0.2, c.from.1.min(ty) - t * 3.0);
            let bez = |a: f32, b: f32, m: f32| (1.0 - e) * (1.0 - e) * a + 2.0 * (1.0 - e) * e * m + e * e * b;
            let x = bez(c.from.0, tx, ctrl.0);
            let y = bez(c.from.1, ty, ctrl.1);
            let r = t * (0.32 - k as f32 * 0.06) * (1.0 - 0.35 * e);
            let a = 1.0 - k as f32 * 0.25;
            o.circle(x, y, r, rgba(GOLD, a));
            if k == 0 {
                o.circle(x - r * 0.25, y - r * 0.25, r * 0.45, rgba(0xfff1a8, a));
            }
        }
    }

    // Milestone banner.
    if let Some(b) = &j.banner {
        let age = b.max - b.life;
        let big = b.text == "BRUTAL!";
        let pop = if big { 1.0 + 1.6 * (1.0 - (age / 0.25).min(1.0)).powi(3) } else { 1.0 + 0.55 * (1.0 - (age / 0.2).min(1.0)).powi(2) };
        let pop = if big { pop * 1.7 } else { pop };
        let alpha = (b.life / 0.4).min(1.0);
        let y = h * 0.27;
        o.rect(0.0, y - t * 1.3, w, t * 2.6, [2.0 / 255.0, 6.0 / 255.0, 12.0 / 255.0, 0.45 * alpha]);
        o.text(&b.text, w / 2.0, y - t * 0.25, t * 1.4 * pop, rgba(b.color, alpha), Align::Center, true);
        o.text(&b.sub, w / 2.0, y + t * 0.85, t * 0.6, rgba(SAND, alpha), Align::Center, true);
    }

    // GO!
    if j.go > 0.0 {
        let k = j.go / 0.8;
        let size = t * 4.5 * (1.0 + (1.0 - k) * 0.7);
        o.text("GO!", w / 2.0, h * 0.42, size, rgba(0x7dff9a, k.min(1.0)), Align::Center, true);
    }
}

/// The countdown before play: 3, 2, 1, each landing big and settling.
fn draw_countdown(o: &mut Overlay, g: &Game, t: f32) {
    let (w, h) = (o.width, o.height);
    o.rect(0.0, 0.0, w, h, [2.0 / 255.0, 6.0 / 255.0, 12.0 / 255.0, 0.3]);
    let left = g.screen_timer.max(0.0) as f32;
    let n = left.ceil().max(1.0) as u32;
    let k = left - left.floor(); // 1 -> 0 through each second
    let k = if k == 0.0 { 1.0 } else { k };
    let pop = 1.0 + 0.7 * k.powi(4);
    let alpha = (k * 3.0).min(1.0);
    let y = h * 0.42;
    o.circle(w / 2.0, y, t * 3.2 * pop, [1.0, 1.0, 1.0, 0.05 * alpha]);
    o.text(&n.to_string(), w / 2.0, y, t * 5.0 * pop, rgba(GOLD, alpha), Align::Center, true);
    let ready = if g.boss.is_some() { "THE CYCLOPS AWAITS" } else { "GET READY" };
    o.text(ready, w / 2.0, y + t * 3.2, t * 0.9, rgba(if g.boss.is_some() { 0xff8a2a } else { TEXT }, 1.0), Align::Center, true);
    // The level's name, large and gold across the top, as the count begins.
    let name = g.level.data.name.to_uppercase();
    let fit = w * 0.86 / (name.chars().count().max(1) as f32 * o.measure("M", 1.0));
    let ny = h * 0.2;
    o.text(&crate::logic::story::depth_line(g.dungeon, g.boss.is_some()), w / 2.0, ny - t * 1.9, t * 0.55, rgba(SAND, 1.0), Align::Center, true);
    draw_logo_word(o, &name, w / 2.0, ny, (t * 2.0).min(fit), g.time as f32);
    draw_ornament(o, w / 2.0, ny + t * 1.55, t * 12.0, t, g.time as f32);
}

// ------------------------------------------------------------------- tour

/**
 * The first-run tour's overlay: letterbox bars slide in, a pulsing frame
 * locks onto what the camera is showing, and a see-through card names it
 * and explains it, with a picture, progress dots and how to move on.
 */
fn draw_tour(o: &mut Overlay, g: &Game, t: f32) {
    let Some(tour) = &g.tour else { return };
    let (w, h) = (o.width, o.height);
    let stop = tour.stop();
    let time = g.time as f32;
    let k = (tour.age as f32 / 0.35).min(1.0);
    let bar_h = h * 0.1 * (1.0 - (1.0 - k).powi(3));
    o.rect(0.0, 0.0, w, bar_h, [0.0, 0.0, 0.0, 0.92]);
    o.rect(0.0, h - bar_h, w, bar_h, [0.0, 0.0, 0.0, 0.92]);

    // Lock-on frame around the target, tightening as the camera arrives.
    let st = tour.t as f32;
    let settle = (st / 0.7).min(1.0);
    if let Some((x, y)) = g.view.project(stop.target, w, h) {
        let size = t * (2.4 + 3.0 * (1.0 - settle)) * (1.0 + (time * 4.0).sin() * 0.04);
        let arm = size * 0.35;
        let lw = (t * 0.1).max(2.0);
        let c = rgba(stop.color, 0.9 * settle.max(0.3));
        for (sx, sy) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let (cx, cy) = (x + sx * size, y + sy * size);
            o.line([cx, cy], [cx - sx * arm, cy], lw, c);
            o.line([cx, cy], [cx, cy - sy * arm], lw, c);
        }
    }

    // The card: a see-through panel above the lower bar.
    let fade = (st / 0.25).min(1.0);
    let cw = (w * 0.42).max(t * 26.0).min(w - t * 1.6);
    let chh = t * 6.4;
    let cx = t * 0.8;
    let cy = h - bar_h - chh - t * 0.6;
    o.rect(cx, cy, cw, chh, [0.02, 0.03, 0.05, 0.5 * fade]);
    o.rect(cx, cy, t * 0.14, chh, rgba(stop.color, 0.9 * fade));
    o.rect_outline(cx, cy, cw, chh, 1.0, [1.0, 1.0, 1.0, 0.18 * fade]);
    let art = t * 4.2;
    let ax = cx + t * 0.6;
    let ay = cy + (chh - art) / 2.0;
    draw_tour_art(o, g.hero_kind, stop.art, ax, ay, art, fade);
    let tx = ax + art + t * 0.7;
    // The title types itself out.
    let shown = ((st * 60.0) as usize).min(stop.title.len());
    o.text(&stop.title[..shown], tx, cy + t * 1.0, t * 1.0, rgba(stop.color, fade), Align::Left, true);
    wrap_left(o, stop.body, tx, cy + t * 2.1, cw - (tx - cx) - t * 0.6, t * 0.62, t * 0.5, rgba(TEXT, fade));

    // Progress dots and the way on.
    let n = tour.stops.len();
    let dy = cy + chh - t * 0.55;
    for i in 0..n {
        let on = i == tour.index;
        let c = if on { rgba(stop.color, fade) } else if i < tour.index { [1.0, 1.0, 1.0, 0.5 * fade] } else { [1.0, 1.0, 1.0, 0.18 * fade] };
        let r = if on { t * 0.16 } else { t * 0.11 };
        o.circle(tx + i as f32 * t * 0.5, dy, r, c);
    }
    let left = (crate::game::TOUR_STOP_SECONDS - tour.t).max(0.0) as f32 / crate::game::TOUR_STOP_SECONDS as f32;
    o.rect(cx, cy + chh - t * 0.08, cw * left, t * 0.08, rgba(stop.color, 0.6 * fade));
    o.text("Enter / click / A: next     Esc: skip the tour", w - t * 0.8, h - bar_h / 2.0, t * 0.5, rgba(TEXT_DIM, k), Align::Right, false);
    o.text("ADVENTURES OF THE ANCIENT BLADES", t * 0.8, bar_h / 2.0, t * 0.5, rgba(SAND, k * 0.8), Align::Left, false);
    o.text(&format!("{} / {}", tour.index + 1, n), w - t * 0.8, bar_h / 2.0, t * 0.5, rgba(SAND, k * 0.8), Align::Right, false);
}

/// The picture on a tour card: a portrait of a foe, or an icon of a thing.
fn draw_tour_art(o: &mut Overlay, hero: crate::logic::heroes::HeroKind, art: crate::game::TourArt, x: f32, y: f32, size: f32, alpha: f32) {
    use crate::game::TourArt;
    use crate::logic::enemy::EnemyKind;
    o.rect(x, y, size, size, [1.0, 1.0, 1.0, 0.06 * alpha]);
    let sp = sprites();
    let fit = |o: &mut Overlay, s: &Sprite| {
        let scale = (size * 0.9) / s.width.max(s.height) as f32;
        let (sw, sh) = (s.width as f32 * scale, s.height as f32 * scale);
        draw_sprite(o, s, x + (size - sw) / 2.0, y + (size - sh) / 2.0, scale, None, alpha);
    };
    match art {
        TourArt::Hero => fit(o, portrait_of(hero)),
        TourArt::Fountain => fit(o, &sp.krampus),
        TourArt::Enemy(kind) => {
            let i = match kind {
                EnemyKind::Skeleton => 0,
                EnemyKind::Bat | EnemyKind::Vampire => 1,
                EnemyKind::Slime => 2,
            };
            let tint = if kind == EnemyKind::Vampire { Some(0x9a3060) } else { None };
            let s = &sp.cast[i].0;
            let scale = (size * 0.9) / s.width.max(s.height) as f32;
            let (sw, sh) = (s.width as f32 * scale, s.height as f32 * scale);
            draw_sprite(o, s, x + (size - sw) / 2.0, y + (size - sh) / 2.0, scale, tint, alpha);
        }
        TourArt::Nugget => fit(o, icon(Icon::Key)),
        TourArt::Gate => fit(o, icon(Icon::Lock)),
        TourArt::Potion(k) => fit(o, icon(Icon::Potion(crate::logic::heroes::HeroKind::Ragnar, k))),
        TourArt::WeaponShrine => fit(o, icon(Icon::Pickup(crate::logic::items::RangedKind::Trishot))),
        TourArt::ItemShrine => fit(o, icon(Icon::Node(crate::logic::heroes::HeroKind::Ragnar, Tree::Shield, 2))),
        TourArt::Cross => {
            let c = rgba(0x4cf06a, alpha);
            o.rect(x + size * 0.4, y + size * 0.15, size * 0.2, size * 0.7, c);
            o.rect(x + size * 0.15, y + size * 0.4, size * 0.7, size * 0.2, c);
        }
        TourArt::Pit => {
            o.circle(x + size / 2.0, y + size / 2.0, size * 0.38, rgba(0x2a1e12, alpha));
            o.circle(x + size / 2.0, y + size / 2.0, size * 0.3, rgba(0x050406, alpha));
        }
    }
}

// ------------------------------------------------------------- world labels

/// Health bar over Ragnar, shrine labels, and floating text projected onto the overlay.
fn draw_world_overlays(o: &mut Overlay, g: &Game, t: f32) {
    let (w, h) = (o.width, o.height);
    let grid = &g.level.grid;
    let view = &g.view;
    if g.hero.alive && g.screen == Screen::Playing {
        let hp = g.hero.pos();
        let p = Vec3::new(hp.x as f32, g.hero.altitude as f32 * STEP + 1.45, hp.y as f32);
        if let Some((x, y)) = view.project(p, w, h) {
            let bw = t * 1.8;
            let bh = (t * 0.22).max(5.0);
            let frac = g.hero.hp as f32 / g.hero.max_hp as f32;
            bar(o, x - bw / 2.0, y, bw, bh, frac, hp_color(frac));
        }
    }
    if g.hero.sees() && g.screen == Screen::Playing {
        // Seer's Sight: every key, bottle, cross and the hidden shrine marked, walls or not.
        let pulse = 0.75 + (g.time as f32 * 5.0).sin() * 0.25;
        let fade = 0.8;
        let mark = |o: &mut Overlay, x: i32, y: i32, color: u32| {
            let (fx, fy) = (x as f32, y as f32);
            if let Some((sx, sy)) = view.project(Vec3::new(fx, floor_at(grid, fx, fy) + 0.9, fy), w, h) {
                let r = t * 0.32 * pulse;
                o.line([sx, sy - r], [sx + r, sy], 2.0, rgba(color, fade));
                o.line([sx + r, sy], [sx, sy + r], 2.0, rgba(color, fade));
                o.line([sx, sy + r], [sx - r, sy], 2.0, rgba(color, fade));
                o.line([sx - r, sy], [sx, sy - r], 2.0, rgba(color, fade));
                o.line([sx, sy + r], [sx, sy + r * 2.2], 1.5, rgba(color, fade * 0.6));
            }
        };
        for (x, y, tile) in grid.cells() {
            match tile {
                crate::logic::types::Tile::Nodule => mark(o, x, y, GOLD),
                crate::logic::types::Tile::Charge => mark(o, x, y, 0x5cff6a),
                _ => {}
            }
        }
        for p in g.potions.iter().filter(|p| !p.taken) {
            mark(o, p.tile.x, p.tile.y, p.def(g.hero_kind).color);
        }
        if let Some(r) = g.relic_shrine {
            mark(o, r.x, r.y, 0x9ad1ff);
        }
    }
    for d in &g.diamonds {
        if !d.active {
            continue;
        }
        let (tx, ty) = (d.tile.x as f32, d.tile.y as f32);
        if let Some((x, y)) = view.project(Vec3::new(tx, floor_at(grid, tx, ty) + 1.25, ty), w, h) {
            let offer = d.current();
            o.text(&offer.name().to_uppercase(), x, y, t * 0.52, rgba(offer.color(), 1.0), Align::Center, true);
        }
    }
    for f in &g.floaters {
        let (fx, fy) = (f.x as f32, f.y as f32);
        let lift = floor_at(grid, fx, fy) + 0.9 + (1.0 - f.life as f32) * 0.6;
        if let Some((x, y)) = view.project(Vec3::new(fx, lift, fy), w, h) {
            let alpha = (f.life as f32 * 2.0).min(1.0);
            // Pop in big, settle, then drift up and fade.
            let age = (f.max_life - f.life) as f32;
            let pop = 1.0 + 0.8 * (1.0 - (age / 0.15).min(1.0)).powi(2);
            o.text(&f.text, x, y, t * 0.64 * pop, rgba(f.color.unwrap_or(GOLD), alpha), Align::Center, true);
        }
    }
}

// ------------------------------------------------------------------ screens

/// Button rectangles `[x, y, w, h]` for a menu, in logical pixels. The title
/// menu is a row along the bottom; the pause menu is a centered column.
pub fn menu_rects(screen: Screen, items: &[MenuItem], w: f32, h: f32) -> Vec<[f32; 4]> {
    let t = tile_for(w, h);
    let bh = t * 1.3;
    let count = items.len();
    match screen {
        Screen::Title if items.contains(&MenuItem::StartOver) => {
            // The two answers side by side at the foot of the "start over?" panel.
            let [px, py, pw, ph] = reset_panel(w, h);
            let (bw, bh, gap) = ((pw - t * 2.2) / 2.0, t * 1.4, t * 0.6);
            let y = py + ph - bh - t * 0.8;
            (0..count).map(|i| [px + (pw - 2.0 * bw - gap) / 2.0 + i as f32 * (bw + gap), y, bw, bh]).collect()
        }
        Screen::Title => {
            // Continue and Start side by side in the middle below the line-up; Quit (desktop) small in the corner.
            let (bw, bh, gap) = (t * 9.0, t * 1.9, t * 0.9);
            let big = items.iter().filter(|i| **i != MenuItem::Quit).count();
            let x0 = w / 2.0 - (big as f32 * bw + (big as f32 - 1.0) * gap) / 2.0;
            items
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    if *item == MenuItem::Quit {
                        let (qw, qh) = (t * 4.6, t * 1.1);
                        [w - qw - t * 0.6, h - qh - t * 0.6, qw, qh]
                    } else {
                        [x0 + i as f32 * (bw + gap), h * 0.875 - bh / 2.0, bw, bh]
                    }
                })
                .collect()
        }
        _ => {
            let bw = t * 9.0;
            let gap = t * 0.45;
            let y0 = h / 2.0 - t * 0.6;
            (0..count).map(|i| [w / 2.0 - bw / 2.0, y0 + i as f32 * (bh + gap), bw, bh]).collect()
        }
    }
}

/// The panel that asks whether to start over, in the middle of the title.
fn reset_panel(w: f32, h: f32) -> [f32; 4] {
    let t = tile_for(w, h);
    let (pw, ph) = ((t * 22.0).min(w - t * 1.2), t * 7.4);
    [w / 2.0 - pw / 2.0, h / 2.0 - ph / 2.0, pw, ph]
}

/// The current screen's buttons, with the highlighted one outlined in gold.
fn draw_menu(o: &mut Overlay, g: &Game, t: f32) {
    let items = g.menu_items();
    let rects = menu_rects(g.screen, &items, o.width, o.height);
    let pulse = 0.75 + (g.time as f32 * 4.0).sin() * 0.25;
    for (i, (item, r)) in items.iter().zip(rects).enumerate() {
        let selected = i == g.menu_index;
        let quit = matches!(item, MenuItem::Quit | MenuItem::QuitToTitle | MenuItem::StartOver);
        let bg = if selected { [1.0, 1.0, 1.0, 0.14] } else { [1.0, 1.0, 1.0, 0.05] };
        o.rect(r[0], r[1], r[2], r[3], [2.0 / 255.0, 6.0 / 255.0, 12.0 / 255.0, 0.85]);
        o.rect(r[0], r[1], r[2], r[3], bg);
        let edge = if selected { rgba(GOLD, pulse) } else { [1.0, 1.0, 1.0, 0.18] };
        o.rect_outline(r[0], r[1], r[2], r[3], (t * 0.08).max(1.0), edge);
        let color = if selected { if quit { 0xff8a8a } else { GOLD } } else { TEXT_DIM };
        let big = matches!(item, MenuItem::Play | MenuItem::Continue);
        if big && selected {
            // The highlighted big button glows gold and breathes.
            o.rect_outline(r[0] - t * 0.12, r[1] - t * 0.12, r[2] + t * 0.24, r[3] + t * 0.24, (t * 0.05).max(1.0), rgba(GOLD, 0.35 * pulse));
        }
        let color = if big && !selected { SAND } else { color };
        o.text(item.label(), r[0] + r[2] / 2.0, r[1] + r[3] / 2.0, t * if big { 1.15 } else { 0.55 }, rgba(color, 1.0), Align::Center, true);
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

/// Start screen. The real 3D Ragnar stands in the middle while the camera
/// circles him; the logo, the quest, the foes and the controls frame him.
fn draw_title(o: &mut Overlay, g: &Game, t: f32) {
    let (w, h) = (o.width, o.height);
    let time = g.time as f32;

    // Shade the scene top and bottom, keep the middle clear for Ragnar.
    let bands = 24;
    for i in 0..bands {
        let y0 = h * i as f32 / bands as f32;
        let v = i as f32 / (bands - 1) as f32;
        let a = if v < 0.3 { 0.8 - v / 0.3 * 0.62 } else if v < 0.76 { 0.18 } else { 0.18 + (v - 0.76) / 0.24 * 0.6 };
        o.rect(0.0, y0, w, h / bands as f32 + 1.0, [2.0 / 255.0, 6.0 / 255.0, 12.0 / 255.0, a]);
    }
    // Side vignettes behind the panels.
    for i in 0..10 {
        let a = 0.05 * (10 - i) as f32 / 10.0;
        o.rect(0.0, 0.0, w * 0.035 * (i + 1) as f32, h, [0.0, 0.0, 0.0, a]);
        o.rect(w - w * 0.035 * (i + 1) as f32, 0.0, w * 0.035 * (i + 1) as f32, h, [0.0, 0.0, 0.0, a]);
    }

    // ---- logo
    o.text("ADVENTURES OF THE", w / 2.0, h * 0.075, t * 0.95, rgba(SAND, 1.0), Align::Center, true);
    draw_logo_word(o, "ANCIENT BLADES", w / 2.0, h * 0.075 + t * 2.6, t * 2.9, time);
    let orn_y = h * 0.075 + t * 4.55;
    draw_ornament(o, w / 2.0, orn_y, t * 15.0, t, time);
    // The three heroes' names, small gold diamonds between them.
    {
        let (y, size) = (orn_y + t * 0.95, t * 0.6);
        let gap = t * 1.4;
        let names = ["Mehmed", "Ragnar", "Miyamoto"];
        let total: f32 = names.iter().map(|n| o.measure(n, size)).sum::<f32>() + gap * 2.0;
        let mut x = w / 2.0 - total / 2.0;
        for (i, n) in names.iter().enumerate() {
            o.text(n, x, y, size, rgba(0xcfc2a0, 1.0), Align::Left, false);
            x += o.measure(n, size);
            if i < 2 {
                let (cx, r, lw) = (x + gap / 2.0, t * 0.16, (t * 0.07).max(1.2));
                let c = rgba(GOLD, 0.9);
                o.line([cx - r, y], [cx, y - r], lw, c);
                o.line([cx, y - r], [cx + r, y], lw, c);
                o.line([cx + r, y], [cx, y + r], lw, c);
                o.line([cx, y + r], [cx - r, y], lw, c);
                x += gap;
            }
        }
    }

    // One line on how to play, tucked under the button.
    o.text(
        "WASD or the left stick to run   Left click / X attack   Right click / B bomb   Space / A jump   H / Back call the old man   Esc pause",
        w / 2.0,
        h * 0.965,
        t * 0.45,
        rgba(TEXT_DIM, 0.9),
        Align::Center,
        false,
    );

    // Under the buttons: where Continue leads, and that Start begins anew.
    if !g.confirm_reset && g.can_continue() {
        let items = g.menu_items();
        let rects = menu_rects(g.screen, &items, w, h);
        for (item, r) in items.iter().zip(&rects) {
            let note = match item {
                MenuItem::Continue => g.continue_caption().unwrap_or_else(|| "Choose a hero and keep what is earned".to_string()),
                MenuItem::Play => "A new adventure, all reset".to_string(),
                _ => continue,
            };
            o.wrap_text(&note, r[0] + r[2] / 2.0, r[1] + r[3] + t * 0.5, r[2], t * 0.55, t * 0.42, rgba(TEXT_DIM, 1.0));
        }
    }
    if g.confirm_reset {
        o.rect(0.0, 0.0, w, h, [2.0 / 255.0, 6.0 / 255.0, 12.0 / 255.0, 0.6]);
        let [px, py, pw, ph] = reset_panel(w, h);
        panel(o, px, py, pw, ph, t);
        o.text("START OVER?", w / 2.0, py + t * 1.35, t * 1.1, rgba(GOLD, 1.0), Align::Center, true);
        o.wrap_text(
            "Every hero's weapons, shields, paths and elixirs, the silver, the runes and the adventure under way will be lost.",
            w / 2.0,
            py + t * 2.6,
            pw - t * 2.4,
            t * 0.72,
            t * 0.5,
            rgba(SAND, 1.0),
        );
    }
    draw_menu(o, g, t);
    o.text(&format!("Best {}", g.score.high), w - t * 0.6, h * 0.035, t * 0.5, rgba(TEXT_DIM, 1.0), Align::Right, false);
}

/// The title word: a dark outline, a bronze bevel below, gold on top, and a
/// shimmer of light sweeping through the letters.
fn draw_logo_word(o: &mut Overlay, word: &str, cx: f32, cy: f32, size: f32, time: f32) {
    let shadow = std::mem::replace(&mut o.shadow, 0.0);
    let n = word.chars().count() as f32;
    let adv = o.measure("M", size);
    let x0 = cx - adv * n / 2.0;
    let sweep = ((time * 0.35) % 1.6) * (n + 6.0) - 3.0;
    let d = (size * 0.05).max(2.0);
    for (i, ch) in word.chars().enumerate() {
        let s = ch.to_string();
        let x = x0 + adv * (i as f32 + 0.5);
        let bounce = (time * 2.2 + i as f32 * 0.5).sin() * size * 0.015;
        let y = cy + bounce;
        for (dx, dy) in [(-d, 0.0), (d, 0.0), (0.0, -d), (0.0, d), (-d, -d), (d, d), (-d, d), (d, -d)] {
            o.text(&s, x + dx, y + dy + d, size, [0.0, 0.0, 0.0, 0.85], Align::Center, true);
        }
        o.text(&s, x, y + d * 1.2, size, rgba(0x8a5a14, 1.0), Align::Center, true);
        let glint = (1.0 - ((i as f32 - sweep).abs() / 1.6)).clamp(0.0, 1.0);
        let color = mix_rgb(GOLD, 0xfff6d0, glint * 0.85);
        o.text(&s, x, y, size, rgba(color, 1.0), Align::Center, true);
    }
    o.shadow = shadow;
}

// ---------------------------------------------------------- character screen

/// A rectangle on screen: x, y, width, height.
#[derive(Clone, Copy, Debug)]
pub struct Rect(pub [f32; 4]);

impl Rect {
    pub fn contains(&self, p: (f32, f32)) -> bool {
        let r = self.0;
        p.0 >= r[0] && p.0 <= r[0] + r[2] && p.1 >= r[1] && p.1 <= r[1] + r[3]
    }
}

/// Where the character screen's parts sit: a third of the screen for each hero, and the buttons.
pub struct SelectLayout {
    pub heroes: [Rect; 3],
    pub back: Rect,
    pub begin: Rect,
}

impl SelectLayout {
    pub fn hero_at(&self, p: (f32, f32)) -> Option<usize> {
        self.heroes.iter().position(|r| r.contains(p))
    }
}

pub fn select_layout(w: f32, h: f32) -> SelectLayout {
    let t = tile_for(w, h);
    let (bw, bh, gap) = (t * 6.0, t * 1.3, t * 0.8);
    let y = h * 0.93 - bh / 2.0;
    SelectLayout {
        heroes: [0, 1, 2].map(|i| Rect([w * i as f32 / 3.0, h * 0.18, w / 3.0, h * 0.62])),
        back: Rect([w / 2.0 - gap / 2.0 - bw, y, bw, bh]),
        begin: Rect([w / 2.0 + gap / 2.0, y, bw, bh]),
    }
}

/**
 * The character screen over its 3D cavern: the title, a name plate under
 * each hero (grey with a lock for those not yet unlocked, gold for the one
 * picked), and Back and Begin.
 */
fn draw_select(o: &mut Overlay, g: &Game, t: f32) {
    use crate::logic::heroes::ROSTER;
    let (w, h) = (o.width, o.height);
    let time = g.time as f32;
    // Darken the top and bottom so the words read over the cavern.
    let bands = 24;
    for i in 0..bands {
        let v = i as f32 / (bands - 1) as f32;
        let a = if v < 0.2 { 0.78 - v / 0.2 * 0.6 } else if v < 0.7 { 0.18 } else { 0.18 + (v - 0.7) / 0.3 * 0.62 };
        o.rect(0.0, h * i as f32 / bands as f32, w, h / bands as f32 + 1.0, [2.0 / 255.0, 3.0 / 255.0, 6.0 / 255.0, a]);
    }

    // ---- title
    o.text("ADVENTURES OF THE ANCIENT BLADES", w / 2.0, h * 0.045, t * 0.55, rgba(SAND, 0.9), Align::Center, false);
    draw_logo_word(o, "CHOOSE YOUR HERO", w / 2.0, h * 0.045 + t * 1.55, t * 1.55, time);
    draw_ornament(o, w / 2.0, h * 0.045 + t * 2.85, t * 11.0, t, time);

    // ---- a plate under each hero's feet
    let hero = g.select_hero();
    for (i, &kind) in ROSTER.iter().enumerate() {
        let picked = i == g.select_index;
        let Some((sx, sy)) = g.view.project(g.view.select_feet(i), w, h) else { continue };
        let shake = if picked && g.select_denied > 0.0 { (g.select_denied as f32 * 70.0).sin() * t * 0.3 } else { 0.0 };
        let (pw, ph) = (t * 6.4, t * 1.9);
        let (px, py) = (sx - pw / 2.0 + shake, sy + t * 0.35);
        let open = kind.unlocked();
        o.rect(px, py, pw, ph, [2.0 / 255.0, 6.0 / 255.0, 12.0 / 255.0, if picked { 0.88 } else { 0.7 }]);
        let pulse = 0.7 + (time * 4.0).sin() * 0.3;
        let edge = match (picked, open) {
            (true, true) => rgba(GOLD, pulse),
            (true, false) => rgba(if g.select_denied > 0.0 { 0xff5a5a } else { 0x9aa4b4 }, 0.9),
            _ => [1.0, 1.0, 1.0, 0.15],
        };
        o.rect_outline(px, py, pw, ph, (t * if picked { 0.1 } else { 0.06 }).max(1.0), edge);
        let name_color = match (open, picked) {
            (true, true) => GOLD,
            (true, false) => SAND,
            (false, _) => 0x8c929c,
        };
        let cx = px + pw / 2.0;
        o.text(&kind.name().to_uppercase(), cx, py + ph * 0.36, t * if picked { 0.95 } else { 0.8 }, rgba(name_color, 1.0), Align::Center, true);
        o.text(kind.title(), cx, py + ph * 0.74, t * 0.5, rgba(if open { TEXT_DIM } else { 0x6c727c }, 1.0), Align::Center, false);
        if !open {
            // A padlock on the plate's shoulder, and the word under it.
            let lock = icon(Icon::Lock);
            let scale = (t * 1.1) / lock.height.max(1) as f32;
            draw_sprite(o, lock, px - lock.width as f32 * scale * 0.4, py - lock.height as f32 * scale * 0.45, scale, Some(0xb4bcc8), 1.0);
            o.text("LOCKED", cx, py + ph + t * 0.45, t * 0.48, rgba(if picked && g.select_denied > 0.0 { 0xff6a6a } else { 0x8c929c }, 1.0), Align::Center, true);
        }
    }

    // ---- buttons: Back, and Begin (dimmed while a locked hero is picked)
    let layout = select_layout(w, h);
    for (r, label, live) in [(layout.back, "BACK", true), (layout.begin, "BEGIN", hero.unlocked())] {
        let r = r.0;
        let hover = g.input.cursor.is_some_and(|p| Rect(r).contains(p));
        o.rect(r[0], r[1], r[2], r[3], [2.0 / 255.0, 6.0 / 255.0, 12.0 / 255.0, 0.85]);
        let main = label == "BEGIN";
        let edge = if main && live { rgba(GOLD, 0.75 + (time * 4.0).sin() * 0.25) } else if hover { [1.0, 1.0, 1.0, 0.4] } else { [1.0, 1.0, 1.0, 0.18] };
        o.rect_outline(r[0], r[1], r[2], r[3], (t * 0.08).max(1.0), edge);
        let color = if !live { 0x5c626c } else if main { GOLD } else { TEXT_DIM };
        o.text(label, r[0] + r[2] / 2.0, r[1] + r[3] / 2.0, t * 0.7, rgba(color, 1.0), Align::Center, true);
    }
    o.text("Left / Right choose     Enter begin     Esc back", w / 2.0, h * 0.985, t * 0.45, rgba(TEXT_DIM, 1.0), Align::Center, false);
}

/// A knotwork-style rule: a line with diamonds at the ends and a gem in the middle.
fn draw_ornament(o: &mut Overlay, cx: f32, cy: f32, width: f32, t: f32, time: f32) {
    let lw = (t * 0.08).max(1.5);
    let c = rgba(GOLD, 0.85);
    let half = width / 2.0;
    o.line([cx - half, cy], [cx - t * 0.8, cy], lw, c);
    o.line([cx + t * 0.8, cy], [cx + half, cy], lw, c);
    let diamond = |o: &mut Overlay, x: f32, r: f32, color: [f32; 4]| {
        o.line([x - r, cy], [x, cy - r], lw, color);
        o.line([x, cy - r], [x + r, cy], lw, color);
        o.line([x + r, cy], [x, cy + r], lw, color);
        o.line([x, cy + r], [x - r, cy], lw, color);
    };
    diamond(o, cx - half, t * 0.3, c);
    diamond(o, cx + half, t * 0.3, c);
    diamond(o, cx, t * 0.5, c);
    let pulse = 0.6 + (time * 3.0).sin() * 0.4;
    o.circle(cx, cy, t * 0.2, rgba(0xff8a2a, pulse));
    for k in [-1.0, 1.0] {
        diamond(o, cx + k * half * 0.5, t * 0.18, rgba(GOLD, 0.6));
    }
}

/// Left-aligned word wrap. Returns the y just below the last line.
#[allow(clippy::too_many_arguments)]
fn wrap_left(o: &mut Overlay, text: &str, x: f32, mut y: f32, max_width: f32, line_height: f32, size: f32, color: [f32; 4]) -> f32 {
    let mut line = String::new();
    for word in text.split(' ') {
        let trial = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if o.measure(&trial, size) > max_width && !line.is_empty() {
            o.text(&line, x, y, size, color, Align::Left, false);
            y += line_height;
            line = word.to_string();
        } else {
            line = trial;
        }
    }
    if !line.is_empty() {
        o.text(&line, x, y, size, color, Align::Left, false);
        y += line_height;
    }
    y
}
