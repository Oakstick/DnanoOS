//! The boss arena's look, and the boss. A bright meadow under a daylight sky:
//! grass with tufts and flowers, bare dirt patches, weathered stone pillars,
//! low brick walls, and a stream under wooden bridges. Then the Cyclops, its
//! bolts and boulders, and the shockwaves of its slams.

use super::*;
use crate::logic::boss::{BossShot, BossState, Cyclops, ShotKind};
use crate::logic::levels::ARENA;

mod pal {
    pub const GRASS_A: u32 = 0x6cbf3e;
    pub const GRASS_B: u32 = 0x62b437;
    pub const GRASS_DEEP: u32 = 0x3f8a26;
    pub const GRASS_TIP: u32 = 0x9ee05a;
    pub const DIRT_B: u32 = 0x7d5b34;
    pub const PEBBLE: u32 = 0xa8a294;
    pub const HEDGE_A: u32 = 0x3f8f32;
    pub const HEDGE_B: u32 = 0x367f2b;
    pub const STONE: u32 = 0xb3ada0;
    pub const STONE_D: u32 = 0x8f8a80;
    pub const MOSS: u32 = 0x5f9a38;
    pub const BRICKS: [u32; 4] = [0xb05c3c, 0x9a4e33, 0xc26c48, 0x8a442e];
    pub const MORTAR: u32 = 0xcbbfa4;
    pub const WOOD: [u32; 3] = [0xa06a38, 0x8a5a2e, 0xb27a44];
    pub const WOOD_D: u32 = 0x6e4422;
    pub const WATER: u32 = 0x3f9fdc;
    pub const BED: u32 = 0x6a6448;
    pub const FLOWERS: [u32; 5] = [0xf8f8f0, 0xffe066, 0xff8ad8, 0x8fb4ff, 0xff6a5a];
    pub const BUSH: [u32; 3] = [0x56bc40, 0x62c848, 0x4aad3a];
    pub const BUSH_LIGHT: u32 = 0x94e466;
    pub const FOREST_BUSH: [u32; 3] = [0x2f6b2a, 0x3a7a2e, 0x285a22];
    pub const FOREST_BUSH_LIGHT: u32 = 0x5a9a3a;
    pub const BERRIES: [u32; 3] = [0xd8262e, 0xf8f8f0, 0xffb0d8];
    pub const BUTTERFLIES: [u32; 4] = [0xf8f8f0, 0xffe066, 0xff9a3a, 0x7ab8ff];
    pub const BOLT: u32 = 0xff7a2a;
    pub const BOLT_CORE: u32 = 0xfff1c0;
    pub const DUST: u32 = 0xe8d8b0;
}
use pal::*;

/// Daylight for the meadow: a blue sky, a warm sun, bright bounce from the grass.
pub fn meadow_atmosphere() -> Atmosphere {
    Atmosphere {
        background: 0x9fd3f2,
        fog_near: 22.0,
        fog_far: 62.0,
        hemi_sky: lin(0xcfe6ff) * 0.62,
        hemi_ground: lin(0x5a7a32) * 0.55,
        ambient: lin(0x8a9a80) * 0.22,
        sun_dir: Vec3::new(-0.45, 1.0, 0.55).normalize(),
        sun: lin(0xfff0d6) * 1.35,
        exposure: 1.05,
        night_sky: 0.0,
        moon_view: Vec3::Y,
    }
}

fn cube(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot_y: f32, color: Vec3) {
    let m = Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(rot_y), pos);
    out.push(Instance::new(m, color, 1.0, Vec3::ZERO, SHADE_LIT));
}


fn in_arena(x: i32, y: i32) -> bool {
    let (x0, y0, x1, y1) = ARENA;
    x >= x0 && x <= x1 && y >= y0 && y <= y1
}

impl WorldView {
    /// The meadow's static geometry. Rock inside the arena is stone pillars
    /// (standing alone) or low brick walls (in runs); rock in the ring is hedge.
    pub(super) fn build_meadow(&mut self, grid: &Grid) {
        let rock = |x: i32, y: i32| grid.get(x, y) == Tile::Rock;
        let pit = |x: i32, y: i32| grid.get(x, y) == Tile::Pit;
        let bridge = |x: i32, y: i32| !pit(x, y) && !rock(x, y) && ((pit(x - 1, y) && pit(x + 1, y)) || (pit(x, y - 1) && pit(x, y + 1)));
        self.water.clear();

        for (x, y, t) in grid.cells() {
            let (xf, yf) = (x as f32, y as f32);
            let (hx, hy) = (x as f64, y as f64);
            if t == Tile::Rock {
                if in_arena(x, y) {
                    cube(&mut self.static_cubes, Vec3::new(xf, -0.05, yf), Vec3::new(1.0, 0.1, 1.0), 0.0, lin(GRASS_B) * 0.9);
                }
                continue;
            }
            if t == Tile::Pit || bridge(x, y) {
                // Stream bed, banks, and (on a bridge) the deck.
                let along_x = pit(x - 1, y) || pit(x + 1, y);
                self.water.push((P::new(x, y), along_x));
                cube(&mut self.static_cubes, Vec3::new(xf, -0.37, yf), Vec3::new(1.0, 0.1, 1.0), 0.0, lin(BED) * (0.9 + hash(hx, hy) as f32 * 0.2));
                for d in CARDINALS {
                    let (dx, dy) = d.vec();
                    let (nx, ny) = (x + dx, y + dy);
                    if pit(nx, ny) || bridge(nx, ny) {
                        continue;
                    }
                    let c = Vec3::new(xf + dx as f32 * 0.5, -0.17, yf + dy as f32 * 0.5);
                    let s = if dx != 0 { Vec3::new(0.06, 0.36, 1.0) } else { Vec3::new(1.0, 0.36, 0.06) };
                    cube(&mut self.static_cubes, c, s, 0.0, lin(DIRT_B));
                    // Stones along the bank.
                    for k in 0..3 {
                        let hk = hash(hx * 3.0 + k as f64, hy * 5.0 + dx as f64 * 2.0 + dy as f64) as f32;
                        if hk < 0.45 {
                            continue;
                        }
                        let off = (k as f32 - 1.0) * 0.32 + (hk - 0.7) * 0.2;
                        let p = if dx != 0 { Vec3::new(c.x - dx as f32 * 0.04, 0.0, c.z + off) } else { Vec3::new(c.x + off, 0.0, c.z - dy as f32 * 0.04) };
                        let sz = 0.1 + hk * 0.08;
                        cube(&mut self.static_cubes, p, Vec3::new(sz, sz * 0.6, sz * 0.9), hk * 3.0, lin(if hk > 0.8 { STONE } else { PEBBLE }));
                    }
                }
                if bridge(x, y) {
                    self.build_bridge(xf, yf, along_x);
                }
                continue;
            }
            // Grass everywhere.
            let jitter = 0.94 + hash(hx, hy) as f32 * 0.12;
            let grass = lin(if (x + y) % 2 == 0 { GRASS_A } else { GRASS_B }) * jitter;
            cube(&mut self.static_cubes, Vec3::new(xf, -0.05, yf), Vec3::new(1.0, 0.1, 1.0), 0.0, grass);
            // Tufts of grass...
            let tufts = 2 + (hash(hx + 1.0, hy + 2.0) * 3.0) as usize;
            for k in 0..tufts {
                let hk = hash(hx * 5.0 + k as f64 * 1.7, hy * 3.0 + 1.0) as f32;
                let hj = hash(hy * 5.0 + k as f64 * 2.3, hx * 7.0 + 2.0) as f32;
                let base = Vec3::new(xf + (hk - 0.5) * 0.85, 0.0, yf + (hj - 0.5) * 0.85);
                for b in 0..3 {
                    let hb = hash(hk as f64 * 91.0 + b as f64, hj as f64 * 37.0) as f32;
                    let tall = 0.1 + hb * 0.1;
                    let lean = (hb - 0.5) * 0.5;
                    let m = Mat4::from_translation(base + Vec3::new((b as f32 - 1.0) * 0.035, 0.0, (hb - 0.5) * 0.04))
                        * Mat4::from_rotation_y(hk * 6.0 + b as f32 * 2.1)
                        * Mat4::from_rotation_z(lean)
                        * Mat4::from_translation(Vec3::new(0.0, tall / 2.0, 0.0))
                        * Mat4::from_scale(Vec3::new(0.03, tall, 0.03));
                    let c = lin(if b == 1 { GRASS_TIP } else { GRASS_DEEP }).lerp(lin(GRASS_A), hb * 0.5);
                    self.static_cubes.push(Instance::new(m, c, 1.0, Vec3::ZERO, SHADE_LIT));
                }
            }
            // ...and here and there a flower.
            if hash(hx * 2.0 + 7.0, hy * 3.0 + 1.0) < 0.22 {
                let hk = hash(hx + 0.3, hy * 1.7) as f32;
                let hj = hash(hy + 0.7, hx * 1.3) as f32;
                let p = Vec3::new(xf + (hk - 0.5) * 0.7, 0.0, yf + (hj - 0.5) * 0.7);
                cube(&mut self.static_cubes, p + Vec3::new(0.0, 0.08, 0.0), Vec3::new(0.02, 0.16, 0.02), 0.0, lin(GRASS_DEEP));
                let petal = lin(FLOWERS[(hash(hx, hy + 5.0) * FLOWERS.len() as f64) as usize % FLOWERS.len()]);
                cube(&mut self.static_cubes, p + Vec3::new(0.0, 0.17, 0.0), Vec3::new(0.09, 0.03, 0.09), hk * 3.0, petal);
                cube(&mut self.static_cubes, p + Vec3::new(0.0, 0.185, 0.0), Vec3::new(0.035, 0.02, 0.035), 0.0, lin(0xffc830));
            }
        }

        // Rock: pillars, low walls, and the hedge ring with its trees.
        let tree = crate::render::props::pine();
        let tree_tint = Vec3::new(1.15, 1.35, 1.05);
        let mut trees: Vec<(P, Vec3, f32, f32)> = Vec::new();
        for (x, y, t) in grid.cells() {
            if t != Tile::Rock {
                continue;
            }
            let (xf, yf) = (x as f32, y as f32);
            let (hx, hy) = (x as f64, y as f64);
            if in_arena(x, y) {
                let horizontal = rock(x - 1, y) || rock(x + 1, y);
                let vertical = rock(x, y - 1) || rock(x, y + 1);
                if !horizontal && !vertical {
                    self.build_pillar(x, y);
                } else {
                    let start = self.static_cubes.len();
                    self.build_low_wall(xf, yf, horizontal);
                    self.rock_index.insert(tile_key(x, y), (start, self.static_cubes.len() - start));
                }
                continue;
            }
            let edge = grid.get(x - 1, y) != Tile::Rock || grid.get(x + 1, y) != Tile::Rock || grid.get(x, y - 1) != Tile::Rock || grid.get(x, y + 1) != Tile::Rock;
            let tall = if edge { 0.75 + hash(hx, hy) as f32 * 0.25 } else { 0.95 + hash(hx, hy) as f32 * 0.3 };
            let shade = hash(hx + 11.0, hy + 5.0) as f32;
            let c = offset_hsl(lin(if shade < 0.5 { HEDGE_A } else { HEDGE_B }), 0.0, 0.0, (shade - 0.5) * 0.06);
            let start = self.static_cubes.len();
            cube(&mut self.static_cubes, Vec3::new(xf, tall / 2.0, yf), Vec3::new(1.0, tall, 1.0), 0.0, c);
            self.rock_index.insert(tile_key(x, y), (start, 1));
            if edge && hash(hx * 5.0 + 1.0, hy * 3.0 + 2.0) < 0.55 {
                trees.push((P::new(x, y), Vec3::new(xf, tall - 0.05, yf), hash(hx, hy) as f32 * TAU, 0.9 + hash(hy, hx) as f32 * 0.6));
            }
        }
        for (key, pos, rot, scale) in trees {
            let start = self.static_cubes.len();
            push_field(&mut self.static_cubes, &tree, pos, rot, scale, tree_tint, Vec3::ZERO, SHADE_LIT);
            self.tree_index.insert(key, self.trees.len());
            self.trees.push(Tree { range: (start, self.static_cubes.len() - start), base: pos, radius: 0.26 * scale + 0.05, height: 1.02 * scale, fade: 1.0, alive: true });
        }

        // Bushes tucked against the hedges, pillars and walls, off the walking lines.
        for (x, y, t) in grid.cells() {
            if !in_arena(x, y) || !matches!(t, Tile::Open | Tile::Charge) || bridge(x, y) {
                continue;
            }
            for d in CARDINALS {
                let (dx, dy) = d.vec();
                if !rock(x + dx, y + dy) {
                    continue;
                }
                let arena_rock = in_arena(x + dx, y + dy);
                let chance = if arena_rock { 0.65 } else { 0.5 };
                let h = hash(x as f64 * 3.1 + dx as f64 * 7.0, y as f64 * 2.3 + dy as f64 * 5.0);
                if h > chance {
                    continue;
                }
                // Slide along the edge a little so rows of bushes do not line up.
                let slide = (hash(y as f64 + dx as f64, x as f64 + dy as f64) as f32 - 0.5) * 0.5;
                let pos = Vec3::new(x as f32 + dx as f32 * 0.3 + dy as f32 * slide, 0.0, y as f32 + dy as f32 * 0.3 + dx as f32 * slide);
                self.build_bush(pos, 0.95 + h as f32 * 0.5, false);
            }
        }
    }

    /// A round leafy bush: a mound of overlapping leaf blocks, lighter on top, now and then with berries or blossoms.
    pub(super) fn build_bush(&mut self, at: Vec3, size: f32, forest: bool) {
        let (leaves, light) = if forest { (FOREST_BUSH, FOREST_BUSH_LIGHT) } else { (BUSH, BUSH_LIGHT) };
        let (hx, hy) = (at.x as f64 * 13.0, at.z as f64 * 17.0);
        let blobs = 11;
        for k in 0..blobs {
            let hk = hash(hx + k as f64 * 1.9, hy - k as f64 * 2.7) as f32;
            let hj = hash(hy + k as f64 * 3.3, hx + k as f64) as f32;
            let a = k as f32 / blobs as f32 * TAU + hk;
            // A dome: blobs on the rim sit low, the ones near the middle high.
            let r = if k == 0 { 0.0 } else if k < 7 { 0.15 + hj * 0.05 } else { 0.07 + hj * 0.04 };
            let s = (if k < 7 { 0.15 } else { 0.18 } + hk * 0.07) * size;
            let y = if k == 0 { 0.24 } else if k < 7 { 0.09 + hj * 0.05 } else { 0.2 + hj * 0.06 } * size;
            let p = at + Vec3::new(a.cos() * r * size, y, a.sin() * r * size);
            let c = lin(leaves[(hk * 3.0) as usize % 3]) * (0.9 + hj * 0.2);
            let m = Mat4::from_translation(p) * Mat4::from_rotation_y(a) * Mat4::from_rotation_x((hk - 0.5) * 0.6) * Mat4::from_scale(Vec3::new(s, s * 0.85, s));
            self.static_cubes.push(Instance::new(m, c, 1.0, Vec3::ZERO, SHADE_LIT));
        }
        // Sunlit leaves on the crown.
        for k in 0..3 {
            let hk = hash(hx - k as f64 * 5.0, hy + k as f64 * 4.0) as f32;
            let a = hk * TAU;
            let p = at + Vec3::new(a.cos() * 0.07 * size, (0.33 + hk * 0.03) * size, a.sin() * 0.07 * size);
            cube(&mut self.static_cubes, p, Vec3::new(0.14, 0.06, 0.14) * size, a, lin(light));
        }
        let flavor = hash(hx * 0.5, hy * 0.5);
        if flavor < 0.45 {
            let berry = lin(BERRIES[(flavor * 20.0) as usize % 3]);
            for k in 0..5 {
                let hk = hash(hx + k as f64 * 7.0, hy + 3.0) as f32;
                let a = hk * TAU;
                let p = at + Vec3::new(a.cos() * 0.2 * size, (0.12 + hk * 0.18) * size, a.sin() * 0.2 * size);
                cube(&mut self.static_cubes, p, Vec3::splat(0.045), 0.0, berry);
            }
        }
    }

    /// A plank bridge over the stream, crossing it the short way.
    fn build_bridge(&mut self, x: f32, y: f32, stream_along_x: bool) {
        // Built as if the stream ran along x (the deck crosses along z), then turned.
        let turn = if stream_along_x { 0.0 } else { PI / 2.0 };
        let q = Quat::from_rotation_y(turn);
        let at = |p: Vec3| Vec3::new(x, 0.0, y) + q * p;
        let (hx, hy) = (x as f64, y as f64);
        for i in 0..8 {
            let z = -0.62 + i as f32 * 0.177;
            let arch = (1.0 - (z / 0.66).powi(2)).max(0.0) * 0.05;
            let c = lin(WOOD[(hash(hx + i as f64, hy) * 3.0) as usize % 3]);
            let tilt = (hash(hx, hy + i as f64) as f32 - 0.5) * 0.06;
            cube(&mut self.static_cubes, at(Vec3::new(tilt, -0.015 + arch, z)), Vec3::new(0.96, 0.05, 0.15), turn + tilt, c);
        }
        // Beams under the deck, posts and rails on either side.
        for sx in [-0.38f32, 0.38] {
            cube(&mut self.static_cubes, at(Vec3::new(sx, -0.08, 0.0)), Vec3::new(0.08, 0.08, 1.35), turn, lin(WOOD_D));
        }
        for sx in [-0.5f32, 0.5] {
            for sz in [-0.5f32, 0.5] {
                cube(&mut self.static_cubes, at(Vec3::new(sx, 0.18, sz)), Vec3::new(0.08, 0.42, 0.08), turn, lin(WOOD_D));
            }
            cube(&mut self.static_cubes, at(Vec3::new(sx, 0.37, 0.0)), Vec3::new(0.06, 0.06, 1.12), turn, lin(WOOD[0]));
            cube(&mut self.static_cubes, at(Vec3::new(sx, 0.2, 0.0)), Vec3::new(0.04, 0.04, 1.02), turn, lin(WOOD[1]));
        }
    }

    /// A weathered column of stone drums on a plinth; some have lost their tops.
    fn build_pillar(&mut self, x: i32, y: i32) {
        let (xf, yf) = (x as f32, y as f32);
        let (hx, hy) = (x as f64, y as f64);
        let start = self.static_cubes.len();
        let broken = hash(hx * 3.0, hy * 7.0) < 0.34;
        cube(&mut self.static_cubes, Vec3::new(xf, 0.08, yf), Vec3::new(0.82, 0.16, 0.82), 0.0, lin(STONE_D));
        cube(&mut self.static_cubes, Vec3::new(xf, 0.19, yf), Vec3::new(0.68, 0.06, 0.68), 0.0, lin(STONE));
        let drums = if broken { 3 } else { 5 };
        let mut top = 0.22;
        for i in 0..drums {
            let hk = hash(hx + i as f64 * 1.3, hy - i as f64) as f32;
            let c = lin(if i % 2 == 0 { STONE } else { STONE_D }) * (0.92 + hk * 0.14);
            let hgt = 0.3;
            cube(&mut self.static_cubes, Vec3::new(xf, top + hgt / 2.0, yf), Vec3::new(0.5, hgt - 0.015, 0.5), (hk - 0.5) * 0.2, c);
            // Fluting: shallow ribs on each face.
            for f in 0..4 {
                let a = f as f32 * PI / 2.0 + (hk - 0.5) * 0.2;
                let n = Vec3::new(a.cos(), 0.0, a.sin());
                cube(&mut self.static_cubes, Vec3::new(xf, top + hgt / 2.0, yf) + n * 0.25, Vec3::new(0.06, hgt - 0.05, 0.06), a, c * 0.9);
            }
            top += hgt;
        }
        if broken {
            // A jagged break, with the fallen pieces in the grass.
            for k in 0..3 {
                let hk = hash(hx * 9.0 + k as f64, hy * 4.0) as f32;
                let off = Vec3::new((hk - 0.5) * 0.3, 0.0, (hash(hy, hx + k as f64) as f32 - 0.5) * 0.3);
                cube(&mut self.static_cubes, Vec3::new(xf, top + 0.05 + hk * 0.08, yf) + off, Vec3::new(0.2, 0.12 + hk * 0.1, 0.2), hk * 4.0, lin(STONE));
            }
            let a = hash(hx, hy) as f32 * TAU;
            let fallen = Vec3::new(xf + a.cos() * 0.62, 0.13, yf + a.sin() * 0.62);
            cube(&mut self.static_cubes, fallen, Vec3::new(0.5, 0.26, 0.28), a, lin(STONE_D));
            cube(&mut self.static_cubes, fallen + Vec3::new(a.sin() * 0.2, -0.05, -a.cos() * 0.2), Vec3::new(0.22, 0.16, 0.2), a + 0.7, lin(STONE));
        } else {
            cube(&mut self.static_cubes, Vec3::new(xf, top + 0.07, yf), Vec3::new(0.74, 0.14, 0.74), 0.0, lin(STONE));
            cube(&mut self.static_cubes, Vec3::new(xf, top + 0.17, yf), Vec3::new(0.62, 0.06, 0.62), 0.0, lin(STONE_D));
            top += 0.2;
        }
        // Moss creeping up from the base.
        for k in 0..7 {
            let hk = hash(hx * 13.0 + k as f64, hy * 17.0) as f32;
            let a = hk * TAU;
            let n = Vec3::new(a.cos(), 0.0, a.sin());
            let h = 0.25 + hash(hy * 3.0, hx + k as f64) as f32 * 0.5;
            cube(&mut self.static_cubes, Vec3::new(xf, h, yf) + n * 0.27, Vec3::new(0.14, 0.1 + hk * 0.1, 0.05), a + PI / 2.0, lin(MOSS) * (0.9 + hk * 0.2));
        }
        let len = self.static_cubes.len() - start;
        self.tree_index.insert(tile_key(x, y), self.trees.len());
        self.trees.push(Tree { range: (start, len), base: Vec3::new(xf, 0.0, yf), radius: 0.42, height: top, fade: 1.0, alive: true });
    }

    /// One tile of low brick wall: a mortar core, three staggered courses of bricks, coping on top.
    fn build_low_wall(&mut self, x: f32, y: f32, horizontal: bool) {
        let turn = if horizontal { 0.0 } else { PI / 2.0 };
        let q = Quat::from_rotation_y(turn);
        let at = |p: Vec3| Vec3::new(x, 0.0, y) + q * p;
        let (hx, hy) = (x as f64, y as f64);
        cube(&mut self.static_cubes, at(Vec3::new(0.0, 0.2, 0.0)), Vec3::new(1.0, 0.4, 0.3), turn, lin(MORTAR));
        for course in 0..3 {
            let yc = 0.07 + course as f32 * 0.132;
            let shift = if course % 2 == 0 { 0.0 } else { 0.125 };
            for b in 0..5 {
                let cx = -0.5 + shift + b as f32 * 0.25;
                let (a, z) = ((cx - 0.115).max(-0.5), (cx + 0.115).min(0.5));
                if z - a < 0.05 {
                    continue;
                }
                let hk = hash(hx * 7.0 + b as f64 + course as f64 * 3.1, hy * 5.0 + course as f64) as f32;
                let c = lin(BRICKS[(hk * 4.0) as usize % 4]) * (0.92 + hk * 0.12);
                cube(&mut self.static_cubes, at(Vec3::new((a + z) / 2.0, yc, 0.0)), Vec3::new(z - a, 0.115, 0.34), turn, c);
            }
        }
        for k in 0..2 {
            let hk = hash(hx * 3.0 + k as f64, hy * 11.0) as f32;
            cube(&mut self.static_cubes, at(Vec3::new(-0.25 + k as f32 * 0.5, 0.435, 0.0)), Vec3::new(0.49, 0.07, 0.4), turn, lin(STONE) * (0.9 + hk * 0.15));
        }
    }

    /// A green cross that appears mid-fight.
    pub fn add_charge(&mut self, grid: &Grid, x: i32, y: i32) {
        self.charges.push((P::new(x, y), Vec3::new(x as f32, floor_at(grid, x as f32, y as f32) + 0.3, y as f32)));
    }

    /// A ring of dust racing out across the ground from a slam.
    pub fn shockwave(&mut self, grid: &Grid, x: f64, y: f64) {
        let p = Vec3::new(x as f32, floor_at(grid, x as f32, y as f32) + 0.04, y as f32);
        self.rings.push((p, 0.0));
    }

    pub(super) fn update_meadow(&mut self, dt: f32) {
        self.rings.retain_mut(|(_, t)| {
            *t += dt;
            *t < RING_SECONDS
        });
    }

    /// Water, butterflies and pollen: the meadow's moving parts.
    pub(super) fn emit_meadow(&self, time: f32, f: &mut Frame) {
        let cube_k = MeshKind::Cube as usize;
        let water = lin(WATER);
        for &(p, along_x) in &self.water {
            let (x, z) = (p.x as f32, p.y as f32);
            let shimmer = 0.94 + (time * 1.7 + x * 0.9 + z * 0.4).sin() * 0.06;
            let m = Mat4::from_translation(Vec3::new(x, -0.1, z)) * Mat4::from_scale(Vec3::new(1.0, 0.02, 1.0));
            f.transparent[cube_k].push(Instance::new(m, water * shimmer, 0.8, Vec3::ZERO, SHADE_LIT));
            // Glints drifting downstream.
            for k in 0..3 {
                let hk = hash(x as f64 * 3.0 + k as f64, z as f64 * 7.0) as f32;
                let u = (time * (0.35 + hk * 0.2) + hk * 7.0).fract();
                let along = u - 0.5;
                let across = (hash(z as f64 + k as f64, x as f64) as f32 - 0.5) * 0.7;
                let pos = if along_x { Vec3::new(x + along, -0.085, z + across) } else { Vec3::new(x + across, -0.085, z + along) };
                let s = if along_x { Vec3::new(0.2, 0.004, 0.035) } else { Vec3::new(0.035, 0.004, 0.2) };
                let m = Mat4::from_translation(pos) * Mat4::from_scale(s);
                f.additive.push(Instance::new(m, Vec3::splat(0.9), 0.35 * (u * PI).sin(), Vec3::ZERO, SHADE_UNLIT));
            }
        }
        // Butterflies wandering over the grass.
        let (x0, y0, x1, y1) = ARENA;
        for i in 0..18 {
            let fi = i as f32;
            let bx = x0 as f32 + hash(fi as f64, 3.0) as f32 * (x1 - x0) as f32;
            let bz = y0 as f32 + hash(fi as f64, 7.0) as f32 * (y1 - y0) as f32;
            let path = |t: f32| Vec3::new(bx + (t * 0.3 + fi).sin() * 2.2, 0.55 + (t * 1.3 + fi).sin() * 0.25, bz + (t * 0.23 + fi * 1.7).cos() * 2.2);
            let p = path(time);
            let ahead = path(time + 0.05) - p;
            let heading = ahead.x.atan2(ahead.z);
            let flap = (time * 17.0 + fi * 2.0).sin() * 0.9;
            let color = lin(BUTTERFLIES[i % BUTTERFLIES.len()]);
            for side in [-1.0f32, 1.0] {
                let m = Mat4::from_translation(p)
                    * Mat4::from_rotation_y(heading)
                    * Mat4::from_rotation_z(side * flap)
                    * Mat4::from_translation(Vec3::new(side * 0.045, 0.0, 0.0))
                    * Mat4::from_scale(Vec3::new(0.08, 0.006, 0.07));
                f.opaque[cube_k].push(Instance::new(m, color, 1.0, color * 0.15, SHADE_LIT));
            }
            let body = Mat4::from_translation(p) * Mat4::from_rotation_y(heading) * Mat4::from_scale(Vec3::new(0.015, 0.015, 0.06));
            f.opaque[cube_k].push(Instance::new(body, lin(0x2a2018), 1.0, Vec3::ZERO, SHADE_LIT));
        }
        // Pollen in the sunlight.
        for (i, b) in self.mote_base.iter().enumerate().take(70) {
            let fi = i as f32;
            let p = Vec3::new(b.x + (time * 0.3 + fi).sin() * 0.5, b.y * 0.8 + (time * 0.5 + fi * 0.7).sin() * 0.2, b.z + (time * 0.25 + fi * 1.3).cos() * 0.5);
            let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.03));
            f.additive.push(Instance::new(m, lin(0xfff6c8), 0.3 + (time * 1.5 + fi).sin() * 0.15, Vec3::ZERO, SHADE_UNLIT));
        }
        for &(p, t) in &self.rings {
            let k = t / RING_SECONDS;
            let m = Mat4::from_translation(p) * Mat4::from_rotation_x(-PI / 2.0) * Mat4::from_scale(Vec3::splat(0.4 + k * 3.4));
            f.transparent[MeshKind::Ring as usize].push(Instance::new(m, lin(DUST), 0.7 * (1.0 - k), Vec3::ZERO, SHADE_UNLIT));
        }
    }

    /// The Cyclops: bobbing, flapping, looking at him, blinking; glowing when it charges.
    pub(super) fn emit_boss(&self, b: &Cyclops, hero: Vec3, time: f32, f: &mut Frame) {
        if matches!(b.state, BossState::Hidden | BossState::Dead) {
            return;
        }
        let rig = &self.models.cyclops;
        let dazed = b.dazed();
        let dying = b.dying_progress() as f32;
        let bob = if dazed { 0.0 } else { (time * 2.1).sin() * 0.07 };
        let mut pos = Vec3::new(b.pos.x as f32, b.hover as f32 + bob, b.pos.y as f32);
        if dying > 0.0 {
            let j = 0.05 + dying * 0.1;
            pos += Vec3::new((time * 71.0).sin() * j, (time * 53.0).cos() * j * 0.5, (time * 67.0).cos() * j);
        }
        let yaw = PI / 2.0 - b.look as f32;
        let tilt = if dazed { (time * 3.0).sin() * 0.25 } else { 0.0 };
        let swell = 1.0 + dying * 0.25 + if dying > 0.0 { (time * 30.0).sin() * 0.04 * dying } else { 0.0 };
        let root = Mat4::from_translation(pos) * Mat4::from_rotation_y(yaw) * Mat4::from_rotation_z(tilt) * Mat4::from_scale(Vec3::splat(swell));

        let mut pose = Pose::rest(rig);
        let beat = if dazed { 3.0 } else { 9.0 };
        let flap = (time * beat).sin() * 0.55 + 0.15;
        if let Some(i) = rig.index("leftWing") {
            pose.rot[i].z = flap;
        }
        if let Some(i) = rig.index("rightWing") {
            pose.rot[i].z = -flap;
        }
        for k in 0..4 {
            if let Some(i) = rig.index(&format!("tentacle{k}")) {
                let kf = k as f32;
                pose.rot[i].x = (time * 3.1 + kf * 1.3).sin() * 0.35;
                pose.rot[i].z = (time * 2.3 + kf * 2.1).cos() * 0.25;
            }
        }
        // The iris follows him within its socket; dazed, it rolls.
        let to_hero = hero - pos;
        let want = to_hero.x.atan2(to_hero.z) - yaw;
        let want = (want + PI).rem_euclid(TAU) - PI;
        if let Some(i) = rig.index("iris") {
            if dazed {
                pose.rot[i].y = (time * 5.0).sin() * 0.3;
                pose.rot[i].x = (time * 5.0).cos() * 0.25;
            } else {
                let flat = Vec2::new(to_hero.x, to_hero.z).length().max(0.5);
                pose.rot[i].y = want.clamp(-0.4, 0.4);
                pose.rot[i].x = ((-to_hero.y + 0.6) / flat).atan().clamp(-0.1, 0.4);
            }
        }
        // Blinks now and then; squints to charge; shuts as it dies.
        let blink_t = (time * 0.31 + 0.2).fract();
        let blink = if blink_t < 0.04 { (blink_t / 0.04 * PI).sin() } else { 0.0 };
        let shut = (b.squint() as f32).max(blink);
        if let Some(i) = rig.index("lidTop") {
            pose.rot[i].x = 1.05 * shut;
        }
        if let Some(i) = rig.index("lidBottom") {
            pose.rot[i].x = -0.55 * shut;
        }

        let flash = b.flash as f32;
        let glow = b.glow() as f32;
        let rage = if b.phase() >= 3 { 0.1 + (time * 6.0).sin().abs() * 0.1 } else { 0.0 };
        let body_emissive = Vec3::splat(flash * 0.5) + lin(0xff2a2a) * rage + Vec3::splat(if dying > 0.0 && (time * 14.0).sin() > 0.3 { 0.6 * dying } else { 0.0 });
        let tint = if flash > 0.0 { Vec3::ONE.lerp(Vec3::new(1.0, 0.55, 0.55), flash) } else { Vec3::ONE };
        let iris_emissive = lin(BOLT) * glow * 0.9 + body_emissive;
        let s = Mat4::from_scale(Vec3::splat(rig.size));
        let out = &mut f.opaque[MeshKind::Cube as usize];
        for (i, part) in rig.parts.iter().enumerate() {
            let em = if part.name == "iris" { iris_emissive } else { body_emissive };
            let pm = root * Mat4::from_translation(pose.pos[i]) * euler(pose.rot[i]);
            for (off, c) in &part.cubes {
                out.push(Instance::new(pm * Mat4::from_translation(*off) * s, *c * tint, 1.0, em, SHADE_LIT));
            }
        }
        // Stars circling a dazed head.
        if dazed {
            for k in 0..5 {
                let a = time * 4.0 + k as f32 * TAU / 5.0;
                let p = pos + Vec3::new(a.cos() * 1.0, 2.75 + (time * 6.0 + k as f32).sin() * 0.1, a.sin() * 1.0);
                let m = Mat4::from_translation(p) * Mat4::from_rotation_y(a * 2.0) * Mat4::from_rotation_z(0.785) * Mat4::from_scale(Vec3::splat(0.2));
                f.opaque[MeshKind::Cube as usize].push(Instance::new(m, lin(0xffd23a), 1.0, lin(0xffc020) * 0.8, SHADE_LIT));
            }
        }
        // Its shadow, smaller and fainter the higher it flies.
        let lift = (b.hover as f32).max(0.0);
        let k = (1.0 - lift / 8.0).clamp(0.25, 1.0);
        let m = Mat4::from_translation(Vec3::new(pos.x, 0.02, pos.z)) * Mat4::from_rotation_x(-PI / 2.0) * Mat4::from_scale(Vec3::new(1.05 * k, 1.05 * k, 1.0));
        f.transparent[MeshKind::Disc as usize].push(Instance::new(m, Vec3::ZERO, 0.4 * k, Vec3::ZERO, SHADE_UNLIT));
        // The eye lights the grass when it charges.
        let eye = root.transform_point3(Vec3::new(0.0, CYCLOPS_EYE_Y, 2.4));
        if glow > 0.0 {
            f.lights.push(Light { pos: eye, color: lin(BOLT) * (1.0 + glow * 5.0), distance: 5.5 });
        }
        if flash > 0.0 {
            f.lights.push(Light { pos: eye, color: Vec3::ONE * flash * 3.0, distance: 4.0 });
        }
    }

    /// Bolts glow and trail; boulders roll.
    pub(super) fn emit_boss_shots(&self, shots: &[BossShot], hero: Vec3, time: f32, f: &mut Frame) {
        let mut lit: Vec<(f32, Vec3)> = Vec::new();
        for s in shots {
            let p = Vec3::new(s.pos.x as f32, 0.0, s.pos.y as f32);
            let v = Vec3::new(s.vel.x as f32, 0.0, s.vel.y as f32);
            match s.kind {
                ShotKind::Bolt => {
                    let c = p + Vec3::new(0.0, 0.72, 0.0);
                    let pulse = 1.0 + (time * 30.0 + p.x * 3.0).sin() * 0.12;
                    let core = Mat4::from_translation(c) * Mat4::from_scale(Vec3::splat(0.13 * pulse));
                    f.transparent[MeshKind::Sphere as usize].push(Instance::new(core, lin(BOLT_CORE), 1.0, Vec3::ZERO, SHADE_UNLIT));
                    let shell = Mat4::from_translation(c) * Mat4::from_scale(Vec3::splat(0.24 * pulse));
                    f.additive.push(Instance::new(shell, lin(BOLT) * 1.3, 0.8, Vec3::ZERO, SHADE_UNLIT));
                    let back = v.normalize_or_zero();
                    for k in 1..4 {
                        let t = Mat4::from_translation(c - back * (k as f32 * 0.14)) * Mat4::from_scale(Vec3::splat(0.2 - k as f32 * 0.04));
                        f.additive.push(Instance::new(t, lin(BOLT), 0.6 - k as f32 * 0.15, Vec3::ZERO, SHADE_UNLIT));
                    }
                    lit.push((c.distance_squared(hero), c));
                }
                ShotKind::Boulder => {
                    let r = s.kind.radius() as f32;
                    let axis = Vec3::new(v.z, 0.0, -v.x).normalize_or_zero();
                    let spin = if axis == Vec3::ZERO { Quat::IDENTITY } else { Quat::from_axis_angle(axis, -(s.rolled as f32) / r) };
                    let root = Mat4::from_translation(p + Vec3::new(0.0, r, 0.0)) * Mat4::from_quat(spin) * Mat4::from_translation(Vec3::new(0.0, -r, 0.0));
                    emit_rig(&mut f.opaque[MeshKind::Cube as usize], &self.models.boulder, &Pose::rest(&self.models.boulder), root, Vec3::ONE, 1.0, Vec3::ZERO);
                    let m = Mat4::from_translation(p + Vec3::new(0.0, 0.02, 0.0)) * Mat4::from_rotation_x(-PI / 2.0) * Mat4::from_scale(Vec3::new(0.42, 0.42, 1.0));
                    f.transparent[MeshKind::Disc as usize].push(Instance::new(m, Vec3::ZERO, 0.35, Vec3::ZERO, SHADE_UNLIT));
                }
            }
        }
        // Only the bolts nearest him get a real light; the rest just glow.
        lit.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        for (_, c) in lit.into_iter().take(6) {
            f.lights.push(Light { pos: c, color: lin(BOLT) * 5.0, distance: 3.2 });
        }
    }
}

/// Seconds a slam's dust ring takes to spread and fade.
const RING_SECONDS: f32 = 0.55;
/// Height of the eye's center above the model root, world units.
const CYCLOPS_EYE_Y: f32 = 18.0 * 0.07;

/// A rough grey boulder for the Cyclops to throw.
pub fn boulder() -> crate::render::voxels::Collected {
    use crate::render::characters::Sculpt;
    let mut s = Sculpt::new(9, 9, 9);
    s.ellipsoid((4.0, 4.0, 4.0), (4.3, 3.9, 4.1), 0, 0x9a948a);
    s.paint(|x, y, z, _| {
        let n = hash(x as f64 * 1.7 + z as f64, y as f64 * 2.3) as f32;
        Some(if n < 0.25 { 0x7a766e } else if n > 0.85 { 0xb8b2a6 } else if n > 0.78 { 0x6f8f3a } else { 0x9a948a })
    });
    s.finish(0.08)
}
