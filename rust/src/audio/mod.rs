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
}

impl Default for AudioManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioManager {
    pub fn new() -> Self {
        let muted = platform::load("vh.muted").map(|v| v == "1").unwrap_or(false);
        Self { synth: None, stream: None, pickup_pitch: 0, muted, ducked: false, tempo_step: 0 }
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

    /// 0..1 how close the nearest enemy is; raises the ambient pulse.
    pub fn set_threat(&mut self, level: f64) {
        self.with(|s| {
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

    pub fn play(&mut self, sfx: Sfx) {
        if sfx == Sfx::Nodule {
            self.pickup_pitch = (self.pickup_pitch + 1) % 2;
        }
        let pitch = self.pickup_pitch as f64;
        self.with(|s| {
            let t = s.now;
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
                Sfx::Slash => sweep(s, t, 1400.0, 500.0, 0.12, Wave::Saw, 0.12),
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
            }
        });
    }
}

fn blip(s: &mut Synth, t: f64, freq: f64, dur: f64, wave: Wave, vol: f64) {
    s.push(Voice::new(t, t + dur + 0.02, wave, Env::constant(freq), Env::ramp(t, vol, 0.001, dur), Bus::Sfx));
}

fn sweep(s: &mut Synth, t: f64, from: f64, to: f64, dur: f64, wave: Wave, vol: f64) {
    s.push(Voice::new(t, t + dur + 0.02, wave, Env::ramp(t, from, to, dur), Env::ramp(t, vol, 0.001, dur), Bus::Sfx));
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
