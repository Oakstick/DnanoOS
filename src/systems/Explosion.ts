import { Grid } from '../world/Grid';
import { Tile, Vec2 } from '../world/types';

/** Entities within this distance of the blast center are hit. */
export const BLAST_RADIUS = 3.0;
/** Rock within this distance is destroyed: enough to punch through a three-tile forest wall. */
export const BREAK_RADIUS = 3.2;
/** Chain-ignition delay range for bombs caught in a blast, seconds. */
export const CHAIN_DELAY = { min: 0.15, max: 0.5 };

/** Damage to the hero by distance from the blast center. */
export function blastDamage(distance: number): number {
  if (distance <= BLAST_RADIUS / 2) return 50;
  if (distance <= BLAST_RADIUS) return 25;
  return 0;
}

export function inBlast(center: Vec2, p: Vec2): boolean {
  return Grid.distSq(center, p) <= BLAST_RADIUS * BLAST_RADIUS;
}

/**
 * Rock that must survive: the outer border (keeps the maze closed and the
 * wrap tunnels intact) and any rock touching the enemy crypt.
 */
export function isProtected(grid: Grid, x: number, y: number): boolean {
  if (x <= 0 || y <= 0 || x >= grid.width - 1 || y >= grid.height - 1) return true;
  for (let dy = -1; dy <= 1; dy++) {
    for (let dx = -1; dx <= 1; dx++) {
      if (grid.get(x + dx, y + dy) === Tile.Nest) return true;
    }
  }
  return false;
}

/** Turns unprotected rock near the center into open floor. Returns the tiles changed. */
export function breakRocks(grid: Grid, center: Vec2, radius = BREAK_RADIUS): Vec2[] {
  const changed: Vec2[] = [];
  const r = Math.ceil(radius);
  for (let dy = -r; dy <= r; dy++) {
    for (let dx = -r; dx <= r; dx++) {
      if (dx * dx + dy * dy > radius * radius) continue;
      const x = center.x + dx;
      const y = center.y + dy;
      if (grid.get(x, y) !== Tile.Rock) continue;
      if (isProtected(grid, x, y)) continue;
      grid.set(x, y, Tile.Open);
      changed.push({ x, y });
    }
  }
  return changed;
}
