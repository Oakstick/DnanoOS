//! The title stage's set, to the art direction: a ruined sanctum of dressed stone in the board's
//! palette. Behind the heroes a great round arch opens on the moonlit mist, between crumbling
//! walls in courses of charcoal and umber blocks; two pillars carry copper and umber banners with
//! amber sigils; hooded stone knights keep watch either side; lanterns hang on iron brackets;
//! flagstones run underfoot with moss in their seams, and a few dark pines stand beyond the
//! walls. It never changes, so it is built once.

use super::*;
use crypt::pal::*;
use crypt::SHADE_STONE;
use std::sync::OnceLock;

const SHADE_BRONZE: f32 = 13.0;
const SHADE_CLOTH: f32 = 8.0;
const SHADE_SOFT: f32 = 7.0;
const SHADE_IRON: f32 = 6.0;

/// The back wall's face, the arch's half width and spring, and the pillars' places.
const WALL_Z: f32 = -2.6;
const ARCH_R: f32 = 1.05;
const SPRING: f32 = 2.0;
const PILLAR_X: f32 = 2.25;
/// Where the wall's lanterns hang, as (x, height): their light is the set's own.
pub(super) const LANTERNS: [(f32, f32); 2] = [(-3.05, 1.95), (3.05, 1.95)];
pub(super) const LANTERN_Z: f32 = WALL_Z + 0.62;

fn jit(hex: u32, h: f64, amount: f32) -> Vec3 {
    lin(hex) * (1.0 - amount + h as f32 * amount * 2.0)
}

struct Set(Vec<Instance>);

impl Set {
    fn block(&mut self, pos: Vec3, size: Vec3, yaw: f32, color: Vec3, shade: f32) {
        let m = Mat4::from_scale_rotation_translation(size, Quat::from_rotation_y(yaw), STAGE + pos);
        self.0.push(Instance::new(m, color, 1.0, Vec3::ZERO, shade));
    }
    fn block_q(&mut self, pos: Vec3, size: Vec3, q: Quat, color: Vec3, shade: f32) {
        let m = Mat4::from_scale_rotation_translation(size, q, STAGE + pos);
        self.0.push(Instance::new(m, color, 1.0, Vec3::ZERO, shade));
    }
    fn glowing(&mut self, pos: Vec3, size: Vec3, color: Vec3, glow: Vec3) {
        let m = Mat4::from_scale_rotation_translation(size, Quat::IDENTITY, STAGE + pos);
        self.0.push(Instance::new(m, color, 1.0, glow, SHADE_LIT));
    }
}

/// The set's pieces, in the title stage's place.
pub(super) fn sanctum() -> &'static [Instance] {
    static SET: OnceLock<Vec<Instance>> = OnceLock::new();
    SET.get_or_init(build)
}

fn build() -> Vec<Instance> {
    let mut s = Set(Vec::new());
    floor(&mut s);
    wall(&mut s);
    arch(&mut s);
    for side in [-1.0f32, 1.0] {
        pillar(&mut s, side);
        statue(&mut s, Vec3::new(side * 3.75, 0.0, -1.55), -side * 0.35, side as f64);
    }
    for (x, y) in LANTERNS {
        lantern(&mut s, x, y);
    }
    s.0
}

/// Flagstones over dark grout (dry: wet ones threw noisy glints round the boys), moss in some
/// seams.
fn floor(s: &mut Set) {
    s.block(Vec3::new(0.0, -0.07, -0.5), Vec3::new(13.0, 0.1, 8.0), 0.0, lin(GROUT), SHADE_LIT);
    for z in -4..=3 {
        for x in -6..=6 {
            let (hx, hz) = (x as f64, z as f64);
            for (k, (ox, oz)) in [(-0.25f32, -0.25f32), (0.25, -0.25), (-0.25, 0.25), (0.25, 0.25)].into_iter().enumerate() {
                let hk = hash(hx * 2.0 + ox as f64 * 7.0 + 31.0, hz * 2.0 + oz as f64 * 5.0 + 17.0);
                let c = jit(FLAG[(hk * 3.0) as usize % 3], hash(hz + k as f64, hx), 0.12) * 0.75;
                let tilt = (hk as f32 - 0.5) * 0.012;
                s.block(Vec3::new(x as f32 + ox, -0.025 + tilt, z as f32 + oz), Vec3::new(0.47, 0.05, 0.47), 0.0, c, SHADE_STONE);
            }
            let hm = hash(hx * 4.1 + 9.0, hz * 3.7 + 2.0);
            if hm < 0.22 {
                let along_x = hm < 0.11;
                let len = 0.35 + hm as f32 * 2.5;
                let size = if along_x { Vec3::new(len, 0.02, 0.05) } else { Vec3::new(0.05, 0.02, len) };
                s.block(Vec3::new(x as f32, 0.003, z as f32), size, 0.0, jit(MOSS[(hm * 30.0) as usize % 3], hm, 0.1), SHADE_SOFT);
            }
        }
    }
}

/// The back wall either side of the arch: a core, faced in courses of blocks of uneven length
/// and depth, its top ragged and lower toward the ends, broken coping and moss along it.
fn wall(s: &mut Set) {
    let course = 0.42;
    for side in [-1.0f32, 1.0] {
        // Column by column, from the arch's pier out to the end.
        let mut x0 = ARCH_R + 0.2;
        let mut col = 0;
        while x0 < 5.8 {
            let w = 0.7 + hash(col as f64 * 3.1, side as f64 + 4.0) as f32 * 0.35;
            let x1 = (x0 + w).min(5.8);
            let hc = hash(col as f64 * 1.7 + 2.0, side as f64 * 3.0) as f32;
            // Taller by the arch, lower toward the ends, and here and there a gap bitten out.
            let top = (3.1 - (x0 - ARCH_R) * 0.22 - hc * 0.5).max(0.9);
            let cx = side * (x0 + x1) / 2.0;
            s.block(
                Vec3::new(cx, top / 2.0, WALL_Z - 0.35),
                Vec3::new(x1 - x0, top, 0.62),
                0.0,
                jit(CHARCOAL[1], hc as f64, 0.05) * 0.7,
                SHADE_STONE,
            );
            let courses = (top / course).floor() as usize;
            for c in 0..courses {
                let shift = if c % 2 == 0 { 0.0 } else { 0.22 };
                let mut u = x0 - shift;
                let mut k = 0;
                while u < x1 {
                    let hk = hash(col as f64 * 5.0 + k as f64 * 1.3 + c as f64 * 7.0, side as f64 * 11.0 + c as f64);
                    let len = 0.32 + hk as f32 * 0.3;
                    let (a, b) = (u.max(x0), (u + len).min(x1));
                    // The top course of each column is part gone.
                    let gone = c + 1 == courses && hash(hk * 17.0, col as f64) < 0.45;
                    if b - a > 0.08 && !gone {
                        let roll = hash(hk * 31.0, side as f64 + col as f64);
                        let col_c = if roll < 0.06 {
                            jit(VERDIGRIS, hk, 0.1)
                        } else if roll < 0.24 {
                            jit(UMBER[(hk * 2.0) as usize % 2], hk, 0.1)
                        } else {
                            jit(CHARCOAL[(hk * 3.0) as usize % 3], hk, 0.12)
                        };
                        let depth = 0.1 + hash(hk * 13.0, 3.0) as f32 * 0.1;
                        let chip = if (hk * 29.0).fract() < 0.25 { (hk * 53.0).fract() as f32 * 0.06 } else { 0.0 };
                        let y = c as f32 * course + course / 2.0 - chip / 2.0;
                        let askew = ((hk * 41.0).fract() as f32 - 0.5) * 0.05;
                        s.block(
                            Vec3::new(side * (a + b) / 2.0, y, WALL_Z - 0.05 + depth / 2.0),
                            Vec3::new(b - a - 0.035, course - 0.035 - chip, depth),
                            askew,
                            col_c,
                            SHADE_STONE,
                        );
                    }
                    u += len;
                    k += 1;
                }
            }
            // A pale plinth along the foot, and moss on what is left of the top.
            s.block(
                Vec3::new(cx, 0.1, WALL_Z + 0.08),
                Vec3::new(x1 - x0, 0.2, 0.14),
                0.0,
                jit(PALE_STONE[1], hc as f64, 0.05) * 0.7,
                SHADE_STONE,
            );
            if hc < 0.6 {
                let mw = (x1 - x0) * (0.4 + hc * 0.5);
                s.block(
                    Vec3::new(cx, top + 0.02, WALL_Z - 0.3),
                    Vec3::new(mw, 0.05, 0.45),
                    hc * 0.3,
                    jit(MOSS[col % 3], hc as f64, 0.1),
                    SHADE_SOFT,
                );
            }
            x0 = x1;
            col += 1;
        }
    }
}

/// The great arch behind Ragnar: pale piers, a ring of wedge stones round a pale keystone, the
/// wall filled in over it, and the moonlit mist beyond.
fn arch(s: &mut Set) {
    let depth = 0.8;
    for side in [-1.0f32, 1.0] {
        let c = jit(PALE_STONE[1], side as f64 + 2.0, 0.05) * 0.75;
        s.block(Vec3::new(side * (ARCH_R + 0.12), SPRING / 2.0, WALL_Z - 0.2), Vec3::new(0.3, SPRING, depth), 0.0, c, SHADE_STONE);
        s.block(
            Vec3::new(side * (ARCH_R + 0.1), SPRING + 0.06, WALL_Z - 0.2),
            Vec3::new(0.42, 0.12, depth + 0.12),
            0.0,
            c * 1.05,
            SHADE_STONE,
        );
        s.block(Vec3::new(side * (ARCH_R + 0.12), 0.12, WALL_Z - 0.2), Vec3::new(0.42, 0.24, depth + 0.1), 0.0, c * 0.9, SHADE_STONE);
    }
    let wedges = 13;
    for k in 0..wedges {
        let a = PI * (k as f32 + 0.5) / wedges as f32;
        let key = k == wedges / 2;
        let c = if key {
            jit(PALE_STONE[0], 0.4, 0.04) * 0.85
        } else if k % 2 == 0 {
            jit(PALE_STONE[1], k as f64, 0.05) * 0.7
        } else {
            jit(CHARCOAL[k % 3], k as f64, 0.1)
        };
        let r = ARCH_R + 0.14;
        let size = Vec3::new(PI * r / wedges as f32 + 0.03, if key { 0.4 } else { 0.3 }, depth + if key { 0.08 } else { 0.0 });
        s.block_q(
            Vec3::new(-a.cos() * r, SPRING + 0.12 + a.sin() * r, WALL_Z - 0.2),
            size,
            Quat::from_rotation_z(a - PI / 2.0),
            c,
            SHADE_STONE,
        );
    }
    // Over the ring: the wall built up in umber and charcoal, a pale cornice on top.
    let crown = SPRING + 0.12 + ARCH_R + 0.3;
    for side in [-1.0f32, 1.0] {
        s.block(
            Vec3::new(side * (ARCH_R * 0.62 + 0.15), SPRING + 0.75, WALL_Z - 0.35),
            Vec3::new(ARCH_R * 0.75, 1.1, 0.6),
            0.0,
            jit(UMBER[1], side as f64, 0.06) * 0.8,
            SHADE_STONE,
        );
    }
    s.block(
        Vec3::new(0.0, crown + 0.2, WALL_Z - 0.3),
        Vec3::new(2.0 * ARCH_R + 0.7, 0.4, 0.66),
        0.0,
        jit(CHARCOAL[0], 1.0, 0.05) * 0.85,
        SHADE_STONE,
    );
    s.block(
        Vec3::new(0.0, crown + 0.46, WALL_Z - 0.3),
        Vec3::new(2.0 * ARCH_R + 0.95, 0.12, 0.8),
        0.0,
        jit(PALE_STONE[1], 2.0, 0.04) * 0.75,
        SHADE_STONE,
    );
    s.block(Vec3::new(0.35, crown + 0.54, WALL_Z - 0.25), Vec3::new(0.7, 0.05, 0.5), 0.2, lin(MOSS[1]), SHADE_SOFT);
    // A copper medallion hung on the keystone, and the dark of the way through.
    s.block(Vec3::new(0.0, SPRING + 0.12 + ARCH_R + 0.05, WALL_Z + 0.26), Vec3::new(0.2, 0.2, 0.04), PI / 4.0, lin(COPPER), SHADE_BRONZE);
}

/// A pillar before the wall: a pale moulded base, drums of charcoal and umber, a pale capital,
/// and a banner hung from a copper rod across its face.
fn pillar(s: &mut Set, side: f32) {
    let (x, z) = (side * PILLAR_X, WALL_Z + 0.4);
    let h = 3.0;
    s.block(Vec3::new(x, 0.14, z), Vec3::new(0.78, 0.28, 0.78), 0.0, jit(PALE_STONE[0], side as f64, 0.05) * 0.75, SHADE_STONE);
    let drums = 7;
    for k in 0..drums {
        let dh = (h - 0.6) / drums as f32;
        let c = if k % 3 == 1 { jit(UMBER[0], k as f64, 0.06) } else { jit(CHARCOAL[k % 3], k as f64 + side as f64, 0.08) };
        s.block(Vec3::new(x, 0.28 + dh * (k as f32 + 0.5), z), Vec3::new(0.58, dh - 0.025, 0.58), 0.0, c, SHADE_STONE);
    }
    s.block(Vec3::new(x, h - 0.17, z), Vec3::new(0.82, 0.34, 0.82), 0.0, jit(PALE_STONE[1], side as f64, 0.05) * 0.7, SHADE_STONE);
    s.block(Vec3::new(x + side * 0.1, h + 0.02, z - 0.05), Vec3::new(0.5, 0.05, 0.45), 0.4, lin(MOSS[0]), SHADE_SOFT);
    // The banner.
    let face = z + 0.33;
    let cloth = lin(BANNER[if side < 0.0 { 0 } else { 1 }]);
    s.block(Vec3::new(x, 2.35, face + 0.02), Vec3::new(0.78, 0.05, 0.05), 0.0, lin(COPPER), SHADE_BRONZE);
    for side_end in [-1.0f32, 1.0] {
        s.block(Vec3::new(x + side_end * 0.4, 2.35, face + 0.02), Vec3::new(0.07, 0.09, 0.07), 0.0, lin(COPPER) * 1.1, SHADE_BRONZE);
    }
    s.block(Vec3::new(x, 1.6, face), Vec3::new(0.6, 1.45, 0.02), 0.0, cloth, SHADE_CLOTH);
    for u in [-0.17f32, 0.17] {
        s.block(Vec3::new(x + u, 0.8, face), Vec3::new(0.24, 0.18, 0.02), 0.0, cloth, SHADE_CLOTH);
    }
    let thread = lin(AMBER);
    s.block(Vec3::new(x, 1.75, face + 0.012), Vec3::new(0.06, 0.5, 0.01), 0.0, thread, SHADE_CLOTH);
    s.block(Vec3::new(x, 1.88, face + 0.012), Vec3::new(0.34, 0.06, 0.01), 0.0, thread, SHADE_CLOTH);
    s.block(Vec3::new(x, 1.55, face + 0.012), Vec3::new(0.24, 0.05, 0.01), 0.0, thread, SHADE_CLOTH);
    s.block(Vec3::new(x, 2.12, face + 0.012), Vec3::new(0.5, 0.03, 0.01), 0.0, thread * 0.8, SHADE_CLOTH);
}

/// A hooded knight of stone on a plinth, facing the camera turned by `yaw`: robe, mantle, the
/// hood's shadow for a face, hands on the pommel of a sword stood point down.
fn statue(s: &mut Set, at: Vec3, yaw: f32, h: f64) {
    let q = Quat::from_rotation_y(yaw);
    let k = 1.3;
    // Pale stone, so the figures read against the darker wall behind them.
    let stone = jit(PALE_STONE[1], h, 0.05) * 0.72;
    let dark = jit(CHARCOAL[1], h, 0.05) * 0.8;
    let pale = jit(PALE_STONE[0], h + 1.0, 0.05) * 0.85;
    let put = |s: &mut Set, c: Vec3, size: Vec3, col: Vec3| s.block_q(at + q * (c * k), size * k, q, col, SHADE_STONE);
    put(s, Vec3::new(0.0, 0.16, 0.0), Vec3::new(0.82, 0.32, 0.64), pale * 0.75);
    put(s, Vec3::new(0.0, 0.35, 0.0), Vec3::new(0.7, 0.06, 0.54), pale * 0.65);
    put(s, Vec3::new(0.0, 0.54, 0.0), Vec3::new(0.62, 0.32, 0.44), stone * 0.9);
    put(s, Vec3::new(0.0, 0.84, 0.0), Vec3::new(0.54, 0.3, 0.38), stone);
    put(s, Vec3::new(0.0, 1.12, 0.0), Vec3::new(0.48, 0.28, 0.34), stone * 1.05);
    put(s, Vec3::new(0.0, 0.94, -0.2), Vec3::new(0.66, 1.0, 0.08), stone * 0.8);
    put(s, Vec3::new(0.0, 1.36, -0.02), Vec3::new(0.72, 0.18, 0.42), stone * 0.95);
    for arm in [-1.0f32, 1.0] {
        let rot = q * Quat::from_rotation_x(-0.5) * Quat::from_rotation_z(arm * 0.35);
        s.block_q(at + q * (Vec3::new(arm * 0.2, 1.16, 0.12) * k), Vec3::new(0.13, 0.42, 0.14) * k, rot, stone * 0.92, SHADE_STONE);
    }
    put(s, Vec3::new(0.0, 1.59, 0.0), Vec3::new(0.34, 0.32, 0.36), stone);
    put(s, Vec3::new(0.0, 1.77, -0.03), Vec3::new(0.24, 0.1, 0.28), stone * 0.95);
    put(s, Vec3::new(0.0, 1.54, 0.16), Vec3::new(0.2, 0.2, 0.06), dark * 0.35);
    put(s, Vec3::new(0.0, 0.74, 0.3), Vec3::new(0.07, 0.8, 0.04), pale * 0.7);
    put(s, Vec3::new(0.0, 1.14, 0.3), Vec3::new(0.32, 0.05, 0.06), pale * 0.65);
    put(s, Vec3::new(0.0, 1.24, 0.29), Vec3::new(0.2, 0.12, 0.13), stone * 1.1);
    put(s, Vec3::new(0.14, 1.53, 0.0), Vec3::new(0.18, 0.04, 0.3), lin(VERDIGRIS));
    put(s, Vec3::new(-0.2, 0.41, 0.12), Vec3::new(0.26, 0.03, 0.2), lin(MOSS[0]));
}

/// A lantern of copper and amber glass on an iron bracket from the wall.
fn lantern(s: &mut Set, x: f32, y: f32) {
    let iron = lin(IRON);
    let wall = WALL_Z + 0.05;
    s.block(Vec3::new(x, y + 0.2, wall + 0.03), Vec3::new(0.12, 0.22, 0.08), 0.0, iron, SHADE_IRON);
    s.block(Vec3::new(x, y + 0.28, wall + 0.27), Vec3::new(0.04, 0.04, 0.5), 0.0, iron, SHADE_IRON);
    let c = Vec3::new(x, y, LANTERN_Z);
    let copper = lin(COPPER);
    s.block(c + Vec3::Y * 0.2, Vec3::new(0.025, 0.12, 0.025), 0.0, iron, SHADE_IRON);
    s.block(c + Vec3::Y * 0.14, Vec3::new(0.22, 0.05, 0.22), PI / 4.0, copper, SHADE_BRONZE);
    s.block(c - Vec3::Y * 0.14, Vec3::new(0.2, 0.05, 0.2), PI / 4.0, copper * 0.9, SHADE_BRONZE);
    for k in 0..4 {
        let a = k as f32 * TAU / 4.0 + PI / 4.0;
        s.block(c + Vec3::new(a.cos() * 0.1, 0.0, a.sin() * 0.1), Vec3::new(0.025, 0.26, 0.025), 0.0, copper * 0.8, SHADE_BRONZE);
    }
    s.glowing(c, Vec3::new(0.15, 0.22, 0.15), lin(AMBER), lin(AMBER) * 1.8);
}
