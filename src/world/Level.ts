import { Grid } from './Grid';
import { Tile, Vec2 } from './types';

/**
 * ASCII legend:
 *   #  rock          .  nodule        o  chem-charge
 *   ' ' open artery  P  drone spawn   G  predator spawn (nest exit)
 *   N  nest interior (predators only)  -  tunnel (wraps horizontally)
 */
export interface LevelData {
  name: string;
  rows: string[];
  predatorSpeed: number;
  droneSpeed: number;
  frenzySeconds: number;
}

export interface ParsedLevel {
  grid: Grid;
  droneSpawn: Vec2;
  predatorSpawn: Vec2;
  nestCenter: Vec2;
  data: LevelData;
}

export function parseLevel(data: LevelData): ParsedLevel {
  const height = data.rows.length;
  const width = data.rows[0].length;
  const tiles: Tile[] = [];
  let droneSpawn: Vec2 | null = null;
  let predatorSpawn: Vec2 | null = null;
  const nestTiles: Vec2[] = [];

  for (let y = 0; y < height; y++) {
    const row = data.rows[y];
    if (row.length !== width) {
      throw new Error(`Level "${data.name}" row ${y} has width ${row.length}, expected ${width}`);
    }
    for (let x = 0; x < width; x++) {
      const c = row[x];
      switch (c) {
        case '#': tiles.push(Tile.Rock); break;
        case '.': tiles.push(Tile.Nodule); break;
        case 'o': tiles.push(Tile.Charge); break;
        case ' ': tiles.push(Tile.Open); break;
        case '-': tiles.push(Tile.Tunnel); break;
        case 'P': tiles.push(Tile.Open); droneSpawn = { x, y }; break;
        case 'G': tiles.push(Tile.Open); predatorSpawn = { x, y }; break;
        case 'N': tiles.push(Tile.Nest); nestTiles.push({ x, y }); break;
        default: throw new Error(`Unknown tile "${c}" at ${x},${y} in "${data.name}"`);
      }
    }
  }

  if (!droneSpawn) throw new Error(`Level "${data.name}" has no drone spawn (P)`);
  if (!predatorSpawn) throw new Error(`Level "${data.name}" has no predator spawn (G)`);

  const nestCenter = nestTiles.length
    ? {
        x: Math.round(nestTiles.reduce((s, p) => s + p.x, 0) / nestTiles.length),
        y: Math.round(nestTiles.reduce((s, p) => s + p.y, 0) / nestTiles.length),
      }
    : predatorSpawn;

  return { grid: new Grid(width, height, tiles), droneSpawn, predatorSpawn, nestCenter, data };
}
