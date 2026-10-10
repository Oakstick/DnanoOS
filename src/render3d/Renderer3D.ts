import * as THREE from 'three';
import { Bomb } from '../entities/Bomb';
import { Diamond } from '../entities/Diamond';
import { Enemy } from '../entities/Enemy';
import { Hero } from '../entities/Hero';
import { Potion } from '../entities/Potion';
import { Projectile } from '../entities/Projectile';
import { ItemKind, ITEMS, RangedKind, WEAPONS } from '../entities/Weapons';
import { Frenzy } from '../systems/Frenzy';
import { Puddle, PUDDLE_SECONDS } from '../systems/Puddles';
import { Grid } from '../world/Grid';
import { CARDINALS, Dir, DIR_VECTORS, Tile } from '../world/types';
import {
  ARROW, BAT, BAT_RIG, BOMB, BOOMERANG, bottleModel, buildRiggedMesh, buildVoxelField, buildVoxelMesh, CRESCENT, MEDKIT,
  DIAMOND, FIREBALL, hideFieldRange, LANTERN, NUGGET, PartDef, Placement, POPOLON, POPOLON_RIG, RiggedMesh, SKELETON,
  SKELETON_RIG, SLIME, TREE_LOW, VoxelField, VoxelModel,
} from './voxels';

/** Moonlit forest palette. Walls are undergrowth and trees; higher floors read a little lighter. */
const COLORS = {
  background: 0x06100b,
  stoneA: 0x2b5a2a, // undergrowth, two shades
  stoneB: 0x234a22,
  floorA: 0x3a6a32, // grass
  floorB: 0x33602c,
  pit: 0x050406,
  pitRim: 0x2a1e12,
  ramp: 0x5a4324, // packed earth
  hollow: 0x3a2a18,
  dirt: 0x4a3620,
  torchLight: 0xd6ff5a, // lanterns
  heroLight: 0xffe6b0,
  moon: 0xbfd4ff,
  stun: 0x6a8cff,
  stunWarn: 0xffffff,
  slash: 0xbfefff,
  shield: 0x7fd7ff,
  invincible: 0xffd84a,
  remedy: 0x4cf06a,
  mote: 0xe4ff6a, // fireflies
};

/** World units per elevation level. */
export const STEP = 0.55;
const MOTE_COUNT = 220;
/** Share of wall tiles facing a road that grow a tree. Each tree is about 260 triangles, so this is the main geometry knob. */
const TREE_DENSITY = 0.5;
/**
 * Static geometry is built per chunk of this many tiles so the camera frustum
 * culls whole chunks behind and beside Popolon instead of drawing the map.
 */
const CHUNK = 32;

/**
 * Light budget. Every point light in the scene costs every lit pixel, so
 * instead of one light per torch or crystal, small pools of lights are moved
 * each frame to the instances nearest the hero. Pool sizes never change,
 * which also avoids shader recompiles.
 */
const LIGHT_POOL = { torches: 3, crystals: 3, shrines: 2, effects: 2 };

export type CameraMode = 'third' | 'top';

/**
 * Over-the-shoulder chase camera. The mouse aims it and Popolon runs wherever
 * it points, so the camera leads and the hero follows. It orbits a pivot set
 * a little to his right, which keeps him out of the middle of the screen and
 * leaves the view down the corridor clear.
 */
const CHASE = {
  distance: 3.4,
  shoulder: 0.3,
  /**
   * The camera orbits a point at Popolon's chest, which keeps him just below
   * the middle of the screen. Pitching down lifts the camera rather than
   * tilting past him, and that is what clears the undergrowth: the corridors
   * are a single tile wide, so a camera at his own height would spend the
   * game behind a hedge. At rest it sits about a tile and a half up.
   */
  pivotHeight: 1.7,
  restPitch: -0.44,
  lookAhead: 2.2,
  minPitch: -1.2,
  maxPitch: 0.1,
  minDistance: 0.9,
  posLerp: 16,
  /** Radians of turn per pixel of mouse movement. */
  sensitivity: 0.0026,
  /** Without the mouse captured the view drifts back behind him at this rate. */
  followLerp: 3.5,
};
/** Undergrowth is at least this tall, which is what the camera has to squeeze past. */
const WALL_TOP = 0.7;
/** How quickly character models ease into a new heading (per second). */
const TURN_RATE = { hero: 14, enemy: 10 };
/** Top-down follow camera: high and slightly south of the hero, looking north. */
const TOP = { height: 8.2, back: 5, lookAhead: 0.4, lerp: 5 };
const VOXEL = 0.075;
const BLAST_SECONDS = 0.45;
const DEBRIS_SECONDS = 1.4;
const GRAVITY = 14;
const REASSEMBLE_START = 0.7;

export interface SceneState {
  grid: Grid;
  hero: Hero;
  enemies: Enemy[];
  frenzy: Frenzy;
  time: number;
  bombs: Bomb[];
  projectiles: Projectile[];
  diamonds: Diamond<string>[];
  potions: Potion[];
  puddles: Puddle[];
}

interface EnemyView {
  rig: RiggedMesh;
  group: THREE.Group;
  material: THREE.MeshStandardMaterial;
  light: THREE.PointLight;
  baseColor: THREE.Color;
  hover: number;
  kind: Enemy['kind'];
  phase: number;
  /** Smoothed model heading. */
  yaw: number;
  rest: Map<string, THREE.Vector3>;
  floor: Map<string, number>;
  scatter: Map<string, { vel: THREE.Vector3; spin: THREE.Vector3 }> | null;
}

interface ProjectileView {
  group: THREE.Group;
  inner: THREE.Object3D;
  spin: number;
}

interface DiamondView {
  group: THREE.Group;
  crystal: THREE.InstancedMesh;
  material: THREE.MeshStandardMaterial;
}

interface Debris {
  mesh: THREE.Mesh;
  vel: THREE.Vector3;
  spin: THREE.Vector3;
  life: number;
  floorY: number;
}

interface Blast {
  light: THREE.PointLight;
  sphere: THREE.Mesh;
  life: number;
  seconds: number;
  intensity: number;
  scale: number;
}

/**
 * Three.js renderer for the voxel dungeon. Grid x maps to world x, grid y to
 * world z, and floor elevation to world y in steps of STEP. The 2D canvas
 * stays on top for HUD, labels, and screens.
 */
export class Renderer3D {
  readonly renderer: THREE.WebGLRenderer;
  readonly scene = new THREE.Scene();
  readonly camera: THREE.PerspectiveCamera;
  cameraMode: CameraMode = 'third';
  /** Where the chase camera looks, in world radians. The mouse drives these. */
  camYaw = 0;
  camPitch = CHASE.restPitch;
  /** True while the mouse is captured; otherwise the view trails his heading. */
  mouseLook = false;

  private levelGroup = new THREE.Group();
  private grid: Grid | null = null;
  private coins: VoxelField | null = null;
  private charges = new Map<string, { mesh: THREE.Object3D }>();
  private motes: THREE.Points | null = null;
  private moteBase: Float32Array = new Float32Array(0);
  private torches = new Map<string, THREE.Object3D>();
  private torchLights: THREE.PointLight[] = [];
  private remedyLights: THREE.PointLight[] = [];
  private shrineLights: THREE.PointLight[] = [];
  private effectLights: THREE.PointLight[] = [];
  private effectCursor = 0;
  private rockIndex = new Map<string, { mesh: THREE.InstancedMesh; index: number }>();
  private treeFields = new Map<string, VoxelField>();
  private bombViews = new Map<Bomb, THREE.InstancedMesh>();
  private projectileViews = new Map<Projectile, ProjectileView>();
  private diamondViews = new Map<Diamond<string>, DiamondView>();
  private potionViews = new Map<Potion, THREE.Group>();
  private puddleMeshes = new Map<string, THREE.Mesh>();
  private debris: Debris[] = [];
  private blasts: Blast[] = [];
  private hero = new THREE.Group();
  private heroRig!: RiggedMesh;
  private heroPhase = 0;
  private heroYaw = 0;
  private heroLight!: THREE.PointLight;
  private slash!: THREE.Mesh;
  private shieldBubble!: THREE.Mesh;
  private shadow!: THREE.Mesh;
  private enemyViews: EnemyView[] = [];
  private camPos = new THREE.Vector3();
  private camLook = new THREE.Vector3();
  private initialized = false;

  constructor(readonly canvas: HTMLCanvasElement) {
    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: true, powerPreference: 'high-performance' });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 1.5));
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;
    this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
    this.renderer.toneMappingExposure = 1.2;

    this.camera = new THREE.PerspectiveCamera(50, 1, 0.1, 80);
    this.scene.background = new THREE.Color(COLORS.background);
    this.scene.fog = new THREE.Fog(COLORS.background, 14, 34);
    this.scene.add(this.levelGroup);

    // Moonlit forest: cool sky, dark mossy ground, a blue key light; lanterns do the warm work.
    this.scene.add(new THREE.HemisphereLight(0x6f8fa8, 0x14260f, 1.0));
    this.scene.add(new THREE.AmbientLight(0x2c3a2a, 0.7));
    const moon = new THREE.DirectionalLight(COLORS.moon, 0.9);
    moon.position.set(-6, 14, 8);
    this.scene.add(moon);

    this.buildLightPools();
    this.buildHero();
    this.resize();
  }

  private buildLightPools(): void {
    const make = (n: number, color: number, distance: number) => {
      const pool: THREE.PointLight[] = [];
      for (let i = 0; i < n; i++) {
        const light = new THREE.PointLight(color, 0, distance, 2);
        this.scene.add(light);
        pool.push(light);
      }
      return pool;
    };
    this.torchLights = make(LIGHT_POOL.torches, COLORS.torchLight, 7);
    this.remedyLights = make(LIGHT_POOL.crystals, COLORS.remedy, 5);
    this.shrineLights = make(LIGHT_POOL.shrines, 0xffffff, 5);
    this.effectLights = make(LIGHT_POOL.effects, 0xffffff, 6);
  }

  // ---------------------------------------------------------------- build

  private buildHero(): void {
    this.heroRig = buildRiggedMesh(POPOLON, POPOLON_RIG, { size: VOXEL });
    this.hero.add(this.heroRig.root);

    this.heroLight = new THREE.PointLight(COLORS.heroLight, 8, 6, 2);
    this.heroLight.position.set(0, 1.2, 0);
    this.hero.add(this.heroLight);

    const slashGeo = new THREE.RingGeometry(0.45, 0.85, 16, 1, Math.PI * 0.2, Math.PI * 0.6);
    const slashMat = new THREE.MeshBasicMaterial({ color: COLORS.slash, transparent: true, opacity: 0, side: THREE.DoubleSide });
    this.slash = new THREE.Mesh(slashGeo, slashMat);
    this.slash.rotation.x = -Math.PI / 2;
    this.slash.rotation.z = Math.PI;
    this.slash.position.set(0, 0.35, 0.2);
    this.hero.add(this.slash);

    this.shieldBubble = new THREE.Mesh(
      new THREE.SphereGeometry(0.58, 16, 12),
      new THREE.MeshBasicMaterial({ color: COLORS.shield, transparent: true, opacity: 0.22, side: THREE.DoubleSide, depthWrite: false }),
    );
    this.shieldBubble.position.y = 0.5;
    this.shieldBubble.visible = false;
    this.hero.add(this.shieldBubble);

    // A drop shadow that stays on the floor while he jumps, so height reads clearly.
    this.shadow = new THREE.Mesh(
      new THREE.CircleGeometry(0.32, 12),
      new THREE.MeshBasicMaterial({ color: 0x000000, transparent: true, opacity: 0.45, depthWrite: false }),
    );
    this.shadow.rotation.x = -Math.PI / 2;
    this.scene.add(this.shadow);

    this.scene.add(this.hero);
  }

  buildLevel(grid: Grid, enemies: Enemy[], diamonds: Diamond<string>[], potions: Potion[] = []): void {
    this.disposeLevel();
    this.grid = grid;
    const floorY = (x: number, y: number) => grid.elevation(x, y) * STEP;

    const chunkOf = (x: number, y: number) => `${Math.floor(x / CHUNK)},${Math.floor(y / CHUNK)}`;
    const groupByChunk = <T extends { x: number; y: number }>(items: T[]): Map<string, T[]> => {
      const map = new Map<string, T[]>();
      for (const it of items) {
        const k = chunkOf(it.x, it.y);
        const list = map.get(k);
        if (list) list.push(it);
        else map.set(k, [it]);
      }
      return map;
    };
    const m = new THREE.Matrix4();
    const color = new THREE.Color();

    // Floors: raised tiles are boxes standing on the ground, so their sides show as cliffs. One mesh per chunk.
    const floorSpots: { x: number; y: number; t: Tile }[] = [];
    grid.forEach((x, y, t) => {
      if (t !== Tile.Rock && t !== Tile.Pit && !grid.isRamp(x, y)) floorSpots.push({ x, y, t });
    });
    const floorGeo = new THREE.BoxGeometry(1, 1, 1);
    const floorMat = new THREE.MeshLambertMaterial({ color: 0xffffff, flatShading: true });
    for (const spots of groupByChunk(floorSpots).values()) {
      const floor = new THREE.InstancedMesh(floorGeo, floorMat, spots.length);
      spots.forEach((s, i) => {
        const h = floorY(s.x, s.y);
        const thick = 0.1 + h;
        m.makeScale(1, thick, 1).setPosition(s.x, h - thick / 2, s.y);
        floor.setMatrixAt(i, m);
        const checker = (s.x + s.y) % 2 === 0;
        color.setHex(s.t === Tile.Nest ? COLORS.hollow : checker ? COLORS.floorA : COLORS.floorB);
        color.offsetHSL(0, 0, (hash(s.x, s.y) - 0.5) * 0.05 + grid.elevation(s.x, s.y) * 0.05);
        floor.setColorAt(i, color);
      });
      floor.instanceMatrix.needsUpdate = true;
      if (floor.instanceColor) floor.instanceColor.needsUpdate = true;
      this.levelGroup.add(floor);
    }

    // Ramps: three ascending slabs toward the higher neighbor, all in one instanced mesh.
    const slabs: THREE.Matrix4[] = [];
    grid.forEach((x, y) => {
      if (!grid.isRamp(x, y)) return;
      const low = grid.elevation(x, y);
      let up: Dir | null = null;
      for (const d of CARDINALS) {
        const n = grid.neighbor({ x, y }, d);
        if (grid.get(n.x, n.y) !== Tile.Rock && !grid.isRamp(n.x, n.y) && grid.elevation(n.x, n.y) > low) up = d;
      }
      if (!up) {
        // Interior ramp tile: lean the way a neighboring ramp tile does, else default right.
        up = 'right';
        for (const d of CARDINALS) {
          const n = grid.neighbor({ x, y }, d);
          if (grid.isRamp(n.x, n.y) && grid.elevation(n.x, n.y) > low) up = d;
        }
      }
      const v = DIR_VECTORS[up];
      for (let i = 0; i < 3; i++) {
        const top = low * STEP + ((i + 1) / 3) * STEP;
        const thick = top + 0.1;
        slabs.push(new THREE.Matrix4().makeScale(v.x ? 1 / 3 : 1, thick, v.y ? 1 / 3 : 1)
          .setPosition(x + v.x * ((i - 1) / 3), (top - 0.1) / 2, y + v.y * ((i - 1) / 3)));
      }
    });
    if (slabs.length) {
      const rampMesh = new THREE.InstancedMesh(
        new THREE.BoxGeometry(1, 1, 1),
        new THREE.MeshLambertMaterial({ color: COLORS.ramp, flatShading: true }),
        slabs.length,
      );
      slabs.forEach((mat, i) => rampMesh.setMatrixAt(i, mat));
      rampMesh.instanceMatrix.needsUpdate = true;
      this.levelGroup.add(rampMesh);
    }

    // Pits: a dark floor far below with a rim, two instanced meshes.
    const pitTiles: { x: number; y: number }[] = [];
    grid.forEach((x, y, t) => {
      if (t === Tile.Pit) pitTiles.push({ x, y });
    });
    if (pitTiles.length) {
      const bottoms = new THREE.InstancedMesh(new THREE.BoxGeometry(1, 0.1, 1), new THREE.MeshLambertMaterial({ color: COLORS.pit }), pitTiles.length);
      const rims = new THREE.InstancedMesh(new THREE.BoxGeometry(1.02, 0.06, 1.02), new THREE.MeshLambertMaterial({ color: COLORS.pitRim, flatShading: true }), pitTiles.length);
      pitTiles.forEach((p, i) => {
        bottoms.setMatrixAt(i, new THREE.Matrix4().makeTranslation(p.x, -0.9, p.y));
        rims.setMatrixAt(i, new THREE.Matrix4().makeTranslation(p.x, -0.03, p.y));
      });
      bottoms.instanceMatrix.needsUpdate = true;
      rims.instanceMatrix.needsUpdate = true;
      this.levelGroup.add(bottoms, rims);
    }

    // Walls: undergrowth blocks rising from the tallest neighboring floor, with trees on the road-facing edge.
    const rockSpots: { x: number; y: number; tall: number }[] = [];
    const lanternSpots: { x: number; y: number }[] = [];
    const treeSpots: (Placement & { y0: number })[] = [];
    const facesRoad = (x: number, y: number) =>
      grid.get(x - 1, y) !== Tile.Rock || grid.get(x + 1, y) !== Tile.Rock || grid.get(x, y - 1) !== Tile.Rock || grid.get(x, y + 1) !== Tile.Rock;
    grid.forEach((x, y, t) => {
      if (t !== Tile.Rock) return;
      const base = floorY(x, y);
      const edge = facesRoad(x, y);
      const tall = base + (edge ? 0.7 + hash(x, y) * 0.25 : 0.9 + hash(x, y) * 0.3);
      rockSpots.push({ x, y, tall });
      if (!edge) return;
      if (grid.isWalkable(x, y + 1) && hash(x * 3, y * 7) < 0.06) {
        lanternSpots.push({ x, y });
      } else if (hash(x * 5 + 1, y * 3 + 2) < TREE_DENSITY) {
        treeSpots.push({ key: key(x, y), x, y: tall - 0.05, y0: y, z: y, rotY: hash(x, y) * Math.PI * 2, scale: 0.8 + hash(y, x) * 0.6 });
      }
    });
    const rockGeo = new THREE.BoxGeometry(1, 1, 1);
    const rockMat = new THREE.MeshLambertMaterial({ color: 0xffffff, flatShading: true });
    for (const spots of groupByChunk(rockSpots).values()) {
      const rocks = new THREE.InstancedMesh(rockGeo, rockMat, spots.length);
      spots.forEach((s, i) => {
        m.makeScale(1, s.tall, 1).setPosition(s.x, s.tall / 2, s.y);
        rocks.setMatrixAt(i, m);
        const shade = hash(s.x + 11, s.y + 5);
        color.setHex(shade < 0.5 ? COLORS.stoneA : COLORS.stoneB).offsetHSL(0, 0, (shade - 0.5) * 0.08);
        rocks.setColorAt(i, color);
        this.rockIndex.set(key(s.x, s.y), { mesh: rocks, index: i });
      });
      rocks.instanceMatrix.needsUpdate = true;
      if (rocks.instanceColor) rocks.instanceColor.needsUpdate = true;
      this.levelGroup.add(rocks);
    }

    // Trees: one instanced field per chunk so the frustum can drop the woods behind the camera.
    const treesByChunk = new Map<string, Placement[]>();
    for (const t of treeSpots) {
      const k = chunkOf(t.x, t.y0);
      const list = treesByChunk.get(k);
      if (list) list.push(t);
      else treesByChunk.set(k, [t]);
    }
    for (const [chunk, spots] of treesByChunk) {
      const field = buildVoxelField(TREE_LOW, spots, { size: 0.17, cheap: true });
      this.treeFields.set(chunk, field);
      this.levelGroup.add(field.mesh);
    }

    for (const spot of lanternSpots) {
      const lantern = buildVoxelMesh(LANTERN, { size: 0.09, emissive: 0.8, emissiveColor: COLORS.torchLight, cheap: true });
      lantern.position.set(spot.x, floorY(spot.x, spot.y) + 0.5, spot.y + 0.5);
      this.levelGroup.add(lantern);
      this.torches.set(key(spot.x, spot.y), lantern);
    }

    // Nuggets: one instanced field, hidden per tile as they are collected.
    const coinSpots: Placement[] = [];
    grid.forEach((x, y, t) => {
      if (t === Tile.Nodule) coinSpots.push({ key: key(x, y), x, y: floorY(x, y), z: y, rotY: hash(x, y) * Math.PI * 2, scale: 0.9 + hash(y, x) * 0.4 });
    });
    this.coins = buildVoxelField(NUGGET, coinSpots, { size: 0.11, emissive: 0.35, emissiveColor: 0xf2c53d, cheap: true });
    this.levelGroup.add(this.coins.mesh);

    grid.forEach((x, y, t) => {
      if (t === Tile.Charge) {
        const cross = buildVoxelMesh(MEDKIT, { size: 0.1, emissive: 0.75, emissiveColor: COLORS.remedy });
        cross.position.set(x, floorY(x, y) + 0.3, y);
        this.levelGroup.add(cross);
        this.charges.set(key(x, y), { mesh: cross });
      }
    });

    for (const e of enemies) this.addEnemy(e);

    for (const d of diamonds) {
      const group = new THREE.Group();
      group.position.set(d.tile.x, floorY(d.tile.x, d.tile.y), d.tile.y);
      const pedestal = new THREE.Mesh(
        new THREE.CylinderGeometry(0.34, 0.4, 0.12, 8),
        new THREE.MeshStandardMaterial({ color: d.kind === 'weapon' ? 0x3b4a6a : 0x5a4a2a, roughness: 0.8, flatShading: true }),
      );
      pedestal.position.y = 0.06;
      group.add(pedestal);
      const crystal = buildVoxelMesh(DIAMOND, { size: 0.1, emissive: 0.8, emissiveColor: 0xffffff });
      crystal.position.y = 0.45;
      group.add(crystal);
      this.levelGroup.add(group);
      this.diamondViews.set(d, { group, crystal, material: crystal.material as THREE.MeshStandardMaterial });
    }

    for (const p of potions) {
      const group = new THREE.Group();
      group.position.set(p.tile.x, floorY(p.tile.x, p.tile.y) + 0.05, p.tile.y);
      group.add(buildVoxelMesh(bottleModel(p.def.hex), { size: 0.1, emissive: 0.5, emissiveColor: p.def.hex }));
      this.levelGroup.add(group);
      this.potionViews.set(p, group);
    }

    // Fireflies drifting over the forest.
    this.moteBase = new Float32Array(MOTE_COUNT * 3);
    const positions = new Float32Array(MOTE_COUNT * 3);
    for (let i = 0; i < MOTE_COUNT; i++) {
      this.moteBase[i * 3] = Math.random() * grid.width;
      this.moteBase[i * 3 + 1] = 0.6 + Math.random() * 1.4;
      this.moteBase[i * 3 + 2] = Math.random() * grid.height;
    }
    positions.set(this.moteBase);
    const moteGeo = new THREE.BufferGeometry();
    moteGeo.setAttribute('position', new THREE.BufferAttribute(positions, 3));
    const moteMat = new THREE.PointsMaterial({
      color: COLORS.mote, size: 0.14, sizeAttenuation: true, transparent: true, opacity: 0.8,
      blending: THREE.AdditiveBlending, depthWrite: false,
    });
    this.motes = new THREE.Points(moteGeo, moteMat);
    this.levelGroup.add(this.motes);

    this.initialized = false;
  }

  /** Adds a view for an enemy; call again for enemies that arrive mid-level. Order must match the game's list. */
  addEnemy(e: Enemy): void {
    const model = modelFor(e);
    const rig = buildRiggedMesh(model.model, model.rig, { size: model.size });
    const group = new THREE.Group();
    group.add(rig.root);
    const baseColor = new THREE.Color(e.color);
    const light = new THREE.PointLight(baseColor, 2.5, 4, 2);
    light.position.y = 0.6;
    group.add(light);
    this.levelGroup.add(group);
    const rest = new Map<string, THREE.Vector3>();
    const floorY = new Map<string, number>();
    for (const [name, part] of rig.parts) {
      rest.set(name, part.position.clone());
      const mesh = part.children[0] as THREE.InstancedMesh | undefined;
      let minY = 0;
      if (mesh) {
        mesh.computeBoundingBox();
        minY = mesh.boundingBox?.min.y ?? 0;
      }
      floorY.set(name, -minY + 0.02);
    }
    this.enemyViews.push({
      rig, group, material: rig.material, light, baseColor, hover: model.hover, kind: e.kind,
      phase: Math.random() * 6, yaw: 0, rest, floor: floorY, scatter: null,
    });
  }

  private disposeLevel(): void {
    this.levelGroup.traverse((obj) => {
      const mesh = obj as THREE.Mesh;
      if (mesh.geometry) mesh.geometry.dispose();
      const mat = mesh.material as THREE.Material | THREE.Material[] | undefined;
      if (Array.isArray(mat)) mat.forEach((mm) => mm.dispose());
      else mat?.dispose();
    });
    this.levelGroup.clear();
    this.grid = null;
    this.coins = null;
    this.motes = null;
    this.charges.clear();
    this.torches.clear();
    this.rockIndex.clear();
    this.treeFields.clear();
    this.enemyViews = [];
    this.diamondViews.clear();
    this.potionViews.clear();
    this.puddleMeshes.clear();
    for (const mesh of this.bombViews.values()) this.scene.remove(mesh);
    this.bombViews.clear();
    for (const view of this.projectileViews.values()) this.scene.remove(view.group);
    this.projectileViews.clear();
    for (const d of this.debris) this.scene.remove(d.mesh);
    this.debris = [];
    for (const b of this.blasts) {
      this.scene.remove(b.sphere);
      b.light.intensity = 0;
    }
    this.blasts = [];
  }

  private floorAt(x: number, y: number): number {
    return this.grid ? this.grid.elevation(Math.round(x), Math.round(y)) * STEP : 0;
  }

  /** Collapse a wall block after a blast and patch the floor beneath it. */
  removeRock(x: number, y: number): void {
    const k = key(x, y);
    const entry = this.rockIndex.get(k);
    if (entry) {
      const m = new THREE.Matrix4().makeScale(0.0001, 0.0001, 0.0001).setPosition(x, -10, y);
      entry.mesh.setMatrixAt(entry.index, m);
      entry.mesh.instanceMatrix.needsUpdate = true;
      this.rockIndex.delete(k);
    }
    const torch = this.torches.get(k);
    if (torch) {
      this.levelGroup.remove(torch);
      this.torches.delete(k);
    }
    // A tree standing on the blasted block falls with it.
    const field = this.treeFields.get(`${Math.floor(x / CHUNK)},${Math.floor(y / CHUNK)}`);
    if (field) hideFieldRange(field, k);
    const h = this.floorAt(x, y);
    const thick = 0.1 + h;
    const patch = new THREE.Mesh(
      new THREE.BoxGeometry(1, thick, 1),
      new THREE.MeshStandardMaterial({ color: COLORS.dirt, roughness: 1, flatShading: true }),
    );
    patch.position.set(x, h - thick / 2, y);
    this.levelGroup.add(patch);
  }

  removeCollectible(x: number, y: number): void {
    const k = key(x, y);
    if (this.coins) hideFieldRange(this.coins, k);
    const c = this.charges.get(k);
    if (c) {
      this.levelGroup.remove(c.mesh);
      this.charges.delete(k);
    }
  }

  addBomb(bomb: Bomb): void {
    const mesh = buildVoxelMesh(BOMB, { size: 0.09 });
    mesh.position.set(bomb.tile.x, this.floorAt(bomb.tile.x, bomb.tile.y), bomb.tile.y);
    this.scene.add(mesh);
    this.bombViews.set(bomb, mesh);
  }

  removeBomb(bomb: Bomb): void {
    const mesh = this.bombViews.get(bomb);
    if (!mesh) return;
    this.scene.remove(mesh);
    mesh.geometry.dispose();
    (mesh.material as THREE.Material).dispose();
    this.bombViews.delete(bomb);
  }

  addProjectile(p: Projectile): void {
    const group = new THREE.Group();
    const inner = new THREE.Group();
    let spin = 0;
    const color = new THREE.Color(p.def.color).getHex();
    switch (p.kind) {
      case 'boomerang':
        inner.add(buildVoxelMesh(BOOMERANG, { size: 0.07 }));
        spin = 14;
        break;
      case 'fireball': {
        inner.add(buildVoxelMesh(FIREBALL, { size: 0.09, emissive: 1.2, emissiveColor: color }));
        const light = new THREE.PointLight(color, 5, 4, 2);
        light.position.y = 0.15;
        inner.add(light);
        spin = 6;
        break;
      }
      case 'crescent':
        inner.add(buildVoxelMesh(CRESCENT, { size: 0.08, emissive: 0.9, emissiveColor: color }));
        break;
      default:
        inner.add(buildVoxelMesh(ARROW, { size: 0.08 }));
        break;
    }
    group.add(inner);
    group.position.set(p.pos.x, this.floorAt(p.pos.x, p.pos.y) + 0.45, p.pos.y);
    group.rotation.y = dirToRotation(p.dir);
    this.scene.add(group);
    this.projectileViews.set(p, { group, inner, spin });
  }

  removeProjectile(p: Projectile): void {
    const view = this.projectileViews.get(p);
    if (!view) return;
    this.scene.remove(view.group);
    view.group.traverse((obj) => {
      const mesh = obj as THREE.Mesh;
      mesh.geometry?.dispose();
      (mesh.material as THREE.Material | undefined)?.dispose?.();
    });
    this.projectileViews.delete(p);
  }

  /** Small light pop where a shot lands or connects. */
  spark(x: number, y: number, color = 0xfff1c0): void {
    this.burst(x, y, { color, intensity: 14, radius: 3, seconds: 0.18, scale: 0.6, startRadius: 0.12 });
  }

  private burst(
    x: number,
    y: number,
    o: { color: number; intensity: number; radius: number; seconds: number; scale: number; startRadius: number },
  ): void {
    const h = this.floorAt(x, y);
    const light = this.effectLights[this.effectCursor++ % this.effectLights.length];
    light.color.setHex(o.color);
    light.intensity = o.intensity;
    light.distance = o.radius;
    light.position.set(x, h + 0.8, y);
    const sphere = new THREE.Mesh(
      new THREE.SphereGeometry(o.startRadius, 12, 8),
      new THREE.MeshBasicMaterial({ color: o.color, transparent: true, opacity: 0.9 }),
    );
    sphere.position.set(x, h + 0.5, y);
    this.scene.add(sphere);
    const blast: Blast = { light, sphere, life: o.seconds, seconds: o.seconds, intensity: o.intensity, scale: o.scale };
    light.userData.owner = blast;
    this.blasts.push(blast);
  }

  /** Flash, shockwave sphere, and flying stone chunks at a blast site. */
  explode(x: number, y: number, brokenTiles: number): void {
    this.burst(x, y, { color: 0xffb060, intensity: 120, radius: 9, seconds: BLAST_SECONDS, scale: 3.5, startRadius: 0.5 });
    const h = this.floorAt(x, y);
    const count = 10 + brokenTiles * 4;
    const geo = new THREE.BoxGeometry(0.16, 0.16, 0.16);
    for (let i = 0; i < count; i++) {
      const shade = Math.random();
      // Torn undergrowth: leaves and clods of earth.
      const mat = new THREE.MeshStandardMaterial({ color: shade < 0.6 ? COLORS.stoneA : COLORS.dirt, flatShading: true });
      const mesh = new THREE.Mesh(geo, mat);
      mesh.position.set(x + (Math.random() - 0.5) * 0.6, h + 0.4 + Math.random() * 0.5, y + (Math.random() - 0.5) * 0.6);
      const a = Math.random() * Math.PI * 2;
      const s = 2 + Math.random() * 3;
      this.scene.add(mesh);
      this.debris.push({
        mesh,
        vel: new THREE.Vector3(Math.cos(a) * s, 4 + Math.random() * 5, Math.sin(a) * s),
        spin: new THREE.Vector3(Math.random() * 8, Math.random() * 8, Math.random() * 8),
        life: DEBRIS_SECONDS * (0.7 + Math.random() * 0.5),
        floorY: h + 0.08,
      });
    }
  }

  // --------------------------------------------------------------- update

  setVisible(visible: boolean): void {
    this.canvas.style.display = visible ? 'block' : 'none';
  }

  resize(): void {
    const w = window.innerWidth;
    const h = window.innerHeight;
    this.renderer.setSize(w, h, false);
    this.camera.aspect = w / h;
    this.camera.updateProjectionMatrix();
  }

  update(dt: number, s: SceneState): void {
    const { hero, enemies, frenzy, time, grid } = s;
    this.grid = grid;
    this.updateEffects(dt, time, s);

    // Hero pose: stride cycle, facing, damage blink, sword swing, boons, jump arc.
    const hp = hero.pos;
    this.heroPhase += dt * hero.motion * (hero.hasted ? 17 : 13);
    const bob = hero.airborne ? 0 : hero.motion * Math.abs(Math.cos(this.heroPhase)) * 0.05;
    this.hero.position.set(hp.x, hero.altitude * STEP + bob, hp.y);
    // Ease into the new heading rather than snapping; fast enough to feel responsive.
    this.heroYaw = lerpAngle(this.heroYaw, dirToRotation(hero.facing), 1 - Math.exp(-TURN_RATE.hero * dt));
    this.hero.rotation.y = this.heroYaw;
    this.animateHero(hero);
    const blink = hero.invulnerable && !hero.invincible && Math.floor(time * 16) % 2 === 0;
    this.heroRig.root.visible = hero.alive && !blink;
    this.heroLight.intensity = hero.alive ? 8 : 0;
    const slashMat = this.slash.material as THREE.MeshBasicMaterial;
    slashMat.opacity = hero.swinging ? Math.sin(hero.swingProgress * Math.PI) * 0.85 : 0;

    // Shadow on the floor under him; smaller and fainter the higher he flies.
    const groundH = this.floorAt(hp.x, hp.y);
    const lift = Math.max(0, hero.altitude * STEP - groundH);
    this.shadow.visible = hero.alive;
    this.shadow.position.set(hp.x, groundH + 0.015, hp.y);
    const sh = Math.max(0.5, 1 - lift * 0.5);
    this.shadow.scale.set(sh, sh, 1);
    (this.shadow.material as THREE.MeshBasicMaterial).opacity = 0.45 * sh;

    if (hero.invincible) {
      this.heroRig.material.emissive.setHex(COLORS.invincible);
      this.heroRig.material.emissiveIntensity = 0.45 + Math.sin(time * 12) * 0.25;
      this.heroLight.color.setHex(COLORS.invincible);
    } else {
      this.heroRig.material.emissiveIntensity = 0;
      this.heroLight.color.setHex(COLORS.heroLight);
    }
    const ghost = hero.invisible;
    this.heroRig.material.transparent = true;
    this.heroRig.material.opacity = ghost ? 0.3 + Math.sin(time * 6) * 0.08 : 1;
    this.heroRig.material.depthWrite = !ghost;
    if (ghost) this.heroLight.intensity = 3;
    this.shieldBubble.visible = hero.alive && hero.shield > 0;
    if (this.shieldBubble.visible) {
      (this.shieldBubble.material as THREE.MeshBasicMaterial).opacity = 0.16 + Math.sin(time * 5) * 0.06 + hero.shield * 0.03;
      this.shieldBubble.rotation.y = time * 0.8;
    }

    enemies.forEach((e, i) => {
      const view = this.enemyViews[i];
      if (!view) return;
      const ep = e.pos;
      const ground = e.flying ? this.floorAt(ep.x, ep.y) : e.elevationOn(grid) * STEP;
      view.group.position.set(ep.x, ground + view.hover + Math.sin(time * 5 + i * 1.7) * 0.05, ep.y);
      if (e.dir !== 'none') {
        view.yaw = lerpAngle(view.yaw, dirToRotation(e.dir), 1 - Math.exp(-TURN_RATE.enemy * dt));
      }
      view.group.rotation.y = view.yaw;
      this.animateEnemy(view, e, dt, time, i);
      const hidden = e.mode === 'eaten' || e.mode === 'nest';
      view.rig.root.visible = !hidden;
      if (e.mode === 'scattered') {
        view.light.intensity = 0.6;
        view.light.color.copy(view.baseColor);
        view.material.color.setHex(0xffffff);
        view.material.emissiveIntensity = 0;
        return;
      }
      if (e.isStunned) {
        const warn = frenzy.warning && Math.floor(time * 8) % 2 === 0;
        view.material.color.setHex(warn ? COLORS.stunWarn : COLORS.stun);
        view.material.emissive.setHex(COLORS.stun);
        view.material.emissiveIntensity = 0.5 + Math.sin(time * 12) * 0.3;
        view.light.color.setHex(COLORS.stun);
      } else {
        view.material.color.setHex(0xffffff);
        view.material.emissiveIntensity = 0;
        view.light.color.copy(view.baseColor);
      }
      view.light.intensity = hidden ? 0 : 2.5;
    });

    if (this.coins) (this.coins.mesh.material as THREE.MeshLambertMaterial).emissiveIntensity = 0.35 + Math.sin(time * 3) * 0.2;
    for (const c of this.charges.values()) {
      c.mesh.rotation.y += dt * 1.5;
      c.mesh.position.y = this.floorAt(c.mesh.position.x, c.mesh.position.z) + 0.15 + Math.sin(time * 3) * 0.05;
    }
    this.assignLightPools(hero, time);
    this.updateCamera(dt, hero);
  }

  private updateEffects(dt: number, time: number, s: SceneState): void {
    for (const [bomb, mesh] of this.bombViews) {
      const mat = mesh.material as THREE.MeshStandardMaterial;
      const on = bomb.isFlashOn(time);
      mat.color.setHex(on ? 0xffffff : 0x9a9a9a);
      mat.emissive.setHex(0xffffff);
      mat.emissiveIntensity = on ? 0.9 : 0;
      if (!s.bombs.includes(bomb)) this.removeBomb(bomb);
    }
    for (const [p, view] of this.projectileViews) {
      if (!s.projectiles.includes(p)) {
        this.removeProjectile(p);
        continue;
      }
      view.group.position.set(p.pos.x, this.floorAt(p.pos.x, p.pos.y) + 0.45, p.pos.y);
      view.group.rotation.y = dirToRotation(p.dir);
      if (view.spin) view.inner.rotation.y += dt * view.spin;
    }

    const live = new Set<string>();
    for (const p of s.puddles) {
      const k = key(p.tile.x, p.tile.y);
      live.add(k);
      let mesh = this.puddleMeshes.get(k);
      if (!mesh) {
        mesh = new THREE.Mesh(
          new THREE.CircleGeometry(0.42, 10),
          new THREE.MeshBasicMaterial({ color: 0x5cff6a, transparent: true, opacity: 0.5, depthWrite: false }),
        );
        mesh.rotation.x = -Math.PI / 2;
        mesh.position.set(p.tile.x, this.floorAt(p.tile.x, p.tile.y) + 0.02, p.tile.y);
        this.levelGroup.add(mesh);
        this.puddleMeshes.set(k, mesh);
      }
      (mesh.material as THREE.MeshBasicMaterial).opacity = 0.55 * Math.min(1, p.life / (PUDDLE_SECONDS * 0.4));
      const wobble = 1 + Math.sin(time * 3 + p.tile.x) * 0.05;
      mesh.scale.set(wobble, 1 / wobble, 1);
    }
    for (const [k, mesh] of this.puddleMeshes) {
      if (live.has(k)) continue;
      this.levelGroup.remove(mesh);
      mesh.geometry.dispose();
      (mesh.material as THREE.Material).dispose();
      this.puddleMeshes.delete(k);
    }

    for (const [p, group] of this.potionViews) {
      group.visible = !p.taken;
      if (!p.taken) {
        group.rotation.y += dt * 1.2;
        group.position.y = this.floorAt(p.tile.x, p.tile.y) + 0.05 + Math.sin(time * 2.5 + p.tile.x) * 0.04;
      }
    }
    for (const [d, view] of this.diamondViews) {
      const showing = d.active;
      view.crystal.visible = showing;
      if (!showing) continue;
      const hex = new THREE.Color(optionColor(d)).getHex();
      view.material.emissive.setHex(hex);
      view.material.color.setHex(hex);
      view.crystal.rotation.y += dt * (2 + d.cycleProgress * 8);
      view.crystal.position.y = 0.45 + Math.sin(time * 2 + d.tile.x) * 0.06;
    }
    if (this.motes) {
      const attr = this.motes.geometry.getAttribute('position') as THREE.BufferAttribute;
      const arr = attr.array as Float32Array;
      for (let i = 0; i < MOTE_COUNT; i++) {
        const b = i * 3;
        arr[b] = this.moteBase[b] + Math.sin(time * 0.5 + i) * 0.4;
        arr[b + 1] = this.moteBase[b + 1] + Math.sin(time * 0.9 + i * 0.7) * 0.2;
        arr[b + 2] = this.moteBase[b + 2] + Math.cos(time * 0.4 + i * 1.3) * 0.4;
      }
      attr.needsUpdate = true;
      (this.motes.material as THREE.PointsMaterial).opacity = 0.6 + Math.sin(time * 2.2) * 0.25;
    }
    for (let i = this.debris.length - 1; i >= 0; i--) {
      const d = this.debris[i];
      d.life -= dt;
      if (d.life <= 0) {
        this.scene.remove(d.mesh);
        (d.mesh.material as THREE.Material).dispose();
        this.debris.splice(i, 1);
        continue;
      }
      d.vel.y -= GRAVITY * dt;
      d.mesh.position.addScaledVector(d.vel, dt);
      if (d.mesh.position.y < d.floorY) {
        d.mesh.position.y = d.floorY;
        d.vel.y *= -0.35;
        d.vel.x *= 0.6;
        d.vel.z *= 0.6;
      }
      d.mesh.rotation.x += d.spin.x * dt;
      d.mesh.rotation.y += d.spin.y * dt;
      d.mesh.rotation.z += d.spin.z * dt;
    }
    for (let i = this.blasts.length - 1; i >= 0; i--) {
      const b = this.blasts[i];
      b.life -= dt;
      const ownsLight = b.light.userData.owner === b;
      if (b.life <= 0) {
        this.scene.remove(b.sphere);
        b.sphere.geometry.dispose();
        (b.sphere.material as THREE.Material).dispose();
        if (ownsLight) b.light.intensity = 0;
        this.blasts.splice(i, 1);
        continue;
      }
      const t = 1 - b.life / b.seconds;
      if (ownsLight) b.light.intensity = b.intensity * (1 - t);
      b.sphere.scale.setScalar(1 + t * b.scale);
      (b.sphere.material as THREE.MeshBasicMaterial).opacity = 0.9 * (1 - t);
    }
  }

  /** Moves each light pool to the pool-sized set of sources nearest the hero. */
  private assignLightPools(hero: Hero, time: number): void {
    const hp = hero.pos;
    const focus = new THREE.Vector3(hp.x, 0, hp.y);

    const torchPoints = [...this.torches.values()].map((l) => l.position);
    nearest(torchPoints, focus, this.torchLights.length).forEach((p, i) => {
      const light = this.torchLights[i];
      light.position.set(p.x, p.y + 0.5, p.z + 0.2);
      light.intensity = 4.5 + Math.sin(time * 3 + i * 2.3) * 0.5 + Math.sin(time * 7 + i) * 0.3;
    });
    for (let i = torchPoints.length; i < this.torchLights.length; i++) this.torchLights[i].intensity = 0;

    const crystalPoints = [...this.charges.values()].map((c) => c.mesh.position);
    nearest(crystalPoints, focus, this.remedyLights.length).forEach((p, i) => {
      const light = this.remedyLights[i];
      light.position.set(p.x, p.y + 0.45, p.z);
      light.intensity = 4 + Math.sin(time * 5) * 1.5;
    });
    for (let i = crystalPoints.length; i < this.remedyLights.length; i++) this.remedyLights[i].intensity = 0;

    const shrines = [...this.diamondViews.entries()].filter(([d]) => d.active);
    const shrinePoints = shrines.map(([, v]) => v.group.position);
    const picked = nearest(shrinePoints, focus, this.shrineLights.length);
    picked.forEach((p, i) => {
      const light = this.shrineLights[i];
      const entry = shrines.find(([, v]) => v.group.position === p);
      if (entry) light.color.set(optionColor(entry[0]));
      light.position.set(p.x, p.y + 0.9, p.z);
      light.intensity = 4 + Math.sin(time * 4) * 1;
    });
    for (let i = picked.length; i < this.shrineLights.length; i++) this.shrineLights[i].intensity = 0;
  }

  /** Legs and arms swing in opposition while moving; the sword arm does the slash; a tuck in the air. */
  private animateHero(hero: Hero): void {
    const stride = Math.sin(this.heroPhase) * hero.motion;
    const p = this.heroRig.parts;
    if (hero.airborne) {
      const t = hero.jumpProgress;
      const tuck = Math.sin(t * Math.PI);
      rotX(p, 'leftLeg', -0.9 * tuck);
      rotX(p, 'rightLeg', -0.9 * tuck);
      rotX(p, 'shieldArm', -0.6 * tuck);
      rotX(p, 'swordArm', 0.4 * tuck);
      rotX(p, 'head', -0.15 * tuck);
      this.heroRig.root.rotation.x = 0.2 * tuck;
      return;
    }
    rotX(p, 'leftLeg', stride * 0.75);
    rotX(p, 'rightLeg', -stride * 0.75);
    rotX(p, 'shieldArm', -stride * 0.35);
    rotX(p, 'swordArm', hero.swinging ? Math.sin(hero.swingProgress * Math.PI) * 1.7 : stride * 0.35);
    rotX(p, 'head', hero.motion * 0.08 + (hero.swinging ? 0.15 : 0));
    this.heroRig.root.rotation.x = hero.motion * (hero.hasted ? 0.14 : 0.07);
  }

  private animateEnemy(view: EnemyView, e: Enemy, dt: number, time: number, i: number): void {
    if (e.mode === 'scattered') {
      this.animateScatter(view, e, dt);
      return;
    }
    if (view.scatter) this.reassemble(view);

    const moving = e.dir !== 'none' && e.mode !== 'nest';
    if (moving) view.phase += dt * e.speed * 2.2;
    const p = view.rig.parts;
    const root = view.rig.root;

    switch (view.kind) {
      case 'skeleton': {
        const s = Math.sin(view.phase);
        rotX(p, 'leftLeg', s * 0.85);
        rotX(p, 'rightLeg', -s * 0.85);
        rotX(p, 'leftArm', -s * 0.5);
        rotX(p, 'rightArm', s * 0.5);
        rotX(p, 'head', Math.sin(view.phase * 2) * 0.1);
        root.rotation.z = Math.sin(view.phase) * 0.06;
        break;
      }
      case 'bat':
      case 'vampire': {
        const beat = time * 15 + i * 2;
        const flap = Math.sin(beat) * 0.8;
        const left = p.get('leftWing');
        const right = p.get('rightWing');
        if (left) left.rotation.z = -flap;
        if (right) right.rotation.z = flap;
        root.rotation.x = moving ? 0.3 : 0.05;
        view.group.position.y += Math.sin(beat) * 0.035;
        break;
      }
      case 'slime': {
        const t = moving ? view.phase : time * 2.5 + i;
        const sq = Math.sin(t);
        root.scale.set(1 - sq * 0.14, 1 + sq * 0.3, 1 - sq * 0.14);
        if (moving) view.group.position.y += Math.max(0, sq) * 0.2;
        root.rotation.x = moving ? 0.12 : 0;
        break;
      }
    }
  }

  /** Bones burst outward and tumble to the floor, lie still, then glide back to their rest pose. */
  private animateScatter(view: EnemyView, e: Enemy, dt: number): void {
    const parts = view.rig.parts;
    const root = view.rig.root;
    root.rotation.set(0, 0, 0);
    root.scale.setScalar(1);

    if (!view.scatter) {
      view.scatter = new Map();
      for (const name of parts.keys()) {
        const a = Math.random() * Math.PI * 2;
        const sp = name === 'body' ? 0.3 : 0.8 + Math.random() * 1.1;
        view.scatter.set(name, {
          vel: new THREE.Vector3(Math.cos(a) * sp, 2.5 + Math.random() * 2.5, Math.sin(a) * sp),
          spin: new THREE.Vector3((Math.random() - 0.5) * 16, (Math.random() - 0.5) * 16, (Math.random() - 0.5) * 16),
        });
      }
    }

    const progress = e.scatterProgress;
    if (progress < REASSEMBLE_START) {
      for (const [name, part] of parts) {
        const st = view.scatter.get(name)!;
        const floorY = view.floor.get(name) ?? 0.05;
        st.vel.y -= GRAVITY * dt;
        part.position.addScaledVector(st.vel, dt);
        if (part.position.y < floorY) {
          part.position.y = floorY;
          st.vel.y *= -0.3;
          st.vel.x *= 0.5;
          st.vel.z *= 0.5;
          st.spin.multiplyScalar(0.4);
        }
        part.rotation.x += st.spin.x * dt;
        part.rotation.y += st.spin.y * dt;
        part.rotation.z += st.spin.z * dt;
      }
    } else {
      const t = (progress - REASSEMBLE_START) / (1 - REASSEMBLE_START);
      const k = 1 - Math.exp(-dt * (6 + t * 30));
      for (const [name, part] of parts) {
        const rest = view.rest.get(name)!;
        part.position.lerp(rest, k);
        part.rotation.x *= 1 - k;
        part.rotation.y *= 1 - k;
        part.rotation.z *= 1 - k;
      }
      root.rotation.z = Math.sin(t * 60) * 0.04 * (1 - t);
    }
  }

  private reassemble(view: EnemyView): void {
    for (const [name, part] of view.rig.parts) {
      const rest = view.rest.get(name);
      if (rest) part.position.copy(rest);
      part.rotation.set(0, 0, 0);
    }
    view.scatter = null;
  }

  private updateCamera(dt: number, hero: Hero): void {
    const hp = hero.pos;
    // Follow the floor under him, not the jump arc, so the view does not bounce on every hop.
    const groundY = this.floorAt(hp.x, hp.y);
    const focus = new THREE.Vector3(hp.x, groundY, hp.y);
    let desired: THREE.Vector3;
    let look: THREE.Vector3;
    let posLerp: number;

    if (this.cameraMode === 'third') {
      // Hands off the mouse, the view drifts back behind whichever way he faces.
      if (!this.mouseLook) {
        this.camYaw = lerpAngle(this.camYaw, dirToYaw(hero.facing), 1 - Math.exp(-CHASE.followLerp * dt));
      }
      const cp = Math.cos(this.camPitch);
      const dir = new THREE.Vector3(cp * Math.cos(this.camYaw), Math.sin(this.camPitch), cp * Math.sin(this.camYaw));
      const right = new THREE.Vector3(-Math.sin(this.camYaw), 0, Math.cos(this.camYaw));
      const pivot = focus.clone()
        .add(new THREE.Vector3(0, CHASE.pivotHeight, 0))
        .addScaledVector(right, CHASE.shoulder);
      desired = pivot.clone().addScaledVector(dir, -this.clearBehind(pivot, dir));
      look = pivot.clone().addScaledVector(dir, CHASE.lookAhead);
      posLerp = CHASE.posLerp;
    } else {
      desired = focus.clone().add(new THREE.Vector3(0, TOP.height, TOP.back));
      look = focus.clone().add(new THREE.Vector3(0, 0, -TOP.lookAhead));
      posLerp = TOP.lerp;
    }

    if (!this.initialized) {
      this.camPos.copy(desired);
      this.camLook.copy(look);
      this.initialized = true;
    } else {
      const k = 1 - Math.exp(-posLerp * dt);
      this.camPos.lerp(desired, k);
      this.camLook.lerp(look, k);
    }
    this.camera.position.copy(this.camPos);
    this.camera.lookAt(this.camLook);
  }

  /** Turns the camera by a mouse movement in pixels. */
  look(dx: number, dy: number): void {
    this.camYaw += dx * CHASE.sensitivity;
    const p = this.camPitch - dy * CHASE.sensitivity;
    this.camPitch = Math.max(CHASE.minPitch, Math.min(CHASE.maxPitch, p));
  }

  /** Points the camera along a grid direction, for a fresh life or dungeon. */
  aimAlong(dir: Dir): void {
    this.camYaw = dirToYaw(dir);
    this.camPitch = CHASE.restPitch;
  }

  /**
   * How far back the camera can sit before undergrowth comes between it and
   * Popolon. Walls are whole tiles, so the ray is walked against the grid
   * rather than the scene, which costs nothing and never snags on a leaf.
   */
  private clearBehind(pivot: THREE.Vector3, dir: THREE.Vector3): number {
    if (!this.grid) return CHASE.distance;
    const steps = 14;
    for (let i = 1; i <= steps; i++) {
      const d = (CHASE.distance * i) / steps;
      const x = Math.round(pivot.x - dir.x * d);
      const z = Math.round(pivot.z - dir.z * d);
      const y = pivot.y - dir.y * d;
      if (this.grid.get(x, z) !== Tile.Rock) continue;
      if (y > this.grid.elevation(x, z) * STEP + WALL_TOP) continue;
      return Math.max(CHASE.minDistance, d - CHASE.distance / steps);
    }
    return CHASE.distance;
  }

  render(): void {
    this.renderer.render(this.scene, this.camera);
  }

  /** World point to window pixel coordinates, or null if behind the camera. */
  project(x: number, y: number, z: number): { x: number; y: number } | null {
    const v = new THREE.Vector3(x, y, z).project(this.camera);
    if (v.z > 1) return null;
    return { x: ((v.x + 1) / 2) * window.innerWidth, y: ((1 - v.y) / 2) * window.innerHeight };
  }

  /** Floor height in world units at a tile, for overlay projection. */
  floorHeight(x: number, y: number): number {
    return this.floorAt(x, y);
  }
}

// ---------------------------------------------------------------- helpers

function key(x: number, y: number): string {
  return `${x},${y}`;
}

/** Camera yaw for a facing: forward = (cos yaw, 0, sin yaw) in world XZ, so 0 = grid right, pi/2 = grid down. */
export function dirToYaw(d: Dir): number {
  const v = DIR_VECTORS[d === 'none' ? 'down' : d];
  return Math.atan2(v.y, v.x);
}

function lerpAngle(a: number, b: number, t: number): number {
  let d = (b - a) % (Math.PI * 2);
  if (d > Math.PI) d -= Math.PI * 2;
  if (d < -Math.PI) d += Math.PI * 2;
  return a + d * t;
}

/** Models face +z (grid "down"); rotate so they face the movement direction. */
export function dirToRotation(d: Dir): number {
  switch (d) {
    case 'down': return 0;
    case 'up': return Math.PI;
    case 'right': return Math.PI / 2;
    case 'left': return -Math.PI / 2;
    default: return 0;
  }
}

/** Display color of what a shrine is currently offering. */
export function optionColor(d: Diamond<string>): string {
  return d.kind === 'weapon' ? WEAPONS[d.current as RangedKind].color : ITEMS[d.current as ItemKind].color;
}

/** Display name of what a shrine is currently offering. */
export function optionName(d: Diamond<string>): string {
  return d.kind === 'weapon' ? WEAPONS[d.current as RangedKind].name : ITEMS[d.current as ItemKind].name;
}

/** The `count` points closest to `focus`, nearest first. */
function nearest(points: THREE.Vector3[], focus: THREE.Vector3, count: number): THREE.Vector3[] {
  if (points.length <= count) return points;
  return points
    .map((p) => ({ p, d: (p.x - focus.x) * (p.x - focus.x) + (p.z - focus.z) * (p.z - focus.z) }))
    .sort((a, b) => a.d - b.d)
    .slice(0, count)
    .map((e) => e.p);
}

function modelFor(e: Enemy): { model: VoxelModel; rig: PartDef[]; size: number; hover: number } {
  switch (e.kind) {
    case 'skeleton': return { model: SKELETON, rig: SKELETON_RIG, size: 0.07, hover: 0 };
    case 'bat': return { model: BAT, rig: BAT_RIG, size: 0.07, hover: 0.55 };
    case 'vampire': return { model: BAT, rig: BAT_RIG, size: 0.08, hover: 0.65 };
    case 'slime': return { model: SLIME, rig: [], size: 0.1, hover: 0 };
  }
}

function rotX(parts: Map<string, THREE.Group>, name: string, angle: number): void {
  const part = parts.get(name);
  if (part) part.rotation.x = angle;
}

function hash(x: number, y: number): number {
  let h = (x * 374761393 + y * 668265263) | 0;
  h = ((h ^ (h >>> 13)) * 1274126177) | 0;
  return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
}
