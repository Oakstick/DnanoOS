//! Sloth's court's own: Belphegor, the prince of Sloth, and his nightmares. Placeholders until he is
//! sculpted: a plain robed figure in his colour about Mammon's height (`super::robed`).
//!
//! Each faces +z, stands in the middle of its grid and is sculpted in the parts its rig swings.

use super::{robed, Robed, ADD_RIG, ADD_SIZE, ADD_VOXEL, PRINCE_RIG, PRINCE_SIZE, PRINCE_VOXEL};
use crate::render::voxels::{Collected, PartDef};

/// Belphegor's grid and voxel, and his rig.
pub const BELPHEGOR_SIZE: (i32, i32, i32) = PRINCE_SIZE;
pub const BELPHEGOR_VOXEL: f32 = PRINCE_VOXEL;
pub const BELPHEGOR_RIG: &[PartDef] = PRINCE_RIG;

/// Belphegor, for now a robed figure in his colour.
pub fn belphegor() -> Collected {
    robed(BELPHEGOR_SIZE, Robed::of(0x8a96b0, 0x6a6a5a, 0x9aa0a8, 0xc0d0ff))
}

/// The nightmare's grid and voxel, and its rig.
pub const NIGHTMARE_SIZE: (i32, i32, i32) = ADD_SIZE;
pub const NIGHTMARE_VOXEL: f32 = ADD_VOXEL;
pub const NIGHTMARE_RIG: &[PartDef] = ADD_RIG;

/// A nightmare, for now a small robed figure in its colour.
pub fn nightmare() -> Collected {
    robed(NIGHTMARE_SIZE, Robed::of(0x6a5a9a, 0x3a3060, 0x8070b0, 0xff60c0))
}
