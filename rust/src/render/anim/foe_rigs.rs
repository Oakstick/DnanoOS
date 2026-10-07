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
    Helmeted,
    Witch,
    Blemmyes,
    Tengu,
    Rasetsu,
    Bat,
    Vampire,
    LargeBat,
    Minion,
    Werewolf,
    DireWolf,
    Cobra,
    Slime,
    Jorogumo,
    Kappa,
    Rokurokubi,
    Oni,
    Kitsune,
    Tulukus,
    Albasti,
    Cin,
    Arcura,
    Karakoncolos,
}

impl FoeRig {
    pub const COUNT: usize = 24;
    /// Every foe rig with a joint table.
    pub const ALL: [FoeRig; Self::COUNT] = [
        FoeRig::Draugr,
        FoeRig::Helmeted,
        FoeRig::Witch,
        FoeRig::Blemmyes,
        FoeRig::Tengu,
        FoeRig::Rasetsu,
        FoeRig::Bat,
        FoeRig::Vampire,
        FoeRig::LargeBat,
        FoeRig::Minion,
        FoeRig::Werewolf,
        FoeRig::DireWolf,
        FoeRig::Cobra,
        FoeRig::Slime,
        FoeRig::Jorogumo,
        FoeRig::Kappa,
        FoeRig::Rokurokubi,
        FoeRig::Oni,
        FoeRig::Kitsune,
        FoeRig::Tulukus,
        FoeRig::Albasti,
        FoeRig::Cin,
        FoeRig::Arcura,
        FoeRig::Karakoncolos,
    ];

    /// The folder of its clips (`assets/anim/<dir>/`) and its name for `--export-rig`.
    pub fn dir(self) -> &'static str {
        match self {
            FoeRig::Draugr => "draugr",
            FoeRig::Helmeted => "helmeted",
            FoeRig::Witch => "witch",
            FoeRig::Blemmyes => "blemmyes",
            FoeRig::Tengu => "tengu",
            FoeRig::Rasetsu => "rasetsu",
            FoeRig::Bat => "bat",
            FoeRig::Vampire => "vampire",
            FoeRig::LargeBat => "large_bat",
            FoeRig::Minion => "minion",
            FoeRig::Werewolf => "werewolf",
            FoeRig::DireWolf => "dire_wolf",
            FoeRig::Cobra => "cobra",
            FoeRig::Slime => "slime",
            FoeRig::Jorogumo => "jorogumo",
            FoeRig::Kappa => "kappa",
            FoeRig::Rokurokubi => "rokurokubi",
            FoeRig::Oni => "oni",
            FoeRig::Kitsune => "kitsune",
            FoeRig::Tulukus => "tulukus",
            FoeRig::Albasti => "albasti",
            FoeRig::Cin => "cin",
            FoeRig::Arcura => "arcura",
            FoeRig::Karakoncolos => "karakoncolos",
        }
    }

    /// The rig a foe of `kind` is animated with by clips, if it has one. (The Peerless wears the
    /// helmeted skeleton's rig as a stand-in only: left to the code.)
    pub fn of(kind: EnemyKind) -> Option<FoeRig> {
        Some(match kind {
            EnemyKind::Skeleton => FoeRig::Draugr,
            EnemyKind::Helmeted => FoeRig::Helmeted,
            EnemyKind::Witch => FoeRig::Witch,
            EnemyKind::Blemmyes => FoeRig::Blemmyes,
            EnemyKind::Tengu => FoeRig::Tengu,
            EnemyKind::Rasetsu => FoeRig::Rasetsu,
            EnemyKind::Bat => FoeRig::Bat,
            EnemyKind::Vampire => FoeRig::Vampire,
            EnemyKind::LargeBat => FoeRig::LargeBat,
            EnemyKind::Minion => FoeRig::Minion,
            EnemyKind::Werewolf => FoeRig::Werewolf,
            EnemyKind::DireWolf => FoeRig::DireWolf,
            EnemyKind::Cobra => FoeRig::Cobra,
            EnemyKind::Slime => FoeRig::Slime,
            EnemyKind::Jorogumo => FoeRig::Jorogumo,
            EnemyKind::Kappa => FoeRig::Kappa,
            EnemyKind::Rokurokubi => FoeRig::Rokurokubi,
            EnemyKind::Oni => FoeRig::Oni,
            EnemyKind::Kitsune => FoeRig::Kitsune,
            EnemyKind::Tulukus => FoeRig::Tulukus,
            EnemyKind::Albasti => FoeRig::Albasti,
            EnemyKind::Cin => FoeRig::Cin,
            EnemyKind::Arcura => FoeRig::Arcura,
            EnemyKind::Karakoncolos => FoeRig::Karakoncolos,
            _ => return None,
        })
    }

    pub fn def(self) -> &'static SkeletonDef {
        self.joints().def
    }

    pub fn joints(self) -> &'static Joints {
        static BUILT: [std::sync::OnceLock<&'static Joints>; FoeRig::COUNT] = [const { std::sync::OnceLock::new() }; FoeRig::COUNT];
        BUILT[self as usize].get_or_init(|| self.table())
    }

    fn table(self) -> &'static Joints {
        use super::foe_beasts as fb;
        match self {
            FoeRig::Draugr => &DRAUGR,
            FoeRig::Helmeted => &HELMETED,
            FoeRig::Witch => &WITCH,
            FoeRig::Blemmyes => &BLEMMYES,
            FoeRig::Tengu => &TENGU,
            FoeRig::Rasetsu => &RASETSU,
            FoeRig::Bat | FoeRig::Vampire => &fb::RAVEN,
            FoeRig::LargeBat => &fb::LARGE_BAT,
            FoeRig::Minion => &fb::MINION_JOINTS,
            FoeRig::Werewolf => &fb::WOLF,
            FoeRig::DireWolf => &fb::DIRE_WOLF,
            FoeRig::Cobra => &fb::COBRA,
            FoeRig::Slime => &fb::BOG_SLIME,
            FoeRig::Jorogumo => &fb::JOROGUMO,
            FoeRig::Kappa => super::foe_stage4::kappa(),
            FoeRig::Rokurokubi => super::foe_stage4::rokurokubi(),
            FoeRig::Oni => super::foe_stage4::oni(),
            FoeRig::Kitsune => super::foe_stage4::kitsune(),
            FoeRig::Tulukus => super::foe_stage5::tulukus(),
            FoeRig::Albasti => super::foe_stage5::albasti(),
            FoeRig::Cin => super::foe_stage5::cin(),
            FoeRig::Arcura => super::foe_stage5::arcura(),
            FoeRig::Karakoncolos => super::foe_stage5::karakoncolos(),
        }
    }

    /// Its sculpt's grid (width, height, depth) and voxel (as `view::models` builds its rig).
    pub fn grid(self) -> ((i32, i32, i32), f32) {
        use crate::render::{foes, foes2, foes3};
        match self {
            FoeRig::Draugr => (foes::DRAUGR_SIZE, 0.0125),
            FoeRig::Helmeted => (foes2::HELMET_SIZE, 0.0125),
            FoeRig::Witch => (foes2::WITCH_SIZE, 0.0125),
            FoeRig::Blemmyes => (foes2::BLEMMYES_SIZE, 0.0145),
            FoeRig::Tengu => (foes3::TENGU_SIZE, 0.0125),
            FoeRig::Rasetsu => (foes3::RASETSU_SIZE, 0.0125),
            FoeRig::Bat => (foes::RAVEN_SIZE, 0.0105),
            FoeRig::Vampire => (foes::RAVEN_SIZE, 0.0125),
            FoeRig::LargeBat => (foes2::BAT_SIZE, 0.011),
            FoeRig::Minion => (foes2::MINION_SIZE, 0.014),
            FoeRig::Werewolf => (foes2::WOLF_SIZE, 0.0135),
            FoeRig::DireWolf => (foes3::DIRE_SIZE, 0.0125),
            FoeRig::Cobra => (foes3::COBRA_SIZE, 0.0125),
            FoeRig::Slime => (foes::SLIME_SIZE, 0.01875),
            FoeRig::Jorogumo => (foes3::JORO_SIZE, 0.0125),
            FoeRig::Kappa => (crate::render::foes4::KAPPA_SIZE, 0.0125),
            FoeRig::Rokurokubi => (crate::render::foes4::ROKURO_SIZE, 0.0125),
            FoeRig::Oni => (crate::render::foes4::ONI_SIZE, 0.0135),
            FoeRig::Kitsune => (crate::render::foes4::KITSUNE_SIZE, 0.0125),
            FoeRig::Tulukus => (crate::render::foes5::TULUKUS_SIZE, 0.0125),
            FoeRig::Albasti => (crate::render::foes5::ALBASTI_SIZE, 0.0125),
            FoeRig::Cin => (crate::render::foes5::CIN_SIZE, 0.0125),
            FoeRig::Arcura => (crate::render::foes5::ARCURA_SIZE, 0.0125),
            FoeRig::Karakoncolos => (crate::render::foes5::KARAKONCOLOS_SIZE, 0.013),
        }
    }

    /// The code's rig, from the sculpt finished solid.
    pub fn code_rig(self) -> Rig {
        use crate::render::characters::solid;
        use crate::render::{foes, foes2, foes3};
        let (_, size) = self.grid();
        solid(|| match self {
            FoeRig::Draugr => build_rig(&foes::draugr(), foes::DRAUGR_RIG, size),
            FoeRig::Helmeted => build_rig(&foes2::helmeted_skeleton(), foes2::HELMET_RIG, size),
            FoeRig::Witch => build_rig(&foes2::witch(), foes2::WITCH_RIG, size),
            FoeRig::Blemmyes => build_rig(&foes2::blemmyes(), foes2::BLEMMYES_RIG, size),
            FoeRig::Tengu => build_rig(&foes3::tengu(), foes3::TENGU_RIG, size),
            FoeRig::Rasetsu => build_rig(&foes3::rasetsu(), foes3::RASETSU_RIG, size),
            FoeRig::Bat => build_rig(&foes::raven(false), foes::RAVEN_RIG, size),
            FoeRig::Vampire => build_rig(&foes::raven(true), foes::RAVEN_RIG, size),
            FoeRig::LargeBat => build_rig(&foes2::large_bat(), foes2::BAT_RIG, size),
            FoeRig::Minion => build_rig(&foes2::minion(), &[], size),
            FoeRig::Werewolf => build_rig(&foes2::loup_garou(), foes2::WOLF_RIG, size),
            FoeRig::DireWolf => build_rig(&foes3::dire_wolf(), foes3::DIRE_RIG, size),
            FoeRig::Cobra => build_rig(&foes3::cobra(), foes3::COBRA_RIG, size),
            FoeRig::Slime => build_rig(&foes::bog_slime(), &[], size),
            FoeRig::Jorogumo => build_rig(&foes3::jorogumo(), foes3::JORO_RIG, size),
            FoeRig::Kappa => build_rig(&crate::render::foes4::kappa(), crate::render::foes4::KAPPA_RIG, size),
            FoeRig::Rokurokubi => build_rig(&crate::render::foes4::rokurokubi(), crate::render::foes4::ROKURO_RIG, size),
            FoeRig::Oni => build_rig(&crate::render::foes4::oni(), crate::render::foes4::ONI_RIG, size),
            FoeRig::Kitsune => build_rig(&crate::render::foes4::kitsune(), crate::render::foes4::KITSUNE_RIG, size),
            FoeRig::Tulukus => build_rig(&crate::render::foes5::tulukus(), crate::render::foes5::TULUKUS_RIG, size),
            FoeRig::Albasti => build_rig(&crate::render::foes5::albasti(), crate::render::foes5::ALBASTI_RIG, size),
            FoeRig::Cin => build_rig(&crate::render::foes5::cin(), crate::render::foes5::CIN_RIG, size),
            FoeRig::Arcura => build_rig(&crate::render::foes5::arcura(), crate::render::foes5::ARCURA_RIG, size),
            FoeRig::Karakoncolos => build_rig(&crate::render::foes5::karakoncolos(), crate::render::foes5::KARAKONCOLOS_RIG, size),
        })
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

/// Skin (`characters::mat::SKIN`): a fist round a grip, kept by the hand when the weapon in it is
/// cut out.
const SKIN_MATS: &[u32] = &[crate::render::characters::mat::SKIN];

/**
 * The helmeted skeleton's joints (grid 40 × 80 × 22, `foes2::HELMET_SIZE`). His waist is above the
 * pelvis (y 34), his chest from the middle of his ribs (45); his neck the vertebrae between his
 * collarbones and his skull (y 55 to 58), the skull, the jaw and the helmet his head. The elbow is
 * its bone ball (42.4), the wrist the top of the bony fist (33.3); the knee its ball (16), the ankle
 * the top of the foot (4.3). The spiked mace in his fist at the sculpt's high x (his own left) is the
 * `offhand`: its iron haft and head, not the bone round it.
 */
pub const HELMETED: Joints = {
    use Bone::*;
    let (l_arm, r_arm, l_leg, r_leg, mid) = (31.5, 7.5, 23.5, 15.5, 19.5);
    Joints {
        def: &HUMANOID,
        from: HUMANOID_FOE_PARTS,
        mirrored: false,
        fists: false,
        at: &[
            [mid, 29.6, 10.5],
            [mid, 34.0, 9.0],
            [mid, 45.0, 9.0],
            [mid, 55.0, 11.0],
            [mid, 58.0, 11.0],
            [l_arm, 54.0, 11.0],
            [l_arm + 0.6, 42.4, 12.0],
            [l_arm, 33.3, 13.6],
            [r_arm, 54.0, 11.0],
            [r_arm - 0.6, 42.4, 12.0],
            [r_arm, 33.3, 13.6],
            [l_leg, 28.0, 11.0],
            [l_leg, 16.0, 11.2],
            [l_leg, 4.3, 10.8],
            [r_leg, 28.0, 11.0],
            [r_leg, 16.0, 11.2],
            [r_leg, 4.3, 10.8],
            [r_arm, 31.8, 15.0],
            [l_arm, 31.8, 15.0],
            [r_arm, 38.0, 12.8],
        ],
        cuts: &[
            Cut::up(b(Hips), b(Spine), 34.0),
            Cut::up(b(Spine), b(Chest), 45.0),
            Cut::cube(b(Chest), b(Neck), [mid - 2.5, 54.5, -INF], [mid + 2.5, INF, INF]),
            Cut::cube(b(UpperArmL), b(Offhand), [l_arm - 6.0, -INF, -INF], [l_arm + 6.0, 36.0, INF]).except(BONE_MATS),
            Cut::down(b(UpperArmL), b(ForearmL), 44.0),
            Cut::down(b(ForearmL), b(HandL), 33.3),
            Cut::down(b(UpperArmR), b(ForearmR), 44.0),
            Cut::down(b(ForearmR), b(HandR), 33.3),
            Cut::down(b(ThighL), b(ShinL), 17.0),
            Cut::down(b(ShinL), b(FootL), 4.3),
            Cut::down(b(ThighR), b(ShinR), 17.0),
            Cut::down(b(ShinR), b(FootR), 4.3),
        ],
    }
};

/**
 * The witch's joints (grid 40 × 96 × 26, `foes2::WITCH_SIZE`). Her waist is the belt (y 42.5), her
 * chest from 50, her neck the column of skin under her chin (y 57.5 up), her head the face, the hair
 * and the hat. Her legs are short under her long robe: their joints are set where a woman's hips are
 * (y 34), not at the code's pivots (22), so her stride moves the robe from there; the robe below her
 * belt is cut down the middle (y 36.5, x 19.5), each half riding its thigh as Mehmed's skirt does,
 * and under it only her boots and stockings are sculpted, so the knee (13) is cut unsleeved. Her wide
 * sleeves bend at the elbow (46); her hand is the green skin inside the cuff. Her staff, in her fist
 * at the sculpt's high x (her own left), is the `offhand`: its wood and its stone.
 */
pub const WITCH: Joints = {
    use crate::render::characters::mat;
    use Bone::*;
    let (l_arm, r_arm, l_leg, r_leg, mid) = (26.5, 12.5, 23.5, 15.5, 19.5);
    Joints {
        def: &HUMANOID,
        from: HUMANOID_FOE_PARTS,
        mirrored: false,
        fists: false,
        at: &[
            [mid, 38.0, 13.0],
            [mid, 42.5, 13.0],
            [mid, 50.0, 13.0],
            [mid, 57.5, 13.0],
            [mid, 61.0, 13.0],
            [l_arm, 56.0, 13.0],
            [l_arm + 1.5, 46.0, 15.0],
            [l_arm + 1.2, 40.5, 18.0],
            [r_arm, 56.0, 13.0],
            [r_arm - 1.5, 46.0, 15.0],
            [r_arm - 1.2, 40.5, 18.0],
            [l_leg, 34.0, 13.0],
            [l_leg, 13.0, 13.0],
            [l_leg, 5.0, 13.5],
            [r_leg, 34.0, 13.0],
            [r_leg, 13.0, 13.0],
            [r_leg, 5.0, 13.5],
            [r_arm - 1.2, 38.6, 19.5],
            [l_arm + 1.8, 38.6, 21.0],
            [r_arm - 1.5, 44.0, 16.0],
        ],
        cuts: &[
            Cut::up(b(Hips), b(Spine), 42.5),
            Cut::up(b(Spine), b(Chest), 50.0),
            Cut::cube(b(Chest), b(Neck), [mid - 2.5, 57.5, -INF], [mid + 2.5, INF, INF]).only(&[mat::SKIN]),
            Cut::cube(b(UpperArmL), b(Offhand), [l_arm - 2.0, -INF, 19.0], [l_arm + 6.0, INF, INF]).only(&[mat::WOOD, 0]),
            Cut::down(b(UpperArmL), b(ForearmL), 46.0),
            Cut::cube(b(ForearmL), b(HandL), [-INF, -INF, -INF], [INF, 40.5, INF]).only(&[mat::SKIN]),
            Cut::down(b(UpperArmR), b(ForearmR), 46.0),
            Cut::cube(b(ForearmR), b(HandR), [-INF, -INF, -INF], [INF, 40.5, INF]).only(&[mat::SKIN]),
            Cut::down(b(ThighL), b(ShinL), 13.0).bare(),
            Cut::down(b(ShinL), b(FootL), 5.0),
            Cut::down(b(ThighR), b(ShinR), 13.0).bare(),
            Cut::down(b(ShinR), b(FootR), 5.0),
            // The robe's halves.
            Cut::down(b(Hips), b(ThighR), 36.5).across([-INF, -INF, -INF], [mid, INF, INF]),
            Cut::down(b(Hips), b(ThighL), 36.5).across([mid, -INF, -INF], [INF, INF, INF]),
        ],
    }
};

/**
 * The blemmyes' joints (grid 48 × 70 × 28, `foes2::BLEMMYES_SIZE`). He has no head: his face is in
 * his chest, so his neck and head bones are empty, set on his shoulders' line. His waist is above the
 * loincloth's rope (y 30.5), his chest above his grin (42: the eyes and the brow ride it, the mouth his
 * belly). His heavy arms bend at the elbow's ball (41.4), the fist below the wrist (30.5); the knee's
 * ball (12), the bare foot below the ankle (4.5). His knotted club, in his fist at the sculpt's high x
 * (his own left), is the `offhand`: its wood.
 */
pub const BLEMMYES: Joints = {
    use crate::render::characters::mat;
    use Bone::*;
    let (l_arm, r_arm, l_leg, r_leg, mid) = (41.5, 5.5, 30.5, 16.5, 23.5);
    Joints {
        def: &HUMANOID,
        from: HUMANOID_FOE_PARTS,
        mirrored: false,
        fists: false,
        at: &[
            [mid, 26.0, 13.0],
            [mid, 30.5, 13.0],
            [mid, 42.0, 13.0],
            [mid, 58.0, 13.0],
            [mid, 60.0, 13.0],
            [l_arm, 54.0, 13.0],
            [l_arm + 1.0, 41.4, 14.2],
            [l_arm + 1.6, 30.5, 16.5],
            [r_arm, 54.0, 13.0],
            [r_arm - 1.0, 41.4, 14.2],
            [r_arm - 1.6, 30.5, 16.5],
            [l_leg, 24.0, 13.0],
            [l_leg, 12.0, 13.4],
            [l_leg, 4.5, 13.0],
            [r_leg, 24.0, 13.0],
            [r_leg, 12.0, 13.4],
            [r_leg, 4.5, 13.0],
            [r_arm - 1.6, 27.0, 18.0],
            [l_arm + 1.6, 27.0, 18.0],
            [r_arm - 1.3, 36.0, 15.3],
        ],
        cuts: &[
            Cut::up(b(Hips), b(Spine), 30.5),
            Cut::up(b(Spine), b(Chest), 42.0),
            Cut::cube(b(UpperArmL), b(Offhand), [l_arm - 6.0, -INF, -INF], [l_arm + 12.0, 31.0, INF]).only(&[mat::WOOD]),
            Cut::down(b(UpperArmL), b(ForearmL), 41.5),
            Cut::down(b(ForearmL), b(HandL), 30.5),
            Cut::down(b(UpperArmR), b(ForearmR), 41.5),
            Cut::down(b(ForearmR), b(HandR), 30.5),
            Cut::down(b(ThighL), b(ShinL), 13.0),
            Cut::down(b(ShinL), b(FootL), 4.5),
            Cut::down(b(ThighR), b(ShinR), 13.0),
            Cut::down(b(ShinR), b(FootR), 4.5),
        ],
    }
};

/**
 * The tengu's joints (grid 48 × 92 × 56, `foes3::TENGU_SIZE`; z 27.5 his middle). His waist is his
 * sash (y 46), his chest from 54; his neck the red column under his head (y 62 up, its skin only),
 * his head the face, the nose, the mane and the beard and the cap. His jacket's wide sleeves end at the
 * elbow (51.5), the red fist below the wrist (43.5); the hakama's legs bend at the knee (19.5) over the
 * leg wraps, the tabi and geta below the ankle (8). The katana in his fist at the sculpt's high x (his
 * own left) is the `offhand`: its guard, blade and the grip below the fist (the grip above it, inside
 * his sleeve's cuff, stays with his hand).
 */
pub const TENGU: Joints = {
    use Bone::*;
    let (l_arm, r_arm, l_leg, r_leg, mid, z) = (35.0, 12.0, 28.5, 18.5, 23.5, 27.5);
    Joints {
        def: &HUMANOID,
        from: HUMANOID_FOE_PARTS,
        mirrored: false,
        fists: false,
        at: &[
            [mid, 40.0, z],
            [mid, 46.0, z],
            [mid, 54.0, z],
            [mid, 62.0, z],
            [mid, 67.0, z],
            [l_arm, 62.0, z],
            [l_arm + 1.1, 51.5, z + 2.2],
            [l_arm + 0.7, 43.5, z + 5.0],
            [r_arm, 62.0, z],
            [r_arm - 1.1, 51.5, z + 2.2],
            [r_arm - 0.7, 43.5, z + 5.0],
            [l_leg, 34.0, z],
            [l_leg, 19.5, z],
            [l_leg + 0.4, 8.0, z + 1.0],
            [r_leg, 34.0, z],
            [r_leg, 19.5, z],
            [r_leg - 0.4, 8.0, z + 1.0],
            [r_arm - 0.7, 41.0, z + 6.0],
            [l_arm + 0.7, 41.0, z + 6.0],
            [r_arm - 0.9, 47.0, z + 3.6],
        ],
        cuts: &[
            Cut::up(b(Hips), b(Spine), 46.0),
            Cut::up(b(Spine), b(Chest), 54.0),
            Cut::cube(b(Chest), b(Neck), [mid - 3.0, 62.0, -INF], [mid + 3.0, INF, INF]).only(SKIN_MATS),
            Cut::cube(b(UpperArmL), b(Offhand), [l_arm - 6.0, -INF, -INF], [l_arm + 12.0, 40.5, INF]).except(SKIN_MATS),
            Cut::down(b(UpperArmL), b(ForearmL), 51.5),
            Cut::down(b(ForearmL), b(HandL), 43.5),
            Cut::down(b(UpperArmR), b(ForearmR), 51.5),
            Cut::down(b(ForearmR), b(HandR), 43.5),
            Cut::down(b(ThighL), b(ShinL), 19.5),
            Cut::down(b(ShinL), b(FootL), 8.0),
            Cut::down(b(ThighR), b(ShinR), 19.5),
            Cut::down(b(ShinR), b(FootR), 8.0),
        ],
    }
};

/**
 * The rasetsu's joints (grid 64 × 116 × 40, `foes3::RASETSU_SIZE`; z 19.5 his middle). His waist is
 * the top of his trousers' seat (y 52), his chest from 64; his neck the thick red column under his
 * head (y 80.5 up), his head the scowl, the horns and the hair; his tail rides his hips. His arms bend
 * at the elbow (65.5), the bracer to the wrist (52), the fist below; his trousers' legs bend at the
 * knee (21) over the shin straps, the clawed foot below the ankle (6). A curved blade in each fist: the
 * one at the sculpt's high x (his own left) the `offhand`, the other the `blade` (the grips and the
 * steel, not the fists).
 */
pub const RASETSU: Joints = {
    use Bone::*;
    let (l_arm, r_arm, l_leg, r_leg, mid, z) = (47.0, 16.0, 38.0, 25.0, 31.5, 19.5);
    Joints {
        def: &HUMANOID,
        from: RASETSU_PARTS,
        mirrored: false,
        fists: false,
        at: &[
            [mid, 44.0, z],
            [mid, 52.0, z],
            [mid, 64.0, z],
            [mid, 80.5, z],
            [mid, 84.0, z],
            [l_arm, 78.0, z],
            [l_arm + 1.6, 65.5, z + 1.0],
            [l_arm + 2.4, 52.0, z + 3.0],
            [r_arm, 78.0, z],
            [r_arm - 1.6, 65.5, z + 1.0],
            [r_arm - 2.4, 52.0, z + 3.0],
            [l_leg, 40.0, z],
            [l_leg, 21.0, z],
            [l_leg, 6.0, z + 1.0],
            [r_leg, 40.0, z],
            [r_leg, 21.0, z],
            [r_leg, 6.0, z + 1.0],
            [r_arm - 2.4, 48.6, z + 3.6],
            [l_arm + 2.4, 48.6, z + 3.6],
            [r_arm - 2.0, 58.0, z + 2.0],
        ],
        cuts: &[
            Cut::up(b(Hips), b(Spine), 52.0),
            Cut::up(b(Spine), b(Chest), 64.0),
            Cut::cube(b(Chest), b(Neck), [mid - 4.5, 80.5, -INF], [mid + 4.5, INF, INF]).only(SKIN_MATS),
            Cut::cube(b(UpperArmL), b(Offhand), [l_arm - 4.0, -INF, -INF], [l_arm + 20.0, 51.0, INF]).except(SKIN_MATS),
            Cut::cube(b(UpperArmR), b(Blade), [r_arm - 20.0, -INF, -INF], [r_arm + 4.0, 51.0, INF]).except(SKIN_MATS),
            Cut::down(b(UpperArmL), b(ForearmL), 65.5),
            Cut::down(b(ForearmL), b(HandL), 52.0),
            Cut::down(b(UpperArmR), b(ForearmR), 65.5),
            Cut::down(b(ForearmR), b(HandR), 52.0),
            Cut::down(b(ThighL), b(ShinL), 21.0),
            Cut::down(b(ShinL), b(FootL), 6.0),
            Cut::down(b(ThighR), b(ShinR), 21.0),
            Cut::down(b(ShinR), b(FootR), 6.0),
        ],
    }
};

/// The rasetsu's code rig: the humanoid foes' parts and his tail, which rides his hips.
const RASETSU_PARTS: &[(&str, usize)] = &[
    ("body", b(Bone::Hips)),
    ("leftLeg", b(Bone::ThighR)),
    ("rightLeg", b(Bone::ThighL)),
    ("leftArm", b(Bone::UpperArmR)),
    ("rightArm", b(Bone::UpperArmL)),
    ("head", b(Bone::Head)),
    ("tail", b(Bone::Hips)),
];

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
            FoeRig::Draugr | FoeRig::Helmeted | FoeRig::Blemmyes | FoeRig::Tengu | FoeRig::Rasetsu => &["slam", "sweep", "rally"],
            FoeRig::Witch => &["hex", "rally"],
            FoeRig::Slime | FoeRig::Jorogumo => &["hex", "rally"],
            FoeRig::Kappa => &["shove"],
            FoeRig::Rokurokubi => &["neck_bite"],
            FoeRig::Oni => &["kanabo", "sweep", "rally"],
            FoeRig::Kitsune => &["foxfire"],
            FoeRig::Tulukus => &["talons"],
            FoeRig::Albasti => &["smother"],
            FoeRig::Cin => &["smokeless"],
            FoeRig::Arcura => &["roots"],
            FoeRig::Karakoncolos => &["call", "slam", "sweep"],
            _ => &["bite", "rally"],
        }
    }

    #[test]
    fn every_foes_clips_read_carry_their_markers_and_rest_on_its_joints() {
        for foe in FoeRig::ALL.into_iter().filter(|f| !crate::render::anim::library::embedded_in(f.dir()).is_empty()) {
            let c = crate::render::anim::library::foe_clips(foe).unwrap_or_else(|| panic!("{foe:?}'s clips read"));
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
