import { Vec2 } from '../world/types';

export const BOMB_FUSE = 3.0;
export const BOMBS_PER_LIFE = 3;

/**
 * A dropped bomb. Sits on a tile, flashes faster as the fuse burns down,
 * then explodes. Other explosions can ignite it early, as with TNT.
 */
export class Bomb {
  readonly tile: Vec2;
  fuse = BOMB_FUSE;

  constructor(tile: Vec2) {
    this.tile = { ...tile };
  }

  /** 0..1 how far the fuse has burned. */
  get progress(): number {
    return 1 - Math.max(0, this.fuse) / BOMB_FUSE;
  }

  /** Flashing state: on/off, accelerating toward detonation. */
  isFlashOn(time: number): boolean {
    const hz = 2 + this.progress * 10;
    return Math.floor(time * hz) % 2 === 0;
  }

  /** Returns true on the tick the bomb should explode. */
  update(dt: number): boolean {
    this.fuse -= dt;
    return this.fuse <= 0;
  }

  /** Shorten the fuse when caught in another blast. */
  ignite(delay: number): void {
    this.fuse = Math.min(this.fuse, delay);
  }
}
