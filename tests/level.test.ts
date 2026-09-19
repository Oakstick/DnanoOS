import { describe, expect, it } from 'vitest';
import { reachable } from '../src/systems/Pathing';
import { parseLevel } from '../src/world/Level';
import {
  composeDungeon, LEVELS, levelFor, NUGGETS_PER_LEVEL, PITS_PER_LEVEL, placeNuggets, placePits, SPEED_SCALE, upscale,
  WORLD_SCALE,
} from '../src/world/levels';
import { Tile, Vec2 } from '../src/world/types';

describe('level data', () => {
  it('every authored level parses with consistent row widths', () => {
    for (const data of LEVELS) {
      const level = parseLevel(data);
      expect(level.grid.width).toBe(data.rows[0].length);
      expect(level.grid.height).toBe(data.rows.length);
      expect(data.heights).toHaveLength(data.rows.length);
    }
  });

  it('the authored dungeon is four quadrants: double the width and height of the base', () => {
    const { rows, heights } = composeDungeon();
    expect(rows[0].length).toBe(53);
    expect(rows).toHaveLength(45);
    expect(heights).toHaveLength(45);
    const text = rows.join('');
    expect(text.split('P')).toHaveLength(2); // exactly one spawn
    expect(text.split('G')).toHaveLength(2); // exactly one hollow exit
    expect(text.split('W')).toHaveLength(3); // two weapon shrines
    expect(text.split('I')).toHaveLength(3); // two item shrines
    expect(text.split('-')).toHaveLength(5); // four outer tunnel mouths, none in the middle
  });

  it('upscales to roads three characters wide', () => {
    const level = parseLevel(LEVELS[0]);
    const g = level.grid;
    expect(WORLD_SCALE).toBe(3);
    expect(g.width).toBe(53 * 3);
    expect(g.height).toBe(45 * 3);
    // Every open tile lies in a run of at least three open tiles both across and along.
    let narrow = 0;
    g.forEach((x, y, t) => {
      if (t === Tile.Rock || t === Tile.Pit) return;
      const openish = (xx: number, yy: number) => g.get(xx, yy) !== Tile.Rock;
      const acrossX = [-2, -1, 0].some((s) => openish(x + s, y) && openish(x + s + 1, y) && openish(x + s + 2, y));
      const acrossY = [-2, -1, 0].some((s) => openish(x, y + s) && openish(x, y + s + 1) && openish(x, y + s + 2));
      if (!(acrossX && acrossY)) narrow++;
    });
    expect(narrow).toBe(0);
    // Single-tile features stay single: one spawn, 25 nuggets, 16 crystals.
    expect(g.count(Tile.Nodule)).toBe(NUGGETS_PER_LEVEL);
    expect(g.count(Tile.Charge)).toBe(16);
  });

  it('pits become one-tile trenches across the full road', () => {
    const level = parseLevel(LEVELS[0]);
    const g = level.grid;
    expect(g.count(Tile.Pit)).toBe(PITS_PER_LEVEL * WORLD_SCALE);
    // Each trench: three pits in a line, with open floor on both sides along the road.
    const pits: Vec2[] = [];
    g.forEach((x, y, t) => {
      if (t === Tile.Pit) pits.push({ x, y });
    });
    for (const p of pits) {
      const vertical = g.isPit(p.x, p.y - 1) || g.isPit(p.x, p.y + 1);
      const horizontal = g.isPit(p.x - 1, p.y) || g.isPit(p.x + 1, p.y);
      expect(vertical || horizontal).toBe(true);
      if (vertical) expect(g.isWalkable(p.x - 1, p.y) && g.isWalkable(p.x + 1, p.y)).toBe(true);
      else expect(g.isWalkable(p.x, p.y - 1) && g.isWalkable(p.x, p.y + 1)).toBe(true);
    }
  });

  it('quadrants rise clockwise with ramps at some gates and ledges at the rest', () => {
    const level = parseLevel(LEVELS[0]);
    const g = level.grid;
    const s = WORLD_SCALE;
    expect(g.elevation(5 * s, 5 * s)).toBe(0); // top-left, the hollow's level
    expect(g.elevation(47 * s, 5 * s)).toBe(1); // top-right
    expect(g.elevation(5 * s, 40 * s)).toBe(1); // bottom-left
    expect(g.elevation(47 * s, 40 * s)).toBe(2); // bottom-right
    const gateX = 26 * s + 1;
    expect(g.isRamp(gateX, 4 * s + 1)).toBe(true); // a ramp gate, its center tile
    expect(g.elevation(gateX, 4 * s + 1)).toBe(0); // ramp block settles to the lower floor even inside
    expect(g.isRamp(gateX, 1 * s + 1)).toBe(false); // a ledge gate
    // From the ledge gate's far edge you can jump up but not walk up.
    const ledgeEdge = { x: 26 * s + 2, y: 1 * s + 1 };
    expect(g.canMove(ledgeEdge, 'right')).toBe(false);
    expect(g.jumpLanding(ledgeEdge, 'right')).toEqual({ x: ledgeEdge.x + 2, y: ledgeEdge.y });
    expect(g.canMove({ x: ledgeEdge.x + 1, y: ledgeEdge.y }, 'left')).toBe(true); // walking down
    // The ramp joins its two levels on foot.
    expect(g.canMove({ x: 26 * s + 2, y: 4 * s + 1 }, 'right')).toBe(true);
  });

  it('cuts pits into straight corridors of the authored grid with level banks', () => {
    const layout = placePits(composeDungeon(), PITS_PER_LEVEL, 1);
    const pits: Vec2[] = [];
    layout.rows.forEach((row, y) => [...row].forEach((c, x) => {
      if (c === '~' || c === '^') pits.push({ x, y });
    }));
    expect(pits).toHaveLength(PITS_PER_LEVEL);
    for (const p of pits) {
      const r = layout.rows;
      const horizontal = r[p.y][p.x] === '~' && r[p.y][p.x - 1] !== '#' && r[p.y][p.x + 1] !== '#' && r[p.y - 1][p.x] === '#' && r[p.y + 1][p.x] === '#';
      const vertical = r[p.y][p.x] === '^' && r[p.y - 1][p.x] !== '#' && r[p.y + 1][p.x] !== '#' && r[p.y][p.x - 1] === '#' && r[p.y][p.x + 1] === '#';
      expect(horizontal || vertical).toBe(true);
    }
    expect(placePits(composeDungeon(), PITS_PER_LEVEL, 1)).toEqual(layout); // deterministic
    expect(placePits(composeDungeon(), PITS_PER_LEVEL, 2)).not.toEqual(layout);
  });

  it('upscale keeps features single and fills blocks', () => {
    const small = { rows: ['#.#', ' ~ ', 'N^-'], heights: ['011', '0/1', '222'] };
    const big = upscale(small, 3);
    expect(big.rows).toEqual([
      '###   ###',
      '### . ###',
      '###   ###',
      '    ~    ',
      '    ~    ',
      '    ~    ',
      'NNN   ---',
      'NNN~~~---',
      'NNN   ---',
    ]);
    expect(big.heights[4]).toBe('000///111');
  });

  it('every nugget, crystal, potion and shrine is reachable with jumps from the spawn', () => {
    for (const data of [LEVELS[0], LEVELS[1], levelFor(9)]) {
      const level = parseLevel(data);
      const seen = reachable(level.grid, level.heroSpawn, true);
      const unreachable: string[] = [];
      level.grid.forEach((x, y, t) => {
        if ((t === Tile.Nodule || t === Tile.Charge) && !seen.has(`${x},${y}`)) unreachable.push(`${x},${y}`);
      });
      for (const p of level.potions) if (!seen.has(`${p.tile.x},${p.tile.y}`)) unreachable.push(`potion ${p.tile.x},${p.tile.y}`);
      for (const s of [...level.weaponShrines, ...level.itemShrines]) if (!seen.has(`${s.x},${s.y}`)) unreachable.push(`shrine ${s.x},${s.y}`);
      expect(unreachable, data.name).toEqual([]);
    }
  });

  it('ground walkers can still reach every quadrant through the ramps', () => {
    const level = parseLevel(LEVELS[0]);
    const seen = reachable(level.grid, level.enemySpawn, false);
    const s = WORLD_SCALE;
    for (const probe of [{ x: 47 * s + 1, y: 1 * s + 1 }, { x: 1 * s + 1, y: 43 * s + 1 }, { x: 47 * s + 1, y: 43 * s + 1 }]) {
      expect(seen.has(`${probe.x},${probe.y}`), `walker reaches ${probe.x},${probe.y}`).toBe(true);
    }
  });

  it('spreads nuggets across the dungeon and differs per level', () => {
    const { rows } = composeDungeon();
    const a = placeNuggets(rows, NUGGETS_PER_LEVEL, 1);
    const b = placeNuggets(rows, NUGGETS_PER_LEVEL, 2);
    expect(a).toEqual(placeNuggets(rows, NUGGETS_PER_LEVEL, 1));
    expect(a).not.toEqual(b);
    const spots: Vec2[] = [];
    a.forEach((row, y) => [...row].forEach((c, x) => {
      if (c === '.') spots.push({ x, y });
    }));
    expect(spots).toHaveLength(NUGGETS_PER_LEVEL);
    let closest = Infinity;
    for (let i = 0; i < spots.length; i++) {
      for (let j = i + 1; j < spots.length; j++) {
        closest = Math.min(closest, Math.abs(spots[i].x - spots[j].x) + Math.abs(spots[i].y - spots[j].y));
      }
    }
    expect(closest).toBeGreaterThanOrEqual(4);
    const quadrants = new Set(spots.map((s) => (s.x < 26 ? 0 : 1) + (s.y < 22 ? 0 : 2)));
    expect(quadrants.size).toBe(4);
  });

  it('later dungeons get their own layout', () => {
    expect(levelFor(7).rows).not.toEqual(levelFor(8).rows);
    expect(parseLevel(levelFor(7)).grid.count(Tile.Nodule)).toBe(NUGGETS_PER_LEVEL);
  });

  it('hollow exit sits just above the hollow with open floor between', () => {
    const level = parseLevel(LEVELS[0]);
    const { x, y } = level.enemySpawn;
    const g = level.grid;
    // Walking down from the exit reaches the hollow within one authored cell.
    let steps = 0;
    while (g.get(x, y + steps) !== Tile.Nest && steps <= WORLD_SCALE) {
      expect(g.isWalkable(x, y + steps, true)).toBe(true);
      steps++;
    }
    expect(g.get(x, y + steps)).toBe(Tile.Nest);
  });

  it('runs everyone at the scaled speed', () => {
    expect(SPEED_SCALE).toBe(0.8);
    expect(LEVELS[0].heroSpeed).toBeCloseTo(6.5 * 0.8);
    expect(LEVELS[0].enemySpeed).toBeCloseTo(5.4 * 0.8);
  });

  it('ramps difficulty beyond authored levels without exceeding caps', () => {
    const far = levelFor(20);
    expect(far.enemySpeed).toBeLessThanOrEqual(9 * SPEED_SCALE);
    expect(far.frenzySeconds).toBeGreaterThanOrEqual(2);
    expect(far.name).toContain('Dungeon 21');
  });
});
