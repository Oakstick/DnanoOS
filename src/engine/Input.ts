import { Dir } from '../world/types';

/**
 * Keyboard + touch swipe. Directions are buffered: the latest requested
 * direction is held until the entity can actually turn that way.
 */
export class Input {
  private buffered: Dir = 'none';
  private pressed = new Set<string>();
  private justPressed = new Set<string>();
  private touchStart: { x: number; y: number } | null = null;

  constructor(target: HTMLElement) {
    window.addEventListener('keydown', (e) => {
      if (e.repeat) return;
      const code = normalizeCode(e);
      this.pressed.add(code);
      this.justPressed.add(code);
      const d = keyToDir(code);
      if (d !== 'none') {
        this.buffered = d;
        e.preventDefault();
      }
      if (['Space', 'Enter', 'Escape', 'KeyM', 'F1'].includes(code)) e.preventDefault();
    });
    window.addEventListener('keyup', (e) => this.pressed.delete(normalizeCode(e)));

    target.addEventListener('touchstart', (e) => {
      const t = e.changedTouches[0];
      this.touchStart = { x: t.clientX, y: t.clientY };
      this.justPressed.add('Touch');
    }, { passive: true });
    target.addEventListener('touchend', (e) => {
      if (!this.touchStart) return;
      const t = e.changedTouches[0];
      const dx = t.clientX - this.touchStart.x;
      const dy = t.clientY - this.touchStart.y;
      if (Math.max(Math.abs(dx), Math.abs(dy)) > 24) {
        this.buffered = Math.abs(dx) > Math.abs(dy) ? (dx > 0 ? 'right' : 'left') : dy > 0 ? 'down' : 'up';
      }
      this.touchStart = null;
    }, { passive: true });
  }

  get direction(): Dir {
    return this.buffered;
  }

  clearDirection(): void {
    this.buffered = 'none';
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
