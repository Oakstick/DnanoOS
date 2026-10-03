//! The Hall of Mirrors as the view draws it (Pride's realm; laid out in `logic::realms::mirrors`):
//! the Spire of the Morning Star.
//!
//! - **The galleries:** floors of black-violet obsidian polished to mirrors, banded round the
//!   light-well and inlaid with gold spokes and peacock eyes (more gold on every floor up). The
//!   walls are white marble on violet plinths under gilt cornices, with gold pilasters and
//!   peacock-eye windows of stained glass.
//! - **The light-well:** a gold balustrade round it, and down it the galleries below, ring under
//!   ring, dimmer as they go. Up it falls the Morning Star's shaft of light, and rings of light
//!   hang in it, one for every gallery the well shows above him.
//! - **The mirror-doors:** gold frames under the Morning Star's sunburst, glass that shows his
//!   reflection when he comes near:
//!   - the vain show him gilded, crowned and whole;
//!   - the true show him as he is, darker as he is hurt, hairline cracks across the glass;
//!   - each shows his health, full in the vain, true in the true.
//! - **The statues:** a statue of him in triumph before every door, his glory's statues on their
//!   plinths, the relay mirrors by the well, the beams they throw (a flickering line, then a hard
//!   white-violet light).
//! - **The throne:** a round mirror floor whose rings fall into the well, the dais and the throne,
//!   the vast mirror, the Morning Star above, and the Peerless in his likeness, a step behind him.

mod decor;

use std::collections::VecDeque;

use super::super::vault_decor::{push_merged, Merged};
use super::super::*;
use crate::logic::combat::fsm::State as Fight;
use crate::logic::realms::mirrors::{on_octagon, Mirrors, CELL, MARGIN, OUTER, THRONE, THRONE_RADIUS, WELL, WELL_CUT};

/// How brightly his lantern burns here: the Morning Star is the light.
pub(super) const LANTERN: f32 = 0.05;

mod pal {
    pub const OBSIDIAN: u32 = 0x2a1e3e;
    pub const OBSIDIAN_L: u32 = 0x3a2a56;
    pub const OBSIDIAN_D: u32 = 0x120c1c;
    pub const RIM: u32 = 0x6a5a92;
    pub const MARBLE: u32 = 0xf2eef8;
    pub const MARBLE_D: u32 = 0xd2c8e0;
    pub const PLINTH: u32 = 0x3a2a58;
    pub const GOLD: u32 = 0xf2c53d;
    pub const GOLD_D: u32 = 0xb8902a;
    pub const GOLD_L: u32 = 0xfff0a0;
    pub const VIOLET: u32 = 0x9a5aff;
    pub const STAR: u32 = 0xf4f0ff;
    pub const GLASS: u32 = 0xc8c0f0;
    pub const TEAL: u32 = 0x1ab0a0;
    pub const SAPPHIRE: u32 = 0x2a4ad8;
    pub const CAP: u32 = 0x2a2040;
}
use pal::*;

/// Polished gold and polished marble or obsidian (see `voxel_surface` in the shader).
const SHADE_GOLD: f32 = 13.0;
const SHADE_MARBLE: f32 = 15.0;
/// The walls stand this tall; the glass of a mirror-door this tall.
const WALL_H: f32 = 2.0;
const GLASS_H: f32 = 1.8;
/// How far below each floor the next gallery down is drawn in the well, and how many are drawn.
const LAYER_DEPTH: f32 = 2.6;
const LAYERS: usize = 3;
/// Things are drawn only this near him (tiles).
const NEAR: f32 = 24.0;
/// Statues are drawn this near him (tiles).
const STATUE_RANGE: f32 = 12.0;
/// His reflection shows in a mirror he is this near.
const REFLECT_RANGE: f32 = 5.0;
/// The Peerless lags him by this many seconds (24 frames).
const ECHO_SECONDS: f32 = 0.4;
/// Seconds a fallen ring takes to drop out of sight.
const RING_FALL: f32 = 1.6;

/// What the view keeps of the realm: his recent poses (for the Peerless's echo), the statues' rig
/// and pose (him in triumph, coarse, in stone), the warps seen (the camera snaps on each), the
/// windows' glow and the shards of the chandeliers, and the art.
pub(in crate::render::view) struct MirrorsView {
    poses: VecDeque<(f32, Pose)>,
    warps: u32,
    windows: Vec<(Vec3, usize)>,
    chandeliers: Vec<(Vec3, usize)>,
    art: Box<Art>,
}

/// The realm's sculpted pieces, and the statues' rig and pose.
struct Art {
    throne: Merged,
    statue: Rig,
    triumph: Pose,
}

fn cube(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot_y: f32, color: Vec3, shade: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(rot_y), pos), color, 1.0, Vec3::ZERO, shade));
}

/// The gallery whose cell holds `p` (7: the empty cell).
fn cell_of(p: P) -> usize {
    let col = ((p.x - MARGIN) / CELL).clamp(0, 3);
    let row = ((p.y - MARGIN) / CELL).clamp(0, 1);
    (row * 4 + col) as usize
}

/// The ring of the octagon a tile lies on (5 at the well's rim, 11 against the wall).
fn ring(dx: i32, dy: i32) -> i32 {
    let diag = ((dx.abs() + dy.abs()) as f32 * OUTER as f32 / crate::logic::realms::mirrors::CUT as f32).ceil() as i32;
    dx.abs().max(dy.abs()).max(diag)
}

/// The turn that faces a model built facing +z along `(x, z)`.
fn yaw_of(x: f64, z: f64) -> f32 {
    (x as f32).atan2(z as f32)
}

impl WorldView {
    /// The realm's static geometry.
    pub(super) fn build_mirrors(&mut self, grid: &Grid, r: &Mirrors) -> MirrorsView {
        let sunburst = decor::sunburst();
        let peacock = decor::peacock_window();
        let stand = decor::relay_stand();
        let rock = |x: i32, y: i32| grid.get(x, y) == Tile::Rock;
        let open = |x: i32, y: i32| grid.in_bounds(x, y) && !rock(x, y);
        let floor = |x: i32, y: i32| grid.is_walkable(x, y, false);
        let plinths: Vec<P> = r.galleries.iter().flat_map(|g| g.plinths.iter().copied()).collect();
        let relays: Vec<(P, Dir)> = r.galleries.iter().flat_map(|g| g.relays.iter().copied()).collect();
        let statues: Vec<P> = r.doors.iter().filter_map(|d| d.statue).collect();
        // Behind every mirror's glass: the reflected room, three rows deep.
        let mut alcove: Vec<P> = Vec::new();
        for d in &r.doors {
            for t in &d.tiles {
                for k in 1..=3 {
                    alcove.push(P::new(t.x, t.y - k));
                }
            }
        }
        let gates = [r.base.entry.tile, r.base.exit.tile];
        let mut windows = Vec::new();
        let mut chandeliers = Vec::new();

        // ---- floors (the throne's is drawn each frame: its rings fall)
        for (x, y, t) in grid.cells() {
            if t == Tile::Rock || t == Tile::Pit {
                continue;
            }
            let p = P::new(x, y);
            let g = cell_of(p);
            if g >= THRONE {
                continue;
            }
            let o = r.galleries[g].center;
            let (dx, dy) = (x - o.x, y - o.y);
            let h = hash(x as f64 * 1.7 + 3.0, y as f64 * 2.3) as f32;
            self.gallery_floor(Vec3::new(x as f32, 0.0, y as f32), dx, dy, g, h, r.door_at(p).is_some());
        }
        // The thresholds behind the portals, walkable when their doors open.
        for gt in [&r.base.entry, &r.base.exit] {
            let (dx, dy) = gt.facing.vec();
            cube(
                &mut self.static_cubes,
                Vec3::new(gt.tile.x as f32 - dx as f32 * 0.5, -0.06, gt.tile.y as f32 - dy as f32 * 0.5),
                Vec3::new(1.0, 0.1, 1.0),
                0.0,
                lin(OBSIDIAN_D),
                SHADE_MARBLE,
            );
        }

        // ---- the reflected rooms behind the glass: the floor and its sides seen in the mirror
        for &p in &alcove {
            let (xf, yf) = (p.x as f32, p.y as f32);
            let h = hash(p.x as f64 * 2.9, p.y as f64 * 1.3) as f32;
            // The floor before the glass, mirrored: its rim first, then its bands.
            let c = if r.door_at(P::new(p.x, p.y + 1)).is_some()
                || alcove.contains(&P::new(p.x, p.y + 1)) && !alcove.contains(&P::new(p.x, p.y + 2))
            {
                RIM
            } else if (p.x + p.y) % 2 == 0 {
                OBSIDIAN
            } else {
                OBSIDIAN_L
            };
            cube(&mut self.static_cubes, Vec3::new(xf, -0.06, yf), Vec3::new(1.0, 0.12, 1.0), 0.0, lin(c) * (0.8 + 0.1 * h), SHADE_MARBLE);
            cube(&mut self.static_cubes, Vec3::new(xf, WALL_H - 0.03, yf), Vec3::new(1.0, 0.1, 1.0), 0.0, lin(CAP) * 0.9, SHADE_MARBLE);
            for side in [-1, 1] {
                if !alcove.contains(&P::new(p.x + side, p.y)) {
                    cube(
                        &mut self.static_cubes,
                        Vec3::new(xf + side as f32 * 0.52, WALL_H / 2.0, yf),
                        Vec3::new(0.06, WALL_H, 1.0),
                        0.0,
                        lin(MARBLE_D) * 0.6,
                        SHADE_MARBLE,
                    );
                }
            }
        }

        // ---- walls, plinths, relay stands, the throne's colonnade
        for (x, y, t) in grid.cells() {
            if t != Tile::Rock {
                continue;
            }
            let p = P::new(x, y);
            if gates.contains(&p) || alcove.contains(&p) {
                continue;
            }
            let faces: Vec<Dir> = CARDINALS.iter().copied().filter(|d| open(x + d.vec().0, y + d.vec().1)).collect();
            let near_floor = faces.len() + [(1, 1), (1, -1), (-1, 1), (-1, -1)].iter().filter(|(dx, dy)| open(x + dx, y + dy)).count();
            let g = cell_of(p);
            let base = Vec3::new(x as f32, 0.0, y as f32);
            if plinths.contains(&p) || statues.contains(&p) {
                self.plinth(base, statues.contains(&p));
                continue;
            }
            if let Some(&(_, d)) = relays.iter().find(|(q, _)| *q == p) {
                let (dx, dy) = d.vec();
                self.plinth_low(base);
                push_merged(
                    &mut self.static_cubes,
                    &stand,
                    Mat4::from_translation(base + Vec3::new(0.0, 0.16, 0.0)) * Mat4::from_rotation_y(yaw_of(dx as f64, dy as f64)),
                    0.04,
                );
                continue;
            }
            if g == THRONE {
                if near_floor > 0 {
                    let start = self.static_cubes.len();
                    self.colonnade(base, (x + y) % 2 == 0);
                    self.fade_with(start, base, WALL_H + 0.6);
                }
                continue;
            }
            if faces.is_empty() {
                if near_floor > 0 {
                    // A corner of the octagon: a gold column where the walls meet.
                    let start = self.static_cubes.len();
                    self.corner_column(base);
                    self.fade_with(start, base, WALL_H + 0.3);
                } else {
                    // Solid masonry between the galleries, capped in dark violet marble, a gold coffer
                    // line every fourth row and column.
                    let h = hash(x as f64 * 0.7, y as f64 * 1.9) as f32;
                    cube(
                        &mut self.static_cubes,
                        base + Vec3::new(0.0, WALL_H - 0.03, 0.0),
                        Vec3::new(1.0, 0.1, 1.0),
                        0.0,
                        lin(CAP) * (0.85 + 0.1 * h),
                        SHADE_MARBLE,
                    );
                    if y.rem_euclid(4) == 0 {
                        cube(
                            &mut self.static_cubes,
                            base + Vec3::new(0.0, WALL_H + 0.025, -0.5),
                            Vec3::new(1.0, 0.012, 0.03),
                            0.0,
                            lin(GOLD_D) * 0.25,
                            SHADE_LIT,
                        );
                    }
                    if x.rem_euclid(4) == 0 {
                        cube(
                            &mut self.static_cubes,
                            base + Vec3::new(-0.5, WALL_H + 0.025, 0.0),
                            Vec3::new(0.03, 0.012, 1.0),
                            0.0,
                            lin(GOLD_D) * 0.25,
                            SHADE_LIT,
                        );
                    }
                }
                continue;
            }
            let start = self.static_cubes.len();
            let window = self.wall_block(base, &faces, g, hash(x as f64 * 3.0, y as f64 * 5.0) as f32, &peacock, r, p);
            if let Some(w) = window {
                windows.push((w, g));
            }
            self.fade_with(start, base, WALL_H + 0.4);
        }

        // ---- the mirror-doors: gold frames under the sunburst, a step of gold at the threshold
        for d in &r.doors {
            self.door_frame(
                d.tiles.first().copied().unwrap_or(P::new(0, 0)),
                d.tiles.last().copied().unwrap_or(P::new(0, 0)),
                d.gallery == THRONE,
                &sunburst,
            );
        }

        // ---- the light-wells: a balustrade round each, its shaft, and the galleries below
        for gal in &r.galleries[..THRONE] {
            let o = gal.center;
            for dy in -WELL..=WELL {
                for dx in -WELL..=WELL {
                    if !on_octagon(dx, dy, WELL, WELL_CUT) {
                        continue;
                    }
                    let p = P::new(o.x + dx, o.y + dy);
                    for d in CARDINALS {
                        let (sx, sy) = d.vec();
                        if floor(p.x + sx, p.y + sy) {
                            self.well_edge(p, d, gal.index);
                        }
                    }
                }
            }
            self.galleries_below(o, gal.index, WELL);
            for (sx, sy) in [(3, 3), (-3, 3), (3, -3), (-3, -3)] {
                chandeliers.push((Vec3::new((o.x + sx) as f32, 2.5, (o.y + sy) as f32), gal.index));
            }
        }
        // Under the throne, the whole spire below.
        self.galleries_below(r.throne, THRONE, THRONE_RADIUS as i32 - 2);

        // The statues' rig: him in triumph, his blade raised to the Morning Star, in coarse stone.
        let statue = self.models.hero.coarse();
        let mut triumph = Pose::rest(&statue);
        triumph.rot_x(&statue, "swordArm", -2.9);
        triumph.rot_x(&statue, "blade", -2.9);
        triumph.rot_x(&statue, "shieldArm", -0.5);
        triumph.rot_x(&statue, "offhand", -0.5);
        triumph.rot_x(&statue, "shield", -0.5);
        triumph.rot_x(&statue, "head", -0.25);
        let art = Box::new(Art { throne: decor::throne(), statue, triumph });
        MirrorsView { poses: VecDeque::new(), warps: r.warps, windows, chandeliers, art }
    }

    /// Walls and columns dissolve when they stand between the camera and him.
    fn fade_with(&mut self, start: usize, base: Vec3, height: f32) {
        let len = self.static_cubes.len() - start;
        let shades: Vec<f32> = self.static_cubes[start..start + len].iter().map(|i| i.emissive[3]).collect();
        self.trees.push(Tree { range: (start, len), base, radius: 0.72, height, fade: 1.0, alive: true, shades: Some(shades) });
    }

    /// A gallery's floor tile: obsidian banded round the well, a rim of pale marble at the well and
    /// the walls, gold spokes, a ring of peacock eyes; more gold on every floor up.
    fn gallery_floor(&mut self, at: Vec3, dx: i32, dy: i32, g: usize, h: f32, door: bool) {
        let out = &mut self.static_cubes;
        let rr = ring(dx, dy);
        if door {
            cube(out, at + Vec3::new(0.0, -0.06, 0.0), Vec3::new(1.0, 0.12, 1.0), 0.0, lin(OBSIDIAN_D), SHADE_MARBLE);
            cube(out, at + Vec3::new(0.0, 0.004, 0.42), Vec3::new(1.0, 0.012, 0.08), 0.0, lin(GOLD), SHADE_GOLD);
            return;
        }
        let rim = rr == WELL + 1 || rr == OUTER;
        let color = if rim {
            lin(RIM) * (0.9 + 0.15 * h)
        } else if rr % 2 == 0 {
            lin(OBSIDIAN) * (0.92 + 0.12 * h)
        } else {
            lin(OBSIDIAN_L) * (0.92 + 0.12 * h)
        };
        cube(out, at + Vec3::new(0.0, -0.06, 0.0), Vec3::new(0.98, 0.12, 0.98), 0.0, color, SHADE_MARBLE);
        let gold = lin(if g >= 3 { GOLD_L } else { GOLD });
        // The spokes: north, south, east, west and the diagonals, a line of gold along each.
        let spoke = if dx == 0 {
            Some(0.0)
        } else if dy == 0 {
            Some(PI / 2.0)
        } else if dx.abs() == dy.abs() {
            Some(if dx.signum() == dy.signum() { -PI / 4.0 } else { PI / 4.0 })
        } else {
            None
        };
        if let (Some(a), false) = (spoke, rim) {
            let len = if a.abs() > 0.1 && a.abs() < 1.0 { 1.38 } else { 1.0 };
            cube(out, at + Vec3::new(0.0, 0.003, 0.0), Vec3::new(0.06 + 0.02 * g as f32, 0.012, len), a, gold, SHADE_GOLD);
        }
        // Rings of gold: one at the middle of the floor, more on the higher floors.
        let rings: &[i32] = match g {
            0 | 1 => &[8],
            2 | 3 => &[7, 9],
            _ => &[7, 8, 9],
        };
        if rings.contains(&rr) && !rim {
            for (sx, sy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                if ring(dx + sx, dy + sy) == rr - 1 {
                    let pos = at + Vec3::new(sx as f32 * 0.46, 0.004, sy as f32 * 0.46);
                    let sc = if sx != 0 { Vec3::new(0.06, 0.012, 1.0) } else { Vec3::new(1.0, 0.012, 0.06) };
                    cube(out, pos, sc, 0.0, gold, SHADE_GOLD);
                }
            }
        }
        // A peacock's eye where a spoke crosses the middle ring.
        if spoke.is_some() && rr == 8 {
            let c = at + Vec3::new(0.0, 0.006, 0.0);
            for (k, (col, s)) in [(GOLD, 0.62f32), (TEAL, 0.48), (SAPPHIRE, 0.32), (0x1a0a3a, 0.16)].iter().enumerate() {
                let glow = if k == 1 || k == 2 { 0.25 } else { 0.0 };
                out.push(Instance::new(
                    Mat4::from_scale_rotation_translation(
                        Vec3::new(*s, 0.012, *s * 1.35),
                        Quat::from_rotation_y(PI / 4.0),
                        c + Vec3::new(0.0, k as f32 * 0.002, 0.0),
                    ),
                    lin(*col),
                    1.0,
                    lin(*col) * glow,
                    if k == 0 { SHADE_GOLD } else { SHADE_MARBLE },
                ));
            }
        }
    }

    /// A wall block: white marble on a violet plinth under a gilt cornice, a gold pilaster every
    /// third block, and on some a peacock-eye window lit from behind. Returns the window's glow.
    #[allow(clippy::too_many_arguments)]
    fn wall_block(&mut self, base: Vec3, faces: &[Dir], g: usize, h: f32, peacock: &Merged, r: &Mirrors, p: P) -> Option<Vec3> {
        let out = &mut self.static_cubes;
        let marble = lin(MARBLE) * (0.94 + 0.08 * h);
        cube(out, base + Vec3::new(0.0, 0.12, 0.0), Vec3::new(1.0, 0.24, 1.0), 0.0, lin(PLINTH), SHADE_MARBLE);
        cube(out, base + Vec3::new(0.0, WALL_H / 2.0 + 0.06, 0.0), Vec3::new(1.0, WALL_H - 0.36, 1.0), 0.0, marble, SHADE_MARBLE);
        cube(out, base + Vec3::new(0.0, WALL_H - 0.12, 0.0), Vec3::new(1.02, 0.08, 1.02), 0.0, lin(GOLD_D), SHADE_GOLD);
        cube(out, base + Vec3::new(0.0, WALL_H, 0.0), Vec3::new(1.1, 0.14, 1.1), 0.0, lin(MARBLE_D), SHADE_MARBLE);
        cube(out, base + Vec3::new(0.0, 0.28, 0.0), Vec3::new(1.004, 0.03, 1.004), 0.0, lin(GOLD), SHADE_GOLD);
        let mut window = None;
        for &d in faces {
            let (dx, dy) = d.vec();
            let along = if dx == 0 { p.x } else { p.y };
            let n = Vec3::new(dx as f32, 0.0, dy as f32);
            let face = base + n * 0.5;
            // (No windows beside the mirror-doors, nor facing away from the camera above.)
            let by_door = r.doors.iter().any(|dr| dr.tiles.iter().any(|t| (t.x - p.x).abs() <= 1 && (t.y - p.y).abs() <= 1));
            if along.rem_euclid(6) == 2 && d != Dir::Up && !by_door && window.is_none() {
                let root = Mat4::from_translation(face + Vec3::new(0.0, 0.42, 0.0)) * Mat4::from_rotation_y(yaw_of(dx as f64, dy as f64));
                push_merged(out, peacock, root, 0.55);
                window = Some(face + n * 0.3 + Vec3::new(0.0, 1.05, 0.0));
            } else if along.rem_euclid(3) == 0 && !by_door {
                let flat = if dx != 0 { Vec3::new(0.1, 1.0, 0.3) } else { Vec3::new(0.3, 1.0, 0.1) };
                let at = base + n * 0.54;
                cube(out, at + Vec3::new(0.0, 0.2, 0.0), Vec3::new(flat.x * 1.4, 0.16, flat.z * 1.4), 0.0, lin(PLINTH), SHADE_MARBLE);
                let gold = lin(if g >= 2 { GOLD } else { GOLD_D });
                cube(out, at + Vec3::new(0.0, WALL_H / 2.0, 0.0), Vec3::new(flat.x, WALL_H - 0.6, flat.z), 0.0, gold, SHADE_GOLD);
                cube(
                    out,
                    at + Vec3::new(0.0, WALL_H - 0.3, 0.0),
                    Vec3::new(flat.x * 1.5, 0.12, flat.z * 1.5),
                    0.0,
                    lin(GOLD_L),
                    SHADE_GOLD,
                );
            }
        }
        window
    }

    /// Where two walls of the octagon meet: a fluted gold column.
    fn corner_column(&mut self, base: Vec3) {
        let out = &mut self.static_cubes;
        cube(out, base + Vec3::new(0.0, 0.14, 0.0), Vec3::new(0.86, 0.28, 0.86), 0.0, lin(PLINTH), SHADE_MARBLE);
        for k in 0..8 {
            let y = 0.32 + k as f32 * 0.2;
            cube(out, base + Vec3::new(0.0, y + 0.1, 0.0), Vec3::new(0.5, 0.2, 0.5), k as f32 * PI / 8.0, lin(MARBLE) * 0.96, SHADE_MARBLE);
        }
        cube(out, base + Vec3::new(0.0, 0.31, 0.0), Vec3::new(0.62, 0.06, 0.62), PI / 4.0, lin(GOLD), SHADE_GOLD);
        cube(out, base + Vec3::new(0.0, WALL_H - 0.1, 0.0), Vec3::new(0.72, 0.14, 0.72), PI / 4.0, lin(GOLD_L), SHADE_GOLD);
        cube(out, base + Vec3::new(0.0, WALL_H, 0.0), Vec3::new(0.9, 0.12, 0.9), 0.0, lin(MARBLE_D), SHADE_MARBLE);
    }

    /// A plinth on the floor: a tall one for a statue of him (one before a door stands lower).
    fn plinth(&mut self, base: Vec3, low: bool) {
        let h = if low { 0.22 } else { 0.42 };
        let out = &mut self.static_cubes;
        cube(out, base + Vec3::new(0.0, 0.04, 0.0), Vec3::new(0.86, 0.08, 0.86), 0.0, lin(PLINTH), SHADE_MARBLE);
        cube(out, base + Vec3::new(0.0, h / 2.0 + 0.04, 0.0), Vec3::new(0.68, h, 0.68), 0.0, lin(MARBLE), SHADE_MARBLE);
        cube(out, base + Vec3::new(0.0, h + 0.05, 0.0), Vec3::new(0.76, 0.04, 0.76), 0.0, lin(GOLD), SHADE_GOLD);
    }

    fn plinth_low(&mut self, base: Vec3) {
        let out = &mut self.static_cubes;
        cube(out, base + Vec3::new(0.0, 0.06, 0.0), Vec3::new(0.8, 0.12, 0.8), PI / 4.0, lin(PLINTH), SHADE_MARBLE);
        cube(out, base + Vec3::new(0.0, 0.14, 0.0), Vec3::new(0.6, 0.04, 0.6), PI / 4.0, lin(GOLD_D), SHADE_GOLD);
    }

    /// The throne's colonnade: white columns and a balustrade between, open to the sky.
    fn colonnade(&mut self, base: Vec3, column: bool) {
        let out = &mut self.static_cubes;
        cube(out, base + Vec3::new(0.0, 0.08, 0.0), Vec3::new(1.0, 0.16, 1.0), 0.0, lin(PLINTH), SHADE_MARBLE);
        if column {
            for k in 0..9 {
                let y = 0.2 + k as f32 * 0.24;
                cube(
                    out,
                    base + Vec3::new(0.0, y + 0.12, 0.0),
                    Vec3::new(0.44, 0.24, 0.44),
                    k as f32 * PI / 9.0,
                    lin(MARBLE),
                    SHADE_MARBLE,
                );
            }
            cube(out, base + Vec3::new(0.0, 0.2, 0.0), Vec3::new(0.62, 0.08, 0.62), PI / 4.0, lin(GOLD), SHADE_GOLD);
            cube(out, base + Vec3::new(0.0, 2.42, 0.0), Vec3::new(0.66, 0.1, 0.66), PI / 4.0, lin(GOLD_L), SHADE_GOLD);
            cube(out, base + Vec3::new(0.0, 2.52, 0.0), Vec3::new(0.86, 0.1, 0.86), 0.0, lin(MARBLE_D), SHADE_MARBLE);
        } else {
            for k in 0..3 {
                let x = -0.3 + k as f32 * 0.3;
                cube(out, base + Vec3::new(x, 0.42, 0.0), Vec3::new(0.12, 0.5, 0.12), PI / 4.0, lin(MARBLE_D), SHADE_MARBLE);
            }
            cube(out, base + Vec3::new(0.0, 0.72, 0.0), Vec3::new(1.0, 0.1, 0.34), 0.0, lin(MARBLE), SHADE_MARBLE);
            cube(out, base + Vec3::new(0.0, 0.78, 0.0), Vec3::new(1.0, 0.03, 0.2), 0.0, lin(GOLD), SHADE_GOLD);
        }
    }

    /// A mirror-door's frame from its west tile to its east: gold posts and lintel, the Morning
    /// Star's sunburst over it (the throne's vast mirror has three).
    fn door_frame(&mut self, west: P, east: P, vast: bool, sunburst: &Merged) {
        let out = &mut self.static_cubes;
        let z = west.y as f32 - 0.44;
        let (x0, x1) = (west.x as f32 - 0.5, east.x as f32 + 0.5);
        let mid = (x0 + x1) / 2.0;
        let top = if vast { GLASS_H + 0.9 } else { GLASS_H };
        for x in [x0, x1] {
            cube(out, Vec3::new(x, top / 2.0 + 0.05, z + 0.06), Vec3::new(0.16, top + 0.1, 0.16), 0.0, lin(GOLD), SHADE_GOLD);
            cube(out, Vec3::new(x, 0.1, z + 0.06), Vec3::new(0.26, 0.2, 0.26), 0.0, lin(GOLD_D), SHADE_GOLD);
        }
        cube(out, Vec3::new(mid, top + 0.1, z + 0.06), Vec3::new(x1 - x0 + 0.3, 0.16, 0.2), 0.0, lin(GOLD), SHADE_GOLD);
        cube(out, Vec3::new(mid, top + 0.2, z + 0.06), Vec3::new(x1 - x0 + 0.1, 0.06, 0.14), 0.0, lin(GOLD_L), SHADE_GOLD);
        let suns: &[f32] = if vast { &[-4.0, 0.0, 4.0] } else { &[0.0] };
        for &sx in suns {
            let m = Mat4::from_translation(Vec3::new(mid + sx, top + 0.18, z + 0.12))
                * Mat4::from_scale(Vec3::splat(if vast { 1.4 } else { 0.9 }));
            push_merged(out, sunburst, m, 0.08);
        }
    }

    /// The edge of the light-well on the side `d` of pit tile `p`: a gold balustrade on the floor's
    /// lip, and the shaft's marble falling to the gallery below.
    fn well_edge(&mut self, p: P, d: Dir, _g: usize) {
        let (sx, sy) = d.vec();
        let out = &mut self.static_cubes;
        let edge = Vec3::new(p.x as f32 + sx as f32 * 0.5, 0.0, p.y as f32 + sy as f32 * 0.5);
        let along = if sx != 0 { Vec3::new(0.0, 0.0, 1.0) } else { Vec3::new(1.0, 0.0, 0.0) };
        let span = |w: f32, h: f32, t: f32| if sx != 0 { Vec3::new(t, h, w) } else { Vec3::new(w, h, t) };
        // The shaft's wall, down to the floor below.
        cube(out, edge + Vec3::new(0.0, -LAYER_DEPTH / 2.0, 0.0), span(1.0, LAYER_DEPTH, 0.12), 0.0, lin(MARBLE_D) * 0.55, SHADE_MARBLE);
        cube(out, edge + Vec3::new(0.0, -0.14, 0.0), span(1.0, 0.1, 0.16), 0.0, lin(GOLD_D), SHADE_GOLD);
        // The balustrade: a rail on balusters.
        let lip = edge + Vec3::new(sx as f32 * 0.1, 0.0, sy as f32 * 0.1);
        for k in [-0.33f32, 0.0, 0.33] {
            cube(out, lip + along * k + Vec3::new(0.0, 0.22, 0.0), Vec3::new(0.07, 0.36, 0.07), PI / 4.0, lin(GOLD), SHADE_GOLD);
        }
        cube(out, lip + Vec3::new(0.0, 0.43, 0.0), span(1.02, 0.06, 0.12), 0.0, lin(GOLD_L), SHADE_GOLD);
        cube(out, lip + Vec3::new(0.0, 0.05, 0.0), span(1.02, 0.06, 0.14), 0.0, lin(GOLD_D), SHADE_GOLD);
    }

    /// The galleries below, seen down the well under (o): rings of their floors, each narrower and
    /// dimmer, with their rails and shafts; under the first, the dark of the spire's foot.
    fn galleries_below(&mut self, o: P, g: usize, opening: i32) {
        let out = &mut self.static_cubes;
        let layers = if g == THRONE { LAYERS + 1 } else { g.min(LAYERS) };
        let depth = if g == THRONE { LAYER_DEPTH * 2.2 } else { LAYER_DEPTH };
        for k in 1..=layers {
            let y = -(k as f32) * depth;
            let dim = if g == THRONE { 0.4 } else { 0.75 } / (1.0 + k as f32 * 0.6);
            let inner = (opening - k as i32).max(2);
            // (Under the throne the spire widens as it goes down, ring under ring.)
            let outer = if g == THRONE { opening + 4 + 2 * k as i32 } else { opening + 3 };
            for dy in -outer..=outer {
                for dx in -outer..=outer {
                    let (adx, ady) = (dx.abs(), dy.abs());
                    let cut = |r: i32| adx <= r && ady <= r && adx + ady <= r + r / 2;
                    if !cut(outer) || cut(inner) {
                        continue;
                    }
                    let at = Vec3::new((o.x + dx) as f32, y, (o.y + dy) as f32);
                    let rr = adx.max(ady);
                    let c = if rr == inner + 1 {
                        lin(RIM)
                    } else if (dx + dy) % 2 == 0 {
                        lin(OBSIDIAN)
                    } else {
                        lin(OBSIDIAN_L)
                    };
                    out.push(Instance::new(
                        Mat4::from_scale_rotation_translation(Vec3::new(1.0, 0.12, 1.0), Quat::IDENTITY, at),
                        c * dim * 2.0,
                        1.0,
                        lin(VIOLET) * 0.04 * dim,
                        SHADE_MARBLE,
                    ));
                    if rr == inner + 1 {
                        out.push(Instance::new(
                            Mat4::from_scale_rotation_translation(Vec3::new(1.0, 0.4, 1.0), Quat::IDENTITY, at + Vec3::new(0.0, 0.26, 0.0)),
                            lin(GOLD_D) * dim,
                            1.0,
                            lin(GOLD) * 0.12 * dim,
                            SHADE_GOLD,
                        ));
                    }
                }
            }
        }
        if g == 0 {
            // The spire's foot: a floor of black glass far down, the court's embers under it.
            for dy in -opening..=opening {
                for dx in -opening..=opening {
                    let at = Vec3::new((o.x + dx) as f32, -LAYER_DEPTH * 1.6, (o.y + dy) as f32);
                    let ember = hash((o.x + dx) as f64 * 3.1, (o.y + dy) as f64 * 1.7) as f32;
                    out.push(Instance::new(
                        Mat4::from_scale_rotation_translation(Vec3::new(1.0, 0.1, 1.0), Quat::IDENTITY, at),
                        lin(OBSIDIAN_D),
                        1.0,
                        lin(0x6a1a5a) * (0.08 + 0.2 * ember * ember),
                        SHADE_MARBLE,
                    ));
                }
            }
        }
    }

    /// The realm's own animation, one step: his poses kept for the echo, and the camera snapped
    /// to him when a mirror carries him off (the warp must not show).
    pub(super) fn update_mirrors_view(&mut self, v: &mut MirrorsView, r: &Mirrors, s: &SceneState, _dt: f32) {
        let now = s.time as f32;
        v.poses.push_back((now, self.hero_pose.clone()));
        while v.poses.front().is_some_and(|(t, _)| now - *t > ECHO_SECONDS * 2.5) {
            v.poses.pop_front();
        }
        if r.warps != v.warps {
            v.warps = r.warps;
            self.initialized = false;
        }
    }

    /// His pose as it was `ECHO_SECONDS` ago.
    fn echo_pose<'a>(&'a self, v: &'a MirrorsView, now: f32) -> Option<&'a Pose> {
        v.poses.iter().rev().find(|(t, _)| *t <= now - ECHO_SECONDS).or(v.poses.front()).map(|(_, p)| p)
    }

    /// What moves, glows and lights in the realm.
    pub(super) fn emit_mirrors(&self, v: &MirrorsView, r: &Mirrors, s: &SceneState, time: f32, hero: Vec3, f: &mut Frame) {
        let near = |p: Vec3| Vec2::new(p.x - hero.x, p.z - hero.z).length_squared() < NEAR * NEAR;
        let hp = s.hero.pos();
        let health = (s.hero.hp.max(0) as f32 / s.hero.max_hp.max(1) as f32).clamp(0.0, 1.0);
        let mut lights: Vec<(Vec3, Vec3, f32)> = Vec::new();
        for gal in &r.galleries[..THRONE] {
            let o = Vec3::new(gal.center.x as f32, 0.0, gal.center.y as f32);
            if !near(o) {
                continue;
            }
            let here = gal.index == r.gallery;
            self.emit_well(r, gal.index, o, here, time, f, &mut lights);
            for &d in &gal.doors {
                self.emit_door(v, r, d, hp, hero, health, time, f, &mut lights);
                let door = &r.doors[d];
                let seen = |p: P| Vec2::new(p.x as f32 - hero.x, p.y as f32 - hero.z).length() < STATUE_RANGE;
                if let (Some(st), true) = (door.statue, door.statue_hp > 0) {
                    if seen(st) {
                        self.emit_statue(v, Vec3::new(st.x as f32, 0.27, st.y as f32), 0.0, door.statue_hp, false, f);
                    }
                }
            }
            // Statues of him line the gallery, more on every floor up; those of his glory are gilded,
            // and the soft light falls on them.
            let standing = (2 + gal.index + r.glory as usize).min(gal.plinths.len());
            for (k, pl) in gal.plinths.iter().enumerate().take(standing) {
                let at = Vec3::new(pl.x as f32, 0.47, pl.y as f32);
                let face = Vec3::new(o.x - at.x, 0.0, o.z - at.z);
                let glory = k < r.glory as usize;
                if Vec2::new(at.x - hero.x, at.z - hero.z).length() > STATUE_RANGE {
                    continue;
                }
                self.emit_statue(v, at, face.x.atan2(face.z), 3, glory, f);
                glow_orb(
                    f,
                    at + Vec3::new(0.0, 0.8, 0.0),
                    0.7,
                    lin(if glory { GOLD_L } else { STAR }),
                    0.06 + 0.02 * (time * 2.0 + k as f32).sin(),
                );
                if here && glory && k < 2 {
                    lights.push((at + Vec3::new(0.0, 1.4, 0.4), lin(0xfff0d0) * 2.5, 3.5));
                }
            }
            // The chandeliers of mirror shards over the well, glinting.
            for (c, g) in &v.chandeliers {
                if *g == gal.index {
                    self.emit_chandelier(*c, time, f);
                }
            }
            if here {
                self.emit_beam(r, time, f, &mut lights);
            }
        }
        // The peacock windows glow, the nearest lighting the floor before them.
        let mut panes: Vec<Vec3> = Vec::new();
        for (w, g) in &v.windows {
            if *g == r.gallery && near(*w) {
                glow_orb(f, *w, 0.55, lin(TEAL), 0.07);
                panes.push(*w);
            }
        }
        for w in nearest(&panes, hero, 3) {
            lights.push((w, lin(0x3ad0c0) * 1.6, 3.5));
        }
        // Glory is light: an aura about him, brighter with every stack.
        if r.glory > 0 && s.hero.alive {
            let k = r.glory.min(6) as f32;
            glow_orb(f, hero + Vec3::new(0.0, 0.6, 0.0), 0.7 + 0.08 * k, lin(GOLD_L), 0.05 + 0.03 * k + 0.01 * (time * 4.0).sin());
            lights.push((hero + Vec3::new(0.0, 1.5, 0.3), lin(GOLD_L) * (0.8 + 0.4 * k), 3.0));
        }
        if r.gallery == THRONE || near(Vec3::new(r.throne.x as f32, 0.0, r.throne.y as f32)) {
            self.emit_throne(v, r, s, hp, hero, health, time, f, &mut lights);
        }
        lights.sort_by(|a, b| {
            let da = (a.0 - hero).length_squared();
            let db = (b.0 - hero).length_squared();
            da.total_cmp(&db)
        });
        for (pos, color, distance) in lights.into_iter().take(6) {
            f.lights.push(Light { pos, color, distance });
        }
    }

    /// A light-well: the Morning Star's shaft falling through it, and the rings of light that hang
    /// in it, one for every gallery it shows above him.
    #[allow(clippy::too_many_arguments)]
    fn emit_well(&self, r: &Mirrors, g: usize, o: Vec3, here: bool, time: f32, f: &mut Frame, lights: &mut Vec<(Vec3, Vec3, f32)>) {
        let star = lin(STAR);
        let bloom = 1.0 + 0.06 * r.loops as f32;
        // The shaft: a column of white light, brighter at its heart.
        for (w, a) in [(2.4f32 * bloom, 0.05f32), (1.2 * bloom, 0.09), (0.45, 0.16)] {
            let m = Mat4::from_translation(o + Vec3::new(0.0, 2.0, 0.0)) * Mat4::from_scale(Vec3::new(w, 16.0, w));
            f.additive.push(Instance::new(m, star, a * (0.9 + 0.1 * (time * 1.3).sin()), Vec3::ZERO, SHADE_UNLIT));
        }
        if !here {
            return;
        }
        // The rings above: as many as the well shows, stacked up the shaft.
        let shown = r.shown_above().min(14);
        for k in 0..shown {
            let y = 0.9 + k as f32 * 0.38;
            let rad = (1.5 - k as f32 * 0.05).max(0.8) * bloom.min(1.4);
            let pulse = 0.8 + 0.2 * (time * 2.0 - k as f32 * 0.6).sin();
            let m = Mat4::from_translation(o + Vec3::new(0.0, y, 0.0))
                * Mat4::from_rotation_x(-PI / 2.0)
                * Mat4::from_scale(Vec3::new(rad, rad, 1.0));
            let c = if k as usize >= THRONE - g { lin(GOLD_L) } else { star };
            f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, c * 2.0, 0.45 * pulse, Vec3::ZERO, SHADE_UNLIT));
        }
        glow_orb(f, o + Vec3::new(0.0, 0.6, 0.0), 2.6 * bloom, star, 0.08);
        lights.push((o + Vec3::new(0.0, 2.6, 0.0), star * 5.0 * bloom, 9.0));
    }

    /// A mirror-door's glass: its face (vain or true), his reflection when he is near, its cracks.
    #[allow(clippy::too_many_arguments)]
    fn emit_door(
        &self,
        v: &MirrorsView,
        r: &Mirrors,
        d: usize,
        hp: V,
        hero: Vec3,
        health: f32,
        time: f32,
        f: &mut Frame,
        lights: &mut Vec<(Vec3, Vec3, f32)>,
    ) {
        let door = &r.doors[d];
        let (west, east) = (door.tiles[0], door.tiles[door.tiles.len() - 1]);
        let glass_z = door.glass() as f32;
        let (x0, x1) = (west.x as f32 - 0.45, east.x as f32 + 0.45);
        let mid = (x0 + x1) / 2.0;
        let vast = door.gallery == THRONE;
        let top = if vast { GLASS_H + 0.9 } else { GLASS_H };
        let true_face = r.shows_true(d);
        let tint = if true_face { lin(GLASS) } else { lin(GOLD_L).lerp(lin(GLASS), 0.4) };
        // The glass: silvered, a sheen sliding across it now and then.
        let m = Mat4::from_translation(Vec3::new(mid, top / 2.0 + 0.04, glass_z + 0.02)) * Mat4::from_scale(Vec3::new(x1 - x0, top, 0.03));
        f.transparent[MeshKind::Cube as usize].push(Instance::new(m, tint, if true_face { 0.26 } else { 0.34 }, tint * 0.12, SHADE_LIT));
        let sweep = ((time * 0.35 + d as f32 * 0.37).fract() * 3.0 - 1.0) * (x1 - x0);
        if sweep.abs() < (x1 - x0) / 2.0 {
            let m = Mat4::from_translation(Vec3::new(mid + sweep, top / 2.0, glass_z + 0.05))
                * Mat4::from_rotation_z(0.5)
                * Mat4::from_scale(Vec3::new(0.12, top * 1.1, 0.01));
            f.additive.push(Instance::new(m, lin(STAR), 0.12, Vec3::ZERO, SHADE_UNLIT));
        }
        // The cracks: hairlines from a point of the glass, more of them the more he is hurt.
        if r.cracked(d) {
            let seed = d as f32 * 7.31;
            let cx = mid + ((seed * 1.7).sin() * 0.3) * (x1 - x0) * 0.5;
            let cy = top * (0.45 + 0.15 * (seed * 2.3).cos());
            let rays = 4 + ((1.0 - health) * 7.0) as usize + usize::from(true_face) * 2;
            for k in 0..rays {
                let mut a = seed + k as f32 * TAU / rays as f32;
                let (mut x, mut y) = (cx, cy);
                for step in 0..4 {
                    let len = 0.16 + 0.08 * ((seed + k as f32 * 3.0 + step as f32).sin() * 0.5 + 0.5);
                    let (nx, ny) = (x + a.cos() * len, y + a.sin() * len);
                    if nx < x0 || nx > x1 || ny < 0.05 || ny > top {
                        break;
                    }
                    let m = Mat4::from_translation(Vec3::new((x + nx) / 2.0, (y + ny) / 2.0, glass_z + 0.045))
                        * Mat4::from_rotation_z(a)
                        * Mat4::from_scale(Vec3::new(len, 0.012, 0.004));
                    f.additive.push(Instance::new(m, lin(0xffffff), 0.5, Vec3::ZERO, SHADE_UNLIT));
                    (x, y) = (nx, ny);
                    a += (seed * 5.0 + k as f32 * 1.3 + step as f32 * 2.1).sin() * 0.6;
                }
            }
        }
        let face_center = Vec3::new(mid, 1.0, glass_z);
        // The statue before it, in the glass: grey stone in the true, gilded in the vain.
        if let (Some(st), true) = (door.statue, door.statue_hp > 0 && (face_center - hero).length() < STATUE_RANGE * 0.75) {
            // (Drawn just behind the glass, where the view from above can find it.)
            let at = Vec3::new(st.x as f32, 0.27, glass_z - 0.55);
            self.emit_statue(v, at, PI, door.statue_hp, !true_face, f);
        }
        if (face_center - hero).length() < 8.0 {
            lights.push((face_center + Vec3::new(0.0, 0.6, 0.7), tint * 1.6, 3.0));
        }
        // His reflection, turned to face him: behind the glass, smaller the farther he stands from
        // it (drawn close behind it, where the view from above can find it).
        let before = hp.y as f32 - glass_z;
        let within = (hp.x as f32) > x0 - 1.0 && (hp.x as f32) < x1 + 1.0;
        if before > 0.0 && before < REFLECT_RANGE && within {
            let x = (hp.x as f32).clamp(x0 + 0.3, x1 - 0.3);
            let at = Vec3::new(x, hero.y.max(0.0), glass_z - 0.35 - before * 0.12);
            self.emit_reflection(at, true_face, health, vast, 1.0 - before * 0.06, f);
        }
    }

    /// His reflection at `at`: the vain face gilded, crowned and whole, the true one as he is,
    /// darker as he is hurt; over its head, the health it shows him.
    fn emit_reflection(&self, at: Vec3, true_face: bool, health: f32, vast: bool, size: f32, f: &mut Frame) {
        let cube_i = MeshKind::Cube as usize;
        let yaw = PI - self.hero_yaw;
        let scale = if vast { 1.25 } else { 1.0 } * size;
        let root =
            Mat4::from_translation(at) * Mat4::from_rotation_y(yaw) * Mat4::from_scale(Vec3::splat(scale)) * euler(self.hero_root_rot);
        let (tint, emissive) = if true_face {
            (Vec3::splat(0.82).lerp(Vec3::new(0.55, 0.22, 0.24), (1.0 - health) * 0.8), Vec3::ZERO)
        } else {
            (Vec3::new(1.3, 1.08, 0.55), lin(GOLD) * 0.18)
        };
        emit_rig(&mut f.rigs, &self.models.hero, &self.hero_pose, root, tint, 1.0, emissive);
        let head = at + Vec3::new(0.0, 1.18 * scale, 0.0);
        if !true_face {
            // A crown of gold.
            for k in 0..7 {
                let a = k as f32 / 7.0 * TAU;
                let p = head + Vec3::new(a.cos() * 0.13, 0.02 + if k % 2 == 0 { 0.05 } else { 0.0 }, a.sin() * 0.13);
                cube(
                    &mut f.opaque[cube_i],
                    p,
                    Vec3::new(0.05, 0.08 + if k % 2 == 0 { 0.06 } else { 0.0 }, 0.05),
                    -a,
                    lin(GOLD_L),
                    SHADE_GOLD,
                );
            }
            glow_orb(f, head + Vec3::new(0.0, 0.1, 0.0), 0.3, lin(GOLD_L), 0.18);
        }
        // The health it shows him: full in the vain, true in the true.
        let shown = if true_face { health } else { 1.0 };
        let bar = head + Vec3::new(0.0, 0.32, 0.0);
        cube(&mut f.opaque[cube_i], bar, Vec3::new(0.62, 0.07, 0.02), 0.0, lin(0x1a1022), SHADE_UNLIT);
        let c = if !true_face {
            lin(GOLD_L)
        } else if shown > 0.5 {
            lin(0x6ae06a)
        } else if shown > 0.25 {
            lin(0xffc040)
        } else {
            lin(0xff4040)
        };
        if shown > 0.0 {
            let w = 0.58 * shown;
            f.additive.push(Instance::new(
                Mat4::from_translation(bar + Vec3::new(-0.29 + w / 2.0, 0.0, 0.015)) * Mat4::from_scale(Vec3::new(w, 0.045, 0.01)),
                c * 1.4,
                0.95,
                Vec3::ZERO,
                SHADE_UNLIT,
            ));
        }
    }

    /// A statue of him in triumph on its plinth at `at`, turned by `yaw`: grey stone; one before a
    /// door cracks as it is struck.
    fn emit_statue(&self, v: &MirrorsView, at: Vec3, yaw: f32, hp: u8, gilded: bool, f: &mut Frame) {
        let scale = 1.25;
        let root = Mat4::from_translation(at) * Mat4::from_rotation_y(yaw) * Mat4::from_scale(Vec3::splat(scale));
        if gilded {
            emit_rig(&mut f.rigs, &v.art.statue, &v.art.triumph, root, Vec3::new(1.5, 1.1, 0.3), 1.0, lin(GOLD) * 0.2);
        } else {
            let tint = Vec3::splat(1.0 + 0.2 * hp as f32) * Vec3::new(0.95, 0.93, 1.05);
            emit_rig_grey(&mut f.rigs, &v.art.statue, &v.art.triumph, root, tint);
        }
        if hp < 3 {
            for k in 0..(3 - hp) * 3 {
                let a = k as f32 * 2.1;
                let p = at + Vec3::new(a.cos() * 0.12, 0.4 + (k as f32 * 0.37).fract() * 0.9, a.sin() * 0.12 + 0.18);
                let m = Mat4::from_translation(p) * Mat4::from_rotation_z(a) * Mat4::from_scale(Vec3::new(0.18, 0.01, 0.01));
                f.opaque[MeshKind::Cube as usize].push(Instance::new(m, lin(0x2a2430), 1.0, Vec3::ZERO, SHADE_UNLIT));
            }
        }
    }

    /// A chandelier of mirror shards: a ring of glinting slivers about a gold boss.
    fn emit_chandelier(&self, c: Vec3, time: f32, f: &mut Frame) {
        cube(&mut f.opaque[MeshKind::Cube as usize], c, Vec3::new(0.12, 0.2, 0.12), time * 0.2, lin(GOLD), SHADE_GOLD);
        for k in 0..12 {
            let a = k as f32 / 12.0 * TAU + time * 0.15;
            let rr = 0.28 + 0.1 * (k % 3) as f32;
            let p = c + Vec3::new(a.cos() * rr, -0.15 - 0.12 * (k % 4) as f32, a.sin() * rr);
            let glint = ((time * 2.3 + k as f32 * 1.7).sin() * 0.5 + 0.5).powi(6);
            let m = Mat4::from_translation(p)
                * Mat4::from_rotation_y(a)
                * Mat4::from_rotation_z(0.3)
                * Mat4::from_scale(Vec3::new(0.03, 0.16, 0.08));
            f.opaque[MeshKind::Cube as usize].push(Instance::new(m, lin(GLASS), 1.0, lin(STAR) * (0.1 + 0.9 * glint), SHADE_LIT));
            if glint > 0.6 {
                glow_orb(f, p, 0.14, lin(STAR), 0.4 * glint);
            }
        }
    }

    /// The beam in his gallery: a flickering line while it is telegraphed, then a hard white-violet
    /// light burning along it.
    fn emit_beam(&self, r: &Mirrors, time: f32, f: &mut Frame, lights: &mut Vec<(Vec3, Vec3, f32)>) {
        let Some(b) = r.beam else { return };
        let (from, to) = (Vec3::new(b.from.x as f32, 0.55, b.from.y as f32), Vec3::new(b.to.x as f32, 0.55, b.to.y as f32));
        let d = to - from;
        let len = d.length().max(0.01);
        let yaw = d.x.atan2(d.z);
        let mid = (from + to) / 2.0;
        let burning = b.burning();
        let tell = (b.age / crate::logic::realms::mirrors::BEAM_TELL) as f32;
        let violet = lin(VIOLET);
        if burning {
            for (w, a, c) in [(0.9f32, 0.2f32, violet), (0.42, 0.5, lin(STAR)), (0.16, 0.9, Vec3::ONE)] {
                let m = Mat4::from_translation(mid) * Mat4::from_rotation_y(yaw) * Mat4::from_scale(Vec3::new(w, w * 0.8, len));
                f.additive.push(Instance::new(m, c * 1.8, a * (0.9 + 0.1 * (time * 40.0).sin()), Vec3::ZERO, SHADE_UNLIT));
            }
            // A scorch along the floor, and the glare at both ends.
            let m = Mat4::from_translation(Vec3::new(mid.x, 0.01, mid.z))
                * Mat4::from_rotation_y(yaw)
                * Mat4::from_scale(Vec3::new(0.7, 0.01, len));
            f.additive.push(Instance::new(m, violet, 0.35, Vec3::ZERO, SHADE_UNLIT));
            glow_orb(f, from, 0.9, lin(STAR), 0.6);
            glow_orb(f, to, 0.7, violet, 0.5);
            lights.push((mid + Vec3::new(0.0, 0.6, 0.0), lin(0xd8c8ff) * 9.0, 7.0));
        } else {
            // The telegraph: a thin line, flickering faster as it comes.
            let flick = ((time * (8.0 + 20.0 * tell)).sin() * 0.5 + 0.5) * 0.5 + 0.3;
            let m = Mat4::from_translation(Vec3::new(mid.x, 0.02, mid.z))
                * Mat4::from_rotation_y(yaw)
                * Mat4::from_scale(Vec3::new(0.05 + 0.25 * tell, 0.01, len));
            f.additive.push(Instance::new(m, violet * 1.5, flick * (0.3 + 0.5 * tell), Vec3::ZERO, SHADE_UNLIT));
            glow_orb(f, from, 0.5 + 0.4 * tell, lin(STAR), 0.3 * tell);
        }
    }

    /// The throne: its round floor (its rings falling), the dais and the throne, the vast mirror and
    /// his reflection in it, the Morning Star above, and the Peerless.
    #[allow(clippy::too_many_arguments)]
    fn emit_throne(
        &self,
        v: &MirrorsView,
        r: &Mirrors,
        s: &SceneState,
        hp: V,
        hero: Vec3,
        health: f32,
        time: f32,
        f: &mut Frame,
        lights: &mut Vec<(Vec3, Vec3, f32)>,
    ) {
        let cube_i = MeshKind::Cube as usize;
        let c = Vec3::new(r.throne.x as f32, 0.0, r.throne.y as f32);
        let shatter = r.fall.filter(|fl| fl.win).map(|fl| fl.t as f32);
        let ring_age = r.ring_age as f32;
        for &p in &r.throne_floor {
            let at = Vec3::new(p.x as f32, 0.0, p.y as f32);
            if (at - hero).length() > NEAR {
                continue;
            }
            let band = r.ring_of(p);
            let pit = s.grid.get(p.x, p.y) == Tile::Pit;
            // A tile that has fallen drops away, turning, and is gone.
            let drop = if let Some(t) = shatter {
                Some(t)
            } else if pit && band + 1 == r.rings && ring_age < RING_FALL {
                Some(ring_age)
            } else if pit {
                continue;
            } else {
                None
            };
            let hsh = hash(p.x as f64 * 1.9, p.y as f64 * 2.7) as f32;
            let (y, tilt) = match drop {
                Some(t) => {
                    let t = (t - hsh * 0.25).max(0.0);
                    if t > RING_FALL {
                        continue;
                    }
                    (-4.0 * t * t, t * (1.0 + 2.0 * hsh))
                }
                None => (0.0, 0.0),
            };
            let col = if r.on_dais(p) {
                lin(MARBLE_D)
            } else if (p.x + p.y) % 2 == 0 {
                lin(OBSIDIAN)
            } else {
                lin(OBSIDIAN_L)
            };
            // (Thick slabs: where the floor has fallen away, its edge is a cliff over the well.)
            let m = Mat4::from_translation(at + Vec3::new(0.0, y - 0.3, 0.0))
                * Mat4::from_rotation_x(tilt)
                * Mat4::from_rotation_z(tilt * 0.6)
                * Mat4::from_scale(Vec3::new(0.98, 0.6, 0.98));
            f.opaque[cube_i].push(Instance::new(m, col * (0.92 + 0.1 * hsh), 1.0, Vec3::ZERO, SHADE_MARBLE));
            // Gold lines where the rings will part: along the edges of a ring's tiles toward the next.
            if drop.is_none() {
                for (sx, sy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let n = P::new(p.x + sx, p.y + sy);
                    if r.throne_floor.contains(&n) && r.ring_of(n) > band {
                        let pos = at + Vec3::new(sx as f32 * 0.47, 0.004, sy as f32 * 0.47);
                        let sc = if sx != 0 { Vec3::new(0.06, 0.012, 1.0) } else { Vec3::new(1.0, 0.012, 0.06) };
                        f.opaque[cube_i].push(Instance::new(
                            Mat4::from_scale_rotation_translation(sc, Quat::IDENTITY, pos),
                            lin(GOLD),
                            1.0,
                            lin(GOLD) * 0.15,
                            SHADE_GOLD,
                        ));
                    }
                }
            }
        }
        // The dais: octagonal steps of white marble edged in gold, and the throne at its foot
        // facing the vast mirror (seen through while he stands behind it).
        if shatter.is_none() {
            for (k, rad) in [3.0f32, 2.2, 1.4].iter().enumerate() {
                let y = 0.02 + k as f32 * 0.06;
                for turn in [0.0, PI / 4.0] {
                    cube(
                        &mut f.opaque[cube_i],
                        c + Vec3::new(0.0, y, 0.0),
                        Vec3::new(*rad * 0.83, 0.05, *rad * 0.83),
                        turn,
                        lin(MARBLE_D) * 0.55,
                        SHADE_MARBLE,
                    );
                    cube(
                        &mut f.opaque[cube_i],
                        c + Vec3::new(0.0, y - 0.004, 0.0),
                        Vec3::new(*rad * 0.83 + 0.05, 0.042, *rad * 0.83 + 0.05),
                        turn,
                        lin(GOLD_D) * 0.7,
                        SHADE_GOLD,
                    );
                }
            }
            let seat_at = c + Vec3::new(0.0, 0.18, 1.7);
            let seat = Mat4::from_translation(seat_at) * Mat4::from_rotation_y(PI);
            let behind = hero.z < seat_at.z && hero.z > seat_at.z - 2.2 && (hero.x - seat_at.x).abs() < 1.2;
            if behind {
                for b in &v.art.throne.boxes {
                    let bm = seat * Mat4::from_scale_rotation_translation(b.size, Quat::IDENTITY, b.center);
                    f.opaque[cube_i].push(Instance::new(bm, b.color, 0.3, Vec3::ZERO, SHADE_DITHER));
                }
            } else {
                push_merged(&mut f.opaque[cube_i], &v.art.throne, seat, 0.03);
            }
        }
        // The Morning Star over the throne, and its light.
        let star = c + Vec3::new(0.0, 5.5, -3.0);
        let speak = if r.echo.fight { 1.4 } else { 1.0 };
        glow_orb(f, star, 2.2 * speak, lin(STAR), 0.5);
        glow_orb(f, star, 5.0, lin(VIOLET), 0.12);
        for k in 0..8 {
            let a = k as f32 / 8.0 * TAU + time * 0.1;
            let m = Mat4::from_translation(star)
                * Mat4::from_rotation_y(a)
                * Mat4::from_rotation_z(0.5)
                * Mat4::from_scale(Vec3::new(0.05, 4.0, 0.05));
            f.additive.push(Instance::new(m, lin(STAR), 0.08, Vec3::ZERO, SHADE_UNLIT));
        }
        lights.push((c + Vec3::new(0.0, 6.0, -2.0), lin(STAR) * 3.0, 14.0));
        // The vast mirror, and him in it.
        for &d in &r.galleries[THRONE].doors {
            self.emit_door(v, r, d, hp, hero, health, time, f, lights);
        }
        // The Peerless.
        for e in s.enemies.iter().filter(|e| e.kind() == EnemyKind::Peerless) {
            let Some(du) = e.duel.as_deref().filter(|d| d.alive()) else { continue };
            self.emit_peerless(v, r, du, time, f, lights);
        }
    }

    /// The Peerless: his likeness in silvered violet, doing what he did a moment ago; rising out
    /// of the floor, reeling when its echo is turned, leaping where he leapt.
    fn emit_peerless(
        &self,
        v: &MirrorsView,
        r: &Mirrors,
        du: &crate::logic::combat::Duelist,
        time: f32,
        f: &mut Frame,
        lights: &mut Vec<(Vec3, Vec3, f32)>,
    ) {
        let rest = Pose::rest(&self.models.hero);
        let reeling = matches!(du.f.state, Fight::Reel { .. });
        let pose = if reeling { &rest } else { self.echo_pose(v, time).unwrap_or(&rest) };
        let risen = r.echo.frame.saturating_sub(r.echo.rise.saturating_sub(crate::logic::realms::mirrors::RISE_FRAMES as u32)) as f32
            / crate::logic::realms::mirrors::RISE_FRAMES as f32;
        let rise = risen.clamp(0.0, 1.0);
        let lift = r
            .echo
            .snap(r.echo.frame.saturating_sub(crate::logic::realms::mirrors::ECHO_DELAY))
            .map(|s| (s.altitude as f32 * STEP).max(0.0))
            .unwrap_or(0.0);
        let at = Vec3::new(du.pos.x as f32, lift - (1.0 - rise) * 1.3, du.pos.y as f32);
        let yaw = yaw_of(du.facing.x, du.facing.y);
        let tilt = if reeling { Vec3::new(-0.35 + 0.08 * (time * 18.0).sin(), 0.0, 0.1) } else { Vec3::ZERO };
        let root = Mat4::from_translation(at) * Mat4::from_rotation_y(yaw) * euler(tilt);
        let flash = if du.flash > 0 { 0.6 } else { 0.0 };
        let emissive = lin(VIOLET) * (0.12 + 0.05 * (time * 3.0).sin()) + Vec3::splat(flash);
        let tint = Vec3::new(0.62, 0.52, 1.0);
        match (&self.models.raised, matches!(du.f.state, Fight::Guard { .. })) {
            (Some((bare, shield)), true) => {
                emit_rig(&mut f.rigs, bare, pose, root, tint, 1.0, emissive);
                let side = pose.pos.get(3).map(|p| p.x * 0.55).unwrap_or(-0.12);
                let up = root * Mat4::from_translation(Vec3::new(side, 0.06, 0.12)) * Mat4::from_rotation_y(PI);
                emit_rig(&mut f.rigs, shield, &Pose::rest(shield), up, tint, 1.0, emissive);
            }
            _ => emit_rig(&mut f.rigs, &self.models.hero, pose, root, tint, 1.0, emissive),
        }
        // Its swing's trail, in violet.
        if let Some((_, phase, _)) = du.f.attack() {
            if phase == crate::logic::combat::Phase::Active {
                for k in 0..3 {
                    let m = root
                        * Mat4::from_translation(Vec3::new(0.0, 0.45 - k as f32 * 0.03, 0.12))
                        * Mat4::from_rotation_y(time * 14.0 + k as f32 * 0.3)
                        * euler(Vec3::new(-PI / 2.0, 0.0, PI))
                        * Mat4::from_scale(Vec3::splat(1.15 + k as f32 * 0.05));
                    f.transparent[MeshKind::Ring as usize].push(Instance::new(
                        m,
                        lin(0xd8c8ff),
                        0.6 - k as f32 * 0.15,
                        Vec3::ZERO,
                        SHADE_UNLIT,
                    ));
                }
            }
        }
        if reeling {
            // Turned: a crack of white light across it.
            glow_orb(f, at + Vec3::new(0.0, 0.7, 0.0), 0.8, lin(STAR), 0.35);
        }
        if rise < 1.0 {
            // The floor gives up its reflection: a pool of silver light.
            let m = Mat4::from_translation(Vec3::new(at.x, 0.02, at.z))
                * Mat4::from_rotation_x(-PI / 2.0)
                * Mat4::from_scale(Vec3::splat(0.9 * (1.0 - rise) + 0.3));
            f.transparent[MeshKind::Disc as usize].push(Instance::new(m, lin(0xd8c8ff) * 1.6, 0.6 * (1.0 - rise), Vec3::ZERO, SHADE_UNLIT));
        }
        glow_orb(f, at + Vec3::new(0.0, 0.6, 0.0), 0.9, lin(VIOLET), 0.12);
        lights.push((at + Vec3::new(0.0, 1.2, 0.0), lin(VIOLET) * 3.0, 4.0));
    }
}

/// The realm's sky, fog and light: the dark of the spire, a single hard white light from above.
pub(super) fn atmosphere(_v: &MirrorsView, r: &Mirrors, _s: &SceneState) -> Atmosphere {
    let top = r.gallery == THRONE;
    let flattered = 1.0 + 0.05 * r.loops.min(6) as f32;
    Atmosphere {
        background: if top { 0x1a1230 } else { 0x07050c },
        fog_near: 18.0,
        fog_far: 48.0,
        hemi_sky: lin(0xd8d0ff) * if top { 0.7 } else { 0.5 } * flattered,
        hemi_ground: lin(0x3a2a60) * 0.5,
        ambient: lin(0x4a3a70) * if top { 0.75 } else { 0.6 },
        sun_dir: Vec3::new(0.25, 1.0, 0.75).normalize(),
        sun: lin(0xf4f0ff) * (if top { 1.6 } else { 1.3 }) * flattered,
        exposure: 1.1,
        night_sky: 0.0,
        bloom: 1.3,
        moon_view: Vec3::new(0.0, 0.3, -1.0).normalize(),
        cloud_tint: lin(0x140c24),
    }
}
