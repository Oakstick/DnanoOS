//! The mix (WARBAND.md §4.6): who is heard first when everything sounds at once. Priorities (the
//! token holder's tell, then the hero's own blows, then the rest), at most three of the same sound
//! inside 40 ms (merged, louder), a budget of 96 effect voices that steals from the least
//! important instead of dropping the newest, a duck of everything else under a parry or a struck
//! hero, a soft limiter on the master, and stereo: a sound pans by its bearing from the camera and
//! falls away over 18 tiles, and one from behind him is a little louder, because the eye can't help
//! there.
//!
//! Everything here is presentation: the round robin draws from the audio manager's own rng, never
//! the simulation's (WARBAND.md I5).

use std::collections::HashMap;

use crate::logic::rng::Rng;
use crate::logic::types::V;

/// How much a sound matters when the voices run short: the higher steals from the lower.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
pub enum Priority {
    /// Steps, drips, the world.
    Ambient = 0,
    /// Everything else: cries, swings, the old effects.
    #[default]
    Rest = 1,
    /// The hero's own blows, and what happens to him (parry, block, struck).
    Hero = 2,
    /// The tell of the foe holding the token: the one sound that must never be lost.
    Tell = 3,
}

/// Effect voices alive at once; past it, the least important voice is stolen.
pub const SFX_VOICES: usize = 96;
/// Seconds a stolen voice takes to fade out (no click).
pub const STEAL_FADE: f64 = 0.005;

/// The same sound at most this many times inside `DEDUPE_WINDOW`; past it, merged into the last.
pub const DEDUPE_MAX: usize = 3;
pub const DEDUPE_WINDOW: f64 = 0.040;
/// Each merged repeat lifts the one it joined by about 1 dB, up to `MERGE_CAP` (about 4 dB).
pub const MERGE_BOOST: f64 = 1.12;
pub const MERGE_CAP: f64 = 1.6;

/// Under a parry or a struck hero, everything else drops 4 dB for 120 ms.
pub const DUCK_GAIN: f64 = 0.631;
pub const DUCK_SECS: f64 = 0.120;

/// Tiles over which a sound falls away to nothing, and the radius inside which it is at full voice.
pub const FALLOFF: f64 = 18.0;
pub const NEAR: f64 = 1.5;
/// A sound from his back arc (more than 120 degrees off the camera's facing) is 3 dB louder.
pub const BACK_BOOST: f64 = 1.413;
pub const BACK_ARC_COS: f64 = -0.5;

/// What the push of a voice carries besides the voice: its place in the stereo field, its
/// importance, the play it belongs to, whether ducks pass it by, and a gain.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Stamp {
    pub pan_l: f64,
    pub pan_r: f64,
    pub prio: Priority,
    /// The play the voice belongs to (0: none); a merged repeat lifts its group.
    pub group: u32,
    /// A ducking sound's own voices are not ducked.
    pub immune: bool,
    pub gain: f64,
    /// A voice (a cry, a word typed out): the voices' volume, not the effects', sets its level.
    pub voice: bool,
}

impl Default for Stamp {
    fn default() -> Self {
        Self { pan_l: 1.0, pan_r: 1.0, prio: Priority::Rest, group: 0, immune: false, gain: 1.0, voice: false }
    }
}

/// Where the ears are: on the hero, facing the way the camera looks (grid units, unit length).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Listener {
    pub pos: V,
    pub facing: V,
}

impl Default for Listener {
    fn default() -> Self {
        Self { pos: V::new(0.0, 0.0), facing: V::new(0.0, -1.0) }
    }
}

/// Constant-power pan, unity at the centre: -1 hard left, +1 hard right. Each side's gain is
/// `sqrt(2) cos` of a quarter turn across, so left² + right² is 2 everywhere and a centred sound
/// plays exactly as the mono bus did.
pub fn pan_gains(pan: f64) -> (f64, f64) {
    let a = (pan.clamp(-1.0, 1.0) + 1.0) * std::f64::consts::FRAC_PI_4;
    (std::f64::consts::SQRT_2 * a.cos(), std::f64::consts::SQRT_2 * a.sin())
}

/// A sound at `at`, heard from `l`: its pan (-1..1) and gain. Close by, it sits nearly central (a
/// blow on his own body is not off to one side); away, it pans by its bearing off the camera's
/// facing and falls away over `FALLOFF` tiles; from his back arc, +3 dB.
pub fn place(l: &Listener, at: V) -> (f64, f64) {
    let (dx, dy) = (at.x - l.pos.x, at.y - l.pos.y);
    let d = (dx * dx + dy * dy).sqrt();
    if d < 1e-6 {
        return (0.0, 1.0);
    }
    let flen = (l.facing.x * l.facing.x + l.facing.y * l.facing.y).sqrt().max(1e-9);
    let (fx, fy) = (l.facing.x / flen, l.facing.y / flen);
    // Grid y runs down the screen: the camera's right is the facing turned a quarter clockwise.
    let (rx, ry) = (-fy, fx);
    let (ux, uy) = (dx / d, dy / d);
    let pan = (ux * rx + uy * ry) * (d / 2.0).min(1.0);
    let far = ((d - NEAR).max(0.0) / (FALLOFF - NEAR)).min(1.0);
    let mut gain = (1.0 - far) * (1.0 - far);
    if ux * fx + uy * fy < BACK_ARC_COS && d > 0.5 {
        gain *= BACK_BOOST;
    }
    (pan, gain)
}

/// Whether a play is heard as itself or merged into the same sound just before it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Admit {
    Play,
    /// Too many of it inside the window: lift the group already sounding instead.
    Merge {
        group: u32,
    },
}

/// At most `DEDUPE_MAX` of the same sound inside `DEDUPE_WINDOW`.
#[derive(Default)]
pub struct Dedupe {
    recent: Vec<(u64, f64, u32)>,
}

impl Dedupe {
    pub fn admit(&mut self, key: u64, now: f64, group: u32) -> Admit {
        self.recent.retain(|r| now - r.1 < DEDUPE_WINDOW && r.1 <= now);
        let same: Vec<&(u64, f64, u32)> = self.recent.iter().filter(|r| r.0 == key).collect();
        if same.len() >= DEDUPE_MAX {
            return Admit::Merge { group: same.last().unwrap().2 };
        }
        self.recent.push((key, now, group));
        Admit::Play
    }
}

/// What a newcomer does when the effect voices are full.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Steal {
    /// There is room.
    Room,
    /// Fade out the voice at this index and take its place.
    Take(usize),
    /// Everything sounding matters more: the newcomer is dropped.
    Refuse,
}

/// The budget: `live` lists the effect voices sounding (index, priority, when each ends). Under
/// `SFX_VOICES` there is room; else the least important goes, and of those the one nearest its
/// end, provided it matters no more than the newcomer.
pub fn steal(live: impl Iterator<Item = (usize, Priority, f64)>, new: Priority) -> Steal {
    let mut count = 0;
    let mut victim: Option<(usize, Priority, f64)> = None;
    for v in live {
        count += 1;
        let better = match victim {
            None => true,
            Some(w) => v.1 < w.1 || (v.1 == w.1 && v.2 < w.2),
        };
        if better {
            victim = Some(v);
        }
    }
    if count < SFX_VOICES {
        return Steal::Room;
    }
    match victim {
        Some((i, p, _)) if p <= new => Steal::Take(i),
        _ => Steal::Refuse,
    }
}

/// A soft limiter on the master, stereo-linked: a peak follower (attack 0.5 ms, release 150 ms)
/// holds the level under `THRESHOLD`, and a gentle knee above `KNEE` keeps any overshoot the
/// follower is too slow for under full scale. Below the threshold it is exactly transparent.
pub struct Limiter {
    env: f64,
    attack: f64,
    release: f64,
}

impl Limiter {
    pub const THRESHOLD: f64 = 0.89;
    pub const KNEE: f64 = 0.94;

    pub fn new(sample_rate: f64) -> Self {
        let k = |secs: f64| 1.0 - (-1.0 / (secs * sample_rate)).exp();
        Self { env: 0.0, attack: k(0.0005), release: k(0.150) }
    }

    pub fn process(&mut self, l: f64, r: f64) -> (f64, f64) {
        let peak = l.abs().max(r.abs());
        let k = if peak > self.env { self.attack } else { self.release };
        self.env += (peak - self.env) * k;
        let g = if self.env > Self::THRESHOLD { Self::THRESHOLD / self.env } else { 1.0 };
        (knee(l * g), knee(r * g))
    }
}

/// Unchanged up to `KNEE`, then bending smoothly toward (never past) full scale.
fn knee(x: f64) -> f64 {
    let a = x.abs();
    if a <= Limiter::KNEE {
        return x;
    }
    let room = 1.0 - Limiter::KNEE;
    x.signum() * (Limiter::KNEE + room * ((a - Limiter::KNEE) / room).tanh())
}

/// Three variants of every recipe, never the same twice in a row.
#[derive(Default)]
pub struct RoundRobin {
    last: HashMap<u64, u8>,
}

pub const VARIANTS: u8 = 3;

impl RoundRobin {
    pub fn next(&mut self, key: u64, rng: &mut Rng) -> u8 {
        let v = match self.last.get(&key) {
            // One of the two it wasn't last time.
            Some(&last) => (last + 1 + (rng.next() * 2.0).min(1.0) as u8) % VARIANTS,
            None => (rng.next() * VARIANTS as f64).min(2.0) as u8,
        };
        self.last.insert(key, v);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pan_law_keeps_power_and_centres_at_unity() {
        let (l, r) = pan_gains(0.0);
        assert!((l - 1.0).abs() < 1e-9 && (r - 1.0).abs() < 1e-9, "the centre plays as the mono bus did");
        for k in -10..=10 {
            let (l, r) = pan_gains(k as f64 / 10.0);
            assert!((l * l + r * r - 2.0).abs() < 1e-9, "power changes across the field at {k}");
        }
        let (l, r) = pan_gains(-1.0);
        assert!(r.abs() < 1e-9 && l > 1.4, "hard left is all left");
        let (l, r) = pan_gains(1.0);
        assert!(l.abs() < 1e-9 && r > 1.4, "hard right is all right");
    }

    #[test]
    fn sounds_pan_by_their_bearing_from_the_camera() {
        // Facing up the screen (grid y down): east is to his right, west to his left.
        let l = Listener { pos: V::new(10.0, 10.0), facing: V::new(0.0, -1.0) };
        let (east, _) = place(&l, V::new(16.0, 10.0));
        let (west, _) = place(&l, V::new(4.0, 10.0));
        let (ahead, _) = place(&l, V::new(10.0, 4.0));
        assert!(east > 0.95 && west < -0.95 && ahead.abs() < 1e-9, "{east} {west} {ahead}");
        // Turned to face east, the same sound east is dead ahead and the one north to his left.
        let l = Listener { pos: V::new(10.0, 10.0), facing: V::new(1.0, 0.0) };
        let (east, _) = place(&l, V::new(16.0, 10.0));
        let (north, _) = place(&l, V::new(10.0, 4.0));
        assert!(east.abs() < 1e-9 && north < -0.95, "{east} {north}");
        // On top of him it sits in the middle.
        let (close, g) = place(&l, V::new(10.3, 10.0));
        assert!(close.abs() < 0.2 && (g - 1.0).abs() < 1e-9, "{close} {g}");
    }

    #[test]
    fn sounds_fall_away_over_eighteen_tiles_and_are_louder_from_behind() {
        let l = Listener { pos: V::new(0.0, 0.0), facing: V::new(0.0, -1.0) };
        let gain = |d: f64| place(&l, V::new(d, 0.0)).1;
        assert!((gain(1.0) - 1.0).abs() < 1e-9);
        assert!(gain(4.0) > gain(8.0) && gain(8.0) > gain(14.0) && gain(14.0) > 0.0);
        assert_eq!(gain(FALLOFF), 0.0);
        assert_eq!(gain(30.0), 0.0);
        let (_, ahead) = place(&l, V::new(0.0, -6.0));
        let (_, behind) = place(&l, V::new(0.0, 6.0));
        assert!((behind / ahead - BACK_BOOST).abs() < 1e-9, "the back arc is +3 dB: {ahead} {behind}");
    }

    #[test]
    fn dedupe_lets_three_through_then_merges() {
        let mut d = Dedupe::default();
        assert_eq!(d.admit(7, 1.000, 1), Admit::Play);
        assert_eq!(d.admit(7, 1.010, 2), Admit::Play);
        assert_eq!(d.admit(9, 1.012, 3), Admit::Play, "another sound is its own");
        assert_eq!(d.admit(7, 1.020, 4), Admit::Play);
        assert_eq!(d.admit(7, 1.030, 5), Admit::Merge { group: 4 }, "the fourth inside 40 ms merges into the last");
        assert_eq!(d.admit(7, 1.035, 6), Admit::Merge { group: 4 });
        // The window moves on: 40 ms after the first, there is room again.
        assert_eq!(d.admit(7, 1.045, 7), Admit::Play);
    }

    #[test]
    fn the_budget_steals_the_least_important_first() {
        let full = |prio: fn(usize) -> Priority| (0..SFX_VOICES).map(move |i| (i, prio(i), 1.0 + i as f64));
        assert_eq!(steal((0..SFX_VOICES - 1).map(|i| (i, Priority::Tell, 1.0)), Priority::Ambient), Steal::Room);
        // All equal: the one nearest its end goes.
        assert_eq!(steal(full(|_| Priority::Rest), Priority::Rest), Steal::Take(0));
        // A tell steals from the rest, never from another tell.
        let mixed = |i: usize| if i.is_multiple_of(2) { Priority::Tell } else { Priority::Rest };
        assert_eq!(steal(full(mixed), Priority::Tell), Steal::Take(1));
        assert_eq!(steal(full(|_| Priority::Tell), Priority::Hero), Steal::Refuse);
        assert_eq!(steal(full(|i| if i == 50 { Priority::Ambient } else { Priority::Hero }), Priority::Rest), Steal::Take(50));
    }

    #[test]
    fn the_limiter_is_transparent_below_and_holds_loud_signals_under_full_scale() {
        let sr = 48_000.0;
        let mut lim = Limiter::new(sr);
        let mut worst = 0.0f64;
        for i in 0..48_000 {
            let x = 0.6 * (i as f64 * 440.0 / sr * std::f64::consts::TAU).sin();
            let (l, r) = lim.process(x, -x);
            worst = worst.max((l - x).abs()).max((r + x).abs());
        }
        assert!(worst < 1e-12, "a quiet signal was changed by {worst}");
        let mut peak = 0.0f64;
        let mut flat = 0;
        for i in 0..48_000 {
            let x = 4.0 * (i as f64 * 220.0 / sr * std::f64::consts::TAU).sin();
            let (l, _) = lim.process(x, x);
            peak = peak.max(l.abs());
            // (In its first milliseconds the follower is still catching up; the knee holds it.)
            if l.abs() >= 0.9999 && i > 480 {
                flat += 1;
            }
        }
        assert!(peak < 1.0, "the limiter let through {peak}");
        assert!(peak > 0.8, "the limiter crushed it to {peak}");
        assert_eq!(flat, 0, "a hard clamp's flat tops");
    }

    #[test]
    fn round_robin_never_plays_the_same_variant_twice_running() {
        let mut rr = RoundRobin::default();
        let mut rng = Rng::new(3);
        let mut seen = [0; 3];
        let mut last = rr.next(1, &mut rng);
        for _ in 0..300 {
            let v = rr.next(1, &mut rng);
            assert_ne!(v, last);
            assert!(v < VARIANTS);
            seen[v as usize] += 1;
            last = v;
        }
        assert!(seen.iter().all(|&n| n > 60), "every variant is heard: {seen:?}");
    }
}
