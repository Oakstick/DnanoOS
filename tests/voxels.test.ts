import { describe, expect, it } from 'vitest';
import {
  BAT, BAT_RIG, buildRiggedMesh, buildVoxelMesh, POPOLON, POPOLON_RIG, SKELETON, SKELETON_RIG, SLIME, spriteFromModel, VoxelModel,
} from '../src/render3d/voxels';

function voxelCount(model: VoxelModel): number {
  let n = 0;
  for (const layer of model.layers) for (const row of layer) for (const c of row) if (c !== '.' && c !== ' ') n++;
  return n;
}

function riggedCount(model: VoxelModel, rig: Parameters<typeof buildRiggedMesh>[1]): number {
  const rigged = buildRiggedMesh(model, rig, { size: 0.1 });
  let n = 0;
  for (const group of rigged.parts.values()) {
    group.traverse((obj) => {
      const mesh = obj as { count?: number; isInstancedMesh?: boolean };
      if (mesh.isInstancedMesh && mesh.count) n += mesh.count;
    });
  }
  return n;
}

describe('voxel models', () => {
  it('every model has consistent layer shapes', () => {
    for (const model of [POPOLON, BAT, SKELETON, SLIME]) {
      const depth = model.layers[0].length;
      const width = model.layers[0][0].length;
      for (const layer of model.layers) {
        expect(layer).toHaveLength(depth);
        for (const row of layer) expect(row).toHaveLength(width);
      }
    }
  });

  it('builds one cube per voxel', () => {
    const mesh = buildVoxelMesh(SLIME, { size: 0.1 });
    expect(mesh.count).toBe(voxelCount(SLIME));
  });

  it('rigs keep every voxel and expose the expected joints', () => {
    expect(riggedCount(POPOLON, POPOLON_RIG)).toBe(voxelCount(POPOLON));
    expect(riggedCount(SKELETON, SKELETON_RIG)).toBe(voxelCount(SKELETON));
    expect(riggedCount(BAT, BAT_RIG)).toBe(voxelCount(BAT));

    const hero = buildRiggedMesh(POPOLON, POPOLON_RIG, { size: 0.1 });
    expect([...hero.parts.keys()].sort()).toEqual(['body', 'head', 'leftLeg', 'rightLeg', 'shieldArm', 'swordArm'].sort());
    const bat = buildRiggedMesh(BAT, BAT_RIG, { size: 0.1 });
    expect(bat.parts.get('leftWing')!.children).toHaveLength(1);
    expect(bat.parts.get('rightWing')!.children).toHaveLength(1);
  });

  it('the HUD portrait is the front view of Popolon', () => {
    const sprite = spriteFromModel(POPOLON);
    expect(sprite.width).toBe(9);
    expect(sprite.height).toBe(POPOLON.layers.length);
    expect(sprite.pixels).toHaveLength(sprite.height);
    const filled = sprite.pixels.flat().filter((c) => c !== null).length;
    expect(filled).toBeGreaterThan(40);
    // Top row is the helmet tip and sword point; bottom row is boots.
    expect(sprite.pixels[sprite.height - 1].some((c) => c === 0x6b3f1d)).toBe(true);
    // Eyes are dark voxels on the front face of the head.
    expect(sprite.pixels.flat()).toContain(0x1a1a24);
  });

  it('every rigged part actually claims voxels', () => {
    for (const [model, rig] of [[POPOLON, POPOLON_RIG], [SKELETON, SKELETON_RIG], [BAT, BAT_RIG]] as const) {
      const rigged = buildRiggedMesh(model, rig, { size: 0.1 });
      for (const def of rig) {
        expect(rigged.parts.get(def.name)!.children.length, `${def.name} is empty`).toBe(1);
      }
    }
  });
});
