//! The Vaults of Mammon (`logic::vault` lays them out), at the Cisterns' bar: Greed's strongrooms
//! cut in dressed stone and heaped with gold. Worn flags underfoot (the great treasury in polished
//! black and cream marble), red runners with gilt borders down the corridors; walls of the
//! crypt's dressed stone under a pale cornice with a gilt band, gilded pilasters along them and
//! gilded columns in the rooms; strongroom doors of riveted iron, niches holding statues or racks
//! of gold bars and plate, paintings in gilt frames (`vault_art`, `vault_decor`); candelabra in
//! the rooms' corners, copper braziers along the great rooms' sides and lanterns on brackets down
//! the corridors, each a warm pool on the gold and the stone with the dark deep between; and
//! Mammon's golden idol on its plinth in the middle of the treasury, hugging a chest. The gold
//! lies everywhere: coins, heaps, treasure heaps and chests are part of the static scene, hidden
//! as they are taken (a chest's lid swings open instead); it is metal, and glints where the
//! lamps catch it. Gems spin, and the items the chests throw out fly, land and stand in a beam
//! of their rarity's colour. The portals in and out are Greed's, as in the Portals of Hell.

use super::crypt::jit;
use super::vault_art::{art, coin_stack, gold, yaw_of, Face, GOLD_GLOW, SHADE_COINS, SHADE_GEM, SHADE_GILT, WALL_H};
use super::vault_decor::{self, push_merged, Merged};
use super::*;
use crate::logic::hub::{Gate, Rect};
use crate::logic::items::item;
use crate::logic::sins::Sin;
use crate::logic::vault::{DropItem, LootKind, Rarity, RoomKind, Vault, DROP_FLIGHT};
use crate::render::characters::{mat, Sculpt};

mod pal {
    pub const FLAME: u32 = 0xffb050;
    pub const FLAME_TIP: u32 = 0xff6a20;
    pub const GOLD: u32 = 0xf2c53d;
    pub const GOLD_L: u32 = 0xfff0a0;
    /// The candles' light, and the braziers' and lanterns'.
    pub const CANDLE: u32 = 0xffc080;
    pub const FIRE: u32 = 0xffb468;
    /// The dark the far rooms fall into: charcoal with the mist's teal in it.
    pub const DARK: u32 = 0x151b1b;
}
use pal::*;

/// Polished gold and polished marble (see `voxel_surface` in the shader; the other courts and
/// realms that borrow the vaults' look use these).
pub const SHADE_GOLD: f32 = 13.0;
pub const SHADE_MARBLE: f32 = 15.0;
/// Loot is drawn only this near the hero (tiles).
const LOOT_RANGE: f32 = 30.0;
/// Chests open over this long once taken.
const LID_SECONDS: f32 = 0.5;
/// The practicals lit at once, nearest the hero first (RP17 bins every one).
pub const VAULT_LAMPS_LIT: usize = 16;

/// Underground, lit by its practicals: a deep warm dark, the candles and braziers pooling on the
/// gold and the stone, a dim pale light from shafts far above, and the far dark and the mist cool.
pub fn vault_atmosphere() -> Atmosphere {
    Atmosphere {
        background: DARK,
        fog_near: 9.0,
        fog_far: 32.0,
        hemi_sky: lin(0x8a6440) * 0.26,
        hemi_ground: lin(0x4a3422) * 0.16,
        ambient: lin(0x2e2924) * 0.75,
        sun_dir: Vec3::new(0.35, 1.0, 0.25).normalize(),
        sun: lin(0xd8c8a8) * 0.3,
        exposure: 1.35,
        night_sky: 0.0,
        bloom: 1.05,
        moon_view: Vec3::new(0.0, 0.3, -1.0).normalize(),
        cloud_tint: lin(DARK),
    }
}

/// The vaults' ground haze (density at the floor, falloff): a low cool mist in the far dark.
pub const VAULT_HAZE: (f32, f32) = (0.032, 1.0);

/// What lights a lamp: candles, a brazier's fire, a fire bowl on a column.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lamp {
    Candles,
    Brazier,
    Bowl,
}

/// What the view keeps of the vault: where the gold lies and what it is, each piece's static boxes
/// (a chest's lid) to hide when it is taken, how each chest is turned and how long it has stood
/// open, the lamps, the two portals, and the models.
pub struct VaultView {
    pub loot: Vec<(P, LootKind)>,
    ranges: Vec<(usize, usize)>,
    hidden: Vec<bool>,
    yaws: Vec<f32>,
    pub opened: Vec<f32>,
    pub lamps: Vec<(Vec3, Lamp)>,
    pub gates: [Gate; 2],
    pub idol: Vec3,
    art: VaultArt,
}

/// The vault's models, merged into boxes: the heaps, the chest and its lid, the statues and
/// urn, the paintings.
pub(super) struct VaultArt {
    pub pile: Merged,
    pub treasure: Merged,
    pub chest: Merged,
    pub lid: Merged,
    pub statues: Vec<Merged>,
    pub paintings: Vec<Merged>,
}

impl VaultArt {
    fn new() -> Self {
        let (chest, lid) = chest();
        Self {
            pile: pile(),
            treasure: vault_decor::treasure(),
            chest,
            lid,
            statues: vault_decor::statues(),
            paintings: vault_decor::paintings(),
        }
    }
}

/// A chest's voxel, and its body's height and depth in voxels (the lid hinges at the back of the top).
const CHEST_VOXEL: f32 = 0.055;
const CHEST_H: f32 = 6.0;
const CHEST_BACK: f32 = 3.5;

/// A chest's lid as it stands `angle` open, on the chest at `m`.
pub(super) fn lid_matrix(m: Mat4, angle: f32) -> Mat4 {
    m * Mat4::from_translation(Vec3::new(0.0, CHEST_H * CHEST_VOXEL, -CHEST_BACK * CHEST_VOXEL)) * Mat4::from_rotation_x(angle)
}

fn cube(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot_y: f32, color: Vec3, shade: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(rot_y), pos), color, 1.0, Vec3::ZERO, shade));
}

// ------------------------------------------------------------------ meshes

/// The coins of a heap, as painted (`SHADE_COINS` draws them).
pub(super) const COIN: u32 = 0xf2cc78;

/// A heap of gold coins: solid, so it merges into a few dozen boxes, its coins drawn by the shader.
pub(super) fn pile() -> Merged {
    let c = COIN | mat::GOLD;
    let mut s = Sculpt::new(11, 6, 11);
    s.ellipsoid((5.0, 0.3, 5.0), (5.4, 4.6, 4.8), 0, c);
    s.ellipsoid((3.2, 0.8, 6.6), (3.0, 2.8, 2.6), 0, c);
    s.ellipsoid((7.4, 1.2, 3.6), (2.6, 3.0, 2.6), 0, c);
    let mut m = vault_decor::merge_solid(&s.finish_solid(0.0), 0.06);
    vault_decor::reshade(&mut m, COIN, SHADE_COINS);
    m
}

/// A treasure chest facing +z: the body, and the lid (its origin on its back bottom edge, the hinge).
/// Dark oak in iron bands, gilt corners and lock plate; gold coins to the brim inside.
pub(super) fn chest() -> (Merged, Merged) {
    let (wood, wood_l) = (art::OAK | mat::WOOD, art::OAK_D | mat::WOOD);
    let (iron, gilt) = (art::IRON_L | mat::METAL, art::GOLD | mat::GOLD);
    let mut b = Sculpt::new(12, 6, 8);
    b.cuboid(0, 0, 0, 11, 5, 7, wood);
    b.paint(|x, y, z, c| {
        let band = x == 2 || x == 9;
        let corner = (x == 0 || x == 11) && (z == 0 || z == 7) && (y == 0 || y == 4);
        let edge = y == 0 || z == 0 || z == 7 || x == 0 || x == 11;
        Some(if corner {
            gilt
        } else if band {
            iron
        } else if edge && (y + z) % 2 == 0 {
            wood_l
        } else {
            c
        })
    });
    // Inside: gold to the brim, seen when the lid opens.
    for z in 1..7 {
        for x in 1..11 {
            b.set(x, 5, z, COIN | mat::GOLD);
        }
    }
    let mut l = Sculpt::new(12, 4, 8);
    for y in 0..4 {
        l.cuboid(0, y, y.min(3), 11, y, 7 - y.min(3), wood);
    }
    l.paint(|x, y, _, c| {
        Some(if x == 2 || x == 9 {
            iron
        } else if y == 3 {
            wood_l
        } else {
            c
        })
    });
    // The lock plate on the front, and gilt caps on the lid's corners.
    l.cuboid(5, 0, 7, 6, 1, 7, gilt);
    for x in [0, 11] {
        l.set(x, 0, 7, gilt);
        l.set(x, 0, 0, gilt);
    }
    let mut body = vault_decor::merge(&b.finish(0.0), CHEST_VOXEL);
    let mut lid = vault_decor::merge(&l.finish(0.0), CHEST_VOXEL);
    for m in [&mut body, &mut lid] {
        vault_decor::reshade(m, art::GOLD, SHADE_GILT);
        vault_decor::reshade(m, COIN, SHADE_COINS);
    }
    for b in lid.boxes.iter_mut() {
        b.center.z += CHEST_BACK * CHEST_VOXEL;
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
            carpet.iter().find(|(c, _)| c.contains(p)).map(
                |(c, along)| {
                    if *along {
                        p.y == (c.y0 + c.y1) / 2
                    } else {
                        p.x == (c.x0 + c.x1) / 2
                    }
                },
            )
        };
        let mut lamps: Vec<(Vec3, Lamp)> = Vec::new();
        let built = self.static_cubes.len();

        // ---- floors: the joints' dark under each room and corridor in one piece, then the flags
        for r in v.rooms.iter().map(|r| r.rect).chain(v.corridors.iter().copied()) {
            let (w, h) = ((r.x1 - r.x0 + 1) as f32, (r.y1 - r.y0 + 1) as f32);
            let c = Vec3::new((r.x0 + r.x1) as f32 / 2.0, -0.075, (r.y0 + r.y1) as f32 / 2.0);
            cube(&mut self.static_cubes, c, Vec3::new(w, 0.13, h), 0.0, lin(art::GROUT), SHADE_LIT);
        }
        for (x, y, t) in grid.cells() {
            if t == Tile::Rock {
                continue;
            }
            let p = P::new(x, y);
            let (xf, yf) = (x as f32, y as f32);
            let (hx, hy) = (x as f64, y as f64);
            let h = hash(hx * 1.7 + 3.0, hy * 2.3);
            let out = &mut self.static_cubes;
            let lift = (hash(hy * 1.9, hx) as f32 - 0.5) * 0.01;
            // Two worn flags to a tile, turned tile by tile (one block, its material drawing the
            // joint), in charcoal and umber, or pale where they border a room.
            let flags = |out: &mut Vec<Instance>, along_x: bool, pale: bool| {
                let hk = hash(hx * 2.1, hy * 1.7 + 9.0);
                let turn = if along_x { PI / 2.0 } else { 0.0 } + (hk as f32 - 0.5) * 0.02;
                let colour = if pale {
                    jit(art::PALE[(hk * 2.0) as usize % 2], hk, 0.06) * 0.72
                } else {
                    jit(art::FLAG[(hk * 3.0) as usize % 3], hk, 0.12)
                };
                cube(out, Vec3::new(xf, -0.062 + lift, yf), Vec3::new(0.965, 0.12, 0.965), turn, colour, hell_art::SHADE_PAIRED);
            };
            match room_of(p) {
                Some(r) => {
                    let rect = r.rect;
                    let on_edge = x == rect.x0 || x == rect.x1 || y == rect.y0 || y == rect.y1;
                    if r.kind == RoomKind::Hall && !on_edge {
                        // The treasury's checker of black and cream marble, polished.
                        let black = (x + y) % 2 == 0;
                        let c = if black { jit(art::MARBLE_BLACK, h, 0.1) } else { jit(art::MARBLE_CREAM, h, 0.06) };
                        cube(out, Vec3::new(xf, -0.06, yf), Vec3::new(0.97, 0.12, 0.97), 0.0, c, SHADE_MARBLE);
                    } else if on_edge {
                        // A border of pale flags round every room, a gilt line along its inner edge.
                        let along_x = y == rect.y0 || y == rect.y1;
                        flags(out, along_x, true);
                        for d in CARDINALS {
                            let n = p.step(d);
                            let inner = rect.contains(n) && !(n.x == rect.x0 || n.x == rect.x1 || n.y == rect.y0 || n.y == rect.y1);
                            if !inner {
                                continue;
                            }
                            let (dx, dy) = d.vec();
                            let pos = Vec3::new(xf + dx as f32 * 0.47, 0.004, yf + dy as f32 * 0.47);
                            let sc = if dx != 0 { Vec3::new(0.05, 0.016, 0.98) } else { Vec3::new(0.98, 0.016, 0.05) };
                            cube(out, pos, sc, 0.0, gold(h) * 0.8, SHADE_GILT);
                        }
                    } else {
                        flags(out, (x + y) % 2 == 0, false);
                    }
                }
                None => {
                    // A corridor: flags either side of a red runner with gilt borders.
                    let along = carpet.iter().find(|(c, _)| c.contains(p)).map(|(_, a)| *a).unwrap_or(true);
                    flags(out, !along, false);
                    match is_carpet(p) {
                        Some(true) => {
                            let red = if h > 0.5 { 0x5e181c } else { 0x521418 };
                            cube(out, Vec3::new(xf, 0.01, yf), Vec3::new(1.0, 0.02, 1.0), 0.0, jit(red, h, 0.06), crypt::SHADE_CLOTH);
                        }
                        Some(false) => {
                            for d in CARDINALS {
                                let n = p.step(d);
                                if is_carpet(n) == Some(true) {
                                    let (dx, dy) = d.vec();
                                    let pos = Vec3::new(xf + dx as f32 * 0.46, 0.022, yf + dy as f32 * 0.46);
                                    let sc = if dx != 0 { Vec3::new(0.06, 0.012, 1.0) } else { Vec3::new(1.0, 0.012, 0.06) };
                                    cube(out, pos, sc, 0.0, gold(h) * 0.85, SHADE_GILT);
                                }
                            }
                        }
                        None => {}
                    }
                }
            }
        }

        // ---- where the corridors' sconces burn (those faces take no other dressing)
        let mut sconces: Vec<(P, P, Dir)> = Vec::new();
        for (c, along) in &carpet {
            let len = if *along { c.x1 - c.x0 + 1 } else { c.y1 - c.y0 + 1 };
            let mut k = 2;
            while k < len - 1 {
                let (tile, wall, dir) = if *along {
                    (P::new(c.x0 + k, c.y0), P::new(c.x0 + k, c.y0 - 1), Dir::Down)
                } else {
                    (P::new(c.x0, c.y0 + k), P::new(c.x0 - 1, c.y0 + k), Dir::Right)
                };
                if rock(wall.x, wall.y) && !rock(tile.x, tile.y) {
                    sconces.push((tile, wall, dir));
                }
                k += 6;
            }
        }
        let art = VaultArt::new();

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
            let open: Vec<Dir> = CARDINALS
                .iter()
                .copied()
                .filter(|d| {
                    let n = p.step(*d);
                    grid.in_bounds(n.x, n.y) && !rock(n.x, n.y)
                })
                .collect();
            if open.is_empty() {
                continue; // solid rock, out of sight
            }
            let start = self.static_cubes.len();
            if open.len() >= 3 {
                // A column in a room, a fire bowl burning on it.
                self.vault_column(Vec3::new(xf, 0.0, yf), hash(hx, hy));
                lamps.push((self.vault_fire_bowl(Vec3::new(xf, 2.04, yf)), Lamp::Bowl));
            } else {
                // Every six blocks along a wall: a pilaster, a painting or a strongroom door, a
                // pilaster, a niche (a statue or a rack of gold).
                let lit_by_sconce = |d: Dir| sconces.iter().any(|&(_, w, sd)| w == p && sd == d);
                let faces: Vec<(Dir, Face)> = open
                    .iter()
                    .map(|&d| {
                        let (dx, dy) = d.vec();
                        let along = if dx == 0 { x } else { y };
                        let pick = hash(hx * 1.3 + dx as f64 * 7.0, hy * 1.7 + dy as f64 * 5.0);
                        let front = p.step(d);
                        let in_room = room_of(front).is_some();
                        let bare = in_room && !v.loot.iter().any(|l| l.at == front);
                        let face = match along.rem_euclid(6) {
                            _ if lit_by_sconce(d) => Face::Sconce,
                            1 if pick < 0.38 => Face::Door,
                            1 => Face::Painting((pick * 7.3 * art.paintings.len() as f64) as usize % art.paintings.len()),
                            // A niche looks out of a straight wall, and not away from the camera above.
                            4 if open.len() == 1 && d != Dir::Up => {
                                if pick < 0.4 {
                                    Face::Hoard
                                } else {
                                    Face::Niche((pick * 9.1 * art.statues.len() as f64) as usize % art.statues.len())
                                }
                            }
                            3 if in_room && open.len() == 1 => Face::Sconce,
                            0 => Face::Pilaster,
                            // Coin drifted up against the wall where nothing stands before it.
                            _ if bare && pick > 0.62 => Face::Drift,
                            _ => Face::Plain,
                        };
                        (d, face)
                    })
                    .collect();
                for c in self.vault_wall(p, &open, &faces, &art) {
                    lamps.push((c, Lamp::Candles));
                }
            }
            let len = self.static_cubes.len() - start;
            self.rock_index.insert(tile_key(x, y), (start, len));
            // Walls and columns dissolve when they stand between the camera and him, as the woods' trees do.
            let shades: Vec<f32> = self.static_cubes[start..start + len].iter().map(|i| i.emissive[3]).collect();
            self.trees.push(Tree {
                range: (start, len),
                base: Vec3::new(xf, 0.0, yf),
                radius: 0.72,
                height: WALL_H + 0.4,
                fade: 1.0,
                alive: true,
                shades: Some(shades),
            });
        }

        // ---- the solid rock between the halls, capped in dark stone at the walls' height, so the
        // vaults read as solid from above (one box for each run of rock along a row), a dull gilt
        // coffer line every four tiles
        // (A tile of rock at a room's outer corner, touching the floor only across the corner, is
        // capped on its own and fades out of the way of the view of him, as the walls do: the
        // chase camera looks across it into the corner.)
        let faces_floor = |q: P| {
            (-1..=1).any(|dy| {
                (-1..=1).any(|dx| {
                    let n = P::new(q.x + dx, q.y + dy);
                    grid.in_bounds(n.x, n.y) && !rock(n.x, n.y)
                })
            })
        };
        for (x, y, t) in grid.cells() {
            let p = P::new(x, y);
            let cardinal = CARDINALS.iter().any(|d| {
                let n = p.step(*d);
                grid.in_bounds(n.x, n.y) && !rock(n.x, n.y)
            });
            if t != Tile::Rock || cardinal || !faces_floor(p) || p == v.entry_gate || p == v.exit_gate || p == v.idol {
                continue;
            }
            let start = self.static_cubes.len();
            let cap = jit(art::CAP, hash(x as f64, y as f64), 0.08) * 0.8;
            cube(
                &mut self.static_cubes,
                Vec3::new(x as f32, WALL_H - 0.05, y as f32),
                Vec3::new(1.0, 0.1, 1.0),
                0.0,
                cap,
                crypt::SHADE_STONE,
            );
            self.trees.push(Tree {
                range: (start, 1),
                base: Vec3::new(x as f32, 0.0, y as f32),
                radius: 0.72,
                height: WALL_H + 0.4,
                fade: 1.0,
                alive: true,
                shades: Some(vec![crypt::SHADE_STONE]),
            });
        }
        let inlay = lin(art::GOLD_DARK) * 0.35;
        for y in 0..grid.height {
            let mut x = 0;
            while x < grid.width {
                let solid = |x: i32| rock(x, y) && !faces_floor(P::new(x, y));
                if !solid(x) {
                    x += 1;
                    continue;
                }
                let x0 = x;
                while x < grid.width && solid(x) {
                    x += 1;
                }
                let n = (x - x0) as f32;
                let cx = x0 as f32 + n / 2.0 - 0.5;
                let cap = jit(art::CAP, hash(x0 as f64, y as f64), 0.08) * 0.8;
                cube(&mut self.static_cubes, Vec3::new(cx, WALL_H - 0.05, y as f32), Vec3::new(n, 0.1, 1.0), 0.0, cap, crypt::SHADE_STONE);
                if y.rem_euclid(4) == 0 {
                    cube(
                        &mut self.static_cubes,
                        Vec3::new(cx, WALL_H + 0.004, y as f32 - 0.5),
                        Vec3::new(n, 0.012, 0.05),
                        0.0,
                        inlay,
                        SHADE_GILT,
                    );
                }
                for gx in x0..x {
                    if gx.rem_euclid(4) == 0 {
                        cube(
                            &mut self.static_cubes,
                            Vec3::new(gx as f32 - 0.5, WALL_H + 0.004, y as f32),
                            Vec3::new(0.05, 0.012, 1.0),
                            0.0,
                            inlay,
                            SHADE_GILT,
                        );
                    }
                }
            }
        }

        // ---- lamps: candelabra in the rooms' corners, braziers along the great rooms' sides (the
        // sconces on the walls are the walls')
        for r in &v.rooms {
            let rect = r.rect;
            let corners = [(rect.x0 + 1, rect.y0 + 1), (rect.x1 - 1, rect.y0 + 1), (rect.x0 + 1, rect.y1 - 1), (rect.x1 - 1, rect.y1 - 1)];
            for (cx, cy) in corners {
                if rock(cx, cy) || P::new(cx, cy) == v.arrival {
                    continue;
                }
                let base = Vec3::new(cx as f32, 0.0, cy as f32);
                self.vault_candelabrum(base, hash(cx as f64 * 1.1, cy as f64 * 0.7));
                lamps.push((base + Vec3::new(0.0, 1.32, 0.0), Lamp::Candles));
            }
            // (Nothing stands where he arrives, nor before either portal.)
            let by_gate = |q: P| [v.arrival, v.entry_gate, v.exit_gate].iter().any(|g| (g.x - q.x).abs() + (g.y - q.y).abs() <= 2);
            let free = |cx: i32, cy: i32| !rock(cx, cy) && !by_gate(P::new(cx, cy)) && !v.loot.iter().any(|l| l.at == P::new(cx, cy));
            let (w, h) = (rect.x1 - rect.x0 + 1, rect.y1 - rect.y0 + 1);
            let mut sides = Vec::new();
            if w >= 16 {
                let cx = (rect.x0 + rect.x1) / 2;
                sides.extend([(cx, rect.y0 + 1), (cx, rect.y1 - 1)]);
            }
            if h >= 16 {
                let cy = (rect.y0 + rect.y1) / 2;
                sides.extend([(rect.x0 + 1, cy), (rect.x1 - 1, cy)]);
            }
            for (cx, cy) in sides.into_iter().filter(|&(cx, cy)| free(cx, cy)) {
                let base = Vec3::new(cx as f32, 0.0, cy as f32);
                self.vault_brazier(base);
                lamps.push((base + Vec3::new(0.0, 0.88, 0.0), Lamp::Brazier));
            }
        }

        // ---- the gold, where it lies: static boxes, each piece's range hidden once it is taken
        let mut ranges = vec![(0, 0); v.loot.len()];
        let mut yaws = vec![0.0; v.loot.len()];
        for (i, l) in v.loot.iter().enumerate() {
            let base = Vec3::new(l.at.x as f32, 0.0, l.at.y as f32);
            let seed = (l.at.x * 7 + l.at.y * 13) as f32;
            let start = self.static_cubes.len();
            let out = &mut self.static_cubes;
            match l.kind {
                LootKind::Coin => coin_stack(out, Mat4::from_translation(base) * Mat4::from_rotation_y(seed), seed),
                LootKind::Pile => {
                    push_merged(out, &art.pile, Mat4::from_translation(base) * Mat4::from_rotation_y(seed * 0.7), GOLD_GLOW);
                    // Coins slid down off it onto the floor.
                    for k in 0..3 {
                        let a = seed * 1.3 + k as f32 * 2.1;
                        let m = Mat4::from_translation(base + Vec3::new(a.cos() * 0.4, 0.0, a.sin() * 0.4)) * Mat4::from_rotation_y(a);
                        coin_stack(out, m * Mat4::from_scale(Vec3::new(1.0, 0.6, 1.0)), seed + k as f32 * 7.0);
                    }
                }
                LootKind::Treasure => {
                    push_merged(out, &art.treasure, Mat4::from_translation(base) * Mat4::from_rotation_y(seed * 0.3), GOLD_GLOW)
                }
                LootKind::Chest => {
                    // It faces away from the wall it stands against.
                    let wall = CARDINALS.iter().copied().find(|d| {
                        let n = l.at.step(*d);
                        rock(n.x, n.y)
                    });
                    let yaw = wall.map(|d| yaw_of(d.opposite())).unwrap_or(0.0) + (seed * 0.37).sin() * 0.06;
                    yaws[i] = yaw;
                    let m = Mat4::from_translation(base) * Mat4::from_rotation_y(yaw);
                    push_merged(out, &art.chest, m, 0.0);
                    let lid = out.len();
                    push_merged(out, &art.lid, lid_matrix(m, 0.0), 0.0);
                    ranges[i] = (lid, out.len() - lid);
                    continue;
                }
                LootKind::Gem(_) => {}
            }
            ranges[i] = (start, self.static_cubes.len() - start);
        }

        // ---- Mammon's idol in the treasury, and the portals
        let idol = Vec3::new(v.idol.x as f32, 0.0, v.idol.y as f32);
        self.build_idol(idol, hall);
        let gates = [
            Gate { tile: v.entry_gate, facing: Dir::Right, sin: Sin::Greed },
            Gate { tile: v.exit_gate, facing: Dir::Left, sin: Sin::Greed },
        ];
        // (Everything above is built in its own materials and colours: the grade leaves it be.)
        self.authored.push((built, self.static_cubes.len() - built));
        for g in &gates {
            self.build_portal_arch(g);
        }
        // The floor is walkable under the arch when the doors open: a dark threshold behind it.
        for g in &gates {
            let (dx, dy) = g.facing.vec();
            self.static_cubes.push(Instance::new(
                Mat4::from_scale_rotation_translation(
                    Vec3::new(1.0, 0.1, 1.0),
                    Quat::IDENTITY,
                    Vec3::new(g.tile.x as f32 - dx as f32 * 0.5, -0.06, g.tile.y as f32 - dy as f32 * 0.5),
                ),
                lin(art::MARBLE_BLACK),
                1.0,
                Vec3::ZERO,
                SHADE_LIT,
            ));
        }
        self.vault = Some(VaultView {
            loot: v.loot.iter().map(|l| (l.at, l.kind)).collect(),
            hidden: vec![false; v.loot.len()],
            ranges,
            yaws,
            opened: vec![0.0; v.loot.len()],
            lamps,
            gates,
            idol,
            art,
        });
    }

    /// Hides a range of the static scene (a piece of gold he has taken), patching just those boxes.
    pub(super) fn hide_static(&mut self, (start, len): (usize, usize)) {
        if len == 0 {
            return;
        }
        let hidden = (Mat4::from_translation(Vec3::new(0.0, -50.0, 0.0)) * Mat4::from_scale(Vec3::ZERO)).to_cols_array();
        for inst in &mut self.static_cubes[start..start + len] {
            inst.m = hidden;
        }
        self.static_patches.push((start, len));
    }

    /// Each frame: the gold he has taken leaves the static scene, and opened chests count their time.
    pub(super) fn update_vault_view(&mut self, s: &SceneState, dt: f32) {
        let mut hide = Vec::new();
        if let Some(vv) = &mut self.vault {
            for (i, l) in s.loot.iter().enumerate() {
                if !l.taken || i >= vv.hidden.len() {
                    continue;
                }
                vv.opened[i] += dt;
                if !vv.hidden[i] {
                    vv.hidden[i] = true;
                    hide.push(vv.ranges[i]);
                }
            }
        }
        for r in hide {
            self.hide_static(r);
        }
    }

    /// Mammon on his plinth: a squat gold figure with horns and a grin, hugging a chest of gold.
    /// The plinth is black marble on a pale stepped base, banded in gilt.
    fn build_idol(&mut self, base: Vec3, _hall: Rect) {
        let out = &mut self.static_cubes;
        let g = lin(art::GOLD) * 0.95;
        let gd = lin(art::GOLD_DARK);
        let pale = lin(art::PALE[0]) * 0.75;
        cube(out, base + Vec3::new(0.0, 0.05, 0.0), Vec3::new(1.04, 0.1, 1.04), 0.0, pale, crypt::SHADE_STONE);
        cube(out, base + Vec3::new(0.0, 0.2, 0.0), Vec3::new(0.94, 0.22, 0.94), 0.0, lin(art::MARBLE_BLACK), SHADE_MARBLE);
        cube(out, base + Vec3::new(0.0, 0.33, 0.0), Vec3::new(0.98, 0.05, 0.98), 0.0, g, SHADE_GILT);
        let y0 = 0.36;
        // Legs folded under a fat belly.
        cube(out, base + Vec3::new(0.0, y0 + 0.12, 0.05), Vec3::new(0.7, 0.24, 0.6), 0.0, gd, SHADE_GILT);
        cube(out, base + Vec3::new(0.0, y0 + 0.42, -0.02), Vec3::new(0.62, 0.42, 0.5), 0.0, g, SHADE_GILT);
        cube(out, base + Vec3::new(0.0, y0 + 0.5, 0.2), Vec3::new(0.46, 0.34, 0.2), 0.0, g * 1.05, SHADE_GILT);
        // The chest in his arms, brimming.
        cube(out, base + Vec3::new(0.0, y0 + 0.36, 0.3), Vec3::new(0.4, 0.22, 0.24), 0.0, lin(art::OAK), crypt::SHADE_WOOD);
        cube(out, base + Vec3::new(0.0, y0 + 0.49, 0.3), Vec3::new(0.36, 0.05, 0.2), 0.0, lin(COIN), SHADE_COINS);
        for side in [-1.0f32, 1.0] {
            cube(out, base + Vec3::new(side * 0.3, y0 + 0.55, 0.05), Vec3::new(0.16, 0.16, 0.24), side * 0.3, g, SHADE_GILT);
            cube(out, base + Vec3::new(side * 0.26, y0 + 0.42, 0.28), Vec3::new(0.14, 0.14, 0.3), 0.0, g, SHADE_GILT);
        }
        // The head: heavy brows, a wide grin, curled horns.
        cube(out, base + Vec3::new(0.0, y0 + 0.86, 0.0), Vec3::new(0.4, 0.36, 0.38), 0.0, g, SHADE_GILT);
        cube(out, base + Vec3::new(0.0, y0 + 0.78, 0.2), Vec3::new(0.28, 0.04, 0.03), 0.0, lin(art::GROUT), SHADE_LIT);
        for side in [-1.0f32, 1.0] {
            cube(out, base + Vec3::new(side * 0.1, y0 + 0.92, 0.2), Vec3::new(0.08, 0.05, 0.02), 0.0, lin(0xe0203a), SHADE_GEM);
            for k in 0..4 {
                let t = k as f32 / 3.0;
                cube(
                    out,
                    base + Vec3::new(side * (0.2 + 0.12 * t), y0 + 1.02 + 0.2 * t - 0.05 * t * t, -0.05 * t),
                    Vec3::splat(0.1 - 0.02 * t),
                    side * t,
                    gd,
                    SHADE_GILT,
                );
            }
        }
        // A crown of coins.
        for k in 0..6 {
            let a = k as f32 / 6.0 * TAU;
            cube(
                out,
                base + Vec3::new(a.cos() * 0.17, y0 + 1.08, a.sin() * 0.17),
                Vec3::new(0.06, 0.08, 0.02),
                -a,
                lin(art::GOLD_PALE),
                SHADE_GILT,
            );
        }
    }

    /// The gold where it lies (its glints), the practicals' fire and light, the portals.
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
                LootKind::Coin | LootKind::Pile | LootKind::Treasure if !taken => {
                    // Now and then a glint off the gold: a small star that flares and goes (the
                    // gold itself is in the static scene, and catches the lamps).
                    let rate = if *kind == LootKind::Coin { 1.3 } else { 1.9 };
                    let g = ((time * rate + seed * 0.37).sin() * 0.5 + 0.5).powi(24);
                    if g > 0.05 {
                        let lift = if *kind == LootKind::Coin { 0.08 } else { 0.22 + 0.1 * (seed * 1.7).sin().abs() };
                        let off = Vec3::new((seed * 2.3).sin() * 0.12, lift, (seed * 3.1).cos() * 0.12);
                        glint(f, base + off, 0.11 + 0.05 * g, g, time + seed);
                    }
                }
                LootKind::Chest if !taken => lit.push((base + Vec3::new(0.0, 0.5, 0.0), lin(0xffd060) * 1.4)),
                LootKind::Chest => {
                    // Its lid swings up and stays up.
                    let age = vv.opened.get(i).copied().unwrap_or(0.0);
                    let open = (age / LID_SECONDS).min(1.0);
                    let angle = -(open * open * (3.0 - 2.0 * open)) * 1.9;
                    let m = Mat4::from_translation(base) * Mat4::from_rotation_y(vv.yaws[i]);
                    push_merged(&mut f.opaque[cube_i], &vv.art.lid, lid_matrix(m, angle), 0.0);
                    if open < 1.0 {
                        // Gold spilling as the lid goes up.
                        for k in 0..10 {
                            let t = (open * 1.6 + k as f32 * 0.13).fract();
                            let a = k as f32 * 2.4 + seed;
                            let sp = base + Vec3::new(a.cos() * 0.35 * t, 0.35 + t * 0.8 - t * t * 0.9, a.sin() * 0.35 * t);
                            f.additive.push(Instance::new(
                                Mat4::from_translation(sp) * Mat4::from_scale(Vec3::splat(0.04)),
                                lin(GOLD_L),
                                0.8 * (1.0 - t),
                                Vec3::ZERO,
                                SHADE_UNLIT,
                            ));
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
                        let pm = Mat4::from_scale_rotation_translation(
                            Vec3::splat(m.size * 0.8),
                            spin,
                            at + spin * (*off * 0.8) - Vec3::new(0.0, 0.28, 0.0),
                        );
                        f.opaque[cube_i].push(Instance::new(pm, *col * c * 1.4, 1.0, c * 0.35, SHADE_LIT));
                    }
                    glow_orb(f, at, 0.3 + (time * 3.0 + seed).sin() * 0.03, c, 0.25);
                    for k in 0..3 {
                        let a = time * 1.5 + k as f32 * TAU / 3.0 + seed;
                        let sp = at + Vec3::new(a.cos() * 0.26, (a * 1.7).sin() * 0.12, a.sin() * 0.26);
                        f.additive.push(Instance::new(
                            Mat4::from_translation(sp) * Mat4::from_scale(Vec3::splat(0.03)),
                            Vec3::ONE.lerp(c, 0.5),
                            0.8,
                            Vec3::ZERO,
                            SHADE_UNLIT,
                        ));
                    }
                    lit.push((at, c * 2.5));
                }
                _ => {}
            }
        }
        self.emit_drops(s, time, f, &mut lit);
        // The practicals' fire on every lamp in sight; the nearest light the gold and the stone.
        let mut lamp_points: Vec<Vec3> = Vec::new();
        for (i, (l, kind)) in vv.lamps.iter().enumerate() {
            if !near(*l) {
                continue;
            }
            lamp_points.push(*l);
            let fi = i as f32;
            match kind {
                Lamp::Candles => {
                    for (j, dx) in [-0.25f32, 0.0, 0.25].into_iter().enumerate() {
                        let fj = fi + j as f32 * 0.31;
                        let sway = Vec3::new((time * 3.1 + fj * 2.0).sin() * 0.008, 0.0, (time * 2.3 + fj).cos() * 0.008);
                        let tall = 0.065 + 0.012 * (time * 11.0 + fj * 5.0).sin();
                        let p = *l + Vec3::new(dx, 0.04 + tall * 0.5, 0.0) + sway;
                        f.additive.push(Instance::new(
                            Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(0.028, tall, 0.028)),
                            lin(FLAME),
                            0.85,
                            Vec3::ZERO,
                            SHADE_UNLIT,
                        ));
                        f.additive.push(Instance::new(
                            Mat4::from_translation(p + Vec3::Y * 0.01) * Mat4::from_scale(Vec3::new(0.012, tall * 0.55, 0.012)),
                            lin(0xfff0c0),
                            0.9,
                            Vec3::ZERO,
                            SHADE_UNLIT,
                        ));
                    }
                    glow_orb(f, *l + Vec3::new(0.0, 0.07, 0.0), 0.34, lin(FLAME), 0.13 + 0.02 * (time * 9.0 + fi).sin());
                }
                Lamp::Brazier | Lamp::Bowl => {
                    for k in 0..5 {
                        let t = (time * 2.2 + fi * 0.37 + k as f32 * 0.2).fract();
                        let a = fi * 1.9 + k as f32 * 1.3;
                        let p = *l + Vec3::new(a.cos() * 0.09 * (1.0 - t), t * 0.4, a.sin() * 0.09 * (1.0 - t));
                        let size = 0.09 * (1.0 - t) + 0.025;
                        let c = lin(FLAME).lerp(lin(FLAME_TIP), t);
                        f.additive.push(Instance::new(
                            Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(size, size * 1.8, size)),
                            c,
                            0.4 * (1.0 - t),
                            Vec3::ZERO,
                            SHADE_UNLIT,
                        ));
                    }
                    glow_orb(f, *l + Vec3::new(0.0, 0.18, 0.0), 0.5, lin(FIRE), 0.15 + 0.03 * (time * 7.0 + fi).sin());
                }
            }
        }
        for (i, p) in nearest(&lamp_points, hero, VAULT_LAMPS_LIT).into_iter().enumerate() {
            let kind = vv.lamps.iter().find(|(q, _)| *q == p).map(|(_, k)| *k).unwrap_or(Lamp::Candles);
            let fi = i as f32;
            let flick = 1.0 + (time * 7.0 + fi * 1.3).sin() * 0.06 + (time * 17.0 + fi).sin() * 0.04;
            let (c, k, reach, up) = match kind {
                Lamp::Candles => (CANDLE, 5.0, 7.0, 0.25),
                Lamp::Brazier => (FIRE, 6.0, 8.0, 0.7),
                Lamp::Bowl => (FIRE, 7.0, 9.0, 0.6),
            };
            f.lights.push(Light { pos: p + Vec3::new(0.0, up, 0.0), color: lin(c) * k * flick, distance: reach });
        }
        let lit_points: Vec<Vec3> = lit.iter().map(|(p, _)| *p).collect();
        for p in nearest(&lit_points, hero, 3) {
            let c = lit.iter().find(|(q, _)| *q == p).map(|(_, c)| *c).unwrap_or(Vec3::ONE);
            f.lights.push(Light { pos: p, color: c, distance: 3.5 });
        }
        // The idol gleams in its own light.
        glow_orb(f, vv.idol + Vec3::new(0.0, 1.1, 0.0), 0.7, lin(GOLD), 0.06 + 0.02 * (time * 2.0).sin());
        f.lights.push(Light { pos: vv.idol + Vec3::new(0.0, 1.8, 0.9), color: lin(CANDLE) * 4.0, distance: 6.0 });
        // The portals: the one he came through shut behind him, the far one opening for the empty-handed.
        let door = s.gate.map(|g| g.door).unwrap_or(0.0);
        self.emit_portal(&vv.gates[0], 0.0, false, time, hero, f);
        self.emit_portal(&vv.gates[1], door, false, time, hero, f);
    }

    /// The items the chests threw: flying in an arc to where they land, then turning over their
    /// spot in a beam of their rarity's colour, a ring of it on the floor.
    fn emit_drops(&self, s: &SceneState, time: f32, f: &mut Frame, lit: &mut Vec<(Vec3, Vec3)>) {
        let cube_i = MeshKind::Cube as usize;
        for (k, d) in s.drops.iter().enumerate() {
            if d.taken {
                continue;
            }
            let from = Vec3::new(d.from.x as f32, 0.0, d.from.y as f32);
            let to = Vec3::new(d.at.x as f32, 0.0, d.at.y as f32);
            let t = (d.age / DROP_FLIGHT).min(1.0) as f32;
            let landed = t >= 1.0;
            let fk = k as f32 * 1.7;
            let pos = if landed {
                to + Vec3::new(0.0, 0.3 + (time * 2.2 + fk).sin() * 0.04, 0.0)
            } else {
                from.lerp(to, t) + Vec3::new(0.0, 0.45 + 4.0 * t * (1.0 - t) * 0.9, 0.0)
            };
            let color = lin(d.rarity.color());
            let spin = time * 1.6 + fk + if landed { 0.0 } else { t * 9.0 };
            let m = Mat4::from_translation(pos) * Mat4::from_rotation_y(spin);
            let glow = color * 0.25;
            match d.item {
                DropItem::Potion(kind) => {
                    let hex = crate::logic::gear::potion(s.hero_kind, kind).color;
                    if let Some(mesh) = self.models.bottles.get(&hex) {
                        emit_mesh(
                            &mut f.opaque[cube_i],
                            mesh,
                            m * Mat4::from_translation(Vec3::new(0.0, -0.2, 0.0)),
                            Vec3::ONE,
                            1.0,
                            lin(hex) * 0.5,
                        );
                    }
                }
                DropItem::Weapon(w) => {
                    let mesh = match w {
                        RangedKind::Boomerang => &self.models.boomerang,
                        RangedKind::Fireball => &self.models.fireball,
                        RangedKind::Crescent => &self.models.crescent,
                        _ => &self.models.arrow,
                    };
                    emit_mesh(&mut f.opaque[cube_i], mesh, m * Mat4::from_rotation_z(0.5), Vec3::ONE, 1.0, glow);
                }
                DropItem::Boon(b) => {
                    let c = lin(item(b).color);
                    emit_mesh(
                        &mut f.opaque[cube_i],
                        &self.models.diamond,
                        m * Mat4::from_translation(Vec3::new(0.0, -0.2, 0.0)),
                        c * 1.3,
                        1.0,
                        c * 0.4,
                    );
                }
                DropItem::Runes(_) => {
                    let c = lin(0x9ad1ff);
                    emit_mesh(
                        &mut f.opaque[cube_i],
                        &self.models.diamond,
                        m * Mat4::from_translation(Vec3::new(0.0, -0.2, 0.0)) * Mat4::from_scale(Vec3::splat(0.7)),
                        c * 1.3,
                        1.0,
                        c * 0.5,
                    );
                }
                DropItem::Bombs(_) => {
                    emit_mesh(
                        &mut f.opaque[cube_i],
                        &self.models.bomb,
                        m * Mat4::from_translation(Vec3::new(0.0, -0.25, 0.0)),
                        Vec3::ONE,
                        1.0,
                        glow * 0.5,
                    );
                }
            }
            glow_orb(f, pos, 0.32, color, 0.22);
            if landed {
                // The beam, stronger the rarer; the legendary's pulses.
                let strength = [0.05f32, 0.08, 0.12, 0.17][d.rarity as usize]
                    * if d.rarity == Rarity::Legendary { 0.8 + 0.4 * (time * 4.0).sin().abs() } else { 1.0 };
                let beam = Mat4::from_translation(to + Vec3::new(0.0, 2.6, 0.0)) * Mat4::from_scale(Vec3::new(0.07, 5.2, 0.07));
                f.additive.push(Instance::new(beam, color, strength, Vec3::ZERO, SHADE_UNLIT));
                let ring = Mat4::from_translation(to + Vec3::new(0.0, 0.03, 0.0))
                    * Mat4::from_rotation_x(-PI / 2.0)
                    * Mat4::from_scale(Vec3::splat(0.3 + 0.02 * (time * 3.0 + fk).sin()));
                f.transparent[MeshKind::Ring as usize].push(Instance::new(ring, color, 0.55, Vec3::ZERO, SHADE_UNLIT));
            }
            lit.push((pos, color * 2.2));
        }
    }
}

/// A glint off gold at `at`: a small four-pointed star facing the eye, `size` across, flaring
/// with `k` (0 to 1), turning a little as it goes.
pub(super) fn glint(f: &mut Frame, at: Vec3, size: f32, k: f32, turn: f32) {
    let face = f.facing * Quat::from_rotation_z(turn * 0.3);
    for (sx, sy) in [(1.0f32, 0.07f32), (0.07, 1.0)] {
        let m = Mat4::from_scale_rotation_translation(Vec3::new(size * sx, size * sy, 0.004), face, at);
        f.additive.push(Instance::new(m, lin(0xfff2c8), 0.9 * k, Vec3::ZERO, SHADE_UNLIT));
    }
    glow_orb(f, at, size * 0.9, lin(GOLD), 0.2 * k);
}
