//! Hrolf's forge: the blacksmith's shop between a boss and the next dungeon.
//! Three tabs (weapons, shields, potions) of rows of buttons, driven by the
//! keyboard, a controller or the mouse. The rows are built fresh from what
//! Sebastian owns, so labels and prices always tell the truth.

use super::{Game, Screen};
use crate::audio::Sfx;
use crate::input::Action;
use crate::logic::gear::{self, Line, ShieldGear, ShieldKind, BELT_SLOTS, LINES, MAX_LEVEL, POTIONS, SHIELDS};
use crate::logic::types::{Dir, PotionKind};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShopTab {
    Weapons,
    Shields,
    Potions,
}

pub const TABS: [ShopTab; 3] = [ShopTab::Weapons, ShopTab::Shields, ShopTab::Potions];

impl ShopTab {
    pub fn label(self) -> &'static str {
        match self {
            ShopTab::Weapons => "WEAPONS",
            ShopTab::Shields => "SHIELDS",
            ShopTab::Potions => "POTIONS",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ShopAction {
    Tab(ShopTab),
    Buy(Line),
    Upgrade(Line),
    Equip(Line),
    BuyShield(ShieldKind),
    Repair,
    Potion(PotionKind),
    Leave,
}

/// What a row of the shop shows; the HUD draws each kind its own way.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RowKind {
    Tabs,
    Weapon(Line),
    Shield(ShieldKind),
    Repair,
    Potions,
    Leave,
}

#[derive(Clone, Debug)]
pub struct ShopButton {
    pub action: ShopAction,
    pub label: String,
    /// Coins it costs, 0 if free.
    pub price: u32,
    /// Can it be pressed at all (owned, maxed and equipped ones cannot).
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
    pub tab: ShopTab,
    pub row: usize,
    pub col: usize,
    pub note: Option<ShopNote>,
    /// 1 when something was just bought, fading: the smith's hammer rings.
    pub flourish: f32,
}

impl Default for ShopState {
    fn default() -> Self {
        Self { tab: ShopTab::Weapons, row: 1, col: 0, note: None, flourish: 0.0 }
    }
}

fn button(action: ShopAction, label: impl Into<String>, price: u32, enabled: bool) -> ShopButton {
    ShopButton { action, label: label.into(), price, enabled }
}

impl Game {
    pub fn open_shop(&mut self) {
        self.shop = ShopState::default();
        self.enter(Screen::Shop);
    }

    /// The rows of the current tab, top to bottom: tabs, goods, and the way out.
    pub fn shop_rows(&self) -> Vec<ShopRow> {
        let mut rows = vec![ShopRow {
            kind: RowKind::Tabs,
            buttons: TABS.iter().map(|&t| button(ShopAction::Tab(t), t.label(), 0, true)).collect(),
        }];
        match self.shop.tab {
            ShopTab::Weapons => {
                for line in LINES {
                    let level = self.arsenal.level(line);
                    let mut b = Vec::new();
                    if level == 0 {
                        b.push(button(ShopAction::Buy(line), format!("BUY {}", line.price()), line.price(), true));
                    } else {
                        let equipped = self.arsenal.equipped == line;
                        b.push(button(ShopAction::Equip(line), if equipped { "EQUIPPED" } else { "EQUIP" }, 0, !equipped));
                        if level < MAX_LEVEL {
                            let p = line.upgrade_price(level);
                            b.push(button(ShopAction::Upgrade(line), format!("UPGRADE {p}"), p, true));
                        } else {
                            b.push(button(ShopAction::Upgrade(line), "MAX LEVEL", 0, false));
                        }
                    }
                    rows.push(ShopRow { kind: RowKind::Weapon(line), buttons: b });
                }
            }
            ShopTab::Shields => {
                for kind in SHIELDS {
                    let mine = self.hero.shield.map(|s| s.kind == kind).unwrap_or(false);
                    let def = gear::shield(kind);
                    let b = if mine {
                        button(ShopAction::BuyShield(kind), "ON YOUR ARM", 0, false)
                    } else {
                        button(ShopAction::BuyShield(kind), format!("BUY {}", def.price), def.price, true)
                    };
                    rows.push(ShopRow { kind: RowKind::Shield(kind), buttons: vec![b] });
                }
                if let Some(s) = self.hero.shield {
                    let p = s.repair_price();
                    let b = if p == 0 { button(ShopAction::Repair, "GOOD AS NEW", 0, false) } else { button(ShopAction::Repair, format!("REPAIR {p}"), p, true) };
                    rows.push(ShopRow { kind: RowKind::Repair, buttons: vec![b] });
                }
            }
            ShopTab::Potions => {
                for pair in POTIONS.chunks(2) {
                    let buttons = pair
                        .iter()
                        .map(|&k| {
                            let p = gear::potion(k).price;
                            button(ShopAction::Potion(k), format!("BUY {p}"), p, true)
                        })
                        .collect();
                    rows.push(ShopRow { kind: RowKind::Potions, buttons });
                }
            }
        }
        rows.push(ShopRow { kind: RowKind::Leave, buttons: vec![button(ShopAction::Leave, "LEAVE THE FORGE", 0, true)] });
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
        // The smith's hammer rings on the anvil in time with his swing.
        let beat = |t: f64| (t * crate::render::view::SMITH_BEAT).floor();
        if beat(self.time) != beat(self.time - dt) {
            self.audio.play(Sfx::Anvil);
        }

        let rows = self.shop_rows();
        let n = rows.len();
        self.shop.row = self.shop.row.min(n - 1);
        match self.input.nav {
            Some(Dir::Up) => self.shop.row = (self.shop.row + n - 1) % n,
            Some(Dir::Down) => self.shop.row = (self.shop.row + 1) % n,
            Some(Dir::Left) if self.shop.row == 0 => self.switch_tab(-1),
            Some(Dir::Right) if self.shop.row == 0 => self.switch_tab(1),
            Some(Dir::Left) => self.shop.col = self.shop.col.saturating_sub(1),
            Some(Dir::Right) => self.shop.col += 1,
            _ => {}
        }
        if self.input.was_pressed(&[Action::NextPotion]) {
            self.switch_tab(1);
        }
        let rows = self.shop_rows();
        let n = rows.len();
        self.shop.row = self.shop.row.min(n - 1);
        if self.shop.row == 0 {
            self.shop.col = TABS.iter().position(|&t| t == self.shop.tab).unwrap_or(0);
        }
        self.shop.col = self.shop.col.min(rows[self.shop.row].buttons.len() - 1);

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
        if self.input.cursor_moved {
            if let Some((r, c)) = self.input.cursor.and_then(hit) {
                self.shop.row = r;
                self.shop.col = c;
            }
        }
        let mut pressed = None;
        if let Some(p) = self.input.click {
            if let Some((r, c)) = hit(p) {
                self.shop.row = r;
                self.shop.col = c;
                pressed = Some(rows[r].buttons[c].clone());
            }
        } else if self.input.was_pressed(&[Action::Confirm, Action::Jump, Action::Attack]) {
            pressed = Some(rows[self.shop.row].buttons[self.shop.col].clone());
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
        let i = TABS.iter().position(|&t| t == self.shop.tab).unwrap_or(0) as i32;
        self.shop.tab = TABS[(i + step).rem_euclid(TABS.len() as i32) as usize];
        self.audio.play(Sfx::Nodule);
    }

    fn note(&mut self, text: impl Into<String>, color: u32) {
        self.shop.note = Some(ShopNote { text: text.into(), color, life: 2.6 });
    }

    /// Takes the coins if he has them; otherwise says so.
    fn pay(&mut self, price: u32) -> bool {
        if self.purse < price {
            self.audio.play(Sfx::Denied);
            self.note(format!("NOT ENOUGH COINS - {} MORE NEEDED", price - self.purse), 0xff6a6a);
            return false;
        }
        self.purse -= price;
        true
    }

    fn sold(&mut self, what: String, color: u32) {
        self.audio.play(Sfx::Buy);
        self.shop.flourish = 1.0;
        self.note(what, color);
    }

    fn press_shop(&mut self, b: ShopButton) {
        if !b.enabled {
            self.audio.play(Sfx::Denied);
            return;
        }
        match b.action {
            ShopAction::Tab(t) => {
                self.shop.tab = t;
                self.audio.play(Sfx::Nodule);
            }
            ShopAction::Leave => {
                self.audio.play(Sfx::Start);
                let next = self.dungeon + 1;
                self.load_level(next);
                self.juice.fade = 1.0;
                self.enter(Screen::Ready);
                self.input.consume(Action::Attack);
            }
            ShopAction::Buy(line) => {
                if self.pay(line.price()) {
                    self.arsenal.levels[line.index()] = 1;
                    self.arsenal.equipped = line;
                    self.hero.arm(&self.arsenal);
                    self.sold(format!("{} FORGED AND EQUIPPED", line.name(1).to_uppercase()), line.color(1));
                }
            }
            ShopAction::Upgrade(line) => {
                let level = self.arsenal.level(line);
                if self.pay(line.upgrade_price(level)) {
                    self.arsenal.levels[line.index()] = level + 1;
                    self.hero.arm(&self.arsenal);
                    self.sold(format!("UPGRADED TO {}", line.name(level + 1).to_uppercase()), line.color(level + 1));
                }
            }
            ShopAction::Equip(line) => {
                self.arsenal.equipped = line;
                self.hero.arm(&self.arsenal);
                self.audio.play(Sfx::Start);
                let level = self.arsenal.level(line);
                self.note(format!("{} IN HAND", line.name(level).to_uppercase()), line.color(level));
            }
            ShopAction::BuyShield(kind) => {
                if self.pay(gear::shield(kind).price) {
                    self.hero.shield = Some(ShieldGear::new(kind));
                    self.sold(format!("{} ON YOUR ARM", gear::shield(kind).name.to_uppercase()), gear::shield(kind).color);
                }
            }
            ShopAction::Repair => {
                let Some(s) = self.hero.shield else { return };
                if self.pay(s.repair_price()) {
                    self.hero.grant_shield();
                    self.sold("SHIELD HAMMERED BACK TO FULL".into(), gear::shield(s.kind).color);
                }
            }
            ShopAction::Potion(kind) => {
                let def = gear::potion(kind);
                if kind == PotionKind::Life && self.score.lives >= crate::logic::systems::MAX_LIVES {
                    self.audio.play(Sfx::Denied);
                    self.note("YOUR LIVES ARE ALREADY FULL", 0xff6a6a);
                    return;
                }
                if kind != PotionKind::Life && self.belt.len() >= BELT_SLOTS {
                    self.audio.play(Sfx::Denied);
                    self.note("YOUR BELT IS FULL", 0xff6a6a);
                    return;
                }
                if self.pay(def.price) {
                    if kind == PotionKind::Life {
                        self.score.gain_life();
                        self.sold("AN EXTRA LIFE".into(), def.color);
                    } else {
                        self.belt.push(kind);
                        self.sold(format!("{} POTION ON YOUR BELT", def.name.to_uppercase()), def.color);
                    }
                }
            }
        }
    }
}
