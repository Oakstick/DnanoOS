/**
 * Fixed-timestep update, variable-rate render.
 * Pauses automatically when the tab is hidden.
 */
export class GameLoop {
  private raf = 0;
  private last = 0;
  private acc = 0;
  private running = false;
  readonly step: number;

  constructor(
    private update: (dt: number) => void,
    private render: (alpha: number) => void,
    stepHz = 120,
  ) {
    this.step = 1 / stepHz;
    document.addEventListener('visibilitychange', () => {
      if (document.hidden) this.stop();
      else this.start();
    });
  }

  start(): void {
    if (this.running) return;
    this.running = true;
    this.last = performance.now();
    this.acc = 0;
    const frame = (now: number) => {
      if (!this.running) return;
      let dt = (now - this.last) / 1000;
      this.last = now;
      if (dt > 0.25) dt = 0.25; // clamp after long stalls
      this.acc += dt;
      let guard = 0;
      while (this.acc >= this.step && guard++ < 10) {
        this.update(this.step);
        this.acc -= this.step;
      }
      this.render(this.acc / this.step);
      this.raf = requestAnimationFrame(frame);
    };
    this.raf = requestAnimationFrame(frame);
  }

  stop(): void {
    this.running = false;
    cancelAnimationFrame(this.raf);
  }
}
