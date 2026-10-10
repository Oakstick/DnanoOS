import { Grid } from '../world/Grid';
import { Dir, Vec2 } from '../world/types';
import { Mover } from './Mover';

/** Seconds a jump takes, and how high (in levels) the arc peaks above the straight line. */
export const JUMP_SECONDS = 0.42;
export const JUMP_PEAK = 1.1;

interface JumpState {
  from: Vec2;
  to: Vec2;
  fromH: number;
  toH: number;
  t: number;
}
import { ITEMS, RangedKind, SPEED_BOOST, WeaponKind, WEAPONS } from './Weapons';

export const HERO_MAX_HP = 100;
export const CONTACT_DAMAGE = 25;
const INVULN_SECONDS = 1.0;
const BLOCK_INVULN_SECONDS = 0.5;
const ATTACK_COOLDOWN = 0.45;
const SWING_SECONDS = 0.18;
/** Speed multiplier while slowed by slime. */
export const SLIME_SLOW = 0.5;
/** How long a step in slime keeps slowing him after he leaves it. */
export const SLIME_SLOW_SECONDS = 1.2;

export type DamageResult = 'damaged' | 'blocked' | 'immune';

/**
 * Popolon. Makes footstep noise while moving; enemies with hearing home in
 * on `heardPos` rather than the true position. Fights with a short sword,
 * a ranged weapon from the shrine, and bombs.
 */
export class Hero extends Mover {
  heardPos: Vec2;
  heardAge = 0; // seconds since the hero last made noise
  motion = 0; // 0..1 visual stride throttle
  alive = true;
  facing: Dir = 'down';

  readonly maxHp = HERO_MAX_HP;
  hp = HERO_MAX_HP;
  invuln = 0; // seconds of damage immunity remaining after a hit
  attackCooldown = 0;
  swing = 0; // seconds of swing animation remaining

  /** The sword by default; a shrine weapon replaces it until its ammo is spent. */
  weapon: WeaponKind = 'sword';
  ammo = 0;
  fireCooldown = 0;

  shield = 0; // hits the shield will still absorb
  invincibleTimer = 0;
  invisibleTimer = 0; // enemies cannot sense or touch him while this runs
  speedTimer = 0;
  slowTimer = 0; // slime underfoot
  readonly baseSpeed: number;
  private jump: JumpState | null = null;
  /** Floor elevation at his feet, or the arc height while airborne, in levels. */
  altitude = 0;

  constructor(start: Vec2, speed: number) {
    super(start, speed);
    this.baseSpeed = speed;
    this.heardPos = { ...start };
  }

  get invulnerable(): boolean {
    return this.invuln > 0;
  }

  get invincible(): boolean {
    return this.invincibleTimer > 0;
  }

  get invisible(): boolean {
    return this.invisibleTimer > 0;
  }

  get hasted(): boolean {
    return this.speedTimer > 0;
  }

  get slowed(): boolean {
    return this.slowTimer > 0;
  }

  get airborne(): boolean {
    return this.jump !== null;
  }

  /** 0..1 through the current jump, 0 on the ground. */
  get jumpProgress(): number {
    return this.jump ? this.jump.t / JUMP_SECONDS : 0;
  }

  /** While airborne the position follows the jump line instead of the grid mover. */
  override get pos(): Vec2 {
    if (!this.jump) return super.pos;
    const t = this.jump.t / JUMP_SECONDS;
    return {
      x: this.jump.from.x + (this.jump.to.x - this.jump.from.x) * t,
      y: this.jump.from.y + (this.jump.to.y - this.jump.from.y) * t,
    };
  }

  /**
   * Leap in the facing direction: two tiles when the landing is clear (over a
   * pit, an enemy, or up one level), one tile otherwise, or a hop in place.
   */
  tryJump(grid: Grid): boolean {
    if (!this.alive || this.jump) return false;
    const from = { ...this.occupied };
    const to = grid.jumpLanding(from, this.facing) ?? from;
    this.tile = from;
    this.offset = 0;
    this.dir = 'none';
    this.jump = {
      from, to,
      fromH: grid.elevation(from.x, from.y),
      toH: grid.elevation(to.x, to.y),
      t: 0,
    };
    return true;
  }

  get swinging(): boolean {
    return this.swing > 0;
  }

  get hasRangedWeapon(): boolean {
    return this.weapon !== 'sword';
  }

  /** 0..1 progress through the current swing, 0 when idle. */
  get swingProgress(): number {
    return this.swing > 0 ? 1 - this.swing / SWING_SECONDS : 0;
  }

  update(dt: number, grid: Grid, wanted: Dir): void {
    this.invuln = Math.max(0, this.invuln - dt);
    this.attackCooldown = Math.max(0, this.attackCooldown - dt);
    this.fireCooldown = Math.max(0, this.fireCooldown - dt);
    this.swing = Math.max(0, this.swing - dt);
    this.invincibleTimer = Math.max(0, this.invincibleTimer - dt);
    this.invisibleTimer = Math.max(0, this.invisibleTimer - dt);
    this.speedTimer = Math.max(0, this.speedTimer - dt);
    this.slowTimer = Math.max(0, this.slowTimer - dt);
    this.speed = this.baseSpeed * (this.hasted ? SPEED_BOOST : 1) * (this.slowed ? SLIME_SLOW : 1);
    if (!this.alive) return;

    if (this.jump) {
      this.jump.t = Math.min(JUMP_SECONDS, this.jump.t + dt);
      const t = this.jump.t / JUMP_SECONDS;
      this.altitude = this.jump.fromH + (this.jump.toH - this.jump.fromH) * t + JUMP_PEAK * Math.sin(Math.PI * t);
      this.motion = Math.min(1, this.motion + dt * 6);
      if (this.jump.t >= JUMP_SECONDS) {
        // Land: snap to the target tile; the thud is a footstep the skeleton can hear.
        this.tile = { ...this.jump.to };
        this.offset = 0;
        this.jump = null;
        this.altitude = grid.elevation(this.tile.x, this.tile.y);
        if (!this.invisible) {
          this.heardPos = { ...this.tile };
          this.heardAge = 0;
        }
      }
      return;
    }

    const before = this.pos;
    // Hold-to-move: he walks only while a direction is held, and rests on the next tile center when released.
    this.step(dt, grid, () => wanted, false, true, true);
    const after = this.pos;
    if (this.dir !== 'none') this.facing = this.dir;
    this.altitude = this.elevationOn(grid);

    const moved = before.x !== after.x || before.y !== after.y;
    if (moved) {
      // An invisible hero makes no footsteps for the skeleton to follow.
      if (!this.invisible) {
        this.heardPos = after;
        this.heardAge = 0;
      } else {
        this.heardAge += dt;
      }
      this.motion = Math.min(1, this.motion + dt * 6);
    } else {
      this.heardAge += dt;
      this.motion = Math.max(0, this.motion - dt * 4);
    }
  }

  /** Applies a hit. Invincibility and the post-hit window ignore it; a shield absorbs it. */
  takeDamage(amount: number): DamageResult {
    if (!this.alive || this.invuln > 0 || this.invincible) return 'immune';
    if (this.shield > 0) {
      this.shield--;
      this.invuln = BLOCK_INVULN_SECONDS;
      return 'blocked';
    }
    this.hp = Math.max(0, this.hp - amount);
    this.invuln = INVULN_SECONDS;
    if (this.hp === 0) this.alive = false;
    return 'damaged';
  }

  /** Starts a swing if the cooldown allows. Returns true if a swing began. */
  tryAttack(): boolean {
    if (!this.alive || this.attackCooldown > 0) return false;
    this.attackCooldown = ATTACK_COOLDOWN;
    this.swing = SWING_SECONDS;
    return true;
  }

  /**
   * Spends a shot of the held ranged weapon if one is ready. Returns the kind
   * fired, or null. The last shot drops the hero back to the sword.
   */
  tryFire(): RangedKind | null {
    if (!this.alive || this.weapon === 'sword' || this.ammo <= 0 || this.fireCooldown > 0) return null;
    const kind = this.weapon;
    this.ammo--;
    this.fireCooldown = WEAPONS[kind].cooldown;
    this.swing = Math.min(SWING_SECONDS, 0.12); // a short arm motion for the shot
    if (this.ammo <= 0) this.revertToSword();
    return kind;
  }

  /** A caught boomerang comes back to the quiver, if the boomerang is still held. */
  refundShot(): void {
    if (this.weapon !== 'boomerang') return;
    this.ammo = Math.min(WEAPONS.boomerang.ammo, this.ammo + 1);
  }

  /** Takes a shrine weapon in place of the sword. */
  equip(weapon: RangedKind): void {
    this.weapon = weapon;
    this.ammo = WEAPONS[weapon].ammo;
    this.fireCooldown = 0;
  }

  revertToSword(): void {
    this.weapon = 'sword';
    this.ammo = 0;
    this.fireCooldown = 0;
  }

  grantShield(): void {
    this.shield = ITEMS.shield.charges;
  }

  grantInvincibility(): void {
    this.invincibleTimer = ITEMS.invincible.seconds;
  }

  grantSpeed(): void {
    this.speedTimer = ITEMS.speed.seconds;
  }

  grantInvisibility(seconds: number): void {
    this.invisibleTimer = seconds;
  }

  /** Stepping in slime. Refreshes while he stays in it. */
  slow(): void {
    this.slowTimer = SLIME_SLOW_SECONDS;
  }

  heal(amount = HERO_MAX_HP): void {
    this.hp = Math.min(this.maxHp, this.hp + amount);
  }

  /** New life: full health, bare sword, all boons gone. Death costs everything. */
  respawn(at: Vec2): void {
    this.teleport(at);
    this.heardPos = { ...at };
    this.heardAge = 0;
    this.alive = true;
    this.motion = 0;
    this.facing = 'down';
    this.hp = this.maxHp;
    this.invuln = 0;
    this.attackCooldown = 0;
    this.swing = 0;
    this.revertToSword();
    this.shield = 0;
    this.invincibleTimer = 0;
    this.invisibleTimer = 0;
    this.speedTimer = 0;
    this.slowTimer = 0;
    this.speed = this.baseSpeed;
    this.jump = null;
    this.altitude = 0;
  }
}
