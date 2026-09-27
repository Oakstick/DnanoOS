//! The Cyclops: a floating eye the size of a hut, the first boss. It drifts
//! around the meadow keeping its distance, squints to telegraph, then hurls
//! bolts and boulders at Ragnar. Wounded, it grows faster and angrier,
//! and starts diving at him; a dive that misses leaves it dazed on the grass,
//! which is the moment to close in with the axe.
//!
//! Positions are in tile units like everything else; the Cyclops flies, so
//! walls, water and pits never stop it, only the arena bounds.

use super::rng::Rng;
use super::types::{dist_sq, V};

/// Health of the first Cyclops; each later one has `CYCLOPS_HP_STEP` more.
pub const CYCLOPS_HP: i32 = 40;
pub const CYCLOPS_HP_STEP: i32 = 12;
/// How far from its center the body reaches, in tiles (for hits both ways).
pub const BODY_RADIUS: f64 = 0.85;
/// Touching it hurts as much as touching any foe.
pub const CONTACT_RADIUS: f64 = 1.0;
/// Resting hover height of the body's underside above the grass, in world units.
pub const HOVER: f64 = 0.55;
/// Where it hangs before it drops in.
pub const SKY: f64 = 7.0;
const ARRIVE_SECONDS: f64 = 1.8;
const MAX_SPEED: f64 = 2.7;
const DIVE_SPEED: f64 = 10.0;
const DAZED_SECONDS: f64 = 2.4;
const DYING_SECONDS: f64 = 2.6;
/// Brief immunity after a hit, so one blow never lands twice.
const HIT_GRACE: f64 = 0.12;
/// Points for felling it.
pub const POINTS_CYCLOPS: u32 = 5000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShotKind {
    /// A glowing bolt from the eye: fast, straight, stings.
    Bolt,
    /// A boulder torn from the ground: slow, rolls, hurts a lot.
    Boulder,
}

impl ShotKind {
    pub fn radius(self) -> f64 {
        match self {
            ShotKind::Bolt => 0.22,
            ShotKind::Boulder => 0.4,
        }
    }
    pub fn damage(self) -> i32 {
        match self {
            ShotKind::Bolt => 15,
            ShotKind::Boulder => 25,
        }
    }
}

/// Something the Cyclops threw. Flies until it hits rock, Ragnar, or its range.
#[derive(Clone, Debug)]
pub struct BossShot {
    pub pos: V,
    pub vel: V,
    pub kind: ShotKind,
    pub life: f64,
    pub alive: bool,
    /// Render-only: how far it has rolled, for the boulder's spin.
    pub rolled: f64,
}

impl BossShot {
    pub fn new(pos: V, dir: f64, speed: f64, kind: ShotKind) -> Self {
        Self { pos, vel: V::new(dir.cos() * speed, dir.sin() * speed), kind, life: 5.0, alive: true, rolled: 0.0 }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Attack {
    /// A fan of bolts aimed at him.
    Fan,
    /// A ring of bolts in every direction.
    Ring,
    /// A quick stream of bolts that tracks him.
    Stream,
    /// Boulders lobbed around him.
    Boulders,
    /// A swoop at him that ends in a slam.
    Dive,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BossState {
    /// Waiting in the sky for the countdown to end.
    Hidden,
    /// Dropping out of the sky.
    Arrive(f64),
    /// Moving to a new spot around him.
    Drift(f64),
    /// Squinting, eye glowing: an attack is coming.
    Charge(f64, Attack),
    /// Throwing: seconds in, shots fired so far.
    Volley(f64, Attack, u32),
    /// Swooping at a point.
    Dive(V),
    /// Stuck in the grass after a slam.
    Dazed(f64),
    /// Getting back up, briefly harmless.
    Recover(f64),
    Dying(f64),
    Dead,
}

/// What the game should make noise and light about this tick.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BossEvent {
    Roar,
    Charge,
    Fire(ShotKind),
    Slam(V),
    /// Health crossed into a new phase (2 or 3).
    Enraged(u8),
    /// A small burst while it dies.
    Spasm(V),
    Died(V),
}

#[derive(Clone, Debug)]
pub struct Cyclops {
    pub pos: V,
    vel: V,
    pub hp: i32,
    pub max_hp: i32,
    pub state: BossState,
    /// Height of the body's underside above the floor, world units.
    pub hover: f64,
    /// Where it looks, radians in the ground plane (x = cos, y = sin).
    pub look: f64,
    /// 1 when just hit, fading.
    pub flash: f64,
    grace: f64,
    target: V,
    cycle: u32,
    min: V,
    max: V,
    rng: Rng,
    spasm_timer: f64,
}

impl Cyclops {
    /// A Cyclops waiting above `home`, confined to the rectangle `min..max`.
    pub fn new(home: V, min: V, max: V, max_hp: i32, seed: u32) -> Self {
        Self {
            pos: home,
            vel: V::default(),
            hp: max_hp,
            max_hp,
            state: BossState::Hidden,
            hover: SKY,
            look: std::f64::consts::FRAC_PI_2,
            flash: 0.0,
            grace: 0.0,
            target: home,
            cycle: 0,
            min,
            max,
            rng: Rng::new(seed),
            spasm_timer: 0.0,
        }
    }

    /// 1 above two thirds health, 2 above a third, 3 below.
    pub fn phase(&self) -> u8 {
        let f = self.hp as f64 / self.max_hp as f64;
        if f > 2.0 / 3.0 {
            1
        } else if f > 1.0 / 3.0 {
            2
        } else {
            3
        }
    }

    pub fn alive(&self) -> bool {
        !matches!(self.state, BossState::Dying(_) | BossState::Dead)
    }

    /// In the fight: arrived and not yet dying.
    pub fn active(&self) -> bool {
        self.alive() && !matches!(self.state, BossState::Hidden | BossState::Arrive(_))
    }

    /// Can blows land right now?
    pub fn vulnerable(&self) -> bool {
        self.active() && self.grace <= 0.0
    }

    pub fn dazed(&self) -> bool {
        matches!(self.state, BossState::Dazed(_))
    }

    /// Does touching it hurt? Not while it is stunned in the grass or getting up.
    pub fn harmful(&self) -> bool {
        self.active() && !matches!(self.state, BossState::Dazed(_) | BossState::Recover(_))
    }

    /// 0..1 eye glow for the renderer: builds through a charge.
    pub fn glow(&self) -> f64 {
        match self.state {
            BossState::Charge(t, _) => (t / self.charge_seconds()).min(1.0),
            BossState::Volley(..) => 1.0,
            BossState::Dive(_) => 0.8,
            _ => 0.0,
        }
    }

    /// 0 open .. 1 shut, for the eyelids.
    pub fn squint(&self) -> f64 {
        match self.state {
            BossState::Charge(t, _) => 0.45 * (t / self.charge_seconds()).min(1.0),
            BossState::Dazed(_) => 0.6,
            BossState::Dying(t) => (t / DYING_SECONDS).min(1.0),
            BossState::Dead => 1.0,
            _ => 0.0,
        }
    }

    pub fn dying_progress(&self) -> f64 {
        match self.state {
            BossState::Dying(t) => (t / DYING_SECONDS).min(1.0),
            BossState::Dead => 1.0,
            _ => 0.0,
        }
    }

    /// Countdown over: drop in.
    pub fn wake(&mut self) {
        if self.state == BossState::Hidden {
            self.state = BossState::Arrive(0.0);
        }
    }

    /// Back up into the sky over `home` (Ragnar lost a life); keeps its wounds.
    pub fn reset(&mut self, home: V) {
        if !self.alive() {
            return;
        }
        self.pos = home;
        self.vel = V::default();
        self.hover = SKY;
        self.state = BossState::Hidden;
        self.flash = 0.0;
        self.grace = 0.0;
        self.look = std::f64::consts::FRAC_PI_2;
    }

    /**
     * A blow for `amount`; dazed, it takes double. Returns true if it landed.
     * The last blow starts the death throes.
     */
    pub fn hit(&mut self, amount: i32, events: &mut Vec<BossEvent>) -> bool {
        if !self.vulnerable() {
            return false;
        }
        let before = self.phase();
        let amount = if self.dazed() { amount * 2 } else { amount };
        self.hp = (self.hp - amount).max(0);
        self.flash = 1.0;
        self.grace = HIT_GRACE;
        if self.hp == 0 {
            self.state = BossState::Dying(0.0);
            self.vel = V::default();
            events.push(BossEvent::Roar);
            return true;
        }
        let after = self.phase();
        if after > before {
            events.push(BossEvent::Enraged(after));
            events.push(BossEvent::Roar);
        }
        true
    }

    fn charge_seconds(&self) -> f64 {
        match self.phase() {
            1 => 0.75,
            2 => 0.62,
            _ => 0.5,
        }
    }

    fn drift_seconds(&mut self) -> f64 {
        let base = match self.phase() {
            1 => 1.9,
            2 => 1.4,
            _ => 1.0,
        };
        base + self.rng.range(0.0, 0.6)
    }

    /// The next attack in its repertoire; later phases add the stream and the dive.
    fn choose_attack(&mut self) -> Attack {
        self.cycle += 1;
        let phase = self.phase();
        if phase >= 2 && self.cycle % 3 == 0 {
            return Attack::Dive;
        }
        let pool: &[Attack] = match phase {
            1 => &[Attack::Fan, Attack::Ring, Attack::Boulders],
            2 => &[Attack::Fan, Attack::Ring, Attack::Stream, Attack::Boulders],
            _ => &[Attack::Ring, Attack::Stream, Attack::Stream, Attack::Boulders, Attack::Fan],
        };
        self.rng.pick(pool)
    }

    /// A spot 4 to 6 tiles from him, inside the arena.
    fn pick_spot(&mut self, hero: V) -> V {
        for _ in 0..12 {
            let a = self.rng.range(0.0, std::f64::consts::TAU);
            let d = self.rng.range(4.0, 6.0);
            let p = V::new(hero.x + a.cos() * d, hero.y + a.sin() * d);
            if p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y {
                return p;
            }
        }
        V::new((self.min.x + self.max.x) / 2.0, self.min.y + 2.0)
    }

    fn clamp(&self, p: V) -> V {
        V::new(p.x.clamp(self.min.x, self.max.x), p.y.clamp(self.min.y, self.max.y))
    }

    fn angle_to(&self, p: V) -> f64 {
        (p.y - self.pos.y).atan2(p.x - self.pos.x)
    }

    /// Glides toward `goal` with some inertia.
    fn steer(&mut self, goal: V, dt: f64, speed: f64) {
        let (dx, dy) = (goal.x - self.pos.x, goal.y - self.pos.y);
        let d = (dx * dx + dy * dy).sqrt();
        let want = if d > 0.05 { V::new(dx / d * speed.min(d * 2.5), dy / d * speed.min(d * 2.5)) } else { V::default() };
        let k = 1.0 - (-4.0 * dt).exp();
        self.vel = V::new(self.vel.x + (want.x - self.vel.x) * k, self.vel.y + (want.y - self.vel.y) * k);
        self.pos = self.clamp(V::new(self.pos.x + self.vel.x * dt, self.pos.y + self.vel.y * dt));
    }

    fn settle_hover(&mut self, goal: f64, dt: f64, rate: f64) {
        self.hover += (goal - self.hover) * (1.0 - (-rate * dt).exp());
    }

    fn turn_to(&mut self, angle: f64, dt: f64, rate: f64) {
        let mut d = (angle - self.look) % std::f64::consts::TAU;
        if d > std::f64::consts::PI {
            d -= std::f64::consts::TAU;
        }
        if d < -std::f64::consts::PI {
            d += std::f64::consts::TAU;
        }
        self.look += d * (1.0 - (-rate * dt).exp());
    }

    /// Where shots leave the eye: a little in front of the body.
    fn muzzle(&self) -> V {
        V::new(self.pos.x + self.look.cos() * 0.7, self.pos.y + self.look.sin() * 0.7)
    }

    pub fn update(&mut self, dt: f64, hero: V, shots: &mut Vec<BossShot>, events: &mut Vec<BossEvent>) {
        self.flash = (self.flash - dt * 4.0).max(0.0);
        self.grace = (self.grace - dt).max(0.0);
        let speed_k = match self.phase() {
            1 => 1.0,
            2 => 1.2,
            _ => 1.4,
        };
        match self.state {
            BossState::Hidden | BossState::Dead => {}
            BossState::Arrive(t) => {
                let t = t + dt;
                let k = (t / ARRIVE_SECONDS).min(1.0);
                // Falls fast, then brakes into its hover.
                self.hover = HOVER + (SKY - HOVER) * (1.0 - k).powi(3);
                self.turn_to(self.angle_to(hero), dt, 4.0);
                if k >= 1.0 {
                    events.push(BossEvent::Roar);
                    events.push(BossEvent::Slam(self.pos));
                    self.target = self.pick_spot(hero);
                    self.state = BossState::Drift(1.2);
                } else {
                    self.state = BossState::Arrive(t);
                }
            }
            BossState::Drift(left) => {
                self.steer(self.target, dt, MAX_SPEED * speed_k);
                self.settle_hover(HOVER, dt, 3.0);
                self.turn_to(self.angle_to(hero), dt, 5.0);
                let left = left - dt;
                let there = dist_sq(self.pos, self.target) < 0.3;
                if left <= 0.0 || (there && left < 0.6) {
                    let attack = self.choose_attack();
                    self.state = BossState::Charge(0.0, attack);
                    events.push(BossEvent::Charge);
                } else {
                    self.state = BossState::Drift(left);
                }
            }
            BossState::Charge(t, attack) => {
                let t = t + dt;
                self.steer(self.pos, dt, 0.0);
                self.turn_to(self.angle_to(hero), dt, 7.0);
                if attack == Attack::Dive {
                    // Rears up before the swoop.
                    self.settle_hover(HOVER + 1.2, dt, 4.0);
                }
                if t >= self.charge_seconds() {
                    if attack == Attack::Dive {
                        // Aim past him so a stationary target is overshot only a little.
                        let a = self.angle_to(hero);
                        let d = dist_sq(self.pos, hero).sqrt() + 0.8;
                        let goal = self.clamp(V::new(self.pos.x + a.cos() * d, self.pos.y + a.sin() * d));
                        self.state = BossState::Dive(goal);
                    } else {
                        self.state = BossState::Volley(0.0, attack, 0);
                    }
                } else {
                    self.state = BossState::Charge(t, attack);
                }
            }
            BossState::Volley(t, attack, fired) => {
                let t = t + dt;
                self.steer(self.pos, dt, 0.0);
                self.turn_to(self.angle_to(hero), dt, if attack == Attack::Stream { 9.0 } else { 3.0 });
                let phase = self.phase();
                let (fired, done) = self.volley(t, attack, fired, phase, hero, shots, events);
                if done {
                    self.target = self.pick_spot(hero);
                    self.state = BossState::Drift(self.drift_seconds());
                } else {
                    self.state = BossState::Volley(t, attack, fired);
                }
            }
            BossState::Dive(goal) => {
                let (dx, dy) = (goal.x - self.pos.x, goal.y - self.pos.y);
                let d = (dx * dx + dy * dy).sqrt();
                let step = DIVE_SPEED * speed_k * dt;
                self.settle_hover(0.05, dt, 9.0);
                if d <= step {
                    self.pos = goal;
                    self.vel = V::default();
                    self.hover = 0.05;
                    events.push(BossEvent::Slam(self.pos));
                    self.state = BossState::Dazed(0.0);
                } else {
                    self.pos = V::new(self.pos.x + dx / d * step, self.pos.y + dy / d * step);
                }
            }
            BossState::Dazed(t) => {
                let t = t + dt;
                self.hover = 0.05;
                self.look += dt * 1.5; // woozy
                if t >= DAZED_SECONDS {
                    self.state = BossState::Recover(0.0);
                    events.push(BossEvent::Roar);
                } else {
                    self.state = BossState::Dazed(t);
                }
            }
            BossState::Recover(t) => {
                let t = t + dt;
                self.settle_hover(HOVER, dt, 3.5);
                self.turn_to(self.angle_to(hero), dt, 4.0);
                if t >= 0.7 {
                    self.target = self.pick_spot(hero);
                    self.state = BossState::Drift(self.drift_seconds());
                } else {
                    self.state = BossState::Recover(t);
                }
            }
            BossState::Dying(t) => {
                let t = t + dt;
                self.settle_hover(HOVER + 0.4, dt, 2.0);
                self.look += dt * (2.0 + t * 4.0);
                self.spasm_timer -= dt;
                if self.spasm_timer <= 0.0 {
                    self.spasm_timer = 0.28 - t * 0.07;
                    let a = self.rng.range(0.0, std::f64::consts::TAU);
                    let r = self.rng.range(0.2, 0.8);
                    events.push(BossEvent::Spasm(V::new(self.pos.x + a.cos() * r, self.pos.y + a.sin() * r)));
                }
                if t >= DYING_SECONDS {
                    self.state = BossState::Dead;
                    events.push(BossEvent::Died(self.pos));
                } else {
                    self.state = BossState::Dying(t);
                }
            }
        }
    }

    /// Fires whatever `attack` has due by `t`. Returns shots fired so far and whether it is over.
    #[allow(clippy::too_many_arguments)]
    fn volley(&mut self, t: f64, attack: Attack, fired: u32, phase: u8, hero: V, shots: &mut Vec<BossShot>, events: &mut Vec<BossEvent>) -> (u32, bool) {
        let at = self.muzzle();
        let aim = self.angle_to(hero);
        match attack {
            Attack::Fan => {
                if fired == 0 {
                    let n = if phase == 1 { 3 } else { 5 };
                    let spread = 0.26;
                    for i in 0..n {
                        let off = (i as f64 - (n - 1) as f64 / 2.0) * spread;
                        shots.push(BossShot::new(at, aim + off, 6.2, ShotKind::Bolt));
                    }
                    events.push(BossEvent::Fire(ShotKind::Bolt));
                }
                (1, t >= 0.55)
            }
            Attack::Ring => {
                // Phase 3 throws a second, offset ring a beat later.
                let rings = if phase >= 3 { 2 } else { 1 };
                let due = if t >= 0.35 && rings == 2 { 2 } else { 1 };
                let mut fired = fired;
                while fired < due {
                    let n = if phase == 1 { 10 } else { 12 };
                    let offset = aim + fired as f64 * std::f64::consts::PI / n as f64;
                    for i in 0..n {
                        let a = offset + i as f64 / n as f64 * std::f64::consts::TAU;
                        shots.push(BossShot::new(self.pos, a, 4.3, ShotKind::Bolt));
                    }
                    events.push(BossEvent::Fire(ShotKind::Bolt));
                    fired += 1;
                }
                (fired, t >= 0.35 * rings as f64 + 0.4)
            }
            Attack::Stream => {
                let n = if phase >= 3 { 8 } else { 6 };
                let gap = 0.15;
                let due = ((t / gap).floor() as u32 + 1).min(n);
                let mut fired = fired;
                while fired < due {
                    let wobble = self.rng.range(-0.06, 0.06);
                    shots.push(BossShot::new(at, aim + wobble, 7.4, ShotKind::Bolt));
                    events.push(BossEvent::Fire(ShotKind::Bolt));
                    fired += 1;
                }
                (fired, fired >= n && t >= n as f64 * gap + 0.3)
            }
            Attack::Boulders => {
                let n = if phase == 1 { 3 } else { 4 };
                let gap = 0.3;
                let due = ((t / gap).floor() as u32 + 1).min(n);
                let mut fired = fired;
                while fired < due {
                    // One straight at him, the rest to either side.
                    let off = match fired {
                        0 => 0.0,
                        k if k % 2 == 1 => 0.35 * k.div_ceil(2) as f64,
                        k => -0.35 * (k / 2) as f64,
                    };
                    shots.push(BossShot::new(at, aim + off, 4.0, ShotKind::Boulder));
                    events.push(BossEvent::Fire(ShotKind::Boulder));
                    fired += 1;
                }
                (fired, fired >= n && t >= n as f64 * gap + 0.4)
            }
            Attack::Dive => (fired, true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arena() -> Cyclops {
        Cyclops::new(V::new(13.0, 6.0), V::new(4.0, 4.0), V::new(22.0, 18.0), CYCLOPS_HP, 7)
    }

    fn run(c: &mut Cyclops, seconds: f64, hero: V, shots: &mut Vec<BossShot>, events: &mut Vec<BossEvent>) {
        let dt = 1.0 / 120.0;
        for _ in 0..(seconds / dt) as usize {
            c.update(dt, hero, shots, events);
        }
    }

    #[test]
    fn it_drops_in_then_throws_things_at_him() {
        let mut c = arena();
        let (mut shots, mut events) = (Vec::new(), Vec::new());
        let hero = V::new(13.0, 17.0);
        run(&mut c, 1.0, hero, &mut shots, &mut events);
        assert_eq!(c.state, BossState::Hidden, "waits for the countdown");
        assert!(!c.vulnerable());
        c.wake();
        run(&mut c, 2.0, hero, &mut shots, &mut events);
        assert!(c.active());
        assert!(events.contains(&BossEvent::Roar));
        assert!((c.hover - HOVER).abs() < 0.1);
        run(&mut c, 8.0, hero, &mut shots, &mut events);
        assert!(events.iter().any(|e| matches!(e, BossEvent::Fire(_))), "never attacked");
        assert!(!shots.is_empty());
        // Everything it throws heads roughly his way or outward from it.
        assert!(c.pos.x >= 4.0 && c.pos.x <= 22.0 && c.pos.y >= 4.0 && c.pos.y <= 18.0, "left the arena: {:?}", c.pos);
    }

    #[test]
    fn wounds_enrage_it_and_the_last_blow_kills() {
        let mut c = arena();
        let (mut shots, mut events) = (Vec::new(), Vec::new());
        c.wake();
        run(&mut c, 2.0, V::new(13.0, 17.0), &mut shots, &mut events);
        events.clear();
        let mut blows = 0;
        while c.alive() {
            run(&mut c, 0.2, V::new(13.0, 17.0), &mut shots, &mut events);
            if c.hit(2, &mut events) {
                blows += 1;
            }
            assert!(blows < 100);
        }
        assert!(events.contains(&BossEvent::Enraged(2)));
        assert!(events.contains(&BossEvent::Enraged(3)));
        run(&mut c, 3.0, V::new(13.0, 17.0), &mut shots, &mut events);
        assert_eq!(c.state, BossState::Dead);
        assert!(events.iter().any(|e| matches!(e, BossEvent::Died(_))));
    }

    #[test]
    fn a_missed_dive_leaves_it_dazed_and_open() {
        let mut c = arena();
        c.hp = c.max_hp / 2; // phase 2 dives
        let (mut shots, mut events) = (Vec::new(), Vec::new());
        c.wake();
        let hero = V::new(10.0, 14.0);
        let mut dazed = false;
        for _ in 0..(30.0 * 120.0) as usize {
            c.update(1.0 / 120.0, hero, &mut shots, &mut events);
            if c.dazed() {
                dazed = true;
                break;
            }
        }
        assert!(dazed, "never dove");
        assert!(events.iter().any(|e| matches!(e, BossEvent::Slam(_))));
        let hp = c.hp;
        assert!(c.hit(2, &mut events));
        assert_eq!(c.hp, hp - 4, "dazed takes double");
        assert!(!c.harmful(), "a dazed eye does not hurt to touch");
    }
}
