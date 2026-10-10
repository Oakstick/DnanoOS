import { Grid } from '../world/Grid';
import { CARDINALS, Dir, Vec2 } from '../world/types';

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

/**
 * First step of a shortest path from `from` to `goal`, or null when no route
 * exists. A slain enemy uses this to find its way home: the hollow has a
 * single entrance, and walking greedily towards it can circle a block forever
 * without ever turning away from the target to go around.
 */
export function stepToward(grid: Grid, from: Vec2, goal: Vec2, flying: boolean, allowNest = true): Dir | null {
  if (from.x === goal.x && from.y === goal.y) return null;
  const key = (p: Vec2) => `${p.x},${p.y}`;
  // Each entry remembers the direction the search first took to get here.
  const first = new Map<string, Dir>([[key(from), 'none']]);
  const queue: Vec2[] = [from];
  while (queue.length > 0) {
    const p = queue.shift()!;
    const opening = first.get(key(p))!;
    for (const d of CARDINALS) {
      if (!grid.canMove(p, d, allowNest, flying)) continue;
      const n = grid.neighbor(p, d);
      const k = key(n);
      if (first.has(k)) continue;
      const step = opening === 'none' ? d : opening;
      if (n.x === goal.x && n.y === goal.y) return step;
      first.set(k, step);
      queue.push(n);
    }
  }
  return null;
}
