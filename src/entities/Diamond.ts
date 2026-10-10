import { Vec2 } from '../world/types';

export const DIAMOND_CYCLE_SECONDS = 1.6;
export const DIAMOND_RESPAWN_SECONDS = 25;

export type DiamondKind = 'weapon' | 'item';

/**
 * A shrine that cycles through a list of options. It advances on its own
 * every few seconds and immediately when struck with the sword; stepping on
 * it takes whatever is showing. Taken shrines come back after a while.
 */
export class Diamond<T extends string> {
  readonly tile: Vec2;
  readonly kind: DiamondKind;
  readonly options: readonly T[];
  index = 0;
  active = true;
  cycleTimer = DIAMOND_CYCLE_SECONDS;
  respawnTimer = 0;

  constructor(kind: DiamondKind, tile: Vec2, options: readonly T[], startIndex = 0) {
    this.kind = kind;
    this.tile = { ...tile };
    this.options = options;
    this.index = startIndex % options.length;
  }

  get current(): T {
    return this.options[this.index];
  }

  /** 0..1 progress toward the next automatic cycle, for the spin animation. */
  get cycleProgress(): number {
    return 1 - Math.max(0, this.cycleTimer) / DIAMOND_CYCLE_SECONDS;
  }

  /** Advance to the next option (sword strike). */
  cycle(): void {
    if (!this.active) return;
    this.index = (this.index + 1) % this.options.length;
    this.cycleTimer = DIAMOND_CYCLE_SECONDS;
  }

  /** Take the showing option; the shrine goes dormant until it respawns. */
  take(): T {
    const chosen = this.current;
    this.active = false;
    this.respawnTimer = DIAMOND_RESPAWN_SECONDS;
    return chosen;
  }

  update(dt: number): void {
    if (!this.active) {
      this.respawnTimer -= dt;
      if (this.respawnTimer <= 0) {
        this.active = true;
        this.cycleTimer = DIAMOND_CYCLE_SECONDS;
      }
      return;
    }
    this.cycleTimer -= dt;
    if (this.cycleTimer <= 0) {
      this.index = (this.index + 1) % this.options.length;
      this.cycleTimer += DIAMOND_CYCLE_SECONDS;
    }
  }
}
