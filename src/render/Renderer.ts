import { WeaponKind } from '../entities/Weapons';
import { BAT, POPOLON, SKELETON, SLIME, Sprite, spriteFromModel } from '../render3d/voxels';
import { Frenzy } from '../systems/Frenzy';
import { Score } from '../systems/Score';

/** HUD band height in tiles: room for the portrait panel. */
export const HUD_ROWS = 3;
/** The layout unit is the window divided into this many columns or rows, whichever is smaller. */
const LAYOUT_COLS = 64;
const LAYOUT_ROWS = 36;
const MIN_TILE = 10;
const MAX_TILE = 28;

export const PALETTE = {
  abyss: '#070608',
  hero: '#f2e3c4',
  frenzied: '#3b5bff',
  frenziedWarn: '#e8f0ff',
  text: '#e6e0d2',
  textDim: '#8f8a80',
} as const;

export interface OverlayLine {
  text: string;
  size?: number;
  color?: string;
}

/** Everything the HUD band needs to show about the hero's state. */
export interface HudStatus {
  hp: number;
  maxHp: number;
  bombs: number;
  weaponKind: WeaponKind;
  weaponName: string;
  weaponColor: string;
  /** Shots left; ignored for the sword. */
  ammo: number;
  shield: number;
  invincibleSeconds: number;
  invisibleSeconds: number;
  speedSeconds: number;
  slowSeconds: number;
  lives: number;
  maxLives: number;
  nuggetsGot: number;
  nuggetsTotal: number;
  /** Rendered frames per second; shown only when set. */
  fps?: number;
}

const PORTRAIT: Sprite = spriteFromModel(POPOLON);
const CAST: { sprite: Sprite; name: string; color: string; line: string }[] = [
  { sprite: spriteFromModel(SKELETON), name: 'Skeleton', color: '#e9e9f2', line: 'Follows your footsteps. Falls apart, gets up again.' },
  { sprite: spriteFromModel(BAT), name: 'Bat', color: '#9a5cff', line: 'Fast, and flies over pits and ledges.' },
  { sprite: spriteFromModel(SLIME), name: 'Slime', color: '#5cff6a', line: 'Slow, but its trail slows you too.' },
];

const RULES: string[] = [
  'Collect all 25 gold nuggets to win. Three lives; dying loses everything you carry.',
  'The dungeon rises in levels. Jump with Space to clear pits, leap over foes, and climb a ledge; ramps let you walk up. Bats fly over everything.',
  'Enemies hunt you, and more keep arriving from the hollow the longer you take.',
  'You start with a sword. Strike a shrine (E) to cycle its gift, step on it to take it: a weapon replaces your sword until its ammo runs out.',
  'Green crosses restore 50 health. Potions: red is an extra life, blue a shield, purple invisibility. Bombs blast through walls. Slime trails slow you.',
];

/**
 * Canvas 2D overlay that floats over the WebGL view: HUD band, world labels,
 * and the start, pause, win, and death screens. Fills the window; `tile` is
 * the layout unit and scales with the shorter window side so text and the
 * HUD band stay legible from phones to 4K.
 */
export class Renderer {
  readonly ctx: CanvasRenderingContext2D;
  tile = 24;
  private w = 1;
  private h = 1;
  private dpr = 1;

  constructor(readonly canvas: HTMLCanvasElement) {
    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('Canvas 2D not supported');
    this.ctx = ctx;
  }

  resize(): void {
    this.w = window.innerWidth;
    this.h = window.innerHeight;
    this.dpr = Math.min(window.devicePixelRatio || 1, 2);
    this.tile = Math.max(MIN_TILE, Math.min(MAX_TILE, this.w / LAYOUT_COLS, this.h / LAYOUT_ROWS));
    this.canvas.style.width = `${this.w}px`;
    this.canvas.style.height = `${this.h}px`;
    this.canvas.width = Math.round(this.w * this.dpr);
    this.canvas.height = Math.round(this.h * this.dpr);
    this.ctx.setTransform(this.dpr, 0, 0, this.dpr, 0, 0);
  }

  get width(): number {
    return this.w;
  }

  get height(): number {
    return this.h;
  }

  begin(): void {
    this.ctx.clearRect(0, 0, this.width, this.height);
  }

  // ------------------------------------------------------------------ HUD

  /** The HUD band sits along the bottom edge of the window. */
  drawHUD(score: Score, levelName: string, frenzy: Frenzy, s: HudStatus, time = 0): void {
    const { ctx, tile: t } = this;
    const band = HUD_ROWS * t;
    const top = this.height - band;
    const w = this.width;
    ctx.fillStyle = 'rgba(2,6,12,0.82)';
    ctx.fillRect(0, top, w, band);

    const fs = Math.max(10, t * 0.6);
    const small = Math.max(9, t * 0.45);
    ctx.textBaseline = 'middle';

    // Portrait panel: Popolon's front view, tinted by his current state.
    const portraitH = band - t * 0.4;
    const scale = portraitH / PORTRAIT.height;
    const px0 = t * 0.4;
    const py0 = top + t * 0.2;
    ctx.fillStyle = 'rgba(255,255,255,0.05)';
    ctx.fillRect(px0 - t * 0.15, py0 - t * 0.1, PORTRAIT.width * scale + t * 0.3, portraitH + t * 0.2);
    const ghost = s.invisibleSeconds > 0;
    const golden = s.invincibleSeconds > 0 && Math.floor(time * 6) % 2 === 0;
    ctx.globalAlpha = ghost ? 0.4 : 1;
    PORTRAIT.pixels.forEach((row, y) => row.forEach((c, x) => {
      if (c === null) return;
      ctx.fillStyle = golden ? '#ffd84a' : `#${c.toString(16).padStart(6, '0')}`;
      ctx.fillRect(px0 + x * scale, py0 + y * scale, scale + 0.5, scale + 0.5);
    }));
    ctx.globalAlpha = 1;
    if (s.shield > 0) {
      ctx.strokeStyle = 'rgba(127,215,255,0.9)';
      ctx.lineWidth = Math.max(1, t * 0.08);
      ctx.strokeRect(px0 - t * 0.15, py0 - t * 0.1, PORTRAIT.width * scale + t * 0.3, portraitH + t * 0.2);
    }

    const cx0 = px0 + PORTRAIT.width * scale + t * 0.7;
    const rowA = top + t * 0.5;
    const rowB = top + t * 1.2;
    const rowC = top + t * 1.95;
    const rowD = top + t * 2.65;

    ctx.textAlign = 'left';
    ctx.font = `bold ${small}px system-ui, sans-serif`;
    ctx.fillStyle = PALETTE.textDim;
    ctx.fillText('POPOLON', cx0, rowA);
    let hx = cx0 + ctx.measureText('POPOLON').width + t * 0.5;
    ctx.font = `bold ${fs}px system-ui, sans-serif`;
    for (let i = 0; i < s.maxLives; i++) {
      ctx.fillStyle = i < s.lives ? '#ff4d6d' : 'rgba(255,255,255,0.18)';
      ctx.fillText('♥', hx, rowA);
      hx += t * 0.7;
    }

    const hpFrac = s.hp / s.maxHp;
    const hpColor = hpFrac > 0.5 ? '#5cff6a' : hpFrac > 0.25 ? '#f2c53d' : '#d8262e';
    this.drawBarAt(cx0, rowB - t * 0.18, t * 6, t * 0.36, hpFrac, hpColor);
    ctx.font = `bold ${small}px system-ui, sans-serif`;
    ctx.fillStyle = PALETTE.text;
    ctx.fillText(`${s.hp}/${s.maxHp}`, cx0 + t * 6.3, rowB);

    this.drawWeaponIcon(s.weaponKind, cx0 + t * 0.35, rowC, t * 0.5, s.weaponColor);
    ctx.fillStyle = s.weaponColor;
    ctx.textAlign = 'left';
    const weaponText = s.weaponKind === 'sword' ? 'SWORD' : `${s.weaponName.toUpperCase()} x${s.ammo}`;
    ctx.fillText(weaponText, cx0 + t * 0.9, rowC);

    const boons: { text: string; color: string }[] = [];
    if (s.shield > 0) boons.push({ text: `SHIELD ${s.shield}`, color: '#7fd7ff' });
    if (s.invincibleSeconds > 0) boons.push({ text: `INVINCIBLE ${Math.ceil(s.invincibleSeconds)}s`, color: '#ffd84a' });
    if (s.invisibleSeconds > 0) boons.push({ text: `INVISIBLE ${Math.ceil(s.invisibleSeconds)}s`, color: '#b06cff' });
    if (s.speedSeconds > 0) boons.push({ text: `SPEED ${Math.ceil(s.speedSeconds)}s`, color: '#7dff9a' });
    if (s.slowSeconds > 0) boons.push({ text: 'SLOWED', color: '#5cff6a' });
    let bx = cx0;
    if (boons.length === 0) {
      ctx.fillStyle = PALETTE.textDim;
      ctx.fillText('NO POTION', bx, rowD);
    }
    for (const b of boons) {
      ctx.fillStyle = b.color;
      ctx.fillRect(bx, rowD - t * 0.18, t * 0.3, t * 0.36);
      ctx.fillText(b.text, bx + t * 0.45, rowD);
      bx += ctx.measureText(b.text).width + t * 1.1;
    }

    ctx.textAlign = 'center';
    ctx.font = `bold ${small}px system-ui, sans-serif`;
    ctx.fillStyle = PALETTE.textDim;
    ctx.fillText('GOLD', w / 2, rowA);
    ctx.font = `bold ${fs}px system-ui, sans-serif`;
    ctx.fillStyle = PALETTE.text;
    ctx.fillText(String(score.score).padStart(6, '0'), w / 2, rowB);
    ctx.font = `bold ${small}px system-ui, sans-serif`;
    ctx.fillStyle = '#f2c53d';
    ctx.fillText(`NUGGETS ${s.nuggetsGot}/${s.nuggetsTotal}`, w / 2, rowC);
    ctx.fillStyle = PALETTE.textDim;
    ctx.fillText(`BEST ${String(score.high).padStart(6, '0')}`, w / 2, rowD);

    ctx.textAlign = 'right';
    ctx.fillStyle = PALETTE.textDim;
    ctx.fillText(levelName.toUpperCase(), w - t * 0.5, rowA);
    let x = w - t * 0.5;
    for (let i = 0; i < s.bombs; i++) {
      ctx.fillStyle = '#c8262e';
      ctx.fillRect(x - t * 0.44, rowB - t * 0.22, t * 0.44, t * 0.44);
      ctx.fillStyle = '#f4f4f8';
      ctx.fillRect(x - t * 0.44, rowB - t * 0.06, t * 0.44, t * 0.12);
      x -= t * 0.6;
    }
    ctx.fillStyle = PALETTE.textDim;
    ctx.fillText(s.bombs > 0 ? 'BOMBS' : 'NO BOMBS', w - t * 0.5, rowC);
    if (s.fps !== undefined) {
      ctx.font = `bold ${small}px monospace`;
      ctx.fillStyle = '#9ff';
      ctx.fillText(`${s.fps} fps`, w - t * 0.5, rowD);
    }

    if (frenzy.active) {
      const bw = w * 0.3;
      const bxx = w / 2 - bw / 2;
      const by = top + band - t * 0.16;
      ctx.fillStyle = 'rgba(255,255,255,0.1)';
      ctx.fillRect(bxx, by, bw, t * 0.1);
      ctx.fillStyle = frenzy.warning ? PALETTE.frenziedWarn : PALETTE.frenzied;
      ctx.fillRect(bxx, by, bw * frenzy.fraction, t * 0.1);
    }
  }

  /** Small glyph for the held weapon, centered at (cx, cy). */
  private drawWeaponIcon(kind: WeaponKind, cx: number, cy: number, size: number, color: string): void {
    const { ctx } = this;
    ctx.strokeStyle = color;
    ctx.fillStyle = color;
    ctx.lineWidth = Math.max(1, size * 0.18);
    ctx.lineCap = 'round';
    const h = size / 2;
    ctx.beginPath();
    switch (kind) {
      case 'sword':
        ctx.moveTo(cx, cy + h);
        ctx.lineTo(cx, cy - h);
        ctx.moveTo(cx - h * 0.6, cy + h * 0.35);
        ctx.lineTo(cx + h * 0.6, cy + h * 0.35);
        ctx.stroke();
        break;
      case 'arrow':
        ctx.moveTo(cx, cy + h);
        ctx.lineTo(cx, cy - h);
        ctx.moveTo(cx - h * 0.5, cy - h * 0.4);
        ctx.lineTo(cx, cy - h);
        ctx.lineTo(cx + h * 0.5, cy - h * 0.4);
        ctx.stroke();
        break;
      case 'double':
        for (const dx of [-h * 0.4, h * 0.4]) {
          ctx.moveTo(cx + dx, cy + h);
          ctx.lineTo(cx + dx, cy - h);
        }
        ctx.stroke();
        break;
      case 'trishot':
        ctx.moveTo(cx, cy + h * 0.6);
        ctx.lineTo(cx, cy - h);
        ctx.moveTo(cx, cy + h * 0.6);
        ctx.lineTo(cx - h, cy + h * 0.2);
        ctx.moveTo(cx, cy + h * 0.6);
        ctx.lineTo(cx + h, cy + h * 0.2);
        ctx.stroke();
        break;
      case 'boomerang':
        ctx.moveTo(cx - h, cy - h * 0.6);
        ctx.lineTo(cx - h * 0.2, cy + h * 0.7);
        ctx.lineTo(cx + h, cy - h * 0.2);
        ctx.stroke();
        break;
      case 'fireball':
        ctx.arc(cx, cy, h * 0.7, 0, Math.PI * 2);
        ctx.fill();
        break;
      case 'crescent':
        ctx.arc(cx, cy, h * 0.9, Math.PI * 1.1, Math.PI * 1.9);
        ctx.stroke();
        break;
    }
  }

  // -------------------------------------------------------------- screens

  /** Pixel sprite at canvas coordinates; (x, y) is the top-left. */
  drawSprite(sprite: Sprite, x: number, y: number, scale: number, alpha = 1): void {
    const { ctx } = this;
    ctx.globalAlpha = alpha;
    sprite.pixels.forEach((row, py) => row.forEach((c, px) => {
      if (c === null) return;
      ctx.fillStyle = `#${c.toString(16).padStart(6, '0')}`;
      ctx.fillRect(x + px * scale, y + py * scale, scale + 0.5, scale + 0.5);
    }));
    ctx.globalAlpha = 1;
  }

  /** Start screen: title, hero shot, the cast, and the rules. */
  drawTitle(best: number, time: number): void {
    const { ctx, tile: t } = this;
    const W = this.width;
    const H = this.height;
    ctx.fillStyle = 'rgba(2,6,12,0.88)';
    ctx.fillRect(0, 0, W, H);
    ctx.textBaseline = 'middle';

    ctx.textAlign = 'center';
    ctx.font = `bold ${t * 2}px system-ui, sans-serif`;
    ctx.fillStyle = '#f2c53d';
    ctx.fillText('VOID HARVEST', W / 2, H * 0.09);
    ctx.font = `bold ${t * 0.7}px system-ui, sans-serif`;
    ctx.fillStyle = '#d7ccb8';
    ctx.fillText('Popolon and the Sunken Dungeon', W / 2, H * 0.09 + t * 1.6);

    const heroH = Math.min(H * 0.34, t * 11);
    const heroScale = heroH / PORTRAIT.height;
    const heroX = W * 0.14 - (PORTRAIT.width * heroScale) / 2;
    const heroY = H * 0.22 + Math.sin(time * 2) * t * 0.1;
    this.drawSprite(PORTRAIT, heroX, heroY, heroScale);
    ctx.font = `bold ${t * 0.8}px system-ui, sans-serif`;
    ctx.fillStyle = '#f2e3c4';
    ctx.fillText('POPOLON', W * 0.14, heroY + heroH + t * 0.9);
    ctx.font = `${t * 0.55}px system-ui, sans-serif`;
    ctx.fillStyle = '#8f8a80';
    ctx.fillText('the knight', W * 0.14, heroY + heroH + t * 1.7);

    const castH = Math.min(H * 0.17, t * 5.5);
    const castY = H * 0.24;
    CAST.forEach((c, i) => {
      const cx = W * (0.42 + i * 0.2);
      const scale = castH / c.sprite.height;
      const x = cx - (c.sprite.width * scale) / 2;
      const bob = Math.sin(time * 3 + i) * t * 0.08;
      this.drawSprite(c.sprite, x, castY + (castH - c.sprite.height * scale) + bob, scale);
      ctx.font = `bold ${t * 0.7}px system-ui, sans-serif`;
      ctx.fillStyle = c.color;
      ctx.fillText(c.name.toUpperCase(), cx, castY + castH + t * 0.9);
      ctx.font = `${t * 0.5}px system-ui, sans-serif`;
      ctx.fillStyle = '#8f8a80';
      this.wrapText(c.line, cx, castY + castH + t * 1.6, W * 0.19, t * 0.62);
    });

    let y = H * 0.6;
    ctx.font = `${t * 0.6}px system-ui, sans-serif`;
    for (const rule of RULES) {
      ctx.fillStyle = '#d7ccb8';
      y = this.wrapText(rule, W / 2, y, W * 0.86, t * 0.75) + t * 0.25;
    }

    ctx.fillStyle = '#8f8a80';
    ctx.font = `${t * 0.55}px system-ui, sans-serif`;
    ctx.fillText('Click to aim with the mouse   Hold W to run, A / D sidestep, S back', W / 2, H * 0.81);
    ctx.fillText('Space jump   E attack   B bomb   C camera   Esc pause   M mute', W / 2, H * 0.855);
    ctx.fillStyle = '#7fd7ff';
    ctx.fillText('Controller: left stick runs, right stick aims, A jump, X attack, B bomb, Start begins', W / 2, H * 0.90);
    ctx.fillStyle = '#8f8a80';
    const pulse = 0.7 + Math.sin(time * 4) * 0.3;
    ctx.globalAlpha = pulse;
    ctx.font = `bold ${t * 0.9}px system-ui, sans-serif`;
    ctx.fillStyle = '#e6e0d2';
    ctx.fillText('Press Enter or tap to begin', W / 2, H * 0.95);
    ctx.globalAlpha = 1;
    ctx.font = `${t * 0.5}px system-ui, sans-serif`;
    ctx.fillStyle = '#8f8a80';
    ctx.textAlign = 'right';
    ctx.fillText(`Best ${best}`, W - t * 0.6, H * 0.04);
  }

  /** Word-wrapped centered text. Returns the y just below the last line. */
  private wrapText(text: string, cx: number, y: number, maxWidth: number, lineHeight: number): number {
    const { ctx } = this;
    const words = text.split(' ');
    let line = '';
    for (const w of words) {
      const trial = line ? `${line} ${w}` : w;
      if (ctx.measureText(trial).width > maxWidth && line) {
        ctx.fillText(line, cx, y);
        y += lineHeight;
        line = w;
      } else {
        line = trial;
      }
    }
    if (line) {
      ctx.fillText(line, cx, y);
      y += lineHeight;
    }
    return y;
  }

  /** Centered overlay for ready, pause, win, and game over screens. */
  drawOverlay(lines: OverlayLine[], dim = 0.65): void {
    const { ctx, tile: t } = this;
    ctx.fillStyle = `rgba(2,6,12,${dim})`;
    ctx.fillRect(0, 0, this.width, this.height);
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    const totalH = lines.reduce((s, l) => s + (l.size ?? 1) * t * 1.4, 0);
    let y = this.height / 2 - totalH / 2;
    for (const l of lines) {
      const size = (l.size ?? 1) * t * 0.9;
      y += size * 0.7;
      ctx.font = `bold ${size}px system-ui, sans-serif`;
      ctx.fillStyle = l.color ?? PALETTE.text;
      ctx.fillText(l.text, this.width / 2, y);
      y += size * 0.85;
    }
  }

  /** Text at canvas pixel coordinates. */
  drawTextAt(px: number, py: number, text: string, alpha: number, color: string = PALETTE.frenziedWarn, scale = 0.55): void {
    const { ctx, tile: t } = this;
    ctx.globalAlpha = alpha;
    ctx.font = `bold ${t * scale}px system-ui, sans-serif`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillStyle = color;
    ctx.fillText(text, px, py);
    ctx.globalAlpha = 1;
  }

  /** Horizontal bar at canvas pixel coordinates, filled to `frac`. */
  drawBarAt(px: number, py: number, w: number, h: number, frac: number, color: string): void {
    const { ctx } = this;
    ctx.fillStyle = 'rgba(0,0,0,0.6)';
    ctx.fillRect(px - 1, py - 1, w + 2, h + 2);
    ctx.fillStyle = 'rgba(255,255,255,0.12)';
    ctx.fillRect(px, py, w, h);
    ctx.fillStyle = color;
    ctx.fillRect(px, py, w * Math.max(0, Math.min(1, frac)), h);
  }
}
