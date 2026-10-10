export enum Tile {
  Rock = 0,
  Open = 1,
  Nodule = 2,
  Charge = 3,
  Nest = 4,
  Tunnel = 5,
  /** A hole in the floor: impassable on foot, jumpable, flyable. */
  Pit = 6,
}

export type Dir = 'up' | 'down' | 'left' | 'right' | 'none';

export interface Vec2 {
  x: number;
  y: number;
}

export const DIR_VECTORS: Record<Dir, Vec2> = {
  up: { x: 0, y: -1 },
  down: { x: 0, y: 1 },
  left: { x: -1, y: 0 },
  right: { x: 1, y: 0 },
  none: { x: 0, y: 0 },
};

export const OPPOSITE: Record<Dir, Dir> = {
  up: 'down',
  down: 'up',
  left: 'right',
  right: 'left',
  none: 'none',
};

export const CARDINALS: Dir[] = ['up', 'left', 'down', 'right'];

/**
 * A screen-relative input (up = forward) mapped to a grid direction given
 * the direction the camera and hero face. Used by the third-person camera.
 */
export function relativeDir(input: Dir, forward: Dir): Dir {
  if (input === 'none') return 'none';
  const f = DIR_VECTORS[forward === 'none' ? 'down' : forward];
  const right = { x: -f.y, y: f.x }; // right of forward, with +y pointing down the grid
  let v: Vec2;
  switch (input) {
    case 'up': v = f; break;
    case 'down': v = { x: -f.x, y: -f.y }; break;
    case 'right': v = right; break;
    case 'left': v = { x: -right.x, y: -right.y }; break;
    default: return 'none';
  }
  if (v.x === 1) return 'right';
  if (v.x === -1) return 'left';
  return v.y === 1 ? 'down' : 'up';
}

/** Potion bottles found lying in the grove. */
export type PotionKind = 'life' | 'shield' | 'invisible';

/** ASCII glyphs for potions in level rows. */
export const POTION_GLYPHS: Record<string, PotionKind> = { r: 'life', b: 'shield', v: 'invisible' };
export const POTION_GLYPH_OF: Record<PotionKind, string> = { life: 'r', shield: 'b', invisible: 'v' };
