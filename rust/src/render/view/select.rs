//! The character screen's stage: Mehmed, Ragnar and Miyamoto side by side on
//! the cracked flagstones of a cavern, lava pouring down its walls on either
//! side and blue crystals glowing among the rocks, embers drifting up. The one
//! you have picked steps forward into a warm light; those not yet unlocked
//! stand grey and dim.

use super::*;
use crate::logic::heroes::{HeroKind, ROSTER};

/// Where the cavern stands, well away from the level, the title stage and the forge.
pub(super) const SELECT_STAGE: Vec3 = Vec3::new(-120.0, 0.0, -40.0);
/// Across the line-up, left to right.
const SLOTS_X: [f32; 3] = [-1.32, 0.0, 1.32];

mod pal {
    // The art direction's palette: charcoal and umber stone, amber lava, crystals of mist teal.
    pub const STONE: [u32; 3] = [0x4a5250, 0x3f4745, 0x5e4c40];
    pub const ROCK: [u32; 4] = [0x2c3230, 0x363c3a, 0x242a28, 0x40342c];
    pub const LAVA: u32 = 0xf09038;
    pub const LAVA_HOT: u32 = 0xffd070;
    pub const CRUST: u32 = 0x1a120c;
    pub const CRYSTAL: u32 = 0x8ad0c4;
    pub const CRYSTAL_D: u32 = 0x4e8a80;
    pub const IRON: u32 = 0x24221f;
    pub const ROPE: u32 = 0x6a5236;
    pub const PLANK: u32 = 0x4e3522;
}
use pal::*;

/// What the character screen is showing: the highlighted place in the line-up, seconds
/// since it moved, and seconds left of the shake after a locked hero was picked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectView {
    pub index: usize,
    pub age: f32,
    pub denied: f32,
}

/// Where hero `i` of the line-up stands (his feet), stepped forward by `step` (0..1).
pub(super) fn slot_feet(i: usize, step: f32) -> Vec3 {
    SELECT_STAGE + Vec3::new(SLOTS_X[i.min(2)], 0.0, if i == 1 { 0.18 } else { 0.0 } + step * 0.32)
}

fn block(out: &mut Vec<Instance>, p: Vec3, s: Vec3, rot: Quat, c: Vec3, glow: Vec3) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(s, rot, p), c, 1.0, glow, SHADE_LIT));
}

/// The cavern's rock, flagstones, crystals and the timbers overhead: everything that stays put.
pub(super) fn cavern() -> Vec<Instance> {
    let mut out = Vec::new();
    let at = |x: f32, y: f32, z: f32| SELECT_STAGE + Vec3::new(x, y, z);
    let h = |a: f64, b: f64| hash(a, b) as f32;
    let flat = Quat::IDENTITY;

    // The stage: big cracked flagstones, a little uneven, their gaps dark.
    for gx in -5..=5 {
        for gz in -2..=2 {
            let k = h(gx as f64 * 3.1 + 7.0, gz as f64 * 5.3 + 1.0);
            let (x, z) = (gx as f32 * 0.72 + (k - 0.5) * 0.05, gz as f32 * 0.72 + 0.35);
            let top = (k - 0.5) * 0.035;
            let c = lin(STONE[(k * 3.0) as usize % 3]) * (0.85 + k * 0.3);
            block(&mut out, at(x, top - 0.1, z), Vec3::new(0.68, 0.2, 0.68), Quat::from_rotation_y((k - 0.5) * 0.06), c, Vec3::ZERO);
            // A crack across some of them.
            if k > 0.6 {
                block(
                    &mut out,
                    at(x + (k - 0.8) * 0.3, top + 0.002, z),
                    Vec3::new(0.02, 0.01, 0.5),
                    Quat::from_rotation_y(k * 3.0),
                    lin(0x1c1816),
                    Vec3::ZERO,
                );
            }
        }
    }
    // Beyond the stage the cave floor falls away, strewn with rubble.
    for i in 0..70 {
        let (a, b) = (h(i as f64, 3.0), h(i as f64, 9.0));
        let x = (a - 0.5) * 13.0;
        let z = -3.4 + b * 1.8 + if x.abs() > 4.2 { b * 4.0 } else { 0.0 };
        let size = 0.2 + h(i as f64, 5.0) * 0.5;
        block(
            &mut out,
            at(x, -0.2 + size * 0.3, z),
            Vec3::new(size, size * 0.7, size * 0.9),
            Quat::from_rotation_y(a * 6.0),
            lin(ROCK[i % 4]),
            Vec3::ZERO,
        );
    }
    block(&mut out, at(0.0, -0.45, -1.0), Vec3::new(16.0, 0.5, 12.0), flat, lin(ROCK[2]), Vec3::ZERO);

    // The back wall: great craggy blocks piled high, and the side walls closing in.
    for gx in -9i32..=9 {
        for gy in 0..8 {
            let k = h(gx as f64 * 1.7, gy as f64 * 2.3);
            let size = 0.8 + k * 0.6;
            let x = gx as f32 * 0.85 + (k - 0.5) * 0.3;
            let z = -4.0 - k * 0.9 - (gx as f32).abs() * 0.05;
            block(
                &mut out,
                at(x, gy as f32 * 0.82 + size * 0.3, z),
                Vec3::splat(size),
                Quat::from_rotation_y((k - 0.5) * 0.8),
                lin(ROCK[(gx + gy).rem_euclid(4) as usize]) * (0.8 + k * 0.4),
                Vec3::ZERO,
            );
        }
    }
    for side in [-1.0f32, 1.0] {
        for gz in -4i32..=4 {
            for gy in 0..8 {
                let k = h(gz as f64 * 2.9 + side as f64 * 5.0, gy as f64 * 1.3);
                let size = 0.9 + k * 0.7;
                let x = side * (5.4 + k * 0.8 + (gz as f32 + 4.0) * 0.12);
                block(
                    &mut out,
                    at(x, gy as f32 * 0.82 + size * 0.3, gz as f32 * 0.9 - 0.5),
                    Vec3::splat(size),
                    Quat::from_rotation_y(k * 2.0),
                    lin(ROCK[(gz + gy).rem_euclid(4) as usize]) * (0.75 + k * 0.4),
                    Vec3::ZERO,
                );
            }
        }
    }
    // Stalactites hanging from the roof (the roof itself is out of sight, and would shade the key light).
    for i in 0..26 {
        let (a, b) = (h(i as f64 * 1.3, 17.0), h(i as f64 * 2.1, 29.0));
        let (x, z) = ((a - 0.5) * 11.0, -3.6 + b * 3.4);
        let len = 0.6 + h(i as f64, 41.0) * 1.6;
        for k in 0..4 {
            let t = k as f32 / 4.0;
            let w = 0.34 * (1.0 - t) + 0.05;
            block(
                &mut out,
                at(x, 6.0 - len * t, z),
                Vec3::new(w, len / 4.0 + 0.02, w),
                Quat::from_rotation_y(a * 5.0 + k as f32),
                lin(ROCK[(i + k) % 4]),
                Vec3::ZERO,
            );
        }
    }
    // Channels in the wall down which the lava pours, and the crusted basins below.
    for &(x, z, top) in &LAVA_FALLS {
        for side in [-1.0f32, 1.0] {
            for k in 0..8 {
                let y = k as f32 * (top / 8.0);
                block(
                    &mut out,
                    at(x + side * 0.42, y + 0.3, z + 0.1),
                    Vec3::new(0.26, top / 8.0 + 0.05, 0.4),
                    Quat::from_rotation_y(side * 0.2),
                    lin(ROCK[k % 4]) * 0.8,
                    Vec3::ZERO,
                );
            }
        }
        for k in 0..10 {
            let a = k as f32 / 10.0 * TAU;
            block(
                &mut out,
                at(x + a.cos() * 0.75, 0.02, z + 0.55 + a.sin() * 0.45),
                Vec3::new(0.32, 0.18, 0.3),
                Quat::from_rotation_y(a),
                lin(CRUST),
                Vec3::ZERO,
            );
        }
    }
    // Crystal clusters: on the floor to the right, high on the right wall, a few to the left.
    for &(x, y, z, count, size) in
        &[(3.5f32, 0.0f32, -1.3f32, 7u32, 1.0f32), (4.3, 2.3, -3.1, 6, 0.8), (-4.1, 0.0, -2.2, 4, 0.6), (2.6, 0.0, -3.2, 3, 0.55)]
    {
        for k in 0..count {
            let (a, b) = (h(x as f64 + k as f64, y as f64 + 3.0), h(z as f64 * 3.0 + k as f64, 11.0));
            let tall = (0.35 + a * 0.55) * size;
            let tilt = Quat::from_rotation_z((b - 0.5) * 0.9) * Quat::from_rotation_x((a - 0.5) * 0.7);
            let base = at(x + (a - 0.5) * 0.6 * size, y, z + (b - 0.5) * 0.5 * size);
            let c = if k % 3 == 0 { lin(CRYSTAL_D) } else { lin(CRYSTAL) };
            block(
                &mut out,
                base + tilt * Vec3::new(0.0, tall * 0.5, 0.0),
                Vec3::new(0.11 * size, tall, 0.11 * size),
                tilt * Quat::from_rotation_y(0.785),
                c,
                lin(CRYSTAL) * 0.9,
            );
            block(
                &mut out,
                base + tilt * Vec3::new(0.0, tall + 0.03, 0.0),
                Vec3::splat(0.06 * size),
                tilt * Quat::from_rotation_y(0.785),
                lin(0xd8f6ff),
                lin(CRYSTAL) * 1.4,
            );
        }
    }
    // A timber beam across the roof, a rope and pulley hanging from it.
    block(&mut out, at(-0.6, 4.6, -1.8), Vec3::new(6.5, 0.22, 0.24), Quat::from_rotation_z(0.04), lin(PLANK), Vec3::ZERO);
    for x in [-3.4f32, 2.2] {
        block(&mut out, at(x, 2.3, -1.8), Vec3::new(0.2, 4.6, 0.22), flat, lin(PLANK) * 0.85, Vec3::ZERO);
    }
    block(&mut out, at(0.9, 3.7, -1.8), Vec3::new(0.025, 1.8, 0.025), flat, lin(ROPE), Vec3::ZERO);
    block(&mut out, at(0.9, 2.75, -1.8), Vec3::new(0.16, 0.16, 0.08), flat, lin(IRON), Vec3::ZERO);
    out
}

/// The lava falls: where each pours down the back wall (x, z) and from how high.
const LAVA_FALLS: [(f32, f32, f32); 2] = [(-3.7, -3.5, 5.2), (3.3, -3.7, 4.6)];

impl WorldView {
    /// The character screen, each frame: the cavern, the moving lava and embers, the heroes and the light.
    pub(super) fn emit_select(&self, sel: SelectView, s: &SceneState, f: &mut Frame) {
        let time = s.time as f32;
        let cube = MeshKind::Cube as usize;
        let at = |x: f32, y: f32, z: f32| SELECT_STAGE + Vec3::new(x, y, z);
        f.atmosphere = cavern_atmosphere();
        f.sets.push(self.select_set.clone());

        // ---- lava: pouring down its channels in bright bands, into glowing basins that bubble
        for (n, &(x, z, top)) in LAVA_FALLS.iter().enumerate() {
            // Three strands side by side, each running in bright veins over a dark crust.
            let segs = 18;
            let seg_h = top / segs as f32;
            for strand in 0..3 {
                let sx = (strand as f32 - 1.0) * 0.16;
                for k in 0..segs {
                    let y = seg_h * k as f32;
                    let phase = y * 2.6 - time * (2.2 + strand as f32 * 0.35) + strand as f32 * 1.9 + n as f32;
                    let vein = (phase.sin() * 0.5 + 0.5).powf(2.2);
                    let glow = lin(0xc83a0a).lerp(lin(LAVA_HOT), vein) * (0.6 + vein * 2.6);
                    let wobble = (y * 3.0 + strand as f32).sin() * 0.02;
                    let m = Mat4::from_translation(at(x + sx + wobble, y + seg_h * 0.5 + 0.15, z + 0.12 + strand as f32 % 2.0 * 0.02))
                        * Mat4::from_scale(Vec3::new(0.17, seg_h + 0.02, 0.14));
                    f.opaque[cube].push(Instance::new(m, lin(0x3a0c04), 1.0, glow, SHADE_LIT));
                }
            }
            // Bright gobs sliding down the fall.
            for k in 0..6 {
                let t = (time * 0.35 + k as f32 / 6.0 + n as f32 * 0.3).fract();
                let p = at(x + ((k as f32 * 1.7).sin()) * 0.15, top * (1.0 - t) + 0.2, z + 0.22);
                f.additive.push(Instance::new(
                    Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(0.14, 0.26, 0.05)),
                    lin(LAVA_HOT) * 1.5,
                    0.55,
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
            // The basin: a crusted pool, cracks of molten rock glowing through it.
            for gx in 0..5 {
                for gz in 0..3 {
                    let k = hash(gx as f64 + n as f64 * 7.0, gz as f64 * 3.0) as f32;
                    let hot = ((time * 0.9 + k * 6.0).sin() * 0.5 + 0.5) * 0.7 + 0.3;
                    let p = at(x - 0.52 + gx as f32 * 0.26, 0.02, z + 0.3 + gz as f32 * 0.25);
                    let m = Mat4::from_translation(p) * Mat4::from_rotation_y(k * 2.0) * Mat4::from_scale(Vec3::new(0.24, 0.06, 0.22));
                    f.opaque[cube].push(Instance::new(
                        m,
                        lin(0x2a0a04),
                        1.0,
                        lin(LAVA).lerp(lin(LAVA_HOT), hot * 0.6) * (0.9 + hot * 1.6),
                        SHADE_LIT,
                    ));
                }
            }
            for k in 0..5 {
                let ph = (time * 0.8 + k as f32 * 0.37 + n as f32).fract();
                let a = k as f32 * 2.3 + n as f32;
                let p = at(x + a.cos() * 0.45, 0.07 + ph * 0.05, z + 0.55 + a.sin() * 0.25);
                f.additive.push(Instance::new(
                    Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.05 + ph * 0.06)),
                    lin(LAVA_HOT),
                    0.8 * (1.0 - ph),
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
            glow_orb(f, at(x, 1.2, z + 0.5), 1.6, lin(LAVA), 0.22);
            glow_orb(f, at(x, top * 0.6, z + 0.3), 1.1, lin(LAVA), 0.16);
            let flick = 1.0 + (time * 5.0 + n as f32).sin() * 0.06;
            f.lights.push(Light { pos: at(x, 0.9, z + 1.0), color: lin(0xff7a28) * 11.0 * flick, distance: 8.0 });
        }
        // ---- embers drifting up from the lava across the cavern
        for k in 0..36 {
            let fk = k as f32;
            let src = LAVA_FALLS[k % 2];
            let t = (time * (0.12 + (fk * 0.37).fract() * 0.08) + fk * 0.618).fract();
            let p = at(
                src.0 * (1.0 - t * 0.7) + (fk * 1.3 + time * 0.7).sin() * 0.5,
                0.2 + t * 4.5,
                src.1 + 0.8 + t * 3.0 + (fk * 2.1).cos() * 0.6,
            );
            let a = (1.0 - t) * (0.5 + 0.5 * (time * 9.0 + fk).sin().abs());
            f.additive.push(Instance::new(
                Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.022)),
                lin(0xffa040) * 1.6,
                a,
                Vec3::ZERO,
                SHADE_UNLIT,
            ));
        }
        // ---- the crystals' glow, and a lantern on its chain swaying from the beam
        for &(x, y, z, r) in &[(3.5f32, 0.5f32, -1.3f32, 1.1f32), (4.3, 2.7, -3.1, 0.9), (-4.1, 0.3, -2.2, 0.6)] {
            let pulse = 1.0 + (time * 1.3 + x).sin() * 0.12;
            glow_orb(f, at(x, y, z), r, lin(CRYSTAL), 0.18 * pulse);
            f.lights.push(Light { pos: at(x, y + 0.2, z + 0.4), color: lin(0x48c8ff) * 4.0 * pulse * r, distance: 4.5 });
        }
        let swing = (time * 0.9).sin() * 0.12;
        let pivot = at(-1.9, 4.5, -1.8);
        let lantern = pivot + Vec3::new(swing.sin() * 1.1, -1.1 * swing.cos(), 0.0);
        let chain =
            Mat4::from_scale_rotation_translation(Vec3::new(0.02, 1.1, 0.02), Quat::from_rotation_z(swing), (pivot + lantern) * 0.5);
        f.opaque[cube].push(Instance::new(chain, lin(IRON), 1.0, Vec3::ZERO, SHADE_LIT));
        f.opaque[cube].push(Instance::new(
            Mat4::from_scale_rotation_translation(Vec3::new(0.16, 0.2, 0.16), Quat::from_rotation_z(swing), lantern),
            lin(0xffd080),
            1.0,
            lin(0xffb050) * 2.4,
            SHADE_LIT,
        ));
        glow_orb(f, lantern, 0.5, lin(0xffb060), 0.25);
        f.lights.push(Light { pos: lantern, color: lin(0xffb060) * 3.5, distance: 5.0 });

        // ---- the heroes: the picked one steps forward into the light; locked ones stand grey
        for (i, &kind) in ROSTER.iter().enumerate() {
            let rig = match kind {
                HeroKind::Ragnar => &self.models.ragnar,
                HeroKind::Mehmed => &self.models.mehmed,
                HeroKind::Miyamoto => &self.models.miyamoto,
            };
            let picked = i == sel.index;
            let step = self.select_step[i];
            let mut feet = slot_feet(i, step);
            if picked && sel.denied > 0.0 {
                feet.x += (sel.denied * 70.0).sin() * 0.035 * (sel.denied / 0.45);
            }
            // Turned a little toward the middle, full on to the camera when picked.
            let yaw = -SLOTS_X[i] * 0.2 * (1.0 - step);
            let breathe = 1.0 + (time * 2.0 + i as f32 * 1.7).sin() * 0.006;
            let root = Mat4::from_translation(feet) * Mat4::from_rotation_y(yaw) * Mat4::from_scale(Vec3::new(1.0, breathe, 1.0));
            let mut pose = Pose::rest(rig);
            pose.ready(rig);
            if let Some(hd) = rig.index("head") {
                pose.rot[hd].y = (time * 0.6 + i as f32 * 2.0).sin() * 0.12;
            }
            if kind.unlocked() || self.preview_heroes {
                emit_rig(&mut f.rigs, rig, &pose, root, Vec3::ONE, 1.0, Vec3::ZERO);
            } else {
                emit_rig_grey(&mut f.rigs, rig, &pose, root, Vec3::new(0.95, 0.97, 1.05) * (0.85 + 0.15 * step));
            }
            // A pool of light at the picked hero's feet: gold if he can be chosen, pale if not.
            if picked {
                let (color, strength) = if kind.unlocked() { (lin(0xffd58a), 0.35) } else { (lin(0x9ab0d0), 0.18) };
                let pool = Mat4::from_translation(feet + Vec3::new(0.0, 0.012, 0.0))
                    * Mat4::from_rotation_x(-PI / 2.0)
                    * Mat4::from_scale(Vec3::new(0.7, 0.7, 1.0));
                f.glow.push(Instance::new(pool, color, strength * (0.85 + 0.15 * (time * 3.0).sin()), Vec3::ZERO, SHADE_GLOW));
                let spot = if kind.unlocked() { lin(0xfff0d8) * 6.0 } else { lin(0xa8bcdc) * 3.0 };
                f.lights.push(Light { pos: feet + Vec3::new(0.2, 2.2, 1.3), color: spot * step.max(0.3), distance: 4.0 });
            }
        }
        // A steady warm light on the whole line-up from the front, and a cool rim behind them all.
        f.lights.push(Light { pos: at(-0.6, 2.8, 2.6), color: lin(0xffe4c4) * 7.0, distance: 7.0 });
        f.lights.push(Light { pos: at(0.0, 2.2, -1.6), color: lin(0x7a9ccc) * 5.0, distance: 5.0 });
        // Dust hanging in the light.
        for k in 0..24 {
            let fk = k as f32;
            let p = at(
                ((fk * 0.618).fract() - 0.5) * 6.0 + (time * 0.2 + fk).sin() * 0.2,
                0.4 + (fk * 0.37).fract() * 2.6 + (time * 0.3 + fk * 1.3).sin() * 0.1,
                ((fk * 0.29).fract() - 0.3) * 2.5,
            );
            glow_orb(f, p, 0.025, lin(0xfff0d0), 0.35 + 0.3 * (time * 1.7 + fk).sin());
        }
    }
}

/// A cavern lit by lava and crystal: no sky, a dark haze, a warm key light from the front.
pub(super) fn cavern_atmosphere() -> Atmosphere {
    Atmosphere {
        background: 0x0b0706,
        fog_near: 6.5,
        fog_far: 17.0,
        hemi_sky: lin(0x3a4458) * 0.38,
        hemi_ground: lin(0x6a2a12) * 0.35,
        ambient: lin(0x2a1c16) * 0.5,
        sun_dir: Vec3::new(-0.45, 0.78, 0.62).normalize(),
        sun: lin(0xffe2c0) * 1.25,
        exposure: 1.15,
        night_sky: 0.0,
        bloom: 1.35,
        moon_view: Vec3::Y,
        cloud_tint: lin(0x161b24),
    }
}
