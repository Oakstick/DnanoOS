//! The Vaults of Mammon (`logic::vault` lays them out): gold and polished
//! marble. Cream marble floors veined in grey, a checker of cream and black
//! marble in the great treasury, red carpets with gold borders down the
//! corridors; walls of white marble blocks on black plinths under gilt
//! cornices, gold pilasters along them and gold columns in the rooms;
//! candelabra burning in every corner and sconces along the corridors; and
//! Mammon's golden idol on its black plinth in the middle of the treasury,
//! hugging a chest. The gold itself lies everywhere: coins, heaps, chests and
//! cut gems, drawn each frame while they are still there to take. The
//! portals in and out are Greed's, as in the Portals of Hell.

use super::*;
use crate::logic::hub::{Gate, Rect};
use crate::logic::sins::Sin;
use crate::logic::vault::{LootKind, RoomKind, Vault};
use crate::render::characters::Sculpt;
use crate::render::props::mesh_from;

mod pal {
    pub const MARBLE: u32 = 0xeee6d6;
    pub const MARBLE_L: u32 = 0xf8f4ec;
    pub const MARBLE_D: u32 = 0xd4c8b0;
    pub const VEIN: u32 = 0xa89c86;
    pub const BLACK: u32 = 0x24222a;
    pub const BLACK_L: u32 = 0x3a3742;
    pub const GOLD: u32 = 0xf2c53d;
    pub const GOLD_D: u32 = 0xb8902a;
    pub const GOLD_L: u32 = 0xfff0a0;
    pub const CARPET: u32 = 0x8a1a22;
    pub const CARPET_D: u32 = 0x6a1218;
    pub const FLAME: u32 = 0xffb050;
    pub const FLAME_TIP: u32 = 0xff6a20;
    pub const WOOD: u32 = 0x5a3a22;
    pub const WOOD_L: u32 = 0x7a5232;
}
use pal::*;

/// Polished gold and polished marble (see `voxel_surface` in the shader).
pub const SHADE_GOLD: f32 = 13.0;
pub const SHADE_MARBLE: f32 = 15.0;
const SHADE_WOOD: f32 = 12.0;
/// The walls stand this tall.
const WALL_H: f32 = 1.6;
/// Loot is drawn only this near the hero (tiles).
const LOOT_RANGE: f32 = 30.0;
/// Chests open over this long once taken.
const LID_SECONDS: f32 = 0.5;

/// Underground, lit by candles and the gold itself: a warm golden haze, no sky.
pub fn vault_atmosphere() -> Atmosphere {
    Atmosphere {
        background: 0x120c06,
        fog_near: 16.0,
        fog_far: 44.0,
        hemi_sky: lin(0xffd890) * 0.42,
        hemi_ground: lin(0x7a5420) * 0.45,
        ambient: lin(0x8a6a34) * 0.42,
        sun_dir: Vec3::new(0.45, 1.0, 0.3).normalize(),
        sun: lin(0xfff0c8) * 1.25,
        exposure: 1.15,
        night_sky: 0.0,
        bloom: 1.15,
        moon_view: Vec3::new(0.0, 0.3, -1.0).normalize(),
        cloud_tint: lin(0x2a1a08),
    }
}

/// What the view keeps of the vault: where the gold lies and what it is, how long each chest has
/// stood open, the lamps, and the two portals.
pub struct VaultView {
    pub loot: Vec<(P, LootKind)>,
    pub opened: Vec<f32>,
    pub lamps: Vec<Vec3>,
    pub gates: [Gate; 2],
    pub idol: Vec3,
}

fn cube(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot_y: f32, color: Vec3, shade: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(rot_y), pos), color, 1.0, Vec3::ZERO, shade));
}

fn marble(h: f64) -> Vec3 {
    let set = [MARBLE, MARBLE, MARBLE_L, MARBLE_D];
    lin(set[(h * 4.0) as usize % 4]) * (0.96 + h as f32 * 0.06)
}

// ------------------------------------------------------------------ meshes

/// A little stack of gold coins.
pub fn coin() -> Mesh {
    let mut s = Sculpt::new(7, 4, 7);
    for y in 0..4 {
        let (cx, cz) = (3.0 + if y % 2 == 1 { 0.6 } else { 0.0 }, 3.0 - if y == 2 { 0.5 } else { 0.0 });
        for z in 0..7 {
            for x in 0..7 {
                let d = ((x as f32 - cx).powi(2) + (z as f32 - cz).powi(2)).sqrt();
                if d <= 2.6 {
                    s.set(x, y, z, if d > 2.0 { GOLD_D } else if y == 3 { GOLD_L } else { GOLD });
                }
            }
        }
    }
    mesh_from(&s.finish(0.03), 0.045)
}

/// A heap of gold coins.
pub fn pile() -> Mesh {
    let mut s = Sculpt::new(11, 6, 11);
    s.ellipsoid((5.0, 0.3, 5.0), (5.4, 4.6, 4.8), 0, GOLD);
    s.ellipsoid((3.2, 0.8, 6.6), (3.0, 2.8, 2.6), 0, GOLD);
    s.ellipsoid((7.4, 1.2, 3.6), (2.6, 3.0, 2.6), 0, GOLD);
    s.paint(|x, y, z, _| {
        let n = ((x * 7 + y * 13 + z * 3) % 11) as f32 / 11.0;
        Some(if n < 0.12 { GOLD_L } else if n > 0.8 { GOLD_D } else if y == 0 { GOLD_D } else { GOLD })
    });
    mesh_from(&s.finish(0.05), 0.06)
}

/// A treasure chest: the body, and the lid (hinged at its back edge, at the lid's origin).
pub fn chest() -> (Mesh, Mesh) {
    let mut b = Sculpt::new(12, 6, 8);
    b.cuboid(0, 0, 0, 11, 5, 7, WOOD);
    b.paint(|x, y, z, c| {
        let band = x == 2 || x == 9 || x == 5 || x == 6;
        let edge = y == 0 || z == 0 || z == 7 || x == 0 || x == 11;
        Some(if band { GOLD_D } else if edge && (y + z) % 2 == 0 { WOOD_L } else { c })
    });
    // Inside: gold to the brim, seen when the lid opens.
    for z in 1..7 {
        for x in 1..11 {
            b.set(x, 5, z, if (x * 3 + z * 5) % 7 == 0 { GOLD_L } else { GOLD });
        }
    }
    let mut l = Sculpt::new(12, 4, 8);
    for y in 0..4 {
        let inset = match y {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => 3,
        };
        l.cuboid(0, y, inset, 11, y, 7 - inset, WOOD);
    }
    l.paint(|x, y, _, c| Some(if x == 2 || x == 9 || x == 5 || x == 6 { GOLD_D } else if y == 3 { WOOD_L } else { c }));
    // The lock plate on the front.
    l.cuboid(5, 0, 7, 6, 1, 7, GOLD);
    let size = 0.055;
    let body = mesh_from(&b.finish(0.03), size);
    // The lid's cubes are shifted so its origin sits on its back bottom edge: the hinge.
    let lid_c = l.finish(0.03);
    let mut lid = mesh_from(&lid_c, size);
    let back = -(lid_c.depth as f32 - 1.0) / 2.0 * size;
    for (p, _) in lid.cubes.iter_mut() {
        p.z -= back;
    }
    (body, lid)
}

impl WorldView {
    /// The vault's static geometry.
    pub(super) fn build_vault(&mut self, grid: &Grid, v: &Vault) {
        let rock = |x: i32, y: i32| grid.get(x, y) == Tile::Rock;
        let room_of = |p: P| v.rooms.iter().find(|r| r.rect.contains(p));
        let hall = v.rooms.iter().find(|r| r.kind == RoomKind::Hall).map(|r| r.rect).unwrap_or(Rect { x0: 0, y0: 0, x1: 0, y1: 0 });
        let carpet: Vec<(Rect, bool)> = v.corridors.iter().map(|c| (*c, c.along_x())).collect();
        let is_carpet = |p: P| -> Option<bool> {
            carpet.iter().find(|(c, _)| c.contains(p)).map(|(c, along)| {
                if *along {
                    p.y == (c.y0 + c.y1) / 2
                } else {
                    p.x == (c.x0 + c.x1) / 2
                }
            })
        };
        let mut lamps: Vec<Vec3> = Vec::new();

        // ---- floors
        for (x, y, t) in grid.cells() {
            if t == Tile::Rock {
                continue;
            }
            let p = P::new(x, y);
            let (xf, yf) = (x as f32, y as f32);
            let (hx, hy) = (x as f64, y as f64);
            let h = hash(hx * 1.7 + 3.0, hy * 2.3);
            let out = &mut self.static_cubes;
            let room = room_of(p);
            if let Some(r) = room {
                let rect = r.rect;
                let on_edge = x == rect.x0 || x == rect.x1 || y == rect.y0 || y == rect.y1;
                if r.kind == RoomKind::Hall && !on_edge {
                    // The treasury's checker of cream and black marble.
                    let black = (x + y) % 2 == 0;
                    let c = if black { lin(BLACK) * (0.9 + h as f32 * 0.2) } else { marble(h) };
                    cube(out, Vec3::new(xf, -0.06, yf), Vec3::new(0.97, 0.12, 0.97), 0.0, c, SHADE_MARBLE);
                    if black && h > 0.7 {
                        cube(out, Vec3::new(xf + (h as f32 - 0.85) * 0.8, 0.004, yf), Vec3::new(0.5, 0.01, 0.03), h as f32 * PI, lin(BLACK_L), SHADE_LIT);
                    }
                } else if on_edge {
                    // A border of darker marble round every room, a gold line along its inner edge.
                    cube(out, Vec3::new(xf, -0.06, yf), Vec3::new(0.97, 0.12, 0.97), 0.0, lin(MARBLE_D) * (0.94 + h as f32 * 0.08), SHADE_MARBLE);
                    for d in CARDINALS {
                        let n = p.step(d);
                        let inner = rect.contains(n) && !(n.x == rect.x0 || n.x == rect.x1 || n.y == rect.y0 || n.y == rect.y1);
                        if !inner {
                            continue;
                        }
                        let (dx, dy) = d.vec();
                        let pos = Vec3::new(xf + dx as f32 * 0.47, 0.004, yf + dy as f32 * 0.47);
                        let sc = if dx != 0 { Vec3::new(0.05, 0.012, 0.98) } else { Vec3::new(0.98, 0.012, 0.05) };
                        cube(out, pos, sc, 0.0, lin(GOLD), SHADE_GOLD);
                    }
                } else {
                    cube(out, Vec3::new(xf, -0.06, yf), Vec3::new(0.97, 0.12, 0.97), 0.0, marble(h), SHADE_MARBLE);
                    if h > 0.82 {
                        // A grey vein across the slab.
                        let a = hash(hx * 5.0, hy * 7.0) as f32 * PI;
                        cube(out, Vec3::new(xf + (h as f32 - 0.9) * 0.6, 0.004, yf + (hash(hy, hx) as f32 - 0.5) * 0.4), Vec3::new(0.6, 0.01, 0.025), a, lin(VEIN), SHADE_LIT);
                    }
                }
            } else {
                // A corridor: marble either side of a red carpet with gold borders.
                cube(out, Vec3::new(xf, -0.06, yf), Vec3::new(0.97, 0.12, 0.97), 0.0, marble(h), SHADE_MARBLE);
                match is_carpet(p) {
                    Some(true) => {
                        cube(out, Vec3::new(xf, 0.01, yf), Vec3::new(1.0, 0.02, 1.0), 0.0, lin(if h > 0.5 { CARPET } else { CARPET_D }), SHADE_LIT);
                    }
                    Some(false) => {
                        // The border tiles: a gold thread along the carpet's edge.
                        for d in CARDINALS {
                            let n = p.step(d);
                            if is_carpet(n) == Some(true) {
                                let (dx, dy) = d.vec();
                                let pos = Vec3::new(xf + dx as f32 * 0.46, 0.025, yf + dy as f32 * 0.46);
                                let sc = if dx != 0 { Vec3::new(0.06, 0.012, 1.0) } else { Vec3::new(1.0, 0.012, 0.06) };
                                cube(out, pos, sc, 0.0, lin(GOLD), SHADE_GOLD);
                            }
                        }
                    }
                    None => {}
                }
            }
        }

        // ---- walls and columns
        for (x, y, t) in grid.cells() {
            if t != Tile::Rock {
                continue;
            }
            let p = P::new(x, y);
            if p == v.entry_gate || p == v.exit_gate || p == v.idol {
                continue;
            }
            let (xf, yf) = (x as f32, y as f32);
            let (hx, hy) = (x as f64, y as f64);
            let open: Vec<Dir> = CARDINALS.iter().copied().filter(|d| {
                let n = p.step(*d);
                grid.in_bounds(n.x, n.y) && !rock(n.x, n.y)
            }).collect();
            if open.is_empty() {
                continue; // solid rock, out of sight
            }
            let start = self.static_cubes.len();
            if open.len() >= 3 {
                self.build_gold_column(Vec3::new(xf, 0.0, yf), hash(hx, hy) as f32);
            } else {
                self.build_wall_block(p, &open, hash(hx * 3.0, hy * 5.0));
            }
            let len = self.static_cubes.len() - start;
            self.rock_index.insert(tile_key(x, y), (start, len));
            // Walls and columns dissolve when they stand between the camera and him, as the woods' trees do.
            let shades: Vec<f32> = self.static_cubes[start..start + len].iter().map(|i| i.emissive[3]).collect();
            self.trees.push(Tree { range: (start, len), base: Vec3::new(xf, 0.0, yf), radius: 0.72, height: WALL_H + 0.4, fade: 1.0, alive: true, shades: Some(shades) });
        }

        // ---- beyond the walls: the dark tops of the rock they are cut from, so every wall looks thick
        for (x, y, t) in grid.cells() {
            if t != Tile::Rock {
                continue;
            }
            let faces_floor = |q: P| CARDINALS.iter().any(|d| {
                let n = q.step(*d);
                grid.in_bounds(n.x, n.y) && !rock(n.x, n.y)
            });
            let p = P::new(x, y);
            if faces_floor(p) {
                continue;
            }
            let behind_a_wall = (-1..=1).any(|dy| (-1..=1).any(|dx| {
                let q = P::new(x + dx, y + dy);
                grid.in_bounds(q.x, q.y) && rock(q.x, q.y) && faces_floor(q)
            }));
            if !behind_a_wall {
                continue;
            }
            let h = hash(x as f64 * 2.1, y as f64 * 1.3) as f32;
            cube(&mut self.static_cubes, Vec3::new(x as f32, WALL_H - 0.03, y as f32), Vec3::new(1.0, 0.1, 1.0), 0.0, lin(BLACK_L) * (0.7 + h * 0.2), SHADE_MARBLE);
        }

        // ---- lamps: candelabra in the rooms' corners, sconces along the corridors
        for r in &v.rooms {
            let rect = r.rect;
            let corners = [(rect.x0 + 1, rect.y0 + 1), (rect.x1 - 1, rect.y0 + 1), (rect.x0 + 1, rect.y1 - 1), (rect.x1 - 1, rect.y1 - 1)];
            for (cx, cy) in corners {
                if rock(cx, cy) || P::new(cx, cy) == v.arrival {
                    continue;
                }
                let base = Vec3::new(cx as f32, 0.0, cy as f32);
                self.build_candelabrum(base);
                lamps.push(base + Vec3::new(0.0, 1.32, 0.0));
            }
            // The larger rooms have candelabra along their long sides too.
            let (w, h) = (rect.x1 - rect.x0 + 1, rect.y1 - rect.y0 + 1);
            if w >= 16 {
                for cy in [rect.y0 + 1, rect.y1 - 1] {
                    let cx = (rect.x0 + rect.x1) / 2;
                    if !rock(cx, cy) && !v.loot.iter().any(|l| l.at == P::new(cx, cy)) {
                        let base = Vec3::new(cx as f32, 0.0, cy as f32);
                        self.build_candelabrum(base);
                        lamps.push(base + Vec3::new(0.0, 1.32, 0.0));
                    }
                }
            }
            if h >= 16 {
                for cx in [rect.x0 + 1, rect.x1 - 1] {
                    let cy = (rect.y0 + rect.y1) / 2;
                    if !rock(cx, cy) && !v.loot.iter().any(|l| l.at == P::new(cx, cy)) {
                        let base = Vec3::new(cx as f32, 0.0, cy as f32);
                        self.build_candelabrum(base);
                        lamps.push(base + Vec3::new(0.0, 1.32, 0.0));
                    }
                }
            }
        }
        for (c, along) in &carpet {
            let len = if *along { c.x1 - c.x0 + 1 } else { c.y1 - c.y0 + 1 };
            let mut k = 2;
            while k < len - 1 {
                // A sconce on the wall to one side, if there is wall there.
                let (tile, wall, dir) = if *along {
                    (P::new(c.x0 + k, c.y0), P::new(c.x0 + k, c.y0 - 1), Dir::Down)
                } else {
                    (P::new(c.x0, c.y0 + k), P::new(c.x0 - 1, c.y0 + k), Dir::Right)
                };
                if rock(wall.x, wall.y) && !rock(tile.x, tile.y) {
                    let (dx, dy) = dir.vec();
                    let at = Vec3::new(wall.x as f32 + dx as f32 * 0.55, 1.1, wall.y as f32 + dy as f32 * 0.55);
                    self.build_sconce(at, dir);
                    lamps.push(at + Vec3::new(0.0, 0.18, 0.0));
                }
                k += 6;
            }
        }

        // ---- Mammon's idol in the treasury, and the portals
        let idol = Vec3::new(v.idol.x as f32, 0.0, v.idol.y as f32);
        self.build_idol(idol, hall);
        let gates = [
            Gate { tile: v.entry_gate, facing: Dir::Right, sin: Sin::Greed },
            Gate { tile: v.exit_gate, facing: Dir::Left, sin: Sin::Greed },
        ];
        for g in &gates {
            self.build_portal_arch(g);
        }
        // The floor is walkable under the arch when the doors open: a marble threshold behind it.
        for g in &gates {
            let (dx, dy) = g.facing.vec();
            self.static_cubes.push(Instance::new(
                Mat4::from_scale_rotation_translation(Vec3::new(1.0, 0.1, 1.0), Quat::IDENTITY, Vec3::new(g.tile.x as f32 - dx as f32 * 0.5, -0.06, g.tile.y as f32 - dy as f32 * 0.5)),
                lin(BLACK),
                1.0,
                Vec3::ZERO,
                SHADE_LIT,
            ));
        }
        self.vault = Some(VaultView {
            loot: v.loot.iter().map(|l| (l.at, l.kind)).collect(),
            opened: vec![0.0; v.loot.len()],
            lamps,
            gates,
            idol,
        });
    }

    /// A wall block: white marble on a black plinth under a gilt cornice, a gold pilaster now and then.
    fn build_wall_block(&mut self, p: P, open: &[Dir], h: f64) {
        let (xf, yf) = (p.x as f32, p.y as f32);
        let out = &mut self.static_cubes;
        // The plinth, the block and the cornice.
        cube(out, Vec3::new(xf, 0.09, yf), Vec3::new(1.0, 0.18, 1.0), 0.0, lin(BLACK) * (0.9 + h as f32 * 0.15), SHADE_MARBLE);
        cube(out, Vec3::new(xf, WALL_H / 2.0 + 0.05, yf), Vec3::new(1.0, WALL_H - 0.3, 1.0), 0.0, marble(h) * 0.97, SHADE_MARBLE);
        cube(out, Vec3::new(xf, WALL_H - 0.09, yf), Vec3::new(1.0, 0.06, 1.0), 0.0, lin(GOLD_D), SHADE_GOLD);
        cube(out, Vec3::new(xf, WALL_H, yf), Vec3::new(1.08, 0.12, 1.08), 0.0, lin(MARBLE_D), SHADE_MARBLE);
        // A course line in the marble.
        cube(out, Vec3::new(xf, 0.82, yf), Vec3::new(1.004, 0.02, 1.004), 0.0, lin(VEIN) * 0.9, SHADE_LIT);
        // Pilasters on the faces that look into a room or corridor, every fourth block.
        if (p.x + p.y).rem_euclid(4) == 0 {
            for &d in open {
                let (dx, dy) = d.vec();
                let face = Vec3::new(xf + dx as f32 * 0.55, 0.0, yf + dy as f32 * 0.55);
                let flat = if dx != 0 { Vec3::new(0.12, 1.0, 0.36) } else { Vec3::new(0.36, 1.0, 0.12) };
                cube(out, face + Vec3::new(0.0, 0.16, 0.0), Vec3::new(flat.x * 1.3, 0.14, flat.z * 1.3), 0.0, lin(BLACK_L), SHADE_MARBLE);
                cube(out, face + Vec3::new(0.0, WALL_H / 2.0 - 0.05, 0.0), Vec3::new(flat.x, WALL_H - 0.5, flat.z), 0.0, lin(GOLD), SHADE_GOLD);
                cube(out, face + Vec3::new(0.0, WALL_H - 0.24, 0.0), Vec3::new(flat.x * 1.4, 0.12, flat.z * 1.4), 0.0, lin(GOLD_L), SHADE_GOLD);
            }
        }
    }

    /// A gold column on a black base.
    fn build_gold_column(&mut self, base: Vec3, h: f32) {
        let out = &mut self.static_cubes;
        cube(out, base + Vec3::new(0.0, 0.1, 0.0), Vec3::new(0.86, 0.2, 0.86), 0.0, lin(BLACK), SHADE_MARBLE);
        cube(out, base + Vec3::new(0.0, 0.25, 0.0), Vec3::new(0.66, 0.1, 0.66), PI / 4.0, lin(GOLD_D), SHADE_GOLD);
        for k in 0..7 {
            let y = 0.3 + k as f32 * 0.22;
            cube(out, base + Vec3::new(0.0, y + 0.11, 0.0), Vec3::new(0.46, 0.22, 0.46), k as f32 * PI / 8.0 + h, lin(GOLD) * (0.95 + 0.05 * (k % 2) as f32), SHADE_GOLD);
        }
        cube(out, base + Vec3::new(0.0, 1.9, 0.0), Vec3::new(0.7, 0.12, 0.7), PI / 4.0, lin(GOLD_L), SHADE_GOLD);
        cube(out, base + Vec3::new(0.0, 2.02, 0.0), Vec3::new(0.9, 0.12, 0.9), 0.0, lin(MARBLE_D), SHADE_MARBLE);
    }

    /// A tall gold candelabrum with three candles.
    fn build_candelabrum(&mut self, base: Vec3) {
        let out = &mut self.static_cubes;
        cube(out, base + Vec3::new(0.0, 0.04, 0.0), Vec3::new(0.4, 0.08, 0.4), PI / 4.0, lin(BLACK), SHADE_MARBLE);
        cube(out, base + Vec3::new(0.0, 0.6, 0.0), Vec3::new(0.07, 1.1, 0.07), 0.0, lin(GOLD), SHADE_GOLD);
        cube(out, base + Vec3::new(0.0, 0.35, 0.0), Vec3::new(0.16, 0.06, 0.16), PI / 4.0, lin(GOLD_D), SHADE_GOLD);
        cube(out, base + Vec3::new(0.0, 1.14, 0.0), Vec3::new(0.56, 0.05, 0.07), 0.0, lin(GOLD), SHADE_GOLD);
        for dx in [-0.25f32, 0.0, 0.25] {
            cube(out, base + Vec3::new(dx, 1.2, 0.0), Vec3::new(0.1, 0.05, 0.1), 0.0, lin(GOLD_D), SHADE_GOLD);
            cube(out, base + Vec3::new(dx, 1.27, 0.0), Vec3::new(0.05, 0.12, 0.05), 0.0, lin(0xf4ecd8), SHADE_LIT);
        }
    }

    /// A wall sconce: a gold bracket holding a candle.
    fn build_sconce(&mut self, at: Vec3, d: Dir) {
        let (dx, dy) = d.vec();
        let out = &mut self.static_cubes;
        let back = at - Vec3::new(dx as f32 * 0.05, 0.0, dy as f32 * 0.05);
        cube(out, back - Vec3::new(0.0, 0.12, 0.0), Vec3::new(if dx != 0 { 0.05 } else { 0.16 }, 0.3, if dx != 0 { 0.16 } else { 0.05 }), 0.0, lin(GOLD_D), SHADE_GOLD);
        cube(out, at, Vec3::new(0.14, 0.04, 0.14), 0.0, lin(GOLD), SHADE_GOLD);
        cube(out, at + Vec3::new(0.0, 0.08, 0.0), Vec3::new(0.05, 0.12, 0.05), 0.0, lin(0xf4ecd8), SHADE_LIT);
    }

    /// Mammon on his plinth: a squat gold figure with horns and a grin, hugging a chest of gold.
    fn build_idol(&mut self, base: Vec3, _hall: Rect) {
        let out = &mut self.static_cubes;
        let g = lin(GOLD);
        let gd = lin(GOLD_D);
        cube(out, base + Vec3::new(0.0, 0.16, 0.0), Vec3::new(1.0, 0.32, 1.0), 0.0, lin(BLACK), SHADE_MARBLE);
        cube(out, base + Vec3::new(0.0, 0.34, 0.0), Vec3::new(0.86, 0.04, 0.86), 0.0, g, SHADE_GOLD);
        let y0 = 0.36;
        // Legs folded under a fat belly.
        cube(out, base + Vec3::new(0.0, y0 + 0.12, 0.05), Vec3::new(0.7, 0.24, 0.6), 0.0, gd, SHADE_GOLD);
        cube(out, base + Vec3::new(0.0, y0 + 0.42, -0.02), Vec3::new(0.62, 0.42, 0.5), 0.0, g, SHADE_GOLD);
        cube(out, base + Vec3::new(0.0, y0 + 0.5, 0.2), Vec3::new(0.46, 0.34, 0.2), 0.0, g * 1.05, SHADE_GOLD);
        // The chest in his arms, brimming.
        cube(out, base + Vec3::new(0.0, y0 + 0.36, 0.3), Vec3::new(0.4, 0.22, 0.24), 0.0, lin(WOOD), SHADE_LIT);
        cube(out, base + Vec3::new(0.0, y0 + 0.49, 0.3), Vec3::new(0.36, 0.05, 0.2), 0.0, lin(GOLD_L), SHADE_GOLD);
        for side in [-1.0f32, 1.0] {
            cube(out, base + Vec3::new(side * 0.3, y0 + 0.55, 0.05), Vec3::new(0.16, 0.16, 0.24), side * 0.3, g, SHADE_GOLD);
            cube(out, base + Vec3::new(side * 0.26, y0 + 0.42, 0.28), Vec3::new(0.14, 0.14, 0.3), 0.0, g, SHADE_GOLD);
        }
        // The head: heavy brows, a wide grin, curled horns.
        cube(out, base + Vec3::new(0.0, y0 + 0.86, 0.0), Vec3::new(0.4, 0.36, 0.38), 0.0, g, SHADE_GOLD);
        cube(out, base + Vec3::new(0.0, y0 + 0.78, 0.2), Vec3::new(0.28, 0.04, 0.03), 0.0, lin(BLACK), SHADE_LIT);
        for side in [-1.0f32, 1.0] {
            cube(out, base + Vec3::new(side * 0.1, y0 + 0.92, 0.2), Vec3::new(0.08, 0.05, 0.02), 0.0, lin(0xe0203a), SHADE_LIT);
            for k in 0..4 {
                let t = k as f32 / 3.0;
                cube(out, base + Vec3::new(side * (0.2 + 0.12 * t), y0 + 1.02 + 0.2 * t - 0.05 * t * t, -0.05 * t), Vec3::splat(0.1 - 0.02 * t), side * t, gd, SHADE_GOLD);
            }
        }
        // A crown of coins.
        for k in 0..6 {
            let a = k as f32 / 6.0 * TAU;
            cube(out, base + Vec3::new(a.cos() * 0.17, y0 + 1.08, a.sin() * 0.17), Vec3::new(0.06, 0.08, 0.02), -a, lin(GOLD_L), SHADE_GOLD);
        }
    }

    /// The gold where it lies, the candle flames, the lamps' light, the portals.
    pub(super) fn emit_vault(&self, s: &SceneState, time: f32, hero: Vec3, f: &mut Frame) {
        let Some(vv) = &self.vault else { return };
        let cube_i = MeshKind::Cube as usize;
        let near = |p: Vec3| Vec2::new(p.x - hero.x, p.z - hero.z).length_squared() < LOOT_RANGE * LOOT_RANGE;
        let mut lit: Vec<(Vec3, Vec3)> = Vec::new();
        for (i, (p, kind)) in vv.loot.iter().enumerate() {
            let taken = s.loot.get(i).is_some_and(|l| l.taken);
            let base = Vec3::new(p.x as f32, 0.0, p.y as f32);
            if !near(base) {
                continue;
            }
            let seed = (p.x * 7 + p.y * 13) as f32;
            match kind {
                LootKind::Coin if !taken => {
                    let m = Mat4::from_translation(base + Vec3::new(0.0, 0.0, 0.0)) * Mat4::from_rotation_y(seed);
                    for (off, c) in &self.models.coin.cubes {
                        f.opaque[cube_i].push(Instance::new(voxel_matrix(&m, self.models.coin.size, *off), *c, 1.0, *c * 0.08, SHADE_GOLD));
                    }
                    // A glint now and then.
                    let g = ((time * 1.7 + seed).sin() * 0.5 + 0.5).powi(8);
                    if g > 0.2 {
                        glow_orb(f, base + Vec3::new(0.0, 0.14, 0.0), 0.12, lin(GOLD_L), 0.35 * g);
                    }
                }
                LootKind::Pile if !taken => {
                    let m = Mat4::from_translation(base) * Mat4::from_rotation_y(seed * 0.7);
                    for (off, c) in &self.models.pile.cubes {
                        f.opaque[cube_i].push(Instance::new(voxel_matrix(&m, self.models.pile.size, *off), *c, 1.0, *c * 0.1, SHADE_GOLD));
                    }
                    lit.push((base + Vec3::new(0.0, 0.4, 0.0), lin(GOLD) * 2.0));
                }
                LootKind::Chest => {
                    let (body, lid) = &self.models.chest;
                    let yaw = if (p.x + p.y) % 2 == 0 { 0.0 } else { PI / 2.0 } + (seed * 0.37).sin() * 0.15;
                    let m = Mat4::from_translation(base) * Mat4::from_rotation_y(yaw);
                    for (off, c) in &body.cubes {
                        let shade = if c.x > 0.3 { SHADE_GOLD } else { SHADE_WOOD };
                        f.opaque[cube_i].push(Instance::new(voxel_matrix(&m, body.size, *off), *c, 1.0, Vec3::ZERO, shade));
                    }
                    let age = vv.opened.get(i).copied().unwrap_or(0.0);
                    let open = if taken { (age / LID_SECONDS).min(1.0) } else { 0.0 };
                    let angle = -(open * open * (3.0 - 2.0 * open)) * 1.9;
                    let hinge = m * Mat4::from_translation(Vec3::new(0.0, 6.0 * body.size, -3.5 * body.size)) * Mat4::from_rotation_x(angle);
                    for (off, c) in &lid.cubes {
                        let shade = if c.x > 0.3 { SHADE_GOLD } else { SHADE_WOOD };
                        f.opaque[cube_i].push(Instance::new(voxel_matrix(&hinge, lid.size, *off), *c, 1.0, Vec3::ZERO, shade));
                    }
                    if !taken {
                        lit.push((base + Vec3::new(0.0, 0.5, 0.0), lin(0xffd060) * 1.6));
                    } else if open < 1.0 {
                        // Gold spilling as the lid goes up.
                        for k in 0..10 {
                            let t = (open * 1.6 + k as f32 * 0.13).fract();
                            let a = k as f32 * 2.4 + seed;
                            let sp = base + Vec3::new(a.cos() * 0.35 * t, 0.35 + t * 0.8 - t * t * 0.9, a.sin() * 0.35 * t);
                            f.additive.push(Instance::new(Mat4::from_translation(sp) * Mat4::from_scale(Vec3::splat(0.04)), lin(GOLD_L), 0.8 * (1.0 - t), Vec3::ZERO, SHADE_UNLIT));
                        }
                    }
                }
                LootKind::Gem(hex) if !taken => {
                    let c = lin(*hex);
                    let bob = (time * 2.0 + seed).sin() * 0.05;
                    let at = base + Vec3::new(0.0, 0.32 + bob, 0.0);
                    let spin = Quat::from_rotation_y(time * 1.4 + seed);
                    let m = &self.models.diamond;
                    for (off, col) in &m.cubes {
                        let pm = Mat4::from_scale_rotation_translation(Vec3::splat(m.size * 0.8), spin, at + spin * (*off * 0.8) - Vec3::new(0.0, 0.28, 0.0));
                        f.opaque[cube_i].push(Instance::new(pm, *col * c * 1.4, 1.0, c * 0.35, SHADE_LIT));
                    }
                    glow_orb(f, at, 0.3 + (time * 3.0 + seed).sin() * 0.03, c, 0.25);
                    for k in 0..3 {
                        let a = time * 1.5 + k as f32 * TAU / 3.0 + seed;
                        let sp = at + Vec3::new(a.cos() * 0.26, (a * 1.7).sin() * 0.12, a.sin() * 0.26);
                        f.additive.push(Instance::new(Mat4::from_translation(sp) * Mat4::from_scale(Vec3::splat(0.03)), Vec3::ONE.lerp(c, 0.5), 0.8, Vec3::ZERO, SHADE_UNLIT));
                    }
                    lit.push((at, c * 2.5));
                }
                _ => {}
            }
        }
        // Candle flames on every lamp in sight; the nearest few light the marble.
        let mut lamp_points: Vec<Vec3> = Vec::new();
        for (i, l) in vv.lamps.iter().enumerate() {
            if !near(*l) {
                continue;
            }
            lamp_points.push(*l);
            let fi = i as f32;
            for k in 0..3 {
                let t = (time * 2.6 + fi * 0.37 + k as f32 * 0.33).fract();
                let p = *l + Vec3::new((fi * 1.9 + k as f32).sin() * 0.03, t * 0.16, (fi * 2.7).cos() * 0.03);
                let size = 0.06 * (1.0 - t) + 0.02;
                let c = lin(FLAME).lerp(lin(FLAME_TIP), t);
                f.additive.push(Instance::new(Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(size, size * 1.6, size)), c, 0.55 * (1.0 - t), Vec3::ZERO, SHADE_UNLIT));
            }
            glow_orb(f, *l + Vec3::new(0.0, 0.05, 0.0), 0.3, lin(FLAME), 0.18 + 0.03 * (time * 9.0 + fi).sin());
        }
        for (i, p) in nearest(&lamp_points, hero, 4).into_iter().enumerate() {
            let flick = 5.0 + (time * 7.0 + i as f32 * 1.3).sin() * 0.5 + (time * 17.0 + i as f32).sin() * 0.25;
            f.lights.push(Light { pos: p + Vec3::new(0.0, 0.3, 0.0), color: lin(0xffc070) * flick, distance: 8.0 });
        }
        let lit_points: Vec<Vec3> = lit.iter().map(|(p, _)| *p).collect();
        for p in nearest(&lit_points, hero, 3) {
            let c = lit.iter().find(|(q, _)| *q == p).map(|(_, c)| *c).unwrap_or(Vec3::ONE);
            f.lights.push(Light { pos: p, color: c, distance: 3.5 });
        }
        // The idol gleams.
        glow_orb(f, vv.idol + Vec3::new(0.0, 1.1, 0.0), 0.9, lin(GOLD), 0.12 + 0.03 * (time * 2.0).sin());
        f.lights.push(Light { pos: vv.idol + Vec3::new(0.0, 1.6, 0.6), color: lin(GOLD) * 4.0, distance: 7.0 });
        // The portals: the one he came through shut behind him, the far one opening for the empty-handed.
        let door = s.gate.map(|g| g.door).unwrap_or(0.0);
        self.emit_portal(&vv.gates[0], 0.0, false, time, hero, f);
        self.emit_portal(&vv.gates[1], door, false, time, hero, f);
    }
}
