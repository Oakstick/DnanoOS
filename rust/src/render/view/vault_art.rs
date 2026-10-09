//! The Vaults of Mammon at the Cisterns' bar: their stone and gold. Greed's vaults are cut in the
//! crypt's dressed stone (`crypt::masonry`, warmed toward umber) under a pale cornice with a
//! gilt band, floored in worn flags (the treasury in polished black and cream marble), and
//! dressed with what greed keeps: gilded pilasters and columns, strongroom doors of riveted iron
//! with gilt locks, niches holding statues or racks of gold bars and plate, paintings in gilt
//! frames, candelabra, copper braziers and lanterns. The gold is metal (`SHADE_GILT`, and heaped
//! coins `SHADE_COINS`: `vault_gold` in the shader), its colour its reflectance, so the
//! practicals' highlights on it do the work against a deep dark.
//!
//! Everything here is built once, with the level, as static cubes.

use super::crypt::{self, block, block_m, jit, masonry, Wall, SHADE_CLOTH, SHADE_IRON, SHADE_STONE, SHADE_WOOD};
use super::vault::SHADE_MARBLE;
use super::vault_decor;
use super::*;

/// Gilt, plate and bars (`vault_gold` in the shader: rubbed edges, tarnished faces).
pub const SHADE_GILT: f32 = 13.4;
/// Heaped coins (`vault_gold`: coins laid over the faces in world space).
pub const SHADE_COINS: f32 = 13.25;
/// Glossy cut gems (`voxel_surface` kind 9).
pub const SHADE_GEM: f32 = 14.0;

pub(super) mod art {
    /// The flags: warm charcoal and umber, worn.
    pub const FLAG: [u32; 3] = [0x5a4f45, 0x4e463f, 0x63564a];
    /// Pale dressed stone: borders, cornices, plinths, jambs.
    pub const PALE: [u32; 2] = [0x9c8f78, 0x8c8069];
    pub const GROUT: u32 = 0x120f0d;
    /// The walls' core and the caps over the solid rock, seen from above.
    pub const CORE: u32 = 0x3a332d;
    pub const CAP: u32 = 0x2a2420;
    /// The treasury's marble.
    pub const MARBLE_BLACK: u32 = 0x1c1a1b;
    pub const MARBLE_CREAM: u32 = 0x62574a;
    /// Gold by its reflectance (yellow, red and pale), and worn dark.
    pub const GOLD: u32 = 0xffd47a;
    pub const GOLD_RED: u32 = 0xffca70;
    pub const GOLD_PALE: u32 = 0xffdd94;
    pub const GOLD_DARK: u32 = 0xd9ab58;
    /// Copper.
    pub const COPPER: u32 = 0x9a5b3c;
    pub const IRON: u32 = 0x2c2a28;
    pub const IRON_L: u32 = 0x45403b;
    pub const OAK: u32 = 0x4a3222;
    pub const OAK_D: u32 = 0x36251a;
    pub const VELVET: u32 = 0x5e1418;
    pub const VELVET_D: u32 = 0x430e12;
    pub const WAX: u32 = 0xe8dcc0;
    /// The hue the crypt's colder stone takes in the vaults (umber and charcoal under candlelight):
    /// its blocks keep their light and dark, not their greens and greys.
    pub const WARM: [f32; 3] = [1.2, 1.0, 0.8];
}
use art::*;

/// The vaults' walls stand this tall; the masonry rises to `FACE_H`, the cornice and coping over it.
pub(super) const WALL_H: f32 = 1.6;
const FACE_H: f32 = 1.38;
/// Where the masonry's blocks stand from a wall's middle, and how far they stand out of it.
pub(super) const STONE_FACE: f32 = 0.4;
pub(super) const STONE_DEPTH: (f32, f32) = (0.07, 0.055);
/// Nothing on a wall's face reaches further than this from its middle above his knees (the
/// cameras take a block reaching over a tile's edge as standing in that tile: RP02).
pub(super) const LIP: f32 = 0.545;

/// A gold, by a hash: mostly yellow, some red, some pale, each a touch brighter or darker.
pub(super) fn gold(h: f64) -> Vec3 {
    let c = if h < 0.55 {
        GOLD
    } else if h < 0.8 {
        GOLD_RED
    } else {
        GOLD_PALE
    };
    lin(c) * (0.82 + 0.18 * ((h * 7.3).fract() as f32))
}

/// Warms the stone just built (from `start`): each block keeps its lightness, in umber.
pub(super) fn warm(out: &mut [Instance], start: usize) {
    for c in &mut out[start..] {
        let l = c.color[0] * 0.3 + c.color[1] * 0.55 + c.color[2] * 0.15;
        c.color[0] = l * WARM[0];
        c.color[1] = l * WARM[1];
        c.color[2] = l * WARM[2];
    }
}

/// How much a piece of gold glows of its own colour: the candlelight it holds in the dark, so a
/// heap far from a lamp still reads as gold.
pub(super) const GOLD_GLOW: f32 = 0.05;

/// What a wall's face holds, every few blocks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Face {
    Plain,
    /// A gilded pilaster.
    Pilaster,
    /// A painting in a gilt frame between velvet drapes (which one).
    Painting(usize),
    /// A strongroom door of riveted iron.
    Door,
    /// A niche holding a statue (which one) or, past the statues, a rack of gold.
    Niche(usize),
    Hoard,
    /// A gilt sconce of two candles (a practical: its light is the room's).
    Sconce,
    /// Coin drifted up against the wall.
    Drift,
}

/// Puts a box at `pos` (centre), `size`, turned `rot_y`, into `out`.
fn bx(out: &mut Vec<Instance>, pos: Vec3, size: Vec3, rot_y: f32, color: Vec3, shade: f32) {
    block(out, pos, size, rot_y, color, shade);
}

/// A coin as three thin boxes, each turned a twelfth on the last (near enough round), lying at
/// `at` (its middle) under `m`, `r` across.
fn coin_at(out: &mut Vec<Instance>, m: Mat4, at: Vec3, r: f32, turn: f32, tilt: f32, c: Vec3) {
    for k in 0..3 {
        let rot = Quat::from_rotation_x(tilt) * Quat::from_rotation_y(turn + k as f32 * PI / 6.0);
        let bm = m * Mat4::from_scale_rotation_translation(Vec3::new(r, 0.018, r), rot, at);
        out.push(Instance::new(bm, c, 1.0, c * GOLD_GLOW, SHADE_GILT));
    }
}

/// A few coins stacked under `m` (on the floor at its origin), and one or two lying by them.
pub(super) fn coin_stack(out: &mut Vec<Instance>, m: Mat4, seed: f32) {
    let h = |k: f32| ((seed * 12.9898 + k * 78.233).sin() * 43758.547).fract().abs();
    let n = 2 + (h(1.0) * 3.0) as usize;
    let c = gold(h(2.0) as f64);
    for k in 0..n {
        let top = k + 1 == n;
        let off = Vec3::new((h(3.0 + k as f32) - 0.5) * 0.02, 0.009 + k as f32 * 0.019, (h(4.0 + k as f32) - 0.5) * 0.02);
        if top {
            coin_at(out, m, off, 0.1, h(5.0) * TAU, 0.0, c * 1.05);
        } else {
            let bm =
                m * Mat4::from_scale_rotation_translation(Vec3::new(0.1, 0.018, 0.1), Quat::from_rotation_y(h(6.0 + k as f32) * TAU), off);
            let c = c * (0.9 + 0.1 * h(7.0 + k as f32));
            out.push(Instance::new(bm, c, 1.0, c * GOLD_GLOW, SHADE_GILT));
        }
    }
    for k in 0..1 + (h(8.0) * 2.0) as usize {
        let a = h(9.0 + k as f32) * TAU;
        let d = 0.11 + h(10.0 + k as f32) * 0.06;
        let at = Vec3::new(a.cos() * d, 0.012, a.sin() * d);
        coin_at(out, m, at, 0.1, a, (h(11.0 + k as f32) - 0.5) * 0.4, gold(h(12.0 + k as f32) as f64));
    }
}

/// A gold bar (an ingot, its top narrower than its foot) lying at `at` along `along`.
pub(super) fn ingot(out: &mut Vec<Instance>, at: Vec3, along_x: bool, c: Vec3) {
    let (l, w) = if along_x { (Vec3::X, Vec3::Z) } else { (Vec3::Z, Vec3::X) };
    block(out, at + Vec3::Y * 0.014, l * 0.15 + w * 0.07 + Vec3::Y * 0.028, 0.0, c, SHADE_GILT);
    block(out, at + Vec3::Y * 0.036, l * 0.13 + w * 0.05 + Vec3::Y * 0.018, 0.0, c * 1.04, SHADE_GILT);
}

impl WorldView {
    /// A wall tile of the vaults at `p`, its faces toward `open`, each dressed as `faces` says
    /// (`art` holds the models the niches and frames hold, and the heaps). Returns where the
    /// candles of its sconces burn.
    pub(super) fn vault_wall(&mut self, p: P, open: &[Dir], faces: &[(Dir, Face)], art: &super::vault::VaultArt) -> Vec<Vec3> {
        let (statues, paintings) = (&art.statues, &art.paintings);
        let mut candles = Vec::new();
        let (xf, yf) = (p.x as f32, p.y as f32);
        let (hx, hy) = (p.x as f64, p.y as f64);
        let face_of = |d: Dir| faces.iter().find(|f| f.0 == d).map(|f| f.1).unwrap_or(Face::Plain);
        let out = &mut self.static_cubes;
        // The core, cut back where a niche or a door is sunk into a face.
        let (mut lo, mut hi) = (Vec3::new(-0.46, 0.0, -0.46), Vec3::new(0.46, 0.0, 0.46));
        for &d in open {
            let cut = match face_of(d) {
                Face::Niche(_) | Face::Hoard => 0.3,
                Face::Door => 0.2,
                _ => 0.0,
            };
            let (dx, dy) = d.vec();
            if dx > 0 {
                hi.x -= cut;
            } else if dx < 0 {
                lo.x += cut;
            } else if dy > 0 {
                hi.z -= cut;
            } else {
                lo.z += cut;
            }
        }
        let core_c = jit(CORE, hash(hx, hy), 0.06);
        let mid = (lo + hi) / 2.0;
        bx(
            out,
            Vec3::new(xf + mid.x, (FACE_H + 0.06) / 2.0, yf + mid.z),
            Vec3::new(hi.x - lo.x, FACE_H + 0.06, hi.z - lo.z),
            0.0,
            core_c,
            SHADE_STONE,
        );
        // The coping over the top, two heavy slabs.
        let split_x = hash(hx + 5.0, hy * 2.0) < 0.5;
        for k in 0..2 {
            let r = hash(hx * 2.1 + k as f64 * 1.7, hy * 3.3 + k as f64) as f32;
            let off = -0.25 + 0.5 * k as f32;
            let (size, o) = if split_x {
                (Vec3::new(0.48, 0.1, 1.04), Vec3::new(off, 0.0, 0.0))
            } else {
                (Vec3::new(1.04, 0.1, 0.48), Vec3::new(0.0, 0.0, off))
            };
            bx(out, Vec3::new(xf, WALL_H - 0.05 - (r - 0.5) * 0.01, yf) + o, size, 0.0, jit(CAP, r as f64, 0.1), SHADE_STONE);
        }
        for &d in open {
            let (dx, dy) = d.vec();
            let n = Vec3::new(dx as f32, 0.0, dy as f32);
            let along = Vec3::new(dy.abs() as f32, 0.0, dx.abs() as f32);
            let turn = if dx != 0 { PI / 2.0 } else { 0.0 };
            let c = Vec3::new(xf, 0.0, yf);
            let kind = face_of(d);
            // The face in dressed stone, an opening left for a niche or a door.
            // (Nothing on a face stands out past `LIP` from the wall's middle above his knees.)
            let openings: &[(f32, f32, f32, f32, f32)] = match kind {
                Face::Niche(_) | Face::Hoard => &[(0.0, 0.27, 0.0, 1.2, 0.0)],
                Face::Door => &[(0.0, 0.33, 0.0, 1.07, 0.0)],
                Face::Pilaster => &[(0.0, 0.12, 0.0, FACE_H, 0.0)],
                Face::Painting(_) => &[(0.0, 0.42, 0.45, FACE_H, 0.0)],
                Face::Sconce => &[(0.0, 0.15, 0.7, 1.12, 0.0)],
                _ => &[],
            };
            let start = out.len();
            masonry(
                out,
                &Wall {
                    centre: c,
                    dir: (dx, dy),
                    half: 0.5,
                    base: 0.0,
                    height: FACE_H,
                    courses: 4,
                    seed: (hx, hy),
                    len: (0.3, 0.24),
                    depth: STONE_DEPTH,
                    face: STONE_FACE,
                    wet: false,
                    ruined: false,
                    foot: None,
                    openings,
                },
            );
            warm(out, start);
            // The cornice: a gilt band under a pale moulding.
            let pale = jit(PALE[(hash(hx + dx as f64, hy + dy as f64) * 2.0) as usize % 2], hash(hy, hx), 0.05) * 0.62;
            bx(
                out,
                c + n * (LIP - 0.025) + Vec3::Y * (FACE_H + 0.025),
                along * 1.0 + n * 0.04 + Vec3::Y * 0.035,
                0.0,
                gold(hash(hx, hy)) * 0.9,
                SHADE_GILT,
            );
            bx(out, c + n * 0.47 + Vec3::Y * (FACE_H + 0.1), along * 1.02 + n * 0.14 + Vec3::Y * 0.12, 0.0, pale, SHADE_STONE);
            bx(out, c + n * 0.5 + Vec3::Y * (FACE_H + 0.17), along * 1.02 + n * 0.08 + Vec3::Y * 0.04, 0.0, pale * 1.08, SHADE_STONE);
            match kind {
                Face::Plain => {}
                Face::Pilaster => {
                    // A gilded pilaster on a pale base, fluted, under a gilt capital.
                    let g = gold(hash(hx * 1.3, hy * 0.7));
                    let f = c + n * 0.48;
                    bx(out, f + Vec3::Y * 0.12, along * 0.3 + n * 0.12 + Vec3::Y * 0.24, 0.0, pale, SHADE_STONE);
                    bx(out, f + Vec3::Y * 0.27, along * 0.26 + n * 0.1 + Vec3::Y * 0.06, 0.0, g * 0.85, SHADE_GILT);
                    bx(out, f + n * 0.015 + Vec3::Y * 0.81, along * 0.2 + n * 0.08 + Vec3::Y * 1.02, 0.0, g, SHADE_GILT);
                    for s in [-1.0f32, 1.0] {
                        bx(
                            out,
                            f + n * 0.045 + along * (s * 0.045) + Vec3::Y * 0.81,
                            along * 0.022 + n * 0.02 + Vec3::Y * 0.9,
                            0.0,
                            g * 0.45,
                            SHADE_GILT,
                        );
                    }
                    bx(out, f + Vec3::Y * 1.33, along * 0.28 + n * 0.12 + Vec3::Y * 0.1, 0.0, g * 1.05, SHADE_GILT);
                }
                Face::Painting(i) => {
                    // A gilt frame, the canvas, a gold rod and a velvet valance over it, drapes either side.
                    let root = Mat4::from_translation(c + n * 0.46) * Mat4::from_rotation_y(yaw_of(d));
                    let put = |out: &mut Vec<Instance>, at: Vec3, size: Vec3, color: Vec3, shade: f32| {
                        out.push(Instance::new(
                            root * Mat4::from_scale_rotation_translation(size, Quat::IDENTITY, at),
                            color,
                            1.0,
                            Vec3::ZERO,
                            shade,
                        ));
                    };
                    let (cw, ch) =
                        (vault_decor::CANVAS.0 as f32 * vault_decor::CANVAS_PX, vault_decor::CANVAS.1 as f32 * vault_decor::CANVAS_PX);
                    let mid = 0.95;
                    let g = gold(hash(hx * 0.3, hy * 1.9));
                    put(out, Vec3::new(0.0, mid, 0.02), Vec3::new(cw + 0.12, ch + 0.12, 0.04), g * 0.85, SHADE_GILT);
                    put(out, Vec3::new(0.0, mid + ch / 2.0 + 0.045, 0.045), Vec3::new(cw + 0.14, 0.03, 0.02), g, SHADE_GILT);
                    put(out, Vec3::new(0.0, mid - ch / 2.0 - 0.045, 0.045), Vec3::new(cw + 0.14, 0.03, 0.02), g * 0.7, SHADE_GILT);
                    vault_decor::push_merged(out, &paintings[i], root * Mat4::from_translation(Vec3::new(0.0, mid - ch / 2.0, 0.04)), 0.0);
                    put(out, Vec3::new(0.0, 1.36, 0.05), Vec3::new(0.84, 0.025, 0.025), g, SHADE_GILT);
                    put(out, Vec3::new(0.0, 1.32, 0.055), Vec3::new(0.82, 0.08, 0.03), lin(VELVET), SHADE_CLOTH);
                    for s in [-1.0f32, 1.0] {
                        put(out, Vec3::new(s * 0.37, 0.92, 0.05), Vec3::new(0.08, 0.82, 0.04), lin(VELVET_D), SHADE_CLOTH);
                        put(out, Vec3::new(s * 0.37, 0.84, 0.06), Vec3::new(0.09, 0.035, 0.03), g, SHADE_GILT);
                    }
                }
                Face::Door => {
                    // A strongroom door: pale jambs and a lintel, a door of riveted iron plates
                    // banded across, a gilt lock plate and ring, heavy hinges.
                    let f = c + n * 0.5;
                    for s in [-1.0f32, 1.0] {
                        bx(
                            out,
                            f - n * 0.03 + along * (s * 0.375) + Vec3::Y * 0.64,
                            along * 0.1 + n * 0.14 + Vec3::Y * 0.9,
                            0.0,
                            pale,
                            SHADE_STONE,
                        );
                    }
                    bx(out, f - n * 0.03 + Vec3::Y * 1.17, along * 0.88 + n * 0.14 + Vec3::Y * 0.2, 0.0, pale * 1.05, SHADE_STONE);
                    let door = f - n * 0.17;
                    let iron = jit(IRON, hash(hx * 2.0, hy), 0.08);
                    bx(out, door + Vec3::Y * 0.64, along * 0.66 + n * 0.06 + Vec3::Y * 0.88, 0.0, iron, SHADE_IRON);
                    bx(out, door + n * 0.035 + Vec3::Y * 0.64, along * 0.02 + n * 0.02 + Vec3::Y * 0.86, 0.0, lin(GROUT), SHADE_IRON);
                    for (k, hh) in [0.32f32, 0.64, 0.96].into_iter().enumerate() {
                        bx(out, door + n * 0.04 + Vec3::Y * hh, along * 0.66 + n * 0.025 + Vec3::Y * 0.07, 0.0, lin(IRON_L), SHADE_IRON);
                        for s in [-1.0f32, -0.33, 0.33, 1.0] {
                            bx(
                                out,
                                door + n * 0.056 + along * (s * 0.29) + Vec3::Y * hh,
                                Vec3::splat(0.026),
                                0.0,
                                lin(IRON_L) * 1.2,
                                SHADE_IRON,
                            );
                        }
                        // The hinges' straps out to the jamb.
                        if k != 1 {
                            bx(
                                out,
                                door + n * 0.05 + along * -0.36 + Vec3::Y * hh,
                                along * 0.1 + n * 0.03 + Vec3::Y * 0.05,
                                0.0,
                                lin(IRON),
                                SHADE_IRON,
                            );
                        }
                    }
                    let g = gold(hash(hx * 3.1, hy * 1.1));
                    bx(out, door + n * 0.05 + along * 0.17 + Vec3::Y * 0.78, along * 0.14 + n * 0.03 + Vec3::Y * 0.18, 0.0, g, SHADE_GILT);
                    bx(
                        out,
                        door + n * 0.068 + along * 0.17 + Vec3::Y * 0.79,
                        along * 0.025 + n * 0.01 + Vec3::Y * 0.06,
                        0.0,
                        lin(GROUT),
                        SHADE_LIT,
                    );
                    bx(
                        out,
                        door + n * 0.07 + along * 0.17 + Vec3::Y * 0.6,
                        along * 0.1 + n * 0.02 + Vec3::Y * 0.1,
                        0.0,
                        g * 0.9,
                        SHADE_GILT,
                    );
                }
                Face::Sconce => {
                    // A gilt back plate, an arm out of it carrying a drip pan and two candles.
                    let g = gold(hash(hx * 1.9, hy * 0.3));
                    // (Set in a shallow recess in the stone, standing no further out than the wall.)
                    let f = c + n * 0.46;
                    bx(out, f + n * 0.01 + Vec3::Y * 0.92, along * 0.14 + n * 0.02 + Vec3::Y * 0.3, 0.0, g * 0.85, SHADE_GILT);
                    bx(out, f + n * 0.03 + Vec3::Y * 0.94, along * 0.06 + n * 0.02 + Vec3::Y * 0.06, PI / 4.0, g, SHADE_GILT);
                    bx(out, f + n * 0.04 + Vec3::Y * 0.84, along * 0.04 + n * 0.06 + Vec3::Y * 0.04, 0.0, g * 0.9, SHADE_GILT);
                    bx(out, f + n * 0.05 + Vec3::Y * 0.875, along * 0.26 + n * 0.06 + Vec3::Y * 0.03, 0.0, g, SHADE_GILT);
                    for s in [-1.0f32, 1.0] {
                        let at = f + n * 0.05 + along * (s * 0.08);
                        bx(out, at + Vec3::Y * 0.94, Vec3::new(0.045, 0.1, 0.045), 0.0, lin(WAX), SHADE_LIT);
                    }
                    candles.push(f + n * 0.05 + Vec3::Y * 0.99);
                }
                Face::Drift => {
                    // A low drift of coin heaped against the wall's foot, a few coins slid out of it.
                    let h = hash(hx * 3.7 + dx as f64, hy * 2.9 + dy as f64) as f32;
                    let m = Mat4::from_translation(c + n * (0.42 + 0.04 * h))
                        * Mat4::from_rotation_y(turn)
                        * Mat4::from_scale(Vec3::new(1.5 + 0.4 * h, 0.55 + 0.3 * h, 0.75));
                    vault_decor::push_merged(out, &art.pile, m, GOLD_GLOW);
                    for k in 0..2 {
                        let u = (k as f32 - 0.5) * 0.5 + (h - 0.5) * 0.2;
                        coin_stack(
                            out,
                            Mat4::from_translation(c + n * 0.86 + along * u) * Mat4::from_scale(Vec3::new(1.0, 0.6, 1.0)),
                            h * 13.0 + k as f32,
                        );
                    }
                }
                Face::Niche(_) | Face::Hoard => {
                    // The niche's back lined in velvet, its sill and its soffit pale stone.
                    let back = c + n * 0.17;
                    bx(out, back + n * 0.01 + Vec3::Y * 0.8, along * 0.56 + n * 0.02 + Vec3::Y * 1.18, 0.0, lin(VELVET_D), SHADE_CLOTH);
                    bx(out, c + n * 0.34 + Vec3::Y * 0.24, along * 0.56 + n * 0.38 + Vec3::Y * 0.08, 0.0, pale, SHADE_STONE);
                    let g = gold(hash(hx * 0.7, hy * 2.3));
                    if let Face::Niche(s) = kind {
                        // A gilt plinth, and the statue on it, looking out; a gold shell over its head.
                        let at = c + n * 0.34 + Vec3::Y * 0.28;
                        bx(out, at + Vec3::Y * 0.04, along * 0.4 + n * 0.3 + Vec3::Y * 0.08, 0.0, lin(MARBLE_BLACK), SHADE_MARBLE);
                        bx(out, at + Vec3::Y * 0.085, along * 0.42 + n * 0.32 + Vec3::Y * 0.015, 0.0, g * 0.8, SHADE_GILT);
                        let m = Mat4::from_translation(at + Vec3::Y * 0.09)
                            * Mat4::from_rotation_y(yaw_of(d))
                            * Mat4::from_scale(Vec3::splat(0.85));
                        vault_decor::push_merged(out, &statues[s], m, 0.0);
                        for (k, w) in [0.5f32, 0.38, 0.24].iter().enumerate() {
                            bx(
                                out,
                                back + n * 0.03 + Vec3::Y * (1.06 + k as f32 * 0.05),
                                along * *w + n * 0.03 + Vec3::Y * 0.05,
                                0.0,
                                g,
                                SHADE_GILT,
                            );
                        }
                    } else {
                        // A rack of gold: two oak shelves, bars stacked on them and on the sill,
                        // a plate standing at the back, a goblet.
                        let wood = jit(OAK, hash(hx, hy * 3.0), 0.08);
                        for hh in [0.62f32, 0.98] {
                            bx(out, c + n * 0.3 + Vec3::Y * hh, along * 0.54 + n * 0.26 + Vec3::Y * 0.035, 0.0, wood, SHADE_WOOD);
                        }
                        for (lvl, hh) in [0.28f32, 0.655, 1.015].into_iter().enumerate() {
                            let rows: &[usize] = if lvl == 1 { &[2, 1] } else { &[3, 2, 1] };
                            for (r, &cnt) in rows.iter().enumerate() {
                                for k in 0..cnt {
                                    let u = (k as f32 - (cnt - 1) as f32 / 2.0) * 0.155;
                                    let bar = gold(hash(hx + lvl as f64 * 3.0 + k as f64, hy + r as f64));
                                    ingot(out, c + n * 0.33 + along * u + Vec3::Y * (hh + r as f32 * 0.05), dx == 0, bar);
                                }
                            }
                        }
                        // A plate on edge against the velvet, and a goblet on the top shelf.
                        let plate = c + n * 0.21 + along * 0.13 + Vec3::Y * 0.84;
                        for k in 0..2 {
                            let rot = Quat::from_rotation_y(turn) * Quat::from_rotation_z(k as f32 * PI / 4.0);
                            block_m(
                                out,
                                Mat4::from_scale_rotation_translation(Vec3::new(0.2, 0.2, 0.015), rot, plate),
                                g * 0.95,
                                SHADE_GILT,
                            );
                        }
                        let gob = c + n * 0.33 - along * 0.16 + Vec3::Y * 1.0;
                        bx(out, gob + Vec3::Y * 0.01, Vec3::new(0.07, 0.02, 0.07), PI / 4.0, g, SHADE_GILT);
                        bx(out, gob + Vec3::Y * 0.06, Vec3::new(0.025, 0.08, 0.025), 0.0, g, SHADE_GILT);
                        bx(out, gob + Vec3::Y * 0.13, Vec3::new(0.08, 0.07, 0.08), PI / 4.0, g * 1.05, SHADE_GILT);
                    }
                }
            }
        }
        candles
    }

    /// A gilded column on a pale stepped base: an octagonal shaft clad in gold in three lengths
    /// between dark bands, under a gilt capital and a pale abacus.
    pub(super) fn vault_column(&mut self, base: Vec3, h: f64) {
        let out = &mut self.static_cubes;
        let pale = jit(PALE[0], h, 0.05) * 0.8;
        bx(out, base + Vec3::Y * 0.07, Vec3::new(0.9, 0.14, 0.9), 0.0, pale * 0.9, SHADE_STONE);
        bx(out, base + Vec3::Y * 0.19, Vec3::new(0.74, 0.12, 0.74), 0.0, pale, SHADE_STONE);
        let g = gold(h);
        bx(out, base + Vec3::Y * 0.29, Vec3::new(0.6, 0.08, 0.6), PI / 4.0, g * 0.85, SHADE_GILT);
        for k in 0..3 {
            let y0 = 0.33 + k as f32 * 0.5;
            for t in [0.0f32, PI / 4.0] {
                bx(out, base + Vec3::Y * (y0 + 0.23), Vec3::new(0.44, 0.44, 0.44), t, g * (0.95 + 0.05 * k as f32), SHADE_GILT);
            }
            bx(out, base + Vec3::Y * (y0 + 0.475), Vec3::new(0.5, 0.05, 0.5), PI / 8.0, lin(IRON), SHADE_IRON);
        }
        bx(out, base + Vec3::Y * 1.88, Vec3::new(0.62, 0.12, 0.62), PI / 4.0, g * 1.05, SHADE_GILT);
        bx(out, base + Vec3::Y * 1.99, Vec3::new(0.86, 0.1, 0.86), 0.0, pale * 1.05, SHADE_STONE);
    }

    /// A copper fire bowl set on a column's top at `top`, its coals glowing. Returns where its fire burns.
    pub(super) fn vault_fire_bowl(&mut self, top: Vec3) -> Vec3 {
        let out = &mut self.static_cubes;
        let copper = lin(COPPER);
        bx(out, top + Vec3::Y * 0.05, Vec3::new(0.26, 0.1, 0.26), PI / 4.0, copper * 0.8, crypt::SHADE_BRONZE);
        bx(out, top + Vec3::Y * 0.16, Vec3::new(0.48, 0.14, 0.48), 0.0, copper, crypt::SHADE_BRONZE);
        bx(out, top + Vec3::Y * 0.16, Vec3::new(0.48, 0.14, 0.48), PI / 4.0, copper * 0.92, crypt::SHADE_BRONZE);
        bx(out, top + Vec3::Y * 0.24, Vec3::new(0.56, 0.04, 0.56), PI / 8.0, copper * 1.1, crypt::SHADE_BRONZE);
        crypt::glowing_block(out, top + Vec3::Y * 0.23, Vec3::new(0.36, 0.03, 0.36), lin(0x2a1408), lin(0xff7a28) * 0.45);
        top + Vec3::Y * 0.26
    }

    /// A tall gilt candelabrum with three candles (their flames are drawn as they move).
    pub(super) fn vault_candelabrum(&mut self, base: Vec3, h: f64) {
        let out = &mut self.static_cubes;
        let g = gold(h);
        for t in [0.0f32, PI / 4.0] {
            bx(out, base + Vec3::Y * 0.03, Vec3::new(0.34, 0.06, 0.07), t + PI / 8.0, g * 0.7, SHADE_GILT);
            bx(out, base + Vec3::Y * 0.03, Vec3::new(0.07, 0.06, 0.34), t + PI / 8.0, g * 0.7, SHADE_GILT);
        }
        bx(out, base + Vec3::Y * 0.12, Vec3::new(0.12, 0.12, 0.12), PI / 4.0, g * 0.85, SHADE_GILT);
        bx(out, base + Vec3::Y * 0.62, Vec3::new(0.055, 1.0, 0.055), 0.0, g, SHADE_GILT);
        bx(out, base + Vec3::Y * 0.45, Vec3::new(0.11, 0.07, 0.11), PI / 4.0, g * 0.9, SHADE_GILT);
        bx(out, base + Vec3::Y * 0.9, Vec3::new(0.1, 0.06, 0.1), PI / 4.0, g * 0.9, SHADE_GILT);
        bx(out, base + Vec3::Y * 1.14, Vec3::new(0.56, 0.045, 0.06), 0.0, g, SHADE_GILT);
        for dx in [-0.25f32, 0.0, 0.25] {
            bx(out, base + Vec3::new(dx, 1.18, 0.0), Vec3::new(0.11, 0.035, 0.11), PI / 4.0, g * 1.05, SHADE_GILT);
            bx(out, base + Vec3::new(dx, 1.26, 0.0), Vec3::new(0.05, 0.13, 0.05), 0.0, lin(WAX), SHADE_LIT);
        }
    }

    /// A copper brazier standing at `base`: a deep bowl on three legs, its coals glowing low in
    /// it (its fire is drawn as it moves).
    pub(super) fn vault_brazier(&mut self, base: Vec3) {
        let out = &mut self.static_cubes;
        let copper = lin(COPPER);
        for k in 0..3 {
            let a = k as f32 * TAU / 3.0 + 0.4;
            let foot = base + Vec3::new(a.cos() * 0.2, 0.0, a.sin() * 0.2);
            let top = base + Vec3::new(a.cos() * 0.12, 0.7, a.sin() * 0.12);
            let rot = Quat::from_rotation_arc(Vec3::Y, (top - foot).normalize());
            block_m(
                out,
                Mat4::from_scale_rotation_translation(Vec3::new(0.05, (top - foot).length(), 0.05), rot, (top + foot) / 2.0),
                copper * 0.7,
                crypt::SHADE_BRONZE,
            );
        }
        bx(out, base + Vec3::Y * 0.68, Vec3::new(0.3, 0.08, 0.3), PI / 4.0, copper * 0.85, crypt::SHADE_BRONZE);
        bx(out, base + Vec3::Y * 0.78, Vec3::new(0.46, 0.14, 0.46), 0.0, copper, crypt::SHADE_BRONZE);
        bx(out, base + Vec3::Y * 0.78, Vec3::new(0.46, 0.14, 0.46), PI / 4.0, copper * 0.92, crypt::SHADE_BRONZE);
        bx(out, base + Vec3::Y * 0.86, Vec3::new(0.54, 0.04, 0.54), PI / 8.0, copper * 1.1, crypt::SHADE_BRONZE);
        crypt::glowing_block(out, base + Vec3::Y * 0.85, Vec3::new(0.34, 0.03, 0.34), lin(0x2a1408), lin(0xff7a28) * 0.45);
    }
}

/// The turn that faces a model built facing +z toward `d`.
pub(super) fn yaw_of(d: Dir) -> f32 {
    let (dx, dz) = d.vec();
    (dx as f32).atan2(dz as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::{parse_level, Theme};
    use crate::logic::levels::level_for;
    use crate::logic::vault::{Vault, SEED};

    /// No block of a wall (a tree's range) that reaches above his knees lies over a floor tile, as
    /// the cameras count a block's tiles (`camera::footprint`: past a tile's edge by more than
    /// 0.05); that is, its faces' dressing keeps within `LIP` of the wall's middle.
    fn walls_keep_to_their_tiles(view: &WorldView, grid: &Grid) {
        for t in &view.trees {
            for c in &view.static_cubes[t.range.0..t.range.0 + t.range.1] {
                let m = Mat4::from_cols_array(&c.m);
                let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
                for k in 0..8 {
                    let l = Vec3::new(
                        if k & 1 == 0 { -0.5 } else { 0.5 },
                        if k & 2 == 0 { -0.5 } else { 0.5 },
                        if k & 4 == 0 { -0.5 } else { 0.5 },
                    );
                    let p = m.transform_point3(l);
                    lo = lo.min(p);
                    hi = hi.max(p);
                }
                if (hi - lo).max_element() < 0.12 || hi.y < 0.45 || lo.y > 2.6 {
                    continue;
                }
                let (x0, x1) = ((lo.x + 0.55).floor() as i32, (hi.x + 0.45).floor() as i32);
                let (z0, z1) = ((lo.z + 0.55).floor() as i32, (hi.z + 0.45).floor() as i32);
                for z in z0..=z1 {
                    for x in x0..=x1 {
                        assert!(
                            grid.get(x, z) == Tile::Rock,
                            "a block of the wall at {} reaches over the floor at {x},{z} ({lo} to {hi})",
                            t.base
                        );
                    }
                }
            }
        }
        const { assert!(LIP < 0.55) };
    }

    #[test]
    fn the_vaults_and_the_counting_house_keep_their_walls_off_the_floor() {
        let v = Vault::generate(SEED, 1);
        let data = v.level(&level_for(1));
        let level = parse_level(&data);
        let mut view = WorldView::new();
        view.build_level(&level.grid, &[], &[], data.theme, None, None, Some(&v), None);
        assert!(view.vault.is_some());
        walls_keep_to_their_tiles(&view, &level.grid);
        // (A budget: the vaults were 247,000 cubes before they were rebuilt.)
        assert!(view.static_cubes.len() < 200_000, "{} cubes", view.static_cubes.len());

        let data = crate::logic::court::greed::counting_house(1);
        let level = parse_level(&data);
        let mut view = WorldView::new();
        view.build_level(&level.grid, &[], &[], Theme::Court(crate::logic::sins::Sin::Greed), None, None, None, None);
        assert!(view.counting.is_some());
        walls_keep_to_their_tiles(&view, &level.grid);
    }
}
