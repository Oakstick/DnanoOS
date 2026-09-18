import { Drone } from '../entities/Drone';
import { GlobalMode, Predator } from '../entities/Predator';
import { Renderer } from '../render/Renderer';

export interface DebugSnapshot {
  drone: Drone;
  predators: Predator[];
  globalMode: GlobalMode;
  modeTimer: number;
  fps: number;
  particleCount: number;
}

/** Toggle with F1. Shows predator targets, AI modes, and timing. */
export class DebugOverlay {
  enabled = false;
  private frames = 0;
  private fpsAccum = 0;
  fps = 0;

  tick(dt: number): void {
    this.frames++;
    this.fpsAccum += dt;
    if (this.fpsAccum >= 0.5) {
      this.fps = Math.round(this.frames / this.fpsAccum);
      this.frames = 0;
      this.fpsAccum = 0;
    }
  }

  draw(r: Renderer, snap: DebugSnapshot): void {
    if (!this.enabled) return;
    const { ctx, tile: t } = r;
    ctx.save();
    ctx.lineWidth = 1;
    ctx.font = `${Math.max(9, t * 0.4)}px monospace`;
    ctx.textBaseline = 'top';
    ctx.textAlign = 'left';

    for (const p of snap.predators) {
      const pos = p.pos;
      const px = (pos.x + 0.5) * t;
      const py = (pos.y + 0.5) * t;
      const tx = (p.target.x + 0.5) * t;
      const ty = (p.target.y + 0.5) * t;
      ctx.strokeStyle = p.color;
      ctx.setLineDash([3, 3]);
      ctx.beginPath();
      ctx.moveTo(px, py);
      ctx.lineTo(tx, ty);
      ctx.stroke();
      ctx.setLineDash([]);
      ctx.strokeRect(tx - t * 0.3, ty - t * 0.3, t * 0.6, t * 0.6);
      ctx.fillStyle = p.color;
      ctx.fillText(`${p.name}:${p.mode}`, px + t * 0.5, py - t * 0.6);
    }

    // Heard position marker for the sound-sensitive hunter.
    const hp = snap.drone.heardPos;
    ctx.strokeStyle = '#ffffff';
    ctx.beginPath();
    ctx.arc((hp.x + 0.5) * t, (hp.y + 0.5) * t, t * 0.5, 0, Math.PI * 2);
    ctx.stroke();

    const lines = [
      `fps ${snap.fps}`,
      `mode ${snap.globalMode} (${snap.modeTimer.toFixed(1)}s)`,
      `drone ${snap.drone.tile.x},${snap.drone.tile.y} ${snap.drone.dir} heardAge ${snap.drone.heardAge.toFixed(2)}`,
      `particles ${snap.particleCount}`,
    ];
    ctx.fillStyle = 'rgba(0,0,0,0.6)';
    ctx.fillRect(4, 4, t * 11, lines.length * t * 0.5 + 8);
    ctx.fillStyle = '#9ff';
    lines.forEach((l, i) => ctx.fillText(l, 8, 8 + i * t * 0.5));
    ctx.restore();
  }
}
