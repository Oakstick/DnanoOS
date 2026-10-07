//! The realms' own foes cut into their bones (ANIMATION.md §11.8), all on the heroes' bones: the
//! ganglöt (its shroud on its hips, its shins under the hem its legs, its staff of bog-oak with the
//! lantern its `offhand`), the gaki (sticks of limbs, the belly a drum on its hips), the ellepige (her
//! gown on her hips, her open back with it: the hollow stays where it is).

use super::foe_build::Build;
use super::skeleton::{Bone, Cut, Joints};
use crate::render::characters::mat;

const INF: f32 = f32::INFINITY;

fn mirror(p: &[[f32; 3]], cx: f32) -> Vec<[f32; 3]> {
    p.iter().map(|q| [2.0 * cx - q[0], q[1], q[2]]).collect()
}

/// The hips, spine and chest at their joints (each `[x, y, z]`: the body may lean), the neck and the
/// head; cut level at the spine's and the chest's heights.
fn torso(b: &mut Build, hips: [f32; 3], spine: [f32; 3], chest: [f32; 3], neck: [f32; 3], head: [f32; 3]) {
    b.at(Bone::Hips, hips).at(Bone::Spine, spine).at(Bone::Chest, chest).at(Bone::Neck, neck).at(Bone::Head, head);
    b.cut(Cut::up(Bone::Hips.index(), Bone::Spine.index(), spine[1])).cut(Cut::up(Bone::Spine.index(), Bone::Chest.index(), chest[1]));
}

fn arms(b: &mut Build, arm_l: &[[f32; 3]], arm_r: &[[f32; 3]]) {
    b.chain(&[Bone::UpperArmL, Bone::ForearmL, Bone::HandL].map(Bone::index), arm_l, None);
    b.chain(&[Bone::UpperArmR, Bone::ForearmR, Bone::HandR].map(Bone::index), arm_r, None);
    b.at(Bone::Offhand, arm_l[2]).at(Bone::Blade, arm_r[2]).at(Bone::Shield, arm_r[1]);
}

fn legs(b: &mut Build, cx: f32, leg_l: &[[f32; 3]; 4]) {
    b.chain(&[Bone::ThighL, Bone::ShinL, Bone::FootL].map(Bone::index), leg_l, None);
    b.chain(&[Bone::ThighR, Bone::ShinR, Bone::FootR].map(Bone::index), &mirror(leg_l, cx), None);
}

/**
 * The ganglöt (grid 46 × 100 × 48, `foes_mire::GANGLOT_SIZE`; x 21.5 and z 17 its middle at the
 * floor): Hel's slow servant, stooped in its shroud. The shroud from the hem to the shoulders its hips,
 * spine and chest, bowed forward as it rises; its shins under the hem its legs (their joint where the
 * code swings them, at the knee); its arms in long sleeves, the one reaching out, the other's fist on
 * the staff; the staff with its crook and the lantern on it (wood, iron, glass and the blue flame) the
 * `offhand`, in that fist.
 */
pub fn ganglot() -> &'static Joints {
    let (cx, cz) = (21.5, 17.0);
    let mut b = Build::humanoid();
    b.cuts_first(&[Cut::cube(Bone::UpperArmL.index(), Bone::Offhand.index(), [-INF; 3], [INF; 3]).except(&[
        mat::SKIN,
        mat::CLOTH,
        mat::IVORY,
    ])]);
    torso(&mut b, [cx, 26.0, cz + 0.5], [cx, 40.0, cz + 2.4], [cx, 52.0, cz + 5.8], [cx, 62.0, cz + 9.4], [cx, 65.0, cz + 11.0]);
    let arm_l = [[cx + 8.0, 63.0, cz + 8.0], [cx + 9.5, 52.0, cz + 12.0], [cx + 10.0, 49.0, cz + 17.0], [cx + 10.0, 48.0, cz + 22.0]];
    let arm_r = [[cx - 8.0, 63.0, cz + 8.0], [cx - 9.0, 54.0, cz + 15.5], [cx - 6.5, 51.0, cz + 23.5], [cx - 6.0, 50.0, cz + 27.5]];
    arms(&mut b, &arm_l, &arm_r);
    b.at(Bone::Offhand, [cx + 10.4, 48.6, cz + 20.0]);
    legs(&mut b, cx, &[[cx + 4.0, 24.0, cz + 0.5], [cx + 4.2, 13.0, cz + 0.5], [cx + 4.4, 4.0, cz + 0.5], [cx + 4.4, 1.0, cz + 7.5]]);
    b.finish("ganglot", false)
}

/**
 * The gaki (grid 52 × 100 × 52, `foes_feast::GAKI_SIZE`; x 25.5 and z 23 its middle): a hungry ghost,
 * its belly a drum on its hips (cut above it, where the ribs begin), the starved ribs and the hunch its
 * chest; its legs dry sticks with knees like knots; its long arms reaching forward with their claws;
 * its needle neck and skull of a face the head, thrust forward.
 */
pub fn gaki() -> &'static Joints {
    let (cx, cz) = (25.5, 23.0);
    let mut b = Build::humanoid();
    torso(&mut b, [cx, 42.0, cz], [cx, 57.0, cz + 2.0], [cx, 64.0, cz + 3.0], [cx, 71.5, cz + 5.0], [cx, 73.0, cz + 6.0]);
    let arm_l = [[cx + 7.0, 69.5, cz + 4.0], [cx + 12.5, 56.0, cz + 1.0], [cx + 9.5, 50.0, cz + 17.5], [cx + 7.5, 46.0, cz + 23.0]];
    arms(&mut b, &arm_l, &mirror(&arm_l, cx));
    legs(&mut b, cx, &[[cx + 5.0, 42.0, cz], [cx + 7.5, 23.0, cz + 3.5], [cx + 6.5, 3.6, cz + 0.5], [cx + 6.5, 1.0, cz + 7.0]]);
    b.finish("gaki", false)
}

/**
 * The ellepige (grid 38 × 96 × 32, `foes_thorns::ELLEPIGE_SIZE`; x 18.5 and z 16 her middle): an
 * elf-maid. Her gown, from the lace at its hem to the bodice, her hips (her slender legs step inside
 * it), her sash her spine, her bodice her chest; her open back, the rotten trough of it, goes with
 * them as it was sculpted. Her arms bare above white bell sleeves, the forearms the sleeves; her head.
 */
pub fn ellepige() -> &'static Joints {
    let (cx, cz) = (18.5, 16.0);
    let mut b = Build::humanoid();
    torso(&mut b, [cx, 30.0, cz], [cx, 46.0, cz], [cx, 56.0, cz], [cx, 69.0, cz], [cx, 71.0, cz]);
    let arm_l = [[25.0, 66.0, cz], [26.2, 55.0, cz + 0.8], [25.4, 45.0, cz + 3.4], [25.4, 40.5, cz + 4.4]];
    arms(&mut b, &arm_l, &mirror(&arm_l, cx));
    legs(&mut b, cx, &[[21.5, 30.0, cz + 2.5], [21.5, 17.0, cz + 2.8], [21.5, 4.0, cz + 3.0], [21.5, 1.0, cz + 8.5]]);
    b.finish("ellepige", false)
}
