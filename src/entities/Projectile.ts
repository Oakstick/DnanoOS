import { Grid } from '../world/Grid';
import { Dir, DIR_VECTORS, OPPOSITE, Vec2 } from '../world/types';
import { Enemy } from './Enemy';
import { RangedKind, WeaponDef, WEAPONS } from './Weapons';

export type ProjectileState = 'flying' | 'wall' | 'spent' | 'caught';

const CATCH_RADIUS = 0.5;

/**
 * A ranged shot. Flies straight along a cardinal direction, passes through
 * tunnels and over the crypt floor, and breaks on rock or at max range.
 * Piercing shots keep going after a hit; returning shots fly back to be caught.
 */
export class Projectile {
  pos: Vec2;
  dir: Dir;
  readonly kind: RangedKind;
  readonly def: WeaponDef;
  traveled = 0;
  alive = true;
  returning = false;
  /** Enemies already struck, so piercing shots hit each one once. */
  readonly struck = new Set<Enemy>();

  constructor(start: Vec2, dir: Dir, kind: RangedKind = 'arrow') {
    this.pos = { ...start };
    this.dir = dir === 'none' ? 'down' : dir;
    this.kind = kind;
    this.def = WEAPONS[kind];
  }

  /** Tile the shot is currently over. */
  get tile(): Vec2 {
    return { x: Math.round(this.pos.x), y: Math.round(this.pos.y) };
  }

  update(dt: number, grid: Grid, heroPos?: Vec2): ProjectileState {
    if (!this.alive) return 'spent';
    let remaining = this.def.speed * dt;
    // Sub-step so a fast shot cannot skip over a wall block.
    while (remaining > 0) {
      const s = Math.min(0.25, remaining);
      remaining -= s;
      const v = DIR_VECTORS[this.dir];
      // Tile centers are integers, so wrap x within [-0.5, width - 0.5).
      const w = grid.width;
      const rawX = this.pos.x + v.x * s + 0.5;
      this.pos = { x: ((rawX % w) + w) % w - 0.5, y: this.pos.y + v.y * s };
      this.traveled += s;

      const t = this.tile;
      if (!grid.isWalkable(t.x, t.y, true)) {
        if (this.def.returns && !this.returning) {
          this.turnBack();
          continue;
        }
        this.alive = false;
        return 'wall';
      }
      if (this.returning && heroPos && Grid.distSq(this.pos, heroPos) <= CATCH_RADIUS * CATCH_RADIUS) {
        this.alive = false;
        return 'caught';
      }
      if (this.traveled >= this.def.range) {
        if (this.def.returns && !this.returning) {
          this.turnBack();
          continue;
        }
        this.alive = false;
        return 'spent';
      }
    }
    return 'flying';
  }

  /** Boomerang reversal: fly back the way it came for a little more than it went out. */
  private turnBack(): void {
    this.returning = true;
    this.dir = OPPOSITE[this.dir];
    // Allow the return leg to travel the outbound distance plus a margin.
    const back = this.traveled + 2;
    this.traveled = this.def.range - back;
    this.struck.clear();
  }
}

/** Spawns the projectiles a weapon fires from a position and facing. */
export function spawnShots(kind: RangedKind, from: Vec2, facing: Dir): Projectile[] {
  const def = WEAPONS[kind];
  const dir = facing === 'none' ? 'down' : facing;
  const f = DIR_VECTORS[dir];
  const right = { x: -f.y, y: f.x };
  const shots: Projectile[] = [];
  for (const off of def.lateral) {
    shots.push(new Projectile({ x: from.x + right.x * off, y: from.y + right.y * off }, dir, kind));
  }
  if (def.spread) {
    const sides: Dir[] = dir === 'up' || dir === 'down' ? ['left', 'right'] : ['up', 'down'];
    for (const side of sides) shots.push(new Projectile(from, side, kind));
  }
  return shots;
}
