//! The old man's workshop: his wares as a place. Six stations stand round one
//! room (the anvil, the shield wall, the alchemy bench, the training yard,
//! the war table and the trophy wall), and each wraps a tab of the wares
//! (`ShopState` is kept: a tree's tab is its station's). LT, RT and Tab step
//! from station to station, the camera following and the old man walking
//! over to work there; Quitmer follows the hero. What a station shows of the
//! war below (the trophies taken, the old man's news, the generals known)
//! comes from `WorkshopData`, which later phases fill.

use super::{Game, ShopState};
use crate::audio::Sfx;
use crate::logic::gear::BELT_SLOTS;
use crate::logic::heroes::HeroKind;
use crate::logic::progression::{Tree, TREES};
use crate::logic::sins::Sin;
use crate::logic::types::PotionKind;

/// A place in the workshop where something is done.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub enum Station {
    /// Weapons: the racks, the hearth, the old man's hammer (and the loadout's whetstone).
    #[default]
    Anvil,
    /// Shields by tier, mended and reforged.
    ShieldWall,
    /// Elixirs on the shelves and the belt laid out.
    Bench,
    /// The path, and (later) mastery and its trials.
    Yard,
    /// The hub on a map table, the generals' sigils, the Ledger pinned beside it.
    WarTable,
    /// What the hero took from below.
    Trophies,
}

/// The stations in the order LT and RT step through them (the loadout follows the last).
pub const STATIONS: [Station; 6] =
    [Station::Anvil, Station::ShieldWall, Station::Bench, Station::Yard, Station::WarTable, Station::Trophies];

/// How many pages LT and RT step through: the six stations and the loadout.
pub const PAGES: usize = STATIONS.len() + 1;

/// How many hooks the trophy wall has.
pub const TROPHY_SLOTS: usize = 10;

impl Station {
    pub fn index(self) -> usize {
        STATIONS.iter().position(|&s| s == self).unwrap_or(0)
    }

    /// The tree whose wares are sold here, if any.
    pub fn tree(self) -> Option<Tree> {
        match self {
            Station::Anvil => Some(Tree::Armory),
            Station::ShieldWall => Some(Tree::Shield),
            Station::Bench => Some(Tree::Elixir),
            Station::Yard => Some(Tree::Path),
            Station::WarTable | Station::Trophies => None,
        }
    }

    /// Where a tree's wares are sold.
    pub fn of_tree(tree: Tree) -> Station {
        match tree {
            Tree::Armory => Station::Anvil,
            Tree::Shield => Station::ShieldWall,
            Tree::Elixir => Station::Bench,
            Tree::Path => Station::Yard,
        }
    }

    /// Its name over the panel.
    pub fn name(self) -> &'static str {
        match self {
            Station::Anvil => "THE ANVIL",
            Station::ShieldWall => "THE SHIELD WALL",
            Station::Bench => "THE ALCHEMY BENCH",
            Station::Yard => "THE TRAINING YARD",
            Station::WarTable => "THE WAR TABLE",
            Station::Trophies => "THE TROPHY WALL",
        }
    }

    /// Its tab's label.
    pub fn label(self) -> &'static str {
        match self {
            Station::Anvil => "ANVIL",
            Station::ShieldWall => "SHIELDS",
            Station::Bench => "BENCH",
            Station::Yard => "YARD",
            Station::WarTable => "WAR TABLE",
            Station::Trophies => "TROPHIES",
        }
    }

    /// Does the old man take up his hammer when something is bought here?
    pub fn hammers(self) -> bool {
        matches!(self, Station::Anvil | Station::ShieldWall)
    }
}

/// Quitmer at the workshop: lying calm, wagging (something was bought), or growling (a hook
/// for later: the prince's fakes, a rival's trophy).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum QuitmerMood {
    #[default]
    Calm,
    Wag,
    Growl,
}

/// Something the hero took from below, hung on the trophy wall: a rival's trophy, a general's
/// banner, a prince's key. (A later phase owns it; this is its placeholder.)
#[derive(Clone, Debug, PartialEq)]
pub struct Trophy {
    pub name: String,
    /// What the old man says of it.
    pub line: String,
    pub color: u32,
}

/// A general known at the war table: whose realm, his name, and what is known of him.
/// (A later phase owns it; this is its placeholder.)
#[derive(Clone, Debug, PartialEq)]
pub struct GeneralCard {
    pub sin: Sin,
    pub name: String,
    pub note: String,
}

/// What the workshop shows of the war below. Filled by later phases (the Ledger, the
/// generals, the old man's counsel); empty, the stations stand ready.
#[derive(Clone, Debug, Default)]
pub struct WorkshopData {
    pub trophies: Vec<Trophy>,
    /// The old man has news: calling him opens at the war table.
    pub news: Vec<String>,
    /// The seven realms' generals, in the order of `SINS`.
    pub generals: [Option<GeneralCard>; 7],
}

/// The old man at work at a station: seconds since he began, and the colour of what he makes.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Work {
    pub station: Station,
    pub t: f32,
    pub color: u32,
}

/// When the hammer falls, in seconds from the start of the work (the last strike is the heavy one).
pub const STRIKES: [f32; 3] = [0.45, 0.95, 1.55];
/// How long a piece of work lasts.
pub const WORK_SECONDS: f32 = 2.3;
/// How long Quitmer wags after a sale.
const WAG_SECONDS: f32 = 2.6;

/// The workshop as the renderer needs it: where the hero stands, what he owns and carries,
/// what the old man is doing and how Quitmer is, and what hangs on the walls.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WorkshopView {
    pub station: Station,
    pub hero: HeroKind,
    /// Highest tier owned per tree (in the order of `Tree::index`).
    pub owned: [u8; 4],
    pub wield: u8,
    /// The shield on his arm, if any.
    pub carry: Option<u8>,
    pub belt: [Option<PotionKind>; BELT_SLOTS],
    pub work: Option<Work>,
    pub mood: QuitmerMood,
    /// The trophies' colours, hook by hook.
    pub trophies: [Option<u32>; TROPHY_SLOTS],
    /// The generals known, by realm: their sin's colour on the war table.
    pub sigils: [Option<u32>; 7],
    /// How many pieces of news are pinned up.
    pub news: u8,
}

impl Game {
    /// The station the hero stands at: the open tree's, the war table's or the trophy wall's,
    /// or (for the loadout) the anvil, where the whetstone is.
    pub fn station(&self) -> Station {
        station_of(&self.shop)
    }

    /// The page LT and RT are on: a station (0..6) or the loadout (6).
    pub(super) fn page(&self) -> usize {
        if self.shop.loadout {
            STATIONS.len()
        } else {
            self.station().index()
        }
    }

    /// Opens page `i`: a station (a tree's tab, the war table, the trophy wall) or the loadout.
    pub(super) fn open_page(&mut self, i: usize) {
        self.shop.loadout = i >= STATIONS.len();
        self.shop.board = None;
        if let Some(&s) = STATIONS.get(i) {
            match s.tree() {
                Some(t) => self.shop.tab = t,
                None => self.shop.board = Some(s),
            }
        }
    }

    /// Straight to a station (from a call, a capture or the map).
    pub fn go_to_station(&mut self, s: Station) {
        self.open_page(s.index());
        self.shop.row = self.first_row(s);
        self.shop.col = 0;
    }

    /// The first row worth the eye at a station: the next tier to take, or the top.
    fn first_row(&self, s: Station) -> usize {
        match s.tree() {
            Some(t) if !self.shop.loadout => (self.progress.hero(self.hero_kind).owned(t) as usize + 1).min(10),
            _ => 1,
        }
    }

    /// Something was made: the old man sets to work at the station, and Quitmer wags.
    pub(super) fn start_work(&mut self, color: u32) {
        self.shop.work = Some(Work { station: self.station(), t: 0.0, color });
        self.set_quitmer_mood(QuitmerMood::Wag, WAG_SECONDS);
    }

    /// Quitmer's mood for `seconds` (then he lies calm again).
    pub fn set_quitmer_mood(&mut self, mood: QuitmerMood, seconds: f32) {
        self.shop.mood = mood;
        self.shop.mood_t = seconds;
    }

    /// The work going on: the hammer's strikes ring out (the last heavy, with a jolt), and
    /// Quitmer settles once the wagging is done.
    pub(super) fn update_workshop(&mut self, dt: f32) {
        if let Some(w) = self.shop.work.as_mut() {
            let before = w.t;
            w.t += dt;
            let (station, now) = (w.station, w.t);
            if now > WORK_SECONDS {
                self.shop.work = None;
            }
            if station.hammers() {
                for (k, &at) in STRIKES.iter().enumerate() {
                    if before < at && now >= at {
                        self.audio.play(Sfx::Anvil);
                        if k == STRIKES.len() - 1 {
                            self.audio.play(Sfx::Clang);
                            self.view.shake(0.18);
                        }
                    }
                }
            }
        }
        if self.shop.mood_t > 0.0 {
            self.shop.mood_t -= dt;
            if self.shop.mood_t <= 0.0 {
                self.shop.mood = QuitmerMood::Calm;
            }
        }
    }

    /// The workshop for the renderer.
    pub fn workshop_view(&self) -> WorkshopView {
        let h = self.progress.hero(self.hero_kind);
        let mut belt = [None; BELT_SLOTS];
        for (slot, &k) in belt.iter_mut().zip(&self.belt) {
            *slot = Some(k);
        }
        let mut trophies = [None; TROPHY_SLOTS];
        for (slot, t) in trophies.iter_mut().zip(&self.workshop.trophies) {
            *slot = Some(t.color);
        }
        WorkshopView {
            station: self.station(),
            hero: self.hero_kind,
            owned: TREES.map(|t| h.owned(t)),
            wield: h.wield,
            carry: self.hero.shield.map(|s| s.tier),
            belt,
            work: self.shop.work,
            mood: self.shop.mood,
            trophies,
            sigils: std::array::from_fn(|i| self.workshop.generals[i].as_ref().map(|g| g.sin.color())),
            news: self.workshop.news.len().min(255) as u8,
        }
    }
}

/// Sample news, trophies and generals, to see the workshop's walls dressed (a capture hook).
pub fn sample() -> WorkshopData {
    use crate::logic::sins::SINS;
    let trophy = |name: &str, line: &str, color: u32| Trophy { name: name.into(), line: line.into(), color };
    let mut generals: [Option<GeneralCard>; 7] = Default::default();
    generals[4] = Some(GeneralCard {
        sin: SINS[4],
        name: "The Steward of the Vault".into(),
        note: "He counts the doors; break his ledger-horn and the vault forgets you.".into(),
    });
    generals[2] =
        Some(GeneralCard { sin: SINS[2], name: "The Red Herald".into(), note: "He musters with a horn. Kill the herald first.".into() });
    WorkshopData {
        trophies: vec![
            trophy("Kara Ayaz's Fang", "He bit at your shield three times. The fourth time you were not there.", 0xd8ccb0),
            trophy("The Bent Crown", "A captain of the vaults wore this. He wears nothing now.", 0xf2c53d),
            trophy("A Raven's Banner", "Torn from the herald's pole. They will look for it.", 0x7a3424),
        ],
        news: vec!["The general of Greed has set a watch on the vault doors. Go round by the cisterns.".into()],
        generals,
    }
}

/// The station a shop state stands at.
pub fn station_of(shop: &ShopState) -> Station {
    if shop.loadout {
        Station::Anvil
    } else {
        shop.board.unwrap_or(Station::of_tree(shop.tab))
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::playing;
    use super::*;
    use crate::game::{RowKind, Screen, ShopAction};
    use crate::input::Action;

    #[test]
    fn lt_and_rt_step_round_the_six_stations_and_the_loadout() {
        let mut g = playing();
        g.open_shop();
        assert_eq!(g.station(), Station::Anvil, "the wares open at the anvil");
        let mut seen = vec![g.station()];
        for _ in 0..PAGES - 1 {
            g.input.inject(Action::TabNext);
            g.update(crate::app::STEP);
            seen.push(g.station());
        }
        assert_eq!(&seen[..6], &STATIONS[..], "in the order they stand");
        assert!(g.shop.loadout, "the loadout after the trophy wall");
        assert_eq!(g.station(), Station::Anvil, "the loadout is kept at the anvil");
        g.input.inject(Action::TabNext);
        g.update(crate::app::STEP);
        assert_eq!((g.station(), g.shop.loadout), (Station::Anvil, false), "round to the anvil again");
        g.input.inject(Action::TabPrev);
        g.update(crate::app::STEP);
        assert!(g.shop.loadout, "and back");
        // Each tree's station sells that tree; the two without wares show their boards.
        for s in STATIONS {
            g.go_to_station(s);
            let rows = g.shop_rows();
            match s.tree() {
                Some(t) => assert!(rows.iter().any(|r| r.kind == RowKind::Tier(t, 1)), "{s:?} sells {t:?}"),
                None => assert!(rows.iter().all(|r| !matches!(r.kind, RowKind::Tier(..))), "{s:?} sells nothing"),
            }
            assert_eq!(rows.last().map(|r| r.kind), Some(RowKind::Leave), "{s:?}: Esc goes to the door");
        }
        // The yard holds a place for the trials, shut for now.
        g.go_to_station(Station::Yard);
        let trials = g.shop_rows().into_iter().find(|r| r.kind == RowKind::Trials).expect("a trials row");
        assert!(!trials.buttons[0].enabled);
        // The tabs name the stations.
        let tabs = &g.shop_rows()[0].buttons;
        assert_eq!(tabs.len(), PAGES);
        assert_eq!(tabs[4].action, ShopAction::Board(Station::WarTable));
    }

    #[test]
    fn calling_him_opens_at_the_war_table_when_he_has_news_else_where_you_left() {
        let mut g = playing();
        g.open_shop();
        g.go_to_station(Station::Bench);
        g.input.inject(Action::Pause);
        g.update(crate::app::STEP);
        g.input.inject(Action::Pause);
        g.update(crate::app::STEP);
        assert_ne!(g.screen, Screen::Shop, "left");
        g.enter(Screen::Playing);
        g.call_old_man();
        assert_eq!((g.screen, g.station()), (Screen::Shop, Station::Bench), "where he left");
        g.enter(Screen::Playing);
        g.workshop.news.push("The general of Greed has set a watch on the vault doors.".into());
        g.call_old_man();
        assert_eq!(g.station(), Station::WarTable, "news first");
        assert!(g.shop.called);
    }

    #[test]
    fn the_maps_workshop_stop_opens_the_workshop() {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        let mut g = Game::new();
        let stop = crate::game::map_stops().into_iter().find(|s| s.label.contains("WORKSHOP")).expect("a workshop stop");
        g.map_jump(stop.stage);
        assert_eq!((g.screen, g.station()), (Screen::Shop, Station::Anvil));
        assert!(!g.shop.called, "leaving goes on down, as after a Cyclops");
    }

    #[test]
    fn a_sale_at_the_anvil_rings_three_strikes_and_quitmer_wags() {
        let mut g = playing();
        g.open_shop();
        g.progress.silver = 5000;
        let rows = g.shop_rows();
        let r = rows.iter().position(|r| r.kind == RowKind::Tier(Tree::Armory, 2)).unwrap();
        g.shop.row = r;
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        let w = g.shop.work.expect("he sets to work");
        assert_eq!(w.station, Station::Anvil);
        assert_eq!(g.workshop_view().mood, QuitmerMood::Wag);
        for _ in 0..((WORK_SECONDS + WAG_SECONDS) / crate::app::STEP as f32) as usize + 2 {
            g.update(crate::app::STEP);
        }
        assert!(g.shop.work.is_none(), "done");
        assert_eq!(g.shop.mood, QuitmerMood::Calm, "settled");
    }
}
