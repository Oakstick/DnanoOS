//! The Serpent Grotto's models, sculpted in voxels and merged into boxes (`vault_decor::merge`):
//! Leviathan's head, the dragon loop on the bell's crown, a drowned chest, a bone raft, a heap of
//! gold, a stone lantern, a giant clam with its pearl, and the skull of a sea-serpent.

use super::super::super::vault_decor::{merge, Merged};
use super::super::super::*;
use crate::render::characters::{mat, Sculpt};

/// Nacre, as a voxel material (the polished marble's gloss, shader kind 10).
const NACRE_MAT: u32 = 10 << 24;

pub(super) mod pal {
    use super::{mat, NACRE_MAT};
    pub const SCALE: u32 = 0x173226 | mat::SLIME;
    pub const SCALE_D: u32 = 0x0c1e16 | mat::SLIME;
    pub const SCALE_L: u32 = 0x2f6a52 | mat::SLIME;
    pub const VERDIGRIS: u32 = 0x4aa088 | mat::GOLD;
    pub const BELLY: u32 = 0xc4bc96 | mat::IVORY;
    pub const BONE: u32 = 0xe6dcc0 | mat::IVORY;
    pub const BONE_D: u32 = 0xb8ac8a | mat::IVORY;
    pub const TOOTH: u32 = 0xf4eedc | mat::IVORY;
    pub const EYE: u32 = 0xb6ff5a | mat::GLOW;
    pub const PUPIL: u32 = 0x050805;
    pub const MAW: u32 = 0x5a1420;
    pub const BRONZE: u32 = 0xa8743a | mat::GOLD;
    pub const BRONZE_D: u32 = 0x6a4a26 | mat::GOLD;
    pub const GOLD: u32 = 0xf2c53d | mat::GOLD;
    pub const GOLD_D: u32 = 0xb8902a | mat::GOLD;
    pub const GOLD_L: u32 = 0xfff0a0 | mat::GOLD;
    pub const WOOD: u32 = 0x3a2a1c | mat::WOOD;
    pub const WOOD_L: u32 = 0x54402a | mat::WOOD;
    pub const ROPE: u32 = 0x5a4a30 | mat::CLOTH;
    pub const STONE: u32 = 0x4a504c;
    pub const STONE_D: u32 = 0x343a38;
    pub const MOSS: u32 = 0x3a6a3a;
    pub const FLAME: u32 = 0xe8ff9a | mat::GLOW;
    pub const NACRE: u32 = 0xeae2f2 | NACRE_MAT;
    pub const NACRE_P: u32 = 0xf2c8d8 | NACRE_MAT;
    pub const SHELL: u32 = 0x8a8496 | NACRE_MAT;
    pub const PEARL: u32 = 0xfdfaff | mat::GLOW;
}
use pal::*;

/// The grotto's models, merged.
pub(in crate::render::view) struct GrottoArt {
    pub head: Merged,
    pub crown: Merged,
    pub chest: Merged,
    pub raft: Merged,
    pub gold: Merged,
    pub lantern: Merged,
    pub clam: Merged,
    pub skull: Merged,
}

impl GrottoArt {
    pub fn new() -> Self {
        Self { head: head(), crown: crown(), chest: chest(), raft: raft(), gold: gold(), lantern: lantern(), clam: clam(), skull: skull() }
    }
}

/// The voxel of Leviathan's head: about two and a quarter tiles from crest to snout.
const HEAD_VOXEL: f32 = 0.075;

/// Leviathan's head, facing +z: a long wet skull with a crest of fins, horns swept back, brows of
/// verdigris over burning green eyes slit like a cat's, the jaws parted on rows of teeth.
fn head() -> Merged {
    let mut s = Sculpt::new(17, 14, 31);
    s.ellipsoid((8.0, 6.5, 9.0), (6.2, 4.6, 8.0), 0, SCALE);
    s.capsule((8.0, 6.2, 12.0), (8.0, 5.0, 28.5), 4.6, 2.6, SCALE);
    s.capsule((8.0, 2.4, 11.0), (8.0, 2.0, 26.5), 3.6, 1.8, SCALE_D);
    // The gape: the upper jaw lifted off the lower, the maw red inside.
    for z in 15..31 {
        for x in 0..17 {
            for y in 3..5 {
                if s.get(x, y, z).is_some() {
                    let edge = x <= 4 || x >= 12;
                    s.set(x, y, z, if edge && (z % 2 == 0) { TOOTH } else { MAW });
                }
            }
        }
    }
    for z in (16..29).step_by(2) {
        let w = if z < 22 { 3 } else { 2 };
        for &x in &[8 - w - 1, 8 + w + 1] {
            s.set(x, 4, z, TOOTH);
            s.set(x, 3, z + 1, TOOTH);
        }
    }
    // Brows and eyes.
    for side in [-1.0f32, 1.0] {
        let bx = 8.0 + side * 4.4;
        s.ellipsoid((bx, 8.6, 13.0), (1.9, 1.3, 3.4), 0, VERDIGRIS);
        let ex = (8.0 + side * 5.3).round() as i32;
        for (dy, dz) in [(0, 0), (0, 1), (1, 0), (1, 1), (-1, 0), (0, -1)] {
            s.set(ex, 7 + dy, 14 + dz, EYE);
        }
        s.set(ex + side as i32, 7, 14, EYE);
        s.set(ex, 7, 14, PUPIL);
        s.set(ex, 8, 14, PUPIL);
        // Horns, swept back over the neck.
        s.capsule((8.0 + side * 3.6, 9.5, 8.0), (8.0 + side * 6.5, 13.0, 0.5), 1.4, 0.5, BONE);
        // Gill frills behind the jaw.
        for k in 0..4 {
            s.capsule(
                (8.0 + side * 5.0, 4.0 + k as f32, 4.0 - k as f32 * 0.5),
                (8.0 + side * 7.8, 5.0 + k as f32 * 1.2, 1.0),
                0.6,
                0.3,
                SCALE_L,
            );
        }
    }
    // A crest of fins down the middle of the skull.
    for z in 0..14 {
        let h = 11 + ((z as f32 * 0.9).sin() * 1.5 + 1.5) as i32;
        for y in 10..=h.min(13) {
            s.set(8, y, z, if y == h.min(13) { VERDIGRIS } else { SCALE_L });
        }
    }
    // Nostrils at the snout's tip.
    s.set(6, 7, 28, PUPIL);
    s.set(10, 7, 28, PUPIL);
    // Scales: rows of lighter edges, the throat pale.
    s.paint(|x, y, z, c| {
        if c == SCALE || c == SCALE_D {
            if y <= 1 {
                return Some(BELLY);
            }
            if (x + z * 2 + y) % 5 == 0 {
                return Some(SCALE_L);
            }
            if (x * 3 + z + y * 2) % 7 == 0 {
                return Some(SCALE_D);
            }
        }
        None
    });
    merge(&s.finish(0.04), HEAD_VOXEL)
}

/// The dragon loop (ryūzu) on the bell's crown: two dragon heads back to back, their bodies an arch.
fn crown() -> Merged {
    let mut s = Sculpt::new(24, 16, 9);
    for k in 0..=20 {
        let a = PI * k as f32 / 20.0;
        let (x, y) = (12.0 - a.cos() * 8.0, 2.0 + a.sin() * 9.0);
        s.ellipsoid((x, y, 4.0), (1.8, 1.8, 2.2), 0, if k % 3 == 0 { BRONZE_D } else { BRONZE });
    }
    for side in [-1.0f32, 1.0] {
        let x = 12.0 + side * 8.5;
        s.ellipsoid((x, 2.5, 4.0), (2.6, 2.2, 3.4), 0, BRONZE);
        s.capsule((x, 2.0, 4.0), (x + side * 3.0, 1.0, 4.0), 1.6, 1.0, BRONZE);
        s.set((x + side * 1.2) as i32, 3, 2, VERDIGRIS);
        s.set((x + side * 1.2) as i32, 3, 6, VERDIGRIS);
        s.capsule((x - side * 1.0, 4.0, 4.0), (x - side * 2.0, 7.0, 4.0), 0.7, 0.3, VERDIGRIS);
    }
    s.cuboid(4, 0, 2, 20, 0, 6, BRONZE_D);
    merge(&s.finish(0.03), 0.09)
}

/// A drowned chest, facing +z: blackened wood, its bands green with verdigris, gold spilling out.
fn chest() -> Merged {
    let mut s = Sculpt::new(12, 9, 8);
    s.cuboid(0, 0, 0, 11, 5, 7, WOOD);
    for y in 6..9 {
        let inset = y - 5;
        s.cuboid(0, y, inset.min(2), 11, y, 7 - inset.min(2), WOOD_L);
    }
    s.paint(|x, _, z, c| {
        Some(if x == 2 || x == 9 {
            VERDIGRIS
        } else if (x + z) % 5 == 0 && c == WOOD {
            WOOD_L
        } else {
            c
        })
    });
    s.cuboid(5, 3, 7, 6, 5, 7, GOLD);
    // The lid sprung: gold over the lip.
    for x in 1..11 {
        s.set(x, 6, 7, if x % 3 == 0 { GOLD_L } else { GOLD });
    }
    for (x, y, z) in [(3, 0, 8), (4, 1, 8), (7, 0, 8), (9, 1, 8)] {
        s.set(x, y, z.min(7), GOLD);
    }
    merge(&s.finish(0.03), 0.06)
}

/// A raft of serpent bones lashed together.
fn raft() -> Merged {
    let mut s = Sculpt::new(24, 4, 16);
    for k in 0..6 {
        let z = 1.5 + k as f32 * 2.6;
        s.capsule((1.0, 1.0, z), (22.5, 1.2, z + 0.4), 1.1, 0.9, if k % 2 == 0 { BONE } else { BONE_D });
    }
    for x in [5.0f32, 18.0] {
        s.capsule((x, 2.2, 0.5), (x + 0.5, 2.2, 15.0), 0.9, 0.8, BONE_D);
        for k in 0..6 {
            let z = (1.5 + k as f32 * 2.6) as i32;
            s.set(x as i32, 3, z, ROPE);
            s.set(x as i32 + 1, 3, z, ROPE);
        }
    }
    // A vertebra or two on the deck.
    s.ellipsoid((11.0, 3.0, 7.0), (1.6, 1.0, 1.4), 0, BONE);
    s.set(11, 3, 5, BONE_D);
    merge(&s.finish(0.05), 0.055)
}

/// A heap of gold coins (and a goblet).
fn gold() -> Merged {
    let mut s = Sculpt::new(12, 7, 12);
    s.ellipsoid((6.0, 0.0, 6.0), (5.6, 4.8, 5.0), 0, GOLD);
    s.ellipsoid((3.4, 0.5, 7.4), (2.8, 2.6, 2.4), 0, GOLD);
    s.capsule((8.0, 3.5, 4.0), (8.0, 6.0, 4.0), 0.6, 1.2, GOLD_L);
    s.paint(|x, y, z, _| {
        let n = ((x * 7 + y * 13 + z * 3) % 11) as f32 / 11.0;
        Some(if n < 0.14 {
            GOLD_L
        } else if n > 0.8 || y == 0 {
            GOLD_D
        } else {
            GOLD
        })
    });
    merge(&s.finish(0.0), 0.05)
}

/// A stone lantern (tōrō) green with moss, its fire-box burning.
fn lantern() -> Merged {
    let mut s = Sculpt::new(11, 24, 11);
    s.cuboid(1, 0, 1, 9, 1, 9, STONE_D);
    s.cuboid(2, 2, 2, 8, 2, 8, STONE);
    s.cuboid(4, 3, 4, 6, 11, 6, STONE);
    s.cuboid(2, 12, 2, 8, 12, 8, STONE_D);
    s.cuboid(3, 13, 3, 7, 16, 7, STONE);
    // The light, out of its four windows.
    s.cuboid(4, 14, 3, 6, 15, 7, FLAME);
    s.cuboid(3, 14, 4, 7, 15, 6, FLAME);
    for y in 17..20 {
        let r = 19 - y;
        s.cuboid(5 - r - 2, y, 5 - r - 2, 5 + r + 2, y, 5 + r + 2, STONE_D);
    }
    for (x, z) in [(0, 0), (10, 0), (0, 10), (10, 10)] {
        s.set(x, 18, z, STONE_D);
    }
    s.cuboid(5, 20, 5, 5, 22, 5, STONE);
    s.set(5, 23, 5, BRONZE);
    s.paint(|x, y, z, c| ((c == STONE || c == STONE_D) && (x + y * 3 + z * 2) % 6 == 0 && y < 13).then_some(MOSS));
    merge(&s.finish(0.05), 0.055)
}

/// A giant clam, half open on its pearl.
fn clam() -> Merged {
    let mut s = Sculpt::new(14, 8, 11);
    s.ellipsoid((7.0, 0.0, 5.5), (6.6, 2.4, 5.0), 0, SHELL);
    for k in 0..6 {
        let x = 1.5 + k as f32 * 2.2;
        s.capsule((x, 2.0, 1.0), (x, 6.5, 2.5), 0.9, 0.7, if k % 2 == 0 { SHELL } else { NACRE });
    }
    s.ellipsoid((7.0, 2.0, 6.0), (5.0, 1.2, 3.6), 0, NACRE_P);
    s.ellipsoid((7.0, 3.0, 6.0), (1.4, 1.4, 1.4), 0, PEARL);
    merge(&s.finish(0.03), 0.045)
}

/// The skull of a sea-serpent, half sunk.
fn skull() -> Merged {
    let mut s = Sculpt::new(14, 9, 22);
    s.ellipsoid((7.0, 3.0, 6.0), (5.4, 4.0, 6.0), 0, BONE);
    s.capsule((7.0, 2.5, 9.0), (7.0, 1.6, 20.5), 3.6, 1.6, BONE);
    for side in [-1.0f32, 1.0] {
        s.ellipsoid((7.0 + side * 3.2, 4.0, 9.0), (1.4, 1.4, 1.8), 0, PUPIL);
        s.capsule((7.0 + side * 3.0, 5.0, 4.0), (7.0 + side * 5.5, 8.0, 0.0), 1.0, 0.4, BONE_D);
    }
    for z in (11..20).step_by(2) {
        s.set(4, 0, z, TOOTH);
        s.set(10, 0, z, TOOTH);
    }
    s.paint(|x, y, z, c| (c == BONE && (x + y + z * 2) % 7 == 0).then_some(BONE_D));
    merge(&s.finish_grounded(0.04), 0.06)
}
