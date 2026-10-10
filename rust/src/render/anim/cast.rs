//! The story's cast by clips (ANIMATION.md §12): which clips play for a boy, the old man, his dog or
//! Krampus at each moment of a scene, where in them, and how they blend; the story's view says what
//! each is doing (`Play`: a base clip and the clips laid over it, each at its time and weight) and
//! the animator gives the pose. Presentation only: the story's beats and timings are the truth, a
//! clip is placed on them (a kick's contact on the ball's moment, the grab on the grab's).
//!
//! It works on each bone's turn in the model away from its rest (its "delta"), not on its turn
//! relative to its parent: so clips made on another body of the same bones play on this one (Scene
//! I's Krampus plays the lair's Krampus's clips, his body three fifths of that one's, his arms a
//! little otherwise), clips blend whatever rests their files give their bones, and what the view
//! lays on top is simple (a head turned to look, a staff held upright, a scarf hanging). The pose is
//! put together down the skeleton from this body's own joints (`matrices`).
//!
//! A change of what plays (`Play::key`) crossfades from the pose before over `BLEND` (or the play's
//! own); a jump in time (a capture, a skip) starts afresh.

use std::f32::consts::PI;

use glam::{Mat4, Quat, Vec3};

use super::clip::Clip;
use super::library::ClipSet;
use super::skeleton::SkeletonDef;

/// Seconds a change of what plays crossfades over (8 frames), unless the play says otherwise.
pub const BLEND: f32 = 8.0 / 60.0;

/// A body's pose as the cast animator keeps it: each bone's turn in the model away from its rest,
/// and how far the root has moved from its rest (metres of this body).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Delta {
    pub rot: Vec<Quat>,
    pub root: Vec3,
}

impl Delta {
    pub fn rest(n: usize) -> Delta {
        Delta { rot: vec![Quat::IDENTITY; n], root: Vec3::ZERO }
    }

    /// `w` of the way to `b`, on the bones of `mask` (all of them if none).
    pub fn blend(&mut self, b: &Delta, w: f32, mask: Option<&[bool]>) {
        if w <= 0.0 {
            return;
        }
        let w = w.min(1.0);
        for (i, (a, b)) in self.rot.iter_mut().zip(&b.rot).enumerate() {
            if mask.is_none_or(|m| m.get(i).copied().unwrap_or(false)) {
                let b = if a.dot(*b) < 0.0 { -*b } else { *b };
                *a = a.slerp(b, w).normalize();
            }
        }
        if mask.is_none_or(|m| m.first().copied().unwrap_or(false)) {
            self.root = self.root.lerp(b.root, w);
        }
    }

    /// Turns bone `b` and everything below it by `q` in the model (about its joint: the bones
    /// below follow it round, as `matrices` puts them together).
    pub fn turn(&mut self, def: &SkeletonDef, b: usize, q: Quat) {
        for i in 0..self.rot.len() {
            if i == b || descends(def, i, b) {
                self.rot[i] = (q * self.rot[i]).normalize();
            }
        }
    }
}

/**
 * Lengthens a gait's stride: each of `bones` swings `gain` times as far from its parent as the pose
 * has it (its turn's angle scaled about the same axis), what hangs below it following. A child's legs
 * cover the ground a run carries a man over only by reaching further and turning over faster
 * (`view::cast_clips`, the boys' runs).
 */
pub fn lengthen(pose: &mut Delta, def: &SkeletonDef, bones: &[usize], gain: f32) {
    if (gain - 1.0).abs() < 1e-4 {
        return;
    }
    let old = pose.rot.clone();
    for i in 0..old.len() {
        let Some(p) = def.parents.get(i).copied().flatten() else { continue };
        let mut local = (old[p].inverse() * old[i]).normalize();
        if bones.contains(&i) {
            let (axis, angle) = local.to_axis_angle();
            let angle = if angle > PI { angle - 2.0 * PI } else { angle };
            local = Quat::from_axis_angle(axis, angle * gain);
        }
        pose.rot[i] = (pose.rot[p] * local).normalize();
    }
}

/**
 * A run's cycle as the ground carries it (ANIMATION.md §12.3, "a calmer sprint"): measured once on
 * a gait's clip, lengthened `gain` times, which samples of its cycle have a foot planted and how far
 * that foot sweeps back under the body over each, and which are in the air. A child running hard
 * takes long strides with a long flight between them, not quick short ones: placed on a stride of
 * the ground longer than its planted feet carry (`at`), the clip plays its stance at the pace the
 * ground passes under the planted foot (so it stays where it lands) and stretches its flight over
 * the rest.
 */
#[derive(Clone, Debug)]
pub struct Stride {
    /// Each sample interval's sweep of the planted foot (metres), or none in the air.
    sweep: Vec<Option<f32>>,
    /// The ground the planted feet carry the body over a cycle.
    pub planted: f32,
    /// The sample the measure starts from (a foot coming down, so no flight runs over the end).
    shift: usize,
}

/// Samples a cycle is measured at.
const STRIDE_SAMPLES: usize = 96;

impl Stride {
    /// Measures clip `clip`'s cycle with `legs` lengthened `gain` times (`lengthen`), its feet the
    /// joints of `feet`, on this body's `joints`.
    pub fn measure(clip: &Clip, def: &SkeletonDef, joints: &[Vec3], legs: &[usize], feet: [usize; 2], gain: f32) -> Stride {
        let n = STRIDE_SAMPLES;
        let (mut locals, mut world, mut pose, mut mats) = (Vec::new(), Vec::new(), Delta::default(), Vec::new());
        let track: Vec<[Vec3; 2]> = (0..n)
            .map(|k| {
                sample_delta(clip, k as f32 / n as f32 * clip.duration(), 1.0, &mut locals, &mut world, &mut pose);
                pose.rot.resize(def.len(), Quat::IDENTITY);
                lengthen(&mut pose, def, legs, gain);
                matrices(def, joints, &pose, &mut mats);
                feet.map(|f| mats[f].w_axis.truncate())
            })
            .collect();
        let floor = track.iter().flat_map(|f| f.iter().map(|p| p.y)).fold(f32::MAX, f32::min);
        let sweep: Vec<Option<f32>> = (0..n)
            .map(|k| {
                let (a, b) = (track[k], track[(k + 1) % n]);
                let i = if a[0].y < a[1].y { 0 } else { 1 };
                let back = a[i].z - b[i].z;
                (a[i].y < floor + 0.02 && b[i].y < floor + 0.02 && back > 0.0).then_some(back)
            })
            .collect();
        let planted = sweep.iter().flatten().sum();
        let shift = (0..n).find(|&k| sweep[k].is_some() && sweep[(k + n - 1) % n].is_none()).unwrap_or(0);
        let sweep = (0..n).map(|k| sweep[(k + shift) % n]).collect();
        Stride { sweep, planted, shift }
    }

    /// Each interval's share of a stride `s` of the ground: the planted ones their sweep, the flights
    /// the rest (at least a little each, the whole scaled to `s` when the planted feet carry more).
    fn ground(&self, s: f32) -> impl Iterator<Item = f32> + '_ {
        let n = self.sweep.len() as f32;
        let flights = self.sweep.iter().filter(|w| w.is_none()).count().max(1) as f32;
        let air = ((s - self.planted) / flights).max(0.2 * s / n);
        let total = self.planted + air * self.sweep.iter().filter(|w| w.is_none()).count() as f32;
        let k = s / total.max(1e-6);
        self.sweep.iter().map(move |w| w.unwrap_or(air) * k)
    }

    /// `d` metres into a stride of `s` metres (wrapping): the share of the clip's cycle there, and
    /// how far through its flight the body is (none on a planted foot).
    pub fn at(&self, d: f32, s: f32) -> (f32, Option<f32>) {
        let n = self.sweep.len();
        let d = d.rem_euclid(s.max(1e-4));
        let mut at = 0.0;
        for (k, g) in self.ground(s).enumerate() {
            if d < at + g || k == n - 1 {
                let u = ((k + self.shift) as f32 + ((d - at) / g.max(1e-6)).clamp(0.0, 1.0)) / n as f32;
                if self.sweep[k].is_some() {
                    return (u, None);
                }
                // (Through this flight: from its first interval in the air to its last.)
                let (mut a, mut b) = (k, k);
                while a > 0 && self.sweep[a - 1].is_none() {
                    a -= 1;
                }
                while b + 1 < n && self.sweep[b + 1].is_none() {
                    b += 1;
                }
                let ground: Vec<f32> = self.ground(s).collect();
                let (from, len) = (ground[..a].iter().sum::<f32>(), ground[a..=b].iter().sum::<f32>());
                return (u, Some(((d - from) / len.max(1e-6)).clamp(0.0, 1.0)));
            }
            at += g;
        }
        (0.0, None)
    }

    /// The ground of a stride `s` spent in the air (metres).
    pub fn flight(&self, s: f32) -> f32 {
        self.ground(s).zip(&self.sweep).filter(|(_, w)| w.is_none()).map(|(g, _)| g).sum()
    }
}

/// Whether bone `i` hangs (at any depth) from bone `b`.
pub fn descends(def: &SkeletonDef, mut i: usize, b: usize) -> bool {
    while let Some(p) = def.parents.get(i).copied().flatten() {
        if p == b {
            return true;
        }
        i = p;
    }
    false
}

/**
 * Clip `clip`'s pose `t` seconds in as turns away from its own rest, its root's move scaled by
 * `scale` (this body's size over the body the clip was made on), into `out`. `world` is a buffer.
 */
pub fn sample_delta(clip: &Clip, t: f32, scale: f32, locals: &mut Vec<super::clip::Trs>, world: &mut Vec<Mat4>, out: &mut Delta) {
    let n = clip.bones();
    locals.resize(n, super::clip::Trs::IDENTITY);
    clip.sample_into(t, locals);
    clip.skeleton.matrices_into(locals, &[], &[], world);
    out.rot.resize(n, Quat::IDENTITY);
    for (i, m) in world.iter().enumerate() {
        out.rot[i] = Quat::from_mat4(m).normalize();
    }
    // The root's joint at rest, and where the pose carries it.
    let rest = clip.skeleton.rest_inv[0].inverse().w_axis.truncate();
    out.root = (world[0].transform_point3(rest) - rest) * scale;
}

/// Each part's matrix in the model for `pose`, put together down the skeleton from this body's
/// joints (`joints[i]`, each part's rest: its voxels are kept relative to it).
pub fn matrices(def: &SkeletonDef, joints: &[Vec3], pose: &Delta, out: &mut Vec<Mat4>) {
    out.clear();
    let mut at: Vec<Vec3> = Vec::with_capacity(joints.len());
    for i in 0..joints.len() {
        let q = pose.rot.get(i).copied().unwrap_or(Quat::IDENTITY);
        let p = match def.parents.get(i).copied().flatten() {
            Some(pa) => at[pa] + pose.rot.get(pa).copied().unwrap_or(Quat::IDENTITY) * (joints[i] - joints[pa]),
            None => joints[i] + pose.root,
        };
        at.push(p);
        out.push(Mat4::from_rotation_translation(q, p));
    }
}

/// Which bones a layer covers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mask {
    /// The whole body.
    All,
    /// All but the hips and legs (`SkeletonDef::legs`): the upper body over a gait.
    Upper,
    /// These bones and everything below them.
    Under(&'static [&'static str]),
    /// The upper body but these bones and everything below them (a speaker's gestures over the legs
    /// he stands on, but the arm that holds his staff).
    UpperBut(&'static [&'static str]),
}

/// A clip at a moment: its name (or failing it, another's), seconds into it (a loop wraps), its weight
/// over what is beneath it, the bones it covers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer {
    pub clips: [&'static str; 2],
    pub t: f32,
    pub w: f32,
    pub mask: Mask,
    /// `t` is a share of the clip's length (a gait's cycle), not seconds.
    pub frac: bool,
    /// `t` counts from this marker of the clip (a kick's contact, `hit`), not from its start.
    pub from: Option<&'static str>,
}

impl Layer {
    pub const fn of(clip: &'static str, t: f32) -> Layer {
        Layer { clips: [clip, clip], t, w: 1.0, mask: Mask::All, frac: false, from: None }
    }

    /// A share `u` of the way through clip `clip` (a cycle).
    pub const fn cycle(clip: &'static str, u: f32) -> Layer {
        Layer { frac: true, ..Layer::of(clip, u) }
    }

    /// `t` seconds from marker `mark` of clip `clip` (negative before it).
    pub const fn at_mark(clip: &'static str, mark: &'static str, t: f32) -> Layer {
        Layer { from: Some(mark), ..Layer::of(clip, t) }
    }

    /// Seconds into `clip` this layer stands at.
    pub fn time_in(&self, clip: &Clip) -> f32 {
        let t = if self.frac { self.t.rem_euclid(1.0) * clip.duration() } else { self.t };
        t + self.from.and_then(|m| clip.mark(m)).unwrap_or(0.0)
    }

    pub const fn or(self, other: &'static str) -> Layer {
        Layer { clips: [self.clips[0], other], ..self }
    }

    pub const fn weight(self, w: f32) -> Layer {
        Layer { w, ..self }
    }

    pub const fn on(self, mask: Mask) -> Layer {
        Layer { mask, ..self }
    }
}

/// What one of the cast plays this frame: up to four layers, the first the base, and a key naming
/// the state (a change of it crossfades), and how long that crossfade takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Play {
    pub layers: [Option<Layer>; 4],
    pub key: u32,
    pub fade: f32,
}

impl Play {
    pub fn base(l: Layer, key: u32) -> Play {
        Play { layers: [Some(l), None, None, None], key, fade: BLEND }
    }

    /// `l` laid over what is there (dropped if four are there already).
    pub fn with(mut self, l: Layer) -> Play {
        if let Some(slot) = self.layers.iter_mut().find(|s| s.is_none()) {
            *slot = Some(l);
        }
        self
    }

    pub fn fading(self, fade: f32) -> Play {
        Play { fade, ..self }
    }
}

/// The clips one of the cast may play: its own first, then others (the lair's Krampus's for
/// Scene I's), each with the size of this body over the body they were made on.
pub type Sources<'a> = &'a [(&'a ClipSet, f32)];

fn find<'a>(sources: Sources<'a>, names: &[&str; 2]) -> Option<(&'a Clip, f32)> {
    names.iter().find_map(|n| sources.iter().find_map(|(set, scale)| set.get(n).map(|c| (c, *scale))))
}

/// One of the cast's animator: its pose from frame to frame, for crossfades.
#[derive(Clone, Debug, Default)]
pub struct CastAnimator {
    key: Option<u32>,
    fade: Option<(Delta, f32, f32)>,
    /// The pose last given, and a buffer for the next.
    prev: Delta,
    cur: Delta,
    layer: Delta,
    locals: Vec<super::clip::Trs>,
    world: Vec<Mat4>,
    masks: Vec<(Mask, Vec<bool>)>,
}

impl CastAnimator {
    /// The pose for `play` after `dt` seconds (none, a jump: no crossfade), or none when its base
    /// clip is missing (the code's pose is drawn instead).
    pub fn step(&mut self, sources: Sources, def: &SkeletonDef, play: &Play, dt: Option<f32>) -> Option<&Delta> {
        let n = def.len();
        let base = play.layers[0]?;
        let (clip, scale) = find(sources, &base.clips)?;
        let mut pose = std::mem::take(&mut self.cur);
        sample_delta(clip, base.time_in(clip), scale, &mut self.locals, &mut self.world, &mut pose);
        pose.rot.resize(n, Quat::IDENTITY);
        for l in play.layers[1..].iter().flatten() {
            let Some((clip, scale)) = find(sources, &l.clips) else { continue };
            if l.w <= 0.0 {
                continue;
            }
            let mut layer = std::mem::take(&mut self.layer);
            sample_delta(clip, l.time_in(clip), scale, &mut self.locals, &mut self.world, &mut layer);
            layer.rot.resize(n, Quat::IDENTITY);
            let mask = self.mask(def, l.mask);
            pose.blend(&layer, l.w, mask.as_deref());
            self.layer = layer;
        }
        // A change of state fades from where it was.
        match dt {
            None => self.fade = None,
            Some(dt) => {
                if self.key != Some(play.key) && self.key.is_some() && self.prev.rot.len() == n {
                    self.fade = (play.fade > 0.0).then(|| (self.prev.clone(), 0.0, play.fade));
                }
                if let Some((from, f, len)) = self.fade.as_mut() {
                    *f += dt;
                    let u = (*f / *len).clamp(0.0, 1.0);
                    let u = u * u * (3.0 - 2.0 * u);
                    let mut mixed = from.clone();
                    mixed.blend(&pose, u, None);
                    pose = mixed;
                    if *f >= *len {
                        self.fade = None;
                    }
                }
            }
        }
        self.key = Some(play.key);
        self.cur = std::mem::replace(&mut self.prev, pose);
        Some(&self.prev)
    }

    /// The pose last given (the view lays its touches on a copy of it).
    pub fn pose(&self) -> &Delta {
        &self.prev
    }

    /// The bones a mask covers, worked out once per mask.
    fn mask(&mut self, def: &SkeletonDef, m: Mask) -> Option<Vec<bool>> {
        if m == Mask::All {
            return None;
        }
        if let Some((_, v)) = self.masks.iter().find(|(k, _)| *k == m) {
            return Some(v.clone());
        }
        let n = def.len();
        let v: Vec<bool> = match m {
            Mask::All => vec![true; n],
            Mask::Upper => (0..n).map(|i| !def.legs.contains(&i)).collect(),
            Mask::Under(names) => {
                let roots: Vec<usize> = names.iter().filter_map(|b| def.index(b)).collect();
                (0..n).map(|i| roots.iter().any(|&r| i == r || descends(def, i, r))).collect()
            }
            Mask::UpperBut(names) => {
                let roots: Vec<usize> = names.iter().filter_map(|b| def.index(b)).collect();
                (0..n).map(|i| !def.legs.contains(&i) && !roots.iter().any(|&r| i == r || descends(def, i, r))).collect()
            }
        };
        self.masks.push((m, v.clone()));
        Some(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::anim::cast_rigs::CastRig;
    use crate::render::anim::library::cast_clips;

    const WANTED: [(CastRig, &[&str]); 4] = [
        (
            CastRig::Kid(0),
            &[
                "idle",
                "idle_happy",
                "idle_happy_2",
                "look",
                "look_nervous",
                "pass",
                "pass_left",
                "kick",
                "miskick",
                "terrified",
                "yell",
                "run",
                "sprint",
                "walk",
                "walk_back",
                "fall",
                "land",
                "talk",
                "talk_m",
                "nod",
                "reach",
                "hug",
                "yell_long",
                "yell_back",
                "arms_up",
                "alarm",
                "reach_out",
            ],
        ),
        (CastRig::OldMan, &["idle", "walk", "talk", "talk_m", "point", "nod", "look", "wave"]),
        (CastRig::Dog, &["idle", "sleep", "lie", "lie_wag", "stand_up", "lie_down", "snarl"]),
        (CastRig::Krampus, &["grab", "hold_up"]),
    ];

    #[test]
    fn the_casts_clips_read_carry_their_markers_and_rest_on_their_joints() {
        for (c, names) in WANTED {
            let set = cast_clips(c).unwrap_or_else(|| panic!("{c:?} has clips"));
            assert!(set.errors.is_empty(), "{c:?}: {:?}", set.errors);
            for n in names {
                let clip = set.get(n).unwrap_or_else(|| panic!("{c:?} has no {n}"));
                assert!(clip.frame_count() > 10, "{c:?} {n}");
            }
            let rig = c.jointed();
            for clip in &set.clips {
                for (b, part) in rig.parts.iter().enumerate() {
                    let at = clip.skeleton.rest_inv[b].inverse().w_axis.truncate();
                    assert!(at.abs_diff_eq(part.rest, 1e-4), "{c:?} {}: {} at {at} not {}", clip.name, part.name, part.rest);
                }
            }
        }
        let kid = cast_clips(CastRig::Kid(0)).unwrap();
        for kick in ["pass", "pass_left", "kick", "miskick"] {
            let k = kid.get(kick).unwrap();
            assert!(k.mark("hit").is_some_and(|h| h > 0.05 && h < k.duration()), "{kick}: its contact");
        }
        for cycle in ["idle", "run", "sprint", "walk", "terrified", "fall", "talk"] {
            assert!(kid.get(cycle).unwrap().markers.looping, "{cycle} loops");
        }
        assert!(kid.get("land").unwrap().mark("land").is_some());
        let k = cast_clips(CastRig::Krampus).unwrap().get("grab").unwrap();
        assert!((k.mark("grab").unwrap() - 0.2).abs() < 1e-3, "the claws close 0.2 s in, as the story's grab follows the gate's burst");
    }

    #[test]
    fn a_pose_at_rest_puts_every_part_on_its_joint_and_a_turn_carries_what_hangs_from_it() {
        let c = CastRig::Kid(1);
        let def = c.joints().def;
        let rig = c.jointed();
        let joints: Vec<Vec3> = rig.parts.iter().map(|p| p.rest).collect();
        let mut pose = Delta::rest(def.len());
        let mut m = Vec::new();
        matrices(def, &joints, &pose, &mut m);
        for (i, m) in m.iter().enumerate() {
            assert!(m.abs_diff_eq(Mat4::from_translation(joints[i]), 1e-5));
        }
        // The chest turned: the scarf and the head go round with it, the hips stay.
        let chest = def.index("chest").unwrap();
        pose.turn(def, chest, Quat::from_rotation_y(1.0));
        matrices(def, &joints, &pose, &mut m);
        let (head, scarf, hips) = (def.index("head").unwrap(), def.index("scarf").unwrap(), def.index("hips").unwrap());
        assert!(m[hips].abs_diff_eq(Mat4::from_translation(joints[hips]), 1e-5));
        for b in [head, scarf] {
            let want = joints[chest] + Quat::from_rotation_y(1.0) * (joints[b] - joints[chest]);
            assert!(m[b].w_axis.truncate().abs_diff_eq(want, 1e-5), "{}", def.bones[b]);
        }
        // Any clip played by the animator gives a pose of every bone; a missing one gives none.
        let set = cast_clips(c).unwrap();
        let mut a = CastAnimator::default();
        let play = Play::base(Layer::cycle("run", 0.3), 1).with(Layer::of("terrified", 1.0).on(Mask::Upper).weight(0.5));
        assert_eq!(a.step(&[(set, 1.0)], def, &play, Some(1.0 / 60.0)).map(|p| p.rot.len()), Some(def.len()));
        assert!(a.step(&[(set, 1.0)], def, &Play::base(Layer::of("no such clip", 0.0), 2), None).is_none());
    }

    #[test]
    fn scene_one_krampus_plays_the_lairs_clips_on_his_own_bones() {
        use crate::render::anim::foe_rigs::FoeRig;
        let lair = crate::render::anim::library::foe_clips(FoeRig::Krampus).unwrap();
        let shade = cast_clips(CastRig::Krampus).unwrap();
        let def = CastRig::Krampus.joints().def;
        let scale = crate::render::cast::KRAMPUS_VOXEL / crate::render::foes_lair::KRAMPUS_VOXEL;
        let mut a = CastAnimator::default();
        let run = Play::base(Layer::cycle("run", 0.25), 4).with(Layer::of("hold_up", 0.3).on(Mask::Under(&["upper_arm.R"])));
        let pose = a.step(&[(shade, 1.0), (lair, scale)], def, &run, None).unwrap().clone();
        // His run's bob is his size (a few centimetres), not the lair's.
        assert!(pose.root.length() < 0.12, "{}", pose.root);
        // His right arm held up over his head with his catch.
        let rig = CastRig::Krampus.jointed();
        let joints: Vec<Vec3> = rig.parts.iter().map(|p| p.rest).collect();
        let mut m = Vec::new();
        matrices(def, &joints, &pose, &mut m);
        let hand = def.index("hand.R").unwrap();
        let shoulder = def.index("upper_arm.R").unwrap();
        assert!(m[hand].w_axis.y > m[shoulder].w_axis.y, "the hand {} above the shoulder {}", m[hand].w_axis.y, m[shoulder].w_axis.y);
    }
}
