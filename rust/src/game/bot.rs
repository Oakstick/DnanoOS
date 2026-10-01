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
use crate::logic::combat::{math, Phase, State};
use crate::logic::gym;
use crate::logic::rng::Rng;
use crate::logic::types::V;
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
    rng: Rng,
    /// The reaction delay drawn for each foe's current move (by foe id and the move's start frame).
    noticed: HashMap<u32, (u32, u16)>,
    /// The attack button was down last update (mashing toggles it).
    attack_down: bool,
    /// Frames the guard stays up after the last strike it was raised for.
    guard_hold: u16,
    /// The expert's timing error for the strike it is waiting on.
    lead: f64,
}

impl Bot {
    pub fn new(skill: Skill, seed: u32) -> Self {
        Self {
            skill,
            rng: Rng::new(seed.wrapping_mul(2654435761).wrapping_add(7)),
            noticed: HashMap::new(),
            attack_down: false,
            guard_hold: 0,
            lead: PARRY_LEAD,
        }
    }

    /// The strikes it has noticed, soonest first.
    fn threats(&mut self, g: &Game) -> Vec<Threat> {
        let Some(spar) = g.spar.as_ref() else { return Vec::new() };
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

    /// This update's stick and buttons.
    pub fn act(&mut self, g: &Game) -> PadState {
        let mut pad = PadState::default();
        let Some(spar) = g.spar.as_ref() else { return pad };
        if spar.phase != BoutPhase::Fight {
            self.attack_down = false;
            return pad;
        }
        let hp = g.hero.pos();
        let threats = self.threats(g);
        let soonest = threats.first().copied();

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

        // Offence: the most open foe (reeling, then recovering), then the nearest.
        let foes: Vec<_> = g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.alive()).collect();
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
        let dir = math::norm(to);
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

/// Plays bout `bout` (0-based) with `skill`, seeded with `seed`, from a fresh game on a clean save.
pub fn play_bout(bout: usize, skill: Skill, seed: u32) -> BoutResult {
    let mut g = Game::new();
    g.reseed(seed);
    g.debug_ring(bout + 1);
    g.hero.free_move = true;
    let mut bot = Bot::new(skill, seed);
    let step = crate::app::STEP;
    let steps = ((LIMIT + super::sparring::COUNTDOWN + 1.0) / step) as usize;
    for _ in 0..steps {
        let pad = bot.act(&g);
        g.input.set_pad(Some(pad));
        g.update(step);
        let spar = g.spar.as_ref().unwrap();
        match spar.phase {
            BoutPhase::Won(_) | BoutPhase::Lost(_) => {
                let won = matches!(spar.phase, BoutPhase::Won(_));
                return BoutResult { won, secs: spar.stats.frames as f64 / 60.0, stats: spar.stats };
            }
            _ => {}
        }
    }
    let spar = g.spar.as_ref().unwrap();
    BoutResult { won: false, secs: spar.stats.frames as f64 / 60.0, stats: spar.stats }
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
    let mut s = Summary::default();
    let mut win_secs = 0.0;
    for &seed in seeds {
        let r = play_bout(bout, skill, seed);
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
