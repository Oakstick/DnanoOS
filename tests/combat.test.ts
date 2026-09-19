import { describe, expect, it } from 'vitest';
import { defaultEnemySpecs, Enemy } from '../src/entities/Enemy';
import { ATTACK_REACH, findAttackTargets } from '../src/systems/Combat';
import { Grid } from '../src/world/Grid';
import { Tile, Vec2 } from '../src/world/types';

function openField(): Grid {
  return new Grid(9, 9, new Array<Tile>(81).fill(Tile.Open));
}

function enemyAt(p: Vec2, mode: Enemy['mode'] = 'chase'): Enemy {
  const spec = defaultEnemySpecs(openField())[0];
  const e = new Enemy(spec, p, 5);
  e.mode = mode;
  return e;
}

describe('findAttackTargets', () => {
  const hero = { x: 4, y: 4 };

  it('hits an enemy directly in front', () => {
    const e = enemyAt({ x: 5, y: 4 });
    expect(findAttackTargets(hero, 'right', [e])).toEqual([e]);
  });

  it('misses an enemy behind or beside beyond the swing width', () => {
    const behind = enemyAt({ x: 3, y: 4 });
    const beside = enemyAt({ x: 4, y: 5 });
    expect(findAttackTargets(hero, 'right', [behind, beside])).toEqual([]);
  });

  it('respects reach', () => {
    const near = enemyAt({ x: 4, y: 4 + ATTACK_REACH - 0.1 });
    const far = enemyAt({ x: 4, y: 4 + ATTACK_REACH + 0.3 });
    expect(findAttackTargets(hero, 'down', [near, far])).toEqual([near]);
  });

  it('hits stunned enemies but never ones in the crypt or already slain', () => {
    const stunned = enemyAt({ x: 4, y: 3 }, 'frenzied');
    const slain = enemyAt({ x: 4, y: 3 }, 'eaten');
    const nesting = enemyAt({ x: 4, y: 3 }, 'nest');
    expect(findAttackTargets(hero, 'up', [stunned, slain, nesting])).toEqual([stunned]);
  });

  it('can hit several enemies in one swing', () => {
    const a = enemyAt({ x: 5, y: 3.6 });
    const b = enemyAt({ x: 5, y: 4.4 });
    expect(findAttackTargets(hero, 'right', [a, b])).toHaveLength(2);
  });
});
