//! The skeletons of the foes Mixamo cannot drive, and their joint tables (ANIMATION.md §11): the
//! flyers (the raven, its night kin and the large bat), the wolves (the loup-garou and the dire
//! wolf), the cobra, the bog slime, the witch's minion, and the jorogumo (a woman's upper body on
//! the heroes' bones, on a spider's carapace with eight legs). Their clips are keyed by script in
//! Blender (`tools/blender/block_foe.py`).
//!
//! As the humanoid foes', their bones are named by the side they lie on (`.L` at +x, the sculpt's
//! high x: the rigs are not mirrored), and each table is measured on its sculpt in voxels of its
//! own grid.

use super::skeleton::{Cut, Joints, SkeletonDef};
use crate::render::characters::mat;

const INF: f32 = f32::INFINITY;

// ------------------------------------------------------------------ the flyers

/// A flyer: its body, its tail, its head and the lower half of its beak or jaw, each wing in two
/// (the arm to the wrist, and the hand: the primaries or the fingers), and its feet.
pub const FLYER: SkeletonDef = SkeletonDef {
    name: "flyer",
    bones: &["body", "tail", "head", "jaw", "wing.L", "hand.L", "wing.R", "hand.R", "foot.L", "foot.R"],
    parents: &[None, Some(0), Some(0), Some(2), Some(0), Some(4), Some(0), Some(6), Some(0), Some(0)],
    legs: &[],
};

mod fly {
    pub const BODY: usize = 0;
    pub const TAIL: usize = 1;
    pub const HEAD: usize = 2;
    pub const JAW: usize = 3;
    pub const WING_L: usize = 4;
    pub const HAND_L: usize = 5;
    pub const WING_R: usize = 6;
    pub const HAND_R: usize = 7;
    pub const FOOT_L: usize = 8;
    pub const FOOT_R: usize = 9;
}

/// The flyers' code rigs: a wing on each side (`leftWing` at the sculpt's low x, its own right).
const FLYER_PARTS: &[(&str, usize)] = &[("body", fly::BODY), ("leftWing", fly::WING_R), ("rightWing", fly::WING_L)];

/// The feet: their leathery shanks and their talons (not the feathers or fur over them).
const FEET: &[u32] = &[mat::LEATHER, mat::IVORY];

/**
 * The raven's joints (grid 76 × 28 × 44, `foes::RAVEN_SIZE`; the night raven's are the same, at its
 * own size). Its body is the ball of its breast and back, the wedge of its tail behind z 9.5, its
 * head and neck before z 28 with the upper beak and the throat hackles, the lower beak its jaw. Each
 * wing hinges where it meets the body (the code's pivots) and again at the wrist, 20 voxels out,
 * where the primaries fan; its feet are the shanks and talons tucked under its belly.
 */
pub const RAVEN: Joints = {
    use fly::*;
    let cx = 37.5;
    Joints {
        def: &FLYER,
        from: FLYER_PARTS,
        mirrored: false,
        fists: false,
        at: &[
            [cx, 12.5, 20.0],
            [cx, 11.0, 9.5],
            [cx, 15.5, 28.0],
            [cx, 14.0, 35.5],
            [45.5, 14.0, 21.5],
            [cx + 20.0, 15.8, 24.0],
            [29.5, 14.0, 21.5],
            [cx - 20.0, 15.8, 24.0],
            [cx + 3.0, 6.5, 17.0],
            [cx - 3.0, 6.5, 17.0],
        ],
        cuts: &[
            Cut::cube(BODY, FOOT_L, [cx + 1.0, -INF, 9.0], [INF, 7.5, 19.5]).only(FEET),
            Cut::cube(BODY, FOOT_R, [-INF, -INF, 9.0], [cx - 1.0, 7.5, 19.5]).only(FEET),
            Cut::plane(BODY, TAIL, 2, 9.5, true),
            Cut::plane(BODY, HEAD, 2, 28.0, false),
            Cut::cube(HEAD, JAW, [cx - 2.5, 12.0, 35.5], [cx + 2.5, 14.5, INF]),
            Cut::plane(WING_L, HAND_L, 0, cx + 20.0, false),
            Cut::plane(WING_R, HAND_R, 0, cx - 20.0, true),
        ],
    }
};

/**
 * The large bat's joints (grid 84 × 30 × 44, `foes2::BAT_SIZE`). Its body is the furry ball, the
 * rump behind z 14 a stub of a tail, its head (the snub face, the ears) before z 27, its jaw the
 * fangs and the chin under the snout. Each wing hinges at the code's pivot and again at the wrist
 * (20 voxels out, where the three long fingers fan and the thumb claw hooks), its feet the bony
 * shanks and claws under it.
 */
pub const LARGE_BAT: Joints = {
    use fly::*;
    let cx = 41.5;
    Joints {
        def: &FLYER,
        from: FLYER_PARTS,
        mirrored: false,
        fists: false,
        at: &[
            [cx, 15.0, 22.0],
            [cx, 15.0, 14.0],
            [cx, 17.0, 27.0],
            [cx, 14.5, 32.5],
            [48.5, 15.0, 22.0],
            [cx + 20.0, 21.0, 27.0],
            [34.5, 15.0, 22.0],
            [cx - 20.0, 21.0, 27.0],
            [cx + 3.5, 10.0, 18.0],
            [cx - 3.5, 10.0, 18.0],
        ],
        cuts: &[
            Cut::cube(BODY, FOOT_L, [cx + 1.0, -INF, -INF], [INF, 11.0, 20.0]).only(FEET),
            Cut::cube(BODY, FOOT_R, [-INF, -INF, -INF], [cx - 1.0, 11.0, 20.0]).only(FEET),
            Cut::plane(BODY, TAIL, 2, 14.0, true),
            Cut::plane(BODY, HEAD, 2, 27.0, false),
            Cut::cube(HEAD, JAW, [cx - 3.0, -INF, 32.5], [cx + 3.0, 14.5, INF]),
            Cut::plane(WING_L, HAND_L, 0, cx + 19.5, false),
            Cut::plane(WING_R, HAND_R, 0, cx - 19.5, true),
        ],
    }
};

// ------------------------------------------------------------------ the wolves

/// A four-legged beast: its hips (the root), its chest and shoulders, its neck and head and jaw, its
/// tail in three, and each leg in three (the upper leg, the lower, the paw).
pub const QUADRUPED: SkeletonDef = SkeletonDef {
    name: "quadruped",
    bones: &[
        "hips",
        "chest",
        "neck",
        "head",
        "jaw",
        "tail.1",
        "tail.2",
        "tail.3",
        "upper_f.L",
        "lower_f.L",
        "paw_f.L",
        "upper_f.R",
        "lower_f.R",
        "paw_f.R",
        "upper_b.L",
        "lower_b.L",
        "paw_b.L",
        "upper_b.R",
        "lower_b.R",
        "paw_b.R",
    ],
    parents: &[
        None,
        Some(0),
        Some(1),
        Some(2),
        Some(3),
        Some(0),
        Some(5),
        Some(6),
        Some(1),
        Some(8),
        Some(9),
        Some(1),
        Some(11),
        Some(12),
        Some(0),
        Some(14),
        Some(15),
        Some(0),
        Some(17),
        Some(18),
    ],
    legs: &[],
};

mod quad {
    pub const HIPS: usize = 0;
    pub const CHEST: usize = 1;
    pub const NECK: usize = 2;
    pub const HEAD: usize = 3;
    pub const JAW: usize = 4;
    pub const TAIL1: usize = 5;
    pub const TAIL2: usize = 6;
    pub const TAIL3: usize = 7;
    pub const UF_L: usize = 8;
    pub const LF_L: usize = 9;
    pub const PF_L: usize = 10;
    pub const UF_R: usize = 11;
    pub const LF_R: usize = 12;
    pub const PF_R: usize = 13;
    pub const UB_L: usize = 14;
    pub const LB_L: usize = 15;
    pub const PB_L: usize = 16;
    pub const UB_R: usize = 17;
    pub const LB_R: usize = 18;
    pub const PB_R: usize = 19;
}

/// The wolves' code rigs: a leg at each corner (the `Left` legs at the sculpt's low x, their own
/// right), the head and the tail.
const QUAD_PARTS: &[(&str, usize)] = &[
    ("body", quad::HIPS),
    ("frontLeftLeg", quad::UF_R),
    ("frontRightLeg", quad::UF_L),
    ("backLeftLeg", quad::UB_R),
    ("backRightLeg", quad::UB_L),
    ("head", quad::HEAD),
    ("tail", quad::TAIL1),
];

/**
 * The loup-garou's joints (grid 30 × 46 × 66, `foes2::WOLF_SIZE`; it faces +z). Its hips are the
 * rump, its chest from z 32 (the deep chest and the shoulders), its neck the ruff from z 46, its head
 * the code's, the lower jaw and the bottom teeth under y 32.5 its jaw. The tail, rising back from
 * the rump, bends at z 5 and 2. The legs hinge at the shoulders and hips (the code's pivots), the
 * knees and hocks at y 15, the paws below y 5 with their claws.
 */
pub const WOLF: Joints = {
    use quad::*;
    let (l, r, cx) = (19.5, 9.5, 14.5);
    Joints {
        def: &QUADRUPED,
        from: QUAD_PARTS,
        mirrored: false,
        fists: false,
        at: &[
            [cx, 29.0, 18.0],
            [cx, 31.0, 32.0],
            [cx, 33.0, 46.0],
            [cx, 34.0, 50.0],
            [cx, 32.5, 55.0],
            [cx, 30.0, 9.5],
            [cx, 33.0, 5.0],
            [cx, 35.0, 2.0],
            [l, 26.0, 46.0],
            [l, 15.0, 47.5],
            [l, 5.0, 46.5],
            [r, 26.0, 46.0],
            [r, 15.0, 47.5],
            [r, 5.0, 46.5],
            [l, 26.0, 16.0],
            [l, 15.0, 14.0],
            [l, 5.0, 15.0],
            [r, 26.0, 16.0],
            [r, 15.0, 14.0],
            [r, 5.0, 15.0],
        ],
        cuts: &[
            Cut::plane(HIPS, CHEST, 2, 32.0, false),
            Cut::plane(CHEST, NECK, 2, 46.0, false),
            Cut::cube(HEAD, JAW, [-INF, -INF, 54.0], [INF, 32.5, INF]),
            Cut::plane(TAIL1, TAIL2, 2, 5.0, true),
            Cut::plane(TAIL2, TAIL3, 2, 2.0, true),
            Cut::down(UF_L, LF_L, 15.0),
            Cut::down(LF_L, PF_L, 5.0),
            Cut::down(UF_R, LF_R, 15.0),
            Cut::down(LF_R, PF_R, 5.0),
            Cut::down(UB_L, LB_L, 15.0),
            Cut::down(LB_L, PB_L, 5.0),
            Cut::down(UB_R, LB_R, 15.0),
            Cut::down(LB_R, PB_R, 5.0),
        ],
    }
};

/**
 * The dire wolf's joints (grid 40 × 62 × 88, `foes3::DIRE_SIZE`; it faces +z). Its hips are the
 * rump, its chest from z 40, its neck the great ruff and mane from z 58, its head the code's, its jaw
 * the lower jaw and its fangs under y 39.5. The tail hangs from the rump and bends at y 31 and 19.
 * The front legs hinge at the shoulders, the elbows at y 17, the paws below 4.6; the hind legs at
 * the hips, the hocks bent back at y 16, the paws below 4.6.
 */
pub const DIRE_WOLF: Joints = {
    use quad::*;
    let (l, r, cx) = (26.5, 12.5, 19.5);
    Joints {
        def: &QUADRUPED,
        from: QUAD_PARTS,
        mirrored: false,
        fists: false,
        at: &[
            [cx, 35.0, 24.0],
            [cx, 38.0, 40.0],
            [cx, 42.0, 58.0],
            [cx, 46.0, 67.0],
            [cx, 39.5, 71.0],
            [cx, 38.0, 15.0],
            [cx, 31.0, 7.0],
            [cx, 19.0, 4.0],
            [l, 34.0, 61.0],
            [l, 17.0, 62.6],
            [l, 4.6, 62.2],
            [r, 34.0, 61.0],
            [r, 17.0, 62.6],
            [r, 4.6, 62.2],
            [l, 33.0, 23.0],
            [l + 0.4, 16.0, 16.0],
            [l + 0.2, 4.6, 20.6],
            [r, 33.0, 23.0],
            [r - 0.4, 16.0, 16.0],
            [r - 0.2, 4.6, 20.6],
        ],
        cuts: &[
            Cut::plane(HIPS, CHEST, 2, 40.0, false),
            Cut::plane(CHEST, NECK, 2, 58.0, false),
            Cut::cube(HEAD, JAW, [-INF, -INF, 70.0], [INF, 39.5, INF]),
            Cut::down(TAIL1, TAIL2, 31.0),
            Cut::down(TAIL2, TAIL3, 19.0),
            Cut::down(UF_L, LF_L, 17.0),
            Cut::down(LF_L, PF_L, 4.6),
            Cut::down(UF_R, LF_R, 17.0),
            Cut::down(LF_R, PF_R, 4.6),
            Cut::down(UB_L, LB_L, 16.0),
            Cut::down(LB_L, PB_L, 4.6),
            Cut::down(UB_R, LB_R, 16.0),
            Cut::down(LB_R, PB_R, 4.6),
        ],
    }
};

// ------------------------------------------------------------------ the cobra

/// A snake: its coil on the ground (the root), its tail's end in two, the neck reared up off the
/// coil in five, the hood, the head and its lower jaw.
pub const SNAKE: SkeletonDef = SkeletonDef {
    name: "snake",
    bones: &["coil", "tail.1", "tail.2", "neck.1", "neck.2", "neck.3", "neck.4", "neck.5", "hood", "head", "jaw"],
    parents: &[None, Some(0), Some(1), Some(0), Some(3), Some(4), Some(5), Some(6), Some(7), Some(8), Some(9)],
    legs: &[],
};

/**
 * The cobra's joints (grid 60 × 84 × 60, `foes3::COBRA_SIZE`). Its coil lies on the ground; its
 * tail's end curls up behind (bent where it turns back, z 14.5). The neck rises off the coil, cut
 * every nine voxels up it (y 15, 24, 33, 42) to the hood at y 50 (the code's head part: the hood
 * spread wide); the head is the skull over the hood (above y 70.5, and before z 42.5), and the jaw
 * its lower half, open on the fangs and the tongue.
 */
pub const COBRA: Joints = {
    let cx = 29.5;
    Joints {
        def: &SNAKE,
        from: &[("body", 0), ("tail", 1), ("neck", 3), ("head", 8)],
        mirrored: false,
        fists: false,
        at: &[
            [cx, 3.0, 29.5],
            [15.5, 3.0, 21.5],
            [15.3, 4.2, 14.5],
            [27.0, 7.0, 35.0],
            [29.2, 15.0, 35.2],
            [cx, 24.0, 34.0],
            [cx, 33.0, 33.3],
            [cx, 42.0, 33.4],
            [cx, 50.0, 34.7],
            [cx, 70.0, 40.0],
            [cx, 70.0, 42.5],
        ],
        cuts: &[
            Cut::plane(1, 2, 2, 14.5, true),
            Cut::up(3, 4, 15.0),
            Cut::up(4, 5, 24.0),
            Cut::up(5, 6, 33.0),
            Cut::up(6, 7, 42.0),
            Cut::cube(8, 9, [-INF, 70.5, -INF], [INF, INF, INF]),
            Cut::cube(8, 9, [-INF, 64.0, 42.5], [INF, INF, INF]),
            Cut::cube(9, 10, [-INF, -INF, 42.5], [INF, 69.8, INF]),
        ],
    }
};

// ------------------------------------------------------------------ the bog slime

/// A slime: its foot on the ground (the root) with a lobe sagging out to each side, its belly, its
/// crown (the eyes, the dome), the lily pad on top, and the mouth.
pub const SLIME: SkeletonDef = SkeletonDef {
    name: "slime",
    bones: &["base", "belly", "crown", "lily", "mouth", "lobe.L", "lobe.R"],
    parents: &[None, Some(0), Some(1), Some(2), Some(1), Some(0), Some(0)],
    legs: &[],
};

/**
 * The bog slime's joints (grid 44 × 32 × 44, `foes::SLIME_SIZE`). Its foot is the sagging rim and
 * the puddle below y 7, its sides out past x 8 and 35 lobes of it; its belly up to y 17, its crown
 * above (the dome and the big eyes); the lily pad in flower on top above y 28; its mouth the gummy
 * grin's lower lip and tongue (its front, z from 35, y 7 to 10.5), so it gapes.
 */
pub const BOG_SLIME: Joints = {
    let c = 21.5;
    Joints {
        def: &SLIME,
        from: &[],
        mirrored: false,
        fists: false,
        at: &[[c, 0.0, c], [c, 7.0, c], [c, 17.0, c], [c, 28.0, c], [c, 10.5, 35.0], [36.0, 3.0, c], [7.0, 3.0, c]],
        cuts: &[
            Cut::up(0, 1, 7.0),
            Cut::cube(0, 5, [35.5, -INF, -INF], [INF, 6.5, INF]),
            Cut::cube(0, 6, [-INF, -INF, -INF], [7.5, 6.5, INF]),
            Cut::cube(1, 4, [9.5, 7.0, 35.0], [33.5, 10.5, INF]),
            Cut::up(1, 2, 17.0),
            Cut::up(2, 3, 28.0),
        ],
    }
};

// ------------------------------------------------------------------ the minion

/// The witch's minion: its robe (the root) and its ragged hem, its head and its hat with a bendy
/// tip, and two little arms with their hands.
pub const MINION: SkeletonDef = SkeletonDef {
    name: "minion",
    bones: &["body", "hem", "head", "hat", "hat.tip", "arm.L", "hand.L", "arm.R", "hand.R"],
    parents: &[None, Some(0), Some(0), Some(2), Some(3), Some(0), Some(5), Some(0), Some(7)],
    legs: &[0, 1],
};

/**
 * The minion's joints (grid 26 × 44 × 22, `foes2::MINION_SIZE`; its code rig is one piece). Its
 * robe is the cone to the floor, the hem below y 5.5; its head, the face and the hair, from y 26;
 * its hat's brim from y 33.5 and the bent cone's tip from y 39.5; each sleeve, reaching forward
 * from the robe's sides (x from 16.5 and to 8.5), an arm, and the little pale hand its hand.
 */
pub const MINION_JOINTS: Joints = {
    let (cx, cz) = (12.5, 10.5);
    Joints {
        def: &MINION,
        from: &[],
        mirrored: false,
        fists: false,
        at: &[
            [cx, 8.0, cz],
            [cx, 5.5, cz],
            [cx, 26.0, cz],
            [cx, 33.5, cz],
            [14.5, 39.5, cz],
            [18.0, 22.0, 11.0],
            [20.0, 15.0, 14.5],
            [7.0, 22.0, 11.0],
            [5.0, 15.0, 14.5],
        ],
        cuts: &[
            Cut::up(0, 2, 26.0),
            Cut::up(2, 3, 33.5),
            Cut::up(3, 4, 39.5),
            Cut::cube(0, 5, [16.5, 12.0, 11.5], [INF, 23.5, INF]),
            Cut::cube(5, 6, [-INF, -INF, -INF], [INF, 16.0, INF]).only(&[mat::SKIN]),
            Cut::cube(0, 7, [-INF, 12.0, 11.5], [8.5, 23.5, INF]),
            Cut::cube(7, 8, [-INF, -INF, -INF], [INF, 16.0, INF]).only(&[mat::SKIN]),
            Cut::plane(0, 1, 1, 5.5, true),
        ],
    }
};

// ------------------------------------------------------------------ the jorogumo

/// The jorogumo: a spider's carapace (the root) and abdomen; on the carapace a woman from the waist
/// up on the heroes' upper bones (named as theirs, so Mixamo's motion retargets onto her: `hips` is
/// her waist), her sickle in her right fist (`blade`); and eight legs in three (`leg0.a` the upper
/// leg up to the knee, `.b` the long shin, `.c` the black point), legs 0, 2, 4 and 6 on her right
/// (front to back), 1, 3, 5 and 7 on her left.
pub const SPIDER_WOMAN: SkeletonDef = SkeletonDef {
    name: "spider woman",
    bones: &[
        "carapace",
        "abdomen",
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
        "blade",
        "leg0.a",
        "leg0.b",
        "leg0.c",
        "leg1.a",
        "leg1.b",
        "leg1.c",
        "leg2.a",
        "leg2.b",
        "leg2.c",
        "leg3.a",
        "leg3.b",
        "leg3.c",
        "leg4.a",
        "leg4.b",
        "leg4.c",
        "leg5.a",
        "leg5.b",
        "leg5.c",
        "leg6.a",
        "leg6.b",
        "leg6.c",
        "leg7.a",
        "leg7.b",
        "leg7.c",
    ],
    parents: &[
        None,
        Some(0),
        Some(0),
        Some(2),
        Some(3),
        Some(4),
        Some(5),
        Some(4),
        Some(7),
        Some(8),
        Some(4),
        Some(10),
        Some(11),
        Some(12),
        Some(0),
        Some(14),
        Some(15),
        Some(0),
        Some(17),
        Some(18),
        Some(0),
        Some(20),
        Some(21),
        Some(0),
        Some(23),
        Some(24),
        Some(0),
        Some(26),
        Some(27),
        Some(0),
        Some(29),
        Some(30),
        Some(0),
        Some(32),
        Some(33),
        Some(0),
        Some(35),
        Some(36),
    ],
    legs: &[0, 1, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37],
};

/// Where each of her legs leaves her carapace and the way it reaches out (level, unit): her legs
/// sweep out at 28, 62, 108 and 148 degrees from her front (`foes3::joro_leg`).
const JORO_LEGS: [([f32; 3], [f32; 3]); 8] = [
    ([35.98, 31.0, 51.51], [-0.4695, 0.0, 0.8829]),
    ([43.02, 31.0, 51.51], [0.4695, 0.0, 0.8829]),
    ([32.88, 31.0, 47.99], [-0.8829, 0.0, 0.4695]),
    ([46.12, 31.0, 47.99], [0.8829, 0.0, 0.4695]),
    ([32.37, 31.0, 41.37], [-0.9511, 0.0, -0.309]),
    ([46.63, 31.0, 41.37], [0.9511, 0.0, -0.309]),
    ([35.53, 31.0, 36.79], [-0.5299, 0.0, -0.848]),
    ([43.47, 31.0, 36.79], [0.5299, 0.0, -0.848]),
];

/// A leg's knee (12 out and 14 up from its root) and ankle (23.5 out, 14 down).
const fn joro_knee(i: usize, out: f32, up: f32) -> [f32; 3] {
    let (r, d) = JORO_LEGS[i];
    [r[0] + d[0] * out, r[1] + up, r[2] + d[2] * out]
}

const fn leg_joints(i: usize) -> [[f32; 3]; 3] {
    [JORO_LEGS[i].0, joro_knee(i, 12.0, 14.0), joro_knee(i, 23.5, -14.0)]
}

const fn leg_cuts(i: usize) -> [Cut; 2] {
    let (o, d) = JORO_LEGS[i];
    let a = 14 + i * 3;
    [Cut::slanted(a, a + 1, o, d, 12.0, false), Cut::slanted(a + 1, a + 2, o, d, 23.5, false)]
}

const JORO_AT: [[f32; 3]; 38] = {
    let cx = 39.5;
    let mut at = [[0.0; 3]; 38];
    let upper = [
        [cx, 30.0, 44.0],
        [cx, 31.0, 34.0],
        [cx, 40.0, 48.5],
        [cx, 46.0, 48.8],
        [cx, 52.0, 49.0],
        [cx, 59.0, 49.0],
        [cx, 64.0, 49.6],
        // Her left arm (the sculpt's high x), its elbow, its wrist; her right, and the sickle's grip.
        [47.0, 58.6, 49.0],
        [49.2, 50.4, 50.6],
        [50.2, 48.0, 56.5],
        [32.0, 58.6, 49.0],
        [29.8, 50.4, 50.6],
        [28.8, 48.0, 56.5],
        [28.8, 48.2, 58.4],
    ];
    let mut k = 0;
    while k < upper.len() {
        at[k] = upper[k];
        k += 1;
    }
    let mut i = 0;
    while i < 8 {
        let j = leg_joints(i);
        at[14 + i * 3] = j[0];
        at[15 + i * 3] = j[1];
        at[16 + i * 3] = j[2];
        i += 1;
    }
    at
};

const JORO_CUTS: [Cut; 26] = {
    let cx = 39.5;
    let first = [
        // The abdomen behind the stalk; her waist up from the carapace, her spine and chest; her
        // neck the column under her head.
        Cut::plane(0, 1, 2, 34.0, true),
        Cut::up(0, 2, 40.5),
        Cut::up(2, 3, 46.0),
        Cut::up(3, 4, 52.0),
        Cut::cube(4, 5, [cx - 2.5, 58.5, -INF], [cx + 2.5, INF, INF]),
        // The sickle out of her right fist first (its handle and its crescent: not her claws).
        Cut::cube(10, 13, [24.0, 45.0, 56.0], [34.0, INF, INF]).except(&[mat::SKIN, mat::IVORY]),
        Cut::down(7, 8, 51.0),
        Cut::plane(8, 9, 2, 56.5, false),
        Cut::down(10, 11, 51.0),
        Cut::plane(11, 12, 2, 56.5, false),
    ];
    let mut cuts = [Cut::cube(0, 0, [0.0; 3], [0.0; 3]); 26];
    let mut k = 0;
    while k < first.len() {
        cuts[k] = first[k];
        k += 1;
    }
    let mut i = 0;
    while i < 8 {
        let c = leg_cuts(i);
        cuts[first.len() + i * 2] = c[0];
        cuts[first.len() + i * 2 + 1] = c[1];
        i += 1;
    }
    cuts
};

/**
 * The jorogumo's joints (grid 80 × 84 × 80, `foes3::JORO_SIZE`). Her carapace is the plated shell;
 * the abdomen, the great bulb, behind the stalk at z 34. Her waist rises off the carapace at y 40.5,
 * her spine from 46, her chest from 52 with the little horns on her shoulders, her neck the column
 * under her head; her arms hinge at the shoulders (the code's), bend at the elbow (y 51) and reach
 * forward to the wrist (z 56.5); the sickle in her right fist is its own bone. Each leg hinges where
 * it leaves the carapace (the code's), at the high knee 12 voxels out, and at the ankle 23.5 out,
 * the cuts slanted across the leg's own reach.
 */
pub const JOROGUMO: Joints = Joints {
    def: &SPIDER_WOMAN,
    from: &[
        ("body", 0),
        ("leg0", 14),
        ("leg1", 17),
        ("leg2", 20),
        ("leg3", 23),
        ("leg4", 26),
        ("leg5", 29),
        ("leg6", 32),
        ("leg7", 35),
        ("leftArm", 10),
        ("rightArm", 7),
        ("head", 6),
    ],
    mirrored: false,
    fists: false,
    at: &JORO_AT,
    cuts: &JORO_CUTS,
};
