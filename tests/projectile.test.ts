import { describe, expect, it } from 'vitest';
import { defaultEnemySpecs, Enemy } from '../src/entities/Enemy';
import { Projectile, spawnShots } from '../src/entities/Projectile';
import { WEAPON_ORDER, WEAPONS } from '../src/entities/Weapons';
import { findProjectileHits } from '../src/systems/Combat';
import { Grid } from '../src/world/Grid';
import { parseLevel } from '../src/world/Level';
import { LEVELS } from '../src/world/levels';
import { Tile, Vec2 } from '../src/world/types';

function corridor(): Grid {
  const R = Tile.Rock;
  const O = Tile.Open;
  return new Grid(8, 3, [R, R, R, R, R, R, R, R, R, O, O, O, O, O, O, R, R, R, R, R, R, R, R, R]);
}

function open(): Grid {
  return new Grid(40, 9, new Array<Tile>(360).fill(Tile.Open));
}

function enemyAt(p: Vec2, mode: Enemy['mode'] = 'chase'): Enemy {
  const e = new Enemy(defaultEnemySpecs(corridor())[0], p, 5);
  e.mode = mode;
  return e;
}

describe('weapon definitions', () => {
  it('offers six weapons in cycle order with sensible stats', () => {
    expect(WEAPON_ORDER).toHaveLength(6);
    for (const kind of WEAPON_ORDER) {
      const d = WEAPONS[kind];
      expect(d.ammo).toBeGreaterThan(0);
      expect(d.speed).toBeGreaterThan(0);
      expect(d.range).toBeGreaterThan(0);
      expect(d.lateral.length).toBeGreaterThan(0);
    }
  });
});

describe('Projectile', () => {
  it('flies straight along its direction', () => {
    const a = new Projectile({ x: 1, y: 1 }, 'right');
    expect(a.update(0.1, corridor())).toBe('flying');
    expect(a.pos.x).toBeCloseTo(1 + WEAPONS.arrow.speed * 0.1);
    expect(a.pos.y).toBe(1);
  });

  it('breaks on the first wall and stops there', () => {
    const a = new Projectile({ x: 1, y: 1 }, 'right');
    expect(a.update(1, corridor())).toBe('wall');
    expect(a.alive).toBe(false);
    expect(a.tile.x).toBe(7);
    expect(a.update(1, corridor())).toBe('spent');
  });

  it('drops after its range in open ground', () => {
    const a = new Projectile({ x: 1, y: 4 }, 'right');
    expect(a.update(2, open())).toBe('spent');
    expect(a.traveled).toBeGreaterThanOrEqual(WEAPONS.arrow.range);
  });

  it('passes through the wrap tunnel of the real level', () => {
    const level = parseLevel(LEVELS[0]);
    const a = new Projectile({ x: 1, y: 31 }, 'left'); // the tunnel row, scaled
    a.update(0.15, level.grid);
    expect(a.alive).toBe(true);
    expect(a.pos.x).toBeGreaterThan(level.grid.width - 3);
  });

  it('boomerang turns back at range and is caught at the hero', () => {
    const hero = { x: 5, y: 4 };
    const b = new Projectile(hero, 'right', 'boomerang');
    let state = b.update(0.4, open(), hero); // ~4 tiles out
    expect(state).toBe('flying');
    expect(b.returning).toBe(false);
    state = b.update(0.2, open(), hero); // past range 5: turns
    expect(b.returning).toBe(true);
    expect(b.dir).toBe('left');
    for (let i = 0; i < 20 && b.alive; i++) state = b.update(0.05, open(), hero);
    expect(state).toBe('caught');
  });

  it('boomerang turns back early when it meets a wall', () => {
    const b = new Projectile({ x: 5, y: 1 }, 'right', 'boomerang');
    b.update(0.3, corridor()); // no hero nearby to catch it
    expect(b.alive).toBe(true);
    expect(b.returning).toBe(true);
    expect(b.dir).toBe('left');
    expect(b.pos.x).toBeLessThan(6.5);
  });

  it('a returning boomerang is caught by a hero standing in its path', () => {
    const hero = { x: 5, y: 1 };
    const b = new Projectile(hero, 'right', 'boomerang');
    expect(b.update(0.3, corridor(), hero)).toBe('caught');
    expect(b.alive).toBe(false);
  });
});

describe('spawnShots', () => {
  it('single weapons fire one shot along the facing', () => {
    const shots = spawnShots('arrow', { x: 3, y: 3 }, 'up');
    expect(shots).toHaveLength(1);
    expect(shots[0].dir).toBe('up');
  });

  it('double arrow fires two parallel shots', () => {
    const shots = spawnShots('double', { x: 3, y: 3 }, 'up');
    expect(shots).toHaveLength(2);
    expect(shots.map((s) => s.pos.x).sort()).toEqual([3 - 0.28, 3 + 0.28]);
    expect(shots.every((s) => s.dir === 'up')).toBe(true);
  });

  it('tri-shot fires forward and to both sides', () => {
    const shots = spawnShots('trishot', { x: 3, y: 3 }, 'right');
    expect(shots.map((s) => s.dir).sort()).toEqual(['down', 'right', 'up']);
  });
});

describe('findProjectileHits', () => {
  it('returns targetable enemies nearest first and skips those already struck', () => {
    const near = enemyAt({ x: 3.3, y: 1 });
    const far = enemyAt({ x: 3.5, y: 1 });
    expect(findProjectileHits({ x: 3, y: 1 }, 0.6, [far, near])).toEqual([near, far]);
    expect(findProjectileHits({ x: 3, y: 1 }, 0.6, [far, near], new Set([near]))).toEqual([far]);
  });

  it('ignores enemies out of reach, in the crypt, or already slain', () => {
    const away = enemyAt({ x: 4, y: 1 });
    const nesting = enemyAt({ x: 3, y: 1 }, 'nest');
    const slain = enemyAt({ x: 3, y: 1 }, 'eaten');
    expect(findProjectileHits({ x: 3, y: 1 }, 0.5, [away, nesting, slain])).toEqual([]);
  });
});
