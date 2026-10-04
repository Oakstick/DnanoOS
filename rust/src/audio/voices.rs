//! The voices of the bestiary (WARBAND.md §5) and the layered hits (§4.6).
//!
//! **The archetype is a family, the breed is a voice.** Every tell begins with its family's
//! signature, so the ear learns the three ways of attack before the faces: the melee brute's low
//! brass swell and the scrape of a weapon rising ("wait"), the body skirmisher's sharp intake of
//! breath and rising snarl ("time it"), the distance hexer's shimmer that peaks as it lets go
//! ("close in now"). Over it goes the breed's own timbre: the draugr's rattling exhale, the
//! raven's caw, the oni's drum-chant grunt, the karakoncolos calling the hero by his own name.
//!
//! Everything is a `Recipe`: a short list of `Layer`s (a wave, a pitch glide, an envelope, a
//! filter, vibrato) that `render` turns into synth voices, so a new voice costs data, not samples.
//! Each recipe has three round-robin variants (pitch, length and noise grain), never the same twice
//! running (`mix::RoundRobin`), and pitch follows size: the giants speak an octave below.
//!
//! Hits are built the same way from up to five layers: a transient, a body by the struck one's
//! matter, a tail (the room), a sweetener and a voice (§4.6's table).

use super::mix::{self, Listener, Priority, Stamp};
use super::synth::{Bus, Env, FilterKind, Synth, Voice, Wave};
use super::wardrum::DrumLevel;
use super::{AudioManager, Sfx};
use crate::logic::combat::{Attack, Breed, MoveId};
use crate::logic::enemy::EnemyKind;
use crate::logic::heroes::HeroKind;
use crate::logic::types::V;

// ------------------------------------------------------------------------------------ recipes

/// A layer's sound source.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tone {
    Sine,
    Square,
    Saw,
    Triangle,
    /// Clean white noise read at a rate (breath, wind, whooshes).
    Hiss(f32),
    /// The crackle buffer read at a rate (fire, grit, gravel).
    Crackle(f32),
}

/// What a layer is for: the family's signature on a tell, the breed's own timbre, or one of a
/// hit's five layers.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Part {
    Signature(Attack),
    Breed,
    Transient,
    Body,
    Tail,
    Sweetener,
    Voice,
}

/// One synth voice of a recipe. Times in seconds from the recipe's start; frequencies in Hz.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Layer {
    pub part: Part,
    pub at: f32,
    pub dur: f32,
    pub tone: Tone,
    /// Pitch at the start and the end, an exponential glide between (oscillators only).
    pub f0: f32,
    pub f1: f32,
    pub peak: f32,
    /// Seconds to the peak (0: struck), then the share of the rest held near it before it decays.
    pub attack: f32,
    pub hold: f32,
    pub filter: Option<(FilterKind, f32, f32)>,
    /// (rate Hz, depth as a share of the pitch); (0, 0) for none.
    pub vibrato: (f32, f32),
    pub cents: f32,
}

const BASE: Layer = Layer {
    part: Part::Breed,
    at: 0.0,
    dur: 0.2,
    tone: Tone::Sine,
    f0: 440.0,
    f1: 440.0,
    peak: 0.05,
    attack: 0.0,
    hold: 0.0,
    filter: None,
    vibrato: (0.0, 0.0),
    cents: 0.0,
};

fn sine(f0: f32, f1: f32) -> Layer {
    Layer { tone: Tone::Sine, f0, f1, ..BASE }
}
fn tri(f0: f32, f1: f32) -> Layer {
    Layer { tone: Tone::Triangle, f0, f1, ..BASE }
}
fn saw(f0: f32, f1: f32) -> Layer {
    Layer { tone: Tone::Saw, f0, f1, ..BASE }
}
fn square(f0: f32, f1: f32) -> Layer {
    Layer { tone: Tone::Square, f0, f1, ..BASE }
}
fn hiss(rate: f32) -> Layer {
    Layer { tone: Tone::Hiss(rate), ..BASE }
}
fn crackle(rate: f32) -> Layer {
    Layer { tone: Tone::Crackle(rate), ..BASE }
}

impl Layer {
    fn at(mut self, t: f32) -> Self {
        self.at = t;
        self
    }
    fn dur(mut self, d: f32) -> Self {
        self.dur = d;
        self
    }
    fn vol(mut self, v: f32) -> Self {
        self.peak = v;
        self
    }
    fn swell(mut self, a: f32) -> Self {
        self.attack = a;
        self
    }
    fn hold(mut self, h: f32) -> Self {
        self.hold = h;
        self
    }
    fn lp(mut self, f: f32, q: f32) -> Self {
        self.filter = Some((FilterKind::Lowpass, f, q));
        self
    }
    fn hp(mut self, f: f32, q: f32) -> Self {
        self.filter = Some((FilterKind::Highpass, f, q));
        self
    }
    fn bp(mut self, f: f32, q: f32) -> Self {
        self.filter = Some((FilterKind::Bandpass, f, q));
        self
    }
    fn vib(mut self, rate: f32, depth: f32) -> Self {
        self.vibrato = (rate, depth);
        self
    }
    fn cents(mut self, c: f32) -> Self {
        self.cents = c;
        self
    }
    fn part(mut self, p: Part) -> Self {
        self.part = p;
        self
    }
}

/// A sound as data: its layers, in order (a tell's family signature first).
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Recipe {
    pub layers: Vec<Layer>,
}

impl Recipe {
    pub fn of(parts: impl IntoIterator<Item = Vec<Layer>>) -> Self {
        Self { layers: parts.into_iter().flatten().collect() }
    }

    /// Seconds from its start to the end of its last layer.
    pub fn duration(&self) -> f32 {
        self.layers.iter().map(|l| l.at + l.dur).fold(0.0, f32::max)
    }
}

fn tagged(layers: Vec<Layer>, p: Part) -> Vec<Layer> {
    layers.into_iter().map(|l| l.part(p)).collect()
}

/// Layers stretched in time, moved in pitch (filters and noise with them) and scaled in gain.
fn shaped(layers: &[Layer], time: f32, pitch: f32, gain: f32) -> Vec<Layer> {
    layers
        .iter()
        .map(|l| {
            let mut l = *l;
            l.at *= time;
            l.dur *= time;
            l.attack *= time;
            l.f0 *= pitch;
            l.f1 *= pitch;
            l.tone = match l.tone {
                Tone::Hiss(r) => Tone::Hiss(r * pitch),
                Tone::Crackle(r) => Tone::Crackle(r * pitch),
                t => t,
            };
            l.filter = l.filter.map(|(k, f, q)| (k, f * pitch, q));
            l.peak *= gain;
            l
        })
        .collect()
}

fn delayed(layers: Vec<Layer>, dt: f32) -> Vec<Layer> {
    layers.into_iter().map(|l| l.at(l.at + dt)).collect()
}

/// How a recipe is played: its pitch (the family signature's apart: it follows size only half as
/// far, so the family stays recognisable in a giant's mouth), its length, and which variant.
#[derive(Clone, Copy, Debug)]
pub struct Render {
    pub pitch: f64,
    pub sig_pitch: f64,
    pub stretch: f64,
    pub variant: u8,
}

impl Default for Render {
    fn default() -> Self {
        Self { pitch: 1.0, sig_pitch: 1.0, stretch: 1.0, variant: 0 }
    }
}

/// The three round-robin variants: a little lower and longer, a little higher and shorter.
const VARIANT_PITCH: [f64; 3] = [1.0, 0.957, 1.046];
const VARIANT_TIME: [f64; 3] = [1.0, 1.045, 0.962];

/// Schedules a recipe's voices from `t0` on the synth (the caller sets the stamp).
pub fn render(s: &mut Synth, r: &Recipe, t0: f64, how: Render) {
    let v = (how.variant % mix::VARIANTS) as usize;
    for (i, l) in r.layers.iter().enumerate() {
        let p = VARIANT_PITCH[v] * if matches!(l.part, Part::Signature(_)) { how.sig_pitch } else { how.pitch };
        let st = how.stretch * VARIANT_TIME[v];
        // The variants lean on different layers too.
        let tilt = if v == 0 { 1.0 } else { 1.0 + 0.12 * (i as f64 * 1.7 + v as f64 * 2.1).sin() };
        let start = t0 + l.at as f64 * st;
        let dur = (l.dur as f64 * st).max(0.004);
        let end = start + dur;
        let peak = (l.peak as f64 * tilt).max(1e-5);
        let a = (l.attack as f64 * st).min(dur * 0.97);
        let mut pts = Vec::with_capacity(4);
        if a < 0.0015 {
            pts.push((start, peak));
        } else {
            pts.push((start, peak * 0.003));
            pts.push((start + a, peak));
        }
        if l.hold > 0.0 {
            let h = start + a + (dur - a) * l.hold as f64;
            if h > start + a + 1e-4 && h < end - 1e-4 {
                pts.push((h, peak * 0.85));
            }
        }
        pts.push((end, peak * 0.01));
        let offset = (v as f64 * 0.71 + i as f64 * 0.137 + 0.05) % 1.7;
        let (wave, noise) = match l.tone {
            Tone::Sine => (Wave::Sine, false),
            Tone::Square => (Wave::Square, false),
            Tone::Saw => (Wave::Saw, false),
            Tone::Triangle => (Wave::Triangle, false),
            Tone::Hiss(rate) => (Wave::Hiss { rate: rate as f64 * p, offset }, true),
            Tone::Crackle(rate) => (Wave::Noise { rate: rate as f64 * p, offset }, true),
        };
        let freq = if noise {
            Env::constant(1.0)
        } else if (l.f0 - l.f1).abs() < 1e-3 {
            Env::constant(l.f0 as f64 * p)
        } else {
            Env::points(vec![(start, l.f0 as f64 * p), (end, l.f1 as f64 * p)])
        };
        let mut voice = Voice::new(start, end + 0.01, wave, freq, Env::points(pts), Bus::Sfx);
        if let Some((k, f, q)) = l.filter {
            voice = voice.filtered(k, f as f64 * p, q as f64);
        }
        if l.vibrato.0 > 0.0 {
            voice = voice.vibrato(l.vibrato.0 as f64, l.vibrato.1 as f64);
        }
        if l.cents != 0.0 {
            voice = voice.detune_cents(l.cents as f64);
        }
        s.push(voice);
    }
}

// ------------------------------------------------------------------------- the families

/// The tell's nominal release (the family signature peaks here); a tell is stretched to its move's
/// real startup.
pub const NOMINAL_RELEASE: f64 = 0.5;

/// The family's signature, first on every tell of its breeds.
pub fn signature(family: Attack) -> Vec<Layer> {
    let layers = match family {
        // Low brass swelling, and a metal scrape as the weapon rises.
        Attack::Melee => vec![
            saw(73.4, 82.4).dur(0.56).vol(0.16).swell(0.38).hold(0.2).lp(760.0, 1.5).vib(5.0, 0.006),
            saw(110.0, 123.5).dur(0.56).vol(0.09).swell(0.38).hold(0.2).lp(980.0, 1.5).cents(7.0).vib(5.6, 0.006),
            hiss(1.0).at(0.1).dur(0.42).vol(0.5).swell(0.34).bp(3400.0, 7.0),
            sine(2500.0, 3300.0).at(0.12).dur(0.38).vol(0.012).swell(0.32).vib(31.0, 0.012),
        ],
        // A sharp intake of breath, cut off, then a snarl rising.
        Attack::Body => vec![
            hiss(1.3).dur(0.2).vol(0.55).swell(0.19).bp(2300.0, 0.9),
            saw(118.0, 205.0).at(0.17).dur(0.33).vol(0.18).swell(0.05).hold(0.3).bp(950.0, 1.4).vib(23.0, 0.06),
            hiss(0.8).at(0.17).dur(0.3).vol(0.45).swell(0.05).bp(560.0, 1.6),
        ],
        // A shimmer rising, peaking as it lets go.
        Attack::Distance => vec![
            sine(1200.0, 2400.0).dur(0.56).vol(0.045).swell(0.5).vib(9.0, 0.015),
            sine(1800.0, 3600.0).dur(0.56).vol(0.028).swell(0.5).cents(12.0).vib(11.0, 0.02),
            tri(600.0, 1200.0).dur(0.56).vol(0.04).swell(0.5),
            hiss(2.0).dur(0.56).vol(0.07).swell(0.5).hp(5200.0, 0.0),
        ],
    };
    tagged(layers, Part::Signature(family))
}

/// How a breed's blows reach the hero: its family.
pub fn family(kind: EnemyKind) -> Attack {
    Breed::of(kind).attack()
}

// -------------------------------------------------------------------------- the timbres

fn rattle(at: f32, n: usize, gap: f32, freq: f32, vol: f32) -> Vec<Layer> {
    (0..n)
        .map(|i| {
            let k = i as f32;
            hiss(1.4)
                .at(at + k * gap + (k * 0.37).sin().abs() * gap * 0.3)
                .dur(0.03)
                .vol(vol * (1.0 - 0.5 * k / n as f32))
                .bp(freq * (1.0 + 0.08 * (k * 1.9).sin()), 5.0)
        })
        .collect()
}

fn caw(at: f32, f: f32, n: usize, gap: f32, vol: f32) -> Vec<Layer> {
    (0..n)
        .flat_map(|i| {
            let t = at + i as f32 * gap;
            [
                saw(f, f * 0.69).at(t).dur(0.2).vol(vol).swell(0.02).hold(0.2).bp(1400.0, 3.0).vib(36.0, 0.04),
                hiss(1.3).at(t).dur(0.16).vol(vol * 2.5).bp(1900.0, 2.0),
            ]
        })
        .collect()
}

fn bubbles(at: f32, f: f32, times: &[f32], vol: f32) -> Vec<Layer> {
    times.iter().enumerate().map(|(i, t)| sine(f * (1.0 + 0.15 * i as f32), f * 2.3).at(at + t).dur(0.05).vol(vol)).collect()
}

fn cackle(at: f32, f: f32, n: usize, gap: f32, rise: f32, vol: f32) -> Vec<Layer> {
    (0..n)
        .flat_map(|i| {
            let t = at + i as f32 * gap;
            let p = f * rise.powi(i as i32);
            [
                saw(p, p * 0.94).at(t).dur(0.07).vol(vol).swell(0.008).bp(1100.0, 2.5),
                hiss(1.2).at(t).dur(0.03).vol(vol * 2.0).bp(1800.0, 2.0),
            ]
        })
        .collect()
}

fn whoosh(at: f32, f: f32, dur: f32, vol: f32) -> Vec<Layer> {
    vec![
        hiss(1.0).at(at).dur(dur).vol(vol).swell(dur * 0.45).bp(f, 1.3),
        sine(f * 0.06, f * 0.035).at(at).dur(dur).vol(vol * 0.08).swell(dur * 0.4),
    ]
}

fn growl(at: f32, f: f32, dur: f32, vol: f32) -> Vec<Layer> {
    vec![
        saw(f, f * 0.95).at(at).dur(dur).vol(vol).swell(0.06).hold(0.5).lp(f * 7.0, 2.0).vib(27.0, 0.08),
        hiss(0.7).at(at).dur(dur).vol(vol * 3.0).swell(0.08).hold(0.4).bp(f * 4.0, 1.3),
    ]
}

fn bell(at: f32, f: f32, dur: f32, vol: f32) -> Vec<Layer> {
    vec![
        sine(f, f).at(at).dur(dur).vol(vol),
        sine(f * 2.76, f * 2.76).at(at).dur(dur * 0.66).vol(vol * 0.5),
        sine(f * 5.4, f * 5.4).at(at).dur(dur * 0.4).vol(vol * 0.25),
    ]
}

fn coins(at: f32, n: usize, spread: f32, vol: f32) -> Vec<Layer> {
    (0..n)
        .flat_map(|i| {
            let k = i as f32;
            let t = at + spread * (k / n as f32 + 0.04 * (k * 2.3).sin());
            let f = 2000.0 + 900.0 * (k * 1.37).sin().abs();
            [sine(f, f).at(t).dur(0.07).vol(vol), sine(f * 2.76, f * 2.76).at(t).dur(0.035).vol(vol * 0.4)]
        })
        .collect()
}

fn snap(at: f32, f: f32, vol: f32) -> Vec<Layer> {
    vec![
        hiss(1.0).at(at).dur(0.02).vol(vol * 0.7).hp(2500.0, 0.0),
        sine(f, f * 0.5).at(at).dur(0.06).vol(vol * 0.2),
        hiss(0.8).at(at).dur(0.05).vol(vol * 0.6).lp(600.0, 1.0),
    ]
}

fn crack(at: f32, vol: f32) -> Vec<Layer> {
    vec![hiss(1.0).at(at).dur(0.035).vol(vol).bp(1800.0, 1.0), saw(400.0, 150.0).at(at).dur(0.03).vol(vol * 0.12).lp(3000.0, 1.0)]
}

/// Water-like or flesh-like squelch.
fn squelch(at: f32, vol: f32) -> Vec<Layer> {
    vec![sine(300.0, 100.0).at(at).dur(0.14).vol(vol * 0.4), hiss(0.9).at(at).dur(0.12).vol(vol).swell(0.01).lp(600.0, 1.0)]
}

/// The vowels of the speaking voices, by their first two formants.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Vowel {
    A,
    E,
    I,
    O,
    U,
}

impl Vowel {
    fn formants(self) -> (f32, f32) {
        match self {
            Vowel::A => (730.0, 1090.0),
            Vowel::E => (530.0, 1840.0),
            Vowel::I => (300.0, 2290.0),
            Vowel::O => (570.0, 840.0),
            Vowel::U => (300.0, 870.0),
        }
    }
}

/// The consonant a syllable opens with.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Onset {
    None,
    R,
    N,
    M,
    K,
    T,
    D,
    Y,
    H,
    B,
    V,
}

/// One spoken syllable of the `Speech` voices grown up: a buzzing saw through the vowel's two
/// formants, a breath through the second (cold and whispered when `breath` is high), and its
/// consonant.
fn syllable(at: f32, f: f32, onset: Onset, v: Vowel, dur: f32, vol: f32, breath: f32) -> Vec<Layer> {
    let (f1, f2) = v.formants();
    let mut out = vec![
        saw(f, f * 0.93).at(at).dur(dur).vol(vol).swell(0.035).hold(0.55).bp(f1, 4.0).vib(5.0, 0.01),
        saw(f, f * 0.93).at(at).dur(dur).vol(vol * 0.6).swell(0.035).hold(0.55).bp(f2, 6.0).vib(5.0, 0.01),
        saw(f, f * 0.93).at(at).dur(dur).vol(vol * 0.12).swell(0.035).hold(0.55).lp(f * 2.5, 1.0),
    ];
    if breath > 0.0 {
        out.push(hiss(1.0).at(at).dur(dur).vol(breath).swell(0.05).hold(0.5).bp(f2, 3.0));
    }
    let c = match onset {
        Onset::None => return out,
        Onset::R => {
            for k in 0..3 {
                out.push(hiss(1.2).at(at - 0.065 + k as f32 * 0.021).dur(0.014).vol(vol * 1.6).bp(1500.0, 2.0));
            }
            return out;
        }
        Onset::N => sine(f, f).at(at - 0.07).dur(0.08).vol(vol * 0.3).swell(0.02).lp(450.0, 1.0),
        Onset::M => sine(f, f).at(at - 0.07).dur(0.08).vol(vol * 0.3).swell(0.02).lp(330.0, 1.0),
        Onset::K => hiss(1.0).at(at - 0.025).dur(0.025).vol(vol * 2.0).bp(1900.0, 2.0),
        Onset::T => hiss(1.0).at(at - 0.016).dur(0.016).vol(vol * 1.4).hp(3500.0, 0.0),
        Onset::D => hiss(1.0).at(at - 0.016).dur(0.016).vol(vol * 1.6).hp(2200.0, 0.0),
        Onset::Y => saw(f, f).at(at - 0.05).dur(0.06).vol(vol * 0.5).swell(0.02).bp(2300.0, 5.0),
        Onset::H => hiss(1.0).at(at - 0.06).dur(0.065).vol(vol * 1.2).swell(0.03).bp(1300.0, 1.0),
        Onset::B => sine(f * 1.2, f).at(at - 0.04).dur(0.045).vol(vol * 0.3).lp(300.0, 1.0),
        Onset::V => hiss(1.0).at(at - 0.05).dur(0.05).vol(vol * 0.8).swell(0.02).lp(2000.0, 1.0),
    };
    out.push(c);
    out
}

/// Spoken words: (onset, vowel, length, pitch) per syllable, from `at`, in a voice at `f`.
/// `cold` whispers it and doubles it an octave down, as the Beast voice growls.
fn speak(at: f32, f: f32, words: &[(Onset, Vowel, f32, f32)], vol: f32, cold: bool) -> Vec<Layer> {
    let mut out = Vec::new();
    let mut t = at;
    for &(o, v, d, p) in words {
        out.extend(syllable(t, f * p, o, v, d, vol, if cold { vol * 0.9 } else { vol * 0.15 }));
        if cold {
            out.push(saw(f * p * 0.5, f * p * 0.465).at(t).dur(d).vol(vol * 0.35).swell(0.04).hold(0.5).lp(380.0, 1.0));
        }
        t += d + 0.03;
    }
    out
}

/// The karakoncolos's tell: the hero's own name, called in a cold voice.
pub fn name_call(hero: HeroKind) -> Vec<Layer> {
    use Onset as O;
    use Vowel as W;
    let words: &[(Onset, Vowel, f32, f32)] = match hero {
        HeroKind::Ragnar => &[(O::R, W::A, 0.24, 1.0), (O::N, W::A, 0.4, 0.87)],
        HeroKind::Mehmed => &[(O::M, W::E, 0.22, 1.0), (O::M, W::E, 0.38, 0.87), (O::D, W::E, 0.0, 0.8)],
        HeroKind::Miyamoto => &[(O::M, W::I, 0.12, 1.0), (O::Y, W::A, 0.13, 0.97), (O::M, W::O, 0.14, 0.93), (O::T, W::O, 0.34, 0.85)],
    };
    speak(0.12, 98.0, words, 0.38, true).into_iter().filter(|l| l.dur > 0.0).collect()
}

// ------------------------------------------------------------------- the breeds' timbres

/// Every body in the game, for the tests and the audio sheet.
pub const ALL_KINDS: &[EnemyKind] = &{
    use EnemyKind::*;
    [
        Skeleton,
        Bat,
        Vampire,
        Slime,
        Witch,
        Minion,
        Helmeted,
        Werewolf,
        Blemmyes,
        LargeBat,
        Tengu,
        Rasetsu,
        DireWolf,
        Jorogumo,
        Cobra,
        Kappa,
        Rokurokubi,
        Kitsune,
        Oni,
        Tulukus,
        Albasti,
        Cin,
        Arcura,
        Karakoncolos,
        Tepegoz,
        Gashadokuro,
        Ejder,
        Goryo,
        FrostGiant,
        Surtr,
        Nidhogg,
        Hel,
        Peerless,
        Ganglot,
        Gaki,
        Ellepige,
        Mammon,
        GoldGolem,
        Lucifer,
        Reflection,
        Leviathan,
        Serpentling,
        Amon,
        Raven,
        Belphegor,
        Nightmare,
        Beelzebub,
        FlySwarm,
        Asmodeus,
    ]
};

/// The six princes' and their adds' voices until their own are made: a kin's, stretched in time,
/// moved in pitch and scaled in gain (`shaped`). `None` for every other kind.
fn kin_voice(kind: EnemyKind) -> Option<(EnemyKind, f32, f32, f32)> {
    use EnemyKind as K;
    Some(match kind {
        // Pride's court: the Peerless's voice, deeper for the prince, brighter for his reflections.
        K::Lucifer => (K::Peerless, 1.2, 0.8, 1.0),
        K::Reflection => (K::Peerless, 0.8, 1.3, 0.9),
        // Envy's court: the root-dragon's, and the cobra's.
        K::Leviathan => (K::Nidhogg, 1.0, 0.85, 1.0),
        K::Serpentling => (K::Cobra, 0.9, 1.2, 0.9),
        // Wrath's court: the fire giant's, and the dark raven's.
        K::Amon => (K::Surtr, 1.0, 1.1, 1.0),
        K::Raven => (K::Vampire, 1.0, 0.9, 1.0),
        // Sloth's court: Hel's servants', slowed; the night hag's.
        K::Belphegor => (K::Ganglot, 1.2, 0.8, 1.0),
        K::Nightmare => (K::Albasti, 1.0, 1.15, 0.9),
        // Gluttony's court: the gaki's, deep for the prince, thin for his swarms.
        K::Beelzebub => (K::Gaki, 1.0, 0.7, 1.0),
        K::FlySwarm => (K::Gaki, 0.8, 1.4, 0.9),
        _ => return None,
    })
}

/// How big a body sounds: pitch goes down as it grows (a giant, 2, speaks an octave below).
pub fn size_of(kind: EnemyKind) -> f32 {
    use EnemyKind as K;
    match kind {
        K::Minion => 0.6,
        K::Bat => 0.7,
        K::Vampire => 0.8,
        K::LargeBat | K::Kitsune => 0.85,
        K::Slime | K::Gaki | K::Ellepige => 0.9,
        K::Cobra | K::Kappa => 0.95,
        K::Skeleton
        | K::Helmeted
        | K::Witch
        | K::Werewolf
        | K::Tengu
        | K::Rokurokubi
        | K::Tulukus
        | K::Albasti
        | K::Cin
        | K::Goryo
        | K::Peerless => 1.0,
        K::Jorogumo | K::Arcura => 1.05,
        K::Hel | K::Ganglot => 1.1,
        K::Rasetsu | K::DireWolf => 1.2,
        K::Blemmyes => 1.25,
        K::Karakoncolos => 1.35,
        K::Oni | K::GoldGolem => 1.4,
        K::Ejder | K::Nidhogg => 1.8,
        K::Tepegoz | K::Gashadokuro | K::FrostGiant | K::Surtr | K::Mammon => 2.0,
        K::Lucifer | K::Leviathan | K::Amon | K::Belphegor | K::Beelzebub | K::Asmodeus => 2.0,
        K::Reflection | K::Nightmare => 1.0,
        K::Serpentling | K::FlySwarm => 0.9,
        K::Raven => 0.8,
    }
}

/// The breed's own tell, over its family's signature (§5's table).
fn breed_tell(kind: EnemyKind, mv: MoveId, hero: HeroKind) -> Vec<Layer> {
    use EnemyKind as K;
    let mut out = match kind {
        // ---- stage 1
        // The draugr: a rattling exhale, grave-cold.
        K::Skeleton => [
            vec![
                hiss(0.7).at(0.05).dur(0.55).vol(0.5).swell(0.2).hold(0.3).bp(700.0, 1.2),
                sine(180.0, 150.0).at(0.05).dur(0.5).vol(0.02).swell(0.2).vib(4.0, 0.02),
            ],
            rattle(0.12, 6, 0.045, 2100.0, 0.7),
        ]
        .concat(),
        // The raven: a harsh caw, twice.
        K::Bat => caw(0.08, 780.0, 2, 0.3, 0.2),
        // The night raven: lower, and doubled.
        K::Vampire => [caw(0.08, 600.0, 2, 0.24, 0.17), caw(0.1, 505.0, 2, 0.24, 0.14)].concat(),
        // The bog slime: a bubbling gurgle.
        K::Slime => [
            bubbles(0.05, 180.0, &[0.0, 0.06, 0.09, 0.17, 0.22, 0.3], 0.1),
            vec![hiss(0.6).at(0.03).dur(0.42).vol(0.5).swell(0.15).lp(400.0, 1.0)],
        ]
        .concat(),
        // ---- stage 2
        // The witch: a cackle rising into the shimmer.
        K::Witch => [cackle(0.04, 380.0, 5, 0.085, 1.07, 0.3), vec![sine(1600.0, 3000.0).at(0.3).dur(0.3).vol(0.025).swell(0.25)]].concat(),
        // Her minion: a squeak.
        K::Minion => vec![
            sine(1300.0, 1900.0).at(0.1).dur(0.09).vol(0.16).vib(18.0, 0.04),
            sine(1500.0, 2200.0).at(0.22).dur(0.1).vol(0.16).vib(18.0, 0.04),
        ],
        // The helmeted skeleton: its helmet rattles, a sword is drawn.
        K::Helmeted => [
            rattle(0.04, 5, 0.035, 2800.0, 0.9),
            vec![
                sine(1250.0, 1250.0).at(0.06).dur(0.12).vol(0.05),
                sine(3450.0, 3450.0).at(0.06).dur(0.08).vol(0.025),
                hiss(1.0).at(0.18).dur(0.3).vol(1.0).swell(0.28).bp(4800.0, 9.0),
                sine(3150.0, 3150.0).at(0.42).dur(0.4).vol(0.025),
            ],
        ]
        .concat(),
        // The loup-garou: a growl into a howling breath.
        K::Werewolf => [
            growl(0.05, 85.0, 0.3, 0.12),
            vec![
                tri(340.0, 520.0).at(0.3).dur(0.3).vol(0.05).swell(0.15).vib(6.0, 0.03),
                hiss(1.0).at(0.3).dur(0.3).vol(0.3).swell(0.15).bp(900.0, 1.5),
            ],
        ]
        .concat(),
        // The blemmyes: a bellow from the mouth in its chest.
        K::Blemmyes => vec![
            saw(92.0, 80.0).at(0.05).dur(0.55).vol(0.35).swell(0.12).hold(0.4).bp(300.0, 2.2),
            saw(92.9, 80.6).at(0.05).dur(0.55).vol(0.08).swell(0.12).hold(0.4).lp(600.0, 1.0),
            sine(46.0, 40.0).at(0.05).dur(0.5).vol(0.14).swell(0.12),
        ],
        // The large bat: an ultrasonic chirp, sweeping.
        K::LargeBat => [
            (0..4).map(|i| sine(7200.0, 3000.0).at(0.05 + i as f32 * 0.06).dur(0.05).vol(0.12)).collect::<Vec<_>>(),
            vec![sine(9000.0, 2500.0).at(0.3).dur(0.2).vol(0.05)],
        ]
        .concat(),
        // ---- stage 3
        // The tengu: a gust of wind, and "kaa".
        K::Tengu => [
            vec![hiss(1.0).at(0.0).dur(0.4).vol(0.6).swell(0.2).bp(900.0, 0.7)],
            speak(0.32, 260.0, &[(Onset::K, Vowel::A, 0.18, 1.0)], 0.5, false),
        ]
        .concat(),
        // The rasetsu: a demonic double laugh, two voices an octave apart.
        K::Rasetsu => [cackle(0.04, 140.0, 4, 0.11, 1.0, 0.45), cackle(0.04, 282.0, 4, 0.11, 1.0, 0.28)].concat(),
        // The dire wolf: a deep growl.
        K::DireWolf => growl(0.04, 62.0, 0.55, 0.11),
        // The jorogumo: a woman's hum turning into a chitter.
        K::Jorogumo => [
            vec![
                tri(330.0, 330.0).at(0.02).dur(0.3).vol(0.08).swell(0.08).vib(5.0, 0.01),
                saw(330.0, 330.0).at(0.02).dur(0.3).vol(0.25).swell(0.08).bp(900.0, 4.0),
            ],
            (0..9).map(|i| hiss(1.4).at(0.28 + i as f32 * 0.022).dur(0.012).vol(0.8).bp(3600.0, 4.0)).collect(),
        ]
        .concat(),
        // The cobra: a hiss with no rattle, its hood flaring.
        K::Cobra => vec![
            hiss(1.0).at(0.02).dur(0.45).vol(0.1).swell(0.05).hold(0.7).hp(4200.0, 0.0),
            hiss(0.8).at(0.05).dur(0.12).vol(0.5).lp(500.0, 1.0),
            sine(120.0, 70.0).at(0.05).dur(0.1).vol(0.1),
        ],
        // ---- stage 4
        // The oni: a drum-chant grunt, twice.
        K::Oni => [0.0f32, 0.22]
            .iter()
            .enumerate()
            .flat_map(|(i, &t)| {
                let p = if i == 0 { 1.0 } else { 0.95 };
                [
                    sine(85.0 * p, 45.0 * p).at(t).dur(0.25).vol(0.3),
                    saw(105.0 * p, 100.0 * p).at(t).dur(0.16).vol(0.3).swell(0.01).bp(480.0, 2.0),
                ]
            })
            .collect(),
        // The kappa: water sloshing, and "hah".
        K::Kappa => [
            vec![hiss(0.8).at(0.0).dur(0.25).vol(0.5).swell(0.1).lp(900.0, 1.0), sine(260.0, 140.0).at(0.05).dur(0.12).vol(0.06)],
            speak(0.3, 210.0, &[(Onset::H, Vowel::A, 0.16, 1.0)], 0.45, false),
        ]
        .concat(),
        // The rokurokubi: a giggle, and her neck creaking.
        K::Rokurokubi => [
            (0..4)
                .map(|i| tri(620.0 + 60.0 * (i % 2) as f32, 600.0).at(0.03 + i as f32 * 0.07).dur(0.05).vol(0.14).bp(1500.0, 1.5))
                .collect::<Vec<_>>(),
            vec![saw(34.0, 40.0).at(0.25).dur(0.3).vol(3.0).swell(0.05).bp(1400.0, 7.0)],
        ]
        .concat(),
        // The kitsune: a fox's bark ringing into a bell.
        K::Kitsune => [
            vec![
                saw(900.0, 520.0).at(0.03).dur(0.12).vol(0.2).swell(0.01).bp(1300.0, 2.0),
                hiss(1.2).at(0.03).dur(0.05).vol(0.4).bp(1600.0, 2.0),
            ],
            bell(0.16, 1568.0, 0.6, 0.045),
        ]
        .concat(),
        // ---- stage 5
        // The karakoncolos: the hero's own name, called in a cold voice.
        K::Karakoncolos => name_call(hero),
        // The tülükuş: the owl's dooming cry, two low notes.
        K::Tulukus => vec![
            sine(330.0, 322.0).at(0.0).dur(0.28).vol(0.09).swell(0.06).hold(0.4),
            hiss(1.0).at(0.0).dur(0.28).vol(0.25).swell(0.06).bp(660.0, 2.0),
            sine(294.0, 284.0).at(0.36).dur(0.45).vol(0.09).swell(0.07).hold(0.4),
            hiss(1.0).at(0.36).dur(0.45).vol(0.25).swell(0.07).bp(590.0, 2.0),
        ],
        // The albastı: a wheezing breath, close.
        K::Albasti => (0..3)
            .flat_map(|i| {
                let t = i as f32 * 0.17;
                [
                    hiss(1.0).at(t).dur(0.14).vol(1.0).swell(0.1).bp(if i % 2 == 0 { 1500.0 } else { 1100.0 }, 3.5),
                    sine(70.0, 64.0).at(t).dur(0.14).vol(0.05).swell(0.1),
                ]
            })
            .collect(),
        // The cin: a flame crackling where there is no fire.
        K::Cin => vec![
            crackle(1.4).at(0.0).dur(0.5).vol(0.5).swell(0.1).hold(0.5).hp(2500.0, 0.0),
            hiss(0.5).at(0.0).dur(0.5).vol(0.6).swell(0.3).lp(380.0, 1.0),
            sine(90.0, 70.0).at(0.05).dur(0.4).vol(0.04).swell(0.2),
        ],
        // The arçura: creaking wood, rustling leaves.
        K::Arcura => [
            vec![saw(41.0, 36.0).at(0.0).dur(0.35).vol(2.0).swell(0.05).hold(0.5).bp(900.0, 8.0)],
            (0..5).map(|i| hiss(1.6).at(0.1 + i as f32 * 0.07).dur(0.06).vol(0.12).swell(0.02).hp(4500.0, 0.0)).collect(),
        ]
        .concat(),
        // ---- stage 6
        // Tepegöz: the one-eyed roar, stone grinding under it.
        K::Tepegoz => vec![
            saw(150.0, 100.0).at(0.0).dur(0.6).vol(0.12).swell(0.1).hold(0.4).lp(900.0, 2.0).vib(5.0, 0.02),
            saw(151.8, 101.0).at(0.0).dur(0.6).vol(0.1).swell(0.1).hold(0.4).lp(900.0, 2.0).vib(5.7, 0.02),
            saw(75.0, 50.0).at(0.0).dur(0.6).vol(0.12).swell(0.1).hold(0.4).lp(500.0, 2.0),
            crackle(0.35).at(0.0).dur(0.6).vol(0.7).swell(0.1).lp(300.0, 1.0),
            hiss(0.4).at(0.05).dur(0.5).vol(0.5).lp(180.0, 1.0),
        ],
        // The gashadokuro: a ringing in the ears, rising.
        K::Gashadokuro => vec![
            sine(3000.0, 4200.0).at(0.0).dur(0.7).vol(0.055).swell(0.55),
            sine(3010.0, 4230.0).at(0.0).dur(0.7).vol(0.04).swell(0.55),
            hiss(1.4).at(0.1).dur(0.03).vol(0.6).bp(1700.0, 5.0),
        ],
        // The seven-headed dragon: seven hisses in a chord.
        K::Ejder => [1.0f32, 1.19, 1.5, 1.78, 2.0, 2.38, 3.0]
            .iter()
            .enumerate()
            .map(|(i, r)| hiss(1.0).at(i as f32 * 0.03).dur(0.4).vol(0.3).swell(0.05).hold(0.5).bp(1400.0 * r, 9.0))
            .collect(),
        // The goryō: thunder murmuring, a court flute.
        K::Goryo => vec![
            hiss(0.5).at(0.0).dur(0.6).vol(1.0).swell(0.2).lp(140.0, 1.0),
            crackle(0.3).at(0.0).dur(0.6).vol(0.5).swell(0.2).lp(200.0, 1.0),
            sine(880.0, 932.0).at(0.1).dur(0.4).vol(0.04).swell(0.08).hold(0.5).vib(5.5, 0.012),
            hiss(1.0).at(0.1).dur(0.4).vol(0.25).swell(0.08).hold(0.5).bp(880.0, 6.0),
        ],
        // ---- stage 7
        // The frost giant: a glacier groaning, ice crackling.
        K::FrostGiant => vec![
            saw(52.0, 44.0).at(0.0).dur(0.65).vol(0.55).swell(0.15).hold(0.4).bp(260.0, 4.0).vib(2.0, 0.02),
            crackle(2.0).at(0.05).dur(0.5).vol(0.5).hp(5000.0, 0.0),
            sine(2400.0, 2300.0).at(0.2).dur(0.3).vol(0.012),
        ],
        // Surtr: a forge roaring.
        K::Surtr => vec![
            hiss(0.6).at(0.0).dur(0.6).vol(0.55).swell(0.25).hold(0.4).lp(900.0, 1.0),
            crackle(1.1).at(0.05).dur(0.55).vol(0.45).hp(2200.0, 0.0),
            saw(66.0, 60.0).at(0.0).dur(0.6).vol(0.12).swell(0.25).lp(300.0, 1.0),
        ],
        // Níðhöggr: roots gnawed, the earth groaning.
        K::Nidhogg => [
            (0..10).map(|i| hiss(1.0).at(i as f32 * 0.04).dur(0.03).vol(0.45).bp(700.0, 3.0)).collect::<Vec<_>>(),
            vec![saw(36.0, 32.0).at(0.0).dur(0.6).vol(0.16).swell(0.2).hold(0.4).lp(220.0, 1.0)],
        ]
        .concat(),
        // Hel: two voices at once, the living and the dead.
        K::Hel => vec![
            tri(392.0, 380.0).at(0.0).dur(0.5).vol(0.05).swell(0.1).hold(0.5).vib(5.0, 0.01),
            saw(392.0, 380.0).at(0.0).dur(0.5).vol(0.12).swell(0.1).hold(0.5).bp(1000.0, 4.0),
            saw(197.0, 186.0).at(0.0).dur(0.5).vol(0.065).swell(0.1).hold(0.5).lp(500.0, 1.0).vib(3.0, 0.02),
            hiss(1.0).at(0.0).dur(0.5).vol(0.2).swell(0.15).hold(0.4).bp(2600.0, 2.0),
        ],
        // ---- the realms' own, from their realm's palette
        // The Peerless: a mirror's glass chime, and his own breath drawn in.
        K::Peerless => [
            bell(0.05, 2093.0 / 2.76 * 2.32, 0.5, 0.05),
            vec![
                hiss(1.0).at(0.0).dur(0.42).vol(0.6).swell(0.4).bp(2000.0, 1.0),
                saw(120.0, 115.0).at(0.4).dur(0.12).vol(0.2).lp(600.0, 1.0),
            ],
        ]
        .concat(),
        // Ganglati and Ganglöt: a slow, wet groan out of the mire.
        K::Ganglot => [
            vec![saw(70.0, 64.0).at(0.0).dur(0.7).vol(0.55).swell(0.3).hold(0.3).bp(380.0, 3.0).vib(1.5, 0.03)],
            bubbles(0.2, 150.0, &[0.0, 0.1, 0.25], 0.07),
        ]
        .concat(),
        // A gaki: a needle-thin whine over a hungry belly.
        K::Gaki => vec![
            sine(1700.0, 1900.0).at(0.0).dur(0.5).vol(0.035).swell(0.1).vib(7.0, 0.03),
            sine(90.0, 80.0).at(0.0).dur(0.5).vol(0.1).swell(0.1).vib(3.0, 0.1),
            hiss(0.6).at(0.05).dur(0.4).vol(0.4).bp(300.0, 2.0),
        ],
        // An ellepige: a sweet hum, and the hollow knock of the trunk behind.
        K::Ellepige => vec![
            tri(523.0, 523.0).at(0.0).dur(0.45).vol(0.08).swell(0.1).hold(0.5).vib(5.5, 0.008),
            tri(659.0, 659.0).at(0.0).dur(0.45).vol(0.05).swell(0.1).hold(0.5).vib(5.0, 0.008),
            hiss(1.0).at(0.25).dur(0.08).vol(1.1).bp(380.0, 9.0),
            hiss(1.0).at(0.33).dur(0.08).vol(1.0).bp(360.0, 9.0),
        ],
        // ---- the prince of Greed
        K::Mammon => match mv {
            // Coins tossed, rained or flung: coins poured and his toll bell.
            MoveId::CoinToss | MoveId::CoinRain | MoveId::CoinVolley => [coins(0.04, 10, 0.4, 0.05), bell(0.05, 174.6, 1.4, 0.09)].concat(),
            // His grasp: the banker's laugh.
            MoveId::Grasp => banker_laugh(0.05),
            // Sceptre and hoard: coins poured, and a low laugh under them.
            _ => [coins(0.04, 14, 0.45, 0.03), shaped(&banker_laugh(0.1), 1.0, 0.8, 0.6)].concat(),
        },
        // A gold golem: gold grinding on gold.
        K::GoldGolem => [
            vec![
                crackle(0.4).at(0.0).dur(0.5).vol(1.2).swell(0.15).lp(400.0, 1.0),
                saw(60.0, 55.0).at(0.0).dur(0.5).vol(0.15).swell(0.15).lp(250.0, 1.0),
            ],
            coins(0.15, 5, 0.3, 0.04),
        ]
        .concat(),
        // The six other princes and their adds: a kin's voice, for now.
        K::Lucifer
        | K::Reflection
        | K::Leviathan
        | K::Serpentling
        | K::Amon
        | K::Raven
        | K::Belphegor
        | K::Nightmare
        | K::Beelzebub
        | K::FlySwarm => {
            let (kin, time, pitch, gain) = kin_voice(kind).unwrap_or((K::Mammon, 1.0, 1.0, 1.0));
            shaped(&breed_tell(kin, mv, hero), time, pitch, gain)
        }
        // ---- the prince of Lust: three heads, three voices
        K::Asmodeus => match mv {
            // The kiss: a soft "mwah" off his lips and a sweet chime under it.
            MoveId::BlownKiss => [kiss_smack(0.12), bell(0.18, 1046.5, 0.7, 0.03), suitor_hum(0.0, 0.5)].concat(),
            // The thorns: brambles rustling up, and the man's low laugh.
            MoveId::ThornLine | MoveId::ThornRing => [
                vec![crackle(1.3).at(0.0).dur(0.55).vol(0.5).swell(0.3).hp(1800.0, 0.0)],
                (0..5).flat_map(|i| snap(0.1 + i as f32 * 0.07, 900.0, 0.35)).collect(),
                suitor_laugh(0.12),
            ]
            .concat(),
            // The bull's head lowered: a snort and a bellow.
            MoveId::BullCharge => vec![
                hiss(0.9).at(0.0).dur(0.16).vol(0.8).bp(900.0, 1.5),
                saw(96.0, 78.0).at(0.14).dur(0.6).vol(0.32).swell(0.1).hold(0.4).bp(420.0, 2.2).vib(5.0, 0.03),
                saw(48.0, 40.0).at(0.14).dur(0.6).vol(0.12).swell(0.1).hold(0.4).lp(300.0, 1.0),
            ],
            // The ram's head: a hard bleat.
            MoveId::RamButt => vec![
                saw(250.0, 210.0).at(0.0).dur(0.42).vol(0.2).swell(0.03).hold(0.3).bp(900.0, 3.0).vib(11.0, 0.06),
                saw(251.5, 211.0).at(0.0).dur(0.42).vol(0.08).swell(0.03).hold(0.3).bp(1700.0, 3.0).vib(11.0, 0.06),
            ],
            // The lance (and whatever else): the man's purr and a ring of steel drawn back.
            _ => [
                suitor_hum(0.0, 0.45),
                vec![sine(1568.0, 1568.0).at(0.32).dur(0.35).vol(0.022), hiss(1.0).at(0.3).dur(0.18).vol(0.5).bp(4200.0, 6.0)],
            ]
            .concat(),
        },
    };
    // A few moves add their own colour (what `tell_sfx` once played for them).
    match mv {
        // The dead rising: a soul's voice climbing.
        MoveId::Raise => out.extend([sine(300.0, 900.0).at(0.05).dur(0.5).vol(0.04).swell(0.3).vib(6.0, 0.02)]),
        // Under the floor: the earth rumbling.
        MoveId::Burrow => out.extend([hiss(0.3).at(0.0).dur(0.6).vol(0.6).swell(0.3).lp(110.0, 1.0)]),
        _ => {}
    }
    out
}

fn banker_laugh(at: f32) -> Vec<Layer> {
    speak(at, 118.0, &[(Onset::H, Vowel::O, 0.14, 1.0), (Onset::H, Vowel::O, 0.14, 0.94), (Onset::H, Vowel::O, 0.22, 0.88)], 0.5, false)
}

/// Asmodeus's man's head: a low, pleased laugh, falling.
fn suitor_laugh(at: f32) -> Vec<Layer> {
    speak(at, 132.0, &[(Onset::H, Vowel::A, 0.11, 1.08), (Onset::H, Vowel::A, 0.11, 1.0), (Onset::H, Vowel::A, 0.2, 0.9)], 0.4, false)
}

/// Asmodeus's purr: "mmm", warm and low, a breath through it.
fn suitor_hum(at: f32, dur: f32) -> Vec<Layer> {
    vec![
        saw(146.8, 138.6).at(at).dur(dur).vol(0.13).swell(0.12).hold(0.5).lp(520.0, 1.2).vib(4.5, 0.012),
        tri(293.7, 277.2).at(at).dur(dur).vol(0.05).swell(0.12).hold(0.5).vib(4.5, 0.012),
        hiss(0.8).at(at).dur(dur).vol(0.18).swell(0.15).hold(0.3).bp(1400.0, 2.0),
    ]
}

/// A kiss blown: lips parting, a soft smack.
fn kiss_smack(at: f32) -> Vec<Layer> {
    vec![
        sine(900.0, 1500.0).at(at).dur(0.05).vol(0.07),
        hiss(1.0).at(at).dur(0.04).vol(0.6).bp(2600.0, 3.0),
        hiss(0.8).at(at + 0.04).dur(0.25).vol(0.3).swell(0.04).bp(1100.0, 1.2),
    ]
}

/// A tell: the family's signature, then the breed's timbre (and for the karakoncolos, the name of
/// the hero it calls). This replaces brawl.rs's `tell_sfx`.
pub fn tell_for(kind: EnemyKind, mv: MoveId, hero: HeroKind) -> Recipe {
    Recipe::of([signature(family(kind)), breed_tell(kind, mv, hero)])
}

/// The voice its cries are made of: its tell's own timbre (a few that do not cry with it have
/// another).
fn breed_voice(kind: EnemyKind) -> Vec<Layer> {
    use EnemyKind as K;
    match kind {
        // The name is a tell, not a cry: he howls like the winter wind.
        K::Karakoncolos => [growl(0.0, 70.0, 0.4, 0.12), vec![hiss(0.6).at(0.0).dur(0.5).vol(0.6).swell(0.15).bp(500.0, 1.5)]].concat(),
        K::Mammon => banker_laugh(0.0),
        K::Asmodeus => [suitor_laugh(0.0), suitor_hum(0.4, 0.3)].concat(),
        // Bones groan; the ringing is the tell's alone.
        K::Gashadokuro => [rattle(0.0, 6, 0.04, 1500.0, 0.7), growl(0.05, 50.0, 0.4, 0.1)].concat(),
        K::Peerless => speak(0.0, 120.0, &[(Onset::H, Vowel::U, 0.18, 1.0)], 0.45, false),
        _ => breed_tell(kind, MoveId::Slam, HeroKind::Ragnar),
    }
}

/// A foe's cry besides its tell.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Cry {
    Hurt,
    Stagger,
    Death,
    /// Waking in its den.
    Alert,
}

fn cry_of(kind: EnemyKind, cry: Cry) -> Recipe {
    let v = breed_voice(kind);
    let v = tagged(v, Part::Voice);
    match cry {
        Cry::Hurt => Recipe::of([shaped(&v, 0.5, 1.15, 1.1)]),
        Cry::Stagger => Recipe::of([
            shaped(&v, 1.25, 0.85, 0.9),
            vec![saw(150.0, 95.0).dur(0.45).vol(0.1).swell(0.04).lp(700.0, 1.0).vib(4.0, 0.03).part(Part::Voice)],
        ]),
        Cry::Alert => Recipe::of([shaped(&v, 0.9, 1.0, 0.85)]),
        Cry::Death => Recipe::of([tagged(death_of(kind), Part::Voice)]),
    }
}

/// How each falls (§5's table).
fn death_of(kind: EnemyKind) -> Vec<Layer> {
    use EnemyKind as K;
    match kind {
        // Collapsing bones.
        K::Skeleton => [rattle(0.0, 12, 0.05, 1800.0, 0.8), vec![sine(140.0, 70.0).at(0.5).dur(0.15).vol(0.2)]].concat(),
        // A caw cut short.
        K::Bat => caw(0.0, 780.0, 1, 0.0, 0.12).into_iter().map(|l| l.dur(0.08)).collect(),
        K::Vampire => [caw(0.0, 600.0, 1, 0.0, 0.1), caw(0.01, 505.0, 1, 0.0, 0.08)].concat().into_iter().map(|l| l.dur(0.09)).collect(),
        // Splat and hiss.
        K::Slime => [squelch(0.0, 1.0), vec![hiss(1.0).at(0.08).dur(0.5).vol(0.12).swell(0.05).hp(3500.0, 0.0)]].concat(),
        // A shriek fading to wind.
        K::Witch => vec![
            saw(900.0, 600.0).dur(0.5).vol(0.15).swell(0.02).bp(1500.0, 2.0).vib(7.0, 0.03),
            hiss(1.0).at(0.2).dur(0.8).vol(0.5).swell(0.2).bp(800.0, 0.8),
        ],
        // Pop.
        K::Minion => vec![sine(900.0, 200.0).dur(0.05).vol(0.2), hiss(1.0).dur(0.02).vol(0.5).hp(2000.0, 0.0)],
        // Armour clattering down.
        K::Helmeted => [
            (0..6).map(|i| sine(1250.0 + 210.0 * (i % 3) as f32, 1240.0).at(i as f32 * 0.07).dur(0.1).vol(0.04)).collect::<Vec<_>>(),
            rattle(0.0, 8, 0.05, 2400.0, 0.6),
        ]
        .concat(),
        // A whimper.
        K::Werewolf => (0..3).map(|i| sine(700.0, 450.0).at(i as f32 * 0.16).dur(0.14).vol(0.09).swell(0.02).vib(8.0, 0.02)).collect(),
        // A groan from the belly.
        K::Blemmyes => {
            vec![saw(70.0, 48.0).dur(0.9).vol(0.4).swell(0.1).hold(0.3).bp(250.0, 2.5), sine(40.0, 32.0).dur(0.8).vol(0.15).swell(0.1)]
        }
        // A squeal.
        K::LargeBat => vec![sine(2500.0, 1800.0).dur(0.3).vol(0.09).vib(14.0, 0.03)],
        // A gust dying away.
        K::Tengu => vec![hiss(1.0).dur(0.9).vol(0.6).swell(0.1).bp(700.0, 0.7), hiss(0.6).at(0.2).dur(0.8).vol(0.4).bp(400.0, 0.7)],
        // A roar, choked.
        K::Rasetsu => [
            vec![saw(160.0, 120.0).dur(0.3).vol(0.15).swell(0.03).lp(900.0, 2.0).vib(6.0, 0.03)],
            bubbles(0.25, 120.0, &[0.0, 0.05, 0.12], 0.08),
        ]
        .concat(),
        // A yelp.
        K::DireWolf => (0..2).map(|i| sine(900.0, 600.0).at(i as f32 * 0.15).dur(0.12).vol(0.12).vib(10.0, 0.02)).collect(),
        // A hiss.
        K::Jorogumo => [
            vec![hiss(1.0).dur(0.5).vol(0.12).swell(0.04).hold(0.3).hp(3800.0, 0.0)],
            (0..5).map(|i| hiss(1.4).at(0.1 + i as f32 * 0.05).dur(0.012).vol(0.4).bp(3200.0, 4.0)).collect(),
        ]
        .concat(),
        // A hiss trailing off.
        K::Cobra => vec![hiss(1.0).dur(0.9).vol(0.14).swell(0.03).hp(4500.0, 0.0)],
        // An earthshaking fall.
        K::Oni => vec![
            sine(55.0, 25.0).dur(1.0).vol(0.5),
            hiss(0.4).dur(0.8).vol(1.0).lp(200.0, 1.0),
            crackle(0.5).dur(0.6).vol(0.6).lp(800.0, 1.0),
        ],
        // Its dish spilling.
        K::Kappa => [
            squelch(0.0, 0.8),
            (0..4).map(|i| sine(1400.0 - 120.0 * i as f32, 900.0).at(0.15 + i as f32 * 0.11).dur(0.04).vol(0.05)).collect(),
        ]
        .concat(),
        // A sigh.
        K::Rokurokubi => vec![hiss(1.0).dur(0.8).vol(0.45).swell(0.25).bp(1100.0, 1.5), tri(420.0, 300.0).dur(0.7).vol(0.03).swell(0.2)],
        // A yip, and bells.
        K::Kitsune => {
            [vec![saw(1100.0, 700.0).dur(0.08).vol(0.25).bp(1500.0, 2.0)], bell(0.08, 1568.0, 0.8, 0.06), bell(0.2, 1976.0, 0.7, 0.045)]
                .concat()
        }
        // A howling wind.
        K::Karakoncolos => {
            vec![hiss(0.8).dur(1.4).vol(0.6).swell(0.3).bp(600.0, 2.5), tri(420.0, 280.0).dur(1.2).vol(0.03).swell(0.3).vib(0.8, 0.08)]
        }
        // A soft thud.
        K::Tulukus => vec![sine(120.0, 60.0).dur(0.15).vol(0.3), hiss(1.0).dur(0.1).vol(0.4).lp(600.0, 1.0)],
        // A rattle.
        K::Albasti => (0..6).map(|i| hiss(0.6).at(i as f32 * 0.07).dur(0.06).vol(0.5).bp(500.0, 2.0)).collect(),
        // The flame gutters.
        K::Cin => vec![
            crackle(1.4).dur(0.7).vol(0.5).hp(2500.0, 0.0),
            hiss(0.5).dur(0.7).vol(0.5).lp(380.0, 1.0),
            sine(80.0, 40.0).dur(0.5).vol(0.05),
        ],
        // Timber cracking.
        K::Arcura => [crack(0.0, 1.0), vec![saw(38.0, 30.0).at(0.05).dur(0.5).vol(1.5).bp(700.0, 6.0)], crack(0.3, 0.6)].concat(),
        // The eye bursts; a rumble.
        K::Tepegoz => [
            squelch(0.0, 1.0),
            vec![hiss(0.3).at(0.1).dur(1.4).vol(1.0).swell(0.2).lp(120.0, 1.0), crackle(0.3).at(0.1).dur(1.2).vol(0.8).lp(300.0, 1.0)],
        ]
        .concat(),
        // A huge bone collapse.
        K::Gashadokuro => [
            rattle(0.0, 20, 0.05, 1400.0, 0.9),
            vec![sine(70.0, 30.0).at(0.8).dur(0.6).vol(0.4), hiss(0.4).at(0.8).dur(0.5).vol(0.8).lp(250.0, 1.0)],
        ]
        .concat(),
        // "Bir daha vur!": the dragon begs for a second blow.
        K::Ejder => speak(
            0.04,
            180.0,
            &[
                (Onset::B, Vowel::I, 0.12, 1.0),
                (Onset::D, Vowel::A, 0.12, 1.05),
                (Onset::H, Vowel::A, 0.16, 1.1),
                (Onset::V, Vowel::U, 0.3, 1.18),
            ],
            0.45,
            false,
        ),
        // Thunder rolling away.
        K::Goryo => vec![hiss(0.4).dur(1.6).vol(1.0).swell(0.05).lp(160.0, 1.0), crackle(0.3).dur(1.4).vol(0.7).lp(250.0, 1.0)],
        // Ice shattering.
        K::FrostGiant => [
            crack(0.0, 1.0),
            (0..6).map(|i| sine(1760.0 * (1.0 + 0.4 * i as f32), 1700.0).at(i as f32 * 0.04).dur(0.4).vol(0.025)).collect(),
        ]
        .concat(),
        // The fire collapsing.
        K::Surtr => vec![
            hiss(0.5).dur(1.2).vol(0.8).lp(700.0, 1.0),
            crackle(1.0).dur(1.0).vol(0.5).hp(2000.0, 0.0),
            sine(60.0, 28.0).dur(0.9).vol(0.3),
        ],
        // Roots snapping.
        K::Nidhogg => (0..5).flat_map(|i| crack(i as f32 * 0.14, 0.9 - 0.1 * i as f32)).collect(),
        // A long exhale.
        K::Hel => {
            vec![hiss(0.7).dur(1.8).vol(0.5).swell(0.2).bp(900.0, 1.2), saw(197.0, 150.0).dur(1.5).vol(0.05).swell(0.2).lp(500.0, 1.0)]
        }
        // His reflection shatters.
        K::Peerless => [
            crack(0.0, 1.0),
            (0..5).map(|i| sine(2093.0 * (1.0 + 0.3 * i as f32), 2000.0).at(i as f32 * 0.03).dur(0.5).vol(0.025)).collect(),
        ]
        .concat(),
        // A wet collapse.
        K::Ganglot => [squelch(0.0, 1.0), vec![saw(64.0, 44.0).dur(0.8).vol(0.35).bp(300.0, 3.0)]].concat(),
        // A thin wail, fading.
        K::Gaki => vec![sine(1800.0, 900.0).dur(0.9).vol(0.07).vib(7.0, 0.04)],
        // A hollow knock, a sigh.
        K::Ellepige => vec![hiss(1.0).dur(0.08).vol(0.9).bp(360.0, 9.0), hiss(1.0).at(0.1).dur(0.7).vol(0.4).swell(0.2).bp(1200.0, 1.5)],
        // An avalanche of coins.
        K::Mammon => [coins(0.0, 40, 1.4, 0.03), vec![hiss(0.5).dur(1.4).vol(0.6).lp(500.0, 1.0)]].concat(),
        K::GoldGolem => [coins(0.0, 16, 0.6, 0.05), vec![crackle(0.4).dur(0.6).vol(1.2).lp(400.0, 1.0)]].concat(),
        // The six other princes and their adds: a kin's fall, for now.
        K::Lucifer
        | K::Reflection
        | K::Leviathan
        | K::Serpentling
        | K::Amon
        | K::Raven
        | K::Belphegor
        | K::Nightmare
        | K::Beelzebub
        | K::FlySwarm => {
            let (kin, time, pitch, gain) = kin_voice(kind).unwrap_or((K::Mammon, 1.0, 1.0, 1.0));
            shaped(&death_of(kin), time, pitch, gain)
        }
        // Asmodeus: the bull's bellow and the ram's bleat dying under the man's long sigh, and smoke.
        K::Asmodeus => [
            vec![
                saw(90.0, 50.0).dur(1.1).vol(0.22).swell(0.05).bp(380.0, 2.0).vib(4.0, 0.04),
                saw(240.0, 150.0).at(0.15).dur(0.8).vol(0.08).bp(900.0, 3.0).vib(10.0, 0.06),
                hiss(0.6).at(0.2).dur(1.6).vol(0.45).swell(0.4).lp(700.0, 1.0),
            ],
            speak(0.3, 120.0, &[(Onset::H, Vowel::A, 0.5, 1.0)], 0.35, false),
        ]
        .concat(),
    }
}

/// Its weapon's whoosh or its release (§5's table); a few moves have their own.
fn swing_of(kind: EnemyKind, mv: MoveId) -> Recipe {
    use EnemyKind as K;
    let layers = match kind {
        // A rusty blade.
        K::Skeleton => [whoosh(0.0, 2200.0, 0.2, 0.6), vec![crackle(1.2).dur(0.15).vol(0.2).bp(2500.0, 2.0)]].concat(),
        // A burst of wingbeats.
        K::Bat | K::Vampire => (0..3).map(|i| hiss(0.8).at(i as f32 * 0.07).dur(0.07).vol(0.8).swell(0.03).lp(700.0, 1.0)).collect(),
        // Wet spit.
        K::Slime => vec![sine(500.0, 160.0).dur(0.1).vol(0.1), hiss(1.0).dur(0.08).vol(0.4).bp(1400.0, 1.5)],
        // A violet star's chime.
        K::Witch => [bell(0.0, 1976.0, 0.3, 0.04), vec![sine(2637.0, 2637.0).at(0.05).dur(0.25).vol(0.03)]].concat(),
        // A small claw.
        K::Minion => vec![hiss(1.0).dur(0.05).vol(0.5).bp(3800.0, 2.0)],
        // A steel whoosh.
        K::Helmeted => [whoosh(0.0, 3000.0, 0.18, 0.6), vec![sine(3600.0, 3600.0).dur(0.25).vol(0.012)]].concat(),
        // A claw raking.
        K::Werewolf => (0..3).map(|i| hiss(1.0).at(i as f32 * 0.04).dur(0.05).vol(0.55).bp(3300.0, 2.0)).collect(),
        // A club, heavy.
        K::Blemmyes => [whoosh(0.0, 520.0, 0.32, 0.9), vec![sine(90.0, 55.0).dur(0.3).vol(0.12)]].concat(),
        // A leathery wing.
        K::LargeBat => (0..2).map(|i| hiss(0.9).at(i as f32 * 0.08).dur(0.1).vol(0.6).swell(0.04).lp(1100.0, 1.0)).collect(),
        // Twin steel.
        K::Tengu => [whoosh(0.0, 3100.0, 0.16, 0.6), whoosh(0.09, 3300.0, 0.16, 0.6)].concat(),
        // Two blades, a crossed ring.
        K::Rasetsu => [whoosh(0.0, 2500.0, 0.2, 0.7), whoosh(0.07, 2300.0, 0.2, 0.7), bell(0.1, 2600.0, 0.4, 0.03)].concat(),
        // Jaws snapping.
        K::DireWolf | K::Gaki => snap(0.0, 160.0, 0.9),
        // A fireball's whoomp.
        K::Jorogumo => vec![
            hiss(0.5).dur(0.35).vol(1.0).swell(0.12).lp(380.0, 1.0),
            sine(110.0, 55.0).dur(0.3).vol(0.15).swell(0.1),
            crackle(1.0).dur(0.3).vol(0.3).hp(2000.0, 0.0),
        ],
        // A strike, snapping.
        K::Cobra => [whoosh(0.0, 1800.0, 0.1, 0.5), snap(0.08, 200.0, 0.8)].concat(),
        // The kanabō: an iron club's whoosh, its studs ringing.
        K::Oni => [whoosh(0.0, 450.0, 0.38, 1.0), bell(0.15, 1900.0, 0.35, 0.02), vec![sine(2350.0, 2350.0).at(0.15).dur(0.3).vol(0.015)]]
            .concat(),
        // A palm slap.
        K::Kappa => vec![hiss(1.0).dur(0.04).vol(1.0).bp(1500.0, 0.8), sine(180.0, 90.0).dur(0.06).vol(0.15)],
        // A neck whipping.
        K::Rokurokubi => vec![hiss(1.0).dur(0.12).vol(0.7).swell(0.1).bp(1600.0, 1.5), sine(700.0, 250.0).dur(0.12).vol(0.03)],
        // Foxfire whispering, three times.
        K::Kitsune => (0..3)
            .flat_map(|i| {
                let t = i as f32 * 0.1;
                [hiss(1.4).at(t).dur(0.12).vol(0.15).swell(0.06).hp(3200.0, 0.0), sine(740.0, 980.0).at(t).dur(0.12).vol(0.02).swell(0.06)]
            })
            .collect(),
        // A swing heavy with fur (his call is the wind).
        K::Karakoncolos if mv == MoveId::Call => vec![hiss(0.8).dur(0.6).vol(0.6).swell(0.2).bp(600.0, 2.5)],
        K::Karakoncolos => [whoosh(0.0, 380.0, 0.35, 0.9), vec![hiss(0.6).dur(0.3).vol(0.5).lp(250.0, 1.0)]].concat(),
        // Silence, then talons ripping.
        K::Tulukus => vec![hiss(1.0).at(0.18).dur(0.09).vol(0.9).bp(2600.0, 1.2), crackle(1.4).at(0.18).dur(0.08).vol(0.4).bp(3000.0, 2.0)],
        // A weight dropping.
        K::Albasti => vec![sine(75.0, 32.0).dur(0.3).vol(0.4), hiss(0.6).dur(0.25).vol(0.8).lp(260.0, 1.0)],
        // A flame snapping.
        K::Cin => vec![
            hiss(1.0).dur(0.03).vol(0.5).hp(2800.0, 0.0),
            hiss(0.6).dur(0.2).vol(0.6).swell(0.05).lp(600.0, 1.0),
            crackle(1.2).dur(0.2).vol(0.3).hp(2500.0, 0.0),
        ],
        // A thorn dart whistling.
        K::Arcura => vec![sine(2600.0, 1500.0).dur(0.22).vol(0.04).vib(14.0, 0.01), hiss(1.0).dur(0.2).vol(0.3).bp(2600.0, 4.0)],
        // A boulder of an arm sweeping.
        K::Tepegoz => [whoosh(0.0, 300.0, 0.5, 1.0), vec![crackle(0.3).dur(0.45).vol(0.6).lp(300.0, 1.0)]].concat(),
        // Bone hands clattering.
        K::Gashadokuro => [rattle(0.0, 8, 0.03, 1700.0, 0.6), whoosh(0.0, 700.0, 0.3, 0.7)].concat(),
        // The heads snapping one after another.
        K::Ejder => (0..4).flat_map(|i| shaped(&snap(i as f32 * 0.12, 160.0, 0.8), 1.0, 1.0 - 0.055 * i as f32, 1.0)).collect(),
        // Lightning cracking.
        K::Goryo => vec![
            hiss(1.0).dur(0.06).vol(0.6).hp(1500.0, 0.0),
            crackle(1.5).dur(0.15).vol(0.6).hp(2500.0, 0.0),
            hiss(0.4).at(0.05).dur(0.5).vol(0.8).lp(250.0, 1.0),
        ],
        // Ice cracking under a slam.
        K::FrostGiant => [crack(0.0, 1.0), bell(0.0, 1760.0 / 2.76 * 2.32, 0.4, 0.025), vec![sine(70.0, 35.0).dur(0.3).vol(0.35)]].concat(),
        // A burning sword's roar.
        K::Surtr => vec![
            hiss(0.6).dur(0.45).vol(0.9).swell(0.2).lp(1200.0, 1.0),
            saw(70.0, 66.0).dur(0.4).vol(0.08).lp(300.0, 1.0),
            crackle(1.0).dur(0.4).vol(0.4).hp(2200.0, 0.0),
        ],
        // A rumble from under the floor.
        K::Nidhogg => vec![
            hiss(0.3).dur(0.8).vol(1.0).swell(0.4).lp(110.0, 1.0),
            crackle(0.25).dur(0.8).vol(0.6).lp(200.0, 1.0),
            sine(38.0, 34.0).dur(0.8).vol(0.15).swell(0.3),
        ],
        // A whisper that drains.
        K::Hel => vec![hiss(1.0).dur(0.45).vol(0.4).swell(0.4).bp(2600.0, 6.0), hiss(1.0).dur(0.45).vol(0.3).swell(0.4).bp(1300.0, 6.0)],
        // His own blade, with a glass edge.
        K::Peerless => [whoosh(0.0, 2800.0, 0.16, 0.6), vec![sine(4186.0, 4186.0).at(0.1).dur(0.15).vol(0.012)]].concat(),
        // A slow drag through the mire.
        K::Ganglot => [whoosh(0.0, 400.0, 0.5, 0.8), squelch(0.35, 0.5)].concat(),
        // A light blade, and leaves.
        K::Ellepige => [
            whoosh(0.0, 2600.0, 0.14, 0.5),
            (0..3).map(|i| hiss(1.6).at(0.05 + i as f32 * 0.05).dur(0.05).vol(0.1).hp(4500.0, 0.0)).collect(),
        ]
        .concat(),
        // His coins in flight, or his sceptre ringing.
        K::Mammon => match mv {
            MoveId::CoinToss | MoveId::CoinRain | MoveId::CoinVolley => [coins(0.0, 6, 0.2, 0.035), whoosh(0.0, 1500.0, 0.2, 0.5)].concat(),
            _ => [whoosh(0.0, 800.0, 0.3, 0.8), bell(0.12, 880.0, 0.6, 0.04)].concat(),
        },
        // A golden fist, and a jingle.
        K::GoldGolem => [whoosh(0.0, 420.0, 0.35, 0.9), coins(0.1, 3, 0.15, 0.03)].concat(),
        // The six other princes and their adds: a kin's swing, for now.
        K::Lucifer
        | K::Reflection
        | K::Leviathan
        | K::Serpentling
        | K::Amon
        | K::Raven
        | K::Belphegor
        | K::Nightmare
        | K::Beelzebub
        | K::FlySwarm => {
            let (kin, time, pitch, gain) = kin_voice(kind).unwrap_or((K::Mammon, 1.0, 1.0, 1.0));
            shaped(&swing_of(kin, mv).layers, time, pitch, gain)
        }
        // Asmodeus: his lance whistling, his kiss blown, the bull's hooves, the ram's skull, thorns.
        K::Asmodeus => match mv {
            MoveId::BlownKiss => [kiss_smack(0.0), whoosh(0.02, 1800.0, 0.3, 0.35)].concat(),
            MoveId::BullCharge => (0..4)
                .flat_map(|i| {
                    [
                        sine(70.0, 40.0).at(i as f32 * 0.11).dur(0.1).vol(0.3),
                        hiss(0.5).at(i as f32 * 0.11).dur(0.08).vol(0.6).lp(300.0, 1.0),
                    ]
                })
                .collect(),
            MoveId::RamButt => [crack(0.0, 0.8), vec![sine(110.0, 55.0).dur(0.2).vol(0.35)], whoosh(0.0, 600.0, 0.2, 0.6)].concat(),
            MoveId::ThornLine | MoveId::ThornRing => {
                vec![crackle(1.6).dur(0.35).vol(0.6).hp(1500.0, 0.0), hiss(0.6).dur(0.3).vol(0.5).lp(500.0, 1.0)]
            }
            _ => [whoosh(0.0, 2400.0, 0.22, 0.7), vec![sine(1975.0, 1975.0).at(0.12).dur(0.3).vol(0.018)]].concat(),
        },
    };
    Recipe { layers }
}

// ----------------------------------------------------------------------- matter and hits

/// What a struck body is made of, for the body and the shatter of a hit (§4.4). Kept local to the
/// audio: it should become `logic::juice::Matter` when the two branches meet.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum SoundMatter {
    Flesh,
    Bone,
    Iron,
    Stone,
    Feather,
    Smoke,
    Ember,
    Ice,
    Gold,
    Slime,
    Wood,
    Chitin,
}

/// Each body's matter (§5's table; of two, the one its blows land on).
pub fn matter_of(kind: EnemyKind) -> SoundMatter {
    use EnemyKind as K;
    use SoundMatter as M;
    match kind {
        K::Skeleton | K::Helmeted | K::Gashadokuro => M::Bone,
        K::Bat | K::Vampire | K::Tengu | K::Tulukus => M::Feather,
        K::Slime | K::Kappa | K::Ganglot => M::Slime,
        K::Jorogumo => M::Chitin,
        K::Kitsune | K::Goryo | K::Hel => M::Smoke,
        K::Cin | K::Surtr => M::Ember,
        K::Arcura | K::Ellepige => M::Wood,
        K::Tepegoz => M::Stone,
        K::FrostGiant | K::Peerless => M::Ice,
        K::Mammon | K::GoldGolem => M::Gold,
        K::Reflection => M::Ice,
        K::Serpentling | K::FlySwarm => M::Chitin,
        K::Raven => M::Feather,
        K::Nightmare => M::Smoke,
        K::Lucifer | K::Leviathan | K::Amon | K::Belphegor | K::Beelzebub | K::Asmodeus => M::Flesh,
        K::Witch
        | K::Minion
        | K::Werewolf
        | K::Blemmyes
        | K::LargeBat
        | K::Rasetsu
        | K::DireWolf
        | K::Cobra
        | K::Rokurokubi
        | K::Oni
        | K::Albasti
        | K::Karakoncolos
        | K::Ejder
        | K::Nidhogg
        | K::Gaki => M::Flesh,
    }
}

/// The thud of a blow by the matter it lands on.
fn matter_body(m: SoundMatter) -> Vec<Layer> {
    use SoundMatter as M;
    let layers = match m {
        M::Flesh => vec![sine(110.0, 52.0).dur(0.14).vol(0.4), hiss(0.8).dur(0.09).vol(1.0).lp(700.0, 1.0)],
        M::Bone => {
            [vec![hiss(1.2).dur(0.05).vol(1.0).bp(1700.0, 3.0), sine(320.0, 260.0).dur(0.06).vol(0.25)], rattle(0.03, 3, 0.03, 2100.0, 0.5)]
                .concat()
        }
        M::Iron => {
            [bell(0.0, 520.0, 0.35, 0.12), vec![hiss(1.0).dur(0.04).vol(0.7).bp(2600.0, 2.0), sine(140.0, 90.0).dur(0.08).vol(0.25)]]
                .concat()
        }
        M::Stone => vec![
            sine(80.0, 45.0).dur(0.2).vol(0.5),
            crackle(0.5).dur(0.18).vol(1.2).lp(1100.0, 1.0),
            hiss(0.5).dur(0.15).vol(0.9).lp(300.0, 1.0),
        ],
        M::Feather => vec![
            hiss(1.5).dur(0.12).vol(0.35).swell(0.02).hp(2200.0, 0.0),
            hiss(0.8).dur(0.1).vol(0.6).lp(800.0, 1.0),
            sine(150.0, 90.0).dur(0.06).vol(0.2),
        ],
        M::Smoke => vec![hiss(0.8).dur(0.25).vol(0.9).swell(0.05).bp(700.0, 0.8), sine(90.0, 60.0).dur(0.15).vol(0.15)],
        M::Ember => vec![
            crackle(1.5).dur(0.2).vol(1.0).hp(2500.0, 0.0),
            sine(100.0, 55.0).dur(0.12).vol(0.35),
            hiss(0.8).dur(0.12).vol(0.7).lp(900.0, 1.0),
        ],
        M::Ice => [
            vec![hiss(1.0).dur(0.02).vol(0.6).hp(3000.0, 0.0), sine(120.0, 70.0).dur(0.08).vol(0.25)],
            bell(0.0, 1800.0 / 2.76 * 2.32, 0.25, 0.05),
        ]
        .concat(),
        M::Gold => [coins(0.0, 4, 0.06, 0.05), bell(0.0, 660.0, 0.3, 0.08), vec![sine(130.0, 80.0).dur(0.08).vol(0.25)]].concat(),
        M::Slime => [squelch(0.0, 1.0), bubbles(0.04, 200.0, &[0.0, 0.04], 0.06)].concat(),
        M::Wood => vec![
            hiss(1.0).dur(0.07).vol(1.2).bp(900.0, 4.0),
            sine(240.0, 200.0).dur(0.08).vol(0.25),
            hiss(0.7).dur(0.06).vol(0.6).lp(400.0, 1.0),
        ],
        M::Chitin => vec![
            hiss(1.0).dur(0.03).vol(0.9).bp(2600.0, 2.0),
            crackle(1.4).dur(0.06).vol(0.7).bp(3200.0, 2.0),
            sine(180.0, 110.0).dur(0.06).vol(0.2),
        ],
    };
    tagged(layers, Part::Body)
}

/// The matter breaking, as a body dies.
fn matter_shatter(m: SoundMatter) -> Vec<Layer> {
    use SoundMatter as M;
    let extra = match m {
        M::Flesh => vec![hiss(0.6).at(0.03).dur(0.35).vol(0.7).lp(400.0, 1.0)],
        M::Bone => rattle(0.02, 10, 0.04, 1900.0, 0.7),
        M::Iron => (0..5).map(|i| sine(980.0 + 170.0 * (i % 3) as f32, 960.0).at(0.04 + i as f32 * 0.06).dur(0.12).vol(0.04)).collect(),
        M::Stone => vec![crackle(0.4).dur(0.7).vol(1.0).swell(0.03).lp(900.0, 1.0), hiss(0.4).dur(0.6).vol(0.9).lp(250.0, 1.0)],
        M::Feather => (0..5).map(|i| hiss(1.0).at(i as f32 * 0.05).dur(0.06).vol(0.6).lp(1000.0, 1.0)).collect(),
        M::Smoke => vec![hiss(0.8).dur(0.7).vol(0.7).swell(0.1).bp(500.0, 0.8)],
        M::Ember => vec![crackle(1.6).dur(0.6).vol(1.0).hp(2500.0, 0.0), hiss(0.6).dur(0.5).vol(0.5).lp(600.0, 1.0)],
        M::Ice => (0..7).map(|i| sine(1600.0 * (1.0 + 0.37 * i as f32), 1500.0).at(i as f32 * 0.03).dur(0.4).vol(0.025)).collect(),
        M::Gold => coins(0.0, 14, 0.5, 0.035),
        M::Slime => [squelch(0.0, 1.0), squelch(0.08, 0.7)].concat(),
        M::Wood => [crack(0.0, 1.0), vec![saw(40.0, 32.0).at(0.04).dur(0.3).vol(1.5).bp(800.0, 6.0)]].concat(),
        M::Chitin => (0..4).flat_map(|i| crack(i as f32 * 0.04, 0.8)).collect(),
    };
    [shaped(&matter_body(m), 1.6, 0.85, 1.1), tagged(extra, Part::Body)].concat()
}

/// A shield's tier, for the clang of a block.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ShieldTier {
    Wood,
    Iron,
    Steel,
    Gilded,
}

impl ShieldTier {
    /// The guard tree's level (0-10) as a shield: wood to 2, iron to 5, steel to 8, then gilded.
    pub fn of_level(level: u8) -> Self {
        match level {
            0..=2 => ShieldTier::Wood,
            3..=5 => ShieldTier::Iron,
            6..=8 => ShieldTier::Steel,
            _ => ShieldTier::Gilded,
        }
    }
}

/// What kind of swing cut the air for nothing.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum SwingKind {
    Blade,
    Heavy,
    Claw,
    Jaws,
    Wings,
    Spell,
}

impl SwingKind {
    /// The hero's weapon: Ragnar's axe swings heavy, the others' blades.
    pub fn of_hero(hero: HeroKind) -> Self {
        match hero {
            HeroKind::Ragnar => SwingKind::Heavy,
            HeroKind::Mehmed | HeroKind::Miyamoto => SwingKind::Blade,
        }
    }
}

/// How a blow came out (§4.6's table).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum HitSound {
    /// A blow on flesh; `crit` rings high.
    Flesh {
        crit: bool,
    },
    /// A blow the armour takes without a flinch.
    Armor,
    /// The last blow of a string.
    Finisher,
    PoiseBreak,
    /// He parried; a prince's blow parried rings deeper.
    Parry {
        prince: bool,
    },
    Block(ShieldTier),
    GuardBroken,
    /// A kill; each in a quick streak a semitone higher.
    Kill {
        streak: u32,
    },
    HeroStruck(HeroKind),
    Whiff(SwingKind),
}

impl HitSound {
    fn slot(self) -> u64 {
        match self {
            HitSound::Flesh { crit } => crit as u64,
            HitSound::Armor => 2,
            HitSound::Finisher => 3,
            HitSound::PoiseBreak => 4,
            HitSound::Parry { prince } => 5 + prince as u64,
            HitSound::Block(t) => 7 + t as u64,
            HitSound::GuardBroken => 11,
            HitSound::Kill { .. } => 12,
            HitSound::HeroStruck(h) => 13 + h as u64,
            HitSound::Whiff(k) => 16 + k as u64,
        }
    }
}

fn click() -> Vec<Layer> {
    vec![hiss(1.0).dur(0.012).vol(0.4).hp(3000.0, 0.0), sine(2500.0, 1800.0).dur(0.012).vol(0.08)]
}

/// Early reflections and a short room after a blow.
fn room(len: f32, vol: f32) -> Vec<Layer> {
    let mut out: Vec<Layer> = [(0.03, 0.35), (0.055, 0.22), (0.085, 0.14)]
        .iter()
        .map(|&(t, g)| hiss(1.0).at(t).dur(0.05).vol(vol * g * 1.6).bp(900.0, 0.7))
        .collect();
    out.push(hiss(0.8).at(0.02).dur(len).vol(vol * 0.35).swell(0.03).lp(1200.0, 1.0));
    tagged(out, Part::Tail)
}

fn shield_clang(t: ShieldTier) -> Vec<Layer> {
    match t {
        ShieldTier::Wood => vec![
            hiss(1.0).dur(0.08).vol(1.3).bp(650.0, 3.0),
            sine(180.0, 150.0).dur(0.1).vol(0.3),
            hiss(0.7).dur(0.05).vol(0.6).lp(400.0, 1.0),
        ],
        ShieldTier::Iron => {
            [bell(0.0, 620.0, 0.4, 0.12), vec![hiss(1.0).dur(0.03).vol(0.9).bp(2400.0, 2.0), sine(150.0, 110.0).dur(0.06).vol(0.2)]]
                .concat()
        }
        ShieldTier::Steel => {
            [bell(0.0, 1046.0, 0.6, 0.1), vec![hiss(1.0).dur(0.02).vol(0.4).hp(3500.0, 0.0), sine(160.0, 120.0).dur(0.05).vol(0.15)]]
                .concat()
        }
        ShieldTier::Gilded => [
            bell(0.0, 1046.0, 0.7, 0.09),
            vec![
                hiss(1.0).dur(0.02).vol(0.4).hp(3500.0, 0.0),
                sine(2093.0, 2093.0).at(0.02).dur(0.9).vol(0.035).vib(5.0, 0.004),
                sine(2637.0, 2637.0).at(0.04).dur(0.9).vol(0.028).vib(5.5, 0.004),
            ],
        ]
        .concat(),
    }
}

fn whiff(k: SwingKind) -> Vec<Layer> {
    match k {
        SwingKind::Blade => [whoosh(0.0, 2600.0, 0.14, 0.7), vec![sine(300.0, 180.0).dur(0.12).vol(0.03)]].concat(),
        SwingKind::Heavy => [whoosh(0.0, 600.0, 0.3, 0.9), vec![sine(120.0, 70.0).dur(0.28).vol(0.08)]].concat(),
        SwingKind::Claw => (0..3).map(|i| hiss(1.0).at(i as f32 * 0.04).dur(0.05).vol(0.5).bp(3300.0, 2.0)).collect(),
        SwingKind::Jaws => snap(0.0, 160.0, 0.8),
        SwingKind::Wings => (0..2).map(|i| hiss(0.9).at(i as f32 * 0.08).dur(0.08).vol(0.8).swell(0.03).lp(900.0, 1.0)).collect(),
        SwingKind::Spell => vec![hiss(1.5).dur(0.2).vol(0.15).swell(0.1).hp(4000.0, 0.0), sine(1500.0, 700.0).dur(0.18).vol(0.03)],
    }
}

/// A hit's layers but its voice (the voice is played on its own, with its own round robin).
pub fn hit_recipe(outcome: HitSound, matter: SoundMatter, size: f32) -> Recipe {
    let deep = 1.0 / size.max(0.3).sqrt();
    let body = |m: SoundMatter, g: f32| shaped(&matter_body(m), 1.0, deep, g);
    let t = |l: Vec<Layer>| tagged(l, Part::Transient);
    let sw = |l: Vec<Layer>| tagged(l, Part::Sweetener);
    let layers = match outcome {
        HitSound::Flesh { crit } => {
            let ring =
                if crit { sw(vec![sine(3520.0, 3520.0).dur(0.35).vol(0.05), sine(5280.0, 5280.0).dur(0.25).vol(0.025)]) } else { vec![] };
            [t(click()), body(matter, 1.0), room(0.25, 0.7), ring].concat()
        }
        // A tick and dull iron.
        HitSound::Armor => [
            t(vec![hiss(1.0).dur(0.008).vol(0.35).hp(4500.0, 0.0), sine(3200.0, 3200.0).dur(0.008).vol(0.04)]),
            tagged(
                vec![
                    hiss(0.8).dur(0.06).vol(0.8).lp(900.0, 1.0),
                    sine(300.0, 260.0).dur(0.15).vol(0.14),
                    sine(828.0, 828.0).dur(0.1).vol(0.03),
                ],
                Part::Body,
            ),
        ]
        .concat(),
        HitSound::Finisher => [
            t(click()),
            t(delayed(click(), 0.03)),
            body(matter, 1.4),
            room(0.7, 0.9),
            sw(vec![sine(62.0, 34.0).dur(0.8).vol(0.45), hiss(0.4).dur(0.6).vol(0.6).lp(150.0, 1.0)]),
        ]
        .concat(),
        HitSound::PoiseBreak => [t(crack(0.0, 1.0)), body(matter, 1.0), sw(bell(0.02, 2637.0 / 2.76 * 2.32, 0.7, 0.04))].concat(),
        HitSound::Parry { prince } => {
            let k = if prince { 1.6 } else { 1.0 };
            let mut l = [
                t(vec![
                    sine(2794.0, 2794.0).dur(0.3).vol(0.09),
                    sine(7711.0, 7711.0).dur(0.18).vol(0.03),
                    hiss(1.0).dur(0.01).vol(0.4).hp(6000.0, 0.0),
                ]),
                tagged(vec![sine(90.0, 50.0).dur(0.15 * k).vol(0.4)], Part::Body),
                tagged(vec![hiss(1.0).at(0.02).dur(0.22 * k).vol(0.6).swell(0.2 * k).bp(1500.0, 1.0)], Part::Tail),
                sw(bell(0.0, 1318.0, 1.2, 0.06)),
            ]
            .concat();
            if prince {
                l.extend(sw(bell(0.0, 523.0, 1.6, 0.07)));
            }
            l
        }
        HitSound::Block(tier) => t(shield_clang(tier)),
        // A crack and a splinter.
        HitSound::GuardBroken => [t(crack(0.0, 1.0)), tagged(rattle(0.02, 6, 0.025, 2800.0, 0.6), Part::Body)].concat(),
        HitSound::Kill { streak } => {
            let up = 2f32.powf(streak.min(12) as f32 / 12.0);
            let mut streak_layers = vec![square(900.0 * up, 200.0 * up).dur(0.25).vol(0.2)];
            if streak >= 2 {
                streak_layers.push(tri(1320.0, 1320.0).at(0.05).dur(0.12).vol(0.08));
            }
            let crunch = (0..3)
                .map(|i| hiss(1.0).at([0.0, 0.012, 0.027][i]).dur(0.03).vol(1.0).bp(900.0, 0.8))
                .chain([crackle(1.0).dur(0.08).vol(0.8).lp(2000.0, 1.0)])
                .collect();
            [t(crunch), shaped(&matter_shatter(matter), 1.0, deep, 1.0), room(0.5, 0.8), sw(streak_layers)].concat()
        }
        // A slap and a thud.
        HitSound::HeroStruck(_) => [
            t(vec![hiss(1.0).dur(0.04).vol(1.1).bp(1200.0, 0.9), sine(220.0, 220.0).dur(0.03).vol(0.1)]),
            tagged(matter_body(SoundMatter::Flesh), Part::Body),
        ]
        .concat(),
        HitSound::Whiff(k) => t(whiff(k)),
    };
    Recipe { layers }
}

/// The hero's own cries.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum HeroCry {
    /// Struck.
    Hurt,
    /// A parry's grunt.
    Grunt,
    /// His guard broken.
    Gasp,
}

/// One voice per hero: Ragnar deep, Mehmed a shout, Miyamoto a clipped kiai.
pub fn hero_voice(hero: HeroKind, cry: HeroCry) -> Recipe {
    use Onset as O;
    use Vowel as W;
    let layers = match (hero, cry) {
        (HeroKind::Ragnar, HeroCry::Hurt) => {
            [speak(0.02, 92.0, &[(O::None, W::O, 0.26, 1.0)], 0.6, false), vec![sine(46.0, 40.0).dur(0.2).vol(0.12)]].concat()
        }
        (HeroKind::Ragnar, HeroCry::Grunt) => speak(0.06, 96.0, &[(O::N, W::U, 0.12, 1.0)], 0.5, false),
        (HeroKind::Mehmed, HeroCry::Hurt) => speak(0.04, 165.0, &[(O::H, W::A, 0.24, 1.0)], 0.55, false),
        (HeroKind::Mehmed, HeroCry::Grunt) => speak(0.05, 150.0, &[(O::H, W::A, 0.1, 1.0)], 0.5, false),
        (HeroKind::Miyamoto, HeroCry::Hurt) => {
            syllable(0.0, 210.0, O::None, W::E, 0.09, 0.55, 0.05).into_iter().map(|l| l.hold(0.9)).collect()
        }
        (HeroKind::Miyamoto, HeroCry::Grunt) => {
            syllable(0.03, 200.0, O::K, W::I, 0.07, 0.5, 0.05).into_iter().map(|l| l.hold(0.9)).collect()
        }
        (h, HeroCry::Gasp) => {
            let (f, len) = match h {
                HeroKind::Ragnar => (1600.0, 0.2),
                HeroKind::Mehmed => (1900.0, 0.18),
                HeroKind::Miyamoto => (2200.0, 0.12),
            };
            vec![hiss(1.2).dur(len).vol(0.6).swell(len * 0.9).bp(f, 1.5)]
        }
    };
    Recipe { layers: tagged(layers, Part::Voice) }
}

/// Everything a breed sounds like (§5's `VoiceSet`; barks belong to rivals and generals).
#[derive(Clone, Debug)]
pub struct VoiceSet {
    pub tell: Recipe,
    pub swing: Recipe,
    pub matter: SoundMatter,
    pub hurt: Recipe,
    pub stagger: Recipe,
    pub death: Recipe,
    pub alert: Recipe,
    pub size: f32,
}

/// A breed's voice set, its tell and swing for the archetype's own move (`MoveId::Slam`'s role).
pub fn voice_set(kind: EnemyKind, hero: HeroKind) -> VoiceSet {
    VoiceSet {
        tell: tell_for(kind, MoveId::Slam, hero),
        swing: swing_of(kind, MoveId::Slam),
        matter: matter_of(kind),
        hurt: cry_of(kind, Cry::Hurt),
        stagger: cry_of(kind, Cry::Stagger),
        death: cry_of(kind, Cry::Death),
        alert: cry_of(kind, Cry::Alert),
        size: size_of(kind),
    }
}

// ----------------------------------------------------------------- playing them

/// The dedupe and round-robin key of a sound: what it is, whose, and which.
fn key(slot: u64, a: u64, b: u64) -> u64 {
    (slot << 48) | (a << 24) | b
}

/// The old effects' keys, so the mix dedupes them too.
pub(super) fn sfx_key(sfx: Sfx) -> u64 {
    key(9, sfx as u64, 0)
}

/// How a recipe goes into the mix.
#[derive(Clone, Copy, Debug)]
pub struct Play {
    pub prio: Priority,
    pub pitch: f64,
    pub sig_pitch: f64,
    pub stretch: f64,
    /// Ducks pass it by.
    pub immune: bool,
    /// It ducks everything else (`mix::DUCK_SECS`).
    pub duck: bool,
}

impl Default for Play {
    fn default() -> Self {
        Self { prio: Priority::Rest, pitch: 1.0, sig_pitch: 1.0, stretch: 1.0, immune: false, duck: false }
    }
}

impl AudioManager {
    /// Where the ears are, each frame: on the hero, facing the way the camera looks.
    pub fn set_listener(&mut self, l: Listener) {
        self.listener = l;
    }

    pub fn listener(&self) -> Listener {
        self.listener
    }

    /// Whose name the karakoncolos calls, and whose voice the parry's grunt and the gasp are.
    pub fn set_hero(&mut self, hero: HeroKind) {
        self.hero = hero;
    }

    /// A foe's tell as its move begins: its family's signature and its own voice, stretched so the
    /// family's peak falls on the move's release, panned from where it stands. The tell of the
    /// token holder: nothing in the mix may steal it.
    pub fn tell(&mut self, kind: EnemyKind, mv: MoveId, at: V, listener: Listener) {
        self.listener = listener;
        let hero = self.hero;
        let r = tell_for(kind, mv, hero);
        let release = mv.data().startup as f64 / 60.0;
        let size = size_of(kind) as f64;
        let how = Play {
            prio: Priority::Tell,
            pitch: 1.0 / size,
            sig_pitch: 1.0 / size.sqrt(),
            stretch: (release / NOMINAL_RELEASE).clamp(0.6, 2.2),
            ..Default::default()
        };
        self.play_recipe(&r, key(1, kind as u64, mv as u64 * 4 + hero as u64), Some(at), how);
    }

    /// Its weapon's whoosh, or its spell's release, on the move's first active frame.
    pub fn swing(&mut self, kind: EnemyKind, mv: MoveId, at: V) {
        let size = size_of(kind) as f64;
        let how = Play { pitch: 1.0 / size.sqrt(), stretch: size.powf(0.25), ..Default::default() };
        self.play_recipe(&swing_of(kind, mv), key(2, kind as u64, mv as u64), Some(at), how);
    }

    /// A foe crying out: hurt, staggered, dying, or woken in its den.
    pub fn foe_cry(&mut self, kind: EnemyKind, cry: Cry, at: V) {
        self.foe_cry_at(kind, cry, Some(at));
    }

    fn foe_cry_at(&mut self, kind: EnemyKind, cry: Cry, at: Option<V>) {
        let size = size_of(kind) as f64;
        let how = Play { pitch: 1.0 / size, stretch: size.powf(0.35), ..Default::default() };
        self.play_recipe(&cry_of(kind, cry), key(3, kind as u64, cry as u64), at, how);
    }

    /// The hero crying out (one voice per hero).
    pub fn hero_cry(&mut self, hero: HeroKind, cry: HeroCry) {
        self.hero_cry_with(hero, cry, false);
    }

    fn hero_cry_with(&mut self, hero: HeroKind, cry: HeroCry, immune: bool) {
        let how = Play { prio: Priority::Hero, immune, ..Default::default() };
        self.play_recipe(&hero_voice(hero, cry), key(4, hero as u64, cry as u64), None, how);
    }

    /**
     * A blow's sound in layers (§4.6): transient, body by the struck one's `matter`, tail,
     * sweetener, and the voice: `kind`'s hurt, stagger or death cry, or the hero's grunt, gasp or
     * hurt. A parry and a struck hero duck everything else for a moment. `at` is where it lands
     * (`None`: on him, centred).
     */
    pub fn hit(&mut self, outcome: HitSound, matter: SoundMatter, kind: EnemyKind, at: Option<V>) {
        let duck = matches!(outcome, HitSound::Parry { .. } | HitSound::HeroStruck(_));
        let how = Play { prio: Priority::Hero, immune: duck, duck, ..Default::default() };
        let r = hit_recipe(outcome, matter, size_of(kind));
        self.play_recipe(&r, key(5, outcome.slot(), matter as u64), at, how);
        let hero = self.hero;
        match outcome {
            HitSound::Flesh { .. } | HitSound::Finisher => self.foe_cry_at(kind, Cry::Hurt, at),
            HitSound::PoiseBreak => self.foe_cry_at(kind, Cry::Stagger, at),
            HitSound::Kill { .. } => self.foe_cry_at(kind, Cry::Death, at),
            HitSound::Parry { .. } => self.hero_cry_with(hero, HeroCry::Grunt, true),
            HitSound::GuardBroken => self.hero_cry_with(hero, HeroCry::Gasp, false),
            HitSound::HeroStruck(h) => self.hero_cry_with(h, HeroCry::Hurt, true),
            HitSound::Armor | HitSound::Block(_) | HitSound::Whiff(_) => {}
        }
    }

    /// One of the old effects, from a place: panned and fallen away as a recipe would be.
    pub fn play_at(&mut self, sfx: Sfx, at: V) {
        let (pan, gain) = mix::place(&self.listener, at);
        if gain < 1e-3 {
            return;
        }
        self.place_next = Some((pan, gain));
        self.play(sfx);
    }

    /// Plays any recipe: placed from the listener, at `how.prio` in the mix, merged into its last
    /// play if it is one too many (`key`), in the next of its three variants.
    pub fn play_recipe(&mut self, r: &Recipe, key: u64, at: Option<V>, how: Play) {
        let Some(synth) = self.synth.clone() else { return };
        if r.layers.is_empty() {
            return;
        }
        let (pan, gain) = at.map_or((0.0, 1.0), |p| mix::place(&self.listener, p));
        if gain < 1e-3 {
            return;
        }
        let variant = self.robin.next(key, &mut self.rng);
        let vp = self.rng.range(0.97, 1.03);
        let vg = self.rng.range(0.9, 1.04);
        let Ok(mut s) = synth.lock() else { return };
        let group = s.next_group();
        if !s.admit(key, group) {
            return;
        }
        if how.duck {
            s.duck_for(mix::DUCK_SECS);
        }
        let (pan_l, pan_r) = mix::pan_gains(pan);
        s.stamp = Stamp { pan_l, pan_r, prio: how.prio, group, immune: how.immune, gain: gain * vg };
        let t = s.now;
        render(&mut s, r, t, Render { pitch: how.pitch * vp, sig_pitch: how.sig_pitch * vp, stretch: how.stretch, variant });
        s.stamp = Stamp::default();
    }

    /// Slowed (or hastened) time: the effects pitch by `s^0.25` and, below full speed, a low-pass
    /// closes over the music (wide open at 1, about 2.7 kHz at half speed, 600 Hz at a quarter).
    pub fn set_time_scale(&mut self, scale: f64) {
        let scale = scale.clamp(0.05, 4.0);
        self.with(|s| {
            s.sfx_rate = scale.powf(0.25);
            s.slow_k = if scale >= 0.999 {
                1.0
            } else {
                let cutoff = 300.0 + 19_700.0 * scale.powi(3);
                1.0 - (-std::f64::consts::TAU * cutoff / s.sample_rate).exp()
            };
        });
    }

    /// The war drum's level, following the Drum's phase (§6).
    pub fn set_drum(&mut self, level: DrumLevel) {
        self.with(|s| s.drum.set(level));
    }

    /// Seconds to the next bar line of the music playing (so a horn can land on it); 0 with no
    /// audio device.
    pub fn next_bar_in(&self) -> f64 {
        self.beat().map_or(0.0, |(bpm, beats)| (4.0 - beats.rem_euclid(4.0)) * 60.0 / bpm)
    }

    /// The music's tempo and the beats since its first downbeat.
    pub fn beat(&self) -> Option<(f64, f64)> {
        let s = self.synth.as_ref()?.lock().ok()?;
        Some(s.beat_clock())
    }
}

// ------------------------------------------------------------------- the audio sheet

/// An audio manager over a synth with no device, for rendering offline.
pub(crate) fn offline() -> (AudioManager, std::sync::Arc<std::sync::Mutex<Synth>>) {
    let synth = std::sync::Arc::new(std::sync::Mutex::new(Synth::new(SHEET_RATE as f64, super::MUSIC_VOLUME)));
    {
        let mut s = synth.lock().unwrap();
        s.master = super::MASTER_VOLUME;
        s.music.running = false;
        s.drone_gain_target = 0.0;
    }
    let mut a = AudioManager::new();
    a.muted = false;
    a.synth = Some(synth.clone());
    (a, synth)
}

const SHEET_RATE: u32 = 48_000;

/// Renders `secs` of the synth, interleaved stereo.
fn render_secs(synth: &std::sync::Mutex<Synth>, secs: f64) -> Vec<f32> {
    let mut buf = vec![0.0f32; (secs * SHEET_RATE as f64) as usize * 2];
    let mut s = synth.lock().unwrap();
    for block in buf.chunks_mut(1024) {
        s.render(block, 2);
    }
    buf
}

/// Peak and RMS of a buffer.
pub fn levels(buf: &[f32]) -> (f32, f32) {
    let peak = buf.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    let rms = (buf.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / buf.len().max(1) as f64).sqrt() as f32;
    (peak, rms)
}

/// One sound of the sheet: its name and its interleaved stereo samples.
pub struct SheetSound {
    pub name: String,
    pub samples: Vec<f32>,
}

fn snake(kind: EnemyKind) -> String {
    let s = format!("{kind:?}");
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

/// Every breed's tell, swing, hit and death, the hits of §4.6, the heroes' cries and the war
/// drum's levels, rendered as the game plays them (through the mix, at the master volume).
pub fn audio_sheet() -> Vec<SheetSound> {
    let (mut a, synth) = offline();
    let mut out = Vec::new();
    let here = V::new(0.0, 0.0);
    let front = V::new(0.0, -3.0);
    let l = Listener::default();
    a.set_listener(l);
    let take = |name: String, secs: f64, out: &mut Vec<SheetSound>| out.push(SheetSound { name, samples: render_secs(&synth, secs) });
    for &kind in ALL_KINDS {
        let k = snake(kind);
        let mv = default_move(kind);
        let heroes: &[HeroKind] =
            if kind == EnemyKind::Karakoncolos { &[HeroKind::Ragnar, HeroKind::Mehmed, HeroKind::Miyamoto] } else { &[HeroKind::Ragnar] };
        for &h in heroes {
            a.set_hero(h);
            a.tell(kind, mv, front, l);
            let name = if heroes.len() > 1 { format!("{k}_tell_{}", h.name().to_lowercase()) } else { format!("{k}_tell") };
            take(name, 1.6, &mut out);
        }
        a.set_hero(HeroKind::Ragnar);
        a.swing(kind, mv, front);
        take(format!("{k}_swing"), 1.0, &mut out);
        a.hit(HitSound::Flesh { crit: false }, matter_of(kind), kind, Some(front));
        take(format!("{k}_hit"), 1.2, &mut out);
        a.hit(HitSound::Kill { streak: 0 }, matter_of(kind), kind, Some(front));
        take(format!("{k}_death"), 2.4, &mut out);
    }
    let foe = EnemyKind::Blemmyes;
    let hits: [(&str, HitSound); 20] = [
        ("flesh", HitSound::Flesh { crit: false }),
        ("flesh_crit", HitSound::Flesh { crit: true }),
        ("armor", HitSound::Armor),
        ("finisher", HitSound::Finisher),
        ("poise_break", HitSound::PoiseBreak),
        ("parry", HitSound::Parry { prince: false }),
        ("parry_prince", HitSound::Parry { prince: true }),
        ("block_wood", HitSound::Block(ShieldTier::Wood)),
        ("block_iron", HitSound::Block(ShieldTier::Iron)),
        ("block_steel", HitSound::Block(ShieldTier::Steel)),
        ("block_gilded", HitSound::Block(ShieldTier::Gilded)),
        ("guard_broken", HitSound::GuardBroken),
        ("kill", HitSound::Kill { streak: 0 }),
        ("kill_streak3", HitSound::Kill { streak: 3 }),
        ("struck_ragnar", HitSound::HeroStruck(HeroKind::Ragnar)),
        ("struck_mehmed", HitSound::HeroStruck(HeroKind::Mehmed)),
        ("struck_miyamoto", HitSound::HeroStruck(HeroKind::Miyamoto)),
        ("whiff_blade", HitSound::Whiff(SwingKind::Blade)),
        ("whiff_heavy", HitSound::Whiff(SwingKind::Heavy)),
        ("whiff_claw", HitSound::Whiff(SwingKind::Claw)),
    ];
    for (name, h) in hits {
        a.hit(h, matter_of(foe), foe, Some(here));
        take(format!("hit_{name}"), 1.8, &mut out);
    }
    for family in [Attack::Melee, Attack::Body, Attack::Distance] {
        let r = Recipe { layers: signature(family) };
        a.play_recipe(&r, key(8, family as u64, 0), None, Play { prio: Priority::Tell, ..Default::default() });
        take(format!("family_{}", family.name().to_lowercase()), 1.0, &mut out);
    }
    // A tell circling him: left, ahead, right, behind (louder), and far off.
    for (i, p) in [V::new(-5.0, 0.0), V::new(0.0, -5.0), V::new(5.0, 0.0), V::new(0.0, 5.0), V::new(0.0, -15.0)].into_iter().enumerate() {
        a.tell(EnemyKind::Oni, MoveId::Kanabo, p, l);
        take(format!("pan_{i}_{}", ["left", "ahead", "right", "behind", "far"][i]), 1.4, &mut out);
    }
    for (name, level) in [("build", DrumLevel::Build), ("peak", DrumLevel::Peak), ("release", DrumLevel::Release)] {
        a.set_drum(level);
        take(format!("drum_{name}"), 10.0, &mut out);
    }
    a.set_drum(DrumLevel::Rest);
    out
}

/// A move each breed is known by, for the sheet: its archetype's own, or its folklore move.
fn default_move(kind: EnemyKind) -> MoveId {
    use EnemyKind as K;
    match kind {
        K::Kappa => MoveId::Shove,
        K::Rokurokubi => MoveId::NeckBite,
        K::Kitsune => MoveId::Foxfire,
        K::Oni => MoveId::Kanabo,
        K::Tulukus => MoveId::Talons,
        K::Albasti => MoveId::Smother,
        K::Cin => MoveId::Smokeless,
        K::Arcura => MoveId::Roots,
        K::Karakoncolos => MoveId::Call,
        K::Gashadokuro => MoveId::Grab,
        K::Ejder => MoveId::Heads,
        K::Goryo => MoveId::Thunder,
        K::FrostGiant => MoveId::FrostSlam,
        K::Nidhogg => MoveId::Burrow,
        K::Hel => MoveId::Raise,
        K::Mammon => MoveId::CoinToss,
        K::Lucifer => MoveId::RadiantCut,
        K::Leviathan => MoveId::CoilLash,
        K::Amon => MoveId::FlameBreath,
        K::Belphegor => MoveId::Swat,
        K::Beelzebub => MoveId::Cleaver,
        K::Asmodeus => MoveId::LanceThrust,
        _ => match family(kind) {
            Attack::Melee => MoveId::Slam,
            Attack::Body => MoveId::Bite,
            Attack::Distance => MoveId::Hex,
        },
    }
}

/// Writes the audio sheet (`--audio-sheet DIR`): one 16-bit stereo WAV per sound, and prints each
/// one's peak and RMS.
#[cfg(not(target_arch = "wasm32"))]
pub fn write_sheet(dir: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    for s in audio_sheet() {
        let (peak, rms) = levels(&s.samples);
        println!("{:<34} peak {:>6.3}  rms {:>6.4}", s.name, peak, rms);
        let data: Vec<u8> = s.samples.iter().flat_map(|v| ((v.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes()).collect();
        let mut out = Vec::with_capacity(44 + data.len());
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes()); // PCM
        out.extend_from_slice(&2u16.to_le_bytes()); // stereo
        out.extend_from_slice(&SHEET_RATE.to_le_bytes());
        out.extend_from_slice(&(SHEET_RATE * 4).to_le_bytes());
        out.extend_from_slice(&4u16.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&data);
        std::fs::write(std::path::Path::new(dir).join(format!("{}.wav", s.name)), out)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_breed_has_its_own_voice_and_its_familys_signature() {
        let tells: Vec<(EnemyKind, Recipe)> = ALL_KINDS.iter().map(|&k| (k, tell_for(k, MoveId::Slam, HeroKind::Ragnar))).collect();
        for (i, (a, ra)) in tells.iter().enumerate() {
            for (b, rb) in &tells[i + 1..] {
                assert_ne!(ra, rb, "{a:?} and {b:?} share a tell");
            }
        }
        // Every tell, for every move and hero, opens with its family's signature, whole.
        let moves = [MoveId::Slam, MoveId::Bite, MoveId::Hex, MoveId::Call, MoveId::Grasp, MoveId::CoinToss, MoveId::Raise, MoveId::Burrow];
        for &kind in ALL_KINDS {
            let sig = signature(family(kind));
            for mv in moves {
                for hero in [HeroKind::Ragnar, HeroKind::Mehmed, HeroKind::Miyamoto] {
                    let t = tell_for(kind, mv, hero);
                    assert_eq!(t.layers[0].part, Part::Signature(family(kind)), "{kind:?}'s tell does not start with its family");
                    assert_eq!(&t.layers[..sig.len()], &sig[..], "{kind:?}'s tell does not carry its family's whole signature");
                    assert!(
                        t.layers.len() > sig.len() && t.layers[sig.len()..].iter().all(|l| l.part == Part::Breed),
                        "{kind:?} has no voice of its own"
                    );
                }
            }
        }
        // The karakoncolos calls each hero by his own name.
        let calls: Vec<Recipe> = [HeroKind::Ragnar, HeroKind::Mehmed, HeroKind::Miyamoto]
            .iter()
            .map(|&h| tell_for(EnemyKind::Karakoncolos, MoveId::Call, h))
            .collect();
        assert!(calls[0] != calls[1] && calls[1] != calls[2] && calls[0] != calls[2]);
        // And the three families' signatures differ.
        assert!(signature(Attack::Melee) != signature(Attack::Body) && signature(Attack::Body) != signature(Attack::Distance));
    }

    /// Renders a recipe alone in a fresh synth at the game's master volume.
    fn raw(r: &Recipe, how: Render) -> Vec<f32> {
        let mut s = Synth::new(48_000.0, 0.0);
        s.master = super::super::MASTER_VOLUME;
        render(&mut s, r, 0.01, how);
        let mut buf = vec![0.0f32; (48_000.0 * (r.duration() as f64 * how.stretch * 1.1 + 0.1)) as usize * 2];
        for block in buf.chunks_mut(1024) {
            s.render(block, 2);
        }
        buf
    }

    /// Under the limiter's threshold, with a little room.
    const HEADROOM: f32 = 0.85;

    fn energy(buf: &[f32]) -> f64 {
        buf.iter().map(|v| (*v as f64).powi(2)).sum()
    }

    #[test]
    fn every_voice_renders_clean_and_its_family_is_heard() {
        for &kind in ALL_KINDS {
            let size = size_of(kind) as f64;
            let how = Render { pitch: 1.0 / size, sig_pitch: 1.0 / size.sqrt(), ..Default::default() };
            let tell = tell_for(kind, default_move(kind), HeroKind::Mehmed);
            let full = raw(&tell, how);
            let (peak, rms) = levels(&full);
            // At the master volume it stays under the limiter's threshold: the limiter never shapes it.
            // At the master volume it stays under the limiter's threshold: the limiter never shapes it.
            assert!(peak < HEADROOM, "{kind:?}'s tell peaks at {peak}");
            assert!(rms > 0.008, "{kind:?}'s tell is too quiet: {rms}");
            let sig = raw(&Recipe { layers: signature(family(kind)) }, how);
            let share = energy(&sig) / energy(&full);
            // The family is a good share of it, and so is the breed's own voice.
            assert!((0.15..0.95).contains(&share), "{kind:?}'s family is {share:.3} of the energy");
            for (what, r) in [
                ("swing", swing_of(kind, default_move(kind))),
                ("hurt", cry_of(kind, Cry::Hurt)),
                ("death", cry_of(kind, Cry::Death)),
                ("stagger", cry_of(kind, Cry::Stagger)),
            ] {
                let buf = raw(&r, how);
                let (peak, rms) = levels(&buf);
                assert!(buf.iter().all(|v| v.is_finite()), "{kind:?}'s {what} is not finite");
                assert!(peak < HEADROOM && rms > 0.004, "{kind:?}'s {what}: peak {peak}, rms {rms}");
            }
        }
    }

    #[test]
    fn every_hit_renders_clean() {
        let outcomes = [
            HitSound::Flesh { crit: true },
            HitSound::Armor,
            HitSound::Finisher,
            HitSound::PoiseBreak,
            HitSound::Parry { prince: true },
            HitSound::Block(ShieldTier::Wood),
            HitSound::Block(ShieldTier::Gilded),
            HitSound::GuardBroken,
            HitSound::Kill { streak: 5 },
            HitSound::HeroStruck(HeroKind::Miyamoto),
            HitSound::Whiff(SwingKind::Spell),
        ];
        let matters =
            [SoundMatter::Flesh, SoundMatter::Bone, SoundMatter::Iron, SoundMatter::Stone, SoundMatter::Feather, SoundMatter::Smoke];
        let more = [SoundMatter::Ember, SoundMatter::Ice, SoundMatter::Gold, SoundMatter::Slime, SoundMatter::Wood, SoundMatter::Chitin];
        for o in outcomes {
            for m in matters.into_iter().chain(more) {
                let buf = raw(&hit_recipe(o, m, 1.0), Render::default());
                let (peak, rms) = levels(&buf);
                assert!(peak < HEADROOM && rms > 0.0015, "{o:?} on {m:?}: peak {peak}, rms {rms}");
            }
        }
        for h in [HeroKind::Ragnar, HeroKind::Mehmed, HeroKind::Miyamoto] {
            for c in [HeroCry::Hurt, HeroCry::Grunt, HeroCry::Gasp] {
                let (peak, rms) = levels(&raw(&hero_voice(h, c), Render::default()));
                assert!(peak < HEADROOM && rms > 0.002, "{h:?}'s {c:?}: peak {peak}, rms {rms}");
            }
        }
    }

    #[test]
    fn a_struck_hero_ducks_the_rest_and_a_tell_is_never_stolen() {
        let (mut a, synth) = offline();
        // A crowd of effects, more than the budget holds (the first stages' and the realms' kinds:
        // every kind's would overrun the synth's store of voices, stolen ones and all).
        for &kind in &ALL_KINDS[..38] {
            a.swing(kind, default_move(kind), V::new(1.0, 1.0));
            a.foe_cry(kind, Cry::Death, V::new(1.0, 1.0));
        }
        assert_eq!(synth.lock().unwrap().sfx_voices(), mix::SFX_VOICES, "the budget is full, and holds");
        a.tell(EnemyKind::Oni, MoveId::Kanabo, V::new(2.0, 0.0), Listener::default());
        // Every voice of the tell got in: the budget stole others for it.
        let r = tell_for(EnemyKind::Oni, MoveId::Kanabo, HeroKind::Ragnar);
        assert_eq!(synth.lock().unwrap().voices_at(Priority::Tell), r.layers.len());
        assert_eq!(synth.lock().unwrap().sfx_voices(), mix::SFX_VOICES);
        a.hit(HitSound::HeroStruck(HeroKind::Ragnar), SoundMatter::Flesh, EnemyKind::Oni, None);
        let s = synth.lock().unwrap();
        assert!(s.duck_until > s.now + 0.1, "a struck hero ducks the rest");
    }

    #[test]
    fn slowed_time_lowers_the_effects_and_muffles_the_music() {
        let (mut a, synth) = offline();
        a.set_time_scale(1.0);
        {
            let s = synth.lock().unwrap();
            assert_eq!((s.sfx_rate, s.slow_k), (1.0, 1.0));
        }
        a.set_time_scale(0.25);
        let s = synth.lock().unwrap();
        assert!((s.sfx_rate - 0.25f64.powf(0.25)).abs() < 1e-9);
        assert!(s.slow_k < 0.2, "the music's low-pass closes: {}", s.slow_k);
    }

    #[test]
    fn the_next_bar_line_follows_the_music() {
        let (mut a, synth) = offline();
        // The free clock at the base tempo: 2.4 s bars from zero.
        synth.lock().unwrap().now = 1.0;
        assert!((a.next_bar_in() - 1.4).abs() < 1e-9);
        // A recording: its own tempo, from its own downbeat.
        a.set_soundtrack(Some(super::super::Soundtrack::Boss));
        let mut buf = vec![0.0f32; 48_000];
        synth.lock().unwrap().render(&mut buf, 2);
        let (bpm, beats) = a.beat().unwrap();
        assert_eq!(bpm, 93.4);
        let pos = 0.5;
        assert!((beats - (pos - 0.946) * 93.4 / 60.0).abs() < 0.01, "{beats}");
        let bar = 4.0 * 60.0 / 93.4;
        let next = a.next_bar_in();
        assert!(next > 0.0 && next <= bar, "{next}");
    }
}
