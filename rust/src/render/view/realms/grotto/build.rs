//! The grotto's static geometry: the Waterline's shelves, walls and rib-bridges; the Drowned
//! Floor where it is, and again beneath the Waterline as the water shows it.

use std::collections::{HashMap, HashSet};

use super::super::super::vault_decor::push_merged;
use super::decor::GrottoArt;
use super::*;

/// What the build hands the view to light and draw each frame.
pub(super) struct Built {
    pub lamps: Vec<Vec3>,
    pub copy_lamps: Vec<Vec3>,
    pub algae: Vec<Vec3>,
    pub ribs: Vec<(Vec3, bool)>,
}

/// The ground as the scenery sees it: a coil's track as it lies when no coil is on it.
struct Ground<'a> {
    grid: &'a Grid,
    base: HashMap<P, (Tile, bool)>,
}

impl Ground<'_> {
    fn get(&self, x: i32, y: i32) -> Tile {
        self.base.get(&P::new(x, y)).map(|b| b.0).unwrap_or_else(|| self.grid.get(x, y))
    }

    fn rock(&self, x: i32, y: i32) -> bool {
        self.get(x, y) == Tile::Rock
    }

    fn bridge(&self, x: i32, y: i32) -> bool {
        self.base.get(&P::new(x, y)).map(|b| b.1).unwrap_or_else(|| self.grid.is_bridge(x, y))
    }

    /// The four sides of `p` that are not rock (and on the same surface).
    fn open_sides(&self, p: P) -> Vec<Dir> {
        CARDINALS
            .iter()
            .copied()
            .filter(|d| {
                let n = p.step(*d);
                self.grid.in_bounds(n.x, n.y) && !self.rock(n.x, n.y) && layer_of(n) == layer_of(p)
            })
            .collect()
    }
}

impl WorldView {
    /// Everything that stands still.
    pub(super) fn build_grotto_static(&mut self, grid: &Grid, r: &Grotto, art: &GrottoArt) -> Built {
        let ground = Ground { grid, base: r.coil_base.iter().flatten().map(|&(p, t, b)| (p, (t, b))).collect() };
        let mut built = Built { lamps: Vec::new(), copy_lamps: Vec::new(), algae: Vec::new(), ribs: Vec::new() };
        self.build_waterline(&ground, r, art, &mut built);
        let mut lamps = Vec::new();
        self.build_drowned(&ground, r, art, Vec3::ZERO, false, &mut lamps);
        built.lamps = lamps;
        let mut lamps = Vec::new();
        self.build_drowned(&ground, r, art, copy_offset(), true, &mut lamps);
        built.copy_lamps = lamps;
        self.build_caps(&ground);
        built
    }

    /// The Waterline: shelves of wet basalt in the water, walls rising out of it, rib-bridges over the
    /// deep water (and the stumps of those that fell), algae glowing along every edge of the deep.
    fn build_waterline(&mut self, g: &Ground, r: &Grotto, art: &GrottoArt, built: &mut Built) {
        let fallen: HashSet<P> = r.fallen.iter().copied().collect();
        let pools: HashSet<P> = r.pools.iter().map(|q| q.at).collect();
        let bell_pool: HashSet<P> = r.bell.pool.iter().copied().collect();
        let deep = |p: P| g.get(p.x, p.y) == Tile::Pit || pools.contains(&p) || bell_pool.contains(&p);
        for y in 0..g.grid.height {
            for x in 0..REGION_W {
                let p = P::new(x, y);
                let (xf, yf) = (x as f32, y as f32);
                let h = hash(x as f64 * 1.7 + 3.0, y as f64 * 2.3);
                let h2 = hash(x as f64 * 5.1 + 11.0, y as f64 * 3.7 + 2.0);
                let t = g.get(x, y);
                if t == Tile::Rock {
                    if p == r.base.entry.tile {
                        continue;
                    }
                    let open = g.open_sides(p);
                    if !open.is_empty() {
                        self.build_basalt_wall(p, &open, h, h2);
                    }
                    continue;
                }
                if t == Tile::Pit {
                    if fallen.contains(&p) {
                        self.build_fallen_rib(p, h);
                    }
                    continue;
                }
                if pools.contains(&p) || bell_pool.contains(&p) {
                    continue;
                }
                if g.bridge(x, y) {
                    let along = !g.rock(x - 1, y) && !g.rock(x + 1, y) && (g.get(x, y - 1) == Tile::Pit || g.get(x, y + 1) == Tile::Pit);
                    let along = along || (g.get(x, y - 1) == Tile::Pit && g.get(x, y + 1) == Tile::Pit);
                    self.build_rib_bridge(g, p, along, h);
                    built.ribs.push((Vec3::new(xf, 0.0, yf), along));
                    continue;
                }
                // A shelf of wet basalt, standing up out of the deep.
                let out = &mut self.static_cubes;
                // Its top in plates of rock, a seam between them, one now and then still wet.
                cube(out, Vec3::new(xf, SHELF_Y - 0.14, yf), Vec3::new(0.995, 0.24, 0.995), 0.0, lin(BASALT_D), SHADE_LIT);
                let split = 0.3 + h as f32 * 0.4;
                let split2 = 0.3 + h2 as f32 * 0.4;
                for (k, (x0, x1, z0, z1)) in
                    [(0.0, split, 0.0, split2), (split, 1.0, 0.0, 1.0 - split2), (0.0, split, split2, 1.0), (split, 1.0, 1.0 - split2, 1.0)]
                        .into_iter()
                        .enumerate()
                {
                    let hk = hash(x as f64 * 9.1 + k as f64 * 3.7, y as f64 * 6.3 + k as f64);
                    let c = lin(if hk > 0.55 { SHELF_L } else { SHELF }) * (0.85 + hk as f32 * 0.3);
                    let size = Vec3::new((x1 - x0) - 0.04, 0.05, (z1 - z0) - 0.04);
                    let pos = Vec3::new(xf - 0.5 + (x0 + x1) / 2.0, SHELF_Y - 0.025 + (hk as f32 - 0.5) * 0.02, yf - 0.5 + (z0 + z1) / 2.0);
                    let c = if hk > 0.9 {
                        c.lerp(lin(ALGAE_D), 0.35)
                    } else if hk < 0.06 {
                        c.lerp(lin(BONE_D), 0.25)
                    } else {
                        c
                    };
                    cube(out, pos, size, 0.0, c, SHADE_LIT);
                }
                cube(
                    out,
                    Vec3::new(xf, SHELF_Y - 0.24 - (SHELF_DEPTH - 0.24) / 2.0, yf),
                    Vec3::new(0.97, SHELF_DEPTH - 0.24, 0.97),
                    0.0,
                    lin(BASALT_D),
                    SHADE_LIT,
                );
                for d in CARDINALS {
                    let n = p.step(d);
                    if !deep(n) {
                        continue;
                    }
                    let (dx, dy) = d.vec();
                    let along_x = dx == 0;
                    let edge = Vec3::new(xf + dx as f32 * 0.47, SHELF_Y + 0.005, yf + dy as f32 * 0.47);
                    let size = if along_x { Vec3::new(1.0, 0.03, 0.08) } else { Vec3::new(0.08, 0.03, 1.0) };
                    let rich = pools.contains(&n) || bell_pool.contains(&n);
                    // The algae fringe, brighter round the pools that only go down.
                    let glow = if rich { lin(ALGAE) * 0.42 } else { lin(FRINGE) * 0.3 };
                    cube_glow(out, edge, size, Quat::IDENTITY, glow, 0.0, SHADE_LIGHT);
                    if rich {
                        for k in 0..2 {
                            let s = (k as f32 - 0.5) * 0.5 + (h2 as f32 - 0.5) * 0.2;
                            let at =
                                edge + if along_x { Vec3::new(s, 0.02, -dy as f32 * 0.06) } else { Vec3::new(-dx as f32 * 0.06, 0.02, s) };
                            let c = if bell_pool.contains(&n) { lin(VERDIGRIS) } else { lin(if k == 0 { NACRE } else { NACRE_P }) };
                            let shade = if bell_pool.contains(&n) { SHADE_BRONZE } else { SHADE_NACRE };
                            cube(out, at, Vec3::new(0.16, 0.08, 0.14), h as f32 * 3.0 + k as f32, c, shade);
                        }
                        if (x + y) % 3 == 0 {
                            built.algae.push(edge + Vec3::new(0.0, 0.15, 0.0));
                        }
                    }
                }
                // What lies on the shelf: pebbles, shells, bones, mats of algae, now and then a skull.
                let base = Vec3::new(xf, SHELF_Y, yf);
                let jit = Vec3::new((h as f32 - 0.5) * 0.5, 0.0, (h2 as f32 - 0.5) * 0.5);
                match (h2 * 100.0) as u32 {
                    0..=13 => {
                        for k in 0..3 {
                            let a = k as f32 * 2.1 + h as f32 * 6.0;
                            cube(
                                out,
                                base + jit + Vec3::new(a.cos() * 0.18, 0.025, a.sin() * 0.18),
                                Vec3::new(0.09, 0.05, 0.07),
                                a,
                                lin(BASALT_L) * 1.2,
                                SHADE_WET,
                            );
                        }
                    }
                    14..=19 => cube(
                        out,
                        base + jit + Vec3::new(0.0, 0.015, 0.0),
                        Vec3::new(0.14, 0.03, 0.11),
                        h as f32 * 5.0,
                        lin(NACRE_P),
                        SHADE_NACRE,
                    ),
                    20..=23 => cube(
                        out,
                        base + jit + Vec3::new(0.0, 0.03, 0.0),
                        Vec3::new(0.42, 0.05, 0.06),
                        h as f32 * 4.0,
                        lin(BONE_D),
                        SHADE_BONE,
                    ),
                    24..=28 => {
                        for k in 0..3 {
                            let a = k as f32 * 2.4 + h as f32 * 7.0;
                            let at = base + jit + Vec3::new(a.cos() * 0.14, 0.006, a.sin() * 0.14);
                            cube_glow(
                                out,
                                at,
                                Vec3::new(0.12, 0.012, 0.08),
                                Quat::from_rotation_y(a),
                                lin(ALGAE_D) * 0.4,
                                0.0,
                                SHADE_LIGHT,
                            );
                        }
                    }
                    98..=99 if g.open_sides(p).len() == 4 => {
                        push_merged(out, &art.skull, Mat4::from_translation(base + jit * 0.5) * Mat4::from_rotation_y(h as f32 * TAU), 0.0)
                    }
                    _ => {}
                }
            }
        }
    }

    /// A wall of the Waterline: wet black basalt from deep under the water to above the shelves,
    /// banded, a stalagmite on top now and then; algae at the waterline, barnacles, green drips, and
    /// a verdigris mooring ring here and there. It dissolves when it stands between the camera and him.
    fn build_basalt_wall(&mut self, p: P, open: &[Dir], h: f64, h2: f64) {
        let (xf, yf) = (p.x as f32, p.y as f32);
        let start = self.static_cubes.len();
        let out = &mut self.static_cubes;
        let low = -SHELF_DEPTH - 0.2;
        cube(out, Vec3::new(xf, (WALL_H + low) / 2.0, yf), Vec3::new(1.0, WALL_H - low, 1.0), 0.0, basalt(h), SHADE_WET);
        for (k, y) in [0.32f32, 0.9].iter().enumerate() {
            let c = if k == 0 { lin(STRATA) } else { lin(BASALT_L) * 0.9 };
            cube(out, Vec3::new(xf, *y + (h as f32 - 0.5) * 0.1, yf), Vec3::new(1.012, 0.05, 1.012), 0.0, c, SHADE_LIT);
        }
        cube(out, Vec3::new(xf, WALL_H, yf), Vec3::new(1.06, 0.12, 1.06), 0.0, lin(BASALT_D) * (0.9 + h2 as f32 * 0.3), SHADE_LIT);
        if h > 0.62 {
            // A stalagmite on the wall's top.
            let (ox, oz) = ((h2 as f32 - 0.5) * 0.4, (h as f32 - 0.8) * 0.8);
            for k in 0..3 {
                let s = 0.36 - k as f32 * 0.1;
                cube(
                    out,
                    Vec3::new(xf + ox, WALL_H + 0.1 + k as f32 * 0.16, yf + oz),
                    Vec3::new(s, 0.18, s),
                    k as f32 * 0.7 + h as f32,
                    basalt(h2),
                    SHADE_WET,
                );
            }
        }
        for &d in open {
            let (dx, dy) = d.vec();
            let along = if dx == 0 { p.x } else { p.y };
            let n = Vec3::new(dx as f32, 0.0, dy as f32);
            let side = Vec3::new(dy as f32, 0.0, dx as f32).abs();
            let face = Vec3::new(xf, 0.0, yf) + n * 0.5;
            let hf = hash(p.x as f64 * 3.1 + dx as f64 * 7.0, p.y as f64 * 2.9 + dy as f64 * 5.0) as f32;
            // The waterline: a band of glowing algae where the water laps the stone.
            cube_glow(
                out,
                face + n * 0.01 + Vec3::new(0.0, 0.0, 0.0),
                side * 1.0 + Vec3::new(0.0, 0.08, 0.0) + n.abs() * 0.03,
                Quat::IDENTITY,
                lin(FRINGE) * 0.3,
                0.0,
                SHADE_LIGHT,
            );
            // Barnacles above it.
            for k in 0..3 {
                let s = (k as f32 - 1.0) * 0.3 + (hf - 0.5) * 0.2;
                let y = 0.1 + ((hf * 7.0 + k as f32 * 1.3).fract()) * 0.25;
                cube(
                    out,
                    face + n * 0.03 + side * s + Vec3::new(0.0, y, 0.0),
                    Vec3::splat(0.07),
                    hf + k as f32,
                    lin(BONE_D) * 0.85,
                    SHADE_BONE,
                );
            }
            if hf > 0.66 {
                // A green drip down the face.
                let s = (hf - 0.8) * 2.0;
                cube_glow(
                    out,
                    face + n * 0.02 + side * s + Vec3::new(0.0, 0.6, 0.0),
                    side * 0.04 + n.abs() * 0.02 + Vec3::new(0.0, 0.7, 0.0),
                    Quat::IDENTITY,
                    lin(ALGAE) * 0.25,
                    0.0,
                    SHADE_LIGHT,
                );
            }
            if along.rem_euclid(7) == 3 {
                // A verdigris mooring ring.
                let at = face + n * 0.06 + Vec3::new(0.0, 0.62, 0.0);
                cube(out, at, side * 0.06 + n.abs() * 0.08 + Vec3::new(0.0, 0.06, 0.0), 0.0, lin(VERDIGRIS_D), SHADE_BRONZE);
                for (sy, sx) in [(0.12f32, 0.0f32), (-0.12, 0.0), (0.0, 0.12), (0.0, -0.12)] {
                    let sz = if sy != 0.0 { side * 0.22 + Vec3::new(0.0, 0.04, 0.0) } else { side * 0.04 + Vec3::new(0.0, 0.24, 0.0) };
                    cube(out, at + n * 0.03 + Vec3::new(0.0, sy, 0.0) + side * sx, sz + n.abs() * 0.04, 0.0, lin(VERDIGRIS), SHADE_BRONZE);
                }
            }
        }
        let len = self.static_cubes.len() - start;
        let shades: Vec<f32> = self.static_cubes[start..start + len].iter().map(|i| i.emissive[3]).collect();
        self.trees.push(Tree {
            range: (start, len),
            base: Vec3::new(xf, 0.0, yf),
            radius: 0.72,
            height: WALL_H + 0.4,
            fade: 1.0,
            alive: true,
            shades: Some(shades),
        });
    }

    /// A rib-bridge tile, the skeleton of a dead sea-serpent laid over the deep: down the bridge's
    /// middle its spine, vertebra after vertebra; either side its ribs, raked back like a fish's;
    /// along the edges a rib arching up, and a strut of bone down into the water.
    fn build_rib_bridge(&mut self, g: &Ground, p: P, along_x: bool, h: f64) {
        let (xf, yf) = (p.x as f32, p.y as f32);
        let out = &mut self.static_cubes;
        let (fwd, side) = if along_x { (Vec3::X, Vec3::Z) } else { (Vec3::Z, Vec3::X) };
        let c = Vec3::new(xf, 0.0, yf);
        let yaw = if along_x { 0.0 } else { PI / 2.0 };
        let deck = |q: P| g.bridge(q.x, q.y) || (g.get(q.x, q.y) != Tile::Rock && g.get(q.x, q.y) != Tile::Pit);
        let across = [-1.0f32, 1.0].map(|s| P::new(p.x + (side.x * s) as i32, p.y + (side.z * s) as i32));
        let spine = across.iter().all(|&q| g.bridge(q.x, q.y)) || across.iter().all(|&q| !g.bridge(q.x, q.y));
        let along_n = if along_x { p.x } else { p.y };
        if spine {
            // Two vertebrae a tile: the centrum, the wings either side, the spike on top.
            for k in 0..2 {
                let at = c + fwd * (k as f32 * 0.5 - 0.25);
                let col = lin(if (along_n + k) % 2 == 0 { BONE } else { BONE_D });
                cube(out, at + Vec3::new(0.0, -0.07, 0.0), Vec3::new(0.38, 0.14, 0.44), yaw, col, SHADE_BONE);
                cube(out, at + Vec3::new(0.0, -0.2, 0.0), Vec3::new(0.3, 0.16, 0.34), yaw, col * 0.8, SHADE_BONE);
                cube(out, at + Vec3::new(0.0, -0.04, 0.0), Vec3::new(0.12, 0.1, 0.96), yaw, col * 0.92, SHADE_BONE);
                cube(out, at + Vec3::new(0.0, 0.02, 0.0), Vec3::new(0.08, 0.06, 0.12), yaw, col * 1.05, SHADE_BONE);
            }
        }
        // The ribs, raked back, off whichever side the spine is (both, on a narrow bridge).
        for s in [-1.0f32, 1.0] {
            let n = across[if s < 0.0 { 0 } else { 1 }];
            let toward_spine = g.bridge(n.x, n.y) && !spine;
            if toward_spine {
                continue;
            }
            for k in 0..2 {
                let hk = hash(p.x as f64 * 3.3 + k as f64 + s as f64, p.y as f64 * 1.7) as f32;
                let col = lin(if hk > 0.5 { BONE } else { BONE_D }) * (0.85 + hk * 0.2);
                let rake = 0.45 + hk * 0.2;
                let root = c + fwd * (k as f32 * 0.5 - 0.25) + side * (s * if spine { 0.18 } else { -0.3 });
                let dir = (side * s + fwd * -rake).normalize();
                let len = if spine { 0.5 } else { 0.85 };
                let at = root + dir * (len / 2.0) + Vec3::new(0.0, -0.06, 0.0);
                let ry = -dir.z.atan2(dir.x);
                out.push(Instance::new(
                    Mat4::from_scale_rotation_translation(Vec3::new(len, 0.08, 0.11), Quat::from_rotation_y(ry), at),
                    col,
                    1.0,
                    Vec3::ZERO,
                    SHADE_BONE,
                ));
            }
        }
        for (s, n) in [-1.0f32, 1.0].into_iter().zip(across) {
            if deck(n) {
                continue;
            }
            // A rib, curving up and in over the deck.
            let base = c + side * (s * 0.5) + fwd * ((h as f32 - 0.5) * 0.3);
            for k in 0..5 {
                let t = k as f32 / 4.0;
                let a = t * 1.9;
                let at = base + side * (s * (0.08 * a.sin() - 0.02)) + Vec3::new(0.0, 0.05 + a.sin() * 0.85, 0.0)
                    - side * (s * (1.0 - a.cos()) * 0.35);
                let tilt = Quat::from_axis_angle(fwd, -s * (0.3 + a * 0.55));
                out.push(Instance::new(
                    Mat4::from_scale_rotation_translation(Vec3::new(0.11, 0.3, 0.11) * (1.0 - t * 0.35), tilt, at),
                    lin(BONE),
                    1.0,
                    Vec3::ZERO,
                    SHADE_BONE,
                ));
            }
            cube(out, base + Vec3::new(0.0, -0.8, 0.0), Vec3::new(0.16, 1.5, 0.16), 0.0, lin(BONE_D) * 0.7, SHADE_BONE);
        }
    }

    /// Where a rib-bridge fell: stumps of rib standing out of the water, a vertebra afloat.
    fn build_fallen_rib(&mut self, p: P, h: f64) {
        let c = Vec3::new(p.x as f32, 0.0, p.y as f32);
        let out = &mut self.static_cubes;
        if h > 0.35 {
            let a = h as f32 * TAU;
            let at = c + Vec3::new(a.cos() * 0.25, 0.05, a.sin() * 0.25);
            out.push(Instance::new(
                Mat4::from_scale_rotation_translation(
                    Vec3::new(0.11, 0.7, 0.11),
                    Quat::from_rotation_y(a) * Quat::from_rotation_x(0.55),
                    at,
                ),
                lin(BONE),
                1.0,
                Vec3::ZERO,
                SHADE_BONE,
            ));
        }
        if h < 0.5 {
            cube(out, c + Vec3::new(0.1, WATER_Y + 0.02, -0.1), Vec3::new(0.34, 0.12, 0.3), h as f32 * 5.0, lin(BONE_D), SHADE_BONE);
        }
    }

    /// The Drowned Floor, shifted by `off`: where it is (`copy` false), or beneath the Waterline as
    /// the water shows it. Warm floors of gold and jade mosaic, walls of basalt banded in verdigris
    /// bronze, stone lanterns, kelp, coral, clams, scattered coins; the vents; the wreck of every
    /// rib-bridge that fell from above. Its lanterns go into `lamps`.
    fn build_drowned(&mut self, g: &Ground, r: &Grotto, art: &GrottoArt, off: Vec3, copy: bool, lamps: &mut Vec<Vec3>) {
        let fallen: HashSet<P> = r.fallen.iter().map(|&p| below(p)).collect();
        let exit = r.base.exit;
        let front = exit.front();
        for y in 0..g.grid.height {
            for x in OX..WIDTH {
                let p = P::new(x, y);
                let q = above(p);
                if copy {
                    // Beneath the Waterline's rock, where nothing shows through, nothing is drawn.
                    let shows = (-1..=1).any(|dx| (0..=3).any(|dy| !g.rock(q.x + dx, y + dy)));
                    if !shows {
                        continue;
                    }
                }
                let (xf, yf) = (x as f32, y as f32);
                let at = Vec3::new(xf, 0.0, yf) + off;
                let h = hash(x as f64 * 1.3 + 7.0, y as f64 * 2.9);
                let h2 = hash(x as f64 * 4.3 + 1.0, y as f64 * 1.9 + 9.0);
                let t = g.get(x, y);
                if t == Tile::Rock {
                    if p == exit.tile {
                        if copy {
                            self.build_copy_portal(at, exit.facing);
                        }
                        continue;
                    }
                    let open = g.open_sides(p);
                    if !open.is_empty() {
                        self.build_drowned_wall(p, at, &open, h, copy);
                    } else if copy {
                        cube(
                            &mut self.static_cubes,
                            at + Vec3::new(0.0, DROWNED_WALL - 0.03, 0.0),
                            Vec3::new(1.0, 0.1, 1.0),
                            0.0,
                            lin(0x1e2a26),
                            SHADE_LIT,
                        );
                    }
                    continue;
                }
                let out = &mut self.static_cubes;
                let bell = r.cavern_at(p) == Some(BELL);
                // The floor: a slab, and a mosaic tile set in it, warm with the light from below.
                let base =
                    if bell { lin(FLOOR_GOLD) } else { lin(if h > 0.55 { FLOOR_SAND } else { FLOOR_GOLD }) } * (0.85 + h2 as f32 * 0.25);
                out.push(Instance::new(
                    Mat4::from_scale_rotation_translation(Vec3::new(0.98, 0.14, 0.98), Quat::IDENTITY, at + Vec3::new(0.0, -0.07, 0.0)),
                    base,
                    1.0,
                    base * if copy { 0.45 } else { 0.05 },
                    SHADE_LIT,
                ));
                let ring = ((xf - front.x as f32).powi(2) + (yf - front.y as f32).powi(2)).sqrt();
                let inlay = if bell {
                    if (ring as i32) % 2 == 0 {
                        lin(GOLD)
                    } else {
                        lin(FLOOR_JADE)
                    }
                } else if (x + y) % 2 == 0 {
                    lin(FLOOR_JADE)
                } else {
                    lin(FLOOR_GOLD) * 1.1
                };
                let shade = if bell && (ring as i32) % 2 == 0 { SHADE_BRONZE } else { SHADE_LIT };
                out.push(Instance::new(
                    Mat4::from_scale_rotation_translation(
                        Vec3::new(0.62, 0.012, 0.62),
                        Quat::from_rotation_y(PI / 4.0),
                        at + Vec3::new(0.0, 0.004, 0.0),
                    ),
                    inlay,
                    1.0,
                    inlay * if copy { 0.5 } else { 0.08 },
                    shade,
                ));
                if r.vent_at(p).is_some() {
                    // A geyser's vent: a cone of basalt round a dark throat, warm light in it.
                    for k in 0..8 {
                        let a = k as f32 / 8.0 * TAU;
                        cube(
                            out,
                            at + Vec3::new(a.cos() * 0.36, 0.09, a.sin() * 0.36),
                            Vec3::new(0.2, 0.18, 0.14),
                            -a,
                            lin(BASALT_L),
                            SHADE_WET,
                        );
                    }
                    cube_glow(
                        out,
                        at + Vec3::new(0.0, 0.02, 0.0),
                        Vec3::new(0.42, 0.02, 0.42),
                        Quat::IDENTITY,
                        lin(0xffe8a0),
                        0.6,
                        SHADE_LIGHT,
                    );
                    continue;
                }
                if fallen.contains(&p) {
                    // The bridge that fell from above, in pieces on the floor.
                    for k in 0..3 {
                        let a = h as f32 * TAU + k as f32 * 1.9;
                        cube(
                            out,
                            at + Vec3::new(a.cos() * 0.25, 0.05, a.sin() * 0.25),
                            Vec3::new(0.6, 0.09, 0.1),
                            a,
                            lin(BONE_D),
                            SHADE_BONE,
                        );
                    }
                    continue;
                }
                let jit = Vec3::new((h as f32 - 0.5) * 0.5, 0.0, (h2 as f32 - 0.5) * 0.5);
                let wall_side = CARDINALS.iter().copied().find(|d| {
                    let n = p.step(*d);
                    g.rock(n.x, n.y)
                });
                if let Some(d) = wall_side.filter(|_| (x * 7 + y * 13).rem_euclid(9) == 0 && !bell) {
                    // A stone lantern against the wall.
                    let (dx, dy) = d.vec();
                    let spot = at + Vec3::new(dx as f32 * 0.22, 0.0, dy as f32 * 0.22);
                    push_merged(out, &art.lantern, Mat4::from_translation(spot) * Mat4::from_rotation_y(h as f32), 0.0);
                    lamps.push(spot + Vec3::new(0.0, 0.85, 0.0));
                    continue;
                }
                match (h2 * 100.0) as u32 {
                    0..=9 => {
                        for k in 0..3 {
                            let a = k as f32 * 2.3 + h as f32 * 9.0;
                            cube(
                                out,
                                at + jit + Vec3::new(a.cos() * 0.15, 0.012, a.sin() * 0.15),
                                Vec3::new(0.08, 0.02, 0.08),
                                a,
                                lin(GOLD),
                                SHADE_BRONZE,
                            );
                        }
                    }
                    10..=14 if wall_side.is_some() => {
                        // Kelp, swaying up into the green.
                        for strand in 0..3 {
                            let sx = (strand as f32 - 1.0) * 0.18 + jit.x * 0.4;
                            let tall = 3 + ((h * 10.0) as usize + strand) % 3;
                            for k in 0..tall {
                                let lean = ((k as f32 * 0.9 + strand as f32).sin()) * 0.08;
                                cube(
                                    out,
                                    at + Vec3::new(sx + lean, 0.14 + k as f32 * 0.26, jit.z * 0.4),
                                    Vec3::new(0.07, 0.28, 0.04),
                                    lean * 4.0,
                                    lin(KELP) * (0.85 + k as f32 * 0.06),
                                    SHADE_WET,
                                );
                            }
                        }
                    }
                    15..=18 => {
                        // Coral.
                        let c = if h > 0.5 { lin(CORAL) } else { lin(CORAL_L) };
                        cube(out, at + jit + Vec3::new(0.0, 0.12, 0.0), Vec3::new(0.08, 0.24, 0.08), 0.0, c, SHADE_LIT);
                        for k in 0..4 {
                            let a = k as f32 * PI / 2.0 + h as f32;
                            out.push(Instance::new(
                                Mat4::from_scale_rotation_translation(
                                    Vec3::new(0.06, 0.2, 0.06),
                                    Quat::from_rotation_y(a) * Quat::from_rotation_z(0.6),
                                    at + jit + Vec3::new(a.cos() * 0.08, 0.26, a.sin() * 0.08),
                                ),
                                c * 1.1,
                                1.0,
                                c * 0.05,
                                SHADE_LIT,
                            ));
                        }
                    }
                    19..=20 if wall_side.is_none() => {
                        push_merged(out, &art.clam, Mat4::from_translation(at + jit * 0.4) * Mat4::from_rotation_y(h as f32 * TAU), 0.02)
                    }
                    21..=24 => cube(
                        out,
                        at + jit + Vec3::new(0.0, 0.02, 0.0),
                        Vec3::new(0.15, 0.04, 0.12),
                        h as f32 * 4.0,
                        lin(NACRE),
                        SHADE_NACRE,
                    ),
                    25..=26 => {
                        // A drum of a fallen column, green-banded.
                        let a = h as f32 * TAU;
                        out.push(Instance::new(
                            Mat4::from_scale_rotation_translation(
                                Vec3::new(0.7, 0.36, 0.36),
                                Quat::from_rotation_y(a),
                                at + Vec3::new(0.0, 0.18, 0.0),
                            ),
                            lin(BASALT_L) * 1.4,
                            1.0,
                            Vec3::ZERO,
                            SHADE_WET,
                        ));
                        cube(out, at + Vec3::new(0.0, 0.18, 0.0), Vec3::new(0.08, 0.38, 0.38), a, lin(VERDIGRIS), SHADE_BRONZE);
                    }
                    _ => {}
                }
            }
        }
    }

    /// A wall of the Drowned Floor: basalt banded in verdigris bronze, a cornice, pilasters of the
    /// drowned temple along it.
    fn build_drowned_wall(&mut self, p: P, at: Vec3, open: &[Dir], h: f64, copy: bool) {
        let start = self.static_cubes.len();
        let out = &mut self.static_cubes;
        cube(out, at + Vec3::new(0.0, DROWNED_WALL / 2.0, 0.0), Vec3::new(1.0, DROWNED_WALL, 1.0), 0.0, basalt(h) * 1.15, SHADE_WET);
        cube(out, at + Vec3::new(0.0, 0.16, 0.0), Vec3::new(1.02, 0.08, 1.02), 0.0, lin(VERDIGRIS_D), SHADE_BRONZE);
        cube(out, at + Vec3::new(0.0, DROWNED_WALL - 0.04, 0.0), Vec3::new(1.04, 0.1, 1.04), 0.0, lin(BASALT_D) * 1.2, SHADE_LIT);
        for &d in open {
            let (dx, dy) = d.vec();
            let n = Vec3::new(dx as f32, 0.0, dy as f32);
            let side = Vec3::new(dy as f32, 0.0, dx as f32).abs();
            cube(
                out,
                at + n * 0.5 + Vec3::new(0.0, DROWNED_WALL - 0.1, 0.0),
                side * 1.04 + n.abs() * 0.08 + Vec3::new(0.0, 0.07, 0.0),
                0.0,
                lin(VERDIGRIS_D),
                SHADE_BRONZE,
            );
        }
        for &d in open {
            let (dx, dy) = d.vec();
            let along = if dx == 0 { p.x } else { p.y };
            let n = Vec3::new(dx as f32, 0.0, dy as f32);
            let side = Vec3::new(dy as f32, 0.0, dx as f32).abs();
            let face = at + n * 0.5;
            if along.rem_euclid(4) == 0 {
                cube(
                    out,
                    face + n * 0.04 + Vec3::new(0.0, DROWNED_WALL / 2.0, 0.0),
                    side * 0.22 + n.abs() * 0.1 + Vec3::new(0.0, DROWNED_WALL - 0.1, 0.0),
                    0.0,
                    lin(VERDIGRIS),
                    SHADE_BRONZE,
                );
                cube(
                    out,
                    face + n * 0.06 + Vec3::new(0.0, DROWNED_WALL - 0.2, 0.0),
                    side * 0.3 + n.abs() * 0.14 + Vec3::new(0.0, 0.1, 0.0),
                    0.0,
                    lin(BRONZE),
                    SHADE_BRONZE,
                );
            } else if along.rem_euclid(4) == 2 {
                // A relief of waves, in bronze.
                for k in 0..3 {
                    let s = (k as f32 - 1.0) * 0.28;
                    cube(
                        out,
                        face + n * 0.02 + side * s + Vec3::new(0.0, 0.7 + (k as f32 * 1.7).sin() * 0.08, 0.0),
                        side * 0.2 + n.abs() * 0.03 + Vec3::new(0.0, 0.08, 0.0),
                        0.0,
                        lin(VERDIGRIS_D) * 1.2,
                        SHADE_BRONZE,
                    );
                }
            }
        }
        if !copy {
            let len = self.static_cubes.len() - start;
            let shades: Vec<f32> = self.static_cubes[start..start + len].iter().map(|i| i.emissive[3]).collect();
            self.trees.push(Tree {
                range: (start, len),
                base: at,
                radius: 0.72,
                height: DROWNED_WALL + 0.3,
                fade: 1.0,
                alive: true,
                shades: Some(shades),
            });
        }
    }

    /// The court's portal as it shows through the water from the Waterline: its towers, its lintel
    /// and the green beyond (the real one is the framework's, where it stands).
    fn build_copy_portal(&mut self, at: Vec3, facing: Dir) {
        let yaw = {
            let (dx, dz) = facing.vec();
            (dx as f32).atan2(dz as f32)
        };
        let root = Mat4::from_translation(at) * Mat4::from_rotation_y(yaw) * Mat4::from_scale(Vec3::splat(0.7));
        let out = &mut self.static_cubes;
        let put = |out: &mut Vec<Instance>, local: Vec3, size: Vec3, color: Vec3, glow: f32, shade: f32| {
            out.push(Instance::new(
                root * Mat4::from_scale_rotation_translation(size, Quat::IDENTITY, local),
                color,
                1.0,
                color * glow,
                shade,
            ));
        };
        for side in [-1.0f32, 1.0] {
            put(out, Vec3::new(side * 0.92, 1.3, 0.0), Vec3::new(0.46, 2.6, 0.58), lin(0x1a1622), 0.0, SHADE_LIT);
            put(out, Vec3::new(side * 0.92, 2.4, 0.0), Vec3::new(0.52, 0.08, 0.62), lin(ALGAE), 0.5, SHADE_LIT);
        }
        put(out, Vec3::new(0.0, 2.75, 0.0), Vec3::new(2.4, 0.22, 0.7), lin(0x1a1622), 0.0, SHADE_LIT);
        put(out, Vec3::new(0.0, 1.35, -0.1), Vec3::new(1.4, 2.6, 0.06), lin(ALGAE), 1.4, SHADE_LIGHT);
    }

    /// The solid rock between the caverns, at the walls' height: rough basalt on the Waterline, each
    /// tile its own slab a little higher or lower, lumps and stalagmites on it, a glow of algae in its
    /// cracks now and then; a darker cap on the Drowned Floor (one box for each run along a row).
    fn build_caps(&mut self, g: &Ground) {
        let faces = |q: P| {
            CARDINALS.iter().any(|d| {
                let n = q.step(*d);
                g.grid.in_bounds(n.x, n.y) && !g.rock(n.x, n.y)
            })
        };
        let solid = |x: i32, y: i32| g.rock(x, y) && !faces(P::new(x, y));
        for y in 0..g.grid.height {
            for x in 0..OX {
                if !solid(x, y) {
                    continue;
                }
                let h = hash(x as f64 * 2.7, y as f64 * 3.3);
                let h2 = hash(x as f64 * 1.1 + 5.0, y as f64 * 4.7);
                let lift = (h2 as f32 - 0.5) * 0.16;
                cube(
                    &mut self.static_cubes,
                    Vec3::new(x as f32, WALL_H - 0.03 + lift, y as f32),
                    Vec3::new(1.0, 0.12, 1.0),
                    0.0,
                    basalt(h2) * 0.85,
                    SHADE_LIT,
                );
                if h > 0.7 {
                    let s = 0.3 + (h as f32 - 0.7) * 2.0;
                    let tall = 0.1 + (h as f32 - 0.7) * 1.4;
                    cube(
                        &mut self.static_cubes,
                        Vec3::new(x as f32 + (h2 as f32 - 0.5) * 0.4, WALL_H + lift + tall / 2.0, y as f32),
                        Vec3::new(s, tall, s * 0.8),
                        h as f32 * 9.0,
                        basalt(h) * 0.9,
                        SHADE_LIT,
                    );
                } else if h < 0.04 {
                    cube_glow(
                        &mut self.static_cubes,
                        Vec3::new(x as f32, WALL_H + lift + 0.035, y as f32),
                        Vec3::new(0.3, 0.02, 0.2),
                        Quat::from_rotation_y(h2 as f32 * 3.0),
                        lin(ALGAE_D) * 0.6,
                        0.0,
                        SHADE_LIGHT,
                    );
                }
            }
            let mut x = OX;
            while x < WIDTH {
                if !solid(x, y) {
                    x += 1;
                    continue;
                }
                let a = x;
                while x < WIDTH && solid(x, y) {
                    x += 1;
                }
                let n = (x - a) as f32;
                cube(
                    &mut self.static_cubes,
                    Vec3::new(a as f32 + n / 2.0 - 0.5, DROWNED_WALL - 0.03, y as f32),
                    Vec3::new(n, 0.1, 1.0),
                    0.0,
                    lin(0x1e2a26),
                    SHADE_LIT,
                );
            }
        }
    }
}
