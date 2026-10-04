//! Gluttony's court's own: Beelzebub, the prince of Gluttony, and his swarms of flies. Placeholders until he is
//! sculpted: a plain robed figure in his colour about Mammon's height (`super::robed`).
//!
//! Each faces +z, stands in the middle of its grid and is sculpted in the parts its rig swings.

use super::{robed, Robed, ADD_RIG, ADD_SIZE, ADD_VOXEL, PRINCE_RIG, PRINCE_SIZE, PRINCE_VOXEL};
use crate::render::voxels::{Collected, PartDef};

/// Beelzebub's grid and voxel, and his rig.
pub const BEELZEBUB_SIZE: (i32, i32, i32) = PRINCE_SIZE;
pub const BEELZEBUB_VOXEL: f32 = PRINCE_VOXEL;
pub const BEELZEBUB_RIG: &[PartDef] = PRINCE_RIG;

/// Beelzebub, for now a robed figure in his colour.
pub fn beelzebub() -> Collected {
    robed(BEELZEBUB_SIZE, Robed::of(0x9ab030, 0x6a4a20, 0x8a8a40, 0xff3020))
}

/// The fly swarm's grid and voxel, and its rig.
pub const FLY_SWARM_SIZE: (i32, i32, i32) = ADD_SIZE;
pub const FLY_SWARM_VOXEL: f32 = ADD_VOXEL;
pub const FLY_SWARM_RIG: &[PartDef] = ADD_RIG;

/// A fly swarm, for now a small robed figure in its colour.
pub fn fly_swarm() -> Collected {
    robed(FLY_SWARM_SIZE, Robed::of(0x4a5020, 0x2a2a10, 0x5a6030, 0xff3020))
}
