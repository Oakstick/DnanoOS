import { AudioManager } from '../engine/Audio';
import { Input } from '../engine/Input';
import { Rng } from '../engine/Rng';
import { Bomb, BOMBS_PER_LIFE } from '../entities/Bomb';
import { Diamond } from '../entities/Diamond';
import { defaultEnemySpecs, Enemy, EnemyContext, GlobalMode } from '../entities/Enemy';
import { CONTACT_DAMAGE, Hero } from '../entities/Hero';
import { INVISIBLE_SECONDS, Potion } from '../entities/Potion';
import { Projectile, spawnShots } from '../entities/Projectile';
import { ITEM_ORDER, ItemKind, ITEMS, RangedKind, WEAPON_ORDER, weaponLabel, WEAPONS } from '../entities/Weapons';
import { HudStatus, Renderer } from '../render/Renderer';
import { dirToYaw, optionColor, optionName, Renderer3D, STEP } from '../render3d/Renderer3D';
import { findAttackTargets, findProjectileHits, POINTS_KILL, tileInReach } from '../systems/Combat';
import { blastDamage, breakRocks, CHAIN_DELAY, inBlast } from '../systems/Explosion';
import { Frenzy } from '../systems/Frenzy';
import { PuddleField } from '../systems/Puddles';
import { MAX_LIVES, POINTS, Score } from '../systems/Score';
import { Grid } from '../world/Grid';
import { parseLevel, ParsedLevel } from '../world/Level';
import { levelFor } from '../world/levels';
import { CARDINALS, Dir, relativeDir, Tile, Vec2 } from '../world/types';

export type Screen = 'title' | 'ready' | 'playing' | 'paused' | 'dying' | 'won' | 'gameover';

/** Reinforcements: the first arrives after this long, then one every interval, up to the cap. */
/** Health a green cross gives back, capped at Popolon's maximum. */
const REMEDY_HEAL = 50;
const SPAWN_FIRST = 20;
const SPAWN_EVERY = 25;
const MAX_ENEMIES = 12;
const SPAWN_CYCLE: Enemy['kind'][] = ['bat', 'slime', 'vampire', 'skeleton'];

/** Alternating scatter/chase schedule in seconds. Frozen while enemies are stunned. */
const MODE_SCHEDULE: { mode: GlobalMode; seconds: number }[] = [
  { mode: 'scatter', seconds: 7 },
  { mode: 'chase', seconds: 20 },
  { mode: 'scatter', seconds: 7 },
  { mode: 'chase', seconds: 20 },
  { mode: 'scatter', seconds: 5 },
  { mode: 'chase', seconds: Infinity },
];

const READY_SECONDS = 2.2;
const DYING_SECONDS = 1.8;
const COLLISION_RADIUS_SQ = 0.45 * 0.45;

interface FloatingText {
  x: number;
  y: number;
  text: string;
  life: number;
  color?: string;
}

export class Game {
  screen: Screen = 'title';
  level!: ParsedLevel;
  grid!: Grid;
  hero!: Hero;
  enemies: Enemy[] = [];
  bombs: Bomb[] = [];
  bombsLeft = BOMBS_PER_LIFE;
  projectiles: Projectile[] = [];
  diamonds: Diamond<string>[] = [];
  potions: Potion[] = [];
  nuggetsTotal = 0;
  nuggetsGot = 0;
  readonly puddles = new PuddleField();
  private trailTile = new Map<Enemy, string>();
  private spawnTimer = SPAWN_FIRST;
  private spawned = 0;
  readonly frenzy = new Frenzy();
  readonly score = new Score();
  readonly audio = new AudioManager();
  readonly input: Input;
  readonly renderer: Renderer;
  readonly renderer3D: Renderer3D;
  showFps = false;
  private rng = new Rng();
  private wantedDir: Dir = 'none';
  /** The corridor the chase camera currently points down. */
  private camDir: Dir = 'none';

  private globalMode: GlobalMode = 'scatter';
  private modeIndex = 0;
  private modeTimer = MODE_SCHEDULE[0].seconds;
  private screenTimer = 0;
  private time = 0;
  private floaters: FloatingText[] = [];
  private nestSlots: Vec2[] = [];
  private renderFrames = 0;
  private renderFpsAt = 0;
  private renderFps = 0;

  constructor(canvas: HTMLCanvasElement, canvas3d: HTMLCanvasElement) {
    this.renderer = new Renderer(canvas);
    this.renderer3D = new Renderer3D(canvas3d);
    this.input = new Input(canvas);
    this.loadLevel(0);
    this.renderer.resize();
    window.addEventListener('resize', () => {
      this.renderer.resize();
      this.renderer3D.resize();
    });
  }

  // ---------------------------------------------------------------- setup

  loadLevel(index: number): void {
    this.score.level = index;
    this.level = parseLevel(levelFor(index));
    this.grid = this.level.grid;
    this.nuggetsTotal = this.grid.count(Tile.Nodule);
    this.nuggetsGot = 0;

    const c = this.level.nestCenter;
    this.nestSlots = [this.level.enemySpawn, { x: c.x - 2, y: c.y }, { x: c.x, y: c.y }, { x: c.x + 2, y: c.y }];

    // Clearing a dungeon keeps the held weapon and its ammo; only death takes them.
    const carried = this.hero?.alive && this.hero.hasRangedWeapon
      ? { weapon: this.hero.weapon as RangedKind, ammo: this.hero.ammo }
      : null;
    this.hero = new Hero(this.level.heroSpawn, this.level.data.heroSpeed);
    this.enemies = defaultEnemySpecs(this.grid).map(
      (spec, i) => new Enemy(spec, this.nestSlots[i] ?? c, this.level.data.enemySpeed),
    );
    this.diamonds = [
      ...this.level.weaponShrines.map((t, i) => new Diamond<string>('weapon', t, WEAPON_ORDER, i)),
      ...this.level.itemShrines.map((t, i) => new Diamond<string>('item', t, ITEM_ORDER, i)),
    ];
    this.potions = this.level.potions.map((p) => new Potion(p.tile, p.kind));
    this.spawnTimer = SPAWN_FIRST;
    this.spawned = 0;
    this.renderer3D.buildLevel(this.grid, this.enemies, this.diamonds, this.potions);
    this.resetRound();
    if (carried) {
      this.hero.equip(carried.weapon);
      this.hero.ammo = carried.ammo;
    }
  }

  /** Puts everyone back at spawn without touching collected gold. */
  private resetRound(): void {
    this.hero.respawn(this.level.heroSpawn);
    this.renderer3D.aimAlong(this.hero.facing);
    this.camDir = this.hero.facing;
    this.enemies.forEach((e, i) => e.reset(this.nestSlots[i] ?? this.level.nestCenter));
    this.modeIndex = 0;
    this.modeTimer = MODE_SCHEDULE[0].seconds;
    this.globalMode = MODE_SCHEDULE[0].mode;
    this.frenzy.cancel();
    this.audio.setFrenzy(false);
    this.floaters.length = 0;
    this.puddles.clear();
    this.trailTile.clear();
    this.input.clearDirection();
    this.wantedDir = 'none';
    this.camDir = 'none';
    this.bombs.forEach((b) => this.renderer3D.removeBomb(b));
    this.bombs = [];
    this.bombsLeft = BOMBS_PER_LIFE;
    this.projectiles.forEach((p) => this.renderer3D.removeProjectile(p));
    this.projectiles = [];
  }

  private startGame(): void {
    this.score.reset();
    this.hero.respawn(this.level.heroSpawn); // a new game starts with the bare sword
    this.loadLevel(0);
    this.enter('ready');
    this.audio.play('start');
  }

  private enter(screen: Screen): void {
    this.screen = screen;
    switch (screen) {
      case 'ready': this.screenTimer = READY_SECONDS; break;
      case 'dying': this.screenTimer = DYING_SECONDS; break;
      default: this.screenTimer = 0;
    }
  }

  // --------------------------------------------------------------- update

  update(dt: number): void {
    this.time += dt;
    this.handleGlobalKeys();

    switch (this.screen) {
      case 'title':
        if (this.input.wasPressed('Enter', 'Touch')) {
          this.audio.unlock();
          this.startGame();
        }
        break;
      case 'ready':
        this.screenTimer -= dt;
        if (this.screenTimer <= 0) this.enter('playing');
        break;
      case 'playing':
        this.tick(dt);
        break;
      case 'paused':
        break;
      case 'dying':
        this.screenTimer -= dt;
        if (this.screenTimer <= 0) {
          if (this.score.loseLife()) this.enter('gameover');
          else {
            this.resetRound();
            this.enter('ready');
          }
        }
        break;
      case 'won':
      case 'gameover':
        if (this.input.wasPressed('Enter', 'Touch')) this.enter('title');
        break;
    }
    this.renderer3D.update(dt, {
      grid: this.grid,
      hero: this.hero,
      enemies: this.enemies,
      frenzy: this.frenzy,
      time: this.time,
      bombs: this.bombs,
      projectiles: this.projectiles,
      diamonds: this.diamonds,
      potions: this.potions,
      puddles: this.puddles.values(),
    });
    this.input.flush();
  }

  private handleGlobalKeys(): void {
    if (this.input.wasPressed('F1')) this.showFps = !this.showFps;
    if (this.input.wasPressed('KeyM')) {
      this.audio.unlock();
      this.audio.toggleMute();
    }
    if (this.input.wasPressed('Escape')) {
      if (this.screen === 'playing') this.enter('paused');
      else if (this.screen === 'paused') this.enter('playing');
    }
    if (this.input.wasPressed('KeyC')) {
      this.renderer3D.cameraMode = this.renderer3D.cameraMode === 'third' ? 'top' : 'third';
    }

    // Mouse aiming only steers the chase camera, never the top-down view.
    const aiming = this.input.aiming && this.renderer3D.cameraMode === 'third';
    this.renderer3D.mouseLook = aiming;
    const look = this.input.takeLook();
    if (aiming && this.screen === 'playing') this.renderer3D.look(look.dx, look.dy);
  }

  /**
   * Movement input. In third person a key means "relative to where the camera
   * looks": W runs into the view, A and D step to either side, S backs out.
   * The corridor taken is the one nearest the camera's heading.
   */
  private resolveWantedDir(): Dir {
    const raw = this.input.direction;
    if (this.renderer3D.cameraMode !== 'third') return raw;
    if (raw === 'none') {
      this.wantedDir = 'none';
      return 'none';
    }
    this.wantedDir = relativeDir(raw, this.cameraFacing());
    return this.wantedDir;
  }

  /**
   * The corridor the camera is pointing down. A band of hysteresis keeps a
   * wobble near the diagonal from flipping him between two of them.
   */
  private cameraFacing(): Dir {
    const yaw = this.renderer3D.camYaw;
    const off = (d: Dir) => {
      let a = (yaw - dirToYaw(d)) % (Math.PI * 2);
      if (a > Math.PI) a -= Math.PI * 2;
      if (a < -Math.PI) a += Math.PI * 2;
      return Math.abs(a);
    };
    if (this.camDir !== 'none' && off(this.camDir) < Math.PI * 0.32) return this.camDir;
    let best: Dir = 'down';
    let bestOff = Infinity;
    for (const d of CARDINALS) {
      const a = off(d);
      if (a < bestOff) {
        bestOff = a;
        best = d;
      }
    }
    this.camDir = best;
    return best;
  }

  private tick(dt: number): void {
    this.updateModeSchedule(dt);

    this.hero.update(dt, this.grid, this.resolveWantedDir());
    this.collect();
    this.visitShrines();
    this.visitPotions();

    if (this.input.wasPressed('KeyE', 'Touch')) this.attack();
    if (this.input.wasPressed('Space') && this.hero.tryJump(this.grid)) this.audio.play('jump');
    if (this.input.wasPressed('KeyB', 'KeyL', 'ShiftLeft', 'ShiftRight')) this.placeBomb();
    this.updateProjectiles(dt);
    this.updateBombs(dt);
    for (const d of this.diamonds) d.update(dt);
    if (this.screen !== 'playing') return;

    if (this.frenzy.update(dt)) {
      this.enemies.forEach((e) => e.exitFrenzy(this.globalMode));
      this.audio.setFrenzy(false);
    }

    const skeleton = this.enemies.find((e) => e.kind === 'skeleton') ?? null;
    const ctx: EnemyContext = {
      grid: this.grid,
      hero: this.hero,
      skeleton,
      nestCenter: this.level.nestCenter,
      nestExit: this.level.enemySpawn,
      globalMode: this.globalMode,
      frenzyActive: this.frenzy.active,
      heroHidden: this.hero.invisible,
      rng: this.rng,
    };
    for (const e of this.enemies) e.update(dt, ctx);
    this.updateSpawns(dt);
    this.updateSlimeTrails(dt);

    this.resolveContact();
    if (this.screen !== 'playing') return;
    this.updateEffects(dt);
  }

  /** Reinforcements crawl out of the hollow on a schedule, cycling through the cast. */
  private updateSpawns(dt: number): void {
    this.spawnTimer -= dt;
    if (this.spawnTimer > 0 || this.enemies.length >= MAX_ENEMIES) return;
    this.spawnTimer = SPAWN_EVERY;
    const kind = SPAWN_CYCLE[this.spawned % SPAWN_CYCLE.length];
    const base = defaultEnemySpecs(this.grid).find((s) => s.kind === kind)!;
    const spec = { ...base, exitDelay: 0.5, name: `${base.name} ${Math.floor(this.spawned / SPAWN_CYCLE.length) + 2}` };
    const enemy = new Enemy(spec, this.level.nestCenter, this.level.data.enemySpeed);
    this.enemies.push(enemy);
    this.renderer3D.addEnemy(enemy);
    this.spawned++;
    this.audio.play('fuse');
    const c = this.level.nestCenter;
    this.floaters.push({ x: c.x, y: c.y - 1, text: 'SOMETHING STIRS', life: 1.6, color: '#b06cff' });
  }

  /** Moving slimes mark every tile they cross; standing in the slime slows the hero. */
  private updateSlimeTrails(dt: number): void {
    this.puddles.update(dt);
    for (const e of this.enemies) {
      if (e.kind !== 'slime' || e.mode === 'nest' || e.mode === 'eaten' || e.mode === 'leaving') continue;
      const o = e.occupied;
      const k = `${o.x},${o.y}`;
      if (this.trailTile.get(e) !== k) {
        this.trailTile.set(e, k);
        if (this.grid.get(o.x, o.y) !== Tile.Nest) this.puddles.add(o);
      }
    }
    if (this.hero.alive && !this.hero.airborne && this.puddles.has(this.hero.occupied)) this.hero.slow();
  }

  private updateModeSchedule(dt: number): void {
    if (this.frenzy.active) return;
    this.modeTimer -= dt;
    if (this.modeTimer <= 0 && this.modeIndex < MODE_SCHEDULE.length - 1) {
      this.modeIndex++;
      this.modeTimer = MODE_SCHEDULE[this.modeIndex].seconds;
      this.globalMode = MODE_SCHEDULE[this.modeIndex].mode;
      this.enemies.forEach((e) => {
        if (e.isDangerous) e.reverse(this.grid);
      });
    }
  }

  private collect(): void {
    if (this.hero.airborne) return; // pickups happen on the ground
    const o = this.hero.occupied;
    const got = this.grid.consume(o.x, o.y);
    if (got !== null) this.renderer3D.removeCollectible(o.x, o.y);
    if (got === Tile.Nodule) {
      this.nuggetsGot++;
      this.score.add(POINTS.nodule);
      this.audio.playReward(this.nuggetsGot);
      this.renderer3D.spark(o.x, o.y, 0xf2c53d);
      this.floaters.push({ x: o.x, y: o.y, text: `${this.nuggetsGot}/${this.nuggetsTotal}`, life: 0.9, color: '#f2c53d' });
      if (this.nuggetsGot >= this.nuggetsTotal) this.win();
    } else if (got === Tile.Charge) {
      const before = this.hero.hp;
      this.hero.heal(REMEDY_HEAL);
      const gained = this.hero.hp - before;
      this.score.add(POINTS.charge);
      this.audio.play('heal');
      this.renderer3D.spark(o.x, o.y, 0x4cf06a);
      this.floaters.push({
        x: o.x, y: o.y, life: 0.9, color: '#5cff6a',
        text: gained > 0 ? `+${gained} HP` : 'FULL HEALTH',
      });
    }
  }

  /** The last nugget: the dungeon is harvested. */
  private win(): void {
    this.audio.play('win');
    this.audio.setFrenzy(false);
    this.audio.setThreat(0);
    this.frenzy.cancel();
    this.renderer3D.spark(this.hero.pos.x, this.hero.pos.y, 0xf2c53d);
    this.enter('won');
  }

  /** Stepping onto an active shrine takes whatever it is showing. */
  private visitShrines(): void {
    if (this.hero.airborne) return;
    const o = this.hero.occupied;
    for (const d of this.diamonds) {
      if (!d.active || d.tile.x !== o.x || d.tile.y !== o.y) continue;
      const chosen = d.take();
      const name = d.kind === 'weapon' ? WEAPONS[chosen as RangedKind].name : ITEMS[chosen as ItemKind].name;
      const color = d.kind === 'weapon' ? WEAPONS[chosen as RangedKind].color : ITEMS[chosen as ItemKind].color;
      if (d.kind === 'weapon') {
        this.hero.equip(chosen as RangedKind);
      } else {
        switch (chosen as ItemKind) {
          case 'shield': this.hero.grantShield(); break;
          case 'invincible': this.hero.grantInvincibility(); break;
          case 'speed': this.hero.grantSpeed(); break;
        }
      }
      this.audio.play('start');
      this.floaters.push({ x: o.x, y: o.y, text: name.toUpperCase(), life: 1.4, color });
    }
  }

  /** Walking over a bottle drinks it. Bottles do not come back within a dungeon. */
  private visitPotions(): void {
    if (this.hero.airborne) return;
    const o = this.hero.occupied;
    for (const p of this.potions) {
      if (p.taken || p.tile.x !== o.x || p.tile.y !== o.y) continue;
      p.taken = true;
      let label = p.def.name.toUpperCase();
      switch (p.kind) {
        case 'life':
          if (!this.score.gainLife()) {
            this.score.add(500);
            label = 'FULL LIVES +500';
          }
          break;
        case 'shield': this.hero.grantShield(); break;
        case 'invisible': this.hero.grantInvisibility(INVISIBLE_SECONDS); break;
      }
      this.audio.play('start');
      this.renderer3D.spark(o.x, o.y, p.def.hex);
      this.floaters.push({ x: o.x, y: o.y, text: label, life: 1.4, color: p.def.color });
    }
  }

  /**
   * The one attack button. Facing a shrine, it cycles the shrine. Otherwise
   * it swings the sword, or fires the ranged weapon that has replaced it.
   */
  private attack(): void {
    if (!this.hero.alive) return;
    let cycled = false;
    for (const d of this.diamonds) {
      if (d.active && tileInReach(this.hero.pos, this.hero.facing, d.tile)) {
        d.cycle();
        cycled = true;
        this.audio.play('nodule');
        this.renderer3D.spark(d.tile.x, d.tile.y, 0xffffff);
      }
    }
    if (cycled) return;

    if (!this.hero.hasRangedWeapon) {
      if (this.hero.tryAttack()) this.performAttack();
      return;
    }
    this.fire();
  }

  /** Sword swing: slays every targetable enemy in front of the hero. */
  private performAttack(): void {
    this.audio.play('slash');
    const hits = findAttackTargets(this.hero.pos, this.hero.facing, this.enemies);
    for (const e of hits) this.slayEnemy(e);
  }

  private slayEnemy(e: Enemy): void {
    const value = e.isStunned ? this.frenzy.consumeValue() : POINTS_KILL;
    this.score.add(value);
    e.kill();
    this.audio.play('eat');
    this.renderer3D.spark(e.pos.x, e.pos.y, 0xffb0b0);
    this.floaters.push({ x: e.pos.x, y: e.pos.y, text: String(value), life: 1 });
  }

  /** Fires the held ranged weapon in the facing direction; the last shot hands the sword back. */
  private fire(): void {
    const kind = this.hero.tryFire();
    if (!kind) return;
    const shots = spawnShots(kind, this.hero.pos, this.hero.facing);
    for (const p of shots) {
      this.projectiles.push(p);
      this.renderer3D.addProjectile(p);
    }
    this.audio.play('arrow');
    if (!this.hero.hasRangedWeapon) {
      this.floaters.push({ x: this.hero.pos.x, y: this.hero.pos.y, text: 'SWORD', life: 1.2, color: '#cfe8f0' });
    }
  }

  private updateProjectiles(dt: number): void {
    for (const p of [...this.projectiles]) {
      const state = p.update(dt, this.grid, this.hero.pos);
      if (p.alive) {
        const hits = findProjectileHits(p.pos, p.def.hitRadius, this.enemies, p.struck);
        for (const e of hits) {
          this.slayEnemy(e);
          p.struck.add(e);
          this.audio.play('arrowHit');
          if (!p.def.pierce) {
            p.alive = false;
            break;
          }
        }
      } else if (state === 'wall') {
        this.renderer3D.spark(p.pos.x, p.pos.y);
      } else if (state === 'caught') {
        this.hero.refundShot();
        this.audio.play('nodule');
      }
      if (!p.alive) {
        this.projectiles = this.projectiles.filter((q) => q !== p);
        this.renderer3D.removeProjectile(p);
      }
    }
  }

  /** Drops a bomb on the hero's current tile, one per tile, from a stock of three. */
  private placeBomb(): void {
    if (this.bombsLeft <= 0 || !this.hero.alive || this.hero.airborne) return;
    const at = this.hero.occupied;
    if (this.bombs.some((b) => b.tile.x === at.x && b.tile.y === at.y)) return;
    const bomb = new Bomb(at);
    this.bombs.push(bomb);
    this.bombsLeft--;
    this.renderer3D.addBomb(bomb);
    this.audio.play('fuse');
  }

  private updateBombs(dt: number): void {
    for (const bomb of [...this.bombs]) {
      if (this.bombs.includes(bomb) && bomb.update(dt)) this.explode(bomb);
    }
  }

  /** TNT-style blast: breaks nearby walls, hurts everyone in range, chains to other bombs. */
  private explode(bomb: Bomb): void {
    this.bombs = this.bombs.filter((b) => b !== bomb);
    this.renderer3D.removeBomb(bomb);
    const c = bomb.tile;

    const broken = breakRocks(this.grid, c);
    for (const t of broken) {
      // A blasted wall becomes floor at the bomb's level.
      this.grid.setElevation(t.x, t.y, this.grid.elevation(c.x, c.y));
      this.renderer3D.removeRock(t.x, t.y);
    }
    this.renderer3D.explode(c.x, c.y, broken.length);
    this.audio.play('explode');

    for (const e of this.enemies) {
      if (e.isTargetable && inBlast(c, e.pos)) this.slayEnemy(e);
    }
    for (const other of this.bombs) {
      if (inBlast(c, other.tile)) other.ignite(CHAIN_DELAY.min + Math.random() * (CHAIN_DELAY.max - CHAIN_DELAY.min));
    }

    const dmg = blastDamage(Math.sqrt(Grid.distSq(c, this.hero.pos)));
    if (dmg > 0) this.hurtHero(dmg);
  }

  /** Enemy contact damages the hero; a dead hero starts the dying sequence. */
  private resolveContact(): void {
    if (this.hero.invisible) return; // they cannot find what they cannot see
    if (this.hero.airborne) return; // sailing over their heads
    const hp = this.hero.pos;
    for (const e of this.enemies) {
      if (!e.isDangerous) continue;
      const sameTile = e.occupied.x === this.hero.occupied.x && e.occupied.y === this.hero.occupied.y;
      if (!sameTile && Grid.distSq(e.pos, hp) > COLLISION_RADIUS_SQ) continue;
      if (this.hurtHero(CONTACT_DAMAGE)) return;
    }
  }

  /** Applies damage with shield and invincibility rules. Returns true if the hero died. */
  private hurtHero(amount: number): boolean {
    const result = this.hero.takeDamage(amount);
    const hp = this.hero.pos;
    if (result === 'blocked') {
      this.audio.play('nodule');
      this.renderer3D.spark(hp.x, hp.y, 0x7fd7ff);
      this.floaters.push({ x: hp.x, y: hp.y, text: 'BLOCKED', life: 0.8, color: '#7fd7ff' });
      return false;
    }
    if (result !== 'damaged') return false;
    this.audio.play('hurt');
    this.renderer3D.spark(hp.x, hp.y, 0xd8262e);
    if (!this.hero.alive) {
      this.killHero();
      return true;
    }
    return false;
  }

  private killHero(): void {
    this.audio.play('death');
    this.audio.setFrenzy(false);
    this.audio.setThreat(0);
    this.frenzy.cancel();
    this.renderer3D.explode(this.hero.pos.x, this.hero.pos.y, 0);
    this.enter('dying');
  }

  private updateEffects(dt: number): void {
    for (let i = this.floaters.length - 1; i >= 0; i--) {
      const f = this.floaters[i];
      f.life -= dt;
      f.y -= dt * 0.8;
      if (f.life <= 0) this.floaters.splice(i, 1);
    }
    let nearest = Infinity;
    for (const e of this.enemies) {
      if (e.isDangerous) nearest = Math.min(nearest, Grid.manhattan(e.pos, this.hero.pos));
    }
    this.audio.setThreat(nearest === Infinity ? 0 : Math.max(0, 1 - nearest / 10));
  }

  // --------------------------------------------------------------- render

  private hudStatus(): HudStatus {
    const label = weaponLabel(this.hero.weapon);
    return {
      hp: this.hero.hp,
      maxHp: this.hero.maxHp,
      bombs: this.bombsLeft,
      weaponKind: this.hero.weapon,
      weaponName: label.name,
      weaponColor: label.color,
      ammo: this.hero.ammo,
      shield: this.hero.shield,
      invincibleSeconds: this.hero.invincibleTimer,
      invisibleSeconds: this.hero.invisibleTimer,
      speedSeconds: this.hero.speedTimer,
      slowSeconds: this.hero.slowTimer,
      lives: this.score.lives,
      maxLives: MAX_LIVES,
      nuggetsGot: this.nuggetsGot,
      nuggetsTotal: this.nuggetsTotal,
      fps: this.showFps ? this.renderFps : undefined,
    };
  }

  render(): void {
    const r = this.renderer;

    this.renderFrames++;
    const now = performance.now();
    if (now - this.renderFpsAt >= 500) {
      this.renderFps = Math.round((this.renderFrames * 1000) / (now - this.renderFpsAt));
      this.renderFrames = 0;
      this.renderFpsAt = now;
    }

    this.renderer3D.render();
    r.begin();
    this.drawWorldOverlays();
    if (this.screen !== 'title') r.drawHUD(this.score, this.level.data.name, this.frenzy, this.hudStatus(), this.time);

    switch (this.screen) {
      case 'title':
        r.drawTitle(this.score.high, this.time);
        break;
      case 'ready':
        r.drawOverlay([
          { text: this.level.data.name.toUpperCase(), size: 1.1, color: '#f2c53d' },
          { text: 'DESCENDING...', size: 0.8, color: '#d7ccb8' },
        ], 0.35);
        break;
      case 'paused':
        r.drawOverlay([{ text: 'PAUSED', size: 1.6 }, { text: 'Esc to resume', size: 0.7, color: '#d7ccb8' }]);
        break;
      case 'won':
        r.drawOverlay([
          { text: 'YOU WON', size: 2.2, color: '#f2c53d' },
          { text: 'Every nugget in the dungeon is yours.', size: 0.9, color: '#d7ccb8' },
          { text: `Gold ${this.score.score}   Lives left ${this.score.lives}`, size: 0.9 },
          { text: `Best ${this.score.high}`, size: 0.8, color: '#d7ccb8' },
          { text: 'Enter or tap to play again', size: 0.8, color: '#8f8a80' },
        ], 0.7);
        break;
      case 'gameover':
        r.drawOverlay([
          { text: 'POPOLON HAS FALLEN', size: 1.6, color: '#d8262e' },
          { text: `Gold ${this.score.score}`, size: 0.9 },
          { text: `Best ${this.score.high}`, size: 0.8, color: '#d7ccb8' },
          { text: 'Enter or tap to try again', size: 0.8, color: '#8f8a80' },
        ]);
        break;
      default:
        break;
    }
  }

  /** Health bar over Popolon, shrine labels, and floating text projected onto the overlay canvas. */
  private drawWorldOverlays(): void {
    const r = this.renderer;

    if (this.hero.alive && this.screen === 'playing') {
      const hp = this.hero.pos;
      const p = this.renderer3D.project(hp.x, this.hero.altitude * STEP + 1.45, hp.y);
      if (p) {
        const w = r.tile * 1.6;
        const h = Math.max(4, r.tile * 0.18);
        const frac = this.hero.hp / this.hero.maxHp;
        const color = frac > 0.5 ? '#5cff6a' : frac > 0.25 ? '#f2c53d' : '#d8262e';
        r.drawBarAt(p.x - w / 2, p.y, w, h, frac, color);
      }
    }
    for (const d of this.diamonds) {
      if (!d.active) continue;
      const p = this.renderer3D.project(d.tile.x, this.renderer3D.floorHeight(d.tile.x, d.tile.y) + 1.25, d.tile.y);
      if (p) r.drawTextAt(p.x, p.y, optionName(d).toUpperCase(), 0.95, optionColor(d), 0.45);
    }
    for (const f of this.floaters) {
      const p = this.renderer3D.project(f.x, this.renderer3D.floorHeight(f.x, f.y) + 0.9 + (1 - f.life) * 0.6, f.y);
      if (p) r.drawTextAt(p.x, p.y, f.text, Math.min(1, f.life * 2), f.color ?? '#f2c53d');
    }
  }
}
