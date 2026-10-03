//! The Look Dev room (TA01 in the Technical Art and Rendering Plan): a small set away from
//! every level where materials and light are judged without a level in the way.
//!
//! Against a mid-grey back wall on a charcoal floor stand, front to back:
//! - a value ramp of eleven greys, black to white, for exposure and crushed shadows;
//! - every voxel material on a stepped block at a neutral mid grey (so only the material
//!   differs), then the same at the colour it usually wears;
//! - the six colours of the shared palette, and the seven realms' proposed surface accents,
//!   each capped with a glowing tile of that realm's identity colour;
//! - Ragnar and a draugr, for scale and for skin, fur, metal and bone under the same light.
//!
//! Three lights: `Neutral` (a white sun and an even grey sky, no fog: the material chart),
//! `Dungeon` (the moonlit night of the dungeons with two torches) and `Realm(n)` (the night
//! under sin n's colour, as its prince's court is lit).
//! `--capture out.png --lookdev neutral|dungeon|realm:N` frames it.

use super::*;

/// Where the room stands: far from every level and set.
pub const LOOKDEV: Vec3 = Vec3::new(-120.0, 0.0, 90.0);

/// A swatch block is this many voxels a side.
const SWATCH: i32 = 8;
const SWATCH_VOXEL: f32 = 0.06;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LookLight {
    Neutral,
    Dungeon,
    Realm(usize),
}

impl LookLight {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "neutral" => Some(Self::Neutral),
            "dungeon" => Some(Self::Dungeon),
            _ => s.strip_prefix("realm:").and_then(|n| n.parse().ok()).map(|n: usize| Self::Realm(n.min(6))),
        }
    }
}

pub struct LookDev {
    pub light: LookLight,
    set: InstanceSet,
    hero: Rig,
}

/// The voxel materials as the shader numbers them (shade mode 5 + n; 0 is plain stone), each
/// with the colour it usually wears.
const MATERIALS: [(&str, f32, u32); 11] = [
    ("stone", 0.0, 0x6e6a60),
    ("metal", 6.0, 0x7a808a),
    ("fur", 7.0, 0x7a5a3a),
    ("cloth", 8.0, 0x8c5835),
    ("leather", 9.0, 0x49392c),
    ("skin", 10.0, 0xe0a878),
    ("ivory", 11.0, 0xe8dcc0),
    ("wood", 12.0, 0x6b3f1d),
    ("gold", 13.0, 0xd8a040),
    ("slime", 14.0, 0x46d23c),
    ("glow", 16.0, 0xc49358),
];

/// The plan's shared palette (sRGB).
const PALETTE: [u32; 6] = [0x192321, 0x49392c, 0x8c5835, 0xc49358, 0x567770, 0xb6b39a];

/// The proposed muted surface accent for each realm, in `SINS` order.
const ACCENTS: [u32; 7] = [0x8873b3, 0x668c62, 0xad5145, 0x75899d, 0xb79b4e, 0xb97742, 0xac657e];

/// A stepped block of `SWATCH` voxels a side at `at` (its base centre): its top front edge is cut
/// back in steps, so it shows the light on its top, its front and the steps between.
fn swatch(out: &mut Vec<Instance>, at: Vec3, color: Vec3, shade: f32, emissive: Vec3) {
    let s = SWATCH_VOXEL;
    let half = (SWATCH as f32 - 1.0) / 2.0;
    for x in 0..SWATCH {
        for y in 0..SWATCH {
            for z in 0..SWATCH {
                // The steps: the front rows (toward the camera, +z) lose their top voxels.
                let cut = (z - (SWATCH - 4)).max(0);
                if y >= SWATCH - cut {
                    continue;
                }
                // Hollow inside: only the shell can be seen.
                let inner = x > 0 && x < SWATCH - 1 && z > 0 && z < SWATCH - 1 && y > 0 && y < SWATCH - cut - 1;
                if inner {
                    continue;
                }
                let p = at + Vec3::new((x as f32 - half) * s, (y as f32 + 0.5) * s, (z as f32 - half) * s);
                out.push(Instance::new(
                    Mat4::from_scale_rotation_translation(Vec3::splat(s), Quat::IDENTITY, p),
                    color,
                    1.0,
                    emissive,
                    shade,
                ));
            }
        }
    }
}

fn slab(out: &mut Vec<Instance>, center: Vec3, size: Vec3, color: Vec3) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(size, Quat::IDENTITY, center), color, 1.0, Vec3::ZERO, SHADE_LIT));
}

fn build_set() -> Vec<Instance> {
    let mut out = Vec::new();
    let o = LOOKDEV;
    // A darker grey floor and mid-grey walls, all plain stone (neutral, so they tint nothing).
    slab(&mut out, o + Vec3::new(0.0, -0.1, 0.0), Vec3::new(9.0, 0.2, 6.0), lin(0x4a4a4a));
    let grey = lin(0x767676);
    slab(&mut out, o + Vec3::new(0.0, 1.5, -3.1), Vec3::new(9.0, 3.0, 0.2), grey);
    slab(&mut out, o + Vec3::new(-4.6, 1.5, -0.5), Vec3::new(0.2, 3.0, 5.4), grey);
    slab(&mut out, o + Vec3::new(4.6, 1.5, -0.5), Vec3::new(0.2, 3.0, 5.4), grey);

    // The value ramp along the front: 0 to 255 in eleven steps.
    for i in 0..11 {
        let v = ((i as f32 / 10.0) * 255.0).round() as u32;
        let hex = (v << 16) | (v << 8) | v;
        slab(&mut out, o + Vec3::new(-3.0 + i as f32 * 0.6, 0.02, 2.3), Vec3::new(0.56, 0.04, 0.5), lin(hex));
    }
    // The materials: neutral grey, then their own colours.
    for (i, (_, shade, hex)) in MATERIALS.iter().enumerate() {
        let x = -3.75 + i as f32 * 0.75;
        let glow = if *shade == 16.0 { lin(*hex) * 0.6 } else { Vec3::ZERO };
        swatch(&mut out, o + Vec3::new(x, 0.0, 1.2), lin(0x808080), *shade, glow);
        swatch(&mut out, o + Vec3::new(x, 0.0, 0.2), lin(*hex), *shade, glow);
    }
    // The palette, then the realms' accents with their identity colours glowing on top.
    for (i, hex) in PALETTE.iter().enumerate() {
        swatch(&mut out, o + Vec3::new(-3.75 + i as f32 * 0.75, 0.0, -0.9), lin(*hex), SHADE_LIT, Vec3::ZERO);
    }
    for (i, hex) in ACCENTS.iter().enumerate() {
        let at = o + Vec3::new(-2.25 + i as f32 * 0.75, 0.0, -1.9);
        swatch(&mut out, at, lin(*hex), SHADE_LIT, Vec3::ZERO);
        let id = lin(SINS[i].color());
        slab(&mut out, at + Vec3::new(0.0, SWATCH as f32 * SWATCH_VOXEL + 0.03, -0.12), Vec3::new(0.3, 0.06, 0.16), id * 0.3);
        if let Some(cap) = out.last_mut() {
            cap.emissive = [id.x * 1.5, id.y * 1.5, id.z * 1.5, 16.0];
        }
    }
    out
}

impl LookDev {
    pub fn new(light: LookLight) -> Self {
        Self { light, set: InstanceSet::new(build_set()), hero: forge::hero_model(HeroKind::Ragnar, 3, Some(3)) }
    }
}

impl WorldView {
    /// Opens the Look Dev room under `light`, the camera on the chart.
    pub fn open_lookdev(&mut self, light: LookLight) {
        match &mut self.lookdev {
            Some(l) => l.light = light,
            None => self.lookdev = Some(LookDev::new(light)),
        }
        self.set_closeup_rise(LOOKDEV + Vec3::new(0.0, 0.35, 0.4), 0.0, 5.8, 0.36);
    }

    pub(super) fn emit_lookdev(&self, s: &SceneState, f: &mut Frame) {
        let Some(l) = &self.lookdev else { return };
        let o = LOOKDEV;
        f.skip_static = true;
        f.bounds = Vec4::new(o.x - 4.5, o.z - 3.0, o.x + 4.5, o.z + 3.0);
        f.sets.push(l.set.clone());
        let torch = |f: &mut Frame, x: f32| {
            let at = o + Vec3::new(x, 1.5, -2.2);
            f.lights.push(Light { pos: at, color: lin(0xffa860) * 3.5, distance: 6.5 });
            glow_orb(f, at, 0.3, lin(0xffb070), 0.8);
        };
        f.atmosphere = match l.light {
            LookLight::Neutral => Atmosphere {
                background: 0x2a2a2a,
                // Out past the room: no fog on the chart (within the camera's far plane, which caps it).
                fog_near: 40.0,
                fog_far: 60.0,
                hemi_sky: Vec3::splat(0.4),
                hemi_ground: Vec3::splat(0.15),
                ambient: Vec3::splat(0.06),
                sun_dir: Vec3::new(-0.45, 0.8, 0.55).normalize(),
                sun: Vec3::splat(1.7),
                exposure: 1.0,
                night_sky: 0.0,
                bloom: 0.6,
                moon_view: Vec3::new(-1.0, 0.5, -0.3).normalize(),
                cloud_tint: Vec3::splat(0.05),
            },
            LookLight::Dungeon => {
                torch(f, -3.6);
                torch(f, 3.6);
                Atmosphere::forest()
            }
            LookLight::Realm(n) => {
                torch(f, -3.6);
                torch(f, 3.6);
                Atmosphere::forest().tinted(SINS[n].color())
            }
        };
        // The night lights only what the moon and the torches reach; keep the sky out of the room.
        f.atmosphere.night_sky = 0.0;
        f.atmosphere.fog_near = f.atmosphere.fog_near.max(30.0);
        f.atmosphere.fog_far = f.atmosphere.fog_far.max(60.0);
        let hero = &l.hero;
        let root = Mat4::from_translation(o + Vec3::new(3.4, 0.0, 1.4)) * Mat4::from_rotation_y(-0.5);
        emit_rig(&mut f.rigs, hero, &Pose::rest(hero), root, Vec3::ONE, 1.0, Vec3::ZERO);
        let draugr = self.models.rig(RigId::Skeleton);
        let root = Mat4::from_translation(o + Vec3::new(-3.4, 0.0, 1.4)) * Mat4::from_rotation_y(0.5);
        emit_rig(&mut f.rigs, draugr, &Pose::rest(draugr), root, Vec3::ONE, 1.0, Vec3::ZERO);
        f.time = s.time as f32;
    }
}
