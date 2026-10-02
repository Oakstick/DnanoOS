//! Procedural audio. No sample files: every effect and the whole soundtrack
//! are synthesized, the same voices the web build made with Web Audio.

pub mod music;
pub mod synth;
pub mod track;

use std::sync::{Arc, Mutex};

use music::BASE_BPM;
use synth::{Bus, Env, Synth, Voice, Wave};
pub use track::Soundtrack;

use crate::logic::rng::Rng;
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
    // ---- Scene I
    /// A church bell tolling far off.
    Toll,
    /// Crows calling from the gate.
    Caw,
    /// A ball kicked.
    Kick,
    /// Iron struck: the gate.
    Clang,
    /// Krampus's bells, jangling out of the fog.
    Bells,
    /// Krampus's shriek as he rushes out.
    Shriek,
    /// A horror sting: the moment he seizes the boy.
    Stinger,
    /// Falling through into the underworld (and the way back slamming shut).
    Portal,
    /// Quitmer's snarl (and a wounded brute's).
    Growl,
    /// A witch's cackle as she conjures.
    Cackle,
    /// The loup-garou's howl, wounded.
    Howl,
    /// The tülükuş's cry as it takes wing: an owl's two low notes, a death foretold.
    Hoot,
    /// His chain snapping taut.
    Chain,
    /// Quitmer settling down with a whine.
    Whimper,
    /// A warrior's soul rising.
    Soul,
    /// The iron door grinding open.
    DoorGrind,
    /// Wind over the graves.
    Wind,
    /// Water dripping in the dungeon.
    Drip,
    // ---- the archangels' powers, each with its own voice
    /// Michael's fire: a blade ringing out in blue flame.
    MichaelFire,
    /// A foe turned by it: a soft chord of grace.
    MichaelTurn,
    /// Gabriel's horn, then the crack and long roll of his thunder.
    GabrielThunder,
    /// Raphael's beam singing out.
    RaphaelBeam,
    /// Uriel's wave: a deep boom and a roaring wall of fire.
    UrielWave,
}

/// Whose voice a line of the story types out in: each has its own blip.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Speech {
    Child,
    Old,
    Beast,
    Narrator,
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
    rng: Rng,
    foot: bool,
    soundtrack: Option<Soundtrack>,
    /// No music at all (the story plays over its own sounds) until the next soundtrack.
    hushed: bool,
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
            rng: Rng::from_time(),
            foot: false,
            soundtrack: None,
            hushed: false,
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
                    s.music.running = self.soundtrack.is_none() && !self.hushed;
                    s.music.bpm = BASE_BPM * TEMPO_STEP.powi(self.tempo_step as i32);
                    s.music_target = if self.ducked { MUSIC_VOLUME * 0.3 } else { MUSIC_VOLUME };
                }
                self.synth = Some(synth);
                self.stream = Some(stream);
                if let Some(t) = self.soundtrack.take() {
                    self.set_soundtrack(Some(t));
                }
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

    /**
     * Plays a recorded soundtrack from the top, looping, in place of the synth
     * tune (which stops and lets its last notes ring out); `None` hands the music
     * back to the synth. Asking for the one already playing leaves it running.
     */
    pub fn set_soundtrack(&mut self, which: Option<Soundtrack>) {
        if self.synth.is_some() && self.soundtrack == which && !self.hushed {
            return;
        }
        self.hushed = false;
        self.soundtrack = which;
        let track = which.and_then(track::Track::new);
        self.with(|s| {
            s.music.running = track.is_none();
            match track {
                Some(t) => {
                    s.track = Some(t);
                    s.track_level = 0.0;
                    s.track_target = 1.0;
                }
                // Fade the recording out; the synth tune picks up underneath.
                None => s.track_target = 0.0,
            }
        });
    }

    /// Lets the music fall silent, the recording and the synth tune both; the next soundtrack brings it back.
    pub fn hush(&mut self) {
        self.soundtrack = None;
        self.hushed = true;
        self.with(|s| {
            s.music.running = false;
            s.track_target = 0.0;
        });
    }

    /**
     * One blip of a line typing out in the story, in the speaker's voice: a
     * chirp for the boys, a low reedy murmur for the old man, a rasp of breath
     * for the dog, a soft tone for the narrator. Each is a little different.
     */
    pub fn play_voice(&mut self, voice: Speech) {
        let p = self.rng.range(0.88, 1.14);
        let offset = self.rng.next() * 2.0;
        self.with(|s| {
            let t = s.now;
            match voice {
                Speech::Child => {
                    blip(s, t, 640.0 * p, 0.035, Wave::Square, 0.022);
                    blip(s, t, 1280.0 * p, 0.025, Wave::Sine, 0.012);
                }
                Speech::Old => {
                    s.push(
                        Voice::new(t, t + 0.07, Wave::Saw, Env::constant(118.0 * p), Env::ramp(t, 0.05, 0.001, 0.065), Bus::Sfx).filtered(
                            synth::FilterKind::Lowpass,
                            700.0,
                            2.0,
                        ),
                    );
                    blip(s, t, 236.0 * p, 0.05, Wave::Triangle, 0.03);
                }
                Speech::Beast => {
                    s.push(
                        Voice::new(
                            t,
                            t + 0.09,
                            Wave::Noise { rate: 0.6, offset },
                            Env::constant(1.0),
                            Env::ramp(t, 0.09, 0.001, 0.08),
                            Bus::Sfx,
                        )
                        .filtered(synth::FilterKind::Bandpass, 260.0 * p, 1.4),
                    );
                    s.push(
                        Voice::new(t, t + 0.09, Wave::Saw, Env::constant(62.0 * p), Env::ramp(t, 0.06, 0.001, 0.08), Bus::Sfx).filtered(
                            synth::FilterKind::Lowpass,
                            400.0,
                            2.0,
                        ),
                    );
                }
                Speech::Narrator => blip(s, t, 392.0 * p, 0.05, Wave::Triangle, 0.018),
            }
        });
    }

    pub fn soundtrack(&self) -> Option<Soundtrack> {
        self.soundtrack
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
                Voice::new(
                    t,
                    t + 0.09,
                    Wave::Noise { rate: 0.9 * p, offset },
                    Env::constant(1.0),
                    Env::ramp(t, vol * 2.2, 0.001, 0.07),
                    Bus::Sfx,
                )
                .filtered(synth::FilterKind::Lowpass, cutoff, 1.0),
            );
            if slimy {
                s.push(Voice::new(
                    t,
                    t + 0.14,
                    Wave::Sine,
                    Env::ramp(t, 320.0 * p, 110.0, 0.12),
                    Env::ramp(t, vol * 1.4, 0.001, 0.12),
                    Bus::Sfx,
                ));
            } else {
                s.push(Voice::new(
                    t,
                    t + 0.06,
                    Wave::Sine,
                    Env::ramp(t, 120.0 * p, 70.0, 0.05),
                    Env::ramp(t, vol * 1.6, 0.001, 0.05),
                    Bus::Sfx,
                ));
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

    /**
     * A kill streak's sting, fiercer with each tier: the horn stab climbs a
     * minor third, a fourth and a fifth up, and from the third tier a crash of
     * cymbals and a second, higher horn join it.
     */
    pub fn play_brutal(&mut self, tier: u32) {
        let offset = self.rng.next() * 2.0;
        self.with(|s| {
            let t = s.now;
            s.var_pitch = [1.0, 2f64.powf(3.0 / 12.0), 2f64.powf(5.0 / 12.0), 2f64.powf(7.0 / 12.0)][tier.min(3) as usize];
            s.var_gain = 1.0;
            brutal(s, t, offset, tier);
            s.var_pitch = 1.0;
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
            if self.foot {
                1.0
            } else {
                0.86
            }
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
                        Voice::new(
                            t,
                            t + 0.16,
                            Wave::Noise { rate: 1.6, offset },
                            Env::constant(1.0),
                            Env::ramp(t, 0.16 * vg, 0.001, 0.14),
                            Bus::Sfx,
                        )
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
                        Voice::new(
                            t,
                            t + 0.12,
                            Wave::Noise { rate: 0.8, offset },
                            Env::constant(1.0),
                            Env::ramp(t, 0.22 * vg, 0.001, 0.1),
                            Bus::Sfx,
                        )
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
                    s.push(
                        Voice::new(t, t + 0.72, Wave::Square, Env::ramp(t, 220.0 * vp, 880.0 * vp, 0.65), gain, Bus::Sfx)
                            .vibrato(14.0, 0.03)
                            .filtered(synth::FilterKind::Lowpass, 2400.0, 1.0),
                    );
                    let gain = Env::points(vec![(t, 0.0001), (t + 0.55, 0.06), (t + 0.7, 0.001)]);
                    s.push(Voice::new(t, t + 0.72, Wave::Sine, Env::ramp(t, 440.0 * vp, 1760.0 * vp, 0.65), gain, Bus::Sfx));
                }
                Sfx::BossBolt => {
                    sweep(s, t, 1300.0, 240.0, 0.18, Wave::Saw, 0.09);
                    sweep(s, t, 260.0, 90.0, 0.14, Wave::Sine, 0.2);
                    s.push(
                        Voice::new(
                            t,
                            t + 0.12,
                            Wave::Noise { rate: 1.2, offset },
                            Env::constant(1.0),
                            Env::ramp(t, 0.1 * vg, 0.001, 0.1),
                            Bus::Sfx,
                        )
                        .filtered(synth::FilterKind::Bandpass, 1800.0 * vp, 1.4),
                    );
                }
                Sfx::BossHurl => {
                    // A heave and a heavy whoosh.
                    sweep(s, t, 120.0, 45.0, 0.3, Wave::Sine, 0.32);
                    s.push(
                        Voice::new(
                            t,
                            t + 0.35,
                            Wave::Noise { rate: 0.7, offset },
                            Env::constant(1.0),
                            Env::points(vec![(t, 0.0001), (t + 0.08, 0.2 * vg), (t + 0.34, 0.001)]),
                            Bus::Sfx,
                        )
                        .filtered(synth::FilterKind::Bandpass, 520.0 * vp, 0.9),
                    );
                }
                Sfx::BossSlam => {
                    sweep(s, t, 95.0, 28.0, 0.6, Wave::Sine, 0.55);
                    sweep(s, t, 180.0, 40.0, 0.25, Wave::Triangle, 0.3);
                    s.push(
                        Voice::new(
                            t,
                            t + 0.55,
                            Wave::Noise { rate: 0.6, offset },
                            Env::constant(1.0),
                            Env::ramp(t, 0.4 * vg, 0.001, 0.5),
                            Bus::Sfx,
                        )
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
                        Voice::new(
                            t,
                            t + 0.05,
                            Wave::Noise { rate: 1.4, offset },
                            Env::constant(1.0),
                            Env::ramp(t, 0.08 * vg, 0.001, 0.04),
                            Bus::Sfx,
                        )
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
                        Voice::new(
                            t,
                            t + 0.35,
                            Wave::Noise { rate: 1.0, offset },
                            Env::constant(1.0),
                            Env::ramp(t, 0.35 * vg, 0.001, 0.3),
                            Bus::Sfx,
                        )
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
                        Voice::new(
                            t,
                            t + 0.6,
                            Wave::Noise { rate: 1.0, offset },
                            Env::constant(1.0),
                            Env::points(vec![(t, 0.0001), (t + 0.25, 0.16 * vg), (t + 0.6, 0.001)]),
                            Bus::Sfx,
                        )
                        .filtered(synth::FilterKind::Bandpass, 900.0 * vp, 1.4),
                    );
                    sweep(s, t, 180.0, 90.0, 0.5, Wave::Sine, 0.08);
                }
                Sfx::Brutal => brutal(s, t, offset, 0),
                Sfx::Zap => {
                    s.push(
                        Voice::new(
                            t,
                            t + 0.22,
                            Wave::Noise { rate: 2.0, offset },
                            Env::constant(1.0),
                            Env::ramp(t, 0.2 * vg, 0.001, 0.2),
                            Bus::Sfx,
                        )
                        .filtered(synth::FilterKind::Highpass, 2500.0, 0.8),
                    );
                    sweep(s, t, 2200.0, 300.0, 0.18, Wave::Saw, 0.08);
                }
                Sfx::Toll => {
                    // A bell far off: a hum an octave under the strike, then inharmonic partials ringing on.
                    for (ratio, g, d) in [
                        (0.5, 0.05, 6.0),
                        (1.0, 0.07, 4.5),
                        (1.19, 0.035, 3.2),
                        (1.5, 0.028, 2.8),
                        (2.0, 0.045, 2.4),
                        (2.52, 0.02, 1.7),
                        (3.0, 0.014, 1.3),
                        (4.2, 0.008, 0.9),
                    ] {
                        let gain = Env::points(vec![(t, 0.0001), (t + 0.008, g * vg), (t + d, 0.0003)]);
                        s.push(Voice::new(t, t + d + 0.05, Wave::Sine, Env::constant(174.6 * ratio * vp), gain, Bus::Sfx));
                    }
                    s.push(
                        Voice::new(
                            t,
                            t + 0.06,
                            Wave::Noise { rate: 1.2, offset },
                            Env::constant(1.0),
                            Env::ramp(t, 0.05, 0.001, 0.05),
                            Bus::Sfx,
                        )
                        .filtered(synth::FilterKind::Bandpass, 1600.0, 1.0),
                    );
                }
                Sfx::Caw => {
                    // Two harsh caws: a buzzing tone that drops, through a nasal band.
                    for k in 0..2 {
                        let at = t + k as f64 * 0.34;
                        let freq = Env::points(vec![(at, 780.0 * vp), (at + 0.05, 900.0 * vp), (at + 0.2, 540.0 * vp)]);
                        let gain = Env::points(vec![(at, 0.0001), (at + 0.02, 0.06 * vg), (at + 0.2, 0.001)]);
                        s.push(Voice::new(at, at + 0.22, Wave::Saw, freq, gain, Bus::Sfx).vibrato(36.0, 0.04).filtered(
                            synth::FilterKind::Bandpass,
                            1400.0,
                            3.0,
                        ));
                        s.push(
                            Voice::new(
                                at,
                                at + 0.16,
                                Wave::Noise { rate: 1.3, offset },
                                Env::constant(1.0),
                                Env::ramp(at, 0.035 * vg, 0.001, 0.15),
                                Bus::Sfx,
                            )
                            .filtered(synth::FilterKind::Bandpass, 1900.0, 2.0),
                        );
                    }
                }
                Sfx::Kick => {
                    sweep(s, t, 150.0, 60.0, 0.1, Wave::Sine, 0.2);
                    s.push(
                        Voice::new(
                            t,
                            t + 0.05,
                            Wave::Noise { rate: 0.9, offset },
                            Env::constant(1.0),
                            Env::ramp(t, 0.1 * vg, 0.001, 0.045),
                            Bus::Sfx,
                        )
                        .filtered(synth::FilterKind::Lowpass, 700.0, 1.0),
                    );
                }
                Sfx::Clang => {
                    // Wrought iron rattling: inharmonic partials and a burst of grit.
                    for (f, d, g) in [(420.0, 0.7, 0.06), (420.0 * 2.76, 0.4, 0.04), (420.0 * 5.4, 0.25, 0.025), (1130.0, 0.5, 0.03)] {
                        blip(s, t, f, d, Wave::Sine, g);
                    }
                    s.push(
                        Voice::new(
                            t,
                            t + 0.12,
                            Wave::Noise { rate: 1.1, offset },
                            Env::constant(1.0),
                            Env::ramp(t, 0.12 * vg, 0.001, 0.1),
                            Bus::Sfx,
                        )
                        .filtered(synth::FilterKind::Bandpass, 2600.0, 1.3),
                    );
                }
                Sfx::Bells => {
                    // Small bells jangling, and a dull cowbell clank or two under them.
                    for k in 0..9 {
                        let at = t + k as f64 * 0.085 + (k * 37 % 11) as f64 * 0.004;
                        let f = 1650.0 + (k * 263 % 900) as f64;
                        blip(s, at, f, 0.16, Wave::Sine, 0.03);
                        blip(s, at, f * 2.76, 0.08, Wave::Sine, 0.012);
                    }
                    for k in 0..3 {
                        let at = t + 0.05 + k as f64 * 0.27;
                        s.push(
                            Voice::new(
                                at,
                                at + 0.25,
                                Wave::Square,
                                Env::constant(540.0 * vp),
                                Env::ramp(at, 0.04 * vg, 0.001, 0.22),
                                Bus::Sfx,
                            )
                            .filtered(synth::FilterKind::Bandpass, 800.0, 2.5),
                        );
                        s.push(
                            Voice::new(
                                at,
                                at + 0.25,
                                Wave::Square,
                                Env::constant(812.0 * vp),
                                Env::ramp(at, 0.025 * vg, 0.001, 0.2),
                                Bus::Sfx,
                            )
                            .filtered(synth::FilterKind::Bandpass, 1200.0, 2.5),
                        );
                    }
                }
                Sfx::Shriek => {
                    roar(s, t, 0.6, 0.95, offset);
                    let freq = Env::points(vec![(t, 1500.0 * vp), (t + 0.15, 1900.0 * vp), (t + 0.8, 700.0 * vp)]);
                    let gain = Env::points(vec![(t, 0.0001), (t + 0.06, 0.07 * vg), (t + 0.5, 0.05 * vg), (t + 0.85, 0.001)]);
                    s.push(Voice::new(t, t + 0.88, Wave::Saw, freq, gain, Bus::Sfx).vibrato(28.0, 0.05).filtered(
                        synth::FilterKind::Bandpass,
                        2100.0,
                        2.0,
                    ));
                    s.push(
                        Voice::new(
                            t,
                            t + 0.7,
                            Wave::Noise { rate: 1.6, offset },
                            Env::constant(1.0),
                            Env::points(vec![(t, 0.0001), (t + 0.05, 0.12 * vg), (t + 0.7, 0.001)]),
                            Bus::Sfx,
                        )
                        .filtered(synth::FilterKind::Highpass, 2400.0, 0.8),
                    );
                }
                Sfx::Cackle => {
                    // A witch's laugh: five cracked "hee"s tumbling down in pitch.
                    for k in 0..5 {
                        let at = t + k as f64 * 0.13;
                        let f = (880.0 - k as f64 * 70.0) * vp;
                        let freq = Env::points(vec![(at, f * 1.25), (at + 0.04, f), (at + 0.11, f * 0.85)]);
                        let gain = Env::points(vec![(at, 0.0001), (at + 0.015, 0.09 * vg), (at + 0.09, 0.05 * vg), (at + 0.12, 0.001)]);
                        s.push(Voice::new(at, at + 0.13, Wave::Saw, freq, gain, Bus::Sfx).vibrato(24.0, 0.04).filtered(
                            synth::FilterKind::Bandpass,
                            1600.0,
                            1.4,
                        ));
                    }
                }
                Sfx::Howl => {
                    // A wolf's howl: rising into a long held note that wavers and falls away.
                    let freq = Env::points(vec![(t, 260.0 * vp), (t + 0.35, 520.0 * vp), (t + 1.1, 540.0 * vp), (t + 1.6, 380.0 * vp)]);
                    let gain = Env::points(vec![(t, 0.0001), (t + 0.2, 0.08 * vg), (t + 1.0, 0.07 * vg), (t + 1.65, 0.001)]);
                    s.push(Voice::new(t, t + 1.7, Wave::Saw, freq, gain, Bus::Sfx).vibrato(5.5, 0.03).filtered(
                        synth::FilterKind::Lowpass,
                        1400.0,
                        1.2,
                    ));
                    let high = Env::points(vec![(t, 520.0 * vp), (t + 0.35, 1040.0 * vp), (t + 1.1, 1080.0 * vp), (t + 1.6, 760.0 * vp)]);
                    let soft = Env::points(vec![(t, 0.0001), (t + 0.25, 0.025 * vg), (t + 1.0, 0.02 * vg), (t + 1.65, 0.001)]);
                    s.push(Voice::new(t, t + 1.7, Wave::Sine, high, soft, Bus::Sfx).vibrato(5.5, 0.03));
                }
                Sfx::Hoot => {
                    // An eagle-owl: a soft low "hoo", a breath, then a longer one falling away, with air in it.
                    for (start, len, f0, f1) in [(0.0, 0.3, 330.0, 305.0), (0.45, 0.7, 320.0, 240.0)] {
                        let a = t + start;
                        let freq = Env::points(vec![(a, f0 * vp), (a + len, f1 * vp)]);
                        let gain = Env::points(vec![(a, 0.0001), (a + 0.07, 0.12 * vg), (a + len * 0.6, 0.08 * vg), (a + len, 0.0005)]);
                        s.push(Voice::new(a, a + len + 0.02, Wave::Sine, freq, gain, Bus::Sfx).vibrato(4.0, 0.012));
                        let breath = Env::points(vec![(a, 0.0001), (a + 0.05, 0.02 * vg), (a + len, 0.0005)]);
                        s.push(Voice::new(a, a + len, Wave::Noise { rate: 0.7, offset }, Env::constant(1.0), breath, Bus::Sfx).filtered(
                            synth::FilterKind::Bandpass,
                            f0 * 2.0 * vp,
                            2.0,
                        ));
                    }
                }
                Sfx::Stinger => {
                    // A boom and a clash of close, sour notes.
                    s.push(Voice::new(t, t + 1.5, Wave::Sine, Env::ramp(t, 70.0, 28.0, 1.2), Env::ramp(t, 0.5 * vg, 0.001, 1.4), Bus::Sfx));
                    for (i, f) in [233.1, 246.9, 261.6, 277.2, 466.2].into_iter().enumerate() {
                        let gain = Env::points(vec![(t, 0.0001), (t + 0.015, 0.045 * vg), (t + 0.5, 0.025 * vg), (t + 1.6, 0.0005)]);
                        s.push(Voice::new(t, t + 1.65, Wave::Saw, Env::constant(f * (1.0 + i as f64 * 0.003)), gain, Bus::Sfx).filtered(
                            synth::FilterKind::Lowpass,
                            2200.0,
                            1.5,
                        ));
                    }
                    s.push(
                        Voice::new(
                            t,
                            t + 0.4,
                            Wave::Noise { rate: 0.8, offset },
                            Env::constant(1.0),
                            Env::ramp(t, 0.3 * vg, 0.001, 0.38),
                            Bus::Sfx,
                        )
                        .filtered(synth::FilterKind::Lowpass, 1200.0, 1.0),
                    );
                }
                Sfx::Portal => {
                    // A rush of air climbing through four bands, a rising tone, then a deep boom and a shimmer.
                    for (k, f) in [380.0, 760.0, 1500.0, 3000.0].into_iter().enumerate() {
                        let at = t + k as f64 * 0.09;
                        let gain = Env::points(vec![(at, 0.0001), (at + 0.25, 0.1 * vg), (at + 0.55, 0.001)]);
                        s.push(
                            Voice::new(
                                at,
                                at + 0.58,
                                Wave::Noise { rate: 1.0, offset: offset + k as f64 * 0.3 },
                                Env::constant(1.0),
                                gain,
                                Bus::Sfx,
                            )
                            .filtered(synth::FilterKind::Bandpass, f * vp, 1.2),
                        );
                    }
                    sweep(s, t, 90.0, 620.0, 0.6, Wave::Triangle, 0.1);
                    let at = t + 0.55;
                    s.push(Voice::new(
                        at,
                        at + 1.0,
                        Wave::Sine,
                        Env::ramp(at, 95.0, 30.0, 0.9),
                        Env::ramp(at, 0.45 * vg, 0.001, 0.95),
                        Bus::Sfx,
                    ));
                    for f in [1568.0, 2093.0, 2637.0] {
                        let gain = Env::points(vec![(at, 0.0001), (at + 0.03, 0.03), (at + 1.2, 0.0005)]);
                        s.push(Voice::new(at, at + 1.25, Wave::Sine, Env::constant(f), gain, Bus::Sfx).vibrato(6.0, 0.01));
                    }
                }
                Sfx::Growl => {
                    // A rumbling snarl (its loudness shudders), rising to a snapping bark.
                    let mut shudder = vec![(t, 0.0001)];
                    for k in 1..24 {
                        let at = t + k as f64 * 0.045;
                        shudder.push((at, if k % 2 == 0 { 0.1 * vg } else { 0.045 * vg }));
                    }
                    shudder.push((t + 1.15, 0.001));
                    for (i, f) in [82.0, 83.4, 164.0].into_iter().enumerate() {
                        let pts: Vec<(f64, f64)> = shudder.iter().map(|&(a, g)| (a, g * if i == 2 { 0.5 } else { 1.0 })).collect();
                        s.push(
                            Voice::new(t, t + 1.18, Wave::Saw, Env::constant(f * vp), Env::points(pts), Bus::Sfx)
                                .vibrato(17.0, 0.07)
                                .filtered(synth::FilterKind::Lowpass, 650.0, 2.0),
                        );
                    }
                    s.push(
                        Voice::new(
                            t,
                            t + 1.18,
                            Wave::Noise { rate: 0.55, offset },
                            Env::constant(1.0),
                            Env::points(shudder.clone()),
                            Bus::Sfx,
                        )
                        .filtered(synth::FilterKind::Bandpass, 250.0 * vp, 1.6),
                    );
                    let at = t + 1.05;
                    let freq = Env::points(vec![(at, 260.0 * vp), (at + 0.06, 320.0 * vp), (at + 0.3, 120.0 * vp)]);
                    s.push(
                        Voice::new(
                            at,
                            at + 0.32,
                            Wave::Saw,
                            freq,
                            Env::points(vec![(at, 0.0001), (at + 0.03, 0.16 * vg), (at + 0.3, 0.001)]),
                            Bus::Sfx,
                        )
                        .filtered(synth::FilterKind::Lowpass, 1400.0, 3.0),
                    );
                    s.push(
                        Voice::new(
                            at,
                            at + 0.25,
                            Wave::Noise { rate: 0.8, offset },
                            Env::constant(1.0),
                            Env::ramp(at, 0.16 * vg, 0.001, 0.24),
                            Bus::Sfx,
                        )
                        .filtered(synth::FilterKind::Bandpass, 700.0, 1.0),
                    );
                }
                Sfx::Chain => {
                    for k in 0..7 {
                        let at = t + k as f64 * 0.05 + (k * 13 % 7) as f64 * 0.006;
                        let f = 2300.0 + (k * 677 % 1500) as f64;
                        blip(s, at, f, 0.05, Wave::Sine, 0.03);
                        blip(s, at, f * 1.48, 0.035, Wave::Sine, 0.015);
                    }
                    s.push(
                        Voice::new(
                            t,
                            t + 0.3,
                            Wave::Noise { rate: 1.5, offset },
                            Env::constant(1.0),
                            Env::ramp(t, 0.05 * vg, 0.001, 0.28),
                            Bus::Sfx,
                        )
                        .filtered(synth::FilterKind::Highpass, 3200.0, 0.8),
                    );
                }
                Sfx::Whimper => {
                    let freq = Env::points(vec![(t, 880.0 * vp), (t + 0.18, 700.0 * vp), (t + 0.32, 830.0 * vp), (t + 0.62, 560.0 * vp)]);
                    let gain = Env::points(vec![(t, 0.0001), (t + 0.05, 0.045 * vg), (t + 0.45, 0.03 * vg), (t + 0.65, 0.001)]);
                    s.push(Voice::new(t, t + 0.66, Wave::Triangle, freq, gain, Bus::Sfx).vibrato(7.0, 0.03).filtered(
                        synth::FilterKind::Lowpass,
                        1600.0,
                        1.0,
                    ));
                    s.push(
                        Voice::new(
                            t + 0.5,
                            t + 1.0,
                            Wave::Noise { rate: 0.7, offset },
                            Env::constant(1.0),
                            Env::points(vec![(t + 0.5, 0.0001), (t + 0.6, 0.05 * vg), (t + 1.0, 0.001)]),
                            Bus::Sfx,
                        )
                        .filtered(synth::FilterKind::Lowpass, 800.0, 1.0),
                    );
                }
                Sfx::Soul => {
                    // A breath of wind and a high, slow-blooming chord that hangs in the air.
                    s.push(Voice::new(
                        t,
                        t + 1.2,
                        Wave::Sine,
                        Env::ramp(t, 110.0, 220.0, 1.0),
                        Env::points(vec![(t, 0.0001), (t + 0.3, 0.08 * vg), (t + 1.2, 0.001)]),
                        Bus::Sfx,
                    ));
                    for (i, f) in [880.0, 1046.5, 1318.5, 1760.0].into_iter().enumerate() {
                        let at = t + i as f64 * 0.06;
                        let gain = Env::points(vec![(at, 0.0001), (at + 0.35, 0.03 * vg), (at + 2.0, 0.0005)]);
                        s.push(
                            Voice::new(at, at + 2.05, Wave::Triangle, Env::constant(f * vp), gain, Bus::Sfx)
                                .vibrato(5.0 + i as f64 * 0.4, 0.008),
                        );
                    }
                    for k in 0..5 {
                        blip(s, t + 0.2 + k as f64 * 0.11, 2637.0 * 2f64.powf((k * 5 % 12) as f64 / 12.0), 0.12, Wave::Sine, 0.02);
                    }
                }
                Sfx::DoorGrind => {
                    // Iron grinding on stone, a groaning hinge, and the thud as it comes to rest.
                    let mut grind = vec![(t, 0.0001)];
                    for k in 1..40 {
                        let at = t + k as f64 * 0.05;
                        grind.push((at, if k % 3 == 0 { 0.16 * vg } else { 0.07 * vg }));
                    }
                    grind.push((t + 2.1, 0.001));
                    s.push(
                        Voice::new(t, t + 2.12, Wave::Noise { rate: 0.5, offset }, Env::constant(1.0), Env::points(grind), Bus::Sfx)
                            .filtered(synth::FilterKind::Lowpass, 520.0, 1.5),
                    );
                    s.push(
                        Voice::new(
                            t,
                            t + 2.1,
                            Wave::Saw,
                            Env::points(vec![(t, 52.0), (t + 2.0, 44.0)]),
                            Env::points(vec![(t, 0.0001), (t + 0.3, 0.08 * vg), (t + 2.0, 0.001)]),
                            Bus::Sfx,
                        )
                        .vibrato(3.0, 0.2)
                        .filtered(synth::FilterKind::Lowpass, 300.0, 2.0),
                    );
                    let freq = Env::points(vec![(t + 0.2, 430.0), (t + 0.9, 380.0), (t + 1.5, 460.0)]);
                    s.push(
                        Voice::new(
                            t + 0.2,
                            t + 1.6,
                            Wave::Triangle,
                            freq,
                            Env::points(vec![(t + 0.2, 0.0001), (t + 0.4, 0.035 * vg), (t + 1.6, 0.001)]),
                            Bus::Sfx,
                        )
                        .vibrato(11.0, 0.02)
                        .filtered(synth::FilterKind::Bandpass, 900.0, 3.0),
                    );
                    let at = t + 2.05;
                    s.push(Voice::new(
                        at,
                        at + 0.7,
                        Wave::Sine,
                        Env::ramp(at, 80.0, 32.0, 0.6),
                        Env::ramp(at, 0.35 * vg, 0.001, 0.65),
                        Bus::Sfx,
                    ));
                }
                Sfx::Wind => {
                    // A long gust, swelling and falling away.
                    let gain =
                        |g: f64| Env::points(vec![(t, 0.0001), (t + 1.4, g * 0.6), (t + 2.6, g), (t + 3.8, g * 0.5), (t + 5.5, 0.0005)]);
                    s.push(
                        Voice::new(t, t + 5.6, Wave::Noise { rate: 0.8, offset }, Env::constant(1.0), gain(0.045 * vg), Bus::Sfx).filtered(
                            synth::FilterKind::Bandpass,
                            520.0 * vp,
                            0.8,
                        ),
                    );
                    s.push(
                        Voice::new(
                            t,
                            t + 5.6,
                            Wave::Noise { rate: 1.3, offset: offset + 0.7 },
                            Env::constant(1.0),
                            gain(0.02 * vg),
                            Bus::Sfx,
                        )
                        .filtered(synth::FilterKind::Bandpass, 1300.0 * vp, 2.0),
                    );
                }
                Sfx::Drip => {
                    sweep(s, t, 1900.0, 900.0, 0.05, Wave::Sine, 0.04);
                    sweep(s, t + 0.13, 1900.0, 900.0, 0.05, Wave::Sine, 0.012);
                    sweep(s, t + 0.27, 1900.0, 900.0, 0.05, Wave::Sine, 0.004);
                }
                Sfx::MichaelFire => {
                    // Steel ringing out, a rushing breath of flame, and a pure fifth climbing with it.
                    blip(s, t, 2093.0, 0.4, Wave::Sine, 0.05);
                    blip(s, t, 2093.0 * 2.76, 0.22, Wave::Sine, 0.02);
                    s.push(
                        Voice::new(
                            t,
                            t + 0.45,
                            Wave::Noise { rate: 1.4, offset },
                            Env::constant(1.0),
                            Env::points(vec![(t, 0.0001), (t + 0.06, 0.14 * vg), (t + 0.44, 0.001)]),
                            Bus::Sfx,
                        )
                        .filtered(synth::FilterKind::Bandpass, 1500.0 * vp, 0.9),
                    );
                    sweep(s, t, 523.25, 1046.5, 0.32, Wave::Triangle, 0.1);
                    sweep(s, t, 783.99, 1568.0, 0.32, Wave::Sine, 0.06);
                }
                Sfx::MichaelTurn => {
                    // A major chord blooming upward, with a bell over it: the foe's heart turned.
                    for (i, f) in [659.25, 830.61, 987.77, 1318.5].into_iter().enumerate() {
                        let at = t + i as f64 * 0.05;
                        let gain = Env::points(vec![(at, 0.0001), (at + 0.03, 0.05 * vg), (at + 0.8, 0.0005)]);
                        s.push(Voice::new(at, at + 0.82, Wave::Sine, Env::constant(f * vp), gain, Bus::Sfx).vibrato(5.0, 0.006));
                    }
                    blip(s, t + 0.2, 2637.0, 0.3, Wave::Sine, 0.025);
                }
                Sfx::GabrielThunder => {
                    // Gabriel's horn: two brassy notes a fourth apart ...
                    for (k, (f, d)) in [(293.66, 0.16), (392.0, 0.45)].into_iter().enumerate() {
                        let at = t + k as f64 * 0.15;
                        for (ratio, g) in [(1.0, 0.07), (1.004, 0.05), (2.0, 0.03)] {
                            let gain = Env::points(vec![(at, 0.0001), (at + 0.03, g * vg), (at + d, g * 0.6 * vg), (at + d + 0.06, 0.001)]);
                            s.push(Voice::new(at, at + d + 0.08, Wave::Saw, Env::constant(f * ratio * vp), gain, Bus::Sfx).filtered(
                                synth::FilterKind::Lowpass,
                                1800.0,
                                1.2,
                            ));
                        }
                    }
                    // ... then the crack overhead and the thunder rolling away.
                    let c = t + 0.55;
                    s.push(
                        Voice::new(
                            c,
                            c + 0.16,
                            Wave::Noise { rate: 2.0, offset },
                            Env::constant(1.0),
                            Env::ramp(c, 0.45 * vg, 0.001, 0.14),
                            Bus::Sfx,
                        )
                        .filtered(synth::FilterKind::Highpass, 1800.0, 0.8),
                    );
                    let mut roll = vec![(c, 0.0001), (c + 0.04, 0.5 * vg)];
                    for k in 1..20 {
                        let at = c + 0.05 + k as f64 * 0.11;
                        let fall = 1.0 - k as f64 / 20.0;
                        roll.push((at, if k % 3 == 0 { 0.45 * fall * vg } else { 0.22 * fall * vg }));
                    }
                    roll.push((c + 2.4, 0.0005));
                    s.push(
                        Voice::new(
                            c,
                            c + 2.45,
                            Wave::Noise { rate: 0.35, offset: offset + 0.5 },
                            Env::constant(1.0),
                            Env::points(roll),
                            Bus::Sfx,
                        )
                        .filtered(synth::FilterKind::Lowpass, 300.0, 1.0),
                    );
                    s.push(Voice::new(c, c + 2.0, Wave::Sine, Env::ramp(c, 70.0, 28.0, 1.8), Env::ramp(c, 0.5 * vg, 0.001, 1.9), Bus::Sfx));
                }
                Sfx::RaphaelBeam => {
                    // A breath of charge, then the beam singing: a humming chord of saws, a sizzle, a high clear tone.
                    sweep(s, t, 200.0, 1200.0, 0.1, Wave::Sine, 0.08);
                    let shape =
                        |g: f64| Env::points(vec![(t + 0.08, 0.0001), (t + 0.12, g * vg), (t + 0.6, g * 0.6 * vg), (t + 0.9, 0.001)]);
                    for f in [110.0, 110.7, 220.0, 330.0] {
                        s.push(
                            Voice::new(t + 0.08, t + 0.92, Wave::Saw, Env::constant(f * vp), shape(0.05), Bus::Sfx)
                                .vibrato(9.0, 0.01)
                                .filtered(synth::FilterKind::Lowpass, 1600.0, 2.0),
                        );
                    }
                    s.push(
                        Voice::new(t + 0.08, t + 0.92, Wave::Noise { rate: 1.8, offset }, Env::constant(1.0), shape(0.07), Bus::Sfx)
                            .filtered(synth::FilterKind::Highpass, 3000.0, 0.8),
                    );
                    for f in [880.0, 1320.0] {
                        s.push(
                            Voice::new(t + 0.08, t + 0.92, Wave::Sine, Env::constant(f * vp), shape(0.03), Bus::Sfx).vibrato(6.0, 0.008),
                        );
                    }
                }
                Sfx::UrielWave => {
                    // A deep boom, a roaring wall of fire rushing out, crackling, and a low brass chord swelling under it all.
                    s.push(Voice::new(t, t + 1.8, Wave::Sine, Env::ramp(t, 60.0, 24.0, 1.6), Env::ramp(t, 0.6 * vg, 0.001, 1.7), Bus::Sfx));
                    s.push(
                        Voice::new(
                            t,
                            t + 0.9,
                            Wave::Noise { rate: 0.9, offset },
                            Env::constant(1.0),
                            Env::points(vec![(t, 0.0001), (t + 0.1, 0.2 * vg), (t + 0.85, 0.001)]),
                            Bus::Sfx,
                        )
                        .filtered(synth::FilterKind::Bandpass, 1400.0 * vp, 0.8),
                    );
                    s.push(
                        Voice::new(
                            t,
                            t + 1.9,
                            Wave::Noise { rate: 0.6, offset: offset + 0.3 },
                            Env::constant(1.0),
                            Env::points(vec![(t, 0.0001), (t + 0.3, 0.3 * vg), (t + 1.85, 0.001)]),
                            Bus::Sfx,
                        )
                        .filtered(synth::FilterKind::Lowpass, 500.0, 1.0),
                    );
                    for k in 0..10 {
                        let at = t + 0.1 + k as f64 * 0.13 + (k * 37 % 11) as f64 * 0.01;
                        s.push(
                            Voice::new(
                                at,
                                at + 0.05,
                                Wave::Noise { rate: 1.5, offset: offset + k as f64 * 0.2 },
                                Env::constant(1.0),
                                Env::ramp(at, 0.07 * vg, 0.001, 0.04),
                                Bus::Sfx,
                            )
                            .filtered(synth::FilterKind::Bandpass, 2500.0 + (k * 311 % 900) as f64, 1.2),
                        );
                    }
                    for f in [73.42, 110.0, 146.83, 220.0] {
                        let gain = Env::points(vec![(t, 0.0001), (t + 0.5, 0.06 * vg), (t + 1.4, 0.04 * vg), (t + 2.4, 0.001)]);
                        s.push(Voice::new(t, t + 2.45, Wave::Saw, Env::constant(f * vp), gain, Bus::Sfx).filtered(
                            synth::FilterKind::Lowpass,
                            900.0,
                            1.5,
                        ));
                    }
                }
                Sfx::BossHit => {
                    // A wet thwack: a thump, a squelch, a bright tick of impact.
                    sweep(s, t, 170.0, 70.0, 0.14, Wave::Sine, 0.34);
                    s.push(
                        Voice::new(
                            t,
                            t + 0.14,
                            Wave::Noise { rate: 0.9, offset },
                            Env::constant(1.0),
                            Env::ramp(t, 0.24 * vg, 0.001, 0.12),
                            Bus::Sfx,
                        )
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

/// The streak sting (see `play_brutal`); `s.var_pitch` carries the tier's key.
fn brutal(s: &mut Synth, t: f64, offset: f64, tier: u32) {
    let (vp, vg) = (s.var_pitch, s.var_gain);
    // A crushing impact, a war-horn stab and a gong-like ring.
    sweep(s, t, 110.0, 30.0, 0.7, Wave::Sine, 0.6);
    s.push(
        Voice::new(t, t + 0.5, Wave::Noise { rate: 0.6, offset }, Env::constant(1.0), Env::ramp(t, 0.45 * vg, 0.001, 0.45), Bus::Sfx)
            .filtered(synth::FilterKind::Lowpass, 700.0, 1.0),
    );
    for (i, f) in [146.83, 220.0, 293.66].into_iter().enumerate() {
        let gain = Env::points(vec![(t + 0.05, 0.0001), (t + 0.12, 0.09), (t + 0.7, 0.05), (t + 1.2, 0.001)]);
        s.push(Voice::new(t + 0.05, t + 1.25, Wave::Saw, Env::constant(f * vp * (1.0 + i as f64 * 0.002)), gain, Bus::Sfx).filtered(
            synth::FilterKind::Lowpass,
            1400.0,
            1.5,
        ));
    }
    for (f, g) in [(523.0, 0.06), (523.0 * 2.76, 0.03), (523.0 * 5.4, 0.015)] {
        let gain = Env::points(vec![(t + 0.05, 0.0001), (t + 0.07, g), (t + 1.5, 0.0005)]);
        s.push(Voice::new(t + 0.05, t + 1.55, Wave::Sine, Env::constant(f * vp), gain, Bus::Sfx));
    }
    if tier >= 2 {
        // A crash of cymbals over it, and a second horn an octave up.
        s.push(
            Voice::new(
                t + 0.04,
                t + 1.1,
                Wave::Noise { rate: 1.9, offset: offset + 0.4 },
                Env::constant(1.0),
                Env::ramp(t + 0.04, 0.2 * vg, 0.001, 1.0),
                Bus::Sfx,
            )
            .filtered(synth::FilterKind::Highpass, 3800.0, 0.7),
        );
        for f in [293.66, 440.0] {
            let gain = Env::points(vec![(t + 0.1, 0.0001), (t + 0.16, 0.05), (t + 0.8, 0.03), (t + 1.3, 0.001)]);
            s.push(Voice::new(t + 0.1, t + 1.35, Wave::Saw, Env::constant(f * 2.0 * vp), gain, Bus::Sfx).filtered(
                synth::FilterKind::Lowpass,
                2200.0,
                1.5,
            ));
        }
    }
    if tier >= 3 {
        // At the top, a war drum rolls in under it.
        for k in 0..4 {
            let at = t + 0.3 + k as f64 * 0.13;
            s.push(Voice::new(at, at + 0.3, Wave::Sine, Env::ramp(at, 120.0, 45.0, 0.25), Env::ramp(at, 0.35 * vg, 0.001, 0.28), Bus::Sfx));
        }
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
    s.push(Voice::new(t, t + dur + 0.02, Wave::Noise { rate: 0.5, offset }, Env::constant(1.0), gain(0.14), Bus::Sfx).filtered(
        synth::FilterKind::Bandpass,
        380.0 * p,
        1.5,
    ));
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
            Voice::new(
                at,
                at + 0.06,
                Wave::Noise { rate: 1.3, offset },
                Env::constant(1.0),
                Env::ramp(at, 0.05 + i as f64 * 0.012, 0.001, 0.05),
                Bus::Sfx,
            )
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

#[cfg(test)]
mod tests {
    use super::*;

    /// An audio manager driving a synth with no device behind it, for rendering in tests.
    fn silent() -> (AudioManager, Arc<Mutex<Synth>>) {
        let synth = Arc::new(Mutex::new(Synth::new(48_000.0, MUSIC_VOLUME)));
        synth.lock().unwrap().master = MASTER_VOLUME;
        let mut a = AudioManager::new();
        a.muted = false;
        a.synth = Some(synth.clone());
        (a, synth)
    }

    #[test]
    fn the_story_sounds_render_clean() {
        let (mut a, synth) = silent();
        a.hush();
        let all = [
            Sfx::Toll,
            Sfx::Caw,
            Sfx::Kick,
            Sfx::Clang,
            Sfx::Bells,
            Sfx::Shriek,
            Sfx::Stinger,
            Sfx::Portal,
            Sfx::Growl,
            Sfx::Chain,
            Sfx::Whimper,
            Sfx::Soul,
            Sfx::DoorGrind,
            Sfx::Wind,
            Sfx::Drip,
            Sfx::Cackle,
            Sfx::Howl,
            Sfx::Hoot,
            // (and the archangels' powers)
            Sfx::MichaelFire,
            Sfx::MichaelTurn,
            Sfx::GabrielThunder,
            Sfx::RaphaelBeam,
            Sfx::UrielWave,
        ];
        let render = |what: &str| {
            let mut buf = vec![0.0f32; 48_000 * 2];
            let mut s = synth.lock().unwrap();
            for block in buf.chunks_mut(512) {
                s.render(block, 1);
            }
            let peak = buf.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(buf.iter().all(|v| v.is_finite()), "{what} produced NaN or infinity");
            assert!(peak > 1e-4, "{what} is silent");
        };
        for sfx in all {
            a.play(sfx);
            render(&format!("{sfx:?}"));
        }
        for tier in 0..4 {
            a.play_brutal(tier);
            render(&format!("streak sting {tier}"));
        }
        for voice in [Speech::Child, Speech::Old, Speech::Beast, Speech::Narrator] {
            a.play_voice(voice);
            render(&format!("{voice:?}"));
        }
    }
}
