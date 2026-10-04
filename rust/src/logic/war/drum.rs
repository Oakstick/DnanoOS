//! The Drum (WARBAND.md §6): the pace of a level, in seconds, above the threat manager's pace of a
//! fight, in frames. It hears the war log, reckons how hard the fight presses on the hero, and keeps
//! the level breathing: build, peak, release, rest.
//!
//! It is not a cheat on the player's skill. It never touches a move's frames, never brings a foe in
//! unannounced, and never refuses a fight he picks: what it holds back is only what the foes would
//! start on their own. It hands the layers below it their bounds (`Bounds`), and the layers beside
//! it (the old man, the prince, the generals, the war-drum music) read its phase.

use super::WarEvent;
use crate::logic::combat::fsm::Defense;

/// The level's breath.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub enum DrumPhase {
    /// Nothing hunts him, or he has just come through a fight: nothing new starts on its own.
    #[default]
    Rest,
    /// A fight is on and growing.
    Build,
    /// The fight presses on him hard.
    Peak,
    /// A fight has broken (a pack cleared or routed, or a peak run long): the pressure eases.
    Release,
}

impl DrumPhase {
    pub fn name(self) -> &'static str {
        match self {
            DrumPhase::Rest => "REST",
            DrumPhase::Build => "BUILD",
            DrumPhase::Peak => "PEAK",
            DrumPhase::Release => "RELEASE",
        }
    }
}

/// How the player wants his levels paced (a setting, in the open: no hidden rubber band).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Pacing {
    /// Longer rests, shorter peaks.
    Steady,
    #[default]
    Standard,
    /// Shorter rests, longer peaks.
    Relentless,
}

impl Pacing {
    /// The longest a peak may run before the Drum calls a release (seconds).
    pub fn peak_cap(self) -> f64 {
        match self {
            Pacing::Steady => 25.0,
            Pacing::Standard => 35.0,
            Pacing::Relentless => 45.0,
        }
    }
    /// How long a release lasts (seconds).
    pub fn release(self) -> f64 {
        match self {
            Pacing::Steady => 10.0,
            Pacing::Standard => 8.0,
            Pacing::Relentless => 5.0,
        }
    }
    /// The intensity at which a build becomes a peak.
    pub fn peak_at(self) -> f64 {
        match self {
            Pacing::Steady => 0.6,
            Pacing::Standard => 0.5,
            Pacing::Relentless => 0.45,
        }
    }
}

/// What the foes may do in the Drum's present phase: the threat pool and its cadence about the
/// field's own, and what may begin on the foes' side.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Bounds {
    /// Added to the threat manager's capacity (never below one point).
    pub capacity: i8,
    /// Frames added between tokens.
    pub cadence: u32,
    /// A general may issue an order; a patrol may whistle.
    pub orders: bool,
    /// The old man may speak; the prince may whisper.
    pub voices: bool,
    /// The chance a kill drops a health potion, as a multiple.
    pub potions: f64,
}

impl Bounds {
    pub const NORMAL: Bounds = Bounds { capacity: 0, cadence: 0, orders: true, voices: false, potions: 1.0 };
}

/// What the fight looks like this frame, from the game's side of it.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Pulse {
    /// Foes awake and hunting him.
    pub engaged: u32,
    /// His health, as a share of his whole.
    pub hp: f64,
    /// His whole health (what a quarter of it is).
    pub max_hp: f64,
}

/// Seconds the Drum remembers damage and landings over.
pub const WINDOW: f64 = 5.0;
/// Seconds between reckonings of the intensity.
pub const RECKON_EVERY: f64 = 0.5;
/// Seconds with nothing hunting him before a build lapses back to rest.
pub const LAPSE: f64 = 3.0;
/// Release lasts at least this long even if a new fight starts in it (seconds).
pub const RELEASE_FLOOR: f64 = 4.0;
/// Foes hunting him that count as the most a fight holds.
pub const ENGAGED_FULL: f64 = 4.0;
/// Landings in the window that count as the most (a pack's pool lets about one a second through).
pub const LANDINGS_FULL: f64 = 4.0;
/// What the release takes off the threat pool and adds to its cadence.
pub const RELEASE_CAPACITY: i8 = -1;
pub const RELEASE_CADENCE: u32 = 30;
pub const RELEASE_POTIONS: f64 = 1.5;

/// The intensity of the fight, 0..1: the foes hunting him and the strikes reaching him in the last
/// `WINDOW` (however he met them: a fight presses on a parrying hero too), then the damage he took
/// and how low he is.
pub fn intensity(taken: f64, landings: u32, pulse: Pulse) -> f64 {
    let fifth = (0.2 * pulse.max_hp).max(1.0);
    let i = 0.30 * (pulse.engaged as f64 / ENGAGED_FULL).min(1.0)
        + 0.30 * (landings as f64 / LANDINGS_FULL).min(1.0)
        + 0.25 * (taken / fifth).min(1.0)
        + 0.15 * (1.0 - pulse.hp).clamp(0.0, 1.0);
    i.clamp(0.0, 1.0)
}

/// The Drum.
#[derive(Clone, Debug, Default)]
pub struct Drum {
    pub phase: DrumPhase,
    pub pacing: Pacing,
    /// Seconds in the present phase.
    pub age: f64,
    /// The last reckoned intensity.
    pub intensity: f64,
    /// (seconds ago, damage) and (seconds ago) of the landings, inside the window.
    taken: Vec<(f64, f64)>,
    landings: Vec<f64>,
    /// Seconds since the last reckoning, and since anything hunted him.
    since_reckon: f64,
    idle: f64,
    /// A pack fell or routed since the last reckoning.
    broke: bool,
    /// Seconds spent in each phase since the level began (rest, build, peak, release).
    pub spent: [f64; 4],
    /// Peaks begun since the level began, and the rests between them (seconds of each finished rest).
    pub peaks: u32,
    pub rests: Vec<f64>,
}

impl Drum {
    pub fn new(pacing: Pacing) -> Self {
        Self { pacing, ..Default::default() }
    }

    /// A fresh level: rest, nothing remembered (the pacing kept).
    pub fn reset(&mut self) {
        *self = Self::new(self.pacing);
    }

    /// The war log, as the Drum hears it.
    pub fn hear(&mut self, e: &WarEvent) {
        match *e {
            WarEvent::Landed { dmg, how, lands, .. } => {
                if dmg > 0.0 {
                    self.taken.push((0.0, dmg));
                }
                if lands && how != Defense::Immune {
                    self.landings.push(0.0);
                }
            }
            WarEvent::Slain { last_of_pack: true, .. } => self.broke = true,
            _ => {}
        }
    }

    /// A pack routed: the fight has broken, as surely as if it fell.
    pub fn routed(&mut self) {
        self.broke = true;
    }

    /// `dt` seconds pass with the fight as `pulse` has it.
    pub fn tick(&mut self, dt: f64, pulse: Pulse) {
        for t in self.taken.iter_mut() {
            t.0 += dt;
        }
        for t in self.landings.iter_mut() {
            *t += dt;
        }
        self.taken.retain(|t| t.0 < WINDOW);
        self.landings.retain(|&t| t < WINDOW);
        self.age += dt;
        self.spent[self.phase as usize] += dt;
        self.idle = if pulse.engaged == 0 { self.idle + dt } else { 0.0 };
        self.since_reckon += dt;
        if self.since_reckon < RECKON_EVERY {
            return;
        }
        self.since_reckon = 0.0;
        let taken: f64 = self.taken.iter().map(|t| t.1).sum();
        self.intensity = intensity(taken, self.landings.len() as u32, pulse);
        let broke = std::mem::take(&mut self.broke);
        let next = match self.phase {
            DrumPhase::Rest if pulse.engaged > 0 => Some(DrumPhase::Build),
            DrumPhase::Build if self.intensity >= self.pacing.peak_at() => Some(DrumPhase::Peak),
            DrumPhase::Build if broke || self.idle >= LAPSE => Some(DrumPhase::Rest),
            DrumPhase::Peak if broke || pulse.engaged == 0 || self.age >= self.pacing.peak_cap() => Some(DrumPhase::Release),
            DrumPhase::Release if self.age >= self.pacing.release() => {
                Some(if pulse.engaged > 0 { DrumPhase::Build } else { DrumPhase::Rest })
            }
            DrumPhase::Release if self.age >= RELEASE_FLOOR && self.intensity >= self.pacing.peak_at() + 0.15 => Some(DrumPhase::Peak),
            _ => None,
        };
        if let Some(p) = next {
            self.enter(p);
        }
    }

    fn enter(&mut self, p: DrumPhase) {
        if self.phase == DrumPhase::Rest {
            self.rests.push(self.age);
        }
        if p == DrumPhase::Peak {
            self.peaks += 1;
        }
        self.phase = p;
        self.age = 0.0;
    }

    /// What the foes may do now.
    pub fn bounds(&self) -> Bounds {
        match self.phase {
            DrumPhase::Rest => Bounds { orders: false, voices: true, ..Bounds::NORMAL },
            DrumPhase::Build => Bounds::NORMAL,
            DrumPhase::Peak => Bounds { orders: false, ..Bounds::NORMAL },
            DrumPhase::Release => {
                Bounds { capacity: RELEASE_CAPACITY, cadence: RELEASE_CADENCE, orders: false, voices: true, potions: RELEASE_POTIONS }
            }
        }
    }

    /// The share of fighting time (build, peak and release) spent at peak.
    pub fn peak_share(&self) -> f64 {
        let fighting = self.spent[1] + self.spent[2] + self.spent[3];
        if fighting <= 0.0 {
            0.0
        } else {
            self.spent[2] / fighting
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::types::V;

    fn landed(dmg: f64) -> WarEvent {
        WarEvent::Landed { foe: 1, kind: None, how: Defense::Open, dmg, dir: V::new(1.0, 0.0), lands: true, bolt: false }
    }

    fn run(d: &mut Drum, secs: f64, pulse: Pulse) {
        for _ in 0..(secs * 60.0) as usize {
            d.tick(1.0 / 60.0, pulse);
        }
    }

    #[test]
    fn the_drum_builds_peaks_releases_and_rests() {
        let mut d = Drum::new(Pacing::Standard);
        let calm = Pulse { engaged: 0, hp: 1.0, max_hp: 100.0 };
        let fight = Pulse { engaged: 4, hp: 0.6, max_hp: 100.0 };
        run(&mut d, 2.0, calm);
        assert_eq!(d.phase, DrumPhase::Rest);
        run(&mut d, 1.0, fight);
        assert_eq!(d.phase, DrumPhase::Build, "foes hunting him: a build");
        for _ in 0..5 {
            d.hear(&landed(10.0));
        }
        run(&mut d, 1.0, fight);
        assert_eq!(d.phase, DrumPhase::Peak, "struck hard and often: a peak ({})", d.intensity);
        d.hear(&WarEvent::Slain {
            foe: 2,
            kind: crate::logic::enemy::EnemyKind::Skeleton,
            at: V::new(0.0, 0.0),
            dir: V::new(1.0, 0.0),
            last_of_pack: true,
        });
        run(&mut d, 1.0, fight);
        assert_eq!(d.phase, DrumPhase::Release, "the pack fell: a release");
        let b = d.bounds();
        assert!(b.capacity < 0 && b.cadence > 0 && !b.orders && b.potions > 1.0, "{b:?}");
        run(&mut d, Pacing::Standard.release() + 1.0, calm);
        assert_eq!(d.phase, DrumPhase::Rest);
        assert!(d.bounds().voices && !d.bounds().orders);
    }

    #[test]
    fn a_peak_never_runs_past_its_cap() {
        for pacing in [Pacing::Steady, Pacing::Standard, Pacing::Relentless] {
            let mut d = Drum::new(pacing);
            let fight = Pulse { engaged: 6, hp: 0.3, max_hp: 100.0 };
            for _ in 0..(120 * 60) {
                if d.landings.len() < 6 {
                    d.hear(&landed(30.0));
                }
                d.tick(1.0 / 60.0, fight);
                if d.phase == DrumPhase::Peak {
                    assert!(d.age <= pacing.peak_cap() + RECKON_EVERY + 1e-6, "{pacing:?} peak ran {}", d.age);
                }
            }
            assert!(d.peaks >= 2, "{pacing:?}: peak, release, peak again under endless pressure");
        }
    }

    #[test]
    fn the_intensity_reads_the_fight() {
        let calm = Pulse { engaged: 0, hp: 1.0, max_hp: 100.0 };
        assert_eq!(intensity(0.0, 0, calm), 0.0);
        let worst = Pulse { engaged: 9, hp: 0.0, max_hp: 100.0 };
        assert!((intensity(100.0, 9, worst) - 1.0).abs() < 1e-9);
        let a = intensity(10.0, 1, Pulse { engaged: 2, hp: 0.9, max_hp: 100.0 });
        let b = intensity(20.0, 3, Pulse { engaged: 4, hp: 0.6, max_hp: 100.0 });
        assert!(b > a);
    }
}
