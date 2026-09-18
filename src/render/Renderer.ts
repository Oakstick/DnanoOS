import { Drone } from '../entities/Drone';
import { Predator } from '../entities/Predator';
import { Frenzy } from '../systems/Frenzy';
import { Particles } from '../systems/Particles';
import { Score } from '../systems/Score';
import { Grid } from '../world/Grid';
import { DIR_VECTORS, Tile } from '../world/types';

export const HUD_ROWS = 2;

export const PALETTE = {
  abyss: '#02060c',
  water: '#04121f',
  rock: '#0b2233',
  rockEdge: '#1f5a66',
  coral: '#c94f7c',
  nest: '#170f2a',
  nodule: '#cfefff',
  charge: '#7dff9a',
  drone: '#e8fff6',
  droneAccent: '#48e0c0',
  frenzied: '#3b5bff',
  frenziedWarn: '#e8f0ff',
  text: '#d7ecf2',
  textDim: '#6f92a0',
} as const;

export interface OverlayLine {
  text: string;
  size?: number;
  color?: string;
}

/**
 * Canvas 2D renderer. Owns canvas sizing; everything is drawn in tile units
 * scaled by `tile` pixels. The HUD lives in HUD_ROWS extra rows below the maze.
 */
export class Renderer {
  readonly ctx: CanvasRenderingContext2D;
  tile = 24;
  private cols = 1;
  private rows = 1;
  private dpr = 1;

  constructor(readonly canvas: HTMLCanvasElement) {
    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('Canvas 2D not supported');
    this.ctx = ctx;
  }

  resize(cols: number, rows: number): void {
    this.cols = cols;
    this.rows = rows + HUD_ROWS;
    this.dpr = Math.min(window.devicePixelRatio || 1, 2);
    const availW = window.innerWidth - 16;
    const availH = window.innerHeight - 16;
    this.tile = Math.max(8, Math.floor(Math.min(availW / this.cols, availH / this.rows)));
    const w = this.tile * this.cols;
    const h = this.tile * this.rows;
    this.canvas.style.width = `${w}px`;
    this.canvas.style.height = `${h}px`;
    this.canvas.width = Math.round(w * this.dpr);
    this.canvas.height = Math.round(h * this.dpr);
    this.ctx.setTransform(this.dpr, 0, 0, this.dpr, 0, 0);
  }

  get width(): number {
    return this.tile * this.cols;
  }

  get height(): number {
    return this.tile * this.rows;
  }

  begin(): void {
    this.ctx.fillStyle = PALETTE.abyss;
    this.ctx.fillRect(0, 0, this.width, this.height);
  }

  drawGrid(grid: Grid, time: number): void {
    const { ctx, tile: t } = this;
    ctx.fillStyle = PALETTE.water;
    ctx.fillRect(0, 0, grid.width * t, grid.height * t);

    grid.forEach((x, y, tileType) => {
      const px = x * t;
      const py = y * t;
      switch (tileType) {
        case Tile.Rock: {
          ctx.fillStyle = PALETTE.rock;
          ctx.fillRect(px, py, t, t);
          // Coral rim wherever rock touches open water.
          ctx.fillStyle = PALETTE.rockEdge;
          const e = Math.max(1, t * 0.12);
          if (grid.isWalkable(x, y - 1, true)) ctx.fillRect(px, py, t, e);
          if (grid.isWalkable(x, y + 1, true)) ctx.fillRect(px, py + t - e, t, e);
          if (grid.isWalkable(x - 1, y, true)) ctx.fillRect(px, py, e, t);
          if (grid.isWalkable(x + 1, y, true)) ctx.fillRect(px + t - e, py, e, t);
          // Sparse coral polyps, deterministic per tile.
          if (hash(x, y) % 7 === 0 && grid.isWalkable(x, y + 1, true)) {
            ctx.fillStyle = PALETTE.coral;
            ctx.beginPath();
            ctx.arc(px + t * 0.5, py + t - e, t * 0.14, 0, Math.PI * 2);
            ctx.fill();
          }
          break;
        }
        case Tile.Nest:
          ctx.fillStyle = PALETTE.nest;
          ctx.fillRect(px, py, t, t);
          break;
        case Tile.Nodule: {
          const pulse = 0.85 + 0.15 * Math.sin(time * 3 + hash(x, y));
          ctx.fillStyle = PALETTE.nodule;
          ctx.globalAlpha = pulse;
          ctx.beginPath();
          ctx.arc(px + t * 0.5, py + t * 0.5, t * 0.09, 0, Math.PI * 2);
          ctx.fill();
          ctx.globalAlpha = 1;
          break;
        }
        case Tile.Charge: {
          const pulse = 0.6 + 0.4 * Math.sin(time * 5);
          const r = t * (0.22 + 0.06 * pulse);
          const g = ctx.createRadialGradient(px + t / 2, py + t / 2, r * 0.2, px + t / 2, py + t / 2, r * 2);
          g.addColorStop(0, PALETTE.charge);
          g.addColorStop(1, 'rgba(125,255,154,0)');
          ctx.fillStyle = g;
          ctx.fillRect(px - t / 2, py - t / 2, t * 2, t * 2);
          ctx.fillStyle = PALETTE.charge;
          ctx.beginPath();
          ctx.arc(px + t / 2, py + t / 2, r, 0, Math.PI * 2);
          ctx.fill();
          break;
        }
        default:
          break;
      }
    });
  }

  drawDrone(d: Drone, time: number): void {
    if (!d.alive) return;
    const { ctx, tile: t } = this;
    const p = d.pos;
    const cx = (p.x + 0.5) * t;
    const cy = (p.y + 0.5) * t;
    const v = DIR_VECTORS[d.dir === 'none' ? 'right' : d.dir];
    const r = t * 0.38;

    ctx.fillStyle = PALETTE.drone;
    ctx.beginPath();
    ctx.ellipse(cx, cy, r, r * 0.8, Math.atan2(v.y, v.x), 0, Math.PI * 2);
    ctx.fill();
    // Sensor eye pointing forward.
    ctx.fillStyle = PALETTE.droneAccent;
    ctx.beginPath();
    ctx.arc(cx + v.x * r * 0.5, cy + v.y * r * 0.5, r * 0.28, 0, Math.PI * 2);
    ctx.fill();
    // Thrust glow.
    if (d.thrust > 0.05) {
      ctx.globalAlpha = d.thrust * (0.5 + 0.5 * Math.sin(time * 30));
      ctx.fillStyle = PALETTE.droneAccent;
      ctx.beginPath();
      ctx.arc(cx - v.x * r * 1.1, cy - v.y * r * 1.1, r * 0.35, 0, Math.PI * 2);
      ctx.fill();
      ctx.globalAlpha = 1;
    }
  }

  drawPredator(p: Predator, frenzy: Frenzy, time: number): void {
    const { ctx, tile: t } = this;
    const pos = p.pos;
    const cx = (pos.x + 0.5) * t;
    const cy = (pos.y + 0.5) * t;
    const r = t * 0.42;
    const bob = Math.sin(time * 6 + pos.x) * t * 0.04;

    let color: string = p.color;
    if (p.mode === 'frenzied') {
      color = frenzy.warning && Math.floor(time * 8) % 2 === 0 ? PALETTE.frenziedWarn : PALETTE.frenzied;
    }

    if (p.mode !== 'eaten') {
      if (p.mode === 'frenzied') {
        const g = ctx.createRadialGradient(cx, cy, r * 0.5, cx, cy, r * 2.2);
        g.addColorStop(0, 'rgba(90,120,255,0.45)');
        g.addColorStop(1, 'rgba(90,120,255,0)');
        ctx.fillStyle = g;
        ctx.fillRect(cx - r * 2.2, cy - r * 2.2, r * 4.4, r * 4.4);
      }
      // Body: rounded head, tapered tail with a wave.
      ctx.fillStyle = color;
      ctx.beginPath();
      ctx.arc(cx, cy + bob - r * 0.1, r, Math.PI, 0);
      const wave = Math.sin(time * 10 + pos.x * 3) * r * 0.25;
      ctx.lineTo(cx + r, cy + bob + r * 0.6);
      ctx.lineTo(cx + r * 0.5, cy + bob + r * 0.3 + wave);
      ctx.lineTo(cx, cy + bob + r * 0.65);
      ctx.lineTo(cx - r * 0.5, cy + bob + r * 0.3 - wave);
      ctx.lineTo(cx - r, cy + bob + r * 0.6);
      ctx.closePath();
      ctx.fill();
    }

    // Eyes (blind, milky). Eaten predators render as eyes only.
    const v = DIR_VECTORS[p.dir === 'none' ? 'down' : p.dir];
    const eyeY = cy + bob - r * 0.25;
    for (const side of [-1, 1]) {
      const ex = cx + side * r * 0.38;
      ctx.fillStyle = p.mode === 'frenzied' ? '#ffd0d8' : '#dfe9ee';
      ctx.beginPath();
      ctx.arc(ex, eyeY, r * 0.22, 0, Math.PI * 2);
      ctx.fill();
      if (p.mode !== 'frenzied') {
        ctx.fillStyle = '#1a2a3a';
        ctx.beginPath();
        ctx.arc(ex + v.x * r * 0.08, eyeY + v.y * r * 0.08, r * 0.1, 0, Math.PI * 2);
        ctx.fill();
      }
    }
  }

  drawParticles(particles: Particles): void {
    particles.draw(this.ctx, this.tile);
  }

  /** Pressure vignette over the maze area. */
  drawVignette(gridRows: number): void {
    const { ctx, tile: t } = this;
    const w = this.width;
    const h = gridRows * t;
    const g = ctx.createRadialGradient(w / 2, h / 2, Math.min(w, h) * 0.35, w / 2, h / 2, Math.max(w, h) * 0.75);
    g.addColorStop(0, 'rgba(0,0,0,0)');
    g.addColorStop(1, 'rgba(0,0,0,0.55)');
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, w, h);
  }

  drawHUD(score: Score, levelName: string, frenzy: Frenzy, gridRows: number): void {
    const { ctx, tile: t } = this;
    const top = gridRows * t;
    const w = this.width;
    ctx.fillStyle = PALETTE.abyss;
    ctx.fillRect(0, top, w, HUD_ROWS * t);

    const fs = Math.max(10, t * 0.6);
    ctx.font = `bold ${fs}px system-ui, sans-serif`;
    ctx.textBaseline = 'middle';

    ctx.fillStyle = PALETTE.textDim;
    ctx.textAlign = 'left';
    ctx.fillText('HARVEST', t * 0.5, top + t * 0.55);
    ctx.fillStyle = PALETTE.text;
    ctx.fillText(String(score.score).padStart(6, '0'), t * 0.5, top + t * 1.4);

    ctx.textAlign = 'center';
    ctx.fillStyle = PALETTE.textDim;
    ctx.fillText('RECORD', w / 2, top + t * 0.55);
    ctx.fillStyle = PALETTE.text;
    ctx.fillText(String(score.high).padStart(6, '0'), w / 2, top + t * 1.4);

    ctx.textAlign = 'right';
    ctx.fillStyle = PALETTE.textDim;
    ctx.fillText(levelName.toUpperCase(), w - t * 0.5, top + t * 0.55);

    // Lives as small drones.
    for (let i = 0; i < score.lives - 1; i++) {
      const cx = w - t * (0.9 + i * 0.8);
      const cy = top + t * 1.4;
      ctx.fillStyle = PALETTE.drone;
      ctx.beginPath();
      ctx.ellipse(cx, cy, t * 0.25, t * 0.2, 0, 0, Math.PI * 2);
      ctx.fill();
    }

    // Frenzy timer bar.
    if (frenzy.active) {
      const bw = w * 0.3;
      const bx = w / 2 - bw / 2;
      const by = top + t * 1.8;
      ctx.fillStyle = 'rgba(255,255,255,0.1)';
      ctx.fillRect(bx, by, bw, t * 0.12);
      ctx.fillStyle = frenzy.warning ? PALETTE.frenziedWarn : PALETTE.frenzied;
      ctx.fillRect(bx, by, bw * frenzy.fraction, t * 0.12);
    }
  }

  /** Centered overlay for title, ready, pause, and game over screens. */
  drawOverlay(lines: OverlayLine[], dim = 0.65): void {
    const { ctx, tile: t } = this;
    ctx.fillStyle = `rgba(2,6,12,${dim})`;
    ctx.fillRect(0, 0, this.width, this.height);
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    const totalH = lines.reduce((s, l) => s + (l.size ?? 1) * t * 1.4, 0);
    let y = this.height / 2 - totalH / 2;
    for (const l of lines) {
      const size = (l.size ?? 1) * t * 0.9;
      y += size * 0.7;
      ctx.font = `bold ${size}px system-ui, sans-serif`;
      ctx.fillStyle = l.color ?? PALETTE.text;
      ctx.fillText(l.text, this.width / 2, y);
      y += size * 0.85;
    }
  }

  drawFloatingText(x: number, y: number, text: string, alpha: number): void {
    const { ctx, tile: t } = this;
    ctx.globalAlpha = alpha;
    ctx.font = `bold ${t * 0.55}px system-ui, sans-serif`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillStyle = PALETTE.frenziedWarn;
    ctx.fillText(text, (x + 0.5) * t, (y + 0.5) * t);
    ctx.globalAlpha = 1;
  }
}

function hash(x: number, y: number): number {
  let h = (x * 374761393 + y * 668265263) | 0;
  h = ((h ^ (h >>> 13)) * 1274126177) | 0;
  return (h ^ (h >>> 16)) >>> 0;
}
