import { Tile, Dir, DIR_VECTORS, Vec2, CARDINALS } from './types';

/**
 * Immutable-shape tile grid. Tiles that get consumed (nodules, charges)
 * are mutated in place via `consume`; layout never changes.
 */
export class Grid {
  readonly width: number;
  readonly height: number;
  private tiles: Tile[];

  constructor(width: number, height: number, tiles: Tile[]) {
    if (tiles.length !== width * height) {
      throw new Error(`Grid size mismatch: expected ${width * height}, got ${tiles.length}`);
    }
    this.width = width;
    this.height = height;
    this.tiles = tiles.slice();
  }

  inBounds(x: number, y: number): boolean {
    return x >= 0 && y >= 0 && x < this.width && y < this.height;
  }

  /** Wraps horizontally so tunnels connect across the edges. */
  wrap(p: Vec2): Vec2 {
    return { x: ((p.x % this.width) + this.width) % this.width, y: p.y };
  }

  get(x: number, y: number): Tile {
    if (!this.inBounds(x, y)) return Tile.Rock;
    return this.tiles[y * this.width + x];
  }

  set(x: number, y: number, t: Tile): void {
    if (this.inBounds(x, y)) this.tiles[y * this.width + x] = t;
  }

  isWalkable(x: number, y: number, allowNest = false): boolean {
    const t = this.get(x, y);
    if (t === Tile.Rock) return false;
    if (t === Tile.Nest) return allowNest;
    return true;
  }

  neighbor(p: Vec2, dir: Dir): Vec2 {
    const v = DIR_VECTORS[dir];
    return this.wrap({ x: p.x + v.x, y: p.y + v.y });
  }

  canMove(p: Vec2, dir: Dir, allowNest = false): boolean {
    if (dir === 'none') return false;
    const n = this.neighbor(p, dir);
    return this.isWalkable(n.x, n.y, allowNest);
  }

  openDirections(p: Vec2, allowNest = false): Dir[] {
    return CARDINALS.filter((d) => this.canMove(p, d, allowNest));
  }

  /** Removes a collectible and returns what was there. */
  consume(x: number, y: number): Tile.Nodule | Tile.Charge | null {
    const t = this.get(x, y);
    if (t === Tile.Nodule || t === Tile.Charge) {
      this.set(x, y, Tile.Open);
      return t;
    }
    return null;
  }

  count(tile: Tile): number {
    let n = 0;
    for (const t of this.tiles) if (t === tile) n++;
    return n;
  }

  forEach(fn: (x: number, y: number, t: Tile) => void): void {
    for (let y = 0; y < this.height; y++) {
      for (let x = 0; x < this.width; x++) fn(x, y, this.tiles[y * this.width + x]);
    }
  }

  static manhattan(a: Vec2, b: Vec2): number {
    return Math.abs(a.x - b.x) + Math.abs(a.y - b.y);
  }

  static distSq(a: Vec2, b: Vec2): number {
    const dx = a.x - b.x;
    const dy = a.y - b.y;
    return dx * dx + dy * dy;
  }
}
