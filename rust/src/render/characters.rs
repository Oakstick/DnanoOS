//! The cast, sculpted in code rather than typed as text grids: boxes,
//! ellipsoids and discs carved into a voxel grid, with a little color jitter
//! so surfaces read as hand-painted rather than flat. Interior voxels are
//! culled, so detail costs only what the camera can see.
//!
//! Axes follow the text models: y up from the feet, z from the back (0) to
//! the front (the face), x across. Characters face +z.

use super::voxels::{Collected, PartDef, Voxel};

/// The shields the small portrait model can carry.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShieldKind {
    Round,
    IronRound,
    Kite,
    Dreadnought,
}

pub struct Sculpt {
    w: i32,
    h: i32,
    d: i32,
    cells: Vec<Option<u32>>,
    /// Which rig part each cell was set for (see `part`); 0 leaves it to the rig's boxes.
    tags: Vec<u8>,
    tag: u8,
}

impl Sculpt {
    pub fn new(w: i32, h: i32, d: i32) -> Self {
        let n = (w * h * d) as usize;
        Self { w, h, d, cells: vec![None; n], tags: vec![0; n], tag: 0 }
    }

    /// Voxels set from now on belong to rig part `tag` (the tag-th `PartDef`; 0 is the body,
    /// or whatever the rig's boxes say). Recolouring keeps a voxel's part.
    pub fn part(&mut self, tag: u8) {
        self.tag = tag;
    }

    pub fn tag_at(&self, x: i32, y: i32, z: i32) -> u8 {
        self.idx(x, y, z).map(|i| self.tags[i]).unwrap_or(0)
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
            self.tags[i] = self.tag;
        }
    }

    /// Recolours a filled voxel, keeping its part.
    pub fn recolor(&mut self, x: i32, y: i32, z: i32, c: u32) {
        if let Some(i) = self.idx(x, y, z) {
            if self.cells[i].is_some() {
                self.cells[i] = Some(c);
            }
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
        let span = |c: f32, r: f32, n: i32| ((c - r).floor().max(0.0) as i32, ((c + r).ceil() as i32).min(n - 1));
        let ((x0, x1), (y0, y1), (z0, z1)) = (span(c.0, r.0, self.w), span(c.1, r.1, self.h), span(c.2, r.2, self.d));
        for y in y0.max(ymin)..=y1 {
            for z in z0..=z1 {
                for x in x0..=x1 {
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
                            self.recolor(x, y, z, n);
                        }
                    }
                }
            }
        }
    }

    /// Finishes the model: jitters colors a few percent and drops fully enclosed voxels
    /// (a voxel against another part is kept, since that part may swing away from it).
    pub fn finish(self, jitter: f32) -> Collected {
        self.finish_on(jitter, false)
    }

    /// Like `finish`, for a model that never leaves the floor: its underside is hidden too.
    pub fn finish_grounded(self, jitter: f32) -> Collected {
        self.finish_on(jitter, true)
    }

    fn finish_on(self, jitter: f32, grounded: bool) -> Collected {
        let mut voxels = Vec::new();
        for y in 0..self.h {
            for z in 0..self.d {
                for x in 0..self.w {
                    let Some(c) = self.get(x, y, z) else { continue };
                    let tag = self.tag_at(x, y, z);
                    let hidden = [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)]
                        .iter()
                        .all(|(dx, dy, dz)| {
                            (grounded && y + dy < 0) || (self.get(x + dx, y + dy, z + dz).is_some() && self.tag_at(x + dx, y + dy, z + dz) == tag)
                        });
                    if hidden {
                        continue;
                    }
                    let k = 1.0 + (noise(x, y, z) - 0.5) * 2.0 * jitter;
                    voxels.push(Voxel { x, y, z, c: scale(c, k), part: tag });
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
    /// Wet, glossy goo.
    pub const SLIME: u32 = 9 << 24;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::voxels::build_rig;

    #[test]
    fn every_character_is_sculpted_and_rigged() {
        for (model, rig) in [
            (ragnar(), RAGNAR_RIG),
            (ragnar_geared(3, Some(ShieldKind::Dreadnought)), RAGNAR_RIG),
            (ragnar_geared(2, None), RAGNAR_RIG),
            (blacksmith(), BLACKSMITH_RIG),
        ] {
            assert!(model.voxels.len() > 100, "too few voxels: {}", model.voxels.len());
            let r = build_rig(&model, rig, 0.05);
            for part in &r.parts {
                assert!(!part.cubes.is_empty(), "empty part {}", part.name);
            }
        }
    }

    #[test]
    fn interiors_are_culled() {
        let mut s = Sculpt::new(5, 5, 5);
        s.cuboid(0, 0, 0, 4, 4, 4, 0xffffff);
        assert_eq!(s.finish(0.0).voxels.len(), 125 - 27);
    }
}
