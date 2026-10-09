//! The Portals of Hell at the Cisterns' bar (RENDERING.md, Stage B, "The Portals of Hell, at the
//! Cisterns' bar"): the hub keeps its own character (eight platforms floating over a lava sea
//! round a volcano, rope bridges between them, seven portals each burning in its sin's colour)
//! and gets the Cisterns' depth and craft.
//!
//! - **The lava** is one sheet in its own material (`SHADE_LAVA`, `lava_at` in the shader): plates
//!   of dark crust drifting, cracked, the heat glowing in the cracks, melting into open amber flow
//!   where it runs hot; on the volcano's flanks and in the falls it runs downhill.
//! - **The platforms** are paved: dark polished basalt in a basket weave, pale dressed stone in a
//!   cross along the ways the bridges come in, a kerb of chunky pale and umber blocks round the
//!   edge, worn, chipped and here and there gone, heat glowing up through cracked flags near the
//!   edges. Their sides are the crypt's `masonry` in courses, with veins of heat in the joints,
//!   and jagged basalt hangs beneath, lit from the lava below.
//! - **The bridges** are old timber and hemp: planks on stringers and transoms, scorched from
//!   below and at their ends, posts with iron bands, two sagging rope rails with hangers down to
//!   the deck, and a copper lantern on each.
//! - **The light**: the lava from below (the sheet's glow in the probes, and lights rising past the
//!   platforms' edges and under the bridges), the portals' braziers in their sins' colours, and the
//!   fire bowls and lanterns as amber pools; a dim smoky light from above.
//!
//! The layout is untouched: every tile, column, gate, bridge and causeway is where it was.

use super::crypt::{
    block, block_m, bone_cubes, brazier_cubes, jit, lantern_cubes, masonry, Wall, SHADE_FLAG, SHADE_IRON, SHADE_SOFT, SHADE_STONE,
    SHADE_WOOD,
};
use super::hell::{glowing, vnoise, LAVA_Y};
use super::*;
use crate::logic::hub::{self, Platform, Rect};

pub(super) mod art {
    /// Basalt, warm charcoal, polished where feet have worn it; its darkest for what hangs beneath.
    pub const BASALT: [u32; 3] = [0x4a4541, 0x403b38, 0x534d48];
    pub const BASALT_D: u32 = 0x2c2826;
    /// Umber and pale dressed stone.
    pub const UMBER: [u32; 2] = [0x5e4a3c, 0x523f33];
    pub const PALE: [u32; 2] = [0x8e8676, 0x7e7666];
    pub const GROUT: u32 = 0x120d0a;
    /// Old timber, scorched; hemp rope.
    pub const TIMBER: [u32; 3] = [0x5a4232, 0x4c382a, 0x664a36];
    pub const SCORCH: u32 = 0x221812;
    pub const HEMP: u32 = 0x7a6446;
    pub const IRON: u32 = 0x2a2624;
    pub const CHAIN: u32 = 0x4a4440;
    /// The lava's crust, and its glow: ember in the cracks, amber where it runs.
    pub const CRUST: u32 = 0x2e1e16;
    pub const EMBER: u32 = 0xff7a20;
    pub const AMBER: u32 = 0xffa040;
    pub const LAVA_GLOW: u32 = 0xff6a18;
}
use art::*;

/// Two basalt flags laid as one block (wet flagstone, `voxel_surface` kind 14, the shader drawing
/// the joint across the block's middle and each half its own shade).
pub(super) const SHADE_PAIRED: f32 = 19.25;
/// Lava (`voxel_surface` kind 12): its crust and glow from the flow's pattern.
pub(super) const SHADE_LAVA: f32 = 17.0;
/// Polished stone (`voxel_surface` kind 10): the seal's obsidian.
pub(super) const SHADE_GLOSS: f32 = 15.0;
/// Ivory and horn (`voxel_surface` kind 6).
pub(super) const SHADE_HORN: f32 = 11.0;
/// Gold and copper (`voxel_surface` kind 8).
pub(super) const SHADE_GOLD: f32 = 13.0;

/// The lava's mean glow, as a sheet of it gives it to the probes (the shader's pattern averages
/// about this).
pub(super) fn lava_glow(k: f32) -> Vec3 {
    lin(LAVA_GLOW) * k
}

/// How many of the lava's lights (rising past the edges, under the bridges) burn at once, the
/// nearest the hero first.
pub(super) const HUB_GLOW_LIT: usize = 6;
/// The hub's fire bowls and lanterns lit at once.
pub(super) const HUB_LAMPS_LIT: usize = 5;

/// Pushes `local` (built about the origin, facing +z) placed by `root`.
pub(super) fn placed(out: &mut Vec<Instance>, root: Mat4, local: &[Instance]) {
    for c in local {
        let m = root * Mat4::from_cols_array(&c.m);
        out.push(Instance { m: m.to_cols_array(), ..*c });
    }
}

/// Turns what was built in the range `from..` plain-lit into stone: everything not glowing and not
/// black (the cave's dark, the portals' void) takes the dressed-stone material, warmed a little.
pub(super) fn as_stone(out: &mut [Instance], warm: f32) {
    for c in out.iter_mut() {
        let lit = c.emissive[3] == SHADE_LIT;
        let glows = c.emissive[0] + c.emissive[1] + c.emissive[2] > 0.0;
        let black = c.color[0] + c.color[1] + c.color[2] < 0.01;
        if lit && !glows && !black {
            c.emissive[3] = SHADE_STONE;
            c.color[0] *= warm * 1.04;
            c.color[1] *= warm;
            c.color[2] *= warm * 0.96;
        }
    }
}

/// Holds a brazier's built flames to an amber glow, so they burn without blowing out to white
/// (the light is its pool's, above them).
pub(super) fn tame_flames(out: &mut [Instance]) {
    for c in out.iter_mut() {
        if c.emissive[3] == SHADE_LIT && c.emissive[0] > 1.5 {
            for k in 0..3 {
                c.emissive[k] *= 0.4;
            }
        }
    }
}

/// Recolours the verdigris `masonry` lays (a damp crypt's) to umber: nothing is green in hell.
fn no_verdigris(out: &mut [Instance]) {
    for c in out.iter_mut() {
        if c.color[1] > c.color[0] * 1.02 {
            let u = lin(UMBER[0]) * (0.9 + 0.2 * (c.color[2] * 37.0).fract());
            c.color[0] = u.x;
            c.color[1] = u.y;
            c.color[2] = u.z;
        }
    }
}

impl WorldView {
    /// The hub's static geometry at the Cisterns' bar (the gym keeps the old builders).
    pub(super) fn build_hub_art(&mut self, grid: &Grid) {
        let start = self.static_cubes.len();
        self.hub_lava_sea();
        for pl in hub::platforms() {
            self.hub_platform(grid, pl);
        }
        for (i, b) in hub::bridges().into_iter().enumerate() {
            self.hub_bridge(b, i);
        }
        let from = self.static_cubes.len();
        self.build_ledge_and_cliffs();
        as_stone(&mut self.static_cubes[from..], 1.1);
        // (The far spires stay plain-lit: dim shapes in the fog, they would only pay for the
        // stone's tooling.)
        self.build_far_rocks();
        self.hub_volcano();
        for g in hub::gates() {
            self.hub_portal(&g);
        }
        self.hub_altar();
        // Built in the board's palette already: the grade leaves it alone.
        self.authored.push((start, self.static_cubes.len() - start));
    }

    /// The lava: one sheet under everything, out to the fog (the shader draws its flow).
    fn hub_lava_sea(&mut self) {
        let c = (hub::HUB_SIZE - 1) as f32 / 2.0;
        let m = Mat4::from_scale_rotation_translation(Vec3::new(180.0, 0.1, 180.0), Quat::IDENTITY, Vec3::new(c, LAVA_Y - 0.05, c));
        self.static_cubes.push(Instance::new(m, lin(CRUST), 1.0, lava_glow(0.35), SHADE_LAVA));
    }

    /// A platform: paved, kerbed, its sides in courses, jagged rock beneath; bones, rubble and
    /// chains; broken columns in its corners.
    fn hub_platform(&mut self, grid: &Grid, pl: Platform) {
        let r = pl.rect;
        let (cx, cz) = r.center();
        let (w, h) = ((r.x1 - r.x0 + 1) as f32, (r.y1 - r.y0 + 1) as f32);
        let (mx, mz) = ((r.x0 + r.x1) / 2, (r.y0 + r.y1) / 2);
        let seed = (r.x0 * 31 + r.y0 * 17) as f64;
        let open_edge = |x: i32, y: i32| grid.get(x, y) == Tile::Pit;
        let out = &mut self.static_cubes;
        // The joints' dark under the whole paving, in one piece.
        block(out, Vec3::new(cx, -0.075, cz), Vec3::new(w, 0.13, h), 0.0, lin(GROUT), SHADE_LIT);
        for p in r.tiles() {
            let (x, z) = (p.x as f32, p.y as f32);
            let (hx, hy) = (p.x as f64, p.y as f64);
            let edge = p.x == r.x0 || p.x == r.x1 || p.y == r.y0 || p.y == r.y1;
            let ring = p.x == r.x0 + 1 || p.x == r.x1 - 1 || p.y == r.y0 + 1 || p.y == r.y1 - 1;
            let path = (p.x - mx).abs() <= 1 || (p.y - mz).abs() <= 1;
            if path && !edge {
                // The processional way: pale dressed flags, two to a tile across the way, worn.
                let along_x = (p.y - mz).abs() <= 1 && (p.x - mx).abs() > 1;
                for (k, s) in [-1.0f32, 1.0].into_iter().enumerate() {
                    let hk = hash(hx * 3.3 + k as f64 * 1.7, hy * 2.9 + 4.0);
                    let off = if along_x { Vec3::new(s * 0.245, 0.0, 0.0) } else { Vec3::new(0.0, 0.0, s * 0.245) };
                    let size = if along_x { Vec3::new(0.465, 0.12, 0.965) } else { Vec3::new(0.965, 0.12, 0.465) };
                    let worn = 0.92 + 0.08 * hk as f32;
                    let lift = (hash(hy + k as f64, hx * 1.3) as f32 - 0.5) * 0.014;
                    block(
                        out,
                        Vec3::new(x, -0.062 + lift, z) + off,
                        size,
                        (hk as f32 - 0.5) * 0.02,
                        jit(PALE[(hk * 2.0) as usize % 2], hk, 0.07) * worn * 0.82,
                        SHADE_STONE,
                    );
                }
                continue;
            }
            // The basalt field: two polished flags to a tile, turned tile by tile (a basket weave).
            // Most tiles are one block whose material draws the joint between its two flags
            // (`SHADE_PAIRED`): two blocks a tile cost an eighth of the frame. Where a flag has
            // cracked or been chipped, the tile is laid as two real flags.
            let along_x = (p.x + p.y) % 2 == 0;
            let scorched = if ring { 0.82 } else { 1.0 };
            let worked = hash(hx * 5.1, hy * 3.3) < if ring || edge { 0.16 } else { 0.06 };
            if !worked {
                let hk = hash(hx * 2.1, hy * 1.7 + 9.0);
                let lift = (hash(hy * 1.9, hx) as f32 - 0.5) * 0.012;
                let turn = if along_x { PI / 2.0 } else { 0.0 } + (hk as f32 - 0.5) * 0.02;
                let colour = jit(BASALT[(hk * 3.0) as usize % 3], hk, 0.12) * scorched;
                block(out, Vec3::new(x, -0.062 + lift, z), Vec3::new(0.965, 0.12, 0.965), turn, colour, SHADE_PAIRED);
            }
            for (k, s) in [-1.0f32, 1.0].into_iter().enumerate().filter(|_| worked) {
                let hk = hash(hx * 2.1 + k as f64 * 3.7, hy * 1.7 + 9.0);
                let off = if along_x { Vec3::new(0.0, 0.0, s * 0.245) } else { Vec3::new(s * 0.245, 0.0, 0.0) };
                // One chipped short at one end, showing the dark joint; or one cracked across,
                // the heat glowing up through the crack.
                let cracked = k == 0 && hash(hx * 7.3, hy * 2.9) < 0.6;
                let chip = if !cracked && k == 1 { 0.06 + hk as f32 * 0.1 } else { 0.0 };
                let shift = if (hk * 31.0).fract() < 0.5 { chip / 2.0 } else { -chip / 2.0 };
                let (size, sh) = if along_x {
                    (Vec3::new(0.965 - chip, 0.12, 0.465), Vec3::new(shift, 0.0, 0.0))
                } else {
                    (Vec3::new(0.465, 0.12, 0.965 - chip), Vec3::new(0.0, 0.0, shift))
                };
                let lift = (hash(hy * 1.9 + k as f64, hx) as f32 - 0.5) * 0.016;
                let colour = jit(BASALT[(hk * 3.0) as usize % 3], hk, 0.12) * scorched;
                let at = Vec3::new(x, -0.062 + lift, z) + off + sh;
                if cracked {
                    let a = if along_x { Vec3::new(0.245, 0.0, 0.0) } else { Vec3::new(0.0, 0.0, 0.245) };
                    let half = if along_x { Vec3::new(0.46, 0.12, 0.465) } else { Vec3::new(0.465, 0.12, 0.46) };
                    block(out, at - a, half, 0.03, colour, SHADE_FLAG);
                    block(out, at + a + Vec3::Y * 0.006, half, -0.025, colour * 0.92, SHADE_FLAG);
                    let seam = if along_x { Vec3::new(0.035, 0.02, 0.44) } else { Vec3::new(0.44, 0.02, 0.035) };
                    glowing(out, Vec3::new(at.x, -0.035, at.z), seam, 0.0, lin(0x3a1206), lin(EMBER) * 1.3);
                } else {
                    block(out, at, size, (hk as f32 - 0.5) * 0.025, colour, SHADE_FLAG);
                }
            }
            if !edge {
                continue;
            }
            // The kerb: chunky pale and umber blocks along the outer edge, worn and chipped, now
            // and then fallen out; where the edge looks over the lava, a stub of parapet.
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                if r.contains(P::new(p.x + dx, p.y + dy)) {
                    continue;
                }
                let hk = hash(hx * 5.3 + dx as f64 * 2.0, hy * 4.1 + dy as f64 * 3.0);
                let lava = open_edge(p.x + dx, p.y + dy) && grid.get(p.x, p.y) == Tile::Open;
                if hk < 0.06 {
                    // Fallen out: the heat shows in the joint where it stood.
                    let o = Vec3::new(dx as f32 * 0.32, -0.03, dy as f32 * 0.32);
                    let s = if dx != 0 { Vec3::new(0.03, 0.02, 0.7) } else { Vec3::new(0.7, 0.02, 0.03) };
                    glowing(out, Vec3::new(x, 0.0, z) + o, s, 0.0, lin(0x3a1206), lin(EMBER) * 1.1);
                    continue;
                }
                // (A stub of parapet stands only in the edge's outer fifth, clear of where he walks.)
                let stub = lava && hk > 0.62;
                let tall = if stub { 0.2 + (hk as f32 - 0.62) * 0.8 } else { 0.17 };
                let deep = if stub { 0.2 } else { 0.4 };
                let long = 0.97 - if (hk * 17.0).fract() < 0.3 { 0.12 } else { 0.0 };
                let o = Vec3::new(dx as f32 * (0.5 - deep / 2.0), tall / 2.0 - 0.13, dy as f32 * (0.5 - deep / 2.0));
                let size = if dx != 0 { Vec3::new(deep, tall, long) } else { Vec3::new(long, tall, deep) };
                let c = if (hk * 7.0).fract() < 0.6 { jit(PALE[(hk * 4.0) as usize % 2], hk, 0.08) * 0.8 } else { jit(UMBER[0], hk, 0.1) };
                block(out, Vec3::new(x, 0.0, z) + o, size, (hk as f32 - 0.5) * 0.06, c, SHADE_STONE);
                if tall > 0.25 && (hk * 23.0).fract() < 0.5 {
                    // The parapet's broken top: a chip of it lying beside.
                    let q = Vec3::new(x + dy as f32 * 0.3 + dx as f32 * 0.32, 0.05, z + dx as f32 * 0.3 + dy as f32 * 0.32);
                    block(out, q, Vec3::new(0.14, 0.1, 0.12), hk as f32 * 5.0, c * 0.9, SHADE_STONE);
                }
            }
        }
        // The sides: dressed stone in three courses under the kerb, on a pale plinth.
        for (k, dir) in [(1, 0), (-1, 0), (0, 1), (0, -1)].into_iter().enumerate() {
            let from = out.len();
            let half = if dir.0 != 0 { h / 2.0 } else { w / 2.0 };
            masonry(
                out,
                &Wall {
                    centre: Vec3::new(cx, 0.0, cz),
                    dir,
                    half,
                    base: -1.02,
                    height: 0.9,
                    courses: 2,
                    seed: (seed + k as f64 * 7.0, seed * 0.3 + k as f64),
                    len: (0.85, 0.6),
                    depth: (0.14, 0.12),
                    face: if dir.0 != 0 { w / 2.0 } else { h / 2.0 } - 0.1,
                    wet: false,
                    ruined: false,
                    foot: None,
                    openings: &[],
                },
            );
            no_verdigris(&mut out[from..]);
        }
        // The core, and the body tapering underneath in rough basalt (plain-lit: seen from below,
        // and far, in the lava's light, the stone's tooling would not show).
        block(out, Vec3::new(cx, -0.6, cz), Vec3::new(w - 0.1, 0.96, h - 0.1), 0.0, lin(BASALT_D), SHADE_LIT);
        for k in 1..=3 {
            let inset = 1.3 * k as f32;
            let top = -1.1 - 0.36 * (k - 1) as f32;
            let hk = hash(seed, k as f64);
            block(
                out,
                Vec3::new(cx, top - 0.23, cz),
                Vec3::new(w - 2.0 * inset, 0.46, h - 2.0 * inset),
                (hk as f32 - 0.5) * 0.08,
                jit(BASALT_D, hk, 0.1),
                SHADE_LIT,
            );
        }
        // Jagged rock hanging beneath, longest in the middle.
        for k in 0..24 {
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
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::new(sz, long / 4.0 + 0.02, sz * 0.85),
                    Quat::from_rotation_y(turn + t) * Quat::from_rotation_x((b - 0.5) * 0.3 * t),
                    Vec3::new(px + (b - 0.5) * 0.1 * t, y, pz + (a - 0.5) * 0.1 * t),
                );
                block_m(out, m, jit(BASALT[s % 3], hash(seed + s as f64, k as f64), 0.12) * 0.62, SHADE_LIT);
            }
        }
        // Veins of heat in the sides' joints.
        for k in 0..8 {
            let hk = hash(seed + k as f64 * 9.1, 2.0) as f32;
            let tall = 0.3 + hash(seed, k as f64 * 3.0) as f32 * 0.5;
            let (p, scale) = match k % 4 {
                0 => (Vec3::new(r.x0 as f32 - 0.62, -0.55, r.y0 as f32 + hk * (h - 1.0)), Vec3::new(0.02, tall, 0.04)),
                1 => (Vec3::new(r.x1 as f32 + 0.62, -0.55, r.y0 as f32 + hk * (h - 1.0)), Vec3::new(0.02, tall, 0.04)),
                2 => (Vec3::new(r.x0 as f32 + hk * (w - 1.0), -0.55, r.y0 as f32 - 0.62), Vec3::new(0.04, tall, 0.02)),
                _ => (Vec3::new(r.x0 as f32 + hk * (w - 1.0), -0.55, r.y1 as f32 + 0.62), Vec3::new(0.04, tall, 0.02)),
            };
            glowing(out, p, scale, 0.0, lin(0x3a1206), lin(EMBER) * 1.4);
        }
        // Bones off the paths through the middle, and heaps of broken basalt.
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
            bone_cubes(out, at, hk as f32);
            placed += 1;
        }
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
                let p = at + Vec3::new((hl - 0.5) * 0.5, size * 0.4 + if j == 4 { 0.07 } else { 0.0 }, (hj - 0.5) * 0.5);
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::new(size, size * 0.8, size * 0.9),
                    Quat::from_rotation_y(hj * TAU) * Quat::from_rotation_z((hl - 0.5) * 0.6),
                    p,
                );
                block_m(out, m, jit(BASALT[j % 3], hl as f64, 0.12) * 0.9, SHADE_STONE);
            }
            heaps += 1;
        }
        // Iron chains hung over the edges into the glow, a skull at the end of one now and then.
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
                block(out, ring, Vec3::new(0.12, 0.06, 0.12), PI / 4.0, lin(CHAIN), SHADE_IRON);
                let mut a = ring + Vec3::new(ox * 0.12, 0.0, oz * 0.12);
                for j in 0..8 {
                    let t = (j + 1) as f32 / 8.0;
                    let swing = (hk as f32 * 9.0 + t * 3.0).sin() * 0.2 * t;
                    let b = Vec3::new(
                        p.x as f32 + ox * (0.68 + t * 0.2) + oz * swing,
                        0.02 - t * 1.5,
                        p.y as f32 + oz * (0.68 + t * 0.2) + ox * swing,
                    );
                    block_m(
                        out,
                        forge::segment(a, b, if j % 2 == 0 { 0.055 } else { 0.035 }),
                        lin(CHAIN) * if j % 2 == 0 { 1.0 } else { 0.75 },
                        SHADE_IRON,
                    );
                    a = b;
                }
                if hk < 0.315 {
                    super::hell::skull(out, a - Vec3::new(0.0, 0.1, 0.0), 0.9);
                }
            }
        }
        // The lava's light, rising past the edges (three a side) and lighting them from below.
        for k in 0..3 {
            let t = (k as f32 + 0.5) / 3.0;
            let (u, v) = (r.x0 as f32 - 0.5 + t * w, r.y0 as f32 - 0.5 + t * h);
            self.hub_glow.push(Vec3::new(u, -0.75, r.y0 as f32 - 1.6));
            self.hub_glow.push(Vec3::new(u, -0.75, r.y1 as f32 + 1.6));
            self.hub_glow.push(Vec3::new(r.x0 as f32 - 1.6, -0.75, v));
            self.hub_glow.push(Vec3::new(r.x1 as f32 + 1.6, -0.75, v));
        }
        for c in hub::columns(r) {
            self.hub_column(c);
        }
    }

    /// A column in a platform's corner: a pale plinth, drums of basalt and umber with a band of
    /// ember runes, and either broken (its drums fallen beside it) or crowned with a pale capital
    /// and a copper fire bowl, burning (a pool of amber light).
    fn hub_column(&mut self, p: P) {
        let (x, z) = (p.x as f32, p.y as f32);
        let (hx, hy) = (p.x as f64, p.y as f64);
        let broken = hash(hx * 3.0, hy * 7.0) < 0.5;
        let start = self.static_cubes.len();
        let out = &mut self.static_cubes;
        let pale = jit(PALE[0], hash(hx, hy), 0.06) * 0.85;
        block(out, Vec3::new(x, 0.1, z), Vec3::new(0.88, 0.2, 0.88), 0.0, pale * 0.9, SHADE_STONE);
        block(out, Vec3::new(x, 0.25, z), Vec3::new(0.72, 0.1, 0.72), 0.0, pale, SHADE_STONE);
        let drums = if broken { 2 + (hash(hy, hx) * 2.0) as i32 } else { 5 };
        let mut top = 0.3;
        for i in 0..drums {
            let hk = hash(hx + i as f64 * 1.3, hy - i as f64);
            let c = if i % 3 == 2 { jit(UMBER[i as usize % 2], hk, 0.08) } else { jit(BASALT[i as usize % 3], hk, 0.1) * 1.05 };
            let wide = 0.5 + hk as f32 * 0.06;
            block(out, Vec3::new(x, top + 0.17, z), Vec3::new(wide, 0.32, wide), (hk as f32 - 0.5) * 0.2, c, SHADE_STONE);
            if i == 1 {
                // A band of runes, glowing ember.
                for f in 0..4 {
                    let a = f as f32 * PI / 2.0;
                    let n = Vec3::new(a.cos(), 0.0, a.sin());
                    glowing(
                        out,
                        Vec3::new(x, top + 0.17, z) + n * (wide / 2.0 + 0.005),
                        Vec3::new(0.2, 0.045, 0.02),
                        a + PI / 2.0,
                        lin(0x4a1406),
                        lin(EMBER) * 0.9,
                    );
                }
            }
            top += 0.34;
        }
        if broken {
            for k in 0..3 {
                let hk = hash(hx * 9.0 + k as f64, hy * 4.0) as f32;
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::new(0.22, 0.14 + hk * 0.12, 0.2),
                    Quat::from_rotation_y(hk * 4.0) * Quat::from_rotation_x((hk - 0.5) * 0.5),
                    Vec3::new(x + (hk - 0.5) * 0.3, top + 0.06 + hk * 0.06, z + (hash(hy, hx + k as f64) as f32 - 0.5) * 0.3),
                );
                block_m(out, m, jit(BASALT[k % 3], hk as f64, 0.1), SHADE_STONE);
            }
            // A drum fallen beside it, sunk askew, and a chip off it (both low, where he walks).
            let a = hash(hx, hy) as f32 * TAU;
            let m = Mat4::from_scale_rotation_translation(
                Vec3::new(0.46, 0.16, 0.46),
                Quat::from_rotation_y(a) * Quat::from_rotation_z(0.12),
                Vec3::new(x + a.cos() * 0.6, 0.06, z + a.sin() * 0.6),
            );
            block_m(out, m, jit(BASALT[2], hash(hy, hx), 0.1), SHADE_STONE);
            block(
                out,
                Vec3::new(x - a.sin() * 0.5, 0.05, z + a.cos() * 0.5),
                Vec3::new(0.16, 0.1, 0.12),
                a,
                jit(BASALT[0], hx, 0.1),
                SHADE_STONE,
            );
        } else {
            block(out, Vec3::new(x, top + 0.07, z), Vec3::new(0.7, 0.14, 0.7), 0.0, pale, SHADE_STONE);
            block(out, Vec3::new(x, top + 0.18, z), Vec3::new(0.78, 0.08, 0.78), 0.0, pale * 0.92, SHADE_STONE);
            top += 0.22;
            // A copper fire bowl on its three legs, burning.
            let bowl = out.len();
            brazier_cubes(out, Vec3::new(x, top, z), 0.8, true);
            tame_flames(&mut out[bowl..]);
            let coals = Vec3::new(x, top + 0.62, z);
            let len = out.len() - start;
            // (The pool's light hangs above the flames, so the copper does not blaze.)
            self.torches.insert(p, (coals + Vec3::Y * 0.3, (start, len)));
            self.smoke.push((coals + Vec3::Y * 0.2, 0.9));
            top += 0.75;
        }
        let len = self.static_cubes.len() - start;
        let shades = self.static_cubes[start..start + len].iter().map(|c| c.emissive[3]).collect();
        self.tree_index.insert(tile_key(p.x, p.y), self.trees.len());
        self.trees.push(Tree {
            range: (start, len),
            base: Vec3::new(x, 0.0, z),
            radius: 0.4,
            height: top,
            fade: 1.0,
            alive: true,
            shades: Some(shades),
        });
    }

    /// A bridge of old timber and hemp over the lava: planks (scorched, a few charred through,
    /// one now and then gone) on stringers and transoms, posts with iron bands, two sagging rope
    /// rails with hangers down to the deck, and a copper lantern on its middle post.
    fn hub_bridge(&mut self, r: Rect, i: usize) {
        let along = r.along_x();
        let (cx, cz) = r.center();
        let len = if along { (r.x1 - r.x0 + 1) as f32 } else { (r.y1 - r.y0 + 1) as f32 } + 0.7;
        let wide = if along { (r.y1 - r.y0 + 1) as f32 } else { (r.x1 - r.x0 + 1) as f32 } - 0.1;
        let turn = if along { 0.0 } else { PI / 2.0 };
        let q = Quat::from_rotation_y(turn);
        let at = |u: f32, y: f32, v: f32| Vec3::new(cx, y, cz) + q * Vec3::new(u, 0.0, v);
        let sag = |u: f32| 0.07 * (PI * u / len).cos().max(0.0);
        let seed = (r.x0 * 13 + r.y0 * 7) as f64;
        let wood = |h: f64, k: usize| jit(TIMBER[k % 3], h, 0.12);
        let out = &mut self.static_cubes;
        let n = (len / 0.2) as i32;
        let mut last_gone = false;
        for k in 0..n {
            let u = -len / 2.0 + (k as f32 + 0.5) * len / n as f32;
            let hk = hash(seed + k as f64 * 1.7, seed * 0.3);
            let gone = !last_gone && hk > 0.97 && k > 3 && k < n - 4;
            last_gone = gone;
            if gone {
                continue;
            }
            let colour = if hk < 0.07 { lin(SCORCH) } else { wood(hash(seed - k as f64, 2.0), k as usize) };
            let jitter = (hash(seed + k as f64, 9.0) as f32 - 0.5) * 0.08;
            let y = -0.035 - sag(u);
            let m = Mat4::from_scale_rotation_translation(
                Vec3::new(0.18, 0.07, wide + jitter),
                Quat::from_rotation_y(turn + jitter * 0.6) * Quat::from_rotation_x(jitter * 0.3),
                at(u, y, jitter * 0.5),
            );
            block_m(out, m, colour, SHADE_WOOD);
            // Scorched at one end where the heat comes up past the edge, an ember in the char.
            let hs = hash(seed * 0.7 + k as f64, 5.0);
            if hs < 0.3 {
                let side = if hs < 0.15 { -1.0 } else { 1.0 };
                let v = side * (wide / 2.0 + jitter * 0.5 - 0.09);
                let c = at(u, y + 0.004, v);
                let m = Mat4::from_scale_rotation_translation(Vec3::new(0.185, 0.07, 0.2), q, c);
                if hs < 0.05 {
                    out.push(Instance::new(m, lin(SCORCH), 1.0, lin(EMBER) * 0.35, SHADE_LIT));
                } else {
                    block_m(out, m, lin(SCORCH) * 1.2, SHADE_WOOD);
                }
            }
        }
        // Stringers under the deck, following its sag, and transoms across under them.
        let dark = lin(TIMBER[1]) * 0.5;
        let segs = 8;
        for side in [-1.0f32, 1.0] {
            for s in 0..segs {
                let (u0, u1) = (-len / 2.0 + s as f32 * len / segs as f32, -len / 2.0 + (s + 1) as f32 * len / segs as f32);
                let (a, b) = (at(u0, -0.14 - sag(u0), side * (wide / 2.0 - 0.25)), at(u1, -0.14 - sag(u1), side * (wide / 2.0 - 0.25)));
                block_m(out, forge::segment(a, b, 0.12), dark, SHADE_WOOD);
            }
        }
        let mut u = -len / 2.0 + 0.6;
        while u < len / 2.0 - 0.4 {
            let m = Mat4::from_scale_rotation_translation(Vec3::new(0.1, 0.1, wide - 0.2), q, at(u, -0.25 - sag(u), 0.0));
            block_m(out, m, dark * 0.9, SHADE_WOOD);
            u += 1.1;
        }
        // Posts with iron bands and caps; rope rails sagging between them, hangers to the deck.
        let posts = [-len / 2.0 + 0.25, 0.0, len / 2.0 - 0.25];
        let hemp = lin(HEMP);
        for side in [-1.0f32, 1.0] {
            let v = side * (wide / 2.0 + 0.02);
            for (k, &u) in posts.iter().enumerate() {
                let hk = hash(seed + k as f64 + side as f64, 3.0);
                block_m(out, forge::segment(at(u, -0.35, v), at(u, 0.8, v), 0.11), wood(hk, k + 1) * 0.85, SHADE_WOOD);
                block(out, at(u, 0.62, v), Vec3::new(0.13, 0.04, 0.13), turn, lin(IRON), SHADE_IRON);
                block(out, at(u, 0.82, v), Vec3::new(0.14, 0.05, 0.14), turn, lin(IRON), SHADE_IRON);
                // The rope lashed round it.
                block(out, at(u, 0.72, v), Vec3::new(0.135, 0.06, 0.135), turn + 0.3, hemp * 0.85, SHADE_SOFT);
            }
            for pair in posts.windows(2) {
                for (top, dip) in [(0.72f32, 0.18f32), (0.38, 0.09)] {
                    let steps = 8;
                    let point = |k: i32| {
                        let t = k as f32 / steps as f32;
                        let u = pair[0] + (pair[1] - pair[0]) * t;
                        at(u, top - dip * (PI * t).sin() - sag(u) * 0.5, v)
                    };
                    for k in 0..steps {
                        block_m(out, forge::segment(point(k), point(k + 1), 0.04), hemp, SHADE_SOFT);
                    }
                }
                // Hangers from the top rail down to the deck's edge.
                for k in 1..6 {
                    let t = k as f32 / 6.0;
                    let u = pair[0] + (pair[1] - pair[0]) * t;
                    let a = at(u, 0.72 - 0.18 * (PI * t).sin() - sag(u) * 0.5, v);
                    block_m(out, forge::segment(a, at(u, -0.02 - sag(u), v), 0.018), hemp * 0.8, SHADE_SOFT);
                }
            }
        }
        // A copper lantern hung from an iron arm on the middle post, its side turn about.
        let side = if i.is_multiple_of(2) { 1.0 } else { -1.0 };
        let start = out.len();
        let post = at(0.0, 0.0, side * (wide / 2.0 + 0.02));
        let arm = q * Vec3::new(0.0, 0.0, side * 0.26);
        block_m(out, forge::segment(post + Vec3::Y * 0.92, post + Vec3::Y * 0.92 + arm, 0.035), lin(IRON), SHADE_IRON);
        block_m(out, forge::segment(post + Vec3::Y * 0.8, post + Vec3::Y * 0.92 + arm * 0.6, 0.025), lin(IRON), SHADE_IRON);
        block_m(out, forge::segment(post + Vec3::Y * 0.8, post + Vec3::Y * 0.97, 0.1), wood(seed, 2) * 0.85, SHADE_WOOD);
        let glass = lantern_cubes(out, post + Vec3::Y * 0.9 + arm, 0.05);
        let len_l = out.len() - start;
        self.torches.insert(P::new(-100 - i as i32, -100), (glass - Vec3::Y * 0.5, (start, len_l)));
        // The lava's light under the deck.
        for s in [-0.25f32, 0.25] {
            self.hub_glow.push(at(s * len, -1.25, 0.0));
        }
    }

    /// The volcano in the middle: a mass of basalt in strata, ash on its shoulders, lava rivers
    /// running down its flanks into the sea, the crater's lake glowing and smoking; the House in
    /// its south face.
    fn hub_volcano(&mut self) {
        let v = hub::volcano();
        let (cx, cz) = v.center();
        let (radius, crater, peak) = (8.2f32, 1.8f32, 11.5f32);
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
                    let rim = r < crater + 0.6;
                    let raw = if r < crater {
                        peak - 1.4
                    } else {
                        let t = ((r - crater) / (radius - crater)).clamp(0.0, 1.0);
                        peak + (LAVA_Y - peak) * t.powf(0.75) + (hk - 0.5) * 0.4 + wobble * 0.35 + if rim { 0.35 } else { 0.0 }
                    };
                    // Strata: the slope in ledges, each band its own stone.
                    let band = (raw / 0.45).floor();
                    let top = if r < crater { raw } else { band * 0.45 + (hk - 0.5) * 0.08 };
                    let tall = top - LAVA_Y + 0.2;
                    let ashen = ((top - 4.0) / 4.5).clamp(0.0, 1.0);
                    let bh = hash(band as f64 * 3.1, 7.0);
                    let base = if bh < 0.25 {
                        jit(UMBER[1], hk as f64, 0.1) * 0.85
                    } else {
                        jit(BASALT[(bh * 9.0) as usize % 3], hk as f64, 0.12) * 0.8
                    };
                    let color = base.lerp(lin(0x6a625c), ashen * 0.55);
                    let a = dz.atan2(dx);
                    let on_river = r > crater + 0.3
                        && rivers.iter().any(|&ra| {
                            let da = (a - ra - 0.22 * (r * 0.9).sin()).rem_euclid(TAU);
                            da.min(TAU - da) < 0.16 + 0.12 * (r / radius)
                        });
                    if r < crater {
                        // The crater's lake.
                        block(out, Vec3::new(x, LAVA_Y - 0.2 + tall / 2.0, z), Vec3::new(0.52, tall, 0.52), 0.0, color, SHADE_STONE);
                        out.push(Instance::new(
                            Mat4::from_scale_rotation_translation(Vec3::new(0.5, 0.06, 0.5), Quat::IDENTITY, Vec3::new(x, top + 0.03, z)),
                            lin(CRUST),
                            1.0,
                            lava_glow(1.3),
                            SHADE_LAVA,
                        ));
                    } else if on_river {
                        // The river's bed, and the lava running down it, tall enough to bridge each
                        // ledge to the next so the river runs unbroken.
                        block(
                            out,
                            Vec3::new(x, LAVA_Y - 0.2 + (tall - 0.3) / 2.0, z),
                            Vec3::new(0.52, tall - 0.3, 0.52),
                            0.0,
                            color * 0.7,
                            SHADE_STONE,
                        );
                        out.push(Instance::new(
                            Mat4::from_scale_rotation_translation(Vec3::new(0.5, 0.62, 0.5), Quat::IDENTITY, Vec3::new(x, top - 0.25, z)),
                            lin(CRUST),
                            1.0,
                            lava_glow(0.85 + hk * 0.2),
                            SHADE_LAVA,
                        ));
                    } else {
                        // The ledge's face a block of its own (one block the whole height would
                        // stretch the stone's tooling into streaks), the body below it darker.
                        let lean = Quat::from_rotation_y((hk - 0.5) * 0.25);
                        let cap = tall.min(0.75);
                        let m = Mat4::from_scale_rotation_translation(
                            Vec3::new(0.54, cap, 0.54),
                            lean,
                            Vec3::new(x, top + 0.2 - cap / 2.0 - 0.2, z),
                        );
                        block_m(out, m, color, SHADE_STONE);
                        // (Below a ledge's face its neighbours hide the body, but at the foot and
                        // where a river cuts down beside it.)
                        let floor = if r > radius - 1.6 { LAVA_Y - 0.2 } else { (top - cap - 1.4).max(LAVA_Y - 0.2) };
                        let body = top - cap - floor;
                        if body > 0.01 {
                            block(out, Vec3::new(x, floor + body / 2.0, z), Vec3::new(0.52, body, 0.52), 0.0, color * 0.8, SHADE_LIT);
                        }
                    }
                }
                dx += step;
            }
            dz += step;
        }
        // The crater's smoke, lit from below.
        self.smoke.push((Vec3::new(cx, peak + 0.2, cz), 2.6));
        self.smoke.push((Vec3::new(cx + 0.6, peak + 0.4, cz - 0.5), 1.8));
        self.smoke.push((Vec3::new(cx - 0.5, peak + 0.3, cz + 0.4), 1.4));
        self.smoke.push((Vec3::new(cx + 0.2, peak + 1.4, cz + 0.2), 2.2));
        self.hub_house(cx, v.y1 as f32 + 0.45);
    }
}
