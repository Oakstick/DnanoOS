//! The brawl (COMBAT.md §9): fighting by frame data, wherever it is fought. In the gym's Sparring
//! Ring it is the lab (bouts, see `sparring`); in the Vaults of Mammon it is the game: the packs
//! are duelists asleep in their rooms, the threat manager paces every foe that hunts him, his
//! blows and theirs land on their frames, and his health, his shield and his blade pay for it.
//!
//! The hero's attack, guard and evade go through his `Fighter`, which runs on his kit (his gear
//! as numbers, `combat::kit`), refreshed every frame so a dulled blade, a broken shield or a
//! potion counts at once.

use super::Game;
use crate::audio::Sfx;
use crate::logic::combat::duelist::{bolt_tint, Arena, Bolt, Den, Flow, GridTerrain, Sense, Terrain, HERO_RADIUS};
use crate::logic::combat::frames::{BOLT_LIFE, BOLT_RADIUS, BOLT_SPEED, REFLECT_POISE, REFLECT_SPEED};
use crate::logic::combat::fsm::{Defense, Effect, Hit, State};
use crate::logic::combat::kit::Kit;
use crate::logic::combat::threat::{assign_slots, Request, Waiter, REVOKE_REST, SPACING};
use crate::logic::combat::{math, secs, Event, Fighter, FrameClock, Input, MoveId, Phase, Reel, ThreatManager};
use crate::logic::enemy::{EnemyKind, EnemyMode};
use crate::logic::gym;
use crate::logic::types::{P, V};
use crate::logic::vault::{LEASH, WAKE_RANGE};

/// Frames between re-dealing the waiting foes' sectors.
const SLOT_EVERY: u32 = 12;
/// Freeze frames for a broken poise, a parry and a kill.
const HITSTOP_BREAK: u16 = 8;
const HITSTOP_PARRY: u16 = 8;
const HITSTOP_KILL: u16 = 6;
/// The slowest and fastest a time-locked bolt flies, as shares of its speed.
const BOLT_PACE: (f64, f64) = (0.5, 1.6);
/// How far off his facing (radians) the soft lock turns his swing onto a foe.
const ASSIST_ANGLE: f64 = 0.8;
/// A blow from outside the frame data (a bomb, an archangel's fire, a spirit) in the vaults:
/// its damage and poise damage (in the ring such blows do nothing: the lab is pure).
const STRAY_DAMAGE: f64 = 20.0;
const STRAY_POISE: f64 = 30.0;
/// How far the hunt's flow field reaches from him, in steps.
const FLOW_REACH: u16 = 48;
/// The vaults' pace: threat points and frames between attack tokens (the ring keeps the lab's
/// `threat::CAPACITY` and `threat::CADENCE`). A pack is a fight, not a pile-on.
pub const VAULT_CAPACITY: u8 = 2;
pub const VAULT_CADENCE: u32 = 60;
/// The realm's weight on its foes: health and damage multiples of their breeds (Greed is the
/// first realm; the realms after it raise these as the keys held mount up).
pub const VAULT_HP: f64 = 0.85;
pub const VAULT_DAMAGE: f64 = 0.8;
/// The chance a foe slain in the vaults drops a health potion (Diablo's globes, by way of the chests' drops).
pub const HEAL_DROP: f64 = 0.2;

/// The cry a foe gives as it begins a strike (its tell, to the ear): the tengu's steel and the
/// cobra's hiss, the dire wolf's growl, the jorogumo's shriek; the rest by their move.
fn tell_sfx(kind: EnemyKind, mv: MoveId) -> Sfx {
    match (kind, mv) {
        (EnemyKind::Tengu | EnemyKind::Cobra, _) => Sfx::Whoosh,
        (EnemyKind::DireWolf, _) => Sfx::Growl,
        (EnemyKind::Jorogumo, _) => Sfx::Shriek,
        (_, MoveId::Slam | MoveId::Sweep) => Sfx::Growl,
        (_, MoveId::Lunge) => Sfx::Howl,
        _ => Sfx::Cackle,
    }
}

/// Where the brawl is fought.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Field {
    /// The gym's Sparring Ring: bouts, no death, no wear.
    Ring,
    /// The Vaults of Mammon: packs in their rooms, his real health, his shield and his blade.
    Vault,
}

/// What a fight has seen (the telemetry of COMBAT.md §6, phase 2).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BoutStats {
    pub frames: u32,
    pub dealt: f64,
    pub taken: f64,
    /// The hero's blows that landed, and foes' that landed on him.
    pub hits: u32,
    pub hurt: u32,
    pub parries: u32,
    pub blocks: u32,
    pub evades: u32,
    pub breaks: u32,
    /// Blows that caught him in his own swing, and guards broken.
    pub caught: u32,
    pub guard_breaks: u32,
    /// Two foes' strikes reaching him inside the reaction window of each other.
    pub overlaps: u32,
    /// Foes slain.
    pub kills: u32,
}

#[derive(Clone, Debug)]
pub struct Brawl {
    pub field: Field,
    /// The hero's fighter.
    pub hero: Fighter,
    /// The aim of his swing, committed on its first active frame, and the duelists it has struck.
    pub aim: V,
    struck: Vec<u32>,
    pub threat: ThreatManager,
    pub bolts: Vec<Bolt>,
    clock: FrameClock,
    pub stats: BoutStats,
    next_id: u32,
    /// The frames the foes' strikes landed on, for counting overlaps.
    strikes: Vec<u32>,
    /// His gear as numbers (COMBAT.md §8), and whether his second wind is spent.
    pub kit: Kit,
    wind_used: bool,
    /// The frame readout (F5).
    pub readout: bool,
    /// The foes fight: the ring's bout is under way (always, in the vaults).
    pub live: bool,
    /// The hunt's way round the walls, drawn from his tile.
    flow: Flow,
    /// What the foes' blows are multiplied by here (the realm's weight; 1 in the ring).
    pub foe_damage: f64,
}

impl Brawl {
    pub fn new(field: Field) -> Self {
        let mut threat = ThreatManager::new();
        if field == Field::Vault {
            threat.capacity = VAULT_CAPACITY;
            threat.cadence = VAULT_CADENCE;
        }
        Self {
            field,
            hero: Fighter::hero(),
            aim: V::new(1.0, 0.0),
            struck: Vec::new(),
            threat,
            bolts: Vec::new(),
            clock: FrameClock::default(),
            stats: BoutStats::default(),
            next_id: 1,
            strikes: Vec::new(),
            kit: Kit::training(),
            wind_used: false,
            readout: field == Field::Ring,
            live: field == Field::Vault,
            flow: Flow::default(),
            foe_damage: if field == Field::Vault { VAULT_DAMAGE } else { 1.0 },
        }
    }

    /// A fresh id for a duelist.
    pub fn next_id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id - 1
    }

    /// A fresh fight: his fighter with `kit`, no strikes, no bolts, no numbers yet.
    pub fn reset(&mut self, kit: Kit) {
        self.kit = kit;
        self.hero = Fighter::with_kit(&kit);
        self.threat.clear();
        self.bolts.clear();
        self.strikes.clear();
        self.struck.clear();
        self.stats = BoutStats::default();
        self.wind_used = false;
    }

    /// He rises again (the vaults after a fall): a fresh fighter and field, the fight's numbers kept.
    pub fn respawn(&mut self, kit: Kit) {
        let stats = self.stats;
        self.reset(kit);
        self.stats = stats;
    }

    /// A foe's strike reached the hero on frame `f` (a blow meeting him, a bolt touching him, however
    /// he met it): one landing inside the reaction window of another counts as an overlap.
    fn land(&mut self, f: u32) {
        if self.strikes.iter().any(|&g| f.abs_diff(g) < SPACING) {
            self.stats.overlaps += 1;
        }
        self.strikes.push(f);
        self.strikes.retain(|&g| f.saturating_sub(g) < 120);
    }

    /// His kit changed (a dulled blade, a broken shield, a potion): the numbers change, the state does not.
    fn refit(&mut self, kit: Kit) {
        if kit == self.kit {
            return;
        }
        self.kit = kit;
        let f = Fighter::with_kit(&kit);
        self.hero.table = f.table;
        self.hero.parry_to = f.parry_to;
        self.hero.evade_total = f.evade_total;
        self.hero.break_frames = f.break_frames;
    }
}

impl Game {
    /// His fighting goes by frame data here: in the Sparring Ring, or anywhere in the vaults.
    pub fn brawling(&self) -> bool {
        match self.brawl.as_ref().map(|b| b.field) {
            Some(Field::Ring) => self.in_ring(),
            Some(Field::Vault) => true,
            None => false,
        }
    }

    /// The duelists' indices in `self.enemies`.
    pub(super) fn duelists(&self) -> Vec<usize> {
        (0..self.enemies.len()).filter(|&i| self.enemies[i].duel.is_some()).collect()
    }

    /// His blade, shield and path as numbers, and what his potions add (strength and Ragnarok hit
    /// harder; haste and Ragnarok quicken him).
    pub fn hero_kit(&self) -> Kit {
        let h = &self.hero;
        let mut k = Kit::of(self.hero_kind, h.blade, h.edge, h.shield.map(|s| s.tier), h.path);
        if h.fx.strength > 0.0 {
            k.strike.power *= 1.25;
        }
        if h.fx.haste > 0.0 {
            k.strike.tempo *= 0.75;
        }
        if h.fx.ragnarok > 0.0 {
            k.strike.power *= 2.0;
            k.strike.tempo *= 0.67;
        }
        k
    }

    /// His buttons: attack and evade into his fighter's buffer; the guard as held.
    pub(super) fn brawl_input(&mut self, attack: bool, evade: bool, guard: bool) {
        let Some(b) = self.brawl.as_mut() else { return };
        if attack {
            b.hero.press(Input::Attack);
        }
        if evade {
            b.hero.press(Input::Evade);
        }
        b.hero.guard_held = guard;
    }

    /// The brawl's frames, as many as the step holds.
    pub(super) fn update_brawl(&mut self, dt: f64) {
        if !self.brawling() {
            return;
        }
        let frames = self.brawl.as_mut().map(|b| b.clock.advance(dt)).unwrap_or(0);
        for _ in 0..frames {
            self.brawl_frame();
            if !self.hero.alive {
                return;
            }
        }
    }

    /// One 60 Hz frame: his kit and fighter, then (while the foes fight) theirs, their bolts and the threat manager.
    fn brawl_frame(&mut self) {
        let kit = self.hero_kit();
        let Some(b) = self.brawl.as_mut() else { return };
        b.refit(kit);
        let cost = kit.swing_cost();
        b.hero.can_attack = self.hero.tireless() || self.hero.stamina + 1e-9 >= cost;
        let events = b.hero.step();
        for e in events {
            match e {
                Event::Began(mv) => {
                    self.hero.exert(cost);
                    let d = self.brawl.as_ref().map(|b| b.hero.data(mv)).unwrap_or(mv.data());
                    self.view.sword_slash_timed(secs(d.startup) as f32, secs(d.active) as f32, secs(d.recovery) as f32);
                    self.audio.play(Sfx::Slash);
                    let aim = self.assisted_aim(d.reach);
                    if let Some(b) = self.brawl.as_mut() {
                        b.struck.clear();
                        b.aim = aim;
                    }
                }
                Event::Active(mv) => {
                    // The aim is committed on the first active frame; a thrown finisher flies down it.
                    let d = self.brawl.as_ref().map(|b| b.hero.data(mv)).unwrap_or(mv.data());
                    let aim = self.assisted_aim(d.reach);
                    let hp = self.hero.pos();
                    if let Some(b) = self.brawl.as_mut() {
                        b.aim = aim;
                        if let (true, Some(t)) = (d.projectile, b.kit.strike.thrown) {
                            b.bolts.push(Bolt {
                                pos: math::add(hp, math::scale(aim, 0.5)),
                                vel: math::scale(aim, t.speed),
                                life: (t.range / t.speed * 60.0) as u16,
                                damage: d.damage,
                                poise: d.poise,
                                reflected: true,
                                touched: false,
                                due: None,
                                tint: 0xffc850,
                            });
                        }
                    }
                }
                Event::Evade if self.hero.try_jump(&self.level.grid) => {
                    self.audio.play(Sfx::Jump);
                    if let Some(b) = self.brawl.as_mut() {
                        b.hero.evade();
                    }
                }
                _ => {}
            }
        }
        let Some(b) = self.brawl.as_ref() else { return };
        self.hero.root = b.hero.root();
        self.hero.blocking = matches!(b.hero.state, State::Guard { .. } | State::Reel { kind: Reel::Blockstun, .. });
        let live = b.live;
        self.hero_strikes();
        if live {
            self.foes_frame();
        }
        if let Some(b) = self.brawl.as_mut() {
            b.threat.tick();
            if b.live {
                b.stats.frames += 1;
            }
        }
    }

    /**
     * The way his swing goes: where he faces, turned onto the nearest duelist within its reach
     * and a tile more, and no more than `ASSIST_ANGLE` off (a soft lock, as the sticks and the four
     * keys are coarse). He turns to it too.
     */
    fn assisted_aim(&mut self, reach: f64) -> V {
        let hp = self.hero.pos();
        let aim = self.hero.aim();
        let best = self
            .enemies
            .iter()
            .filter_map(|e| e.duel.as_deref())
            .filter(|d| d.alive())
            .map(|d| (math::sub(d.pos, hp), d.radius()))
            .filter(|(to, r)| math::len(*to) <= reach + 1.0 + r && math::dot(math::norm(*to), aim) >= ASSIST_ANGLE.cos())
            .min_by(|a, b| math::len(a.0).total_cmp(&math::len(b.0)));
        let Some((to, _)) = best else { return aim };
        let to = math::norm(to);
        self.hero.heading = math::angle(to);
        to
    }

    /// On his swing's active frames, every duelist in its cone that it has not struck yet takes the blow.
    fn hero_strikes(&mut self) {
        let Some(b) = self.brawl.as_ref() else { return };
        let Some((mv, Phase::Active, _)) = b.hero.attack() else { return };
        let d = b.hero.data(mv);
        if d.projectile {
            return; // thrown, not swung
        }
        let strike = b.kit.strike;
        let hp = self.hero.pos();
        let aim = b.aim;
        let targets: Vec<usize> = self
            .duelists()
            .into_iter()
            .filter(|&i| {
                let du = self.enemies[i].duel.as_ref().unwrap();
                du.alive() && !b.struck.contains(&du.id) && math::in_cone(hp, aim, du.pos, du.radius(), d.reach, d.half_angle)
            })
            .collect();
        let landed = targets.len();
        for i in targets {
            let du = self.enemies[i].duel.as_ref().unwrap();
            let id = du.id;
            // From behind (or the flank), the fang or the flanking blow bites deeper.
            let off = math::dot(du.facing, math::norm(math::sub(hp, du.pos))).clamp(-1.0, 1.0).acos();
            let back = if off >= strike.back_from { strike.back } else { 1.0 };
            if let Some(b) = self.brawl.as_mut() {
                b.struck.push(id);
            }
            let hit = Hit { damage: d.damage * back, poise: d.poise, hitstun: d.hitstun, pierce: strike.pierce, crit: strike.crit };
            self.duel_hit(i, hit, d.hitstop, hp);
        }
        if landed > 0 {
            if strike.blood > 0.0 {
                // A cursed blade drinks from the hand that holds it.
                self.hero.hp = (self.hero.hp - (strike.blood * landed as f64) as i32).max(1);
            }
            if self.brawl.as_ref().is_some_and(|b| b.field == Field::Vault) {
                // In the vaults his edge wears as in any real fight (the ring's never does).
                self.dull_blade();
            }
        }
    }

    /// A blow lands on duelist `i` from `from`: damage through its fighter, the flash and shove, the
    /// numbers, its pack woken, and death (in the vaults with a kill's spoils).
    fn duel_hit(&mut self, i: usize, hit: Hit, hitstop: u16, from: V) {
        self.wake_den_of(i);
        let Some(du) = self.enemies[i].duel.as_mut() else { return };
        let r = du.f.take_hit(&hit);
        du.hp -= r.damage;
        du.shove(from, matches!(r.effect, Effect::Flinched(_) | Effect::Broken));
        let (p, dead) = (du.pos, du.hp <= 0.0);
        if dead {
            du.f.kill();
        }
        let mut stop = hitstop;
        let (text, color) = match r.effect {
            Effect::Broken => ("BREAK!".to_string(), 0xffd040),
            Effect::Armored => (format!("{}", r.damage), 0x9aa4b0),
            _ if hit.damage > 0.0 && r.damage > hit.damage.round() => (format!("{}!", r.damage), 0xff9a3a),
            _ => (format!("{}", r.damage), 0xffffff),
        };
        self.floater(p.x, p.y - 0.7, text, 0.8, Some(color));
        let vault = if let Some(b) = self.brawl.as_mut() {
            b.stats.dealt += r.damage;
            b.stats.hits += 1;
            if r.effect == Effect::Broken {
                b.stats.breaks += 1;
            }
            if dead {
                b.stats.kills += 1;
            }
            b.field == Field::Vault
        } else {
            false
        };
        match r.effect {
            Effect::Broken => {
                stop = stop.max(HITSTOP_BREAK);
                self.audio.play(Sfx::Clang);
                self.view.sparkle(&self.level.grid, p.x, p.y, 0.9, 0xffd040, 26);
                self.view.shake(0.25);
            }
            Effect::Armored => {
                self.audio.play(Sfx::Clang);
                self.view.spark(&self.level.grid, p.x, p.y, 0xc8d0da);
            }
            _ => {
                self.audio.play(Sfx::ArrowHit);
                self.view.spark(&self.level.grid, p.x, p.y, 0xffb0b0);
                self.view.shake(0.08);
            }
        }
        if dead && self.brawl.as_ref().is_some_and(|b| b.kit.terror) {
            // Terrify (Ragnar's level 8): the foes round the kill lose half their poise.
            let r2 = crate::logic::progression::path::TERROR_RADIUS.powi(2);
            for e in self.enemies.iter_mut() {
                if let Some(o) = e.duel.as_deref_mut().filter(|o| o.alive() && crate::logic::types::dist_sq(o.pos, p) <= r2) {
                    let half = o.f.max_poise / 2.0;
                    o.f.shake(half);
                }
            }
        }
        if dead {
            stop = stop.max(HITSTOP_KILL);
            let color = self.enemies[i].b.color;
            self.enemies[i].b.mode = EnemyMode::Eaten;
            self.view.dust(&self.level.grid, p.x, p.y, color, 18, 2.6);
            self.view.sparkle(&self.level.grid, p.x, p.y, 0.6, 0xfff1c0, 12);
            self.view.shake(0.18);
            if vault {
                self.kill_spoils(p, crate::logic::systems::POINTS_KILL);
                if self.rng.next() < HEAL_DROP {
                    let at = P::new(p.x.round() as i32, p.y.round() as i32);
                    if let Some(v) = self.vault.as_mut() {
                        let item = crate::logic::vault::DropItem::Potion(crate::logic::types::PotionKind::Health);
                        v.drops.push(crate::logic::vault::Drop {
                            at,
                            from: at,
                            item,
                            rarity: crate::logic::vault::Rarity::Common,
                            age: 0.0,
                            taken: false,
                        });
                    }
                }
            } else {
                self.audio.play_kill(0);
            }
        }
        self.hitstop = self.hitstop.max(secs(stop));
    }

    /// Something outside the frame data reached duelist `i` (a bomb, an archangel's fire, a spirit):
    /// in the ring nothing (the lab is pure); in the vaults a plain blow through its fighter.
    pub(super) fn stray_blow(&mut self, i: usize) {
        if self.brawl.as_ref().is_some_and(|b| b.field == Field::Vault) {
            let from = self.hero.pos();
            self.duel_hit(i, Hit { damage: STRAY_DAMAGE, poise: STRAY_POISE, hitstun: 12, ..Default::default() }, 2, from);
        }
    }

    /// Wakes duelist `i`'s pack (every foe of its den), if it was asleep.
    fn wake_den_of(&mut self, i: usize) {
        let Some(den) = self.enemies.get(i).and_then(|e| e.duel.as_ref()).and_then(|d| d.den) else { return };
        if den.awake && !den.returning {
            return;
        }
        for e in self.enemies.iter_mut() {
            if let Some(d) = e.duel.as_deref_mut().and_then(|d| d.den.as_mut()).filter(|d| d.lo == den.lo && d.hi == den.hi) {
                d.awake = true;
                d.returning = false;
            }
        }
    }

    /**
     * The vaults' packs: a den stirs when he steps into it or comes near one of its foes; a hunt is
     * given up (its foes walk home, tokens returned) when he is far from the den, and taken up
     * again if he comes back near.
     */
    fn update_dens(&mut self) {
        let hp = self.hero.pos();
        let mut stirred: Vec<Den> = Vec::new();
        for e in &self.enemies {
            let Some(d) = e.duel.as_deref().filter(|d| d.alive()) else { continue };
            let Some(den) = d.den.filter(|den| !den.awake) else { continue };
            let near = den.contains(hp) || crate::logic::types::dist_sq(d.pos, hp) < WAKE_RANGE * WAKE_RANGE;
            if near && !stirred.iter().any(|s| s.lo == den.lo && s.hi == den.hi) {
                stirred.push(den);
            }
        }
        for den in &stirred {
            let c = den.centre();
            self.floater(c.x, c.y, "THE PACK STIRS", 1.4, Some(0xffc85a));
            self.audio.play(Sfx::Growl);
        }
        let mut called_off = Vec::new();
        for e in self.enemies.iter_mut() {
            let Some(d) = e.duel.as_deref_mut() else { continue };
            let pos = d.pos;
            let id = d.id;
            let Some(den) = d.den.as_mut() else { continue };
            if !den.awake && stirred.iter().any(|s| s.lo == den.lo && s.hi == den.hi) {
                den.awake = true;
            }
            if den.awake && !den.returning && crate::logic::types::dist_sq(den.centre(), hp) > LEASH * LEASH {
                den.returning = true;
                called_off.push(id);
            } else if den.returning && crate::logic::types::dist_sq(pos, hp) < WAKE_RANGE * WAKE_RANGE {
                den.returning = false;
            }
            if den.returning {
                d.token = false;
                d.slot = None;
            }
        }
        if let Some(b) = self.brawl.as_mut() {
            for id in called_off {
                b.threat.release(id);
            }
        }
    }

    /// The foes' frame: their fighters, the tokens, the sectors, their minds, their blows, the bolts.
    fn foes_frame(&mut self) {
        let field = self.brawl.as_ref().map(|b| b.field).unwrap_or(Field::Ring);
        if field == Field::Vault {
            self.update_dens();
        }
        let hp = self.hero.pos();
        let swing = matches!(self.brawl.as_ref().and_then(|b| b.hero.attack()), Some((_, Phase::Startup, _)));
        let sense = Sense { hero: hp, aim: self.hero.aim(), swing };
        let ids = self.duelists();
        let frame = self.brawl.as_ref().map(|b| b.threat.frame).unwrap_or(0);

        // Their fighters: a hex looses its bolt; a move run its course gives its token back.
        let mut bolts = Vec::new();
        let mut released = Vec::new();
        for &i in &ids {
            let du = self.enemies[i].duel.as_mut().unwrap();
            for e in du.step() {
                match e {
                    Event::Active(mv) => {
                        if mv.data().projectile {
                            let at = math::add(du.pos, math::scale(du.aim, 0.5));
                            bolts.push(Bolt {
                                pos: at,
                                vel: math::scale(du.aim, BOLT_SPEED),
                                life: BOLT_LIFE,
                                damage: mv.data().damage * du.breed.damage * self.brawl.as_ref().map(|b| b.foe_damage).unwrap_or(1.0),
                                poise: 0.0,
                                reflected: false,
                                touched: false,
                                due: self.brawl.as_ref().and_then(|b| b.threat.landing(du.id)),
                                tint: bolt_tint(du.breed.kind),
                            });
                        }
                    }
                    Event::Ended(_) => {
                        du.token = false;
                        released.push(du.id);
                    }
                    _ => {}
                }
            }
            // A token holder that reels (or a spent token's timeout) gives it back.
            let b = self.brawl.as_ref().unwrap();
            if du.token && (du.f.reeling().is_some() || du.f.is_dead() || b.threat.expired(du.id)) {
                du.token = false;
                du.rest = REVOKE_REST;
                released.push(du.id);
            }
        }
        if !bolts.is_empty() {
            self.audio.play(Sfx::Zap);
        }
        let b = self.brawl.as_mut().unwrap();
        for id in released {
            b.threat.release(id);
        }
        b.bolts.extend(bolts);

        // Tokens: the engaged ask, the manager grants (one a frame at most); holders on their way update their plans.
        let mut requests = Vec::new();
        for &i in &ids {
            let du = self.enemies[i].duel.as_ref().unwrap();
            if !du.engaged() {
                continue;
            }
            if du.token {
                b.threat.project(du.id, du.eta(&sense));
                continue;
            }
            if du.f.is_neutral() && du.rest == 0 && du.backstep == 0 {
                let to = math::sub(du.pos, hp);
                requests.push(Request {
                    id: du.id,
                    cost: du.stats().cost,
                    eta: du.eta(&sense),
                    dist: math::len(to),
                    front: math::dot(sense.aim, math::norm(to)),
                    wait: du.wait,
                });
            }
        }
        if let Some(id) = b.threat.grant(&requests) {
            if let Some(i) = ids.iter().copied().find(|&i| self.enemies[i].duel.as_ref().unwrap().id == id) {
                self.enemies[i].duel.as_mut().unwrap().token = true;
            }
        }

        // The ground: the ring's square, or the vaults' tiles and the hunt's flow field from his tile.
        let mut flow = std::mem::take(&mut b.flow);
        let ho = P::new(hp.x.round() as i32, hp.y.round() as i32);
        if field == Field::Vault && (flow.from != ho || flow.dist(ho) != 0) {
            flow = Flow::new(&self.level.grid, ho, FLOW_REACH);
        }
        let (lo, hi) = match field {
            Field::Ring => gym::ring_bounds(),
            Field::Vault => (V::new(0.0, 0.0), V::new(self.level.grid.width as f64 - 1.0, self.level.grid.height as f64 - 1.0)),
        };
        let arena = Arena { lo, hi };
        let tiles = GridTerrain { grid: &self.level.grid, flow: &flow };
        let terrain: &dyn Terrain = match field {
            Field::Ring => &arena,
            Field::Vault => &tiles,
        };

        // Sectors for the waiting.
        if frame.is_multiple_of(SLOT_EVERY) {
            let waiters: Vec<Waiter> = ids
                .iter()
                .map(|&i| self.enemies[i].duel.as_ref().unwrap())
                .filter(|d| d.engaged() && !d.token)
                .map(|d| Waiter { id: d.id, pos: d.pos, ring: d.stats().ring, prev: d.slot.map(|s| s.sector) })
                .collect();
            // (In the ring only the square's edge counts, as the lab was tuned; in the vaults the walls.)
            let open = |p: V| field == Field::Ring || terrain.open(p);
            let slots = assign_slots(hp, sense.aim, &waiters, lo, hi, &open);
            for &i in &ids {
                let du = self.enemies[i].duel.as_mut().unwrap();
                if du.engaged() {
                    du.slot = slots.iter().copied().find(|s| s.id == du.id);
                }
            }
        }

        // Their minds: each moves, and a token holder in reach asks the commit gate to strike.
        let bodies: Vec<(u32, V, f64)> =
            ids.iter().map(|&i| self.enemies[i].duel.as_ref().unwrap()).filter(|d| d.alive()).map(|d| (d.id, d.pos, d.radius())).collect();
        for &i in &ids {
            let du = self.enemies[i].duel.as_mut().unwrap();
            if du.den.is_some_and(|d| !d.awake) {
                continue; // asleep
            }
            let near: Vec<(V, f64)> = bodies
                .iter()
                .filter(|o| o.0 != du.id && (o.1.x - du.pos.x).abs() < 2.0 && (o.1.y - du.pos.y).abs() < 2.0)
                .map(|o| (o.1, o.2))
                .collect();
            let want = du.think(&sense, terrain, &near);
            let p = du.pos;
            self.enemies[i].m.tile = P::new(p.x.round() as i32, p.y.round() as i32);
            let Some(mv) = want else { continue };
            let du = self.enemies[i].duel.as_mut().unwrap();
            let b = self.brawl.as_mut().unwrap();
            let lands = du.lands_in(mv, hp);
            if b.threat.may_commit(du.id, lands) && du.begin(mv) {
                b.threat.commit(du.id, lands);
                let sfx = tell_sfx(du.breed.kind, mv);
                self.audio.play(sfx);
            }
        }
        if let Some(b) = self.brawl.as_mut() {
            b.flow = flow;
        }

        // Their blows on the hero.
        for &i in &ids {
            let du = self.enemies[i].duel.as_ref().unwrap();
            let Some(mv) = du.f.active() else { continue };
            let d = mv.data();
            if du.struck || d.projectile || !math::in_cone(du.pos, du.aim, hp, HERO_RADIUS, d.reach, d.half_angle) {
                continue;
            }
            let scale = self.brawl.as_ref().map(|b| b.foe_damage).unwrap_or(1.0);
            let (from, damage) = (du.pos, d.damage * du.breed.damage * scale);
            self.enemies[i].duel.as_mut().unwrap().struck = true;
            if self.blow_on_hero(from, damage, d.hitstop, false) == Defense::Parried {
                let guard = self.brawl.as_ref().map(|b| b.kit.guard).unwrap_or(crate::logic::combat::kit::Guard::TRAINING);
                let du = self.enemies[i].duel.as_mut().unwrap();
                du.f.deflect(guard.deflect, guard.riposte);
                du.flash = 8;
                if guard.thorns > 0.0 {
                    let hp = self.hero.pos();
                    self.duel_hit(i, Hit { damage: guard.thorns, ..Default::default() }, 2, hp);
                }
            }
            if !self.hero.alive {
                return;
            }
        }

        self.update_bolts(&ids);

        // The ring's bout is won when its last foe falls.
        if field == Field::Ring {
            let alive = self.enemies.iter().filter_map(|e| e.duel.as_ref()).filter(|d| d.alive()).count();
            if alive == 0 && self.brawl.as_ref().is_some_and(|b| b.live) {
                self.finish_bout(true);
            }
        }
    }

    /**
     * A foe's blow (or bolt) from `from` reaches the hero: his fighter decides how he meets it
     * (evade, parry, block, a broken guard, or open), and his health pays for it. In the ring he
     * is never slain (the bout is lost); in the vaults he is, and his shield wears as it blocks.
     */
    fn blow_on_hero(&mut self, from: V, damage: f64, hitstop: u16, bolt: bool) -> Defense {
        let hp = self.hero.pos();
        let from_front = math::dot(self.hero.aim(), math::norm(math::sub(from, hp))) > 0.0;
        let airborne = self.hero.airborne();
        let stamina = self.hero.stamina;
        let Some(b) = self.brawl.as_mut() else { return Defense::Open };
        let swinging = b.hero.attack().is_some();
        let vault = b.field == Field::Vault;
        // Whatever reaches him counts as a landing, however he meets it.
        let f = b.threat.frame;
        b.land(f);
        let kit = b.kit;
        let cost = if bolt && kit.guard.bolt_proof { 0.0 } else { damage * kit.guard.cost };
        let def = b.hero.defend(airborne, from_front, stamina, cost);
        let mut loss = 0.0;
        let mut wear = 0.0;
        match def {
            Defense::Immune => {}
            Defense::Evaded => {
                b.stats.evades += 1;
                self.floater(hp.x, hp.y - 0.9, "EVADED", 0.6, Some(0x9ad1ff));
            }
            Defense::Parried => {
                b.stats.parries += 1;
                // The aegis (the tenth shield) gives him back a little with every parry.
                self.hero.hp = (self.hero.hp + kit.guard.aegis as i32).min(self.hero.max_hp);
                self.hitstop = self.hitstop.max(secs(HITSTOP_PARRY));
                self.audio.play(Sfx::Clang);
                self.view.sparkle(&self.level.grid, (hp.x + from.x) / 2.0, (hp.y + from.y) / 2.0, 0.9, 0xffe070, 30);
                self.floater(hp.x, hp.y - 0.9, "PARRY!", 0.9, Some(0xffd040));
            }
            Defense::Blocked => {
                b.stats.blocks += 1;
                wear = damage;
                self.hero.drain(cost);
                self.audio.play(Sfx::Clang);
                self.view.spark(&self.level.grid, hp.x, hp.y, 0xc8d0da);
                self.floater(hp.x, hp.y - 0.9, "BLOCKED", 0.6, Some(0xc8d0da));
            }
            Defense::GuardBroken => {
                b.stats.guard_breaks += 1;
                loss = (damage * kit.guard.break_share * kit.taken).round();
                wear = damage * (1.0 - kit.guard.break_share);
                self.hero.drain(self.hero.stamina);
                self.audio.play(Sfx::ShieldBreak);
                self.floater(hp.x, hp.y - 0.9, "GUARD BROKEN", 1.0, Some(0xff7a3a));
                self.view.shake(0.3);
            }
            Defense::Open => {
                if swinging {
                    b.stats.caught += 1;
                }
                let r = b.hero.take_hit(&Hit { damage, hitstun: math::hero_hitstun(damage), ..Default::default() });
                loss = (r.damage * kit.taken).round();
                self.hitstop = self.hitstop.max(secs(hitstop));
                self.audio.play(Sfx::Hurt);
                self.view.shake(0.22);
            }
        }
        if vault && wear > 0.0 {
            self.wear_shield(wear);
        }
        if loss > 0.0 {
            self.hero.hp = (self.hero.hp - loss as i32).max(0);
            self.juice.hurt = 1.0;
            self.juice.hp_flash = 1.0;
            self.juice.hp_flash_color = 0xff4d4d;
            let wind = self.brawl.as_ref().map(|b| (b.kit.second_wind, b.wind_used)).unwrap_or((0.0, true));
            if self.hero.hp == 0 && wind.0 > 0.0 && !wind.1 {
                // The second wind (Valkyrie Call, the last stand, the World Tree): once a fight, the end is put off.
                self.hero.hp = ((self.hero.max_hp as f64 * wind.0).round() as i32).max(1);
                if let Some(b) = self.brawl.as_mut() {
                    b.wind_used = true;
                }
                self.audio.play(Sfx::Heal);
                self.view.sparkle(&self.level.grid, hp.x, hp.y, 1.2, 0xfff1a8, 40);
                self.floater(hp.x, hp.y - 1.4, "SECOND WIND", 1.4, Some(0xfff1a8));
            }
            self.floater(hp.x, hp.y - 0.5, format!("-{loss}"), 0.9, Some(0xff4a3a));
            let down = self.hero.hp == 0;
            let live = self.brawl.as_ref().is_some_and(|b| b.live);
            if let Some(b) = self.brawl.as_mut() {
                b.stats.taken += loss;
                b.stats.hurt += 1;
            }
            if down && live {
                if vault {
                    self.hero.alive = false;
                    self.kill_hero();
                } else {
                    self.finish_bout(false);
                }
            }
        }
        def
    }

    /// His shield takes `amount` of wear from a blow it caught; worn through, it breaks for good
    /// (unless it is too well made to), and his guard is bare hands until the old man reforges it.
    fn wear_shield(&mut self, amount: f64) {
        let Some(s) = self.hero.shield.as_mut() else { return };
        let floor = if crate::logic::progression::ward(s.tier).unbreakable { 1 } else { 0 };
        s.durability = (s.durability - amount.round() as i32).max(floor);
        if s.durability <= 0 {
            self.hero.shield = None;
            self.shatter_shield();
        }
    }

    /// Hex bolts fly; one meets the hero (or, thrown by him or parried back, the foes); they burn out
    /// at the ring's edge or against a wall.
    fn update_bolts(&mut self, ids: &[usize]) {
        let hp = self.hero.pos();
        let Some(b) = self.brawl.as_mut() else { return };
        let ring = b.field == Field::Ring;
        let (lo, hi) = gym::ring_bounds();
        let now = b.threat.frame;
        let mut bolts = std::mem::take(&mut b.bolts);
        for b in bolts.iter_mut() {
            // Time-locked: on its way in, it paces itself to touch him on its planned frame.
            if let (Some(due), false, false) = (b.due, b.reflected, b.touched) {
                let dir = math::norm(b.vel);
                let along = math::dot(dir, math::sub(hp, b.pos)) - BOLT_RADIUS - HERO_RADIUS;
                if along > 0.0 {
                    let frames = (due + 1).saturating_sub(now).max(1) as f64;
                    let speed = (along / frames * 60.0).clamp(BOLT_SPEED * BOLT_PACE.0, BOLT_SPEED * BOLT_PACE.1);
                    b.vel = math::scale(dir, speed);
                }
            }
            b.pos = math::add(b.pos, math::scale(b.vel, 1.0 / 60.0));
            b.life = b.life.saturating_sub(1);
            let out = if ring {
                b.pos.x < lo.x || b.pos.x > hi.x || b.pos.y < lo.y || b.pos.y > hi.y
            } else {
                !self.level.grid.is_walkable(b.pos.x.round() as i32, b.pos.y.round() as i32, false)
            };
            if out {
                b.life = 0;
            }
            if b.life == 0 {
                continue;
            }
            if !b.reflected {
                if !b.touched && math::len(math::sub(b.pos, hp)) <= BOLT_RADIUS + HERO_RADIUS {
                    b.touched = true;
                    let from = math::sub(b.pos, b.vel);
                    if self.blow_on_hero(from, b.damage, 3, true) == Defense::Parried {
                        // Turned back at the foes, faster, and harder as his shield is better.
                        let reflect = self.brawl.as_ref().map(|b| b.kit.guard.reflect).unwrap_or(1.0);
                        b.vel = math::scale(b.vel, -REFLECT_SPEED);
                        b.reflected = true;
                        b.damage *= reflect;
                        b.poise = REFLECT_POISE;
                        b.tint = 0xffc850;
                    } else if !self.hero.airborne() {
                        b.life = 0;
                    }
                    if !self.hero.alive {
                        return;
                    }
                }
                continue;
            }
            let hit = ids.iter().copied().find(|&i| {
                let du = self.enemies[i].duel.as_ref().unwrap();
                du.alive() && math::len(math::sub(b.pos, du.pos)) <= BOLT_RADIUS + du.radius()
            });
            if let Some(i) = hit {
                b.life = 0;
                let d = MoveId::Hex.data();
                self.duel_hit(i, Hit { damage: b.damage, poise: b.poise, hitstun: 20, ..Default::default() }, d.hitstop, b.pos);
            }
        }
        bolts.retain(|b| b.life > 0);
        if let Some(b) = self.brawl.as_mut() {
            let fresh = std::mem::take(&mut b.bolts);
            b.bolts = bolts;
            b.bolts.extend(fresh);
        }
    }
}
