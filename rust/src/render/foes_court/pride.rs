//! Pride's court's own: Lucifer, the prince of Pride, and the reflections he steps out of the light. Placeholders until he is
//! sculpted: a plain robed figure in his colour about Mammon's height (`super::robed`).
//!
//! Each faces +z, stands in the middle of its grid and is sculpted in the parts its rig swings.

use super::{robed, Robed, ADD_RIG, ADD_SIZE, ADD_VOXEL, PRINCE_RIG, PRINCE_SIZE, PRINCE_VOXEL};
use crate::render::voxels::{Collected, PartDef};

/// Lucifer's grid and voxel, and his rig.
pub const LUCIFER_SIZE: (i32, i32, i32) = PRINCE_SIZE;
pub const LUCIFER_VOXEL: f32 = PRINCE_VOXEL;
pub const LUCIFER_RIG: &[PartDef] = PRINCE_RIG;

/// Lucifer, for now a robed figure in his colour.
pub fn lucifer() -> Collected {
    robed(LUCIFER_SIZE, Robed::of(0xfff0c0, 0xe2b040, 0xf0e0d0, 0xffffff))
}

/// The reflection's grid and voxel, and its rig.
pub const REFLECTION_SIZE: (i32, i32, i32) = ADD_SIZE;
pub const REFLECTION_VOXEL: f32 = ADD_VOXEL;
pub const REFLECTION_RIG: &[PartDef] = ADD_RIG;

/// A reflection, for now a small robed figure in its colour.
pub fn reflection() -> Collected {
    robed(REFLECTION_SIZE, Robed::of(0xc8d8ff, 0xe0e8ff, 0xd8e0f0, 0x9ab8ff))
}
