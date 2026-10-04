//! The old gatekeeper's wares: after each Cyclops, or wherever the hero calls
//! him (he promised he would come). Four tabs, one for each of the hero's
//! trees (weapons, path, shields, elixirs), each a column of ten tiers:
//! unlock the next, wield or carry what is owned, buy bottles of the elixirs
//! learned. A fifth, the loadout: the archangel he swears to (whose power
//! comes back to him on a timer), the outfit he wears, his blade sharpened
//! for gold, and a broken shield forged anew. Driven by the keyboard, a
//! controller or the mouse. The rows are built fresh from the hero's
//! progress, so labels and prices always tell the truth. The tabs are the
//! stations of his workshop (`workshop.rs`), where the camera and the old man go.

use super::workshop::{QuitmerMood, Station, Work, PAGES, STATIONS, TROPHY_SLOTS};
use super::{Game, Screen};
use crate::audio::Sfx;
use crate::input::Action;
use crate::logic::angels::{Archangel, ARCHANGELS};
use crate::logic::gear::{self, ShieldGear, BELT_SLOTS};
use crate::logic::progression::{self, Refusal, Tree, TIERS};
use crate::logic::systems::MAX_LIVES;
use crate::logic::types::{Dir, PotionKind};
use crate::render::heroes::outfit_names;
use crate::render::hud;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ShopAction {
    Tab(Tree),
    /// The loadout tab.
    Loadout,
    /// A station without wares: the war table, the trophy wall.
    Board(Station),
    /// Swear to an archangel: his power, on its timer.
    Patron(Archangel),
    /// Wear an outfit (0 is his own).
    Outfit(u8),
    /// Hone the blade in hand back to keen.
    Sharpen,
    /// Forge a broken shield anew.
    Reforge,
    /// Pay for (or unseal with a relic) the next tier of a tree.
    Unlock(Tree, u8),
    /// Take an owned weapon in hand, or an owned shield on the arm.
    Wield(u8),
    Carry(u8),
    /// A bottle of an unlocked elixir for the belt.
    Bottle(PotionKind),
    Repair,
    Life,
    /// The training yard's trials of mastery (not yet open).
    Trials,
    Leave,
}

/// What a row of the shop shows; the HUD draws each kind its own way.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RowKind {
    Tabs,
    Tier(Tree, u8),
    /// The loadout: the archangels, the outfits, the whetstone.
    Patron,
    Outfit,
    Sharpen,
    Repair,
    Life,
    /// The training yard's trials (shut for now).
    Trials,
    /// A realm's general at the war table (by the order of `SINS`).
    General(u8),
    /// A hook on the trophy wall.
    Trophy(u8),
    Leave,
}

#[derive(Clone, Debug)]
pub struct ShopButton {
    pub action: ShopAction,
    pub label: String,
    /// Silver it costs, 0 if free.
    pub price: u32,
    /// Rune fragments it costs on top.
    pub runes: u32,
    /// Can it be pressed at all (owned, locked and equipped ones cannot).
    pub enabled: bool,
}

#[derive(Clone, Debug)]
pub struct ShopRow {
    pub kind: RowKind,
    pub buttons: Vec<ShopButton>,
}

/// A line of feedback under the rows: what was bought, or why not.
#[derive(Clone, Debug)]
pub struct ShopNote {
    pub text: String,
    pub color: u32,
    pub life: f32,
}

#[derive(Clone, Debug)]
pub struct ShopState {
    pub tab: Tree,
    pub row: usize,
    pub col: usize,
    pub note: Option<ShopNote>,
    /// 1 when something was just bought, fading.
    pub flourish: f32,
    /// Called from inside a dungeon: leaving goes back to it rather than on to the next.
    pub called: bool,
    /// The loadout tab is open (rather than the tree in `tab`).
    pub loadout: bool,
    /// A station without wares is open (the war table, the trophy wall), rather than `tab`'s.
    pub board: Option<Station>,
    /// The old man at work on what was just bought.
    pub work: Option<Work>,
    /// Quitmer's mood, and seconds before he lies calm again.
    pub mood: QuitmerMood,
    pub mood_t: f32,
}

impl Default for ShopState {
    fn default() -> Self {
        Self {
            tab: Tree::Armory,
            row: 1,
            col: 0,
            note: None,
            flourish: 0.0,
            called: false,
            loadout: false,
            board: None,
            work: None,
            mood: QuitmerMood::Calm,
            mood_t: 0.0,
        }
    }
}

fn button(action: ShopAction, label: impl Into<String>, price: u32, runes: u32, enabled: bool) -> ShopButton {
    ShopButton { action, label: label.into(), price, runes, enabled }
}

impl Game {
    /// The old man's wares after a Cyclops, on the first row of weapons.
    pub fn open_shop(&mut self) {
        self.shop = ShopState::default();
        self.shop.row = self.first_row_of_interest();
        self.enter(Screen::Shop);
    }

    /// The row to start on: the next tier to take in the open tab (the archangels on the loadout,
    /// the top of a board).
    fn first_row_of_interest(&self) -> usize {
        if self.shop.loadout || self.shop.board.is_some() {
            return 1;
        }
        let owned = self.progress.hero(self.hero_kind).owned(self.shop.tab);
        (owned as usize + 1).min(TIERS as usize)
    }

    /// The shield row: mend the one on his arm, or forge anew the one that broke.
    fn shield_row(&self) -> ShopRow {
        let h = self.progress.hero(self.hero_kind);
        let tier = self.hero.shield.map(|s| s.tier).unwrap_or(h.carry);
        let b = if self.hero.shield.is_none() && h.is_broken(tier) {
            let p = gear::reforge_price(tier);
            button(ShopAction::Reforge, format!("REFORGE {p}"), p, 0, true)
        } else {
            let fraction = self.hero.shield.map(|s| s.fraction()).unwrap_or(0.0);
            let p = progression::repair_price(tier, fraction);
            if p == 0 {
                button(ShopAction::Repair, "GOOD AS NEW", 0, 0, false)
            } else {
                button(ShopAction::Repair, format!("MEND {p}"), p, 0, true)
            }
        };
        ShopRow { kind: RowKind::Repair, buttons: vec![b] }
    }

    /// The loadout's rows: the archangels, the outfits, the whetstone, the shield.
    fn loadout_rows(&self) -> Vec<ShopRow> {
        let hero = self.hero_kind;
        let h = self.progress.hero(hero);
        let mut rows = Vec::new();
        rows.push(ShopRow {
            kind: RowKind::Patron,
            buttons: ARCHANGELS
                .iter()
                .map(|&a| button(ShopAction::Patron(a), format!("{} {}s", a.name().to_uppercase(), a.recharge() as u32), 0, 0, true))
                .collect(),
        });
        rows.push(ShopRow {
            kind: RowKind::Outfit,
            buttons: outfit_names(hero)
                .iter()
                .enumerate()
                .map(|(i, n)| button(ShopAction::Outfit(i as u8), n.to_uppercase(), 0, 0, true))
                .collect(),
        });
        let edge = h.sharpness(h.wield);
        let p = gear::sharpen_price(h.wield, edge);
        let sharpen = if p == 0 {
            button(ShopAction::Sharpen, "KEEN", 0, 0, false)
        } else {
            button(ShopAction::Sharpen, format!("SHARPEN {p}"), p, 0, true)
        };
        rows.push(ShopRow { kind: RowKind::Sharpen, buttons: vec![sharpen] });
        rows.push(self.shield_row());
        rows
    }

    /// The rows of the current tab, top to bottom: tabs, ten tiers, extras, and the way out.
    pub fn shop_rows(&self) -> Vec<ShopRow> {
        let hero = self.hero_kind;
        let h = self.progress.hero(hero);
        let tree = self.shop.tab;
        // A tab for each station in the order they stand, then the loadout.
        let mut tabs: Vec<ShopButton> = STATIONS
            .iter()
            .map(|&s| match s.tree() {
                Some(t) => button(ShopAction::Tab(t), s.label(), 0, 0, true),
                None => button(ShopAction::Board(s), s.label(), 0, 0, true),
            })
            .collect();
        tabs.push(button(ShopAction::Loadout, "LOADOUT", 0, 0, true));
        let mut rows = vec![ShopRow { kind: RowKind::Tabs, buttons: tabs }];
        let leave = if self.shop.called { "BACK TO THE DUNGEON" } else { "DOWN INTO THE DARK" };
        let leave_row = ShopRow { kind: RowKind::Leave, buttons: vec![button(ShopAction::Leave, leave, 0, 0, true)] };
        if self.shop.loadout {
            rows.extend(self.loadout_rows());
            rows.push(leave_row);
            return rows;
        }
        match self.shop.board {
            Some(Station::WarTable) => {
                // A row for each realm's general: known, or no word of him yet.
                for (i, g) in self.workshop.generals.iter().enumerate() {
                    let label = if g.is_some() { "KNOWN" } else { "NO WORD" };
                    let b = button(ShopAction::Board(Station::WarTable), label, 0, 0, false);
                    rows.push(ShopRow { kind: RowKind::General(i as u8), buttons: vec![b] });
                }
                rows.push(leave_row);
                return rows;
            }
            Some(_) => {
                for i in 0..TROPHY_SLOTS {
                    let label = if i < self.workshop.trophies.len() { "HUNG" } else { "EMPTY" };
                    let b = button(ShopAction::Board(Station::Trophies), label, 0, 0, false);
                    rows.push(ShopRow { kind: RowKind::Trophy(i as u8), buttons: vec![b] });
                }
                rows.push(leave_row);
                return rows;
            }
            None => {}
        }
        for tier in 1..=TIERS {
            let b = if h.has(tree, tier) {
                match tree {
                    Tree::Armory => {
                        let held = h.wield == tier;
                        button(ShopAction::Wield(tier), if held { "IN HAND" } else { "WIELD" }, 0, 0, !held)
                    }
                    Tree::Shield => {
                        let worn = self.hero.shield.map(|s| s.tier == tier).unwrap_or(false);
                        if h.is_broken(tier) && !worn {
                            let p = gear::reforge_price(tier);
                            button(ShopAction::Reforge, format!("REFORGE {p}"), p, 0, true)
                        } else {
                            button(ShopAction::Carry(tier), if worn { "ON YOUR ARM" } else { "CARRY" }, 0, 0, !worn)
                        }
                    }
                    Tree::Path => button(ShopAction::Unlock(tree, tier), "LEARNED", 0, 0, false),
                    Tree::Elixir => {
                        let kind = progression::ELIXIRS[(tier - 1) as usize];
                        let p = progression::bottle_price(kind);
                        button(ShopAction::Bottle(kind), format!("BUY {p}"), p, 0, true)
                    }
                }
            } else if h.next(tree) == Some(tier) {
                if h.found(tree, tier) {
                    button(ShopAction::Unlock(tree, tier), "UNSEAL", 0, 0, true)
                } else {
                    let p = progression::price(tree, tier);
                    let verb = if tree == Tree::Path { "LEARN" } else { "UNLOCK" };
                    button(ShopAction::Unlock(tree, tier), format!("{verb} {}", p.silver), p.silver, p.runes, true)
                }
            } else if h.found(tree, tier) {
                button(ShopAction::Unlock(tree, tier), "RELIC FOUND", 0, 0, false)
            } else if progression::sealed(tier) {
                button(ShopAction::Unlock(tree, tier), "SEALED", 0, 0, false)
            } else {
                button(ShopAction::Unlock(tree, tier), "LOCKED", 0, 0, false)
            };
            rows.push(ShopRow { kind: RowKind::Tier(tree, tier), buttons: vec![b] });
        }
        match tree {
            Tree::Shield => rows.push(self.shield_row()),
            Tree::Elixir => {
                let p = progression::bottle_price(PotionKind::Life);
                rows.push(ShopRow { kind: RowKind::Life, buttons: vec![button(ShopAction::Life, format!("BUY {p}"), p, 0, true)] });
            }
            // The yard keeps a place for the trials of mastery, not yet open.
            Tree::Path => rows.push(ShopRow { kind: RowKind::Trials, buttons: vec![button(ShopAction::Trials, "SOON", 0, 0, false)] }),
            _ => {}
        }
        rows.push(leave_row);
        rows
    }

    /// One update of the shop: move the highlight, hover, press.
    pub(super) fn update_shop(&mut self, dt: f64) {
        let dtf = dt as f32;
        self.update_workshop(dtf);
        self.shop.flourish = (self.shop.flourish - dtf * 1.5).max(0.0);
        if let Some(n) = &mut self.shop.note {
            n.life -= dtf;
            if n.life <= 0.0 {
                self.shop.note = None;
            }
        }

        let rows = self.shop_rows();
        let n = rows.len();
        self.shop.row = self.shop.row.min(n - 1);
        let wide = rows[self.shop.row].buttons.len();
        match self.input.nav {
            Some(Dir::Up) => self.shop.row = (self.shop.row + n - 1) % n,
            Some(Dir::Down) => self.shop.row = (self.shop.row + 1) % n,
            Some(Dir::Left) if self.shop.row == 0 => self.switch_tab(-1),
            Some(Dir::Right) if self.shop.row == 0 => self.switch_tab(1),
            // A row of several (the archangels, the outfits): left and right choose along it.
            Some(Dir::Left) if wide > 1 => self.shop.col = (self.shop.col + wide - 1) % wide,
            Some(Dir::Right) if wide > 1 => self.shop.col = (self.shop.col + 1) % wide,
            _ => {}
        }
        // LT and RT (or Tab, Y) step from station to station.
        if self.input.was_pressed(&[Action::NextPotion, Action::TabNext]) {
            self.switch_tab(1);
        }
        if self.input.was_pressed(&[Action::TabPrev]) {
            self.switch_tab(-1);
        }
        let rows = self.shop_rows();
        let n = rows.len();
        self.shop.row = self.shop.row.min(n - 1);
        let wide = rows[self.shop.row].buttons.len();
        self.shop.col = if self.shop.row == 0 {
            self.tab_index()
        } else if wide > 1 {
            self.shop.col.min(wide - 1)
        } else {
            0
        };

        let rects = hud::shop_rects(&rows, self.viewport.0, self.viewport.1);
        let hit = |p: (f32, f32)| {
            for (r, row) in rects.iter().enumerate() {
                for (c, b) in row.iter().enumerate() {
                    if p.0 >= b[0] && p.0 <= b[0] + b[2] && p.1 >= b[1] && p.1 <= b[1] + b[3] {
                        return Some((r, c));
                    }
                }
            }
            None
        };
        // Hovering a tier's line (not only its button) shows it in the card.
        let row_hit = |p: (f32, f32)| hud::shop_row_at(&rows, self.viewport.0, self.viewport.1, p);
        if self.input.cursor_moved {
            if let Some(p) = self.input.cursor {
                if let Some((r, c)) = hit(p) {
                    self.shop.row = r;
                    self.shop.col = c;
                } else if let Some(r) = row_hit(p) {
                    self.shop.row = r;
                }
            }
        }
        let mut pressed = None;
        if let Some(p) = self.input.click {
            if let Some((r, c)) = hit(p) {
                self.shop.row = r;
                self.shop.col = c;
                pressed = Some(rows[r].buttons[c].clone());
            } else if let Some(r) = row_hit(p) {
                self.shop.row = r;
            }
        } else if self.input.was_pressed(&[Action::Confirm, Action::Jump, Action::Attack]) && !self.input.was_pressed(&[Action::TabNext]) {
            pressed = Some(rows[self.shop.row].buttons[self.shop.col.min(rows[self.shop.row].buttons.len() - 1)].clone());
        }
        if self.input.was_pressed(&[Action::Pause]) {
            // Escape heads for the door; pressed again, it leaves.
            if self.shop.row == n - 1 {
                pressed = Some(rows[n - 1].buttons[0].clone());
            } else {
                self.shop.row = n - 1;
                self.shop.col = 0;
            }
        }
        if let Some(b) = pressed {
            self.press_shop(b);
        }
    }

    /// Which tab is open: a station's (0..6), or the loadout (6).
    fn tab_index(&self) -> usize {
        self.page()
    }

    fn switch_tab(&mut self, step: i32) {
        let n = PAGES as i32;
        let i = (self.tab_index() as i32 + step).rem_euclid(n) as usize;
        self.open_tab(i);
    }

    /// Opens tab `i`: a station's, or the loadout after them.
    fn open_tab(&mut self, i: usize) {
        self.open_page(i);
        if self.shop.row != 0 {
            self.shop.row = self.first_row_of_interest();
        }
        self.shop.col = 0;
        self.audio.play(Sfx::Nodule);
    }

    fn note(&mut self, text: impl Into<String>, color: u32) {
        self.shop.note = Some(ShopNote { text: text.into(), color, life: 2.6 });
    }

    /// Takes the silver if he has it; otherwise says so.
    fn pay(&mut self, price: u32) -> bool {
        if self.progress.silver < price {
            self.audio.play(Sfx::Denied);
            self.note(format!("NOT ENOUGH {} - {} MORE NEEDED", progression::coin(self.hero_kind), price - self.progress.silver), 0xff6a6a);
            return false;
        }
        self.progress.silver -= price;
        true
    }

    fn sold(&mut self, what: String, color: u32) {
        self.audio.play(Sfx::Buy);
        self.shop.flourish = 1.0;
        self.start_work(color);
        self.note(what, color);
        self.save_progress();
        self.remember_belt();
    }

    fn press_shop(&mut self, b: ShopButton) {
        if !b.enabled {
            self.audio.play(Sfx::Denied);
            return;
        }
        let hero = self.hero_kind;
        match b.action {
            ShopAction::Tab(t) => self.open_tab(Station::of_tree(t).index()),
            ShopAction::Board(s) => self.open_tab(s.index()),
            ShopAction::Loadout => self.open_tab(STATIONS.len()),
            ShopAction::Patron(a) => {
                if self.progress.hero(hero).patron == Some(a) {
                    self.note(format!("{} ALREADY WATCHES OVER YOU", a.name().to_uppercase()), a.color());
                    return;
                }
                self.progress.hero_mut(hero).patron = Some(a);
                self.patron_wait = 0.0;
                self.audio.play(Sfx::Milestone);
                self.shop.flourish = 1.0;
                self.note(
                    format!("{} WATCHES OVER YOU - HIS POWER RETURNS EVERY {} SECONDS", a.name().to_uppercase(), a.recharge() as u32),
                    a.color(),
                );
                self.save_progress();
            }
            ShopAction::Outfit(k) => {
                self.progress.hero_mut(hero).outfit = k;
                self.equip_from_progress();
                self.audio.play(Sfx::Start);
                self.note(format!("YOU WEAR THE {}", outfit_names(hero)[k as usize].to_uppercase()), 0xe8dcc0);
                self.save_progress();
            }
            ShopAction::Sharpen => {
                let h = *self.progress.hero(hero);
                if self.pay(gear::sharpen_price(h.wield, h.sharpness(h.wield))) {
                    self.progress.hero_mut(hero).edge[(h.wield - 1) as usize] = 100;
                    self.equip_from_progress();
                    let n = progression::node(hero, Tree::Armory, h.wield);
                    self.sold(format!("{} IS KEEN AGAIN", n.name.to_uppercase()), n.color);
                }
            }
            ShopAction::Reforge => {
                let h = *self.progress.hero(hero);
                // The broken one on his arm, or the one chosen on the shields' tab.
                let tier = match rows_tier(&self.shop_rows(), self.shop.row) {
                    Some(t) => t,
                    None => h.carry,
                };
                if self.pay(gear::reforge_price(tier)) {
                    let h = self.progress.hero_mut(hero);
                    h.broken &= !(1 << tier);
                    h.carry = tier;
                    self.hero.shield = Some(ShieldGear::new(tier));
                    self.equip_from_progress();
                    let n = progression::node(hero, Tree::Shield, tier);
                    self.sold(format!("{} FORGED ANEW - ON YOUR ARM", n.name.to_uppercase()), n.color);
                }
            }
            ShopAction::Leave => {
                self.audio.play(Sfx::Start);
                if self.shop.called {
                    // Back where he called from: the dungeon waited.
                    self.enter(Screen::Ready);
                } else {
                    self.after_shop();
                }
                self.juice.fade = 1.0;
                self.input.consume(Action::Attack);
            }
            ShopAction::Unlock(tree, tier) => match self.progress.unlock(hero, tree, tier) {
                Ok(_) => {
                    let n = progression::node(hero, tree, tier);
                    if tree == Tree::Shield {
                        self.progress.hero_mut(hero).broken &= !(1 << tier);
                        self.hero.shield = Some(ShieldGear::new(tier));
                    }
                    self.equip_from_progress();
                    let verb = match tree {
                        Tree::Armory => "IN YOUR HAND",
                        Tree::Path => "LEARNED",
                        Tree::Shield => "ON YOUR ARM",
                        Tree::Elixir => "BREWED - BOTTLES FOR SALE",
                    };
                    self.sold(format!("{} {verb}", n.name.to_uppercase()), n.color);
                }
                Err(Refusal::Silver(more)) => {
                    self.audio.play(Sfx::Denied);
                    self.note(format!("NOT ENOUGH {} - {more} MORE NEEDED", progression::coin(hero)), 0xff6a6a);
                }
                Err(Refusal::Runes(more)) => {
                    self.audio.play(Sfx::Denied);
                    self.note(format!("SEALED - {more} MORE {}, OR FIND ITS RELIC", progression::gems(hero, more)), 0x9ad1ff);
                }
                Err(_) => self.audio.play(Sfx::Denied),
            },
            ShopAction::Wield(tier) => {
                self.progress.hero_mut(hero).wield = tier;
                self.equip_from_progress();
                self.audio.play(Sfx::Start);
                let n = progression::node(hero, Tree::Armory, tier);
                self.note(format!("{} IN HAND", n.name.to_uppercase()), n.color);
                self.save_progress();
            }
            ShopAction::Carry(tier) => {
                self.progress.hero_mut(hero).carry = tier;
                self.hero.shield = Some(ShieldGear::new(tier));
                self.equip_from_progress();
                self.audio.play(Sfx::Start);
                let n = progression::node(hero, Tree::Shield, tier);
                self.note(format!("{} ON YOUR ARM", n.name.to_uppercase()), n.color);
                self.save_progress();
            }
            ShopAction::Repair => {
                let tier = self.hero.shield.map(|s| s.tier).unwrap_or(self.progress.hero(hero).carry);
                let fraction = self.hero.shield.map(|s| s.fraction()).unwrap_or(0.0);
                if self.pay(progression::repair_price(tier, fraction)) {
                    self.hero.shield = Some(ShieldGear::new(tier));
                    self.equip_from_progress();
                    let n = progression::node(hero, Tree::Shield, tier);
                    self.sold(format!("{} MENDED", n.name.to_uppercase()), n.color);
                }
            }
            ShopAction::Bottle(kind) => {
                let def = gear::potion(hero, kind);
                if self.belt.len() >= BELT_SLOTS {
                    self.audio.play(Sfx::Denied);
                    self.note("YOUR BELT IS FULL", 0xff6a6a);
                    return;
                }
                if self.pay(progression::bottle_price(kind)) {
                    self.belt.push(kind);
                    self.sold(format!("{} ON YOUR BELT", def.name.to_uppercase()), def.color);
                }
            }
            ShopAction::Trials => self.audio.play(Sfx::Denied),
            ShopAction::Life => {
                if self.score.lives >= MAX_LIVES {
                    self.audio.play(Sfx::Denied);
                    self.note("YOUR LIVES ARE ALREADY FULL", 0xff6a6a);
                    return;
                }
                if self.pay(progression::bottle_price(PotionKind::Life)) {
                    self.score.gain_life();
                    self.sold("AN EXTRA LIFE".into(), 0xff6d8d);
                }
            }
        }
    }
}

/// The shield tier a row of the shields' tab stands for (none for the other rows).
fn rows_tier(rows: &[ShopRow], row: usize) -> Option<u8> {
    match rows.get(row).map(|r| r.kind) {
        Some(RowKind::Tier(Tree::Shield, t)) => Some(t),
        _ => None,
    }
}

impl Game {
    /**
     * The gatekeeper's promise: wherever he is, the hero can call for help and
     * the old man's workshop opens (the dungeon waits): at the war table if he
     * has news, otherwise at the station the hero left.
     */
    pub fn call_old_man(&mut self) {
        self.audio.play(Sfx::Whoosh);
        let last = if self.shop.loadout { None } else { Some(self.station()) };
        self.shop = ShopState { called: true, ..ShopState::default() };
        match (self.workshop.news.is_empty(), last) {
            (false, _) => self.go_to_station(Station::WarTable),
            (true, Some(s)) => self.go_to_station(s),
            (true, None) => {
                self.open_page(STATIONS.len());
                self.shop.row = 1;
            }
        }
        self.enter(Screen::Shop);
        self.juice.fade = 1.0;
        self.input.consume(Action::Attack);
        self.input.consume(Action::Confirm);
    }
}
