import { Dir } from '../world/types';

/**
 * Keyboard, touch and gamepad. Movement is hold-to-move: `direction` is the
 * most recently pressed direction that is still held, or 'none' when nothing
 * is held. Touch works as a drag joystick: the finger's offset from where it
 * landed is the held direction until it lifts. A controller's buttons are
 * reported as the key codes they stand in for, so the rest of the game never
 * has to know which one you are playing with.
 */
/** Gamepad buttons in the standard layout, mapped to the keys they stand for. */
const PAD_BUTTONS: Record<number, string> = {
  0: 'Space',       // A / cross: jump
  2: 'KeyE',        // X / square: attack
  7: 'KeyE',        // right trigger: attack
  1: 'KeyB',        // B / circle: bomb
  4: 'KeyB',        // left bumper: bomb
  3: 'KeyC',        // Y / triangle: swap camera
  9: 'Enter',       // start
  8: 'Escape',      // back / select
};
/** D-pad buttons, which move him exactly like the sticks do. */
const PAD_DPAD: Record<number, Dir> = { 12: 'up', 13: 'down', 14: 'left', 15: 'right' };
/** Past this much push a stick counts as held; a grid needs a decisive shove. */
const MOVE_DEADZONE = 0.4;
const LOOK_DEADZONE = 0.16;
/** Right stick at full tilt turns the camera this fast, in mouse pixels per second. */
const PAD_LOOK_SPEED = 950;
/** Seconds the camera keeps obeying the stick after you let go of it. */
const PAD_AIM_MEMORY = 1.2;
export class Input {
  private held: Dir[] = [];
  /** Increments on every direction press so consumers can react once per press. */
  directionSeq = 0;
  private pressed = new Set<string>();
  private justPressed = new Set<string>();
  private touchStart: { x: number; y: number } | null = null;
  private touchDir: Dir = 'none';
  private lookX = 0;
  private lookY = 0;
  private readonly target: HTMLElement;
  private padDir: Dir = 'none';
  private padHeld = new Set<string>();
  private padAimAge = PAD_AIM_MEMORY;
  /** True once a controller has been seen, for the hint on the title screen. */
  padConnected = false;

  constructor(target: HTMLElement) {
    this.target = target;
    window.addEventListener('keydown', (e) => {
      if (e.repeat) return;
      const code = normalizeCode(e);
      this.pressed.add(code);
      this.justPressed.add(code);
      const d = keyToDir(code);
      if (d !== 'none') {
        this.held = this.held.filter((h) => h !== d);
        this.held.push(d);
        this.directionSeq++;
        e.preventDefault();
      }
      if (['Space', 'Enter', 'Escape', 'KeyM', 'F1'].includes(code)) e.preventDefault();
    });
    window.addEventListener('keyup', (e) => {
      const code = normalizeCode(e);
      this.pressed.delete(code);
      const d = keyToDir(code);
      if (d !== 'none') this.held = this.held.filter((h) => h !== d);
    });
    window.addEventListener('blur', () => {
      this.held = [];
      this.pressed.clear();
    });

    // Mouse aiming: clicking the view captures the pointer, after which the
    // mouse turns the camera instead of moving a cursor around the page.
    target.addEventListener('mousedown', () => {
      if (document.pointerLockElement !== target) target.requestPointerLock?.();
    });
    document.addEventListener('mousemove', (e) => {
      if (document.pointerLockElement !== target) return;
      this.lookX += e.movementX;
      this.lookY += e.movementY;
    });
    document.addEventListener('pointerlockchange', () => {
      this.lookX = 0;
      this.lookY = 0;
    });

    target.addEventListener('touchstart', (e) => {
      const t = e.changedTouches[0];
      this.touchStart = { x: t.clientX, y: t.clientY };
      this.justPressed.add('Touch');
    }, { passive: true });
    target.addEventListener('touchmove', (e) => {
      if (!this.touchStart) return;
      const t = e.changedTouches[0];
      const dx = t.clientX - this.touchStart.x;
      const dy = t.clientY - this.touchStart.y;
      if (Math.max(Math.abs(dx), Math.abs(dy)) > 18) {
        const d: Dir = Math.abs(dx) > Math.abs(dy) ? (dx > 0 ? 'right' : 'left') : dy > 0 ? 'down' : 'up';
        if (d !== this.touchDir) this.directionSeq++;
        this.touchDir = d;
      }
    }, { passive: true });
    const release = () => {
      this.touchStart = null;
      this.touchDir = 'none';
    };
    target.addEventListener('touchend', release, { passive: true });
    target.addEventListener('touchcancel', release, { passive: true });
  }

  /**
   * Reads the controller. Called once per fixed update, which is what makes
   * stick aiming run at the same speed whatever the frame rate is doing.
   */
  poll(dt: number): void {
    const pads = navigator.getGamepads ? navigator.getGamepads() : [];
    let pad: Gamepad | null = null;
    for (const p of pads) if (p && p.connected) { pad = p; break; }
    this.padAimAge += dt;
    if (!pad) {
      if (this.padHeld.size > 0) {
        for (const code of this.padHeld) this.pressed.delete(code);
        this.padHeld.clear();
      }
      this.padDir = 'none';
      return;
    }
    this.padConnected = true;

    // Buttons, as the keys they stand for, so presses arrive like any other.
    const down = new Set<string>();
    pad.buttons.forEach((b, i) => {
      if (!b.pressed) return;
      const code = PAD_BUTTONS[i];
      if (code) down.add(code);
    });
    for (const code of down) {
      if (!this.padHeld.has(code)) this.justPressed.add(code);
      this.pressed.add(code);
    }
    for (const code of this.padHeld) if (!down.has(code)) this.pressed.delete(code);
    this.padHeld = down;

    // Left stick, or the d-pad, chooses the corridor.
    let dir: Dir = 'none';
    for (const [i, d] of Object.entries(PAD_DPAD)) {
      if (pad.buttons[Number(i)]?.pressed) dir = d;
    }
    if (dir === 'none') {
      const x = pad.axes[0] ?? 0;
      const y = pad.axes[1] ?? 0;
      if (Math.max(Math.abs(x), Math.abs(y)) >= MOVE_DEADZONE) {
        dir = Math.abs(x) > Math.abs(y) ? (x > 0 ? 'right' : 'left') : (y > 0 ? 'down' : 'up');
      }
    }
    if (dir !== this.padDir && dir !== 'none') this.directionSeq++;
    this.padDir = dir;

    // Right stick aims, squared so small pushes stay fine and full tilt is quick.
    const lx = curve(pad.axes[2] ?? 0);
    const ly = curve(pad.axes[3] ?? 0);
    if (lx !== 0 || ly !== 0) {
      this.lookX += lx * PAD_LOOK_SPEED * dt;
      this.lookY += ly * PAD_LOOK_SPEED * dt;
      this.padAimAge = 0;
    }
  }

  /** True while the mouse or the right stick is aiming the camera. */
  get aiming(): boolean {
    return document.pointerLockElement === this.target || this.padAimAge < PAD_AIM_MEMORY;
  }

  /** Mouse movement in pixels since the last call, then cleared. */
  takeLook(): { dx: number; dy: number } {
    const d = { dx: this.lookX, dy: this.lookY };
    this.lookX = 0;
    this.lookY = 0;
    return d;
  }

  /** Held movement direction right now, or 'none'. */
  get direction(): Dir {
    if (this.padDir !== 'none') return this.padDir;
    if (this.touchDir !== 'none') return this.touchDir;
    return this.held.length ? this.held[this.held.length - 1] : 'none';
  }

  clearDirection(): void {
    this.held = [];
    this.touchDir = 'none';
    this.padDir = 'none';
  }

  isDown(code: string): boolean {
    return this.pressed.has(code);
  }

  /** True once per press; cleared at end of each frame via flush(). */
  wasPressed(...codes: string[]): boolean {
    return codes.some((c) => this.justPressed.has(c));
  }

  flush(): void {
    this.justPressed.clear();
  }
}

/** Stick response: squared past the deadzone, so a nudge is a nudge. */
function curve(v: number): number {
  if (Math.abs(v) < LOOK_DEADZONE) return 0;
  const t = (Math.abs(v) - LOOK_DEADZONE) / (1 - LOOK_DEADZONE);
  return Math.sign(v) * t * t;
}

/** Prefer the physical key code; fall back to the logical key for virtual keyboards. */
function normalizeCode(e: KeyboardEvent): string {
  if (e.code) return e.code;
  if (e.key === ' ') return 'Space';
  return e.key.length === 1 ? `Key${e.key.toUpperCase()}` : e.key;
}

function keyToDir(code: string): Dir {
  switch (code) {
    case 'ArrowUp': case 'KeyW': return 'up';
    case 'ArrowDown': case 'KeyS': return 'down';
    case 'ArrowLeft': case 'KeyA': return 'left';
    case 'ArrowRight': case 'KeyD': return 'right';
    default: return 'none';
  }
}
