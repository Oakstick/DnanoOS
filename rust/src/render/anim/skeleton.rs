//! The heroes' jointed skeleton: twenty bones in a hierarchy (hips, spine, chest, neck and head;
//! upper arm, forearm and hand, thigh, shin and foot each side; the weapon on the sword hand, the
//! offhand weapon on the shield hand and the shield on the shield forearm), and the cutting of a
//! hero's rig (its six parts as the code swings them, and the weapons and shield layered on) into
//! those bones. Each voxel keeps to one bone, rigidly: the voxel look stays.
//!
//! The cut is data: each hero has a table of joints, measured on his sculpt as it stands, and a
//! list of cuts, each moving the voxels of one bone that lie in a box to the next bone down the
//! limb (`Joints`). Ragnar's table is here; Mehmed and Miyamoto follow with theirs.

use glam::Vec3;

use crate::logic::heroes::HeroKind;
use crate::render::voxels::{Rig, RigPart};

/// How many bones the skeleton has.
pub const BONE_COUNT: usize = 20;

/// The bones, parents before children: the order of the jointed rig's parts and of a clip's
/// tracks. The names are the glTF nodes' and Blender's bones' (`.L` and `.R` are his own left and
/// right, so Blender mirrors a pose across him).
pub const BONES: [&str; BONE_COUNT] = [
    "hips",
    "spine",
    "chest",
    "neck",
    "head",
    "upper_arm.L",
    "forearm.L",
    "hand.L",
    "upper_arm.R",
    "forearm.R",
    "hand.R",
    "thigh.L",
    "shin.L",
    "foot.L",
    "thigh.R",
    "shin.R",
    "foot.R",
    "blade",
    "offhand",
    "shield",
];

/// A bone by its place in `BONES`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bone {
    Hips,
    Spine,
    Chest,
    Neck,
    Head,
    UpperArmL,
    ForearmL,
    HandL,
    UpperArmR,
    ForearmR,
    HandR,
    ThighL,
    ShinL,
    FootL,
    ThighR,
    ShinR,
    FootR,
    Blade,
    Offhand,
    Shield,
}

impl Bone {
    pub const fn index(self) -> usize {
        self as usize
    }
}

const fn of(b: Bone) -> Option<usize> {
    Some(b as usize)
}

/// Each bone's parent (the hips are the root).
pub const PARENTS: [Option<usize>; BONE_COUNT] = {
    use Bone::*;
    [
        None,
        of(Hips),
        of(Spine),
        of(Chest),
        of(Neck),
        of(Chest),
        of(UpperArmL),
        of(ForearmL),
        of(Chest),
        of(UpperArmR),
        of(ForearmR),
        of(Hips),
        of(ThighL),
        of(ShinL),
        of(Hips),
        of(ThighR),
        of(ShinR),
        of(HandR),
        of(HandL),
        of(ForearmL),
    ]
};

/// The bone each part of the code's rig starts in, before the cuts send voxels further down
/// the limb.
const FROM_PART: [(&str, Bone); 9] = [
    ("body", Bone::Hips),
    ("leftLeg", Bone::ThighL),
    ("rightLeg", Bone::ThighR),
    ("shieldArm", Bone::UpperArmL),
    ("swordArm", Bone::UpperArmR),
    ("head", Bone::Head),
    ("blade", Bone::Blade),
    ("offhand", Bone::Offhand),
    ("shield", Bone::Shield),
];

/// A cut: every voxel of `from` whose place in the sculpt's grid lies in the box `lo..=hi`
/// (voxel centres; unbounded sides are infinite) goes to `to`.
#[derive(Clone, Copy, Debug)]
pub struct Cut {
    pub from: Bone,
    pub to: Bone,
    pub lo: [f32; 3],
    pub hi: [f32; 3],
}

const INF: f32 = f32::INFINITY;

/// The shade mode of a skin voxel in a rig part (`voxels::make_part`: 5 and the material).
const SKIN_SHADE: f32 = 5.0 + (crate::render::characters::mat::SKIN >> 24) as f32;

impl Cut {
    /// Below the plane `y` (a joint across the limb).
    const fn below(from: Bone, to: Bone, y: f32) -> Self {
        Self { from, to, lo: [-INF, -INF, -INF], hi: [INF, y, INF] }
    }

    /// Above the plane `y`.
    const fn above(from: Bone, to: Bone, y: f32) -> Self {
        Self { from, to, lo: [-INF, y, -INF], hi: [INF, INF, INF] }
    }

    fn holds(&self, g: [f32; 3]) -> bool {
        (0..3).all(|k| g[k] >= self.lo[k] && g[k] <= self.hi[k])
    }
}

/// A hero's joints, measured on his sculpt: where each bone turns, in voxels of his own grid as
/// he is sculpted (x as in the sculpt, before his rig turns him left for right), and the cuts
/// that share his voxels out among the bones, applied in order.
pub struct Joints {
    pub at: [[f32; 3]; BONE_COUNT],
    pub cuts: &'static [Cut],
}

/// Ragnar's joints (his grid is 60 × 92 × 36, `heroes::RAGNAR_SIZE`). His waist is the top of
/// his belt (y 35), his chest from under the mantle (45); his neck is the column of skin under his
/// head (y 56 to 60), his beard and braids go with his head. His sleeves bunch at the elbow (44.5),
/// the bracers end at the wrist (34.5); the fur cuffs of his boots ride on the shin (the knee at
/// 14.5) and the foot is the boot below the ankle (4.5) and its toe. The shield arm and the legs'
/// left are at the sculpt's low x (his own left once the rig mirrors him).
pub const RAGNAR: Joints = {
    use Bone::*;
    let (l_arm, r_arm, l_leg, r_leg) = (7.5, 51.5, 21.5, 37.5);
    Joints {
        at: [
            [29.5, 30.0, 17.5],
            [29.5, 35.5, 17.0],
            [29.5, 45.5, 16.5],
            [29.5, 55.5, 16.5],
            [29.5, 60.5, 16.0],
            [10.5, 54.0, 16.5],
            [l_arm, 44.5, 17.0],
            [l_arm, 34.5, 17.2],
            [48.5, 54.0, 16.5],
            [r_arm, 44.5, 17.0],
            [r_arm, 34.5, 17.2],
            [l_leg, 28.0, 19.5],
            [l_leg, 14.5, 19.3],
            [l_leg, 4.5, 19.0],
            [r_leg, 28.0, 19.5],
            [r_leg, 14.5, 19.3],
            [r_leg, 4.5, 19.0],
            // The grip in each fist, and the middle of the shield forearm where the shield is strapped.
            [r_arm, 29.8, 17.4],
            [l_arm, 29.8, 17.4],
            [l_arm, 40.0, 17.0],
        ],
        cuts: &[
            Cut::above(Hips, Spine, 35.5),
            Cut::above(Spine, Chest, 45.5),
            // The neck: the column of skin under the head, not the beard before it.
            Cut { from: Head, to: Neck, lo: [23.5, -INF, 10.5], hi: [35.5, 60.5, 20.5] },
            Cut::below(UpperArmL, ForearmL, 44.5),
            Cut::below(ForearmL, HandL, 34.5),
            Cut::below(UpperArmR, ForearmR, 44.5),
            Cut::below(ForearmR, HandR, 34.5),
            Cut::below(ThighL, ShinL, 14.5),
            Cut::below(ShinL, FootL, 4.5),
            // The toe of the boot, reaching out before the shaft above the ankle.
            Cut { from: ShinL, to: FootL, lo: [-INF, -INF, 24.5], hi: [INF, 8.5, INF] },
            Cut::below(ThighR, ShinR, 14.5),
            Cut::below(ShinR, FootR, 4.5),
            Cut { from: ShinR, to: FootR, lo: [-INF, -INF, 24.5], hi: [INF, 8.5, INF] },
        ],
    }
};

/// A hero's joint table, if he has one yet.
pub fn joints(hero: HeroKind) -> Option<&'static Joints> {
    match hero {
        HeroKind::Ragnar => Some(&RAGNAR),
        HeroKind::Mehmed | HeroKind::Miyamoto => None,
    }
}

/// Where a joint of the sculpt's grid lies in the model (as `voxels::make_part` places a pivot,
/// the rig turned left for right as `forge::hero_rigs` turns it).
pub fn joint_in_model(g: [f32; 3], width: i32, depth: i32, size: f32) -> Vec3 {
    let (ox, oz) = ((width - 1) as f32 / 2.0, (depth - 1) as f32 / 2.0);
    Vec3::new((ox - g[0]) * size, g[1] * size, (g[2] - oz) * size)
}

/// Which bone each voxel of `rig` goes to, part by part (the cut, without building anything).
pub fn assign(rig: &Rig, joints: &Joints, width: i32, depth: i32) -> Vec<Vec<usize>> {
    let size = rig.size;
    let (ox, oz) = ((width - 1) as f32 / 2.0, (depth - 1) as f32 / 2.0);
    rig.parts
        .iter()
        .map(|part| {
            let start = FROM_PART.iter().find(|(n, _)| *n == part.name).map(|(_, b)| *b).unwrap_or(Bone::Hips);
            // A held weapon is modelled with the fist round its grip: the fist stays with the hand,
            // so a turn of the weapon in it turns the weapon alone.
            let fist = match start {
                Bone::Blade => Some(Bone::HandR),
                Bone::Offhand => Some(Bone::HandL),
                _ => None,
            };
            part.cubes
                .iter()
                .enumerate()
                .map(|(j, (off, _))| {
                    if let (Some(hand), true) = (fist, part.shade.get(j).copied() == Some(SKIN_SHADE)) {
                        return hand.index();
                    }
                    let at = part.rest + *off;
                    let g = [ox - at.x / size, at.y / size - 0.5, at.z / size + oz];
                    let mut bone = start;
                    for cut in joints.cuts {
                        if cut.from == bone && cut.holds(g) {
                            bone = cut.to;
                        }
                    }
                    bone.index()
                })
                .collect()
        })
        .collect()
}

/**
 * The hero's rig cut into the skeleton's bones: each voxel of `rig` (the code's rig, its six parts
 * and its layered weapons and shield) goes to its bone by `joints`, keeping its place, colour and
 * material; each bone's joint is where the table puts it, and its parent is the skeleton's. The
 * parts come in the order of `BONES`. `width` and `depth` are the sculpt's grid, `rig.size` its
 * voxel.
 */
pub fn jointed(rig: &Rig, joints: &Joints, width: i32, depth: i32) -> Rig {
    let size = rig.size;
    let rest: Vec<Vec3> = joints.at.iter().map(|&g| joint_in_model(g, width, depth, size)).collect();
    let mut cubes: Vec<Vec<(Vec3, Vec3)>> = vec![Vec::new(); BONE_COUNT];
    let mut shade: Vec<Vec<f32>> = vec![Vec::new(); BONE_COUNT];
    for (part, bones) in rig.parts.iter().zip(assign(rig, joints, width, depth)) {
        for (j, ((off, c), bone)) in part.cubes.iter().zip(bones).enumerate() {
            cubes[bone].push((part.rest + *off - rest[bone], *c));
            shade[bone].push(part.shade.get(j).copied().unwrap_or(0.0));
        }
    }
    let parts = BONES
        .iter()
        .enumerate()
        .zip(cubes.into_iter().zip(shade))
        .map(|((i, &name), (cubes, shade))| {
            let min_y = cubes.iter().map(|(p, _)| p.y - size / 2.0).fold(f32::INFINITY, f32::min);
            RigPart { name, rest: rest[i], cubes, shade, min_y: if min_y.is_finite() { min_y } else { 0.0 }, parent: PARENTS[i] }
        })
        .collect();
    Rig::new(size, parts)
}

/// A colour for each bone, for the debug capture that paints every voxel its bone's colour
/// (`VH_ANIM=bones`): neighbours down a limb differ, the two sides differ.
pub fn bone_color(i: usize) -> Vec3 {
    const COLORS: [u32; BONE_COUNT] = [
        0xffffff, 0xffd400, 0xff7a00, 0x8a5a2a, 0xff2a8a, 0x2a6aff, 0x2affd4, 0x9a2aff, 0xff2a2a, 0xffb0b0, 0x7a0000, 0x2aff2a, 0x007a2a,
        0xd4ff8a, 0x00d4ff, 0x005a8a, 0xb0e8ff, 0x404040, 0xa0a0a0, 0x5a2a00,
    ];
    crate::render::voxels::lin(COLORS[i % BONE_COUNT])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::voxels::{build_layered_rig, build_rig};

    /// Ragnar's code rig as `forge::hero_rigs` builds it for the play (mirrored), from his bare sculpt.
    fn ragnar_rig() -> (Rig, i32, i32) {
        let body = crate::render::heroes::ragnar_hd(None);
        let (w, d) = (body.width, body.depth);
        let mut c = body;
        for v in c.voxels.iter_mut() {
            v.x = w - 1 - v.x;
        }
        let defs: Vec<_> = crate::render::heroes::RAGNAR_HD_RIG
            .iter()
            .map(|p| crate::render::voxels::PartDef {
                name: p.name,
                boxes: p.boxes,
                pivot: [(w - 1) as f32 - p.pivot[0], p.pivot[1], p.pivot[2]],
            })
            .collect();
        let empty = crate::render::voxels::Collected { voxels: Vec::new(), width: w, depth: d, height: c.height };
        let rig = build_layered_rig(
            &c,
            &defs,
            &[("blade", &empty, [0.0; 3]), ("offhand", &empty, [0.0; 3]), ("shield", &empty, [0.0; 3])],
            0.0125,
        );
        (rig, w, d)
    }

    #[test]
    fn the_skeleton_is_ordered_parents_first() {
        for (i, p) in PARENTS.iter().enumerate() {
            assert!(p.is_none_or(|p| p < i), "{} hangs from a later bone", BONES[i]);
        }
        assert_eq!(PARENTS.iter().filter(|p| p.is_none()).count(), 1, "one root");
        assert_eq!(BONES[Bone::Shield.index()], "shield");
    }

    #[test]
    fn the_cut_keeps_every_voxel_once() {
        let (rig, w, d) = ragnar_rig();
        let cut = jointed(&rig, &RAGNAR, w, d);
        assert_eq!(cut.parts.len(), BONE_COUNT);
        let count = |r: &Rig| r.parts.iter().map(|p| p.cubes.len()).sum::<usize>();
        assert_eq!(count(&cut), count(&rig));
        // Every voxel stays where it was, as the code rig's rest pose has it.
        let mut before: Vec<(i64, i64, i64)> =
            rig.parts.iter().flat_map(|p| p.cubes.iter().map(move |(o, _)| p.rest + *o)).map(key).collect();
        let mut after: Vec<(i64, i64, i64)> =
            cut.parts.iter().flat_map(|p| p.cubes.iter().map(move |(o, _)| p.rest + *o)).map(key).collect();
        before.sort_unstable();
        after.sort_unstable();
        assert_eq!(before, after);
        // Every bone of his body has voxels (the weapons are empty here).
        for (i, p) in cut.parts.iter().enumerate().take(Bone::Blade.index()) {
            assert!(p.cubes.len() > 20, "{} has {} voxels", BONES[i], p.cubes.len());
            assert_eq!(p.parent, PARENTS[i]);
        }
    }

    fn key(p: Vec3) -> (i64, i64, i64) {
        let k = (p / 0.0125 * 2.0).round();
        (k.x as i64, k.y as i64, k.z as i64)
    }

    #[test]
    fn no_voxel_is_stranded_on_another_limb() {
        // Each voxel lies on its own bone's side of the joint below it, and within reach of its joint.
        let (rig, w, d) = ragnar_rig();
        let cut = jointed(&rig, &RAGNAR, w, d);
        let size = cut.size;
        for (i, part) in cut.parts.iter().enumerate() {
            for (off, _) in &part.cubes {
                // Nothing on a limb lies further than its length from its joint.
                let reach = match i {
                    0..=2 => 40.0,
                    4 => 45.0,
                    _ => 16.0,
                } * size;
                assert!(off.length() <= reach, "{} has a voxel {:.3} from its joint", BONES[i], off.length());
            }
        }
        // Down each limb, a bone's voxels lie above the next bone's joint.
        for (a, b) in [(5, 6), (6, 7), (8, 9), (9, 10), (11, 12), (12, 13), (14, 15), (15, 16)] {
            let (up, down) = (&cut.parts[a], &cut.parts[b]);
            let joint = down.rest.y - up.rest.y;
            assert!(up.cubes.iter().all(|(o, _)| o.y >= joint - size * 0.6), "{} has a voxel below {}", BONES[a], BONES[b]);
        }
        // The sides keep to their own side of him: the left limbs at +x, the right at -x.
        for (name, sign) in [("hand.L", 1.0), ("hand.R", -1.0), ("foot.L", 1.0), ("foot.R", -1.0)] {
            let p = &cut.parts[cut.index(name).unwrap()];
            assert!(p.cubes.iter().all(|(o, _)| (p.rest.x + o.x) * sign > 0.0), "{name} reaches across");
        }
    }

    #[test]
    fn only_ragnar_has_a_table_yet() {
        assert!(joints(HeroKind::Ragnar).is_some());
        let rig = build_rig(&crate::render::characters::ragnar(), crate::render::characters::RAGNAR_RIG, 0.05);
        assert!(rig.parts.iter().all(|p| p.parent.is_none()), "the code's rigs have no parents");
    }
}
