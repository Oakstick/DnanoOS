//! The boss arena's look, and the boss. The Cyclops' meadow at dusk round the ruins of an old
//! estate (`meadow_field` builds it): turf and wildflowers, trodden earth, a stream under old
//! timber bridges, dressed-stone pillars and low walls, a ruined boundary wall and the treeline.
//! Then the Cyclops, its bolts and boulders, and the shockwaves of its slams.

use super::*;
use crate::logic::boss::{BossShot, BossState, Cyclops, ShotKind};
use crate::logic::levels::{ARENA, STREAM_ROW};

mod pal {
    /// Moths and the last butterflies at dusk: pale stone, amber, rust.
    pub const BUTTERFLIES: [u32; 3] = [0xd8d0b8, 0xc89a3a, 0x9a5a3a];
    /// Fireflies over the water: the board's amber.
    pub const FIREFLY: u32 = 0xf0c070;
    pub const BOLT: u32 = 0xff7a2a;
    pub const BOLT_CORE: u32 = 0xfff1c0;
    /// The slams' dust: trodden earth thrown up.
    pub const DUST: u32 = 0xb8a888;
}
use crate::render::characters::Sculpt;
use crate::render::foes::CYCLOPS_EYE;
use crate::render::foes::CYCLOPS_VOXEL;
use crate::render::voxels::Collected;
use pal::*;

/// Dusk over the meadow, in the Cisterns' key: the sun low in the west-north-west and amber, its
/// long shadows across the field; the sky's light a weak mist teal, the ambient low, so the
/// braziers' and lanterns' pools carry the shade and the teal is left to the shadows and the mist.
pub fn meadow_atmosphere() -> Atmosphere {
    Atmosphere {
        background: 0x3a4442,
        fog_near: 16.0,
        fog_far: 46.0,
        hemi_sky: lin(0x6e8a86) * 0.32,
        hemi_ground: lin(0x3a3424) * 0.3,
        ambient: lin(0x2e3634) * 0.5,
        sun_dir: Vec3::new(-0.78, 0.36, -0.5).normalize(),
        sun: lin(0xffa860) * 1.55,
        exposure: 1.2,
        night_sky: 2.0,
        bloom: 0.95,
        moon_view: Vec3::new(-0.42, 0.2, -0.9).normalize(),
        cloud_tint: lin(0x2e2a26),
    }
}

/// What a sin's colour does to a hide: most of the way to it, kept bright.
fn sin_skin(hex: u32) -> Vec3 {
    let c = lin(hex);
    Vec3::ONE.lerp(c / c.max_element().max(1e-3), 0.75)
}

impl WorldView {
    /// The meadow's static geometry: the field, the ruins, the stream and the treeline
    /// (`meadow_field`). The stream's water and its plan come from `logic::garden::stream_plan`.
    pub(super) fn build_meadow(&mut self, grid: &Grid) {
        self.water.clear();
        self.build_meadow_field(grid);
    }

    /// A green cross that appears mid-fight.
    pub fn add_charge(&mut self, grid: &Grid, x: i32, y: i32) {
        self.charges.push((P::new(x, y), Vec3::new(x as f32, floor_at(grid, x as f32, y as f32) + 0.3, y as f32)));
    }

    /// A ring of dust racing out across the ground from a slam.
    pub fn shockwave(&mut self, grid: &Grid, x: f64, y: f64) {
        let p = Vec3::new(x as f32, floor_at(grid, x as f32, y as f32) + 0.04, y as f32);
        self.rings.push((p, 0.0));
    }

    pub(super) fn update_meadow(&mut self, dt: f32) {
        self.rings.retain_mut(|(_, t)| {
            *t += dt;
            *t < RING_SECONDS
        });
    }

    /// The meadow's moving parts: the spring's fall, moths and the last butterflies, fireflies
    /// over the stream, dust in the low sun, and the slams' rings.
    pub(super) fn emit_meadow(&self, time: f32, f: &mut Frame) {
        let cube_k = MeshKind::Cube as usize;
        for (i, &(mouth, out, foot)) in self.spouts.iter().enumerate() {
            super::crypt::emit_fall(f, mouth, out, foot, time, i as f32);
        }
        // Butterflies and moths wandering over the grass, fewer at dusk.
        let (x0, y0, x1, y1) = ARENA;
        for i in 0..10 {
            let fi = i as f32;
            let bx = x0 as f32 + hash(fi as f64, 3.0) as f32 * (x1 - x0) as f32;
            let bz = y0 as f32 + hash(fi as f64, 7.0) as f32 * (y1 - y0) as f32;
            let path = |t: f32| {
                Vec3::new(bx + (t * 0.3 + fi).sin() * 2.2, 0.45 + (t * 1.3 + fi).sin() * 0.2, bz + (t * 0.23 + fi * 1.7).cos() * 2.2)
            };
            let p = path(time);
            let ahead = path(time + 0.05) - p;
            let heading = ahead.x.atan2(ahead.z);
            let flap = (time * 17.0 + fi * 2.0).sin() * 0.9;
            let color = lin(BUTTERFLIES[i % BUTTERFLIES.len()]);
            for side in [-1.0f32, 1.0] {
                let m = Mat4::from_translation(p)
                    * Mat4::from_rotation_y(heading)
                    * Mat4::from_rotation_z(side * flap)
                    * Mat4::from_translation(Vec3::new(side * 0.04, 0.0, 0.0))
                    * Mat4::from_scale(Vec3::new(0.07, 0.006, 0.06));
                f.opaque[cube_k].push(Instance::new(m, color, 1.0, Vec3::ZERO, crypt::SHADE_CLOTH));
            }
            let body = Mat4::from_translation(p) * Mat4::from_rotation_y(heading) * Mat4::from_scale(Vec3::new(0.015, 0.015, 0.06));
            f.opaque[cube_k].push(Instance::new(body, lin(0x2a2018), 1.0, Vec3::ZERO, SHADE_LIT));
        }
        // Fireflies coming out over the stream and along the treeline.
        let (w, h) = (self.level_dims.x, self.level_dims.y);
        for i in 0..28 {
            let fi = i as f32;
            let k = hash(fi as f64 * 1.3, 5.0) as f32;
            let home = if i < 16 {
                Vec3::new(x0 as f32 + k * (x1 - x0) as f32, 0.35, STREAM_ROW as f32 + (hash(fi as f64, 9.0) as f32 - 0.5) * 1.6)
            } else {
                let t = k * 2.0 * (w + h);
                let (x, z) = if t < w {
                    (t, 1.0)
                } else if t < 2.0 * w {
                    (t - w, h - 2.0)
                } else if t < 2.0 * w + h {
                    (1.0, t - 2.0 * w)
                } else {
                    (w - 2.0, t - 2.0 * w - h)
                };
                Vec3::new(x, 0.6, z)
            };
            let p =
                home + Vec3::new((time * 0.5 + fi).sin() * 0.5, (time * 0.9 + fi * 0.7).sin() * 0.18, (time * 0.4 + fi * 1.3).cos() * 0.5);
            let phase = (time * (0.35 + (fi * 0.618).fract() * 0.3) + fi * 0.37).fract();
            let flash = (1.0 - (phase * 3.0 - 0.5).abs() * 2.0).max(0.0);
            glow_orb(f, p, 0.035 + 0.015 * flash, lin(FIREFLY) * 0.8, 0.3 + 0.7 * flash * flash);
            if flash > 0.3 {
                glow_orb(f, p, 0.14 + 0.08 * flash, lin(FIREFLY) * 0.8, 0.25 * flash);
            }
        }
        // Dust in the low sun.
        for (i, b) in self.mote_base.iter().enumerate().take(50) {
            let fi = i as f32;
            let p = Vec3::new(
                b.x + (time * 0.3 + fi).sin() * 0.5,
                b.y * 0.8 + (time * 0.5 + fi * 0.7).sin() * 0.2,
                b.z + (time * 0.25 + fi * 1.3).cos() * 0.5,
            );
            let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.03));
            f.additive.push(Instance::new(m, lin(0xffd8a0), 0.18 + (time * 1.5 + fi).sin() * 0.1, Vec3::ZERO, SHADE_UNLIT));
        }
        for &(p, t) in &self.rings {
            let k = t / RING_SECONDS;
            let m = Mat4::from_translation(p) * Mat4::from_rotation_x(-PI / 2.0) * Mat4::from_scale(Vec3::splat(0.4 + k * 3.4));
            f.transparent[MeshKind::Ring as usize].push(Instance::new(m, lin(DUST), 0.7 * (1.0 - k), Vec3::ZERO, SHADE_UNLIT));
        }
    }

    /// The Cyclops: bobbing, flapping, looking at him, blinking; glowing when it charges.
    pub(super) fn emit_boss(&self, b: &Cyclops, hero: Vec3, time: f32, f: &mut Frame) {
        if matches!(b.state, BossState::Hidden | BossState::Dead) {
            return;
        }
        let rig = &self.models.cyclops;
        let dazed = b.dazed();
        let dying = b.dying_progress() as f32;
        let bob = if dazed { 0.0 } else { (time * 2.1).sin() * 0.07 };
        let mut pos = Vec3::new(b.pos.x as f32, b.hover as f32 + bob, b.pos.y as f32);
        if dying > 0.0 {
            let j = 0.05 + dying * 0.1;
            pos += Vec3::new((time * 71.0).sin() * j, (time * 53.0).cos() * j * 0.5, (time * 67.0).cos() * j);
        }
        let yaw = PI / 2.0 - b.look as f32;
        let tilt = if dazed { (time * 3.0).sin() * 0.25 } else { 0.0 };
        let swell = 1.0 + dying * 0.25 + if dying > 0.0 { (time * 30.0).sin() * 0.04 * dying } else { 0.0 };
        let root =
            Mat4::from_translation(pos) * Mat4::from_rotation_y(yaw) * Mat4::from_rotation_z(tilt) * Mat4::from_scale(Vec3::splat(swell));

        let mut pose = Pose::rest(rig);
        let beat = if dazed { 3.0 } else { 9.0 };
        let flap = (time * beat).sin() * 0.55 + 0.15;
        if let Some(i) = rig.index("leftWing") {
            pose.rot[i].z = flap;
        }
        if let Some(i) = rig.index("rightWing") {
            pose.rot[i].z = -flap;
        }
        for k in 0..4 {
            if let Some(i) = rig.index(&format!("tentacle{k}")) {
                let kf = k as f32;
                pose.rot[i].x = (time * 3.1 + kf * 1.3).sin() * 0.35;
                pose.rot[i].z = (time * 2.3 + kf * 2.1).cos() * 0.25;
            }
        }
        // The iris follows him within its socket; dazed, it rolls.
        let to_hero = hero - pos;
        let want = to_hero.x.atan2(to_hero.z) - yaw;
        let want = (want + PI).rem_euclid(TAU) - PI;
        if let Some(i) = rig.index("iris") {
            if dazed {
                pose.rot[i].y = (time * 5.0).sin() * 0.3;
                pose.rot[i].x = (time * 5.0).cos() * 0.25;
            } else {
                let flat = Vec2::new(to_hero.x, to_hero.z).length().max(0.5);
                pose.rot[i].y = want.clamp(-0.4, 0.4);
                pose.rot[i].x = ((-to_hero.y + 0.6) / flat).atan().clamp(-0.1, 0.4);
            }
        }
        // Blinks now and then; squints to charge; shuts as it dies.
        let blink_t = (time * 0.31 + 0.2).fract();
        let blink = if blink_t < 0.04 { (blink_t / 0.04 * PI).sin() } else { 0.0 };
        let shut = (b.squint() as f32).max(blink);
        if let Some(i) = rig.index("lid") {
            pose.rot[i].x = 1.2 * shut;
        }

        let flash = b.flash as f32 * if self.reduce_flashing { 0.35 } else { 1.0 };
        let glow = b.glow() as f32;
        let rage = if b.phase() >= 3 { 0.1 + (time * 6.0).sin().abs() * 0.1 } else { 0.0 };
        let body_emissive = Vec3::splat(flash * 0.5)
            + lin(0xff2a2a) * rage
            + Vec3::splat(if dying > 0.0 && (self.reduce_flashing || (time * 14.0).sin() > 0.3) {
                if self.reduce_flashing {
                    0.25 * dying
                } else {
                    0.6 * dying
                }
            } else {
                0.0
            });
        let tint = if flash > 0.0 { Vec3::ONE.lerp(Vec3::new(1.0, 0.55, 0.55), flash) } else { Vec3::ONE };
        // A prince: the Cyclops' hide in his sin's colour.
        let tint = tint * self.tint.map(sin_skin).unwrap_or(Vec3::ONE);
        // The iris smoulders even at rest, so the eye reads with the light behind it.
        let iris_emissive = lin(0xffb21e) * 0.04 + lin(BOLT) * glow * 0.9 + body_emissive;
        let out = &mut f.opaque[MeshKind::Cube as usize];
        for (i, part) in rig.parts.iter().enumerate() {
            let em = if part.name == "iris" { iris_emissive } else { body_emissive };
            let pm = root * Mat4::from_translation(pose.pos[i]) * euler(pose.rot[i]);
            for (j, (off, c)) in part.cubes.iter().enumerate() {
                let shade = part.shade.get(j).copied().filter(|&m| m > 0.0).unwrap_or(SHADE_LIT);
                out.push(Instance::new(voxel_matrix(&pm, rig.size, *off), *c * tint, 1.0, em, shade));
            }
        }
        // Stars circling a dazed head.
        if dazed {
            for k in 0..5 {
                let a = time * 4.0 + k as f32 * TAU / 5.0;
                let p = pos + Vec3::new(a.cos() * 1.0, 2.75 + (time * 6.0 + k as f32).sin() * 0.1, a.sin() * 1.0);
                let m = Mat4::from_translation(p)
                    * Mat4::from_rotation_y(a * 2.0)
                    * Mat4::from_rotation_z(0.785)
                    * Mat4::from_scale(Vec3::splat(0.2));
                f.opaque[MeshKind::Cube as usize].push(Instance::new(m, lin(0xffd23a), 1.0, lin(0xffc020) * 0.8, SHADE_LIT));
            }
        }
        // Its shadow, smaller and fainter the higher it flies.
        let lift = (b.hover as f32).max(0.0);
        let k = (1.0 - lift / 8.0).clamp(0.25, 1.0);
        let m = Mat4::from_translation(Vec3::new(pos.x, 0.02, pos.z))
            * Mat4::from_rotation_x(-PI / 2.0)
            * Mat4::from_scale(Vec3::new(1.05 * k, 1.05 * k, 1.0));
        f.transparent[MeshKind::Disc as usize].push(Instance::new(m, Vec3::ZERO, 0.4 * k, Vec3::ZERO, SHADE_UNLIT));
        // The eye lights the grass when it charges.
        let eye = root.transform_point3(Vec3::new(0.0, CYCLOPS_EYE_Y, 2.4));
        if glow > 0.0 {
            f.lights.push(Light { pos: eye, color: lin(BOLT) * (1.0 + glow * 5.0), distance: 5.5 });
        }
        if flash > 0.0 {
            f.lights.push(Light { pos: eye, color: Vec3::ONE * flash * 3.0, distance: 4.0 });
        }
    }

    /// Bolts glow and trail; boulders roll.
    pub(super) fn emit_boss_shots(&self, shots: &[BossShot], hero: Vec3, time: f32, f: &mut Frame) {
        let mut lit: Vec<(f32, Vec3)> = Vec::new();
        for s in shots {
            let p = Vec3::new(s.pos.x as f32, 0.0, s.pos.y as f32);
            let v = Vec3::new(s.vel.x as f32, 0.0, s.vel.y as f32);
            match s.kind {
                ShotKind::Bolt => {
                    let c = p + Vec3::new(0.0, 0.72, 0.0);
                    let pulse = 1.0 + (time * 30.0 + p.x * 3.0).sin() * 0.12;
                    let core = Mat4::from_translation(c) * Mat4::from_scale(Vec3::splat(0.13 * pulse));
                    f.transparent[MeshKind::Sphere as usize].push(Instance::new(core, lin(BOLT_CORE), 1.0, Vec3::ZERO, SHADE_UNLIT));
                    let shell = Mat4::from_translation(c) * Mat4::from_scale(Vec3::splat(0.24 * pulse));
                    f.additive.push(Instance::new(shell, lin(BOLT) * 1.3, 0.8, Vec3::ZERO, SHADE_UNLIT));
                    let back = v.normalize_or_zero();
                    for k in 1..4 {
                        let t = Mat4::from_translation(c - back * (k as f32 * 0.14)) * Mat4::from_scale(Vec3::splat(0.2 - k as f32 * 0.04));
                        f.additive.push(Instance::new(t, lin(BOLT), 0.6 - k as f32 * 0.15, Vec3::ZERO, SHADE_UNLIT));
                    }
                    lit.push((c.distance_squared(hero), c));
                }
                ShotKind::Boulder => {
                    let r = s.kind.radius() as f32;
                    let axis = Vec3::new(v.z, 0.0, -v.x).normalize_or_zero();
                    let spin = if axis == Vec3::ZERO { Quat::IDENTITY } else { Quat::from_axis_angle(axis, -(s.rolled as f32) / r) };
                    let root = Mat4::from_translation(p + Vec3::new(0.0, r, 0.0))
                        * Mat4::from_quat(spin)
                        * Mat4::from_translation(Vec3::new(0.0, -r, 0.0));
                    emit_rig(&mut f.rigs, &self.models.boulder, &Pose::rest(&self.models.boulder), root, Vec3::ONE, 1.0, Vec3::ZERO);
                    let m = Mat4::from_translation(p + Vec3::new(0.0, 0.02, 0.0))
                        * Mat4::from_rotation_x(-PI / 2.0)
                        * Mat4::from_scale(Vec3::new(0.42, 0.42, 1.0));
                    f.transparent[MeshKind::Disc as usize].push(Instance::new(m, Vec3::ZERO, 0.35, Vec3::ZERO, SHADE_UNLIT));
                }
            }
        }
        // Only the bolts nearest him get a real light; the rest just glow.
        lit.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        for (_, c) in lit.into_iter().take(6) {
            f.lights.push(Light { pos: c, color: lin(BOLT) * 5.0, distance: 3.2 });
        }
    }
}

/// Seconds a slam's dust ring takes to spread and fade.
const RING_SECONDS: f32 = 0.55;
/// Height of the eye's center above the model root, world units.
const CYCLOPS_EYE_Y: f32 = CYCLOPS_EYE[1] * CYCLOPS_VOXEL;

/// A rough grey boulder for the Cyclops to throw.
pub fn boulder() -> Collected {
    use Sculpt;
    let mut s = Sculpt::new(9, 9, 9);
    s.ellipsoid((4.0, 4.0, 4.0), (4.3, 3.9, 4.1), 0, 0x9a948a);
    s.paint(|x, y, z, _| {
        let n = hash(x as f64 * 1.7 + z as f64, y as f64 * 2.3) as f32;
        Some(if n < 0.25 {
            0x7a766e
        } else if n > 0.85 {
            0xb8b2a6
        } else if n > 0.78 {
            0x6f8f3a
        } else {
            0x9a948a
        })
    });
    s.finish(0.08)
}
