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
//! Against the later stages' legends (COMBAT.md §11) the adept and the expert also walk out of
//! a burrow's mark on the floor (leaping when it is too late to walk), step out of fire,
//! leap a grab they cannot parry (the adept), strike Tepegöz only while his eye is in reach (the
//! expert), and never strike a felled dragon begging for a second blow. Nothing out of reach is a
//! target. Against a prince (§13) they step off his coins before they burst, by the way clearest
//! of the others, and held in his fist they strike him (as a masher would anyway).
//!
//! `balance` plays every bout with every skill over a few seeds and reports win rate, time to
//! kill and damage taken against the targets of COMBAT.md §2.1 (`gym::Bout::target`).

use super::sparring::{BoutPhase, BoutStats};
use super::Game;
use crate::input::{Action, PadState};
use crate::logic::combat::archetype::Power;
use crate::logic::combat::duelist::HERO_RADIUS;
use crate::logic::combat::frames::{Shape, BOLT_RADIUS, BOLT_SPEED};
use crate::logic::combat::fsm::GuardRule;
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

/// A strike on its way to the hero: frames until it lands, where it comes from, and what his
/// guard can do about it.
#[derive(Clone, Copy, Debug)]
struct Threat {
    t: f64,
    from: V,
    rule: GuardRule,
}

/// Ground he must be off before it falls (a mark, fire): frames until it does, and the quickest
/// way out.
#[derive(Clone, Copy, Debug)]
struct Dodge {
    t: f64,
    out: V,
}

/// Frames before a blow from the ground falls at which the bot leaps instead of walking.
const LEAP_AT: f64 = 8.0;
/// Frames ahead the bot looks to leave the ground a blow will fall on.
const DODGE_AHEAD: f64 = 40.0;
/// Frames earlier than its flight says that a bolt is reckoned to arrive: it reaches him on the
/// frame it is drawn touching him, a frame or two before a blow's first active frame would.
const BOLT_EARLY: f64 = 2.0;
/// Frames before a grab the adept leaps it.
const GRAB_LEAP: f64 = 9.0;

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
            let data = d.data(mv);
            if frame < entry.1 || data.phase(frame) == Phase::Recovery || !data.strikes() {
                continue;
            }
            let to = math::sub(hp, d.pos);
            let dist = math::len(to);
            if data.projectile || !data.parriable() {
                continue; // its bolt is watched in flight, below; the ground's blows are dodged
            }
            let start = (data.startup as f64 - frame as f64).max(0.0);
            let off = math::dot(math::norm(to), d.aim).clamp(-1.0, 1.0).acos();
            let t = match data.shape {
                Shape::Quake { radius, .. } if dist <= radius + HERO_RADIUS + 0.3 => start,
                _ => {
                    let aimed = off <= data.half_angle + 0.35;
                    if !aimed || dist > data.reach + HERO_RADIUS + 0.4 {
                        continue;
                    }
                    start
                }
            };
            let rule = if data.blockable() { GuardRule::Full } else { GuardRule::ParryOnly };
            out.push(Threat { t, from: d.pos, rule });
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
                let t = ((along - BOLT_RADIUS - HERO_RADIUS).max(0.0) / (speed / 60.0)).floor() - BOLT_EARLY;
                out.push(Threat { t, from: math::sub(b.pos, b.vel), rule: GuardRule::Full });
            }
        }
        out.sort_by(|a, b| a.t.total_cmp(&b.t));
        out
    }

    /**
     * The ground it has seen a blow is about to fall on, with him on it: a burrow's mark on the
     * floor (once its reaction has passed), Surtr's fire under his feet. The way out is straight
     * away from a mark's middle, off the fire.
     */
    fn dodges(&mut self, g: &Game) -> Vec<Dodge> {
        let Some(spar) = g.brawl.as_ref() else { return Vec::new() };
        let hp = g.hero.pos();
        let now = spar.threat.frame;
        let mut out = Vec::new();
        for d in g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.alive()) {
            let State::Attack { mv, frame } = d.f.state else { continue };
            let data = d.data(mv);
            if data.phase(frame) == Phase::Recovery || data.parriable() || !data.strikes() {
                continue;
            }
            let started = now.saturating_sub(frame as u32);
            let seen = self.noticed.get(&d.id).is_some_and(|&(at, delay)| at == started && delay > 0 && frame >= delay);
            if !seen {
                continue;
            }
            let t = (data.startup as f64 - frame as f64).max(0.0);
            if let Shape::Mark { radius } = data.shape {
                let away = math::sub(hp, d.lore.mark);
                if math::len(away) < radius + HERO_RADIUS + 0.35 {
                    let out_dir = if math::len(away) < 0.05 {
                        let a = math::norm(math::sub(hp, d.lore.from));
                        V::new(-a.y, a.x)
                    } else {
                        math::norm(away)
                    };
                    out.push(Dodge { t, out: out_dir });
                }
            }
        }
        for h in spar.hazards.iter().filter(|h| h.burns(hp, HERO_RADIUS + 0.1)) {
            out.push(Dodge { t: 0.0, out: math::norm(math::sub(hp, h.pos)) });
        }
        // A prince's coins lying on the floor, with him on one: off by the way clearest of them all.
        let armed: Vec<&crate::logic::combat::prince::Trap> = spar.traps.iter().filter(|t| t.armed()).collect();
        if let Some(t) = armed.iter().filter(|t| t.reaches(hp, HERO_RADIUS + 0.35)).min_by_key(|t| t.fuse) {
            let clearance = |dir: V| {
                let p = math::add(hp, math::scale(dir, 1.7));
                armed.iter().map(|o| math::len(math::sub(p, o.pos)) - o.radius).fold(f64::MAX, f64::min)
            };
            let away = math::sub(hp, t.pos);
            let base = if math::len(away) < 0.05 { 0.0 } else { math::angle(away) };
            let best = (0..16)
                .map(|k| math::from_angle(base + k as f64 * std::f64::consts::TAU / 16.0))
                .filter(|&dir| Self::keep_in(g, hp, dir) == dir)
                .max_by(|a, b| clearance(*a).total_cmp(&clearance(*b)))
                .unwrap_or_else(|| math::from_angle(base));
            out.push(Dodge { t: t.fuse as f64, out: best });
        }
        for d in out.iter_mut() {
            d.out = Self::keep_in(g, hp, d.out);
        }
        out.sort_by(|a, b| a.t.total_cmp(&b.t));
        out
    }

    /// `out`, or the nearest way round from it that keeps him on the fight's floor (walking or
    /// leaping off the ring calls the bout off).
    fn keep_in(g: &Game, hp: V, out: V) -> V {
        let inside = |p: V| {
            if g.in_ring() {
                let (lo, hi) = gym::ring_bounds();
                p.x > lo.x + 0.8 && p.x < hi.x - 0.8 && p.y > lo.y + 0.8 && p.y < hi.y - 0.8
            } else {
                g.level.grid.is_walkable(p.x.round() as i32, p.y.round() as i32, false)
            }
        };
        let base = math::angle(out);
        for k in [0.0, 0.6, -0.6, 1.2, -1.2, 1.8, -1.8, std::f64::consts::PI] {
            let dir = math::from_angle(base + k);
            if inside(math::add(hp, math::scale(dir, 1.6))) {
                return dir;
            }
        }
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
        let dodges = self.dodges(g);
        let can_leap = !g.hero.airborne();
        let leap = |pad: &mut PadState, out: V| {
            pad.left = (out.x as f32, out.y as f32);
            if can_leap {
                pad.actions.insert(Action::Jump);
            }
        };
        let guard_lead = match self.skill {
            Skill::Novice => -1.0,
            Skill::Adept => BLOCK_LEAD,
            Skill::Expert => self.lead,
        };
        let dodge = dodges.first().copied().filter(|d| self.skill != Skill::Novice && d.t <= DODGE_AHEAD);
        if let Some(d) = dodge {
            // A blow from the ground about to fall with him on it, and one to guard coming at the same
            // time, or too late to walk out: the leap clears them all.
            let both = soonest.is_some_and(|t| t.t <= guard_lead + 4.0 && d.t <= t.t + 12.0);
            if d.t <= LEAP_AT || both {
                leap(&mut pad, d.out);
                self.attack_down = false;
                return pad;
            }
        }
        if self.skill != Skill::Novice && soonest.is_none_or(|t| t.t > guard_lead) {
            // Off the ground a blow is about to fall on (the guard can wait).
            if let Some(d) = dodge {
                pad.left = (d.out.x as f32, d.out.y as f32);
                self.attack_down = false;
                return pad;
            }
        }
        // A grab no shield holds: the adept leaps it (the expert parries it below).
        if let Some(t) = soonest.filter(|t| self.skill == Skill::Adept && t.rule == GuardRule::ParryOnly && t.t <= GRAB_LEAP) {
            leap(&mut pad, math::norm(math::sub(hp, t.from)));
            return pad;
        }

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
        if self.guard_hold > 0 && dodge.is_none() {
            self.guard_hold -= 1;
            pad.actions.insert(Action::Block);
            if self.guard_hold == 0 {
                self.lead = PARRY_LEAD + (self.rng.next() * 5.0).floor() - 2.0;
            }
            return pad;
        }
        self.guard_hold = 0;

        // Offence: the most open foe (reeling, then recovering), then the nearest; in the vaults those
        // hunting him first, and when none is, the nearest pack asleep (to wake it).
        // (Nothing out of his reach is a target, nor a felled dragon begging to be struck again.)
        let all: Vec<_> =
            g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.alive() && !d.untouchable() && !d.begging()).collect();
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
        // A felled dragon in the way of the swing: a careful player holds his hand.
        let begging_near = g
            .enemies
            .iter()
            .filter_map(|e| e.duel.as_deref())
            .any(|d| d.begging() && math::len(math::sub(d.pos, hp)) < ENGAGE + d.radius() + 0.8);
        let swing = match self.skill {
            Skill::Novice => true,
            _ if begging_near => false,
            // It keeps a block's worth in hand.
            Skill::Adept => stamina >= 16.0 + RESERVE,
            // Into openings only: never into a wind-up that would land on its swing.
            // ...and never into a swing it could not guard out of before the next strike it has seen lands;
            // and Tepegöz only while his eye is in reach.
            Skill::Expert => {
                let winding = matches!(target.f.attack(), Some((_, Phase::Startup | Phase::Active, _)));
                // A press now commits it to the rest of this swing and the next one's cancel frame
                // (from neutral, L1's); nothing it has seen may land before it can guard again, nor
                // a blow from the ground fall before it could walk out.
                let commit = match spar.hero.attack() {
                    Some((mv, _, frame)) => {
                        let d = spar.hero.data(mv);
                        let next = d.chain.map(|c| spar.hero.data(c).cancel_frame()).unwrap_or(0);
                        d.cancel_frame().saturating_sub(frame) as f64 + next as f64 + 2.0
                    }
                    None => COMMIT,
                };
                let clear = soonest.is_none_or(|t| t.t > commit) && dodges.first().is_none_or(|d| d.t > commit + DODGE_AHEAD * 0.5);
                let warded = target.breed.power == Power::Ward && !target.exposed();
                stamina >= 16.0 && clear && !warded && (openness(target) < 2 || !winding)
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
pub(super) fn fresh_game(seed: u32) -> Game {
    let mut g = Game::new();
    g.reseed(seed);
    // The harnesses play without the juice (WARBAND.md §4): their numbers must not move with the feel.
    g.set_juice_off(crate::render::profile::fx::JUICE);
    g.progress = crate::logic::progression::Progress::default();
    g.saved_run = None;
    // (And no grudges carried over from the last run's crawl: each run meets its own.)
    g.warband = Default::default();
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
    pub(super) fn equip(&self, g: &mut Game) {
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

/// The kit tiers a stage's report plays: two below its bouts' own tier, theirs, and three above
/// (within 1 to 10); the third stage's are 1, 4 and 7.
pub fn stage_tiers(stage: u8) -> Vec<u8> {
    if stage <= 3 {
        return vec![1, 4, 7];
    }
    let tier = gym::bouts_of(stage).next().map(|k| gym::BOUTS[k].tier).unwrap_or(1);
    let mut tiers = vec![tier.saturating_sub(2).max(1), tier, (tier + 3).min(progression::TIERS)];
    tiers.dedup();
    tiers
}

/// A stage's bouts (COMBAT.md §10, §11) with every skill, and Ragnar's kit at each tier of `tiers`
/// (1: a fresh Ragnar, as in `balance`).
pub fn stage_report(seeds: &[u32], stage: u8, tiers: &[u8]) -> String {
    let mut out = String::from(
        "bout                 tier  skill   win   ttk(s)  target    taken  hurt  parry block overlaps
",
    );
    for k in gym::bouts_of(stage) {
        let b = &gym::BOUTS[k];
        for &tier in tiers {
            let loadout = (tier > 1).then(|| Loadout::all(HeroKind::Ragnar, tier));
            for skill in Skill::ALL {
                let s = summarize_as(k, skill, seeds, loadout);
                let target =
                    if skill == Skill::Expert { format!("{:>4.0}-{:<4.0}", b.target.0, b.target.1) } else { "    -    ".to_string() };
                out += &format!(
                    "{:<20} {:>4}  {:<7} {:>4.0}% {:>7.1}  {}  {:>5.0}  {:>4.1}  {:>5.1} {:>5.1} {:>4}
",
                    b.name,
                    tier,
                    skill.name(),
                    s.win_rate() * 100.0,
                    s.ttk,
                    target,
                    s.taken,
                    s.hurt,
                    s.parries,
                    s.blocks,
                    s.overlaps
                );
            }
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
    /// The Drum (WARBAND.md §6): the share of fighting time at peak, the peaks, and the mean rest
    /// between fights (seconds; 0 if none finished).
    pub peak_share: f64,
    pub peaks: u32,
    pub rest: f64,
    /// The packs (WARBAND.md §7): broken, and rallies sounded.
    pub routs: u32,
    pub rallies: u32,
}

/// The Drum, second by second through a crawl: (seconds, intensity, phase).
pub type DrumTrace = Vec<(f64, f64, crate::logic::war::drum::DrumPhase)>;

impl Crawl {
    /// Overlapping strikes per minute of fighting.
    pub fn overlaps_per_minute(&self) -> f64 {
        self.stats.overlaps as f64 / (self.combat_secs / 60.0).max(1e-9)
    }
}

/// A bot crawls the Vaults of Mammon from the antechamber, waking pack after pack and fighting it
/// out, until every foe is slain, its lives run out, or `limit` seconds pass.
pub fn crawl(skill: Skill, seed: u32, loadout: Option<Loadout>, limit: f64) -> Crawl {
    crawl_traced(skill, seed, loadout, limit, &mut None)
}

/// A crawl, with the Drum's trace kept in `trace` if it is given one.
pub fn crawl_traced(skill: Skill, seed: u32, loadout: Option<Loadout>, limit: f64, trace: &mut Option<DrumTrace>) -> Crawl {
    let mut g = fresh_game(seed);
    if let Some(n) = std::env::var("VH_CRAWL_RIVALS").ok().and_then(|v| v.parse::<usize>().ok()) {
        // Rivals waiting in the vaults (WARBAND.md §8), for a crawl against the Ledger.
        use crate::logic::enemy::EnemyKind;
        use crate::logic::war::names::Deed;
        for k in [EnemyKind::Skeleton, EnemyKind::Werewolf, EnemyKind::Witch, EnemyKind::Blemmyes].into_iter().take(n) {
            let id = g.promote_rival(k, Deed::KillingBlow);
            for _ in 0..2 {
                g.warband.save.ledger.rank_up(id);
            }
        }
    }
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
        if let Some(tr) = trace.as_mut() {
            if tr.last().is_none_or(|l| t - l.0 >= 1.0) {
                tr.push((t, g.war.drum.intensity, g.war.drum.phase));
            }
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
    if std::env::var_os("VH_CRAWL_LEFT").is_some() {
        let hp = g.hero.pos();
        eprintln!("hero at ({:.1}, {:.1})", hp.x, hp.y);
        for d in g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.alive()) {
            eprintln!(
                "  left: {:?} at ({:.1}, {:.1}) hp {:.0}/{:.0} den {:?} state {:?}",
                d.breed.kind, d.pos.x, d.pos.y, d.hp, d.max_hp, d.den, d.f.state
            );
        }
    }
    let drum = &g.war.drum;
    out.peak_share = drum.peak_share();
    out.peaks = drum.peaks;
    out.routs = g.war.tally.routs;
    out.rallies = g.war.tally.rallies;
    out.rest = if drum.rests.is_empty() { 0.0 } else { drum.rests.iter().sum::<f64>() / drum.rests.len() as f64 };
    out
}

/// Crawls by every skill (and the kit at each tier of `tiers`, Ragnar's), one line each.
pub fn crawl_report(seeds: &[u32], tiers: &[u8], limit: f64) -> String {
    let mut out = String::from(
        "skill   tier  cleared  kills/foes  deaths  minutes  combat  peak  taken  parry block caught  overlaps/min  at-peak peaks  rest  routs rallies\n",
    );
    for &tier in tiers {
        for skill in [Skill::Adept, Skill::Expert] {
            let runs: Vec<Crawl> = seeds.iter().map(|&s| crawl(skill, s, Some(Loadout::all(HeroKind::Ragnar, tier)), limit)).collect();
            let n = runs.len() as f64;
            let avg = |f: &dyn Fn(&Crawl) -> f64| runs.iter().map(f).sum::<f64>() / n;
            out += &format!(
                "{:<7} {:>4}  {:>5.0}%  {:>5.0}/{:<4.0}  {:>6.1}  {:>7.1}  {:>6.1}  {:>4.0}  {:>5.0}  {:>5.0} {:>5.0} {:>6.0}  {:>12.2}  {:>6.0}% {:>5.1} {:>5.1} {:>6.1} {:>7.1}\n",
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
                avg(&|c| c.overlaps_per_minute()),
                avg(&|c| c.peak_share) * 100.0,
                avg(&|c| c.peaks as f64),
                avg(&|c| c.rest),
                avg(&|c| c.routs as f64),
                avg(&|c| c.rallies as f64)
            );
        }
    }
    out
}

/// How a fight in a prince's court went.
#[derive(Clone, Copy, Debug, Default)]
pub struct CourtFight {
    /// The prince fell, and the key lay on the floor.
    pub slain: bool,
    pub game_over: bool,
    pub deaths: u32,
    /// Seconds of play, the phase he was brought to, and the fight's numbers (the last life's).
    pub secs: f64,
    pub phase: u8,
    pub stats: BoutStats,
}

/// A bot fights Greed's prince in his court, the Counting House, until he falls, its lives run
/// out, or `limit` seconds pass.
pub fn court_fight(skill: Skill, seed: u32, loadout: Option<Loadout>, limit: f64) -> CourtFight {
    let mut g = fresh_game(seed);
    g.map_jump(super::save::Stage::Court(crate::logic::sins::Sin::Greed));
    if let Some(l) = loadout {
        l.equip(&mut g);
    }
    // (It plays from the top view, as in the ring and the vaults: its stick is the world's way round.)
    g.view.camera_mode = crate::render::view::CameraMode::Top;
    g.enter(super::Screen::Playing);
    let kit = g.hero_kit();
    if let Some(b) = g.brawl.as_mut() {
        b.reset(kit);
    }
    g.hero.free_move = true;
    let mut bot = Bot::new(skill, seed);
    let lives = g.score.lives;
    let step = crate::app::STEP;
    let mut out = CourtFight::default();
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
            super::Screen::Ready => g.enter(super::Screen::Playing),
            _ => {}
        }
        out.phase = out.phase.max(g.prince().map(|d| d.breed.phase).unwrap_or(0));

        if g.court.as_ref().is_some_and(|c| c.slain) {
            out.slain = true;
            break;
        }
    }
    out.secs = t;
    out.deaths = lives.saturating_sub(g.score.lives) + u32::from(out.game_over);
    out.stats = g.brawl.as_ref().map(|b| b.stats).unwrap_or_default();
    out
}
