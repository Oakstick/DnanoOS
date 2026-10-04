//! The war drum (WARBAND.md §4.6, §6): a synth drum layer over the music, locked to its beat
//! (`Synth::beat_clock`: a recording's measured tempo and downbeat, or the synth tune's own
//! sequencer), its density following the Drum's phase: silent at rest, a heartbeat in the build,
//! full drums at the peak, and on the release a single low hit, then silence.

use super::synth::{Bus, Env, FilterKind, Synth, Voice, Wave};

/// How hard the war drum plays (the Drum's phase, §6.2).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DrumLevel {
    /// Silent.
    #[default]
    Rest,
    /// A heartbeat, twice a bar.
    Build,
    /// Full drums: the great drum on every beat, toms and rims between, a roll every fourth bar.
    Peak,
    /// One low hit on the next beat, then silence.
    Release,
}

/// How far ahead hits are queued (the sequencer's lookahead).
const LOOKAHEAD: f64 = 0.12;

#[derive(Default)]
pub struct WarDrum {
    pub level: DrumLevel,
    /// The last sixteenth queued, counted from the music's first downbeat.
    last: Option<i64>,
    /// The release's one hit is queued.
    released: bool,
}

impl WarDrum {
    pub fn set(&mut self, level: DrumLevel) {
        if level != self.level {
            self.level = level;
            self.released = false;
        }
    }

    /// Queues the hits of the sixteenths about to begin.
    pub fn pump(&mut self, s: &mut Synth) {
        let (bpm, beats) = s.beat_clock();
        let six = 60.0 / bpm.max(20.0) / 4.0;
        let steps = beats * 4.0;
        let first = steps.floor() as i64 + 1;
        // The clock jumped (the recording looped, another song began): count afresh.
        if self.last.is_some_and(|l| l > first + 8 || l < first - 64) {
            self.last = None;
        }
        let mut k = self.last.map_or(first, |l| (l + 1).max(first));
        loop {
            let at = s.now + (k as f64 - steps) * six;
            if at > s.now + LOOKAHEAD {
                break;
            }
            self.hit(k, at, six, s);
            self.last = Some(k);
            k += 1;
        }
    }

    fn hit(&mut self, k: i64, at: f64, six: f64, s: &mut Synth) {
        let step = k.rem_euclid(16);
        let bar = k.div_euclid(16);
        match self.level {
            DrumLevel::Rest => {}
            DrumLevel::Build => {
                if step == 0 || step == 8 {
                    heartbeat(s, at, six * 4.0);
                }
            }
            DrumLevel::Peak => {
                let roll = bar % 4 == 3 && step >= 12;
                match step {
                    0 => great_drum(s, at, 1.0),
                    8 => great_drum(s, at, 0.85),
                    4 | 12 if !roll => great_drum(s, at, 0.55),
                    6 | 14 if !roll => tom(s, at, 1.0),
                    10 if bar % 2 == 1 => tom(s, at, 0.8),
                    _ => {}
                }
                if step % 4 == 2 && !roll {
                    rim(s, at, 0.7);
                }
                if roll {
                    rim(s, at, 0.6 + (step - 12) as f64 * 0.15);
                    tom(s, at, 0.5 + (step - 12) as f64 * 0.15);
                }
            }
            DrumLevel::Release => {
                if !self.released && step % 4 == 0 {
                    self.released = true;
                    low_hit(s, at);
                }
            }
        }
    }
}

/// Lub-dub, the second a third of a beat after the first.
fn heartbeat(s: &mut Synth, t: f64, beat: f64) {
    for (d, f, v) in [(0.0, 64.0, 0.55), (beat / 3.0, 56.0, 0.38)] {
        let at = t + d;
        s.push(Voice::new(at, at + 0.24, Wave::Sine, Env::ramp(at, f * 1.4, f, 0.05), Env::ramp(at, v, 0.001, 0.22), Bus::Music));
        s.push(
            Voice::new(
                at,
                at + 0.08,
                Wave::Hiss { rate: 0.6, offset: 0.3 },
                Env::constant(1.0),
                Env::ramp(at, v * 0.5, 0.001, 0.07),
                Bus::Music,
            )
            .filtered(FilterKind::Lowpass, 220.0, 1.0),
        );
    }
}

/// The great drum (a taiko): a deep skin, its body, and the stick's slap.
fn great_drum(s: &mut Synth, t: f64, v: f64) {
    let f = if v < 0.7 { 74.0 } else { 58.0 };
    s.push(Voice::new(t, t + 0.6, Wave::Sine, Env::ramp(t, f * 1.9, f, 0.06), Env::ramp(t, 0.62 * v, 0.001, 0.56), Bus::Music));
    s.push(
        Voice::new(t, t + 0.16, Wave::Hiss { rate: 0.7, offset: 0.9 }, Env::constant(1.0), Env::ramp(t, 0.5 * v, 0.001, 0.14), Bus::Music)
            .filtered(FilterKind::Lowpass, 480.0, 1.0),
    );
    s.push(
        Voice::new(
            t,
            t + 0.04,
            Wave::Hiss { rate: 1.2, offset: 1.4 },
            Env::constant(1.0),
            Env::ramp(t, 0.25 * v, 0.001, 0.025),
            Bus::Music,
        )
        .filtered(FilterKind::Bandpass, 1700.0, 1.0),
    );
}

fn tom(s: &mut Synth, t: f64, v: f64) {
    s.push(Voice::new(t, t + 0.3, Wave::Sine, Env::ramp(t, 190.0, 118.0, 0.05), Env::ramp(t, 0.3 * v, 0.001, 0.26), Bus::Music));
}

/// A rim or frame-drum tap between the beats.
fn rim(s: &mut Synth, t: f64, v: f64) {
    s.push(
        Voice::new(t, t + 0.06, Wave::Hiss { rate: 1.1, offset: 0.2 }, Env::constant(1.0), Env::ramp(t, 0.3 * v, 0.001, 0.05), Bus::Music)
            .filtered(FilterKind::Bandpass, 1350.0, 1.3),
    );
    s.push(Voice::new(t, t + 0.04, Wave::Triangle, Env::constant(410.0), Env::ramp(t, 0.06 * v, 0.001, 0.03), Bus::Music));
}

/// The release: one low hit that rolls away.
fn low_hit(s: &mut Synth, t: f64) {
    s.push(Voice::new(t, t + 1.9, Wave::Sine, Env::ramp(t, 82.0, 34.0, 0.25), Env::ramp(t, 0.75, 0.001, 1.85), Bus::Music));
    s.push(
        Voice::new(t, t + 1.1, Wave::Hiss { rate: 0.4, offset: 0.6 }, Env::constant(1.0), Env::ramp(t, 0.6, 0.001, 1.0), Bus::Music)
            .filtered(FilterKind::Lowpass, 260.0, 1.0),
    );
    s.push(Voice::new(t, t + 1.0, Wave::Saw, Env::constant(36.0), Env::ramp(t, 0.1, 0.001, 0.95), Bus::Music).filtered(
        FilterKind::Lowpass,
        160.0,
        1.0,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The music bus's energy in each tenth of a second over `secs`, with the war drum at `level`.
    fn energy(level: DrumLevel, secs: f64) -> Vec<f64> {
        let mut s = Synth::new(48_000.0, 0.42);
        s.drone_gain_target = 0.0;
        s.drum.set(level);
        let mut buf = vec![0.0f32; 4_800 * 2];
        (0..(secs * 10.0) as usize)
            .map(|_| {
                s.render(&mut buf, 2);
                buf.iter().map(|v| (*v as f64).powi(2)).sum::<f64>()
            })
            .collect()
    }

    #[test]
    fn the_war_drum_follows_the_drums_phase() {
        // (The threat drone fades out over the first few tenths.)
        let rest: f64 = energy(DrumLevel::Rest, 4.0)[5..].iter().sum();
        let build: f64 = energy(DrumLevel::Build, 4.0).iter().sum();
        let peak: f64 = energy(DrumLevel::Peak, 4.0).iter().sum();
        let release = energy(DrumLevel::Release, 6.0);
        assert!(rest < 1.0, "silent at rest: {rest}");
        assert!(build > rest + 1.0 && peak > build * 1.5, "rest {rest}, build {build}, peak {peak}");
        // One hit, inside the first beat (100 bpm: 0.6 s), then it rolls away to nothing.
        let first = release.iter().position(|&e| e > 1.0).expect("the release hits");
        assert!(first <= 7, "the hit waited {first} tenths");
        assert!(release[release.len() - 10..].iter().all(|&e| e < 0.05), "a single hit, then silence");
    }

    #[test]
    fn the_war_drum_lands_on_the_beat() {
        // The free clock at 100 bpm: beats every 0.6 s from time zero; the build's lub falls on
        // beats 0 and 2 of each bar.
        let mut s = Synth::new(48_000.0, 0.42);
        s.drone_gain_target = 0.0;
        s.drum.set(DrumLevel::Build);
        let mut buf = vec![0.0f32; 48_000 * 5];
        for block in buf.chunks_mut(480) {
            s.render(block, 1);
        }
        let onset =
            |from: f64| (((from * 48_000.0) as usize)..buf.len()).find(|&i| buf[i].abs() > 0.01).map(|i| i as f64 / 48_000.0).unwrap();
        for beat in [2.0, 4.0, 6.0] {
            let t = onset(beat * 0.6 - 0.05);
            assert!((t - beat * 0.6).abs() < 0.01, "beat {beat} sounded at {t}");
        }
    }
}
