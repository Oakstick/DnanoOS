//! How the hero's gear looks in use: the rig of the weapon and shield he
//! carries (rebuilt whenever either changes), his thrown armory weapons in
//! flight, the auras of his elixirs and skills, lightning arcs, decoys and
//! blinks.

use super::*;
use crate::logic::angels::Archangel;
use crate::logic::heroes::HeroKind;
use crate::logic::items::RangedKind;
use crate::logic::progression::{self, Tree};
use crate::logic::types::V;
use crate::render::characters::mat::SKIN;
use crate::render::voxels::Collected;
use crate::render::{armory, heroes};

/// The heroes' voxels: a quarter of a tile's voxel, as fine as Scene I's cast.
const HERO_VOXEL: f32 = VOXEL / 4.0;
const ZAP_SECONDS: f32 = 0.22;
const GHOSTS: usize = 4;
/// How long an arrow of a rain takes to fall.
const RAIN_SECONDS: f32 = 0.28;
/// How long a sun-blade's flash takes to fade.
const FLASH_SECONDS: f32 = 0.6;

/// An arrow of a heavenly bow, falling from `top` to `bottom`.
pub(super) struct Falling {
    pub top: Vec3,
    pub bottom: Vec3,
    pub life: f32,
    pub color: Vec3,
}

/// A lightning arc between two points, flickering while it lasts.
pub(super) struct Zap {
    pub a: Vec3,
    pub b: Vec3,
    pub life: f32,
    pub seed: f32,
}

/// A line segment as a stretched cube from `a` to `b`.
pub(super) fn segment(a: Vec3, b: Vec3, thick: f32) -> Mat4 {
    let d = b - a;
    let len = d.length().max(1e-4);
    let rot = Quat::from_rotation_arc(Vec3::Y, d / len);
    Mat4::from_scale_rotation_translation(Vec3::new(thick, len, thick), rot, (a + b) * 0.5)
}

/// How a hero is dressed and armed for his model: his blade's tier, the shield on his arm, his
/// outfit and the skills of his path (ribbons on his arm). (Rust is a tint, see `emit_hero_rig`.)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct Kit {
    pub blade: u8,
    pub shield: Option<u8>,
    pub outfit: u8,
    pub path: u8,
}

impl Kit {
    pub fn plain(blade: u8, shield: Option<u8>) -> Self {
        Self { blade, shield, outfit: 0, path: 0 }
    }

    /// The hero as he stands.
    pub fn of(h: &Hero) -> Self {
        Self { blade: h.blade, shield: h.shield.map(|s| s.tier), outfit: h.outfit, path: h.path }
    }
}

/// A hero's sculpted body for `kit` (dressed, beribboned, the shield on his back if any) and his rig's parts.
fn hero_body(hero: HeroKind, kit: Kit) -> (Collected, &'static [PartDef]) {
    let spec = kit.shield.map(|t| {
        let n = progression::node(hero, Tree::Shield, t);
        (n.form, n.color, t)
    });
    let (mut body, defs) = match hero {
        HeroKind::Ragnar => (heroes::ragnar_hd(spec), heroes::RAGNAR_HD_RIG),
        HeroKind::Mehmed => (heroes::mehmed_hd(spec), heroes::MEHMED_HD_RIG),
        HeroKind::Miyamoto => (heroes::miyamoto_hd(spec), heroes::MIYAMOTO_HD_RIG),
    };
    let shoulder = defs.iter().find(|d| d.name == "shieldArm").map(|d| d.pivot[1]).unwrap_or(0.0);
    heroes::dress(&mut body, hero, kit.outfit);
    let colors: Vec<u32> = (1..=kit.path.min(progression::TIERS)).map(|t| progression::node(hero, Tree::Path, t).color).collect();
    heroes::ribbons(&mut body, shoulder, &colors);
    (body, defs)
}

pub(super) fn hero_model(hero: HeroKind, blade: u8, shield: Option<u8>) -> Rig {
    hero_rigs(hero, Kit::plain(blade, shield)).0
}

/// The middle of each hero's shield forearm (x, y, z) and its girth there, in voxels of his own
/// grid as he is sculpted: where his shield is strapped.
fn forearm(hero: HeroKind) -> (f32, f32, f32, f32) {
    match hero {
        HeroKind::Ragnar => (7.5, 40.0, 17.0, 5.0),
        HeroKind::Mehmed => (21.5, 36.0, 19.5, 5.8),
        HeroKind::Miyamoto => (21.5, 34.0, 20.5, 5.2),
    }
}

/// How thick a shield is carved, in voxels (its face, and the rim and boss standing out of it).
const SHIELD_THICK: i32 = 4;
/// A shield on his arm is a little smaller than the one that hung on his back.
const STRAPPED_SCALE: f32 = 0.8;

/// The shield as it is carved into his back, on its own: the voxels it adds to the bare body (and
/// not those of his back that it shades).
fn back_shield(with: &Collected, bare: &Collected) -> Collected {
    let plain: std::collections::HashSet<(i32, i32, i32, u32)> = bare.voxels.iter().map(|v| (v.x, v.y, v.z, v.c)).collect();
    let added: Vec<Voxel> =
        with.voxels.iter().filter(|v| !plain.contains(&(v.x, v.y, v.z, v.c))).map(|v| Voxel { part: 0, ..*v }).collect();
    let out = added.iter().map(|v| v.z).min().unwrap_or(0);
    let voxels = added.into_iter().filter(|v| v.z < out + SHIELD_THICK).collect();
    Collected { voxels, width: with.width, depth: with.depth, height: with.height }
}

/// The shield off his back and strapped to his shield forearm, a little smaller: turned edge-on
/// to his front, its face looking out from his side, the inside of it against the arm.
fn strapped(back: &Collected, hero: HeroKind, scale: f32) -> Collected {
    let (ax, ay, az, girth) = forearm(hero);
    let span = |f: fn(&Voxel) -> i32| {
        let lo = back.voxels.iter().map(f).min().unwrap_or(0);
        let hi = back.voxels.iter().map(f).max().unwrap_or(0);
        (lo, hi)
    };
    let (x0, x1) = span(|v| v.x);
    let (y0, y1) = span(|v| v.y);
    let (_, inner) = span(|v| v.z);
    let (cx, cy) = ((x0 + x1) as f32 / 2.0, (y0 + y1) as f32 / 2.0);
    let at: std::collections::HashMap<(i32, i32, i32), Voxel> = back.voxels.iter().map(|v| ((v.x, v.y, v.z), *v)).collect();
    let against = (ax - girth - 0.5).round() as i32;
    // Each voxel of the smaller shield takes the colour of the one it shrank from (its face
    // across his front and back, its height up his arm, its thickness out from his side).
    let reach = |lo: i32, hi: i32, c: f32| ((lo as f32 - c) * scale).floor() as i32..=((hi as f32 - c) * scale).ceil() as i32;
    let mut voxels = Vec::new();
    for dy in reach(y0, y1, cy) {
        for dx in reach(x0, x1, cx) {
            let (sx, sy) = ((cx + dx as f32 / scale).round() as i32, (cy + dy as f32 / scale).round() as i32);
            for z in inner + 1 - SHIELD_THICK..=inner {
                if let Some(v) = at.get(&(sx, sy, z)) {
                    voxels.push(Voxel {
                        x: against - (inner - z),
                        y: (ay + dy as f32).round() as i32,
                        z: (az + dx as f32).round() as i32,
                        ..*v
                    });
                }
            }
        }
    }
    Collected { voxels, ..*back }
}

/// Turns a model of `width` left for right, so what he holds in his model's right fist (at its
/// high x) is in his own right hand as he faces out of it.
fn mirror(c: &mut Collected, width: i32) {
    for v in c.voxels.iter_mut() {
        v.x = width - 1 - v.x;
    }
}

/**
 * The detailed hero as he is played, and (if he carries a shield) as he holds it up before him.
 * He has a weapon of his armory in his right hand (and its twin in his left, for a pair) and the
 * shield of his tree on his left forearm. The weapons and the shield are layered parts ("blade",
 * "offhand", "shield") that follow the arms, pivoting where the arms do. Raised, he is drawn
 * without it, and the shield on its own as it hung on his back, to be turned and set before him.
 */
pub(super) fn hero_rigs(hero: HeroKind, kit: Kit) -> (Rig, Option<(Rig, Rig)>) {
    hero_rigs_strapped(hero, kit, STRAPPED_SCALE)
}

/// `hero_rigs` with the shield on his arm at `strap` of its size on his back.
fn hero_rigs_strapped(hero: HeroKind, kit: Kit, strap: f32) -> (Rig, Option<(Rig, Rig)>) {
    let w = progression::node(hero, Tree::Armory, kit.blade);
    let (mut body, defs) = hero_body(hero, Kit { shield: None, ..kit });
    let (width, depth, height) = (body.width, body.depth, body.height);
    let empty = || Collected { voxels: Vec::new(), width, depth, height };
    let mut back = kit.shield.map(|_| back_shield(&hero_body(hero, kit).0, &body));
    let mut arm = back.as_ref().map(|b| strapped(b, hero, strap)).unwrap_or_else(empty);
    let mut main = armory::weapon_for(hero, w.form, w.color, false);
    let mut off = if armory::paired(w.form) { armory::weapon_for(hero, w.form, w.color, true) } else { empty() };
    // He is sculpted with his weapon in the fist at the model's high x: his left as he faces out.
    for c in [&mut body, &mut arm, &mut main, &mut off].into_iter().chain(back.as_mut()) {
        mirror(c, width);
    }
    let defs: Vec<PartDef> = defs
        .iter()
        .map(|d| PartDef { name: d.name, boxes: d.boxes, pivot: [(width - 1) as f32 - d.pivot[0], d.pivot[1], d.pivot[2]] })
        .collect();
    let pivot = |name: &str| defs.iter().find(|d| d.name == name).map(|d| d.pivot).unwrap_or([0.0; 3]);
    let (sword, shield) = (pivot("swordArm"), pivot("shieldArm"));
    let rig = |arm: &Collected| {
        build_layered_rig(&body, &defs, &[("blade", &main, sword), ("offhand", &off, shield), ("shield", arm, shield)], HERO_VOXEL)
    };
    let raised = back.map(|b| (rig(&empty()), build_rig(&b, &[], HERO_VOXEL)));
    (rig(&arm), raised)
}

/// A hero's sculpt grid (width, height, depth), which his joint table is measured in.
fn sculpt_size(hero: HeroKind) -> (i32, i32, i32) {
    match hero {
        HeroKind::Ragnar => heroes::RAGNAR_SIZE,
        HeroKind::Mehmed => heroes::MEHMED_SIZE,
        HeroKind::Miyamoto => heroes::MIYAMOTO_SIZE,
    }
}

/// The shield on his arm as his clips carry it: as big as it hung on his back (they raise it before
/// him to block, where the code swaps in the shield off his back, `WorldView::emit`).
const JOINTED_STRAP: f32 = 1.0;

/// The hero dressed and armed for `kit` cut into his skeleton's bones, if he has a joint table
/// (`anim::skeleton`). With `VH_ANIM=bones` every voxel is painted its bone's colour, to check the
/// cut.
pub(super) fn jointed_rig(hero: HeroKind, kit: Kit) -> Option<Rig> {
    crate::render::anim::skeleton::joints(hero)?;
    jointed_of(hero, &hero_rigs_strapped(hero, kit, JOINTED_STRAP).0)
}

/// The hero's rig `rig` (as `hero_rigs` makes it) cut into his skeleton's bones.
fn jointed_of(hero: HeroKind, rig: &Rig) -> Option<Rig> {
    use crate::render::anim::{library::Mode, skeleton};
    let joints = skeleton::joints(hero)?;
    let (w, _, d) = sculpt_size(hero);
    let mut cut = skeleton::jointed(rig, joints, w, d);
    if Mode::from_env() == Mode::Bones {
        for (i, part) in cut.parts.iter_mut().enumerate() {
            let c = skeleton::bone_color(i);
            for (k, (_, color)) in part.cubes.iter_mut().enumerate() {
                // (A little of the voxel's own shade kept, so the shapes still read.)
                *color = c * (0.7 + 0.3 * (k % 3) as f32 / 2.0);
            }
            part.shade.iter_mut().for_each(|m| *m = 0.0);
        }
    }
    Some(cut)
}

/// The jointed hero as he stands with blade `blade` and shield `shield` (`--export-rig`, tests).
pub fn jointed_hero(hero: HeroKind, blade: u8, shield: Option<u8>) -> Option<Rig> {
    jointed_rig(hero, Kit::plain(blade, shield))
}

/// A thrown armory weapon on its own, for its flight: the model without the fist, centred.
pub(super) fn thrown_model(hero: HeroKind, blade: u8) -> Rig {
    let w = progression::node(hero, Tree::Armory, blade);
    let mut c = armory::hero_weapon(w.form, w.color, false);
    // Drop the fist (skin), keep the weapon.
    c.voxels.retain(|v| v.c >> 24 != SKIN >> 24);
    build_rig(&c, &[], HERO_VOXEL)
}

impl WorldView {
    /// A crackling arc from `from` to `to` (tile positions), at chest height.
    pub fn zap(&mut self, grid: &Grid, from: V, to: V) {
        let lift = |p: V| Vec3::new(p.x as f32, floor_at(grid, p.x as f32, p.y as f32) + 0.6, p.y as f32);
        let seed = self.rng.nextf() * 100.0;
        self.zaps.push(Zap { a: lift(from), b: lift(to), life: ZAP_SECONDS, seed });
        self.sparkle(grid, to.x, to.y, 0.6, 0xbfe8ff, 10);
    }

    /// Rebuilds the hero's rig when he, his weapon or his shield changes.
    /// How many voxels the hero's model has (for tests: an outfit or ribbons rebuild it).
    #[cfg(test)]
    pub fn hero_voxels(&self) -> usize {
        self.models.hero.parts.iter().map(|p| p.cubes.len()).sum()
    }

    pub(super) fn refit_hero(&mut self, hero: HeroKind, h: &Hero) {
        let key = (hero, Kit::of(h));
        if key != self.models.hero_key {
            match self.models.spare.take() {
                // His shield broke: the rig made for it with the level (no shield, none raised).
                Some((k, rig, jointed)) if k == key => {
                    self.models.hero = rig;
                    self.models.raised = None;
                    self.models.jointed = jointed;
                }
                _ => {
                    (self.models.hero, self.models.raised) = hero_rigs(key.0, key.1);
                    self.models.jointed = jointed_rig(key.0, key.1);
                }
            }
            // (His weapon thrown is only made again for another weapon.)
            if (key.0, key.1.blade) != (self.models.hero_key.0, self.models.hero_key.1.blade) {
                self.models.thrown = thrown_model(key.0, key.1.blade);
            }
            if key.0 != self.models.hero_key.0 {
                // Another hero's joints sit elsewhere: start his pose afresh.
                self.hero_pose = Pose::rest(&self.models.hero);
                self.hero_anim.reset();
            }
            self.models.hero_key = key;
        }
    }

    /// Makes, with the level, the hero as a broken shield would leave him (kept for `refit_hero`),
    /// unless he has none to break.
    pub(super) fn make_spare_hero(&mut self) {
        let (hero, kit) = self.models.hero_key;
        if kit.shield.is_none() {
            self.models.spare = None;
            return;
        }
        let key = (hero, Kit { shield: None, ..kit });
        if self.models.spare.as_ref().is_none_or(|s| s.0 != key) {
            let rig = hero_rigs(key.0, key.1).0;
            let jointed = jointed_rig(key.0, key.1);
            self.models.spare = Some((key, rig, jointed));
        }
    }

    /// Arrows raining out of the sky onto `at` (a tile position), in `color`.
    pub fn rain(&mut self, grid: &Grid, at: V, color: u32) {
        let ground = Vec3::new(at.x as f32, floor_at(grid, at.x as f32, at.y as f32) + 0.3, at.y as f32);
        for k in 0..3 {
            let off = Vec3::new((self.rng.nextf() - 0.5) * 0.5, 0.0, (self.rng.nextf() - 0.5) * 0.5);
            self.rains.push(Falling {
                top: ground + off + Vec3::new(0.6, 7.0, 0.3),
                bottom: ground + off,
                life: RAIN_SECONDS + k as f32 * 0.06,
                color: lin(color),
            });
        }
        self.sparkle(grid, at.x, at.y, 0.4, color, 12);
    }

    /// A blinding flash of light at `at` (a sun-blade's), fading out over a moment.
    pub fn flash_light(&mut self, grid: &Grid, at: V, color: u32) {
        let p = Vec3::new(at.x as f32, floor_at(grid, at.x as f32, at.y as f32) + 1.0, at.y as f32);
        self.flashes.push((p, 1.0, lin(color)));
    }

    pub(super) fn update_forge_fx(&mut self, dt: f32, hero: &Hero) {
        self.zaps.retain_mut(|z| {
            z.life -= dt;
            z.life > 0.0
        });
        self.rains.retain_mut(|r| {
            r.life -= dt;
            r.life > -0.15
        });
        for f in self.flashes.iter_mut() {
            f.1 -= dt / FLASH_SECONDS;
        }
        self.flashes.retain(|f| f.1 > 0.0);
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
        let spark = |f: &mut Frame, p: Vec3, size: f32, c: u32, a: f32| {
            f.additive.push(Instance::new(
                Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(size)),
                lin(c) * 1.5,
                a,
                Vec3::ZERO,
                SHADE_UNLIT,
            ));
        };
        if hero.alive {
            // Afterimages from haste.
            let rig = self.hero_rig(hero);
            for (p, yaw, life) in &self.ghosts {
                let root = Mat4::from_translation(*p) * Mat4::from_rotation_y(*yaw);
                let (tint, alpha, glow) = (lin(0xffe066), 0.28 * life, lin(0xffd23a) * 0.4);
                match self.clip_pose() {
                    Some((jointed, mats)) => f.rigs_blend.push_matrices(jointed, mats, root, tint, alpha, glow, rig_look::TINT_ALL, 0.0),
                    None => emit_rig(&mut f.rigs_blend, rig, &self.hero_pose, root, tint, alpha, glow),
                }
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
                        f.additive.push(Instance::new(
                            Mat4::from_translation(p) * Mat4::from_scale(s),
                            lin(0xa6f0ff),
                            0.8 * (1.0 - rise),
                            Vec3::ZERO,
                            SHADE_UNLIT,
                        ));
                    }
                }
            }
            if fx.fire > 0.0 {
                // Dragonfire: flames licking up all round him.
                for k in 0..10 {
                    let a = k as f32 * TAU / 10.0 + time * 1.5;
                    let rise = (time * 2.2 + k as f32 * 0.31).fract();
                    spark(
                        f,
                        pos + Vec3::new(a.cos() * 0.42, rise * 0.9, a.sin() * 0.42),
                        0.07 * (1.0 - rise * 0.5),
                        if k % 2 == 0 { 0xff7a2a } else { 0xffd23a },
                        0.85 * (1.0 - rise),
                    );
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
                f.transparent[MeshKind::Sphere as usize].push(Instance::new(
                    m,
                    lin(0x9aa4b0),
                    0.12 + 0.04 * (time * 3.0).sin(),
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
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
        for r in &self.rains {
            // Each arrow a bright streak dropping to the ground, then a spark where it struck.
            let u = (1.0 - r.life / RAIN_SECONDS).clamp(0.0, 1.0);
            let tip = r.top.lerp(r.bottom, u);
            let tail = r.top.lerp(r.bottom, (u - 0.25).max(0.0));
            if r.life > 0.0 {
                f.additive.push(Instance::new(segment(tail, tip, 0.05), r.color * 1.8, 0.95, Vec3::ZERO, SHADE_UNLIT));
                f.additive.push(Instance::new(segment(tail, tip, 0.14), r.color, 0.3, Vec3::ZERO, SHADE_UNLIT));
            } else {
                glow_orb(f, r.bottom, 0.5, r.color, 0.5);
            }
        }
        for &(p, k, c) in &self.flashes {
            glow_orb(f, p, 1.5 + (1.0 - k) * 4.5, c, 0.9 * k);
            f.lights.push(Light { pos: p, color: c * 40.0 * k, distance: 9.0 });
        }
    }

    /**
     * The ancestral spirits at the hero's side (path 9 of Mehmed and Miyamoto):
     * pale, see-through warriors of his own people, their weapons raised as they strike.
     */
    pub(super) fn emit_spirits(&self, s: &SceneState, time: f32, f: &mut Frame) {
        let rig = match s.hero_kind {
            HeroKind::Mehmed => &self.models.spirits[0],
            HeroKind::Miyamoto => &self.models.spirits[1],
            HeroKind::Ragnar => return,
        };
        let hp = s.hero.pos();
        for (i, sp) in s.spirits.iter().enumerate() {
            let (x, z) = (sp.pos.x as f32, sp.pos.y as f32);
            let bob = (time * 2.0 + i as f32 * 1.7).sin() * 0.08;
            let p = Vec3::new(x, floor_at(s.grid, x, z) + 0.25 + bob, z);
            let yaw = ((hp.x - sp.pos.x) as f32).atan2((hp.y - sp.pos.y) as f32) + PI * 0.5 * (1.0 - sp.strike as f32);
            let lunge = sp.strike as f32 * 0.25;
            let root = Mat4::from_translation(p)
                * Mat4::from_rotation_y(yaw)
                * Mat4::from_translation(Vec3::new(0.0, 0.0, lunge))
                * Mat4::from_scale(Vec3::splat(0.8));
            let a = 0.32 + 0.08 * (time * 5.0 + i as f32).sin();
            emit_rig(&mut f.rigs_blend, rig, &Pose::rest(rig), root, lin(0xa8c8ff), a, lin(0x6a9aff) * 0.5);
            glow_orb(f, p + Vec3::new(0.0, 0.8, 0.0), 0.7, lin(0x8ab0ff), 0.18);
        }
    }

    /// A greatsword's holy fire, burning on the tiles it was laid on.
    pub(super) fn emit_fires(&self, s: &SceneState, time: f32, f: &mut Frame) {
        for (n, &(t, left)) in s.fires.iter().enumerate() {
            let k = (left as f32).min(1.0);
            let base = Vec3::new(t.x as f32, floor_at(s.grid, t.x as f32, t.y as f32), t.y as f32);
            for j in 0..7 {
                let a = j as f32 * 0.9 + n as f32;
                let rise = (time * 2.4 + j as f32 * 0.19 + n as f32 * 0.37).fract();
                let p = base + Vec3::new(a.cos() * 0.3 * (1.0 - rise), rise * 0.8, a.sin() * 0.3 * (1.0 - rise));
                let c = if j % 3 == 0 { 0xfff1a8 } else { 0xff9a3a };
                f.additive.push(Instance::new(
                    Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.09 * (1.0 - rise * 0.6))),
                    lin(c) * 1.6,
                    0.9 * (1.0 - rise) * k,
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
            glow_orb(f, base + Vec3::new(0.0, 0.3, 0.0), 0.6, lin(0xffb050), 0.35 * k);
            if n % 2 == 0 {
                f.lights.push(Light { pos: base + Vec3::new(0.0, 0.5, 0.0), color: lin(0xffa040) * 4.0 * k, distance: 3.0 });
            }
        }
    }

    /// A decoy of Loki's trick: a flickering violet phantom of the hero where he stood.
    pub(super) fn emit_decoy(&self, at: Vec3, time: f32, f: &mut Frame) {
        let root = Mat4::from_translation(at) * Mat4::from_rotation_y(self.hero_yaw + (time * 0.8).sin() * 0.3);
        let a = 0.35 + 0.1 * (time * 9.0).sin();
        emit_rig(&mut f.rigs_blend, &self.models.hero, &Pose::rest(&self.models.hero), root, lin(0xc89aff), a, lin(0x9a4aff) * 0.6);
        f.lights.push(Light { pos: at + Vec3::new(0.0, 0.9, 0.0), color: lin(0x9a4aff) * 3.0, distance: 3.0 });
    }

    /// One shot of the hero's own: a thrown armory weapon, or a tier-3 blade's wave.
    pub(super) fn emit_forged_shot(&self, kind: RangedKind, pos: Vec3, yaw: f32, time: f32, f: &mut Frame) -> bool {
        let cube = MeshKind::Cube as usize;
        let group = Mat4::from_translation(pos) * Mat4::from_rotation_y(yaw);
        match kind {
            RangedKind::Hurled(hero, tier) if progression::node(hero, Tree::Armory, tier).form == progression::Form::Gun => {
                // A lead ball, glowing hot, trailing powder smoke.
                let m = Mat4::from_translation(pos) * Mat4::from_scale(Vec3::splat(0.07));
                f.opaque[MeshKind::Sphere as usize].push(Instance::new(m, lin(0x6a6a70), 1.0, lin(0xffa050) * 0.8, SHADE_LIT));
                let trail = group * Mat4::from_translation(Vec3::new(0.0, 0.0, -0.35)) * Mat4::from_scale(Vec3::new(0.05, 0.05, 0.7));
                f.transparent[cube].push(Instance::new(trail, lin(0xc8c8d0), 0.35, Vec3::ZERO, SHADE_UNLIT));
            }
            RangedKind::Hurled(hero, tier) => {
                let rig = &self.models.thrown;
                // A spear flies point first; blades and hammers tumble end over end.
                let spin = if progression::blade(hero, tier).throw.map(|t| t.homing).unwrap_or(false) {
                    group * Mat4::from_rotation_x(PI / 2.0) * Mat4::from_translation(Vec3::new(0.0, -0.9, 0.0))
                } else {
                    group * Mat4::from_rotation_x(time * 18.0) * Mat4::from_translation(Vec3::new(0.0, -0.55, 0.0))
                };
                emit_rig(
                    &mut f.rigs,
                    rig,
                    &Pose::rest(rig),
                    spin * Mat4::from_translation(Vec3::new(-0.13, 0.0, 0.0)),
                    Vec3::ONE,
                    1.0,
                    Vec3::ZERO,
                );
                let n = progression::node(hero, Tree::Armory, tier);
                if tier >= progression::SEALED_FROM {
                    f.lights.push(Light { pos, color: lin(n.color) * 4.0, distance: 3.0 });
                    glow_orb(f, pos, 0.5, lin(n.color), 0.3);
                }
                if progression::blade(hero, tier).chain > 0 {
                    for k in 0..3 {
                        let t = (time * 14.0 + k as f32 * 3.1).floor();
                        let h = |q: f32| (t * 12.9898 + q * 78.233).sin().fract().abs();
                        let a = pos + Vec3::new(h(1.0) - 0.5, h(2.0) - 0.5, h(3.0) - 0.5) * 0.6;
                        let b = a + Vec3::new(h(4.0) - 0.5, h(5.0) - 0.5, h(6.0) - 0.5) * 0.4;
                        f.additive.push(Instance::new(segment(a, b, 0.025), lin(0xbfe8ff) * 1.8, 0.9, Vec3::ZERO, SHADE_UNLIT));
                    }
                }
            }
            RangedKind::Grace => {
                // Michael's fire: a blue flame with a white heart, sparks streaming off it.
                let blue = lin(Archangel::Michael.color());
                let pulse = 1.0 + (time * 18.0).sin() * 0.12;
                glow_orb(f, pos, 0.55 * pulse, blue, 0.75);
                let m = Mat4::from_translation(pos)
                    * Mat4::from_rotation_y(time * 9.0)
                    * Mat4::from_rotation_x(time * 7.0)
                    * Mat4::from_scale(Vec3::splat(0.14 * pulse));
                f.additive.push(Instance::new(m, Vec3::ONE.lerp(blue, 0.3) * 2.4, 0.95, Vec3::ZERO, SHADE_UNLIT));
                for k in 0..4 {
                    let t = (time * 16.0 + k as f32 * 2.3).floor();
                    let h = |q: f32| (t * 12.9898 + q * 78.233).sin().fract() - 0.5;
                    let a = pos + Vec3::new(h(1.0), h(2.0), h(3.0)) * 0.35;
                    f.additive.push(Instance::new(
                        segment(a, a + Vec3::new(h(4.0), h(5.0), h(6.0)) * 0.2, 0.03),
                        blue * 2.0,
                        0.9,
                        Vec3::ZERO,
                        SHADE_UNLIT,
                    ));
                }
                f.lights.push(Light { pos, color: blue * 5.0, distance: 3.2 });
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
