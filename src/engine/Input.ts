import { Dir } from '../world/types';

/**
 * Keyboard + touch. Movement is hold-to-move: `direction` is the most
 * recently pressed direction key that is still held, or 'none' when nothing
 * is held. Touch works as a drag joystick: the finger's offset from where it
 * landed is the held direction until it lifts.
 */
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

  /** True while the mouse is captured and aiming the camera. */
  get aiming(): boolean {
    return document.pointerLockElement === this.target;
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
    if (this.touchDir !== 'none') return this.touchDir;
    return this.held.length ? this.held[this.held.length - 1] : 'none';
  }

  clearDirection(): void {
    this.held = [];
    this.touchDir = 'none';
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
