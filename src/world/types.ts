export enum Tile {
  Rock = 0,
  Open = 1,
  Nodule = 2,
  Charge = 3,
  Nest = 4,
  Tunnel = 5,
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
