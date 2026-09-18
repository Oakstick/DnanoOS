import { DebugOverlay } from '../debug/DebugOverlay';
import { AudioManager } from '../engine/Audio';
import { Input } from '../engine/Input';
import { Rng } from '../engine/Rng';
import { Drone } from '../entities/Drone';
import { defaultPredatorSpecs, GlobalMode, Predator, PredatorContext } from '../entities/Predator';
import { Renderer } from '../render/Renderer';
import { Frenzy } from '../systems/Frenzy';
import { Particles } from '../systems/Particles';
import { POINTS, Score } from '../systems/Score';
import { Grid } from '../world/Grid';
import { parseLevel, ParsedLevel } from '../world/Level';
import { levelFor } from '../world/levels';
import { Tile, Vec2 } from '../world/types';

export type Screen = 'title' | 'ready' | 'playing' | 'paused' | 'dying' | 'levelclear' | 'gameover';

/** Alternating scatter/chase schedule in seconds. Frozen during frenzy. */
const MODE_SCHEDULE: { mode: GlobalMode; seconds: number }[] = [
  { mode: 'scatter', seconds: 7 },
  { mode: 'chase', seconds: 20 },
  { mode: 'scatter', seconds: 7 },
  { mode: 'chase', seconds: 20 },
  { mode: 'scatter', seconds: 5 },
  { mode: 'chase', seconds: Infinity },
];

const READY_SECONDS = 2.2;
const DYING_SECONDS = 1.8;
const CLEAR_SECONDS = 2.5;
const COLLISION_RADIUS_SQ = 0.45 * 0.45;

interface FloatingText {
  x: number;
  y: number;
  text: string;
  life: number;
}

export class Game {
  screen: Screen = 'title';
  level!: ParsedLevel;
  grid!: Grid;
  drone!: Drone;
  predators: Predator[] = [];
  readonly frenzy = new Frenzy();
  readonly score = new Score();
  readonly particles = new Particles();
  readonly audio = new AudioManager();
  readonly debug = new DebugOverlay();
  readonly input: Input;
  readonly renderer: Renderer;
  private rng = new Rng();

  private globalMode: GlobalMode = 'scatter';
  private modeIndex = 0;
  private modeTimer = MODE_SCHEDULE[0].seconds;
  private screenTimer = 0;
  private time = 0;
  private floaters: FloatingText[] = [];
  private nestSlots: Vec2[] = [];

  constructor(canvas: HTMLCanvasElement) {
    this.renderer = new Renderer(canvas);
    this.input = new Input(canvas);
    this.loadLevel(0);
    window.addEventListener('resize', () => this.renderer.resize(this.grid.width, this.grid.height));
  }

  // ---------------------------------------------------------------- setup

  loadLevel(index: number): void {
    this.score.level = index;
    this.level = parseLevel(levelFor(index));
    this.grid = this.level.grid;
    this.renderer.resize(this.grid.width, this.grid.height);

    const c = this.level.nestCenter;
    this.nestSlots = [this.level.predatorSpawn, { x: c.x - 2, y: c.y }, { x: c.x, y: c.y }, { x: c.x + 2, y: c.y }];

    this.drone = new Drone(this.level.droneSpawn, this.level.data.droneSpeed);
    this.predators = defaultPredatorSpecs(this.grid).map(
      (spec, i) => new Predator(spec, this.nestSlots[i] ?? c, this.level.data.predatorSpeed),
    );
    this.resetRound();
  }

  /** Puts everyone back at spawn without touching collected nodules. */
  private resetRound(): void {
    this.drone.respawn(this.level.droneSpawn);
    this.predators.forEach((p, i) => p.reset(this.nestSlots[i] ?? this.level.nestCenter));
    this.modeIndex = 0;
    this.modeTimer = MODE_SCHEDULE[0].seconds;
    this.globalMode = MODE_SCHEDULE[0].mode;
    this.frenzy.cancel();
    this.audio.setFrenzy(false);
    this.particles.clear();
    this.floaters.length = 0;
    this.input.clearDirection();
  }

  private startGame(): void {
    this.score.reset();
    this.loadLevel(0);
    this.enter('ready');
    this.audio.play('start');
  }

  private enter(screen: Screen): void {
    this.screen = screen;
    switch (screen) {
      case 'ready': this.screenTimer = READY_SECONDS; break;
      case 'dying': this.screenTimer = DYING_SECONDS; break;
      case 'levelclear': this.screenTimer = CLEAR_SECONDS; break;
      default: this.screenTimer = 0;
    }
  }

  // --------------------------------------------------------------- update

  update(dt: number): void {
    this.time += dt;
    this.debug.tick(dt);
    this.handleGlobalKeys();

    switch (this.screen) {
      case 'title':
        if (this.input.wasPressed('Enter', 'Space', 'Touch')) {
          this.audio.unlock();
          this.startGame();
        }
        break;
      case 'ready':
        this.screenTimer -= dt;
        if (this.screenTimer <= 0) this.enter('playing');
        break;
      case 'playing':
        this.tick(dt);
        break;
      case 'paused':
        break;
      case 'dying':
        this.screenTimer -= dt;
        this.particles.update(dt);
        if (this.screenTimer <= 0) {
          if (this.score.loseLife()) this.enter('gameover');
          else {
            this.resetRound();
            this.enter('ready');
          }
        }
        break;
      case 'levelclear':
        this.screenTimer -= dt;
        this.particles.update(dt);
        if (this.screenTimer <= 0) {
          this.loadLevel(this.score.level + 1);
          this.enter('ready');
        }
        break;
      case 'gameover':
        if (this.input.wasPressed('Enter', 'Space', 'Touch')) this.enter('title');
        break;
    }
    this.input.flush();
  }

  private handleGlobalKeys(): void {
    if (this.input.wasPressed('F1')) this.debug.enabled = !this.debug.enabled;
    if (this.input.wasPressed('KeyM')) {
      this.audio.unlock();
      this.audio.toggleMute();
    }
    if (this.input.wasPressed('Escape')) {
      if (this.screen === 'playing') this.enter('paused');
      else if (this.screen === 'paused') this.enter('playing');
    }
  }

  private tick(dt: number): void {
    this.updateModeSchedule(dt);

    this.drone.update(dt, this.grid, this.input.direction);
    this.collect();

    if (this.frenzy.update(dt)) {
      this.predators.forEach((p) => p.exitFrenzy(this.globalMode));
      this.audio.setFrenzy(false);
    }

    const hunter = this.predators.find((p) => p.kind === 'hunter') ?? null;
    const ctx: PredatorContext = {
      grid: this.grid,
      drone: this.drone,
      hunter,
      nestCenter: this.level.nestCenter,
      nestExit: this.level.predatorSpawn,
      globalMode: this.globalMode,
      frenzyActive: this.frenzy.active,
      rng: this.rng,
    };
    for (const p of this.predators) p.update(dt, ctx);

    this.resolveCollisions();
    if (this.screen !== 'playing') return;
    this.updateEffects(dt);

    if (this.grid.count(Tile.Nodule) === 0 && this.grid.count(Tile.Charge) === 0) {
      this.audio.play('clear');
      this.audio.setFrenzy(false);
      this.enter('levelclear');
    }
  }

  private updateModeSchedule(dt: number): void {
    if (this.frenzy.active) return;
    this.modeTimer -= dt;
    if (this.modeTimer <= 0 && this.modeIndex < MODE_SCHEDULE.length - 1) {
      this.modeIndex++;
      this.modeTimer = MODE_SCHEDULE[this.modeIndex].seconds;
      this.globalMode = MODE_SCHEDULE[this.modeIndex].mode;
      // Mode change: predators reverse, a classic tell for the player.
      this.predators.forEach((p) => {
        if (p.isDangerous) p.reverse(this.grid);
      });
    }
  }

  private collect(): void {
    const o = this.drone.occupied;
    const got = this.grid.consume(o.x, o.y);
    if (got === Tile.Nodule) {
      this.score.add(POINTS.nodule);
      this.audio.play('nodule');
      this.particles.emit(o.x, o.y, 3, '#cfefff', 1.5, 0.35, 0.08);
    } else if (got === Tile.Charge) {
      this.score.add(POINTS.charge);
      this.audio.play('charge');
      this.audio.setFrenzy(true);
      this.frenzy.start(this.level.data.frenzySeconds);
      this.predators.forEach((p) => p.enterFrenzy(this.grid));
      this.particles.emit(o.x, o.y, 40, '#7dff9a', 5, 0.9, 0.14);
    }
  }

  private resolveCollisions(): void {
    const dp = this.drone.pos;
    for (const p of this.predators) {
      if (p.mode === 'eaten' || p.mode === 'nest' || p.mode === 'leaving') continue;
      const sameTile = p.occupied.x === this.drone.occupied.x && p.occupied.y === this.drone.occupied.y;
      if (!sameTile && Grid.distSq(p.pos, dp) > COLLISION_RADIUS_SQ) continue;

      if (p.isEdible) {
        const value = this.frenzy.consumeValue();
        this.score.add(value);
        p.consume();
        this.audio.play('eat');
        this.particles.emit(p.pos.x, p.pos.y, 30, p.color, 4, 0.8, 0.12);
        this.floaters.push({ x: p.pos.x, y: p.pos.y, text: String(value), life: 1 });
      } else if (p.isDangerous) {
        this.killDrone();
        return;
      }
    }
  }

  private killDrone(): void {
    this.drone.alive = false;
    this.audio.play('death');
    this.audio.setFrenzy(false);
    this.audio.setThreat(0);
    this.frenzy.cancel();
    this.particles.emit(this.drone.pos.x, this.drone.pos.y, 60, '#e8fff6', 4, 1.4, 0.12);
    this.enter('dying');
  }

  private updateEffects(dt: number): void {
    this.particles.update(dt);
    if (this.drone.thrust > 0.3 && Math.random() < dt * 30) {
      const p = this.drone.pos;
      this.particles.emit(p.x, p.y, 1, '#48e0c0', 1, 0.3, 0.06);
    }
    if (Math.random() < dt * 4) {
      this.particles.emitDrift(Math.random() * this.grid.width, this.grid.height - 0.5, 'rgba(180,220,240,0.5)');
    }
    for (let i = this.floaters.length - 1; i >= 0; i--) {
      const f = this.floaters[i];
      f.life -= dt;
      f.y -= dt * 0.8;
      if (f.life <= 0) this.floaters.splice(i, 1);
    }
    // Ambient threat pulse rises as dangerous predators close in.
    let nearest = Infinity;
    for (const p of this.predators) {
      if (p.isDangerous) nearest = Math.min(nearest, Grid.manhattan(p.pos, this.drone.pos));
    }
    this.audio.setThreat(nearest === Infinity ? 0 : Math.max(0, 1 - nearest / 10));
  }

  // --------------------------------------------------------------- render

  render(): void {
    const r = this.renderer;
    r.begin();
    r.drawGrid(this.grid, this.time);
    r.drawParticles(this.particles);
    for (const p of this.predators) r.drawPredator(p, this.frenzy, this.time);
    r.drawDrone(this.drone, this.time);
    for (const f of this.floaters) r.drawFloatingText(f.x, f.y, f.text, Math.min(1, f.life * 2));
    r.drawVignette(this.grid.height);
    r.drawHUD(this.score, this.level.data.name, this.frenzy, this.grid.height);

    this.debug.draw(r, {
      drone: this.drone,
      predators: this.predators,
      globalMode: this.globalMode,
      modeTimer: this.modeTimer,
      fps: this.debug.fps,
      particleCount: this.particles.count,
    });

    switch (this.screen) {
      case 'title':
        r.drawOverlay([
          { text: 'VOID HARVEST', size: 2.2, color: '#48e0c0' },
          { text: 'Descend. Harvest the nodules. Stay silent.', size: 0.8, color: '#9fc4d0' },
          { text: 'Chem-charges flip the hunt: eat them while they glow.', size: 0.8, color: '#9fc4d0' },
          { text: ' ', size: 0.6 },
          { text: 'Enter or tap to descend', size: 1 },
          { text: 'WASD / arrows / swipe to move   Esc pause   M mute   F1 debug', size: 0.6, color: '#6f92a0' },
          { text: `Record ${this.score.high}`, size: 0.7, color: '#6f92a0' },
        ], 0.8);
        break;
      case 'ready':
        r.drawOverlay([
          { text: this.level.data.name.toUpperCase(), size: 1.1, color: '#48e0c0' },
          { text: 'DESCENDING...', size: 0.8, color: '#9fc4d0' },
        ], 0.35);
        break;
      case 'paused':
        r.drawOverlay([{ text: 'PAUSED', size: 1.6 }, { text: 'Esc to resume', size: 0.7, color: '#9fc4d0' }]);
        break;
      case 'levelclear':
        r.drawOverlay([{ text: 'ARTERY HARVESTED', size: 1.4, color: '#7dff9a' }], 0.3);
        break;
      case 'gameover':
        r.drawOverlay([
          { text: 'CRUSHED BY THE DEEP', size: 1.6, color: '#ff4d6d' },
          { text: `Harvest ${this.score.score}`, size: 0.9 },
          { text: `Record ${this.score.high}`, size: 0.8, color: '#9fc4d0' },
          { text: 'Enter or tap to surface', size: 0.8, color: '#6f92a0' },
        ]);
        break;
      default:
        break;
    }
  }
}
