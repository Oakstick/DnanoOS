import { describe, expect, it } from 'vitest';
import { Rng } from '../src/engine/Rng';
import { defaultEnemySpecs, Enemy, EnemyContext } from '../src/entities/Enemy';
import { Hero } from '../src/entities/Hero';
import { parseLevel } from '../src/world/Level';
import { LEVELS } from '../src/world/levels';
import { Tile } from '../src/world/types';

function setup() {
  const level = parseLevel(LEVELS[0]);
  const hero = new Hero(level.heroSpawn, 6);
  const specs = defaultEnemySpecs(level.grid);
  const skeleton = new Enemy(specs[0], level.enemySpawn, 5);
  const ctx: EnemyContext = {
    grid: level.grid,
    hero,
    skeleton,
    nestCenter: level.nestCenter,
    nestExit: level.enemySpawn,
    globalMode: 'chase',
    frenzyActive: false,
    rng: new Rng(42),
  };
  return { level, hero, skeleton, ctx, specs };
}

describe('Enemy', () => {
  it('names the Knightmare cast', () => {
    const { specs } = setup();
    expect(specs.map((s) => s.kind)).toEqual(['skeleton', 'bat', 'vampire', 'slime']);
    expect(specs[3].speedFactor).toBeLessThan(1);
    expect(specs[1].speedFactor).toBeGreaterThan(1);
  });

  it('leaves the crypt and starts hunting', () => {
    const { skeleton, ctx } = setup();
    for (let i = 0; i < 60; i++) skeleton.update(1 / 60, ctx);
    expect(skeleton.mode).toBe('chase');
    expect(skeleton.isDangerous).toBe(true);
    expect(skeleton.isTargetable).toBe(true);
  });

  it('never occupies rock while stunned over a long random walk', () => {
    const { skeleton, ctx } = setup();
    for (let i = 0; i < 60; i++) skeleton.update(1 / 60, ctx);
    skeleton.enterFrenzy(ctx.grid);
    expect(skeleton.isStunned).toBe(true);
    expect(skeleton.isDangerous).toBe(false);
    ctx.frenzyActive = true;
    for (let i = 0; i < 60 * 30; i++) {
      skeleton.update(1 / 60, ctx);
      const o = skeleton.occupied;
      expect(ctx.grid.get(o.x, o.y)).not.toBe(Tile.Rock);
    }
  });

  it('non-skeletons return to the crypt after being slain, then re-emerge', () => {
    const { ctx, specs, level } = setup();
    const bat = new Enemy(specs[1], level.enemySpawn, 5);
    for (let i = 0; i < 60 * 6; i++) bat.update(1 / 60, ctx);
    expect(bat.mode).toBe('chase');
    bat.kill();
    expect(bat.mode).toBe('eaten');
    expect(bat.isTargetable).toBe(false);
    let sawLeaving = false;
    for (let i = 0; i < 60 * 30 && bat.mode !== 'chase'; i++) {
      bat.update(1 / 60, ctx);
      if (bat.mode === 'leaving') sawLeaving = true;
    }
    expect(sawLeaving).toBe(true);
    expect(bat.mode).toBe('chase');
  });

  it('a slain skeleton scatters in place and reassembles after three seconds', () => {
    const { skeleton, ctx } = setup();
    for (let i = 0; i < 60 * 3; i++) skeleton.update(1 / 60, ctx);
    const where = { ...skeleton.occupied };
    skeleton.kill();
    expect(skeleton.mode).toBe('scattered');
    expect(skeleton.isTargetable).toBe(false);
    expect(skeleton.isDangerous).toBe(false);
    expect(skeleton.tile).toEqual(where);

    for (let i = 0; i < 60 * 2.5; i++) skeleton.update(1 / 60, ctx);
    expect(skeleton.mode).toBe('scattered');
    expect(skeleton.tile).toEqual(where);
    expect(skeleton.scatterProgress).toBeGreaterThan(0.8);

    for (let i = 0; i < 60 * 0.6; i++) skeleton.update(1 / 60, ctx);
    expect(skeleton.mode).toBe('chase');
    expect(skeleton.isDangerous).toBe(true);
    expect(skeleton.scatterProgress).toBe(0);
  });

  it('a skeleton reassembling during a stun comes back stunned', () => {
    const { skeleton, ctx } = setup();
    for (let i = 0; i < 60 * 3; i++) skeleton.update(1 / 60, ctx);
    skeleton.kill();
    ctx.frenzyActive = true;
    for (let i = 0; i < 60 * 3.2; i++) skeleton.update(1 / 60, ctx);
    expect(skeleton.mode).toBe('frenzied');
  });

  it('skeleton loses the hero once the footsteps have been silent long enough', () => {
    const { skeleton, ctx, hero } = setup();
    for (let i = 0; i < 60; i++) skeleton.update(1 / 60, ctx);
    hero.heardAge = 0;
    skeleton.update(1 / 60, ctx);
    expect(skeleton.target).toEqual(hero.heardPos);
    hero.heardAge = 10;
    skeleton.update(1 / 60, ctx);
    expect(skeleton.target).toEqual(skeleton.scatterTarget);
  });

  it('enemies still in the crypt ignore a power crystal', () => {
    const { ctx, specs, level } = setup();
    const slime = new Enemy(specs[3], level.nestCenter, 5);
    slime.enterFrenzy(ctx.grid);
    expect(slime.mode).toBe('nest');
  });
});
