//! The seven seal-stones (BACKLOG B44): each prince drops one as he falls, each its own stone in
//! its sin's colour (PRINCES-EPIC.md §9.6's table): Lucifer's a star-cut amethyst, Leviathan's an
//! emerald like a serpent's eye, Amon's a ruby glowing like a coal, Belphegor's a moonstone,
//! Mammon's a citrine cut like a coin, Beelzebub's an amber with a fly in it, Asmodeus's a rose-cut
//! pink stone. Drawn live from cubes wherever one is: lying in the court, set in the House of
//! Seven Seals, at Hızır's.

use super::*;
use crate::logic::sins::Sin;
use std::f32::consts::{FRAC_PI_4, TAU};

/// A cube of the stone: at `off` from its middle (scaled by `size`), `scale` big, turned by `rot`.
fn facet(f: &mut Frame, at: Vec3, rot: Quat, off: Vec3, scale: Vec3, size: f32, color: Vec3, glow: f32) {
    let m = Mat4::from_scale_rotation_translation(scale * size, rot, at + rot * (off * size));
    f.opaque[MeshKind::Cube as usize].push(Instance::new(m, color, 1.0, color * glow, SHADE_LIT));
}

/// Sin `sin`'s seal-stone at `at`, about `size` tall, turning with `time` (`spin` 0 holds it still,
/// as when set in a socket), lit from within by `glow`.
pub(super) fn emit_seal_stone(f: &mut Frame, at: Vec3, sin: Sin, time: f32, size: f32, spin: f32, glow: f32) {
    let c = lin(sin.color());
    let rot = Quat::from_rotation_y(time * spin);
    let diamond = Quat::from_rotation_z(FRAC_PI_4) * Quat::from_rotation_x(FRAC_PI_4);
    match sin {
        // A star-cut amethyst: a point-up octahedron with four rays.
        Sin::Pride => {
            facet(f, at, rot * diamond, Vec3::ZERO, Vec3::splat(0.55), size, c, glow);
            for k in 0..4 {
                let a = k as f32 * TAU / 4.0;
                let r = Quat::from_rotation_y(a);
                facet(f, at, rot * r, Vec3::new(0.42, 0.0, 0.0), Vec3::new(0.34, 0.12, 0.12), size, c * 1.15, glow);
            }
        }
        // An emerald like a serpent's eye: a tall stone with a dark slit.
        Sin::Envy => {
            facet(f, at, rot * Quat::from_rotation_y(FRAC_PI_4), Vec3::ZERO, Vec3::new(0.42, 0.8, 0.42), size, c, glow);
            facet(f, at, rot, Vec3::new(0.0, 0.0, 0.22), Vec3::new(0.06, 0.58, 0.05), size, lin(0x0a140a), 0.0);
        }
        // A ruby glowing like a coal: a chunky cluster, pulsing.
        Sin::Wrath => {
            let pulse = 1.0 + 0.4 * (time * 3.0).sin();
            for (k, off) in
                [Vec3::ZERO, Vec3::new(0.18, 0.14, 0.05), Vec3::new(-0.15, 0.1, -0.08), Vec3::new(0.02, -0.16, 0.14)].iter().enumerate()
            {
                let r = Quat::from_rotation_y(k as f32 * 1.3) * diamond;
                facet(f, at, rot * r, *off, Vec3::splat(0.42 - k as f32 * 0.06), size, c, glow * pulse);
            }
        }
        // A moonstone: a smooth flattened round, its sheen pale.
        Sin::Sloth => {
            for k in 0..4 {
                let r = Quat::from_rotation_y(k as f32 * FRAC_PI_4 / 2.0);
                facet(f, at, rot * r, Vec3::ZERO, Vec3::new(0.62, 0.36, 0.62), size, c, glow);
            }
            facet(f, at, rot, Vec3::new(0.0, 0.16, 0.0), Vec3::new(0.38, 0.1, 0.38), size, lin(0xe8f0ff), glow * 0.8);
        }
        // A citrine cut like a coin: a thick disc with a rim.
        Sin::Greed => {
            let face = Quat::from_rotation_x(TAU / 4.0);
            for k in 0..4 {
                let r = face * Quat::from_rotation_y(k as f32 * FRAC_PI_4 / 2.0);
                facet(f, at, rot * r, Vec3::ZERO, Vec3::new(0.7, 0.18, 0.7), size, c, glow);
            }
            facet(f, at, rot * face, Vec3::new(0.0, 0.11, 0.0), Vec3::new(0.34, 0.04, 0.34), size, lin(0xfff0a0), glow);
        }
        // An amber with a tiny fly in it.
        Sin::Gluttony => {
            for k in 0..3 {
                let r = Quat::from_rotation_y(k as f32 * 0.5) * Quat::from_rotation_x(k as f32 * 0.4);
                facet(f, at, rot * r, Vec3::ZERO, Vec3::new(0.56, 0.5, 0.52), size, c, glow * 0.8);
            }
            facet(f, at, rot, Vec3::new(0.04, 0.02, 0.3), Vec3::new(0.1, 0.06, 0.04), size, lin(0x1a120a), 0.0);
            facet(f, at, rot, Vec3::new(0.04, 0.07, 0.3), Vec3::new(0.16, 0.03, 0.03), size, lin(0x3a3028), 0.0);
        }
        // A rose-cut pink stone: a low dome of petals.
        Sin::Lust => {
            for k in 0..6 {
                let r = Quat::from_rotation_y(k as f32 * TAU / 6.0);
                facet(f, at, rot * r * Quat::from_rotation_z(0.5), Vec3::new(0.22, 0.0, 0.0), Vec3::new(0.34, 0.2, 0.26), size, c, glow);
            }
            facet(f, at, rot * diamond, Vec3::new(0.0, 0.14, 0.0), Vec3::splat(0.26), size, lin(0xffc0dc), glow);
        }
    }
    glow_orb(f, at, size * 0.9, c, 0.22 * glow.max(0.2));
}
