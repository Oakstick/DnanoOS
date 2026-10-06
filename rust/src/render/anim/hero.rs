//! The hero's animator: which clips play, where in them, and how they blend. It reads only what the
//! view is told of him each frame (`Drive`) and nothing reads it back, so the simulation is the
//! same with clips or without.
//!
//! His body is in one of three states: on his feet (idle and run blended by his pace, the run's
//! rate matched to his speed over the ground), in a strike (the clip warped onto the move's frame
//! data) or in a leap (the clip run by the leap's progress, then its landing). A change of state
//! crossfades from the pose he was in over `BLEND_FRAMES`, except into a strike, whose wind-up
//! must show from its first frame. On his feet, the guard and a blow's flinch are laid over his
//! upper body, his legs walking on beneath (no blow roots him, COMBAT.md §1.2). The juice is added
//! on top: the blade's overshoot after a strike, the shield's jolt.

use glam::Quat;

use super::clip::{blend, Clip, Locals, FPS};
use super::library::HeroClips;
use super::skeleton::{Bone, BONE_COUNT};
use super::warp::warp;

/// Frames a crossfade between two states takes (none into a strike).
pub const BLEND_FRAMES: f32 = 8.0;
/// Frames the crossfade into a leap takes: the leap is short, and its spring starts at once.
pub const JUMP_BLEND_FRAMES: f32 = 4.0;
/// Frames the guard and the flinch take to come over him, and to leave him.
const LAYER_IN_FRAMES: f32 = 3.0;
const LAYER_OUT_FRAMES: f32 = 8.0;
/// Frames the guard takes to come up (`combat::fsm::GUARD_RAISE`): the block clip's raise fills them.
const GUARD_RAISE_FRAMES: f32 = crate::logic::combat::fsm::GUARD_RAISE as f32;
/// The blade's overshoot past the strike's end, at its most (radians about his upright).
const OVERSHOOT: f32 = 0.14;
/// Metres a run cycle carries him when its markers do not say.
const DEFAULT_STRIDE: f32 = 2.0;
/// His speed over the ground (tiles a second) where the walk gives way to the run: all walk below
/// the first, all run above the second. (Under his guard he moves at about 2.9, a walk.)
const WALK_TO_RUN: [f32; 2] = [2.4, 4.0];

/// A strike under way.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Strike {
    /// Seconds into the move (its fighter's frame where frame data rules it).
    pub t: f32,
    /// Its phases' ends in seconds: 0, the startup's end, the active frames' end, the recovery's end.
    pub bounds: [f32; 4],
    /// The view's swing side: -1 the forehand (from his right across to his left: the code's
    /// swing coils that way first), +1 the backhand.
    pub side: f32,
    /// Counts his swings, so a new one on the same side starts afresh.
    pub serial: u32,
    /// The string's finisher (L3): its own, heavier clip where he has one.
    pub heavy: bool,
}

/// What the hero is doing this frame, as the view reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Drive {
    pub dt: f32,
    /// His speed over the ground (tiles, a metre each, a second) and how much of a stride he is in (0 to 1).
    pub speed: f32,
    pub motion: f32,
    /// How far through a leap (0 to 1), while he is in the air.
    pub airborne: Option<f32>,
    /// The leap is his evade in a fight (the roll, where he has one, rather than the jump).
    pub evade: bool,
    /// Seconds since his guard turned a blow (a parry), while its snap and recoil play.
    pub parry: Option<f32>,
    pub strike: Option<Strike>,
    /// His shield is up.
    pub blocking: bool,
    /// Reeling from a blow that struck him: how far through (0 to 1).
    pub reel: Option<f32>,
    /// The springs of the juice are on (the blade's overshoot), and the shield's tilt back from a
    /// blow it took (radians).
    pub springs: bool,
    pub shield_tilt: f32,
    /// He holds his weapon in both fists (`hold::two_hands`), and carries his shield slung on his back
    /// but to guard (`hold::sling`).
    pub two_hands: bool,
    pub slung: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Body {
    Feet,
    Strike(u32),
    Leap,
}

/// The bones the guard and the flinch take over: all but his hips and legs.
const UPPER: [bool; BONE_COUNT] = {
    let mut m = [true; BONE_COUNT];
    let legs = [Bone::Hips, Bone::ThighL, Bone::ShinL, Bone::FootL, Bone::ThighR, Bone::ShinR, Bone::FootR];
    let mut k = 0;
    while k < legs.len() {
        m[legs[k] as usize] = false;
        k += 1;
    }
    m
};

/// The animator's memory from frame to frame.
#[derive(Clone, Debug)]
pub struct Animator {
    body: Body,
    /// The pose being left, frozen, and how many frames of the crossfade from it have run (of how many).
    fade: Option<(Locals, f32, f32)>,
    last: Option<Locals>,
    idle_t: f32,
    /// How far through its cycle the run is (0 to 1).
    run_cycle: f32,
    /// Seconds since he landed, while the leap's landing plays out.
    landing: Option<f32>,
    was_airborne: bool,
    /// The leap under way (or landing) is the evade's roll.
    rolling: bool,
    guard_t: f32,
    guard_w: f32,
    hit_t: f32,
    hit_w: f32,
}

impl Default for Animator {
    fn default() -> Self {
        Self {
            body: Body::Feet,
            fade: None,
            last: None,
            idle_t: 0.0,
            run_cycle: 0.0,
            landing: None,
            was_airborne: false,
            rolling: false,
            guard_t: 0.0,
            guard_w: 0.0,
            hit_t: 0.0,
            hit_w: 0.0,
        }
    }
}

fn smooth(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/// `over` laid on `base` by `w`, on the bones `mask` names.
fn layer(base: &Locals, over: &Locals, w: f32, mask: &[bool; BONE_COUNT]) -> Locals {
    if w <= 0.0 {
        return *base;
    }
    std::array::from_fn(|i| if mask[i] { base[i].lerp(&over[i], w.min(1.0)) } else { base[i] })
}

/// A clip's markers `names` in seconds, or (where it has none) these shares of its length.
fn marks(clip: &Clip, names: &[&str], shares: &[f32]) -> Vec<f32> {
    names.iter().zip(shares).map(|(n, s)| clip.mark(n).unwrap_or(s * clip.duration())).collect()
}

impl Animator {
    /// The pose for this frame and the juice's turns of bones (in the model's axes), or none when
    /// a clip the moment needs is missing: then the code's animation plays instead.
    pub fn step(&mut self, clips: &HeroClips, d: &Drive) -> Option<(Locals, [Quat; BONE_COUNT])> {
        let (idle, run) = (clips.get("idle")?, clips.get("run")?);
        let dt = d.dt.max(0.0);

        // On his feet: idle and run, the run's rate matched to his speed (a stride a cycle).
        // (With a walk, the walk and the run share one cycle, its rate from the stride between theirs.)
        self.idle_t += dt;
        let walk = clips.get("walk");
        let run_stride = run.markers.stride.unwrap_or(DEFAULT_STRIDE).max(0.1);
        let running = if walk.is_some() { smooth((d.speed - WALK_TO_RUN[0]) / (WALK_TO_RUN[1] - WALK_TO_RUN[0])) } else { 1.0 };
        let stride =
            walk.map(|w| w.markers.stride.unwrap_or(run_stride).max(0.1)).unwrap_or(run_stride) * (1.0 - running) + run_stride * running;
        self.run_cycle = (self.run_cycle + dt * d.speed.max(0.0) / stride).fract();
        let pace = smooth(d.motion);
        let mut gait = run.sample(self.run_cycle * run.duration());
        if let (Some(w), true) = (walk, running < 1.0) {
            gait = blend(&w.sample(self.run_cycle * w.duration()), &gait, running);
        }
        let mut feet = blend(&idle.sample(self.idle_t), &gait, pace);

        // The guard over his upper body: raised in the guard's frames, then held (the clip's hold loops).
        if d.blocking {
            self.guard_t += dt;
            self.guard_w = (self.guard_w + dt * FPS / LAYER_IN_FRAMES).min(1.0);
        } else {
            self.guard_w = (self.guard_w - dt * FPS / LAYER_OUT_FRAMES).max(0.0);
            if self.guard_w == 0.0 {
                self.guard_t = 0.0;
            }
        }
        if self.guard_w > 0.0 {
            let block = clips.get("block")?;
            let raised = block.mark("raised").unwrap_or(0.1 * block.duration());
            let rise = GUARD_RAISE_FRAMES / FPS;
            // (Held, his own hold where he has one; else the raise's tail, looping.)
            let pose = match (self.guard_t <= rise, clips.get("block_idle")) {
                (true, _) => block.sample(warp(self.guard_t, &[0.0, rise], &[0.0, raised])),
                (false, Some(hold)) => hold.sample(self.guard_t - rise),
                (false, None) => block.sample(raised + (self.guard_t - rise).rem_euclid((block.duration() - raised).max(1e-3))),
            };
            feet = layer(&feet, &pose, smooth(self.guard_w), &UPPER);
        }
        // A blow's flinch over his upper body, through its reel.
        if let Some(p) = d.reel {
            self.hit_t = p;
            self.hit_w = (self.hit_w + dt * FPS / LAYER_IN_FRAMES).min(1.0);
        } else {
            self.hit_w = (self.hit_w - dt * FPS / LAYER_OUT_FRAMES).max(0.0);
        }
        if self.hit_w > 0.0 {
            if let Some(hit) = clips.get("hit") {
                feet = layer(&feet, &hit.sample(self.hit_t * hit.duration()), smooth(self.hit_w), &UPPER);
            }
        }

        // A parry: the shield snapped up and knocked back, over his upper body.
        let mut parried = 0.0;
        if let (Some(t), Some(parry)) = (d.parry, clips.get("parry")) {
            if t <= parry.duration() {
                parried = smooth((parry.duration() - t) / (LAYER_OUT_FRAMES / FPS)).min(1.0);
                feet = layer(&feet, &parry.sample(t), parried, &UPPER);
            }
        }

        // The leap (the roll, for the evade in a fight), and its landing.
        if self.was_airborne && d.airborne.is_none() {
            self.landing = Some(0.0);
        }
        if d.airborne.is_some() && !self.was_airborne {
            self.rolling = d.evade && clips.get("roll").is_some();
        }
        self.was_airborne = d.airborne.is_some();
        if let Some(l) = self.landing.as_mut() {
            *l += dt;
        }
        let jump = clips.get(if self.rolling { "roll" } else { "jump" });
        if d.airborne.is_some() || self.landing.is_some() {
            jump?;
        }
        let jump_marks = jump.map(|j| marks(j, &["takeoff", "land", "recover_end"], &[0.0, 0.7, 1.0]));
        // The landing plays on until it settles, or until he strikes or runs off.
        if let (Some(l), Some(m)) = (self.landing, &jump_marks) {
            if l > m[2] - m[1] || d.strike.is_some() || d.blocking || d.motion > 0.6 {
                self.landing = None;
            }
        }

        let (body, pose) = if let (Some(p), Some(j), Some(m)) = (d.airborne, jump, &jump_marks) {
            (Body::Leap, j.sample(m[0] + (m[1] - m[0]) * p.clamp(0.0, 1.0)))
        } else if let Some(s) = d.strike {
            let name = if s.heavy && clips.get("slash_heavy").is_some() {
                "slash_heavy"
            } else if s.side < 0.0 {
                "slash_fore"
            } else {
                "slash_back"
            };
            let clip = clips.get(name)?;
            let m = marks(clip, &["windup_end", "active_end", "recover_end"], &[0.3, 0.45, 1.0]);
            let t = warp(s.t, &s.bounds, &[0.0, m[0], m[1], m[2]]);
            (Body::Strike(s.serial), clip.sample(t))
        } else if let (Some(l), Some(j), Some(m)) = (self.landing, jump, &jump_marks) {
            // (The landing's settle, his legs taking it, his upper body as his feet have it.)
            let settle = j.sample(m[1] + l);
            (Body::Leap, settle)
        } else {
            (Body::Feet, feet)
        };

        // A change of state crossfades from where he was, except into a strike.
        if body != self.body {
            self.fade = match (body, self.last) {
                (Body::Strike(_), _) | (_, None) => None,
                (Body::Leap, Some(last)) => Some((last, 0.0, JUMP_BLEND_FRAMES)),
                (_, Some(last)) => Some((last, 0.0, BLEND_FRAMES)),
            };
            self.body = body;
        }
        let mut out = pose;
        if let Some((from, f, len)) = self.fade.as_mut() {
            *f += dt * FPS;
            out = blend(from, &pose, smooth(*f / *len));
            if *f >= *len {
                self.fade = None;
            }
        }
        self.last = Some(out);

        // What he carries, held on the finished pose: both fists on the grip, the shield slung.
        if d.two_hands {
            super::hold::two_hands(&clips.skeleton, &mut out);
        }
        if d.slung {
            super::hold::sling(&clips.skeleton, &mut out, smooth(self.guard_w).max(parried));
        }

        // The juice: the blade carried on past the strike's end and back, the shield knocked back.
        let mut extra = [Quat::IDENTITY; BONE_COUNT];
        if let (Body::Strike(_), Some(s), true) = (body, d.strike, d.springs) {
            let rec = (s.bounds[3] - s.bounds[2]).max(1e-3);
            let u = (s.t - s.bounds[2]) / rec;
            if u > 0.0 {
                let k = (std::f32::consts::PI * (u / 0.4).min(1.0)).sin();
                // (The forehand carries on round to his left, about his upright; the backhand to his
                // right. In both fists, his chest carries the two arms round together.)
                let carrier = if d.two_hands { Bone::Chest } else { Bone::UpperArmR };
                extra[carrier.index()] = Quat::from_rotation_y(-OVERSHOOT * k * s.side.signum());
            }
        }
        if d.shield_tilt != 0.0 && self.guard_w > 0.0 {
            // (A slung shield before him is knocked back about its own middle; one strapped on, with his forearm.)
            let tilted = if d.slung { Bone::Shield } else { Bone::ForearmL };
            extra[tilted.index()] = Quat::from_rotation_x(-d.shield_tilt * smooth(self.guard_w));
        }
        Some((out, extra))
    }

    /// Starts afresh (another hero, or his clips changed).
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::anim::clip::{Markers, Skeleton, Trs};
    use glam::Vec3;

    /// A clip whose spine turns `angle` about x through it, from 0 at its start.
    fn ramp(name: &str, frames: usize, angle: f32, marks: &[(&str, f32)], looping: bool) -> Clip {
        let rest: Locals = [Trs::IDENTITY; BONE_COUNT];
        let sk = Skeleton::new(rest, glam::Mat4::IDENTITY);
        let poses: Vec<Locals> = (0..frames)
            .map(|f| {
                let mut p = rest;
                p[Bone::Spine.index()].r = Quat::from_rotation_x(angle * f as f32 / (frames - 1) as f32);
                p[Bone::ThighL.index()].t = Vec3::new(0.0, angle, 0.0);
                p
            })
            .collect();
        let markers = Markers { marks: marks.iter().map(|(n, f)| (n.to_string(), *f)).collect(), looping, stride: Some(2.0) };
        Clip::from_frames(name, sk, &poses, markers)
    }

    fn clips() -> HeroClips {
        let all = vec![
            ramp("idle", 61, 0.1, &[], true),
            ramp("run", 31, 0.2, &[], true),
            ramp("slash_fore", 41, 1.0, &[("windup_end", 12.0), ("active_end", 18.0), ("recover_end", 40.0)], false),
            ramp("slash_back", 41, -1.0, &[("windup_end", 12.0), ("active_end", 18.0), ("recover_end", 40.0)], false),
            ramp("block", 31, 0.5, &[("raised", 6.0)], false),
            ramp("jump", 41, 0.8, &[("takeoff", 0.0), ("land", 30.0), ("recover_end", 40.0)], false),
        ];
        HeroClips { skeleton: all[0].skeleton.clone(), clips: all, load_ms: 0.0, errors: Vec::new() }
    }

    fn spine(p: &Locals) -> f32 {
        p[Bone::Spine.index()].r.to_euler(glam::EulerRot::XYZ).0
    }

    #[test]
    fn a_strike_snaps_in_and_lands_on_its_frames() {
        let c = clips();
        let mut a = Animator::default();
        let d = Drive { dt: 1.0 / 60.0, ..Default::default() };
        for _ in 0..30 {
            a.step(&c, &d).unwrap();
        }
        // L1's frames (6, 4, 14) at the move's first frame: the clip's first frame, no blend from the idle.
        let bounds = [0.0, 6.0 / 60.0, 10.0 / 60.0, 24.0 / 60.0];
        let strike = |t: f32| Drive { strike: Some(Strike { t, bounds, side: -1.0, serial: 1, heavy: false }), ..d };
        let (p, _) = a.step(&c, &strike(0.0)).unwrap();
        assert!(spine(&p).abs() < 1e-4, "{}", spine(&p));
        // The startup's end shows the clip's wind-up end (frame 12 of 40: 0.3 of the turn).
        let (p, _) = a.step(&c, &strike(bounds[1])).unwrap();
        assert!((spine(&p) - 0.3).abs() < 1e-3, "{}", spine(&p));
        let (p, _) = a.step(&c, &strike(bounds[2])).unwrap();
        assert!((spine(&p) - 0.45).abs() < 1e-3, "{}", spine(&p));
        // The backhand plays its own clip.
        let (p, _) = a.step(&c, &Drive { strike: Some(Strike { t: bounds[1], bounds, side: 1.0, serial: 2, heavy: false }), ..d }).unwrap();
        assert!((spine(&p) + 0.3).abs() < 1e-3, "{}", spine(&p));
    }

    #[test]
    fn leaving_a_strike_crossfades_back_to_his_feet() {
        let c = clips();
        let mut a = Animator::default();
        let d = Drive { dt: 1.0 / 60.0, ..Default::default() };
        let bounds = [0.0, 0.1, 0.2, 0.4];
        a.step(&c, &Drive { strike: Some(Strike { t: 0.4, bounds, side: -1.0, serial: 1, heavy: false }), ..d }).unwrap();
        // The frame after, the pose is still near the strike's end (1.0) and comes back over the blend.
        let (p, _) = a.step(&c, &d).unwrap();
        assert!(spine(&p) > 0.9, "{}", spine(&p));
        for _ in 0..BLEND_FRAMES as usize {
            a.step(&c, &d).unwrap();
        }
        let (p, _) = a.step(&c, &d).unwrap();
        assert!(spine(&p) < 0.11, "{}", spine(&p));
    }

    #[test]
    fn the_guard_rises_in_its_frames_and_leaves_his_legs_alone() {
        let c = clips();
        let mut a = Animator::default();
        let d = Drive { dt: 1.0 / 60.0, blocking: true, ..Default::default() };
        for _ in 0..6 {
            a.step(&c, &d).unwrap();
        }
        let (p, _) = a.step(&c, &d).unwrap();
        // Raised (past frame 6 of the block's 30: 0.1 of its 0.5 at least), the legs as the idle has them.
        assert!(spine(&p) >= 0.1, "{}", spine(&p));
        assert!((p[Bone::ThighL.index()].t.y - 0.1).abs() < 1e-4);
    }

    #[test]
    fn the_run_keeps_pace_with_his_feet() {
        // At 4 tiles a second and a stride of 2, a cycle takes half a second.
        let c = clips();
        let mut a = Animator::default();
        let d = Drive { dt: 1.0 / 60.0, speed: 4.0, motion: 1.0, ..Default::default() };
        for _ in 0..15 {
            a.step(&c, &d).unwrap();
        }
        assert!((a.run_cycle - 0.5).abs() < 1e-3, "{}", a.run_cycle);
    }

    #[test]
    fn the_finisher_the_roll_and_the_parry_have_their_own_clips() {
        let mut c = clips();
        c.clips.push(ramp("slash_heavy", 41, 2.0, &[("windup_end", 12.0), ("active_end", 18.0), ("recover_end", 40.0)], false));
        c.clips.push(ramp("roll", 41, -0.8, &[("takeoff", 0.0), ("land", 30.0), ("recover_end", 40.0)], false));
        c.clips.push(ramp("parry", 21, 0.9, &[("raised", 3.0)], false));
        let mut a = Animator::default();
        let d = Drive { dt: 1.0 / 60.0, ..Default::default() };
        let bounds = [0.0, 0.1, 0.2, 0.4];
        let (p, _) = a.step(&c, &Drive { strike: Some(Strike { t: 0.1, bounds, side: -1.0, serial: 1, heavy: true }), ..d }).unwrap();
        assert!((spine(&p) - 0.6).abs() < 1e-3, "the finisher's clip: {}", spine(&p));
        // The evade leaps by the roll, a leap elsewhere by the jump (both to their land at the leap's end).
        let mut a = Animator::default();
        let (p, _) = a.step(&c, &Drive { airborne: Some(1.0), evade: true, ..d }).unwrap();
        assert!(spine(&p) < 0.0, "rolled: {}", spine(&p));
        let mut a = Animator::default();
        let (p, _) = a.step(&c, &Drive { airborne: Some(1.0), ..d }).unwrap();
        assert!(spine(&p) > 0.0, "jumped: {}", spine(&p));
        // The parry's snap over his upper body, then gone when it has played.
        let mut a = Animator::default();
        let (p, _) = a.step(&c, &Drive { parry: Some(0.1), ..d }).unwrap();
        assert!((spine(&p) - 0.27).abs() < 1e-3, "parried: {}", spine(&p));
        let (p, _) = a.step(&c, &Drive { parry: Some(1.0), ..d }).unwrap();
        assert!(spine(&p) < 0.1, "{}", spine(&p));
    }

    #[test]
    fn a_walk_gives_way_to_the_run_with_his_speed() {
        let mut c = clips();
        c.clips.push(ramp("walk", 31, -0.5, &[], true));
        let mut a = Animator::default();
        let thigh = |p: &Locals| p[Bone::ThighL.index()].t.y;
        let slow = Drive { dt: 1.0 / 60.0, speed: 1.0, motion: 1.0, ..Default::default() };
        let (p, _) = a.step(&c, &slow).unwrap();
        assert!((thigh(&p) + 0.5).abs() < 1e-4, "walking: {}", thigh(&p));
        let (p, _) = a.step(&c, &Drive { speed: 5.0, ..slow }).unwrap();
        assert!((thigh(&p) - 0.2).abs() < 1e-4, "running: {}", thigh(&p));
    }

    #[test]
    fn a_missing_clip_hands_back_to_the_code() {
        let mut c = clips();
        c.clips.retain(|k| k.name != "slash_back");
        let mut a = Animator::default();
        let d = Drive { dt: 1.0 / 60.0, ..Default::default() };
        assert!(a.step(&c, &d).is_some());
        let back = Drive { strike: Some(Strike { t: 0.0, bounds: [0.0, 0.1, 0.2, 0.4], side: 1.0, serial: 1, heavy: false }), ..d };
        assert!(a.step(&c, &back).is_none());
        c.clips.retain(|k| k.name != "idle");
        assert!(a.step(&c, &d).is_none());
    }
}
