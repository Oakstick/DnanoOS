//! Lust's court, **the Rose Bower** (`logic::court::lust` lays it out), where Asmodeus waits: its
//! art, to the Garden of Thorns' palette (moonlight on crimson; rose marble, gold, black bramble).
//!
//! A walled garden under the night sky: its walls are hedges as tall as a man and a half, dark leaf
//! thick with crimson, pink and white roses on the garden side, black thorn canes over their tops.
//! Paths of rose marble cross it north to south and east to west, and ring the fountain in the
//! middle; the rest is lawn strewn with fallen petals. The fountain is a round marble basin, a
//! pedestal in it lifting a gilt bowl and a golden rose, water arching from the bowl into the
//! basin with petals floating on it. Four bronze censers stand on marble plinths on round pads of
//! marble, cold: struck, their embers glow and their incense smoke rises (the view reads them from
//! the embers the fight leaves in their bowls). Two runs of hedge stand out from the east and west
//! walls, rose beds near the corners, and lanterns of rose glass on gilt posts beside the paths. At
//! the north end the bridal arch, marble posts wound with roses under a gilt curve draped in veils,
//! and behind it the bridal bed under its canopy, crimson and white and strewn with petals, candles
//! at its feet. The gate he came in by is Lust's portal in the south hedge. Petals drift down and
//! fireflies wander. His thorns from the ground are drawn here too (`emit_bower_thorns`).

use super::*;
use crate::logic::combat::prince::{Trap, SPIKES};
use crate::logic::court::lust::{self as court, Piece, ARCH, BED, CENSERS, CENTRE, FOUNTAIN};
use crate::logic::hub::Gate;

mod pal {
    pub const LEAF_DEEP: u32 = 0x10241a;
    pub const LEAF_D: u32 = 0x1a3a24;
    pub const LEAF: u32 = 0x24502e;
    pub const LEAF_L: u32 = 0x31663a;
    pub const LEAF_MOON: u32 = 0x4e7c62;
    pub const CRIMSON: u32 = 0xa80e2c;
    pub const RED: u32 = 0xd81c46;
    pub const PINK: u32 = 0xff4a9a;
    pub const BLUSH: u32 = 0xffaacc;
    pub const WHITE: u32 = 0xf6eaf0;
    pub const BRAMBLE: u32 = 0x22141a;
    pub const CANE: u32 = 0x3c2430;
    pub const THORN: u32 = 0x0a0507;
    pub const LAWN: u32 = 0x2e5c34;
    pub const LAWN_L: u32 = 0x3a6c3c;
    pub const MARBLE: u32 = 0xd8cacc;
    pub const MARBLE_D: u32 = 0xbaa8ae;
    pub const MARBLE_W: u32 = 0xe8dee0;
    pub const GROUT: u32 = 0x6a4a58;
    pub const GOLD: u32 = 0xe0b45a;
    pub const GOLD_D: u32 = 0xa88038;
    pub const BRONZE: u32 = 0x9a6a34;
    pub const BRONZE_D: u32 = 0x5a3c1c;
    pub const VERDIGRIS: u32 = 0x4a7a6a;
    pub const WATER: u32 = 0x1a2a3e;
    pub const SILK: u32 = 0xfaf2f6;
    pub const SILK_P: u32 = 0xf6c6dc;
    pub const VEIL: u32 = 0xffd0e4;
    pub const COVER: u32 = 0x8a0c2a;
    pub const WOOD: u32 = 0x3a1a14;
    pub const ROSE_GLASS: u32 = 0xff6aa8;
    pub const LANTERN: u32 = 0xffb0c8;
    pub const EMBER: u32 = 0xff7a2a;
    pub const SMOKE: u32 = 0xd8c8e8;
    pub const CANDLE: u32 = 0xfff0d8;
    pub const FLAME: u32 = 0xffc060;
    pub const FIREFLY: u32 = 0xd8ff8a;
    pub const MOONLIGHT: u32 = 0xa8c8c4;
}
use pal::*;

/// The hedges stand this tall.
const HEDGE_H: f32 = 2.3;
/// Shades: the court's marble and its gold and bronze (the vaults' own), cloth.
const SHADE_MARBLE: f32 = super::vault::SHADE_MARBLE;
const SHADE_GOLD: f32 = super::vault::SHADE_GOLD;
const SHADE_CLOTH: f32 = 8.0;

/// Moonlight on a crimson garden: the board's mist teal from above, a rose dark below, the
/// lanterns and the censers' embers the warm lights.
pub fn lust_atmosphere() -> Atmosphere {
    Atmosphere {
        background: 0x1a0a16,
        fog_near: 18.0,
        fog_far: 48.0,
        hemi_sky: lin(0x7e9c9a) * 0.5,
        hemi_ground: lin(0x4a2232) * 0.45,
        ambient: lin(0x4a3040) * 0.7,
        sun_dir: Vec3::new(-0.35, 1.0, 0.55).normalize(),
        sun: lin(MOONLIGHT) * 1.15,
        exposure: 1.2,
        night_sky: 1.0,
        bloom: 1.0,
        moon_view: Vec3::new(-0.4, 0.5, -1.0).normalize(),
        cloud_tint: lin(0x24101c),
    }
}

/// Its ground haze: density and falloff (a little rose mist about the lawn).
pub const HAZE: (f32, f32) = (0.018, 1.7);

/// What the view keeps of the Rose Bower for its moving parts: its lanterns, the fountain's bowl
/// (where the water leaves it), the censers' bowls, the candles by the bed, and the gate.
pub struct BowerView {
    lanterns: Vec<Vec3>,
    spout: Vec3,
    censers: Vec<Vec3>,
    candles: Vec<Vec3>,
    gate: Gate,
}

/// A hash of a tile and a salt, 0 to 1.
fn h(x: i32, y: i32, k: u32) -> f32 {
    hash(x as f64 * 1.37 + k as f64 * 17.1, y as f64 * 2.11 + k as f64 * 5.3) as f32
}

/// A box: centre, size, turn, colour, shade.
fn bx(out: &mut Vec<Instance>, c: Vec3, s: Vec3, rot: Quat, color: u32, shade: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(s, rot, c), lin(color), 1.0, Vec3::ZERO, shade));
}

/// A box glowing with its own colour (`glow` of it).
fn bxg(out: &mut Vec<Instance>, c: Vec3, s: Vec3, rot: Quat, color: u32, glow: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(s, rot, c), lin(color), 1.0, lin(color) * glow, SHADE_LIT));
}

/// A beam from `a` to `b`, `t` thick.
fn beam(out: &mut Vec<Instance>, a: Vec3, b: Vec3, t: f32, color: u32, shade: f32) {
    let d = b - a;
    let len = d.length().max(1e-4);
    out.push(Instance::new(
        Mat4::from_scale_rotation_translation(Vec3::new(t, len, t), Quat::from_rotation_arc(Vec3::Y, d / len), (a + b) * 0.5),
        lin(color),
        1.0,
        Vec3::ZERO,
        shade,
    ));
}

/// A rose: its outer petals and its heart, opening along `n`.
fn rose(out: &mut Vec<Instance>, at: Vec3, n: Vec3, size: f32, color: u32, seed: f32) {
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

/// The rose colours of a stretch of hedge: mostly crimson, some pink, some white.
fn roses_of(x: i32, y: i32) -> [u32; 3] {
    match ((h(x / 3, y / 3, 9) * 5.0) as i32, (h(x, y, 2) * 3.0) as i32) {
        (0, _) => [PINK, BLUSH, WHITE],
        (1, _) => [WHITE, BLUSH, WHITE],
        (_, 0) => [RED, CRIMSON, PINK],
        _ => [CRIMSON, RED, CRIMSON],
    }
}

/// A marble slab of path at (x, y): flags of rose marble set in dark grout, pale at their edges.
fn flags(out: &mut Vec<Instance>, x: i32, y: i32) {
    let c = Vec3::new(x as f32, 0.0, y as f32);
    bx(out, c + Vec3::new(0.0, -0.07, 0.0), Vec3::new(1.0, 0.12, 1.0), Quat::IDENTITY, GROUT, SHADE_MARBLE);
    for k in 0..4 {
        let (gx, gz) = ((k % 2) as f32 - 0.5, (k / 2) as f32 - 0.5);
        let j = h(x, y, 300 + k);
        let col = [MARBLE, MARBLE_D, MARBLE_W, MARBLE][(j * 4.0) as usize % 4];
        bx(
            out,
            c + Vec3::new(gx * 0.5, -0.005, gz * 0.5),
            Vec3::new(0.46, 0.03, 0.46),
            Quat::from_rotation_y((j - 0.5) * 0.04),
            col,
            SHADE_MARBLE,
        );
    }
}

impl WorldView {
    /// The Rose Bower's static scene.
    pub(super) fn build_lust_court(&mut self, grid: &Grid) {
        let c = Vec3::new(CENTRE.0 as f32, 0.0, CENTRE.1 as f32);
        let floor = |x: i32, y: i32| court::in_garden(x, y);
        let path = |x: i32, y: i32| {
            let r = Vec2::new((x - CENTRE.0) as f32, (y - CENTRE.1) as f32).length();
            (x - CENTRE.0).abs() <= 1
                || (y - CENTRE.1).abs() <= 1
                || (2.6..=4.2).contains(&r)
                || CENSERS.iter().any(|&(cx, cy)| (cx - x).abs() + (cy - y).abs() <= 1)
        };

        // ---- the floor: lawn and petals, paths of rose marble
        for y in 0..grid.height {
            for x in 0..grid.width {
                if !floor(x, y) && !(x == court::GATE.0 && y == court::GATE.1) {
                    continue;
                }
                let out = &mut self.static_cubes;
                let (xf, yf) = (x as f32, y as f32);
                if path(x, y) || (x == court::GATE.0 && y >= court::GATE.1 - 1) {
                    flags(out, x, y);
                } else {
                    let j = h(x, y, 1);
                    let col = if j > 0.6 { LAWN_L } else { LAWN };
                    out.push(Instance::new(
                        Mat4::from_scale_rotation_translation(Vec3::new(1.0, 0.12, 1.0), Quat::IDENTITY, Vec3::new(xf, -0.06, yf)),
                        lin(col),
                        1.0,
                        Vec3::ZERO,
                        SHADE_GRASS,
                    ));
                }
                // Fallen petals, thicker toward the arch and the fountain.
                let near = (Vec2::new(xf - c.x, yf - c.z).length() < 6.0) as u32 + (y < 9) as u32;
                for k in 0..(1 + near * 2) {
                    if h(x, y, 400 + k) < 0.55 {
                        continue;
                    }
                    let col = [CRIMSON, RED, PINK, WHITE][(h(x, y, 410 + k) * 4.0) as usize % 4];
                    let at = Vec3::new(xf + h(x, y, 420 + k) - 0.5, 0.012, yf + h(x, y, 430 + k) - 0.5);
                    bx(out, at, Vec3::new(0.09, 0.01, 0.06), Quat::from_rotation_y(h(x, y, 440 + k) * TAU), col, SHADE_LIT);
                }
            }
        }

        // ---- the walls: hedges of roses on the garden side, bramble over them
        for (x, y, t) in grid.cells() {
            if t != Tile::Rock || floor(x, y) || (x == court::GATE.0 && y == court::GATE.1) {
                continue;
            }
            let faces: Vec<Dir> = CARDINALS
                .iter()
                .copied()
                .filter(|d| {
                    let n = P::new(x, y).step(*d);
                    floor(n.x, n.y) && court::piece(n.x, n.y).is_none_or(|p| p == Piece::Hedge)
                })
                .collect();
            let near = !faces.is_empty() || [(-1, -1), (1, -1), (-1, 1), (1, 1)].iter().any(|&(dx, dy)| floor(x + dx, y + dy));
            if !near || (y == court::GATE.1 && (x - court::GATE.0).abs() <= 1) {
                continue;
            }
            let start = self.static_cubes.len();
            self.bower_hedge(x, y, &faces);
            self.close_tree(x, y, start, 0.72, HEDGE_H + 0.3);
        }

        // ---- what stands on the floor
        for (x, y, _) in grid.cells() {
            let Some(piece) = court::piece(x, y) else { continue };
            let base = Vec3::new(x as f32, 0.0, y as f32);
            let start = self.static_cubes.len();
            match piece {
                Piece::Hedge => {
                    self.bower_hedge(x, y, &CARDINALS);
                    self.close_tree(x, y, start, 0.72, HEDGE_H + 0.3);
                }
                Piece::Roses => {
                    self.rose_bed(base, x, y);
                    self.close_tree(x, y, start, 0.8, 1.6);
                }
                Piece::Censer => {
                    self.censer(base);
                    let len = self.static_cubes.len() - start;
                    self.rock_index.insert(tile_key(x, y), (start, len));
                }
                Piece::Arch => {
                    self.arch_post(base, x);
                    self.close_tree(x, y, start, 0.5, 3.6);
                }
                Piece::Fountain | Piece::Bed => {}
            }
        }
        let spout = self.fountain(c);
        let candles = self.bridal_bed();
        self.bridal_arch();

        // ---- lanterns of rose glass on gilt posts beside the paths
        let mut lanterns = Vec::new();
        for &(x, y) in &[(14, 21), (18, 21), (14, 11), (18, 11), (11, 14), (11, 18), (21, 14), (21, 18)] {
            if grid.is_walkable(x, y, false) {
                lanterns.push(self.lantern_post(Vec3::new(x as f32 + if x < CENTRE.0 { 0.3 } else { -0.3 }, 0.0, y as f32)));
            }
        }

        // ---- the gate: Lust's portal in the south hedge, looking into the garden
        let gate = Gate { tile: P::new(court::GATE.0, court::GATE.1), facing: Dir::Up, sin: crate::logic::sins::Sin::Lust };
        self.build_court_portal(&gate);
        let censers = CENSERS.iter().map(|&(x, y)| Vec3::new(x as f32, 1.42, y as f32)).collect();
        self.bower = Some(BowerView { lanterns, spout, censers, candles, gate });
    }

    /// The cubes laid since `start` stand on rock tile (x, y), and fade out of the camera's way.
    fn close_tree(&mut self, x: i32, y: i32, start: usize, radius: f32, height: f32) {
        let len = self.static_cubes.len() - start;
        self.rock_index.insert(tile_key(x, y), (start, len));
        let shades: Vec<f32> = self.static_cubes[start..start + len].iter().map(|i| i.emissive[3]).collect();
        self.trees.push(Tree {
            range: (start, len),
            base: Vec3::new(x as f32, 0.0, y as f32),
            radius,
            height,
            fade: 1.0,
            alive: true,
            shades: Some(shades),
        });
    }

    /// A block of rose hedge on (x, y): a dark body, leaf clumps thick with roses on each side that
    /// faces the garden (fullest toward the camera), a crown of leaves and roses with black thorn
    /// canes arching over it.
    fn bower_hedge(&mut self, x: i32, y: i32, faces: &[Dir]) {
        let out = &mut self.static_cubes;
        let c = Vec3::new(x as f32, 0.0, y as f32);
        let top = HEDGE_H - 0.1 * h(x, y, 3);
        let roses = roses_of(x, y);
        bx(out, c + Vec3::new(0.0, (top - 0.12) / 2.0, 0.0), Vec3::new(0.98, top - 0.12, 0.98), Quat::IDENTITY, LEAF_DEEP, SHADE_LIT);
        // The crown: leaf clumps, sprigs, roses on top.
        for k in 0..16u32 {
            let (gx, gz) = ((k % 4) as f32 - 1.5, (k / 4) as f32 - 1.5);
            let j = h(x, y, 10 + k);
            let size = Vec3::new(0.27 + j * 0.09, 0.14 + j * 0.12, 0.27 + h(x, y, 20 + k) * 0.09);
            let col = [LEAF, LEAF_L, LEAF_D, LEAF_MOON, LEAF, LEAF_L][((j * 13.0) as usize + k as usize) % 6];
            let at = c + Vec3::new(gx * 0.235 + (j - 0.5) * 0.06, top - 0.08 + size.y * 0.4, gz * 0.235 + (h(x, y, 70 + k) - 0.5) * 0.06);
            bx(out, at, size, Quat::from_rotation_y((j - 0.5) * 1.0), col, SHADE_LIT);
        }
        for k in 0..(1 + (h(x, y, 30) * 4.0) as usize) {
            let (a, r) = (h(x, y, 31 + k as u32) * TAU, 0.12 + h(x, y, 41 + k as u32) * 0.3);
            rose(out, c + Vec3::new(a.cos() * r, top + 0.14, a.sin() * r), Vec3::Y, 0.12, roses[k % 3], a);
        }
        // Black canes arching over the top, thorny.
        for k in 0..2 {
            let j = h(x, y, 200 + k);
            let a = c + Vec3::new((j - 0.5) * 0.8, top, (h(x, y, 210 + k) - 0.5) * 0.8);
            let b = c + Vec3::new((h(x, y, 250 + k) - 0.5) * 0.9, top, (h(x, y, 260 + k) - 0.5) * 0.9);
            let mid = (a + b) * 0.5 + Vec3::new(0.0, 0.25 + j * 0.2, 0.0);
            beam(out, a, mid, 0.05, CANE, SHADE_LIT);
            beam(out, mid, b, 0.045, BRAMBLE, SHADE_LIT);
            let tip = mid + Vec3::new((h(x, y, 220 + k) - 0.5) * 0.3, 0.14, (h(x, y, 230 + k) - 0.5) * 0.3);
            beam(out, mid, tip, 0.025, THORN, SHADE_LIT);
        }
        for &d in faces {
            let (dx, dz) = d.vec();
            let n = Vec3::new(dx as f32, 0.0, dz as f32);
            let side = Vec3::new(dz as f32, 0.0, dx as f32).abs();
            let salt = 50 + d as u32 * 13;
            let rich = d == Dir::Down;
            let (cols, rows) = if rich { (4, 5) } else { (3, 4) };
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
            let count = if rich { 6 + (h(x, y, salt + 40) * 4.0) as u32 } else { 2 + (h(x, y, salt + 40) * 3.0) as u32 };
            for k in 0..count {
                let j = h(x, y, salt + 60 + k);
                let up = 0.35 + h(x, y, salt + 80 + k) * (top - 0.6);
                rose(
                    out,
                    c + n * 0.57 + side * ((j - 0.5) * 0.86) + Vec3::new(0.0, up, 0.0),
                    n,
                    0.1 + j * 0.05,
                    roses[(k as usize) % 3],
                    j,
                );
            }
            // Thorns standing out between the leaves.
            for k in 0..3 {
                let j = h(x, y, salt + 140 + k);
                let at = c + n * 0.6 + side * (j - 0.5) * 0.8 + Vec3::new(0.0, 0.3 + h(x, y, salt + 150 + k) * (top - 0.5), 0.0);
                beam(out, at - n * 0.08, at + n * 0.1 + Vec3::new(0.0, 0.04, 0.0), 0.02, THORN, SHADE_LIT);
            }
            if rich && (x + y) % 3 == 0 {
                // A gilded trellis rail along its foot.
                bx(
                    out,
                    c + n * 0.55 + Vec3::new(0.0, 0.16, 0.0),
                    side * 1.0 + Vec3::new(0.0, 0.03, 0.0) + n.abs() * 0.03,
                    Quat::IDENTITY,
                    GOLD_D,
                    SHADE_GOLD,
                );
            }
        }
    }

    /// A bed of roses near a corner: a low marble kerb, a mound of dark leaf, roses all over it.
    fn rose_bed(&mut self, base: Vec3, x: i32, y: i32) {
        let out = &mut self.static_cubes;
        for k in 0..12 {
            let a = k as f32 / 12.0 * TAU;
            bx(
                out,
                base + Vec3::new(a.cos() * 0.55, 0.08, a.sin() * 0.55),
                Vec3::new(0.32, 0.16, 0.12),
                Quat::from_rotation_y(-a + PI / 2.0),
                MARBLE_D,
                SHADE_MARBLE,
            );
        }
        for k in 0..14u32 {
            let j = h(x, y, 500 + k);
            let a = j * TAU + k as f32;
            let r = 0.15 + h(x, y, 510 + k) * 0.32;
            let up = 0.25 + (1.0 - r / 0.5) * 0.7 * h(x, y, 520 + k);
            bx(
                out,
                base + Vec3::new(a.cos() * r, up, a.sin() * r),
                Vec3::new(0.34, 0.3, 0.34),
                Quat::from_rotation_y(a),
                [LEAF, LEAF_D, LEAF_L, LEAF_MOON][k as usize % 4],
                SHADE_LIT,
            );
        }
        let roses = roses_of(x + 7, y + 3);
        for k in 0..16u32 {
            let a = h(x, y, 530 + k) * TAU;
            let r = h(x, y, 540 + k) * 0.48;
            let up = 0.45 + (1.0 - r / 0.5) * 0.6;
            let n = Vec3::new(a.cos() * r, 0.6, a.sin() * r).normalize();
            rose(out, base + Vec3::new(a.cos() * r, up, a.sin() * r), n, 0.11 + h(x, y, 550 + k) * 0.05, roses[k as usize % 3], a);
        }
    }

    /// A bronze censer on a marble plinth, on its round pad of marble: a fluted plinth, a tripod of
    /// lion's legs, a deep bowl, a pierced dome of a lid with a finial. (Its embers and smoke are
    /// drawn each frame.)
    fn censer(&mut self, base: Vec3) {
        let out = &mut self.static_cubes;
        for k in 0..10 {
            let a = k as f32 / 10.0 * TAU;
            bx(
                out,
                base + Vec3::new(a.cos() * 0.62, 0.03, a.sin() * 0.62),
                Vec3::new(0.42, 0.06, 0.2),
                Quat::from_rotation_y(-a + PI / 2.0),
                MARBLE_W,
                SHADE_MARBLE,
            );
        }
        bx(out, base + Vec3::new(0.0, 0.08, 0.0), Vec3::new(0.82, 0.16, 0.82), Quat::IDENTITY, MARBLE_D, SHADE_MARBLE);
        bx(out, base + Vec3::new(0.0, 0.42, 0.0), Vec3::new(0.56, 0.56, 0.56), Quat::IDENTITY, MARBLE, SHADE_MARBLE);
        for k in 0..8 {
            let a = k as f32 / 8.0 * TAU;
            bx(
                out,
                base + Vec3::new(a.cos() * 0.29, 0.42, a.sin() * 0.29),
                Vec3::new(0.05, 0.5, 0.05),
                Quat::IDENTITY,
                MARBLE_W,
                SHADE_MARBLE,
            );
        }
        bx(out, base + Vec3::new(0.0, 0.74, 0.0), Vec3::new(0.7, 0.08, 0.7), Quat::IDENTITY, MARBLE_D, SHADE_MARBLE);
        bx(out, base + Vec3::new(0.0, 0.79, 0.0), Vec3::new(0.62, 0.03, 0.62), Quat::from_rotation_y(PI / 4.0), GOLD_D, SHADE_GOLD);
        let top = base + Vec3::new(0.0, 0.8, 0.0);
        for k in 0..3 {
            let a = k as f32 / 3.0 * TAU + 0.5;
            let foot = top + Vec3::new(a.cos() * 0.24, 0.02, a.sin() * 0.24);
            let knee = top + Vec3::new(a.cos() * 0.2, 0.16, a.sin() * 0.2);
            beam(out, foot, knee, 0.05, BRONZE_D, SHADE_GOLD);
            beam(out, knee, top + Vec3::new(a.cos() * 0.12, 0.26, a.sin() * 0.12), 0.05, BRONZE, SHADE_GOLD);
            bx(out, foot, Vec3::new(0.08, 0.04, 0.1), Quat::from_rotation_y(-a), BRONZE_D, SHADE_GOLD);
        }
        // The bowl: rounded (each slice an octagon), widening up, a band of verdigris round it.
        for (k, r) in [0.13f32, 0.2, 0.25, 0.28, 0.3].into_iter().enumerate() {
            let y = 0.27 + k as f32 * 0.055;
            let col = if k == 2 { VERDIGRIS } else { BRONZE };
            for turn in [0.0, PI / 4.0] {
                bx(out, top + Vec3::new(0.0, y, 0.0), Vec3::new(r * 1.85, 0.06, r * 1.85), Quat::from_rotation_y(turn), col, SHADE_GOLD);
            }
        }
        bx(out, top + Vec3::new(0.0, 0.52, 0.0), Vec3::new(0.64, 0.03, 0.64), Quat::from_rotation_y(PI / 8.0), GOLD_D, SHADE_GOLD);
        // The pierced lid: a dome, the smoke's holes dark in it, a rosebud finial.
        for (k, r) in [0.28f32, 0.25, 0.21, 0.15, 0.09].into_iter().enumerate() {
            let y = 0.56 + k as f32 * 0.045;
            for turn in [0.0, PI / 4.0] {
                bx(
                    out,
                    top + Vec3::new(0.0, y, 0.0),
                    Vec3::new(r * 1.85, 0.05, r * 1.85),
                    Quat::from_rotation_y(turn),
                    if k % 2 == 0 { BRONZE } else { BRONZE_D },
                    SHADE_GOLD,
                );
            }
        }
        for j in 0..8 {
            let a = j as f32 / 8.0 * TAU;
            bx(
                out,
                top + Vec3::new(a.cos() * 0.25, 0.6, a.sin() * 0.25),
                Vec3::new(0.05, 0.035, 0.05),
                Quat::from_rotation_y(-a),
                0x120a08,
                SHADE_LIT,
            );
        }
        bx(out, top + Vec3::new(0.0, 0.8, 0.0), Vec3::new(0.08, 0.1, 0.08), Quat::from_rotation_y(PI / 4.0), GOLD, SHADE_GOLD);
        bx(out, top + Vec3::new(0.0, 0.87, 0.0), Vec3::new(0.05, 0.07, 0.05), Quat::IDENTITY, GOLD_D, SHADE_GOLD);
    }

    /// One post of the bridal arch: a marble column wound with climbing roses.
    fn arch_post(&mut self, base: Vec3, x: i32) {
        let out = &mut self.static_cubes;
        bx(out, base + Vec3::new(0.0, 0.12, 0.0), Vec3::new(0.8, 0.24, 0.8), Quat::IDENTITY, MARBLE_D, SHADE_MARBLE);
        for k in 0..10 {
            let y = 0.3 + k as f32 * 0.3;
            bx(
                out,
                base + Vec3::new(0.0, y + 0.15, 0.0),
                Vec3::new(0.46, 0.3, 0.46),
                Quat::from_rotation_y(k as f32 * PI / 10.0),
                if k % 3 == 2 { MARBLE_W } else { MARBLE },
                SHADE_MARBLE,
            );
        }
        bx(out, base + Vec3::new(0.0, 3.36, 0.0), Vec3::new(0.66, 0.14, 0.66), Quat::IDENTITY, GOLD, SHADE_GOLD);
        let roses = [CRIMSON, RED, PINK, WHITE];
        for k in 0..18 {
            let t = k as f32 / 18.0;
            let a = t * 3.0 * TAU + x as f32;
            let at = base + Vec3::new(a.cos() * 0.3, 0.3 + t * 3.0, a.sin() * 0.3);
            bx(out, at, Vec3::new(0.2, 0.16, 0.2), Quat::from_rotation_y(a), [LEAF, LEAF_L, LEAF_D][k % 3], SHADE_LIT);
            if k % 2 == 0 {
                rose(
                    out,
                    at + Vec3::new(a.cos() * 0.12, 0.04, a.sin() * 0.12),
                    Vec3::new(a.cos(), 0.3, a.sin()),
                    0.11,
                    roses[(k / 2 + x as usize) % 4],
                    a,
                );
            }
        }
    }

    /// The gilt curve of the bridal arch over the way between its posts, thick with roses, veils
    /// hanging from it, a gold heart at its crown.
    fn bridal_arch(&mut self) {
        let out = &mut self.static_cubes;
        let (a, b) = (Vec3::new(ARCH[0].0 as f32, 3.4, ARCH[0].1 as f32), Vec3::new(ARCH[1].0 as f32, 3.4, ARCH[1].1 as f32));
        let mid = (a + b) * 0.5;
        let span = (b.x - a.x) / 2.0;
        let rise = 1.1;
        let n = 16;
        let point = |t: f32| mid + Vec3::new(-span * (t * PI).cos(), rise * (t * PI).sin(), 0.0);
        let roses = [CRIMSON, RED, PINK, WHITE];
        for k in 0..n {
            let (p, q) = (point(k as f32 / n as f32), point((k + 1) as f32 / n as f32));
            beam(out, p, q, 0.12, GOLD, SHADE_GOLD);
            let m = (p + q) * 0.5;
            bx(
                out,
                m + Vec3::new(0.0, 0.08, 0.0),
                Vec3::new(0.3, 0.2, 0.3),
                Quat::from_rotation_y(k as f32),
                [LEAF, LEAF_L, LEAF_MOON][k % 3],
                SHADE_LIT,
            );
            rose(out, m + Vec3::new(0.0, 0.2, 0.06), Vec3::new(0.0, 0.6, 1.0), 0.14, roses[k % 4], k as f32);
            if k % 3 == 1 {
                rose(out, m + Vec3::new(0.0, 0.0, 0.18), Vec3::Z, 0.12, roses[(k + 2) % 4], k as f32 * 1.7);
            }
        }
        // Veils draped in swags along the curve, and falling in long tails at either end.
        for k in 0..5 {
            let (t0, t1) = (k as f32 / 5.0, (k + 1) as f32 / 5.0);
            let (a, b) = (point(t0) + Vec3::new(0.0, -0.08, 0.16), point(t1) + Vec3::new(0.0, -0.08, 0.16));
            let n = 8;
            for j in 0..n {
                let (u0, u1) = (j as f32 / n as f32, (j + 1) as f32 / n as f32);
                let sag = |u: f32| a.lerp(b, u) - Vec3::new(0.0, 0.32 * (u * PI).sin(), 0.0);
                let (p, q) = (sag(u0), sag(u1));
                let d = q - p;
                out.push(Instance::new(
                    Mat4::from_scale_rotation_translation(
                        Vec3::new(d.length() + 0.02, 0.07, 0.02),
                        Quat::from_rotation_z(d.y.atan2(d.x)),
                        (p + q) * 0.5,
                    ),
                    lin(if k % 2 == 0 { VEIL } else { SILK_P }),
                    1.0,
                    lin(VEIL) * 0.08,
                    SHADE_CLOTH,
                ));
            }
        }
        for t in [0.03f32, 0.97] {
            let top = point(t) + Vec3::new(0.0, -0.1, 0.2);
            for j in 0..10 {
                let y = j as f32 * 0.24;
                let sway = (j as f32 * 0.7).sin() * 0.04;
                out.push(Instance::new(
                    Mat4::from_scale_rotation_translation(
                        Vec3::new(0.16 - j as f32 * 0.006, 0.25, 0.02),
                        Quat::from_rotation_z(sway),
                        top - Vec3::new(0.0, y + 0.12, 0.0),
                    ),
                    lin(if j % 3 == 0 { SILK_P } else { VEIL }),
                    1.0,
                    lin(VEIL) * 0.08,
                    SHADE_CLOTH,
                ));
            }
        }
        // The gold heart at its crown.
        let crown = point(0.5) + Vec3::new(0.0, 0.35, 0.1);
        for s in [-1.0f32, 1.0] {
            bx(out, crown + Vec3::new(s * 0.13, 0.08, 0.0), Vec3::new(0.24, 0.24, 0.08), Quat::IDENTITY, GOLD, SHADE_GOLD);
        }
        bx(out, crown - Vec3::new(0.0, 0.06, 0.0), Vec3::new(0.26, 0.26, 0.08), Quat::from_rotation_z(PI / 4.0), GOLD, SHADE_GOLD);
        bx(out, crown + Vec3::new(0.0, 0.0, 0.05), Vec3::new(0.12, 0.12, 0.04), Quat::from_rotation_z(PI / 4.0), 0xd01a40, SHADE_GOLD);
    }

    /// The bridal bed behind the arch, against the north hedge: a gilt frame, white sheets, a
    /// crimson coverlet strewn with petals, pillows, four posts and a canopy draped in veils.
    /// Returns where its candles burn.
    fn bridal_bed(&mut self) -> Vec<Vec3> {
        let out = &mut self.static_cubes;
        let (x0, x1, y) = BED;
        let mid = Vec3::new((x0 + x1) as f32 / 2.0, 0.0, y as f32 - 0.1);
        let (w, d) = ((x1 - x0 + 1) as f32 - 0.2, 0.95);
        bx(out, mid + Vec3::new(0.0, 0.2, 0.0), Vec3::new(w, 0.4, d), Quat::IDENTITY, WOOD, SHADE_LIT);
        bx(out, mid + Vec3::new(0.0, 0.42, 0.0), Vec3::new(w + 0.04, 0.06, d + 0.04), Quat::IDENTITY, GOLD_D, SHADE_GOLD);
        bx(out, mid + Vec3::new(0.0, 0.56, 0.0), Vec3::new(w - 0.1, 0.22, d - 0.08), Quat::IDENTITY, SILK, SHADE_CLOTH);
        bx(out, mid + Vec3::new(0.0, 0.68, 0.12), Vec3::new(w - 0.06, 0.06, d * 0.7), Quat::IDENTITY, COVER, SHADE_CLOTH);
        bx(out, mid + Vec3::new(0.0, 0.62, 0.48), Vec3::new(w - 0.06, 0.18, 0.04), Quat::IDENTITY, COVER, SHADE_CLOTH);
        for k in 0..3 {
            bx(
                out,
                mid + Vec3::new((k as f32 - 1.0) * 0.8, 0.74, -0.32),
                Vec3::new(0.6, 0.14, 0.24),
                Quat::from_rotation_y((k as f32 - 1.0) * 0.1),
                if k == 1 { SILK_P } else { SILK },
                SHADE_CLOTH,
            );
        }
        for k in 0..24u32 {
            let at = mid + Vec3::new((h(k as i32, 7, 600) - 0.5) * w * 0.9, 0.72, (h(k as i32, 7, 610) - 0.1) * 0.5);
            bx(out, at, Vec3::new(0.08, 0.01, 0.06), Quat::from_rotation_y(k as f32), [CRIMSON, RED, WHITE][k as usize % 3], SHADE_LIT);
        }
        // The headboard: gilt, a heart in it.
        bx(out, mid + Vec3::new(0.0, 0.95, -0.46), Vec3::new(w, 1.1, 0.08), Quat::IDENTITY, GOLD_D, SHADE_GOLD);
        bx(out, mid + Vec3::new(0.0, 1.0, -0.41), Vec3::new(w - 0.3, 0.8, 0.03), Quat::IDENTITY, COVER, SHADE_CLOTH);
        // Four posts and the canopy, its drapes hanging and tied back at the foot.
        let top = 2.7;
        for (sx, sz) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let p = mid + Vec3::new(sx * w / 2.0, 0.0, sz * d / 2.0);
            bx(out, p + Vec3::new(0.0, top / 2.0, 0.0), Vec3::new(0.08, top, 0.08), Quat::IDENTITY, GOLD, SHADE_GOLD);
            bx(out, p + Vec3::new(0.0, top + 0.06, 0.0), Vec3::new(0.14, 0.12, 0.14), Quat::from_rotation_y(PI / 4.0), GOLD, SHADE_GOLD);
            if sz > 0.0 {
                for j in 0..8 {
                    let y = top - 0.15 - j as f32 * 0.3;
                    let tie = if j >= 4 { 0.2 } else { 0.08 + j as f32 * 0.03 };
                    bx(
                        out,
                        p + Vec3::new(-sx * tie, y, 0.02),
                        Vec3::new(0.2 - (j >= 4) as i32 as f32 * 0.08, 0.32, 0.04),
                        Quat::IDENTITY,
                        if j % 2 == 0 { VEIL } else { SILK_P },
                        SHADE_CLOTH,
                    );
                }
                bx(out, p + Vec3::new(-sx * 0.2, 1.4, 0.03), Vec3::new(0.24, 0.05, 0.06), Quat::IDENTITY, GOLD, SHADE_GOLD);
            }
        }
        bx(out, mid + Vec3::new(0.0, top + 0.02, 0.0), Vec3::new(w + 0.2, 0.08, d + 0.2), Quat::IDENTITY, GOLD_D, SHADE_GOLD);
        bx(out, mid + Vec3::new(0.0, top - 0.12, 0.0), Vec3::new(w, 0.22, d), Quat::IDENTITY, COVER, SHADE_CLOTH);
        for k in 0..9 {
            let x = -w / 2.0 + (k as f32 + 0.5) * w / 9.0;
            bx(
                out,
                mid + Vec3::new(x, top - 0.3, d / 2.0 + 0.02),
                Vec3::new(w / 9.0 - 0.02, 0.16 + (k % 2) as f32 * 0.08, 0.03),
                Quat::IDENTITY,
                GOLD_D,
                SHADE_GOLD,
            );
        }
        // Candles on tall gilt stands either side of the foot.
        let mut candles = Vec::new();
        for s in [-1.0f32, 1.0] {
            let p = mid + Vec3::new(s * (w / 2.0 + 0.45), 0.0, 0.7);
            bx(out, p + Vec3::new(0.0, 0.05, 0.0), Vec3::new(0.26, 0.1, 0.26), Quat::from_rotation_y(PI / 4.0), GOLD_D, SHADE_GOLD);
            bx(out, p + Vec3::new(0.0, 0.65, 0.0), Vec3::new(0.05, 1.2, 0.05), Quat::IDENTITY, GOLD, SHADE_GOLD);
            for k in 0..3 {
                let a = k as f32 / 3.0 * TAU;
                let q = p + Vec3::new(a.cos() * 0.16, 1.25, a.sin() * 0.16);
                beam(out, p + Vec3::new(0.0, 1.15, 0.0), q, 0.03, GOLD, SHADE_GOLD);
                bx(out, q + Vec3::new(0.0, 0.1, 0.0), Vec3::new(0.06, 0.18, 0.06), Quat::IDENTITY, CANDLE, SHADE_LIT);
                candles.push(q + Vec3::new(0.0, 0.24, 0.0));
            }
        }
        candles
    }

    /// The fountain in the middle: a round marble basin with a moulded rim, water in it with petals
    /// floating, a fluted pedestal lifting a gilt bowl, a golden rose over the bowl. Returns the
    /// bowl's lip, where the water leaves it.
    fn fountain(&mut self, c: Vec3) -> Vec3 {
        let out = &mut self.static_cubes;
        let r = FOUNTAIN as f32 + 0.35;
        // The basin's floor and its water.
        bx(out, c + Vec3::new(0.0, -0.02, 0.0), Vec3::new(r * 2.0, 0.08, r * 2.0), Quat::IDENTITY, MARBLE_D, SHADE_MARBLE);
        out.push(Instance::new(
            Mat4::from_scale_rotation_translation(Vec3::new(r * 1.9, 0.04, r * 1.9), Quat::IDENTITY, c + Vec3::new(0.0, 0.32, 0.0)),
            lin(WATER),
            1.0,
            Vec3::ZERO,
            SHADE_WATER,
        ));
        // The rim: a ring of moulded marble blocks, a lip on top.
        let n = 32;
        for k in 0..n {
            let a = k as f32 / n as f32 * TAU;
            let p = c + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
            let rot = Quat::from_rotation_y(-a + PI / 2.0);
            bx(out, p + Vec3::new(0.0, 0.22, 0.0), Vec3::new(TAU * r / n as f32 + 0.04, 0.44, 0.3), rot, MARBLE, SHADE_MARBLE);
            bx(out, p + Vec3::new(0.0, 0.48, 0.0), Vec3::new(TAU * r / n as f32 + 0.06, 0.08, 0.42), rot, MARBLE_W, SHADE_MARBLE);
            if k % 4 == 0 {
                bx(
                    out,
                    p + Vec3::new(0.0, 0.24, 0.0) + Vec3::new(a.cos(), 0.0, a.sin()) * 0.16,
                    Vec3::new(0.1, 0.3, 0.04),
                    rot,
                    GOLD_D,
                    SHADE_GOLD,
                );
            }
        }
        // Petals floating on the water.
        for k in 0..30u32 {
            let a = h(k as i32, 3, 700) * TAU;
            let rr = 0.9 + h(k as i32, 3, 710) * (r - 1.2);
            bx(
                out,
                c + Vec3::new(a.cos() * rr, 0.35, a.sin() * rr),
                Vec3::new(0.09, 0.01, 0.06),
                Quat::from_rotation_y(a * 3.0),
                [CRIMSON, RED, PINK, WHITE][k as usize % 4],
                SHADE_LIT,
            );
        }
        // The pedestal, fluted, and the bowl on it.
        for k in 0..6 {
            let y = 0.3 + k as f32 * 0.2;
            let w = 0.5 - (k as f32 - 2.5).abs() * 0.04;
            bx(
                out,
                c + Vec3::new(0.0, y + 0.1, 0.0),
                Vec3::new(w, 0.2, w),
                Quat::from_rotation_y(k as f32 * PI / 12.0),
                if k % 2 == 0 { MARBLE } else { MARBLE_W },
                SHADE_MARBLE,
            );
        }
        for (k, (rr, y)) in [(0.3f32, 1.55f32), (0.55, 1.62), (0.8, 1.7), (0.95, 1.78)].into_iter().enumerate() {
            let m = 16;
            for j in 0..m {
                let a = j as f32 / m as f32 * TAU;
                bx(
                    out,
                    c + Vec3::new(a.cos() * rr, y, a.sin() * rr),
                    Vec3::new(TAU * rr / m as f32 + 0.06, 0.1, 0.2),
                    Quat::from_rotation_y(-a + PI / 2.0),
                    if k == 3 { GOLD } else { GOLD_D },
                    SHADE_GOLD,
                );
            }
        }
        out.push(Instance::new(
            Mat4::from_scale_rotation_translation(Vec3::new(1.6, 0.03, 1.6), Quat::IDENTITY, c + Vec3::new(0.0, 1.78, 0.0)),
            lin(WATER),
            1.0,
            Vec3::ZERO,
            SHADE_WATER,
        ));
        // The golden rose over the bowl, on its stem.
        beam(out, c + Vec3::new(0.0, 1.75, 0.0), c + Vec3::new(0.0, 2.45, 0.0), 0.06, GOLD_D, SHADE_GOLD);
        for k in 0..2 {
            let s = if k == 0 { 1.0 } else { -1.0 };
            bx(out, c + Vec3::new(s * 0.12, 2.08, 0.0), Vec3::new(0.2, 0.03, 0.09), Quat::from_rotation_z(s * 0.4), GOLD_D, SHADE_GOLD);
        }
        let bloom = c + Vec3::new(0.0, 2.55, 0.0);
        for k in 0..5 {
            let a = k as f32 / 5.0 * TAU;
            bx(
                out,
                bloom + Vec3::new(a.cos() * 0.1, 0.0, a.sin() * 0.1),
                Vec3::new(0.18, 0.2, 0.06),
                Quat::from_rotation_y(-a + PI / 2.0) * Quat::from_rotation_x(-0.3),
                GOLD,
                SHADE_GOLD,
            );
        }
        bx(out, bloom + Vec3::new(0.0, 0.06, 0.0), Vec3::new(0.14, 0.16, 0.14), Quat::from_rotation_y(0.5), 0xfbe08a, SHADE_GOLD);
        c + Vec3::new(0.0, 1.8, 0.0)
    }

    /// A lantern of rose glass on a gilt post. Returns where it glows.
    fn lantern_post(&mut self, base: Vec3) -> Vec3 {
        let out = &mut self.static_cubes;
        bx(out, base + Vec3::new(0.0, 0.06, 0.0), Vec3::new(0.22, 0.12, 0.22), Quat::from_rotation_y(PI / 4.0), GOLD_D, SHADE_GOLD);
        bx(out, base + Vec3::new(0.0, 0.85, 0.0), Vec3::new(0.05, 1.6, 0.05), Quat::IDENTITY, GOLD, SHADE_GOLD);
        let at = base + Vec3::new(0.0, 1.78, 0.0);
        bx(out, at + Vec3::new(0.0, 0.16, 0.0), Vec3::new(0.2, 0.04, 0.2), Quat::from_rotation_y(PI / 4.0), GOLD, SHADE_GOLD);
        bx(out, at + Vec3::new(0.0, 0.22, 0.0), Vec3::new(0.06, 0.08, 0.06), Quat::IDENTITY, GOLD_D, SHADE_GOLD);
        bxg(out, at, Vec3::new(0.15, 0.24, 0.15), Quat::from_rotation_y(PI / 4.0), ROSE_GLASS, 1.4);
        bx(out, at - Vec3::new(0.0, 0.15, 0.0), Vec3::new(0.12, 0.04, 0.12), Quat::IDENTITY, GOLD_D, SHADE_GOLD);
        at
    }

    /// Each frame: the lanterns, the fountain's water, the censers (embers and smoke while they
    /// smoke), the bed's candles, petals drifting and fireflies, the prince's own light, the gate.
    pub(super) fn emit_lust_court(&self, s: &SceneState, time: f32, hero: Vec3, f: &mut Frame) {
        let Some(bv) = &self.bower else { return };
        f.bounds = Vec4::new(-0.5, -0.5, s.grid.width as f32 - 0.5, s.grid.height as f32 - 0.5);
        // The lanterns: rose glass glowing, the nearest lighting the paths.
        for (i, &p) in bv.lanterns.iter().enumerate() {
            glow_orb(f, p, 0.5, lin(LANTERN), 0.18 + 0.03 * (time * 3.0 + i as f32).sin());
        }
        for (i, p) in nearest(&bv.lanterns, hero, 4).into_iter().enumerate() {
            let flick = 3.4 + (time * 4.0 + i as f32 * 1.7).sin() * 0.3;
            f.lights.push(Light { pos: p, color: lin(LANTERN) * flick, distance: 6.5 });
        }
        // The fountain: water arching from the bowl's lip down into the basin, the splash rings.
        let c = Vec3::new(CENTRE.0 as f32, 0.0, CENTRE.1 as f32);
        for k in 0..24 {
            let a = k as f32 / 24.0 * TAU;
            let u = (time * 0.9 + k as f32 * 0.37).fract();
            let out = Vec3::new(a.cos(), 0.0, a.sin());
            let p = bv.spout + out * (0.95 + u * 0.9) + Vec3::new(0.0, 0.1 - u * u * 1.45, 0.0);
            let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.05 - u * 0.02));
            f.additive.push(Instance::new(m, lin(0xc8e0ff) * 0.9, 0.55, Vec3::ZERO, SHADE_UNLIT));
        }
        for k in 0..3 {
            let u = (time * 0.5 + k as f32 / 3.0).fract();
            let r = 1.9 + u * 0.6;
            let m = Mat4::from_translation(c + Vec3::new(0.0, 0.35, 0.0))
                * Mat4::from_rotation_x(-PI / 2.0)
                * Mat4::from_scale(Vec3::new(r, r, 1.0));
            f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, lin(0xd8e8ff), 0.25 * (1.0 - u), Vec3::ZERO, SHADE_UNLIT));
        }
        f.lights.push(Light { pos: c + Vec3::new(0.0, 0.8, 0.0), color: lin(0x8ab0e0) * 2.2, distance: 5.0 });
        f.lights.push(Light { pos: c + Vec3::new(0.0, 3.0, 0.0), color: lin(0xfbe08a) * 1.6, distance: 4.0 });
        // The censers: smoking ones (their puffs on the scene) glow and send up incense.
        for (i, &p) in bv.censers.iter().enumerate() {
            let lit =
                s.hazards.iter().any(|hz| hz.radius == 0.0 && (hz.pos.x as f32 - p.x).abs() < 0.2 && (hz.pos.y as f32 - p.z).abs() < 0.2);
            if !lit {
                // Cold: a faint glint on the bronze, so it can be found in the dark.
                glow_orb(f, p + Vec3::new(0.0, 0.1, 0.0), 0.35, lin(0xd8a060), 0.05);
                continue;
            }
            let fi = i as f32;
            glow_orb(f, p + Vec3::new(0.0, 0.05, 0.0), 0.7, lin(EMBER), 0.4 + 0.08 * (time * 9.0 + fi).sin());
            for k in 0..5 {
                let a = k as f32 * 1.3 + time * 2.0;
                let m = Mat4::from_translation(p + Vec3::new(a.cos() * 0.12, 0.02, a.sin() * 0.12)) * Mat4::from_scale(Vec3::splat(0.06));
                f.additive.push(Instance::new(m, lin(EMBER) * 2.4, 0.9, Vec3::ZERO, SHADE_UNLIT));
            }
            for k in 0..12 {
                let u = (time * 0.35 + k as f32 / 12.0 + fi * 0.21).fract();
                let sway = Vec3::new(
                    (time * 0.7 + k as f32 * 1.7 + fi).sin() * (0.08 + u * 0.45),
                    0.0,
                    (time * 0.5 + k as f32 * 2.3).cos() * (0.06 + u * 0.35),
                );
                let at = p + Vec3::new(0.0, 0.2 + u * 3.0, 0.0) + sway;
                let size = 0.08 + u * 0.3;
                let m = Mat4::from_translation(at) * Mat4::from_scale(Vec3::splat(size));
                f.transparent[MeshKind::Sphere as usize].push(Instance::new(
                    m,
                    lin(SMOKE),
                    0.16 * (1.0 - u) * (u * 5.0).min(1.0),
                    lin(0x4a3040) * 0.2,
                    SHADE_UNLIT,
                ));
            }
            let flick = 4.5 + (time * 11.0 + fi).sin() * 0.6;
            f.lights.push(Light { pos: p + Vec3::new(0.0, 0.5, 0.0), color: lin(EMBER) * flick, distance: 5.5 });
        }
        // The candles by the bed.
        for (i, &p) in bv.candles.iter().enumerate() {
            let flick = (time * 13.0 + i as f32 * 2.1).sin() * 0.1;
            let m = Mat4::from_translation(p + Vec3::new(0.0, 0.03, 0.0)) * Mat4::from_scale(Vec3::new(0.035, 0.08 + flick * 0.2, 0.035));
            f.additive.push(Instance::new(m, lin(FLAME) * 2.6, 0.95, Vec3::ZERO, SHADE_UNLIT));
            glow_orb(f, p, 0.25, lin(FLAME), 0.2);
        }
        if let Some(&p) = bv.candles.first() {
            let mid = (p + *bv.candles.last().unwrap_or(&p)) * 0.5;
            f.lights.push(Light { pos: mid + Vec3::new(0.0, 0.4, 0.4), color: lin(FLAME) * 3.2, distance: 5.5 });
        }
        // Petals drifting down through the air about him, and fireflies wandering low.
        for k in 0..36 {
            let seed = k as f32 * 7.31;
            let u = (time * (0.08 + (k % 5) as f32 * 0.012) + seed * 0.13).fract();
            let x = 3.0 + ((seed * 3.7).sin() * 0.5 + 0.5) * 26.0 + (time * 0.4 + seed).sin() * 0.6;
            let z = 3.0 + ((seed * 1.9).cos() * 0.5 + 0.5) * 26.0 + (time * 0.3 + seed).cos() * 0.6;
            let p = Vec3::new(x, 4.5 - u * 4.5, z);
            if (p - hero).length() > 14.0 {
                continue;
            }
            let rot = Quat::from_rotation_y(time * 2.0 + seed) * Quat::from_rotation_x(time * 3.0 + seed);
            let col = [CRIMSON, RED, PINK, WHITE][k % 4];
            f.opaque[MeshKind::Cube as usize].push(Instance::new(
                Mat4::from_scale_rotation_translation(Vec3::new(0.08, 0.01, 0.06), rot, p),
                lin(col),
                1.0,
                lin(col) * 0.1,
                SHADE_LIT,
            ));
        }
        for k in 0..18 {
            let seed = k as f32 * 4.17;
            let base = Vec3::new(4.0 + ((seed * 2.3).sin() * 0.5 + 0.5) * 24.0, 0.0, 4.0 + ((seed * 1.3).cos() * 0.5 + 0.5) * 24.0);
            let p = base
                + Vec3::new(
                    (time * 0.6 + seed).sin() * 1.2,
                    0.5 + (time * 0.9 + seed * 2.0).sin().abs() * 0.8,
                    (time * 0.45 + seed).cos() * 1.2,
                );
            let blink = ((time * 2.3 + seed).sin() * 0.5 + 0.5).powi(3);
            if blink > 0.05 && (p - hero).length() < 16.0 {
                glow_orb(f, p, 0.16, lin(FIREFLY), 0.5 * blink);
            }
        }
        // The prince stands in his own light: rose before him and above, the cold moon behind.
        for e in s.enemies {
            let Some(d) = e.duel.as_deref().filter(|d| d.alive() && d.breed.kind == EnemyKind::Asmodeus) else { continue };
            let p = Vec3::new(d.pos.x as f32, 0.0, d.pos.y as f32);
            let ahead = Vec3::new(d.facing.x as f32, 0.0, d.facing.y as f32);
            f.lights.push(Light { pos: p + ahead * 2.4 + Vec3::new(0.0, 3.6, 0.0), color: lin(0xffb0c8) * 5.0, distance: 7.5 });
            f.lights.push(Light { pos: p - ahead * 1.8 + Vec3::new(0.0, 3.2, 0.0), color: lin(MOONLIGHT) * 4.0, distance: 5.5 });
        }
        // The arch's heart catches the light; the gate he came in by.
        f.lights.push(Light { pos: Vec3::new(16.0, 3.6, 5.6), color: lin(0xff6a9a) * 2.4, distance: 6.0 });
        let door = s.gate.map(|g| g.door).unwrap_or(0.0);
        self.emit_portal(&bv.gate, door, false, time, hero, f);
    }

    /// Asmodeus's thorns: where each will burst, a dark stain in the floor with a ring of rose light
    /// closing on it (the tell), then black thorns thrust up out of the ground, and sinking back.
    pub(super) fn emit_bower_thorns(&self, traps: &[Trap], grid: &Grid, time: f32, f: &mut Frame) {
        for t in traps {
            let ground = floor_at(grid, t.pos.x as f32, t.pos.y as f32);
            let at = Vec3::new(t.pos.x as f32, ground, t.pos.y as f32);
            let seed = (t.pos.x * 3.7 + t.pos.y * 1.9) as f32;
            let r = t.radius as f32;
            if let Some(b) = t.burst {
                if b >= SPIKES {
                    continue;
                }
                let u = b as f32 / SPIKES as f32;
                let rise = if u < 0.15 { u / 0.15 } else { 1.0 - ((u - 0.15) / 0.85).powi(2) };
                for k in 0..11 {
                    let a = k as f32 * 2.39 + seed;
                    let rr = if k == 0 { 0.0 } else { r * (0.25 + 0.65 * ((k * 37 % 10) as f32 / 10.0)) };
                    let tall = (0.6 + 0.5 * ((k * 13 % 7) as f32 / 7.0)) * rise;
                    let lean = 0.25 * (a * 2.0).sin();
                    let p = at + Vec3::new(a.cos() * rr, tall * 0.5, a.sin() * rr);
                    let m = Mat4::from_translation(p)
                        * Mat4::from_rotation_y(a)
                        * Mat4::from_rotation_z(lean)
                        * Mat4::from_scale(Vec3::new(0.07, tall.max(0.01), 0.07));
                    f.opaque[MeshKind::Cube as usize].push(Instance::new(m, lin(0x24160f), 1.0, Vec3::ZERO, SHADE_LIT));
                    if k % 3 == 0 {
                        // A rose on the thorn's tip.
                        let tip = p + Vec3::new(-lean * tall * 0.5, tall * 0.5, 0.0);
                        let m = Mat4::from_translation(tip) * Mat4::from_scale(Vec3::splat(0.09 * rise));
                        f.opaque[MeshKind::Sphere as usize].push(Instance::new(m, lin(0xb0122e), 1.0, lin(0x60081a) * 0.4, SHADE_LIT));
                    }
                }
                f.lights.push(Light { pos: at + Vec3::new(0.0, 0.5, 0.0), color: lin(0xff3a5a) * 3.0 * (1.0 - u), distance: 2.6 });
                continue;
            }
            // The tell: a dark stain where the ground will open, a ring of rose light closing in.
            let fill = if t.flight > 0 { 0.0 } else { t.tell() };
            let late = t.flight == 0 && t.fuse <= 8;
            let blink = if late { 0.6 + (time * 40.0).sin().abs() * 0.8 } else { 0.0 };
            let m = Mat4::from_translation(at + Vec3::new(0.0, 0.02, 0.0))
                * Mat4::from_rotation_x(-PI / 2.0)
                * Mat4::from_scale(Vec3::new(r, r, 1.0));
            f.transparent[MeshKind::Disc as usize].push(Instance::new(
                m,
                lin(0x2a0810) * (0.6 + blink),
                0.25 + 0.35 * fill,
                Vec3::ZERO,
                SHADE_UNLIT,
            ));
            let n = ((TAU * r) / 0.22).ceil() as i32;
            for k in 0..n {
                let a = k as f32 * TAU / n as f32 + time * 0.6;
                let p = at + Vec3::new(a.cos() * r, 0.03, a.sin() * r);
                let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(0.08, 0.012, 0.08));
                f.additive.push(Instance::new(m, lin(0xff4a7a) * (0.9 + blink), 0.7, Vec3::ZERO, SHADE_UNLIT));
            }
            let inner = r * (1.0 - fill);
            let m = Mat4::from_translation(at + Vec3::new(0.0, 0.03, 0.0))
                * Mat4::from_rotation_x(-PI / 2.0)
                * Mat4::from_scale(Vec3::new(inner + 0.05, inner + 0.05, 1.0));
            f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, lin(0xff7aa8) * 1.2, 0.5, Vec3::ZERO, SHADE_UNLIT));
            // Thorn tips pricking through the floor as the fuse burns.
            for k in 0..4 {
                let a = seed + k as f32 * 1.7;
                let rr = r * 0.45 * (k as f32 / 4.0 + 0.2);
                let tall = 0.12 * fill;
                let p = at + Vec3::new(a.cos() * rr, tall * 0.5, a.sin() * rr);
                let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(0.04, tall.max(0.005), 0.04));
                f.opaque[MeshKind::Cube as usize].push(Instance::new(m, lin(0x2a1a12), 1.0, Vec3::ZERO, SHADE_LIT));
            }
        }
    }
}
