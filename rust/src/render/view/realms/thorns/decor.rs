//! The Garden of Thorns' props, built into the static scene from boxes: rose hedges with their
//! lovely faces toward the camera and black bramble behind, rose arches with their lanterns, fruit
//! trees, the bridal bowers and the dead grooms' bones, the censers, the marble fountains the koi
//! swim in, the Bride's pavilion, the elf-maids' fairy ring and the Black Thorn.

use super::super::super::*;

pub(super) mod pal {
    pub const LEAF_DEEP: u32 = 0x10241a;
    pub const LEAF_D: u32 = 0x1a3a24;
    pub const LEAF: u32 = 0x24502e;
    pub const LEAF_L: u32 = 0x31663a;
    /// Leaf tips silvered by the moon.
    pub const LEAF_MOON: u32 = 0x4e7c62;
    pub const CRIMSON: u32 = 0xa80e2c;
    pub const RED: u32 = 0xd81c46;
    pub const PINK: u32 = 0xff4a9a;
    pub const BLUSH: u32 = 0xffaacc;
    pub const WHITE: u32 = 0xf6eaf0;
    pub const BRAMBLE: u32 = 0x22141a;
    pub const BRAMBLE_L: u32 = 0x2c1820;
    pub const THORN: u32 = 0x0a0507;
    pub const DEAD_LEAF: u32 = 0x3a2a1c;
    pub const FLAG: u32 = 0xdcbcc8;
    pub const FLAG_D: u32 = 0xc6a2b2;
    pub const GROUT: u32 = 0x6a4a58;
    pub const MARBLE: u32 = 0xf2e0e8;
    pub const MARBLE_D: u32 = 0xd6bcc8;
    pub const MARBLE_W: u32 = 0xfaf4f6;
    pub const GOLD: u32 = 0xe0b45a;
    pub const GOLD_D: u32 = 0xa88038;
    pub const EARTH: u32 = 0x463028;
    pub const EARTH_D: u32 = 0x36241e;
    pub const MOSS: u32 = 0x22321e;
    pub const CARPET: u32 = 0x6a0e24;
    pub const SILK: u32 = 0xfaf2f6;
    pub const SILK_P: u32 = 0xf6c6dc;
    pub const BONE: u32 = 0xe8dcc4;
    pub const BONE_D: u32 = 0xb8a88c;
    pub const RUST: u32 = 0x6a4a3a;
    pub const STEEL: u32 = 0x9aa0aa;
    pub const BRONZE: u32 = 0x9a6a34;
    pub const BRONZE_D: u32 = 0x5a3c1c;
    pub const WATER: u32 = 0x10202e;
    pub const LILY: u32 = 0x2e6a3a;
    pub const BARK: u32 = 0x3a2618;
    pub const PEACH_BLOSSOM: u32 = 0xffb8d4;
    pub const PEACH: u32 = 0xffa070;
    pub const POMEGRANATE: u32 = 0xb81830;
    pub const DARK_LEAF: u32 = 0x1e4a26;
    pub const ROSE_GLASS: u32 = 0xff6aa8;
    pub const MUSHROOM: u32 = 0xf0e8f4;
}
use pal::*;

/// The hedges stand this tall.
pub(super) const HEDGE_H: f32 = 1.7;

/// A hash of a tile and a salt, 0 to 1.
pub(super) fn h(x: i32, y: i32, k: u32) -> f32 {
    hash(x as f64 * 1.37 + k as f64 * 17.1, y as f64 * 2.11 + k as f64 * 5.3) as f32
}

/// A box: centre, size, turn, colour, shade.
pub(super) fn bx(out: &mut Vec<Instance>, c: Vec3, s: Vec3, rot: Quat, color: u32, shade: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(s, rot, c), lin(color), 1.0, Vec3::ZERO, shade));
}

/// A box glowing with its own colour (`glow` of it).
pub(super) fn bxg(out: &mut Vec<Instance>, c: Vec3, s: Vec3, rot: Quat, color: u32, glow: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(s, rot, c), lin(color), 1.0, lin(color) * glow, SHADE_LIT));
}

/// A beam from `a` to `b`, `t` thick.
pub(super) fn beam(out: &mut Vec<Instance>, a: Vec3, b: Vec3, t: f32, color: u32) {
    let d = b - a;
    let len = d.length().max(1e-4);
    out.push(Instance::new(
        Mat4::from_scale_rotation_translation(Vec3::new(t, len, t), Quat::from_rotation_arc(Vec3::Y, d / len), (a + b) * 0.5),
        lin(color),
        1.0,
        Vec3::ZERO,
        SHADE_LIT,
    ));
}

/// A rose: its outer petals and its heart, opening along `n`.
pub(super) fn rose(out: &mut Vec<Instance>, at: Vec3, n: Vec3, size: f32, color: u32, seed: f32) {
    let rot = Quat::from_rotation_arc(Vec3::Y, n.normalize_or(Vec3::Y)) * Quat::from_rotation_y(seed * 6.0);
    let heart = match color {
        WHITE => BLUSH,
        BLUSH => PINK,
        _ => 0xff6a8a,
    };
    out.push(Instance::new(
        Mat4::from_scale_rotation_translation(Vec3::new(size, size * 0.7, size), rot, at),
        lin(color),
        1.0,
        lin(color) * 0.12,
        SHADE_LIT,
    ));
    out.push(Instance::new(
        Mat4::from_scale_rotation_translation(
            Vec3::new(size * 0.55, size * 0.5, size * 0.55),
            rot * Quat::from_rotation_y(0.78),
            at + n * size * 0.3,
        ),
        lin(heart),
        1.0,
        lin(heart) * 0.15,
        SHADE_LIT,
    ));
}

/// What a hedge's side shows: nothing (against more hedge), its lovely rose face, or bramble.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Face {
    None,
    Lovely,
    Bramble,
}

/// The rose colours of a stretch of hedge: mostly crimson, some pink, some white.
fn roses_of(x: i32, y: i32) -> [u32; 3] {
    match ((h(x / 3, y / 3, 9) * 5.0) as i32, (h(x, y, 2) * 3.0) as i32) {
        (0, _) => [PINK, BLUSH, WHITE],
        (1, _) => [WHITE, BLUSH, WHITE],
        (_, 0) => [RED, CRIMSON, PINK],
        _ => [CRIMSON, RED, CRIMSON],
    }
}

/// A block of rose hedge on tile `p`: a dark body, a crown of leaf clumps with roses on top, and
/// each side by its face: thick with leaves and roses where it faces a lovely path (and the camera),
/// black bramble where it faces a thorn path or turns away from the camera.
pub(super) fn hedge(out: &mut Vec<Instance>, p: P, faces: [(Dir, Face); 4]) {
    let (x, y) = (p.x, p.y);
    let c = Vec3::new(x as f32, 0.0, y as f32);
    let top = HEDGE_H;
    let roses = roses_of(x, y);
    // Hedge that faces nothing lovely is bramble through and through, its top too.
    let lovely = faces.iter().any(|f| f.1 == Face::Lovely);
    bx(
        out,
        c + Vec3::new(0.0, (top - 0.12) / 2.0, 0.0),
        Vec3::new(0.98, top - 0.12, 0.98),
        Quat::IDENTITY,
        if lovely { LEAF_DEEP } else { BRAMBLE },
        SHADE_LIT,
    );
    if !lovely {
        bramble_top(out, p);
        for k in 0..6u32 {
            let j = h(x, y, 10 + k);
            let at = c + Vec3::new((j - 0.5) * 0.7, top - 0.04, (h(x, y, 70 + k) - 0.5) * 0.7);
            bx(
                out,
                at,
                Vec3::new(0.24, 0.1, 0.22),
                Quat::from_rotation_y(j * 4.0),
                [0x1c2618, DEAD_LEAF, 0x262a1a][k as usize % 3],
                SHADE_LIT,
            );
        }
    } else {
        lush_crown(out, p, roses);
    }
    for (d, face) in faces {
        hedge_face(out, p, d, face, roses);
    }
}

/// A lovely hedge's crown: a four-by-four of small leaf clumps, moonlit at their tips, sprigs
/// standing out of it, roses on top.
fn lush_crown(out: &mut Vec<Instance>, p: P, roses: [u32; 3]) {
    let (x, y) = (p.x, p.y);
    let c = Vec3::new(x as f32, 0.0, y as f32);
    let top = HEDGE_H;
    for k in 0..16u32 {
        let (gx, gz) = ((k % 4) as f32 - 1.5, (k / 4) as f32 - 1.5);
        let j = h(x, y, 10 + k);
        let size = Vec3::new(0.27 + j * 0.09, 0.14 + j * 0.12, 0.27 + h(x, y, 20 + k) * 0.09);
        let col = [LEAF, LEAF_L, LEAF_D, LEAF_MOON, LEAF, LEAF_L][((j * 13.0) as usize + k as usize) % 6];
        let at = c + Vec3::new(
            gx * 0.235 + (j - 0.5) * 0.06,
            top - 0.08 + size.y * 0.4 + (h(x, y, 60 + k) - 0.5) * 0.08,
            gz * 0.235 + (h(x, y, 70 + k) - 0.5) * 0.06,
        );
        bx(out, at, size, Quat::from_rotation_y((j - 0.5) * 1.0), col, SHADE_LIT);
    }
    for k in 0..3u32 {
        let j = h(x, y, 80 + k);
        let at = c + Vec3::new((j - 0.5) * 0.8, top + 0.16, (h(x, y, 90 + k) - 0.5) * 0.8);
        bx(
            out,
            at,
            Vec3::new(0.1, 0.14, 0.1),
            Quat::from_rotation_y(j * 6.0) * Quat::from_rotation_z((j - 0.5) * 0.8),
            if k == 0 { LEAF_MOON } else { LEAF_L },
            SHADE_LIT,
        );
    }
    let crown_roses = (h(x, y, 30) * 4.0) as usize;
    for k in 0..crown_roses {
        let (a, r) = (h(x, y, 31 + k as u32) * TAU, 0.12 + h(x, y, 41 + k as u32) * 0.3);
        rose(out, c + Vec3::new(a.cos() * r, top + 0.14, a.sin() * r), Vec3::Y, 0.12, roses[k % 3], a);
    }
}

/// One side of a hedge block, by what it faces.
fn hedge_face(out: &mut Vec<Instance>, p: P, d: Dir, face: Face, roses: [u32; 3]) {
    let (x, y) = (p.x, p.y);
    let c = Vec3::new(x as f32, 0.0, y as f32);
    let top = HEDGE_H;
    let (dx, dz) = d.vec();
    let n = Vec3::new(dx as f32, 0.0, dz as f32);
    let side = Vec3::new(dz as f32, 0.0, dx as f32).abs();
    let salt = 50 + d as u32 * 13;
    // Facing away from the camera, the loveliest side is bramble: it is never seen.
    let face = if face == Face::Lovely && d == Dir::Up { Face::Bramble } else { face };
    match face {
        Face::None => {}
        Face::Lovely => {
            let rich = d == Dir::Down;
            // Rows of clumps up the face: four across and four up facing the camera, fewer on the flanks.
            let (cols, rows) = if rich { (4, 4) } else { (3, 3) };
            for k in 0..cols * rows {
                let (ci, ri) = (k % cols, k / cols);
                let j = h(x, y, salt + k);
                let along = (ci as f32 + 0.5) / cols as f32 - 0.5 + (j - 0.5) * 0.1;
                let up = 0.18 + (ri as f32 + 0.5) / rows as f32 * (top - 0.35) + (h(x, y, salt + 20 + k) - 0.5) * 0.12;
                let size = side * (0.26 + j * 0.1) + Vec3::new(0.0, 0.22 + j * 0.12, 0.0) + n.abs() * 0.12;
                let col = [LEAF, LEAF_D, LEAF_L, LEAF, LEAF_D][(k as usize + (j * 5.0) as usize) % 5];
                bx(
                    out,
                    c + n * (0.47 + j * 0.05) + side * along + Vec3::new(0.0, up, 0.0),
                    size,
                    Quat::from_rotation_y((j - 0.5) * 0.5),
                    col,
                    SHADE_LIT,
                );
            }
            let count = if rich { 5 + (h(x, y, salt + 40) * 4.0) as u32 } else { 1 + (h(x, y, salt + 40) * 3.0) as u32 };
            for k in 0..count {
                let j = h(x, y, salt + 60 + k);
                let along = j - 0.5;
                let up = 0.35 + h(x, y, salt + 80 + k) * (top - 0.6);
                rose(out, c + n * 0.57 + side * (along * 0.86) + Vec3::new(0.0, up, 0.0), n, 0.1 + j * 0.04, roses[(k as usize) % 3], j);
            }
            if rich && (x + y) % 4 == 0 {
                // A gilded trellis rail along the foot of it.
                bx(
                    out,
                    c + n * 0.55 + Vec3::new(0.0, 0.16, 0.0),
                    side * 1.0 + Vec3::new(0.0, 0.03, 0.0) + n.abs() * 0.03,
                    Quat::IDENTITY,
                    GOLD_D,
                    SHADE_LIT,
                );
            }
        }
        Face::Bramble => {
            // Black stems crossing the face, thorns standing out of them, a dead leaf or two.
            for k in 0..3 {
                let j = h(x, y, salt + 100 + k);
                let a = c + n * 0.5 + side * (j - 0.5) * 0.8 + Vec3::new(0.0, 0.1, 0.0);
                let b = c + n * 0.52 + side * (0.5 - h(x, y, salt + 110 + k)) * 0.9 + Vec3::new(0.0, top - 0.15, 0.0);
                beam(out, a, b, 0.05 + j * 0.03, if k == 0 { BRAMBLE_L } else { BRAMBLE });
                for t in 0..3 {
                    let q = a.lerp(b, 0.2 + t as f32 * 0.3);
                    bx(out, q + n * 0.05, Vec3::new(0.025, 0.025, 0.025) + n.abs() * 0.1, Quat::IDENTITY, THORN, SHADE_LIT);
                }
            }
            let j = h(x, y, salt + 130);
            bx(
                out,
                c + n * 0.53 + side * (j - 0.5) * 0.6 + Vec3::new(0.0, 0.4 + j, 0.0),
                side * 0.14 + Vec3::new(0.0, 0.08, 0.0) + n.abs() * 0.02,
                Quat::from_rotation_y(j),
                DEAD_LEAF,
                SHADE_LIT,
            );
        }
    }
}

/// Black bramble seen from above where the hedge is thickest: a sprig of thorns, now and then a
/// groom's skull caught in it. (The flat top of it is laid by the caller, a run at a time.)
pub(super) fn bramble_top(out: &mut Vec<Instance>, p: P) {
    let (x, y) = (p.x, p.y);
    let c = Vec3::new(x as f32, HEDGE_H, y as f32);
    // A tangle of black canes arching over the top, thorny, a withered leaf caught in it.
    for k in 0..3 {
        let j = h(x, y, 200 + k);
        let a = c + Vec3::new((j - 0.5) * 0.8, -0.02, (h(x, y, 210 + k) - 0.5) * 0.8);
        let b = c + Vec3::new((h(x, y, 250 + k) - 0.5) * 0.9, -0.02, (h(x, y, 260 + k) - 0.5) * 0.9);
        let mid = (a + b) * 0.5 + Vec3::new(0.0, 0.14 + j * 0.16, 0.0);
        let cane = [0x3c2430, 0x2a1820, 0x46303a][k as usize];
        beam(out, a, mid, 0.05, cane);
        beam(out, mid, b, 0.045, cane);
        let tip = mid + Vec3::new((h(x, y, 220 + k) - 0.5) * 0.3, 0.12, (h(x, y, 230 + k) - 0.5) * 0.3);
        beam(out, mid, tip, 0.025, THORN);
    }
    let j = h(x, y, 270);
    if j < 0.35 {
        bx(
            out,
            c + Vec3::new((j - 0.17) * 2.0, 0.06, (h(x, y, 271) - 0.5) * 0.6),
            Vec3::new(0.13, 0.03, 0.09),
            Quat::from_rotation_y(j * 20.0),
            DEAD_LEAF,
            SHADE_LIT,
        );
    } else if j > 0.9 {
        // A rose-hip, dark red, the last of the roses.
        bx(out, c + Vec3::new((j - 0.95) * 4.0, 0.1, (h(x, y, 272) - 0.5) * 0.6), Vec3::splat(0.07), Quat::IDENTITY, 0x6a0c1c, SHADE_LIT);
    }
    if h(x, y, 240) < 0.06 {
        skull(out, c + Vec3::new(0.1, 0.06, 0.0), h(x, y, 241) * TAU, 0.16);
    }
}

/// A skull, `s` across, turned `yaw`.
pub(super) fn skull(out: &mut Vec<Instance>, at: Vec3, yaw: f32, s: f32) {
    let r = Quat::from_rotation_y(yaw);
    bx(out, at + Vec3::new(0.0, s * 0.45, 0.0), Vec3::new(s, s * 0.9, s * 1.1), r, BONE, SHADE_LIT);
    bx(out, at + r * Vec3::new(0.0, s * 0.12, s * 0.42), Vec3::new(s * 0.7, s * 0.3, s * 0.4), r, BONE_D, SHADE_LIT);
    for side in [-1.0f32, 1.0] {
        bx(out, at + r * Vec3::new(side * s * 0.22, s * 0.5, s * 0.56), Vec3::splat(s * 0.22), r, 0x1a1210, SHADE_LIT);
    }
}

/// A dead groom's remains lying along `yaw`: skull, ribs, the long bones, his rusted blade and helm.
pub(super) fn groom(out: &mut Vec<Instance>, at: Vec3, yaw: f32) {
    let r = Quat::from_rotation_y(yaw);
    let p = |v: Vec3| at + r * v;
    skull(out, p(Vec3::new(0.0, 0.0, -0.32)), yaw + 0.4, 0.14);
    bx(out, p(Vec3::new(0.0, 0.03, -0.08)), Vec3::new(0.03, 0.04, 0.3), r, BONE_D, SHADE_LIT);
    for k in 0..4 {
        let z = -0.18 + k as f32 * 0.06;
        bx(out, p(Vec3::new(0.0, 0.05, z)), Vec3::new(0.22 - k as f32 * 0.02, 0.03, 0.025), r, BONE, SHADE_LIT);
    }
    for side in [-1.0f32, 1.0] {
        beam(out, p(Vec3::new(side * 0.06, 0.03, 0.08)), p(Vec3::new(side * 0.12, 0.03, 0.4)), 0.035, BONE);
        beam(out, p(Vec3::new(side * 0.12, 0.03, -0.18)), p(Vec3::new(side * 0.22, 0.03, 0.06)), 0.03, BONE_D);
    }
    // His sword beside him, and his helm rolled away.
    beam(out, p(Vec3::new(0.24, 0.025, -0.36)), p(Vec3::new(0.3, 0.025, 0.36)), 0.035, RUST);
    bx(out, p(Vec3::new(0.25, 0.03, -0.22)), Vec3::new(0.14, 0.03, 0.03), r, BRONZE_D, SHADE_LIT);
    bx(out, p(Vec3::new(-0.26, 0.08, 0.3)), Vec3::new(0.15, 0.13, 0.17), r * Quat::from_rotation_z(0.5), STEEL, SHADE_LIT);
}

/// A rose arch over a path at `c`, spanning `across` (a unit vector), with its lantern hanging
/// from the crown. Returns where the lantern glows.
pub(super) fn arch(out: &mut Vec<Instance>, c: Vec3, across: Vec3, seed: f32) -> Vec3 {
    let span = 1.02;
    let post_h = 2.0;
    let rise = 0.55;
    let roses = [CRIMSON, RED, PINK, WHITE];
    for s in [-1.0f32, 1.0] {
        let foot = c + across * (s * span);
        bx(out, foot + Vec3::new(0.0, 0.08, 0.0), Vec3::new(0.16, 0.16, 0.16), Quat::IDENTITY, MARBLE_D, SHADE_LIT);
        bx(out, foot + Vec3::new(0.0, post_h / 2.0, 0.0), Vec3::new(0.07, post_h, 0.07), Quat::IDENTITY, GOLD, SHADE_LIT);
        // Climbing roses up the post.
        for k in 0..4 {
            let y = 0.4 + k as f32 * 0.42;
            let j = (seed * 7.3 + k as f32 * 1.9 + s).sin() * 0.5 + 0.5;
            bx(
                out,
                foot + Vec3::new(0.0, y, 0.0),
                Vec3::new(0.18, 0.22, 0.18),
                Quat::from_rotation_y(j * 3.0),
                [LEAF, LEAF_L, LEAF_D][k % 3],
                SHADE_LIT,
            );
            if k % 2 == s.max(0.0) as usize {
                rose(out, foot + Vec3::new(0.0, y + 0.08, 0.0) + across * (s * 0.09), across * s, 0.1, roses[(k + seed as usize) % 4], j);
            }
        }
    }
    // The curve over the top, thick with roses.
    let n = 9;
    let mut prev = c + across * (-span) + Vec3::new(0.0, post_h, 0.0);
    for k in 1..=n {
        let a = k as f32 / n as f32 * PI;
        let q = c + across * (-span * a.cos()) + Vec3::new(0.0, post_h + rise * a.sin(), 0.0);
        beam(out, prev, q, 0.06, GOLD);
        let mid = (prev + q) * 0.5;
        bx(
            out,
            mid + Vec3::new(0.0, 0.05, 0.0),
            Vec3::new(0.2, 0.14, 0.2),
            Quat::from_rotation_y(k as f32),
            [LEAF_L, LEAF_MOON, LEAF_L, LEAF][k % 4],
            SHADE_LIT,
        );
        if k % 2 == 1 {
            rose(out, mid + Vec3::new(0.0, 0.14, 0.0), Vec3::Y, 0.12, roses[(k + (seed * 3.0) as usize) % 4], seed + k as f32);
        }
        prev = q;
    }
    // The lantern: a chain, a gilt cap, a rose-glass body.
    let crown = c + Vec3::new(0.0, post_h + rise, 0.0);
    bx(out, crown - Vec3::new(0.0, 0.18, 0.0), Vec3::new(0.015, 0.3, 0.015), Quat::IDENTITY, GOLD_D, SHADE_LIT);
    bx(out, crown - Vec3::new(0.0, 0.34, 0.0), Vec3::new(0.14, 0.04, 0.14), Quat::from_rotation_y(PI / 4.0), GOLD, SHADE_LIT);
    bxg(out, crown - Vec3::new(0.0, 0.45, 0.0), Vec3::new(0.11, 0.17, 0.11), Quat::from_rotation_y(PI / 4.0), ROSE_GLASS, 1.6);
    bx(out, crown - Vec3::new(0.0, 0.55, 0.0), Vec3::new(0.08, 0.03, 0.08), Quat::IDENTITY, GOLD_D, SHADE_LIT);
    crown - Vec3::new(0.0, 0.45, 0.0)
}

/// A fruit tree on `base`: a peach in pink blossom with its fruit, or a dark pomegranate hung
/// with red fruit.
pub(super) fn fruit_tree(out: &mut Vec<Instance>, base: Vec3, peach: bool, seed: f32) {
    let lean = Vec3::new((seed * 3.1).sin() * 0.12, 1.0, (seed * 2.3).cos() * 0.12).normalize();
    let fork = base + lean * 1.0;
    beam(out, base, fork, 0.16, BARK);
    beam(out, base, base + Vec3::new(0.18, 0.12, 0.05), 0.1, BARK);
    let (leaf, light, fruit) = if peach { (PEACH_BLOSSOM, 0xffd4e4, PEACH) } else { (DARK_LEAF, LEAF_L, POMEGRANATE) };
    for k in 0..4 {
        let a = seed * 5.0 + k as f32 * TAU / 4.0;
        let tip = fork + Vec3::new(a.cos() * 0.55, 0.45 + (k % 2) as f32 * 0.2, a.sin() * 0.55);
        beam(out, fork, tip, 0.07, BARK);
    }
    let crown = fork + Vec3::new(0.0, 0.62, 0.0);
    for k in 0..11 {
        let a = seed * 3.0 + k as f32 * 2.4;
        let r = 0.25 + ((k * 7) % 5) as f32 * 0.1;
        let at = crown + Vec3::new(a.cos() * r, ((k * 3) % 4) as f32 * 0.12 - 0.15, a.sin() * r);
        let s = 0.36 + ((k * 5) % 3) as f32 * 0.08;
        bx(out, at, Vec3::new(s, s * 0.75, s), Quat::from_rotation_y(a), if k % 3 == 0 { light } else { leaf }, SHADE_LIT);
    }
    for k in 0..9 {
        let a = seed * 1.7 + k as f32 * 2.1;
        let at = crown + Vec3::new(a.cos() * 0.52, -0.2 + (k % 3) as f32 * 0.16, a.sin() * 0.52);
        bxg(out, at, Vec3::splat(if peach { 0.1 } else { 0.11 }), Quat::from_rotation_y(a), fruit, 0.08);
    }
    // Fallen petals or fruit about the foot.
    for k in 0..5 {
        let a = seed * 4.1 + k as f32 * 1.3;
        let at = base + Vec3::new(a.cos() * (0.3 + k as f32 * 0.1), 0.01, a.sin() * (0.3 + k as f32 * 0.1));
        bx(out, at, Vec3::new(0.07, 0.015, 0.05), Quat::from_rotation_y(a), if peach { PEACH_BLOSSOM } else { POMEGRANATE }, SHADE_LIT);
    }
}

/// A bridal bower on tile `at`: a pergola of gilded posts and rose garlands over a bed of white
/// silk, sheer drapes on three sides (open to the camera), and in it the dead groom.
pub(super) fn bower(out: &mut Vec<Instance>, at: Vec3, seed: f32) {
    let hgt = 1.55;
    // The bed: a low marble plinth and a silk coverlet with a rose-silk runner.
    bx(out, at + Vec3::new(0.0, 0.08, 0.0), Vec3::new(0.86, 0.16, 0.9), Quat::IDENTITY, MARBLE_D, SHADE_LIT);
    bx(out, at + Vec3::new(0.0, 0.19, 0.0), Vec3::new(0.8, 0.06, 0.84), Quat::IDENTITY, SILK, SHADE_LIT);
    bx(out, at + Vec3::new(0.0, 0.225, 0.1), Vec3::new(0.8, 0.012, 0.18), Quat::IDENTITY, SILK_P, SHADE_LIT);
    bx(out, at + Vec3::new(0.0, 0.25, -0.33), Vec3::new(0.5, 0.08, 0.14), Quat::IDENTITY, SILK_P, SHADE_LIT);
    groom(out, at + Vec3::new(0.0, 0.22, 0.04), seed * 0.4);
    for (sx, sz) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let foot = at + Vec3::new(sx * 0.44, 0.0, sz * 0.44);
        bx(out, foot + Vec3::new(0.0, hgt / 2.0, 0.0), Vec3::new(0.06, hgt, 0.06), Quat::IDENTITY, GOLD, SHADE_LIT);
        bx(out, foot + Vec3::new(0.0, hgt, 0.0), Vec3::new(0.1, 0.06, 0.1), Quat::IDENTITY, GOLD_D, SHADE_LIT);
    }
    // The garlands along the top, and slats across it.
    for (a, b) in
        [((-1.0f32, -1.0f32), (1.0f32, -1.0f32)), ((-1.0, 1.0), (1.0, 1.0)), ((-1.0, -1.0), (-1.0, 1.0)), ((1.0, -1.0), (1.0, 1.0))]
    {
        let pa = at + Vec3::new(a.0 * 0.44, hgt, a.1 * 0.44);
        let pb = at + Vec3::new(b.0 * 0.44, hgt, b.1 * 0.44);
        beam(out, pa, pb, 0.05, GOLD_D);
        for k in 0..4 {
            let q = pa.lerp(pb, (k as f32 + 0.5) / 4.0) + Vec3::new(0.0, -0.04 - (k as f32 * 0.8).sin().abs() * 0.06, 0.0);
            bx(out, q, Vec3::splat(0.13), Quat::from_rotation_y(k as f32 + seed), [LEAF, LEAF_L][k % 2], SHADE_LIT);
            if k % 2 == 0 {
                rose(out, q + Vec3::new(0.0, 0.06, 0.0), Vec3::Y, 0.1, [WHITE, PINK, CRIMSON][(k + seed as usize) % 3], seed + k as f32);
            }
        }
    }
    for k in 0..3 {
        let z = -0.3 + k as f32 * 0.3;
        bx(out, at + Vec3::new(0.0, hgt + 0.03, z), Vec3::new(0.9, 0.025, 0.05), Quat::IDENTITY, GOLD_D, SHADE_LIT);
    }
    // Silk drapes tied back at the four posts, falling in folds; a swag across the back.
    for (sx, sz) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let post = at + Vec3::new(sx * 0.44, 0.0, sz * 0.44);
        for k in 0..3 {
            let y = hgt - 0.25 - k as f32 * 0.42;
            let width = 0.2 - (k as f32 - 1.0).abs() * 0.05 - if k == 1 { 0.08 } else { 0.0 };
            let inward = Vec3::new(-sx, 0.0, 0.0) * (width * 0.5);
            bx(
                out,
                post + inward + Vec3::new(0.0, y, 0.0),
                Vec3::new(width, 0.42, 0.06),
                Quat::IDENTITY,
                if k == 1 { SILK_P } else { SILK },
                SHADE_LIT,
            );
        }
        bx(out, post + Vec3::new(-sx * 0.04, hgt - 0.7, 0.0), Vec3::new(0.1, 0.04, 0.09), Quat::IDENTITY, GOLD, SHADE_LIT);
    }
    for k in 0..5 {
        let x = -0.36 + k as f32 * 0.18;
        let sag = (k as f32 - 2.0).abs() * 0.05;
        bx(
            out,
            at + Vec3::new(x, hgt - 0.14 + sag, -0.44),
            Vec3::new(0.19, 0.16, 0.03),
            Quat::IDENTITY,
            if k % 2 == 0 { SILK } else { SILK_P },
            SHADE_LIT,
        );
    }
}

/// A censer on its marble pedestal: a bronze bowl on three legs, coals in it. Returns where the
/// smoke rises from.
pub(super) fn censer(out: &mut Vec<Instance>, at: Vec3) -> Vec3 {
    bx(out, at + Vec3::new(0.0, 0.15, 0.0), Vec3::new(0.7, 0.3, 0.7), Quat::IDENTITY, MARBLE_D, SHADE_LIT);
    bx(out, at + Vec3::new(0.0, 0.33, 0.0), Vec3::new(0.56, 0.06, 0.56), Quat::from_rotation_y(PI / 4.0), MARBLE, SHADE_LIT);
    for k in 0..3 {
        let a = k as f32 * TAU / 3.0;
        beam(out, at + Vec3::new(a.cos() * 0.2, 0.36, a.sin() * 0.2), at + Vec3::new(a.cos() * 0.12, 0.72, a.sin() * 0.12), 0.04, BRONZE_D);
    }
    for (k, (w, y)) in [(0.36f32, 0.74f32), (0.46, 0.82), (0.5, 0.88)].into_iter().enumerate() {
        bx(
            out,
            at + Vec3::new(0.0, y, 0.0),
            Vec3::new(w, 0.08, w),
            Quat::from_rotation_y(k as f32 * 0.4),
            if k == 1 { BRONZE } else { BRONZE_D },
            SHADE_LIT,
        );
    }
    for k in 0..5 {
        let a = k as f32 * 1.3;
        bxg(out, at + Vec3::new(a.cos() * 0.12, 0.93, a.sin() * 0.12), Vec3::splat(0.08), Quat::from_rotation_y(a), 0xff5a2a, 0.8);
    }
    at + Vec3::new(0.0, 1.0, 0.0)
}

/// A koi pond as a marble fountain: a rim round its tiles, dark water, lily pads, and a spout at
/// its north rim. Returns where the jet leaves the spout.
pub(super) fn fountain(out: &mut Vec<Instance>, tiles: &[P], golden: bool) -> Vec3 {
    let inside = |p: P| tiles.contains(&p);
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for t in tiles {
        let c = Vec3::new(t.x as f32, 0.0, t.y as f32);
        lo = lo.min(c);
        hi = hi.max(c);
        bx(out, c + Vec3::new(0.0, -0.2, 0.0), Vec3::new(1.0, 0.1, 1.0), Quat::IDENTITY, 0x1a1a24, SHADE_LIT);
        out.push(Instance::new(
            Mat4::from_scale_rotation_translation(Vec3::new(1.0, 0.02, 1.0), Quat::IDENTITY, c + Vec3::new(0.0, -0.08, 0.0)),
            lin(WATER),
            1.0,
            Vec3::ZERO,
            SHADE_WATER,
        ));
        for d in [Dir::Up, Dir::Down, Dir::Left, Dir::Right] {
            if inside(t.step(d)) {
                continue;
            }
            let (dx, dz) = d.vec();
            let n = Vec3::new(dx as f32, 0.0, dz as f32);
            let side = Vec3::new(dz as f32, 0.0, dx as f32).abs();
            bx(
                out,
                c + n * 0.44 + Vec3::new(0.0, 0.06, 0.0),
                side * 1.06 + Vec3::new(0.0, 0.2, 0.0) + n.abs() * 0.14,
                Quat::IDENTITY,
                MARBLE,
                SHADE_LIT,
            );
            bx(
                out,
                c + n * 0.44 + Vec3::new(0.0, 0.17, 0.0),
                side * 1.08 + Vec3::new(0.0, 0.03, 0.0) + n.abs() * 0.18,
                Quat::IDENTITY,
                if golden { GOLD } else { MARBLE_W },
                SHADE_LIT,
            );
        }
        let j = h(t.x, t.y, 300);
        if j < 0.6 {
            bx(
                out,
                c + Vec3::new((j - 0.3) * 0.8, -0.06, (h(t.x, t.y, 301) - 0.5) * 0.6),
                Vec3::new(0.22, 0.012, 0.2),
                Quat::from_rotation_y(j * 9.0),
                LILY,
                SHADE_LIT,
            );
        }
    }
    let mid = (lo + hi) * 0.5;
    let spout = Vec3::new(mid.x, 0.0, lo.z - 0.44);
    bx(out, spout + Vec3::new(0.0, 0.35, 0.0), Vec3::new(0.16, 0.5, 0.16), Quat::IDENTITY, MARBLE_W, SHADE_LIT);
    bx(
        out,
        spout + Vec3::new(0.0, 0.64, 0.0),
        Vec3::new(0.24, 0.1, 0.24),
        Quat::from_rotation_y(PI / 4.0),
        if golden { GOLD } else { MARBLE },
        SHADE_LIT,
    );
    spout + Vec3::new(0.0, 0.7, 0.0)
}

/// The Bride's own pavilion on the heart's north hedge: a pale marble dais, four slender columns,
/// a silk screen behind her garlanded with roses, a crescent of gilt over it. No roof: she is seen.
pub(super) fn pavilion(out: &mut Vec<Instance>, at: Vec3) {
    let base = at + Vec3::new(0.0, HEDGE_H, 0.0);
    bx(out, base + Vec3::new(0.0, 0.08, 0.0), Vec3::new(2.9, 0.16, 1.9), Quat::IDENTITY, MARBLE, SHADE_LIT);
    bx(out, base + Vec3::new(0.0, 0.2, 0.0), Vec3::new(2.4, 0.1, 1.5), Quat::IDENTITY, MARBLE_W, SHADE_LIT);
    // Steps down its front, over the hedge.
    for k in 0..3 {
        bx(
            out,
            at + Vec3::new(0.0, HEDGE_H - 0.3 * k as f32, 0.95 + k as f32 * 0.2),
            Vec3::new(1.4, 0.12, 0.3),
            Quat::IDENTITY,
            MARBLE_D,
            SHADE_LIT,
        );
    }
    for (sx, sz) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let c = base + Vec3::new(sx * 1.1, 0.0, sz * 0.62);
        bx(out, c + Vec3::new(0.0, 1.5, 0.0), Vec3::new(0.16, 2.6, 0.16), Quat::IDENTITY, MARBLE_W, SHADE_LIT);
        bx(out, c + Vec3::new(0.0, 2.85, 0.0), Vec3::new(0.26, 0.1, 0.26), Quat::IDENTITY, GOLD, SHADE_LIT);
        bx(out, c + Vec3::new(0.0, 0.3, 0.0), Vec3::new(0.24, 0.1, 0.24), Quat::IDENTITY, MARBLE_D, SHADE_LIT);
    }
    // The silk screen behind, in folds, and roses along its top.
    for k in 0..9 {
        let x = -1.0 + k as f32 * 0.25;
        bx(
            out,
            base + Vec3::new(x, 1.45, -0.7 - (k % 2) as f32 * 0.03),
            Vec3::new(0.25, 2.5, 0.04),
            Quat::IDENTITY,
            if k % 3 == 1 { SILK_P } else { SILK },
            SHADE_LIT,
        );
        rose(out, base + Vec3::new(x, 2.75, -0.66), Vec3::Z, 0.13, [WHITE, PINK, CRIMSON][k % 3], k as f32);
    }
    let n = 11;
    let mut prev = base + Vec3::new(-1.1, 2.9, -0.62);
    for k in 1..=n {
        let a = k as f32 / n as f32 * PI;
        let q = base + Vec3::new(-1.1 * a.cos(), 2.9 + a.sin() * 0.7, -0.62);
        beam(out, prev, q, 0.07, GOLD);
        prev = q;
    }
}

/// A fairy ring of pale mushrooms round `c`, where the elf-maids dance.
pub(super) fn fairy_ring(out: &mut Vec<Instance>, c: Vec3, r: f32) {
    for k in 0..22 {
        let a = k as f32 / 22.0 * TAU;
        let at = c + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
        let s = 0.06 + (k % 3) as f32 * 0.02;
        bx(out, at + Vec3::new(0.0, s * 0.6, 0.0), Vec3::new(s * 0.4, s * 1.2, s * 0.4), Quat::IDENTITY, 0xd8d0c8, SHADE_LIT);
        bxg(out, at + Vec3::new(0.0, s * 1.3, 0.0), Vec3::new(s * 1.4, s * 0.5, s * 1.4), Quat::from_rotation_y(a), MUSHROOM, 0.35);
    }
}

/// The Black Thorn, rooted at `base`: a dead tree of black thorns, low and wide, its crooked limbs
/// spreading over the gate beside it (to the east) and back over the bramble, thorns all along
/// them. It is a silhouette, rimmed faintly in the realm's violet so that it reads against the dark.
pub(super) fn black_thorn(out: &mut Vec<Instance>, base: Vec3) {
    let rim = |c: u32| lin(c);
    let limb = |out: &mut Vec<Instance>, a: Vec3, b: Vec3, t: f32| {
        let d = b - a;
        let len = d.length().max(1e-4);
        out.push(Instance::new(
            Mat4::from_scale_rotation_translation(Vec3::new(t, len, t), Quat::from_rotation_arc(Vec3::Y, d / len), (a + b) * 0.5),
            rim(0x0c080e),
            1.0,
            rim(0x4a1656) * 0.2,
            SHADE_LIT,
        ));
    };
    let thorn = |out: &mut Vec<Instance>, at: Vec3, dir: Vec3, len: f32| {
        let tip = at + dir * len;
        out.push(Instance::new(
            Mat4::from_scale_rotation_translation(Vec3::new(0.035, len, 0.035), Quat::from_rotation_arc(Vec3::Y, dir), (at + tip) * 0.5),
            rim(0x140a10),
            1.0,
            rim(0xa01838) * 0.32,
            SHADE_LIT,
        ));
    };
    // The trunk: thick and twisted, leaning toward the gate.
    let mut at = base;
    let mut dir = Vec3::new(0.25, 1.0, 0.15).normalize();
    for k in 0..4 {
        let next = at + dir * 0.55;
        limb(out, at, next, 0.62 - k as f32 * 0.07);
        for t in 0..5 {
            let a = k as f32 * 1.7 + t as f32 * 1.26;
            let q = at.lerp(next, 0.2 + t as f32 * 0.15);
            thorn(out, q + Vec3::new(a.cos(), 0.0, a.sin()) * 0.26, Vec3::new(a.cos(), 0.35, a.sin()).normalize(), 0.3);
        }
        at = next;
        dir = (dir + Vec3::new((k as f32 * 2.1).sin() * 0.25, 0.0, (k as f32 * 1.3).cos() * 0.15)).normalize();
    }
    // Limbs: spreading low and wide, three levels of them, the greatest over the gate.
    let mut tips: Vec<(Vec3, Vec3, f32, f32)> = vec![
        (at, Vec3::new(1.0, 0.18, 0.3).normalize(), 2.2, 0.3),
        (at, Vec3::new(-1.0, 0.25, -0.1).normalize(), 1.6, 0.26),
        (at, Vec3::new(0.3, 0.45, 0.9).normalize(), 1.3, 0.22),
        (at, Vec3::new(0.1, 0.9, -0.4).normalize(), 0.9, 0.2),
        (at - dir * 0.8, Vec3::new(0.8, -0.05, 0.6).normalize(), 1.5, 0.22),
    ];
    for depth in 0..3 {
        let mut next = Vec::new();
        for (i, &(from, d, len, thick)) in tips.iter().enumerate() {
            let to = from + d * len;
            limb(out, from, to, thick);
            for t in 0..4 {
                let q = from.lerp(to, 0.15 + t as f32 * 0.22);
                let a = (i * 5 + t * 3 + depth) as f32 * 1.9;
                let side = d.cross(Vec3::new(a.cos(), 0.4, a.sin())).normalize_or(Vec3::Y);
                thorn(out, q, side, 0.26 - depth as f32 * 0.05);
            }
            if depth < 2 {
                for b in 0..2 {
                    let a = (i as f32 + depth as f32 * 3.0) * 1.3 + b as f32 * 2.6;
                    let nd = (d + Vec3::new(a.cos() * 0.6, if b == 0 { 0.2 } else { -0.15 }, a.sin() * 0.6)).normalize();
                    next.push((to, nd, len * 0.62, thick * 0.6));
                }
            }
        }
        tips = next;
    }
    // Roots clutching the ground.
    for k in 0..6 {
        let a = k as f32 * 1.05 + 0.4;
        limb(out, base + Vec3::new(0.0, 0.25, 0.0), base + Vec3::new(a.cos() * 1.0, -0.05, a.sin() * 0.7 + 0.2), 0.16);
    }
}
