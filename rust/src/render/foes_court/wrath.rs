//! Wrath's court's own: Amon, the prince of Wrath, and his ravens. Placeholders until he is
//! sculpted: a plain robed figure in his colour about Mammon's height (`super::robed`).
//!
//! Each faces +z, stands in the middle of its grid and is sculpted in the parts its rig swings.

use super::{robed, Robed, ADD_RIG, ADD_SIZE, ADD_VOXEL, PRINCE_RIG, PRINCE_SIZE, PRINCE_VOXEL};
use crate::render::voxels::{Collected, PartDef};

/// Amon's grid and voxel, and his rig.
pub const AMON_SIZE: (i32, i32, i32) = PRINCE_SIZE;
pub const AMON_VOXEL: f32 = PRINCE_VOXEL;
pub const AMON_RIG: &[PartDef] = PRINCE_RIG;

/// Amon, for now a robed figure in his colour.
pub fn amon() -> Collected {
    robed(AMON_SIZE, Robed::of(0xe0402a, 0x3a2a2a, 0xa83020, 0xffb020))
}

/// The raven's grid and voxel, and its rig.
pub const RAVEN_SIZE: (i32, i32, i32) = ADD_SIZE;
pub const RAVEN_VOXEL: f32 = ADD_VOXEL;
pub const RAVEN_RIG: &[PartDef] = ADD_RIG;

/// A raven, for now a small robed figure in its colour.
pub fn raven() -> Collected {
    robed(RAVEN_SIZE, Robed::of(0x3a3040, 0x5a4a5a, 0x2a2430, 0xff4020))
}
