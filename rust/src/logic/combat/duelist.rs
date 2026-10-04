//! A foe's mind under the frame-data rules: where it walks, which way it faces, and when it wants
//! to strike. It moves freely (not tile to tile) over its `Terrain` (the Sparring Ring's square,
//! or the Vaults' rooms and corridors, round whose walls it finds its way), keeps its distance from
//! the others and the hero, strafes to the sector the threat manager gave it while it waits, and
//! closes in only with a token. A foe with a den idles there until it is woken, and goes home when
//! the hero is long gone. Its moves run on its own `Fighter`, its numbers on its `Breed`.
//!
//! The later stages' bodies carry their legends too (COMBAT.md §11): a burrow under the floor to a
//! mark that follows the hero and then holds, a cin gone in smoke, a kappa's spilled dish, a felled
//! dragon begging for a second blow, the dead climbing out of the floor, a ring that turns blades,
//! a ghost nothing touches.
//!
//! A body foe bites from beside him and stays where it bit: nothing of it darts across the floor,
//! stretches out to him or springs back. Only a distance foe strikes from afar, and only with a
//! bolt, each kind's its own (`BoltLook`).
//!
//! A prince (COMBAT.md §13) thinks as every foe does, from the table of his phase; his hide turns
//! blows outside his recoveries, and while his fist holds the hero he keeps still, open.

use super::archetype::{Archetype, Breed, Power, FLEE};
use super::frames::{MoveData, MoveId, Phase, Rider, Shape, BOLT_RADIUS, BOLT_SPEED, MARK_FOLLOW};
use super::fsm::{Event, Fighter, State};
use super::math::{add, dot, in_cone, len, norm, scale, sub, turn_toward};
use super::threat::Slot;
use super::FPS;
use crate::logic::enemy::EnemyKind;
use crate::logic::grid::Grid;
use crate::logic::pathing::route_toward;
use crate::logic::types::{CARDINALS, P, V};
use std::collections::VecDeque;

/// The hero's body radius, for spacing and hitboxes.
pub const HERO_RADIUS: f64 = 0.3;
/// Knockback a flinch gives, in tiles per second, and how much of it each frame keeps.
pub const KNOCKBACK: f64 = 3.0;
pub const KNOCK_KEEP: f64 = 0.82;
/// What a bolt looks like in flight, by who loosed it: the bog slime's glob of green spit, the
/// witch's violet hex, the jorogumo's ball of fire, the kitsune's foxfire, the cin's pale smokeless
/// flame, the arçura's thorny root dart, the goryō's lightning, Hel's cold drain; and the hero's
/// own thrown blade.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum BoltLook {
    Spit,
    Hex,
    Fire,
    Foxfire,
    Smokeless,
    Thorn,
    Lightning,
    Drain,
    /// Mammon's coins, flung in a fan.
    Coin,
    Blade,
}

impl BoltLook {
    /// The bolt `kind` looses.
    pub fn of(kind: EnemyKind) -> BoltLook {
        match kind {
            EnemyKind::Slime => BoltLook::Spit,
            EnemyKind::Jorogumo => BoltLook::Fire,
            EnemyKind::Kitsune => BoltLook::Foxfire,
            EnemyKind::Cin => BoltLook::Smokeless,
            EnemyKind::Arcura => BoltLook::Thorn,
            EnemyKind::Goryo => BoltLook::Lightning,
            EnemyKind::Hel => BoltLook::Drain,
            EnemyKind::Mammon => BoltLook::Coin,
            EnemyKind::Leviathan => BoltLook::Spit,
            _ => BoltLook::Hex,
        }
    }

    /// Its colour.
    pub fn tint(self) -> u32 {
        match self {
            BoltLook::Spit => 0x7aff5a,
            BoltLook::Hex => 0xc070ff,
            BoltLook::Fire => FIRE_TINT,
            BoltLook::Foxfire => 0x8af0ff,
            BoltLook::Smokeless => 0xfff2c8,
            BoltLook::Thorn => 0x8a6a2a,
            BoltLook::Lightning => 0xd8e8ff,
            BoltLook::Drain => 0x5a7cff,
            BoltLook::Coin => 0xe8b030,
            BoltLook::Blade => REFLECT_TINT,
        }
    }
}

/// The jorogumo's fire: her bolts, and her tell.
pub const FIRE_TINT: u32 = 0xff8a2a;
/// A bolt flying at the foes: his thrown blade, or a bolt parried back.
pub const REFLECT_TINT: u32 = 0xffc850;
/// The cin goes up in smoke from a hero this near, stands this far from him again after
/// `VANISH_FRAMES`, and may not vanish again for `VANISH_REST` frames.
pub const VANISH_NEAR: f64 = 2.6;
pub const VANISH_FAR: f64 = 6.0;
pub const VANISH_FRAMES: u16 = 24;
pub const VANISH_REST: u16 = 300;
/// The kappa's spilled dish: frames until it is full again, what a blow on it is worth meanwhile,
/// and its pace.
pub const SPILL_FRAMES: u16 = 360;
pub const SPILLED_TAKEN: f64 = 1.75;
pub const SPILLED_PACE: f64 = 0.6;
/// The ejder's plea: frames it lies begging for a second blow, and the share of its health that
/// blow gives back (a head grown again).
pub const PLEA_FRAMES: u16 = 150;
pub const PLEA_HEALTH: f64 = 1.0 / 7.0;
/// Tepegöz's ring: the share of a blow that tells while his eye is out of reach.
pub const WARD_SHARE: f64 = 0.15;
/// Frames the raised dead take to climb out of the floor, and how many one may raise at once.
pub const RISE_FRAMES: u16 = 50;
pub const MAX_RAISED: u8 = 2;
/// Frames at the start of a burrow while it is still above the floor.
pub const BURROW_SINK: u16 = 10;
/// The tülükuş's talons: the startup frames it is up on its wings over him, out of reach (it rises
/// straight up where it stands, and comes straight down).
pub const ALOFT: (u16, u16) = (4, 22);
/// The arçura slips away this much faster, shrunk to the grass.
pub const GROW_PACE: f64 = 1.5;
/// The kitsune conjures its illusions when its health first falls under each of these shares.
pub const CONJURE_AT: [f64; 2] = [0.7, 0.35];

/// A bolt in flight (a parried one flies back at the foes).
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
    /// What it looks like, and its colour (gold once it flies at the foes).
    pub look: BoltLook,
    pub tint: u32,
    /// What it does besides its damage (a drain), and the duelist who loosed it (0: him).
    pub rider: Rider,
    pub owner: u32,
    /// Bolts loosed together (a volley) share a number and land as one strike; 0 alone.
    pub volley: u32,
    /// An illusion's: it passes through him with nothing behind it.
    pub phantom: bool,
}

/// What a foe knows of the hero this frame.
#[derive(Clone, Copy, Debug)]
pub struct Sense {
    pub hero: V,
    /// Which way he faces (unit).
    pub aim: V,
}

/// The ground a foe fights on.
pub trait Terrain {
    /// Where a body of `radius` at `from` ends up after moving by `step`, sliding along what it cannot cross.
    fn slide(&self, from: V, step: V, radius: f64) -> V;
    /// The point to head for on the way from `from` to `to`: `to` itself while the way is clear.
    fn toward(&self, from: V, to: V) -> V;
    /// Is the straight way from `a` to `b` clear (for a bolt, a call)?
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

/// A foe's legend at work (COMBAT.md §11): what its folklore power keeps track of.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Lore {
    /// Frames until the kappa's spilled dish is full again (0: full).
    pub spilled: u16,
    /// Frames left gone in smoke, and before the cin may vanish again.
    pub vanish: u16,
    pub vanish_rest: u16,
    /// Frames left that the felled ejder lies begging for a second blow.
    pub plea: u16,
    /// An illusion of the kitsune's: one blow and it is gone, and its foxfire only seems.
    pub illusion: bool,
    /// The one who made it: an illusion's kitsune, a raised dead's Hel.
    pub master: Option<u32>,
    /// Frames left climbing out of the floor (the raised dead).
    pub rise: u16,
    /// It has fallen once and risen again (the Burning Field's dead): a second death pays nothing.
    pub risen: bool,
    /// The dead it has raised still standing (Hel), as the game counts them.
    pub raised: u8,
    /// Where a burrow or a vanishing set out from.
    pub from: V,
    /// Where its burrow comes up (on the hero), or where a vanished cin will stand.
    pub mark: V,
    /// How many times the kitsune has conjured its illusions.
    pub conjured: u8,
    /// The blow of a flurry its `struck` belongs to.
    pub bite: u8,
    /// Frames left that a prince's fist holds the hero (his grasp landed): he keeps still, open.
    pub hold: u16,
    /// A prince has raised his golems (once in a fight).
    pub summoned: bool,
    /// A beam's length as rock lets it run along the aim (locked with it); a charge's run so far.
    pub span: f64,
    /// The hot bell Leviathan is coiled round (Envy's third phase): his scales are burnt off on
    /// the side he turns to it (`prince::envy::burnt_side`).
    pub bell: Option<V>,
}

/// Something a foe's legend did this frame, for the game to show and sound.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Cue {
    /// The cin went up in smoke here, and will stand there.
    Vanished { from: V, to: V },
    /// It stands here again.
    Appeared(V),
    /// The kappa's dish is full again.
    Refilled,
    /// Its charge ran into rock here (`Shape::Charge`): the game may daze it.
    Crashed(V),
    /// A prince's health has fallen into the next phase of his fight.
    Phase(u8),
}

/// A patient duelist's tracking never runs into this many frames before its blow: a late step
/// aside still beats it.
pub const PATIENT_COMMIT: u16 = 12;

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
    /// Knockback velocity, fading.
    pub knock: V,
    /// Frames of the white flash of a blow.
    pub flash: u16,
    /// Its room in the Vaults (none in the ring: always in the fight).
    pub den: Option<Den>,
    /// Frames before each of its moves (in its breed's order) may be begun again.
    pub cool: [u16; 4],
    /// Its legend at work, and what it did this frame.
    pub lore: Lore,
    pub cues: Vec<Cue>,
    /// Frames its tracking may run past each move's own (a Patient rival, WARBAND.md §8.2), never
    /// into the last `PATIENT_COMMIT` frames before the blow.
    pub patient: u16,
    /// A puppet: it never thinks for itself (no token, no walking, no choice of move); whoever
    /// holds its strings moves it and begins its moves (the Peerless, Pride's echo of the hero).
    pub puppet: bool,
    /// Its charge ran into rock this move (`Shape::Charge`): the game may daze it. Cleared as its
    /// next move begins.
    pub crashed: bool,
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
            knock: V::new(0.0, 0.0),
            flash: 0,
            den: None,
            cool: [0; 4],
            lore: Lore::default(),
            cues: Vec::new(),
            patient: 0,
            puppet: false,
            crashed: false,
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
        self.knock = V::new(0.0, 0.0);
        self.vel = V::new(0.0, 0.0);
        self.rest = 30;
        self.cool = [0; 4];
        self.lore = Lore { illusion: self.lore.illusion, master: self.lore.master, ..Lore::default() };
        self.breed.phase = 1;
    }

    /// A move as this body performs it: a big body's blows reach farther, and it begins them
    /// from farther off.
    pub fn data(&self, mv: MoveId) -> MoveData {
        // (Its fighter's own table: the hero's string as a kit swings it, for one that wears his.)
        let mut d = self.f.data(mv);
        let r = self.breed.reach;
        if r > 0.0 && !d.projectile && matches!(d.shape, Shape::Cone | Shape::Quake { .. }) {
            d.reach += r;
            d.range.1 += r;
            if let Shape::Quake { radius, share } = d.shape {
                d.shape = Shape::Quake { radius: radius + r, share };
            }
        }
        d
    }

    /// What its blows carry besides their damage: the move's own rider, or its body's.
    pub fn rider(&self, mv: MoveId) -> Rider {
        match mv.data().rider {
            Rider::None => self.breed.rider,
            r => r,
        }
    }

    /// How far the hero stands off its facing, in radians.
    fn off(&self, s: &Sense) -> f64 {
        let to = norm(sub(s.hero, self.pos));
        dot(to, self.facing).clamp(-1.0, 1.0).acos()
    }

    /// The move it would use now, from where it stands and the way it faces: the first it knows
    /// that is rested, whose range and angle the hero is in, with a clear line if it wants one.
    pub fn choose(&self, s: &Sense, terrain: &dyn Terrain) -> Option<MoveId> {
        let d = len(sub(s.hero, self.pos));
        let off = self.off(s);
        self.breed
            .moves()
            .iter()
            .enumerate()
            .find(|&(k, &mv)| {
                let m = self.data(mv);
                self.cool.get(k).is_none_or(|&c| c == 0)
                    && d >= m.range.0
                    && d <= m.range.1
                    && off >= m.off.0
                    && off <= m.off.1
                    && (!m.line || terrain.clear(self.pos, s.hero))
                    && (mv != MoveId::Raise || self.lore.raised < MAX_RAISED)
            })
            .map(|(_, &mv)| mv)
    }

    /// Frames until its strike would land if it set out now: the walk into range, then its startup.
    pub fn eta(&self, s: &Sense) -> u32 {
        let d = len(sub(s.hero, self.pos));
        let m = self.data(self.breed.main_move());
        let range = m.range.1;
        let walk = ((d - range).max(0.0) / self.stats().walk * FPS).ceil() as u32;
        walk + lands_after(&m, d.min(range)) as u32
    }

    /// Frames from beginning `mv` now until it reaches the hero: its startup, and a bolt's flight.
    pub fn lands_in(&self, mv: MoveId, hero: V) -> u16 {
        lands_after(&self.data(mv), len(sub(hero, self.pos)))
    }

    /// Begins `mv` with the hero at `hero`: its aim starts where it faces, a burrow's mark on the
    /// floor where he stands (as far as the move reaches), and the move rests for its cooldown.
    pub fn begin(&mut self, mv: MoveId, hero: V) -> bool {
        if !self.f.begin(mv) {
            return false;
        }
        self.aim = self.facing;
        self.struck = false;
        self.wait = 0;
        self.lore.bite = 0;
        self.lore.from = self.pos;
        self.lore.span = 0.0;
        self.crashed = false;
        let d = self.data(mv);
        if matches!(d.shape, Shape::Mark { .. }) {
            self.lore.mark = self.within_reach(hero, d.reach);
        }
        if let Some(k) = self.breed.moves().iter().position(|&m| m == mv) {
            if let Some(c) = self.cool.get_mut(k) {
                *c = d.cooldown;
            }
        }
        true
    }

    /// `p`, or the nearest point to it no farther than `reach` from where its move set out.
    fn within_reach(&self, p: V, reach: f64) -> V {
        let to = sub(p, self.lore.from);
        if len(to) > reach {
            add(self.lore.from, scale(norm(to), reach))
        } else {
            p
        }
    }

    /// Its fighter's frame, plus what the end of a move means for it, and its legend's clocks.
    pub fn step(&mut self) -> Vec<Event> {
        self.flash = self.flash.saturating_sub(1);
        self.rest = self.rest.saturating_sub(1);
        for c in self.cool.iter_mut() {
            *c = c.saturating_sub(1);
        }
        self.lore.vanish_rest = self.lore.vanish_rest.saturating_sub(1);
        if self.lore.spilled > 0 {
            self.lore.spilled -= 1;
            if self.lore.spilled == 0 {
                self.cues.push(Cue::Refilled);
            }
        }
        if self.lore.vanish > 0 {
            self.lore.vanish -= 1;
            if self.lore.vanish == 0 {
                self.pos = self.lore.mark;
                self.cues.push(Cue::Appeared(self.pos));
            }
        }
        // Climbing out of the floor, or lying felled and begging: its fighter keeps still.
        if self.lore.rise > 0 {
            self.lore.rise -= 1;
            return Vec::new();
        }
        if self.lore.plea > 0 || self.lore.hold > 0 {
            return Vec::new();
        }
        let ev = self.f.step();
        if ev.iter().any(|e| matches!(e, Event::Ended(_))) {
            self.rest = self.stats().rest;
        }
        ev
    }

    /// Out of the hero's reach: up on its wings over him, under the floor in a burrow, gone in
    /// smoke, or a spirit whose wrath is not upon it.
    pub fn untouchable(&self) -> bool {
        if self.lore.vanish > 0 {
            return true;
        }
        match self.f.state {
            State::Attack { mv: mv @ (MoveId::Burrow | MoveId::Dive), frame } => frame >= BURROW_SINK && frame < mv.data().startup,
            State::Attack { mv: MoveId::Talons, frame } => (ALOFT.0..ALOFT.1).contains(&frame),
            // In the air of a leap (only a puppet of the hero's leaps).
            State::Evade { frame } => frame < super::fsm::EVADE_AIR,
            State::Attack { .. } | State::Reel { .. } => false,
            _ => self.breed.power == Power::Spirit,
        }
    }

    /// Tepegöz's eye within reach (or a prince's hide open): bent over in a recovery, reeling, or
    /// holding the hero in his fist.
    pub fn exposed(&self) -> bool {
        matches!(self.f.attack(), Some((_, Phase::Recovery, _))) || self.f.reeling().is_some() || self.lore.hold > 0
    }

    /// What a blow on it is worth by its legend: a share through Tepegöz's ring while his eye is
    /// out of reach, more on a kappa whose dish is spilled.
    pub fn taken(&self) -> f64 {
        match self.breed.power {
            Power::Ward if !self.exposed() => WARD_SHARE,
            p if p.is_hide() && !self.exposed() => super::prince::HIDE_SHARE,
            Power::Dish if self.lore.spilled > 0 => SPILLED_TAKEN,
            _ => 1.0,
        }
    }

    /// What a blow from `from` is worth on it: as `taken`, but nothing on an elf-maid's lovely
    /// front (she is only to be struck from behind), and all of it on Leviathan's side burnt by
    /// the bell.
    pub fn taken_from(&self, from: V) -> f64 {
        if let Some(bell) = self.lore.bell.filter(|_| self.breed.power.is_hide()) {
            if super::prince::envy::burnt_side(self.pos, from, bell) {
                return 1.0;
            }
        }
        if self.breed.power == Power::Hollow {
            let to = V::new(from.x - self.pos.x, from.y - self.pos.y);
            if to.x * self.facing.x + to.y * self.facing.y > 0.0 {
                return 0.0;
            }
        }
        self.taken()
    }

    /// Lying felled, begging for a second blow (the ejder).
    pub fn begging(&self) -> bool {
        self.lore.plea > 0
    }

    /// In the fight's reckoning: alive (begging counts) and not an illusion.
    pub fn counts(&self) -> bool {
        self.alive() && !self.lore.illusion
    }

    /**
     * Does its move's blow reach a body of `radius` at `p` this frame, and at what share of its
     * damage? The cone; the cone and the ground shaking round it as the club lands; the mark on
     * the floor. Nothing for a bolt or a spell.
     */
    pub fn reaches(&self, p: V, radius: f64) -> Option<f64> {
        let (mv, phase, frame) = self.f.attack()?;
        if phase != Phase::Active {
            return None;
        }
        let d = self.data(mv);
        if d.projectile {
            return None;
        }
        let f = frame - d.startup;
        match d.shape {
            Shape::Cone => in_cone(self.pos, self.aim, p, radius, d.reach, d.half_angle).then_some(1.0),
            Shape::Quake { radius: ring, share } => {
                if in_cone(self.pos, self.aim, p, radius, d.reach, d.half_angle) {
                    Some(1.0)
                } else {
                    (f == 0 && len(sub(p, self.pos)) <= ring + radius).then_some(share)
                }
            }
            Shape::Mark { radius: r } => (len(sub(p, self.lore.mark)) <= r + radius).then_some(1.0),
            Shape::Beam { width, .. } => {
                let end = add(self.pos, scale(self.aim, self.lore.span));
                (from_segment(self.pos, end, p) <= width / 2.0 + radius).then_some(1.0)
            }
            Shape::Charge { .. } => (len(sub(p, self.pos)) <= d.reach + radius).then_some(1.0),
            Shape::Volley { .. } | Shape::Spell | Shape::Traps { .. } | Shape::TrapLine { .. } => None,
        }
    }

    /// Where the cin stands again: `VANISH_FAR` from him, away from him if there is room, else
    /// round to either side.
    fn vanish_spot(&self, hero: V, terrain: &dyn Terrain) -> V {
        let away = norm(sub(self.pos, hero));
        let away = if len(away) < 1e-9 { V::new(-1.0, 0.0) } else { away };
        let base = away.y.atan2(away.x);
        for k in [0.0, 0.8, -0.8, 1.6, -1.6, 2.4, -2.4, std::f64::consts::PI] {
            let a = base + k;
            let p = add(hero, scale(V::new(a.cos(), a.sin()), VANISH_FAR));
            if terrain.open(p) {
                return p;
            }
        }
        add(hero, scale(away, VANISH_FAR * 0.5))
    }

    /**
     * One frame of thought and motion. Asleep in its den it stands; going home it walks there.
     * In the fight it tracks the hero through its startup (a burrow's mark on the floor following
     * him, no faster than `MARK_FOLLOW`), and burrows to its mark; every other move it makes where
     * it stands. Free, it backs off (or vanishes), closes in (with a token, round any wall), or
     * strafes to its slot. Returns the move it wants to begin now (the threat manager's commit
     * gate decides). Felled, rising, or gone in smoke, it does nothing.
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
        if self.lore.plea > 0 || self.lore.rise > 0 || self.lore.hold > 0 {
            self.vel = V::new(0.0, 0.0);
            return None;
        }
        if self.lore.vanish > 0 {
            // Gone in smoke, it drifts unseen to where it will stand.
            let u = 1.0 - self.lore.vanish as f64 / VANISH_FRAMES as f64;
            self.pos = add(self.lore.from, scale(sub(self.lore.mark, self.lore.from), u));
            self.facing = turn_toward(self.facing, dir, turn);
            self.vel = V::new(0.0, 0.0);
            return None;
        }
        // The way to him (or to its slot), round whatever stands between.
        let way_to = |p: V, here: V| norm(sub(terrain.toward(here, p), here));
        let mut v = V::new(0.0, 0.0);
        let mut want = None;
        match self.f.state {
            State::Attack { mv, frame } => {
                let data = self.data(mv);
                let track = data.track + self.patient.min(data.startup.saturating_sub(PATIENT_COMMIT + data.track));
                if frame < track {
                    self.facing = turn_toward(self.facing, dir, turn);
                    self.aim = self.facing;
                    if let Shape::Beam { length, .. } = data.shape {
                        // (Its length as the rock lets it run, locked with the aim.)
                        self.lore.span = beam_span(terrain, self.pos, self.aim, length);
                    }
                    if matches!(data.shape, Shape::Mark { .. }) {
                        let goal = self.within_reach(s.hero, data.reach);
                        let gap = sub(goal, self.lore.mark);
                        let step = MARK_FOLLOW / FPS;
                        self.lore.mark = if len(gap) <= step { goal } else { add(self.lore.mark, scale(norm(gap), step)) };
                    }
                }
                if let (Shape::Charge { length, speed }, Phase::Active) = (data.shape, data.phase(frame)) {
                    // The run along the locked aim, until its length is run or the rock stops it.
                    let step = (speed / FPS).min(length - self.lore.span).max(0.0);
                    if step > 0.0 && !self.crashed {
                        let want = add(self.pos, scale(self.aim, step));
                        let to = terrain.slide(self.pos, sub(want, self.pos), st.radius);
                        let moved = len(sub(to, self.pos));
                        self.lore.span += step;
                        if moved < step * 0.5 || !terrain.open(add(to, scale(self.aim, st.radius + 0.1))) {
                            self.crashed = true;
                            self.cues.push(Cue::Crashed(to));
                        }
                        self.vel = scale(sub(to, self.pos), FPS);
                        self.pos = to;
                    } else {
                        self.vel = V::new(0.0, 0.0);
                    }
                    return None;
                }
                if matches!(mv, MoveId::Burrow | MoveId::Dive) {
                    // Down under the floor and along to its mark, there by its first active frame;
                    // it bursts up, strikes and recovers where it came up.
                    let sink = BURROW_SINK;
                    let at = if data.phase(frame) == Phase::Startup {
                        let u = frame.saturating_sub(sink) as f64 / (data.startup - sink).max(1) as f64;
                        let u = u * u * (3.0 - 2.0 * u);
                        add(self.lore.from, scale(sub(self.lore.mark, self.lore.from), u))
                    } else if data.phase(frame) == Phase::Active {
                        self.lore.mark
                    } else {
                        self.pos
                    };
                    if data.phase(frame) != Phase::Recovery {
                        self.vel = scale(sub(at, self.pos), FPS);
                        self.pos = terrain.slide(self.pos, sub(at, self.pos), st.radius);
                        return None;
                    }
                }
            }
            State::Neutral => {
                self.facing = turn_toward(self.facing, dir, turn);
                if self.arch == Archetype::Hexer && d < FLEE {
                    if self.breed.power == Power::Vanish && self.lore.vanish_rest == 0 && d < VANISH_NEAR {
                        // Gone in a breath of smoke, to stand far off.
                        let spot = self.vanish_spot(s.hero, terrain);
                        self.lore.from = self.pos;
                        self.lore.mark = spot;
                        self.lore.vanish = VANISH_FRAMES;
                        self.lore.vanish_rest = VANISH_REST;
                        self.cues.push(Cue::Vanished { from: self.pos, to: spot });
                        self.vel = V::new(0.0, 0.0);
                        return None;
                    }
                    let pace = if self.breed.power == Power::Grow { st.retreat * GROW_PACE } else { st.retreat };
                    v = self.flee(dir, pace, terrain);
                } else if self.token {
                    if let Some(mv) = self.choose(s, terrain) {
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
        if self.lore.spilled > 0 {
            v = scale(v, SPILLED_PACE);
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

    /// A blow landed on it: the white flash, and a shove away from `from` if it flinched (a brute
    /// gives little ground, a giant hardly any).
    pub fn shove(&mut self, from: V, flinched: bool) {
        self.flash = 6;
        if flinched {
            let giant = self.radius() > 0.7;
            let k = match (self.arch == Archetype::Brute, giant) {
                (true, true) => 0.15,
                (true, false) => 0.4,
                (false, true) => 0.3,
                (false, false) => 1.0,
            };
            self.knock = scale(norm(sub(self.pos, from)), KNOCKBACK * k);
        }
    }
}

/// Frames from beginning a move of data `d` at `dist` tiles from the hero until it reaches him:
/// its startup; for a bolt (loosed half a tile ahead), the frames of flight until it touches him;
/// for coins thrown on the floor, their flight and their fuse.
/// What a bolt's flight leaves out of the distance between a caster and him (it leaves half a
/// tile out, and touches at the sum of the radii), and the slowest and fastest it may be paced
/// (tiles a second): the crossfire reckons whether two bolts can meet on one frame.
pub const BOLT_RADIUS_SUM: f64 = 0.5 + BOLT_RADIUS + HERO_RADIUS;
pub const BOLT_SPEED_RANGE: (f64, f64) = (BOLT_SPEED * 0.5, BOLT_SPEED * 1.6);

pub fn lands_after(d: &MoveData, dist: f64) -> u16 {
    if let Some((_, _, fuse, _)) = d.traps() {
        return d.startup + super::prince::COIN_FLIGHT + fuse;
    }
    if let Shape::Charge { speed, .. } = d.shape {
        // The run to him.
        let run = (dist - d.reach).max(0.0);
        return d.startup + (run / speed.max(0.1) * FPS).ceil() as u16;
    }
    if d.projectile {
        let travel = (dist - 0.5 - BOLT_RADIUS - HERO_RADIUS).max(0.0);
        return d.startup + ((travel / BOLT_SPEED * FPS).ceil() as u16).saturating_sub(1);
    }
    d.startup
}

/// How far a beam from `from` along `aim` runs before solid rock stops it, up to `length`.
pub fn beam_span(terrain: &dyn Terrain, from: V, aim: V, length: f64) -> f64 {
    const STEP: f64 = 0.25;
    let mut span = 0.0;
    while span < length {
        let next = (span + STEP).min(length);
        if !terrain.clear(add(from, scale(aim, span)), add(from, scale(aim, next))) {
            break;
        }
        span = next;
    }
    span
}

/// The distance from `p` to the segment `a`..`b`.
pub fn from_segment(a: V, b: V, p: V) -> f64 {
    let ab = sub(b, a);
    let l2 = dot(ab, ab);
    if l2 < 1e-12 {
        return len(sub(p, a));
    }
    let t = (dot(sub(p, a), ab) / l2).clamp(0.0, 1.0);
    len(sub(p, add(a, scale(ab, t))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::combat::frames::BITE_RANGE;

    fn arena() -> Arena {
        Arena { lo: V::new(0.0, 0.0), hi: V::new(16.0, 16.0) }
    }

    /// Open ground to the west of a wall of rock at `x` (and nothing else in the way).
    struct Wall {
        x: f64,
    }

    impl Terrain for Wall {
        fn slide(&self, from: V, step: V, radius: f64) -> V {
            let p = add(from, step);
            V::new(p.x.min(self.x - radius), p.y)
        }
        fn toward(&self, _from: V, to: V) -> V {
            to
        }
        fn clear(&self, a: V, b: V) -> bool {
            a.x < self.x - 0.25 && b.x < self.x - 0.25
        }
        fn open(&self, p: V) -> bool {
            p.x < self.x - 0.4
        }
    }

    /// A brute that wears `shape` as its first move (in its fighter's own table, as a kit's string
    /// is worn), begun on the hero at `hero`, run to its first active frame.
    fn wearing(shape: Shape, at: V, hero: V, terrain: &dyn Terrain) -> Duelist {
        let mut d = Duelist::new(1, Archetype::Brute, at, norm(sub(hero, at)));
        d.f.table[0] =
            MoveData { name: "TEST", startup: 30, active: 40, recovery: 40, track: 12, reach: 0.9, shape, ..MoveId::Slam.data() };
        let s = Sense { hero, aim: V::new(-1.0, 0.0) };
        assert!(d.begin(MoveId::L1, hero));
        for _ in 0..200 {
            d.step();
            d.think(&s, terrain, &[]);
            if matches!(d.f.attack(), Some((_, Phase::Active, _))) {
                break;
            }
        }
        assert!(matches!(d.f.attack(), Some((_, Phase::Active, _))));
        d
    }

    #[test]
    fn a_beam_runs_along_the_locked_aim_until_the_rock_and_strikes_within_half_its_width() {
        let wall = Wall { x: 6.0 };
        let shape = Shape::Beam { length: 10.0, width: 1.0 };
        let d = wearing(shape, V::new(1.0, 5.0), V::new(9.0, 5.0), &wall);
        assert!(d.lore.span > 4.0 && d.lore.span < 5.0, "stopped by the rock: {}", d.lore.span);
        assert_eq!(beam_span(&arena(), V::new(1.0, 5.0), V::new(1.0, 0.0), 10.0), 10.0, "open ground: its whole length");
        assert_eq!(d.reaches(V::new(4.0, 5.3), 0.2), Some(1.0), "on the line");
        assert_eq!(d.reaches(V::new(4.0, 5.8), 0.2), None, "beside it");
        assert_eq!(d.reaches(V::new(8.0, 5.0), 0.2), None, "behind the rock");
        let data = d.data(MoveId::L1);
        assert!(data.blockable() && !data.parriable());
        assert_eq!(lands_after(&data, 6.0), data.startup, "it lands as it lights");
    }

    #[test]
    fn a_charge_runs_its_length_along_its_aim_and_a_wall_stops_it_short() {
        // On open ground: it runs its length and no farther, and reaches him on the way.
        let shape = Shape::Charge { length: 6.0, speed: 12.0 };
        let mut d = wearing(shape, V::new(2.0, 8.0), V::new(14.0, 8.0), &arena());
        let s = Sense { hero: V::new(14.0, 8.0), aim: V::new(-1.0, 0.0) };
        let mut reached = false;
        for _ in 0..40 {
            reached |= d.reaches(V::new(6.0, 8.0), 0.3).is_some();
            d.step();
            d.think(&s, &arena(), &[]);
        }
        assert!((d.pos.x - 8.0).abs() < 0.05 && !d.crashed, "ran its length: {:?}", d.pos);
        assert!(reached, "it passed him");
        // Into a wall: stopped short, crashed, and it says so.
        let wall = Wall { x: 5.0 };
        let mut d = wearing(shape, V::new(1.0, 5.0), V::new(9.0, 5.0), &wall);
        let s = Sense { hero: V::new(9.0, 5.0), aim: V::new(-1.0, 0.0) };
        for _ in 0..40 {
            d.step();
            d.think(&s, &wall, &[]);
        }
        assert!(d.crashed && d.pos.x < 5.0, "{:?}", d.pos);
        assert!(d.cues.iter().any(|c| matches!(c, Cue::Crashed(_))));
        // Its next move clears the crash.
        while !d.f.is_neutral() {
            d.step();
        }
        assert!(d.begin(MoveId::L1, s.hero) && !d.crashed);
        // It lands when its run reaches him.
        let data = d.data(MoveId::L1);
        assert!(lands_after(&data, 6.9) > lands_after(&data, 1.0) && lands_after(&data, 0.5) == data.startup);
        assert!(data.blockable() && data.parriable());
    }

    #[test]
    fn with_a_token_the_skirmisher_walks_up_beside_him_and_bites_where_it_stands() {
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
        assert_eq!(wanted, Some(MoveId::Bite));
        let gap = 12.0 - d.pos.x;
        assert!(gap <= BITE_RANGE + 1e-9, "beside him: {gap}");
        // The bite carries it nowhere, and afterwards it does not spring back: it stays where it bit.
        assert!(d.begin(MoveId::Bite, s.hero));
        let at = d.pos;
        let data = MoveId::Bite.data();
        for _ in 0..data.total() + 10 {
            d.step();
            d.think(&s, &arena(), &[]);
            assert!(len(sub(d.pos, at)) < 0.05, "it moved to {:?} from {at:?}", d.pos);
        }
        // From farther off it has no move at all: it walks.
        let far = Duelist::new(2, Archetype::Skirmisher, V::new(9.5, 8.0), V::new(1.0, 0.0));
        assert_eq!(far.choose(&s, &arena()), None);
    }

    #[test]
    fn a_committed_slam_no_longer_follows_the_hero() {
        let mut d = Duelist::new(1, Archetype::Brute, V::new(8.0, 8.0), V::new(1.0, 0.0));
        let mut s = Sense { hero: V::new(9.5, 8.0), aim: V::new(-1.0, 0.0) };
        assert_eq!(d.choose(&s, &arena()), Some(MoveId::Slam));
        d.begin(MoveId::Slam, s.hero);
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
    fn a_strike_is_planned_for_the_frame_it_reaches_the_hero() {
        assert_eq!(lands_after(&MoveId::Slam.data(), 1.5), 42);
        // A bite from beside him lands on its first active frame.
        assert_eq!(lands_after(&MoveId::Bite.data(), 1.0), 18);
        // A bolt from 6.1 tiles: 5.0 to fly at 7/60 a frame, 43 frames counting the one it is loosed on.
        assert_eq!(lands_after(&MoveId::Hex.data(), 6.1), 36 + 43 - 1);
        assert_eq!(lands_after(&MoveId::Roots.data(), 6.1), 38 + 43 - 1, "the root dart flies like any bolt");
        // A burrow comes up when the startup ends, wherever he stands.
        assert_eq!(lands_after(&MoveId::Burrow.data(), 6.0), 60);
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
        let s = Sense { hero, aim: V::new(-1.0, 0.0) };
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
        let mut d = Duelist::bred(1, Breed::of(EnemyKind::Skeleton), V::new(4.0, 4.0), V::new(1.0, 0.0));
        d.den = Some(Den { lo: V::new(2.0, 2.0), hi: V::new(8.0, 8.0), home: V::new(4.0, 4.0), awake: false, returning: false });
        d.token = true;
        let s = Sense { hero: V::new(10.0, 4.0), aim: V::new(-1.0, 0.0) };
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
        let s = Sense { hero: V::new(14.0, 14.0), aim: V::new(-1.0, 0.0) };
        a.think(&s, &arena(), &[(V::new(8.2, 8.0), 0.38)]);
        assert!(a.pos.x < 8.0, "pushed apart");
    }

    fn bred(kind: EnemyKind, at: V) -> Duelist {
        Duelist::bred(1, Breed::of(kind), at, V::new(1.0, 0.0))
    }

    /// Runs `d` through `frames` frames of its move with the hero at `hero`.
    fn play(d: &mut Duelist, hero: V, frames: u16) {
        let s = Sense { hero, aim: V::new(-1.0, 0.0) };
        for _ in 0..frames {
            d.step();
            d.think(&s, &arena(), &[]);
        }
    }

    #[test]
    fn a_burrows_mark_follows_the_hero_slower_than_he_walks_then_holds() {
        let mut d = bred(EnemyKind::Nidhogg, V::new(2.0, 8.0));
        assert!(d.begin(MoveId::Burrow, V::new(8.0, 8.0)));
        assert_eq!(d.lore.mark, V::new(8.0, 8.0), "on him");
        // He walks off at 5.2 tiles a second: the mark comes after him at half that.
        let burrow = MoveId::Burrow.data();
        let mut hero = V::new(8.0, 8.0);
        let s = |hero: V| Sense { hero, aim: V::new(1.0, 0.0) };
        for _ in 0..burrow.track {
            hero.y += 5.2 / 60.0;
            d.step();
            d.think(&s(hero), &arena(), &[]);
        }
        let mark = d.lore.mark;
        assert!((mark.y - 8.0 - MARK_FOLLOW * burrow.track as f64 / 60.0).abs() < 0.05, "{mark:?}");
        for _ in 0..10 {
            hero.y += 5.2 / 60.0;
            d.step();
            d.think(&s(hero), &arena(), &[]);
        }
        assert_eq!(d.lore.mark, mark, "held once its tracking is over");
        // A mark no farther than the move reaches.
        let mut far = bred(EnemyKind::Nidhogg, V::new(1.0, 8.0));
        far.begin(MoveId::Burrow, V::new(15.0, 1.0));
        assert!(len(sub(far.lore.mark, V::new(1.0, 8.0))) <= burrow.reach + 1e-9);
    }

    #[test]
    fn the_burrower_goes_under_the_floor_and_comes_up_beneath_him() {
        let hero = V::new(8.0, 8.0);
        let mut n = bred(EnemyKind::Nidhogg, V::new(2.0, 8.0));
        n.begin(MoveId::Burrow, hero);
        play(&mut n, hero, 5);
        assert!(!n.untouchable(), "still diving in");
        play(&mut n, hero, 20);
        assert!(n.untouchable(), "under the floor");
        play(&mut n, hero, MoveId::Burrow.data().startup - 25);
        assert!(!n.untouchable() && len(sub(n.pos, hero)) < 0.05, "up beneath him: {:?}", n.pos);
        assert_eq!(n.reaches(hero, HERO_RADIUS), Some(1.0));
        assert_eq!(n.reaches(V::new(8.0, 9.6), HERO_RADIUS), None, "out of the circle");
    }

    #[test]
    fn every_body_foe_bites_from_beside_him_and_stays_where_it_bit() {
        // The talons, the weight, the shove, the bite on the neck: each from beside him, and
        // the body that struck is where it began, its head and limbs at home.
        let hero = V::new(8.0, 8.0);
        for kind in [EnemyKind::Tulukus, EnemyKind::Albasti, EnemyKind::Kappa, EnemyKind::Rokurokubi, EnemyKind::Ejder] {
            let mut d = bred(kind, V::new(8.0 - 1.0 - d_reach(kind), 8.0));
            let s = Sense { hero, aim: V::new(-1.0, 0.0) };
            let mv = d.choose(&s, &arena()).unwrap_or_else(|| panic!("{kind:?} bites from beside him"));
            assert!(d.begin(mv, hero));
            let at = d.pos;
            let data = d.data(mv);
            let mut landed = false;
            for _ in 0..data.total() + 10 {
                d.step();
                d.think(&s, &arena(), &[]);
                landed |= d.reaches(hero, HERO_RADIUS).is_some();
                assert!(len(sub(d.pos, at)) < 0.05, "{kind:?} moved to {:?} from {at:?}", d.pos);
            }
            assert!(landed, "{kind:?}: it reached him");
            // A step back out of its reach, and it has nothing to strike with.
            let away = bred(kind, V::new(8.0 - BITE_RANGE - d_reach(kind) - 0.6, 8.0));
            assert_eq!(away.choose(&s, &arena()), None, "{kind:?} does not strike from afar");
        }
    }

    #[test]
    fn the_owl_rises_out_of_reach_where_it_stands_and_drops_its_talons_beside_him() {
        let hero = V::new(8.0, 8.0);
        let mut d = bred(EnemyKind::Tulukus, V::new(7.0, 8.0));
        assert!(d.begin(MoveId::Talons, hero));
        play(&mut d, hero, ALOFT.0 + 1);
        assert!(d.untouchable(), "up on its wings");
        assert_eq!(d.pos, V::new(7.0, 8.0), "straight up, not across");
        play(&mut d, hero, ALOFT.1 - ALOFT.0);
        assert!(!d.untouchable(), "down again before it strikes");
        play(&mut d, hero, MoveId::Talons.data().startup - ALOFT.1);
        assert_eq!(d.reaches(hero, HERO_RADIUS), Some(1.0));
    }

    /// What a big body adds to its reach.
    fn d_reach(kind: EnemyKind) -> f64 {
        Breed::of(kind).reach
    }

    #[test]
    fn the_cin_vanishes_from_a_hero_who_comes_near_and_stands_far_off() {
        let mut d = bred(EnemyKind::Cin, V::new(8.0, 8.0));
        let hero = V::new(9.5, 8.0);
        play(&mut d, hero, 1);
        assert!(d.untouchable() && d.cues.iter().any(|c| matches!(c, Cue::Vanished { .. })));
        play(&mut d, hero, VANISH_FRAMES);
        assert!(!d.untouchable());
        assert!((len(sub(d.pos, hero)) - VANISH_FAR).abs() < 0.1, "{:?}", d.pos);
        assert!(d.lore.vanish_rest > 0, "and not again at once");
    }

    #[test]
    fn a_giant_reaches_farther() {
        let g = bred(EnemyKind::Tepegoz, V::new(8.0, 8.0));
        assert!(g.data(MoveId::Slam).reach > MoveId::Slam.data().reach + 0.5);
        assert!(g.choose(&Sense { hero: V::new(10.2, 8.0), aim: V::new(-1.0, 0.0) }, &arena()) == Some(MoveId::Slam));
    }

    #[test]
    fn the_ring_turns_blades_but_for_the_eye_and_a_spilled_dish_leaves_the_kappa_weak() {
        let mut t = bred(EnemyKind::Tepegoz, V::new(8.0, 8.0));
        assert_eq!(t.taken(), WARD_SHARE);
        t.begin(MoveId::Slam, V::new(9.0, 8.0));
        play(&mut t, V::new(9.0, 8.0), MoveId::Slam.data().startup + 7);
        assert!(t.exposed() && t.taken() == 1.0, "bent over in his recovery, his eye in reach");
        let mut k = bred(EnemyKind::Kappa, V::new(8.0, 8.0));
        assert_eq!(k.taken(), 1.0);
        k.lore.spilled = SPILL_FRAMES;
        assert_eq!(k.taken(), SPILLED_TAKEN);
        for _ in 0..SPILL_FRAMES {
            k.step();
        }
        assert!(k.lore.spilled == 0 && k.cues.contains(&Cue::Refilled));
        // A ghost is out of reach but in its wrath.
        let mut g = bred(EnemyKind::Goryo, V::new(8.0, 8.0));
        assert!(g.untouchable());
        g.begin(MoveId::Thunder, V::new(12.0, 8.0));
        assert!(!g.untouchable());
    }
}
