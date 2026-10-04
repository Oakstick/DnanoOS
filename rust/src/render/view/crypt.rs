//! The Bone Vaults (dungeon III, and the estates deeper still): the first level built to the art
//! direction (the *Voxel Dungeon* board, stage B). The estate's maze is cut from stone instead of
//! hedges: walls of dressed blocks in charcoal and umber on pale-stone plinths,
//! verdigris creeping over some blocks, monumental pillars, banners of copper and umber cloth,
//! wet flagstones underfoot, and amber braziers burning against a cold mist-teal dark.
//!
//! The palette is the board's six: charcoal, umber, copper, amber, mist teal and pale stone. The
//! albedos below are lighter than the swatches, which are the colours as they read in the
//! board's dim light; under the vaults' own light they come out near them.

use super::*;

pub(super) mod pal {
    /// Stone: charcoal greys with a cold cast, umber browns, pale stone for dressings.
    pub const CHARCOAL: [u32; 3] = [0x4a5250, 0x3f4745, 0x535a57];
    pub const UMBER: [u32; 2] = [0x5e4c40, 0x52423a];
    pub const PALE_STONE: [u32; 2] = [0x8e8878, 0x7e7868];
    pub const VERDIGRIS: u32 = 0x4e6e66;
    pub const GROUT: u32 = 0x1a1f1e;
    pub const FLAG: [u32; 3] = [0x4c5452, 0x464e4c, 0x504a44];
    pub const COPPER: u32 = 0x9a5b3c;
    pub const BANNER: [u32; 2] = [0x7a3424, 0x5a3a2c];
    pub const AMBER: u32 = 0xd6a15b;
    pub const EMBER: u32 = 0xffa040;
    pub const BONE: u32 = 0xcfc7b0;
    pub const MIST_TEAL: u32 = 0x6e9a90;
    /// Moss in three greens, where the damp gathers.
    pub const MOSS: [u32; 3] = [0x2e4426, 0x3a5230, 0x283c22];
    pub const IRON: u32 = 0x2a2624;
    /// The dark beyond the light: charcoal with the mist's teal in it.
    pub const DARK: u32 = 0x1a2c2a;
}
use pal::*;

/// Dressed stone and wet flagstone (`voxel_surface` kinds 13 and 14).
pub const SHADE_STONE: f32 = 18.0;
pub const SHADE_FLAG: f32 = 19.0;
const SHADE_CLOTH: f32 = 8.0;
/// Soft, strand-shaded growth (`voxel_surface` fur): moss and leaves.
const SHADE_SOFT: f32 = 7.0;
const SHADE_IRON: f32 = 6.0;
const SHADE_BRONZE: f32 = 13.0;

/// How high the vaults' walls stand above their floor (just above his chest, so the top view
/// sees him past them), and their courses of blocks.
const WALL_H: f32 = 1.15;
const COURSES: usize = 3;

fn jit(hex: u32, h: f64, amount: f32) -> Vec3 {
    lin(hex) * (1.0 - amount + h as f32 * amount * 2.0)
}

fn block(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot_y: f32, color: Vec3, shade: f32) {
    let m = Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(rot_y), pos);
    out.push(Instance::new(m, color, 1.0, Vec3::ZERO, shade));
}

fn block_m(out: &mut Vec<Instance>, m: Mat4, color: Vec3, shade: f32) {
    out.push(Instance::new(m, color, 1.0, Vec3::ZERO, shade));
}

fn glowing_block(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, color: Vec3, glow: Vec3) {
    let m = Mat4::from_scale_rotation_translation(scale, Quat::IDENTITY, pos);
    out.push(Instance::new(m, color, 1.0, glow, SHADE_LIT));
}

/// The vaults' light: no sky, a cold teal dark lit by its braziers, and a faint teal light
/// falling from shafts far above (it rims the tops of things).
pub fn crypt_atmosphere() -> Atmosphere {
    Atmosphere {
        background: DARK,
        fog_near: 7.0,
        fog_far: 26.0,
        hemi_sky: lin(MIST_TEAL) * 0.35,
        hemi_ground: lin(0x3a2e27) * 0.12,
        ambient: lin(0x2a3634) * 0.9,
        sun_dir: Vec3::new(0.35, 1.0, 0.25).normalize(),
        sun: lin(0x9ec4bc) * 0.3,
        exposure: 1.35,
        night_sky: 0.0,
        bloom: 1.1,
        moon_view: Vec3::new(-1.103, 0.534, -0.311).normalize(),
        cloud_tint: lin(DARK),
    }
}

impl WorldView {
    pub(super) fn build_crypt(&mut self, grid: &Grid) {
        let floor_y = |x: i32, y: i32| grid.elevation(x, y) as f32 * STEP;
        let rock = |x: i32, y: i32| grid.get(x, y) == Tile::Rock;

        // ---- floors: flagstones in their grout, raised ground on umber blocks
        for (x, y, t) in grid.cells() {
            if t == Tile::Rock || t == Tile::Pit || grid.is_ramp(x, y) || grid.is_bridge(x, y) {
                continue;
            }
            let (xf, yf) = (x as f32, y as f32);
            let (hx, hy) = (x as f64, y as f64);
            let h = floor_y(x, y);
            if h > 0.0 {
                block(
                    &mut self.static_cubes,
                    Vec3::new(xf, h / 2.0 - 0.06, yf),
                    Vec3::new(1.0, h + 0.08, 1.0),
                    0.0,
                    jit(UMBER[1], hash(hx, hy), 0.08),
                    SHADE_STONE,
                );
            }
            if t == Tile::Nest {
                // The hollow the dead climb out of: a sunken pit of bones.
                block(&mut self.static_cubes, Vec3::new(xf, h - 0.06, yf), Vec3::new(1.0, 0.1, 1.0), 0.0, lin(HOLLOW), SHADE_STONE);
                for k in 0..3 {
                    let hk = hash(hx * 3.1 + k as f64, hy * 1.7) as f32;
                    self.bone(Vec3::new(xf + (hk - 0.5) * 0.7, h, yf + (0.5 - hash(hy, hx + k as f64) as f32) * 0.7), hk);
                }
                continue;
            }
            block(&mut self.static_cubes, Vec3::new(xf, h - 0.06, yf), Vec3::new(1.0, 0.1, 1.0), 0.0, lin(GROUT), SHADE_LIT);
            for (k, (ox, oz)) in [(-0.25f32, -0.25f32), (0.25, -0.25), (-0.25, 0.25), (0.25, 0.25)].into_iter().enumerate() {
                let hk = hash(hx * 2.0 + ox as f64 * 7.0 + 1.3, hy * 2.0 + oz as f64 * 5.0);
                let c = jit(FLAG[(hk * 3.0) as usize % 3], hash(hy + k as f64, hx), 0.1);
                let tilt = (hk as f32 - 0.5) * 0.012;
                block(
                    &mut self.static_cubes,
                    Vec3::new(xf + ox, h - 0.025 + tilt, yf + oz),
                    Vec3::new(0.47, 0.05, 0.47),
                    0.0,
                    c,
                    SHADE_FLAG,
                );
            }
            // Moss where the damp gathers in the seams.
            self.floor_growth(xf, yf, h, hx, hy);
            // Now and then the bones of the vaults' dead, swept to the foot of a wall.
            if hash(hx * 5.0 + 3.0, hy * 7.0) < 0.05
                && CARDINALS.iter().any(|d| {
                    let (dx, dy) = d.vec();
                    rock(x + dx, y + dy)
                })
            {
                let hk = hash(hx, hy * 3.0) as f32;
                self.bone(Vec3::new(xf + (hk - 0.5) * 0.5, h, yf + (0.5 - hk) * 0.5), hk);
            }
        }

        // ---- ramps: pale stone steps toward the higher neighbour
        for (x, y, _) in grid.cells() {
            if !grid.is_ramp(x, y) || grid.is_bridge(x, y) {
                continue;
            }
            let low = grid.elevation(x, y);
            let up = CARDINALS
                .iter()
                .copied()
                .find(|&d| {
                    let n = grid.neighbor(P::new(x, y), d);
                    grid.get(n.x, n.y) != Tile::Rock && grid.elevation(n.x, n.y) > low
                })
                .unwrap_or(Dir::Up);
            let (vx, vy) = up.vec();
            for i in 0..3 {
                let top = low as f32 * STEP + ((i + 1) as f32 / 3.0) * STEP;
                let thick = top + 0.1;
                let o = (i as f32 - 1.0) / 3.0;
                let h = hash(x as f64 + i as f64, y as f64);
                let m = Mat4::from_translation(Vec3::new(x as f32 + vx as f32 * o, (top - 0.1) / 2.0, y as f32 + vy as f32 * o))
                    * Mat4::from_scale(Vec3::new(if vx != 0 { 1.0 / 3.0 } else { 1.0 }, thick, if vy != 0 { 1.0 / 3.0 } else { 1.0 }));
                self.static_cubes.push(Instance::new(m, jit(PALE_STONE[1], h, 0.06), 1.0, Vec3::ZERO, SHADE_STONE));
            }
        }

        // ---- pits: a black drop with a pale stone kerb
        for (x, y, t) in grid.cells() {
            if t != Tile::Pit {
                continue;
            }
            let bottom = Mat4::from_translation(Vec3::new(x as f32, -0.9, y as f32)) * Mat4::from_scale(Vec3::new(1.0, 0.1, 1.0));
            self.static_cubes.push(Instance::new(bottom, lin(PIT), 1.0, Vec3::ZERO, SHADE_LIT));
            let rim = Mat4::from_translation(Vec3::new(x as f32, -0.03, y as f32)) * Mat4::from_scale(Vec3::new(1.02, 0.06, 1.02));
            self.static_cubes.push(Instance::new(rim, lin(PALE_STONE[1]) * 0.7, 1.0, Vec3::ZERO, SHADE_STONE));
        }

        // ---- the walls: dressed stone, faced along every road; pillars, banners and braziers
        let mut braziers: Vec<(P, Dir)> = Vec::new();
        for (x, y, t) in grid.cells() {
            if t != Tile::Rock {
                continue;
            }
            // The gatehouse stands on the gate's tile and the two either side of it.
            if self.gate_tile.is_some_and(|g| g.y == y && (g.x - x).abs() <= 1) {
                continue;
            }
            let (xf, yf) = (x as f32, y as f32);
            let (hx, hy) = (x as f64, y as f64);
            let base = floor_y(x, y);
            let top = base + WALL_H;
            let start = self.static_cubes.len();
            // The core, a touch inside the faces so the blocks stand proud of it.
            block(
                &mut self.static_cubes,
                Vec3::new(xf, (top - 0.1) / 2.0, yf),
                Vec3::new(0.96, top + 0.1, 0.96),
                0.0,
                jit(CHARCOAL[1], hash(hx, hy), 0.05) * 0.8,
                SHADE_STONE,
            );
            let roads: Vec<Dir> = CARDINALS
                .iter()
                .copied()
                .filter(|d| {
                    let (dx, dy) = d.vec();
                    grid.in_bounds(x + dx, y + dy) && !rock(x + dx, y + dy)
                })
                .collect();
            for &d in &roads {
                self.build_masonry(xf, yf, base, d, hx, hy);
            }
            // A coping of dark stone across the top.
            block(
                &mut self.static_cubes,
                Vec3::new(xf, top + 0.04, yf),
                Vec3::new(1.0, 0.08, 1.0),
                0.0,
                jit(CHARCOAL[(hash(hx + 3.0, hy) * 3.0) as usize % 3], hash(hy, hx), 0.08) * 0.85,
                SHADE_STONE,
            );
            self.wall_growth(xf, yf, top, hx, hy);
            let len = self.static_cubes.len() - start;
            self.rock_index.insert(tile_key(x, y), (start, len));
            // A wall on raised ground stands well over his head from the road below it: it fades
            // out of the way of the view of him, as trees do.
            let lowest = roads.iter().map(|d| {
                let (dx, dy) = d.vec();
                floor_y(x + dx, y + dy)
            });
            if lowest.fold(f32::INFINITY, f32::min) < base - 0.1 {
                let shades = self.static_cubes[start..start + len].iter().map(|c| c.emissive[3]).collect();
                self.tree_index.insert(tile_key(x, y), self.trees.len());
                self.trees.push(Tree {
                    range: (start, len),
                    base: Vec3::new(xf, 0.0, yf),
                    radius: 0.7,
                    height: top,
                    fade: 1.0,
                    alive: true,
                    shades: Some(shades),
                });
            }
            if roads.is_empty() {
                continue;
            }
            let pick = hash(hx * 5.0 + 1.0, hy * 3.0 + 2.0);
            if hash(hx * 3.0, hy * 7.0) < 0.13 {
                braziers.push((P::new(x, y), roads[0]));
            } else if pick < 0.07 {
                self.pillar(xf, yf, base, roads[0], hx, hy);
            } else if pick < 0.16 {
                self.banner(xf, yf, base, roads[(pick * 50.0) as usize % roads.len()], hx, hy);
            } else if pick < 0.2 {
                self.statue(xf, yf, top + 0.08, roads[0], hx, hy);
            } else if pick < 0.25 {
                self.wall_lantern(P::new(x, y), base, roads[(pick * 70.0) as usize % roads.len()]);
            }
        }
        // ---- arches over the passages (up to three wide), each with a lantern hung from its crown
        let open = |x: i32, y: i32| grid.in_bounds(x, y) && !matches!(grid.get(x, y), Tile::Rock | Tile::Pit) && !grid.is_ramp(x, y);
        let mut last_arch: Option<(P, bool)> = None;
        for (x, y, _) in grid.cells() {
            for across_x in [true, false] {
                // From a wall on the west (or north) side: the run of floor across to the wall opposite.
                let (sx, sy) = if across_x { (1, 0) } else { (0, 1) };
                if !open(x, y) || !rock(x - sx, y - sy) {
                    continue;
                }
                let mut width = 0;
                while width < 4 && open(x + sx * width, y + sy * width) {
                    width += 1;
                }
                if width == 0 || width > 3 || !rock(x + sx * width, y + sy * width) {
                    continue;
                }
                // A passage, not a junction: open before and behind along it, all one height.
                let (ax, ay) = (sy, sx);
                let h0 = floor_y(x, y);
                let passage = (0..width).all(|k| {
                    let (tx, ty) = (x + sx * k, y + sy * k);
                    open(tx + ax, ty + ay) && open(tx - ax, ty - ay) && (floor_y(tx, ty) - h0).abs() < 0.01
                });
                let (hx, hy) = (x as f64, y as f64);
                if !passage || hash(hx * 2.3 + 7.0, hy * 1.9 + 3.0) > 0.07 {
                    continue;
                }
                // Not one just behind another.
                if last_arch.is_some_and(|(p, a)| a == across_x && (p.x - x).abs() + (p.y - y).abs() <= 2) {
                    continue;
                }
                last_arch = Some((P::new(x, y), across_x));
                self.arch(P::new(x, y), h0, across_x, width);
            }
        }
        for (spot, d) in braziers {
            self.brazier(grid, spot, d);
        }
        self.build_mist(grid, &HashMap::new());
        // The vaults' mist is the board's teal.
        for m in self.mist.iter_mut() {
            let c = lin(MIST_TEAL) * 0.55;
            m.color[0] = c.x;
            m.color[1] = c.y;
            m.color[2] = c.z;
        }
    }

    /// The road-facing side `d` of a wall tile in courses of dressed blocks of uneven length,
    /// staggered, on a pale plinth; here and there a block of umber, or green with verdigris.
    fn build_masonry(&mut self, x: f32, y: f32, base: f32, d: Dir, hx: f64, hy: f64) {
        let (dx, dy) = d.vec();
        let turn = if dx != 0 { PI / 2.0 } else { 0.0 };
        let at = |u: f32, h: f32, out: f32| {
            Vec3::new(x + dx as f32 * out + if dx != 0 { 0.0 } else { u }, h, y + dy as f32 * out + if dx != 0 { u } else { 0.0 })
        };
        // The plinth.
        let hk = hash(hx * 3.0 + dx as f64, hy * 5.0 + dy as f64);
        block(
            &mut self.static_cubes,
            at(0.0, base + 0.1, 0.47),
            Vec3::new(1.0, 0.2, 0.1),
            turn,
            jit(PALE_STONE[1], hk, 0.05) * 0.8,
            SHADE_STONE,
        );
        let course_h = (WALL_H - 0.2) / COURSES as f32;
        for course in 0..COURSES {
            let shift = if course % 2 == 0 { 0.0 } else { 0.17 };
            let mut u: f32 = -0.5 - shift;
            let mut k = 0;
            while u < 0.5 {
                let hk = hash(hx * 7.0 + k as f64 * 1.3 + course as f64 * 5.0 + dx as f64, hy * 3.0 + dy as f64 * 7.0 + course as f64);
                let len = 0.3 + hk as f32 * 0.22;
                let (a, b) = (u.max(-0.5), (u + len).min(0.5));
                if b - a > 0.06 {
                    let roll = hash(hk * 31.0, hx + hy);
                    let c = if roll < 0.04 {
                        jit(VERDIGRIS, hk, 0.1)
                    } else if roll < 0.19 {
                        jit(UMBER[(hk * 2.0) as usize % 2], hk, 0.1)
                    } else {
                        jit(CHARCOAL[(hk * 3.0) as usize % 3], hk, 0.12)
                    };
                    let proud = 0.44 + (roll as f32) * 0.03;
                    let h = base + 0.2 + course as f32 * course_h + course_h / 2.0;
                    block(
                        &mut self.static_cubes,
                        at((a + b) / 2.0, h, proud),
                        Vec3::new(b - a - 0.025, course_h - 0.025, 0.08),
                        turn,
                        c,
                        SHADE_STONE,
                    );
                }
                u += len;
                k += 1;
            }
        }
    }

    /// A monumental pillar against the wall: a square shaft of dark stone and umber drums on a
    /// moulded pale base, under a pale capital, standing well above the wall.
    fn pillar(&mut self, x: f32, y: f32, base: f32, d: Dir, hx: f64, hy: f64) {
        let (dx, dy) = d.vec();
        let p = Vec3::new(x + dx as f32 * 0.32, base, y + dy as f32 * 0.32);
        let start = self.static_cubes.len();
        block(
            &mut self.static_cubes,
            p + Vec3::Y * 0.12,
            Vec3::new(0.62, 0.24, 0.62),
            0.0,
            jit(PALE_STONE[0], hash(hx, hy), 0.05),
            SHADE_STONE,
        );
        let drums = 6;
        let h = 2.4;
        for k in 0..drums {
            let dh = (h - 0.5) / drums as f32;
            let c =
                if k % 3 == 1 { jit(UMBER[0], hash(hy, hx + k as f64), 0.06) } else { jit(CHARCOAL[k % 3], hash(hx + k as f64, hy), 0.08) };
            block(
                &mut self.static_cubes,
                p + Vec3::Y * (0.24 + dh * (k as f32 + 0.5)),
                Vec3::new(0.46, dh - 0.02, 0.46),
                0.0,
                c,
                SHADE_STONE,
            );
        }
        block(
            &mut self.static_cubes,
            p + Vec3::Y * (h - 0.13),
            Vec3::new(0.66, 0.26, 0.66),
            0.0,
            jit(PALE_STONE[1], hash(hy, hx), 0.05) * 0.75,
            SHADE_STONE,
        );
        // It stands taller than him: it fades out of the way of the view of him, as trees do.
        let len = self.static_cubes.len() - start;
        self.trees.push(Tree {
            range: (start, len),
            base: p,
            radius: 0.4,
            height: h,
            fade: 1.0,
            alive: true,
            shades: Some(vec![SHADE_STONE; len]),
        });
    }

    /// A banner on the wall's face: a long drop of copper-red or umber cloth from a copper rod,
    /// an amber sigil sewn on it, its foot cut in a swallowtail.
    fn banner(&mut self, x: f32, y: f32, base: f32, d: Dir, hx: f64, hy: f64) {
        let (dx, dy) = d.vec();
        let turn = if dx != 0 { PI / 2.0 } else { 0.0 };
        let at = |h: f32, out: f32, u: f32| {
            Vec3::new(x + dx as f32 * out + if dx != 0 { 0.0 } else { u }, base + h, y + dy as f32 * out + if dx != 0 { u } else { 0.0 })
        };
        let cloth = lin(BANNER[(hash(hx, hy * 3.0) * 2.0) as usize % 2]);
        block(&mut self.static_cubes, at(1.42, 0.53, 0.0), Vec3::new(0.62, 0.04, 0.04), turn, lin(COPPER), SHADE_BRONZE);
        block(&mut self.static_cubes, at(0.95, 0.52, 0.0), Vec3::new(0.48, 0.9, 0.02), turn, cloth, SHADE_CLOTH);
        for u in [-0.15f32, 0.15] {
            block(&mut self.static_cubes, at(0.43, 0.52, u), Vec3::new(0.18, 0.14, 0.02), turn, cloth, SHADE_CLOTH);
        }
        // The sigil: a ring and a cross bar, in amber thread.
        let thread = lin(AMBER);
        block(&mut self.static_cubes, at(1.08, 0.535, 0.0), Vec3::new(0.06, 0.32, 0.01), turn, thread, SHADE_CLOTH);
        block(&mut self.static_cubes, at(1.12, 0.535, 0.0), Vec3::new(0.24, 0.05, 0.01), turn, thread, SHADE_CLOTH);
        block(&mut self.static_cubes, at(0.92, 0.535, 0.0), Vec3::new(0.18, 0.04, 0.01), turn, thread, SHADE_CLOTH);
    }

    /// A brazier at the wall's foot: a copper bowl on three legs, its coals and flames burning
    /// amber. It lights the vault (as the estate's lanterns did).
    fn brazier(&mut self, grid: &Grid, spot: P, d: Dir) {
        let (dx, dy) = d.vec();
        let floor = grid.elevation(spot.x + dx, spot.y + dy) as f32 * STEP;
        let pos = Vec3::new(spot.x as f32 + dx as f32 * 0.72, floor, spot.y as f32 + dy as f32 * 0.72);
        let start = self.static_cubes.len();
        let copper = lin(COPPER);
        for k in 0..3 {
            let a = k as f32 * TAU / 3.0;
            block(
                &mut self.static_cubes,
                pos + Vec3::new(a.cos() * 0.13, 0.3, a.sin() * 0.13),
                Vec3::new(0.05, 0.6, 0.05),
                a,
                copper * 0.8,
                SHADE_BRONZE,
            );
        }
        block(&mut self.static_cubes, pos + Vec3::Y * 0.64, Vec3::new(0.42, 0.1, 0.42), PI / 4.0, copper, SHADE_BRONZE);
        block(&mut self.static_cubes, pos + Vec3::Y * 0.72, Vec3::new(0.5, 0.06, 0.5), 0.0, copper * 0.9, SHADE_BRONZE);
        let ember = lin(EMBER);
        glowing_block(&mut self.static_cubes, pos + Vec3::Y * 0.77, Vec3::new(0.36, 0.06, 0.36), lin(0x3a1a0a), ember * 1.2);
        for (k, (ox, oz, s)) in [(0.0f32, 0.0f32, 0.16f32), (0.08, 0.05, 0.1), (-0.07, -0.04, 0.11)].into_iter().enumerate() {
            let h = 0.86 + k as f32 * 0.04;
            glowing_block(
                &mut self.static_cubes,
                pos + Vec3::new(ox, h, oz),
                Vec3::splat(s),
                lin(AMBER),
                lin(AMBER) * (2.5 - k as f32 * 0.5),
            );
        }
        self.torches.insert(spot, (pos + Vec3::Y * 0.4, (start, self.static_cubes.len() - start)));
    }

    /// Moss creeping over the top of a wall.
    fn wall_growth(&mut self, x: f32, y: f32, top: f32, hx: f64, hy: f64) {
        let hk = hash(hx * 1.7 + 9.0, hy * 2.9 + 4.0);
        if hk < 0.45 {
            // A creeping patch in small clumps, thickest in its middle.
            let (px, pz) = ((hash(hx * 2.0, hy) as f32 - 0.5) * 0.5, (hash(hy * 2.0, hx) as f32 - 0.5) * 0.5);
            for k in 0..3 + (hk * 14.0) as usize {
                let a = hash(hx + k as f64 * 3.1, hy * 1.3 + k as f64) as f32;
                let b = hash(hy + k as f64 * 2.3, hx * 1.1 + k as f64) as f32;
                let (a, b) = (0.5 + px + (a - 0.5) * 0.45, 0.5 + pz + (b - 0.5) * 0.45);
                let size = Vec3::new(0.08 + (a * 7.0).fract() * 0.12, 0.025 + (b * 5.0).fract() * 0.035, 0.08 + (b * 9.0).fract() * 0.12);
                let at = Vec3::new(x + (a - 0.5) * 0.6, top + 0.08 + size.y / 2.0, y + (b - 0.5) * 0.6);
                block(&mut self.static_cubes, at, size, a * 3.0, jit(MOSS[k % 3], b as f64, 0.1), SHADE_SOFT);
            }
        }
    }

    /// Moss along the seams of a flagstone.
    fn floor_growth(&mut self, x: f32, y: f32, h: f32, hx: f64, hy: f64) {
        let hm = hash(hx * 4.1 + 2.0, hy * 3.7 + 6.0);
        if hm < 0.16 {
            // Moss in a seam, along x or along z through the tile's middle.
            let along_x = hm < 0.08;
            let len = 0.3 + hm as f32 * 3.0;
            let size = if along_x { Vec3::new(len, 0.025, 0.05) } else { Vec3::new(0.05, 0.025, len) };
            let off = (hash(hy, hx * 2.0) as f32 - 0.5) * 0.4;
            let at = Vec3::new(x + if along_x { off } else { 0.0 }, h + 0.005, y + if along_x { 0.0 } else { off });
            block(&mut self.static_cubes, at, size, 0.0, jit(MOSS[(hm * 30.0) as usize % 3], hm, 0.1), SHADE_SOFT);
        }
    }

    /// An arch across a narrow passage at (`at`): piers against the walls either side rising
    /// past them, a round arch of wedge stones with a keystone, a lintel course over it, and a
    /// lantern hung on a chain from the crown. High enough to walk under; it fades out of the
    /// way of the view of him, as trees do. `across_x`: the walls stand east and west.
    fn arch(&mut self, at: P, floor: f32, across_x: bool, width: i32) {
        // `at` is the first tile of the run across; the arch is centred on the run.
        let half = (width - 1) as f32 / 2.0;
        let (cx, cz) = if across_x { (at.x as f32 + half, at.y as f32) } else { (at.x as f32, at.y as f32 + half) };
        let (hx, hy) = (at.x as f64, at.y as f64);
        // Local frame: u across the passage (between the walls), w along it.
        let frame =
            |u: f32, h: f32, w: f32| if across_x { Vec3::new(cx + u, floor + h, cz + w) } else { Vec3::new(cx + w, floor + h, cz + u) };
        // (A quarter turn back, so the local across axis lands on +z as `frame` puts it.)
        let turn = if across_x { 0.0 } else { -PI / 2.0 };
        let start = self.static_cubes.len();
        let spring = 1.75;
        let r = width as f32 / 2.0;
        let depth = 0.38 + 0.08 * (width - 1) as f32;
        // The piers: pale stone, from the floor to the spring, standing proud of the wall.
        for side in [-1.0f32, 1.0] {
            let c = jit(PALE_STONE[(hash(hx + side as f64, hy) * 2.0) as usize % 2], hash(hy, hx + side as f64), 0.06) * 0.8;
            block(&mut self.static_cubes, frame(side * r, spring / 2.0, 0.0), Vec3::new(0.22, spring, depth + 0.04), turn, c, SHADE_STONE);
            block(
                &mut self.static_cubes,
                frame(side * (r - 0.02), spring + 0.05, 0.0),
                Vec3::new(0.3, 0.1, depth + 0.1),
                turn,
                c * 1.05,
                SHADE_STONE,
            );
        }
        // The arch: wedge stones round the half circle, the middle one the keystone.
        let wedges = 7 + 2 * width as usize;
        for k in 0..wedges {
            let a = PI * (k as f32 + 0.5) / wedges as f32;
            let (u, h) = (-a.cos() * r, spring + 0.1 + a.sin() * r);
            let key = k == wedges / 2;
            let c = if key { jit(PALE_STONE[0], hash(hx, hy), 0.04) } else { jit(CHARCOAL[k % 3], hash(hx + k as f64, hy), 0.1) };
            let size = Vec3::new(PI * r / wedges as f32 + 0.02, if key { 0.32 } else { 0.24 }, depth + if key { 0.06 } else { 0.0 });
            let rot = Quat::from_rotation_y(turn) * Quat::from_rotation_z(a - PI / 2.0);
            let m = Mat4::from_scale_rotation_translation(size, rot, frame(u, h, 0.0));
            block_m(&mut self.static_cubes, m, c, SHADE_STONE);
        }
        // The lintel course over it, and the haunches filled up to it.
        block(
            &mut self.static_cubes,
            frame(0.0, spring + 0.1 + r + 0.28, 0.0),
            Vec3::new(2.0 * r + 0.24, 0.2, depth + 0.02),
            turn,
            jit(UMBER[0], hash(hy, hx), 0.08),
            SHADE_STONE,
        );
        for side in [-1.0f32, 1.0] {
            let fill = r * 0.9;
            block(
                &mut self.static_cubes,
                frame(side * (r - 0.12), spring + 0.1 + fill / 2.0 + 0.1, 0.0),
                Vec3::new(0.26, fill + 0.2, depth),
                turn,
                jit(CHARCOAL[2], hash(hx, hy + side as f64), 0.1),
                SHADE_STONE,
            );
        }
        // A little moss on the lintel, now and then.
        if hash(hx * 3.0, hy * 5.0) < 0.5 {
            block(
                &mut self.static_cubes,
                frame(0.15, spring + 0.1 + r + 0.4, 0.0),
                Vec3::new(0.5, 0.04, depth * 0.8),
                turn,
                lin(MOSS[1]),
                SHADE_SOFT,
            );
        }
        let len = self.static_cubes.len() - start;
        let shades = self.static_cubes[start..start + len].iter().map(|c| c.emissive[3]).collect();
        self.trees.push(Tree {
            range: (start, len),
            base: frame(0.0, spring - 0.2, 0.0) - Vec3::Y * 0.0,
            radius: r + 0.25,
            height: r + 0.7,
            fade: 1.0,
            alive: true,
            shades: Some(shades),
        });
        // The lantern under the crown.
        self.hanging_lantern(at, frame(0.0, spring + 0.1 + r - 0.12, 0.0), 0.55);
    }

    /// A lantern of copper and amber glass hung on a chain of `drop` from `from`; it lights the
    /// vault (one of the torches, keyed by the tile `at`).
    fn hanging_lantern(&mut self, at: P, from: Vec3, drop: f32) {
        let start = self.static_cubes.len();
        let links = (drop / 0.07) as usize;
        for k in 0..links {
            let turn = if k % 2 == 0 { 0.0 } else { PI / 2.0 };
            block(
                &mut self.static_cubes,
                from - Vec3::Y * (k as f32 * 0.07 + 0.035),
                Vec3::new(0.035, 0.06, 0.012),
                turn,
                lin(IRON),
                SHADE_IRON,
            );
        }
        let c = from - Vec3::Y * (drop + 0.13);
        let copper = lin(COPPER);
        block(&mut self.static_cubes, c + Vec3::Y * 0.13, Vec3::new(0.2, 0.04, 0.2), PI / 4.0, copper, SHADE_BRONZE);
        block(&mut self.static_cubes, c - Vec3::Y * 0.12, Vec3::new(0.18, 0.04, 0.18), PI / 4.0, copper * 0.9, SHADE_BRONZE);
        for k in 0..4 {
            let a = k as f32 * TAU / 4.0 + PI / 4.0;
            block(
                &mut self.static_cubes,
                c + Vec3::new(a.cos() * 0.09, 0.0, a.sin() * 0.09),
                Vec3::new(0.025, 0.24, 0.025),
                0.0,
                copper * 0.8,
                SHADE_BRONZE,
            );
        }
        glowing_block(&mut self.static_cubes, c, Vec3::new(0.13, 0.2, 0.13), lin(AMBER), lin(AMBER) * 1.8);
        // (The light sits at the glass: torches are lit half a unit above their point.)
        self.torches.insert(at, (c - Vec3::Y * 0.5, (start, self.static_cubes.len() - start)));
    }

    /// A lantern on an iron bracket from the wall's face over the road `d`.
    fn wall_lantern(&mut self, wall: P, base: f32, d: Dir) {
        let (dx, dy) = d.vec();
        let face = Vec3::new(wall.x as f32 + dx as f32 * 0.5, base + 1.45, wall.y as f32 + dy as f32 * 0.5);
        let out = Vec3::new(dx as f32, 0.0, dy as f32);
        let iron = lin(IRON);
        block(&mut self.static_cubes, face + out * 0.02, Vec3::new(0.12, 0.2, 0.12), 0.0, iron, SHADE_IRON);
        block(
            &mut self.static_cubes,
            face + out * 0.17 + Vec3::Y * 0.06,
            Vec3::new(0.3, 0.035, 0.3) * (out.abs() + Vec3::new(0.0, 1.0, 0.0)) + Vec3::new(0.03, 0.0, 0.03),
            0.0,
            iron,
            SHADE_IRON,
        );
        self.hanging_lantern(wall, face + out * 0.3 + Vec3::Y * 0.04, 0.1);
    }

    /// A hooded knight of stone on a plinth on top of the wall, looking down on the road `d`: a
    /// long robe, a mantle over the shoulders, the hood's shadow for a face, both hands on the
    /// pommel of a sword stood point down before him. A monument over the maze; it fades out of
    /// the way of the view of him.
    fn statue(&mut self, x: f32, y: f32, base: f32, d: Dir, hx: f64, hy: f64) {
        let (dx, dy) = d.vec();
        let yaw = -(dy as f32).atan2(dx as f32) + PI / 2.0;
        let q = Quat::from_rotation_y(yaw);
        let origin = Vec3::new(x + dx as f32 * 0.08, base, y + dy as f32 * 0.08);
        // Local: x across him, y up, z toward the road.
        let at = |l: Vec3| origin + q * l;
        let start = self.static_cubes.len();
        let stone = jit(CHARCOAL[1], hash(hx, hy), 0.06) * 0.85;
        let dark = stone * 0.7;
        let pale = jit(PALE_STONE[0], hash(hy, hx), 0.05);
        let mut put = |c: Vec3, s: Vec3, col: Vec3| {
            let m = Mat4::from_scale_rotation_translation(s, q, at(c));
            self.static_cubes.push(Instance::new(m, col, 1.0, Vec3::ZERO, SHADE_STONE));
        };
        put(Vec3::new(0.0, 0.14, 0.0), Vec3::new(0.82, 0.28, 0.64), pale * 0.8);
        put(Vec3::new(0.0, 0.31, 0.0), Vec3::new(0.7, 0.06, 0.54), pale * 0.7);
        // The robe in three falls, widest at the hem, and the cape hanging behind.
        put(Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.62, 0.32, 0.44), stone * 0.9);
        put(Vec3::new(0.0, 0.8, 0.0), Vec3::new(0.54, 0.3, 0.38), stone);
        put(Vec3::new(0.0, 1.08, 0.0), Vec3::new(0.48, 0.28, 0.34), stone * 1.05);
        put(Vec3::new(0.0, 0.9, -0.2), Vec3::new(0.66, 1.0, 0.08), dark);
        // Shoulders under the mantle, and the arms coming down to the hands at the pommel.
        put(Vec3::new(0.0, 1.32, -0.02), Vec3::new(0.72, 0.18, 0.42), stone * 0.95);
        for side in [-1.0f32, 1.0] {
            let arm = Mat4::from_scale_rotation_translation(
                Vec3::new(0.13, 0.42, 0.14),
                q * Quat::from_rotation_x(-0.5) * Quat::from_rotation_z(side * 0.35),
                at(Vec3::new(side * 0.2, 1.12, 0.12)),
            );
            self.static_cubes.push(Instance::new(arm, stone * 0.92, 1.0, Vec3::ZERO, SHADE_STONE));
        }
        let mut put = |c: Vec3, s: Vec3, col: Vec3| {
            let m = Mat4::from_scale_rotation_translation(s, q, at(c));
            self.static_cubes.push(Instance::new(m, col, 1.0, Vec3::ZERO, SHADE_STONE));
        };
        // The hood, bowed a little, the shadow of the face under it.
        put(Vec3::new(0.0, 1.55, 0.0), Vec3::new(0.34, 0.32, 0.36), stone);
        put(Vec3::new(0.0, 1.73, -0.03), Vec3::new(0.24, 0.1, 0.28), stone * 0.95);
        put(Vec3::new(0.0, 1.5, 0.16), Vec3::new(0.2, 0.2, 0.06), dark * 0.4);
        // The sword, point down before him on the plinth, his hands on its pommel.
        put(Vec3::new(0.0, 0.7, 0.3), Vec3::new(0.07, 0.8, 0.04), pale * 0.75);
        put(Vec3::new(0.0, 1.1, 0.3), Vec3::new(0.32, 0.05, 0.06), pale * 0.7);
        put(Vec3::new(0.0, 1.2, 0.29), Vec3::new(0.2, 0.12, 0.13), stone * 1.1);
        // Verdigris and moss where the damp has had its way with him.
        if hash(hx * 2.0, hy * 3.0) < 0.6 {
            put(Vec3::new(0.14, 1.49, 0.0), Vec3::new(0.18, 0.04, 0.3), lin(VERDIGRIS));
        }
        put(Vec3::new(-0.2, 0.37, 0.12), Vec3::new(0.24, 0.03, 0.18), lin(MOSS[0]));
        let len = self.static_cubes.len() - start;
        let shades = self.static_cubes[start..start + len].iter().map(|c| c.emissive[3]).collect();
        self.trees.push(Tree {
            range: (start, len),
            base: origin,
            radius: 0.45,
            height: 1.8,
            fade: 1.0,
            alive: true,
            shades: Some(shades),
        });
    }

    /// A little heap of bones: a skull and a few long bones, pale on the stone.
    fn bone(&mut self, at: Vec3, h: f32) {
        let bone = lin(BONE);
        block(&mut self.static_cubes, at + Vec3::Y * 0.06, Vec3::new(0.12, 0.11, 0.14), h * 6.0, bone, SHADE_STONE);
        block(&mut self.static_cubes, at + Vec3::new(0.0, 0.04, 0.07), Vec3::new(0.08, 0.04, 0.03), h * 6.0, bone * 0.8, SHADE_STONE);
        for k in 0..2 {
            let a = h * 4.0 + k as f32 * 1.9;
            block(
                &mut self.static_cubes,
                at + Vec3::new(a.cos() * 0.14, 0.015, a.sin() * 0.14),
                Vec3::new(0.24, 0.03, 0.035),
                a,
                bone * 0.92,
                SHADE_STONE,
            );
        }
    }
}
