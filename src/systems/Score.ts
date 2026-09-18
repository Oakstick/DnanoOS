export const POINTS = {
  nodule: 10,
  charge: 50,
} as const;

const HIGH_SCORE_KEY = 'vh.highscore';

export class Score {
  score = 0;
  high = 0;
  lives = 3;
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
      this.lives++;
      return true;
    }
    return false;
  }

  loseLife(): boolean {
    this.lives--;
    return this.lives <= 0;
  }

  reset(): void {
    this.score = 0;
    this.lives = 3;
    this.level = 0;
    this.nextLifeAt = 10_000;
  }
}
