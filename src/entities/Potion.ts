import { PotionKind, Vec2 } from '../world/types';

export interface PotionDef {
  name: string;
  color: string;
  /** Liquid color as a hex number for the 3D bottle. */
  hex: number;
}

export const POTIONS: Record<PotionKind, PotionDef> = {
  life: { name: 'Extra Life', color: '#ff4d6d', hex: 0xff4d6d },
  shield: { name: 'Shield', color: '#4da3ff', hex: 0x4da3ff },
  invisible: { name: 'Invisibility', color: '#b06cff', hex: 0xb06cff },
};

export const INVISIBLE_SECONDS = 10;

/** A bottle lying on a tile. Taken once, never respawns within the level. */
export class Potion {
  readonly tile: Vec2;
  readonly kind: PotionKind;
  taken = false;

  constructor(tile: Vec2, kind: PotionKind) {
    this.tile = { ...tile };
    this.kind = kind;
  }

  get def(): PotionDef {
    return POTIONS[this.kind];
  }
}
