//! The fifth stage's foes cut into their bones (ANIMATION.md §11.7): the albastı, the cin, the arçura
//! and the karakoncolos on the heroes' bones (their limbs sculpted bent, cut square to each bend), the
//! tülükuş on a bird's (its folded wings in two, its legs and talons).

use super::foe_build::Build;
use super::skeleton::{Bone, Cut, Joints};
use crate::render::characters::mat;

const INF: f32 = f32::INFINITY;

fn mirror(p: &[[f32; 3]], cx: f32) -> Vec<[f32; 3]> {
    p.iter().map(|q| [2.0 * cx - q[0], q[1], q[2]]).collect()
}

/// Both arms (the `.L` one given, the other mirrored about `cx`) and both legs, cut at each bend;
/// the empty weapon bones in the fists.
fn limbs(b: &mut Build, cx: f32, arm_l: &[[f32; 3]; 4], leg_l: &[[f32; 3]; 4]) {
    let ar = mirror(arm_l, cx);
    let lr = mirror(leg_l, cx);
    b.chain(&[Bone::UpperArmL, Bone::ForearmL, Bone::HandL].map(Bone::index), arm_l, None);
    b.chain(&[Bone::UpperArmR, Bone::ForearmR, Bone::HandR].map(Bone::index), &ar, None);
    b.chain(&[Bone::ThighL, Bone::ShinL, Bone::FootL].map(Bone::index), leg_l, None);
    b.chain(&[Bone::ThighR, Bone::ShinR, Bone::FootR].map(Bone::index), &lr, None);
    b.at(Bone::Offhand, arm_l[2]).at(Bone::Blade, ar[2]).at(Bone::Shield, [ar[1][0], (ar[1][1] + ar[2][1]) / 2.0, ar[1][2]]);
}

/// The torso: the hips, the waist at `waist`, the chest at `chest`, the neck and the head's joints.
fn torso(b: &mut Build, hips: [f32; 3], waist: f32, chest: f32, neck: [f32; 3], head: [f32; 3]) {
    b.at(Bone::Hips, hips).at(Bone::Spine, [hips[0], waist, hips[2]]).at(Bone::Chest, [hips[0], chest, hips[2]]);
    b.at(Bone::Neck, neck).at(Bone::Head, head);
    b.cut(Cut::up(Bone::Hips.index(), Bone::Spine.index(), waist)).cut(Cut::up(Bone::Spine.index(), Bone::Chest.index(), chest));
}

/**
 * The albastı (grid 48 × 116 × 54, `foes5::ALBASTI_SIZE`): crooked, her knees bent forward and bony,
 * her arms long past all reason with the hands hanging by her knees, her head thrust out before her.
 */
pub fn albasti() -> &'static Joints {
    let (cx, cz) = (23.5, 26.5);
    let mut b = Build::humanoid();
    torso(&mut b, [cx, 55.0, cz], 62.0, 76.0, [cx, 92.0, cz + 3.0], [cx, 96.0, cz + 6.5]);
    limbs(
        &mut b,
        cx,
        &[[cx + 11.5, 92.0, cz + 4.0], [cx + 15.3, 64.0, cz + 3.6], [cx + 14.5, 33.0, cz + 12.0], [cx + 14.5, 22.0, cz + 15.0]],
        &[[cx + 5.5, 53.0, cz - 3.0], [cx + 7.1, 31.0, cz + 6.0], [cx + 7.3, 5.5, cz - 1.0], [cx + 8.1, 1.0, cz + 6.0]],
    );
    b.finish("albasti", false)
}

/**
 * The cin (grid 44 × 122 × 40, `foes5::CIN_SIZE`): robed to his slippers (his legs, lost in the robe,
 * are his slippers only, on the joints a man's legs would have), wide sleeves bent at the elbow; the
 * crooked staff with its smokeless fire in his fist at the sculpt's low x (his own right): the
 * `blade`, its wood and its fire.
 */
pub fn cin() -> &'static Joints {
    let (cx, cz) = (21.5, 19.5);
    let mut b = Build::humanoid();
    // (The staff out of his arm before its elbow is cut.)
    b.cuts_first(&[Cut::cube(Bone::UpperArmR.index(), Bone::Blade.index(), [-INF; 3], [INF; 3]).except(&[mat::SKIN, mat::CLOTH])]);
    torso(&mut b, [cx, 44.0, cz], 56.0, 68.0, [cx, 82.0, cz], [cx, 84.0, cz + 1.0]);
    let arm_l = [[cx + 11.0, 82.0, cz], [cx + 13.6, 66.0, cz + 1.0], [cx + 14.4, 56.0, cz + 5.0], [cx + 14.6, 51.0, cz + 6.5]];
    // (His staff arm is held otherwise: its elbow and wrist its own.)
    let arm_r = [[cx - 11.0, 82.0, cz], [cx - 13.6, 67.0, cz + 0.6], [cx - 13.8, 61.0, cz + 6.6], [cx - 13.9, 57.0, cz + 9.0]];
    let leg_l = [[cx + 4.0, 44.0, cz], [cx + 4.0, 24.0, cz], [cx + 4.0, 9.5, cz], [cx + 4.5, 1.0, cz + 9.0]];
    b.chain(&[Bone::UpperArmL, Bone::ForearmL, Bone::HandL].map(Bone::index), &arm_l, None);
    b.chain(&[Bone::UpperArmR, Bone::ForearmR, Bone::HandR].map(Bone::index), &arm_r, None);
    b.chain(&[Bone::ThighL, Bone::ShinL, Bone::FootL].map(Bone::index), &leg_l, None);
    b.chain(&[Bone::ThighR, Bone::ShinR, Bone::FootR].map(Bone::index), &mirror(&leg_l, cx), None);
    b.at(Bone::Offhand, arm_l[2]).at(Bone::Blade, [cx - 13.85, 58.9, cz + 8.1]).at(Bone::Shield, [cx - 13.7, 64.0, cz + 3.0]);
    b.finish("cin", false)
}

/**
 * The arçura (grid 46 × 132 × 36, `foes5::ARCURA_SIZE`): a goat's legs, the knee high and forward and
 * the hock back (thigh to the knee, shin to the hock, the foot the cannon and the cloven hoof), long
 * lean arms with fingers of twigs; its short tail on its hips.
 */
pub fn arcura() -> &'static Joints {
    let (cx, cz) = (22.5, 17.5);
    let mut b = Build::humanoid();
    b.part("tail", Bone::Hips.index());
    torso(&mut b, [cx, 61.0, cz], 70.0, 82.0, [cx, 97.0, cz + 0.5], [cx, 100.0, 18.3]);
    limbs(
        &mut b,
        cx,
        &[[cx + 8.5, 96.0, cz + 0.5], [cx + 11.5, 76.0, cz + 0.4], [cx + 12.5, 57.0, cz + 3.6], [cx + 13.0, 44.0, cz + 5.5]],
        &[[cx + 5.5, 61.0, cz - 1.0], [cx + 6.1, 46.0, cz + 4.6], [cx + 6.4, 25.0, cz - 5.0], [cx + 6.4, 0.0, cz + 1.5]],
    );
    b.finish("arcura", false)
}

/**
 * The karakoncolos (grid 72 × 138 × 58, `foes5::KARAKONCOLOS_SIZE`): an ape's long thick arms with
 * huge clawed hands, bowed legs with the knees bent, the head set low and forward in its hunch; its
 * shaggy coat goes with each part as it was sculpted.
 */
pub fn karakoncolos() -> &'static Joints {
    let (cx, cz) = (35.5, 28.5);
    let mut b = Build::humanoid();
    torso(&mut b, [cx, 57.0, cz], 68.0, 84.0, [cx, 104.0, cz + 4.5], [cx, 109.0, cz + 8.0]);
    limbs(
        &mut b,
        cx,
        &[[cx + 21.0, 109.0, cz + 2.5], [cx + 25.0, 76.0, cz + 3.0], [cx + 25.6, 41.0, cz + 9.0], [cx + 26.0, 28.0, cz + 12.0]],
        &[[cx + 9.5, 57.0, cz - 2.0], [cx + 11.1, 31.0, cz + 4.0], [cx + 11.7, 8.0, cz - 1.0], [cx + 12.3, 1.0, cz + 7.5]],
    );
    b.finish("karakoncolos", false)
}

/**
 * The tülükuş (grid 48 × 88 × 46, `foes5::TULUKUS_SIZE`): an owl standing on its talons. Its body (the
 * root, with its tail), its round head, each wing folded down its side in two (from the shoulder to
 * the wrist, about y 37, and the primaries below), each leg (the feathered trousers) and its foot
 * (the toes and talons, below y 6.5).
 */
pub fn tulukus() -> &'static Joints {
    let (cx, cz) = (23.5, 22.5);
    let mut b = Build::new("body", [cx, 28.0, cz]);
    let head = b.bone("head", 0, [cx, 54.0, cz + 0.5]);
    let wing_l = b.bone("wing.L", 0, [35.5, 53.0, 21.5]);
    let hand_l = b.bone("hand.L", wing_l, [36.5, 37.0, 20.0]);
    let wing_r = b.bone("wing.R", 0, [11.5, 53.0, 21.5]);
    let hand_r = b.bone("hand.R", wing_r, [10.5, 37.0, 20.0]);
    let leg_l = b.bone("leg.L", 0, [cx + 6.0, 16.0, cz + 1.0]);
    let foot_l = b.bone("foot.L", leg_l, [cx + 6.8, 6.5, cz + 2.5]);
    let leg_r = b.bone("leg.R", 0, [cx - 6.0, 16.0, cz + 1.0]);
    let foot_r = b.bone("foot.R", leg_r, [cx - 6.8, 6.5, cz + 2.5]);
    b.part("head", head).part("rightWing", wing_l).part("leftWing", wing_r).part("rightLeg", leg_l).part("leftLeg", leg_r);
    b.cut(Cut::down(wing_l, hand_l, 37.0))
        .cut(Cut::down(wing_r, hand_r, 37.0))
        .cut(Cut::down(leg_l, foot_l, 6.5))
        .cut(Cut::down(leg_r, foot_r, 6.5));
    b.finish("tulukus", false)
}
