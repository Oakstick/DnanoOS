//! How a hero holds what he carries, laid on the pose his clips give him after they are sampled and
//! blended: both fists on the grip of a two-handed weapon (Miyamoto's katana), and a shield slung on
//! his back that comes before him only to guard. Worked out every frame on the finished pose, so a
//! crossfade, the guard laid over his upper body or a flinch never parts his hands from the hilt.
//!
//! The grip: the sword hand's fist stays where the clip puts the weapon (the blade's place and turn
//! are the clip's), the off hand closes on the grip a fist's height below it, turned as the blade is,
//! and a two-bone solve bends each arm to reach (the elbows bending the way the clip bends them). The
//! heroes' arms are short against their shoulders (a voxel hero's build), so where the two arms
//! cannot meet on the grip, it is drawn toward the middle of his front (`BEFORE`) until they can,
//! each shoulder coming forward and in a little (`REACH_IN`); what is still short leaves the off
//! fist at the end of its arm, by the grip.

use glam::{Mat4, Quat, Vec3};

use super::clip::{Locals, Skeleton, Trs};
use super::skeleton::{Bone, PARENTS};

/// The way a held weapon points out of the fist at rest, in the model's axes: up, leaning forward
/// 0.42 voxels a voxel (`armory::LEAN`).
fn weapon_rest() -> Vec3 {
    Vec3::new(0.0, 1.0, 0.42).normalize()
}

/// How far below the sword hand's grip the off hand closes on it (metres: a fist, 8.4 voxels).
pub const FIST_GAP: f32 = 0.105;
/// How far each shoulder may come forward and in to reach the grip (metres).
pub const REACH_IN: f32 = 0.09;
/// How far before the middle of his chest the grip is drawn when his arms cannot meet where the clip
/// holds it (metres: the depth of his breastplate and a fist).
pub const BEFORE: f32 = 0.19;
/// How much of the grip's reach out to his side the clip gives it is kept (the rest drawn in to his
/// middle).
pub const INWARD: f32 = 0.45;
/// How far out to his side the shield swings on its way between his back and his front (metres).
const SWING_OUT: f32 = 0.16;
/// Where the shield hangs on his back, from where it was carved there (metres): lower, and a little
/// to his shield side, so his sword shoulder and his cuts show over it from the chase camera.
const SLUNG: Vec3 = Vec3::new(0.04, -0.14, 0.0);
/// Where the shield's middle stands from the middle of his shield forearm to guard (metres: a little
/// up, and out before it).
const GUARD_OUT: Vec3 = Vec3::new(0.0, 0.02, 0.07);

fn at(m: &Mat4) -> Vec3 {
    m.w_axis.truncate()
}

/// A turn of `q` about the point `p`.
fn about(p: Vec3, q: Quat) -> Mat4 {
    Mat4::from_translation(p) * Mat4::from_quat(q) * Mat4::from_translation(-p)
}

/// Writes back the bones `bones` (parents first) from their matrices in the model `world`: each
/// one's transform relative to its parent.
fn write(sk: &Skeleton, pose: &mut Locals, world: &[Mat4], bones: &[Bone]) {
    for &b in bones {
        let i = b.index();
        let parent = PARENTS[i].map(|p| world[p]).unwrap_or(sk.pre);
        let (s, r, t) = (parent.inverse() * world[i]).to_scale_rotation_translation();
        pose[i] = Trs { t, r: r.normalize(), s };
    }
}

/// One arm bent to bring its wrist to `wrist` (in the model): its shoulder first moved by `shift`,
/// the upper arm turned about the shoulder and the forearm about the elbow, the elbow staying on the
/// side the pose bends it to. Returns how far short of `wrist` the arm's end stays (zero when it
/// reaches).
fn reach(sk: &Skeleton, world: &mut [Mat4], arm: [Bone; 3], shift: Vec3, wrist: Vec3) -> Vec3 {
    let [ua, fa, hand] = arm.map(Bone::index);
    let rest = |i: usize| at(&sk.rest_inv[i].inverse());
    let (a, b) = ((rest(fa) - rest(ua)).length(), (rest(hand) - rest(fa)).length());
    let lift = Mat4::from_translation(shift);
    let (s, e0, h0) = (at(&world[ua]) + shift, at(&world[fa]) + shift, at(&world[hand]) + shift);
    let to = wrist - s;
    let u = to.normalize_or(Vec3::NEG_Y);
    let d = to.length().clamp((a - b).abs() + 1e-4, a + b - 1e-4);
    // The bend's side: where the pose has the elbow, off the line to the wrist.
    let side = (e0 - s) - u * (e0 - s).dot(u);
    let side = if side.length_squared() > 1e-10 { side.normalize() } else { u.any_orthonormal_vector() };
    let cos = ((a * a + d * d - b * b) / (2.0 * a * d)).clamp(-1.0, 1.0);
    let elbow = s + u * (a * cos) + side * (a * (1.0 - cos * cos).sqrt());
    let turn1 = about(s, Quat::from_rotation_arc((e0 - s).normalize_or(u), (elbow - s).normalize_or(u)));
    let h1 = (turn1 * lift).transform_point3(h0 - shift);
    let reached = s + u * d;
    let turn2 = about(elbow, Quat::from_rotation_arc((h1 - elbow).normalize_or(u), (reached - elbow).normalize_or(u)));
    world[ua] = turn1 * lift * world[ua];
    world[fa] = turn2 * turn1 * lift * world[fa];
    reached - wrist
}

/**
 * Both fists on the grip of the weapon in his sword hand: the sword hand keeps the weapon where
 * the clip holds it (drawn toward his off hand where that cannot otherwise reach), turned square
 * round its grip; the off hand closes on the grip `FIST_GAP` below it; the arms bend to reach.
 */
pub fn two_hands(sk: &Skeleton, pose: &mut Locals) {
    let mut w = sk.world(pose);
    let rest = |b: Bone| sk.rest_inv[b.index()].inverse();
    // The weapon's turn and place now (from its rest in the model), and the grip's line.
    let turn = w[Bone::Blade.index()] * sk.rest_inv[Bone::Blade.index()];
    let q = Quat::from_mat4(&turn).normalize();
    let line = q * weapon_rest();
    let held = at(&w[Bone::Blade.index()]);
    let (grip_r, grip_l) = (at(&rest(Bone::Blade)), at(&rest(Bone::Offhand)));
    let (wrist_r0, wrist_l0) = (at(&rest(Bone::HandR)), at(&rest(Bone::HandL)));
    let wrist_r = |g: Vec3| g + q * (wrist_r0 - grip_r);
    let wrist_l = |g: Vec3| g - line * FIST_GAP + q * (wrist_l0 - grip_l);
    let span = |arm: [Bone; 3]| {
        let [a, b, c] = arm.map(|b| at(&rest(b)));
        ((b - a).length() + (c - b).length()) * 0.999
    };
    let (arm_r, arm_l) = ([Bone::UpperArmR, Bone::ForearmR, Bone::HandR], [Bone::UpperArmL, Bone::ForearmL, Bone::HandL]);
    let (reach_r, reach_l) = (span(arm_r), span(arm_l));
    let (s_r, s_l) = (at(&w[Bone::UpperArmR.index()]), at(&w[Bone::UpperArmL.index()]));
    let short = |s: Vec3, t: Vec3, reach: f32| (t - s).length() - reach;

    // The grip where both wrists can reach it, as near as can be to where the clip holds it: drawn
    // toward his front (the middle of his chest, a fist before it, at the grip's height) until both
    // arms reach it, each shoulder coming forward and in toward it as far as `REACH_IN` allows. (His
    // shoulders are wide for his arms: from his sides, where the clip's arms leave his fists, the
    // two cannot meet on one grip.)
    // (In his chest's frame as it stands at rest: the clip's arms leave the grip far out at his side,
    // where the source's long arms had it in the middle; it is drawn in toward his middle and out
    // before his breastplate, keeping its height and its swing from side to side.)
    let carried = w[Bone::Chest.index()] * sk.rest_inv[Bone::Chest.index()];
    let chest = at(&rest(Bone::Chest));
    let held_at = carried.inverse().transform_point3(held);
    let drawn = carried.transform_point3(Vec3::new(chest.x + (held_at.x - chest.x) * INWARD, held_at.y, held_at.z.max(chest.z + BEFORE)));
    let front = carried.transform_point3(Vec3::new(chest.x, held_at.y.clamp(chest.y - 0.15, chest.y + 0.12), chest.z + BEFORE));
    let fits = |g: Vec3| short(s_r, wrist_r(g), reach_r) <= REACH_IN && short(s_l, wrist_l(g), reach_l) <= REACH_IN;
    let g = if fits(drawn) {
        drawn
    } else if !fits(front) {
        front
    } else {
        let (mut lo, mut hi) = (0.0, 1.0);
        for _ in 0..10 {
            let mid = (lo + hi) * 0.5;
            if fits(drawn.lerp(front, mid)) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        drawn.lerp(front, hi)
    };
    // (The shoulders come forward and in, level: never down toward a low grip.)
    let level = |v: Vec3| carried.transform_vector3(carried.inverse().transform_vector3(v) * Vec3::new(1.0, 0.0, 1.0));
    let lean_in = |s: Vec3, t: Vec3, reach: f32| level(t - s).normalize_or_zero() * short(s, t, reach).clamp(0.0, REACH_IN);
    let (in_r, in_l) = (lean_in(s_r, wrist_r(g), reach_r), lean_in(s_l, wrist_l(g), reach_l));

    // The sword arm first: the weapon goes where its fist reaches.
    let moved = Mat4::from_translation(g - held) * turn;
    let mut hand_r = moved * rest(Bone::HandR);
    let mut blade = moved * rest(Bone::Blade);
    let miss = reach(sk, &mut w, arm_r, in_r, wrist_r(g));
    hand_r = Mat4::from_translation(miss) * hand_r;
    blade = Mat4::from_translation(miss) * blade;
    let g = g + miss;
    // Then the off hand, turned as the weapon is, its grip on the line below the sword hand's.
    let place_l = Mat4::from_translation(g - line * FIST_GAP) * Mat4::from_quat(q) * Mat4::from_translation(-grip_l);
    let hand_l = place_l * rest(Bone::HandL);
    let miss = reach(sk, &mut w, arm_l, in_l, wrist_l(g));
    w[Bone::HandL.index()] = Mat4::from_translation(miss) * hand_l;
    w[Bone::HandR.index()] = hand_r;
    w[Bone::Blade.index()] = blade;
    write(sk, pose, &w, &[Bone::UpperArmL, Bone::ForearmL, Bone::HandL, Bone::UpperArmR, Bone::ForearmR, Bone::HandR, Bone::Blade]);
}

/// Where the shield stands before him to guard: its middle a little before the middle of his shield
/// forearm, wherever his guard holds that, and square to the foe (the face that looked out from his
/// back turned to the front), whatever way his chest is turned in the stance.
fn guard_place(sk: &Skeleton, w: &[Mat4]) -> Mat4 {
    let (fore, hand) = (at(&w[Bone::ForearmL.index()]), at(&w[Bone::HandL.index()]));
    let rest = sk.rest_inv[Bone::Shield.index()].inverse();
    let middle = (fore + hand) * 0.5 + GUARD_OUT;
    Mat4::from_translation(middle) * Mat4::from_rotation_y(std::f32::consts::PI) * Mat4::from_translation(-at(&rest)) * rest
}

/**
 * The shield slung on his back (riding on his chest, lower than it was carved there, `SLUNG`) or, by
 * `front` (0 to 1), brought round his shield side to stand before him on his shield forearm
 * (`guard_place`).
 */
pub fn sling(sk: &Skeleton, pose: &mut Locals, front: f32) {
    let mut w = sk.world(pose);
    let (chest, shield) = (Bone::Chest.index(), Bone::Shield.index());
    let carried = w[chest] * sk.rest_inv[chest];
    let back = carried * Mat4::from_translation(SLUNG) * sk.rest_inv[shield].inverse();
    let k = front.clamp(0.0, 1.0);
    let m = if k <= 0.0 {
        back
    } else {
        let before = guard_place(sk, &w);
        if k >= 1.0 {
            before
        } else {
            let (_, qa, ta) = back.to_scale_rotation_translation();
            let (_, qb, tb) = before.to_scale_rotation_translation();
            // (Round his shield side on the way, not through him.)
            let out = carried.transform_vector3(Vec3::X).normalize_or_zero() * SWING_OUT * (std::f32::consts::PI * k).sin();
            Mat4::from_rotation_translation(qa.slerp(qb, k).normalize(), ta.lerp(tb, k) + out)
        }
    };
    w[shield] = m;
    write(sk, pose, &w, &[Bone::Shield]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::heroes::HeroKind;
    use crate::render::anim::skeleton::joints;
    use crate::render::anim::skeleton::BONE_COUNT;

    /// A skeleton at Miyamoto's joints (bones unturned at rest, as the clips have them).
    fn miyamoto() -> (Skeleton, [Vec3; BONE_COUNT]) {
        let rig = crate::render::view::jointed_hero(HeroKind::Miyamoto, 1, Some(1)).unwrap();
        assert!(joints(HeroKind::Miyamoto).is_some());
        let at: [Vec3; BONE_COUNT] = std::array::from_fn(|i| rig.parts[i].rest);
        let rest: Locals = std::array::from_fn(|i| Trs { t: at[i] - PARENTS[i].map(|p| at[p]).unwrap_or(Vec3::ZERO), ..Trs::IDENTITY });
        (Skeleton::new(rest, Mat4::IDENTITY), at)
    }

    fn turn(pose: &mut Locals, b: Bone, q: Quat) {
        pose[b.index()].r = q * pose[b.index()].r;
    }

    #[test]
    fn both_fists_close_on_the_grip() {
        let (sk, _) = miyamoto();
        let mut pose = sk.rest_locals();
        // A guard before him: both arms forward and in, the weapon pointing up and forward.
        turn(&mut pose, Bone::UpperArmR, Quat::from_rotation_x(-1.0) * Quat::from_rotation_z(0.5));
        turn(&mut pose, Bone::ForearmR, Quat::from_rotation_x(-0.6));
        turn(&mut pose, Bone::UpperArmL, Quat::from_rotation_x(-0.8) * Quat::from_rotation_z(-0.6));
        two_hands(&sk, &mut pose);
        let w = sk.world(&pose);
        let blade = w[Bone::Blade.index()] * sk.rest_inv[Bone::Blade.index()];
        let off = w[Bone::Offhand.index()] * sk.rest_inv[Bone::Offhand.index()];
        let line = Quat::from_mat4(&blade).normalize() * weapon_rest();
        // The off hand's grip is on the weapon's line, a fist below the sword hand's, turned as it is.
        let grip_r = at(&w[Bone::Blade.index()]);
        let grip_l = at(&w[Bone::Offhand.index()]);
        let gap = grip_r - grip_l;
        assert!((gap.length() - FIST_GAP).abs() < 0.03, "the fists {:.3} apart", gap.length());
        assert!(gap.normalize().dot(line) > 0.9, "the off hand below the sword hand on the grip");
        let (qa, qb) = (Quat::from_mat4(&blade).normalize(), Quat::from_mat4(&off).normalize());
        assert!(qa.dot(qb).abs() > 0.999, "the off fist turned as the weapon is");
        // Every arm stays whole: the hands at the ends of their forearms, the forearms at the elbows.
        for (up, down) in [
            (Bone::UpperArmL, Bone::ForearmL),
            (Bone::ForearmL, Bone::HandL),
            (Bone::UpperArmR, Bone::ForearmR),
            (Bone::ForearmR, Bone::HandR),
        ] {
            let t = pose[down.index()].t;
            assert!((t - sk.rest[down.index()].t).length() < 1e-3, "{down:?} parted from {up:?}");
        }
    }

    #[test]
    fn the_shield_rides_his_back_and_comes_before_him_to_guard() {
        let (sk, at0) = miyamoto();
        // Slung, it rides his chest (lower than it was carved), however his arm swings.
        let mut pose = sk.rest_locals();
        turn(&mut pose, Bone::UpperArmL, Quat::from_rotation_x(0.9));
        turn(&mut pose, Bone::Spine, Quat::from_rotation_y(0.4));
        sling(&sk, &mut pose, 0.0);
        let m = sk.matrices(&pose, &at0, &[Quat::IDENTITY; BONE_COUNT]);
        let chest = m[Bone::Chest.index()];
        let shield = m[Bone::Shield.index()];
        let slung = at0[Bone::Shield.index()] + SLUNG - at0[Bone::Chest.index()];
        assert!((chest.inverse() * shield).abs_diff_eq(Mat4::from_translation(slung), 1e-4));
        assert!(m[Bone::Shield.index()].w_axis.z < 0.0, "on his back");
        // Guarding, it stands before him, its face to the foe, however his chest is turned.
        let mut raised = sk.rest_locals();
        turn(&mut raised, Bone::Spine, Quat::from_rotation_y(-0.6));
        turn(&mut raised, Bone::UpperArmL, Quat::from_rotation_x(-1.2));
        turn(&mut raised, Bone::ForearmL, Quat::from_rotation_y(-1.2));
        sling(&sk, &mut raised, 1.0);
        let m = sk.matrices(&raised, &at0, &[Quat::IDENTITY; BONE_COUNT]);
        let shield = m[Bone::Shield.index()];
        assert!(shield.w_axis.z > 0.1, "before him: {}", shield.w_axis);
        assert!(shield.transform_vector3(Vec3::Z).z < -0.99, "turned round");
        // Half way, it is out at his side.
        let mut half = sk.rest_locals();
        sling(&sk, &mut half, 0.5);
        let m = sk.matrices(&half, &at0, &[Quat::IDENTITY; BONE_COUNT]);
        assert!(m[Bone::Shield.index()].w_axis.x > 0.1, "round his shield side: {}", m[Bone::Shield.index()].w_axis);
    }
}
