//! Recorded soundtracks: Ogg Vorbis files built into the binary, decoded a
//! packet at a time inside the audio callback (no threads, so it works the
//! same on the web), resampled to the device rate and looped seamlessly.

use std::collections::VecDeque;
use std::io::Cursor;

use lewton::inside_ogg::OggStreamReader;

/// Which recording plays; `None` in the audio manager means the synth's own tune.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Soundtrack {
    /// "The New Adventure": dungeon I.
    Level1,
    /// The title screen and the blacksmith.
    Menu,
    /// The Cyclops fight.
    Boss,
    /// "Bastion Lux: Enter": one of the songs the later levels draw from.
    BastionLux,
    /// "The Last One".
    LastOne,
    /// "Desolation Vesper".
    DesolationVesper,
    /// "Level 1 Boss": the other song a boss fight may draw.
    LevelOneBoss,
}

impl Soundtrack {
    fn bytes(self) -> &'static [u8] {
        match self {
            Soundtrack::Level1 => include_bytes!("../../assets/music/level1.ogg"),
            Soundtrack::Menu => include_bytes!("../../assets/music/menu.ogg"),
            Soundtrack::Boss => include_bytes!("../../assets/music/boss.ogg"),
            Soundtrack::BastionLux => include_bytes!("../../assets/music/bastion_lux.ogg"),
            Soundtrack::LastOne => include_bytes!("../../assets/music/last_one.ogg"),
            Soundtrack::DesolationVesper => include_bytes!("../../assets/music/desolation_vesper.ogg"),
            Soundtrack::LevelOneBoss => include_bytes!("../../assets/music/level1_boss.ogg"),
        }
    }

    /// The songs every level after dungeon I draws one of at random (the hub, the realms, the vaults,
    /// the later dungeons and the gym).
    pub const LEVELS: [Soundtrack; 3] = [Soundtrack::BastionLux, Soundtrack::LastOne, Soundtrack::DesolationVesper];
    /// The songs a boss fight draws one of at random.
    pub const BOSSES: [Soundtrack; 2] = [Soundtrack::Boss, Soundtrack::LevelOneBoss];

    /// Playback gain that sits the recording at the synth tune's loudness under the effects.
    pub fn gain(self) -> f64 {
        match self {
            Soundtrack::Level1 => 1.0,
            Soundtrack::Menu => 0.85,
            Soundtrack::Boss => 0.8,
            Soundtrack::BastionLux => 0.74,
            Soundtrack::LastOne => 0.84,
            Soundtrack::DesolationVesper => 1.4,
            Soundtrack::LevelOneBoss => 0.87,
        }
    }
}

pub struct Track {
    pub which: Soundtrack,
    reader: Option<OggStreamReader<Cursor<&'static [u8]>>>,
    rate: f64,
    channels: usize,
    pending: VecDeque<(f32, f32)>,
    /// The two source frames being interpolated between, and how far along.
    a: (f32, f32),
    b: (f32, f32),
    frac: f64,
}

impl Track {
    pub fn new(which: Soundtrack) -> Option<Self> {
        let reader = OggStreamReader::new(Cursor::new(which.bytes())).ok()?;
        let rate = reader.ident_hdr.audio_sample_rate as f64;
        let channels = reader.ident_hdr.audio_channels as usize;
        let mut t = Self { which, reader: Some(reader), rate, channels, pending: VecDeque::new(), a: (0.0, 0.0), b: (0.0, 0.0), frac: 0.0 };
        t.a = t.pull();
        t.b = t.pull();
        Some(t)
    }

    /// The next source frame, decoding more when needed and starting over at the end.
    fn pull(&mut self) -> (f32, f32) {
        let mut restarts = 0;
        while self.pending.is_empty() {
            let packet = self.reader.as_mut().and_then(|r| r.read_dec_packet_itl().ok().flatten());
            match packet {
                Some(samples) => {
                    let ch = self.channels.max(1);
                    for f in samples.chunks(ch) {
                        let l = f[0] as f32 / 32768.0;
                        let r = if ch > 1 { f[1] as f32 / 32768.0 } else { l };
                        self.pending.push_back((l, r));
                    }
                }
                None => {
                    // The end: loop from the top (the recording runs right up to both ends).
                    restarts += 1;
                    if restarts > 1 {
                        return (0.0, 0.0);
                    }
                    self.reader = OggStreamReader::new(Cursor::new(self.which.bytes())).ok();
                }
            }
        }
        self.pending.pop_front().unwrap_or((0.0, 0.0))
    }

    /// One output frame at `out_rate`, linearly resampled.
    pub fn next(&mut self, out_rate: f64) -> (f32, f32) {
        let k = self.frac as f32;
        let out = (self.a.0 + (self.b.0 - self.a.0) * k, self.a.1 + (self.b.1 - self.a.1) * k);
        self.frac += self.rate / out_rate;
        while self.frac >= 1.0 {
            self.frac -= 1.0;
            self.a = self.b;
            self.b = self.pull();
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_one_decodes_and_resamples() {
        let mut t = Track::new(Soundtrack::Level1).expect("the soundtrack decodes");
        assert_eq!(t.rate, 48_000.0);
        assert_eq!(t.channels, 2);
        // Two seconds at a 44.1 kHz device: real signal, never out of range.
        let mut energy = 0.0f64;
        for _ in 0..88_200 {
            let (l, r) = t.next(44_100.0);
            assert!(l.abs() <= 1.0 && r.abs() <= 1.0);
            energy += (l * l + r * r) as f64;
        }
        assert!(energy > 100.0, "the track is silent: {energy}");
    }

    #[test]
    fn every_recording_decodes() {
        for which in Soundtrack::LEVELS.into_iter().chain(Soundtrack::BOSSES) {
            let mut t = Track::new(which).unwrap_or_else(|| panic!("{which:?} decodes"));
            let energy: f64 = (0..48_000).map(|_| t.next(48_000.0)).map(|(l, r)| (l * l + r * r) as f64).sum();
            assert!(energy > 0.0, "{which:?} is silent");
        }
    }
}

/// Calibration for `Soundtrack::gain`: `cargo test --lib loudness -- --ignored --nocapture`
/// prints how loud a recording plays next to the synth tune (level 1 matches it at gain 1).
#[cfg(test)]
mod loudness {
    use super::*;
    use crate::audio::synth::Synth;

    fn rms(s: &mut Synth, secs: usize) -> f64 {
        let mut buf = vec![0.0f32; 48_000 * 2];
        let mut sum = 0.0f64;
        let mut n = 0usize;
        for i in 0..secs {
            s.render(&mut buf, 2);
            if i >= 2 {
                sum += buf.iter().map(|v| (*v as f64).powi(2)).sum::<f64>();
                n += buf.len();
            }
        }
        (sum / n as f64).sqrt()
    }

    #[test]
    #[ignore]
    fn measure() {
        let mut a = Synth::new(48_000.0, 0.42);
        a.music.running = true;
        a.music.loops = 1;
        a.intensity = 0.3;
        let synth = rms(&mut a, 20);
        for which in [Soundtrack::Level1, Soundtrack::Menu].into_iter().chain(Soundtrack::LEVELS).chain(Soundtrack::BOSSES) {
            let mut b = Synth::new(48_000.0, 0.42);
            b.track = Track::new(which);
            b.track_target = 1.0;
            b.track_level = 1.0;
            let track = rms(&mut b, 20);
            println!("{which:?}: synth tune rms {synth:.4}, track rms {track:.4}, ratio {:.3}", synth / track);
        }
    }
}
