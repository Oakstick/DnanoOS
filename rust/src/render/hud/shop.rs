//! The blacksmith's shop screen: a panel on the right over the 3D forge,
//! with tabs, rows of goods, and a line of feedback. `shop_rects` gives the
//! button rectangles so the game can hit-test the same layout it draws.

use super::*;
use crate::game::{RowKind, ShopAction, ShopRow, ShopTab};
use crate::logic::gear::{self, Line, BELT_SLOTS};
use crate::render::icons::{icon, Icon};

struct Layout {
    /// Panel x, y, width, height.
    panel: [f32; 4],
    rows: Vec<[f32; 4]>,
    buttons: Vec<Vec<[f32; 4]>>,
    /// Where the strip above the goods sits (belt or current shield).
    strip: [f32; 4],
}

fn layout(rows: &[ShopRow], w: f32, h: f32) -> Layout {
    let t = tile_for(w, h);
    let margin = t * 0.4;
    let x0 = (w * 0.41).max(w - margin - t * 38.0);
    let pw = w - margin - x0;
    let (y0, ph) = (margin, h - margin * 2.0);
    let pad = t * 0.6;
    let inner = pw - pad * 2.0;
    let strip = [x0 + pad, y0 + t * 4.3, inner, t * 2.1];
    let has_strip = rows.iter().any(|r| matches!(r.kind, RowKind::Potions | RowKind::Shield(_)));
    let mut y = y0 + t * 4.3 + if has_strip { t * 2.5 } else { 0.0 };
    let mut out_rows = Vec::new();
    let mut buttons = Vec::new();
    for row in rows {
        let (rect, b): ([f32; 4], Vec<[f32; 4]>) = match row.kind {
            RowKind::Tabs => {
                let gap = t * 0.3;
                let bw = (inner - gap * 2.0) / 3.0;
                let r = [x0 + pad, y0 + t * 2.4, inner, t * 1.3];
                (r, (0..row.buttons.len()).map(|i| [r[0] + i as f32 * (bw + gap), r[1], bw, r[3]]).collect())
            }
            RowKind::Weapon(_) => {
                let r = [x0 + pad, y, inner, t * 5.0];
                y += t * 5.4;
                let bw = t * 6.6;
                (r, (0..row.buttons.len()).map(|i| [r[0] + r[2] - bw - t * 0.4, r[1] + t * 0.8 + i as f32 * t * 1.75, bw, t * 1.45]).collect())
            }
            RowKind::Shield(_) => {
                let r = [x0 + pad, y, inner, t * 3.6];
                y += t * 3.95;
                let bw = t * 6.6;
                (r, vec![[r[0] + r[2] - bw - t * 0.4, r[1] + (r[3] - t * 1.45) / 2.0, bw, t * 1.45]])
            }
            RowKind::Repair => {
                let r = [x0 + pad, y, inner, t * 2.0];
                y += t * 2.3;
                let bw = t * 6.6;
                (r, vec![[r[0] + r[2] - bw - t * 0.4, r[1] + (r[3] - t * 1.35) / 2.0, bw, t * 1.35]])
            }
            RowKind::Potions => {
                let r = [x0 + pad, y, inner, t * 3.2];
                y += t * 3.45;
                let cw = (inner - t * 0.3) / 2.0;
                let bw = t * 4.3;
                (r, (0..row.buttons.len()).map(|i| [r[0] + i as f32 * (cw + t * 0.3) + cw - bw - t * 0.3, r[1] + r[3] - t * 1.55, bw, t * 1.25]).collect())
            }
            RowKind::Leave => {
                let bw = t * 9.0;
                let r = [x0 + pad, y0 + ph - t * 1.9, inner, t * 1.4];
                (r, vec![[x0 + pw / 2.0 - bw / 2.0, r[1], bw, r[3]]])
            }
        };
        out_rows.push(rect);
        buttons.push(b);
    }
    Layout { panel: [x0, y0, pw, ph], rows: out_rows, buttons, strip }
}

/// Button rectangles per row, for hit-testing the shop.
pub fn shop_rects(rows: &[ShopRow], w: f32, h: f32) -> Vec<Vec<[f32; 4]>> {
    layout(rows, w, h).buttons
}

const SMITH_LINES: [&str; 3] = [
    "\"Fine steel for fine coin, viking. What'll it be?\"",
    "\"A shield is only as good as the arm behind it. And the iron in it.\"",
    "\"My brews? Drink one when you need it, not before.\"",
];

pub(super) fn draw_shop(o: &mut Overlay, g: &Game, t: f32) {
    let (w, h) = (o.width, o.height);
    let rows = g.shop_rows();
    let l = layout(&rows, w, h);
    let time = g.time as f32;
    let [x0, y0, pw, ph] = l.panel;
    let small = (t * 0.5).max(11.0);

    // Darken the right side, frame the panel.
    o.rect(x0, y0, pw, ph, [0.03, 0.025, 0.02, 0.96]);
    o.rect_outline(x0, y0, pw, ph, (t * 0.08).max(1.5), rgba(0x8a6a3a, 0.9));
    o.rect_outline(x0 + t * 0.15, y0 + t * 0.15, pw - t * 0.3, ph - t * 0.3, 1.0, rgba(0x5a4428, 0.8));
    let flourish = g.shop.flourish;
    o.text("HROLF THE BLACKSMITH", x0 + t * 0.7, y0 + t * 1.15, t * (1.0 + flourish * 0.08), rgba(mix_rgb(0xf2c078, 0xffffff, flourish * 0.6), 1.0), Align::Left, true);
    // The purse.
    let coins = format!("{}", g.purse);
    let cs = t * 0.95;
    let cx = x0 + pw - t * 0.7;
    o.text(&coins, cx, y0 + t * 1.15, cs, rgba(GOLD, 1.0), Align::Right, true);
    let iw = t * 1.1;
    draw_sprite(o, icon(Icon::Coin), cx - o.measure(&coins, cs) - iw - t * 0.25, y0 + t * 1.15 - iw / 2.0, iw / 16.0, None, 1.0);

    for (r, row) in rows.iter().enumerate() {
        let rect = l.rows[r];
        match row.kind {
            RowKind::Tabs | RowKind::Leave => {}
            RowKind::Weapon(line) => draw_weapon_row(o, g, t, rect, line),
            RowKind::Shield(kind) => draw_shield_row(o, g, t, rect, kind),
            RowKind::Repair => {
                o.rect(rect[0], rect[1], rect[2], rect[3], [1.0, 1.0, 1.0, 0.04]);
                o.text("REPAIR YOUR SHIELD", rect[0] + t * 0.5, rect[1] + rect[3] / 2.0, small, rgba(SAND, 1.0), Align::Left, true);
            }
            RowKind::Potions => {
                let cw = (rect[2] - t * 0.3) / 2.0;
                for (c, b) in row.buttons.iter().enumerate() {
                    if let ShopAction::Potion(kind) = b.action {
                        draw_potion_cell(o, t, [rect[0] + c as f32 * (cw + t * 0.3), rect[1], cw, rect[3]], kind);
                    }
                }
            }
        }
    }

    // The strip over the goods: the belt, or the shield on his arm.
    let s = l.strip;
    match g.shop.tab {
        ShopTab::Potions => {
            o.text("YOUR BELT", s[0], s[1] + s[3] / 2.0, small, rgba(SAND, 1.0), Align::Left, true);
            let slot = t * 1.8;
            let sx = s[0] + o.measure("YOUR BELT", small) + t * 0.6;
            for i in 0..BELT_SLOTS {
                let x = sx + i as f32 * (slot + t * 0.25);
                let y = s[1] + (s[3] - slot) / 2.0;
                o.rect(x, y, slot, slot, [1.0, 1.0, 1.0, 0.06]);
                o.rect_outline(x, y, slot, slot, 1.0, [1.0, 1.0, 1.0, 0.2]);
                if let Some(&k) = g.belt.get(i) {
                    draw_sprite(o, icon(Icon::Potion(k)), x + slot * 0.1, y + slot * 0.1, slot * 0.8 / 16.0, None, 1.0);
                }
            }
            let lives = format!("LIVES {}/{}", g.score.lives, crate::logic::systems::MAX_LIVES);
            o.text(&lives, s[0] + s[2], s[1] + s[3] / 2.0, small, rgba(0xff6d8d, 1.0), Align::Right, true);
        }
        ShopTab::Shields => {
            o.text("ON YOUR ARM", s[0], s[1] + s[3] / 2.0, small, rgba(SAND, 1.0), Align::Left, true);
            let sx = s[0] + o.measure("ON YOUR ARM", small) + t * 0.6;
            match g.hero.shield {
                Some(sh) => {
                    let def = gear::shield(sh.kind);
                    let iw = t * 1.5;
                    draw_sprite(o, icon(Icon::Shield(sh.kind)), sx, s[1] + (s[3] - iw) / 2.0, iw / 16.0, None, 1.0);
                    o.text(&def.name.to_uppercase(), sx + iw + t * 0.4, s[1] + t * 0.55, small, rgba(def.color, 1.0), Align::Left, true);
                    let bx = sx + iw + t * 0.4;
                    bar(o, bx, s[1] + t * 1.15, t * 8.0, t * 0.35, sh.fraction(), if sh.fraction() > 0.35 { 0x9ad1ff } else { 0xff6a4a });
                    o.text(&format!("{} / {}", sh.durability, def.durability), bx + t * 8.4, s[1] + t * 1.3, small * 0.9, rgba(TEXT, 1.0), Align::Left, true);
                }
                None => o.text("NONE - IT BROKE", sx, s[1] + s[3] / 2.0, small, rgba(0xff8a6a, 1.0), Align::Left, true),
            }
        }
        ShopTab::Weapons => {}
    }

    // Buttons.
    let pulse = 0.75 + (time * 4.0).sin() * 0.25;
    for (r, row) in rows.iter().enumerate() {
        for (c, b) in row.buttons.iter().enumerate() {
            let rect = l.buttons[r][c];
            let selected = r == g.shop.row && c == g.shop.col;
            let active_tab = matches!(b.action, ShopAction::Tab(tab) if tab == g.shop.tab);
            let afford = b.price <= g.purse;
            let base = if active_tab { [0.55, 0.38, 0.16, 0.9] } else { [0.02, 0.02, 0.03, 0.85] };
            o.rect(rect[0], rect[1], rect[2], rect[3], base);
            if selected {
                o.rect(rect[0], rect[1], rect[2], rect[3], [1.0, 1.0, 1.0, 0.12]);
            }
            let edge = if selected { rgba(GOLD, pulse) } else { [1.0, 1.0, 1.0, 0.16] };
            o.rect_outline(rect[0], rect[1], rect[2], rect[3], (t * 0.07).max(1.0), edge);
            let color = if !b.enabled {
                0x7f7a70
            } else if b.price > 0 && !afford {
                0xff7a6a
            } else if selected || active_tab {
                0xffffff
            } else {
                TEXT
            };
            let size = if matches!(row.kind, RowKind::Potions) { t * 0.56 } else { t * 0.64 };
            if b.price > 0 {
                // A coin in front of the price.
                let iw = size * 1.1;
                let tw = o.measure(&b.label, size) + iw + t * 0.15;
                let lx = rect[0] + rect[2] / 2.0 - tw / 2.0;
                o.text(&b.label, lx, rect[1] + rect[3] / 2.0, size, rgba(color, 1.0), Align::Left, true);
                draw_sprite(o, icon(Icon::Coin), lx + tw - iw, rect[1] + rect[3] / 2.0 - iw / 2.0, iw / 16.0, None, if afford { 1.0 } else { 0.5 });
            } else {
                o.text(&b.label, rect[0] + rect[2] / 2.0, rect[1] + rect[3] / 2.0, size, rgba(color, 1.0), Align::Center, true);
            }
        }
    }

    // Feedback, or a hint.
    let ny = y0 + ph - t * 2.55;
    match &g.shop.note {
        Some(n) => {
            let a = (n.life * 2.0).min(1.0);
            o.text(&n.text, x0 + pw / 2.0, ny, t * 0.62, rgba(n.color, a), Align::Center, true);
        }
        None => o.text(
            "Arrows or stick to choose   Enter or A to buy   Tab or LT next tab   Esc to leave",
            x0 + pw / 2.0,
            ny,
            small * 0.9,
            rgba(TEXT_DIM, 1.0),
            Align::Center,
            false,
        ),
    }

    // The smith's words, bottom left under the forge.
    let line = SMITH_LINES[match g.shop.tab {
        ShopTab::Weapons => 0,
        ShopTab::Shields => 1,
        ShopTab::Potions => 2,
    }];
    let bw = x0 - t * 1.2;
    panel(o, t * 0.4, h - t * 2.6, bw, t * 2.2, t);
    o.text("HROLF", t * 0.9, h - t * 1.95, small, rgba(0xf2c078, 1.0), Align::Left, true);
    wrap_left(o, line, t * 0.9, h - t * 1.2, bw - t * 1.0, t * 0.6, small * 0.95, rgba(TEXT, 1.0));
}

fn draw_weapon_row(o: &mut Overlay, g: &Game, t: f32, r: [f32; 4], line: Line) {
    let owned = g.arsenal.level(line);
    let equipped = owned > 0 && g.arsenal.equipped == line;
    o.rect(r[0], r[1], r[2], r[3], if equipped { [0.9, 0.7, 0.3, 0.1] } else { [1.0, 1.0, 1.0, 0.04] });
    if equipped {
        o.rect(r[0], r[1], t * 0.12, r[3], rgba(GOLD, 0.9));
    }
    // The three levels, lit up to what he owns.
    let iw = t * 3.2;
    for level in 1..=3u8 {
        let x = r[0] + t * 0.4 + (level - 1) as f32 * (iw + t * 0.3);
        let y = r[1] + t * 0.45;
        o.rect(x - 2.0, y - 2.0, iw + 4.0, iw + 4.0, [0.0, 0.0, 0.0, 0.4]);
        let (tint, alpha) = if level <= owned {
            (None, 1.0)
        } else if level == owned + 1 {
            (None, 0.45)
        } else {
            (Some(0x2a2a30), 0.8)
        };
        draw_sprite(o, icon(Icon::Weapon(line, level)), x, y, iw / 16.0, tint, alpha);
        let label = format!("{}", level);
        o.text(&label, x + iw / 2.0, y + iw + t * 0.5, t * 0.46, rgba(if level <= owned { GOLD } else { TEXT_DIM }, 1.0), Align::Center, true);
    }
    let tx = r[0] + t * 0.4 + 3.0 * (iw + t * 0.3) + t * 0.3;
    let shown = owned.max(1);
    let size = t * 0.75;
    o.text(&line.name(shown).to_uppercase(), tx, r[1] + t * 0.8, size, rgba(if owned > 0 { line.color(shown) } else { TEXT_DIM }, 1.0), Align::Left, true);
    let status = if owned == 0 {
        "NOT OWNED".to_string()
    } else if equipped {
        format!("LEVEL {owned}  IN HAND")
    } else {
        format!("LEVEL {owned}")
    };
    o.text(&status, tx, r[1] + t * 1.6, t * 0.5, rgba(if equipped { GOLD } else { SAND }, 1.0), Align::Left, true);
    // The whole line at a glance: level names, lit up to what he owns.
    let mut nx = tx;
    for level in 1..=3u8 {
        let name = format!("{} {}", level, line.name(level).to_uppercase());
        let c = if level <= owned { line.color(level) } else { 0x6a665e };
        o.text(&name, nx, r[1] + t * 2.3, t * 0.42, rgba(c, 1.0), Align::Left, false);
        nx += o.measure(&name, t * 0.42) + t * 0.6;
    }
    // What the next level brings, or what this one does.
    let (label, blurb) = if owned == 0 {
        ("", line.blurb(1))
    } else if owned < 3 {
        ("NEXT: ", line.blurb(owned + 1))
    } else {
        ("", line.blurb(3))
    };
    let text = format!("{label}{blurb}");
    let tw = r[0] + r[2] - t * 7.4 - tx;
    wrap_left(o, &text, tx, r[1] + t * 3.1, tw, t * 0.62, t * 0.5, rgba(TEXT, 0.85));
}

fn draw_shield_row(o: &mut Overlay, g: &Game, t: f32, r: [f32; 4], kind: gear::ShieldKind) {
    let def = gear::shield(kind);
    let mine = g.hero.shield.map(|s| s.kind == kind).unwrap_or(false);
    o.rect(r[0], r[1], r[2], r[3], if mine { [0.5, 0.7, 0.9, 0.1] } else { [1.0, 1.0, 1.0, 0.04] });
    let iw = t * 2.9;
    draw_sprite(o, icon(Icon::Shield(kind)), r[0] + t * 0.4, r[1] + (r[3] - iw) / 2.0, iw / 16.0, None, 1.0);
    let tx = r[0] + t * 0.4 + iw + t * 0.6;
    o.text(&def.name.to_uppercase(), tx, r[1] + t * 0.8, t * 0.72, rgba(def.color, 1.0), Align::Left, true);
    o.text(&format!("BLOCKS {}%   DURABILITY {}", (def.block * 100.0).round(), def.durability), tx, r[1] + t * 1.65, t * 0.5, rgba(SAND, 1.0), Align::Left, true);
    o.text(def.blurb, tx, r[1] + t * 2.5, t * 0.48, rgba(TEXT, 0.85), Align::Left, false);
}

fn draw_potion_cell(o: &mut Overlay, t: f32, r: [f32; 4], kind: crate::logic::types::PotionKind) {
    let def = gear::potion(kind);
    o.rect(r[0], r[1], r[2], r[3], [1.0, 1.0, 1.0, 0.04]);
    let iw = t * 2.5;
    draw_sprite(o, icon(Icon::Potion(kind)), r[0] + t * 0.3, r[1] + (r[3] - iw) / 2.0, iw / 16.0, None, 1.0);
    let tx = r[0] + t * 0.3 + iw + t * 0.4;
    o.text(&def.name.to_uppercase(), tx, r[1] + t * 0.6, t * 0.6, rgba(def.color, 1.0), Align::Left, true);
    let tw = r[2] - (tx - r[0]) - t * 4.8;
    wrap_left(o, def.blurb, tx, r[1] + t * 1.25, tw, t * 0.5, t * 0.42, rgba(TEXT, 0.85));
}
