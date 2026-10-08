//! The Cyclops' meadow at the Cisterns' bar (RENDERING.md, Stage B, "The Cyclops' meadow, at the
//! Cisterns' bar"): a meadow at dusk round the ruins of an old estate. Turf in drifts of
//! wildflowers, darker in the hollows; tracks of trodden earth with stones in them, wet where they
//! run down to the stream; the stream under old timber bridges, its banks dressed stone where they
//! were built and earth and reeds where they are wild, a spring head pouring into it at one end and
//! a culvert taking it at the other; the pillars and low walls the crypt's chunky dressed stone,
//! chipped, mossy and fallen; a ruined boundary wall round the arena and a treeline beyond it,
//! dark against the dusk; and copper braziers and lanterns on the ruins, each a pool of amber light.
//!
//! What it replaced: one even lawn (`SHADE_GRASS`), plain-lit tufts, flowers and voxel bushes, brick
//! walls, pale drums and voxel hedges (no `voxel_surface` material: RP14), five fire bowls lit as
//! candles (RP17 under-used), and the garden's river builder. Every piece here is built in the
//! board's palette with its material and is left alone by the grade (`authored`).
//!
//! The layout is untouched: the same tiles are floor, water, bridge and rock; the pillars and low
//! walls stand no higher than they did; and a broken wall takes its stones and lantern with it
//! (`rock_index`, `torches`).

use super::crypt::pal::{CHARCOAL, IRON, MOSS, PALE_STONE, UMBER};
use super::crypt::{
    block, block_m, brazier_cubes, jit, lantern_cubes, masonry, moss_cap, spout_cubes, Wall, SHADE_FLAG, SHADE_IRON, SHADE_SOFT,
    SHADE_STONE, SHADE_WOOD,
};
use super::garden::WATER_DROP;
use super::garden_beds::SHADE_GRAVEL;
use super::*;
use crate::logic::levels::{ARENA, ARENA_GATE, BRIDGES, STREAM_ROW};

pub(super) mod pal {
    /// Meadow turf: moss and olive, deep in the hollows, drier and paler on the rises.
    pub const TURF_DEEP: u32 = 0x37552a;
    pub const TURF: u32 = 0x557a33;
    pub const TURF_DRY: u32 = 0x7a7a40;
    /// Trodden earth: umber, darker where it is wet.
    pub const EARTH: [u32; 3] = [0x3e3022, 0x46362a, 0x382c20];
    /// The stream's bed and its wild banks.
    pub const MUD: u32 = 0x231d16;
    pub const BANK: u32 = 0x3a2e22;
    /// Reeds and their heads.
    pub const REED: [u32; 2] = [0x56663a, 0x4a582e];
    pub const CATTAIL: u32 = 0x4a3020;
    /// Wildflowers in the board's quieter colours: buttercup amber, poppy rust, cornflower dusk,
    /// daisy pale stone, marigold copper.
    pub const BLOOMS: [u32; 5] = [0xc0983e, 0x8a4a3a, 0x6a6890, 0xc8c2ae, 0xa06a3c];
    /// Old timber, silvered and umber.
    pub const TIMBER: [u32; 3] = [0x5e5446, 0x52483c, 0x6a5e4c];
    /// Leaves and needles of the treeline: dark, a silhouette against the dusk.
    pub const LEAF: [u32; 3] = [0x30402a, 0x384a2e, 0x425434];
    pub const LEAF_LIT: u32 = 0x56603a;
    pub const PINE: [u32; 2] = [0x22372a, 0x2b4130];
    pub const BARK: [u32; 2] = [0x3e3128, 0x30271f];
    /// Bramble and ivy on the ruins.
    pub const BRAMBLE: [u32; 2] = [0x2c3a24, 0x344428];
    /// The hedgerow grown over the boundary wall: hawthorn and bramble, its new growth catching
    /// the low sun, a blossom or a haw here and there.
    pub const HEDGE: [u32; 3] = [0x2e4226, 0x36502c, 0x2a3c22];
    pub const HEDGE_LIT: u32 = 0x4e6436;
    pub const HAW: [u32; 2] = [0xc8c0a8, 0x8a3a2a];
    /// The mist over the water: mist teal, greyed.
    pub const MIST: u32 = 0x7a8c88;
}
use pal::*;

/// How high the low walls stand (as the brick walls did: a shin, so he sees over them).
const LOW_WALL_H: f32 = 0.48;
/// The stream's bed, below the floor.
const BED: f32 = -0.36;
/// Turf stands this much proud of the trodden earth.
const TURF_LIFT: f32 = 0.014;

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Smooth value noise over the ground (0..1), so tones and drifts run across tiles.
fn noise(x: f32, z: f32) -> f32 {
    let (ix, iz) = (x.floor(), z.floor());
    let (fx, fz) = (x - ix, z - iz);
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let h = |a: f32, b: f32| hash(a as f64 * 1.731 + 17.0, b as f64 * 2.917 + 3.0) as f32;
    let (a, b, c, d) = (h(ix, iz), h(ix + 1.0, iz), h(ix, iz + 1.0), h(ix + 1.0, iz + 1.0));
    let (u, v) = (s(fx), s(fz));
    let top = a + (b - a) * u;
    let bottom = c + (d - c) * u;
    top + (bottom - top) * v
}

/// How lush the turf is at a spot: 0 in a hollow, 1 high and dry.
fn lush(x: f32, z: f32) -> f32 {
    noise(x * 0.27, z * 0.27) * 0.6 + noise(x * 0.73 + 9.0, z * 0.73 + 4.0) * 0.4
}

fn in_arena(x: f32, z: f32) -> bool {
    let (x0, y0, x1, y1) = ARENA;
    x > x0 as f32 - 0.5 && x < x1 as f32 + 0.5 && z > y0 as f32 - 0.5 && z < y1 as f32 + 0.5
}

/// How trodden the ground is at a spot (0 turf .. 1 bare earth): the track from where he comes in
/// over the middle bridge to the gate, short tracks to the side bridges, earth scuffed round the
/// pillars, a few bare patches, and the ring's walks worn down their middles.
fn worn(x: f32, z: f32, pillars: &[Vec2]) -> f32 {
    let ragged = 0.8 + 0.4 * noise(x * 2.3 + 5.0, z * 2.3 + 1.0);
    if !in_arena(x, z) {
        return (0.18 + 0.5 * noise(x * 1.1 + 3.0, z * 1.1 + 8.0)) * ragged;
    }
    // The track wanders, and the turf closes over it here and there.
    let cx = ARENA_GATE.0 as f32 + (z * 0.55).sin() * 0.34 + (z * 1.31 + 2.0).sin() * 0.09;
    let mut w = (1.0 - smooth(0.1, 0.3, (x - cx).abs())) * (0.55 + 0.6 * noise(z * 0.7 + 2.0, x * 0.2 + 7.0));
    for b in [BRIDGES[0], BRIDGES[2]] {
        let along = 1.0 - smooth(0.9, 2.1, (z - STREAM_ROW as f32).abs());
        let off = (x - b as f32 - (z * 0.9).sin() * 0.08).abs();
        w = w.max((1.0 - smooth(0.1, 0.3, off)) * along);
    }
    for p in pillars {
        let d = Vec2::new(x - p.x, z - p.y).length();
        w = w.max((1.0 - smooth(0.42, 0.75, d)) * (0.45 + 0.6 * noise(x * 3.1, z * 3.1)));
    }
    w = w.max(smooth(0.9, 0.97, noise(x * 0.43 + 13.0, z * 0.43 + 21.0)));
    w * ragged
}

/// The turf's tone at a spot, from how lush and how worn it is.
fn turf_tone(l: f32, w: f32, k: f32) -> Vec3 {
    let c = if l < 0.5 { lin(TURF_DEEP).lerp(lin(TURF), l * 2.0) } else { lin(TURF).lerp(lin(TURF_DRY), (l - 0.5) * 1.1) };
    // Darker in the hollows, where the dusk light does not reach.
    let c = c * (0.7 + 0.38 * smooth(0.18, 0.62, l));
    // Thinner and yellowed at the edge of a track.
    c.lerp(lin(0x6a6638), w * 0.55) * (0.95 + k * 0.1)
}

/// A frame along one side of a tile: `u` along the side, `o` out from the tile's middle toward
/// `d`, heights `y` as given.
fn side_at(c: Vec3, (dx, dy): (i32, i32)) -> impl Fn(f32, f32, f32) -> Vec3 {
    move |u: f32, y: f32, o: f32| {
        Vec3::new(c.x + dx as f32 * o + if dx != 0 { 0.0 } else { u }, y, c.z + dy as f32 * o + if dx != 0 { u } else { 0.0 })
    }
}

fn side_turn((dx, _): (i32, i32)) -> f32 {
    if dx != 0 {
        PI / 2.0
    } else {
        0.0
    }
}

/// A tuft of three blades at `p`, `tall` at most.
fn tuft(out: &mut Vec<Instance>, p: Vec3, tone: Vec3, tall: f32, k: f32) {
    for b in 0..3 {
        let hb = hash(k as f64 * 91.0 + b as f64, k as f64 * 37.0 + 3.0) as f32;
        let t = tall * (0.55 + hb * 0.45);
        let lean = Quat::from_rotation_y(k * 6.0 + b as f32 * 2.1) * Quat::from_rotation_z((hb - 0.5) * 0.8);
        let at = p + Vec3::new((b as f32 - 1.0) * 0.024, t / 2.0, (hb - 0.5) * 0.04);
        block_m(
            out,
            Mat4::from_scale_rotation_translation(Vec3::new(0.017, t, 0.017), lean, at),
            tone * (0.82 + 0.14 * b as f32),
            SHADE_SOFT,
        );
    }
}

/// A wildflower at `p`: a stem, a leaf at its foot and its bloom.
fn flower(out: &mut Vec<Instance>, p: Vec3, bloom: Vec3, k: f32) {
    let tall = 0.1 + k * 0.12;
    let lean = Quat::from_rotation_z((k - 0.5) * 0.3) * Quat::from_rotation_x(((k * 7.0).fract() - 0.5) * 0.3);
    let top = p + lean * Vec3::Y * tall;
    block_m(
        out,
        Mat4::from_scale_rotation_translation(Vec3::new(0.012, tall, 0.012), lean, p + lean * Vec3::Y * (tall / 2.0)),
        lin(0x3a5228),
        SHADE_SOFT,
    );
    block(out, p + Vec3::new(0.02, 0.025, 0.0), Vec3::new(0.06, 0.02, 0.035), k * 9.0, lin(0x405a2c), SHADE_SOFT);
    block(out, top, Vec3::new(0.05, 0.026, 0.05), k * 5.0, bloom * (0.85 + k * 0.2), SHADE_SOFT);
    if k > 0.55 {
        // A second bloom on a side stem.
        let side = p + Vec3::new((k - 0.75) * 0.12, tall * 0.7, ((k * 13.0).fract() - 0.5) * 0.1);
        block(out, side, Vec3::new(0.04, 0.022, 0.04), k * 11.0, bloom * 0.92, SHADE_SOFT);
    }
}

/// A tree of the treeline standing at `base`, `height` tall, its crown at most `reach` out: an
/// oak's crown of big overlapping clumps on a short trunk (a dark mass against the dusk, its top
/// lit by the low sun), or a fir as a dense cone of drooping tiers.
fn field_tree(out: &mut Vec<Instance>, base: Vec3, fir: bool, height: f32, reach: f32, seed: f64) {
    let r = |k: f64| hash(seed * 1.7 + k * 3.3, seed * 0.9 - k * 1.1) as f32;
    let trunk_h = if fir { height * 0.18 } else { height * 0.36 };
    let lean = Vec3::new((r(1.0) - 0.5) * 0.14, 1.0, (r(2.0) - 0.5) * 0.14).normalize();
    let top = base + lean * trunk_h;
    let w = 0.12 + height * 0.014;
    block_m(
        out,
        Mat4::from_scale_rotation_translation(
            Vec3::new(w, trunk_h + 0.3, w),
            Quat::from_rotation_arc(Vec3::Y, lean) * Quat::from_rotation_y(r(5.0) * 2.0),
            base + lean * ((trunk_h + 0.3) / 2.0),
        ),
        jit(BARK[0], r(6.0) as f64, 0.12),
        SHADE_WOOD,
    );
    for k in 0..3 {
        let a = k as f32 / 3.0 * TAU + r(7.0) * 2.0;
        let d = Vec3::new(a.cos(), -0.35, a.sin());
        block_m(
            out,
            Mat4::from_scale_rotation_translation(
                Vec3::new(0.07, 0.22, 0.07),
                Quat::from_rotation_arc(Vec3::Y, d),
                base + d * 0.09 + Vec3::Y * 0.04,
            ),
            lin(BARK[1]) * 0.9,
            SHADE_WOOD,
        );
    }
    if fir {
        // Tiers from wide to narrow, each a heart turned from the last and four boughs drooping
        // out of it, so the cone reads solid.
        let tiers = 6;
        let crown = height - trunk_h;
        for i in 0..tiers {
            let t = i as f32 / tiers as f32;
            let wd = reach * (1.0 - t * 0.8);
            let y = crown * t * 0.92;
            let c = jit(PINE[i % 2], r(25.0 + i as f64) as f64, 0.1) * (0.8 + t * 0.35);
            block(
                out,
                top + Vec3::Y * (y + crown * 0.08),
                Vec3::new(wd * 0.85, crown * 0.18, wd * 0.85),
                i as f32 * 0.785 + r(27.0),
                c * 0.8,
                SHADE_SOFT,
            );
            for b in 0..6 {
                let a = b as f32 / 6.0 * TAU + i as f32 * 0.52 + r(26.0 + b as f64) * 0.4;
                let len = wd * (0.9 + r(28.0 + (i * 5 + b) as f64) * 0.3);
                let dir = Vec3::new(a.cos(), 0.0, a.sin());
                let rot = Quat::from_rotation_y(-a) * Quat::from_rotation_z(-0.38);
                block_m(
                    out,
                    Mat4::from_scale_rotation_translation(
                        Vec3::new(len, 0.09, wd * 0.45),
                        rot,
                        top + Vec3::Y * (y + 0.04) + dir * len * 0.45,
                    ),
                    c,
                    SHADE_SOFT,
                );
            }
        }
        block(out, top + Vec3::Y * (crown + 0.06), Vec3::new(0.08, 0.26, 0.08), 0.0, lin(PINE[1]), SHADE_SOFT);
        return;
    }
    for b in 0..2 {
        let a = r(8.0 + b as f64) * TAU;
        let d = Vec3::new(a.cos() * 0.8, 1.0, a.sin() * 0.8).normalize();
        let l = height * 0.2;
        block_m(
            out,
            Mat4::from_scale_rotation_translation(Vec3::new(0.07, l, 0.07), Quat::from_rotation_arc(Vec3::Y, d), top + d * l * 0.5),
            lin(BARK[0]),
            SHADE_WOOD,
        );
    }
    let crown = height - trunk_h;
    // The crown in three puffs, each round (two squares a quarter turn apart), widest low down,
    // their tones darkening toward the shade; then clumps breaking the outline, and its top lit.
    let puffs: [(f32, f32, f32, u32); 3] = [(0.28, 1.75, 0.42, LEAF[0]), (0.58, 1.55, 0.4, LEAF[1]), (0.84, 0.95, 0.3, LEAF[2])];
    for (i, (y, w, h, col)) in puffs.into_iter().enumerate() {
        let off = Vec3::new((r(40.0 + i as f64) - 0.5) * reach * 0.3, crown * y, (r(41.0 + i as f64) - 0.5) * reach * 0.3);
        let turn = r(42.0 + i as f64) * 1.5;
        for q in 0..2 {
            block(
                out,
                top + off,
                Vec3::new(reach * w, crown * h, reach * w * 0.92),
                turn + q as f32 * PI / 4.0,
                jit(col, r(43.0 + i as f64 + q as f64) as f64, 0.1) * (0.9 + q as f32 * 0.08),
                SHADE_SOFT,
            );
        }
    }
    let mut k = 10.0;
    for j in 0..16 {
        k += 1.0;
        let a = j as f32 / 16.0 * TAU * 3.0 + r(k) * 0.6;
        let y = 0.2 + (j % 3) as f32 * 0.25 + (r(k + 0.4) - 0.5) * 0.12;
        let rr = reach * (0.8 - (y - 0.2) * 0.45) * (0.85 + r(k + 0.5) * 0.25);
        let s = reach * (0.28 + r(k + 0.2) * 0.16);
        let at = top + Vec3::new(a.cos() * rr, crown * y, a.sin() * rr);
        let rot = Quat::from_rotation_y(a + r(k + 1.3))
            * Quat::from_rotation_x((r(k + 1.5) - 0.5) * 0.6)
            * Quat::from_rotation_z((r(k + 1.6) - 0.5) * 0.6);
        block_m(
            out,
            Mat4::from_scale_rotation_translation(Vec3::new(s, s * 0.7, s * 0.85), rot, at),
            jit(LEAF[j % 3], r(k + 1.1) as f64, 0.12),
            SHADE_SOFT,
        );
    }
    for j in 0..2 {
        let a = j as f32 * 2.6 + r(30.0) * 3.0;
        let cap = reach * (0.42 + r(31.0 + j as f64) * 0.12);
        let at = top + Vec3::new(a.cos() * reach * 0.22, crown * 0.97, a.sin() * reach * 0.22);
        block(
            out,
            at,
            Vec3::new(cap, cap * 0.45, cap * 0.85),
            r(32.0 + j as f64) * 3.0,
            jit(LEAF_LIT, r(33.0 + j as f64) as f64, 0.1) * 0.85,
            SHADE_SOFT,
        );
    }
}

/// Bramble and ivy in a few dark clumps round `at`, `span` across, none higher than `tall`.
fn bramble(out: &mut Vec<Instance>, at: Vec3, span: f32, tall: f32, (hx, hy): (f64, f64)) {
    for k in 0..4 {
        let a = hash(hx * 3.1 + k as f64, hy * 1.7) as f32;
        let b = hash(hy * 2.3 + k as f64, hx * 2.9) as f32;
        let s = 0.12 + a * 0.14;
        let h = (tall * (0.4 + b * 0.6)).max(0.06);
        let p = at + Vec3::new((a - 0.5) * span, h / 2.0, (b - 0.5) * span);
        block(out, p, Vec3::new(s, h, s * 0.9), a * 4.0, jit(BRAMBLE[k % 2], b as f64, 0.12), SHADE_SOFT);
        if k % 2 == 0 {
            block(
                out,
                p + Vec3::Y * (h / 2.0),
                Vec3::new(s * 0.6, 0.05, s * 0.5),
                a * 4.0 + 0.7,
                jit(MOSS[k % 3], a as f64, 0.1) * 1.3,
                SHADE_SOFT,
            );
        }
    }
}

/// The lantern just built (`lantern_cubes` lays its glass last) glows softer: its pool on the
/// ground is the light, not the glass.
fn dim_glass(out: &mut [Instance]) {
    if let Some(g) = out.last_mut() {
        for e in g.emissive.iter_mut().take(3) {
            *e *= 0.55;
        }
    }
}

impl WorldView {
    /// The meadow's static geometry (see the module's notes). Rock inside the arena is a pillar
    /// (standing alone) or a low wall (in runs); rock in the ring is the ruined boundary wall, and
    /// at the level's edge the treeline.
    pub(super) fn build_meadow_field(&mut self, grid: &Grid) {
        let rock = |x: i32, y: i32| grid.get(x, y) == Tile::Rock;
        let (w, h) = (grid.width, grid.height);
        let border = |x: i32, y: i32| x == 0 || y == 0 || x == w - 1 || y == h - 1;
        let arena = |x: i32, y: i32| in_arena(x as f32, y as f32);
        let pillars: Vec<Vec2> = grid
            .cells()
            .filter(|&(x, y, t)| t == Tile::Rock && arena(x, y) && !(rock(x - 1, y) || rock(x + 1, y) || rock(x, y - 1) || rock(x, y + 1)))
            .map(|(x, y, _)| Vec2::new(x as f32, y as f32))
            .collect();
        let water = |x: i32, y: i32| grid.get(x, y) == Tile::Pit || grid.is_bridge(x, y);

        // ---- the practicals first, so rain can stand where they light the ground
        for (x, y, t) in grid.cells() {
            if t != Tile::Rock || !arena(x, y) {
                continue;
            }
            if pillars.contains(&Vec2::new(x as f32, y as f32)) {
                self.ruin_pillar(x, y);
            } else {
                let start = self.static_cubes.len();
                self.low_wall(grid, x, y);
                let len = self.static_cubes.len() - start;
                self.rock_index.insert(tile_key(x, y), (start, len));
                self.authored.push((start, len));
            }
        }

        // ---- the ground
        let start = self.static_cubes.len();
        for (x, y, t) in grid.cells() {
            if t == Tile::Rock || t == Tile::Pit || grid.is_bridge(x, y) {
                continue;
            }
            self.meadow_ground(grid, x, y, &pillars);
        }
        self.authored.push((start, self.static_cubes.len() - start));

        // ---- the stream, its banks, bridges, spring head and culvert
        let start = self.static_cubes.len();
        for (x, y, _) in grid.cells() {
            if water(x, y) {
                self.stream_tile(grid, x, y);
            }
        }
        self.authored.push((start, self.static_cubes.len() - start));

        // ---- the ring: the ruined boundary wall, the treeline at the edge
        let mut trees: Vec<(Option<P>, Vec3, bool, f32, f32)> = Vec::new();
        for (x, y, t) in grid.cells() {
            if t != Tile::Rock || arena(x, y) {
                continue;
            }
            // The gatehouse stands here (its towers either side): no wall through it.
            if self.gate_tile.is_some_and(|g| g.y == y && (g.x - x).abs() <= 1) {
                continue;
            }
            let (hx, hy) = (x as f64, y as f64);
            let start = self.static_cubes.len();
            if border(x, y) {
                // The treeline's floor: dark turf and undergrowth.
                let c = Vec3::new(x as f32, 0.0, y as f32);
                block(
                    &mut self.static_cubes,
                    c - Vec3::Y * 0.05,
                    Vec3::new(1.0, 0.1, 1.0),
                    0.0,
                    turf_tone(0.15, 0.0, hash(hx, hy) as f32) * 0.7,
                    SHADE_GRASS,
                );
                bramble(&mut self.static_cubes, c, 0.8, 0.3, (hx * 1.3, hy * 0.7));
                let pick = hash(hx * 5.0 + 1.0, hy * 3.0 + 2.0);
                if pick < 0.7 {
                    let south = y == h - 1;
                    let north = y == 0;
                    let height = if north {
                        2.7 + hash(hy * 1.3, hx * 0.7) as f32 * 1.1
                    } else if south {
                        1.7 + hash(hy * 1.3, hx * 0.7) as f32 * 0.6
                    } else {
                        2.2 + hash(hy * 1.3, hx * 0.7) as f32 * 0.9
                    };
                    let jitter = Vec3::new((hash(hx + 1.0, hy) as f32 - 0.5) * 0.3, 0.0, (hash(hx, hy + 1.0) as f32 - 0.5) * 0.3);
                    trees.push((
                        Some(P::new(x, y)),
                        c + jitter,
                        hash(hx * 7.0 + 3.0, hy * 2.0) < 0.55,
                        height,
                        0.62 + hash(hx * 2.0, hy * 5.0) as f32 * 0.2,
                    ));
                }
            } else {
                self.ring_wall(grid, x, y);
            }
            let len = self.static_cubes.len() - start;
            self.rock_index.insert(tile_key(x, y), (start, len));
            self.authored.push((start, len));
        }
        // Beyond the level's edge, the woods go on: a skirt of dark ground and a row of trees more.
        let start = self.static_cubes.len();
        let (wf, hf) = (w as f32, h as f32);
        let skirt = 1.6;
        for (c, s) in [
            (Vec3::new(wf / 2.0 - 0.5, 0.0, -0.5 - skirt / 2.0), Vec3::new(wf + 2.0 * skirt, 0.1, skirt)),
            (Vec3::new(wf / 2.0 - 0.5, 0.0, hf - 0.5 + skirt / 2.0), Vec3::new(wf + 2.0 * skirt, 0.1, skirt)),
            (Vec3::new(-0.5 - skirt / 2.0, 0.0, hf / 2.0 - 0.5), Vec3::new(skirt, 0.1, hf)),
            (Vec3::new(wf - 0.5 + skirt / 2.0, 0.0, hf / 2.0 - 0.5), Vec3::new(skirt, 0.1, hf)),
        ] {
            block(&mut self.static_cubes, c - Vec3::Y * 0.05, s, 0.0, turf_tone(0.1, 0.0, 0.5) * 0.6, SHADE_GRASS);
            block(
                &mut self.static_cubes,
                c - Vec3::Y * 0.9,
                s + Vec3::new(0.02, 1.6, 0.02) - Vec3::Y * 0.1,
                0.0,
                jit(UMBER[1], 0.4, 0.05) * 0.6,
                SHADE_STONE,
            );
        }
        let mut i = 0;
        let mut ring_spot = |x: f32, z: f32, south: bool, trees: &mut Vec<(Option<P>, Vec3, bool, f32, f32)>| {
            i += 1;
            let k = hash(x as f64 * 2.1 + 7.0, z as f64 * 1.3 + 3.0);
            if k > 0.75 {
                return;
            }
            let height = if south { 1.5 + k as f32 * 0.8 } else { 2.6 + k as f32 * 1.6 };
            let j = Vec3::new((hash(i as f64, 1.0) as f32 - 0.5) * 0.4, 0.0, (hash(i as f64, 2.0) as f32 - 0.5) * 0.4);
            trees.push((None, Vec3::new(x, 0.0, z) + j, k < 0.4, height, 0.7 + k as f32 * 0.2));
        };
        for x in -1..=w {
            ring_spot(x as f32, -1.25, false, &mut trees);
            ring_spot(x as f32, hf - 0.5 + 0.8, true, &mut trees);
        }
        for z in 0..h {
            ring_spot(-1.25, z as f32, false, &mut trees);
            ring_spot(wf - 0.5 + 0.75, z as f32, z as f32 > hf * 0.8, &mut trees);
        }
        self.authored.push((start, self.static_cubes.len() - start));
        for (key, pos, pine, height, reach) in trees {
            let start = self.static_cubes.len();
            field_tree(&mut self.static_cubes, pos, pine, height, reach, (pos.x * 131.0 + pos.z * 17.0) as f64 * 0.37);
            let len = self.static_cubes.len() - start;
            let shades = self.static_cubes[start..start + len].iter().map(|c| c.emissive[3]).collect();
            if let Some(k) = key {
                self.tree_index.insert(k, self.trees.len());
            }
            self.trees.push(Tree {
                range: (start, len),
                base: pos,
                radius: reach + 0.1,
                height,
                fade: 1.0,
                alive: true,
                shades: Some(shades),
            });
            self.authored.push((start, len));
        }
        self.meadow_mist(grid);
    }

    /// One floor tile in four quarters, each turf or trodden earth by where it lies; tufts, and
    /// wildflowers where a drift crosses it; stones in the earth, and rain standing in it by the
    /// water and the lanterns.
    fn meadow_ground(&mut self, grid: &Grid, x: i32, y: i32, pillars: &[Vec2]) {
        let (xf, yf) = (x as f32, y as f32);
        let (hx, hy) = (x as f64, y as f64);
        let rock = |x: i32, y: i32| grid.get(x, y) == Tile::Rock;
        // Long grass against walls, banks and the ring; short where the fights are.
        let sheltered = CARDINALS.iter().any(|d| {
            let (dx, dy) = d.vec();
            rock(x + dx, y + dy) || grid.get(x + dx, y + dy) == Tile::Pit
        }) || !in_arena(xf, yf);
        let lit = self.torches.values().any(|(t, _)| (t.x - xf).abs() < 2.2 && (t.z - yf).abs() < 2.2);
        let out = &mut self.static_cubes;
        // Quarters of a tile, or sixteenths where a track crosses it, so its edges are ragged.
        let track = (0..9).any(|i| worn(xf + (i % 3) as f32 * 0.4 - 0.4, yf + (i / 3) as f32 * 0.4 - 0.4, pillars) > 0.3);
        let n = if track { 4 } else { 2 };
        let cell = 1.0 / n as f32;
        let mut bare = 0;
        for q in 0..n * n {
            let (sx, sz) = (xf - 0.5 + cell * ((q % n) as f32 + 0.5), yf - 0.5 + cell * ((q / n) as f32 + 0.5));
            let wv = worn(sx, sz, pillars);
            let k = hash(hx * 3.1 + q as f64, hy * 1.7 + q as f64 * 2.0) as f32;
            if wv > 0.55 {
                bare += 1;
                let c = jit(EARTH[(k * 3.0) as usize % 3], k as f64, 0.08) * (0.88 + 0.2 * noise(sx * 1.3, sz * 1.3));
                block(out, Vec3::new(sx, -0.05, sz), Vec3::new(cell, 0.1, cell), 0.0, c, SHADE_GRAVEL);
                // Stones trodden into it, and now and then a flat one set in the track.
                let (a, b) = (hash(hx * 7.1 + q as f64 * 3.0, hy * 3.3) as f32, hash(hy * 5.7 + q as f64, hx * 2.9) as f32);
                if a < 0.6 * cell * 2.0 {
                    let c = if b < 0.5 { jit(PALE_STONE[q % 2], b as f64, 0.1) * 0.6 } else { jit(CHARCOAL[q % 3], b as f64, 0.1) * 0.85 };
                    block(
                        out,
                        Vec3::new(sx + (a - 0.3) * cell, 0.006, sz + (b - 0.5) * cell * 0.8),
                        Vec3::new(0.035 + a * 0.04, 0.03, 0.03 + b * 0.035),
                        a * 6.0,
                        c,
                        SHADE_STONE,
                    );
                }
                if k < 0.05 {
                    block(
                        out,
                        Vec3::new(sx, 0.004, sz),
                        Vec3::new(0.22, 0.03, 0.18),
                        k * 30.0,
                        jit(PALE_STONE[1], k as f64, 0.06) * 0.58,
                        SHADE_FLAG,
                    );
                }
                // Rain standing in the track, by the water and the lanterns, where it holds the light.
                let hp = hash(hx * 2.3 + q as f64 * 5.0 + 17.0, hy * 3.1 + 5.0) as f32;
                if hp < if lit { 0.22 } else { 0.0 } * cell * 2.0 {
                    let s = (0.2 + hash(hy, hx + q as f64) as f32 * 0.14).min(cell * 0.9);
                    self.puddles.push((Vec3::new(sx, 0.006, sz), Vec2::new(s, s * 0.8)));
                }
            } else {
                let l = lush(sx, sz);
                block(
                    out,
                    Vec3::new(sx, -0.05 + TURF_LIFT / 2.0, sz),
                    Vec3::new(cell, 0.1 + TURF_LIFT, cell),
                    0.0,
                    turf_tone(l, wv, k),
                    SHADE_GRASS,
                );
            }
        }
        let bare_all = n * n;
        // Tufts: long ones in the shelter of walls and banks and at a track's edge, short in the open.
        let edge = bare > 0 && bare < bare_all;
        let tufts = if sheltered {
            5
        } else if edge {
            4
        } else {
            2
        };
        for j in 0..tufts {
            let (a, b) = (hash(hx * 5.0 + j as f64 * 1.7, hy * 3.0 + 1.0) as f32, hash(hy * 5.0 + j as f64 * 2.3, hx * 7.0 + 2.0) as f32);
            let p = Vec3::new(xf + (a - 0.5) * 0.9, TURF_LIFT, yf + (b - 0.5) * 0.9);
            if worn(p.x, p.z, pillars) > 0.62 && !edge {
                continue;
            }
            let tone = turf_tone(lush(p.x, p.z), 0.0, a) * 1.2 + lin(0x6a6030) * 0.12;
            let tall = if sheltered { 0.16 + a * 0.12 } else { 0.08 + a * 0.07 };
            tuft(out, p, tone, tall, a * 13.0 + b * 7.0);
        }
        // Wildflowers in drifts: where a drift crosses the tile, a few blooms of its colour.
        let drift = noise(xf * 0.33 + 31.0, yf * 0.33 + 7.0);
        if drift > 0.52 && bare * 4 < bare_all * 3 {
            let n = (((drift - 0.52) * 32.0) as usize + 2).min(10);
            let pick = hash((xf / 4.0).floor() as f64 * 3.7 + 1.0, (yf / 4.0).floor() as f64 * 2.3 + 5.0);
            let pair = [BLOOMS[(pick * 5.0) as usize % 5], BLOOMS[(pick * 13.0) as usize % 5]];
            for j in 0..n {
                let (a, b) = (hash(hx * 9.1 + j as f64, hy * 4.3 + 2.0) as f32, hash(hy * 7.3 + j as f64, hx * 6.1 + 1.0) as f32);
                let p = Vec3::new(xf + (a - 0.5) * 0.86, TURF_LIFT, yf + (b - 0.5) * 0.86);
                if worn(p.x, p.z, pillars) > 0.5 {
                    continue;
                }
                let bloom = lin(pair[if (a * 7.0).fract() < 0.75 { 0 } else { 1 }]);
                flower(out, p, bloom, b);
            }
        }
    }

    /// A pillar of the old estate: chunky dressed stone on a pale plinth, moss in its seams and ivy
    /// up one side; standing, it carries a copper brazier (a practical: its pool lies on the
    /// ground round it); broken, its drums lie fallen in the grass.
    fn ruin_pillar(&mut self, x: i32, y: i32) {
        let (xf, yf) = (x as f32, y as f32);
        let (hx, hy) = (x as f64, y as f64);
        let c = Vec3::new(xf, 0.0, yf);
        let start = self.static_cubes.len();
        // The two corner pillars across the arena from each other have fallen; the rest stand.
        let broken = (x < ARENA_GATE.0 && y > STREAM_ROW) || (x > ARENA_GATE.0 && y < STREAM_ROW && y < 7);
        let out = &mut self.static_cubes;
        // The plinth: two steps, each of four chunky slabs, a touch askew and chipped.
        for (step, (half, h0, h1)) in [(0.43f32, 0.0f32, 0.16f32), (0.35, 0.16, 0.25)].into_iter().enumerate() {
            for (k, (sx, sz)) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)].into_iter().enumerate() {
                let hk = hash(hx * 3.0 + k as f64 + step as f64 * 7.0, hy * 5.0 + k as f64);
                let chip = if hk < 0.3 { 0.03 } else { 0.0 };
                block(
                    out,
                    c + Vec3::new(sx * half / 2.0, (h0 + h1 - chip) / 2.0, sz * half / 2.0),
                    Vec3::new(half - 0.02 - chip, h1 - h0 - chip, half - 0.02),
                    (hk as f32 - 0.5) * 0.05,
                    jit(PALE_STONE[(k + step) % 2], hk, 0.06) * (0.76 - step as f32 * 0.04),
                    SHADE_STONE,
                );
            }
        }
        // Moss on the plinth's step.
        block(out, c + Vec3::new(0.2, 0.17, -0.25), Vec3::new(0.26, 0.025, 0.12), 0.4, jit(MOSS[1], hx, 0.1), SHADE_SOFT);
        // The shaft: drums of chunky stone, each its own stone and a touch turned, blocks standing
        // proud of them here and there, moss in some seams.
        let drums = if broken { 3 } else { 5 };
        let mut top = 0.25;
        for i in 0..drums {
            let hk = hash(hx + i as f64 * 1.3, hy - i as f64 * 0.7);
            let dh = 0.3;
            let col = if i % 3 == 1 { jit(UMBER[i % 2], hk, 0.08) } else { jit(CHARCOAL[(i + x as usize) % 3], hk, 0.1) };
            let chip = if (hk * 29.0).fract() < 0.3 { 0.04 } else { 0.0 };
            let turn = (hk as f32 - 0.5) * 0.18;
            block(out, c + Vec3::Y * (top + dh / 2.0), Vec3::new(0.5 - chip, dh - 0.025, 0.5), turn, col, SHADE_STONE);
            for j in 0..2 {
                let hj = hash(hk * 17.0 + j as f64, hx + hy);
                let a = (hj as f32 * 4.0).floor() * PI / 2.0 + turn;
                let n = Vec3::new(a.cos(), 0.0, a.sin());
                let along = Vec3::new(-n.z, 0.0, n.x) * ((hj as f32 * 7.0).fract() - 0.5) * 0.25;
                block(out, c + n * 0.24 + along + Vec3::Y * (top + dh / 2.0), Vec3::new(0.06, dh - 0.05, 0.2), a, col * 0.92, SHADE_STONE);
            }
            if i > 0 && hash(hk * 5.0, 3.0) < 0.5 {
                block(out, c + Vec3::Y * top, Vec3::new(0.52, 0.025, 0.52), turn, jit(MOSS[i % 3], hk, 0.1), SHADE_SOFT);
            }
            top += dh;
        }
        // Ivy up the side away from the dusk sun.
        let side = Vec3::new(0.27, 0.0, 0.0);
        for k in 0..6 {
            let hk = hash(hx * 13.0 + k as f64, hy * 17.0) as f32;
            let hgt = 0.3 + k as f32 * (top - 0.4) / 6.0;
            block(
                out,
                c + side + Vec3::new(0.0, hgt, (hk - 0.5) * 0.3),
                Vec3::new(0.05, 0.16 + hk * 0.1, 0.1 + hk * 0.1),
                0.0,
                jit(BRAMBLE[k % 2], hk as f64, 0.15) * 1.2,
                SHADE_SOFT,
            );
        }
        if broken {
            // A jagged break, and the fallen drums lying in the grass beside it, mossed over.
            for k in 0..3 {
                let hk = hash(hx * 9.0 + k as f64, hy * 4.0) as f32;
                let off = Vec3::new((hk - 0.5) * 0.28, 0.0, (hash(hy, hx + k as f64) as f32 - 0.5) * 0.28);
                block(
                    out,
                    c + off + Vec3::Y * (top + 0.05 + hk * 0.07),
                    Vec3::new(0.18, 0.1 + hk * 0.1, 0.2),
                    hk * 4.0,
                    jit(CHARCOAL[k], hk as f64, 0.1),
                    SHADE_STONE,
                );
            }
            let a = hash(hx, hy) as f32 * TAU;
            for k in 0..2 {
                let d = Vec3::new(a.cos(), 0.0, a.sin()) * (0.6 + k as f32 * 0.32);
                let rot = Quat::from_rotation_y(a + k as f32 * 0.5) * Quat::from_rotation_z(PI / 2.0);
                let col = jit(if k == 0 { UMBER[0] } else { CHARCOAL[1] }, hash(hx + k as f64, hy), 0.08);
                block_m(
                    out,
                    Mat4::from_scale_rotation_translation(Vec3::new(0.27, 0.48, 0.48), rot, c + d + Vec3::Y * 0.17),
                    col,
                    SHADE_STONE,
                );
                block(out, c + d + Vec3::Y * 0.33, Vec3::new(0.3, 0.03, 0.2), a, jit(MOSS[k], hx, 0.1), SHADE_SOFT);
            }
        } else {
            // The capital, an abacus over it, and the brazier.
            let hk = hash(hy, hx);
            block(out, c + Vec3::Y * (top + 0.07), Vec3::new(0.66, 0.14, 0.66), 0.0, jit(PALE_STONE[0], hk, 0.05) * 0.72, SHADE_STONE);
            block(out, c + Vec3::Y * (top + 0.18), Vec3::new(0.58, 0.08, 0.58), 0.05, jit(PALE_STONE[1], hk, 0.05) * 0.68, SHADE_STONE);
            top += 0.22;
            // (Its legs sunk in the capital: a fire bowl on the pillar's head.)
            let s = 0.85;
            let foot = top - 0.5 * s;
            brazier_cubes(out, c + Vec3::Y * foot, s, false);
            let flame = c + Vec3::Y * (foot + 0.84 * s);
            self.torches.insert(P::new(x, y), (flame - Vec3::Y * 0.5, (start, 0)));
            self.smoke.push((flame + Vec3::Y * 0.2, 0.9));
            top = flame.y + 0.15;
        }
        let len = self.static_cubes.len() - start;
        if let Some(t) = self.torches.get_mut(&P::new(x, y)) {
            t.1 = (start, len);
        }
        let shades = self.static_cubes[start..start + len].iter().map(|c| c.emissive[3]).collect();
        self.tree_index.insert(tile_key(x, y), self.trees.len());
        self.trees.push(Tree { range: (start, len), base: c, radius: 0.42, height: top, fade: 1.0, alive: true, shades: Some(shades) });
        self.authored.push((start, len));
    }

    /// One tile of a low wall: the crypt's dressed stone on both faces (and the ends of a run),
    /// a pale coping chipped and mossed, now and then a course fallen, the stones at its foot;
    /// bramble against it; at the end of a run, a lantern on an iron crook.
    fn low_wall(&mut self, grid: &Grid, x: i32, y: i32) {
        let rock = |x: i32, y: i32| grid.get(x, y) == Tile::Rock;
        let (xf, yf) = (x as f32, y as f32);
        let (hx, hy) = (x as f64, y as f64);
        let horizontal = rock(x - 1, y) || rock(x + 1, y);
        let ruined = hash(hx * 2.7 + 1.0, hy * 3.3) < 0.35;
        let c = Vec3::new(xf, 0.0, yf);
        let start = self.static_cubes.len();
        let out = &mut self.static_cubes;
        // The core, its top dark under the coping.
        let core = if horizontal { Vec3::new(1.0, LOW_WALL_H - 0.04, 0.22) } else { Vec3::new(0.22, LOW_WALL_H - 0.04, 1.0) };
        block(out, c + Vec3::Y * (core.y / 2.0), core, 0.0, jit(CHARCOAL[1], hx, 0.05) * 0.6, SHADE_STONE);
        let faces: Vec<(i32, i32)> = if horizontal { vec![(0, -1), (0, 1)] } else { vec![(-1, 0), (1, 0)] };
        for d in faces {
            masonry(
                out,
                &Wall {
                    centre: c,
                    dir: d,
                    half: 0.5,
                    base: 0.0,
                    height: LOW_WALL_H,
                    courses: 2,
                    seed: (hx * 1.9 + d.0 as f64 * 3.0, hy * 2.1 + d.1 as f64 * 5.0),
                    len: (0.24, 0.2),
                    depth: (0.05, 0.06),
                    face: 0.1,
                    wet: false,
                    ruined,
                    foot: Some(0.0),
                    openings: &[],
                },
            );
        }
        // The ends of a run.
        let ends: Vec<(i32, i32)> = if horizontal { vec![(-1, 0), (1, 0)] } else { vec![(0, -1), (0, 1)] };
        let mut end_of_run = None;
        for d in ends {
            if rock(x + d.0, y + d.1) {
                continue;
            }
            end_of_run = Some(d);
            masonry(
                out,
                &Wall {
                    centre: c,
                    dir: d,
                    half: 0.14,
                    base: 0.0,
                    height: LOW_WALL_H,
                    courses: 2,
                    seed: (hx * 3.7 + d.0 as f64, hy * 1.3 + d.1 as f64),
                    len: (0.12, 0.1),
                    depth: (0.04, 0.05),
                    face: 0.44,
                    wet: false,
                    ruined: false,
                    foot: Some(0.0),
                    openings: &[],
                },
            );
        }
        // The coping: two pale slabs, chipped; on a ruined stretch only one, the other fallen.
        let turn = if horizontal { 0.0 } else { PI / 2.0 };
        for k in 0..2 {
            let hk = hash(hx * 3.0 + k as f64, hy * 11.0);
            if ruined && k == (hk * 2.0) as usize % 2 {
                continue;
            }
            let u = -0.25 + k as f32 * 0.5;
            let at = if horizontal { Vec3::new(xf + u, LOW_WALL_H + 0.02, yf) } else { Vec3::new(xf, LOW_WALL_H + 0.02, yf + u) };
            let chip = if hk < 0.35 { 0.05 } else { 0.0 };
            block(
                out,
                at,
                Vec3::new(0.48 - chip, 0.07, 0.36),
                turn + (hk as f32 - 0.5) * 0.06,
                jit(PALE_STONE[k], hk, 0.06) * 0.7,
                SHADE_STONE,
            );
        }
        // Moss in the coping's joints and creeping over its edge.
        for k in 0..3 {
            let (a, b) = (hash(hx * 5.3 + k as f64, hy * 2.9) as f32, hash(hy * 4.7 + k as f64, hx * 3.1) as f32);
            if a > 0.6 {
                continue;
            }
            let along = (a - 0.3) * 1.4;
            let across = if b < 0.5 { -0.16 } else { 0.16 };
            let at = if horizontal {
                Vec3::new(xf + along, LOW_WALL_H + 0.06, yf + across)
            } else {
                Vec3::new(xf + across, LOW_WALL_H + 0.06, yf + along)
            };
            block(out, at, Vec3::new(0.12 + b * 0.08, 0.025, 0.07), turn + a, jit(MOSS[k], b as f64, 0.1) * 0.85, SHADE_SOFT);
        }
        // Bramble against one face now and then.
        if hash(hx * 4.3, hy * 2.1 + 9.0) < 0.5 {
            let side = if hash(hy, hx * 3.0) < 0.5 { -1.0 } else { 1.0 };
            let off = if horizontal {
                Vec3::new((hash(hx, hy * 7.0) as f32 - 0.5) * 0.5, 0.0, side * 0.3)
            } else {
                Vec3::new(side * 0.3, 0.0, (hash(hx, hy * 7.0) as f32 - 0.5) * 0.5)
            };
            bramble(out, c + off, 0.3, 0.3, (hx * 2.0, hy * 3.0));
        }
        // A lantern on a short iron crook at the end of the run nearer the arena's middle.
        // (Close by the track up the middle, the end away from it, so the way in from the south
        // is not walked between two lamps.)
        let near_track = (x - ARENA_GATE.0).abs() <= 2 && y > STREAM_ROW;
        let toward =
            |d: (i32, i32)| ((d.0 != 0 && (d.0 > 0) == (x < ARENA_GATE.0)) != near_track) || (d.1 != 0 && (d.1 > 0) == (y < STREAM_ROW));
        if let Some((dx, dy)) = end_of_run.filter(|&d| toward(d)) {
            let foot = c + Vec3::new(dx as f32 * 0.3, LOW_WALL_H + 0.05, dy as f32 * 0.3);
            let iron = lin(IRON).lerp(lin(0x5a3a26), 0.3);
            block(out, foot + Vec3::Y * 0.27, Vec3::new(0.045, 0.54, 0.045), 0.0, iron, SHADE_IRON);
            block(out, foot + Vec3::Y * 0.04, Vec3::new(0.12, 0.08, 0.12), 0.0, iron, SHADE_IRON);
            let arm = Vec3::new(dx as f32, 0.0, dy as f32);
            block(
                out,
                foot + Vec3::Y * 0.54 + arm * 0.1,
                Vec3::new(0.22 * arm.x.abs() + 0.03, 0.03, 0.22 * arm.z.abs() + 0.03),
                0.0,
                iron,
                SHADE_IRON,
            );
            let glass = lantern_cubes(out, foot + Vec3::Y * 0.52 + arm * 0.2, 0.03);
            dim_glass(out);
            let len = self.static_cubes.len() - start;
            self.torches.insert(P::new(x, y), (glass - Vec3::Y * 0.5, (start, len)));
        }
    }

    /// A tile of the boundary wall round the arena, ruined and grown over: the crypt's stone on
    /// every face that looks onto open ground, a hedgerow of hawthorn and bramble on top of it
    /// (its new growth catching the low sun, a blossom or a haw here and there), and where the
    /// wall has fallen, rubble and moss; on the side walls by the gaps, a lantern on an iron post.
    fn ring_wall(&mut self, grid: &Grid, x: i32, y: i32) {
        let (xf, yf) = (x as f32, y as f32);
        let (hx, hy) = (x as f64, y as f64);
        let c = Vec3::new(xf, 0.0, yf);
        let r = |k: f64| hash(hx * 3.1 + k * 1.37, hy * 2.3 + k * 0.71) as f32;
        let fallen = r(0.0) < 0.22;
        let base = if fallen { 0.3 } else { 0.42 } + (r(1.0) - 0.5) * 0.06;
        let start = self.static_cubes.len();
        let out = &mut self.static_cubes;
        block(
            out,
            c + Vec3::Y * ((base - 0.04) / 2.0),
            Vec3::new(0.92, base - 0.04, 0.92),
            0.0,
            jit(CHARCOAL[2], hx, 0.05) * 0.5,
            SHADE_STONE,
        );
        let mut faces = Vec::new();
        for d in CARDINALS {
            let (dx, dy) = d.vec();
            let (nx, ny) = (x + dx, y + dy);
            if !grid.in_bounds(nx, ny) || grid.get(nx, ny) == Tile::Rock {
                continue;
            }
            faces.push((dx, dy));
            masonry(
                out,
                &Wall {
                    centre: c,
                    dir: (dx, dy),
                    half: 0.5,
                    base: 0.0,
                    height: base,
                    courses: 2,
                    seed: (hx * 2.9 + dx as f64 * 3.0, hy * 1.7 + dy as f64 * 5.0),
                    len: (0.26, 0.22),
                    depth: (0.05, 0.07),
                    face: 0.43,
                    wet: false,
                    ruined: fallen,
                    foot: Some(0.0),
                    openings: &[],
                },
            );
        }
        if fallen {
            // Rubble and moss where the wall has come down, bramble in it.
            for k in 0..4 {
                let (a, b) = (r(10.0 + k as f64), r(20.0 + k as f64));
                block(
                    out,
                    c + Vec3::new((a - 0.5) * 0.6, base + 0.03, (b - 0.5) * 0.6),
                    Vec3::splat(0.12 + a * 0.09),
                    a * 5.0,
                    jit(CHARCOAL[k % 3], b as f64, 0.1),
                    SHADE_STONE,
                );
            }
            moss_cap(out, c + Vec3::Y * base, 0.9, (hx * 1.3, hy * 0.7));
            bramble(out, c + Vec3::Y * base, 0.7, 0.32, (hx * 0.9, hy * 1.1));
        } else {
            // The hedgerow: a dark heart, its faces in clumps standing out their own depth, the top
            // uneven, sprays reaching up out of it.
            let top = base + 0.48 + (r(2.0) - 0.5) * 0.1;
            block(out, c + Vec3::Y * ((base + top) / 2.0), Vec3::new(0.8, top - base, 0.8), r(3.0) * 0.2, lin(HEDGE[2]) * 0.5, SHADE_SOFT);
            let mut k = 4.0;
            for &(dx, dy) in &faces {
                let at = side_at(c, (dx, dy));
                let turn = side_turn((dx, dy));
                for tier in 0..2 {
                    let mut u = -0.5 - tier as f32 * 0.08;
                    while u < 0.5 {
                        k += 1.0;
                        let len = 0.2 + r(k) * 0.16;
                        let (a, b) = (u.max(-0.5), (u + len).min(0.5));
                        u += len;
                        if b - a < 0.06 {
                            continue;
                        }
                        let (y0, y1) = if tier == 0 { (base - 0.06, base + 0.26) } else { (base + 0.2, top) };
                        let depth = 0.14 + r(k + 0.5) * 0.1;
                        let front = 0.47 + r(k + 0.3) * 0.06 - tier as f32 * 0.03;
                        let lit = r(k + 2.1) < 0.22;
                        let tone = if lit { lin(HEDGE_LIT) } else { lin(HEDGE[(r(k + 1.2) * 3.0) as usize % 3]) };
                        let rot = Quat::from_rotation_y(turn + (r(k + 1.8) - 0.5) * 0.2) * Quat::from_rotation_x((r(k + 2.4) - 0.5) * 0.2);
                        block_m(
                            out,
                            Mat4::from_scale_rotation_translation(
                                Vec3::new(b - a + 0.04, y1 - y0, depth),
                                rot,
                                at((a + b) / 2.0, (y0 + y1) / 2.0, front - depth / 2.0),
                            ),
                            tone * (0.86 + r(k + 1.5) * 0.28) * if tier == 0 { 0.75 } else { 1.0 },
                            SHADE_SOFT,
                        );
                        if r(k + 3.3) < 0.12 {
                            let p = at((a + b) / 2.0 + (r(k + 3.5) - 0.5) * 0.1, y0 + (y1 - y0) * (0.4 + r(k + 3.7) * 0.5), front + 0.01);
                            block(out, p, Vec3::splat(0.04), r(k + 3.9) * 3.0, lin(HAW[(r(k + 4.1) * 2.0) as usize % 2]), SHADE_SOFT);
                        }
                    }
                }
            }
            // Its top: clumps in a loose three by three, each its own height and tone, spilling a
            // little past the wall's line, so from above it reads as a mound of growth, not a box.
            for j in 0..9 {
                let (a, b) = (r(40.0 + j as f64), r(50.0 + j as f64));
                let (gx, gz) = ((j % 3) as f32 - 1.0, (j / 3) as f32 - 1.0);
                let s = 0.3 + a * 0.16;
                let h = 0.08 + b * 0.16;
                let tone = if (a * 7.0).fract() < 0.35 { lin(HEDGE_LIT) } else { lin(HEDGE[j % 3]) * 1.15 };
                block(
                    out,
                    c + Vec3::new(gx * 0.31 + (a - 0.5) * 0.14, top - 0.06 + h / 2.0, gz * 0.31 + (b - 0.5) * 0.14),
                    Vec3::new(s, h, s * 0.9),
                    a * 3.0,
                    tone * (0.85 + b * 0.3),
                    SHADE_SOFT,
                );
            }
            // Sprays reaching up out of it.
            for j in 0..2 {
                if r(60.0 + j as f64) < 0.4 {
                    continue;
                }
                let (a, b) = (r(61.0 + j as f64), r(62.0 + j as f64));
                let tall = 0.14 + a * 0.16;
                let lean = Quat::from_rotation_y(b * 6.0) * Quat::from_rotation_z((a - 0.5) * 0.7);
                block_m(
                    out,
                    Mat4::from_scale_rotation_translation(
                        Vec3::new(0.08, tall, 0.07),
                        lean,
                        c + Vec3::new((a - 0.5) * 0.5, top + tall * 0.4, (b - 0.5) * 0.5),
                    ),
                    lin(HEDGE_LIT),
                    SHADE_SOFT,
                );
            }
        }
        // A lantern on an iron post at the side walls by the gaps, over the arena.
        let over = faces.iter().copied().find(|&(dx, dy)| in_arena((x + dx) as f32, (y + dy) as f32));
        let side_gap = (x == 2 || x == grid.width - 3) && (y == 5 || y == grid.height - 7);
        if let (Some((dx, dy)), true) = (over, side_gap) {
            let dir = Vec3::new(dx as f32, 0.0, dy as f32);
            let foot = c + dir * 0.38 + Vec3::Y * base;
            let iron = lin(IRON).lerp(lin(0x5a3a26), 0.3);
            block(out, foot + Vec3::Y * 0.4, Vec3::new(0.05, 0.8, 0.05), 0.0, iron, SHADE_IRON);
            block(
                out,
                foot + Vec3::Y * 0.8 + dir * 0.1,
                Vec3::new(0.22 * dir.x.abs() + 0.03, 0.03, 0.22 * dir.z.abs() + 0.03),
                0.0,
                iron,
                SHADE_IRON,
            );
            let glass = lantern_cubes(out, foot + Vec3::Y * 0.78 + dir * 0.2, 0.03);
            dim_glass(out);
            let len = self.static_cubes.len() - start;
            self.torches.insert(P::new(x, y), (glass - Vec3::Y * 0.5, (start, len)));
        }
    }

    /// One tile of the stream: its bed and stones, its banks (dressed stone near the bridges and
    /// at its ends, earth and reeds where it runs wild), and on a bridge tile the bridge; at its
    /// east end the spring head pouring into it, at its west the culvert that takes it.
    fn stream_tile(&mut self, grid: &Grid, x: i32, y: i32) {
        let (xf, yf) = (x as f32, y as f32);
        let (hx, hy) = (x as f64, y as f64);
        let water = |x: i32, y: i32| grid.get(x, y) == Tile::Pit || grid.is_bridge(x, y);
        let bridge = grid.is_bridge(x, y);
        let surface = -WATER_DROP;
        let c = Vec3::new(xf, 0.0, yf);
        let out = &mut self.static_cubes;
        // The bed: dark mud, stones, a few big ones breaking the surface.
        block(out, Vec3::new(xf, BED - 0.05, yf), Vec3::new(1.0, 0.1, 1.0), 0.0, jit(MUD, hash(hx, hy), 0.1), SHADE_GRAVEL);
        for k in 0..5 {
            let (a, b) = (hash(hx * 7.0 + k as f64, hy * 3.0) as f32, hash(hy * 5.0 + k as f64, hx * 2.0) as f32);
            let s = 0.07 + a * 0.08;
            block(
                out,
                Vec3::new(xf + (a - 0.5) * 0.8, BED + s * 0.25, yf + (b - 0.5) * 0.6),
                Vec3::new(s * 1.2, s * 0.6, s),
                a * 3.0,
                jit(CHARCOAL[k % 3], b as f64, 0.1) * 0.7,
                SHADE_FLAG,
            );
        }
        if !bridge && hash(hx * 3.3 + 2.0, hy) < 0.3 {
            let a = hash(hx, hy * 9.0) as f32;
            block(
                out,
                Vec3::new(xf + (a - 0.5) * 0.5, surface - 0.04, yf + (a - 0.5) * 0.3),
                Vec3::new(0.24, 0.2, 0.2),
                a * 4.0,
                jit(CHARCOAL[1], a as f64, 0.08) * 0.8,
                SHADE_FLAG,
            );
        }
        // Built banks by the bridges and at the ends; wild ones between.
        let near_bridge = BRIDGES.iter().any(|&b| (b - x).abs() <= 1);
        let (x0, _, x1, _) = ARENA;
        let built = near_bridge || x == x0 || x == x1;
        for d in CARDINALS {
            let (dx, dy) = d.vec();
            let (nx, ny) = (x + dx, y + dy);
            if water(nx, ny) {
                continue;
            }
            let at = side_at(c, (dx, dy));
            let turn = side_turn((dx, dy));
            let seed = (hx * 2.3 + dx as f64, hy * 1.7 + dy as f64);
            let end = dy == 0;
            if built || bridge || end {
                let start_bank = out.len();
                let top = if end { 0.55 } else { 0.03 };
                let openings: &[(f32, f32, f32, f32, f32)] = if end && dx < 0 { &[(0.0, 0.26, 0.0, 0.3, 0.2)] } else { &[] };
                masonry(
                    out,
                    &Wall {
                        centre: Vec3::new(nx as f32, 0.0, ny as f32),
                        dir: (-dx, -dy),
                        half: 0.5,
                        base: BED,
                        height: top - BED,
                        courses: if end { 4 } else { 2 },
                        seed,
                        len: (0.24, 0.2),
                        depth: (0.04, 0.05),
                        face: 0.43,
                        wet: true,
                        ruined: false,
                        foot: None,
                        openings,
                    },
                );
                for k in 0..2 {
                    let hk = hash(hx * 3.1 + k as f64 + dx as f64, hy * 1.9 + dy as f64);
                    block(
                        out,
                        at(-0.25 + k as f32 * 0.5, top + 0.03, 0.45),
                        Vec3::new(0.49, 0.06, 0.2),
                        turn + (hk as f32 - 0.5) * 0.04,
                        jit(PALE_STONE[k], hk, 0.05) * 0.68,
                        SHADE_STONE,
                    );
                }
                if hash(hx * 5.0 + dx as f64, hy * 3.0 + dy as f64) < 0.5 {
                    block(out, at(0.1, top + 0.065, 0.42), Vec3::new(0.4, 0.02, 0.1), turn, jit(MOSS[1], hx, 0.1), SHADE_SOFT);
                }
                if end && dx > 0 {
                    // The spring head: a mask in the wall pouring into the stream, a lantern over it.
                    let face = Vec3::new(xf + 0.5 - 0.04, 0.2, yf);
                    let lip = spout_cubes(out, face, Vec3::new(-1.0, 0.0, 0.0), seed);
                    self.spouts.push((lip, Vec3::new(-1.0, 0.0, 0.0), surface));
                    // (On an iron post at the wall's corner, high, so its pool lies on the banks and
                    // the water without a glare under it.)
                    let foot = Vec3::new(xf + 0.62, 0.55, yf + 0.36);
                    let iron = lin(IRON).lerp(lin(0x5a3a26), 0.3);
                    block(out, foot + Vec3::Y * 0.4, Vec3::new(0.05, 0.8, 0.05), 0.0, iron, SHADE_IRON);
                    block(out, foot + Vec3::new(-0.1, 0.8, 0.0), Vec3::new(0.25, 0.03, 0.03), 0.0, iron, SHADE_IRON);
                    let glass = lantern_cubes(out, foot + Vec3::new(-0.2, 0.78, 0.0), 0.03);
                    dim_glass(out);
                    self.torches.insert(P::new(x, y), (glass - Vec3::Y * 0.5, (start_bank, out.len() - start_bank)));
                } else if end {
                    // The culvert: its dark mouth, and an iron grate across it.
                    block(out, Vec3::new(xf - 0.62, BED + 0.28, yf), Vec3::new(0.1, 0.56, 0.6), 0.0, lin(0x070808), SHADE_LIT);
                    for k in 0..5 {
                        let u = -0.2 + k as f32 * 0.1;
                        block(out, Vec3::new(xf - 0.48, BED + 0.3, yf + u), Vec3::new(0.025, 0.6, 0.025), 0.0, lin(IRON), SHADE_IRON);
                    }
                    block(out, Vec3::new(xf - 0.48, surface + 0.1, yf), Vec3::new(0.03, 0.025, 0.56), 0.0, lin(IRON), SHADE_IRON);
                }
                continue;
            }
            // A wild bank: the cut earth, irregular, a lip of turf over it with grass hanging down,
            // wet stones at the waterline, and reeds.
            let mut u = -0.5;
            let mut k = 0;
            while u < 0.5 {
                let hk = hash(hx * 4.1 + k as f64 + dx as f64 * 3.0, hy * 2.7 + dy as f64 * 5.0) as f32;
                let len = (0.22 + hk * 0.2).min(0.5 - u + 0.01);
                let depth = 0.1 + hk * 0.12;
                let top = -0.025 - hk * 0.04;
                block(
                    out,
                    at(u + len / 2.0, (BED + top) / 2.0, 0.5 - depth / 2.0),
                    Vec3::new(len + 0.01, top - BED, depth),
                    turn + (hk - 0.5) * 0.08,
                    jit(BANK, hk as f64, 0.12),
                    SHADE_GRAVEL,
                );
                // The wet band where the water laps it.
                block(
                    out,
                    at(u + len / 2.0, surface + 0.02, 0.5 - depth - 0.005),
                    Vec3::new(len, 0.05, 0.02),
                    turn,
                    lin(0x1c1812),
                    SHADE_FLAG,
                );
                u += len;
                k += 1;
            }
            block(out, at(0.0, -0.01, 0.48), Vec3::new(1.0, 0.04, 0.1), turn, turf_tone(0.3, 0.0, 0.5) * 0.75, SHADE_GRASS);
            for j in 0..3 {
                let hj = hash(hx * 6.3 + j as f64 + dy as f64, hy * 2.2 + dx as f64) as f32;
                block_m(
                    out,
                    Mat4::from_scale_rotation_translation(
                        Vec3::new(0.1 + hj * 0.08, 0.14, 0.03),
                        Quat::from_rotation_y(turn) * Quat::from_rotation_x(0.35 * dy as f32),
                        at((hj - 0.5) * 0.85, -0.06, 0.42),
                    ),
                    turf_tone(0.4, 0.0, hj) * 0.9,
                    SHADE_SOFT,
                );
            }
            for j in 0..3 {
                let hj = hash(hx * 3.7 + j as f64, hy * 6.1 + dx as f64 + dy as f64) as f32;
                if hj > 0.65 {
                    continue;
                }
                let s = 0.08 + hj * 0.1;
                block(
                    out,
                    at((hj - 0.3) * 1.2, surface + 0.01, 0.3 - hj * 0.08),
                    Vec3::new(s * 1.3, s * 0.7, s),
                    hj * 5.0,
                    jit(CHARCOAL[j], hj as f64, 0.1) * 0.65,
                    SHADE_FLAG,
                );
            }
            if hash(hx * 11.0 + dx as f64, hy * 13.0 + dy as f64) < 0.5 {
                let at0 = (hash(hx * 2.0 + dy as f64, hy) as f32 - 0.5) * 0.6;
                for b in 0..8 {
                    let hb = hash(hx * 17.0 + b as f64, hy + dy as f64 * 3.0 + dx as f64) as f32;
                    let tall = 0.3 + hb * 0.28;
                    let base = at(at0 + (hb - 0.5) * 0.36, surface, 0.33 + (b % 3) as f32 * 0.04);
                    let lean = Quat::from_rotation_y(hb * 6.0) * Quat::from_rotation_z((hb - 0.5) * 0.35);
                    block_m(
                        out,
                        Mat4::from_scale_rotation_translation(Vec3::new(0.014, tall, 0.014), lean, base + lean * Vec3::Y * (tall / 2.0)),
                        jit(REED[b % 2], hb as f64, 0.12),
                        SHADE_SOFT,
                    );
                    if b % 3 == 0 {
                        block_m(
                            out,
                            Mat4::from_scale_rotation_translation(
                                Vec3::new(0.028, 0.08, 0.028),
                                lean,
                                base + lean * Vec3::Y * (tall * 0.84),
                            ),
                            lin(CATTAIL),
                            SHADE_SOFT,
                        );
                    } else if b % 3 == 1 {
                        block_m(
                            out,
                            Mat4::from_scale_rotation_translation(
                                Vec3::new(0.04, tall * 0.7, 0.006),
                                lean * Quat::from_rotation_x(0.3),
                                base + Vec3::Y * (tall * 0.3),
                            ),
                            jit(REED[0], hb as f64, 0.1) * 1.1,
                            SHADE_SOFT,
                        );
                    }
                }
            }
        }
        if bridge {
            self.timber_bridge(x, y);
        }
    }

    /// An old timber bridge over the stream on the tile at `(x, y)`, the path crossing it along z:
    /// planks silvered and worn on two stringers, posts and rails, iron straps, moss on the posts'
    /// heads. The middle bridge has a lantern hung from a crook on its post.
    fn timber_bridge(&mut self, x: i32, y: i32) {
        let (xf, yf) = (x as f32, y as f32);
        let (hx, hy) = (x as f64, y as f64);
        let start = self.static_cubes.len();
        let out = &mut self.static_cubes;
        for i in 0..7 {
            let z = -0.56 + i as f32 * 0.187;
            let arch = (1.0 - (z / 0.62).powi(2)).max(0.0) * 0.04;
            let hk = hash(hx + i as f64 * 1.3, hy * 2.0);
            let tilt = (hk as f32 - 0.5) * 0.05;
            let w = 0.9 + hk as f32 * 0.06;
            block(
                out,
                Vec3::new(xf + tilt * 0.4, 0.012 + arch, yf + z),
                Vec3::new(w, 0.05, 0.17),
                tilt,
                jit(TIMBER[(hk * 3.0) as usize % 3], hk, 0.1),
                SHADE_WOOD,
            );
        }
        for sx in [-0.36f32, 0.36] {
            block(out, Vec3::new(xf + sx, -0.07, yf), Vec3::new(0.1, 0.12, 1.36), 0.0, jit(TIMBER[1], hx, 0.06) * 0.7, SHADE_WOOD);
        }
        let iron = lin(IRON).lerp(lin(0x5a3a26), 0.35);
        for sx in [-0.47f32, 0.47] {
            for (k, sz) in [-0.52f32, 0.0, 0.52].into_iter().enumerate() {
                let hk = hash(hx * 3.0 + k as f64 + sx as f64, hy * 5.0);
                let tall = 0.52 + hk as f32 * 0.06;
                block(
                    out,
                    Vec3::new(xf + sx, tall / 2.0 - 0.08, yf + sz),
                    Vec3::new(0.08, tall + 0.16, 0.08),
                    (hk as f32 - 0.5) * 0.1,
                    jit(TIMBER[k % 3], hk, 0.1) * 0.85,
                    SHADE_WOOD,
                );
                block(out, Vec3::new(xf + sx, 0.06, yf + sz), Vec3::new(0.1, 0.04, 0.1), 0.0, iron, SHADE_IRON);
                if hk < 0.6 {
                    block(
                        out,
                        Vec3::new(xf + sx, tall - 0.06, yf + sz),
                        Vec3::new(0.09, 0.03, 0.09),
                        0.0,
                        jit(MOSS[k % 3], hk, 0.1),
                        SHADE_SOFT,
                    );
                }
            }
            block(out, Vec3::new(xf + sx, 0.44, yf), Vec3::new(0.06, 0.06, 1.16), 0.0, jit(TIMBER[0], hy, 0.08), SHADE_WOOD);
            block(out, Vec3::new(xf + sx, 0.24, yf), Vec3::new(0.04, 0.045, 1.08), 0.0, jit(TIMBER[2], hx, 0.08) * 0.9, SHADE_WOOD);
        }
        if x == ARENA_GATE.0 {
            // A lantern hung over the water from the middle bridge's post, its light on the wet planks.
            let post = Vec3::new(xf + 0.47, 0.0, yf + 0.52);
            block(out, post + Vec3::Y * 0.85, Vec3::new(0.06, 0.6, 0.06), 0.0, jit(TIMBER[1], hx, 0.06), SHADE_WOOD);
            block(out, post + Vec3::new(0.12, 1.12, 0.0), Vec3::new(0.3, 0.04, 0.04), 0.0, iron, SHADE_IRON);
            let glass = lantern_cubes(out, post + Vec3::new(0.24, 1.1, 0.0), 0.05);
            dim_glass(out);
            let len = self.static_cubes.len() - start;
            self.torches.insert(P::new(x, y), (glass - Vec3::Y * 0.5, (start, len)));
        }
    }

    /// Mist lying on the stream (thick) and in the treeline (thin), mist teal greyed; none over
    /// the meadow itself, where it would veil the turf (the haze does the rest).
    fn meadow_mist(&mut self, grid: &Grid) {
        self.mist.clear();
        let tone = lin(MIST);
        for (x, y, t) in grid.cells() {
            let stream = t == Tile::Pit || grid.is_bridge(x, y);
            let edge = !in_arena(x as f32, y as f32);
            let layers: &[(f32, f32)] = if stream {
                &[(-0.07, 0.3), (0.14, 0.2)]
            } else if edge {
                &[(0.12, 0.22), (0.45, 0.14)]
            } else {
                continue;
            };
            for (k, &(lift, alpha)) in layers.iter().enumerate() {
                let m = Mat4::from_translation(Vec3::new(x as f32, lift, y as f32)) * Mat4::from_scale(Vec3::new(1.0, 0.002, 1.0));
                self.mist.push(Instance::new(m, tone, alpha, Vec3::new(20.0 + k as f32, lift, 0.0), SHADE_CLOUD));
            }
        }
    }
}

/// The meadow's practicals: amber, each a pool of its own (RP17 bins them all).
pub(super) const MEADOW_LAMPS_LIT: usize = 10;

/// An ember-lit copper and amber of the braziers and lanterns (shared with `emit`).
pub(super) fn lamp_colour() -> Vec3 {
    lin(super::estate::pal::LANTERN)
}
