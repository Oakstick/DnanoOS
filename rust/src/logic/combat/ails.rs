//! What the later stages' legends do to the hero (COMBAT.md §11): the frost giant's chill and
//! the freezing after it, the arçura's roots, the owl's doom, Níðhöggr's venom, Surtr's fire
//! underfoot, a pin or a grab, a call drawing him in, a shove; and the brief marks the legends
//! leave on the scene for the view (lightning, roots, ice, an eruption, smoke). Every one is a
//! clock in 60 Hz frames; the brawl applies them (`game::brawl`).

use super::math::{len, sub};
use crate::logic::types::V;

/// Chill: frames it lasts and the share of his pace it leaves him; chilled again meanwhile, he
/// freezes solid for `FREEZE_FRAMES`.
pub const CHILL_FRAMES: u16 = 180;
pub const CHILL_PACE: f64 = 0.55;
pub const FREEZE_FRAMES: u16 = 50;
/// The owl's omen: frames it hangs over him, and what it makes of the next wound he takes.
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
/// The call: tiles per second it draws him, and the share of that left while he walks away.
pub const PULL_SPEED: f64 = 3.0;
pub const PULL_RESISTED: f64 = 0.35;
/// A shove: tiles per second for each tile it carries him, and the share of its speed each frame keeps.
pub const SHOVE_SPEED: f64 = 9.0;
pub const SHOVE_KEEP: f64 = 0.85;
/// A hold: frames between bites; a pin's bite takes this share of the pounce's damage, a grab's
/// damage comes in this many bites; each strike of his struggles this many frames off a pin.
pub const BITE_EVERY: u16 = 12;
pub const PIN_BITE: f64 = 0.3;
pub const GRAB_BITES: u16 = 3;
pub const STRUGGLE: u16 = 8;

/// Held fast: by whom, frames left, frames to the next bite and between bites, what a bite takes,
/// and whether it is a pin (struggled out of) or a grab.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Held {
    pub by: u32,
    pub left: u16,
    pub bite: u16,
    pub every: u16,
    pub damage: f64,
    pub pin: bool,
}

/// Why a wound came.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Wound {
    Venom,
    Fire,
    Bite,
}

/// Everything ailing him.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ails {
    pub chill: u16,
    pub frozen: u16,
    pub rooted: u16,
    pub doom: u16,
    pub venom: u16,
    pub scorch: u16,
    pub held: Option<Held>,
    /// Drawn toward a caller: which duelist, and frames left.
    pub pull: Option<(u32, u16)>,
    /// Shoved: tiles per second, fading.
    pub shove: V,
}

impl Ails {
    /// Chilled: the first time he slows; chilled again while he still is, he freezes. True if he froze.
    pub fn chill(&mut self) -> bool {
        if self.chill > 0 {
            self.chill = 0;
            self.frozen = FREEZE_FRAMES;
            true
        } else {
            self.chill = CHILL_FRAMES;
            false
        }
    }

    /// The share of his pace his ails leave him: none rooted, frozen or held; a little chilled.
    pub fn pace(&self) -> f64 {
        if self.rooted > 0 || self.frozen > 0 || self.held.is_some() {
            0.0
        } else if self.chill > 0 {
            CHILL_PACE
        } else {
            1.0
        }
    }

    /// He may leap (not rooted, frozen or held).
    pub fn can_leap(&self) -> bool {
        self.rooted == 0 && self.frozen == 0 && self.held.is_none()
    }

    /// The doom on him, spent by a wound: what that wound is multiplied by.
    pub fn spend_doom(&mut self) -> f64 {
        if self.doom > 0 {
            self.doom = 0;
            DOOM_MULT
        } else {
            1.0
        }
    }

    /// He strikes while pinned: he struggles that much nearer to free.
    pub fn struggle(&mut self) {
        if let Some(h) = self.held.as_mut().filter(|h| h.pin) {
            h.left = h.left.saturating_sub(STRUGGLE);
        }
    }

    /**
     * One frame: the clocks run down, and the wounds that come due are returned: venom every
     * `VENOM_EVERY` frames, fire every `SCORCH_EVERY` while he stands in it (`burning`), a hold's
     * bites. A hold run out lets him go.
     */
    pub fn step(&mut self, burning: bool) -> Vec<(f64, Wound)> {
        let mut out = Vec::new();
        self.chill = self.chill.saturating_sub(1);
        self.frozen = self.frozen.saturating_sub(1);
        self.rooted = self.rooted.saturating_sub(1);
        self.doom = self.doom.saturating_sub(1);
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
        if let Some(h) = self.held.as_mut() {
            h.bite = h.bite.saturating_sub(1);
            if h.bite == 0 {
                out.push((h.damage, Wound::Bite));
                h.bite = h.every;
            }
            h.left = h.left.saturating_sub(1);
            if h.left == 0 {
                self.held = None;
            }
        }
        if let Some((id, left)) = self.pull {
            self.pull = (left > 1).then_some((id, left - 1));
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
    fn chilled_twice_he_freezes_and_venom_fire_and_bites_come_due() {
        let mut a = Ails::default();
        assert!(!a.chill());
        assert_eq!(a.pace(), CHILL_PACE);
        assert!(a.chill(), "chilled again while chilled");
        assert!(a.frozen > 0 && a.pace() == 0.0 && !a.can_leap());
        let mut a = Ails { venom: 2 * VENOM_EVERY, ..Ails::default() };
        let wounds: Vec<_> = (0..2 * VENOM_EVERY).flat_map(|_| a.step(false)).collect();
        assert_eq!(wounds, vec![(VENOM_DAMAGE, Wound::Venom); 2]);
        let mut a = Ails::default();
        let fire: Vec<_> = (0..SCORCH_EVERY * 2).flat_map(|_| a.step(true)).collect();
        assert_eq!(fire.len(), 2, "on stepping in, and once more");
        // A pin's bites, struggled short; then he is free.
        let mut a =
            Ails { held: Some(Held { by: 1, left: 60, bite: BITE_EVERY, every: BITE_EVERY, damage: 5.0, pin: true }), ..Ails::default() };
        a.struggle();
        a.struggle();
        let bites = (0..60).flat_map(|_| a.step(false)).count();
        assert_eq!(bites, 3, "44 frames held, a bite every 12");
        assert!(a.held.is_none() && a.can_leap());
        // The omen's doom is spent by one wound.
        let mut a = Ails { doom: DOOM_FRAMES, ..Ails::default() };
        assert_eq!((a.spend_doom(), a.spend_doom()), (DOOM_MULT, 1.0));
    }
}
