import { describe, expect, it } from 'vitest';
import { Bomb, BOMB_FUSE } from '../src/entities/Bomb';
import { blastDamage, BLAST_RADIUS, BREAK_RADIUS, breakRocks, inBlast, isProtected } from '../src/systems/Explosion';
import { parseLevel } from '../src/world/Level';
import { LEVELS } from '../src/world/levels';
import { Grid } from '../src/world/Grid';
import { Tile } from '../src/world/types';

function solid(size = 15): Grid {
  // All rock with a single open tile in the middle.
  const tiles = new Array<Tile>(size * size).fill(Tile.Rock);
  const c = Math.floor(size / 2);
  tiles[c * size + c] = Tile.Open;
  return new Grid(size, size, tiles);
}

describe('Bomb', () => {
  it('burns its fuse and reports the detonation tick', () => {
    const b = new Bomb({ x: 1, y: 1 });
    expect(b.update(BOMB_FUSE / 2)).toBe(false);
    expect(b.progress).toBeCloseTo(0.5);
    expect(b.update(BOMB_FUSE)).toBe(true);
  });

  it('can be ignited early but never lengthened', () => {
    const b = new Bomb({ x: 1, y: 1 });
    b.ignite(0.2);
    expect(b.fuse).toBeCloseTo(0.2);
    b.ignite(5);
    expect(b.fuse).toBeCloseTo(0.2);
  });
});

describe('Explosion', () => {
  it('breaks every unprotected rock within the break radius', () => {
    const g = solid();
    const c = { x: 7, y: 7 };
    const changed = breakRocks(g, c);
    let expected = 0;
    for (let dy = -4; dy <= 4; dy++) {
      for (let dx = -4; dx <= 4; dx++) {
        if (dx === 0 && dy === 0) continue;
        if (dx * dx + dy * dy <= BREAK_RADIUS * BREAK_RADIUS) expected++;
      }
    }
    expect(changed).toHaveLength(expected);
    expect(g.get(7, 4)).toBe(Tile.Open); // three tiles away: through a three-tile wall
    expect(g.get(7, 3)).toBe(Tile.Rock); // four tiles away stays
    expect(g.get(10, 10)).toBe(Tile.Rock); // diagonal 4.2 stays
  });

  it('never breaks the border', () => {
    const g = solid(9);
    breakRocks(g, { x: 2, y: 2 });
    expect(g.get(0, 0)).toBe(Tile.Rock);
    expect(g.get(0, 2)).toBe(Tile.Rock);
    expect(g.get(2, 0)).toBe(Tile.Rock);
    expect(g.get(3, 3)).toBe(Tile.Open);
  });

  it('protects rock touching the hollow in the real level', () => {
    const level = parseLevel(LEVELS[0]);
    const { grid, nestCenter } = level;
    // Find a rock tile bordering the hollow and one far from it.
    let nearHollow: { x: number; y: number } | null = null;
    grid.forEach((x, y, t) => {
      if (nearHollow) return;
      if (t === Tile.Rock && Math.abs(x - nestCenter.x) <= 12 && Math.abs(y - nestCenter.y) <= 5 && isProtected(grid, x, y)) nearHollow = { x, y };
    });
    expect(nearHollow).not.toBeNull();
    // An interior wall block far from the hollow is fair game (authored cell 2,2 is rock).
    expect(grid.get(7, 7)).toBe(Tile.Rock);
    expect(isProtected(grid, 7, 7)).toBe(false);
  });

  it('keeps all nuggets when blasting a wall (only adds connectivity)', () => {
    const level = parseLevel(LEVELS[0]);
    const before = level.grid.count(Tile.Nodule);
    breakRocks(level.grid, { x: 6, y: 4 });
    expect(level.grid.count(Tile.Nodule)).toBe(before);
  });

  it('scales hero damage with distance', () => {
    expect(blastDamage(BLAST_RADIUS * 0.4)).toBe(50);
    expect(blastDamage(BLAST_RADIUS * 0.8)).toBe(25);
    expect(blastDamage(BLAST_RADIUS + 0.1)).toBe(0);
    expect(inBlast({ x: 0, y: 0 }, { x: 2, y: 2 })).toBe(true);
    expect(inBlast({ x: 0, y: 0 }, { x: 3.5, y: 0 })).toBe(false);
  });
});
