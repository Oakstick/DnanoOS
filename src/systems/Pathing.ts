import { Grid } from '../world/Grid';
import { CARDINALS, Vec2 } from '../world/types';

/**
 * Every tile a walker can reach from `start`. With `canJump`, jump landings
 * count as moves too, which is how the hero crosses pits and climbs ledges.
 * Used by tests to prove every pickup is reachable, and handy for tooling.
 */
export function reachable(grid: Grid, start: Vec2, canJump: boolean, allowNest = false): Set<string> {
  const key = (p: Vec2) => `${p.x},${p.y}`;
  const seen = new Set<string>([key(start)]);
  const queue: Vec2[] = [start];
  const push = (p: Vec2) => {
    const k = key(p);
    if (!seen.has(k)) {
      seen.add(k);
      queue.push(p);
    }
  };
  while (queue.length) {
    const p = queue.shift()!;
    for (const d of CARDINALS) {
      if (grid.canMove(p, d, allowNest)) push(grid.neighbor(p, d));
      if (canJump) {
        const land = grid.jumpLanding(p, d);
        if (land) push(land);
      }
    }
  }
  return seen;
}
