//! The whole run, played by one bot (WARBAND.md §16, W9): the seven realms in a fixed order with
//! their generals and the princes' courts, the House door, the Pit's seven levels, Krampus and the
//! ending, with one game (one save, one purse, one Ledger) carried through it all. The tier rises
//! with the keys as the game's does (`Game::realm_tier`); the kit is what the bot bought at the old
//! man's between the parts (`shop`), and its mastery what it earned on the way.
//!
//! What the bots cannot play, and what stands in for it:
//!
//! * **The Garden and its Cyclops**: no bot walks the Garden. The run begins at the old man's after
//!   the Cyclops, with what a player brings out of the Garden: every golden key's silver, the
//!   Cyclops' bounty and runes and his relic (`garden_purse`); no kills are counted there.
//! * **Five realms' ground**: the bots read the vaults and the Rotting Feast as a player does
//!   (`bots_walk`), but not the spatial lies of the other five (Pride's mirrors, Envy's two layers,
//!   the Field's drifting slabs, the Mire's turning land, the Garden of Thorns' facades). Their
//!   fights are played on the realm's own ground with its rules, and when the bot has gone
//!   `GUIDE_AFTER` seconds with no blow landing either way (twice that without a kill), or stood
//!   `STUCK_AFTER` in one place with nothing hunting it, the harness carries it to the doorstep of
//!   the nearest den still standing (`guide`), as a player who has learnt the realm walks there;
//!   each carry is counted (`carried`). Such a realm is given `STAND_IN_LIMIT`, and lost to its
//!   ground it does not end the run: its fights are counted and the run goes on to the court.
//!   (The bots were taught the least they need there: off the Field's lava, and an elf-maid
//!   struck from behind; `Bot::floor`.)
//! * **The way from a cleared realm to its court**: once every foe he can reach is down (or the
//!   realm's time is up) the run goes to the court (the gate's last trap is not played).
//! * **The trials** at the yard: when a rank's deeds are met, the bot fights the trial's bout of the
//!   ring with its kit and passes on a win (the trial's own deed count is not asked).
//!
//! The bot visits the old man (`shop`) before each court and after it, as H does anywhere, and at
//! the bottom of the Pit before Krampus. A fight lost (all lives gone, or not won in its time) is
//! played again from its checkpoint by Continue, as a player would, up to `TRIES` times; the run
//! ends at a part lost that often. `--run SKILL SEED [ORDER]` prints one run part by part,
//! `--runs SKILL N [ORDER]` the parts of N seeds side by side.

use super::bot::{fresh_game, play_bout_mastered, Bot, Loadout, Skill};
use super::save::Stage;
use super::{Game, Screen};
use crate::input::PadState;
use crate::logic::combat::State;
use crate::logic::gear::{self, ShieldGear, BELT_SLOTS};
use crate::logic::heroes::HeroKind;
use crate::logic::progression::{self, Tree};
use crate::logic::sins::Sin;
use crate::logic::types::{PotionKind, P, V};
use crate::render::view::CameraMode;
use std::fmt::Write;

/// The two orders the run is played in: the vaults first (the realm the bots know), and the
/// reverse, so every realm is met both early and late.
pub const ORDERS: [[Sin; 7]; 2] = [
    [Sin::Greed, Sin::Pride, Sin::Envy, Sin::Wrath, Sin::Sloth, Sin::Gluttony, Sin::Lust],
    [Sin::Lust, Sin::Gluttony, Sin::Sloth, Sin::Wrath, Sin::Envy, Sin::Pride, Sin::Greed],
];

/// Tries a part allows (the first, then Continue twice) before the run ends there.
pub const TRIES: u32 = 3;
/// Seconds a try at each kind of part may run.
pub const REALM_LIMIT: f64 = 900.0;
/// ...and a realm the bots cannot read (`bots_walk`): its own rules (the risen dead, the turning
/// land) would keep a bot fighting there for ever.
pub const STAND_IN_LIMIT: f64 = 600.0;
pub const COURT_LIMIT: f64 = 480.0;
pub const PIT_LIMIT: f64 = 1200.0;
pub const LAIR_LIMIT: f64 = 600.0;
/// Seconds after Krampus falls for the boy to reach him and Scene II to begin.
const ENDING_LIMIT: f64 = 40.0;
/// Seconds without a blow landing either way before the bot is carried to the next den (or twice
/// that without a kill).
pub const GUIDE_AFTER: f64 = 30.0;
/// Seconds standing in one place with nothing hunting it before the bot is carried on.
pub const STUCK_AFTER: f64 = 6.0;
/// Silver the shopping rule keeps back for bottles.
pub const RESERVE: u32 = 60;
/// Below this edge the bot has its blade sharpened.
const SHARPEN_BELOW: f64 = 0.7;
/// Frames a foe that came in mid-fight must stand seen before its first tell (WARBAND.md I7).
pub const TELL_FRAMES: u32 = 60;

/// What a part of the run is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartKind {
    Realm(Sin),
    Court(Sin),
    Pit,
    Lair,
}

impl PartKind {
    pub fn name(self) -> String {
        match self {
            PartKind::Realm(s) => format!("realm {}", s.name().to_lowercase()),
            PartKind::Court(s) => format!("court {}", s.prince().to_lowercase()),
            PartKind::Pit => "the pit".into(),
            PartKind::Lair => "krampus".into(),
        }
    }
}

/// How one part of the run went (every try of it added up).
#[derive(Clone, Debug)]
pub struct Part {
    pub kind: PartKind,
    /// The tier it was played at, and the kit (blade, shield, path) and mastery rank it was met with.
    pub tier: usize,
    pub kit: (u8, u8, u8),
    pub rank: u8,
    /// His mastery's deeds at its start (WARBAND.md §11.1).
    pub deeds: u32,
    /// Every foe down, the prince or Krampus slain, the bottom reached.
    pub done: bool,
    /// Tries lost (all lives gone) before it was done or given up.
    pub game_overs: u32,
    pub secs: f64,
    /// Lives lost, the lives left after it.
    pub deaths: u32,
    pub lives: u32,
    pub taken: f64,
    pub kills: u32,
    /// Foes in it at the start (a realm's), and standing at its end.
    pub foes: u32,
    pub left: u32,
    /// Rivals met (their cards shown; each once a try), made, slain.
    pub met: u32,
    pub made: u32,
    pub rivals_slain: u32,
    /// The Drum: seconds in rest, build, peak and release; peaks; the rests between them.
    pub spent: [f64; 4],
    pub peaks: u32,
    pub rests: (f64, u32),
    /// The breaths between peaks (seconds in all, how many).
    pub gaps: (f64, u32),
    pub routs: u32,
    /// The general's instruments broken (of three), and whether he fell.
    pub broken: usize,
    pub general_slain: bool,
    pub overlaps: u32,
    /// Foes that came in mid-fight and could land a blow within `TELL_FRAMES` of being seen.
    pub sudden: u32,
    /// Silver and runes come in during it, silver lost (a rival's take, Mammon's grasp).
    pub silver_in: u32,
    pub silver_lost: u32,
    pub runes_in: u32,
    /// Spent at the old man's after it, and what was bought.
    pub spent_silver: u32,
    pub spent_runes: u32,
    pub bought: Vec<String>,
    /// Edge his blade lost (1 is keen to rusted through), and the least it was at.
    pub worn: f64,
    pub dullest: f64,
    /// Times the harness carried it to a den (see the module's note).
    pub carried: u32,
    /// The Pit's levels.
    pub pit: Vec<super::pit::LevelTally>,
}

impl Part {
    fn new(kind: PartKind, g: &Game) -> Self {
        let h = g.progress.hero(g.hero_kind);
        Part {
            kind,
            tier: g.realm_tier(),
            kit: (h.owned(Tree::Armory), h.owned(Tree::Shield), h.owned(Tree::Path)),
            rank: h.mastery.rank,
            deeds: h.mastery.total(),
            done: false,
            game_overs: 0,
            secs: 0.0,
            deaths: 0,
            lives: 0,
            taken: 0.0,
            kills: 0,
            foes: 0,
            left: 0,
            met: 0,
            made: 0,
            rivals_slain: 0,
            spent: [0.0; 4],
            peaks: 0,
            rests: (0.0, 0),
            gaps: (0.0, 0),
            routs: 0,
            broken: 0,
            general_slain: false,
            overlaps: 0,
            sudden: 0,
            silver_in: 0,
            silver_lost: 0,
            runes_in: 0,
            spent_silver: 0,
            spent_runes: 0,
            bought: Vec::new(),
            worn: 0.0,
            dullest: 1.0,
            carried: 0,
            pit: Vec::new(),
        }
    }

    /// The share of fighting time (build, peak, release) at peak.
    pub fn peak_share(&self) -> f64 {
        let f = self.spent[1] + self.spent[2] + self.spent[3];
        if f > 0.0 {
            self.spent[2] / f
        } else {
            0.0
        }
    }

    /// The mean rest between fights (seconds).
    pub fn rest(&self) -> f64 {
        self.rests.0 / self.rests.1.max(1) as f64
    }

    /// The mean breath between one peak's end and the next one's start (seconds; none: 0).
    pub fn gap(&self) -> f64 {
        self.gaps.0 / self.gaps.1.max(1) as f64
    }
}

/// A counter the game resets now and then (a fall, a new level): what it counted before each reset
/// is banked, so the total runs on.
#[derive(Clone, Copy, Debug, Default)]
struct Banked {
    bank: f64,
    last: f64,
    start: Option<f64>,
}

impl Banked {
    fn see(&mut self, v: f64) {
        if self.start.is_none() {
            self.start = Some(v);
        } else if v < self.last - 1e-9 {
            self.bank += self.last;
        }
        self.last = v;
    }

    fn total(&self) -> f64 {
        self.bank + self.last - self.start.unwrap_or(0.0)
    }
}

/// What a part's loop watches, step by step.
#[derive(Default)]
struct Watch {
    kills: Banked,
    taken: Banked,
    overlaps: Banked,
    routs: Banked,
    made: Banked,
    /// The rivals whose cards were shown (each counted once, however often he is met again).
    met: std::collections::HashSet<crate::logic::war::ledger::RivalId>,
    spent: [Banked; 4],
    peaks: Banked,
    rest_sum: Banked,
    rest_n: Banked,
    gap_sum: Banked,
    gap_n: Banked,
    silver: Option<u32>,
    runes: Option<u32>,
    dying: bool,
    edge: Option<f64>,
    /// Foes first seen mid-fight: (id, the frame seen); and those already judged.
    fresh: Vec<(u32, u32)>,
    seen: std::collections::HashSet<u32>,
    /// The fight's marks (his blows landed, theirs, kills), and the seconds since they last moved.
    mark: (u32, u32, u32),
    idle: f64,
    since_kill: f64,
    stood: V,
    still: f64,
}

impl Watch {
    fn look(&mut self, g: &Game, part: &mut Part, step: f64) {
        if let Some(b) = g.brawl.as_ref() {
            self.kills.see(b.stats.kills as f64);
            self.taken.see(b.stats.taken);
            self.overlaps.see(b.stats.overlaps as f64);
            // Foes come in mid-fight: from the frame each is first seen to its first tell.
            let now = b.threat.frame;
            let first = self.seen.is_empty();
            // (A fallen foe's id may be handed to a new one: the gone are forgotten.)
            let living: std::collections::HashSet<u32> =
                g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.alive()).map(|d| d.id).collect();
            if !first {
                self.seen.retain(|id| living.contains(id));
                self.fresh.retain(|f| living.contains(&f.0));
            }
            for d in g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.alive()) {
                if self.seen.insert(d.id) && !first && g.screen == Screen::Playing && d.lore.master.is_none() {
                    // (One out of a door of the Pit was told by its door, grinding open a second
                    // before: `pit::DOOR_TELL`.)
                    let door = g.pit.is_some()
                        && crate::logic::pit::doors()
                            .iter()
                            .any(|dr| dr.mouth().iter().any(|m| (m.x as f64 - d.pos.x).hypot(m.y as f64 - d.pos.y) < 0.6));
                    let told = if door { (crate::logic::pit::DOOR_TELL * 60.0) as u32 } else { 0 };
                    self.fresh.push((d.id, now.saturating_sub(told)));
                }
                if let (Some(k), State::Attack { mv, frame: 0..=1 }) = (self.fresh.iter().position(|f| f.0 == d.id), d.f.state) {
                    // (From the frame it came in to the frame its first blow could land.)
                    if now.saturating_sub(self.fresh[k].1) + u32::from(d.data(mv).startup) < TELL_FRAMES {
                        part.sudden += 1;
                        if std::env::var_os("VH_RUN_LEFT").is_some() {
                            eprintln!(
                                "sudden: {:?} {:?} seen {} f before its {:?} (startup {}) at ({:.1}, {:.1}), hero at ({:.1}, {:.1}), orders bitten {:?}",
                                part.kind,
                                d.breed.kind,
                                now.saturating_sub(self.fresh[k].1),
                                mv,
                                d.data(mv).startup,
                                d.pos.x,
                                d.pos.y,
                                g.hero.pos().x,
                                g.hero.pos().y,
                                g.general.as_ref().map(|r| r.tally.bitten)
                            );
                        }
                    }
                    self.fresh.remove(k);
                }
            }
        }
        let w = &g.war;
        self.routs.see(w.tally.routs as f64);
        self.made.see(w.grudges.promoted as f64);
        self.met.extend(w.grudges.met.iter().copied());
        for k in 0..4 {
            self.spent[k].see(w.drum.spent[k]);
        }
        self.peaks.see(w.drum.peaks as f64);
        self.rest_sum.see(w.drum.rests.iter().sum());
        self.rest_n.see(w.drum.rests.len() as f64);
        self.gap_sum.see(w.drum.gaps.iter().sum());
        self.gap_n.see(w.drum.gaps.len() as f64);
        let (s, r) = (g.progress.silver, g.progress.runes);
        if let Some(was) = self.silver {
            if s > was {
                part.silver_in += s - was;
            } else {
                part.silver_lost += was - s;
            }
        }
        if let Some(was) = self.runes {
            part.runes_in += r.saturating_sub(was);
        }
        (self.silver, self.runes) = (Some(s), Some(r));
        if let Some(e) = self.edge {
            part.worn += (e - g.hero.edge).max(0.0);
        }
        self.edge = Some(g.hero.edge);
        part.dullest = part.dullest.min(g.hero.edge);
        let dying = g.screen == Screen::Dying;
        if dying && !self.dying {
            part.deaths += 1;
        }
        self.dying = dying;
        if let Some(pit) = g.pit.as_ref().filter(|p| p.tallies.len() >= part.pit.len()) {
            part.pit.clone_from(&pit.tallies);
        }
        if let Some(r) = g.general.as_ref() {
            part.broken = part.broken.max(r.command.broken());
            part.general_slain |= r.command.slain;
        }
        // (The guide's clock: seconds since a blow last landed either way.)
        let mark = g.brawl.as_ref().map(|b| (b.stats.hits, b.stats.landings, b.stats.kills)).unwrap_or_default();
        if mark != self.mark {
            self.idle = 0.0;
        } else {
            self.idle += step;
        }
        self.since_kill = if mark.2 != self.mark.2 { 0.0 } else { self.since_kill + step };
        self.mark = mark;
        // (...and since it last moved, with nothing hunting it: it stands where it cannot go on.)
        let hp = g.hero.pos();
        let hunted = g.enemies.iter().filter_map(|e| e.duel.as_deref()).any(|d| d.alive() && d.engaged());
        if hunted || (hp.x - self.stood.x).hypot(hp.y - self.stood.y) > 1.0 || self.stood.x.is_nan() {
            (self.stood, self.still) = (hp, 0.0);
        } else {
            self.still += step;
        }
    }

    fn close(&self, part: &mut Part) {
        part.kills += self.kills.total().round() as u32;
        part.taken += self.taken.total();
        part.overlaps += self.overlaps.total().round() as u32;
        part.routs += self.routs.total().round() as u32;
        part.made += self.made.total().round() as u32;
        part.met += self.met.len() as u32;
        for k in 0..4 {
            part.spent[k] += self.spent[k].total();
        }
        part.peaks += self.peaks.total().round() as u32;
        part.rests.0 += self.rest_sum.total();
        part.rests.1 += self.rest_n.total().round() as u32;
        part.gaps.0 += self.gap_sum.total();
        part.gaps.1 += self.gap_n.total().round() as u32;
    }
}

/// How a try ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum End {
    Done,
    GameOver,
    Time,
}

/// How the whole run went.
#[derive(Clone, Debug)]
pub struct WholeRun {
    pub skill: Skill,
    pub seed: u32,
    pub order: usize,
    /// What the Garden left in the purse (silver, runes).
    pub garden: (u32, u32),
    pub parts: Vec<Part>,
    /// Scene II began: the run is finished.
    pub ending: bool,
    /// The part it ended at, lost `TRIES` times.
    pub lost_at: Option<String>,
    /// The purse at the end (silver, runes).
    pub purse: (u32, u32),
    /// The Ledger at the end: rivals made in all, living, slain, the highest rank, lessons carried.
    pub ledger: (u32, u32, u32, u8, u32),
    /// How the rivals' remembered fights ended: the hero won, the rival won, he fled, the hero left.
    pub outcomes: [u32; 4],
    /// New Game+: the times over (0 a first run).
    pub ng: u8,
}

impl WholeRun {
    /// Lives lost over the whole run, and the tries lost.
    pub fn deaths(&self) -> u32 {
        self.parts.iter().map(|p| p.deaths).sum()
    }
    pub fn game_overs(&self) -> u32 {
        self.parts.iter().map(|p| p.game_overs).sum()
    }
    /// Realms whose every foe fell, and princes slain.
    pub fn realms_done(&self) -> usize {
        self.parts.iter().filter(|p| matches!(p.kind, PartKind::Realm(_)) && p.done).count()
    }
    pub fn princes_slain(&self) -> usize {
        self.parts.iter().filter(|p| matches!(p.kind, PartKind::Court(_)) && p.done).count()
    }
    /// Seconds in all.
    pub fn secs(&self) -> f64 {
        self.parts.iter().map(|p| p.secs).sum()
    }
    /// The furthest the run got: the last part played, done or not.
    pub fn reached(&self) -> String {
        self.parts.last().map(|p| p.kind.name()).unwrap_or_default()
    }
}

/// What a player brings out of the Garden and its Cyclops: every golden key's silver, the Cyclops'
/// bounty, his runes and his relic.
fn garden_purse(g: &mut Game) -> (u32, u32) {
    // (The Garden counted in a game of its own: the run's begins at the old man's.)
    let mut garden = fresh_game(0);
    garden.map_jump(Stage::Dungeon(0));
    let keys = garden.keys_total;
    g.progress.silver += keys * gear::COINS_KEY + gear::COINS_BOSS;
    g.progress.runes += gear::RUNES_BOSS;
    let hero = g.hero_kind;
    if let Some(tree) = g.progress.relic_tree(hero) {
        g.progress.find_relic(hero, tree);
    }
    (g.progress.silver, g.progress.runes)
}

/**
 * The old man's, between parts, by one plain rule a careful player might keep: the blade
 * sharpened below 70% keen, the shield reforged or mended; then the lowest of the three trees
 * (blade, shield, path; in that order on a tie) raised a tier while the purse holds its price and
 * `RESERVE` more (sealed tiers want their runes too, or a relic); then the belt filled with health
 * bottles. The elixir tree, the patrons and the outfits are left alone. A rank whose deeds are met
 * is tried at the yard (the trial's bout of the ring; a win passes).
 */
fn shop(g: &mut Game, skill: Skill, seed: u32, part: &mut Part) {
    let hero = g.hero_kind;
    let (silver0, runes0) = (g.progress.silver, g.progress.runes);
    let h = *g.progress.hero(hero);
    let edge = h.sharpness(h.wield);
    let p = gear::sharpen_price(h.wield, edge);
    if edge < SHARPEN_BELOW && g.progress.silver >= p {
        g.progress.silver -= p;
        g.progress.hero_mut(hero).edge[(h.wield - 1) as usize] = 100;
        part.bought.push("sharpen".into());
    }
    if h.is_broken(h.carry) {
        let p = gear::reforge_price(h.carry);
        if g.progress.silver >= p {
            g.progress.silver -= p;
            g.progress.hero_mut(hero).broken &= !(1 << h.carry);
            g.hero.shield = Some(ShieldGear::new(h.carry));
            part.bought.push("reforge".into());
        }
    } else if let Some(s) = g.hero.shield.as_ref().filter(|s| s.fraction() < 1.0) {
        let p = s.repair_price();
        if g.progress.silver >= p {
            g.progress.silver -= p;
            g.hero.shield = Some(ShieldGear::new(h.carry));
            part.bought.push("mend".into());
        }
    }
    loop {
        let h = *g.progress.hero(hero);
        let Some(tree) = [Tree::Armory, Tree::Shield, Tree::Path].into_iter().filter(|&t| h.next(t).is_some()).min_by_key(|&t| h.owned(t))
        else {
            break;
        };
        let tier = h.owned(tree) + 1;
        match g.progress.can_unlock(hero, tree, tier) {
            Ok(p) if p.silver == 0 || g.progress.silver >= p.silver + RESERVE => {
                let _ = g.progress.unlock(hero, tree, tier);
                part.bought.push(format!("{}{}", ["blade", "path", "shield", "elixir"][tree.index()], g.progress.hero(hero).owned(tree)));
            }
            _ => break,
        }
    }
    let carry = g.progress.hero(hero).carry;
    if g.hero.shield.as_ref().is_none_or(|s| s.tier != carry) && !g.progress.hero(hero).is_broken(carry) {
        g.hero.shield = Some(ShieldGear::new(carry));
    }
    let bottle = progression::bottle_price(PotionKind::Health);
    let mut bottles = 0;
    while g.belt.len() < BELT_SLOTS && g.progress.silver >= bottle {
        g.progress.silver -= bottle;
        g.belt.push(PotionKind::Health);
        bottles += 1;
    }
    if bottles > 0 {
        part.bought.push(format!("{bottles} bottles"));
    }
    // The yard: a rank's trial, once its deeds are met.
    let m = g.progress.hero(hero).mastery;
    if let Some(rank) = m.ready() {
        let t = crate::logic::war::mastery::trial(hero, rank);
        let h = *g.progress.hero(hero);
        let kit = Loadout { hero, blade: h.owned(Tree::Armory), shield: h.owned(Tree::Shield), path: h.owned(Tree::Path) };
        let r = play_bout_mastered(t.bout, skill, seed, Some(kit), m.rank);
        let passed = r.won && r.secs <= t.limit;
        if g.progress.hero_mut(hero).mastery.tried(rank, t.count, passed) {
            part.bought.push(format!("rank {rank}"));
        } else {
            part.bought.push(format!("trial {rank} failed"));
        }
    }
    g.equip_from_progress();
    part.spent_silver += silver0.saturating_sub(g.progress.silver);
    part.spent_runes += runes0.saturating_sub(g.progress.runes);
}

/// Into play: the camera the harnesses play from, the kit on, the bot free to walk.
fn into_play(g: &mut Game, cam: Option<CameraMode>) {
    if let Some(cam) = cam {
        g.view.camera_mode = cam;
    }
    g.enter(Screen::Playing);
    let kit = g.hero_kit();
    if let Some(b) = g.brawl.as_mut() {
        b.reset(kit);
    }
    g.hero.free_move = true;
}

/**
 * Carries the bot to the doorstep of the nearest den still standing: a floor tile three to six
 * tiles from its home that its home can walk to (the realm's own ground, its rules running).
 */
fn guide(g: &mut Game) -> bool {
    let hp = g.hero.pos();
    let homes: Vec<V> = g
        .enemies
        .iter()
        .filter_map(|e| e.duel.as_deref())
        .filter(|d| d.alive() && !d.puppet)
        .map(|d| d.den.as_ref().map(|n| n.home).unwrap_or(d.pos))
        .collect();
    let Some(home) = homes.into_iter().min_by(|a, b| (a.x - hp.x).hypot(a.y - hp.y).total_cmp(&(b.x - hp.x).hypot(b.y - hp.y))) else {
        return false;
    };
    let from = P::new(home.x.round() as i32, home.y.round() as i32);
    // (Floor it can walk on from the den itself where the den stands on floor; else any floor about it.)
    let near = if Bot::floor(g, from) { crate::logic::pathing::reachable(&g.level.grid, from, false, false) } else { Default::default() };
    let mut spots: Vec<(i64, P)> = (-7..=7)
        .flat_map(|dy| (-7..=7).map(move |dx| P::new(from.x + dx, from.y + dy)))
        .filter(|p| Bot::floor(g, *p) && (near.is_empty() || near.contains(p)))
        .map(|p| {
            let d = ((p.x - from.x) as f64).hypot((p.y - from.y) as f64);
            (((d - 4.5).abs() * 100.0) as i64, p)
        })
        .collect();
    spots.sort_by_key(|(s, p)| (*s, p.y, p.x));
    let Some(&(_, at)) = spots.first() else { return false };
    g.hero.place(at);
    true
}

/// One try: the bot plays until `done`, the lives run out, or `limit` seconds pass.
fn play(g: &mut Game, bot: &mut Bot, limit: f64, guided: bool, done: &dyn Fn(&Game) -> bool, w: &mut Watch, part: &mut Part) -> End {
    let step = crate::app::STEP;
    let mut t = 0.0;
    while t < limit {
        let playing = g.screen == Screen::Playing;
        let pad = if playing { bot.act(g) } else { PadState::default() };
        g.input.set_pad(Some(pad));
        g.update(step);
        t += step;
        part.secs += step;
        w.look(g, part, step);
        if let Some(every) = std::env::var("VH_RUN_TRACE").ok().and_then(|v| v.parse::<f64>().ok()).filter(|e| (t / e).fract() < step / e) {
            // (`VH_RUN_TRACE=SECONDS`: where he is and what stands, every so often.)
            let hp = g.hero.pos();
            let foes: Vec<_> =
                g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.alive()).map(|d| (d.breed.kind, d.hp as i32)).collect();
            eprintln!(
                "{} t {t:.0}/{every} {:?} hero ({:.1}, {:.1}) hp {} lives {} edge {:.2} idle {:.0} foes {foes:?}",
                part.kind.name(),
                g.screen,
                hp.x,
                hp.y,
                g.hero.hp,
                g.score.lives,
                g.hero.edge,
                w.idle
            );
        }
        match g.screen {
            Screen::GameOver => return End::GameOver,
            Screen::Ready | Screen::View => g.enter(Screen::Playing),
            Screen::Toll => return End::Done,
            _ => {}
        }
        if done(g) {
            return End::Done;
        }
        if guided && (w.idle >= GUIDE_AFTER || w.still >= STUCK_AFTER || w.since_kill >= 2.0 * GUIDE_AFTER) && g.screen == Screen::Playing {
            if guide(g) {
                part.carried += 1;
            }
            (w.idle, w.still, w.since_kill) = (0.0, 0.0, 0.0);
        }
    }
    End::Time
}

/// The realms whose ground the bots walk as a player does: the vaults and the Rotting Feast. The
/// other five lie to them (see the module's note).
pub fn bots_walk(sin: Sin) -> bool {
    matches!(sin, Sin::Greed | Sin::Gluttony)
}

/// Every duelist of the level down (a realm cleared): every one he could reach, that is (a ghost
/// left hovering over a chasm counts as gone).
fn cleared(g: &Game) -> bool {
    !g.enemies.iter().any(|e| {
        e.duel.as_deref().is_some_and(|d| d.alive() && !d.puppet && Bot::floor(g, P::new(d.pos.x.round() as i32, d.pos.y.round() as i32)))
    })
}

/// A part: entered at `stage`, played (and played again by Continue after a loss) until done.
#[allow(clippy::too_many_arguments)]
fn part(
    g: &mut Game,
    bot: &mut Bot,
    kind: PartKind,
    stage: Option<Stage>,
    cam: Option<CameraMode>,
    limit: f64,
    done: &dyn Fn(&Game) -> bool,
) -> Part {
    if let Some(s) = stage {
        // (The bottles bought at the old man's go with him.)
        let belt = g.belt.clone();
        g.map_jump(s);
        g.belt = belt;
    }
    let mut p = Part::new(kind, g);
    into_play(g, cam);
    p.foes = g.enemies.iter().filter(|e| e.duel.is_some()).count() as u32;
    let slain0 = g.warband.save.ledger.slain;
    let guided = matches!(kind, PartKind::Realm(_)) && std::env::var_os("VH_RUN_NOGUIDE").is_none();
    loop {
        let mut w = Watch { stood: V::new(f64::NAN, f64::NAN), ..Watch::default() };
        let end = play(g, bot, limit, guided, done, &mut w, &mut p);
        w.close(&mut p);
        match end {
            End::Done => {
                p.done = true;
                break;
            }
            End::GameOver => {
                p.game_overs += 1;
                if p.game_overs >= TRIES {
                    break;
                }
                // Continue: back to the checkpoint with what the save holds.
                g.continue_game();
                into_play(g, cam);
            }
            // (A realm not cleared in its time: the run goes on to the court. A fight not won in
            // its time is a try lost.)
            End::Time if guided => break,
            End::Time => {
                p.game_overs += 1;
                if p.game_overs >= TRIES {
                    break;
                }
                g.continue_game();
                into_play(g, cam);
            }
        }
    }
    p.left = g.enemies.iter().filter(|e| e.duel.as_deref().is_some_and(|d| d.alive())).count() as u32;
    if std::env::var_os("VH_RUN_LEFT").is_some() && !p.done {
        let hp = g.hero.pos();
        eprintln!("{} not done: hero at ({:.1}, {:.1})", kind.name(), hp.x, hp.y);
        for d in g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.alive()) {
            eprintln!(
                "  left: {:?} at ({:.1}, {:.1}) hp {:.0}/{:.0} puppet {} engaged {} state {:?}",
                d.breed.kind,
                d.pos.x,
                d.pos.y,
                d.hp,
                d.max_hp,
                d.puppet,
                d.engaged(),
                d.f.state
            );
        }
    }
    p.lives = g.score.lives;
    p.rivals_slain = g.warband.save.ledger.slain.saturating_sub(slain0);
    p
}

/**
 * The whole run by `skill`, seeded with `seed`, its realms in `ORDERS[order]`: from the old man's
 * after the Garden, realm and court seven times with the old man's between, then the House door,
 * the Pit, Krampus and the ending.
 */
pub fn whole_run(skill: Skill, seed: u32, order: usize) -> WholeRun {
    // (`VH_RUN_ORDER=wrath,lust,...`: the realms in another order, for a closer look at one.)
    let custom: Vec<Sin> = std::env::var("VH_RUN_ORDER").map(|v| v.split(',').filter_map(Sin::from_name).collect()).unwrap_or_default();
    let sins = if custom.is_empty() { ORDERS[order.min(1)].to_vec() } else { custom };
    let mut run = run_of(skill, seed, &sins, true);
    run.order = order;
    run
}

/**
 * New Game+ `ng` times over (`newgame`): the whole run played as `whole_run` plays it, then again
 * from the start with what it left (the kit, the purse, the living rivals) a tier harder, `ng`
 * times; the last is returned. A run that does not reach the end hands on what it had where it
 * stopped, as if it had (the adept's runs end at a prince). `ng` 0 is `whole_run` itself.
 */
pub fn whole_run_ng(skill: Skill, seed: u32, order: usize, ng: u8) -> WholeRun {
    if ng == 0 {
        return whole_run(skill, seed, order);
    }
    let sins = ORDERS[order.min(1)];
    let mut g = fresh_game(seed);
    let mut run = play_run(&mut g, skill, seed, &sins, true);
    for _ in 0..ng.min(super::newgame::NG_MAX) {
        let offer = g.ng_offer.take().unwrap_or_else(|| g.new_game_plus_offer());
        // (Back to the title, as the end or Quit to Title leaves him, and the run closed.)
        if g.story.is_some() {
            g.finish_ending();
        }
        g.story = None;
        g.enter(Screen::Title);
        g.close_run();
        // (Closing the run wipes the voices: the harness keeps the prince's cheats off, `fresh_game`.)
        g.voices.deceit.off = true;
        g.ng_offer = Some(offer);
        g.take_new_game_plus();
        run = play_run(&mut g, skill, seed, &sins, true);
    }
    run.order = order;
    finish_report(&mut run, &g);
    run
}

/// The run through the realms of `sins` in that order, and on through the finale if `finale`.
pub fn run_of(skill: Skill, seed: u32, sins: &[Sin], finale: bool) -> WholeRun {
    let mut g = fresh_game(seed);
    let mut run = play_run(&mut g, skill, seed, sins, finale);
    finish_report(&mut run, &g);
    run
}

/// The purse and the Ledger at the run's end, into its report.
fn finish_report(run: &mut WholeRun, g: &Game) {
    run.purse = (g.progress.silver, g.progress.runes);
    let l = &g.warband.save.ledger;
    run.ledger = (
        run.parts.iter().map(|p| p.made).sum(),
        l.live().count() as u32,
        l.slain,
        l.rivals.iter().map(|r| r.rank).max().unwrap_or(0),
        l.rivals.iter().filter(|r| r.lesson.is_some()).count() as u32,
    );
    use crate::logic::war::ledger::Outcome;
    for e in l.rivals.iter().flat_map(|r| r.memory.iter()) {
        run.outcomes[match e.outcome {
            Outcome::HeroWon => 0,
            Outcome::RivalWon => 1,
            Outcome::RivalFled => 2,
            Outcome::HeroLeft => 3,
        }] += 1;
    }
}

fn play_run(g: &mut Game, skill: Skill, seed: u32, sins: &[Sin], finale: bool) -> WholeRun {
    g.hero_kind = HeroKind::Ragnar;
    let garden = garden_purse(g);
    let mut bot = Bot::new(skill, seed);
    let mut run = WholeRun {
        skill,
        seed,
        order: 0,
        garden,
        parts: Vec::new(),
        ending: false,
        lost_at: None,
        purse: (0, 0),
        ledger: (0, 0, 0, 0, 0),
        outcomes: [0; 4],
        ng: g.ng_plus,
    };
    let mut before = Part::new(PartKind::Pit, g);
    shop(g, skill, seed, &mut before);
    let mut first_shop = Some(before);
    for &sin in sins {
        let limit = if bots_walk(sin) { REALM_LIMIT } else { STAND_IN_LIMIT };
        let mut realm = part(g, &mut bot, PartKind::Realm(sin), Some(Stage::Realm(sin)), None, limit, &cleared);
        if let Some(b) = first_shop.take() {
            // (What was bought with the Garden's purse goes on the first realm's line.)
            realm.spent_silver += b.spent_silver;
            realm.spent_runes += b.spent_runes;
            realm.bought.splice(0..0, b.bought);
        }
        // (A realm the bots cannot read, lost to its ground, is passed as a player who learnt it
        // would pass it: its fights are counted, and the run goes on to the court.)
        let lost = realm.game_overs >= TRIES && bots_walk(sin);
        if !lost {
            // (He calls the old man before the court, as H does anywhere: the same rule.)
            shop(g, skill, seed, &mut realm);
        }
        run.parts.push(realm);
        if lost {
            run.lost_at = Some(PartKind::Realm(sin).name());
            return run;
        }
        let slain = |g: &Game| g.court.as_ref().is_some_and(|c| c.slain);
        let mut court = part(g, &mut bot, PartKind::Court(sin), Some(Stage::Court(sin)), Some(CameraMode::Top), COURT_LIMIT, &slain);
        if !court.done {
            run.parts.push(court);
            run.lost_at = Some(PartKind::Court(sin).name());
            return run;
        }
        shop(g, skill, seed, &mut court);
        run.parts.push(court);
    }
    if !finale {
        return run;
    }
    // The House door: the Pit, its seven levels, and at the bottom Krampus's lair.
    let mut pit = part(g, &mut bot, PartKind::Pit, Some(Stage::Pit(1)), Some(CameraMode::Top), PIT_LIMIT, &|g: &Game| g.in_lair());
    let reached = pit.done;
    if reached {
        // (He calls the old man at the bottom, before Krampus: the same rule.)
        shop(g, skill, seed, &mut pit);
    }
    run.parts.push(pit);
    if !reached {
        run.lost_at = Some(PartKind::Pit.name());
        return run;
    }
    let slain = |g: &Game| g.lair.as_ref().is_some_and(|l| l.slain);
    let lair = part(g, &mut bot, PartKind::Lair, None, Some(CameraMode::Top), LAIR_LIMIT, &slain);
    let won = lair.done;
    run.parts.push(lair);
    if !won {
        run.lost_at = Some(PartKind::Lair.name());
        return run;
    }
    // The boy runs to him, and Scene II begins.
    let step = crate::app::STEP;
    let mut t = 0.0;
    while t < ENDING_LIMIT && !run.ending {
        g.input.set_pad(Some(PadState::default()));
        g.update(step);
        t += step;
        run.ending = g.story.as_ref().is_some_and(|s| s.scene == crate::logic::story::Scene::Two);
    }
    run
}

/// The run as a table, part by part.
pub fn run_table(r: &WholeRun) -> String {
    // (The Drum paces the realms and the Pit; a court keeps its own pace.)
    let drum = |fights: bool, v: f64, d: usize| if fights { "-".to_string() } else { format!("{v:.d$}") };
    let mut out = format!(
        "THE WHOLE RUN {}{} seed {} order {} (Garden's purse {} silver, {} runes)\n",
        r.skill.name(),
        if r.ng > 0 { format!(" NEW GAME+{}", r.ng) } else { String::new() },
        r.seed,
        r.order,
        r.garden.0,
        r.garden.1
    );
    out += "part              tier kit    rk done  go  min  died lives taken kills/foes met made slain peak  rest rout inst  ovl sudden carry  worn dull silver+ lost runes+  spent  bought\n";
    for p in &r.parts {
        let fights = matches!(p.kind, PartKind::Court(_) | PartKind::Lair);
        let _ = writeln!(
            out,
            "{:<17} {:>4} {:>2}/{:>2}/{:>2} {:>2} {:>4} {:>3} {:>4.1} {:>4} {:>5} {:>5.0} {:>5}/{:<4} {:>3} {:>4} {:>5} {:>4} {:>5} {:>4} {:>2}{:<2} {:>4} {:>6} {:>5} {:>5.2} {:>4.0}% {:>7} {:>4} {:>6} {:>4}/{:<2} {}",
            p.kind.name(),
            p.tier,
            p.kit.0,
            p.kit.1,
            p.kit.2,
            p.rank,
            if p.done { "yes" } else { "no" },
            p.game_overs,
            p.secs / 60.0,
            p.deaths,
            p.lives,
            p.taken,
            p.kills,
            p.foes,
            p.met,
            p.made,
            p.rivals_slain,
            drum(fights, p.peak_share() * 100.0, 0),
            drum(fights, p.rest(), 1),
            p.routs,
            p.broken,
            if p.general_slain { "+" } else { "" },
            p.overlaps,
            p.sudden,
            p.carried,
            p.worn,
            p.dullest * 100.0,
            p.silver_in,
            p.silver_lost,
            p.runes_in,
            p.spent_silver,
            p.spent_runes,
            p.bought.join(" ")
        );
        for l in &p.pit {
            let _ = writeln!(
                out,
                "    level {} {:>4.0} s, {:>3} kills, rode in {}, out {}, left {}, took {:.0}, falls {}{}",
                l.level,
                l.secs,
                l.kills,
                l.riders_in,
                l.riders_out,
                l.left,
                l.taken,
                l.falls,
                if l.leapt { ", leapt" } else { "" }
            );
        }
    }
    let end =
        if r.ending { "THE ENDING REACHED".to_string() } else { format!("ended at {}", r.lost_at.clone().unwrap_or_else(|| r.reached())) };
    let _ = writeln!(
        out,
        "{end}: {:.0} min, {} lives lost, {} tries lost, {} realms cleared, {} princes slain",
        r.secs() / 60.0,
        r.deaths(),
        r.game_overs(),
        r.realms_done(),
        r.princes_slain()
    );
    out
}

/**
 * A prince fought as the run meets him: `keys` other sins' keys held (so the court is weighed at
 * tier `keys + 1`), Ragnar's kit as `kit` gives it, `limit` seconds: whether he fell, the seconds,
 * the phase reached, his health left (a share), lives lost.
 */
pub fn court_at(sin: Sin, skill: Skill, seed: u32, kit: Loadout, keys: usize, limit: f64) -> (bool, f64, u8, f64, u32) {
    let mut g = fresh_game(seed);
    g.sins_cleared = crate::logic::sins::SINS.iter().filter(|&&s| s != sin).take(keys).fold(0, |m, s| m | s.bit());
    // (`VH_GENERAL=slain` or `decree`: the prince's terms as his general left them.)
    if let Ok(v) = std::env::var("VH_GENERAL") {
        let mut r = g.general_records();
        r.set(sin, crate::logic::war::general::Record { met: true, broken: 0, decree_done: v == "decree", slain: v == "slain" });
        g.warband.save.generals = r.encode();
    }
    g.map_jump(Stage::Court(sin));
    kit.equip(&mut g);
    into_play(&mut g, Some(CameraMode::Top));
    let mut bot = Bot::new(skill, seed);
    let lives = g.score.lives;
    let (step, mut t, mut phase) = (crate::app::STEP, 0.0, 0);
    let mut slain = false;
    while t < limit {
        let pad = if g.screen == Screen::Playing { bot.act(&g) } else { PadState::default() };
        g.input.set_pad(Some(pad));
        g.update(step);
        t += step;
        match g.screen {
            Screen::GameOver => break,
            Screen::Ready | Screen::View => g.enter(Screen::Playing),
            _ => {}
        }
        phase = phase.max(g.prince().map(|d| d.breed.phase).unwrap_or(0));
        if g.court.as_ref().is_some_and(|c| c.slain) {
            slain = true;
            break;
        }
    }
    let left = g.prince().map(|d| d.hp / d.max_hp).unwrap_or(0.0);
    (slain, t, phase, left, lives.saturating_sub(g.score.lives))
}

/// Several runs side by side: each run's end, then part by part (in the order played) the mean of
/// what every run that reached it did there.
pub fn runs_summary(runs: &[WholeRun]) -> String {
    let mut out = String::new();
    for r in runs {
        let _ = writeln!(
            out,
            "{} seed {} order {}: {} at {:.0} min, {} lives lost, {} tries lost, {} realms cleared, {} princes slain, purse {} silver {} runes, rivals {} made {} living {} slain, rank {} at most, {} lessons, fights remembered {:?} (hero won, rival won, fled, hero left)",
            r.skill.name(),
            r.seed,
            r.order,
            if r.ending { "the ending reached".to_string() } else { format!("ended at {}", r.lost_at.clone().unwrap_or_else(|| r.reached())) },
            r.secs() / 60.0,
            r.deaths(),
            r.game_overs(),
            r.realms_done(),
            r.princes_slain(),
            r.purse.0,
            r.purse.1,
            r.ledger.0,
            r.ledger.1,
            r.ledger.2,
            r.ledger.3,
            r.ledger.4,
            r.outcomes
        );
    }
    out += "part              runs done tier  kit        rank deeds  min  died  lost taken kills  met made slain peak  rest  gap rout inst ovl sudden carry silver+ spent\n";
    let longest = runs.iter().map(|r| r.parts.len()).max().unwrap_or(0);
    for k in 0..longest {
        let here: Vec<&Part> = runs.iter().filter_map(|r| r.parts.get(k)).collect();
        let n = here.len() as f64;
        let avg = |f: &dyn Fn(&Part) -> f64| here.iter().map(|p| f(p)).sum::<f64>() / n;
        let fights = matches!(here[0].kind, PartKind::Court(_) | PartKind::Lair);
        let kinds: Vec<String> = here.iter().map(|p| p.kind.name()).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
        let _ = writeln!(
            out,
            "{:<17} {:>4} {:>3.0}% {:>4.1} {:>4.1}/{:>3.1}/{:>3.1} {:>4.1} {:>5.0} {:>4.1} {:>5.1} {:>4.1} {:>5.0} {:>5.0} {:>4.1} {:>4.1} {:>5.1} {:>4} {:>5} {:>4} {:>4.1} {:>4.1} {:>3.1} {:>6.1} {:>5.1} {:>7.0} {:>5.0}",
            if kinds.len() == 1 { kinds[0].clone() } else { format!("part {}", k + 1) },
            here.len(),
            avg(&|p| f64::from(u8::from(p.done))) * 100.0,
            avg(&|p| p.tier as f64),
            avg(&|p| p.kit.0 as f64),
            avg(&|p| p.kit.1 as f64),
            avg(&|p| p.kit.2 as f64),
            avg(&|p| p.rank as f64),
            avg(&|p| p.deeds as f64),
            avg(&|p| p.secs / 60.0),
            avg(&|p| p.deaths as f64),
            avg(&|p| p.game_overs as f64),
            avg(&|p| p.taken),
            avg(&|p| p.kills as f64),
            avg(&|p| p.met as f64),
            avg(&|p| p.made as f64),
            avg(&|p| p.rivals_slain as f64),
            if fights { "-".to_string() } else { format!("{:.0}%", avg(&|p| p.peak_share()) * 100.0) },
            if fights { "-".to_string() } else { format!("{:.1}", avg(&|p| p.rest())) },
            if fights { "-".to_string() } else { format!("{:.1}", avg(&|p| p.gap())) },
            avg(&|p| p.routs as f64),
            avg(&|p| p.broken as f64),
            avg(&|p| p.overlaps as f64),
            avg(&|p| p.sudden as f64),
            avg(&|p| p.carried as f64),
            avg(&|p| p.silver_in as f64),
            avg(&|p| p.spent_silver as f64)
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /**
     * The whole run's first parts (WARBAND.md W9), as `--runs` plays them: the vaults and Mammon's
     * court with one save, by every skill. The curve at its start: the expert clears the vaults and
     * fells Mammon without a game over, his blade still sharper than dull when he meets the prince;
     * the adept clears the vaults and fells Mammon too (with the kit his purse buys); the masher
     * loses every life in the vaults, three times over, and the run ends there.
     */
    #[test]
    fn the_whole_run_begins_on_its_curve() {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test-run"));
        let novice = run_of(Skill::Novice, 1, &[Sin::Greed], false);
        assert_eq!(novice.lost_at.as_deref(), Some("realm greed"), "the masher falls in the first realm");
        assert_eq!(novice.parts[0].game_overs, TRIES);
        for skill in [Skill::Adept, Skill::Expert] {
            let r = run_of(skill, 1, &[Sin::Greed], false);
            let (realm, court) = (&r.parts[0], &r.parts[1]);
            assert!(realm.done && realm.game_overs == 0, "{skill:?}: {realm:?}");
            assert!(court.done && court.game_overs == 0, "{skill:?}: {court:?}");
            assert!(court.kit.0 >= 2 && court.kit.1 >= 2, "{skill:?}: the purse buys the second tiers: {court:?}");
            // (The adept, a hero on the move, may meet one bolt paced late in a crawl: W4's log.)
            assert!(realm.overlaps + court.overlaps <= u32::from(skill == Skill::Adept), "{skill:?}: {realm:?}");
            assert_eq!(realm.sudden, 0, "{skill:?}: nothing comes in without a second's tell");
            if skill == Skill::Expert {
                assert!(realm.dullest > crate::logic::gear::DULL_EDGE, "a realm leaves his blade short of dull: {realm:?}");
            }
        }
    }
}
