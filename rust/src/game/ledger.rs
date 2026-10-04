//! The Ledger of Grudges at run time (WARBAND.md §8): the run's Warband as saved (`vh.warband`),
//! the rival name card shown when a rival is met, and the Ledger board from the pause menu (THE
//! LEDGER). The fight's wiring (rivals placed as pack leaders, the war log read into each one's
//! `EncounterTracker`) calls in here: `meet_rival` when he comes within ten tiles,
//! `promote_rival` and `rival_fought` when a fight is over.

use super::{Game, MenuItem, Screen};
use crate::audio::Sfx;
use crate::input::Action;
use crate::logic::enemy::EnemyKind;
use crate::logic::sins::Sin;
use crate::logic::types::{PotionKind, V};
use crate::logic::war::ledger::{bark, Encounter, Family, Ledger, Lesson, Outcome, Place, Rival, RivalId, Situation, Trait, Verbs};
use crate::logic::war::names::Deed;
use crate::logic::war::save::WarSave;
use crate::platform;

/// Where the run's Warband is saved, beside `vh.run`.
pub(super) const WARBAND_KEY: &str = "vh.warband";
/// How long a rival's name card stays up.
pub const CARD_SECONDS: f64 = 2.5;

/// A rival's name card, up beside him.
#[derive(Clone, Debug, PartialEq)]
pub struct RivalCard {
    pub rival: RivalId,
    /// The duelist it follows, if any; else the enemy at this index; else where it was put up.
    pub foe: Option<u32>,
    pub enemy: Option<usize>,
    pub at: V,
    /// The game's time it went up.
    pub since: f64,
    /// What he said.
    pub bark: String,
}

/// The run's Warband: what is saved, and what is on screen of it.
#[derive(Clone, Debug, Default)]
pub struct Warband {
    pub save: WarSave,
    pub card: Option<RivalCard>,
    /// The Ledger board is open over the pause menu.
    pub board_open: bool,
}

impl Warband {
    /// What the last run saved, if a run is under way to continue (tests start from nothing).
    pub fn load() -> Warband {
        if cfg!(test) {
            return Warband::default();
        }
        Warband::read()
    }

    /// What the save holds, with a run under way (the Ledger goes with the run).
    pub(super) fn read() -> Warband {
        let run = platform::load(super::save::RUN_KEY).filter(|s| !s.trim().is_empty());
        let save = run.and_then(|_| platform::load(WARBAND_KEY)).and_then(|s| WarSave::decode(&s)).unwrap_or_default();
        Warband { save, ..Warband::default() }
    }
}

impl Game {
    /// The run's Ledger of Grudges.
    pub fn ledger(&self) -> &Ledger {
        &self.warband.save.ledger
    }

    /// The card up now, if any: its rival, where he stands, and how long it has been up.
    pub fn rival_card(&self) -> Option<(&Rival, &RivalCard, V, f64)> {
        let c = self.warband.card.as_ref()?;
        let age = self.time - c.since;
        if !(0.0..CARD_SECONDS).contains(&age) {
            return None;
        }
        let r = self.ledger().get(c.rival)?;
        let by_id = c.foe.and_then(|id| self.enemies.iter().find(|e| e.duel.as_ref().is_some_and(|d| d.id == id)));
        let by_index = c.enemy.and_then(|i| self.enemies.get(i));
        let at = by_id.or(by_index).map(|e| e.pos()).unwrap_or(c.at);
        Some((r, c, at, age))
    }

    /**
     * A rival comes within ten tiles (WARBAND.md §8.4): his name card beside him (following duelist
     * `foe`), and his bark, drawn from the presentation's rng. The integrator adds the 0.5×
     * dilation for 0.25 s through the juice when it lands.
     */
    pub fn meet_rival(&mut self, rival: RivalId, foe: Option<u32>, at: V) {
        let roll = self.war.pres.index(1 << 16);
        let hero = self.hero_kind;
        let Some(r) = self.ledger().get(rival) else { return };
        let said = bark(r, Situation::Meet, hero, roll);
        self.warband.card = Some(RivalCard { rival, foe, enemy: None, at, since: self.time, bark: said });
        // The old man names one true trait of his, if one is known.
        self.counsel_rival(rival);
    }

    /// What a rival says when he wins, flees or dies (the integrator floats it over him).
    pub fn rival_bark(&mut self, rival: RivalId, situation: Situation) -> Option<String> {
        let roll = self.war.pres.index(1 << 16);
        let hero = self.hero_kind;
        self.ledger().get(rival).map(|r| bark(r, situation, hero, roll))
    }

    /// A foe did a deed: he becomes a rival (seeded from the simulation's own rng, so a seeded run
    /// promotes the same names), and the run's Warband is saved.
    pub fn promote_rival(&mut self, kind: EnemyKind, deed: Deed) -> RivalId {
        let seed = (self.rng.next() * u32::MAX as f64) as u32;
        let hero = self.hero_kind;
        let id = self.warband.save.ledger.promote(kind, deed, hero, seed);
        self.save_warband();
        id
    }

    /// A fight with a rival is over: remembered, ranked up if he won it, saved.
    pub fn rival_fought(&mut self, rival: RivalId, e: Encounter) {
        self.warband.save.ledger.after_fight(rival, e);
        self.save_warband();
    }

    /// The Warband to the save, with the run: the snapshot is marked (`persist`), so a rival's
    /// fall and his reward are written together, at a calm moment.
    pub(super) fn save_warband(&mut self) {
        self.mark_saves();
    }

    /// Start over: the Ledger, the generals, the lies seen and the counsel's flags are gone.
    pub(super) fn wipe_warband(&mut self) {
        self.warband = Warband::default();
        self.voices = super::counsel::Voices::default();
        self.mark_saves();
    }

    /// THE LEDGER from the pause menu: the board over the paused game.
    pub fn open_ledger(&mut self) {
        self.warband.board_open = true;
        self.audio.play(Sfx::Whoosh);
        self.input.consume(Action::Confirm);
        self.input.consume(Action::Jump);
        self.input.consume(Action::Tap);
    }

    /// The board is up: any of the back keys (or a click) puts it away.
    pub(super) fn update_ledger_board(&mut self) {
        let back = [Action::Confirm, Action::Jump, Action::Guide, Action::Pause, Action::Tap];
        if !self.input.was_pressed(&back) && self.input.click.is_none() {
            return;
        }
        for a in back {
            self.input.consume(a);
        }
        self.input.consume(Action::Attack);
        self.input.click = None;
        self.warband.board_open = false;
        self.audio.play(Sfx::Nodule);
        if let Some(i) = self.menu_items().iter().position(|&m| m == MenuItem::Ledger) {
            self.menu_index = i;
        }
    }

    /// Capture hook (`ledger`): six made-up rivals across the ranks and the cultures, one of them
    /// slain, and the board open (over the pause menu when playing).
    pub fn debug_ledger(&mut self) {
        let hero = self.hero_kind;
        let l = &mut self.warband.save.ledger;
        *l = Ledger::default();
        let fight = |outcome: Outcome, verbs: Verbs, elixirs: Vec<PotionKind>, place: Place| Encounter {
            place,
            outcome,
            hero,
            family: Family::of(hero, 3),
            tier: 3,
            elixirs,
            verbs,
        };
        let cast: [(EnemyKind, Deed, u8, Verbs, Vec<PotionKind>, Place); 6] = [
            (
                EnemyKind::Karakoncolos,
                Deed::ThreeGuardBreaks,
                5,
                Verbs { blocks: 9, guard_breaks: 3, parries: 2, ..Verbs::default() },
                vec![],
                Place::Realm(Sin::Greed),
            ),
            (EnemyKind::Oni, Deed::KillingBlow, 4, Verbs { parries: 7, wounds: 5, ..Verbs::default() }, vec![], Place::Realm(Sin::Wrath)),
            (
                EnemyKind::FrostGiant,
                Deed::SurvivedLow,
                3,
                Verbs { blocks: 1, wounds: 9, ..Verbs::default() },
                vec![PotionKind::Fire],
                Place::Realm(Sin::Sloth),
            ),
            (EnemyKind::Witch, Deed::LeftBehind, 3, Verbs { evades: 6, ..Verbs::default() }, vec![], Place::Dungeon(1)),
            (EnemyKind::Gaki, Deed::TookSilver, 2, Verbs { wounds: 3, ..Verbs::default() }, vec![], Place::Realm(Sin::Gluttony)),
            (EnemyKind::Jorogumo, Deed::KillingBlow, 1, Verbs { evades: 2, ..Verbs::default() }, vec![], Place::Ring),
        ];
        let mut ids = Vec::new();
        for (k, (kind, deed, rank, verbs, elixirs, place)) in cast.into_iter().enumerate() {
            let id = l.promote(kind, deed, hero, 1000 + k as u32 * 77);
            // Each win a rank, each fight remembered (the lesson drawn from them).
            for _ in 1..rank {
                l.after_fight(id, fight(Outcome::RivalFled, verbs, elixirs.clone(), place));
            }
            if rank == 1 {
                l.record(id, fight(Outcome::RivalWon, verbs, elixirs.clone(), place));
            }
            ids.push(id);
        }
        // What the hero has found out: the first's weakness and nature, the second's strength...
        let reveal = |l: &mut Ledger, id: RivalId, which: [bool; 3]| {
            let traits = l.get(id).map(|r| r.true_traits()).unwrap();
            for (t, on) in traits.into_iter().zip(which) {
                if on {
                    l.reveal(id, t);
                }
            }
        };
        reveal(l, ids[0], [false, true, true]);
        reveal(l, ids[1], [true, false, false]);
        reveal(l, ids[3], [true, true, true]);
        l.took_silver(ids[4], 140);
        // ...and the third's lesson made the antidote's, to show it.
        if let Some(r) = l.get_mut(ids[2]) {
            r.lesson = Some(Lesson::Antidote(PotionKind::Fire));
        }
        l.slain(ids[5]);
        if self.screen == Screen::Playing {
            self.enter(Screen::Paused);
        }
        self.warband.board_open = true;
    }

    /// Capture hook (`rival:N`): the nearest foe made a rival of rank N (a lesson from rank 3, his
    /// weakness known from rank 2), his name card up over him.
    pub fn debug_rival(&mut self, rank: u8) {
        let hp = self.hero.pos();
        let near = (0..self.enemies.len()).filter(|&i| self.enemies[i].duel.as_ref().is_none_or(|d| d.alive())).min_by(|&a, &b| {
            let d = |i: usize| crate::logic::types::dist_sq(self.enemies[i].pos(), hp);
            d(a).total_cmp(&d(b))
        });
        let (kind, at, foe) = match near {
            Some(i) => (self.enemies[i].kind(), self.enemies[i].pos(), self.enemies[i].duel.as_ref().map(|d| d.id)),
            None => (EnemyKind::Karakoncolos, V::new(hp.x, hp.y - 2.0), None),
        };
        let hero = self.hero_kind;
        let l = &mut self.warband.save.ledger;
        let id = l.promote(kind, Deed::ThreeGuardBreaks, hero, 4242 + rank as u32);
        let verbs = Verbs { parries: 5, blocks: 2, guard_breaks: 1, ..Verbs::default() };
        for _ in 1..rank.clamp(1, 5) {
            l.after_fight(
                id,
                Encounter {
                    place: Place::Realm(Sin::Greed),
                    outcome: Outcome::RivalFled,
                    hero,
                    family: Family::Steel,
                    tier: 2,
                    elixirs: vec![],
                    verbs,
                },
            );
        }
        if rank >= 2 {
            let w = l.get(id).map(|r| Trait::Weakness(r.weakness)).unwrap();
            l.reveal(id, w);
        }
        self.meet_rival(id, foe, at);
        if foe.is_none() {
            if let (Some(c), Some(i)) = (self.warband.card.as_mut(), near) {
                c.enemy = Some(i);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::STEP;

    #[test]
    fn the_ledger_opens_from_the_pause_menu_and_is_part_of_the_run() {
        let mut g = Game::new();
        g.debug_dungeon(0);
        g.enter(Screen::Paused);
        let id = g.promote_rival(EnemyKind::Oni, Deed::KillingBlow);
        assert_eq!(g.ledger().live().count(), 1);
        // THE LEDGER: the board opens over the pause menu; Esc puts it away, still paused.
        g.menu_index = g.menu_items().iter().position(|&m| m == MenuItem::Ledger).unwrap();
        g.input.inject(Action::Confirm);
        g.update(STEP);
        assert!(g.warband.board_open);
        g.update(STEP);
        assert!(g.warband.board_open, "it stays up");
        g.input.inject(Action::Pause);
        g.update(STEP);
        assert!(!g.warband.board_open);
        assert_eq!(g.screen, Screen::Paused);
        assert_eq!(g.menu_items()[g.menu_index], MenuItem::Ledger, "THE LEDGER stays highlighted");
        // Met, his card is up beside him for two and a half seconds, his bark on it.
        g.meet_rival(id, None, V::new(3.0, 3.0));
        let (r, card, at, _) = g.rival_card().unwrap();
        assert_eq!((r.id, at), (id, V::new(3.0, 3.0)));
        assert!(!card.bark.is_empty());
        g.time += CARD_SECONDS + 0.1;
        assert!(g.rival_card().is_none());
        // A fight remembered; then Start over wipes the run's Ledger.
        g.rival_fought(
            id,
            Encounter {
                place: Place::Dungeon(0),
                outcome: Outcome::RivalWon,
                hero: g.hero_kind,
                family: Family::Steel,
                tier: 1,
                elixirs: vec![],
                verbs: Verbs::default(),
            },
        );
        assert_eq!(g.ledger().get(id).unwrap().rank, 2);
        g.answer_reset(true);
        assert!(g.ledger().rivals.is_empty(), "Start wipes the Ledger");
    }

    #[test]
    fn the_capture_hooks_fill_the_ledger_and_name_the_nearest_foe() {
        let mut g = Game::new();
        g.debug_dungeon(0);
        g.debug_ledger();
        assert_eq!(g.ledger().rivals.len(), 6);
        assert_eq!(g.ledger().live().count(), 5);
        assert!(g.ledger().live().any(|r| r.lesson.is_some()));
        assert!(g.warband.board_open);
        g.debug_rival(3);
        let (r, ..) = g.rival_card().unwrap();
        assert_eq!(r.rank, 3);
        assert!(r.card().lesson.is_some(), "a rank 3 rival carries his lesson on his card");
    }
}
