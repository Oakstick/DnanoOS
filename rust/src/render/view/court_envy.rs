//! Envy's court, **the Drowned Bell** (`logic::court::envy` lays it out), where Leviathan waits:
//! its art, in the Serpent Grotto's materials (wet green-black stone, bleached serpent bone,
//! verdigris bronze, jade water) with Envy's green as its accent.
//!
//! A drowned temple, round: walls of green-black ashlar, algae at their foot, the vertebrae of a
//! great serpent set along them with its ribs arching up between; a ring of shallow jade water
//! round the central floor, six broken columns standing in it; flagstones within, an ouroboros
//! of jade tesserae coiled round the middle and a ring of pale stone inlaid where the bell's toll
//! reaches. The bronze bell (Dōjō-ji's, that Kiyohime coiled round) hangs in the middle from a
//! cracked beam on two stone posts, its mouth an arm's height over the floor; green with
//! verdigris, it swings when he tolls it and glows red with heat while he coils round it. At the
//! north end the drowned altar: three steps, a coiled stone serpent, two stone lanterns burning
//! green. Blue nazar beads hang on cords from the dark all round, against the evil eye. The gate
//! he came in by is Envy's portal in the south wall.
//!
//! Each frame: the water (its ripples, the springs bubbling, the light it throws on the stone),
//! the lanterns, the bell, and what Leviathan's fight puts on the floor that is Envy's own (his
//! burning coil's fires, the coveted thunder's marks, the water churning over his dive).

use super::*;
use crate::logic::combat::prince::envy::{fire_spots, thunder_marks, FIRE_RADIUS, FLAMES};
use crate::logic::combat::{MoveId, Phase, State};
use crate::logic::court::envy::{self as plan, Piece, ALTAR, BELL, BELL_RIM, CENTRE, SPRINGS, WATER as SHORES};
use crate::logic::hub::Gate;

mod pal {
    pub const STONE: u32 = 0x1e2824;
    pub const STONE_L: u32 = 0x2a3631;
    pub const STONE_D: u32 = 0x131a17;
    pub const ASHLAR: u32 = 0x24302b;
    pub const MOSS: u32 = 0x2a4a38;
    pub const FRINGE: u32 = 0x3a6a5a;
    pub const JADE: u32 = 0x46806a;
    pub const JADE_L: u32 = 0x6aa88a;
    pub const PALE: u32 = 0x9aa894;
    pub const VERDIGRIS: u32 = 0x4aa088;
    pub const VERDIGRIS_D: u32 = 0x2e6a5a;
    pub const BRONZE: u32 = 0xa8743a;
    pub const BRONZE_D: u32 = 0x6a4a26;
    pub const BONE: u32 = 0xe6dcc0;
    pub const BONE_D: u32 = 0xb8ac8a;
    pub const WOOD: u32 = 0x2e241c;
    pub const WOOD_L: u32 = 0x4a3a2a;
    pub const CORD: u32 = 0x3a3024;
    pub const WATER: u32 = 0x2a6a5e;
    pub const SEABED: u32 = 0x1a2a24;
    pub const NAZAR: u32 = 0x1e48c8;
    pub const NAZAR_L: u32 = 0x7ab8ff;
    pub const FLAME: u32 = 0x9aff8a;
    pub const FLAME_TIP: u32 = 0x3ad06a;
    pub const LANTERN: u32 = 0x8af0a0;
    pub const ENVY: u32 = 0x5adf7a;
    pub const HEAT: u32 = 0xff6a2a;
    pub const FIRE: u32 = 0xff9a3a;
    pub const FIRE_TIP: u32 = 0xffe070;
}
use pal::*;

/// The voxel materials' shade modes (as the grotto's): stone, ivory, bronze, wet, wood.
const SHADE_STONE: f32 = 18.0;
const SHADE_BONE: f32 = 11.0;
const SHADE_BRONZE: f32 = 13.0;
const SHADE_WET: f32 = 14.0;
const SHADE_WOOD: f32 = 12.0;

/// The hall's walls stand this tall; the bell's posts this; the floor under the water lies this low.
const WALL_H: f32 = 2.6;
const POST_H: f32 = 4.9;
const SEABED_Y: f32 = -0.14;
const WATER_Y: f32 = -0.02;
/// The bell: its mouth over the floor, its height, and where it hangs from.
const BELL_MOUTH: f32 = 0.85;
const BELL_HEIGHT: f32 = 2.7;
/// The ring of pale stone where the toll reaches, and the ouroboros's radius.
const TOLL_RING: f32 = 3.6;
const OUROBOROS: f32 = 6.2;

/// The court's light: dark and cold, the deep's teal from above, jade off the water below.
pub fn envy_atmosphere() -> Atmosphere {
    Atmosphere {
        background: 0x040a08,
        fog_near: 16.0,
        fog_far: 44.0,
        hemi_sky: lin(0x7ea49c) * 0.62,
        hemi_ground: lin(0x3a6a50) * 0.55,
        ambient: lin(0x2c3c36) * 1.15,
        sun_dir: Vec3::new(-0.3, 1.0, 0.45).normalize(),
        sun: lin(0xc8e0d4) * 0.95,
        exposure: 1.25,
        night_sky: 0.0,
        bloom: 0.9,
        moon_view: Vec3::new(0.0, 0.3, -1.0).normalize(),
        cloud_tint: lin(0x040a08),
    }
}

/// Its ground haze: a cold breath off the water (density and falloff).
pub const HAZE: (f32, f32) = (0.016, 1.6);

/// What the view keeps of the Drowned Bell for its moving parts.
#[derive(Clone, Debug)]
pub struct BellView {
    pub gate: Gate,
    /// The green-burning lanterns: the altar's two and the sconces round the wall.
    pub lanterns: Vec<Vec3>,
    /// The nazar cords: where each hangs from, and how far down its lowest bead is.
    pub cords: Vec<(Vec3, f32)>,
    /// The water's tiles (their middles, on the surface).
    pub water: Vec<Vec3>,
}

fn cube(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot_y: f32, color: Vec3, shade: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(rot_y), pos), color, 1.0, Vec3::ZERO, shade));
}

/// A beam of `thick` square section from `a` to `b`.
fn bar(out: &mut Vec<Instance>, a: Vec3, b: Vec3, thick: f32, color: Vec3, shade: f32) {
    let mid = (a + b) * 0.5;
    let len = (b - a).length();
    let rot = Quat::from_rotation_arc(Vec3::Y, (b - a).normalize_or_zero());
    out.push(Instance::new(Mat4::from_scale_rotation_translation(Vec3::new(thick, len, thick), rot, mid), color, 1.0, Vec3::ZERO, shade));
}

/// Green-black stone, a little different slab to slab.
fn stone(h: f64) -> Vec3 {
    let set = [STONE, STONE, STONE_L, STONE_D];
    lin(set[(h * 4.0) as usize % 4]) * (0.92 + h as f32 * 0.12)
}

impl WorldView {
    /// The Drowned Bell's static scene.
    pub(super) fn build_envy_court(&mut self, grid: &Grid) {
        let c = Vec3::new(CENTRE.0 as f32, 0.0, CENTRE.1 as f32);
        let floor_tile = |x: i32, y: i32| plan::in_hall(x, y);
        let gate_tile = |x: i32, y: i32| x == plan::GATE.0 && y == plan::GATE.1;
        let mut water = Vec::new();

        // ---- the floor: flagstones within, the seabed under the water's ring, the stone rim
        for y in 0..grid.height {
            for x in 0..grid.width {
                if !floor_tile(x, y) && !gate_tile(x, y) {
                    continue;
                }
                let (xf, yf) = (x as f32, y as f32);
                let h = hash(x as f64 * 1.3 + 7.0, y as f64 * 2.9);
                let out = &mut self.static_cubes;
                if plan::in_water(xf as f64, yf as f64) {
                    // The seabed: dark stone and silt, a little lower, the water drawn over it.
                    cube(
                        out,
                        Vec3::new(xf, SEABED_Y - 0.06, yf),
                        Vec3::new(1.0, 0.12, 1.0),
                        0.0,
                        lin(SEABED) * (0.85 + h as f32 * 0.3),
                        SHADE_WET,
                    );
                    if h > 0.7 {
                        // A stone fallen from the walls long ago, half sunk.
                        let a = h as f32 * 9.0;
                        cube(
                            out,
                            Vec3::new(xf + (h as f32 - 0.85) * 0.8, SEABED_Y + 0.02, yf - 0.2),
                            Vec3::new(0.32, 0.1, 0.24),
                            a,
                            stone(h) * 0.8,
                            SHADE_WET,
                        );
                    }
                    water.push(Vec3::new(xf, WATER_Y, yf));
                    continue;
                }
                let d = Vec2::new(xf - c.x, yf - c.z).length();
                let rim = d > SHORES.1 as f32;
                let col = if rim { lin(ASHLAR) * (0.9 + h as f32 * 0.2) } else { stone(h) };
                cube(out, Vec3::new(xf, -0.06, yf), Vec3::new(0.97, 0.12, 0.97), 0.0, col, SHADE_STONE);
                // Moss in the joints, and here and there a crack.
                if h < 0.35 {
                    let a = (h * 17.0) as f32;
                    cube(
                        out,
                        Vec3::new(xf + 0.49, 0.002, yf + (h as f32 - 0.17) * 2.0),
                        Vec3::new(0.03, 0.006, 0.5),
                        a * 0.1,
                        lin(MOSS),
                        SHADE_WET,
                    );
                }
                if h > 0.86 && !rim {
                    let a = (h * 31.0) as f32;
                    let o = Vec3::new(xf + (h as f32 - 0.93) * 3.0, 0.003, yf);
                    let dir = Vec3::new(a.cos(), 0.0, -a.sin());
                    for (k, bend) in [(-1.0f32, 0.3f32), (0.0, 0.0), (1.0, -0.25)] {
                        cube(out, o + dir * (k * 0.22), Vec3::new(0.24, 0.006, 0.014), a + bend, lin(STONE_D) * 0.6, SHADE_STONE);
                    }
                }
            }
        }
        // The water's edge: a lip of worn stone along both shores, and algae just under it.
        for (r, out_side) in [(SHORES.0 as f32 - 0.5, true), (SHORES.1 as f32 + 0.5, false)] {
            let n = (TAU * r / 0.42).round() as i32;
            for k in 0..n {
                let a = k as f32 * TAU / n as f32;
                let p = c + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
                let (px, pz) = (p.x.round() as i32, p.z.round() as i32);
                if !floor_tile(px, pz) || plan::piece(px, pz).is_some() {
                    continue;
                }
                let h = hash(k as f64 * 3.1, r as f64) as f32;
                cube(
                    &mut self.static_cubes,
                    p + Vec3::new(0.0, -0.035, 0.0),
                    Vec3::new(0.44, 0.07, 0.2),
                    -a + PI / 2.0,
                    lin(STONE_L) * (0.9 + h * 0.2),
                    SHADE_STONE,
                );
                let o = if out_side { 0.16 } else { -0.16 };
                let q = c + Vec3::new(a.cos() * (r + o), SEABED_Y + 0.05, a.sin() * (r + o));
                cube(&mut self.static_cubes, q, Vec3::new(0.4, 0.05, 0.14), -a + PI / 2.0, lin(FRINGE) * (0.8 + h * 0.4), SHADE_WET);
            }
        }
        // The ouroboros: a serpent of jade tesserae coiled round the middle, its head at the north
        // biting its tail; the scales along its back paler.
        let n = (TAU * OUROBOROS / 0.26).round() as i32;
        for k in 0..n {
            let u = k as f32 / n as f32;
            // (From its head at the north, round by the east, thinning to the tail in its mouth.)
            let a = -PI / 2.0 + 0.12 + u * (TAU - 0.24);
            let girth = 0.5 * (1.0 - u * 0.75) + 0.08;
            for (j, off) in [-1.0f32, 0.0, 1.0].into_iter().enumerate() {
                let r = OUROBOROS + off * girth * 0.4;
                let p = c + Vec3::new(a.cos() * r, 0.004, a.sin() * r);
                let col = if j == 1 {
                    if k % 3 == 0 {
                        lin(JADE_L)
                    } else {
                        lin(JADE)
                    }
                } else {
                    lin(VERDIGRIS_D)
                };
                cube(&mut self.static_cubes, p, Vec3::new(0.22, 0.014, 0.18 * girth / 0.58 + 0.06), -a + PI / 2.0, col, SHADE_BRONZE);
            }
        }
        // Its head: a wedge of jade, two eyes of the envious green.
        let head = c + Vec3::new(0.0, 0.006, -OUROBOROS);
        cube(&mut self.static_cubes, head, Vec3::new(0.72, 0.016, 0.5), 0.0, lin(JADE), SHADE_BRONZE);
        cube(&mut self.static_cubes, head + Vec3::new(0.48, 0.0, 0.0), Vec3::new(0.34, 0.016, 0.34), PI / 4.0, lin(JADE), SHADE_BRONZE);
        for s in [-1.0f32, 1.0] {
            let e = head + Vec3::new(0.12, 0.004, s * 0.14);
            self.static_cubes.push(Instance::new(
                Mat4::from_translation(e) * Mat4::from_scale(Vec3::new(0.09, 0.012, 0.09)),
                lin(ENVY),
                1.0,
                lin(ENVY) * 0.8,
                SHADE_LIT,
            ));
        }
        // The pale ring where the toll reaches, and a band of bronze about the bell's foot.
        for (r, col, w, shade) in [(TOLL_RING, PALE, 0.08, SHADE_STONE), (BELL_RIM as f32 + 0.32, BRONZE_D, 0.12, SHADE_BRONZE)] {
            let n = (TAU * r / 0.3).round() as i32;
            for k in 0..n {
                let a = k as f32 * TAU / n as f32;
                let p = c + Vec3::new(a.cos() * r, 0.003, a.sin() * r);
                cube(&mut self.static_cubes, p, Vec3::new(0.31, 0.01, w), -a + PI / 2.0, lin(col), shade);
            }
        }

        // ---- the walls: green-black ashlar, a serpent's vertebrae set along them, ribs between
        let mut lanterns = Vec::new();
        for (x, y, t) in grid.cells() {
            if t != Tile::Rock || floor_tile(x, y) || gate_tile(x, y) {
                continue;
            }
            let open: Vec<Dir> = CARDINALS
                .iter()
                .copied()
                .filter(|d| {
                    let n = P::new(x, y).step(*d);
                    floor_tile(n.x, n.y)
                })
                .collect();
            let near = !open.is_empty() || [(-1, -1), (1, -1), (-1, 1), (1, 1)].iter().any(|&(dx, dy)| floor_tile(x + dx, y + dy));
            if !near || (y == plan::GATE.1 && (x - plan::GATE.0).abs() <= 1) {
                continue;
            }
            let start = self.static_cubes.len();
            let (xf, yf) = (x as f32, y as f32);
            let h = hash(x as f64 * 3.7, y as f64 * 1.9);
            let out = &mut self.static_cubes;
            // Courses of ashlar, each a little different, the foot green with algae.
            for k in 0..4 {
                let hk = hash(x as f64 * 1.1 + k as f64, y as f64 * 0.7) as f32;
                let yy = k as f32 * 0.65 + 0.325;
                let col = if k == 0 { lin(MOSS).lerp(lin(ASHLAR), 0.45) } else { lin(ASHLAR) * (0.82 + hk * 0.3) };
                cube(out, Vec3::new(xf, yy, yf), Vec3::new(1.0, 0.63, 1.0), 0.0, col, SHADE_STONE);
            }
            cube(out, Vec3::new(xf, WALL_H + 0.05, yf), Vec3::new(1.08, 0.1, 1.08), 0.0, lin(STONE_D), SHADE_STONE);
            for &d in &open {
                let (dx, dy) = d.vec();
                let nrm = Vec3::new(dx as f32, 0.0, dy as f32);
                let along = if dx == 0 { Vec3::X } else { Vec3::Z };
                let face = Vec3::new(xf, 0.0, yf) + nrm * 0.5;
                let size = |a: f32, up: f32, o: f32| along * a + Vec3::Y * up + Vec3::new(nrm.x.abs(), 0.0, nrm.z.abs()) * o;
                // The vertebrae: a chain of bone along the wall, each a rounded body, its spine
                // swept back along the chain and its two wings angled down.
                for j in 0..2 {
                    let at = face + along * (j as f32 * 0.5 - 0.25) + Vec3::new(0.0, 1.25, 0.0) + nrm * 0.05;
                    let bone = lin(BONE) * (0.85 + h as f32 * 0.2);
                    for turn in [0.0f32, PI / 4.0] {
                        let q = Quat::from_axis_angle(along, turn);
                        out.push(Instance::new(
                            Mat4::from_scale_rotation_translation(size(0.28, 0.2, 0.2), q, at),
                            bone,
                            1.0,
                            Vec3::ZERO,
                            SHADE_BONE,
                        ));
                    }
                    let lean = Quat::from_axis_angle(nrm.cross(Vec3::Y).normalize_or_zero(), 0.5);
                    out.push(Instance::new(
                        Mat4::from_scale_rotation_translation(size(0.07, 0.3, 0.06), lean, at + Vec3::new(0.0, 0.22, 0.0) + along * 0.06),
                        lin(BONE_D),
                        1.0,
                        Vec3::ZERO,
                        SHADE_BONE,
                    ));
                    for side in [-1.0f32, 1.0] {
                        let wing = Quat::from_axis_angle(nrm, side * 0.9);
                        out.push(Instance::new(
                            Mat4::from_scale_rotation_translation(
                                size(0.05, 0.22, 0.05),
                                wing,
                                at + Vec3::new(0.0, -0.08, 0.0) + along * (side * 0.14),
                            ),
                            lin(BONE_D) * 0.9,
                            1.0,
                            Vec3::ZERO,
                            SHADE_BONE,
                        ));
                    }
                }
                // A rib arching up from every other, curving over toward the hall.
                if (x + y) % 2 == 0 {
                    for k in 0..6 {
                        let u = k as f32 / 5.0;
                        let p = face + Vec3::new(0.0, 1.35 + u * 1.1, 0.0) + nrm * (0.06 + (u * PI * 0.5).sin() * 0.3);
                        cube(out, p, size(0.07, 0.24, 0.07), 0.0, lin(BONE) * (0.8 + 0.2 * (1.0 - u)), SHADE_BONE);
                    }
                }
            }
            let len = self.static_cubes.len() - start;
            self.rock_index.insert(tile_key(x, y), (start, len));
            let shades: Vec<f32> = self.static_cubes[start..start + len].iter().map(|i| i.emissive[3]).collect();
            self.trees.push(Tree {
                range: (start, len),
                base: Vec3::new(xf, 0.0, yf),
                radius: 0.72,
                height: WALL_H + 0.3,
                fade: 1.0,
                alive: true,
                shades: Some(shades),
            });
        }
        // Sconces burning green round the wall, between the columns (eight), out of the way.
        for k in 0..8 {
            let a = (k as f32 + 0.5) * TAU / 8.0;
            let p = c + Vec3::new(a.cos() * 13.1, 1.75, a.sin() * 13.1);
            if (p.z - plan::GATE.1 as f32).abs() < 3.0 && (p.x - plan::GATE.0 as f32).abs() < 2.5 {
                continue;
            }
            let out = &mut self.static_cubes;
            let inward = (c - p).normalize_or_zero();
            cube(out, p - Vec3::new(0.0, 0.12, 0.0), Vec3::new(0.3, 0.08, 0.3), 0.0, lin(BRONZE_D), SHADE_BRONZE);
            cube(out, p - Vec3::new(0.0, 0.32, 0.0) - inward * 0.12, Vec3::new(0.08, 0.36, 0.08), 0.0, lin(BRONZE_D), SHADE_BRONZE);
            lanterns.push(p);
        }

        // ---- the columns, the altar and the bell's frame
        for (x, y, _) in grid.cells() {
            let Some(piece) = plan::piece(x, y) else { continue };
            let base = Vec3::new(x as f32, 0.0, y as f32);
            let h = hash(x as f64 * 2.3, y as f64 * 1.7) as f32;
            let start = self.static_cubes.len();
            let tall = match piece {
                Piece::Pillar => self.envy_column(base, h),
                Piece::Altar => {
                    self.envy_altar_step(x, y, base, h);
                    0.0
                }
                Piece::Bell => 0.0,
            };
            if tall > 0.0 {
                let len = self.static_cubes.len() - start;
                self.rock_index.insert(tile_key(x, y), (start, len));
                let shades: Vec<f32> = self.static_cubes[start..start + len].iter().map(|i| i.emissive[3]).collect();
                self.trees.push(Tree {
                    range: (start, len),
                    base,
                    radius: 0.5,
                    height: tall,
                    fade: 1.0,
                    alive: true,
                    shades: Some(shades),
                });
            }
        }
        lanterns.extend(self.envy_altar());
        self.envy_bell_frame();

        // ---- the nazar cords, hanging from the dark all round, over the altar and from the beam
        let mut cords = Vec::new();
        for k in 0..14 {
            let a = (k as f32 + 0.25) * TAU / 14.0;
            let r = 12.6 + 0.4 * ((k * 5) % 3) as f32;
            let top = c + Vec3::new(a.cos() * r, 5.5, a.sin() * r);
            cords.push((top, 2.6 + 0.5 * ((k * 7) % 4) as f32));
        }
        for x in [14.6f32, 17.4] {
            cords.push((Vec3::new(x, 5.0, ALTAR.1 as f32 + 0.4), 2.2));
        }
        // ---- the gate: Envy's portal in the south wall, looking into the hall
        let gate = Gate { tile: P::new(plan::GATE.0, plan::GATE.1), facing: Dir::Up, sin: crate::logic::sins::Sin::Envy };
        self.build_court_portal(&gate);
        let _ = SPRINGS;
        self.bell = Some(BellView { gate, lanterns, cords, water });
    }

    /// A broken column standing in the water: a plinth green with algae, drums of green-black
    /// stone banded in verdigris bronze, its top broken off jagged at its own height. Returns it.
    fn envy_column(&mut self, base: Vec3, h: f32) -> f32 {
        let out = &mut self.static_cubes;
        cube(
            out,
            base + Vec3::new(0.0, SEABED_Y + 0.12, 0.0),
            Vec3::new(0.95, 0.26, 0.95),
            0.0,
            lin(MOSS).lerp(lin(STONE), 0.4),
            SHADE_WET,
        );
        cube(out, base + Vec3::new(0.0, 0.18, 0.0), Vec3::new(0.82, 0.14, 0.82), PI / 4.0, lin(VERDIGRIS_D), SHADE_BRONZE);
        let drums = 5 + (h * 6.0) as usize;
        let mut y = 0.25;
        for k in 0..drums {
            let tall = 0.38;
            let col = lin(STONE_L) * (0.85 + 0.15 * ((k + (h * 9.0) as usize) % 3) as f32 / 2.0);
            cube(out, base + Vec3::new(0.0, y + tall / 2.0, 0.0), Vec3::new(0.6, tall, 0.6), k as f32 * 0.21 + h, col, SHADE_STONE);
            if k % 3 == 1 {
                cube(
                    out,
                    base + Vec3::new(0.0, y + tall - 0.02, 0.0),
                    Vec3::new(0.66, 0.06, 0.66),
                    k as f32 * 0.21 + h,
                    lin(VERDIGRIS),
                    SHADE_BRONZE,
                );
            }
            y += tall;
        }
        // The break: two jagged stubs at different heights.
        cube(out, base + Vec3::new(0.12, y + 0.12, -0.08), Vec3::new(0.32, 0.24, 0.4), h * 3.0, lin(STONE_L) * 0.9, SHADE_STONE);
        cube(out, base + Vec3::new(-0.15, y + 0.05, 0.12), Vec3::new(0.26, 0.1, 0.3), h * 5.0, lin(STONE) * 0.9, SHADE_STONE);
        // Algae where the water meets it.
        for k in 0..6 {
            let a = k as f32 * TAU / 6.0 + h;
            cube(
                out,
                base + Vec3::new(a.cos() * 0.31, 0.04, a.sin() * 0.31),
                Vec3::new(0.2, 0.12, 0.05),
                -a + PI / 2.0,
                lin(FRINGE),
                SHADE_WET,
            );
        }
        y + 0.3
    }

    /// One of the altar's tiles: three steps of stone up from the south.
    fn envy_altar_step(&mut self, _x: i32, y: i32, base: Vec3, h: f32) {
        let out = &mut self.static_cubes;
        let back = y == ALTAR.1;
        let tall = if back { 0.6 } else { 0.3 };
        cube(out, base + Vec3::new(0.0, tall / 2.0, 0.0), Vec3::new(1.0, tall, 1.0), 0.0, stone(h as f64) * 1.05, SHADE_STONE);
        cube(out, base + Vec3::new(0.0, tall + 0.015, 0.47), Vec3::new(1.0, 0.03, 0.06), 0.0, lin(VERDIGRIS_D), SHADE_BRONZE);
    }

    /// The altar's crown: a coiled serpent of stone in the middle, its head raised to the hall,
    /// and a stone lantern at each end. Returns the lanterns' flames.
    fn envy_altar(&mut self) -> Vec<Vec3> {
        let mid = Vec3::new((ALTAR.0 + ALTAR.2) as f32 / 2.0, 0.6, ALTAR.1 as f32);
        let out = &mut self.static_cubes;
        // The coil: three turns of stone, narrowing upward.
        for turn in 0..3 {
            let r = 0.7 - turn as f32 * 0.17;
            let y = mid.y + 0.12 + turn as f32 * 0.2;
            let n = 14;
            for k in 0..n {
                let a = k as f32 * TAU / n as f32 + turn as f32 * 0.3;
                cube(
                    out,
                    mid + Vec3::new(a.cos() * r, y - mid.y, a.sin() * r),
                    Vec3::new(0.24, 0.2, 0.2),
                    -a,
                    lin(STONE_L) * (0.9 + 0.1 * (k % 2) as f32),
                    SHADE_STONE,
                );
            }
        }
        // The neck up from the coil, the head looking south down the hall, its eyes green.
        for k in 0..5 {
            let u = k as f32 / 4.0;
            cube(out, mid + Vec3::new(0.0, 0.8 + u * 0.7, 0.1 + u * 0.25), Vec3::new(0.2, 0.22, 0.2), 0.0, lin(STONE_L), SHADE_STONE);
        }
        let head = mid + Vec3::new(0.0, 1.6, 0.48);
        cube(out, head, Vec3::new(0.34, 0.22, 0.44), 0.0, lin(STONE_L), SHADE_STONE);
        cube(out, head + Vec3::new(0.0, -0.08, 0.26), Vec3::new(0.24, 0.12, 0.2), 0.0, lin(STONE), SHADE_STONE);
        for s in [-1.0f32, 1.0] {
            let e = head + Vec3::new(s * 0.15, 0.05, 0.14);
            out.push(Instance::new(
                Mat4::from_translation(e) * Mat4::from_scale(Vec3::splat(0.06)),
                lin(ENVY),
                1.0,
                lin(ENVY) * 1.6,
                SHADE_LIT,
            ));
        }
        // The stone lanterns at its ends: a foot, a post, the firebox, a roof and its knob.
        let mut flames = Vec::new();
        for x in [ALTAR.0 as f32, ALTAR.2 as f32] {
            let b = Vec3::new(x, 0.6, ALTAR.1 as f32 + 0.1);
            cube(out, b + Vec3::new(0.0, 0.08, 0.0), Vec3::new(0.56, 0.16, 0.56), PI / 4.0, lin(STONE_L), SHADE_STONE);
            cube(out, b + Vec3::new(0.0, 0.5, 0.0), Vec3::new(0.2, 0.7, 0.2), 0.0, lin(STONE), SHADE_STONE);
            cube(out, b + Vec3::new(0.0, 0.9, 0.0), Vec3::new(0.5, 0.1, 0.5), 0.0, lin(STONE_L), SHADE_STONE);
            for (dx, dz) in [(-0.2f32, -0.2f32), (0.2, -0.2), (-0.2, 0.2), (0.2, 0.2)] {
                cube(out, b + Vec3::new(dx, 1.12, dz), Vec3::new(0.07, 0.34, 0.07), 0.0, lin(STONE), SHADE_STONE);
            }
            cube(out, b + Vec3::new(0.0, 1.34, 0.0), Vec3::new(0.72, 0.1, 0.72), PI / 4.0, lin(STONE_L), SHADE_STONE);
            cube(out, b + Vec3::new(0.0, 1.44, 0.0), Vec3::new(0.44, 0.1, 0.44), PI / 4.0, lin(STONE_L) * 0.9, SHADE_STONE);
            cube(out, b + Vec3::new(0.0, 1.55, 0.0), Vec3::new(0.12, 0.12, 0.12), PI / 4.0, lin(VERDIGRIS), SHADE_BRONZE);
            flames.push(b + Vec3::new(0.0, 1.06, 0.0));
        }
        flames
    }

    /// The bell's frame: two posts of stone on the bell's ground, north-west and south-east, and
    /// the beam across them cracked over the middle, sagging, a chain down from the crack.
    fn envy_bell_frame(&mut self) {
        let c = Vec3::new(BELL.x as f32, 0.0, BELL.y as f32);
        let posts = [c + Vec3::new(-1.3, 0.0, -1.3), c + Vec3::new(1.3, 0.0, 1.3)];
        for (k, p) in posts.into_iter().enumerate() {
            let start = self.static_cubes.len();
            let out = &mut self.static_cubes;
            cube(out, p + Vec3::new(0.0, 0.15, 0.0), Vec3::new(0.5, 0.3, 0.5), 0.0, lin(STONE_L), SHADE_STONE);
            for j in 0..7 {
                let y = 0.3 + j as f32 * 0.65;
                cube(
                    out,
                    p + Vec3::new(0.0, y + 0.32, 0.0),
                    Vec3::new(0.34, 0.64, 0.34),
                    j as f32 * 0.1,
                    lin(STONE) * (0.9 + 0.1 * (j % 2) as f32),
                    SHADE_STONE,
                );
            }
            cube(out, p + Vec3::new(0.0, POST_H + 0.1, 0.0), Vec3::new(0.5, 0.2, 0.5), PI / 4.0, lin(STONE_L), SHADE_STONE);
            // Nazar cords tied round each, under the beam.
            cube(out, p + Vec3::new(0.0, POST_H - 0.6, 0.0), Vec3::new(0.38, 0.05, 0.38), 0.0, lin(CORD), SHADE_WOOD);
            let len = self.static_cubes.len() - start;
            let shades: Vec<f32> = self.static_cubes[start..start + len].iter().map(|i| i.emissive[3]).collect();
            let _ = k;
            self.trees.push(Tree {
                range: (start, len),
                base: p,
                radius: 0.4,
                height: POST_H + 0.3,
                fade: 1.0,
                alive: true,
                shades: Some(shades),
            });
        }
        // The beam: two timbers meeting over the middle, cracked there and sagging into it.
        let top = POST_H + 0.32;
        let mid = c + Vec3::new(0.0, top - 0.22, 0.0);
        let out = &mut self.static_cubes;
        for p in posts {
            let a = p + Vec3::new(0.0, top, 0.0);
            bar(out, a, mid, 0.32, lin(WOOD), SHADE_WOOD);
            bar(out, a + Vec3::new(0.0, 0.12, 0.0), mid + Vec3::new(0.0, 0.12, 0.0), 0.12, lin(WOOD_L), SHADE_WOOD);
        }
        // The crack: splinters standing out of it.
        for k in 0..5 {
            let a = k as f32 * 1.3;
            bar(out, mid, mid + Vec3::new(a.cos() * 0.25, 0.2 + (k % 2) as f32 * 0.1, a.sin() * 0.25), 0.05, lin(WOOD_L), SHADE_WOOD);
        }
        // A band of bronze round the break, and the heavy chain down to the bell.
        cube(out, mid, Vec3::new(0.46, 0.4, 0.46), PI / 4.0, lin(VERDIGRIS_D), SHADE_BRONZE);
        let crown = BELL_MOUTH + BELL_HEIGHT + 0.5;
        let links = ((mid.y - crown) / 0.3).max(1.0) as i32;
        for k in 0..links {
            let y = mid.y - 0.2 - k as f32 * 0.3;
            cube(
                out,
                Vec3::new(c.x, y, c.z),
                Vec3::new(0.1, 0.3, 0.24),
                if k % 2 == 0 { 0.0 } else { PI / 2.0 },
                lin(0x2a2a26),
                SHADE_BRONZE,
            );
        }
    }

    /// Each frame: the water, the lanterns, the nazar cords, the bell, Leviathan's own light and
    /// what his fight puts on the floor, and the gate.
    pub(super) fn emit_envy_court(&self, s: &SceneState, time: f32, hero: Vec3, f: &mut Frame) {
        let Some(bv) = &self.bell else { return };
        f.bounds = Vec4::new(-0.5, -0.5, s.grid.width as f32 - 0.5, s.grid.height as f32 - 0.5);
        let cube_i = MeshKind::Cube as usize;
        let c = Vec3::new(BELL.x as f32, 0.0, BELL.y as f32);

        // ---- the water: a jade sheen over the seabed, slowly turning round the hall
        for (i, p) in bv.water.iter().enumerate() {
            let to = Vec2::new(p.x - c.x, p.z - c.z).normalize_or_zero();
            let flow = Vec3::new(-to.y * 0.05, to.x * 0.05, 0.0);
            let m = Mat4::from_translation(*p) * Mat4::from_scale(Vec3::new(1.0, 0.002, 1.0));
            let _ = i;
            f.transparent[cube_i].push(Instance::new(m, lin(WATER), 0.55, flow, SHADE_WATER));
        }
        // Rings spreading where drops fall from the dark, and the springs bubbling.
        for k in 0..10 {
            let fk = k as f32;
            let t = (time * 0.45 + fk * 0.37).fract();
            let cycle = (time * 0.45 + fk * 0.37).floor();
            let a = hash(cycle as f64 * 1.7, fk as f64) as f32 * TAU;
            let r = SHORES.0 as f32 + 0.4 + hash(fk as f64, cycle as f64 * 2.3) as f32 * (SHORES.1 - SHORES.0) as f32 * 0.8;
            let at = c + Vec3::new(a.cos() * r, WATER_Y + 0.01, a.sin() * r);
            for j in 0..2 {
                let rr = 0.05 + (t - j as f32 * 0.15).max(0.0) * 0.6;
                let m = Mat4::from_translation(at) * Mat4::from_rotation_x(-PI / 2.0) * Mat4::from_scale(Vec3::new(rr, rr, 1.0));
                f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, lin(0x9fc8b8), 0.35 * (1.0 - t), Vec3::ZERO, SHADE_UNLIT));
            }
        }
        for (k, &(x, y)) in SPRINGS.iter().enumerate() {
            let at = Vec3::new(x as f32, WATER_Y, y as f32);
            for j in 0..4 {
                let t = (time * 0.9 + j as f32 * 0.25 + k as f32 * 0.13).fract();
                let a = (j as f32 * 2.1 + k as f32) + time * 0.3;
                let p = at + Vec3::new(a.cos() * 0.18, 0.02 + t * 0.12, a.sin() * 0.18);
                f.additive.push(Instance::new(
                    Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.035 * (1.0 - t) + 0.01)),
                    lin(0xb8f0d8),
                    0.5 * (1.0 - t),
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
        }
        // The light off the water crawling over the floor near him.
        let (hx, hz) = (hero.x.round() as i32, hero.z.round() as i32);
        for z in hz - 7..=hz + 7 {
            for x in hx - 7..=hx + 7 {
                if !plan::in_hall(x, z) || plan::piece(x, z).is_some() {
                    continue;
                }
                let seed = hash(x as f64 * 1.9, z as f64 * 2.7) as f32;
                let wet = plan::in_water(x as f64, z as f64);
                if !wet {
                    continue;
                }
                let y = if wet { SEABED_Y + 0.01 } else { 0.012 };
                let ph = seed * 7.3;
                let off = Vec3::new((time * 0.55 + ph).sin() * 0.35, 0.0, (time * 0.47 + ph * 1.7).cos() * 0.35);
                let alpha = (0.03 + 0.025 * (time * 1.3 + ph * 2.0).sin()) * if wet { 1.5 } else { 0.6 };
                let m = Mat4::from_translation(Vec3::new(x as f32, y, z as f32) + off)
                    * Mat4::from_rotation_y(ph + time * 0.25)
                    * Mat4::from_scale(Vec3::new(0.42, 0.004, 0.028));
                f.additive.push(Instance::new(m, lin(0xc8ffd0), alpha.max(0.0), Vec3::ZERO, SHADE_UNLIT));
            }
        }

        // ---- the lanterns burning green, and their light (the nearest; the altar's whatever)
        for (i, b) in bv.lanterns.iter().enumerate() {
            let fi = i as f32;
            for k in 0..4 {
                let t = (time * 2.0 + fi * 0.41 + k as f32 * 0.25).fract();
                let a = fi * 1.7 + k as f32 * 1.5;
                let p = *b + Vec3::new(a.cos() * 0.06 * (1.0 - t), 0.03 + t * 0.3, a.sin() * 0.06 * (1.0 - t));
                let size = 0.1 * (1.0 - t) + 0.025;
                f.additive.push(Instance::new(
                    Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(size, size * 1.7, size)),
                    lin(FLAME).lerp(lin(FLAME_TIP), t),
                    0.6 * (1.0 - t),
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
            glow_orb(f, *b + Vec3::new(0.0, 0.12, 0.0), 0.45, lin(LANTERN), 0.14 + 0.03 * (time * 6.0 + fi).sin());
        }
        for (i, p) in nearest(&bv.lanterns, hero, 4).into_iter().enumerate() {
            let flick = 3.6 + (time * 5.0 + i as f32 * 1.3).sin() * 0.4;
            f.lights.push(Light { pos: p + Vec3::new(0.0, 0.2, 0.0), color: lin(LANTERN) * flick, distance: 8.0 });
        }
        // The altar in its own green, and the cold of the deep from above on the bell.
        f.lights.push(Light { pos: Vec3::new(CENTRE.0 as f32, 2.6, ALTAR.3 as f32 + 1.6), color: lin(ENVY) * 3.2, distance: 7.0 });
        f.lights.push(Light { pos: c + Vec3::new(0.0, 6.5, 1.5), color: lin(0x9ad8cc) * 5.0, distance: 10.0 });

        // ---- the nazar cords: a cord down from the dark, its beads turning a little
        for (i, &(top, len)) in bv.cords.iter().enumerate() {
            let sway = (time * 0.7 + i as f32 * 1.3).sin() * 0.04;
            let foot = top + Vec3::new(sway, -len, sway * 0.5);
            let m = Mat4::from_scale_rotation_translation(
                Vec3::new(0.018, len, 0.018),
                Quat::from_rotation_arc(Vec3::Y, (top - foot).normalize_or_zero()),
                (top + foot) * 0.5,
            );
            f.opaque[cube_i].push(Instance::new(m, lin(CORD), 1.0, Vec3::ZERO, SHADE_LIT));
            for k in 0..3 {
                let at = foot + (top - foot) * (k as f32 * 0.16);
                self.envy_nazar(f, at, 0.8 - k as f32 * 0.12, time + i as f32);
            }
        }

        // ---- the bell, and Leviathan's own light and fight
        let levi = s.enemies.iter().filter_map(|e| e.duel.as_deref()).find(|d| d.alive() && d.breed.kind == EnemyKind::Leviathan);
        let hot = levi.is_some_and(|d| d.lore.bell.is_some());
        let swing = levi
            .and_then(|d| match d.f.state {
                State::Attack { mv: MoveId::BellToll, frame } => {
                    let data = MoveId::BellToll.data();
                    let since = frame as f32 - data.startup as f32;
                    Some(if since < 0.0 { 0.0 } else { (since * 0.25).sin() * 0.12 * (-since / 40.0).exp() })
                }
                _ => None,
            })
            .unwrap_or(0.0);
        let toward = levi.map(|d| Vec2::new(d.pos.x as f32 - c.x, d.pos.y as f32 - c.z).normalize_or_zero()).unwrap_or(Vec2::Y);
        // Dithered out of the way when it stands between the camera and him, or Leviathan.
        let eye = self.cam_pos;
        let mut sights = vec![hero + Vec3::new(0.0, 0.6, 0.0)];
        sights.extend(levi.map(|d| Vec3::new(d.pos.x as f32, 1.8, d.pos.y as f32)));
        let blocks = sights.iter().any(|&to| {
            (0..24).any(|k| {
                let p = eye.lerp(to, k as f32 / 24.0 * 0.95);
                p.y > BELL_MOUTH - 0.2
                    && p.y < BELL_MOUTH + BELL_HEIGHT + 0.6
                    && Vec2::new(p.x - c.x, p.z - c.z).length() < BELL_RIM as f32 + 0.2
            })
        });
        self.envy_bell(f, c, hot, swing, toward, time, if blocks { 0.35 } else { 1.0 });
        if hot {
            self.envy_coils(f, c, toward, time);
        }
        if let Some(d) = levi {
            let p = Vec3::new(d.pos.x as f32, 0.0, d.pos.y as f32);
            let ahead = Vec3::new(d.facing.x as f32, 0.0, d.facing.y as f32);
            if !d.untouchable() {
                // A cold key before him and above; the envious green as his rim.
                f.lights.push(Light { pos: p + ahead * 2.4 + Vec3::new(0.0, 3.6, 0.0), color: lin(0xd8f0e4) * 8.0, distance: 8.0 });
                f.lights.push(Light { pos: p - ahead * 1.6 + Vec3::new(0.0, 3.0, 0.0), color: lin(ENVY) * 5.0, distance: 6.0 });
            }
            self.envy_floor_marks(f, d, s, time);
        }

        // ---- the gate he came in by
        let door = s.gate.map(|g| g.door).unwrap_or(0.0);
        self.emit_portal(&bv.gate, door, false, time, hero, f);
    }

    /// A nazar bead: rings of dark blue, white and pale blue round a black eye, turned to the camera.
    fn envy_nazar(&self, f: &mut Frame, at: Vec3, s: f32, time: f32) {
        let disc = MeshKind::Disc as usize;
        let toward = f.facing * Vec3::Z;
        for (k, (r, col)) in [(0.12f32, NAZAR), (0.085, 0xf4f4ff), (0.06, NAZAR_L), (0.028, 0x050505)].iter().enumerate() {
            let m = Mat4::from_scale_rotation_translation(Vec3::new(r * s, r * s, 1.0), f.facing, at + toward * (k as f32 * 0.005));
            let col = lin(*col);
            f.opaque[disc].push(Instance::new(m, col, 1.0, col * 0.3, SHADE_LIT));
        }
        glow_orb(f, at, 0.22 * s, lin(NAZAR_L), 0.1 + 0.03 * (time * 3.0).sin());
    }

    /// The bell: staves of verdigris bronze flaring at the lip and rounding at the shoulder, its
    /// bands, four panels of bosses and the striking-seat, the dome and its serpent loop. It swings
    /// by `swing` (about the chain, toward `toward`); `hot`, its bronze glows red, and hottest on
    /// the side Leviathan coils against.
    #[allow(clippy::too_many_arguments)]
    fn envy_bell(&self, f: &mut Frame, c: Vec3, hot: bool, swing: f32, toward: Vec2, time: f32, fade: f32) {
        let cube_i = MeshKind::Cube as usize;
        let shade = if fade < 0.999 { SHADE_DITHER } else { SHADE_BRONZE };
        let pivot = Vec3::new(c.x, BELL_MOUTH + BELL_HEIGHT + 0.5, c.z);
        let axis = Vec3::new(-toward.y, 0.0, toward.x).normalize_or_zero();
        let tilt = if axis.length_squared() > 0.0 { Quat::from_axis_angle(axis, swing) } else { Quat::IDENTITY };
        let place = |p: Vec3| pivot + tilt * (p - pivot);
        let radius = BELL_RIM as f32 - 0.05;
        let (bands, staves) = (11, 40);
        let profile = |t: f32| {
            let mut r = radius * (1.0 - 0.08 * t);
            if t < 0.12 {
                r *= 1.0 + 0.06 * (1.0 - t / 0.12);
            }
            if t > 0.82 {
                let u = (t - 0.82) / 0.18;
                r *= (1.0 - u * u * 0.85).max(0.1).sqrt();
            }
            r
        };
        let heat_at = Vec2::new(toward.x, toward.y);
        let pulse = 0.85 + 0.15 * (time * 3.0).sin();
        for b in 0..bands {
            let t = (b as f32 + 0.5) / bands as f32;
            let r = profile(t);
            let y = BELL_MOUTH + t * BELL_HEIGHT;
            let raised = matches!(b, 1 | 5 | 6 | 10);
            for i in 0..staves {
                let a = i as f32 / staves as f32 * TAU;
                // Verdigris streaked down the bronze, thickest low on it and along the bands.
                let streak = 0.45 + 0.3 * (a * 3.0 + t * 1.5).sin() + 0.15 * (a * 11.0 + 1.3).sin();
                let patina = (streak * (1.0 - t * 0.5) + if raised { 0.15 } else { 0.0 }).clamp(0.0, 1.0);
                let grain = 0.94 + 0.08 * hash(i as f64 * 1.3, b as f64 * 2.1) as f32;
                let mut col = (lin(BRONZE) * 0.8).lerp(lin(VERDIGRIS) * 0.85, patina * 0.7) * grain * if raised { 0.85 } else { 1.0 };
                let mut glow = Vec3::ZERO;
                if hot {
                    // Hottest where he coils against it, and low on it.
                    let side = (Vec2::new(a.cos(), a.sin()).dot(heat_at) * 0.5 + 0.5).powf(3.0);
                    let k = (0.15 + 0.85 * side) * pulse * (1.0 - t * 0.6);
                    col = col.lerp(lin(0x6a2410), 0.45 * k);
                    glow = lin(HEAT) * 0.35 * k * fade;
                }
                let w = TAU * r / staves as f32 * 1.12;
                let thick = if raised { 0.28 } else { 0.2 };
                let at = place(Vec3::new(c.x + a.cos() * r, y, c.z + a.sin() * r));
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::new(w, BELL_HEIGHT / bands as f32 * 1.05, thick),
                    tilt * Quat::from_rotation_y(-(PI / 2.0 + a)),
                    at,
                );
                f.opaque[cube_i].push(Instance::new(m, col, fade, glow, shade));
                let quarter = (a / (PI / 2.0)).fract();
                if (0.62..0.8).contains(&t) && (0.2..0.8).contains(&quarter) && i % 2 == 0 {
                    let at = place(Vec3::new(c.x + a.cos() * (r + 0.12), y, c.z + a.sin() * (r + 0.12)));
                    let m = Mat4::from_scale_rotation_translation(Vec3::splat(0.15), tilt * Quat::from_rotation_y(-a), at);
                    f.opaque[cube_i].push(Instance::new(m, lin(BRONZE) * 1.1, fade, glow, shade));
                }
                if !(0.04..=0.96).contains(&quarter) {
                    let at = place(Vec3::new(c.x + a.cos() * (r + 0.11), y, c.z + a.sin() * (r + 0.11)));
                    let m = Mat4::from_scale_rotation_translation(
                        Vec3::new(0.18, BELL_HEIGHT / bands as f32, 0.1),
                        tilt * Quat::from_rotation_y(-(PI / 2.0 + a)),
                        at,
                    );
                    f.opaque[cube_i].push(Instance::new(m, lin(BRONZE_D), fade, glow, shade));
                }
            }
        }
        // The dome, and its loop: a serpent of bronze biting the chain.
        let top = BELL_MOUTH + BELL_HEIGHT;
        for k in 0..3 {
            let rr = radius * 0.6 * (1.0 - k as f32 * 0.3);
            let m = Mat4::from_scale_rotation_translation(
                Vec3::new(rr * 2.0, 0.16, rr * 2.0),
                tilt * Quat::from_rotation_y(k as f32 * 0.4),
                place(Vec3::new(c.x, top + 0.08 + k as f32 * 0.16, c.z)),
            );
            f.opaque[cube_i].push(Instance::new(m, lin(VERDIGRIS_D), fade, Vec3::ZERO, shade));
        }
        for k in 0..7 {
            let a = k as f32 / 6.0 * PI;
            let at = place(Vec3::new(c.x + a.cos() * 0.28, top + 0.45 + a.sin() * 0.28, c.z));
            let m = Mat4::from_scale_rotation_translation(Vec3::new(0.14, 0.14, 0.18), tilt, at);
            f.opaque[cube_i].push(Instance::new(m, lin(VERDIGRIS), fade, Vec3::ZERO, shade));
        }
        // Its mouth's shadow on the floor, and its own warm light; hot, a red glow under it.
        let m = Mat4::from_translation(Vec3::new(c.x, 0.01, c.z))
            * Mat4::from_rotation_x(-PI / 2.0)
            * Mat4::from_scale(Vec3::new(radius, radius, 1.0));
        f.transparent[MeshKind::Disc as usize].push(Instance::new(m, lin(0x020403), 0.55, Vec3::ZERO, SHADE_UNLIT));
        if hot {
            f.lights.push(Light {
                pos: c + Vec3::new(heat_at.x * 1.2, 1.4, heat_at.y * 1.2),
                color: lin(HEAT) * 5.5 * pulse,
                distance: 6.0,
            });
            glow_orb(f, c + Vec3::new(heat_at.x * 1.3, 1.6, heat_at.y * 1.3), 1.6, lin(HEAT), 0.12 * pulse);
            // Wisps of steam off the hot bronze.
            for k in 0..8 {
                let t = (time * 0.5 + k as f32 * 0.125).fract();
                let a = k as f32 * 0.8 + time * 0.2;
                let p = c + Vec3::new(a.cos() * radius * 0.9, BELL_MOUTH + 1.0 + t * 2.4, a.sin() * radius * 0.9);
                f.transparent[MeshKind::Sphere as usize].push(Instance::new(
                    Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.12 + t * 0.3)),
                    lin(0xc8d0cc),
                    0.18 * (1.0 - t),
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
        } else {
            f.lights.push(Light { pos: c + Vec3::new(0.0, 2.2, 2.4), color: lin(0xd8a860) * 2.0, distance: 5.0 });
        }
    }

    /// Coiled round the bell: two turns of his body about its shoulder and waist, from the side
    /// he lies against, scaled green-black with the pale belly in.
    fn envy_coils(&self, f: &mut Frame, c: Vec3, toward: Vec2, time: f32) {
        let cube_i = MeshKind::Cube as usize;
        let start = toward.y.atan2(toward.x);
        let radius = BELL_RIM as f32 + 0.28;
        let turns = 1.7 * TAU;
        let mut u = 0.0;
        let mut k = 0;
        while u < turns {
            let a = start + u;
            let y = BELL_MOUTH + 0.5 + (u / turns) * 1.5 + (time * 1.3 + u * 2.0).sin() * 0.03;
            let girth = 0.42 * (1.0 - u / turns * 0.45);
            let at = Vec3::new(c.x + a.cos() * (radius + girth * 0.5), y, c.z + a.sin() * (radius + girth * 0.5));
            let tangent = Quat::from_rotation_y(-a);
            let scale = if k % 3 == 0 { lin(0x386a58) } else { lin(0x24443a) };
            f.opaque[cube_i].push(Instance::new(
                Mat4::from_scale_rotation_translation(Vec3::new(girth, girth * 0.9, 0.26), tangent, at),
                scale,
                1.0,
                Vec3::ZERO,
                SHADE_WET,
            ));
            let belly = Vec3::new(c.x + a.cos() * (radius + 0.02), y - girth * 0.1, c.z + a.sin() * (radius + 0.02));
            f.opaque[cube_i].push(Instance::new(
                Mat4::from_scale_rotation_translation(Vec3::new(0.06, girth * 0.7, 0.24), tangent, belly),
                lin(0x8e9a76),
                1.0,
                Vec3::ZERO,
                SHADE_BONE,
            ));
            u += 0.11;
            k += 1;
        }
    }

    /**
     * What Leviathan's fight puts on the floor that is Envy's own: the water churning over him as
     * he dives; his burning coil's fires (a ring of embers swelling at each as they come due, then
     * flames); the coveted thunder's two other marks, rings that fill as the first does; the
     * green breath of the covet gathering on him; the toll's ring shaking out from the bell.
     */
    fn envy_floor_marks(&self, f: &mut Frame, d: &crate::logic::combat::Duelist, s: &SceneState, time: f32) {
        let State::Attack { mv, frame } = d.f.state else { return };
        let data = d.data(mv);
        let phase = data.phase(frame);
        let fill = if phase == Phase::Startup { frame as f32 / data.startup as f32 } else { 1.0 };
        let late = phase == Phase::Startup && data.startup - frame <= 8;
        let blink = if late { 0.6 + (time * 40.0).sin().abs() * 0.8 } else { 0.0 };
        let ring = |f: &mut Frame, at: Vec3, r: f32, color: Vec3, alpha: f32| {
            let n = ((TAU * r) / 0.22).ceil() as i32;
            for k in 0..n {
                let a = k as f32 * TAU / n as f32;
                let m =
                    Mat4::from_translation(at + Vec3::new(a.cos() * r, 0.03, a.sin() * r)) * Mat4::from_scale(Vec3::new(0.08, 0.012, 0.08));
                f.additive.push(Instance::new(m, color, alpha, Vec3::ZERO, SHADE_UNLIT));
            }
        };
        let floor = |p: crate::logic::types::V| Vec3::new(p.x as f32, floor_at(s.grid, p.x as f32, p.y as f32), p.y as f32);
        match mv {
            MoveId::Dive if phase == Phase::Startup => {
                // A wake of rings over him as he goes under, and the water heaving where he will rise.
                let at = floor(d.pos);
                for k in 0..3 {
                    let t = (time * 1.2 + k as f32 / 3.0).fract();
                    let r = 0.3 + t * 1.3;
                    let m = Mat4::from_translation(at + Vec3::new(0.0, 0.04, 0.0))
                        * Mat4::from_rotation_x(-PI / 2.0)
                        * Mat4::from_scale(Vec3::new(r, r, 1.0));
                    f.transparent[MeshKind::Annulus as usize].push(Instance::new(
                        m,
                        lin(0x9fe8c8),
                        0.5 * (1.0 - t),
                        Vec3::ZERO,
                        SHADE_UNLIT,
                    ));
                }
            }
            MoveId::BurningCoil => {
                for p in fire_spots(BELL, d.lore.mark) {
                    let at = floor(p);
                    if phase == Phase::Startup {
                        // Embers in the floor, a ring of fire growing in from its edge.
                        ring(f, at, FIRE_RADIUS as f32, lin(FIRE) * (1.0 + blink), 0.75);
                        let inner = FIRE_RADIUS as f32 * (1.0 - fill);
                        let m = Mat4::from_translation(at + Vec3::new(0.0, 0.025, 0.0))
                            * Mat4::from_rotation_x(-PI / 2.0)
                            * Mat4::from_scale(Vec3::new(FIRE_RADIUS as f32, FIRE_RADIUS as f32, 1.0));
                        f.transparent[MeshKind::Disc as usize].push(Instance::new(
                            m,
                            lin(HEAT) * (0.6 + blink * 0.6),
                            0.1 + 0.25 * fill,
                            Vec3::ZERO,
                            SHADE_UNLIT,
                        ));
                        let m = Mat4::from_translation(at + Vec3::new(0.0, 0.03, 0.0))
                            * Mat4::from_rotation_x(-PI / 2.0)
                            * Mat4::from_scale(Vec3::new(inner + 0.05, inner + 0.05, 1.0));
                        f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, lin(FIRE_TIP) * 1.2, 0.5, Vec3::ZERO, SHADE_UNLIT));
                    } else {
                        // The flames, standing for a while and dying down.
                        let since = (frame - data.startup) as f32;
                        let dying = (1.0 - since / FLAMES as f32).clamp(0.0, 1.0);
                        if dying <= 0.0 {
                            continue;
                        }
                        for k in 0..8 {
                            let a = k as f32 * 2.4 + p.x as f32 * 3.0;
                            let r = FIRE_RADIUS as f32 * (0.2 + 0.65 * ((k * 37 % 10) as f32 / 10.0));
                            let flick = ((time * (8.0 + k as f32)) + a).sin() * 0.5 + 0.5;
                            let tall = (0.4 + 0.6 * flick) * dying;
                            let q = at + Vec3::new(a.cos() * r, tall * 0.5, a.sin() * r);
                            let m = Mat4::from_translation(q) * Mat4::from_rotation_y(a) * Mat4::from_scale(Vec3::new(0.13, tall, 0.13));
                            let col = if k % 3 == 0 { lin(FIRE_TIP) } else { lin(HEAT) };
                            f.additive.push(Instance::new(m, col * 2.2, 0.85 * dying, Vec3::ZERO, SHADE_UNLIT));
                        }
                        f.lights.push(Light { pos: at + Vec3::new(0.0, 0.5, 0.0), color: lin(FIRE) * 3.5 * dying, distance: 3.0 });
                    }
                }
            }
            MoveId::CovetThunder if phase == Phase::Startup => {
                // The two marks behind him, filling with the first (the brawl draws the first).
                for p in thunder_marks(d.lore.mark, d.pos).into_iter().skip(1) {
                    let at = floor(p);
                    ring(f, at, 1.0, lin(ENVY) * (1.2 + blink), 0.8);
                    let r = 1.0 * fill;
                    let m = Mat4::from_translation(at + Vec3::new(0.0, 0.028, 0.0))
                        * Mat4::from_rotation_x(-PI / 2.0)
                        * Mat4::from_scale(Vec3::new(r.max(0.05), r.max(0.05), 1.0));
                    f.transparent[MeshKind::Disc as usize].push(Instance::new(
                        m,
                        lin(ENVY) * 0.8,
                        0.12 + 0.18 * fill,
                        Vec3::ZERO,
                        SHADE_UNLIT,
                    ));
                }
            }
            MoveId::BellToll => {
                if phase == Phase::Startup {
                    // The bell humming: a faint ring of green round it as the strike gathers.
                    let c = Vec3::new(BELL.x as f32, 0.0, BELL.y as f32);
                    let m = Mat4::from_translation(c + Vec3::new(0.0, 0.03, 0.0))
                        * Mat4::from_rotation_x(-PI / 2.0)
                        * Mat4::from_scale(Vec3::new(3.6, 3.6, 1.0));
                    f.transparent[MeshKind::Disc as usize].push(Instance::new(
                        m,
                        lin(0xd8a860) * (0.5 + blink),
                        0.05 + 0.12 * fill,
                        Vec3::ZERO,
                        SHADE_UNLIT,
                    ));
                } else {
                    // The toll shaking out from the bell over the floor.
                    let since = (frame - data.startup) as f32;
                    let c = Vec3::new(BELL.x as f32, 0.04, BELL.y as f32);
                    for k in 0..3 {
                        let t = (since / 40.0 - k as f32 * 0.15).clamp(0.0, 1.0);
                        if t <= 0.0 || t >= 1.0 {
                            continue;
                        }
                        let r = BELL_RIM as f32 + t * 4.0;
                        let m = Mat4::from_translation(c) * Mat4::from_rotation_x(-PI / 2.0) * Mat4::from_scale(Vec3::new(r, r, 1.0));
                        f.transparent[MeshKind::Annulus as usize].push(Instance::new(
                            m,
                            lin(0xffd8a0),
                            0.6 * (1.0 - t),
                            Vec3::ZERO,
                            SHADE_UNLIT,
                        ));
                    }
                }
            }
            _ => {}
        }
        if crate::logic::combat::prince::envy::coveted(mv).is_some() && phase == Phase::Startup {
            // The covet: green breath gathering round his head, his eyes on the hero's power.
            let at = Vec3::new(d.pos.x as f32, 2.6, d.pos.y as f32);
            glow_orb(f, at, 0.6 + fill * 1.2, lin(ENVY), 0.2 + 0.25 * fill);
            for k in 0..6 {
                let a = k as f32 * TAU / 6.0 + time * 2.0;
                let r = 1.4 * (1.0 - fill) + 0.3;
                let p = at + Vec3::new(a.cos() * r, (time * 3.0 + k as f32).sin() * 0.2, a.sin() * r);
                f.additive.push(Instance::new(
                    Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.06)),
                    lin(ENVY) * 2.0,
                    0.8,
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
            f.lights.push(Light { pos: at, color: lin(ENVY) * (2.0 + 4.0 * fill), distance: 6.0 });
        }
    }
}
