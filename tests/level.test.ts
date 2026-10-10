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


  it('the authored dungeon is four mirrored quadrants, 27 x 23 in all', () => {
    const { rows, heights } = composeDungeon();
    expect(rows[0].length).toBe(27);
    expect(rows).toHaveLength(23);
    expect(heights).toHaveLength(23);
    const text = rows.join('');
    expect(text.split('P')).toHaveLength(2); // exactly one spawn
    expect(text.split('G')).toHaveLength(2); // exactly one hollow exit
    expect(text.split('W')).toHaveLength(3); // two weapon shrines
    expect(text.split('I')).toHaveLength(3); // two item shrines
    expect(text.split('-')).toHaveLength(5); // four outer tunnel mouths, none in the middle
  });

  it('keeps corridors one tile wide, widening only inside the hollow chambers', () => {
    const level = parseLevel(LEVELS[0]);
    const g = level.grid;
    expect(WORLD_SCALE).toBe(1);
    expect(g.width).toBe(27);
    expect(g.height).toBe(23);
    // The hollow and its three mirrored clearings are rooms, not corridors.
    const inChamber = (x: number, y: number) => (x >= 6 && x <= 8 || x >= 18 && x <= 20)
      && (y >= 8 && y <= 9 || y >= 13 && y <= 14);
    const roomy = (x: number, y: number) => {
      const t = g.get(x, y);
      return t !== Tile.Rock && t !== Tile.Pit;
    };
    const wide: string[] = [];
    g.forEach((x, y) => {
      if (!(roomy(x, y) && roomy(x + 1, y) && roomy(x, y + 1) && roomy(x + 1, y + 1))) return;
      for (const [cx, cy] of [[x, y], [x + 1, y], [x, y + 1], [x + 1, y + 1]]) {
        if (!inChamber(cx, cy)) wide.push(`${x},${y}`);
      }
    });
    expect(wide).toEqual([]);
    expect(g.count(Tile.Nodule)).toBe(NUGGETS_PER_LEVEL);
    expect(g.count(Tile.Charge)).toBe(16);
  });

  it('cuts each pit as a single tile spanning its corridor', () => {
    const level = parseLevel(LEVELS[0]);
    const g = level.grid;
    expect(g.count(Tile.Pit)).toBe(PITS_PER_LEVEL * WORLD_SCALE);
    const pits: Vec2[] = [];
    g.forEach((x, y, t) => {
      if (t === Tile.Pit) pits.push({ x, y });
    });
    for (const p of pits) {
      // Floor on the two ends of the corridor, rock on the two sides.
      const across = g.isWalkable(p.x - 1, p.y) && g.isWalkable(p.x + 1, p.y)
        && g.get(p.x, p.y - 1) === Tile.Rock && g.get(p.x, p.y + 1) === Tile.Rock;
      const along = g.isWalkable(p.x, p.y - 1) && g.isWalkable(p.x, p.y + 1)
        && g.get(p.x - 1, p.y) === Tile.Rock && g.get(p.x + 1, p.y) === Tile.Rock;
      expect(across || along, `pit ${p.x},${p.y}`).toBe(true);
    }
  });

  it('quadrants rise clockwise with ramps at some gates and ledges at the rest', () => {
    const level = parseLevel(LEVELS[0]);
    const g = level.grid;
    expect(g.elevation(3, 1)).toBe(0); // top-left, the hollow's level
    expect(g.elevation(23, 1)).toBe(1); // top-right
    expect(g.elevation(3, 21)).toBe(1); // bottom-left
    expect(g.elevation(23, 21)).toBe(2); // bottom-right
    const seam = 13;
    expect(g.isRamp(seam, 4)).toBe(true); // the tunnel row gate carries a ramp
    expect(g.elevation(seam, 4)).toBe(0); // a gate tile settles to the lower floor
    expect(g.isRamp(seam, 1)).toBe(false); // a ledge gate
    // From the ledge gate you can jump up but not walk up.
    expect(g.canMove({ x: seam, y: 1 }, 'right')).toBe(false);
    expect(g.jumpLanding({ x: seam, y: 1 }, 'right')).toEqual({ x: seam + 2, y: 1 });
    expect(g.canMove({ x: seam + 1, y: 1 }, 'left')).toBe(true); // walking down
    expect(g.canMove({ x: seam, y: 4 }, 'right')).toBe(true); // the ramp joins its two levels
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
    for (const probe of [{ x: 23, y: 1 }, { x: 3, y: 21 }, { x: 23, y: 21 }]) {
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
    const quadrants = new Set(spots.map((s) => (s.x < 13 ? 0 : 1) + (s.y < 11 ? 0 : 2)));
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
