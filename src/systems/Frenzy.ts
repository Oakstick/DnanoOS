/**
 * The state inversion. Tracks the frenzy window and the escalating value of
 * enemies consumed within a single window.
 */
export class Frenzy {
  timer = 0;
  duration = 0;
  chain = 0;

  get active(): boolean {
    return this.timer > 0;
  }

  /** Last two seconds: enemies flash as a warning. */
  get warning(): boolean {
    return this.active && this.timer <= 2;
  }

  /** 0..1 remaining fraction for the HUD bar. */
  get fraction(): number {
    return this.duration > 0 ? Math.max(0, this.timer / this.duration) : 0;
  }

  start(seconds: number): void {
    this.timer = seconds;
    this.duration = seconds;
    this.chain = 0;
  }

  /** Returns true on the tick the window closes. */
  update(dt: number): boolean {
    if (this.timer <= 0) return false;
    this.timer -= dt;
    if (this.timer <= 0) {
      this.timer = 0;
      return true;
    }
    return false;
  }

  /** Score for the next enemy eaten in this window: 200, 400, 800, 1600. */
  consumeValue(): number {
    const value = 200 * Math.pow(2, this.chain);
    this.chain++;
    return value;
  }

  cancel(): void {
    this.timer = 0;
  }
}
