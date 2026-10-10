//! The controller's rumble (UI.md §2.6, §3): a pulse for each moment the fight wants felt in the
//! hands, scaled by the player's setting. The game queues the moments (presentation only: nothing
//! in the simulation reads them) and the app plays them through gilrs's force feedback, where the
//! pad and the platform have it; elsewhere, and on the web, nothing happens, quietly.

use crate::platform;

/// A moment felt in the hands.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Rumble {
    /// His blow lands: light and short.
    HitLanded,
    /// A blow lands on him: stronger.
    HitTaken,
    /// His guard is broken: long and low.
    GuardBroken,
    /// A parry: short and sharp.
    Parry,
    /// The ground shaken (a boss's slam, a quake), at this strength from 0 to 1 (by distance).
    Quake(f32),
}

/// One pulse of the two motors: the strong (low, heavy) and the weak (high, buzzing), each 0 to
/// 1, for so many milliseconds.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Pulse {
    pub strong: f32,
    pub weak: f32,
    pub ms: u32,
}

/// The rumble setting.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum RumbleLevel {
    Off,
    Low,
    #[default]
    High,
}

pub const RUMBLE_KEY: &str = "vh.rumble";

impl RumbleLevel {
    pub const ALL: [RumbleLevel; 3] = [RumbleLevel::Off, RumbleLevel::Low, RumbleLevel::High];

    /// How strong a pulse is felt at this setting.
    pub fn scale(self) -> f32 {
        match self {
            RumbleLevel::Off => 0.0,
            RumbleLevel::Low => 0.45,
            RumbleLevel::High => 1.0,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            RumbleLevel::Off => "off",
            RumbleLevel::Low => "low",
            RumbleLevel::High => "high",
        }
    }

    pub fn from_name(s: &str) -> Option<RumbleLevel> {
        Self::ALL.into_iter().find(|l| l.name() == s.trim())
    }

    /// The next setting round (off, low, high, off...), for a settings button.
    pub fn next(self) -> RumbleLevel {
        match self {
            RumbleLevel::Off => RumbleLevel::Low,
            RumbleLevel::Low => RumbleLevel::High,
            RumbleLevel::High => RumbleLevel::Off,
        }
    }

    /// As saved (tests keep to the default, high).
    pub fn load() -> RumbleLevel {
        if cfg!(test) {
            return RumbleLevel::default();
        }
        platform::load(RUMBLE_KEY).and_then(|s| Self::from_name(&s)).unwrap_or_default()
    }

    /// Saved (never from tests; captures save into their own folder).
    pub fn save(self) {
        if cfg!(test) {
            return;
        }
        platform::save(RUMBLE_KEY, self.name());
    }
}

impl Rumble {
    /// The pulse at full strength.
    pub fn full(self) -> Pulse {
        match self {
            Rumble::HitLanded => Pulse { strong: 0.0, weak: 0.45, ms: 70 },
            Rumble::HitTaken => Pulse { strong: 0.6, weak: 0.5, ms: 170 },
            Rumble::GuardBroken => Pulse { strong: 0.55, weak: 0.1, ms: 450 },
            Rumble::Parry => Pulse { strong: 0.15, weak: 0.85, ms: 60 },
            Rumble::Quake(s) => {
                let s = s.clamp(0.0, 1.0);
                Pulse { strong: 0.95 * s, weak: 0.35 * s, ms: 320 }
            }
        }
    }

    /// The pulse at `level`, or none (rumble off, or too faint to feel).
    pub fn pulse(self, level: RumbleLevel) -> Option<Pulse> {
        let k = level.scale();
        let p = self.full();
        let p = Pulse { strong: p.strong * k, weak: p.weak * k, ms: p.ms };
        (p.strong.max(p.weak) >= 0.02).then_some(p)
    }
}

/// Plays pulses on a controller through gilrs, holding each effect until it has run.
#[derive(Default)]
pub struct Rumbler {
    #[cfg(not(target_arch = "wasm32"))]
    playing: Vec<(gilrs::ff::Effect, web_time::Instant)>,
}

impl Rumbler {
    /// Plays `p` on pad `id`; where force feedback is missing, nothing happens (no error, no log).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn play(&mut self, gilrs: &mut gilrs::Gilrs, id: gilrs::GamepadId, p: Pulse) {
        use gilrs::ff::{BaseEffect, BaseEffectType, EffectBuilder, Repeat, Replay, Ticks};
        self.tidy();
        if !gilrs.connected_gamepad(id).is_some_and(|g| g.is_ff_supported()) {
            return;
        }
        let mag = |v: f32| (v.clamp(0.0, 1.0) * u16::MAX as f32) as u16;
        let play_for = Ticks::from_ms(p.ms);
        let scheduling = Replay { play_for, ..Default::default() };
        let mut b = EffectBuilder::new();
        if p.strong > 0.0 {
            b.add_effect(BaseEffect { kind: BaseEffectType::Strong { magnitude: mag(p.strong) }, scheduling, ..Default::default() });
        }
        if p.weak > 0.0 {
            b.add_effect(BaseEffect { kind: BaseEffectType::Weak { magnitude: mag(p.weak) }, scheduling, ..Default::default() });
        }
        let Ok(effect) = b.gamepads(&[id]).repeat(Repeat::For(play_for)).finish(gilrs) else { return };
        if effect.play().is_ok() {
            let until = web_time::Instant::now() + std::time::Duration::from_millis(p.ms as u64 + 50);
            self.playing.push((effect, until));
        }
    }

    /// On the web the browser's rumble is not reached: nothing happens.
    #[cfg(target_arch = "wasm32")]
    pub fn play(&mut self, _gilrs: &mut gilrs::Gilrs, _id: gilrs::GamepadId, _p: Pulse) {}

    /// Lets go of the effects that have run.
    pub fn tidy(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let now = web_time::Instant::now();
            self.playing.retain(|(_, until)| *until > now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rumble_scales_with_the_setting_and_off_is_nothing() {
        for r in [Rumble::HitLanded, Rumble::HitTaken, Rumble::GuardBroken, Rumble::Parry, Rumble::Quake(1.0)] {
            assert_eq!(r.pulse(RumbleLevel::Off), None, "{r:?}");
            let high = r.pulse(RumbleLevel::High).unwrap();
            let low = r.pulse(RumbleLevel::Low).unwrap();
            assert_eq!(high, r.full());
            assert!(low.strong <= high.strong && low.weak < high.weak && low.ms == high.ms, "{r:?}");
        }
        // The taken blow outweighs the landed one; the broken guard lasts longest; the parry is quick.
        let full = |r: Rumble| r.full();
        assert!(full(Rumble::HitTaken).strong > full(Rumble::HitLanded).strong);
        assert!(full(Rumble::GuardBroken).ms > full(Rumble::HitTaken).ms);
        assert!(full(Rumble::Parry).ms <= full(Rumble::HitLanded).ms);
        assert!(Rumble::Quake(0.0).pulse(RumbleLevel::High).is_none(), "a quake far away is not felt");
        assert!(Rumble::Quake(0.5).full().strong < Rumble::Quake(1.0).full().strong);
    }

    #[test]
    fn the_setting_round_trips_and_cycles() {
        for l in RumbleLevel::ALL {
            assert_eq!(RumbleLevel::from_name(l.name()), Some(l));
        }
        assert_eq!(RumbleLevel::from_name("loud"), None);
        assert_eq!(RumbleLevel::default(), RumbleLevel::High);
        assert_eq!(RumbleLevel::High.next(), RumbleLevel::Off);
        assert_eq!(RumbleLevel::Off.next().next(), RumbleLevel::High);
    }
}
