//! Clips: glTF skeletal animation on the heroes' skeleton, read once from the GLB files embedded
//! in the game, resampled at 60 Hz (the combat frame), and sampled between two frames with a
//! quaternion slerp. Each clip has a sidecar of markers (`Markers`) that Blender's export writes.

use glam::{Mat4, Quat, Vec3};

use super::skeleton::{BONES, BONE_COUNT, PARENTS};

/// The rate clips are kept at: one key a combat frame.
pub const FPS: f32 = 60.0;

/// A joint's transform relative to its parent: translation, rotation and scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trs {
    pub t: Vec3,
    pub r: Quat,
    pub s: Vec3,
}

impl Trs {
    pub const IDENTITY: Trs = Trs { t: Vec3::ZERO, r: Quat::IDENTITY, s: Vec3::ONE };

    pub fn mat(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.s, self.r, self.t)
    }

    /// `u` of the way from `self` to `b`: the rotation along the shorter arc.
    pub fn lerp(&self, b: &Trs, u: f32) -> Trs {
        let r = if self.r.dot(b.r) < 0.0 { -b.r } else { b.r };
        Trs { t: self.t.lerp(b.t, u), r: self.r.slerp(r, u).normalize(), s: self.s.lerp(b.s, u) }
    }
}

/// A pose of the skeleton: every bone's transform relative to its parent.
pub type Locals = [Trs; BONE_COUNT];

/// `u` of the way from pose `a` to pose `b`, bone by bone.
pub fn blend(a: &Locals, b: &Locals, u: f32) -> Locals {
    if u <= 0.0 {
        return *a;
    }
    if u >= 1.0 {
        return *b;
    }
    std::array::from_fn(|i| a[i].lerp(&b[i], u))
}

/// The markers of a clip, as its sidecar JSON gives them: named moments in frames from its start
/// (`windup_end`, `hit`, `active_end`, `recover_end`, `foot_l`, `foot_r`, `takeoff`, `apex`,
/// `land`, `raised`...), whether it loops, and for a gait the ground it covers in one cycle.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Markers {
    pub marks: Vec<(String, f32)>,
    pub looping: bool,
    /// Metres (tiles) a cycle of a gait carries him, for matching its rate to his pace.
    pub stride: Option<f32>,
}

impl Markers {
    /// Reads a sidecar: `{"clip": "run", "fps": 60, "frames": 36, "loop": true, "stride": 2.0,
    /// "markers": {"foot_l": 0, "foot_r": 18}}`. Marker frames at another `fps` are brought to 60 Hz.
    pub fn parse(json: &str) -> Result<Markers, String> {
        let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("markers: {e}"))?;
        let fps = v.get("fps").and_then(|f| f.as_f64()).unwrap_or(FPS as f64) as f32;
        let mut marks: Vec<(String, f32)> = v
            .get("markers")
            .and_then(|m| m.as_object())
            .map(|m| m.iter().filter_map(|(k, f)| f.as_f64().map(|f| (k.clone(), f as f32 * FPS / fps))).collect())
            .unwrap_or_default();
        marks.sort_by(|a, b| a.1.total_cmp(&b.1));
        Ok(Markers {
            marks,
            looping: v.get("loop").and_then(|l| l.as_bool()).unwrap_or(false),
            stride: v.get("stride").and_then(|s| s.as_f64()).map(|s| s as f32),
        })
    }

    /// The frame of marker `name`.
    pub fn get(&self, name: &str) -> Option<f32> {
        self.marks.iter().find(|m| m.0 == name).map(|m| m.1)
    }
}

/// The skeleton's rest as the clips have it: each bone's rest transform, what lies above the root
/// (the armature's own node), and each bone's rest matrix in the model inverted (to bind a voxel to
/// it). Every clip of a hero shares it.
#[derive(Clone, Debug)]
pub struct Skeleton {
    pub rest: Locals,
    pub pre: Mat4,
    pub rest_inv: [Mat4; BONE_COUNT],
}

impl Skeleton {
    /// The skeleton with `rest`, under `pre`.
    pub fn new(rest: Locals, pre: Mat4) -> Self {
        let world = world_of(&rest, pre, &[Quat::IDENTITY; BONE_COUNT]);
        Self { rest, pre, rest_inv: world.map(|m| m.inverse()) }
    }

    /**
     * Each part's matrix in the model for pose `locals`: its bone's matrix now, times its bone's
     * rest matrix inverted, times the joint's place at rest (`joints[i]`, the part's `rest`: its
     * voxels are kept relative to that). At rest every part's matrix is the plain translation to
     * its joint, as the code's pose draws it. `extra` turns a bone (and so everything below it) about
     * its joint in the model's axes (the juice: a blade's overshoot, a shield's jolt).
     */
    pub fn matrices(&self, locals: &Locals, joints: &[Vec3], extra: &[Quat; BONE_COUNT]) -> [Mat4; BONE_COUNT] {
        let world = world_of(locals, self.pre, extra);
        std::array::from_fn(|i| world[i] * self.rest_inv[i] * Mat4::from_translation(joints.get(i).copied().unwrap_or(Vec3::ZERO)))
    }

    /// Every bone's matrix in the model for pose `locals` (no juice).
    pub fn world(&self, locals: &Locals) -> [Mat4; BONE_COUNT] {
        world_of(locals, self.pre, &[Quat::IDENTITY; BONE_COUNT])
    }
}

/// Every bone's matrix in the model for `locals`, parents first; `extra` turns a bone about its
/// joint in the model's axes before its children follow.
fn world_of(locals: &Locals, pre: Mat4, extra: &[Quat; BONE_COUNT]) -> [Mat4; BONE_COUNT] {
    let mut world = [Mat4::IDENTITY; BONE_COUNT];
    for i in 0..BONE_COUNT {
        let parent = PARENTS[i].map(|p| world[p]).unwrap_or(pre);
        let mut m = parent * locals[i].mat();
        if extra[i] != Quat::IDENTITY {
            let at = m.w_axis.truncate();
            m = Mat4::from_translation(at) * Mat4::from_quat(extra[i]) * Mat4::from_translation(-at) * m;
        }
        world[i] = m;
    }
    world
}

/// A clip, resampled to one pose every 60th of a second.
#[derive(Clone, Debug)]
pub struct Clip {
    pub name: String,
    pub markers: Markers,
    /// The skeleton's rest as this clip's file has it.
    pub skeleton: Skeleton,
    /// `frames.len() / BONE_COUNT` poses, bone by bone.
    frames: Vec<Trs>,
}

impl Clip {
    /// How many 60 Hz frames it has (at least one).
    pub fn frame_count(&self) -> usize {
        self.frames.len() / BONE_COUNT
    }

    /// Its length in seconds: from its first frame to its last (a loop's last frame is its first again).
    pub fn duration(&self) -> f32 {
        (self.frame_count() - 1) as f32 / FPS
    }

    /// Marker `name` in seconds.
    pub fn mark(&self, name: &str) -> Option<f32> {
        self.markers.get(name).map(|f| f / FPS)
    }

    /// The pose `t` seconds in: between the two frames about it, the rotations slerped. A looping
    /// clip wraps round; another holds its ends.
    pub fn sample(&self, t: f32) -> Locals {
        let n = self.frame_count();
        let last = (n - 1) as f32;
        let mut f = t * FPS;
        if self.markers.looping && last > 0.0 {
            f = f.rem_euclid(last);
        }
        let f = f.clamp(0.0, last);
        let i = (f.floor() as usize).min(n - 1);
        let j = (i + 1).min(n - 1);
        let u = f - i as f32;
        let (a, b) = (&self.frames[i * BONE_COUNT..(i + 1) * BONE_COUNT], &self.frames[j * BONE_COUNT..(j + 1) * BONE_COUNT]);
        std::array::from_fn(|k| a[k].lerp(&b[k], u))
    }

    /// A clip made of poses keyed at 60 Hz (tests, and clips made in code).
    pub fn from_frames(name: &str, skeleton: Skeleton, poses: &[Locals], markers: Markers) -> Clip {
        let frames = poses.iter().flat_map(|p| p.iter().copied()).collect();
        Clip { name: name.to_string(), markers, skeleton, frames }
    }

    /**
     * Reads a clip from a GLB file and its markers' JSON. The file holds the skeleton (nodes named
     * as `BONES`, each bone's parent the skeleton's, the root under any nodes of its own) and an
     * animation (the first) of their translations, rotations and scales; tracks for other nodes
     * are skipped, and a bone with no track holds its rest. Linear and step keys are read as they
     * are; a cubic spline's keys are taken at their values (Blender writes linear keys, sampled
     * every frame).
     */
    pub fn from_glb(name: &str, glb: &[u8], json: &str) -> Result<Clip, String> {
        let markers = Markers::parse(json)?;
        let gltf = gltf::Gltf::from_slice(glb).map_err(|e| format!("{name}: {e}"))?;
        let blob = gltf.blob.as_deref();
        let nodes: Vec<gltf::Node> = gltf.nodes().collect();
        let mut parent = vec![None; nodes.len()];
        for n in &nodes {
            for c in n.children() {
                parent[c.index()] = Some(n.index());
            }
        }
        let trs = |n: &gltf::Node| {
            let (t, r, s) = n.transform().decomposed();
            Trs { t: Vec3::from(t), r: Quat::from_array(r).normalize(), s: Vec3::from(s) }
        };
        // The bones' nodes, by name; each bone's parent must be the skeleton's.
        let mut node_of = [usize::MAX; BONE_COUNT];
        for (b, bone) in BONES.iter().enumerate() {
            let n = nodes.iter().find(|n| n.name() == Some(*bone)).ok_or_else(|| format!("{name}: no bone {bone}"))?;
            node_of[b] = n.index();
        }
        for b in 0..BONE_COUNT {
            let want = PARENTS[b].map(|p| node_of[p]);
            let has = parent[node_of[b]];
            if PARENTS[b].is_some() && has != want {
                return Err(format!("{name}: {} hangs from the wrong bone", BONES[b]));
            }
        }
        // What lies above the root.
        let mut pre = Mat4::IDENTITY;
        let mut up = parent[node_of[0]];
        while let Some(p) = up {
            pre = trs(&nodes[p]).mat() * pre;
            up = parent[p];
        }
        let rest: Locals = std::array::from_fn(|b| trs(&nodes[node_of[b]]));
        let skeleton = Skeleton::new(rest, pre);

        // The tracks: (bone, which, times, values).
        enum Values {
            T(Vec<Vec3>),
            R(Vec<Quat>),
            S(Vec<Vec3>),
        }
        let mut tracks: Vec<(usize, Values, Vec<f32>, bool)> = Vec::new();
        let mut end = 0.0f32;
        if let Some(anim) = gltf.animations().next() {
            for ch in anim.channels() {
                let node = ch.target().node().index();
                let Some(bone) = node_of.iter().position(|&n| n == node) else { continue };
                let reader = ch.reader(|b| match b.source() {
                    gltf::buffer::Source::Bin => blob,
                    gltf::buffer::Source::Uri(_) => None,
                });
                let times: Vec<f32> = reader.read_inputs().ok_or_else(|| format!("{name}: a track without times"))?.collect();
                let interp = ch.sampler().interpolation();
                let cubic = interp == gltf::animation::Interpolation::CubicSpline;
                // A cubic spline's outputs come in threes (in-tangent, value, out-tangent).
                let pick = |k: usize| if cubic { k % 3 == 1 } else { true };
                let values = match reader.read_outputs().ok_or_else(|| format!("{name}: a track without values"))? {
                    gltf::animation::util::ReadOutputs::Translations(it) => {
                        Values::T(it.enumerate().filter(|(k, _)| pick(*k)).map(|(_, v)| Vec3::from(v)).collect())
                    }
                    gltf::animation::util::ReadOutputs::Rotations(it) => Values::R(
                        it.into_f32().enumerate().filter(|(k, _)| pick(*k)).map(|(_, v)| Quat::from_array(v).normalize()).collect(),
                    ),
                    gltf::animation::util::ReadOutputs::Scales(it) => {
                        Values::S(it.enumerate().filter(|(k, _)| pick(*k)).map(|(_, v)| Vec3::from(v)).collect())
                    }
                    gltf::animation::util::ReadOutputs::MorphTargetWeights(_) => continue,
                };
                let n = match &values {
                    Values::T(v) | Values::S(v) => v.len(),
                    Values::R(v) => v.len(),
                };
                if n != times.len() || n == 0 {
                    return Err(format!("{name}: a track of {} keys and {} times", n, times.len()));
                }
                end = end.max(*times.last().unwrap_or(&0.0));
                tracks.push((bone, values, times, interp == gltf::animation::Interpolation::Step));
            }
        }
        // The first key is the clip's start (Blender slides an action to zero; a clip that starts later keeps its lead-in).
        let start = tracks.iter().filter_map(|t| t.2.first().copied()).fold(f32::INFINITY, f32::min);
        let start = if start.is_finite() { start } else { 0.0 };
        let frames_n = ((end - start) * FPS).round().max(0.0) as usize + 1;
        let mut frames = Vec::with_capacity(frames_n * BONE_COUNT);
        for f in 0..frames_n {
            let t = start + f as f32 / FPS;
            let mut pose = rest;
            for (bone, values, times, step) in &tracks {
                // The key at or before t, and how far on to the next.
                let k = times.partition_point(|&x| x <= t).saturating_sub(1);
                let k1 = (k + 1).min(times.len() - 1);
                let u = if k1 == k || *step { 0.0 } else { ((t - times[k]) / (times[k1] - times[k])).clamp(0.0, 1.0) };
                let p = &mut pose[*bone];
                match values {
                    Values::T(v) => p.t = v[k].lerp(v[k1], u),
                    Values::S(v) => p.s = v[k].lerp(v[k1], u),
                    Values::R(v) => {
                        let b = if v[k].dot(v[k1]) < 0.0 { -v[k1] } else { v[k1] };
                        p.r = v[k].slerp(b, u).normalize();
                    }
                }
            }
            frames.extend_from_slice(&pose);
        }
        Ok(Clip { name: name.to_string(), markers, skeleton, frames })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skeleton() -> Skeleton {
        let rest: Locals = std::array::from_fn(|i| Trs { t: Vec3::new(0.0, 0.1 * i as f32, 0.0), ..Trs::IDENTITY });
        Skeleton::new(rest, Mat4::IDENTITY)
    }

    #[test]
    fn sampling_slerps_between_frames_and_loops() {
        let sk = skeleton();
        let mut a = sk.rest;
        let mut b = sk.rest;
        a[3].r = Quat::IDENTITY;
        b[3].r = Quat::from_rotation_x(1.0);
        let clip = Clip::from_frames("t", sk.clone(), &[a, b, a], Markers { looping: true, ..Default::default() });
        assert_eq!(clip.frame_count(), 3);
        let half = clip.sample(0.5 / FPS);
        assert!(half[3].r.angle_between(Quat::from_rotation_x(0.5)) < 1e-4);
        // Two frames on, it is back at the start; past its end it wraps.
        assert!(clip.sample(2.0 / FPS)[3].r.angle_between(Quat::IDENTITY) < 1e-4);
        assert!(clip.sample(2.5 / FPS)[3].r.angle_between(Quat::from_rotation_x(0.5)) < 1e-4);
    }

    #[test]
    fn at_rest_every_part_sits_on_its_joint() {
        let sk = skeleton();
        let joints: Vec<Vec3> = (0..BONE_COUNT).map(|i| Vec3::new(i as f32, 1.0, -2.0)).collect();
        let m = sk.matrices(&sk.rest, &joints, &[Quat::IDENTITY; BONE_COUNT]);
        for (i, m) in m.iter().enumerate() {
            assert!(m.abs_diff_eq(Mat4::from_translation(joints[i]), 1e-5), "{}", BONES[i]);
        }
    }

    #[test]
    fn a_turn_of_a_bone_carries_its_children_round_its_joint() {
        let sk = skeleton();
        let mut pose = sk.rest;
        // The chest turned a quarter about y: the shoulders and arms swing round it.
        pose[2].r = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let joints: Vec<Vec3> = (0..BONE_COUNT).map(|_| Vec3::ZERO).collect();
        let plain = sk.matrices(&sk.rest, &joints, &[Quat::IDENTITY; BONE_COUNT]);
        let turned = sk.matrices(&pose, &joints, &[Quat::IDENTITY; BONE_COUNT]);
        let probe = Vec3::new(1.0, 0.0, 0.0);
        // The chest's own probe turns; the hips' does not.
        assert!(turned[0].transform_point3(probe).abs_diff_eq(plain[0].transform_point3(probe), 1e-5));
        let c = turned[2].transform_point3(probe) - turned[2].transform_point3(Vec3::ZERO);
        assert!(c.abs_diff_eq(Vec3::new(0.0, 0.0, -1.0), 1e-5), "{c}");
        // And the extra turn does the same in the model's axes.
        let mut extra = [Quat::IDENTITY; BONE_COUNT];
        extra[2] = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let by_extra = sk.matrices(&sk.rest, &joints, &extra);
        for i in 0..BONE_COUNT {
            assert!(by_extra[i].abs_diff_eq(turned[i], 1e-5), "{}", BONES[i]);
        }
    }

    #[test]
    fn markers_read_from_their_sidecar() {
        let m = Markers::parse(r#"{"clip":"run","fps":30,"loop":true,"stride":2.0,"markers":{"foot_r":9,"foot_l":0}}"#).unwrap();
        assert!(m.looping);
        assert_eq!(m.stride, Some(2.0));
        assert_eq!(m.marks, vec![("foot_l".to_string(), 0.0), ("foot_r".to_string(), 18.0)]);
        assert!(Markers::parse("not json").is_err());
    }
}
