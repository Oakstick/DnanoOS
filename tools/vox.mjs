/**
 * MagicaVoxel bridge.
 *
 *   node tools/vox.mjs import <file.vox> [--name NAME] [--model N] [--front ymin|ymax]
 *   node tools/vox.mjs export <MODEL_NAME> [--out file.vox]
 *   node tools/vox.mjs info <file.vox>
 *
 * `import` turns a .vox file into a `VoxelModel` literal you can paste into
 * src/render3d/voxels.ts. `export` goes the other way, so a model that lives
 * in voxels.ts can be opened and edited in MagicaVoxel.
 *
 * .vox is a RIFF-like format: a MAIN chunk holding SIZE/XYZI pairs (one per
 * model) and one RGBA palette. MagicaVoxel is z-up with y running away from
 * the default camera; our format is y-up with rows running back to front, so
 * the axes are remapped on the way through.
 */
import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = path.resolve(import.meta.dirname, '..');
const VOXELS_TS = path.join(ROOT, 'src', 'render3d', 'voxels.ts');

// Keys usable in a layer string: '.' and ' ' mean empty, so they are excluded.
const KEYS = 'ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz0123456789';

// ------------------------------------------------------------------ palette

/**
 * MagicaVoxel's built-in palette, used when a file carries no RGBA chunk.
 * Entries 1..216 step r, g and b through six levels each (r slowest); the
 * last 40 are the pure red, green, blue and grey ramps.
 */
function defaultPalette() {
  const out = [];
  const L = [255, 204, 153, 102, 51, 0];
  for (const r of L) for (const g of L) for (const b of L) out.push((r << 16) | (g << 8) | b);
  const ramp = [238, 221, 187, 170, 136, 119, 85, 68, 34, 17];
  for (const v of ramp) out.push(v << 16);
  for (const v of ramp) out.push(v << 8);
  for (const v of ramp) out.push(v);
  for (const v of ramp) out.push((v << 16) | (v << 8) | v);
  return out.slice(0, 256);
}

// ------------------------------------------------------------------- reader

/** Walks the chunk tree and collects the SIZE/XYZI model pairs and the palette. */
function parseVox(buf) {
  if (buf.toString('ascii', 0, 4) !== 'VOX ') throw new Error('not a .vox file (bad magic)');
  const version = buf.readInt32LE(4);
  const models = [];
  let palette = null;
  let pending = null;

  const walk = (start, end) => {
    let p = start;
    while (p + 12 <= end) {
      const id = buf.toString('ascii', p, p + 4);
      const contentSize = buf.readInt32LE(p + 4);
      const childrenSize = buf.readInt32LE(p + 8);
      const content = p + 12;
      const children = content + contentSize;

      if (id === 'SIZE') {
        pending = { x: buf.readInt32LE(content), y: buf.readInt32LE(content + 4), z: buf.readInt32LE(content + 8) };
      } else if (id === 'XYZI') {
        const n = buf.readInt32LE(content);
        const voxels = new Array(n);
        for (let i = 0; i < n; i++) {
          const o = content + 4 + i * 4;
          voxels[i] = { x: buf[o], y: buf[o + 1], z: buf[o + 2], i: buf[o + 3] };
        }
        models.push({ size: pending ?? { x: 0, y: 0, z: 0 }, voxels });
        pending = null;
      } else if (id === 'RGBA') {
        palette = [];
        for (let i = 0; i < 256; i++) {
          const o = content + i * 4;
          palette.push((buf[o] << 16) | (buf[o + 1] << 8) | buf[o + 2]);
        }
      }
      if (childrenSize > 0) walk(children, children + childrenSize);
      p = children + childrenSize;
    }
  };

  const mainSize = buf.readInt32LE(12);
  walk(20 + mainSize, buf.length);
  return { version, models, palette: palette ?? defaultPalette() };
}

/** Crops to the occupied box and maps vox axes onto our layers/rows/columns. */
function toModel(model, palette, front) {
  const { voxels } = model;
  if (voxels.length === 0) throw new Error('model has no voxels');
  const lo = { x: Infinity, y: Infinity, z: Infinity };
  const hi = { x: -Infinity, y: -Infinity, z: -Infinity };
  for (const v of voxels) {
    lo.x = Math.min(lo.x, v.x); hi.x = Math.max(hi.x, v.x);
    lo.y = Math.min(lo.y, v.y); hi.y = Math.max(hi.y, v.y);
    lo.z = Math.min(lo.z, v.z); hi.z = Math.max(hi.z, v.z);
  }
  const width = hi.x - lo.x + 1;
  const depth = hi.y - lo.y + 1;
  const height = hi.z - lo.z + 1;

  // Count colours so the most used ones get the earliest, clearest keys.
  const used = new Map();
  for (const v of voxels) {
    const c = palette[v.i - 1] ?? 0;
    used.set(c, (used.get(c) ?? 0) + 1);
  }
  const order = [...used.entries()].sort((a, b) => b[1] - a[1]).map(([c]) => c);
  if (order.length > KEYS.length) {
    throw new Error(`model uses ${order.length} colours; the text format holds ${KEYS.length}. Reduce colours in MagicaVoxel first.`);
  }
  const keyOf = new Map(order.map((c, i) => [c, KEYS[i]]));

  const layers = [];
  for (let y = 0; y < height; y++) layers.push(Array.from({ length: depth }, () => new Array(width).fill('.')));
  for (const v of voxels) {
    const c = palette[v.i - 1] ?? 0;
    const col = v.x - lo.x;
    const row = front === 'ymax' ? v.y - lo.y : depth - 1 - (v.y - lo.y);
    layers[v.z - lo.z][row][col] = keyOf.get(c);
  }
  return {
    width, depth, height,
    palette: order.map((c) => [keyOf.get(c), c]),
    layers: layers.map((rows) => rows.map((r) => r.join(''))),
  };
}

/** Prints the model as a TypeScript literal in the house style. */
function emit(name, m) {
  const pal = m.palette.map(([k, c]) => `${k}: 0x${c.toString(16).padStart(6, '0')}`).join(', ');
  const lines = [];
  lines.push(`/** ${name}: ${m.width} wide, ${m.depth} deep, ${m.height} tall. Imported from MagicaVoxel. */`);
  lines.push(`export const ${name}: VoxelModel = {`);
  lines.push(`  palette: { ${pal} },`);
  lines.push('  layers: [');
  m.layers.forEach((rows, i) => {
    lines.push(`    // layer ${i}`);
    lines.push(`    [${rows.map((r) => `'${r}'`).join(', ')}],`);
  });
  lines.push('  ],');
  lines.push('};');
  return lines.join('\n');
}

// ------------------------------------------------------------------- writer

function chunk(id, content, children = Buffer.alloc(0)) {
  const head = Buffer.alloc(12);
  head.write(id, 0, 4, 'ascii');
  head.writeInt32LE(content.length, 4);
  head.writeInt32LE(children.length, 8);
  return Buffer.concat([head, content, children]);
}

/** Builds a .vox buffer from one of our text models. */
function writeVox(model) {
  const colors = [];
  const indexOf = new Map();
  for (const [, c] of Object.entries(model.palette)) {
    if (!indexOf.has(c)) {
      colors.push(c);
      indexOf.set(c, colors.length); // .vox colour indices start at 1
    }
  }
  const depth = Math.max(...model.layers.map((rows) => rows.length));
  const width = Math.max(...model.layers.flatMap((rows) => rows.map((r) => r.length)));
  const height = model.layers.length;

  const voxels = [];
  model.layers.forEach((rows, z) => rows.forEach((row, row_i) => {
    for (let x = 0; x < row.length; x++) {
      const key = row[x];
      if (key === '.' || key === ' ') continue;
      const c = model.palette[key];
      if (c === undefined) throw new Error(`palette missing key "${key}"`);
      voxels.push({ x, y: depth - 1 - row_i, z, i: indexOf.get(c) });
    }
  }));

  const size = Buffer.alloc(12);
  size.writeInt32LE(width, 0);
  size.writeInt32LE(depth, 4);
  size.writeInt32LE(height, 8);

  const xyzi = Buffer.alloc(4 + voxels.length * 4);
  xyzi.writeInt32LE(voxels.length, 0);
  voxels.forEach((v, n) => {
    const o = 4 + n * 4;
    xyzi[o] = v.x; xyzi[o + 1] = v.y; xyzi[o + 2] = v.z; xyzi[o + 3] = v.i;
  });

  const rgba = Buffer.alloc(256 * 4);
  colors.forEach((c, i) => {
    const o = i * 4;
    rgba[o] = (c >> 16) & 0xff; rgba[o + 1] = (c >> 8) & 0xff; rgba[o + 2] = c & 0xff; rgba[o + 3] = 255;
  });

  const header = Buffer.alloc(8);
  header.write('VOX ', 0, 4, 'ascii');
  header.writeInt32LE(150, 4);
  const body = Buffer.concat([chunk('SIZE', size), chunk('XYZI', xyzi), chunk('RGBA', rgba)]);
  return Buffer.concat([header, chunk('MAIN', Buffer.alloc(0), body)]);
}

/** Loads voxels.ts by transpiling it with the esbuild that ships inside vite. */
async function loadModels() {
  const cache = path.join(ROOT, 'node_modules', '.cache', 'void-harvest');
  mkdirSync(cache, { recursive: true });
  const out = path.join(cache, 'voxels.mjs');
  const esbuild = path.join(ROOT, 'node_modules', 'esbuild', 'bin', 'esbuild');
  execFileSync(process.execPath, [esbuild, VOXELS_TS, '--format=esm', `--outfile=${out}`], { stdio: 'pipe' });
  const mod = await import(`${pathToFileURL(out).href}?t=${Date.now()}`);
  rmSync(out, { force: true });
  return mod;
}

// --------------------------------------------------------------------- main

async function main() {
    const [cmd, target, ...rest] = process.argv.slice(2);
  const flag = (name, fallback) => {
    const i = rest.indexOf(`--${name}`);
    return i === -1 ? fallback : rest[i + 1];
  };

  if (cmd === 'import' || cmd === 'info') {
    if (!target) throw new Error('usage: node tools/vox.mjs import <file.vox>');
    const parsed = parseVox(readFileSync(target));
    if (cmd === 'info') {
      console.log(`${path.basename(target)}: version ${parsed.version}, ${parsed.models.length} model(s)`);
      parsed.models.forEach((m, i) => console.log(`  [${i}] ${m.size.x}x${m.size.y}x${m.size.z}, ${m.voxels.length} voxels`));
      process.exit(0);
    }
    const index = Number(flag('model', '0'));
    const m = parsed.models[index];
    if (!m) throw new Error(`no model ${index}; the file has ${parsed.models.length}`);
    const name = (flag('name', path.basename(target, '.vox'))).toUpperCase().replace(/[^A-Z0-9_]/g, '_');
    const built = toModel(m, parsed.palette, flag('front', 'ymin'));
    console.error(`${built.width}w x ${built.depth}d x ${built.height}h, ${m.voxels.length} voxels, ${built.palette.length} colours`);
    console.log(emit(name, built));
  } else if (cmd === 'export') {
    if (!target) throw new Error('usage: node tools/vox.mjs export <MODEL_NAME>');
    const models = await loadModels();
    const model = models[target];
    if (!model || !model.layers) {
      const names = Object.keys(models).filter((k) => models[k] && models[k].layers);
      throw new Error(`no model "${target}". Available: ${names.join(', ')}`);
    }
    const out = flag('out', path.join(ROOT, 'art', `${target.toLowerCase()}.vox`));
    mkdirSync(path.dirname(out), { recursive: true });
    writeFileSync(out, writeVox(model));
    console.log(`wrote ${path.relative(ROOT, out)}`);
  } else {
    console.error('usage: node tools/vox.mjs <import|export|info> <target> [flags]');
    process.exit(1);
  }
}

export { defaultPalette, emit, parseVox, toModel, writeVox };

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main();
}
