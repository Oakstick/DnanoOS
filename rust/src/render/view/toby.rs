//! Toby's things (B08, `logic::tale`): each a small model of his, lying where the realm left it,
//! drawn in play (bobbing a little in a warm glow until the hero takes it), and at Hızır's in the
//! scene after each prince, and at the end.

use super::*;
use crate::logic::tale::{Token, TOBY_COLOR};

/// Shade modes for the things' plain blocks (cloth, wood, metal: 5 + the material).
const SHADE_METAL: f32 = 6.0;
const SHADE_CLOTH: f32 = 8.0;
const SHADE_GOLD: f32 = 13.0;

#[allow(clippy::too_many_arguments)]
/// One block of a thing, in its own space (metres, its foot at the origin), placed by `root`.
fn put(f: &mut Frame, mesh: MeshKind, root: Mat4, at: Vec3, size: Vec3, rot: Quat, color: u32, shade: f32, glow: f32) {
    let m = root * Mat4::from_scale_rotation_translation(size, rot, at);
    f.opaque[mesh as usize].push(Instance::new(m, lin(color), 1.0, lin(color) * glow, shade));
}

/// Draws `token` at `root` (its foot on the ground, facing +z), `glow` lifting its colours a little
/// so it reads in the dark.
pub(super) fn token_model(f: &mut Frame, token: Token, root: Mat4, glow: f32) {
    use MeshKind::{Cube, Cylinder, Sphere};
    let q = Quat::IDENTITY;
    let v = Vec3::new;
    match token {
        Token::Ball => {
            put(f, Sphere, root, v(0.0, 0.11, 0.0), Vec3::splat(0.11), q, 0x9a5a2e, SHADE_LIT, glow);
            put(f, Sphere, root, v(0.0, 0.11, 0.0), v(0.112, 0.034, 0.112), Quat::from_rotation_z(0.4), 0x4a2a16, SHADE_LIT, glow);
        }
        Token::Drawing => {
            // A sheet of paper lying a little curled, four of them holding hands on it, a sun.
            let tilt = Quat::from_rotation_x(-1.45);
            let sheet = root * Mat4::from_translation(v(0.0, 0.02, 0.0)) * Mat4::from_quat(tilt);
            put(f, Cube, sheet, v(0.0, 0.0, 0.0), v(0.3, 0.22, 0.006), q, 0xf2ead8, SHADE_CLOTH, glow);
            for (k, c) in [0x34588e, 0x5e7a34, 0x923a2c, 0x5a3e6e].into_iter().enumerate() {
                let x = -0.09 + k as f32 * 0.06;
                let tall = if k == 0 { 0.07 } else { 0.085 + k as f32 * 0.004 };
                put(f, Cube, sheet, v(x, -0.04 + tall * 0.5, 0.005), v(0.012, tall, 0.004), q, c, SHADE_CLOTH, glow);
                put(f, Sphere, sheet, v(x, -0.04 + tall + 0.012, 0.005), v(0.014, 0.014, 0.004), q, 0xe8b890, SHADE_CLOTH, glow);
            }
            put(f, Cube, sheet, v(0.0, 0.0, 0.006), v(0.2, 0.006, 0.004), q, 0x2a2a2a, SHADE_CLOTH, glow);
            put(f, Sphere, sheet, v(0.11, 0.07, 0.006), v(0.022, 0.022, 0.004), q, 0xf2c53d, SHADE_CLOTH, glow + 0.3);
            // TOBY in the corner: a red scrawl.
            put(f, Cube, sheet, v(-0.09, 0.08, 0.006), v(0.07, 0.008, 0.004), q, 0xc8322a, SHADE_CLOTH, glow);
        }
        Token::Soldier => {
            put(f, Cylinder, root, v(0.0, 0.006, 0.0), v(0.05, 0.012, 0.05), q, 0x3a6a3a, SHADE_METAL, glow);
            for x in [-0.012, 0.012] {
                put(f, Cube, root, v(x, 0.04, 0.0), v(0.016, 0.055, 0.02), q, 0x1e2a4a, SHADE_METAL, glow);
            }
            put(f, Cube, root, v(0.0, 0.095, 0.0), v(0.045, 0.055, 0.03), q, 0xc8322a, SHADE_METAL, glow);
            put(f, Cube, root, v(0.0, 0.095, 0.016), v(0.006, 0.05, 0.004), q, 0xf2c53d, SHADE_GOLD, glow);
            put(f, Sphere, root, v(0.0, 0.135, 0.0), Vec3::splat(0.017), q, 0xe8b890, SHADE_METAL, glow);
            // The tall hat, a corner chipped off.
            put(f, Cube, root, v(0.0, 0.17, 0.0), v(0.032, 0.045, 0.032), q, 0x141418, SHADE_METAL, glow);
            put(f, Cube, root, v(0.012, 0.19, 0.012), v(0.012, 0.012, 0.012), q, 0x8a8a90, SHADE_METAL, glow);
            // His musket on his shoulder.
            put(f, Cube, root, v(0.03, 0.12, 0.0), v(0.008, 0.11, 0.008), Quat::from_rotation_z(0.15), 0x5a3a22, SHADE_METAL, glow);
        }
        Token::Mitten => {
            let lie = root * Mat4::from_translation(v(0.0, 0.025, 0.0)) * Mat4::from_rotation_x(-PI / 2.0);
            put(f, Cube, lie, v(0.0, 0.03, 0.0), v(0.1, 0.12, 0.045), q, 0xc8322a, SHADE_CLOTH, glow);
            put(f, Sphere, lie, v(0.0, 0.09, 0.0), v(0.05, 0.04, 0.023), q, 0xc8322a, SHADE_CLOTH, glow);
            put(f, Cube, lie, v(0.065, 0.04, 0.0), v(0.035, 0.06, 0.04), Quat::from_rotation_z(-0.4), 0xc8322a, SHADE_CLOTH, glow);
            put(f, Cube, lie, v(0.0, -0.04, 0.0), v(0.105, 0.035, 0.05), q, 0xf0e6d0, SHADE_CLOTH, glow);
            // Singed at the tip.
            put(f, Sphere, lie, v(0.0, 0.11, 0.0), v(0.04, 0.02, 0.025), q, 0x2a1810, SHADE_CLOTH, 0.0);
        }
        Token::Chalk => {
            put(
                f,
                Cylinder,
                root,
                v(0.0, 0.012, 0.0),
                v(0.012, 0.035, 0.012),
                Quat::from_rotation_z(PI / 2.0),
                0xf4f2ea,
                SHADE_LIT,
                glow + 0.2,
            );
            // TOBY WAS HERE, and an arrow pointing down: chalk on the floor beside it.
            let mark = |f: &mut Frame, a: Vec3, b: Vec3| {
                let mid = (a + b) * 0.5;
                let len = a.distance(b);
                let yaw = (b.x - a.x).atan2(b.z - a.z);
                put(f, Cube, root, mid + v(0.0, 0.003, 0.0), v(0.012, 0.002, len), Quat::from_rotation_y(yaw), 0xe8e6de, SHADE_LIT, 0.5);
            };
            // A T, an O, a B, a Y, and the arrow under them.
            let x0 = -0.32;
            mark(f, v(x0, 0.0, -0.1), v(x0 + 0.08, 0.0, -0.1));
            mark(f, v(x0 + 0.04, 0.0, -0.1), v(x0 + 0.04, 0.0, 0.0));
            for k in 0..8 {
                let a0 = k as f32 / 8.0 * TAU;
                let a1 = (k + 1) as f32 / 8.0 * TAU;
                let c = v(x0 + 0.14, 0.0, -0.05);
                mark(f, c + v(a0.cos() * 0.035, 0.0, a0.sin() * 0.05), c + v(a1.cos() * 0.035, 0.0, a1.sin() * 0.05));
            }
            mark(f, v(x0 + 0.2, 0.0, -0.1), v(x0 + 0.2, 0.0, 0.0));
            mark(f, v(x0 + 0.2, 0.0, -0.1), v(x0 + 0.25, 0.0, -0.075));
            mark(f, v(x0 + 0.25, 0.0, -0.075), v(x0 + 0.2, 0.0, -0.05));
            mark(f, v(x0 + 0.2, 0.0, -0.05), v(x0 + 0.25, 0.0, -0.025));
            mark(f, v(x0 + 0.25, 0.0, -0.025), v(x0 + 0.2, 0.0, 0.0));
            mark(f, v(x0 + 0.29, 0.0, -0.1), v(x0 + 0.315, 0.0, -0.05));
            mark(f, v(x0 + 0.34, 0.0, -0.1), v(x0 + 0.315, 0.0, -0.05));
            mark(f, v(x0 + 0.315, 0.0, -0.05), v(x0 + 0.315, 0.0, 0.0));
            mark(f, v(x0 + 0.16, 0.0, 0.05), v(x0 + 0.16, 0.0, 0.2));
            mark(f, v(x0 + 0.16, 0.0, 0.2), v(x0 + 0.11, 0.0, 0.15));
            mark(f, v(x0 + 0.16, 0.0, 0.2), v(x0 + 0.21, 0.0, 0.15));
        }
        Token::Bell => {
            let hang = root * Mat4::from_translation(v(0.0, 0.0, 0.0)) * Mat4::from_rotation_z(0.5);
            put(f, Cylinder, hang, v(0.0, 0.06, 0.0), v(0.055, 0.05, 0.055), q, 0xd8a840, SHADE_GOLD, glow);
            put(f, Sphere, hang, v(0.0, 0.11, 0.0), v(0.045, 0.035, 0.045), q, 0xd8a840, SHADE_GOLD, glow);
            put(f, Cylinder, hang, v(0.0, 0.013, 0.0), v(0.065, 0.01, 0.065), q, 0xb8882a, SHADE_GOLD, glow);
            put(f, Sphere, hang, v(0.0, 0.02, 0.0), Vec3::splat(0.016), q, 0x4a3a20, SHADE_METAL, glow);
            // The strap he tore it off by.
            put(f, Cube, hang, v(0.0, 0.16, 0.0), v(0.025, 0.06, 0.008), Quat::from_rotation_x(0.3), 0x5a2a18, SHADE_CLOTH, glow);
        }
        Token::Scarf => {
            // The end of it, lying in an S, red with its cream stripes, its fringe at the tip.
            for k in 0..9 {
                let u = k as f32 / 8.0;
                let p = v(-0.22 + u * 0.44, 0.012, (u * PI * 1.5).sin() * 0.07);
                let next = v(-0.22 + (u + 0.125) * 0.44, 0.012, ((u + 0.125) * PI * 1.5).sin() * 0.07);
                let yaw = (next.z - p.z).atan2(next.x - p.x);
                let c = if k % 3 == 1 { 0xf0e6d0 } else { 0xc8322a };
                put(f, Cube, root, p, v(0.065, 0.02, 0.1), Quat::from_rotation_y(-yaw), c, SHADE_CLOTH, glow);
            }
            for k in 0..5 {
                let z = -0.04 + k as f32 * 0.02;
                put(f, Cube, root, v(0.27, 0.006, z), v(0.04, 0.006, 0.006), q, 0xc8322a, SHADE_CLOTH, glow);
            }
        }
    }
}

impl WorldView {
    /// The realm's token where it lies, until he takes it: bobbing a little and turning, in a warm
    /// glow of Toby's red, motes rising round it, so it reads from the top camera too.
    pub(super) fn emit_token(&self, s: &SceneState, f: &mut Frame) {
        let Some((token, at)) = self.token else { return };
        let time = s.time as f32;
        let ground = floor_at(s.grid, at.x as f32, at.y as f32);
        let base = Vec3::new(at.x as f32, ground, at.y as f32);
        let lift = if matches!(token, Token::Chalk | Token::Scarf | Token::Drawing) { 0.0 } else { 0.08 + (time * 2.2).sin() * 0.04 };
        let root = Mat4::from_translation(base + Vec3::Y * lift) * Mat4::from_rotation_y(time * 0.8) * Mat4::from_scale(Vec3::splat(1.6));
        token_model(f, token, root, 0.18);
        let warm = lin(TOBY_COLOR);
        glow_orb(f, base + Vec3::Y * 0.25, 0.9, warm, 0.16 + 0.05 * (time * 3.0).sin());
        let ring = Mat4::from_translation(base + Vec3::Y * 0.015)
            * Mat4::from_rotation_x(-PI / 2.0)
            * Mat4::from_scale(Vec3::new(0.55, 0.55, 1.0));
        f.glow.push(Instance::new(ring, warm, 0.22, Vec3::ZERO, SHADE_GLOW));
        for k in 0..8 {
            let fk = k as f32;
            let u = (time * 0.5 + fk / 8.0).fract();
            let a = fk * 2.4 + time;
            glow_orb(f, base + Vec3::new(a.cos() * 0.3, 0.1 + u * 1.0, a.sin() * 0.3), 0.03, lin(0xfff0d8), 0.7 * (1.0 - u));
        }
        f.lights.push(Light { pos: base + Vec3::new(0.0, 0.8, 0.0), color: warm * 2.2, distance: 3.0 });
    }
}

/// The boy inside the warrior's soul-glow (B07): his outline in spirit light, in his soul's colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoyGlow {
    /// The boy, as the cast's kid he is (`tale::boy_of`).
    pub kid: usize,
    pub tint: u32,
    /// The game's time it began.
    pub since: f64,
    /// When he falls (the boy stays, standing where the warrior fell, until the round begins again);
    /// else at a shrine, as the blessing comes (a breath of him, and gone).
    pub fall: bool,
}

/// Seconds the boy shows at a shrine, and after a fall.
pub const SHRINE_GLOW: f32 = 2.4;
pub const FALL_GLOW: f32 = 3.2;

impl WorldView {
    /// A boy's outline in spirit light at `feet`, facing `yaw`, `alpha` strong, with the soul's
    /// glow about him: the character screen's, the shrines', the fall's.
    pub(super) fn emit_boy_ghost(&self, f: &mut Frame, kid: usize, feet: Vec3, yaw: f32, tint: Vec3, alpha: f32, scale: f32) {
        if alpha <= 0.01 {
            return;
        }
        let rig = &self.models.kids[kid.min(3)];
        let root = Mat4::from_translation(feet) * Mat4::from_rotation_y(yaw) * Mat4::from_scale(Vec3::splat(scale));
        f.rigs_blend.push(rig, &Pose::rest(rig), root, tint, alpha, Vec3::ZERO, rig_look::GHOST, 0.0);
        glow_orb(f, feet + Vec3::Y * 0.45 * scale, 0.9 * scale, tint, 0.2 * alpha.min(1.0) * 2.0);
    }

    /// The boy where the warrior fell, or at the shrine he stands on (`boy_glow`).
    pub(super) fn emit_boy_glow(&self, s: &SceneState, f: &mut Frame, hero_pos: Vec3) {
        let Some(g) = self.boy_glow else { return };
        let age = (s.time - g.since) as f32;
        let (life, rise) = if g.fall { (FALL_GLOW, 0.9) } else { (SHRINE_GLOW, 0.5) };
        if age < 0.0 || age > life || (g.fall && s.hero.alive) {
            return;
        }
        let up = (age / rise).clamp(0.0, 1.0);
        let fade = 1.0 - ((age - (life - 0.8)) / 0.8).clamp(0.0, 1.0);
        let alpha = up * fade * if g.fall { 0.5 } else { 0.36 };
        // A little toward the camera, so the warrior's body does not hide him, and facing it.
        let d = self.cam_pos - hero_pos;
        let flat = Vec3::new(d.x, 0.0, d.z).normalize_or(Vec3::Z);
        let feet = hero_pos + flat * if g.fall { 0.0 } else { 0.35 } + Vec3::Y * (-0.35 * (1.0 - up) + if g.fall { 0.0 } else { 0.1 * up });
        let tint = lin(g.tint);
        // (Larger than life, as the warrior is: from the camera over the field he still reads as a boy.)
        self.emit_boy_ghost(f, g.kid, feet, flat.x.atan2(flat.z), tint, alpha, if g.fall { 1.7 } else { 1.4 });
        f.lights.push(Light { pos: feet + Vec3::Y * 1.0, color: tint * 2.5 * alpha, distance: 3.0 });
    }
}
