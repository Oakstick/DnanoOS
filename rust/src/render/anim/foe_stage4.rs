//! The fourth stage's foes cut into their bones (ANIMATION.md §11.7): the kappa and the oni on the
//! heroes' bones (their limbs sculpted bent: cut square to each bend, `foe_build::chain`), the
//! rokurokubi kneeling with her neck in six, the kitsune on the wolves' bones with its tail in three.

use super::foe_build::Build;
use super::skeleton::{Bone, Cut, Joints};
use crate::render::characters::mat;

const INF: f32 = f32::INFINITY;

/// A humanoid's arm (shoulder, elbow, wrist, the hand's end) and leg (hip, knee, ankle, the toe),
/// cut at each bend.
fn arm(b: &mut Build, left: bool, p: [[f32; 3]; 4]) {
    let bones = if left { [Bone::UpperArmL, Bone::ForearmL, Bone::HandL] } else { [Bone::UpperArmR, Bone::ForearmR, Bone::HandR] };
    b.chain(&bones.map(Bone::index), &p, None);
    // (Nothing in its fists: the weapon bones sit in them, empty.)
    let (fist, weapon) = if left { (Bone::HandL, Bone::Offhand) } else { (Bone::HandR, Bone::Blade) };
    let at = p[2];
    b.at(weapon, at);
    let _ = fist;
}

fn leg(b: &mut Build, left: bool, p: [[f32; 3]; 4]) {
    let bones = if left { [Bone::ThighL, Bone::ShinL, Bone::FootL] } else { [Bone::ThighR, Bone::ShinR, Bone::FootR] };
    b.chain(&bones.map(Bone::index), &p, None);
}

fn mirror(p: [[f32; 3]; 4], cx: f32) -> [[f32; 3]; 4] {
    p.map(|q| [2.0 * cx - q[0], q[1], q[2]])
}

/**
 * The kappa (grid 55 × 80 × 56, `foes4::KAPPA_SIZE`; x 27 its middle, z 25 its hips). Squat: its
 * thighs bent out over its knees, its short arms thrust forward and out, the palms open; every limb
 * is cut square to its bends. Its waist above its pot belly's middle (34), its chest from 40 (the
 * shell on its back rides its spine and chest); its neck the thick column under its great head (to
 * 51), its head the rest with the dish; the water in the dish its own bone (`water`, on the head):
 * spilled, the view sinks it as the code does.
 */
pub fn kappa() -> &'static Joints {
    let (cx, cz) = (27.0, 25.0);
    let mut b = Build::humanoid();
    b.at(Bone::Hips, [cx, 28.0, cz]).at(Bone::Spine, [cx, 34.0, cz]).at(Bone::Chest, [cx, 40.0, cz + 1.0]);
    b.at(Bone::Neck, [cx, 45.0, cz + 2.0]).at(Bone::Head, [cx, 51.0, cz + 4.0]);
    let arm_l = [[cx + 12.5, 46.5, cz + 2.0], [cx + 18.0, 39.5, cz + 9.0], [cx + 16.5, 40.5, cz + 17.0], [cx + 16.5, 45.0, cz + 21.0]];
    arm(&mut b, true, arm_l);
    arm(&mut b, false, mirror(arm_l, cx));
    let leg_l = [[cx + 8.5, 29.0, cz - 1.5], [cx + 17.0, 20.0, cz + 3.5], [cx + 16.0, 5.0, cz - 0.5], [cx + 16.0, 1.0, cz + 8.0]];
    leg(&mut b, true, leg_l);
    leg(&mut b, false, mirror(leg_l, cx));
    b.at(Bone::Shield, [cx - 17.0, 40.0, cz + 13.0]);
    let water = b.bone("water", Bone::Head.index(), [cx, 74.0, cz + 5.5]);
    b.part("water", water);
    b.cut(Cut::up(Bone::Hips.index(), Bone::Spine.index(), 34.0))
        .cut(Cut::up(Bone::Spine.index(), Bone::Chest.index(), 40.0))
        .cut(Cut::cube(Bone::Head.index(), Bone::Neck.index(), [-INF, -INF, -INF], [INF, 51.0, INF]));
    b.finish("kappa", false)
}

/**
 * The oni (grid 91 × 138 × 57, `foes4::ONI_SIZE`; x 45 his middle). His waist the top of his tiger
 * skin (58), his chest from 74; his neck the column under his head (from 96), his head the rest. His
 * arms hang bent at the elbow, cuffed in iron at the wrist; his legs bowed at the knee. The kanabo in
 * his fist at the sculpt's high x (his own left) is the `offhand`: the iron club below his fist.
 */
pub fn oni() -> &'static Joints {
    let (cx, cz) = (45.0, 28.0);
    let mut b = Build::humanoid();
    b.at(Bone::Hips, [cx, 52.0, cz]).at(Bone::Spine, [cx, 58.0, cz + 2.0]).at(Bone::Chest, [cx, 74.0, cz + 2.0]);
    b.at(Bone::Neck, [cx, 95.0, cz + 1.0]).at(Bone::Head, [cx, 98.0, 29.5]);
    let arm_l = [[cx + 25.0, 94.0, cz], [cx + 30.5, 72.0, cz + 1.0], [cx + 31.5, 56.0, cz + 4.0], [cx + 31.5, 44.0, cz + 6.0]];
    arm(&mut b, true, arm_l);
    arm(&mut b, false, mirror(arm_l, cx));
    b.at(Bone::Offhand, [cx + 31.5, 50.0, cz + 5.2]).at(Bone::Blade, [cx - 31.5, 50.0, cz + 5.2]);
    let leg_l = [[cx + 11.0, 48.0, cz], [cx + 15.0, 27.0, cz + 1.0], [cx + 16.0, 7.5, cz - 2.0], [cx + 16.0, 1.0, cz + 10.0]];
    leg(&mut b, true, leg_l);
    leg(&mut b, false, mirror(leg_l, cx));
    b.at(Bone::Shield, [cx - 31.0, 64.0, cz + 3.0]);
    let (h, s, c, n, ua) = (Bone::Hips.index(), Bone::Spine.index(), Bone::Chest.index(), Bone::Neck.index(), Bone::UpperArmL.index());
    // (The kanabo first, out of the arm before its elbow is cut: the iron below his fist.)
    let club = Cut::cube(ua, Bone::Offhand.index(), [cx + 18.0, -INF, -INF], [cx + 46.0, 44.5, INF]).except(&[mat::SKIN]);
    b.cuts_first(&[club]);
    b.cut(Cut::up(h, s, 58.0))
        .cut(Cut::up(s, c, 74.0))
        .cut(Cut::cube(c, n, [cx - 7.0, 96.0, -INF], [cx + 7.0, INF, INF]).only(&[mat::SKIN]));
    b.finish("oni", false)
}

/**
 * The rokurokubi (grid 49 × 142 × 45, `foes4::ROKURO_SIZE`). She kneels: her folded legs and the
 * kimono pooled round them are her body (the root), her upright torso from her obi (y 24) her chest;
 * her arms in their long sleeves rest her hands on her knees. Her neck rises out of her collar in six
 * bones along its curves (`foes4::ROKURO_NECK`), each cut square to its bend, to her head far above.
 */
pub fn rokurokubi() -> &'static Joints {
    let (cx, cz) = (24.0, 22.0);
    let tz = cz - 3.5;
    let mut b = Build::new("body", [cx, 8.0, cz]);
    let chest = b.bone("chest", 0, [cx, 24.0, tz]);
    let nk = crate::render::foes4::ROKURO_NECK;
    let mut necks = [0usize; 6];
    let names = ["neck.1", "neck.2", "neck.3", "neck.4", "neck.5", "neck.6"];
    for (k, name) in names.into_iter().enumerate() {
        necks[k] = b.bone(name, if k == 0 { chest } else { necks[k - 1] }, [0.0; 3]);
    }
    let head = b.bone("head", necks[5], [cx, 121.0, 28.0]);
    let jaw = b.bone("jaw", head, [cx, 123.0, 30.0]);
    let mut arms = [0usize; 6];
    for (k, name) in ["upper_arm.L", "forearm.L", "hand.L", "upper_arm.R", "forearm.R", "hand.R"].into_iter().enumerate() {
        arms[k] = b.bone(name, if k % 3 == 0 { chest } else { arms[k - 1] }, [0.0; 3]);
    }
    b.part("neck", necks[0]).part("head", head).part("leftArm", arms[3]).part("rightArm", arms[0]);
    b.chain(&necks, &[nk[0], nk[2], nk[3], nk[4], nk[5], nk[6], nk[8]], None);
    let arm_l = [[cx + 8.0, 36.5, tz], [cx + 10.6, 25.5, tz + 1.5], [cx + 6.4, 15.8, cz + 6.0], [cx + 5.0, 13.5, cz + 10.0]];
    b.chain(&arms[0..3], &arm_l, None);
    b.chain(&arms[3..6], &mirror(arm_l, cx), None);
    b.cut(Cut::up(0, chest, 24.0))
        .cut(Cut::cube(head, jaw, [cx - 4.5, -INF, 31.0], [cx + 4.5, 122.5, INF]).except(&[mat::FUR, mat::CLOTH]));
    b.legs(&[0]);
    b.finish("rokurokubi", false)
}

/**
 * The kitsune (grid 45 × 102 × 117, `foes4::KITSUNE_SIZE`; it faces +z). On the wolves' bones: its
 * hips the haunches, its chest from z 58, its neck the column up from its chest to z 86, its head the
 * fine wedge and the tall ears, its jaw the lower muzzle; its tail raised behind it in an S in three
 * (the foxfire over its tip riding the last); its forelegs straight, its hind legs bent at the hock,
 * each cut square to its bends.
 */
pub fn kitsune() -> &'static Joints {
    let cx = 22.0;
    let mut b = Build::new("hips", [cx, 40.0, 39.0]);
    let chest = b.bone("chest", 0, [cx, 41.0, 58.0]);
    let neck = b.bone("neck", chest, [cx, 41.0, 79.0]);
    let head = b.bone("head", neck, [cx, 47.0, 86.0]);
    let jaw = b.bone("jaw", head, [cx, 50.0, 95.0]);
    let t1 = b.bone("tail.1", 0, [0.0; 3]);
    let t2 = b.bone("tail.2", t1, [0.0; 3]);
    let t3 = b.bone("tail.3", t2, [0.0; 3]);
    let mut legs = Vec::new();
    for (end, parent) in [("f", chest), ("b", 0)] {
        for side in ["L", "R"] {
            let u = b.bone(leak(format!("upper_{end}.{side}")), parent, [0.0; 3]);
            let l = b.bone(leak(format!("lower_{end}.{side}")), u, [0.0; 3]);
            let p = b.bone(leak(format!("paw_{end}.{side}")), l, [0.0; 3]);
            legs.push([u, l, p]);
        }
    }
    // (Front left and right, back left and right; `.L` at the sculpt's high x.)
    b.part("frontRightLeg", legs[0][0]).part("frontLeftLeg", legs[1][0]).part("backRightLeg", legs[2][0]).part("backLeftLeg", legs[3][0]);
    b.part("head", head).part("tail", t1);
    for (k, x) in [27.5f32, 16.5].into_iter().enumerate() {
        let out = if k == 0 { 1.0 } else { -1.0 };
        b.chain(&legs[k], &[[x, 40.0, 74.0], [x + out * 0.4, 28.0, 71.0], [x + out * 0.3, 8.0, 73.0], [x, 0.0, 77.0]], None);
    }
    for (k, x) in [28.0f32, 16.0].into_iter().enumerate() {
        let out = if k == 0 { 1.0 } else { -1.0 };
        b.chain(
            &legs[2 + k],
            &[[x, 37.0, 40.5], [x + out * 0.6, 25.0, 45.0], [x + out * 0.6, 13.0, 34.0], [x + out * 0.5, 0.0, 39.0]],
            None,
        );
    }
    b.chain(&[t1, t2, t3], &[[22.0, 43.0, 31.0], [23.5, 45.0, 15.0], [21.0, 68.0, 13.5], [22.0, 93.5, 31.0]], None);
    b.cut(Cut::plane(0, chest, 2, 58.0, false)).cut(Cut::cube(head, neck, [-INF, -INF, -INF], [INF, INF, 86.0])).cut(Cut::cube(
        head,
        jaw,
        [-INF, -INF, 93.0],
        [INF, 51.5, INF],
    ));
    b.legs(&[]);
    b.finish("kitsune", false)
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}
