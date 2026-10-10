//! The sixth stage's foes cut into their bones (ANIMATION.md §11.7): Tepegöz and the gashadokuro, giants
//! on the heroes' bones (the gashadokuro's hanging jaw its own); the goryō, a ghost from the waist up
//! on them (its robe trails to wisps: no legs to walk); the seven-headed ejder on a body of its own,
//! each neck in two with its head, its four short legs in two.

use super::foe_build::Build;
use super::skeleton::{Bone, Cut, Joints};

fn mirror(p: &[[f32; 3]], cx: f32) -> Vec<[f32; 3]> {
    p.iter().map(|q| [2.0 * cx - q[0], q[1], q[2]]).collect()
}

fn limbs(b: &mut Build, cx: f32, arm_l: &[[f32; 3]; 4], leg_l: Option<&[[f32; 3]; 4]>) {
    let ar = mirror(arm_l, cx);
    b.chain(&[Bone::UpperArmL, Bone::ForearmL, Bone::HandL].map(Bone::index), arm_l, None);
    b.chain(&[Bone::UpperArmR, Bone::ForearmR, Bone::HandR].map(Bone::index), &ar, None);
    if let Some(leg_l) = leg_l {
        b.chain(&[Bone::ThighL, Bone::ShinL, Bone::FootL].map(Bone::index), leg_l, None);
        b.chain(&[Bone::ThighR, Bone::ShinR, Bone::FootR].map(Bone::index), &mirror(leg_l, cx), None);
    }
    b.at(Bone::Offhand, arm_l[2]).at(Bone::Blade, ar[2]).at(Bone::Shield, ar[1]);
}

fn torso(b: &mut Build, hips: [f32; 3], waist: f32, chest: f32, neck: [f32; 3], head: [f32; 3]) {
    b.at(Bone::Hips, hips).at(Bone::Spine, [hips[0], waist, hips[2]]).at(Bone::Chest, [hips[0], chest, hips[2]]);
    b.at(Bone::Neck, neck).at(Bone::Head, head);
    b.cut(Cut::up(Bone::Hips.index(), Bone::Spine.index(), waist)).cut(Cut::up(Bone::Spine.index(), Bone::Chest.index(), chest));
}

/**
 * Tepegöz (grid 100 × 138 × 100, `foes6::TEPEGOZ_SIZE`; x and z 49.5 his middle): a giant, stooped,
 * his arms thicker than a man's waist with huge hands, his legs like tree trunks. His head the bald
 * skull with its one great eye (the view lights the eye where the clip carries the head).
 */
pub fn tepegoz() -> &'static Joints {
    let (cx, cz) = (49.5, 49.5);
    let mut b = Build::humanoid();
    torso(&mut b, [cx, 60.0, cz], 66.0, 80.0, [cx, 100.0, cz + 4.0], [cx, 107.0, cz + 8.5]);
    limbs(
        &mut b,
        cx,
        &[[cx + 23.5, 97.0, cz + 4.0], [cx + 27.5, 74.0, cz + 3.0], [cx + 29.5, 54.5, cz + 7.0], [cx + 30.0, 42.0, cz + 11.0]],
        Some(&[[cx + 10.5, 57.0, cz], [cx + 11.3, 32.0, cz + 1.0], [cx + 11.7, 8.5, cz - 1.5], [cx + 12.0, 1.0, cz + 9.0]]),
    );
    b.finish("tepegoz", false)
}

/**
 * The gashadokuro (grid 72 × 124 × 120, `foes6::GASHADOKURO_SIZE`): a giant skeleton bowed over, its
 * knees bent, its long arms reaching out before it with the fingers spread to seize, its huge skull
 * hung forward on its spine; its jaw, hanging open from its hinges, its own bone. The burial shroud
 * over its back rides its chest.
 */
pub fn gashadokuro() -> &'static Joints {
    let (cx, bz) = (35.5, 55.5);
    let mut b = Build::humanoid();
    torso(&mut b, [cx, 56.0, bz], 66.0, 80.0, [cx, 95.0, bz + 9.0], [cx, 100.0, bz + 17.0]);
    limbs(
        &mut b,
        cx,
        &[[cx + 15.5, 92.0, bz + 4.5], [cx + 19.5, 78.0, bz + 24.0], [cx + 18.0, 75.0, bz + 43.0], [cx + 17.0, 76.0, bz + 53.0]],
        Some(&[[cx + 8.5, 54.5, bz - 2.0], [cx + 9.6, 35.0, bz + 8.6], [cx + 9.8, 7.0, bz - 2.0], [cx + 10.0, 1.0, bz + 8.0]]),
    );
    let jaw = b.bone("jaw", Bone::Head.index(), [cx, 100.8, bz + 26.4]);
    b.part("jaw", jaw);
    b.finish("gashadokuro", false)
}

/**
 * The goryō (grid 60 × 124 × 92, `foes6::GORYO_SIZE`): a ghost in a court robe. Its robe, its train
 * and the wisps it frays into are its hips (it floats: its legs are empty bones under it); its arms in
 * their great sleeves held out before it, the thin hands crackling; its head.
 */
pub fn goryo() -> &'static Joints {
    let (cx, cz) = (29.5, 45.5);
    let mut b = Build::humanoid();
    torso(&mut b, [cx, 60.0, cz], 70.0, 78.0, [cx, 86.0, cz + 1.5], [cx, 88.0, cz + 1.5]);
    let arm_l = [[cx + 11.0, 85.0, cz + 1.0], [cx + 14.5, 75.0, cz + 8.0], [cx + 12.0, 73.0, cz + 21.0], [cx + 11.6, 74.5, cz + 24.0]];
    limbs(&mut b, cx, &arm_l, None);
    for (side, x) in [(true, cx + 5.0), (false, cx - 5.0)] {
        let (t, s, f) = if side { (Bone::ThighL, Bone::ShinL, Bone::FootL) } else { (Bone::ThighR, Bone::ShinR, Bone::FootR) };
        b.at(t, [x, 58.0, cz]).at(s, [x, 32.0, cz]).at(f, [x, 6.0, cz]);
    }
    b.legs(&[]);
    b.finish("goryo", false)
}

/**
 * The ejder (grid 132 × 132 × 102, `foes6::EJDER_SIZE`): the seven-headed dragon. Its body the root
 * (the deep chest under the necks, the belly, the haunches); each neck from its root on the shoulders
 * in two (cut halfway along it) to its head; its tail curled round its side; each short heavy leg in
 * two (to the elbow, then the forearm and the clawed foot).
 */
pub fn ejder() -> &'static Joints {
    let (ex, ez) = (65.5, 50.5);
    let roots: [[f32; 3]; 7] = [
        [ex - 26.0, 41.5, ez + 13.0],
        [ex - 19.0, 49.5, ez + 14.0],
        [ex - 10.0, 54.0, ez + 13.0],
        [ex, 55.5, ez + 12.0],
        [ex + 10.0, 54.0, ez + 13.0],
        [ex + 19.0, 49.5, ez + 14.0],
        [ex + 26.0, 41.5, ez + 13.0],
    ];
    let heads: [[f32; 3]; 7] = [
        [ex - 52.0, 79.0, ez + 20.0],
        [ex - 41.0, 96.0, ez + 22.0],
        [ex - 23.0, 107.5, ez + 22.0],
        [ex, 113.5, ez + 19.0],
        [ex + 23.0, 107.5, ez + 22.0],
        [ex + 41.0, 96.0, ez + 22.0],
        [ex + 52.0, 79.0, ez + 20.0],
    ];
    let mut b = Build::new("body", [ex, 30.0, ez]);
    for k in 0..7 {
        let a = b.bone(leak(format!("neck{k}.a")), 0, [0.0; 3]);
        let m = b.bone(leak(format!("neck{k}.b")), a, [0.0; 3]);
        let h = b.bone(leak(format!("head{k}")), m, [0.0; 3]);
        let (r, hd) = (roots[k], heads[k]);
        let mid = [(r[0] + hd[0]) / 2.0, (r[1] + hd[1]) / 2.0 + 4.0, (r[2] + hd[2]) / 2.0];
        b.part(leak(format!("neck{k}")), a);
        b.chain(&[a, m, h], &[r, mid, hd, [hd[0], hd[1] - 2.0, hd[2] + 14.0]], None);
    }
    let tail = b.bone("tail", 0, [ex, 29.0, ez - 29.0]);
    b.part("tail", tail);
    let legs: [[f32; 3]; 4] =
        [[ex - 23.0, 29.0, ez + 12.0], [ex + 23.0, 29.0, ez + 12.0], [ex - 18.5, 28.0, ez - 22.0], [ex + 18.5, 28.0, ez - 22.0]];
    for (i, (name, part)) in
        [("leg_f.R", "frontLeftLeg"), ("leg_f.L", "frontRightLeg"), ("leg_b.R", "backLeftLeg"), ("leg_b.L", "backRightLeg")]
            .into_iter()
            .enumerate()
    {
        let side = if i % 2 == 0 { -1.0 } else { 1.0 };
        let j = legs[i];
        let knee = [j[0] + side * 7.5, j[1] - 12.0, j[2] + if i < 2 { 1.0 } else { 6.0 }];
        let foot = [j[0] + side * 7.0, 0.0, j[2] + if i < 2 { 8.0 } else { 5.0 }];
        let up = b.bone(name, 0, [0.0; 3]);
        let low = b.bone(leak(format!("{name}.low")), up, [0.0; 3]);
        b.part(part, up);
        b.chain(&[up, low], &[j, knee, foot], None);
    }
    b.legs(&[]);
    b.finish("ejder", false)
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}
