//! The hymn at the end (FINALE.md §4): the first gentle sound since the old man's door. A choir
//! of soft voices singing "oo" and "ah" (saws through the vowels' two formants, two singers to a
//! part, a little apart in pitch and tremble, a pure tone under each), slow to swell and slow to
//! fade, with high bells like a celesta's falling through it. It begins as the light reaches the
//! bottom of the Pit, swells as the platform rises, opens out into the white, sings softly under
//! the words and gently under the credits, and ends on a long chord.
//!
//! The game sounds it bar by bar as Scene II plays (`game::ending`), so a skip can let it go
//! (`hymn_release`); the game's own music has stopped for it.

use super::mix::{self, Priority, Stamp};
use super::synth::{Bus, Env, FilterKind, Synth, Voice, Wave};
use super::AudioManager;

/// Seconds a bar of the hymn lasts: four slow beats at fifty a minute.
pub const BAR: f64 = 4.8;
/// Seconds a chord takes to swell in, and to die away after its bar.
const SWELL: f64 = 1.5;
const RELEASE: f64 = 2.8;
/// The mix's group every voice of the hymn carries, so they can be let go together.
const GROUP: u32 = 0x4e_4d_4e;

/// The vowel the choir sings on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Vowel {
    Ah,
    Oo,
}

impl Vowel {
    /// The first two formants (Hz) and their weights.
    fn formants(self) -> [(f64, f64); 2] {
        match self {
            Vowel::Ah => [(730.0, 1.0), (1090.0, 0.55)],
            Vowel::Oo => [(320.0, 1.0), (800.0, 0.35)],
        }
    }
}

/// One bar: its chord (MIDI notes, low to high), how full the choir sings it (0..1), on which
/// vowel, and the bells over it (beat of the bar, MIDI note).
#[derive(Clone, Copy, Debug)]
pub struct Bar {
    pub notes: [u8; 5],
    pub swell: f32,
    pub vowel: Vowel,
    pub bells: &'static [(f32, u8)],
}

const D: [u8; 5] = [50, 57, 62, 66, 69];
const D_OPEN: [u8; 5] = [50, 62, 66, 69, 74];
const G_D: [u8; 5] = [50, 59, 62, 67, 71];
const BM: [u8; 5] = [47, 54, 62, 66, 71];
const G_ADD9: [u8; 5] = [43, 55, 62, 69, 71];
const G: [u8; 5] = [43, 55, 59, 62, 67];
const EM7: [u8; 5] = [52, 59, 62, 67, 71];
const ASUS: [u8; 5] = [45, 57, 62, 64, 69];
const A: [u8; 5] = [45, 57, 61, 64, 69];

const fn bar(notes: [u8; 5], swell: f32, vowel: Vowel, bells: &'static [(f32, u8)]) -> Bar {
    Bar { notes, swell, vowel, bells }
}

/// The whole hymn, bar by bar, in D. Bar 0 sounds as the light comes down; bars 1 to 3 swell
/// with the rise; bar 4 opens into the white; 5 to 11 lie under the words; 12 to 20 under the
/// credits, the last held long.
pub const SCORE: &[Bar] = &[
    // The light
    bar(D, 0.35, Vowel::Oo, &[(2.0, 81)]),
    // The rise
    bar(G_D, 0.5, Vowel::Oo, &[(1.0, 83), (3.0, 86)]),
    bar(BM, 0.62, Vowel::Oo, &[(0.5, 86), (2.0, 83), (3.0, 81)]),
    bar(G_ADD9, 0.78, Vowel::Ah, &[(0.5, 83), (1.5, 86), (2.5, 88), (3.25, 90)]),
    // Into the white
    bar(D_OPEN, 1.0, Vowel::Ah, &[(0.0, 93), (0.25, 90), (0.5, 88), (0.75, 86), (1.0, 83), (1.25, 81), (1.5, 78), (2.0, 74)]),
    // Under the words
    bar(EM7, 0.5, Vowel::Oo, &[(0.5, 83)]),
    bar(ASUS, 0.48, Vowel::Oo, &[(1.0, 81)]),
    bar(A, 0.46, Vowel::Oo, &[(0.5, 85)]),
    bar(D, 0.48, Vowel::Oo, &[(1.0, 86)]),
    bar(G_D, 0.46, Vowel::Oo, &[(0.5, 83)]),
    bar(BM, 0.46, Vowel::Oo, &[(1.0, 78)]),
    bar(D, 0.5, Vowel::Oo, &[(0.5, 81), (2.0, 86)]),
    // Under the credits
    bar(D, 0.58, Vowel::Ah, &[(0.0, 78), (1.0, 81), (2.0, 83), (3.0, 81)]),
    bar(G, 0.6, Vowel::Ah, &[(0.0, 86), (2.0, 83)]),
    bar(BM, 0.6, Vowel::Ah, &[(0.0, 81), (1.0, 78), (2.0, 76)]),
    bar(EM7, 0.58, Vowel::Ah, &[(0.0, 78), (2.0, 74)]),
    bar(G_ADD9, 0.62, Vowel::Ah, &[(0.0, 83), (1.0, 86), (2.0, 88)]),
    bar(ASUS, 0.6, Vowel::Ah, &[(0.0, 86), (2.0, 83)]),
    bar(A, 0.58, Vowel::Ah, &[(0.0, 85), (2.0, 81)]),
    bar(D_OPEN, 0.62, Vowel::Ah, &[(0.0, 86), (1.0, 90), (2.0, 93)]),
    // The last chord, held
    bar(D, 0.5, Vowel::Oo, &[(0.0, 86), (2.0, 81)]),
];

fn hz(midi: u8) -> f64 {
    440.0 * 2f64.powf((midi as f64 - 69.0) / 12.0)
}

/// Pushes a voice of the hymn into the mix, panned, where nothing else can take its place.
fn put(s: &mut Synth, v: Voice, pan: f64) {
    let (pan_l, pan_r) = mix::pan_gains(pan.clamp(-1.0, 1.0));
    s.stamp = Stamp { pan_l, pan_r, prio: Priority::Hero, group: GROUP, immune: true, gain: 1.0, voice: false };
    s.push(v);
    s.stamp = Stamp::default();
}

/// A soft swell to `peak`, held through the bar, dying away after it.
fn swell(t: f64, held: f64, peak: f64, fade: f64) -> Env {
    Env::points(vec![(t, peak * 1e-3), (t + SWELL, peak), (t + held, peak * 0.82), (t + held + fade, peak * 1e-3)])
}

/// One part of the choir on `midi`: two singers a little apart, each through the vowel's two
/// formants, and a pure tone under them.
#[allow(clippy::too_many_arguments)]
fn sing(s: &mut Synth, t: f64, midi: u8, held: f64, fade: f64, vol: f64, vowel: Vowel, pan: f64, k: usize) {
    let f = hz(midi);
    let end = t + held + fade;
    for (n, (cents, rate)) in [(-6.0, 4.7), (7.0, 5.4)].into_iter().enumerate() {
        let side = if n == 0 { -0.18 } else { 0.18 };
        for (formant, weight) in vowel.formants() {
            // A formant under the note's own pitch would starve it: follow the note up.
            let centre = formant.max(f * 1.08);
            let v = Voice::new(t, end, Wave::Saw, Env::constant(f), swell(t, held, vol * weight, fade), Bus::Sfx)
                .detune_cents(cents + k as f64 * 0.7)
                .filtered(FilterKind::Bandpass, centre, 2.6)
                .vibrato(rate + k as f64 * 0.13, 0.0042);
            put(s, v, pan + side);
        }
    }
    let pure = Voice::new(t, end, Wave::Sine, Env::constant(f), swell(t, held, vol * 0.42, fade), Bus::Sfx).vibrato(4.9, 0.003);
    put(s, pure, pan);
}

/// A high bell, struck: a celesta's bright plate, its octave, and a faint inharmonic shimmer.
fn bell(s: &mut Synth, t: f64, midi: u8, vol: f64, pan: f64) {
    let f = hz(midi);
    for (ratio, g, d) in [(1.0, 1.0, 2.6), (2.0, 0.32, 1.4), (4.07, 0.08, 0.6)] {
        let gain = Env::points(vec![(t, vol * g * 0.01), (t + 0.006, vol * g), (t + d, vol * g * 1e-3)]);
        put(s, Voice::new(t, t + d + 0.02, Wave::Sine, Env::constant(f * ratio), gain, Bus::Sfx), pan);
    }
}

/// Schedules bar `n` of the hymn from `t` (synth seconds).
pub fn sing_bar(s: &mut Synth, t: f64, n: usize) {
    let Some(b) = SCORE.get(n) else { return };
    let last = n + 1 == SCORE.len();
    let (held, fade) = if last { (BAR * 1.4, 7.0) } else { (BAR + 0.35, RELEASE) };
    let full = b.swell as f64;
    for (k, &note) in b.notes.iter().enumerate() {
        // The low voices carry the chord, the high ones float over it a little softer.
        let vol = full * [0.034, 0.026, 0.024, 0.022, 0.019][k];
        let pan = (k as f64 - 2.0) * 0.22;
        sing(s, t, note, held, fade, vol, b.vowel, pan, k);
    }
    // Air over the voices: a breath of high hiss, swelling with them.
    let offset = (n as f64 * 0.37) % 1.5;
    let air =
        Voice::new(t, t + held + fade, Wave::Hiss { rate: 1.0, offset }, Env::constant(1.0), swell(t, held, 0.012 * full, fade), Bus::Sfx)
            .filtered(FilterKind::Bandpass, 6800.0, 1.4);
    put(s, air, 0.0);
    let beat = BAR / 4.0;
    for (i, &(at, note)) in b.bells.iter().enumerate() {
        let pan = ((i as f64 * 1.7 + n as f64).sin()) * 0.5;
        bell(s, t + at as f64 * beat, note, 0.045 * (0.6 + 0.4 * full), pan);
    }
}

impl AudioManager {
    /// Bar `n` of the hymn, beginning now.
    pub fn hymn_bar(&mut self, n: usize) {
        self.with(|s| {
            let t = s.now + 0.02;
            sing_bar(s, t, n);
        });
    }

    /// Lets the hymn go: what still sounds of it fades over `secs`, and nothing more of it starts.
    pub fn hymn_release(&mut self, secs: f64) {
        self.with(|s| s.release_group(GROUP, secs));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(synth: &std::sync::Mutex<Synth>, secs: f64) -> Vec<f32> {
        let mut buf = vec![0.0f32; (secs * 48_000.0) as usize * 2];
        let mut s = synth.lock().unwrap();
        for block in buf.chunks_mut(1024) {
            s.render(block, 2);
        }
        buf
    }

    #[test]
    fn the_hymn_keeps_to_d_major_and_its_voices_fit_the_mix() {
        // D major: D E F# G A B C#.
        let scale = [2, 4, 6, 7, 9, 11, 1];
        for b in SCORE {
            for n in b.notes.iter().chain(b.bells.iter().map(|(_, n)| n)) {
                assert!(scale.contains(&(n % 12)), "note {n} outside the key");
            }
            assert!(b.bells.iter().all(|&(at, _)| (0.0..4.0).contains(&at)));
            assert!(b.notes.windows(2).all(|w| w[0] < w[1]), "chords are voiced low to high");
        }
        // The fullest bar and the one before it, ringing together, stay inside the voice budget.
        let mut s = Synth::new(48_000.0, 0.42);
        sing_bar(&mut s, 0.0, 3);
        sing_bar(&mut s, BAR, 4);
        s.now = BAR + 0.1;
        assert!(s.sfx_voices() < mix::SFX_VOICES, "{} voices", s.sfx_voices());
    }

    #[test]
    fn it_sounds_soft_and_clean_and_a_release_lets_it_go() {
        let (mut audio, synth) = crate::audio::voices::offline();
        audio.hymn_bar(4);
        let buf = render(&synth, 3.0);
        let peak = buf.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(buf.iter().all(|v| v.is_finite()));
        assert!(peak > 0.03, "audible: {peak}");
        assert!(peak < 0.9, "never harsh: {peak}");
        // The quiet first bar is quieter than the climax.
        let (mut quiet, synth2) = crate::audio::voices::offline();
        quiet.hymn_bar(0);
        let soft = render(&synth2, 3.0).iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(soft < peak, "{soft} vs {peak}");
        // Let go, it falls silent within the release and nothing more of it starts.
        audio.hymn_bar(5);
        audio.hymn_release(0.5);
        let after = render(&synth, 1.5);
        let tail = &after[after.len() / 2..];
        let left = tail.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(left < 0.01, "silent after the release: {left}");
    }
}
