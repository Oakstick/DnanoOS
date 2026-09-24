/**
 * Procedural backing track: an upbeat 8-bar dance-pop loop, built live out
 * of oscillators and noise, so the whole soundtrack costs a few kilobytes.
 *
 * I - V - vi - IV in C twice over (the second half resolving F - G), with a
 * four-on-the-floor kick, claps on two and four, open hats on the offbeats,
 * a bouncing octave bassline, a sixteenth-note arpeggio and a composed lead
 * that answers itself: a question over bars 1-4, a higher reply over 5-8.
 *
 * It moves on its own: the first time through it builds up layer by layer,
 * every fourth bar ends in a snare fill, every other pass the lead ornaments
 * its phrase. The game pushes it too: the tempo rises every five nuggets,
 * danger thickens the drums and doubles the bass, low health muffles it all.
 */
use super::synth::{Bus, Env, FilterKind, Synth, Voice, Wave};

/// Beats per minute at the start of a dungeon.
pub const BASE_BPM: f64 = 104.0;
/// Sixteenths in the loop: eight bars of four beats.
const LOOP_STEPS: usize = 128;
/// How far ahead notes are queued.
const LOOKAHEAD: f64 = 0.12;
/// A light shuffle on the offbeat sixteenths: bouncy, not lazy.
const SWING: f64 = 0.12;

const C4: f64 = 261.626;
fn hz(semitones: f64) -> f64 {
    C4 * 2f64.powf(semitones / 12.0)
}

struct Chord {
    /// Bass root, in semitones from middle C.
    root: f64,
    /// Stab voicing around middle C.
    stab: [f64; 4],
    /// Arpeggio tones, an octave up.
    arp: [f64; 4],
}

const C: Chord = Chord { root: -24.0, stab: [4.0, 7.0, 12.0, 14.0], arp: [12.0, 16.0, 19.0, 24.0] };
const G: Chord = Chord { root: -17.0, stab: [2.0, 7.0, 11.0, 14.0], arp: [11.0, 14.0, 19.0, 23.0] };
const AM: Chord = Chord { root: -15.0, stab: [4.0, 9.0, 12.0, 16.0], arp: [12.0, 16.0, 21.0, 24.0] };
const F: Chord = Chord { root: -19.0, stab: [5.0, 9.0, 12.0, 17.0], arp: [12.0, 17.0, 21.0, 24.0] };

/// A bar each.
const BARS: [&Chord; 8] = [&C, &G, &AM, &F, &C, &G, &F, &G];

/// The lead, per bar: (sixteenth, semitone, length in sixteenths).
const MELODY: [&[(usize, f64, usize)]; 8] = [
    &[(0, 12.0, 2), (2, 16.0, 2), (4, 19.0, 3), (8, 16.0, 1), (10, 19.0, 2), (12, 21.0, 2), (14, 19.0, 2)],
    &[(0, 19.0, 3), (4, 17.0, 1), (6, 14.0, 2), (8, 11.0, 2), (10, 14.0, 2), (12, 17.0, 4)],
    &[(0, 16.0, 2), (2, 12.0, 2), (4, 16.0, 2), (6, 21.0, 4), (10, 19.0, 2), (12, 16.0, 2), (14, 14.0, 2)],
    &[(0, 12.0, 3), (4, 14.0, 1), (6, 17.0, 2), (8, 21.0, 2), (10, 19.0, 2), (12, 17.0, 2), (14, 16.0, 2)],
    &[(0, 24.0, 2), (2, 19.0, 2), (4, 16.0, 2), (6, 19.0, 2), (8, 24.0, 3), (12, 26.0, 2), (14, 24.0, 2)],
    &[(0, 23.0, 3), (4, 19.0, 1), (6, 23.0, 2), (8, 26.0, 4), (12, 23.0, 2), (14, 19.0, 2)],
    &[(0, 21.0, 2), (2, 17.0, 2), (4, 21.0, 2), (6, 24.0, 4), (10, 21.0, 2), (12, 19.0, 2), (14, 17.0, 2)],
    &[(0, 19.0, 2), (2, 23.0, 2), (4, 26.0, 4), (8, 23.0, 2), (10, 26.0, 2), (12, 31.0, 4)],
];

#[derive(Default)]
pub struct Sequencer {
    pub running: bool,
    pub bpm: f64,
    pub step: usize,
    next_time: f64,
    /// Times through the loop so far; the first pass is the build-up.
    pub loops: u32,
}

impl Sequencer {
    pub fn new() -> Self {
        Self { running: false, bpm: BASE_BPM, step: 0, next_time: -1.0, loops: 0 }
    }

    fn step_dur(&self) -> f64 {
        60.0 / self.bpm.max(20.0) / 4.0
    }

    pub fn pump(&mut self, s: &mut Synth) {
        if !self.running {
            return;
        }
        if self.next_time < 0.0 || self.next_time < s.now - 0.25 {
            // First start, or the device stalled: pick the groove up from now rather than catch up.
            self.next_time = s.now + 0.08;
        }
        while self.next_time < s.now + LOOKAHEAD {
            self.play_step(self.step, self.next_time, s);
            self.next_time += self.step_dur();
            self.step += 1;
            if self.step == LOOP_STEPS {
                self.step = 0;
                self.loops += 1;
            }
        }
    }

    fn play_step(&self, step: usize, at: f64, s: &mut Synth) {
        let bar = step / 16;
        let beat = step % 16;
        let chord = BARS[bar];
        let sd = self.step_dur();
        let t = at + if beat % 2 == 1 { sd * SWING } else { 0.0 } + s.rng.next() * 0.003;
        let intensity = s.intensity;
        // The first pass builds: drums and bass, then the arp, then everything.
        let intro = self.loops == 0;
        let arp_on = !intro || bar >= 2;
        let lead_on = !intro || bar >= 4;
        let fill = bar % 4 == 3 && beat >= 12;

        // --- drums
        if beat % 4 == 0 {
            kick(s, t, if beat == 0 { 0.42 } else { 0.36 });
        }
        if beat == 4 || beat == 12 {
            clap(s, t, 0.2);
        }
        if fill {
            snare(s, t, 0.07 + (beat - 12) as f64 * 0.03);
        }
        if beat % 4 == 2 {
            open_hat(s, t, 0.05);
        } else if beat % 2 == 1 {
            hat(s, t, 0.022 + 0.03 * intensity);
        }
        if step == 0 && self.loops > 0 {
            crash(s, t);
        }
        // Danger layers: ghost snares, then a pushing extra kick.
        if intensity > 0.35 && (beat == 7 || beat == 15) && !fill {
            snare(s, t, 0.04 + 0.04 * intensity);
        }
        if intensity > 0.6 && beat == 14 {
            kick(s, t, 0.2);
        }

        // --- bass: bouncing octaves on the eighths, sixteenths when danger is close
        let bass_div = if intensity > 0.7 { 1 } else { 2 };
        if beat % bass_div == 0 {
            let up = (beat / bass_div) % 2 == 1;
            // The bar's last eighth walks toward the next root.
            let next = BARS[(bar + 1) % 8].root;
            let note = if beat == 14 { chord.root + (next - chord.root).signum() * 2.0 } else { chord.root + if up { 12.0 } else { 0.0 } };
            bass(s, t, note, sd * bass_div as f64 * 0.85);
        }

        // --- chord stabs on the one and the and-of-two
        if beat == 0 || beat == 6 || (beat == 10 && self.loops % 2 == 1) {
            stab(s, t, &chord.stab, if beat == 0 { 0.2 } else { 0.13 });
        }

        // --- arpeggio: sixteenths up the chord
        if arp_on {
            let pattern = [0usize, 1, 2, 3, 2, 1, 2, 3];
            let tone = chord.arp[pattern[beat % 8]];
            let vol = 0.028 + 0.02 * intensity;
            arp(s, t, tone, sd * 0.9, vol);
        }

        // --- lead
        if lead_on {
            for &(at_step, note, len) in MELODY[bar] {
                if at_step == beat {
                    let dur = sd * len as f64 * 0.92;
                    lead(s, t, note, dur);
                    // Every other pass, a grace note from below dresses the phrase.
                    if self.loops % 2 == 1 && len >= 2 && s.rng.next() < 0.35 {
                        lead(s, t - sd * 0.25, note - 2.0, sd * 0.22);
                    }
                }
            }
        }
    }
}

/// Punchy sine kick: a fast pitch drop gives it the click, the tail gives it weight.
fn kick(s: &mut Synth, t: f64, vol: f64) {
    s.push(Voice::new(t, t + 0.3, Wave::Sine, Env::ramp(t, 160.0, 45.0, 0.09), Env::ramp(t, vol, 0.001, 0.26), Bus::Music));
}

/// Clap: three quick noise bursts, the way hands never quite land together.
fn clap(s: &mut Synth, t: f64, vol: f64) {
    for (i, d) in [0.0, 0.009, 0.018].into_iter().enumerate() {
        let offset = s.rng.next() * 1.5;
        let v = if i == 2 { vol } else { vol * 0.6 };
        let len = if i == 2 { 0.16 } else { 0.03 };
        s.push(
            Voice::new(t + d, t + d + len + 0.02, Wave::Noise { rate: 1.3, offset }, Env::constant(1.0), Env::ramp(t + d, v, 0.001, len), Bus::Music)
                .filtered(FilterKind::Bandpass, 1500.0, 0.9),
        );
    }
}

fn snare(s: &mut Synth, t: f64, vol: f64) {
    let offset = s.rng.next() * 1.5;
    s.push(
        Voice::new(t, t + 0.12, Wave::Noise { rate: 1.4, offset }, Env::constant(1.0), Env::ramp(t, vol, 0.001, 0.1), Bus::Music)
            .filtered(FilterKind::Bandpass, 1900.0, 0.8),
    );
    s.push(Voice::new(t, t + 0.06, Wave::Triangle, Env::ramp(t, 240.0, 160.0, 0.05), Env::ramp(t, vol * 0.8, 0.001, 0.05), Bus::Music));
}

fn hat(s: &mut Synth, t: f64, vol: f64) {
    let offset = s.rng.next() * 1.5;
    s.push(
        Voice::new(t, t + 0.035, Wave::Noise { rate: 2.4, offset }, Env::constant(1.0), Env::ramp(t, vol, 0.001, 0.03), Bus::Music)
            .filtered(FilterKind::Highpass, 6500.0, 1.0),
    );
}

fn open_hat(s: &mut Synth, t: f64, vol: f64) {
    let offset = s.rng.next() * 1.5;
    s.push(
        Voice::new(t, t + 0.16, Wave::Noise { rate: 2.2, offset }, Env::constant(1.0), Env::ramp(t, vol, 0.001, 0.14), Bus::Music)
            .filtered(FilterKind::Highpass, 5500.0, 1.0),
    );
}

/// A washy crash marking the top of each pass.
fn crash(s: &mut Synth, t: f64) {
    let offset = s.rng.next() * 1.0;
    s.push(
        Voice::new(t, t + 1.1, Wave::Noise { rate: 1.9, offset }, Env::constant(1.0), Env::ramp(t, 0.07, 0.001, 1.0), Bus::Music)
            .filtered(FilterKind::Highpass, 4200.0, 0.7),
    );
}

/// Filtered saw bass, short and plucky so the octaves bounce.
fn bass(s: &mut Synth, t: f64, semitones: f64, dur: f64) {
    let gain = Env::points(vec![(t, 0.0001), (t + 0.006, 0.2), (t + dur, 0.001)]);
    s.push(Voice::new(t, t + dur + 0.03, Wave::Saw, Env::constant(hz(semitones)), gain, Bus::Music).filtered(FilterKind::Lowpass, 750.0, 3.0));
    let sub = Env::points(vec![(t, 0.0001), (t + 0.006, 0.12), (t + dur, 0.001)]);
    s.push(Voice::new(t, t + dur + 0.03, Wave::Sine, Env::constant(hz(semitones)), sub, Bus::Music));
}

/// Bright chord stab, detuned a hair per note so it shimmers.
fn stab(s: &mut Synth, t: f64, voicing: &[f64], vol: f64) {
    for &semi in voicing {
        let cents = (s.rng.next() - 0.5) * 12.0;
        let gain = Env::points(vec![(t, 0.0001), (t + 0.01, vol / voicing.len() as f64), (t + 0.35, 0.001)]);
        s.push(
            Voice::new(t, t + 0.4, Wave::Saw, Env::constant(hz(semi)), gain, Bus::Music)
                .detune_cents(cents)
                .filtered(FilterKind::Lowpass, 2400.0, 1.0),
        );
    }
}

fn arp(s: &mut Synth, t: f64, semitones: f64, dur: f64, vol: f64) {
    let gain = Env::points(vec![(t, 0.0001), (t + 0.004, vol), (t + dur, 0.001)]);
    s.push(
        Voice::new(t, t + dur + 0.02, Wave::Square, Env::constant(hz(semitones)), gain, Bus::Music)
            .filtered(FilterKind::Lowpass, 3000.0, 1.0),
    );
}

/// The lead: square over triangle an octave down, with a little vibrato on held notes.
fn lead(s: &mut Synth, t: f64, semitones: f64, dur: f64) {
    let f = hz(semitones);
    let gain = Env::points(vec![(t, 0.0001), (t + 0.012, 0.075), (t + dur * 0.7, 0.05), (t + dur, 0.001)]);
    s.push(Voice::new(t, t + dur + 0.03, Wave::Square, Env::constant(f), gain, Bus::Music).vibrato(5.5, if dur > 0.2 { 0.004 } else { 0.0 }));
    let body = Env::points(vec![(t, 0.0001), (t + 0.012, 0.06), (t + dur, 0.001)]);
    s.push(Voice::new(t, t + dur + 0.03, Wave::Triangle, Env::constant(f * 0.5), body, Bus::Music));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faster_tempo_packs_more_steps_into_a_second() {
        let count = |bpm: f64| {
            let mut s = Synth::new(48_000.0, 0.42);
            s.music.running = true;
            s.music.bpm = bpm;
            let mut buf = vec![0.0f32; 48_000 * 4];
            s.render(&mut buf, 1);
            s.music.step + s.music.loops as usize * LOOP_STEPS
        };
        assert!(count(BASE_BPM * 1.2) > count(BASE_BPM));
    }

    #[test]
    fn the_mix_stays_under_full_scale_even_in_danger() {
        let mut s = Synth::new(48_000.0, 0.42);
        s.music.running = true;
        s.intensity = 1.0;
        s.music.loops = 1; // every layer on
        let mut buf = vec![0.0f32; 48_000 * 8];
        s.render(&mut buf, 1);
        let peak = buf.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak > 0.02, "silent: {peak}");
        assert!(peak < 0.99, "clipping: {peak}");
    }
}
