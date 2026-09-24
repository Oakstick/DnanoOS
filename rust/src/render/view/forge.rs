//! The blacksmith's forge (the shop's backdrop), the look of what the smith
//! sells once it is in use (new projectiles, potion auras, lightning arcs,
//! swiftness afterimages), and a few meshes they need.

use super::*;
use crate::logic::items::RangedKind;

/// Hammer blows per second; the game rings the anvil on the same beat.
pub const SMITH_BEAT: f64 = 1.25;
/// Where the forge stands, well away from the level and the title stage.
pub(super) const FORGE: Vec3 = Vec3::new(-80.0, 0.0, -40.0);
const ZAP_SECONDS: f32 = 0.22;
const GHOSTS: usize = 4;

/// A lightning arc between two points, flickering while it lasts.
pub(super) struct Zap {
    pub a: Vec3,
    pub b: Vec3,
    pub life: f32,
    pub seed: f32,
}

/// A small throwing knife, blade forward (+z).
pub(super) fn knife_model() -> crate::render::voxels::Collected {
    use crate::render::characters::Sculpt;
    let mut s = Sculpt::new(3, 2, 9);
    for z in 0..3 {
        s.set(1, 0, z, 0x6b3f1d);
    }
    s.cuboid(0, 0, 3, 2, 0, 3, 0xc8942a);
    for z in 4..9 {
        s.set(1, 0, z, 0xc8d0da);
        s.set(1, 1, z, 0xf4f8fc);
    }
    s.finish(0.03)
}

/// A line segment as a stretched cube from `a` to `b`.
fn segment(a: Vec3, b: Vec3, thick: f32) -> Mat4 {
    let d = b - a;
    let len = d.length().max(1e-4);
    let rot = Quat::from_rotation_arc(Vec3::Y, d / len);
    Mat4::from_scale_rotation_translation(Vec3::new(thick, len, thick), rot, (a + b) * 0.5)
}

impl WorldView {
    /// A crackling arc from `from` to `to` (tile positions), at chest height.
    pub fn zap(&mut self, grid: &Grid, from: crate::logic::types::V, to: crate::logic::types::V) {
        let lift = |p: crate::logic::types::V| Vec3::new(p.x as f32, floor_at(grid, p.x as f32, p.y as f32) + 0.6, p.y as f32);
        let seed = self.rng.nextf() * 100.0;
        self.zaps.push(Zap { a: lift(from), b: lift(to), life: ZAP_SECONDS, seed });
        self.sparkle(grid, to.x, to.y, 0.6, 0xbfe8ff, 10);
    }

    pub(super) fn update_forge_fx(&mut self, dt: f32, hero: &Hero) {
        self.zaps.retain_mut(|z| {
            z.life -= dt;
            z.life > 0.0
        });
        // Swiftness leaves a trail of fading afterimages.
        self.ghost_clock += dt;
        if hero.hasted() && hero.motion > 0.3 && self.ghost_clock > 0.05 {
            self.ghost_clock = 0.0;
            let p = hero.pos();
            self.ghosts.insert(0, (Vec3::new(p.x as f32, hero.altitude as f32 * STEP, p.y as f32), self.hero_yaw, 1.0));
            self.ghosts.truncate(GHOSTS);
        }
        for g in self.ghosts.iter_mut() {
            g.2 -= dt * 3.0;
        }
        self.ghosts.retain(|g| g.2 > 0.0);
    }

    /// Sebastian's rig for the axe and shield he has right now.
    pub(super) fn hero_rig(&self, hero: &Hero) -> &Rig {
        let key = (hero.axe_level.clamp(1, 3), hero.shield.map(|s| s.kind));
        self.models.heroes.get(&key).unwrap_or(&self.models.popolon)
    }

    /// Auras and trails of the potions he has drunk, and the lightning arcs.
    pub(super) fn emit_hero_fx(&self, hero: &Hero, pos: Vec3, time: f32, f: &mut Frame) {
        let cube = MeshKind::Cube as usize;
        if hero.alive {
            // Afterimages from swiftness.
            let rig = self.hero_rig(hero);
            for (p, yaw, life) in &self.ghosts {
                let root = Mat4::from_translation(*p) * Mat4::from_rotation_y(*yaw);
                emit_rig(&mut f.transparent[cube], rig, &self.hero_pose, root, lin(0xffe066), 0.28 * life, lin(0xffd23a) * 0.4);
            }
            let fx = &hero.fx;
            if fx.strength > 0.0 {
                let pulse = 0.7 + (time * 6.0).sin() * 0.3;
                f.lights.push(Light { pos: pos + Vec3::new(0.0, 0.8, 0.0), color: lin(0xff3a2a) * 3.0 * pulse, distance: 3.0 });
                for k in 0..6 {
                    let a = time * 2.5 + k as f32 * TAU / 6.0;
                    let rise = (time * 1.3 + k as f32 * 0.37).fract();
                    let p = pos + Vec3::new(a.cos() * 0.4, 0.1 + rise * 1.1, a.sin() * 0.4);
                    f.additive.push(Instance::new(Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.05)), lin(0xff4a3a) * 1.5, 0.8 * (1.0 - rise), Vec3::ZERO, SHADE_UNLIT));
                }
            }
            if fx.arcane > 0.0 {
                for k in 0..8 {
                    let a = time * 3.5 + k as f32 * TAU / 8.0;
                    let p = pos + Vec3::new(a.cos() * 0.55, 0.6 + (time * 4.0 + k as f32).sin() * 0.25, a.sin() * 0.55);
                    f.additive.push(Instance::new(Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.045)), lin(0xb07aff) * 1.6, 0.9, Vec3::ZERO, SHADE_UNLIT));
                }
            }
            if fx.regeneration > 0.0 {
                for k in 0..5 {
                    let rise = (time * 0.8 + k as f32 * 0.2).fract();
                    let a = k as f32 * 2.4;
                    let p = pos + Vec3::new(a.cos() * 0.3, 0.2 + rise * 1.2, a.sin() * 0.3);
                    let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(0.09, 0.03, 0.03));
                    f.additive.push(Instance::new(m, lin(0x7dff6a), 0.8 * (1.0 - rise), Vec3::ZERO, SHADE_UNLIT));
                    let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(0.03, 0.09, 0.03));
                    f.additive.push(Instance::new(m, lin(0x7dff6a), 0.8 * (1.0 - rise), Vec3::ZERO, SHADE_UNLIT));
                }
            }
            if fx.lightning > 0.0 {
                // Little sparks crawling over him.
                for k in 0..3 {
                    let t = (time * 9.0 + k as f32 * 3.1).floor();
                    let h = |n: f32| (t * 12.9898 + n * 78.233).sin().fract().abs();
                    let a = pos + Vec3::new(h(1.0) - 0.5, 0.3 + h(2.0) * 0.9, h(3.0) - 0.5) * Vec3::new(0.7, 1.0, 0.7);
                    let b = a + Vec3::new(h(4.0) - 0.5, h(5.0) - 0.5, h(6.0) - 0.5) * 0.35;
                    f.additive.push(Instance::new(segment(a, b, 0.025), lin(0xbfe8ff) * 1.6, 0.9, Vec3::ZERO, SHADE_UNLIT));
                }
            }
        }
        for z in &self.zaps {
            let k = z.life / ZAP_SECONDS;
            let flick = (time * 60.0).floor();
            let mut prev = z.a;
            let steps = 7;
            for i in 1..=steps {
                let t = i as f32 / steps as f32;
                let mut p = z.a.lerp(z.b, t);
                if i < steps {
                    let h = |n: f32| ((flick + z.seed) * 12.9898 + i as f32 * 4.1 + n * 78.233).sin().fract() - 0.5;
                    p += Vec3::new(h(1.0), h(2.0) * 0.6, h(3.0)) * 0.45;
                }
                f.additive.push(Instance::new(segment(prev, p, 0.05), lin(0xbfe8ff) * 2.0, k, Vec3::ZERO, SHADE_UNLIT));
                f.additive.push(Instance::new(segment(prev, p, 0.14), lin(0x6aa8ff), 0.35 * k, Vec3::ZERO, SHADE_UNLIT));
                prev = p;
            }
            f.lights.push(Light { pos: z.b, color: lin(0xbfe8ff) * 6.0 * k, distance: 4.0 });
        }
    }

    /// One shot from the blacksmith's weapons, drawn along its heading.
    pub(super) fn emit_forged_shot(&self, kind: RangedKind, pos: Vec3, yaw: f32, time: f32, f: &mut Frame) -> bool {
        let cube = MeshKind::Cube as usize;
        let group = Mat4::from_translation(pos) * Mat4::from_rotation_y(yaw);
        match kind {
            RangedKind::Knife | RangedKind::TwinKnives | RangedKind::SeekingKnives => {
                let tint = if kind == RangedKind::SeekingKnives { Vec3::new(0.8, 0.95, 1.3) } else { Vec3::ONE };
                let spin = group * Mat4::from_rotation_z(time * 20.0);
                emit_rig(&mut f.opaque[cube], &self.models.knife, &Pose::rest(&self.models.knife), spin * Mat4::from_translation(Vec3::new(0.0, 0.0, -0.13)), tint, 1.0, Vec3::ZERO);
                if kind == RangedKind::SeekingKnives {
                    let m = group * Mat4::from_translation(Vec3::new(0.0, 0.0, -0.25)) * Mat4::from_scale(Vec3::new(0.05, 0.05, 0.4));
                    f.additive.push(Instance::new(m, lin(0x6aa8ff), 0.6, Vec3::ZERO, SHADE_UNLIT));
                }
            }
            RangedKind::FlameArrow | RangedKind::HolyArrow => {
                let holy = kind == RangedKind::HolyArrow;
                let tint = if holy { Vec3::new(1.6, 1.4, 0.6) } else { Vec3::ONE };
                emit_mesh(&mut f.opaque[cube], &self.models.arrow, group, tint, 1.0, if holy { lin(0xffe066) * 0.6 } else { Vec3::ZERO });
                let head = group.transform_point3(Vec3::new(0.0, 0.0, 0.3));
                let (c, s) = if holy { (lin(0xfff6c0), 0.28) } else { (lin(0xff7a2a), 0.16) };
                let pulse = 1.0 + (time * 25.0 + pos.x).sin() * 0.15;
                f.additive.push(Instance::new(Mat4::from_translation(head) * Mat4::from_scale(Vec3::splat(s * pulse)), c * 1.5, 0.8, Vec3::ZERO, SHADE_UNLIT));
                if holy {
                    let m = group * Mat4::from_translation(Vec3::new(0.0, 0.0, -0.3)) * Mat4::from_scale(Vec3::new(0.08, 0.08, 0.9));
                    f.additive.push(Instance::new(m, lin(0xfff6c0), 0.5, Vec3::ZERO, SHADE_UNLIT));
                }
                f.lights.push(Light { pos: head, color: c * if holy { 6.0 } else { 3.5 }, distance: 3.5 });
            }
            RangedKind::SteelBoomerang | RangedKind::FireBoomerang => {
                let fire = kind == RangedKind::FireBoomerang;
                let (tint, glow) = if fire { (Vec3::new(1.6, 0.8, 0.4), lin(0xff5a1a) * 0.9) } else { (Vec3::new(0.95, 1.15, 1.45), Vec3::ZERO) };
                let spin = group * Mat4::from_rotation_y(time * 16.0) * Mat4::from_scale(Vec3::splat(if fire { 1.2 } else { 1.15 }));
                emit_mesh(&mut f.opaque[cube], &self.models.boomerang, spin, tint, 1.0, glow);
                if fire {
                    f.lights.push(Light { pos, color: lin(0xff7a2a) * 4.0, distance: 3.5 });
                    for k in 0..3 {
                        let a = time * 16.0 + k as f32 * TAU / 3.0;
                        let p = pos + Vec3::new(a.cos() * 0.3, 0.05, a.sin() * 0.3);
                        f.additive.push(Instance::new(Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.09)), lin(0xffa040) * 1.5, 0.7, Vec3::ZERO, SHADE_UNLIT));
                    }
                }
            }
            _ => return false,
        }
        true
    }

    /**
     * Hrolf's forge: a stone floor, a brick hearth with glowing coals and a
     * chimney, an anvil, a rack of weapons, a barrel, and the smith himself
     * hammering away while Sebastian looks on.
     */
    pub(super) fn emit_forge(&self, s: &SceneState, f: &mut Frame) {
        let time = s.time as f32;
        let cube = MeshKind::Cube as usize;
        let cyl = MeshKind::Cylinder as usize;
        let at = |x: f32, y: f32, z: f32| FORGE + Vec3::new(x, y, z);
        let box_ = |f: &mut Frame, p: Vec3, sc: Vec3, c: u32| {
            f.opaque[cube].push(Instance::new(Mat4::from_translation(p) * Mat4::from_scale(sc), lin(c), 1.0, Vec3::ZERO, SHADE_LIT));
        };

        // Floor: big flagstones.
        for gx in -4..=4 {
            for gz in -3..=3 {
                let h = hash(gx as f64 + 40.0, gz as f64 + 7.0) as f32;
                let c = if h < 0.5 { 0x5a5650 } else { 0x4e4a45 };
                box_(f, at(gx as f32 * 0.9, -0.05, gz as f32 * 0.9), Vec3::new(0.86, 0.1, 0.86), c);
            }
        }
        // Back wall: timber frame and plaster, so it reads as a workshop.
        for i in -4..=4 {
            let x = i as f32 * 0.9;
            box_(f, at(x, 1.3, -2.9), Vec3::new(0.84, 2.6, 0.2), if i % 2 == 0 { 0xb8a888 } else { 0xa89878 });
            box_(f, at(x + 0.45, 1.3, -2.75), Vec3::new(0.12, 2.6, 0.12), 0x4a3018);
        }
        box_(f, at(0.0, 2.55, -2.75), Vec3::new(8.2, 0.14, 0.16), 0x4a3018);

        // The hearth: a brick box, a bed of coals, a chimney.
        let hearth = at(-2.2, 0.0, -2.0);
        for (i, y) in [0.15f32, 0.45, 0.75].into_iter().enumerate() {
            for k in 0..5 {
                let off = if i % 2 == 0 { 0.0 } else { 0.12 };
                let c = [0x9a4e33, 0xb05c3c, 0x8a442e][(k + i) % 3];
                box_(f, hearth + Vec3::new(-0.5 + off + k as f32 * 0.24, y, 0.45), Vec3::new(0.22, 0.28, 0.25), c);
            }
            box_(f, hearth + Vec3::new(-0.62, y, 0.0), Vec3::new(0.25, 0.28, 0.95), 0x8a442e);
            box_(f, hearth + Vec3::new(0.62, y, 0.0), Vec3::new(0.25, 0.28, 0.95), 0x9a4e33);
        }
        box_(f, hearth + Vec3::new(0.0, 0.5, -0.3), Vec3::new(1.2, 1.0, 0.4), 0x7a3e2a);
        box_(f, hearth + Vec3::new(0.0, 1.9, -0.2), Vec3::new(0.7, 2.0, 0.6), 0x8a442e);
        let flick = 1.0 + (time * 11.0).sin() * 0.08 + (time * 4.3).sin() * 0.06;
        for k in 0..9 {
            let (gx, gz) = ((k % 3) as f32 - 1.0, (k / 3) as f32 - 1.0);
            let p = hearth + Vec3::new(gx * 0.22, 0.95, gz * 0.2);
            let heat = 0.8 + ((time * 3.0 + k as f32 * 1.7).sin() * 0.2);
            f.opaque[cube].push(Instance::new(Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.2)), lin(0x3a1a10), 1.0, lin(0xff6a1a) * heat * 1.3, SHADE_LIT));
        }
        for k in 0..10 {
            let rise = (time * 0.9 + k as f32 * 0.13).fract();
            let p = hearth + Vec3::new(((k * 7) % 5) as f32 * 0.12 - 0.25 + (time * 2.0 + k as f32).sin() * 0.05, 1.05 + rise * 0.9, ((k * 3) % 4) as f32 * 0.1 - 0.15);
            f.additive.push(Instance::new(Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.05)), lin(0xffa040) * 1.5, 0.9 * (1.0 - rise), Vec3::ZERO, SHADE_UNLIT));
        }

        // The anvil: a waisted iron block with a horn.
        let anvil = at(0.2, 0.0, 0.2);
        box_(f, anvil + Vec3::new(0.0, 0.18, 0.0), Vec3::new(0.34, 0.36, 0.3), 0x3a2a1c);
        box_(f, anvil + Vec3::new(0.0, 0.42, 0.0), Vec3::new(0.3, 0.14, 0.22), 0x3e4248);
        box_(f, anvil + Vec3::new(0.0, 0.54, 0.0), Vec3::new(0.62, 0.12, 0.3), 0x4a4e56);
        box_(f, anvil + Vec3::new(0.4, 0.55, 0.0), Vec3::new(0.2, 0.08, 0.18), 0x4a4e56);
        box_(f, anvil + Vec3::new(0.54, 0.56, 0.0), Vec3::new(0.12, 0.05, 0.1), 0x5a5e66);
        // A glowing blade on the anvil.
        let beat = (s.time * SMITH_BEAT).fract() as f32;
        let hot = 0.8 + (1.0 - beat) * 0.6;
        f.opaque[cube].push(Instance::new(Mat4::from_translation(anvil + Vec3::new(-0.05, 0.62, 0.02)) * Mat4::from_scale(Vec3::new(0.46, 0.03, 0.09)), lin(0x5a2a1a), 1.0, lin(0xff7a2a) * hot, SHADE_LIT));

        // Weapon rack on the right: axes, a bow's arrows, a shield.
        let rack = at(2.6, 0.0, -2.55);
        box_(f, rack + Vec3::new(0.0, 1.0, 0.0), Vec3::new(1.6, 0.1, 0.1), 0x5a3a1c);
        box_(f, rack + Vec3::new(0.0, 0.3, 0.0), Vec3::new(1.6, 0.1, 0.1), 0x5a3a1c);
        for (i, x) in [-0.55f32, 0.0, 0.55].into_iter().enumerate() {
            box_(f, rack + Vec3::new(x, 0.75, 0.1), Vec3::new(0.06, 1.2, 0.06), 0x6b3f1d);
            let blade = if i == 1 { 0xffd84a } else { 0xc8d0da };
            box_(f, rack + Vec3::new(x + 0.12, 1.2, 0.1), Vec3::new(0.2, 0.28, 0.04), blade);
            if i == 1 {
                box_(f, rack + Vec3::new(x - 0.12, 1.2, 0.1), Vec3::new(0.2, 0.28, 0.04), blade);
            }
        }
        let shield = Mat4::from_translation(rack + Vec3::new(-1.3, 0.9, 0.1)) * Mat4::from_rotation_x(PI / 2.0) * Mat4::from_scale(Vec3::new(0.9, 0.06, 0.9));
        f.opaque[cyl].push(Instance::new(shield, lin(0xd8262e), 1.0, Vec3::ZERO, SHADE_LIT));
        // A barrel and a whetstone.
        let barrel = Mat4::from_translation(at(-3.2, 0.35, -0.6)) * Mat4::from_scale(Vec3::new(1.1, 1.4, 1.1));
        f.opaque[cyl].push(Instance::new(barrel, lin(0x7a4e28), 1.0, Vec3::ZERO, SHADE_LIT));
        box_(f, at(-3.2, 0.72, -0.6), Vec3::new(0.5, 0.02, 0.5), 0x3a6a8a);
        box_(f, at(3.0, 0.3, 0.6), Vec3::new(0.5, 0.6, 0.4), 0x5a3a1c);
        let wheel = Mat4::from_translation(at(3.0, 0.78, 0.6)) * Mat4::from_rotation_z(PI / 2.0) * Mat4::from_rotation_y(time * 4.0) * Mat4::from_scale(Vec3::new(0.7, 0.2, 0.7));
        f.opaque[cyl].push(Instance::new(wheel, lin(0x8a867c), 1.0, Vec3::ZERO, SHADE_LIT));

        // Hrolf, behind the anvil, swinging his hammer on the beat.
        let smith = &self.models.blacksmith;
        let mut pose = Pose::rest(smith);
        // Raise slowly, strike fast.
        let swing = if beat < 0.72 { -2.3 * (beat / 0.72).powf(0.7) } else { -2.3 + 2.55 * ((beat - 0.72) / 0.28).powi(2) };
        if let Some(i) = smith.index("hammerArm") {
            pose.rot[i].x = swing.min(0.25);
        }
        if let Some(i) = smith.index("head") {
            pose.rot[i].x = 0.15 + (time * 0.7).sin() * 0.05;
        }
        let root = Mat4::from_translation(at(0.05, 0.0, -0.75)) * Mat4::from_rotation_y(0.35) * Mat4::from_scale(Vec3::splat(1.05));
        emit_rig(&mut f.opaque[cube], smith, &pose, root, Vec3::ONE, 1.0, Vec3::ZERO);
        // Sparks fly on each strike.
        if beat < 0.25 {
            let k = beat / 0.25;
            for i in 0..14 {
                let fi = i as f32;
                let a = fi * 2.399 + (s.time * SMITH_BEAT).floor() as f32;
                let sp = 0.8 + (fi * 0.37) % 1.0;
                let p = anvil + Vec3::new(a.cos() * sp * k, 0.65 + k * (1.2 - k * 1.4) * sp, a.sin() * sp * k * 0.6);
                f.additive.push(Instance::new(Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.035)), lin(0xffd060) * 2.0, 1.0 - k, Vec3::ZERO, SHADE_UNLIT));
            }
            f.lights.push(Light { pos: anvil + Vec3::new(0.0, 0.9, 0.2), color: lin(0xffc060) * 10.0 * (1.0 - k), distance: 4.0 });
        }

        // Sebastian, in his current gear, watching the work.
        let hero = s.hero;
        let rig = self.hero_rig(hero);
        let mut hp = Pose::rest(rig);
        if let Some(h) = rig.index("head") {
            hp.rot[h].y = (time * 0.5).sin() * 0.2;
        }
        let root = Mat4::from_translation(at(-1.25, 0.0, 0.9)) * Mat4::from_rotation_y(0.95);
        emit_rig(&mut f.opaque[cube], rig, &hp, root, Vec3::ONE, 1.0, Vec3::ZERO);

        // Light: the hearth's glow, a warm fill, a cool night rim from the door.
        f.lights.push(Light { pos: hearth + Vec3::new(0.0, 1.3, 0.8), color: lin(0xff8a3a) * 26.0 * flick, distance: 7.0 });
        f.lights.push(Light { pos: at(1.5, 2.4, 2.2), color: lin(0xffd09a) * 16.0, distance: 9.0 });
        f.lights.push(Light { pos: at(3.5, 2.0, 1.5), color: lin(0x8fb4ff) * 8.0, distance: 8.0 });
    }
}
