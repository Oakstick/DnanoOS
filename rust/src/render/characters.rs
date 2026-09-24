//! The cast, sculpted in code rather than typed as text grids: boxes,
//! ellipsoids and discs carved into a voxel grid, with a little color jitter
//! so surfaces read as hand-painted rather than flat. Interior voxels are
//! culled, so detail costs only what the camera can see.
//!
//! Axes follow the text models: y up from the feet, z from the back (0) to
//! the front (the face), x across. Characters face +z.

use super::voxels::{Collected, PartDef, Voxel};

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
    ch(16) | ch(8) | ch(0)
}

// ------------------------------------------------------------------ Sebastian

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

/// Sebastian the viking: 15 wide, 9 deep, 23 tall.
pub fn sebastian() -> Collected {
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
    // Round shield slung on the back: quartered paint, iron rim, gold boss.
    for y in 5..=15 {
        for x in 2..=12 {
            let (dx, dy) = (x as f32 - 7.0, y as f32 - 10.0);
            let r = (dx * dx + dy * dy).sqrt();
            if r > 4.4 {
                continue;
            }
            let c = if r < 1.2 {
                GOLD
            } else if r > 3.6 {
                IRON
            } else if (dx >= 0.0) != (dy >= 0.0) {
                SHIELD_RED
            } else {
                SHIELD_CREAM
            };
            s.set(x, y, 0, c);
        }
    }
    // Arms: sleeves, leather bracers, hands.
    for (x0, x1) in [(1, 2), (12, 13)] {
        s.cuboid(x0, 11, 3, x1, 13, 4, TUNIC);
        s.cuboid(x0, 9, 3, x1, 10, 4, LEATHER);
        s.cuboid(x0, 7, 3, x1, 8, 4, SKIN);
    }
    // The bearded axe in his right hand.
    for y in 5..=17 {
        s.set(13, y, 4, HAFT);
    }
    s.cuboid(12, 7, 3, 13, 8, 4, SKIN);
    for (z, y0, y1) in [(5, 14, 17), (6, 13, 17), (7, 12, 18)] {
        for y in y0..=y1 {
            s.set(13, y, z, STEEL);
        }
    }
    for y in 11..=18 {
        s.set(13, y, 8, EDGE);
    }
    s.set(13, 11, 7, STEEL);
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

/// Sebastian's joints. Leg boxes stop short of the cloak and shield on his back.
pub const SEBASTIAN_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[([3, 0, 2], [6, 6, 8])], pivot: [5.0, 7.0, 4.0] },
    PartDef { name: "rightLeg", boxes: &[([8, 0, 2], [11, 6, 8])], pivot: [9.0, 7.0, 4.0] },
    PartDef { name: "shieldArm", boxes: &[([0, 5, 2], [2, 14, 8])], pivot: [2.0, 13.5, 3.5] },
    PartDef { name: "swordArm", boxes: &[([12, 4, 2], [14, 18, 8])], pivot: [12.0, 13.5, 3.5] },
    PartDef { name: "head", boxes: &[([0, 15, 0], [14, 22, 8]), ([4, 10, 6], [10, 14, 8])], pivot: [7.0, 14.5, 3.5] },
];

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::voxels::build_rig;

    #[test]
    fn every_character_is_sculpted_and_rigged() {
        for (model, rig) in [
            (sebastian(), SEBASTIAN_RIG),
            (draugr(), DRAUGR_RIG),
            (raven(false), RAVEN_RIG),
            (raven(true), RAVEN_RIG),
            (bog_slime(), &[] as &[PartDef]),
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
