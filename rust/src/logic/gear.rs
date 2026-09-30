//! What the hero carries into a dungeon: the shield on his arm (from his
//! shield tree, wearing down as it blocks), the potions on his belt (brewed
//! from his elixir tree), and what silver and runes each deed is worth. His
//! weapons, shields and elixirs themselves are defined in `progression`.

use super::heroes::HeroKind;
use super::progression::{self, Tree};
use super::types::PotionKind;

// ------------------------------------------------------------------ shields

/// A shield on his arm (tier 1..=10 of his shield tree) and how much it has left.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ShieldGear {
    pub tier: u8,
    pub durability: i32,
    /// Mending carried between whole points.
    pub mend_carry: f64,
}

impl ShieldGear {
    pub fn new(tier: u8) -> Self {
        Self { tier, durability: progression::ward(tier).durability, mend_carry: 0.0 }
    }
    pub fn max(&self) -> i32 {
        progression::ward(self.tier).durability
    }
    pub fn fraction(&self) -> f32 {
        self.durability as f32 / self.max() as f32
    }
    /// Silver to hammer it back to full.
    pub fn repair_price(&self) -> u32 {
        progression::repair_price(self.tier, self.fraction())
    }
}

// ------------------------------------------------------------------ potions

/// How many potions fit on his belt.
pub const BELT_SLOTS: usize = 5;

#[derive(Clone, Copy, Debug)]
pub struct PotionDef {
    pub name: &'static str,
    pub color: u32,
    /// Seconds the effect lasts; 0 for instant ones.
    pub seconds: f64,
}

/// Seconds a brew's effect lasts; 0 for instant ones.
pub fn potion_seconds(kind: PotionKind) -> f64 {
    match kind {
        PotionKind::Strength => 20.0,
        PotionKind::Haste => 10.0,
        PotionKind::Fury | PotionKind::Fire => 15.0,
        PotionKind::Invulnerability => 5.0,
        PotionKind::Ragnarok => 20.0,
        PotionKind::Spirit => 30.0,
        _ => 0.0,
    }
}

/// A potion as `hero` knows it: his elixir tree names the brew; an extra life is the same for all.
pub fn potion(hero: HeroKind, kind: PotionKind) -> PotionDef {
    let seconds = potion_seconds(kind);
    match progression::elixir_tier(kind) {
        Some(tier) => {
            let n = progression::node(hero, Tree::Elixir, tier);
            PotionDef { name: n.name, color: n.color, seconds }
        }
        None => PotionDef { name: "Extra Life", color: 0xff6d8d, seconds },
    }
}

/// Every bottle there is, for building their models: the ten brews and an extra life.
pub const ALL_POTIONS: [PotionKind; 11] = [
    PotionKind::Health,
    PotionKind::Stamina,
    PotionKind::Strength,
    PotionKind::Haste,
    PotionKind::Bark,
    PotionKind::Fury,
    PotionKind::Fire,
    PotionKind::Invulnerability,
    PotionKind::Ragnarok,
    PotionKind::Spirit,
    PotionKind::Life,
];

/// Potions lying in the first dungeon, in order of the bottles it places.
pub const FOUND_POTIONS: [PotionKind; 5] =
    [PotionKind::Life, PotionKind::Health, PotionKind::Strength, PotionKind::Haste, PotionKind::Stamina];

// ------------------------------------------------------------ silver and runes

/// Silver for each golden key, each foe, and a fallen boss.
pub const COINS_KEY: u32 = 30;
pub const COINS_KILL: u32 = 5;
pub const COINS_BOSS: u32 = 250;
/// Rune fragments a fallen Cyclops leaves (one more for each dungeon deeper), and one in each hidden shrine.
pub const RUNES_BOSS: u32 = 3;
pub const RUNES_SHRINE: u32 = 1;
/// The chance that a slain foe leaves a rune fragment.
pub const RUNE_DROP: f64 = 0.04;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shield_mends_for_silver_in_proportion_to_its_dents() {
        let mut s = ShieldGear::new(4);
        assert_eq!(s.repair_price(), 0);
        s.durability /= 2;
        let half = s.repair_price();
        s.durability = 0;
        assert!(s.repair_price() > half && half > 0);
    }

    #[test]
    fn each_hero_names_his_own_brews() {
        assert_eq!(potion(HeroKind::Ragnar, PotionKind::Health).name, "Minor Health Potion");
        assert_eq!(potion(HeroKind::Mehmed, PotionKind::Haste).name, "Elixir of the Desert Wind");
        assert_eq!(potion(HeroKind::Miyamoto, PotionKind::Life).name, "Extra Life");
        assert!(potion(HeroKind::Ragnar, PotionKind::Invulnerability).seconds > 0.0);
    }
}
