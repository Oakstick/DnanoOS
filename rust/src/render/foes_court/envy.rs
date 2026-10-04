//! Envy's court's own: Leviathan, the prince of Envy, and his brood, the serpentlings. Placeholders until he is
//! sculpted: a plain robed figure in his colour about Mammon's height (`super::robed`).
//!
//! Each faces +z, stands in the middle of its grid and is sculpted in the parts its rig swings.

use super::{robed, Robed, ADD_RIG, ADD_SIZE, ADD_VOXEL, PRINCE_RIG, PRINCE_SIZE, PRINCE_VOXEL};
use crate::render::voxels::{Collected, PartDef};

/// Leviathan's grid and voxel, and his rig.
pub const LEVIATHAN_SIZE: (i32, i32, i32) = PRINCE_SIZE;
pub const LEVIATHAN_VOXEL: f32 = PRINCE_VOXEL;
pub const LEVIATHAN_RIG: &[PartDef] = PRINCE_RIG;

/// Leviathan, for now a robed figure in his colour.
pub fn leviathan() -> Collected {
    robed(LEVIATHAN_SIZE, Robed::of(0x3aa88a, 0x8a7a3a, 0x5ac0a0, 0xd0ff60))
}

/// The serpentling's grid and voxel, and its rig.
pub const SERPENTLING_SIZE: (i32, i32, i32) = ADD_SIZE;
pub const SERPENTLING_VOXEL: f32 = ADD_VOXEL;
pub const SERPENTLING_RIG: &[PartDef] = ADD_RIG;

/// A serpentling, for now a small robed figure in its colour.
pub fn serpentling() -> Collected {
    robed(SERPENTLING_SIZE, Robed::of(0x5ac0a0, 0x3a6a50, 0x7ad0b0, 0xd0ff60))
}
