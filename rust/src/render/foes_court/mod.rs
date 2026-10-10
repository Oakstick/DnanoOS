//! The princes' courts' own (ROADMAP.md, phases 3 to 9): each prince and what he raises, a module
//! per sin. Each faces +z, stands in the middle of its grid and is sculpted in the parts its rig
//! swings (legs, arms and head about the body).
//!
//! `greed` holds Mammon and his gold golems. The other six (`pride`, `envy`, `wrath`, `sloth`,
//! `gluttony`, `lust`) are placeholders until their princes are sculpted: a plain robed figure in
//! the prince's colour about Mammon's height, and a small one for his adds, cheap to build (every
//! rig is built at startup), on the shared grids and rigs here (`robed`).

pub mod envy;
pub mod gluttony;
pub mod greed;
pub mod lust;
pub mod pride;
pub mod sloth;
pub mod wrath;

pub use greed::{gold_golem, mammon, GOLEM_RIG, GOLEM_SIZE, GOLEM_VOXEL, MAMMON_RIG, MAMMON_SIZE, MAMMON_VOXEL};

use crate::render::carve::{ball, v};
use crate::render::characters::{mat, Sculpt};
use crate::render::voxels::{Collected, PartDef};

/// A placeholder prince's grid and voxel: about three tiles tall, as Mammon is.
pub const PRINCE_SIZE: (i32, i32, i32) = (40, 64, 32);
pub const PRINCE_VOXEL: f32 = 0.05;
/// A placeholder add's grid and voxel: about a tile and a half tall.
pub const ADD_SIZE: (i32, i32, i32) = (24, 40, 20);
pub const ADD_VOXEL: f32 = 0.04;

/// The rig of a robed figure in a grid of `w` x `h` x `d`: hips, shoulders and neck.
const fn robed_rig(w: i32, h: i32, d: i32) -> [PartDef; 5] {
    let (cx, cz, f) = (w as f32 / 2.0, d as f32 / 2.0, (h - 2) as f32);
    [
        PartDef { name: "leftLeg", boxes: &[], pivot: [cx - 0.08 * f, 0.32 * f, cz] },
        PartDef { name: "rightLeg", boxes: &[], pivot: [cx + 0.08 * f, 0.32 * f, cz] },
        PartDef { name: "leftArm", boxes: &[], pivot: [cx - 0.17 * f, 0.6 * f, cz] },
        PartDef { name: "rightArm", boxes: &[], pivot: [cx + 0.17 * f, 0.6 * f, cz] },
        PartDef { name: "head", boxes: &[], pivot: [cx, 0.66 * f, cz] },
    ]
}

/// The placeholder prince's rig, and the placeholder add's.
pub const PRINCE_RIG: &[PartDef] = &robed_rig(PRINCE_SIZE.0, PRINCE_SIZE.1, PRINCE_SIZE.2);
pub const ADD_RIG: &[PartDef] = &robed_rig(ADD_SIZE.0, ADD_SIZE.1, ADD_SIZE.2);

/// A placeholder figure's colours (each carrying its material): its robe, the trim at its hem,
/// belt and brow, its face and hands, and its eyes.
#[derive(Clone, Copy, Debug)]
pub struct Robed {
    pub robe: u32,
    pub trim: u32,
    pub skin: u32,
    pub eye: u32,
}

impl Robed {
    /// A robe of `color`, trimmed in `trim`, with a face of `skin` and eyes of `eye` (plain
    /// colours: the materials are added).
    pub const fn of(color: u32, trim: u32, skin: u32, eye: u32) -> Robed {
        Robed { robe: color | mat::CLOTH, trim: trim | mat::GOLD, skin: skin | mat::SKIN, eye: eye | mat::GLOW }
    }
}

/// A plain robed figure filling a grid of `size` (`PRINCE_SIZE` or `ADD_SIZE`): a bell of a robe
/// to the floor, feet under its hem, sleeved arms, a round head with two glowing eyes and a band
/// of trim about the brow. Parts as `robed_rig`: 0 the body, 1 and 2 the legs, 3 and 4 the arms,
/// 5 the head.
pub fn robed(size: (i32, i32, i32), c: Robed) -> Collected {
    let (w, h, d) = size;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz, f) = (w as f32 / 2.0, d as f32 / 2.0, (h - 2) as f32);
    // The robe, its hem and belt trimmed.
    s.part(0);
    s.capsule((cx, 0.1 * f, cz), (cx, 0.6 * f, cz), 0.2 * f, 0.13 * f, c.robe);
    s.capsule((cx, 0.08 * f, cz), (cx, 0.1 * f, cz), 0.205 * f, 0.205 * f, c.trim);
    s.capsule((cx, 0.42 * f, cz), (cx, 0.44 * f, cz), 0.172 * f, 0.172 * f, c.trim);
    // Legs under it, the feet showing at the hem.
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        let x = cx + side * 0.08 * f;
        s.capsule((x, 0.32 * f, cz), (x, 0.05 * f, cz), 0.055 * f, 0.055 * f, c.robe);
        ball(&mut s, v(x, 0.03 * f, cz + 0.06 * f), v(0.05 * f, 0.03 * f, 0.08 * f), c.trim);
    }
    // Arms in their sleeves, hands bare.
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(3 + i as u8);
        let (sx, hx) = (cx + side * 0.17 * f, cx + side * 0.22 * f);
        s.capsule((sx, 0.6 * f, cz), (hx, 0.38 * f, cz + 0.04 * f), 0.05 * f, 0.045 * f, c.robe);
        ball(&mut s, v(hx, 0.34 * f, cz + 0.05 * f), v(0.045 * f, 0.05 * f, 0.045 * f), c.skin);
    }
    // The head: a band of trim about the brow, two eyes.
    s.part(5);
    ball(&mut s, v(cx, 0.75 * f, cz), v(0.085 * f, 0.095 * f, 0.085 * f), c.skin);
    s.capsule((cx, 0.8 * f, cz), (cx, 0.83 * f, cz), 0.088 * f, 0.08 * f, c.trim);
    for side in [-1.0f32, 1.0] {
        let (x, y) = ((cx + side * 0.035 * f).round() as i32, (0.76 * f).round() as i32);
        if let Some(z) = (0..d).rev().find(|&z| s.get(x, y, z).is_some()) {
            s.recolor(x, y, z, c.eye);
        }
    }
    s.finish(0.03)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every placeholder: about the height it should be, and every part of its rig with voxels.
    #[test]
    fn every_placeholder_prince_is_about_mammons_height_and_every_add_smaller() {
        let princes = [
            (pride::lucifer(), pride::LUCIFER_RIG, pride::LUCIFER_VOXEL),
            (envy::leviathan(), envy::LEVIATHAN_RIG, envy::LEVIATHAN_VOXEL),
            (wrath::amon(), wrath::AMON_RIG, wrath::AMON_VOXEL),
            (sloth::belphegor(), sloth::BELPHEGOR_RIG, sloth::BELPHEGOR_VOXEL),
            (gluttony::beelzebub(), gluttony::BEELZEBUB_RIG, gluttony::BEELZEBUB_VOXEL),
            (lust::asmodeus(), lust::ASMODEUS_RIG, lust::ASMODEUS_VOXEL),
        ];
        let adds = [
            (pride::reflection(), pride::REFLECTION_RIG, pride::REFLECTION_VOXEL),
            (envy::serpentling(), envy::SERPENTLING_RIG, envy::SERPENTLING_VOXEL),
            (wrath::raven(), wrath::RAVEN_RIG, wrath::RAVEN_VOXEL),
            (sloth::nightmare(), sloth::NIGHTMARE_RIG, sloth::NIGHTMARE_VOXEL),
            (gluttony::fly_swarm(), gluttony::FLY_SWARM_RIG, gluttony::FLY_SWARM_VOXEL),
        ];
        for (i, (c, rig, voxel)) in princes.iter().chain(adds.iter()).enumerate() {
            let top = c.voxels.iter().map(|v| v.y).max().unwrap() as f32 * voxel;
            if i < princes.len() {
                assert!(top > 2.4 && top < 3.6, "prince {i}: {top} tiles tall");
            } else {
                assert!(top > 1.0 && top < 2.2, "add {i}: {top} tiles tall");
            }
            for part in 0..=rig.len() as u8 {
                assert!(c.voxels.iter().any(|v| v.part == part), "{i}: part {part}");
            }
        }
    }
}
