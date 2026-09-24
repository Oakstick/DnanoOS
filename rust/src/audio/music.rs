/**
 * Procedural backing track: a Norse war song, built live out of oscillators
 * and noise, so the whole soundtrack costs a few kilobytes.
 *
 * D minor with a modal color: D - D - C - D, Bb - C - A - D, over a hurdy-gurdy
 * drone on D and A that never lets go. Booming war drums gallop under it,
 * stomps and claps mark the backbeat, a lyre plucks the chords, a bone flute
 * carries the tune (climbing to a raised C sharp before it comes home), and a
 * chanting choir swells through the second half. A war horn opens the song.
 *
 * It moves on its own: the first pass builds up layer by layer, every fourth
 * bar ends in a tom roll, and the warband shouts "HEY!" at the turnarounds.
 * The game pushes it too: the tempo rises every five nuggets, danger brings
 * in rolling drums and horn blasts, and low health muffles it all.
 */
use super::synth::{Bus, Env, FilterKind, Synth, Voice, Wave};

/// Beats per minute at the start of a dungeon: a marching war tempo.
pub const BASE_BPM: f64 = 100.0;
/// Sixteenths in the loop: eight bars of four beats.
const LOOP_STEPS: usize = 128;
/// How far ahead notes are queued.
const LOOKAHEAD: f64 = 0.12;

const C4: f64 = 261.626;
fn hz(semitones: f64) -> f64 {
    C4 * 2f64.powf(semitones / 12.0)
}

/// Bass root per bar, in semitones from middle C: D D C D | Bb C A D.
const ROOTS: [f64; 8] = [-22.0, -22.0, -24.0, -22.0, -26.0, -24.0, -15.0, -22.0];
/// Lyre tones per bar: a fifth, the octave and the third, around middle C.
const LYRE: [[f64; 4]; 8] = [
    [2.0, 9.0, 14.0, 17.0],
    [2.0, 9.0, 14.0, 17.0],
    [0.0, 7.0, 12.0, 16.0],
    [2.0, 9.0, 14.0, 17.0],
    [-2.0, 5.0, 10.0, 14.0],
    [0.0, 7.0, 12.0, 16.0],
    [-3.0, 4.0, 9.0, 13.0],
    [2.0, 9.0, 14.0, 17.0],
];

/// The bone flute's tune, per bar: (sixteenth, semitone, length in sixteenths).
const MELODY: [&[(usize, f64, usize)]; 8] = [
    &[(0, 9.0, 4), (4, 12.0, 2), (6, 14.0, 6), (12, 12.0, 2), (14, 9.0, 2)],
    &[(0, 7.0, 4), (4, 9.0, 2), (6, 5.0, 4), (10, 7.0, 2), (12, 9.0, 4)],
    &[(0, 7.0, 3), (3, 4.0, 1), (4, 7.0, 2), (6, 12.0, 4), (10, 9.0, 2), (12, 7.0, 4)],
    &[(0, 9.0, 6), (6, 5.0, 2), (8, 2.0, 8)],
    &[(0, 17.0, 4), (4, 14.0, 2), (6, 12.0, 2), (8, 10.0, 4), (12, 12.0, 4)],
    &[(0, 16.0, 4), (4, 12.0, 2), (6, 14.0, 2), (8, 16.0, 4), (12, 19.0, 4)],
    &[(0, 21.0, 6), (6, 19.0, 2), (8, 16.0, 4), (12, 13.0, 4)],
    &[(0, 14.0, 8), (8, 9.0, 4), (12, 14.0, 4)],
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
            // First start, or the device stalled: pick the song up from now rather than catch up.
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
        let sd = self.step_dur();
        let t = at + s.rng.next() * 0.004;
        let intensity = s.intensity;
        let root = ROOTS[bar];
        // The first pass builds: horn, drone and drums, then the lyre, then the flute and choir.
        let intro = self.loops == 0;
        let lyre_on = !intro || bar >= 2;
        let tune_on = !intro || bar >= 4;
        let fill = bar % 4 == 3 && beat >= 12;

        // --- the war horn opens the song and returns at the top of every pass
        if step == 0 {
            horn(s, t, -10.0, sd * 14.0, 0.16);
        }
        if intensity > 0.65 && step == 64 {
            horn(s, t, -3.0, sd * 8.0, 0.12);
        }

        // --- drone: D and A held under everything, renewed each bar
        if beat == 0 {
            let dur = sd * 16.0 + 0.12;
            drone(s, t, -22.0, dur, 0.05);
            drone(s, t, -15.0, dur, 0.035);
            // Plucked bass under the drone follows the chord.
            bass(s, t, root, sd * 6.0);
        }
        if beat == 8 {
            bass(s, t, root + 7.0, sd * 4.0);
        }

        // --- war drums: a gallop of booms, stomps on the backbeat
        match beat {
            0 | 8 => tom(s, t, 72.0, 0.46),
            6 | 14 if !fill => tom(s, t, 88.0, 0.3),
            3 | 11 => tom(s, t, 105.0, 0.14),
            _ => {}
        }
        if beat == 4 || beat == 12 {
            stomp(s, t, 0.22);
        }
        if beat % 2 == 1 {
            shaker(s, t, 0.018 + 0.03 * intensity);
        }
        if fill {
            // A roll down the toms into the next phrase.
            tom(s, t, 150.0 - (beat - 12) as f64 * 16.0, 0.2 + (beat - 12) as f64 * 0.05);
        }
        // Danger: the drums roll on the sixteenths.
        if intensity > 0.5 && beat % 2 == 1 && !fill {
            tom(s, t, 118.0, 0.06 + 0.08 * intensity);
        }
        // The warband shouts at the turnarounds.
        if (bar == 3 || bar == 7) && beat == 12 && (!intro || intensity > 0.5) {
            shout(s, t);
        }

        // --- lyre: plucked eighth-note arpeggios
        if lyre_on && beat % 2 == 0 {
            let pattern = [0usize, 1, 2, 3, 2, 1, 2, 1];
            let tone = LYRE[bar][pattern[(beat / 2) % 8]];
            lyre(s, t, tone, 0.05 + 0.02 * intensity);
        }

        // --- choir: a chanting swell through the second half of the song
        if tune_on && bar >= 4 && beat == 0 {
            let dur = sd * 16.0 + 0.25;
            for (i, semi) in [root + 24.0, root + 31.0, root + 36.0].into_iter().enumerate() {
                choir(s, t, semi, dur, if i == 0 { 0.07 } else { 0.05 });
            }
        }

        // --- bone flute: the tune
        if tune_on {
            for &(at_step, note, len) in MELODY[bar] {
                if at_step == beat {
                    flute(s, t, note + 12.0, sd * len as f64 * 0.95);
                    // Every other pass, a grace note from above ornaments the long notes.
                    if self.loops % 2 == 1 && len >= 4 && s.rng.next() < 0.5 {
                        flute(s, t - sd * 0.3, note + 14.0, sd * 0.28);
                    }
                }
            }
        }
    }
}

/// War drum: a deep sine boom falling in pitch, with a skin slap of noise on top.
fn tom(s: &mut Synth, t: f64, freq: f64, vol: f64) {
    s.push(Voice::new(t, t + 0.5, Wave::Sine, Env::ramp(t, freq * 1.6, freq, 0.07), Env::ramp(t, vol, 0.001, 0.45), Bus::Music));
    let offset = s.rng.next() * 1.5;
    s.push(
        Voice::new(t, t + 0.08, Wave::Noise { rate: 0.8, offset }, Env::constant(1.0), Env::ramp(t, vol * 0.4, 0.001, 0.06), Bus::Music)
            .filtered(FilterKind::Lowpass, 900.0, 1.0),
    );
}

/// A stomp on wooden boards and a clap together.
fn stomp(s: &mut Synth, t: f64, vol: f64) {
    s.push(Voice::new(t, t + 0.2, Wave::Sine, Env::ramp(t, 140.0, 60.0, 0.05), Env::ramp(t, vol * 1.2, 0.001, 0.16), Bus::Music));
    for (i, d) in [0.0, 0.011, 0.02].into_iter().enumerate() {
        let offset = s.rng.next() * 1.5;
        let v = if i == 2 { vol } else { vol * 0.55 };
        let len = if i == 2 { 0.14 } else { 0.03 };
        s.push(
            Voice::new(t + d, t + d + len + 0.02, Wave::Noise { rate: 1.2, offset }, Env::constant(1.0), Env::ramp(t + d, v, 0.001, len), Bus::Music)
                .filtered(FilterKind::Bandpass, 1150.0, 0.9),
        );
    }
}

/// A bone rattle on the offbeats.
fn shaker(s: &mut Synth, t: f64, vol: f64) {
    let offset = s.rng.next() * 1.5;
    s.push(
        Voice::new(t, t + 0.05, Wave::Noise { rate: 2.2, offset }, Env::constant(1.0), Env::ramp(t, vol, 0.001, 0.045), Bus::Music)
            .filtered(FilterKind::Highpass, 5000.0, 1.0),
    );
}

/// The warband's "HEY!": a few rough voices through a vowel formant.
fn shout(s: &mut Synth, t: f64) {
    for (i, f) in [165.0, 196.0, 220.0].into_iter().enumerate() {
        let d = i as f64 * 0.012;
        let gain = Env::points(vec![(t + d, 0.0001), (t + d + 0.02, 0.09), (t + d + 0.2, 0.001)]);
        s.push(
            Voice::new(t + d, t + d + 0.24, Wave::Saw, Env::points(vec![(t + d, f * 1.08), (t + d + 0.18, f * 0.9)]), gain, Bus::Music)
                .filtered(FilterKind::Bandpass, 760.0, 1.4),
        );
    }
    let offset = s.rng.next() * 1.5;
    s.push(
        Voice::new(t, t + 0.18, Wave::Noise { rate: 1.0, offset }, Env::constant(1.0), Env::ramp(t, 0.06, 0.001, 0.16), Bus::Music)
            .filtered(FilterKind::Bandpass, 1400.0, 1.2),
    );
}

/// Hurdy-gurdy drone: a buzzing saw, softened, with a slow swell and a slight beat.
fn drone(s: &mut Synth, t: f64, semitones: f64, dur: f64, vol: f64) {
    let gain = Env::points(vec![(t, vol * 0.7), (t + 0.08, vol), (t + dur - 0.1, vol * 0.9), (t + dur, 0.001)]);
    s.push(
        Voice::new(t, t + dur + 0.02, Wave::Saw, Env::constant(hz(semitones)), gain, Bus::Music)
            .detune_cents(3.0)
            .filtered(FilterKind::Lowpass, 850.0, 2.0),
    );
}

/// A round plucked bass note under the drone.
fn bass(s: &mut Synth, t: f64, semitones: f64, dur: f64) {
    let gain = Env::points(vec![(t, 0.0001), (t + 0.008, 0.2), (t + dur, 0.001)]);
    s.push(Voice::new(t, t + dur + 0.03, Wave::Triangle, Env::constant(hz(semitones)), gain, Bus::Music));
}

/// The lyre: a bright pluck that rings a moment, with a soft octave shimmer.
fn lyre(s: &mut Synth, t: f64, semitones: f64, vol: f64) {
    let f = hz(semitones);
    s.push(Voice::new(t, t + 0.7, Wave::Triangle, Env::constant(f), Env::points(vec![(t, 0.0001), (t + 0.003, vol), (t + 0.65, 0.001)]), Bus::Music));
    s.push(Voice::new(t, t + 0.35, Wave::Sine, Env::constant(f * 2.0), Env::ramp(t, vol * 0.35, 0.001, 0.3), Bus::Music));
}

/// The bone flute: a breathy sine with vibrato and a puff of air on each note.
fn flute(s: &mut Synth, t: f64, semitones: f64, dur: f64) {
    let f = hz(semitones);
    let gain = Env::points(vec![(t, 0.0001), (t + 0.04, 0.085), (t + dur * 0.8, 0.07), (t + dur, 0.001)]);
    s.push(Voice::new(t, t + dur + 0.03, Wave::Sine, Env::constant(f), gain, Bus::Music).vibrato(5.2, if dur > 0.25 { 0.007 } else { 0.0 }));
    let body = Env::points(vec![(t, 0.0001), (t + 0.04, 0.02), (t + dur, 0.001)]);
    s.push(Voice::new(t, t + dur + 0.03, Wave::Triangle, Env::constant(f * 2.0), body, Bus::Music));
    let offset = s.rng.next() * 1.5;
    s.push(
        Voice::new(t, t + 0.08, Wave::Noise { rate: 1.0, offset }, Env::constant(1.0), Env::ramp(t, 0.03, 0.001, 0.07), Bus::Music)
            .filtered(FilterKind::Bandpass, 2400.0, 1.5),
    );
}

/// The war horn: brassy saws sliding up into the note, swelling slowly, with a fifth above.
fn horn(s: &mut Synth, t: f64, semitones: f64, dur: f64, vol: f64) {
    for (semi, v) in [(semitones, vol), (semitones + 7.0, vol * 0.6), (semitones - 12.0, vol * 0.5)] {
        let f = hz(semi);
        let gain = Env::points(vec![(t, 0.0001), (t + 0.28, v), (t + dur * 0.75, v * 0.8), (t + dur, 0.001)]);
        s.push(
            Voice::new(t, t + dur + 0.03, Wave::Saw, Env::points(vec![(t, f * 0.94), (t + 0.2, f)]), gain, Bus::Music)
                .filtered(FilterKind::Lowpass, 950.0, 1.5)
                .vibrato(4.5, 0.004),
        );
    }
}

/// Chanting choir: soft saws through an "aah" formant, swelling in, trembling slightly.
fn choir(s: &mut Synth, t: f64, semitones: f64, dur: f64, vol: f64) {
    let cents = (s.rng.next() - 0.5) * 10.0;
    let gain = Env::points(vec![(t, 0.0001), (t + 0.6, vol), (t + dur - 0.2, vol * 0.85), (t + dur, 0.001)]);
    s.push(
        Voice::new(t, t + dur + 0.03, Wave::Saw, Env::constant(hz(semitones)), gain, Bus::Music)
            .detune_cents(cents)
            .filtered(FilterKind::Bandpass, 850.0, 1.8)
            .vibrato(5.0, 0.005),
    );
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
        let mut buf = vec![0.0f32; 48_000 * 24];
        for block in buf.chunks_mut(480) {
            s.render(block, 1);
        }
        let peak = buf.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak > 0.02, "silent: {peak}");
        assert!(peak < 0.99, "clipping: {peak}");
    }

    #[test]
    fn the_tune_stays_in_d_minor() {
        // D natural minor plus the raised C sharp that leads home.
        let scale = [2, 4, 5, 7, 9, 10, 12, 13];
        for bar in MELODY {
            for &(_, note, _) in bar.iter() {
                let pc = (note as i32).rem_euclid(12);
                assert!(scale.iter().any(|&s| s % 12 == pc), "note {note} outside the mode");
            }
        }
    }
}
