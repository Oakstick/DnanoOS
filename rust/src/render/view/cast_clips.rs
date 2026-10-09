//! The story's cast by their clips (ANIMATION.md §12): each moment of a scene as the staging in
//! `story` and `ascent` sets it out (what a boy is doing, `Act`; the old man's, the dog's, Krampus's
//! moments) becomes a `Play` of their clips, placed on the story's timings (a kick's contact on the
//! ball's moment, the grab on the grab's), and the cast animator (`anim::cast`) gives each part's
//! matrix; the view lays its touches on the pose (a head turned where the story looks, the old man's
//! staff held upright, a scarf and a beard hanging and swinging after the body). The code's poses stay
//! the fallback (`VH_ANIM=code`, a clip missing) and the comparison (`--anim-compare`).

use std::f32::consts::{PI, TAU};

use super::story::Act;
use super::*;
use crate::logic::story::stage;
use crate::render::anim::cast::{self, CastAnimator, Delta, Layer, Mask, Play};
use crate::render::anim::cast_faces::{self, Face};
use crate::render::anim::cast_rigs::CastRig;
use crate::render::anim::library::{self, ClipSet, Mode};
use crate::render::anim::skeleton::SkeletonDef;

/// How many of the cast have an animator (the four boys, the old man, the dog, Krampus).
const SLOTS: usize = 7;

/// The cast's animators, kept from frame to frame for their crossfades (drawing is `&self`).
#[derive(Default)]
pub(super) struct CastAnims {
    anims: [CastAnimator; SLOTS],
    /// The scene's clock when each was last posed (a jump in it starts afresh).
    at: [Option<f32>; SLOTS],
    /// What hangs and swings after the body (a scarf's end, the beard, the robe), up to four a slot.
    swing: [[Option<Quat>; 4]; SLOTS],
    /// Each one's parts' matrices as last drawn (the tests look at them).
    pub(super) last: [Vec<Mat4>; SLOTS],
}

/// The turn about the upright that `q` makes of the way forward: its heading alone.
pub(super) fn heading(q: Quat) -> Quat {
    let f = q * Vec3::Z;
    Quat::from_rotation_y(f.x.atan2(f.z))
}

/// The heading of a bone's turn (about the upright, toward its left), radians.
fn yaw_of(q: Quat) -> f32 {
    let f = q * Vec3::Z;
    f.x.atan2(f.z)
}

/// Squares the chest, neck and head to the hips (their twist about the upright taken out), so a
/// clip's body faces where the story faces it: the boy who screams "He took him!" screams it at us.
pub(super) fn square_up(pose: &mut Delta, def: &SkeletonDef) {
    let Some(hips) = def.index("hips") else { return };
    for b in ["spine", "chest", "neck", "head"] {
        let Some(i) = def.index(b) else { continue };
        let twist = yaw_of(pose.rot[i]) - yaw_of(pose.rot[hips]);
        pose.turn(def, i, Quat::from_rotation_y(-twist));
    }
}

/// The head's turn about the upright away from the chest's, in radians (toward its left).
fn head_yaw(pose: &Delta, def: &SkeletonDef) -> f32 {
    let (Some(chest), Some(head)) = (def.index("chest"), def.index("head")) else { return 0.0 };
    let (c, h) = (pose.rot[chest] * Vec3::Z, pose.rot[head] * Vec3::Z);
    let d = h.x.atan2(h.z) - c.x.atan2(c.z);
    (d + PI).rem_euclid(TAU) - PI
}

/// Turns the head to look `yaw` radians from the chest's heading (toward its left) and `pitch`
/// down, taking back `steady` of the clip's own turn of it first (a speaker who in his clip talks
/// to someone at his side looks where the story looks).
pub(super) fn look_steady(pose: &mut Delta, def: &SkeletonDef, yaw: f32, pitch: f32, steady: f32) {
    let own = head_yaw(pose, def);
    look(pose, def, yaw - own * steady, pitch);
}

/// Turns the head `yaw` (toward its left) and `pitch` (down) radians in the body, the neck taking
/// two fifths of it.
pub(super) fn look(pose: &mut Delta, def: &SkeletonDef, yaw: f32, pitch: f32) {
    if yaw == 0.0 && pitch == 0.0 {
        return;
    }
    let (Some(neck), Some(head)) = (def.index("neck"), def.index("head")) else { return };
    pose.turn(def, neck, Quat::from_rotation_y(yaw * 0.4) * Quat::from_rotation_x(pitch * 0.4));
    pose.turn(def, head, Quat::from_rotation_y(yaw * 0.6) * Quat::from_rotation_x(pitch * 0.6));
}

impl WorldView {
    /// Makes the cast's jointed rigs (and their meshes at every level of detail) and reads their
    /// clips, as a scene begins: about 120 ms for Scene I's (behind its loading screen) that would
    /// otherwise fall on its first frame; Scene II has only the freed boy.
    pub fn prepare_cast(&self, scene_two: bool) {
        if self.anim_mode == Mode::Code {
            return;
        }
        let cast: &[CastRig] = if scene_two { &[CastRig::Kid(0)] } else { &CastRig::ALL };
        for &c in cast {
            library::cast_clips(c);
            let rig = self.models.jointed_cast(c);
            for level in 0..LOD_LEVELS {
                rig.lod(level).mesh();
            }
        }
        library::foe_clips(crate::render::anim::foe_rigs::FoeRig::Krampus);
    }

    /**
     * Cast member `c`'s jointed rig and each part's matrix in the model for `play` at the scene's
     * clock `time`, `touch` laid on the pose first; none when the code animates the cast
     * (`VH_ANIM=code`, the code half of `--anim-compare`) or a clip the moment needs is missing.
     */
    pub(super) fn cast_mats(
        &self,
        c: CastRig,
        play: &Play,
        time: f32,
        touch: impl FnOnce(&mut Delta, &SkeletonDef),
    ) -> Option<(&Rig, Vec<Mat4>)> {
        if self.anim_mode == Mode::Code {
            return None;
        }
        let own = library::cast_clips(c);
        let mut sources: Vec<(&ClipSet, f32)> = own.into_iter().map(|s| (s, 1.0)).collect();
        if c == CastRig::Krampus {
            // (The lair's Krampus's clips, his body a size and two fifths of this one's.)
            if let Some(lair) = library::foe_clips(crate::render::anim::foe_rigs::FoeRig::Krampus) {
                sources.push((lair, crate::render::cast::KRAMPUS_VOXEL / crate::render::foes_lair::KRAMPUS_VOXEL));
            }
        }
        if sources.is_empty() {
            return None;
        }
        let def = c.joints().def;
        let slot = c.slot();
        let mut st = self.cast_anim.borrow_mut();
        let dt = st.at[slot].map(|a| time - a).filter(|d| (0.0..0.25).contains(d));
        st.at[slot] = Some(time);
        let mut pose = st.anims[slot].step(&sources, def, play, dt)?.clone();
        touch(&mut pose, def);
        // What hangs: a boy's scarf's end, the old man's beard; it hangs down whichever way he turns,
        // swinging after him.
        // The old man's robe is one garment: it hangs from his hips, following them nearly whole and
        // a little the swing of his legs beneath it, the hem lagging after.
        let hung: &[(&str, f32)] = match c {
            CastRig::Kid(_) => &[("scarf", 0.35)],
            CastRig::OldMan => &[("beard.1", 0.6), ("beard.2", 0.35), ("robe", 0.9), ("robe.hem", 0.75)],
            _ => &[],
        };
        let legs = match (def.index("hips"), def.index("thigh.L"), def.index("thigh.R")) {
            (Some(h), Some(l), Some(r)) if c == CastRig::OldMan => {
                let (a, b) = (pose.rot[h].inverse() * pose.rot[l], pose.rot[h].inverse() * pose.rot[r]);
                Quat::IDENTITY.slerp(a.slerp(if a.dot(b) < 0.0 { -b } else { b }, 0.5), 0.35)
            }
            _ => Quat::IDENTITY,
        };
        for (k, &(bone, follow)) in hung.iter().enumerate() {
            let (Some(b), Some(p)) = (def.index(bone), def.index(bone).and_then(|b| def.parents[b])) else { continue };
            let parent = pose.rot[p];
            let mut want = heading(parent).slerp(parent, follow);
            if bone == "robe" {
                want = (want * legs).normalize();
            }
            let now = match (st.swing[slot][k], dt) {
                (Some(was), Some(dt)) => was.slerp(want, 1.0 - (-dt * 9.0).exp()).normalize(),
                _ => want,
            };
            st.swing[slot][k] = Some(now);
            pose.rot[b] = now;
        }
        drop(st);
        let rig = self.models.jointed_cast(c);
        let joints: Vec<Vec3> = rig.parts.iter().map(|p| p.rest).collect();
        let mut mats = Vec::with_capacity(joints.len());
        cast::matrices(def, &joints, &pose, &mut mats);
        if c == CastRig::OldMan {
            hold_staff(rig, def, &mut mats);
        }
        self.cast_anim.borrow_mut().last[slot].clone_from(&mats);
        Some((rig, mats))
    }

    /// A boy by his clips: `which` doing `act` (the scene's clock `time`), his head turned `look`
    /// radians, at `root`. False when the code must draw him.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn emit_kid_clips(
        &self,
        f: &mut Frame,
        which: usize,
        root: Mat4,
        act: Act,
        time: f32,
        look_yaw: f32,
        glow: Vec3,
        alpha: f32,
    ) -> bool {
        let play = kid_play(which, act, time);
        let pitch = match act {
            Act::Gaze(k) => -0.85 * k,
            Act::HandUp(_, g) => -0.85 * g,
            Act::Peer => 0.15,
            // (The one who screams it leans back in his clip: his head kept up to the camera.)
            Act::Yell(_, 0) => 0.25,
            Act::Yell(..) => -0.1,
            _ => 0.0,
        };
        let steady = match act {
            Act::Talk(_) | Act::Stand | Act::Scared(_) | Act::Nod(..) => 0.6,
            // (The one who screams it faces where he screams, whatever his clip does with his head.)
            Act::Yell(_, 0) => 1.0,
            _ => 0.0,
        };
        let Some((rig, mut mats)) = self.cast_mats(CastRig::Kid(which.min(3) as u8), &play, time, |pose, def| {
            if let Act::Dash { dist, speed, .. } = act {
                dash_touch(pose, def, which, dist, speed);
            }
            if matches!(act, Act::Yell(_, 0)) {
                square_up(pose, def);
            }
            look_steady(pose, def, look_yaw, pitch, steady)
        }) else {
            return false;
        };
        cast_faces::wear(rig, &mut mats, self.kid_face(which, act, time));
        if alpha < 0.99 {
            f.rigs.push_matrices(rig, &mats, root, Vec3::ONE, alpha, Vec3::ZERO, rig_look::FADED, 0.0);
        } else {
            f.rigs.push_matrices(rig, &mats, root, Vec3::ONE, 1.0, glow, rig_look::PLAIN, 0.0);
        }
        true
    }
}

impl WorldView {
    /**
     * A boy's face this frame (`anim::cast_faces`): shouting "He took him!" his mouth wide while
     * the line types and a moment after, then gasping; speaking, it opens and shuts as his line
     * types (shut at its stops, wider on a shouted line), shut once it is said; and every few
     * seconds, each boy at his own time, a blink (not while he screams).
     */
    pub(super) fn kid_face(&self, which: usize, act: Act, time: f32) -> Face {
        let typing = self.story.map(|sv| {
            let line = sv.scene.beats()[sv.beat.min(sv.scene.beats().len() - 1)].line.as_bytes();
            let at = sv.shown as usize;
            let stop = at > 0 && at < line.len() && matches!(line[at - 1], b'.' | b',' | b'!' | b'?' | b'-');
            (at < line.len() && !stop, line.contains(&b'!'))
        });
        let flap = ((time * 4.6 + which as f32 * 0.37) * TAU).sin() * 0.5 + 0.5;
        let mouth = match act {
            Act::Yell(t, _) => {
                let shouting = typing.is_some_and(|(on, _)| on) || t < 3.2;
                if shouting {
                    if flap < 0.12 {
                        1
                    } else {
                        2
                    }
                } else {
                    // Out of breath after it.
                    (((time * 1.6 + which as f32 * 0.5) * TAU).sin() > -0.2) as u8
                }
            }
            Act::Talk(_) | Act::Nod(_, true) => match typing {
                Some((true, shout)) if flap > 0.75 && shout => 2,
                Some((true, _)) if flap > 0.4 => 1,
                _ => 0,
            },
            _ => 0,
        };
        let period = 3.3 + which as f32 * 0.7;
        let blink = !matches!(act, Act::Yell(..)) && (time + which as f32 * 1.9).rem_euclid(period) < 0.13;
        Face { mouth, blink }
    }
}

/// The idle each boy stands in, so the four never breathe in step.
const IDLES: [&str; 4] = ["idle_happy", "idle", "idle_happy_2", "look"];

/// The boys' gaits, slowest first: the clip, its length (seconds a cycle), the ground a cycle of it
/// carries a boy at his scale with his planted foot fixed (metres: calibrated,
/// `tests::the_boys_feet_stay_planted_as_they_run`; less than the source's own travel where it has a
/// flight; Mixamo's Jogging was tried: its stance is short and its flight long, and played faster or
/// lengthened its planted foot slips, so the run takes over from the walk), how much of a lengthened stride (`anim::cast::lengthen`) its planted foot takes up
/// (the swing's reach grows more than the stance's), and the speed (metres a second) up to which
/// each is used.
pub(super) const GAITS: [(&str, f32, f32, f32, f32); 3] =
    [("walk", 82.0 / 60.0, 0.25, 0.8, 0.4), ("run", 32.0 / 60.0, 0.50, 0.75, 1.8), ("sprint", 32.0 / 60.0, 0.50, 0.75, f32::MAX)];

/// How far a boy's legs may reach past the clip's (`anim::cast::lengthen`): longer, and the run
/// reads as a child's legs flung out frantically (the 1.0.16 release's 1.6).
const REACH: f32 = 1.45;

/// A boy's run at a speed: the clip, how much further his legs reach than its own
/// (`anim::cast::lengthen`), and the ground a cycle of it covers (a stride: two steps).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Gait {
    pub clip: &'static str,
    pub gain: f32,
    pub stride: f32,
    /// Placed on the ground by its planted feet, its flight stretched over the rest (`Stride`); the
    /// walk, which has no flight, is played evenly.
    pub flies: bool,
}

/// Strides a second a boy's run turns over at `speed` (metres a second): a child running hard, not
/// a wind-up toy (the 1.0.16 release's ran three to four a second at 3 to 4.3 m/s).
pub(super) fn cadence(speed: f32) -> f32 {
    (1.75 + 0.24 * speed).min(2.9)
}

/// The gait a boy runs at `speed`: his speed over the clip's own pace shared between a longer reach
/// of his legs (at most `REACH` times the clip's, in steps of a twentieth, as `stride_of` measures
/// them) and, for the run and the sprint, a long flight between his steps at a child's cadence.
pub(super) fn gait(which: usize, speed: f32) -> Gait {
    let k = GAITS.iter().position(|g| speed < g.4).unwrap_or(GAITS.len() - 1);
    // (Two sprint and two run, where it is a sprint at all.)
    let k = if k == 2 && which % 2 == 1 { 1 } else { k };
    let (clip, len, stride, take, _) = GAITS[k];
    let gain = ((speed / (stride / len)).sqrt().clamp(1.0, REACH) * 20.0).round() / 20.0;
    let planted = stride * (1.0 + take * (gain - 1.0));
    if k == 0 {
        return Gait { clip, gain, stride: planted, flies: false };
    }
    Gait { clip, gain, stride: (speed / cadence(speed)).max(planted), flies: true }
}

/// Gait `clip`'s cycle measured at a reach of `gain` (once each, kept).
pub(super) fn stride_of(clip: &'static str, gain: f32) -> Option<std::sync::Arc<cast::Stride>> {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, OnceLock};
    type Measured = Mutex<HashMap<(&'static str, u32), Arc<cast::Stride>>>;
    static MEASURED: OnceLock<Measured> = OnceLock::new();
    let key = (clip, (gain * 20.0).round() as u32);
    let mut map = MEASURED.get_or_init(Default::default).lock().ok()?;
    if let Some(s) = map.get(&key) {
        return Some(s.clone());
    }
    let c = CastRig::Kid(0);
    let (set, def) = (library::cast_clips(c)?, c.joints().def);
    let source = set.get(clip)?;
    let joints: Vec<Vec3> = (0..def.len()).map(|b| source.skeleton.rest_inv[b].inverse().w_axis.truncate()).collect();
    let legs: Vec<usize> = LEGS.iter().filter_map(|b| def.index(b)).collect();
    let feet = [def.index("foot.L")?, def.index("foot.R")?];
    let s = Arc::new(cast::Stride::measure(source, def, &joints, &legs, feet, gain));
    map.insert(key, s.clone());
    Some(s)
}

/// The bones a run lengthens.
const LEGS: [&str; 4] = ["thigh.L", "thigh.R", "shin.L", "shin.R"];

/// Boy `which` running at `speed`, `dist` over the ground: his gait, the share of its cycle he is
/// at, and how far through a flight between steps he is (none with a foot down).
pub(super) fn dash(which: usize, dist: f32, speed: f32) -> (Gait, f32, Option<f32>) {
    let g = gait(which, speed);
    let offset = which as f32 * 0.17;
    match g.flies.then(|| stride_of(g.clip, g.gain)).flatten() {
        Some(s) => {
            let (u, air) = s.at(dist + offset * g.stride, g.stride);
            (g, u, air)
        }
        None => (g, dist / g.stride + offset, None),
    }
}

/**
 * What the view lays on a running boy's clip: his legs reaching further (`gain`), his arms driving
 * harder, his body leaning into the run and rising through each flight (it carries the ground the
 * planted feet do not: `Stride`), his head kept up. The tests run the same.
 */
pub(super) fn dash_touch(pose: &mut Delta, def: &SkeletonDef, which: usize, dist: f32, speed: f32) {
    let (g, _, air) = dash(which, dist, speed);
    if g.gain > 1.0 {
        let legs: Vec<usize> = LEGS.iter().filter_map(|b| def.index(b)).collect();
        cast::lengthen(pose, def, &legs, g.gain);
    }
    if !g.flies {
        let arms: Vec<usize> = ["upper_arm.L", "upper_arm.R"].iter().filter_map(|b| def.index(b)).collect();
        cast::lengthen(pose, def, &arms, g.gain.sqrt());
        return;
    }
    // Arms pumping: the upper arms swing further, the elbows a little more bent.
    let hard = ((speed - 1.5) / 2.5).clamp(0.0, 1.0);
    let arms: Vec<usize> = ["upper_arm.L", "upper_arm.R"].iter().filter_map(|b| def.index(b)).collect();
    cast::lengthen(pose, def, &arms, 1.0 + (g.gain - 1.0) * 0.8 + 0.25 * hard);
    let fore: Vec<usize> = ["forearm.L", "forearm.R"].iter().filter_map(|b| def.index(b)).collect();
    cast::lengthen(pose, def, &fore, 1.0 + 0.3 * hard);
    // Leaning into it from the hips, the head kept level to look where he runs.
    let lean = 0.06 + 0.16 * hard;
    if let Some(spine) = def.index("spine") {
        pose.turn(def, spine, Quat::from_rotation_x(lean));
    }
    look(pose, def, 0.0, -lean * 0.7);
    // Up through each flight: the ground it carries over at his speed, as a ball thrown.
    if let Some(p) = air {
        let s = stride_of(g.clip, g.gain).map(|s| s.flight(g.stride) / 2.0).unwrap_or(0.0);
        let time = s / speed.max(0.1);
        let rise = (9.8 * time * time / 8.0 * 1.6).min(0.045);
        pose.root.y += 4.0 * rise * p * (1.0 - p);
    }
}

/// What a boy's clips play for `act`.
pub(super) fn kid_play(which: usize, act: Act, time: f32) -> Play {
    let off = which as f32 * 1.37;
    let idle = Layer::of(IDLES[which % 4], time + off * 2.0).or("idle");
    match act {
        Act::Stand => Play::base(idle, 1),
        Act::Run { phase, pace } => {
            let cycle = phase / TAU + which as f32 * 0.17;
            if pace < 0.7 {
                // Walking, or the first steps out of a stand.
                let walk = Layer::cycle("walk", cycle * 0.5);
                if pace < 0.45 {
                    Play::base(idle, 1).with(walk.weight(pace / 0.45))
                } else {
                    Play::base(walk, 2)
                }
            } else {
                Play::base(Layer::cycle(if which.is_multiple_of(2) { "sprint" } else { "run" }, cycle).or("run"), 3)
            }
        }
        // Over the ground: the cycle by the ground covered, so the feet stay planted.
        Act::Dash { dist, speed, pace, .. } => {
            let (g, u, _) = dash(which, dist, speed);
            let layer = Layer::cycle(g.clip, u).or("run");
            if pace < 0.99 {
                Play::base(idle, 17).with(layer.weight(pace))
            } else {
                Play::base(layer, 18)
            }
        }
        // A kick: the contact on the ball's moment; the four pass each their own way.
        Act::Kick(k) => {
            let clip = ["pass", "pass_left", "kick", "pass"][which % 4];
            Play::base(Layer::at_mark(clip, "hit", k), 10 + which as u32).fading(0.1)
        }
        Act::Miskick(k) => Play::base(Layer::at_mark("miskick", "hit", k), 16).fading(0.1),
        Act::Peer => Play::base(Layer::of("look_nervous", time + off), 4),
        // (Terror's crouch, fists up before his face, trembling: the clip's second to fourth seconds
        // swung back and forth, not its long cower with his head down.)
        Act::Scared(a) => Play::base(idle, 5).with(Layer::of("terrified", 3.0 + (time * 0.8 + off).sin()).weight(a.min(1.0))),
        Act::Talk(t) => Play::base(Layer::of(if which.is_multiple_of(2) { "talk" } else { "talk_m" }, t + off), 6),
        Act::Up(u) => Play::base(Layer::at_mark("land", "land", 0.4 + u * 1.5), 7),
        Act::Land(t) => Play::base(Layer::at_mark("land", "land", t + which as f32 * 0.12), 7).fading(0.05),
        Act::Flail(t) => Play::base(Layer::of("fall", t + off), 8).fading(0.05),
        // "He took him!": each his own way. The one who screams it leans back yelling, both arms
        // flung up; one recoils, arms up before his face; one yells after him, an arm flung out at
        // the gate, the other up.
        Act::Yell(t, k) => {
            let arms = Mask::Under(&["upper_arm.L", "upper_arm.R"]);
            match k {
                0 => Play::base(Layer::of("yell_back", 1.0 + t * 0.7).or("yell"), 9).with(Layer::of("arms_up", time).on(arms)),
                1 => Play::base(Layer::of("alarm", time + off), 19)
                    .with(Layer::of("yell_long", 1.2 + t * 0.6).on(Mask::Under(&["neck"])).weight(0.6)),
                _ => Play::base(Layer::of("yell_long", 1.4 + t * 0.6).or("yell"), 20).with(Layer::of("reach_out", time).on(arms)),
            }
            .fading(0.12)
        }
        Act::Hug(k) => Play::base(Layer::of("hug", time), 12).with(idle.weight(1.0 - k)),
        Act::Gaze(_) => Play::base(Layer::of("idle", time + off), 13),
        Act::HandUp(k, _) => Play::base(Layer::of("idle", time + off), 13).with(Layer::of("reach", 1.5).on(Mask::Upper).weight(k)),
        Act::Nod(t, speaking) => Play::base(Layer::of("nod", t + off * 0.2), 14)
            .with(Layer::of("talk", t).on(Mask::Under(&["upper_arm.L", "upper_arm.R"])).weight(if speaking { 0.6 } else { 0.0 })),
        Act::Back(t, a) => {
            Play::base(Layer::of("walk_back", t * 1.3), 15).with(Layer::of("terrified", time + off).on(Mask::Upper).weight(a))
        }
    }
}

/// The old man's moments, as the gateway's staging sets them out: on his feet by his corner the
/// whole scene, leaning on his staff.
#[derive(Clone, Copy, Debug)]
pub(super) enum OldAct {
    /// Standing, dozing over his staff (1 asleep, his chin on his chest) or awake.
    Stand { doze: f32 },
    /// Speaking (seconds into the line; which of his lines).
    Talk { t: f32, line: usize },
    /// His free hand held out (to the dog, to the door), 0 to 1, as he speaks.
    Hand { t: f32, raise: f32 },
    /// Looking away, nodding in his guilt.
    Nod { t: f32 },
    /// Stepping aside (`pace` 0 standing, 1 shuffling; `cycle` through his shuffle), a hand raised
    /// (`wave`).
    Step { pace: f32, cycle: f32, wave: f32 },
}

/// His gestures cover his upper body but the arm that holds his staff.
const GESTURE: Mask = Mask::UpperBut(&["upper_arm.R"]);

pub(super) fn old_man_play(a: OldAct, time: f32) -> Play {
    let idle = |slow: f32| Layer::of("idle", time * slow);
    match a {
        OldAct::Stand { doze } => Play::base(idle(1.0 - 0.5 * doze), 1).fading(0.5),
        OldAct::Talk { t, line } => Play::base(idle(1.0), 2 + (line % 2) as u32)
            .with(Layer::of(if line.is_multiple_of(2) { "talk" } else { "talk_m" }, t + 0.5).on(GESTURE).weight(0.8))
            .fading(0.5),
        OldAct::Hand { t, raise } => Play::base(idle(1.0), 4)
            .with(Layer::of("talk", t).on(GESTURE).weight(0.5))
            .with(Layer::of("point", 0.8 + 0.6 * raise).on(Mask::Under(&["upper_arm.L"])).weight(raise))
            .fading(0.5),
        OldAct::Nod { t } => Play::base(idle(1.0), 5).with(Layer::of("nod", t).on(GESTURE)).fading(0.5),
        OldAct::Step { pace, cycle, wave } => Play::base(idle(1.0), 7)
            .with(Layer::cycle("walk", cycle).weight(pace))
            .with(Layer::of("wave", time).on(Mask::Under(&["upper_arm.L"])).weight(wave))
            .fading(0.4),
    }
}

/// Quitmer's moments.
#[derive(Clone, Copy, Debug)]
pub(super) enum DogAct {
    /// On his straw: asleep (`sleep` 1) or watching, his tail going (`wag`).
    Lie { sleep: f32, wag: f32 },
    /// Getting up (0 lying, 1 on his feet).
    Rise(f32),
    /// On his feet, straining at his chain (`strain` 0 to 1).
    Strain(f32),
    /// Going back down onto his straw (0 standing, 1 lying).
    Down(f32),
}

pub(super) fn dog_play(a: DogAct, time: f32) -> Play {
    match a {
        DogAct::Lie { sleep, wag } => {
            let lie = Layer::of(if wag > 0.3 { "lie_wag" } else { "lie" }, time);
            Play::base(lie, 1).with(Layer::of("sleep", time).weight(sleep)).fading(0.5)
        }
        DogAct::Rise(u) => Play::base(Layer::cycle("stand_up", u.min(0.999)), 2).fading(0.2),
        DogAct::Strain(s) => Play::base(Layer::of("idle", time), 3).with(Layer::of("snarl", time).weight(s)).fading(0.2),
        DogAct::Down(u) => Play::base(Layer::cycle("lie_down", u.min(0.999)), 4).fading(0.3),
    }
}

/// Krampus in Scene I, `k` seconds into the beat he takes the boy: in the fog, the rush, the grab
/// through the gate, and back into the graves with his catch held up.
pub(super) fn krampus_play(k: f32, phase: f32) -> Play {
    let cycle = phase / TAU;
    if k < stage::RUSH {
        Play::base(Layer::of("idle", k + 2.0), 1)
    } else if k < stage::GATE_BURST {
        Play::base(Layer::cycle("run", cycle), 2).fading(0.15)
    } else if k < stage::TURN + 0.1 {
        Play::base(Layer::at_mark("grab", "grab", k - stage::GRAB), 3).fading(0.06)
    } else {
        Play::base(Layer::cycle("run", cycle), 4).with(Layer::of("hold_up", k).on(Mask::Under(&["upper_arm.R"]))).fading(0.25)
    }
}

/// Where the old man's hand holds his staff, in his chest's frame as it stands at rest (metres of his
/// model): out at his right side, a little before him, at about his waist; and how far the staff
/// leans out from him (radians).
pub(super) const STAFF_HAND: Vec3 = Vec3::new(-0.31, 0.56, 0.12);
const STAFF_LEAN: f32 = 0.12;

/// Aims the old man's staff arm (his right) so his hand is at `STAFF_HAND` beside him, whatever his
/// clip does with it: the arm turns as one from the shoulder, riding his chest.
pub(super) fn staff_arm(pose: &mut Delta, def: &SkeletonDef, joints: &[Vec3]) {
    let (Some(ch), Some(up), Some(fore), Some(hand), Some(grip)) =
        (def.index("chest"), def.index("upper_arm.R"), def.index("forearm.R"), def.index("hand.R"), def.index("blade"))
    else {
        return;
    };
    let s = joints[up];
    let q = Quat::from_rotation_arc((joints[grip] - s).normalize(), (STAFF_HAND - s).normalize());
    let r = (pose.rot[ch] * q).normalize();
    for b in [up, fore, hand] {
        pose.rot[b] = r;
    }
}

/**
 * The old man's staff beside him, never through him: it passes through the fist of his staff arm
 * (where `staff_arm` holds it), standing up out of his heading and leaning a little out to his
 * right, its foot on the floor (it slides through his fist to reach it).
 */
pub(super) fn hold_staff(rig: &Rig, def: &SkeletonDef, mats: &mut [Mat4]) {
    let (Some(hips), Some(hand), Some(staff)) = (def.index("hips"), def.index("hand.R"), def.index("blade")) else { return };
    let grip = mats[hand].transform_point3(rig.parts[staff].rest - rig.parts[hand].rest);
    let turn = heading(Quat::from_mat4(&mats[hips]).normalize()) * Quat::from_rotation_z(STAFF_LEAN);
    let up = turn * Vec3::Y;
    // (Its foot: the lowest of its voxels below its joint.)
    let foot = rig.parts[staff].cubes.iter().map(|(o, _)| o.y).fold(0.0f32, f32::min) - rig.size * 0.5;
    let joint = grip + up * ((-foot * up.y - grip.y) / up.y);
    mats[staff] = Mat4::from_rotation_translation(turn, joint);
}

/// Where a point of cast member `c`'s grid is, on bone `bone` posed by `mats`.
pub(super) fn bone_point(c: CastRig, rig: &Rig, mats: &[Mat4], bone: &str, g: [f32; 3]) -> Option<Vec3> {
    let b = rig.index(bone)?;
    let p = crate::render::anim::cast_rigs::grid_point(c, g);
    Some(mats.get(b)?.transform_point3(p - rig.parts[b].rest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;
    use crate::logic::story::{Place, SCENE_ONE};

    /// The old man as his clips posed him at every moment of the gateway (each action beat through
    /// its hold, each line through its first 12 s, every 0.1 s), with his rig and the scene's clock.
    fn old_man_through_the_gateway(mut look: impl FnMut(usize, f32, &Rig, &[Mat4])) {
        let mut game = Game::new();
        let c = CastRig::OldMan;
        for (beat, b) in SCENE_ONE.iter().enumerate() {
            if b.shot.place() != Place::Gateway {
                continue;
            }
            let end = if b.waits() { 12.0 } else { b.hold as f32 - 0.01 };
            let mut t = 0.0;
            while t <= end {
                game.story_jump(beat, t as f64);
                let sv = game.story.as_ref().unwrap().view();
                game.view.story = Some(sv);
                let mut f = Frame::default();
                game.view.emit(&game.scene(), &mut f);
                let mats = game.view.cast_anim.borrow().last[c.slot()].clone();
                assert!(!mats.is_empty(), "beat {beat} at {t:.1}: the old man was not posed by his clips");
                look(beat, t, game.view.models.jointed_cast(c), &mats);
                t += 0.1;
            }
        }
    }

    /// The user: "his stick is cutting his body". His staff stands beside him at every moment of the
    /// gateway: none of its voxels inside the box round any part of his body (his head, hood and
    /// beard, his chest and cape, his robe, his free arm; but the arm that holds it), and its foot on
    /// the flagstones, not in them.
    #[test]
    fn the_old_mans_staff_never_passes_through_him_and_stands_on_the_floor() {
        let def = CastRig::OldMan.joints().def;
        let staff = def.index("blade").unwrap();
        let holding: Vec<usize> = ["upper_arm.R", "forearm.R", "hand.R", "blade"].iter().filter_map(|b| def.index(b)).collect();
        let mut faults = Vec::new();
        old_man_through_the_gateway(|beat, t, rig, mats| {
            let h = rig.size * 0.5;
            let points: Vec<Vec3> = rig.parts[staff].cubes.iter().map(|(o, _)| mats[staff].transform_point3(*o)).collect();
            let low = points.iter().map(|p| p.y).fold(f32::MAX, f32::min) - h;
            if !(-0.012..0.02).contains(&low) {
                faults.push(format!("beat {beat} at {t:.1}: its foot at {low:.3}"));
            }
            for (i, part) in rig.parts.iter().enumerate() {
                if holding.contains(&i) || part.cubes.is_empty() {
                    continue;
                }
                let lo = part.cubes.iter().fold(Vec3::MAX, |a, (o, _)| a.min(*o)) - Vec3::splat(h);
                let hi = part.cubes.iter().fold(Vec3::MIN, |a, (o, _)| a.max(*o)) + Vec3::splat(h);
                let inv = mats[i].inverse();
                if let Some(p) = points.iter().find(|p| {
                    let q = inv.transform_point3(**p);
                    q.cmpgt(lo).all() && q.cmplt(hi).all()
                }) {
                    faults.push(format!("beat {beat} at {t:.1}: the staff at {p:.2} inside his {}", part.name));
                    break;
                }
            }
        });
        assert!(faults.is_empty(), "{} moments:\n{}", faults.len(), faults.iter().take(30).cloned().collect::<Vec<_>>().join("\n"));
    }

    /// How far a boy's planted foot slides over the ground, as a share of the ground he covers,
    /// running at `speed` with his clips played by `act_at(t, dist)`.
    fn slide(which: usize, speed: f32, act_at: impl Fn(f32, f32) -> Act) -> f32 {
        use crate::render::anim::cast::matrices;
        let c = CastRig::Kid(1);
        let (set, def) = (library::cast_clips(c).unwrap(), c.joints().def);
        let rig = c.jointed();
        let joints: Vec<Vec3> = rig.parts.iter().map(|p| p.rest).collect();
        let feet = [def.index("foot.L").unwrap(), def.index("foot.R").unwrap()];
        let mut a = CastAnimator::default();
        let (dt, mut mats) = (1.0 / 60.0, Vec::new());
        let mut track: Vec<[Vec3; 2]> = Vec::new();
        for k in 0..240 {
            let t = k as f32 * dt;
            let dist = speed * t;
            let act = act_at(t, dist);
            let mut pose = a.step(&[(set, 1.0)], def, &kid_play(which, act, t), Some(dt)).unwrap().clone();
            if let Act::Dash { dist, speed, .. } = act {
                dash_touch(&mut pose, def, which, dist, speed);
            }
            matrices(def, &joints, &pose, &mut mats);
            // (Each ankle.)
            track.push([0, 1].map(|i| mats[feet[i]].w_axis.truncate() + Vec3::new(0.0, 0.0, dist)));
        }
        let floor = track.iter().flat_map(|f| f.iter().map(|p| p.y)).fold(f32::MAX, f32::min);
        // (Over the frames a foot is planted: how far it moves, against how far he goes meanwhile.)
        let (mut slid, mut covered) = (0.0f32, 0.0f32);
        for w in track.windows(2).skip(30) {
            // (The lower foot, while it is within a few millimetres of the lowest it goes.)
            let i = if w[0][0].y < w[0][1].y { 0 } else { 1 };
            if w[0][i].y < floor + 0.02 && w[1][i].y < floor + 0.02 {
                slid += w[1][i].z - w[0][i].z;
                covered += speed * dt;
            }
        }
        slid / covered.max(1e-6)
    }

    /// The boys' feet stay where they land: run at the speeds the story runs them (the fetch, the
    /// chase after Krampus, Scene II's run), their planted foot slides under a tenth of the ground
    /// they cover (it slid as much as the clip played at the old rate fell short of it). And they run
    /// like children running hard, not wind-up toys: at the chase's speeds their strides turn over
    /// under three times a second (the 1.0.16 release's, four), a long flight between their steps.
    #[test]
    fn the_boys_feet_stay_planted_as_they_run() {
        let mut worst = 0.0f32;
        for (which, speed) in [0.25, 0.6, 0.95, 1.2, 1.87, 3.0, 3.9, 4.3].into_iter().flat_map(|v| [(0, v), (1, v)]) {
            let now = slide(which, speed, |t, dist| Act::Dash { phase: t * 13.0, pace: 1.0, dist, speed });
            let then = slide(which, speed, |t, _| Act::Run { phase: t * 13.0, pace: 1.0 });
            let g = gait(which, speed);
            let air = stride_of(g.clip, g.gain).map(|s| s.flight(g.stride) / g.stride).filter(|_| g.flies).unwrap_or(0.0);
            eprintln!(
                "boy {which} at {speed} m/s: {} reaching {:.2}, {:.2} m a stride, {:.1} strides a second, {:.0}% of it in the air;                  the planted foot slides {:.0}% of the ground covered (by the old rate {:.0}%)",
                g.clip,
                g.gain,
                g.stride,
                speed / g.stride,
                air * 100.0,
                now * 100.0,
                then * 100.0
            );
            worst = worst.max(now.abs());
            if speed >= 3.0 {
                assert!(speed / g.stride < 3.0, "boy {which} at {speed} m/s turns over {:.1} strides a second", speed / g.stride);
            }
        }
        assert!(worst < 0.1, "the planted foot slides {worst:.2} of the ground covered");
    }

    /// The camera frames his face at `story::marks::OLD_MAN_FACE`: it is where his clips put it as he
    /// stands talking (within a hand's breadth).
    #[test]
    fn the_old_mans_face_is_where_the_camera_looks_for_it() {
        let c = CastRig::OldMan;
        let face = crate::render::anim::cast_rigs::grid_point(c, [29.5, 76.0, 31.0]);
        let mark = super::super::story::marks::OLD_MAN_FACE - super::super::story::marks::OLD_MAN_AT;
        let mut worst = 0.0f32;
        old_man_through_the_gateway(|beat, t, rig, mats| {
            if SCENE_ONE[beat].shot != crate::logic::story::Shot::OldMan {
                return;
            }
            let head = rig.index("head").unwrap();
            let p = mats[head].transform_point3(face - rig.parts[head].rest);
            let (dy, dr) = ((p.y - mark.y).abs(), (Vec2::new(p.x, p.z).length() - Vec2::new(mark.x, mark.z).length()).abs());
            worst = worst.max(dy.max(dr));
            assert!(dy < 0.08 && dr < 0.12, "beat {beat} at {t:.1}: his face at {p:.2}, the camera's mark {mark:.2}");
        });
        eprintln!("the old man's face is at most {worst:.3} from the camera's mark");
    }
}
