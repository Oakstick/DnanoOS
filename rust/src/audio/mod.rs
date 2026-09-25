//! Procedural audio. No sample files: every effect and the whole soundtrack
//! are synthesized, the same voices the web build made with Web Audio.

pub mod music;
pub mod synth;

use std::sync::{Arc, Mutex};

use music::BASE_BPM;
use synth::{Bus, Env, Synth, Voice, Wave};

use crate::platform;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sfx {
    Nodule,
    Charge,
    Eat,
    Death,
    Clear,
    Start,
    Slash,
    Hurt,
    Fuse,
    Explode,
    Arrow,
    ArrowHit,
    /// Every nugget collected: the congratulations fanfare.
    Win,
    Jump,
    Heal,
    /// Countdown beep.
    Tick,
    /// Countdown release.
    Go,
    /// Landing from a jump.
    Land,
    /// A nugget milestone (every five).
    Milestone,
    /// The Cyclops bellows: arriving, enraged, getting up, dying.
    BossRoar,
    /// Its eye winds up to attack.
    BossCharge,
    /// A bolt spat from the eye.
    BossBolt,
    /// A boulder hurled.
    BossHurl,
    /// It hits the ground.
    BossSlam,
    /// A blow lands on it.
    BossHit,
    /// Its death cry.
    BossDeath,
    /// The smith's hammer on the anvil.
    Anvil,
    /// Coins changing hands at the forge.
    Buy,
    /// Not enough coins, or not possible.
    Denied,
    /// A potion going down.
    Drink,
    /// A shield splintering.
    ShieldBreak,
    /// A lightning potion's arc.
    Zap,
    /// The tour's camera swooping to the next stop.
    Whoosh,
    /// A x3 kill streak.
    Brutal,
}

const MASTER_VOLUME: f64 = 0.5;
/// The backing track sits well under the effects; it is a bed, not a feature.
const MUSIC_VOLUME: f64 = 0.42;
/// Each tempo step (every five nuggets) speeds the music up by this factor.
pub const TEMPO_STEP: f64 = 1.05;

pub struct AudioManager {
    synth: Option<Arc<Mutex<Synth>>>,
    #[allow(dead_code)]
    stream: Option<cpal::Stream>,
    pickup_pitch: u32,
    pub muted: bool,
    ducked: bool,
    tempo_step: u32,
    rng: crate::logic::rng::Rng,
    foot: bool,
}

impl Default for AudioManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioManager {
    pub fn new() -> Self {
        let muted = platform::load("vh.muted").map(|v| v == "1").unwrap_or(false);
        Self {
            synth: None,
            stream: None,
            pickup_pitch: 0,
            muted,
            ducked: false,
            tempo_step: 0,
            rng: crate::logic::rng::Rng::from_time(),
            foot: false,
        }
    }

    /// Opens the audio device on first use. On the web this must follow a user gesture.
    pub fn unlock(&mut self) {
        if self.synth.is_some() || std::env::var_os("VH_NO_AUDIO").is_some() {
            return;
        }
        match open_stream(MUSIC_VOLUME) {
            Some((synth, stream)) => {
                {
                    let mut s = synth.lock().unwrap();
                    s.master = if self.muted { 0.0 } else { MASTER_VOLUME };
                    s.music.running = true;
                    s.music.bpm = BASE_BPM * TEMPO_STEP.powi(self.tempo_step as i32);
                    s.music_target = if self.ducked { MUSIC_VOLUME * 0.3 } else { MUSIC_VOLUME };
                }
                self.synth = Some(synth);
                self.stream = Some(stream);
            }
            None => {
                // No output device: the game runs silent rather than failing.
                self.synth = None;
            }
        }
    }

    fn with(&self, f: impl FnOnce(&mut Synth)) {
        if let Some(s) = &self.synth {
            if let Ok(mut s) = s.lock() {
                f(&mut s);
            }
        }
    }

    pub fn toggle_mute(&mut self) -> bool {
        self.muted = !self.muted;
        let muted = self.muted;
        self.with(|s| s.master = if muted { 0.0 } else { MASTER_VOLUME });
        platform::save("vh.muted", if self.muted { "1" } else { "0" });
        self.muted
    }

    /// 0..1 how close the nearest enemy is; raises the ambient pulse and thickens the drums.
    pub fn set_threat(&mut self, level: f64) {
        self.with(|s| {
            s.intensity = level;
            s.drone_gain_target = 0.016 + level * 0.17;
            s.drone_freq_target = 42.0 + level * 30.0;
        });
    }

    /// Drops the music under a pause or a death, and lifts it again after.
    pub fn duck_music(&mut self, under: bool) {
        self.ducked = under;
        self.with(|s| s.music_target = if under { MUSIC_VOLUME * 0.3 } else { MUSIC_VOLUME });
    }

    pub fn set_frenzy(&mut self, active: bool) {
        self.with(|s| s.frenzy = active);
    }

    /**
     * The backing track's pace: step 0 is the resting tempo and every step is a
     * little quicker. The game raises it by one for every five nuggets, so the
     * music presses harder the closer the hoard is to complete.
     */
    pub fn set_tempo_step(&mut self, step: u32) {
        self.tempo_step = step;
        let bpm = BASE_BPM * TEMPO_STEP.powi(step as i32);
        self.with(|s| s.music.bpm = bpm);
    }

    pub fn tempo_step(&self) -> u32 {
        self.tempo_step
    }

    /**
     * Nugget reward: a coin. Struck metal is inharmonic, so the clink is three
     * partials at bell ratios rather than a chord. Over it go the two rising
     * notes an arcade coin is made of, a fourth apart, with a thin shimmer on
     * the second. The whole thing creeps up in pitch as the hoard grows.
     */
    pub fn play_reward(&mut self, step: u32) {
        self.with(|s| {
            let t = s.now;
            let base = 988.0 * 1.018f64.powi(step.min(24) as i32);
            blip(s, t, base, 0.06, Wave::Sine, 0.10);
            blip(s, t, base * 2.76, 0.05, Wave::Sine, 0.07);
            blip(s, t, base * 5.4, 0.04, Wave::Sine, 0.035);
            blip(s, t, base, 0.09, Wave::Square, 0.15);
            blip(s, t + 0.07, base * 1.335, 0.24, Wave::Square, 0.14);
            blip(s, t + 0.07, base * 2.67, 0.26, Wave::Sine, 0.045);
            blip(s, t + 0.09, base * 4.0, 0.16, Wave::Sine, 0.025);
        });
    }

    /// 0..1 how badly hurt the hero is; muffles the music.
    pub fn set_danger(&mut self, level: f64) {
        self.with(|s| s.danger_target = level.clamp(0.0, 1.0));
    }

    /// A footstep, varied every time so a run never sounds like a loop. Slime squelches.
    pub fn play_step(&mut self, loudness: f64, slimy: bool) {
        self.foot = !self.foot;
        let (p, g) = (1.0 + self.rng.range(-0.08, 0.08) + if self.foot { 0.05 } else { -0.05 }, self.rng.range(0.8, 1.1));
        let offset = self.rng.next() * 2.5;
        self.with(|s| {
            let t = s.now;
            let vol = 0.05 * loudness * g;
            let cutoff = if slimy { 480.0 } else { 900.0 * p };
            s.push(
                Voice::new(t, t + 0.09, Wave::Noise { rate: 0.9 * p, offset }, Env::constant(1.0), Env::ramp(t, vol * 2.2, 0.001, 0.07), Bus::Sfx)
                    .filtered(synth::FilterKind::Lowpass, cutoff, 1.0),
            );
            if slimy {
                s.push(Voice::new(t, t + 0.14, Wave::Sine, Env::ramp(t, 320.0 * p, 110.0, 0.12), Env::ramp(t, vol * 1.4, 0.001, 0.12), Bus::Sfx));
            } else {
                s.push(Voice::new(t, t + 0.06, Wave::Sine, Env::ramp(t, 120.0 * p, 70.0, 0.05), Env::ramp(t, vol * 1.6, 0.001, 0.05), Bus::Sfx));
            }
        });
    }

    /// A kill; each one in a quick streak rings a semitone higher.
    pub fn play_kill(&mut self, streak: u32) {
        let up = 2f64.powf(streak.min(12) as f64 / 12.0);
        let (p, g) = (up * self.rng.range(0.97, 1.03), self.rng.range(0.9, 1.05));
        self.with(|s| {
            let t = s.now;
            s.var_pitch = p;
            s.var_gain = g;
            sweep(s, t, 900.0, 200.0, 0.25, Wave::Square, 0.2);
            if streak >= 2 {
                blip(s, t + 0.05, 1320.0, 0.12, Wave::Triangle, 0.08);
            }
            s.var_pitch = 1.0;
            s.var_gain = 1.0;
        });
    }

    pub fn play(&mut self, sfx: Sfx) {
        if sfx == Sfx::Nodule {
            self.pickup_pitch = (self.pickup_pitch + 1) % 2;
        }
        let pitch = self.pickup_pitch as f64;
        // Dynamic variation: no two plays of an effect come out exactly alike.
        // Cues that carry meaning (countdown, fanfare) stay exact.
        let exact = matches!(sfx, Sfx::Tick | Sfx::Go | Sfx::Win | Sfx::Milestone | Sfx::Brutal);
        let vp = if exact { 1.0 } else { self.rng.range(0.94, 1.06) };
        let vg = if exact { 1.0 } else { self.rng.range(0.85, 1.05) };
        let side = if sfx == Sfx::Slash {
            self.foot = !self.foot;
            if self.foot { 1.0 } else { 0.86 }
        } else {
            1.0
        };
        let offset = self.rng.next() * 2.0;
        self.with(|s| {
            let t = s.now;
            s.var_pitch = vp * side;
            s.var_gain = vg;
            match sfx {
                Sfx::Nodule => blip(s, t, 520.0 + pitch * 90.0, 0.05, Wave::Square, 0.12),
                Sfx::Charge => sweep(s, t, 200.0, 900.0, 0.35, Wave::Saw, 0.2),
                Sfx::Heal => {
                    // A warm rising third, the sound of catching your breath.
                    blip(s, t, 523.0, 0.14, Wave::Sine, 0.18);
                    blip(s, t + 0.09, 659.0, 0.16, Wave::Sine, 0.18);
                    blip(s, t + 0.19, 784.0, 0.26, Wave::Sine, 0.16);
                }
                Sfx::Eat => sweep(s, t, 900.0, 200.0, 0.25, Wave::Square, 0.2),
                Sfx::Death => sweep(s, t, 400.0, 40.0, 1.1, Wave::Saw, 0.3),
                Sfx::Slash => {
                    // Blade swish plus a breath of air behind it; forehand and backhand differ.
                    sweep(s, t, 1400.0, 500.0, 0.12, Wave::Saw, 0.1);
                    s.push(
                        Voice::new(t, t + 0.16, Wave::Noise { rate: 1.6, offset }, Env::constant(1.0), Env::ramp(t, 0.16 * vg, 0.001, 0.14), Bus::Sfx)
                            .filtered(synth::FilterKind::Bandpass, 2600.0 * vp * side, 1.2),
                    );
                }
                Sfx::Tick => {
                    blip(s, t, 880.0, 0.1, Wave::Square, 0.12);
                    blip(s, t, 1760.0, 0.06, Wave::Sine, 0.05);
                }
                Sfx::Go => {
                    blip(s, t, 1175.0, 0.28, Wave::Square, 0.13);
                    blip(s, t, 1760.0, 0.32, Wave::Triangle, 0.12);
                    sweep(s, t, 220.0, 440.0, 0.2, Wave::Triangle, 0.14);
                }
                Sfx::Land => {
                    s.push(
                        Voice::new(t, t + 0.12, Wave::Noise { rate: 0.8, offset }, Env::constant(1.0), Env::ramp(t, 0.22 * vg, 0.001, 0.1), Bus::Sfx)
                            .filtered(synth::FilterKind::Lowpass, 700.0 * vp, 1.0),
                    );
                    sweep(s, t, 150.0, 55.0, 0.12, Wave::Sine, 0.25);
                }
                Sfx::Milestone => {
                    for (i, f) in [784.0, 988.0, 1175.0, 1568.0].into_iter().enumerate() {
                        blip(s, t + i as f64 * 0.06, f, 0.18, Wave::Triangle, 0.12);
                    }
                    blip(s, t + 0.24, 2349.0, 0.3, Wave::Sine, 0.05);
                }
                Sfx::Hurt => sweep(s, t, 300.0, 120.0, 0.3, Wave::Square, 0.22),
                Sfx::Fuse => blip(s, t, 1200.0, 0.04, Wave::Square, 0.08),
                Sfx::Arrow => sweep(s, t, 2400.0, 900.0, 0.14, Wave::Triangle, 0.14),
                Sfx::Jump => sweep(s, t, 300.0, 700.0, 0.16, Wave::Triangle, 0.16),
                Sfx::ArrowHit => blip(s, t, 180.0, 0.08, Wave::Square, 0.18),
                Sfx::Explode => {
                    sweep(s, t, 160.0, 25.0, 0.9, Wave::Saw, 0.6);
                    sweep(s, t, 900.0, 60.0, 0.25, Wave::Square, 0.3);
                }
                Sfx::Clear => {
                    for i in 0..4 {
                        blip(s, t + i as f64 * 0.12, 440.0 * 1.25f64.powi(i), 0.1, Wave::Triangle, 0.2);
                    }
                }
                Sfx::Win => congratulations(s, t),
                Sfx::Start => {
                    blip(s, t, 330.0, 0.12, Wave::Triangle, 0.2);
                    blip(s, t + 0.15, 440.0, 0.12, Wave::Triangle, 0.2);
                }
                Sfx::BossRoar => roar(s, t, 1.0, 1.3, offset),
                Sfx::BossDeath => roar(s, t, 1.35, 2.4, offset),
                Sfx::BossCharge => {
                    // A rising, wavering whine: the eye filling with light.
                    let gain = Env::points(vec![(t, 0.0001), (t + 0.5, 0.09), (t + 0.7, 0.001)]);
                    s.push(Voice::new(t, t + 0.72, Wave::Square, Env::ramp(t, 220.0 * vp, 880.0 * vp, 0.65), gain, Bus::Sfx).vibrato(14.0, 0.03).filtered(synth::FilterKind::Lowpass, 2400.0, 1.0));
                    let gain = Env::points(vec![(t, 0.0001), (t + 0.55, 0.06), (t + 0.7, 0.001)]);
                    s.push(Voice::new(t, t + 0.72, Wave::Sine, Env::ramp(t, 440.0 * vp, 1760.0 * vp, 0.65), gain, Bus::Sfx));
                }
                Sfx::BossBolt => {
                    sweep(s, t, 1300.0, 240.0, 0.18, Wave::Saw, 0.09);
                    sweep(s, t, 260.0, 90.0, 0.14, Wave::Sine, 0.2);
                    s.push(
                        Voice::new(t, t + 0.12, Wave::Noise { rate: 1.2, offset }, Env::constant(1.0), Env::ramp(t, 0.1 * vg, 0.001, 0.1), Bus::Sfx)
                            .filtered(synth::FilterKind::Bandpass, 1800.0 * vp, 1.4),
                    );
                }
                Sfx::BossHurl => {
                    // A heave and a heavy whoosh.
                    sweep(s, t, 120.0, 45.0, 0.3, Wave::Sine, 0.32);
                    s.push(
                        Voice::new(t, t + 0.35, Wave::Noise { rate: 0.7, offset }, Env::constant(1.0), Env::points(vec![(t, 0.0001), (t + 0.08, 0.2 * vg), (t + 0.34, 0.001)]), Bus::Sfx)
                            .filtered(synth::FilterKind::Bandpass, 520.0 * vp, 0.9),
                    );
                }
                Sfx::BossSlam => {
                    sweep(s, t, 95.0, 28.0, 0.6, Wave::Sine, 0.55);
                    sweep(s, t, 180.0, 40.0, 0.25, Wave::Triangle, 0.3);
                    s.push(
                        Voice::new(t, t + 0.55, Wave::Noise { rate: 0.6, offset }, Env::constant(1.0), Env::ramp(t, 0.4 * vg, 0.001, 0.5), Bus::Sfx)
                            .filtered(synth::FilterKind::Lowpass, 420.0, 1.0),
                    );
                }
                Sfx::Anvil => {
                    // Struck steel rings at inharmonic partials; a dull thud underneath.
                    let v = 0.05 * vg;
                    for (f, d, g) in [(1180.0, 0.5, 1.0), (1180.0 * 2.76, 0.3, 0.5), (1180.0 * 5.4, 0.18, 0.3), (640.0, 0.25, 0.4)] {
                        blip(s, t, f, d, Wave::Sine, v * g);
                    }
                    s.push(
                        Voice::new(t, t + 0.05, Wave::Noise { rate: 1.4, offset }, Env::constant(1.0), Env::ramp(t, 0.08 * vg, 0.001, 0.04), Bus::Sfx)
                            .filtered(synth::FilterKind::Bandpass, 3200.0, 1.2),
                    );
                }
                Sfx::Buy => {
                    // A jingle of coins, then a bright confirming chord.
                    for i in 0..5 {
                        let at = t + i as f64 * 0.045;
                        blip(s, at, 1900.0 + (i * 173 % 500) as f64, 0.06, Wave::Sine, 0.06);
                        blip(s, at, 1900.0 * 2.76, 0.03, Wave::Sine, 0.02);
                    }
                    for (i, f) in [659.25, 830.61, 987.77].into_iter().enumerate() {
                        blip(s, t + 0.22 + i as f64 * 0.05, f, 0.3, Wave::Triangle, 0.1);
                    }
                }
                Sfx::Denied => {
                    blip(s, t, 196.0, 0.12, Wave::Square, 0.1);
                    blip(s, t + 0.13, 147.0, 0.2, Wave::Square, 0.1);
                }
                Sfx::Drink => {
                    // Glug, glug, and a sparkle as it takes.
                    for i in 0..3 {
                        let at = t + i as f64 * 0.11;
                        sweep(s, at, 260.0, 520.0, 0.08, Wave::Sine, 0.16);
                    }
                    for (i, f) in [1318.5, 1760.0, 2637.0].into_iter().enumerate() {
                        blip(s, t + 0.36 + i as f64 * 0.05, f, 0.15, Wave::Sine, 0.05);
                    }
                }
                Sfx::ShieldBreak => {
                    s.push(
                        Voice::new(t, t + 0.35, Wave::Noise { rate: 1.0, offset }, Env::constant(1.0), Env::ramp(t, 0.35 * vg, 0.001, 0.3), Bus::Sfx)
                            .filtered(synth::FilterKind::Bandpass, 1400.0, 0.8),
                    );
                    sweep(s, t, 500.0, 90.0, 0.3, Wave::Square, 0.14);
                    for i in 0..4 {
                        blip(s, t + 0.04 + i as f64 * 0.05, 2400.0 - i as f64 * 300.0, 0.05, Wave::Square, 0.03);
                    }
                }
                Sfx::Whoosh => {
                    // Air rushing past as the camera flies.
                    s.push(
                        Voice::new(t, t + 0.6, Wave::Noise { rate: 1.0, offset }, Env::constant(1.0), Env::points(vec![(t, 0.0001), (t + 0.25, 0.16 * vg), (t + 0.6, 0.001)]), Bus::Sfx)
                            .filtered(synth::FilterKind::Bandpass, 900.0 * vp, 1.4),
                    );
                    sweep(s, t, 180.0, 90.0, 0.5, Wave::Sine, 0.08);
                }
                Sfx::Brutal => {
                    // A crushing impact, a war-horn stab and a gong-like ring.
                    sweep(s, t, 110.0, 30.0, 0.7, Wave::Sine, 0.6);
                    s.push(
                        Voice::new(t, t + 0.5, Wave::Noise { rate: 0.6, offset }, Env::constant(1.0), Env::ramp(t, 0.45, 0.001, 0.45), Bus::Sfx)
                            .filtered(synth::FilterKind::Lowpass, 700.0, 1.0),
                    );
                    for (i, f) in [146.83, 220.0, 293.66].into_iter().enumerate() {
                        let gain = Env::points(vec![(t + 0.05, 0.0001), (t + 0.12, 0.09), (t + 0.7, 0.05), (t + 1.2, 0.001)]);
                        s.push(Voice::new(t + 0.05, t + 1.25, Wave::Saw, Env::constant(f * (1.0 + i as f64 * 0.002)), gain, Bus::Sfx).filtered(synth::FilterKind::Lowpass, 1400.0, 1.5));
                    }
                    for (f, g) in [(523.0, 0.06), (523.0 * 2.76, 0.03), (523.0 * 5.4, 0.015)] {
                        let gain = Env::points(vec![(t + 0.05, 0.0001), (t + 0.07, g), (t + 1.5, 0.0005)]);
                        s.push(Voice::new(t + 0.05, t + 1.55, Wave::Sine, Env::constant(f), gain, Bus::Sfx));
                    }
                }
                Sfx::Zap => {
                    s.push(
                        Voice::new(t, t + 0.22, Wave::Noise { rate: 2.0, offset }, Env::constant(1.0), Env::ramp(t, 0.2 * vg, 0.001, 0.2), Bus::Sfx)
                            .filtered(synth::FilterKind::Highpass, 2500.0, 0.8),
                    );
                    sweep(s, t, 2200.0, 300.0, 0.18, Wave::Saw, 0.08);
                }
                Sfx::BossHit => {
                    // A wet thwack: a thump, a squelch, a bright tick of impact.
                    sweep(s, t, 170.0, 70.0, 0.14, Wave::Sine, 0.34);
                    s.push(
                        Voice::new(t, t + 0.14, Wave::Noise { rate: 0.9, offset }, Env::constant(1.0), Env::ramp(t, 0.24 * vg, 0.001, 0.12), Bus::Sfx)
                            .filtered(synth::FilterKind::Lowpass, 900.0 * vp, 2.0),
                    );
                    blip(s, t, 2200.0, 0.03, Wave::Square, 0.05);
                    sweep(s, t + 0.02, 520.0, 260.0, 0.1, Wave::Square, 0.06);
                }
            }
            s.var_pitch = 1.0;
            s.var_gain = 1.0;
        });
    }
}

/**
 * A monster's bellow: a stack of detuned saws sliding down under a growl of
 * filtered noise, with a slow wobble. `pitch` lowers it (larger is deeper
 * and longer is sadder: the death cry uses both).
 */
fn roar(s: &mut Synth, t: f64, deep: f64, dur: f64, offset: f64) {
    let (p, g) = (s.var_pitch / deep, s.var_gain);
    let gain = |peak: f64| Env::points(vec![(t, 0.0001), (t + 0.12, peak * g), (t + dur * 0.6, peak * 0.7 * g), (t + dur, 0.001)]);
    for (i, ratio) in [1.0, 1.013, 0.497, 1.51].into_iter().enumerate() {
        let f0 = 190.0 * p * ratio;
        let freq = Env::points(vec![(t, f0 * 0.8), (t + 0.15, f0), (t + dur, f0 * 0.45)]);
        s.push(
            Voice::new(t, t + dur + 0.02, Wave::Saw, freq, gain(if i == 2 { 0.12 } else { 0.07 }), Bus::Sfx)
                .vibrato(5.5 + i as f64 * 0.7, 0.025)
                .filtered(synth::FilterKind::Lowpass, 1100.0, 3.0),
        );
    }
    s.push(
        Voice::new(t, t + dur + 0.02, Wave::Noise { rate: 0.5, offset }, Env::constant(1.0), gain(0.14), Bus::Sfx)
            .filtered(synth::FilterKind::Bandpass, 380.0 * p, 1.5),
    );
}

fn blip(s: &mut Synth, t: f64, freq: f64, dur: f64, wave: Wave, vol: f64) {
    let (p, g) = (s.var_pitch, s.var_gain);
    s.push(Voice::new(t, t + dur + 0.02, wave, Env::constant(freq * p), Env::ramp(t, vol * g, 0.001, dur), Bus::Sfx));
}

fn sweep(s: &mut Synth, t: f64, from: f64, to: f64, dur: f64, wave: Wave, vol: f64) {
    let (p, g) = (s.var_pitch, s.var_gain);
    s.push(Voice::new(t, t + dur + 0.02, wave, Env::ramp(t, from * p, to * p, dur), Env::ramp(t, vol * g, 0.001, dur), Bus::Sfx));
}

/**
 * The hoard is complete. A brassy four-note run up the C major arpeggio, a
 * drum-roll of noise under it, then a ringing chord with a touch of vibrato
 * and sparkles falling over the top: unmistakably "you did it", and long
 * enough to land over the win screen.
 */
fn congratulations(s: &mut Synth, t: f64) {
    const RUN: [(f64, f64); 4] = [(0.0, 523.25), (0.11, 659.25), (0.22, 783.99), (0.33, 1046.5)];
    for (d, f) in RUN {
        blip(s, t + d, f, 0.16, Wave::Square, 0.12);
        blip(s, t + d, f * 2.0, 0.12, Wave::Sine, 0.05);
    }
    // Snare roll building into the chord.
    for i in 0..10 {
        let at = t + i as f64 * 0.045;
        let offset = s.rng.next() * 1.5;
        s.push(
            Voice::new(at, at + 0.06, Wave::Noise { rate: 1.3, offset }, Env::constant(1.0), Env::ramp(at, 0.05 + i as f64 * 0.012, 0.001, 0.05), Bus::Sfx)
                .filtered(synth::FilterKind::Bandpass, 1900.0, 0.9),
        );
    }
    // The chord: C major with the octave, held and shimmering.
    let at = t + 0.46;
    for (i, f) in [523.25, 659.25, 783.99, 1046.5].into_iter().enumerate() {
        let gain = Env::points(vec![(at, 0.0001), (at + 0.04, 0.085), (at + 0.6, 0.06), (at + 1.7, 0.001)]);
        s.push(Voice::new(at, at + 1.75, Wave::Triangle, Env::constant(f), gain, Bus::Sfx).vibrato(5.5 + i as f64 * 0.3, 0.006));
        let gain = Env::points(vec![(at, 0.0001), (at + 0.02, 0.05), (at + 0.9, 0.001)]);
        s.push(Voice::new(at, at + 0.95, Wave::Square, Env::constant(f * 0.5), gain, Bus::Sfx));
    }
    // A low thump under the downbeat.
    s.push(Voice::new(at, at + 0.5, Wave::Sine, Env::ramp(at, 180.0, 60.0, 0.3), Env::ramp(at, 0.35, 0.001, 0.45), Bus::Sfx));
    // Sparkles.
    for _ in 0..14 {
        let d = at + 0.05 + s.rng.next() * 1.3;
        let f = 2093.0 * 2f64.powf(s.rng.pick(&[0.0, 4.0, 7.0, 12.0]) / 12.0);
        let vol = 0.03 + s.rng.next() * 0.02;
        blip(s, d, f, 0.09, Wave::Sine, vol);
    }
}

#[cfg(not(target_arch = "wasm32"))]
type StreamResult = Option<(Arc<Mutex<Synth>>, cpal::Stream)>;
#[cfg(target_arch = "wasm32")]
type StreamResult = Option<(Arc<Mutex<Synth>>, cpal::Stream)>;

fn open_stream(music_level: f64) -> StreamResult {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let host = cpal::default_host();
    let device = host.default_output_device()?;
    let supported = device.default_output_config().ok()?;
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let channels = config.channels as usize;
    let synth = Arc::new(Mutex::new(Synth::new(config.sample_rate.0 as f64, music_level)));
    let err = |e| eprintln!("audio stream error: {e}");

    let s = synth.clone();
    let stream = match format {
        cpal::SampleFormat::F32 => device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _| {
                    if let Ok(mut s) = s.lock() {
                        s.render(data, channels);
                    }
                },
                err,
                None,
            )
            .ok()?,
        cpal::SampleFormat::I16 => {
            let mut scratch: Vec<f32> = Vec::new();
            device
                .build_output_stream(
                    &config,
                    move |data: &mut [i16], _| {
                        scratch.resize(data.len(), 0.0);
                        if let Ok(mut s) = s.lock() {
                            s.render(&mut scratch, channels);
                        }
                        for (o, v) in data.iter_mut().zip(&scratch) {
                            *o = (v * i16::MAX as f32) as i16;
                        }
                    },
                    err,
                    None,
                )
                .ok()?
        }
        cpal::SampleFormat::U16 => {
            let mut scratch: Vec<f32> = Vec::new();
            device
                .build_output_stream(
                    &config,
                    move |data: &mut [u16], _| {
                        scratch.resize(data.len(), 0.0);
                        if let Ok(mut s) = s.lock() {
                            s.render(&mut scratch, channels);
                        }
                        for (o, v) in data.iter_mut().zip(&scratch) {
                            *o = ((v * 0.5 + 0.5) * u16::MAX as f32) as u16;
                        }
                    },
                    err,
                    None,
                )
                .ok()?
        }
        _ => return None,
    };
    stream.play().ok()?;
    Some((synth, stream))
}
