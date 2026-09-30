//! The old gatekeeper's wares: after each Cyclops, or wherever the hero calls
//! him (he promised he would come). Four tabs, one for each of the hero's
//! trees (weapons, path, shields, elixirs), each a column of ten tiers:
//! unlock the next, wield or carry what is owned, buy bottles of the elixirs
//! learned. Driven by the keyboard, a controller or the mouse. The rows are
//! built fresh from the hero's progress, so labels and prices always tell the truth.

use super::{Game, Screen};
use crate::audio::Sfx;
use crate::input::Action;
use crate::logic::gear::{self, ShieldGear, BELT_SLOTS};
use crate::logic::progression::{self, Refusal, Tree, TIERS, TREES};
use crate::logic::types::{Dir, PotionKind};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ShopAction {
    Tab(Tree),
    /// Pay for (or unseal with a relic) the next tier of a tree.
    Unlock(Tree, u8),
    /// Take an owned weapon in hand, or an owned shield on the arm.
    Wield(u8),
    Carry(u8),
    /// A bottle of an unlocked elixir for the belt.
    Bottle(PotionKind),
    Repair,
    Life,
    Leave,
}

/// What a row of the shop shows; the HUD draws each kind its own way.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RowKind {
    Tabs,
    Tier(Tree, u8),
    Repair,
    Life,
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
}

impl Default for ShopState {
    fn default() -> Self {
        Self { tab: Tree::Armory, row: 1, col: 0, note: None, flourish: 0.0, called: false }
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

    /// The row to start on: the next tier to take in the open tab.
    fn first_row_of_interest(&self) -> usize {
        let owned = self.progress.hero(self.hero_kind).owned(self.shop.tab);
        (owned as usize + 1).min(TIERS as usize)
    }

    /// The rows of the current tab, top to bottom: tabs, ten tiers, extras, and the way out.
    pub fn shop_rows(&self) -> Vec<ShopRow> {
        let hero = self.hero_kind;
        let h = self.progress.hero(hero);
        let tree = self.shop.tab;
        let mut rows = vec![ShopRow {
            kind: RowKind::Tabs,
            buttons: TREES.iter().map(|&t| button(ShopAction::Tab(t), t.label(), 0, 0, true)).collect(),
        }];
        for tier in 1..=TIERS {
            let b = if h.has(tree, tier) {
                match tree {
                    Tree::Armory => {
                        let held = h.wield == tier;
                        button(ShopAction::Wield(tier), if held { "IN HAND" } else { "WIELD" }, 0, 0, !held)
                    }
                    Tree::Shield => {
                        let worn = self.hero.shield.map(|s| s.tier == tier).unwrap_or(false);
                        button(ShopAction::Carry(tier), if worn { "ON YOUR ARM" } else { "CARRY" }, 0, 0, !worn)
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
            Tree::Shield => {
                let tier = self.hero.shield.map(|s| s.tier).unwrap_or(h.carry);
                let fraction = self.hero.shield.map(|s| s.fraction()).unwrap_or(0.0);
                let p = progression::repair_price(tier, fraction);
                let b = if p == 0 { button(ShopAction::Repair, "GOOD AS NEW", 0, 0, false) } else { button(ShopAction::Repair, format!("MEND {p}"), p, 0, true) };
                rows.push(ShopRow { kind: RowKind::Repair, buttons: vec![b] });
            }
            Tree::Elixir => {
                let p = progression::bottle_price(PotionKind::Life);
                rows.push(ShopRow { kind: RowKind::Life, buttons: vec![button(ShopAction::Life, format!("BUY {p}"), p, 0, true)] });
            }
            _ => {}
        }
        let leave = if self.shop.called { "BACK TO THE DUNGEON" } else { "DOWN INTO THE DARK" };
        rows.push(ShopRow { kind: RowKind::Leave, buttons: vec![button(ShopAction::Leave, leave, 0, 0, true)] });
        rows
    }

    /// One update of the shop: move the highlight, hover, press.
    pub(super) fn update_shop(&mut self, dt: f64) {
        let dtf = dt as f32;
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
        match self.input.nav {
            Some(Dir::Up) => self.shop.row = (self.shop.row + n - 1) % n,
            Some(Dir::Down) => self.shop.row = (self.shop.row + 1) % n,
            Some(Dir::Left) if self.shop.row == 0 => self.switch_tab(-1),
            Some(Dir::Right) if self.shop.row == 0 => self.switch_tab(1),
            _ => {}
        }
        // LT and RT (or Tab, Y) page through the trees.
        if self.input.was_pressed(&[Action::NextPotion, Action::TabNext]) {
            self.switch_tab(1);
        }
        if self.input.was_pressed(&[Action::TabPrev]) {
            self.switch_tab(-1);
        }
        let rows = self.shop_rows();
        let n = rows.len();
        self.shop.row = self.shop.row.min(n - 1);
        self.shop.col = if self.shop.row == 0 { TREES.iter().position(|&t| t == self.shop.tab).unwrap_or(0) } else { 0 };

        let rects = crate::render::hud::shop_rects(&rows, self.viewport.0, self.viewport.1);
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
        let row_hit = |p: (f32, f32)| crate::render::hud::shop_row_at(&rows, self.viewport.0, self.viewport.1, p);
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

    fn switch_tab(&mut self, step: i32) {
        let i = TREES.iter().position(|&t| t == self.shop.tab).unwrap_or(0) as i32;
        self.shop.tab = TREES[(i + step).rem_euclid(TREES.len() as i32) as usize];
        if self.shop.row != 0 {
            self.shop.row = self.first_row_of_interest();
        }
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
            ShopAction::Tab(t) => {
                self.shop.tab = t;
                self.audio.play(Sfx::Nodule);
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
            ShopAction::Life => {
                if self.score.lives >= crate::logic::systems::MAX_LIVES {
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

impl Game {
    /**
     * The gatekeeper's promise: wherever he is, the hero can call for help and
     * the old man's wares open (the dungeon waits).
     */
    pub fn call_old_man(&mut self) {
        self.audio.play(Sfx::Whoosh);
        self.shop = ShopState { called: true, ..ShopState::default() };
        self.enter(Screen::Shop);
        self.juice.fade = 1.0;
        self.input.consume(Action::Attack);
        self.input.consume(Action::Confirm);
    }
}
