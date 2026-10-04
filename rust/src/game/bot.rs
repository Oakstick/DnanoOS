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
//! Against the prince's cheats (WARBAND.md §10.3, §15.2), when nothing hunts it, the adept and the
//! expert refuse a whisper (they call the old man, who denies it), strike a mimic chest
//! rather than trade with him, and strike a mimic chest before they would open it.
//!
//! `balance` plays every bout with every skill over a few seeds and reports win rate, time to
//! kill and damage taken against the targets of COMBAT.md §2.1 (`gym::Bout::target`).

use super::sparring::{BoutPhase, BoutStats};
use super::Game;
use crate::input::{Action, PadState};
use crate::logic::combat::archetype::{Attack, Power};
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
    /// Duelists it leaves alone (a general it means to skip, and his room: WARBAND.md §15.1).
    pub ignore: Vec<u32>,
    /// The drop it is walking to and for how many updates, and those it gave up on (one wedged
    /// behind a pillar's corner would hold it for good).
    chase: (P, u32),
    given_up: Vec<P>,
    /// Where it stood a while ago, and for how many updates it has pushed without getting
    /// anywhere: wedged on a corner the straight line cuts, it takes the tiles' way round.
    stand: (V, u32),
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
            ignore: Vec::new(),
            chase: (P::new(-1, -1), 0),
            given_up: Vec::new(),
            stand: (V::new(f64::NAN, f64::NAN), 0),
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
        // A general's kindled marks (WARBAND.md §9): off the one he stands on, as its fuse runs out.
        if let Some(r) = g.general.as_ref() {
            let reach = crate::logic::war::general::KINDLE_RADIUS + HERO_RADIUS + 0.35;
            for v in r.volleys.iter().filter(|v| !v.turned && v.burst.is_none()) {
                let Some(due) = v.due else { continue };
                for m in v.marks.iter().filter(|m| math::len(math::sub(hp, **m)) < reach) {
                    let away = math::sub(hp, *m);
                    let dir = if math::len(away) < 0.05 { V::new(1.0, 0.0) } else { math::norm(away) };
                    // (Off it as soon as it is seen, while nothing else is close: not in the last
                    // moment, when other blows may be on their way.)
                    let _ = due;
                    out.push(Dodge { t: DODGE_AHEAD * 0.9, out: dir });
                }
            }
        }
        for d in out.iter_mut() {
            d.out = Self::keep_in(g, hp, d.out);
        }
        out.sort_by(|a, b| a.t.total_cmp(&b.t));
        out
    }

    /**
     * A general's instruments and the gate his muster glows at (WARBAND.md §15.2): with nothing
     * hunting it near (never mid-fight), the bot bars a glowing gate it can reach in time and breaks the instruments
     * it passes, the expert going farther out of its way than the adept (the novice never).
     */
    fn general_errand(&mut self, g: &Game, hp: V, hunting_near: bool) -> Option<PadState> {
        let (out_of_way, gate_reach) = match self.skill {
            Skill::Novice => return None,
            Skill::Adept => (6.0, 9.0),
            Skill::Expert => (16.0, 18.0),
        };
        let (instruments, gate) = g.general_targets();
        let mut pad = PadState::default();
        if hunting_near {
            return None;
        }
        if let Some(at) = gate.filter(|at| math::len(math::sub(*at, hp)) < gate_reach) {
            let d = math::len(math::sub(at, hp));
            if d > 0.35 {
                let dir = self.way(g, hp, at);
                pad.left = (dir.x as f32, dir.y as f32);
            }
            self.attack_down = false;
            return Some(pad);
        }
        let (_, at) = instruments
            .into_iter()
            .filter(|(_, at)| math::len(math::sub(*at, hp)) < out_of_way)
            .min_by(|a, b| math::len(math::sub(a.1, hp)).total_cmp(&math::len(math::sub(b.1, hp))))?;
        let to = math::sub(at, hp);
        let dir = if math::len(to) > 0.6 { self.way(g, hp, at) } else { math::norm(to) };
        pad.left = (dir.x as f32, dir.y as f32);
        if math::len(to) <= ENGAGE && g.hero.stamina >= 16.0 {
            self.attack_down = !self.attack_down;
            if self.attack_down {
                pad.actions.insert(Action::Attack);
            }
        }
        Some(pad)
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
        // Pushing a second (120 updates) without moving a tenth of a tile: wedged, so no shortcut.
        if math::len(math::sub(from, self.stand.0)) > 0.1 || self.stand.0.x.is_nan() {
            self.stand = (from, 0);
        } else {
            self.stand.1 += 1;
        }
        // (Not in the ring: there it presses on a foe's body by design, and the lab's numbers hold.)
        let wedged = self.stand.1 > 120 && !g.in_ring();
        let n = (math::len(math::sub(to, from)) / 0.3).ceil().max(1.0) as usize;
        let clear = !wedged
            && (0..=n).all(|k| {
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

        // The prince's cheats, while nothing hunts it.
        if let Some(p) = self.see_through(g, &mut pad) {
            return p;
        }
        // Ragnar's War Cry (his mastery's fourth rank): the expert cries when a body foe stands about
        // him (not reeling) and no other kind is left to strike him in it, and nothing it has seen
        // lands before the cry is done.
        if self.skill == Skill::Expert && g.war_cry_ready() {
            let cry = crate::logic::war::mastery::WAR_CRY;
            let living: Vec<_> = g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.alive()).collect();
            let near = living
                .iter()
                .filter(|d| d.breed.attack() == Attack::Body && d.f.reeling().is_none())
                .filter(|d| math::len(math::sub(d.pos, hp)) <= cry.radius + d.radius())
                .count();
            // (A bolt or a blade from outside the cry would find him in it.)
            let worth = near >= 1 && living.iter().all(|d| d.breed.attack() == Attack::Body);
            if worth && soonest.is_none_or(|t| t.t > cry.total() as f64) && dodges.is_empty() {
                pad.actions.insert(Action::Step);
                self.attack_down = false;
                return pad;
            }
        }

        // Offence: the most open foe (reeling, then recovering), then the nearest; in the vaults those
        // hunting him first, and when none is, the nearest pack asleep (to wake it).
        // (Nothing out of his reach is a target, nor a felled dragon begging to be struck again.)
        let all: Vec<_> = g
            .enemies
            .iter()
            .filter_map(|e| e.duel.as_deref())
            .filter(|d| d.alive() && !d.untouchable() && !d.begging())
            .filter(|d| !self.ignore.contains(&d.id))
            .collect();
        let hunting: Vec<_> = all.iter().copied().filter(|d| d.engaged()).collect();
        // (Between fights only: walking off on an errand mid-fight drags its foes' blows and bolts
        // off their planned frames.)
        let hunting_near = hunting.iter().any(|d| !d.puppet);
        if let Some(errand) = self.general_errand(g, hp, hunting_near) {
            return errand;
        }
        // Between fights (or hurt), what lies near goes into the belt first: a potion, a weapon, a boon.
        let hurt = (g.hero.hp as f64) < g.hero.max_hp as f64 * 0.6;
        if self.skill != Skill::Novice && (hunting.is_empty() || hurt) {
            let near = g.vault.as_ref().and_then(|v| {
                v.drops
                    .iter()
                    .filter(|d| !d.taken && d.landed() && !self.given_up.contains(&d.at))
                    .map(|d| d.at.v())
                    .filter(|p| math::len(math::sub(*p, hp)) < 6.0)
                    .min_by(|a, b| math::len(math::sub(*a, hp)).total_cmp(&math::len(math::sub(*b, hp))))
            });
            if let Some(p) = near {
                let at = P::new(p.x.round() as i32, p.y.round() as i32);
                self.chase = if self.chase.0 == at { (at, self.chase.1 + 1) } else { (at, 0) };
                if self.chase.1 > 1200 {
                    self.given_up.push(at);
                }
                let dir = self.way(g, hp, p);
                pad.left = (dir.x as f32, dir.y as f32);
                return pad;
            }
        }
        let foes = if hunting.is_empty() { all } else { hunting };
        let openness = |d: &crate::logic::combat::Duelist| {
            if Self::belphegor_asleep(d) || d.f.vulnerable() || d.f.reeling().is_some() {
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
        if swing && Self::belphegor_asleep(target) {
            // (Nothing turns it toward a sleeper: it leans into him, to face him as it swings.)
            pad.left = (dir.x as f32 * 0.4, dir.y as f32 * 0.4);
        }
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

impl Bot {
    /// Belphegor asleep or waking (PRINCES.md §4): his hide is open and he mends while he sleeps,
    /// so a player who knows strikes him before anything else (his nightmares keep their distance
    /// to be chased). Only his fight asks it.
    fn belphegor_asleep(d: &crate::logic::combat::Duelist) -> bool {
        d.breed.kind == crate::logic::enemy::EnemyKind::Belphegor && d.lore.slumber.open()
    }

    /// What it does about the prince's cheats (not the novice: it falls for them all), when no foe
    /// hunts it: a whisper in the air, it calls the old man; a mimic chest
    /// near, it walks up to striking distance (no nearer: near enough to trade, or to open the
    /// chest, is too near) and strikes. `None`: nothing to see through.
    fn see_through(&mut self, g: &Game, pad: &mut PadState) -> Option<PadState> {
        if self.skill == Skill::Novice {
            return None;
        }
        if g.enemies.iter().filter_map(|e| e.duel.as_deref()).any(|d| d.engaged()) {
            return None;
        }
        let d = &g.voices.deceit;
        if d.whisper.is_some() {
            pad.actions.insert(Action::Call);
            return Some(pad.clone());
        }
        let hp = g.hero.pos();
        let target = d.mimic.as_ref().filter(|m| !m.done).map(|m| m.at).filter(|p| math::len(math::sub(*p, hp)) < SEE_THROUGH)?;
        let dist = math::len(math::sub(target, hp));
        if dist > STRIKE_AT {
            let dir = self.way(g, hp, target);
            pad.left = (dir.x as f32, dir.y as f32);
            self.attack_down = false;
        } else {
            self.attack_down = !self.attack_down;
            if self.attack_down {
                pad.actions.insert(Action::Attack);
            }
        }
        Some(pad.clone())
    }
}

/// How far off it notices a cheat standing in the world, and how near it walks to strike it.
const SEE_THROUGH: f64 = 14.0;
const STRIKE_AT: f64 = 1.3;

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
    // ...and without the prince's cheats (WARBAND.md §10.3), which would move them too.
    g.voices.deceit.off = true;
    g.progress = crate::logic::progression::Progress::default();
    g.saved_run = None;
    // (And no grudges carried over from the last run's crawl: each run meets its own.)
    g.warband = Default::default();
    g.hero_kind = HeroKind::Ragnar;
    // `VH_MASTERY=N`: every hero's mastery at rank N, to run any harness at that rank (WARBAND.md §11.1).
    let rank = std::env::var("VH_MASTERY").ok().and_then(|v| v.parse::<u8>().ok()).unwrap_or(0);
    for h in g.progress.heroes.iter_mut() {
        h.mastery.rank = rank.min(crate::logic::war::mastery::RANKS);
    }
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
    play_bout_mastered(bout, skill, seed, loadout, 0)
}

/// The same, with his mastery at `rank` (WARBAND.md §11.1; the bots never cry War Cry).
pub fn play_bout_mastered(bout: usize, skill: Skill, seed: u32, loadout: Option<Loadout>, rank: u8) -> BoutResult {
    let mut g = fresh_game(seed);
    g.debug_ring(bout + 1);
    if let Some(l) = loadout {
        // (After the jump, which takes the saved run's hero from the title.)
        l.equip(&mut g);
        g.start_bout();
    }
    if rank > 0 {
        let hero = g.hero_kind;
        g.progress.hero_mut(hero).mastery.rank = rank;
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
    summarize_mastered(bout, skill, seeds, loadout, 0)
}

/// The same, with his mastery at `rank`.
pub fn summarize_mastered(bout: usize, skill: Skill, seeds: &[u32], loadout: Option<Loadout>, rank: u8) -> Summary {
    let mut s = Summary::default();
    let mut win_secs = 0.0;
    for &seed in seeds {
        let r = play_bout_mastered(bout, skill, seed, loadout, rank);
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

/// The bouts the mastery sweep plays: the Brute, the Pack and the Trio.
pub const MASTERY_BOUTS: [usize; 3] = [0, 1, 3];

/**
 * What mastery is worth (WARBAND.md §11.1): every hero at tiers 1 and 4, the novice and the
 * expert, each bout of `MASTERY_BOUTS` at rank 0 and rank 5: wins, time to kill (of the wins) and
 * damage dealt per second of fighting. Mastery rewards skill: the expert's gain should be the
 * larger.
 */
pub fn mastery_sweep(seeds: &[u32]) -> String {
    let mut out = String::from(
        "hero      tier skill   bout                 | rank 0: win  ttk   dps | rank 5: win  ttk   dps | dps gain
",
    );
    for hero in ROSTER {
        for tier in [1, 4] {
            for skill in [Skill::Novice, Skill::Expert] {
                for k in MASTERY_BOUTS {
                    let l = Some(Loadout::all(hero, tier));
                    let (a, b) = (mastery_line(k, skill, seeds, l, 0), mastery_line(k, skill, seeds, l, 5));
                    out += &format!(
                        "{:<9} {:>4} {:<7} {:<20} | {:>10.0}% {:>5.1} {:>5.1} | {:>10.0}% {:>5.1} {:>5.1} | {:>+7.0}%
",
                        hero.name(),
                        tier,
                        skill.name(),
                        gym::BOUTS[k].name,
                        a.0 * 100.0,
                        a.1,
                        a.2,
                        b.0 * 100.0,
                        b.1,
                        b.2,
                        (b.2 / a.2.max(1e-9) - 1.0) * 100.0
                    );
                }
            }
        }
    }
    out
}

/// One bout and skill at a rank: the win rate, the mean time to kill (wins) and the damage dealt
/// per second of fighting (every run).
pub fn mastery_line(bout: usize, skill: Skill, seeds: &[u32], loadout: Option<Loadout>, rank: u8) -> (f64, f64, f64) {
    let (mut wins, mut win_secs, mut dealt, mut secs) = (0, 0.0, 0.0, 0.0);
    for &seed in seeds {
        let r = play_bout_mastered(bout, skill, seed, loadout, rank);
        if r.won {
            wins += 1;
            win_secs += r.secs;
        }
        dealt += r.stats.dealt;
        secs += r.secs;
    }
    let n = seeds.len().max(1) as f64;
    (wins as f64 / n, if wins > 0 { win_secs / wins as f64 } else { 0.0 }, dealt / secs.max(1e-9))
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
    /// The Ledger (§8): rivals met in the last life, and foes made rivals.
    pub rivals_met: u32,
    pub promoted: u32,
}

/**
 * `runs` crawls in a row by `skill` with Ragnar's kit at `tier`, one Ledger carried through them all
 * (WARBAND.md §15.2): how the rivals grow, rank and learn, and how the bot fares against them.
 */
pub fn rivals_report(runs: u32, seed: u32, skill: Skill, tier: u8) -> String {
    use std::fmt::Write;
    let mut ledger = crate::logic::war::ledger::Ledger::default();
    let mut out = String::from("run  cleared  kills  deaths  minutes  met  made  live  slain\n");
    for r in 0..runs {
        let c = crawl_full(
            crate::logic::sins::Sin::Greed,
            skill,
            seed + r,
            Some(Loadout::all(HeroKind::Ragnar, tier)),
            900.0,
            &mut None,
            false,
            Some(&mut ledger),
        )
        .0;
        let _ = writeln!(
            out,
            "{:>3}  {:>7}  {:>2}/{:<2}  {:>6}  {:>7.1}  {:>3}  {:>4}  {:>4}  {:>5}",
            r + 1,
            if c.cleared { "yes" } else { "no" },
            c.kills,
            c.foes,
            c.deaths,
            c.secs / 60.0,
            c.rivals_met,
            c.promoted,
            ledger.live().count(),
            ledger.slain
        );
    }
    out += "\nthe Ledger at the end:\n";
    for r in &ledger.rivals {
        let lesson = r.lesson.map(|l| format!("{l:?}")).unwrap_or_else(|| "-".into());
        let _ = writeln!(
            out,
            "  {:<40} {:<14} rank {}  {}  lesson {:<10} {:?} / {:?} / {:?}  fights {}",
            r.full_name(),
            format!("{:?}", r.kind),
            r.rank,
            if r.alive { "alive" } else { "slain" },
            lesson,
            r.strength,
            r.weakness,
            r.personality,
            r.memory.len()
        );
    }
    out
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
    crawl_full(crate::logic::sins::Sin::Greed, skill, seed, loadout, limit, trace, false, None).0
}

/// How a crawl through a general's realm went (WARBAND.md §15.2): the crawl, his orders, his
/// instruments broken, and whether he was slain.
#[derive(Clone, Copy, Debug, Default)]
pub struct GeneralCrawl {
    pub crawl: Crawl,
    pub tally: super::general::GeneralTally,
    pub broken: usize,
    pub slain: bool,
    pub decree_done: bool,
}

/**
 * A bot crawls the realm of `sin` with its general, as `crawl` does the vaults: until every foe is
 * slain (with `skip_general`, every foe but the general's own pack, which it leaves asleep unless
 * it wakes), its lives run out, or `limit` seconds pass.
 */
pub fn crawl_realm(
    sin: crate::logic::sins::Sin,
    skill: Skill,
    seed: u32,
    loadout: Option<Loadout>,
    limit: f64,
    trace: &mut Option<DrumTrace>,
    skip_general: bool,
) -> (Crawl, GeneralCrawl) {
    crawl_full(sin, skill, seed, loadout, limit, trace, skip_general, None)
}

/// A crawl of `sin`'s realm (see `crawl_realm`), against `ledger`'s rivals if it is given one (and
/// what the crawl made of them written back into it).
#[allow(clippy::too_many_arguments)]
fn crawl_full(
    sin: crate::logic::sins::Sin,
    skill: Skill,
    seed: u32,
    loadout: Option<Loadout>,
    limit: f64,
    trace: &mut Option<DrumTrace>,
    skip_general: bool,
    ledger: Option<&mut crate::logic::war::ledger::Ledger>,
) -> (Crawl, GeneralCrawl) {
    let mut g = fresh_game(seed);
    if let Some(l) = ledger.as_deref() {
        g.warband.save.ledger = l.clone();
    }
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
    g.map_jump(super::save::Stage::Realm(sin));
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
    // (Skipping him, the bot goes round his post: his room's pack is held out of the fight, a
    // stand-in for a player who never sets foot there, while his orders run on.)
    let skipped = if skip_general { g.general_pack() } else { Vec::new() };
    for d in g.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).filter(|d| skipped.contains(&d.id)) {
        d.puppet = true;
    }
    bot.ignore = skipped.clone();
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
        let left = |d: &crate::logic::combat::Duelist| d.alive() && !skipped.contains(&d.id);
        if !g.enemies.iter().any(|e| e.duel.as_deref().is_some_and(left)) {
            out.cleared = true;
            break;
        }
    }
    out.secs = t;
    out.deaths = lives.saturating_sub(g.score.lives) + u32::from(out.game_over);
    out.stats = g.brawl.as_ref().map(|b| b.stats).unwrap_or_default();
    out.kills = out.foes.saturating_sub(g.enemies.iter().filter(|e| e.duel.as_ref().is_some_and(|d| d.alive())).count() as u32);
    if std::env::var_os("VH_CRAWL_LEFT").is_some() {
        let hp = g.hero.pos();
        eprintln!("hero at ({:.1}, {:.1})", hp.x, hp.y);
        let (inst, gate) = g.general_targets();
        eprintln!("  general's instruments standing {inst:?}, glowing gate {gate:?}");
        for d in g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.alive()) {
            eprintln!(
                "  left: {:?} at ({:.1}, {:.1}) hp {:.0}/{:.0} den {:?} state {:?}",
                d.breed.kind, d.pos.x, d.pos.y, d.hp, d.max_hp, d.den, d.f.state
            );
        }
    }
    if let Some(l) = ledger {
        *l = g.warband.save.ledger.clone();
    }
    out.rivals_met = g.war.grudges.met.len() as u32;
    out.promoted = g.war.grudges.promoted;
    let drum = &g.war.drum;
    out.peak_share = drum.peak_share();
    out.peaks = drum.peaks;
    out.routs = g.war.tally.routs;
    out.rallies = g.war.tally.rallies;
    out.rest = if drum.rests.is_empty() { 0.0 } else { drum.rests.iter().sum::<f64>() / drum.rests.len() as f64 };
    let general = g
        .general
        .as_ref()
        .map(|r| GeneralCrawl {
            crawl: out,
            tally: r.tally,
            broken: r.command.broken(),
            slain: r.command.slain,
            decree_done: r.command.decree_done,
        })
        .unwrap_or(GeneralCrawl { crawl: out, ..GeneralCrawl::default() });
    (out, general)
}

/// `--general SIN SKILL TIER SEED`: one realm with its general, the bot taking him on and then
/// leaving him be, and what his orders came to in each (WARBAND.md §15.2).
pub fn general_report(sin: crate::logic::sins::Sin, skill: Skill, tier: u8, seed: u32, limit: f64) -> String {
    let kit = Some(Loadout::all(HeroKind::Ragnar, tier));
    let mut out = format!(
        "{} - {} ({}), {} with Ragnar's tier-{tier} kit, seed {seed}
",
        sin.realm(),
        crate::logic::war::general::General::of(sin).name,
        sin.title(),
        skill.name()
    );
    out += "general  cleared  minutes  deaths  kills/foes  announced  bit  failed  answered  broken  barred  turned  bursts  burned  whistles  decrees  slain-at  overlaps
";
    let mut times = [0.0; 2];
    for (k, skip) in [false, true].into_iter().enumerate() {
        let (c, r) = crawl_realm(sin, skill, seed, kit, limit, &mut None, skip);
        times[k] = c.secs;
        out += &format!(
            "{:<8} {:>7} {:>8.1} {:>7} {:>5}/{:<5} {:>10} {:>4} {:>7} {:>9} {:>4}/3 {:>7} {:>7} {:>7} {:>7} {:>9} {:>8} {:>9} {:>9}
",
            if skip { "alive" } else { "fought" },
            if c.cleared { "yes" } else { "no" },
            c.secs / 60.0,
            c.deaths,
            c.kills,
            c.foes,
            r.tally.announced,
            r.tally.bitten,
            r.tally.failed,
            r.tally.answered,
            r.broken,
            r.tally.barred,
            r.tally.turned,
            r.tally.bursts,
            r.tally.burned,
            r.tally.whistles,
            r.tally.decrees,
            r.tally.slain_at.map(|t| format!("{:.0}s", t)).unwrap_or_else(|| if r.slain { "yes".into() } else { "-".into() }),
            c.stats.overlaps
        );
    }
    let diff = (times[1] - times[0]) / times[0].max(1.0) * 100.0;
    out += &format!(
        "time with him alive against slain: {:+.0}% (target within 25%)
",
        diff
    );
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

/// A bot fights the prince of `sin` in his court (Greed's: the Counting House) until he falls, its
/// lives run out, or `limit` seconds pass. (A court not yet built is the Cyclops' meadow: no prince
/// stands there to fall.)
pub fn court_fight(sin: crate::logic::sins::Sin, skill: Skill, seed: u32, loadout: Option<Loadout>, limit: f64) -> CourtFight {
    let mut g = fresh_game(seed);
    g.map_jump(super::save::Stage::Court(sin));
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
