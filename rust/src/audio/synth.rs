//! A tiny software synthesizer that reproduces the web build's Web Audio graph:
//! oscillators and noise with exponential envelopes, biquad filters, a music
//! bus behind a warbling low-pass, an ambient threat drone and a crackle bed.
//! It runs inside the audio callback, so the music sequencer is sample-accurate.
//! The effects bus is stereo (each voice panned where its sound is) and goes through the mix
//! (`mix`): a voice budget, a duck and a soft limiter on the master.

use std::f64::consts::TAU;

use super::mix::{self, Limiter, Priority, Stamp};
use crate::logic::rng::Rng;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Wave {
    Sine,
    Square,
    Saw,
    Triangle,
    /// The shared noise buffer, read at `rate` from `offset` seconds in.
    Noise {
        rate: f64,
        offset: f64,
    },
    /// Clean white noise (no crackle), read the same way: breath, wind, whooshes (`voices`).
    Hiss {
        rate: f64,
        offset: f64,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bus {
    /// Effects, straight to the master.
    Sfx,
    /// Music voices, through the tape low-pass.
    Music,
}

/// Web Audio style automation: values at times, exponential ramps between them.
#[derive(Clone, Debug)]
pub struct Env {
    points: Vec<(f64, f64)>,
    seg: usize,
    value: f64,
    mul: f64,
    seg_end: f64,
}

impl Env {
    pub fn constant(v: f64) -> Self {
        Self::points(vec![(0.0, v)])
    }

    /// `(time, value)` pairs; times are absolute synth seconds, values must be positive.
    pub fn points(points: Vec<(f64, f64)>) -> Self {
        let value = points[0].1;
        Self { points, seg: 0, value, mul: 1.0, seg_end: f64::NEG_INFINITY }
    }

    /// setValueAtTime(v0, t) then exponentialRampToValueAtTime(v1, t + dur).
    pub fn ramp(t: f64, v0: f64, v1: f64, dur: f64) -> Self {
        Self::points(vec![(t, v0), (t + dur, v1)])
    }

    /// The value at `t`, stepping forward one sample of `dt`. Times only move forward.
    fn next(&mut self, t: f64, dt: f64) -> f64 {
        if t >= self.seg_end {
            // Find the segment that holds `t`.
            while self.seg + 1 < self.points.len() && t >= self.points[self.seg + 1].0 {
                self.seg += 1;
            }
            let (t0, v0) = self.points[self.seg];
            if self.seg + 1 < self.points.len() {
                let (t1, v1) = self.points[self.seg + 1];
                let span = (t1 - t0).max(1e-9);
                let into = ((t - t0) / span).clamp(0.0, 1.0);
                self.value = v0 * (v1 / v0).powf(into);
                self.mul = (v1 / v0).powf(dt / span);
                self.seg_end = t1;
            } else {
                self.value = v0;
                self.mul = 1.0;
                self.seg_end = f64::INFINITY;
            }
        }
        let v = self.value;
        self.value *= self.mul;
        v
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FilterKind {
    Lowpass,
    Highpass,
    Bandpass,
}

/// RBJ biquad with Web Audio's parameter conventions (Q in dB for low/high-pass).
#[derive(Clone, Copy, Debug)]
pub struct Biquad {
    kind: FilterKind,
    q: f64,
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Biquad {
    pub fn new(kind: FilterKind, freq: f64, q: f64, sample_rate: f64) -> Self {
        let mut f = Self { kind, q, b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0, x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 };
        f.set_freq(freq, sample_rate);
        f
    }

    pub fn set_freq(&mut self, freq: f64, sample_rate: f64) {
        let w0 = TAU * (freq / sample_rate).clamp(1e-5, 0.49);
        let (sin, cos) = w0.sin_cos();
        let alpha = match self.kind {
            FilterKind::Bandpass => sin / (2.0 * self.q.max(1e-4)),
            _ => sin / (2.0 * 10f64.powf(self.q / 20.0)),
        };
        let (b0, b1, b2) = match self.kind {
            FilterKind::Lowpass => ((1.0 - cos) / 2.0, 1.0 - cos, (1.0 - cos) / 2.0),
            FilterKind::Highpass => ((1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0),
            FilterKind::Bandpass => (alpha, 0.0, -alpha),
        };
        let a0 = 1.0 + alpha;
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = -2.0 * cos / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    pub fn process(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// One scheduled note or noise hit.
#[derive(Clone, Debug)]
pub struct Voice {
    pub start: f64,
    pub stop: f64,
    pub wave: Wave,
    pub freq: Env,
    /// Frequency multiplier from detune (cents), fixed per voice.
    pub detune: f64,
    /// (rate Hz, depth as a fraction of the frequency).
    pub vibrato: Option<(f64, f64)>,
    pub gain: Env,
    pub filter: Option<(FilterKind, f64, f64)>,
    pub bus: Bus,
    biquad: Option<Biquad>,
    phase: f64,
    started: bool,
    /// Its place in the mix (`mix::Stamp`): stereo gains, priority, group, immunity to ducks.
    pan: (f64, f64),
    prio: Priority,
    group: u32,
    immune: bool,
    /// A gain on top of its envelope: the stamp's, lifted by merged repeats.
    boost: f64,
    /// A voice, on the voices' volume (else an effect, on theirs).
    voice: bool,
    /// When it was stolen by the voice budget: it fades out over `mix::STEAL_FADE`.
    stolen: Option<f64>,
}

impl Voice {
    pub fn new(start: f64, stop: f64, wave: Wave, freq: Env, gain: Env, bus: Bus) -> Self {
        Self {
            start,
            stop,
            wave,
            freq,
            detune: 1.0,
            vibrato: None,
            gain,
            filter: None,
            bus,
            biquad: None,
            phase: 0.0,
            started: false,
            pan: (1.0, 1.0),
            prio: Priority::Rest,
            group: 0,
            immune: false,
            boost: 1.0,
            voice: false,
            stolen: None,
        }
    }

    pub fn detune_cents(mut self, cents: f64) -> Self {
        self.detune = 2f64.powf(cents / 1200.0);
        self
    }

    pub fn filtered(mut self, kind: FilterKind, freq: f64, q: f64) -> Self {
        self.filter = Some((kind, freq, q));
        self
    }

    pub fn vibrato(mut self, rate: f64, depth: f64) -> Self {
        self.vibrato = Some((rate, depth));
        self
    }
}

/// PolyBLEP correction for a naive discontinuity at phase 0.
fn blep(t: f64, dt: f64) -> f64 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

/// Everything the audio thread owns.
pub struct Synth {
    pub sample_rate: f64,
    /// Seconds of audio rendered so far; the clock every event is scheduled against.
    pub now: f64,
    voices: Vec<Voice>,
    noise: Vec<f32>,
    white: Vec<f32>,
    pub rng: Rng,

    pub master: f64,
    /// The player's volumes (UI.md §8), 0 to 1: the music (the tune, the recordings and the bed
    /// under them), the effects, and the voices (cries, the story's words).
    pub music_volume: f64,
    pub sfx_volume: f64,
    pub voice_volume: f64,
    music_level: f64,
    pub music_target: f64,
    tone: Biquad,
    crackle: Biquad,
    crackle_pos: usize,

    // Threat drone.
    drone_phase: f64,
    drone_gain: f64,
    pub drone_gain_target: f64,
    drone_freq: f64,
    pub drone_freq_target: f64,

    // Frenzy wail.
    pub frenzy: bool,
    /// Per-play variation applied by the effect helpers: pitch and gain multipliers.
    pub var_pitch: f64,
    pub var_gain: f64,
    /// 0..1 how close the danger is; the music layers in extra percussion as it rises.
    pub intensity: f64,
    /// 0..1 how hurt the hero is; the music muffles as it rises.
    pub danger_target: f64,
    danger: f64,
    frenzy_phase: f64,

    pub music: super::music::Sequencer,
    control_countdown: usize,

    /// A recorded soundtrack playing in place of the sequencer, and its fade.
    pub track: Option<super::track::Track>,
    pub track_level: f64,
    pub track_target: f64,
    /// A low-pass on the recording that closes as the hero gets hurt, per channel.
    track_lp: (f64, f64),
    track_lp_k: f64,

    /// What the effects pushed next carry into the mix (`mix::Stamp`); reset after each play.
    pub stamp: Stamp,
    /// Until when everything but the immune is ducked (`mix::DUCK_GAIN`), and the duck's level now.
    pub duck_until: f64,
    duck: f64,
    limiter: Limiter,
    /// The time scale's pitch on the effects (`s^0.25`), and its low-pass on the music: two
    /// one-pole stages per channel, wide open (`k = 1`) at full speed.
    pub sfx_rate: f64,
    pub slow_k: f64,
    slow: [f64; 4],
    dedupe: mix::Dedupe,
    groups: u32,
    /// The war-drum layer over the recordings, locked to their tempo.
    pub drum: super::wardrum::WarDrum,
}

/// The music bus low-pass: open enough to keep the upbeat track bright, still warm.
pub const MUSIC_LOWPASS: f64 = 6200.0;

impl Synth {
    pub fn new(sample_rate: f64, music_level: f64) -> Self {
        // (Under test the noise is the same every run, so the level checks are too.)
        let mut rng = if cfg!(test) { Rng::new(0x5eed) } else { Rng::from_time() };
        let noise = build_noise(sample_rate, &mut rng);
        let white = (0..(sample_rate * 2.0) as usize).map(|_| (rng.next() * 2.0 - 1.0) as f32).collect();
        Self {
            sample_rate,
            now: 0.0,
            voices: Vec::with_capacity(256),
            noise,
            white,
            rng,
            master: 0.5,
            music_volume: 1.0,
            sfx_volume: 1.0,
            voice_volume: 1.0,
            music_level,
            music_target: music_level,
            tone: Biquad::new(FilterKind::Lowpass, MUSIC_LOWPASS, 0.6, sample_rate),
            crackle: Biquad::new(FilterKind::Bandpass, 3200.0, 0.5, sample_rate),
            crackle_pos: 0,
            drone_phase: 0.0,
            drone_gain: 0.016,
            drone_gain_target: 0.016,
            drone_freq: 42.0,
            drone_freq_target: 42.0,
            frenzy: false,
            var_pitch: 1.0,
            var_gain: 1.0,
            intensity: 0.0,
            danger_target: 0.0,
            danger: 0.0,
            frenzy_phase: 0.0,
            music: super::music::Sequencer::new(),
            control_countdown: 0,
            track: None,
            track_level: 0.0,
            track_target: 0.0,
            track_lp: (0.0, 0.0),
            track_lp_k: 1.0,
            stamp: Stamp::default(),
            duck_until: f64::NEG_INFINITY,
            duck: 1.0,
            limiter: Limiter::new(sample_rate),
            sfx_rate: 1.0,
            slow_k: 1.0,
            slow: [0.0; 4],
            dedupe: mix::Dedupe::default(),
            groups: 0,
            drum: Default::default(),
        }
    }

    /// Schedules a voice. An effect voice carries the current stamp into the mix and must fit the
    /// budget (`mix::SFX_VOICES`): when it is full the least important voice is stolen (faded
    /// out), or, if everything sounding matters more, this one is dropped.
    pub fn push(&mut self, mut v: Voice) {
        if v.bus == Bus::Sfx {
            let st = self.stamp;
            v.pan = (st.pan_l, st.pan_r);
            v.prio = st.prio;
            v.group = st.group;
            v.immune = st.immune;
            v.boost = st.gain;
            v.voice = st.voice;
            let now = self.now;
            let live = self
                .voices
                .iter()
                .enumerate()
                .filter(|(_, o)| o.bus == Bus::Sfx && o.stolen.is_none() && o.stop > now)
                .map(|(i, o)| (i, o.prio, o.stop));
            match mix::steal(live, v.prio) {
                mix::Steal::Room => {}
                mix::Steal::Take(i) => {
                    let o = &mut self.voices[i];
                    o.stolen = Some(now.max(o.start));
                    o.stop = o.stop.min(now.max(o.start) + mix::STEAL_FADE);
                }
                mix::Steal::Refuse => return,
            }
        }
        if self.voices.len() < 512 {
            self.voices.push(v);
        }
    }

    /// Effect voices sounding or waiting to (not those stolen and fading).
    pub fn sfx_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.bus == Bus::Sfx && v.stolen.is_none() && v.stop > self.now).count()
    }

    /// Effect voices of a priority sounding or waiting to.
    #[cfg(test)]
    pub fn voices_at(&self, prio: Priority) -> usize {
        self.voices.iter().filter(|v| v.bus == Bus::Sfx && v.stolen.is_none() && v.stop > self.now && v.prio == prio).count()
    }

    /// Lifts every voice of a play by `k` (a merged repeat: `mix::Dedupe`), up to `mix::MERGE_CAP`.
    pub fn boost_group(&mut self, group: u32, k: f64) {
        for v in self.voices.iter_mut().filter(|v| v.group == group && group != 0) {
            v.boost = (v.boost * k).min(mix::MERGE_CAP);
        }
    }

    /// A fresh group for the next play's voices.
    pub fn next_group(&mut self) -> u32 {
        self.groups = self.groups.wrapping_add(1).max(1);
        self.groups
    }

    /// Whether a play of sound `key` is heard as itself; if it is one too many inside the dedupe
    /// window, the last one is lifted instead (merged, louder) and this one is not played.
    pub fn admit(&mut self, key: u64, group: u32) -> bool {
        match self.dedupe.admit(key, self.now, group) {
            mix::Admit::Play => true,
            mix::Admit::Merge { group } => {
                self.boost_group(group, mix::MERGE_BOOST);
                false
            }
        }
    }

    /// Lets every voice of `group` go: those sounding fade out over `secs` from where they are,
    /// those still waiting to start never do (the hymn, skipped).
    pub fn release_group(&mut self, group: u32, secs: f64) {
        let now = self.now;
        for v in self.voices.iter_mut().filter(|v| v.group == group && group != 0) {
            if v.start >= now {
                v.stop = v.start;
            } else if v.stop > now + secs {
                let level = v.gain.value.max(1e-5);
                v.gain = Env::points(vec![(now, level), (now + secs, level * 1e-3)]);
                v.stop = now + secs;
            }
        }
    }

    /// Ducks everything but the immune for `secs` from now.
    pub fn duck_for(&mut self, secs: f64) {
        self.duck_until = self.duck_until.max(self.now + secs);
    }

    /// The beat under the music now: (bpm, beats since the first downbeat). A recording counts
    /// from its own measured downbeat; the synth tune from its sequencer; with neither, a free
    /// clock at the base tempo.
    pub fn beat_clock(&self) -> (f64, f64) {
        if let Some(t) = self.track.as_ref().filter(|_| self.track_target > 0.0) {
            let bpm = t.which.bpm();
            return (bpm, (t.position() - t.which.downbeat()) * bpm / 60.0);
        }
        let seq = &self.music;
        if seq.running && seq.next_time >= 0.0 {
            let step_dur = 60.0 / seq.bpm.max(20.0) / 4.0;
            let steps = seq.step as f64 - (seq.next_time - self.now) / step_dur;
            return (seq.bpm, steps / 4.0);
        }
        let bpm = super::music::BASE_BPM;
        (bpm, self.now * bpm / 60.0)
    }

    /// Fills an interleaved buffer: the effects in stereo, each voice where its sound is; the
    /// synth tune mono; a recorded soundtrack in stereo.
    pub fn render(&mut self, out: &mut [f32], channels: usize) {
        let channels = channels.max(1);
        let dt = 1.0 / self.sample_rate;
        let sr = self.sample_rate;
        let drone_k = 1.0 - (-dt / 0.1).exp();
        let music_k = 1.0 - (-dt / (0.4 / 3.0)).exp();
        let track_k = 1.0 - (-dt / 0.35).exp();
        let duck_k = 1.0 - (-dt / 0.005).exp();
        let noise_len = self.noise.len();

        for frame in out.chunks_mut(channels) {
            if self.control_countdown == 0 {
                self.control_countdown = 32;
                self.control();
            }
            self.control_countdown -= 1;

            let t = self.now;
            // Effects: those a duck passes by, and the rest, per side.
            let (mut keep_l, mut keep_r, mut sfx_l, mut sfx_r) = (0.0, 0.0, 0.0, 0.0);
            let mut music = 0.0;
            let rate = self.sfx_rate;
            for v in self.voices.iter_mut() {
                if t < v.start || t >= v.stop {
                    continue;
                }
                if !v.started {
                    v.started = true;
                    v.biquad = v.filter.map(|(k, f, q)| Biquad::new(k, f, q, sr));
                    if let Wave::Noise { offset, .. } | Wave::Hiss { offset, .. } = v.wave {
                        v.phase = offset;
                    }
                }
                let sped = if v.bus == Bus::Sfx { rate } else { 1.0 };
                let g = v.gain.next(t, dt);
                let mut f = v.freq.next(t, dt) * v.detune * sped;
                if let Some((rate, depth)) = v.vibrato {
                    f *= 1.0 + depth * (TAU * rate * (t - v.start)).sin();
                }
                let inc = f * dt;
                let s = match v.wave {
                    Wave::Sine => (TAU * v.phase).sin(),
                    Wave::Square => {
                        let naive = if v.phase < 0.5 { 1.0 } else { -1.0 };
                        naive + blep(v.phase, inc) - blep((v.phase + 0.5).fract(), inc)
                    }
                    Wave::Saw => 2.0 * v.phase - 1.0 - blep(v.phase, inc),
                    Wave::Triangle => 1.0 - 4.0 * (v.phase - 0.5).abs(),
                    Wave::Noise { .. } => self.noise[(v.phase * sr) as usize % noise_len] as f64,
                    Wave::Hiss { .. } => self.white[(v.phase * sr) as usize % self.white.len()] as f64,
                };
                v.phase = match v.wave {
                    // Noise keeps its read position (seconds into the buffer) in the phase.
                    Wave::Noise { rate, .. } | Wave::Hiss { rate, .. } => v.phase + rate * sped * dt,
                    _ => (v.phase + inc).fract(),
                };
                let mut s = s * g;
                if let Some(b) = v.biquad.as_mut() {
                    s = b.process(s);
                }
                match v.bus {
                    Bus::Sfx => {
                        s *= v.boost * if v.voice { self.voice_volume } else { self.sfx_volume };
                        if let Some(at) = v.stolen {
                            s *= (1.0 - (t - at) / mix::STEAL_FADE).max(0.0);
                        }
                        if v.immune {
                            keep_l += s * v.pan.0;
                            keep_r += s * v.pan.1;
                        } else {
                            sfx_l += s * v.pan.0;
                            sfx_r += s * v.pan.1;
                        }
                    }
                    Bus::Music => music += s,
                }
            }

            // Music: tape low-pass over the voices, crackle bed joins after it.
            self.music_level += (self.music_target - self.music_level) * music_k;
            let crackle = self.crackle.process(self.noise[self.crackle_pos] as f64) * 0.02;
            self.crackle_pos = (self.crackle_pos + 1) % noise_len;
            let music_out = (self.tone.process(music) + crackle) * self.music_level;

            // Threat drone: barely there at rest, swelling as enemies close in.
            self.drone_gain += (self.drone_gain_target - self.drone_gain) * drone_k;
            self.drone_freq += (self.drone_freq_target - self.drone_freq) * drone_k;
            let f = self.drone_freq + 6.0 * (TAU * 0.15 * t).sin();
            self.drone_phase = (self.drone_phase + f * dt).fract();
            let drone = (TAU * self.drone_phase).sin() * self.drone_gain;

            let mut frenzy = 0.0;
            if self.frenzy {
                let f = 220.0 + 40.0 * (TAU * 7.0 * t).sin();
                self.frenzy_phase = (self.frenzy_phase + f * dt).fract();
                frenzy = (1.0 - 4.0 * (self.frenzy_phase - 0.5).abs()) * 0.08;
            }

            let (mut tl, mut tr) = (0.0, 0.0);
            self.track_level += (self.track_target - self.track_level) * track_k;
            if let Some(track) = self.track.as_mut() {
                let (l, r) = track.next(sr);
                let k = self.track_lp_k;
                self.track_lp.0 += (l as f64 - self.track_lp.0) * k;
                self.track_lp.1 += (r as f64 - self.track_lp.1) * k;
                let g = self.track_level * self.music_level * track.which.gain();
                tl = self.track_lp.0 * g;
                tr = self.track_lp.1 * g;
            }
            // Slowed time closes a low-pass over the music (wide open, so exact, at full speed).
            let k = self.slow_k;
            let sl = &mut self.slow;
            sl[0] += (music_out + tl - sl[0]) * k;
            sl[1] += (sl[0] - sl[1]) * k;
            sl[2] += (music_out + tr - sl[2]) * k;
            sl[3] += (sl[2] - sl[3]) * k;
            let mv = self.music_volume;
            let (music_l, music_r) = (sl[1] * mv, sl[3] * mv);
            // A parry or a struck hero ducks everything but its own voices.
            let duck_target = if t < self.duck_until { mix::DUCK_GAIN } else { 1.0 };
            self.duck += (duck_target - self.duck) * duck_k;
            let d = self.duck;
            let bed = (drone + frenzy) * mv;
            let (left, right) = self
                .limiter
                .process((keep_l + d * (sfx_l + music_l) + bed) * self.master, (keep_r + d * (sfx_r + music_r) + bed) * self.master);
            let (left, right) = (left as f32, right as f32);
            match frame.len() {
                1 => frame[0] = (left + right) * 0.5,
                _ => {
                    for (c, s) in frame.iter_mut().enumerate() {
                        *s = match c {
                            0 => left,
                            1 => right,
                            _ => (left + right) * 0.5,
                        };
                    }
                }
            }
            self.now += dt;
        }
        let now = self.now;
        self.voices.retain(|v| v.stop > now);
        if self.track_target == 0.0 && self.track_level < 1e-4 {
            self.track = None;
        }
    }

    /// Control-rate work: the tape warble and the music sequencer.
    fn control(&mut self) {
        // Low health pulls the tape filter down, as if the music were heard underwater.
        self.danger += (self.danger_target - self.danger) * 0.004;
        let wobble = (TAU * 0.23 * self.now).sin() * 180.0 * (1.0 - 0.5 * self.danger);
        let cutoff = MUSIC_LOWPASS + (1250.0 - MUSIC_LOWPASS) * self.danger;
        self.tone.set_freq(cutoff + wobble, self.sample_rate);
        // The recording stays clean until he is hurt, then muffles the same way.
        let track_cutoff = 20_000.0 + (1250.0 - 20_000.0) * self.danger.powf(0.7);
        self.track_lp_k = 1.0 - (-TAU * track_cutoff / self.sample_rate).exp();
        let mut seq = std::mem::take(&mut self.music);
        seq.pump(self);
        self.music = seq;
        let mut drum = std::mem::take(&mut self.drum);
        drum.pump(self);
        self.drum = drum;
    }
}

/**
 * Three seconds of near-silence with scattered pops and a whisper of hiss,
 * looped forever. Real crackle is sparse impulses, not continuous noise.
 */
fn build_noise(sample_rate: f64, rng: &mut Rng) -> Vec<f32> {
    let len = (sample_rate * 3.0) as usize;
    let mut d: Vec<f32> = (0..len).map(|_| ((rng.next() * 2.0 - 1.0) * 0.06) as f32).collect();
    for _ in 0..420 {
        let at = (rng.next() * (len - 40) as f64) as usize;
        let amp = 0.25 + rng.next() * 0.6;
        for i in 0..24 {
            d[at + i] += (amp * (rng.next() * 2.0 - 1.0) * (1.0 - i as f64 / 24.0)) as f32;
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exponential_envelope_hits_its_targets() {
        let mut e = Env::ramp(0.0, 1.0, 0.001, 1.0);
        let dt = 1.0 / 1000.0;
        let mut t = 0.0;
        let mut last = 0.0;
        for _ in 0..1000 {
            last = e.next(t, dt);
            t += dt;
        }
        assert!((last - 0.001).abs() < 0.0002, "{last}");
    }

    #[test]
    fn renders_finite_audio() {
        let mut s = Synth::new(48_000.0, 0.42);
        s.push(Voice::new(0.0, 0.2, Wave::Square, Env::constant(440.0), Env::ramp(0.0, 0.2, 0.001, 0.2), Bus::Sfx));
        let mut buf = vec![0.0f32; 48_000];
        s.render(&mut buf, 2);
        assert!(buf.iter().all(|v| v.is_finite()));
        assert!(buf.iter().any(|v| v.abs() > 0.01));
    }
}
