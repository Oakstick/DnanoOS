//! The Warband at run time (WARBAND.md): the war log the brawl writes, read once a frame by the
//! layers above the fight and the feel around it.
//!
//! W0 lays the pipe: every outcome of a fight is noted here as a `WarEvent`, and the tally keeps
//! count of them (the telemetry the later layers and the tests read). The Drum, the packs, the
//! Ledger, the generals and the juice each become a reader of the same log.

use super::brawl::Field;
use super::Game;
use crate::logic::combat::fsm::Defense;
use crate::logic::war::drum::{Bounds, Drum, Pulse};

/// The least the Drum leaves of a pack's threat pool.
const RELEASE_FLOOR_POINTS: u8 = 2;
use crate::logic::war::{PresRng, WarEvent};

/// What the war log has seen since the level began.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WarTally {
    pub tells: u32,
    /// Strikes that reached him and count as landings, and how he met them.
    pub landings: u32,
    pub parried: u32,
    pub blocked: u32,
    pub evaded: u32,
    pub broken: u32,
    pub open: u32,
    pub whiffs: u32,
    /// His blows that landed on foes.
    pub hero_hits: u32,
    pub kills: u32,
    /// Packs (or bouts) whose last foe fell.
    pub packs_cleared: u32,
    pub drinks: u32,
}

impl WarTally {
    fn count(&mut self, e: &WarEvent) {
        match *e {
            WarEvent::Tell { .. } => self.tells += 1,
            WarEvent::Landed { how, lands, .. } => {
                if lands {
                    self.landings += 1;
                }
                match how {
                    Defense::Parried => self.parried += 1,
                    Defense::Blocked => self.blocked += 1,
                    Defense::Evaded => self.evaded += 1,
                    Defense::GuardBroken => self.broken += 1,
                    Defense::Open => self.open += 1,
                    Defense::Immune => {}
                }
            }
            WarEvent::Whiffed { .. } => self.whiffs += 1,
            WarEvent::HeroHit { .. } => self.hero_hits += 1,
            WarEvent::Slain { last_of_pack, .. } => {
                self.kills += 1;
                if last_of_pack {
                    self.packs_cleared += 1;
                }
            }
            WarEvent::Drank { .. } => self.drinks += 1,
        }
    }
}

/// The war's state on the game: the log of this frame, the presentation's rng, the tally.
#[derive(Clone, Debug, Default)]
pub struct War {
    /// Written in the order things happen; emptied once a frame by `update_war`.
    pub log: Vec<WarEvent>,
    /// What the last `update_war` read (the readers of this frame look here).
    pub read: Vec<WarEvent>,
    pub pres: PresRng,
    pub tally: WarTally,
    /// The level's breath (§6).
    pub drum: Drum,
}

impl War {
    /// A fresh level: no log, no tally (the presentation's rng runs on).
    pub fn reset(&mut self) {
        self.log.clear();
        self.read.clear();
        self.tally = WarTally::default();
        self.drum.reset();
    }
}

impl Game {
    /// Something happened in the fight: into the war log.
    pub(super) fn war_note(&mut self, e: WarEvent) {
        self.war.log.push(e);
    }

    /// Once a frame (`dt` seconds of play): the war log is read by the tally and the Drum, and the
    /// Drum's bounds go down to the threat manager.
    pub(super) fn update_war(&mut self, dt: f64) {
        let log = std::mem::take(&mut self.war.log);
        for e in &log {
            self.war.tally.count(e);
            self.war.drum.hear(e);
        }
        self.war.read = log;
        let engaged = self.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.engaged() && !d.puppet).count() as u32;
        let pulse = Pulse { engaged, hp: self.hero.hp as f64 / self.hero.max_hp.max(1) as f64, max_hp: self.hero.max_hp as f64 };
        self.war.drum.tick(dt, pulse);
        self.pace_the_brawl();
    }

    /// What the Drum allows now (with `VH_NO_DRUM` set, for A/B runs of the harness, it allows
    /// what the field always did).
    pub(super) fn drum_bounds(&self) -> Bounds {
        static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if *OFF.get_or_init(|| std::env::var_os("VH_NO_DRUM").is_some()) {
            Bounds::NORMAL
        } else {
            self.war.drum.bounds()
        }
    }

    /// The Drum's bounds on the threat manager: the vaults' and the realms' packs (the ring is the
    /// lab, and a prince's court keeps its own pace).
    fn pace_the_brawl(&mut self) {
        let court = self.in_court();
        let bounds = self.drum_bounds();
        let Some(b) = self.brawl.as_mut().filter(|b| b.field == Field::Vault && !court) else { return };
        let (capacity, cadence) = b.base_pace();
        // (Never below two points: a brute, which costs two, may still strike in a release.)
        b.threat.capacity = (capacity as i8 + bounds.capacity).max(RELEASE_FLOOR_POINTS as i8) as u8;
        b.threat.cadence = cadence + bounds.cadence;
    }
}
