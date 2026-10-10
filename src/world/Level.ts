import { Grid } from './Grid';
import { POTION_GLYPHS, PotionKind, Tile, Vec2 } from './types';

/**
 * ASCII legend for `rows`:
 *   #  rock          .  nodule        o  chem-charge      ~  pit
 *   ' ' open floor   P  hero spawn    G  enemy spawn (hollow exit)
 *   N  hollow interior (enemies only)  -  tunnel (wraps horizontally)
 *   W  weapon shrine (open tile)     I  item shrine (open tile)
 *   r  life potion   b  shield potion   v  invisibility potion (open tiles)
 *
 * Optional `heights` rows run parallel to `rows`:
 *   0 1 2  floor elevation in levels     /  ramp joining two levels
 *   anything else (or no heights at all) = level 0
 */
export interface LevelData {
  name: string;
  rows: string[];
  heights?: string[];
  enemySpeed: number;
  heroSpeed: number;
  frenzySeconds: number;
}

export interface ParsedLevel {
  grid: Grid;
  heroSpawn: Vec2;
  enemySpawn: Vec2;
  nestCenter: Vec2;
  weaponShrines: Vec2[];
  itemShrines: Vec2[];
  potions: { tile: Vec2; kind: PotionKind }[];
  data: LevelData;
}

export function parseLevel(data: LevelData): ParsedLevel {
  const height = data.rows.length;
  const width = data.rows[0].length;
  const tiles: Tile[] = [];
  const heights: number[] = [];
  const ramps: boolean[] = [];
  let heroSpawn: Vec2 | null = null;
  let enemySpawn: Vec2 | null = null;
  const nestTiles: Vec2[] = [];
  const weaponShrines: Vec2[] = [];
  const itemShrines: Vec2[] = [];
  const potions: { tile: Vec2; kind: PotionKind }[] = [];

  if (data.heights && data.heights.length !== height) {
    throw new Error(`Level "${data.name}" heights have ${data.heights.length} rows, expected ${height}`);
  }

  for (let y = 0; y < height; y++) {
    const row = data.rows[y];
    if (row.length !== width) {
      throw new Error(`Level "${data.name}" row ${y} has width ${row.length}, expected ${width}`);
    }
    const hrow = data.heights?.[y] ?? '';
    for (let x = 0; x < width; x++) {
      const c = row[x];
      switch (c) {
        case '#': tiles.push(Tile.Rock); break;
        case '.': tiles.push(Tile.Nodule); break;
        case 'o': tiles.push(Tile.Charge); break;
        case ' ': tiles.push(Tile.Open); break;
        case '~': tiles.push(Tile.Pit); break;
        case '-': tiles.push(Tile.Tunnel); break;
        case 'P': tiles.push(Tile.Open); heroSpawn = { x, y }; break;
        case 'G': tiles.push(Tile.Open); enemySpawn = { x, y }; break;
        case 'N': tiles.push(Tile.Nest); nestTiles.push({ x, y }); break;
        case 'W': tiles.push(Tile.Open); weaponShrines.push({ x, y }); break;
        case 'I': tiles.push(Tile.Open); itemShrines.push({ x, y }); break;
        default:
          if (POTION_GLYPHS[c]) {
            tiles.push(Tile.Open);
            potions.push({ tile: { x, y }, kind: POTION_GLYPHS[c] });
            break;
          }
          throw new Error(`Unknown tile "${c}" at ${x},${y} in "${data.name}"`);
      }
      const h = hrow[x] ?? '0';
      ramps.push(h === '/');
      heights.push(h >= '0' && h <= '9' ? Number(h) : 0);
    }
  }

  if (!heroSpawn) throw new Error(`Level "${data.name}" has no hero spawn (P)`);
  if (!enemySpawn) throw new Error(`Level "${data.name}" has no enemy spawn (G)`);

  // Ramps take the lower of the floors they join (spreading inward through wide ramp blocks);
  // rock takes its tallest walkable neighbor's floor so walls rise from the raised ground around them.
  const idx = (x: number, y: number) => y * width + x;
  const inb = (x: number, y: number) => x >= 0 && y >= 0 && x < width && y < height;
  const floorAt = (x: number, y: number): number | null => {
    if (!inb(x, y)) return null;
    const t = tiles[idx(x, y)];
    if (t === Tile.Rock || ramps[idx(x, y)]) return null;
    return heights[idx(x, y)];
  };
  const neighbors = (x: number, y: number) => [[x - 1, y], [x + 1, y], [x, y - 1], [x, y + 1]] as const;
  const rampSettled = new Set<number>();
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const around = neighbors(x, y).map(([nx, ny]) => floorAt(nx, ny)).filter((h): h is number => h !== null);
      if (ramps[idx(x, y)] && around.length) {
        heights[idx(x, y)] = Math.min(...around);
        rampSettled.add(idx(x, y));
      }
    }
  }
  for (let pass = 0; pass < width + height; pass++) {
    let changed = false;
    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        const i = idx(x, y);
        if (!ramps[i] || rampSettled.has(i)) continue;
        const near = neighbors(x, y)
          .filter(([nx, ny]) => inb(nx, ny) && ramps[idx(nx, ny)] && rampSettled.has(idx(nx, ny)))
          .map(([nx, ny]) => heights[idx(nx, ny)]);
        if (near.length) {
          heights[i] = Math.min(...near);
          rampSettled.add(i);
          changed = true;
        }
      }
    }
    if (!changed) break;
  }
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      if (tiles[idx(x, y)] !== Tile.Rock) continue;
      const around = neighbors(x, y)
        .filter(([nx, ny]) => inb(nx, ny) && tiles[idx(nx, ny)] !== Tile.Rock)
        .map(([nx, ny]) => heights[idx(nx, ny)]);
      if (around.length) heights[idx(x, y)] = Math.max(...around);
    }
  }

  const nestCenter = nestTiles.length
    ? {
        x: Math.round(nestTiles.reduce((s, p) => s + p.x, 0) / nestTiles.length),
        y: Math.round(nestTiles.reduce((s, p) => s + p.y, 0) / nestTiles.length),
      }
    : enemySpawn;

  return {
    grid: new Grid(width, height, tiles, heights, ramps),
    heroSpawn, enemySpawn, nestCenter, weaponShrines, itemShrines, potions, data,
  };
}
