//! The fighter's state machine (COMBAT.md §3): one struct for the hero and every foe. It runs
//! one step per 60 Hz frame, keeps a one-slot input buffer, applies the cancel rules, and takes
//! blows through poise, armor and decaying hitstun.

use super::frames::{MoveData, MoveId, Phase};
use super::kit::Kit;
use super::math::{self, COMBO_RESET, POISE_REGEN_DELAY, POISE_REGEN_FRAMES, PUNISH_MULT, VULNERABLE_MULT};

/// Frames a press waits in the buffer for a state that can take it.
pub const BUFFER: u16 = 8;
/// Frames after a move ends in which an attack still continues its string.
pub const CHAIN_GRACE: u16 = 12;
/// The guard is up from its second frame; a blow on guard frames PARRY_FROM..=PARRY_TO is parried.
pub const GUARD_RAISE: u16 = 2;
pub const PARRY_FROM: u16 = 2;
pub const PARRY_TO: u16 = 9;
/// Frames after lowering the guard before it can be raised again (no parry by mashing).
pub const GUARD_LOCK: u16 = 10;
/// The evade: frames in the air (untouchable) and on landing.
pub const EVADE_AIR: u16 = 25;
pub const EVADE_LAND: u16 = 6;
pub const EVADE_TOTAL: u16 = EVADE_AIR + EVADE_LAND;
/// The hero's untouchable frames after a blow lands on him.
pub const MERCY: u16 = 30;
/// Frames a parried attacker reels (longer than any recovery, so a parry always opens more than a
/// dodge), and the share of its poise it loses.
pub const DEFLECT: u16 = 60;
pub const PARRY_POISE: f64 = 0.5;
/// Stamina a blocked blow costs, per point of its damage (the training buckler's stability, 30%).
pub const BLOCK_COST: f64 = 0.7;
/// Frames the hero reels when his guard breaks, and the share of the blow that gets through.
pub const GUARD_BREAK: u16 = 48;
pub const GUARD_BREAK_SHARE: f64 = 0.5;

/// Why a fighter reels.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reel {
    Hitstun,
    Blockstun,
    /// Poise broken.
    Staggered,
    /// Parried.
    Deflected,
    GuardBroken,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    /// Free: walking, waiting, ready to act.
    Neutral,
    /// Performing a move; `frame` counts from 0, its first startup frame.
    Attack {
        mv: MoveId,
        frame: u16,
    },
    /// The guard, raised `frame` frames ago.
    Guard {
        frame: u16,
    },
    /// The evade (the hero's leap), `frame` frames since take-off.
    Evade {
        frame: u16,
    },
    /// Reeling from a blow: nothing can be done for `left` more frames.
    Reel {
        kind: Reel,
        left: u16,
        total: u16,
    },
    Dead,
}

/// A press the buffer holds. (The guard is held, not pressed: see `Fighter::guard_held`.)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Input {
    Attack,
    Evade,
}

/// What a frame brought about.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Event {
    /// A move began (its frame 0).
    Began(MoveId),
    /// Its first active frame.
    Active(MoveId),
    /// It ran its course.
    Ended(MoveId),
    /// The fighter evades now: the owner starts the leap and calls `evade`.
    Evade,
    /// A reel ran out.
    Recovered(Reel),
}

/// A blow, as it arrives: its damage (power already in it), poise damage and hitstun; poise it
/// adds to the armor check alone (armor-piercing), and what it adds to the multiplier on an
/// opening (a critical blow).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Hit {
    pub damage: f64,
    pub poise: f64,
    pub hitstun: u16,
    pub pierce: f64,
    pub crit: f64,
}

/// What a blow did to the struck.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Effect {
    /// The armor held: damage, but no flinch.
    Armored,
    /// Flinched, for this many frames of hitstun.
    Flinched(u16),
    /// Poise broken: staggered.
    Broken,
    /// Already reeling (staggered, deflected, guard broken): it keeps reeling.
    Held,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Received {
    pub damage: f64,
    pub effect: Effect,
}

/// What the guard can do against a blow: block it or parry it (most), only parry it (a grab:
/// no shield holds it), or nothing at all (what bursts from the floor or falls from the sky).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GuardRule {
    Full,
    ParryOnly,
    /// Only the shield holds it (a beam of light): no parry turns it, and a guard raised in the
    /// parry window blocks it.
    BlockOnly,
    Bare,
}

/// How the hero met a blow that reached him.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Defense {
    /// In the air: it passed under him.
    Evaded,
    /// In his mercy frames after a blow: nothing lands.
    Immune,
    Parried,
    Blocked,
    GuardBroken,
    /// Nothing between him and it.
    Open,
}

#[derive(Clone, Debug)]
pub struct Fighter {
    pub state: State,
    pub poise: f64,
    /// No poise at all (0) means it flinches from everything and never breaks (the hero).
    pub max_poise: f64,
    /// Poise damage a blow needs to make it flinch outside its moves.
    pub armor: f64,
    /// Frames it stays staggered when its poise breaks, and frames of resolve afterwards.
    pub stagger: u16,
    pub resolve_frames: u16,
    /// Frames left in which its poise cannot break again.
    pub resolve: u16,
    /// Hits taken in the current combo.
    pub combo: u16,
    /// Untouchable frames after a blow, and how many each blow gives (the hero's mercy; none for foes).
    pub mercy: u16,
    pub mercy_frames: u16,
    /// Set by the owner each frame: may it start an attack (the hero's stamina)?
    pub can_attack: bool,
    /// Set by the owner each frame: is the guard button held?
    pub guard_held: bool,
    /// The hero's string as his kit swings it (L1, L2, L3); foes' moves come from the table.
    pub table: [MoveData; 3],
    /// The last guard frame that parries, the evade's whole length, and how long a broken guard reels.
    pub parry_to: u16,
    pub evade_total: u16,
    pub break_frames: u16,
    regen_wait: u16,
    calm: u16,
    grace: Option<(MoveId, u16)>,
    buffer: Option<(Input, u16)>,
    guard_lock: u16,
}

impl Fighter {
    pub fn new(max_poise: f64, armor: f64, stagger: u16, resolve_frames: u16) -> Self {
        Self {
            state: State::Neutral,
            poise: max_poise,
            max_poise,
            armor,
            stagger,
            resolve_frames,
            resolve: 0,
            combo: 0,
            mercy: 0,
            mercy_frames: 0,
            can_attack: true,
            guard_held: false,
            regen_wait: 0,
            calm: COMBO_RESET,
            grace: None,
            buffer: None,
            guard_lock: 0,
            table: MoveId::HERO.map(MoveId::data),
            parry_to: PARRY_TO,
            evade_total: EVADE_TOTAL,
            break_frames: GUARD_BREAK,
        }
    }

    /// The hero: no poise, mercy frames after every blow, the training kit.
    pub fn hero() -> Self {
        Self { mercy_frames: MERCY, ..Self::new(0.0, 0.0, 0, 0) }
    }

    /// The hero with `kit`: his string, his parry window, his evade, his broken guard.
    pub fn with_kit(kit: &Kit) -> Self {
        Self {
            table: kit.moves(),
            parry_to: kit.guard.parry_to(),
            evade_total: EVADE_AIR + kit.evade_land,
            break_frames: kit.guard.break_frames,
            ..Self::hero()
        }
    }

    /// A move's frame data as this fighter performs it.
    pub fn data(&self, mv: MoveId) -> MoveData {
        match mv {
            MoveId::L1 => self.table[0],
            MoveId::L2 => self.table[1],
            MoveId::L3 => self.table[2],
            _ => mv.data(),
        }
    }

    /// A press, held in the buffer until a state can take it (or it grows stale).
    pub fn press(&mut self, input: Input) {
        self.buffer = Some((input, 0));
    }

    /// What the buffer holds.
    pub fn buffered(&self) -> Option<Input> {
        self.buffer.map(|b| b.0)
    }

    /// The move under way, its phase and frame.
    pub fn attack(&self) -> Option<(MoveId, Phase, u16)> {
        match self.state {
            State::Attack { mv, frame } => Some((mv, self.data(mv).phase(frame), frame)),
            _ => None,
        }
    }

    /// On an active frame of `mv`.
    pub fn active(&self) -> Option<MoveId> {
        self.attack().filter(|a| a.1 == Phase::Active).map(|a| a.0)
    }

    pub fn is_dead(&self) -> bool {
        self.state == State::Dead
    }

    pub fn is_neutral(&self) -> bool {
        self.state == State::Neutral
    }

    /// Its poise is coming back (no blow for a while).
    pub fn regenerating(&self) -> bool {
        self.regen_wait == 0
    }

    pub fn reeling(&self) -> Option<Reel> {
        match self.state {
            State::Reel { kind, .. } => Some(kind),
            _ => None,
        }
    }

    /// Open to the ×1.5 of a staggered, deflected or guard-broken fighter.
    pub fn vulnerable(&self) -> bool {
        matches!(self.reeling(), Some(Reel::Staggered | Reel::Deflected | Reel::GuardBroken))
    }

    /// The guard is up, and on a parry frame.
    pub fn parrying(&self) -> bool {
        matches!(self.state, State::Guard { frame } if (PARRY_FROM..=self.parry_to).contains(&frame))
    }

    /// The share of its pace a foe keeps in its state (planted in a swing, rooted while reeling).
    pub fn root(&self) -> f64 {
        match self.state {
            State::Neutral | State::Guard { .. } | State::Evade { .. } => 1.0,
            State::Attack { mv, frame } => match self.data(mv).phase(frame) {
                Phase::Startup => 0.3,
                Phase::Active => 0.15,
                Phase::Recovery => 0.4,
            },
            State::Reel { kind: Reel::Blockstun, .. } => 0.2,
            State::Reel { .. } | State::Dead => 0.0,
        }
    }

    /// The share of his pace the hero keeps in his state: planted in his own swing, and by nothing
    /// else. No blow slows his feet or roots him: its hitstun, a block's stun and a broken guard only
    /// keep him from swinging or guarding for their frames.
    pub fn stride(&self) -> f64 {
        match self.state {
            State::Attack { .. } => self.root(),
            _ => 1.0,
        }
    }

    /// Damage multiplier for a blow on it now (COMBAT.md §2, `S`).
    pub fn vulnerability(&self) -> f64 {
        if self.vulnerable() {
            VULNERABLE_MULT
        } else if matches!(self.attack(), Some((_, Phase::Recovery, _))) {
            PUNISH_MULT
        } else {
            1.0
        }
    }

    /// Poise damage a blow needs to make it flinch now.
    pub fn current_armor(&self) -> f64 {
        match self.state {
            State::Attack { mv, frame } => match self.data(mv).phase(frame) {
                Phase::Recovery => 0.0,
                _ => self.armor.max(self.data(mv).armor),
            },
            State::Reel { .. } => 0.0,
            _ => self.armor,
        }
    }

    /// Starts `mv` now, if it is free to (a foe's choice; the hero's attacks come through the buffer).
    pub fn begin(&mut self, mv: MoveId) -> bool {
        if self.state != State::Neutral {
            return false;
        }
        self.state = State::Attack { mv, frame: 0 };
        self.grace = None;
        true
    }

    /// The leap took off.
    pub fn evade(&mut self) {
        self.state = State::Evade { frame: 0 };
    }

    pub fn kill(&mut self) {
        self.state = State::Dead;
    }

    /// One frame: timers, the state's own clock, the guard, then the buffer.
    pub fn step(&mut self) -> Vec<Event> {
        let mut ev = Vec::new();
        if self.state == State::Dead {
            return ev;
        }
        self.mercy = self.mercy.saturating_sub(1);
        self.resolve = self.resolve.saturating_sub(1);
        self.guard_lock = self.guard_lock.saturating_sub(1);
        if self.regen_wait > 0 {
            self.regen_wait -= 1;
        } else if self.max_poise > 0.0 {
            self.poise = (self.poise + self.max_poise / POISE_REGEN_FRAMES).min(self.max_poise);
        }
        self.grace = self.grace.and_then(|(m, t)| (t < CHAIN_GRACE).then_some((m, t + 1)));

        match self.state {
            State::Reel { kind, left, total } => {
                if left <= 1 {
                    self.state = State::Neutral;
                    if kind == Reel::Staggered {
                        self.poise = self.max_poise;
                        self.resolve = self.resolve_frames;
                    }
                    if kind == Reel::Blockstun && self.guard_held {
                        // The guard stays up through the blockstun: no fresh parry window from it.
                        self.state = State::Guard { frame: self.parry_to + 1 };
                    }
                    ev.push(Event::Recovered(kind));
                } else {
                    self.state = State::Reel { kind, left: left - 1, total };
                }
            }
            State::Attack { mv, frame } => {
                let d = self.data(mv);
                let f = frame + 1;
                if f >= d.total() {
                    self.state = State::Neutral;
                    self.grace = Some((mv, 0));
                    ev.push(Event::Ended(mv));
                } else {
                    self.state = State::Attack { mv, frame: f };
                    if f == d.startup {
                        ev.push(Event::Active(mv));
                    }
                }
            }
            State::Guard { frame } => self.state = State::Guard { frame: frame.saturating_add(1) },
            State::Evade { frame } => {
                self.state = if frame + 1 >= self.evade_total { State::Neutral } else { State::Evade { frame: frame + 1 } };
            }
            State::Neutral | State::Dead => {}
        }

        if matches!(self.reeling(), Some(Reel::Hitstun | Reel::Staggered | Reel::Deflected)) {
            self.calm = 0;
        } else {
            self.calm = self.calm.saturating_add(1);
            if self.calm >= COMBO_RESET {
                self.combo = 0;
            }
        }

        self.apply_guard();

        if let Some((input, age)) = self.buffer {
            if self.consume(input, &mut ev) {
                self.buffer = None;
            } else if self.holds_attack(input) {
                // Pressed during a move's swing: it waits for the cancel frame without going stale.
            } else if age + 1 >= BUFFER {
                self.buffer = None;
            } else {
                self.buffer = Some((input, age + 1));
            }
        }
        ev
    }

    /// The guard follows the button: raised from neutral (or out of a recovery's cancel frame), lowered on release.
    fn apply_guard(&mut self) {
        match self.state {
            State::Guard { .. } if !self.guard_held => {
                self.state = State::Neutral;
                self.guard_lock = GUARD_LOCK;
            }
            State::Neutral if self.guard_held && self.guard_lock == 0 => self.state = State::Guard { frame: 0 },
            State::Attack { mv, frame } if self.guard_held && frame >= self.data(mv).cancel_frame() => {
                self.state = State::Guard { frame: 0 }
            }
            _ => {}
        }
    }

    /// An attack pressed after a move's startup is kept until its cancel frame.
    fn holds_attack(&self, input: Input) -> bool {
        match (input, self.state) {
            (Input::Attack, State::Attack { mv, frame }) => {
                let d = self.data(mv);
                d.chain.is_some() && frame >= d.startup && frame < d.cancel_frame()
            }
            _ => false,
        }
    }

    /// Takes the buffered press if the state allows it (the cancel rules of COMBAT.md §3).
    fn consume(&mut self, input: Input, ev: &mut Vec<Event>) -> bool {
        match input {
            Input::Attack => {
                if !self.can_attack {
                    return false;
                }
                let next = match self.state {
                    State::Neutral => Some(self.grace.and_then(|(m, _)| self.data(m).chain).unwrap_or(MoveId::L1)),
                    State::Attack { mv, frame } if frame >= self.data(mv).cancel_frame() => self.data(mv).chain,
                    _ => None,
                };
                let Some(mv) = next else { return false };
                self.state = State::Attack { mv, frame: 0 };
                self.grace = None;
                ev.push(Event::Began(mv));
                true
            }
            Input::Evade => {
                let free = match self.state {
                    State::Neutral | State::Guard { .. } => true,
                    State::Attack { mv, frame } => {
                        let d = self.data(mv);
                        frame < d.startup || frame >= d.cancel_frame()
                    }
                    _ => false,
                };
                if free {
                    ev.push(Event::Evade);
                }
                free
            }
        }
    }

    /**
     * A blow reached the hero: in the air it passes under him, in his mercy frames it does nothing; on a raised
     * guard from the front it is parried (the window), blocked (if he has the `stamina` for its
     * `cost`), or it breaks the guard, as far as `rule` lets the guard count. Otherwise he is
     * open, and the owner applies the hit.
     */
    pub fn defend(&mut self, airborne: bool, from_front: bool, stamina: f64, cost: f64, rule: GuardRule) -> Defense {
        if airborne {
            return Defense::Evaded;
        }
        if self.mercy > 0 {
            return Defense::Immune;
        }
        let State::Guard { frame } = self.state else { return Defense::Open };
        if frame < GUARD_RAISE || !from_front || rule == GuardRule::Bare {
            return Defense::Open;
        }
        if frame <= self.parry_to && rule != GuardRule::BlockOnly {
            return Defense::Parried;
        }
        if rule == GuardRule::ParryOnly {
            return Defense::Open;
        }
        if stamina + 1e-9 >= cost {
            let stun = math::blockstun(cost);
            self.state = State::Reel { kind: Reel::Blockstun, left: stun, total: stun };
            return Defense::Blocked;
        }
        self.state = State::Reel { kind: Reel::GuardBroken, left: self.break_frames, total: self.break_frames };
        Defense::GuardBroken
    }

    /// A blow lands: the damage it deals here (state and proration), the poise it takes, and whether it flinches or breaks.
    pub fn take_hit(&mut self, hit: &Hit) -> Received {
        let open = self.vulnerability();
        let state = if open > 1.0 { open + hit.crit } else { 1.0 };
        self.combo = self.combo.saturating_add(1);
        let n = self.combo;
        let damage = math::damage(hit.damage, 1.0, state, n);
        let armor = self.current_armor();
        self.mercy = self.mercy_frames;
        self.calm = 0;
        if self.max_poise > 0.0 {
            self.poise -= hit.poise;
            self.regen_wait = POISE_REGEN_DELAY;
            if self.poise <= 0.0 {
                self.poise = 0.0;
                if self.resolve == 0 && !self.vulnerable() {
                    self.state = State::Reel { kind: Reel::Staggered, left: self.stagger, total: self.stagger };
                    return Received { damage, effect: Effect::Broken };
                }
            }
        }
        if self.vulnerable() {
            return Received { damage, effect: Effect::Held };
        }
        if hit.poise + hit.pierce >= armor {
            let hs = math::hitstun(hit.hitstun, n);
            self.state = State::Reel { kind: Reel::Hitstun, left: hs, total: hs };
            return Received { damage, effect: Effect::Flinched(hs) };
        }
        Received { damage, effect: Effect::Armored }
    }

    /// Parried: it reels for `frames`, and half its poise goes and `riposte` more (all of it
    /// breaks it, as any blow would).
    pub fn deflect(&mut self, frames: u16, riposte: f64) {
        if self.max_poise > 0.0 {
            self.poise -= self.max_poise * PARRY_POISE + riposte;
            self.regen_wait = POISE_REGEN_DELAY;
            if self.poise <= 0.0 && self.resolve == 0 {
                self.poise = 0.0;
                self.state = State::Reel { kind: Reel::Staggered, left: self.stagger, total: self.stagger };
                return;
            }
            self.poise = self.poise.max(0.0);
        }
        self.state = State::Reel { kind: Reel::Deflected, left: frames, total: frames };
    }

    /// A shock to its poise without a blow (a kill's terror beside it): it breaks if it runs out.
    pub fn shake(&mut self, poise: f64) -> bool {
        if self.max_poise <= 0.0 || self.is_dead() {
            return false;
        }
        self.poise -= poise;
        self.regen_wait = POISE_REGEN_DELAY;
        if self.poise <= 0.0 {
            self.poise = 0.0;
            if self.resolve == 0 && !self.vulnerable() {
                self.state = State::Reel { kind: Reel::Staggered, left: self.stagger, total: self.stagger };
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Steps `f` until `frames` frames have run, pressing `presses` on the given frames; returns each event with its frame.
    fn run(f: &mut Fighter, frames: u16, presses: &[(u16, Input)]) -> Vec<(u16, Event)> {
        let mut out = Vec::new();
        for t in 0..frames {
            for &(at, i) in presses {
                if at == t {
                    f.press(i);
                }
            }
            for e in f.step() {
                out.push((t, e));
            }
        }
        out
    }

    fn began(log: &[(u16, Event)]) -> Vec<(u16, MoveId)> {
        log.iter().filter_map(|&(t, e)| if let Event::Began(m) = e { Some((t, m)) } else { None }).collect()
    }

    #[test]
    fn a_press_held_through_the_swing_comes_out_as_the_next_hit_on_its_cancel_frame() {
        let mut f = Fighter::hero();
        // Mashed early: L2 still comes out on L1's cancel frame, L3 on L2's.
        let log = run(&mut f, 70, &[(0, Input::Attack), (2, Input::Attack), (16, Input::Attack)]);
        assert_eq!(began(&log), [(0, MoveId::L1), (14, MoveId::L2), (27, MoveId::L3)]);
        let actives: Vec<u16> = log.iter().filter(|e| matches!(e.1, Event::Active(_))).map(|e| e.0).collect();
        assert_eq!(actives, [6, 19, 36], "the hits land where COMBAT.md says");
    }

    #[test]
    fn the_string_restarts_after_its_grace_and_a_stale_press_is_dropped() {
        let mut f = Fighter::hero();
        // L1 ends on frame 24; a press 12 frames later still chains, one later does not.
        let log = run(&mut f, 80, &[(0, Input::Attack), (24 + CHAIN_GRACE, Input::Attack)]);
        assert_eq!(began(&log)[1].1, MoveId::L2);
        let mut f = Fighter::hero();
        let log = run(&mut f, 80, &[(0, Input::Attack), (25 + CHAIN_GRACE, Input::Attack)]);
        assert_eq!(began(&log)[1].1, MoveId::L1);
        // A press while reeling waits BUFFER frames, no more.
        let mut f = Fighter::hero();
        f.take_hit(&Hit { damage: 10.0, poise: 0.0, hitstun: 20, ..Default::default() });
        let log = run(&mut f, 40, &[(5, Input::Attack)]);
        assert!(began(&log).is_empty(), "pressed 15 frames before the hitstun ends");
        let log = run(&mut f, 40, &[]);
        assert!(began(&log).is_empty());
    }

    #[test]
    fn the_evade_cancels_startup_and_recovery_but_never_the_active_frames() {
        let mut f = Fighter::hero();
        let log = run(&mut f, 6, &[(0, Input::Attack), (3, Input::Evade)]);
        assert!(log.contains(&(3, Event::Evade)), "out of the startup");
        let mut f = Fighter::hero();
        let log = run(&mut f, 30, &[(0, Input::Attack), (7, Input::Evade)]);
        let at = log.iter().find(|e| e.1 == Event::Evade).map(|e| e.0);
        assert_eq!(at, Some(MoveId::L1.data().cancel_frame()), "held through the active frames, out at the cancel frame");
    }

    #[test]
    fn hits_decay_poise_breaks_and_resolve_stops_a_second_break() {
        // A Skirmisher: poise 25, no armor.
        let mut f = Fighter::new(25.0, 0.0, 50, 60);
        let l1 = Hit { damage: 10.0, poise: 20.0, hitstun: 14, ..Default::default() };
        let r = f.take_hit(&l1);
        assert_eq!(r.effect, Effect::Flinched(14));
        let r = f.take_hit(&l1);
        assert_eq!(r.effect, Effect::Broken, "40 poise from 25");
        assert_eq!(r.damage, 9.0, "the second hit of a combo deals 92%");
        let r = f.take_hit(&Hit { damage: 16.0, poise: 45.0, hitstun: 24, ..Default::default() });
        assert_eq!((r.effect, r.damage), (Effect::Held, 20.0), "x1.5 on the staggered, x0.84 for the third hit");
        for _ in 0..50 {
            f.step();
        }
        assert!(f.is_neutral() && f.poise == 25.0 && f.resolve > 0);
        // Resolve: emptied again, it only flinches.
        let r = f.take_hit(&Hit { damage: 10.0, poise: 40.0, hitstun: 14, ..Default::default() });
        assert!(matches!(r.effect, Effect::Flinched(_)));
    }

    #[test]
    fn the_brutes_armor_shrugs_off_light_blows_but_not_the_finisher() {
        let mut f = Fighter::new(100.0, 35.0, 90, 120);
        assert_eq!(f.take_hit(&Hit { damage: 10.0, poise: 20.0, hitstun: 14, ..Default::default() }).effect, Effect::Armored);
        assert!(f.begin(MoveId::Slam));
        assert_eq!(
            f.take_hit(&Hit { damage: 11.0, poise: 20.0, hitstun: 14, ..Default::default() }).effect,
            Effect::Armored,
            "nor mid-slam"
        );
        assert!(matches!(f.take_hit(&Hit { damage: 16.0, poise: 45.0, hitstun: 24, ..Default::default() }).effect, Effect::Flinched(_)));
        // In recovery its armor is gone and it takes the punish multiplier.
        let mut f = Fighter::new(100.0, 35.0, 90, 120);
        f.begin(MoveId::Slam);
        for _ in 0..50 {
            f.step();
        }
        assert_eq!(f.attack().map(|a| a.1), Some(Phase::Recovery));
        let r = f.take_hit(&Hit { damage: 10.0, poise: 20.0, hitstun: 14, ..Default::default() });
        assert_eq!((r.damage, r.effect), (13.0, Effect::Flinched(14)));
        // Poise comes back after its delay.
        let mut f = Fighter::new(100.0, 35.0, 90, 120);
        f.take_hit(&Hit { damage: 10.0, poise: 50.0, hitstun: 14, ..Default::default() });
        for _ in 0..POISE_REGEN_DELAY {
            f.step();
        }
        assert_eq!(f.poise, 50.0);
        for _ in 0..45 {
            f.step();
        }
        assert!((f.poise - 100.0).abs() < 1e-9);
    }

    #[test]
    fn the_guard_parries_in_its_window_blocks_after_it_and_breaks_without_stamina() {
        let mut f = Fighter::hero();
        f.guard_held = true;
        f.step();
        assert_eq!(f.defend(false, true, 100.0, 15.0, GuardRule::Full), Defense::Open, "still rising on frame 0");
        f.step();
        f.step();
        assert_eq!(f.defend(false, true, 100.0, 15.0, GuardRule::Full), Defense::Parried);
        assert_eq!(f.defend(false, false, 100.0, 15.0, GuardRule::Full), Defense::Open, "nothing from behind");
        // A beam in the window: only the shield holds it, no parry.
        let mut g = f.clone();
        assert_eq!(g.defend(false, true, 100.0, 15.0, GuardRule::BlockOnly), Defense::Blocked);
        for _ in 0..8 {
            f.step();
        }
        assert_eq!(f.defend(false, true, 100.0, 15.0, GuardRule::Full), Defense::Blocked);
        for _ in 0..30 {
            f.step();
        }
        assert!(matches!(f.state, State::Guard { frame } if frame > PARRY_TO), "the guard stays up, past its window");
        assert_eq!(f.defend(false, true, 5.0, 15.0, GuardRule::Full), Defense::GuardBroken);
        assert!(f.vulnerable());
        // Lowered, it cannot be raised again at once.
        let mut f = Fighter::hero();
        f.guard_held = true;
        f.step();
        f.guard_held = false;
        f.step();
        f.guard_held = true;
        for _ in 0..GUARD_LOCK - 1 {
            f.step();
            assert!(f.is_neutral());
        }
        f.step();
        assert_eq!(f.state, State::Guard { frame: 0 });
        assert_eq!(f.defend(true, true, 0.0, 99.0, GuardRule::Full), Defense::Evaded, "in the air nothing reaches him");
    }

    #[test]
    fn no_blow_on_the_hero_slows_his_feet_only_his_own_swing_plants_him() {
        let mut f = Fighter::hero();
        f.take_hit(&Hit { damage: 20.0, hitstun: 20, ..Default::default() });
        assert_eq!((f.reeling(), f.stride()), (Some(Reel::Hitstun), 1.0), "struck, he walks on");
        let mut f = Fighter::hero();
        f.guard_held = true;
        for _ in 0..12 {
            f.step();
        }
        assert_eq!(f.defend(false, true, 100.0, 15.0, GuardRule::Full), Defense::Blocked);
        assert_eq!((f.reeling(), f.stride()), (Some(Reel::Blockstun), 1.0), "blocking, he walks on");
        assert_eq!(f.defend(false, true, 0.0, 15.0, GuardRule::Full), Defense::Open, "mid-blockstun the guard is not up");
        f.state = State::Guard { frame: 20 };
        assert_eq!(f.defend(false, true, 0.0, 15.0, GuardRule::Full), Defense::GuardBroken);
        assert_eq!((f.reeling(), f.stride()), (Some(Reel::GuardBroken), 1.0), "his guard broken, he walks on");
        // His own swing still plants him (a foe's planted the same way).
        let mut f = Fighter::hero();
        f.press(Input::Attack);
        f.step();
        assert!(f.stride() < 1.0 && f.stride() == f.root());
    }

    #[test]
    fn a_parried_attacker_reels_and_a_deflect_on_empty_poise_staggers() {
        let mut f = Fighter::new(25.0, 0.0, 50, 60);
        f.begin(MoveId::Bite);
        f.deflect(DEFLECT, 0.0);
        assert_eq!(f.reeling(), Some(Reel::Deflected));
        assert!((f.poise - 12.5).abs() < 1e-9);
        let mut f = Fighter::new(25.0, 0.0, 50, 60);
        f.poise = 10.0;
        f.deflect(DEFLECT, 0.0);
        assert_eq!(f.reeling(), Some(Reel::Staggered));
    }
}
