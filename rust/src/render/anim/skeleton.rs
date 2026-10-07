//! The heroes' jointed skeleton: twenty bones in a hierarchy (hips, spine, chest, neck and head;
//! upper arm, forearm and hand, thigh, shin and foot each side; the weapon on the sword hand, the
//! offhand weapon on the shield hand and the shield on the shield forearm), and the cutting of a
//! hero's rig (its six parts as the code swings them, and the weapons and shield layered on) into
//! those bones. Each voxel keeps to one bone, rigidly: the voxel look stays.
//!
//! The cut is data: each hero has a table of joints, measured on his sculpt as it stands, and a
//! list of cuts, each moving the voxels of one bone that lie in a box to the next bone down the
//! limb (`Joints`). The three heroes' tables are here.
//!
//! A skeleton is data too (`SkeletonDef`: its bones' names and parents): the heroes' twenty bones
//! (`HUMANOID`) serve the humanoid foes as well, and the others have their own (`anim::foe_rigs`:
//! bats, wolves, the cobra, the slime, the minion, the jorogumo). A joint table names its skeleton.

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
const HERO_PARTS: &[(&str, usize)] = &[
    ("body", Bone::Hips as usize),
    ("leftLeg", Bone::ThighL as usize),
    ("rightLeg", Bone::ThighR as usize),
    ("shieldArm", Bone::UpperArmL as usize),
    ("swordArm", Bone::UpperArmR as usize),
    ("head", Bone::Head as usize),
    ("blade", Bone::Blade as usize),
    ("offhand", Bone::Offhand as usize),
    ("shield", Bone::Shield as usize),
];

/// A skeleton: its bones' names (parents before children: the order of a jointed rig's parts and
/// of a clip's tracks) and each one's parent (the first bone is the root).
#[derive(Debug)]
pub struct SkeletonDef {
    pub name: &'static str,
    pub bones: &'static [&'static str],
    pub parents: &'static [Option<usize>],
    /// The bones a flinch laid over the body leaves to the gait beneath (the hips and legs); none
    /// for a small body the flinch takes whole.
    pub legs: &'static [usize],
}

impl SkeletonDef {
    pub fn len(&self) -> usize {
        self.bones.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bones.is_empty()
    }

    /// A bone by its name.
    pub fn index(&self, name: &str) -> Option<usize> {
        self.bones.iter().position(|b| *b == name)
    }
}

/// The heroes' twenty bones, which the humanoid foes share (their clips retargeted from Mixamo as
/// the heroes' are).
pub const HUMANOID: SkeletonDef = SkeletonDef {
    name: "humanoid",
    bones: &BONES,
    parents: &PARENTS,
    legs: &[
        Bone::Hips as usize,
        Bone::ThighL as usize,
        Bone::ShinL as usize,
        Bone::FootL as usize,
        Bone::ThighR as usize,
        Bone::ShinR as usize,
        Bone::FootR as usize,
    ],
};

/// A cut: every voxel of `from` whose place in the sculpt's grid lies in the box `lo..=hi`
/// (voxel centres; unbounded sides are infinite) goes to `to`. A cut across a limb at a height
/// (`below`, `above`, `below_within`) is sleeved: the band of the bone above it is laid on the bone
/// below as well (`SLEEVE`), within the cut's box across the limb.
#[derive(Clone, Copy, Debug)]
pub struct Cut {
    pub from: usize,
    pub to: usize,
    pub lo: [f32; 3],
    pub hi: [f32; 3],
    pub sleeve: bool,
    /// The axis the cut's plane lies across (1, y, for a limb hanging down; 0 for a wing, 2 for a
    /// tail or a neck reaching back or forward).
    pub axis: usize,
    /// Materials (`characters::mat`, 0 for plain) the cut takes only (`keep`), or leaves (not
    /// `keep`); empty: any. (A weapon in a bony fist: its leather and iron, not the bone round it.)
    pub mats: &'static [u32],
    pub keep: bool,
    /// A plane at a slant (a spider's leg reaching out at an angle): the coordinate along the cut's
    /// axis is the voxel's distance from this point along this unit direction, not its grid
    /// coordinate.
    pub slant: Option<([f32; 3], [f32; 3])>,
}

const INF: f32 = f32::INFINITY;

/// How deep, in voxels, the band of a bone above a joint that the bone below it carries too.
const SLEEVE: f32 = 2.5;

/// The shade mode of a skin voxel in a rig part (`voxels::make_part`: 5 and the material).
const SKIN_SHADE: f32 = 5.0 + (crate::render::characters::mat::SKIN >> 24) as f32;

impl Cut {
    /// Below the plane `y` (a joint across the limb).
    const fn below(from: Bone, to: Bone, y: f32) -> Self {
        Self::down(from as usize, to as usize, y)
    }

    /// Above the plane `y`.
    const fn above(from: Bone, to: Bone, y: f32) -> Self {
        Self::up(from as usize, to as usize, y)
    }

    /// Below the plane `y` within the box `lo..=hi` across it (x and z): a joint of one limb where
    /// the plane would also cross another part (a skirt's halves, a leg inside a skirt).
    const fn below_within(from: Bone, to: Bone, y: f32, lo: [f32; 2], hi: [f32; 2]) -> Self {
        Self::down(from as usize, to as usize, y).across([lo[0], -INF, lo[1]], [hi[0], INF, hi[1]])
    }

    /// A box, unsleeved.
    const fn boxed(from: Bone, to: Bone, lo: [f32; 3], hi: [f32; 3]) -> Self {
        Self::cube(from as usize, to as usize, lo, hi)
    }

    /// Bone `from`'s voxels below the plane `y` go to bone `to` (sleeved).
    pub const fn down(from: usize, to: usize, y: f32) -> Self {
        Self::plane(from, to, 1, y, true)
    }

    /// Above the plane `y` (sleeved).
    pub const fn up(from: usize, to: usize, y: f32) -> Self {
        Self::plane(from, to, 1, y, false)
    }

    /// Across axis `axis` at `at`: the voxels on its low side (`low`) or its high side go to `to`
    /// (sleeved: the band of `from` by the plane is laid on `to` as well).
    pub const fn plane(from: usize, to: usize, axis: usize, at: f32, low: bool) -> Self {
        let (mut lo, mut hi) = ([-INF; 3], [INF; 3]);
        if low {
            hi[axis] = at;
        } else {
            lo[axis] = at;
        }
        Self { from, to, lo, hi, sleeve: true, axis, mats: &[], keep: true, slant: None }
    }

    /// Across a slanted plane: the voxels whose distance from `origin` along `dir` (a unit vector,
    /// in the grid) is below `at` (`low`) or above it go to `to` (sleeved).
    pub const fn slanted(from: usize, to: usize, origin: [f32; 3], dir: [f32; 3], at: f32, low: bool) -> Self {
        Self { slant: Some((origin, dir)), ..Self::plane(from, to, 1, at, low) }
    }

    /// A box, unsleeved.
    pub const fn cube(from: usize, to: usize, lo: [f32; 3], hi: [f32; 3]) -> Self {
        Self { from, to, lo, hi, sleeve: false, axis: 1, mats: &[], keep: true, slant: None }
    }

    /// A plane's cut kept to the box `lo..=hi` on the other two axes (its own axis's bounds are
    /// the plane's).
    pub const fn across(self, lo: [f32; 3], hi: [f32; 3]) -> Self {
        let (mut l, mut h) = (lo, hi);
        l[self.axis] = self.lo[self.axis];
        h[self.axis] = self.hi[self.axis];
        Self { lo: l, hi: h, ..self }
    }

    /// Only the voxels of these materials.
    pub const fn only(self, mats: &'static [u32]) -> Self {
        Self { mats, keep: true, ..self }
    }

    /// All but the voxels of these materials.
    pub const fn except(self, mats: &'static [u32]) -> Self {
        Self { mats, keep: false, ..self }
    }

    /// The same cut without its sleeve (a joint hidden under a skirt, whose band would be the skirt's).
    pub const fn bare(self) -> Self {
        Self { sleeve: false, ..self }
    }

    /// A cut across the limb at a height (`below` or `above`): its plane, and whether the bone it
    /// sends voxels to lies below it.
    fn crease(&self) -> Option<(f32, bool)> {
        if !self.sleeve {
            return None;
        }
        let a = self.axis;
        match (self.lo[a] == -INF, self.hi[a] == INF) {
            (true, false) => Some((self.hi[a], true)),
            (false, true) => Some((self.lo[a], false)),
            _ => None,
        }
    }

    fn holds(&self, g: [f32; 3], shade: f32) -> bool {
        let g = self.measure(g);
        (0..3).all(|k| g[k] >= self.lo[k] && g[k] <= self.hi[k]) && self.takes(shade)
    }

    /// A voxel's place as the cut measures it: its grid coordinates, with the slanted plane's
    /// distance in place of the axis's.
    fn measure(&self, mut g: [f32; 3]) -> [f32; 3] {
        if let Some((o, d)) = self.slant {
            g[self.axis] = (0..3).map(|k| (g[k] - o[k]) * d[k]).sum();
        }
        g
    }

    /// Whether a voxel of shade mode `shade` (`voxels::make_part`: 0 plain, 5 and its material) is
    /// one this cut moves.
    fn takes(&self, shade: f32) -> bool {
        if self.mats.is_empty() {
            return true;
        }
        let m = if shade >= 5.0 { (shade - 5.0).round() as u32 } else { 0 };
        self.mats.iter().any(|&k| k >> 24 == m) == self.keep
    }
}

/// A body's joints, measured on its sculpt: where each bone turns, in voxels of its own grid as
/// it is sculpted (x as in the sculpt, before a hero's rig turns him left for right), and the cuts
/// that share its voxels out among the bones, applied in order.
pub struct Joints {
    /// Its skeleton.
    pub def: &'static SkeletonDef,
    pub at: &'static [[f32; 3]],
    pub cuts: &'static [Cut],
    /// The bone each part of the code's rig starts in, by the part's name (a part not named starts
    /// in the root).
    pub from: &'static [(&'static str, usize)],
    /// Its rig is mirrored, left for right, from its sculpt (the heroes': `forge::hero_rigs`); the
    /// foes' are not.
    pub mirrored: bool,
    /// The fists modelled round a held weapon's grip go to the hands (the heroes' armory).
    pub fists: bool,
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
        def: &HUMANOID,
        from: HERO_PARTS,
        mirrored: true,
        fists: true,
        at: &[
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
            Cut::boxed(Head, Neck, [23.5, -INF, 10.5], [35.5, 60.5, 20.5]),
            Cut::below(UpperArmL, ForearmL, 44.5),
            Cut::below(ForearmL, HandL, 34.5),
            Cut::below(UpperArmR, ForearmR, 44.5),
            Cut::below(ForearmR, HandR, 34.5),
            Cut::below(ThighL, ShinL, 14.5),
            Cut::below(ShinL, FootL, 4.5),
            // The toe of the boot, reaching out before the shaft above the ankle.
            Cut::boxed(ShinL, FootL, [-INF, -INF, 24.5], [INF, 8.5, INF]),
            Cut::below(ThighR, ShinR, 14.5),
            Cut::below(ShinR, FootR, 4.5),
            Cut::boxed(ShinR, FootR, [-INF, -INF, 24.5], [INF, 8.5, INF]),
        ],
    }
};

/// Mehmed's joints (his grid is 96 × 124 × 40, `heroes::MEHMED_SIZE`). His waist is the top of
/// his sash (y 35.5), his chest from the frogging up (46); his neck is the column of skin behind his
/// beard (y 56 to 60), the beard, the börk and its flap go with his head. His wide sleeves bend at
/// the elbow (41.5), and their gold cuffs stay on the forearm, the fist below the wrist (28.5).
///
/// His kaftan's skirt (and the crimson robe under it, and the sash's hanging end) is cut down the
/// middle below the sash (y 29.5), each half riding its thigh: his stride shows from behind as the
/// skirt parts and swings, where a skirt kept on his hips would hide his legs and slide over his
/// boots. Under the skirt only his boots are sculpted: the knee (14.5) is hidden, so it is cut
/// unsleeved (a sleeve there would lay a band of the skirt on his shin), and the shin is the boot
/// from the hem down to the ankle (5.5), the foot the boot below it and its turned-up toe.
pub const MEHMED: Joints = {
    use Bone::*;
    let (l_arm, r_arm, l_leg, r_leg, mid) = (21.5, 73.5, 39.5, 55.5, 47.5);
    Joints {
        def: &HUMANOID,
        from: HERO_PARTS,
        mirrored: true,
        fists: true,
        at: &[
            [mid, 30.0, 19.5],
            [mid, 35.5, 19.5],
            [mid, 46.0, 19.5],
            [mid, 56.5, 18.5],
            [mid, 60.5, 18.5],
            [l_arm, 54.0, 18.5],
            [l_arm, 41.5, 19.2],
            [l_arm, 28.5, 19.5],
            [r_arm, 54.0, 18.5],
            [r_arm, 41.5, 19.2],
            [r_arm, 28.5, 19.5],
            [l_leg, 28.0, 20.5],
            [l_leg, 14.5, 21.0],
            [l_leg, 5.5, 21.5],
            [r_leg, 28.0, 20.5],
            [r_leg, 14.5, 21.0],
            [r_leg, 5.5, 21.5],
            // The grip in each fist (where `armory::weapon_for` leans the weapon from), and the middle
            // of the shield forearm where the kalkan is strapped (`forge::forearm`).
            [r_arm, 26.0, 19.5],
            [l_arm, 26.0, 19.5],
            [l_arm, 36.0, 19.5],
        ],
        cuts: &[
            Cut::above(Hips, Spine, 35.5),
            Cut::above(Spine, Chest, 46.0),
            // The neck: the column of skin behind the beard.
            Cut::boxed(Head, Neck, [42.5, -INF, 13.0], [52.5, 60.5, 19.4]),
            Cut::below(UpperArmL, ForearmL, 41.5),
            Cut::below(ForearmL, HandL, 28.5),
            Cut::below(UpperArmR, ForearmR, 41.5),
            Cut::below(ForearmR, HandR, 28.5),
            // (The boots, before the skirt joins the thighs: the knee's cut is not to take the skirt.)
            Cut::below(ThighL, ShinL, 14.5).bare(),
            Cut::below(ShinL, FootL, 5.5),
            Cut::boxed(ShinL, FootL, [-INF, -INF, 27.5], [INF, 8.5, INF]),
            Cut::below(ThighR, ShinR, 14.5).bare(),
            Cut::below(ShinR, FootR, 5.5),
            Cut::boxed(ShinR, FootR, [-INF, -INF, 27.5], [INF, 8.5, INF]),
            // The skirt's halves.
            Cut::below_within(Hips, ThighL, 29.5, [-INF, -INF], [mid, INF]),
            Cut::below_within(Hips, ThighR, 29.5, [mid, -INF], [INF, INF]),
        ],
    }
};

/// Miyamoto's joints (his grid is 96 × 120 × 44, `heroes::MIYAMOTO_SIZE`). His waist is the top of
/// his obi (y 37.5), the short sword thrust through it staying with his hips; his chest is the dō from
/// the middle of its lames up (48); his neck is the lacquered throat guard (y 54 to 59), and the
/// kabuto, its neck guard and crest go with his head. His shoulder guards (sode) hang over the upper
/// arms and swing with them, as sculpted; the elbow is below them (37, its sleeve two rows of silk
/// and mail, none of the lames), the gauntlet and fist below the wrist (29.5).
///
/// His armoured skirt (kusazuri) is cut between its top lame and the next (y 29.5): the top lame
/// stays on his hips under the obi, the lower lames and the seat of his hakama are cut down the
/// middle, each half riding its thigh, so the panels part and swing with his stride. The knee is at
/// the top of his shin guards (15.5), its sleeve kept to the leg; the foot is the tabi and sandal.
///
/// His shield is not strapped to his arm: it hangs on his back while he fights two-handed (the
/// shield's joint is its middle there), and the animator brings it before him to guard
/// (`anim::hold`).
pub const MIYAMOTO: Joints = {
    use Bone::*;
    let (l_arm, r_arm, l_leg, r_leg, mid) = (21.5, 73.5, 39.5, 55.5, 47.5);
    Joints {
        def: &HUMANOID,
        from: HERO_PARTS,
        mirrored: true,
        fists: true,
        at: &[
            [mid, 30.0, 21.5],
            [mid, 37.5, 21.5],
            [mid, 48.0, 21.5],
            [mid, 54.5, 21.0],
            [mid, 60.0, 20.5],
            [l_arm, 52.0, 19.5],
            [l_arm, 37.0, 20.0],
            [l_arm, 29.5, 20.5],
            [r_arm, 52.0, 19.5],
            [r_arm, 37.0, 20.0],
            [r_arm, 29.5, 20.5],
            [l_leg, 29.0, 23.0],
            [l_leg, 15.5, 24.0],
            [l_leg, 5.5, 24.5],
            [r_leg, 29.0, 23.0],
            [r_leg, 15.5, 24.0],
            [r_leg, 5.5, 24.5],
            [r_arm, 26.0, 19.5],
            [l_arm, 26.0, 19.5],
            // The middle of the shield slung on his back (`heroes::MIYAMOTO_SHIELD_AT`).
            [mid, 42.5, 10.0],
        ],
        cuts: &[
            Cut::above(Hips, Spine, 37.5),
            // The short sword's hilt, above the obi, stays with the scabbard on his hips.
            Cut::boxed(Spine, Hips, [30.0, 35.0, 32.5], [39.0, 42.0, 42.0]),
            Cut::above(Spine, Chest, 48.0),
            Cut::boxed(Head, Neck, [37.0, -INF, 12.0], [58.0, 59.5, 31.0]),
            Cut::below(UpperArmL, ForearmL, 37.0),
            Cut::below(ForearmL, HandL, 29.5),
            Cut::below(UpperArmR, ForearmR, 37.0),
            Cut::below(ForearmR, HandR, 29.5),
            Cut::below_within(ThighL, ShinL, 15.5, [l_leg - 7.0, -INF], [l_leg + 7.0, INF]),
            Cut::below(ShinL, FootL, 5.5),
            Cut::below_within(ThighR, ShinR, 15.5, [r_leg - 7.0, -INF], [r_leg + 7.0, INF]),
            Cut::below(ShinR, FootR, 5.5),
            Cut::below_within(Hips, ThighL, 29.5, [-INF, -INF], [mid, INF]),
            Cut::below_within(Hips, ThighR, 29.5, [mid, -INF], [INF, INF]),
        ],
    }
};

/// A hero's joint table.
pub fn joints(hero: HeroKind) -> Option<&'static Joints> {
    Some(match hero {
        HeroKind::Ragnar => &RAGNAR,
        HeroKind::Mehmed => &MEHMED,
        HeroKind::Miyamoto => &MIYAMOTO,
    })
}

/// Where a joint of the sculpt's grid lies in the model (as `voxels::make_part` places a pivot,
/// the rig turned left for right as `forge::hero_rigs` turns it).
pub fn joint_in_model(g: [f32; 3], width: i32, depth: i32, size: f32) -> Vec3 {
    joint_at(g, width, depth, size, true)
}

/// Where a joint of the sculpt's grid lies in the model, the rig mirrored from the sculpt or not.
pub fn joint_at(g: [f32; 3], width: i32, depth: i32, size: f32, mirrored: bool) -> Vec3 {
    let (ox, oz) = ((width - 1) as f32 / 2.0, (depth - 1) as f32 / 2.0);
    let x = if mirrored { ox - g[0] } else { g[0] - ox };
    Vec3::new(x * size, g[1] * size, (g[2] - oz) * size)
}

/// A point of the model (a voxel's centre) in the sculpt's grid.
fn grid_of(at: Vec3, width: i32, depth: i32, size: f32, mirrored: bool) -> [f32; 3] {
    let (ox, oz) = ((width - 1) as f32 / 2.0, (depth - 1) as f32 / 2.0);
    let gx = if mirrored { ox - at.x / size } else { ox + at.x / size };
    [gx, at.y / size - 0.5, at.z / size + oz]
}

/// Which bone each voxel of `rig` goes to, part by part (the cut, without building anything).
pub fn assign(rig: &Rig, joints: &Joints, width: i32, depth: i32) -> Vec<Vec<usize>> {
    let size = rig.size;
    rig.parts
        .iter()
        .map(|part| {
            let start = joints.from.iter().find(|(n, _)| *n == part.name).map(|(_, b)| *b).unwrap_or(0);
            // A held weapon is modelled with the fist round its grip: the fist stays with the hand,
            // so a turn of the weapon in it turns the weapon alone.
            let fist = match start {
                b if joints.fists && b == Bone::Blade.index() => Some(Bone::HandR.index()),
                b if joints.fists && b == Bone::Offhand.index() => Some(Bone::HandL.index()),
                _ => None,
            };
            part.cubes
                .iter()
                .enumerate()
                .map(|(j, (off, _))| {
                    let shade = part.shade.get(j).copied().unwrap_or(0.0);
                    if let (Some(hand), true) = (fist, shade == SKIN_SHADE) {
                        return hand;
                    }
                    let g = grid_of(part.rest + *off, width, depth, size, joints.mirrored);
                    let mut bone = start;
                    for cut in joints.cuts {
                        if cut.from == bone && cut.holds(g, shade) {
                            bone = cut.to;
                        }
                    }
                    bone
                })
                .collect()
        })
        .collect()
}

/**
 * The rig cut into its skeleton's bones: each voxel of `rig` (the code's rig, its parts and any
 * layered weapons and shield) goes to its bone by `joints`, keeping its place, colour and material;
 * each bone's joint is where the table puts it, and its parent is the skeleton's. The parts come in
 * the order of the skeleton's bones. `width` and `depth` are the sculpt's grid, `rig.size` its
 * voxel.
 */
pub fn jointed(rig: &Rig, joints: &Joints, width: i32, depth: i32) -> Rig {
    let def = joints.def;
    let n = def.len();
    assert_eq!(joints.at.len(), n, "{}: a joint for every bone", def.name);
    let size = rig.size;
    let rest: Vec<Vec3> = joints.at.iter().map(|&g| joint_at(g, width, depth, size, joints.mirrored)).collect();
    let mut cubes: Vec<Vec<(Vec3, Vec3)>> = vec![Vec::new(); n];
    let mut shade: Vec<Vec<f32>> = vec![Vec::new(); n];
    for (part, bones) in rig.parts.iter().zip(assign(rig, joints, width, depth)) {
        for (j, ((off, c), bone)) in part.cubes.iter().zip(bones).enumerate() {
            let at = part.rest + *off;
            let m = part.shade.get(j).copied().unwrap_or(0.0);
            cubes[bone].push((at - rest[bone], *c));
            shade[bone].push(m);
            // The sleeve: a band of the bone above a joint across its limb is laid on the bone below
            // as well, so a bend overlaps itself rather than opening a gap at the crease.
            let g0 = grid_of(at, width, depth, size, joints.mirrored);
            for cut in joints.cuts {
                let Some((plane, down)) = cut.crease() else { continue };
                let a = cut.axis;
                let g = cut.measure(g0);
                let inside = (0..3).filter(|&k| k != a).all(|k| g[k] >= cut.lo[k] && g[k] <= cut.hi[k]);
                let near = if down { (plane..=plane + SLEEVE).contains(&g[a]) } else { (plane - SLEEVE..=plane).contains(&g[a]) };
                if cut.from == bone && inside && near && cut.takes(m) {
                    let to = cut.to;
                    cubes[to].push((at - rest[to], *c));
                    shade[to].push(m);
                }
            }
        }
    }
    let parts = def
        .bones
        .iter()
        .enumerate()
        .zip(cubes.into_iter().zip(shade))
        .map(|((i, &name), (cubes, shade))| {
            let min_y = cubes.iter().map(|(p, _)| p.y - size / 2.0).fold(f32::INFINITY, f32::min);
            RigPart { name, rest: rest[i], cubes, shade, min_y: if min_y.is_finite() { min_y } else { 0.0 }, parent: def.parents[i] }
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
    // (Past twenty, each colour again a little darker, so a long chain's neighbours still differ.)
    crate::render::voxels::lin(COLORS[i % BONE_COUNT]) * if (i / BONE_COUNT).is_multiple_of(2) { 1.0 } else { 0.55 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::voxels::build_layered_rig;

    const HEROES: [HeroKind; 3] = [HeroKind::Ragnar, HeroKind::Mehmed, HeroKind::Miyamoto];

    /// A hero's code rig as `forge::hero_rigs` builds it for the play (mirrored), from his bare
    /// sculpt, and his grid's width, depth and height.
    fn code_rig(hero: HeroKind) -> (Rig, i32, i32, i32) {
        use crate::render::heroes;
        let (body, defs) = match hero {
            HeroKind::Ragnar => (heroes::ragnar_hd(None), heroes::RAGNAR_HD_RIG),
            HeroKind::Mehmed => (heroes::mehmed_hd(None), heroes::MEHMED_HD_RIG),
            HeroKind::Miyamoto => (heroes::miyamoto_hd(None), heroes::MIYAMOTO_HD_RIG),
        };
        let (w, d, h) = (body.width, body.depth, body.height);
        let mut c = body;
        for v in c.voxels.iter_mut() {
            v.x = w - 1 - v.x;
        }
        let defs: Vec<_> = defs
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
        (rig, w, d, h)
    }

    /// A hero's code rig, cut, and his grid's height.
    fn cut_of(hero: HeroKind) -> (Rig, Rig, i32) {
        let (rig, w, d, h) = code_rig(hero);
        let cut = jointed(&rig, joints(hero).unwrap(), w, d);
        (rig, cut, h)
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
        for hero in HEROES {
            let (rig, cut, _) = cut_of(hero);
            assert_eq!(cut.parts.len(), BONE_COUNT);
            let count = |r: &Rig| r.parts.iter().map(|p| p.cubes.len()).sum::<usize>();
            // Every voxel stays where it was, as the code rig's rest pose has it; the only ones added
            // are the sleeves' copies, each where a voxel already is.
            let mut before: Vec<(i64, i64, i64)> =
                rig.parts.iter().flat_map(|p| p.cubes.iter().map(move |(o, _)| p.rest + *o)).map(key).collect();
            let mut after: Vec<(i64, i64, i64)> =
                cut.parts.iter().flat_map(|p| p.cubes.iter().map(move |(o, _)| p.rest + *o)).map(key).collect();
            before.sort_unstable();
            after.sort_unstable();
            let extra = count(&cut) - count(&rig);
            assert!(extra > 0 && extra < count(&rig) / 5, "{hero:?}: the sleeves add {extra} of {}", count(&rig));
            after.dedup();
            before.dedup();
            assert_eq!(before, after, "{hero:?}");
            // Every bone of his body has voxels (the weapons and shield are empty here).
            for (i, p) in cut.parts.iter().enumerate().take(Bone::Blade.index()) {
                assert!(p.cubes.len() > 20, "{hero:?}: {} has {} voxels", BONES[i], p.cubes.len());
                assert_eq!(p.parent, PARENTS[i]);
            }
        }
    }

    fn key(p: Vec3) -> (i64, i64, i64) {
        let k = (p / 0.0125 * 2.0).round();
        (k.x as i64, k.y as i64, k.z as i64)
    }

    #[test]
    fn no_voxel_is_stranded_on_another_limb() {
        for hero in HEROES {
            let (_, cut, h) = cut_of(hero);
            let size = cut.size;
            // (Reaches in Ragnar's voxels, scaled to the hero's height.)
            let scale = h as f32 / crate::render::heroes::RAGNAR_SIZE.1 as f32;
            // A skirt's halves ride the thighs, reaching out and down past the knee.
            let skirt = match hero {
                HeroKind::Ragnar => None,
                HeroKind::Mehmed => Some(30.0),
                HeroKind::Miyamoto => Some(24.0),
            };
            for (i, part) in cut.parts.iter().enumerate() {
                for (off, _) in &part.cubes {
                    // Nothing on a limb lies further than its length from its joint.
                    let reach = match (i, skirt) {
                        (0..=2, _) => 40.0 * scale,
                        (4, _) => 45.0 * scale,
                        (11 | 14, Some(r)) => r,
                        _ => 16.0 * scale,
                    } * size;
                    let g = (part.rest + *off) / size;
                    assert!(off.length() <= reach, "{hero:?}: {} has a voxel {:.3} from its joint (at {g})", BONES[i], off.length());
                }
            }
            // Down each limb, a bone's voxels lie above the next bone's joint (but a skirt's).
            for (a, b) in [(5, 6), (6, 7), (8, 9), (9, 10), (11, 12), (12, 13), (14, 15), (15, 16)] {
                if hero == HeroKind::Mehmed && (a == 11 || a == 14) {
                    continue;
                }
                let (up, down) = (&cut.parts[a], &cut.parts[b]);
                let joint = down.rest.y - up.rest.y;
                assert!(up.cubes.iter().all(|(o, _)| o.y >= joint - size * 0.6), "{hero:?}: {} has a voxel below {}", BONES[a], BONES[b]);
            }
            // The sides keep to their own side of him: the left limbs at +x, the right at -x (the
            // skirt's halves too).
            for (name, sign) in [("hand.L", 1.0), ("hand.R", -1.0), ("foot.L", 1.0), ("foot.R", -1.0), ("thigh.L", 1.0), ("thigh.R", -1.0)]
            {
                let p = &cut.parts[cut.index(name).unwrap()];
                assert!(p.cubes.iter().all(|(o, _)| (p.rest.x + o.x) * sign > 0.0), "{hero:?}: {name} reaches across");
            }
        }
    }

    #[test]
    fn the_sleeves_lie_at_their_joints() {
        // A sleeve's voxel is a copy of one of the bone above a joint, laid on the bone below it
        // within a band of the joint: never on another limb, never far from the crease.
        for hero in HEROES {
            let (_, cut, _) = cut_of(hero);
            let size = cut.size;
            let mut owners: std::collections::HashMap<(i64, i64, i64), Vec<usize>> = std::collections::HashMap::new();
            for (i, p) in cut.parts.iter().enumerate() {
                for (o, _) in &p.cubes {
                    owners.entry(key(p.rest + *o)).or_default().push(i);
                }
            }
            let mut sleeves = 0;
            for (k, bones) in &owners {
                if bones.len() < 2 {
                    continue;
                }
                sleeves += 1;
                assert_eq!(bones.len(), 2, "{hero:?}: a voxel in {} bones", bones.len());
                let (a, b) = (bones[0].min(bones[1]), bones[0].max(bones[1]));
                assert_eq!(PARENTS[b], Some(a), "{hero:?}: a sleeve between {} and {}", BONES[a], BONES[b]);
                let y = k.1 as f32 * size / 2.0;
                let joint = cut.parts[b].rest.y;
                assert!(
                    (y - joint).abs() <= (SLEEVE + 3.0) * size,
                    "{hero:?}: a sleeve of {} {:.1} voxels from its joint",
                    BONES[b],
                    (y - joint) / size
                );
            }
            assert!(sleeves > 100, "{hero:?}: {sleeves} sleeve voxels");
            // Mehmed's skirt never rides his shins (their knee is unsleeved, under the skirt).
            if hero == HeroKind::Mehmed {
                for shin in ["shin.L", "shin.R"] {
                    let p = &cut.parts[cut.index(shin).unwrap()];
                    assert!(p.cubes.iter().all(|(o, _)| p.rest.y + o.y < 8.0 * size), "{shin} carries the skirt");
                }
            }
        }
    }

    #[test]
    fn every_hero_has_a_table_and_the_code_rigs_stay_flat() {
        for hero in HEROES {
            assert!(joints(hero).is_some());
        }
        let rig = crate::render::voxels::build_rig(&crate::render::characters::ragnar(), crate::render::characters::RAGNAR_RIG, 0.05);
        assert!(rig.parts.iter().all(|p| p.parent.is_none()), "the code's rigs have no parents");
    }
}
