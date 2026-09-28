//! Scene I on screen. Two sets stand far from the level and the other stages.
//! The first is the edge of an old graveyard at dusk: a stone wall with a
//! wrought-iron gate between two pillars, graves and bare trees in the fog
//! behind it, a chapel beyond, and a lantern on the gate post. The second is
//! the dungeon gateway at the top of the underworld: wet flagstones, torches
//! on the pillars, the iron door under a carved skull, the old man's stool
//! and lantern, Quitmer's straw and chain, and a stair going down behind the
//! door. The cast plays out each beat on them and the camera cuts from shot
//! to shot.

use super::*;
use crate::logic::story::{first, stage, Place, Shot, Speaker, SCENE_ONE};
use crate::render::cast;
use crate::render::voxels::PartDef;

/// Where the two sets stand, well away from everything else.
pub(super) const GRAVEYARD: Vec3 = Vec3::new(-160.0, 0.0, -40.0);
pub(super) const GATEWAY: Vec3 = Vec3::new(-200.0, 0.0, -40.0);

/// Voxel sizes of the cast, in world units.
pub(super) const KID_VOXEL: f32 = 0.0115;
pub(super) const OLD_MAN_VOXEL: f32 = 0.012;
pub(super) const DOG_VOXEL: f32 = 0.013;
pub(super) const KRAMPUS_VOXEL: f32 = 0.015;

/// Material shade modes for plain blocks (5 + the material's number).
const SHADE_METAL: f32 = 6.0;
const SHADE_CLOTH: f32 = 8.0;
const SHADE_WOOD: f32 = 12.0;

/// What the story shows this frame: the beat, seconds into it, seconds since the scene
/// began, how much of the line has typed, and which of the warriors' souls have risen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StoryView {
    pub beat: usize,
    pub t: f32,
    pub age: f32,
    pub shown: f32,
    pub souls: u8,
}

impl StoryView {
    fn shot(&self) -> Shot {
        SCENE_ONE[self.beat.min(SCENE_ONE.len() - 1)].shot
    }

    fn speaker(&self) -> Speaker {
        SCENE_ONE[self.beat.min(SCENE_ONE.len() - 1)].speaker
    }

    /// Seconds since the first beat of `shot` began: negative before it, large after it.
    fn since(&self, shot: Shot) -> f32 {
        match first(shot) {
            Some(i) if self.beat == i => self.t,
            Some(i) if self.beat > i => 1000.0,
            _ => -1.0,
        }
    }

    /// How many of the boys' lines have been said before this beat (who speaks next).
    fn boys_lines(&self) -> usize {
        self.said_before(Speaker::Boys)
    }

    /// How many lines `who` has spoken before this beat.
    fn said_before(&self, who: Speaker) -> usize {
        SCENE_ONE[..self.beat.min(SCENE_ONE.len())].iter().filter(|b| b.speaker == who).count()
    }
}

// ------------------------------------------------------------------ helpers

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn h(a: f64, b: f64) -> f32 {
    hash(a, b) as f32
}

fn block(out: &mut Vec<Instance>, p: Vec3, s: Vec3, rot: Quat, c: Vec3, shade: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(s, rot, p), c, 1.0, Vec3::ZERO, shade));
}

fn glowing(out: &mut Vec<Instance>, p: Vec3, s: Vec3, rot: Quat, c: Vec3, glow: Vec3) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(s, rot, p), c, 1.0, glow, SHADE_LIT));
}

/// The heading that looks from `from` toward `to`, for a model facing +z.
fn yaw_to(from: Vec3, to: Vec3) -> f32 {
    (to.x - from.x).atan2(to.z - from.z)
}

/// A point of a model's voxel grid, posed: `p` lies in part `part`, whose joint is at grid point `pivot`.
fn rig_point(rig: &Rig, pose: &Pose, root: Mat4, part: usize, pivot: [f32; 3], p: [f32; 3]) -> Vec3 {
    let local = (Vec3::from(p) - Vec3::from(pivot)) * rig.size;
    (root * Mat4::from_translation(pose.pos[part]) * euler(pose.rot[part])).transform_point3(local)
}

fn pivot_of(defs: &[PartDef], name: &str) -> [f32; 3] {
    defs.iter().find(|d| d.name == name).map(|d| d.pivot).unwrap_or([0.0; 3])
}

/// Like `emit_rig`, but screen-door faded by `alpha` (a shadow thinning into fog), all plain lit.
fn emit_rig_faded(out: &mut Vec<Instance>, rig: &Rig, pose: &Pose, root: Mat4, alpha: f32) {
    for (i, part) in rig.parts.iter().enumerate() {
        let pm = root * Mat4::from_translation(pose.pos[i]) * euler(pose.rot[i]);
        for (off, c) in &part.cubes {
            out.push(Instance::new(voxel_matrix(&pm, rig.size, *off), *c, alpha, Vec3::ZERO, SHADE_DITHER));
        }
    }
}

/// A warrior's soul: the model drawn see-through in glowing spirit light, its colours kept only as shading.
fn emit_rig_ghost(out: &mut Vec<Instance>, rig: &Rig, pose: &Pose, root: Mat4, tint: Vec3, alpha: f32) {
    for (i, part) in rig.parts.iter().enumerate() {
        let pm = root * Mat4::from_translation(pose.pos[i]) * euler(pose.rot[i]);
        for (off, c) in &part.cubes {
            let l = c.dot(Vec3::new(0.2126, 0.7152, 0.0722)).sqrt();
            out.push(Instance::new(voxel_matrix(&pm, rig.size, *off), tint * (0.2 + 0.55 * l), alpha, Vec3::ZERO, SHADE_UNLIT));
        }
    }
}

// ---------------------------------------------------------------- the sets

/// The graveyard's stone, iron, graves, trees and chapel: everything that stays put.
pub(super) fn graveyard_set() -> Vec<Instance> {
    let mut out = Vec::new();
    let at = |x: f32, y: f32, z: f32| GRAVEYARD + Vec3::new(x, y, z);
    let flat = Quat::IDENTITY;
    let iron = lin(0x1c1c20);

    // ---- turf outside the wall and in the churchyard, a trodden path up to the gate
    for gx in -15..=15 {
        for gz in -19..=9 {
            let k = h(gx as f64 * 1.3 + 0.5, gz as f64 * 2.1 + 3.0);
            let (x, z) = (gx as f32, gz as f32);
            let path = gx == 0 && gz >= -2;
            if path {
                block(&mut out, at(x, -0.16, z), Vec3::new(1.0, 0.3, 1.0), flat, lin(0x4a3c2c) * (0.85 + 0.25 * k), SHADE_LIT);
                continue;
            }
            let inside = z < -2.0;
            let c = if inside { lin(0x38422e) } else { lin(0x40502e) } * (0.78 + 0.4 * k);
            block(&mut out, at(x, -0.15 + (k - 0.5) * 0.04, z), Vec3::new(1.0, 0.3, 1.0), flat, c, SHADE_GRASS);
        }
    }
    // Flat stones along the path, sunk in the mud.
    for i in 0..16 {
        let (a, b) = (h(i as f64, 7.0), h(i as f64, 13.0));
        let z = 8.0 - i as f32 * 0.62;
        if z < -1.6 {
            break;
        }
        block(&mut out, at((a - 0.5) * 0.7, 0.005, z + (b - 0.5) * 0.2), Vec3::new(0.3 + a * 0.2, 0.04, 0.25 + b * 0.15), Quat::from_rotation_y(a * 3.0), lin(0x6a665e) * (0.8 + 0.3 * b), SHADE_LIT);
    }

    // ---- the churchyard wall, stone in courses with a coping on top, the gate gap in the middle
    for side in [-1.0f32, 1.0] {
        let mut x = 1.62;
        let mut n = 0;
        while x < 12.0 {
            for row in 0..4 {
                let k = h(x as f64 * 3.1 + side as f64, row as f64 * 1.7);
                let len = 0.42 + k * 0.22;
                let off = if row % 2 == 0 { 0.0 } else { 0.21 };
                let xc = side * (x + off + len / 2.0);
                let moss = row == 0 && k > 0.55;
                let c = if moss { lin(0x56603e) } else { lin(0x6e6a62) * (0.75 + 0.35 * k) };
                block(&mut out, at(xc, 0.125 + row as f32 * 0.235, -2.0 + (k - 0.5) * 0.02), Vec3::new(len - 0.02, 0.22, 0.38), flat, c, SHADE_LIT);
            }
            let k = h(x as f64, 99.0 + side as f64);
            block(&mut out, at(side * (x + 0.25), 1.0, -2.0), Vec3::new(0.52, 0.1, 0.48), Quat::from_rotation_y((k - 0.5) * 0.05), lin(0x7a766c) * (0.8 + 0.25 * k), SHADE_LIT);
            x += 0.5;
            n += 1;
        }
        let _ = n;
        // Iron railings along the top: spear-headed bars and two rails.
        let mut bx = 1.72;
        while bx < 11.9 {
            let bent = h(bx as f64 * 5.0, side as f64) > 0.93;
            let tilt = if bent { Quat::from_rotation_z(side * 0.25) } else { flat };
            block(&mut out, at(side * bx, 1.4, -2.0), Vec3::new(0.035, 0.7, 0.035), tilt, iron, SHADE_METAL);
            block(&mut out, at(side * bx, 1.78, -2.0) + tilt * Vec3::new(0.0, 0.02, 0.0), Vec3::new(0.07, 0.07, 0.035), tilt * Quat::from_rotation_z(0.785), iron, SHADE_METAL);
            bx += 0.16;
        }
        for y in [1.15f32, 1.62] {
            block(&mut out, at(side * 6.8, y, -2.0), Vec3::new(10.2, 0.03, 0.04), flat, iron, SHADE_METAL);
        }
    }
    // ---- the gate pillars: a plinth, a shaft of dressed stone, a cap and a stone ball
    for side in [-1.0f32, 1.0] {
        let x = side * 1.34;
        block(&mut out, at(x, 0.15, -2.0), Vec3::new(0.7, 0.3, 0.7), flat, lin(0x625e56), SHADE_LIT);
        for k in 0..6 {
            let v = h(k as f64, side as f64 * 3.0);
            block(&mut out, at(x, 0.47 + k as f32 * 0.34, -2.0), Vec3::new(0.54, 0.33, 0.54), Quat::from_rotation_y((v - 0.5) * 0.04), lin(0x76726a) * (0.82 + 0.25 * v), SHADE_LIT);
        }
        block(&mut out, at(x, 2.4, -2.0), Vec3::new(0.72, 0.12, 0.72), flat, lin(0x827e74), SHADE_LIT);
        block(&mut out, at(x, 2.5, -2.0), Vec3::new(0.5, 0.08, 0.5), flat, lin(0x7a766c), SHADE_LIT);
        out.push(Instance::new(Mat4::from_scale_rotation_translation(Vec3::splat(0.19), flat, at(x, 2.73, -2.0)), lin(0x7e7a70), 1.0, Vec3::ZERO, SHADE_LIT));
        // Ivy creeping up the left pillar.
        if side < 0.0 {
            for k in 0..40 {
                let (a, b) = (h(k as f64, 31.0), h(k as f64, 37.0));
                let y = 0.2 + a * 1.8;
                let face = if b > 0.5 { 0.28 } else { -0.28 };
                block(&mut out, at(x + (b - 0.5) * 0.5, y, -2.0 + face), Vec3::new(0.08, 0.07, 0.03), Quat::from_rotation_z(a * 4.0), lin(0x2e4a26) * (0.8 + 0.4 * b), SHADE_LIT);
            }
        }
    }
    // ---- the iron arch over the gate, scrolled, with a cross at its crown
    for i in 0..=40 {
        let t = i as f32 / 40.0;
        let x = -1.08 + 2.16 * t;
        let y = 2.28 + (t * PI).sin() * 0.5;
        block(&mut out, at(x, y, -2.0), Vec3::new(0.07, 0.04, 0.04), Quat::from_rotation_z(((t - 0.5) * PI).cos() * 0.0 + (0.5 - t) * 1.2), iron, SHADE_METAL);
        if i % 5 == 0 && i > 0 && i < 40 {
            let top = 2.28 + (t * PI).sin() * 0.5;
            block(&mut out, at(x, (2.24 + top) / 2.0, -2.0), Vec3::new(0.025, top - 2.24, 0.025), flat, iron, SHADE_METAL);
        }
    }
    block(&mut out, at(-0.0, 2.2, -2.0), Vec3::new(2.2, 0.035, 0.04), flat, iron, SHADE_METAL);
    for k in 0..8 {
        let a = k as f32 / 8.0 * TAU;
        for side in [-1.0f32, 1.0] {
            block(&mut out, at(side * 0.45 + a.cos() * 0.09, 2.45 + a.sin() * 0.09, -2.0), Vec3::new(0.03, 0.03, 0.03), flat, iron, SHADE_METAL);
        }
    }
    block(&mut out, at(0.0, 3.0, -2.0), Vec3::new(0.04, 0.38, 0.04), flat, iron, SHADE_METAL);
    block(&mut out, at(0.0, 3.07, -2.0), Vec3::new(0.2, 0.04, 0.04), flat, iron, SHADE_METAL);
    // The lantern's bracket, sticking out from the right pillar over the path.
    block(&mut out, at(1.34, 1.95, -1.55), Vec3::new(0.04, 0.04, 0.5), flat, iron, SHADE_METAL);
    block(&mut out, at(1.34, 1.83, -1.33), Vec3::new(0.02, 0.2, 0.02), Quat::from_rotation_x(0.6), iron, SHADE_METAL);

    // ---- the graves: rows of headstones, crosses and obelisks, a clear walk up the middle
    for row in 0..8 {
        let z = -3.2 - row as f32 * 1.3;
        let mut x = -9.6 + (row % 2) as f32 * 0.7;
        while x < 9.6 {
            let k = h(x as f64 * 2.3 + row as f64, row as f64 * 7.1);
            let (px, pz) = (x + (k - 0.5) * 0.4, z + (h(x as f64, row as f64 + 50.0) - 0.5) * 0.35);
            x += 1.35 + k * 0.4;
            if px.abs() < 1.0 && row < 7 {
                continue;
            }
            grave(&mut out, at(px, 0.0, pz), k, h(px as f64 * 9.0, pz as f64 * 3.0));
        }
    }
    // ---- a mausoleum among them, its door a black mouth
    let m = |x: f32, y: f32, z: f32| at(-4.4 + x, y, -8.2 + z);
    block(&mut out, m(0.0, 0.1, 0.0), Vec3::new(2.4, 0.2, 2.6), flat, lin(0x5e5a52), SHADE_LIT);
    block(&mut out, m(0.0, 0.95, -0.1), Vec3::new(2.0, 1.5, 2.2), flat, lin(0x6a665e), SHADE_LIT);
    for side in [-1.0f32, 1.0] {
        block(&mut out, m(side * 0.55, 2.02, -0.1), Vec3::new(1.25, 0.1, 2.4), Quat::from_rotation_z(side * -0.5), lin(0x4e4a44), SHADE_LIT);
        block(&mut out, m(side * 0.72, 0.86, 1.1), Vec3::new(0.16, 1.32, 0.16), flat, lin(0x7a766e), SHADE_LIT);
    }
    for k in 0..5 {
        let w = 2.0 - k as f32 * 0.4;
        block(&mut out, m(0.0, 1.78 + k as f32 * 0.1, 1.02), Vec3::new(w, 0.1, 0.12), flat, lin(0x726e66), SHADE_LIT);
    }
    block(&mut out, m(0.0, 0.72, 1.0), Vec3::new(0.62, 1.05, 0.06), flat, lin(0x060506), SHADE_UNLIT);
    block(&mut out, m(0.0, 1.35, 1.03), Vec3::new(0.8, 0.12, 0.06), flat, lin(0x6e6a62), SHADE_LIT);
    // ---- bare trees in the churchyard, and a big crooked one outside the wall
    for (i, &(x, z, height)) in [(-5.4f32, -3.9f32, 3.4f32), (5.1, -5.7, 3.9), (-8.3, -10.6, 4.4), (7.8, -11.8, 4.1), (-3.9, 1.2, 4.6), (9.2, 0.4, 3.6)].iter().enumerate() {
        dead_tree(&mut out, at(x, 0.0, z), height, i as f64 * 7.3 + 1.0);
    }
    // ---- the chapel beyond, a bell tower, one window lit
    let c = |x: f32, y: f32, z: f32| at(2.0 + x, y, -17.5 + z);
    block(&mut out, c(0.0, 1.7, 0.0), Vec3::new(5.4, 3.4, 3.8), flat, lin(0x4a4640), SHADE_LIT);
    for side in [-1.0f32, 1.0] {
        block(&mut out, c(0.0, 3.95, side * 1.05), Vec3::new(5.8, 0.18, 2.5), Quat::from_rotation_x(side * 0.62), lin(0x2e2a28), SHADE_LIT);
    }
    block(&mut out, c(3.4, 3.2, 0.3), Vec3::new(1.5, 6.4, 1.5), flat, lin(0x504a44), SHADE_LIT);
    for k in 0..6 {
        let w = 1.6 - k as f32 * 0.27;
        block(&mut out, c(3.4, 6.55 + k as f32 * 0.3, 0.3), Vec3::new(w, 0.3, w), Quat::from_rotation_y(0.785), lin(0x2a2624), SHADE_LIT);
    }
    block(&mut out, c(3.4, 5.4, 1.06), Vec3::new(0.5, 0.8, 0.05), flat, lin(0x0c0a0a), SHADE_UNLIT);
    for (x, lit) in [(-1.6f32, true), (0.0, false), (1.6, true)] {
        let glow = if lit { lin(0xffa050) * 1.4 } else { Vec3::ZERO };
        glowing(&mut out, c(x, 1.9, 1.92), Vec3::new(0.45, 0.9, 0.05), flat, if lit { lin(0xffc070) } else { lin(0x14121a) }, glow);
    }
    // ---- outside: tussocks of long grass, stones, a scatter of dead leaves
    for i in 0..90 {
        let (a, b) = (h(i as f64, 3.3), h(i as f64, 5.9));
        let (x, z) = ((a - 0.5) * 20.0, -1.4 + b * 9.0);
        if x.abs() < 1.1 {
            continue;
        }
        for k in 0..4 {
            let lean = Quat::from_rotation_z((h(i as f64, k as f64) - 0.5) * 0.7) * Quat::from_rotation_x((h(k as f64, i as f64) - 0.5) * 0.6);
            let tall = 0.14 + h(i as f64 * 3.0, k as f64) * 0.2;
            block(&mut out, at(x + k as f32 * 0.03, tall / 2.0, z), Vec3::new(0.02, tall, 0.02), lean, lin(0x6a7040) * (0.6 + 0.5 * b), SHADE_LIT);
        }
    }
    for i in 0..60 {
        let (a, b) = (h(i as f64, 71.0), h(i as f64, 73.0));
        let (x, z) = ((a - 0.5) * 8.0, -1.6 + b * 6.0);
        let c = [0x8a4a1e, 0x9a6a24, 0x6a3a1a][i % 3];
        block(&mut out, at(x, 0.008, z), Vec3::new(0.07, 0.01, 0.05), Quat::from_rotation_y(a * 9.0), lin(c), SHADE_LIT);
    }
    out
}

/// One grave at `p`, its kind and weathering picked by `k` and `j`.
fn grave(out: &mut Vec<Instance>, p: Vec3, k: f32, j: f32) {
    let stone = lin(0x7a766c) * (0.62 + 0.35 * j);
    let moss = lin(0x4e5a38) * (0.8 + 0.3 * j);
    let lean = Quat::from_rotation_z((k - 0.5) * 0.25) * Quat::from_rotation_x((j - 0.5) * 0.22);
    let put = |out: &mut Vec<Instance>, off: Vec3, s: Vec3, c: Vec3| block(out, p + lean * off, s, lean, c, SHADE_LIT);
    // A low mound of earth in front of most.
    if j > 0.25 {
        block(out, p + Vec3::new(0.0, 0.02, 0.5), Vec3::new(0.42, 0.07, 0.75), Quat::from_rotation_y((k - 0.5) * 0.2), lin(0x3a3226) * (0.8 + 0.3 * k), SHADE_GRASS);
    }
    match (k * 5.0) as i32 {
        0 | 1 => {
            // A rounded headstone.
            put(out, Vec3::new(0.0, 0.3, 0.0), Vec3::new(0.46, 0.6, 0.1), stone);
            put(out, Vec3::new(0.0, 0.63, 0.0), Vec3::new(0.36, 0.07, 0.1), stone);
            put(out, Vec3::new(0.0, 0.69, 0.0), Vec3::new(0.22, 0.05, 0.1), stone);
            if j > 0.6 {
                put(out, Vec3::new(-0.1, 0.1, 0.03), Vec3::new(0.28, 0.2, 0.06), moss);
            }
            // An inscription, worn: a few dark lines.
            for r in 0..3 {
                put(out, Vec3::new(0.0, 0.46 - r as f32 * 0.09, 0.052), Vec3::new(0.26 - r as f32 * 0.04, 0.02, 0.005), lin(0x3a3830));
            }
        }
        2 => {
            // A cross on a plinth.
            put(out, Vec3::new(0.0, 0.07, 0.0), Vec3::new(0.32, 0.14, 0.24), stone * 0.9);
            put(out, Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.09, 0.75, 0.09), stone);
            put(out, Vec3::new(0.0, 0.66, 0.0), Vec3::new(0.42, 0.09, 0.09), stone);
            if j > 0.5 {
                for a in 0..6 {
                    let t = a as f32 / 6.0 * TAU;
                    put(out, Vec3::new(t.cos() * 0.13, 0.66 + t.sin() * 0.13, 0.0), Vec3::new(0.05, 0.05, 0.06), stone);
                }
            }
        }
        3 => {
            // An obelisk.
            put(out, Vec3::new(0.0, 0.1, 0.0), Vec3::new(0.36, 0.2, 0.36), stone * 0.9);
            for (i, (w, hgt)) in [(0.24f32, 0.4f32), (0.2, 0.4), (0.16, 0.34)].into_iter().enumerate() {
                put(out, Vec3::new(0.0, 0.2 + i as f32 * 0.38 + hgt / 2.0, 0.0), Vec3::new(w, hgt, w), stone);
            }
            put(out, Vec3::new(0.0, 1.38, 0.0), Vec3::new(0.11, 0.11, 0.11), stone);
        }
        _ => {
            // An old slab, sunk and tilted, half broken.
            let tilt = Quat::from_rotation_x(-0.35 - j * 0.3) * lean;
            block(out, p + Vec3::new(0.0, 0.18, 0.0), Vec3::new(0.44, 0.42, 0.1), tilt, stone * 0.85, SHADE_LIT);
            block(out, p + Vec3::new(0.18, 0.03, 0.3), Vec3::new(0.16, 0.06, 0.12), Quat::from_rotation_y(k * 4.0), stone * 0.8, SHADE_LIT);
        }
    }
}

/// A leafless tree: a crooked trunk splitting into branches and twigs, black against the sky.
fn dead_tree(out: &mut Vec<Instance>, base: Vec3, height: f32, seed: f64) {
    let bark = lin(0x241f1a);
    fn branch(out: &mut Vec<Instance>, from: Vec3, dir: Vec3, len: f32, thick: f32, depth: u32, seed: f64, bark: Vec3) {
        let segs = 3;
        let mut p = from;
        let mut d = dir.normalize();
        for i in 0..segs {
            let k = hash(seed + i as f64 * 1.7, depth as f64) as f32;
            d = (d + Vec3::new((k - 0.5) * 0.5, 0.05, (hash(seed, i as f64 + 9.0) as f32 - 0.5) * 0.5)).normalize();
            let l = len / segs as f32;
            let q = p + d * l;
            let w = thick * (1.0 - i as f32 / segs as f32 * 0.4);
            let rot = Quat::from_rotation_arc(Vec3::Y, d);
            block(out, (p + q) * 0.5, Vec3::new(w, l + w * 0.5, w), rot, bark * (0.9 + 0.2 * k), SHADE_LIT);
            p = q;
        }
        if depth == 0 || thick < 0.025 {
            return;
        }
        let n = if depth >= 2 { 3 } else { 2 };
        for b in 0..n {
            let k = hash(seed * 1.3 + b as f64, depth as f64 * 3.1) as f32;
            let a = k * TAU + b as f32 * 2.1;
            let nd = (d * 0.9 + Vec3::new(a.cos(), 0.35 + k * 0.4, a.sin()) * 0.9).normalize();
            branch(out, p, nd, len * (0.55 + k * 0.2), thick * 0.55, depth - 1, seed * 1.7 + b as f64 * 11.0, bark);
        }
    }
    let lean = Vec3::new((hash(seed, 1.0) as f32 - 0.5) * 0.35, 1.0, (hash(seed, 2.0) as f32 - 0.5) * 0.35);
    branch(out, base, lean, height * 0.5, 0.2, 3, seed, bark);
    // Roots gripping the ground.
    for r in 0..4 {
        let a = r as f32 / 4.0 * TAU + seed as f32;
        let d = Vec3::new(a.cos(), -0.25, a.sin());
        block(out, base + d * 0.18 + Vec3::new(0.0, 0.05, 0.0), Vec3::new(0.09, 0.4, 0.09), Quat::from_rotation_arc(Vec3::Y, d), bark, SHADE_LIT);
    }
}

/// The dungeon gateway's stone, props and the stair behind the door: everything that stays put.
pub(super) fn gateway_set() -> Vec<Instance> {
    let mut out = Vec::new();
    let at = |x: f32, y: f32, z: f32| GATEWAY + Vec3::new(x, y, z);
    let flat = Quat::IDENTITY;
    let stone = |k: f32| lin(0x4a4640) * (0.7 + 0.45 * k);

    // ---- the floor: big flagstones, some slick with wet, puddles in the hollows
    for gx in -7..=7 {
        for gz in -6..=6 {
            let k = h(gx as f64 * 1.9 + 4.0, gz as f64 * 2.7 + 1.0);
            let (x, z) = (gx as f32 * 0.62, gz as f32 * 0.62 + 0.3);
            let wet = h(gx as f64 * 0.7, gz as f64 * 0.9 + 3.0) > 0.7;
            let c = if wet { lin(0x2a2c30) * (0.8 + 0.3 * k) } else { lin(0x46423c) * (0.72 + 0.4 * k) };
            block(&mut out, at(x, -0.1 + (k - 0.5) * 0.025, z), Vec3::new(0.59, 0.2, 0.59), Quat::from_rotation_y((k - 0.5) * 0.03), c, if wet { SHADE_METAL } else { SHADE_LIT });
        }
    }
    block(&mut out, at(0.0, -0.25, 0.3), Vec3::new(9.4, 0.2, 8.4), flat, lin(0x141210), SHADE_LIT);
    // ---- walls: the back with the door's arch, the sides, the front with the arch they came through
    let course = |out: &mut Vec<Instance>, from: Vec3, along: Vec3, len: f32, top: f32, gap: Option<(f32, f32, f32)>, seed: f64| {
        // Stone courses from `from` along `along` for `len`, up to `top`, leaving an arched
        // opening (centre, half width, height of its springing): stones are cut back to its edge.
        let rows = (top / 0.34).ceil() as i32;
        for row in 0..rows {
            let mut u = if row % 2 == 0 { 0.0 } else { -0.3 };
            let y = row as f32 * 0.34 + 0.17;
            while u < len {
                let k = h(u as f64 * 3.7 + seed, row as f64 * 1.3 + seed);
                let w = 0.5 + k * 0.3;
                let (mut a, mut b) = (u.max(0.0), (u + w).min(len));
                u += w;
                if let Some((gc, half, gh)) = gap {
                    // How wide the opening is at this course's height (a half-ellipse above the springing).
                    let rise = half * 0.55 + 0.16;
                    let open = if y - 0.17 < gh { half + 0.14 } else if y - 0.17 < gh + rise { (half + 0.14) * (1.0 - ((y - 0.17 - gh) / rise).powi(2)).max(0.0).sqrt() } else { 0.0 };
                    if open > 0.0 && b > gc - open && a < gc + open {
                        if (a + b) / 2.0 < gc {
                            b = gc - open;
                        } else {
                            a = gc + open;
                        }
                    }
                }
                if b - a < 0.06 {
                    continue;
                }
                let mid = (a + b) / 2.0;
                let wet = k > 0.82;
                let c = if wet { lin(0x2e3a30) * (0.8 + 0.3 * k) } else { stone(k) };
                block(out, from + along * mid + Vec3::new(0.0, y, 0.0), (along.abs() * (b - a - 0.02)) + Vec3::new(0.0, 0.32, 0.0) + (Vec3::ONE - along.abs() - Vec3::Y) * (0.34 + k * 0.05), flat, c, SHADE_LIT);
            }
        }
    };
    course(&mut out, at(-4.6, 0.0, -3.6), Vec3::X, 9.2, 4.8, Some((4.6, 0.98, 2.4)), 1.0);
    course(&mut out, at(-4.6, 0.0, 4.2), Vec3::X, 9.2, 4.8, Some((4.6, 0.88, 2.2)), 2.0);
    course(&mut out, at(-4.6, 0.0, -3.6), Vec3::Z, 7.8, 4.8, None, 3.0);
    course(&mut out, at(4.6, 0.0, -3.6), Vec3::Z, 7.8, 4.8, None, 4.0);
    // The arch they came through, sealed now with dark stone set deep.
    block(&mut out, at(0.0, 1.4, 4.55), Vec3::new(2.0, 2.9, 0.3), flat, lin(0x26241f), SHADE_LIT);
    for (gc_z, half, gh, dz) in [(-3.6f32, 0.98f32, 2.4f32, 0.0f32), (4.2, 0.88, 2.2, 0.0)] {
        // Voussoirs round each arch.
        for i in 0..=12 {
            let a = i as f32 / 12.0 * PI;
            let p = at(-a.cos() * (half + 0.12), gh + a.sin() * (half + 0.12) * 0.55, gc_z + dz);
            block(&mut out, p, Vec3::new(0.26, 0.2, 0.42), Quat::from_rotation_z(a - PI / 2.0), stone(0.9), SHADE_LIT);
        }
        for side in [-1.0f32, 1.0] {
            for k in 0..7 {
                block(&mut out, at(side * (half + 0.12), 0.17 + k as f32 * 0.34, gc_z), Vec3::new(0.26, 0.32, 0.42), flat, stone(0.8 + (k % 2) as f32 * 0.15), SHADE_LIT);
            }
        }
    }
    // The skull carved in the keystone over the iron door, its eye sockets holding red embers.
    let sk = at(0.0, 3.2, -3.36);
    block(&mut out, sk, Vec3::new(0.34, 0.3, 0.12), flat, lin(0xa89e88), SHADE_LIT);
    block(&mut out, sk + Vec3::new(0.0, -0.18, 0.0), Vec3::new(0.22, 0.1, 0.1), flat, lin(0xa0967e), SHADE_LIT);
    for side in [-1.0f32, 1.0] {
        glowing(&mut out, sk + Vec3::new(side * 0.075, 0.02, 0.065), Vec3::new(0.08, 0.07, 0.02), flat, lin(0x300808), lin(0xff2a10) * 1.6);
    }
    block(&mut out, sk + Vec3::new(0.0, -0.06, 0.065), Vec3::new(0.04, 0.05, 0.02), flat, lin(0x1a1410), SHADE_LIT);
    // The iron tympanum filling the arch over the door leaves.
    for i in 0..9 {
        let x = -0.84 + i as f32 * 0.21;
        let top = 2.4 + ((1.0 - (x / 0.98).powi(2)).max(0.0)).sqrt() * 0.98 * 0.55;
        block(&mut out, at(x, (2.42 + top) / 2.0, -3.5), Vec3::new(0.04, top - 2.42, 0.04), flat, lin(0x26282c), SHADE_METAL);
    }
    block(&mut out, at(0.0, 2.44, -3.5), Vec3::new(1.96, 0.06, 0.08), flat, lin(0x26282c), SHADE_METAL);
    // ---- pillars down each side, and ribs of a vault springing between them
    for &(x, z) in &[(-3.35f32, -1.3f32), (3.35, -1.3), (-3.35, 2.0), (3.35, 2.0)] {
        block(&mut out, at(x, 0.12, z), Vec3::new(0.8, 0.24, 0.8), flat, stone(0.6), SHADE_LIT);
        for k in 0..12 {
            block(&mut out, at(x, 0.4 + k as f32 * 0.34, z), Vec3::new(0.6, 0.33, 0.6), Quat::from_rotation_y((h(k as f64, x as f64) - 0.5) * 0.05), stone(0.55 + 0.2 * h(k as f64, z as f64)), SHADE_LIT);
        }
        block(&mut out, at(x, 4.2, z), Vec3::new(0.78, 0.2, 0.78), flat, stone(0.7), SHADE_LIT);
        // A torch bracket on the inner face.
        let inward = -x.signum();
        block(&mut out, at(x + inward * 0.36, 1.72, z), Vec3::new(0.12, 0.06, 0.06), flat, lin(0x222226), SHADE_METAL);
        block(&mut out, at(x + inward * 0.42, 1.82, z), Vec3::new(0.05, 0.32, 0.05), Quat::from_rotation_z(inward * -0.25), lin(0x4a3220), SHADE_WOOD);
    }
    for z in [-1.3f32, 2.0] {
        for i in 0..=18 {
            let a = i as f32 / 18.0 * PI;
            let p = at(-a.cos() * 3.35, 4.3 + a.sin() * 0.9, z);
            block(&mut out, p, Vec3::new(0.46, 0.3, 0.5), Quat::from_rotation_z(a - PI / 2.0), stone(0.5 + 0.2 * h(i as f64, z as f64)), SHADE_LIT);
        }
    }
    // ---- the old man's corner: a three-legged stool, a clay mug on the floor, a blanket
    let st = at(2.05, 0.0, -2.75);
    block(&mut out, st + Vec3::new(0.0, 0.42, 0.0), Vec3::new(0.4, 0.06, 0.4), Quat::from_rotation_y(0.3), lin(0x5a3e24), SHADE_WOOD);
    for k in 0..3 {
        let a = k as f32 / 3.0 * TAU + 0.3;
        block(&mut out, st + Vec3::new(a.cos() * 0.13, 0.2, a.sin() * 0.13), Vec3::new(0.05, 0.42, 0.05), Quat::from_rotation_z(a.cos() * 0.12) * Quat::from_rotation_x(-a.sin() * 0.12), lin(0x4a3220), SHADE_WOOD);
    }
    block(&mut out, st + Vec3::new(0.34, 0.06, 0.32), Vec3::new(0.09, 0.12, 0.09), flat, lin(0x8a5a3a), SHADE_LIT);
    block(&mut out, st + Vec3::new(0.0, 0.47, 0.0), Vec3::new(0.34, 0.04, 0.3), Quat::from_rotation_y(0.2), lin(0x6a3a2a), SHADE_CLOTH);
    // ---- Quitmer's corner: straw, a battered bowl with a bone, the ring in the wall for his chain
    for i in 0..70 {
        let (a, b) = (h(i as f64, 5.0), h(i as f64, 8.0));
        let p = at(-1.95 + (a - 0.5) * 1.3, 0.02 + b * 0.04, -2.55 + (b - 0.5) * 1.0);
        block(&mut out, p, Vec3::new(0.24, 0.015, 0.025), Quat::from_rotation_y(a * 7.0), lin(0xb09a50) * (0.6 + 0.5 * b), SHADE_LIT);
    }
    let bowl = at(-0.95, 0.0, -2.85);
    block(&mut out, bowl + Vec3::new(0.0, 0.05, 0.0), Vec3::new(0.32, 0.1, 0.32), flat, lin(0x3a3c40), SHADE_METAL);
    block(&mut out, bowl + Vec3::new(0.0, 0.095, 0.0), Vec3::new(0.24, 0.012, 0.24), flat, lin(0x1a1614), SHADE_LIT);
    block(&mut out, bowl + Vec3::new(0.05, 0.12, 0.02), Vec3::new(0.22, 0.04, 0.04), Quat::from_rotation_y(0.6), lin(0xd8ceb4), SHADE_LIT);
    for side in [-1.0f32, 1.0] {
        block(&mut out, bowl + Vec3::new(0.05, 0.12, 0.02) + Quat::from_rotation_y(0.6) * Vec3::new(side * 0.12, 0.0, 0.0), Vec3::splat(0.06), flat, lin(0xe0d6bc), SHADE_LIT);
    }
    for k in 0..12 {
        let a = k as f32 / 12.0 * TAU;
        block(&mut out, at(-1.95 + a.cos() * 0.09, 0.62 + a.sin() * 0.09, -3.37), Vec3::splat(0.035), flat, lin(0x38383c), SHADE_METAL);
    }
    block(&mut out, at(-1.95, 0.72, -3.39), Vec3::new(0.08, 0.08, 0.04), flat, lin(0x2a2a2e), SHADE_METAL);
    // Bones and a skull by the wall; chains hanging on the side walls; skulls in niches.
    for i in 0..9 {
        let (a, b) = (h(i as f64, 41.0), h(i as f64, 43.0));
        block(&mut out, at(-3.6 + a * 1.2, 0.02, -3.1 + b * 0.8), Vec3::new(0.2, 0.035, 0.035), Quat::from_rotation_y(a * 5.0), lin(0xcfc4a8) * (0.8 + 0.2 * b), SHADE_LIT);
    }
    block(&mut out, at(-3.1, 0.07, -2.9), Vec3::new(0.14, 0.13, 0.15), Quat::from_rotation_y(0.7), lin(0xd4caae), SHADE_LIT);
    for &(x, z) in &[(-4.38f32, 0.4f32), (4.38, -0.2), (4.38, 3.2)] {
        for k in 0..10 {
            let y = 3.2 - k as f32 * 0.12;
            let r = if k % 2 == 0 { Quat::IDENTITY } else { Quat::from_rotation_y(PI / 2.0) };
            block(&mut out, at(x, y, z), Vec3::new(0.02, 0.1, 0.06), r, lin(0x303236), SHADE_METAL);
        }
    }
    for &(x, z) in &[(-4.6f32, 1.25f32), (-4.6, -2.2), (4.6, 0.9), (4.6, -2.4)] {
        // A niche in the wall (its dark back just proud of the face), a skull set in it.
        let inward = -x.signum();
        block(&mut out, at(x + inward * 0.2, 1.45, z), Vec3::new(0.02, 0.5, 0.45), flat, lin(0x0e0c0a), SHADE_UNLIT);
        block(&mut out, at(x + inward * 0.28, 1.33, z), Vec3::new(0.13, 0.12, 0.14), flat, lin(0xcfc4a6), SHADE_LIT);
    }
    // ---- behind the door: a stair going down into red light
    for k in 0..10 {
        let z = -3.95 - k as f32 * 0.42;
        let y = -0.12 - k as f32 * 0.26;
        block(&mut out, at(0.0, y, z), Vec3::new(1.9, 0.26, 0.44), flat, lin(0x2e221e) * (0.8 + 0.2 * h(k as f64, 3.0)), SHADE_LIT);
        for side in [-1.0f32, 1.0] {
            block(&mut out, at(side * 1.15, y + 1.3, z), Vec3::new(0.4, 2.8, 0.44), flat, lin(0x241c18) * (0.7 + 0.3 * h(k as f64, side as f64)), SHADE_LIT);
        }
        block(&mut out, at(0.0, y + 3.1, z), Vec3::new(2.3, 0.3, 0.44), flat, lin(0x0e0a09), SHADE_LIT);
    }
    glowing(&mut out, at(0.0, -2.9, -8.4), Vec3::new(1.9, 0.1, 1.4), flat, lin(0x2a0400), lin(0xff3a0a) * 2.2);
    out
}

/// Low mist over a set: sheets whose drifting noise the shader draws.
fn mist_sheets(origin: Vec3, x: (i32, i32), z: (i32, i32), layers: &[(f32, f32)], thick: impl Fn(i32, i32) -> f32) -> Vec<Instance> {
    let mut out = Vec::new();
    for gx in x.0..=x.1 {
        for gz in z.0..=z.1 {
            let k = thick(gx, gz);
            if k <= 0.0 {
                continue;
            }
            for (i, &(lift, alpha)) in layers.iter().enumerate() {
                let m = Mat4::from_translation(origin + Vec3::new(gx as f32, lift, gz as f32)) * Mat4::from_scale(Vec3::new(1.0, 0.002, 1.0));
                out.push(Instance::new(m, lin(0xb4b8c8), alpha * k, Vec3::new(20.0 + i as f32, lift, 0.0), SHADE_CLOUD));
            }
        }
    }
    out
}

pub(super) fn graveyard_mist() -> Vec<Instance> {
    // Thick among the graves, thinning out past the wall.
    mist_sheets(GRAVEYARD, (-13, 13), (-17, 6), &[(0.08, 0.6), (0.25, 0.52), (0.48, 0.4), (0.8, 0.28), (1.25, 0.16)], |_x, z| if z <= -2 { 1.0 } else { (1.0 - (z as f32 + 2.0) / 7.0).max(0.25) })
}

pub(super) fn gateway_mist() -> Vec<Instance> {
    mist_sheets(GATEWAY, (-4, 4), (-3, 4), &[(0.06, 0.3), (0.2, 0.2)], |_x, _z| 1.0)
}

// ------------------------------------------------------------ atmospheres

/// Dusk over the graveyard: a low sun off to the left behind the graves, violet haze.
fn dusk(darken: f32, fog: f32) -> Atmosphere {
    Atmosphere {
        background: 0x4c3c58,
        fog_near: mix(11.0, 1.5, fog),
        fog_far: mix(34.0, 5.0, fog),
        hemi_sky: lin(0x8a7ab8) * 0.55 * (1.0 - 0.35 * darken),
        hemi_ground: lin(0x3a3024) * 0.45,
        ambient: lin(0x5a4a64) * 0.22,
        sun_dir: Vec3::new(-0.72, 0.3, -0.62).normalize(),
        sun: lin(0xffa062) * 2.1 * (1.0 - 0.55 * darken),
        exposure: 1.2,
        night_sky: 2.0,
        bloom: 0.95,
        moon_view: Vec3::new(-0.62, 0.1, -0.78).normalize(),
        cloud_tint: lin(0x4a3444),
    }
}

/// The gateway: no sky, a cold faint light from the grating far above, torchlight doing the rest.
fn crypt() -> Atmosphere {
    Atmosphere {
        background: 0x07080b,
        fog_near: 4.5,
        fog_far: 15.0,
        hemi_sky: lin(0x44526a) * 0.38,
        hemi_ground: lin(0x2a2018) * 0.3,
        ambient: lin(0x1c2028) * 0.75,
        sun_dir: Vec3::new(0.25, 0.9, 0.35).normalize(),
        sun: lin(0x8aa0c8) * 0.4,
        exposure: 1.2,
        night_sky: 0.0,
        bloom: 1.3,
        moon_view: Vec3::Y,
        cloud_tint: lin(0x161b24),
    }
}

// ------------------------------------------------------------ choreography

/// Where each friend stands kicking the ball about (graveyard coordinates).
const KID_HOME: [Vec3; 4] = [Vec3::new(0.5, 0.0, 0.7), Vec3::new(-1.05, 0.0, 1.05), Vec3::new(-0.45, 0.0, 2.15), Vec3::new(0.55, 0.0, 2.2)];
/// Where the boy who fetches the ball stands at the gate, peering through.
const GATE_SPOT: Vec3 = Vec3::new(0.12, 0.0, -1.42);
/// The passes, kicker to receiver, one at each of `stage::KICKS`.
const PASSES: [(usize, usize); 5] = [(0, 1), (1, 3), (3, 2), (2, 0), (0, 2)];
/// Where the ball comes to rest among the graves.
const BALL_REST: Vec3 = Vec3::new(0.38, 0.07, -4.3);
const BALL_R: f32 = 0.07;
/// Krampus: waiting in the fog, at the gate, reaching through it, and gone into the graves.
const K_FOG: Vec3 = Vec3::new(0.35, 0.0, -6.4);
const K_GATE: Vec3 = Vec3::new(0.12, 0.0, -2.55);
const K_GRAB: Vec3 = Vec3::new(0.12, 0.0, -2.12);
const K_GONE: Vec3 = Vec3::new(0.8, 0.0, -8.2);
/// The gateway: where the three land, the old man stands, the dog lies and his chain is fixed.
const LAND: [Vec3; 3] = [Vec3::new(-0.58, 0.0, 1.62), Vec3::new(0.08, 0.0, 1.88), Vec3::new(0.66, 0.0, 1.58)];
const OLD_MAN_AT: Vec3 = Vec3::new(1.45, 0.0, -2.2);
const DOG_BED: Vec3 = Vec3::new(-1.95, 0.0, -2.35);
const DOG_LUNGE: Vec3 = Vec3::new(-1.72, 0.0, -1.55);
const CHAIN_RING: Vec3 = Vec3::new(-1.95, 0.62, -3.37);
const CHAIN_LEN: f32 = 2.35;
/// The souls rise before the iron door: Mehmed, Ragnar and Miyamoto, as on the character screen.
const SOUL_AT: [Vec3; 3] = [Vec3::new(-0.82, 0.0, -2.62), Vec3::new(0.0, 0.0, -2.82), Vec3::new(0.82, 0.0, -2.62)];
/// Each soul's tint: steel blue for the viking, jade for the janissary, rose for the samurai.
pub(super) const SOUL_TINT: [u32; 3] = [0x7ae8c8, 0x7ab4ff, 0xff98b0];
/// The middle of the three boys in the gateway, for looking at them.
const BOYS_MID: Vec3 = Vec3::new(0.05, 0.0, 1.7);

/// What a boy is doing, for his limbs.
#[derive(Clone, Copy, Debug)]
pub(super) enum Act {
    Stand,
    /// Running, or walking with `pace` below 1: the stride's phase.
    Run { phase: f32, pace: f32 },
    /// A kick: seconds since the foot met the ball (negative while winding up).
    Kick(f32),
    Peer,
    Scared(f32),
    Talk(f32),
    /// Picking himself up off the floor: 0 flat on his front, 1 standing.
    Up(f32),
    Flail(f32),
}

fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

fn ease_in(t: f32) -> f32 {
    t.clamp(0.0, 1.0).powi(2)
}

/// A boy's feet on the ground, facing `yaw`.
pub(super) fn kid_root(feet: Vec3, yaw: f32) -> Mat4 {
    Mat4::from_translation(feet) * Mat4::from_rotation_y(yaw)
}

impl WorldView {
    /// The soul-rise smoothing and anything else the story animates by the step.
    pub(super) fn update_story(&mut self, dt: f32, sv: StoryView) {
        let before = first(Shot::Souls).map(|i| sv.beat < i).unwrap_or(true);
        for (i, r) in self.story_rise.iter_mut().enumerate() {
            // The old man names Ragnar first, then Mehmed, then Miyamoto; they stand Mehmed, Ragnar, Miyamoto.
            let bit = [1u8, 0, 2][i];
            let goal = if sv.souls & (1 << bit) != 0 { 1.0 } else { 0.0 };
            let step = dt * 0.9;
            *r = if before {
                0.0
            } else if goal > *r {
                (*r + step).min(goal)
            } else {
                (*r - step).max(goal)
            };
        }
    }

    /// Scene I, each frame: whichever set the beat is on, its cast and its light.
    pub(super) fn emit_story(&self, sv: StoryView, s: &SceneState, f: &mut Frame) {
        let cube = MeshKind::Cube as usize;
        match sv.shot().place() {
            Place::Graveyard => {
                f.opaque[cube].extend_from_slice(&self.story_sets[0]);
                f.transparent[cube].extend_from_slice(&self.story_mist[0]);
                f.bounds = Vec4::new(GRAVEYARD.x - 13.5, GRAVEYARD.z - 17.5, GRAVEYARD.x + 13.5, GRAVEYARD.z + 6.5);
                self.emit_graveyard(sv, s, f);
            }
            Place::Gateway => {
                f.opaque[cube].extend_from_slice(&self.story_sets[1]);
                f.transparent[cube].extend_from_slice(&self.story_mist[1]);
                f.bounds = Vec4::new(GATEWAY.x - 4.5, GATEWAY.z - 3.5, GATEWAY.x + 4.5, GATEWAY.z + 4.5);
                self.emit_gateway(sv, s, f);
            }
        }
    }

    // -------------------------------------------------------- the graveyard

    fn emit_graveyard(&self, sv: StoryView, s: &SceneState, f: &mut Frame) {
        let time = s.time as f32;
        let t = sv.t;
        let shot = sv.shot();
        let cube = MeshKind::Cube as usize;
        let at = |p: Vec3| GRAVEYARD + p;
        let taken = sv.since(Shot::Taken);
        let pursuit = if shot == Shot::Pursuit { t } else { -1.0 };
        // The light dims and the fog closes in once Krampus comes, and thickens as they chase him.
        let darken = smooth(stage::EYES, stage::GRAB, taken).max(if taken > 100.0 { 1.0 } else { 0.0 });
        let fog = if pursuit >= 0.0 { smooth(0.6, stage::FLASH + 0.4, pursuit) } else { 0.0 };
        f.atmosphere = dusk(darken * 0.6, fog);

        // ---- the ball
        let feet_of = |i: usize| KID_HOME[i] + Vec3::new(0.0, BALL_R, 0.0);
        let ball = match shot {
            Shot::Title | Shot::Establish => {
                let t = if shot == Shot::Title { 0.0 } else { t };
                let mut p = feet_of(0) + Vec3::new(0.08, 0.0, -0.1);
                for (n, &(a, b)) in PASSES.iter().enumerate() {
                    let k = stage::KICKS[n];
                    if t >= k {
                        let u = ((t - k) / 0.62).min(1.0);
                        p = feet_of(a).lerp(feet_of(b), ease_out(u)) + Vec3::new(0.0, (u * PI).sin() * 0.12, 0.0);
                    }
                }
                if t >= stage::MISKICK {
                    let u = ((t - stage::MISKICK) / (stage::BALL_AT_GATE - stage::MISKICK)).min(1.0);
                    let gate = Vec3::new(0.02, BALL_R, -2.0);
                    // Bouncing away in three shrinking hops.
                    let hop = ((u * 3.0).fract() * PI).sin() * 0.35 * (1.0 - u) + 0.0;
                    p = feet_of(2).lerp(gate, u) + Vec3::new(0.0, hop, 0.0);
                    if t >= stage::BALL_AT_GATE {
                        let v = ((t - stage::BALL_AT_GATE) / 1.1).min(1.0);
                        p = gate.lerp(BALL_REST, ease_out(v));
                    }
                }
                Some(p)
            }
            _ => Some(BALL_REST),
        };
        if let Some(p) = ball {
            let spin = Quat::from_rotation_x(-p.z * 9.0) * Quat::from_rotation_z(p.x * 9.0);
            f.opaque[MeshKind::Sphere as usize].push(Instance::new(Mat4::from_scale_rotation_translation(Vec3::splat(BALL_R), spin, at(p)), lin(0x9a5a2e), 1.0, Vec3::ZERO, SHADE_LIT));
            // A darker band round it.
            let band = Mat4::from_scale_rotation_translation(Vec3::new(BALL_R * 1.02, BALL_R * 0.3, BALL_R * 1.02), spin, at(p));
            f.opaque[MeshKind::Sphere as usize].push(Instance::new(band, lin(0x4a2a16), 1.0, Vec3::ZERO, SHADE_LIT));
        }

        // ---- the gate's two leaves: ajar, then burst open by Krampus, swinging to rest
        let open = if taken < stage::GATE_BURST {
            0.12 + if taken > stage::EYES { (time * 30.0).sin() * 0.015 } else { 0.0 }
        } else {
            let u = (taken - stage::GATE_BURST).min(60.0);
            1.4 + 0.5 * (-2.5 * u).exp() * (8.0 * u).cos() * smooth(0.0, 0.05, u) - 1.28 * (1.0 - smooth(0.0, 0.12, u))
        };
        for side in [-1.0f32, 1.0] {
            // Each leaf swings out toward the path about its hinge on the pillar.
            let hinge = at(Vec3::new(side * 1.06, 0.0, -2.0));
            let q = Quat::from_rotation_y(side * open);
            for &(off, size) in gate_leaf().iter() {
                let local = Vec3::new(-side * off.x, off.y, off.z);
                f.opaque[cube].push(Instance::new(Mat4::from_scale_rotation_translation(size, q, hinge + q * local), lin(0x1c1c20), 1.0, Vec3::ZERO, SHADE_METAL));
            }
        }

        // ---- the lantern on its bracket, flickering as he comes, out for a moment when he strikes
        let lamp = at(Vec3::new(1.34, 1.64, -1.1));
        let mut flick = 1.0 + (time * 11.0).sin() * 0.04 + (time * 5.3).sin() * 0.05;
        if (0.0..stage::GONE + 0.4).contains(&taken) {
            let n = (time * 23.0).sin() * (time * 7.7).cos();
            flick *= if n > 0.35 { 0.15 } else { 0.8 };
            if (stage::GRAB..stage::GRAB + 0.55).contains(&taken) {
                flick = 0.45 + 0.1 * (time * 40.0).sin();
            }
        }
        let swing = (time * 1.3).sin() * 0.06 + if (stage::RUSH..stage::GONE).contains(&taken) { (time * 17.0).sin() * 0.12 } else { 0.0 };
        let lq = Quat::from_rotation_x(swing);
        f.opaque[cube].push(Instance::new(Mat4::from_scale_rotation_translation(Vec3::new(0.02, 0.12, 0.02), lq, lamp + lq * Vec3::new(0.0, 0.14, 0.0)), lin(0x1c1c20), 1.0, Vec3::ZERO, SHADE_METAL));
        f.opaque[cube].push(Instance::new(Mat4::from_scale_rotation_translation(Vec3::new(0.15, 0.2, 0.15), lq, lamp), lin(0xffd49a), 1.0, lin(0xffa040) * 2.2 * flick, SHADE_LIT));
        f.opaque[cube].push(Instance::new(Mat4::from_scale_rotation_translation(Vec3::new(0.19, 0.04, 0.19), lq, lamp + lq * Vec3::new(0.0, 0.12, 0.0)), lin(0x1c1c20), 1.0, Vec3::ZERO, SHADE_METAL));
        glow_orb(f, lamp, 0.7, lin(0xffa050), 0.3 * flick);
        f.lights.push(Light { pos: lamp + Vec3::new(0.0, -0.15, 0.35), color: lin(0xffb060) * 4.2 * flick, distance: 5.5 });
        // A soft warm fill on the friends from where the camera stands, so faces read against the sunset.
        f.lights.push(Light { pos: at(Vec3::new(0.4, 2.2, 4.2)), color: lin(0xffc8a8) * 3.2 * (1.0 - 0.4 * darken), distance: 7.5 });
        f.lights.push(Light { pos: at(Vec3::new(3.6, 0.8, -17.0)), color: lin(0xffa050) * 3.0, distance: 4.0 });

        // ---- crows on the gate and the pillar, flying off when the eyes open in the fog
        for (i, perch) in [Vec3::new(0.34, 2.92, -2.0), Vec3::new(1.34, 2.93, -2.0), Vec3::new(-0.42, 2.8, -2.0)].into_iter().enumerate() {
            let off = taken - stage::EYES - i as f32 * 0.12;
            let (pos, flap, yaw) = if off > 0.0 {
                let dir = Vec3::new([0.6, 1.0, -0.9][i], 0.0, -1.0).normalize();
                (perch + dir * off * 3.2 + Vec3::new(0.0, off * 1.6 + off * off * 0.4, 0.0), (time * 34.0 + i as f32).sin(), dir.x.atan2(dir.z))
            } else {
                (perch, -0.2, [0.4, -0.9, 2.6][i] + (time * 0.7 + i as f32 * 2.0).sin() * 0.5)
            };
            if off > 4.0 {
                continue;
            }
            self.emit_crow(f, at(pos), yaw, flap, (time * 3.0 + i as f32).sin() * 0.3);
        }

        // ---- the friends
        let establish = if shot == Shot::Establish { t } else if shot == Shot::Title { 0.0 } else { 100.0 };
        let ball_now = ball.unwrap_or(BALL_REST);
        for i in 0..4 {
            let home = KID_HOME[i];
            let mut feet = home;
            let mut yaw = yaw_to(home, if establish < stage::MISKICK + 0.3 { ball_now } else { GATE_SPOT });
            let mut act = Act::Stand;
            let mut look = 0.0;
            let mut hidden = false;
            // Kicks and runs during the establishing shot.
            if establish < 100.0 {
                for (n, &(a, _)) in PASSES.iter().enumerate() {
                    let k = establish - stage::KICKS[n];
                    if a == i && (-0.25..0.45).contains(&k) {
                        act = Act::Kick(k);
                    }
                }
                if i == 2 && (-0.25..0.45).contains(&(establish - stage::MISKICK)) {
                    act = Act::Kick(establish - stage::MISKICK);
                }
                if i == 0 && establish >= stage::FETCH {
                    let u = ((establish - stage::FETCH) / (stage::AT_GATE - stage::FETCH)).min(1.0);
                    feet = home.lerp(GATE_SPOT, u);
                    yaw = yaw_to(home, GATE_SPOT);
                    act = if u < 1.0 { Act::Run { phase: establish * 13.0, pace: 1.0 } } else { Act::Peer };
                }
                if i != 0 && establish > stage::BALL_AT_GATE {
                    look = (establish * 1.3 + i as f32).sin() * 0.25;
                }
            } else if taken >= 0.0 {
                if i == 0 {
                    // The one at the gate: startled, turning to run, seized and carried off.
                    feet = GATE_SPOT;
                    yaw = PI;
                    act = if taken < stage::EYES { Act::Peer } else { Act::Scared(smooth(stage::EYES, stage::EYES + 0.3, taken) * 0.4) };
                    if taken > stage::RUSH {
                        let u = smooth(stage::RUSH, stage::GRAB, taken);
                        yaw = PI - u * 2.6;
                        feet = GATE_SPOT + Vec3::new(0.0, 0.0, u * 0.28);
                        act = Act::Run { phase: taken * 15.0, pace: u };
                    }
                    if taken >= stage::GRAB {
                        hidden = true; // carried: drawn with Krampus
                    }
                } else {
                    let fright = smooth(stage::RUSH, stage::RUSH + 0.35, taken);
                    feet = home + Vec3::new(0.0, 0.0, fright * 0.45);
                    yaw = yaw_to(home, GATE_SPOT + Vec3::new(0.0, 0.0, -1.5));
                    act = if fright > 0.0 { Act::Scared(fright) } else { Act::Stand };
                    if pursuit >= 0.0 {
                        // After him: through the gate and into the fog.
                        let start = (i - 1) as f32 * 0.14;
                        let u = ((pursuit - start) / 2.4).clamp(0.0, 1.0);
                        let from = home + Vec3::new(0.0, 0.0, 0.45);
                        let gate = Vec3::new((i as f32 - 2.0) * 0.28, 0.0, -2.0);
                        let beyond = Vec3::new((i as f32 - 2.0) * 0.5, 0.0, -5.6);
                        feet = if u < 0.5 { from.lerp(gate, u * 2.0) } else { gate.lerp(beyond, (u - 0.5) * 2.0) };
                        yaw = if u < 0.5 { yaw_to(from, gate) } else { yaw_to(gate, beyond) };
                        act = if pursuit < start { Act::Scared(1.0) } else { Act::Run { phase: pursuit * 14.0 + i as f32, pace: 1.0 } };
                    }
                }
            }
            if hidden || (i == 0 && pursuit >= 0.0) {
                continue;
            }
            self.emit_kid(f, i, kid_root(at(feet), yaw), act, time, look, Vec3::ZERO);
        }

        // ---- Krampus: eyes in the fog, the rush, the grab, and back into the graves with the boy
        if (0.0..stage::GONE).contains(&taken) {
            let k = taken;
            let (mut pos, mut yaw, mut run, mut reach, mut lift, mut rear) = (K_FOG, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32);
            let mut alpha = smooth(stage::EYES, stage::RUSH, k) * 0.45;
            if k >= stage::RUSH {
                let u = ease_in((k - stage::RUSH) / (stage::GATE_BURST - stage::RUSH));
                pos = K_FOG.lerp(K_GATE, u);
                run = 1.0;
                alpha = 0.45 + 0.55 * u;
            }
            if k >= stage::GATE_BURST {
                let u = ease_out((k - stage::GATE_BURST) / (stage::GRAB - stage::GATE_BURST));
                pos = K_GATE.lerp(K_GRAB, u);
                reach = u;
                run = 1.0 - u;
                alpha = 1.0;
            }
            if k >= stage::GRAB {
                let u = smooth(stage::GRAB, stage::GRAB + 0.3, k);
                reach = 1.0 - u;
                lift = u;
                rear = smooth(stage::GRAB, stage::GRAB + 0.25, k) * (1.0 - smooth(stage::TURN - 0.2, stage::TURN, k));
            }
            if k >= stage::TURN {
                let u = smooth(stage::TURN, stage::TURN + 0.3, k);
                yaw = u * PI;
                let v = ease_in((k - stage::TURN - 0.15) / (stage::GONE - stage::TURN - 0.15));
                pos = K_GRAB.lerp(K_GONE, v);
                run = smooth(stage::TURN, stage::TURN + 0.25, k);
                alpha = 1.0 - smooth(stage::TURN + 0.45, stage::GONE, k);
            }
            let hop = if run > 0.0 { (k * 9.0).sin().abs() * 0.12 * run } else { 0.0 };
            let hand = self.emit_krampus(f, at(pos + Vec3::new(0.0, hop, 0.0)), yaw, run, k * 11.0, reach, lift, rear, alpha, time);
            if k >= stage::GRAB {
                // The lantern's glow still catches the boy.
                f.lights.push(Light { pos: hand + Vec3::new(0.35, 0.1, 0.7), color: lin(0xffc080) * 1.0 * alpha, distance: 1.3 });
                // The boy in his claws: held round the middle, kicking and flailing.
                let (kw, _, kd) = cast::KID_SIZE;
                let waist = (Vec3::from(cast::KID_WAIST) - Vec3::new((kw - 1) as f32 / 2.0, 0.0, (kd - 1) as f32 / 2.0)) * KID_VOXEL;
                let root = Mat4::from_translation(hand) * Mat4::from_rotation_y(yaw + PI * 0.5) * Mat4::from_rotation_z(-1.1 + (time * 9.0).sin() * 0.12) * Mat4::from_translation(-waist);
                if alpha > 0.5 {
                    self.emit_kid(f, 0, root, Act::Flail(time), time, 0.0, Vec3::ZERO);
                } else {
                    let (pose, _) = self.kid_pose(0, Act::Flail(time), time, 0.0);
                    emit_rig_faded(&mut f.opaque[cube], &self.models.kids[0], &pose, root, alpha * 2.0);
                }
            }
            // Smoke boiling off him as he moves, left hanging in the air behind him.
            let back = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
            for n in 0..40 {
                let fk = n as f32;
                let age = (time * 1.1 + fk * 0.37).fract();
                let p = at(pos) + back * (age * (0.3 + run * 1.2)) + Vec3::new((fk * 2.3).sin() * 0.35, 0.15 + (fk * 0.61).fract() * 0.9 + age * 0.25, (fk * 1.7).cos() * 0.3);
                let size = 0.05 + age * 0.13;
                f.transparent[MeshKind::Sphere as usize].push(Instance::new(Mat4::from_scale_rotation_translation(Vec3::splat(size), Quat::IDENTITY, p), lin(0x040306), alpha.max(0.3) * 0.13 * (1.0 - age) * (0.3 + 0.7 * run.max(reach)), Vec3::ZERO, SHADE_UNLIT));
            }
        }
        // A cold wind of leaves and grit when he strikes.
        if (stage::RUSH..stage::GONE + 1.5).contains(&taken) {
            for n in 0..30 {
                let fk = n as f32;
                let u = ((taken - stage::RUSH) * 0.8 + fk * 0.113).fract();
                let p = at(Vec3::new(-2.5 + u * 5.0 + (fk * 3.1).sin() * 0.4, 0.1 + ((u * 6.0 + fk).sin() * 0.5 + 0.5) * 0.8, -1.8 + (fk * 1.9).cos() * 1.5 + u * 2.0));
                let c = [0x8a4a1e, 0x9a6a24, 0x3a2a1a][n % 3];
                f.opaque[cube].push(Instance::new(Mat4::from_scale_rotation_translation(Vec3::new(0.06, 0.008, 0.04), Quat::from_rotation_x(u * 20.0 + fk) * Quat::from_rotation_y(fk), p), lin(c), 1.0, Vec3::ZERO, SHADE_LIT));
            }
        }
    }

    fn emit_crow(&self, f: &mut Frame, pos: Vec3, yaw: f32, flap: f32, head: f32) {
        let cube = MeshKind::Cube as usize;
        let q = Quat::from_rotation_y(yaw);
        let black = lin(0x101014);
        let put = |f: &mut Frame, off: Vec3, s: Vec3, r: Quat, c: Vec3| f.opaque[cube].push(Instance::new(Mat4::from_scale_rotation_translation(s, q * r, pos + q * off), c, 1.0, Vec3::ZERO, SHADE_LIT));
        put(f, Vec3::new(0.0, 0.05, 0.0), Vec3::new(0.07, 0.07, 0.15), Quat::from_rotation_x(-0.3), black);
        let hq = Quat::from_rotation_y(head);
        put(f, Vec3::new(0.0, 0.1, 0.08), Vec3::splat(0.055), hq, black);
        put(f, Vec3::new(0.0, 0.095, 0.08) + hq * Vec3::new(0.0, 0.0, 0.05), Vec3::new(0.018, 0.018, 0.05), hq, lin(0x2a2622));
        put(f, Vec3::new(0.0, 0.03, -0.1), Vec3::new(0.05, 0.015, 0.08), Quat::from_rotation_x(0.3), black);
        for side in [-1.0f32, 1.0] {
            let r = Quat::from_rotation_z(side * flap * 0.9);
            put(f, Vec3::new(side * 0.035, 0.07, 0.0) + r * Vec3::new(side * 0.06, 0.0, 0.0), Vec3::new(0.12, 0.012, 0.08), r, black);
        }
    }

    // ---------------------------------------------------------- the gateway

    fn emit_gateway(&self, sv: StoryView, s: &SceneState, f: &mut Frame) {
        let time = s.time as f32;
        let t = sv.t;
        let shot = sv.shot();
        let cube = MeshKind::Cube as usize;
        let at = |p: Vec3| GATEWAY + p;
        f.atmosphere = crypt();
        let arrive = sv.since(Shot::Arrive);
        let easy = sv.since(Shot::Easy);
        let descent = sv.since(Shot::Descent).min(60.0);

        gateway_lights(f, time);

        // ---- the door: shut, then grinding open on the stair and its red glow
        let open = smooth(stage::DOOR, stage::DOOR + 2.0, descent) * 1.45;
        gateway_door(f, open, time);

        // ---- the way they came: a swirling rent in the arch, and it closing
        if arrive < stage::CLOSE + 0.45 {
            let c = at(Vec3::new(0.0, 1.15, 4.12));
            let close = smooth(stage::CLOSE, stage::CLOSE + 0.35, arrive);
            let open_k = smooth(0.0, 0.25, arrive) * (1.0 - close);
            let flash = if arrive > stage::CLOSE { (1.0 - (arrive - stage::CLOSE) / 0.45).max(0.0) } else { 0.0 };
            for k in 0..7 {
                let r = (0.25 + k as f32 * 0.12) * open_k;
                let spin = time * (2.0 + k as f32 * 0.6) * if k % 2 == 0 { 1.0 } else { -1.0 };
                let m = Mat4::from_translation(c + Vec3::new(0.0, 0.0, -0.01 * k as f32)) * Mat4::from_rotation_z(spin) * Mat4::from_scale(Vec3::new(r * 1.25, r * 1.7, 1.0));
                f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, lin(if k % 2 == 0 { 0xb8a0ff } else { 0x7ae0ff }) * 1.6, 0.45 * open_k, Vec3::ZERO, SHADE_UNLIT));
            }
            let disc = Mat4::from_translation(c + Vec3::new(0.0, 0.0, 0.02)) * Mat4::from_scale(Vec3::new(0.82 * open_k, 1.12 * open_k, 1.0));
            f.transparent[MeshKind::Disc as usize].push(Instance::new(disc, lin(0x4a3a8a), 0.85 * open_k, Vec3::ZERO, SHADE_UNLIT));
            glow_orb(f, c, 0.9 + flash * 2.2, lin(0xa898ff), 0.16 * open_k + flash * 0.7);
            f.lights.push(Light { pos: c + Vec3::new(0.0, 0.0, -0.5), color: lin(0xb8a8ff) * (4.0 * open_k + 16.0 * flash), distance: 6.0 });
            // Wisps of the graveyard's fog pouring through.
            for n in 0..16 {
                let fk = n as f32;
                let u = (time * 0.9 + fk * 0.21).fract();
                let p = c + Vec3::new((fk * 2.1).sin() * 0.5 * (1.0 - u), -0.9 + (fk * 1.3).cos() * 0.2 + u * 0.3, -u * 1.6);
                f.transparent[MeshKind::Sphere as usize].push(Instance::new(Mat4::from_scale_rotation_translation(Vec3::splat(0.12 + u * 0.25), Quat::IDENTITY, p), lin(0xb4a8c8), 0.2 * (1.0 - u) * open_k, Vec3::ZERO, SHADE_LIT));
            }
        }

        // ---- the souls of the three warriors, rising before the door, then going into the boys
        let taken_k = smooth(stage::SOULS_TAKEN, stage::SOULS_TAKEN + 1.2, descent);
        for i in 0..3 {
            let rise = self.story_rise[i];
            if rise <= 0.0 || taken_k >= 1.0 {
                continue;
            }
            let kind = [crate::logic::heroes::HeroKind::Mehmed, crate::logic::heroes::HeroKind::Ragnar, crate::logic::heroes::HeroKind::Miyamoto][i];
            let boy = self.story_boy_feet(sv, i);
            let base = SOUL_AT[i].lerp(boy + Vec3::new(0.0, 0.2, 0.0), ease_in(taken_k)) + Vec3::new(0.0, (taken_k * PI).sin() * 0.6, 0.0);
            self.emit_soul(f, kind, at(base), yaw_to(SOUL_AT[i], BOYS_MID), rise, 1.0 - taken_k * 0.85, lin(SOUL_TINT[i]), time + i as f32 * 1.3);
        }

        // ---- Quitmer
        let (mut feet, mut yaw, mut strain, mut lunge, mut snap) = (DOG_BED, yaw_to(DOG_BED, BOYS_MID) - 0.4, 0.0f32, 0.0f32, 0.0f32);
        let mut sleep = 0.0;
        let (calm, wag): (f32, f32);
        if arrive < stage::GROWL - 0.4 {
            // Asleep on his straw until the boys come tumbling in.
            sleep = 1.0 - smooth(stage::TUMBLE[0] + 0.6, stage::TUMBLE[0] + 1.0, arrive);
            calm = 1.0 - smooth(1.9, 2.6, arrive);
            wag = 0.0;
        } else if easy < 0.5 {
            // Up, and at them as far as his chain allows.
            let u = smooth(stage::GROWL - 0.4, stage::GROWL + 0.4, arrive);
            calm = 0.0;
            strain = u;
            feet = DOG_BED.lerp(DOG_LUNGE, u);
            yaw = yaw_to(feet, BOYS_MID);
            lunge = ((time * 5.5).sin() * 0.5 + 0.5).powf(2.0) * strain;
            snap = ((time * 7.0).sin() * 0.5 + 0.5) * strain;
            wag = 0.0;
        } else {
            // "Easy, Quitmer." He backs down and lies on his straw, watching.
            let back = smooth(0.5, 1.1, easy);
            let down = smooth(1.0, 2.1, easy);
            strain = 1.0 - back;
            feet = DOG_LUNGE.lerp(DOG_BED, down);
            yaw = lerp_angle(yaw_to(DOG_LUNGE, BOYS_MID), yaw_to(DOG_BED, BOYS_MID) - 0.4, down);
            calm = down;
            snap = ((time * 7.0).sin() * 0.5 + 0.5) * strain;
            wag = 0.6 * down;
        }
        let collar = self.emit_dog(f, at(feet), yaw, strain, lunge, calm, snap, sleep, wag, time);
        if strain > 0.1 {
            f.lights.push(Light { pos: at(feet + Vec3::new(1.1, 1.1, -0.4)), color: lin(0xffc89a) * 2.4 * strain, distance: 3.0 });
        }
        // His chain, taut when he strains, slack on the flagstones when he lies.
        let ring = at(CHAIN_RING);
        let span = collar - ring;
        let d = span.length();
        let sag = ((CHAIN_LEN * CHAIN_LEN - d * d).max(0.0)).sqrt() * 0.5;
        let links = (CHAIN_LEN / 0.055) as usize;
        for n in 0..links {
            let u = n as f32 / links as f32;
            let mut p = ring + span * u - Vec3::new(0.0, sag * (u * PI).sin(), 0.0);
            p.y = p.y.max(GATEWAY.y + 0.02);
            let q = Quat::from_rotation_arc(Vec3::Z, span.normalize_or(Vec3::Z)) * if n % 2 == 0 { Quat::IDENTITY } else { Quat::from_rotation_z(PI / 2.0) };
            f.opaque[cube].push(Instance::new(Mat4::from_scale_rotation_translation(Vec3::new(0.035, 0.012, 0.06), q, p), lin(0x3a3c40), 1.0, Vec3::ZERO, SHADE_METAL));
        }

        // ---- the old man
        let dog_dir = yaw_to(OLD_MAN_AT, feet);
        let boys_dir = yaw_to(OLD_MAN_AT, BOYS_MID);
        let mut om_yaw = boys_dir;
        let mut head = 0.0;
        let mut doze = 0.0;
        let mut raise = 0.0;
        let mut talk = None;
        let mut om_feet = OLD_MAN_AT;
        if arrive < stage::REVEAL + 0.3 {
            doze = 1.0 - smooth(2.2, 2.45, arrive);
            head = (time * 0.4).sin() * 0.08 * doze;
        }
        if shot == Shot::Snarl {
            head = (dog_dir - boys_dir) * 0.5 * smooth(0.3, 1.0, t);
        }
        if shot == Shot::Easy {
            om_yaw = lerp_angle(boys_dir, dog_dir, smooth(0.0, 0.5, t) * 0.6);
            head = (dog_dir - om_yaw) * 0.6 * smooth(0.0, 0.5, t);
            raise = smooth(0.1, 0.5, t) * (1.0 - smooth(1.6, 2.2, t));
            talk = Some(t);
        }
        if sv.speaker() == Speaker::OldMan && shot != Shot::Easy {
            talk = Some(t);
            if Some(sv.beat) == first(Shot::OldMan) {
                // Turning from the dog to the boys.
                om_yaw = lerp_angle(lerp_angle(boys_dir, dog_dir, 0.6), boys_dir, smooth(0.0, 0.8, t));
            }
        }
        if shot == Shot::Souls {
            // An arm sweeping toward the door as the souls come up.
            raise = smooth(0.6, 1.4, t) * 0.8;
            head = -0.5 * smooth(0.8, 1.6, t) * (1.0 - smooth(3.5, 4.5, t));
        }
        if descent < 100.0 && descent >= 0.0 {
            // Stepping aside for them; a hand raised to see them off.
            om_feet = OLD_MAN_AT.lerp(OLD_MAN_AT + Vec3::new(0.45, 0.0, 0.25), smooth(stage::DOOR - 0.6, stage::DOOR + 0.6, descent));
            om_yaw = yaw_to(om_feet, BOYS_MID.lerp(Vec3::new(0.0, 0.0, -3.4), smooth(stage::WALK, stage::WALK + 5.0, descent)));
            raise = smooth(stage::WALK, stage::WALK + 0.6, descent) * 0.9;
        }
        self.emit_old_man(f, at(om_feet), om_yaw, head, doze, raise, talk, time);

        // ---- the three boys
        let speaker_boy = (sv.boys_lines() % 3) + 1;
        for i in 1..4usize {
            let b = i - 1;
            let tumble = arrive - stage::TUMBLE[b];
            if tumble < 0.0 {
                continue;
            }
            let mut feet = self.story_boy_feet(sv, b);
            let mut yaw = yaw_to(feet, OLD_MAN_AT.lerp(DOG_BED, 0.5));
            let mut act = Act::Stand;
            let mut look = 0.0;
            let mut root = None;
            if tumble < 0.5 {
                // Thrown out of the rent in the air, rolling over as they land.
                let u = tumble / 0.5;
                let from = Vec3::new(LAND[b].x * 0.6, 0.55, 3.95);
                let p = from.lerp(LAND[b], ease_out(u)) + Vec3::new(0.0, (u * PI).sin() * 0.25, 0.0);
                let spin = u * (PI * 2.0 + PI * 0.5);
                root = Some(Mat4::from_translation(at(p) + Vec3::new(0.0, 0.1, 0.0)) * Mat4::from_rotation_y(PI) * Mat4::from_rotation_x(spin) * Mat4::from_translation(Vec3::new(0.0, -0.1, 0.0)));
                act = Act::Flail(time);
            } else if tumble < 1.9 {
                let up = smooth(1.3, 1.9, tumble);
                yaw = PI + (b as f32 - 1.0) * 0.3;
                act = Act::Up(up);
            } else if arrive < stage::REVEAL + 0.4 {
                // Up, and looking round at where they are.
                look = (time * 1.1 + b as f32 * 2.0).sin() * 0.6;
                let face = yaw_to(feet, OLD_MAN_AT.lerp(DOG_BED, 0.5));
                yaw = lerp_angle(PI + (b as f32 - 1.0) * 0.3, face, smooth(stage::REVEAL - 0.6, stage::REVEAL + 0.2, arrive));
            }
            if shot == Shot::Snarl || (shot == Shot::Arrive && arrive > stage::GROWL) {
                let fright = if shot == Shot::Snarl { 1.0 } else { smooth(stage::GROWL, stage::GROWL + 0.4, arrive) };
                feet += Vec3::new(0.0, 0.0, 0.22 * fright);
                act = Act::Scared(fright * if b == 0 { 1.0 } else { 0.55 });
                yaw = yaw_to(feet, DOG_LUNGE);
            }
            if sv.speaker() == Speaker::Boys && i == speaker_boy && shot != Shot::Descent {
                act = Act::Talk(t);
            }
            if shot == Shot::Souls || (shot == Shot::Boys && sv.souls != 0) {
                yaw = yaw_to(feet, SOUL_AT[1]);
                look = (SOUL_AT[b].x - feet.x) * 0.3;
            }
            let mut glow = Vec3::ZERO;
            if descent < 100.0 && descent >= 0.0 {
                let arrive_at = stage::SOULS_TAKEN + 1.2;
                let g = (1.0 - (descent - arrive_at).abs() / 0.9).max(0.0) + if descent > arrive_at { 0.15 * (1.0 - smooth(arrive_at, arrive_at + 3.0, descent)) } else { 0.0 };
                glow = lin(SOUL_TINT[b]) * g * 0.22;
                if descent > stage::WALK {
                    let walk = descent - stage::WALK - b as f32 * 0.25;
                    if walk > 0.0 {
                        act = Act::Run { phase: walk * 8.5, pace: 0.5 };
                    }
                    yaw = yaw_to(feet, Vec3::new(feet.x * 0.3, 0.0, -4.5));
                }
            }
            let root = root.unwrap_or_else(|| kid_root(at(feet), yaw));
            self.emit_kid(f, i, root, act, time, look, glow);
        }

        // ---- drips from the vault, rings spreading where they land; dust in the torchlight
        for (n, &(x, z)) in [(-1.2f32, 1.4f32), (2.2, 0.4), (-0.4, -1.6), (1.1, 2.9)].iter().enumerate() {
            let period = 2.3 + n as f32 * 0.7;
            let u = ((time + n as f32 * 1.1) / period).fract() * period;
            let fall = 0.55;
            if u < fall {
                let y = 3.6 - (u / fall).powi(2) * 3.6;
                f.additive.push(Instance::new(Mat4::from_translation(at(Vec3::new(x, y, z))) * Mat4::from_scale(Vec3::new(0.012, 0.035, 0.012)), lin(0xa8c8e8), 0.7, Vec3::ZERO, SHADE_UNLIT));
            } else if u < fall + 0.8 {
                let r = (u - fall) / 0.8;
                let m = Mat4::from_translation(at(Vec3::new(x, 0.012, z))) * Mat4::from_rotation_x(-PI / 2.0) * Mat4::from_scale(Vec3::new(0.05 + r * 0.25, 0.05 + r * 0.25, 1.0));
                f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, lin(0xa8b8c8), 0.45 * (1.0 - r), Vec3::ZERO, SHADE_UNLIT));
            }
        }
        for k in 0..30 {
            let fk = k as f32;
            let p = at(Vec3::new(((fk * 0.618).fract() - 0.5) * 7.0 + (time * 0.15 + fk).sin() * 0.25, 0.3 + (fk * 0.37).fract() * 2.8 + (time * 0.2 + fk * 1.3).sin() * 0.12, ((fk * 0.29).fract() - 0.5) * 6.0));
            glow_orb(f, p, 0.02, lin(0xffe0b0), 0.25 + 0.2 * (time * 1.7 + fk).sin());
        }
    }

    /// Where boy `b` (of the three in the gateway) stands: landed, then walking to the door and down.
    fn story_boy_feet(&self, sv: StoryView, b: usize) -> Vec3 {
        let descent = sv.since(Shot::Descent).min(60.0);
        let home = LAND[b] + Vec3::new(0.0, 0.0, -0.05);
        if descent < 0.0 || descent < stage::WALK {
            return home;
        }
        let walk = (descent - stage::WALK - b as f32 * 0.25).max(0.0);
        let door = Vec3::new((b as f32 - 1.0) * 0.32, 0.0, -3.35);
        let path = home.distance(door);
        let d = walk * 0.95;
        if d < path {
            home.lerp(door, d / path)
        } else {
            // Down the stair beyond, into the red light.
            let e = d - path;
            door + Vec3::new(0.0, -e * 0.55, -e)
        }
    }

    // --------------------------------------------------------- the cast

    /// A boy's limbs for what he is doing, and how far he leans and bobs.
    fn kid_pose(&self, which: usize, act: Act, time: f32, look: f32) -> (Pose, (f32, f32)) {
        let rig = &self.models.kids[which.min(3)];
        let mut pose = Pose::rest(rig);
        let idx = |n: &str| rig.index(n).unwrap_or(0);
        let (la, lb, aa, ab, hd) = (idx("legA"), idx("legB"), idx("armA"), idx("armB"), idx("head"));
        let breathe = (time * 2.2 + which as f32 * 1.3).sin();
        let (mut lean, mut bob) = (0.0, 0.0);
        pose.rot[hd].y = look;
        pose.rot[aa].z = -0.06;
        pose.rot[ab].z = 0.06;
        match act {
            Act::Stand => {
                pose.rot[hd].x = breathe * 0.03;
                pose.rot[aa].x = breathe * 0.04;
                pose.rot[ab].x = -breathe * 0.04;
            }
            Act::Run { phase, pace } => {
                let s = phase.sin();
                pose.rot[la].x = -0.95 * s * pace;
                pose.rot[lb].x = 0.95 * s * pace;
                pose.rot[aa].x = 0.85 * s * pace;
                pose.rot[ab].x = -0.85 * s * pace;
                lean = 0.2 * pace;
                bob = phase.cos().abs() * 0.035 * pace;
            }
            Act::Kick(k) => {
                // Wind up, swing through, follow through.
                let swing = if k < 0.0 { 0.9 * smooth(-0.25, -0.05, k) } else { 0.9 - 2.2 * smooth(0.0, 0.12, k) + 1.3 * smooth(0.15, 0.45, k) };
                pose.rot[lb].x = swing;
                pose.rot[aa].x = -0.5 * (1.0 - smooth(0.2, 0.45, k));
                pose.rot[ab].x = 0.4 * (1.0 - smooth(0.2, 0.45, k));
                lean = -0.08;
            }
            Act::Peer => {
                lean = 0.22;
                pose.rot[hd].x = 0.2;
                pose.rot[aa].x = -1.2 + (time * 3.0).sin() * 0.05;
                pose.rot[ab].x = -0.3;
            }
            Act::Scared(a) => {
                let shake = (time * 31.0 + which as f32).sin() * 0.04 * a;
                pose.rot[aa].x = -2.5 * a + shake;
                pose.rot[ab].x = -2.5 * a - shake;
                pose.rot[aa].z = -0.35 * a;
                pose.rot[ab].z = 0.35 * a;
                pose.rot[hd].x = -0.15 * a;
                lean = -0.12 * a;
            }
            Act::Talk(t) => {
                pose.rot[hd].x = (t * 8.0).sin() * 0.07;
                pose.rot[ab].x = -0.7 - (t * 4.0).sin() * 0.25;
                pose.rot[ab].z = 0.25;
                pose.rot[aa].x = -0.2;
                bob = ((t * 8.0).sin() * 0.5 + 0.5) * 0.012;
            }
            Act::Up(u) => {
                // Arms pushing off the floor, then a knee under him, then up.
                pose.rot[aa].x = -1.4 * (1.0 - u);
                pose.rot[ab].x = -1.4 * (1.0 - u);
                pose.rot[la].x = -1.2 * (u * PI).sin();
                pose.rot[hd].x = -0.4 * (1.0 - u);
                lean = (1.0 - u) * PI * 0.5;
                bob = (1.0 - u) * 0.14;
            }
            Act::Flail(t) => {
                pose.rot[la].x = (t * 15.0).sin() * 0.9;
                pose.rot[lb].x = -(t * 15.0).sin() * 0.9;
                pose.rot[aa].x = -2.2 + (t * 13.0).sin() * 0.7;
                pose.rot[ab].x = -2.2 - (t * 13.0).cos() * 0.7;
                pose.rot[hd].x = (t * 6.0).sin() * 0.2;
            }
        }
        (pose, (lean, bob))
    }

    pub(super) fn emit_kid(&self, f: &mut Frame, which: usize, root: Mat4, act: Act, time: f32, look: f32, glow: Vec3) {
        let (pose, (lean, bob)) = self.kid_pose(which, act, time, look);
        let root = root * Mat4::from_translation(Vec3::new(0.0, bob, 0.0)) * Mat4::from_rotation_x(lean);
        emit_rig(&mut f.opaque[MeshKind::Cube as usize], &self.models.kids[which.min(3)], &pose, root, Vec3::ONE, 1.0, glow);
        if glow.length_squared() > 0.0001 {
            glow_orb(f, root.transform_point3(Vec3::new(0.0, 0.35, 0.0)), 0.75, glow.normalize_or_zero(), (glow.length() * 2.5).min(1.0) * 0.45);
        }
    }

    /**
     * Quitmer: straining at his chain and snapping (`strain`, `lunge`, `snap`), or lying calm
     * on his straw (`calm`), asleep (`sleep`), his tail wagging (`wag`). Returns where his
     * collar's ring is, for the chain.
     */
    #[allow(clippy::too_many_arguments)]
    fn emit_dog(&self, f: &mut Frame, feet: Vec3, yaw: f32, strain: f32, lunge: f32, calm: f32, snap: f32, sleep: f32, wag: f32, time: f32) -> Vec3 {
        let rig = &self.models.dog;
        let mut pose = Pose::rest(rig);
        let idx = |n: &str| rig.index(n).unwrap_or(0);
        let (jaw, head, fa, fb, ha, hb, tail) = (idx("jaw"), idx("head"), idx("foreA"), idx("foreB"), idx("hindA"), idx("hindB"), idx("tail"));
        let breathe = (time * if sleep > 0.5 { 1.4 } else { 2.6 }).sin();
        // Braced legs and a lowered head when he strains; legs folded flat when he lies.
        pose.rot[fa].x = -0.4 * strain - 1.5 * calm;
        pose.rot[fb].x = -0.4 * strain - 1.5 * calm;
        pose.rot[ha].x = 0.35 * strain - 1.35 * calm;
        pose.rot[hb].x = 0.35 * strain - 1.35 * calm;
        pose.rot[head].x = 0.28 * strain + 0.06 * calm + 0.45 * sleep + breathe * 0.02;
        pose.rot[head].y = (time * 0.5).sin() * 0.15 * calm * (1.0 - sleep);
        pose.rot[jaw].x = 0.55 * snap + 0.05 * breathe.max(0.0) * (1.0 - sleep);
        pose.rot[tail].y = (time * (4.0 + wag * 6.0)).sin() * (0.15 + wag * 0.5);
        pose.rot[tail].x = -0.5 * strain + 0.6 * calm;
        let forward = Vec3::new(yaw.sin(), 0.0, yaw.cos());
        let sink = calm * 17.0 * DOG_VOXEL;
        let root = Mat4::from_translation(feet + forward * (lunge * 0.1) + Vec3::new(0.0, -sink + breathe * 0.004, 0.0))
            * Mat4::from_rotation_y(yaw)
            * Mat4::from_rotation_x(0.1 * strain + lunge * 0.05);
        emit_rig(&mut f.opaque[MeshKind::Cube as usize], rig, &pose, root, Vec3::ONE, 1.0, Vec3::ZERO);
        // Eyes glowing amber (dim when he dozes).
        for e in cast::DOG_EYES {
            let p = rig_point(rig, &pose, root, head, pivot_of(cast::DOG_RIG, "head"), [e[0], e[1], e[2] + 1.0]);
            glow_orb(f, p, 0.07, lin(0xffa030), (0.9 - 0.8 * sleep) * (1.0 + 0.3 * strain));
        }
        let (w, _, d) = cast::DOG_SIZE;
        let body_pivot = [(w - 1) as f32 / 2.0, 0.0, (d - 1) as f32 / 2.0];
        rig_point(rig, &pose, root, 0, body_pivot, cast::DOG_COLLAR_RING)
    }

    /// The old man: facing `yaw`, his head turned `head` and drooping with sleep (`doze`),
    /// his free hand raised (`raise`), talking (seconds into the line) or not.
    #[allow(clippy::too_many_arguments)]
    fn emit_old_man(&self, f: &mut Frame, feet: Vec3, yaw: f32, head: f32, doze: f32, raise: f32, talk: Option<f32>, time: f32) {
        let rig = &self.models.old_man;
        let mut pose = Pose::rest(rig);
        let idx = |n: &str| rig.index(n).unwrap_or(0);
        let (hd, staff, free) = (idx("head"), idx("staffArm"), idx("freeArm"));
        let breathe = (time * 1.6).sin();
        pose.rot[hd].y = head;
        pose.rot[hd].x = 0.5 * doze + breathe * 0.02;
        pose.rot[staff].x = breathe * 0.01;
        pose.rot[free].x = -1.25 * raise;
        pose.rot[free].z = 0.2 * raise;
        if let Some(t) = talk {
            pose.rot[hd].x += (t * 5.5).sin() * 0.05 * (1.0 - smooth(3.0, 5.0, t));
            pose.rot[free].x += -0.35 - (t * 2.7).sin() * 0.2;
        }
        let sway = (time * 0.7).sin() * 0.012 * (1.0 + 2.0 * doze);
        let root = Mat4::from_translation(feet) * Mat4::from_rotation_y(yaw) * Mat4::from_rotation_z(sway) * Mat4::from_rotation_x(0.05);
        emit_rig(&mut f.opaque[MeshKind::Cube as usize], rig, &pose, root, Vec3::ONE, 1.0, Vec3::ZERO);
    }

    /**
     * Krampus, running (`run`, stride `phase`), reaching with his right claw (`reach`), holding
     * it up with his catch (`lift`), rearing back to bellow (`rear`), and thinning away into
     * the fog as `alpha` falls. Returns where the claws of his right hand are.
     */
    #[allow(clippy::too_many_arguments)]
    fn emit_krampus(&self, f: &mut Frame, feet: Vec3, yaw: f32, run: f32, phase: f32, reach: f32, lift: f32, rear: f32, alpha: f32, time: f32) -> Vec3 {
        let rig = &self.models.krampus;
        let mut pose = Pose::rest(rig);
        let idx = |n: &str| rig.index(n).unwrap_or(0);
        let (hd, ar, al, lr, ll) = (idx("head"), idx("armR"), idx("armL"), idx("legR"), idx("legL"));
        let s = phase.sin();
        pose.rot[lr].x = -0.8 * s * run;
        pose.rot[ll].x = 0.8 * s * run;
        pose.rot[al].x = -0.9 * s * run - 0.3 * rear;
        pose.rot[ar].x = 0.9 * s * run * (1.0 - reach) - 1.5 * reach - 1.05 * lift;
        pose.rot[ar].z = -0.35 * lift;
        pose.rot[al].z = 0.3 * rear;
        pose.rot[hd].x = -0.35 * rear + (time * 3.0).sin() * 0.05;
        pose.rot[hd].y = (time * 1.7).sin() * 0.1 * (1.0 - run);
        let root = Mat4::from_translation(feet) * Mat4::from_rotation_y(yaw) * Mat4::from_rotation_x(0.28 * run - 0.25 * rear + 0.1 * reach);
        let cube = MeshKind::Cube as usize;
        if alpha >= 0.99 {
            emit_rig(&mut f.opaque[cube], rig, &pose, root, Vec3::ONE, 1.0, Vec3::ZERO);
        } else if alpha > 0.01 {
            emit_rig_faded(&mut f.opaque[cube], rig, &pose, root, alpha);
        }
        // Ember eyes, bright even when the rest of him is only a shape in the fog.
        let eyes = alpha.max(0.0).powf(0.3).min(1.0);
        let mut mid = Vec3::ZERO;
        for e in cast::KRAMPUS_EYES {
            let p = rig_point(rig, &pose, root, hd, pivot_of(cast::KRAMPUS_RIG, "head"), e);
            mid += p * 0.5;
            glow_orb(f, p, 0.12, lin(0xff3010), 1.4 * eyes);
            f.additive.push(Instance::new(Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.035)), lin(0xff6a30) * 2.0, eyes, Vec3::ZERO, SHADE_UNLIT));
        }
        let ahead = (root.transform_vector3(Vec3::Z)).normalize_or(Vec3::Z);
        // Their light falls ahead of him, on what he comes for more than on his own face.
        f.lights.push(Light { pos: mid + ahead * 1.2 - Vec3::new(0.0, 0.35, 0.0), color: lin(0xff2a10) * 0.6 * eyes, distance: 1.8 });
        rig_point(rig, &pose, root, ar, pivot_of(cast::KRAMPUS_RIG, "armR"), cast::KRAMPUS_HAND)
    }

    /// A warrior's soul: rising out of the floor in a column of light, hovering, glowing.
    #[allow(clippy::too_many_arguments)]
    fn emit_soul(&self, f: &mut Frame, kind: crate::logic::heroes::HeroKind, feet: Vec3, yaw: f32, rise: f32, scale: f32, tint: Vec3, time: f32) {
        use crate::logic::heroes::HeroKind;
        let rig = match kind {
            HeroKind::Ragnar => &self.models.ragnar,
            HeroKind::Mehmed => &self.models.mehmed,
            HeroKind::Miyamoto => &self.models.miyamoto,
        };
        let r = ease_out(rise);
        let hover = (time * 1.6).sin() * 0.035;
        let base = feet + Vec3::new(0.0, -1.2 * (1.0 - r) + 0.08 * r + hover, 0.0);
        let root = Mat4::from_translation(base) * Mat4::from_rotation_y(yaw) * Mat4::from_scale(Vec3::splat(scale));
        let pose = Pose::rest(rig);
        // Thinner up close, where many layers of it would pile up into a white glare.
        let near = (self.cam_pos.distance(base + Vec3::new(0.0, 0.6, 0.0)) / 3.6).clamp(0.4, 1.0);
        emit_rig_ghost(&mut f.transparent[MeshKind::Cube as usize], rig, &pose, root, tint * (1.0 + 0.15 * (time * 3.0).sin()), 0.24 * r * near);
        glow_orb(f, base + Vec3::new(0.0, 0.6 * scale, 0.0), 1.1 * scale, tint, 0.18 * r);
        // A ring of light on the floor where it rose, and motes streaming up.
        let ring = Mat4::from_translation(feet + Vec3::new(0.0, 0.012, 0.0)) * Mat4::from_rotation_x(-PI / 2.0) * Mat4::from_scale(Vec3::new(0.45, 0.45, 1.0));
        f.glow.push(Instance::new(ring, tint, 0.35 * r * (1.0 - (1.0 - scale) * 1.2).max(0.0), Vec3::ZERO, SHADE_GLOW));
        for k in 0..10 {
            let fk = k as f32;
            let u = (time * 0.6 + fk * 0.1).fract();
            let a = fk * 2.4 + time * 0.8;
            let p = base + Vec3::new(a.cos() * 0.3, u * 1.4 * scale, a.sin() * 0.3);
            glow_orb(f, p, 0.03, tint, 0.6 * r * (1.0 - u));
        }
        f.lights.push(Light { pos: base + Vec3::new(0.0, 0.8, 0.4), color: tint * 2.5 * r, distance: 3.0 });
    }

    // --------------------------------------------------------- the camera

    /// The story's camera for this moment: where it is, what it looks at, and what is in focus.
    pub(super) fn story_camera(&self, sv: StoryView) -> (Vec3, Vec3, Vec3) {
        let t = sv.t;
        let (g, d) = (GRAVEYARD, GATEWAY);
        let v = |x: f32, y: f32, z: f32| Vec3::new(x, y, z);
        let lerp3 = |a: Vec3, b: Vec3, u: f32| a.lerp(b, u);
        // Held by hand: a slow drift that never quite settles.
        let sway = v((sv.age * 0.53).sin() * 0.018, (sv.age * 0.71).sin() * 0.012, (sv.age * 0.37).cos() * 0.01);
        let crane_from = (g + v(4.8, 6.4, 9.6), g + v(0.0, 1.3, -6.0));
        let (eye, look, focus) = match sv.shot() {
            Shot::Title => (crane_from.0, crane_from.1, g + v(0.0, 0.5, 0.0)),
            Shot::Establish => {
                if t < stage::MISKICK + 0.35 {
                    // Down from over the graves to the friends at their game, faces lit by the lantern.
                    let u = smooth(0.0, 4.2, t);
                    let eye = lerp3(crane_from.0, g + v(2.35, 0.95, 3.7), u);
                    let look = lerp3(crane_from.1, g + v(-0.2, 0.5, 1.35), u);
                    (eye, look, g + v(-0.1, 0.4, 1.5))
                } else {
                    // The ball rolls away through the gate; one of them runs after it.
                    let w = smooth(stage::MISKICK + 0.35, stage::AT_GATE + 0.3, t);
                    (lerp3(g + v(2.9, 1.2, 2.05), g + v(2.45, 1.1, 1.3), w), g + v(-0.05, 0.72, -1.7), g + GATE_SPOT + v(0.0, 0.5, 0.0))
                }
            }
            Shot::Taken => {
                if t < stage::GRAB {
                    let u = smooth(0.0, stage::GRAB, t);
                    (lerp3(g + v(1.6, 1.15, 2.4), g + v(1.3, 1.05, 1.7), u), g + v(0.12, 0.95, -2.9), g + v(0.12, 1.0, -2.6))
                } else if t < stage::TURN + 0.25 {
                    // Low, looking up at him with the boy in his claws.
                    let u = smooth(stage::GRAB, stage::TURN + 0.25, t);
                    (g + v(0.62, 0.62, 0.55) + v(-u * 0.08, 0.02 * u, -u * 0.2), g + v(0.05, 1.05, -2.0), g + v(0.1, 1.3, -2.1))
                } else {
                    let u = smooth(stage::TURN + 0.25, 4.6, t);
                    (lerp3(g + v(0.9, 1.0, 2.6), g + v(0.75, 1.1, 2.2), u), g + v(0.25, 0.9, -3.8), g + v(0.3, 0.8, -4.5))
                }
            }
            Shot::Scream => (g + v(0.28, 0.62, -0.72), g + v(-0.05, 0.52, 1.55), g + v(0.0, 0.5, 1.5)),
            Shot::Pursuit => {
                let u = smooth(0.0, stage::FLASH + 0.4, t);
                (lerp3(g + v(0.3, 1.05, 3.9), g + v(0.1, 1.1, -0.2), u), g + v(0.0, 0.85, -4.8), g + v(0.0, 0.6, -2.5 - u * 2.0))
            }
            Shot::Arrive => {
                if t < stage::REVEAL {
                    (d + v(1.1, 1.05, -0.7), d + v(0.0, 0.75, 3.2), d + v(0.0, 0.5, 2.4))
                } else {
                    let u = smooth(stage::REVEAL, 5.6, t);
                    (lerp3(d + v(-0.1, 1.28, 3.55), d + v(-0.05, 1.22, 3.15), u), d + v(0.05, 0.8, -2.5), d + v(0.0, 0.8, -2.2))
                }
            }
            // Side on: straining at the end of his chain, lunging at the boys off to the left.
            Shot::Snarl => (d + v(0.62, 0.78, -2.3) + v(0.0, 0.0, t.min(4.0) * 0.03), d + v(-1.5, 0.5, -0.95), d + v(-1.45, 0.62, -1.0)),
            Shot::Easy => (d + v(0.55, 0.95, 0.95), d + v(-0.2, 0.62, -2.15), d + OLD_MAN_AT + v(0.0, 0.9, 0.0)),
            Shot::OldMan | Shot::Promise => {
                let (eye, look) = if sv.shot() == Shot::OldMan && sv.said_before(Speaker::OldMan) % 2 == 1 {
                    (d + v(0.55, 1.12, -0.45), d + OLD_MAN_AT + v(0.0, 0.98, 0.0))
                } else {
                    (d + v(2.4, 1.05, -0.55), d + OLD_MAN_AT + v(-0.05, 0.95, 0.0))
                };
                let push = 1.0 - 0.08 * smooth(0.0, 6.0, t);
                (look + (eye - look) * push, look, d + OLD_MAN_AT + v(0.0, 0.95, 0.0))
            }
            Shot::Boys => {
                let (eye, look) = if sv.said_before(Speaker::Boys) % 2 == 0 { (d + v(0.72, 0.82, -0.4), d + v(0.05, 0.48, 1.7)) } else { (d + v(-0.6, 0.78, -0.25), d + v(0.1, 0.5, 1.75)) };
                let push = 1.0 - 0.06 * smooth(0.0, 5.0, t);
                (look + (eye - look) * push, look, d + BOYS_MID + v(0.0, 0.5, 0.0))
            }
            Shot::Souls => {
                let u = smooth(0.0, 8.0, t);
                (lerp3(d + v(0.1, 1.3, 3.75), d + v(0.05, 1.25, 3.2), u), d + v(0.0, 1.0, -2.3), d + v(0.0, 0.9, -2.6))
            }
            Shot::Descent => {
                let u = smooth(stage::WALK, stage::WALK + 9.0, t);
                (lerp3(d + v(0.0, 1.3, 3.85), d + v(0.0, 1.25, 1.2), u), lerp3(d + v(0.0, 0.6, -2.2), d + v(0.0, 0.2, -5.0), u), d + v(0.0, 0.7, 0.4).lerp(d + v(0.0, 0.8, -3.2), u))
            }
        };
        (eye + sway, look + sway * 0.5, focus)
    }

    /// The part of the world the shadow map covers during the story.
    pub(super) fn story_shadow_box(&self, sv: StoryView) -> (Vec3, f32) {
        match sv.shot().place() {
            Place::Graveyard => (GRAVEYARD + Vec3::new(0.0, 0.6, -2.8), 10.0),
            Place::Gateway => (GATEWAY + Vec3::new(0.0, 1.0, 0.0), 6.5),
        }
    }
}

/// One leaf of the graveyard gate, from its hinge (at x = 0) out to the latch: bars with spear
/// points, rails, a scroll in the middle. (offset, size) pairs.
fn gate_leaf() -> Vec<(Vec3, Vec3)> {
    let mut out = Vec::new();
    out.push((Vec3::new(0.02, 0.98, 0.0), Vec3::new(0.05, 1.96, 0.05)));
    for i in 0..7 {
        let x = 0.15 + i as f32 * 0.14;
        let top = 1.78 + if i % 2 == 0 { 0.1 } else { 0.0 };
        out.push((Vec3::new(x, top / 2.0 + 0.04, 0.0), Vec3::new(0.03, top, 0.03)));
        out.push((Vec3::new(x, top + 0.07, 0.0), Vec3::new(0.06, 0.06, 0.03)));
    }
    for y in [0.2f32, 0.95, 1.62] {
        out.push((Vec3::new(0.52, y, 0.0), Vec3::new(1.04, 0.035, 0.04)));
    }
    for k in 0..10 {
        let a = k as f32 / 10.0 * TAU;
        out.push((Vec3::new(0.52 + a.cos() * 0.16, 1.28 + a.sin() * 0.16, 0.0), Vec3::new(0.03, 0.03, 0.03)));
    }
    out.push((Vec3::new(1.0, 0.95, 0.03), Vec3::new(0.05, 0.12, 0.04)));
    out
}

/// One leaf of the iron door, from its hinge (at x = 0) to the middle: plates, straps, rivets,
/// a ring to pull it by. (offset, size, colour, shade) tuples.
fn door_leaf() -> Vec<(Vec3, Vec3, u32, f32)> {
    let mut out = Vec::new();
    for row in 0..5 {
        let y = 0.25 + row as f32 * 0.475;
        out.push((Vec3::new(0.485, y, 0.0), Vec3::new(0.95, 0.46, 0.1), if row % 2 == 0 { 0x2c2e32 } else { 0x323438 }, SHADE_METAL));
    }
    for y in [0.3f32, 1.2, 2.1] {
        out.push((Vec3::new(0.485, y, 0.06), Vec3::new(0.97, 0.09, 0.03), 0x3e3a36, SHADE_METAL));
        for k in 0..6 {
            out.push((Vec3::new(0.08 + k as f32 * 0.165, y, 0.08), Vec3::splat(0.035), 0x7a7e84, SHADE_METAL));
        }
    }
    out.push((Vec3::new(0.82, 1.1, 0.1), Vec3::new(0.05, 0.05, 0.05), 0x5a5c62, SHADE_METAL));
    for k in 0..10 {
        let a = k as f32 / 10.0 * TAU;
        out.push((Vec3::new(0.82 + a.cos() * 0.09, 0.98 + a.sin() * 0.09, 0.1), Vec3::splat(0.028), 0x6a6c72, SHADE_METAL));
    }
    out
}

/// The crypt's light: torches on the pillars, the old man's lantern, cool fills on faces.
fn gateway_lights(f: &mut Frame, time: f32) {
    let cube = MeshKind::Cube as usize;
    let at = |p: Vec3| GATEWAY + p;
    // ---- the torches on the pillars
    for (n, &(x, z)) in [(-3.35f32, -1.3f32), (3.35, -1.3), (-3.35, 2.0), (3.35, 2.0)].iter().enumerate() {
        let inward = -x.signum();
        let base = at(Vec3::new(x + inward * 0.47, 2.0, z));
        let fl = 1.0 + (time * 9.0 + n as f32 * 1.7).sin() * 0.07 + (time * 23.0 + n as f32).sin() * 0.04;
        for k in 0..6 {
            let a = time * 4.0 + k as f32 * 1.1 + n as f32;
            let rise = (time * 3.0 + k as f32 * 0.37).fract();
            let p = base + Vec3::new(a.cos() * 0.035 * (1.0 - rise), rise * 0.22, a.sin() * 0.035 * (1.0 - rise));
            f.additive.push(Instance::new(Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.06 * (1.0 - rise * 0.6))), lin(if k % 2 == 0 { 0xffa040 } else { 0xffd070 }) * 1.5, 0.85 * (1.0 - rise), Vec3::ZERO, SHADE_UNLIT));
        }
        glow_orb(f, base + Vec3::new(0.0, 0.08, 0.0), 0.55, lin(0xff9a40), 0.3 * fl);
        f.lights.push(Light { pos: base + Vec3::new(inward * 0.15, 0.1, 0.0), color: lin(0xffb074) * 4.6 * fl, distance: 6.0 });
    }
    // The old man's lantern by his stool.
    let lantern = at(Vec3::new(2.4, 0.13, -2.25));
    f.opaque[cube].push(Instance::new(Mat4::from_scale_rotation_translation(Vec3::new(0.14, 0.2, 0.14), Quat::from_rotation_y(0.4), lantern), lin(0xffd49a), 1.0, lin(0xffa040) * 1.8, SHADE_LIT));
    f.opaque[cube].push(Instance::new(Mat4::from_scale_rotation_translation(Vec3::new(0.17, 0.03, 0.17), Quat::from_rotation_y(0.4), lantern + Vec3::new(0.0, 0.11, 0.0)), lin(0x222226), 1.0, Vec3::ZERO, SHADE_METAL));
    glow_orb(f, lantern, 0.45, lin(0xffa050), 0.25);
    f.lights.push(Light { pos: lantern + Vec3::new(0.0, 0.3, 0.2), color: lin(0xffb060) * 2.5, distance: 3.5 });
    // Cool fills on faces from the front and on the old man and the dog; a faint red seeping under the door.
    f.lights.push(Light { pos: at(Vec3::new(0.3, 2.4, 3.2)), color: lin(0x9ab0d8) * 2.8, distance: 7.0 });
    f.lights.push(Light { pos: at(Vec3::new(1.0, 2.0, 0.2)), color: lin(0xa8bce0) * 2.6, distance: 5.0 });
    f.lights.push(Light { pos: at(Vec3::new(-1.2, 1.5, -0.3)), color: lin(0xc8b8a8) * 2.2, distance: 3.6 });
}

/// The door to the stair: its two leaves swung `open` (radians) and the red glow beyond.
fn gateway_door(f: &mut Frame, open: f32, time: f32) {
    let cube = MeshKind::Cube as usize;
    let at = |p: Vec3| GATEWAY + p;
    for side in [-1.0f32, 1.0] {
        // Each leaf swings in, away from the room, about its hinge in the jamb.
        let hinge = at(Vec3::new(side * 0.97, 0.0, -3.47));
        let q = Quat::from_rotation_y(-side * open);
        for &(off, size, c, mat) in door_leaf().iter() {
            let local = Vec3::new(-side * off.x, off.y, off.z);
            f.opaque[cube].push(Instance::new(Mat4::from_scale_rotation_translation(size, q, hinge + q * local), lin(c), 1.0, Vec3::ZERO, mat));
        }
    }
    let red = 0.35 + open * 3.0;
    glowing(&mut f.opaque[cube], at(Vec3::new(0.0, 0.015, -3.36)), Vec3::new(1.85, 0.02, 0.04), Quat::IDENTITY, lin(0x300800), lin(0xff3a10) * (0.8 + 0.3 * (time * 2.0).sin()) * (1.0 - open / 1.45));
    f.lights.push(Light { pos: at(Vec3::new(0.0, 0.3, -3.3 - open * 0.4)), color: lin(0xff4a1a) * red, distance: 3.0 + open * 3.5 });
    if open > 0.05 {
        f.lights.push(Light { pos: at(Vec3::new(0.0, -1.5, -6.5)), color: lin(0xff3a0a) * 8.0 * open, distance: 7.0 });
        glow_orb(f, at(Vec3::new(0.0, 0.9, -4.6)), 1.6, lin(0xff4a18), 0.2 * open);
        glow_orb(f, at(Vec3::new(0.0, -0.9, -6.6)), 2.2, lin(0xff3010), 0.35 * open);
        for n in 0..20 {
            let fk = n as f32;
            let u = (time * 0.35 + fk * 0.13).fract();
            let p = at(Vec3::new((fk * 1.7).sin() * 0.7, -1.4 + u * 3.5, -6.0 + u * 3.0 + (fk * 2.3).cos() * 0.3));
            f.additive.push(Instance::new(Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.025)), lin(0xff8a30) * 1.6, (1.0 - u) * open * 0.8, Vec3::ZERO, SHADE_UNLIT));
        }
    }
}

/// Where the hero stands at the old man's, and the old man.
const WARES_HERO: Vec3 = Vec3::new(0.9, 0.0, 0.3);
const WARES_OLD_MAN: Vec3 = Vec3::new(-1.2, 0.0, -1.1);
const WARES_DOG: Vec3 = Vec3::new(-0.4, 0.0, -0.45);

impl WorldView {
    /// The old man's wares: the camera looks past them to the right, where the panel goes.
    pub(super) fn wares_camera(&self, t: f32) -> (Vec3, Vec3) {
        let a = (t * 0.2).sin() * 0.12;
        (GATEWAY + Vec3::new(2.2 + a, 1.85, 3.55), GATEWAY + Vec3::new(1.6 + a * 0.5, 0.8, -1.2))
    }

    /**
     * The shop's backdrop: the crypt by the gate to the underworld, the door
     * shut on its red glow, the old gatekeeper leaning on his staff and talking
     * now and then, Quitmer lying on his straw wagging, and the hero before
     * them with the weapon and shield he has chosen.
     */
    pub(super) fn emit_wares(&self, s: &SceneState, f: &mut Frame) {
        let time = s.time as f32;
        let cube = MeshKind::Cube as usize;
        let at = |p: Vec3| GATEWAY + p;
        f.atmosphere = crypt();
        f.opaque[cube].extend_from_slice(&self.story_sets[1]);
        f.transparent[cube].extend_from_slice(&self.story_mist[1]);
        f.bounds = Vec4::new(GATEWAY.x - 4.5, GATEWAY.z - 3.5, GATEWAY.x + 4.5, GATEWAY.z + 4.5);
        gateway_lights(f, time);
        gateway_door(f, 0.0, time);

        // The old man, turned to the hero, a gesture every few seconds.
        let talk = time % 7.0;
        self.emit_old_man(f, at(WARES_OLD_MAN), yaw_to(WARES_OLD_MAN, WARES_HERO), 0.0, 0.0, 0.0, Some(talk), time);
        // Quitmer, off his chain, lying at ease by the old man.
        let yaw = yaw_to(WARES_DOG, WARES_HERO) - 0.3;
        self.emit_dog(f, at(WARES_DOG), yaw, 0.0, 0.0, 1.0, 0.0, 0.0, 0.7, time);
        // The hero, breathing, looking the old man's wares over.
        let rig = &self.models.hero;
        let mut pose = Pose::rest(rig);
        let breathe = (time * 1.8).sin();
        if let Some(i) = rig.index("head") {
            pose.rot[i].x = 0.08 + breathe * 0.02;
        }
        let root = Mat4::from_translation(at(WARES_HERO)) * Mat4::from_rotation_y(yaw_to(WARES_HERO, WARES_OLD_MAN)) * Mat4::from_scale(Vec3::new(1.0, 1.0 + breathe * 0.006, 1.0));
        emit_rig(&mut f.opaque[cube], rig, &pose, root, Vec3::ONE, 1.0, Vec3::ZERO);
        f.lights.push(Light { pos: at(WARES_HERO + Vec3::new(0.6, 1.8, 1.4)), color: lin(0xffe0c0) * 3.0, distance: 4.0 });
    }
}
