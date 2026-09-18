import { describe, expect, it } from 'vitest';
import { Rng } from '../src/engine/Rng';
import { Drone } from '../src/entities/Drone';
import { defaultPredatorSpecs, Predator, PredatorContext } from '../src/entities/Predator';
import { parseLevel } from '../src/world/Level';
import { LEVELS } from '../src/world/levels';
import { Tile } from '../src/world/types';

function setup() {
  const level = parseLevel(LEVELS[0]);
  const drone = new Drone(level.droneSpawn, 6);
  const specs = defaultPredatorSpecs(level.grid);
  const hunter = new Predator(specs[0], level.predatorSpawn, 5);
  const ctx: PredatorContext = {
    grid: level.grid,
    drone,
    hunter,
    nestCenter: level.nestCenter,
    nestExit: level.predatorSpawn,
    globalMode: 'chase',
    frenzyActive: false,
    rng: new Rng(42),
  };
  return { level, drone, hunter, ctx, specs };
}

describe('Predator', () => {
  it('leaves the nest and starts hunting', () => {
    const { hunter, ctx } = setup();
    for (let i = 0; i < 60; i++) hunter.update(1 / 60, ctx);
    expect(hunter.mode).toBe('chase');
    expect(hunter.isDangerous).toBe(true);
  });

  it('never occupies rock while frenzied over a long random walk', () => {
    const { hunter, ctx } = setup();
    for (let i = 0; i < 60; i++) hunter.update(1 / 60, ctx);
    hunter.enterFrenzy(ctx.grid);
    expect(hunter.isEdible).toBe(true);
    ctx.frenzyActive = true;
    for (let i = 0; i < 60 * 30; i++) {
      hunter.update(1 / 60, ctx);
      const o = hunter.occupied;
      expect(ctx.grid.get(o.x, o.y)).not.toBe(Tile.Rock);
    }
  });

  it('returns to the nest after being eaten, then re-emerges', () => {
    const { hunter, ctx } = setup();
    for (let i = 0; i < 60 * 5; i++) hunter.update(1 / 60, ctx);
    hunter.consume();
    expect(hunter.mode).toBe('eaten');
    let sawLeaving = false;
    for (let i = 0; i < 60 * 30 && hunter.mode !== 'chase'; i++) {
      hunter.update(1 / 60, ctx);
      if (hunter.mode === 'leaving') sawLeaving = true;
    }
    expect(sawLeaving).toBe(true);
    expect(hunter.mode).toBe('chase');
  });

  it('hunter loses the drone once it has been silent long enough', () => {
    const { hunter, ctx, drone } = setup();
    for (let i = 0; i < 60; i++) hunter.update(1 / 60, ctx);
    drone.heardAge = 0;
    hunter.update(1 / 60, ctx);
    expect(hunter.target).toEqual(drone.heardPos);
    drone.heardAge = 10;
    hunter.update(1 / 60, ctx);
    expect(hunter.target).toEqual(hunter.scatterTarget);
  });

  it('predators still in the nest ignore a frenzy', () => {
    const { ctx, specs, level } = setup();
    const lurker = new Predator(specs[3], level.nestCenter, 5);
    lurker.enterFrenzy(ctx.grid);
    expect(lurker.mode).toBe('nest');
  });
});
