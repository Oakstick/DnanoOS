//! Lust's court's own: Asmodeus, the prince of Lust, and none of his own (his are the elf-maids of the Garden of Thorns, `foes_thorns`). Placeholders until he is
//! sculpted: a plain robed figure in his colour about Mammon's height (`super::robed`).
//!
//! Each faces +z, stands in the middle of its grid and is sculpted in the parts its rig swings.

use super::{robed, Robed, PRINCE_RIG, PRINCE_SIZE, PRINCE_VOXEL};
use crate::render::voxels::{Collected, PartDef};

/// Asmodeus's grid and voxel, and his rig.
pub const ASMODEUS_SIZE: (i32, i32, i32) = PRINCE_SIZE;
pub const ASMODEUS_VOXEL: f32 = PRINCE_VOXEL;
pub const ASMODEUS_RIG: &[PartDef] = PRINCE_RIG;

/// Asmodeus, for now a robed figure in his colour.
pub fn asmodeus() -> Collected {
    robed(ASMODEUS_SIZE, Robed::of(0xe0507a, 0xe2b040, 0xf0b0b0, 0xff80c0))
}
