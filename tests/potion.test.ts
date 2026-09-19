import { describe, expect, it } from 'vitest';
import { Rng } from '../src/engine/Rng';
import { defaultEnemySpecs, Enemy, EnemyContext } from '../src/entities/Enemy';
import { Hero } from '../src/entities/Hero';
import { INVISIBLE_SECONDS, Potion, POTIONS } from '../src/entities/Potion';
import { MAX_LIVES, Score } from '../src/systems/Score';
import { Grid } from '../src/world/Grid';
import { parseLevel } from '../src/world/Level';
import { composeDungeon, LEVELS, placePotions, POTION_PLAN } from '../src/world/levels';
import { Tile } from '../src/world/types';

describe('potions in the grove', () => {
  it('each grove holds the planned bottles on open floor, apart from nuggets', () => {
    const level = parseLevel(LEVELS[0]);
    expect(level.potions.map((p) => p.kind).sort()).toEqual([...POTION_PLAN].sort());
    for (const p of level.potions) {
      expect(level.grid.get(p.tile.x, p.tile.y)).toBe(Tile.Open);
    }
    const nuggets: { x: number; y: number }[] = [];
    level.grid.forEach((x, y, t) => {
      if (t === Tile.Nodule) nuggets.push({ x, y });
    });
    for (const p of level.potions) {
      for (const n of nuggets) expect(Math.abs(n.x - p.tile.x) + Math.abs(n.y - p.tile.y)).toBeGreaterThan(0);
    }
  });

  it('placement is deterministic and differs per seed', () => {
    const rows = composeDungeon().rows;
    expect(placePotions(rows, POTION_PLAN, 1)).toEqual(placePotions(rows, POTION_PLAN, 1));
    expect(placePotions(rows, POTION_PLAN, 1)).not.toEqual(placePotions(rows, POTION_PLAN, 2));
  });

  it('definitions cover every kind with a name and color', () => {
    for (const kind of ['life', 'shield', 'invisible'] as const) {
      expect(POTIONS[kind].name.length).toBeGreaterThan(0);
      expect(POTIONS[kind].color).toMatch(/^#/);
    }
    const p = new Potion({ x: 1, y: 1 }, 'life');
    expect(p.def.name).toBe('Extra Life');
    expect(p.taken).toBe(false);
  });
});

describe('lives', () => {
  it('start at three and never exceed three', () => {
    const s = new Score();
    expect(s.lives).toBe(3);
    expect(MAX_LIVES).toBe(3);
    expect(s.gainLife()).toBe(false);
    s.loseLife();
    expect(s.gainLife()).toBe(true);
    expect(s.lives).toBe(3);
    s.reset();
    expect(s.lives).toBe(3);
  });

  it('a score bonus life is also capped', () => {
    const s = new Score();
    expect(s.add(10_000)).toBe(false);
    expect(s.lives).toBe(3);
    s.loseLife();
    expect(s.add(10_000)).toBe(true);
    expect(s.lives).toBe(3);
  });
});

describe('invisibility', () => {
  function grid(): Grid {
    return new Grid(9, 3, new Array<Tile>(27).fill(Tile.Open));
  }

  it('runs out and stops footsteps while active', () => {
    const h = new Hero({ x: 1, y: 1 }, 5);
    h.grantInvisibility(INVISIBLE_SECONDS);
    expect(h.invisible).toBe(true);
    const g = grid();
    const heardBefore = { ...h.heardPos };
    for (let i = 0; i < 30; i++) h.update(1 / 60, g, 'right');
    expect(h.heardPos).toEqual(heardBefore); // moved, but silently
    expect(h.heardAge).toBeGreaterThan(0);
    for (let i = 0; i < (INVISIBLE_SECONDS + 1) * 60; i++) h.update(1 / 60, g, 'none');
    expect(h.invisible).toBe(false);
  });

  it('makes chasing enemies fall back to their corner', () => {
    const level = parseLevel(LEVELS[0]);
    const hero = new Hero(level.heroSpawn, 5);
    const bat = new Enemy(defaultEnemySpecs(level.grid)[1], level.heroSpawn, 5);
    bat.mode = 'chase';
    const ctx: EnemyContext = {
      grid: level.grid,
      hero,
      skeleton: null,
      nestCenter: level.nestCenter,
      nestExit: level.enemySpawn,
      globalMode: 'chase',
      frenzyActive: false,
      heroHidden: false,
      rng: new Rng(1),
    };
    bat.update(1 / 60, ctx);
    expect(bat.target).not.toEqual(bat.scatterTarget);
    ctx.heroHidden = true;
    bat.update(1 / 60, ctx);
    expect(bat.target).toEqual(bat.scatterTarget);
  });
});
