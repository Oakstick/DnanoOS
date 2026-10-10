import { describe, expect, it } from 'vitest';
import { CONTACT_DAMAGE, Hero, HERO_MAX_HP } from '../src/entities/Hero';
import { ITEMS, SPEED_BOOST, WEAPONS } from '../src/entities/Weapons';
import { Grid } from '../src/world/Grid';
import { Tile } from '../src/world/types';

function corridor(): Grid {
  const R = Tile.Rock;
  const O = Tile.Open;
  return new Grid(5, 3, [R, R, R, R, R, R, O, O, O, R, R, R, R, R, R]);
}

describe('Hero', () => {
  it('starts at full health and takes contact damage', () => {
    const h = new Hero({ x: 1, y: 1 }, 5);
    expect(h.hp).toBe(HERO_MAX_HP);
    expect(h.takeDamage(CONTACT_DAMAGE)).toBe('damaged');
    expect(h.hp).toBe(HERO_MAX_HP - CONTACT_DAMAGE);
    expect(h.alive).toBe(true);
  });

  it('is immune for a short window after a hit', () => {
    const h = new Hero({ x: 1, y: 1 }, 5);
    h.takeDamage(10);
    expect(h.invulnerable).toBe(true);
    expect(h.takeDamage(10)).toBe('immune');
    expect(h.hp).toBe(HERO_MAX_HP - 10);
    const g = corridor();
    for (let i = 0; i < 70; i++) h.update(1 / 60, g, 'none');
    expect(h.invulnerable).toBe(false);
    expect(h.takeDamage(10)).toBe('damaged');
  });

  it('dies when health reaches zero', () => {
    const h = new Hero({ x: 1, y: 1 }, 5);
    for (let i = 0; i < 4; i++) {
      h.takeDamage(CONTACT_DAMAGE);
      h.invuln = 0;
    }
    expect(h.hp).toBe(0);
    expect(h.alive).toBe(false);
    expect(h.takeDamage(1)).toBe('immune');
  });

  it('a shield absorbs hits until its charges run out', () => {
    const h = new Hero({ x: 1, y: 1 }, 5);
    h.grantShield();
    expect(h.shield).toBe(ITEMS.shield.charges);
    for (let i = 0; i < ITEMS.shield.charges; i++) {
      expect(h.takeDamage(CONTACT_DAMAGE)).toBe('blocked');
      h.invuln = 0;
    }
    expect(h.hp).toBe(HERO_MAX_HP);
    expect(h.shield).toBe(0);
    expect(h.takeDamage(CONTACT_DAMAGE)).toBe('damaged');
  });

  it('invincibility ignores damage and expires', () => {
    const h = new Hero({ x: 1, y: 1 }, 5);
    h.grantInvincibility();
    expect(h.invincible).toBe(true);
    expect(h.takeDamage(CONTACT_DAMAGE)).toBe('immune');
    const g = corridor();
    for (let i = 0; i < (ITEMS.invincible.seconds + 0.5) * 60; i++) h.update(1 / 60, g, 'none');
    expect(h.invincible).toBe(false);
    expect(h.takeDamage(CONTACT_DAMAGE)).toBe('damaged');
  });

  it('speed boosts movement and wears off', () => {
    const h = new Hero({ x: 1, y: 1 }, 5);
    const g = corridor();
    h.grantSpeed();
    h.update(1 / 60, g, 'none');
    expect(h.speed).toBeCloseTo(5 * SPEED_BOOST);
    for (let i = 0; i < (ITEMS.speed.seconds + 0.5) * 60; i++) h.update(1 / 60, g, 'none');
    expect(h.speed).toBeCloseTo(5);
  });

  it('starts with the sword, which cannot be fired', () => {
    const h = new Hero({ x: 1, y: 1 }, 5);
    expect(h.weapon).toBe('sword');
    expect(h.hasRangedWeapon).toBe(false);
    expect(h.tryFire()).toBeNull();
    expect(h.tryAttack()).toBe(true);
  });

  it('a shrine weapon replaces the sword, fires on cooldown, and refunds caught boomerangs', () => {
    const h = new Hero({ x: 1, y: 1 }, 5);
    h.equip('boomerang');
    expect(h.hasRangedWeapon).toBe(true);
    expect(h.ammo).toBe(WEAPONS.boomerang.ammo);
    expect(h.tryFire()).toBe('boomerang');
    expect(h.tryFire()).toBeNull(); // cooldown
    expect(h.ammo).toBe(WEAPONS.boomerang.ammo - 1);
    h.refundShot();
    expect(h.ammo).toBe(WEAPONS.boomerang.ammo);
    h.refundShot();
    expect(h.ammo).toBe(WEAPONS.boomerang.ammo); // never above the stock
  });

  it('the last shot hands the sword back', () => {
    const h = new Hero({ x: 1, y: 1 }, 5);
    const g = corridor();
    h.equip('fireball');
    for (let shot = 0; shot < WEAPONS.fireball.ammo; shot++) {
      expect(h.tryFire()).toBe('fireball');
      for (let i = 0; i < 40; i++) h.update(1 / 60, g, 'none');
    }
    expect(h.weapon).toBe('sword');
    expect(h.ammo).toBe(0);
    expect(h.tryFire()).toBeNull();
    h.refundShot(); // no boomerang in hand, nothing to refund
    expect(h.ammo).toBe(0);
  });

  it('death costs everything: health back, bare sword, all boons gone', () => {
    const h = new Hero({ x: 1, y: 1 }, 5);
    h.equip('fireball');
    h.grantShield();
    h.grantSpeed();
    h.grantInvisibility(5);
    h.takeDamage(HERO_MAX_HP);
    h.respawn({ x: 2, y: 1 });
    expect(h.alive).toBe(true);
    expect(h.hp).toBe(HERO_MAX_HP);
    expect(h.weapon).toBe('sword');
    expect(h.ammo).toBe(0);
    expect(h.shield).toBe(0);
    expect(h.hasted).toBe(false);
    expect(h.invisible).toBe(false);
    expect(h.tile).toEqual({ x: 2, y: 1 });
  });

  it('attacks respect the cooldown', () => {
    const h = new Hero({ x: 1, y: 1 }, 5);
    expect(h.tryAttack()).toBe(true);
    expect(h.swinging).toBe(true);
    expect(h.tryAttack()).toBe(false);
    const g = corridor();
    for (let i = 0; i < 30; i++) h.update(1 / 60, g, 'none');
    expect(h.swinging).toBe(false);
    expect(h.tryAttack()).toBe(true);
  });

  it('remembers its facing after stopping', () => {
    const h = new Hero({ x: 1, y: 1 }, 4);
    const g = corridor();
    for (let i = 0; i < 120; i++) h.update(1 / 60, g, 'right');
    expect(h.dir).toBe('none');
    expect(h.facing).toBe('right');
  });
});
