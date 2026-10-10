//! The Hall of Mirrors' art that is not architecture: the Morning Star's sunburst over every
//! mirror-door, the peacock-eye windows of stained glass, the relay mirrors on their stands, and
//! the throne itself. Each is sculpted in voxels and merged into boxes (`vault_decor::merge`).

use super::super::super::vault_decor::{merge, Merged};
use crate::render::characters::{mat, Sculpt};

const MARBLE_MAT: u32 = 10 << 24;
const G: u32 = 0xf2c53d | mat::GOLD;
const GD: u32 = 0xb8902a | mat::GOLD;
const GL: u32 = 0xfff0a0 | mat::GOLD;
const M: u32 = 0xf2eef8 | MARBLE_MAT;
const MD: u32 = 0xd2c8e0 | MARBLE_MAT;
const VELVET: u32 = 0x5a1a8a | mat::CLOTH;
const VELVET_D: u32 = 0x3a1060 | mat::CLOTH;
const AMETHYST: u32 = 0xb07aff;

/// The Morning Star in gold: a disc with sixteen rays, long and short, standing upright (+z out of
/// the wall), its base at y = 0. About a tile across.
pub fn sunburst() -> Merged {
    let n = 21;
    let mut s = Sculpt::new(n, n, 2);
    let c = (n - 1) as f32 / 2.0;
    for y in 0..n {
        for x in 0..n {
            let (dx, dy) = (x as f32 - c, y as f32 - c);
            let r = (dx * dx + dy * dy).sqrt();
            let a = dy.atan2(dx);
            let ray = (a * 8.0).cos();
            let long = (a * 16.0).cos() > 0.0;
            if r <= 3.2 {
                s.set(x, y, 1, if r <= 1.6 { GL } else { G });
                s.set(x, y, 0, GD);
            } else if ray > 0.92 && r <= c || (long && ray > 0.55 && r <= c * 0.72) {
                s.set(x, y, 1, if r < c * 0.5 { G } else { GD });
            }
        }
    }
    merge(&s.finish(0.0), 0.05)
}

/// A peacock's eye in stained glass, lit from behind: an upright oval of teal, sapphire, gold and
/// violet, barbs fanning round it, in a gold frame (+z out of the wall).
pub fn peacock_window() -> Merged {
    let (w, h) = (13, 21);
    let mut s = Sculpt::new(w, h, 1);
    let (cx, cy) = ((w - 1) as f32 / 2.0, 11.0);
    for y in 0..h {
        for x in 0..w {
            let (dx, dy) = ((x as f32 - cx) / 6.0, (y as f32 - cy) / 9.5);
            let r = (dx * dx + dy * dy).sqrt();
            let edge = x == 0 || x == w - 1 || y == 0 || y == h - 1;
            let c = if edge {
                G
            } else if r < 0.22 {
                0x1a0a3a
            } else if r < 0.36 {
                0x2a4ad8
            } else if r < 0.48 {
                0x1ab0a0
            } else if r < 0.6 {
                0xf2c53d
            } else if (dx.atan2(dy) * 7.0).sin() > 0.6 {
                0x2a8a6a
            } else {
                0x4a2a8a
            };
            s.set(x, y, 0, c);
        }
    }
    merge(&s.finish(0.0), 0.06)
}

/// A relay mirror: a gold tripod holding a round glass tilted to catch the light of the well, the
/// glass facing +z. Its base at y = 0.
pub fn relay_stand() -> Merged {
    let mut s = Sculpt::new(13, 26, 9);
    for (fx, fz) in [(2.0, 1.0), (10.0, 1.0), (6.0, 8.0)] {
        s.capsule((fx, 0.0, fz), (6.0, 12.0, 4.0), 0.7, 0.7, GD);
    }
    s.capsule((6.0, 10.0, 4.0), (6.0, 15.0, 4.0), 1.0, 1.0, G);
    for y in 14..26 {
        for x in 0..13 {
            let (dx, dy) = (x as f32 - 6.0, y as f32 - 20.0);
            let r = (dx * dx + dy * dy).sqrt();
            if r <= 6.0 {
                let z = 4 + ((y - 20) as f32 * -0.25).round() as i32;
                s.set(x, y, z.clamp(0, 8), if r > 5.0 { GL } else { 0xe8e4f4 | mat::METAL });
            }
        }
    }
    merge(&s.finish(0.0), 0.05)
}

/// The throne: a gold seat on a violet cushion, its tall back crowned with the Morning Star and
/// set with amethysts, facing +z. About a tile wide and two tall.
pub fn throne() -> Merged {
    let (w, h, d) = (19, 40, 14);
    let mut s = Sculpt::new(w, h, d);
    // The seat and its legs.
    s.cuboid(1, 0, 3, 17, 9, 13, GD);
    s.cuboid(2, 1, 4, 16, 8, 12, M);
    s.cuboid(1, 9, 3, 17, 10, 13, G);
    s.cuboid(3, 11, 4, 15, 12, 12, VELVET);
    // The arms, ending in lions' paws of gold.
    for x in [0, 16] {
        s.cuboid(x, 10, 4, x + 2, 16, 13, G);
        s.cuboid(x, 17, 10, x + 2, 18, 13, GL);
    }
    // The back: tall, rounded at the top, velvet within gold.
    for y in 10..38 {
        let top = y as f32 - 30.0;
        let half = if top > 0.0 { (9.0 * (1.0 - (top / 8.0).powi(2)).max(0.0).sqrt()) as i32 } else { 9 };
        for x in (9 - half)..=(9 + half) {
            for z in 0..3 {
                let rim = (x - 9).abs() >= half - 1 || y >= 37;
                // The back, seen from behind: gold panels round a violet inlay and a sunburst.
                let back = if z == 0 && !rim {
                    let (dx, dy) = ((x - 9) as f32, (y - 28) as f32);
                    let r = (dx * dx + dy * dy).sqrt();
                    if r < 2.2 {
                        GL
                    } else if r < 6.5 && ((dy.atan2(dx) * 8.0).cos() > 0.7) {
                        G
                    } else if (x - 9).abs() >= half - 3 || y <= 12 {
                        GD
                    } else {
                        VELVET_D
                    }
                } else {
                    G
                };
                s.set(
                    x,
                    y,
                    z,
                    if rim {
                        G
                    } else if z == 0 {
                        back
                    } else if (x + y) % 5 == 0 {
                        VELVET_D
                    } else {
                        VELVET
                    },
                );
            }
        }
    }
    // The Morning Star on its crest, and amethysts down the back.
    for (dx, dy) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1), (2, 0), (-2, 0), (0, 2), (0, -2)] {
        s.set(9 + dx, 34 + dy, 3, if dx == 0 && dy == 0 { 0xffffff | mat::GLOW } else { GL });
    }
    for y in [16, 22, 28] {
        s.set(9, y, 3, AMETHYST | mat::GLOW);
    }
    s.paint(|x, y, _, c| Some(if c == M && (x + y) % 7 == 0 { MD } else { c }));
    merge(&s.finish(0.0), 0.045)
}
