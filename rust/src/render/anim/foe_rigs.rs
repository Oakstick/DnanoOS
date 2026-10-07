//! The foes' skeletons and joint tables (ANIMATION.md §11): the humanoid foes on the heroes' twenty
//! bones (so Mixamo's motion retargets onto them as onto the heroes), the others on skeletons of
//! their own (bats, wolves, the cobra, the slime, the minion, the jorogumo). Each table is measured
//! on its sculpt, as the heroes' are, and cuts the foe's rig (its parts as the code swings them)
//! into its bones; the sculpt is finished solid for it (`characters::solid`), so a joint that opens
//! shows bone and cloth, not a hollow shell. At rest the jointed foe is the code's, voxel for voxel.
//!
//! The foes are not mirrored as the heroes are (`forge::hero_rigs`): each holds its weapon in the
//! hand its sculpt gives it, which, as they face the hero, is their own left (the sculpt's `rightArm`
//! is at +x). So their bones are named by the side they are on (`.L` at +x, as the heroes'), and the
//! weapon hangs from `hand.L` as the `offhand`: the Mixamo clips are mirrored for them
//! (`tools/blender/mixamo_foe.py`), left-handed swordsmen of the dead.

use crate::logic::enemy::EnemyKind;
use crate::render::voxels::{build_rig, Rig};

use super::skeleton::{Bone, Cut, Joints, SkeletonDef, HUMANOID};

const INF: f32 = f32::INFINITY;

/// A foe's body as its clips know it: one per sculpt (the vampire is the night raven's sculpt at
/// another size, so it has clips of its own).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum FoeRig {
    Draugr,
}

impl FoeRig {
    pub const COUNT: usize = 1;
    /// Every foe rig with a joint table.
    pub const ALL: [FoeRig; Self::COUNT] = [FoeRig::Draugr];

    /// The folder of its clips (`assets/anim/<dir>/`) and its name for `--export-rig`.
    pub fn dir(self) -> &'static str {
        match self {
            FoeRig::Draugr => "draugr",
        }
    }

    /// The rig a foe of `kind` is animated with by clips, if it has one. (The Peerless wears the
    /// helmeted skeleton's rig as a stand-in only: left to the code.)
    pub fn of(kind: EnemyKind) -> Option<FoeRig> {
        match kind {
            EnemyKind::Skeleton => Some(FoeRig::Draugr),
            _ => None,
        }
    }

    pub fn def(self) -> &'static SkeletonDef {
        match self {
            FoeRig::Draugr => &HUMANOID,
        }
    }

    pub fn joints(self) -> &'static Joints {
        match self {
            FoeRig::Draugr => &DRAUGR,
        }
    }

    /// Its sculpt's grid (width, height, depth) and voxel.
    pub fn grid(self) -> ((i32, i32, i32), f32) {
        match self {
            FoeRig::Draugr => (crate::render::foes::DRAUGR_SIZE, 0.0125),
        }
    }

    /// The code's rig, from the sculpt finished solid.
    pub fn code_rig(self) -> Rig {
        use crate::render::characters::solid;
        let (_, size) = self.grid();
        match self {
            FoeRig::Draugr => solid(|| build_rig(&crate::render::foes::draugr(), crate::render::foes::DRAUGR_RIG, size)),
        }
    }

    /// The foe cut into its skeleton's bones. With `VH_ANIM=bones` every voxel is painted its
    /// bone's colour.
    pub fn jointed(self) -> Rig {
        use super::{library::Mode, skeleton};
        let ((w, _, d), _) = self.grid();
        let mut cut = skeleton::jointed(&self.code_rig(), self.joints(), w, d);
        if Mode::from_env() == Mode::Bones {
            for (i, part) in cut.parts.iter_mut().enumerate() {
                let c = skeleton::bone_color(i);
                for (k, (_, color)) in part.cubes.iter_mut().enumerate() {
                    *color = c * (0.7 + 0.3 * (k % 3) as f32 / 2.0);
                }
                part.shade.iter_mut().for_each(|m| *m = 0.0);
            }
        }
        cut
    }

    /// The way the weapon in its fist points at rest (the model's axes), for the retarget's grip
    /// (`tools/blender/mixamo_foe.py` reads the same from the rig).
    pub fn weapon_rest(self) -> Option<glam::Vec3> {
        match self {
            // The notched sword hangs point down from the bony fist, leaning forward a little.
            FoeRig::Draugr => Some(glam::Vec3::new(0.0, -0.975, 0.22).normalize()),
        }
    }
}

const fn b(bone: Bone) -> usize {
    bone as usize
}

/// The code rig's parts as the humanoid foes are sculpted: `leftArm` and `leftLeg` at the sculpt's
/// low x, which (unmirrored) is the foe's own right.
const HUMANOID_FOE_PARTS: &[(&str, usize)] = &[
    ("body", b(Bone::Hips)),
    ("leftLeg", b(Bone::ThighR)),
    ("rightLeg", b(Bone::ThighL)),
    ("leftArm", b(Bone::UpperArmR)),
    ("rightArm", b(Bone::UpperArmL)),
    ("head", b(Bone::Head)),
];

/// What a bony fist is made of (`characters::mat::IVORY`): the cut of a weapon out of the fist round
/// its grip takes everything else.
const BONE_MATS: &[u32] = &[crate::render::characters::mat::IVORY];

/**
 * The draugr's joints (grid 52 × 84 × 28, `foes::DRAUGR_SIZE`). His waist is the top of his belt
 * (y 35.5), his chest from the middle of his ribs (46); his neck is the column of vertebrae under
 * his skull (y 56 to 61), the skull, the helmet and its horns his head. His mail sleeves end above
 * the elbow (the bone ball at 43.2), the forearm's two bones run to the wrist (33.5), the hand is the
 * fist and its bony fingers. The thigh bones end at the knee's ball (17.5), the shins at the ankle
 * (4.5), the feet are the rotten shoe on his right (at +x, his own left) and the bare toes.
 *
 * The notched sword in his fist (at the sculpt's high x: his own left) is its own bone, the
 * `offhand`, its joint in the fist: everything of that arm below the elbow that is not bone (the
 * leather grip, the iron pommel, the rusted guard and the blade), so the clips can turn it in his
 * fist as a held sword turns.
 */
pub const DRAUGR: Joints = {
    use Bone::*;
    let (l_arm, r_arm, l_leg, r_leg, mid) = (42.5, 8.5, 33.5, 17.5, 25.5);
    Joints {
        def: &HUMANOID,
        from: HUMANOID_FOE_PARTS,
        mirrored: false,
        fists: false,
        at: &[
            [mid, 30.0, 12.5],
            [mid, 35.5, 12.0],
            [mid, 46.0, 12.0],
            [mid, 56.5, 12.0],
            [mid, 61.5, 13.0],
            [l_arm, 54.0, 13.5],
            [l_arm + 0.6, 44.0, 14.0],
            [l_arm, 33.5, 15.4],
            [r_arm, 54.0, 13.5],
            [r_arm - 0.6, 44.0, 14.0],
            [r_arm, 33.5, 15.4],
            [l_leg, 27.5, 13.0],
            [l_leg, 17.5, 13.2],
            [l_leg, 4.5, 12.6],
            [r_leg, 27.5, 13.0],
            [r_leg, 17.5, 13.2],
            [r_leg, 4.5, 12.6],
            // The claw's grip (empty: his right hand holds nothing), the sword's grip in his left
            // fist, and the middle of his right forearm (no shield).
            [r_arm, 30.8, 16.6],
            [l_arm, 30.8, 16.6],
            [r_arm, 39.0, 14.6],
        ],
        cuts: &[
            Cut::up(b(Hips), b(Spine), 35.5),
            Cut::up(b(Spine), b(Chest), 46.0),
            // The neck: the vertebrae under the skull, not the jaw and cheeks before them.
            Cut::cube(b(Head), b(Neck), [mid - 3.0, -INF, 9.0], [mid + 3.0, 61.5, 15.5]),
            // The sword out of his fist first (its grip runs up through it to the pommel above).
            Cut::cube(b(UpperArmL), b(Offhand), [l_arm - 4.5, -INF, -INF], [l_arm + 4.5, 38.5, INF]).except(BONE_MATS),
            Cut::down(b(UpperArmL), b(ForearmL), 44.0),
            Cut::down(b(ForearmL), b(HandL), 33.5),
            Cut::down(b(UpperArmR), b(ForearmR), 44.0),
            Cut::down(b(ForearmR), b(HandR), 33.5),
            Cut::down(b(ThighL), b(ShinL), 17.5),
            Cut::down(b(ShinL), b(FootL), 4.5),
            // The shoe's toe, out before the ankle.
            Cut::cube(b(ShinL), b(FootL), [-INF, -INF, 15.5], [INF, 5.5, INF]),
            Cut::down(b(ThighR), b(ShinR), 17.5),
            Cut::down(b(ShinR), b(FootR), 4.5),
            Cut::cube(b(ShinR), b(FootR), [-INF, -INF, 15.5], [INF, 5.5, INF]),
        ],
    }
};

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn key(p: Vec3, size: f32) -> (i64, i64, i64) {
        let k = (p / size * 2.0).round();
        (k.x as i64, k.y as i64, k.z as i64)
    }

    #[test]
    fn every_foe_rig_is_cut_whole_and_in_place() {
        for foe in FoeRig::ALL {
            let code = foe.code_rig();
            let cut = foe.jointed();
            let def = foe.def();
            assert_eq!(cut.parts.len(), def.len(), "{foe:?}");
            let size = code.size;
            // Every voxel of the code's rig is where it was, in some bone; the bones add only
            // sleeves (copies laid where a voxel already is).
            let all = |r: &Rig| {
                let mut v: Vec<_> = r.parts.iter().flat_map(|p| p.cubes.iter().map(move |(o, _)| key(p.rest + *o, size))).collect();
                v.sort_unstable();
                v.dedup();
                v
            };
            assert_eq!(all(&code), all(&cut), "{foe:?}");
            let n = |r: &Rig| r.parts.iter().map(|p| p.cubes.len()).sum::<usize>();
            assert!(n(&cut) >= n(&code) && n(&cut) < n(&code) * 5 / 4, "{foe:?}: {} against {}", n(&cut), n(&code));
            for (i, p) in cut.parts.iter().enumerate() {
                assert_eq!(p.parent, def.parents[i], "{foe:?} {}", def.bones[i]);
            }
        }
    }

    /// The moves' clips each foe rig must have, by its archetype's moves (and the pack leader's rally).
    fn moves_of(foe: FoeRig) -> &'static [&'static str] {
        match foe {
            FoeRig::Draugr => &["slam", "sweep", "rally"],
        }
    }

    #[test]
    fn every_foes_clips_read_carry_their_markers_and_rest_on_its_joints() {
        for foe in FoeRig::ALL {
            let c = crate::render::anim::library::foe_clips(foe).unwrap_or_else(|| panic!("{foe:?}'s clips are embedded"));
            assert!(c.errors.is_empty(), "{foe:?}: {:?}", c.errors);
            assert!(c.clips.iter().any(|k| super::super::foe::IDLE.contains(&k.name.as_str())), "{foe:?} has no idle");
            assert!(c.clips.iter().any(|k| super::super::foe::GAIT.contains(&k.name.as_str())), "{foe:?} has no gait");
            for k in &c.clips {
                assert_eq!(k.bones(), foe.def().len(), "{foe:?} {}", k.name);
                assert!(k.frame_count() > 8, "{foe:?} {}", k.name);
            }
            for name in moves_of(foe).iter().copied().chain(["hit"]) {
                assert!(c.get(name).is_some(), "{foe:?} has no {name}");
            }
            for name in moves_of(foe) {
                let k = c.get(name).unwrap();
                let m = |n: &str| k.mark(n).unwrap_or_else(|| panic!("{foe:?} {name} has no {n}"));
                assert!(
                    0.0 < m("windup_end") && m("windup_end") < m("active_end") && m("active_end") <= m("recover_end"),
                    "{foe:?} {name}"
                );
                assert!((m("recover_end") - k.duration()).abs() < 1e-4, "{foe:?} {name} ends at its last frame");
            }
            for cycle in ["idle", "walk"] {
                if let Some(k) = c.get(cycle) {
                    assert!(k.markers.looping, "{foe:?}: {cycle} loops");
                }
            }
            // Out to Blender and back: every joint is where the game cut it.
            let rig = foe.jointed();
            for k in &c.clips {
                for (b, part) in rig.parts.iter().enumerate() {
                    let at = k.skeleton.rest_inv[b].inverse().w_axis.truncate();
                    assert!(at.abs_diff_eq(part.rest, 1e-4), "{foe:?} {}: {} at {at} not {}", k.name, part.name, part.rest);
                }
            }
        }
    }

    #[test]
    fn the_draugr_keeps_his_limbs_on_their_sides() {
        let cut = FoeRig::Draugr.jointed();
        let size = cut.size;
        // His sword's bone holds the sword: long, reaching far below his fist; his fist holds no iron.
        let sword = &cut.parts[Bone::Offhand.index()];
        assert!(sword.cubes.len() > 150, "{} voxels of sword", sword.cubes.len());
        assert!(sword.cubes.iter().any(|(o, _)| o.y < -24.0 * size), "the blade reaches down");
        assert!(cut.parts[Bone::Blade.index()].cubes.is_empty(), "his right hand holds nothing");
        // Every bone of his body has voxels.
        for (i, p) in cut.parts.iter().enumerate().take(Bone::Blade.index()) {
            assert!(p.cubes.len() > 20, "{} has {}", HUMANOID.bones[i], p.cubes.len());
        }
        // The left limbs at +x, the right at -x (as the heroes'), and nothing far from its joint.
        for (name, sign) in [("hand.L", 1.0), ("hand.R", -1.0), ("foot.L", 1.0), ("foot.R", -1.0), ("thigh.L", 1.0), ("thigh.R", -1.0)] {
            let p = &cut.parts[cut.index(name).unwrap()];
            assert!(p.cubes.iter().all(|(o, _)| (p.rest.x + o.x) * sign > 0.0), "{name} reaches across");
        }
        for (a, bb) in [(5, 6), (6, 7), (8, 9), (9, 10), (11, 12), (12, 13), (14, 15), (15, 16)] {
            let (up, down) = (&cut.parts[a], &cut.parts[bb]);
            let joint = down.rest.y - up.rest.y;
            assert!(
                up.cubes.iter().all(|(o, _)| o.y >= joint - size * 0.6),
                "{} has a voxel below {}",
                HUMANOID.bones[a],
                HUMANOID.bones[bb]
            );
        }
    }
}
