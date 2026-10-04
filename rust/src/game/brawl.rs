//! The brawl (COMBAT.md §9): fighting by frame data, wherever it is fought. In the gym's Sparring
//! Ring it is the lab (bouts, see `sparring`); in the Vaults of Mammon it is the game: the packs
//! are duelists asleep in their rooms, the threat manager paces every foe that hunts him, his
//! blows and theirs land on their frames, and his health, his shield and his blade pay for it.
//!
//! The hero's attack, guard and evade go through his `Fighter`, which runs on his kit (his gear
//! as numbers, `combat::kit`), refreshed every frame so a dulled blade, a broken shield or a
//! potion counts at once.
//!
//! The later stages' foes bring their legends (COMBAT.md §11): blows of new shapes (a mark on the
//! floor, a volley, a quake), bolts of their own (spit, fire, foxfire, a root dart that bursts into
//! roots, lightning), what their blows carry (a shove, frostbite, venom, a drain), the roar that
//! dooms him, the dead raised, illusions, a dragon that begs for a second blow, a ring that turns
//! blades, fire left on the floor. What they do to him is his `Ails`; none of it slows him, holds
//! him or roots him. Nor does a blow itself: struck, blocking or with his guard broken he walks at
//! his whole pace (his fighter's reel only keeps him from swinging or guarding for its frames).
//!
//! A prince (COMBAT.md §13) fights on the same rules: his phases change as his health falls, his
//! coins lie on the floor and burst (`Trap`), and his grasp is the one thing that holds the hero:
//! his feet, not his hands, until a blow on the prince breaks it.

use super::feel::Met;
use super::Game;
use crate::audio::voices::matter_of as sound_matter;
use crate::audio::{Cry, HitSound, Sfx};
use crate::logic::combat::ails::{Ails, Flash, FlashKind, Hazard, Wound, DOOM_FRAMES, FIRE_FRAMES, FIRE_RADIUS, FROST_FRAMES, SHOVE_SPEED};
use crate::logic::combat::archetype::Power;
use crate::logic::combat::duelist::{Arena, Bolt, BoltLook, Cue, Den, Flow, GridTerrain, Lore, Sense, Terrain, HERO_RADIUS, REFLECT_TINT};
use crate::logic::combat::duelist::{CONJURE_AT, MAX_RAISED, PLEA_FRAMES, PLEA_HEALTH, RISE_FRAMES, SPILL_FRAMES};
use crate::logic::combat::frames::{Rider, Shape, BOLT_LIFE, BOLT_RADIUS, BOLT_SPEED, REFLECT_POISE, REFLECT_SPEED};
use crate::logic::combat::fsm::{Defense, Effect, GuardRule, Hit, State};
use crate::logic::combat::kit::Kit;
use crate::logic::combat::prince::{self, Trap, COIN_FLIGHT, DRAIN_EVERY, DRAIN_HEAL, DRAIN_SILVER, FIST, GRASP_HOLD};
use crate::logic::combat::threat::{assign_slots, Request, Waiter, REVOKE_REST, SLACK, SPACING};
use crate::logic::combat::{math, secs, Breed, Duelist, Event, Fighter, FrameClock, Input, MoveId, Phase, Reel, ThreatManager};
use crate::logic::enemy::{spec_of, Enemy, EnemyKind, EnemyMode};
use crate::logic::gym;
use crate::logic::juice::Stop;
use crate::logic::types::{P, V};
use crate::logic::vault::{LEASH, WAKE_RANGE};
use crate::logic::war::WarEvent;

/// Frames between re-dealing the waiting foes' sectors.
const SLOT_EVERY: u32 = 12;
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
/// The spacing of a kitsune's illusions either side of it, and of the dead Hel raises from him.
const CONJURE_SPREAD: f64 = 1.6;
const RAISE_SPREAD: f64 = 1.8;

/// Every blow and wound on the hero printed as it comes, when `VH_TRACE` is set (for tuning: see
/// `--bout` in main.rs).
fn tracing() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VH_TRACE").is_some())
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
    /// Foes' strikes that reached him, however he met them.
    pub landings: u32,
}

/// A foe's blow (or bolt) as it reaches him: from where, how hard, its freeze frames, what the
/// guard may do against it, what it carries, who struck it, and whether it counts as a strike
/// landing (a volley's later bolts do not).
#[derive(Clone, Copy, Debug)]
struct Blow {
    from: V,
    damage: f64,
    hitstop: u16,
    bolt: bool,
    rule: GuardRule,
    rider: Rider,
    by: u32,
    lands: bool,
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
    /// The frame readout (F8).
    pub readout: bool,
    /// The foes fight: the ring's bout is under way (always, in the vaults).
    pub live: bool,
    /// The hunt's way round the walls, drawn from his tile.
    flow: Flow,
    /// What the foes' blows are multiplied by here (the realm's weight; 1 in the ring).
    pub foe_damage: f64,
    /// What the legends have done to him; the floor burning; the marks on the scene.
    pub ails: Ails,
    pub hazards: Vec<Hazard>,
    pub flashes: Vec<Flash>,
    /// The last volley's number, and the last that reached him (it lands once).
    volleys: u32,
    landed_volley: u32,
    /// A realm's multiple on his strike power (Pride's Glory); 1 elsewhere.
    pub might: f64,
    /// A prince's coins on the floor, and their throws' numbers (a throw lands as one strike).
    pub traps: Vec<Trap>,
    /// The prince whose fist holds him (his feet stay where they are until he strikes the prince).
    pub held: Option<u32>,
    /// The duelists whose present move has met him (however he met it): the rest whiff.
    met: Vec<u32>,
    /// The threat pool and cadence this field was made with, before the Drum bounds them.
    base: Option<(u8, u32)>,
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
            ails: Ails::default(),
            hazards: Vec::new(),
            flashes: Vec::new(),
            volleys: 0,
            landed_volley: 0,
            might: 1.0,
            traps: Vec::new(),
            held: None,
            met: Vec::new(),
            base: None,
        }
    }

    /// A fresh id for a duelist.
    pub fn next_id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id - 1
    }

    /// A fresh fight: his fighter with `kit`, no strikes, no bolts, no ails, no numbers yet.
    pub fn reset(&mut self, kit: Kit) {
        self.kit = kit;
        self.hero = Fighter::with_kit(&kit);
        self.threat.clear();
        self.bolts.clear();
        self.strikes.clear();
        self.struck.clear();
        self.stats = BoutStats::default();
        self.wind_used = false;
        self.ails = Ails::default();
        self.hazards.clear();
        self.flashes.clear();
        self.traps.clear();
        self.held = None;
        self.met.clear();
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
        self.stats.landings += 1;
        if self.strikes.iter().any(|&g| f.abs_diff(g) < SPACING) {
            self.stats.overlaps += 1;
        }
        self.strikes.push(f);
        self.strikes.retain(|&g| f.saturating_sub(g) < 120);
    }

    /// The threat pool and cadence this field was made with (taken the first time it is asked).
    pub fn base_pace(&mut self) -> (u8, u32) {
        *self.base.get_or_insert((self.threat.capacity, self.threat.cadence))
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

    /// The index of duelist `id`.
    fn duelist_index(&self, id: u32) -> Option<usize> {
        (0..self.enemies.len()).find(|&i| self.enemies[i].duel.as_ref().is_some_and(|d| d.id == id))
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
        // (In a prince's fist only a blow on him frees his feet: no leap.)
        if evade && b.held.is_none() {
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

    /// One 60 Hz frame: his kit and fighter, then (while the foes fight) theirs, their bolts, the
    /// threat manager, and what the legends do to him.
    fn brawl_frame(&mut self) {
        let mut kit = self.hero_kit();
        kit.strike.power *= self.brawl.as_ref().map(|b| b.might).unwrap_or(1.0);
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
                                look: BoltLook::Blade,
                                tint: REFLECT_TINT,
                                rider: Rider::None,
                                owner: 0,
                                volley: 0,
                                phantom: false,
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
        self.hero.root = if b.held.is_some() { 0.0 } else { b.hero.stride() };
        self.hero.blocking = matches!(b.hero.state, State::Guard { .. } | State::Reel { kind: Reel::Blockstun, .. });
        let live = b.live;
        self.hero_strikes();
        if live {
            self.foes_frame();
            if self.hero.alive {
                self.update_ails();
            }
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
     * keys are coarse). He turns to it too. (Nothing out of his reach draws it, a ghost not raging
     * or an owl aloft; nor a felled dragon begging for the blow that would bring it back.)
     */
    fn assisted_aim(&mut self, reach: f64) -> V {
        let hp = self.hero.pos();
        let aim = self.hero.aim();
        let best = self
            .enemies
            .iter()
            .filter_map(|e| e.duel.as_deref())
            .filter(|d| d.alive() && !d.untouchable() && !d.begging())
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
                du.alive()
                    && !du.untouchable()
                    && !b.struck.contains(&du.id)
                    && math::in_cone(hp, aim, du.pos, du.radius(), d.reach, d.half_angle)
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

    /**
     * A blow lands on duelist `i` from `from`: damage through its fighter (as much of it as its
     * legend lets through), the flash and shove, the numbers, its pack woken, and death (in the
     * vaults with a kill's spoils). An illusion is gone at a touch; a felled ejder struck again
     * rises; one about to fall begs instead; a wounded kitsune conjures its illusions.
     */
    pub(super) fn duel_hit(&mut self, i: usize, mut hit: Hit, hitstop: u16, from: V) {
        self.wake_den_of(i);
        // A rival's old wound, and the elixir he has learnt.
        let grudge = self.enemies[i].duel.as_ref().map(|d| (self.rival_takes(d.id, from), self.rival_dreads(d.id))).unwrap_or((1.0, 1.0));
        hit.damage *= grudge.0;
        hit.poise *= grudge.1;
        let Some(du) = self.enemies[i].duel.as_mut() else { return };
        if !du.alive() || du.untouchable() {
            return;
        }
        let p = du.pos;
        if du.lore.illusion {
            // One blow, and an illusion is a wisp of smoke and a leaf.
            du.f.kill();
            self.enemies[i].b.mode = EnemyMode::Eaten;
            self.flash(Flash::new(FlashKind::Smoke, p));
            self.floater(p.x, p.y - 0.7, "ILLUSION", 0.8, Some(0xd8c8ff));
            self.audio.play(Sfx::Cackle);
            self.view.dust(&self.level.grid, p.x, p.y, 0xd8d0e8, 14, 1.8);
            return;
        }
        if du.begging() {
            // Struck a second time, the dragon rises: a head grows back.
            du.lore.plea = 0;
            du.hp = (du.max_hp * PLEA_HEALTH).round();
            du.f = du.breed.fighter();
            du.flash = 8;
            self.floater(p.x, p.y - 1.4, "STRUCK TWICE: IT RISES AGAIN", 1.6, Some(0xff7a3a));
            self.audio.play(Sfx::BossRoar);
            self.view.sparkle(&self.level.grid, p.x, p.y, 1.2, 0x9aff6a, 24);
            return;
        }
        // A blow on a prince whose fist holds him breaks its hold (the blow lands on him open).
        let freed = du.lore.hold > 0;
        let share = du.taken_from(from);
        du.lore.hold = 0;
        hit.damage *= share;
        hit.poise *= share;
        let r = du.f.take_hit(&hit);
        du.hp -= r.damage;
        // A prince's health fallen into his next phase: its table of moves, rested.
        if du.arch == crate::logic::combat::Archetype::Prince && du.hp > 0.0 {
            let phase = prince::phase_of(du.hp, du.max_hp);
            if phase > du.breed.phase {
                du.breed.phase = phase;
                du.cool = [0; 4];
                du.cues.push(Cue::Phase(phase));
            }
        }
        du.shove(from, matches!(r.effect, Effect::Flinched(_) | Effect::Broken));
        let dead = du.hp <= 0.0;
        let plea = dead && du.breed.power == Power::Plea;
        if plea {
            // Felled, it begs for a second blow: "Bir daha vur!" (but a second blow brings it back).
            du.hp = 0.0;
            du.lore.plea = PLEA_FRAMES;
            du.token = false;
        } else if dead {
            du.f.kill();
        }
        let conjure = !dead
            && du.breed.power == Power::Illusions
            && CONJURE_AT.get(du.lore.conjured as usize).is_some_and(|&at| du.hp < du.max_hp * at);
        if conjure {
            du.lore.conjured += 1;
        }
        let id = du.id;
        let kind = du.breed.kind;
        let (text, color) = match r.effect {
            _ if share == 0.0 => ("STRIKE FROM BEHIND".to_string(), 0xf0b0d0),
            _ if share < 1.0 => (format!("WARDED {}", r.damage), 0x8a929e),
            Effect::Broken => ("BREAK!".to_string(), 0xffd040),
            Effect::Armored => (format!("{}", r.damage), 0x9aa4b0),
            _ if hit.damage > 0.0 && r.damage > hit.damage.round() => (format!("{}!", r.damage), 0xff9a3a),
            _ => (format!("{}", r.damage), 0xffffff),
        };
        self.floater(p.x, p.y - 0.7, text, 0.8, Some(color));
        let hp = self.hero.pos();
        let finisher = from == hp && self.brawl.as_ref().and_then(|b| b.hero.attack()).is_some_and(|a| a.0 == MoveId::L3);
        let dir = math::norm(math::sub(p, from));
        self.war_note(WarEvent::HeroHit { foe: id, kind, effect: r.effect, dmg: r.damage, finisher, dir, at: p, warded: 1.0 - share });
        if let Some(b) = self.brawl.as_mut() {
            b.stats.dealt += r.damage;
            b.stats.hits += 1;
            if r.effect == Effect::Broken {
                b.stats.breaks += 1;
            }
            if plea {
                b.threat.release(id);
            }
        }
        if share < 1.0 {
            // The peri's ring turns the blade: sparks off a body no sword cuts.
            self.audio.play(Sfx::Clang);
            self.view.spark(&self.level.grid, p.x, p.y, 0xffe8a0);
        } else {
            match r.effect {
                Effect::Broken => {
                    self.audio.hit(HitSound::PoiseBreak, sound_matter(kind), kind, Some(p));
                    self.view.sparkle(&self.level.grid, p.x, p.y, 0.9, 0xffd040, 26);
                }
                Effect::Armored => {
                    self.audio.hit(HitSound::Armor, sound_matter(kind), kind, Some(p));
                    self.view.spark(&self.level.grid, p.x, p.y, 0xc8d0da);
                }
                _ => {
                    let crit = hit.damage > 0.0 && r.damage > hit.damage.round();
                    let sound = if finisher { HitSound::Finisher } else { HitSound::Flesh { crit } };
                    self.audio.hit(sound, sound_matter(kind), kind, Some(p));
                    self.view.spark(&self.level.grid, p.x, p.y, 0xffb0b0);
                }
            }
        }
        self.feel_hero_hit(i, kind, r.effect, finisher, dir, p, hitstop);
        if plea {
            self.floater(p.x, p.y - 1.6, "BİR DAHA VUR!", 2.0, Some(0xffd040));
            self.floater(p.x, p.y - 1.1, "\"STRIKE AGAIN!\" IT BEGS", 2.0, Some(0xffe8b0));
            self.audio.play(Sfx::Whimper);
            self.view.shake(0.2);
        }
        if conjure {
            self.conjure(i);
        }
        if freed {
            self.release_grasp();
            let hp = self.hero.pos();
            self.floater(hp.x, hp.y - 1.3, "BROKE FREE", 1.2, Some(0xffe070));
            self.audio.play(Sfx::Clang);
        }
        if dead && !plea {
            if self.brawl.as_ref().is_some_and(|b| b.kit.terror) {
                // Terrify (Ragnar's level 8): the foes round the kill lose half their poise.
                let r2 = crate::logic::progression::path::TERROR_RADIUS.powi(2);
                for e in self.enemies.iter_mut() {
                    if let Some(o) = e.duel.as_deref_mut().filter(|o| o.alive() && crate::logic::types::dist_sq(o.pos, p) <= r2) {
                        let half = o.f.max_poise / 2.0;
                        o.f.shake(half);
                    }
                }
            }
            let last = self.slay(i);
            self.freeze(if last { Stop::LastOfPack } else { Stop::Kill });
        }
    }

    /**
     * Duelist `i` falls: its colour in the dust, the kill counted (in the vaults its spoils, and a
     * chance of a potion), and those who stood by its power fall with it: a kitsune's illusions
     * thin into smoke, the dead Hel raised sink back.
     */
    fn slay(&mut self, i: usize) -> bool {
        let Some(du) = self.enemies[i].duel.as_mut() else { return false };
        if du.alive() {
            du.f.kill();
        }
        let (p, id, illusion, risen) = (du.pos, du.id, du.lore.illusion, du.lore.risen);
        let prince = du.arch == crate::logic::combat::Archetype::Prince;
        let vault = self.brawl.as_ref().is_some_and(|b| b.field == Field::Vault);
        if self.brawl.as_ref().is_some_and(|b| b.held == Some(id)) {
            self.release_grasp();
        }
        if let Some(b) = self.brawl.as_mut() {
            if !illusion {
                b.stats.kills += 1;
            }
            // In the vaults its token goes back as it falls: the slain are swept from the field
            // before their next frame, and a token never given back would close the pool for good.
            // (The ring's slain stay on its floor and give theirs back on their next frame, as the
            // lab was tuned.)
            if b.field == Field::Vault {
                b.threat.release(id);
            }
        }
        let color = self.enemies[i].b.color;
        let mut last = false;
        if !illusion {
            let (kind, den) = self.enemies[i].duel.as_ref().map(|d| (d.breed.kind, d.den)).unwrap_or((EnemyKind::Skeleton, None));
            let same = |o: &Duelist| match (den, o.den) {
                (Some(a), Some(b)) => a.lo == b.lo && a.hi == b.hi,
                (None, None) => true,
                _ => false,
            };
            let last_of_pack = !self.enemies.iter().filter_map(|e| e.duel.as_deref()).any(|o| o.id != id && o.counts() && same(o));
            let dir = math::norm(math::sub(p, self.hero.pos()));
            self.war_note(WarEvent::Slain { foe: id, kind, at: p, dir, last_of_pack });
            self.feel_slain(i, dir, last_of_pack);
            last = last_of_pack;
        }
        self.enemies[i].b.mode = EnemyMode::Eaten;
        self.view.dust(&self.level.grid, p.x, p.y, color, 18, 2.6);
        self.view.sparkle(&self.level.grid, p.x, p.y, 0.6, 0xfff1c0, 12);
        if prince && self.in_court() {
            // A prince in his court: his key, and everything a boss's fall brings (`court`).
            self.prince_slain(p);
        } else if vault && !risen {
            // (The risen dead pay nothing the second time: no farming the Burning Field.)
            self.kill_spoils(p, crate::logic::systems::POINTS_KILL);
            if self.gilded_leader(id) {
                // A gilded leader's spoils come twice over.
                self.kill_spoils(p, crate::logic::systems::POINTS_KILL);
            }
            if self.rng.next() < HEAL_DROP * self.drum_bounds().potions {
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
        }
        let followers: Vec<usize> = (0..self.enemies.len())
            .filter(|&k| self.enemies[k].duel.as_ref().is_some_and(|d| d.alive() && d.lore.master == Some(id)))
            .collect();
        if !followers.is_empty() {
            let raised = self.enemies[followers[0]].duel.as_ref().is_some_and(|d| !d.lore.illusion);
            let text = if prince {
                "THE GOLD FALLS WITH HIM"
            } else if raised {
                "THE DEAD FALL WITH HER"
            } else {
                "THE ILLUSIONS FADE"
            };
            self.floater(p.x, p.y - 1.4, text, 1.4, Some(0xd8c8ff));
        }
        for k in followers {
            let fp = self.enemies[k].duel.as_ref().map(|d| d.pos).unwrap_or(p);
            if let Some(d) = self.enemies[k].duel.as_mut() {
                d.f.kill();
            }
            self.enemies[k].b.mode = EnemyMode::Eaten;
            self.flash(Flash::new(FlashKind::Smoke, fp));
            self.view.dust(&self.level.grid, fp.x, fp.y, 0xc8c0b0, 12, 1.6);
        }
        last
    }

    /// He is carried `step` tiles by a legend (a shove); in the ring, no farther than its edge
    /// (only his own feet take him out of a bout).
    fn carry_hero(&mut self, step: V) {
        let mut step = step;
        if self.brawl.as_ref().is_some_and(|b| b.field == Field::Ring) {
            let hp = self.hero.pos();
            let (lo, hi) = gym::ring_bounds();
            let to = math::add(hp, step);
            let to = V::new(to.x.clamp(lo.x + 0.6, hi.x - 0.6), to.y.clamp(lo.y + 0.6, hi.y - 0.6));
            step = math::sub(to, hp);
        }
        self.hero.shove(&self.level.grid, step);
    }

    /// A legend's mark on the scene.
    fn flash(&mut self, f: Flash) {
        if let Some(b) = self.brawl.as_mut() {
            b.flashes.push(f);
        }
    }

    /// A new duelist of `kind` comes into the fight at `at` (an illusion, a raised dead), facing
    /// `facing`, its legend begun as `lore`. Returns its id.
    pub(super) fn spawn_duelist(&mut self, kind: EnemyKind, at: V, facing: V, lore: Lore) -> Option<u32> {
        let id = self.brawl.as_mut()?.next_id();
        let spec = spec_of(&self.level.grid, kind);
        let mut e = Enemy::new(&spec, P::new(at.x.round() as i32, at.y.round() as i32), self.level.data.enemy_speed);
        e.b.mode = EnemyMode::Chase;
        e.b.name = spec.name.to_uppercase();
        let mut du = Duelist::bred(id, Breed::of(kind), at, facing);
        du.lore = lore;
        du.rest = 60;
        e.duel = Some(Box::new(du));
        self.view.add_enemy(&e);
        self.enemies.push(e);
        Some(id)
    }

    /// Where on the fight's floor `p` may stand: inside the ring (or on open floor in the vaults),
    /// else where it stood.
    fn on_floor(&self, p: V, or: V) -> V {
        let ok = match self.brawl.as_ref().map(|b| b.field) {
            Some(Field::Ring) => {
                let (lo, hi) = gym::ring_bounds();
                Arena { lo, hi }.open(p)
            }
            _ => self.level.grid.is_walkable(p.x.round() as i32, p.y.round() as i32, false),
        };
        if ok {
            p
        } else {
            or
        }
    }

    /// The wounded kitsune conjures two illusions of itself, one either side, and takes the place
    /// of one of them: which is which, only its blows (and a blow) will tell.
    fn conjure(&mut self, i: usize) {
        let Some(du) = self.enemies[i].duel.as_ref() else { return };
        let (id, pos, facing, hp, max) = (du.id, du.pos, du.facing, du.hp, du.max_hp);
        let across = V::new(-facing.y, facing.x);
        let spots = [-1.0, 1.0].map(|s| self.on_floor(math::add(pos, math::scale(across, s * CONJURE_SPREAD)), pos));
        let swap = (self.rng.next() * 3.0) as usize;
        for (k, at) in spots.into_iter().enumerate() {
            let lore = Lore { illusion: true, master: Some(id), ..Lore::default() };
            if let Some(sid) = self.spawn_duelist(EnemyKind::Kitsune, at, facing, lore) {
                if let Some(e) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == sid) {
                    (e.hp, e.max_hp) = (hp, max);
                    if k == swap {
                        // The real one stands where this illusion would have.
                        e.pos = pos;
                    }
                }
                if k == swap {
                    if let Some(d) = self.enemies[i].duel.as_mut() {
                        d.pos = at;
                    }
                }
            }
            self.flash(Flash::new(FlashKind::Smoke, at));
        }
        self.flash(Flash::new(FlashKind::Smoke, pos));
        self.floater(pos.x, pos.y - 1.3, "ILLUSIONS!", 1.2, Some(0xd8c8ff));
        self.audio.play(Sfx::Cackle);
    }

    /// Hel raises the dead: draugr climb out of the floor about him, up to her number.
    fn raise_dead(&mut self, i: usize) {
        let Some(du) = self.enemies[i].duel.as_ref() else { return };
        let (id, pos, room) = (du.id, du.pos, MAX_RAISED.saturating_sub(du.lore.raised));
        let hp = self.hero.pos();
        let toward = math::norm(math::sub(pos, hp));
        let base = math::angle(toward);
        for k in 0..room {
            let a = base + if k == 0 { 0.9 } else { -0.9 };
            let at = self
                .on_floor(math::add(hp, math::scale(math::from_angle(a), RAISE_SPREAD)), math::add(hp, math::scale(toward, RAISE_SPREAD)));
            let facing = math::norm(math::sub(hp, at));
            self.spawn_duelist(EnemyKind::Skeleton, at, facing, Lore { master: Some(id), rise: RISE_FRAMES, ..Lore::default() });
            self.flash(Flash::new(FlashKind::Grave, at));
            self.view.dust(&self.level.grid, at.x, at.y, 0x6a5a48, 16, 1.4);
        }
        if room > 0 {
            self.floater(hp.x, hp.y - 1.5, "THE DEAD RISE", 1.4, Some(0x9ab0ff));
        }
    }

    /// A plain blow of the training sword's first swing on duelist `k` (tests).
    #[cfg(test)]
    pub(super) fn call_duel_hit(&mut self, k: usize, from: V) {
        let d = MoveId::L1.data();
        self.duel_hit(k, Hit { damage: d.damage, poise: d.poise, hitstun: d.hitstun, ..Default::default() }, 0, from);
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
            let first =
                self.enemies.iter().filter_map(|e| e.duel.as_deref()).find(|d| d.den.is_some_and(|o| o.lo == den.lo && o.hi == den.hi));
            match first.map(|d| (d.breed.kind, d.pos)) {
                Some((kind, at)) => self.audio.foe_cry(kind, Cry::Alert, at),
                None => self.audio.play(Sfx::Growl),
            }
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
            for &id in &called_off {
                b.threat.release(id);
            }
        }
        self.grudges_left(&called_off);
    }

    /**
     * The foes' frame: their fighters (bolts loosed, spells cast, fire left on the floor), their
     * legends' cues, the tokens, the sectors, their minds, their blows (by shape, with what they
     * carry), the bolts, and the dragon's plea running out.
     */
    fn foes_frame(&mut self) {
        let field = self.brawl.as_ref().map(|b| b.field).unwrap_or(Field::Ring);
        if field == Field::Vault {
            self.update_dens();
            self.packs_frame();
        }
        let hp = self.hero.pos();
        let sense = Sense { hero: hp, aim: self.hero.aim() };
        let ids = self.duelists();
        let frame = self.brawl.as_ref().map(|b| b.threat.frame).unwrap_or(0);
        self.count_the_raised(&ids);

        // Their fighters: a hex looses its bolts; a move's first active frame does its work (a
        // spell cast, the ground shaken, fire left); a move run its course gives its token back.
        let mut bolts = Vec::new();
        let mut released = Vec::new();
        let mut actives = Vec::new();
        let mut whiffs = Vec::new();
        let foe_damage = self.brawl.as_ref().map(|b| b.foe_damage).unwrap_or(1.0);
        for &i in &ids {
            let du = self.enemies[i].duel.as_mut().unwrap();
            for e in du.step() {
                match e {
                    Event::Active(mv) => {
                        let d = du.data(mv);
                        if d.projectile {
                            let b = self.brawl.as_mut().unwrap();
                            let (count, spread) = match d.shape {
                                Shape::Volley { count, spread } => (count, spread),
                                _ => (1, 0.0),
                            };
                            let volley = if count > 1 {
                                b.volleys += 1;
                                b.volleys
                            } else {
                                0
                            };
                            let base = math::angle(du.aim);
                            for k in 0..count {
                                let a = if count > 1 { base - spread + 2.0 * spread * k as f64 / (count - 1) as f64 } else { base };
                                let dir = math::from_angle(a);
                                let look = BoltLook::of(du.breed.kind);
                                bolts.push(Bolt {
                                    pos: math::add(du.pos, math::scale(dir, 0.5)),
                                    vel: math::scale(dir, BOLT_SPEED),
                                    life: BOLT_LIFE,
                                    damage: d.damage * du.breed.damage * foe_damage,
                                    poise: 0.0,
                                    reflected: false,
                                    touched: false,
                                    due: b.threat.landing(du.id),
                                    look,
                                    tint: look.tint(),
                                    rider: du.rider(mv),
                                    owner: du.id,
                                    volley,
                                    phantom: du.lore.illusion,
                                });
                            }
                        } else {
                            actives.push((i, mv));
                        }
                    }
                    Event::Ended(mv) => {
                        du.token = false;
                        released.push(du.id);
                        let d = du.data(mv);
                        // (Coins on the floor burst after the throw has ended: they are not its swing.)
                        let swung = d.strikes() && !d.projectile && !matches!(d.shape, Shape::Traps { .. });
                        if swung && !self.brawl.as_ref().is_some_and(|b| b.met.contains(&du.id)) {
                            whiffs.push(WarEvent::Whiffed { foe: du.id, kind: du.breed.kind, mv });
                        }
                        if mv == MoveId::Call {
                            // It has called him in: it asks again at once, for the blow that follows.
                            du.rest = 0;
                        }
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
        for e in whiffs {
            self.war_note(e);
        }
        for (i, mv) in actives {
            self.first_active(i, mv);
        }
        self.read_cues(&ids);
        self.update_traps();
        self.update_grasp();
        if !self.hero.alive {
            return;
        }

        // Tokens: the engaged ask, the manager grants (one a frame at most); holders on their way update
        // their plans. (Not one felled, rising, or gone in smoke.)
        let b = self.brawl.as_mut().unwrap();
        let mut requests = Vec::new();
        for &i in &ids {
            let du = self.enemies[i].duel.as_ref().unwrap();
            if !du.engaged() || du.puppet || du.begging() || du.lore.rise > 0 || du.lore.vanish > 0 {
                continue;
            }
            if du.token {
                b.threat.project(du.id, du.eta(&sense));
                continue;
            }
            let wish = self.war.directives.iter().find(|d| d.0 == du.id).map(|d| d.1).unwrap_or_default();
            if du.f.is_neutral() && du.rest == 0 && wish.ask {
                let to = math::sub(du.pos, hp);
                requests.push(Request {
                    id: du.id,
                    cost: du.stats().cost,
                    eta: du.eta(&sense),
                    dist: math::len(to),
                    front: math::dot(sense.aim, math::norm(to)),
                    wait: du.wait,
                    bias: wish.bias,
                });
            }
        }
        if let Some(id) = b.threat.grant(&requests) {
            if let Some(i) = ids.iter().copied().find(|&i| self.enemies[i].duel.as_ref().unwrap().id == id) {
                self.enemies[i].duel.as_mut().unwrap().token = true;
            }
        }

        // The ground: the ring's square, or the vaults' tiles and the hunt's flow field from his tile.
        let b = self.brawl.as_mut().unwrap();
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
                .filter(|d| d.engaged() && !d.token && !d.puppet)
                .map(|d| {
                    let wish = self.war.directives.iter().find(|w| w.0 == d.id).map(|w| w.1).unwrap_or_default();
                    Waiter { id: d.id, pos: d.pos, ring: d.stats().ring + wish.back, prev: d.slot.map(|s| s.sector), want: wish.want }
                })
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
        let bodies: Vec<(u32, V, f64)> = ids
            .iter()
            .map(|&i| self.enemies[i].duel.as_ref().unwrap())
            .filter(|d| d.alive() && !d.untouchable())
            .map(|d| (d.id, d.pos, d.radius()))
            .collect();
        let mut began = Vec::new();
        for &i in &ids {
            let du = self.enemies[i].duel.as_mut().unwrap();
            if du.den.is_some_and(|d| !d.awake) {
                continue; // asleep
            }
            if du.puppet {
                // Moved and set going by whoever holds its strings; its moves play out below.
                let p = du.pos;
                self.enemies[i].m.tile = P::new(p.x.round() as i32, p.y.round() as i32);
                continue;
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
            let id = self.enemies[i].duel.as_ref().map(|d| d.id).unwrap_or(0);
            let evading = self.brawl.as_ref().is_some_and(|b| matches!(b.hero.state, State::Evade { .. }));
            if self.war.grudges.waits(&self.warband.save.ledger, &mut self.rng, evading, id, mv) {
                continue; // a rival who learnt the hero's answer takes his time
            }
            let du = self.enemies[i].duel.as_mut().unwrap();
            let b = self.brawl.as_mut().unwrap();
            let lands = du.lands_in(mv, hp);
            if b.threat.may_commit(du.id, lands) && du.begin(mv, hp) {
                b.threat.commit(du.id, lands);
                let listener = self.audio.listener();
                self.audio.tell(du.breed.kind, mv, du.pos, listener);
                began.push((i, mv));
            }
        }
        if let Some(b) = self.brawl.as_mut() {
            b.flow = flow;
        }
        for (i, mv) in began {
            if let Some(du) = self.enemies[i].duel.as_ref() {
                let (foe, kind, at) = (du.id, du.breed.kind, du.pos);
                if let Some(b) = self.brawl.as_mut() {
                    b.met.retain(|&m| m != foe);
                }
                self.rival_began(foe);
                self.war_note(WarEvent::Tell { foe, kind, mv, at });
            }
            self.on_begin(i, mv);
        }

        // Their blows on the hero, by their shapes; a flurry's every bite strikes anew.
        for &i in &ids {
            let du = self.enemies[i].duel.as_mut().unwrap();
            let Some((mv, Phase::Active, frame)) = du.f.attack() else { continue };
            let d = du.data(mv);
            if !d.strikes() || d.projectile {
                continue;
            }
            let bite = d.hit_index(frame - d.startup);
            if bite != du.lore.bite {
                du.lore.bite = bite;
                du.struck = false;
            }
            if du.struck {
                continue;
            }
            let Some(share) = du.reaches(hp, HERO_RADIUS) else { continue };
            du.struck = true;
            let rule = if !d.parriable() {
                GuardRule::Bare
            } else if !d.blockable() {
                GuardRule::ParryOnly
            } else {
                GuardRule::Full
            };
            let blow = Blow {
                from: du.pos,
                damage: d.damage * du.breed.damage * foe_damage * share,
                hitstop: d.hitstop,
                bolt: false,
                rule,
                rider: du.rider(mv),
                by: du.id,
                // (A flurry lands as one strike: its first bite.)
                lands: bite == 0,
            };
            let power = du.breed.power;
            let grasp = blow.rider == Rider::Grasp;
            let met = self.blow_on_hero(blow);
            if grasp && met == Defense::Open && self.hero.alive {
                self.seize(i);
            }
            if met == Defense::Parried {
                let guard = self.brawl.as_ref().map(|b| b.kit.guard).unwrap_or(crate::logic::combat::kit::Guard::TRAINING);
                let du = self.enemies[i].duel.as_mut().unwrap();
                du.f.deflect(guard.deflect, guard.riposte);
                du.flash = 8;
                let p = du.pos;
                if power == Power::Dish && du.lore.spilled == 0 {
                    // Bowed to, the kappa bows back, and the water in its dish spills out: it is weak until it fills again.
                    du.lore.spilled = SPILL_FRAMES;
                    self.flash(Flash::new(FlashKind::Splash, p));
                    self.floater(p.x, p.y - 1.2, "IT BOWS: ITS DISH SPILLS", 1.4, Some(0x8ad8ff));
                    self.audio.play(Sfx::Drip);
                    self.view.sparkle(&self.level.grid, p.x, p.y, 1.0, 0x8ad8ff, 22);
                }
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
        self.update_pleas(&ids);

        // The ring's bout is won when its last foe falls (its illusions do not count; a dragon
        // begging does).
        if field == Field::Ring {
            let standing = self.enemies.iter().filter_map(|e| e.duel.as_ref()).filter(|d| d.counts()).count();
            if standing == 0 && self.brawl.as_ref().is_some_and(|b| b.live) {
                self.finish_bout(true);
            }
        }
    }

    /// Each of Hel's: how many of the dead she raised still stand.
    fn count_the_raised(&mut self, ids: &[usize]) {
        let raised: Vec<(u32, u8)> = ids
            .iter()
            .filter_map(|&i| self.enemies[i].duel.as_ref())
            .filter(|d| d.breed.power == Power::Necromancy)
            .map(|d| {
                let n =
                    ids.iter().filter_map(|&k| self.enemies[k].duel.as_ref()).filter(|o| o.alive() && o.lore.master == Some(d.id)).count();
                (d.id, n as u8)
            })
            .collect();
        for (id, n) in raised {
            if let Some(d) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == id) {
                d.lore.raised = n;
            }
        }
    }

    /// A move has just begun: the owl's cry as it rises on its wings dooms him.
    fn on_begin(&mut self, i: usize, mv: MoveId) {
        let Some(du) = self.enemies[i].duel.as_ref() else { return };
        if mv == MoveId::Talons && du.breed.power == Power::Omen {
            let hp = self.hero.pos();
            let doomed = match self.brawl.as_mut().filter(|b| b.ails.doom == 0) {
                Some(b) => {
                    b.ails.doom = DOOM_FRAMES;
                    true
                }
                None => false,
            };
            if doomed {
                self.floater(hp.x, hp.y - 1.3, "DOOMED: THE OWL HAS CRIED", 1.4, Some(0xb08aff));
            }
        }
    }

    /// A move's first active frame, beyond its blow: the roar heard, the dead raised, the ground
    /// shaken, an eruption, ice bursting from the floor; Surtr's blade sets the floor alight.
    fn first_active(&mut self, i: usize, mv: MoveId) {
        if mv == MoveId::Rally {
            self.rally(i);
            return;
        }
        let Some(du) = self.enemies[i].duel.as_ref() else { return };
        let (kind, at) = (du.breed.kind, du.pos);
        self.audio.swing(kind, mv, at);
        let Some(du) = self.enemies[i].duel.as_ref() else { return };
        let (pos, aim, power, mark) = (du.pos, du.aim, du.breed.power, du.lore.mark);
        let d = du.data(mv);
        let hp = self.hero.pos();
        match mv {
            MoveId::Call => {
                // His name roared in a loved one's voice: heard within its reach, it dooms him.
                let name = self.hero_kind.name().to_uppercase();
                if math::len(math::sub(hp, pos)) <= d.reach {
                    if let Some(b) = self.brawl.as_mut() {
                        b.ails.doom = DOOM_FRAMES;
                    }
                    self.floater(hp.x, hp.y - 1.4, format!("\"{name}!\" DOOMED"), 1.6, Some(0xb08aff));
                } else {
                    self.floater(hp.x, hp.y - 1.4, format!("\"{name}...\""), 1.6, Some(0xa8d8ff));
                }
            }
            MoveId::Raise => self.raise_dead(i),
            MoveId::CoinToss | MoveId::CoinRain => self.throw_coins(i, mv),
            MoveId::Kanabo | MoveId::HoardSlam => {
                let mut f = Flash::new(FlashKind::Quake, math::add(pos, math::scale(aim, d.reach * 0.7)));
                if let Shape::Quake { radius, .. } = d.shape {
                    f.length = radius;
                }
                self.flash(f);
                self.audio.play(Sfx::BossSlam);
                self.feel_quake(math::add(pos, math::scale(aim, d.reach * 0.7)));
                self.view.dust(&self.level.grid, pos.x + aim.x * d.reach * 0.7, pos.y + aim.y * d.reach * 0.7, 0x8a7a64, 26, 3.0);
            }
            MoveId::Talons => {
                let at = math::add(pos, math::scale(aim, d.reach * 0.6));
                self.view.dust(&self.level.grid, at.x, at.y, 0x8a7a64, 12, 1.4);
            }
            MoveId::Burrow => {
                self.flash(Flash::new(FlashKind::Erupt, mark));
                self.audio.play(Sfx::BossSlam);
                self.feel_quake(mark);
                self.view.dust(&self.level.grid, mark.x, mark.y, 0x5a4a30, 30, 3.2);
            }
            MoveId::FrostSlam => {
                // Ice bursts from the floor where the fists came down, no farther than they reach.
                let mut f = Flash::new(FlashKind::Ice, pos);
                f.dir = aim;
                f.length = d.reach;
                self.flash(f);
                self.audio.play(Sfx::BossSlam);
                self.feel_quake(pos);
            }
            _ => {}
        }
        if power == Power::Fire && matches!(mv, MoveId::Slam | MoveId::Sweep) {
            // The sword of Muspelheim leaves the floor burning where it fell.
            let spots: Vec<V> = if mv == MoveId::Slam {
                vec![math::add(pos, math::scale(aim, d.reach * 0.75))]
            } else {
                (-1..=1)
                    .map(|k| {
                        math::add(pos, math::scale(math::from_angle(math::angle(aim) + k as f64 * d.half_angle * 0.55), d.reach * 0.7))
                    })
                    .collect()
            };
            if let Some(b) = self.brawl.as_mut() {
                b.hazards.extend(spots.into_iter().map(|p| Hazard { pos: p, radius: FIRE_RADIUS, life: FIRE_FRAMES }));
            }
            self.audio.play(Sfx::UrielWave);
        }
    }

    /// What the legends did this frame, shown: the cin's smoke as it goes and comes, the kappa's dish full again.
    fn read_cues(&mut self, ids: &[usize]) {
        for &i in ids {
            let Some(du) = self.enemies[i].duel.as_mut() else { continue };
            if du.cues.is_empty() {
                continue;
            }
            let (cues, p) = (std::mem::take(&mut du.cues), du.pos);
            for c in cues {
                match c {
                    Cue::Vanished { from, .. } => {
                        self.flash(Flash::new(FlashKind::Smoke, from));
                        self.audio.play(Sfx::Whoosh);
                    }
                    Cue::Appeared(at) => self.flash(Flash::new(FlashKind::Smoke, at)),
                    Cue::Refilled => self.view.sparkle(&self.level.grid, p.x, p.y, 1.1, 0x8ad8ff, 12),
                    Cue::Phase(phase) => self.prince_phase(i, phase),
                }
            }
        }
    }

    /// A felled dragon left alone: its plea runs out, and it dies ("my mother bore me but once").
    fn update_pleas(&mut self, ids: &[usize]) {
        for &i in ids {
            let Some(du) = self.enemies[i].duel.as_mut() else { continue };
            if !du.begging() {
                continue;
            }
            du.lore.plea -= 1;
            if du.lore.plea == 0 {
                let p = du.pos;
                self.floater(p.x, p.y - 1.4, "STRUCK ONCE, IT DIES", 1.6, Some(0xffd040));
                self.slay(i);
            }
        }
    }

    /**
     * The legends' work on him this frame: a shove carries him, frostbite, venom and Surtr's fire
     * burn; the fires and the scene's marks age.
     */
    fn update_ails(&mut self) {
        let hp = self.hero.pos();
        let Some(b) = self.brawl.as_mut() else { return };
        let burning = b.hazards.iter().any(|h| h.burns(hp, HERO_RADIUS));
        for h in b.hazards.iter_mut() {
            h.life = h.life.saturating_sub(1);
        }
        b.hazards.retain(|h| h.life > 0);
        for f in b.flashes.iter_mut() {
            f.age += 1;
        }
        b.flashes.retain(|f| f.age < f.life);
        let wounds = b.ails.step(burning);
        let shove = b.ails.shove;
        if math::len(shove) > 0.0 {
            self.carry_hero(math::scale(shove, 1.0 / 60.0));
        }
        for (amount, why) in wounds {
            let color = match why {
                Wound::Frost => 0x9ad8ff,
                Wound::Venom => 0x8ad04a,
                Wound::Fire => 0xff7a2a,
            };
            let taken = self.brawl.as_ref().map(|b| b.kit.taken).unwrap_or(1.0);
            self.wound_hero((amount * taken).round(), color);
            if tracing() {
                let frame = self.brawl.as_ref().map(|b| b.threat.frame).unwrap_or(0);
                eprintln!("f{frame} wound {why:?} {} hp {}", (amount * taken).round(), self.hero.hp);
            }
            if !self.hero.alive {
                return;
            }
        }
    }

    /**
     * A foe's blow (or bolt) reaches the hero: his fighter decides how he meets it (evade, parry,
     * block, a broken guard, or open), as far as the blow lets his guard count; his health pays
     * for it (half again if the owl's doom is on him); and what it carries rides in with it. In the
     * ring he is never slain (the bout is lost); in the vaults he is, and his shield wears as it blocks.
     */
    fn blow_on_hero(&mut self, blow: Blow) -> Defense {
        let hp = self.hero.pos();
        let from_front = math::dot(self.hero.aim(), math::norm(math::sub(blow.from, hp))) > 0.0;
        let airborne = self.hero.airborne();
        let stamina = self.hero.stamina;
        let Some(b) = self.brawl.as_mut() else { return Defense::Open };
        let swinging = b.hero.attack().is_some();
        let vault = b.field == Field::Vault;
        // Whatever reaches him counts as a landing, however he meets it.
        if blow.lands {
            let f = b.threat.frame;
            b.land(f);
        }
        let kit = b.kit;
        let damage = blow.damage;
        if tracing() {
            eprintln!(
                "   hero {:?} front {from_front} air {airborne} aim {:?} from {:?} at {:?}",
                b.hero.state,
                self.hero.aim(),
                blow.from,
                hp
            );
        }
        let cost = if blow.bolt && kit.guard.bolt_proof { 0.0 } else { damage * kit.guard.cost };
        let def = b.hero.defend(airborne, from_front, stamina, cost, blow.rule);
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
                self.view.sparkle(&self.level.grid, (hp.x + blow.from.x) / 2.0, (hp.y + blow.from.y) / 2.0, 0.9, 0xffe070, 30);
                self.floater(hp.x, hp.y - 0.9, "PARRY!", 0.9, Some(0xffd040));
            }
            Defense::Blocked => {
                b.stats.blocks += 1;
                wear = damage;
                self.hero.drain(cost);
                self.view.spark(&self.level.grid, hp.x, hp.y, 0xc8d0da);
                self.floater(hp.x, hp.y - 0.9, "BLOCKED", 0.6, Some(0xc8d0da));
            }
            Defense::GuardBroken => {
                b.stats.guard_breaks += 1;
                loss = (damage * kit.guard.break_share * kit.taken).round();
                wear = damage * (1.0 - kit.guard.break_share);
                self.hero.drain(self.hero.stamina);
                self.floater(hp.x, hp.y - 0.9, "GUARD BROKEN", 1.0, Some(0xff7a3a));
            }
            Defense::Open => {
                if swinging {
                    b.stats.caught += 1;
                }
                let r = b.hero.take_hit(&Hit { damage, hitstun: math::hero_hitstun(damage), ..Default::default() });
                loss = (r.damage * kit.taken).round();
            }
        }
        let doom = if loss > 0.0 { self.brawl.as_mut().map(|b| b.ails.spend_doom()).unwrap_or(1.0) } else { 1.0 };
        if doom > 1.0 {
            loss = (loss * doom).round();
            self.floater(hp.x, hp.y - 1.2, "THE OMEN COMES TRUE", 1.0, Some(0xb08aff));
        }
        if vault && wear > 0.0 {
            let wear = wear * self.rival_wear(blow.by);
            self.wear_shield(wear);
        }
        match (def, blow.rider) {
            (Defense::Open, rider) => self.ride(rider, &blow, loss),
            (Defense::Blocked | Defense::GuardBroken, Rider::Shove(tiles)) => self.ride(Rider::Shove(tiles * 0.5), &blow, 0.0),
            (Defense::Blocked | Defense::GuardBroken, Rider::Frost) => self.ride(Rider::Frost, &blow, 0.0),
            _ => {}
        }
        if loss > 0.0 {
            self.wound_hero(loss, 0xff4a3a);
        }
        let kind = self.duelist_index(blow.by).and_then(|k| self.enemies[k].duel.as_ref()).map(|d| d.breed.kind);
        if let Some(b) = self.brawl.as_mut() {
            if !b.met.contains(&blow.by) {
                b.met.push(blow.by);
            }
        }
        let dir = math::norm(math::sub(hp, blow.from));
        let met = match def {
            Defense::Immune => None,
            Defense::Evaded => Some(Met::Evaded),
            Defense::Parried => Some(Met::Parried),
            Defense::Blocked => Some(Met::Blocked),
            Defense::GuardBroken => Some(Met::GuardBroken),
            Defense::Open => Some(Met::Struck),
        };
        if let Some(met) = met {
            self.feel_struck(met, blow.by, dir, loss, blow.hitstop);
        }
        self.war_note(WarEvent::Landed { foe: blow.by, kind, how: def, dmg: loss, dir, lands: blow.lands, bolt: blow.bolt });
        if tracing() {
            let who =
                self.duelist_index(blow.by).and_then(|k| self.enemies[k].duel.as_ref()).map(|d| (d.breed.kind, d.f.attack().map(|a| a.0)));
            let frame = self.brawl.as_ref().map(|b| b.threat.frame).unwrap_or(0);
            eprintln!("f{frame} blow {who:?} bolt {} {:?} -> {def:?} loss {loss} hp {}", blow.bolt, blow.rider, self.hero.hp);
        }
        def
    }

    /// What a blow carries, on him: a shove, frostbite, venom; a drain heals the one who struck.
    /// (None of it slows him or holds him.)
    fn ride(&mut self, rider: Rider, blow: &Blow, loss: f64) {
        let hp = self.hero.pos();
        if rider == Rider::Drain {
            if let Some(d) = self.duelist_index(blow.by).and_then(|k| self.enemies[k].duel.as_mut()).filter(|_| loss > 0.0) {
                d.hp = (d.hp + loss).min(d.max_hp);
                let p = d.pos;
                self.floater(p.x, p.y - 1.0, format!("+{loss}"), 0.9, Some(0x7aa0ff));
            }
            return;
        }
        let Some(b) = self.brawl.as_mut() else { return };
        let (text, color) = match rider {
            Rider::None | Rider::Raise(_) | Rider::Drain | Rider::Grasp => return,
            Rider::Shove(tiles) => {
                b.ails.shove = math::scale(math::norm(math::sub(hp, blow.from)), SHOVE_SPEED * tiles);
                return;
            }
            Rider::Frost => {
                b.ails.frost = FROST_FRAMES;
                ("FROSTBITE", 0x9ad8ff)
            }
            Rider::Venom(frames) => {
                b.ails.venom = b.ails.venom.max(frames);
                ("VENOM", 0x8ad04a)
            }
        };
        self.floater(hp.x, hp.y - 1.25, text, 1.1, Some(color));
    }

    /// He loses `loss` health (shown in `color`): his second wind, if he has one, puts the end off
    /// once; at none the vaults slay him and the ring's bout is lost. (The realms after the vaults
    /// wound him through it too: their fires, acids, thorns and falls.)
    pub(super) fn wound_hero(&mut self, loss: f64, color: u32) {
        if loss <= 0.0 || !self.hero.alive {
            return;
        }
        let hp = self.hero.pos();
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
        self.floater(hp.x, hp.y - 0.5, format!("-{loss}"), 0.9, Some(color));
        self.keep_immortal();
        let down = self.hero.hp == 0;
        let (live, vault) = self.brawl.as_ref().map(|b| (b.live, b.field == Field::Vault)).unwrap_or((false, false));
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

    /// A foe's bolt lands at `at` (on him, on his shield, or against a wall): the root dart bursts
    /// into roots, the thunderbolt comes down as lightning, the spit splashes, the fire scatters.
    fn bolt_lands(&mut self, look: BoltLook, at: V) {
        match look {
            BoltLook::Thorn => {
                self.flash(Flash::new(FlashKind::Roots, at));
                self.view.dust(&self.level.grid, at.x, at.y, 0x5a4a30, 14, 1.6);
            }
            BoltLook::Lightning => {
                self.flash(Flash::new(FlashKind::Lightning, at));
                self.audio.play(Sfx::GabrielThunder);
                self.view.shake(0.2);
            }
            BoltLook::Spit => self.view.dust(&self.level.grid, at.x, at.y, 0x7aff5a, 10, 1.2),
            BoltLook::Fire => self.view.sparkle(&self.level.grid, at.x, at.y, 0.6, 0xffa040, 14),
            _ => {}
        }
    }

    /// Bolts fly; one meets the hero (or, thrown by him or parried back, the foes); they burn out
    /// at the ring's edge or against a wall. An illusion's bolt is smoke at a touch; a volley's
    /// bolts land as one strike; where a foe's bolt lands, it lands as its kind does (`bolt_lands`).
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
            if b.due.is_some_and(|due| now > due + SLACK) && !b.reflected && !b.touched {
                // It has missed its moment (he stepped off its line): it burns out, and leaves the
                // time it was given to the others' strikes.
                b.life = 0;
                self.view.dust(&self.level.grid, b.pos.x, b.pos.y, b.tint, 6, 0.8);
                continue;
            }
            let out = if ring {
                b.pos.x < lo.x || b.pos.x > hi.x || b.pos.y < lo.y || b.pos.y > hi.y
            } else {
                !self.level.grid.is_walkable(b.pos.x.round() as i32, b.pos.y.round() as i32, false)
            };
            if out {
                b.life = 0;
                if !b.reflected && !b.phantom {
                    self.bolt_lands(b.look, b.pos);
                }
            }
            if b.life == 0 {
                continue;
            }
            if !b.reflected {
                if !b.touched && math::len(math::sub(b.pos, hp)) <= BOLT_RADIUS + HERO_RADIUS {
                    b.touched = true;
                    if b.phantom {
                        // An illusion's fire: nothing behind it.
                        b.life = 0;
                        self.view.dust(&self.level.grid, b.pos.x, b.pos.y, 0xb8c8ff, 6, 1.0);
                        continue;
                    }
                    let from = math::sub(b.pos, b.vel);
                    let lands = {
                        let brawl = self.brawl.as_mut().unwrap();
                        let first = b.volley == 0 || b.volley != brawl.landed_volley;
                        if b.volley != 0 {
                            brawl.landed_volley = b.volley;
                        }
                        first
                    };
                    let blow =
                        Blow { from, damage: b.damage, hitstop: 3, bolt: true, rule: GuardRule::Full, rider: b.rider, by: b.owner, lands };
                    if self.blow_on_hero(blow) == Defense::Parried {
                        // Turned back at the foes, faster, and harder as his shield is better.
                        let reflect = self.brawl.as_ref().map(|b| b.kit.guard.reflect).unwrap_or(1.0);
                        b.vel = math::scale(b.vel, -REFLECT_SPEED);
                        b.reflected = true;
                        b.damage *= reflect;
                        b.poise = REFLECT_POISE;
                        b.tint = REFLECT_TINT;
                        b.rider = Rider::None;
                    } else if !self.hero.airborne() {
                        b.life = 0;
                        self.bolt_lands(b.look, b.pos);
                    }
                    if !self.hero.alive {
                        return;
                    }
                }
                continue;
            }
            let hit = ids.iter().copied().find(|&i| {
                let du = self.enemies[i].duel.as_ref().unwrap();
                du.alive() && !du.untouchable() && math::len(math::sub(b.pos, du.pos)) <= BOLT_RADIUS + du.radius()
            });
            if let Some(i) = hit {
                b.life = 0;
                let d = MoveId::Hex.data();
                let share = self.enemies[i].duel.as_ref().map(|du| self.ward_share(du.id)).unwrap_or(1.0);
                let hit = Hit { damage: b.damage * share, poise: b.poise * share, hitstun: 20, ..Default::default() };
                self.duel_hit(i, hit, d.hitstop, b.pos);
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

impl Game {
    /**
     * A prince's grasp has closed on him: his fist holds him (his feet, not his hands) and the
     * prince keeps still in his recovery, his hide open, until a blow on him breaks it or the fist
     * lets go of itself. Nothing else falls on him meanwhile (the threat manager is kept quiet).
     */
    fn seize(&mut self, i: usize) {
        let Some(du) = self.enemies[i].duel.as_mut() else { return };
        let d = du.data(MoveId::Grasp);
        du.lore.hold = GRASP_HOLD;
        du.f.state = State::Attack { mv: MoveId::Grasp, frame: d.startup + d.active };
        let (id, pos, aim, radius) = (du.id, du.pos, du.aim, du.radius());
        let hp = self.hero.pos();
        let to = math::add(pos, math::scale(aim, radius + FIST));
        self.hero.shove(&self.level.grid, math::sub(to, hp));
        self.hero.heading = math::angle(math::sub(pos, self.hero.pos()));
        if let Some(b) = self.brawl.as_mut() {
            b.held = Some(id);
            b.threat.quiet_until = b.threat.frame + GRASP_HOLD as u32;
        }
        let hp = self.hero.pos();
        self.floater(hp.x, hp.y - 1.5, "IN HIS FIST: STRIKE HIM!", 1.6, Some(0xffd040));
        self.audio.play(Sfx::Brutal);
        self.view.shake(0.3);
    }

    /// The fist lets go of him (struck, felled, or of itself): his feet are his own again.
    pub(super) fn release_grasp(&mut self) {
        let Some(b) = self.brawl.as_mut() else { return };
        let Some(id) = b.held.take() else { return };
        b.threat.quiet_until = b.threat.frame;
        if let Some(d) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == id) {
            d.lore.hold = 0;
        }
    }

    /// The fist holding him: every `DRAIN_EVERY` frames it takes his silver and heals the prince,
    /// until it lets go of itself.
    fn update_grasp(&mut self) {
        let Some(id) = self.brawl.as_ref().and_then(|b| b.held) else { return };
        let Some(i) = self.duelist_index(id) else {
            self.release_grasp();
            return;
        };
        let Some(du) = self.enemies[i].duel.as_mut().filter(|d| d.alive() && d.lore.hold > 0) else {
            self.release_grasp();
            return;
        };
        du.lore.hold -= 1;
        let p = du.pos;
        if du.lore.hold == 0 {
            self.release_grasp();
            self.floater(p.x, p.y - 2.0, "HE LETS GO", 1.0, Some(0xd8c8a0));
            return;
        }
        if !(GRASP_HOLD - du.lore.hold).is_multiple_of(DRAIN_EVERY) {
            return;
        }
        let heal = (du.max_hp * DRAIN_HEAL).round();
        du.hp = (du.hp + heal).min(du.max_hp);
        let silver = DRAIN_SILVER.min(self.progress.silver);
        self.progress.silver -= silver;
        let hp = self.hero.pos();
        let coin = crate::logic::progression::coin(self.hero_kind);
        if silver > 0 {
            self.floater(hp.x, hp.y - 0.9, format!("-{silver} {coin}"), 0.9, Some(0xd8d8e0));
        }
        self.floater(p.x, p.y - 2.2, format!("+{heal}"), 0.9, Some(0xf2c53d));
        self.audio.play(Sfx::Nodule);
        self.view.sparkle(&self.level.grid, (hp.x + p.x) / 2.0, (hp.y + p.y) / 2.0, 0.7, 0xf2c53d, 10);
    }

    /// A prince's coin toss (or rain) as its first active frame comes: coins fly from his hand (or
    /// fall from above) onto the floor about the hero, one where he stands, to burst together.
    fn throw_coins(&mut self, i: usize, mv: MoveId) {
        let Some(du) = self.enemies[i].duel.as_ref() else { return };
        let d = du.data(mv);
        let Shape::Traps { count, radius, fuse } = d.shape else { return };
        let foe_damage = self.brawl.as_ref().map(|b| b.foe_damage).unwrap_or(1.0);
        let (id, pos, damage) = (du.id, du.pos, d.damage * du.breed.damage * foe_damage);
        let rain = mv == MoveId::CoinRain;
        let hp = self.hero.pos();
        let turn = self.rng.next() * std::f64::consts::TAU;
        let spots: Vec<V> = prince::scatter(count, hp, if rain { 1.6 } else { 1.8 }, turn, rain)
            .into_iter()
            .enumerate()
            .filter_map(|(k, p)| (k == 0 || self.on_floor(p, hp) == p).then_some(p))
            .collect();
        let Some(b) = self.brawl.as_mut() else { return };
        b.volleys += 1;
        let volley = b.volleys;
        // (They burst as the threat manager planned: the flight from this frame, then the fuse.)
        b.traps.extend(spots.into_iter().map(|p| Trap {
            pos: p,
            from: pos,
            flight: COIN_FLIGHT + 1,
            fuse,
            full: fuse,
            radius,
            damage,
            owner: id,
            volley,
            burst: None,
            fell: rain,
        }));
        self.audio.play(if rain { Sfx::Toll } else { Sfx::Whoosh });
        if rain {
            self.floater(hp.x, hp.y - 1.6, "IT RAINS GOLD", 1.0, Some(0xf2c53d));
        }
    }

    /// The coins on the floor: their fuses burn, and those that burst throw up gold spikes; one
    /// throw's spikes reach him as one strike, unless he is off them or in the air.
    fn update_traps(&mut self) {
        let hp = self.hero.pos();
        let Some(b) = self.brawl.as_mut() else { return };
        let mut traps = std::mem::take(&mut b.traps);
        let mut bursts = Vec::new();
        let mut on_him: Vec<Trap> = Vec::new();
        for t in traps.iter_mut() {
            if t.step() {
                bursts.push(t.pos);
                if t.reaches(hp, HERO_RADIUS) && !on_him.iter().any(|o| o.volley == t.volley) {
                    on_him.push(*t);
                }
            }
        }
        traps.retain(|t| t.live());
        b.traps = traps;
        if !bursts.is_empty() {
            self.audio.play(Sfx::Clang);
            self.view.shake(0.12);
        }
        for p in bursts {
            self.view.sparkle(&self.level.grid, p.x, p.y, 0.7, 0xffd860, 14);
        }
        for t in on_him {
            let blow = Blow {
                from: t.pos,
                damage: t.damage,
                hitstop: 5,
                bolt: false,
                rule: GuardRule::Bare,
                rider: Rider::None,
                by: t.owner,
                lands: true,
            };
            self.blow_on_hero(blow);
            if !self.hero.alive {
                return;
            }
        }
    }
}
