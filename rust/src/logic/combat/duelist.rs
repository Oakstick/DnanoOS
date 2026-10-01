//! A foe's mind in the Sparring Ring: where it walks, which way it faces, and when it wants to
//! strike. It moves freely (not tile to tile), keeps its distance from the others and the
//! hero, strafes to the sector the threat manager gave it while it waits, and closes in only
//! with a token. Its moves run on its own `Fighter`.

use super::archetype::{Archetype, FLEE, HEX_FAR, HEX_NEAR, LUNGE_FAR, LUNGE_NEAR, SWEEP_NEAR};
use super::frames::{MoveId, Phase, BOLT_RADIUS, BOLT_SPEED};
use super::fsm::{Event, Fighter, State};
use super::math::{add, dot, len, norm, scale, sub, turn_toward};
use super::threat::Slot;
use super::FPS;
use crate::logic::types::V;

/// The hero's body radius, for spacing and hitboxes.
pub const HERO_RADIUS: f64 = 0.3;
/// Knockback a flinch gives, in tiles per second, and how much of it each frame keeps.
pub const KNOCKBACK: f64 = 3.0;
pub const KNOCK_KEEP: f64 = 0.82;
/// Frames the Skirmisher springs back after its lunge.
pub const BACKSTEP: u16 = 30;

/// A hex bolt in flight (a parried one flies back at the foes).
#[derive(Clone, Copy, Debug)]
pub struct Bolt {
    pub pos: V,
    /// Tiles per second.
    pub vel: V,
    /// Frames left.
    pub life: u16,
    pub damage: f64,
    pub reflected: bool,
    /// It has reached the hero (struck, been blocked, or passed under his leap): it strikes once.
    pub touched: bool,
}

/// What a foe knows of the hero this frame.
#[derive(Clone, Copy, Debug)]
pub struct Sense {
    pub hero: V,
    /// Which way he faces (unit).
    pub aim: V,
}

/// The ground it fights on: no foe leaves `lo`..`hi`.
#[derive(Clone, Copy, Debug)]
pub struct Arena {
    pub lo: V,
    pub hi: V,
}

#[derive(Clone, Debug)]
pub struct Duelist {
    pub id: u32,
    pub arch: Archetype,
    pub f: Fighter,
    pub hp: f64,
    pub pos: V,
    /// Which way it faces (unit).
    pub facing: V,
    /// Its velocity last frame, tiles per second (for its gait).
    pub vel: V,
    /// It holds an attack token.
    pub token: bool,
    /// Frames it has waited for one.
    pub wait: u16,
    /// Frames before it may ask again.
    pub rest: u16,
    /// The aim of its move, committed once its tracking frames are over.
    pub aim: V,
    /// Its move has met the hero (landed, blocked, parried or passed through his evade): no second hit.
    pub struck: bool,
    /// Where it waits.
    pub slot: Option<Slot>,
    /// Frames of springing back (the Skirmisher, after a lunge).
    pub backstep: u16,
    /// Knockback velocity, fading.
    pub knock: V,
    /// Frames of the white flash of a blow.
    pub flash: u16,
}

impl Duelist {
    pub fn new(id: u32, arch: Archetype, pos: V, facing: V) -> Self {
        Self {
            id,
            arch,
            f: arch.fighter(),
            hp: arch.stats().hp,
            pos,
            facing: norm(facing),
            vel: V::new(0.0, 0.0),
            token: false,
            wait: 0,
            rest: 30,
            aim: norm(facing),
            struck: false,
            slot: None,
            backstep: 0,
            knock: V::new(0.0, 0.0),
            flash: 0,
        }
    }

    pub fn alive(&self) -> bool {
        !self.f.is_dead()
    }

    pub fn radius(&self) -> f64 {
        self.arch.stats().radius
    }

    /// How far the hero stands off its facing, in radians.
    fn off(&self, s: &Sense) -> f64 {
        let to = norm(sub(s.hero, self.pos));
        dot(to, self.facing).clamp(-1.0, 1.0).acos()
    }

    /// The move it would use now, from where it stands and the way it faces.
    pub fn choose(&self, s: &Sense) -> Option<MoveId> {
        let d = len(sub(s.hero, self.pos));
        let off = self.off(s).to_degrees();
        match self.arch {
            Archetype::Brute => {
                if d <= 1.7 && off <= 60.0 {
                    Some(MoveId::Slam)
                } else if d <= SWEEP_NEAR + HERO_RADIUS && off > 60.0 {
                    Some(MoveId::Sweep)
                } else {
                    None
                }
            }
            Archetype::Skirmisher => ((LUNGE_NEAR..=LUNGE_FAR).contains(&d) && off <= 30.0).then_some(MoveId::Lunge),
            Archetype::Hexer => ((HEX_NEAR..=HEX_FAR).contains(&d) && off <= 45.0).then_some(MoveId::Hex),
        }
    }

    /// Frames until its strike would land if it set out now: the walk into range, then its startup.
    pub fn eta(&self, s: &Sense) -> u32 {
        let d = len(sub(s.hero, self.pos));
        let range = self.arch.range();
        let walk = ((d - range).max(0.0) / self.arch.stats().walk * FPS).ceil() as u32;
        walk + lands_after(self.arch.moves()[0], d.min(range)) as u32
    }

    /// Frames from beginning `mv` now until it reaches the hero: its startup, and a bolt's flight.
    pub fn lands_in(&self, mv: MoveId, hero: V) -> u16 {
        lands_after(mv, len(sub(hero, self.pos)))
    }

    /// Begins `mv`: its aim starts where it faces.
    pub fn begin(&mut self, mv: MoveId) -> bool {
        if !self.f.begin(mv) {
            return false;
        }
        self.aim = self.facing;
        self.struck = false;
        self.wait = 0;
        true
    }

    /// Its fighter's frame, plus what the end of a move means for it.
    pub fn step(&mut self) -> Vec<Event> {
        self.flash = self.flash.saturating_sub(1);
        self.rest = self.rest.saturating_sub(1);
        let ev = self.f.step();
        for e in &ev {
            if let Event::Ended(mv) = e {
                self.rest = self.arch.stats().rest;
                if *mv == MoveId::Lunge {
                    self.backstep = BACKSTEP;
                }
            }
        }
        ev
    }

    /**
     * One frame of thought and motion. Tracks the hero through its startup and travels through a
     * lunge's active frames; free, it backs off, closes in (with a token), or strafes to its
     * slot. Returns the move it wants to begin now (the threat manager's commit gate decides).
     */
    pub fn think(&mut self, s: &Sense, arena: &Arena, others: &[(V, f64)]) -> Option<MoveId> {
        if !self.alive() {
            return None;
        }
        let st = self.arch.stats();
        let turn = st.turn / FPS;
        let to = sub(s.hero, self.pos);
        let d = len(to);
        let dir = norm(to);
        let mut v = V::new(0.0, 0.0);
        let mut want = None;
        match self.f.state {
            State::Attack { mv, frame } => {
                let data = mv.data();
                if frame < data.track {
                    self.facing = turn_toward(self.facing, dir, turn);
                    self.aim = self.facing;
                }
                // A lunge carries it along its aim, up to the hero and no farther.
                if data.phase(frame) == Phase::Active && data.travel > 0.0 {
                    let mut step = data.travel / data.active as f64;
                    if dot(self.aim, dir) > 0.0 {
                        step = step.min((d - st.radius - HERO_RADIUS).max(0.0));
                    }
                    v = scale(self.aim, step * FPS);
                }
            }
            State::Neutral => {
                self.facing = turn_toward(self.facing, dir, turn);
                if self.arch == Archetype::Hexer && d < FLEE {
                    v = self.flee(dir, st.retreat, arena);
                } else if self.backstep > 0 {
                    self.backstep -= 1;
                    v = scale(dir, -st.retreat);
                } else if self.token {
                    if self.arch == Archetype::Skirmisher && d < LUNGE_NEAR {
                        v = scale(dir, -st.retreat); // room to spring
                    } else if let Some(mv) = self.choose(s) {
                        want = Some(mv);
                    } else {
                        v = scale(dir, st.walk);
                    }
                } else {
                    self.wait = self.wait.saturating_add(1);
                    if let Some(slot) = self.slot {
                        let to_slot = sub(slot.target, self.pos);
                        let gap = len(to_slot);
                        if gap > 0.25 {
                            v = scale(norm(to_slot), st.strafe.min(gap * 4.0));
                        }
                    }
                }
            }
            _ => {}
        }
        // Keep apart from the others (one in a move holds its ground), and out of the hero.
        let others = if matches!(self.f.state, State::Attack { .. }) { &[][..] } else { others };
        for &(p, r) in others {
            let away = sub(self.pos, p);
            let gap = len(away) - (r + st.radius + 0.15);
            if gap < 0.0 {
                v = add(v, scale(norm(away), -gap * 8.0));
            }
        }
        let room = d - (st.radius + HERO_RADIUS + 0.1);
        if room < 0.0 && !matches!(self.f.state, State::Attack { .. }) {
            v = add(v, scale(dir, room * 8.0));
        }
        v = add(v, self.knock);
        self.knock = scale(self.knock, KNOCK_KEEP);
        self.vel = v;
        self.pos = add(self.pos, scale(v, 1.0 / FPS));
        self.pos = V::new(
            self.pos.x.clamp(arena.lo.x + st.radius, arena.hi.x - st.radius),
            self.pos.y.clamp(arena.lo.y + st.radius, arena.hi.y - st.radius),
        );
        want
    }

    /// Backing away from the hero; along the wall when the wall is behind.
    fn flee(&self, dir: V, speed: f64, arena: &Arena) -> V {
        let away = scale(dir, -1.0);
        let ahead = add(self.pos, scale(away, 1.5));
        let inside = ahead.x > arena.lo.x + 1.0 && ahead.x < arena.hi.x - 1.0 && ahead.y > arena.lo.y + 1.0 && ahead.y < arena.hi.y - 1.0;
        if inside {
            return scale(away, speed);
        }
        // Cornered: slip sideways, toward the roomier side.
        let side = V::new(-dir.y, dir.x);
        let c = V::new((arena.lo.x + arena.hi.x) / 2.0, (arena.lo.y + arena.hi.y) / 2.0);
        let toward_centre = dot(side, sub(c, self.pos)) >= 0.0;
        scale(if toward_centre { side } else { scale(side, -1.0) }, speed)
    }

    /// A blow landed on it: the white flash, and a shove away from `from` if it flinched.
    pub fn shove(&mut self, from: V, flinched: bool) {
        self.flash = 6;
        if flinched {
            let k = if self.arch == Archetype::Brute { 0.4 } else { 1.0 };
            self.knock = scale(norm(sub(self.pos, from)), KNOCKBACK * k);
        }
    }
}

/// Frames from beginning `mv` at `dist` tiles from the hero until it reaches him: its startup, and
/// for a bolt (loosed half a tile ahead) the frames of flight until it touches him.
pub fn lands_after(mv: MoveId, dist: f64) -> u16 {
    let d = mv.data();
    if !d.projectile {
        return d.startup;
    }
    let travel = (dist - 0.5 - BOLT_RADIUS - HERO_RADIUS).max(0.0);
    d.startup + ((travel / BOLT_SPEED * FPS).ceil() as u16).saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arena() -> Arena {
        Arena { lo: V::new(0.0, 0.0), hi: V::new(16.0, 16.0) }
    }

    #[test]
    fn with_a_token_the_skirmisher_closes_in_and_lunges_from_its_range() {
        let mut d = Duelist::new(1, Archetype::Skirmisher, V::new(2.0, 8.0), V::new(1.0, 0.0));
        let s = Sense { hero: V::new(12.0, 8.0), aim: V::new(-1.0, 0.0) };
        assert_eq!(d.think(&s, &arena(), &[]), None);
        assert_eq!(d.pos.x, 2.0, "no token, no slot: it holds");
        d.token = true;
        let mut wanted = None;
        for _ in 0..300 {
            d.step();
            if let Some(mv) = d.think(&s, &arena(), &[]) {
                wanted = Some(mv);
                break;
            }
        }
        assert_eq!(wanted, Some(MoveId::Lunge));
        let gap = 12.0 - d.pos.x;
        assert!((LUNGE_NEAR..=LUNGE_FAR + 1e-9).contains(&gap), "{gap}");
        // The lunge carries it 2.6 tiles over its active frames (with the hero out of its way).
        let far = Sense { hero: V::new(15.5, 8.0), aim: V::new(-1.0, 0.0) };
        assert!(d.begin(MoveId::Lunge));
        let x0 = d.pos.x;
        let data = MoveId::Lunge.data();
        for _ in 0..data.total() - 1 {
            d.step();
            d.think(&far, &arena(), &[]);
        }
        assert!((d.pos.x - x0 - data.travel).abs() < 0.01, "{} tiles", d.pos.x - x0);
        d.step();
        assert!(d.backstep > 0, "and it springs back");
        // Into the hero, it stops at his body.
        let mut d = Duelist::new(1, Archetype::Skirmisher, V::new(10.0, 8.0), V::new(1.0, 0.0));
        d.begin(MoveId::Lunge);
        for _ in 0..data.total() - 1 {
            d.step();
            d.think(&s, &arena(), &[]);
        }
        assert!(d.pos.x < 12.0 - 0.6, "{}", d.pos.x);
    }

    #[test]
    fn a_committed_slam_no_longer_follows_the_hero() {
        let mut d = Duelist::new(1, Archetype::Brute, V::new(8.0, 8.0), V::new(1.0, 0.0));
        let mut s = Sense { hero: V::new(9.5, 8.0), aim: V::new(-1.0, 0.0) };
        assert_eq!(d.choose(&s), Some(MoveId::Slam));
        d.begin(MoveId::Slam);
        let track = MoveId::Slam.data().track;
        for _ in 0..track {
            d.step();
            d.think(&s, &arena(), &[]);
        }
        let aim = d.aim;
        s.hero = V::new(8.0, 9.5); // he steps round to its side
        for _ in 0..10 {
            d.step();
            d.think(&s, &arena(), &[]);
        }
        assert_eq!((d.aim.x, d.aim.y), (aim.x, aim.y), "the aim is committed");
        // Flanked and close, it sweeps instead.
        let fresh = Duelist::new(2, Archetype::Brute, V::new(8.0, 8.0), V::new(1.0, 0.0));
        assert_eq!(fresh.choose(&s), Some(MoveId::Sweep));
    }

    #[test]
    fn the_hexer_backs_away_from_a_hero_who_comes_close_and_slips_along_a_wall() {
        let mut d = Duelist::new(1, Archetype::Hexer, V::new(8.0, 8.0), V::new(1.0, 0.0));
        let s = Sense { hero: V::new(10.0, 8.0), aim: V::new(-1.0, 0.0) };
        d.think(&s, &arena(), &[]);
        assert!(d.pos.x < 8.0);
        let mut d = Duelist::new(1, Archetype::Hexer, V::new(0.6, 8.0), V::new(1.0, 0.0));
        let s = Sense { hero: V::new(2.5, 8.0), aim: V::new(-1.0, 0.0) };
        for _ in 0..20 {
            d.think(&s, &arena(), &[]);
        }
        assert!((d.pos.y - 8.0).abs() > 0.5, "cornered, it slips sideways");
    }

    #[test]
    fn waiting_foes_keep_their_distance_from_each_other() {
        let mut a = Duelist::new(1, Archetype::Skirmisher, V::new(8.0, 8.0), V::new(1.0, 0.0));
        let s = Sense { hero: V::new(14.0, 14.0), aim: V::new(-1.0, 0.0) };
        a.think(&s, &arena(), &[(V::new(8.2, 8.0), 0.38)]);
        assert!(a.pos.x < 8.0, "pushed apart");
    }
}
