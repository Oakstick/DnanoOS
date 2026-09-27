//! What the blacksmith sells: four weapon lines with three levels each,
//! four shields that wear down as they block, and the potions Sebastian
//! carries on his belt. Prices, stats and names live here; the shop screen
//! and the combat code read them.

use super::items::RangedKind;
use super::types::PotionKind;

// ------------------------------------------------------------------ weapons

/// A family of weapons the smith can forge and improve.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Line {
    Axe,
    Arrow,
    Knife,
    Boomerang,
}

pub const LINES: [Line; 4] = [Line::Axe, Line::Arrow, Line::Knife, Line::Boomerang];
pub const MAX_LEVEL: u8 = 3;

impl Line {
    pub fn index(self) -> usize {
        match self {
            Line::Axe => 0,
            Line::Arrow => 1,
            Line::Knife => 2,
            Line::Boomerang => 3,
        }
    }

    /// The weapon's name at `level` (1..=3).
    pub fn name(self, level: u8) -> &'static str {
        match (self, level.clamp(1, 3)) {
            (Line::Axe, 1) => "Axe",
            (Line::Axe, 2) => "Battle Axe",
            (Line::Axe, _) => "Great Axe",
            (Line::Arrow, 1) => "Arrows",
            (Line::Arrow, 2) => "Flame Arrows",
            (Line::Arrow, _) => "Holy Arrows",
            (Line::Knife, 1) => "Throwing Knife",
            (Line::Knife, 2) => "Twin Knives",
            (Line::Knife, _) => "Seeking Knives",
            (Line::Boomerang, 1) => "Boomerang",
            (Line::Boomerang, 2) => "Steel Boomerang",
            (Line::Boomerang, _) => "Fire Boomerang",
        }
    }

    /// What the level adds, in a few words for the shop.
    pub fn blurb(self, level: u8) -> &'static str {
        match (self, level.clamp(1, 3)) {
            (Line::Axe, 1) => "Close and deadly. Your trusty bearded axe.",
            (Line::Axe, 2) => "Heavier blow, longer reach, wider swing.",
            (Line::Axe, _) => "Double-bladed. A huge arc that hits all around.",
            (Line::Arrow, 1) => "A straight, fast shot down the corridor.",
            (Line::Arrow, 2) => "Three burning arrows side by side.",
            (Line::Arrow, _) => "Blazing light that pierces everything. Highest damage.",
            (Line::Knife, 1) => "Quick to throw, quick to throw again.",
            (Line::Knife, 2) => "Two knives at once.",
            (Line::Knife, _) => "Three knives that curve after your foes.",
            (Line::Boomerang, 1) => "Cuts through foes and flies back to you.",
            (Line::Boomerang, 2) => "Harder, farther, a wider return path.",
            (Line::Boomerang, _) => "Sets foes ablaze. Highest boomerang damage.",
        }
    }

    pub fn color(self, level: u8) -> u32 {
        match (self, level.clamp(1, 3)) {
            (Line::Axe, 1) => 0xcfe8f0,
            (Line::Axe, 2) => 0xff8a6a,
            (Line::Axe, _) => 0xffd84a,
            (Line::Arrow, 1) => 0xe0c9a6,
            (Line::Arrow, 2) => 0xff8a2a,
            (Line::Arrow, _) => 0xfff1a8,
            (Line::Knife, 1) => 0xc8d0da,
            (Line::Knife, 2) => 0xdfe6ee,
            (Line::Knife, _) => 0x9ad1ff,
            (Line::Boomerang, 1) => 0xc98f4a,
            (Line::Boomerang, 2) => 0xb8c8d8,
            (Line::Boomerang, _) => 0xff7a2a,
        }
    }

    /// Coins to buy the first level (the axe is already his).
    pub fn price(self) -> u32 {
        match self {
            Line::Axe => 0,
            Line::Arrow => 120,
            Line::Knife => 150,
            Line::Boomerang => 180,
        }
    }

    /// Coins to go from `level` to `level + 1`.
    pub fn upgrade_price(self, level: u8) -> u32 {
        let base = match self {
            Line::Axe => 160,
            Line::Arrow => 180,
            Line::Knife => 190,
            Line::Boomerang => 210,
        };
        if level <= 1 {
            base
        } else {
            base * 2 + 60
        }
    }

    /// The projectile a ranged line throws at `level`; None for the axe.
    pub fn ranged(self, level: u8) -> Option<RangedKind> {
        Some(match (self, level.clamp(1, 3)) {
            (Line::Axe, _) => return None,
            (Line::Arrow, 1) => RangedKind::Arrow,
            (Line::Arrow, 2) => RangedKind::FlameArrow,
            (Line::Arrow, _) => RangedKind::HolyArrow,
            (Line::Knife, 1) => RangedKind::Knife,
            (Line::Knife, 2) => RangedKind::TwinKnives,
            (Line::Knife, _) => RangedKind::SeekingKnives,
            (Line::Boomerang, 1) => RangedKind::Boomerang,
            (Line::Boomerang, 2) => RangedKind::SteelBoomerang,
            (Line::Boomerang, _) => RangedKind::FireBoomerang,
        })
    }
}

/// How an axe of a given level swings.
#[derive(Clone, Copy, Debug)]
pub struct AxeStats {
    /// Tiles in front of him the blade reaches.
    pub reach: f64,
    /// Half-width of the swing to either side.
    pub half_width: f64,
    /// How far behind his shoulder line the arc still bites.
    pub behind: f64,
    /// Damage to a boss per blow.
    pub damage: i32,
    pub cooldown: f64,
}

pub fn axe(level: u8) -> AxeStats {
    match level.clamp(1, 3) {
        1 => AxeStats { reach: 1.2, half_width: 0.7, behind: 0.25, damage: 2, cooldown: 0.45 },
        2 => AxeStats { reach: 1.5, half_width: 0.95, behind: 0.3, damage: 3, cooldown: 0.45 },
        _ => AxeStats { reach: 1.75, half_width: 1.35, behind: 0.7, damage: 5, cooldown: 0.5 },
    }
}

/// The weapons he owns (level per line, 0 = not owned) and the one in hand.
#[derive(Clone, Debug)]
pub struct Arsenal {
    pub levels: [u8; 4],
    pub equipped: Line,
}

impl Default for Arsenal {
    fn default() -> Self {
        Self { levels: [1, 0, 0, 0], equipped: Line::Axe }
    }
}

impl Arsenal {
    pub fn level(&self, line: Line) -> u8 {
        self.levels[line.index()]
    }
    pub fn owns(&self, line: Line) -> bool {
        self.level(line) > 0
    }
    /// The axe's level, which is what he swings when the axe is in hand.
    pub fn axe_level(&self) -> u8 {
        self.level(Line::Axe).max(1)
    }
    /// The ranged weapon in hand, if the equipped line is not the axe.
    pub fn main_ranged(&self) -> Option<RangedKind> {
        self.equipped.ranged(self.level(self.equipped))
    }
}

// ------------------------------------------------------------------ shields

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ShieldKind {
    Round,
    IronRound,
    Kite,
    Dreadnought,
}

pub const SHIELDS: [ShieldKind; 4] = [ShieldKind::Round, ShieldKind::IronRound, ShieldKind::Kite, ShieldKind::Dreadnought];

#[derive(Clone, Copy, Debug)]
pub struct ShieldDef {
    pub name: &'static str,
    pub blurb: &'static str,
    pub color: u32,
    pub price: u32,
    /// Damage it can soak up before it breaks.
    pub durability: i32,
    /// Share of each hit it takes instead of him.
    pub block: f64,
}

pub fn shield(kind: ShieldKind) -> ShieldDef {
    match kind {
        ShieldKind::Round => ShieldDef { name: "Round Shield", blurb: "Wood and a boss of iron. Takes half of each hit.", color: 0xb8864a, price: 60, durability: 50, block: 0.5 },
        ShieldKind::IronRound => ShieldDef { name: "Iron Round Shield", blurb: "Iron-banded. Blocks more and lasts longer.", color: 0xb8c0cc, price: 160, durability: 100, block: 0.6 },
        ShieldKind::Kite => ShieldDef { name: "Kite Shield", blurb: "Tall and sturdy. Takes most of every hit.", color: 0xe0443a, price: 260, durability: 160, block: 0.72 },
        ShieldKind::Dreadnought => ShieldDef { name: "Dreadnought Shield", blurb: "Riveted steel. Barely a scratch gets through.", color: 0x8fa8c8, price: 420, durability: 260, block: 0.85 },
    }
}

/// A shield on his arm and how much it has left.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ShieldGear {
    pub kind: ShieldKind,
    pub durability: i32,
}

impl ShieldGear {
    pub fn new(kind: ShieldKind) -> Self {
        Self { kind, durability: shield(kind).durability }
    }
    pub fn fraction(&self) -> f32 {
        self.durability as f32 / shield(self.kind).durability as f32
    }
    /// Coins to hammer it back to full.
    pub fn repair_price(&self) -> u32 {
        let missing = 1.0 - self.fraction();
        ((shield(self.kind).price as f32 * 0.6 * missing).ceil() as u32).max(if missing > 0.0 { 5 } else { 0 })
    }
}

// ------------------------------------------------------------------ potions

/// How many potions fit on his belt.
pub const BELT_SLOTS: usize = 5;

#[derive(Clone, Copy, Debug)]
pub struct PotionDef {
    pub name: &'static str,
    pub blurb: &'static str,
    pub color: u32,
    pub price: u32,
    /// Seconds the effect lasts; 0 for instant ones.
    pub seconds: f64,
}

pub const POTIONS: [PotionKind; 11] = [
    PotionKind::Health,
    PotionKind::Regeneration,
    PotionKind::Swiftness,
    PotionKind::NightVision,
    PotionKind::TrueSight,
    PotionKind::Strength,
    PotionKind::IronSkin,
    PotionKind::Arcane,
    PotionKind::Lightning,
    PotionKind::Invulnerability,
    PotionKind::Life,
];

pub fn potion(kind: PotionKind) -> PotionDef {
    match kind {
        PotionKind::Health => PotionDef { name: "Health", blurb: "Restores 50 health, and a moment of immunity.", color: 0x3fd24a, price: 40, seconds: 0.0 },
        PotionKind::Regeneration => PotionDef { name: "Regeneration", blurb: "Heals 5 health a second for 12 seconds.", color: 0xa6f04a, price: 60, seconds: 12.0 },
        PotionKind::Swiftness => PotionDef { name: "Swiftness", blurb: "Run faster for 15 seconds.", color: 0xffd23a, price: 60, seconds: 15.0 },
        PotionKind::NightVision => PotionDef { name: "Night Vision", blurb: "See clearly in the dark woods for 45 seconds.", color: 0x4a6cff, price: 50, seconds: 45.0 },
        PotionKind::TrueSight => PotionDef { name: "True Sight", blurb: "Reveals every nugget and treasure, even through walls.", color: 0xff4ad8, price: 80, seconds: 30.0 },
        PotionKind::Strength => PotionDef { name: "Strength", blurb: "Doubles weapon damage for 20 seconds.", color: 0xe0303a, price: 90, seconds: 20.0 },
        PotionKind::IronSkin => PotionDef { name: "Iron Skin", blurb: "Takes 60% less damage for 20 seconds.", color: 0x9aa4b0, price: 90, seconds: 20.0 },
        PotionKind::Arcane => PotionDef { name: "Arcane", blurb: "Attack twice as often for 20 seconds.", color: 0x9a4aff, price: 90, seconds: 20.0 },
        PotionKind::Lightning => PotionDef { name: "Lightning", blurb: "Every attack also zaps nearby foes. 20 seconds.", color: 0xff9a2a, price: 110, seconds: 20.0 },
        PotionKind::Invulnerability => PotionDef { name: "Invulnerability", blurb: "Nothing can hurt you for 8 seconds.", color: 0xffe066, price: 150, seconds: 8.0 },
        PotionKind::Life => PotionDef { name: "Extra Life", blurb: "One more life, up to three.", color: 0xff6d8d, price: 350, seconds: 0.0 },
    }
}

/// Potions lying in the first dungeon, in order of the bottles it places.
pub const FOUND_POTIONS: [PotionKind; 5] = [PotionKind::Life, PotionKind::Health, PotionKind::Strength, PotionKind::Swiftness, PotionKind::TrueSight];

// -------------------------------------------------------------------- coins

/// Coins for each golden key, each foe, and a fallen boss.
pub const COINS_KEY: u32 = 30;
pub const COINS_KILL: u32 = 5;
pub const COINS_BOSS: u32 = 250;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_line_has_three_named_levels_and_rising_prices() {
        for line in LINES {
            for level in 1..=MAX_LEVEL {
                assert!(!line.name(level).is_empty());
                assert_eq!(line.ranged(level).is_none(), line == Line::Axe);
            }
            assert!(line.upgrade_price(2) > line.upgrade_price(1));
        }
        assert!(axe(3).damage > axe(2).damage && axe(2).damage > axe(1).damage);
        assert!(axe(3).half_width > axe(1).half_width);
    }

    #[test]
    fn better_shields_block_more_and_last_longer() {
        for w in SHIELDS.windows(2) {
            let (a, b) = (shield(w[0]), shield(w[1]));
            assert!(b.block > a.block && b.durability > a.durability && b.price > a.price);
        }
        let mut s = ShieldGear::new(ShieldKind::Kite);
        assert_eq!(s.repair_price(), 0);
        s.durability /= 2;
        assert!(s.repair_price() > 0);
    }
}
