//! Weapons, shrine boons, potions, bombs and shrines: the small rule objects.

use super::gear;
use super::types::{PotionKind, P};

// ------------------------------------------------------------------ weapons

/// Ranged weapons offered by the weapon shrine, in cycle order.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum RangedKind {
    Arrow,
    Double,
    Trishot,
    Boomerang,
    Fireball,
    Crescent,
    // Forged by the blacksmith.
    FlameArrow,
    HolyArrow,
    Knife,
    TwinKnives,
    SeekingKnives,
    SteelBoomerang,
    FireBoomerang,
}

/// Everything the hero can hold: the default sword, or one ranged weapon that replaces it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum WeaponKind {
    Sword,
    Ranged(RangedKind),
}

/// The melee weapon: Ragnar's bearded axe.
pub const SWORD_NAME: &str = "Axe";
pub const SWORD_COLOR: u32 = 0xcfe8f0;

#[derive(Clone, Copy, Debug)]
pub struct WeaponDef {
    pub name: &'static str,
    pub color: u32,
    /// Shots granted on pickup.
    pub ammo: u32,
    /// Tiles per second.
    pub speed: f64,
    /// Tiles before the shot drops (or, for the boomerang, turns back).
    pub range: f64,
    /// Distance from the shot's center at which an enemy is hit.
    pub hit_radius: f64,
    /// Keeps flying after a kill.
    pub pierce: bool,
    /// Flies back along its path and refunds the shot if caught.
    pub returns: bool,
    /// Lateral offsets (tiles) for parallel shots; one entry per projectile.
    pub lateral: &'static [f64],
    /// Also fire perpendicular copies (left and right of facing).
    pub spread: bool,
    pub cooldown: f64,
    /// Damage to a boss per hit (ordinary foes fall to any hit).
    pub damage: i32,
    /// Curves toward the nearest foe.
    pub homing: bool,
    /// Sets what it hits burning.
    pub burn: bool,
}

pub const WEAPON_ORDER: [RangedKind; 6] = [
    RangedKind::Arrow,
    RangedKind::Double,
    RangedKind::Trishot,
    RangedKind::Boomerang,
    RangedKind::Fireball,
    RangedKind::Crescent,
];

pub fn weapon(kind: RangedKind) -> WeaponDef {
    let base = WeaponDef {
        name: "", color: 0xffffff, ammo: 6, speed: 14.0, range: 14.0, hit_radius: 0.5,
        pierce: false, returns: false, lateral: &[0.0], spread: false, cooldown: 0.3,
        damage: 1, homing: false, burn: false,
    };
    match kind {
        RangedKind::Arrow => WeaponDef { name: "Arrow", color: 0xe0c9a6, ammo: 7, cooldown: 0.25, ..base },
        RangedKind::Double => WeaponDef { name: "Double Arrow", color: 0xf2e3c4, ammo: 7, lateral: &[-0.28, 0.28], ..base },
        RangedKind::Trishot => WeaponDef { name: "Tri-Shot", color: 0x9ad1ff, speed: 13.0, range: 12.0, spread: true, cooldown: 0.35, ..base },
        RangedKind::Boomerang => WeaponDef {
            name: "Boomerang", color: 0xc98f4a, ammo: 5, speed: 10.0, range: 5.0, hit_radius: 0.6,
            pierce: true, returns: true, cooldown: 0.4, damage: 2, ..base
        },
        RangedKind::Fireball => WeaponDef {
            name: "Fireball", color: 0xff8a2a, ammo: 5, speed: 8.0, range: 12.0, hit_radius: 0.6, pierce: true, cooldown: 0.45, damage: 2, ..base
        },
        RangedKind::Crescent => WeaponDef { name: "Crescent", color: 0x8fe7ff, speed: 16.0, range: 6.0, hit_radius: 0.9, pierce: true, ..base },
        RangedKind::FlameArrow => WeaponDef {
            name: "Flame Arrows", color: 0xff8a2a, lateral: &[-0.3, 0.0, 0.3], cooldown: 0.3, burn: true, ..base
        },
        RangedKind::HolyArrow => WeaponDef {
            name: "Holy Arrows", color: 0xfff1a8, speed: 18.0, range: 18.0, hit_radius: 0.6, pierce: true, cooldown: 0.3, damage: 3, ..base
        },
        RangedKind::Knife => WeaponDef { name: "Throwing Knife", color: 0xc8d0da, speed: 16.0, range: 9.0, cooldown: 0.18, ..base },
        RangedKind::TwinKnives => WeaponDef {
            name: "Twin Knives", color: 0xdfe6ee, speed: 16.0, range: 9.0, lateral: &[-0.22, 0.22], cooldown: 0.2, ..base
        },
        RangedKind::SeekingKnives => WeaponDef {
            name: "Seeking Knives", color: 0x9ad1ff, speed: 12.0, range: 12.0, lateral: &[-0.3, 0.0, 0.3], cooldown: 0.24, homing: true, ..base
        },
        RangedKind::SteelBoomerang => WeaponDef {
            name: "Steel Boomerang", color: 0xb8c8d8, speed: 11.0, range: 7.0, hit_radius: 0.85,
            pierce: true, returns: true, cooldown: 0.4, damage: 3, ..base
        },
        RangedKind::FireBoomerang => WeaponDef {
            name: "Fire Boomerang", color: 0xff7a2a, speed: 12.0, range: 7.5, hit_radius: 0.9,
            pierce: true, returns: true, cooldown: 0.38, damage: 3, burn: true, ..base
        },
    }
}

/// Display name and color for any held weapon, sword included (at the axe's level).
pub fn weapon_label(kind: WeaponKind, axe_level: u8) -> (&'static str, u32) {
    match kind {
        WeaponKind::Sword => (gear::Line::Axe.name(axe_level), gear::Line::Axe.color(axe_level)),
        WeaponKind::Ranged(r) => {
            let d = weapon(r);
            (d.name, d.color)
        }
    }
}

// -------------------------------------------------------------------- boons

/// Boons offered by the item shrine, in cycle order.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ItemKind {
    Shield,
    Invincible,
    Speed,
}

#[derive(Clone, Copy, Debug)]
pub struct ItemDef {
    pub name: &'static str,
    pub color: u32,
    /// Duration in seconds for timed boons.
    pub seconds: f64,
    /// Hits absorbed for the shield.
    pub charges: u32,
}

pub const ITEM_ORDER: [ItemKind; 3] = [ItemKind::Shield, ItemKind::Invincible, ItemKind::Speed];

pub fn item(kind: ItemKind) -> ItemDef {
    match kind {
        ItemKind::Shield => ItemDef { name: "Shield", color: 0x7fd7ff, seconds: 0.0, charges: 3 },
        ItemKind::Invincible => ItemDef { name: "Invincibility", color: 0xffd84a, seconds: 8.0, charges: 0 },
        ItemKind::Speed => ItemDef { name: "Speed", color: 0x7dff9a, seconds: 15.0, charges: 0 },
    }
}

pub const SPEED_BOOST: f64 = 1.4;

// ------------------------------------------------------------------ potions

/// A bottle lying on a tile. Taken once, never respawns within the level.
#[derive(Clone, Debug)]
pub struct Potion {
    pub tile: P,
    pub kind: PotionKind,
    pub taken: bool,
}

impl Potion {
    pub fn new(tile: P, kind: PotionKind) -> Self {
        Self { tile, kind, taken: false }
    }
    pub fn def(&self) -> gear::PotionDef {
        gear::potion(self.kind)
    }
}

// -------------------------------------------------------------------- bombs

pub const BOMB_FUSE: f64 = 3.0;
pub const BOMBS_PER_LIFE: u32 = 3;

/**
 * A dropped bomb. Sits on a tile, flashes faster as the fuse burns down,
 * then explodes. Other explosions can ignite it early, as with TNT.
 */
#[derive(Clone, Debug)]
pub struct Bomb {
    pub tile: P,
    pub fuse: f64,
}

impl Bomb {
    pub fn new(tile: P) -> Self {
        Self { tile, fuse: BOMB_FUSE }
    }

    /// 0..1 how far the fuse has burned.
    pub fn progress(&self) -> f64 {
        1.0 - self.fuse.max(0.0) / BOMB_FUSE
    }

    /// Flashing state: on/off, accelerating toward detonation.
    pub fn is_flash_on(&self, time: f64) -> bool {
        let hz = 2.0 + self.progress() * 10.0;
        ((time * hz).floor() as i64).rem_euclid(2) == 0
    }

    /// Returns true on the tick the bomb should explode.
    pub fn update(&mut self, dt: f64) -> bool {
        self.fuse -= dt;
        self.fuse <= 0.0
    }

    /// Shorten the fuse when caught in another blast.
    pub fn ignite(&mut self, delay: f64) {
        self.fuse = self.fuse.min(delay);
    }
}

// ------------------------------------------------------------------ shrines

pub const DIAMOND_CYCLE_SECONDS: f64 = 1.6;
pub const DIAMOND_RESPAWN_SECONDS: f64 = 25.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiamondKind {
    Weapon,
    Item,
}

/// What a shrine is showing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Offer {
    Weapon(RangedKind),
    Item(ItemKind),
}

impl Offer {
    pub fn name(self) -> &'static str {
        match self {
            Offer::Weapon(w) => weapon(w).name,
            Offer::Item(i) => item(i).name,
        }
    }
    pub fn color(self) -> u32 {
        match self {
            Offer::Weapon(w) => weapon(w).color,
            Offer::Item(i) => item(i).color,
        }
    }
}

/**
 * A shrine that cycles through a list of options. It advances on its own
 * every few seconds and immediately when struck; stepping on it takes
 * whatever is showing. Taken shrines come back after a while.
 */
#[derive(Clone, Debug)]
pub struct Diamond {
    pub tile: P,
    pub kind: DiamondKind,
    pub index: usize,
    pub active: bool,
    pub cycle_timer: f64,
    pub respawn_timer: f64,
}

impl Diamond {
    pub fn new(kind: DiamondKind, tile: P, start_index: usize) -> Self {
        let len = Self::len_of(kind);
        Self {
            tile,
            kind,
            index: start_index % len,
            active: true,
            cycle_timer: DIAMOND_CYCLE_SECONDS,
            respawn_timer: 0.0,
        }
    }

    fn len_of(kind: DiamondKind) -> usize {
        match kind {
            DiamondKind::Weapon => WEAPON_ORDER.len(),
            DiamondKind::Item => ITEM_ORDER.len(),
        }
    }

    pub fn current(&self) -> Offer {
        match self.kind {
            DiamondKind::Weapon => Offer::Weapon(WEAPON_ORDER[self.index]),
            DiamondKind::Item => Offer::Item(ITEM_ORDER[self.index]),
        }
    }

    /// 0..1 progress toward the next automatic cycle, for the spin animation.
    pub fn cycle_progress(&self) -> f64 {
        1.0 - self.cycle_timer.max(0.0) / DIAMOND_CYCLE_SECONDS
    }

    /// Advance to the next option (a strike).
    pub fn cycle(&mut self) {
        if !self.active {
            return;
        }
        self.index = (self.index + 1) % Self::len_of(self.kind);
        self.cycle_timer = DIAMOND_CYCLE_SECONDS;
    }

    /// Take the showing option; the shrine goes dormant until it respawns.
    pub fn take(&mut self) -> Offer {
        let chosen = self.current();
        self.active = false;
        self.respawn_timer = DIAMOND_RESPAWN_SECONDS;
        chosen
    }

    pub fn update(&mut self, dt: f64) {
        if !self.active {
            self.respawn_timer -= dt;
            if self.respawn_timer <= 0.0 {
                self.active = true;
                self.cycle_timer = DIAMOND_CYCLE_SECONDS;
            }
            return;
        }
        self.cycle_timer -= dt;
        if self.cycle_timer <= 0.0 {
            self.index = (self.index + 1) % Self::len_of(self.kind);
            self.cycle_timer += DIAMOND_CYCLE_SECONDS;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shrine_cycles_and_respawns() {
        let mut d = Diamond::new(DiamondKind::Item, P::new(0, 0), 0);
        assert_eq!(d.current(), Offer::Item(ItemKind::Shield));
        d.cycle();
        assert_eq!(d.current(), Offer::Item(ItemKind::Invincible));
        assert_eq!(d.take(), Offer::Item(ItemKind::Invincible));
        assert!(!d.active);
        d.update(DIAMOND_RESPAWN_SECONDS + 0.1);
        assert!(d.active);
    }

    #[test]
    fn bomb_fuse_and_chain() {
        let mut b = Bomb::new(P::new(1, 1));
        assert!(!b.update(1.0));
        b.ignite(0.2);
        assert!(b.update(0.25));
    }
}
