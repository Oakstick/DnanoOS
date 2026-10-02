//! A scripted player for the Sparring Ring, and the balance harness built on it (COMBAT.md,
//! phase 2). The bot plays through the real controller path (a pad's stick and buttons fed to
//! `Input`), and sees only what a player sees: a foe's tell from the frame its reaction delay has
//! passed, bolts in flight, its own stamina. Three skills:
//!
//! - **Novice**: walks at the nearest foe and mashes attack. Never guards.
//! - **Adept**: holds the guard when it sees a strike coming (reaction 18 ± 4 frames), attacks
//!   whatever is in reach while it has the stamina, keeping a block's worth in hand.
//! - **Expert**: raises the guard to put each strike in the parry window (reaction 12 ± 2,
//!   timing ± 2), and strikes only into openings (recovery, reeling, or a foe not winding up),
//!   never into a swing it could not guard out of before the next strike it has seen.
//!
//! `balance` plays every bout with every skill over a few seeds and reports win rate, time to
//! kill and damage taken against the targets of COMBAT.md §2.1 (`gym::Bout::target`).

use super::sparring::{BoutPhase, BoutStats};
use super::Game;
use crate::input::{Action, PadState};
use crate::logic::combat::duelist::HERO_RADIUS;
use crate::logic::combat::frames::{BOLT_RADIUS, BOLT_SPEED};
use crate::logic::combat::Kit;
use crate::logic::combat::{math, Phase, State};
use crate::logic::gym;
use crate::logic::heroes::{HeroKind, ROSTER};
use crate::logic::progression::{self, Tree};
use crate::logic::rng::Rng;
use crate::logic::types::{P, V};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Skill {
    Novice,
    Adept,
    Expert,
}

impl Skill {
    pub const ALL: [Skill; 3] = [Skill::Novice, Skill::Adept, Skill::Expert];

    pub fn name(self) -> &'static str {
        match self {
            Skill::Novice => "novice",
            Skill::Adept => "adept",
            Skill::Expert => "expert",
        }
    }

    /// Frames before it notices a tell, and how far that wanders either way.
    fn reaction(self) -> (u16, u16) {
        match self {
            Skill::Novice => (u16::MAX, 0),
            Skill::Adept => (18, 4),
            Skill::Expert => (12, 2),
        }
    }
}

/// A strike on its way to the hero: frames until it lands, and where it comes from.
#[derive(Clone, Copy, Debug)]
struct Threat {
    t: f64,
    from: V,
}

/// Frames before impact the expert raises its guard (the hit then lands on guard frame 5 or so).
const PARRY_LEAD: f64 = 6.0;
/// Frames before impact the adept raises its guard.
const BLOCK_LEAD: f64 = 22.0;
/// The reach it closes to before swinging: the string's reach, less a little.
const ENGAGE: f64 = 1.3;
/// Stamina the adept keeps for its guard.
const RESERVE: f64 = 24.0;
/// Frames from pressing attack until the guard can come up again (L1's cancel frame, and the raise).
const COMMIT: f64 = 16.0;

pub struct Bot {
    pub skill: Skill,
    /// The way round the walls to its target: from which tile, to which, and the step (the vaults).
    route: Option<(P, P, V)>,
    rng: Rng,
    /// The reaction delay drawn for each foe's current move (by foe id and the move's start frame).
    noticed: HashMap<u32, (u32, u16)>,
    /// The attack button was down last update (mashing toggles it).
    attack_down: bool,
    /// Frames the guard stays up after the last strike it was raised for.
    guard_hold: u16,
    /// The expert's timing error for the strike it is waiting on.
    lead: f64,
    /// Frames before it may drink again.
    sip: u16,
}

impl Bot {
    pub fn new(skill: Skill, seed: u32) -> Self {
        Self {
            skill,
            route: None,
            sip: 0,
            rng: Rng::new(seed.wrapping_mul(2654435761).wrapping_add(7)),
            noticed: HashMap::new(),
            attack_down: false,
            guard_hold: 0,
            lead: PARRY_LEAD,
        }
    }

    /// The strikes it has noticed, soonest first.
    fn threats(&mut self, g: &Game) -> Vec<Threat> {
        let Some(spar) = g.brawl.as_ref() else { return Vec::new() };
        let hp = g.hero.pos();
        let now = spar.threat.frame;
        let (base, spread) = self.skill.reaction();
        let mut out = Vec::new();
        for d in g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.alive()) {
            let State::Attack { mv, frame } = d.f.state else {
                self.noticed.remove(&d.id);
                continue;
            };
            let started = now.saturating_sub(frame as u32);
            let entry = self.noticed.entry(d.id).or_insert((started, 0));
            if entry.0 != started {
                *entry = (started, 0);
            }
            if entry.1 == 0 {
                let jitter = if spread == 0 { 0 } else { (self.rng.next() * (2 * spread + 1) as f64) as u16 };
                entry.1 = base.saturating_add(jitter).saturating_sub(spread).max(1);
            }
            let data = mv.data();
            if frame < entry.1 || data.phase(frame) == Phase::Recovery {
                continue;
            }
            let to = math::sub(hp, d.pos);
            let dist = math::len(to);
            if data.projectile {
                continue; // its bolt is watched in flight, below
            }
            let aimed = math::dot(math::norm(to), d.aim).clamp(-1.0, 1.0).acos() <= data.half_angle + 0.35;
            if !aimed || dist > data.reach + data.travel + HERO_RADIUS + 0.4 {
                continue;
            }
            // A lunge lands once it has closed the gap.
            let close = if data.travel > 0.0 {
                ((dist - data.reach - HERO_RADIUS).max(0.0) / (data.travel / data.active as f64)).floor()
            } else {
                0.0
            };
            let t = (data.startup as f64 - frame as f64).max(0.0) + close;
            out.push(Threat { t, from: d.pos });
        }
        for b in &spar.bolts {
            if b.reflected || b.touched {
                continue;
            }
            let dir = math::norm(b.vel);
            let to = math::sub(hp, b.pos);
            let along = math::dot(dir, to);
            let across = (math::len(to).powi(2) - along * along).max(0.0).sqrt();
            if along > 0.0 && across < BOLT_RADIUS + HERO_RADIUS + 0.1 {
                let speed = math::len(b.vel).max(BOLT_SPEED * 0.25);
                let t = ((along - BOLT_RADIUS - HERO_RADIUS).max(0.0) / (speed / 60.0)).floor();
                out.push(Threat { t, from: math::sub(b.pos, b.vel) });
            }
        }
        out.sort_by(|a, b| a.t.total_cmp(&b.t));
        out
    }

    /// Which way to walk from `from` to `to`: straight while nothing stands between, else the
    /// first step of the shortest way round (searched again only when either end changes tile).
    fn way(&mut self, g: &Game, from: V, to: V) -> V {
        let grid = &g.level.grid;
        let tile = |p: V| P::new(p.x.round() as i32, p.y.round() as i32);
        let n = (math::len(math::sub(to, from)) / 0.3).ceil().max(1.0) as usize;
        let clear = (0..=n).all(|k| {
            let p = math::add(from, math::scale(math::sub(to, from), k as f64 / n as f64));
            [(-0.28, -0.28), (0.28, -0.28), (-0.28, 0.28), (0.28, 0.28)].iter().all(|&(dx, dy)| {
                let t = tile(V::new(p.x + dx, p.y + dy));
                grid.is_walkable(t.x, t.y, false)
            })
        });
        if clear {
            self.route = None;
            return math::norm(math::sub(to, from));
        }
        let (a, b) = (tile(from), tile(to));
        if let Some((ra, rb, step)) = self.route {
            if ra == a && rb == b {
                return math::norm(math::sub(step, from));
            }
        }
        let step = match crate::logic::pathing::route_toward(grid, a, to, false, false, false) {
            Some(d) => {
                let (dx, dy) = d.vec();
                P::new(a.x + dx, a.y + dy).v()
            }
            None => to,
        };
        self.route = Some((a, b, step));
        math::norm(math::sub(step, from))
    }

    /// This update's stick and buttons.
    pub fn act(&mut self, g: &Game) -> PadState {
        let mut pad = PadState::default();
        let Some(spar) = g.brawl.as_ref() else { return pad };
        if !spar.live || !g.brawling() {
            self.attack_down = false;
            return pad;
        }
        let hp = g.hero.pos();
        let threats = self.threats(g);
        let soonest = threats.first().copied();

        // Low on health, the adept and the expert drink from the belt.
        self.sip = self.sip.saturating_sub(1);
        if self.skill != Skill::Novice && self.sip == 0 && !g.belt.is_empty() && (g.hero.hp as f64) < g.hero.max_hp as f64 * 0.4 {
            self.sip = 240;
            pad.actions.insert(Action::Drink);
        }

        // Defence.
        let guard_at = match self.skill {
            Skill::Novice => None,
            Skill::Adept => soonest.filter(|t| t.t <= BLOCK_LEAD),
            Skill::Expert => soonest.filter(|t| t.t <= self.lead),
        };
        if let Some(t) = guard_at {
            self.guard_hold = 8;
            pad.actions.insert(Action::Block);
            let face = math::norm(math::sub(t.from, hp));
            pad.left = (face.x as f32, face.y as f32);
            return pad;
        }
        if self.guard_hold > 0 {
            self.guard_hold -= 1;
            pad.actions.insert(Action::Block);
            if self.guard_hold == 0 {
                self.lead = PARRY_LEAD + (self.rng.next() * 5.0).floor() - 2.0;
            }
            return pad;
        }

        // Offence: the most open foe (reeling, then recovering), then the nearest; in the vaults those
        // hunting him first, and when none is, the nearest pack asleep (to wake it).
        let all: Vec<_> = g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.alive()).collect();
        let hunting: Vec<_> = all.iter().copied().filter(|d| d.engaged()).collect();
        // Between fights (or hurt), what lies near goes into the belt first: a potion, a weapon, a boon.
        let hurt = (g.hero.hp as f64) < g.hero.max_hp as f64 * 0.6;
        if self.skill != Skill::Novice && (hunting.is_empty() || hurt) {
            let near = g.vault.as_ref().and_then(|v| {
                v.drops
                    .iter()
                    .filter(|d| !d.taken && d.landed())
                    .map(|d| d.at.v())
                    .filter(|p| math::len(math::sub(*p, hp)) < 6.0)
                    .min_by(|a, b| math::len(math::sub(*a, hp)).total_cmp(&math::len(math::sub(*b, hp))))
            });
            if let Some(p) = near {
                let dir = self.way(g, hp, p);
                pad.left = (dir.x as f32, dir.y as f32);
                return pad;
            }
        }
        let foes = if hunting.is_empty() { all } else { hunting };
        let openness = |d: &crate::logic::combat::Duelist| {
            if d.f.vulnerable() || d.f.reeling().is_some() {
                0
            } else if matches!(d.f.attack(), Some((_, Phase::Recovery, _))) {
                1
            } else {
                2
            }
        };
        let target = foes.iter().min_by(|a, b| {
            let (da, db) = (math::len(math::sub(a.pos, hp)), math::len(math::sub(b.pos, hp)));
            if self.skill == Skill::Novice {
                da.total_cmp(&db)
            } else {
                openness(a).cmp(&openness(b)).then(da.total_cmp(&db))
            }
        });
        let Some(target) = target else { return pad };
        let to = math::sub(target.pos, hp);
        let dist = math::len(to) - target.radius();
        let dir = self.way(g, hp, target.pos);
        if dist > ENGAGE {
            pad.left = (dir.x as f32, dir.y as f32);
            self.attack_down = false;
            return pad;
        }
        let stamina = g.hero.stamina;
        let swing = match self.skill {
            Skill::Novice => true,
            // It keeps a block's worth in hand.
            Skill::Adept => stamina >= 16.0 + RESERVE,
            // Into openings only: never into a wind-up that would land on its swing.
            // ...and never into a swing it could not guard out of before the next strike it has seen lands.
            Skill::Expert => {
                let winding = matches!(target.f.attack(), Some((_, Phase::Startup | Phase::Active, _)));
                let clear = soonest.is_none_or(|t| t.t > COMMIT);
                stamina >= 16.0 && clear && (openness(target) < 2 || !winding)
            }
        };
        if swing {
            // Mash: the button goes down every other update.
            self.attack_down = !self.attack_down;
            if self.attack_down {
                pad.actions.insert(Action::Attack);
            }
        }
        pad
    }
}

/// How a bout went.
#[derive(Clone, Copy, Debug)]
pub struct BoutResult {
    pub won: bool,
    /// Seconds of fighting (to the win, the defeat, or the time limit).
    pub secs: f64,
    pub stats: BoutStats,
}

/// Seconds a bout may run before it counts as a timeout (neither won nor lost).
pub const LIMIT: f64 = 120.0;

/// A new game on a fresh save, whatever the save folder holds: no progress, no adventure under way
/// (one run's broken shield or bought tier must never leak into the next).
fn fresh_game(seed: u32) -> Game {
    let mut g = Game::new();
    g.reseed(seed);
    g.progress = crate::logic::progression::Progress::default();
    g.saved_run = None;
    g.hero_kind = HeroKind::Ragnar;
    g.equip_from_progress();
    g
}

/// A hero, with his blade, shield and path at their tiers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Loadout {
    pub hero: HeroKind,
    pub blade: u8,
    pub shield: u8,
    pub path: u8,
}

impl Loadout {
    /// Every tree at `tier`.
    pub fn all(hero: HeroKind, tier: u8) -> Self {
        Self { hero, blade: tier, shield: tier, path: tier }
    }

    /// Puts it on the game's hero.
    fn equip(&self, g: &mut Game) {
        g.hero_kind = self.hero;
        let h = g.progress.hero_mut(self.hero);
        h.owned[Tree::Armory.index()] = self.blade;
        h.owned[Tree::Shield.index()] = self.shield;
        h.owned[Tree::Path.index()] = self.path;
        h.wield = self.blade;
        h.carry = self.shield;
        g.hero.shield = Some(crate::logic::gear::ShieldGear::new(self.shield));
        g.equip_from_progress();
    }
}

/// Plays bout `bout` (0-based) with `skill`, seeded with `seed`, from a fresh game on a clean save.
pub fn play_bout(bout: usize, skill: Skill, seed: u32) -> BoutResult {
    play_bout_as(bout, skill, seed, None)
}

/// The same, with the hero and his kit of `loadout` (none: a fresh Ragnar).
pub fn play_bout_as(bout: usize, skill: Skill, seed: u32, loadout: Option<Loadout>) -> BoutResult {
    let mut g = fresh_game(seed);
    g.debug_ring(bout + 1);
    if let Some(l) = loadout {
        // (After the jump, which takes the saved run's hero from the title.)
        l.equip(&mut g);
        g.start_bout();
    }
    g.hero.free_move = true;
    let mut bot = Bot::new(skill, seed);
    let step = crate::app::STEP;
    let steps = ((LIMIT + super::sparring::COUNTDOWN + 1.0) / step) as usize;
    for _ in 0..steps {
        let pad = bot.act(&g);
        g.input.set_pad(Some(pad));
        g.update(step);
        let (spar, b) = (g.spar.as_ref().unwrap(), g.brawl.as_ref().unwrap());
        match spar.phase {
            BoutPhase::Won(_) | BoutPhase::Lost(_) => {
                let won = matches!(spar.phase, BoutPhase::Won(_));
                return BoutResult { won, secs: b.stats.frames as f64 / 60.0, stats: b.stats };
            }
            _ => {}
        }
    }
    let b = g.brawl.as_ref().unwrap();
    BoutResult { won: false, secs: b.stats.frames as f64 / 60.0, stats: b.stats }
}

/// One bout and skill over every seed: the averages.
#[derive(Clone, Copy, Debug, Default)]
pub struct Summary {
    pub runs: u32,
    pub wins: u32,
    /// Mean seconds to win (wins only; 0 if none).
    pub ttk: f64,
    /// Mean health lost (every run).
    pub taken: f64,
    pub parries: f64,
    pub blocks: f64,
    pub hurt: f64,
    pub caught: f64,
    pub guard_breaks: f64,
    pub overlaps: u32,
}

impl Summary {
    pub fn win_rate(&self) -> f64 {
        self.wins as f64 / self.runs.max(1) as f64
    }
}

/// Plays `bout` with `skill` once per seed in `seeds`.
pub fn summarize(bout: usize, skill: Skill, seeds: &[u32]) -> Summary {
    summarize_as(bout, skill, seeds, None)
}

/// The same, with `loadout`.
pub fn summarize_as(bout: usize, skill: Skill, seeds: &[u32], loadout: Option<Loadout>) -> Summary {
    let mut s = Summary::default();
    let mut win_secs = 0.0;
    for &seed in seeds {
        let r = play_bout_as(bout, skill, seed, loadout);
        s.runs += 1;
        if r.won {
            s.wins += 1;
            win_secs += r.secs;
        }
        s.taken += r.stats.taken;
        s.parries += r.stats.parries as f64;
        s.blocks += r.stats.blocks as f64;
        s.hurt += r.stats.hurt as f64;
        s.caught += r.stats.caught as f64;
        s.guard_breaks += r.stats.guard_breaks as f64;
        s.overlaps += r.stats.overlaps;
    }
    let n = s.runs.max(1) as f64;
    s.ttk = if s.wins > 0 { win_secs / s.wins as f64 } else { 0.0 };
    s.taken /= n;
    s.parries /= n;
    s.blocks /= n;
    s.hurt /= n;
    s.caught /= n;
    s.guard_breaks /= n;
    s
}

/// Every bout with every skill over `seeds`, as a table against the targets.
pub fn balance(seeds: &[u32]) -> String {
    let mut out = String::from("bout                 skill   win   ttk(s)  target    taken  hurt caught gbrk  parry block overlaps\n");
    for (k, b) in gym::BOUTS.iter().enumerate() {
        for skill in Skill::ALL {
            let s = summarize(k, skill, seeds);
            let target = if skill == Skill::Expert { format!("{:>4.0}-{:<4.0}", b.target.0, b.target.1) } else { "    -    ".to_string() };
            out += &format!(
                "{:<20} {:<7} {:>4.0}% {:>7.1}  {}  {:>5.0}  {:>4.1} {:>5.1} {:>4.1}  {:>5.1} {:>5.1} {:>4}\n",
                b.name,
                skill.name(),
                s.win_rate() * 100.0,
                s.ttk,
                target,
                s.taken,
                s.hurt,
                s.caught,
                s.guard_breaks,
                s.parries,
                s.blocks,
                s.overlaps
            );
        }
    }
    out
}

/// The tiers the kit sweep plays.
pub const SWEEP: [u8; 4] = [1, 4, 7, 10];

/// Every hero at each tier of `SWEEP` (blade, shield and path alike): the expert's times on the
/// Brute, the Trio and the Gauntlet, and how the adept fares on the Brute and the Trio (COMBAT.md §8).
pub fn kit_sweep(seeds: &[u32]) -> String {
    let mut out = String::from(
        "hero      tier  tempo power reach parry cost | expert: brute   trio  gauntlet      | adept: brute (blocks)  trio (win, blocks)\n",
    );
    for hero in ROSTER {
        for tier in SWEEP {
            let l = Some(Loadout::all(hero, tier));
            let k = Kit::of(hero, tier, 1.0, Some(tier), tier);
            let eb = summarize_as(0, Skill::Expert, seeds, l);
            let et = summarize_as(3, Skill::Expert, seeds, l);
            let eg = summarize_as(4, Skill::Expert, seeds, l);
            let ab = summarize_as(0, Skill::Adept, seeds, l);
            let at = summarize_as(3, Skill::Adept, seeds, l);
            out += &format!(
                "{:<9} {:>4}  {:>5.2} {:>5.2} {:>5.1} {:>4}f {:>4.2} | {:>13.1} {:>6.1} {:>6.1} ({:>3.0}%) | {:>12.1} ({:>4.1})  {:>4.0}% {:>5.1}\n",
                hero.name(),
                tier,
                k.strike.tempo,
                k.strike.power,
                k.strike.reach,
                k.guard.parry,
                k.guard.cost,
                eb.ttk,
                et.ttk,
                eg.ttk,
                eg.win_rate() * 100.0,
                ab.ttk,
                ab.blocks,
                at.win_rate() * 100.0,
                at.blocks
            );
        }
    }
    out
}

/// Every blade and shield as the ring reads it (COMBAT.md §8).
pub fn kit_table() -> String {
    let mut out = String::from("hero      tier blade                     tempo power poise reach  arc   finisher        traits\n");
    for hero in ROSTER {
        for tier in 1..=progression::TIERS {
            let s = Kit::of(hero, tier, 1.0, None, 0).strike;
            let l3 = Kit::of(hero, tier, 1.0, None, 0).moves()[2];
            let finisher = if s.thrown.is_some() {
                "thrown".to_string()
            } else if s.whirl {
                "whirl".to_string()
            } else {
                format!("{:.1} t", l3.reach)
            };
            let mut traits = Vec::new();
            if s.pierce > 0.0 {
                traits.push("pierce".to_string());
            }
            if s.crit > 0.0 {
                traits.push(format!("crit +{:.2}", s.crit));
            }
            if s.blood > 0.0 {
                traits.push(format!("blood {}", s.blood));
            }
            if s.finisher_power > 1.0 {
                traits.push(format!("finisher x{}", s.finisher_power));
            }
            out += &format!(
                "{:<9} {:>4} {:<25} {:>5.2} {:>5.2} {:>5.2} {:>+5.1} {:>+4.0}deg {:<15} {}\n",
                hero.name(),
                tier,
                progression::node(hero, Tree::Armory, tier).name,
                s.tempo,
                s.power,
                s.poise,
                s.reach,
                s.arc.to_degrees(),
                finisher,
                traits.join(", ")
            );
        }
    }
    out += "\ntier shield                    parry cost  riposte deflect reflect bolt-proof break\n";
    for tier in 1..=progression::TIERS {
        let g = crate::logic::combat::kit::Guard::of(tier);
        out += &format!(
            "{:>4} {:<25} {:>4}f {:>4.2} {:>7.0} {:>6}f {:>6.1}x {:<10} {:.0}%/{}f\n",
            tier,
            progression::node(HeroKind::Ragnar, Tree::Shield, tier).name,
            g.parry,
            g.cost,
            g.riposte,
            g.deflect,
            g.reflect,
            if g.bolt_proof { "yes" } else { "no" },
            g.break_share * 100.0,
            g.break_frames
        );
    }
    out
}

/// What each part of a kit is worth: the expert's time on the Brute and the Trio with one tree
/// raised at a time (`hero`, tier `tier`), against none raised.
pub fn kit_parts(hero: HeroKind, tier: u8, seeds: &[u32]) -> String {
    let parts = [
        ("none", Loadout { hero, blade: 1, shield: 1, path: 0 }),
        ("blade", Loadout { hero, blade: tier, shield: 1, path: 0 }),
        ("shield", Loadout { hero, blade: 1, shield: tier, path: 0 }),
        ("path", Loadout { hero, blade: 1, shield: 1, path: tier }),
        ("all", Loadout::all(hero, tier)),
    ];
    let mut out = format!(
        "{} tier {tier}: part   brute  trio
",
        hero.name()
    );
    for (name, l) in parts {
        let b = summarize_as(0, Skill::Expert, seeds, Some(l));
        let t = summarize_as(3, Skill::Expert, seeds, Some(l));
        out += &format!(
            "{:<20} {:>6.1} {:>5.1}
",
            name, b.ttk, t.ttk
        );
    }
    out
}

/// How a crawl through the Vaults of Mammon went (COMBAT.md §9).
#[derive(Clone, Copy, Debug, Default)]
pub struct Crawl {
    /// Every foe of the vaults slain.
    pub cleared: bool,
    /// Lives lost, and whether they ran out.
    pub deaths: u32,
    pub game_over: bool,
    pub foes: u32,
    pub kills: u32,
    /// Seconds in all, and seconds with at least one foe hunting him.
    pub secs: f64,
    pub combat_secs: f64,
    /// The most foes hunting him at once.
    pub peak: usize,
    pub stats: BoutStats,
}

impl Crawl {
    /// Overlapping strikes per minute of fighting.
    pub fn overlaps_per_minute(&self) -> f64 {
        self.stats.overlaps as f64 / (self.combat_secs / 60.0).max(1e-9)
    }
}

/// A bot crawls the Vaults of Mammon from the antechamber, waking pack after pack and fighting it
/// out, until every foe is slain, its lives run out, or `limit` seconds pass.
pub fn crawl(skill: Skill, seed: u32, loadout: Option<Loadout>, limit: f64) -> Crawl {
    let mut g = fresh_game(seed);
    g.map_jump(super::save::Stage::Realm(crate::logic::sins::Sin::Greed));
    if let Some(l) = loadout {
        l.equip(&mut g);
    }
    g.enter(super::Screen::Playing);
    let kit = g.hero_kit();
    if let Some(b) = g.brawl.as_mut() {
        b.reset(kit);
    }
    g.hero.free_move = true;
    let mut bot = Bot::new(skill, seed);
    let foes = g.enemies.iter().filter(|e| e.duel.is_some()).count() as u32;
    let lives = g.score.lives;
    let step = crate::app::STEP;
    let mut out = Crawl { foes, ..Crawl::default() };
    let mut t = 0.0;
    while t < limit {
        let playing = g.screen == super::Screen::Playing;
        let pad = if playing { bot.act(&g) } else { PadState::default() };
        g.input.set_pad(Some(pad));
        g.update(step);
        t += step;
        match g.screen {
            super::Screen::GameOver => {
                out.game_over = true;
                break;
            }
            super::Screen::Toll => break,
            super::Screen::Ready => g.enter(super::Screen::Playing),
            _ => {}
        }
        let hunting = g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.engaged()).count();
        out.peak = out.peak.max(hunting);
        if hunting > 0 {
            out.combat_secs += step;
        }
        if !g.enemies.iter().any(|e| e.duel.as_ref().is_some_and(|d| d.alive())) {
            out.cleared = true;
            break;
        }
    }
    out.secs = t;
    out.deaths = lives.saturating_sub(g.score.lives) + u32::from(out.game_over);
    out.stats = g.brawl.as_ref().map(|b| b.stats).unwrap_or_default();
    out.kills = out.foes - g.enemies.iter().filter(|e| e.duel.as_ref().is_some_and(|d| d.alive())).count() as u32;
    out
}

/// Crawls by every skill (and the kit at each tier of `tiers`, Ragnar's), one line each.
pub fn crawl_report(seeds: &[u32], tiers: &[u8], limit: f64) -> String {
    let mut out =
        String::from("skill   tier  cleared  kills/foes  deaths  minutes  combat  peak  taken  parry block caught  overlaps/min\n");
    for &tier in tiers {
        for skill in [Skill::Adept, Skill::Expert] {
            let runs: Vec<Crawl> = seeds.iter().map(|&s| crawl(skill, s, Some(Loadout::all(HeroKind::Ragnar, tier)), limit)).collect();
            let n = runs.len() as f64;
            let avg = |f: &dyn Fn(&Crawl) -> f64| runs.iter().map(f).sum::<f64>() / n;
            out += &format!(
                "{:<7} {:>4}  {:>5.0}%  {:>5.0}/{:<4.0}  {:>6.1}  {:>7.1}  {:>6.1}  {:>4.0}  {:>5.0}  {:>5.0} {:>5.0} {:>6.0}  {:>12.2}\n",
                skill.name(),
                tier,
                avg(&|c| f64::from(u8::from(c.cleared))) * 100.0,
                avg(&|c| c.kills as f64),
                avg(&|c| c.foes as f64),
                avg(&|c| c.deaths as f64),
                avg(&|c| c.secs / 60.0),
                avg(&|c| c.combat_secs / 60.0),
                avg(&|c| c.peak as f64),
                avg(&|c| c.stats.taken),
                avg(&|c| c.stats.parries as f64),
                avg(&|c| c.stats.blocks as f64),
                avg(&|c| c.stats.caught as f64),
                avg(&|c| c.overlaps_per_minute())
            );
        }
    }
    out
}
