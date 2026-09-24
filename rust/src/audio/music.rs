/**
 * Procedural lo-fi backing track. Nothing is sampled: the sequencer walks a
 * four-bar loop of sixteenths and builds each hit out of oscillators and
 * noise. What makes it lo-fi rather than chiptune is the treatment around
 * the notes: one gentle warbling low-pass over everything, swung offbeats,
 * detuned chords, and a sparse crackle bed. I-vi-IV-V in C with sevenths
 * and ninths.
 *
 * The tempo is live: the game nudges it up as the hoard grows, and the new
 * tempo takes over from the next sixteenth, so the groove never stumbles.
 */
use super::synth::{Bus, Env, FilterKind, Synth, Voice, Wave};

/// Beats per minute at the start of a dungeon.
pub const BASE_BPM: f64 = 78.0;
/// Sixteenths in the loop: four bars of four beats.
const LOOP_STEPS: usize = 64;
/// How far ahead notes are queued.
const LOOKAHEAD: f64 = 0.12;
/// How late an offbeat sixteenth lands, as a fraction of a sixteenth.
const SWING: f64 = 0.34;

const C4: f64 = 261.626;
fn hz(semitones: f64) -> f64 {
    C4 * 2f64.powf(semitones / 12.0)
}

struct Chord {
    /// Bass root, down in the second octave.
    root: f64,
    /// Upper voicing, sevenths and ninths rather than plain triads.
    voicing: [f64; 4],
    /// Notes the melody may pick from over this chord.
    scale: [f64; 5],
}

/// I - vi - IV - V in C major, a bar each.
const PROGRESSION: [Chord; 4] = [
    Chord { root: -24.0, voicing: [4.0, 7.0, 11.0, 14.0], scale: [12.0, 16.0, 19.0, 21.0, 24.0] }, // Cmaj9
    Chord { root: -15.0, voicing: [7.0, 12.0, 16.0, 19.0], scale: [12.0, 14.0, 16.0, 19.0, 21.0] }, // Am7
    Chord { root: -19.0, voicing: [9.0, 12.0, 16.0, 21.0], scale: [12.0, 14.0, 17.0, 19.0, 21.0] }, // Fmaj9
    Chord { root: -17.0, voicing: [5.0, 11.0, 14.0, 17.0], scale: [11.0, 14.0, 17.0, 19.0, 23.0] }, // G7
];

#[derive(Default)]
pub struct Sequencer {
    pub running: bool,
    pub bpm: f64,
    step: usize,
    next_time: f64,
}

impl Sequencer {
    pub fn new() -> Self {
        Self { running: false, bpm: BASE_BPM, step: 0, next_time: -1.0 }
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
            self.step = (self.step + 1) % LOOP_STEPS;
        }
    }

    fn play_step(&self, step: usize, at: f64, s: &mut Synth) {
        let chord = &PROGRESSION[step / 16];
        let beat = step % 16;
        let sd = self.step_dur();
        // Offbeat sixteenths land late; that lilt is the whole feel of the genre.
        let t = at + if beat % 4 == 2 { sd * SWING } else { 0.0 } + s.rng.next() * 0.006;

        if beat == 0 || beat == 8 {
            kick(s, t, if beat == 0 { 0.42 } else { 0.34 });
        }
        if beat == 4 || beat == 12 {
            snare(s, t);
        }
        if beat == 14 && s.rng.next() < 0.4 {
            kick(s, t, 0.2);
        }
        if beat % 2 == 0 {
            hat(s, t, if beat % 4 == 0 { 0.07 } else { 0.045 });
        }
        // Intensity layers: as enemies close in, the offbeat sixteenths fill with
        // hats, then ghost kicks push the groove forward.
        let intensity = s.intensity;
        if intensity > 0.3 && beat % 2 == 1 {
            hat(s, t, 0.02 + 0.035 * intensity);
        }
        if intensity > 0.6 && (beat == 3 || beat == 11) {
            kick(s, t, 0.12 + 0.1 * intensity);
        }
        if beat == 0 {
            bass(s, t, chord.root, sd * 3.0);
            keys(s, t, &chord.voicing, 0.3);
        }
        if beat == 6 {
            keys(s, t, &chord.voicing, 0.17);
        }
        if beat == 10 {
            bass(s, t, chord.root + 7.0, sd * 2.0);
        }
        if beat == 14 {
            bass(s, t, chord.root + 12.0, sd);
        }
        // A sparse melody, picked fresh each bar so the loop never quite repeats.
        if (beat == 2 || beat == 7 || beat == 11) && s.rng.next() < 0.45 {
            let note = chord.scale[s.rng.index(chord.scale.len())];
            lead(s, t, note);
        }
    }
}

/// Soft sine thump with the pitch dropping away under it.
fn kick(s: &mut Synth, t: f64, vol: f64) {
    s.push(Voice::new(t, t + 0.25, Wave::Sine, Env::ramp(t, 125.0, 42.0, 0.11), Env::ramp(t, vol, 0.001, 0.22), Bus::Music));
}

/// Brushed backbeat: a band of noise, short and dry.
fn snare(s: &mut Synth, t: f64) {
    let offset = s.rng.next() * 1.5;
    s.push(
        Voice::new(t, t + 0.2 / 1.4, Wave::Noise { rate: 1.4, offset }, Env::constant(1.0), Env::ramp(t, 0.22, 0.001, 0.13), Bus::Music)
            .filtered(FilterKind::Bandpass, 1700.0, 0.8),
    );
}

fn hat(s: &mut Synth, t: f64, vol: f64) {
    let offset = s.rng.next() * 1.5;
    // Stays under the tape filter above, or the hats vanish into it entirely.
    s.push(
        Voice::new(t, t + 0.08 / 2.4, Wave::Noise { rate: 2.4, offset }, Env::constant(1.0), Env::ramp(t, vol, 0.001, 0.05), Bus::Music)
            .filtered(FilterKind::Highpass, 3000.0, 1.0),
    );
}

/// Round triangle bass, well below everything else.
fn bass(s: &mut Synth, t: f64, semitones: f64, dur: f64) {
    let gain = Env::points(vec![(t, 0.0001), (t + 0.02, 0.2), (t + dur, 0.001)]);
    s.push(Voice::new(t, t + dur + 0.05, Wave::Triangle, Env::constant(hz(semitones)), gain, Bus::Music));
}

/// The chord itself: each note detuned a little, which is what blurs it.
fn keys(s: &mut Synth, t: f64, voicing: &[f64], vol: f64) {
    for &semi in voicing {
        let cents = (s.rng.next() - 0.5) * 14.0;
        let dur = 1.1 + s.rng.next() * 0.3;
        let gain = Env::points(vec![(t, 0.0001), (t + 0.05, vol / voicing.len() as f64), (t + dur, 0.001)]);
        s.push(Voice::new(t, t + dur + 0.05, Wave::Triangle, Env::constant(hz(semi)), gain, Bus::Music).detune_cents(cents));
    }
}

fn lead(s: &mut Synth, t: f64, semitones: f64) {
    let cents = (s.rng.next() - 0.5) * 10.0;
    let dur = 0.4 + s.rng.next() * 0.4;
    let gain = Env::points(vec![(t, 0.0001), (t + 0.03, 0.15), (t + dur, 0.001)]);
    s.push(Voice::new(t, t + dur + 0.05, Wave::Sine, Env::constant(hz(semitones)), gain, Bus::Music).detune_cents(cents));
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
            s.music.step
        };
        assert!(count(BASE_BPM * 1.2) > count(BASE_BPM));
    }
}
