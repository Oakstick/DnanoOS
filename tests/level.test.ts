import { describe, expect, it } from 'vitest';
import { parseLevel } from '../src/world/Level';
import { LEVELS, levelFor } from '../src/world/levels';
import { Tile, Vec2 } from '../src/world/types';

describe('level data', () => {
  it('every authored level parses with consistent row widths', () => {
    for (const data of LEVELS) {
      const level = parseLevel(data);
      expect(level.grid.width).toBe(data.rows[0].length);
      expect(level.grid.height).toBe(data.rows.length);
    }
  });

  it('all nodules and charges are reachable from the drone spawn', () => {
    const level = parseLevel(LEVELS[0]);
    const { grid, droneSpawn } = level;
    const seen = new Set<string>();
    const queue: Vec2[] = [droneSpawn];
    seen.add(`${droneSpawn.x},${droneSpawn.y}`);
    while (queue.length) {
      const p = queue.shift()!;
      for (const d of ['up', 'down', 'left', 'right'] as const) {
        if (!grid.canMove(p, d)) continue;
        const n = grid.neighbor(p, d);
        const key = `${n.x},${n.y}`;
        if (!seen.has(key)) {
          seen.add(key);
          queue.push(n);
        }
      }
    }
    const unreachable: string[] = [];
    grid.forEach((x, y, t) => {
      if ((t === Tile.Nodule || t === Tile.Charge) && !seen.has(`${x},${y}`)) unreachable.push(`${x},${y}`);
    });
    expect(unreachable).toEqual([]);
  });

  it('has exactly four chem-charges and a sensible nodule count', () => {
    const level = parseLevel(LEVELS[0]);
    expect(level.grid.count(Tile.Charge)).toBe(4);
    expect(level.grid.count(Tile.Nodule)).toBeGreaterThan(100);
  });

  it('nest exit sits directly above the nest', () => {
    const level = parseLevel(LEVELS[0]);
    const below = level.grid.get(level.predatorSpawn.x, level.predatorSpawn.y + 1);
    expect(below).toBe(Tile.Nest);
  });

  it('ramps difficulty beyond authored levels without exceeding caps', () => {
    const far = levelFor(20);
    expect(far.predatorSpeed).toBeLessThanOrEqual(9);
    expect(far.frenzySeconds).toBeGreaterThanOrEqual(2);
    expect(far.name).toContain('Trench 21');
  });
});
