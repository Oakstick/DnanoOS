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
            self.churchyard_tile(grid, p, first_tree, tomb, candle);
            tree_done |= first_tree;
            tomb_done |= tomb;
            candles += candle as usize;
        }
        for p in &plan.flowers {
            self.parterre_tile(grid, *p, feature);
        }
        // The temples on their lawns, in dressed stone (the shrines are built with their beds).
        for block in &plan.temples {
            let n = block.len() as f32;
            let cx = block.iter().map(|p| p.x as f32).sum::<f32>() / n;
            let cz = block.iter().map(|p| p.y as f32).sum::<f32>() / n;
            let base = block.iter().map(|p| floor_y(p.x, p.y)).fold(0.0, f32::max) + 0.05;
            let pos = Vec3::new(cx, base - 0.005, cz);
            let start = self.static_cubes.len();
            let flame = garden_landmarks::temple_cubes(&mut self.static_cubes, pos, (cx as f64, cz as f64));
            let len = self.static_cubes.len() - start;
            let shades = self.static_cubes[start..start + len].iter().map(|c| c.emissive[3]).collect();
            // It fades like a tree when it comes between the camera and Ragnar.
            self.trees.push(Tree {
                range: (start, len),
                base: pos,
                radius: 0.9,
                height: 2.0,
                fade: 1.0,
                alive: true,
                shades: Some(shades),
            });
            self.authored.push((start, len));
            self.candles.push(flame);
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
            // In the garden the mist lies on the walks, the water and the graves, not over the
            // beds (it would veil them grey; the hedges hold it out).
            if self.estate_art && t == Tile::Rock && !thick {
                continue;
            }
            // (In the garden the walks keep two sheets, the haze doing the rest: a third cost a
            // millisecond of overdraw.)
            let layers: &[(f32, f32)] = if thick {
                &[(0.1, 0.55), (0.28, 0.48), (0.5, 0.36), (0.8, 0.22)]
            } else if self.estate_art {
                &[(0.12, 0.38), (0.4, 0.28)]
            } else {
                &[(0.12, 0.34), (0.34, 0.27), (0.62, 0.16)]
            };
            for (k, &(lift, alpha)) in layers.iter().enumerate() {
                let m = Mat4::from_translation(Vec3::new(x as f32, floor + lift, y as f32)) * Mat4::from_scale(Vec3::new(1.0, 0.002, 1.0));
                let tone = if self.estate_art { lin(estate::pal::MIST) } else { lin(MIST) };
                self.mist.push(Instance::new(m, tone, alpha, Vec3::new(20.0 + k as f32, lift, 0.0), SHADE_CLOUD));
            }
        }
    }

    /// The channels: bed, stone banks, weirs, lily pads, reeds; a grotto where a stream rises from the ground, rocks where one pours off the edge.
    pub(super) fn build_river(&mut self, grid: &Grid, plan: &GardenPlan, water: &HashMap<P, i32>) {
        let start = self.static_cubes.len();
        self.build_river_cubes(grid, plan, water);
        if self.estate_art {
            // In the garden, reeds and pads are strand-shaded growth, and the lilies' pinks and
            // whites come down to the board's quieter rose and pale stone.
            for c in self.static_cubes[start..].iter_mut().filter(|c| c.emissive[3] == SHADE_LIT) {
                let (r, g, b) = (c.color[0], c.color[1], c.color[2]);
                if g > r && g > b {
                    c.emissive[3] = crypt::SHADE_SOFT;
                } else if r > g * 1.15 && b > g * 0.9 {
                    c.color = [r * 0.5, g * 0.45, b * 0.42, c.color[3]];
                    c.emissive[3] = crypt::SHADE_SOFT;
                }
            }
        }
    }

    fn build_river_cubes(&mut self, grid: &Grid, plan: &GardenPlan, water: &HashMap<P, i32>) {
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
                    if self.estate_art {
                        // The garden's banks: the crypt's dressed stone, chunky and chipped,
                        // dark and wet low down, under a coping of pale slabs.
                        let start = self.static_cubes.len();
                        crypt::masonry(
                            &mut self.static_cubes,
                            &crypt::Wall {
                                centre: Vec3::new(np.x as f32, 0.0, np.y as f32),
                                dir: (-dx, -dy),
                                half: 0.5,
                                base: bed,
                                height: top - bed,
                                courses: 2,
                                seed: (hx * 2.3 + dx as f64, hy * 1.7 + dy as f64),
                                len: (0.24, 0.2),
                                depth: (0.04, 0.05),
                                face: 0.43,
                                wet: true,
                                ruined: false,
                                foot: None,
                                openings: &[],
                            },
                        );
                        for k in 0..2 {
                            let hk = hash(hx * 3.1 + k as f64 + dx as f64, hy * 1.9 + dy as f64);
                            let u = -0.25 + k as f32 * 0.5;
                            crypt::block(
                                &mut self.static_cubes,
                                at(u, top + 0.03),
                                Vec3::new(0.49, 0.06, 0.2),
                                turn + (hk as f32 - 0.5) * 0.04,
                                crypt::jit(crypt::pal::PALE_STONE[k], hk, 0.05) * 0.72,
                                crypt::SHADE_STONE,
                            );
                        }
                        self.authored.push((start, self.static_cubes.len() - start));
                    } else {
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
                    }
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
        let start = self.static_cubes.len();
        self.build_stone_bridge_cubes(grid, b);
        if self.estate_art {
            // In the garden, the crypt's stone: pale dressed stone kept as dark as the old
            // pieces were against the bridge's pale, the deck wet flagstone.
            let pale = lin(BRIDGE);
            let lum = |c: Vec3| c.dot(Vec3::new(0.2126, 0.7152, 0.0722));
            for (k, c) in self.static_cubes[start..].iter_mut().enumerate() {
                let old = Vec3::new(c.color[0], c.color[1], c.color[2]);
                let h = hash(b.x as f64 * 3.1 + k as f64, b.y as f64 * 1.7);
                let new = crypt::jit(crypt::pal::PALE_STONE[k % 2], h, 0.06) * 0.74 * (lum(old) / lum(pale)).min(1.1);
                c.color = [new.x, new.y, new.z, c.color[3]];
                c.emissive[3] = if k < 3 { crypt::SHADE_FLAG } else { crypt::SHADE_STONE };
            }
            self.authored.push((start, self.static_cubes.len() - start));
        }
    }

    fn build_stone_bridge_cubes(&mut self, grid: &Grid, b: P) {
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

    // -------------------------------------------------------------- per frame

    /// The moving water: surfaces, falls, foam and spray; then the mist over everything.
    pub(super) fn emit_garden(&self, time: f32, f: &mut Frame) {
        let cube_i = MeshKind::Cube as usize;
        let sphere = MeshKind::Sphere as usize;
        let Some(plan) = &self.garden else {
            f.sets_blend.push(self.mist_set.clone());
            return;
        };
        // Rain standing in the walks, holding the lanterns.
        for (i, &(p, size)) in self.puddles.iter().enumerate() {
            if p.distance_squared(self.focus_point) < 24.0 * 24.0 {
                crypt::emit_pool(f, p, size, i as f32 * 0.7);
            }
        }
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
                // (Pale mist teal, the board's, not a neon green.)
                glow_orb(f, p, 0.32 * pulse, lin(0x8fd0c0), 0.2 * pulse);
                glow_orb(f, p, 0.08, lin(0xd8f0e8), 0.7);
                for s in 1..6 {
                    let q = at(t - s as f32 * 0.035) + Vec3::new(0.0, (time * 7.0 + s as f32).sin() * 0.02, 0.0);
                    let m = Mat4::from_translation(q) * Mat4::from_scale(Vec3::splat(0.028 - s as f32 * 0.004));
                    f.additive.push(Instance::new(m, lin(0xa8e0d0), 0.5 * (1.0 - s as f32 / 6.0), Vec3::ZERO, SHADE_UNLIT));
                }
                if p.distance_squared(self.focus_point) < 18.0 * 18.0 {
                    f.lights.push(Light { pos: p, color: lin(0x7ad0b8) * 0.6 * pulse, distance: 2.6 });
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
