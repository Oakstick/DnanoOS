//! The model of the ellepiger, the elf-maids of the Garden of Thorns (REALMS.md §6): for now a
//! plain stand-in figure (legs, body, arms, head), facing +z, in the parts its rig swings.

use super::characters::{mat, Sculpt};
use super::voxels::{Collected, PartDef};

/// The model's grid, and the size of one of its voxels in tiles.
pub const ELLEPIGE_SIZE: (i32, i32, i32) = (28, 64, 20);
pub const ELLEPIGE_VOXEL: f32 = 0.0125;

pub const ELLEPIGE_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [10.0, 24.0, 10.0] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [18.0, 24.0, 10.0] },
    PartDef { name: "leftArm", boxes: &[], pivot: [6.0, 48.0, 10.0] },
    PartDef { name: "rightArm", boxes: &[], pivot: [22.0, 48.0, 10.0] },
    PartDef { name: "head", boxes: &[], pivot: [14.0, 52.0, 10.0] },
];

const BODY: u32 = 0xf4ecf0 | mat::CLOTH;
const DARK: u32 = 0xd8c8d0 | mat::CLOTH;
const SKIN: u32 = 0xf0d8c8 | mat::SKIN;
const EYE: u32 = 0x101010;

pub fn ellepige() -> Collected {
    let (w, h, d) = ELLEPIGE_SIZE;
    let mut s = Sculpt::new(w, h, d);
    for (i, x) in [10.0f32, 18.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        s.capsule((x, 24.0, 10.0), (x, 3.0, 10.0), 2.6, 2.2, DARK);
    }
    s.part(0);
    s.capsule((14.0, 26.0, 10.0), (14.0, 47.0, 10.0), 6.0, 5.0, BODY);
    s.ellipsoid((14.0, 34.0, 11.0), (6.5 * 0.8, 6.0, 5.5 * 0.8), 0, BODY);
    for (i, x) in [6.0f32, 22.0].into_iter().enumerate() {
        s.part(3 + i as u8);
        s.capsule((x, 48.0, 10.0), (x, 30.0, 11.0), 2.2, 1.8, BODY);
        s.ellipsoid((x, 29.0, 11.0), (1.8, 1.8, 1.8), 0, SKIN);
    }
    s.part(5);
    s.ellipsoid((14.0, 56.0, 10.0), (5.0, 6.0, 5.0), 0, SKIN);
    for x in [12, 16] {
        s.set(x, 57, 15, EYE);
    }
    s.finish(0.0)
}
