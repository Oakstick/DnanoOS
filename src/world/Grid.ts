import { Tile, Dir, DIR_VECTORS, Vec2, CARDINALS } from './types';

/** Tiles a jump covers: two forward, or one when two is blocked. */
export const JUMP_TILES = 2;

/**
 * Tile grid with elevation. Layout never changes except by bombs; nodules
 * and charges are consumed in place. Every tile has an integer floor height
 * (0 = ground). Walking may step down any drop but climb only via a ramp;
 * jumping climbs one level. Flyers ignore height and pits.
 */
export class Grid {
  readonly width: number;
  readonly height: number;
  private tiles: Tile[];
  private heights: number[];
  private ramps: boolean[];

  constructor(width: number, height: number, tiles: Tile[], heights?: number[], ramps?: boolean[]) {
    if (tiles.length !== width * height) {
      throw new Error(`Grid size mismatch: expected ${width * height}, got ${tiles.length}`);
    }
    this.width = width;
    this.height = height;
    this.tiles = tiles.slice();
    this.heights = heights ? heights.slice() : new Array<number>(tiles.length).fill(0);
    this.ramps = ramps ? ramps.slice() : new Array<boolean>(tiles.length).fill(false);
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

  /** Floor elevation in levels; rock reports the height of its tallest walkable neighbor's floor. */
  elevation(x: number, y: number): number {
    if (!this.inBounds(x, y)) return 0;
    return this.heights[y * this.width + x];
  }

  setElevation(x: number, y: number, h: number): void {
    if (this.inBounds(x, y)) this.heights[y * this.width + x] = h;
  }

  isRamp(x: number, y: number): boolean {
    return this.inBounds(x, y) && this.ramps[y * this.width + x];
  }

  isPit(x: number, y: number): boolean {
    return this.get(x, y) === Tile.Pit;
  }

  /** Standable floor: not rock, not a pit, and the nest only when allowed. */
  isWalkable(x: number, y: number, allowNest = false): boolean {
    const t = this.get(x, y);
    if (t === Tile.Rock || t === Tile.Pit) return false;
    if (t === Tile.Nest) return allowNest;
    return true;
  }

  /** Air: anything but rock. Flyers cross pits and hover over the nest. */
  isFlyable(x: number, y: number, allowNest = false): boolean {
    const t = this.get(x, y);
    if (t === Tile.Rock) return false;
    if (t === Tile.Nest) return allowNest;
    return true;
  }

  neighbor(p: Vec2, dir: Dir): Vec2 {
    const v = DIR_VECTORS[dir];
    return this.wrap({ x: p.x + v.x, y: p.y + v.y });
  }

  /**
   * Can a mover standing on `p` step to the next tile in `dir`? On foot that
   * means solid floor and no climb, unless a ramp joins the two tiles. In the
   * air only rock stops you.
   */
  canMove(p: Vec2, dir: Dir, allowNest = false, flying = false): boolean {
    if (dir === 'none') return false;
    const n = this.neighbor(p, dir);
    if (flying) return this.isFlyable(n.x, n.y, allowNest);
    if (!this.isWalkable(n.x, n.y, allowNest)) return false;
    const climb = this.elevation(n.x, n.y) - this.elevation(p.x, p.y);
    if (climb <= 0) return true;
    return climb === 1 && (this.isRamp(p.x, p.y) || this.isRamp(n.x, n.y));
  }

  openDirections(p: Vec2, allowNest = false, flying = false): Dir[] {
    return CARDINALS.filter((d) => this.canMove(p, d, allowNest, flying));
  }

  /**
   * Where a jump from `p` in `dir` lands: two tiles ahead if that floor exists
   * and is at most one level up and the tile in between is not rock; else one
   * tile ahead under the same rules; else null (a hop in place).
   */
  jumpLanding(p: Vec2, dir: Dir): Vec2 | null {
    if (dir === 'none') return null;
    const from = this.elevation(p.x, p.y);
    const one = this.neighbor(p, dir);
    const two = this.neighbor(one, dir);
    const landable = (t: Vec2) => this.isWalkable(t.x, t.y) && this.elevation(t.x, t.y) - from <= 1;
    if (this.get(one.x, one.y) !== Tile.Rock && landable(two)) return two;
    if (landable(one)) return one;
    return null;
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
