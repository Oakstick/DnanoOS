//! The foes' animator (ANIMATION.md §11): which of a foe's clips play, where in them, and how they
//! blend, read from what the view is told of it each frame (`FoeDrive`: its pace, its move and the
//! move's frames, its reel). Presentation only: nothing in the simulation reads it, so a fight is the
//! same with clips or without.
//!
//! As the heroes' (`anim::hero`): on its feet the idle and the gait (walk, giving way to a run where
//! it has one) blended by its pace, the gait's rate matched to its speed over the ground (a stride a
//! cycle); a move's clip warped onto the move's frame data (its wind-up ends exactly on the startup's
//! last frame, its blow spans the active frames, its follow-through the recovery); a blow's flinch
//! laid over its upper body (or the whole of a small body), a stagger or a deflection played whole
//! through the reel's frames. A change of state crossfades over `BLEND_FRAMES`, but into a strike,
//! whose wind-up must show from its first frame. A moment whose clip it lacks hands back to the code.

use super::clip::{blend_into, Clip, Trs, FPS};
use super::library::ClipSet;
use super::skeleton::SkeletonDef;
use super::warp::warp;

/// Frames a crossfade between two states takes (none into a strike).
pub const BLEND_FRAMES: f32 = 8.0;
/// Frames the flinch takes to come over it, and to leave it.
const LAYER_IN_FRAMES: f32 = 3.0;
const LAYER_OUT_FRAMES: f32 = 8.0;
/// Seconds its pace takes to go from standing to its gait (and back).
const PACE_SECONDS: f32 = 0.18;
/// Speed over the ground (tiles a second) where a walk gives way to a run: all walk below the
/// first, all run above the second.
pub const WALK_TO_RUN: [f32; 2] = [2.2, 3.4];
/// Metres a gait's cycle carries it when its markers do not say.
const DEFAULT_STRIDE: f32 = 1.2;
/// The fastest a gait plays against its own pace (its clip's length a cycle): the foes cover the
/// ground faster than their legs' length would let them (a draugr's 1.8 tiles a second is ten of his
/// strides), so past this their feet slide rather than their legs spin.
const MAX_GAIT_RATE: f32 = 1.8;

/// A move under way.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoeStrike {
    /// Its clip: its own (by the move's name), or failing that its kind's (a slam, a bite).
    pub clips: [&'static str; 2],
    /// Seconds into the move (its fighter's frame).
    pub t: f32,
    /// Its phases' ends in seconds: 0, the startup's end, the active frames' end, the recovery's end.
    pub bounds: [f32; 4],
    /// Counts its moves, so a move begun again at once starts afresh.
    pub serial: u32,
}

/// A reel, as it shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoeReel {
    /// A blow taken (hitstun, blockstun): a flinch over the upper body.
    Hit,
    /// Its poise broken, or its guard: it staggers, whole.
    Stagger,
    /// Its blow parried: a short recoil, whole.
    Deflected,
}

/// What the foe is doing this frame, as the view reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FoeDrive {
    pub dt: f32,
    /// Its speed over the ground (tiles a second), and whether it is on the move.
    pub speed: f32,
    pub moving: bool,
    pub strike: Option<FoeStrike>,
    /// Reeling: how, and how far through (0 to 1).
    pub reel: Option<(FoeReel, f32)>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Body {
    Feet,
    Strike(u32),
    Reel(FoeReel),
}

/// The animator's memory from frame to frame, and its buffers.
#[derive(Clone, Debug)]
pub struct FoeAnimator {
    body: Body,
    /// The pose being left, frozen, how many frames of the crossfade from it have run (of how many).
    fade: Option<(Vec<Trs>, f32, f32)>,
    last: Vec<Trs>,
    idle_t: f32,
    /// How far through its cycle the gait is (0 to 1).
    cycle: f32,
    /// How much of its gait shows (0 standing, 1 on the move).
    pace: f32,
    hit_t: f32,
    hit_w: f32,
    scratch: Vec<Trs>,
    over: Vec<Trs>,
}

impl Default for FoeAnimator {
    fn default() -> Self {
        Self {
            body: Body::Feet,
            fade: None,
            last: Vec::new(),
            idle_t: 0.0,
            cycle: 0.0,
            pace: 0.0,
            hit_t: 0.0,
            hit_w: 0.0,
            scratch: Vec::new(),
            over: Vec::new(),
        }
    }
}

fn smooth(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/// The first of `names` the set has.
fn first<'a>(clips: &'a ClipSet, names: &[&str]) -> Option<&'a Clip> {
    names.iter().find_map(|n| clips.get(n))
}

/// The names a foe's clips go by: its stand, its gait, its run.
pub const IDLE: [&str; 2] = ["idle", "hover"];
pub const GAIT: [&str; 3] = ["walk", "fly", "slither"];

impl FoeAnimator {
    /// Starts it at phase `phase` (seconds into its idle and its gait), so a pack does not move in step.
    pub fn new(phase: f32) -> Self {
        Self { idle_t: phase, cycle: (phase * 0.37).fract(), ..Self::default() }
    }

    /**
     * The pose for this frame into `out` (a bone each of `def`), or false when a clip the moment
     * needs is missing: then the code's animation plays instead (and the animator starts afresh
     * when the clips take over again).
     */
    pub fn step(&mut self, clips: &ClipSet, def: &SkeletonDef, d: &FoeDrive, out: &mut Vec<Trs>) -> bool {
        let n = def.len();
        let Some(idle) = first(clips, &IDLE) else { return false };
        let dt = d.dt.max(0.0);
        out.clear();
        out.resize(n, Trs::IDENTITY);
        self.scratch.resize(n, Trs::IDENTITY);
        self.over.resize(n, Trs::IDENTITY);

        // On its feet: the idle and the gait blended by its pace, the gait's rate matched to its speed.
        self.idle_t += dt;
        let target = if d.moving { 1.0 } else { 0.0 };
        let step = dt / PACE_SECONDS;
        self.pace = if self.pace < target { (self.pace + step).min(target) } else { (self.pace - step).max(target) };
        idle.sample_into(self.idle_t, out);
        if let Some(gait) = first(clips, &GAIT) {
            let run = clips.get("run");
            let running = if run.is_some() { smooth((d.speed - WALK_TO_RUN[0]) / (WALK_TO_RUN[1] - WALK_TO_RUN[0])) } else { 0.0 };
            let s_gait = gait.markers.stride.unwrap_or(DEFAULT_STRIDE).max(0.05);
            let s_run = run.and_then(|r| r.markers.stride).unwrap_or(s_gait).max(0.05);
            let stride = s_gait * (1.0 - running) + s_run * running;
            let natural = gait.duration() * (1.0 - running) + run.map(|r| r.duration()).unwrap_or(gait.duration()) * running;
            let rate = (d.speed.max(0.0) / stride).min(MAX_GAIT_RATE / natural.max(0.05));
            // (Standing, its gait idles on at the pace it last had: it never snaps to a new phase.)
            self.cycle = (self.cycle + dt * rate * smooth(self.pace)).fract();
            if self.pace > 0.0 {
                gait.sample_into(self.cycle * gait.duration(), &mut self.scratch);
                if let (Some(r), true) = (run, running > 0.0) {
                    r.sample_into(self.cycle * r.duration(), &mut self.over);
                    blend_into(&mut self.scratch, &self.over, running);
                }
                blend_into(out, &self.scratch, smooth(self.pace));
            }
        }

        // A blow's flinch over its upper body (all of a small one), through its reel.
        let flinch = matches!(d.reel, Some((FoeReel::Hit, _)));
        if let (true, Some((_, p))) = (flinch, d.reel) {
            self.hit_t = p;
            self.hit_w = (self.hit_w + dt * FPS / LAYER_IN_FRAMES).min(1.0);
        } else {
            self.hit_w = (self.hit_w - dt * FPS / LAYER_OUT_FRAMES).max(0.0);
        }
        if self.hit_w > 0.0 {
            let Some(hit) = clips.get("hit") else { return false };
            hit.sample_into(self.hit_t * hit.duration(), &mut self.over);
            let w = smooth(self.hit_w);
            for (i, (o, h)) in out.iter_mut().zip(&self.over).enumerate() {
                if !def.legs.contains(&i) {
                    *o = o.lerp(h, w);
                }
            }
        }

        // The state: a strike, a reel played whole, or its feet.
        let body = if let Some(s) = d.strike {
            let Some(clip) = s.clips.iter().find_map(|n| clips.get(n)) else { return false };
            let m: Vec<f32> = ["windup_end", "active_end", "recover_end"]
                .iter()
                .zip([0.3, 0.45, 1.0])
                .map(|(name, share)| clip.mark(name).unwrap_or(share * clip.duration()))
                .collect();
            let t = warp(s.t, &s.bounds, &[0.0, m[0], m[1], m[2]]);
            clip.sample_into(t, out);
            Body::Strike(s.serial)
        } else if let Some((kind @ (FoeReel::Stagger | FoeReel::Deflected), p)) = d.reel {
            let names: &[&str] = if kind == FoeReel::Stagger { &["stagger", "hit"] } else { &["deflected", "hit"] };
            let Some(clip) = first(clips, names) else { return false };
            clip.sample_into(p.clamp(0.0, 1.0) * clip.duration(), out);
            Body::Reel(kind)
        } else {
            Body::Feet
        };

        // A change of state crossfades from where it was, except into a strike from its start (a
        // strike taken up past its wind-up, as a charge's recovery after its run, fades in).
        if body != self.body {
            self.fade = match body {
                Body::Strike(_) if !d.strike.is_some_and(|s| s.t > s.bounds[1]) => None,
                _ if self.last.len() == n => Some((self.last.clone(), 0.0, BLEND_FRAMES)),
                _ => None,
            };
            self.body = body;
        }
        if let Some((from, f, len)) = self.fade.as_mut() {
            *f += dt * FPS;
            let u = smooth(*f / *len);
            self.scratch.copy_from_slice(from);
            blend_into(&mut self.scratch, out, u);
            out.copy_from_slice(&self.scratch);
            if *f >= *len {
                self.fade = None;
            }
        }
        self.last.clear();
        self.last.extend_from_slice(out);
        true
    }

    /// Starts afresh (the code took a moment over, or its clips changed).
    pub fn reset(&mut self) {
        let phase = self.idle_t;
        *self = Self::new(phase);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::anim::clip::{Markers, Skeleton};
    use crate::render::anim::skeleton::HUMANOID;
    use glam::{Quat, Vec3};

    const SPINE: usize = 1;
    const THIGH: usize = 11;

    /// A clip whose spine turns `angle` about x through it, from 0 at its start; its thigh carries
    /// `angle` as a mark.
    fn ramp(name: &str, frames: usize, angle: f32, marks: &[(&str, f32)], looping: bool) -> Clip {
        let rest = vec![Trs::IDENTITY; HUMANOID.len()];
        let sk = Skeleton::of(&HUMANOID, rest.clone(), glam::Mat4::IDENTITY);
        let poses: Vec<Vec<Trs>> = (0..frames)
            .map(|f| {
                let mut p = rest.clone();
                p[SPINE].r = Quat::from_rotation_x(angle * f as f32 / (frames - 1) as f32);
                p[THIGH].t = Vec3::new(0.0, angle, 0.0);
                p
            })
            .collect();
        let markers = Markers { marks: marks.iter().map(|(n, f)| (n.to_string(), *f)).collect(), looping, stride: Some(1.0) };
        Clip::from_poses(name, sk, &poses, markers)
    }

    fn set(extra: &[Clip]) -> ClipSet {
        let mut all = vec![
            ramp("idle", 61, 0.1, &[], true),
            ramp("walk", 31, 0.2, &[], true),
            ramp("slam", 41, 1.0, &[("windup_end", 12.0), ("active_end", 18.0), ("recover_end", 40.0)], false),
            ramp("hit", 21, 0.5, &[], false),
        ];
        all.extend_from_slice(extra);
        ClipSet { skeleton: all[0].skeleton.clone(), clips: all, load_ms: 0.0, errors: Vec::new() }
    }

    fn spine(p: &[Trs]) -> f32 {
        p[SPINE].r.to_euler(glam::EulerRot::XYZ).0
    }

    const DT: f32 = 1.0 / 60.0;

    #[test]
    fn a_move_snaps_in_and_lands_on_its_frames() {
        let c = set(&[]);
        let mut a = FoeAnimator::default();
        let mut out = Vec::new();
        let d = FoeDrive { dt: DT, ..Default::default() };
        for _ in 0..20 {
            assert!(a.step(&c, &HUMANOID, &d, &mut out));
        }
        // A slam of 24, 6 and 30 frames: its first frame is the clip's first, no blend from the idle.
        let bounds = [0.0, 24.0 / 60.0, 30.0 / 60.0, 60.0 / 60.0];
        let strike = |t: f32| FoeDrive { strike: Some(FoeStrike { clips: ["slam", "slam"], t, bounds, serial: 1 }), ..d };
        a.step(&c, &HUMANOID, &strike(0.0), &mut out);
        assert!(spine(&out).abs() < 1e-4, "{}", spine(&out));
        // The startup's last frame shows the clip's wind-up end (frame 12 of 40: 0.3 of the turn).
        a.step(&c, &HUMANOID, &strike(bounds[1]), &mut out);
        assert!((spine(&out) - 0.3).abs() < 1e-3, "{}", spine(&out));
        a.step(&c, &HUMANOID, &strike(bounds[2]), &mut out);
        assert!((spine(&out) - 0.45).abs() < 1e-3, "{}", spine(&out));
        // After it, back to its feet over the crossfade.
        a.step(&c, &HUMANOID, &d, &mut out);
        assert!(spine(&out) > 0.4, "{}", spine(&out));
        for _ in 0..BLEND_FRAMES as usize + 1 {
            a.step(&c, &HUMANOID, &d, &mut out);
        }
        assert!(spine(&out) < 0.11, "{}", spine(&out));
    }

    #[test]
    fn the_gait_keeps_pace_with_its_feet_and_the_flinch_leaves_its_legs() {
        let c = set(&[]);
        let mut a = FoeAnimator::default();
        let mut out = Vec::new();
        // At 2 tiles a second and a stride of 1, a cycle takes half a second (once its pace is up).
        let walk = FoeDrive { dt: DT, speed: 2.0, moving: true, ..Default::default() };
        for _ in 0..30 {
            a.step(&c, &HUMANOID, &walk, &mut out);
        }
        let c0 = a.cycle;
        for _ in 0..15 {
            a.step(&c, &HUMANOID, &walk, &mut out);
        }
        assert!(((a.cycle - c0).rem_euclid(1.0) - 0.5).abs() < 1e-3, "{} then {}", c0, a.cycle);
        assert!((out[THIGH].t.y - 0.2).abs() < 1e-4, "walking: {}", out[THIGH].t.y);
        // A blow: the flinch over the upper body, the legs walking on.
        for _ in 0..4 {
            a.step(&c, &HUMANOID, &FoeDrive { reel: Some((FoeReel::Hit, 0.5)), ..walk }, &mut out);
        }
        assert!((spine(&out) - 0.25).abs() < 1e-3, "flinched: {}", spine(&out));
        assert!((out[THIGH].t.y - 0.2).abs() < 1e-4, "its legs walk on: {}", out[THIGH].t.y);
    }

    #[test]
    fn a_missing_clip_hands_back_to_the_code() {
        let c = set(&[]);
        let mut a = FoeAnimator::default();
        let mut out = Vec::new();
        let d = FoeDrive { dt: DT, ..Default::default() };
        assert!(a.step(&c, &HUMANOID, &d, &mut out));
        let sweep =
            FoeDrive { strike: Some(FoeStrike { clips: ["sweep", "sweep"], t: 0.0, bounds: [0.0, 0.3, 0.4, 0.8], serial: 2 }), ..d };
        assert!(!a.step(&c, &HUMANOID, &sweep, &mut out));
        // A stagger without its own clip plays the flinch whole; with one, its own.
        assert!(a.step(&c, &HUMANOID, &FoeDrive { reel: Some((FoeReel::Stagger, 1.0)), ..d }, &mut out));
        let c = set(&[ramp("stagger", 21, -0.7, &[], false)]);
        let mut a = FoeAnimator::default();
        a.step(&c, &HUMANOID, &FoeDrive { reel: Some((FoeReel::Stagger, 1.0)), ..d }, &mut out);
        // (Into a reel, it fades from where it stood: a frame of the eight.)
        assert!(spine(&out) < 0.0, "{}", spine(&out));
    }
}
