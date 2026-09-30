//! The old gatekeeper's wares: a panel on the right over the 3D crypt, with
//! a tab for each of the hero's four trees, the tree's ten tiers as a column
//! of rows, a card telling the highlighted tier's legend, ability and price,
//! and a line of feedback. `shop_rects` gives the button rectangles (and
//! `shop_row_at` the rows) so the game can hit-test the same layout it draws.

use super::*;
use crate::game::{RowKind, ShopAction, ShopRow};
use crate::logic::gear::BELT_SLOTS;
use crate::logic::progression::{self, Tree, TIERS};
use crate::logic::story::roman;
use crate::logic::systems::MAX_LIVES;
use crate::logic::types::PotionKind;
use crate::render::icons::{icon, Icon};

struct Layout {
    /// Panel x, y, width, height.
    panel: [f32; 4],
    rows: Vec<[f32; 4]>,
    buttons: Vec<Vec<[f32; 4]>>,
    /// The card about the highlighted row.
    card: [f32; 4],
}

fn layout(rows: &[ShopRow], w: f32, h: f32) -> Layout {
    let t = tile_for(w, h);
    let margin = t * 0.4;
    let x0 = (w * 0.41).max(w - margin - t * 36.0);
    let pw = w - margin - x0;
    let (y0, ph) = (margin, h - margin * 2.0);
    let pad = t * 0.6;
    let inner = pw - pad * 2.0;
    let rh = t * 1.5;
    let first = y0 + t * 4.9;
    let mut out_rows = Vec::new();
    let mut buttons = Vec::new();
    let mut i = 0;
    for row in rows {
        let (rect, b): ([f32; 4], Vec<[f32; 4]>) = match row.kind {
            RowKind::Tabs => {
                let gap = t * 0.25;
                let n = row.buttons.len().max(1) as f32;
                let bw = (inner - gap * (n - 1.0)) / n;
                let r = [x0 + pad, y0 + t * 2.3, inner, t * 1.3];
                (r, (0..row.buttons.len()).map(|k| [r[0] + k as f32 * (bw + gap), r[1], bw, r[3]]).collect())
            }
            RowKind::Tier(..) | RowKind::Repair | RowKind::Life => {
                let extra = if matches!(row.kind, RowKind::Tier(..)) { 0.0 } else { t * 0.15 };
                let r = [x0 + pad, first + i as f32 * (rh + t * 0.08) + extra, inner, rh];
                i += 1;
                let bw = t * 7.4;
                (r, vec![[r[0] + r[2] - bw - t * 0.2, r[1] + (rh - t * 1.15) / 2.0, bw, t * 1.15]])
            }
            RowKind::Leave => {
                let bw = t * 10.0;
                let r = [x0 + pad, y0 + ph - t * 1.9, inner, t * 1.4];
                (r, vec![[x0 + pw / 2.0 - bw / 2.0, r[1], bw, r[3]]])
            }
        };
        out_rows.push(rect);
        buttons.push(b);
    }
    let card_top = first + 11.0 * (rh + t * 0.08) + t * 0.3;
    let card = [x0 + pad, card_top, inner, (y0 + ph - t * 3.2 - card_top).max(t * 5.0)];
    Layout { panel: [x0, y0, pw, ph], rows: out_rows, buttons, card }
}

/// Button rectangles per row, for hit-testing the shop.
pub fn shop_rects(rows: &[ShopRow], w: f32, h: f32) -> Vec<Vec<[f32; 4]>> {
    layout(rows, w, h).buttons
}

/// The tier (or extra) row under the pointer, if any.
pub fn shop_row_at(rows: &[ShopRow], w: f32, h: f32, p: (f32, f32)) -> Option<usize> {
    let l = layout(rows, w, h);
    rows.iter().enumerate().position(|(i, r)| {
        let b = l.rows[i];
        matches!(r.kind, RowKind::Tier(..) | RowKind::Repair | RowKind::Life)
            && p.0 >= b[0]
            && p.0 <= b[0] + b[2]
            && p.1 >= b[1]
            && p.1 <= b[1] + b[3]
    })
}

/// What the old man says, by the tab (or because he was called).
fn old_man_line(g: &Game) -> &'static str {
    if g.shop.called {
        return "\"You called, and I came. I told you I would. What do you need?\"";
    }
    match g.shop.tab {
        Tree::Armory => "\"Old blades of old heroes. I kept them for the ones brave enough to go down.\"",
        Tree::Path => "\"The soul you carry remembers how he fought. Let him teach you, a step at a time.\"",
        Tree::Shield => "\"Down there a good shield is worth more than a brave heart. Take one.\"",
        Tree::Elixir => "\"Drink when you need it, boy. Not before.\"",
    }
}

pub(super) fn draw_shop(o: &mut Overlay, g: &Game, t: f32) {
    let (w, h) = (o.width, o.height);
    let rows = g.shop_rows();
    let l = layout(&rows, w, h);
    let time = g.time as f32;
    let [x0, y0, pw, ph] = l.panel;
    let small = (t * 0.5).max(11.0);
    let hero = g.hero_kind;
    let prog = g.progress.hero(hero);
    let tree = g.shop.tab;

    // The panel.
    o.rect(x0, y0, pw, ph, [0.03, 0.03, 0.04, 0.95]);
    o.rect_outline(x0, y0, pw, ph, (t * 0.08).max(1.5), rgba(0x6a7a9a, 0.9));
    o.rect_outline(x0 + t * 0.15, y0 + t * 0.15, pw - t * 0.3, ph - t * 0.3, 1.0, rgba(0x3a4458, 0.8));
    let flourish = g.shop.flourish;
    o.text(
        "THE OLD GATEKEEPER",
        x0 + t * 0.7,
        y0 + t * 1.15,
        t * (0.95 + flourish * 0.08),
        rgba(mix_rgb(0xa8c4e8, 0xffffff, flourish * 0.6), 1.0),
        Align::Left,
        true,
    );
    // Silver and runes.
    let cs = t * 0.85;
    let mut cx = x0 + pw - t * 0.7;
    let runes = format!("{}", g.progress.runes);
    o.text(&runes, cx, y0 + t * 1.15, cs, rgba(0x9ad1ff, 1.0), Align::Right, true);
    let iw = t * 1.0;
    cx -= o.measure(&runes, cs) + iw + t * 0.15;
    draw_sprite(o, icon(Icon::Gem(g.hero_kind)), cx, y0 + t * 1.15 - iw / 2.0, iw / 16.0, None, 1.0);
    cx -= t * 0.7;
    let silver = format!("{}", g.progress.silver);
    o.text(&silver, cx, y0 + t * 1.15, cs, rgba(GOLD, 1.0), Align::Right, true);
    cx -= o.measure(&silver, cs) + iw + t * 0.15;
    draw_sprite(o, icon(Icon::Money(g.hero_kind)), cx, y0 + t * 1.15 - iw / 2.0, iw / 16.0, None, 1.0);

    // The tree's name and how far along it he is.
    let ty = y0 + t * 4.25;
    o.text(&tree.title(hero).to_uppercase(), x0 + t * 0.7, ty, small, rgba(GOLD, 1.0), Align::Left, true);
    o.text(&format!("{} / {}", prog.owned(tree), TIERS), x0 + pw - t * 0.7, ty, small, rgba(SAND, 1.0), Align::Right, true);

    // The rows.
    let pulse = 0.75 + (time * 4.0).sin() * 0.25;
    for (r, row) in rows.iter().enumerate() {
        let rect = l.rows[r];
        let selected = r == g.shop.row;
        match row.kind {
            RowKind::Tier(tr, tier) => {
                let n = progression::node(hero, tr, tier);
                let owned = prog.has(tr, tier);
                let next = prog.next(tr) == Some(tier);
                let bg = if owned {
                    [0.55, 0.45, 0.25, 0.14]
                } else if next {
                    [0.4, 0.55, 0.8, 0.12]
                } else {
                    [1.0, 1.0, 1.0, 0.03]
                };
                o.rect(rect[0], rect[1], rect[2], rect[3], bg);
                if selected {
                    o.rect_outline(rect[0], rect[1], rect[2], rect[3], (t * 0.06).max(1.0), rgba(GOLD, pulse));
                }
                let mut x = rect[0] + t * 0.3;
                let cy = rect[1] + rect[3] / 2.0;
                o.text(
                    &roman(tier as usize),
                    x + t * 0.55,
                    cy,
                    small * 0.9,
                    rgba(if owned { GOLD } else { TEXT_DIM }, 1.0),
                    Align::Center,
                    true,
                );
                x += t * 1.3;
                let iw = t * 1.2;
                let (tint, alpha) = if owned || next { (None, 1.0) } else { (Some(0x2a2a30), 0.8) };
                draw_sprite(o, icon(Icon::Node(hero, tr, tier)), x, cy - iw / 2.0, iw / 16.0, tint, alpha);
                x += iw + t * 0.35;
                let name = n.name.to_uppercase();
                let room = rect[0] + rect[2] - t * 7.9 - x;
                let size = if o.measure(&name, small) > room { small * 0.82 } else { small };
                let color = if owned || next { n.color } else { 0x6a665e };
                o.text(&name, x, cy, size, rgba(color, 1.0), Align::Left, true);
                if progression::sealed(tier) && !owned {
                    let rw = t * 0.7;
                    let found = prog.found(tr, tier);
                    draw_sprite(
                        o,
                        icon(Icon::Gem(g.hero_kind)),
                        x + o.measure(&name, size) + t * 0.25,
                        cy - rw / 2.0,
                        rw / 16.0,
                        if found { None } else { Some(0x4a5060) },
                        1.0,
                    );
                }
            }
            RowKind::Repair => {
                o.rect(rect[0], rect[1], rect[2], rect[3], [1.0, 1.0, 1.0, 0.05]);
                if selected {
                    o.rect_outline(rect[0], rect[1], rect[2], rect[3], (t * 0.06).max(1.0), rgba(GOLD, pulse));
                }
                let cy = rect[1] + rect[3] / 2.0;
                o.text("MEND YOUR SHIELD", rect[0] + t * 0.5, cy - t * 0.25, small, rgba(SAND, 1.0), Align::Left, true);
                let frac = g.hero.shield.map(|s| s.fraction()).unwrap_or(0.0);
                bar(o, rect[0] + t * 0.5, cy + t * 0.25, t * 7.0, t * 0.26, frac, if frac > 0.35 { 0x9ad1ff } else { 0xff6a4a });
            }
            RowKind::Life => {
                o.rect(rect[0], rect[1], rect[2], rect[3], [1.0, 1.0, 1.0, 0.05]);
                if selected {
                    o.rect_outline(rect[0], rect[1], rect[2], rect[3], (t * 0.06).max(1.0), rgba(GOLD, pulse));
                }
                let cy = rect[1] + rect[3] / 2.0;
                let iw = t * 1.2;
                draw_sprite(o, icon(Icon::Potion(hero, PotionKind::Life)), rect[0] + t * 1.6, cy - iw / 2.0, iw / 16.0, None, 1.0);
                let lives = format!("EXTRA LIFE   {}/{}", g.score.lives, MAX_LIVES);
                o.text(&lives, rect[0] + t * 3.15, cy, small, rgba(0xff6d8d, 1.0), Align::Left, true);
            }
            RowKind::Tabs | RowKind::Leave => {}
        }
    }

    // Buttons.
    for (r, row) in rows.iter().enumerate() {
        for (c, b) in row.buttons.iter().enumerate() {
            let rect = l.buttons[r][c];
            let selected = r == g.shop.row && (c == g.shop.col || row.kind != RowKind::Tabs);
            let active_tab = matches!(b.action, ShopAction::Tab(tab) if tab == g.shop.tab);
            let afford = b.price <= g.progress.silver && b.runes <= g.progress.runes;
            let base = if active_tab { [0.25, 0.32, 0.5, 0.95] } else { [0.02, 0.02, 0.03, 0.85] };
            o.rect(rect[0], rect[1], rect[2], rect[3], base);
            if selected && b.enabled {
                o.rect(rect[0], rect[1], rect[2], rect[3], [1.0, 1.0, 1.0, 0.12]);
            }
            let edge = if selected && (row.kind == RowKind::Tabs || row.kind == RowKind::Leave) {
                rgba(GOLD, pulse)
            } else {
                [1.0, 1.0, 1.0, 0.16]
            };
            o.rect_outline(rect[0], rect[1], rect[2], rect[3], (t * 0.06).max(1.0), edge);
            let color = if !b.enabled {
                0x7f7a70
            } else if (b.price > 0 || b.runes > 0) && !afford {
                0xff7a6a
            } else if selected || active_tab {
                0xffffff
            } else {
                TEXT
            };
            let size = if row.kind == RowKind::Tabs { t * 0.58 } else { t * 0.5 };
            let cy = rect[1] + rect[3] / 2.0;
            if b.price > 0 || b.runes > 0 {
                // The price, a coin, and the runes too for a sealed tier.
                let iw = size * 1.15;
                let rune_part = if b.runes > 0 { format!("+{}", b.runes) } else { String::new() };
                let mut tw = o.measure(&b.label, size) + iw + t * 0.12;
                if b.runes > 0 {
                    tw += t * 0.2 + o.measure(&rune_part, size) + iw;
                }
                let mut lx = rect[0] + rect[2] / 2.0 - tw / 2.0;
                o.text(&b.label, lx, cy, size, rgba(color, 1.0), Align::Left, true);
                lx += o.measure(&b.label, size) + t * 0.12;
                draw_sprite(o, icon(Icon::Money(g.hero_kind)), lx, cy - iw / 2.0, iw / 16.0, None, if afford { 1.0 } else { 0.5 });
                if b.runes > 0 {
                    lx += iw + t * 0.2;
                    o.text(
                        &rune_part,
                        lx,
                        cy,
                        size,
                        rgba(if g.progress.runes >= b.runes { 0x9ad1ff } else { 0xff7a6a }, 1.0),
                        Align::Left,
                        true,
                    );
                    lx += o.measure(&rune_part, size);
                    draw_sprite(o, icon(Icon::Gem(g.hero_kind)), lx, cy - iw / 2.0, iw / 16.0, None, 1.0);
                }
            } else {
                o.text(&b.label, rect[0] + rect[2] / 2.0, cy, size, rgba(color, 1.0), Align::Center, true);
            }
        }
    }

    draw_card(o, g, t, l.card, &rows);

    // Feedback, or a hint.
    let ny = y0 + ph - t * 2.55;
    match &g.shop.note {
        Some(n) => {
            let a = (n.life * 2.0).min(1.0);
            o.text(&n.text, x0 + pw / 2.0, ny, t * 0.58, rgba(n.color, a), Align::Center, true);
        }
        None => o.text(
            "Arrows or stick to choose   Enter or A to take   Tab or LT next tree   Esc to leave",
            x0 + pw / 2.0,
            ny,
            small * 0.85,
            rgba(TEXT_DIM, 1.0),
            Align::Center,
            false,
        ),
    }

    // The old man's words, bottom left under the crypt.
    let bw = x0 - t * 1.2;
    panel(o, t * 0.4, h - t * 2.6, bw, t * 2.2, t);
    o.text("THE OLD MAN", t * 0.9, h - t * 1.95, small, rgba(0xa8c4e8, 1.0), Align::Left, true);
    wrap_left(o, old_man_line(g), t * 0.9, h - t * 1.2, bw - t * 1.0, t * 0.6, small * 0.95, rgba(TEXT, 1.0));
}

/// The card for the highlighted row: the tier's legend, what it does, and what it takes.
fn draw_card(o: &mut Overlay, g: &Game, t: f32, c: [f32; 4], rows: &[ShopRow]) {
    let hero = g.hero_kind;
    let prog = g.progress.hero(hero);
    let small = (t * 0.5).max(11.0);
    o.rect(c[0], c[1], c[2], c[3], [0.0, 0.0, 0.0, 0.35]);
    o.rect_outline(c[0], c[1], c[2], c[3], 1.0, [1.0, 1.0, 1.0, 0.12]);
    let kind = rows.get(g.shop.row).map(|r| r.kind).unwrap_or(RowKind::Tabs);
    let (tree, tier) = match kind {
        RowKind::Tier(tr, tier) => (tr, tier),
        RowKind::Repair => (Tree::Shield, g.hero.shield.map(|s| s.tier).unwrap_or(prog.carry)),
        _ => {
            let open = g.shop.tab;
            (open, prog.next(open).unwrap_or(TIERS))
        }
    };
    let n = progression::node(hero, tree, tier);
    let pad = t * 0.5;
    let iw = t * 3.4;
    o.rect(c[0] + pad - 2.0, c[1] + pad - 2.0, iw + 4.0, iw + 4.0, [1.0, 1.0, 1.0, 0.05]);
    draw_sprite(o, icon(Icon::Node(hero, tree, tier)), c[0] + pad, c[1] + pad, iw / 16.0, None, 1.0);
    let tx = c[0] + pad + iw + t * 0.5;
    let tw = c[0] + c[2] - pad - tx;
    let name = n.name.to_uppercase();
    let size = if o.measure(&name, t * 0.8) > tw { t * 0.62 } else { t * 0.8 };
    o.text(&name, tx, c[1] + pad + t * 0.4, size, rgba(n.color, 1.0), Align::Left, true);
    o.text(
        &format!("{} {}  -  {}", tree.step(), roman(tier as usize), tree.title(hero).to_uppercase()),
        tx,
        c[1] + pad + t * 1.2,
        small * 0.85,
        rgba(SAND, 1.0),
        Align::Left,
        false,
    );
    let ly = wrap_left(o, n.lore, tx, c[1] + pad + t * 1.95, tw, t * 0.55, small * 0.88, rgba(TEXT_DIM, 1.0));
    let ay = (ly + t * 0.25).max(c[1] + pad + iw + t * 0.3);
    o.text(&n.ability.to_uppercase(), c[0] + pad, ay, small, rgba(GOLD, 1.0), Align::Left, true);
    let ey = wrap_left(o, n.effect, c[0] + pad, ay + t * 0.7, c[2] - pad * 2.0, t * 0.58, small * 0.95, rgba(TEXT, 1.0));
    if tree == Tree::Shield {
        let wd = progression::ward(tier);
        let extra = if tier >= 4 { "   KEEPS EVERY GIFT OF THE SHIELDS BEFORE IT" } else { "" };
        o.text(&format!("DURABILITY {}{extra}", wd.durability), c[0] + pad, ey + t * 0.1, small * 0.8, rgba(SAND, 1.0), Align::Left, false);
    }

    // Its standing: owned, the way to it, or what stands in the way.
    let owned = prog.has(tree, tier);
    let status = if owned {
        match tree {
            Tree::Armory if prog.wield == tier => ("IN YOUR HAND".to_string(), GOLD),
            Tree::Shield if g.hero.shield.map(|s| s.tier) == Some(tier) => ("ON YOUR ARM".to_string(), GOLD),
            Tree::Elixir => (
                format!(
                    "BREWED - {} {} A BOTTLE",
                    progression::bottle_price(progression::ELIXIRS[(tier - 1) as usize]),
                    progression::coin(hero)
                ),
                0x7dff9a,
            ),
            _ => ("YOURS".to_string(), 0x7dff9a),
        }
    } else if prog.found(tree, tier) {
        if prog.next(tree) == Some(tier) {
            ("ITS RELIC IS FOUND - UNSEAL IT FOR NOTHING".to_string(), 0x9ad1ff)
        } else {
            ("ITS RELIC IS FOUND - IT IS YOURS ONCE THE TIERS BEFORE IT ARE".to_string(), 0x9ad1ff)
        }
    } else if prog.next(tree) != Some(tier) {
        (format!("TAKE {} {} FIRST", tree.step(), roman(tier as usize - 1)), TEXT_DIM)
    } else if progression::sealed(tier) {
        let p = progression::price(tree, tier);
        (
            format!(
                "SEALED - {} {} AND {} {}, OR FIND ITS RELIC",
                p.silver,
                progression::coin(hero),
                p.runes,
                progression::gems(hero, p.runes)
            ),
            0x9ad1ff,
        )
    } else {
        (format!("{} {}", progression::price(tree, tier).silver, progression::coin(hero)), GOLD)
    };
    let sy = c[1] + c[3] - pad - if g.shop.tab == Tree::Elixir { t * 2.0 } else { t * 0.2 };
    o.text(&status.0, c[0] + pad, sy, small * 0.9, rgba(status.1, 1.0), Align::Left, true);
    if !owned && progression::sealed(tier) && !prog.found(tree, tier) && prog.next(tree) == Some(tier) {
        let hint = format!("Relics wait in {}s and on fallen Cyclopes.", progression::hidden_place(hero));
        o.text(&hint, c[0] + pad, sy - t * 0.62, small * 0.8, rgba(TEXT_DIM, 1.0), Align::Left, false);
    }

    // On the elixirs, the belt at the foot of the card.
    if g.shop.tab == Tree::Elixir {
        let slot = t * 1.3;
        let by = c[1] + c[3] - pad - slot;
        o.text("YOUR BELT", c[0] + pad, by + slot / 2.0, small * 0.9, rgba(SAND, 1.0), Align::Left, true);
        let bx = c[0] + pad + o.measure("YOUR BELT", small * 0.9) + t * 0.5;
        for i in 0..BELT_SLOTS {
            let x = bx + i as f32 * (slot + t * 0.2);
            o.rect(x, by, slot, slot, [1.0, 1.0, 1.0, 0.06]);
            o.rect_outline(x, by, slot, slot, 1.0, [1.0, 1.0, 1.0, 0.2]);
            if let Some(&k) = g.belt.get(i) {
                draw_sprite(o, icon(Icon::Potion(hero, k)), x + slot * 0.1, by + slot * 0.1, slot * 0.8 / 16.0, None, 1.0);
            }
        }
    }
}
