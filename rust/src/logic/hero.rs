use super::gear::{self, ShieldGear, ShieldKind};
use super::grid::Grid;
use super::items::{item, weapon, ItemKind, RangedKind, WeaponKind, SPEED_BOOST};
use super::mover::Mover;
use super::types::{Dir, P, V};

/// Seconds a jump takes, and how high (in levels) the arc peaks above the straight line.
pub const JUMP_SECONDS: f64 = 0.42;
pub const JUMP_PEAK: f64 = 1.1;

pub const HERO_MAX_HP: i32 = 100;
pub const CONTACT_DAMAGE: i32 = 25;
const INVULN_SECONDS: f64 = 1.0;
const BLOCK_INVULN_SECONDS: f64 = 0.5;
/// Potion effects on attack speed, damage taken and healing.
const ARCANE_COOLDOWN: f64 = 0.5;
const IRON_SKIN_TAKEN: f64 = 0.4;
const REGEN_PER_SECOND: f64 = 5.0;
/// A health potion's moment of immunity.
pub const HEALTH_IMMUNITY: f64 = 1.5;
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

/// Seconds left on each timed potion (swiftness and invulnerability use the older boon timers).
#[derive(Clone, Copy, Debug, Default)]
pub struct Effects {
    pub strength: f64,
    pub arcane: f64,
    pub regeneration: f64,
    pub iron_skin: f64,
    pub lightning: f64,
    pub night_vision: f64,
    pub true_sight: f64,
}

impl Effects {
    fn tick(&mut self, dt: f64) {
        for t in [
            &mut self.strength,
            &mut self.arcane,
            &mut self.regeneration,
            &mut self.iron_skin,
            &mut self.lightning,
            &mut self.night_vision,
            &mut self.true_sight,
        ] {
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
    from: P,
    to: P,
    from_h: f64,
    to_h: f64,
    t: f64,
}

/**
 * Sebastian. Makes footstep noise while moving; enemies with hearing home in
 * on `heard_pos` rather than the true position. Fights with a short sword,
 * a ranged weapon from the shrine, and bombs.
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
    /// The weapon he bought and equipped: the axe, or a ranged weapon with endless use.
    pub main: WeaponKind,
    /// The axe's level (1..=3): reach, arc and damage of every swing.
    pub axe_level: u8,
    pub ammo: u32,
    pub fire_cooldown: f64,

    /// The shield on his arm, wearing down as it blocks.
    pub shield: Option<ShieldGear>,
    /// Set on the hit that breaks the shield; the game clears it.
    pub shield_broke: bool,
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
}

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
            axe_level: 1,
            ammo: 0,
            fire_cooldown: 0.0,
            shield: Some(ShieldGear::new(ShieldKind::Round)),
            shield_broke: false,
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
    /// Damage multiplier from a strength potion.
    pub fn might(&self) -> i32 {
        if self.fx.strength > 0.0 { 2 } else { 1 }
    }
    fn cooldown_scale(&self) -> f64 {
        if self.fx.arcane > 0.0 { ARCANE_COOLDOWN } else { 1.0 }
    }
    pub fn occupied(&self) -> P {
        self.m.occupied()
    }
    pub fn dir(&self) -> Dir {
        self.m.dir
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
            None => self.m.pos(),
            Some(j) => {
                let t = j.t / JUMP_SECONDS;
                V::new(
                    j.from.x as f64 + (j.to.x - j.from.x) as f64 * t,
                    j.from.y as f64 + (j.to.y - j.from.y) as f64 * t,
                )
            }
        }
    }

    /**
     * Leap in the facing direction: two tiles when the landing is clear (over a
     * pit, an enemy, or up one level), one tile otherwise, or a hop in place.
     */
    pub fn try_jump(&mut self, grid: &Grid) -> bool {
        if !self.alive || self.jump.is_some() {
            return false;
        }
        let from = self.m.occupied();
        let to = grid.jump_landing(from, self.facing).unwrap_or(from);
        self.m.tile = from;
        self.m.offset = 0.0;
        self.m.dir = Dir::None;
        self.jump = Some(JumpState {
            from,
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
        self.fx.tick(dt);
        if self.fx.regeneration > 0.0 && self.alive {
            self.regen_carry += REGEN_PER_SECOND * dt;
            let whole = self.regen_carry.floor();
            self.regen_carry -= whole;
            self.heal(whole as i32);
        }
        self.m.speed = self.base_speed
            * if self.hasted() { SPEED_BOOST } else { 1.0 }
            * if self.slowed() { SLIME_SLOW } else { 1.0 };
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
                self.altitude = grid.elevation(j.to.x, j.to.y) as f64;
                self.vel *= LANDING_KEEP;
                if !self.invisible() {
                    self.heard_pos = j.to.v();
                    self.heard_age = 0.0;
                }
            }
            return;
        }

        let before = self.pos();
        // Hold-to-move: he runs only while a direction is held, and coasts to rest on the next tile center.
        self.drive(dt, grid, wanted);
        let after = self.pos();
        if self.m.dir != Dir::None {
            self.facing = self.m.dir;
        }
        self.altitude = self.m.elevation_on(grid);

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
     * Applies a hit. Invincibility and the post-hit window ignore it; iron
     * skin shrinks it; the shield takes its share and wears down by as much,
     * breaking when it runs out. Blocked means none of it got through.
     */
    pub fn take_damage(&mut self, amount: i32) -> DamageResult {
        if !self.alive || self.invuln > 0.0 || self.invincible() {
            return DamageResult::Immune;
        }
        let mut amount = amount;
        if self.fx.iron_skin > 0.0 {
            amount = ((amount as f64) * IRON_SKIN_TAKEN).ceil() as i32;
        }
        if let Some(s) = &mut self.shield {
            let share = ((amount as f64) * gear::shield(s.kind).block).round() as i32;
            let absorbed = share.min(s.durability);
            s.durability -= absorbed;
            amount -= absorbed;
            if s.durability <= 0 {
                self.shield = None;
                self.shield_broke = true;
            }
            if amount <= 0 {
                self.invuln = BLOCK_INVULN_SECONDS;
                return DamageResult::Blocked;
            }
        }
        self.hp = (self.hp - amount).max(0);
        self.invuln = INVULN_SECONDS;
        if self.hp == 0 {
            self.alive = false;
        }
        DamageResult::Damaged
    }

    /// Starts a swing if the cooldown allows. Returns true if a swing began.
    pub fn try_attack(&mut self) -> bool {
        if !self.alive || self.attack_cooldown > 0.0 {
            return false;
        }
        self.attack_cooldown = gear::axe(self.axe_level).cooldown * self.cooldown_scale();
        self.swing = SWING_SECONDS;
        true
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

    /// Sets his own weapon and axe level from what he owns.
    pub fn arm(&mut self, arsenal: &gear::Arsenal) {
        self.axe_level = arsenal.axe_level();
        self.main = arsenal.main_ranged().map(WeaponKind::Ranged).unwrap_or(WeaponKind::Sword);
        let pickup = self.holding_pickup() && self.ammo > 0;
        if !pickup {
            self.weapon = self.main;
            self.ammo = 0;
        }
    }

    /// A shield boon or shield potion: a fresh round shield, or his own mended.
    pub fn grant_shield(&mut self) {
        match &mut self.shield {
            Some(s) => s.durability = gear::shield(s.kind).durability,
            None => self.shield = Some(ShieldGear::new(ShieldKind::Round)),
        }
    }

    /// Drinks a potion with a timed or instant effect (Extra Life is the game's to handle).
    pub fn drink(&mut self, kind: crate::logic::types::PotionKind) {
        use crate::logic::types::PotionKind as K;
        let secs = gear::potion(kind).seconds;
        match kind {
            K::Health => {
                self.heal(50);
                self.invuln = self.invuln.max(HEALTH_IMMUNITY);
            }
            K::Regeneration => self.fx.regeneration = secs,
            K::Swiftness => self.speed_timer = secs,
            K::NightVision => self.fx.night_vision = secs,
            K::TrueSight => self.fx.true_sight = secs,
            K::Strength => self.fx.strength = secs,
            K::IronSkin => self.fx.iron_skin = secs,
            K::Arcane => self.fx.arcane = secs,
            K::Lightning => self.fx.lightning = secs,
            K::Invulnerability => self.invincible_timer = secs,
            K::Life => {}
        }
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

    /// New life: full health, bare sword, all boons gone. Death costs everything.
    pub fn respawn(&mut self, at: P) {
        self.m.teleport(at);
        self.heard_pos = at.v();
        self.heard_age = 0.0;
        self.alive = true;
        self.motion = 0.0;
        self.facing = Dir::Down;
        self.hp = self.max_hp;
        self.invuln = 0.0;
        self.attack_cooldown = 0.0;
        self.swing = 0.0;
        // Death takes shrine pickups and potion effects; what he bought stays his.
        self.revert_to_sword();
        self.shield_broke = false;
        self.fx = Effects::default();
        self.regen_carry = 0.0;
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

#[cfg(test)]
mod tests {
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
    fn hitbox_tile_is_still_the_mover_tile() {
        let g = corridor(20);
        let mut h = Hero::new(P::new(1, 1), 5.2);
        while h.m.offset < 0.5 {
            h.update(DT, &g, Dir::Right);
        }
        assert_eq!(h.occupied(), P::new(2, 1));
    }
}
