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
    /// What hangs and swings after the body (a scarf's end, the beard), each slot's two.
    swing: [[Option<Quat>; 2]; SLOTS],
}

/// The turn about the upright that `q` makes of the way forward: its heading alone.
pub(super) fn heading(q: Quat) -> Quat {
    let f = q * Vec3::Z;
    Quat::from_rotation_y(f.x.atan2(f.z))
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
        let hung: &[(&str, f32)] = match c {
            CastRig::Kid(_) => &[("scarf", 0.35)],
            CastRig::OldMan => &[("beard.1", 0.6), ("beard.2", 0.35)],
            _ => &[],
        };
        for (k, &(bone, follow)) in hung.iter().enumerate() {
            let (Some(b), Some(p)) = (def.index(bone), def.index(bone).and_then(|b| def.parents[b])) else { continue };
            let parent = pose.rot[p];
            let want = heading(parent).slerp(parent, follow);
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
            Act::Yell(_) => -0.2,
            _ => 0.0,
        };
        let steady = if matches!(act, Act::Talk(_) | Act::Stand | Act::Scared(_) | Act::Nod(..)) { 0.6 } else { 0.0 };
        let Some((rig, mats)) =
            self.cast_mats(CastRig::Kid(which.min(3) as u8), &play, time, |pose, def| look_steady(pose, def, look_yaw, pitch, steady))
        else {
            return false;
        };
        if alpha < 0.99 {
            f.rigs.push_matrices(rig, &mats, root, Vec3::ONE, alpha, Vec3::ZERO, rig_look::FADED, 0.0);
        } else {
            f.rigs.push_matrices(rig, &mats, root, Vec3::ONE, 1.0, glow, rig_look::PLAIN, 0.0);
        }
        true
    }
}

/// The idle each boy stands in, so the four never breathe in step.
const IDLES: [&str; 4] = ["idle_happy", "idle", "idle_happy_2", "look"];

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
        Act::Yell(t) => Play::base(Layer::of("yell", 1.1 + t * 0.8), 9),
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

/// The old man's moments, as the gateway's staging sets them out.
#[derive(Clone, Copy, Debug)]
pub(super) enum OldAct {
    /// On his stool, dozing (1 asleep, his chin on his chest) or awake.
    Sit { doze: f32 },
    /// Seated, speaking (seconds into the line; which of his lines).
    Talk { t: f32, line: usize },
    /// Seated, his free hand held out (to the dog, to the door), 0 to 1.
    Hand { t: f32, raise: f32 },
    /// Looking away, nodding in his guilt.
    Nod { t: f32 },
    /// Rising from the stool (seconds since he began).
    Rise { t: f32 },
    /// On his feet, stepping aside (`pace` 0 standing, 1 walking), a hand raised (`wave`).
    Stand { pace: f32, wave: f32 },
}

pub(super) fn old_man_play(a: OldAct, time: f32) -> Play {
    match a {
        OldAct::Sit { doze } => Play::base(Layer::of("sit_idle", time * (1.0 - 0.6 * doze)), 1).fading(0.4),
        OldAct::Talk { t, line } => {
            Play::base(Layer::of(if line % 2 == 0 { "sit_talk_2" } else { "sit_talk" }, t + 0.5), 2 + (line % 2) as u32).fading(0.4)
        }
        OldAct::Hand { t, raise } => {
            Play::base(Layer::of("sit_talk_2", t), 4).with(Layer::of("sit_point", 1.2).on(Mask::Upper).weight(raise)).fading(0.4)
        }
        OldAct::Nod { t } => Play::base(Layer::of("nod", t), 5).fading(0.4),
        OldAct::Rise { t } => Play::base(Layer::of("sit_to_stand", t), 6).fading(0.3),
        OldAct::Stand { pace, wave } => {
            let mut p = Play::base(Layer::of("idle", time), 7);
            if pace > 0.0 {
                p = p.with(Layer::of("walk", time).weight(pace));
            }
            p.with(Layer::of("wave", time).on(Mask::Under(&["upper_arm.L"])).weight(wave)).fading(0.3)
        }
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

/// Where a point of cast member `c`'s grid is, on bone `bone` posed by `mats`.
pub(super) fn bone_point(c: CastRig, rig: &Rig, mats: &[Mat4], bone: &str, g: [f32; 3]) -> Option<Vec3> {
    let b = rig.index(bone)?;
    let p = crate::render::anim::cast_rigs::grid_point(c, g);
    Some(mats.get(b)?.transform_point3(p - rig.parts[b].rest))
}
