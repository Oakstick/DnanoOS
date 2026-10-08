//! The story's cast on skeletons (ANIMATION.md §12): the four friends and the old gatekeeper on the
//! heroes' twenty bones (so Mixamo's motion retargets onto them as onto the heroes), with a bone or
//! two of their own for what swings (a boy's scarf, the old man's beard); Quitmer, his dog, on the
//! wolves' skeleton; and Scene I's Krampus on the lair's Krampus's bones, so his clips play on him.
//! Each table is measured on its sculpt (`render::cast`), as the heroes' are, and cuts the code's rig
//! into its bones. The cast is not mirrored: its bones are named by the side they lie on (`.L` at
//! +x, the figure's own left as it faces +z), the code rig's `A` parts (low x) its right.

use crate::render::cast;
use crate::render::characters::mat;
use crate::render::voxels::{build_rig, PartDef, Rig};

use super::foe_beasts::QUADRUPED;
use super::foe_build::Build;
use super::foe_courts::{arms, legs, mirror, torso};
use super::skeleton::{Bone, Cut, Joints};

const INF: f32 = f32::INFINITY;

/// One of the cast as its clips know it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum CastRig {
    /// The four friends (0 is the one Krampus takes): one sculpt's body, four outfits.
    Kid(u8),
    OldMan,
    Dog,
    /// Krampus as Scene I shows him, out of the fog.
    Krampus,
}

impl CastRig {
    pub const ALL: [CastRig; 7] =
        [CastRig::Kid(0), CastRig::Kid(1), CastRig::Kid(2), CastRig::Kid(3), CastRig::OldMan, CastRig::Dog, CastRig::Krampus];

    /// Its name for `--export-rig` (`kid0` to `kid3`, `oldman`, `dog`, `krampus_shade`).
    pub fn name(self) -> String {
        match self {
            CastRig::Kid(k) => format!("kid{k}"),
            CastRig::OldMan => "oldman".into(),
            CastRig::Dog => "dog".into(),
            CastRig::Krampus => "krampus_shade".into(),
        }
    }

    /// The folder of its clips (`assets/anim/<dir>/`): the four boys share theirs (one body).
    pub fn dir(self) -> &'static str {
        match self {
            CastRig::Kid(_) => "kid",
            CastRig::OldMan => "oldman",
            CastRig::Dog => "dog",
            CastRig::Krampus => "krampus_shade",
        }
    }

    /// The rig named `name`, as `--export-rig` takes it.
    pub fn named(name: &str) -> Option<CastRig> {
        CastRig::ALL.into_iter().find(|c| c.name() == name)
    }

    /// Its slot among the cast (0 to 6).
    pub fn slot(self) -> usize {
        match self {
            CastRig::Kid(k) => (k as usize).min(3),
            CastRig::OldMan => 4,
            CastRig::Dog => 5,
            CastRig::Krampus => 6,
        }
    }

    pub fn joints(self) -> &'static Joints {
        static BUILT: [std::sync::OnceLock<&'static Joints>; 4] = [const { std::sync::OnceLock::new() }; 4];
        let (i, f): (usize, fn() -> &'static Joints) = match self {
            CastRig::Kid(_) => (0, kid),
            CastRig::OldMan => (1, old_man),
            CastRig::Dog => (2, dog),
            CastRig::Krampus => (3, krampus),
        };
        BUILT[i].get_or_init(f)
    }

    /// Its sculpt's grid (width, height, depth) and voxel, as `view::models` builds its rig.
    pub fn grid(self) -> ((i32, i32, i32), f32) {
        match self {
            CastRig::Kid(_) => (cast::KID_SIZE, cast::KID_VOXEL),
            CastRig::OldMan => (cast::OLD_MAN_SIZE, cast::OLD_MAN_VOXEL),
            CastRig::Dog => (cast::DOG_SIZE, cast::DOG_VOXEL),
            CastRig::Krampus => (cast::KRAMPUS_SIZE, cast::KRAMPUS_VOXEL),
        }
    }

    /// The code's rig from the sculpt finished solid (a boy's with his scarf's end a part of its own).
    pub fn code_rig(self) -> Rig {
        use crate::render::characters::solid;
        let (_, size) = self.grid();
        solid(|| match self {
            CastRig::Kid(k) => {
                let mut defs: Vec<PartDef> =
                    cast::KID_RIG.iter().map(|d| PartDef { name: d.name, boxes: d.boxes, pivot: d.pivot }).collect();
                defs.push(PartDef { name: "scarf", boxes: &[], pivot: SCARF });
                build_rig(&cast::kid(k as usize), &defs, size)
            }
            CastRig::OldMan => build_rig(&cast::old_man(), cast::OLD_MAN_RIG, size),
            CastRig::Dog => build_rig(&cast::dog(), cast::DOG_RIG, size),
            CastRig::Krampus => build_rig(&cast::krampus_shade(), cast::KRAMPUS_RIG, size),
        })
    }

    /// It cut into its skeleton's bones (`VH_ANIM=bones` paints each bone its colour).
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

/// Where a boy's scarf's end hangs from, under its knot (his grid).
const SCARF: [f32; 3] = [13.5, 31.5, 20.5];

/**
 * A boy's joints (grid 36 × 64 × 26, `cast::KID_SIZE`; x 17.5 and z 12.5 his middle). His hips are
 * the skirt of his coat (to y 25), his waist its belt (26 to 27), his chest the rest of the coat, his
 * shoulders and the scarf wound round his neck; his neck is the column of skin under his head (y 34
 * to 37), his head and cap above it. The sleeves bend at the elbow (27), the cuffs stay on the
 * forearm, the mitten or hand below the wrist (19.5). His breeches are the thigh to the knee (12.5),
 * the socks and the boot's shaft the shin, the boot below the ankle (4.5) and its toe the foot. The
 * striped end of his scarf hangs from the knot on its own bone (`scarf`), so it can swing.
 */
fn kid() -> &'static Joints {
    let (cx, cz) = (17.5, 12.5);
    let mut b = Build::humanoid();
    b.part("legA", Bone::ThighR.index()).part("legB", Bone::ThighL.index());
    b.part("armA", Bone::UpperArmR.index()).part("armB", Bone::UpperArmL.index());
    b.at(Bone::Hips, [cx, 21.0, cz]).at(Bone::Spine, [cx, 25.5, cz]).at(Bone::Chest, [cx, 30.5, cz]);
    b.at(Bone::Neck, [cx, 34.0, cz + 0.5]).at(Bone::Head, [cx, 37.5, cz + 0.5]);
    b.cut(Cut::up(Bone::Hips.index(), Bone::Spine.index(), 25.5)).cut(Cut::up(Bone::Spine.index(), Bone::Chest.index(), 30.5));
    b.cut(Cut::cube(Bone::Head.index(), Bone::Neck.index(), [12.5, -INF, 7.0], [22.5, 37.4, 18.5]));
    let arm_l = [[30.5, 32.0, 12.5], [30.7, 27.0, 12.8], [30.8, 19.5, 13.3], [30.8, 14.0, 13.5]];
    // (Straight down from the shoulder: cut level at the elbow and the wrist.)
    for (arm, s) in [(arm_l, 1.0f32), (mirror(&arm_l, cx).try_into().unwrap_or(arm_l), -1.0)] {
        let (up, fore, hand) =
            if s > 0.0 { (Bone::UpperArmL, Bone::ForearmL, Bone::HandL) } else { (Bone::UpperArmR, Bone::ForearmR, Bone::HandR) };
        b.at(up, arm[0]).at(fore, arm[1]).at(hand, arm[2]);
        b.cut(Cut::down(up.index(), fore.index(), arm[1][1])).cut(Cut::down(fore.index(), hand.index(), arm[2][1]));
    }
    b.at(Bone::Offhand, [30.8, 17.0, 13.4]).at(Bone::Blade, [4.2, 17.0, 13.4]).at(Bone::Shield, [30.6, 24.0, 13.0]);
    for (x, thigh, shin, foot) in [(23.0, Bone::ThighL, Bone::ShinL, Bone::FootL), (12.0, Bone::ThighR, Bone::ShinR, Bone::FootR)] {
        b.at(thigh, [x, 20.0, cz]).at(shin, [x, 12.5, 12.4]).at(foot, [x, 4.5, 12.4]);
        b.cut(Cut::down(thigh.index(), shin.index(), 12.5)).cut(Cut::down(shin.index(), foot.index(), 4.5));
        b.cut(Cut::cube(shin.index(), foot.index(), [-INF, -INF, 16.5], [INF, 8.5, INF]));
    }
    let scarf = b.bone("scarf", Bone::Chest.index(), SCARF);
    b.part("scarf", scarf);
    b.finish("kid", false)
}

/**
 * The old man's joints (grid 60 × 104 × 40, `cast::OLD_MAN_SIZE`; x 29.5 his middle, stooped
 * forward a little higher up). His hips the rope belt (y 34), his chest from the middle of his back
 * (52) with the cape over his shoulders; his neck the skin under the hood (only skin, not the beard
 * before it), his head the hood, the face and the moustache. His beard hangs from his chin in two
 * (`beard.1` below y 63.5, `beard.2` below 49.5), so it can sway. The robe below the belt is one
 * garment on bones of its own (`robe`, and `robe.hem` below the knee), his legs under it empty but
 * for the sandals and toes, the feet. His staff arm (his right, at low x) is bent, its hand before him: cut square to the bend;
 * the staff (its wood and leather wraps) is the `blade`, its joint in his fist. His free arm hangs.
 */
fn old_man() -> &'static Joints {
    let cx = 29.5;
    let mut b = Build::humanoid();
    b.part("staffArm", Bone::UpperArmR.index()).part("freeArm", Bone::UpperArmL.index());
    b.cuts_first(&[Cut::cube(Bone::UpperArmR.index(), Bone::Blade.index(), [-INF; 3], [INF; 3]).only(&[mat::WOOD, mat::LEATHER])]);
    torso(&mut b, [cx, 34.0, 20.0], [cx, 42.0, 20.5], [cx, 52.0, 21.0], [cx, 60.0, 21.5], [cx, 64.0, 22.0]);
    b.cut(Cut::cube(Bone::Head.index(), Bone::Neck.index(), [24.0, -INF, 15.0], [35.0, 63.5, 27.0]).only(&[mat::SKIN]));
    let arm_r = [[15.5, 59.0, 21.0], [12.6, 52.0, 24.5], [10.6, 47.0, 30.5], [10.2, 41.0, 34.5]];
    let arm_l = [[43.5, 59.0, 21.0], [46.6, 50.5, 22.3], [47.6, 42.0, 23.2], [47.6, 35.0, 24.0]];
    arms(&mut b, &arm_l, &arm_r);
    b.at(Bone::Blade, [10.2, 45.0, 34.0]);
    // His legs are hidden under the robe: only the sandals and toes peeping out are theirs (the feet).
    for (x, thigh, shin, foot, (x0, x1)) in
        [(36.0, Bone::ThighL, Bone::ShinL, Bone::FootL, (cx, INF)), (23.0, Bone::ThighR, Bone::ShinR, Bone::FootR, (-INF, cx))]
    {
        b.at(thigh, [x, 33.0, 21.0]).at(shin, [x, 17.0, 22.5]).at(foot, [x, 3.5, 27.0]);
        b.cut(Cut::cube(Bone::Hips.index(), foot.index(), [x0, -INF, 24.5], [x1, 4.5, INF]).only(&[mat::LEATHER, mat::SKIN]));
    }
    // The robe below the belt one garment, hanging from his hips in two (to the knee, and the hem
    // below it), swinging after him (`view::cast_clips`), never parted by his legs.
    let robe = b.bone("robe", Bone::Hips.index(), [cx, 33.5, 20.5]);
    let hem = b.bone("robe.hem", robe, [cx, 16.0, 20.5]);
    b.cut(Cut::down(Bone::Hips.index(), robe, 33.5));
    b.cut(Cut::down(robe, hem, 16.0));
    let beard1 = b.bone("beard.1", Bone::Head.index(), [cx, 63.5, 28.0]);
    let beard2 = b.bone("beard.2", beard1, [cx, 49.5, 30.0]);
    b.cut(Cut::cube(Bone::Head.index(), beard1, [-INF, -INF, 22.0], [INF, 63.5, INF]).only(&[mat::FUR]));
    b.cut(Cut::down(beard1, beard2, 49.5));
    b.finish("oldman", false)
}

/**
 * Quitmer's joints (grid 48 × 72 × 116, `cast::DOG_SIZE`; x 23.5 his middle, his nose at high z).
 * On the wolves' skeleton: his hips the rump and its hams, his chest from z 62 (the barrel's front,
 * the deep chest and the shoulders), his neck and its spiked collar from z 86 (above the chest's
 * underside), his head the code's (the skull, the mask and the cropped ears), the lower jaw its own.
 * His tail curls up from the rump: cut at z 11 and at y 50. Each leg below the body: the forelegs at
 * the elbow (14) and the pastern (5.5), the hind legs at the hock (12) and the pastern (4.6).
 */
fn dog() -> &'static Joints {
    use super::foe_beasts::quad::*;
    let cx = 23.5;
    let (l, r) = (cx + 9.6, cx - 9.6);
    let mut b = Build::new("hips", [cx, 34.0, 30.0]);
    let names = QUADRUPED.bones;
    let parents = QUADRUPED.parents;
    let at: [[f32; 3]; 20] = [
        [cx, 34.0, 30.0],
        [cx, 36.0, 62.0],
        [cx, 44.0, 86.0],
        [cx, 48.0, 90.0],
        [cx, 44.0, 98.0],
        [cx, 41.0, 20.0],
        [cx, 45.0, 11.0],
        [cx, 51.0, 5.0],
        [l, 26.0, 79.5],
        [l, 14.0, 80.0],
        [l, 5.5, 82.0],
        [r, 26.0, 79.5],
        [r, 14.0, 80.0],
        [r, 5.5, 82.0],
        [l, 21.0, 26.5],
        [l, 12.0, 23.5],
        [l, 4.6, 28.4],
        [r, 21.0, 26.5],
        [r, 12.0, 23.5],
        [r, 4.6, 28.4],
    ];
    for i in 1..names.len() {
        b.bone(names[i], parents[i].unwrap_or(0), at[i]);
    }
    for (part, bone) in [("jaw", JAW), ("head", HEAD), ("foreA", UF_R), ("foreB", UF_L), ("hindA", UB_R), ("hindB", UB_L), ("tail", TAIL1)]
    {
        b.part(part, bone);
    }
    b.cut(Cut::plane(HIPS, CHEST, 2, 62.0, false));
    b.cut(Cut::plane(CHEST, NECK, 2, 86.0, false).across([-INF, 37.0, -INF], [INF, INF, INF]));
    b.cut(Cut::plane(TAIL1, TAIL2, 2, 11.0, true));
    b.cut(Cut::plane(TAIL2, TAIL3, 1, 50.0, false));
    for (up, low, paw, knee, ankle) in
        [(UF_L, LF_L, PF_L, 14.0, 5.5), (UF_R, LF_R, PF_R, 14.0, 5.5), (UB_L, LB_L, PB_L, 12.0, 4.6), (UB_R, LB_R, PB_R, 12.0, 4.6)]
    {
        b.cut(Cut::down(up, low, knee)).cut(Cut::down(low, paw, ankle));
    }
    b.finish("dog", false)
}

/**
 * Scene I's Krampus (grid 92 × 128 × 84, `cast::KRAMPUS_SIZE`; x 45.5 his middle): the lair's
 * beast at the size the boys first see him, sculpted the same (the lair's is cut on the same planes,
 * `foe_courts::krampus`), so his foe clips play on him. His basket and its birches ride his chest,
 * his long arms hang to his clawed hands, his goat's legs bend at the hock. His fists hold nothing
 * (the `blade` and `offhand` empty), nor has he a sack (its bone empty).
 */
fn krampus() -> &'static Joints {
    let cx = 45.5;
    let mut b = Build::humanoid();
    b.part("armR", Bone::UpperArmR.index()).part("armL", Bone::UpperArmL.index());
    b.part("legR", Bone::ThighR.index()).part("legL", Bone::ThighL.index());
    torso(&mut b, [cx, 44.0, 38.0], [cx, 58.0, 39.0], [cx, 70.0, 42.0], [cx, 80.0, 46.0], [cx, 82.0, 48.0]);
    let arm_l = [[cx + 21.0, 81.0, 44.0], [cx + 26.0, 66.0, 50.0], [cx + 28.0, 51.0, 57.0], [cx + 28.0, 40.0, 64.0]];
    arms(&mut b, &arm_l, &mirror(&arm_l, cx));
    legs(&mut b, cx, &[[cx + 10.0, 45.0, 40.0], [cx + 10.0, 25.0, 32.5], [cx + 10.0, 6.0, 43.0], [cx + 10.0, 1.0, 50.0]]);
    b.bone("sack", Bone::Hips.index(), [cx - 17.0, 57.0, 38.0]);
    b.finish("krampus", false)
}

/// Where a point of a cast member's grid lies in its model (unmirrored, as the cast's rigs are).
pub fn grid_point(c: CastRig, g: [f32; 3]) -> glam::Vec3 {
    let ((w, _, d), size) = c.grid();
    super::skeleton::joint_at(g, w, d, size, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn key(p: Vec3, size: f32) -> (i64, i64, i64) {
        let k = (p / size * 2.0).round();
        (k.x as i64, k.y as i64, k.z as i64)
    }

    #[test]
    fn every_cast_rig_is_cut_whole_and_in_place() {
        for c in CastRig::ALL {
            let code = c.code_rig();
            let cut = c.jointed();
            let def = c.joints().def;
            assert_eq!(cut.parts.len(), def.len(), "{c:?}");
            let size = code.size;
            let mut before: Vec<_> = code.parts.iter().flat_map(|p| p.cubes.iter().map(move |(o, _)| key(p.rest + *o, size))).collect();
            let mut after: Vec<_> = cut.parts.iter().flat_map(|p| p.cubes.iter().map(move |(o, _)| key(p.rest + *o, size))).collect();
            before.sort_unstable();
            before.dedup();
            after.sort_unstable();
            after.dedup();
            assert_eq!(before, after, "{c:?}: every voxel where it was");
            for (i, p) in cut.parts.iter().enumerate() {
                assert_eq!(p.parent, def.parents[i], "{c:?} {}", def.bones[i]);
            }
        }
    }

    #[test]
    fn the_cast_keeps_its_limbs_to_their_sides_and_what_swings_on_its_own_bones() {
        let side = |c: CastRig, name: &str, sign: f32| {
            let cut = c.jointed();
            let p = &cut.parts[cut.index(name).unwrap()];
            assert!(!p.cubes.is_empty(), "{c:?}: {name} is empty");
            assert!(p.cubes.iter().all(|(o, _)| (p.rest.x + o.x) * sign > 0.0), "{c:?}: {name} reaches across");
        };
        for c in [CastRig::Kid(0), CastRig::Kid(3), CastRig::OldMan, CastRig::Krampus] {
            for (n, s) in [("hand.L", 1.0), ("hand.R", -1.0), ("foot.L", 1.0), ("foot.R", -1.0)] {
                side(c, n, s);
            }
            if c != CastRig::OldMan {
                side(c, "thigh.L", 1.0);
                side(c, "thigh.R", -1.0);
            }
        }
        for (n, s) in [("paw_f.L", 1.0), ("paw_f.R", -1.0), ("paw_b.L", 1.0), ("paw_b.R", -1.0)] {
            side(CastRig::Dog, n, s);
        }
        let count = |c: CastRig, n: &str| {
            let cut = c.jointed();
            cut.parts[cut.index(n).unwrap()].cubes.len()
        };
        for k in 0..4 {
            assert!(count(CastRig::Kid(k), "scarf") > 30, "kid {k}'s scarf");
        }
        assert!(count(CastRig::OldMan, "beard.1") > 200 && count(CastRig::OldMan, "beard.2") > 100);
        assert!(count(CastRig::OldMan, "blade") > 300, "his staff");
        assert!(count(CastRig::OldMan, "neck") > 10);
        // His robe one garment: on its own bones, none of it on his legs.
        assert!(count(CastRig::OldMan, "robe") > 3000 && count(CastRig::OldMan, "robe.hem") > 3000);
        for leg in ["thigh.L", "thigh.R", "shin.L", "shin.R"] {
            assert_eq!(count(CastRig::OldMan, leg), 0, "{leg}");
        }
        assert!(count(CastRig::Dog, "jaw") > 50 && count(CastRig::Dog, "tail.3") > 10);
        // Krampus's are the lair's bones, so the lair's clips read for him.
        let lair = super::super::foe_rigs::FoeRig::Krampus.def();
        assert_eq!(CastRig::Krampus.joints().def.bones, lair.bones);
        assert_eq!(CastRig::Krampus.joints().def.parents, lair.parents);
    }
}
