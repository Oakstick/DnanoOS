/** A gold nugget is worth a kill; only fifty exist per grove. */
export const POINTS = {
  nodule: 50,
  charge: 50,
} as const;

const HIGH_SCORE_KEY = 'vh.highscore';

/** Popolon starts with three lives and can never hold more. */
export const MAX_LIVES = 3;

export class Score {
  score = 0;
  high = 0;
  lives = MAX_LIVES;
  level = 0;
  private nextLifeAt = 10_000;

  constructor() {
    try {
      this.high = Number(localStorage.getItem(HIGH_SCORE_KEY) ?? 0) || 0;
    } catch {
      this.high = 0;
    }
  }

  /** Returns true if an extra life was awarded. */
  add(points: number): boolean {
    this.score += points;
    if (this.score > this.high) {
      this.high = this.score;
      try {
        localStorage.setItem(HIGH_SCORE_KEY, String(this.high));
      } catch {
        /* ignore */
      }
    }
    if (this.score >= this.nextLifeAt) {
      this.nextLifeAt += 10_000;
      return this.gainLife();
    }
    return false;
  }

  /** Returns true if a life was actually added (not already at the cap). */
  gainLife(): boolean {
    if (this.lives >= MAX_LIVES) return false;
    this.lives++;
    return true;
  }

  loseLife(): boolean {
    this.lives--;
    return this.lives <= 0;
  }

  reset(): void {
    this.score = 0;
    this.lives = MAX_LIVES;
    this.level = 0;
    this.nextLifeAt = 10_000;
  }
}
