//! The dungeons' terrain: a night forest cut by old forest roads. Walkable
//! tiles are packed-earth roads with grass verges, wheel ruts, a grassy
//! strip down the middle, fallen leaves and the odd mushroom. Wall tiles
//! are mossy mounds edged along every road by a dry-stone fence, with ferns,
//! bushes and a mixed wood of pines, oaks and birches growing on them.

use super::*;

mod pal {
    pub const ROAD: u32 = 0x6e553a;
    pub const ROAD_D: u32 = 0x5a4430;
    pub const RUT: u32 = 0x4a3826;
    pub const CLIFF: u32 = 0x4a3c2e;
    pub const VERGE: u32 = 0x3e6e30;
    pub const VERGE_D: u32 = 0x2f5a26;
    pub const TUFT: u32 = 0x5a8a3a;
    pub const MOUND: u32 = 0x2c4424;
    pub const MOUND_D: u32 = 0x243a1e;
    pub const FERN: u32 = 0x2f6b2a;
    pub const FERN_L: u32 = 0x4a8a38;
    pub const STONES: [u32; 4] = [0x7a7c76, 0x6a6c66, 0x8a8a82, 0x5e605a];
    pub const MOSS: u32 = 0x4a7a32;
    pub const LEAVES: [u32; 4] = [0xc87a2a, 0xe0a030, 0x8a5a2a, 0x6a8a2a];
    pub const PEBBLE: u32 = 0x8a8a84;
    pub const STEP_STONE: u32 = 0x76766e;
}
use pal::*;

/// Height of a wall mound above the floor it rises from.
const MOUND_H: f32 = 0.18;
/// One course of fence stones.
const COURSE: f32 = 0.17;

fn cube(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot_y: f32, color: Vec3) {
    let m = Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(rot_y), pos);
    out.push(Instance::new(m, color, 1.0, Vec3::ZERO, SHADE_LIT));
}

fn jit(hex: u32, h: f64, amount: f32) -> Vec3 {
    lin(hex) * (1.0 - amount + h as f32 * amount * 2.0)
}

/// Size (radius, height) of a tree mesh, for the camera's see-through test.
fn tree_extent(m: &Mesh) -> (f32, f32) {
    let mut r: f32 = 0.0;
    let mut h: f32 = 0.0;
    for (p, _) in &m.cubes {
        r = r.max(Vec2::new(p.x, p.z).length());
        h = h.max(p.y);
    }
    (r * 0.75, h + m.size)
}

impl WorldView {
    pub(super) fn build_forest(&mut self, grid: &Grid) {
        let floor_y = |x: i32, y: i32| grid.elevation(x, y) as f32 * STEP;
        let rock = |x: i32, y: i32| grid.get(x, y) == Tile::Rock;
        let white = Vec3::ONE;

        // ---- roads
        for (x, y, t) in grid.cells() {
            if t == Tile::Rock || t == Tile::Pit || grid.is_ramp(x, y) {
                continue;
            }
            let (xf, yf) = (x as f32, y as f32);
            let (hx, hy) = (x as f64, y as f64);
            let h = floor_y(x, y);
            let road = if t == Tile::Nest { lin(HOLLOW) } else { jit(if (x + y) % 2 == 0 { ROAD } else { ROAD_D }, hash(hx, hy), 0.06) * (1.0 + grid.elevation(x, y) as f32 * 0.06) };
            if h > 0.0 {
                // Raised ground: an earthen cliff with the road on top.
                let body = h - 0.04;
                cube(&mut self.static_cubes, Vec3::new(xf, body / 2.0 - 0.05, yf), Vec3::new(1.0, body + 0.1, 1.0), 0.0, jit(CLIFF, hash(hx + 2.0, hy), 0.08));
                cube(&mut self.static_cubes, Vec3::new(xf, h - 0.02, yf), Vec3::new(1.0, 0.04, 1.0), 0.0, road);
            } else {
                cube(&mut self.static_cubes, Vec3::new(xf, -0.05, yf), Vec3::new(1.0, 0.1, 1.0), 0.0, road);
            }
            if t == Tile::Nest {
                continue;
            }
            // Grass verges where the road meets the woods.
            for d in CARDINALS {
                let (dx, dy) = d.vec();
                if !rock(x + dx, y + dy) {
                    continue;
                }
                let c = Vec3::new(xf + dx as f32 * 0.4, h + 0.006, yf + dy as f32 * 0.4);
                let s = if dx != 0 { Vec3::new(0.2, 0.012, 1.0) } else { Vec3::new(1.0, 0.012, 0.2) };
                cube(&mut self.static_cubes, c, s, 0.0, jit(if hash(hx + dx as f64, hy) < 0.5 { VERGE } else { VERGE_D }, hash(hy, hx), 0.08));
                for k in 0..3 {
                    let hk = hash(hx * 7.0 + k as f64 + dx as f64 * 3.0, hy * 5.0 + dy as f64 * 3.0) as f32;
                    let along = (k as f32 - 1.0) * 0.32 + (hk - 0.5) * 0.15;
                    let p = if dx != 0 { Vec3::new(c.x + (hk - 0.5) * 0.1, h, c.z + along) } else { Vec3::new(c.x + along, h, c.z + (hk - 0.5) * 0.1) };
                    self.tuft(p, hk, TUFT, VERGE_D);
                }
            }
            // Ruts and a grassy middle on straight stretches.
            let open = |dx: i32, dy: i32| !rock(x + dx, y + dy);
            let along_x = open(-1, 0) && open(1, 0) && !open(0, -1) && !open(0, 1);
            let along_z = open(0, -1) && open(0, 1) && !open(-1, 0) && !open(1, 0);
            if along_x || along_z {
                for side in [-0.2f32, 0.2] {
                    let (p, s) = if along_x { (Vec3::new(xf, h + 0.004, yf + side), Vec3::new(1.0, 0.008, 0.09)) } else { (Vec3::new(xf + side, h + 0.004, yf), Vec3::new(0.09, 0.008, 1.0)) };
                    cube(&mut self.static_cubes, p, s, 0.0, lin(RUT));
                }
                if hash(hx * 3.0, hy * 3.0 + 1.0) < 0.65 {
                    let s = if along_x { Vec3::new(1.0, 0.01, 0.1) } else { Vec3::new(0.1, 0.01, 1.0) };
                    cube(&mut self.static_cubes, Vec3::new(xf, h + 0.005, yf), s, 0.0, jit(VERGE_D, hash(hx, hy + 4.0), 0.1));
                    let hk = hash(hx + 9.0, hy * 2.0) as f32;
                    let off = (hk - 0.5) * 0.6;
                    self.tuft(if along_x { Vec3::new(xf + off, h, yf) } else { Vec3::new(xf, h, yf + off) }, hk, TUFT, VERGE_D);
                }
            }
            // Fallen leaves, pebbles, and now and then a mushroom or a twig.
            for k in 0..3 {
                let hk = hash(hx * 11.0 + k as f64, hy * 13.0 - k as f64) as f32;
                let hj = hash(hy * 17.0 + k as f64, hx * 5.0) as f32;
                let c = lin(LEAVES[(hk * 4.0) as usize % 4]);
                cube(&mut self.static_cubes, Vec3::new(xf + (hk - 0.5) * 0.8, h + 0.008, yf + (hj - 0.5) * 0.8), Vec3::new(0.06, 0.006, 0.045), hk * 9.0, c);
            }
            if hash(hx * 2.0 + 5.0, hy) < 0.45 {
                let hk = hash(hx, hy * 7.0) as f32;
                cube(&mut self.static_cubes, Vec3::new(xf + (hk - 0.5) * 0.6, h + 0.02, yf + (0.5 - hk) * 0.5), Vec3::new(0.07, 0.04, 0.06), hk * 5.0, lin(PEBBLE));
            }
            let m = hash(hx * 5.0, hy * 5.0 + 3.0);
            if m < 0.09 {
                // A red toadstool by the verge.
                let a = (m * 70.0) as f32;
                let p = Vec3::new(xf + a.cos() * 0.36, h, yf + a.sin() * 0.36);
                cube(&mut self.static_cubes, p + Vec3::new(0.0, 0.035, 0.0), Vec3::new(0.025, 0.07, 0.025), 0.0, lin(0xe8e0d0));
                cube(&mut self.static_cubes, p + Vec3::new(0.0, 0.075, 0.0), Vec3::new(0.08, 0.03, 0.08), a, lin(0xc8262e));
                cube(&mut self.static_cubes, p + Vec3::new(0.012, 0.092, 0.0), Vec3::new(0.018, 0.006, 0.018), 0.0, lin(0xf4f4f0));
            } else if m > 0.9 {
                let a = (m * 40.0) as f32;
                cube(&mut self.static_cubes, Vec3::new(xf + (m as f32 - 0.95) * 4.0, h + 0.012, yf), Vec3::new(0.3, 0.02, 0.025), a, lin(0x5a3a1c));
            }
        }

        // ---- ramps: mossy stone steps toward the higher neighbor
        for (x, y, _) in grid.cells() {
            if !grid.is_ramp(x, y) {
                continue;
            }
            let low = grid.elevation(x, y);
            let mut up: Option<Dir> = None;
            for d in CARDINALS {
                let n = grid.neighbor(P::new(x, y), d);
                if grid.get(n.x, n.y) != Tile::Rock && !grid.is_ramp(n.x, n.y) && grid.elevation(n.x, n.y) > low {
                    up = Some(d);
                }
            }
            let up = up.unwrap_or_else(|| {
                let mut u = Dir::Right;
                for d in CARDINALS {
                    let n = grid.neighbor(P::new(x, y), d);
                    if grid.is_ramp(n.x, n.y) && grid.elevation(n.x, n.y) > low {
                        u = d;
                    }
                }
                u
            });
            let (vx, vy) = up.vec();
            for i in 0..3 {
                let top = low as f32 * STEP + ((i + 1) as f32 / 3.0) * STEP;
                let thick = top + 0.1;
                let o = (i as f32 - 1.0) / 3.0;
                let h = hash(x as f64 + i as f64, y as f64);
                let m = Mat4::from_translation(Vec3::new(x as f32 + vx as f32 * o, (top - 0.1) / 2.0, y as f32 + vy as f32 * o))
                    * Mat4::from_scale(Vec3::new(if vx != 0 { 1.0 / 3.0 } else { 1.0 }, thick, if vy != 0 { 1.0 / 3.0 } else { 1.0 }));
                self.static_cubes.push(Instance::new(m, jit(STEP_STONE, h, 0.1), 1.0, Vec3::ZERO, SHADE_LIT));
                // A lip of moss along each step's front edge.
                let lip = Vec3::new(x as f32 + vx as f32 * (o - 1.0 / 6.0), top + 0.005, y as f32 + vy as f32 * (o - 1.0 / 6.0));
                let s = if vx != 0 { Vec3::new(0.05, 0.012, 0.9) } else { Vec3::new(0.9, 0.012, 0.05) };
                cube(&mut self.static_cubes, lip, s, 0.0, lin(MOSS));
            }
        }

        // ---- pits: a dark drop with a crumbling earth rim
        for (x, y, t) in grid.cells() {
            if t != Tile::Pit {
                continue;
            }
            let bottom = Mat4::from_translation(Vec3::new(x as f32, -0.9, y as f32)) * Mat4::from_scale(Vec3::new(1.0, 0.1, 1.0));
            self.static_cubes.push(Instance::new(bottom, lin(PIT), 1.0, Vec3::ZERO, SHADE_LIT));
            let rim = Mat4::from_translation(Vec3::new(x as f32, -0.03, y as f32)) * Mat4::from_scale(Vec3::new(1.02, 0.06, 1.02));
            self.static_cubes.push(Instance::new(rim, lin(PIT_RIM), 1.0, Vec3::ZERO, SHADE_LIT));
        }

        // ---- the woods: mounds, stone fences, ferns, bushes; trees and lanterns after
        let species = [super::super::props::pine(), super::super::props::oak(), super::super::props::birch()];
        let extents: Vec<(f32, f32)> = species.iter().map(tree_extent).collect();
        let (lantern, lantern_glass) = super::super::props::lantern();
        let lantern_glow = lin(TORCH_LIGHT) * 0.8;
        let mut trees: Vec<(P, Vec3, f32, f32, usize)> = Vec::new();
        let mut lanterns: Vec<(P, Dir)> = Vec::new();
        for (x, y, t) in grid.cells() {
            if t != Tile::Rock {
                continue;
            }
            let (xf, yf) = (x as f32, y as f32);
            let (hx, hy) = (x as f64, y as f64);
            let base = floor_y(x, y);
            let top = base + MOUND_H;
            let start = self.static_cubes.len();
            cube(&mut self.static_cubes, Vec3::new(xf, (top - 0.1) / 2.0, yf), Vec3::new(1.0, top + 0.1, 1.0), 0.0, jit(if hash(hx + 11.0, hy + 5.0) < 0.5 { MOUND } else { MOUND_D }, hash(hx, hy), 0.08));
            let roads: Vec<Dir> = CARDINALS.iter().copied().filter(|d| {
                let (dx, dy) = d.vec();
                grid.in_bounds(x + dx, y + dy) && !rock(x + dx, y + dy)
            }).collect();
            for &d in &roads {
                self.build_fence(xf, yf, top, d, hx, hy);
            }
            // Ferns on the mound.
            for k in 0..2 {
                let hk = hash(hx * 3.0 + k as f64 * 5.0, hy * 7.0 + 1.0) as f32;
                let hj = hash(hy * 3.0 + k as f64, hx * 5.0 + 2.0) as f32;
                self.fern(Vec3::new(xf + (hk - 0.5) * 0.5, top, yf + (hj - 0.5) * 0.5), hk);
            }
            // Bushes tucked behind the fence.
            if !roads.is_empty() {
                let bushes = 1 + (hash(hx * 9.0, hy * 2.0) * 1.9) as usize;
                for k in 0..bushes {
                    let hk = hash(hx * 13.0 + k as f64 * 3.0, hy * 11.0) as f32;
                    let d = roads[k % roads.len()];
                    let (dx, dy) = d.vec();
                    let slide = (hk - 0.5) * 0.6;
                    let p = Vec3::new(xf + dx as f32 * 0.12 + dy as f32 * slide, top, yf + dy as f32 * 0.12 + dx as f32 * slide);
                    self.build_bush(p, 0.8 + hk * 0.45, true);
                }
            }
            self.rock_index.insert(tile_key(x, y), (start, self.static_cubes.len() - start));
            // Trees: dense in the heart of the woods, sparser by the roads.
            let pick = hash(hx * 5.0 + 1.0, hy * 3.0 + 2.0);
            let edge = !roads.is_empty();
            if edge && hash(hx * 3.0, hy * 7.0) < 0.07 {
                lanterns.push((P::new(x, y), roads[0]));
            } else if pick < if edge { 0.5 } else { 0.75 } {
                let kind = match hash(hx * 7.0 + 3.0, hy * 2.0) {
                    s if s < 0.55 => 0,
                    s if s < 0.82 => 1,
                    _ => 2,
                };
                let scale = 0.85 + hash(hy, hx) as f32 * 0.6 + if edge { 0.0 } else { 0.2 };
                let off = if edge { Vec3::ZERO } else { Vec3::new((hash(hx + 1.0, hy) as f32 - 0.5) * 0.3, 0.0, (hash(hx, hy + 1.0) as f32 - 0.5) * 0.3) };
                trees.push((P::new(x, y), Vec3::new(xf, top - 0.02, yf) + off, hash(hx, hy) as f32 * TAU, scale, kind));
            }
        }
        for (key, pos, rot, scale, kind) in trees {
            let start = self.static_cubes.len();
            push_field(&mut self.static_cubes, &species[kind], pos, rot, scale, white, Vec3::ZERO, SHADE_LIT);
            let (r, h) = extents[kind];
            self.tree_index.insert(key, self.trees.len());
            self.trees.push(Tree { range: (start, self.static_cubes.len() - start), base: pos, radius: r * scale + 0.05, height: h * scale, fade: 1.0, alive: true });
        }
        for (spot, d) in lanterns {
            let (dx, dy) = d.vec();
            let pos = Vec3::new(spot.x as f32 + dx as f32 * 0.3, floor_y(spot.x, spot.y) + MOUND_H, spot.y as f32 + dy as f32 * 0.3);
            let start = self.static_cubes.len();
            push_field(&mut self.static_cubes, &lantern, pos, 0.0, 1.0, white, Vec3::ZERO, SHADE_LIT);
            push_field(&mut self.static_cubes, &lantern_glass, pos, 0.0, 1.0, white, lantern_glow, SHADE_LIT);
            self.torches.insert(spot, (pos, (start, self.static_cubes.len() - start)));
        }
    }

    /// A dry-stone fence along the road-facing side `d` of a wall tile: two staggered courses and mossy caps.
    fn build_fence(&mut self, x: f32, y: f32, top: f32, d: Dir, hx: f64, hy: f64) {
        let (dx, dy) = d.vec();
        let turn = if dx != 0 { PI / 2.0 } else { 0.0 };
        // Along the edge: local u runs along it, the fence sits 0.38 out toward the road.
        let at = |u: f32, h: f32| Vec3::new(x + dx as f32 * 0.38 + if dx != 0 { 0.0 } else { u }, h, y + dy as f32 * 0.38 + if dx != 0 { u } else { 0.0 });
        for course in 0..2 {
            let shift = if course % 2 == 0 { 0.0 } else { 0.11 };
            let mut u: f32 = -0.5 + shift;
            let mut k = 0;
            while u < 0.5 {
                let hk = hash(hx * 7.0 + k as f64 * 1.3 + course as f64 * 5.0 + dx as f64, hy * 3.0 + dy as f64 * 7.0) as f32;
                let len = 0.18 + hk * 0.08;
                let (a, b) = (u.max(-0.5), (u + len).min(0.5));
                if b - a > 0.05 {
                    let hgt = COURSE * (0.85 + hk * 0.3);
                    let c = jit(STONES[(hk * 4.0) as usize % 4], hk as f64, 0.08);
                    cube(&mut self.static_cubes, at((a + b) / 2.0, top + course as f32 * COURSE + hgt / 2.0), Vec3::new(b - a - 0.015, hgt, 0.22 - hk * 0.04), turn + (hk - 0.5) * 0.12, c);
                }
                u += len;
                k += 1;
            }
        }
        // Flat cap stones, some with moss on top.
        for k in 0..3 {
            let hk = hash(hx * 3.0 + k as f64 + dx as f64 * 11.0, hy * 5.0 + dy as f64) as f32;
            let u = -0.33 + k as f32 * 0.33;
            let h = top + COURSE * 2.0 + 0.03;
            let c = if hk < 0.4 { lin(MOSS) } else { jit(STONES[k % 4], hk as f64, 0.08) };
            cube(&mut self.static_cubes, at(u, h), Vec3::new(0.3, 0.06, 0.26), turn + (hk - 0.5) * 0.2, c);
        }
    }

    /// A tuft of grass blades.
    fn tuft(&mut self, base: Vec3, h: f32, tip: u32, root: u32) {
        for b in 0..3 {
            let hb = hash(h as f64 * 91.0 + b as f64, h as f64 * 37.0 + 3.0) as f32;
            let tall = 0.07 + hb * 0.08;
            let m = Mat4::from_translation(base + Vec3::new((b as f32 - 1.0) * 0.03, 0.0, (hb - 0.5) * 0.04))
                * Mat4::from_rotation_y(h * 6.0 + b as f32 * 2.1)
                * Mat4::from_rotation_z((hb - 0.5) * 0.5)
                * Mat4::from_translation(Vec3::new(0.0, tall / 2.0, 0.0))
                * Mat4::from_scale(Vec3::new(0.025, tall, 0.025));
            self.static_cubes.push(Instance::new(m, lin(if b == 1 { tip } else { root }), 1.0, Vec3::ZERO, SHADE_LIT));
        }
    }

    /// A fern: fronds arching out from the middle.
    fn fern(&mut self, base: Vec3, h: f32) {
        for k in 0..5 {
            let a = k as f32 / 5.0 * TAU + h * 3.0;
            let len = 0.18 + hash(h as f64 * 13.0 + k as f64, 2.0) as f32 * 0.1;
            let m = Mat4::from_translation(base)
                * Mat4::from_rotation_y(a)
                * Mat4::from_rotation_x(-0.9)
                * Mat4::from_translation(Vec3::new(0.0, len / 2.0, 0.0))
                * Mat4::from_scale(Vec3::new(0.05, len, 0.015));
            self.static_cubes.push(Instance::new(m, lin(if k % 2 == 0 { FERN } else { FERN_L }), 1.0, Vec3::ZERO, SHADE_LIT));
        }
    }
}
