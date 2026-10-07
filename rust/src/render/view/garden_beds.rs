//! The Garden of Krampus at the Cisterns' bar (RENDERING.md, Stage B, "The Garden, at the
//! Cisterns' bar"): the maze's walls become beds ringed by clipped yew, its floors gravel walks
//! with turf verges and wet flagstone avenues, lit by amber lanterns on iron posts.
//!
//! What it replaced: every floor was one even lawn, every wall a low mound behind a dry-stone
//! fence with voxel bushes and little voxel trees, all of it plain-lit (no `voxel_surface`
//! material: RP14) and graded after the fact. Here every piece is built in the board's palette
//! with its material: dressed stone (the crypt's `masonry`) for the terraces' retaining walls,
//! steps and kerbs, wet flagstone (`SHADE_FLAG`) and gravel (`SHADE_GRAVEL`) underfoot, strand-shaded growth
//! (`SHADE_SOFT`) for yew, turf tufts and leaves, bark (`SHADE_WOOD`) and iron (`SHADE_IRON`).
//!
//! The layout is untouched: the same tiles are floor and wall, the hedges stand where the fences
//! stood and no higher than a fence and its stones did (sight lines unchanged), and a broken wall
//! takes its hedge, bed and tree with it (`rock_index`, `tree_index`).

use super::crypt::pal::{CHARCOAL, MOSS, PALE_STONE, UMBER};
use super::crypt::{
    block, block_m, flagstones, jit, lantern_cubes, masonry, moss_seam, Wall, SHADE_IRON, SHADE_SOFT, SHADE_STONE, SHADE_WOOD,
};
use super::garden::Feature;
use super::*;

pub(super) mod pal {
    /// Clipped yew: a deep blue-green, darkest at the base where the light never reaches, the
    /// clipped top a little paler where the moon lies on it.
    pub const YEW: [u32; 3] = [0x29472a, 0x31532f, 0x253f26];
    pub const YEW_TOP: [u32; 2] = [0x48683a, 0x415f35];
    /// The new growth at the tips, which catches the lanterns.
    pub const YEW_TIP: u32 = 0x5e7e40;
    pub const YEW_BASE: u32 = 0x15231a;
    /// Turf in the board's moss and olive, and the beds' dark earth.
    pub const TURF: [u32; 3] = [0x4c7a2e, 0x588834, 0x446e2c];
    pub const EARTH: [u32; 2] = [0x2e261e, 0x372c22];
    /// The walks' gravel: umber greys, wet.
    pub const GRAVEL: [u32; 1] = [0x3e3830];
    /// Fallen leaves: copper and umber.
    pub const LEAVES: [u32; 3] = [0x8a4a22, 0x9a6a2a, 0x5e3a1e];
    pub const BARK: [u32; 2] = [0x3e3128, 0x30271f];
    /// Broadleaf crowns: moss and olive, lit olive on top.
    pub const LEAF: [u32; 3] = [0x283326, 0x2e3a2a, 0x37432f];
    pub const LEAF_LIT: u32 = 0x4c5838;
    /// Blooms, quiet: rust rose, amber, dusk lavender, pale stone.
    pub const BLOOMS: [u32; 4] = [0x8a3e34, 0xa8803e, 0x6e5a82, 0xb8ae94];
    pub const PINE: [u32; 2] = [0x1d2f23, 0x25382a];
    /// The woods beyond the walls: darker still, a silhouette against the sky.
    pub const WOODS: [u32; 2] = [0x16241b, 0x1b2b20];
    pub const RUST: u32 = 0x5a3a26;
    pub const IRON: u32 = 0x2a2624;
}
use pal::*;

/// Wet gravel (`voxel_surface` kind 11: `gravel` in the shader).
pub(super) const SHADE_GRAVEL: f32 = 16.0;
/// How tall the yew stands over its bed: the old fence's top (its mound, two courses and the
/// cap), so what the cameras see past is unchanged.
pub(super) const HEDGE_H: f32 = 0.58;
/// How deep a hedge reaches in from the tile's edge.
const HEDGE_T: f32 = 0.42;
/// A bed's earth stands this far over its tile's floor.
const BED: f32 = 0.05;
/// The lawn along a walk, from the hedge's foot (a 2-tile walk keeps a little over half its
/// width in gravel), and its darker strip in the hedge's shadow.
const VERGE: f32 = 0.42;
const FOOT: f32 = 0.13;

/// Turf in one of its tones, picked by `k`.
fn turf(k: f32) -> Vec3 {
    jit(TURF[(k * 7.0) as usize % 3], k as f64, 0.1)
}
/// Lanterns stand at least this far apart (tiles, either axis), so each is a pool of its own.
const LANTERN_GAP: i32 = 6;

/// A frame along one side of a tile: `u` runs along the side, `o` out from the tile's middle
/// toward `d`, heights over `c.y`.
fn side_at(c: Vec3, (dx, dy): (i32, i32)) -> impl Fn(f32, f32, f32) -> Vec3 {
    move |u: f32, y: f32, o: f32| {
        Vec3::new(c.x + dx as f32 * o + if dx != 0 { 0.0 } else { u }, c.y + y, c.z + dy as f32 * o + if dx != 0 { u } else { 0.0 })
    }
}

fn side_turn((dx, _): (i32, i32)) -> f32 {
    if dx != 0 {
        PI / 2.0
    } else {
        0.0
    }
}

/// One side of a bed in clipped yew: a dark core, the darkest band at its foot, the face in
/// clumps of two tiers each standing out its own depth, the top in slabs at uneven heights, and
/// now and then a sprig the shears missed. `c` is the tile's middle on the bed; `h` its height.
pub(super) fn hedge(out: &mut Vec<Instance>, c: Vec3, d: (i32, i32), h: f32, seed: (f64, f64)) {
    let at = side_at(c, d);
    let turn = side_turn(d);
    let (hx, hy) = seed;
    let r = |k: f64| hash(hx * 3.1 + k * 1.37 + d.0 as f64 * 5.0, hy * 2.3 + k * 0.71 + d.1 as f64 * 7.0) as f32;
    let mid = 0.5 - HEDGE_T / 2.0;
    block(out, at(0.0, (h - 0.06) / 2.0, mid), Vec3::new(1.0, h - 0.06, HEDGE_T - 0.06), turn, lin(YEW[2]) * 0.55, SHADE_SOFT);
    block(out, at(0.0, 0.07, mid + 0.012), Vec3::new(1.0, 0.14, HEDGE_T - 0.02), turn, jit(YEW_BASE, r(1.0) as f64, 0.1), SHADE_SOFT);
    // The face, in clumps of three tiers: the lowest darker and set back, the others fuller,
    // each clump standing out its own depth, a touch turned, some catching more of the light.
    let mut k: f64 = 0.0;
    let tiers = [(0.11f32, 0.27f32, 0.7f32, 0.015f32), (0.25, 0.44, 0.92, 0.0), (0.42, h - 0.04, 1.0, 0.0)];
    for (tier, &(y0, y1, shade, back)) in tiers.iter().enumerate() {
        let mut u: f32 = -0.5 - tier as f32 * 0.07;
        while u < 0.5 {
            k += 1.0;
            let len = 0.17 + r(k) * 0.13;
            let (a, b) = (u.max(-0.5), (u + len).min(0.5));
            u += len;
            if b - a < 0.05 {
                continue;
            }
            let front = 0.5 + r(k + 0.3) * 0.028 - back;
            let depth = 0.12 + r(k + 0.6) * 0.06;
            let lift = (r(k + 0.9) - 0.5) * 0.035;
            let lit = r(k + 2.1) < 0.26;
            let tone = if lit {
                lin(if r(k + 2.2) < 0.4 { YEW_TIP } else { YEW_TOP[(r(k + 1.2) * 2.0) as usize % 2] }) * 0.88
            } else {
                lin(YEW[(r(k + 1.2) * 3.0) as usize % 3])
            };
            let c = tone * (1.0 - 0.14 + r(k + 1.5) * 0.28) * shade;
            let rot = Quat::from_rotation_y(turn + (r(k + 1.8) - 0.5) * 0.09) * Quat::from_rotation_x((r(k + 2.4) - 0.5) * 0.08);
            block_m(
                out,
                Mat4::from_scale_rotation_translation(
                    Vec3::new(b - a + 0.025, y1 - y0 + 0.02, depth),
                    rot,
                    at((a + b) / 2.0, (y0 + y1) / 2.0 + lift, front - depth / 2.0),
                ),
                c,
                SHADE_SOFT,
            );
        }
    }
    // The clipped top: bumps at their own heights, the shears' line wandering, the outer edge
    // rounded over.
    let mut u: f32 = -0.5;
    while u < 0.49 {
        k += 1.0;
        let len = (0.2 + r(k) * 0.14).min(0.5 - u + 0.03);
        let top = h + (r(k + 0.4) - 0.5) * 0.045;
        let depth = HEDGE_T - 0.06 + r(k + 0.7) * 0.05;
        let c = jit(YEW_TOP[(r(k + 1.4) * 2.0) as usize % 2], r(k + 1.7) as f64, 0.14);
        block(
            out,
            at(u + len / 2.0, top - 0.04, 0.49 - depth / 2.0),
            Vec3::new(len + 0.02, 0.08, depth),
            turn + (r(k + 1.1) - 0.5) * 0.06,
            c,
            SHADE_SOFT,
        );
        block(out, at(u + len / 2.0, top - 0.07, 0.505), Vec3::new(len + 0.01, 0.06, 0.04), turn, c * 0.82, SHADE_SOFT);
        u += len;
    }
    // A sprig or two the shears missed.
    for s in 0..2 {
        if r(40.0 + s as f64) < 0.55 {
            continue;
        }
        let uu = (r(41.0 + s as f64) - 0.5) * 0.8;
        let up = r(42.0 + s as f64) < 0.5;
        let p = if up { at(uu, h + 0.03, 0.5 - HEDGE_T * r(43.0) * 0.8) } else { at(uu, h * (0.4 + r(44.0) * 0.4), 0.53) };
        block(out, p, Vec3::new(0.06, 0.08, 0.06), r(45.0) * 3.0, lin(YEW_TIP), SHADE_SOFT);
    }
}

/// A tuft of three blades of grass at `p`.
fn tuft(out: &mut Vec<Instance>, p: Vec3, k: f32) {
    let tone = jit(TURF[(k * 7.0) as usize % 3], k as f64, 0.1) * 1.25 + lin(0x6a6030) * 0.15;
    for b in 0..3 {
        let hb = hash(k as f64 * 91.0 + b as f64, k as f64 * 37.0 + 3.0) as f32;
        let tall = 0.07 + hb * 0.09;
        let lean = Quat::from_rotation_y(k * 6.0 + b as f32 * 2.1) * Quat::from_rotation_z((hb - 0.5) * 0.7);
        let at = p + Vec3::new((b as f32 - 1.0) * 0.025, tall / 2.0, (hb - 0.5) * 0.04);
        block_m(
            out,
            Mat4::from_scale_rotation_translation(Vec3::new(0.018, tall, 0.018), lean, at),
            tone * (0.85 + 0.1 * b as f32),
            SHADE_SOFT,
        );
    }
}

/// A leaf lying flat at `p`.
fn leaf(out: &mut Vec<Instance>, p: Vec3, k: f32) {
    let m = Mat4::from_scale_rotation_translation(
        Vec3::new(0.06, 0.01, 0.045),
        Quat::from_rotation_y(k * 9.0) * Quat::from_rotation_x((k * 7.0).fract() * 0.4 - 0.2),
        p,
    );
    block_m(out, m, jit(LEAVES[(k * 11.0) as usize % 3], k as f64, 0.15), SHADE_SOFT);
}

/// Which kind of tree grows where.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A broad crown on a real trunk, in tiers: dark under, lit on top.
    Broadleaf,
    /// A clipped yew column.
    Yew,
    /// A pine in flat tiers.
    Pine,
}

/// A tree standing at `base`, `height` tall, its crown at most `reach` from the trunk.
fn garden_tree(out: &mut Vec<Instance>, base: Vec3, kind: Kind, height: f32, reach: f32, woods: bool, seed: f64) {
    let r = |k: f64| hash(seed * 1.7 + k * 3.3, seed * 0.9 - k * 1.1) as f32;
    let dim = if woods { 0.8 } else { 1.0 };
    // The trunk, a little crooked, tapering, with a ridge of bark; roots at its foot.
    let trunk_h = match kind {
        Kind::Broadleaf => height * 0.5,
        Kind::Yew => height * 0.15,
        Kind::Pine => height * 0.35,
    };
    let lean = Vec3::new((r(1.0) - 0.5) * 0.18, 1.0, (r(2.0) - 0.5) * 0.18).normalize();
    let segs = 2;
    let mut p = base;
    for i in 0..segs {
        let w = 0.13 - i as f32 * 0.03 + height * 0.01;
        let l = trunk_h / segs as f32;
        let d = (lean + Vec3::new((r(3.0 + i as f64) - 0.5) * 0.2, 0.0, (r(4.0 + i as f64) - 0.5) * 0.2)).normalize();
        let q = p + d * l;
        let rot = Quat::from_rotation_arc(Vec3::Y, d) * Quat::from_rotation_y(r(5.0) * 2.0);
        let c = jit(BARK[i % 2], r(6.0 + i as f64) as f64, 0.12);
        block_m(out, Mat4::from_scale_rotation_translation(Vec3::new(w, l + w * 0.4, w), rot, (p + q) * 0.5), c, SHADE_WOOD);
        if i == 0 {
            block_m(
                out,
                Mat4::from_scale_rotation_translation(
                    Vec3::new(w * 0.3, l * 0.8, w * 0.7),
                    rot,
                    (p + q) * 0.5 + rot * Vec3::new(w * 0.5, 0.0, 0.0),
                ),
                c * 0.8,
                SHADE_WOOD,
            );
        }
        p = q;
    }
    for k in 0..3 {
        let a = k as f32 / 3.0 * TAU + r(7.0) * 2.0;
        let d = Vec3::new(a.cos(), -0.35, a.sin());
        block_m(
            out,
            Mat4::from_scale_rotation_translation(
                Vec3::new(0.06, 0.2, 0.06),
                Quat::from_rotation_arc(Vec3::Y, d),
                base + d * 0.08 + Vec3::Y * 0.04,
            ),
            lin(BARK[1]) * 0.9,
            SHADE_WOOD,
        );
    }
    let top = p;
    match kind {
        Kind::Broadleaf => {
            // Two boughs out of the fork, then the crown in three tiers of clumps.
            for b in 0..2 {
                let a = r(8.0 + b as f64) * TAU;
                let d = Vec3::new(a.cos() * 0.8, 1.0, a.sin() * 0.8).normalize();
                let l = height * 0.18;
                block_m(
                    out,
                    Mat4::from_scale_rotation_translation(Vec3::new(0.06, l, 0.06), Quat::from_rotation_arc(Vec3::Y, d), top + d * l * 0.5),
                    lin(BARK[0]),
                    SHADE_WOOD,
                );
            }
            let crown = height - trunk_h;
            let tiers: [(f32, f32, usize, u32); 3] = [(0.1, 1.0, 12, LEAF[0]), (0.4, 0.85, 11, LEAF[1]), (0.7, 0.55, 7, LEAF[2])];
            let mut k = 10.0;
            for (t, (y, spread, n, col)) in tiers.into_iter().enumerate() {
                for j in 0..n {
                    k += 1.0;
                    let a = j as f32 / n as f32 * TAU + r(k) * 0.9 + t as f32;
                    let rr = reach * spread * (0.5 + r(k + 0.5) * 0.4);
                    let s = reach * (0.26 + r(k + 0.2) * 0.17) * (1.1 - t as f32 * 0.15);
                    let at = top + Vec3::new(a.cos() * rr, crown * y + (r(k + 0.7) - 0.5) * 0.12, a.sin() * rr);
                    let c = jit(col, r(k + 1.1) as f64, 0.12) * dim;
                    let rot = Quat::from_rotation_y(r(k + 1.3) * 3.0)
                        * Quat::from_rotation_x((r(k + 1.5) - 0.5) * 0.7)
                        * Quat::from_rotation_z((r(k + 1.6) - 0.5) * 0.5);
                    block_m(out, Mat4::from_scale_rotation_translation(Vec3::new(s, s * 0.7, s * 0.9), rot, at), c, SHADE_SOFT);
                }
            }
            // The crown's lit top, in a few small clumps.
            for j in 0..3 {
                let a = j as f32 * 2.1 + r(30.0) * 3.0;
                let cap = reach * (0.24 + r(31.0 + j as f64) * 0.1);
                let at = top + Vec3::new(a.cos() * reach * 0.25, crown * 0.96, a.sin() * reach * 0.25);
                block(
                    out,
                    at,
                    Vec3::new(cap, cap * 0.6, cap * 0.85),
                    r(32.0 + j as f64) * 3.0,
                    jit(LEAF_LIT, r(33.0 + j as f64) as f64, 0.1) * dim * 0.9,
                    SHADE_SOFT,
                );
            }
        }
        Kind::Yew => {
            // A column clipped to a blunt point, in drums that bulge a little.
            let drums = 5;
            let col_h = height - trunk_h;
            for i in 0..drums {
                let t = i as f32 / drums as f32;
                let w = reach * (1.0 - t * 0.55) * (0.92 + r(20.0 + i as f64) * 0.12);
                let dh = col_h / drums as f32;
                let c = jit(if woods { WOODS[i % 2] } else { YEW[i % 3] }, r(21.0 + i as f64) as f64, 0.1) * (0.8 + t * 0.35);
                block(
                    out,
                    top + Vec3::Y * (dh * (i as f32 + 0.5)),
                    Vec3::new(w * 1.6, dh + 0.04, w * 1.6),
                    r(22.0 + i as f64) * 1.5,
                    c,
                    SHADE_SOFT,
                );
            }
            block(out, top + Vec3::Y * (col_h + 0.04), Vec3::new(reach * 0.4, 0.12, reach * 0.4), 0.7, lin(YEW_TOP[0]) * dim, SHADE_SOFT);
        }
        Kind::Pine => {
            // Tiers of boughs reaching out from the trunk and drooping at their tips, each tier
            // turned from the last and narrower, over a dark heart.
            let tiers = 5;
            let crown = height - trunk_h;
            for i in 0..tiers {
                let t = i as f32 / tiers as f32;
                let w = reach * (1.0 - t * 0.75);
                let y = crown * (t * 0.9);
                let c = jit(if woods { WOODS[i % 2] } else { PINE[i % 2] }, r(25.0 + i as f64) as f64, 0.1) * (0.82 + t * 0.35);
                block(out, top + Vec3::Y * (y + 0.04), Vec3::new(w * 0.9, 0.12, w * 0.9), r(27.0 + i as f64) * 3.0, c * 0.62, SHADE_SOFT);
                let boughs = if i + 2 >= tiers { 3 } else { 4 };
                for b in 0..boughs {
                    let a = b as f32 / boughs as f32 * TAU + i as f32 * 0.7 + r(26.0 + b as f64) * 0.6;
                    let len = w * (0.85 + r(28.0 + (i * 5 + b) as f64) * 0.3);
                    let dir = Vec3::new(a.cos(), 0.0, a.sin());
                    let rot = Quat::from_rotation_y(-a) * Quat::from_rotation_z(-0.22);
                    let at = top + Vec3::Y * (y + 0.06) + dir * len * 0.5;
                    block_m(
                        out,
                        Mat4::from_scale_rotation_translation(Vec3::new(len, 0.08, 0.2 * (1.0 - t * 0.4) + 0.06), rot, at),
                        c * (0.95 + 0.1 * (b % 2) as f32),
                        SHADE_SOFT,
                    );
                    // The tip, drooping.
                    block(out, top + Vec3::Y * (y - 0.02) + dir * len * 0.95, Vec3::new(0.08, 0.07, 0.1), -a, c * 1.12, SHADE_SOFT);
                }
            }
            block(out, top + Vec3::Y * (crown + 0.04), Vec3::new(0.08, 0.22, 0.08), 0.0, lin(PINE[1]) * dim, SHADE_SOFT);
            // The trunk on up through the tiers.
            block(out, top + Vec3::Y * crown * 0.45, Vec3::new(0.06, crown * 0.9, 0.06), 0.0, lin(BARK[1]), SHADE_WOOD);
        }
    }
}

/// A lantern on an iron post at `p` (its foot), its arm reaching toward `arm`: a pale stone
/// foot, the post with a collar, the arm and its brace, and the lantern of copper and amber glass
/// hung from it. Returns the middle of the glass.
pub(super) fn lamp_post(out: &mut Vec<Instance>, p: Vec3, arm: Vec3, post: f32) -> Vec3 {
    let iron = lin(IRON).lerp(lin(RUST), 0.3);
    block(out, p + Vec3::Y * 0.07, Vec3::new(0.2, 0.14, 0.2), 0.3, jit(PALE_STONE[1], 0.3, 0.04) * 0.7, SHADE_STONE);
    block(out, p + Vec3::Y * (0.14 + post / 2.0), Vec3::new(0.05, post, 0.05), 0.0, iron, SHADE_IRON);
    block(out, p + Vec3::Y * 0.26, Vec3::new(0.085, 0.08, 0.085), 0.0, iron, SHADE_IRON);
    block(out, p + Vec3::Y * (0.14 + post), Vec3::new(0.075, 0.05, 0.075), 0.0, iron, SHADE_IRON);
    let a = arm.normalize_or(Vec3::X);
    let turn = (-a.z).atan2(a.x);
    let top = p + Vec3::Y * (0.12 + post);
    block(out, top + a * 0.15, Vec3::new(0.3, 0.035, 0.035), turn, iron, SHADE_IRON);
    // The brace under the arm.
    let brace = Quat::from_rotation_y(turn) * Quat::from_rotation_z(0.8);
    block_m(
        out,
        Mat4::from_scale_rotation_translation(Vec3::new(0.025, 0.2, 0.025), brace, top + a * 0.08 - Vec3::Y * 0.08),
        iron,
        SHADE_IRON,
    );
    lantern_cubes(out, top + a * 0.27, 0.04)
}

/// What a tile reads as underfoot.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Floor {
    /// Wet flagstones: beside the water and round the fountain.
    Avenue,
    /// The fountain plaza's corners: the fountain lays its own paving.
    Plaza,
    /// Gravel with turf verges along the hedges.
    Walk,
}

impl WorldView {
    /// The garden's ground, beds and lanterns, in place of `build_forest`'s lawn and mounds.
    pub(super) fn build_garden_board(&mut self, grid: &Grid) {
        let floor_y = |x: i32, y: i32| grid.elevation(x, y) as f32 * STEP;
        let rock = |x: i32, y: i32| grid.get(x, y) == Tile::Rock;
        let feature = self.garden.as_ref().map(garden::features).unwrap_or_default();
        self.estate_art = true;
        let water = |p: P| matches!(feature.get(&p), Some(Feature::Water | Feature::Fountain));
        let fountain = self.garden.as_ref().and_then(|g| g.fountain).map(|f| f.p);
        let (w, h) = (grid.width, grid.height);
        let border = |x: i32, y: i32| x == 0 || y == 0 || x == w - 1 || y == h - 1;

        // ---- the beds: yew round every side on a walk, trees and lanterns after
        let mut trees: Vec<(P, Vec3, Kind, f32, f32, bool)> = Vec::new();
        let mut lanterns: Vec<(P, Vec3, Vec3)> = Vec::new();
        for (x, y, t) in grid.cells() {
            if t != Tile::Rock {
                continue;
            }
            let p = P::new(x, y);
            let here = feature.get(&p).copied();
            if matches!(here, Some(Feature::Water | Feature::Grave | Feature::Flowers | Feature::Fountain)) {
                continue;
            }
            // The gatehouse stands on the gate's tile and the two either side of it.
            if self.gate_tile.is_some_and(|g| g.y == y && (g.x - x).abs() <= 1) {
                continue;
            }
            let roads: Vec<Dir> = CARDINALS
                .iter()
                .copied()
                .filter(|d| {
                    let (dx, dy) = d.vec();
                    grid.in_bounds(x + dx, y + dy) && !rock(x + dx, y + dy)
                })
                .collect();
            let start = self.static_cubes.len();
            // Some beds are planted (flowers in the board's quieter colours), the rest are lawn
            // with cushions of box, or earth under the trees.
            let planted = here.is_none() && !border(x, y) && hash(x as f64 * 2.17 + 3.0, y as f64 * 1.31 + 9.0) < 0.3;
            self.bed_tile(grid, p, &roads, here, planted);
            let len = self.static_cubes.len() - start;
            self.rock_index.insert(tile_key(x, y), (start, len));
            self.authored.push((start, len));
            if here.is_some() || planted {
                continue; // a temple, a shrine or a flower bed stands here instead of a tree
            }
            let (hx, hy) = (x as f64, y as f64);
            let base = floor_y(x, y) + BED;
            // A lantern on the outer corner of a bed where two walks cross.
            let corner = roads.iter().flat_map(|a| roads.iter().map(move |b| (*a, *b))).find(|(a, b)| {
                let ((ax, ay), (bx, by)) = (a.vec(), b.vec());
                ax * bx + ay * by == 0 && ax + bx != 0 && grid.in_bounds(x + ax + bx, y + ay + by) && !rock(x + ax + bx, y + ay + by)
            });
            if let Some((a, b)) = corner.filter(|_| !border(x, y) && hash(hx * 3.7 + 1.0, hy * 2.9 + 4.0) < 0.6) {
                let near = lanterns.iter().any(|(q, _, _)| (q.x - x).abs() < LANTERN_GAP && (q.y - y).abs() < LANTERN_GAP);
                if !near {
                    let ((ax, ay), (bx, by)) = (a.vec(), b.vec());
                    let out = Vec3::new((ax + bx) as f32, 0.0, (ay + by) as f32);
                    let foot = Vec3::new(x as f32, base, y as f32) + out * 0.36;
                    lanterns.push((p, foot, out));
                    continue;
                }
            }
            // Trees: the woods along the island's rim, and in the beds, denser away from the walks.
            let pick = hash(hx * 5.0 + 1.0, hy * 3.0 + 2.0);
            let away = roads.iter().fold(Vec3::ZERO, |s, d| {
                let (dx, dy) = d.vec();
                s - Vec3::new(dx as f32, 0.0, dy as f32)
            });
            let jitter = Vec3::new((hash(hx + 1.0, hy) as f32 - 0.5) * 0.16, 0.0, (hash(hx, hy + 1.0) as f32 - 0.5) * 0.16);
            let at = Vec3::new(x as f32, base, y as f32) + away * 0.18 + jitter;
            let kind = match hash(hx * 7.0 + 3.0, hy * 2.0) {
                s if s < 0.5 => Kind::Broadleaf,
                s if s < 0.75 => Kind::Yew,
                _ => Kind::Pine,
            };
            if border(x, y) {
                if pick < 0.8 {
                    // Out on the rim, tall and dark: the woods beyond the walls against the sky.
                    let outward = Vec3::new(
                        if x == 0 {
                            -1.0
                        } else if x == w - 1 {
                            1.0
                        } else {
                            0.0
                        },
                        0.0,
                        if y == 0 {
                            -1.0
                        } else if y == h - 1 {
                            1.0
                        } else {
                            0.0
                        },
                    );
                    let kind = if kind == Kind::Broadleaf { Kind::Pine } else { kind };
                    let height = 2.3 + hash(hy * 1.3, hx * 0.7) as f32 * 1.0;
                    trees.push((p, Vec3::new(x as f32, base, y as f32) + outward * 0.22 + jitter, kind, height, 0.42, true));
                }
            } else if pick < if roads.is_empty() { 0.72 } else { 0.48 } {
                let height = 1.35 + hash(hy, hx) as f32 * 0.6 + if roads.is_empty() { 0.25 } else { 0.0 };
                let reach = if kind == Kind::Yew { 0.2 } else { 0.42 };
                trees.push((p, at, kind, height, reach, false));
            }
        }
        for (key, pos, kind, height, reach, woods) in trees {
            let start = self.static_cubes.len();
            garden_tree(&mut self.static_cubes, pos, kind, height, reach, woods, (key.x * 131 + key.y * 17) as f64 * 0.37);
            let len = self.static_cubes.len() - start;
            let shades = self.static_cubes[start..start + len].iter().map(|c| c.emissive[3]).collect();
            self.tree_index.insert(key, self.trees.len());
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
        for (spot, foot, out) in lanterns {
            let start = self.static_cubes.len();
            let glass = lamp_post(&mut self.static_cubes, foot, out, 1.42);
            let len = self.static_cubes.len() - start;
            // (The light is lit half a unit above a torch's point.)
            self.torches.insert(spot, (glass - Vec3::Y * 0.5, (start, len)));
            self.authored.push((start, len));
        }
        // ---- floors (after the lanterns, so rain can stand where they light it)
        let start = self.static_cubes.len();
        for (x, y, t) in grid.cells() {
            if t == Tile::Rock || t == Tile::Pit || grid.is_ramp(x, y) || grid.is_bridge(x, y) {
                continue;
            }
            let p = P::new(x, y);
            let plaza = fountain.is_some_and(|f| (f.x - x).abs() == 1 && (f.y - y).abs() == 1);
            let avenue = CARDINALS.iter().any(|d| {
                let (dx, dy) = d.vec();
                water(P::new(x + dx, y + dy))
            });
            let kind = if plaza {
                Floor::Plaza
            } else if avenue {
                Floor::Avenue
            } else {
                Floor::Walk
            };
            self.floor_tile(grid, p, t, kind, &feature);
        }
        // ---- ramps: pale stone steps, moss along each nose
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
                    !rock(n.x, n.y) && !grid.is_ramp(n.x, n.y) && grid.elevation(n.x, n.y) > low
                })
                .or_else(|| {
                    CARDINALS.iter().copied().find(|&d| {
                        let n = grid.neighbor(P::new(x, y), d);
                        grid.is_ramp(n.x, n.y) && grid.elevation(n.x, n.y) > low
                    })
                })
                .unwrap_or(Dir::Right);
            let (vx, vy) = up.vec();
            for i in 0..3 {
                let top = low as f32 * STEP + ((i + 1) as f32 / 3.0) * STEP;
                let o = (i as f32 - 1.0) / 3.0;
                let hk = hash(x as f64 + i as f64, y as f64);
                let size = Vec3::new(if vx != 0 { 1.0 / 3.0 } else { 1.0 }, top + 0.1, if vy != 0 { 1.0 / 3.0 } else { 1.0 });
                block(
                    &mut self.static_cubes,
                    Vec3::new(x as f32 + vx as f32 * o, (top - 0.1) / 2.0, y as f32 + vy as f32 * o),
                    size,
                    0.0,
                    jit(PALE_STONE[(hk * 2.0) as usize % 2], hk, 0.06) * 0.78,
                    SHADE_STONE,
                );
                let lip =
                    Vec3::new(x as f32 + vx as f32 * (o - 1.0 / 6.0 + 0.02), top + 0.008, y as f32 + vy as f32 * (o - 1.0 / 6.0 + 0.02));
                let s = if vx != 0 { Vec3::new(0.04, 0.016, 0.7 + hk as f32 * 0.2) } else { Vec3::new(0.7 + hk as f32 * 0.2, 0.016, 0.04) };
                block(&mut self.static_cubes, lip, s, 0.0, jit(MOSS[i % 3], hk, 0.1), SHADE_SOFT);
            }
        }
        // ---- pits: a black drop with a pale stone kerb
        for (x, y, t) in grid.cells() {
            if t != Tile::Pit {
                continue;
            }
            block(&mut self.static_cubes, Vec3::new(x as f32, -0.9, y as f32), Vec3::new(1.0, 0.1, 1.0), 0.0, lin(PIT), SHADE_LIT);
            block(
                &mut self.static_cubes,
                Vec3::new(x as f32, -0.03, y as f32),
                Vec3::new(1.02, 0.06, 1.02),
                0.0,
                jit(PALE_STONE[1], hash(x as f64, y as f64), 0.05) * 0.6,
                SHADE_STONE,
            );
        }
        self.authored.push((start, self.static_cubes.len() - start));

        if let Some(plan) = self.garden.clone() {
            self.build_garden(grid, &plan, &feature);
        }
        self.build_mist(grid, &feature);
    }

    /// One tile of floor: its terrace's earth and retaining walls, then flagstones, the plaza's
    /// bed, or a gravel walk with turf verges against the hedges, tufts, pebbles, leaves and the
    /// odd puddle.
    fn floor_tile(&mut self, grid: &Grid, p: P, t: Tile, kind: Floor, feature: &HashMap<P, Feature>) {
        let (x, y) = (p.x, p.y);
        let (xf, yf) = (x as f32, y as f32);
        let (hx, hy) = (x as f64, y as f64);
        let h = grid.elevation(x, y) as f32 * STEP;
        let out = &mut self.static_cubes;
        if h > 0.0 {
            block(
                out,
                Vec3::new(xf, h / 2.0 - 0.08, yf),
                Vec3::new(1.0, h + 0.04, 1.0),
                0.0,
                jit(UMBER[1], hash(hx, hy), 0.08) * 0.8,
                SHADE_STONE,
            );
        }
        retaining_walls(out, grid, p, h);
        if t == Tile::Nest {
            // The hollow the dead climb out of: dark sunken earth, roots and bones.
            block(out, Vec3::new(xf, h - 0.07, yf), Vec3::new(1.0, 0.1, 1.0), 0.0, lin(HOLLOW) * 0.7, SHADE_LIT);
            for k in 0..2 {
                let hk = hash(hx * 3.1 + k as f64, hy * 1.7) as f32;
                crypt::bone_cubes(out, Vec3::new(xf + (hk - 0.5) * 0.6, h - 0.02, yf + (0.5 - hash(hy, hx + k as f64) as f32) * 0.6), hk);
            }
            return;
        }
        match kind {
            Floor::Avenue => {
                flagstones(out, Vec3::new(xf, h, yf), (hx, hy));
                moss_seam(out, Vec3::new(xf, h, yf), (hx, hy));
            }
            Floor::Plaza => {
                block(out, Vec3::new(xf, h - 0.05, yf), Vec3::new(1.0, 0.1, 1.0), 0.0, lin(crypt::pal::GROUT), SHADE_LIT);
            }
            Floor::Walk => {
                // The gravel: one tone, its chips and puddles shaded in world space, so the walk
                // reads as one bed and not as a tile's slab.
                block(out, Vec3::new(xf, h - 0.05, yf), Vec3::new(1.0, 0.1, 1.0), 0.0, lin(GRAVEL[0]), SHADE_GRAVEL);
                let hedged = |dx: i32, dy: i32| {
                    let n = P::new(x + dx, y + dy);
                    grid.in_bounds(n.x, n.y)
                        && grid.get(n.x, n.y) == Tile::Rock
                        && !matches!(feature.get(&n), Some(Feature::Water | Feature::Fountain))
                };
                let c = Vec3::new(xf, h, yf);
                // The walk's gravel, between the lawns: what is left of the tile.
                let (mut lo, mut hi) = (Vec2::splat(-0.5), Vec2::splat(0.5));
                for (i, d) in CARDINALS.iter().enumerate() {
                    let dv = d.vec();
                    if !hedged(dv.0, dv.1) {
                        continue;
                    }
                    match dv {
                        (1, _) => hi.x = 0.5 - VERGE,
                        (-1, _) => lo.x = -0.5 + VERGE,
                        (_, 1) => hi.y = 0.5 - VERGE,
                        _ => lo.y = -0.5 + VERGE,
                    }
                    let at = side_at(c, dv);
                    let turn = side_turn(dv);
                    let k = hash(hx * 2.7 + i as f64, hy * 1.9 - i as f64) as f32;
                    // The lawn along the hedge, a touch proud of the gravel: darker in the
                    // hedge's shadow, in two or three tones along its length; the line of setts
                    // that edges it.
                    block(out, at(0.0, 0.0, 0.5 - FOOT / 2.0), Vec3::new(1.0, 0.05, FOOT), turn, turf(k + 0.3) * 0.62, SHADE_GRASS);
                    block(
                        out,
                        at(0.0, 0.0, 0.5 - FOOT - (VERGE - FOOT) / 2.0),
                        Vec3::new(1.0, 0.05, VERGE - FOOT),
                        turn,
                        turf(k),
                        SHADE_GRASS,
                    );
                    block(
                        out,
                        at(0.0, 0.006, 0.5 - VERGE - 0.012),
                        Vec3::new(1.0, 0.035, 0.03),
                        turn,
                        jit(PALE_STONE[1], k as f64, 0.08) * 0.55,
                        SHADE_STONE,
                    );
                    for t in 0..3 {
                        let kt = hash(hx * 4.1 + t as f64 + i as f64 * 5.0, hy * 3.9 + t as f64) as f32;
                        if kt < 0.7 {
                            tuft(out, at((kt - 0.5) * 0.9, 0.02, 0.5 - FOOT - kt * (VERGE - FOOT) * 0.8), kt * 13.0 + i as f32 + t as f32);
                        }
                    }
                    // Leaves blown onto the lawn's edge.
                    for j in 0..2 {
                        let kj = hash(hx * 5.3 + j as f64 + i as f64 * 3.0, hy * 4.1) as f32;
                        if kj < 0.5 {
                            leaf(out, at((kj - 0.25) * 1.8, 0.03, 0.5 - VERGE + 0.04 + kj * 0.12), kj * 17.0);
                        }
                    }
                }
                // Lawn at an inner corner, where the walks turn round a bed.
                for (dx, dy) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
                    if hedged(dx, dy) && !hedged(dx, 0) && !hedged(0, dy) {
                        let k = hash(hx + dx as f64 * 3.0, hy + dy as f64 * 5.0) as f32;
                        block(
                            out,
                            Vec3::new(xf + dx as f32 * (0.5 - VERGE / 2.0), h, yf + dy as f32 * (0.5 - VERGE / 2.0)),
                            Vec3::new(VERGE, 0.05, VERGE),
                            0.0,
                            turf(k),
                            SHADE_GRASS,
                        );
                    }
                }
                let span = hi - lo;
                // Pebbles kicked up out of the gravel.
                for j in 0..5 {
                    let (a, b) = (hash(hx * 7.1 + j as f64, hy * 3.3) as f32, hash(hy * 5.7 + j as f64, hx * 2.9) as f32);
                    let c = if a < 0.5 { jit(PALE_STONE[j % 2], b as f64, 0.1) * 0.6 } else { jit(CHARCOAL[j % 3], b as f64, 0.1) };
                    block(
                        out,
                        Vec3::new(xf + lo.x + span.x * (0.1 + a * 0.8), h + 0.008, yf + lo.y + span.y * (0.1 + b * 0.8)),
                        Vec3::new(0.035 + a * 0.035, 0.02, 0.03 + b * 0.03),
                        a * 6.0,
                        c,
                        SHADE_STONE,
                    );
                }
                // Now and then rain stands in a hollow of the walk, more often by a lantern, where
                // it holds the light.
                let hp = hash(hx * 2.3 + 17.0, hy * 3.1 + 5.0);
                let lit = self.torches.values().any(|(t, _)| (t.x - xf).abs() < 2.5 && (t.z - yf).abs() < 2.5);
                if hp < if lit { 0.35 } else { 0.06 } && span.min_element() > 0.3 {
                    let (a, b) = (hash(hx, hy * 7.0) as f32, hash(hy, hx * 7.0) as f32);
                    let at = Vec3::new(xf + lo.x + span.x * (0.3 + a * 0.4), h + 0.006, yf + lo.y + span.y * (0.3 + b * 0.4));
                    self.puddles.push((at, Vec2::new(span.x * (0.45 + a * 0.3), span.y * (0.4 + b * 0.3))));
                }
            }
        }
    }

    /// A flower garden's tile (the plan's parterres): a lawn, low clipped box along its walks
    /// and its edges, inside it flowers in rows, and now and then a yew cone or a stone urn.
    pub(super) fn parterre_tile(&mut self, grid: &Grid, p: P, feature: &HashMap<P, Feature>) {
        let (x, y) = (p.x, p.y);
        let (xf, yf) = (x as f32, y as f32);
        let (hx, hy) = (x as f64, y as f64);
        let floor = grid.elevation(x, y) as f32 * STEP;
        let base = floor + BED;
        let start = self.static_cubes.len();
        let out = &mut self.static_cubes;
        let k = hash(hx * 1.3, hy * 2.7);
        block(out, Vec3::new(xf, (base - 0.12) / 2.0, yf), Vec3::new(1.0, base + 0.12, 1.0), 0.0, turf(k as f32) * 0.8, SHADE_GRASS);
        retaining_walls(out, grid, p, floor);
        let c = Vec3::new(xf, base, yf);
        let (mut lo, mut hi) = (Vec2::splat(-0.42), Vec2::splat(0.42));
        for d in CARDINALS {
            let (dx, dy) = d.vec();
            let n = P::new(x + dx, y + dy);
            let open = grid.in_bounds(n.x, n.y) && grid.get(n.x, n.y) != Tile::Rock;
            let other = grid.get(n.x, n.y) == Tile::Rock && feature.get(&n) != Some(&Feature::Flowers);
            if !(open || other) {
                continue;
            }
            hedge(out, c, (dx, dy), 0.26, (hx * 1.7, hy * 2.3));
            match (dx, dy) {
                (1, _) => hi.x = 0.5 - HEDGE_T,
                (-1, _) => lo.x = -0.5 + HEDGE_T,
                (_, 1) => hi.y = 0.5 - HEDGE_T,
                _ => lo.y = -0.5 + HEDGE_T,
            }
        }
        let centre = hash(hx * 3.0, hy * 17.0);
        if centre < 0.2 {
            // A clipped yew cone on a little plinth.
            block(out, c + Vec3::Y * 0.04, Vec3::new(0.2, 0.08, 0.2), 0.0, jit(PALE_STONE[0], k, 0.05) * 0.7, SHADE_STONE);
            for (j, w) in [0.22f32, 0.18, 0.13, 0.08, 0.04].into_iter().enumerate() {
                block(
                    out,
                    c + Vec3::Y * (0.12 + j as f32 * 0.09),
                    Vec3::new(w, 0.1, w),
                    j as f32 * 0.4,
                    jit(YEW[j % 3], k, 0.08) * (1.0 + j as f32 * 0.06),
                    SHADE_SOFT,
                );
            }
            block(out, c + Vec3::Y * 0.52, Vec3::new(0.035, 0.05, 0.035), 0.0, lin(YEW_TIP), SHADE_SOFT);
        } else if centre < 0.35 {
            garden_landmarks::urn_cubes(out, c, (hx, hy));
        }
        planting(out, c, lo, hi - lo, (hx, hy));
        let len = self.static_cubes.len() - start;
        self.rock_index.insert(tile_key(x, y), (start, len));
        self.authored.push((start, len));
    }

    /// A wall tile as a bed: its earth (and terrace walls), clipped yew along every side on a
    /// walk, and on its earth moss, leaves and now and then a clipped topiary. A temple's lawn
    /// is kept low (box, not yew) so the temple reads.
    fn bed_tile(&mut self, grid: &Grid, p: P, roads: &[Dir], here: Option<Feature>, planted: bool) {
        let (x, y) = (p.x, p.y);
        let (xf, yf) = (x as f32, y as f32);
        let (hx, hy) = (x as f64, y as f64);
        let floor = grid.elevation(x, y) as f32 * STEP;
        let base = floor + BED;
        let lawn = here == Some(Feature::Temple);
        let out = &mut self.static_cubes;
        let k = hash(hx * 1.3, hy * 2.7);
        // A lawn inside the yew, darker than the walks' (the hedge shades it); earth where
        // flowers are planted.
        let grass = !planted;
        let ground = if grass { turf(k as f32) * 0.9 } else { jit(EARTH[(k * 2.0) as usize % 2], k, 0.1) };
        block(
            out,
            Vec3::new(xf, (base - 0.12) / 2.0, yf),
            Vec3::new(1.0, base + 0.12, 1.0),
            0.0,
            ground,
            if grass { SHADE_GRASS } else { SHADE_LIT },
        );
        retaining_walls(out, grid, p, floor);
        let c = Vec3::new(xf, base, yf);
        let height = if lawn { 0.3 } else { HEDGE_H - BED + (hash(hx * 0.37, hy * 0.53) as f32 - 0.5) * 0.06 };
        // A wayside shrine faces the first walk it stands on (the yew opened for it).
        let shrine = (here == Some(Feature::Shrine)).then(|| {
            CARDINALS
                .iter()
                .copied()
                .find(|d| {
                    let (dx, dy) = d.vec();
                    let t = grid.get(x + dx, y + dy);
                    t != Tile::Rock && t != Tile::Nest
                })
                .unwrap_or(Dir::Down)
        });
        for &d in roads {
            if Some(d) == shrine {
                // A low kerb where the yew stands back from the shrine.
                let at = side_at(c, d.vec());
                block(
                    out,
                    at(0.0, 0.03, 0.44),
                    Vec3::new(1.0, 0.06, 0.12),
                    side_turn(d.vec()),
                    jit(PALE_STONE[1], k, 0.05) * 0.65,
                    SHADE_STONE,
                );
                continue;
            }
            hedge(out, c, d.vec(), height, (hx, hy));
        }
        if let Some(face) = shrine {
            let (dx, dy) = face.vec();
            let yaw = (dx as f32).atan2(dy as f32);
            let at = c + Vec3::new(dx as f32, 0.0, dy as f32) * 0.2;
            if hash(hx * 2.9 + 1.0, hy * 3.7) < 0.3 {
                garden_landmarks::urn_cubes(out, at, (hx, hy));
            } else {
                let flame = garden_landmarks::shrine_cubes(out, at, yaw, (hx, hy));
                self.candles.push(flame);
            }
        }
        if lawn {
            return;
        }
        // Inside the yew, the bed: low cushions of box and laurel, moss and fallen leaves.
        let (mut lo, mut hi) = (Vec2::splat(-0.5), Vec2::splat(0.5));
        for d in roads {
            match d.vec() {
                (1, _) => hi.x = 0.5 - HEDGE_T,
                (-1, _) => lo.x = -0.5 + HEDGE_T,
                (_, 1) => hi.y = 0.5 - HEDGE_T,
                _ => lo.y = -0.5 + HEDGE_T,
            }
        }
        let span = hi - lo;
        if planted {
            planting(out, c, lo, span, (hx, hy));
            return;
        }
        for j in 0..4 {
            let (a, b, e) = (
                hash(hx * 4.3 + j as f64, hy * 1.9) as f32,
                hash(hy * 3.7 + j as f64, hx * 2.2) as f32,
                hash(hx * 1.1 + j as f64 * 7.0, hy * 5.3) as f32,
            );
            let at = c + Vec3::new(lo.x + span.x * (0.15 + a * 0.7), 0.0, lo.y + span.y * (0.15 + b * 0.7));
            if j < 3 {
                let w = (0.2 + e * 0.16).min(span.max_element() * 0.9 + 0.1);
                let tall = 0.16 + e * 0.2;
                let tone = jit(LEAF[(j + (e * 3.0) as usize) % 3], a as f64, 0.12) * 1.05;
                block(out, at + Vec3::Y * (tall / 2.0), Vec3::new(w, tall, w * 0.85), a * 3.0, tone, SHADE_SOFT);
                block(out, at + Vec3::Y * tall, Vec3::new(w * 0.6, 0.06, w * 0.55), a * 3.0 + 0.6, tone * 1.18, SHADE_SOFT);
            } else if a < 0.5 {
                block(out, at + Vec3::Y * 0.012, Vec3::new(0.22, 0.025, 0.18), a * 4.0, jit(MOSS[j % 3], b as f64, 0.1), SHADE_SOFT);
            } else {
                leaf(out, at + Vec3::Y * 0.01, a * 23.0 + b);
            }
        }
    }
}

/// Flowers planted in rows over `lo..lo + span` round `c` (on the bed): low mounds of leaves,
/// each with a bloom, in a pair of the board's quieter colours (rust rose, amber, dusk
/// lavender, pale stone), a sprig of something taller now and then.
pub(super) fn planting(out: &mut Vec<Instance>, c: Vec3, lo: Vec2, span: Vec2, (hx, hy): (f64, f64)) {
    let pick = hash(hx * 3.3 + 1.0, hy * 2.9 + 7.0);
    let pair = [BLOOMS[(pick * 4.0) as usize % 4], BLOOMS[(pick * 11.0) as usize % 4]];
    let (nx, nz) = (((span.x / 0.17) as usize).clamp(1, 6), ((span.y / 0.17) as usize).clamp(1, 6));
    for i in 0..nx {
        for j in 0..nz {
            let k = hash(hx * 5.1 + i as f64 * 1.7, hy * 4.3 + j as f64 * 2.3) as f32;
            let at = c + Vec3::new(
                lo.x + span.x * (i as f32 + 0.5) / nx as f32 + (k - 0.5) * 0.03,
                0.0,
                lo.y + span.y * (j as f32 + 0.5) / nz as f32,
            );
            let leafy = jit(LEAF[(i + j) % 3], k as f64, 0.12) * 0.95;
            block(out, at + Vec3::Y * 0.04, Vec3::new(0.12, 0.08, 0.11), k * 3.0, leafy, SHADE_SOFT);
            let bloom = jit(pair[(i + j) % 2], k as f64, 0.1);
            let tall = if k < 0.15 { 0.16 } else { 0.1 };
            block(out, at + Vec3::Y * tall, Vec3::new(0.06, 0.05, 0.06), k * 5.0, bloom, SHADE_SOFT);
            if k > 0.7 {
                block(out, at + Vec3::new(0.03, 0.09, -0.02), Vec3::new(0.045, 0.04, 0.045), k * 7.0, bloom * 1.1, SHADE_SOFT);
            }
        }
    }
}

/// Dressed-stone retaining walls on every side of the tile at `p` (its floor `h`) where the
/// ground falls away to a lower walk or bed: courses of the crypt's chunky blocks over a plinth.
fn retaining_walls(out: &mut Vec<Instance>, grid: &Grid, p: P, h: f32) {
    if h <= 0.0 {
        return;
    }
    for d in CARDINALS {
        let (dx, dy) = d.vec();
        let (nx, ny) = (p.x + dx, p.y + dy);
        if !grid.in_bounds(nx, ny) || grid.get(nx, ny) == Tile::Pit || grid.is_ramp(nx, ny) || grid.is_ramp(p.x, p.y) {
            continue;
        }
        let low = grid.elevation(nx, ny) as f32 * STEP;
        if low >= h - 0.1 {
            continue;
        }
        let height = h - low;
        masonry(
            out,
            &Wall {
                centre: Vec3::new(p.x as f32, 0.0, p.y as f32),
                dir: (dx, dy),
                half: 0.5,
                base: low,
                height,
                courses: (((height - 0.2) / 0.19).round() as usize).max(1),
                seed: (p.x as f64 * 1.3 + 7.0, p.y as f64 * 1.7 + 3.0),
                len: (0.26, 0.22),
                depth: (0.03, 0.05),
                face: 0.46,
                wet: false,
                ruined: false,
                foot: None,
                openings: &[],
            },
        );
    }
}
