import { describe, expect, it } from 'vitest';
import { CARDINALS, relativeDir } from '../src/world/types';

describe('third-person input mapping', () => {
  it('is the identity when facing grid up', () => {
    for (const d of CARDINALS) expect(relativeDir(d, 'up')).toBe(d);
  });

  it('turns with the facing', () => {
    expect(relativeDir('up', 'right')).toBe('right');
    expect(relativeDir('down', 'right')).toBe('left');
    expect(relativeDir('right', 'right')).toBe('down');
    expect(relativeDir('left', 'right')).toBe('up');
    expect(relativeDir('up', 'down')).toBe('down');
    expect(relativeDir('left', 'down')).toBe('right');
    expect(relativeDir('right', 'left')).toBe('up');
  });

  it('passes none through', () => {
    expect(relativeDir('none', 'left')).toBe('none');
  });
});
