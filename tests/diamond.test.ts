import { describe, expect, it } from 'vitest';
import { Diamond, DIAMOND_CYCLE_SECONDS, DIAMOND_RESPAWN_SECONDS } from '../src/entities/Diamond';
import { ITEM_ORDER, WEAPON_ORDER } from '../src/entities/Weapons';
import { parseLevel } from '../src/world/Level';
import { LEVELS } from '../src/world/levels';
import { Tile } from '../src/world/types';

describe('Diamond shrine', () => {
  it('cycles on its own over time and wraps around', () => {
    const d = new Diamond('weapon', { x: 1, y: 1 }, WEAPON_ORDER);
    expect(d.current).toBe('arrow');
    d.update(DIAMOND_CYCLE_SECONDS + 0.01);
    expect(d.current).toBe('double');
    for (let i = 0; i < 5; i++) d.update(DIAMOND_CYCLE_SECONDS);
    expect(d.current).toBe('arrow');
  });

  it('advances immediately when struck', () => {
    const d = new Diamond('item', { x: 1, y: 1 }, ITEM_ORDER);
    expect(d.current).toBe('shield');
    d.cycle();
    expect(d.current).toBe('invincible');
    d.cycle();
    d.cycle();
    expect(d.current).toBe('shield');
  });

  it('goes dormant when taken and returns after the respawn delay', () => {
    const d = new Diamond('weapon', { x: 1, y: 1 }, WEAPON_ORDER, 4);
    expect(d.take()).toBe('fireball');
    expect(d.active).toBe(false);
    d.cycle();
    expect(d.current).toBe('fireball'); // dormant shrines do not cycle
    d.update(DIAMOND_RESPAWN_SECONDS - 0.5);
    expect(d.active).toBe(false);
    d.update(1);
    expect(d.active).toBe(true);
  });

  it('the first grove has two weapon shrines and two item shrines on open floor', () => {
    const level = parseLevel(LEVELS[0]);
    expect(level.weaponShrines).toHaveLength(2);
    expect(level.itemShrines).toHaveLength(2);
    for (const t of [...level.weaponShrines, ...level.itemShrines]) {
      expect(level.grid.get(t.x, t.y)).toBe(Tile.Open);
    }
  });
});
