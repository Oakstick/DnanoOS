/** Ranged weapons offered by the weapon shrine, in cycle order. */
export type RangedKind = 'arrow' | 'double' | 'trishot' | 'boomerang' | 'fireball' | 'crescent';
/** Everything the hero can hold: the default sword, or one ranged weapon that replaces it. */
export type WeaponKind = 'sword' | RangedKind;

export const SWORD = { name: 'Sword', color: '#cfe8f0' } as const;

export interface WeaponDef {
  name: string;
  color: string;
  /** Shots granted on pickup and on each new life. */
  ammo: number;
  /** Tiles per second. */
  speed: number;
  /** Tiles before the shot drops (or, for the boomerang, turns back). */
  range: number;
  /** Distance from the shot's center at which an enemy is hit. */
  hitRadius: number;
  /** Keeps flying after a kill. */
  pierce: boolean;
  /** Flies back along its path and refunds the shot if caught. */
  returns: boolean;
  /** Lateral offsets (tiles) for parallel shots; one entry per projectile. */
  lateral: number[];
  /** Also fire perpendicular copies (left and right of facing). */
  spread: boolean;
  cooldown: number;
}

export const WEAPON_ORDER: RangedKind[] = ['arrow', 'double', 'trishot', 'boomerang', 'fireball', 'crescent'];

export const WEAPONS: Record<RangedKind, WeaponDef> = {
  arrow: {
    name: 'Arrow', color: '#e0c9a6', ammo: 7, speed: 14, range: 14, hitRadius: 0.5,
    pierce: false, returns: false, lateral: [0], spread: false, cooldown: 0.25,
  },
  double: {
    name: 'Double Arrow', color: '#f2e3c4', ammo: 7, speed: 14, range: 14, hitRadius: 0.5,
    pierce: false, returns: false, lateral: [-0.28, 0.28], spread: false, cooldown: 0.3,
  },
  trishot: {
    name: 'Tri-Shot', color: '#9ad1ff', ammo: 6, speed: 13, range: 12, hitRadius: 0.5,
    pierce: false, returns: false, lateral: [0], spread: true, cooldown: 0.35,
  },
  boomerang: {
    name: 'Boomerang', color: '#c98f4a', ammo: 5, speed: 10, range: 5, hitRadius: 0.6,
    pierce: true, returns: true, lateral: [0], spread: false, cooldown: 0.4,
  },
  fireball: {
    name: 'Fireball', color: '#ff8a2a', ammo: 5, speed: 8, range: 12, hitRadius: 0.6,
    pierce: true, returns: false, lateral: [0], spread: false, cooldown: 0.45,
  },
  crescent: {
    name: 'Crescent', color: '#8fe7ff', ammo: 6, speed: 16, range: 6, hitRadius: 0.9,
    pierce: true, returns: false, lateral: [0], spread: false, cooldown: 0.3,
  },
};

/** Boons offered by the item shrine, in cycle order. */
export type ItemKind = 'shield' | 'invincible' | 'speed';

export interface ItemDef {
  name: string;
  color: string;
  /** Duration in seconds for timed boons. */
  seconds: number;
  /** Hits absorbed for the shield. */
  charges: number;
}

export const ITEM_ORDER: ItemKind[] = ['shield', 'invincible', 'speed'];

export const ITEMS: Record<ItemKind, ItemDef> = {
  shield: { name: 'Shield', color: '#7fd7ff', seconds: 0, charges: 3 },
  invincible: { name: 'Invincibility', color: '#ffd84a', seconds: 8, charges: 0 },
  speed: { name: 'Speed', color: '#7dff9a', seconds: 15, charges: 0 },
};

export const SPEED_BOOST = 1.4;

/** Display name and color for any held weapon, sword included. */
export function weaponLabel(kind: WeaponKind): { name: string; color: string } {
  return kind === 'sword' ? SWORD : { name: WEAPONS[kind].name, color: WEAPONS[kind].color };
}
