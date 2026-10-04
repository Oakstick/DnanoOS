//! The two voices at run time (WARBAND.md §10): what the old man says and when, how it reaches the
//! screen, and what he says at the workshop.
//!
//! The counsel's rules live in `logic::war::counsel`; this hears the game for them (a fall and what
//! struck it, a rival met, a realm entered, the blade and the shield wearing, a cheat seen through)
//! and delivers one line at a time to the bottom-left panel (`render::hud::voices`): typed out in
//! amber, with Quitmer's bark as his signature, at most one every 45 s, and only while the Drum
//! allows voices (Rest, Release) or on the death screen; never in Peak. The prince's whisper uses
//! the same panel and gets the old man's signs wrong (`game::deceit`).
//!
//! At the workshop his line depends on what just happened below (`workshop_counsel`), and the War
//! Table sells the rivals' true weaknesses (`buy_intel`).

use super::deceit::Deceit;
use super::{Game, Screen};
use crate::audio::{Sfx, Speech};
use crate::logic::enemy::EnemyKind;
use crate::logic::sins::Sin;
use crate::logic::types::V;
use crate::logic::war::counsel::{self, Answer, Frame, GearWarning, GeneralIntel, Trigger};
use crate::logic::war::deceit::{CheatKind, SEEN_RUNES};
use crate::logic::war::ledger::{counsel_line_for, RivalId, Trait};
use crate::logic::war::WarEvent;

/// Letters typed a second.
pub const TYPE_RATE: f64 = 42.0;
/// Seconds a line stays up once typed.
pub const HOLD: f64 = 5.0;
/// A voice blip every this many letters.
const BLIP_EVERY: usize = 3;
/// The old man's colour: amber (the prince's whisper comes in his sin's colour instead).
pub const AMBER: u32 = 0xffa83a;
/// Below this share of its strength, his shield is cracking.
const CRACKING: f32 = 0.3;

/// Who is speaking in the panel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Speaker {
    OldMan,
    /// The prince of this sin, in the old man's words.
    Whisper(Sin),
}

/// A line in the panel: whose, what, since when (the game's time), and the sounds made for it.
#[derive(Clone, PartialEq, Debug)]
pub struct Spoken {
    pub who: Speaker,
    pub text: String,
    pub since: f64,
    blips: usize,
    echoed: bool,
}

impl Spoken {
    /// Letters shown by `now`.
    pub fn shown(&self, now: f64) -> usize {
        (((now - self.since).max(0.0) * TYPE_RATE) as usize).min(self.text.chars().count())
    }

    /// Typed out and held long enough.
    pub fn done(&self, now: f64) -> bool {
        now - self.since > self.text.chars().count() as f64 / TYPE_RATE + HOLD
    }

    /// The text's colour: amber, or the sin's.
    pub fn color(&self) -> u32 {
        match self.who {
            Speaker::OldMan => AMBER,
            Speaker::Whisper(sin) => sin.color(),
        }
    }
}

/// The voices on the game: the old man's counsel, the line in the panel, the prince's cheats.
#[derive(Clone, Debug, Default)]
pub struct Voices {
    pub counsel: counsel::Counsel,
    pub spoken: Option<Spoken>,
    pub deceit: Deceit,
    /// The counsel's flags were read from the save.
    loaded: bool,
    /// Where he is: a realm (and whether it is the vaults), for knowing when he enters one.
    place: Option<(Sin, bool)>,
    shield: f32,
    /// What the old man has to say at the workshop, since the last thing that happened below.
    pub note: Option<String>,
}

/// What the War Table asks for a rival's true weakness: 40 silver at rank 1, up to 120 at rank 5.
pub fn intel_price(rank: u8) -> u32 {
    40 + 20 * (rank.clamp(1, 5) as u32 - 1)
}

/// How many rivals the War Table offers news of at once.
pub const INTEL_ROWS: usize = 3;

impl Game {
    /// What the generals' layer knows of the realm of `sin` (W6 fills it from
    /// `logic::war::general`: the general's name, one instrument, where it stands). Until then the
    /// old man says only what is sure.
    pub fn general_intel(&self, sin: Sin) -> Option<GeneralIntel> {
        // What the old man knows of the realm's general (WARBAND.md §9, §10.1): his name, and where
        // the first of his instruments still standing is from where the hero came in. True by
        // construction: read from the general the realm holds.
        let run = self.general.as_ref().filter(|r| r.sin == sin && !r.command.slain)?;
        let general = crate::logic::war::general::General::of(sin);
        let k = (0..3).find(|&k| !run.command.instruments[k].broken)?;
        let at = run.posts.instruments[k];
        let from = self.level.hero_spawn;
        let (dx, dy) = ((at.x - from.x) as f64, (at.y - from.y) as f64);
        let ns = if dy < -dx.abs() * 0.5 {
            "north"
        } else if dy > dx.abs() * 0.5 {
            "south"
        } else {
            ""
        };
        let ew = if dx > dy.abs() * 0.5 {
            "east"
        } else if dx < -dy.abs() * 0.5 {
            "west"
        } else {
            ""
        };
        let way = match (ns, ew) {
            ("", "") => "close by where you came in".to_string(),
            (n, "") => format!("to the {n} of where you came in"),
            ("", e) => format!("to the {e} of where you came in"),
            (n, e) => format!("to the {n}-{e} of where you came in"),
        };
        Some(GeneralIntel {
            name: crate::game::general::title_case(&run.name),
            // ("THE COUNTING-BELL" as the line says it: "counting-bell".)
            instrument: general.instruments[k].trim_start_matches("THE ").to_lowercase(),
            place: way,
        })
    }

    /// The line in the voices' panel, if one is up.
    pub fn voices_line(&self) -> Option<&Spoken> {
        self.voices.spoken.as_ref().filter(|s| !s.done(self.time))
    }

    /// The old man hears something; what he means to say waits for its moment. His once-flags are
    /// kept with the run.
    pub(super) fn counsel_hear(&mut self, t: Trigger) {
        let before = self.voices.counsel.flags.len();
        self.voices.counsel.hear(t, self.hero_kind, self.time);
        if self.voices.counsel.flags.len() != before {
            self.warband.save.counsel = self.voices.counsel.encode();
            self.save_warband();
        }
    }

    /// The gear wears: his warning instead of a floater.
    pub(super) fn counsel_gear(&mut self, w: GearWarning) {
        self.counsel_hear(Trigger::Gear(w));
    }

    /// A rival came within ten tiles: one true trait, if one is known.
    pub(super) fn counsel_rival(&mut self, id: RivalId) {
        if let Some(r) = self.ledger().get(id).cloned() {
            self.counsel_hear(Trigger::RivalMet(&r));
        }
    }

    /// He fell: to what (the last foe whose blow reached him), and where. The counter for that
    /// breed waits for the death screen, and the workshop will remember it.
    pub(super) fn counsel_fell(&mut self) {
        let landed = |e: &WarEvent| match *e {
            WarEvent::Landed { kind: Some(k), dmg, .. } if dmg > 0.0 => Some(k),
            _ => None,
        };
        let by = self.war.log.iter().rev().find_map(landed).or_else(|| self.war.read.iter().rev().find_map(landed));
        let room = self.room_key();
        self.counsel_hear(Trigger::Died { by, room });
        if let Some(kind) = by {
            let body = Answer::of(kind).body(kind);
            self.voices.note = Some(format!("The {} put you down. {body}", crate::logic::war::names::breed_name(kind)));
        }
    }

    /// The room he stands in, as a key: the vaults' room, a realm's den, else a ten-tile square.
    fn room_key(&self) -> u64 {
        let p = self.hero.occupied();
        let realm = self.realm.map(|s| s.index() as u64 + 1).unwrap_or(0) << 32;
        if let Some(v) = &self.vault {
            if let Some(i) = v.rooms.iter().position(|r| r.rect.contains(p)) {
                return realm | 0x1_0000 | i as u64;
            }
        }
        if let Some(run) = &self.run {
            if let Some(i) = run.base().packs.iter().position(|k| k.den.contains(p)) {
                return realm | 0x2_0000 | i as u64;
            }
        }
        realm | ((p.x.max(0) / 10) as u64) << 8 | (p.y.max(0) / 10) as u64
    }

    /// A line into the panel now: the old man's with Quitmer's bark, the whisper's without.
    pub(super) fn speak(&mut self, who: Speaker, text: String) {
        if who == Speaker::OldMan {
            self.audio.play(Sfx::Bark);
        }
        self.voices.spoken = Some(Spoken { who, text, since: self.time, blips: 0, echoed: false });
    }

    /// Once a frame: the counsel's flags from the save (once), a realm entered or left, the
    /// shield's wear, the prince's cheats, and the line in the panel typed out with its sounds.
    pub(super) fn update_voices(&mut self, dt: f64) {
        if !self.voices.loaded {
            self.voices.loaded = true;
            self.voices.counsel = counsel::Counsel::decode(&self.warband.save.counsel);
            self.voices.shield = 1.0;
        }
        let place = self.realm.filter(|_| self.vault.is_some() || self.run.is_some()).map(|s| (s, self.vault.is_some()));
        if place != self.voices.place && matches!(self.screen, Screen::Playing | Screen::Ready) {
            self.leave_deceit();
            self.voices.place = place;
            self.voices.counsel.new_visit();
            if let Some((sin, _)) = place {
                let intel = self.general_intel(sin);
                self.counsel_hear(Trigger::RealmEntered { sin, intel: intel.as_ref() });
                self.deal_deceit(sin);
            }
        }
        // What of the cheats stands in the world, for the renderer.
        let d = &self.voices.deceit;
        self.view.deceit = crate::render::view::DeceitView {
            keeper: d.keeper.as_ref().filter(|k| k.demon.is_none()).map(|k| (k.at, k.facing, d.sin.map(|s| s.color()).unwrap_or(0xffffff))),
            chest: d.mimic.as_ref().filter(|m| !m.done).map(|m| (m.at, V::new(0.0, 1.0))),
        };
        if self.screen == Screen::Playing {
            let frac = self.hero.shield.map(|s| s.fraction()).unwrap_or(1.0);
            if frac < CRACKING && self.voices.shield >= CRACKING && frac > 0.0 {
                self.counsel_gear(GearWarning::ShieldCracking);
            }
            self.voices.shield = frac;
            self.update_deceit(dt);
        }
        // The next line, when the old man may speak and the panel is free.
        let open = (self.screen == Screen::Playing && self.drum_bounds().voices) || self.screen == Screen::Dying;
        if self.voices_line().is_none() {
            if let Some(line) = self.voices.counsel.next(self.time, open) {
                self.speak(Speaker::OldMan, line.text);
            }
        }
        // Typed out: a blip every few letters, and for the whisper its reversed echo at the end.
        let now = self.time;
        if let Some(s) = self.voices.spoken.as_mut() {
            let shown = s.shown(now);
            let mut blips = 0;
            while s.blips * BLIP_EVERY < shown {
                s.blips += 1;
                blips += 1;
            }
            let echo = matches!(s.who, Speaker::Whisper(_)) && !s.echoed && shown >= s.text.chars().count();
            if echo {
                s.echoed = true;
            }
            for _ in 0..blips.min(2) {
                self.audio.play_voice(Speech::Old);
            }
            if echo {
                self.audio.play(Sfx::Whisper);
            }
        }
    }

    /// A cheat of the prince's seen through: the count kept with the run, runes, and (but for the
    /// whisper, which he answers at the workshop) his praise, with Quitmer's bark.
    pub(super) fn seen_through(&mut self, kind: CheatKind) {
        self.warband.save.lies += 1;
        self.progress.runes += SEEN_RUNES;
        self.save_progress();
        self.save_warband();
        self.audio.play(Sfx::Bark);
        let hp = self.hero.pos();
        self.floater(hp.x, hp.y - 1.2, format!("SEEN THROUGH  +{SEEN_RUNES} RUNES"), 2.0, Some(AMBER));
        if kind != CheatKind::Whisper {
            self.counsel_hear(Trigger::SeenThrough(kind));
        }
        self.voices.note = Some(counsel::praise_line(kind, self.hero_kind));
    }

    /// He called the old man: if a whisper is still in the air, the old man denies it, puts the
    /// truth in its place, and Quitmer barks.
    pub(super) fn voices_called(&mut self) {
        let Some(live) = self.voices.deceit.whisper.take() else { return };
        self.seen_through(CheatKind::Whisper);
        self.voices.note = Some(counsel::denial(&live.w.truth));
        self.set_quitmer_mood(super::QuitmerMood::Wag, 3.0);
        if matches!(self.voices.spoken.as_ref().map(|s| s.who), Some(Speaker::Whisper(_))) {
            self.voices.spoken = None;
        }
    }

    /// What the old man says at the workshop, by what just happened below: a whisper denied, a fall
    /// and its counter, a cheat seen through; else at the war table and the trophy wall, the lies
    /// the hero has seen through this run. `None`: the station's own line.
    pub fn workshop_counsel(&self) -> Option<String> {
        if let Some(n) = &self.voices.note {
            return Some(n.clone());
        }
        use super::Station;
        if matches!(self.station(), Station::WarTable | Station::Trophies) || self.shop.called {
            return counsel::lies_remark(self.warband.save.lies, self.hero_kind);
        }
        None
    }

    /// The rivals the War Table has word of: living, those whose weakness is still unknown first,
    /// then by rank.
    pub fn intel_rivals(&self) -> Vec<RivalId> {
        let mut live: Vec<_> = self.ledger().live().collect();
        live.sort_by_key(|r| (r.knows(Trait::Weakness(r.weakness)), std::cmp::Reverse(r.rank), r.id));
        live.into_iter().take(INTEL_ROWS).map(|r| r.id).collect()
    }

    /// Buys a rival's true weakness at the War Table: revealed in the Ledger, and the old man says it.
    pub(super) fn buy_intel(&mut self, id: RivalId) {
        let Some(r) = self.ledger().get(id).cloned() else { return };
        let w = Trait::Weakness(r.weakness);
        if r.knows(w) {
            self.audio.play(Sfx::Denied);
            return;
        }
        if !self.pay(intel_price(r.rank)) {
            return;
        }
        self.warband.save.ledger.reveal(id, w);
        self.save_warband();
        let r = self.ledger().get(id).cloned().unwrap_or(r);
        self.voices.note = Some(counsel::framed(self.hero_kind, Frame::Rival, &counsel_line_for(&r)));
        self.sold(format!("{} - {}", r.full_name().to_uppercase(), w.name().to_uppercase()), AMBER);
    }

    /// Capture hook (`counsel`): he has just fallen to the nearest foe (or an oni), and the old man
    /// speaks at once.
    pub fn debug_counsel(&mut self) {
        let hp = self.hero.pos();
        let kind = self
            .enemies
            .iter()
            .filter(|e| e.duel.as_ref().is_none_or(|d| d.alive()))
            .min_by(|a, b| crate::logic::types::dist_sq(a.pos(), hp).total_cmp(&crate::logic::types::dist_sq(b.pos(), hp)))
            .map(|e| e.kind())
            .unwrap_or(EnemyKind::Oni);
        self.voices.loaded = true;
        self.voices.counsel.flags.clear();
        self.voices.counsel.last = None;
        let room = self.room_key();
        self.voices.counsel.hear(Trigger::Died { by: Some(kind), room }, self.hero_kind, self.time);
        if let Some(line) = self.voices.counsel.next(self.time, true) {
            self.speak(Speaker::OldMan, line.text);
        }
    }
}

/// Where a point lies from another, as a unit vector (zero if they meet).
pub(super) fn toward(from: V, to: V) -> V {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    let l = (dx * dx + dy * dy).sqrt();
    if l < 1e-9 {
        V::new(0.0, 0.0)
    } else {
        V::new(dx / l, dy / l)
    }
}

#[cfg(test)]
mod tests {

    /// The old man names the realm's general and where an instrument of his stands, from the
    /// general the realm really holds (WARBAND.md §10.1: true by construction).
    #[test]
    fn the_old_man_names_the_realms_general_and_his_instrument() {
        let g = crate::game::realms::tests::in_realm(Sin::Greed);
        let intel = g.general_intel(Sin::Greed).expect("the vaults hold a general");
        let run = g.general.as_ref().unwrap();
        assert_eq!(intel.name.to_uppercase(), run.name.to_uppercase());
        assert!(crate::logic::war::general::General::of(Sin::Greed)
            .instruments
            .iter()
            .any(|i| i.trim_start_matches("THE ").to_lowercase() == intel.instrument));
        assert!(intel.place.contains("where you came in"));
    }
    use super::super::tests::playing;
    use super::*;
    use crate::logic::war::drum::DrumPhase;

    #[test]
    fn the_old_man_speaks_once_in_45_seconds_and_never_at_the_peak() {
        let mut g = playing();
        g.update(crate::app::STEP);
        g.counsel_gear(GearWarning::BladeDulling);
        g.counsel_gear(GearWarning::ShieldBroken);
        // At the peak he holds his tongue.
        g.war.drum.phase = DrumPhase::Peak;
        let peak = g.drum_bounds();
        assert!(!peak.voices);
        g.update_voices(0.0);
        assert!(g.voices_line().is_none(), "never in Peak");
        // At rest he speaks, amber, with Quitmer's bark; and only one line.
        g.war.drum.phase = DrumPhase::Rest;
        g.update_voices(0.0);
        let s = g.voices_line().expect("he speaks at rest").clone();
        assert_eq!((s.who, s.color()), (Speaker::OldMan, AMBER));
        // The panel frees, but the second waits out the gap.
        g.time += s.text.len() as f64 / TYPE_RATE + HOLD + 1.0;
        g.update_voices(0.0);
        assert!(g.voices_line().is_none(), "one line in 45 s at most");
        g.time = s.since + counsel::GAP + 0.1;
        g.update_voices(0.0);
        assert!(g.voices_line().is_some_and(|l| l.text != s.text), "the next line after the gap");
    }

    #[test]
    fn a_fall_brings_the_breeds_counter_to_the_death_screen_and_the_workshop() {
        let mut g = playing();
        g.update(crate::app::STEP);
        g.war.log.push(WarEvent::Landed {
            foe: 1,
            kind: Some(EnemyKind::Gashadokuro),
            how: crate::logic::combat::fsm::Defense::Open,
            dmg: 30.0,
            dir: V::new(1.0, 0.0),
            lands: true,
            bolt: false,
        });
        g.counsel_fell();
        g.enter(Screen::Dying);
        g.update_voices(0.0);
        let line = g.voices_line().expect("on the death screen").text.to_lowercase();
        assert!(line.contains(Answer::ParryOrLeap.keyword()), "{line}");
        assert!(g.workshop_counsel().unwrap().contains("gashadokuro"));
        // Once per breed, for the run (and kept in the save).
        assert!(g.warband.save.counsel.contains(&format!("d{}", counsel::kind_index(EnemyKind::Gashadokuro))));
    }

    #[test]
    fn the_war_table_sells_a_rivals_true_weakness_by_rank() {
        use crate::game::{RowKind, ShopAction};
        use crate::logic::war::names::Deed;
        let mut g = playing();
        let id = g.promote_rival(EnemyKind::Oni, Deed::KillingBlow);
        for _ in 0..2 {
            g.warband.save.ledger.rank_up(id);
        }
        let rank = g.ledger().get(id).unwrap().rank;
        assert_eq!(intel_price(1), 40);
        assert_eq!(intel_price(5), 120);
        g.open_shop();
        g.go_to_station(super::super::Station::WarTable);
        g.progress.silver = 500;
        let rows = g.shop_rows();
        let r = rows.iter().position(|r| matches!(r.kind, RowKind::Rival(_))).expect("a rival's row");
        assert_eq!(rows[r].buttons[0].action, ShopAction::Intel(id));
        assert_eq!(rows[r].buttons[0].price, intel_price(rank));
        g.shop.row = r;
        g.input.inject(crate::input::Action::Confirm);
        g.update(crate::app::STEP);
        let rv = g.ledger().get(id).unwrap();
        assert!(rv.knows(Trait::Weakness(rv.weakness)), "revealed in the Ledger");
        assert_eq!(g.progress.silver, 500 - intel_price(rank));
        let said = g.workshop_counsel().unwrap();
        assert!(said.contains(&Trait::Weakness(rv.weakness).keyword()), "and he says it: {said}");
        // Known, it is not sold twice.
        assert!(!g.shop_rows()[r].buttons[0].enabled);
    }
}
