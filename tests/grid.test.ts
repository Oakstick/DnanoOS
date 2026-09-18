import { describe, expect, it } from 'vitest';
import { Grid } from '../src/world/Grid';
import { Tile } from '../src/world/types';

function tiny(): Grid {
  // 5x3: tunnel row in the middle, rock border top/bottom.
  const R = Tile.Rock;
  const O = Tile.Open;
  const T = Tile.Tunnel;
  const N = Tile.Nodule;
  const C = Tile.Charge;
  const X = Tile.Nest;
  return new Grid(5, 3, [
    R, R, R, R, R,
    T, N, C, X, T,
    R, R, O, R, R,
  ]);
}

describe('Grid', () => {
  it('rejects mismatched tile arrays', () => {
    expect(() => new Grid(2, 2, [Tile.Open])).toThrow();
  });

  it('treats out-of-bounds as rock', () => {
    const g = tiny();
    expect(g.get(-1, 1)).toBe(Tile.Rock);
    expect(g.get(0, 99)).toBe(Tile.Rock);
  });

  it('wraps horizontally through tunnels', () => {
    const g = tiny();
    expect(g.neighbor({ x: 4, y: 1 }, 'right')).toEqual({ x: 0, y: 1 });
    expect(g.neighbor({ x: 0, y: 1 }, 'left')).toEqual({ x: 4, y: 1 });
    expect(g.canMove({ x: 4, y: 1 }, 'right')).toBe(true);
  });

  it('nest tiles are walkable only when allowed', () => {
    const g = tiny();
    expect(g.canMove({ x: 2, y: 1 }, 'right')).toBe(false);
    expect(g.canMove({ x: 2, y: 1 }, 'right', true)).toBe(true);
  });

  it('consumes collectibles exactly once', () => {
    const g = tiny();
    expect(g.consume(1, 1)).toBe(Tile.Nodule);
    expect(g.consume(1, 1)).toBeNull();
    expect(g.consume(2, 1)).toBe(Tile.Charge);
    expect(g.consume(0, 1)).toBeNull();
    expect(g.count(Tile.Nodule)).toBe(0);
  });

  it('lists open directions', () => {
    const g = tiny();
    expect(g.openDirections({ x: 2, y: 1 }).sort()).toEqual(['down', 'left'].sort());
  });
});
