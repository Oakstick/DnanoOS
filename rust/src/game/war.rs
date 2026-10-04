//! The Warband at run time (WARBAND.md): the war log the brawl writes, read once a frame by the
//! layers above the fight and the feel around it.
//!
//! W0 lays the pipe: every outcome of a fight is noted here as a `WarEvent`, and the tally keeps
//! count of them (the telemetry the later layers and the tests read). The Drum, the packs, the
//! Ledger, the generals and the juice each become a reader of the same log.

use super::Game;
use crate::logic::combat::fsm::Defense;
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
}

impl War {
    /// A fresh level: no log, no tally (the presentation's rng runs on).
    pub fn reset(&mut self) {
        self.log.clear();
        self.read.clear();
        self.tally = WarTally::default();
    }
}

impl Game {
    /// Something happened in the fight: into the war log.
    pub(super) fn war_note(&mut self, e: WarEvent) {
        self.war.log.push(e);
    }

    /// Once a frame: the war log is read, by the tally now and by the layers as they are built.
    pub(super) fn update_war(&mut self) {
        let log = std::mem::take(&mut self.war.log);
        for e in &log {
            self.war.tally.count(e);
        }
        self.war.read = log;
    }
}
