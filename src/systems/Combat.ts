import { Enemy } from '../entities/Enemy';
import { Dir, DIR_VECTORS, Vec2 } from '../world/types';

/** How far in front of the hero the sword reaches, in tiles. */
export const ATTACK_REACH = 1.2;
/** Half-width of the swing to either side, in tiles. */
export const ATTACK_HALF_WIDTH = 0.7;

export const POINTS_KILL = 100;
/** An arrow connects when its head is this close to an enemy's center. */
export const ARROW_HIT_RADIUS = 0.5;

/** First targetable enemy within arrow reach of the given point, nearest first. */
export function findArrowHit(point: Vec2, enemies: Enemy[]): Enemy | null {
  return findProjectileHits(point, ARROW_HIT_RADIUS, enemies)[0] ?? null;
}

/**
 * Targetable enemies within `radius` of a point, nearest first, skipping any
 * in `exclude` (already struck by a piercing shot).
 */
export function findProjectileHits(point: Vec2, radius: number, enemies: Enemy[], exclude?: Set<Enemy>): Enemy[] {
  const r2 = radius * radius;
  const hits: { e: Enemy; d: number }[] = [];
  for (const e of enemies) {
    if (!e.isTargetable || exclude?.has(e)) continue;
    const ep = e.pos;
    const dx = ep.x - point.x;
    const dy = ep.y - point.y;
    const d = dx * dx + dy * dy;
    if (d <= r2) hits.push({ e, d });
  }
  return hits.sort((a, b) => a.d - b.d).map((h) => h.e);
}

/** Objects on a tile the sword can reach: used to strike shrines. */
export function tileInReach(heroPos: Vec2, facing: Dir, tile: Vec2): boolean {
  const f = DIR_VECTORS[facing === 'none' ? 'down' : facing];
  const dx = tile.x - heroPos.x;
  const dy = tile.y - heroPos.y;
  const along = dx * f.x + dy * f.y;
  const across = Math.abs(dx * f.y - dy * f.x);
  return along >= -0.25 && along <= ATTACK_REACH && across <= ATTACK_HALF_WIDTH;
}

/**
 * Enemies caught by a sword swing: those within reach along the facing
 * direction and within the swing's lateral half-width. Only enemies that can
 * currently be hit are returned.
 */
export function findAttackTargets(heroPos: Vec2, facing: Dir, enemies: Enemy[]): Enemy[] {
  const f = DIR_VECTORS[facing === 'none' ? 'down' : facing];
  const hits: Enemy[] = [];
  for (const e of enemies) {
    if (!e.isTargetable) continue;
    const ep = e.pos;
    const dx = ep.x - heroPos.x;
    const dy = ep.y - heroPos.y;
    const along = dx * f.x + dy * f.y; // distance in front of the hero
    const across = Math.abs(dx * f.y - dy * f.x); // lateral offset
    if (along >= -0.25 && along <= ATTACK_REACH && across <= ATTACK_HALF_WIDTH) hits.push(e);
  }
  return hits;
}
