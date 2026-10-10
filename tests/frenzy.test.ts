import { describe, expect, it } from 'vitest';
import { Frenzy } from '../src/systems/Frenzy';

describe('Frenzy', () => {
  it('is inactive until started', () => {
    const f = new Frenzy();
    expect(f.active).toBe(false);
    expect(f.update(1)).toBe(false);
  });

  it('counts down and reports the tick it ends', () => {
    const f = new Frenzy();
    f.start(3);
    expect(f.active).toBe(true);
    expect(f.update(1)).toBe(false);
    expect(f.fraction).toBeCloseTo(2 / 3);
    expect(f.update(1)).toBe(false);
    expect(f.warning).toBe(true);
    expect(f.update(1.5)).toBe(true);
    expect(f.active).toBe(false);
    expect(f.update(1)).toBe(false);
  });

  it('doubles the consume value along a chain and resets on start', () => {
    const f = new Frenzy();
    f.start(5);
    expect([f.consumeValue(), f.consumeValue(), f.consumeValue(), f.consumeValue()]).toEqual([200, 400, 800, 1600]);
    f.start(5);
    expect(f.consumeValue()).toBe(200);
  });
});
