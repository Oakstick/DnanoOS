import { Rng } from '../engine/Rng';
import { LevelData } from './Level';
import { POTION_GLYPH_OF, PotionKind } from './types';

/** Everyone moves at this fraction of the original tuning. */
export const SPEED_SCALE = 0.8;
/** Gold nuggets scattered through each dungeon; collecting them all wins. */
export const NUGGETS_PER_LEVEL = 25;
/** Potion bottles per dungeon: one extra life, two shields, two invisibility. */
export const POTION_PLAN: PotionKind[] = ['life', 'shield', 'shield', 'invisible', 'invisible'];
/** Pits cut across corridors that must be jumped. */
export const PITS_PER_LEVEL = 6;
/** Manhattan distance kept between two pits. */
const MIN_PIT_SPACING = 6;
/** Tiles per authored cell: corridors are this many tiles wide. */
export const WORLD_SCALE = 1;

/**
 * 14 x 12 base quadrant, mirrored into a 27 x 23 dungeon. Corridors are one
 * tile wide. The chamber at rows 7-10 is the enemy hollow where slain foes
 * reform, reached by the passage above its G gate.
 */
const QUADRANT: string[] = [
  '##############',
  '#o........W.o#',
  '#.##.##.##.#.#',
  '#.##.##.##.#.#',
  '-.P..........#',
  '#.##.##.##.#.#',
  '#.##.##.##.#.#',
  '#.I..##G##...#',
  '#.##.#NNN#.#.#',
  '#.##.#NNN#.#.#',
  '#o...#####..o#',
  '##############',
];

/** Rows of the shared column that open between the left and right halves. */
const COLUMN_GATES = [1, 4, 10];
/** Columns of the middle row that open between the top and bottom halves. */
const ROW_GATES = [1, 10, 16, 25];
/** Gates that carry a ramp; every other gate is a one-level ledge you must jump up. */
const RAMP_COLUMN_GATES = [4];
const RAMP_ROW_GATES = [10, 16];

const QUADRANT_LEVELS = { tl: 0, tr: 1, bl: 1, br: 2 };

export interface Layout {
  rows: string[];
  heights: string[];
}

/**
 * Mirrors the quadrant right and down into a 27 x 23 dungeon. The shared
 * border column and row are punched through at a few gates. Only the top-left
 * copy keeps its hollow and spawn; the other hollows become walled clearings.
 * Shrines alternate quadrants: weapons top-left and bottom-right, items
 * top-right and bottom-left.
 */
export function composeDungeon(base: string[] = QUADRANT): Layout {
  const top = base.map((row) => row + row.split('').reverse().join('').slice(1));
  const rows = [...top, ...top.slice(0, -1).reverse()];
  const grid = rows.map((r) => r.split(''));
  const w = grid[0].length;
  const h = grid.length;
  const midX = (w - 1) / 2;
  const midY = (h - 1) / 2;

  const inTopLeft = (x: number, y: number) => x < midX && y < midY;
  const quadrant = (x: number, y: number) => (x < midX ? 0 : 1) + (y < midY ? 0 : 2);

  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const c = grid[y][x];
      switch (c) {
        case 'N':
        case 'G':
        case 'P':
          if (!inTopLeft(x, y)) grid[y][x] = '.';
          break;
        case 'W':
          if (quadrant(x, y) !== 0 && quadrant(x, y) !== 3) grid[y][x] = '.';
          break;
        case 'I':
          if (quadrant(x, y) !== 1 && quadrant(x, y) !== 2) grid[y][x] = '.';
          break;
        case '-':
          if (x === midX) grid[y][x] = '.';
          break;
        default:
          break;
      }
    }
  }

  for (const y of COLUMN_GATES) {
    grid[y][midX] = '.';
    grid[h - 1 - y][midX] = '.';
  }
  for (const x of ROW_GATES) grid[midY][x] = '.';

  // Elevation: quadrant levels; a gate tile takes the lower side so you walk on
  // and jump (or ramp) up; tunnel mouths are ramps so the wrap works both ways.
  const levelOf = (x: number, y: number): number => {
    if (x < midX && y < midY) return QUADRANT_LEVELS.tl;
    if (x > midX && y < midY) return QUADRANT_LEVELS.tr;
    if (x < midX && y > midY) return QUADRANT_LEVELS.bl;
    if (x > midX && y > midY) return QUADRANT_LEVELS.br;
    // Shared column or row: the lower neighbor's level.
    if (x === midX) return y < midY ? QUADRANT_LEVELS.tl : QUADRANT_LEVELS.bl;
    return x < midX ? QUADRANT_LEVELS.tl : QUADRANT_LEVELS.tr;
  };
  const heights = grid.map((row, y) => row.map((c, x) => {
    if (c === '-') return '/';
    if (x === midX && (RAMP_COLUMN_GATES.includes(y) || RAMP_COLUMN_GATES.includes(h - 1 - y))) return '/';
    if (y === midY && RAMP_ROW_GATES.includes(x)) return '/';
    return String(levelOf(x, y));
  }));

  return { rows: grid.map((r) => r.join('')), heights: heights.map((r) => r.join('')) };
}

/**
 * Cuts pits into straight corridor segments, away from gates, the spawn, the
 * hollow, and shrines, with both banks at the same level so a two-tile jump
 * always clears them. '~' marks a pit in a left-right corridor, '^' one in an
 * up-down corridor; after upscaling each becomes a one-tile trench across the
 * full road width.
 */
export function placePits(layout: Layout, count: number, seed: number): Layout {
  const grid = layout.rows.map((r) => r.split(''));
  const w = grid[0].length;
  const h = grid.length;
  const midX = (w - 1) / 2;
  const midY = (h - 1) / 2;
  const open = (x: number, y: number) => x >= 0 && y >= 0 && x < w && y < h && (grid[y][x] === '.' || grid[y][x] === ' ');
  const rock = (x: number, y: number) => x < 0 || y < 0 || x >= w || y >= h || grid[y][x] === '#';
  const level = (x: number, y: number) => layout.heights[y][x];

  const candidates: { x: number; y: number; glyph: string }[] = [];
  for (let y = 1; y < h - 1; y++) {
    for (let x = 1; x < w - 1; x++) {
      if (grid[y][x] !== '.') continue;
      if (Math.abs(x - midX) <= 3 || Math.abs(y - midY) <= 3) continue;
      const horizontal = open(x - 1, y) && open(x + 1, y) && rock(x, y - 1) && rock(x, y + 1)
        && level(x - 1, y) === level(x + 1, y) && level(x - 1, y) === level(x, y);
      const vertical = open(x, y - 1) && open(x, y + 1) && rock(x - 1, y) && rock(x + 1, y)
        && level(x, y - 1) === level(x, y + 1) && level(x, y - 1) === level(x, y);
      if (horizontal) candidates.push({ x, y, glyph: '~' });
      else if (vertical) candidates.push({ x, y, glyph: '^' });
    }
  }

  const rng = new Rng(seed * 104729 + 3);
  for (let i = candidates.length - 1; i > 0; i--) {
    const j = Math.floor(rng.next() * (i + 1));
    [candidates[i], candidates[j]] = [candidates[j], candidates[i]];
  }
  // With one-tile corridors a single pit can seal off a whole wing, so a cut is
  // only kept when every tile still walkable from the hollow stays walkable.
  const passable = (x: number, y: number): boolean => {
    if (x < 0 || y < 0 || x >= w || y >= h) return false;
    const c = grid[y][x];
    return c !== '#' && c !== '~' && c !== '^';
  };
  let start = { x: 1, y: 1 };
  for (let y = 0; y < h; y++) {
    const x = grid[y].indexOf('G');
    if (x >= 0) start = { x, y };
  }
  const floodSize = (): number => {
    const seen = new Set([`${start.x},${start.y}`]);
    const stack = [start];
    let n = 0;
    while (stack.length > 0) {
      const p = stack.pop()!;
      n++;
      for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) {
        const q = { x: p.x + dx, y: p.y + dy };
        const key = `${q.x},${q.y}`;
        if (seen.has(key) || !passable(q.x, q.y)) continue;
        seen.add(key);
        stack.push(q);
      }
    }
    return n;
  };

  const chosen: { x: number; y: number; glyph: string }[] = [];
  let reach = floodSize();
  for (const c of candidates) {
    if (chosen.length >= count) break;
    if (!chosen.every((p) => Math.abs(p.x - c.x) + Math.abs(p.y - c.y) >= MIN_PIT_SPACING)) continue;
    const was = grid[c.y][c.x];
    grid[c.y][c.x] = c.glyph;
    const now = floodSize();
    if (now === reach - 1) {
      chosen.push(c);
      reach = now;
    } else {
      grid[c.y][c.x] = was; // this cut would strand part of the dungeon
    }
  }
  return { rows: grid.map((r) => r.join('')), heights: layout.heights };
}

/**
 * Blows the authored layout up by WORLD_SCALE so every road is that many
 * tiles wide. Walls, floor, hollow, and tunnel cells fill their block; a
 * single-tile feature (nugget, crystal, shrine, spawn, potion) sits at the
 * block's center on open floor; a pit becomes a one-tile trench across the
 * road so a jump still clears it. Heights fill their block.
 */
export function upscale(layout: Layout, scale: number = WORLD_SCALE): Layout {
  const mid = Math.floor(scale / 2);
  const rows: string[] = [];
  const heights: string[] = [];
  layout.rows.forEach((row, y) => {
    for (let dy = 0; dy < scale; dy++) {
      let out = '';
      let hout = '';
      for (let x = 0; x < row.length; x++) {
        const c = row[x];
        const hch = layout.heights[y][x];
        for (let dx = 0; dx < scale; dx++) {
          let t: string;
          switch (c) {
            case '#': case ' ': case 'N': case '-':
              t = c;
              break;
            case '~': // trench across a left-right road: the middle column of the block
              t = dx === mid ? '~' : ' ';
              break;
            case '^': // trench across an up-down road: the middle row of the block
              t = dy === mid ? '~' : ' ';
              break;
            default: // single-tile features at the block center, floor around them
              t = dx === mid && dy === mid ? c : ' ';
          }
          out += t;
          hout += hch;
        }
      }
      rows.push(out);
      heights.push(hout);
    }
  });
  return { rows, heights };
}

/**
 * Replaces the coin carpet with `count` gold nuggets. Every '.' becomes plain
 * floor, then nuggets are placed on open tiles by a seeded shuffle with a
 * minimum spacing that relaxes only if the dungeon cannot fit them all.
 */
export function placeNuggets(rows: string[], count: number, seed: number): string[] {
  const grid = rows.map((r) => r.split('').map((c) => (c === '.' ? ' ' : c)));
  const open: { x: number; y: number }[] = [];
  grid.forEach((row, y) => row.forEach((c, x) => {
    if (c === ' ') open.push({ x, y });
  }));

  const rng = new Rng(seed);
  for (let i = open.length - 1; i > 0; i--) {
    const j = Math.floor(rng.next() * (i + 1));
    [open[i], open[j]] = [open[j], open[i]];
  }

  const chosen: { x: number; y: number }[] = [];
  let minDist = 5;
  while (chosen.length < count && minDist >= 0) {
    for (const p of open) {
      if (chosen.length >= count) break;
      const farEnough = chosen.every((c) => Math.abs(c.x - p.x) + Math.abs(c.y - p.y) >= minDist);
      if (farEnough && !chosen.includes(p)) chosen.push(p);
    }
    minDist--;
  }
  for (const p of chosen) grid[p.y][p.x] = '.';
  return grid.map((r) => r.join(''));
}

/**
 * Drops potion bottles on plain open floor, away from nuggets and each other.
 * Bottles never share a tile with anything else.
 */
export function placePotions(rows: string[], plan: PotionKind[], seed: number): string[] {
  const grid = rows.map((r) => r.split(''));
  const open: { x: number; y: number }[] = [];
  const busy: { x: number; y: number }[] = [];
  grid.forEach((row, y) => row.forEach((c, x) => {
    if (c === ' ') open.push({ x, y });
    else if (c !== '#' && c !== '~' && c !== '^') busy.push({ x, y });
  }));

  const rng = new Rng(seed * 7919 + 17);
  for (let i = open.length - 1; i > 0; i--) {
    const j = Math.floor(rng.next() * (i + 1));
    [open[i], open[j]] = [open[j], open[i]];
  }

  // Two separate rules: never crowd a nugget or shrine, and keep the bottles
  // well apart from each other. Sharing one distance made them clump, because
  // in a small dungeon no tile is far from every nugget at once.
  const chosen: { x: number; y: number }[] = [];
  let apart = 8;
  while (chosen.length < plan.length && apart >= 0) {
    for (const p of open) {
      if (chosen.length >= plan.length) break;
      if (chosen.includes(p)) continue;
      if (busy.some((c) => Math.abs(c.x - p.x) + Math.abs(c.y - p.y) < 2)) continue;
      if (chosen.some((c) => Math.abs(c.x - p.x) + Math.abs(c.y - p.y) < apart)) continue;
      chosen.push(p);
    }
    apart--;
  }
  chosen.forEach((p, i) => {
    grid[p.y][p.x] = POTION_GLYPH_OF[plan[i]];
  });
  return grid.map((r) => r.join(''));
}

/**
 * Pits, nuggets and potions for a dungeon, seeded so each has its own layout,
 * placed on the authored grid and then upscaled to wide roads.
 */
export function decorateDungeon(layout: Layout, seed: number): Layout {
  const pitted = placePits(layout, PITS_PER_LEVEL, seed);
  const rows = placePotions(placeNuggets(pitted.rows, NUGGETS_PER_LEVEL, seed), POTION_PLAN, seed);
  return upscale({ rows, heights: pitted.heights });
}

const DUNGEON = composeDungeon();

function dungeon(name: string, seed: number, heroSpeed: number, enemySpeed: number, frenzySeconds: number): LevelData {
  const layout = decorateDungeon(DUNGEON, seed);
  return { name, rows: layout.rows, heights: layout.heights, heroSpeed, enemySpeed, frenzySeconds };
}

export const LEVELS: LevelData[] = [
  dungeon('Dungeon I: Sunken Halls', 1, 6.5 * SPEED_SCALE, 5.4 * SPEED_SCALE, 7),
  dungeon('Dungeon II: The Cisterns', 2, 7 * SPEED_SCALE, 6.3 * SPEED_SCALE, 5.5),
  dungeon('Dungeon III: Bone Vaults', 3, 7.5 * SPEED_SCALE, 7.3 * SPEED_SCALE, 4),
];

/** Beyond the last authored level, speeds keep ramping and hazards land in new spots. */
export function levelFor(index: number): LevelData {
  if (index < LEVELS.length) return LEVELS[index];
  const base = LEVELS[LEVELS.length - 1];
  const extra = index - LEVELS.length + 1;
  const layout = decorateDungeon(DUNGEON, index + 1);
  return {
    ...base,
    rows: layout.rows,
    heights: layout.heights,
    name: `Dungeon ${index + 1}: Deeper Still`,
    enemySpeed: Math.min(base.enemySpeed + extra * 0.3 * SPEED_SCALE, 9 * SPEED_SCALE),
    heroSpeed: Math.min(base.heroSpeed + extra * 0.2 * SPEED_SCALE, 9 * SPEED_SCALE),
    frenzySeconds: Math.max(base.frenzySeconds - extra * 0.5, 2),
  };
}
