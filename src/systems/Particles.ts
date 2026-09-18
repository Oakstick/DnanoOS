interface Particle {
  x: number;
  y: number;
  vx: number;
  vy: number;
  life: number;
  maxLife: number;
  size: number;
  color: string;
}

/** Lightweight particle pool in tile units. */
export class Particles {
  private list: Particle[] = [];
  private readonly max = 600;

  emit(x: number, y: number, count: number, color: string, speed = 3, life = 0.6, size = 0.12): void {
    for (let i = 0; i < count; i++) {
      if (this.list.length >= this.max) this.list.shift();
      const a = Math.random() * Math.PI * 2;
      const s = speed * (0.3 + Math.random() * 0.7);
      this.list.push({
        x, y,
        vx: Math.cos(a) * s,
        vy: Math.sin(a) * s,
        life: life * (0.6 + Math.random() * 0.4),
        maxLife: life,
        size: size * (0.5 + Math.random()),
        color,
      });
    }
  }

  /** Slow upward drift, used for ambient marine snow. */
  emitDrift(x: number, y: number, color: string): void {
    if (this.list.length >= this.max) return;
    this.list.push({ x, y, vx: (Math.random() - 0.5) * 0.2, vy: -0.15 - Math.random() * 0.2, life: 4, maxLife: 4, size: 0.05, color });
  }

  update(dt: number): void {
    for (let i = this.list.length - 1; i >= 0; i--) {
      const p = this.list[i];
      p.life -= dt;
      if (p.life <= 0) {
        this.list.splice(i, 1);
        continue;
      }
      p.x += p.vx * dt;
      p.y += p.vy * dt;
      p.vx *= 1 - 2 * dt;
      p.vy *= 1 - 2 * dt;
    }
  }

  draw(ctx: CanvasRenderingContext2D, tile: number): void {
    for (const p of this.list) {
      const a = Math.max(0, p.life / p.maxLife);
      ctx.globalAlpha = a;
      ctx.fillStyle = p.color;
      const r = p.size * tile;
      ctx.beginPath();
      ctx.arc((p.x + 0.5) * tile, (p.y + 0.5) * tile, r, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.globalAlpha = 1;
  }

  clear(): void {
    this.list.length = 0;
  }

  get count(): number {
    return this.list.length;
  }
}
