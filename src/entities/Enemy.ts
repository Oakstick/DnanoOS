import { Rng } from '../engine/Rng';
import { stepToward } from '../systems/Pathing';
import { Grid } from '../world/Grid';
import { CARDINALS, Dir, DIR_VECTORS, OPPOSITE, Tile, Vec2 } from '../world/types';
import { Hero } from './Hero';
import { Mover } from './Mover';

export type EnemyKind = 'skeleton' | 'bat' | 'vampire' | 'slime';
/**
 * `frenzied` is the stunned state triggered by a power crystal.
 * `scattered` is a slain skeleton lying in pieces before it reassembles.
 */
export type EnemyMode = 'nest' | 'leaving' | 'scatter' | 'chase' | 'frenzied' | 'eaten' | 'scattered';
export type GlobalMode = 'scatter' | 'chase';

export interface EnemyContext {
  grid: Grid;
  hero: Hero;
  skeleton: Enemy | null;
  nestCenter: Vec2;
  nestExit: Vec2;
  globalMode: GlobalMode;
  frenzyActive: boolean;
  /** The hero is invisible: nobody can sense him, everyone falls back to scattering. */
  heroHidden?: boolean;
  rng: Rng;
}

export interface EnemySpec {
  kind: EnemyKind;
  name: string;
  color: string;
  scatterTarget: Vec2;
  exitDelay: number;
  speedFactor: number;
}

/** Seconds a skeleton keeps homing after the hero stops making footsteps. */
export const HEARING_MEMORY = 1.5;
/** Seconds a slain skeleton lies in pieces before pulling itself back together. */
export const REASSEMBLE_SECONDS = 3;

export class Enemy extends Mover {
  readonly kind: EnemyKind;
  readonly name: string;
  readonly color: string;
  readonly scatterTarget: Vec2;
  readonly speedFactor: number;
  mode: EnemyMode = 'nest';
  baseSpeed: number;
  exitTimer: number;
  scatterTimer = 0;
  target: Vec2; // exposed for the debug overlay
  private readonly initialExitDelay: number;

  constructor(spec: EnemySpec, start: Vec2, baseSpeed: number) {
    super(start, baseSpeed);
    this.kind = spec.kind;
    this.name = spec.name;
    this.color = spec.color;
    this.scatterTarget = spec.scatterTarget;
    this.speedFactor = spec.speedFactor;
    this.baseSpeed = baseSpeed;
    this.exitTimer = spec.exitDelay;
    this.initialExitDelay = spec.exitDelay;
    this.target = { ...spec.scatterTarget };
    this.flying = spec.kind === 'bat' || spec.kind === 'vampire';
  }

  /** Free-roaming and able to hurt the hero. */
  get isDangerous(): boolean {
    return this.mode === 'scatter' || this.mode === 'chase';
  }

  /** Stunned by a power crystal: harmless and worth chain points. */
  get isStunned(): boolean {
    return this.mode === 'frenzied';
  }

  /** Can be hit by the sword, an arrow, or a blast. */
  get isTargetable(): boolean {
    return this.isDangerous || this.isStunned;
  }

  /** 0..1 through the bone-pile phase; 0 when not scattered. */
  get scatterProgress(): number {
    return this.mode === 'scattered' ? 1 - Math.max(0, this.scatterTimer) / REASSEMBLE_SECONDS : 0;
  }

  reset(at: Vec2): void {
    this.teleport(at);
    this.mode = 'nest';
    this.exitTimer = this.initialExitDelay;
    this.scatterTimer = 0;
  }

  /** Called when a power crystal is taken. Enemies in or leaving the nest are unaffected. */
  enterFrenzy(grid: Grid): void {
    if (this.mode !== 'scatter' && this.mode !== 'chase') return;
    this.mode = 'frenzied';
    this.reverse(grid);
  }

  exitFrenzy(globalMode: GlobalMode): void {
    if (this.mode === 'frenzied') this.mode = globalMode;
  }

  /**
   * Slain by the hero. Skeletons collapse in place and reassemble after a
   * few seconds; everything else returns to the crypt to respawn.
   */
  kill(): void {
    if (this.kind === 'skeleton') {
      this.teleport(this.occupied);
      this.mode = 'scattered';
      this.scatterTimer = REASSEMBLE_SECONDS;
      return;
    }
    this.mode = 'eaten';
  }

  update(dt: number, ctx: EnemyContext): void {
    const { grid } = ctx;

    if (this.mode === 'scattered') {
      this.scatterTimer -= dt;
      if (this.scatterTimer <= 0) this.mode = ctx.frenzyActive ? 'frenzied' : ctx.globalMode;
      return;
    }
    if (this.mode === 'nest') {
      this.exitTimer -= dt;
      if (this.exitTimer > 0) return;
      this.mode = 'leaving';
    }
    if (this.mode === 'scatter' || this.mode === 'chase') this.mode = ctx.globalMode;

    this.target = this.computeTarget(ctx);
    this.speed = this.baseSpeed * this.speedFactor * this.speedMultiplier(grid);
    const allowNest = this.mode === 'eaten' || this.mode === 'leaving';
    this.step(dt, grid, (tile, current) => this.decideAtCenter(tile, current, ctx, allowNest), allowNest);
  }

  /** Runs at each tile center: applies nest transitions, then picks a direction. */
  private decideAtCenter(tile: Vec2, current: Dir, ctx: EnemyContext, allowNest: boolean): Dir {
    const { grid } = ctx;
    let mayReverse = false;

    if (this.mode === 'leaving' && tile.x === ctx.nestExit.x && tile.y === ctx.nestExit.y) {
      this.mode = ctx.frenzyActive ? 'frenzied' : ctx.globalMode;
      this.target = this.computeTarget(ctx);
      allowNest = false;
    } else if (this.mode === 'eaten' && grid.get(tile.x, tile.y) === Tile.Nest) {
      this.mode = 'leaving';
      this.target = ctx.nestExit;
      mayReverse = true;
    }

    // Going home is the one trip that must always succeed, so it is routed
    // rather than eyeballed: the hollow has a single mouth and a greedy walker
    // can orbit the block outside it without ever turning away to go around.
    if (this.mode === 'eaten') {
      const step = stepToward(grid, tile, this.target, this.flying);
      if (step) return step;
    }

    let options = CARDINALS.filter((d) => grid.canMove(tile, d, allowNest, this.flying));
    if (!mayReverse) {
      const noReverse = options.filter((d) => d !== OPPOSITE[current]);
      if (noReverse.length > 0) options = noReverse;
    }
    if (!allowNest) {
      options = options.filter((d) => {
        const n = grid.neighbor(tile, d);
        return grid.get(n.x, n.y) !== Tile.Nest;
      });
    }
    if (options.length === 0) return current;
    if (this.mode === 'frenzied') return ctx.rng.pick(options);

    let best = options[0];
    let bestD = Infinity;
    for (const d of options) {
      const n = grid.neighbor(tile, d);
      const dist = Grid.distSq(n, this.target);
      if (dist < bestD) {
        bestD = dist;
        best = d;
      }
    }
    return best;
  }

  private speedMultiplier(grid: Grid): number {
    if (this.mode === 'eaten') return 1.7;
    if (this.mode === 'frenzied') return 0.55;
    if (this.mode === 'leaving') return 0.7;
    if (grid.get(this.tile.x, this.tile.y) === Tile.Tunnel) return 0.55;
    return 1;
  }

  private computeTarget(ctx: EnemyContext): Vec2 {
    const { hero } = ctx;
    switch (this.mode) {
      case 'eaten': return ctx.nestCenter;
      case 'leaving': return ctx.nestExit;
      case 'scatter': return this.scatterTarget;
      case 'frenzied': return this.scatterTarget; // unused; stunned movement is random
      case 'nest': return this.scatterTarget;
      case 'scattered': return this.scatterTarget;
      case 'chase': break;
    }
    if (ctx.heroHidden) return this.scatterTarget;
    const hp = hero.pos;
    switch (this.kind) {
      case 'skeleton':
        // Eyeless but sharp-eared: homes on the last place it heard footsteps.
        return hero.heardAge < HEARING_MEMORY ? hero.heardPos : this.scatterTarget;
      case 'bat': {
        // Swoops at where the hero is heading.
        const v = DIR_VECTORS[hero.dir];
        return { x: hp.x + v.x * 4, y: hp.y + v.y * 4 };
      }
      case 'vampire': {
        // Flanks: mirrors the skeleton around a point ahead of the hero.
        const anchor = ctx.skeleton ? ctx.skeleton.pos : this.pos;
        const v = DIR_VECTORS[hero.dir];
        const ahead = { x: hp.x + v.x * 2, y: hp.y + v.y * 2 };
        return { x: ahead.x + (ahead.x - anchor.x), y: ahead.y + (ahead.y - anchor.y) };
      }
      case 'slime':
        // Oozes toward the hero when far, slinks back to its corner when close.
        return Grid.manhattan(this.pos, hp) > 8 ? hp : this.scatterTarget;
    }
  }
}

export function defaultEnemySpecs(grid: Grid): EnemySpec[] {
  const w = grid.width - 1;
  const h = grid.height - 1;
  return [
    { kind: 'skeleton', name: 'Skeleton', color: '#e9e9f2', scatterTarget: { x: w - 2, y: 0 }, exitDelay: 0, speedFactor: 1 },
    { kind: 'bat', name: 'Bat', color: '#9a5cff', scatterTarget: { x: 2, y: 0 }, exitDelay: 2.5, speedFactor: 1.12 },
    { kind: 'vampire', name: 'Vampire Bat', color: '#4f6dff', scatterTarget: { x: w, y: h }, exitDelay: 5, speedFactor: 1.05 },
    { kind: 'slime', name: 'Slime', color: '#5cff6a', scatterTarget: { x: 0, y: h }, exitDelay: 7.5, speedFactor: 0.8 },
  ];
}
