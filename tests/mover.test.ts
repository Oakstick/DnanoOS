import { describe, expect, it } from 'vitest';
import { Mover } from '../src/entities/Mover';
import { Grid } from '../src/world/Grid';
import { Tile } from '../src/world/types';

function corridor(): Grid {
  // 6x3 open corridor with rock border; no wrap tiles.
  const R = Tile.Rock;
  const O = Tile.Open;
  return new Grid(6, 3, [
    R, R, R, R, R, R,
    R, O, O, O, O, R,
    R, R, R, R, R, R,
  ]);
}

describe('Mover', () => {
  it('advances along a direction and stops at walls', () => {
    const g = corridor();
    const m = new Mover({ x: 1, y: 1 }, 2); // 2 tiles/sec
    for (let i = 0; i < 40; i++) m.step(0.1, g, () => 'right');
    expect(m.tile).toEqual({ x: 4, y: 1 });
    expect(m.offset).toBe(0);
    expect(m.dir).toBe('none');
  });

  it('crosses tile centers at the right times', () => {
    const g = corridor();
    const m = new Mover({ x: 1, y: 1 }, 1);
    expect(m.step(0.5, g, () => 'right')).toBe(false);
    expect(m.pos.x).toBeCloseTo(1.5);
    expect(m.step(0.5, g, () => 'right')).toBe(true);
    expect(m.tile.x).toBe(2);
  });

  it('reports the occupied tile as the nearest center', () => {
    const g = corridor();
    const m = new Mover({ x: 1, y: 1 }, 1);
    m.step(0.4, g, () => 'right');
    expect(m.occupied).toEqual({ x: 1, y: 1 });
    m.step(0.2, g, () => 'right');
    expect(m.occupied).toEqual({ x: 2, y: 1 });
  });

  it('reverses mid-tile without teleporting', () => {
    const g = corridor();
    const m = new Mover({ x: 1, y: 1 }, 1);
    m.step(0.3, g, () => 'right');
    const before = m.pos;
    m.reverse(g);
    expect(m.dir).toBe('left');
    expect(m.pos.x).toBeCloseTo(before.x);
    expect(m.pos.y).toBeCloseTo(before.y);
  });

  it('ignores unwalkable requests and keeps going', () => {
    const g = corridor();
    const m = new Mover({ x: 1, y: 1 }, 1);
    m.step(1, g, () => 'right');
    m.step(1, g, () => 'up'); // wall above; should continue right
    expect(m.tile.x).toBe(3);
    expect(m.dir).toBe('right');
  });
});
