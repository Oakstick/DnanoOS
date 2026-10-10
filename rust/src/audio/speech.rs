//! The spoken lines (B10): a warmer, more expressive synth speech. No words are recorded; a line
//! typing out is spoken in babble, syllable by syllable, each one a buzz through the two formants of
//! its vowel, its consonant before it, in the speaker's own voice and the line's mood.
//!
//! - **Who** (`Talker`): every speaker has a timbre of his own (`Timbre`): the three boys apart
//!   (Mehmed's boy steady and middle-pitched, Ragnar's boy punchy and lower, Miyamoto's boy high,
//!   soft and breathy), Toby highest and brightest, Hızır low, slow and reedy with an old man's
//!   tremor, each prince his own (Lucifer bright and proud, Leviathan hissing, Amon rasping, Belphegor
//!   slow, Mammon nasal, Beelzebub wet and deep, Asmodeus whispering), Krampus a growl an octave under.
//! - **How** (`Mood`): fear, anger, resolve, warmth, gravity, grief, menace and joy move the pitch,
//!   its range, the pace, the loudness, the breath and a tremble.
//! - **Prosody**: a sentence falls as it goes (declination); a question rises at its end, a full stop
//!   falls, an exclamation peaks and is louder; a word in capitals is stressed (higher, longer,
//!   louder). The pitch wanders a little from syllable to syllable, the same way every time.
//!
//! The text decides the syllables (`syllable_at`); the synth plays them (`AudioManager::speak`).

use super::synth::{Bus, Env, FilterKind, Synth, Voice, Wave};
use crate::logic::sins::Sin;

/// Who is speaking.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Talker {
    /// One of the three boys, by the cast's kid he is (1 Mehmed's, 2 Ragnar's, 3 Miyamoto's).
    Kid(u8),
    /// The three boys as one (Scene I).
    Boys,
    Toby,
    Hizir,
    /// Quitmer's snarl.
    Dog,
    Narrator,
    /// A prince, whispering in a realm or speaking in his court.
    Prince(Sin),
    Krampus,
}

/// How a line is said.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Mood {
    Plain,
    Fear,
    Anger,
    Resolve,
    Warmth,
    Grave,
    Grief,
    Menace,
    Joy,
}

impl Mood {
    /// The mood a stage direction (a beat's cue) asks for, else what the line itself says: a line
    /// shouted (an exclamation in capitals) is fear or anger by the speaker, a question plain.
    pub fn of(cue: &str, line: &str) -> Mood {
        let cue = cue.to_lowercase();
        let has = |w: &[&str]| w.iter().any(|w| cue.contains(w));
        if has(&["scream", "terrif", "breathless", "afraid", "trembl", "scared"]) {
            Mood::Fear
        } else if has(&["angr", "furious", "snarl"]) {
            Mood::Anger
        } else if has(&["firm", "determin", "resolute", "fierce"]) {
            Mood::Resolve
        } else if has(&["gently", "warm", "kindly", "softly", "smil", "fond"]) {
            Mood::Warmth
        } else if has(&["grave", "heavily", "hoarse", "solemn"]) {
            Mood::Grave
        } else if has(&["looking away", "quietly", "sad", "sorrow", "tears"]) {
            Mood::Grief
        } else if has(&["menac", "cold", "whisper"]) {
            Mood::Menace
        } else if has(&["joy", "laugh", "grin", "beaming", "delight", "cheer"]) {
            Mood::Joy
        } else if line.contains('!') && line.chars().filter(|c| c.is_ascii_uppercase()).count() > 6 {
            Mood::Resolve
        } else {
            Mood::Plain
        }
    }

    /// Its pitch, range, length of syllable, loudness, extra breath and tremble (a vibrato depth).
    fn shape(self) -> (f64, f64, f64, f64, f64, f64) {
        match self {
            Mood::Plain => (1.0, 1.0, 1.0, 1.0, 0.0, 0.0),
            Mood::Fear => (1.16, 1.45, 0.84, 1.05, 0.12, 0.035),
            Mood::Anger => (1.08, 1.25, 0.9, 1.3, 0.04, 0.0),
            Mood::Resolve => (0.97, 0.8, 1.0, 1.12, 0.0, 0.0),
            Mood::Warmth => (0.96, 0.9, 1.15, 0.85, 0.06, 0.0),
            Mood::Grave => (0.91, 0.7, 1.25, 0.92, 0.04, 0.0),
            Mood::Grief => (0.95, 0.6, 1.3, 0.72, 0.16, 0.02),
            Mood::Menace => (0.84, 0.6, 1.2, 1.0, 0.1, 0.0),
            Mood::Joy => (1.12, 1.5, 0.9, 1.1, 0.03, 0.0),
        }
    }
}

/// A voice's make: its pitch and the scale of its formants (a child's are higher), the source's
/// wave, how much breath goes with it, its vibrato, a syllable's length, the pitch's range over a
/// sentence, a growl of noise, an octave beneath, and its loudness.
#[derive(Clone, Copy, Debug)]
pub struct Timbre {
    pub f0: f64,
    pub formant: f64,
    pub wave: Wave,
    pub breath: f64,
    pub vib: (f64, f64),
    pub syll: f64,
    pub range: f64,
    pub rasp: f64,
    pub sub: f64,
    pub gain: f64,
}

const fn timbre(f0: f64, formant: f64, wave: Wave, breath: f64, vib: (f64, f64), syll: f64, range: f64) -> Timbre {
    Timbre { f0, formant, wave, breath, vib, syll, range, rasp: 0.0, sub: 0.0, gain: 1.0 }
}

impl Talker {
    pub fn timbre(self) -> Timbre {
        match self {
            // Steady, middle of the three.
            Talker::Kid(1) => timbre(292.0, 1.2, Wave::Saw, 0.08, (5.2, 0.01), 0.075, 0.22),
            // Punchy and lower: the one who screamed "He took him!".
            Talker::Kid(2) => Timbre { gain: 1.12, ..timbre(258.0, 1.15, Wave::Saw, 0.04, (5.8, 0.008), 0.068, 0.3) },
            // High, soft and breathy.
            Talker::Kid(_) => timbre(338.0, 1.3, Wave::Triangle, 0.2, (6.4, 0.014), 0.082, 0.18),
            Talker::Boys => Timbre { gain: 1.1, ..timbre(285.0, 1.2, Wave::Saw, 0.08, (5.6, 0.012), 0.072, 0.28) },
            Talker::Toby => timbre(372.0, 1.34, Wave::Saw, 0.1, (6.0, 0.012), 0.07, 0.32),
            // Low, slow and reedy, with an old man's tremor.
            Talker::Hizir => Timbre { sub: 0.15, ..timbre(112.0, 0.92, Wave::Saw, 0.22, (4.2, 0.022), 0.11, 0.15) },
            Talker::Dog => Timbre { rasp: 0.8, sub: 0.6, ..timbre(62.0, 0.7, Wave::Saw, 0.3, (0.0, 0.0), 0.09, 0.1) },
            Talker::Narrator => timbre(165.0, 1.0, Wave::Triangle, 0.06, (0.0, 0.0), 0.1, 0.1),
            Talker::Prince(sin) => match sin {
                Sin::Pride => timbre(150.0, 1.08, Wave::Square, 0.05, (5.0, 0.006), 0.1, 0.24),
                Sin::Envy => timbre(128.0, 1.0, Wave::Saw, 0.5, (9.0, 0.02), 0.09, 0.2),
                Sin::Wrath => Timbre { rasp: 0.35, gain: 1.15, ..timbre(92.0, 0.9, Wave::Saw, 0.1, (0.0, 0.0), 0.085, 0.3) },
                Sin::Sloth => timbre(84.0, 0.88, Wave::Triangle, 0.3, (2.5, 0.015), 0.17, 0.08),
                Sin::Greed => timbre(108.0, 1.12, Wave::Square, 0.08, (4.0, 0.008), 0.095, 0.18),
                Sin::Gluttony => Timbre { sub: 0.5, rasp: 0.15, ..timbre(72.0, 0.82, Wave::Saw, 0.15, (3.0, 0.012), 0.12, 0.15) },
                Sin::Lust => timbre(156.0, 1.06, Wave::Triangle, 0.75, (5.5, 0.01), 0.11, 0.2),
            },
            Talker::Krampus => Timbre { rasp: 0.5, sub: 0.7, gain: 1.15, ..timbre(64.0, 0.78, Wave::Saw, 0.2, (0.0, 0.0), 0.12, 0.25) },
        }
    }
}

/// The vowels, by their first two formants (an adult man's; a voice scales them).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Vowel {
    A,
    E,
    I,
    O,
    U,
}

impl Vowel {
    fn formants(self) -> (f64, f64) {
        match self {
            Vowel::A => (730.0, 1090.0),
            Vowel::E => (530.0, 1840.0),
            Vowel::I => (300.0, 2290.0),
            Vowel::O => (570.0, 840.0),
            Vowel::U => (330.0, 900.0),
        }
    }

    fn of(c: char) -> Option<Vowel> {
        match c.to_ascii_lowercase() {
            'a' => Some(Vowel::A),
            'e' => Some(Vowel::E),
            'i' | 'y' | 'ı' | 'İ' => Some(Vowel::I),
            'o' | 'ö' | 'Ö' => Some(Vowel::O),
            'u' | 'ü' | 'Ü' => Some(Vowel::U),
            _ => None,
        }
    }
}

/// The sound before the vowel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Onset {
    None,
    /// A hiss (s, f, h, sh).
    Hiss,
    /// A stop (t, k, p, d, g, b, c).
    Stop,
    /// A hum (m, n, l, r, w, v).
    Hum,
}

/// One syllable of a line as it is spoken.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Syllable {
    pub vowel: Vowel,
    pub onset: Onset,
    /// How far through its sentence (0 its start, 1 its end).
    pub pos: f64,
    /// In its sentence's last word, and how the sentence ends ('.', '?', '!', or ' ').
    pub last: bool,
    pub end: char,
    /// A word in capitals, or the first syllable of a shouted sentence's words.
    pub stress: bool,
    /// Its place in the line (the pitch's wander).
    pub n: usize,
}

/// The syllable that begins at character `i` of `line`, if one does: a vowel that does not follow
/// another vowel of the same word.
pub fn syllable_at(line: &str, i: usize) -> Option<Syllable> {
    let chars: Vec<char> = line.chars().collect();
    let c = *chars.get(i)?;
    let vowel = Vowel::of(c)?;
    if i > 0 && Vowel::of(chars[i - 1]).is_some() {
        return None;
    }
    let onset = match i.checked_sub(1).map(|k| chars[k].to_ascii_lowercase()) {
        Some('s' | 'f' | 'h' | 'z' | 'x' | 'j' | 'ş' | 'ç') => Onset::Hiss,
        Some('t' | 'k' | 'p' | 'd' | 'g' | 'b' | 'c' | 'q') => Onset::Stop,
        Some('m' | 'n' | 'l' | 'r' | 'w' | 'v') => Onset::Hum,
        _ => Onset::None,
    };
    let stop = |c: char| matches!(c, '.' | '?' | '!');
    let start = (0..i).rev().find(|&k| stop(chars[k])).map(|k| k + 1).unwrap_or(0);
    let end_at = (i..chars.len()).find(|&k| stop(chars[k]));
    let end = end_at.map(|k| chars[k]).unwrap_or(' ');
    let stop_at = end_at.unwrap_or(chars.len());
    let pos = (i - start) as f64 / (stop_at - start).max(1) as f64;
    // The last word: no space between here and the sentence's end.
    let last = (i..stop_at).all(|k| chars[k] != ' ');
    let word_start = (0..i).rev().find(|&k| !chars[k].is_alphabetic()).map(|k| k + 1).unwrap_or(0);
    let word_end = (i..chars.len()).find(|&k| !chars[k].is_alphabetic()).unwrap_or(chars.len());
    let word = &chars[word_start..word_end];
    let caps = word.len() >= 2 && word.iter().all(|c| !c.is_lowercase());
    let n = chars[..i].iter().filter(|c| Vowel::of(**c).is_some()).count();
    Some(Syllable { vowel, onset, pos, last, end, stress: caps || (end == '!' && word_start == i.saturating_sub(1)), n })
}

/// The syllables a typewriter's step through `line` (characters `from` to `to`) begins.
pub fn syllables(line: &str, from: usize, to: usize) -> Vec<Syllable> {
    (from..to).filter_map(|i| syllable_at(line, i)).collect()
}

/// Plays one syllable on the synth now, in `who`'s voice and `mood` (the caller stamps it a voice).
pub fn render(s: &mut Synth, who: Talker, mood: Mood, syl: Syllable) {
    let tb = who.timbre();
    let (mp, mr, ml, mg, mb, mt) = mood.shape();
    let t = s.now;
    let range = tb.range * mr;
    // The pitch: the sentence falling as it goes, a little wander, stress, and its end.
    let wander = ((syl.n as f64) * 1.7).sin() * 0.25 + ((syl.n as f64) * 0.61).cos() * 0.12;
    let mut f = tb.f0 * mp * (1.0 + range * (0.12 - 0.22 * syl.pos + wander * 0.5));
    let mut glide = 1.0 - range * 0.12;
    let mut dur = tb.syll * ml;
    let mut gain = 0.05 * tb.gain * mg;
    if syl.stress {
        f *= 1.0 + range * 0.6;
        dur *= 1.2;
        gain *= 1.3;
    }
    match (syl.last, syl.end) {
        (true, '?') => glide = 1.0 + range * 1.3,
        (true, '.') => {
            glide = 1.0 - range * 0.7;
            dur *= 1.35;
        }
        (true, '!') => {
            f *= 1.0 + range * 0.5;
            glide = 1.0 - range * 0.5;
            dur *= 1.3;
            gain *= 1.15;
        }
        (_, '!') => gain *= 1.12,
        _ => {}
    }
    let f1f2 = syl.vowel.formants();
    let (f1, f2) = (f1f2.0 * tb.formant, f1f2.1 * tb.formant);
    let end = t + dur;
    let env = |g: f64| Env::points(vec![(t, g * 0.004), (t + 0.012, g), (t + dur * 0.65, g * 0.75), (end, g * 0.004)]);
    let pitch = Env::points(vec![(t, f), (end, f * glide)]);
    let vib = (tb.vib.0.max(if mt > 0.0 { 8.5 } else { 0.0 }), tb.vib.1 + mt);
    let voiced = |w: Wave, g: f64, kind: FilterKind, at: f64, q: f64| {
        let mut v = Voice::new(t, end + 0.01, w, pitch.clone(), env(g), Bus::Sfx).filtered(kind, at, q);
        if vib.0 > 0.0 && vib.1 > 0.0 {
            v = v.vibrato(vib.0, vib.1);
        }
        v
    };
    s.push(voiced(tb.wave, gain, FilterKind::Bandpass, f1, 4.5));
    s.push(voiced(tb.wave, gain * 0.65, FilterKind::Bandpass, f2, 6.5));
    s.push(voiced(tb.wave, gain * 0.16, FilterKind::Lowpass, f * 2.6, 1.0));
    let breath = (tb.breath + mb).min(1.0);
    if breath > 0.0 {
        let off = (syl.n as f64 * 0.137) % 1.7;
        s.push(
            Voice::new(t, end + 0.01, Wave::Hiss { rate: 1.0, offset: off }, Env::constant(1.0), env(gain * breath * 1.4), Bus::Sfx)
                .filtered(FilterKind::Bandpass, f2, 2.5),
        );
    }
    if tb.sub > 0.0 {
        let low = Env::points(vec![(t, f * 0.5), (end, f * 0.5 * glide)]);
        s.push(Voice::new(t, end + 0.01, Wave::Saw, low, env(gain * tb.sub), Bus::Sfx).filtered(FilterKind::Lowpass, 380.0, 1.0));
    }
    if tb.rasp > 0.0 {
        let off = (syl.n as f64 * 0.31) % 1.9;
        s.push(
            Voice::new(t, end + 0.01, Wave::Noise { rate: 0.5, offset: off }, Env::constant(1.0), env(gain * tb.rasp * 1.6), Bus::Sfx)
                .filtered(FilterKind::Bandpass, f1 * 0.6, 1.4),
        );
    }
    // The consonant before it.
    let off = (syl.n as f64 * 0.53) % 1.3;
    match syl.onset {
        Onset::None => {}
        Onset::Hiss => s.push(
            Voice::new(
                t - 0.0,
                t + 0.045,
                Wave::Hiss { rate: 1.2, offset: off },
                Env::constant(1.0),
                Env::ramp(t, gain * 0.9, gain * 0.01, 0.045),
                Bus::Sfx,
            )
            .filtered(FilterKind::Highpass, 3200.0 * tb.formant, 0.0),
        ),
        Onset::Stop => s.push(
            Voice::new(
                t,
                t + 0.02,
                Wave::Hiss { rate: 1.0, offset: off },
                Env::constant(1.0),
                Env::ramp(t, gain * 1.6, gain * 0.01, 0.02),
                Bus::Sfx,
            )
            .filtered(FilterKind::Bandpass, 1900.0 * tb.formant, 1.5),
        ),
        Onset::Hum => {
            s.push(Voice::new(t, t + 0.04, Wave::Sine, Env::constant(f), Env::ramp(t, gain * 0.5, gain * 0.01, 0.04), Bus::Sfx).filtered(
                FilterKind::Lowpass,
                500.0,
                1.0,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_breaks_into_syllables_with_its_sentence_shape() {
        let line = "NO! He took him! LET HIM GO!";
        let syl: Vec<Syllable> = (0..line.chars().count()).filter_map(|i| syllable_at(line, i)).collect();
        assert_eq!(syl.len(), 7, "no, he, too(k), him, le(t), him, go");
        assert!(syl[0].stress && syl[0].end == '!', "NO! is stressed");
        assert!(syl.last().unwrap().last && syl.last().unwrap().stress);
        let q = "Will you carry them?";
        let last = (0..q.len()).filter_map(|i| syllable_at(q, i)).next_back().unwrap();
        assert_eq!(last.end, '?');
        assert!(last.last);
        // Hızır's dotless i is a vowel too.
        assert!(syllable_at("Hızır", 1).is_some());
        assert!(syllables("aaa", 0, 3).len() == 1, "one vowel run, one syllable");
    }

    #[test]
    fn every_voice_differs_from_every_other() {
        let mut all = vec![Talker::Kid(1), Talker::Kid(2), Talker::Kid(3), Talker::Toby, Talker::Hizir, Talker::Krampus, Talker::Narrator];
        all.extend(crate::logic::sins::SINS.iter().map(|&s| Talker::Prince(s)));
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                let (x, y) = (a.timbre(), b.timbre());
                let apart = (x.f0 / y.f0).ln().abs() > 0.05 || (x.formant - y.formant).abs() > 0.04 || (x.breath - y.breath).abs() > 0.15;
                assert!(apart, "{a:?} and {b:?} sound alike");
            }
        }
        assert_eq!(Mood::of("screaming", ""), Mood::Fear);
        assert_eq!(Mood::of("heavily", ""), Mood::Grave);
        assert_eq!(Mood::of("", "We will."), Mood::Plain);
    }
}
