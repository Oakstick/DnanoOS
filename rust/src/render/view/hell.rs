//! The Portals of Hell (`logic::hub` lays it out): eight stone platforms
//! floating over a sea of lava round a volcano, joined by rope-railed wooden
//! bridges. The platforms are cracked basalt flagstones with glowing veins,
//! jagged rock hanging beneath them, bones and broken columns on them. The
//! volcano smokes, lava rivers run down its flanks, and the House of the
//! Devil stands in its south face, horned spires and red windows, behind a
//! wall of fire until the seal breaks and the causeways rise. Each sin's
//! portal is a black arch with its name in gold letters over doors that show
//! the sin in relief, braziers burning in its colour, and a swirl of its
//! realm behind the doors when they open. The hero came in from the cliffs
//! to the south, through a cave.

use super::*;
use crate::logic::hub::{self, Gate, Platform, Rect};
use crate::logic::sins::{Sin, SINS};
use crate::render::voxels::Mesh;

mod pal {
    pub const BASALT: [u32; 4] = [0x3e3838, 0x4a4242, 0x585050, 0x363030];
    pub const SLAB: [u32; 4] = [0x6a605a, 0x5a524e, 0x786c64, 0x645a54];
    pub const CRUST: u32 = 0x2a120c;
    pub const LAVA_DEEP: u32 = 0x8a1e06;
    pub const LAVA: u32 = 0xff4a0a;
    pub const LAVA_HOT: u32 = 0xffb040;
    pub const BONE: u32 = 0xd8ccb0;
    pub const BONE_D: u32 = 0xa89878;
    pub const WOOD: [u32; 3] = [0x5a3a22, 0x4a3020, 0x6a4428];
    pub const WOOD_D: u32 = 0x3a2616;
    pub const CHAR: u32 = 0x221610;
    pub const ROPE: u32 = 0x8a6a42;
    pub const IRON: u32 = 0x2a2a2e;
    pub const IRON_L: u32 = 0x4a4650;
    pub const CHAIN: u32 = 0x6e6a74;
    pub const CHAIN_D: u32 = 0x4c4852;
    pub const OBSIDIAN: u32 = 0x1c1622;
    pub const OBSIDIAN_L: u32 = 0x2e2638;
    pub const VOID: u32 = 0x050305;
    pub const ASH: u32 = 0x5a5250;
    pub const HORN: u32 = 0xd8c8a8;
    pub const HORN_D: u32 = 0x3a3028;
    pub const GOLD: u32 = 0xe8b84a;
    pub const EMBER: u32 = 0xff7a20;
    pub const WINDOW: u32 = 0xff3010;
}
use crate::logic;
use crate::logic::sins::ALL_SINS;
use crate::logic::types::Dir;
use pal::*;

/// The lava's surface, far below the platforms.
const LAVA_Y: f32 = -2.45;
/// The portal doors: a voxel of their relief, and the canvas they are drawn on (both leaves).
const DOOR_VOXEL: f32 = 0.03;
const DOOR_W: i32 = 40;
const DOOR_H: i32 = 80;
/// A voxel of the gold letters over each portal.
const LETTER_VOXEL: f32 = 0.035;
/// How far from the seal stone its seven sockets stand.
const SEAL_RING: f32 = 0.5;

/// Dusk in hell: a dark red sky, the lava lighting everything from below, embers in the air.
pub fn hell_atmosphere() -> Atmosphere {
    Atmosphere {
        background: 0x2a0a0e,
        fog_near: 18.0,
        fog_far: 52.0,
        hemi_sky: lin(0x9a6a7a) * 0.9,
        hemi_ground: lin(0xff5a1a) * 0.6,
        ambient: lin(0x6a4448) * 0.6,
        sun_dir: Vec3::new(0.35, 0.8, 0.45).normalize(),
        sun: lin(0xffb08a) * 1.7,
        exposure: 1.3,
        night_sky: 0.0,
        bloom: 1.2,
        moon_view: Vec3::new(0.0, 0.3, -1.0).normalize(),
        cloud_tint: lin(0x3a1010),
    }
}

fn cube(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot_y: f32, color: Vec3) {
    out.push(Instance::new(
        Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(rot_y), pos),
        color,
        1.0,
        Vec3::ZERO,
        SHADE_LIT,
    ));
}

fn glowing(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot_y: f32, color: Vec3, emissive: Vec3) {
    out.push(Instance::new(
        Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(rot_y), pos),
        color,
        1.0,
        emissive,
        SHADE_LIT,
    ));
}

/// Smooth value noise, 0 to 1.
fn vnoise(x: f32, z: f32) -> f32 {
    let (xi, zi) = (x.floor(), z.floor());
    let (fx, fz) = (x - xi, z - zi);
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let h = |a: f32, b: f32| hash(a as f64 * 1.37 + 11.0, b as f64 * 1.71 + 3.0) as f32;
    let (a, b, c, d) = (h(xi, zi), h(xi + 1.0, zi), h(xi, zi + 1.0), h(xi + 1.0, zi + 1.0));
    let (u, v) = (s(fx), s(fz));
    a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v
}

/// How hot the lava runs at a point: rivers of glow between cooling crust.
fn lava_heat(x: f32, z: f32) -> f32 {
    0.5 * vnoise(x * 0.22, z * 0.22) + 0.32 * vnoise(x * 0.6 + 7.0, z * 0.6 + 3.0) + 0.18 * vnoise(x * 1.7, z * 1.7)
}

/// The lava's colour and glow at a point.
fn lava_look(x: f32, z: f32) -> (Vec3, Vec3) {
    let hot = ((lava_heat(x, z) - 0.47) / 0.33).clamp(0.0, 1.0);
    let color = lin(CRUST).lerp(lin(LAVA_DEEP), hot);
    let glow = lin(LAVA).lerp(lin(LAVA_HOT), ((hot - 0.7) / 0.3).max(0.0)) * (hot * hot * 1.5 + 0.035);
    (color, glow)
}

fn stone(h: f64, set: &[u32]) -> Vec3 {
    lin(set[(h * set.len() as f64) as usize % set.len()]) * (0.92 + h as f32 * 0.14)
}

/// The yaw that turns a model built facing +z to face `d`.
fn yaw_of(d: Dir) -> f32 {
    let (dx, dz) = d.vec();
    (dx as f32).atan2(dz as f32)
}

/// A root for a portal gate: at its tile, turned to face into its platform.
fn gate_root(g: &Gate) -> Mat4 {
    Mat4::from_translation(Vec3::new(g.tile.x as f32, 0.0, g.tile.y as f32)) * Mat4::from_rotation_y(yaw_of(g.facing))
}

/// Puts a box at `local` (centre) under `root`.
fn put(out: &mut Vec<Instance>, root: Mat4, local: Vec3, scale: Vec3, color: Vec3, emissive: Vec3) {
    out.push(Instance::new(root * Mat4::from_scale_rotation_translation(scale, Quat::IDENTITY, local), color, 1.0, emissive, SHADE_LIT));
}

impl WorldView {
    /// The hub's static geometry.
    pub(super) fn build_hell(&mut self, grid: &Grid) {
        self.water.clear();
        self.build_lava_sea(grid);
        if self.gym {
            // The gym: its platforms and bridges over the same lava, nothing else.
            use logic::gym;
            for k in 0..gym::STAGES {
                self.build_platform(grid, Platform { rect: gym::platform(k), col: k as i32, row: 0, sin: None });
            }
            for b in gym::bridges() {
                self.build_wood_bridge(b);
            }
            return;
        }
        for pl in hub::platforms() {
            self.build_platform(grid, pl);
        }
        for b in hub::bridges() {
            self.build_wood_bridge(b);
        }
        self.build_ledge_and_cliffs();
        self.build_volcano();
        for g in hub::gates() {
            self.build_portal_arch(&g);
        }
        self.build_altar();
        self.build_far_rocks();
    }

    /// Far off across the lava: spires and stacks of black rock standing out of the sea, dim in the fog.
    fn build_far_rocks(&mut self) {
        let out = &mut self.static_cubes;
        let c = (hub::HUB_SIZE - 1) as f32 / 2.0;
        for k in 0..44 {
            let fk = k as f64;
            let a = k as f32 / 44.0 * TAU + (hash(fk, 21.0) as f32 - 0.5) * 0.12;
            let r = 44.0 + hash(fk * 1.7, 5.0) as f32 * 10.0;
            let (x, z) = (c + a.cos() * r, c + a.sin() * r);
            if z > c + 28.0 && (x - c).abs() < 20.0 {
                continue; // the cliffs he came down stand there
            }
            let tall = 4.0 + hash(fk * 3.1, 9.0) as f32 * 9.0;
            let wide = 1.2 + hash(fk * 2.3, 4.0) as f32 * 2.0;
            let turn = hash(fk, 13.0) as f32 * PI;
            let lean = Vec3::new(hash(fk, 23.0) as f32 - 0.5, 0.0, hash(29.0, fk) as f32 - 0.5) * 1.2;
            for s in 0..6 {
                let t = s as f32 / 6.0;
                let hs = hash(fk + s as f64, 2.0);
                let sz = wide * (1.0 - t * 0.85) * (0.85 + hs as f32 * 0.3);
                let p = Vec3::new(x, LAVA_Y + tall * (t + 0.08), z)
                    + lean * t
                    + Vec3::new((hs as f32 - 0.5) * 0.5, 0.0, (hash(s as f64, fk) as f32 - 0.5) * 0.5);
                cube(out, p, Vec3::new(sz, tall / 6.0 + 0.15, sz * 0.8), turn + t * 0.9, stone(hs, &BASALT) * 0.7);
            }
            // A lesser spike beside it.
            let side = Vec3::new(hash(fk * 5.0, 3.0) as f32 - 0.5, 0.0, hash(7.0, fk * 5.0) as f32 - 0.5) * 3.0;
            for s in 0..3 {
                let t = s as f32 / 3.0;
                let sz = wide * 0.5 * (1.0 - t * 0.8);
                cube(
                    out,
                    Vec3::new(x, LAVA_Y + tall * 0.45 * (t + 0.15), z) + side,
                    Vec3::new(sz, tall * 0.15 + 0.1, sz * 0.8),
                    turn + 1.0 + t,
                    stone(hash(fk, s as f64 + 8.0), &BASALT) * 0.65,
                );
            }
            if hash(fk, 17.0) < 0.35 {
                glowing(
                    out,
                    Vec3::new(x, LAVA_Y + tall * 0.35, z + wide * 0.4),
                    Vec3::new(0.12, tall * 0.4, 0.08),
                    turn,
                    lin(LAVA_DEEP),
                    lin(LAVA) * 1.2,
                );
            }
        }
    }

    /// Lava under and round everything: fine near the platforms, coarser further out.
    fn build_lava_sea(&mut self, grid: &Grid) {
        let out = &mut self.static_cubes;
        let (gw, gh) = (grid.width, grid.height);
        let n = gw.max(gh);
        for y in 0..gh {
            for x in 0..gw {
                if grid.get(x, y) != Tile::Pit && !(x == gw - 1 || y == gh - 1 || x == 0 || y == 0) {
                    continue;
                }
                for (sx, sz) in [(-0.25f32, -0.25f32), (0.25, -0.25), (-0.25, 0.25), (0.25, 0.25)] {
                    let (px, pz) = (x as f32 + sx, y as f32 + sz);
                    let (c, e) = lava_look(px, pz);
                    let lift = if e.x < 0.3 { 0.03 } else { 0.0 };
                    glowing(out, Vec3::new(px, LAVA_Y - 0.05 + lift, pz), Vec3::new(0.5, 0.1, 0.5), 0.0, c, e);
                }
            }
        }
        // Beyond the hub: one-tile blocks to 24 tiles out, then three-tile blocks to 48.
        for (step, reach, inner) in [(1.0f32, 24.0f32, 0.0f32), (3.0, 48.0, 24.0)] {
            let lo = -reach;
            let hi = n as f32 - 1.0 + reach;
            let mut z = lo;
            while z <= hi {
                let mut x = lo;
                while x <= hi {
                    let inside_hub = x > -0.5 && x < n as f32 - 0.5 && z > -0.5 && z < n as f32 - 0.5;
                    let inside_inner = x > -inner - 0.5 && x < n as f32 - 0.5 + inner && z > -inner - 0.5 && z < n as f32 - 0.5 + inner;
                    if !inside_hub && !(inner > 0.0 && inside_inner) {
                        let (c, e) = lava_look(x, z);
                        glowing(out, Vec3::new(x, LAVA_Y - 0.05, z), Vec3::new(step, 0.1, step), 0.0, c, e);
                    }
                    x += step;
                }
                z += step;
            }
        }
    }

    /// A platform: flagstones on a basalt body with jagged rock hanging beneath, bones, broken columns, a ruined parapet.
    fn build_platform(&mut self, grid: &Grid, pl: Platform) {
        let r = pl.rect;
        let (cx, cz) = r.center();
        let (w, h) = ((r.x1 - r.x0 + 1) as f32, (r.y1 - r.y0 + 1) as f32);
        let seed = (r.x0 * 31 + r.y0 * 17) as f64;
        let out = &mut self.static_cubes;
        // The flagstones.
        for p in r.tiles() {
            let (x, z) = (p.x as f32, p.y as f32);
            let (hx, hy) = (p.x as f64, p.y as f64);
            let hk = hash(hx * 3.1, hy * 2.7);
            let tilt = (hash(hy, hx) as f32 - 0.5) * 0.05;
            if hk < 0.07 {
                // A flagstone split in two, lava glowing up through the crack.
                let along = hash(hx + 5.0, hy) < 0.5;
                for s in [-1.0f32, 1.0] {
                    let off = if along { Vec3::new(s * 0.245, 0.0, 0.0) } else { Vec3::new(0.0, 0.0, s * 0.245) };
                    let sc = if along { Vec3::new(0.44, 0.12, 0.94) } else { Vec3::new(0.94, 0.12, 0.44) };
                    cube(out, Vec3::new(x, -0.065 - s * 0.01, z) + off, sc, tilt, stone(hash(hx + s as f64, hy), &SLAB));
                }
                let sc = if along { Vec3::new(0.05, 0.02, 0.9) } else { Vec3::new(0.9, 0.02, 0.05) };
                glowing(out, Vec3::new(x, -0.03, z), sc, 0.0, lin(LAVA_DEEP), lin(LAVA) * 1.1);
            } else if hk < 0.16 {
                // A scorched flagstone, dusted with ash.
                cube(out, Vec3::new(x, -0.06, z), Vec3::new(0.95, 0.12, 0.95), tilt, stone(hash(hx, hy), &SLAB) * 0.55);
                let a = hash(hx * 2.0, hy * 9.0) as f32;
                cube(
                    out,
                    Vec3::new(x + (a - 0.5) * 0.3, 0.004, z + (hash(hy * 4.0, hx) as f32 - 0.5) * 0.3),
                    Vec3::new(0.3 + a * 0.3, 0.008, 0.25 + a * 0.2),
                    a * PI,
                    lin(ASH) * 0.8,
                );
            } else {
                cube(out, Vec3::new(x, -0.06, z), Vec3::new(0.95, 0.12, 0.95), tilt, stone(hash(hx, hy), &SLAB));
                if hk > 0.96 {
                    // A thin glowing vein across it.
                    let a = hash(hx * 7.0, hy * 5.0) as f32 * PI;
                    glowing(out, Vec3::new(x, 0.002, z), Vec3::new(0.5, 0.012, 0.03), a, lin(LAVA_DEEP), lin(LAVA) * 0.8);
                }
            }
        }
        // The body, tapering underneath.
        cube(out, Vec3::new(cx, -0.62, cz), Vec3::new(w + 0.1, 1.0, h + 0.1), 0.0, lin(BASALT[3]));
        for k in 1..=3 {
            let inset = 1.3 * k as f32;
            let top = -1.12 - 0.36 * (k - 1) as f32;
            cube(
                out,
                Vec3::new(cx, top - 0.23, cz),
                Vec3::new(w - 2.0 * inset, 0.46, h - 2.0 * inset),
                0.0,
                lin(BASALT[k as usize % 4]) * 0.9,
            );
        }
        // Jagged rock hanging beneath, longest in the middle.
        for k in 0..38 {
            let a = hash(seed + k as f64 * 3.3, seed * 0.7 - k as f64) as f32;
            let b = hash(seed * 1.3 - k as f64 * 2.1, seed + k as f64 * 5.7) as f32;
            let (px, pz) = (r.x0 as f32 - 0.3 + a * (w - 0.4), r.y0 as f32 - 0.3 + b * (h - 0.4));
            let edge = ((px - cx).abs() / (w / 2.0)).max((pz - cz).abs() / (h / 2.0));
            let base = if edge > 0.8 {
                -1.1
            } else if edge > 0.55 {
                -1.5
            } else {
                -1.9
            };
            let long = 0.5 + hash(seed + k as f64, 3.0) as f32 * 1.2 * (1.2 - edge * 0.6);
            let size = 0.45 + hash(seed - k as f64, 7.0) as f32 * 0.6;
            let turn = a * TAU;
            for s in 0..4 {
                let t = s as f32 / 4.0;
                let sz = size * (1.0 - t * 0.8);
                let y = base - long * (t + 0.125);
                cube(
                    out,
                    Vec3::new(px + (b - 0.5) * 0.1 * t, y, pz + (a - 0.5) * 0.1 * t),
                    Vec3::new(sz, long / 4.0 + 0.02, sz * 0.85),
                    turn + t,
                    stone(hash(seed + s as f64, k as f64), &BASALT) * 0.85,
                );
            }
        }
        // Veins of lava glowing in the platform's sides.
        for k in 0..6 {
            let hk = hash(seed + k as f64 * 9.1, 2.0) as f32;
            let side = k % 4;
            let (p, scale) = match side {
                0 => (Vec3::new(r.x0 as f32 - 0.56, -0.5, r.y0 as f32 + hk * (h - 1.0)), Vec3::new(0.02, 0.8, 0.06)),
                1 => (Vec3::new(r.x1 as f32 + 0.56, -0.5, r.y0 as f32 + hk * (h - 1.0)), Vec3::new(0.02, 0.8, 0.06)),
                2 => (Vec3::new(r.x0 as f32 + hk * (w - 1.0), -0.5, r.y0 as f32 - 0.56), Vec3::new(0.06, 0.8, 0.02)),
                _ => (Vec3::new(r.x0 as f32 + hk * (w - 1.0), -0.5, r.y1 as f32 + 0.56), Vec3::new(0.06, 0.8, 0.02)),
            };
            glowing(out, p, scale, 0.0, lin(LAVA_DEEP), lin(LAVA) * 1.2);
        }
        // A ruined parapet along the outer edges, broken where bridges, gates and the ledge meet it.
        let open_edge = |x: i32, y: i32| grid.get(x, y) == Tile::Pit;
        for p in r.tiles() {
            let on_edge = p.x == r.x0 || p.x == r.x1 || p.y == r.y0 || p.y == r.y1;
            if !on_edge || grid.get(p.x, p.y) != Tile::Open {
                continue;
            }
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                if !open_edge(p.x + dx, p.y + dy) || !r.contains(P::new(p.x - dx, p.y - dy)) {
                    continue;
                }
                let hk = hash(p.x as f64 * 5.3 + dx as f64, p.y as f64 * 4.1 + dy as f64);
                if hk < 0.45 {
                    continue;
                }
                let tall = 0.12 + (hk as f32 - 0.45) * 0.55;
                let pos = Vec3::new(p.x as f32 + dx as f32 * 0.42, tall / 2.0, p.y as f32 + dy as f32 * 0.42);
                let sc = if dx != 0 { Vec3::new(0.18, tall, 0.9) } else { Vec3::new(0.9, tall, 0.18) };
                cube(out, pos, sc, (hk as f32 - 0.7) * 0.2, stone(hk, &BASALT));
            }
        }
        // Bones and skulls, off the paths through the middle.
        let (mx, mz) = ((r.x0 + r.x1) / 2, (r.y0 + r.y1) / 2);
        let mut placed = 0;
        for k in 0..60 {
            if placed >= 8 {
                break;
            }
            let x = r.x0 + 1 + (hash(seed + k as f64 * 1.9, 11.0) * (w - 2.0) as f64) as i32;
            let y = r.y0 + 1 + (hash(seed - k as f64 * 2.3, 13.0) * (h - 2.0) as f64) as i32;
            if (x - mx).abs() <= 1 || (y - mz).abs() <= 1 || grid.get(x, y) != Tile::Open {
                continue;
            }
            let hk = hash(x as f64 * 7.0, y as f64 * 3.0);
            let at = Vec3::new(x as f32 + (hk as f32 - 0.5) * 0.5, 0.0, y as f32 + (hash(y as f64, x as f64) as f32 - 0.5) * 0.5);
            bones(out, at, hk);
            placed += 1;
        }
        // Heaps of broken basalt, off the paths too.
        let mut heaps = 0;
        for k in 0..60 {
            if heaps >= 5 {
                break;
            }
            let x = r.x0 + 1 + (hash(seed + k as f64 * 2.9, 17.0) * (w - 2.0) as f64) as i32;
            let y = r.y0 + 1 + (hash(seed - k as f64 * 3.7, 19.0) * (h - 2.0) as f64) as i32;
            if (x - mx).abs() <= 1 || (y - mz).abs() <= 1 || grid.get(x, y) != Tile::Open {
                continue;
            }
            let hk = hash(x as f64 * 5.0, y as f64 * 9.0);
            let at = Vec3::new(x as f32 + (hk as f32 - 0.5) * 0.4, 0.0, y as f32 + (hash(y as f64 * 2.0, x as f64) as f32 - 0.5) * 0.4);
            for j in 0..5 {
                let hj = hash(hk * 31.0 + j as f64, x as f64 + y as f64) as f32;
                let hl = hash(j as f64 * 7.0, hk * 13.0) as f32;
                let size = 0.08 + hj * 0.16;
                let p = at + Vec3::new((hl - 0.5) * 0.5, size * 0.4 + if j == 4 { 0.12 } else { 0.0 }, (hj - 0.5) * 0.5);
                cube(out, p, Vec3::new(size, size * 0.8, size * 0.9), hj * TAU, stone(hl as f64, &BASALT) * (0.9 + hj * 0.2));
            }
            heaps += 1;
        }
        // Iron chains hung over the edges, into the glow.
        for p in r.tiles() {
            let on_edge = p.x == r.x0 || p.x == r.x1 || p.y == r.y0 || p.y == r.y1;
            if !on_edge || grid.get(p.x, p.y) != Tile::Open {
                continue;
            }
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                if !open_edge(p.x + dx, p.y + dy) || !r.contains(P::new(p.x - dx, p.y - dy)) {
                    continue;
                }
                let hk = hash(p.x as f64 * 5.3 + dx as f64, p.y as f64 * 4.1 + dy as f64);
                if !(0.28..0.35).contains(&hk) {
                    continue;
                }
                let (ox, oz) = (dx as f32, dy as f32);
                let ring = Vec3::new(p.x as f32 + ox * 0.44, 0.06, p.y as f32 + oz * 0.44);
                cube(out, ring, Vec3::new(0.12, 0.06, 0.12), PI / 4.0, lin(CHAIN));
                let mut a = ring + Vec3::new(ox * 0.12, 0.0, oz * 0.12);
                let links = 8;
                for j in 0..links {
                    let t = (j + 1) as f32 / links as f32;
                    let swing = (hk as f32 * 9.0 + t * 3.0).sin() * 0.2 * t;
                    let b = Vec3::new(
                        p.x as f32 + ox * (0.64 + t * 0.2) + oz * swing,
                        0.02 - t * 1.5,
                        p.y as f32 + oz * (0.64 + t * 0.2) + ox * swing,
                    );
                    out.push(Instance::new(
                        forge::segment(a, b, if j % 2 == 0 { 0.055 } else { 0.035 }),
                        lin(if j % 2 == 0 { CHAIN } else { CHAIN_D }),
                        1.0,
                        Vec3::ZERO,
                        SHADE_LIT,
                    ));
                    a = b;
                }
                if hk < 0.315 {
                    // A skull hung from the end of it.
                    skull(out, a - Vec3::new(0.0, 0.1, 0.0), 0.9);
                }
            }
        }
        // Broken columns in the corners.
        for c in hub::columns(r) {
            self.build_column(c);
        }
    }

    /// A column of basalt drums on a plinth, red runes glowing on one; broken, or crowned with a fire bowl.
    fn build_column(&mut self, p: P) {
        let (x, z) = (p.x as f32, p.y as f32);
        let (hx, hy) = (p.x as f64, p.y as f64);
        let broken = hash(hx * 3.0, hy * 7.0) < 0.5;
        let start = self.static_cubes.len();
        let out = &mut self.static_cubes;
        cube(out, Vec3::new(x, 0.09, z), Vec3::new(0.84, 0.18, 0.84), 0.0, lin(BASALT[1]));
        cube(out, Vec3::new(x, 0.21, z), Vec3::new(0.7, 0.06, 0.7), 0.0, lin(BASALT[2]));
        let drums = if broken { 2 + (hash(hy, hx) * 2.0) as i32 } else { 6 };
        let mut top = 0.24;
        for i in 0..drums {
            let hk = hash(hx + i as f64 * 1.3, hy - i as f64) as f32;
            let c = stone(hk as f64, &BASALT);
            cube(out, Vec3::new(x, top + 0.16, z), Vec3::new(0.52, 0.31, 0.52), (hk - 0.5) * 0.25, c);
            if i == 1 {
                // A band of runes, glowing red.
                for f in 0..4 {
                    let a = f as f32 * PI / 2.0;
                    let n = Vec3::new(a.cos(), 0.0, a.sin());
                    glowing(
                        out,
                        Vec3::new(x, top + 0.16, z) + n * 0.265,
                        Vec3::new(0.2, 0.05, 0.02),
                        a + PI / 2.0,
                        lin(0x5a0a06),
                        lin(0xff2a0a) * 0.9,
                    );
                }
            }
            top += 0.32;
        }
        if broken {
            for k in 0..3 {
                let hk = hash(hx * 9.0 + k as f64, hy * 4.0) as f32;
                cube(
                    out,
                    Vec3::new(x + (hk - 0.5) * 0.3, top + 0.06 + hk * 0.06, z + (hash(hy, hx + k as f64) as f32 - 0.5) * 0.3),
                    Vec3::new(0.2, 0.12 + hk * 0.1, 0.2),
                    hk * 4.0,
                    lin(BASALT[0]),
                );
            }
            let a = hash(hx, hy) as f32 * TAU;
            cube(out, Vec3::new(x + a.cos() * 0.66, 0.15, z + a.sin() * 0.66), Vec3::new(0.5, 0.3, 0.3), a, lin(BASALT[2]));
        } else {
            cube(out, Vec3::new(x, top + 0.07, z), Vec3::new(0.72, 0.14, 0.72), 0.0, lin(BASALT[1]));
            top += 0.14;
            // An iron fire bowl, burning.
            cube(out, Vec3::new(x, top + 0.07, z), Vec3::new(0.42, 0.14, 0.42), PI / 4.0, lin(IRON));
            glowing(out, Vec3::new(x, top + 0.15, z), Vec3::new(0.3, 0.04, 0.3), 0.0, lin(EMBER), lin(EMBER) * 2.0);
            self.candles.push(Vec3::new(x, top + 0.5, z));
            self.smoke.push((Vec3::new(x, top + 0.35, z), 0.9));
        }
        let len = self.static_cubes.len() - start;
        self.tree_index.insert(tile_key(p.x, p.y), self.trees.len());
        self.trees.push(Tree {
            range: (start, len),
            base: Vec3::new(x, 0.0, z),
            radius: 0.4,
            height: top,
            fade: 1.0,
            alive: true,
            shades: None,
        });
    }

    /// A wooden bridge over the lava: planks (some charred, a few gone) on two stringers, posts and sagging rope rails, a lantern halfway.
    fn build_wood_bridge(&mut self, r: Rect) {
        let along = r.along_x();
        let (cx, cz) = r.center();
        let len = if along { (r.x1 - r.x0 + 1) as f32 } else { (r.y1 - r.y0 + 1) as f32 } + 0.7;
        let wide = if along { (r.y1 - r.y0 + 1) as f32 } else { (r.x1 - r.x0 + 1) as f32 } - 0.1;
        // Local frame: u along the bridge, v across it.
        let turn = if along { 0.0 } else { PI / 2.0 };
        let q = Quat::from_rotation_y(turn);
        let at = |u: f32, y: f32, v: f32| Vec3::new(cx, y, cz) + q * Vec3::new(u, 0.0, v);
        let sag = |u: f32| 0.06 * (PI * (u / len + 0.5)).cos().max(0.0);
        let seed = (r.x0 * 13 + r.y0 * 7) as f64;
        let out = &mut self.static_cubes;
        let n = (len / 0.21) as i32;
        let mut last_gone = false;
        for i in 0..n {
            let u = -len / 2.0 + (i as f32 + 0.5) * len / n as f32;
            let hk = hash(seed + i as f64 * 1.7, seed * 0.3);
            let gone = !last_gone && hk > 0.97 && i > 3 && i < n - 4;
            last_gone = gone;
            if gone {
                continue;
            }
            let color = if hk < 0.07 { lin(CHAR) } else { stone(hash(seed - i as f64, 2.0), &WOOD) };
            let jitter = (hash(seed + i as f64, 9.0) as f32 - 0.5) * 0.06;
            cube(out, at(u, -0.035 - sag(u), jitter * 0.5), Vec3::new(0.18, 0.07, wide + jitter), turn + jitter, color);
        }
        // Stringers under the deck, following the sag.
        let segs = 8;
        for side in [-1.0f32, 1.0] {
            for s in 0..segs {
                let (u0, u1) = (-len / 2.0 + s as f32 * len / segs as f32, -len / 2.0 + (s + 1) as f32 * len / segs as f32);
                let (a, b) = (at(u0, -0.14 - sag(u0), side * (wide / 2.0 - 0.25)), at(u1, -0.14 - sag(u1), side * (wide / 2.0 - 0.25)));
                out.push(Instance::new(forge::segment(a, b, 0.12), lin(WOOD_D), 1.0, Vec3::ZERO, SHADE_LIT));
            }
        }
        // Posts, and rope rails between them.
        let posts = [-len / 2.0 + 0.25, 0.0, len / 2.0 - 0.25];
        for side in [-1.0f32, 1.0] {
            let v = side * (wide / 2.0 + 0.02);
            for &u in &posts {
                out.push(Instance::new(forge::segment(at(u, -0.35, v), at(u, 0.78, v), 0.1), lin(WOOD_D), 1.0, Vec3::ZERO, SHADE_LIT));
                cube(out, at(u, 0.8, v), Vec3::new(0.13, 0.05, 0.13), turn, lin(WOOD[0]));
            }
            for pair in posts.windows(2) {
                for (top, dip) in [(0.72f32, 0.16f32), (0.38, 0.08)] {
                    let steps = 8;
                    let point = |k: i32| {
                        let t = k as f32 / steps as f32;
                        let u = pair[0] + (pair[1] - pair[0]) * t;
                        at(u, top - dip * (PI * t).sin() - sag(u) * 0.5, v)
                    };
                    for k in 0..steps {
                        out.push(Instance::new(forge::segment(point(k), point(k + 1), 0.035), lin(ROPE), 1.0, Vec3::ZERO, SHADE_LIT));
                    }
                }
            }
        }
        // A lantern hung on each middle post.
        for side in [-1.0f32, 1.0] {
            let p = at(0.0, 0.62, side * (wide / 2.0 + 0.2));
            cube(out, p, Vec3::new(0.16, 0.2, 0.16), turn, lin(IRON));
            glowing(out, p, Vec3::new(0.1, 0.14, 0.18), turn, lin(EMBER), lin(0xffa040) * 1.6);
            self.candles.push(p);
        }
    }

    /// The ledge he came in along, and the cliffs of the world above with the cave he came out of.
    fn build_ledge_and_cliffs(&mut self) {
        let l = hub::ledge();
        let out = &mut self.static_cubes;
        for p in l.tiles() {
            let (x, z) = (p.x as f32, p.y as f32);
            let hk = hash(p.x as f64 * 2.3, p.y as f64 * 1.9);
            cube(
                out,
                Vec3::new(x, -0.07 + (hk as f32 - 0.5) * 0.03, z),
                Vec3::new(0.97, 0.14, 0.97),
                hk as f32 * 0.2,
                stone(hk, &BASALT) * 1.1,
            );
        }
        let (cx, _) = l.center();
        let (w, len) = ((l.x1 - l.x0 + 1) as f32, (l.y1 - l.y0 + 1) as f32);
        let mid_z = (l.y0 + l.y1) as f32 / 2.0;
        cube(out, Vec3::new(cx, -0.7, mid_z), Vec3::new(w + 0.3, 1.2, len), 0.0, lin(BASALT[3]));
        for k in 0..10 {
            let a = hash(k as f64 * 3.1, 5.0) as f32;
            let b = hash(k as f64 * 1.7, 9.0) as f32;
            let long = 0.6 + a * 1.1;
            cube(
                out,
                Vec3::new(cx + (a - 0.5) * w, -1.3 - long / 2.0, l.y0 as f32 + b * len),
                Vec3::new(0.6 - a * 0.3, long, 0.6),
                a * TAU,
                lin(BASALT[0]) * 0.85,
            );
        }
        // Behind him the way back has fallen in: boulders heaped across the ledge's end.
        let end = l.y1 as f32 + 0.6;
        for k in 0..7 {
            let hk = hash(k as f64 * 5.1, 3.0) as f32;
            let hj = hash(k as f64 * 2.7, 8.0) as f32;
            let size = 0.6 + hk * 0.7;
            cube(
                out,
                Vec3::new(cx - 1.8 + k as f32 * 0.6, size * 0.4, end + hj * 0.6),
                Vec3::new(size, size * 0.9, size * 0.8),
                hk * 3.0,
                stone(hk as f64, &BASALT) * 1.1,
            );
        }
        // Beyond the rockfall the old path runs on to the cliffs of the world above.
        cube(out, Vec3::new(cx, -0.75, end + 3.0), Vec3::new(w + 0.4, 1.4, 5.0), 0.0, lin(BASALT[3]));
        for k in 0..6 {
            let z = end + 1.2 + k as f32 * 0.8;
            cube(
                out,
                Vec3::new(cx, -0.07, z),
                Vec3::new(w, 0.14, 0.75),
                (hash(k as f64, 4.0) as f32 - 0.5) * 0.1,
                stone(hash(k as f64, 6.0), &BASALT) * 1.1,
            );
        }
        // The cliff face: great blocks of rock rising out of the lava along the south, the cave in their middle.
        let face_z = hub::HUB_SIZE as f32 + 4.0;
        for k in 0..28 {
            let x = cx - 14.0 + k as f32 * 1.05;
            let hk = hash(k as f64 * 4.3, 1.0) as f32;
            let hj = hash(k as f64 * 2.1, 7.0) as f32;
            if (x - cx).abs() < 1.9 {
                continue; // the cave mouth
            }
            let tall = 5.5 + hk * 3.5 - ((x - cx).abs() / 14.0) * 2.5;
            cube(
                out,
                Vec3::new(x, LAVA_Y + tall / 2.0, face_z + 1.0 + hj * 0.6),
                Vec3::new(1.3, tall, 2.4),
                (hk - 0.5) * 0.3,
                stone(hk as f64, &BASALT),
            );
            cube(
                out,
                Vec3::new(x + (hj - 0.5) * 0.4, LAVA_Y + tall + 0.35, face_z + 1.3),
                Vec3::new(0.9, 0.7, 1.6),
                hj * 2.0,
                stone(hj as f64, &BASALT) * 1.1,
            );
        }
        // The cave: dark within, a lintel of rock over it, torches either side.
        cube(out, Vec3::new(cx, 1.2, face_z + 1.1), Vec3::new(3.6, 2.6, 0.2), 0.0, lin(VOID));
        cube(out, Vec3::new(cx, 3.0, face_z + 0.9), Vec3::new(4.4, 1.2, 1.8), 0.05, lin(BASALT[2]));
        for side in [-1.0f32, 1.0] {
            cube(out, Vec3::new(cx + side * 2.2, 1.0, face_z + 0.9), Vec3::new(1.0, 3.0, 1.8), side * 0.08, lin(BASALT[1]));
            let torch = Vec3::new(cx + side * 1.75, 1.35, face_z + 0.1);
            out.push(Instance::new(
                forge::segment(torch - Vec3::new(0.0, 0.45, 0.0), torch, 0.07),
                lin(WOOD_D),
                1.0,
                Vec3::ZERO,
                SHADE_LIT,
            ));
            self.candles.push(torch + Vec3::new(0.0, 0.1, 0.0));
            self.smoke.push((torch + Vec3::new(0.0, 0.2, 0.0), 0.5));
        }
    }

    /// The volcano in the middle, and the House of the Devil in its south face.
    fn build_volcano(&mut self) {
        let v = hub::volcano();
        let (cx, cz) = v.center();
        let (radius, crater, peak) = (8.2f32, 1.7f32, 10.5f32);
        let house = |x: f32, z: f32| (x - cx).abs() < 2.9 && z > cz + 2.3;
        let rivers = [-2.2f32, -0.9, 0.35, 2.75];
        let out = &mut self.static_cubes;
        let step = 0.5;
        let mut dz = -radius;
        while dz <= radius {
            let mut dx = -radius;
            while dx <= radius {
                let r = (dx * dx + dz * dz).sqrt();
                let wobble = (vnoise(dx * 0.7 + 3.0, dz * 0.7 + 9.0) - 0.5) * 0.9;
                let (x, z) = (cx + dx, cz + dz);
                if r <= radius + wobble && !house(x, z) {
                    let hk = hash(dx as f64 * 7.3, dz as f64 * 5.9) as f32;
                    let top = if r < crater {
                        peak - 1.2
                    } else {
                        let t = ((r - crater) / (radius - crater)).clamp(0.0, 1.0);
                        peak + (LAVA_Y - peak) * t.powf(0.8) + (hk - 0.5) * 0.35 + wobble * 0.3
                    };
                    let tall = top - LAVA_Y + 0.2;
                    let ashen = ((top - 3.5) / 3.5).clamp(0.0, 1.0);
                    let color = stone(hk as f64, &BASALT).lerp(lin(ASH), ashen * 0.6);
                    cube(out, Vec3::new(x, LAVA_Y - 0.2 + tall / 2.0, z), Vec3::new(0.52, tall, 0.52), 0.0, color);
                    // Lava rivers running down its flanks, and the crater's pool.
                    let a = dz.atan2(dx);
                    let on_river = r > crater + 0.3
                        && rivers.iter().any(|&ra| {
                            let da = (a - ra - 0.22 * (r * 0.9).sin()).rem_euclid(TAU);
                            da.min(TAU - da) < 0.11 + 0.1 * (r / radius)
                        });
                    if r < crater {
                        glowing(out, Vec3::new(x, top + 0.03, z), Vec3::new(0.5, 0.06, 0.5), 0.0, lin(LAVA_DEEP), lin(LAVA_HOT) * 2.2);
                    } else if on_river {
                        // Tall enough to bridge the step down to the next block, so the river runs unbroken.
                        glowing(
                            out,
                            Vec3::new(x, top - 0.14, z),
                            Vec3::new(0.52, 0.44, 0.52),
                            0.0,
                            lin(LAVA_DEEP),
                            lin(LAVA) * (1.4 + hk * 0.6),
                        );
                    }
                }
                dx += step;
            }
            dz += step;
        }
        self.smoke.push((Vec3::new(cx, peak + 0.2, cz), 2.6));
        self.smoke.push((Vec3::new(cx + 0.6, peak + 0.4, cz - 0.5), 1.8));
        self.smoke.push((Vec3::new(cx - 0.5, peak + 0.3, cz + 0.4), 1.4));
        self.build_house(cx, v.y1 as f32 + 0.45);
    }

    /// The House of the Devil: a black keep in the volcano's face, horned spires, red windows, an iron door under a skull.
    fn build_house(&mut self, cx: f32, face: f32) {
        let out = &mut self.static_cubes;
        let hall = |out: &mut Vec<Instance>, x: f32, y: f32, z: f32, sx: f32, sy: f32, sz: f32, c: u32| {
            cube(out, Vec3::new(x, y, z), Vec3::new(sx, sy, sz), 0.0, lin(c))
        };
        // Plinth and steps down to where the causeway will arrive.
        hall(out, cx, -0.2, face - 1.6, 6.2, 0.5, 3.8, BASALT[3]);
        for k in 0..3 {
            hall(out, cx, -0.04 - k as f32 * 0.12, face + 0.12 + k as f32 * 0.18, 2.6 - k as f32 * 0.2, 0.1, 0.2, BASALT[1]);
        }
        // The hall: black walls with pilasters, a stepped roof with spikes.
        hall(out, cx, 1.6, face - 1.6, 3.8, 3.2, 3.0, OBSIDIAN);
        for k in 0..5 {
            let x = cx - 1.8 + k as f32 * 0.9;
            hall(out, x, 1.6, face - 0.08, 0.22, 3.2, 0.18, OBSIDIAN_L);
        }
        for k in 0..4 {
            let w = 3.9 - k as f32 * 0.8;
            hall(out, cx, 3.35 + k as f32 * 0.3, face - 1.6, w, 0.3, 3.1 - k as f32 * 0.5, if k % 2 == 0 { OBSIDIAN_L } else { OBSIDIAN });
        }
        for k in 0..7 {
            let x = cx - 1.8 + k as f32 * 0.6;
            out.push(Instance::new(
                forge::segment(Vec3::new(x, 3.2, face - 0.1), Vec3::new(x, 3.75, face - 0.1), 0.07),
                lin(IRON),
                1.0,
                Vec3::ZERO,
                SHADE_LIT,
            ));
        }
        // Red windows.
        for (x, y) in [(cx - 1.35, 2.3), (cx + 1.35, 2.3), (cx - 1.35, 1.1), (cx + 1.35, 1.1)] {
            glowing(out, Vec3::new(x, y, face - 0.05), Vec3::new(0.16, 0.5, 0.06), 0.0, lin(0x3a0804), lin(WINDOW) * 2.0);
        }
        // Towers either side, with spires and great curved horns.
        for side in [-1.0f32, 1.0] {
            let tx = cx + side * 2.35;
            hall(out, tx, 2.5, face - 0.9, 1.15, 5.0, 1.15, OBSIDIAN);
            hall(out, tx, 5.05, face - 0.9, 1.35, 0.14, 1.35, OBSIDIAN_L);
            for k in 0..6 {
                let s = 1.0 - k as f32 * 0.16;
                cube(
                    out,
                    Vec3::new(tx, 5.3 + k as f32 * 0.26, face - 0.9),
                    Vec3::new(0.9 * s, 0.27, 0.9 * s),
                    PI / 4.0,
                    lin(if k % 2 == 0 { OBSIDIAN_L } else { OBSIDIAN }),
                );
            }
            glowing(out, Vec3::new(tx, 3.6, face - 0.3), Vec3::new(0.14, 0.6, 0.06), 0.0, lin(0x3a0804), lin(WINDOW) * 2.0);
            horn(out, Vec3::new(tx + side * 0.4, 4.9, face - 0.9), side, 1.0);
        }
        // The central spire behind, horned like its master.
        let (sx, sz) = (cx, face - 2.4);
        hall(out, sx, 4.9, sz, 1.4, 3.4, 1.4, OBSIDIAN);
        for k in 0..7 {
            let s = 1.0 - k as f32 * 0.13;
            cube(
                out,
                Vec3::new(sx, 6.7 + k as f32 * 0.3, sz),
                Vec3::new(1.1 * s, 0.31, 1.1 * s),
                PI / 4.0,
                lin(if k % 2 == 0 { OBSIDIAN_L } else { OBSIDIAN }),
            );
        }
        for side in [-1.0f32, 1.0] {
            horn(out, Vec3::new(sx + side * 0.55, 6.2, sz), side, 1.6);
        }
        glowing(out, Vec3::new(sx, 5.4, sz + 0.72), Vec3::new(0.5, 0.5, 0.04), PI / 4.0, lin(0x3a0804), lin(WINDOW) * 2.2);
        // The door: a pointed arch of red-glowing stone, iron leaves studded with spikes, a skull above.
        let door_c = Vec3::new(cx, 0.0, face);
        for side in [-1.0f32, 1.0] {
            glowing(out, door_c + Vec3::new(side * 0.72, 1.1, 0.02), Vec3::new(0.16, 2.2, 0.1), 0.0, lin(0x4a0c08), lin(WINDOW) * 0.6);
            for k in 0..5 {
                let t = k as f32 / 4.0;
                let a = t * 0.95;
                let p = Vec3::new(side * (0.95 * a.cos() - 0.23), 2.2 + 0.7 * a.sin(), 0.02);
                out.push(Instance::new(
                    Mat4::from_translation(door_c + p) * Mat4::from_rotation_z(-side * a) * Mat4::from_scale(Vec3::new(0.14, 0.12, 0.12)),
                    lin(0x4a0c08),
                    1.0,
                    lin(WINDOW) * 0.6,
                    SHADE_LIT,
                ));
            }
            cube(out, door_c + Vec3::new(side * 0.33, 1.1, -0.04), Vec3::new(0.64, 2.2, 0.08), 0.0, lin(IRON));
            for k in 0..4 {
                for j in 0..2 {
                    cube(
                        out,
                        door_c + Vec3::new(side * (0.18 + j as f32 * 0.28), 0.35 + k as f32 * 0.5, 0.02),
                        Vec3::splat(0.06),
                        PI / 4.0,
                        lin(IRON_L),
                    );
                }
            }
        }
        skull(out, door_c + Vec3::new(0.0, 2.75, 0.05), 1.4);
        for side in [-1.0f32, 1.0] {
            let b = door_c + Vec3::new(side * 1.25, 0.0, 0.4);
            cube(out, b + Vec3::new(0.0, 0.35, 0.0), Vec3::new(0.12, 0.7, 0.12), 0.0, lin(IRON));
            cube(out, b + Vec3::new(0.0, 0.72, 0.0), Vec3::new(0.34, 0.1, 0.34), PI / 4.0, lin(IRON));
            self.candles.push(b + Vec3::new(0.0, 0.95, 0.0));
            self.smoke.push((b + Vec3::new(0.0, 1.0, 0.0), 0.9));
        }
    }

    /// A portal gate's stonework: a black arch with bands of its sin's colour, its name in gold letters, a gem and horns over it, braziers before it.
    pub(super) fn build_portal_arch(&mut self, g: &Gate) {
        let root = gate_root(g);
        let color = lin(g.sin.color());
        let trim = color * 0.55;
        let glow = color * 0.25;
        let out = &mut self.static_cubes;
        put(out, root, Vec3::new(0.0, 0.04, 0.05), Vec3::new(1.5, 0.08, 1.0), lin(OBSIDIAN_L), Vec3::ZERO);
        put(out, root, Vec3::new(0.0, 0.08, 0.52), Vec3::new(1.4, 0.03, 0.06), trim, glow);
        // A ring of rune stones set in the tile before the doors, where he stands to go through.
        for k in 0..10 {
            let a = k as f32 / 10.0 * TAU;
            let m = root
                * Mat4::from_scale_rotation_translation(
                    Vec3::new(0.14, 0.02, 0.05),
                    Quat::from_rotation_y(-a - PI / 2.0),
                    Vec3::new(a.cos() * 0.4, 0.008, 1.0 + a.sin() * 0.4),
                );
            out.push(Instance::new(m, trim, 1.0, glow, SHADE_LIT));
        }
        // Dark behind the doors (set back, so the leaves swing clear of it).
        put(out, root, Vec3::new(0.0, 1.35, -0.78), Vec3::new(1.42, 2.7, 0.1), lin(VOID), Vec3::ZERO);
        for side in [-1.0f32, 1.0] {
            let x = side * 0.92;
            put(out, root, Vec3::new(x, 0.16, 0.0), Vec3::new(0.66, 0.32, 0.8), lin(OBSIDIAN_L), Vec3::ZERO);
            put(out, root, Vec3::new(x, 1.45, 0.0), Vec3::new(0.46, 2.3, 0.58), lin(OBSIDIAN), Vec3::ZERO);
            for y in [0.62f32, 2.35] {
                put(out, root, Vec3::new(x, y, 0.0), Vec3::new(0.5, 0.08, 0.62), trim, glow);
            }
            put(out, root, Vec3::new(x, 2.7, 0.0), Vec3::new(0.62, 0.2, 0.72), lin(OBSIDIAN_L), Vec3::ZERO);
            // A curved horn rising from each end of the lintel.
            let base = root.transform_point3(Vec3::new(side * 1.05, 3.1, 0.0));
            let mut p = base;
            for k in 0..6 {
                let t = (k + 1) as f32 / 6.0;
                let local = Vec3::new(side * (1.05 + 0.35 * t + 0.1 * t * t), 3.1 + 0.75 * t, -0.15 * t * t);
                let q = root.transform_point3(local);
                out.push(Instance::new(
                    forge::segment(p, q, 0.16 * (1.0 - t * 0.75)),
                    if k >= 4 { lin(HORN_D) } else { lin(HORN) },
                    1.0,
                    Vec3::ZERO,
                    SHADE_LIT,
                ));
                p = q;
            }
            // A brazier before the pillar.
            put(out, root, Vec3::new(side * 1.2, 0.36, 0.8), Vec3::new(0.12, 0.72, 0.12), lin(IRON), Vec3::ZERO);
            put(out, root, Vec3::new(side * 1.2, 0.76, 0.8), Vec3::new(0.38, 0.12, 0.38), lin(IRON), Vec3::ZERO);
        }
        put(out, root, Vec3::new(0.0, 2.95, 0.0), Vec3::new(2.4, 0.52, 0.6), lin(OBSIDIAN_L), Vec3::ZERO);
        put(out, root, Vec3::new(0.0, 2.68, 0.3), Vec3::new(2.3, 0.04, 0.02), trim, glow);
        put(out, root, Vec3::new(0.0, 3.22, 0.3), Vec3::new(2.3, 0.04, 0.02), trim, glow);
        for k in 0..4 {
            let w = 2.0 - k as f32 * 0.5;
            put(
                out,
                root,
                Vec3::new(0.0, 3.3 + k as f32 * 0.18, 0.0),
                Vec3::new(w, 0.18, 0.5),
                lin(if k % 2 == 0 { OBSIDIAN } else { OBSIDIAN_L }),
                Vec3::ZERO,
            );
        }
        let gem = root
            * Mat4::from_translation(Vec3::new(0.0, 4.15, 0.0))
            * Mat4::from_rotation_z(PI / 4.0)
            * Mat4::from_scale(Vec3::splat(0.24));
        out.push(Instance::new(gem, color, 1.0, color * 1.2, SHADE_LIT));
        // The sin's name in gold letters across the lintel.
        let name = &self.models.portal_names[g.sin.index()];
        let at = root * Mat4::from_translation(Vec3::new(0.0, 2.95, 0.31));
        for (off, c) in &name.cubes {
            out.push(Instance::new(voxel_matrix(&at, name.size, *off), *c, 1.0, *c * 0.25, SHADE_LIT));
        }
    }

    /// The Seal of the Seven: a round dais of black steps, seven socket stones in a ring round a horned seal
    /// stone with red runes, and a sin's gem lighting each socket once he has earned its key.
    fn build_altar(&mut self) {
        let p = hub::altar();
        let c = Vec3::new(p.x as f32, 0.0, p.y as f32);
        let out = &mut self.static_cubes;
        // Three round steps, each four bars turned through the eighth of a circle.
        for (k, (r, top, h)) in [(1.35f32, 0.04f32, 0.04f32), (0.95, 0.09, 0.05), (0.62, 0.17, 0.08)].iter().enumerate() {
            for s in 0..4 {
                let turn = s as f32 * PI / 4.0;
                cube(
                    out,
                    c + Vec3::new(0.0, top - h / 2.0, 0.0),
                    Vec3::new(r * 2.0, *h, r * 0.83),
                    turn,
                    lin(BASALT[1 + k % 2]) * (1.0 - k as f32 * 0.06),
                );
            }
        }
        // A ring of red runes inlaid in the second step, running socket to socket.
        for k in 0..16 {
            let a = k as f32 / 16.0 * TAU;
            let m = Mat4::from_scale_rotation_translation(
                Vec3::new(0.24, 0.012, 0.03),
                Quat::from_rotation_y(-a - PI / 2.0),
                c + Vec3::new(a.cos() * 0.8, 0.095, a.sin() * 0.8),
            );
            out.push(Instance::new(m, lin(0x2a0606), 1.0, lin(0xff2a0a) * 0.35, SHADE_LIT));
        }
        // The seven sockets: an obsidian block, an iron collar, and a dark hole where the gem will sit.
        for i in 0..SINS.len() {
            let a = i as f32 / 7.0 * TAU - PI / 2.0;
            let s = c + Vec3::new(a.cos() * SEAL_RING, 0.17, a.sin() * SEAL_RING);
            cube(out, s + Vec3::new(0.0, 0.07, 0.0), Vec3::new(0.15, 0.14, 0.15), a, lin(OBSIDIAN));
            cube(out, s + Vec3::new(0.0, 0.15, 0.0), Vec3::new(0.13, 0.02, 0.13), PI / 4.0, lin(IRON_L));
            cube(out, s + Vec3::new(0.0, 0.14, 0.0), Vec3::new(0.06, 0.03, 0.06), PI / 4.0, lin(0x0a0608));
        }
        // The seal stone: a black monolith with rune bands, a cap, and two horns.
        cube(out, c + Vec3::new(0.0, 0.72, 0.0), Vec3::new(0.5, 1.1, 0.5), 0.0, lin(OBSIDIAN));
        for y in [0.55f32, 0.95] {
            for f in 0..4 {
                let a = f as f32 * PI / 2.0;
                let n = Vec3::new(a.cos(), 0.0, a.sin());
                glowing(
                    out,
                    c + Vec3::new(0.0, y, 0.0) + n * 0.255,
                    Vec3::new(0.3, 0.05, 0.02),
                    a + PI / 2.0,
                    lin(0x5a0a06),
                    lin(0xff2a0a) * 0.9,
                );
            }
        }
        // The seal itself on the south face: seven dim gems round a red eye.
        for (i, sin) in SINS.iter().enumerate() {
            let a = i as f32 / 7.0 * TAU - PI / 2.0;
            let g = c + Vec3::new(a.cos() * 0.14, 0.75 + a.sin() * 0.14, 0.26);
            out.push(Instance::new(
                Mat4::from_scale_rotation_translation(Vec3::splat(0.035), Quat::from_rotation_z(PI / 4.0), g),
                lin(sin.color()) * 0.6,
                1.0,
                lin(sin.color()) * 0.35,
                SHADE_LIT,
            ));
        }
        glowing(out, c + Vec3::new(0.0, 0.75, 0.26), Vec3::new(0.06, 0.06, 0.02), PI / 4.0, lin(0x3a0604), lin(0xff2a0a) * 1.4);
        cube(out, c + Vec3::new(0.0, 1.32, 0.0), Vec3::new(0.64, 0.1, 0.64), 0.0, lin(OBSIDIAN_L));
        cube(out, c + Vec3::new(0.0, 1.5, 0.0), Vec3::new(0.4, 0.28, 0.4), 0.0, lin(OBSIDIAN));
        for side in [-1.0f32, 1.0] {
            horn(out, c + Vec3::new(side * 0.16, 1.6, 0.0), side, 0.55);
        }
    }

    // ------------------------------------------------------------------ each frame

    /// The hub's moving parts: the doors and their portals, the braziers, the seal's gems, the causeways and the fire wall,
    /// lava bubbling and falling, embers and ash in the air, and demons wheeling far off.
    pub(super) fn emit_hell(&self, s: &SceneState, time: f32, hero: Vec3, f: &mut Frame) {
        if self.gym {
            self.emit_lava_life(s.grid, time, hero, f);
            return;
        }
        let hv = s.hub.unwrap_or_default();
        self.emit_portals(&hv, time, hero, f);
        self.emit_seal(&hv, time, f);
        self.emit_causeways(&hv, time, f);
        self.emit_lava_life(s.grid, time, hero, f);
        self.emit_sky(time, f);
        // The crater's glow, and the House's windows.
        let (cx, cz) = hub::volcano().center();
        let flick = 1.0 + (time * 3.1).sin() * 0.15 + (time * 7.3).sin() * 0.08;
        f.lights.push(Light { pos: Vec3::new(cx, 11.5, cz), color: lin(LAVA) * 9.0 * flick, distance: 16.0 });
        glow_orb(f, Vec3::new(cx, 9.6, cz), 2.6, lin(LAVA), 0.35 * flick);
        f.lights.push(Light { pos: Vec3::new(cx, 2.5, hub::volcano().y1 as f32 + 1.4), color: lin(WINDOW) * 4.0, distance: 6.0 });
    }

    fn emit_portals(&self, hv: &HubView, time: f32, hero: Vec3, f: &mut Frame) {
        for g in hub::gates() {
            let i = g.sin.index();
            self.emit_portal(&g, hv.doors[i], hv.sins & g.sin.bit() != 0, time, hero, f);
        }
    }

    /// One portal: its doors `door` of the way open, the sin's key in the lock when `cleared`, the
    /// braziers, and the realm swirling beyond the open doors.
    pub(super) fn emit_portal(&self, g: &Gate, door: f32, cleared: bool, time: f32, hero: Vec3, f: &mut Frame) {
        let cube_i = MeshKind::Cube as usize;
        {
            let i = g.sin.index();
            let root = gate_root(g);
            let color = lin(g.sin.color());
            let door = door.clamp(0.0, 1.0);
            let angle = door * door * (3.0 - 2.0 * door) * 1.65;
            // The reliefs only where the camera can make them out (the tour flies it to each door).
            let spot = Vec3::new(g.tile.x as f32, 1.2, g.tile.y as f32);
            let near = spot.distance(self.cam_pos) < 30.0 || spot.distance(hero) < 12.0;
            // The leaves: a plate of dark iron each, the relief standing out of it.
            let leaves = &self.models.portal_doors[i];
            for (k, side) in [(0usize, -1.0f32), (1, 1.0)] {
                let hinge = Vec3::new(side * 0.62, 0.0, 0.02);
                let turn = if side < 0.0 { angle } else { -angle };
                let pm = root * Mat4::from_translation(hinge) * Mat4::from_rotation_y(turn);
                let plate = pm
                    * Mat4::from_scale_rotation_translation(Vec3::new(0.6, 2.4, 0.06), Quat::IDENTITY, Vec3::new(-side * 0.3, 1.2, -0.03));
                f.opaque[cube_i].push(Instance::new(plate, lin(IRON), 1.0, Vec3::ZERO, SHADE_LIT));
                if near {
                    let m = &leaves[k];
                    let shine = color * (0.05 + 0.25 * door);
                    for (off, c) in &m.cubes {
                        f.opaque[cube_i].push(Instance::new(voxel_matrix(&pm, m.size, *off), *c, 1.0, shine * 0.4, SHADE_LIT));
                    }
                }
            }
            // A cleared realm's key, set in the lock between the leaves.
            if cleared {
                let lock = root
                    * Mat4::from_translation(Vec3::new(0.0, 1.2, 0.1))
                    * Mat4::from_rotation_z(PI / 4.0)
                    * Mat4::from_scale(Vec3::splat(0.14));
                f.opaque[cube_i].push(Instance::new(lock, color, 1.0, color * 1.5, SHADE_LIT));
                glow_orb(f, root.transform_point3(Vec3::new(0.0, 1.2, 0.2)), 0.35, color, 0.4);
            }
            // The rune ring before the doors, glowing brighter as they open.
            let ring = root
                * Mat4::from_translation(Vec3::new(0.0, 0.03, 1.0))
                * Mat4::from_rotation_x(-PI / 2.0)
                * Mat4::from_scale(Vec3::splat(0.42 + 0.03 * (time * 3.0 + i as f32).sin()));
            f.transparent[MeshKind::Ring as usize].push(Instance::new(ring, color, 0.1 + 0.5 * door, Vec3::ZERO, SHADE_UNLIT));
            // The realm swirling beyond the open doors.
            if door > 0.0 {
                let centre = Vec3::new(0.0, 1.25, -0.3);
                let plane = root * Mat4::from_scale_rotation_translation(Vec3::new(1.2, 2.45, 0.02), Quat::IDENTITY, centre);
                f.additive.push(Instance::new(plane, color * 0.6, 0.35 * door, Vec3::ZERO, SHADE_UNLIT));
                for arm in 0..3 {
                    for k in 0..16 {
                        let t = k as f32 / 16.0;
                        let a = time * 2.2 + arm as f32 * TAU / 3.0 + t * 5.0;
                        let r = 0.05 + t * 0.52;
                        let p = centre + Vec3::new(a.cos() * r, a.sin() * r * 1.75, 0.02);
                        let c = Vec3::ONE.lerp(color, t);
                        f.additive.push(Instance::new(
                            root * Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.06 + 0.04 * (1.0 - t))),
                            c,
                            0.7 * door * (1.0 - t * 0.5),
                            Vec3::ZERO,
                            SHADE_UNLIT,
                        ));
                    }
                }
                glow_orb(f, root.transform_point3(centre + Vec3::new(0.0, 0.0, 0.3)), 1.1, color, 0.35 * door);
            }
            // Braziers burning in the sin's colour (calm gold once the realm is cleared).
            let fire = if cleared { lin(0xffc860) } else { color };
            for side in [-1.0f32, 1.0] {
                let b = root.transform_point3(Vec3::new(side * 1.2, 0.84, 0.8));
                for k in 0..7 {
                    let fk = k as f32 + side * 7.0 + i as f32 * 3.0;
                    let t = (time * 2.3 + fk * 0.37).fract();
                    let p = b + Vec3::new((fk * 1.9).sin() * 0.07, t * 0.42, (fk * 2.7).cos() * 0.07);
                    let size = 0.1 * (1.0 - t) + 0.02;
                    let c = fire * (1.1 - 0.6 * t);
                    f.additive.push(Instance::new(
                        Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(size, size * 1.5, size)),
                        c,
                        0.45 * (1.0 - t),
                        Vec3::ZERO,
                        SHADE_UNLIT,
                    ));
                }
                glow_orb(f, b + Vec3::new(0.0, 0.15, 0.0), 0.35, fire, 0.18);
            }
            if near {
                let flick = 3.0 + (time * 9.0 + i as f32).sin() * 0.4;
                f.lights.push(Light {
                    pos: root.transform_point3(Vec3::new(0.0, 1.6, 1.0)),
                    color: fire * flick * (1.0 + door * 1.5),
                    distance: 5.5,
                });
            }
        }
    }

    /// The seal's sockets: a gem glowing in each for the keys he holds.
    fn emit_seal(&self, hv: &HubView, time: f32, f: &mut Frame) {
        let p = hub::altar();
        let c = Vec3::new(p.x as f32, 0.36, p.y as f32);
        let mut held = 0;
        for (i, sin) in SINS.iter().enumerate() {
            if hv.sins & sin.bit() == 0 {
                continue;
            }
            held += 1;
            let a = i as f32 / 7.0 * TAU - PI / 2.0;
            let at = c + Vec3::new(a.cos() * SEAL_RING, 0.02 + (time * 2.0 + i as f32).sin() * 0.015, a.sin() * SEAL_RING);
            let color = lin(sin.color());
            let m = Mat4::from_translation(at)
                * Mat4::from_rotation_y(time + i as f32)
                * Mat4::from_rotation_z(PI / 4.0)
                * Mat4::from_scale(Vec3::splat(0.09));
            f.opaque[MeshKind::Cube as usize].push(Instance::new(m, color, 1.0, color * 1.6, SHADE_LIT));
            glow_orb(f, at, 0.18, color, 0.5);
        }
        if held > 0 {
            f.lights.push(Light { pos: c + Vec3::new(0.0, 0.6, 0.0), color: lin(0xffb070) * (1.0 + held as f32 * 0.5), distance: 4.0 });
        }
        if hv.sins & ALL_SINS == ALL_SINS {
            let beam = Mat4::from_translation(c + Vec3::new(0.0, 6.4, 0.0)) * Mat4::from_scale(Vec3::new(0.18, 10.0, 0.18));
            f.additive.push(Instance::new(beam, lin(0xffc070), 0.18 + (time * 2.0).sin() * 0.04, Vec3::ZERO, SHADE_UNLIT));
        }
    }

    /// The causeways rising out of the lava as the seal breaks, and the wall of fire round the volcano dying as they do.
    fn emit_causeways(&self, hv: &HubView, time: f32, f: &mut Frame) {
        let cube_i = MeshKind::Cube as usize;
        let seal = hv.seal.clamp(0.0, 1.0);
        if seal > 0.0 {
            for c in hub::causeways() {
                let along = c.along_x();
                let n = if along { c.x1 - c.x0 + 1 } else { c.y1 - c.y0 + 1 };
                let wide = if along { (c.y1 - c.y0 + 1) as f32 } else { (c.x1 - c.x0 + 1) as f32 };
                let (cx, cz) = c.center();
                for k in 0..n {
                    // Each block follows the one before it up, from the platform's end to the volcano's.
                    let d = k as f32 / n as f32 * 0.45;
                    let u = ((seal - d) / 0.55).clamp(0.0, 1.0);
                    let u = u * u * (3.0 - 2.0 * u);
                    if u <= 0.0 {
                        continue;
                    }
                    let top = (LAVA_Y - 1.0) * (1.0 - u);
                    let at = if along { Vec3::new(c.x0 as f32 + k as f32, 0.0, cz) } else { Vec3::new(cx, 0.0, c.y0 as f32 + k as f32) };
                    let size = if along { Vec3::new(0.98, 0.4, wide) } else { Vec3::new(wide, 0.4, 0.98) };
                    let hk = hash(at.x as f64 * 3.0, at.z as f64 * 5.0);
                    let hot = (1.0 - u) * 2.0;
                    f.opaque[cube_i].push(Instance::new(
                        Mat4::from_scale_rotation_translation(size, Quat::IDENTITY, Vec3::new(at.x, top - 0.2, at.z)),
                        stone(hk, &SLAB),
                        1.0,
                        lin(LAVA) * hot,
                        SHADE_LIT,
                    ));
                    let below = top - 0.4 - LAVA_Y + 0.2;
                    if below > 0.05 {
                        let pillar = if along { Vec3::new(0.9, below, wide * 0.7) } else { Vec3::new(wide * 0.7, below, 0.9) };
                        f.opaque[cube_i].push(Instance::new(
                            Mat4::from_scale_rotation_translation(
                                pillar,
                                Quat::IDENTITY,
                                Vec3::new(at.x, LAVA_Y - 0.2 + below / 2.0, at.z),
                            ),
                            stone(hk, &BASALT) * 0.8,
                            1.0,
                            lin(LAVA) * hot * 0.5,
                            SHADE_LIT,
                        ));
                    }
                    if u < 1.0 {
                        // Lava pouring off it as it comes up.
                        for j in 0..3 {
                            let tt = (time * 1.7 + j as f32 * 0.33 + hk as f32).fract();
                            let p = Vec3::new(at.x + (j as f32 - 1.0) * 0.4, top - 0.4 - tt * 1.2, at.z + (hk as f32 - 0.5) * 0.6);
                            f.additive.push(Instance::new(
                                Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(0.08, 0.3, 0.08)),
                                lin(LAVA_HOT),
                                0.8 * (1.0 - tt),
                                Vec3::ZERO,
                                SHADE_UNLIT,
                            ));
                        }
                    }
                }
            }
        }
        // The wall of fire round the volcano, until the causeways are up.
        let fade = 1.0 - seal;
        if fade <= 0.0 {
            return;
        }
        let v = hub::volcano();
        let (x0, z0, x1, z1) = (v.x0 as f32 - 1.6, v.y0 as f32 - 1.6, v.x1 as f32 + 1.6, v.y1 as f32 + 1.6);
        let mut spots = Vec::new();
        let mut t = x0;
        while t <= x1 {
            spots.push(Vec3::new(t, LAVA_Y, z0));
            spots.push(Vec3::new(t, LAVA_Y, z1));
            t += 0.75;
        }
        let mut t = z0 + 0.75;
        while t < z1 {
            spots.push(Vec3::new(x0, LAVA_Y, t));
            spots.push(Vec3::new(x1, LAVA_Y, t));
            t += 0.75;
        }
        for (k, p) in spots.iter().enumerate() {
            let fk = k as f32 * 1.37;
            // A bed of fire at the foot, and tongues of flame licking up out of it, each on its own clock.
            let base = *p + Vec3::new(0.0, 0.3, 0.0);
            let breathe = 0.8 + (time * 4.0 + fk).sin() * 0.2;
            f.additive.push(Instance::new(
                Mat4::from_translation(base) * Mat4::from_scale(Vec3::new(0.45, 0.35 * breathe, 0.45)),
                lin(LAVA_HOT).lerp(lin(0xff5a10), 0.5),
                0.14 * fade,
                Vec3::ZERO,
                SHADE_UNLIT,
            ));
            if k % 3 == 0 {
                glow_orb(f, base, 1.2, lin(LAVA), 0.12 * fade);
            }
            for j in 0..3 {
                let fj = j as f32;
                let speed = 1.3 + (fk * 0.9 + fj).sin().abs() * 0.9;
                let tt = (time * speed + fk * 0.21 + fj * 0.33).fract();
                let height = 1.2 + ((fk * 0.7 + fj * 1.7).sin() * 0.5 + 0.5) * 1.8;
                let rise = tt * height * fade;
                let flare = 1.0 + (time * 11.0 + fk + fj * 3.0).sin() * 0.25;
                let w = (0.28 * (1.0 - tt).powf(0.7) + 0.04) * fade.sqrt() * flare;
                let c = lin(LAVA_HOT).lerp(lin(0xc01808), tt * tt);
                let sway = Vec3::new((fk + fj * 2.0 + time * 2.5).sin() * 0.25 * tt, 0.0, (fk * 1.3 + time * 2.0 + fj).cos() * 0.25 * tt);
                let q = base + Vec3::new(0.0, rise, 0.0) + sway;
                f.additive.push(Instance::new(
                    Mat4::from_translation(q) * Mat4::from_scale(Vec3::new(w, w * 1.8, w)),
                    c,
                    0.3 * (1.0 - tt * tt) * fade,
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
        }
        let (cx, cz) = v.center();
        for (dx, dz) in [(0.0f32, -1.0f32), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0)] {
            f.lights.push(Light { pos: Vec3::new(cx + dx * 8.5, 0.5, cz + dz * 8.5), color: lin(LAVA) * 5.0 * fade, distance: 9.0 });
        }
    }

    /// Lava bubbling and glinting near him, lava falls off the platforms' edges, embers rising and ash falling.
    fn emit_lava_life(&self, grid: &Grid, time: f32, hero: Vec3, f: &mut Frame) {
        let lava_at = |x: f32, z: f32| {
            let (tx, tz) = (x.round() as i32, z.round() as i32);
            !grid.in_bounds(tx, tz) || grid.get(tx, tz) == Tile::Pit
        };
        let sphere = MeshKind::Sphere as usize;
        // Bubbles: each site swells a dome and pops, on its own clock.
        for k in 0..26 {
            let fk = k as f32;
            let cycle = 2.4 + (fk * 0.7).sin().abs() * 1.6;
            let epoch = ((time + fk * 0.37) / cycle).floor();
            let t = ((time + fk * 0.37) / cycle).fract();
            let x = hero.x + (hash(fk as f64, epoch as f64) as f32 - 0.5) * 30.0;
            let z = hero.z + (hash(epoch as f64 + 3.0, fk as f64) as f32 - 0.5) * 30.0;
            if !lava_at(x, z) {
                continue;
            }
            let p = Vec3::new(x, LAVA_Y, z);
            if t < 0.8 {
                let r = 0.08 + t * 0.3;
                let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(r, r * 0.7, r));
                f.opaque[sphere].push(Instance::new(m, lin(LAVA_DEEP), 1.0, lin(LAVA) * (1.2 + t), SHADE_LIT));
            } else {
                let k2 = (t - 0.8) / 0.2;
                let m = Mat4::from_translation(p + Vec3::new(0.0, 0.02, 0.0))
                    * Mat4::from_rotation_x(-PI / 2.0)
                    * Mat4::from_scale(Vec3::splat(0.2 + k2 * 0.5));
                f.transparent[MeshKind::Ring as usize].push(Instance::new(m, lin(LAVA_HOT), 0.8 * (1.0 - k2), Vec3::ZERO, SHADE_UNLIT));
                for j in 0..4 {
                    let a = j as f32 * TAU / 4.0 + fk;
                    let q = p + Vec3::new(a.cos() * k2 * 0.4, 0.3 * (PI * k2).sin(), a.sin() * k2 * 0.4);
                    f.additive.push(Instance::new(
                        Mat4::from_translation(q) * Mat4::from_scale(Vec3::splat(0.05)),
                        lin(LAVA_HOT),
                        1.0 - k2,
                        Vec3::ZERO,
                        SHADE_UNLIT,
                    ));
                }
            }
        }
        // Lava falls pouring off the platforms' outer edges (the hub's; the gym has none).
        for (i, (top, out)) in LAVA_FALLS.iter().enumerate().filter(|_| !self.gym) {
            let p = Vec3::new(top.0, -0.2, top.1);
            let dist = p.distance(hero);
            if dist > 40.0 {
                continue;
            }
            let o = Vec3::new(out.0, 0.0, out.1);
            let n = 10;
            for k in 0..n {
                let t = k as f32 / n as f32;
                let y = -0.3 + (LAVA_Y + 0.3) * t;
                let flow = ((time * 2.6 - t * 3.0 + i as f32).sin() * 0.5 + 0.5) * 0.6 + 0.6;
                let q = p + o * (0.1 + 0.45 * t * t) + Vec3::new(0.0, y + 0.3, 0.0);
                let m = Mat4::from_translation(q)
                    * Mat4::from_scale(Vec3::new(0.34 - t * 0.08, (LAVA_Y.abs() / n as f32) + 0.06, 0.34 - t * 0.08));
                f.opaque[MeshKind::Cube as usize].push(Instance::new(
                    m,
                    lin(LAVA_DEEP),
                    1.0,
                    lin(LAVA).lerp(lin(LAVA_HOT), flow - 0.6) * flow * 1.6,
                    SHADE_LIT,
                ));
            }
            let foot = p + o * 0.6 + Vec3::new(0.0, LAVA_Y + 0.3, 0.0);
            glow_orb(f, foot, 0.8, lin(LAVA_HOT), 0.35);
            if dist < 16.0 {
                f.lights.push(Light { pos: foot + Vec3::new(0.0, 0.8, 0.0), color: lin(LAVA) * 3.0, distance: 5.0 });
            }
        }
        // Embers rising from the lava all round him.
        for k in 0..150 {
            let fk = k as f32;
            let life = 3.5 + (fk * 1.3).sin().abs() * 2.5;
            let t = ((time + fk * 0.61) / life).fract();
            let epoch = ((time + fk * 0.61) / life).floor();
            let bx = hero.x + (hash(fk as f64 * 1.3, epoch as f64) as f32 - 0.5) * 34.0;
            let bz = hero.z + (hash(epoch as f64, fk as f64 * 2.1) as f32 - 0.5) * 34.0;
            let y = LAVA_Y + t * 9.0;
            let p = Vec3::new(bx + (time * 0.8 + fk).sin() * 0.4 * t, y, bz + (time * 0.6 + fk * 1.7).cos() * 0.4 * t);
            let a = (1.0 - t) * (0.6 + 0.4 * (time * 13.0 + fk).sin().abs());
            f.additive.push(Instance::new(
                Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.03 + 0.02 * (1.0 - t))),
                lin(if k % 3 == 0 { LAVA_HOT } else { EMBER }) * 1.6,
                a,
                Vec3::ZERO,
                SHADE_UNLIT,
            ));
        }
        // Ash drifting down.
        for k in 0..70 {
            let fk = k as f32;
            let t = ((time * 0.07 + fk * 0.113).fract()) * 1.0;
            let bx = hero.x + (hash(fk as f64 * 3.7, 1.0) as f32 - 0.5) * 30.0;
            let bz = hero.z + (hash(2.0, fk as f64 * 1.9) as f32 - 0.5) * 30.0;
            let p = Vec3::new(bx + (time * 0.4 + fk).sin() * 0.8, 9.0 - t * 11.0, bz + (time * 0.3 + fk).cos() * 0.8);
            let m = Mat4::from_translation(p)
                * Mat4::from_rotation_y(time + fk)
                * Mat4::from_rotation_x(time * 1.3 + fk)
                * Mat4::from_scale(Vec3::new(0.06, 0.01, 0.05));
            f.transparent[MeshKind::Cube as usize].push(Instance::new(m, lin(0x6a6260), 0.7, Vec3::ZERO, SHADE_LIT));
        }
    }

    /// Demons wheeling far off in the red sky.
    fn emit_sky(&self, time: f32, f: &mut Frame) {
        let (cx, cz) = hub::volcano().center();
        let cube_i = MeshKind::Cube as usize;
        for k in 0..6 {
            let fk = k as f32;
            let r = 34.0 + fk * 4.0;
            let a = time * (0.05 + 0.015 * fk) * if k % 2 == 0 { 1.0 } else { -1.0 } + fk * 1.9;
            let p = Vec3::new(cx + a.cos() * r, 11.0 + fk * 1.2 + (time * 0.7 + fk).sin() * 1.2, cz + a.sin() * r);
            let heading = if k % 2 == 0 { -a } else { PI - a };
            let flap = (time * (5.0 + fk * 0.3) + fk).sin() * 0.7;
            let body = Mat4::from_translation(p) * Mat4::from_rotation_y(heading) * Mat4::from_scale(Vec3::new(0.25, 0.22, 0.9));
            f.opaque[cube_i].push(Instance::new(body, lin(0x140808), 1.0, Vec3::ZERO, SHADE_LIT));
            for side in [-1.0f32, 1.0] {
                let wing = Mat4::from_translation(p)
                    * Mat4::from_rotation_y(heading)
                    * Mat4::from_rotation_z(side * flap)
                    * Mat4::from_translation(Vec3::new(side * 0.8, 0.0, 0.0))
                    * Mat4::from_scale(Vec3::new(1.5, 0.04, 0.7));
                f.opaque[cube_i].push(Instance::new(wing, lin(0x1c0a0a), 1.0, Vec3::ZERO, SHADE_LIT));
            }
            let eyes = Mat4::from_translation(p)
                * Mat4::from_rotation_y(heading)
                * Mat4::from_translation(Vec3::new(0.0, 0.05, 0.48))
                * Mat4::from_scale(Vec3::new(0.2, 0.05, 0.05));
            f.opaque[cube_i].push(Instance::new(eyes, lin(0xff2a0a), 1.0, lin(0xff2a0a) * 3.0, SHADE_LIT));
        }
    }
}

/// Where lava pours off the platforms: a point on an outer edge, and the way out from it.
const LAVA_FALLS: [((f32, f32), (f32, f32)); 10] = [
    ((8.0, 5.45), (0.0, -1.0)),
    ((16.0, 5.45), (0.0, -1.0)),
    ((35.5, 5.45), (0.0, -1.0)),
    ((56.0, 5.45), (0.0, -1.0)),
    ((5.45, 28.0), (-1.0, 0.0)),
    ((5.45, 48.0), (-1.0, 0.0)),
    ((58.55, 36.0), (1.0, 0.0)),
    ((58.55, 56.0), (1.0, 0.0)),
    ((9.0, 58.55), (0.0, 1.0)),
    ((54.0, 58.55), (0.0, 1.0)),
];

/// Bones lying about: a skull, a pair of long bones, or a ribcage.
fn bones(out: &mut Vec<Instance>, at: Vec3, h: f64) {
    let a = h as f32 * TAU;
    let q = Quat::from_rotation_y(a);
    let kind = (h * 7.0) as i32 % 3;
    match kind {
        0 => skull(out, at + Vec3::new(0.0, 0.08, 0.0), 0.8),
        1 => {
            for (off, turn) in [(Vec3::new(0.0, 0.03, 0.0), 0.0f32), (Vec3::new(0.05, 0.06, 0.08), 0.7)] {
                let r = Quat::from_rotation_y(a + turn);
                let p = at + q * off;
                out.push(Instance::new(
                    Mat4::from_scale_rotation_translation(Vec3::new(0.36, 0.04, 0.045), r, p),
                    lin(BONE),
                    1.0,
                    Vec3::ZERO,
                    SHADE_LIT,
                ));
                for s in [-1.0f32, 1.0] {
                    out.push(Instance::new(
                        Mat4::from_scale_rotation_translation(Vec3::new(0.06, 0.06, 0.08), r, p + r * Vec3::new(s * 0.18, 0.0, 0.0)),
                        lin(BONE_D),
                        1.0,
                        Vec3::ZERO,
                        SHADE_LIT,
                    ));
                }
            }
        }
        _ => {
            out.push(Instance::new(
                Mat4::from_scale_rotation_translation(Vec3::new(0.05, 0.05, 0.42), q, at + Vec3::new(0.0, 0.03, 0.0)),
                lin(BONE_D),
                1.0,
                Vec3::ZERO,
                SHADE_LIT,
            ));
            for k in 0..4 {
                let z = -0.14 + k as f32 * 0.09;
                for s in [-1.0f32, 1.0] {
                    let p = at + q * Vec3::new(s * 0.1, 0.07, z);
                    let r = q * Quat::from_rotation_z(s * 0.9);
                    out.push(Instance::new(
                        Mat4::from_scale_rotation_translation(Vec3::new(0.18, 0.025, 0.03), r, p),
                        lin(BONE),
                        1.0,
                        Vec3::ZERO,
                        SHADE_LIT,
                    ));
                }
            }
        }
    }
}

/// A skull, `size` times the ordinary.
fn skull(out: &mut Vec<Instance>, at: Vec3, size: f32) {
    let s = size;
    out.push(Instance::new(
        Mat4::from_scale_rotation_translation(Vec3::new(0.16, 0.14, 0.18) * s, Quat::IDENTITY, at + Vec3::new(0.0, 0.03, 0.0) * s),
        lin(BONE),
        1.0,
        Vec3::ZERO,
        SHADE_LIT,
    ));
    out.push(Instance::new(
        Mat4::from_scale_rotation_translation(Vec3::new(0.11, 0.05, 0.08) * s, Quat::IDENTITY, at + Vec3::new(0.0, -0.05, 0.07) * s),
        lin(BONE_D),
        1.0,
        Vec3::ZERO,
        SHADE_LIT,
    ));
    for side in [-1.0f32, 1.0] {
        out.push(Instance::new(
            Mat4::from_scale_rotation_translation(
                Vec3::new(0.045, 0.04, 0.02) * s,
                Quat::IDENTITY,
                at + Vec3::new(side * 0.04, 0.04, 0.09) * s,
            ),
            lin(0x1a1414),
            1.0,
            Vec3::ZERO,
            SHADE_LIT,
        ));
    }
}

/// A great curved horn rising from `base`, sweeping out to `side` (-1 or 1).
fn horn(out: &mut Vec<Instance>, base: Vec3, side: f32, size: f32) {
    let mut p = base;
    for k in 0..8 {
        let t = (k + 1) as f32 / 8.0;
        let q = base + Vec3::new(side * (0.55 * t + 0.25 * t * t) * size, (0.9 * t - 0.2 * t * t * t) * size, -0.1 * t * size);
        let thick = 0.26 * size * (1.0 - t * 0.8);
        out.push(Instance::new(forge::segment(p, q, thick), if k >= 6 { lin(HORN_D) } else { lin(HORN) }, 1.0, Vec3::ZERO, SHADE_LIT));
        p = q;
    }
}

// ------------------------------------------------------------ the doors' reliefs

/// A picture in relief: each cell of the canvas how far it stands out (1 to 3) and its colour.
struct Relief {
    px: Vec<Option<(u8, u32)>>,
}

impl Relief {
    fn new() -> Self {
        Self { px: vec![None; (DOOR_W * DOOR_H) as usize] }
    }

    fn set(&mut self, x: i32, y: i32, d: u8, c: u32) {
        if x < 0 || y < 0 || x >= DOOR_W || y >= DOOR_H {
            return;
        }
        let i = (y * DOOR_W + x) as usize;
        if self.px[i].is_none_or(|(cd, _)| d >= cd) {
            self.px[i] = Some((d, c));
        }
    }

    /// Every cell whose centre passes `inside`.
    fn fill(&mut self, d: u8, c: u32, inside: impl Fn(f32, f32) -> bool) {
        for y in 0..DOOR_H {
            for x in 0..DOOR_W {
                if inside(x as f32 + 0.5, y as f32 + 0.5) {
                    self.set(x, y, d, c);
                }
            }
        }
    }

    fn disc(&mut self, cx: f32, cy: f32, r: f32, d: u8, c: u32) {
        self.fill(d, c, |x, y| (x - cx).powi(2) + (y - cy).powi(2) <= r * r);
    }

    fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, d: u8, c: u32) {
        self.fill(d, c, |x, y| ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2) <= 1.0);
    }

    fn ring(&mut self, cx: f32, cy: f32, r0: f32, r1: f32, d: u8, c: u32) {
        self.fill(d, c, |x, y| {
            let r = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
            r >= r0 && r <= r1
        });
    }

    fn rect(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, d: u8, c: u32) {
        self.fill(d, c, |x, y| x >= x0 && x <= x1 && y >= y0 && y <= y1);
    }

    /// A thick line.
    fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, th: f32, d: u8, c: u32) {
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len2 = (dx * dx + dy * dy).max(1e-6);
        self.fill(d, c, |x, y| {
            let t = (((x - x0) * dx + (y - y0) * dy) / len2).clamp(0.0, 1.0);
            let (px, py) = (x0 + dx * t, y0 + dy * t);
            (x - px).powi(2) + (y - py).powi(2) <= (th / 2.0).powi(2)
        });
    }

    fn tri(&mut self, a: (f32, f32), b: (f32, f32), c3: (f32, f32), d: u8, c: u32) {
        let area = |p: (f32, f32), q: (f32, f32), r: (f32, f32)| (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0);
        self.fill(d, c, |x, y| {
            let p = (x, y);
            let (e1, e2, e3) = (area(a, b, p), area(b, c3, p), area(c3, a, p));
            (e1 >= 0.0 && e2 >= 0.0 && e3 >= 0.0) || (e1 <= 0.0 && e2 <= 0.0 && e3 <= 0.0)
        });
    }

    /// Discs of radius `r` along a path, alternating two colours: a serpent's body.
    fn chain(&mut self, points: &[(f32, f32)], r: f32, d: u8, c: [u32; 2]) {
        for (k, &(x, y)) in points.iter().enumerate() {
            self.disc(x, y, r, d, c[(k / 2) % 2]);
        }
    }
}

/// The frame every door shares: a raised border with gold rivets, and a boss at the top of each leaf.
fn door_frame(r: &mut Relief) {
    let edge = IRON_L;
    r.rect(0.0, 0.0, DOOR_W as f32, 2.0, 1, edge);
    r.rect(0.0, DOOR_H as f32 - 2.0, DOOR_W as f32, DOOR_H as f32, 1, edge);
    r.rect(0.0, 0.0, 2.0, DOOR_H as f32, 1, edge);
    r.rect(DOOR_W as f32 - 2.0, 0.0, DOOR_W as f32, DOOR_H as f32, 1, edge);
    r.rect(18.6, 0.0, 21.4, DOOR_H as f32, 1, edge);
    for k in 0..13 {
        let y = 3.5 + k as f32 * 6.0;
        for x in [1.0f32, 20.0, 39.0] {
            r.disc(x, y, 0.8, 2, 0xb8903a);
        }
    }
}

/// The two leaves of a sin's portal door, their relief standing out of the plate. Each mesh's
/// offsets are from its hinge (the left leaf's at its left edge, the right leaf's at its right).
pub fn portal_doors(sin: Sin) -> [Mesh; 2] {
    let mut r = Relief::new();
    door_frame(&mut r);
    emblem(&mut r, sin);
    let mut left = Vec::new();
    let mut right = Vec::new();
    let depth = |x: i32, y: i32| {
        if x < 0 || y < 0 || x >= DOOR_W || y >= DOOR_H {
            0
        } else {
            r.px[(y * DOOR_W + x) as usize].map_or(0, |(d, _)| d)
        }
    };
    for y in 0..DOOR_H {
        for x in 0..DOOR_W {
            let Some((d, c)) = r.px[(y * DOOR_W + x) as usize] else { continue };
            let up = (DOOR_H - 1 - y) as f32 + 0.5;
            let color = lin(c);
            // A layer shows if it is the front one, or a neighbour stands lower beside it.
            let low = depth(x - 1, y).min(depth(x + 1, y)).min(depth(x, y - 1)).min(depth(x, y + 1));
            for layer in 0..d {
                if layer + 1 < d && layer >= low {
                    continue;
                }
                let z = (layer as f32 + 0.5) * DOOR_VOXEL;
                if x < DOOR_W / 2 {
                    left.push((Vec3::new((x as f32 + 0.5) * DOOR_VOXEL, up * DOOR_VOXEL, z), color * (0.9 + 0.1 * layer as f32)));
                } else {
                    right.push((
                        Vec3::new(-((DOOR_W - x) as f32 - 0.5) * DOOR_VOXEL, up * DOOR_VOXEL, z),
                        color * (0.9 + 0.1 * layer as f32),
                    ));
                }
            }
        }
    }
    [Mesh { size: DOOR_VOXEL, cubes: left }, Mesh { size: DOOR_VOXEL, cubes: right }]
}

/// Each sin in relief, on a canvas 40 wide and 80 tall (y downward).
fn emblem(r: &mut Relief, sin: Sin) {
    let cx = DOOR_W as f32 / 2.0;
    match sin {
        Sin::Pride => {
            // A peacock fanning its tail beneath a crown.
            let (v, vd, gold, teal, blue) = (0x9a5aff, 0x5a2ab0, 0xf2c53d, 0x2ad0c0, 0x3a4aff);
            let (fx, fy) = (cx, 60.0);
            r.fill(1, vd, |x, y| y < fy && (x - fx).powi(2) + (y - fy).powi(2) <= 27.0f32.powi(2));
            for k in 0..11 {
                let a = PI + (k as f32 + 0.5) / 11.0 * PI;
                let (ex, ey) = (fx + a.cos() * 23.5, fy + a.sin() * 23.5);
                r.line(fx, fy, ex, ey, 1.2, 2, v);
                r.disc(ex, ey, 3.3, 2, gold);
                r.disc(ex, ey, 2.2, 3, teal);
                r.disc(ex, ey, 1.0, 3, blue);
            }
            r.ellipse(cx, 62.0, 3.6, 7.0, 3, blue);
            r.line(cx, 56.0, cx, 46.5, 2.8, 3, blue);
            r.disc(cx, 45.5, 2.5, 3, blue);
            r.tri((cx + 1.8, 45.0), (cx + 4.5, 46.0), (cx + 1.8, 47.0), 3, gold);
            for k in -1..=1 {
                let x = cx + k as f32 * 1.4;
                r.line(cx, 43.5, x, 39.5, 0.6, 3, gold);
                r.disc(x, 39.3, 0.8, 3, gold);
            }
            r.line(cx - 1.5, 68.0, cx - 2.5, 74.0, 1.0, 2, gold);
            r.line(cx + 1.5, 68.0, cx + 2.5, 74.0, 1.0, 2, gold);
            r.rect(cx - 8.0, 12.0, cx + 8.0, 15.0, 2, gold);
            for k in 0..5 {
                let x = cx - 8.0 + k as f32 * 4.0;
                r.tri((x - 1.8, 12.5), (x + 1.8, 12.5), (x, 5.5 + (k % 2) as f32 * 2.0), 2, gold);
                r.disc(x, 5.8 + (k % 2) as f32 * 2.0, 0.9, 3, v);
            }
            r.disc(cx, 13.5, 1.0, 3, 0xff3a5a);
        }
        Sin::Envy => {
            // A great slit-pupilled eye in the coils of a serpent, lesser eyes watching from the corners.
            let (g, gd, white, black) = (0x3ad06a, 0x1e7a3a, 0xd8e0b8, 0x0c100c);
            let lens = |x: f32, y: f32, s: f32| {
                (x - cx).powi(2) + (y - (40.0 - 10.0 * s)).powi(2) <= (15.0 * s).powi(2)
                    && (x - cx).powi(2) + (y - (40.0 + 10.0 * s)).powi(2) <= (15.0 * s).powi(2)
            };
            r.fill(2, gd, |x, y| lens(x, y, 1.0));
            r.fill(2, white, |x, y| lens(x, y, 0.85));
            r.disc(cx, 40.0, 5.4, 3, g);
            r.ellipse(cx, 40.0, 1.3, 4.6, 3, black);
            r.disc(cx - 2.0, 38.0, 0.9, 3, 0xffffff);
            let mut body = Vec::new();
            for k in 0..70 {
                let t = k as f32 / 69.0;
                let a = PI * 0.5 + t * TAU * 0.92;
                let wob = (t * 30.0).sin() * 1.2;
                body.push((cx + a.cos() * (15.5 + wob), 40.0 + a.sin() * (21.0 + wob)));
            }
            r.chain(&body, 2.3, 2, [g, gd]);
            let (hx, hy) = body[0];
            r.tri((hx - 3.2, hy - 1.0), (hx + 3.2, hy - 1.0), (hx, hy + 4.5), 3, g);
            r.disc(hx, hy - 1.5, 2.8, 3, g);
            r.disc(hx - 1.3, hy - 2.0, 0.6, 3, 0xff3a2a);
            r.disc(hx + 1.3, hy - 2.0, 0.6, 3, 0xff3a2a);
            r.line(hx, hy + 4.0, hx, hy + 7.0, 0.5, 3, 0xff3a2a);
            r.line(hx, hy + 7.0, hx - 1.2, hy + 8.5, 0.5, 3, 0xff3a2a);
            r.line(hx, hy + 7.0, hx + 1.2, hy + 8.5, 0.5, 3, 0xff3a2a);
            for (ex, ey) in [(8.0f32, 8.0f32), (32.0, 8.0), (8.0, 72.0), (32.0, 72.0)] {
                r.ellipse(ex, ey, 3.6, 1.8, 2, white);
                r.disc(ex, ey, 1.4, 3, g);
                r.ellipse(ex, ey, 0.4, 1.2, 3, black);
            }
        }
        Sin::Wrath => {
            // A snarling horned face in flames, over crossed axes.
            let (red, rd, orange, yellow, horn_c, steel, wood) = (0xff3a2a, 0x7a1408, 0xff8a2a, 0xffd060, 0xd8c8a8, 0xa8acb4, 0x6a4428);
            for s in [-1.0f32, 1.0] {
                r.line(cx - s * 14.0, 72.0, cx + s * 11.0, 22.0, 1.8, 1, wood);
                let (ax, ay) = (cx + s * 11.0, 24.0);
                r.fill(2, steel, |x, y| {
                    (x - ax).powi(2) + (y - ay).powi(2) <= 36.0
                        && (x - (ax - s * 3.5)).powi(2) + (y - ay).powi(2) > 16.0
                        && (x - ax) * s > -1.0
                });
            }
            for k in 0..11 {
                let a = PI * 0.95 + k as f32 / 10.0 * PI * 1.1;
                let (bx, by) = (cx + a.cos() * 10.0, 40.0 + a.sin() * 10.0);
                let (tx, ty) = (cx + a.cos() * (19.0 + (k % 2) as f32 * 3.0), 40.0 + a.sin() * (19.0 + (k % 2) as f32 * 3.0));
                let n = (-a.sin(), a.cos());
                r.tri(
                    (bx - n.0 * 3.0, by - n.1 * 3.0),
                    (bx + n.0 * 3.0, by + n.1 * 3.0),
                    (tx, ty),
                    1,
                    if k % 2 == 0 { orange } else { yellow },
                );
                r.tri((bx - n.0 * 1.5, by - n.1 * 1.5), (bx + n.0 * 1.5, by + n.1 * 1.5), ((bx + tx) / 2.0, (by + ty) / 2.0), 2, red);
            }
            r.disc(cx, 41.0, 9.5, 2, rd);
            r.disc(cx, 42.0, 8.0, 2, red);
            for s in [-1.0f32, 1.0] {
                let mut prev = (cx + s * 7.0, 34.0);
                for k in 1..=7 {
                    let t = k as f32 / 7.0;
                    let p = (cx + s * (7.0 + 6.0 * t + 1.5 * t * t), 34.0 - 16.0 * t + 4.0 * t * t * t);
                    r.line(prev.0, prev.1, p.0, p.1, 3.4 * (1.0 - t * 0.7), 3, if k > 5 { 0x3a3028 } else { horn_c });
                    prev = p;
                }
                r.tri((cx + s * 1.0, 38.5), (cx + s * 6.5, 36.0), (cx + s * 5.5, 40.5), 3, yellow);
            }
            r.line(cx - 7.0, 35.0, cx, 38.5, 1.6, 3, 0x2a0604);
            r.line(cx + 7.0, 35.0, cx, 38.5, 1.6, 3, 0x2a0604);
            r.ellipse(cx, 47.5, 5.2, 2.6, 3, 0x1a0404);
            for k in 0..4 {
                let x = cx - 3.6 + k as f32 * 2.4;
                r.tri((x - 0.8, 45.2), (x + 0.8, 45.2), (x, 47.4), 3, 0xf0e8d8);
                r.tri((x - 0.6, 49.8), (x + 0.6, 49.8), (x, 48.2), 3, 0xf0e8d8);
            }
        }
        Sin::Sloth => {
            // A snail on a stopped hourglass, hung with cobwebs.
            let (blue, wood, sand, web, shell, shell_d, body) = (0x7a9ac8, 0x6a4428, 0xd8b870, 0xc8c8d4, 0xa0784a, 0x6a4a2a, 0x8aa070);
            for (x0, sx) in [(2.0f32, 1.0f32), (38.0, -1.0)] {
                for k in 0..5 {
                    let a = k as f32 / 4.0 * PI / 2.0;
                    r.line(x0, 2.0, x0 + sx * a.cos() * 13.0, 2.0 + a.sin() * 13.0, 0.5, 1, web);
                }
                for rr in [4.5f32, 8.0, 11.5] {
                    for k in 0..4 {
                        let (a0, a1) = (k as f32 / 4.0 * PI / 2.0, (k + 1) as f32 / 4.0 * PI / 2.0);
                        r.line(x0 + sx * a0.cos() * rr, 2.0 + a0.sin() * rr, x0 + sx * a1.cos() * rr, 2.0 + a1.sin() * rr, 0.5, 1, web);
                    }
                }
            }
            r.rect(10.0, 16.0, 30.0, 19.0, 2, wood);
            r.rect(10.0, 54.0, 30.0, 57.0, 2, wood);
            r.line(12.0, 19.0, 12.0, 54.0, 1.4, 2, wood);
            r.line(28.0, 19.0, 28.0, 54.0, 1.4, 2, wood);
            r.tri((14.0, 19.5), (26.0, 19.5), (cx, 36.0), 1, blue);
            r.tri((14.0, 53.5), (26.0, 53.5), (cx, 36.0), 1, blue);
            r.tri((15.5, 53.5), (24.5, 53.5), (cx, 46.5), 2, sand);
            r.rect(16.0, 19.5, 24.0, 21.0, 2, sand);
            r.disc(25.0, 67.0, 6.0, 2, shell);
            for k in 0..28 {
                let t = k as f32 / 27.0;
                let a = t * TAU * 1.6;
                let rr = 5.2 * (1.0 - t);
                r.disc(25.0 + a.cos() * rr, 67.0 + a.sin() * rr, 0.6, 3, shell_d);
            }
            r.ellipse(15.0, 72.0, 9.0, 2.2, 2, body);
            r.line(8.0, 71.0, 6.5, 64.5, 0.7, 2, body);
            r.line(10.0, 71.0, 10.0, 64.0, 0.7, 2, body);
            r.disc(6.5, 64.3, 0.9, 3, 0x2a2a2a);
            r.disc(10.0, 63.8, 0.9, 3, 0x2a2a2a);
            r.line(30.0, 30.0, 33.0, 27.0, 0.6, 2, web);
            r.line(33.0, 27.0, 30.5, 24.0, 0.6, 2, web);
        }
        Sin::Greed => {
            // Gold spilling from a chest into a grasping hand.
            let (gold, gold_d, wood, iron, claw, claw_d) = (0xf2c53d, 0xb8862a, 0x6a4428, 0x3a3a3e, 0x6a5a6a, 0x3a2a3a);
            r.rect(8.0, 61.0, 32.0, 75.0, 2, wood);
            r.rect(7.0, 53.0, 33.0, 58.0, 1, wood);
            for x in [11.0f32, 29.0] {
                r.rect(x - 0.8, 61.0, x + 0.8, 75.0, 3, iron);
                r.rect(x - 0.8, 53.0, x + 0.8, 58.0, 2, iron);
            }
            r.rect(18.0, 64.0, 22.0, 68.0, 3, gold);
            r.disc(cx, 66.0, 0.8, 3, iron);
            r.ellipse(cx, 60.0, 11.0, 3.0, 3, gold);
            let mut k = 0;
            for row in 0..7 {
                let n = 2 + row / 2;
                for j in 0..n {
                    k += 1;
                    let spread = (row as f32 + 1.0) * 1.7;
                    let x = cx + (j as f32 - (n as f32 - 1.0) / 2.0) * 4.2 + ((k * 7) % 5) as f32 * 0.3;
                    let y = 55.0 - row as f32 * 3.6 - (x - cx).abs() * 0.1;
                    let _ = spread;
                    r.disc(x, y, 1.9, 3, gold);
                    r.ring(x, y, 1.2, 1.9, 3, gold_d);
                }
            }
            r.line(cx, 0.0, cx, 12.0, 6.0, 2, claw);
            r.ellipse(cx, 16.0, 6.0, 4.5, 2, claw);
            for (k, dx) in [-4.5f32, -1.5, 1.5, 4.5].iter().enumerate() {
                let (x0, y0) = (cx + dx, 19.0);
                let (x1, y1) = (cx + dx * 1.35, 25.0);
                let (x2, y2) = (cx + dx * 1.1, 29.0 + (k % 2) as f32);
                r.line(x0, y0, x1, y1, 2.0, 3, claw);
                r.line(x1, y1, x2, y2, 1.6, 3, claw);
                r.tri((x2 - 0.8, y2), (x2 + 0.8, y2), (x2 - dx * 0.1, y2 + 2.2), 3, claw_d);
                if k == 1 {
                    r.rect(x0 - 1.1, y0 + 1.5, x0 + 1.1, y0 + 2.3, 3, gold);
                }
            }
            r.line(cx - 6.0, 15.0, cx - 9.0, 21.0, 2.0, 3, claw);
            r.tri((cx - 9.8, 21.0), (cx - 8.2, 21.0), (cx - 9.3, 23.0), 3, claw_d);
        }
        Sin::Gluttony => {
            // A fanged maw swallowing a feast.
            let (lips, dark, fang, tongue, meat, bone, grape, gold, apple, leaf) =
                (0xa8301a, 0x1a0806, 0xe8e0c8, 0xd0506a, 0xc07a3a, 0xe8dcc0, 0x7a3a9a, 0xf2c53d, 0xd82a2a, 0x3a8a3a);
            // The lips a rim round the dark of the mouth.
            r.fill(2, lips, |x, y| {
                ((x - cx) / 16.0).powi(2) + ((y - 52.0) / 14.0).powi(2) <= 1.0
                    && ((x - cx) / 12.5).powi(2) + ((y - 52.0) / 10.5).powi(2) > 1.0
            });
            r.ellipse(cx, 52.0, 12.5, 10.5, 1, dark);
            r.ellipse(cx, 58.0, 6.5, 3.2, 2, tongue);
            for k in 0..6 {
                let x = cx - 10.0 + k as f32 * 4.0;
                let long = if k % 2 == 0 { 5.5 } else { 3.5 };
                r.tri((x - 1.4, 42.5), (x + 1.4, 42.5), (x, 42.5 + long), 3, fang);
                r.tri((x - 1.2, 61.8), (x + 1.2, 61.8), (x, 61.8 - long * 0.7), 3, fang);
            }
            r.line(8.0, 24.0, 15.0, 17.0, 2.2, 2, bone);
            r.disc(7.5, 24.8, 1.4, 2, bone);
            r.ellipse(17.0, 15.5, 4.5, 3.4, 3, meat);
            r.rect(27.0, 9.0, 32.0, 10.0, 2, gold);
            r.tri((26.0, 10.0), (33.0, 10.0), (29.5, 16.0), 3, gold);
            r.line(29.5, 16.0, 29.5, 20.0, 0.9, 3, gold);
            r.rect(27.5, 20.0, 31.5, 21.0, 3, gold);
            for (k, (gx, gy)) in
                [(0.0f32, 0.0f32), (-1.6, 1.8), (1.6, 1.8), (0.0, 3.6), (-1.6, 3.8), (1.6, 3.8), (0.0, 5.6)].iter().enumerate()
            {
                r.disc(cx + gx, 30.0 + gy, 1.25, 3, if k % 2 == 0 { grape } else { 0x5a2a7a });
            }
            r.line(cx, 29.0, cx + 1.0, 27.0, 0.6, 3, leaf);
            r.disc(11.0, 34.0, 2.6, 3, apple);
            r.ellipse(12.5, 31.0, 1.3, 0.6, 3, leaf);
            r.disc(30.0, 32.0, 1.0, 2, 0xd07a3a);
            r.disc(28.0, 35.0, 0.8, 2, 0xd07a3a);
        }
        Sin::Lust => {
            // Two serpents twined into a heart, a rose in it, thorned roses below.
            let (rose, crimson, serp, belly, leaf, gold) = (0xff4a9a, 0xc01840, 0xd83a6a, 0xf0b060, 0x2a7a3a, 0xf2c53d);
            let heart = |t: f32| {
                let x = 16.0 * t.sin().powi(3);
                let y = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
                (cx + x * 0.95, 40.0 - y * 0.95)
            };
            let left: Vec<(f32, f32)> = (0..60).map(|k| heart(-PI + 0.12 + k as f32 / 59.0 * (PI - 0.3))).collect();
            let right: Vec<(f32, f32)> = (0..60).map(|k| heart(PI - 0.12 - k as f32 / 59.0 * (PI - 0.3))).collect();
            r.chain(&left, 2.2, 2, [serp, belly]);
            r.chain(&right, 2.2, 2, [serp, belly]);
            for (path, s) in [(&left, -1.0f32), (&right, 1.0)] {
                let (hx, hy) = path[path.len() - 1];
                r.disc(hx, hy, 2.8, 3, serp);
                r.disc(hx + s * 0.8, hy - 0.8, 0.6, 3, gold);
                r.line(hx - s * 1.8, hy + 0.2, hx - s * 3.5, hy + 0.8, 0.5, 3, 0xff2a2a);
            }
            for k in 0..5 {
                let rr = 5.0 - k as f32 * 1.0;
                r.disc(cx + (k as f32 * 0.7).sin() * 0.6, 41.0, rr, 3, if k % 2 == 0 { crimson } else { rose });
            }
            for (x, flip) in [(7.0f32, 1.0f32), (33.0, -1.0)] {
                r.line(x, 76.0, x + flip * 2.0, 62.0, 0.9, 2, leaf);
                for k in 0..4 {
                    let y = 74.0 - k as f32 * 3.0;
                    r.tri((x + flip * 0.4, y), (x + flip * 0.4, y - 1.0), (x + flip * 1.8, y - 1.2), 2, leaf);
                }
                r.ellipse(x - flip * 1.8, 68.0, 2.2, 1.0, 2, leaf);
                r.disc(x + flip * 2.0, 60.5, 3.4, 3, crimson);
                r.disc(x + flip * 2.0, 60.5, 2.0, 3, rose);
                r.disc(x + flip * 2.0, 60.5, 0.8, 3, crimson);
            }
        }
    }
}

// ------------------------------------------------------------ the names

/// A 5x7 letter, row by row from the top.
fn glyph(c: char) -> Option<[&'static str; 7]> {
    Some(match c {
        'A' => [" ### ", "#   #", "#   #", "#####", "#   #", "#   #", "#   #"],
        'D' => ["#### ", "#   #", "#   #", "#   #", "#   #", "#   #", "#### "],
        'E' => ["#####", "#    ", "#    ", "#### ", "#    ", "#    ", "#####"],
        'G' => [" ####", "#    ", "#    ", "# ###", "#   #", "#   #", " ### "],
        'H' => ["#   #", "#   #", "#   #", "#####", "#   #", "#   #", "#   #"],
        'I' => ["#####", "  #  ", "  #  ", "  #  ", "  #  ", "  #  ", "#####"],
        'L' => ["#    ", "#    ", "#    ", "#    ", "#    ", "#    ", "#####"],
        'N' => ["#   #", "##  #", "# # #", "#  ##", "#   #", "#   #", "#   #"],
        'O' => [" ### ", "#   #", "#   #", "#   #", "#   #", "#   #", " ### "],
        'P' => ["#### ", "#   #", "#   #", "#### ", "#    ", "#    ", "#    "],
        'R' => ["#### ", "#   #", "#   #", "#### ", "# #  ", "#  # ", "#   #"],
        'S' => [" ####", "#    ", "#    ", " ### ", "    #", "    #", "#### "],
        'T' => ["#####", "  #  ", "  #  ", "  #  ", "  #  ", "  #  ", "  #  "],
        'U' => ["#   #", "#   #", "#   #", "#   #", "#   #", "#   #", " ### "],
        'V' => ["#   #", "#   #", "#   #", "#   #", "#   #", " # # ", "  #  "],
        'W' => ["#   #", "#   #", "#   #", "# # #", "# # #", "## ##", "#   #"],
        'Y' => ["#   #", "#   #", " # # ", "  #  ", "  #  ", "  #  ", "  #  "],
        _ => return None,
    })
}

/// A word in raised gold letters, centred on the origin, two voxels deep toward +z.
pub fn carved_name(word: &str) -> Mesh {
    let letters: Vec<[&str; 7]> = word.chars().filter_map(glyph).collect();
    let width = letters.len() as f32 * 6.0 - 1.0;
    let mut cubes = Vec::new();
    for (i, g) in letters.iter().enumerate() {
        for (row, line) in g.iter().enumerate() {
            for (col, ch) in line.chars().enumerate() {
                if ch != '#' {
                    continue;
                }
                let x = (i as f32 * 6.0 + col as f32 + 0.5 - width / 2.0) * LETTER_VOXEL;
                let y = (3.0 - row as f32) * LETTER_VOXEL;
                for layer in 0..2 {
                    let shade = if layer == 1 { 1.0 } else { 0.8 };
                    cubes.push((Vec3::new(x, y, (layer as f32 + 0.5) * LETTER_VOXEL), lin(GOLD) * shade));
                }
            }
        }
    }
    Mesh { size: LETTER_VOXEL, cubes }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sin_can_be_spelled_and_every_door_carries_its_relief() {
        for sin in SINS {
            assert!(sin.title().chars().all(|c| glyph(c).is_some()), "a letter of {} is missing", sin.title());
            let name = carved_name(sin.title());
            assert!(!name.cubes.is_empty());
            let [l, r] = portal_doors(sin);
            assert!(l.cubes.len() > 300 && r.cubes.len() > 300, "{sin:?}: {} + {} voxels", l.cubes.len(), r.cubes.len());
            assert!(l.cubes.len() + r.cubes.len() < 9000, "{sin:?} too heavy: {}", l.cubes.len() + r.cubes.len());
            // The leaves stay within their halves of the doorway.
            assert!(l.cubes.iter().all(|(p, _)| p.x >= 0.0 && p.x <= 0.61));
            assert!(r.cubes.iter().all(|(p, _)| p.x <= 0.0 && p.x >= -0.61));
        }
    }

    #[test]
    fn the_lava_glows_in_rivers_between_dark_crust() {
        let mut hot = 0;
        let mut crust = 0;
        for z in 0..40 {
            for x in 0..40 {
                let (_, e) = lava_look(x as f32 * 1.3, z as f32 * 1.3);
                if e.x > 0.6 {
                    hot += 1;
                } else if e.x < 0.1 {
                    crust += 1;
                }
            }
        }
        assert!(hot > 100 && crust > 100, "hot {hot}, crust {crust}");
    }
}
