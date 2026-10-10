import { describe, expect, it } from 'vitest';
import { defaultEnemySpecs, Enemy } from '../src/entities/Enemy';
import { Hero, JUMP_SECONDS } from '../src/entities/Hero';
import { Grid } from '../src/world/Grid';
import { Tile } from '../src/world/types';

/**
 * 9x3: a corridor with a pit at x=3 and a one-level ledge from x=6 on.
 *   x: 0 1 2 3 4 5 6 7 8
 *      # O O ~ O O O O #     heights: 0 0 0 0 0 0 1 1 1
 */
function course(): Grid {
  const R = Tile.Rock;
  const O = Tile.Open;
  const P = Tile.Pit;
  const tiles = [R, R, R, R, R, R, R, R, R, R, O, O, P, O, O, O, O, R, R, R, R, R, R, R, R, R, R];
  const heights = [0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1];
  return new Grid(9, 3, tiles, heights);
}

describe('elevation rules', () => {
  it('walkers cannot enter pits or climb ledges, but may step down', () => {
    const g = course();
    expect(g.canMove({ x: 2, y: 1 }, 'right')).toBe(false); // pit
    expect(g.canMove({ x: 5, y: 1 }, 'right')).toBe(false); // ledge up
    expect(g.canMove({ x: 6, y: 1 }, 'left')).toBe(true); // step down
  });

  it('flyers cross pits and ledges alike', () => {
    const g = course();
    expect(g.canMove({ x: 2, y: 1 }, 'right', false, true)).toBe(true);
    expect(g.canMove({ x: 5, y: 1 }, 'right', false, true)).toBe(true);
  });

  it('a ramp joins two levels on foot', () => {
    const R = Tile.Rock;
    const O = Tile.Open;
    const g = new Grid(4, 1, [O, O, O, O], [0, 0, 1, 1], [false, true, false, false]);
    expect(g.canMove({ x: 1, y: 0 }, 'right')).toBe(true);
    expect(g.canMove({ x: 0, y: 0 }, 'right')).toBe(true);
    expect(R).toBe(Tile.Rock);
  });

  it('jump landings clear a pit and climb one level, or fall back to one tile', () => {
    const g = course();
    expect(g.jumpLanding({ x: 2, y: 1 }, 'right')).toEqual({ x: 4, y: 1 }); // over the pit
    expect(g.jumpLanding({ x: 4, y: 1 }, 'right')).toEqual({ x: 6, y: 1 }); // up onto the ledge
    expect(g.jumpLanding({ x: 6, y: 1 }, 'right')).toEqual({ x: 7, y: 1 }); // two ahead is rock, one is fine
    expect(g.jumpLanding({ x: 7, y: 1 }, 'right')).toBeNull(); // wall directly ahead: nowhere to go
    expect(g.jumpLanding({ x: 1, y: 1 }, 'left')).toBeNull();
  });
});

describe('hero jump', () => {
  it('arcs two tiles forward over a pit and lands aligned', () => {
    const g = course();
    const h = new Hero({ x: 2, y: 1 }, 5);
    h.facing = 'right';
    expect(h.tryJump(g)).toBe(true);
    expect(h.airborne).toBe(true);
    expect(h.tryJump(g)).toBe(false); // no double jump
    let peak = 0;
    for (let i = 0; i < 20; i++) {
      h.update(JUMP_SECONDS / 20, g, 'none');
      peak = Math.max(peak, h.altitude);
    }
    h.update(0.01, g, 'none');
    expect(h.airborne).toBe(false);
    expect(h.tile).toEqual({ x: 4, y: 1 });
    expect(h.offset).toBe(0);
    expect(peak).toBeGreaterThan(0.9);
    expect(h.altitude).toBe(0);
  });

  it('climbs a ledge and reports the new elevation', () => {
    const g = course();
    const h = new Hero({ x: 4, y: 1 }, 5);
    h.facing = 'right';
    h.tryJump(g);
    for (let i = 0; i < 30; i++) h.update(JUMP_SECONDS / 20, g, 'none');
    expect(h.tile).toEqual({ x: 6, y: 1 });
    expect(h.altitude).toBe(1);
  });

  it('hops in place when nothing ahead can be landed on', () => {
    const g = course();
    const h = new Hero({ x: 7, y: 1 }, 5);
    h.facing = 'right';
    h.tryJump(g);
    for (let i = 0; i < 30; i++) h.update(JUMP_SECONDS / 20, g, 'none');
    expect(h.tile).toEqual({ x: 7, y: 1 });
    expect(h.altitude).toBe(1);
  });

  it('cannot walk into the pit on foot', () => {
    const g = course();
    const h = new Hero({ x: 2, y: 1 }, 5);
    for (let i = 0; i < 60; i++) h.update(1 / 60, g, 'right');
    expect(h.tile).toEqual({ x: 2, y: 1 });
  });
});

describe('flying enemies', () => {
  it('bats fly, skeletons and slimes walk', () => {
    const specs = defaultEnemySpecs(course());
    const kinds = Object.fromEntries(specs.map((s) => [s.kind, new Enemy(s, { x: 1, y: 1 }, 5).flying]));
    expect(kinds).toEqual({ skeleton: false, bat: true, vampire: true, slime: false });
  });
});
