//! Dungeon I dressed as an English landscape garden (`logic::garden` plans
//! it): channels running from the Krampus fountain (`fountain.rs`) in
//! stone-edged beds with lily pads and reeds, waterfalls off the island's
//! edges, balustraded stone bridges,
//! a churchyard behind iron railings, flower beds inside clipped box hedges,
//! round classical temples on the lawns and small wayside shrines with
//! candles. The water itself is drawn each frame (it moves); the rest is
//! static level geometry.

use super::*;
use crate::logic::garden::GardenPlan;
use crate::render::characters::Sculpt;
use crate::render::props::mesh_from;

mod pal {
    pub const MUD: u32 = 0x1e1a14;
    pub const PEBBLE: [u32; 3] = [0x6a6a62, 0x57574f, 0x7c7a70];
    pub const BANK: [u32; 3] = [0x8a8678, 0x7a766a, 0x9a9486];
    pub const COPING: u32 = 0xa8a292;
    pub const BRIDGE: u32 = 0xbab2a0;
    pub const BRIDGE_D: u32 = 0x9a927f;
    pub const PAD: u32 = 0x2f6a2a;
    pub const PAD_L: u32 = 0x4a8a38;
    pub const LILY: [u32; 2] = [0xf0d8e0, 0xe890b0];
    pub const REED: u32 = 0x5a7a34;
    pub const CATTAIL: u32 = 0x4a3020;
    pub const GRAVE_EARTH: u32 = 0x2a2a1c;
    pub const GRAVE_GRASS: u32 = 0x33422a;
    pub const HEADSTONE: [u32; 4] = [0x8a8a86, 0x76766f, 0x9c9a92, 0x686862];
    pub const LICHEN: u32 = 0x6a7a3a;
    pub const IRON: u32 = 0x24262a;
    pub const DEAD_WOOD: u32 = 0x2e2622;
    pub const SOIL: u32 = 0x3a2a1c;
    pub const BOX: u32 = 0x24521f;
    pub const BOX_L: u32 = 0x2f6428;
    pub const LEAF: u32 = 0x2f5a2a;
    pub const STEM: u32 = 0x3e6a2e;
    pub const BLOOMS: [u32; 6] = [0xc8283a, 0xe86a8a, 0x8a6ad0, 0xece8dc, 0xe8c83a, 0x4a6ad8];
    pub const ROCK: [u32; 3] = [0x5e6258, 0x4e5249, 0x6e7066];
    pub const CAVE: u32 = 0x050606;
    pub const IVY: u32 = 0x2a5a24;
    pub const MIST: u32 = 0x7c8a98;
}
use pal::*;

/// How far the water surface lies below the floor it runs beside.
pub const WATER_DROP: f32 = 0.13;
/// The river bed, below the floor.
const BED_DROP: f32 = 0.36;

fn cube(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot_y: f32, color: Vec3) {
    let m = Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(rot_y), pos);
    out.push(Instance::new(m, color, 1.0, Vec3::ZERO, SHADE_LIT));
}

fn cube_shade(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot: Quat, color: Vec3, emissive: Vec3, shade: f32) {
    let m = Mat4::from_scale_rotation_translation(scale, rot, pos);
    out.push(Instance::new(m, color, 1.0, emissive, shade));
}

fn jit(hex: u32, h: f64, amount: f32) -> Vec3 {
    lin(hex) * (1.0 - amount + h as f32 * amount * 2.0)
}

/// What the garden has made of a wall tile.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Feature {
    Water,
    Fountain,
    Grave,
    Flowers,
    Temple,
    Shrine,
}

pub fn features(plan: &GardenPlan) -> HashMap<P, Feature> {
    let mut map = HashMap::new();
    for p in &plan.graveyard {
        map.insert(*p, Feature::Grave);
    }
    for p in &plan.flowers {
        map.insert(*p, Feature::Flowers);
    }
    for t in &plan.temples {
        for p in t {
            map.insert(*p, Feature::Temple);
        }
    }
    for p in &plan.shrines {
        map.insert(*p, Feature::Shrine);
    }
    for r in plan.water() {
        map.insert(r.p, Feature::Water);
    }
    if let Some(f) = plan.fountain {
        map.insert(f.p, Feature::Fountain);
    }
    map
}

// ------------------------------------------------------------------ models

/// A round classical temple (a monopteros): stepped plinth, eight columns, a green copper dome, a statue inside.
pub fn temple() -> (Mesh, Mesh) {
    const STONE: u32 = 0xd6d0be;
    const STONE_D: u32 = 0xb4ae9c;
    const MARBLE: u32 = 0xeae6dc;
    const DOME: u32 = 0x5f8f7c;
    const DOME_L: u32 = 0x7aa894;
    const GILT: u32 = 0xe8c050;
    const FLAME: u32 = 0xffd070;
    let mut s = Sculpt::new(27, 32, 27);
    let c = 13.0f32;
    let disk = |s: &mut Sculpt, y: i32, r: f32, color: u32| {
        for z in 0..27 {
            for x in 0..27 {
                let d = ((x as f32 - c).powi(2) + (z as f32 - c).powi(2)).sqrt();
                if d <= r {
                    s.set(x, y, z, color);
                }
            }
        }
    };
    disk(&mut s, 0, 12.6, STONE_D);
    disk(&mut s, 1, 11.4, STONE);
    disk(&mut s, 2, 10.3, STONE_D);
    disk(&mut s, 3, 9.6, MARBLE);
    for k in 0..8 {
        let a = k as f32 / 8.0 * TAU + 0.39;
        let (cx, cz) = ((c + 7.6 * a.cos()).round() as i32, (c + 7.6 * a.sin()).round() as i32);
        s.cuboid(cx - 1, 4, cz - 1, cx + 1, 4, cz + 1, STONE_D);
        s.cuboid(cx, 5, cz, cx, 18, cz, STONE);
        s.cuboid(cx - 1, 19, cz - 1, cx + 1, 19, cz + 1, STONE_D);
    }
    for y in 20..=21 {
        for z in 0..27 {
            for x in 0..27 {
                let d = ((x as f32 - c).powi(2) + (z as f32 - c).powi(2)).sqrt();
                if (6.6..=9.8).contains(&d) {
                    s.set(x, y, z, if y == 20 && (x + z) % 2 == 0 { STONE_D } else { STONE });
                }
            }
        }
    }
    s.ellipsoid((c, 21.0, c), (9.0, 7.6, 9.0), 22, DOME);
    s.paint(|x, _y, z, col| {
        let a = (z as f32 - c).atan2(x as f32 - c);
        if col == DOME && ((a / TAU * 8.0).fract() < 0.12) {
            Some(DOME_L)
        } else {
            None
        }
    });
    s.cuboid(12, 29, 12, 14, 29, 14, GILT);
    s.cuboid(13, 30, 13, 13, 31, 13, GILT);
    // The statue on its pedestal, and a votive flame at its feet.
    s.cuboid(12, 4, 12, 14, 7, 14, STONE_D);
    s.cuboid(12, 8, 13, 14, 12, 13, MARBLE);
    s.cuboid(13, 8, 12, 13, 12, 14, MARBLE);
    s.ellipsoid((13.0, 14.0, 13.0), (1.3, 1.5, 1.3), 13, MARBLE);
    s.set(11, 13, 13, MARBLE);
    s.set(15, 13, 13, MARBLE);
    s.set(13, 4, 16, FLAME);
    s.set(13, 5, 16, FLAME);
    split(mesh_from(&s.finish(0.03), 0.062), lin(FLAME))
}

/// A wayside shrine: a stone pillar with a candle in its niche, under a little slate roof and a cross.
pub fn wayside_shrine() -> (Mesh, Mesh) {
    const STONE: u32 = 0x9c988a;
    const STONE_D: u32 = 0x7e7a6e;
    const SLATE: u32 = 0x464a54;
    const WAX: u32 = 0xeee6cc;
    const FLAME: u32 = 0xffc860;
    let mut s = Sculpt::new(11, 20, 11);
    s.cuboid(2, 0, 2, 8, 1, 8, STONE_D);
    s.cuboid(3, 2, 3, 7, 11, 7, STONE);
    for y in 6..=9 {
        for x in 4..=6 {
            s.clear(x, y, 7);
            s.clear(x, y, 6);
        }
    }
    s.cuboid(4, 5, 6, 6, 5, 7, STONE_D);
    s.set(5, 6, 6, WAX);
    s.set(5, 7, 6, FLAME);
    for y in 12..=15 {
        let i = y - 12;
        s.cuboid(1 + i, y, 1, 9 - i, y, 9, SLATE);
    }
    s.cuboid(5, 16, 5, 5, 18, 5, STONE_D);
    s.cuboid(4, 17, 5, 6, 17, 5, STONE_D);
    split(mesh_from(&s.finish(0.04), 0.05), lin(FLAME))
}

/// A stone urn on a plinth, spilling flowers.
pub fn urn() -> Mesh {
    const STONE: u32 = 0xb0aa98;
    const STONE_D: u32 = 0x8e8878;
    let mut s = Sculpt::new(9, 16, 9);
    s.cuboid(1, 0, 1, 7, 1, 7, STONE_D);
    s.cuboid(2, 2, 2, 6, 5, 6, STONE);
    s.cuboid(1, 6, 1, 7, 6, 7, STONE_D);
    s.ellipsoid((4.0, 9.5, 4.0), (3.0, 3.0, 3.0), 7, STONE);
    s.cuboid(3, 12, 3, 5, 12, 5, STONE_D);
    s.cuboid(2, 13, 2, 6, 13, 6, STONE);
    for (x, z) in [(3, 3), (5, 4), (4, 5), (3, 5), (5, 3), (4, 3)] {
        s.set(x, 14, z, BLOOMS[(x + z) as usize % 5]);
        s.set(x, 13, z, LEAF);
    }
    mesh_from(&s.finish(0.04), 0.05)
}

/// Pulls the cubes of one color (a flame) out into their own mesh, so they can glow.
fn split(m: Mesh, key: Vec3) -> (Mesh, Mesh) {
    let (glow, rest): (Vec<_>, Vec<_>) = m.cubes.into_iter().partition(|(_, c)| (*c - key).length() < 0.02);
    (Mesh { size: m.size, cubes: rest }, Mesh { size: m.size, cubes: glow })
}

// ------------------------------------------------------------------- build

impl WorldView {
    /// The garden's static geometry. `feature` says what each wall tile became.
    pub(super) fn build_garden(&mut self, grid: &Grid, plan: &GardenPlan, feature: &HashMap<P, Feature>) {
        let floor_y = |x: i32, y: i32| grid.elevation(x, y) as f32 * STEP;
        let water: HashMap<P, i32> = plan.water().map(|r| (r.p, r.level)).collect();
        self.build_river(grid, plan, &water);
        for b in &plan.bridges {
            self.build_stone_bridge(grid, *b);
        }
        if let Some(f) = plan.fountain {
            self.build_fountain(grid, f, plan);
        }

        // Churchyard, flowers, temples' lawns, shrines.
        let grave_center = plan.graveyard_center().map(|(x, y)| P::new(x.round() as i32, y.round() as i32));
        if let (Some((cx, cz)), Some(c)) = (plan.graveyard_center(), grave_center) {
            self.wisps = Some(Vec3::new(cx, floor_y(c.x, c.y) + 0.1, cz));
        }
        let mut tree_done = false;
        let mut tomb_done = false;
        let mut candles = 0;
        let mut graves: Vec<P> = plan.graveyard.clone();
        graves.sort_by_key(|p| (p.y, p.x));
        for p in graves {
            let first_tree = !tree_done && grave_center.is_some_and(|c| (p.x - c.x).abs() + (p.y - c.y).abs() <= 2);
            let tomb = !tomb_done && !first_tree && p.y == plan.graveyard.iter().map(|q| q.y).max().unwrap_or(0) && p.x % 2 == 1;
            let candle = candles < 3 && !first_tree && !tomb && hash(p.x as f64 * 5.0, p.y as f64 * 3.0) < 0.35;
            self.build_grave_tile(grid, p, first_tree, tomb, candle);
            tree_done |= first_tree;
            tomb_done |= tomb;
            candles += candle as usize;
        }
        for p in &plan.flowers {
            self.build_flower_tile(grid, *p, feature);
        }
        let (temple, temple_flame) = temple();
        for block in &plan.temples {
            let n = block.len() as f32;
            let cx = block.iter().map(|p| p.x as f32).sum::<f32>() / n;
            let cz = block.iter().map(|p| p.y as f32).sum::<f32>() / n;
            let base = block.iter().map(|p| floor_y(p.x, p.y)).fold(0.0, f32::max) + forest::MOUND_H;
            let pos = Vec3::new(cx, base - 0.01, cz);
            let start = self.static_cubes.len();
            push_field(&mut self.static_cubes, &temple, pos, 0.0, 1.0, Vec3::ONE, Vec3::ZERO, SHADE_LIT);
            // It fades like a tree when it comes between the camera and Ragnar.
            self.trees.push(Tree {
                range: (start, self.static_cubes.len() - start),
                base: pos,
                radius: 0.85,
                height: 2.0,
                fade: 1.0,
                alive: true,
                shades: None,
            });
            push_field(&mut self.static_cubes, &temple_flame, pos, 0.0, 1.0, Vec3::ONE, lin(0xffb050) * 2.5, SHADE_LIT);
            self.candles.push(pos + Vec3::new(0.0, 0.35, 0.18));
        }
        let (shrine, shrine_flame) = wayside_shrine();
        let urn = urn();
        for (i, p) in plan.shrines.iter().enumerate() {
            let top = floor_y(p.x, p.y) + forest::MOUND_H;
            // Face the path.
            let face = CARDINALS
                .iter()
                .copied()
                .find(|d| {
                    let (dx, dy) = d.vec();
                    let t = grid.get(p.x + dx, p.y + dy);
                    t != Tile::Rock && t != Tile::Nest
                })
                .unwrap_or(Dir::Down);
            let (dx, dy) = face.vec();
            let rot = (dx as f32).atan2(dy as f32);
            let pos = Vec3::new(p.x as f32 + dx as f32 * 0.12, top - 0.01, p.y as f32 + dy as f32 * 0.12);
            let start = self.static_cubes.len();
            if i % 3 == 2 {
                push_field(&mut self.static_cubes, &urn, pos, rot, 1.0, Vec3::ONE, Vec3::ZERO, SHADE_LIT);
            } else {
                push_field(&mut self.static_cubes, &shrine, pos, rot, 1.0, Vec3::ONE, Vec3::ZERO, SHADE_LIT);
                push_field(&mut self.static_cubes, &shrine_flame, pos, rot, 1.0, Vec3::ONE, lin(0xffb050) * 2.5, SHADE_LIT);
                self.candles.push(pos + Vec3::new(dx as f32 * 0.1, 0.4, dy as f32 * 0.1));
            }
            // A bomb takes the shrine with the wall.
            let key = tile_key(p.x, p.y);
            if let Some((s0, len)) = self.rock_index.get(&key).copied() {
                if s0 + len == start {
                    self.rock_index.insert(key, (s0, self.static_cubes.len() - s0));
                }
            }
        }
    }

    /// Ground mist: two thin sheets over every tile, a third over water and graves.
    pub(super) fn build_mist(&mut self, grid: &Grid, feature: &HashMap<P, Feature>) {
        let floor_y = |x: i32, y: i32| grid.elevation(x, y) as f32 * STEP;
        let water: HashMap<P, i32> = self.garden.as_ref().map(|g| g.water().map(|r| (r.p, r.level)).collect()).unwrap_or_default();
        self.mist.clear();
        for (x, y, t) in grid.cells() {
            let p = P::new(x, y);
            let mut floor = floor_y(x, y);
            if let Some(&level) = water.get(&p) {
                floor = level as f32 * STEP - WATER_DROP;
            } else if t == Tile::Pit {
                floor = -0.3;
            }
            let thick = matches!(feature.get(&p), Some(Feature::Water) | Some(Feature::Grave));
            let layers: &[(f32, f32)] =
                if thick { &[(0.1, 0.55), (0.28, 0.48), (0.5, 0.36), (0.8, 0.22)] } else { &[(0.12, 0.34), (0.34, 0.27), (0.62, 0.16)] };
            for (k, &(lift, alpha)) in layers.iter().enumerate() {
                let m = Mat4::from_translation(Vec3::new(x as f32, floor + lift, y as f32)) * Mat4::from_scale(Vec3::new(1.0, 0.002, 1.0));
                self.mist.push(Instance::new(m, lin(MIST), alpha, Vec3::new(20.0 + k as f32, lift, 0.0), SHADE_CLOUD));
            }
        }
    }

    /// The channels: bed, stone banks, weirs, lily pads, reeds; a grotto where a stream rises from the ground, rocks where one pours off the edge.
    pub(super) fn build_river(&mut self, grid: &Grid, plan: &GardenPlan, water: &HashMap<P, i32>) {
        let fountain = plan.fountain.map(|f| f.p);
        for channel in &plan.channels {
            let n = channel.len();
            let off_edge = channel.last().is_some_and(|r| r.p.x == 0 || r.p.y == 0 || r.p.x == grid.width - 1 || r.p.y == grid.height - 1);
            for (i, r) in channel.iter().enumerate() {
                let (x, y) = (r.p.x, r.p.y);
                let (xf, yf) = (x as f32, y as f32);
                let (hx, hy) = (x as f64, y as f64);
                let lv = r.level as f32 * STEP;
                let bed = lv - BED_DROP;
                // The bed: a column of mud up to a pebbly floor.
                cube(
                    &mut self.static_cubes,
                    Vec3::new(xf, (bed - 1.2) / 2.0, yf),
                    Vec3::new(1.0, bed + 1.2, 1.0),
                    0.0,
                    jit(MUD, hash(hx, hy), 0.1),
                );
                for k in 0..4 {
                    let hk = hash(hx * 7.0 + k as f64, hy * 3.0) as f32;
                    let hj = hash(hy * 5.0 + k as f64, hx * 2.0) as f32;
                    cube(
                        &mut self.static_cubes,
                        Vec3::new(xf + (hk - 0.5) * 0.8, bed + 0.02, yf + (hj - 0.5) * 0.7),
                        Vec3::new(0.08 + hk * 0.06, 0.04, 0.07 + hj * 0.05),
                        hk * 3.0,
                        lin(PEBBLE[k % 3]),
                    );
                }
                // Banks wherever the channel meets something that is not water (the fountain has its own wall).
                let bridge = grid.is_bridge(x, y);
                for d in CARDINALS {
                    let (dx, dy) = d.vec();
                    let np = P::new(x + dx, y + dy);
                    if let Some(&nl) = water.get(&np) {
                        // A weir where the channel steps down into the next reach.
                        if nl < r.level {
                            let low = nl as f32 * STEP - BED_DROP;
                            let at = Vec3::new(xf + dx as f32 * 0.5, (low + lv - WATER_DROP - 0.02) / 2.0, yf + dy as f32 * 0.5);
                            let s = if dx != 0 {
                                Vec3::new(0.16, lv - WATER_DROP - 0.02 - low, 1.0)
                            } else {
                                Vec3::new(1.0, lv - WATER_DROP - 0.02 - low, 0.16)
                            };
                            cube(&mut self.static_cubes, at, s, 0.0, lin(BANK[0]));
                            let lip = Vec3::new(xf + dx as f32 * 0.5, lv - WATER_DROP - 0.03, yf + dy as f32 * 0.5);
                            cube(
                                &mut self.static_cubes,
                                lip,
                                if dx != 0 { Vec3::new(0.2, 0.05, 1.02) } else { Vec3::new(1.02, 0.05, 0.2) },
                                0.0,
                                lin(COPING),
                            );
                        }
                        continue;
                    }
                    // A bridge's deck runs on over the path; off the edge there is nothing to bank.
                    if !grid.in_bounds(np.x, np.y) || Some(np) == fountain || (bridge && grid.get(np.x, np.y) != Tile::Rock) {
                        continue;
                    }
                    let turn = if dx != 0 { PI / 2.0 } else { 0.0 };
                    let top = lv + 0.03;
                    let at = |u: f32, hgt: f32| {
                        Vec3::new(
                            xf + dx as f32 * 0.45 + if dx != 0 { 0.0 } else { u },
                            hgt,
                            yf + dy as f32 * 0.45 + if dx != 0 { u } else { 0.0 },
                        )
                    };
                    // Squared stones in courses, and a coping along the top.
                    let mut u = -0.5;
                    let mut k = 0;
                    while u < 0.5 {
                        let hk = hash(hx * 3.0 + k as f64 + dx as f64 * 5.0, hy * 7.0 + dy as f64 * 3.0) as f32;
                        let len = (0.2 + hk * 0.12).min(0.5 - u);
                        cube(
                            &mut self.static_cubes,
                            at(u + len / 2.0, (bed + top) / 2.0),
                            Vec3::new(len - 0.01, top - bed, 0.1),
                            turn,
                            jit(BANK[k % 3], hk as f64, 0.06),
                        );
                        u += len;
                        k += 1;
                    }
                    cube(&mut self.static_cubes, at(0.0, top + 0.025), Vec3::new(1.0, 0.05, 0.16), turn, lin(COPING));
                    // Reeds along some banks.
                    if !bridge && hash(hx * 11.0 + dx as f64, hy * 13.0 + dy as f64) < 0.45 {
                        for b in 0..5 {
                            let hb = hash(hx * 17.0 + b as f64, hy + dy as f64 * 3.0 + dx as f64) as f32;
                            let tall = 0.22 + hb * 0.2;
                            let base = at((hb - 0.5) * 0.8, lv - WATER_DROP) - Vec3::new(dx as f32, 0.0, dy as f32) * 0.09;
                            let m = Mat4::from_translation(base)
                                * Mat4::from_rotation_z((hb - 0.5) * 0.3)
                                * Mat4::from_translation(Vec3::new(0.0, tall / 2.0, 0.0))
                                * Mat4::from_scale(Vec3::new(0.014, tall, 0.014));
                            self.static_cubes.push(Instance::new(m, lin(REED), 1.0, Vec3::ZERO, SHADE_LIT));
                            if b % 2 == 0 {
                                cube(
                                    &mut self.static_cubes,
                                    base + Vec3::new(0.0, tall * 0.85, 0.0),
                                    Vec3::new(0.028, 0.07, 0.028),
                                    0.0,
                                    lin(CATTAIL),
                                );
                            }
                        }
                    }
                }
                // Lily pads on the quiet reaches, and in a pool where a channel ends short of the edge.
                let level = |j: usize| channel.get(j).map(|t| t.level);
                let calm =
                    !bridge && i > 0 && level(i - 1) == Some(r.level) && (level(i + 1) == Some(r.level) || (i + 1 == n && !off_edge));
                if calm && hash(hx * 3.0 + 1.0, hy * 5.0) < 0.55 {
                    for k in 0..3 {
                        let hk = hash(hx * 19.0 + k as f64, hy * 23.0) as f32;
                        let hj = hash(hy * 29.0 + k as f64, hx * 31.0) as f32;
                        let p = Vec3::new(xf + (hk - 0.5) * 0.6, lv - WATER_DROP + 0.006, yf + (hj - 0.5) * 0.5);
                        let size = 0.13 + hk * 0.08;
                        cube(&mut self.static_cubes, p, Vec3::new(size, 0.012, size), hk * TAU, lin(if k % 2 == 0 { PAD } else { PAD_L }));
                        if hj < 0.35 {
                            cube(
                                &mut self.static_cubes,
                                p + Vec3::new(0.0, 0.03, 0.0),
                                Vec3::new(0.05, 0.04, 0.05),
                                hk * 2.0,
                                lin(LILY[k % 2]),
                            );
                            cube(&mut self.static_cubes, p + Vec3::new(0.0, 0.055, 0.0), Vec3::new(0.025, 0.02, 0.025), 0.0, lin(0xf2d060));
                        }
                    }
                }
            }

            // A stream with no fountain rises from a grotto: rough rocks arched over a black cave mouth, ivy trailing.
            if fountain.is_none() {
                if let Some(src) = channel.first() {
                    let (xf, yf) = (src.p.x as f32, src.p.y as f32);
                    let lv = src.level as f32 * STEP;
                    let inward = if src.p.x == grid.width - 1 { 1.0 } else { -1.0 };
                    cube(&mut self.static_cubes, Vec3::new(xf + inward * 0.42, lv + 0.2, yf), Vec3::new(0.1, 0.9, 0.9), 0.0, lin(CAVE));
                    for k in 0..14 {
                        let hk = hash(k as f64 * 3.0, 7.0) as f32;
                        let hj = hash(k as f64 * 5.0, 11.0) as f32;
                        let a = k as f32 / 13.0 * PI;
                        let (side, hgt) = (a.cos() * 0.55, lv - 0.2 + a.sin() * 0.9);
                        let size = 0.24 + hk * 0.16;
                        cube(
                            &mut self.static_cubes,
                            Vec3::new(xf + inward * (0.2 + hj * 0.2), hgt, yf + side),
                            Vec3::new(size, size * 0.85, size),
                            hk * 3.0,
                            jit(ROCK[k % 3], hj as f64, 0.08),
                        );
                        if hj < 0.4 {
                            cube(
                                &mut self.static_cubes,
                                Vec3::new(xf + inward * 0.05, hgt - 0.12, yf + side),
                                Vec3::new(0.05, 0.24, 0.12),
                                0.0,
                                lin(IVY),
                            );
                        }
                    }
                }
            }
            // Rocks either side of the falls where a channel leaves the island.
            if off_edge && n >= 2 {
                let out = channel[n - 1];
                let prev = channel[n - 2].p;
                let (ax, az) = ((out.p.x - prev.x) as f32, (out.p.y - prev.y) as f32);
                let (xf, yf) = (out.p.x as f32, out.p.y as f32);
                let lv = out.level as f32 * STEP;
                for side in [-1.0f32, 1.0] {
                    for k in 0..3 {
                        let hk = hash(k as f64 + side as f64 * 3.0 + xf as f64, 17.0 + yf as f64) as f32;
                        let along = 0.3 - k as f32 * 0.25;
                        let p = Vec3::new(xf + ax * along - az * side * 0.5, lv - 0.05 + hk * 0.1, yf + az * along + ax * side * 0.5);
                        cube(&mut self.static_cubes, p, Vec3::new(0.3, 0.28 + hk * 0.15, 0.26), hk * 2.0, jit(ROCK[k], hk as f64, 0.08));
                    }
                }
            }
        }
    }

    /// A stone bridge with a balustrade on each side over an arch; up a ramp it climbs in steps.
    pub(super) fn build_stone_bridge(&mut self, grid: &Grid, b: P) {
        let (xf, yf) = (b.x as f32, b.y as f32);
        let (hx, hy) = (b.x as f64, b.y as f64);
        let low = grid.elevation(b.x, b.y) as f32 * STEP;
        // Which way the path runs (north-south for a river flowing west), and which end is higher.
        // A path two tiles wide crosses on two bridge tiles side by side, one after the other along the water.
        let path = |x: i32, y: i32| grid.get(x, y) != Tile::Rock && !grid.is_bridge(x, y);
        let along_z = !(path(b.x - 1, b.y) || path(b.x + 1, b.y)) || path(b.x, b.y - 1) || path(b.x, b.y + 1);
        let (ax, az) = if along_z { (0.0f32, 1.0f32) } else { (1.0, 0.0) };
        let hi_end = if grid.is_ramp(b.x, b.y) {
            let a = if along_z { grid.elevation(b.x, b.y + 1) } else { grid.elevation(b.x + 1, b.y) };
            let c = if along_z { grid.elevation(b.x, b.y - 1) } else { grid.elevation(b.x - 1, b.y) };
            if a > c {
                1.0
            } else {
                -1.0
            }
        } else {
            0.0
        };
        let rise = if hi_end != 0.0 { STEP } else { 0.0 };
        // Deck height at a point u (-0.5..0.5) along the path.
        let deck = |u: f32| if hi_end == 0.0 { low } else { low + rise * (u * hi_end + 0.5).clamp(0.0, 1.0) };
        let at = |u: f32, side: f32, hgt: f32| Vec3::new(xf + ax * u + az * side, hgt, yf + az * u + ax * side);
        let dims = |across: f32, hgt: f32, along: f32| if along_z { Vec3::new(across, hgt, along) } else { Vec3::new(along, hgt, across) };
        // The deck: three paving slabs (steps when climbing).
        for i in 0..3 {
            let u = (i as f32 - 1.0) / 3.0;
            // Climbing, each slab is a step up to the height of its upper edge.
            let top = if hi_end == 0.0 { low } else { deck(u + hi_end / 6.0) };
            let hk = hash(hx + i as f64, hy * 3.0);
            cube(&mut self.static_cubes, at(u, 0.0, top - 0.06), dims(0.84, 0.12, 1.0 / 3.0 - 0.01), 0.0, jit(BRIDGE, hk, 0.06));
        }
        // Spandrel walls under the deck, each with an arch ring of voussoirs; none on a side
        // where the next bridge tile of a wide crossing carries the deck on.
        let water_top = low - WATER_DROP;
        let widened = |side: f32| {
            let s = if side < 0.0 { -1 } else { 1 };
            if along_z {
                grid.is_bridge(b.x + s, b.y)
            } else {
                grid.is_bridge(b.x, b.y + s)
            }
        };
        for side in [-0.42f32, 0.42] {
            if widened(side) {
                continue;
            }
            for k in 0..9 {
                let t = k as f32 / 8.0;
                let u = -0.5 + t;
                let a = t * PI;
                let arch = water_top + 0.05 + a.sin() * 0.24;
                let top = deck(u) - 0.1;
                if top > arch {
                    cube(
                        &mut self.static_cubes,
                        at(u, side, (arch + top) / 2.0),
                        dims(0.08, top - arch, 1.0 / 8.0 + 0.01),
                        0.0,
                        jit(BRIDGE_D, hash(hx * 3.0 + k as f64, hy + side as f64), 0.05),
                    );
                }
                cube(&mut self.static_cubes, at(u, side, arch + 0.03), dims(0.1, 0.06, 1.0 / 8.0), 0.0, lin(BRIDGE));
            }
            // Abutments down to the bed at each end.
            for end in [-0.46f32, 0.46] {
                let top = deck(end) - 0.08;
                let bottom = low - BED_DROP;
                cube(&mut self.static_cubes, at(end, side, (top + bottom) / 2.0), dims(0.12, top - bottom, 0.1), 0.0, lin(BRIDGE_D));
            }
            // The balustrade: plinth, balusters, handrail; a pier with a ball at each end.
            let (d0, d1) = (deck(-0.5), deck(0.5));
            let slope = (d1 - d0).atan2(1.0);
            let len = (1.0 + (d1 - d0).powi(2)).sqrt();
            let mid = (d0 + d1) / 2.0;
            let tilt = if along_z { Quat::from_rotation_x(-slope) } else { Quat::from_rotation_z(slope) };
            cube_shade(&mut self.static_cubes, at(0.0, side, mid + 0.03), dims(0.1, 0.06, len), tilt, lin(BRIDGE_D), Vec3::ZERO, SHADE_LIT);
            cube_shade(
                &mut self.static_cubes,
                at(0.0, side, mid + 0.25),
                dims(0.11, 0.05, len + 0.02),
                tilt,
                lin(BRIDGE),
                Vec3::ZERO,
                SHADE_LIT,
            );
            for k in 0..7 {
                let u = -0.36 + k as f32 * 0.12;
                cube(&mut self.static_cubes, at(u, side, deck(u) + 0.14), dims(0.045, 0.17, 0.045), 0.0, lin(BRIDGE));
            }
            for end in [-0.47f32, 0.47] {
                cube(&mut self.static_cubes, at(end, side, deck(end) + 0.16), dims(0.13, 0.33, 0.1), 0.0, lin(BRIDGE_D));
                cube(&mut self.static_cubes, at(end, side, deck(end) + 0.37), Vec3::splat(0.07), 0.6, lin(BRIDGE));
            }
        }
    }

    /// A churchyard tile: dark turf, leaning headstones and crosses, iron railings to the paths.
    fn build_grave_tile(&mut self, grid: &Grid, p: P, dead_tree: bool, tomb: bool, candle: bool) {
        let (x, y) = (p.x, p.y);
        let (xf, yf) = (x as f32, y as f32);
        let (hx, hy) = (x as f64, y as f64);
        let base = grid.elevation(x, y) as f32 * STEP;
        let top = base + 0.1;
        let start = self.static_cubes.len();
        cube(
            &mut self.static_cubes,
            Vec3::new(xf, (top - 0.1) / 2.0, yf),
            Vec3::new(1.0, top + 0.1, 1.0),
            0.0,
            jit(GRAVE_EARTH, hash(hx, hy), 0.08),
        );
        cube(&mut self.static_cubes, Vec3::new(xf, top + 0.005, yf), Vec3::new(1.0, 0.01, 1.0), 0.0, jit(GRAVE_GRASS, hash(hy, hx), 0.1));
        let roads: Vec<Dir> = CARDINALS
            .iter()
            .copied()
            .filter(|d| {
                let (dx, dy) = d.vec();
                let t = grid.get(x + dx, y + dy);
                grid.in_bounds(x + dx, y + dy) && t != Tile::Rock
            })
            .collect();
        // Wrought-iron railings with spear tips along each path.
        for &d in &roads {
            let (dx, dy) = d.vec();
            let turn = if dx != 0 { PI / 2.0 } else { 0.0 };
            let at = |u: f32, hgt: f32| {
                Vec3::new(xf + dx as f32 * 0.44 + if dx != 0 { 0.0 } else { u }, hgt, yf + dy as f32 * 0.44 + if dx != 0 { u } else { 0.0 })
            };
            let iron = lin(IRON);
            for k in 0..10 {
                let u = -0.45 + k as f32 * 0.1;
                cube_shade(&mut self.static_cubes, at(u, top + 0.15), Vec3::new(0.018, 0.3, 0.018), Quat::IDENTITY, iron, Vec3::ZERO, 6.0);
                cube_shade(
                    &mut self.static_cubes,
                    at(u, top + 0.315),
                    Vec3::new(0.03, 0.035, 0.03),
                    Quat::from_rotation_y(0.785),
                    iron,
                    Vec3::ZERO,
                    6.0,
                );
            }
            for hgt in [0.06, 0.26] {
                cube_shade(
                    &mut self.static_cubes,
                    at(0.0, top + hgt),
                    Vec3::new(1.0, 0.02, 0.02),
                    Quat::from_rotation_y(turn),
                    iron,
                    Vec3::ZERO,
                    6.0,
                );
            }
        }
        // Graves face the nearest path.
        let face = roads.first().copied().unwrap_or(Dir::Down);
        let (fx, fz) = {
            let (a, b) = face.vec();
            (a as f32, b as f32)
        };
        let rot = fx.atan2(fz);
        let q = Quat::from_rotation_y(rot);
        if dead_tree {
            self.dead_tree(Vec3::new(xf, top, yf), hash(hx, hy) as f32);
        } else if tomb {
            // A little tomb: stone chest with a pediment roof and a dark door.
            let o = Vec3::new(xf, top, yf);
            cube_shade(
                &mut self.static_cubes,
                o + Vec3::new(0.0, 0.22, 0.0),
                Vec3::new(0.62, 0.44, 0.52),
                q,
                lin(HEADSTONE[2]),
                Vec3::ZERO,
                SHADE_LIT,
            );
            cube_shade(
                &mut self.static_cubes,
                o + q * Vec3::new(0.0, 0.16, 0.265),
                Vec3::new(0.2, 0.3, 0.02),
                q,
                lin(0x141414),
                Vec3::ZERO,
                SHADE_LIT,
            );
            for s in [-1.0f32, 1.0] {
                let roof = q * Quat::from_rotation_z(s * 0.5);
                cube_shade(
                    &mut self.static_cubes,
                    o + q * Vec3::new(s * 0.16, 0.52, 0.0),
                    Vec3::new(0.4, 0.05, 0.6),
                    roof,
                    lin(HEADSTONE[3]),
                    Vec3::ZERO,
                    SHADE_LIT,
                );
            }
            cube_shade(
                &mut self.static_cubes,
                o + Vec3::new(0.0, 0.66, 0.0),
                Vec3::new(0.08, 0.1, 0.08),
                q,
                lin(HEADSTONE[0]),
                Vec3::ZERO,
                SHADE_LIT,
            );
        } else {
            let count = 1 + (hash(hx * 3.0, hy * 5.0) * 1.99) as usize;
            for k in 0..count {
                let hk = hash(hx * 13.0 + k as f64, hy * 7.0) as f32;
                let slot = if count == 1 { 0.0 } else { (k as f32 - 0.5) * 0.44 };
                let o = Vec3::new(xf, top, yf) + q * Vec3::new(slot, 0.0, -0.1);
                let lean = Quat::from_rotation_y(rot + (hk - 0.5) * 0.3)
                    * Quat::from_rotation_x((hk - 0.5) * 0.25)
                    * Quat::from_rotation_z((hk - 0.3) * 0.12);
                let stone = jit(HEADSTONE[(hk * 4.0) as usize % 4], hk as f64, 0.06);
                match (hk * 3.0) as usize {
                    0 => {
                        // Rounded headstone.
                        cube_shade(
                            &mut self.static_cubes,
                            o + Vec3::new(0.0, 0.15, 0.0),
                            Vec3::new(0.2, 0.3, 0.05),
                            lean,
                            stone,
                            Vec3::ZERO,
                            SHADE_LIT,
                        );
                        cube_shade(
                            &mut self.static_cubes,
                            o + lean * Vec3::new(0.0, 0.31, 0.0),
                            Vec3::new(0.14, 0.04, 0.05),
                            lean,
                            stone,
                            Vec3::ZERO,
                            SHADE_LIT,
                        );
                        cube_shade(
                            &mut self.static_cubes,
                            o + lean * Vec3::new(0.05, 0.12, 0.028),
                            Vec3::new(0.06, 0.05, 0.005),
                            lean,
                            lin(LICHEN),
                            Vec3::ZERO,
                            SHADE_LIT,
                        );
                    }
                    1 => {
                        // A cross.
                        cube_shade(
                            &mut self.static_cubes,
                            o + lean * Vec3::new(0.0, 0.18, 0.0),
                            Vec3::new(0.05, 0.36, 0.05),
                            lean,
                            stone,
                            Vec3::ZERO,
                            SHADE_LIT,
                        );
                        cube_shade(
                            &mut self.static_cubes,
                            o + lean * Vec3::new(0.0, 0.26, 0.0),
                            Vec3::new(0.2, 0.05, 0.05),
                            lean,
                            stone,
                            Vec3::ZERO,
                            SHADE_LIT,
                        );
                    }
                    _ => {
                        // An obelisk.
                        cube_shade(
                            &mut self.static_cubes,
                            o + Vec3::new(0.0, 0.04, 0.0),
                            Vec3::new(0.14, 0.08, 0.14),
                            q,
                            stone,
                            Vec3::ZERO,
                            SHADE_LIT,
                        );
                        for (i, (w, hgt)) in [(0.09f32, 0.14f32), (0.075, 0.12), (0.06, 0.1)].iter().enumerate() {
                            let yy = 0.08 + [0.07, 0.2, 0.31][i];
                            cube_shade(
                                &mut self.static_cubes,
                                o + Vec3::new(0.0, yy, 0.0),
                                Vec3::new(*w, *hgt, *w),
                                q,
                                stone,
                                Vec3::ZERO,
                                SHADE_LIT,
                            );
                        }
                        cube_shade(
                            &mut self.static_cubes,
                            o + Vec3::new(0.0, 0.42, 0.0),
                            Vec3::new(0.04, 0.04, 0.04),
                            q * Quat::from_rotation_x(0.785),
                            stone,
                            Vec3::ZERO,
                            SHADE_LIT,
                        );
                    }
                }
                // The grave itself, a low turf mound in front.
                cube_shade(
                    &mut self.static_cubes,
                    o + q * Vec3::new(0.0, 0.02, 0.2),
                    Vec3::new(0.18, 0.04, 0.3),
                    q,
                    lin(GRAVE_EARTH),
                    Vec3::ZERO,
                    SHADE_LIT,
                );
            }
        }
        if candle {
            // A grave lantern with a candle glowing inside.
            let o = Vec3::new(xf, top, yf) + q * Vec3::new(0.3, 0.0, 0.25);
            cube(&mut self.static_cubes, o + Vec3::new(0.0, 0.05, 0.0), Vec3::new(0.1, 0.1, 0.1), rot, lin(HEADSTONE[1]));
            cube_shade(
                &mut self.static_cubes,
                o + Vec3::new(0.0, 0.13, 0.0),
                Vec3::new(0.04, 0.06, 0.04),
                Quat::IDENTITY,
                lin(0xffd070),
                lin(0xffb050) * 2.5,
                SHADE_LIT,
            );
            cube(&mut self.static_cubes, o + Vec3::new(0.0, 0.2, 0.0), Vec3::new(0.12, 0.03, 0.12), rot, lin(HEADSTONE[1]));
            self.candles.push(o + Vec3::new(0.0, 0.2, 0.0));
        }
        self.rock_index.insert(tile_key(x, y), (start, self.static_cubes.len() - start));
    }

    /// A gnarled dead tree: a black trunk splitting into crooked bare branches.
    fn dead_tree(&mut self, base: Vec3, seed: f32) {
        let wood = lin(DEAD_WOOD);
        cube(&mut self.static_cubes, base + Vec3::new(0.0, 0.35, 0.0), Vec3::new(0.12, 0.7, 0.12), seed * 3.0, wood);
        let mut tips = vec![(base + Vec3::new(0.0, 0.68, 0.0), 0.0f32, 0.36f32, 0.08f32)];
        for depth in 0..3 {
            let mut next = vec![];
            for (k, &(from, yaw, len, thick)) in tips.iter().enumerate() {
                for b in 0..2 {
                    let h = hash(seed as f64 * 50.0 + depth as f64 * 7.0 + k as f64, b as f64 * 3.0) as f32;
                    let yaw = yaw + if b == 0 { 1.2 } else { -1.3 } + (h - 0.5) * 1.4 + seed * 2.0;
                    let pitch = 0.5 + h * 0.5;
                    let dir = Vec3::new(yaw.cos() * pitch.sin(), pitch.cos(), yaw.sin() * pitch.sin());
                    let mid = from + dir * (len / 2.0);
                    let rot = Quat::from_rotation_arc(Vec3::Y, dir);
                    cube_shade(&mut self.static_cubes, mid, Vec3::new(thick, len, thick), rot, wood, Vec3::ZERO, SHADE_LIT);
                    next.push((from + dir * len, yaw, len * 0.7, thick * 0.7));
                }
            }
            tips = next;
        }
    }

    /// A flower bed inside a clipped box hedge: roses, lavender, white and yellow blooms, now and then a topiary or an urn.
    fn build_flower_tile(&mut self, grid: &Grid, p: P, feature: &HashMap<P, Feature>) {
        let (x, y) = (p.x, p.y);
        let (xf, yf) = (x as f32, y as f32);
        let (hx, hy) = (x as f64, y as f64);
        let base = grid.elevation(x, y) as f32 * STEP;
        let top = base + 0.12;
        let start = self.static_cubes.len();
        cube(
            &mut self.static_cubes,
            Vec3::new(xf, (top - 0.1) / 2.0, yf),
            Vec3::new(1.0, top + 0.1, 1.0),
            0.0,
            jit(0x2c4424, hash(hx, hy), 0.08),
        );
        cube(&mut self.static_cubes, Vec3::new(xf, top + 0.01, yf), Vec3::new(0.98, 0.02, 0.98), 0.0, jit(SOIL, hash(hy, hx), 0.1));
        // Box hedges along the paths and along the edges of the bed.
        for d in CARDINALS {
            let (dx, dy) = d.vec();
            let n = P::new(x + dx, y + dy);
            let open = grid.in_bounds(n.x, n.y) && grid.get(n.x, n.y) != Tile::Rock;
            let other = grid.get(n.x, n.y) == Tile::Rock && feature.get(&n) != Some(&Feature::Flowers);
            if !(open || other) {
                continue;
            }
            let at = Vec3::new(xf + dx as f32 * 0.42, top + 0.1, yf + dy as f32 * 0.42);
            let s = if dx != 0 { Vec3::new(0.16, 0.2, 1.0) } else { Vec3::new(1.0, 0.2, 0.16) };
            cube(&mut self.static_cubes, at, s, 0.0, jit(BOX, hash(hx + dx as f64, hy + dy as f64), 0.06));
            cube(&mut self.static_cubes, at + Vec3::new(0.0, 0.1, 0.0), s * Vec3::new(0.9, 0.05, 0.9), 0.0, lin(BOX_L));
        }
        // What grows here.
        let pick = hash(hx * 7.0 + 5.0, hy * 11.0);
        let centre = hash(hx * 3.0, hy * 17.0);
        let (c1, c2) = (BLOOMS[(pick * 6.0) as usize % 6], BLOOMS[(pick * 13.0) as usize % 6]);
        for i in 0..4 {
            for j in 0..4 {
                let (u, v) = (-0.27 + i as f32 * 0.18, -0.27 + j as f32 * 0.18);
                if centre < 0.35 && i > 0 && i < 3 && j > 0 && j < 3 {
                    continue; // room for the centrepiece
                }
                let hk = hash(hx * 29.0 + i as f64, hy * 31.0 + j as f64) as f32;
                let pos = Vec3::new(xf + u + (hk - 0.5) * 0.05, top + 0.02, yf + v + (0.5 - hk) * 0.05);
                cube(&mut self.static_cubes, pos + Vec3::new(0.0, 0.03, 0.0), Vec3::new(0.09, 0.06, 0.09), hk * 3.0, lin(LEAF));
                let bloom = if (i + j) % 2 == 0 { c1 } else { c2 };
                let lavender = bloom == BLOOMS[2];
                let tall = if lavender { 0.2 } else { 0.1 + hk * 0.06 };
                cube(&mut self.static_cubes, pos + Vec3::new(0.0, tall / 2.0, 0.0), Vec3::new(0.014, tall, 0.014), 0.0, lin(STEM));
                if lavender {
                    for s in 0..3 {
                        cube(
                            &mut self.static_cubes,
                            pos + Vec3::new(0.0, tall - 0.02 - s as f32 * 0.035, 0.0),
                            Vec3::new(0.03, 0.03, 0.03),
                            s as f32,
                            lin(bloom),
                        );
                    }
                } else {
                    cube(
                        &mut self.static_cubes,
                        pos + Vec3::new(0.0, tall, 0.0),
                        Vec3::new(0.06, 0.045, 0.06),
                        hk * 2.0,
                        jit(bloom, hk as f64, 0.08),
                    );
                }
            }
        }
        if centre < 0.18 {
            // A clipped topiary cone.
            for (k, w) in [0.2f32, 0.16, 0.12, 0.08, 0.04].iter().enumerate() {
                cube(
                    &mut self.static_cubes,
                    Vec3::new(xf, top + 0.06 + k as f32 * 0.09, yf),
                    Vec3::new(*w, 0.1, *w),
                    k as f32 * 0.3,
                    jit(BOX, hash(hx, k as f64), 0.05),
                );
            }
        } else if centre < 0.35 {
            push_field(&mut self.static_cubes, &urn(), Vec3::new(xf, top, yf), 0.0, 0.9, Vec3::ONE, Vec3::ZERO, SHADE_LIT);
        }
        self.rock_index.insert(tile_key(x, y), (start, self.static_cubes.len() - start));
    }

    // -------------------------------------------------------------- per frame

    /// The moving water: surfaces, falls, foam and spray; then the mist over everything.
    pub(super) fn emit_garden(&self, time: f32, f: &mut Frame) {
        let cube_i = MeshKind::Cube as usize;
        let sphere = MeshKind::Sphere as usize;
        let Some(plan) = &self.garden else {
            f.sets_blend.push(self.mist_set.clone());
            return;
        };
        let (w, h) = (self.level_dims.x as i32, self.level_dims.y as i32);
        for (ci, channel) in plan.channels.iter().enumerate() {
            let n = channel.len();
            let off_edge = channel.last().is_some_and(|r| r.p.x == 0 || r.p.y == 0 || r.p.x == w - 1 || r.p.y == h - 1);
            for (i, r) in channel.iter().enumerate() {
                let lv = r.level as f32 * STEP - WATER_DROP;
                let (xf, yf) = (r.p.x as f32, r.p.y as f32);
                // Which way it runs here: on toward the next tile (the last keeps the heading it came in on).
                let (fx, fy) = match (channel.get(i + 1), i.checked_sub(1).and_then(|j| channel.get(j))) {
                    (Some(nx), _) => (nx.p.x - r.p.x, nx.p.y - r.p.y),
                    (None, Some(pv)) => (r.p.x - pv.p.x, r.p.y - pv.p.y),
                    _ => (0, 0),
                };
                // A pool at a dead end barely moves.
                let speed = if !off_edge && i + 2 >= n { 0.06 } else { 0.45 };
                let flow = Vec3::new(fx as f32 * speed, fy as f32 * speed, 0.0);
                let m = Mat4::from_translation(Vec3::new(xf, lv, yf)) * Mat4::from_scale(Vec3::new(1.0, 0.002, 1.0));
                f.transparent[cube_i].push(Instance::new(m, lin(0x17323a), 0.92, flow, SHADE_WATER));
                if i + 1 < n {
                    let next = channel[i + 1];
                    if next.level != r.level {
                        // A cascade from the higher reach down to the lower.
                        let (hi, lo) = if next.level < r.level { (*r, next) } else { (next, *r) };
                        let (dx, dy) = (lo.p.x - hi.p.x, lo.p.y - hi.p.y);
                        let top = hi.level as f32 * STEP - WATER_DROP;
                        let low = lo.level as f32 * STEP - WATER_DROP;
                        let at = Vec3::new(hi.p.x as f32 + dx as f32 * 0.5, (top + low) / 2.0, hi.p.y as f32 + dy as f32 * 0.5);
                        let s = if dx != 0 { Vec3::new(0.002, top - low, 1.0) } else { Vec3::new(1.0, top - low, 0.002) };
                        f.transparent[cube_i].push(Instance::new(
                            Mat4::from_translation(at) * Mat4::from_scale(s),
                            lin(0xc8dce4),
                            0.85,
                            Vec3::ZERO,
                            SHADE_WATER,
                        ));
                        self.foam(Vec3::new(at.x + dx as f32 * 0.1, low + 0.02, at.z + dy as f32 * 0.1), time, (i + ci * 50) as f32, f);
                    }
                } else if off_edge {
                    // Over the edge and down into the clouds, throwing spray, mist boiling up below.
                    let (ax, az) = (fx as f32, fy as f32);
                    let edge = Vec3::new(xf + ax * 0.5, lv, yf + az * 0.5);
                    let drop = 4.2;
                    let size = if ax != 0.0 { Vec3::new(0.002, drop, 0.96) } else { Vec3::new(0.96, drop, 0.002) };
                    let m = Mat4::from_translation(edge + Vec3::new(ax * 0.02, -drop / 2.0, az * 0.02)) * Mat4::from_scale(size);
                    f.transparent[cube_i].push(Instance::new(m, lin(0xc8dce4), 0.8, Vec3::ZERO, SHADE_WATER));
                    for k in 0..12 {
                        let fk = k as f32 + ci as f32 * 5.0;
                        let t = (time * 0.6 + fk * 0.37).fract();
                        let across = (fk * 1.7).sin() * 0.4;
                        let p = edge + Vec3::new(ax * (0.1 + t * 0.5) - az * across, -t * t * 2.8, az * (0.1 + t * 0.5) + ax * across);
                        let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.07 + t * 0.12));
                        f.additive.push(Instance::new(m, lin(0xcfe0ea), 0.25 * (1.0 - t), Vec3::ZERO, SHADE_UNLIT));
                    }
                    for k in 0..6 {
                        let fk = k as f32 * 1.3 + ci as f32;
                        let t = (time * 0.12 + fk * 0.17).fract();
                        let across = (fk * 2.3).sin() * 0.5;
                        let p = edge + Vec3::new(ax * (0.6 + t * 0.8) - az * across, -2.6 + t * 2.4, az * (0.6 + t * 0.8) + ax * across);
                        let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.3 + t * 0.5));
                        f.transparent[sphere].push(Instance::new(m, lin(0xb4c4cc), 0.12 * (PI * t).sin(), Vec3::ZERO, SHADE_LIT));
                    }
                }
            }
            // Foam flecks drifting downstream.
            if n >= 2 {
                let count = (n / 3).clamp(3, 10);
                for k in 0..count {
                    let fk = k as f32 + ci as f32 * 0.37;
                    let s = (time * 0.12 * 20.0 / n as f32 + fk * 0.618).fract() * (n as f32 - 1.0);
                    let i = s.floor() as usize;
                    let (a, b) = (channel[i], channel[(i + 1).min(n - 1)]);
                    let t = s - i as f32;
                    let side = (fk * 2.3).sin() * 0.3;
                    let (dx, dy) = ((b.p.x - a.p.x) as f32, (b.p.y - a.p.y) as f32);
                    let p = Vec3::new(
                        a.p.x as f32 + dx * t - dy * side,
                        a.level.min(b.level) as f32 * STEP - WATER_DROP + 0.01,
                        a.p.y as f32 + dy * t + dx * side,
                    );
                    let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(0.04, 0.005, 0.03));
                    f.additive.push(Instance::new(m, lin(0xb8ccd4), 0.35, Vec3::ZERO, SHADE_UNLIT));
                }
            }
        }
        // Will-o'-the-wisps over the churchyard: pale lights that bob and wander, trailing sparks.
        if let Some(home) = self.wisps {
            for k in 0..3 {
                let fk = k as f32;
                let t = time * (0.19 + fk * 0.05) + fk * 2.1;
                let at = |t: f32| {
                    home + Vec3::new(t.sin() * 1.7 + (t * 2.3).sin() * 0.3, 0.5 + (t * 5.0 + fk).sin() * 0.12, (t * 0.8 + fk).cos() * 1.4)
                };
                let p = at(t);
                let pulse = 0.8 + (time * 3.1 + fk * 1.9).sin() * 0.2;
                glow_orb(f, p, 0.36 * pulse, lin(0x5cffd0), 0.32 * pulse);
                glow_orb(f, p, 0.09, lin(0xe0fff6), 0.9);
                for s in 1..6 {
                    let q = at(t - s as f32 * 0.035) + Vec3::new(0.0, (time * 7.0 + s as f32).sin() * 0.02, 0.0);
                    let m = Mat4::from_translation(q) * Mat4::from_scale(Vec3::splat(0.028 - s as f32 * 0.004));
                    f.additive.push(Instance::new(m, lin(0x8affe0), 0.7 * (1.0 - s as f32 / 6.0), Vec3::ZERO, SHADE_UNLIT));
                }
                if p.distance_squared(self.focus_point) < 18.0 * 18.0 {
                    f.lights.push(Light { pos: p, color: lin(0x4cffc8) * 0.9 * pulse, distance: 2.6 });
                }
            }
        }
        f.sets_blend.push(self.mist_set.clone());
    }

    pub(super) fn foam(&self, at: Vec3, time: f32, seed: f32, f: &mut Frame) {
        for k in 0..8 {
            let fk = k as f32 + seed * 3.0;
            let t = (time * 1.3 + fk * 0.29).fract();
            let p = at + Vec3::new((fk * 1.3).sin() * 0.3, t * 0.25, (fk * 2.1).cos() * 0.4);
            let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.06 + t * 0.1));
            f.additive.push(Instance::new(m, lin(0xdce8ee), 0.3 * (1.0 - t), Vec3::ZERO, SHADE_UNLIT));
        }
    }
}
