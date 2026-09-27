//! How the hero's gear looks in use: the rig of the weapon and shield he
//! carries (rebuilt whenever either changes), his thrown armory weapons in
//! flight, the auras of his elixirs and skills, lightning arcs, decoys and
//! blinks.

use super::*;
use crate::logic::heroes::HeroKind;
use crate::logic::items::RangedKind;
use crate::logic::progression::{self, Tree};

const ZAP_SECONDS: f32 = 0.22;
const GHOSTS: usize = 4;

/// A lightning arc between two points, flickering while it lasts.
pub(super) struct Zap {
    pub a: Vec3,
    pub b: Vec3,
    pub life: f32,
    pub seed: f32,
}

/// A line segment as a stretched cube from `a` to `b`.
fn segment(a: Vec3, b: Vec3, thick: f32) -> Mat4 {
    let d = b - a;
    let len = d.length().max(1e-4);
    let rot = Quat::from_rotation_arc(Vec3::Y, d / len);
    Mat4::from_scale_rotation_translation(Vec3::new(thick, len, thick), rot, (a + b) * 0.5)
}

/// Where the sword arm and the shield arm pivot in the detailed hero grid.
const SWORD_PIVOT: [f32; 3] = [24.0, 27.0, 8.0];
const SHIELD_PIVOT: [f32; 3] = [5.0, 27.0, 8.0];

/**
 * The detailed hero with a weapon of his armory in hand (and its twin in the
 * other, for a pair) and a shield of his tree on his back, or none. The
 * weapons are layered parts ("blade", "offhand") that follow the arms.
 */
pub(super) fn hero_model(hero: HeroKind, blade: u8, shield: Option<u8>) -> Rig {
    let w = progression::node(hero, Tree::Armory, blade);
    let body = characters::ragnar_hd(shield.map(|t| {
        let n = progression::node(hero, Tree::Shield, t);
        (n.form, n.color, t)
    }));
    let main = crate::render::armory::hero_weapon(w.form, w.color, false);
    let off = if crate::render::armory::paired(w.form) {
        crate::render::armory::hero_weapon(w.form, w.color, true)
    } else {
        crate::render::voxels::Collected { voxels: Vec::new(), width: body.width, depth: body.depth, height: body.height }
    };
    build_layered_rig(&body, characters::RAGNAR_HD_RIG, &[("blade", &main, SWORD_PIVOT), ("offhand", &off, SHIELD_PIVOT)], VOXEL / 2.0)
}

/// A thrown armory weapon on its own, for its flight: the model without the fist, centred.
pub(super) fn thrown_model(hero: HeroKind, blade: u8) -> Rig {
    let w = progression::node(hero, Tree::Armory, blade);
    let mut c = crate::render::armory::hero_weapon(w.form, w.color, false);
    // Drop the fist (skin), keep the weapon.
    c.voxels.retain(|v| v.c >> 24 != crate::render::characters::mat::SKIN >> 24);
    build_rig(&c, &[], VOXEL / 2.0)
}

impl WorldView {
    /// A crackling arc from `from` to `to` (tile positions), at chest height.
    pub fn zap(&mut self, grid: &Grid, from: crate::logic::types::V, to: crate::logic::types::V) {
        let lift = |p: crate::logic::types::V| Vec3::new(p.x as f32, floor_at(grid, p.x as f32, p.y as f32) + 0.6, p.y as f32);
        let seed = self.rng.nextf() * 100.0;
        self.zaps.push(Zap { a: lift(from), b: lift(to), life: ZAP_SECONDS, seed });
        self.sparkle(grid, to.x, to.y, 0.6, 0xbfe8ff, 10);
    }

    /// Rebuilds the hero's rig when his weapon or shield changes.
    pub(super) fn refit_hero(&mut self, hero: HeroKind, h: &Hero) {
        let key = (hero, h.blade, h.shield.map(|s| s.tier));
        if key != self.models.hero_key {
            self.models.hero = hero_model(key.0, key.1, key.2);
            self.models.thrown = thrown_model(key.0, key.1);
            self.models.hero_key = key;
        }
    }

    pub(super) fn update_forge_fx(&mut self, dt: f32, hero: &Hero) {
        self.zaps.retain_mut(|z| {
            z.life -= dt;
            z.life > 0.0
        });
        // Swiftness leaves a trail of fading afterimages.
        self.ghost_clock += dt;
        let swift = hero.hasted() || hero.fx.haste > 0.0 || hero.fx.ragnarok > 0.0;
        if swift && hero.motion > 0.3 && self.ghost_clock > 0.05 {
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

    /// The hero's rig for the weapon and shield he has right now.
    pub(super) fn hero_rig(&self, _hero: &Hero) -> &Rig {
        &self.models.hero
    }

    /// Auras and trails of the elixirs he has drunk and the skills at work, and the lightning arcs.
    pub(super) fn emit_hero_fx(&self, hero: &Hero, pos: Vec3, time: f32, f: &mut Frame) {
        let cube = MeshKind::Cube as usize;
        let spark = |f: &mut Frame, p: Vec3, size: f32, c: u32, a: f32| {
            f.additive.push(Instance::new(Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(size)), lin(c) * 1.5, a, Vec3::ZERO, SHADE_UNLIT));
        };
        if hero.alive {
            // Afterimages from haste.
            let rig = self.hero_rig(hero);
            for (p, yaw, life) in &self.ghosts {
                let root = Mat4::from_translation(*p) * Mat4::from_rotation_y(*yaw);
                emit_rig(&mut f.transparent[cube], rig, &self.hero_pose, root, lin(0xffe066), 0.28 * life, lin(0xffd23a) * 0.4);
            }
            let fx = &hero.fx;
            if fx.strength > 0.0 || fx.fury > 0.0 {
                let pulse = 0.7 + (time * 6.0).sin() * 0.3;
                let c = if fx.fury > 0.0 { 0xff4a8a } else { 0xff3a2a };
                f.lights.push(Light { pos: pos + Vec3::new(0.0, 0.8, 0.0), color: lin(c) * 3.0 * pulse, distance: 3.0 });
                for k in 0..6 {
                    let a = time * 2.5 + k as f32 * TAU / 6.0;
                    let rise = (time * 1.3 + k as f32 * 0.37).fract();
                    spark(f, pos + Vec3::new(a.cos() * 0.4, 0.1 + rise * 1.1, a.sin() * 0.4), 0.05, c, 0.8 * (1.0 - rise));
                }
            }
            if fx.spirit > 0.0 {
                for k in 0..5 {
                    let rise = (time * 0.8 + k as f32 * 0.2).fract();
                    let a = k as f32 * 2.4;
                    let p = pos + Vec3::new(a.cos() * 0.3, 0.2 + rise * 1.2, a.sin() * 0.3);
                    for s in [Vec3::new(0.09, 0.03, 0.03), Vec3::new(0.03, 0.09, 0.03)] {
                        f.additive.push(Instance::new(Mat4::from_translation(p) * Mat4::from_scale(s), lin(0xa6f0ff), 0.8 * (1.0 - rise), Vec3::ZERO, SHADE_UNLIT));
                    }
                }
            }
            if fx.fire > 0.0 {
                // Dragonfire: flames licking up all round him.
                for k in 0..10 {
                    let a = k as f32 * TAU / 10.0 + time * 1.5;
                    let rise = (time * 2.2 + k as f32 * 0.31).fract();
                    spark(f, pos + Vec3::new(a.cos() * 0.42, rise * 0.9, a.sin() * 0.42), 0.07 * (1.0 - rise * 0.5), if k % 2 == 0 { 0xff7a2a } else { 0xffd23a }, 0.85 * (1.0 - rise));
                }
                f.lights.push(Light { pos: pos + Vec3::new(0.0, 0.5, 0.0), color: lin(0xff7a2a) * 4.0, distance: 3.5 });
            }
            if fx.ragnarok > 0.0 || hero.heart > 0.0 {
                // The end of the world crackling over him, or the forge-heart's embers.
                let c = if fx.ragnarok > 0.0 { 0xb07aff } else { 0xff9a3a };
                for k in 0..3 {
                    let t = (time * 9.0 + k as f32 * 3.1).floor();
                    let h = |n: f32| (t * 12.9898 + n * 78.233).sin().fract().abs();
                    let a = pos + Vec3::new(h(1.0) - 0.5, 0.3 + h(2.0) * 0.9, h(3.0) - 0.5) * Vec3::new(0.7, 1.0, 0.7);
                    let b = a + Vec3::new(h(4.0) - 0.5, h(5.0) - 0.5, h(6.0) - 0.5) * 0.35;
                    f.additive.push(Instance::new(segment(a, b, 0.025), lin(c) * 1.6, 0.9, Vec3::ZERO, SHADE_UNLIT));
                }
            }
            if hero.bark > 0 {
                // A shield potion's hardened skin: a faint shell round him.
                let m = Mat4::from_translation(pos + Vec3::new(0.0, 0.5, 0.0)) * Mat4::from_scale(Vec3::new(0.45, 0.6, 0.45));
                f.transparent[MeshKind::Sphere as usize].push(Instance::new(m, lin(0x9aa4b0), 0.12 + 0.04 * (time * 3.0).sin(), Vec3::ZERO, SHADE_UNLIT));
            }
            if hero.blade >= progression::SEALED_FROM {
                // A sealed blade hums with its own light.
                let n = progression::node(self.models.hero_key.0, Tree::Armory, hero.blade);
                let fl = 0.8 + 0.2 * (time * 5.0).sin();
                f.lights.push(Light { pos: pos + Vec3::new(0.0, 1.0, 0.0), color: lin(n.color) * 2.2 * fl, distance: 2.6 });
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

    /// A decoy of Loki's trick: a flickering violet phantom of the hero where he stood.
    pub(super) fn emit_decoy(&self, at: Vec3, time: f32, f: &mut Frame) {
        let cube = MeshKind::Cube as usize;
        let root = Mat4::from_translation(at) * Mat4::from_rotation_y(self.hero_yaw + (time * 0.8).sin() * 0.3);
        let a = 0.35 + 0.1 * (time * 9.0).sin();
        emit_rig(&mut f.transparent[cube], &self.models.hero, &Pose::rest(&self.models.hero), root, lin(0xc89aff), a, lin(0x9a4aff) * 0.6);
        f.lights.push(Light { pos: at + Vec3::new(0.0, 0.9, 0.0), color: lin(0x9a4aff) * 3.0, distance: 3.0 });
    }

    /// One shot of the hero's own: a thrown armory weapon, or a tier-3 blade's wave.
    pub(super) fn emit_forged_shot(&self, kind: RangedKind, pos: Vec3, yaw: f32, time: f32, f: &mut Frame) -> bool {
        let cube = MeshKind::Cube as usize;
        let group = Mat4::from_translation(pos) * Mat4::from_rotation_y(yaw);
        match kind {
            RangedKind::Hurled(tier) => {
                let rig = &self.models.thrown;
                // A spear flies point first; blades and hammers tumble end over end.
                let spin = if progression::blade(tier).throw.map(|t| t.homing).unwrap_or(false) {
                    group * Mat4::from_rotation_x(PI / 2.0) * Mat4::from_translation(Vec3::new(0.0, -0.9, 0.0))
                } else {
                    group * Mat4::from_rotation_x(time * 18.0) * Mat4::from_translation(Vec3::new(0.0, -0.55, 0.0))
                };
                emit_rig(&mut f.opaque[cube], rig, &Pose::rest(rig), spin * Mat4::from_translation(Vec3::new(-0.13, 0.0, 0.0)), Vec3::ONE, 1.0, Vec3::ZERO);
                let n = progression::node(self.models.hero_key.0, Tree::Armory, tier);
                if tier >= progression::SEALED_FROM {
                    f.lights.push(Light { pos, color: lin(n.color) * 4.0, distance: 3.0 });
                    glow_orb(f, pos, 0.5, lin(n.color), 0.3);
                }
                if progression::blade(tier).chain > 0 {
                    for k in 0..3 {
                        let t = (time * 14.0 + k as f32 * 3.1).floor();
                        let h = |q: f32| (t * 12.9898 + q * 78.233).sin().fract().abs();
                        let a = pos + Vec3::new(h(1.0) - 0.5, h(2.0) - 0.5, h(3.0) - 0.5) * 0.6;
                        let b = a + Vec3::new(h(4.0) - 0.5, h(5.0) - 0.5, h(6.0) - 0.5) * 0.4;
                        f.additive.push(Instance::new(segment(a, b, 0.025), lin(0xbfe8ff) * 1.8, 0.9, Vec3::ZERO, SHADE_UNLIT));
                    }
                }
            }
            RangedKind::Wave => {
                // A pale crescent of force skimming the floor.
                let pulse = 1.0 + (time * 20.0).sin() * 0.08;
                let m = group * Mat4::from_rotation_x(-PI / 2.0) * Mat4::from_scale(Vec3::new(0.55 * pulse, 0.55 * pulse, 1.0));
                f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, lin(0x9ad1ff) * 1.8, 0.55, Vec3::ZERO, SHADE_UNLIT));
                emit_mesh(&mut f.transparent[cube], &self.models.crescent, group, lin(0x9ad1ff), 0.7, lin(0x9ad1ff) * 0.8);
                f.lights.push(Light { pos, color: lin(0x9ad1ff) * 3.0, distance: 2.8 });
            }
            _ => return false,
        }
        true
    }
}
