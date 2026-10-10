//! What the view hands the GPU each frame: instanced boxes with a shade mode, lights, the
//! atmosphere (sky, fog, sun, bloom), and the frame that gathers them.

use super::*;

/// One instanced draw. `emissive.w` carries the shading mode.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct Instance {
    pub m: [f32; 16],
    /// Linear RGB and opacity.
    pub color: [f32; 4],
    /// Linear emitted radiance and the shading mode.
    pub emissive: [f32; 4],
}

pub const SHADE_LIT: f32 = 0.0;

pub const SHADE_UNLIT: f32 = 1.0;

/// Lit, with the emissive scaled by the shared nugget pulse.
pub const SHADE_PULSE: f32 = 2.0;

/// Lit, and screen-door dithered away by `1 - alpha` (trees fading out of the camera's way).
pub const SHADE_DITHER: f32 = 3.0;

/// Lit, and on its top face a field of grass blades drawn with parallax occlusion mapping.
pub const SHADE_GRASS: f32 = 4.0;

/// A layer of the cloud sea the dungeon floats on (or, with a seed of 20 and up, ground mist).
pub const SHADE_CLOUD: f32 = 5.0;

/// Moving water: rippled, mirroring the night sky; sheets that stand upright fall as cascades.
pub const SHADE_WATER: f32 = 20.0;

/// A jet of water: a glassy rope with bubbles rushing along it.
pub const SHADE_JET: f32 = 21.0;

/// A soft glowing orb: bright in the middle, fading to nothing at its rim (additive, on a sphere).
pub const SHADE_GLOW: f32 = 22.0;

/// A halo of light round a lamp or a wisp: `radius` across, `strength` bright.
pub fn glow_orb(f: &mut Frame, at: Vec3, radius: f32, color: Vec3, strength: f32) {
    let m = Mat4::from_scale_rotation_translation(Vec3::splat(radius), f.facing, at);
    f.glow.push(Instance::new(m, color, strength, Vec3::ZERO, SHADE_GLOW));
}

impl Instance {
    pub fn new(m: Mat4, color: Vec3, alpha: f32, emissive: Vec3, shade: f32) -> Self {
        Self { m: m.to_cols_array(), color: [color.x, color.y, color.z, alpha], emissive: [emissive.x, emissive.y, emissive.z, shade] }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MeshKind {
    Cube = 0,
    Sphere = 1,
    Disc = 2,
    Ring = 3,
    Cylinder = 4,
    /// An open tube along y, flaring a little toward the bottom (a veil of falling water).
    Tube = 5,
    /// A thin full ring in the XY plane (ripples).
    Annulus = 6,
}

pub const MESH_KINDS: usize = 7;

static NEXT_SET: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// A prebuilt list of instances that does not change once made (a scene's set, its ground
/// mist): the GPU keeps it after the first upload and draws it again each frame for nothing.
#[derive(Clone, Default)]
pub struct InstanceSet {
    pub id: u64,
    pub instances: std::sync::Arc<Vec<Instance>>,
}

impl InstanceSet {
    pub fn new(instances: Vec<Instance>) -> Self {
        Self { id: NEXT_SET.fetch_add(1, std::sync::atomic::Ordering::Relaxed), instances: std::sync::Arc::new(instances) }
    }

    pub fn len(&self) -> usize {
        self.instances.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }
}

/// How a rig's voxels are coloured when drawn whole (see `RigList`).
pub mod rig_look {
    /// Plain: the voxel's colour times the tint, but glowing voxels keep their own light.
    pub const PLAIN: u32 = 0;
    /// Drained to pale stone and tinted (a hero not yet unlocked).
    pub const GREY: u32 = 1;
    /// The voxel's own colour, plain lit, screen-door faded by alpha.
    pub const FADED: u32 = 2;
    /// See-through spirit light: the tint, shaded by the colour's lightness, unlit.
    pub const GHOST: u32 = 4;
    /// The tint falls on glowing voxels too (the hero's own rig).
    pub const TINT_ALL: u32 = 8;
    /// The emitted light gathered at the silhouette (a foe's tell): a breath of it over the body,
    /// all of it on the faces the eye sees edge-on, so the move stays readable under it.
    pub const RIM: u32 = 16;
    /// Only what is not jelly (`mat::SLIME`), solid: what a slime holds (drawn with the solid rigs).
    pub const INNARDS: u32 = 32;
    /// Only the jelly, see-through and glossy, its skin bent soft and rippling (drawn with the
    /// see-through rigs, its `rust` the ripple's strength): the bog slime's (RENDERING.md).
    pub const GOO: u32 = 64;
}

/// One rig drawn from the mesh the GPU keeps of it, as the shader reads it.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct RigDraw {
    /// Linear tint and opacity.
    pub tint: [f32; 4],
    /// Linear emitted radiance, and how far the blade's metal has rusted.
    pub emissive: [f32; 4],
    /// Where its part matrices start in the frame's list.
    pub part_base: u32,
    /// `rig_look` bits.
    pub look: u32,
    /// Parts whose metal rusts (from the mesh).
    pub rust_mask: u32,
    /// The rig's voxel size.
    pub size: f32,
}

/**
 * How rigs choose their level of detail (RP12): by how many pixels one of their voxels covers
 * under this frame's camera. Below `LOD_FINE` pixels the rig drops to half the resolution, below
 * `LOD_COARSE` to a quarter: each level is used while its own voxels stay under about four
 * pixels (as the old rule of coarse foes past six units did at 1080p, now for any rig, camera and
 * screen). Coming back up wants 20% more than going down, so a rig at the edge does not flicker.
 */
#[derive(Clone, Copy, Debug)]
pub struct LodPolicy {
    pub eye: Vec3,
    /// Pixels a world unit covers one unit from a perspective camera, or anywhere under the map's orthographic one.
    pub px_per_unit: f32,
    pub ortho: bool,
    /// Higher drops detail sooner (a lower preset); 0 keeps every rig whole.
    pub bias: f32,
    /// Tints each level (whole, half, quarter) to see where they switch.
    pub debug: bool,
}

impl Default for LodPolicy {
    fn default() -> Self {
        Self { eye: Vec3::ZERO, px_per_unit: 1000.0, ortho: false, bias: 1.0, debug: false }
    }
}

pub const LOD_FINE: f32 = 2.0;
pub const LOD_COARSE: f32 = 1.0;
const LOD_RETURN: f32 = 1.2;

impl LodPolicy {
    /// Pixels a voxel `size` across covers at `at`.
    pub fn pixels(&self, size: f32, at: Vec3) -> f32 {
        if self.ortho {
            size * self.px_per_unit
        } else {
            size * self.px_per_unit / at.distance(self.eye).max(0.05)
        }
    }

    /// The level for `px` pixels a voxel, coming from level `was` (if it was drawn last frame).
    pub fn level(&self, px: f32, was: Option<u8>) -> u8 {
        if self.bias <= 0.0 {
            return 0;
        }
        let px = px / self.bias;
        let threshold = |l: u8| if l == 1 { LOD_FINE } else { LOD_COARSE };
        let target = if px < LOD_COARSE {
            2
        } else if px < LOD_FINE {
            1
        } else {
            0
        };
        match was {
            Some(mut l) if target < l => {
                while l > target && px > threshold(l) * LOD_RETURN {
                    l -= 1;
                }
                l
            }
            _ => target,
        }
    }
}

/// Rigs to draw this frame: each one's mesh, its draw record, and the posed parts' matrices;
/// the same again one level coarser for the far shadow cascade; and the level each rig was
/// drawn at last time (by rig and by its turn among that rig's draws), for the hysteresis.
#[derive(Default)]
pub struct RigList {
    pub meshes: Vec<std::sync::Arc<RigMesh>>,
    pub draws: Vec<RigDraw>,
    pub parts: Vec<[f32; 16]>,
    pub far_meshes: Vec<std::sync::Arc<RigMesh>>,
    pub far_draws: Vec<RigDraw>,
    pub lod: LodPolicy,
    /// How many rigs went out at each level this frame.
    pub levels: [u32; 3],
    memory: std::collections::HashMap<(u64, u32), (u8, u64)>,
    turns: std::collections::HashMap<u64, u32>,
    frame: u64,
}

impl RigList {
    /// Poses `rig` at `root` and queues it at the level of detail its size on screen calls for.
    /// See-through, it is drawn with every face (`Rig::mesh_whole`). A part with a parent turns
    /// with it: its joint rides on the parent's matrix, worked out first.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn push(&mut self, rig: &Rig, pose: &Pose, root: Mat4, tint: Vec3, alpha: f32, emissive: Vec3, look: u32, rust: f32) {
        self.queue(rig, root, tint, alpha, emissive, look, rust, |parts| {
            let base = parts.len();
            for (i, part) in rig.parts.iter().enumerate() {
                let m = match part.parent {
                    // (Multiplied as it always was, so a rig without parents draws exactly as before.)
                    None => root * Mat4::from_translation(pose.pos[i]) * euler(pose.rot[i]),
                    Some(p) => {
                        Mat4::from_cols_array(&parts[base + p])
                            * Mat4::from_translation(pose.pos[i] - rig.parts[p].rest)
                            * euler(pose.rot[i])
                    }
                };
                parts.push(m.to_cols_array());
            }
        });
    }

    /// Queues `rig` with each part's matrix given in the model's space (a clip's pose,
    /// `anim::Skeleton::matrices`), placed at `root`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn push_matrices(
        &mut self,
        rig: &Rig,
        mats: &[Mat4],
        root: Mat4,
        tint: Vec3,
        alpha: f32,
        emissive: Vec3,
        look: u32,
        rust: f32,
    ) {
        self.queue(rig, root, tint, alpha, emissive, look, rust, |parts| {
            parts
                .extend(rig.parts.iter().enumerate().map(|(i, _)| (root * mats.get(i).copied().unwrap_or(Mat4::IDENTITY)).to_cols_array()));
        });
    }

    /// Picks the level of detail, adds the parts' matrices (`parts` writes them) and queues the
    /// draw and its far cascade's twin.
    #[allow(clippy::too_many_arguments)]
    fn queue(
        &mut self,
        rig: &Rig,
        root: Mat4,
        tint: Vec3,
        alpha: f32,
        emissive: Vec3,
        look: u32,
        rust: f32,
        parts: impl FnOnce(&mut Vec<[f32; 16]>),
    ) {
        let turn = self.turns.entry(rig.id()).or_insert(0);
        let key = (rig.id(), *turn);
        *turn += 1;
        let scale = root.x_axis.truncate().length();
        let px = self.lod.pixels(rig.size * scale, root.w_axis.truncate());
        let level = self.lod.level(px, self.memory.get(&key).filter(|m| m.1 + 1 >= self.frame).map(|m| m.0));
        self.memory.insert(key, (level, self.frame));
        self.levels[level as usize] += 1;
        let tint =
            if self.lod.debug { tint * [Vec3::ONE, Vec3::new(0.5, 1.4, 0.5), Vec3::new(1.5, 0.45, 0.45)][level as usize] } else { tint };
        let shown = rig.lod(level);
        // (See-through, every face shows; but jelly is a skin: only its outside and its inside.)
        let mesh = if alpha < 1.0 && look & rig_look::GOO == 0 { shown.mesh_whole() } else { shown.mesh() };
        let part_base = self.parts.len() as u32;
        parts(&mut self.parts);
        let draw = RigDraw {
            tint: [tint.x, tint.y, tint.z, alpha],
            emissive: [emissive.x, emissive.y, emissive.z, rust],
            part_base,
            look,
            rust_mask: mesh.rust_mask,
            size: shown.size,
        };
        // The far cascade's texels are coarse: a level coarser casts the same shadow there.
        let far = rig.lod((level + 1).min(LOD_LEVELS - 1));
        self.far_draws.push(RigDraw { size: far.size, ..draw });
        self.far_meshes.push(far.mesh());
        self.draws.push(draw);
        self.meshes.push(mesh);
    }

    pub fn clear(&mut self) {
        self.meshes.clear();
        self.draws.clear();
        self.parts.clear();
        self.far_meshes.clear();
        self.far_draws.clear();
        self.levels = [0; 3];
        self.turns.clear();
        self.frame += 1;
        // Forget rigs not drawn for a while.
        if self.frame.is_multiple_of(120) {
            let now = self.frame;
            self.memory.retain(|_, m| m.1 + 120 > now);
        }
    }

    pub fn len(&self) -> usize {
        self.draws.len()
    }

    pub fn is_empty(&self) -> bool {
        self.draws.is_empty()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub pos: Vec3,
    /// Linear color times intensity.
    pub color: Vec3,
    pub distance: f32,
}

/// The sky, fog and ambient light a level is drawn under.
/// `a` mixed `k` of the way to `b`, both sRGB hex.
pub(crate) fn mix_hex(a: u32, b: u32, k: f32) -> u32 {
    let ch = |s: u32| {
        let x = ((a >> s) & 255) as f32;
        let y = ((b >> s) & 255) as f32;
        ((x + (y - x) * k.clamp(0.0, 1.0)) as u32) << s
    };
    ch(16) | ch(8) | ch(0)
}

#[derive(Clone, Copy, Debug)]
pub struct Atmosphere {
    /// Sky and fog color, sRGB hex.
    pub background: u32,
    pub fog_near: f32,
    pub fog_far: f32,
    pub hemi_sky: Vec3,
    pub hemi_ground: Vec3,
    pub ambient: Vec3,
    /// Direction toward the moon or sun, and its light.
    pub sun_dir: Vec3,
    pub sun: Vec3,
    pub exposure: f32,
    /// 1 for the starry night sky with its full moon, 2 for the dusk, 3 for the Portals of Hell's
    /// smoke, 0 for a plain background.
    pub night_sky: f32,
    /// How strongly bright things glow (bloom).
    pub bloom: f32,
    /// Where the moon (or at dusk the setting sun) hangs in the sky.
    pub moon_view: Vec3,
    /// The cloud sea's color.
    pub cloud_tint: Vec3,
}

impl Atmosphere {
    /// The dungeons: a moonlit forest at night.
    /// The same sky under a sin's colour, for its realm and its prince's court.
    pub fn tinted(mut self, hex: u32) -> Self {
        let c = lin(hex);
        let toward = |v: Vec3, k: f32| v.lerp(c * (v.max_element().max(0.02) / c.max_element().max(1e-3)), k);
        self.background = mix_hex(self.background, hex, 0.35);
        self.hemi_sky = toward(self.hemi_sky, 0.5);
        self.hemi_ground = toward(self.hemi_ground, 0.3);
        self.ambient = toward(self.ambient, 0.45);
        self.sun = toward(self.sun, 0.3);
        self.cloud_tint = toward(self.cloud_tint, 0.5);
        self
    }

    pub fn forest() -> Self {
        Self {
            background: BACKGROUND,
            fog_near: 14.0,
            fog_far: 34.0,
            hemi_sky: lin(0x6f8fa8) * 0.85,
            hemi_ground: lin(0x14260f),
            ambient: lin(0x2c3a2a) * 0.7,
            sun_dir: Vec3::new(-6.0, 14.0, 8.0).normalize(),
            sun: lin(MOON) * 1.15,
            exposure: 1.2,
            night_sky: 1.0,
            bloom: 1.0,
            moon_view: Vec3::new(-1.103, 0.534, -0.311).normalize(),
            cloud_tint: lin(0x161b24),
        }
    }
}

impl Default for Atmosphere {
    fn default() -> Self {
        Self::forest()
    }
}

/// A set standing away from the level that is lit by its own probes (RP18), as a level is: the
/// story's sets. The probes are worked out once from its cubes under `atm` (the set's settled
/// light), over `size` units from `origin`, and kept.
#[derive(Clone)]
pub struct SetProbes {
    pub set: InstanceSet,
    pub origin: Vec3,
    pub size: Vec2,
    pub atm: Atmosphere,
}

/// Ground haze (RP19): fog thickest at `floor` and thinning above it, `falloff` units to fall
/// to a third; `density` is how much of the light it takes, per unit, at the floor. It also
/// scatters the nearest lamps' light toward the eye (a glow in the air round them).
#[derive(Clone, Copy, Debug, Default)]
pub struct Haze {
    pub density: f32,
    pub floor: f32,
    pub falloff: f32,
}

#[derive(Default)]
pub struct Frame {
    pub opaque: [Vec<Instance>; MESH_KINDS],
    /// Whole rigs drawn from their resident meshes: solid, and see-through (drawn before other see-through things).
    pub rigs: RigList,
    pub rigs_blend: RigList,
    /// Prebuilt cubes the GPU keeps: solid ones, and see-through ones drawn where see-through cubes go.
    pub sets: Vec<InstanceSet>,
    pub sets_blend: Vec<InstanceSet>,
    /// See-through cubes drawn before every other see-through thing (the smoke wall far behind it all).
    pub transparent_early: Vec<Instance>,
    pub transparent: [Vec<Instance>; MESH_KINDS],
    pub additive: Vec<Instance>,
    /// Soft glowing orbs, drawn additively on discs facing the camera (`SHADE_GLOW`): halos round lamps, wisps.
    pub glow: Vec<Instance>,
    /// Turns a disc in the XY plane to face the camera.
    pub facing: Quat,
    pub lights: Vec<Light>,
    pub coin_pulse: f32,
    /// Seconds since start, for the twinkling stars.
    pub time: f32,
    /// The level's footprint (min x, min z, max x, max z), for the cloud sea around it.
    pub bounds: Vec4,
    /// The title diorama draws on its own; the level's static geometry sits this frame out.
    pub skip_static: bool,
    /// The first light is the hero's own lantern (which lights the scene but not the haze).
    pub lantern: bool,
    pub atmosphere: Atmosphere,
    pub haze: Haze,
    /// With `skip_static`: the set on show lit by its own probes, and others to have ready (sent
    /// up and their probes worked out) before they are shown.
    pub probes: Option<SetProbes>,
    pub probes_warm: Vec<SetProbes>,
}

impl Frame {
    pub fn clear(&mut self) {
        for v in self.opaque.iter_mut().chain(self.transparent.iter_mut()) {
            v.clear();
        }
        self.transparent_early.clear();
        self.rigs.clear();
        self.rigs_blend.clear();
        self.sets.clear();
        self.sets_blend.clear();
        self.haze = Haze::default();
        self.probes = None;
        self.probes_warm.clear();
        self.additive.clear();
        self.glow.clear();
        self.lights.clear();
        self.lantern = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_follow_the_pixels_a_voxel_covers_with_a_margin_to_come_back() {
        let lod = LodPolicy::default();
        assert_eq!(lod.level(5.0, None), 0);
        assert_eq!(lod.level(1.5, None), 1);
        assert_eq!(lod.level(0.7, None), 2);
        // Dropping is at once; coming back wants 20% more than the threshold.
        assert_eq!(lod.level(1.9, Some(0)), 1);
        assert_eq!(lod.level(2.1, Some(1)), 1, "just over: stays coarse");
        assert_eq!(lod.level(2.5, Some(1)), 0);
        assert_eq!(lod.level(1.1, Some(2)), 2);
        assert_eq!(lod.level(3.0, Some(2)), 0, "far over both: straight back to whole");
        // A higher bias drops sooner; none keeps every rig whole.
        assert_eq!(LodPolicy { bias: 1.5, ..lod }.level(2.5, None), 1);
        assert_eq!(LodPolicy { bias: 0.0, ..lod }.level(0.1, None), 0);
    }

    #[test]
    fn a_voxel_covers_fewer_pixels_further_off_but_not_under_the_map_camera() {
        let lod = LodPolicy { px_per_unit: 1000.0, ..LodPolicy::default() };
        let near = lod.pixels(0.01, Vec3::new(0.0, 0.0, 2.0));
        let far = lod.pixels(0.01, Vec3::new(0.0, 0.0, 8.0));
        assert!((near - 5.0).abs() < 1e-4 && (far - 1.25).abs() < 1e-4);
        let map = LodPolicy { ortho: true, px_per_unit: 50.0, ..lod };
        assert_eq!(map.pixels(0.01, Vec3::new(0.0, 0.0, 2.0)), map.pixels(0.01, Vec3::new(0.0, 0.0, 80.0)));
    }

    #[test]
    fn a_rig_without_parents_is_posed_exactly_as_before() {
        // A foe's rig in a pose with every part turned and moved: each part's matrix is the root,
        // then its joint, then its turn, to the bit (what the GPU drew before parts had parents).
        let rig = crate::render::voxels::build_rig(&crate::render::foes::draugr(), crate::render::foes::DRAUGR_RIG, 0.0125);
        assert!(rig.parts.iter().all(|p| p.parent.is_none()));
        let mut pose = super::super::Pose::rest(&rig);
        for (i, (p, r)) in pose.pos.iter_mut().zip(pose.rot.iter_mut()).enumerate() {
            *p += Vec3::new(0.01 * i as f32, -0.02, 0.03);
            *r = Vec3::new(0.3 * i as f32, -0.2, 0.11 * i as f32);
        }
        let root =
            Mat4::from_translation(Vec3::new(3.0, 0.5, -2.0)) * Mat4::from_rotation_y(0.7) * Mat4::from_scale(Vec3::new(1.1, 0.9, 1.1));
        let mut list = RigList::default();
        list.push(&rig, &pose, root, Vec3::ONE, 1.0, Vec3::ZERO, 0, 0.0);
        for i in 0..rig.parts.len() {
            let before = (root * Mat4::from_translation(pose.pos[i]) * euler(pose.rot[i])).to_cols_array();
            assert_eq!(list.parts[i], before, "part {i}");
        }
    }

    #[test]
    fn a_part_with_a_parent_turns_with_it() {
        let part = |rest: Vec3, parent| crate::render::voxels::RigPart {
            name: "p",
            rest,
            cubes: vec![(Vec3::ZERO, Vec3::ONE)],
            shade: vec![0.0],
            min_y: 0.0,
            parent,
        };
        let rig = Rig::new(0.1, vec![part(Vec3::new(0.0, 1.0, 0.0), None), part(Vec3::new(0.0, 0.5, 0.0), Some(0))]);
        let mut pose = super::super::Pose::rest(&rig);
        // The parent turned a quarter about z: the child, half a unit below its joint, swings out to +x...
        pose.rot[0] = Vec3::new(0.0, 0.0, std::f32::consts::FRAC_PI_2);
        let mut list = RigList::default();
        list.push(&rig, &pose, Mat4::IDENTITY, Vec3::ONE, 1.0, Vec3::ZERO, 0, 0.0);
        let child = Mat4::from_cols_array(&list.parts[1]);
        let at = child.transform_point3(Vec3::ZERO);
        assert!(at.abs_diff_eq(Vec3::new(0.5, 1.0, 0.0), 1e-5), "{at}");
        // ...and turned with it.
        assert!(child.transform_vector3(Vec3::X).abs_diff_eq(Vec3::Y, 1e-5));
    }
}
