//! The cast, sculpted in code rather than typed as text grids: boxes,
//! ellipsoids and discs carved into a voxel grid, with a little color jitter
//! so surfaces read as hand-painted rather than flat. Interior voxels are
//! culled, so detail costs only what the camera can see.
//!
//! Axes follow the text models: y up from the feet, z from the back (0) to
//! the front (the face), x across. Characters face +z.

use super::voxels::{Collected, PartDef, Voxel};
use crate::logic::gear::ShieldKind;

pub struct Sculpt {
    w: i32,
    h: i32,
    d: i32,
    cells: Vec<Option<u32>>,
}

impl Sculpt {
    pub fn new(w: i32, h: i32, d: i32) -> Self {
        Self { w, h, d, cells: vec![None; (w * h * d) as usize] }
    }

    fn idx(&self, x: i32, y: i32, z: i32) -> Option<usize> {
        if x < 0 || y < 0 || z < 0 || x >= self.w || y >= self.h || z >= self.d {
            return None;
        }
        Some(((y * self.d + z) * self.w + x) as usize)
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, c: u32) {
        if let Some(i) = self.idx(x, y, z) {
            self.cells[i] = Some(c);
        }
    }

    pub fn clear(&mut self, x: i32, y: i32, z: i32) {
        if let Some(i) = self.idx(x, y, z) {
            self.cells[i] = None;
        }
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> Option<u32> {
        self.idx(x, y, z).and_then(|i| self.cells[i])
    }

    /// Inclusive box.
    #[allow(clippy::too_many_arguments)]
    pub fn cuboid(&mut self, x0: i32, y0: i32, z0: i32, x1: i32, y1: i32, z1: i32, c: u32) {
        for y in y0..=y1 {
            for z in z0..=z1 {
                for x in x0..=x1 {
                    self.set(x, y, z, c);
                }
            }
        }
    }

    /// Ellipsoid around a (possibly fractional) center, clipped to `y >= ymin`.
    #[allow(clippy::too_many_arguments)]
    pub fn ellipsoid(&mut self, c: (f32, f32, f32), r: (f32, f32, f32), ymin: i32, color: u32) {
        for y in ymin.max(0)..self.h {
            for z in 0..self.d {
                for x in 0..self.w {
                    let dx = (x as f32 - c.0) / r.0;
                    let dy = (y as f32 - c.1) / r.1;
                    let dz = (z as f32 - c.2) / r.2;
                    if dx * dx + dy * dy + dz * dz <= 1.0 {
                        self.set(x, y, z, color);
                    }
                }
            }
        }
    }

    /// Fills a tapered capsule from `a` (radius `ra`) to `b` (radius `rb`), in voxel units.
    #[allow(clippy::too_many_arguments)]
    pub fn capsule(&mut self, a: (f32, f32, f32), b: (f32, f32, f32), ra: f32, rb: f32, c: u32) {
        let r = ra.max(rb) + 1.0;
        let lo = (a.0.min(b.0) - r, a.1.min(b.1) - r, a.2.min(b.2) - r);
        let hi = (a.0.max(b.0) + r, a.1.max(b.1) + r, a.2.max(b.2) + r);
        let ab = (b.0 - a.0, b.1 - a.1, b.2 - a.2);
        let len2 = (ab.0 * ab.0 + ab.1 * ab.1 + ab.2 * ab.2).max(1e-6);
        for y in lo.1.floor() as i32..=hi.1.ceil() as i32 {
            for z in lo.2.floor() as i32..=hi.2.ceil() as i32 {
                for x in lo.0.floor() as i32..=hi.0.ceil() as i32 {
                    let p = (x as f32 - a.0, y as f32 - a.1, z as f32 - a.2);
                    let t = ((p.0 * ab.0 + p.1 * ab.1 + p.2 * ab.2) / len2).clamp(0.0, 1.0);
                    let d = (p.0 - ab.0 * t, p.1 - ab.1 * t, p.2 - ab.2 * t);
                    let rad = ra + (rb - ra) * t;
                    if d.0 * d.0 + d.1 * d.1 + d.2 * d.2 <= rad * rad {
                        self.set(x, y, z, c);
                    }
                }
            }
        }
    }

    /// Recolors every filled voxel the predicate picks.
    pub fn paint(&mut self, mut f: impl FnMut(i32, i32, i32, u32) -> Option<u32>) {
        for y in 0..self.h {
            for z in 0..self.d {
                for x in 0..self.w {
                    if let Some(c) = self.get(x, y, z) {
                        if let Some(n) = f(x, y, z, c) {
                            self.set(x, y, z, n);
                        }
                    }
                }
            }
        }
    }

    /// Finishes the model: jitters colors a few percent and drops fully enclosed voxels.
    pub fn finish(self, jitter: f32) -> Collected {
        let mut voxels = Vec::new();
        for y in 0..self.h {
            for z in 0..self.d {
                for x in 0..self.w {
                    let Some(c) = self.get(x, y, z) else { continue };
                    let hidden = [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)]
                        .iter()
                        .all(|(dx, dy, dz)| self.get(x + dx, y + dy, z + dz).is_some());
                    if hidden {
                        continue;
                    }
                    let k = 1.0 + (noise(x, y, z) - 0.5) * 2.0 * jitter;
                    voxels.push(Voxel { x, y, z, c: scale(c, k) });
                }
            }
        }
        Collected { voxels, width: self.w, depth: self.d, height: self.h }
    }
}

fn noise(x: i32, y: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841) ^ (z as u32).wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 16;
    (h & 0xffff) as f32 / 65535.0
}

fn scale(c: u32, k: f32) -> u32 {
    let ch = |s: u32| ((((c >> s) & 255) as f32 * k).clamp(0.0, 255.0) as u32) << s;
    ch(16) | ch(8) | ch(0) | (c & 0xff00_0000)
}

// ------------------------------------------------------------------ Ragnar

mod viking {
    pub const SKIN: u32 = 0xf0c49c;
    pub const SKIN_D: u32 = 0xd49e76;
    pub const BEARD: u32 = 0xc4581c;
    pub const BEARD_L: u32 = 0xe2823a;
    pub const BEARD_D: u32 = 0x93400f;
    pub const EYE: u32 = 0x1d3563;
    pub const HELM: u32 = 0x8d96a3;
    pub const HELM_D: u32 = 0x5e6672;
    pub const BRONZE: u32 = 0xc8903c;
    pub const HORN: u32 = 0xf0e6cc;
    pub const HORN_TIP: u32 = 0xc9b88f;
    pub const TUNIC: u32 = 0x2e5f96;
    pub const TUNIC_D: u32 = 0x224a78;
    pub const MAIL: u32 = 0xa3abb6;
    pub const MAIL_D: u32 = 0x767e89;
    pub const BELT: u32 = 0x5c3a1c;
    pub const GOLD: u32 = 0xf2c53d;
    pub const FUR: u32 = 0x8f6c49;
    pub const FUR_L: u32 = 0xc2a27c;
    pub const CLOAK: u32 = 0x7d2a24;
    pub const TROUSER: u32 = 0x55593f;
    pub const WRAP: u32 = 0xbfa272;
    pub const BOOT: u32 = 0x4e321a;
    pub const LEATHER: u32 = 0x7a4a24;
    pub const SHIELD_RED: u32 = 0xb8242a;
    pub const SHIELD_CREAM: u32 = 0xe9dcbc;
    pub const IRON: u32 = 0x5a616b;
    pub const HAFT: u32 = 0x6b3f1d;
    pub const STEEL: u32 = 0xc8d0da;
    pub const EDGE: u32 = 0xf2f5f8;
}

/// Ragnar the viking with his starting kit: 15 wide, 9 deep, 23 tall.
pub fn ragnar() -> Collected {
    ragnar_geared(1, Some(ShieldKind::Round))
}

/**
 * Ragnar carrying a given axe (level 1 bearded axe, 2 battle axe, 3
 * double-bladed great axe) and shield on his back (or none, if it broke).
 */
pub fn ragnar_geared(axe_level: u8, shield: Option<ShieldKind>) -> Collected {
    use viking::*;
    let mut s = Sculpt::new(15, 23, 9);
    // Boots with fur cuffs, wrapped trousers.
    for (x0, x1) in [(4, 6), (8, 10)] {
        s.cuboid(x0, 0, 3, x1, 1, 6, BOOT);
        s.cuboid(x0, 2, 3, x1, 2, 5, FUR_L);
        s.cuboid(x0, 3, 3, x1, 6, 5, TROUSER);
        for y in [3, 5] {
            s.cuboid(x0, y, 3, x1, y, 5, WRAP);
        }
    }
    // Tunic, skirt hem and belt with a gold buckle and a pouch.
    s.cuboid(3, 7, 2, 11, 13, 5, TUNIC);
    s.cuboid(3, 7, 2, 11, 7, 5, TUNIC_D);
    s.cuboid(3, 8, 2, 11, 8, 5, BELT);
    s.set(7, 8, 5, GOLD);
    s.cuboid(9, 7, 6, 10, 8, 6, LEATHER);
    // Chainmail vest over the chest.
    for y in 9..=13 {
        for x in 4..=10 {
            s.set(x, y, 5, if (x + y) % 2 == 0 { MAIL } else { MAIL_D });
        }
    }
    // Fur mantle on the shoulders, red cloak down the back.
    s.cuboid(2, 13, 1, 12, 14, 5, FUR);
    s.paint(|x, y, z, c| if c == FUR && (x + z + y) % 3 == 0 { Some(FUR_L) } else { None });
    s.cuboid(3, 7, 1, 11, 12, 1, CLOAK);
    // The shield slung on his back.
    if let Some(kind) = shield {
        back_shield(&mut s, kind);
    }
    // Arms: sleeves, leather bracers, hands.
    for (x0, x1) in [(1, 2), (12, 13)] {
        s.cuboid(x0, 11, 3, x1, 13, 4, TUNIC);
        s.cuboid(x0, 9, 3, x1, 10, 4, LEATHER);
        s.cuboid(x0, 7, 3, x1, 8, 4, SKIN);
    }
    // The axe in his right hand.
    hand_axe(&mut s, axe_level);
    // Neck and head.
    s.cuboid(6, 14, 3, 8, 14, 4, SKIN);
    s.cuboid(5, 15, 2, 9, 19, 5, SKIN);
    s.cuboid(5, 15, 2, 9, 19, 2, BEARD_D); // hair at the back
    s.set(6, 17, 5, EYE);
    s.set(8, 17, 5, EYE);
    s.set(6, 18, 5, BEARD_D);
    s.set(8, 18, 5, BEARD_D);
    s.set(7, 16, 6, SKIN_D); // nose
    // The beard: full cheeks, a mustache, two braids with gold beads.
    s.cuboid(5, 15, 5, 9, 16, 5, BEARD);
    s.cuboid(5, 13, 6, 9, 16, 6, BEARD);
    s.cuboid(6, 12, 6, 8, 12, 6, BEARD);
    s.cuboid(6, 13, 7, 8, 15, 7, BEARD);
    for x in [5, 6, 8, 9] {
        s.set(x, 16, 6, BEARD_L);
    }
    s.set(7, 16, 6, SKIN_D);
    for x in [6, 8] {
        s.set(x, 11, 6, BEARD);
        s.set(x, 10, 6, GOLD);
    }
    s.paint(|x, y, _z, c| if c == BEARD && (x * 3 + y) % 4 == 0 { Some(BEARD_L) } else { None });
    // Braid down the back.
    for y in 11..=14 {
        s.set(7, y, 1, BEARD);
    }
    s.set(7, 11, 1, GOLD);
    // Helmet: an iron dome with a bronze band, a ridge and a nose guard.
    s.ellipsoid((7.0, 19.0, 3.5), (3.0, 3.3, 2.9), 19, HELM);
    s.paint(|_x, y, _z, c| if c == HELM && y == 19 { Some(BRONZE) } else { None });
    for z in 1..=6 {
        if s.get(7, 21, z).is_some() {
            s.set(7, 22, z, HELM_D);
        }
    }
    s.set(7, 18, 6, HELM);
    s.set(7, 17, 6, HELM);
    // Horns sweeping up and out.
    for (x, y) in [(3, 19), (2, 20), (1, 21)] {
        for z in 3..=4 {
            s.set(x, y, z, HORN);
            s.set(14 - x, y, z, HORN);
        }
    }
    s.set(1, 22, 3, HORN_TIP);
    s.set(13, 22, 3, HORN_TIP);
    s.finish(0.05)
}

/// The axe at each level, in the right hand (x = 13).
fn hand_axe(s: &mut Sculpt, level: u8) {
    use viking::*;
    const DARK_STEEL: u32 = 0x8a929e;
    const WRAP_RED: u32 = 0x9a1c22;
    const GILT: u32 = 0xffd84a;
    const DARK_HAFT: u32 = 0x3e2412;
    match level {
        1 => {
            for y in 5..=17 {
                s.set(13, y, 4, HAFT);
            }
            for (z, y0, y1) in [(5, 14, 17), (6, 13, 17), (7, 12, 18)] {
                for y in y0..=y1 {
                    s.set(13, y, z, STEEL);
                }
            }
            for y in 11..=18 {
                s.set(13, y, 8, EDGE);
            }
            s.set(13, 11, 7, STEEL);
        }
        2 => {
            // Battle axe: a red-wrapped haft, a broad dark blade and a back spike.
            for y in 4..=18 {
                s.set(13, y, 4, if y % 2 == 0 && (9..=13).contains(&y) { WRAP_RED } else { HAFT });
            }
            for (z, y0, y1) in [(5, 13, 18), (6, 12, 18), (7, 11, 19)] {
                for y in y0..=y1 {
                    s.set(13, y, z, DARK_STEEL);
                }
            }
            for y in 10..=19 {
                s.set(13, y, 8, EDGE);
            }
            for z in 2..=3 {
                s.set(13, 16, z, DARK_STEEL);
            }
            s.set(13, 19, 4, DARK_STEEL);
        }
        _ => {
            // Great axe: dark haft with gold bands, a blade either side, gilded edges.
            for y in 3..=18 {
                s.set(13, y, 4, if y % 4 == 1 { GILT } else { DARK_HAFT });
            }
            for (z, y0, y1) in [(5, 12, 18), (6, 11, 18), (7, 10, 19)] {
                for y in y0..=y1 {
                    s.set(13, y, z, STEEL);
                }
            }
            for y in 9..=19 {
                s.set(13, y, 8, GILT);
            }
            for (z, y0, y1) in [(3, 12, 18), (2, 11, 18)] {
                for y in y0..=y1 {
                    s.set(13, y, z, STEEL);
                }
            }
            for y in 10..=19 {
                s.set(13, y, 1, GILT);
            }
            s.set(13, 19, 4, GILT);
        }
    }
    s.cuboid(12, 7, 3, 13, 8, 4, SKIN);
}

/// A shield on his back (z = 0): its shape and paint tell the four apart.
fn back_shield(s: &mut Sculpt, kind: ShieldKind) {
    use viking::*;
    const WOOD: u32 = 0x9a6a3a;
    const WOOD_D: u32 = 0x7a522a;
    const STEEL_D: u32 = 0x6e7888;
    const PLATE: u32 = 0xa8b4c4;
    const RIVET: u32 = 0xe8eef4;
    for y in 3..=16 {
        for x in 2..=12 {
            let (dx, dy) = (x as f32 - 7.0, y as f32 - 10.0);
            let c = match kind {
                ShieldKind::Round | ShieldKind::IronRound => {
                    let r = (dx * dx + dy * dy).sqrt();
                    if r > 4.4 {
                        continue;
                    }
                    if kind == ShieldKind::Round {
                        if r < 1.2 {
                            IRON
                        } else if r > 3.6 {
                            IRON
                        } else if x % 2 == 0 {
                            WOOD
                        } else {
                            WOOD_D
                        }
                    } else if r < 1.2 {
                        GOLD
                    } else if r > 3.6 {
                        STEEL_D
                    } else if (r - 2.6).abs() < 0.35 && (x + y) % 2 == 0 {
                        RIVET
                    } else {
                        MAIL
                    }
                }
                ShieldKind::Kite | ShieldKind::Dreadnought => {
                    // Broad at the top, tapering to a point at the bottom.
                    let half = if y >= 11 { 4.5 } else { 4.5 * (y as f32 - 3.0) / 8.0 + 0.5 };
                    if dx.abs() > half {
                        continue;
                    }
                    let rim = dx.abs() > half - 1.0 || y == 16;
                    if kind == ShieldKind::Kite {
                        if rim {
                            IRON
                        } else if (dx >= 0.0) != (dy >= 1.0) {
                            SHIELD_RED
                        } else {
                            SHIELD_CREAM
                        }
                    } else if rim {
                        GOLD
                    } else if (x + y) % 3 == 0 && dx.abs() > half - 2.2 {
                        RIVET
                    } else if dx.abs() < 0.6 || y == 12 {
                        STEEL_D
                    } else {
                        PLATE
                    }
                }
            };
            s.set(x, y, 0, c);
        }
    }
}

/// Ragnar's joints. Leg boxes stop short of the cloak and shield on his back.
pub const RAGNAR_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[([3, 0, 2], [6, 6, 8])], pivot: [5.0, 7.0, 4.0] },
    PartDef { name: "rightLeg", boxes: &[([8, 0, 2], [11, 6, 8])], pivot: [9.0, 7.0, 4.0] },
    PartDef { name: "shieldArm", boxes: &[([0, 5, 2], [2, 14, 8])], pivot: [2.0, 13.5, 3.5] },
    PartDef { name: "swordArm", boxes: &[([12, 3, 1], [14, 19, 8])], pivot: [12.0, 13.5, 3.5] },
    PartDef { name: "head", boxes: &[([0, 15, 0], [14, 22, 8]), ([4, 10, 6], [10, 14, 8])], pivot: [7.0, 14.5, 3.5] },
];

// ------------------------------------------------------- Ragnar, in detail

/// Voxel materials, carried in the top byte of a voxel's color. The shader
/// gives each its own surface: see `voxel_surface` in shaders.wgsl.
pub mod mat {
    pub const METAL: u32 = 1 << 24;
    pub const FUR: u32 = 2 << 24;
    pub const CLOTH: u32 = 3 << 24;
    pub const LEATHER: u32 = 4 << 24;
    pub const SKIN: u32 = 5 << 24;
    pub const IVORY: u32 = 6 << 24;
    pub const WOOD: u32 = 7 << 24;
    pub const GOLD: u32 = 8 << 24;
}

mod hd {
    use super::mat;
    pub const SKIN: u32 = 0xf0c49c | mat::SKIN;
    pub const SKIN_D: u32 = 0xd49e76 | mat::SKIN;
    pub const BLUSH: u32 = 0xe8a486 | mat::SKIN;
    pub const EYE_W: u32 = 0xf4f0e6;
    pub const IRIS: u32 = 0x2f5c9c;
    pub const PUPIL: u32 = 0x121a2c;
    pub const BEARD: u32 = 0xc4581c | mat::FUR;
    pub const BEARD_L: u32 = 0xe2823a | mat::FUR;
    pub const BEARD_D: u32 = 0x93400f | mat::FUR;
    pub const HAIR: u32 = 0x8a3c10 | mat::FUR;
    pub const HELM: u32 = 0x8d96a3 | mat::METAL;
    pub const HELM_D: u32 = 0x6a727e | mat::METAL;
    pub const HELM_L: u32 = 0xaab2bc | mat::METAL;
    pub const RIVET: u32 = 0xe4e9ee | mat::METAL;
    pub const BRONZE: u32 = 0xc8903c | mat::GOLD;
    pub const BRONZE_D: u32 = 0x9a6a28 | mat::GOLD;
    pub const GOLD: u32 = 0xf2c53d | mat::GOLD;
    pub const HORN: u32 = 0xf2e8d0 | mat::IVORY;
    pub const HORN_M: u32 = 0xdccca6 | mat::IVORY;
    pub const HORN_TIP: u32 = 0xb4a176 | mat::IVORY;
    pub const TUNIC: u32 = 0x2e5f96 | mat::CLOTH;
    pub const TUNIC_D: u32 = 0x244d7c | mat::CLOTH;
    pub const TUNIC_L: u32 = 0x3a70aa | mat::CLOTH;
    pub const TRIM: u32 = 0xd0a848 | mat::CLOTH;
    pub const TRIM_D: u32 = 0x8a2a24 | mat::CLOTH;
    pub const CLOAK: u32 = 0x7d2a24 | mat::CLOTH;
    pub const CLOAK_D: u32 = 0x5e1e1a | mat::CLOTH;
    pub const TROUSER: u32 = 0x55593f | mat::CLOTH;
    pub const TROUSER_D: u32 = 0x464a33 | mat::CLOTH;
    pub const FUR: u32 = 0x8f6c49 | mat::FUR;
    pub const FUR_L: u32 = 0xc2a27c | mat::FUR;
    pub const FUR_D: u32 = 0x654a30 | mat::FUR;
    pub const BELT: u32 = 0x5c3a1c | mat::LEATHER;
    pub const LEATHER: u32 = 0x7a4a24 | mat::LEATHER;
    pub const LEATHER_D: u32 = 0x55331a | mat::LEATHER;
    pub const STITCH: u32 = 0xd8bc8e | mat::LEATHER;
    pub const WRAP: u32 = 0xb89a68 | mat::LEATHER;
    pub const BOOT: u32 = 0x4e321a | mat::LEATHER;
    pub const BOOT_D: u32 = 0x33200f | mat::LEATHER;
    pub const WOOD: u32 = 0x9a6a3a | mat::WOOD;
    pub const WOOD_D: u32 = 0x7a522a | mat::WOOD;
    pub const HAFT: u32 = 0x6b3f1d | mat::WOOD;
    pub const DARK_HAFT: u32 = 0x3e2412 | mat::WOOD;
    pub const IRON: u32 = 0x5a616b | mat::METAL;
    pub const STEEL: u32 = 0xc8d0da | mat::METAL;
    pub const STEEL_D: u32 = 0x8a929e | mat::METAL;
    pub const EDGE: u32 = 0xf2f5f8 | mat::METAL;
    pub const PLATE: u32 = 0xa8b4c4 | mat::METAL;
    pub const WRAP_RED: u32 = 0x9a1c22 | mat::LEATHER;
    pub const GILT: u32 = 0xffd84a | mat::GOLD;
    pub const PAINT_RED: u32 = 0xb8242a | mat::WOOD;
    pub const PAINT_CREAM: u32 = 0xe9dcbc | mat::WOOD;
}

/// A ball of voxels.
fn blob(s: &mut Sculpt, c: (f32, f32, f32), r: f32, color: u32) {
    s.ellipsoid(c, (r, r, r), 0, color);
}

/**
 * Ragnar at twice the resolution of `ragnar_geared` (the same size in
 * the world): a riveted spangenhelm with ivory horns, a braided fur beard, a
 * fur mantle, a woven tunic with embroidered trim, a stitched belt with a
 * gold buckle and a pouch, laced bracers, cross-gartered trousers, fur-cuffed
 * boots. 30 wide, 46 tall, 18 deep; voxels carry materials.
 */
pub fn ragnar_hd(axe_level: u8, shield: Option<ShieldKind>) -> Collected {
    use hd::*;
    let mut s = Sculpt::new(30, 46, 18);

    // ---- legs: boots with fur cuffs, cross-gartered trousers
    for (x0, x1) in [(8, 13), (16, 21)] {
        s.cuboid(x0, 0, 6, x1, 3, 13, BOOT);
        s.cuboid(x0 + 1, 0, 14, x1 - 1, 2, 14, BOOT);
        s.cuboid(x0, 0, 6, x1, 0, 13, BOOT_D);
        s.cuboid(x0 + 1, 0, 14, x1 - 1, 0, 14, BOOT_D);
        for x in (x0..=x1).step_by(2) {
            s.set(x, 3, 14, STITCH);
        }
        s.cuboid(x0, 7, 7, x1, 13, 12, TROUSER);
        s.cuboid(x0 - 1, 4, 5, x1 + 1, 6, 14, FUR);
        for (x, z) in [(x0 - 1, 5), (x1 + 1, 5), (x0 - 1, 14), (x1 + 1, 14)] {
            for y in 4..=6 {
                s.clear(x, y, z);
            }
        }
        for x in x0 - 1..=x1 + 1 {
            for z in 5..=14 {
                let rim = x == x0 - 1 || x == x1 + 1 || z == 5 || z == 14;
                if rim && s.get(x, 4, z).is_some() && noise(x, 3, z) > 0.55 {
                    s.set(x, 3, z, FUR_D);
                }
                if rim && s.get(x, 6, z).is_some() && noise(x, 7, z) > 0.6 {
                    s.set(x, 7, z, FUR_L);
                }
            }
        }
        s.paint(|x, y, z, c| {
            let edge = x == x0 || x == x1 || z == 7 || z == 12;
            if c == TROUSER && edge && (8..=12).contains(&y) && ((x + z + y) % 6 == 0 || (x + z - y).rem_euclid(6) == 0) {
                Some(WRAP)
            } else if c == TROUSER && noise(x, y, z) < 0.08 {
                Some(TROUSER_D)
            } else {
                None
            }
        });
    }

    // ---- tunic, embroidered hem, red cloak down the back
    s.cuboid(7, 14, 5, 22, 27, 11, TUNIC);
    s.cuboid(6, 14, 4, 23, 16, 12, TUNIC);
    for y in 17..=27 {
        for (x, z) in [(7, 5), (22, 5), (7, 11), (22, 11)] {
            s.clear(x, y, z);
        }
    }
    s.paint(|x, y, z, c| {
        if c != TUNIC {
            return None;
        }
        if y == 14 {
            return Some(if (x + z) % 4 == 0 { TRIM_D } else { TRIM });
        }
        if y <= 16 && (x % 3 == 0) && z >= 12 {
            return Some(TUNIC_D); // skirt folds
        }
        let n = noise(x, y, z);
        Some(if n < 0.08 { TUNIC_D } else if n > 0.93 { TUNIC_L } else { TUNIC })
    });
    s.cuboid(7, 14, 3, 22, 27, 4, CLOAK);
    s.paint(|x, y, z, c| if c == CLOAK && (x % 4 == 0 || (y == 14 && noise(x, y, z) > 0.5)) { Some(CLOAK_D) } else { None });

    // ---- belt: stitched leather, a gold buckle, a pouch at the right hip
    s.cuboid(6, 15, 4, 23, 17, 12, BELT);
    s.paint(|x, y, z, c| {
        let face = z == 12 || x == 6 || x == 23 || z == 4;
        if c == BELT && face && (y == 15 || y == 17) && (x + z) % 2 == 0 {
            Some(STITCH)
        } else {
            None
        }
    });
    s.cuboid(13, 14, 13, 16, 16, 13, GOLD);
    s.set(14, 15, 13, BELT);
    s.set(15, 15, 13, BRONZE_D);
    s.cuboid(18, 14, 13, 22, 16, 14, LEATHER);
    s.cuboid(18, 15, 15, 22, 16, 15, LEATHER_D);
    for x in [18, 20, 22] {
        s.set(x, 15, 15, STITCH);
    }
    s.set(20, 16, 15, GOLD);

    // ---- the shield on his back
    if let Some(kind) = shield {
        back_shield_hd(&mut s, kind);
    }

    // ---- arms: sleeves, laced bracers, hands
    for (x0, x1, outer) in [(2, 5, 2), (24, 27, 27)] {
        s.cuboid(x0, 22, 6, x1, 27, 10, TUNIC);
        s.cuboid(x0, 17, 6, x1, 21, 10, LEATHER);
        s.cuboid(x0, 17, 6, x1, 17, 10, LEATHER_D);
        s.cuboid(x0, 21, 6, x1, 21, 10, LEATHER_D);
        for y in 18..=20 {
            s.set(outer, y, 7 + (y % 2) * 2, STITCH);
        }
        s.cuboid(x0, 13, 6, x1, 16, 10, SKIN);
        for x in x0..=x1 {
            s.set(x, 13, 10, SKIN_D);
        }
    }
    hand_axe_hd(&mut s, axe_level);

    // ---- fur mantle over the shoulders, with tufts hanging from its edge
    s.ellipsoid((14.5, 28.5, 7.5), (11.8, 3.2, 6.3), 26, FUR);
    for y in 30..=32 {
        for z in 0..18 {
            for x in 10..=19 {
                s.clear(x, y, z);
            }
        }
    }
    for x in 2..=27 {
        for z in 1..=14 {
            if s.get(x, 26, z) == Some(FUR) && noise(x, 25, z) > 0.5 {
                s.set(x, 25, z, FUR);
                if noise(x, 24, z) > 0.72 {
                    s.set(x, 24, z, FUR_D);
                }
            }
        }
    }
    s.paint(|x, y, z, c| {
        if c != FUR || y < 24 {
            return None;
        }
        let n = noise(x, y / 2, z);
        Some(if (x * 7 + z * 3) % 5 == 0 { FUR_L } else if n < 0.28 { FUR_D } else if n > 0.72 { FUR_L } else { FUR })
    });

    // ---- neck and head
    s.cuboid(12, 28, 7, 17, 29, 10, SKIN);
    s.cuboid(10, 30, 4, 19, 39, 11, SKIN);
    for y in 30..=39 {
        for (x, z) in [(10, 4), (19, 4), (10, 11), (19, 11)] {
            s.clear(x, y, z);
        }
    }
    s.cuboid(10, 30, 4, 19, 38, 5, HAIR);
    for x in [10, 19] {
        s.cuboid(x, 31, 5, x, 38, 7, HAIR);
        s.cuboid(x, 33, 8, x, 34, 9, SKIN_D); // ears
    }
    // Eyes: white, iris toward the middle, a pupil; bushy brows above.
    for (w, i) in [(11, 12), (18, 17)] {
        s.set(w, 34, 11, EYE_W);
        s.set(w, 35, 11, EYE_W);
        s.set(i, 35, 11, IRIS);
        s.set(i, 34, 11, PUPIL);
    }
    for x in [10, 11, 12, 13, 16, 17, 18, 19] {
        s.set(x, 36, 11, BEARD_D);
        if x != 13 && x != 16 {
            s.set(x, 36, 12, BEARD_D);
        }
    }
    s.set(11, 33, 11, BLUSH);
    s.set(18, 33, 11, BLUSH);
    // Nose.
    s.cuboid(14, 32, 12, 15, 35, 12, SKIN);
    s.cuboid(14, 32, 13, 15, 33, 13, SKIN_D);

    // ---- the beard: full and long, a drooping mustache, two gold-ringed braids
    s.ellipsoid((14.5, 27.0, 12.0), (5.8, 6.4, 2.7), 20, BEARD);
    s.paint(|x, y, z, c| if c == BEARD && (y > 32 || (y == 32 && !(12..=17).contains(&x))) && z <= 11 { Some(SKIN) } else { None });
    for y in 33..40 {
        for z in 0..18 {
            for x in 0..30 {
                if s.get(x, y, z) == Some(BEARD) {
                    s.clear(x, y, z);
                }
            }
        }
    }
    s.cuboid(10, 32, 12, 19, 32, 13, BEARD_L);
    s.cuboid(11, 33, 12, 13, 33, 12, BEARD_L);
    s.cuboid(16, 33, 12, 18, 33, 12, BEARD_L);
    for x in [9, 20] {
        s.cuboid(x, 29, 12, x, 32, 13, BEARD_L);
    }
    for bx in [12, 16] {
        for y in 17..=22 {
            s.cuboid(bx, y, 13, bx + 1, y, 13, if y % 2 == 0 { BEARD } else { BEARD_D });
        }
        for y in [18, 21] {
            s.cuboid(bx, y, 13, bx + 1, y, 14, GOLD);
        }
        s.cuboid(bx, 17, 13, bx + 1, 17, 13, BEARD_L);
    }
    s.paint(|x, y, z, c| {
        if c != BEARD {
            return None;
        }
        let n = noise(x, y / 3, z);
        Some(if n < 0.3 { BEARD_D } else if n > 0.7 { BEARD_L } else { BEARD })
    });
    // A braid down the back.
    for y in 21..=29 {
        s.cuboid(14, y, 3, 15, y, 3, if y % 2 == 0 { HAIR } else { BEARD_D });
    }
    s.cuboid(14, 23, 2, 15, 23, 3, GOLD);

    // ---- helmet: a hammered iron spangenhelm, bronze band and strips, rivets
    s.ellipsoid((14.5, 37.5, 7.5), (5.9, 7.0, 4.9), 37, HELM);
    s.paint(|x, y, z, c| {
        if c != HELM {
            return None;
        }
        if y <= 38 {
            return Some(if y == 38 && (x + z) % 3 == 0 { RIVET } else { BRONZE });
        }
        let strip = (14..=15).contains(&x) || (7..=8).contains(&z);
        if strip {
            return Some(if y % 3 == 0 { RIVET } else { BRONZE_D });
        }
        let n = noise(x, y, z);
        Some(if n < 0.1 { HELM_D } else if n > 0.92 { HELM_L } else { HELM })
    });
    s.cuboid(14, 35, 12, 15, 38, 12, HELM);
    s.set(14, 37, 12, RIVET);

    // ---- horns sweeping up and out from bronze collars
    for side in [-1.0f32, 1.0] {
        let steps = 22;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = 14.5 + side * (5.6 + 7.2 * t - 0.6 * t * t);
            let y = 39.2 + 1.6 * t + 4.2 * t * t;
            let z = 7.5 - 1.2 * t;
            let r = 1.75 * (1.0 - t) + 0.45;
            let ring = ((t * 7.0).fract() < 0.18) && t > 0.2 && t < 0.85;
            let color = if t < 0.1 {
                BRONZE
            } else if t > 0.82 {
                HORN_TIP
            } else if ring || t > 0.6 {
                HORN_M
            } else {
                HORN
            };
            blob(&mut s, (x, y, z), r, color);
        }
    }
    s.finish(0.035)
}

/// Ragnar's axe at each level, in his right hand (x 26..27, blade forward).
fn hand_axe_hd(s: &mut Sculpt, level: u8) {
    use hd::*;
    let blade = |s: &mut Sculpt, profile: &[(i32, i32, i32)], body: u32, edge: u32| {
        let last = profile.len() - 1;
        for (i, &(z, y0, y1)) in profile.iter().enumerate() {
            for y in y0..=y1 {
                for x in 26..=27 {
                    s.set(x, y, z, if i == last { edge } else { body });
                }
            }
        }
    };
    match level {
        1 => {
            for y in 8..=36 {
                s.cuboid(26, y, 8, 27, y, 9, if (12..=19).contains(&y) && y % 2 == 0 { LEATHER_D } else { HAFT });
            }
            blade(s, &[(10, 31, 35), (11, 30, 35), (12, 29, 36), (13, 28, 36), (14, 27, 36), (15, 26, 36), (16, 26, 36)], STEEL, EDGE);
            s.cuboid(25, 32, 7, 28, 35, 10, STEEL_D);
            s.set(25, 34, 10, RIVET);
            s.set(28, 34, 10, RIVET);
        }
        2 => {
            for y in 7..=36 {
                s.cuboid(26, y, 8, 27, y, 9, if (18..=26).contains(&y) && y % 2 == 0 { WRAP_RED } else { HAFT });
            }
            blade(s, &[(10, 29, 36), (11, 28, 36), (12, 27, 37), (13, 26, 37), (14, 25, 37), (15, 24, 37), (16, 24, 37)], STEEL_D, EDGE);
            s.cuboid(25, 31, 7, 28, 35, 10, IRON);
            for z in 3..=6 {
                s.cuboid(26, 32, z, 27, 33, z, STEEL_D);
            }
            s.cuboid(26, 37, 8, 27, 37, 9, STEEL_D);
        }
        _ => {
            for y in 6..=36 {
                s.cuboid(26, y, 8, 27, y, 9, if y % 6 == 0 { GILT } else { DARK_HAFT });
            }
            blade(s, &[(10, 28, 36), (11, 27, 36), (12, 26, 37), (13, 25, 37), (14, 24, 37), (15, 23, 37), (16, 23, 37)], STEEL, GILT);
            blade(s, &[(7, 28, 36), (6, 27, 36), (5, 26, 37), (4, 25, 37), (3, 24, 37), (2, 23, 37), (1, 23, 37)], STEEL, GILT);
            s.cuboid(25, 30, 7, 28, 35, 10, GILT);
            s.cuboid(26, 37, 8, 27, 37, 9, GILT);
        }
    }
    // The fist around the haft.
    s.cuboid(24, 13, 6, 27, 16, 10, SKIN);
    s.cuboid(24, 13, 10, 27, 13, 10, SKIN_D);
}

/// The shield on his back: its face at z 0..1, rim and boss standing out.
fn back_shield_hd(s: &mut Sculpt, kind: ShieldKind) {
    use hd::*;
    for y in 8..=34 {
        for x in 3..=26 {
            let (dx, dy) = (x as f32 - 14.5, y as f32 - 21.0);
            let r = (dx * dx + dy * dy).sqrt();
            let (face, raised) = match kind {
                ShieldKind::Round | ShieldKind::IronRound => {
                    if r > 9.0 {
                        continue;
                    }
                    let rim = r > 8.0;
                    let boss = r < 2.3;
                    let rivet = (r - 8.5).abs() < 0.5 && (dx.atan2(dy) * 12.0 / std::f32::consts::TAU).fract().abs() < 0.12;
                    let face = if kind == ShieldKind::Round {
                        if rim || boss {
                            IRON
                        } else if x % 4 == 0 {
                            WOOD_D
                        } else if (x / 4) % 2 == 0 {
                            WOOD
                        } else {
                            PAINT_RED
                        }
                    } else if boss {
                        GOLD
                    } else if rim {
                        STEEL_D
                    } else if (r - 5.4).abs() < 0.45 && (x + y) % 2 == 0 {
                        RIVET
                    } else {
                        PLATE
                    };
                    (if rivet { RIVET } else { face }, rim || boss)
                }
                ShieldKind::Kite | ShieldKind::Dreadnought => {
                    let half = if y >= 22 { 9.0 } else { 9.0 * (y as f32 - 8.0) / 14.0 + 0.8 };
                    if dx.abs() > half || y > 33 {
                        continue;
                    }
                    let rim = dx.abs() > half - 1.2 || y == 33;
                    let face = if kind == ShieldKind::Kite {
                        if rim {
                            IRON
                        } else if (dx >= 0.0) != (dy >= 2.0) {
                            PAINT_RED
                        } else {
                            PAINT_CREAM
                        }
                    } else if rim {
                        GOLD
                    } else if (x + y) % 3 == 0 && dx.abs() > half - 2.6 {
                        RIVET
                    } else if dx.abs() < 0.8 || y == 24 {
                        STEEL_D
                    } else {
                        PLATE
                    };
                    (face, rim)
                }
            };
            s.set(x, y, 1, face);
            if raised {
                s.set(x, y, 0, face);
            }
        }
    }
}

/// The detailed Ragnar's joints, in its doubled grid.
pub const RAGNAR_HD_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[([6, 0, 4], [14, 13, 16])], pivot: [11.0, 14.0, 9.5] },
    PartDef { name: "rightLeg", boxes: &[([15, 0, 4], [23, 13, 16])], pivot: [18.5, 14.0, 9.5] },
    PartDef { name: "shieldArm", boxes: &[([0, 10, 4], [5, 29, 16])], pivot: [5.0, 27.0, 8.0] },
    PartDef { name: "swordArm", boxes: &[([24, 6, 0], [29, 37, 17])], pivot: [24.0, 27.0, 8.0] },
    PartDef { name: "head", boxes: &[([0, 30, 2], [29, 45, 17]), ([9, 17, 13], [20, 29, 17])], pivot: [14.5, 29.0, 7.5] },
];

// ------------------------------------------------- Mehmed and Miyamoto

/// A sculpt seen through an x offset, so a model can be drawn in its body's
/// own coordinates (the same as Ragnar's, shifted) while its weapon reaches
/// past the edge of those.
struct Offset<'a> {
    s: &'a mut Sculpt,
    dx: i32,
}

impl Offset<'_> {
    fn set(&mut self, x: i32, y: i32, z: i32, c: u32) {
        self.s.set(x + self.dx, y, z, c);
    }

    fn clear(&mut self, x: i32, y: i32, z: i32) {
        self.s.clear(x + self.dx, y, z);
    }

    #[allow(clippy::too_many_arguments)]
    fn cuboid(&mut self, x0: i32, y0: i32, z0: i32, x1: i32, y1: i32, z1: i32, c: u32) {
        self.s.cuboid(x0 + self.dx, y0, z0, x1 + self.dx, y1, z1, c);
    }

    fn ellipsoid(&mut self, c: (f32, f32, f32), r: (f32, f32, f32), ymin: i32, color: u32) {
        self.s.ellipsoid((c.0 + self.dx as f32, c.1, c.2), r, ymin, color);
    }

    fn capsule(&mut self, a: (f32, f32, f32), b: (f32, f32, f32), ra: f32, rb: f32, c: u32) {
        let d = self.dx as f32;
        self.s.capsule((a.0 + d, a.1, a.2), (b.0 + d, b.1, b.2), ra, rb, c);
    }

    fn paint(&mut self, mut f: impl FnMut(i32, i32, i32, u32) -> Option<u32>) {
        let dx = self.dx;
        self.s.paint(|x, y, z, c| f(x - dx, y, z, c));
    }
}

/// All the voxel columns whose centres lie within `half` of `c`.
fn span(c: f32, half: f32) -> (i32, i32) {
    ((c - half).ceil() as i32, (c + half).floor() as i32)
}

mod jan {
    use super::mat;
    pub const SKIN: u32 = 0xd9a47c | mat::SKIN;
    pub const SKIN_D: u32 = 0xb8825c | mat::SKIN;
    pub const EYE_W: u32 = 0xf2eee4;
    pub const IRIS: u32 = 0x4a2c16;
    pub const PUPIL: u32 = 0x140c08;
    pub const HAIR: u32 = 0x221812 | mat::FUR;
    pub const BEARD: u32 = 0x1e1510 | mat::FUR;
    pub const BEARD_L: u32 = 0x3a2a1e | mat::FUR;
    pub const BROW: u32 = 0x18110c | mat::FUR;
    pub const FELT: u32 = 0xf0ebe0 | mat::CLOTH;
    pub const FELT_D: u32 = 0xd6cfbf | mat::CLOTH;
    pub const GOLD: u32 = 0xf2c53d | mat::GOLD;
    pub const GOLD_D: u32 = 0xb88a2a | mat::GOLD;
    pub const GEM: u32 = 0x1fb8a4;
    pub const KAFTAN: u32 = 0x1d6a3a | mat::CLOTH;
    pub const KAFTAN_D: u32 = 0x145230 | mat::CLOTH;
    pub const THREAD: u32 = 0xae8c36 | mat::CLOTH;
    pub const ROBE: u32 = 0x8e1c26 | mat::CLOTH;
    pub const ROBE_D: u32 = 0x6e141c | mat::CLOTH;
    pub const SASH: u32 = 0xa01e2a | mat::CLOTH;
    pub const SASH_D: u32 = 0x7a1620 | mat::CLOTH;
    pub const BOOT: u32 = 0x8a2a1c | mat::LEATHER;
    pub const BOOT_D: u32 = 0x5e1c12 | mat::LEATHER;
    pub const SHAFT: u32 = 0x4e301a | mat::WOOD;
    pub const STEEL: u32 = 0xc8d0da | mat::METAL;
    pub const STEEL_D: u32 = 0x8a929e | mat::METAL;
    pub const EDGE: u32 = 0xf2f5f8 | mat::METAL;
    pub const TASSEL: u32 = 0xb81e26 | mat::CLOTH;
}

/**
 * Mehmed the janissary, the size of the detailed Ragnar: a tall white börk
 * with its flap hanging down his back and a gilt spoon-holder plate set with
 * a green stone, a black beard and a heavy moustache, a green kaftan worked
 * with gold to the ankles, frogged across the chest and open below a crimson
 * sash over a crimson robe, red boots with turned-up toes, and in his left
 * hand a halberd taller than he is. 48 wide (his body in the middle, as
 * Ragnar's is in his), 62 tall, 20 deep; voxels carry materials.
 */
pub fn mehmed() -> Collected {
    use jan::*;
    let mut base = Sculpt::new(48, 62, 20);
    let mut s = Offset { s: &mut base, dx: 4 };
    let cx = 19.5f32;

    // ---- boots: red leather, the toes turned up
    for (x0, x1) in [(13, 18), (21, 26)] {
        s.cuboid(x0, 0, 7, x1, 4, 14, BOOT);
        s.cuboid(x0 + 1, 0, 15, x1 - 1, 1, 15, BOOT);
        s.cuboid(x0 + 2, 2, 16, x1 - 2, 2, 16, BOOT_D);
        s.cuboid(x0, 0, 7, x1, 0, 15, BOOT_D);
    }

    // ---- the kaftan's skirt, flaring to the ankles, open down the front over a crimson robe
    for y in 4..=15 {
        let t = (15 - y) as f32 / 11.0;
        let (x0, x1) = span(cx, 8.6 + t * 2.6);
        let (z0, z1) = (5 - (t * 1.6).round() as i32, 14 + (t * 1.6).round() as i32);
        s.cuboid(x0, y, z0, x1, y, z1, KAFTAN);
        let (g0, g1) = span(cx, 1.0 + t * 2.2);
        for x in g0..=g1 {
            s.set(x, y, z1, if y % 3 == 0 { ROBE_D } else { ROBE });
            s.set(x, y, z1 - 1, ROBE);
        }
        s.set(g0 - 1, y, z1, GOLD);
        s.set(g1 + 1, y, z1, GOLD);
    }

    // ---- the crimson sash, a loose end hanging at the front
    {
        let (x0, x1) = span(cx, 9.6);
        s.cuboid(x0, 15, 4, x1, 17, 15, SASH);
        s.cuboid(x0, 16, 4, x1, 16, 15, SASH_D);
        s.cuboid(14, 9, 16, 16, 15, 16, SASH);
        s.cuboid(14, 9, 16, 16, 9, 16, GOLD_D);
    }

    // ---- the kaftan's body, frogged across the chest with gold
    {
        let (x0, x1) = span(cx, 8.6);
        s.cuboid(x0, 18, 5, x1, 28, 14, KAFTAN);
        let (x0, x1) = span(cx, 9.8);
        s.cuboid(x0, 25, 5, x1, 28, 14, KAFTAN);
        for y in [19, 21, 23, 25] {
            s.cuboid(16, y, 15, 23, y, 15, GOLD);
            s.set(15, y, 15, GOLD_D);
            s.set(24, y, 15, GOLD_D);
        }
        s.cuboid(19, 18, 15, 20, 26, 15, GOLD_D);
    }

    // ---- arms: wide sleeves with gold cuffs; the left hand grips the halberd, the right hangs in a fist
    for (x0, x1) in [(7, 10), (29, 32)] {
        s.cuboid(x0, 16, 6, x1, 27, 12, KAFTAN);
        s.cuboid(x0 - 1, 14, 5, x1 + 1, 16, 13, KAFTAN);
        s.cuboid(x0 - 1, 14, 5, x1 + 1, 14, 13, GOLD_D);
        s.cuboid(x0, 11, 7, x1, 13, 11, SKIN);
        s.cuboid(x0, 11, 11, x1, 11, 11, SKIN_D);
    }
    s.cuboid(3, 11, 7, 6, 13, 11, SKIN);

    // ---- the damask: a lattice of gold thread over the green, a gold flower in every lozenge
    s.paint(|x, y, z, c| {
        if c != KAFTAN {
            return None;
        }
        let u = if z <= 5 || z >= 14 { x } else { z + 40 };
        let (a, b) = ((u + y).rem_euclid(6), (u - y).rem_euclid(6));
        if a == 0 || b == 0 {
            Some(THREAD)
        } else if a == 3 && b == 3 {
            Some(GOLD)
        } else if noise(x, y, z) < 0.12 {
            Some(KAFTAN_D)
        } else {
            None
        }
    });

    // ---- collar and neck
    s.cuboid(15, 28, 6, 24, 29, 13, KAFTAN_D);
    s.cuboid(15, 29, 6, 24, 29, 13, GOLD_D);
    s.cuboid(17, 28, 8, 22, 30, 11, SKIN);

    // ---- head
    s.cuboid(15, 30, 5, 24, 39, 12, SKIN);
    for y in 30..=39 {
        for (x, z) in [(15, 5), (24, 5), (15, 12), (24, 12)] {
            s.clear(x, y, z);
        }
    }
    s.cuboid(15, 30, 5, 24, 38, 6, HAIR);
    for x in [15, 24] {
        s.cuboid(x, 31, 6, x, 38, 8, HAIR);
        s.cuboid(x, 33, 9, x, 34, 10, SKIN_D);
    }

    // ---- a full black beard, trimmed square, and a heavy moustache
    s.ellipsoid((cx, 28.4, 12.2), (5.4, 3.9, 2.6), 24, BEARD);
    s.paint(|_, y, z, c| (c == BEARD && y > 31 && z <= 12).then_some(SKIN));
    s.paint(|x, y, z, c| (c == BEARD && noise(x, y / 2, z) > 0.72).then_some(BEARD_L));
    s.cuboid(16, 32, 13, 23, 32, 13, BEARD_L);
    s.cuboid(15, 30, 13, 15, 32, 13, BEARD_L);
    s.cuboid(24, 30, 13, 24, 32, 13, BEARD_L);
    s.cuboid(18, 31, 13, 21, 31, 13, BEARD);

    // ---- eyes under heavy, knitted brows; a strong nose
    for (w, i) in [(16, 17), (23, 22)] {
        s.set(w, 34, 12, EYE_W);
        s.set(w, 35, 12, EYE_W);
        s.set(i, 35, 12, IRIS);
        s.set(i, 34, 12, PUPIL);
    }
    for x in [15, 16, 17, 22, 23, 24] {
        s.set(x, 36, 12, BROW);
        s.set(x, 36, 13, BROW);
    }
    for x in [18, 21] {
        s.set(x, 35, 13, BROW);
        s.set(x, 36, 13, BROW);
    }
    s.cuboid(19, 32, 13, 20, 35, 13, SKIN);
    s.cuboid(19, 32, 14, 20, 33, 14, SKIN_D);

    // ---- the börk: white felt on a gold band, tapering, its flap folded down his back
    for y in 38..=54 {
        let taper = if y < 41 { 0.0 } else { (y - 41) as f32 * 0.07 };
        let (x0, x1) = span(cx, 5.3 - taper);
        let (z0, z1) = ((4.6 + taper * 0.6).round() as i32, (12.4 - taper * 0.6).round() as i32);
        s.cuboid(x0, y, z0, x1, y, z1, if y <= 39 { GOLD } else { FELT });
    }
    s.paint(|x, y, z, c| (c == FELT && y > 41 && (x + z) % 4 == 0).then_some(FELT_D));
    {
        let (x0, x1) = span(cx, 4.4);
        s.cuboid(x0, 30, 4, x1, 53, 4, FELT);
        s.cuboid(x0, 52, 4, x1, 54, 5, FELT);
        s.paint(|x, y, z, c| (c == FELT && z == 4 && y < 52 && x % 3 == 0).then_some(FELT_D));
        s.cuboid(x0, 30, 4, x1, 30, 4, GOLD_D);
    }
    // The spoon-holder plate over the brow, set with a green stone, a plume socket above.
    {
        let (x0, x1) = span(cx, 2.6);
        s.cuboid(x0, 40, 13, x1, 47, 13, GOLD);
        s.cuboid(x0 + 1, 48, 13, x1 - 1, 48, 13, GOLD);
        s.cuboid(x0 + 1, 41, 14, x1 - 1, 41, 14, GOLD_D);
        s.cuboid(x0 + 1, 46, 14, x1 - 1, 46, 14, GOLD_D);
        s.cuboid(19, 43, 14, 20, 44, 14, GEM);
        s.cuboid(19, 49, 12, 20, 51, 13, GOLD_D);
    }

    // ---- the halberd: a shaft banded with gold, a red tassel, a crescent axe, a back spike, a spear point
    for y in 0..=56 {
        let band = matches!(y, 0 | 1 | 18 | 30 | 43 | 56);
        s.cuboid(1, y, 9, 2, y, 10, if band { GOLD } else { SHAFT });
    }
    s.cuboid(0, 39, 8, 3, 42, 11, TASSEL);
    s.cuboid(-1, 38, 9, 4, 38, 10, TASSEL);
    for y in 44..=56 {
        let t = (y - 44) as f32 / 12.0;
        let reach = 1.2 + 3.3 * (t * std::f32::consts::PI).sin();
        let out = (0.0 - reach).round() as i32;
        for x in out..=0 {
            let c = if x == out { EDGE } else { STEEL };
            s.set(x, y, 9, c);
            s.set(x, y, 10, c);
        }
    }
    s.cuboid(3, 49, 9, 4, 50, 10, STEEL_D);
    s.set(5, 49, 9, STEEL_D);
    s.set(5, 49, 10, STEEL_D);
    s.cuboid(1, 57, 9, 2, 59, 10, STEEL);
    s.cuboid(1, 60, 9, 2, 60, 10, EDGE);
    s.set(1, 61, 9, EDGE);
    drop(s);
    base.finish(0.035)
}

/// Mehmed's joints, in his 48-wide grid: the head with its hat and the top of his beard.
pub const MEHMED_RIG: &[PartDef] = &[PartDef { name: "head", boxes: &[([17, 30, 0], [30, 61, 19]), ([18, 26, 12], [29, 29, 15])], pivot: [23.5, 29.0, 9.0] }];

mod sam {
    use super::mat;
    pub const SKIN: u32 = 0xe2b48c | mat::SKIN;
    pub const SKIN_D: u32 = 0xa8785a | mat::SKIN;
    pub const EYE_W: u32 = 0xeeeae0;
    pub const PUPIL: u32 = 0x0e0a08;
    pub const LACQUER: u32 = 0xb8222a | mat::LEATHER;
    pub const LACQUER_D: u32 = 0x8a1820 | mat::LEATHER;
    pub const BLACK: u32 = 0x1c1a1e | mat::LEATHER;
    pub const IRON: u32 = 0x2c2a30 | mat::METAL;
    pub const GOLD: u32 = 0xf2c53d | mat::GOLD;
    pub const GOLD_D: u32 = 0xb88a2a | mat::GOLD;
    pub const LACING: u32 = 0x24367a | mat::CLOTH;
    pub const OBI: u32 = 0x2a4aa8 | mat::CLOTH;
    pub const OBI_D: u32 = 0x1e3a86 | mat::CLOTH;
    pub const HAKAMA: u32 = 0x2e2c36 | mat::CLOTH;
    pub const HAKAMA_D: u32 = 0x201e28 | mat::CLOTH;
    pub const KOTE: u32 = 0x26242e | mat::CLOTH;
    pub const TABI: u32 = 0xe8e2d4 | mat::CLOTH;
    pub const STRAW: u32 = 0xc8a868 | mat::WOOD;
    pub const MASK: u32 = 0xc0262c | mat::LEATHER;
    pub const TEETH: u32 = 0xf0ecdc | mat::IVORY;
    pub const WHISKER: u32 = 0x141010 | mat::FUR;
    pub const STEEL: u32 = 0xc8d0da | mat::METAL;
    pub const EDGE: u32 = 0xf6f9fc | mat::METAL;
    pub const WRAP: u32 = 0x18161a | mat::CLOTH;
    pub const SAME: u32 = 0xeae4d4 | mat::IVORY;
}

/**
 * Miyamoto the samurai, the size of the detailed Ragnar: a black iron kabuto
 * crowned with great gilt horns, its neck guard of red lacquered lames and
 * gilt turnbacks; a red oni mask with a snarl of white teeth and black
 * whiskers; a cuirass, shoulder guards and armoured skirt of red lacquered
 * lames edged in gold and laced in blue silk; a blue obi with a scabbard at
 * the hip; dark hakama, black shin guards, straw sandals over white tabi;
 * and a long katana raised in his right hand. 48 wide (his body in the
 * middle), 60 tall, 22 deep; voxels carry materials.
 */
pub fn miyamoto() -> Collected {
    use sam::*;
    let mut base = Sculpt::new(48, 60, 22);
    let mut s = Offset { s: &mut base, dx: 4 };
    let cx = 19.5f32;

    // ---- straw sandals over split-toed tabi
    for (x0, x1) in [(13, 18), (21, 26)] {
        s.cuboid(x0, 0, 8, x1, 0, 17, STRAW);
        s.cuboid(x0, 1, 8, x1, 2, 16, TABI);
        s.cuboid(x0 + 2, 1, 16, x0 + 2, 2, 17, BLACK);
    }
    // ---- black lacquered shin guards, splinted with gold, over the hakama
    for (x0, x1) in [(13, 18), (21, 26)] {
        s.cuboid(x0, 3, 8, x1, 9, 15, HAKAMA);
        s.cuboid(x0, 3, 15, x1, 9, 16, BLACK);
        for x in (x0..=x1).step_by(2) {
            s.cuboid(x, 3, 16, x, 8, 16, GOLD_D);
        }
        s.cuboid(x0, 9, 15, x1, 9, 16, GOLD);
    }
    // ---- hakama: wide pleated trousers
    for y in 8..=14 {
        let t = (y - 8) as f32 / 6.0;
        let (x0, x1) = span(cx, 7.4 + t * 1.2);
        s.cuboid(x0, y, 6, x1, y, 16, HAKAMA);
    }
    s.paint(|x, y, _, c| (c == HAKAMA && y > 2 && x % 3 == 0).then_some(HAKAMA_D));

    // ---- the kusazuri: an armoured skirt of lacquered lames, flaring from the waist
    for y in 10..=17 {
        let t = (17 - y) as f32 / 7.0;
        let (x0, x1) = span(cx, 9.0 + t * 2.0);
        let (z0, z1) = (5 - (t * 1.6).round() as i32, 16 + (t * 1.6).round() as i32);
        s.cuboid(x0, y, z0, x1, y, z1, if (y - 10) % 3 == 0 { GOLD_D } else { LACQUER });
    }
    // ---- the obi, and a black scabbard slung at the left hip
    {
        let (x0, x1) = span(cx, 9.2);
        s.cuboid(x0, 17, 5, x1, 18, 16, OBI);
        s.cuboid(x0, 18, 5, x1, 18, 16, OBI_D);
        s.capsule((12.5, 18.0, 17.0), (8.0, 11.0, 3.0), 0.8, 0.7, BLACK);
        s.capsule((12.5, 18.0, 17.0), (12.2, 17.5, 16.2), 1.0, 1.0, GOLD);
    }
    // ---- the dō: a cuirass of red lames edged with gold; black breastplate top, a gilt crest
    {
        let (x0, x1) = span(cx, 8.6);
        for y in 19..=27 {
            s.cuboid(x0, y, 6, x1, y, 15, if (y - 19) % 3 == 0 { GOLD_D } else { LACQUER });
        }
        s.cuboid(x0, 28, 6, x1, 29, 15, BLACK);
        s.cuboid(x0, 29, 6, x1, 29, 15, GOLD);
        s.ellipsoid((cx, 23.5, 16.0), (1.6, 1.6, 0.7), 0, GOLD);
        s.set(19, 23, 17, GOLD_D);
        s.set(20, 24, 17, GOLD_D);
    }
    // ---- sode: great shoulder guards of lames, spreading as they hang
    for side in [-1i32, 1] {
        for y in 20..=29 {
            let flare = ((29 - y) as f32 * 0.2).round() as i32;
            let (xa, xb) = if side < 0 { (6 - flare, 10 - flare) } else { (29 + flare, 33 + flare) };
            let c = if y == 29 { BLACK } else if (29 - y) % 3 == 0 { GOLD_D } else { LACQUER };
            s.cuboid(xa, y, 7, xb, y, 14, c);
        }
    }
    // Blue silk lacing through every red lame; gaps between the skirt's panels.
    s.paint(|x, y, z, c| {
        if c != LACQUER {
            return None;
        }
        if (10..=17).contains(&y) && (x == 15 || x == 24) {
            return Some(LACQUER_D);
        }
        // Laced in pairs of blue cords down each lame.
        ((x + z).rem_euclid(5) == 0 && y % 3 == 1).then_some(LACING)
    });

    // ---- arms: sleeves of dark silk sewn with gilt plates, gauntleted fists
    s.cuboid(7, 14, 7, 10, 25, 12, KOTE);
    s.cuboid(7, 11, 7, 10, 13, 12, BLACK);
    s.cuboid(7, 11, 12, 10, 11, 12, GOLD_D);
    s.capsule((30.5, 26.0, 10.5), (31.0, 20.5, 12.5), 2.2, 2.0, KOTE);
    s.capsule((31.0, 20.5, 12.5), (30.5, 18.8, 16.0), 2.0, 1.8, KOTE);
    s.paint(|x, y, z, c| (c == KOTE && (x + y * 2 + z) % 5 == 0).then_some(GOLD_D));
    s.ellipsoid((30.5, 18.3, 16.6), (1.9, 1.9, 1.9), 0, BLACK);

    // ---- the katana, raised: a silk-wrapped hilt through his fist, a gilt guard, a long blade with a bright edge
    {
        let fist = (30.5f32, 18.3f32, 16.6f32);
        let (dx, dy, dz) = (0.27f32, 0.96f32, 0.08f32);
        let n = (dx * dx + dy * dy + dz * dz).sqrt();
        let d = (dx / n, dy / n, dz / n);
        let at = |k: f32| (fist.0 + d.0 * k, fist.1 + d.1 * k, fist.2 + d.2 * k);
        for i in -7i32..=2 {
            let p = at(i as f32);
            let c = if i == -7 { GOLD } else if i.rem_euclid(2) == 0 { SAME } else { WRAP };
            s.capsule(p, p, 0.8, 0.8, c);
        }
        s.ellipsoid(at(3.0), (1.9, 0.6, 1.9), 0, GOLD);
        s.capsule(at(3.5), at(4.5), 0.7, 0.7, GOLD_D);
        for i in 0..=31 {
            let t = i as f32 / 31.0;
            let bend = t * t * 1.8;
            let p = at(4.5 + i as f32);
            let (x, y, z) = ((p.0 - bend * 0.25).round() as i32, p.1.round() as i32, (p.2 - bend).round() as i32);
            s.set(x, y, z, STEEL);
            if i < 29 {
                s.set(x + 1, y, z, EDGE);
            } else {
                s.set(x, y + 1, z, EDGE);
            }
        }
    }

    // ---- throat guard, the head, and the red oni mask with its snarl
    s.cuboid(15, 27, 7, 24, 29, 13, BLACK);
    s.cuboid(16, 27, 14, 23, 29, 14, LACQUER_D);
    s.cuboid(16, 27, 14, 23, 27, 14, GOLD_D);
    s.cuboid(15, 30, 6, 24, 39, 13, SKIN);
    s.cuboid(15, 30, 11, 24, 34, 14, MASK);
    s.cuboid(19, 33, 15, 20, 35, 15, MASK);
    s.cuboid(19, 33, 16, 20, 33, 16, MASK);
    s.set(18, 33, 15, MASK);
    s.set(21, 33, 15, MASK);
    for x in 17..=22 {
        s.set(x, 32, 15, if x % 2 == 0 { TEETH } else { PUPIL });
        s.set(x, 31, 15, if x % 2 == 1 { TEETH } else { PUPIL });
    }
    s.set(16, 31, 15, TEETH);
    s.set(23, 31, 15, TEETH);
    s.cuboid(16, 33, 15, 17, 33, 15, WHISKER);
    s.cuboid(22, 33, 15, 23, 33, 15, WHISKER);
    s.set(15, 34, 15, WHISKER);
    s.set(24, 34, 15, WHISKER);
    s.cuboid(19, 29, 15, 20, 30, 15, WHISKER);
    // Fierce eyes in the shadow of the visor.
    s.cuboid(15, 35, 13, 24, 36, 13, SKIN_D);
    for (w, p) in [(16, 17), (23, 22)] {
        s.set(w, 35, 13, EYE_W);
        s.set(p, 35, 13, PUPIL);
    }
    s.cuboid(16, 36, 14, 18, 36, 14, MASK);
    s.cuboid(21, 36, 14, 23, 36, 14, MASK);

    // ---- the kabuto: a black iron bowl studded with gilt rivets, a visor, flaring neck-guard lames, gilt turnbacks
    s.ellipsoid((cx, 37.5, 10.0), (6.4, 6.0, 5.6), 37, IRON);
    s.paint(|x, y, _, c| (c == IRON && y > 38 && x % 3 == 0 && y % 2 == 0).then_some(GOLD_D));
    s.cuboid(14, 37, 12, 25, 37, 16, BLACK);
    s.cuboid(14, 37, 16, 25, 37, 16, GOLD);
    for (i, y) in [36, 34, 32, 30].into_iter().enumerate() {
        let out = i as f32 * 1.1;
        let (x0, x1) = span(cx, 6.4 + out);
        let z0 = (4.0 - out).round() as i32;
        for yy in y - 1..=y {
            let c = if yy == y - 1 { GOLD_D } else { LACQUER };
            for z in z0..=12 {
                s.set(x0, yy, z, c);
                s.set(x1, yy, z, c);
            }
            for x in x0..=x1 {
                s.set(x, yy, z0, c);
            }
        }
    }
    for (xa, xb) in [(12, 14), (25, 27)] {
        s.cuboid(xa, 34, 12, xb, 37, 14, LACQUER);
        s.cuboid(xa, 37, 12, xb, 37, 14, GOLD);
        s.cuboid(xa, 34, 14, xb, 36, 14, GOLD_D);
    }
    // ---- the crest: a gilt disc on the brow, and great gilt horns rising from it in a wide V
    s.ellipsoid((cx, 40.0, 16.2), (2.0, 2.0, 0.8), 0, GOLD);
    for side in [-1.0f32, 1.0] {
        let steps = 28;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = cx + side * (1.8 + 7.0 * t + 1.5 * t * t);
            let y = 40.5 + 16.0 * t - 2.0 * t * t * t;
            let r = 1.1 - 0.55 * t;
            s.capsule((x, y, 16.2), (x, y, 16.2), r, r, if t > 0.88 { GOLD_D } else { GOLD });
        }
    }
    drop(s);
    base.finish(0.035)
}

/// Miyamoto's joints, in his 48-wide grid: the head with its helmet and crest.
pub const MIYAMOTO_RIG: &[PartDef] = &[PartDef { name: "head", boxes: &[([13, 30, 0], [36, 59, 21])], pivot: [23.5, 30.0, 10.0] }];

// --------------------------------------------------------------------- draugr

/// The draugr: an undead viking, 13 wide, 7 deep, 21 tall.
pub fn draugr() -> Collected {
    const BONE: u32 = 0xe2d6bc;
    const BONE_S: u32 = 0xb4a286;
    const CAVITY: u32 = 0x1f1a16;
    const GLOW: u32 = 0x6ff0ff;
    const RUST: u32 = 0x7a5a42;
    const RUST_L: u32 = 0x9c7052;
    const RAG: u32 = 0x3b3744;
    const HORN: u32 = 0xcdbf98;
    const BLADE: u32 = 0x8d8a86;
    let mut s = Sculpt::new(13, 21, 7);
    // Feet and leg bones with knobbly knees.
    for x in [4, 8] {
        s.cuboid(x - 1, 0, 2, x, 0, 4, BONE);
        for y in 1..=6 {
            s.set(x, y, 3, BONE);
        }
        s.cuboid(x - 1, 3, 3, x, 3, 3, BONE_S);
    }
    // Pelvis, spine, and a ribcage of solid bands over a dark hollow.
    s.cuboid(3, 7, 2, 9, 7, 4, BONE);
    for y in 8..=14 {
        s.set(6, y, 2, BONE_S);
    }
    s.cuboid(4, 9, 2, 8, 13, 4, CAVITY);
    for y in [9, 11, 13] {
        for x in 3..=9 {
            s.set(x, y, 1, BONE);
            s.set(x, y, 5, BONE);
        }
        for z in 1..=5 {
            s.set(3, y, z, BONE);
            s.set(9, y, z, BONE);
        }
    }
    for y in 9..=13 {
        s.set(6, y, 5, BONE_S); // sternum
    }
    // Scraps of rusted mail still clinging on.
    s.paint(|x, y, z, c| if c == BONE && y >= 9 && y <= 13 && (x * 7 + y * 3 + z) % 5 == 0 { Some(RUST) } else { None });
    // A tattered cloak on the back, full of holes and ragged at the hem.
    for y in 5..=13 {
        for x in 3..=9 {
            let hole = noise(x, y, 99) < 0.22 || (y < 7 && (x + y) % 2 == 0);
            if !hole {
                s.set(x, y, 0, RAG);
            }
        }
    }
    // Shoulders, arms, clawed hands.
    s.cuboid(2, 14, 2, 10, 14, 4, BONE);
    for x in [1, 11] {
        for y in 7..=13 {
            s.set(x, y, 3, BONE);
        }
        s.cuboid(x, 6, 3, x, 6, 4, BONE_S);
    }
    // A notched, rusted blade in the right hand.
    for y in 1..=5 {
        s.set(11, y, 4, BLADE);
    }
    s.set(11, 2, 4, RUST);
    s.cuboid(11, 6, 3, 11, 6, 5, RUST_L);
    // Neck, skull with glowing eyes and a gap-toothed jaw.
    s.set(6, 15, 3, BONE_S);
    s.cuboid(4, 16, 1, 8, 19, 5, BONE);
    s.set(5, 18, 5, GLOW);
    s.set(7, 18, 5, GLOW);
    s.set(6, 17, 5, CAVITY);
    for x in 4..=8 {
        s.set(x, 16, 5, if x % 2 == 0 { BONE } else { CAVITY });
    }
    // Rusted helmet with one broken horn.
    s.ellipsoid((6.0, 19.0, 3.0), (2.8, 2.2, 2.7), 19, RUST);
    s.paint(|x, y, z, c| if c == RUST && y == 20 && (x + z) % 2 == 0 { Some(RUST_L) } else { None });
    for (x, y) in [(3, 19), (2, 20)] {
        s.set(x, y, 3, HORN);
    }
    s.set(9, 19, 3, HORN);
    s.finish(0.06)
}

pub const DRAUGR_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[([3, 0, 1], [5, 6, 6])], pivot: [4.0, 7.0, 3.0] },
    PartDef { name: "rightLeg", boxes: &[([7, 0, 1], [9, 6, 6])], pivot: [8.0, 7.0, 3.0] },
    PartDef { name: "leftArm", boxes: &[([0, 0, 1], [2, 13, 6])], pivot: [1.5, 14.0, 3.0] },
    PartDef { name: "rightArm", boxes: &[([10, 0, 1], [12, 13, 6])], pivot: [10.5, 14.0, 3.0] },
    PartDef { name: "head", boxes: &[([1, 15, 0], [11, 20, 6])], pivot: [6.0, 15.0, 3.0] },
];

// --------------------------------------------------------------------- ravens

/// A raven with its wings spread: 19 wide, 11 deep, 7 tall. `night` is the
/// larger, red-eyed night raven.
pub fn raven(night: bool) -> Collected {
    let (black, sheen, under, eye) =
        if night { (0x2a1838, 0x5c2f80, 0x1a0f24, 0xff3a3a) } else { (0x1c1e26, 0x33415e, 0x101117, 0xffd23a) };
    const BEAK: u32 = 0x5f5d58;
    const BEAK_D: u32 = 0x3b3a37;
    let mut s = Sculpt::new(19, 7, 11);
    s.ellipsoid((9.0, 3.0, 5.0), (2.3, 2.1, 3.3), 0, black);
    s.paint(|_x, y, _z, c| if c == black && y <= 1 { Some(under) } else if c == black && y >= 4 { Some(sheen) } else { None });
    s.ellipsoid((9.0, 4.0, 8.0), (1.7, 1.6, 1.6), 0, black);
    s.set(9, 4, 10, BEAK);
    s.set(9, 3, 10, BEAK_D);
    s.set(9, 4, 9, BEAK);
    s.set(8, 5, 9, eye);
    s.set(10, 5, 9, eye);
    // Tail fan.
    s.cuboid(8, 2, 0, 10, 3, 1, black);
    s.cuboid(7, 3, 0, 11, 3, 0, sheen);
    // Talons.
    for x in [8, 10] {
        s.set(x, 0, 5, BEAK_D);
    }
    // Wings: broad at the body, narrowing to fingered primaries at the tips.
    for i in 0..7 {
        let z0 = 2 + i / 3;
        let z1 = 7 - i / 2;
        let y = 3 + i / 3;
        for (x, _) in [(6 - i, ()), (12 + i, ())] {
            for z in z0..=z1 {
                let c = if z == z1 { sheen } else if i >= 4 && z == z0 { under } else { black };
                s.set(x, y, z, c);
            }
            if i % 2 == 0 && i >= 2 {
                s.set(x, y, z0 - 1, sheen); // primary feather tips
            }
            if i < 2 {
                s.set(x, y - 1, (z0 + z1) / 2, under);
            }
        }
    }
    s.finish(0.07)
}

/// Wings hinge where they meet the body.
pub const RAVEN_RIG: &[PartDef] = &[
    PartDef { name: "leftWing", boxes: &[([0, 0, 0], [6, 6, 10])], pivot: [7.0, 3.5, 5.0] },
    PartDef { name: "rightWing", boxes: &[([12, 0, 0], [18, 6, 10])], pivot: [11.0, 3.5, 5.0] },
];

// ----------------------------------------------------------------- bog slime

/// A bog slime: a glossy dome with bubbles, big eyes and a gummy grin. 11 x 8 x 11.
pub fn bog_slime() -> Collected {
    const GREEN: u32 = 0x3fe05a;
    const DEEP: u32 = 0x1f8a33;
    const LIGHT: u32 = 0xa6ffb6;
    const GLOSS: u32 = 0xeefff0;
    const BUBBLE: u32 = 0x8cffa0;
    const WHITE: u32 = 0xf4f4f8;
    const PUPIL: u32 = 0x14161c;
    const MOUTH: u32 = 0x2c0e14;
    let mut s = Sculpt::new(11, 8, 11);
    s.ellipsoid((5.0, -0.5, 5.0), (5.3, 7.8, 5.3), 0, GREEN);
    s.paint(|x, y, z, _c| {
        if y <= 1 {
            Some(DEEP)
        } else if y >= 5 && x <= 4 && z <= 5 {
            Some(LIGHT)
        } else if noise(x, y, z) < 0.07 {
            Some(BUBBLE)
        } else {
            None
        }
    });
    s.set(3, 6, 4, GLOSS);
    s.set(4, 6, 3, GLOSS);
    s.set(4, 7, 4, GLOSS);
    // Eyes pop out of the front.
    for x0 in [2, 6] {
        s.cuboid(x0, 4, 10, x0 + 2, 5, 10, WHITE);
        s.set(x0 + 1, 4, 10, PUPIL);
        s.set(x0 + 1, 5, 10, PUPIL);
    }
    s.cuboid(4, 2, 9, 6, 2, 10, MOUTH);
    s.set(5, 2, 10, WHITE);
    s.finish(0.05)
}

// ----------------------------------------------------------------- blacksmith

/**
 * Hrolf the blacksmith: a broad, bald smith with a great black beard, bare
 * arms, a leather apron and a hammer. 17 wide, 23 tall, 12 deep.
 */
pub fn blacksmith() -> Collected {
    const SKIN: u32 = 0xe0a878;
    const SKIN_D: u32 = 0xb87e54;
    const BEARD: u32 = 0x2a1e18;
    const BEARD_L: u32 = 0x4a3a30;
    const SHIRT: u32 = 0x6a6e5a;
    const APRON: u32 = 0x7a4a24;
    const APRON_D: u32 = 0x5a3418;
    const TROUSER: u32 = 0x3e3a34;
    const BOOT: u32 = 0x2e2018;
    const IRON: u32 = 0x4a4e56;
    const IRON_L: u32 = 0x7a808a;
    const HAFT: u32 = 0x6b3f1d;
    const EYE: u32 = 0x1c1c24;
    const BRASS: u32 = 0xd8a040;
    let mut s = Sculpt::new(17, 23, 12);
    // Legs and boots, planted wide.
    for (x0, x1) in [(4, 7), (9, 12)] {
        s.cuboid(x0, 0, 3, x1, 1, 8, BOOT);
        s.cuboid(x0, 2, 4, x1, 6, 7, TROUSER);
    }
    // A barrel chest and belly.
    s.ellipsoid((8.0, 11.0, 5.5), (5.5, 5.0, 3.6), 6, SHIRT);
    s.cuboid(3, 7, 3, 13, 14, 8, SHIRT);
    // The apron, belt and a brass buckle.
    s.cuboid(4, 3, 9, 12, 14, 9, APRON);
    s.cuboid(5, 5, 10, 7, 7, 10, APRON_D); // pocket
    s.cuboid(3, 8, 3, 13, 8, 9, APRON_D);
    s.set(8, 8, 9, BRASS);
    // Massive bare arms.
    for (x0, x1) in [(0, 2), (14, 16)] {
        s.cuboid(x0, 7, 4, x1, 14, 7, SKIN);
        s.cuboid(x0, 7, 4, x1, 8, 7, SKIN_D);
    }
    // The hammer in his right fist, held out over the anvil.
    for z in 6..=11 {
        s.set(15, 8, z, HAFT);
    }
    s.cuboid(14, 7, 10, 16, 10, 11, IRON);
    s.cuboid(14, 10, 10, 16, 10, 11, IRON_L);
    // Neck, bald head, big ears.
    s.cuboid(6, 15, 4, 10, 15, 7, SKIN);
    s.ellipsoid((8.0, 18.5, 6.0), (3.2, 3.4, 3.1), 16, SKIN);
    s.set(4, 18, 6, SKIN_D);
    s.set(12, 18, 6, SKIN_D);
    s.set(7, 21, 8, 0xf0c0a0); // shine on the scalp
    // Bushy brows and eyes.
    s.cuboid(6, 19, 9, 7, 19, 9, BEARD);
    s.cuboid(9, 19, 9, 10, 19, 9, BEARD);
    s.set(6, 18, 9, EYE);
    s.set(10, 18, 9, EYE);
    s.set(8, 17, 10, SKIN_D); // nose
    // The great beard, down over the chest.
    s.cuboid(5, 12, 8, 11, 17, 9, BEARD);
    s.cuboid(6, 10, 9, 10, 13, 10, BEARD);
    s.cuboid(7, 9, 10, 9, 10, 10, BEARD);
    s.cuboid(6, 16, 9, 10, 16, 9, BEARD_L); // mustache
    s.set(8, 16, 10, BEARD_L);
    s.paint(|x, y, _z, c| if c == BEARD && (x + y * 2) % 5 == 0 { Some(BEARD_L) } else { None });
    s.finish(0.05)
}

/// The hammer arm swings from the shoulder; the head nods.
pub const BLACKSMITH_RIG: &[PartDef] = &[
    PartDef { name: "hammerArm", boxes: &[([14, 6, 3], [16, 14, 11])], pivot: [15.0, 13.5, 5.5] },
    PartDef { name: "head", boxes: &[([3, 15, 0], [13, 22, 11])], pivot: [8.0, 15.0, 6.0] },
];

// -------------------------------------------------------------------- cyclops

mod cyc {
    pub const SKIN: u32 = 0x9a4f86;
    pub const SKIN_D: u32 = 0x6e3160;
    pub const SKIN_L: u32 = 0xc987b0;
    pub const WART: u32 = 0x5a2450;
    pub const SCLERA: u32 = 0xf4eedc;
    pub const SCLERA_D: u32 = 0xdcd2b8;
    pub const VEIN: u32 = 0xd23a3a;
    pub const IRIS: u32 = 0xffb21e;
    pub const IRIS_D: u32 = 0xd8641a;
    pub const PUPIL: u32 = 0x160810;
    pub const SHINE: u32 = 0xffffff;
    pub const MAW: u32 = 0x2a0a16;
    pub const TOOTH: u32 = 0xf2ead0;
    pub const HORN: u32 = 0xeadfc2;
    pub const HORN_D: u32 = 0x9c8a66;
    pub const MEMBRANE: u32 = 0x4e1f47;
    pub const BONE: u32 = 0x8c5a7e;
    pub const LASH: u32 = 0x1c0a18;
}

/// The Cyclops' grid: 45 wide (wings spread), 34 tall (horns), 29 deep.
pub const CYCLOPS_SIZE: (i32, i32, i32) = (45, 34, 29);
/// Center of the great eye in its voxel grid; the iris and lids pivot here.
pub const CYCLOPS_EYE: [f32; 3] = [22.0, 18.0, 15.5];
const EYE_R: f32 = 9.3;

/**
 * The Cyclops: a round, warty, mauve body that is mostly one enormous eye,
 * a jagged grin under it, curling horns, stubby bat wings and four
 * tentacles dangling beneath. The iris and the lids are separate layers
 * (`cyclops_iris`, `cyclops_lid`) so it can look around and blink.
 */
pub fn cyclops() -> Collected {
    use cyc::*;
    let (w, h, d) = CYCLOPS_SIZE;
    let [ex, ey, ez] = CYCLOPS_EYE;
    let mut s = Sculpt::new(w, h, d);
    // Body.
    s.ellipsoid((ex, ey, 13.0), (12.0, 11.5, 11.5), 0, SKIN);
    s.paint(|x, y, z, _| {
        let n = noise(x, y, z);
        if (y as f32) < ey - 7.0 {
            Some(if n < 0.3 { SKIN } else { SKIN_L })
        } else if n < 0.06 {
            Some(WART)
        } else if (y as f32) > ey + 6.0 && (z as f32) < 12.0 {
            Some(SKIN_D)
        } else {
            None
        }
    });
    // The eye bulges out of the front.
    s.ellipsoid((ex, ey, ez), (EYE_R, EYE_R, EYE_R), 0, SCLERA);
    s.paint(|x, y, z, c| {
        if c != SCLERA {
            return None;
        }
        let (dx, dy, dz) = (x as f32 - ex, y as f32 - ey, z as f32 - ez);
        let r = (dx * dx + dy * dy + dz * dz).sqrt();
        let front = dz / r.max(0.01);
        // Bloodshot toward the rim, clean around the iris.
        if front < 0.8 && noise(x, y, z) < 0.13 {
            Some(VEIN)
        } else if front < 0.45 {
            Some(SCLERA_D)
        } else {
            None
        }
    });
    // A jagged grin below the eye.
    s.paint(|x, y, z, _| {
        let dx = (x as f32 - ex).abs();
        if !(6..=9).contains(&y) || dx > 6.5 - (9 - y) as f32 * 0.8 || z < 16 {
            return None;
        }
        let tooth = (y == 9 && x % 2 == 0) || (y == 6 && x % 2 == 1);
        Some(if tooth { TOOTH } else { MAW })
    });
    // Horns, curling up and out.
    for side in [-1.0f32, 1.0] {
        for i in 0..9 {
            let k = i as f32 / 8.0;
            let cx = ex + side * (6.0 + k * 5.0 + (k * 2.6).sin() * 1.5);
            let cy = 27.0 + k * 6.0 - k * k * 1.5;
            let cz = 11.5 + k * 2.5;
            let r = 1.9 - k * 1.3;
            let c = if k > 0.7 { HORN_D } else { HORN };
            s.ellipsoid((cx, cy, cz), (r, r, r), 0, c);
        }
    }
    // Stubby bat wings, flat membranes on bony fingers.
    for side in [-1i32, 1] {
        for i in 0..11 {
            let x = ex as i32 + side * (11 + i);
            let top = 23 + i / 3;
            let bottom = 22 - ((10 - i) as f32 * 0.8) as i32 + if i % 4 == 3 { 2 } else { 0 };
            for y in bottom..=top {
                let c = if y == top || (i % 4 == 0 && y > bottom) { BONE } else { MEMBRANE };
                s.set(x, y, 10, c);
                if i < 4 {
                    s.set(x, y, 11, c);
                }
            }
        }
    }
    // Four tentacles hanging beneath, curling at the tips.
    for (k, &(tx, tz)) in TENTACLES.iter().enumerate() {
        for y in 0..6 {
            let curl = if y < 2 { (2 - y) as f32 * 0.8 * if k % 2 == 0 { 1.0 } else { -1.0 } } else { 0.0 };
            let r = 1.0 + y as f32 * 0.22;
            let c = if y % 2 == 0 { SKIN_D } else { SKIN };
            s.ellipsoid((tx + curl, y as f32, tz), (r, 0.7, r), y, c);
        }
    }
    s.finish(0.06)
}

const TENTACLES: [(f32, f32); 4] = [(16.0, 9.0), (28.0, 9.0), (18.0, 17.0), (26.0, 17.0)];

/// The iris and slit pupil: a cap sitting just proud of the eye's front.
pub fn cyclops_iris() -> Collected {
    use cyc::*;
    let (w, h, d) = CYCLOPS_SIZE;
    let [ex, ey, ez] = CYCLOPS_EYE;
    let mut s = Sculpt::new(w, h, d);
    for y in 0..h {
        for z in 0..d {
            for x in 0..w {
                let (dx, dy, dz) = (x as f32 - ex, y as f32 - ey, z as f32 - ez);
                let r = (dx * dx + dy * dy + dz * dz).sqrt();
                if r <= EYE_R || r > EYE_R + 1.2 || dz <= 0.0 {
                    continue;
                }
                let off = (dx * dx + dy * dy).sqrt();
                if off > 4.6 {
                    continue;
                }
                let c = if dx.abs() < 0.9 && dy.abs() < 3.6 {
                    PUPIL
                } else if off > 3.6 {
                    IRIS_D
                } else {
                    IRIS
                };
                s.set(x, y, z, c);
            }
        }
    }
    s.set(ex as i32 - 2, ey as i32 + 2, (ez + EYE_R + 0.8) as i32, SHINE);
    s.finish(0.05)
}

/// An eyelid shell: `top` covers the eye from above, else from below. At rest it is folded back.
pub fn cyclops_lid(top: bool) -> Collected {
    use cyc::*;
    let (w, h, d) = CYCLOPS_SIZE;
    let [ex, ey, ez] = CYCLOPS_EYE;
    let mut s = Sculpt::new(w, h, d);
    for y in 0..h {
        for z in 0..d {
            for x in 0..w {
                let (dx, dy, dz) = (x as f32 - ex, y as f32 - ey, z as f32 - ez);
                let r = (dx * dx + dy * dy + dz * dz).sqrt();
                if r <= EYE_R + 1.2 || r > EYE_R + 2.3 || dz <= -2.0 {
                    continue;
                }
                let elev = (dy / r).asin();
                let edge = if top { 0.62 } else { -0.72 };
                let inside = if top { elev > edge } else { elev < edge };
                if !inside {
                    continue;
                }
                let rim = (elev - edge).abs() < 0.12;
                s.set(x, y, z, if rim { SKIN_L } else if noise(x, y, z) < 0.08 { WART } else { SKIN });
                if top && rim && dz > 2.0 && x % 2 == 0 {
                    s.set(x, y - 1, z + 1, LASH);
                }
            }
        }
    }
    s.finish(0.05)
}

/// Wings hinge at the body; tentacles hang from their roots.
pub const CYCLOPS_RIG: &[PartDef] = &[
    PartDef { name: "leftWing", boxes: &[([0, 10, 0], [10, 30, 28])], pivot: [10.5, 22.0, 10.5] },
    PartDef { name: "rightWing", boxes: &[([34, 10, 0], [44, 30, 28])], pivot: [33.5, 22.0, 10.5] },
    PartDef { name: "tentacle0", boxes: &[([12, 0, 5], [20, 5, 13])], pivot: [16.0, 6.0, 9.0] },
    PartDef { name: "tentacle1", boxes: &[([24, 0, 5], [32, 5, 13])], pivot: [28.0, 6.0, 9.0] },
    PartDef { name: "tentacle2", boxes: &[([14, 0, 14], [21, 5, 21])], pivot: [18.0, 6.0, 17.0] },
    PartDef { name: "tentacle3", boxes: &[([23, 0, 14], [30, 5, 21])], pivot: [26.0, 6.0, 17.0] },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::voxels::build_rig;

    #[test]
    fn the_cyclops_has_an_eye_that_moves_and_lids_that_blink() {
        let r = crate::render::voxels::build_layered_rig(
            &cyclops(),
            CYCLOPS_RIG,
            &[("iris", &cyclops_iris(), CYCLOPS_EYE), ("lidTop", &cyclops_lid(true), CYCLOPS_EYE), ("lidBottom", &cyclops_lid(false), CYCLOPS_EYE)],
            0.07,
        );
        for name in ["body", "iris", "lidTop", "lidBottom", "leftWing", "rightWing", "tentacle0", "tentacle3"] {
            let i = r.index(name).unwrap_or_else(|| panic!("no {name}"));
            assert!(r.parts[i].cubes.len() > 10, "{name} has {} cubes", r.parts[i].cubes.len());
        }
    }

    #[test]
    fn every_character_is_sculpted_and_rigged() {
        for (model, rig) in [
            (ragnar(), RAGNAR_RIG),
            (ragnar_geared(3, Some(ShieldKind::Dreadnought)), RAGNAR_RIG),
            (ragnar_geared(2, None), RAGNAR_RIG),
            (ragnar_hd(1, Some(ShieldKind::Round)), RAGNAR_HD_RIG),
            (ragnar_hd(3, Some(ShieldKind::Dreadnought)), RAGNAR_HD_RIG),
            (ragnar_hd(2, Some(ShieldKind::Kite)), RAGNAR_HD_RIG),
            (blacksmith(), BLACKSMITH_RIG),
            (draugr(), DRAUGR_RIG),
            (raven(false), RAVEN_RIG),
            (raven(true), RAVEN_RIG),
            (bog_slime(), &[] as &[PartDef]),
            (mehmed(), MEHMED_RIG),
            (miyamoto(), MIYAMOTO_RIG),
        ] {
            assert!(model.voxels.len() > 100, "too few voxels: {}", model.voxels.len());
            let r = build_rig(&model, rig, 0.05);
            for part in &r.parts {
                assert!(!part.cubes.is_empty(), "empty part {}", part.name);
            }
        }
    }

    #[test]
    fn detailed_ragnar_carries_materials() {
        let model = ragnar_hd(1, Some(ShieldKind::Round));
        let rig = build_rig(&model, RAGNAR_HD_RIG, 0.025);
        let shades: std::collections::HashSet<i32> = rig.parts.iter().flat_map(|p| p.shade.iter().map(|&s| s as i32)).collect();
        for m in 6..=13 {
            assert!(shades.contains(&m), "no voxels of material {m}");
        }
        // The axe goes with the arm, the horns with the head.
        let arm = &rig.parts[rig.index("swordArm").unwrap()];
        assert!(arm.cubes.len() > 150, "sword arm has {} cubes", arm.cubes.len());
    }

    #[test]
    fn interiors_are_culled() {
        let mut s = Sculpt::new(5, 5, 5);
        s.cuboid(0, 0, 0, 4, 4, 4, 0xffffff);
        assert_eq!(s.finish(0.0).voxels.len(), 125 - 27);
    }
}
