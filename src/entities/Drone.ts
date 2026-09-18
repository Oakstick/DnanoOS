import { Grid } from '../world/Grid';
import { Dir, Vec2 } from '../world/types';
import { Mover } from './Mover';

/**
 * The player's bio-scavenger drone. Emits "sound" while moving; predators
 * with hearing home in on `heardPos` rather than the true position.
 */
export class Drone extends Mover {
  heardPos: Vec2;
  heardAge = 0; // seconds since the drone last made noise
  thrust = 0; // 0..1 visual throttle
  alive = true;

  constructor(start: Vec2, speed: number) {
    super(start, speed);
    this.heardPos = { ...start };
  }

  update(dt: number, grid: Grid, wanted: Dir): void {
    if (!this.alive) return;
    const before = this.pos;
    this.step(dt, grid, () => wanted, false, true);
    const after = this.pos;
    const moved = before.x !== after.x || before.y !== after.y;
    if (moved) {
      this.heardPos = after;
      this.heardAge = 0;
      this.thrust = Math.min(1, this.thrust + dt * 6);
    } else {
      this.heardAge += dt;
      this.thrust = Math.max(0, this.thrust - dt * 4);
    }
  }

  respawn(at: Vec2): void {
    this.teleport(at);
    this.heardPos = { ...at };
    this.heardAge = 0;
    this.alive = true;
    this.thrust = 0;
  }
}
