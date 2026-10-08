//! The bestiary on screen (`game::bestiary`, `logic::bestiary`): over a dim, a glass page ruled in
//! the old man's amber. Along its top the groups as tabs (LB and RB between them); then three
//! columns: the group's pages (a "?" for those not yet met), the page's foe as a live figure turning
//! slowly with its name, where its tale comes from, the question its archetype asks and the
//! player's figures with it, and the words: the legend, how it fights, and the old man's line
//! beside his face. The words scroll a line at a time when they outgrow their column.

use std::sync::atomic::Ordering;

use super::style::{self, size};
use super::*;
use crate::game::bestiary::{Bestiary, Page};
use crate::game::counsel::AMBER;
use crate::logic::bestiary::{self as book, Entry, Group, Lore, Tally, GROUPS};
use crate::logic::combat::archetype::Archetype;
use crate::render::portrait::{Figure, PortraitId, Pose, EVERY_FRAME};

/// The headings' colours: the legend gold, how it fights a cool blue (apart from both gold and
/// amber for every palette), the old man's amber.
const LEGEND: u32 = GOLD;
const FIGHTS: u32 = 0x9ad1ff;
/// The tag of each archetype: the brute copper, the skirmisher sand, the hexer violet, a prince gold.
fn arch_color(a: Archetype) -> u32 {
    match a {
        Archetype::Brute => 0xd88a5a,
        Archetype::Skirmisher => 0xd7ccb8,
        Archetype::Hexer => 0xb89aff,
        Archetype::Prince => GOLD,
    }
}

/// Where everything on the page is (logical pixels), for drawing and for the pointer.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub panel: [f32; 4],
    /// One per group, in `GROUPS`' order.
    pub tabs: Vec<[f32; 4]>,
    /// The arrows either side of the tabs.
    pub prev: [f32; 4],
    pub next: [f32; 4],
    /// The group's heading.
    pub heading: [f32; 4],
    /// The list's column, and its rows in view: (the page's place in its group, its rectangle).
    pub list: [f32; 4],
    pub rows: Vec<(usize, [f32; 4])>,
    /// The figure's square, and the column under it (name, tags, figures).
    pub figure: [f32; 4],
    pub card: [f32; 4],
    /// The words' column (it scrolls).
    pub words: [f32; 4],
    /// The footer, and its Back button.
    pub footer: [f32; 4],
    pub back: [f32; 4],
}

/// The page laid out on a `w` by `h` screen, for a group of `n` pages with page `index` chosen
/// (the list scrolls to keep it in view).
pub fn layout(w: f32, h: f32, n: usize, index: usize) -> Layout {
    let t = tile_for(w, h);
    let safe = style::safe(w, h);
    let pw = (w - safe * 2.0).min(t * 62.0);
    let ph = h - safe * 2.0;
    let (px, py) = ((w - pw) / 2.0, safe);
    let pad = t * 0.8;
    // The tabs, between their arrows.
    let ty = py + pad + t * 2.3;
    let th = t * 1.25;
    let aw = t * 2.2;
    let prev = [px + pad, ty, aw, th];
    let next = [px + pw - pad - aw, ty, aw, th];
    let (tx0, tx1) = (prev[0] + aw + t * 0.3, next[0] - t * 0.3);
    let gap = t * 0.18;
    let tw = (tx1 - tx0 - gap * (GROUPS.len() as f32 - 1.0)) / GROUPS.len() as f32;
    let tabs = (0..GROUPS.len()).map(|k| [tx0 + k as f32 * (tw + gap), ty, tw, th]).collect();
    let heading = [px + pad, ty + th + t * 0.35, pw - pad * 2.0, t * 1.2];
    // The footer, and the three columns between.
    let footer = [px + pad, py + ph - pad - t * 1.2, pw - pad * 2.0, t * 1.2];
    let back = [footer[0] + footer[2] - t * 5.2, footer[1], t * 5.2, footer[3]];
    let top = heading[1] + heading[3] + t * 0.45;
    let bottom = footer[1] - t * 0.5;
    let inner = pw - pad * 2.0;
    let col_gap = t * 0.9;
    let lw = (inner * 0.21).clamp(t * 9.0, t * 13.0);
    let mw = (inner * 0.27).clamp(t * 10.0, t * 15.0);
    let list = [px + pad, top, lw, bottom - top];
    let mx = list[0] + lw + col_gap;
    let fs = mw.min((bottom - top) * 0.56);
    let figure = [mx + (mw - fs) / 2.0, top, fs, fs];
    let card = [mx, top + fs + t * 0.35, mw, bottom - top - fs - t * 0.35];
    let wx = mx + mw + col_gap;
    let words = [wx, top, px + pw - pad - wx, bottom - top];
    // The rows in view, the chosen one kept from the edges.
    let rh = t * 1.25;
    let fit = ((list[3] / rh).floor() as usize).max(1);
    let first = if n <= fit { 0 } else { index.saturating_sub(fit / 2).min(n - fit) };
    let rows = (first..n.min(first + fit)).map(|i| (i, [list[0], top + (i - first) as f32 * rh, lw, rh])).collect();
    Layout { panel: [px, py, pw, ph], tabs, prev, next, heading, list, rows, figure, card, words, footer, back }
}

/// One line of the words: its text and how it is set.
#[derive(Clone, Debug)]
enum Line {
    Heading(&'static str, u32),
    Body(String, u32),
    /// The old man's words, beside his face.
    His(String),
    Gap,
}

/// What a page says: its words, or what is known of a page not yet met.
struct Shown {
    figure: Figure,
    name: String,
    native: String,
    origin: String,
    arch: Option<(Archetype, crate::logic::combat::archetype::Attack)>,
    tally: Option<Tally>,
    legend: String,
    fights: String,
    old_man: String,
    open: bool,
}

/// The figure a page shows.
fn figure_of(entry: Entry) -> Figure {
    match entry {
        Entry::Foe(k) => Figure::Foe(k),
        Entry::Cyclops => Figure::Cyclops,
        Entry::Mimic => Figure::Mimic,
        Entry::General(s) => Figure::Foe(crate::logic::war::general::General::of(s).body),
    }
}

fn shown(g: &Game, b: &Bestiary, page: Page) -> Shown {
    match page {
        Page::Fixed(entry) => {
            let lore: Lore = book::lore(entry);
            if !b.shown(page) {
                return Shown {
                    figure: figure_of(entry),
                    name: "? ? ?".into(),
                    native: String::new(),
                    origin: "NOT YET MET".into(),
                    arch: None,
                    tally: None,
                    legend: "You have not met this one yet. When you have seen it with your own eyes, its page will open.".into(),
                    fights: String::new(),
                    old_man: "I will tell you of it when you have seen it yourself. Some things are better met than read of.".into(),
                    open: false,
                };
            }
            Shown {
                figure: figure_of(entry),
                name: lore.name.to_uppercase(),
                native: lore.native.into(),
                origin: lore.origin.into(),
                arch: book::archetype(entry),
                tally: (!b.review || b.book.unlocked(entry)).then(|| b.book.tally(entry)),
                legend: lore.legend.into(),
                fights: lore.fights.into(),
                old_man: lore.old_man.into(),
                open: true,
            }
        }
        Page::Rival(i) => {
            let Some(r) = b.book.rivals.get(i) else {
                return Shown {
                    figure: Figure::Foe(crate::logic::enemy::EnemyKind::Skeleton),
                    name: "NO ONE YET".into(),
                    native: String::new(),
                    origin: String::new(),
                    arch: None,
                    tally: None,
                    legend: "No one down there knows your name. Yet.".into(),
                    fights: String::new(),
                    old_man: String::new(),
                    open: false,
                };
            };
            let breed = crate::logic::war::names::breed_name(r.kind);
            let (tale, word) = book::rival_words(breed, r.alive);
            // What the Ledger of this run knows of him, if he is in it.
            let live = g.ledger().rivals.iter().find(|x| x.full_name() == r.name || bestiary_clean(&x.full_name()) == r.name);
            let fights = match live {
                Some(x) => {
                    let c = x.card();
                    let traits: Vec<String> =
                        c.traits.iter().map(|(label, v)| format!("{}: {}", title_case(label), v.as_deref().unwrap_or("?"))).collect();
                    let mut s = format!("He fights as a {breed} does, and better. What you have found of him: {}.", traits.join(", "));
                    if let Some(l) = c.lesson {
                        s.push_str(&format!(" {l}"));
                        if !s.ends_with('.') {
                            s.push('.');
                        }
                    }
                    s
                }
                None => format!("He fought as a {breed} does, and better. What you found of him went with the run you met him in."),
            };
            Shown {
                figure: Figure::Foe(r.kind),
                name: r.name.to_uppercase(),
                native: format!("{}, rank {} of 5{}", title_case(breed), r.rank, if r.alive { "" } else { ", slain" }),
                origin: "The Ledger of Grudges".into(),
                arch: book::archetype(Entry::Foe(r.kind)),
                tally: Some(r.tally),
                legend: tale,
                fights,
                old_man: word.into(),
                open: true,
            }
        }
    }
}

/// A name as the book keeps it (the save's separators made spaces).
fn bestiary_clean(name: &str) -> String {
    name.chars().map(|c| if matches!(c, ';' | '\n' | '\r') { ' ' } else { c }).collect::<String>().trim().to_string()
}

/// "STRENGTH" as "Strength", "loup-garou" as "Loup-garou".
fn title_case(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut c = lower.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// `text` in `style`, made smaller until it fits `max` (never under 60%).
fn fit(o: &mut Overlay, text: &str, x: f32, y: f32, max: f32, style: &Style) -> f32 {
    let mut s = *style;
    while s.size > style.size * 0.6 && o.measure_styled(text, &s) > max {
        s = s.size(s.size * 0.93);
    }
    o.text_styled(text, x, y, &s)
}

pub(super) fn draw(o: &mut Overlay, g: &Game, t: f32) {
    let b = &g.bestiary;
    let (w, h) = (o.width, o.height);
    let group = b.group_now();
    let pages = b.pages(group);
    let index = b.index.min(pages.len().saturating_sub(1));
    let lay = layout(w, h, pages.len(), index);
    let time = g.time as f32;
    let (alpha, slide) = style::appear(time - b.since as f32, f32::MAX);
    o.rect(0.0, 0.0, w, h, [INK[0], INK[1], INK[2], 0.88 * alpha]);
    let mut panel = lay.panel;
    panel[1] += slide * t * 0.8;
    let dy = panel[1] - lay.panel[1];
    o.alpha = alpha;
    style::panel_ruled(o, panel, t, 1.0, AMBER);
    let [px, py, pw, _] = panel;
    let pad = t * 0.8;

    // ---- the heading: the book's name, what it is, how far it is filled
    o.text_styled("THE BESTIARY", px + pad, py + pad + t * 0.55, &style::name(t, 1.2, rgba(GOLD, 1.0)));
    let (open, all) = b.total();
    let count = if b.review { format!("EVERY PAGE OPEN FOR REVIEW   {all} PAGES") } else { format!("{open} OF {all} PAGES") };
    o.text_styled(&count, px + pw - pad, py + pad + t * 0.4, &style::label(t, size::LABEL, rgba(SAND, 1.0)).right());
    o.text_styled(
        "The old man's book of every foe you have met down here.",
        px + pw - pad,
        py + pad + t * 1.25,
        &style::body(t, 0.56, rgba(TEXT_DIM, 1.0)).right(),
    );

    // ---- the tabs, between LB and RB
    let shift = |r: [f32; 4]| [r[0], r[1] + dy, r[2], r[3]];
    for (k, (&grp, r)) in GROUPS.iter().zip(&lay.tabs).enumerate() {
        let r = shift(*r);
        let here = k == b.group;
        let (opened, of) = b.progress(grp);
        let lit = opened > 0;
        let back = if here { rgba(COPPER, 0.55) } else { [1.0, 1.0, 1.0, if lit { 0.06 } else { 0.025 }] };
        glyphs::rounded_rect(o, r[0], r[1], r[2], r[3], r[3] * 0.3, back);
        if here {
            o.rect(r[0] + r[2] * 0.12, r[1] + r[3] - (t * 0.08).max(2.0), r[2] * 0.76, (t * 0.08).max(2.0), rgba(GOLD, 1.0));
        }
        let color = if here {
            GOLD
        } else if lit {
            SAND
        } else {
            0x6a645a
        };
        let st = style::label(t, 0.5, rgba(color, 1.0)).centered();
        fit(o, grp.short(), r[0] + r[2] / 2.0, r[1] + r[3] * 0.42, r[2] - t * 0.2, &st);
        // How much of it is open: a fine bar along its foot.
        if of > 0 && opened > 0 {
            let fw = (r[2] - t * 0.5) * opened as f32 / of as f32;
            o.rect(r[0] + t * 0.25, r[1] + r[3] * 0.78, fw, (t * 0.05).max(1.0), rgba(AMBER, if here { 0.9 } else { 0.5 }));
        }
    }
    for (r, action, left) in [(shift(lay.prev), Action::TabPrev, true), (shift(lay.next), Action::TabNext, false)] {
        glyphs::rounded_rect(o, r[0], r[1], r[2], r[3], r[3] * 0.3, [1.0, 1.0, 1.0, 0.04]);
        let (cx, cy) = (r[0] + r[2] / 2.0, r[1] + r[3] / 2.0);
        let s = t * 0.18;
        let tip = if left { r[0] + t * 0.35 } else { r[0] + r[2] - t * 0.35 };
        let base = if left { tip + s * 1.4 } else { tip - s * 1.4 };
        o.triangle([tip, cy], [base, cy - s], [base, cy + s], rgba(GOLD, 0.9));
        let gx = if left { cx + t * 0.25 } else { cx - t * 0.25 };
        let gw = glyphs::measure_prompt_text(o, &g.input, &format!("{{{}}}", crate::input::token(action)), &style::label(t, 0.5, [1.0; 4]));
        glyphs::prompt_text(
            o,
            &g.input,
            &format!("{{{}}}", crate::input::token(action)),
            gx - gw / 2.0,
            cy,
            &style::label(t, 0.5, rgba(TEXT, 1.0)),
        );
    }

    // ---- the group's heading and what it holds
    let hd = shift(lay.heading);
    let hw = o.text_styled(group.title(), hd[0], hd[1] + hd[3] * 0.5, &style::name(t, 0.82, rgba(GOLD, 1.0)));
    let (opened, of) = b.progress(group);
    let tail = if group == Group::Rivals { format!("{of} REMEMBERED") } else { format!("{opened} OF {of} MET") };
    let tail_w = o.text_styled(&tail, hd[0] + hd[2], hd[1] + hd[3] * 0.5, &style::label(t, 0.52, rgba(SAND, 1.0)).right());
    fit(
        o,
        group.blurb(),
        hd[0] + hw + t * 0.8,
        hd[1] + hd[3] * 0.5,
        hd[2] - hw - tail_w - t * 1.6,
        &style::body(t, 0.56, rgba(TEXT_DIM, 1.0)),
    );
    style::divider(o, hd[0], hd[1] + hd[3] + t * 0.2, hd[2], t, rgba(COPPER, 0.6));

    // ---- the list
    if pages.is_empty() {
        let l = shift(lay.list);
        o.wrap_styled(
            "No one down there knows your name. Yet. A foe who does a deed against you earns one, and the book remembers him.",
            l[0],
            l[1] + t * 0.6,
            l[2],
            t * 0.8,
            &style::body(t, 0.56, rgba(TEXT_DIM, 1.0)),
        );
    }
    for &(i, r) in &lay.rows {
        let r = shift(r);
        let page = pages[i];
        let chosen = i == index;
        if chosen {
            glyphs::rounded_rect(o, r[0], r[1] + t * 0.08, r[2], r[3] - t * 0.16, t * 0.2, rgba(COPPER, 0.35));
            o.rect(r[0], r[1] + t * 0.2, (t * 0.08).max(2.0), r[3] - t * 0.4, rgba(GOLD, 1.0));
        }
        let open = b.shown(page);
        let cy = r[1] + r[3] / 2.0;
        let mark = if open { GOLD } else { 0x5a554c };
        style::diamond(o, r[0] + t * 0.55, cy, t * 0.16, rgba(mark, if open { 1.0 } else { 0.8 }));
        let label = match page {
            _ if !open => "? ? ?".to_string(),
            Page::Fixed(e) => book::lore(e).name.to_string(),
            Page::Rival(k) => b.book.rivals.get(k).map(|r| r.name.clone()).unwrap_or_default(),
        };
        let color = if chosen {
            GOLD
        } else if open {
            TEXT
        } else {
            0x7a7468
        };
        fit(o, &label, r[0] + t * 1.1, cy, r[2] - t * 1.3, &style::body(t, 0.62, rgba(color, 1.0)));
    }
    // More above or below: small arrows at the list's ends.
    if let (Some(&(first, _)), Some(&(last, lr))) = (lay.rows.first(), lay.rows.last()) {
        let l = shift(lay.list);
        if first > 0 {
            let (cx, y) = (l[0] + l[2] / 2.0, l[1] - t * 0.15);
            o.triangle([cx, y - t * 0.2], [cx - t * 0.25, y + t * 0.05], [cx + t * 0.25, y + t * 0.05], rgba(SAND, 0.8));
        }
        if last + 1 < pages.len() {
            let (cx, y) = (l[0] + l[2] / 2.0, lr[1] + dy + lr[3] + t * 0.1);
            o.triangle([cx, y + t * 0.2], [cx - t * 0.25, y - t * 0.05], [cx + t * 0.25, y - t * 0.05], rgba(SAND, 0.8));
        }
    }

    let Some(page) = pages.get(index).copied() else {
        draw_footer(o, g, &lay, dy, t);
        o.alpha = 1.0;
        return;
    };
    let s = shown(g, b, page);
    // The page fades in as it is turned to.
    let turn = ((time - b.turned as f32) / 0.18).clamp(0.0, 1.0);
    o.alpha = alpha * (0.35 + 0.65 * turn);

    // ---- the figure, turning, on a warm ground in a gold frame (a silhouette until it is met)
    let f = shift(lay.figure);
    o.rect(f[0], f[1], f[2], f[3], [0.03, 0.025, 0.02, 0.9]);
    style::glow(o, f[0] + f[2] * 0.5, f[1] + f[3] * 0.62, f[2] * 0.62, rgba(if s.open { 0xd08a3a } else { 0x5a6a7a }, 0.4));
    // (A shadow on the ground under it.)
    style::glow(o, f[0] + f[2] * 0.5, f[1] + f[3] * 0.93, f[2] * 0.3, [0.0, 0.0, 0.0, 0.5]);
    let tint = if s.open { [1.0; 4] } else { [0.0, 0.0, 0.0, 0.92] };
    o.portrait(PortraitId::Figure(s.figure), Pose::live(time, EVERY_FRAME), f[0], f[1], f[2], f[3], false, tint);
    if !s.open {
        o.text_styled("?", f[0] + f[2] / 2.0, f[1] + f[3] / 2.0, &style::name(t, 3.0, rgba(SAND, 0.75)).centered());
    }
    o.rect_outline(f[0], f[1], f[2], f[3], (t * 0.06).max(1.0), rgba(0xf2c46a, 0.9));
    for (cx, cy) in [(f[0], f[1]), (f[0] + f[2], f[1]), (f[0], f[1] + f[3]), (f[0] + f[2], f[1] + f[3])] {
        style::diamond(o, cx, cy, t * 0.13, rgba(0xf2c46a, 1.0));
    }

    // ---- the card under it: name, tongue, origin, the archetype's question, the figures
    let c = shift(lay.card);
    let mut y = c[1] + t * 0.55;
    let name_style = style::name(t, 0.9, rgba(GOLD, 1.0)).centered();
    let name_lines = o.wrap_lines(&s.name, c[2], &name_style);
    for l in name_lines.iter().take(2) {
        fit(o, l, c[0] + c[2] / 2.0, y, c[2], &name_style);
        y += t * 0.95;
    }
    if !s.native.is_empty() {
        let st = style::body(t, 0.56, rgba(SAND, 1.0)).centered();
        for l in o.wrap_lines(&s.native, c[2], &st).iter().take(2) {
            o.text_styled(l, c[0] + c[2] / 2.0, y, &st);
            y += t * 0.72;
        }
    }
    y += t * 0.25;
    if !s.origin.is_empty() {
        let st = style::label(t, 0.48, rgba(TEXT, 1.0));
        let label = s.origin.to_uppercase();
        let room = c[2] - t * 0.8;
        let st = if o.measure_styled(&label, &st) > room { st.size(st.size * room / o.measure_styled(&label, &st)) } else { st };
        style::pill(o, &label, c[0] + c[2] / 2.0, y, &st.centered(), [0.0, 0.0, 0.0, 0.5], Some(rgba(COPPER, 0.7)));
        y += t * 0.85;
    }
    if let Some((arch, attack)) = s.arch {
        let q = book::question(arch);
        let st = style::label(t, 0.46, rgba(arch_color(arch), 1.0));
        let room = c[2] - t * 0.8;
        let st = if o.measure_styled(q, &st) > room { st.size(st.size * room / o.measure_styled(q, &st)) } else { st };
        style::pill(o, q, c[0] + c[2] / 2.0, y, &st.centered(), [0.0, 0.0, 0.0, 0.5], Some(rgba(arch_color(arch), 0.8)));
        y += t * 0.8;
        let how = match attack {
            crate::logic::combat::archetype::Attack::Melee => "BLOWS AT ARM'S LENGTH",
            crate::logic::combat::archetype::Attack::Body => "BITES FROM BESIDE YOU",
            crate::logic::combat::archetype::Attack::Distance => "BOLTS FROM AFAR",
        };
        o.text_styled(how, c[0] + c[2] / 2.0, y, &style::label(t, 0.44, rgba(TEXT_DIM, 1.0)).centered());
        y += t * 0.75;
    }
    // The player's figures with it: met, felled, felled you.
    if let Some(tally) = s.tally {
        let bottom = c[1] + c[3];
        let fy = (y + t * 0.55).min(bottom - t * 1.0);
        let cw = c[2] / 3.0;
        for (k, (label, n)) in [("MET", tally.met), ("FELLED", tally.felled), ("FELLED YOU", tally.felled_by)].into_iter().enumerate() {
            let cx = c[0] + cw * (k as f32 + 0.5);
            o.text_styled(
                &n.to_string(),
                cx,
                fy,
                &style::name(t, 0.95, rgba(if n > 0 { TEXT } else { TEXT_DIM }, 1.0)).centered().tabular(),
            );
            o.text_styled(label, cx, fy + t * 0.75, &style::label(t, 0.42, rgba(TEXT_DIM, 1.0)).centered());
        }
    }

    // ---- the words
    let wr = shift(lay.words);
    let units = size::BODY * 0.9 * prefs().prose;
    let body = style::body(t, units, rgba(TEXT, 1.0));
    let line_h = t * units * 1.36;
    let face = t * 2.6;
    let mut lines: Vec<Line> = Vec::new();
    let part = |lines: &mut Vec<Line>, head: &'static str, color: u32, text: &str, his: bool| {
        if text.is_empty() {
            return;
        }
        lines.push(Line::Heading(head, color));
        let width = if his { wr[2] - face - t * 0.6 } else { wr[2] - t * 0.2 };
        for l in o.wrap_lines(text, width, &body) {
            lines.push(if his { Line::His(l) } else { Line::Body(l, TEXT) });
        }
        lines.push(Line::Gap);
    };
    part(&mut lines, "THE LEGEND", LEGEND, &s.legend, false);
    part(&mut lines, "HOW IT FIGHTS", FIGHTS, &s.fights, false);
    part(&mut lines, "THE OLD MAN SAYS", AMBER, &s.old_man, true);
    let height = |l: &Line| match l {
        Line::Heading(..) => t * 1.1,
        Line::Body(..) | Line::His(..) => line_h,
        Line::Gap => t * 0.45,
    };
    // How far it can scroll: the first line from which the rest fit.
    let total: f32 = lines.iter().map(height).sum();
    let mut max = 0;
    let mut rest = total;
    let room = wr[3] - t * 0.6;
    while max < lines.len() && rest > room {
        rest -= height(&lines[max]);
        max += 1;
    }
    b.max_scroll.store(max, Ordering::Relaxed);
    let from = b.scroll.min(max);
    let mut y = wr[1] + t * 0.45;
    let bottom = wr[1] + wr[3];
    let mut his_face: Option<f32> = None;
    for l in &lines[from..] {
        let lh = height(l);
        if y + lh * 0.6 > bottom {
            break;
        }
        match l {
            Line::Heading(head, color) => {
                let hs = style::label(t, size::LABEL * 0.92, rgba(*color, 1.0)).tracking(0.04);
                let hw = o.text_styled(head, wr[0], y, &hs);
                o.rect(wr[0], y + t * 0.4, hw, (t * 0.04).max(1.0), rgba(*color, 0.5));
                if *color == AMBER {
                    his_face = Some(y + t * 0.75);
                }
            }
            Line::Body(text, color) => {
                o.text_styled(text, wr[0], y, &body.color(rgba(*color, 1.0)));
            }
            Line::His(text) => {
                o.text_styled(text, wr[0] + face + t * 0.6, y, &body.color(rgba(0xf4dcb0, 1.0)));
            }
            Line::Gap => {}
        }
        y += lh;
    }
    // His face beside his words, round, on his warm ground.
    if let Some(fy) = his_face {
        if fy + face <= bottom + t * 0.2 {
            style::disc(o, wr[0] + face / 2.0, fy + face / 2.0, face * 0.53, rgba(0x2a1a0c, 0.95));
            style::glow(o, wr[0] + face * 0.45, fy + face * 0.42, face * 0.6, rgba(0xd08a3a, 0.4));
            o.portrait(PortraitId::OldMan, Pose::live(time, 1.0), wr[0], fy, face, face, true, [1.0; 4]);
            style::ring(
                o,
                wr[0] + face / 2.0,
                fy + face / 2.0,
                face * 0.52,
                (t * 0.06).max(1.0),
                0.0,
                std::f32::consts::TAU,
                rgba(AMBER, 0.9),
            );
        }
    }
    // The scroll's track, when the words outgrow their column.
    if max > 0 {
        let x = wr[0] + wr[2] - (t * 0.08).max(2.0);
        let track = [x, wr[1] + t * 0.2, (t * 0.08).max(2.0), wr[3] - t * 0.4];
        o.rect(track[0], track[1], track[2], track[3], [1.0, 1.0, 1.0, 0.08]);
        let shown_share = (wr[3] / total).clamp(0.1, 1.0);
        let th = track[3] * shown_share;
        let ty = track[1] + (track[3] - th) * (from as f32 / max as f32);
        o.rect(track[0], ty, track[2], th, rgba(AMBER, 0.8));
        if from < max {
            let st = style::label(t, 0.44, rgba(SAND, 0.9)).right();
            o.text_styled("MORE BELOW", x - t * 0.3, bottom - t * 0.2, &st);
        }
    }
    o.alpha = alpha;
    draw_footer(o, g, &lay, dy, t);
    o.alpha = 1.0;
}

/// The footer: the controls in the glyphs of the device used last, and the Back button.
fn draw_footer(o: &mut Overlay, g: &Game, lay: &Layout, dy: f32, t: f32) {
    let f = [lay.footer[0], lay.footer[1] + dy, lay.footer[2], lay.footer[3]];
    let st = style::label(t, 0.5, rgba(TEXT_DIM, 1.0));
    let scroll = if g.input.last_device().is_pad() { "{Look} scroll" } else { "Wheel / PgUp PgDn scroll" };
    let text = format!("{{TabPrev}} {{TabNext}} group     {{Nav}} page     {scroll}");
    glyphs::prompt_text(o, &g.input, &text, f[0], f[1] + f[3] / 2.0, &st);
    let r = [lay.back[0], lay.back[1] + dy, lay.back[2], lay.back[3]];
    let hover = g.input.cursor.is_some_and(|p| p.0 >= r[0] && p.0 <= r[0] + r[2] && p.1 >= lay.back[1] && p.1 <= lay.back[1] + r[3]);
    glyphs::rounded_rect(o, r[0], r[1], r[2], r[3], r[3] * 0.3, if hover { rgba(COPPER, 0.5) } else { [1.0, 1.0, 1.0, 0.06] });
    let bs = style::label(t, 0.52, rgba(if hover { GOLD } else { TEXT }, 1.0)).centered();
    glyphs::prompt_text(o, &g.input, "{Back} BACK", r[0] + r[2] / 2.0, r[1] + r[3] / 2.0, &bs);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_fits_the_screen_at_every_size_and_scale() {
        for (w, h) in [(1280.0, 720.0), (1920.0, 1080.0), (3840.0, 2160.0), (1280.0, 800.0), (2560.0, 1080.0), (1024.0, 768.0)] {
            for scale in [0.8, 1.0, 1.5] {
                style::set_ui_scale(scale);
                for n in [0, 4, 13, 30] {
                    let l = layout(w, h, n, n / 2);
                    let [px, py, pw, ph] = l.panel;
                    let within =
                        |r: &[f32; 4]| r[0] >= px - 0.5 && r[1] >= py - 0.5 && r[0] + r[2] <= px + pw + 0.5 && r[1] + r[3] <= py + ph + 0.5;
                    for r in l.tabs.iter().chain([&l.prev, &l.next, &l.list, &l.figure, &l.card, &l.words, &l.footer, &l.back]) {
                        assert!(within(r), "{w}x{h} at {scale}: {r:?} outside {:?}", l.panel);
                        assert!(r[2] > 0.0 && r[3] > 0.0, "{w}x{h} at {scale}: {r:?}");
                    }
                    assert_eq!(l.tabs.len(), GROUPS.len());
                    // The columns stand apart, left to right.
                    assert!(l.list[0] + l.list[2] <= l.figure[0] && l.card[0] + l.card[2] <= l.words[0], "{w}x{h} at {scale}");
                    // The rows in view are in the list, the chosen one among them, one after another.
                    for (k, (i, r)) in l.rows.iter().enumerate() {
                        assert!(r[1] + r[3] <= l.list[1] + l.list[3] + 0.5);
                        if k > 0 {
                            assert_eq!(*i, l.rows[k - 1].0 + 1);
                        }
                    }
                    if n > 0 {
                        assert!(l.rows.iter().any(|(i, _)| *i == n / 2), "{w}x{h} at {scale}: the chosen page is in view");
                    }
                    // The words' column is wide enough to read, and the figure is not a postage stamp.
                    let t = tile_for(w, h);
                    assert!(l.words[2] >= t * 14.0, "{w}x{h} at {scale}: the words are {} wide", l.words[2] / t);
                    assert!(l.figure[2] >= t * 7.0, "{w}x{h} at {scale}: the figure is {} wide", l.figure[2] / t);
                }
            }
        }
        style::set_ui_scale(1.0);
    }

    #[test]
    fn the_words_are_at_least_eighteen_pixels_at_1080p() {
        style::set_ui_scale(1.0);
        let t = tile_for(1920.0, 1080.0);
        let units = size::BODY * 0.9;
        // (The size keeps the old font's meaning: the capitals are 0.7 of it.)
        assert!(t * units >= 18.0, "{}", t * units);
    }
}
