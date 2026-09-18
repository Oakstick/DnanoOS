import { Rng } from '../engine/Rng';
import { Grid } from '../world/Grid';
import { CARDINALS, Dir, DIR_VECTORS, OPPOSITE, Tile, Vec2 } from '../world/types';
import { Drone } from './Drone';
import { Mover } from './Mover';

export type PredatorKind = 'hunter' | 'ambusher' | 'flanker' | 'lurker';
export type PredatorMode = 'nest' | 'leaving' | 'scatter' | 'chase' | 'frenzied' | 'eaten';
export type GlobalMode = 'scatter' | 'chase';

export interface PredatorContext {
  grid: Grid;
  drone: Drone;
  hunter: Predator | null;
  nestCenter: Vec2;
  nestExit: Vec2;
  globalMode: GlobalMode;
  frenzyActive: boolean;
  rng: Rng;
}

export interface PredatorSpec {
  kind: PredatorKind;
  name: string;
  color: string;
  scatterTarget: Vec2;
  exitDelay: number;
}

/** Seconds a hunter keeps homing after the drone goes silent. */
export const HEARING_MEMORY = 1.5;

export class Predator extends Mover {
  readonly kind: PredatorKind;
  readonly name: string;
  readonly color: string;
  readonly scatterTarget: Vec2;
  mode: PredatorMode = 'nest';
  baseSpeed: number;
  exitTimer: number;
  target: Vec2; // exposed for the debug overlay
  private readonly initialExitDelay: number;

  constructor(spec: PredatorSpec, start: Vec2, baseSpeed: number) {
    super(start, baseSpeed);
    this.kind = spec.kind;
    this.name = spec.name;
    this.color = spec.color;
    this.scatterTarget = spec.scatterTarget;
    this.baseSpeed = baseSpeed;
    this.exitTimer = spec.exitDelay;
    this.initialExitDelay = spec.exitDelay;
    this.target = { ...spec.scatterTarget };
  }

  get isDangerous(): boolean {
    return this.mode === 'scatter' || this.mode === 'chase';
  }

  get isEdible(): boolean {
    return this.mode === 'frenzied';
  }

  reset(at: Vec2): void {
    this.teleport(at);
    this.mode = 'nest';
    this.exitTimer = this.initialExitDelay;
  }

  /** Called when a chem-charge is vented. Predators in or leaving the nest are unaffected. */
  enterFrenzy(grid: Grid): void {
    if (this.mode !== 'scatter' && this.mode !== 'chase') return;
    this.mode = 'frenzied';
    this.reverse(grid);
  }

  exitFrenzy(globalMode: GlobalMode): void {
    if (this.mode === 'frenzied') this.mode = globalMode;
  }

  /** Called when the drone consumes this predator. */
  consume(): void {
    this.mode = 'eaten';
  }

  update(dt: number, ctx: PredatorContext): void {
    const { grid } = ctx;

    if (this.mode === 'nest') {
      this.exitTimer -= dt;
      if (this.exitTimer > 0) return;
      this.mode = 'leaving';
    }
    // Follow the global scatter/chase schedule while free-roaming.
    if (this.mode === 'scatter' || this.mode === 'chase') this.mode = ctx.globalMode;

    this.target = this.computeTarget(ctx);
    this.speed = this.baseSpeed * this.speedMultiplier(grid);
    const allowNest = this.mode === 'eaten' || this.mode === 'leaving';
    this.step(dt, grid, (tile, current) => this.decideAtCenter(tile, current, ctx, allowNest), allowNest);
  }

  /** Runs at each tile center: applies nest transitions, then picks a direction. */
  private decideAtCenter(tile: Vec2, current: Dir, ctx: PredatorContext, allowNest: boolean): Dir {
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

    let options = CARDINALS.filter((d) => grid.canMove(tile, d, allowNest));
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

  private computeTarget(ctx: PredatorContext): Vec2 {
    const { drone } = ctx;
    switch (this.mode) {
      case 'eaten': return ctx.nestCenter;
      case 'leaving': return ctx.nestExit;
      case 'scatter': return this.scatterTarget;
      case 'frenzied': return this.scatterTarget; // unused; frenzied movement is random
      case 'nest': return this.scatterTarget;
      case 'chase': break;
    }
    const dp = drone.pos;
    switch (this.kind) {
      case 'hunter':
        // Blind but acute hearing: homes on the last place the drone made noise.
        return drone.heardAge < HEARING_MEMORY ? drone.heardPos : this.scatterTarget;
      case 'ambusher': {
        const v = DIR_VECTORS[drone.dir];
        return { x: dp.x + v.x * 4, y: dp.y + v.y * 4 };
      }
      case 'flanker': {
        const anchor = ctx.hunter ? ctx.hunter.pos : this.pos;
        const v = DIR_VECTORS[drone.dir];
        const ahead = { x: dp.x + v.x * 2, y: dp.y + v.y * 2 };
        return { x: ahead.x + (ahead.x - anchor.x), y: ahead.y + (ahead.y - anchor.y) };
      }
      case 'lurker':
        return Grid.manhattan(this.pos, dp) > 8 ? dp : this.scatterTarget;
    }
  }
}

export function defaultPredatorSpecs(grid: Grid): PredatorSpec[] {
  const w = grid.width - 1;
  const h = grid.height - 1;
  return [
    { kind: 'hunter', name: 'Gulper', color: '#ff4d6d', scatterTarget: { x: w - 2, y: 0 }, exitDelay: 0 },
    { kind: 'ambusher', name: 'Lantern', color: '#ff9de2', scatterTarget: { x: 2, y: 0 }, exitDelay: 2.5 },
    { kind: 'flanker', name: 'Siphon', color: '#5ef2ff', scatterTarget: { x: w, y: h }, exitDelay: 5 },
    { kind: 'lurker', name: 'Barbel', color: '#ffb347', scatterTarget: { x: 0, y: h }, exitDelay: 7.5 },
  ];
}
