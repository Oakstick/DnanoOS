//! New Game+ (the user's call, 2026-10-09, on QA.md's proposal). Once the tale is told the title
//! offers NEW GAME+ beside Start: the adventure begins again from Scene I with the finished run's
//! kit (every hero's tiers of the old man's trees, his mastery, his loadout), its purse (silver and
//! runes), and its living rivals seeded back into the Ledger of Grudges; every foe and prince is a
//! tier harder. Finished again, it offers the next (NG+2, NG+3), up to `NG_MAX`. Start stays beside
//! it: a fresh adventure, all reset (the offer stays on the title).
//!
//! "A tier harder" is an offset on top of the numbers as they are, never a change to them: the
//! realms' tier (`Game::realm_tier`) gains one for each time over, so the dealer deals a stage
//! deeper (`logic::realms::Dealer`, to the seventh), the packs grow and weigh as a later realm's do
//! (`logic::realms::weight`), each prince fights at the next tier (`court::weight`), the Pit and
//! Krampus too (`lair_weight`); the Garden brings a stage deeper (`foe_stage_of`). A first run's
//! numbers are untouched (`ng_plus` 0).
//!
//! The run's level goes with its checkpoint (`Run::ng`); the offer is saved apart (`NG_KEY`, the
//! kit and the rivals beside it) from the moment the end begins until the New Game+ it offers
//! reaches its first checkpoint.

use super::Game;
use crate::logic::progression::Progress;
use crate::logic::war::ledger::Ledger;
use crate::platform;

/// The most times over: each a tier harder (NG+3 sends the seventh stage's foes from the first
/// realm on, the princes three tiers up).
pub const NG_MAX: u8 = 3;
/// Where the offer is saved: its level, and the kit and rivals it keeps.
pub const NG_KEY: &str = "vh.ngplus";
const NG_KIT_KEY: &str = "vh.ngplus.kit";
const NG_RIVALS_KEY: &str = "vh.ngplus.rivals";

/// What New Game+ begins with: its level (1 to `NG_MAX`), the finished run's kit and purse, and
/// its rivals still standing.
#[derive(Clone, Debug, PartialEq)]
pub struct NgOffer {
    pub level: u8,
    pub progress: Progress,
    pub ledger: Ledger,
}

impl NgOffer {
    /// The offer as the save keeps it (three keys).
    fn entries(&self) -> [(&'static str, String); 3] {
        [(NG_KEY, format!("v1 {}", self.level)), (NG_KIT_KEY, self.progress.encode()), (NG_RIVALS_KEY, self.ledger.encode())]
    }

    pub(super) fn read() -> Option<NgOffer> {
        let level = platform::load(NG_KEY)?.strip_prefix("v1 ")?.trim().parse::<u8>().ok()?;
        if !(1..=NG_MAX).contains(&level) {
            return None;
        }
        let progress = Progress::decode(&platform::load(NG_KIT_KEY)?)?;
        let ledger = platform::load(NG_RIVALS_KEY).and_then(|s| Ledger::decode(&s)).unwrap_or_default();
        Some(NgOffer { level, progress, ledger })
    }
}

/// The offer the save holds, if the tale was told (tests start from nothing).
pub(super) fn load_offer() -> Option<NgOffer> {
    if cfg!(test) {
        return None;
    }
    NgOffer::read()
}

impl Game {
    /// What New Game+ would begin with if the tale ended now: the next level (capped), the kit and
    /// purse as they stand, the Ledger's living rivals (their tally of the slain starts again).
    pub fn new_game_plus_offer(&self) -> NgOffer {
        let mut ledger = self.ledger().clone();
        ledger.rivals.retain(|r| r.alive);
        ledger.slain = 0;
        NgOffer { level: self.ng_plus.saturating_add(1).min(NG_MAX), progress: self.progress, ledger }
    }

    /// The end begins (before the run is closed): New Game+ is offered, and saved.
    pub(super) fn offer_new_game_plus(&mut self) {
        let offer = self.new_game_plus_offer();
        if self.persist.live {
            let entries = offer.entries();
            let pairs: Vec<(&str, &str)> = entries.iter().map(|(k, v)| (*k, v.as_str())).collect();
            if let Err(e) = platform::save_all(&pairs) {
                platform::warn(&format!("could not save the New Game+ offer: {e}"));
            }
        }
        self.ng_offer = Some(offer);
    }

    /// NEW GAME+ on the title: with an adventure under way it asks first, as Start does.
    pub(super) fn choose_new_game_plus(&mut self) {
        if self.can_continue() {
            self.confirm_ng = true;
            self.start_from_title();
        } else {
            self.take_new_game_plus();
            self.play_from_title();
        }
    }

    /**
     * New Game+ begins (the run already closed): the offer's kit, purse and rivals, a tier harder
     * for each time over. Nothing is written yet: the run's first checkpoint saves it all and uses
     * the offer up (`settle_new_game_plus`), so leaving before it loses nothing.
     */
    pub fn take_new_game_plus(&mut self) {
        let Some(offer) = self.ng_offer.clone() else { return };
        self.progress = offer.progress;
        self.warband.save.ledger = offer.ledger;
        self.ng_plus = offer.level;
        self.ng_taken = true;
        self.equip_from_progress();
    }

    /// A checkpoint: a New Game+ taken is under way, so its offer is used up.
    pub(super) fn settle_new_game_plus(&mut self) {
        if !self.ng_taken {
            return;
        }
        self.ng_taken = false;
        self.ng_offer = None;
        if self.persist.live {
            let _ = platform::save_all(&[(NG_KEY, ""), (NG_KIT_KEY, ""), (NG_RIVALS_KEY, "")]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The offer survives the save, and a wiped or spoiled one is no offer.
    #[test]
    fn the_offer_survives_the_save() {
        platform::use_dir_on_this_thread(std::env::temp_dir().join("vh-test-ngplus"));
        let mut g = Game::new();
        g.progress.silver = 777;
        g.progress.runes = 12;
        g.debug_ledger();
        g.persist.live = true;
        g.offer_new_game_plus();
        let back = NgOffer::read().expect("the offer reads back");
        assert_eq!(back.level, 1);
        assert_eq!((back.progress.silver, back.progress.runes), (777, 12));
        assert_eq!(back.ledger.rivals.len(), g.ledger().live().count());
        platform::save(NG_KEY, "v1 9");
        assert!(NgOffer::read().is_none(), "past the cap");
        platform::save(NG_KEY, "");
        assert!(NgOffer::read().is_none(), "wiped");
    }
}
