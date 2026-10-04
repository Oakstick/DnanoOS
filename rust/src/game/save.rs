//! The adventure is saved at the start of every stage (a dungeon, the
//! Cyclops' arena, the old man's wares after it), so Continue on the title
//! picks up where the player left off: the same hero, the same stage, the
//! score, lives and belt he had when he reached it. Start begins a new
//! adventure and wipes everything earned, after asking.

use super::{Game, MenuItem, Screen};
use crate::audio::Sfx;
use crate::input::Action;
use crate::logic::heroes::{HeroKind, ROSTER};
use crate::logic::progression::Progress;
use crate::logic::sins::{Sin, ALL_SINS, SINS};
use crate::logic::types::PotionKind;
use crate::platform;

/// Where the adventure's checkpoint is saved.
const RUN_KEY: &str = "vh.run";

/// Every brew, in the order the save names them.
const BREWS: [PotionKind; 11] = [
    PotionKind::Health,
    PotionKind::Stamina,
    PotionKind::Strength,
    PotionKind::Haste,
    PotionKind::Bark,
    PotionKind::Fury,
    PotionKind::Fire,
    PotionKind::Invulnerability,
    PotionKind::Ragnarok,
    PotionKind::Spirit,
    PotionKind::Life,
];

/// Where a saved adventure stands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    /// At the start of dungeon `n`.
    Dungeon(usize),
    /// In the Cyclops' arena after dungeon `n`.
    Arena(usize),
    /// Cyclops `n` is slain: at the old man's wares, before dungeon `n + 1`.
    Shop(usize),
    /// In the Portals of Hell.
    Hub,
    /// In the realm of a sin.
    Realm(Sin),
    /// In the court of its prince.
    Court(Sin),
    /// The prince is slain: at the old man's wares, before the way back to the hub.
    SinShop(Sin),
    /// The gym, with one of every foe.
    Gym,
    /// At the start of level `n` (1 to 7) of the Pit.
    Pit(u8),
    /// Krampus's lair at the bottom of the Pit (FINALE.md §3).
    Lair,
}

/// A checkpoint of the adventure under way.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Run {
    pub hero: HeroKind,
    pub stage: Stage,
    pub score: u32,
    pub lives: u32,
    pub belt: Vec<PotionKind>,
    /// The sins whose keys he holds (`Sin::bit`).
    pub sins: u8,
}

impl Run {
    /// A line for the save file: "v2 hero stage dungeon score lives belt sins" (for a realm, its
    /// court or the wares after it, the number is the sin's).
    /// (A "v1" line, from before the Portals of Hell, has no sins and still loads.)
    pub fn encode(&self) -> String {
        let (kind, n) = match self.stage {
            Stage::Dungeon(n) => ('d', n),
            Stage::Arena(n) => ('a', n),
            Stage::Shop(n) => ('s', n),
            Stage::Hub => ('h', 0),
            Stage::Realm(s) => ('r', s.index()),
            Stage::Court(s) => ('c', s.index()),
            Stage::SinShop(s) => ('e', s.index()),
            Stage::Gym => ('g', 0),
            Stage::Pit(n) => ('p', n as usize),
            Stage::Lair => ('k', 0),
        };
        let belt: Vec<String> = self.belt.iter().filter_map(|b| BREWS.iter().position(|x| x == b)).map(|i| i.to_string()).collect();
        let belt = if belt.is_empty() { "-".to_string() } else { belt.join(",") };
        format!("v2 {} {kind} {n} {} {} {belt} {}", self.hero.slot(), self.score, self.lives, self.sins)
    }

    pub fn decode(text: &str) -> Option<Run> {
        let mut f = text.split_whitespace();
        let version = f.next()?;
        if version != "v1" && version != "v2" {
            return None;
        }
        let hero = *ROSTER.get(f.next()?.parse::<usize>().ok()?)?;
        let kind = f.next()?;
        let n: usize = f.next()?.parse().ok()?;
        let stage = match kind {
            "d" => Stage::Dungeon(n),
            "a" => Stage::Arena(n),
            "s" => Stage::Shop(n),
            "h" => Stage::Hub,
            "r" => Stage::Realm(*SINS.get(n)?),
            "c" => Stage::Court(*SINS.get(n)?),
            "e" => Stage::SinShop(*SINS.get(n)?),
            "g" => Stage::Gym,
            "p" if (1..=crate::logic::pit::LEVELS as usize).contains(&n) => Stage::Pit(n as u8),
            "k" => Stage::Lair,
            _ => return None,
        };
        let score = f.next()?.parse().ok()?;
        let lives = f.next()?.parse::<u32>().ok()?.max(1);
        let belt = match f.next()? {
            "-" => Vec::new(),
            list => list.split(',').map(|i| i.parse::<usize>().ok().and_then(|i| BREWS.get(i).copied())).collect::<Option<_>>()?,
        };
        let sins = if version == "v2" { f.next()?.parse::<u8>().ok()? & ALL_SINS } else { 0 };
        Some(Run { hero, stage, score, lives, belt, sins })
    }
}

/// The adventure saved last time, if there is one (tests start from nothing).
pub(super) fn load_run() -> Option<Run> {
    if cfg!(test) {
        return None;
    }
    platform::load(RUN_KEY).and_then(|s| Run::decode(&s))
}

impl Game {
    /// Is there anything for Continue to pick up: an adventure under way, or silver and unlocks?
    pub fn can_continue(&self) -> bool {
        self.saved_run.is_some() || self.progress != Progress::default()
    }

    /// What Continue leads back to, for the line under its button.
    pub fn continue_caption(&self) -> Option<String> {
        let run = self.saved_run.as_ref()?;
        let place = match run.stage {
            Stage::Dungeon(n) => format!("dungeon {}", n + 1),
            Stage::Arena(n) => format!("the Cyclops of dungeon {}", n + 1),
            Stage::Shop(_) => "the old man's wares".to_string(),
            Stage::Hub => "the Portals of Hell".to_string(),
            Stage::Realm(s) => format!("the realm of {}", s.name()),
            Stage::Court(s) => format!("the court of {}", s.prince()),
            Stage::SinShop(_) => "the old man's wares".to_string(),
            Stage::Gym => "the gym".to_string(),
            Stage::Pit(n) => format!("level {n} of the Pit"),
            Stage::Lair => "Krampus's lair".to_string(),
        };
        Some(format!("{}, {place}", run.hero.name()))
    }

    /// Saves the adventure as it stands, at the start of `stage`.
    pub(super) fn checkpoint(&mut self, stage: Stage) {
        let run = Run {
            hero: self.hero_kind,
            stage,
            score: self.score.score,
            lives: self.score.lives,
            belt: self.belt.clone(),
            sins: self.sins_cleared,
        };
        self.write_run(Some(run));
    }

    /// Bottles bought at the old man's stay on the belt when the game is continued.
    pub(super) fn remember_belt(&mut self) {
        if let Some(mut run) = self.saved_run.clone() {
            run.belt = self.belt.clone();
            self.write_run(Some(run));
        }
    }

    fn write_run(&mut self, run: Option<Run>) {
        if !cfg!(test) {
            platform::save(RUN_KEY, &run.as_ref().map(Run::encode).unwrap_or_default());
        }
        self.saved_run = run;
        // The run's Warband (the Ledger of Grudges) is written with it.
        self.save_warband();
    }

    /// Continue: straight back into the saved stage, skipping the story and the heroes.
    /// With unlocks but no adventure under way yet, the character screen.
    pub(super) fn continue_game(&mut self) {
        self.audio.unlock();
        self.input.consume(Action::Attack);
        self.input.consume(Action::Confirm);
        let Some(run) = self.saved_run.clone() else {
            self.open_select();
            return;
        };
        self.audio.play(Sfx::Start);
        self.hero_kind = run.hero;
        self.tour = None;
        self.tour_pending = false;
        self.score.resume(run.score, run.lives);
        self.juice.score_shown = run.score as f64;
        self.new_kit();
        self.belt = run.belt;
        self.sins_cleared = run.sins;
        match run.stage {
            Stage::Dungeon(n) => {
                self.load_level(n);
                self.enter(Screen::Ready);
            }
            Stage::Arena(n) => {
                self.load_boss(n);
                self.enter(Screen::Ready);
            }
            Stage::Shop(n) => {
                // Leaving the wares goes on to the next dungeon, as it did.
                self.load_level(n);
                self.open_shop();
            }
            Stage::Hub => {
                self.load_hub();
                self.enter(Screen::Ready);
            }
            Stage::Realm(s) => {
                self.load_realm(s);
                self.enter(Screen::Ready);
            }
            Stage::Court(s) => {
                self.load_court(s);
                self.enter(Screen::Ready);
            }
            Stage::SinShop(s) => {
                // Leaving the wares goes back to the hub with the sin's key.
                self.load_court(s);
                self.open_shop();
            }
            Stage::Gym => {
                self.load_gym();
                self.enter(Screen::Ready);
            }
            Stage::Pit(n) => {
                self.load_pit(n);
                self.enter(Screen::Ready);
            }
            Stage::Lair => {
                self.load_lair();
                self.enter(Screen::Ready);
            }
        }
        // Back into play: the camera is chosen first.
        if self.screen == Screen::Ready {
            self.ask_view(false);
        }
        self.juice.fade = 1.0;
    }

    /// Start on the title: a new adventure. With something saved, it asks first.
    pub(super) fn start_from_title(&mut self) {
        if self.can_continue() {
            self.confirm_reset = true;
            // "Keep my progress" is highlighted, so a stray Enter loses nothing.
            self.menu_index = 1;
            self.audio.play(Sfx::Denied);
        } else {
            self.play_from_title();
        }
    }

    /// The answer to "start over?".
    pub(super) fn answer_reset(&mut self, erase: bool) {
        self.confirm_reset = false;
        self.menu_index = 0;
        self.input.consume(Action::Attack);
        self.input.consume(Action::Confirm);
        if erase {
            self.reset_everything();
            self.play_from_title();
        }
    }

    /// Every hero's unlocks, the silver, the runes and the adventure under way are gone,
    /// and the first dungeon's tour is owed again. The best score stays on the board.
    fn reset_everything(&mut self) {
        self.progress = Progress::default();
        self.save_progress();
        self.write_run(None);
        self.wipe_warband();
        self.hero_kind = HeroKind::Ragnar;
        self.sins_cleared = 0;
        if !cfg!(test) {
            super::tour::forget_tour();
            self.tour_pending = true;
        }
    }

    /// The title's buttons: Continue (when there is something to continue), Start, the
    /// controller page and the map, or the two answers while it asks whether to start over.
    pub(super) fn title_items(&self) -> Vec<MenuItem> {
        if self.confirm_reset {
            vec![MenuItem::StartOver, MenuItem::KeepProgress]
        } else if self.can_continue() {
            vec![MenuItem::Continue, MenuItem::Play, MenuItem::Controls, MenuItem::Map]
        } else {
            vec![MenuItem::Play, MenuItem::Controls, MenuItem::Map]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_survives_the_save_file() {
        let runs = [
            Run {
                hero: HeroKind::Miyamoto,
                stage: Stage::Arena(2),
                score: 12_345,
                lives: 2,
                belt: vec![PotionKind::Health, PotionKind::Life, PotionKind::Spirit],
                sins: 0,
            },
            Run { hero: HeroKind::Ragnar, stage: Stage::Dungeon(0), score: 0, lives: 3, belt: Vec::new(), sins: 0 },
            Run { hero: HeroKind::Mehmed, stage: Stage::Shop(4), score: 99, lives: 1, belt: vec![PotionKind::Ragnarok], sins: 0b101_0011 },
            Run { hero: HeroKind::Ragnar, stage: Stage::Hub, score: 5, lives: 2, belt: Vec::new(), sins: 0x7f },
            Run { hero: HeroKind::Ragnar, stage: Stage::Realm(Sin::Wrath), score: 5, lives: 2, belt: Vec::new(), sins: 0x03 },
            Run { hero: HeroKind::Miyamoto, stage: Stage::Court(Sin::Lust), score: 5, lives: 2, belt: Vec::new(), sins: 0x03 },
            Run { hero: HeroKind::Mehmed, stage: Stage::SinShop(Sin::Pride), score: 5, lives: 2, belt: Vec::new(), sins: 0x01 },
            Run { hero: HeroKind::Ragnar, stage: Stage::Gym, score: 0, lives: 3, belt: Vec::new(), sins: 0 },
            Run { hero: HeroKind::Mehmed, stage: Stage::Pit(4), score: 77, lives: 2, belt: vec![PotionKind::Health], sins: 0x7f },
            Run { hero: HeroKind::Mehmed, stage: Stage::Lair, score: 7, lives: 2, belt: Vec::new(), sins: 0x7f },
        ];
        for run in runs {
            assert_eq!(Run::decode(&run.encode()), Some(run));
        }
        // A save from before the Portals of Hell still loads, with no sins cleared.
        assert_eq!(Run::decode("v1 1 d 2 450 3 -").map(|r| (r.stage, r.sins)), Some((Stage::Dungeon(2), 0)));
        assert_eq!(Run::decode(""), None, "a wiped save holds no run");
        assert_eq!(Run::decode("v1 7 d 0 0 3 -"), None);
        assert_eq!(Run::decode("v2 0 p 8 0 3 - 127"), None, "the Pit has seven levels");
    }
}
