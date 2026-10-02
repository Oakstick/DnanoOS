//! The Sparring Ring (COMBAT.md §5): the combat lab west of the gym's arrival platform.
//!
//! Inside the ring the hero fights by frame data: his attack, guard and evade go through his
//! `Fighter` (the input buffer, the cancel rules, the string of three), and the foes are
//! duelists of the three archetypes whose attacks the threat manager paces. Step in and a bout
//! begins; lose and it begins again; win and the next one comes. Nothing here touches his
//! shield, his lives or his gold.

use super::Game;
use crate::audio::Sfx;
use crate::logic::combat::duelist::{Arena, Bolt, Sense, HERO_RADIUS};
use crate::logic::combat::frames::{BOLT_LIFE, BOLT_RADIUS, BOLT_SPEED, REFLECT_POISE, REFLECT_SPEED};
use crate::logic::combat::fsm::{Defense, Effect, Hit, State};
use crate::logic::combat::kit::Kit;
use crate::logic::combat::threat::{assign_slots, Request, Waiter, REVOKE_REST};
use crate::logic::combat::{math, secs, Duelist, Event, Fighter, FrameClock, Input, MoveId, Phase, Reel, ThreatManager};
use crate::logic::enemy::{enemy_specs, Enemy, EnemyMode};
use crate::logic::gym;
use crate::logic::progression::STAMINA_PER_BLOW;
use crate::logic::types::{P, V};
use crate::render::view::CameraMode;

/// Seconds of the countdown before a bout, and of the pause after one is won or lost.
pub const COUNTDOWN: f64 = 2.0;
/// (Long enough to read the results card.)
pub const AFTERMATH: f64 = 4.5;
/// Frames between re-dealing the waiting foes' sectors.
const SLOT_EVERY: u32 = 12;
/// Freeze frames for a broken poise, a parry and a kill.
const HITSTOP_BREAK: u16 = 8;
const HITSTOP_PARRY: u16 = 8;
const HITSTOP_KILL: u16 = 6;
/// The combat log: one line per bout, in the save folder (the web build keeps the last lines in storage).
pub const LOG_FILE: &str = "combat_log.csv";
const LOG_HEADER: &str = "time,hero,tier,bout,result,secs,dealt,taken,hits,hurt,parries,blocks,evades,breaks,caught,guard_breaks,overlaps";
/// The slowest and fastest a time-locked bolt flies, as shares of its speed.
const BOLT_PACE: (f64, f64) = (0.5, 1.6);
/// How far off his facing (radians) the soft lock turns his swing onto a foe.
const ASSIST_ANGLE: f64 = 0.8;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BoutPhase {
    /// The hero is outside the ring.
    Idle,
    /// Seconds before the foes move.
    Countdown(f64),
    Fight,
    /// Seconds of the pause after a bout won, or lost.
    Won(f64),
    Lost(f64),
}

/// What a bout has seen (the telemetry of COMBAT.md §6, phase 2).
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
}

#[derive(Clone, Debug)]
pub struct Sparring {
    /// Which of `gym::BOUTS`.
    pub bout: usize,
    pub phase: BoutPhase,
    /// The hero's fighter.
    pub hero: Fighter,
    /// The aim of his swing, committed on its first active frame, and the duelists it has struck.
    pub aim: V,
    struck: Vec<u32>,
    pub threat: ThreatManager,
    pub bolts: Vec<Bolt>,
    clock: FrameClock,
    pub stats: BoutStats,
    /// The last bout won, with its numbers, shown until the next is won.
    pub last: Option<(usize, BoutStats)>,
    /// The frame readout (F5).
    pub readout: bool,
    next_id: u32,
    /// The frames the foes' strikes landed on, for counting overlaps.
    strikes: Vec<u32>,
    /// The camera before the ring turned it to the top view (the ring is fought from above, like the vaults).
    pub camera_before: Option<CameraMode>,
    /// His gear as numbers (COMBAT.md §8), fixed for the bout, and whether his second wind is spent.
    pub kit: Kit,
    wind_used: bool,
}

impl Default for Sparring {
    fn default() -> Self {
        Self::new()
    }
}

impl Sparring {
    pub fn new() -> Self {
        Self {
            bout: 0,
            phase: BoutPhase::Idle,
            hero: Fighter::hero(),
            aim: V::new(1.0, 0.0),
            struck: Vec::new(),
            threat: ThreatManager::new(),
            bolts: Vec::new(),
            clock: FrameClock::default(),
            stats: BoutStats::default(),
            last: None,
            readout: true,
            next_id: 1,
            strikes: Vec::new(),
            camera_before: None,
            kit: Kit::training(),
            wind_used: false,
        }
    }

    /// A foe's strike reached the hero on frame `f` (a blow meeting him, a bolt touching him, however
    /// he met it): one landing inside the reaction window of another counts as an overlap.
    fn land(&mut self, f: u32) {
        if self.strikes.iter().any(|&g| f.abs_diff(g) < crate::logic::combat::threat::SPACING) {
            self.stats.overlaps += 1;
        }
        self.strikes.push(f);
        self.strikes.retain(|&g| f.saturating_sub(g) < 120);
    }

    /// The bout's name, numbered.
    pub fn title(&self) -> String {
        format!("BOUT {}: {}", self.bout + 1, gym::BOUTS[self.bout].name)
    }
}

impl Game {
    /// The hero stands in the Sparring Ring: his fighting goes by frame data.
    pub fn in_ring(&self) -> bool {
        self.spar.is_some() && gym::ring().contains(self.hero.occupied())
    }

    /// The duelists' indices in `self.enemies`.
    fn duelists(&self) -> Vec<usize> {
        (0..self.enemies.len()).filter(|&i| self.enemies[i].duel.is_some()).collect()
    }

    /// Takes the bout's foes off the ring.
    fn clear_duelists(&mut self) {
        if self.enemies.iter().any(|e| e.duel.is_some()) {
            self.enemies.retain(|e| e.duel.is_none());
            self.view.clear_enemies();
            for e in &self.enemies {
                self.view.add_enemy(e);
            }
        }
        if let Some(s) = self.spar.as_mut() {
            s.threat.clear();
            s.bolts.clear();
            s.strikes.clear();
        }
    }

    /// The bout's foes take their places along the ring's west side, and the countdown starts.
    pub(super) fn start_bout(&mut self) {
        self.clear_duelists();
        let kit = self.hero_kit();
        let Some(spar) = self.spar.as_mut() else { return };
        let bout = gym::BOUTS[spar.bout];
        let specs = enemy_specs(&self.level.grid, 2);
        let mut fresh = Vec::new();
        for (arch, at) in bout.foes.iter().zip(gym::bout_spawns(bout.foes.len())) {
            let spec = specs.iter().find(|s| s.kind == arch.kind()).expect("an archetype's body");
            let mut e = Enemy::new(spec, P::new(at.x.round() as i32, at.y.round() as i32), self.level.data.enemy_speed);
            e.b.mode = EnemyMode::Chase;
            e.b.name = arch.name().to_string();
            e.duel = Some(Box::new(Duelist::new(spar.next_id, *arch, at, V::new(1.0, 0.0))));
            spar.next_id += 1;
            fresh.push(e);
        }
        spar.phase = BoutPhase::Countdown(COUNTDOWN);
        spar.stats = BoutStats::default();
        if spar.camera_before.is_none() {
            spar.camera_before = Some(self.view.camera_mode);
            self.view.camera_mode = CameraMode::Top;
        }
        spar.kit = kit;
        spar.hero = Fighter::with_kit(&kit);
        spar.wind_used = false;
        let title = spar.title();
        for e in fresh {
            self.view.add_enemy(&e);
            self.enemies.push(e);
        }
        self.hero.hp = self.hero.max_hp;
        let hp = self.hero.pos();
        self.floater(hp.x, hp.y - 1.4, title, COUNTDOWN, Some(0xffc860));
        self.audio.play(Sfx::Tick);
    }

    /// Out of the ring: the bout is called off.
    fn end_bout(&mut self) {
        self.clear_duelists();
        if let Some(s) = self.spar.as_mut() {
            s.phase = BoutPhase::Idle;
            s.hero = Fighter::with_kit(&s.kit);
            if let Some(mode) = s.camera_before.take() {
                self.view.camera_mode = mode;
            }
        }
        self.hero.root = 1.0;
        self.hero.blocking = false;
    }

    /// His blade, shield and path as the ring's numbers.
    pub fn hero_kit(&self) -> Kit {
        Kit::of(self.hero_kind, self.hero.blade, self.hero.edge, self.hero.shield.map(|s| s.tier), self.hero.path)
    }

    /// F6: on to the next bout.
    pub(super) fn next_bout(&mut self) {
        let inside = self.in_ring();
        let Some(s) = self.spar.as_mut() else { return };
        s.bout = (s.bout + 1) % gym::BOUTS.len();
        if inside {
            self.start_bout();
        }
    }

    /// The ring's real-time flow (bouts begin, end, start over), then its frames.
    pub(super) fn update_sparring(&mut self, dt: f64) {
        let inside = self.in_ring();
        let Some(phase) = self.spar.as_ref().map(|s| s.phase) else { return };
        match phase {
            BoutPhase::Idle => {
                if inside {
                    self.start_bout();
                }
            }
            BoutPhase::Countdown(_) | BoutPhase::Fight if !inside => {
                self.end_bout();
                return;
            }
            BoutPhase::Countdown(t) => {
                let left = t - dt;
                if let Some(s) = self.spar.as_mut() {
                    s.phase = if left <= 0.0 { BoutPhase::Fight } else { BoutPhase::Countdown(left) };
                }
                if left <= 0.0 {
                    self.audio.play(Sfx::Go);
                }
            }
            BoutPhase::Fight => {}
            BoutPhase::Won(t) | BoutPhase::Lost(t) => {
                let left = t - dt;
                if left > 0.0 {
                    if let Some(s) = self.spar.as_mut() {
                        s.phase = if matches!(phase, BoutPhase::Won(_)) { BoutPhase::Won(left) } else { BoutPhase::Lost(left) };
                    }
                } else {
                    if let Some(s) = self.spar.as_mut() {
                        if matches!(phase, BoutPhase::Won(_)) {
                            s.bout = (s.bout + 1) % gym::BOUTS.len();
                        }
                        s.phase = BoutPhase::Idle;
                    }
                    self.hero.hp = self.hero.max_hp;
                    self.clear_duelists();
                    if inside {
                        self.start_bout();
                    }
                }
            }
        }
        if !self.in_ring() {
            return;
        }
        let frames = self.spar.as_mut().map(|s| s.clock.advance(dt)).unwrap_or(0);
        for _ in 0..frames {
            self.spar_frame();
        }
    }

    /// One 60 Hz frame of the ring: the hero's fighter, then (in a fight) the foes, their bolts and the threat manager.
    fn spar_frame(&mut self) {
        let cost = self.spar.as_ref().map(|s| s.kit.swing_cost()).unwrap_or(STAMINA_PER_BLOW);
        let can_attack = self.hero.tireless() || self.hero.stamina + 1e-9 >= cost;
        let Some(spar) = self.spar.as_mut() else { return };
        spar.hero.can_attack = can_attack;
        let events = spar.hero.step();
        for e in events {
            match e {
                Event::Began(mv) => {
                    self.hero.exert(cost);
                    let d = self.spar.as_ref().map(|s| s.hero.data(mv)).unwrap_or(mv.data());
                    self.view.sword_slash_timed(secs(d.startup) as f32, secs(d.active) as f32, secs(d.recovery) as f32);
                    self.audio.play(Sfx::Slash);
                    let aim = self.assisted_aim(d.reach);
                    if let Some(s) = self.spar.as_mut() {
                        s.struck.clear();
                        s.aim = aim;
                    }
                }
                Event::Active(mv) => {
                    // The aim is committed on the first active frame; a thrown finisher flies down it.
                    let d = self.spar.as_ref().map(|s| s.hero.data(mv)).unwrap_or(mv.data());
                    let aim = self.assisted_aim(d.reach);
                    let hp = self.hero.pos();
                    if let Some(s) = self.spar.as_mut() {
                        s.aim = aim;
                        if let (true, Some(t)) = (d.projectile, s.kit.strike.thrown) {
                            s.bolts.push(Bolt {
                                pos: math::add(hp, math::scale(aim, 0.5)),
                                vel: math::scale(aim, t.speed),
                                life: (t.range / t.speed * 60.0) as u16,
                                damage: d.damage,
                                poise: d.poise,
                                reflected: true,
                                touched: false,
                                due: None,
                            });
                        }
                    }
                }
                Event::Evade if self.hero.try_jump(&self.level.grid) => {
                    self.audio.play(Sfx::Jump);
                    if let Some(s) = self.spar.as_mut() {
                        s.hero.evade();
                    }
                }
                _ => {}
            }
        }
        let Some(spar) = self.spar.as_ref() else { return };
        self.hero.root = spar.hero.root();
        self.hero.blocking = matches!(spar.hero.state, State::Guard { .. } | State::Reel { kind: Reel::Blockstun, .. });
        let fighting = spar.phase == BoutPhase::Fight;
        self.hero_strikes();
        if fighting {
            self.foes_frame();
        }
        if let Some(s) = self.spar.as_mut() {
            s.threat.tick();
            if s.phase == BoutPhase::Fight {
                s.stats.frames += 1;
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
        let Some(spar) = self.spar.as_ref() else { return };
        let Some((mv, Phase::Active, _)) = spar.hero.attack() else { return };
        let d = spar.hero.data(mv);
        if d.projectile {
            return; // thrown, not swung
        }
        let strike = spar.kit.strike;
        let hp = self.hero.pos();
        let aim = spar.aim;
        let targets: Vec<usize> = self
            .duelists()
            .into_iter()
            .filter(|&i| {
                let du = self.enemies[i].duel.as_ref().unwrap();
                du.alive() && !spar.struck.contains(&du.id) && math::in_cone(hp, aim, du.pos, du.radius(), d.reach, d.half_angle)
            })
            .collect();
        let landed = targets.len();
        for i in targets {
            let du = self.enemies[i].duel.as_ref().unwrap();
            let id = du.id;
            // From behind (or the flank), the fang or the flanking blow bites deeper.
            let off = math::dot(du.facing, math::norm(math::sub(hp, du.pos))).clamp(-1.0, 1.0).acos();
            let back = if off >= strike.back_from { strike.back } else { 1.0 };
            if let Some(s) = self.spar.as_mut() {
                s.struck.push(id);
            }
            let hit = Hit { damage: d.damage * back, poise: d.poise, hitstun: d.hitstun, pierce: strike.pierce, crit: strike.crit };
            self.duel_hit(i, hit, d.hitstop, hp);
        }
        if landed > 0 && strike.blood > 0.0 {
            // A cursed blade drinks from the hand that holds it.
            self.hero.hp = (self.hero.hp - (strike.blood * landed as f64) as i32).max(1);
        }
    }

    /// A blow lands on duelist `i` from `from`: damage through its fighter, the flash and shove, the numbers, and death.
    fn duel_hit(&mut self, i: usize, hit: Hit, hitstop: u16, from: V) {
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
        if let Some(s) = self.spar.as_mut() {
            s.stats.dealt += r.damage;
            s.stats.hits += 1;
            if r.effect == Effect::Broken {
                s.stats.breaks += 1;
            }
        }
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
        if dead && self.spar.as_ref().is_some_and(|s| s.kit.terror) {
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
            self.audio.play_kill(0);
            self.view.shake(0.18);
        }
        self.hitstop = self.hitstop.max(secs(stop));
    }

    /// Something outside the frame data reached duelist `i` (a spirit at his side, holy fire): in
    /// the ring it does nothing (the audit of COMBAT.md §8 cuts what bypasses the state machine).
    pub(super) fn stray_blow(&mut self, i: usize) {
        let _ = i;
    }

    /// The foes' frame: their fighters, the tokens, the sectors, their minds, their blows, the bolts, and the end of the bout.
    fn foes_frame(&mut self) {
        let hp = self.hero.pos();
        let swing = matches!(self.spar.as_ref().and_then(|s| s.hero.attack()), Some((_, Phase::Startup, _)));
        let sense = Sense { hero: hp, aim: self.hero.aim(), swing };
        let (lo, hi) = gym::ring_bounds();
        let arena = Arena { lo, hi };
        let ids = self.duelists();
        let frame = self.spar.as_ref().map(|s| s.threat.frame).unwrap_or(0);

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
                                damage: mv.data().damage,
                                poise: 0.0,
                                reflected: false,
                                touched: false,
                                due: self.spar.as_ref().and_then(|s| s.threat.landing(du.id)),
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
            let spar = self.spar.as_ref().unwrap();
            if du.token && (du.f.reeling().is_some() || du.f.is_dead() || spar.threat.expired(du.id)) {
                du.token = false;
                du.rest = REVOKE_REST;
                released.push(du.id);
            }
        }
        if !bolts.is_empty() {
            self.audio.play(Sfx::Zap);
        }
        let spar = self.spar.as_mut().unwrap();
        for id in released {
            spar.threat.release(id);
        }
        spar.bolts.extend(bolts);

        // Tokens: the waiting ask, the manager grants (one a frame at most); holders on their way update their plans.
        let mut requests = Vec::new();
        for &i in &ids {
            let du = self.enemies[i].duel.as_ref().unwrap();
            if du.token {
                spar.threat.project(du.id, du.eta(&sense));
                continue;
            }
            if du.alive() && du.f.is_neutral() && du.rest == 0 && du.backstep == 0 {
                let to = math::sub(du.pos, hp);
                requests.push(Request {
                    id: du.id,
                    cost: du.arch.stats().cost,
                    eta: du.eta(&sense),
                    dist: math::len(to),
                    front: math::dot(sense.aim, math::norm(to)),
                    wait: du.wait,
                });
            }
        }
        if let Some(id) = spar.threat.grant(&requests) {
            if let Some(i) = ids.iter().copied().find(|&i| self.enemies[i].duel.as_ref().unwrap().id == id) {
                self.enemies[i].duel.as_mut().unwrap().token = true;
            }
        }

        // Sectors for the waiting.
        if frame.is_multiple_of(SLOT_EVERY) {
            let waiters: Vec<Waiter> = ids
                .iter()
                .map(|&i| self.enemies[i].duel.as_ref().unwrap())
                .filter(|d| d.alive() && !d.token)
                .map(|d| Waiter { id: d.id, pos: d.pos, ring: d.arch.stats().ring, prev: d.slot.map(|s| s.sector) })
                .collect();
            let slots = assign_slots(hp, sense.aim, &waiters, lo, hi);
            for &i in &ids {
                let du = self.enemies[i].duel.as_mut().unwrap();
                du.slot = slots.iter().copied().find(|s| s.id == du.id);
            }
        }

        // Their minds: each moves, and a token holder in reach asks the commit gate to strike.
        let bodies: Vec<(u32, V, f64)> =
            ids.iter().map(|&i| self.enemies[i].duel.as_ref().unwrap()).filter(|d| d.alive()).map(|d| (d.id, d.pos, d.radius())).collect();
        for &i in &ids {
            let du = self.enemies[i].duel.as_mut().unwrap();
            let others: Vec<(V, f64)> = bodies.iter().filter(|b| b.0 != du.id).map(|b| (b.1, b.2)).collect();
            let Some(mv) = du.think(&sense, &arena, &others) else { continue };
            let spar = self.spar.as_mut().unwrap();
            let lands = du.lands_in(mv, hp);
            if spar.threat.may_commit(du.id, lands) && du.begin(mv) {
                spar.threat.commit(du.id, lands);
                let sfx = match mv {
                    MoveId::Slam | MoveId::Sweep => Sfx::Growl,
                    MoveId::Lunge => Sfx::Howl,
                    _ => Sfx::Cackle,
                };
                self.audio.play(sfx);
            }
            let p = self.enemies[i].pos();
            self.enemies[i].m.tile = P::new(p.x.round() as i32, p.y.round() as i32);
        }

        // Their blows on the hero.
        for &i in &ids {
            let du = self.enemies[i].duel.as_ref().unwrap();
            let Some(mv) = du.f.active() else { continue };
            let d = mv.data();
            if du.struck || d.projectile || !math::in_cone(du.pos, du.aim, hp, HERO_RADIUS, d.reach, d.half_angle) {
                continue;
            }
            let (from, damage) = (du.pos, d.damage);
            self.enemies[i].duel.as_mut().unwrap().struck = true;
            if self.blow_on_hero(from, damage, d.hitstop, false) == Defense::Parried {
                let guard = self.spar.as_ref().map(|s| s.kit.guard).unwrap_or(crate::logic::combat::kit::Guard::TRAINING);
                let du = self.enemies[i].duel.as_mut().unwrap();
                du.f.deflect(guard.deflect, guard.riposte);
                du.flash = 8;
                if guard.thorns > 0.0 {
                    let hp = self.hero.pos();
                    self.duel_hit(i, Hit { damage: guard.thorns, ..Default::default() }, 2, hp);
                }
            }
        }

        self.update_bolts(&ids);

        // The end of the bout.
        let alive = self.enemies.iter().filter_map(|e| e.duel.as_ref()).filter(|d| d.alive()).count();
        if alive == 0 && self.spar.as_ref().is_some_and(|s| s.phase == BoutPhase::Fight) {
            self.finish_bout(true);
        }
    }

    /**
     * The bout is over: won (every foe down) or lost (his health gone). The results card shows
     * its numbers through the aftermath, and a line goes into the combat log (`combat_log.csv`
     * by the saves) so playtests leave data to tune from (COMBAT.md phase 2).
     */
    fn finish_bout(&mut self, won: bool) {
        let hp = self.hero.pos();
        let (hero, tier) = (self.hero_kind, self.hero.blade);
        let Some(s) = self.spar.as_mut() else { return };
        s.phase = if won { BoutPhase::Won(AFTERMATH) } else { BoutPhase::Lost(AFTERMATH) };
        if won {
            s.last = Some((s.bout, s.stats));
        } else {
            s.hero.state = State::Reel { kind: Reel::Staggered, left: 90, total: 90 };
        }
        let st = s.stats;
        let line = format!(
            "{},{},{},{},{},{:.2},{:.0},{:.0},{},{},{},{},{},{},{},{},{}",
            crate::platform::now_secs(),
            hero.name(),
            tier,
            s.bout + 1,
            if won { "won" } else { "lost" },
            st.frames as f64 / 60.0,
            st.dealt,
            st.taken,
            st.hits,
            st.hurt,
            st.parries,
            st.blocks,
            st.evades,
            st.breaks,
            st.caught,
            st.guard_breaks,
            st.overlaps
        );
        crate::platform::append_line(LOG_FILE, LOG_HEADER, &line);
        if won {
            self.audio.play(Sfx::Win);
        } else {
            self.audio.play(Sfx::Death);
            self.floater(hp.x, hp.y - 1.4, "DEFEATED", 1.5, Some(0xff6a4a));
        }
    }

    /**
     * A foe's blow (or bolt) from `from` reaches the hero: his fighter decides how he meets it
     * (evade, parry, block, a broken guard, or open), and his health pays for it. At none left
     * the bout is lost; he is never slain in the ring.
     */
    fn blow_on_hero(&mut self, from: V, damage: f64, hitstop: u16, bolt: bool) -> Defense {
        let hp = self.hero.pos();
        let from_front = math::dot(self.hero.aim(), math::norm(math::sub(from, hp))) > 0.0;
        let airborne = self.hero.airborne();
        let stamina = self.hero.stamina;
        let swinging = self.spar.as_ref().is_some_and(|s| s.hero.attack().is_some());
        let Some(spar) = self.spar.as_mut() else { return Defense::Open };
        // Whatever reaches him counts as a landing, however he meets it.
        let f = spar.threat.frame;
        spar.land(f);
        let kit = spar.kit;
        let cost = if bolt && kit.guard.bolt_proof { 0.0 } else { damage * kit.guard.cost };
        let def = spar.hero.defend(airborne, from_front, stamina, cost);
        let mut loss = 0.0;
        match def {
            Defense::Immune => {}
            Defense::Evaded => {
                spar.stats.evades += 1;
                self.floater(hp.x, hp.y - 0.9, "EVADED", 0.6, Some(0x9ad1ff));
            }
            Defense::Parried => {
                spar.stats.parries += 1;
                // The aegis (the tenth shield) gives him back a little with every parry.
                self.hero.hp = (self.hero.hp + kit.guard.aegis as i32).min(self.hero.max_hp);
                self.hitstop = self.hitstop.max(secs(HITSTOP_PARRY));
                self.audio.play(Sfx::Clang);
                self.view.sparkle(&self.level.grid, (hp.x + from.x) / 2.0, (hp.y + from.y) / 2.0, 0.9, 0xffe070, 30);
                self.floater(hp.x, hp.y - 0.9, "PARRY!", 0.9, Some(0xffd040));
            }
            Defense::Blocked => {
                spar.stats.blocks += 1;
                self.hero.drain(cost);
                self.audio.play(Sfx::Clang);
                self.view.spark(&self.level.grid, hp.x, hp.y, 0xc8d0da);
                self.floater(hp.x, hp.y - 0.9, "BLOCKED", 0.6, Some(0xc8d0da));
            }
            Defense::GuardBroken => {
                spar.stats.guard_breaks += 1;
                loss = (damage * kit.guard.break_share * kit.taken).round();
                self.hero.drain(self.hero.stamina);
                self.audio.play(Sfx::ShieldBreak);
                self.floater(hp.x, hp.y - 0.9, "GUARD BROKEN", 1.0, Some(0xff7a3a));
                self.view.shake(0.3);
            }
            Defense::Open => {
                if swinging {
                    spar.stats.caught += 1;
                }
                let r = spar.hero.take_hit(&Hit { damage, hitstun: math::hero_hitstun(damage), ..Default::default() });
                loss = (r.damage * kit.taken).round();
                self.hitstop = self.hitstop.max(secs(hitstop));
                self.audio.play(Sfx::Hurt);
                self.view.shake(0.22);
            }
        }
        if loss > 0.0 {
            self.hero.hp = (self.hero.hp - loss as i32).max(0);
            let wind = self.spar.as_ref().map(|s| (s.kit.second_wind, s.wind_used)).unwrap_or((0.0, true));
            if self.hero.hp == 0 && wind.0 > 0.0 && !wind.1 {
                // The second wind (Valkyrie Call, the last stand, the World Tree): once a bout, the end is put off.
                self.hero.hp = ((self.hero.max_hp as f64 * wind.0).round() as i32).max(1);
                if let Some(s) = self.spar.as_mut() {
                    s.wind_used = true;
                }
                self.audio.play(Sfx::Heal);
                self.view.sparkle(&self.level.grid, hp.x, hp.y, 1.2, 0xfff1a8, 40);
                self.floater(hp.x, hp.y - 1.4, "SECOND WIND", 1.4, Some(0xfff1a8));
            }
            self.floater(hp.x, hp.y - 0.5, format!("-{loss}"), 0.9, Some(0xff4a3a));
            let lost = self.hero.hp == 0;
            let fighting = self.spar.as_ref().is_some_and(|s| s.phase == BoutPhase::Fight);
            if let Some(s) = self.spar.as_mut() {
                s.stats.taken += loss;
                s.stats.hurt += 1;
            }
            if lost && fighting {
                self.finish_bout(false);
            }
        }
        def
    }

    /// Hex bolts fly; one meets the hero (or, parried, the foes); they burn out at the ring's edge.
    fn update_bolts(&mut self, ids: &[usize]) {
        let (lo, hi) = gym::ring_bounds();
        let hp = self.hero.pos();
        let Some(spar) = self.spar.as_mut() else { return };
        let now = spar.threat.frame;
        let mut bolts = std::mem::take(&mut spar.bolts);
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
            if b.pos.x < lo.x || b.pos.x > hi.x || b.pos.y < lo.y || b.pos.y > hi.y {
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
                        let reflect = self.spar.as_ref().map(|s| s.kit.guard.reflect).unwrap_or(1.0);
                        b.vel = math::scale(b.vel, -REFLECT_SPEED);
                        b.reflected = true;
                        b.damage *= reflect;
                        b.poise = REFLECT_POISE;
                    } else if !self.hero.airborne() {
                        b.life = 0;
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
        if let Some(s) = self.spar.as_mut() {
            let fresh = std::mem::take(&mut s.bolts);
            s.bolts = bolts;
            s.bolts.extend(fresh);
        }
    }

    /// Into the Sparring Ring (from anywhere), for bout `n` (1-based; 0 keeps the one it is on): the
    /// bout's countdown starts at once. (Captures and tests.)
    pub fn debug_ring(&mut self, n: usize) {
        if !self.theme().is_gym() {
            self.map_jump(super::save::Stage::Gym);
        }
        self.enter(super::Screen::Playing);
        self.hero.place(gym::ring_start());
        self.hero.face(crate::logic::types::Dir::Left);
        if let Some(s) = self.spar.as_mut() {
            if n > 0 {
                s.bout = (n - 1) % gym::BOUTS.len();
            }
        }
        self.start_bout();
    }

    /// The hero's buttons in the ring: attack and evade into his fighter's buffer; the guard as held.
    pub(super) fn spar_input(&mut self, attack: bool, evade: bool, guard: bool) {
        let Some(s) = self.spar.as_mut() else { return };
        if attack {
            s.hero.press(Input::Attack);
        }
        if evade {
            s.hero.press(Input::Evade);
        }
        s.hero.guard_held = guard;
    }
}
