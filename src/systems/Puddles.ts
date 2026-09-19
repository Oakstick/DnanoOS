import { Vec2 } from '../world/types';

export const PUDDLE_SECONDS = 8;

export interface Puddle {
  tile: Vec2;
  life: number;
}

/** Temporary slime marks left on tiles; each fades out on its own timer. */
export class PuddleField {
  private map = new Map<string, Puddle>();

  private static key(t: Vec2): string {
    return `${t.x},${t.y}`;
  }

  /** Drops (or refreshes) a puddle on a tile. */
  add(tile: Vec2, seconds = PUDDLE_SECONDS): void {
    const k = PuddleField.key(tile);
    const existing = this.map.get(k);
    if (existing) existing.life = Math.max(existing.life, seconds);
    else this.map.set(k, { tile: { ...tile }, life: seconds });
  }

  has(tile: Vec2): boolean {
    return this.map.has(PuddleField.key(tile));
  }

  update(dt: number): void {
    for (const [k, p] of this.map) {
      p.life -= dt;
      if (p.life <= 0) this.map.delete(k);
    }
  }

  clear(): void {
    this.map.clear();
  }

  get count(): number {
    return this.map.size;
  }

  entries(): IterableIterator<[string, Puddle]> {
    return this.map.entries();
  }

  values(): Puddle[] {
    return [...this.map.values()];
  }
}
