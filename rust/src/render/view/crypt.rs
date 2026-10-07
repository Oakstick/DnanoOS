//! The Bone Vaults (dungeon III, and the estates deeper still): the first level built to the art
//! direction (the *Voxel Dungeon* board, stage B). The estate's maze is cut from stone instead of
//! hedges: walls of dressed blocks in charcoal and umber on pale-stone plinths,
//! verdigris creeping over some blocks, monumental pillars, banners of copper and umber cloth,
//! wet flagstones underfoot, and amber braziers burning against a cold mist-teal dark.
//!
//! The Cisterns (dungeon II) are the same stone, flooded: standing water over the low floors,
//! copper spouts pouring into it from the walls, a dark wet band along the walls' foot, more
//! columns and vaults, fewer banners and bones, and a cooler light.
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
pub(super) const SHADE_CLOTH: f32 = 8.0;
/// Soft, strand-shaded growth (`voxel_surface` fur): moss and leaves.
pub(super) const SHADE_SOFT: f32 = 7.0;
pub(super) const SHADE_IRON: f32 = 6.0;
pub(super) const SHADE_BRONZE: f32 = 13.0;
pub(super) const SHADE_WOOD: f32 = 12.0;

/// How high the vaults' walls stand above their floor (just above his chest, so the top view
/// sees him past them), and their courses of blocks.
const WALL_H: f32 = 1.15;
const COURSES: usize = 3;

pub(super) fn jit(hex: u32, h: f64, amount: f32) -> Vec3 {
    lin(hex) * (1.0 - amount + h as f32 * amount * 2.0)
}

pub(super) fn block(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot_y: f32, color: Vec3, shade: f32) {
    let m = Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(rot_y), pos);
    out.push(Instance::new(m, color, 1.0, Vec3::ZERO, shade));
}

pub(super) fn block_m(out: &mut Vec<Instance>, m: Mat4, color: Vec3, shade: f32) {
    out.push(Instance::new(m, color, 1.0, Vec3::ZERO, shade));
}

/// One face of a wall in dressed stone (`masonry`): the wall's middle line at `centre` (its
/// core reaching `half_depth` either side), the face looking along `dir`, running `half` either
/// way along it, standing `height` over `base` in `courses` on a pale plinth. A vault's wall tile
/// is one unit long and three courses high; the story's gateway runs its walls whole.
#[derive(Clone)]
pub(super) struct Wall<'a> {
    pub centre: Vec3,
    pub dir: (i32, i32),
    pub half: f32,
    pub base: f32,
    pub height: f32,
    pub courses: usize,
    /// The hashes' seed (a vault's tile).
    pub seed: (f64, f64),
    /// Blocks' length (shortest, and how much longer they may run) and how far they stand out.
    pub len: (f32, f32),
    pub depth: (f32, f32),
    /// Where the blocks' backs stand from the middle line.
    pub face: f32,
    /// The lowest course dark and slick where water stood, and more verdigris (the cisterns).
    pub wet: bool,
    /// Half the top course has fallen.
    pub ruined: bool,
    /// The floor before the face, where fallen blocks lie (none if `None`).
    pub foot: Option<f32>,
    /// Openings left in it: (middle along the face, half width, its sill, the top of its sides,
    /// the rise of a round head over that), heights over `base`: stones are cut back to their edges.
    pub openings: &'a [(f32, f32, f32, f32, f32)],
}

impl Wall<'_> {
    /// A vault's wall tile `(x, y)` on the floor `base`, its face toward `d`.
    fn tile(x: f32, y: f32, base: f32, d: Dir, seed: (f64, f64), wet: bool, foot: Option<f32>, ruined: bool) -> Wall<'static> {
        Wall {
            centre: Vec3::new(x, 0.0, y),
            dir: d.vec(),
            half: 0.5,
            base,
            height: WALL_H,
            courses: COURSES,
            seed,
            len: (0.3, 0.22),
            depth: (0.08, 0.08),
            face: 0.46,
            wet,
            ruined,
            foot,
            openings: &[],
        }
    }
}

/// The face of a wall in courses of dressed blocks of uneven length, staggered, on a pale
/// plinth; here and there a block of umber, or green with verdigris. The blocks are chunky and
/// old: each stands out its own depth, a touch askew, some chipped short. Now and then one has
/// fallen out and lies broken at the wall's foot, and a `ruined` wall has lost half its top course.
pub(super) fn masonry(out: &mut Vec<Instance>, w: &Wall) {
    let (dx, dy) = w.dir;
    let (hx, hy) = w.seed;
    let (x, y, base) = (w.centre.x, w.centre.z, w.base);
    let turn = if dx != 0 { PI / 2.0 } else { 0.0 };
    let at = |u: f32, h: f32, o: f32| {
        Vec3::new(x + dx as f32 * o + if dx != 0 { 0.0 } else { u }, h, y + dy as f32 * o + if dx != 0 { u } else { 0.0 })
    };
    // How far an opening at `u` leaves the stone open at height `h` (0 where it is closed).
    let open_at = |o: &(f32, f32, f32, f32, f32), h: f32| -> f32 {
        let (_, half, sill, side, rise) = *o;
        if h < sill {
            0.0
        } else if h < side {
            half
        } else if rise > 0.0 && h < side + rise {
            half * (1.0 - ((h - side) / rise).powi(2)).max(0.0).sqrt()
        } else {
            0.0
        }
    };
    // Cuts a block's run a..b at height range h0..h1 back to the openings' edges.
    let cut = |mut a: f32, mut b: f32, h0: f32, h1: f32| -> Option<(f32, f32)> {
        for o in w.openings {
            let open = open_at(o, h0 + 0.01).max(open_at(o, h1 - 0.01)).max(open_at(o, (h0 + h1) / 2.0));
            if open > 0.0 && b > o.0 - open && a < o.0 + open {
                if (a + b) / 2.0 < o.0 {
                    b = o.0 - open;
                } else {
                    a = o.0 + open;
                }
            }
        }
        (b - a > 0.06).then_some((a, b))
    };
    // The plinth (a vault's tile is one piece; a long wall is laid in slabs).
    let hk = hash(hx * 3.0 + dx as f64, hy * 5.0 + dy as f64);
    if w.half <= 0.5 {
        block(out, at(0.0, base + 0.1, w.face + 0.01), Vec3::new(1.0, 0.2, 0.1), turn, jit(PALE_STONE[1], hk, 0.05) * 0.8, SHADE_STONE);
    } else {
        let mut u = -w.half;
        let mut k = 0;
        while u < w.half {
            let hk = hash(hx * 3.0 + k as f64 * 1.9, hy * 5.0 + dy as f64 + k as f64);
            let len = 0.8 + hk as f32 * 0.6;
            if let Some((a, b)) = cut(u.max(-w.half), (u + len).min(w.half), 0.0, 0.2) {
                block(
                    out,
                    at((a + b) / 2.0, base + 0.11, w.face + 0.04),
                    Vec3::new(b - a - 0.02, 0.22, 0.16),
                    turn,
                    jit(PALE_STONE[k % 2], hk, 0.05) * 0.8,
                    SHADE_STONE,
                );
            }
            u += len;
            k += 1;
        }
    }
    let course_h = (w.height - 0.2) / w.courses as f32;
    let mut fallen = 0;
    for course in 0..w.courses {
        let shift = if course % 2 == 0 { 0.0 } else { 0.17 };
        let mut u: f32 = -w.half - shift;
        let mut k = 0;
        while u < w.half {
            let hk = hash(hx * 7.0 + k as f64 * 1.3 + course as f64 * 5.0 + dx as f64, hy * 3.0 + dy as f64 * 7.0 + course as f64);
            let len = w.len.0 + hk as f32 * w.len.1;
            let (a, b) = (u.max(-w.half), (u + len).min(w.half));
            let gone = hash(hk * 17.0 + 3.0, hy * 1.3 + hx);
            let missing = if w.ruined && course == w.courses - 1 { gone < 0.5 } else { gone < 0.035 };
            let y0 = base + 0.2 + course as f32 * course_h;
            let run = cut(a, b, y0 - base, y0 - base + course_h);
            if missing {
                if w.openings.is_empty() || run.is_some() {
                    fallen += 1;
                }
            } else if let Some((a, b)) = run {
                let roll = hash(hk * 31.0, hx + hy);
                let c = if roll < if w.wet { 0.1 } else { 0.04 } {
                    jit(VERDIGRIS, hk, 0.1)
                } else if roll < 0.19 {
                    jit(UMBER[(hk * 2.0) as usize % 2], hk, 0.1)
                } else {
                    jit(CHARCOAL[(hk * 3.0) as usize % 3], hk, 0.12)
                };
                // In the cisterns the lowest course is dark and slick where the water stood.
                let c = if w.wet && course == 0 { c * 0.55 + lin(0x1c2a24) * 0.35 } else { c };
                // Each block its own depth out of the core, a touch askew, some chipped short.
                let depth = w.depth.0 + hash(hk * 13.0, hy + 1.0) as f32 * w.depth.1;
                let chip = if (hk * 29.0).fract() < 0.25 { (hk * 53.0).fract() as f32 * 0.05 } else { 0.0 };
                let askew = ((hk * 41.0).fract() as f32 - 0.5) * 0.06;
                let h = y0 + course_h / 2.0 - chip / 2.0;
                block(
                    out,
                    at((a + b) / 2.0, h, w.face + depth / 2.0),
                    Vec3::new(b - a - 0.03, course_h - 0.03 - chip, depth),
                    turn + askew,
                    c,
                    SHADE_STONE,
                );
            }
            u += len;
            k += 1;
        }
    }
    // The fallen blocks, broken in pieces at the foot of the wall.
    let Some(foot) = w.foot else { return };
    let pieces = (fallen * 2).min(7 * (w.half * 2.0).ceil() as usize);
    let scale = (w.len.0 / 0.3).sqrt();
    for k in 0..pieces {
        let r1 = hash(hx * 3.7 + k as f64 * 1.9 + dx as f64, hy * 2.3 + dy as f64 * 5.0 + k as f64) as f32;
        let r2 = hash(hy * 4.1 + k as f64 * 2.7, hx * 1.7 + dy as f64 + k as f64 * 3.0) as f32;
        let s = (0.07 + r1 * 0.13) * scale;
        let size = Vec3::new(s * (1.0 + r2 * 0.6), s * (0.6 + r2 * 0.5), s * (0.8 + r1 * 0.4));
        let rot = Quat::from_rotation_y(r1 * 6.0) * Quat::from_rotation_x((r2 - 0.5) * 0.6) * Quat::from_rotation_z((r1 - 0.5) * 0.5);
        let u = (r2 - 0.5) * 0.85 * w.half * 2.0;
        if w.openings.iter().any(|o| (u - o.0).abs() < o.1 + 0.1) {
            continue;
        }
        let pos = at(u, foot + size.y * 0.4, w.face + 0.1 + r1 * 0.25);
        let c = jit(CHARCOAL[k % 3], r2 as f64, 0.12) * 0.9;
        block_m(out, Mat4::from_scale_rotation_translation(size, rot, pos), c, SHADE_STONE);
    }
}

pub(super) fn glowing_block(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, color: Vec3, glow: Vec3) {
    let m = Mat4::from_scale_rotation_translation(scale, Quat::IDENTITY, pos);
    out.push(Instance::new(m, color, 1.0, glow, SHADE_LIT));
}

/// The vaults' light: no sky, a cold teal dark lit by its braziers, and a faint teal light
/// falling from shafts far above (it rims the tops of things). The cisterns' is cooler, more of
/// it from above off the water.
pub fn crypt_atmosphere(cisterns: bool) -> Atmosphere {
    Atmosphere {
        background: DARK,
        fog_near: 7.0,
        fog_far: 26.0,
        hemi_sky: lin(MIST_TEAL) * if cisterns { 0.5 } else { 0.35 },
        hemi_ground: lin(0x3a2e27) * 0.12,
        ambient: lin(0x2a3634) * 0.9,
        sun_dir: Vec3::new(0.35, 1.0, 0.25).normalize(),
        sun: lin(0x9ec4bc) * if cisterns { 0.42 } else { 0.3 },
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
        let cis = self.cisterns;
        // In the cisterns the water stands in pools over the low floors, in blocks of rooms.
        let flooded = |x: i32, y: i32| {
            let cell = |s: i32, k: f64| hash(x.div_euclid(s) as f64 * 1.7 + k, y.div_euclid(s) as f64 * 2.3 + k);
            cis && grid.elevation(x, y) == 0 && (cell(3, 11.0) < 0.4 || cell(2, 23.0) < 0.12)
        };

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
            flagstones(&mut self.static_cubes, Vec3::new(xf, h, yf), (hx, hy));
            // Moss where the damp gathers in the seams.
            self.floor_growth(xf, yf, h, hx, hy);
            if flooded(x, y) {
                self.flood.push(Vec3::new(xf, h + 0.03, yf));
            }
            // Now and then the bones of the vaults' dead, swept to the foot of a wall.
            if hash(hx * 5.0 + 3.0, hy * 7.0) < if cis { 0.012 } else { 0.05 }
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
            // Now and then a wall has crumbled: most of its top course lies on the floor.
            let ruined = hash(hx * 6.1 + 2.0, hy * 4.3 + 8.0) < 0.18;
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
                // What falls from the wall lies on the road's floor (not down a pit or on a stair).
                let (nx, ny) = (x + d.vec().0, y + d.vec().1);
                let plain = !matches!(grid.get(nx, ny), Tile::Pit | Tile::Nest) && !grid.is_ramp(nx, ny) && !grid.is_bridge(nx, ny);
                masonry(
                    &mut self.static_cubes,
                    &Wall::tile(xf, yf, base, d, (hx, hy), self.cisterns, plain.then(|| floor_y(nx, ny)), ruined),
                );
            }
            // A coping of heavy dark slabs across the top, cracked through; on a ruined wall, what
            // is left of it, settled askew over the rubble core.
            let split_x = hash(hx + 5.0, hy * 2.0) < 0.5;
            let slabs = if ruined { 3 } else { 2 };
            for k in 0..slabs {
                let r = hash(hx * 2.1 + k as f64 * 1.7, hy * 3.3 + k as f64) as f32;
                if ruined && r < 0.35 {
                    continue;
                }
                let w = 1.0 / slabs as f32;
                let off = -0.5 + w * (k as f32 + 0.5);
                let (size, o) = if split_x {
                    (Vec3::new(w - 0.025, 0.11, 1.0), Vec3::new(off, 0.0, 0.0))
                } else {
                    (Vec3::new(1.0, 0.11, w - 0.025), Vec3::new(0.0, 0.0, off))
                };
                let tilt = (r - 0.5) * if ruined { 0.16 } else { 0.03 };
                let drop = if ruined { r * 0.05 } else { (r - 0.5) * 0.015 };
                let m = Mat4::from_scale_rotation_translation(
                    size,
                    Quat::from_rotation_x(tilt * 0.5) * Quat::from_rotation_z(tilt),
                    Vec3::new(xf, top + 0.055 - drop, yf) + o,
                );
                block_m(
                    &mut self.static_cubes,
                    m,
                    jit(CHARCOAL[(hash(hx + 3.0 + k as f64, hy) * 3.0) as usize % 3], r as f64, 0.08) * 0.85,
                    SHADE_STONE,
                );
            }
            if !ruined {
                self.wall_growth(xf, yf, top, hx, hy);
            }
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
            if cis {
                // The cisterns: columns and spouts, lanterns, few braziers, no banners.
                let road = roads[(pick * 50.0) as usize % roads.len()];
                let (dx, dy) = road.vec();
                let (nx, ny) = (x + dx, y + dy);
                let plain = grid.get(nx, ny) != Tile::Pit && !grid.is_ramp(nx, ny) && !grid.is_bridge(nx, ny);
                if hash(hx * 3.0, hy * 7.0) < 0.06 {
                    braziers.push((P::new(x, y), roads[0]));
                } else if pick < 0.16 {
                    self.pillar(xf, yf, base, roads[0], hx, hy);
                } else if pick < 0.24 && plain && !ruined {
                    self.spout(xf, yf, base, road, floor_y(nx, ny), hx, hy);
                } else if pick < 0.26 && !ruined {
                    self.statue(xf, yf, top + 0.11, roads[0], hx, hy);
                } else if pick < 0.33 {
                    self.wall_lantern(P::new(x, y), base, road);
                }
            } else if hash(hx * 3.0, hy * 7.0) < 0.13 {
                braziers.push((P::new(x, y), roads[0]));
            } else if pick < 0.07 {
                self.pillar(xf, yf, base, roads[0], hx, hy);
            } else if pick < 0.16 {
                self.banner(xf, yf, base, roads[(pick * 50.0) as usize % roads.len()], hx, hy);
            } else if pick < 0.2 && !ruined {
                self.statue(xf, yf, top + 0.11, roads[0], hx, hy);
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
                if !passage || hash(hx * 2.3 + 7.0, hy * 1.9 + 3.0) > if cis { 0.14 } else { 0.07 } {
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

    /// The vaults' gatehouse in the north wall: two squat towers of dressed stone, each with a
    /// hooded knight on top keeping watch; between them a round arch, its keystone a carved skull,
    /// closed by iron gates under a fixed grille (the gates and lock lamps move, so the gate's
    /// own code draws them each frame); a stepped pale pediment over all; copper braziers before
    /// the towers. The lock lamps sit in iron sockets in a fan over the arch.
    pub(super) fn build_crypt_gate(&mut self, grid: &Grid, gate: P, sockets: &[Vec3]) {
        let floor = grid.elevation(gate.x, gate.y + 1) as f32 * STEP;
        let o = Vec3::new(gate.x as f32, floor, gate.y as f32);
        let (gx, gy) = (gate.x as f64, gate.y as f64);
        let out = &mut self.static_cubes;
        // Threshold, and the dark way down behind the gates.
        block(out, o + Vec3::new(0.0, -0.04, 0.05), Vec3::new(1.1, 0.1, 0.95), 0.0, jit(PALE_STONE[1], 0.3, 0.0) * 0.7, SHADE_STONE);
        block(out, o + Vec3::new(0.0, 0.85, -0.42), Vec3::new(0.9, 1.8, 0.08), 0.0, lin(0x060606), SHADE_LIT);
        // The towers.
        let course_h = 0.4;
        let courses = 6;
        let tower_top = -0.2 + courses as f32 * course_h;
        for side in [-1.0f32, 1.0] {
            let out = &mut self.static_cubes;
            let t = o + Vec3::new(side * 0.98, 0.0, 0.0);
            block(out, t + Vec3::new(0.0, 0.05, 0.0), Vec3::new(0.98, 0.3, 0.98), 0.0, jit(PALE_STONE[1], gx, 0.04) * 0.8, SHADE_STONE);
            for k in 0..courses {
                let h = hash(gx + side as f64 * 3.0, k as f64 * 1.7);
                let c = if k == 3 {
                    jit(PALE_STONE[0], h, 0.05) * 0.75
                } else if h < 0.3 {
                    jit(UMBER[k % 2], h, 0.08)
                } else {
                    jit(CHARCOAL[k % 3], h, 0.1)
                };
                let w = 0.88 + (h as f32 - 0.5) * 0.04;
                let y0 = -0.2 + k as f32 * course_h;
                block(
                    out,
                    t + Vec3::new(0.0, y0 + course_h / 2.0, 0.0),
                    Vec3::new(w, course_h - 0.03, w),
                    (h as f32 - 0.5) * 0.04,
                    c,
                    SHADE_STONE,
                );
            }
            // A heavy cap, and verdigris down the stone where the water runs.
            block(
                out,
                t + Vec3::new(0.0, tower_top + 0.07, 0.0),
                Vec3::new(0.98, 0.14, 0.98),
                0.0,
                jit(CHARCOAL[0], gx, 0.05) * 0.8,
                SHADE_STONE,
            );
            block(
                out,
                t + Vec3::new(side * 0.2, tower_top - 0.5, 0.45),
                Vec3::new(0.12, 0.9, 0.02),
                0.0,
                jit(VERDIGRIS, gy, 0.1),
                SHADE_STONE,
            );
            block(out, t + Vec3::new(-side * 0.15, tower_top + 0.15, 0.25), Vec3::new(0.3, 0.04, 0.3), 0.4, lin(MOSS[1]), SHADE_SOFT);
            // A copper brazier before each tower (its flames are the gate's).
            let b = o + Vec3::new(side * 0.98, 0.0, 0.62);
            let copper = lin(COPPER);
            for k in 0..3 {
                let a = k as f32 * TAU / 3.0;
                block(out, b + Vec3::new(a.cos() * 0.1, 0.26, a.sin() * 0.1), Vec3::new(0.045, 0.52, 0.045), a, copper * 0.8, SHADE_BRONZE);
            }
            block(out, b + Vec3::Y * 0.55, Vec3::new(0.34, 0.09, 0.34), PI / 4.0, copper, SHADE_BRONZE);
            block(out, b + Vec3::Y * 0.61, Vec3::new(0.4, 0.05, 0.4), 0.0, copper * 0.9, SHADE_BRONZE);
            glowing_block(out, b + Vec3::Y * 0.645, Vec3::new(0.3, 0.04, 0.3), lin(0x3a1a0a), lin(EMBER) * 1.2);
            self.smoke.push((b + Vec3::new(0.0, 0.75, 0.0), 1.0));
            // The watcher on the tower.
            self.statue(o.x + side * 0.98, o.z, floor + tower_top + 0.14, Dir::Down, gx + side as f64 * 7.0, gy);
        }
        let out = &mut self.static_cubes;
        // The arch: piers, a round ring of wedge stones, the keystone a skull.
        let spring = 1.15;
        let r = 0.49;
        for side in [-1.0f32, 1.0] {
            block(
                out,
                o + Vec3::new(side * 0.53, spring / 2.0, 0.02),
                Vec3::new(0.2, spring, 0.4),
                0.0,
                jit(PALE_STONE[1], gy, 0.05) * 0.8,
                SHADE_STONE,
            );
        }
        let wedges = 9;
        for k in 0..wedges {
            let a = PI * (k as f32 + 0.5) / wedges as f32;
            let key = k == wedges / 2;
            let c = if key {
                lin(BONE) * 0.8
            } else if k % 2 == 0 {
                jit(PALE_STONE[0], hash(gx + k as f64, gy), 0.05) * 0.75
            } else {
                jit(CHARCOAL[k % 3], hash(gx, gy + k as f64), 0.1)
            };
            let size = Vec3::new(PI * r / wedges as f32 + 0.03, if key { 0.2 } else { 0.14 }, if key { 0.44 } else { 0.4 });
            let m = Mat4::from_scale_rotation_translation(
                size,
                Quat::from_rotation_z(a - PI / 2.0),
                o + Vec3::new(-a.cos() * r, spring + a.sin() * r, 0.02),
            );
            block_m(out, m, c, SHADE_STONE);
        }
        // The skull's eyes and nose, and its jaw over the crown.
        let skull = o + Vec3::new(0.0, spring + r, 0.24);
        for side in [-1.0f32, 1.0] {
            block(out, skull + Vec3::new(side * 0.035, 0.03, 0.0), Vec3::new(0.04, 0.04, 0.02), 0.0, lin(0x0e0c0a), SHADE_LIT);
        }
        block(out, skull + Vec3::new(0.0, -0.02, 0.0), Vec3::new(0.02, 0.025, 0.02), 0.0, lin(0x0e0c0a), SHADE_LIT);
        block(out, skull + Vec3::new(0.0, -0.08, -0.01), Vec3::new(0.08, 0.04, 0.04), 0.0, lin(BONE) * 0.7, SHADE_STONE);
        // A fixed iron grille in the arch over the gates: a rail at the spring, bars fanned up from it.
        block(out, o + Vec3::new(0.0, spring - 0.02, -0.02), Vec3::new(0.86, 0.04, 0.04), 0.0, lin(IRON), SHADE_IRON);
        for k in 0..5 {
            let a = PI * (k as f32 + 1.0) / 6.0;
            let len = r - 0.07;
            let m = Mat4::from_scale_rotation_translation(
                Vec3::new(0.025, len, 0.025),
                Quat::from_rotation_z(a - PI / 2.0),
                o + Vec3::new(-a.cos() * len / 2.0, spring + a.sin() * len / 2.0, -0.02),
            );
            block_m(out, m, lin(IRON), SHADE_IRON);
        }
        // The wall over the arch, in courses, up to a cornice and a stepped pale pediment.
        let wall_top = 2.25;
        for (x0, x1) in [(-0.63f32, -0.2f32), (0.2, 0.63)] {
            block(
                out,
                o + Vec3::new((x0 + x1) / 2.0, (spring + 1.62) / 2.0, -0.03),
                Vec3::new(x1 - x0, 1.62 - spring, 0.32),
                0.0,
                jit(CHARCOAL[1], gx, 0.05) * 0.85,
                SHADE_STONE,
            );
        }
        for k in 0..2 {
            let y0 = 1.62 + k as f32 * 0.315;
            for j in 0..3 {
                let h = hash(gx + j as f64 * 2.0, k as f64 + 4.0);
                let c = if h < 0.3 { jit(UMBER[j % 2], h, 0.08) } else { jit(CHARCOAL[(j + k) % 3], h, 0.1) };
                let shift = if k == 0 { 0.0 } else { 0.21 };
                let a = (-0.63 + j as f32 * 0.42 + shift - 0.21 * (k as f32)).max(-0.63);
                let b = (a + 0.42 + if k == 0 { 0.0 } else { 0.21 }).min(0.63);
                block(
                    out,
                    o + Vec3::new((a + b) / 2.0, y0 + 0.155, -0.02),
                    Vec3::new(b - a - 0.02, 0.29, 0.34 + h as f32 * 0.04),
                    0.0,
                    c,
                    SHADE_STONE,
                );
            }
        }
        for (k, w) in [1.36f32, 1.0, 0.6].into_iter().enumerate() {
            let c = jit(PALE_STONE[k % 2], gy + k as f64, 0.05) * (0.8 - k as f32 * 0.04);
            block(
                out,
                o + Vec3::new(0.0, wall_top + 0.06 + k as f32 * 0.13, 0.0),
                Vec3::new(w, 0.13, 0.42 - k as f32 * 0.04),
                0.0,
                c,
                SHADE_STONE,
            );
        }
        block(out, o + Vec3::new(0.3, wall_top + 0.14, 0.12), Vec3::new(0.4, 0.04, 0.2), 0.2, lin(MOSS[2]), SHADE_SOFT);
        // Iron sockets for the lock lamps.
        for &s in sockets {
            block(out, o + s - Vec3::new(0.0, 0.0, 0.03), Vec3::new(0.11, 0.11, 0.04), PI / 4.0, lin(IRON), SHADE_IRON);
        }
    }

    /// A monumental pillar against the wall: a square shaft of dark stone and umber drums on a
    /// moulded pale base, under a pale capital, standing well above the wall.
    fn pillar(&mut self, x: f32, y: f32, base: f32, d: Dir, hx: f64, hy: f64) {
        let (dx, dy) = d.vec();
        let p = Vec3::new(x + dx as f32 * 0.32, base, y + dy as f32 * 0.32);
        let start = self.static_cubes.len();
        let h = 2.4;
        pillar_cubes(&mut self.static_cubes, p, h, 6, 1.0, (hx, hy));
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

    /// A banner on the wall's face (see `banner_cubes`).
    fn banner(&mut self, x: f32, y: f32, base: f32, d: Dir, hx: f64, hy: f64) {
        let (dx, dy) = d.vec();
        banner_cubes(&mut self.static_cubes, Vec3::new(x, base, y), (dx, dy), 0.0, (hx, hy));
    }

    /// A brazier at the wall's foot (see `brazier_cubes`). It lights the vault (as the estate's
    /// lanterns did).
    fn brazier(&mut self, grid: &Grid, spot: P, d: Dir) {
        let (dx, dy) = d.vec();
        let floor = grid.elevation(spot.x + dx, spot.y + dy) as f32 * STEP;
        let pos = Vec3::new(spot.x as f32 + dx as f32 * 0.72, floor, spot.y as f32 + dy as f32 * 0.72);
        let start = self.static_cubes.len();
        brazier_cubes(&mut self.static_cubes, pos, 1.0, true);
        self.torches.insert(spot, (pos + Vec3::Y * 0.4, (start, self.static_cubes.len() - start)));
    }

    /// Moss creeping over the top of a wall.
    fn wall_growth(&mut self, x: f32, y: f32, top: f32, hx: f64, hy: f64) {
        moss_cap(&mut self.static_cubes, Vec3::new(x, top + 0.11, y), 0.6, (hx, hy));
    }

    /// Moss along the seams of a flagstone.
    fn floor_growth(&mut self, x: f32, y: f32, h: f32, hx: f64, hy: f64) {
        moss_seam(&mut self.static_cubes, Vec3::new(x, h, y), (hx, hy));
    }

    /// An arch across a narrow passage at (`at`): see `arch_cubes`. High enough to walk under;
    /// it fades out of the way of the view of him, as trees do. `across_x`: the walls stand east
    /// and west.
    fn arch(&mut self, at: P, floor: f32, across_x: bool, width: i32) {
        // `at` is the first tile of the run across; the arch is centred on the run.
        let half = (width - 1) as f32 / 2.0;
        let (cx, cz) = if across_x { (at.x as f32 + half, at.y as f32) } else { (at.x as f32, at.y as f32 + half) };
        let start = self.static_cubes.len();
        let r = width as f32 / 2.0;
        let spring = 1.75;
        let spec = Arch {
            centre: Vec3::new(cx, floor, cz),
            across_x,
            r,
            rise: r,
            spring,
            depth: 0.38 + 0.08 * (width - 1) as f32,
            piers: true,
            lintel: true,
            seed: (at.x as f64, at.y as f64),
        };
        let crown = arch_cubes(&mut self.static_cubes, &spec);
        let len = self.static_cubes.len() - start;
        let shades = self.static_cubes[start..start + len].iter().map(|c| c.emissive[3]).collect();
        let foot = Vec3::new(cx, floor + spring - 0.2, cz);
        self.trees.push(Tree {
            range: (start, len),
            base: foot,
            radius: r + 0.25,
            height: r + 0.7,
            fade: 1.0,
            alive: true,
            shades: Some(shades),
        });
        // The lantern under the crown.
        self.hanging_lantern(at, crown - Vec3::Y * 0.12, 0.55);
    }

    /// A lantern hung from `from` (see `lantern_cubes`); it lights the vault (one of the
    /// torches, keyed by the tile `at`).
    fn hanging_lantern(&mut self, at: P, from: Vec3, drop: f32) {
        let start = self.static_cubes.len();
        let c = lantern_cubes(&mut self.static_cubes, from, drop);
        // (The light sits at the glass: torches are lit half a unit above their point.)
        self.torches.insert(at, (c - Vec3::Y * 0.5, (start, self.static_cubes.len() - start)));
    }

    /// A lantern on an iron bracket from the wall's face over the road `d`.
    fn wall_lantern(&mut self, wall: P, base: f32, d: Dir) {
        let (dx, dy) = d.vec();
        let face = Vec3::new(wall.x as f32 + dx as f32 * 0.5, base + 1.45, wall.y as f32 + dy as f32 * 0.5);
        let out = Vec3::new(dx as f32, 0.0, dy as f32);
        let hook = bracket_cubes(&mut self.static_cubes, face, out);
        self.hanging_lantern(wall, hook, 0.1);
    }

    /// A spout in the wall's face over the road `d`: a pale stone mask with a dark mouth and a
    /// copper lip, pouring a thin fall into the water below (drawn each frame), which pools at
    /// the wall's foot (`foot`, the road's floor).
    #[allow(clippy::too_many_arguments)]
    fn spout(&mut self, x: f32, y: f32, base: f32, d: Dir, foot: f32, hx: f64, hy: f64) {
        let (dx, dy) = d.vec();
        let out = Vec3::new(dx as f32, 0.0, dy as f32);
        let face = Vec3::new(x, base + 0.78, y) + out * 0.5;
        let mouth = spout_cubes(&mut self.static_cubes, face, out, (hx, hy));
        self.spouts.push((mouth, out, foot));
        // It pools where it falls.
        let below = Vec3::new(x, 0.0, y) + out;
        if !self.flood.iter().any(|p| (p.x - below.x).abs() < 0.1 && (p.z - below.z).abs() < 0.1) {
            self.flood.push(Vec3::new(below.x, foot + 0.03, below.z));
        }
    }

    /// The cisterns' water: still pools over the floors, and the spouts' falls with their foam.
    pub(super) fn emit_cistern_water(&self, time: f32, f: &mut Frame) {
        for (i, p) in self.flood.iter().enumerate() {
            emit_pool(f, *p, Vec2::ONE, i as f32 * 0.7);
        }
        for (i, &(mouth, out, foot)) in self.spouts.iter().enumerate() {
            emit_fall(f, mouth, out, foot, time, i as f32);
        }
    }

    /// A hooded knight of stone on top of the wall, looking down on the road `d` (see
    /// `statue_cubes`). A monument over the maze; it fades out of the way of the view of him.
    fn statue(&mut self, x: f32, y: f32, base: f32, d: Dir, hx: f64, hy: f64) {
        let (dx, dy) = d.vec();
        let yaw = -(dy as f32).atan2(dx as f32) + PI / 2.0;
        let origin = Vec3::new(x + dx as f32 * 0.08, base, y + dy as f32 * 0.08);
        let start = self.static_cubes.len();
        statue_cubes(&mut self.static_cubes, origin, yaw, 1.0, (hx, hy));
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

    /// A little heap of bones (see `bone_cubes`).
    fn bone(&mut self, at: Vec3, h: f32) {
        bone_cubes(&mut self.static_cubes, at, h);
    }
}

// ------------------------------------------------------------------------- the builders
//
// The vaults' pieces as plain builders on a list of cubes, so other sets (the story's gateway)
// are built of the same stone.

/// A monumental pillar standing at `p` (its foot): a square shaft of dark stone and umber drums
/// on a moulded pale base, under a pale capital; `h` tall in `drums`, `w` times the vaults' girth.
pub(super) fn pillar_cubes(out: &mut Vec<Instance>, p: Vec3, h: f32, drums: usize, w: f32, (hx, hy): (f64, f64)) {
    block(out, p + Vec3::Y * 0.12, Vec3::new(0.62 * w, 0.24, 0.62 * w), 0.0, jit(PALE_STONE[0], hash(hx, hy), 0.05), SHADE_STONE);
    for k in 0..drums {
        let dh = (h - 0.5) / drums as f32;
        let c = if k % 3 == 1 { jit(UMBER[0], hash(hy, hx + k as f64), 0.06) } else { jit(CHARCOAL[k % 3], hash(hx + k as f64, hy), 0.08) };
        block(out, p + Vec3::Y * (0.24 + dh * (k as f32 + 0.5)), Vec3::new(0.46 * w, dh - 0.02, 0.46 * w), 0.0, c, SHADE_STONE);
    }
    block(
        out,
        p + Vec3::Y * (h - 0.13),
        Vec3::new(0.66 * w, 0.26, 0.66 * w),
        0.0,
        jit(PALE_STONE[1], hash(hy, hx), 0.05) * 0.75,
        SHADE_STONE,
    );
}

/// A banner on a wall's face at `wall` (the wall's middle at its foot), toward `d`: a long drop
/// of copper-red or umber cloth from a copper rod, an amber sigil sewn on it, its foot cut in a
/// swallowtail; `lift` higher than a vault's.
pub(super) fn banner_cubes(out: &mut Vec<Instance>, wall: Vec3, (dx, dy): (i32, i32), lift: f32, (hx, hy): (f64, f64)) {
    let turn = if dx != 0 { PI / 2.0 } else { 0.0 };
    let at = |h: f32, o: f32, u: f32| {
        Vec3::new(
            wall.x + dx as f32 * o + if dx != 0 { 0.0 } else { u },
            wall.y + lift + h,
            wall.z + dy as f32 * o + if dx != 0 { u } else { 0.0 },
        )
    };
    let cloth = lin(BANNER[(hash(hx, hy * 3.0) * 2.0) as usize % 2]);
    block(out, at(1.42, 0.53, 0.0), Vec3::new(0.62, 0.04, 0.04), turn, lin(COPPER), SHADE_BRONZE);
    block(out, at(0.95, 0.52, 0.0), Vec3::new(0.48, 0.9, 0.02), turn, cloth, SHADE_CLOTH);
    for u in [-0.15f32, 0.15] {
        block(out, at(0.43, 0.52, u), Vec3::new(0.18, 0.14, 0.02), turn, cloth, SHADE_CLOTH);
    }
    // The sigil: a ring and a cross bar, in amber thread.
    let thread = lin(AMBER);
    block(out, at(1.08, 0.535, 0.0), Vec3::new(0.06, 0.32, 0.01), turn, thread, SHADE_CLOTH);
    block(out, at(1.12, 0.535, 0.0), Vec3::new(0.24, 0.05, 0.01), turn, thread, SHADE_CLOTH);
    block(out, at(0.92, 0.535, 0.0), Vec3::new(0.18, 0.04, 0.01), turn, thread, SHADE_CLOTH);
}

/// A copper brazier standing at `pos`, `s` times the vaults' size: a bowl on three legs, its
/// coals and (with `flames`; else they are drawn as they move) flames burning amber.
pub(super) fn brazier_cubes(out: &mut Vec<Instance>, pos: Vec3, s: f32, flames: bool) {
    let copper = lin(COPPER);
    for k in 0..3 {
        let a = k as f32 * TAU / 3.0;
        block(out, pos + Vec3::new(a.cos() * 0.13, 0.3, a.sin() * 0.13) * s, Vec3::new(0.05, 0.6, 0.05) * s, a, copper * 0.8, SHADE_BRONZE);
    }
    block(out, pos + Vec3::Y * 0.64 * s, Vec3::new(0.42, 0.1, 0.42) * s, PI / 4.0, copper, SHADE_BRONZE);
    block(out, pos + Vec3::Y * 0.72 * s, Vec3::new(0.5, 0.06, 0.5) * s, 0.0, copper * 0.9, SHADE_BRONZE);
    let ember = lin(EMBER);
    glowing_block(out, pos + Vec3::Y * 0.77 * s, Vec3::new(0.36, 0.06, 0.36) * s, lin(0x3a1a0a), ember * 1.2);
    if !flames {
        return;
    }
    for (k, (ox, oz, q)) in [(0.0f32, 0.0f32, 0.16f32), (0.08, 0.05, 0.1), (-0.07, -0.04, 0.11)].into_iter().enumerate() {
        let h = 0.86 + k as f32 * 0.04;
        glowing_block(out, pos + Vec3::new(ox, h, oz) * s, Vec3::splat(q * s), lin(AMBER), lin(AMBER) * (2.5 - k as f32 * 0.5));
    }
}

/// Moss creeping over a wall's top round `at` (on the top), over a patch `span` across.
pub(super) fn moss_cap(out: &mut Vec<Instance>, at: Vec3, span: f32, (hx, hy): (f64, f64)) {
    let hk = hash(hx * 1.7 + 9.0, hy * 2.9 + 4.0);
    if hk < 0.45 {
        // A creeping patch in small clumps, thickest in its middle.
        let (px, pz) = ((hash(hx * 2.0, hy) as f32 - 0.5) * 0.5, (hash(hy * 2.0, hx) as f32 - 0.5) * 0.5);
        for k in 0..3 + (hk * 14.0) as usize {
            let a = hash(hx + k as f64 * 3.1, hy * 1.3 + k as f64) as f32;
            let b = hash(hy + k as f64 * 2.3, hx * 1.1 + k as f64) as f32;
            let (a, b) = (0.5 + px + (a - 0.5) * 0.45, 0.5 + pz + (b - 0.5) * 0.45);
            let size = Vec3::new(0.08 + (a * 7.0).fract() * 0.12, 0.025 + (b * 5.0).fract() * 0.035, 0.08 + (b * 9.0).fract() * 0.12);
            let p = Vec3::new(at.x + (a - 0.5) * span, at.y + size.y / 2.0, at.z + (b - 0.5) * span);
            block(out, p, size, a * 3.0, jit(MOSS[k % 3], b as f64, 0.1), SHADE_SOFT);
        }
    }
}

/// Moss in a flagstone's seam (now and then), the tile's middle at `at` on its top.
pub(super) fn moss_seam(out: &mut Vec<Instance>, at: Vec3, (hx, hy): (f64, f64)) {
    let hm = hash(hx * 4.1 + 2.0, hy * 3.7 + 6.0);
    if hm < 0.16 {
        // Moss in a seam, along x or along z through the tile's middle.
        let along_x = hm < 0.08;
        let len = 0.3 + hm as f32 * 3.0;
        let size = if along_x { Vec3::new(len, 0.025, 0.05) } else { Vec3::new(0.05, 0.025, len) };
        let off = (hash(hy, hx * 2.0) as f32 - 0.5) * 0.4;
        let p = Vec3::new(at.x + if along_x { off } else { 0.0 }, at.y + 0.005, at.z + if along_x { 0.0 } else { off });
        block(out, p, size, 0.0, jit(MOSS[(hm * 30.0) as usize % 3], hm, 0.1), SHADE_SOFT);
    }
}

/// An arch of dressed stone: `centre` on the floor under its crown, its span across x
/// (`across_x`) or z, `r` from the middle to each side and rising `rise` over its springing
/// (round when they are equal), `depth` through. With `piers`, pale piers from the floor to the
/// spring standing proud of the walls; with `lintel`, a course over it and the haunches filled
/// up to it. Returns the point under the crown.
#[derive(Clone, Copy)]
pub(super) struct Arch {
    pub centre: Vec3,
    pub across_x: bool,
    pub r: f32,
    pub rise: f32,
    pub spring: f32,
    pub depth: f32,
    pub piers: bool,
    pub lintel: bool,
    pub seed: (f64, f64),
}

pub(super) fn arch_cubes(out: &mut Vec<Instance>, a: &Arch) -> Vec3 {
    let (hx, hy) = a.seed;
    let (cx, floor, cz) = (a.centre.x, a.centre.y, a.centre.z);
    // Local frame: u across the span, w through it.
    let frame =
        |u: f32, h: f32, w: f32| if a.across_x { Vec3::new(cx + u, floor + h, cz + w) } else { Vec3::new(cx + w, floor + h, cz + u) };
    // (A quarter turn back, so the local across axis lands on +z as `frame` puts it.)
    let turn = if a.across_x { 0.0 } else { -PI / 2.0 };
    let (r, rise, spring, depth) = (a.r, a.rise, a.spring, a.depth);
    if a.piers {
        for side in [-1.0f32, 1.0] {
            let c = jit(PALE_STONE[(hash(hx + side as f64, hy) * 2.0) as usize % 2], hash(hy, hx + side as f64), 0.06) * 0.8;
            block(out, frame(side * r, spring / 2.0, 0.0), Vec3::new(0.22, spring, depth + 0.04), turn, c, SHADE_STONE);
            block(out, frame(side * (r - 0.02), spring + 0.05, 0.0), Vec3::new(0.3, 0.1, depth + 0.1), turn, c * 1.05, SHADE_STONE);
        }
    }
    // The wedge stones round the curve, each square to it, the middle one the keystone.
    let wedges = 7 + (4.0 * r).round() as usize;
    let arc = PI * (r + rise) / 2.0;
    for k in 0..wedges {
        let t = PI * (k as f32 + 0.5) / wedges as f32;
        let (u, h) = (-t.cos() * r, spring + 0.1 + t.sin() * rise);
        let tangent = (rise * t.cos()).atan2(r * t.sin());
        let key = k == wedges / 2;
        let c = if key { jit(PALE_STONE[0], hash(hx, hy), 0.04) } else { jit(CHARCOAL[k % 3], hash(hx + k as f64, hy), 0.1) };
        let size = Vec3::new(arc / wedges as f32 + 0.02, if key { 0.32 } else { 0.24 }, depth + if key { 0.06 } else { 0.0 });
        let rot = Quat::from_rotation_y(turn) * Quat::from_rotation_z(tangent);
        block_m(out, Mat4::from_scale_rotation_translation(size, rot, frame(u, h, 0.0)), c, SHADE_STONE);
    }
    if a.lintel {
        // The lintel course over it, and the haunches filled up to it.
        block(
            out,
            frame(0.0, spring + 0.1 + rise + 0.28, 0.0),
            Vec3::new(2.0 * r + 0.24, 0.2, depth + 0.02),
            turn,
            jit(UMBER[0], hash(hy, hx), 0.08),
            SHADE_STONE,
        );
        for side in [-1.0f32, 1.0] {
            let fill = rise * 0.9;
            block(
                out,
                frame(side * (r - 0.12), spring + 0.1 + fill / 2.0 + 0.1, 0.0),
                Vec3::new(0.26, fill + 0.2, depth),
                turn,
                jit(CHARCOAL[2], hash(hx, hy + side as f64), 0.1),
                SHADE_STONE,
            );
        }
        // A little moss on the lintel, now and then.
        if hash(hx * 3.0, hy * 5.0) < 0.5 {
            block(out, frame(0.15, spring + 0.1 + rise + 0.4, 0.0), Vec3::new(0.5, 0.04, depth * 0.8), turn, lin(MOSS[1]), SHADE_SOFT);
        }
    }
    frame(0.0, spring + 0.1 + rise, 0.0)
}

/// A lantern of copper and amber glass hung on a chain of `drop` from `from`. Returns the
/// middle of its glass.
pub(super) fn lantern_cubes(out: &mut Vec<Instance>, from: Vec3, drop: f32) -> Vec3 {
    let links = (drop / 0.07) as usize;
    for k in 0..links {
        let turn = if k % 2 == 0 { 0.0 } else { PI / 2.0 };
        block(out, from - Vec3::Y * (k as f32 * 0.07 + 0.035), Vec3::new(0.035, 0.06, 0.012), turn, lin(IRON), SHADE_IRON);
    }
    let c = from - Vec3::Y * (drop + 0.13);
    let copper = lin(COPPER);
    block(out, c + Vec3::Y * 0.13, Vec3::new(0.2, 0.04, 0.2), PI / 4.0, copper, SHADE_BRONZE);
    block(out, c - Vec3::Y * 0.12, Vec3::new(0.18, 0.04, 0.18), PI / 4.0, copper * 0.9, SHADE_BRONZE);
    for k in 0..4 {
        let a = k as f32 * TAU / 4.0 + PI / 4.0;
        block(out, c + Vec3::new(a.cos() * 0.09, 0.0, a.sin() * 0.09), Vec3::new(0.025, 0.24, 0.025), 0.0, copper * 0.8, SHADE_BRONZE);
    }
    glowing_block(out, c, Vec3::new(0.13, 0.2, 0.13), lin(AMBER), lin(AMBER) * 1.8);
    c
}

/// An iron bracket out of a wall's face at `face`, toward `out`. Returns the point a lantern
/// hangs from.
pub(super) fn bracket_cubes(out: &mut Vec<Instance>, face: Vec3, dir: Vec3) -> Vec3 {
    let iron = lin(IRON);
    block(out, face + dir * 0.02, Vec3::new(0.12, 0.2, 0.12), 0.0, iron, SHADE_IRON);
    block(
        out,
        face + dir * 0.17 + Vec3::Y * 0.06,
        Vec3::new(0.3, 0.035, 0.3) * (dir.abs() + Vec3::new(0.0, 1.0, 0.0)) + Vec3::new(0.03, 0.0, 0.03),
        0.0,
        iron,
        SHADE_IRON,
    );
    face + dir * 0.3 + Vec3::Y * 0.04
}

/// A spout's mask on a wall's face at `face`, toward `dir`: a pale stone boss with a brow, a
/// dark mouth and a copper lip green with age, and the green the water has left below it.
/// Returns where its water leaves the lip.
pub(super) fn spout_cubes(out: &mut Vec<Instance>, face: Vec3, dir: Vec3, (hx, hy): (f64, f64)) -> Vec3 {
    let turn = if dir.x != 0.0 { PI / 2.0 } else { 0.0 };
    let pale = jit(PALE_STONE[(hash(hx, hy) * 2.0) as usize % 2], hash(hy, hx), 0.05) * 0.75;
    block(out, face + dir * 0.1, Vec3::new(0.34, 0.34, 0.08), turn, pale, SHADE_STONE);
    block(out, face + dir * 0.13 + Vec3::Y * 0.14, Vec3::new(0.38, 0.06, 0.1), turn, pale * 0.9, SHADE_STONE);
    for side in [-1.0f32, 1.0] {
        let across = Vec3::new(dir.z, 0.0, dir.x) * side * 0.08;
        block(out, face + dir * 0.145 + across + Vec3::Y * 0.06, Vec3::new(0.05, 0.04, 0.02), turn, lin(0x0e0c0a), SHADE_LIT);
    }
    block(out, face + dir * 0.145 - Vec3::Y * 0.06, Vec3::new(0.12, 0.07, 0.02), turn, lin(0x0e0c0a), SHADE_LIT);
    let lip = face + dir * 0.2 - Vec3::Y * 0.09;
    block(
        out,
        lip,
        if dir.x != 0.0 { Vec3::new(0.16, 0.03, 0.1) } else { Vec3::new(0.1, 0.03, 0.16) },
        0.0,
        lin(VERDIGRIS) * 1.1,
        SHADE_BRONZE,
    );
    block(out, face + dir * 0.065 - Vec3::Y * 0.42, Vec3::new(0.14, 0.5, 0.02), turn, jit(VERDIGRIS, hx, 0.1) * 0.8, SHADE_STONE);
    lip + dir * 0.07
}

/// Shallow standing water at `p` (its surface), `size` across: barely moving, the stone
/// showing through, mostly a sheen that holds the lamps.
pub(super) fn emit_pool(f: &mut Frame, p: Vec3, size: Vec2, drift: f32) {
    let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(size.x, 0.002, size.y));
    let flow = Vec3::new(drift.cos() * 0.03, drift.sin() * 0.03, 0.0);
    f.transparent[MeshKind::Cube as usize].push(Instance::new(m, lin(0x10201f), 0.4, flow, SHADE_WATER));
}

/// A thin fall of water from `mouth` (leaving toward `out`) down to the pool at `foot`, with
/// fine drops thrown up where it lands and rings spreading.
pub(super) fn emit_fall(f: &mut Frame, mouth: Vec3, out: Vec3, foot: f32, time: f32, i: f32) {
    let cube_i = MeshKind::Cube as usize;
    // The fall bows out a little from the lip as it drops.
    let drop = mouth.y - foot - 0.03;
    let at = mouth + out * 0.06 - Vec3::Y * drop / 2.0;
    let size = if out.x != 0.0 { Vec3::new(0.002, drop, 0.1) } else { Vec3::new(0.1, drop, 0.002) };
    f.transparent[cube_i].push(Instance::new(
        Mat4::from_translation(at) * Mat4::from_scale(size),
        lin(0xc8dce4),
        0.85,
        Vec3::ZERO,
        SHADE_WATER,
    ));
    let foot_at = Vec3::new(at.x, foot + 0.035, at.z);
    for k in 0..6 {
        let fk = k as f32 * 1.7 + i * 3.1;
        let t = (time * 1.6 + fk * 0.31).fract();
        let a = fk * 2.4;
        let p = foot_at + Vec3::new(a.cos() * 0.14 * t, 0.12 * (PI * t).sin(), a.sin() * 0.14 * t);
        let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.018));
        f.additive.push(Instance::new(m, lin(0xcfe0ea), 0.45 * (1.0 - t), Vec3::ZERO, SHADE_UNLIT));
    }
    for k in 0..2 {
        let t = (time * 0.7 + k as f32 * 0.5 + i * 0.37).fract();
        let r = 0.06 + t * 0.32;
        let m = Mat4::from_translation(foot_at) * Mat4::from_rotation_x(-PI / 2.0) * Mat4::from_scale(Vec3::new(r, r, 1.0));
        f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, lin(0x9fb8be), 0.3 * (1.0 - t), Vec3::ZERO, SHADE_UNLIT));
    }
}

/// A hooded knight of stone on a plinth, `s` times the vaults' size, standing at `origin`
/// facing `yaw` (+z turned by it): a long robe, a mantle over the shoulders, the hood's shadow for
/// a face, both hands on the pommel of a sword stood point down before him.
pub(super) fn statue_cubes(out: &mut Vec<Instance>, origin: Vec3, yaw: f32, s: f32, (hx, hy): (f64, f64)) {
    let q = Quat::from_rotation_y(yaw);
    // Local: x across him, y up, z toward the road.
    let at = |l: Vec3| origin + q * (l * s);
    let stone = jit(CHARCOAL[1], hash(hx, hy), 0.06) * 0.85;
    let dark = stone * 0.7;
    let pale = jit(PALE_STONE[0], hash(hy, hx), 0.05);
    let mut put = |c: Vec3, size: Vec3, col: Vec3| {
        let m = Mat4::from_scale_rotation_translation(size * s, q, at(c));
        out.push(Instance::new(m, col, 1.0, Vec3::ZERO, SHADE_STONE));
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
            Vec3::new(0.13, 0.42, 0.14) * s,
            q * Quat::from_rotation_x(-0.5) * Quat::from_rotation_z(side * 0.35),
            at(Vec3::new(side * 0.2, 1.12, 0.12)),
        );
        out.push(Instance::new(arm, stone * 0.92, 1.0, Vec3::ZERO, SHADE_STONE));
    }
    let mut put = |c: Vec3, size: Vec3, col: Vec3| {
        let m = Mat4::from_scale_rotation_translation(size * s, q, at(c));
        out.push(Instance::new(m, col, 1.0, Vec3::ZERO, SHADE_STONE));
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
}

/// A little heap of bones: a skull and a few long bones, pale on the stone.
pub(super) fn bone_cubes(out: &mut Vec<Instance>, at: Vec3, h: f32) {
    let bone = lin(BONE);
    block(out, at + Vec3::Y * 0.06, Vec3::new(0.12, 0.11, 0.14), h * 6.0, bone, SHADE_STONE);
    block(out, at + Vec3::new(0.0, 0.04, 0.07), Vec3::new(0.08, 0.04, 0.03), h * 6.0, bone * 0.8, SHADE_STONE);
    for k in 0..2 {
        let a = h * 4.0 + k as f32 * 1.9;
        block(out, at + Vec3::new(a.cos() * 0.14, 0.015, a.sin() * 0.14), Vec3::new(0.24, 0.03, 0.035), a, bone * 0.92, SHADE_STONE);
    }
}

/// Flagstones over the unit cell at `at` (its middle, on its top): dark grout and four slabs,
/// each a touch askew in height, in the board's three flag greys.
pub(super) fn flagstones(out: &mut Vec<Instance>, at: Vec3, (hx, hy): (f64, f64)) {
    block(out, at - Vec3::Y * 0.06, Vec3::new(1.0, 0.1, 1.0), 0.0, lin(GROUT), SHADE_LIT);
    for (k, (ox, oz)) in [(-0.25f32, -0.25f32), (0.25, -0.25), (-0.25, 0.25), (0.25, 0.25)].into_iter().enumerate() {
        let hk = hash(hx * 2.0 + ox as f64 * 7.0 + 1.3, hy * 2.0 + oz as f64 * 5.0);
        let c = jit(FLAG[(hk * 3.0) as usize % 3], hash(hy + k as f64, hx), 0.1);
        let tilt = (hk as f32 - 0.5) * 0.012;
        block(out, at + Vec3::new(ox, -0.025 + tilt, oz), Vec3::new(0.47, 0.05, 0.47), 0.0, c, SHADE_FLAG);
    }
}
