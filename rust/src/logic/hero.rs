use super::gear::ShieldGear;
use super::grid::Grid;
use super::heroes::HeroKind;
use super::items::{item, weapon, ItemKind, RangedKind, WeaponKind, SPEED_BOOST};
use super::mover::Mover;
use super::progression::{self, path, STAMINA_MAX, STAMINA_PER_BLOW, STAMINA_REGEN, STAMINA_REST};
use super::types::{Dir, P, V};
use crate::logic::gear;
use crate::logic::types::{PotionKind, Tile, CARDINALS};

/// Seconds a jump takes, and how high (in levels) the arc peaks above the straight line.
pub const JUMP_SECONDS: f64 = 0.42;
pub const JUMP_PEAK: f64 = 1.1;

pub const HERO_MAX_HP: i32 = 100;
pub const CONTACT_DAMAGE: i32 = 25;
const INVULN_SECONDS: f64 = 1.0;
const BLOCK_INVULN_SECONDS: f64 = 0.5;
/// Elixirs: what they heal, how they change his speed, his blows and the hits he takes.
pub const HEALTH_POTION: i32 = 35;
const HASTE_SPEED: f64 = 2.0;
const HASTE_COOLDOWN: f64 = 0.5;
const STRENGTH_BONUS: i32 = 2;
const BARK_HP: i32 = 50;
const FURY_TAKEN: f64 = 0.5;
const FURY_MAX: i32 = 5;
const RAGNAROK_SPEED: f64 = 1.3;
const RAGNAROK_COOLDOWN: f64 = 0.67;
const RAGNAROK_DRAIN: f64 = 2.0;
const SPIRIT_REGEN: f64 = 3.0;
/// A health potion's moment of immunity.
pub const HEALTH_IMMUNITY: f64 = 1.5;
/// The blink of Bifrost Step leaves him untouchable this long.
const STEP_IMMUNITY: f64 = 0.3;
const SWING_SECONDS: f64 = 0.18;
/// Speed multiplier while slowed by slime.
pub const SLIME_SLOW: f64 = 0.5;
/// How long a step in slime keeps slowing him after he leaves it.
pub const SLIME_SLOW_SECONDS: f64 = 1.2;

// Weight and momentum. Top speed is untouched (`Mover::speed`); these only
// shape how he gets there and how he lets go of it. Position, `occupied`
// and every hit test still come from the same grid mover as before.

/// Seconds from standing still to top speed while a direction is held.
pub const ACCEL_SECONDS: f64 = 0.2;
/// Seconds to brake from top speed to a standstill before turning around.
pub const BRAKE_SECONDS: f64 = 0.12;
/// Strongest friction when coasting to a stop, as seconds to shed top speed.
pub const STOP_SECONDS: f64 = 0.1;
/// Coasting never drops below this share of top speed, so he always reaches the tile center.
pub const MIN_COAST: f64 = 0.15;
/// Share of speed kept through a corner.
pub const CORNER_KEEP: f64 = 0.75;
/// Share of running speed kept through a landing.
pub const LANDING_KEEP: f64 = 0.6;
/// Standing still, a new direction first turns him on the spot; he only steps
/// off once it has been held this long. A tap turns, a hold walks.
pub const TURN_TAP_SECONDS: f64 = 0.12;

/// Seconds left on each timed elixir (invulnerability uses the boon timer).
#[derive(Clone, Copy, Debug, Default)]
pub struct Effects {
    pub strength: f64,
    pub haste: f64,
    pub fury: f64,
    pub fire: f64,
    pub ragnarok: f64,
    pub spirit: f64,
}

impl Effects {
    fn tick(&mut self, dt: f64) {
        for t in [&mut self.strength, &mut self.haste, &mut self.fury, &mut self.fire, &mut self.ragnarok, &mut self.spirit] {
            *t = (*t - dt).max(0.0);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DamageResult {
    Damaged,
    Blocked,
    Immune,
}

#[derive(Clone, Copy, Debug)]
struct JumpState {
    /// Where the leap starts and lands (tile centers in grid movement, anywhere in free movement),
    /// and the tile it lands on.
    from_pos: V,
    to_pos: V,
    to: P,
    from_h: f64,
    to_h: f64,
    t: f64,
}

/// Behind a raised shield he walks at this share of his pace.
pub const BLOCK_SPEED: f64 = 0.55;

/**
 * The hero. Makes footstep noise while moving; enemies with hearing home in
 * on `heard_pos` rather than the true position. Fights with the blade of his
 * armory (swung or thrown), a ranged weapon from a shrine, and bombs; every
 * blow costs stamina. His shield and his path's skills guard him.
 */
#[derive(Clone, Debug)]
pub struct Hero {
    pub m: Mover,
    pub heard_pos: V,
    /// Seconds since the hero last made noise.
    pub heard_age: f64,
    /// 0..1 visual stride throttle.
    pub motion: f64,
    pub alive: bool,
    pub facing: Dir,

    pub max_hp: i32,
    pub hp: i32,
    /// Seconds of damage immunity remaining after a hit.
    pub invuln: f64,
    pub attack_cooldown: f64,
    /// Seconds of swing animation remaining.
    pub swing: f64,

    /// What is in his hand: his main weapon, or a shrine weapon until its ammo is spent.
    pub weapon: WeaponKind,
    /// His own: the armory blade swung (Sword), or thrown (a Hurled weapon that never runs out).
    pub main: WeaponKind,
    /// Who he is: whose armory and path he has.
    pub kind: HeroKind,
    /// The armory tier in his hand (1..=10).
    pub blade: u8,
    /// How far along his heroic path he is (0..=10): the skills he has.
    pub path: u8,
    pub ammo: u32,
    pub fire_cooldown: f64,
    /// Stamina for blows and blinks; it comes back once he has rested a moment.
    pub stamina: f64,
    stamina_rest: f64,
    /// Set when a blow is refused for want of stamina; the game clears it.
    pub winded: bool,
    /// Forge-Heart: seconds of free swings left, and until it can come again.
    pub heart: f64,
    pub heart_cooldown: f64,
    /// Bifrost Step: seconds until he can blink again.
    pub step_cooldown: f64,
    /// Health soaked up by a shield potion before his own is touched.
    pub bark: i32,
    /// Berserker's Draught: blows taken while it runs, each adding to his own.
    pub fury_stacks: i32,
    /// Set on a hit his shield took some of (for its gifts); the game clears it.
    pub blocked_hit: bool,
    /// A last stand (Mehmed's and Miyamoto's path 8): seconds in which he cannot die and strikes triple.
    pub last_stand: f64,
    drain_carry: f64,

    /// The shield on his arm, wearing down as it blocks.
    pub shield: Option<ShieldGear>,
    /// Set on the hit that breaks the shield; the game clears it.
    pub shield_broke: bool,
    /// The shield is raised (LB, or Shift, held): it takes the whole of every hit, and he walks slower.
    pub blocking: bool,
    /// The share of his pace his fighting leaves him (the Sparring Ring's state machine: planted in
    /// a swing, rooted while reeling); 1 elsewhere.
    pub root: f64,
    /// The share of his pace a realm leaves him (mud, a full belly, a drowsy spell); 1 elsewhere.
    /// The realm sets it every tick.
    pub pace: f64,
    /// Maximum health a realm's hunger has taken from him (the Rotting Feast); 0 elsewhere.
    pub hunger: i32,
    /// How sharp the blade in his hand is, 0 to 1: a dull one bites the boss less (`gear::edge_power`).
    pub edge: f64,
    /// The outfit he wears, and how many skills of his path he has learned (each a ribbon on his arm).
    pub outfit: u8,
    /// Timed potion effects, seconds left each.
    pub fx: Effects,
    regen_carry: f64,
    pub invincible_timer: f64,
    /// Enemies cannot sense or touch him while this runs.
    pub invisible_timer: f64,
    pub speed_timer: f64,
    /// Slime underfoot.
    pub slow_timer: f64,
    pub base_speed: f64,
    jump: Option<JumpState>,
    /// Floor elevation at his feet, or the arc height while airborne, in levels.
    pub altitude: f64,
    /// Ground speed along `m.dir` in tiles per second; eases toward `m.speed`.
    pub vel: f64,
    /// Seconds left before a fresh turn on the spot may become a step.
    pub turn_timer: f64,

    /// Free movement: he walks anywhere, not just tile to tile (the grid mover then only
    /// tracks the tile he stands on, for everything else in the game).
    pub free_move: bool,
    /// Which way the player wants him to go, in grid units (any length; zero to stop).
    pub wish: V,
    free_pos: V,
    free_vel: V,
    synced: P,
    /// Which way he faces, as an angle in the grid's x/y plane: any way in free movement,
    /// one of the four in grid movement.
    pub heading: f64,
}

/// How far apart the points are that a free leap or dash checks along its way, in tiles.
const FREE_PROBE: f64 = 0.05;

/// His body's radius against walls in free movement, in tiles.
pub const FREE_RADIUS: f64 = 0.3;

impl Hero {
    pub fn new(start: P, speed: f64) -> Self {
        Self {
            m: Mover::new(start, speed),
            heard_pos: start.v(),
            heard_age: 0.0,
            motion: 0.0,
            alive: true,
            facing: Dir::Down,
            max_hp: HERO_MAX_HP,
            hp: HERO_MAX_HP,
            invuln: 0.0,
            attack_cooldown: 0.0,
            swing: 0.0,
            weapon: WeaponKind::Sword,
            main: WeaponKind::Sword,
            kind: HeroKind::Ragnar,
            blade: 1,
            path: 0,
            ammo: 0,
            fire_cooldown: 0.0,
            stamina: STAMINA_MAX,
            stamina_rest: 0.0,
            winded: false,
            heart: 0.0,
            heart_cooldown: 0.0,
            step_cooldown: 0.0,
            bark: 0,
            fury_stacks: 0,
            blocked_hit: false,
            last_stand: 0.0,
            drain_carry: 0.0,
            shield: Some(ShieldGear::new(1)),
            shield_broke: false,
            blocking: false,
            root: 1.0,
            pace: 1.0,
            hunger: 0,
            edge: 1.0,
            outfit: 0,
            fx: Effects::default(),
            regen_carry: 0.0,
            invincible_timer: 0.0,
            invisible_timer: 0.0,
            speed_timer: 0.0,
            slow_timer: 0.0,
            base_speed: speed,
            jump: None,
            altitude: 0.0,
            vel: 0.0,
            turn_timer: 0.0,
            free_move: false,
            wish: V::new(0.0, 0.0),
            free_pos: start.v(),
            free_vel: V::new(0.0, 0.0),
            synced: start,
            heading: std::f64::consts::FRAC_PI_2,
        }
    }

    pub fn invulnerable(&self) -> bool {
        self.invuln > 0.0
    }
    pub fn invincible(&self) -> bool {
        self.invincible_timer > 0.0
    }
    pub fn invisible(&self) -> bool {
        self.invisible_timer > 0.0
    }
    pub fn hasted(&self) -> bool {
        self.speed_timer > 0.0
    }
    pub fn slowed(&self) -> bool {
        self.slow_timer > 0.0
    }
    pub fn airborne(&self) -> bool {
        self.jump.is_some()
    }
    pub fn swinging(&self) -> bool {
        self.swing > 0.0
    }
    pub fn has_ranged_weapon(&self) -> bool {
        self.weapon != WeaponKind::Sword
    }
    /// Holding a shrine weapon with limited ammo rather than his own.
    pub fn holding_pickup(&self) -> bool {
        self.weapon != self.main
    }
    /// A blow of `base` damage as his elixirs make it: strength and fury add to it, Ragnarok doubles it.
    pub fn blow(&self, base: i32) -> i32 {
        // Mehmed's blows land harder (at least one more).
        let mut d = (base as f64 * self.kind.might()).ceil() as i32;
        if self.fx.strength > 0.0 {
            d += STRENGTH_BONUS;
        }
        if self.fx.fury > 0.0 {
            d += self.fury_stacks;
        }
        if self.fx.ragnarok > 0.0 {
            d *= 2;
        }
        if self.last_stand > 0.0 {
            d *= path::STAND_MULT;
        }
        d
    }
    fn cooldown_scale(&self) -> f64 {
        // Miyamoto strikes again sooner than the others.
        let mut s = self.kind.tempo();
        if self.path >= 1 {
            s *= path::RAGE_COOLDOWN;
        }
        if self.fx.haste > 0.0 {
            s *= HASTE_COOLDOWN;
        }
        if self.fx.ragnarok > 0.0 {
            s *= RAGNAROK_COOLDOWN;
        }
        s
    }
    /// Seer's Sight (Ragnar's and Mehmed's path 3): secrets through walls, sight in the dark, critical hits.
    pub fn sees(&self) -> bool {
        self.path >= 3 && self.kind != HeroKind::Miyamoto
    }
    /// His blows cost nothing: Forge-Heart, or the Ragnarok tonic.
    pub fn tireless(&self) -> bool {
        self.heart > 0.0 || self.fx.ragnarok > 0.0
    }
    /// His health cap: more from path level 2 (for Ragnar, only while a shield is with him).
    pub fn health_cap(&self) -> i32 {
        let guard = self.path >= 2 && (self.kind != HeroKind::Ragnar || self.shield.is_some());
        // Mehmed can take more than Ragnar, Miyamoto less.
        (HERO_MAX_HP as f64 * self.kind.vigor()).round() as i32 + if guard { path::GUARD_HP } else { 0 } - self.hunger
    }
    /// Spends stamina for a blow or a blink; false (and winded) if he has too little.
    fn spend(&mut self, cost: f64) -> bool {
        if self.tireless() {
            return true;
        }
        if self.stamina + 1e-9 < cost {
            self.winded = true;
            return false;
        }
        self.stamina -= cost;
        self.stamina_rest = STAMINA_REST;
        if self.kind == HeroKind::Ragnar && self.stamina < STAMINA_PER_BLOW && self.path >= 7 && self.heart_cooldown <= 0.0 {
            // Forge-Heart: at the end of his breath the mountain's own stamina takes over.
            self.heart = path::HEART_SECONDS;
            self.heart_cooldown = path::HEART_COOLDOWN;
        }
        true
    }
    pub fn occupied(&self) -> P {
        self.m.occupied()
    }

    /// The way his blows, shots, leaps and dashes go: a unit vector along his
    /// heading in free movement, straight along one of the four on the grid.
    pub fn aim(&self) -> V {
        if self.free_move {
            V::new(self.heading.cos(), self.heading.sin())
        } else {
            self.facing.or_down().vecf()
        }
    }

    /// Turns him to face `d` (at a spawn, a cheat, a test).
    pub fn face(&mut self, d: Dir) {
        self.facing = d;
        self.heading = angle_of(d);
    }

    /// A dash in free movement: along his heading, up to `tiles`, stopping short of whatever
    /// he could not walk into, and ending wherever that is rather than on a tile center.
    fn dash_free(&mut self, grid: &Grid, tiles: f64, cost: f64) -> Option<(V, V)> {
        let before = self.pos();
        let aim = self.aim();
        let mut at = before;
        let mut tile = self.occupied();
        let mut gone = 0.0;
        while gone < tiles {
            let next = V::new(at.x + aim.x * FREE_PROBE, at.y + aim.y * FREE_PROBE);
            let t = grid.wrap(P::new(next.x.round() as i32, next.y.round() as i32));
            if t != tile && free_blocked(grid, tile, t.x, t.y) {
                break;
            }
            at = next;
            tile = t;
            gone += FREE_PROBE;
        }
        if gone < 0.3 || !self.spend(cost) {
            return None;
        }
        self.m.teleport(tile);
        self.free_pos = at;
        self.free_vel = V::new(0.0, 0.0);
        self.synced = tile;
        self.push_out(grid);
        self.altitude = free_floor(grid, self.free_pos);
        self.vel = 0.0;
        if !self.invisible() {
            self.heard_pos = self.free_pos;
            self.heard_age = 0.0;
        }
        Some((before, self.free_pos))
    }
    pub fn dir(&self) -> Dir {
        self.m.dir
    }

    /// Carried `step` tiles in free movement by something other than his own feet (a sumo charge,
    /// a call drawing him in, a giant's hand), sliding along whatever stops him. Nothing moves him
    /// in the air, or on the grid.
    pub fn shove(&mut self, grid: &Grid, step: V) {
        if !self.alive || !self.free_move || self.jump.is_some() || (step.x == 0.0 && step.y == 0.0) {
            return;
        }
        if self.m.tile != self.synced {
            self.free_pos = self.m.tile.v();
        }
        self.free_pos.x += step.x;
        self.push_out(grid);
        self.free_pos.y += step.y;
        self.push_out(grid);
        let tile = grid.wrap(P::new(self.free_pos.x.round() as i32, self.free_pos.y.round() as i32));
        self.m.tile = tile;
        self.m.offset = 0.0;
        self.synced = tile;
    }

    /// 0..1 through the current jump, 0 on the ground.
    pub fn jump_progress(&self) -> f64 {
        self.jump.map(|j| j.t / JUMP_SECONDS).unwrap_or(0.0)
    }

    /// 0..1 progress through the current swing, 0 when idle.
    pub fn swing_progress(&self) -> f64 {
        if self.swing > 0.0 {
            1.0 - self.swing / SWING_SECONDS
        } else {
            0.0
        }
    }

    /// While airborne the position follows the jump line instead of the grid mover.
    pub fn pos(&self) -> V {
        match self.jump {
            None if self.free_move && self.m.tile == self.synced => self.free_pos,
            None => self.m.pos(),
            Some(j) => {
                let t = j.t / JUMP_SECONDS;
                V::new(j.from_pos.x + (j.to_pos.x - j.from_pos.x) * t, j.from_pos.y + (j.to_pos.y - j.from_pos.y) * t)
            }
        }
    }

    /**
     * Leap the way he faces: two tiles when the landing is clear (over a pit,
     * an enemy, or up one level), one tile otherwise, or a hop in place. On
     * the grid it goes tile to tile; in free movement it follows his heading
     * and lands wherever it comes down.
     */
    pub fn try_jump(&mut self, grid: &Grid) -> bool {
        if !self.alive || self.jump.is_some() {
            return false;
        }
        let from_pos = self.pos();
        let from = self.m.occupied();
        let (to, to_pos) = if self.free_move {
            free_landing(grid, from_pos, self.aim()).unwrap_or((from, from_pos))
        } else {
            let to = grid.jump_landing(from, self.facing).unwrap_or(from);
            (to, to.v())
        };
        if self.kind == HeroKind::Miyamoto && self.path >= 3 {
            // Wind Step: every leap is a dodge.
            self.invuln = self.invuln.max(path::DODGE_SECONDS);
        }
        self.m.tile = from;
        self.m.offset = 0.0;
        self.m.dir = Dir::None;
        self.jump = Some(JumpState {
            from_pos,
            to_pos,
            to,
            from_h: grid.elevation(from.x, from.y) as f64,
            to_h: grid.elevation(to.x, to.y) as f64,
            t: 0.0,
        });
        true
    }

    pub fn update(&mut self, dt: f64, grid: &Grid, wanted: Dir) {
        self.invuln = (self.invuln - dt).max(0.0);
        self.attack_cooldown = (self.attack_cooldown - dt).max(0.0);
        self.fire_cooldown = (self.fire_cooldown - dt).max(0.0);
        self.swing = (self.swing - dt).max(0.0);
        self.invincible_timer = (self.invincible_timer - dt).max(0.0);
        self.invisible_timer = (self.invisible_timer - dt).max(0.0);
        self.speed_timer = (self.speed_timer - dt).max(0.0);
        self.slow_timer = (self.slow_timer - dt).max(0.0);
        self.step_cooldown = (self.step_cooldown - dt).max(0.0);
        self.last_stand = (self.last_stand - dt).max(0.0);
        self.heart = (self.heart - dt).max(0.0);
        self.heart_cooldown = (self.heart_cooldown - dt).max(0.0);
        self.fx.tick(dt);
        if self.fx.fury <= 0.0 {
            self.fury_stacks = 0;
        }
        self.max_hp = self.health_cap();
        self.hp = self.hp.min(self.max_hp);
        if self.alive {
            // Stamina comes back once he stops spending it; a skald's song brings it faster.
            self.stamina_rest = (self.stamina_rest - dt).max(0.0);
            if self.stamina_rest <= 0.0 {
                let rate = STAMINA_REGEN * if self.path >= 4 { path::SONG_REGEN } else { 1.0 };
                self.stamina = (self.stamina + rate * dt).min(STAMINA_MAX);
            }
            if self.fx.spirit > 0.0 {
                self.regen_carry += SPIRIT_REGEN * dt;
                let whole = self.regen_carry.floor();
                self.regen_carry -= whole;
                self.heal(whole as i32);
            }
            if self.fx.ragnarok > 0.0 {
                // The end-times tonic eats at him, but never takes his last breath.
                self.drain_carry += RAGNAROK_DRAIN * dt;
                let whole = self.drain_carry.floor() as i32;
                self.drain_carry -= whole as f64;
                self.hp = (self.hp - whole).max(1);
            }
            if let Some(s) = &mut self.shield {
                let mend = progression::ward(s.tier).mend;
                if mend > 0.0 && s.durability < s.max() {
                    s.mend_carry += mend * dt;
                    let whole = s.mend_carry.floor();
                    s.mend_carry -= whole;
                    s.durability = (s.durability + whole as i32).min(s.max());
                }
            }
        }
        if self.heart > 0.0 {
            // Forge-Heart shrugs off the slime.
            self.slow_timer = 0.0;
        }
        self.m.speed = self.base_speed
            * if self.hasted() { SPEED_BOOST } else { 1.0 }
            * if self.fx.haste > 0.0 { HASTE_SPEED } else { 1.0 }
            * if self.fx.ragnarok > 0.0 { RAGNAROK_SPEED } else { 1.0 }
            * if self.slowed() { SLIME_SLOW } else { 1.0 }
            * if self.blocking { BLOCK_SPEED } else { 1.0 }
            * self.root
            * self.pace;
        if !self.alive {
            return;
        }

        if let Some(mut j) = self.jump {
            j.t = (j.t + dt).min(JUMP_SECONDS);
            let t = j.t / JUMP_SECONDS;
            self.altitude = j.from_h + (j.to_h - j.from_h) * t + JUMP_PEAK * (std::f64::consts::PI * t).sin();
            self.motion = (self.motion + dt * 6.0).min(1.0);
            self.jump = Some(j);
            if j.t >= JUMP_SECONDS {
                // Land: settle on the target tile; the thud is a footstep the skeleton can hear.
                self.m.tile = j.to;
                self.m.offset = 0.0;
                self.jump = None;
                self.free_pos = j.to_pos;
                self.synced = j.to;
                self.altitude = if self.free_move { free_floor(grid, j.to_pos) } else { grid.elevation(j.to.x, j.to.y) as f64 };
                if self.free_move {
                    // Come down against a wall's edge: step clear of it.
                    self.push_out(grid);
                }
                self.vel *= LANDING_KEEP;
                if !self.invisible() {
                    self.heard_pos = j.to_pos;
                    self.heard_age = 0.0;
                }
            }
            return;
        }

        let before = self.pos();
        if self.free_move {
            self.drive_free(dt, grid);
        } else {
            // Hold-to-move: he runs only while a direction is held, and coasts to rest on the next tile center.
            self.drive(dt, grid, wanted);
            if self.m.dir != Dir::None {
                self.facing = self.m.dir;
            }
            self.heading = angle_of(self.facing);
            self.altitude = self.m.elevation_on(grid);
            self.free_pos = self.m.pos();
            self.synced = self.m.tile;
        }
        let after = self.pos();

        let moved = before.x != after.x || before.y != after.y;
        if moved {
            // An invisible hero makes no footsteps for the skeleton to follow.
            if !self.invisible() {
                self.heard_pos = after;
                self.heard_age = 0.0;
            } else {
                self.heard_age += dt;
            }
            // The stride follows how fast he is really going.
            let target = if self.m.speed > 0.0 { (self.vel / self.m.speed).clamp(0.0, 1.0) } else { 0.0 };
            let step = dt * 6.0;
            self.motion += (target - self.motion).clamp(-step, step);
        } else {
            self.heard_age += dt;
            self.motion = (self.motion - dt * 4.0).max(0.0);
        }
    }

    /**
     * Free movement: he runs any way the stick or keys point, with the same
     * acceleration, top speed and friction as on the grid, sliding along
     * walls, pits and ledges too high to climb. Ramps carry him up; a drop he
     * steps straight off. The grid mover keeps the tile he stands on.
     */
    fn drive_free(&mut self, dt: f64, grid: &Grid) {
        let top = self.m.speed;
        if self.m.tile != self.synced {
            // Put somewhere by the game (a respawn, a cheat): start from there.
            self.free_pos = self.m.tile.v();
            self.free_vel = V::new(0.0, 0.0);
            self.altitude = grid.elevation(self.m.tile.x, self.m.tile.y) as f64;
        }
        let len = (self.wish.x * self.wish.x + self.wish.y * self.wish.y).sqrt();
        let (tx, ty) = if len > 1e-3 { (self.wish.x / len * top, self.wish.y / len * top) } else { (0.0, 0.0) };
        // (Rooted by a swing he still stops at once: the rates never fall below half his own pace's.)
        let brisk = top.max(self.base_speed * 0.5);
        let rate = if len > 1e-3 { brisk / ACCEL_SECONDS } else { brisk / STOP_SECONDS };
        let (dx, dy) = (tx - self.free_vel.x, ty - self.free_vel.y);
        let d = (dx * dx + dy * dy).sqrt();
        let step = rate * dt * 1.6;
        self.free_vel = if d <= step { V::new(tx, ty) } else { V::new(self.free_vel.x + dx / d * step, self.free_vel.y + dy / d * step) };
        if len > 1e-3 {
            self.heading = self.wish.y.atan2(self.wish.x);
            let (c, s) = (self.heading.cos(), self.heading.sin());
            self.facing = if c.abs() >= s.abs() {
                if c > 0.0 {
                    Dir::Right
                } else {
                    Dir::Left
                }
            } else if s > 0.0 {
                Dir::Down
            } else {
                Dir::Up
            };
        }
        // Move one axis at a time so he slides along whatever stops him.
        self.free_pos.x += self.free_vel.x * dt;
        self.push_out(grid);
        self.free_pos.y += self.free_vel.y * dt;
        self.push_out(grid);
        let w = grid.width as f64;
        if self.free_pos.x < -0.5 {
            self.free_pos.x += w;
        } else if self.free_pos.x >= w - 0.5 {
            self.free_pos.x -= w;
        }
        let tile = grid.wrap(P::new(self.free_pos.x.round() as i32, self.free_pos.y.round() as i32));
        let speed = (self.free_vel.x * self.free_vel.x + self.free_vel.y * self.free_vel.y).sqrt();
        self.m.tile = tile;
        self.m.offset = 0.0;
        self.m.dir = if speed > 0.05 { self.facing } else { Dir::None };
        self.m.world_width = grid.width;
        self.synced = tile;
        self.vel = speed;
        // Up ramps smoothly; down a drop quickly but not in a single frame.
        let floor = free_floor(grid, self.free_pos);
        if floor >= self.altitude {
            self.altitude = floor;
        } else {
            self.altitude += (floor - self.altitude) * (1.0 - (-18.0 * dt).exp());
        }
    }

    /// Pushes him out of every tile around him that he may not stand on from where he is.
    fn push_out(&mut self, grid: &Grid) {
        let here = P::new(self.free_pos.x.round() as i32, self.free_pos.y.round() as i32);
        let here = grid.wrap(here);
        for oy in -1..=1 {
            for ox in -1..=1 {
                if ox == 0 && oy == 0 {
                    continue;
                }
                let (nx, ny) = (here.x + ox, here.y + oy);
                if !free_blocked(grid, here, nx, ny) {
                    continue;
                }
                // Nearest point of that tile's square to his center.
                let (cx, cy) =
                    (self.free_pos.x.clamp(nx as f64 - 0.5, nx as f64 + 0.5), self.free_pos.y.clamp(ny as f64 - 0.5, ny as f64 + 0.5));
                let (ex, ey) = (self.free_pos.x - cx, self.free_pos.y - cy);
                let dist = (ex * ex + ey * ey).sqrt();
                if dist >= FREE_RADIUS {
                    continue;
                }
                if dist > 1e-6 {
                    self.free_pos.x = cx + ex / dist * FREE_RADIUS;
                    self.free_pos.y = cy + ey / dist * FREE_RADIUS;
                    // Lose the speed that went into the wall.
                    let (nxv, nyv) = (ex / dist, ey / dist);
                    let into = self.free_vel.x * nxv + self.free_vel.y * nyv;
                    if into < 0.0 {
                        self.free_vel.x -= into * nxv;
                        self.free_vel.y -= into * nyv;
                    }
                } else if ox != 0 {
                    self.free_pos.x = nx as f64 - ox as f64 * (0.5 + FREE_RADIUS);
                } else {
                    self.free_pos.y = ny as f64 - oy as f64 * (0.5 + FREE_RADIUS);
                }
            }
        }
    }

    /**
     * Grid movement with mass. Holding a direction accelerates him to top
     * speed; letting go coasts him to the next tile center under friction
     * that eases him to rest; asking for the way he came brakes first and
     * only then turns him around; corners bleed a little speed.
     */
    fn drive(&mut self, dt: f64, grid: &Grid, wanted: Dir) {
        let top = self.m.speed;
        let m = &mut self.m;
        m.world_width = grid.width;

        if m.dir == Dir::None {
            if wanted == Dir::None {
                self.vel = 0.0;
                self.turn_timer = 0.0;
                return;
            }
            // Turn on the spot first: facing a wall, a pit or an enemy is a move of its own.
            if wanted != self.facing {
                self.facing = wanted;
                self.turn_timer = TURN_TAP_SECONDS;
                self.vel = 0.0;
                return;
            }
            if self.turn_timer > 0.0 {
                self.turn_timer -= dt;
                self.vel = 0.0;
                return;
            }
            if grid.can_move(m.tile, wanted, false, false) {
                m.dir = wanted; // from rest, or with whatever a landing carried
            } else {
                self.vel = 0.0;
                return;
            }
        }

        if wanted == m.dir.opposite() {
            self.vel -= top / BRAKE_SECONDS * dt;
            if self.vel <= 0.0 {
                m.reverse(grid);
                self.vel = 0.0;
                return;
            }
        } else if wanted == Dir::None {
            // Friction sized to bring him to rest right on the coming center.
            let to_center = (1.0 - m.offset).max(1e-6);
            let need = self.vel * self.vel / (2.0 * to_center);
            let friction = need.min(top / STOP_SECONDS);
            self.vel = (self.vel - friction * dt).max(top * MIN_COAST);
        } else if self.vel > top {
            // Top speed just dropped (slime, haste running out): shed the excess smoothly.
            self.vel = (self.vel - top / BRAKE_SECONDS * dt).max(top);
        } else {
            self.vel = (self.vel + top / ACCEL_SECONDS * dt).min(top);
        }

        let mut remaining = self.vel * dt;
        while remaining > 0.0 {
            let to_center = 1.0 - m.offset;
            if remaining < to_center {
                m.offset += remaining;
                break;
            }
            remaining -= to_center;
            m.tile = grid.wrap(m.next_tile(m.tile));
            m.offset = 0.0;

            if wanted != Dir::None && grid.can_move(m.tile, wanted, false, false) {
                if wanted == m.dir.opposite() {
                    // Came to a halt on the center while braking: turn here.
                    m.dir = wanted;
                    self.vel = 0.0;
                    break;
                }
                if wanted != m.dir {
                    self.vel *= CORNER_KEEP;
                    remaining *= CORNER_KEEP;
                }
                m.dir = wanted;
            } else if wanted == Dir::None || !grid.can_move(m.tile, m.dir, false, false) {
                // Released, or the way ahead is blocked: rest on this center.
                m.dir = Dir::None;
                self.vel = 0.0;
                break;
            }
        }
    }

    /**
     * Applies a hit. Invincibility and the post-hit window ignore it; a
     * berserker's rage makes it land harder and a berserker's draught softer
     * (and feeds his fury); a shield potion's bark soaks it first; the shield
     * takes its share and wears down by as much, breaking when it runs out
     * unless it is too well made to. Blocked means none of it got through.
     */
    pub fn take_damage(&mut self, amount: i32) -> DamageResult {
        if !self.alive || self.invuln > 0.0 || self.invincible() {
            return DamageResult::Immune;
        }
        let mut amount = amount as f64;
        if self.path >= 1 && self.kind == HeroKind::Ragnar {
            amount *= path::RAGE_TAKEN;
        }
        if self.fx.fury > 0.0 {
            amount *= FURY_TAKEN;
            self.fury_stacks = (self.fury_stacks + 1).min(FURY_MAX);
        }
        let mut amount = amount.round().max(1.0) as i32;
        if self.bark > 0 {
            let soaked = self.bark.min(amount);
            self.bark -= soaked;
            amount -= soaked;
            if amount <= 0 {
                self.invuln = BLOCK_INVULN_SECONDS;
                return DamageResult::Blocked;
            }
        }
        if let Some(s) = &mut self.shield {
            let w = progression::ward(s.tier);
            // Raised, the shield takes all of the blow; slung, its share.
            let share = if self.blocking { amount } else { ((amount as f64) * w.block).round() as i32 };
            let floor = if w.unbreakable { 1 } else { 0 };
            let absorbed = share.min(s.durability - floor).max(0);
            s.durability -= absorbed;
            amount -= absorbed;
            if absorbed > 0 {
                self.blocked_hit = true;
                self.hp = (self.hp + w.aegis).min(self.max_hp);
            }
            if s.durability <= 0 {
                self.shield = None;
                self.shield_broke = true;
            }
            if amount <= 0 {
                self.invuln = BLOCK_INVULN_SECONDS;
                return DamageResult::Blocked;
            }
        }
        // In a last stand nothing can take his last breath.
        self.hp = (self.hp - amount).max(if self.last_stand > 0.0 { 1 } else { 0 });
        self.invuln = INVULN_SECONDS;
        if self.hp == 0 {
            self.alive = false;
        }
        if self.kind != HeroKind::Ragnar && self.path >= 7 && self.heart_cooldown <= 0.0 {
            // The unyielding heart (Zenith Heart, Rebel Heart): hurt, he shrugs off every hindrance.
            self.heart = path::DEFIANCE_SECONDS;
            self.heart_cooldown = path::DEFIANCE_COOLDOWN;
        }
        DamageResult::Damaged
    }

    /// Spends `cost` stamina if he has it (false, and winded, if not).
    pub fn exert(&mut self, cost: f64) -> bool {
        self.spend(cost)
    }

    /// Takes `cost` stamina, as much as he has (a blow caught on his guard).
    pub fn drain(&mut self, cost: f64) {
        self.stamina = (self.stamina - cost).max(0.0);
        self.stamina_rest = STAMINA_REST;
    }

    /// Starts a swing if the cooldown and his stamina allow. Returns true if a swing began.
    pub fn try_attack(&mut self) -> bool {
        if !self.alive || self.blocking || self.attack_cooldown > 0.0 || !self.spend(STAMINA_PER_BLOW) {
            return false;
        }
        self.attack_cooldown = progression::blade(self.kind, self.blade).cooldown * self.cooldown_scale();
        self.swing = SWING_SECONDS;
        true
    }

    /**
     * Bifrost Step: a flash of light carries him up to three tiles the way he
     * faces, as far as the floor lets him walk. Returns where he left and
     * where he landed, or None if it is not ready, he is too tired, or there
     * is nowhere to go.
     */
    pub fn try_step(&mut self, grid: &Grid) -> Option<(V, V)> {
        if !self.alive || self.path < 5 || self.jump.is_some() || self.step_cooldown > 0.0 {
            return None;
        }
        let cost = path::STEP_STAMINA * if self.kind == HeroKind::Miyamoto && self.path >= 3 { path::WIND_STEP_COST } else { 1.0 };
        let r = self.dash(grid, path::STEP_TILES as i32, cost)?;
        self.step_cooldown = path::STEP_COOLDOWN;
        self.invuln = self.invuln.max(STEP_IMMUNITY);
        Some(r)
    }

    /**
     * Carries him up to `tiles` tiles the way he faces, as far as the floor lets
     * him walk, for `cost` stamina (a blink, or a blade's lunge). Returns where
     * he left and where he landed, or None if there is nowhere to go.
     */
    pub fn dash(&mut self, grid: &Grid, tiles: i32, cost: f64) -> Option<(V, V)> {
        if !self.alive || self.jump.is_some() {
            return None;
        }
        if self.free_move {
            return self.dash_free(grid, tiles as f64, cost);
        }
        let from = self.occupied();
        let mut to = from;
        for _ in 0..tiles {
            let (dx, dy) = self.facing.vec();
            let n = grid.wrap(P::new(to.x + dx, to.y + dy));
            if free_blocked(grid, to, n.x, n.y) || !grid.is_walkable(n.x, n.y, false) {
                break;
            }
            to = n;
        }
        if to == from || !self.spend(cost) {
            return None;
        }
        let before = self.pos();
        self.m.teleport(to);
        self.free_pos = to.v();
        self.free_vel = V::new(0.0, 0.0);
        self.synced = to;
        self.altitude = grid.elevation(to.x, to.y) as f64;
        self.vel = 0.0;
        if !self.invisible() {
            self.heard_pos = to.v();
            self.heard_age = 0.0;
        }
        Some((before, to.v()))
    }

    /**
     * Fires the held ranged weapon if it is ready. Returns the kind fired, or
     * None. His own weapon never runs dry; a shrine weapon spends ammo, and
     * its last shot puts his own weapon back in his hand.
     */
    pub fn try_fire(&mut self) -> Option<RangedKind> {
        let WeaponKind::Ranged(kind) = self.weapon else { return None };
        let pickup = self.holding_pickup();
        if !self.alive || (pickup && self.ammo == 0) || self.fire_cooldown > 0.0 {
            return None;
        }
        // His own thrown weapon costs stamina like a swing; a shrine's spends its ammo instead.
        if !pickup && !self.spend(STAMINA_PER_BLOW) {
            return None;
        }
        self.fire_cooldown = weapon(kind).cooldown * self.cooldown_scale();
        self.swing = SWING_SECONDS.min(0.12); // a short arm motion for the shot
        if pickup {
            self.ammo -= 1;
            if self.ammo == 0 {
                self.revert_to_sword();
            }
        }
        Some(kind)
    }

    /// A caught boomerang comes back to the quiver, if a shrine boomerang is still held.
    pub fn refund_shot(&mut self) {
        if self.weapon != WeaponKind::Ranged(RangedKind::Boomerang) || !self.holding_pickup() {
            return;
        }
        self.ammo = (self.ammo + 1).min(weapon(RangedKind::Boomerang).ammo);
    }

    /// Takes a shrine weapon in place of his own until its ammo runs out.
    pub fn equip(&mut self, w: RangedKind) {
        self.weapon = WeaponKind::Ranged(w);
        self.ammo = weapon(w).ammo;
        self.fire_cooldown = 0.0;
    }

    /// Puts his own weapon (bought and equipped) back in his hand.
    pub fn revert_to_sword(&mut self) {
        self.weapon = self.main;
        self.ammo = 0;
        self.fire_cooldown = 0.0;
    }

    /// Puts armory tier `tier` in his hand: swung, or thrown if it is a throwing weapon.
    pub fn arm(&mut self, tier: u8) {
        self.blade = tier.clamp(1, progression::TIERS);
        self.main = if progression::blade(self.kind, self.blade).throw.is_some() {
            WeaponKind::Ranged(RangedKind::Hurled(self.kind, self.blade))
        } else {
            WeaponKind::Sword
        };
        let pickup = self.holding_pickup() && self.ammo > 0;
        if !pickup {
            self.weapon = self.main;
            self.ammo = 0;
        }
    }

    /// A shield boon: his own shield mended, or (if it broke) a plain first-tier one.
    pub fn grant_shield(&mut self) {
        match &mut self.shield {
            Some(s) => s.durability = s.max(),
            None => self.shield = Some(ShieldGear::new(1)),
        }
    }

    /// Drinks a potion with a timed or instant effect (Extra Life is the game's to handle).
    pub fn drink(&mut self, kind: PotionKind) {
        use PotionKind as K;
        let secs = gear::potion_seconds(kind);
        match kind {
            K::Health => {
                self.heal(HEALTH_POTION);
                self.invuln = self.invuln.max(HEALTH_IMMUNITY);
            }
            K::Stamina => self.stamina = STAMINA_MAX,
            K::Strength => self.fx.strength = secs,
            K::Haste => self.fx.haste = secs,
            K::Bark => self.bark = BARK_HP,
            K::Fury => {
                self.fx.fury = secs;
                self.fury_stacks = 0;
            }
            K::Fire => self.fx.fire = secs,
            K::Invulnerability => self.invincible_timer = self.invincible_timer.max(secs),
            K::Ragnarok => self.fx.ragnarok = secs,
            K::Spirit => self.fx.spirit = secs,
            K::Life => {}
        }
    }

    /// The World Tree's restoration: full health and stamina, and every ill (slime, the tonic's drain) purged.
    pub fn restore(&mut self) {
        self.hp = self.max_hp;
        self.stamina = STAMINA_MAX;
        self.slow_timer = 0.0;
        self.fx.ragnarok = 0.0;
    }
    pub fn grant_invincibility(&mut self) {
        self.invincible_timer = item(ItemKind::Invincible).seconds;
    }
    pub fn grant_speed(&mut self) {
        self.speed_timer = item(ItemKind::Speed).seconds;
    }
    pub fn grant_invisibility(&mut self, seconds: f64) {
        self.invisible_timer = seconds;
    }

    /// Stepping in slime. Refreshes while he stays in it.
    pub fn slow(&mut self) {
        self.slow_timer = SLIME_SLOW_SECONDS;
    }

    pub fn heal(&mut self, amount: i32) {
        self.hp = (self.hp + amount).min(self.max_hp);
    }

    /// Picks him up and sets him down on `at` (a debugging hook): nothing else changes.
    pub fn place(&mut self, at: P) {
        self.m.teleport(at);
        self.free_pos = at.v();
        self.free_vel = V::new(0.0, 0.0);
        self.synced = at;
        self.jump = None;
    }

    /// New life: full health, bare sword, all boons gone. Death costs everything.
    pub fn respawn(&mut self, at: P) {
        self.m.teleport(at);
        self.free_pos = at.v();
        self.free_vel = V::new(0.0, 0.0);
        self.synced = at;
        self.heard_pos = at.v();
        self.heard_age = 0.0;
        self.alive = true;
        self.motion = 0.0;
        self.face(Dir::Down);
        self.hp = self.health_cap();
        self.invuln = 0.0;
        self.attack_cooldown = 0.0;
        self.swing = 0.0;
        // Death takes shrine pickups and potion effects; what he bought stays his.
        self.revert_to_sword();
        self.shield_broke = false;
        self.fx = Effects::default();
        self.regen_carry = 0.0;
        self.drain_carry = 0.0;
        self.stamina = STAMINA_MAX;
        self.stamina_rest = 0.0;
        self.winded = false;
        self.heart = 0.0;
        self.heart_cooldown = 0.0;
        self.step_cooldown = 0.0;
        self.bark = 0;
        self.fury_stacks = 0;
        self.blocked_hit = false;
        self.last_stand = 0.0;
        self.max_hp = self.health_cap();
        self.invincible_timer = 0.0;
        self.invisible_timer = 0.0;
        self.speed_timer = 0.0;
        self.slow_timer = 0.0;
        self.m.speed = self.base_speed;
        self.jump = None;
        self.altitude = 0.0;
        self.vel = 0.0;
        self.turn_timer = 0.0;
    }
}

/// Whether free movement may not enter tile (x, y) from tile `from`: no floor, or a climb without a ramp.
fn free_blocked(grid: &Grid, from: P, x: i32, y: i32) -> bool {
    let n = grid.wrap(P::new(x, y));
    if !grid.is_walkable(n.x, n.y, false) {
        return true;
    }
    let climb = grid.elevation(n.x, n.y) - grid.elevation(from.x, from.y);
    climb > 0 && !(climb == 1 && (grid.is_ramp(from.x, from.y) || grid.is_ramp(n.x, n.y)))
}

/// The angle of one of the four directions in the grid's x/y plane (Down is +y).
fn angle_of(d: Dir) -> f64 {
    let v = d.or_down().vecf();
    v.y.atan2(v.x)
}

/**
 * Where a free leap from `from` along `aim` comes down: two tiles on if that
 * spot is floor no more than a level up and nothing but pits, foes or low
 * ground lies between (no rock), else one tile on, else nowhere. Returns the
 * landing tile and point.
 */
fn free_landing(grid: &Grid, from: V, aim: V) -> Option<(P, V)> {
    let start = grid.wrap(P::new(from.x.round() as i32, from.y.round() as i32));
    let base = grid.elevation(start.x, start.y);
    let at = |d: f64| V::new(from.x + aim.x * d, from.y + aim.y * d);
    let tile = |p: V| grid.wrap(P::new(p.x.round() as i32, p.y.round() as i32));
    for reach in [2.0, 1.0] {
        let land = at(reach);
        let t = tile(land);
        if t == start || !grid.is_walkable(t.x, t.y, false) || grid.elevation(t.x, t.y) - base > 1 {
            continue;
        }
        let mut d = FREE_PROBE;
        let mut clear = true;
        while d < reach {
            let q = tile(at(d));
            if grid.get(q.x, q.y) == Tile::Rock {
                clear = false;
                break;
            }
            d += FREE_PROBE;
        }
        if clear {
            return Some((t, land));
        }
    }
    None
}

/// The floor height (in levels) under a free-moving point: level ground, or partway up a ramp.
fn free_floor(grid: &Grid, p: V) -> f64 {
    let t = grid.wrap(P::new(p.x.round() as i32, p.y.round() as i32));
    let low = grid.elevation(t.x, t.y);
    if !grid.is_ramp(t.x, t.y) {
        return low as f64;
    }
    for d in CARDINALS {
        let n = grid.neighbor(t, d);
        if grid.get(n.x, n.y) != Tile::Rock && !grid.is_ramp(n.x, n.y) && grid.elevation(n.x, n.y) > low {
            let (dx, dy) = d.vec();
            let along = (p.x - t.x as f64) * dx as f64 + (p.y - t.y as f64) * dy as f64 + 0.5;
            return low as f64 + along.clamp(0.0, 1.0);
        }
    }
    low as f64
}

#[cfg(test)]
mod tests {
    #[test]
    fn walks_freely_and_slides_along_walls() {
        let g = corridor(8);
        let mut h = Hero::new(P::new(1, 1), 5.0);
        h.free_move = true;
        // Straight along the corridor, then pressing into its wall: he goes on, but never in.
        h.wish = V::new(1.0, 1.0);
        for _ in 0..120 {
            h.update(1.0 / 120.0, &g, Dir::None);
        }
        let p = h.pos();
        assert!(p.x > 2.5, "he moved along: {p:?}");
        assert!((p.y - 1.0).abs() <= 0.5 - FREE_RADIUS + 1e-6, "he stayed off the wall: {p:?}");
        assert_eq!(h.facing, Dir::Right);
        assert_eq!(h.occupied(), P::new(p.x.round() as i32, 1));
    }

    use super::*;
    use crate::logic::types::Tile;

    fn corridor(len: i32) -> Grid {
        let mut tiles = vec![Tile::Rock; (len * 3) as usize];
        for x in 1..len - 1 {
            tiles[(len + x) as usize] = Tile::Open;
        }
        Grid::new(len, 3, tiles, vec![0; (len * 3) as usize], vec![false; (len * 3) as usize])
    }

    const DT: f64 = 1.0 / 120.0;

    /// A walled room, open inside.
    fn room(w: i32, h: i32) -> Grid {
        let mut tiles = vec![Tile::Rock; (w * h) as usize];
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                tiles[(y * w + x) as usize] = Tile::Open;
            }
        }
        Grid::new(w, h, tiles, vec![0; (w * h) as usize], vec![false; (w * h) as usize])
    }

    fn run(h: &mut Hero, g: &Grid, seconds: f64) {
        for _ in 0..(seconds / DT) as usize {
            h.update(DT, g, Dir::None);
        }
    }

    #[test]
    fn in_free_movement_he_aims_and_leaps_any_way() {
        let g = room(12, 12);
        let mut h = Hero::new(P::new(4, 4), 5.0);
        h.free_move = true;
        // A nudge down and to the right, not at 45 degrees, then let go.
        h.wish = V::new(1.0, 0.5);
        run(&mut h, &g, 0.05);
        h.wish = V::new(0.0, 0.0);
        run(&mut h, &g, 0.5);
        let aim = h.aim();
        let want = V::new(2.0 / 5f64.sqrt(), 1.0 / 5f64.sqrt());
        assert!((aim.x - want.x).abs() < 1e-6 && (aim.y - want.y).abs() < 1e-6, "aims where he turned: {aim:?}");
        assert_eq!(h.facing, Dir::Right, "the nearest of the four, for whatever still wants one");
        // A leap follows the aim two tiles and comes down where it lands, not on a tile center.
        let from = h.pos();
        assert!(h.try_jump(&g));
        run(&mut h, &g, JUMP_SECONDS + 0.05);
        let to = h.pos();
        assert!(((to.x - from.x) - want.x * 2.0).abs() < 0.05 && ((to.y - from.y) - want.y * 2.0).abs() < 0.05, "{from:?} -> {to:?}");
        assert!((to.x - to.x.round()).abs() > 0.05 || (to.y - to.y.round()).abs() > 0.05, "snapped to a tile center: {to:?}");
        assert_eq!(h.occupied(), P::new(to.x.round() as i32, to.y.round() as i32));
        // On the grid the aim is one of the four, as before.
        h.free_move = false;
        h.face(Dir::Up);
        assert_eq!((h.aim().x, h.aim().y), (0.0, -1.0));
    }

    #[test]
    fn a_free_leap_clears_a_pit_but_not_a_wall() {
        let mut g = room(10, 7);
        for y in 1..6 {
            g.set(4, y, Tile::Pit);
        }
        let mut h = Hero::new(P::new(3, 3), 5.0);
        h.free_move = true;
        // Slightly off the row, heading a little upward: over the pit and down on the far side.
        h.heading = -0.15f64;
        assert!(h.try_jump(&g));
        run(&mut h, &g, JUMP_SECONDS + 0.05);
        let to = h.pos();
        assert!(to.x > 4.5 && h.occupied().x == 5, "over the pit: {to:?}");
        // Into the wall: one tile would do, two would not; right against it, a hop in place.
        let mut h = Hero::new(P::new(7, 3), 5.0);
        h.free_move = true;
        h.heading = 0.0;
        assert!(h.try_jump(&g));
        run(&mut h, &g, JUMP_SECONDS + 0.05);
        assert_eq!(h.occupied(), P::new(8, 3));
        assert!(h.pos().x <= 8.5 - FREE_RADIUS + 1e-6, "kept out of the wall: {:?}", h.pos());
    }

    #[test]
    fn a_free_dash_goes_any_way_and_stops_short_of_a_wall() {
        let g = room(8, 8);
        let mut h = Hero::new(P::new(2, 2), 5.0);
        h.free_move = true;
        h.path = 5;
        h.stamina = STAMINA_MAX;
        // Bifrost Step, mostly right and a little down: three tiles along it.
        h.heading = 1f64.atan2(3.0);
        let (from, to) = h.try_step(&g).expect("a step");
        let d = ((to.x - from.x).powi(2) + (to.y - from.y).powi(2)).sqrt();
        assert!((d - 3.0).abs() < 0.1, "went {d}");
        assert_eq!(h.pos(), to);
        // Straight at the wall two tiles off, it stops short, still in the room.
        h.step_cooldown = 0.0;
        h.stamina = STAMINA_MAX;
        h.heading = 0.0;
        let (_, to) = h.try_step(&g).expect("a step");
        assert!(to.x <= 6.5 - FREE_RADIUS + 1e-6 && to.x > 5.5, "{to:?}");
    }

    #[test]
    fn turns_on_the_spot_before_walking() {
        let g = corridor(40);
        let mut h = Hero::new(P::new(3, 1), 5.2);
        // Up is a wall: he turns to face it and stays put.
        for _ in 0..30 {
            h.update(DT, &g, Dir::Up);
        }
        assert_eq!(h.facing, Dir::Up);
        assert_eq!(h.pos(), crate::logic::types::V::new(3.0, 1.0));
        // A tap to the right turns without stepping.
        h.update(DT, &g, Dir::Right);
        h.update(DT, &g, Dir::None);
        assert_eq!(h.facing, Dir::Right);
        assert_eq!(h.pos().x, 3.0);
        // Holding it walks.
        for _ in 0..60 {
            h.update(DT, &g, Dir::Right);
        }
        assert!(h.pos().x > 3.5);
    }

    #[test]
    fn accelerates_from_rest_instead_of_snapping() {
        let g = corridor(40);
        let mut h = Hero::new(P::new(1, 1), 5.2);
        h.facing = Dir::Right;
        h.update(DT, &g, Dir::Right);
        let first = h.pos().x - 1.0;
        assert!(first > 0.0 && first < 5.2 * DT * 0.2, "first step {first}");
    }

    #[test]
    fn top_speed_is_unchanged() {
        let g = corridor(60);
        let mut h = Hero::new(P::new(1, 1), 5.2);
        for _ in 0..120 {
            h.update(DT, &g, Dir::Right);
        }
        assert!((h.vel - 5.2).abs() < 1e-9);
        let x0 = h.pos().x;
        for _ in 0..60 {
            h.update(DT, &g, Dir::Right);
        }
        assert!((h.pos().x - x0 - 5.2 * 0.5).abs() < 1e-6, "moved {}", h.pos().x - x0);
    }

    #[test]
    fn coasts_to_rest_on_a_tile_center() {
        let g = corridor(60);
        let mut h = Hero::new(P::new(1, 1), 5.2);
        for _ in 0..70 {
            h.update(DT, &g, Dir::Right);
        }
        let mut last_step = f64::MAX;
        let mut x = h.pos().x;
        for _ in 0..240 {
            h.update(DT, &g, Dir::None);
            let nx = h.pos().x;
            let step = nx - x;
            assert!(step <= last_step + 1e-9, "friction never speeds him up");
            last_step = step;
            x = nx;
        }
        assert_eq!(h.dir(), Dir::None);
        assert_eq!(h.m.offset, 0.0);
        assert_eq!(h.vel, 0.0);
    }

    #[test]
    fn brakes_before_turning_around() {
        let g = corridor(60);
        let mut h = Hero::new(P::new(1, 1), 5.2);
        for _ in 0..120 {
            h.update(DT, &g, Dir::Right);
        }
        let x0 = h.pos().x;
        h.update(DT, &g, Dir::Left);
        assert!(h.pos().x > x0, "still sliding forward while braking");
        for _ in 0..60 {
            h.update(DT, &g, Dir::Left);
        }
        assert_eq!(h.dir(), Dir::Left);
    }

    #[test]
    fn every_blow_costs_stamina_and_it_comes_back_after_a_rest() {
        let g = corridor(10);
        let mut h = Hero::new(P::new(3, 1), 5.2);
        let mut swings = 0;
        for _ in 0..40 {
            h.attack_cooldown = 0.0;
            if h.try_attack() {
                swings += 1;
            }
        }
        assert_eq!(swings, (STAMINA_MAX / STAMINA_PER_BLOW) as usize, "out of breath after a flurry");
        assert!(h.winded);
        for _ in 0..600 {
            h.update(DT, &g, Dir::None);
        }
        assert_eq!(h.stamina, STAMINA_MAX);
        // Forge-Heart (path 7) takes over when his breath runs out.
        h.path = 7;
        for _ in 0..40 {
            h.attack_cooldown = 0.0;
            h.try_attack();
        }
        assert!(h.heart > 0.0 && h.tireless());
    }

    #[test]
    fn a_great_shield_never_breaks_and_heals_him_as_it_blocks() {
        let mut h = Hero::new(P::new(3, 1), 5.2);
        h.shield = Some(ShieldGear::new(10));
        h.hp = 50;
        for _ in 0..200 {
            h.invuln = 0.0;
            h.take_damage(40);
            if !h.alive {
                break;
            }
            h.hp = h.hp.max(60);
        }
        assert!(h.shield.is_some(), "tier 8 and up never break");
        let mut h = Hero::new(P::new(3, 1), 5.2);
        h.shield = Some(ShieldGear::new(10));
        h.hp = 50;
        h.take_damage(10);
        assert!(h.hp > 50 - 2, "the block healed most of what got through: {}", h.hp);
    }

    #[test]
    fn bifrost_step_blinks_him_ahead_until_a_wall() {
        let g = corridor(8);
        let mut h = Hero::new(P::new(1, 1), 5.2);
        h.facing = Dir::Right;
        assert!(h.try_step(&g).is_none(), "he has not walked the path that far yet");
        h.path = 5;
        let (_, to) = h.try_step(&g).expect("blinks");
        assert_eq!(to, V::new(4.0, 1.0));
        assert!(h.try_step(&g).is_none(), "it needs a moment to come again");
        h.step_cooldown = 0.0;
        let (_, to) = h.try_step(&g).expect("blinks again");
        assert_eq!(to, V::new(6.0, 1.0), "the wall stops the light");
    }

    #[test]
    fn hitbox_tile_is_still_the_mover_tile() {
        let g = corridor(20);
        let mut h = Hero::new(P::new(1, 1), 5.2);
        while h.m.offset < 0.5 {
            h.update(DT, &g, Dir::Right);
        }
        assert_eq!(h.occupied(), P::new(2, 1));
    }
}
