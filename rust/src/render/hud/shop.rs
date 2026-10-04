//! The old gatekeeper's workshop: a panel on the right over the 3D room, named
//! for the station the hero stands at, with a tab for each station (and the
//! loadout), the station's tree's ten tiers as a column of rows (or the war
//! table's generals, or the trophy wall's hooks), a card telling the
//! highlighted row's legend, ability and price, and a line of feedback.
//! `shop_rects` gives the button rectangles (and `shop_row_at` the rows) so
//! the game can hit-test the same layout it draws.

use super::*;
use crate::game::{RowKind, ShopAction, ShopRow, Station};
use crate::logic::angels::ARCHANGELS;
use crate::logic::gear::{self, BELT_SLOTS};
use crate::logic::progression::{self, Tree, TIERS};
use crate::logic::sins::SINS;
use crate::logic::story::roman;
use crate::logic::systems::MAX_LIVES;
use crate::logic::types::PotionKind;
use crate::render::heroes::outfit_names;
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
    let mut y = first;
    let mut wide_rows = false;
    for row in rows {
        let (rect, b): ([f32; 4], Vec<[f32; 4]>) = match row.kind {
            RowKind::Tabs => {
                let gap = t * 0.25;
                let n = row.buttons.len().max(1) as f32;
                let bw = (inner - gap * (n - 1.0)) / n;
                let r = [x0 + pad, y0 + t * 2.3, inner, t * 1.3];
                (r, (0..row.buttons.len()).map(|k| [r[0] + k as f32 * (bw + gap), r[1], bw, r[3]]).collect())
            }
            RowKind::Tier(..)
            | RowKind::Repair
            | RowKind::Life
            | RowKind::Sharpen
            | RowKind::Trials
            | RowKind::General(_)
            | RowKind::Rival(_)
            | RowKind::Trophy(_) => {
                let extra = if matches!(row.kind, RowKind::Tier(..) | RowKind::General(_) | RowKind::Rival(_) | RowKind::Trophy(_)) {
                    0.0
                } else {
                    t * 0.15
                };
                let r = [x0 + pad, y + extra, inner, rh];
                y += rh + t * 0.08;
                let bw = t * 7.4;
                (r, vec![[r[0] + r[2] - bw - t * 0.2, r[1] + (rh - t * 1.15) / 2.0, bw, t * 1.15]])
            }
            RowKind::Patron | RowKind::Outfit => {
                // A title over a row of choices.
                wide_rows = true;
                let r = [x0 + pad, y + t * 0.1, inner, t * 2.6];
                y += t * 2.85;
                let gap = t * 0.2;
                let n = row.buttons.len().max(1) as f32;
                let bw = (inner - t * 0.4 - gap * (n - 1.0)) / n;
                (r, (0..row.buttons.len()).map(|k| [r[0] + t * 0.2 + k as f32 * (bw + gap), r[1] + t * 1.25, bw, t * 1.1]).collect())
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
    // The loadout's rows are fewer and taller: its card follows them.
    let card_top = if wide_rows { y + t * 0.4 } else { first + 11.0 * (rh + t * 0.08) + t * 0.3 };
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
        matches!(
            r.kind,
            RowKind::Tier(..)
                | RowKind::Repair
                | RowKind::Life
                | RowKind::Sharpen
                | RowKind::Patron
                | RowKind::Outfit
                | RowKind::Trials
                | RowKind::General(_)
                | RowKind::Rival(_)
                | RowKind::Trophy(_)
        ) && p.0 >= b[0]
            && p.0 <= b[0] + b[2]
            && p.1 >= b[1]
            && p.1 <= b[1] + b[3]
    })
}

/// What the old man says: his counsel on what just happened down there (a fall and its answer, a
/// whisper denied, a lie seen through: `Game::workshop_counsel`), the news at the war table, or
/// else the station's own line (or that he was called).
fn old_man_line(g: &Game) -> String {
    if g.station() == Station::WarTable {
        if let Some(news) = g.workshop.news.first() {
            return format!("\"{news}\"");
        }
    }
    // At the yard with a trial set, he says what it is.
    if g.station() == Station::Yard && !g.shop.loadout {
        if let Some(tr) = g.trial_ready() {
            return format!("\"{}\"", tr.line);
        }
    }
    if let Some(said) = g.workshop_counsel() {
        return format!("\"{said}\"");
    }
    let line = if g.shop.called {
        "\"You called, and I came. I told you I would. What do you need?\""
    } else if g.shop.loadout {
        "\"Swear to an angel, dress as you like, and keep that blade sharp. Rust has killed more heroes than monsters have.\""
    } else {
        match g.station() {
            Station::Anvil => "\"Old blades of old heroes. I kept them for the ones brave enough to go down.\"",
            Station::Yard => "\"The soul you carry remembers how he fought. Let him teach you, a step at a time.\"",
            Station::ShieldWall => "\"Down there a good shield is worth more than a brave heart. Take one.\"",
            Station::Bench => "\"Drink when you need it, boy. Not before.\"",
            Station::WarTable => "\"Sit. When I hear what waits below, and who commands it, I will tell you here.\"",
            Station::Trophies => "\"Hang what you take from them here. Let the next one see it.\"",
        }
    };
    line.to_string()
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
    // Charcoal, a copper edge and a fine pale-stone line inside it.
    o.rect(x0, y0, pw, ph, [INK[0], INK[1], INK[2], 0.95]);
    o.rect_outline(x0, y0, pw, ph, (t * 0.08).max(1.5), rgba(COPPER, 0.9));
    o.rect_outline(x0 + t * 0.15, y0 + t * 0.15, pw - t * 0.3, ph - t * 0.3, 1.0, [EDGE[0], EDGE[1], EDGE[2], 0.18]);
    let flourish = g.shop.flourish;
    let station = g.station();
    o.text(
        station.name(),
        x0 + t * 0.7,
        y0 + t * 1.15,
        t * (0.95 + flourish * 0.08),
        rgba(mix_rgb(GOLD, 0xffffff, flourish * 0.6), 1.0),
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

    // The tree's name and how far along it he is (on the loadout: whose it is; on a board, what
    // hangs there).
    let ty = y0 + t * 4.25;
    if g.shop.loadout {
        o.text("LOADOUT", x0 + t * 0.7, ty, small, rgba(GOLD, 1.0), Align::Left, true);
        o.text(&hero.name().to_uppercase(), x0 + pw - t * 0.7, ty, small, rgba(SAND, 1.0), Align::Right, true);
    } else if station == Station::WarTable {
        let known = g.workshop.generals.iter().filter(|c| c.is_some()).count();
        o.text("HELL'S GENERALS", x0 + t * 0.7, ty, small, rgba(GOLD, 1.0), Align::Left, true);
        o.text(&format!("{known} / 7"), x0 + pw - t * 0.7, ty, small, rgba(SAND, 1.0), Align::Right, true);
    } else if station == Station::Trophies {
        o.text("WHAT YOU TOOK FROM BELOW", x0 + t * 0.7, ty, small, rgba(GOLD, 1.0), Align::Left, true);
        let n = g.workshop.trophies.len().min(crate::game::workshop::TROPHY_SLOTS);
        o.text(
            &format!("{n} / {}", crate::game::workshop::TROPHY_SLOTS),
            x0 + pw - t * 0.7,
            ty,
            small,
            rgba(SAND, 1.0),
            Align::Right,
            true,
        );
    } else {
        o.text(&tree.title(hero).to_uppercase(), x0 + t * 0.7, ty, small, rgba(GOLD, 1.0), Align::Left, true);
        o.text(&format!("{} / {}", prog.owned(tree), TIERS), x0 + pw - t * 0.7, ty, small, rgba(SAND, 1.0), Align::Right, true);
    }

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
                    [0.43, 0.6, 0.56, 0.12]
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
                if g.hero.shield.is_none() && prog.is_broken(prog.carry) {
                    o.text("REFORGE YOUR SHIELD", rect[0] + t * 0.5, cy - t * 0.25, small, rgba(SAND, 1.0), Align::Left, true);
                    o.text("DESTROYED", rect[0] + t * 0.5, cy + t * 0.3, small * 0.85, rgba(0xff8a6a, 1.0), Align::Left, true);
                } else {
                    o.text("MEND YOUR SHIELD", rect[0] + t * 0.5, cy - t * 0.25, small, rgba(SAND, 1.0), Align::Left, true);
                    let frac = g.hero.shield.map(|s| s.fraction()).unwrap_or(0.0);
                    bar(o, rect[0] + t * 0.5, cy + t * 0.25, t * 7.0, t * 0.26, frac, if frac > 0.35 { 0x9ad1ff } else { 0xff6a4a });
                }
            }
            RowKind::Sharpen => {
                o.rect(rect[0], rect[1], rect[2], rect[3], [1.0, 1.0, 1.0, 0.05]);
                if selected {
                    o.rect_outline(rect[0], rect[1], rect[2], rect[3], (t * 0.06).max(1.0), rgba(GOLD, pulse));
                }
                let cy = rect[1] + rect[3] / 2.0;
                let edge = prog.sharpness(prog.wield);
                o.text("SHARPEN YOUR BLADE", rect[0] + t * 0.5, cy - t * 0.25, small, rgba(SAND, 1.0), Align::Left, true);
                let c = if edge >= gear::DULL_EDGE {
                    0xe8c860
                } else if edge >= gear::RUSTED_EDGE {
                    0xd88a3a
                } else {
                    0xc84a2a
                };
                bar(o, rect[0] + t * 0.5, cy + t * 0.25, t * 7.0, t * 0.26, edge as f32, c);
            }
            RowKind::Patron | RowKind::Outfit => {
                o.rect(rect[0], rect[1], rect[2], rect[3], [1.0, 1.0, 1.0, 0.05]);
                if selected {
                    o.rect_outline(rect[0], rect[1], rect[2], rect[3], (t * 0.06).max(1.0), rgba(GOLD, pulse));
                }
                let ty = rect[1] + t * 0.6;
                let (title, now, color) = if row.kind == RowKind::Patron {
                    match prog.patron {
                        Some(a) => ("ARCHANGEL'S PROTECTION", format!("{} WATCHES OVER YOU", a.name().to_uppercase()), a.color()),
                        None => ("ARCHANGEL'S PROTECTION", "NONE SWORN".to_string(), TEXT_DIM),
                    }
                } else {
                    ("OUTFIT", outfit_names(hero)[prog.outfit.min(3) as usize].to_uppercase(), 0xe8dcc0)
                };
                o.text(title, rect[0] + t * 0.5, ty, small, rgba(SAND, 1.0), Align::Left, true);
                o.text(&now, rect[0] + rect[2] - t * 0.5, ty, small * 0.9, rgba(color, 1.0), Align::Right, true);
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
            RowKind::Trials => {
                o.rect(rect[0], rect[1], rect[2], rect[3], [1.0, 1.0, 1.0, 0.03]);
                if selected {
                    o.rect_outline(rect[0], rect[1], rect[2], rect[3], (t * 0.06).max(1.0), rgba(GOLD, pulse));
                }
                let cy = rect[1] + rect[3] / 2.0;
                let mst = prog.mastery;
                let way = crate::logic::war::mastery::way(hero);
                let lit = if g.trial_ready().is_some() { GOLD } else { SAND };
                o.text("MASTERY", rect[0] + t * 0.5, cy - t * 0.25, small, rgba(lit, 1.0), Align::Left, true);
                o.text(
                    &format!("{}  {}/{}", way.name, mst.rank, crate::logic::war::mastery::RANKS),
                    rect[0] + t * 0.5,
                    cy + t * 0.3,
                    small * 0.78,
                    rgba(TEXT_DIM, 1.0),
                    Align::Left,
                    false,
                );
            }
            RowKind::General(i) => {
                let sin = SINS[(i as usize).min(6)];
                let card = g.workshop.generals.get(i as usize).and_then(|c| c.as_ref());
                o.rect(rect[0], rect[1], rect[2], rect[3], if card.is_some() { [0.55, 0.45, 0.25, 0.14] } else { [1.0, 1.0, 1.0, 0.03] });
                if selected {
                    o.rect_outline(rect[0], rect[1], rect[2], rect[3], (t * 0.06).max(1.0), rgba(GOLD, pulse));
                }
                let cy = rect[1] + rect[3] / 2.0;
                let sw = t * 0.55;
                let lit = if card.is_some() { 1.0 } else { 0.35 };
                o.rect(rect[0] + t * 0.45, cy - sw / 2.0, sw, sw, rgba(sin.color(), lit));
                let name = match card {
                    Some(c) => format!("{} - {}", sin.title(), c.name.to_uppercase()),
                    None => format!("{} - {}", sin.title(), sin.realm().to_uppercase()),
                };
                let color = if card.is_some() { sin.color() } else { 0x8a857a };
                o.text(&name, rect[0] + t * 1.4, cy, small, rgba(color, 1.0), Align::Left, true);
            }
            RowKind::Rival(i) => {
                let r = g.intel_rivals().get(i as usize).and_then(|&id| g.ledger().get(id));
                o.rect(rect[0], rect[1], rect[2], rect[3], [0.45, 0.2, 0.15, 0.14]);
                if selected {
                    o.rect_outline(rect[0], rect[1], rect[2], rect[3], (t * 0.06).max(1.0), rgba(GOLD, pulse));
                }
                let cy = rect[1] + rect[3] / 2.0;
                if let Some(r) = r {
                    let pip = t * 0.11;
                    for k in 0..r.rank.min(5) {
                        o.circle(rect[0] + t * 0.5 + k as f32 * pip * 2.6, cy, pip, rgba(0xeab55e, 1.0));
                    }
                    let name = r.full_name().to_uppercase();
                    o.text(&name, rect[0] + t * 1.9, cy, small * 0.92, rgba(0xff9a6a, 1.0), Align::Left, true);
                }
            }
            RowKind::Trophy(i) => {
                let trophy = g.workshop.trophies.get(i as usize);
                o.rect(rect[0], rect[1], rect[2], rect[3], if trophy.is_some() { [0.55, 0.45, 0.25, 0.14] } else { [1.0, 1.0, 1.0, 0.03] });
                if selected {
                    o.rect_outline(rect[0], rect[1], rect[2], rect[3], (t * 0.06).max(1.0), rgba(GOLD, pulse));
                }
                let cy = rect[1] + rect[3] / 2.0;
                o.text(&roman(i as usize + 1), rect[0] + t * 0.85, cy, small * 0.9, rgba(TEXT_DIM, 1.0), Align::Center, true);
                let (text, color) = match trophy {
                    Some(tr) => (tr.name.to_uppercase(), tr.color),
                    None => ("AN EMPTY HOOK".to_string(), 0x6a665e),
                };
                o.text(&text, rect[0] + t * 1.6, cy, small, rgba(color, 1.0), Align::Left, true);
            }
            RowKind::Tabs | RowKind::Leave => {}
        }
    }

    // Buttons.
    for (r, row) in rows.iter().enumerate() {
        for (c, b) in row.buttons.iter().enumerate() {
            let rect = l.buttons[r][c];
            let selected = r == g.shop.row && (c == g.shop.col || row.buttons.len() == 1);
            // The open tab, the archangel he is sworn to, the outfit he wears.
            let active_tab = match b.action {
                ShopAction::Tab(tab) => !g.shop.loadout && g.shop.board.is_none() && tab == g.shop.tab,
                ShopAction::Board(s) => row.kind == RowKind::Tabs && !g.shop.loadout && g.shop.board == Some(s),
                ShopAction::Loadout => g.shop.loadout,
                ShopAction::Patron(a) => prog.patron == Some(a),
                ShopAction::Outfit(k) => prog.outfit == k,
                _ => false,
            };
            let afford = b.price <= g.progress.silver && b.runes <= g.progress.runes;
            let base = if active_tab { [0.36, 0.2, 0.12, 0.95] } else { [INK[0], INK[1], INK[2], 0.85] };
            o.rect(rect[0], rect[1], rect[2], rect[3], base);
            if selected && b.enabled {
                o.rect(rect[0], rect[1], rect[2], rect[3], [1.0, 1.0, 1.0, 0.12]);
            }
            let edge =
                if selected && (row.buttons.len() > 1 || row.kind == RowKind::Leave) { rgba(GOLD, pulse) } else { rgba(COPPER, 0.45) };
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
            let size = if row.kind == RowKind::Tabs {
                // Seven tabs: the longer names are set smaller to fit.
                let fit = (rect[2] - t * 0.3) / o.measure(&b.label, t * 0.58).max(1.0);
                t * 0.58 * fit.min(1.0)
            } else if row.buttons.len() > 1 {
                t * 0.46
            } else {
                t * 0.5
            };
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
            "Arrows or stick to choose   Enter or A to take   Tab or LT next station   Esc to leave",
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
    o.text("THE OLD MAN", t * 0.9, h - t * 1.95, small, rgba(GOLD, 1.0), Align::Left, true);
    wrap_left(o, &old_man_line(g), t * 0.9, h - t * 1.2, bw - t * 1.0, t * 0.6, small * 0.95, rgba(TEXT, 1.0));
}

/// The loadout's card: the highlighted archangel, outfit, blade or shield.
fn draw_loadout_card(o: &mut Overlay, g: &Game, t: f32, c: [f32; 4], rows: &[ShopRow]) {
    let hero = g.hero_kind;
    let prog = g.progress.hero(hero);
    let small = (t * 0.5).max(11.0);
    let pad = t * 0.5;
    let (x, w) = (c[0] + pad, c[2] - pad * 2.0);
    let mut y = c[1] + pad + t * 0.4;
    let kind = rows.get(g.shop.row).map(|r| r.kind).unwrap_or(RowKind::Tabs);
    let title = |o: &mut Overlay, y: f32, text: &str, color: u32| o.text(text, x, y, t * 0.75, rgba(color, 1.0), Align::Left, true);
    match kind {
        RowKind::Patron => {
            let a = ARCHANGELS[g.shop.col.min(3)];
            let sworn = prog.patron == Some(a);
            let (ex, ey, r) = (c[0] + c[2] - pad - t * 1.0, y + t * 0.3, t * 0.8);
            o.circle(ex, ey, r, rgba(a.color(), 0.9));
            o.circle(ex, ey, r * 0.55, rgba(0xf0ebe0, 1.0));
            o.arc(ex, ey - r * 0.85, r * 0.55, 0.0, std::f32::consts::TAU, (t * 0.08).max(1.2), rgba(0xffe070, 1.0));
            title(o, y, &a.name().to_uppercase(), a.color());
            y += t * 0.9;
            y = wrap_left(o, a.power(), x, y, w - t * 2.4, t * 0.58, small, rgba(TEXT, 1.0)) + t * 0.2;
            o.text(
                &format!("RETURNS EVERY {} SECONDS   A BOSS LOSES {}", a.recharge() as u32, a.boss_damage()),
                x,
                y,
                small * 0.9,
                rgba(GOLD, 1.0),
                Align::Left,
                true,
            );
            y += t * 0.75;
            wrap_left(
                o,
                "Call on him with R or LT. A blessing from a shrine is used first while you carry one; then his power, each time it comes back.",
                x,
                y,
                w,
                t * 0.55,
                small * 0.85,
                rgba(TEXT_DIM, 1.0),
            );
            let (s, sc) = if sworn { ("HE WATCHES OVER YOU", GOLD) } else { ("PRESS TO SWEAR TO HIM - IT COSTS NOTHING", 0x7dff9a) };
            o.text(s, x, c[1] + c[3] - pad - t * 0.2, small * 0.9, rgba(sc, 1.0), Align::Left, true);
        }
        RowKind::Outfit => {
            let k = g.shop.col.min(3);
            let worn = prog.outfit as usize == k;
            title(o, y, &outfit_names(hero)[k].to_uppercase(), 0xe8dcc0);
            y += t * 0.9;
            wrap_left(o, "Cloth and colours over the same steel: an outfit changes how he looks, and nothing else. Try it on and see him in the crypt.", x, y, w, t * 0.58, small, rgba(TEXT, 1.0));
            let (s, sc) = if worn { ("YOU WEAR IT", GOLD) } else { ("PRESS TO WEAR IT - IT COSTS NOTHING", 0x7dff9a) };
            o.text(s, x, c[1] + c[3] - pad - t * 0.2, small * 0.9, rgba(sc, 1.0), Align::Left, true);
        }
        RowKind::Sharpen => {
            let n = progression::node(hero, Tree::Armory, prog.wield);
            let edge = prog.sharpness(prog.wield);
            title(o, y, &n.name.to_uppercase(), n.color);
            y += t * 0.9;
            o.text(
                &format!("EDGE {}%   IT BITES THE BOSS AT {}%", (edge * 100.0).round(), (gear::edge_power(edge) * 100.0).round()),
                x,
                y,
                small * 0.95,
                rgba(GOLD, 1.0),
                Align::Left,
                true,
            );
            y += t * 0.75;
            wrap_left(
                o,
                "Every blow that lands wears the edge away and rust creeps in. A dull blade bites the boss less; a rusted one glances off foes it should have felled. Gold puts the edge back.",
                x,
                y,
                w,
                t * 0.55,
                small * 0.9,
                rgba(TEXT, 1.0),
            );
        }
        RowKind::Repair => {
            let tier = g.hero.shield.map(|s| s.tier).unwrap_or(prog.carry);
            let n = progression::node(hero, Tree::Shield, tier);
            title(o, y, &n.name.to_uppercase(), n.color);
            y += t * 0.9;
            let text = if g.hero.shield.is_none() && prog.is_broken(tier) {
                format!(
                    "DESTROYED. A shield that breaks is gone: the old man forges it anew for {} {}.",
                    gear::reforge_price(tier),
                    progression::coin(hero)
                )
            } else {
                format!(
                    "Durability {} of {}. Hold LB or Shift to raise it: it takes the whole of every blow, but you walk slower and cannot strike. If it breaks, it is gone until it is reforged.",
                    g.hero.shield.map(|s| s.durability).unwrap_or(0),
                    progression::ward(tier).durability
                )
            };
            wrap_left(o, &text, x, y, w, t * 0.58, small * 0.95, rgba(TEXT, 1.0));
        }
        _ => {
            title(o, y, "YOUR LOADOUT", GOLD);
            y += t * 0.9;
            wrap_left(
                o,
                "Swear to one archangel, whose power comes back to you on a timer: Michael in 10 seconds, Gabriel 20, Raphael 30, Uriel 45. Choose your outfit, sharpen your blade, and forge anew a shield that broke.",
                x,
                y,
                w,
                t * 0.58,
                small * 0.95,
                rgba(TEXT, 1.0),
            );
        }
    }
}

/// The card for the highlighted row: the tier's legend, what it does, and what it takes.
fn draw_card(o: &mut Overlay, g: &Game, t: f32, c: [f32; 4], rows: &[ShopRow]) {
    let hero = g.hero_kind;
    let prog = g.progress.hero(hero);
    let small = (t * 0.5).max(11.0);
    o.rect(c[0], c[1], c[2], c[3], [INK[0], INK[1], INK[2], 0.5]);
    o.rect_outline(c[0], c[1], c[2], c[3], 1.0, rgba(COPPER, 0.4));
    if g.shop.loadout {
        draw_loadout_card(o, g, t, c, rows);
        return;
    }
    let kind = rows.get(g.shop.row).map(|r| r.kind).unwrap_or(RowKind::Tabs);
    if g.shop.board.is_some() || kind == RowKind::Trials {
        draw_board_card(o, g, t, c, kind);
        return;
    }
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
        let line = format!("DURABILITY {}   +{} HEALTH{extra}", wd.durability, wd.vigor);
        o.text(&line, c[0] + pad, ey + t * 0.1, small * 0.8, rgba(SAND, 1.0), Align::Left, false);
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

/// The card at the war table, the trophy wall and the yard's trials: a general known or not, and
/// the old man's news; a trophy and his word on it; the trials to come.
fn draw_board_card(o: &mut Overlay, g: &Game, t: f32, c: [f32; 4], kind: RowKind) {
    let small = (t * 0.5).max(11.0);
    let pad = t * 0.5;
    let (x, w) = (c[0] + pad, c[2] - pad * 2.0);
    let mut y = c[1] + pad + t * 0.4;
    let title = |o: &mut Overlay, y: f32, text: &str, color: u32| o.text(text, x, y, t * 0.75, rgba(color, 1.0), Align::Left, true);
    let station = g.station();
    match kind {
        RowKind::Trials => {
            super::mastery::draw_mastery_card(o, g, t, c);
        }
        RowKind::Trophy(i) => match g.workshop.trophies.get(i as usize) {
            Some(tr) => {
                title(o, y, &tr.name.to_uppercase(), tr.color);
                y += t * 0.9;
                wrap_left(o, &format!("\"{}\"", tr.line), x, y, w, t * 0.58, small * 0.95, rgba(TEXT, 1.0));
            }
            None => {
                title(o, y, "AN EMPTY HOOK", SAND);
                y += t * 0.9;
                wrap_left(
                    o,
                    "What you take from below hangs here: the trophies of the rivals you slay, the banners of the generals you break, the princes' keys. Every one has its story, and the old man will tell it.",
                    x,
                    y,
                    w,
                    t * 0.58,
                    small * 0.95,
                    rgba(TEXT, 1.0),
                );
            }
        },
        RowKind::Rival(i) => {
            let Some(r) = g.intel_rivals().get(i as usize).and_then(|&id| g.ledger().get(id)) else { return };
            title(o, y, &r.full_name().to_uppercase(), 0xff9a6a);
            y += t * 0.75;
            let breed = crate::logic::war::names::breed_name(r.kind);
            o.text(&format!("RANK {} {}", r.rank, breed.to_uppercase()), x, y, small * 0.9, rgba(TEXT_DIM, 1.0), Align::Left, true);
            y += t * 0.8;
            let weak = crate::logic::war::ledger::Trait::Weakness(r.weakness);
            let text = if r.knows(weak) {
                format!("\"{}\"", crate::logic::war::ledger::counsel_line_for(r))
            } else {
                format!(
                    "\"I know where {} can be had. It will cost you {} silver; what I sell is true.\"",
                    r.name,
                    crate::game::counsel::intel_price(r.rank)
                )
            };
            wrap_left(o, &text, x, y, w, t * 0.58, small * 0.95, rgba(TEXT, 1.0));
        }
        _ if station == Station::WarTable => {
            let i = match kind {
                RowKind::General(i) => i as usize,
                _ => 0,
            };
            let sin = SINS[i.min(6)];
            match g.workshop.generals.get(i).and_then(|c| c.as_ref()) {
                Some(card) => {
                    title(o, y, &card.name.to_uppercase(), sin.color());
                    y += t * 0.9;
                    y = wrap_left(o, &card.note, x, y, w, t * 0.58, small * 0.95, rgba(TEXT, 1.0));
                }
                None => {
                    title(o, y, &format!("THE GENERAL OF {}", sin.title()), sin.color());
                    y += t * 0.9;
                    y = wrap_left(
                        o,
                        &format!(
                            "No word yet from {}. Whoever holds it for {} keeps out of sight; the old man will hear of him.",
                            sin.realm(),
                            sin.prince()
                        ),
                        x,
                        y,
                        w,
                        t * 0.58,
                        small * 0.95,
                        rgba(TEXT, 1.0),
                    );
                }
            }
            // The news he has, at the foot of the card.
            let ny = (y + t * 0.5).max(c[1] + c[3] * 0.5);
            o.text("NEWS FROM BELOW", x, ny, small * 0.9, rgba(GOLD, 1.0), Align::Left, true);
            if g.workshop.news.is_empty() {
                o.text("None. He only speaks when it matters.", x, ny + t * 0.65, small * 0.88, rgba(TEXT_DIM, 1.0), Align::Left, false);
            }
            let mut ly = ny + t * 0.65;
            for n in g.workshop.news.iter().take(3) {
                ly = wrap_left(o, n, x, ly, w, t * 0.55, small * 0.88, rgba(TEXT, 1.0)) + t * 0.1;
            }
        }
        _ => {
            title(o, y, station.name(), GOLD);
        }
    }
}
