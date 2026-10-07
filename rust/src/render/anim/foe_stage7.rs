//! The seventh stage's foes cut into their bones (ANIMATION.md §11.7): the frost giant and Surtr, giants
//! on the heroes' bones (Surtr's flaming sword his `offhand`); Hel on them, her gown on her hips, her
//! staff her `offhand`; Níðhöggr on a serpent's body, its neck rearing out of its coils in five.

use super::foe_build::Build;
use super::skeleton::{Bone, Cut, Joints};
use crate::render::characters::mat;

const INF: f32 = f32::INFINITY;

fn mirror(p: &[[f32; 3]], cx: f32) -> Vec<[f32; 3]> {
    p.iter().map(|q| [2.0 * cx - q[0], q[1], q[2]]).collect()
}

fn arms(b: &mut Build, arm_l: &[[f32; 3]; 4], arm_r: &[[f32; 3]]) {
    b.chain(&[Bone::UpperArmL, Bone::ForearmL, Bone::HandL].map(Bone::index), arm_l, None);
    b.chain(&[Bone::UpperArmR, Bone::ForearmR, Bone::HandR].map(Bone::index), arm_r, None);
    b.at(Bone::Offhand, arm_l[2]).at(Bone::Blade, arm_r[2]).at(Bone::Shield, arm_r[1]);
}

fn legs(b: &mut Build, cx: f32, leg_l: &[[f32; 3]; 4]) {
    b.chain(&[Bone::ThighL, Bone::ShinL, Bone::FootL].map(Bone::index), leg_l, None);
    b.chain(&[Bone::ThighR, Bone::ShinR, Bone::FootR].map(Bone::index), &mirror(leg_l, cx), None);
}

fn torso(b: &mut Build, hips: [f32; 3], waist: f32, chest: f32, neck: [f32; 3], head: [f32; 3]) {
    b.at(Bone::Hips, hips).at(Bone::Spine, [hips[0], waist, hips[2]]).at(Bone::Chest, [hips[0], chest, hips[2]]);
    b.at(Bone::Neck, neck).at(Bone::Head, head);
    b.cut(Cut::up(Bone::Hips.index(), Bone::Spine.index(), waist)).cut(Cut::up(Bone::Spine.index(), Bone::Chest.index(), chest));
}

/**
 * The frost giant (grid 96 × 144 × 60, `foes7::FROST_GIANT_SIZE`; x 47.5 and z 29.5 his middle): his
 * bearskin on his hips, his arms bent at the elbow with bracers of ice, fists with ice on the knuckles.
 */
pub fn frost_giant() -> &'static Joints {
    let (cx, cz) = (47.5, 29.5);
    let mut b = Build::humanoid();
    torso(&mut b, [cx, 62.0, cz], 72.0, 88.0, [cx, 108.0, cz + 2.0], [cx, 112.0, cz + 2.5]);
    let arm_l = [[cx + 26.0, 107.0, cz - 1.0], [cx + 33.0, 82.0, cz - 3.5], [cx + 35.0, 61.0, cz + 6.5], [cx + 36.0, 50.0, cz + 10.0]];
    arms(&mut b, &arm_l, &mirror(&arm_l, cx));
    legs(&mut b, cx, &[[cx + 10.5, 60.0, cz], [cx + 12.5, 34.0, cz + 2.0], [cx + 13.6, 8.5, cz - 2.4], [cx + 14.0, 1.0, cz + 9.0]]);
    b.finish("frost_giant", false)
}

/**
 * Surtr (grid 104 × 172 × 110, `foes7::SURTR_SIZE`; x 51.5 and z 54.5 his middle): his kilt of iron
 * plates on his hips, flames off his shoulders on his chest; the left fist smouldering; the flaming
 * sword in his fist at the sculpt's high x (his own left) the `offhand`, below the fist.
 */
pub fn surtr() -> &'static Joints {
    let (cx, cz) = (51.5, 54.5);
    let mut b = Build::humanoid();
    let hand = [cx + 32.6, 75.0, cz + 13.0];
    b.cuts_first(&[
        Cut::cube(Bone::UpperArmL.index(), Bone::Offhand.index(), [cx + 18.0, -INF, cz + 7.0], [INF, 77.0, INF]).except(&[mat::SKIN])
    ]);
    torso(&mut b, [cx, 68.0, cz], 78.0, 96.0, [cx, 118.0, cz + 1.0], [cx, 123.0, cz + 1.0]);
    let arm_l = [[cx + 27.5, 116.0, cz - 1.0], [cx + 33.5, 94.0, cz + 2.0], [cx + 32.8, 79.7, cz + 10.3], [cx + 32.4, 70.0, cz + 16.0]];
    let arm_r = [[cx - 27.5, 116.0, cz - 1.0], [cx - 34.5, 91.0, cz - 3.5], [cx - 37.0, 68.5, cz + 5.0], [cx - 38.0, 58.0, cz + 9.0]];
    arms(&mut b, &arm_l, &arm_r);
    b.at(Bone::Offhand, hand);
    legs(&mut b, cx, &[[cx + 11.0, 66.0, cz], [cx + 13.5, 37.0, cz + 2.5], [cx + 15.0, 9.0, cz - 2.6], [cx + 15.5, 1.0, cz + 9.0]]);
    b.finish("surtr", false)
}

/**
 * Hel (grid 60 × 158 × 60, `foes7::HEL_SIZE`): her gown, flaring to the floor and trailing, on her
 * hips (her legs, thin under it, step inside it); her arms thin under wide torn sleeves, one living and
 * one bare bone; her staff in her fist at the sculpt's high x (her own left) the `offhand`.
 */
pub fn hel() -> &'static Joints {
    let (cx, cz) = (29.5, 29.5);
    let mut b = Build::humanoid();
    b.cuts_first(&[Cut::cube(Bone::UpperArmL.index(), Bone::Offhand.index(), [-INF; 3], [INF; 3]).except(&[mat::SKIN, mat::CLOTH])]);
    torso(&mut b, [cx, 66.0, cz], 74.0, 86.0, [cx, 100.0, cz], [cx, 106.0, cz + 0.6]);
    let arm_l = [[cx + 10.5, 100.0, cz], [cx + 13.2, 86.0, cz + 1.5], [cx + 14.6, 82.5, cz + 7.6], [cx + 15.0, 82.0, cz + 12.0]];
    let arm_r = [[cx - 10.5, 100.0, cz], [cx - 13.0, 86.0, cz + 2.5], [cx - 12.2, 82.5, cz + 11.0], [cx - 12.0, 82.0, cz + 15.0]];
    arms(&mut b, &arm_l, &arm_r);
    b.at(Bone::Offhand, [cx + 15.0, 82.0, cz + 10.0]);
    legs(&mut b, cx, &[[cx + 4.0, 64.0, cz], [cx + 4.6, 34.0, cz + 1.0], [cx + 4.8, 5.0, cz + 0.8], [cx + 5.0, 1.0, cz + 11.0]]);
    b.finish("hel", false)
}

/**
 * Níðhöggr (grid 86 × 124 × 88, `foes7::NIDHOGG_SIZE`): its coils on the ground and the roots it gnaws
 * the root; its tail; its neck rearing out of the coils in an S, in five, cut square to each bend; its
 * long horned head, its jaws gaping.
 */
pub fn nidhogg() -> &'static Joints {
    let (cx, cz) = (42.5, 43.5);
    let at = |x: f32, y: f32, z: f32| [cx + x, y, cz + z];
    let mut b = Build::new("coil", [cx, 8.0, cz]);
    let tail = b.bone("tail", 0, [cx - 22.0, 4.8, cz - 17.0]);
    let mut necks = [0usize; 5];
    for (k, name) in ["neck.1", "neck.2", "neck.3", "neck.4", "neck.5"].into_iter().enumerate() {
        necks[k] = b.bone(name, if k == 0 { 0 } else { necks[k - 1] }, [0.0; 3]);
    }
    let head = b.bone("head", necks[4], [0.0; 3]);
    b.part("tail", tail).part("neck", necks[0]).part("head", head);
    let pts = [
        at(-2.0, 12.0, -10.0),
        at(5.5, 38.0, 2.0),
        at(2.5, 51.0, 7.0),
        at(-3.5, 63.0, 4.0),
        at(-4.5, 75.0, -2.0),
        at(1.0, 86.5, 1.0),
        at(0.0, 96.0, 20.0),
    ];
    let mut bones = necks.to_vec();
    bones.push(head);
    b.chain(&bones, &pts, None);
    b.legs(&[]);
    b.finish("nidhogg", false)
}
