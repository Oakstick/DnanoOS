//! What the later stages' legends do to the hero (COMBAT.md §11): the frost giant's frostbite,
//! the owl's and the karakoncolos's doom, Níðhöggr's venom, Surtr's fire underfoot, a shove; and
//! the brief marks the legends leave on the scene for the view (lightning, roots, ice, an
//! eruption, smoke). None of them slows him, holds him or roots him. Every one is a clock in
//! 60 Hz frames; the brawl applies them (`game::brawl`).

use super::math::{len, sub};
use crate::logic::types::V;

/// Frostbite: frames it lasts, and the health it takes, and how often.
pub const FROST_FRAMES: u16 = 180;
pub const FROST_EVERY: u16 = 30;
pub const FROST_DAMAGE: f64 = 2.0;
/// The omen (the owl's cry, the karakoncolos's roar): frames it hangs over him, and what it makes
/// of the next wound he takes.
pub const DOOM_FRAMES: u16 = 240;
pub const DOOM_MULT: f64 = 1.3;
/// Venom: health it takes, and how often.
pub const VENOM_EVERY: u16 = 30;
pub const VENOM_DAMAGE: f64 = 3.0;
/// Surtr's fire: frames a patch burns, how wide, and how often and how hard it scorches him.
pub const FIRE_FRAMES: u16 = 180;
pub const FIRE_RADIUS: f64 = 0.7;
pub const SCORCH_EVERY: u16 = 20;
pub const SCORCH_DAMAGE: f64 = 5.0;
/// A shove: tiles per second for each tile it carries him, and the share of its speed each frame keeps.
pub const SHOVE_SPEED: f64 = 9.0;
pub const SHOVE_KEEP: f64 = 0.85;

/// Why a wound came.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Wound {
    Frost,
    Venom,
    Fire,
}

/// Everything ailing him.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ails {
    pub frost: u16,
    pub doom: u16,
    pub venom: u16,
    pub scorch: u16,
    /// Shoved: tiles per second, fading.
    pub shove: V,
}

impl Ails {
    /// The doom on him, spent by a wound: what that wound is multiplied by.
    pub fn spend_doom(&mut self) -> f64 {
        if self.doom > 0 {
            self.doom = 0;
            DOOM_MULT
        } else {
            1.0
        }
    }

    /**
     * One frame: the clocks run down, and the wounds that come due are returned: frostbite every
     * `FROST_EVERY` frames and venom every `VENOM_EVERY`, fire every `SCORCH_EVERY` while he stands
     * in it (`burning`).
     */
    pub fn step(&mut self, burning: bool) -> Vec<(f64, Wound)> {
        let mut out = Vec::new();
        self.doom = self.doom.saturating_sub(1);
        if self.frost > 0 {
            self.frost -= 1;
            if self.frost.is_multiple_of(FROST_EVERY) {
                out.push((FROST_DAMAGE, Wound::Frost));
            }
        }
        if self.venom > 0 {
            self.venom -= 1;
            if self.venom.is_multiple_of(VENOM_EVERY) {
                out.push((VENOM_DAMAGE, Wound::Venom));
            }
        }
        if burning {
            if self.scorch == 0 {
                out.push((SCORCH_DAMAGE, Wound::Fire));
                self.scorch = SCORCH_EVERY;
            }
            self.scorch -= 1;
        } else {
            self.scorch = 0;
        }
        self.shove = V::new(self.shove.x * SHOVE_KEEP, self.shove.y * SHOVE_KEEP);
        if len(self.shove) < 0.05 {
            self.shove = V::new(0.0, 0.0);
        }
        out
    }
}

/// Ground that burns (Surtr's sword): where, how wide, and frames left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hazard {
    pub pos: V,
    pub radius: f64,
    pub life: u16,
}

impl Hazard {
    /// Does it burn a body of `radius` at `p`?
    pub fn burns(&self, p: V, radius: f64) -> bool {
        self.life > 0 && len(sub(p, self.pos)) <= self.radius + radius
    }
}

/// A legend's mark on the scene for a moment.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FlashKind {
    /// The goryō's lightning striking down.
    Lightning,
    /// The arçura's roots bursting up and sinking back.
    Roots,
    /// The frost giant's spikes of ice along its lane (`dir`, `length`).
    Ice,
    /// Níðhöggr bursting up out of the floor.
    Erupt,
    /// The cin's smoke, an illusion gone.
    Smoke,
    /// The kappa's dish spilling.
    Splash,
    /// The ground shaking round the oni's club.
    Quake,
    /// The dead climbing out of the floor.
    Grave,
}

/// One such mark: what, where (and which way, how long, for a lane), its age and how long it lasts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flash {
    pub kind: FlashKind,
    pub pos: V,
    pub dir: V,
    pub length: f64,
    pub age: u16,
    pub life: u16,
}

impl Flash {
    pub fn new(kind: FlashKind, pos: V) -> Self {
        let life = match kind {
            FlashKind::Lightning => 18,
            FlashKind::Roots => 70,
            FlashKind::Ice => 80,
            FlashKind::Erupt => 30,
            FlashKind::Smoke => 30,
            FlashKind::Splash => 30,
            FlashKind::Quake => 24,
            FlashKind::Grave => 50,
        };
        Self { kind, pos, dir: V::new(1.0, 0.0), length: 0.0, age: 0, life }
    }

    /// 0 to 1 through its life.
    pub fn progress(&self) -> f32 {
        self.age as f32 / self.life.max(1) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frostbite_venom_and_fire_come_due() {
        // Frostbite bites every half second while it lasts, and is gone after.
        let mut a = Ails { frost: FROST_FRAMES, ..Ails::default() };
        let frost: Vec<_> = (0..FROST_FRAMES + 60).flat_map(|_| a.step(false)).collect();
        assert_eq!(frost, vec![(FROST_DAMAGE, Wound::Frost); (FROST_FRAMES / FROST_EVERY) as usize]);
        let mut a = Ails { venom: 2 * VENOM_EVERY, ..Ails::default() };
        let wounds: Vec<_> = (0..2 * VENOM_EVERY).flat_map(|_| a.step(false)).collect();
        assert_eq!(wounds, vec![(VENOM_DAMAGE, Wound::Venom); 2]);
        let mut a = Ails::default();
        let fire: Vec<_> = (0..SCORCH_EVERY * 2).flat_map(|_| a.step(true)).collect();
        assert_eq!(fire.len(), 2, "on stepping in, and once more");
        // The omen's doom is spent by one wound.
        let mut a = Ails { doom: DOOM_FRAMES, ..Ails::default() };
        assert_eq!((a.spend_doom(), a.spend_doom()), (DOOM_MULT, 1.0));
    }
}
