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
        Self {
            m: m.to_cols_array(),
            color: [color.x, color.y, color.z, alpha],
            emissive: [emissive.x, emissive.y, emissive.z, shade],
        }
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
    /// 1 for the starry night sky with its full moon, 0 for a plain background.
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

#[derive(Default)]
pub struct Frame {
    pub opaque: [Vec<Instance>; MESH_KINDS],
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
    pub atmosphere: Atmosphere,
}

impl Frame {
    pub fn clear(&mut self) {
        for v in self.opaque.iter_mut().chain(self.transparent.iter_mut()) {
            v.clear();
        }
        self.transparent_early.clear();
        self.additive.clear();
        self.glow.clear();
        self.lights.clear();
    }
}
