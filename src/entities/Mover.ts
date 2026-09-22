import { Grid } from '../world/Grid';
import { Dir, DIR_VECTORS, OPPOSITE, Tile, Vec2 } from '../world/types';

/**
 * Grid-locked mover. `tile` is the integer tile we are leaving/standing on and
 * `offset` (0..1) is how far we have traveled toward the next tile along `dir`.
 * Decisions happen only when we arrive at a tile center (offset wraps past 1).
 */
export class Mover {
  tile: Vec2;
  offset = 0;
  dir: Dir = 'none';
  speed: number; // tiles per second
  /** Flyers ignore elevation and pits. */
  flying = false;
  /** Width of the world last stepped through, so `occupied` can wrap in a tunnel. */
  private worldWidth = 0;

  constructor(start: Vec2, speed: number) {
    this.tile = { ...start };
    this.speed = speed;
  }

  /** Continuous position in tile units (tile centers are integers). */
  get pos(): Vec2 {
    const v = DIR_VECTORS[this.dir];
    return { x: this.tile.x + v.x * this.offset, y: this.tile.y + v.y * this.offset };
  }

  /** Floor elevation under the mover, blended between the tiles it is crossing. */
  elevationOn(grid: Grid): number {
    this.worldWidth = grid.width;
    const a = grid.elevation(this.tile.x, this.tile.y);
    if (this.dir === 'none' || this.offset === 0) return a;
    const n = grid.wrap(this.nextTile(this.tile));
    const b = grid.get(n.x, n.y) === Tile.Rock ? a : grid.elevation(n.x, n.y);
    return a + (b - a) * this.offset;
  }

  /** Tile the mover is "in" for collision and pickup purposes. */
  /**
   * The tile this mover counts as standing on. Half a tile past the centre it
   * becomes the tile ahead, wrapped, so a mover crossing a tunnel edge reports
   * the tile it is really entering rather than one off the end of the map.
   */
  get occupied(): Vec2 {
    const p = this.offset >= 0.5 ? this.nextTile(this.tile) : this.tile;
    if (this.worldWidth <= 0) return p;
    return { x: ((p.x % this.worldWidth) + this.worldWidth) % this.worldWidth, y: p.y };
  }

  protected nextTile(from: Vec2): Vec2 {
    const v = DIR_VECTORS[this.dir];
    return { x: from.x + v.x, y: from.y + v.y };
  }

  teleport(to: Vec2): void {
    this.tile = { ...to };
    this.offset = 0;
    this.dir = 'none';
  }

  /** Immediately flip direction, preserving position. */
  reverse(grid: Grid): void {
    this.worldWidth = grid.width;
    if (this.dir === 'none' || this.offset === 0) {
      this.dir = OPPOSITE[this.dir];
      return;
    }
    this.tile = grid.wrap(this.nextTile(this.tile));
    this.offset = 1 - this.offset;
    this.dir = OPPOSITE[this.dir];
  }

  /**
   * Advance along the grid. `decide` is asked for a direction at every tile
   * center (and once when idle). Returns true if the mover crossed a center.
   */
  step(
    dt: number,
    grid: Grid,
    decide: (tile: Vec2, current: Dir) => Dir,
    allowNest = false,
    allowMidReverse = false,
    stopWhenIdle = false,
  ): boolean {
    this.worldWidth = grid.width;
    let crossed = false;

    if (this.dir === 'none') {
      const want = decide(this.tile, this.dir);
      if (grid.canMove(this.tile, want, allowNest, this.flying)) this.dir = want;
      else return false;
    } else if (allowMidReverse) {
      const want = decide(this.tile, this.dir);
      if (want === OPPOSITE[this.dir]) this.reverse(grid);
    }

    let remaining = this.speed * dt;
    while (remaining > 0) {
      const toCenter = 1 - this.offset;
      if (remaining < toCenter) {
        this.offset += remaining;
        remaining = 0;
        break;
      }
      // Arrive at the next tile center.
      remaining -= toCenter;
      this.tile = grid.wrap(this.nextTile(this.tile));
      this.offset = 0;
      crossed = true;

      const want = decide(this.tile, this.dir);
      if (want !== 'none' && grid.canMove(this.tile, want, allowNest, this.flying)) {
        this.dir = want;
      } else if ((stopWhenIdle && want === 'none') || !grid.canMove(this.tile, this.dir, allowNest, this.flying)) {
        // Either nothing is asked for (hold-to-move release) or the way is blocked: rest on this center.
        this.dir = 'none';
        remaining = 0;
        break;
      }
    }
    return crossed;
  }
}
