//! New Game+ (the user's call, 2026-10-09, on QA.md's proposal). Once the tale is told the title
//! offers NEW GAME+ beside Start: the adventure begins again from Scene I with the finished run's
//! kit (every hero's tiers of the old man's trees, his mastery, his loadout), its purse (silver and
//! runes), and its living rivals seeded back into the Ledger of Grudges; every foe and prince is a
//! tiers harder (`NG_TIERS`). Finished again, it offers the next (NG+2, NG+3), up to `NG_MAX`. Start stays beside
//! it: a fresh adventure, all reset (the offer stays on the title).
//!
//! "Tiers harder" is an offset on top of the numbers as they are, never a change to them: the
//! realms' tier (`Game::realm_tier`) gains `NG_TIERS` for each time over (two: one was easier for
//! the expert than his first run, the kit outgrowing it; the user's call, 2026-10-09), so the
//! dealer deals stages deeper (`logic::realms::Dealer`, to the seventh), the packs grow and weigh as a later realm's do
//! (`logic::realms::weight`), each prince fights tiers up (`court::weight`), the Pit and
//! Krampus too (`lair_weight`); the Garden brings stages deeper (`foe_stage_of`). On top of the
//! tiers each level weighs more (`NG_PRINCE`, `NG_KRAMPUS`, `NG_FOES`: health, blows, a prince's
//! poise), multiplied in where each weight is applied (the user's call: keep the gear, make the
//! princes bite). A first run's numbers are untouched (`ng_plus` 0: every weight 1).
//!
//! The run's level goes with its checkpoint (`Run::ng`); the offer is saved apart (`NG_KEY`, the
//! kit and the rivals beside it) from the moment the end begins until the New Game+ it offers
//! reaches its first checkpoint.

use super::Game;
use crate::logic::progression::Progress;
use crate::logic::war::ledger::Ledger;
use crate::platform;

/// The most times over: each `NG_TIERS` harder (NG+3 sends the seventh stage's foes from the first
/// realm on, the princes six tiers up).
pub const NG_MAX: u8 = 3;
/// The tiers each time over adds to the realms' tier, the princes', the Pit's and the Garden's stage.
pub const NG_TIERS: u8 = 2;
/**
 * The princes' health, blows and poise at NG+1, NG+2 and NG+3, multiplied on top of everything
 * else (the tiers, `court::weight`, `court::temper`). The tiers alone did not bite: the kit a
 * finished run carries (8/8/8 from the first realm, the blade 10 by the second court) outweighed
 * them, and the expert's NG+1 was quicker than his first run, the princes falling in one or two
 * minutes (WARBAND.md §14). Their blows and poise rise with every level; their health takes its big
 * step at NG+1 and eases after it, since the two tiers each level adds already weigh more on the
 * late princes every time (the court's curve): at 2.0 or more from NG+2 on, Asmodeus at the
 * eleventh tier and beyond outlasted the expert's eight minutes.
 */
pub const NG_PRINCE: [(f64, f64, f64); NG_MAX as usize] = [(2.5, 1.5, 1.1), (1.75, 1.75, 1.2), (1.5, 2.0, 1.3)];
/// Krampus's, as the princes' blows and poise but a lighter hand on his health: he is met two tiers
/// deeper than the last prince, and his lair's weight is not tempered (at the princes' health he
/// outlasted the expert's ten minutes at NG+3).
pub const NG_KRAMPUS: [(f64, f64, f64); NG_MAX as usize] = [(2.0, 1.5, 1.1), (1.6, 1.75, 1.2), (1.4, 2.0, 1.3)];
/// Every other foe's health and blows at NG+1, NG+2 and NG+3, on top of the tiers (the realms', the
/// Vaults', the generals' and the princes' adds, the Pit's, the Perchten, the Garden's bites).
pub const NG_FOES: [(f64, f64); NG_MAX as usize] = [(1.75, 1.75), (1.85, 1.85), (2.0, 2.0)];
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

/// What a prince of New Game+ `level` is multiplied by on top of his weight: his
/// health, his blows, his poise (all 1 in a first run).
pub fn prince_weight(level: u8) -> (f64, f64, f64) {
    match level {
        0 => (1.0, 1.0, 1.0),
        n => NG_PRINCE[(n.min(NG_MAX) - 1) as usize],
    }
}

/// What Krampus is multiplied by in New Game+ `level` on top of his lair's weight: his health, his
/// blows, his poise (all 1 in a first run).
pub fn krampus_weight(level: u8) -> (f64, f64, f64) {
    match level {
        0 => (1.0, 1.0, 1.0),
        n => NG_KRAMPUS[(n.min(NG_MAX) - 1) as usize],
    }
}

/// What every other foe of New Game+ `level` is multiplied by on top of its weight: its health and
/// its blows (both 1 in a first run).
pub fn foe_weight(level: u8) -> (f64, f64) {
    match level {
        0 => (1.0, 1.0),
        n => NG_FOES[(n.min(NG_MAX) - 1) as usize],
    }
}

/// A weight (health, blows) with this New Game+'s foe weight on top.
pub fn with_foe_weight((hp, damage): (f64, f64), level: u8) -> (f64, f64) {
    let (h, d) = foe_weight(level);
    (hp * h, damage * d)
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

    /// A prince's (or Krampus's) weight on top of his court's or lair's in this New Game+: his
    /// health, his blows, his poise (`prince_weight`).
    pub fn ng_prince(&self) -> (f64, f64, f64) {
        prince_weight(self.ng_plus)
    }

    /// Krampus's weight on top of his lair's in this New Game+ (`krampus_weight`).
    pub fn ng_krampus(&self) -> (f64, f64, f64) {
        krampus_weight(self.ng_plus)
    }

    /// A foe's weight (health, blows) with this New Game+'s on top (`foe_weight`).
    pub fn ng_foe(&self, weight: (f64, f64)) -> (f64, f64) {
        with_foe_weight(weight, self.ng_plus)
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
     * New Game+ begins (the run already closed): the offer's kit, purse and rivals, tiers harder
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

    /// A first run's weights are untouched (all 1); New Game+'s goes on top of the tiers, on the
    /// princes, Krampus, and the foes, as `prince_weight` and `foe_weight` say.
    #[test]
    fn new_game_plus_weighs_the_princes_krampus_and_the_foes() {
        use crate::game::brawl::{Brawl, Field};
        use crate::game::save::Stage;
        use crate::logic::enemy::EnemyKind;
        use crate::logic::hub::Rect;
        use crate::logic::realms::Pack;
        use crate::logic::sins::Sin;
        use crate::logic::types::P;
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test-ngplus-weight"));
        assert_eq!((prince_weight(0), krampus_weight(0), foe_weight(0)), ((1.0, 1.0, 1.0), (1.0, 1.0, 1.0), (1.0, 1.0)));
        assert_eq!(Game::new().ng_foe((0.7, 0.9)), (0.7, 0.9), "a first run's foes as weighed");
        let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
        for ng in 1..=NG_MAX {
            let (wh, wd, wp) = prince_weight(ng);
            let (fh, fd) = foe_weight(ng);
            assert!(wh > 1.0 && wd > 1.0 && wp > 1.0 && fh > 1.0 && fd > 1.0, "NG+{ng} bites");
            if ng > 1 {
                let ((_, pd, pp), (_, kd, kp), (lh, ld)) = (prince_weight(ng - 1), krampus_weight(ng - 1), foe_weight(ng - 1));
                assert!(wd > pd && wp > pp && fh > lh && fd > ld, "NG+{ng} harder than the one before");
                assert!(krampus_weight(ng).1 > kd && krampus_weight(ng).2 > kp);
            }
            // Mammon at the same tier: a first run's with keys held, New Game+'s without.
            let mammon = |level: u8| {
                let mut g = Game::new();
                g.ng_plus = level;
                if level == 0 {
                    g.sins_cleared = [Sin::Pride, Sin::Envy, Sin::Wrath, Sin::Sloth, Sin::Gluttony, Sin::Lust]
                        .iter()
                        .take((NG_TIERS * ng) as usize)
                        .fold(0, |m, s| m | s.bit());
                }
                g.map_jump(Stage::Court(Sin::Greed));
                let d = g.enemies[0].duel.as_deref().unwrap();
                (g.realm_tier(), d.max_hp, d.breed.damage, d.breed.poise)
            };
            let ((t0, h0, d0, p0), (t1, h1, d1, p1)) = (mammon(0), mammon(ng));
            assert_eq!(t0, t1, "the same tier");
            assert!(near(h1 / h0, wh) && near(d1 / d0, wd) && near(p1 / p0, wp), "Mammon at NG+{ng}");
            // Krampus: his lair's weight at his tier, and New Game+'s on top.
            let krampus = |ng: u8| {
                let mut g = Game::new();
                g.ng_plus = ng;
                g.map_jump(Stage::Lair);
                let id = g.lair.as_ref().unwrap().krampus;
                let d = g.enemies.iter().filter_map(|e| e.duel.as_deref()).find(|d| d.id == id).unwrap();
                let (lh, ld) = super::super::lair::lair_weight(g.realm_tier());
                (d.max_hp / lh, d.breed.damage / ld, d.breed.poise)
            };
            let ((h0, d0, p0), (h1, d1, p1)) = (krampus(0), krampus(ng));
            let (kh, kd, kp) = krampus_weight(ng);
            assert!(kh > 1.0 && kd > 1.0 && kp > 1.0);
            assert!(near(h1 / h0, kh) && near(d1 / d0, kd) && near(p1 / p0, kp), "Krampus at NG+{ng}");
            // A realm's foe at the same tier.
            let foe = |ng: u8| {
                let mut g = Game::new();
                g.ng_plus = ng;
                let pack = Pack { den: Rect { x0: 4, y0: 4, x1: 8, y1: 8 }, foes: vec![(EnemyKind::Skeleton, 1, P::new(5, 5))] };
                let e = g.raise_packs(&[pack], 3, &mut Brawl::new(Field::Vault));
                let d = e[0].duel.as_deref().unwrap();
                (d.max_hp, d.breed.damage)
            };
            let ((h0, d0), (h1, d1)) = (foe(0), foe(ng));
            assert!(near(h1 / h0, fh) && near(d1 / d0, fd), "a realm's foe at NG+{ng}");
        }
    }
}
