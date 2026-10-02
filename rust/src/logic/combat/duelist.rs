//! A foe's mind under the frame-data rules: where it walks, which way it faces, and when it wants
//! to strike. It moves freely (not tile to tile) over its `Terrain` (the Sparring Ring's square,
//! or the Vaults' rooms and corridors, round whose walls it finds its way), keeps its distance from
//! the others and the hero, strafes to the sector the threat manager gave it while it waits, and
//! closes in only with a token. A foe with a den idles there until it is woken, and goes home when
//! the hero is long gone. Its moves run on its own `Fighter`, its numbers on its `Breed`.

use super::archetype::{Archetype, Breed, FLEE, HEX_FAR, HEX_NEAR, LUNGE_FAR, LUNGE_NEAR, SWEEP_NEAR};
use super::frames::{MoveId, Phase, BOLT_RADIUS, BOLT_SPEED};
use super::fsm::{Event, Fighter, State};
use super::math::{add, dot, len, norm, scale, sub, turn_toward};
use super::threat::Slot;
use super::FPS;
use crate::logic::grid::Grid;
use crate::logic::pathing::route_toward;
use crate::logic::types::{CARDINALS, P, V};
use std::collections::VecDeque;

/// The hero's body radius, for spacing and hitboxes.
pub const HERO_RADIUS: f64 = 0.3;
/// Knockback a flinch gives, in tiles per second, and how much of it each frame keeps.
pub const KNOCKBACK: f64 = 3.0;
pub const KNOCK_KEEP: f64 = 0.82;
/// Frames the Skirmisher springs back after its lunge.
pub const BACKSTEP: u16 = 30;
/// The Skirmisher's sidestep: seen a swing begin within this reach, it hops back this many frames
/// at this pace, then cannot again for `SIDESTEP_REST` frames. A mashed first swing whiffs, and
/// its lunge comes into the recovery.
pub const SIDESTEP_RANGE: f64 = 2.4;
pub const SIDESTEP: u16 = 12;
pub const SIDESTEP_SPEED: f64 = 8.0;
pub const SIDESTEP_REST: u16 = 120;
/// A hex bolt's colours: the witch's violet, the bog slime's green.
pub const HEX_TINT: u32 = 0xc070ff;
pub const SPIT_TINT: u32 = 0x7aff5a;

/// A hex bolt in flight (a parried one flies back at the foes).
#[derive(Clone, Copy, Debug)]
pub struct Bolt {
    pub pos: V,
    /// Tiles per second.
    pub vel: V,
    /// Frames left.
    pub life: u16,
    pub damage: f64,
    /// Its poise damage, once it flies at the foes (thrown by him, or parried back).
    pub poise: f64,
    pub reflected: bool,
    /// It has reached the hero (struck, been blocked, or passed under his leap): it strikes once.
    pub touched: bool,
    /// The frame it is due to reach him, as the threat manager planned it: it flies straight, and
    /// paces itself to arrive then (a hero walking into it does not bring it sooner).
    pub due: Option<u32>,
    /// Its colour.
    pub tint: u32,
}

/// What a foe knows of the hero this frame.
#[derive(Clone, Copy, Debug)]
pub struct Sense {
    pub hero: V,
    /// Which way he faces (unit).
    pub aim: V,
    /// He is winding up a swing.
    pub swing: bool,
}

/// The ground a foe fights on.
pub trait Terrain {
    /// Where a body of `radius` at `from` ends up after moving by `step`, sliding along what it cannot cross.
    fn slide(&self, from: V, step: V, radius: f64) -> V;
    /// The point to head for on the way from `from` to `to`: `to` itself while the way is clear.
    fn toward(&self, from: V, to: V) -> V;
    /// Is the straight way from `a` to `b` clear (for a lunge, a bolt)?
    fn clear(&self, a: V, b: V) -> bool;
    /// Is there room to stand at `p`?
    fn open(&self, p: V) -> bool;
    /// The middle of the ground, where a cornered foe slips toward (none: it picks the roomier side).
    fn centre(&self) -> Option<V> {
        None
    }
}

/// The Sparring Ring's square: no foe leaves `lo`..`hi`, and nothing stands in the way.
#[derive(Clone, Copy, Debug)]
pub struct Arena {
    pub lo: V,
    pub hi: V,
}

impl Terrain for Arena {
    fn slide(&self, from: V, step: V, radius: f64) -> V {
        let p = add(from, step);
        V::new(p.x.clamp(self.lo.x + radius, self.hi.x - radius), p.y.clamp(self.lo.y + radius, self.hi.y - radius))
    }
    fn toward(&self, _from: V, to: V) -> V {
        to
    }
    fn clear(&self, _a: V, _b: V) -> bool {
        true
    }
    fn open(&self, p: V) -> bool {
        p.x > self.lo.x + 1.0 && p.x < self.hi.x - 1.0 && p.y > self.lo.y + 1.0 && p.y < self.hi.y - 1.0
    }
    fn centre(&self) -> Option<V> {
        Some(V::new((self.lo.x + self.hi.x) / 2.0, (self.lo.y + self.hi.y) / 2.0))
    }
}

/// Walking distances over a grid from one tile (the hero's), out to a reach: the way the hunt
/// goes round walls, drawn once for every foe each time he steps onto a new tile.
#[derive(Clone, Debug, Default)]
pub struct Flow {
    pub from: P,
    w: i32,
    dist: Vec<u16>,
}

impl Flow {
    pub fn new(grid: &Grid, from: P, reach: u16) -> Self {
        let (w, h) = (grid.width, grid.height);
        let mut dist = vec![u16::MAX; (w * h).max(0) as usize];
        let idx = |p: P| (p.y * w + p.x) as usize;
        if from.x < 0 || from.y < 0 || from.x >= w || from.y >= h {
            return Self { from, w, dist };
        }
        dist[idx(from)] = 0;
        let mut queue = VecDeque::from([from]);
        while let Some(p) = queue.pop_front() {
            let d = dist[idx(p)];
            if d >= reach {
                continue;
            }
            for dir in CARDINALS {
                let (dx, dy) = dir.vec();
                let n = P::new(p.x + dx, p.y + dy);
                if n.x < 0 || n.y < 0 || n.x >= w || n.y >= h || !grid.is_walkable(n.x, n.y, false) || dist[idx(n)] != u16::MAX {
                    continue;
                }
                dist[idx(n)] = d + 1;
                queue.push_back(n);
            }
        }
        Self { from, w, dist }
    }

    /// Steps from `p` to the hero's tile (none: out of reach).
    pub fn dist(&self, p: P) -> u16 {
        if p.x < 0 || p.y < 0 || p.x >= self.w {
            return u16::MAX;
        }
        self.dist.get((p.y * self.w + p.x) as usize).copied().unwrap_or(u16::MAX)
    }

    /// The neighbour of `p` one step nearer the hero.
    pub fn step(&self, p: P) -> Option<P> {
        let here = self.dist(p);
        CARDINALS
            .iter()
            .map(|d| {
                let (dx, dy) = d.vec();
                P::new(p.x + dx, p.y + dy)
            })
            .filter(|&n| self.dist(n) < here)
            .min_by_key(|&n| self.dist(n))
    }
}

/// The Vaults: tiles, walls between them, and the hero's flow field.
pub struct GridTerrain<'a> {
    pub grid: &'a Grid,
    pub flow: &'a Flow,
}

fn tile(p: V) -> P {
    P::new(p.x.round() as i32, p.y.round() as i32)
}

impl GridTerrain<'_> {
    /// Can a body of `radius` stand at `p` (every tile its square touches walkable)?
    fn fits(&self, p: V, radius: f64) -> bool {
        let r = radius * 0.9;
        [(-r, -r), (r, -r), (-r, r), (r, r)].iter().all(|&(dx, dy)| {
            let t = tile(V::new(p.x + dx, p.y + dy));
            self.grid.is_walkable(t.x, t.y, false)
        })
    }
}

impl Terrain for GridTerrain<'_> {
    fn slide(&self, from: V, step: V, radius: f64) -> V {
        let full = add(from, step);
        if self.fits(full, radius) {
            return full;
        }
        let x = V::new(from.x + step.x, from.y);
        if self.fits(x, radius) {
            return x;
        }
        let y = V::new(from.x, from.y + step.y);
        if self.fits(y, radius) {
            return y;
        }
        from
    }

    fn toward(&self, from: V, to: V) -> V {
        if self.clear(from, to) {
            return to;
        }
        let here = tile(from);
        // Near the hero, his flow field knows the way; farther afield, a search does.
        if self.flow.dist(tile(to)) <= 4 {
            if let Some(n) = self.flow.step(here) {
                return n.v();
            }
        }
        match route_toward(self.grid, here, to, false, false, false) {
            Some(d) => {
                let (dx, dy) = d.vec();
                P::new(here.x + dx, here.y + dy).v()
            }
            None => to,
        }
    }

    fn clear(&self, a: V, b: V) -> bool {
        let n = (len(sub(b, a)) / 0.3).ceil().max(1.0) as usize;
        (0..=n).all(|k| self.fits(add(a, scale(sub(b, a), k as f64 / n as f64)), 0.25))
    }

    fn open(&self, p: V) -> bool {
        self.fits(p, 0.4)
    }
}

/// A foe's room in the Vaults: its bounds, its post, and whether it is awake (hunting) or going home.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Den {
    pub lo: V,
    pub hi: V,
    pub home: V,
    pub awake: bool,
    pub returning: bool,
}

impl Den {
    pub fn contains(&self, p: V) -> bool {
        p.x >= self.lo.x && p.x <= self.hi.x && p.y >= self.lo.y && p.y <= self.hi.y
    }
    pub fn centre(&self) -> V {
        V::new((self.lo.x + self.hi.x) / 2.0, (self.lo.y + self.hi.y) / 2.0)
    }
}

#[derive(Clone, Debug)]
pub struct Duelist {
    pub id: u32,
    pub arch: Archetype,
    /// Its body: the archetype's numbers scaled.
    pub breed: Breed,
    pub f: Fighter,
    /// Its health, and what it is when whole (its breed's, times the realm's weight).
    pub hp: f64,
    pub max_hp: f64,
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
    /// Frames left of a sidestep, and before the next may come.
    pub sidestep: u16,
    pub sidestep_rest: u16,
    /// Its room in the Vaults (none in the ring: always in the fight).
    pub den: Option<Den>,
}

impl Duelist {
    /// One of the archetype's own bodies (the ring's foes).
    pub fn new(id: u32, arch: Archetype, pos: V, facing: V) -> Self {
        Self::bred(id, Breed::of(arch.kind()), pos, facing)
    }

    /// A foe of `breed`.
    pub fn bred(id: u32, breed: Breed, pos: V, facing: V) -> Self {
        Self {
            id,
            arch: breed.arch,
            breed,
            f: breed.fighter(),
            hp: breed.stats().hp,
            max_hp: breed.stats().hp,
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
            sidestep: 0,
            sidestep_rest: 0,
            den: None,
        }
    }

    pub fn alive(&self) -> bool {
        !self.f.is_dead()
    }

    /// Its archetype's numbers as its body has them.
    pub fn stats(&self) -> super::archetype::Stats {
        self.breed.stats()
    }

    pub fn radius(&self) -> f64 {
        self.breed.radius
    }

    /// In the fight: no den, or woken from it and not gone home.
    pub fn engaged(&self) -> bool {
        self.alive() && self.den.is_none_or(|d| d.awake && !d.returning)
    }

    /// Back at its post, whole, idling (the hero fell and the vaults begin again).
    pub fn reset_home(&mut self) {
        if let Some(d) = self.den.as_mut() {
            self.pos = d.home;
            d.awake = false;
            d.returning = false;
        }
        self.f = self.breed.fighter();
        self.hp = self.max_hp;
        self.token = false;
        self.slot = None;
        self.backstep = 0;
        self.sidestep = 0;
        self.knock = V::new(0.0, 0.0);
        self.vel = V::new(0.0, 0.0);
        self.rest = 30;
    }

    /// How far the hero stands off its facing, in radians.
    fn off(&self, s: &Sense) -> f64 {
        let to = norm(sub(s.hero, self.pos));
        dot(to, self.facing).clamp(-1.0, 1.0).acos()
    }

    /// The move it would use now, from where it stands and the way it faces (a lunge and a bolt want a clear line).
    pub fn choose(&self, s: &Sense, terrain: &dyn Terrain) -> Option<MoveId> {
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
            Archetype::Skirmisher => {
                ((LUNGE_NEAR..=LUNGE_FAR).contains(&d) && off <= 30.0 && terrain.clear(self.pos, s.hero)).then_some(MoveId::Lunge)
            }
            Archetype::Hexer => {
                ((HEX_NEAR..=HEX_FAR).contains(&d) && off <= 45.0 && terrain.clear(self.pos, s.hero)).then_some(MoveId::Hex)
            }
        }
    }

    /// Frames until its strike would land if it set out now: the walk into range, then its startup.
    pub fn eta(&self, s: &Sense) -> u32 {
        let d = len(sub(s.hero, self.pos));
        let range = self.arch.range();
        let walk = ((d - range).max(0.0) / self.stats().walk * FPS).ceil() as u32;
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
        self.sidestep_rest = self.sidestep_rest.saturating_sub(1);
        let ev = self.f.step();
        for e in &ev {
            if let Event::Ended(mv) = e {
                self.rest = self.stats().rest;
                if *mv == MoveId::Lunge {
                    self.backstep = BACKSTEP;
                }
            }
        }
        ev
    }

    /**
     * One frame of thought and motion. Asleep in its den it stands; going home it walks there.
     * In the fight it tracks the hero through its startup and travels through a lunge's active
     * frames; free, it backs off, closes in (with a token, round any wall), or strafes to its slot.
     * Returns the move it wants to begin now (the threat manager's commit gate decides).
     */
    pub fn think(&mut self, s: &Sense, terrain: &dyn Terrain, others: &[(V, f64)]) -> Option<MoveId> {
        if !self.alive() {
            return None;
        }
        let st = self.stats();
        if let Some(den) = self.den {
            if !den.awake {
                self.vel = V::new(0.0, 0.0);
                return None;
            }
            if den.returning {
                let to = sub(den.home, self.pos);
                if len(to) < 0.3 {
                    self.den = Some(Den { awake: false, returning: false, ..den });
                    self.vel = V::new(0.0, 0.0);
                    return None;
                }
                let way = norm(sub(terrain.toward(self.pos, den.home), self.pos));
                self.facing = turn_toward(self.facing, way, st.turn / FPS);
                self.vel = scale(way, st.walk);
                self.pos = terrain.slide(self.pos, scale(self.vel, 1.0 / FPS), st.radius);
                return None;
            }
        }
        let turn = st.turn / FPS;
        let to = sub(s.hero, self.pos);
        let d = len(to);
        let dir = norm(to);
        // The way to him (or to its slot), round whatever stands between.
        let way_to = |p: V, here: V| norm(sub(terrain.toward(here, p), here));
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
                if self.arch == Archetype::Skirmisher && s.swing && d <= SIDESTEP_RANGE && self.sidestep_rest == 0 {
                    self.sidestep = SIDESTEP;
                    self.sidestep_rest = SIDESTEP_REST;
                }
                if self.sidestep > 0 {
                    self.sidestep -= 1;
                    v = scale(dir, -SIDESTEP_SPEED);
                } else if self.arch == Archetype::Hexer && d < FLEE {
                    v = self.flee(dir, st.retreat, terrain);
                } else if self.backstep > 0 {
                    self.backstep -= 1;
                    v = scale(dir, -st.retreat);
                } else if self.token {
                    if self.arch == Archetype::Skirmisher && d < LUNGE_NEAR {
                        v = scale(dir, -st.retreat); // room to spring
                    } else if let Some(mv) = self.choose(s, terrain) {
                        want = Some(mv);
                    } else {
                        v = scale(way_to(s.hero, self.pos), st.walk);
                    }
                } else {
                    self.wait = self.wait.saturating_add(1);
                    if let Some(slot) = self.slot {
                        let gap = len(sub(slot.target, self.pos));
                        if gap > 0.25 {
                            v = scale(way_to(slot.target, self.pos), st.strafe.min(gap * 4.0));
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
        self.pos = terrain.slide(self.pos, scale(v, 1.0 / FPS), st.radius);
        want
    }

    /// Backing away from the hero; when the wall is behind, sideways: toward the middle of the ring,
    /// or (among walls) to whichever side has room.
    fn flee(&self, dir: V, speed: f64, terrain: &dyn Terrain) -> V {
        let away = scale(dir, -1.0);
        if terrain.open(add(self.pos, scale(away, 1.5))) {
            return scale(away, speed);
        }
        let side = V::new(-dir.y, dir.x);
        if let Some(c) = terrain.centre() {
            let toward_centre = dot(side, sub(c, self.pos)) >= 0.0;
            return scale(if toward_centre { side } else { scale(side, -1.0) }, speed);
        }
        for s in [side, scale(side, -1.0)] {
            if terrain.open(add(self.pos, scale(s, 1.5))) {
                return scale(s, speed);
            }
        }
        V::new(0.0, 0.0)
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

/// Frames from beginning `mv` at `dist` tiles from the hero until it reaches him: its startup; for
/// a lunge, the active frames it travels before it is near enough; for a bolt (loosed half a tile
/// ahead), the frames of flight until it touches him.
pub fn lands_after(mv: MoveId, dist: f64) -> u16 {
    let d = mv.data();
    if d.projectile {
        let travel = (dist - 0.5 - BOLT_RADIUS - HERO_RADIUS).max(0.0);
        return d.startup + ((travel / BOLT_SPEED * FPS).ceil() as u16).saturating_sub(1);
    }
    if d.travel > 0.0 {
        let gap = (dist - d.reach - HERO_RADIUS).max(0.0);
        let frames = (gap / (d.travel / d.active as f64)).ceil() as u16;
        return d.startup + frames.saturating_sub(1).min(d.active - 1);
    }
    d.startup
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
        let s = Sense { hero: V::new(12.0, 8.0), aim: V::new(-1.0, 0.0), swing: false };
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
        let far = Sense { hero: V::new(15.5, 8.0), aim: V::new(-1.0, 0.0), swing: false };
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
        let mut s = Sense { hero: V::new(9.5, 8.0), aim: V::new(-1.0, 0.0), swing: false };
        assert_eq!(d.choose(&s, &arena()), Some(MoveId::Slam));
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
        assert_eq!(fresh.choose(&s, &arena()), Some(MoveId::Sweep));
    }

    #[test]
    fn the_hexer_backs_away_from_a_hero_who_comes_close_and_slips_along_a_wall() {
        let mut d = Duelist::new(1, Archetype::Hexer, V::new(8.0, 8.0), V::new(1.0, 0.0));
        let s = Sense { hero: V::new(10.0, 8.0), aim: V::new(-1.0, 0.0), swing: false };
        d.think(&s, &arena(), &[]);
        assert!(d.pos.x < 8.0);
        let mut d = Duelist::new(1, Archetype::Hexer, V::new(0.6, 8.0), V::new(1.0, 0.0));
        let s = Sense { hero: V::new(2.5, 8.0), aim: V::new(-1.0, 0.0), swing: false };
        for _ in 0..20 {
            d.think(&s, &arena(), &[]);
        }
        assert!((d.pos.y - 8.0).abs() > 0.5, "cornered, it slips sideways");
    }

    #[test]
    fn the_skirmisher_sidesteps_a_swing_it_sees_but_not_twice_running() {
        let mut d = Duelist::new(1, Archetype::Skirmisher, V::new(8.0, 8.0), V::new(1.0, 0.0));
        let swing = Sense { hero: V::new(9.3, 8.0), aim: V::new(-1.0, 0.0), swing: true };
        for _ in 0..SIDESTEP {
            d.step();
            d.think(&swing, &arena(), &[]);
        }
        let gap = 9.3 - d.pos.x;
        assert!(gap > MoveId::L1.data().reach + d.radius() + 0.1, "out of the swing's reach: {gap}");
        // Seen again at once, it holds its ground.
        d.pos = V::new(8.0, 8.0);
        d.think(&swing, &arena(), &[]);
        assert!(d.pos.x >= 8.0 - 1e-9, "resting");
    }

    #[test]
    fn a_strike_is_planned_for_the_frame_it_reaches_the_hero() {
        assert_eq!(lands_after(MoveId::Slam, 1.5), 42);
        // A lunge from 2.4 tiles: 1.2 to close at 0.325 a frame, so on its fourth active frame.
        assert_eq!(lands_after(MoveId::Lunge, 2.4), 18 + 3);
        assert_eq!(lands_after(MoveId::Lunge, 1.0), 18, "already close: its first");
        // A bolt from 6.1 tiles: 5.0 to fly at 7/60 a frame, 43 frames counting the one it is loosed on.
        assert_eq!(lands_after(MoveId::Hex, 6.1), 36 + 43 - 1);
    }

    /// A room 12 by 9 with a wall down its middle, open only at the bottom.
    fn walled() -> Grid {
        use crate::logic::types::Tile;
        let (w, h) = (12, 9);
        let mut tiles = vec![Tile::Rock; (w * h) as usize];
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let wall = x == 6 && y < h - 2;
                tiles[(y * w + x) as usize] = if wall { Tile::Rock } else { Tile::Open };
            }
        }
        Grid::new(w, h, tiles, vec![0; (w * h) as usize], vec![false; (w * h) as usize])
    }

    #[test]
    fn round_a_wall_it_finds_the_way_and_never_walks_through_it() {
        let grid = walled();
        let hero = V::new(9.0, 2.0);
        let flow = Flow::new(&grid, P::new(9, 2), 40);
        assert_eq!(flow.dist(P::new(9, 2)), 0);
        assert!(flow.dist(P::new(3, 2)) > 12, "the long way round: {}", flow.dist(P::new(3, 2)));
        let terrain = GridTerrain { grid: &grid, flow: &flow };
        assert!(!terrain.clear(V::new(3.0, 2.0), hero), "the wall is in the way");
        let mut d = Duelist::new(1, Archetype::Brute, V::new(3.0, 2.0), V::new(1.0, 0.0));
        d.token = true;
        let s = Sense { hero, aim: V::new(-1.0, 0.0), swing: false };
        let mut reached = false;
        for _ in 0..60 * 20 {
            d.step();
            if d.think(&s, &terrain, &[]).is_some() {
                reached = true;
                break;
            }
            let t = P::new(d.pos.x.round() as i32, d.pos.y.round() as i32);
            assert!(grid.is_walkable(t.x, t.y, false), "inside the wall at {:?}", d.pos);
        }
        assert!(reached, "it came round to slam: {:?}", d.pos);
        // Walking straight at a wall, it slides along it instead.
        let p = terrain.slide(V::new(5.0, 3.0), V::new(0.5, 0.3), 0.4);
        assert!(p.x < 5.5 && p.y > 3.0, "{p:?}");
    }

    #[test]
    fn a_foe_sleeps_in_its_den_hunts_when_woken_and_walks_home_when_called_off() {
        let mut d = Duelist::bred(1, Breed::of(crate::logic::enemy::EnemyKind::Skeleton), V::new(4.0, 4.0), V::new(1.0, 0.0));
        d.den = Some(Den { lo: V::new(2.0, 2.0), hi: V::new(8.0, 8.0), home: V::new(4.0, 4.0), awake: false, returning: false });
        d.token = true;
        let s = Sense { hero: V::new(10.0, 4.0), aim: V::new(-1.0, 0.0), swing: false };
        d.think(&s, &arena(), &[]);
        assert_eq!(d.pos.x, 4.0, "asleep");
        assert!(!d.engaged());
        d.den.as_mut().unwrap().awake = true;
        for _ in 0..30 {
            d.think(&s, &arena(), &[]);
        }
        assert!(d.pos.x > 4.5 && d.engaged(), "hunting");
        d.den.as_mut().unwrap().returning = true;
        for _ in 0..600 {
            d.think(&s, &arena(), &[]);
        }
        assert!(!d.den.unwrap().awake && len(sub(d.pos, V::new(4.0, 4.0))) < 0.3, "home, and asleep again");
        // A draugr is a small brute: a fifth of the Brute's health and less than its bite.
        assert!(d.stats().hp < 100.0 && d.stats().armor < MoveId::L1.data().poise);
    }

    #[test]
    fn waiting_foes_keep_their_distance_from_each_other() {
        let mut a = Duelist::new(1, Archetype::Skirmisher, V::new(8.0, 8.0), V::new(1.0, 0.0));
        let s = Sense { hero: V::new(14.0, 14.0), aim: V::new(-1.0, 0.0), swing: false };
        a.think(&s, &arena(), &[(V::new(8.2, 8.0), 0.38)]);
        assert!(a.pos.x < 8.0, "pushed apart");
    }
}
