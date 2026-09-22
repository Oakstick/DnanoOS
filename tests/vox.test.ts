import { describe, expect, it } from 'vitest';
// @ts-expect-error - plain JS tool module, no type declarations
import { defaultPalette, parseVox, toModel, writeVox } from '../tools/vox.mjs';
import { BAT, POPOLON, SKELETON, SLIME, VoxelModel } from '../src/render3d/voxels';

/** Every occupied cell of a text model as "x,row,layer" -> colour, with empty edges trimmed. */
function cells(model: VoxelModel): Map<string, number> {
  const raw: { x: number; row: number; layer: number; c: number }[] = [];
  model.layers.forEach((rows, layer) => rows.forEach((row, r) => {
    for (let x = 0; x < row.length; x++) {
      const key = row[x];
      if (key === '.' || key === ' ') continue;
      raw.push({ x, row: r, layer, c: model.palette[key] });
    }
  }));
  const minX = Math.min(...raw.map((v) => v.x));
  const minRow = Math.min(...raw.map((v) => v.row));
  const minLayer = Math.min(...raw.map((v) => v.layer));
  return new Map(raw.map((v) => [`${v.x - minX},${v.row - minRow},${v.layer - minLayer}`, v.c]));
}

/** Round-trips a model out to .vox bytes and back into the text format. */
function roundTrip(model: VoxelModel): VoxelModel {
  const parsed = parseVox(writeVox(model));
  const built = toModel(parsed.models[0], parsed.palette, 'ymin');
  return { palette: Object.fromEntries(built.palette), layers: built.layers };
}

const CHARACTERS: [string, VoxelModel][] = [
  ['POPOLON', POPOLON], ['SKELETON', SKELETON], ['BAT', BAT], ['SLIME', SLIME],
];

describe('MagicaVoxel bridge', () => {
  it('writes a file MagicaVoxel recognises: VOX magic, one model, one palette', () => {
    const buf = writeVox(POPOLON);
    expect(buf.toString('ascii', 0, 4)).toBe('VOX ');
    expect(buf.readInt32LE(4)).toBe(150);
    const parsed = parseVox(buf);
    expect(parsed.models).toHaveLength(1);
    expect(parsed.palette).toHaveLength(256);
  });

  it('sizes the .vox model to the text model, with z up and y deep', () => {
    const { models } = parseVox(writeVox(POPOLON));
    const { size } = models[0];
    expect([size.x, size.y, size.z]).toEqual([9, 5, 14]); // 9 wide, 5 deep, 14 tall
  });

  it.each(CHARACTERS)('round-trips %s through .vox without losing a voxel', (_name, model) => {
    const before = cells(model);
    const after = cells(roundTrip(model));
    expect(after.size).toBe(before.size);
    expect([...after.entries()].sort()).toEqual([...before.entries()].sort());
  });

  it("points the model at MagicaVoxel's camera: Popolon's eyes sit on the front face", () => {
    const parsed = parseVox(writeVox(POPOLON));
    const model = parsed.models[0];
    const eyes = model.voxels.filter((v: { i: number }) => parsed.palette[v.i - 1] === 0x1a1a24);
    expect(eyes).toHaveLength(2); // exactly the two dark eye voxels
    const ys = model.voxels.map((v: { y: number }) => v.y);
    const front = Math.min(...ys);
    const back = Math.max(...ys);
    // MagicaVoxel's default camera looks along +y, so the face must be at low y.
    for (const eye of eyes) expect(eye.y).toBeLessThan((front + back) / 2);
    expect(eyes.every((e: { z: number }) => e.z === 10)).toBe(true); // the head layer, z up
  });

  it('reads MagicaVoxel default-palette files: 216 colour cube then four ramps', () => {
    const pal = defaultPalette();
    expect(pal).toHaveLength(256);
    expect(pal[0]).toBe(0xffffff);
    expect(pal[215]).toBe(0x000000); // last of the 6x6x6 cube
    expect(pal[255]).toBe(0x111111); // darkest grey of the final ramp
  });
});
