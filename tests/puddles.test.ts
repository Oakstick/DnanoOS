import { describe, expect, it } from 'vitest';
import { Hero, SLIME_SLOW } from '../src/entities/Hero';
import { Mover } from '../src/entities/Mover';
import { PUDDLE_SECONDS, PuddleField } from '../src/systems/Puddles';
import { Grid } from '../src/world/Grid';
import { Tile } from '../src/world/types';

function corridor(): Grid {
  const R = Tile.Rock;
  const O = Tile.Open;
  return new Grid(8, 3, [R, R, R, R, R, R, R, R, R, O, O, O, O, O, O, R, R, R, R, R, R, R, R, R]);
}

describe('slime puddles', () => {
  it('fade out after their lifetime and refresh when re-marked', () => {
    const f = new PuddleField();
    f.add({ x: 1, y: 1 });
    expect(f.has({ x: 1, y: 1 })).toBe(true);
    f.update(PUDDLE_SECONDS - 1);
    f.add({ x: 1, y: 1 });
    f.update(2);
    expect(f.has({ x: 1, y: 1 })).toBe(true); // refreshed
    f.update(PUDDLE_SECONDS);
    expect(f.has({ x: 1, y: 1 })).toBe(false);
    expect(f.count).toBe(0);
  });

  it('slows the hero for a while', () => {
    const h = new Hero({ x: 1, y: 1 }, 5);
    const g = corridor();
    h.slow();
    h.update(1 / 60, g, 'none');
    expect(h.slowed).toBe(true);
    expect(h.speed).toBeCloseTo(5 * SLIME_SLOW);
    for (let i = 0; i < 90; i++) h.update(1 / 60, g, 'none');
    expect(h.slowed).toBe(false);
    expect(h.speed).toBeCloseTo(5);
  });
});

describe('hold-to-move', () => {
  it('a mover asked for nothing rests on the next tile center', () => {
    const g = corridor();
    const m = new Mover({ x: 1, y: 1 }, 2);
    m.step(0.3, g, () => 'right', false, true, true); // 0.6 tiles in
    expect(m.dir).toBe('right');
    m.step(0.5, g, () => 'none', false, true, true); // released: reaches tile 2 and stops there
    expect(m.tile).toEqual({ x: 2, y: 1 });
    expect(m.offset).toBe(0);
    expect(m.dir).toBe('none');
    m.step(1, g, () => 'none', false, true, true);
    expect(m.tile).toEqual({ x: 2, y: 1 }); // stays put
  });

  it('the hero stops when the key is released and resumes when pressed', () => {
    const g = corridor();
    const h = new Hero({ x: 1, y: 1 }, 4);
    for (let i = 0; i < 30; i++) h.update(1 / 60, g, 'right');
    const moving = h.dir;
    for (let i = 0; i < 60; i++) h.update(1 / 60, g, 'none');
    expect(moving).toBe('right');
    expect(h.dir).toBe('none');
    expect(h.offset).toBe(0);
    const rested = h.tile.x;
    for (let i = 0; i < 30; i++) h.update(1 / 60, g, 'right');
    expect(h.pos.x).toBeGreaterThan(rested);
  });
});
