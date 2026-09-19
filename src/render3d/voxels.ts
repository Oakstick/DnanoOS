import * as THREE from 'three';

/**
 * Tiny voxel model format. `layers` run from the bottom of the model to the
 * top; each layer is a list of rows from back (-z) to front (+z); each row is
 * a string of palette keys along x. '.' is empty. Models face +z by default.
 */
export interface VoxelModel {
  palette: Record<string, number>;
  layers: string[][];
}

export interface VoxelMeshOptions {
  /** World size of one voxel. */
  size: number;
  /** Extra emissive strength for the whole model. */
  emissive?: number;
  /** Emissive tint; defaults to white, which washes out saturated palettes. */
  emissiveColor?: number;
  roughness?: number;
  /** Use Lambert shading instead of PBR: far cheaper per pixel, fine for big static fields. */
  cheap?: boolean;
}

export type VoxelMaterial = THREE.MeshStandardMaterial | THREE.MeshLambertMaterial;

interface Voxel {
  x: number;
  y: number;
  z: number;
  c: number;
}

function collectVoxels(model: VoxelModel): { voxels: Voxel[]; width: number; depth: number } {
  const voxels: Voxel[] = [];
  let width = 0;
  let depth = 0;
  model.layers.forEach((rows, y) => {
    depth = Math.max(depth, rows.length);
    rows.forEach((row, z) => {
      width = Math.max(width, row.length);
      for (let x = 0; x < row.length; x++) {
        const key = row[x];
        if (key === '.' || key === ' ') continue;
        const c = model.palette[key];
        if (c === undefined) throw new Error(`voxel palette missing key "${key}"`);
        voxels.push({ x, y, z, c });
      }
    });
  });
  return { voxels, width, depth };
}

function makeMaterial(opts: VoxelMeshOptions): VoxelMaterial {
  if (opts.cheap) {
    return new THREE.MeshLambertMaterial({
      color: 0xffffff,
      flatShading: true,
      emissive: opts.emissiveColor ?? 0xffffff,
      emissiveIntensity: opts.emissive ?? 0,
    });
  }
  return new THREE.MeshStandardMaterial({
    color: 0xffffff,
    roughness: opts.roughness ?? 0.85,
    metalness: 0,
    flatShading: true,
    emissive: opts.emissiveColor ?? 0xffffff,
    emissiveIntensity: opts.emissive ?? 0,
  });
}

/** Instanced cubes for a voxel list, each placed relative to `origin` (in voxel units). */
function instanceVoxels(
  voxels: Voxel[],
  origin: [number, number, number],
  size: number,
  material: VoxelMaterial,
): THREE.InstancedMesh {
  const geo = new THREE.BoxGeometry(size, size, size);
  const mesh = new THREE.InstancedMesh(geo, material, voxels.length);
  const m = new THREE.Matrix4();
  const color = new THREE.Color();
  voxels.forEach((v, i) => {
    m.makeTranslation((v.x - origin[0]) * size, (v.y + 0.5 - origin[1]) * size, (v.z - origin[2]) * size);
    mesh.setMatrixAt(i, m);
    color.setHex(v.c);
    mesh.setColorAt(i, color);
  });
  mesh.instanceMatrix.needsUpdate = true;
  if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
  return mesh;
}

/** Builds an instanced cube mesh for a model, centered on x/z with its base at y = 0. */
export function buildVoxelMesh(model: VoxelModel, opts: VoxelMeshOptions): THREE.InstancedMesh {
  const { voxels, width, depth } = collectVoxels(model);
  return instanceVoxels(voxels, [(width - 1) / 2, 0, (depth - 1) / 2], opts.size, makeMaterial(opts));
}

export interface Placement {
  key: string;
  x: number;
  y: number;
  z: number;
  rotY?: number;
  /** Per-placement scale multiplier. */
  scale?: number;
}

export interface VoxelField {
  mesh: THREE.InstancedMesh;
  /** Instance index range [start, count] for each placement key. */
  ranges: Map<string, [number, number]>;
}

/**
 * Many copies of one model in a single instanced mesh: one draw call for a
 * whole forest of trees or a floor full of coins. Copies can be hidden later
 * by zero-scaling their instance range.
 */
export function buildVoxelField(model: VoxelModel, placements: Placement[], opts: VoxelMeshOptions): VoxelField {
  const { voxels, width, depth } = collectVoxels(model);
  const ox = (width - 1) / 2;
  const oz = (depth - 1) / 2;
  const geo = new THREE.BoxGeometry(opts.size, opts.size, opts.size);
  const mesh = new THREE.InstancedMesh(geo, makeMaterial(opts), voxels.length * placements.length);
  const ranges = new Map<string, [number, number]>();
  const m = new THREE.Matrix4();
  const q = new THREE.Quaternion();
  const pos = new THREE.Vector3();
  const scl = new THREE.Vector3();
  const color = new THREE.Color();
  const up = new THREE.Vector3(0, 1, 0);
  let i = 0;
  for (const p of placements) {
    const s = opts.size * (p.scale ?? 1);
    q.setFromAxisAngle(up, p.rotY ?? 0);
    ranges.set(p.key, [i, voxels.length]);
    for (const v of voxels) {
      const local = new THREE.Vector3((v.x - ox) * s, (v.y + 0.5) * s, (v.z - oz) * s).applyQuaternion(q);
      pos.set(p.x + local.x, p.y + local.y, p.z + local.z);
      scl.setScalar(p.scale ?? 1);
      m.compose(pos, q, scl);
      mesh.setMatrixAt(i, m);
      color.setHex(v.c);
      mesh.setColorAt(i, color);
      i++;
    }
  }
  mesh.instanceMatrix.needsUpdate = true;
  if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
  return { mesh, ranges };
}

/** Hides one placement of a field by collapsing its instances. */
export function hideFieldRange(field: VoxelField, key: string): void {
  const range = field.ranges.get(key);
  if (!range) return;
  const m = new THREE.Matrix4().makeScale(0, 0, 0).setPosition(0, -50, 0);
  for (let i = range[0]; i < range[0] + range[1]; i++) field.mesh.setMatrixAt(i, m);
  field.mesh.instanceMatrix.needsUpdate = true;
  field.ranges.delete(key);
}

// ---------------------------------------------------------------- sprites

export interface Sprite {
  width: number;
  height: number;
  /** Row-major from the top of the model down; null is transparent. */
  pixels: (number | null)[][];
}

/**
 * Front view of a model as a 2D sprite: for each column and height, the
 * frontmost voxel's color. Used for HUD portraits so they always match the
 * in-world character.
 */
export function spriteFromModel(model: VoxelModel): Sprite {
  const { voxels, width } = collectVoxels(model);
  const height = model.layers.length;
  const pixels: (number | null)[][] = [];
  for (let y = height - 1; y >= 0; y--) {
    const row: (number | null)[] = [];
    for (let x = 0; x < width; x++) {
      let best: Voxel | null = null;
      for (const v of voxels) {
        if (v.x === x && v.y === y && (!best || v.z > best.z)) best = v;
      }
      row.push(best ? best.c : null);
    }
    pixels.push(row);
  }
  return { width, height, pixels };
}

// ------------------------------------------------------------------- rigs

/** Inclusive voxel box: [x0, y0, z0], [x1, y1, z1]. */
export type VoxelBox = [[number, number, number], [number, number, number]];

/** A named, independently rotating part of a model, cut from its voxels by boxes. */
export interface PartDef {
  name: string;
  boxes: VoxelBox[];
  /** Rotation pivot in voxel units of the model grid. */
  pivot: [number, number, number];
}

export interface RiggedMesh {
  root: THREE.Group;
  /** Named parts plus 'body' for everything not claimed by a part. */
  parts: Map<string, THREE.Group>;
  material: THREE.MeshStandardMaterial;
}

function inBox(v: Voxel, b: VoxelBox): boolean {
  return v.x >= b[0][0] && v.x <= b[1][0] && v.y >= b[0][1] && v.y <= b[1][1] && v.z >= b[0][2] && v.z <= b[1][2];
}

/**
 * Splits a model into pivoted parts. Each part is a Group positioned at its
 * pivot so rotating the group swings the part around that joint. All parts
 * share one material so tinting the whole character stays a single write.
 */
export function buildRiggedMesh(model: VoxelModel, partDefs: PartDef[], opts: VoxelMeshOptions): RiggedMesh {
  const { voxels, width, depth } = collectVoxels(model);
  const ox = (width - 1) / 2;
  const oz = (depth - 1) / 2;
  // Characters are small and tinted at runtime; they keep the full material.
  const material = makeMaterial({ ...opts, cheap: false }) as THREE.MeshStandardMaterial;
  const root = new THREE.Group();
  const parts = new Map<string, THREE.Group>();

  const buckets = new Map<string, Voxel[]>();
  const body: Voxel[] = [];
  for (const v of voxels) {
    const def = partDefs.find((p) => p.boxes.some((b) => inBox(v, b)));
    if (def) {
      if (!buckets.has(def.name)) buckets.set(def.name, []);
      buckets.get(def.name)!.push(v);
    } else {
      body.push(v);
    }
  }

  const addPart = (name: string, list: Voxel[], pivot: [number, number, number]) => {
    const group = new THREE.Group();
    group.position.set((pivot[0] - ox) * opts.size, pivot[1] * opts.size, (pivot[2] - oz) * opts.size);
    if (list.length > 0) group.add(instanceVoxels(list, pivot, opts.size, material));
    root.add(group);
    parts.set(name, group);
  };

  addPart('body', body, [ox, 0, oz]);
  for (const def of partDefs) addPart(def.name, buckets.get(def.name) ?? [], def.pivot);
  return { root, parts, material };
}

// ------------------------------------------------------------------ models
// Legend is per model. Each layer has the same number of rows; rows have the
// same length. Front of the model is the last row of each layer.

const P = {
  red: 0xd8262e,
  darkRed: 0x8f1a1f,
  blue: 0x2b4fd6,
  skin: 0xf2c9a0,
  white: 0xf4f4f8,
  gold: 0xf2c53d,
  cyan: 0x8fe7ff,
  steel: 0xb8c0cc,
  dark: 0x1a1a24,
  brown: 0x6b3f1d,
  purple: 0x7a3fe0,
  darkPurple: 0x3d1f78,
  bone: 0xe8e8f0,
  eyeRed: 0xff2a2a,
  green: 0x3fe05a,
  lightGreen: 0x9dffab,
  darkGreen: 0x1c7a2c,
  orange: 0xff8a2a,
  yellow: 0xffe066,
};

/** Popolon: 9 wide, 5 deep, 14 tall. Sword raised on his right (+x), shield on his left. */
export const POPOLON: VoxelModel = {
  palette: {
    R: P.red, r: P.darkRed, B: P.blue, S: P.skin, W: P.white, Y: P.gold,
    C: P.cyan, G: P.steel, K: P.dark, D: P.brown,
  },
  layers: [
    // 0-1 boots
    ['.........', '..DD.DD..', '..DD.DD..', '..DD.DD..', '.........'],
    ['.........', '..DD.DD..', '..DD.DD..', '..DD.DD..', '.........'],
    // 2-4 legs
    ['.........', '..BB.BB..', '..BB.BB..', '..BB.BB..', '.........'],
    ['.........', '..BB.BB..', '..BB.BB..', '..BB.BB..', '.........'],
    ['.........', '..BBBBB..', '..BBBBB..', '..BBBBB..', '.........'],
    // 5-8 torso, arms, shield (x=0), sword hilt (x=8)
    ['.........', '.SRRRRRS.', 'GSRRRRRSY', '.SRRRRRS.', 'G........'],
    ['.........', '.RRRRRRR.', 'GRRRRRRRC', '.RRRRRRR.', 'G........'],
    ['.........', '.RRrRrRR.', 'GRRRRRRRC', '.RRRRRRR.', 'G........'],
    ['.........', '.RRRRRRR.', 'GRRRRRRRC', '.RRRRRRR.', 'G........'],
    // 9-11 head with eyes on the front row
    ['.........', '...SSS...', '...SSS..C', '...SSS...', '.........'],
    ['.........', '...SSS...', '...SSS..C', '...KSK...', '.........'],
    ['.........', '...SSS...', '...SSS..C', '...SSS...', '.........'],
    // 12-13 helmet and horns, blade tip
    ['.........', '..RRRRR..', '.YRRRRRYC', '..RRRRR..', '.........'],
    ['.........', '...RRR...', '.Y.RRR.YC', '...RRR...', '.........'],
  ],
};

/** Popolon's joints: legs at the hip, arms at the shoulder, head at the neck. */
export const POPOLON_RIG: PartDef[] = [
  { name: 'leftLeg', boxes: [[[2, 0, 0], [3, 3, 4]]], pivot: [2.5, 4, 2] },
  { name: 'rightLeg', boxes: [[[5, 0, 0], [6, 3, 4]]], pivot: [5.5, 4, 2] },
  { name: 'shieldArm', boxes: [[[0, 5, 0], [1, 8, 4]]], pivot: [1, 8.5, 2] },
  { name: 'swordArm', boxes: [[[7, 5, 0], [7, 8, 4]], [[8, 5, 0], [8, 13, 4]]], pivot: [7, 8.5, 2] },
  { name: 'head', boxes: [[[1, 9, 0], [7, 13, 4]]], pivot: [4, 9, 2] },
];

/** Bat: 11 wide, 3 deep, 4 tall. */
export const BAT: VoxelModel = {
  palette: { P: P.purple, p: P.darkPurple, K: P.dark, Y: P.yellow },
  layers: [
    ['...........', '....pKp....', '...........'],
    ['P.........P', 'PPPPpppPPPP', '....pYp....'],
    ['PP.......PP', '.PPPpppPPP.', '....ppp....'],
    ['...........', '..PP...PP..', '...........'],
  ],
};

/** Bat wings hinge where they meet the body. */
export const BAT_RIG: PartDef[] = [
  { name: 'leftWing', boxes: [[[0, 0, 0], [3, 3, 2]]], pivot: [4, 1.5, 1] },
  { name: 'rightWing', boxes: [[[7, 0, 0], [10, 3, 2]]], pivot: [6, 1.5, 1] },
];

/** Skeleton: 7 wide, 3 deep, 12 tall, club raised on the right. */
export const SKELETON: VoxelModel = {
  palette: { W: P.bone, K: P.dark, E: P.eyeRed, D: P.brown },
  layers: [
    ['.......', '.W...W.', '.......'],
    ['.......', '.W...W.', '.......'],
    ['.......', '.W...W.', '.......'],
    ['.......', '.W...W.', '.......'],
    ['.......', '.WWWWW.', '.......'],
    ['.......', 'W.WWW.D', '.......'],
    ['.......', 'W.W.W.D', '.......'],
    ['.......', 'W.WWW.D', '.......'],
    ['.......', 'W.WWW.D', '.......'],
    ['.......', '..WWW.D', '.......'],
    ['.......', '..WWW..', '..EWE..'],
    ['.......', '..WWW..', '..WWW..'],
  ],
};

/** Skeleton joints: thin legs at the pelvis, arms at the shoulder, skull at the neck. */
export const SKELETON_RIG: PartDef[] = [
  { name: 'leftLeg', boxes: [[[1, 0, 0], [1, 3, 2]]], pivot: [1, 4, 1] },
  { name: 'rightLeg', boxes: [[[5, 0, 0], [5, 3, 2]]], pivot: [5, 4, 1] },
  { name: 'leftArm', boxes: [[[0, 5, 0], [0, 8, 2]]], pivot: [0, 9, 1] },
  { name: 'clubArm', boxes: [[[6, 5, 0], [6, 9, 2]]], pivot: [6, 9.5, 1] },
  { name: 'head', boxes: [[[2, 10, 0], [4, 11, 2]]], pivot: [3, 10, 1] },
];

/** Slime: 7 wide, 7 deep, 5 tall. */
export const SLIME: VoxelModel = {
  palette: { G: P.green, L: P.lightGreen, g: P.darkGreen, K: P.dark },
  layers: [
    ['.ggggg.', 'ggggggg', 'ggggggg', 'ggggggg', 'ggggggg', 'ggggggg', '.ggggg.'],
    ['.GGGGG.', 'GGGGGGG', 'GGGGGGG', 'GGGGGGG', 'GGGGGGG', 'GGGGGGG', '.GGGGG.'],
    ['..GGG..', '.GGGGG.', 'GGGGGGG', 'GGGGGGG', 'GGGGGGG', '.GKGKG.', '..GGG..'],
    ['.......', '..GGG..', '.GGGGG.', '.GGLGG.', '.GGGGG.', '..GGG..', '.......'],
    ['.......', '.......', '..LLL..', '..LLL..', '..LLL..', '.......', '.......'],
  ],
};

/** Gold nugget: a lumpy chunk, 4 wide, 3 deep, 3 tall, brighter on top. */
export const NUGGET: VoxelModel = {
  palette: { Y: P.gold, y: P.yellow, d: 0xb8902a },
  layers: [
    ['.dYd', 'dYYY', '.dY.'],
    ['..Yy', '.YyY', '..Y.'],
    ['....', '..y.', '....'],
  ],
};

/** Power crystal: 3 wide, 3 deep, 5 tall diamond. */
export const CRYSTAL: VoxelModel = {
  palette: { G: P.green, L: P.lightGreen },
  layers: [
    ['...', '.G.', '...'],
    ['.G.', 'GLG', '.G.'],
    ['.L.', 'LLL', '.L.'],
    ['.G.', 'GLG', '.G.'],
    ['...', '.L.', '...'],
  ],
};

/** TNT-style bomb: 5 wide, 5 deep, 6 tall with a fuse on top. */
export const BOMB: VoxelModel = {
  palette: { R: P.red, r: P.darkRed, W: P.white, K: P.dark, G: P.steel },
  layers: [
    ['rRRRr', 'RRRRR', 'RRRRR', 'RRRRR', 'rRRRr'],
    ['RRRRR', 'RRRRR', 'RRRRR', 'RRRRR', 'RRRRR'],
    ['WWWWW', 'WWKWW', 'WKKKW', 'WWKWW', 'WWWWW'],
    ['RRRRR', 'RRRRR', 'RRRRR', 'RRRRR', 'RRRRR'],
    ['rRRRr', 'RRRRR', 'RRRRR', 'RRRRR', 'rRRRr'],
    ['.....', '.....', '..G..', '.....', '.....'],
  ],
};

/** Arrow lying flat: 1 wide, 7 deep, 1 tall. Steel tip at the front (+z), feathers at the back. */
export const ARROW: VoxelModel = {
  palette: { D: P.brown, G: P.steel, W: P.white },
  layers: [[
    'W', 'W', 'D', 'D', 'D', 'D', 'G',
  ]],
};

/** Boomerang lying flat: an L of wood with pale tips, 5 wide, 5 deep, 1 tall. */
export const BOOMERANG: VoxelModel = {
  palette: { D: P.brown, W: P.white },
  layers: [[
    'W....',
    'D....',
    'D....',
    'D....',
    'WDDDW',
  ]],
};

/** Fireball: 3x3x3 with a bright core. */
export const FIREBALL: VoxelModel = {
  palette: { O: P.orange, Y: P.yellow },
  layers: [
    ['.O.', 'OOO', '.O.'],
    ['OOO', 'OYO', 'OOO'],
    ['.O.', 'OOO', '.O.'],
  ],
};

/** Crescent blade lying flat: a wide arc, 7 wide, 3 deep, 1 tall. */
export const CRESCENT: VoxelModel = {
  palette: { C: P.cyan, W: P.white },
  layers: [[
    'C.....C',
    'CC...CC',
    '.CCWCC.',
  ]],
};

/** Potion bottle: 3 wide, 3 deep, 6 tall. Liquid color is filled in per kind. */
export function bottleModel(liquid: number): VoxelModel {
  return {
    palette: { L: liquid, G: 0xcfe8f0, K: P.brown },
    layers: [
      ['.L.', 'LLL', '.L.'],
      ['LLL', 'LLL', 'LLL'],
      ['LLL', 'LLL', 'LLL'],
      ['.G.', 'GLG', '.G.'],
      ['...', '.G.', '...'],
      ['...', '.K.', '...'],
    ],
  };
}

/** Shrine crystal: an 8-tall diamond, 5 wide and deep at the waist. */
export const DIAMOND: VoxelModel = {
  palette: { W: P.white, L: P.cyan },
  layers: [
    ['.....', '.....', '..W..', '.....', '.....'],
    ['.....', '..L..', '.LWL.', '..L..', '.....'],
    ['..L..', '.LLL.', 'LLWLL', '.LLL.', '..L..'],
    ['.LLL.', 'LLLLL', 'LLWLL', 'LLLLL', '.LLL.'],
    ['..L..', '.LLL.', 'LLWLL', '.LLL.', '..L..'],
    ['.....', '..L..', '.LWL.', '..L..', '.....'],
    ['.....', '.....', '..W..', '.....', '.....'],
  ],
};

/** Wall torch: 1 wide, 1 deep, 5 tall. */
export const TORCH: VoxelModel = {
  palette: { D: P.brown, O: P.orange, Y: P.yellow },
  layers: [['D'], ['D'], ['D'], ['O'], ['Y']],
};

/** Hanging lantern on a post: 1 wide, 1 deep, 6 tall, glowing cage on top. */
export const LANTERN: VoxelModel = {
  palette: { D: P.brown, G: 0xd6ff5a, K: P.dark },
  layers: [['D'], ['D'], ['D'], ['D'], ['G'], ['K']],
};

/** Lean forest tree for dense woods: 3 wide, 3 deep, 6 tall, 22 voxels. Use with a larger voxel size. */
export const TREE_LOW: VoxelModel = {
  palette: { T: 0x4a2f16, G: 0x2f6b2a, g: 0x24521f, L: 0x4c8f3c },
  layers: [
    ['...', '.T.', '...'],
    ['...', '.T.', '...'],
    ['.g.', 'gGg', '.g.'],
    ['gGg', 'GGG', 'gGg'],
    ['.G.', 'GLG', '.G.'],
    ['...', '.L.', '...'],
  ],
};

/** Forest tree: 5 wide, 5 deep, 10 tall. Trunk, then a tapered canopy. */
export const TREE: VoxelModel = {
  palette: { T: 0x4a2f16, G: 0x2f6b2a, g: 0x24521f, L: 0x4c8f3c },
  layers: [
    ['.....', '.....', '..T..', '.....', '.....'],
    ['.....', '.....', '..T..', '.....', '.....'],
    ['.....', '.....', '..T..', '.....', '.....'],
    ['.....', '.gGg.', '.GTG.', '.gGg.', '.....'],
    ['.gGg.', 'gGGGg', 'GGTGG', 'gGGGg', '.gGg.'],
    ['.GGG.', 'GGLGG', 'GLGLG', 'GGLGG', '.GGG.'],
    ['.gGg.', 'gGGGg', 'GGGGG', 'gGGGg', '.gGg.'],
    ['.....', '.GGG.', '.GLG.', '.GGG.', '.....'],
    ['.....', '..G..', '.GLG.', '..G..', '.....'],
    ['.....', '.....', '..L..', '.....', '.....'],
  ],
};
