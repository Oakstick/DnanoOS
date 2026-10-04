//! The Rotting Feast as the view draws it (Gluttony's realm; laid out in `logic::realms::feast`): a
//! banquet palace that is a gullet, its materials turning at every seam.
//!
//! - The Hall of Courses: cream marble floors with a red runner up the middle, walls of marble over
//!   dark wainscot under a red damask frieze, silver sconces, still lifes in gilt frames; four long
//!   tables in white linen laid with silver domes, candelabra, roasts too glossy to be real.
//! - The Kitchens: terracotta and cream tile slick with grease, white tiled walls with a delft band,
//!   copper pans, butcher's rails of hams and sausages, ovens shaped like mouths, copper cauldrons
//!   over open fires, chopping blocks, and the golden crust floors over the Stomach.
//! - The Larder: salt-crusted flags and walls, shelves of jars and cheeses, salt barrels, hams on
//!   hooks, all turning to wet ribbed flesh toward its north end; ribs for pillars.
//! - The Stomach: a cavity of flesh in rugae folds round glowing acid pools, glands alight in its
//!   walls, half-digested armour in its folds, and at its heart a single place set under a dome.
//!
//! The seams' arches are rings of teeth (carved marble, then copper, then real), which close as he
//! passes; the first rows past each wear the old zone's dressing. The tells of what is coming:
//! red veins in the Hall's marble, the Kitchens' copper sweating and its north tiles pulsing,
//! crumbs rolling north and every flame leaning into the throat's draught. In the Last Course the
//! walls close in, acid floods the rim, the swarm hunts him as a cloud and food flies at him.

mod art;

use super::super::vault_decor::{self, push_merged, Merged, CANVAS, CANVAS_PX};
use super::super::*;
use crate::logic::realms::feast::{Feast, FoodKind, Zone, ACID_BAND, MORSEL_SPEED, SWARM_RADIUS};
use art::Fang;

/// How brightly his lantern burns here.
pub(super) const LANTERN: f32 = 0.55;

/// Each zone's walls: high in the Hall, about a fifth lower in each zone after.
const WALL_H: [f32; 4] = [2.0, 1.6, 1.3, 1.05];
/// What moves and glows is drawn only this near him (tiles).
const NEAR: f32 = 22.0;
/// Shades by voxel material (5 and its kind; see `voxel_surface`).
const METAL: f32 = 6.0;
const CLOTH: f32 = 8.0;
const SKIN: f32 = 10.0;
const IVORY: f32 = 11.0;
const WOOD: f32 = 12.0;
const GOLD: f32 = 13.0;
const SLIME: f32 = 14.0;
const MARBLE: f32 = 15.0;

mod pal {
    pub const C_MARBLE: u32 = 0xaca290;
    pub const MARBLE_L: u32 = 0xb8ae9c;
    pub const MARBLE_D: u32 = 0x988e7c;
    pub const VEIN: u32 = 0xa8303a;
    pub const VEIN_D: u32 = 0x7a1a24;
    pub const WAINSCOT: u32 = 0x4a2c1a;
    pub const DAMASK: u32 = 0x7a1622;
    pub const DAMASK_L: u32 = 0x9a2a30;
    pub const C_GOLD: u32 = 0xe8b840;
    pub const SILVER: u32 = 0xd8dce4;
    pub const SILVER_D: u32 = 0x9aa0aa;
    pub const LINEN: u32 = 0xe6e0d4;
    pub const LINEN_D: u32 = 0xdcd6ca;
    pub const CARPET: u32 = 0x8a1a22;
    pub const CARPET_D: u32 = 0x6a1218;
    pub const CANDLE: u32 = 0xf4ecd8;
    pub const FLAME: u32 = 0xffb050;
    pub const FLAME_TIP: u32 = 0xff6a20;
    pub const TERRACOTTA: u32 = 0x6e4838;
    pub const CREAM: u32 = 0xb8a888;
    pub const GROUT: u32 = 0x3a2a20;
    pub const GREASE: u32 = 0x3a2a12;
    pub const TILE_W: u32 = 0xc8c4b8;
    pub const TILE_W2: u32 = 0xb8b4a8;
    pub const DELFT: u32 = 0x3a5aa8;
    pub const TILE_RED: u32 = 0x8a2a1e;
    pub const SOOT: u32 = 0x221a16;
    pub const IRON: u32 = 0x3a3c42;
    pub const STONE: u32 = 0x8a8680;
    pub const STONE_D: u32 = 0x5e5a54;
    pub const SALT: u32 = 0xeeeae2;
    pub const PLANK: u32 = 0x6a4428;
    pub const JAR: u32 = 0x9ab0a0;
    pub const FLESH: u32 = 0xa83a44;
    pub const FLESH_L: u32 = 0xd06a70;
    pub const FLESH_D: u32 = 0x6a1a26;
    pub const FLESH_DD: u32 = 0x3e0e16;
    pub const RIB: u32 = 0xeae0c8;
    pub const GLAND: u32 = 0xd0bc50;
    pub const ACID: u32 = 0xa89a3c;
    pub const ACID_D: u32 = 0x6e6626;
    pub const ACID_HOT: u32 = 0xffa040;
    pub const CRUST: u32 = 0xd8a04a;
    pub const CRUST_L: u32 = 0xf0c870;
    pub const CRUST_D: u32 = 0x8a5a20;
    pub const CRUMB: u32 = 0xc89048;
    pub const FLY: u32 = 0x141010;
}
use pal::*;

/// The turn that faces a model built facing +z toward `d`.
fn yaw_to(d: Dir) -> f32 {
    let (dx, dz) = d.vec();
    (dx as f32).atan2(dz as f32)
}

fn cube(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot_y: f32, color: Vec3, shade: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(rot_y), pos), color, 1.0, Vec3::ZERO, shade));
}

fn glow_cube(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot_y: f32, color: Vec3, glow: f32) {
    out.push(Instance::new(
        Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(rot_y), pos),
        color,
        1.0,
        color * glow,
        SHADE_LIT,
    ));
}

/// A box at `local` under `root`.
fn put(out: &mut Vec<Instance>, root: Mat4, local: Vec3, scale: Vec3, color: Vec3, shade: f32) {
    out.push(Instance::new(root * Mat4::from_scale_rotation_translation(scale, Quat::IDENTITY, local), color, 1.0, Vec3::ZERO, shade));
}

/// A flat octagon (two squares turned) of `r` at height `y`, `t` thick.
fn octagon(out: &mut Vec<Instance>, at: Vec3, r: f32, t: f32, color: Vec3, shade: f32) {
    for a in [0.0, PI / 4.0] {
        cube(out, at, Vec3::new(r * 1.66, t, r * 1.66), a, color, shade);
    }
}

/// The realm's models.
struct Art {
    food: Vec<Merged>,
    cloche: Merged,
    teeth: Vec<Merged>,
    cauldron: Merged,
    oven: Merged,
    pan: Merged,
    block: Merged,
    ham: Merged,
    sausages: Merged,
    chair: Merged,
    barrel: Merged,
    rib: Merged,
    digested: Merged,
    font: Merged,
    still_life: Merged,
}

impl Art {
    fn new() -> Self {
        let paintings = vault_decor::paintings();
        Self {
            food: FoodKind::ALL.iter().map(|&k| art::food(k)).collect(),
            cloche: art::cloche(),
            teeth: (0..3).map(|i| art::tooth([Fang::Marble, Fang::Copper, Fang::Ivory][i], WALL_H[i] + 0.55)).collect(),
            cauldron: art::cauldron(),
            oven: art::oven(),
            pan: art::pan(),
            block: art::chopping_block(),
            ham: art::ham(),
            sausages: art::sausages(),
            chair: art::chair(),
            barrel: art::barrel(),
            rib: art::rib(),
            digested: art::digested(),
            font: art::font(),
            still_life: paintings.into_iter().nth(2).unwrap_or(Merged { boxes: Vec::new() }),
        }
    }

    fn dish(&self, k: FoodKind) -> &Merged {
        &self.food[FoodKind::ALL.iter().position(|&f| f == k).unwrap_or(0)]
    }
}

/// What the view keeps of the realm (boxed, so the realms' views stay small side by side).
pub(in crate::render::view) struct FeastView(Box<Kept>);

impl std::ops::Deref for FeastView {
    type Target = Kept;
    fn deref(&self) -> &Kept {
        &self.0
    }
}

impl std::ops::DerefMut for FeastView {
    fn deref_mut(&mut self) -> &mut Kept {
        &mut self.0
    }
}

/// The static ranges to hide (food eaten, fonts drained, crust broken) and what of them is hidden,
/// the flames, fires, glands, drips and pools that move or glow, the Kitchens' pulsing tiles, the
/// models, and its own clock.
pub(in crate::render::view) struct Kept {
    food: Vec<(usize, usize)>,
    eaten: Vec<bool>,
    fonts: Vec<(usize, usize)>,
    drained: Vec<bool>,
    crust: Vec<(usize, usize)>,
    broken: Vec<bool>,
    candles: Vec<Vec3>,
    fires: Vec<(Vec3, f32)>,
    glands: Vec<(Vec3, f32)>,
    drips: Vec<Vec3>,
    pools: Vec<Vec3>,
    pulses: Vec<Vec3>,
    art: Art,
    time: f32,
}

impl WorldView {
    /// The realm's static geometry.
    pub(super) fn build_feast(&mut self, grid: &Grid, r: &Feast) -> FeastView {
        let mut v = FeastView(Box::new(Kept {
            food: Vec::new(),
            eaten: vec![false; r.food.len()],
            fonts: Vec::new(),
            drained: vec![false; r.fonts.len()],
            crust: Vec::new(),
            broken: vec![false; r.crust.len()],
            candles: Vec::new(),
            fires: Vec::new(),
            glands: Vec::new(),
            drips: Vec::new(),
            pools: Vec::new(),
            pulses: Vec::new(),
            art: Art::new(),
            time: 0.0,
        }));
        // Under it all, the dark between the slabs.
        let (w, h) = (grid.width as f32, grid.height as f32);
        cube(&mut self.static_cubes, Vec3::new(w / 2.0 - 0.5, -0.4, h / 2.0 - 0.5), Vec3::new(w, 0.1, h), 0.0, lin(0x1a0c0a), SHADE_LIT);
        self.feast_floors(grid, r, &mut v);
        self.feast_walls(grid, r, &mut v);
        self.feast_caps(grid, r);
        self.feast_teeth(grid, r, &v);
        self.feast_hall(grid, r, &mut v);
        self.feast_kitchens(r, &mut v);
        self.feast_larder(r, &v);
        self.feast_stomach(grid, r, &mut v);
        self.feast_food(r, &mut v);
        v
    }

    /// The floors: marble, tile, salt turning to flesh, flesh; the crust is apart.
    fn feast_floors(&mut self, grid: &Grid, r: &Feast, v: &mut FeastView) {
        let [hall, kitchens, larder] = r.rooms;
        // (The props standing on rock tiles have the floor under them too.)
        let props: std::collections::HashSet<P> =
            r.tables.iter().chain(&r.hearths).chain(&r.blocks).chain(&r.pillars).chain(&r.barrels).copied().chain([r.dome]).collect();
        for (x, y, t) in grid.cells() {
            if (t == Tile::Rock && !props.contains(&P::new(x, y))) || t == Tile::Pit {
                continue;
            }
            let p = P::new(x, y);
            if r.crust_at(p).is_some() {
                continue;
            }
            let (xf, yf) = (x as f32, y as f32);
            let (hx, hy) = (x as f64, y as f64);
            let h = hash(hx * 1.7 + 3.0, hy * 2.3) as f32;
            let h2 = hash(hx * 5.1, hy * 3.7 + 1.0) as f32;
            let out = &mut self.static_cubes;
            match r.dressing_at(hy) {
                Zone::Hall => {
                    let north = ((hall.y1 - y) as f32 / (hall.y1 - hall.y0) as f32).clamp(0.0, 1.0);
                    let set = [C_MARBLE, C_MARBLE, MARBLE_L, MARBLE_D];
                    let c = lin(set[(h * 4.0) as usize % 4]) * (0.95 + h2 * 0.07);
                    cube(out, Vec3::new(xf, -0.06, yf), Vec3::new(0.97, 0.12, 0.97), 0.0, c, MARBLE);
                    // Red veins in the marble, more of them toward the throat: the tell of flesh to come.
                    let veins = (h2 < 0.04 + north * 0.45) as usize + (h2 < north * 0.25) as usize;
                    for k in 0..veins {
                        // A thin jagged thread of red, three strokes long.
                        let mut at = Vec3::new(xf + (h - 0.5) * 0.6, 0.004, yf + (h2 - 0.5) * 0.6 - k as f32 * 0.2);
                        let mut a = (h * 7.0 + k as f32 * 2.1) % PI;
                        for j in 0..3 {
                            let len = 0.22 + 0.1 * ((j + k) % 2) as f32;
                            let step = Vec3::new(a.cos(), 0.0, -a.sin()) * len;
                            cube(out, at + step * 0.5, Vec3::new(len, 0.01, 0.018), a, lin(if j == 1 { VEIN } else { VEIN_D }), SHADE_LIT);
                            at += step;
                            a += if (j + k) % 2 == 0 { 0.7 } else { -0.6 };
                        }
                    }
                    // The runner up the middle of the hall, gold at its edges.
                    if (19..=21).contains(&x) && y >= hall.y0 {
                        let edge = x != 20;
                        cube(
                            out,
                            Vec3::new(xf, 0.01, yf),
                            Vec3::new(1.0, 0.02, 1.0),
                            0.0,
                            lin(if edge { CARPET_D } else { CARPET }),
                            CLOTH,
                        );
                        if edge {
                            let sx = if x == 19 { -0.42 } else { 0.42 };
                            cube(out, Vec3::new(xf + sx, 0.025, yf), Vec3::new(0.06, 0.012, 1.0), 0.0, lin(C_GOLD), GOLD);
                        }
                    }
                }
                Zone::Kitchens => {
                    // Small tiles, terracotta and cream, the grout dark, the floor slick with grease.
                    cube(out, Vec3::new(xf, -0.07, yf), Vec3::new(1.0, 0.1, 1.0), 0.0, lin(GROUT), SHADE_LIT);
                    for (k, (dx, dz)) in [(-0.25f32, -0.25f32), (0.25, -0.25), (-0.25, 0.25), (0.25, 0.25)].into_iter().enumerate() {
                        // Terracotta, a cream tile set in a diamond now and then.
                        let (qx, qy) = (x * 2 + k as i32 % 2, y * 2 + k as i32 / 2);
                        let cream = (qx + qy).rem_euclid(6) == 0 && (qx - qy).rem_euclid(6) == 0;
                        let n = hash(hx * 2.0 + k as f64, hy * 2.0) as f32;
                        let c = if cream { lin(CREAM) * (0.85 + n * 0.1) } else { lin(TERRACOTTA) * (0.78 + n * 0.3) };
                        cube(out, Vec3::new(xf + dx, -0.02, yf + dz), Vec3::new(0.46, 0.04, 0.46), 0.0, c, 9.0);
                    }
                    if h2 < 0.1 {
                        // A smear of grease.
                        let a = h * PI;
                        cube(
                            out,
                            Vec3::new(xf + (h - 0.5) * 0.4, 0.002, yf + (h2 - 0.05) * 2.0),
                            Vec3::new(0.32, 0.004, 0.2),
                            a,
                            lin(GREASE),
                            9.0,
                        );
                    }
                    // The north end's tiles pulse, as if something breathed under them.
                    if y < kitchens.y0 + 5 && h2 > 0.7 {
                        v.pulses.push(Vec3::new(xf, 0.0, yf));
                    }
                }
                Zone::Larder => {
                    if fleshed(larder, x, y) {
                        // Wet flesh, ribbed across.
                        let c = lin(FLESH).lerp(lin(FLESH_D), blob(x * 3 + 1, y * 3)) * (0.9 + h * 0.15);
                        cube(out, Vec3::new(xf, -0.06, yf), Vec3::new(1.0, 0.12, 1.0), 0.0, c, SKIN);
                        let z = yf + (h - 0.5) * 0.2;
                        cube(out, Vec3::new(xf, 0.0, z), Vec3::new(1.0, 0.04, 0.3), 0.0, lin(FLESH_L) * 0.75, SKIN);
                        if h2 > 0.8 {
                            // A wet sheen.
                            cube(out, Vec3::new(xf + (h - 0.5) * 0.4, 0.004, yf), Vec3::new(0.3, 0.006, 0.2), h * 3.0, lin(FLESH_L), SLIME);
                        }
                    } else {
                        // Stone flags crusted with salt.
                        let c = lin(if h2 > 0.5 { STONE } else { STONE_D }) * (0.9 + h * 0.2);
                        cube(out, Vec3::new(xf, -0.06, yf), Vec3::new(0.96, 0.12, 0.96), 0.0, c, SHADE_LIT);
                        for k in 0..(h2 * 2.6) as i32 {
                            let o = Vec3::new(hash(hx * 3.0 + k as f64, hy) as f32 - 0.5, 0.01, hash(hy * 3.0, hx + k as f64) as f32 - 0.5)
                                * 0.8;
                            cube(
                                out,
                                Vec3::new(xf, 0.0, yf) + o,
                                Vec3::splat(0.04 + 0.02 * h2),
                                h * 3.0 + k as f32,
                                lin(SALT) * 0.8,
                                SHADE_LIT,
                            );
                        }
                    }
                }
                Zone::Stomach => {
                    let c = lin(FLESH).lerp(lin(FLESH_D), blob(x * 2, y * 2 + 7)) * (0.88 + h * 0.12);
                    cube(out, Vec3::new(xf, -0.06, yf), Vec3::new(1.0, 0.12, 1.0), 0.0, c, SKIN);
                    // The rugae: low folds running round the cavity, meeting from tile to tile.
                    let (dx, dz) = (xf - r.stomach.x as f32, yf - r.stomach.y as f32);
                    let d = (dx * dx + dz * dz).sqrt();
                    let a = dz.atan2(dx);
                    let ring = (d / 1.5).round() * 1.5;
                    let wob = (a * 5.0 + ring).sin() * 0.15;
                    let on = Vec3::new(r.stomach.x as f32 + a.cos() * (ring + wob), 0.015, r.stomach.y as f32 + a.sin() * (ring + wob));
                    cube(out, on, Vec3::new(0.34, 0.05, 1.25), -a, lin(FLESH_L) * 0.7, SKIN);
                    if h2 > 0.86 {
                        cube(out, Vec3::new(xf + (h - 0.5) * 0.5, 0.004, yf), Vec3::new(0.26, 0.006, 0.18), h * 3.0, lin(FLESH_L), SLIME);
                    }
                }
            }
        }
        // The pylorus's threshold: flesh under its teeth, walkable when they part.
        let e = r.base.exit.tile;
        cube(&mut self.static_cubes, Vec3::new(e.x as f32, -0.06, e.y as f32), Vec3::new(1.0, 0.12, 1.0), 0.0, lin(FLESH_DD), SLIME);
    }

    /// The walls: each zone's own, the props set in them.
    fn feast_walls(&mut self, grid: &Grid, r: &Feast, v: &mut FeastView) {
        let rock = |x: i32, y: i32| grid.get(x, y) == Tile::Rock;
        let props: std::collections::HashSet<P> = r
            .tables
            .iter()
            .chain(&r.hearths)
            .chain(&r.blocks)
            .chain(&r.pillars)
            .chain(&r.barrels)
            .copied()
            .chain([r.dome, r.base.entry.tile, r.base.exit.tile])
            .chain(r.ovens.iter().map(|o| o.0))
            .collect();
        let larder = r.rooms[2];
        for (x, y, t) in grid.cells() {
            if t != Tile::Rock {
                continue;
            }
            let p = P::new(x, y);
            if props.contains(&p) {
                continue;
            }
            let open: Vec<Dir> = CARDINALS
                .iter()
                .copied()
                .filter(|d| {
                    let n = p.step(*d);
                    grid.in_bounds(n.x, n.y) && !rock(n.x, n.y)
                })
                .collect();
            if open.is_empty() {
                continue;
            }
            let zone = r.dressing_at(y as f64);
            let wall = WALL_H[zone.index()];
            let (xf, yf) = (x as f32, y as f32);
            let h = hash(x as f64 * 3.0, y as f64 * 5.0) as f32;
            let start = self.static_cubes.len();
            match zone {
                Zone::Hall => self.hall_wall(p, &open, h, v),
                Zone::Kitchens => self.kitchen_wall(p, &open, h, v),
                Zone::Larder => {
                    if fleshed(larder, x, y) {
                        self.flesh_wall(p, &open, h, wall, false, v);
                    } else {
                        self.larder_wall(p, &open, h, v);
                    }
                }
                Zone::Stomach => self.flesh_wall(p, &open, h, wall, true, v),
            }
            let len = self.static_cubes.len() - start;
            self.rock_index.insert(tile_key(x, y), (start, len));
            // Walls dissolve when they stand between the camera and him, as the vaults' do.
            let shades: Vec<f32> = self.static_cubes[start..start + len].iter().map(|i| i.emissive[3]).collect();
            self.trees.push(Tree {
                range: (start, len),
                base: Vec3::new(xf, 0.0, yf),
                radius: 0.72,
                height: wall + 0.4,
                fade: 1.0,
                alive: true,
                shades: Some(shades),
            });
        }
    }

    /// The Hall's wall: dark wainscot, cream marble, a red damask frieze, a gilt cornice; along it
    /// silver sconces, still lifes in gilt frames and gold pilasters.
    fn hall_wall(&mut self, p: P, open: &[Dir], h: f32, v: &mut FeastView) {
        let (xf, yf) = (p.x as f32, p.y as f32);
        let wall = WALL_H[0];
        let out = &mut self.static_cubes;
        let c = Vec3::new(xf, 0.0, yf);
        cube(out, c + Vec3::new(0.0, 0.36, 0.0), Vec3::new(1.0, 0.72, 1.0), 0.0, lin(WAINSCOT) * (0.9 + h * 0.15), WOOD);
        cube(out, c + Vec3::new(0.0, 0.74, 0.0), Vec3::new(1.03, 0.05, 1.03), 0.0, lin(C_GOLD), GOLD);
        cube(out, c + Vec3::new(0.0, 1.06, 0.0), Vec3::new(1.0, 0.62, 1.0), 0.0, lin(C_MARBLE) * (0.94 + h * 0.06), MARBLE);
        cube(out, c + Vec3::new(0.0, 1.55, 0.0), Vec3::new(1.02, 0.36, 1.02), 0.0, lin(if h > 0.5 { DAMASK } else { DAMASK_L }), CLOTH);
        cube(out, c + Vec3::new(0.0, wall - 0.1, 0.0), Vec3::new(1.06, 0.08, 1.06), 0.0, lin(C_GOLD), GOLD);
        cube(out, c + Vec3::new(0.0, wall, 0.0), Vec3::new(1.1, 0.12, 1.1), 0.0, lin(MARBLE_D), MARBLE);
        for &d in open {
            if d == Dir::Up {
                continue; // (faces the camera never sees)
            }
            let (dx, dz) = d.vec();
            let n = Vec3::new(dx as f32, 0.0, dz as f32);
            let along = if dx == 0 { p.x } else { p.y };
            let root = Mat4::from_translation(c + n * 0.5) * Mat4::from_rotation_y(yaw_to(d));
            match along.rem_euclid(6) {
                0 => {
                    // A silver sconce, its candle burning.
                    put(out, root, Vec3::new(0.0, 1.0, 0.04), Vec3::new(0.12, 0.3, 0.06), lin(SILVER_D), METAL);
                    put(out, root, Vec3::new(0.0, 1.16, 0.14), Vec3::new(0.16, 0.04, 0.16), lin(SILVER), METAL);
                    put(out, root, Vec3::new(0.0, 1.26, 0.14), Vec3::new(0.05, 0.16, 0.05), lin(CANDLE), SHADE_LIT);
                    v.candles.push(root.transform_point3(Vec3::new(0.0, 1.38, 0.14)));
                }
                3 => {
                    // A still life of the feast in a gilt frame.
                    let (cw, ch) = (CANVAS.0 as f32 * CANVAS_PX, CANVAS.1 as f32 * CANVAS_PX);
                    put(out, root, Vec3::new(0.0, 1.12, 0.02), Vec3::new(cw + 0.12, ch + 0.12, 0.04), lin(C_GOLD), GOLD);
                    push_merged(out, &v.art.still_life, root * Mat4::from_translation(Vec3::new(0.0, 1.12 - ch / 2.0, 0.045)), 0.04);
                }
                1 | 5 => {
                    // A gold pilaster.
                    put(out, root, Vec3::new(0.0, 1.05, 0.03), Vec3::new(0.18, 1.6, 0.08), lin(C_GOLD) * 0.9, GOLD);
                }
                _ => {}
            }
        }
    }

    /// The Kitchens' wall: white glazed tile over a dark red skirting, a delft band, soot above;
    /// copper pans hanging, butcher's rails of hams and sausages.
    fn kitchen_wall(&mut self, p: P, open: &[Dir], h: f32, v: &mut FeastView) {
        let (xf, yf) = (p.x as f32, p.y as f32);
        let wall = WALL_H[1];
        let out = &mut self.static_cubes;
        let c = Vec3::new(xf, 0.0, yf);
        cube(out, c + Vec3::new(0.0, 0.15, 0.0), Vec3::new(1.01, 0.3, 1.01), 0.0, lin(TILE_RED), SLIME);
        for k in 0..3 {
            let y0 = 0.3 + k as f32 * 0.22;
            cube(
                out,
                c + Vec3::new(0.0, y0 + 0.11, 0.0),
                Vec3::new(1.0, 0.21, 1.0),
                0.0,
                lin(if (k + p.x + p.y) % 2 == 0 { TILE_W } else { TILE_W2 }),
                SLIME,
            );
        }
        cube(out, c + Vec3::new(0.0, 1.03, 0.0), Vec3::new(1.012, 0.1, 1.012), 0.0, lin(DELFT), SLIME);
        cube(out, c + Vec3::new(0.0, 1.27, 0.0), Vec3::new(1.0, 0.38, 1.0), 0.0, lin(TILE_W) * (0.85 + h * 0.1), SLIME);
        cube(out, c + Vec3::new(0.0, wall - 0.07, 0.0), Vec3::new(1.04, 0.14, 1.04), 0.0, lin(SOOT), SHADE_LIT);
        for &d in open {
            if d == Dir::Up {
                continue;
            }
            let (dx, dz) = d.vec();
            let n = Vec3::new(dx as f32, 0.0, dz as f32);
            let along = if dx == 0 { p.x } else { p.y };
            let root = Mat4::from_translation(c + n * 0.5) * Mat4::from_rotation_y(yaw_to(d));
            // Delft tiles in the band: a blue figure on every other.
            if along % 2 == 0 {
                put(out, root, Vec3::new(0.0, 1.03, 0.012), Vec3::new(0.3, 0.07, 0.01), lin(TILE_W), SLIME);
            }
            match along.rem_euclid(5) {
                1 => {
                    // A copper pan, sweating.
                    push_merged(out, &v.art.pan, root * Mat4::from_translation(Vec3::new(0.0, 0.62, 0.06)), 0.0);
                    v.drips.push(root.transform_point3(Vec3::new(0.0, 0.6, 0.1)));
                }
                3 => {
                    // A butcher's rail: a ham and sausages on hooks.
                    put(out, root, Vec3::new(0.0, 1.38, 0.1), Vec3::new(0.9, 0.04, 0.04), lin(IRON), METAL);
                    push_merged(out, &v.art.ham, root * Mat4::from_translation(Vec3::new(-0.22, 0.38, 0.18)), 0.0);
                    push_merged(out, &v.art.sausages, root * Mat4::from_translation(Vec3::new(0.22, 0.38, 0.12)), 0.0);
                }
                _ => {}
            }
        }
    }

    /// The Larder's wall: stone crusted with salt, shelves of jars and cheeses, hams on hooks.
    fn larder_wall(&mut self, p: P, open: &[Dir], h: f32, v: &FeastView) {
        let (xf, yf) = (p.x as f32, p.y as f32);
        let wall = WALL_H[2];
        let out = &mut self.static_cubes;
        let c = Vec3::new(xf, 0.0, yf);
        cube(out, c + Vec3::new(0.0, wall / 2.0, 0.0), Vec3::new(1.0, wall, 1.0), 0.0, lin(STONE) * (0.85 + h * 0.2), MARBLE);
        cube(out, c + Vec3::new(0.0, wall, 0.0), Vec3::new(1.06, 0.08, 1.06), 0.0, lin(STONE_D), MARBLE);
        for &d in open {
            let (dx, dz) = d.vec();
            let n = Vec3::new(dx as f32, 0.0, dz as f32);
            let side = Vec3::new(dz as f32, 0.0, dx as f32).abs();
            // Salt crusting the foot of the wall and its top.
            for k in 0..4 {
                let s = hash(xf as f64 * 7.0 + k as f64, yf as f64 * 3.0) as f32;
                let at = c + n * 0.5 + side * (s - 0.5) * 0.9 + Vec3::new(0.0, if k < 2 { 0.06 } else { wall - 0.05 }, 0.0);
                cube(out, at, Vec3::splat(0.1 + s * 0.08), s * 3.0, lin(SALT), IVORY);
            }
            if d == Dir::Up {
                continue;
            }
            let along = if dx == 0 { p.x } else { p.y };
            let root = Mat4::from_translation(c + n * 0.5) * Mat4::from_rotation_y(yaw_to(d));
            if along.rem_euclid(3) == 1 {
                // Hams and sausages hung from the top of the wall, out over the floor: the meat
                // that will be the ceiling further on.
                put(out, root, Vec3::new(0.0, wall - 0.06, 0.2), Vec3::new(0.9, 0.05, 0.4), lin(PLANK), WOOD);
                push_merged(out, &v.art.ham, root * Mat4::from_translation(Vec3::new(-0.2, wall - 1.06, 0.34)), 0.0);
                push_merged(out, &v.art.sausages, root * Mat4::from_translation(Vec3::new(0.22, wall - 1.06, 0.3)), 0.0);
            }
            if along.rem_euclid(3) == 0 {
                // A shelf of jars and cheeses.
                for (k, y) in [0.55f32, 0.95].into_iter().enumerate() {
                    put(out, root, Vec3::new(0.0, y, 0.12), Vec3::new(0.94, 0.04, 0.24), lin(PLANK), WOOD);
                    for j in 0..3 {
                        let x = -0.3 + j as f32 * 0.3;
                        let cheese = (j + k + along as usize).is_multiple_of(3);
                        if cheese {
                            put(out, root, Vec3::new(x, y + 0.07, 0.12), Vec3::new(0.18, 0.1, 0.18), lin(0xe8b838), SHADE_LIT);
                        } else {
                            put(out, root, Vec3::new(x, y + 0.1, 0.12), Vec3::new(0.11, 0.16, 0.11), lin(JAR), SLIME);
                            put(out, root, Vec3::new(x, y + 0.19, 0.12), Vec3::new(0.12, 0.03, 0.12), lin(PLANK), WOOD);
                        }
                    }
                }
            }
        }
    }

    /// Flesh for a wall: bulging folds, wet and dark, a rib or a tooth in it, a gland alight;
    /// in the Stomach, the armour it has not finished digesting.
    fn flesh_wall(&mut self, p: P, open: &[Dir], h: f32, wall: f32, stomach: bool, v: &mut FeastView) {
        let (xf, yf) = (p.x as f32, p.y as f32);
        let c = Vec3::new(xf, 0.0, yf);
        let out = &mut self.static_cubes;
        cube(out, c + Vec3::new(0.0, wall / 2.0, 0.0), Vec3::new(1.0, wall, 1.0), 0.0, lin(FLESH_D) * (0.8 + h * 0.2), SKIN);
        cube(out, c + Vec3::new(0.0, wall + 0.02, 0.0), Vec3::new(1.06, 0.08, 1.06), 0.0, lin(FLESH_DD), SKIN);
        for &d in open {
            // Folds heaped against the face, one over another.
            let (dx, dz) = d.vec();
            let n = Vec3::new(dx as f32, 0.0, dz as f32);
            let across = Vec3::new(dz as f32, 0.0, dx as f32).abs();
            for k in 0..3 {
                let s = hash(xf as f64 + k as f64 * 3.1 + dx as f64, yf as f64 * 1.7 + dz as f64) as f32;
                let y = wall * (0.18 + k as f32 * 0.3);
                let size = across * (0.7 + s * 0.35) + n.abs() * 0.24 + Vec3::new(0.0, wall * 0.26, 0.0);
                let col = lin(if k == 1 { FLESH } else { FLESH_L }).lerp(lin(FLESH_D), s * 0.6);
                cube(out, c + n * 0.52 + across * (s - 0.5) * 0.2 + Vec3::new(0.0, y, 0.0), size, (s - 0.5) * 0.25, col, SKIN);
            }
            if h > 0.7 {
                cube(
                    out,
                    c + n * 0.66 + Vec3::new(0.0, wall * 0.5, 0.0),
                    across * 0.3 + n.abs() * 0.04 + Vec3::new(0.0, wall * 0.5, 0.0),
                    0.0,
                    lin(FLESH_L),
                    SLIME,
                );
            }
        }
        for &d in open {
            let (dx, dz) = d.vec();
            let n = Vec3::new(dx as f32, 0.0, dz as f32);
            let along = if dx == 0 { p.x } else { p.y };
            let face = c + n * 0.52;
            let pick = hash(xf as f64 * 2.3 + dx as f64, yf as f64 * 1.9 + dz as f64) as f32;
            if pick < 0.3 {
                // A gland, alight.
                let at = face + Vec3::new(0.0, wall * (0.35 + pick), 0.0);
                glow_cube(out, at, Vec3::new(0.24, 0.2, 0.24), pick * 4.0, lin(GLAND), 0.9);
                v.glands.push((at, 0.25 + pick * 0.4));
            } else if pick < 0.45 && !stomach {
                // A rib standing out of it.
                let root = Mat4::from_translation(face - n * 0.1) * Mat4::from_rotation_y(yaw_to(d) + PI);
                push_merged(out, &v.art.rib, root * Mat4::from_scale(Vec3::splat(0.55)), 0.0);
            } else if pick > 0.8 && d != Dir::Up {
                // Teeth, sprouting from the folds.
                for k in 0..3 {
                    let side = Vec3::new(dz as f32, 0.0, dx as f32) * ((k as f32 - 1.0) * 0.28);
                    cube(out, face + side + Vec3::new(0.0, 0.12, 0.0), Vec3::new(0.08, 0.22, 0.08), k as f32, lin(RIB), IVORY);
                }
            }
            if stomach && along.rem_euclid(7) == 0 && d != Dir::Up {
                let root = Mat4::from_translation(face + n * 0.1) * Mat4::from_rotation_y(yaw_to(d) + pick);
                push_merged(out, &v.art.digested, root, 0.0);
            }
        }
    }

    /// The solid rock between the zones, capped at its zone's height so the gullet reads solid.
    fn feast_caps(&mut self, grid: &Grid, r: &Feast) {
        let rock = |x: i32, y: i32| grid.get(x, y) == Tile::Rock;
        let faces = |q: P| {
            CARDINALS.iter().any(|d| {
                let n = q.step(*d);
                grid.in_bounds(n.x, n.y) && !rock(n.x, n.y)
            })
        };
        for y in 0..grid.height {
            let zone = r.dressing_at(y as f64);
            let wall = WALL_H[zone.index()];
            let (cap, line) = match zone {
                Zone::Hall => (0x3a2418, C_GOLD),
                Zone::Kitchens => (0x2a2220, 0x4a3a32),
                Zone::Larder => (0x4a4642, 0x6a1a26),
                Zone::Stomach => (0x3a1218, 0x5a1a24),
            };
            let mut x = 0;
            while x < grid.width {
                let solid = |x: i32| rock(x, y) && !faces(P::new(x, y));
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
                cube(&mut self.static_cubes, Vec3::new(cx, wall - 0.03, y as f32), Vec3::new(n, 0.1, 1.0), 0.0, lin(cap), SHADE_LIT);
                if y.rem_euclid(4) == 0 {
                    cube(
                        &mut self.static_cubes,
                        Vec3::new(cx, wall + 0.025, y as f32 - 0.5),
                        Vec3::new(n, 0.012, 0.05),
                        0.0,
                        lin(line) * 0.8,
                        SHADE_LIT,
                    );
                }
            }
        }
    }

    /// Every seam's arches are rings of teeth: a tooth standing either side of each mouth, on the
    /// old zone's face and on the new one's.
    fn feast_teeth(&mut self, grid: &Grid, r: &Feast, v: &FeastView) {
        let art = &v.art.teeth;
        for (i, s) in r.seams.iter().enumerate() {
            for arch in [&s.narrow, &s.wide] {
                for (mouth, face) in [(&arch.south, Dir::Down), (&arch.north, Dir::Up)] {
                    let row = mouth[0].y;
                    let x0 = mouth.iter().map(|p| p.x).min().unwrap_or(0) - 1;
                    let x1 = mouth.iter().map(|p| p.x).max().unwrap_or(0) + 1;
                    for x in [x0, x1] {
                        if grid.get(x, row) != Tile::Rock {
                            continue;
                        }
                        let (dx, dz) = face.vec();
                        let inward = if x == x0 { 0.18 } else { -0.18 };
                        let at = Vec3::new(x as f32 + inward, 0.0, row as f32 + dz as f32 * 0.32 + dx as f32);
                        let m = Mat4::from_translation(at)
                            * Mat4::from_rotation_y(yaw_to(face) + inward * 2.0)
                            * Mat4::from_scale(Vec3::new(1.5, 1.0, 1.5));
                        push_merged(&mut self.static_cubes, &art[i], m, 0.0);
                    }
                }
            }
        }
    }

    /// The Hall's long tables: linen, a gold runner, silver domes, candelabra, roasts, the places
    /// laid; high-backed chairs drawn up to them.
    fn feast_hall(&mut self, grid: &Grid, r: &Feast, v: &mut FeastView) {
        let food: std::collections::HashSet<P> = r.food.iter().map(|f| f.at).collect();
        for &t in &r.tables {
            let (xf, yf) = (t.x as f32, t.y as f32);
            let c = Vec3::new(xf, 0.0, yf);
            let n_end = !r.tables.contains(&P::new(t.x, t.y - 1));
            let s_end = !r.tables.contains(&P::new(t.x, t.y + 1));
            let h = hash(xf as f64 * 1.3, yf as f64 * 0.7) as f32;
            let out = &mut self.static_cubes;
            cube(out, c + Vec3::new(0.0, 0.36, 0.0), Vec3::new(0.84, 0.72, 0.9), 0.0, lin(WAINSCOT), WOOD);
            cube(out, c + Vec3::new(0.0, 0.76, 0.0), Vec3::new(1.04, 0.04, 1.0), 0.0, lin(LINEN), CLOTH);
            for sx in [-0.52f32, 0.52] {
                cube(out, c + Vec3::new(sx, 0.56, 0.0), Vec3::new(0.03, 0.42, 1.0), 0.0, lin(LINEN_D), CLOTH);
            }
            for (end, sz) in [(n_end, -0.5f32), (s_end, 0.5)] {
                if end {
                    cube(out, c + Vec3::new(0.0, 0.56, sz), Vec3::new(1.04, 0.42, 0.03), 0.0, lin(LINEN_D), CLOTH);
                }
            }
            cube(out, c + Vec3::new(0.0, 0.785, 0.0), Vec3::new(0.24, 0.006, 1.0), 0.0, lin(C_GOLD), GOLD);
            let top = c + Vec3::new(0.0, 0.78, 0.0);
            match t.y.rem_euclid(4) {
                0 => {
                    // A silver candelabrum.
                    octagon(out, top + Vec3::new(0.0, 0.02, 0.0), 0.1, 0.03, lin(SILVER), METAL);
                    cube(out, top + Vec3::new(0.0, 0.22, 0.0), Vec3::new(0.04, 0.4, 0.04), 0.0, lin(SILVER), METAL);
                    cube(out, top + Vec3::new(0.0, 0.42, 0.0), Vec3::new(0.04, 0.03, 0.4), 0.0, lin(SILVER), METAL);
                    for dz in [-0.18f32, 0.0, 0.18] {
                        cube(out, top + Vec3::new(0.0, 0.5, dz), Vec3::new(0.035, 0.12, 0.035), 0.0, lin(CANDLE), SHADE_LIT);
                        v.candles.push(top + Vec3::new(0.0, 0.6, dz));
                    }
                }
                1 => {
                    octagon(out, top + Vec3::new(0.0, 0.01, 0.0), 0.26, 0.02, lin(SILVER_D), METAL);
                    push_merged(
                        out,
                        &v.art.cloche,
                        Mat4::from_translation(top + Vec3::new(0.0, 0.02, 0.0)) * Mat4::from_rotation_y(h * TAU),
                        0.0,
                    );
                }
                2 => {
                    octagon(out, top + Vec3::new(0.0, 0.01, 0.0), 0.28, 0.02, lin(SILVER), METAL);
                    let dish = if h > 0.5 { FoodKind::Roast } else { FoodKind::Ham };
                    push_merged(
                        out,
                        v.art.dish(dish),
                        Mat4::from_translation(top + Vec3::new(0.0, 0.02, 0.0)) * Mat4::from_rotation_y(h * TAU),
                        0.06,
                    );
                }
                _ => {
                    let dish = [FoodKind::Fruit, FoodKind::Pie, FoodKind::Cake, FoodKind::Pudding][(h * 4.0) as usize % 4];
                    octagon(out, top + Vec3::new(0.0, 0.01, 0.0), 0.26, 0.02, lin(SILVER), METAL);
                    push_merged(
                        out,
                        v.art.dish(dish),
                        Mat4::from_translation(top + Vec3::new(0.0, 0.02, 0.0)) * Mat4::from_scale(Vec3::splat(0.8)),
                        0.0,
                    );
                }
            }
            // The places laid either side: a plate, a goblet, a knife and a fork.
            for sx in [-1.0f32, 1.0] {
                let at = top + Vec3::new(sx * 0.33, 0.0, 0.0);
                octagon(out, at + Vec3::new(0.0, 0.01, 0.0), 0.1, 0.012, lin(LINEN), SHADE_LIT);
                octagon(out, at + Vec3::new(0.0, 0.018, 0.0), 0.07, 0.006, lin(C_GOLD), GOLD);
                cube(out, at + Vec3::new(0.0, 0.01, 0.16), Vec3::new(0.012, 0.006, 0.14), 0.0, lin(SILVER), METAL);
                cube(out, at + Vec3::new(0.0, 0.01, -0.16), Vec3::new(0.012, 0.006, 0.14), 0.0, lin(SILVER), METAL);
                let g = at + Vec3::new(-sx * 0.12, 0.0, 0.2);
                cube(out, g + Vec3::new(0.0, 0.05, 0.0), Vec3::new(0.015, 0.1, 0.015), 0.0, lin(SILVER), METAL);
                cube(out, g + Vec3::new(0.0, 0.13, 0.0), Vec3::new(0.06, 0.07, 0.06), 0.0, lin(SILVER), METAL);
                cube(out, g + Vec3::new(0.0, 0.15, 0.0), Vec3::new(0.045, 0.02, 0.045), 0.0, lin(0x6a0a18), SLIME);
            }
            // The chairs drawn up to it, where the floor is free.
            for (sx, d) in [(-1, Dir::Right), (1, Dir::Left)] {
                let q = P::new(t.x + sx, t.y);
                if grid.is_walkable(q.x, q.y, false) && !food.contains(&q) && t.y % 2 == 0 {
                    let m = Mat4::from_translation(Vec3::new(xf + sx as f32 * 0.68, 0.0, yf)) * Mat4::from_rotation_y(yaw_to(d));
                    push_merged(out, &v.art.chair, m, 0.0);
                }
            }
        }
        // Tall silver candelabra in the Hall's corners.
        let hall = r.rooms[0];
        for (x, y) in [(hall.x0, hall.y0), (hall.x1, hall.y0), (hall.x0, hall.y1), (hall.x1, hall.y1)] {
            if !grid.is_walkable(x, y, false) || P::new(x, y) == r.base.arrival {
                continue;
            }
            let base = Vec3::new(x as f32, 0.0, y as f32);
            let out = &mut self.static_cubes;
            octagon(out, base + Vec3::new(0.0, 0.04, 0.0), 0.2, 0.08, lin(SILVER_D), METAL);
            cube(out, base + Vec3::new(0.0, 0.7, 0.0), Vec3::new(0.06, 1.3, 0.06), 0.0, lin(SILVER), METAL);
            cube(out, base + Vec3::new(0.0, 1.36, 0.0), Vec3::new(0.5, 0.04, 0.06), 0.0, lin(SILVER), METAL);
            for dx in [-0.22f32, 0.0, 0.22] {
                cube(out, base + Vec3::new(dx, 1.45, 0.0), Vec3::new(0.05, 0.14, 0.05), 0.0, lin(CANDLE), SHADE_LIT);
                v.candles.push(base + Vec3::new(dx, 1.56, 0.0));
            }
        }
    }

    /// The Kitchens: cauldrons over open fires, chopping blocks, ovens like mouths, the font.
    fn feast_kitchens(&mut self, r: &Feast, v: &mut FeastView) {
        // Each hearth is four tiles; its fire burns in the middle of them.
        let mut done: Vec<P> = Vec::new();
        for &t in &r.hearths {
            if done.iter().any(|d| (d.x - t.x).abs() <= 1 && (d.y - t.y).abs() <= 1) {
                continue;
            }
            done.push(t);
            let c = Vec3::new(t.x as f32 + 0.5, 0.0, t.y as f32 + 0.5);
            let out = &mut self.static_cubes;
            // A raised hearth of sooty brick, the fire in a pit in its middle.
            for (k, (dx, dz)) in [(-0.47f32, 0.0f32), (0.47, 0.0), (0.0, -0.47), (0.0, 0.47)].into_iter().enumerate() {
                let along_x = k >= 2;
                let size = if along_x { Vec3::new(1.86, 0.26, 0.92) } else { Vec3::new(0.92, 0.26, 1.86) };
                cube(out, c + Vec3::new(dx, 0.13, dz), size, 0.0, lin(0x6a3424) * (0.9 + 0.1 * k as f32 * 0.3), SHADE_LIT);
            }
            cube(out, c + Vec3::new(0.0, 0.27, 0.0), Vec3::new(1.9, 0.04, 1.9), 0.0, lin(0x3a2018), SHADE_LIT);
            glow_cube(out, c + Vec3::new(0.0, 0.28, 0.0), Vec3::new(1.1, 0.03, 1.1), 0.0, lin(0xff5a10), 1.2);
            for k in 0..3 {
                let a = k as f32 * PI / 3.0 + 0.3;
                cube(out, c + Vec3::new(0.0, 0.32, 0.0), Vec3::new(1.0, 0.09, 0.11), a, lin(0x3a2414), WOOD);
            }
            push_merged(out, &v.art.cauldron, Mat4::from_translation(c + Vec3::new(0.0, 0.3, 0.0)), 0.0);
            v.fires.push((c + Vec3::new(0.0, 0.34, 0.0), 0.5));
        }
        for &b in &r.blocks {
            let h = hash(b.x as f64, b.y as f64 * 1.3) as f32;
            push_merged(
                &mut self.static_cubes,
                &v.art.block,
                Mat4::from_translation(Vec3::new(b.x as f32, 0.0, b.y as f32)) * Mat4::from_rotation_y(h * TAU),
                0.0,
            );
        }
        for &(at, d) in &r.ovens {
            let (dx, dz) = d.vec();
            let c = Vec3::new(at.x as f32, 0.0, at.y as f32);
            let out = &mut self.static_cubes;
            // The wall it is built into, and the oven proud of it.
            cube(out, c + Vec3::new(0.0, WALL_H[1] / 2.0, 0.0), Vec3::new(1.0, WALL_H[1], 1.0), 0.0, lin(0x6a2e1c), SHADE_LIT);
            let m = Mat4::from_translation(c + Vec3::new(dx as f32, 0.0, dz as f32) * 0.64) * Mat4::from_rotation_y(yaw_to(d));
            push_merged(out, &v.art.oven, m, 0.0);
            v.fires.push((c + Vec3::new(dx as f32, 0.0, dz as f32) * 0.62 + Vec3::new(0.0, 0.4, 0.0), 0.28));
        }
    }

    /// The Larder: rib pillars and salt barrels.
    fn feast_larder(&mut self, r: &Feast, v: &FeastView) {
        let (art_rib, barrel) = (&v.art.rib, &v.art.barrel);
        for &p in &r.pillars {
            let c = Vec3::new(p.x as f32, 0.0, p.y as f32);
            let out = &mut self.static_cubes;
            octagon(out, c + Vec3::new(0.0, 0.1, 0.0), 0.45, 0.2, lin(FLESH_D), SLIME);
            for (k, yaw) in [0.0f32, PI].into_iter().enumerate() {
                let m = Mat4::from_translation(c + Vec3::new(if k == 0 { -0.12 } else { 0.12 }, 0.0, 0.0)) * Mat4::from_rotation_y(yaw);
                push_merged(out, art_rib, m, 0.0);
            }
        }
        for &p in &r.barrels {
            let h = hash(p.x as f64 * 2.0, p.y as f64) as f32;
            push_merged(
                &mut self.static_cubes,
                barrel,
                Mat4::from_translation(Vec3::new(p.x as f32, 0.0, p.y as f32)) * Mat4::from_rotation_y(h * TAU),
                0.0,
            );
        }
    }

    /// The Stomach: its acid pools, the table at its heart, the purge fonts and the crust.
    fn feast_stomach(&mut self, grid: &Grid, r: &Feast, v: &mut FeastView) {
        for &p in &r.pools {
            let c = Vec3::new(p.x as f32, 0.0, p.y as f32);
            let out = &mut self.static_cubes;
            glow_cube(out, c + Vec3::new(0.0, -0.1, 0.0), Vec3::new(1.0, 0.04, 1.0), 0.0, lin(ACID_D), 0.55);
            // A lip of flesh round the pool where it meets the floor.
            for d in CARDINALS {
                let n = p.step(d);
                if grid.is_walkable(n.x, n.y, false) {
                    let (dx, dz) = d.vec();
                    let sc = if dx != 0 { Vec3::new(0.12, 0.1, 1.0) } else { Vec3::new(1.0, 0.1, 0.12) };
                    cube(out, c + Vec3::new(dx as f32 * 0.46, -0.02, dz as f32 * 0.46), sc, 0.0, lin(FLESH_L) * 0.8, SLIME);
                }
            }
            v.pools.push(c + Vec3::new(0.0, -0.06, 0.0));
        }
        // The table at the heart, laid for one, facing the way he comes.
        let c = Vec3::new(r.dome.x as f32, 0.0, r.dome.y as f32);
        let out = &mut self.static_cubes;
        octagon(out, c + Vec3::new(0.0, 0.38, 0.0), 0.5, 0.76, lin(LINEN_D), CLOTH);
        octagon(out, c + Vec3::new(0.0, 0.775, 0.0), 0.56, 0.03, lin(LINEN), CLOTH);
        octagon(out, c + Vec3::new(0.0, 0.05, 0.0), 0.52, 0.04, lin(C_GOLD), GOLD);
        let place = c + Vec3::new(0.0, 0.79, 0.2);
        octagon(out, place, 0.16, 0.015, lin(LINEN), SHADE_LIT);
        octagon(out, place + Vec3::new(0.0, 0.01, 0.0), 0.12, 0.006, lin(C_GOLD), GOLD);
        cube(out, place + Vec3::new(-0.24, 0.0, 0.0), Vec3::new(0.02, 0.008, 0.2), 0.0, lin(SILVER), METAL);
        cube(out, place + Vec3::new(0.24, 0.0, 0.0), Vec3::new(0.02, 0.008, 0.2), 0.0, lin(SILVER), METAL);
        let g = place + Vec3::new(0.2, 0.0, -0.2);
        cube(out, g + Vec3::new(0.0, 0.06, 0.0), Vec3::new(0.02, 0.12, 0.02), 0.0, lin(SILVER), METAL);
        cube(out, g + Vec3::new(0.0, 0.15, 0.0), Vec3::new(0.08, 0.08, 0.08), 0.0, lin(SILVER), METAL);
        for sx in [-0.32f32, 0.32] {
            let at = c + Vec3::new(sx, 0.79, -0.18);
            octagon(out, at, 0.05, 0.02, lin(SILVER), METAL);
            cube(out, at + Vec3::new(0.0, 0.12, 0.0), Vec3::new(0.035, 0.22, 0.035), 0.0, lin(CANDLE), SHADE_LIT);
            v.candles.push(at + Vec3::new(0.0, 0.27, 0.0));
        }
        // The purge fonts.
        for f in &r.fonts {
            let at = Vec3::new(f.at.x as f32, 0.0, f.at.y as f32);
            push_merged(&mut self.static_cubes, &v.art.font, Mat4::from_translation(at + Vec3::new(0.0, -0.02, 0.0)), 0.0);
            let start = self.static_cubes.len();
            octagon(&mut self.static_cubes, at + Vec3::new(0.0, 0.1, 0.06), 0.36, 0.04, lin(ACID), SLIME);
            glow_cube(&mut self.static_cubes, at + Vec3::new(0.0, 0.13, 0.06), Vec3::new(0.3, 0.01, 0.3), 0.6, lin(ACID), 0.8);
            v.fonts.push((start, self.static_cubes.len() - start));
        }
        // The Kitchens' crust floors: golden pastry, latticed, cracked, over the dark.
        for k in &r.crust {
            let c = Vec3::new(k.at.x as f32, 0.0, k.at.y as f32);
            let h = hash(c.x as f64 * 1.9, c.z as f64 * 2.7) as f32;
            let start = self.static_cubes.len();
            let out = &mut self.static_cubes;
            cube(out, c + Vec3::new(0.0, -0.04, 0.0), Vec3::new(0.99, 0.08, 0.99), 0.0, lin(CRUST) * (0.92 + h * 0.12), WOOD);
            for j in 0..2 {
                let o = -0.22 + j as f32 * 0.44;
                cube(out, c + Vec3::new(o, 0.01, 0.0), Vec3::new(0.12, 0.03, 0.99), 0.0, lin(CRUST_L), WOOD);
                cube(out, c + Vec3::new(0.0, 0.02, o), Vec3::new(0.99, 0.03, 0.12), 0.0, lin(CRUST_L) * 0.95, WOOD);
            }
            if h > 0.45 {
                cube(out, c + Vec3::new((h - 0.7) * 0.6, 0.025, 0.1), Vec3::new(0.5, 0.01, 0.025), h * 5.0, lin(CRUST_D), SHADE_LIT);
            }
            v.crust.push((start, self.static_cubes.len() - start));
        }
    }

    /// The food where it lies, on the zone's own stand: a silver gueridon in the Hall, a stool in
    /// the Kitchens, a crate in the Larder, a fold of the floor in the Stomach. The dish is hidden
    /// once eaten; its stand stays, bare.
    fn feast_food(&mut self, r: &Feast, v: &mut FeastView) {
        for food in &r.food {
            let c = Vec3::new(food.at.x as f32, 0.0, food.at.y as f32);
            let h = hash(c.x as f64 * 3.3, c.z as f64 * 1.1) as f32;
            let out = &mut self.static_cubes;
            let top = match r.zone_at(food.at.y as f64) {
                Zone::Hall => {
                    octagon(out, c + Vec3::new(0.0, 0.02, 0.0), 0.16, 0.04, lin(SILVER_D), METAL);
                    cube(out, c + Vec3::new(0.0, 0.24, 0.0), Vec3::new(0.05, 0.44, 0.05), 0.0, lin(SILVER), METAL);
                    octagon(out, c + Vec3::new(0.0, 0.46, 0.0), 0.3, 0.025, lin(SILVER), METAL);
                    0.475
                }
                Zone::Kitchens => {
                    for k in 0..3 {
                        let a = k as f32 / 3.0 * TAU + h;
                        cube(out, c + Vec3::new(a.cos() * 0.18, 0.17, a.sin() * 0.18), Vec3::new(0.05, 0.34, 0.05), a, lin(0x4a2e18), WOOD);
                    }
                    octagon(out, c + Vec3::new(0.0, 0.36, 0.0), 0.27, 0.04, lin(PLANK), WOOD);
                    0.38
                }
                Zone::Larder => {
                    cube(out, c + Vec3::new(0.0, 0.15, 0.0), Vec3::new(0.56, 0.3, 0.5), h * 0.4, lin(PLANK), WOOD);
                    cube(out, c + Vec3::new(0.0, 0.15, 0.0), Vec3::new(0.58, 0.05, 0.52), h * 0.4, lin(0x4a2e18), WOOD);
                    0.3
                }
                Zone::Stomach => {
                    octagon(out, c + Vec3::new(0.0, 0.03, 0.0), 0.3, 0.06, lin(FLESH_L) * 0.7, SLIME);
                    0.06
                }
            };
            let start = out.len();
            let m = Mat4::from_translation(c + Vec3::new(0.0, top, 0.0)) * Mat4::from_rotation_y(h * TAU);
            push_merged(out, v.art.dish(food.kind), m, 0.05);
            v.food.push((start, out.len() - start));
        }
    }

    /// The realm's own animation, one step: food eaten, fonts drained and crust broken leave the scene.
    pub(super) fn update_feast_view(&mut self, v: &mut FeastView, r: &Feast, _s: &SceneState, dt: f32) {
        v.time += dt;
        let mut hide = Vec::new();
        for (i, f) in r.food.iter().enumerate() {
            if f.eaten && !v.eaten.get(i).copied().unwrap_or(true) {
                v.eaten[i] = true;
                hide.push(v.food[i]);
            }
        }
        for (i, f) in r.fonts.iter().enumerate() {
            if f.used && !v.drained.get(i).copied().unwrap_or(true) {
                v.drained[i] = true;
                hide.push(v.fonts[i]);
            }
        }
        for (i, k) in r.crust.iter().enumerate() {
            if k.broken && !v.broken.get(i).copied().unwrap_or(true) {
                v.broken[i] = true;
                hide.push(v.crust[i]);
            }
        }
        for range in hide {
            self.hide_static(range);
        }
    }

    /// What moves, glows and lights in the realm.
    pub(super) fn emit_feast(&self, v: &FeastView, r: &Feast, s: &SceneState, time: f32, hero: Vec3, f: &mut Frame) {
        let near = |p: Vec3| Vec2::new(p.x - hero.x, p.z - hero.z).length_squared() < NEAR * NEAR;
        let hz = r.dressing_at(hero.z as f64);
        let mut lamps: Vec<(Vec3, Vec3, f32)> = Vec::new();
        // Flames lean into the throat's draught (north), more the deeper he is.
        let lean = 0.03 + 0.03 * hz.index() as f32;
        for (i, c) in v.candles.iter().enumerate() {
            if !near(*c) {
                continue;
            }
            flame(f, *c, 0.055, lean, time, i as f32);
            lamps.push((*c + Vec3::new(0.0, 0.5, 0.0), lin(0xffc070) * 1.8, 5.5));
        }
        for (i, (c, size)) in v.fires.iter().enumerate() {
            if !near(*c) {
                continue;
            }
            for k in 0..4 {
                let o = Vec3::new(((i * 7 + k) as f32).sin() * size * 0.4, 0.0, ((i * 3 + k) as f32).cos() * size * 0.4);
                flame(f, *c + o, size * 0.32, lean * 2.0, time * 1.3, i as f32 * 3.0 + k as f32);
            }
            glow_orb(f, *c + Vec3::new(0.0, size * 0.4, 0.0), size * 1.6, lin(0xff7a20), 0.2 + 0.04 * (time * 11.0 + i as f32).sin());
            lamps.push((*c + Vec3::new(0.0, 0.6, 0.0), lin(0xff8a30) * (3.2 + 0.5 * (time * 9.0 + i as f32).sin()), 7.0));
        }
        for (i, (c, size)) in v.glands.iter().enumerate() {
            if !near(*c) {
                continue;
            }
            let pulse = 0.5 + 0.5 * (time * 2.2 + i as f32 * 1.7).sin();
            glow_orb(f, *c, size * (1.4 + pulse * 0.4), lin(GLAND), 0.12 + pulse * 0.12);
            if i % 3 == 0 {
                lamps.push((*c, lin(GLAND) * (1.5 + pulse * 1.5), 4.0));
            }
        }
        for (i, c) in v.pools.iter().enumerate() {
            if !near(*c) {
                continue;
            }
            glow_orb(f, *c + Vec3::new(0.0, 0.2, 0.0), 0.9, lin(ACID), 0.1 + 0.03 * (time * 3.0 + i as f32).sin());
            let t = (time * 0.7 + i as f32 * 0.37).fract();
            let o = Vec3::new(((i * 13) as f32).sin() * 0.3, t * 0.25, ((i * 5) as f32).cos() * 0.3);
            f.additive.push(Instance::new(
                Mat4::from_translation(*c + o) * Mat4::from_scale(Vec3::splat(0.07 * (1.0 - t))),
                lin(ACID),
                0.7,
                Vec3::ZERO,
                SHADE_UNLIT,
            ));
            if i % 4 == 0 {
                lamps.push((*c + Vec3::new(0.0, 0.4, 0.0), lin(ACID) * 2.5, 4.5));
            }
        }
        // The copper sweats: drops gathering and falling.
        for (i, c) in v.drips.iter().enumerate() {
            if !near(*c) {
                continue;
            }
            let t = (time * 0.6 + i as f32 * 0.29).fract();
            let p = *c - Vec3::new(0.0, t * t * 0.6, 0.0);
            f.additive.push(Instance::new(
                Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(0.02, 0.035, 0.02)),
                lin(0xffd8a0),
                0.6,
                Vec3::ZERO,
                SHADE_UNLIT,
            ));
        }
        // The Kitchens' north tiles pulse.
        for (i, c) in v.pulses.iter().enumerate() {
            if !near(*c) {
                continue;
            }
            let pulse = (time * 1.6 + (i % 5) as f32 * 0.4).sin().max(0.0).powi(3);
            if pulse > 0.05 {
                let m = Mat4::from_translation(*c + Vec3::new(0.0, 0.01, 0.0)) * Mat4::from_scale(Vec3::new(0.94, 0.01, 0.94));
                f.transparent[MeshKind::Cube as usize].push(Instance::new(
                    m,
                    lin(0xa01828),
                    0.16 * pulse,
                    lin(0xa01828) * 0.2 * pulse,
                    SHADE_UNLIT,
                ));
            }
        }
        self.feast_crumbs(r, time, hero, f);
        self.feast_flies(r, time, hero, f);
        self.feast_broken_crust(r, time, f);
        self.feast_seam_teeth(r, time, hero, f);
        self.feast_dome(v, r, time, f, &mut lamps);
        self.feast_pylorus(r, time, f);
        if let Some(c) = &r.climax {
            self.feast_last_course(v, r, c, time, hero, f, &mut lamps);
        }
        lamps.sort_by(|a, b| {
            let da = Vec2::new(a.0.x - hero.x, a.0.z - hero.z).length_squared();
            let db = Vec2::new(b.0.x - hero.x, b.0.z - hero.z).length_squared();
            da.total_cmp(&db)
        });
        for (pos, color, distance) in lamps.into_iter().take(7) {
            f.lights.push(Light { pos: pos + Vec3::new(0.0, 0.25, 0.0), color, distance });
        }
        let _ = s;
    }

    /// Crumbs rolling over the floor toward the stomach, as fast as the floor drifts.
    fn feast_crumbs(&self, r: &Feast, time: f32, hero: Vec3, f: &mut Frame) {
        for k in 0..36 {
            let seed = k as f32 * 7.31;
            let span = 9.0;
            let x = hero.x + ((seed * 1.37).sin() * 0.5 + 0.5) * 14.0 - 7.0;
            let base = hero.z + ((seed * 2.11).cos() * 0.5 + 0.5) * span - span * 0.4;
            let rate = r.drift_at(base as f64) as f32;
            if rate <= 0.0 {
                continue;
            }
            let z = base - ((time * rate * 1.6 + seed).rem_euclid(span));
            let (tx, tz) = (x.round() as i32, z.round() as i32);
            if !s_walkable(r, tx, tz) {
                continue;
            }
            let roll = time * rate * 9.0 + seed;
            let m = Mat4::from_translation(Vec3::new(x, 0.025, z))
                * Mat4::from_rotation_x(roll)
                * Mat4::from_scale(Vec3::splat(0.035 + (k % 3) as f32 * 0.01));
            f.opaque[MeshKind::Cube as usize].push(Instance::new(m, lin(CRUMB), 1.0, Vec3::ZERO, SHADE_LIT));
        }
    }

    /// Flies thick in the air of the Larder and the Stomach, and before the Last Course the swarm
    /// drifting through the Stomach as a living darkness.
    fn feast_flies(&self, r: &Feast, time: f32, hero: Vec3, f: &mut Frame) {
        let deep = r.dressing_at(hero.z as f64) >= Zone::Larder;
        if deep {
            for k in 0..40 {
                let s = k as f32 * 3.7;
                let a = time * (2.0 + (s % 1.3)) + s;
                let c = hero + Vec3::new((s * 1.1).sin() * 5.0, 0.0, (s * 0.7).cos() * 4.0);
                let p = c + Vec3::new(a.cos() * 0.5, 0.6 + (a * 1.7).sin() * 0.3 + (s % 1.0), (a * 1.3).sin() * 0.5);
                f.opaque[MeshKind::Cube as usize].push(Instance::new(
                    Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.025)),
                    lin(FLY),
                    1.0,
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
        }
        if r.climax.is_none() && r.in_stomach(crate::logic::types::V::new(hero.x as f64, hero.z as f64)) {
            let c = Vec3::new(r.stomach.x as f32 + (time * 0.13).sin() * 6.0, 1.2, r.stomach.y as f32 + (time * 0.17).cos() * 6.0);
            swarm(f, c, 1.6, time, 90);
        }
    }

    /// Where the crust broke: a hole down into the Stomach's glow.
    fn feast_broken_crust(&self, r: &Feast, time: f32, f: &mut Frame) {
        for k in r.crust.iter().filter(|k| k.broken) {
            let c = Vec3::new(k.at.x as f32, 0.0, k.at.y as f32);
            f.opaque[MeshKind::Cube as usize].push(Instance::new(
                Mat4::from_translation(c + Vec3::new(0.0, -0.12, 0.0)) * Mat4::from_scale(Vec3::new(1.0, 0.02, 1.0)),
                lin(0x0a0404),
                1.0,
                lin(0xff7a20) * (0.15 + 0.05 * (time * 2.0 + c.x).sin()),
                SHADE_LIT,
            ));
            for j in 0..4 {
                let a = j as f32 * PI / 2.0 + c.x;
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::new(0.3, 0.06, 0.12),
                    Quat::from_rotation_y(a),
                    c + Vec3::new(a.cos() * 0.4, -0.03, a.sin() * 0.4),
                );
                f.opaque[MeshKind::Cube as usize].push(Instance::new(m, lin(CRUST_D), 1.0, Vec3::ZERO, SHADE_LIT));
            }
        }
    }

    /// The seams: closed ones shut by interlocking teeth and plugged with flesh; open narrow arches
    /// half-shut to him while he is too full to fit.
    fn feast_seam_teeth(&self, r: &Feast, time: f32, hero: Vec3, f: &mut Frame) {
        let cube_i = MeshKind::Cube as usize;
        for (i, s) in r.seams.iter().enumerate() {
            let fang = lin([MARBLE_L, 0xd88a50, 0xf0e6c8][i]);
            let gum = lin(0xb04050);
            for arch in [&s.narrow, &s.wide] {
                let shut = if s.closed { (s.since as f32 / 0.6).min(1.0) } else { 0.0 };
                let barred = !s.closed && !arch.wide && r.barred;
                if !s.closed && !barred {
                    continue;
                }
                for &p in &arch.tiles {
                    let c = Vec3::new(p.x as f32, 0.0, p.y as f32);
                    if !near_to(c, hero, NEAR) {
                        continue;
                    }
                    let wall = WALL_H[s.from.index()];
                    let south = arch.south.contains(&p);
                    let mouth = south || arch.north.contains(&p);
                    // Which way the mouth looks: toward the zone it opens on.
                    let n = Vec3::new(0.0, 0.0, if south { 1.0 } else { -1.0 });
                    if s.closed {
                        // Flesh swells to fill it as the teeth meet (set back behind the teeth at a mouth).
                        let k = shut * shut * (3.0 - 2.0 * shut);
                        let (back, depth) = if mouth { (-0.12, 0.76) } else { (0.0, 1.0) };
                        let m = Mat4::from_translation(c + n * back + Vec3::new(0.0, wall * k * 0.5, 0.0))
                            * Mat4::from_scale(Vec3::new(1.0, wall * k, depth));
                        f.opaque[cube_i].push(Instance::new(m, lin(FLESH), 1.0, Vec3::ZERO, SKIN));
                    }
                    if !mouth {
                        continue;
                    }
                    // The jaws: a row of teeth hanging from above and a row rising from below,
                    // biting shut; while he is too full they stand half shut, a slit too narrow for him.
                    let close = if s.closed { shut } else { 0.6 + 0.04 * (time * 3.0).sin() };
                    let reach = 0.08 + wall * 0.5 * close;
                    let face = c + n * 0.4;
                    for (y, col) in [(wall - 0.08, gum), (0.08, gum)] {
                        let m = Mat4::from_translation(face + Vec3::new(0.0, y, 0.0)) * Mat4::from_scale(Vec3::new(1.0, 0.16, 0.22));
                        f.opaque[cube_i].push(Instance::new(m, col, 1.0, Vec3::ZERO, SKIN));
                    }
                    for j in 0..3 {
                        let x = -0.33 + j as f32 * 0.33;
                        for (upper, dx) in [(true, x), (false, x + 0.165)] {
                            if dx > 0.45 {
                                continue;
                            }
                            let y = if upper { wall - 0.16 - reach / 2.0 } else { 0.16 + reach / 2.0 };
                            let m = Mat4::from_translation(face + Vec3::new(dx, y, 0.0)) * Mat4::from_scale(Vec3::new(0.15, reach, 0.12));
                            f.opaque[cube_i].push(Instance::new(m, fang, 1.0, Vec3::ZERO, IVORY));
                            let tip = if upper { y - reach / 2.0 - 0.04 } else { y + reach / 2.0 + 0.04 };
                            let m = Mat4::from_translation(face + Vec3::new(dx, tip, 0.0)) * Mat4::from_scale(Vec3::new(0.07, 0.08, 0.07));
                            f.opaque[cube_i].push(Instance::new(m, fang * 0.85, 1.0, Vec3::ZERO, IVORY));
                        }
                    }
                    if barred {
                        glow_orb(f, face + Vec3::new(0.0, wall * 0.5, 0.0), 0.5, lin(0xff5a3a), 0.12);
                    }
                }
            }
        }
    }

    /// The single place set at the Stomach's heart: under its silver dome until he comes, and then
    /// the dome lifted off an empty plate.
    fn feast_dome(&self, v: &FeastView, r: &Feast, time: f32, f: &mut Frame, lamps: &mut Vec<(Vec3, Vec3, f32)>) {
        let place = Vec3::new(r.dome.x as f32, 0.795, r.dome.y as f32 + 0.2);
        let cube_i = MeshKind::Cube as usize;
        let m = match &r.climax {
            None if !r.lifted => Mat4::from_translation(place) * Mat4::from_scale(Vec3::splat(0.85)),
            Some(c) => {
                // Lifted, it hangs over the empty plate, turning.
                let up = (c.t as f32 / 0.8).min(1.0);
                Mat4::from_translation(place + Vec3::new(0.0, 0.2 + up * 0.9 + (time * 1.5).sin() * 0.05, 0.0))
                    * Mat4::from_rotation_y(time * 0.8)
                    * Mat4::from_rotation_x(0.5 * up)
                    * Mat4::from_scale(Vec3::splat(0.85))
            }
            None => return,
        };
        push_merged(&mut f.opaque[cube_i], &v.art.cloche, m, 0.05);
        glow_orb(f, place + Vec3::new(0.0, 0.4, 0.0), 0.7, lin(0xfff0d0), 0.08);
        lamps.push((place + Vec3::new(0.0, 1.4, 0.0), lin(0xfff0d0) * 1.6, 5.0));
    }

    /// The pylorus: a ring of teeth round the door to the court, parting when it opens.
    fn feast_pylorus(&self, r: &Feast, time: f32, f: &mut Frame) {
        let g = r.base.exit;
        let root = Mat4::from_translation(Vec3::new(g.tile.x as f32, 0.0, g.tile.y as f32)) * Mat4::from_rotation_y(yaw_to(g.facing));
        let open = r.climax.as_ref().map(|c| c.teeth as f32).unwrap_or(0.0);
        let cube_i = MeshKind::Cube as usize;
        let centre = Vec3::new(0.0, 1.25, 0.62);
        // The gum ring.
        for k in 0..20 {
            let a = k as f32 / 20.0 * TAU;
            let p = centre + Vec3::new(a.cos() * 1.05, a.sin() * 1.05, 0.0);
            let m = root * Mat4::from_translation(p) * Mat4::from_rotation_z(a) * Mat4::from_scale(Vec3::new(0.24, 0.36, 0.28));
            f.opaque[cube_i].push(Instance::new(m, lin(0xa83040), 1.0, Vec3::ZERO, SLIME));
        }
        // The teeth, pointing in, drawn back as they part.
        for k in 0..14 {
            let a = k as f32 / 14.0 * TAU + 0.1;
            let len = (0.5 + 0.1 * (k % 2) as f32) * (1.0 - open * 0.5);
            // Shut, their points meet in the middle (chattering); open, they draw back to the gum.
            let chatter = 0.03 * (time * 6.0 + k as f32).sin() * (1.0 - open);
            let r_in = 1.0 - len * 0.5 + open * 0.2 + chatter;
            let p = centre + Vec3::new(a.cos() * r_in, a.sin() * r_in, 0.04);
            let m = root * Mat4::from_translation(p) * Mat4::from_rotation_z(a + PI / 2.0) * Mat4::from_scale(Vec3::new(0.14, len, 0.14));
            f.opaque[cube_i].push(Instance::new(m, lin(0xf0e6c8), 1.0, Vec3::ZERO, IVORY));
        }
        if open > 0.5 {
            glow_orb(f, root.transform_point3(centre), 1.2, lin(0xff8a2a), 0.15 * open);
        }
    }

    /// The Last Course: the walls closing in on the pylorus, the acid flooding inside them, the
    /// swarm hunting him, the food in the air.
    #[allow(clippy::too_many_arguments)]
    fn feast_last_course(
        &self,
        v: &FeastView,
        r: &Feast,
        c: &crate::logic::realms::feast::Climax,
        time: f32,
        hero: Vec3,
        f: &mut Frame,
        lamps: &mut Vec<(Vec3, Vec3, f32)>,
    ) {
        let cube_i = MeshKind::Cube as usize;
        let sq = Vec3::new(r.squeeze.x as f32, 0.0, r.squeeze.y as f32);
        let st = Vec3::new(r.stomach.x as f32, 0.0, r.stomach.y as f32);
        let walls = c.walls() as f32;
        let dry = c.dry() as f32;
        let inside = |p: Vec3| Vec2::new(p.x - st.x, p.z - st.z).length() <= r.stomach_radius as f32 + 0.6;
        // The walls: a ring of heaving flesh and teeth.
        let n = ((walls * TAU) / 0.55).ceil().max(12.0) as usize;
        for k in 0..n {
            let a = k as f32 / n as f32 * TAU;
            let p = sq + Vec3::new(a.cos() * walls, 0.0, a.sin() * walls);
            if !inside(p) {
                continue;
            }
            let heave = 1.0 + 0.12 * (time * 4.0 + k as f32 * 0.7).sin();
            let m = Mat4::from_translation(p + Vec3::new(0.0, 0.55 * heave, 0.0))
                * Mat4::from_rotation_y(-a)
                * Mat4::from_scale(Vec3::new(0.7, 1.1 * heave, 0.62));
            f.opaque[cube_i].push(Instance::new(m, lin(if k % 2 == 0 { FLESH } else { FLESH_D }), 1.0, Vec3::ZERO, SLIME));
            let tip = p - Vec3::new(a.cos(), 0.0, a.sin()) * 0.4;
            let m = Mat4::from_translation(tip + Vec3::new(0.0, 0.35, 0.0))
                * Mat4::from_rotation_y(-a)
                * Mat4::from_rotation_z(PI / 2.0)
                * Mat4::from_scale(Vec3::new(0.1, 0.32, 0.1));
            f.opaque[cube_i].push(Instance::new(m, lin(RIB), 1.0, Vec3::ZERO, IVORY));
        }
        // The acid, flooding in from the rim: every tile between the walls and the dry floor.
        let reach = walls.ceil() as i32 + 1;
        let (cx, cz) = (sq.x as i32, sq.z as i32);
        for z in cz - reach..=cz + reach {
            for x in cx - reach..=cx + reach {
                let p = Vec3::new(x as f32, 0.0, z as f32);
                let d = Vec2::new(p.x - sq.x, p.z - sq.z).length();
                if d < dry - 0.5 || d > walls + 0.5 || !inside(p) || !near_to(p, hero, NEAR) {
                    continue;
                }
                let k = ((d - dry) / ACID_BAND as f32).clamp(0.0, 1.0);
                let m = Mat4::from_translation(p + Vec3::new(0.0, 0.03 + 0.04 * k, 0.0)) * Mat4::from_scale(Vec3::new(1.0, 0.02, 1.0));
                let col = lin(ACID).lerp(lin(ACID_HOT), 0.3 + 0.3 * (time * 2.0 + p.x * 0.7 + p.z).sin());
                f.transparent[cube_i].push(Instance::new(m, col, 0.55 + 0.25 * k, col * 0.6, SHADE_UNLIT));
                if (x * 7 + z * 13 + (time * 3.0) as i32) % 9 == 0 {
                    let t = (time * 1.4 + p.x * 0.3).fract();
                    let b = p + Vec3::new(0.2, 0.06 + t * 0.2, -0.1);
                    f.additive.push(Instance::new(
                        Mat4::from_translation(b) * Mat4::from_scale(Vec3::splat(0.06 * (1.0 - t))),
                        col,
                        0.7,
                        Vec3::ZERO,
                        SHADE_UNLIT,
                    ));
                }
            }
        }
        let rim = sq + (Vec3::new(hero.x, 0.0, hero.z) - sq).normalize_or_zero() * dry.max(0.5);
        lamps.push((rim + Vec3::new(0.0, 0.5, 0.0), lin(ACID) * 4.0, 7.0));
        // The swarm, a dark cloud hunting him.
        let sw = Vec3::new(c.swarm.x as f32, 0.9, c.swarm.y as f32);
        swarm(f, sw, SWARM_RADIUS as f32, time, 160);
        // The food in the air, lobbed: a shadow on the floor under it.
        for m in &c.morsels {
            let flight = (m.age as f32 * MORSEL_SPEED as f32).min(9.0);
            let lob = if m.reflected { 0.6 } else { 0.5 + (flight * 0.4).sin().abs() * 0.6 };
            let p = Vec3::new(m.pos.x as f32, lob, m.pos.y as f32);
            let spin = Mat4::from_rotation_y(time * 9.0) * Mat4::from_rotation_x(time * 7.0);
            push_merged(
                &mut f.opaque[cube_i],
                v.art.dish(m.kind),
                Mat4::from_translation(p) * spin * Mat4::from_scale(Vec3::splat(0.6)),
                if m.reflected { 0.4 } else { 0.08 },
            );
            let shadow = Mat4::from_translation(Vec3::new(p.x, 0.02, p.z)) * Mat4::from_scale(Vec3::new(0.3, 0.01, 0.3));
            f.transparent[cube_i].push(Instance::new(shadow, Vec3::ZERO, 0.4, Vec3::ZERO, SHADE_UNLIT));
            if m.reflected {
                glow_orb(f, p, 0.4, lin(0xfff0c0), 0.25);
            }
        }
    }
}

/// A candle's flame at `c`, `size` across, its tip leaning north by `lean`.
fn flame(f: &mut Frame, c: Vec3, size: f32, lean: f32, time: f32, seed: f32) {
    for k in 0..3 {
        let t = (time * 2.6 + seed * 0.37 + k as f32 * 0.33).fract();
        let p = c + Vec3::new((seed * 1.9 + k as f32).sin() * 0.02, t * size * 2.6, -lean * t * 1.4 + (seed * 2.7).cos() * 0.02);
        let sz = size * (1.0 - t) + size * 0.35;
        let col = lin(FLAME).lerp(lin(FLAME_TIP), t);
        f.additive.push(Instance::new(
            Mat4::from_translation(p) * Mat4::from_rotation_x(-lean * 4.0) * Mat4::from_scale(Vec3::new(sz, sz * 1.7, sz)),
            col,
            0.55 * (1.0 - t),
            Vec3::ZERO,
            SHADE_UNLIT,
        ));
    }
    glow_orb(f, c + Vec3::new(0.0, size, -lean), size * 5.0, lin(FLAME), 0.16 + 0.03 * (time * 9.0 + seed).sin());
}

/// A swarm of flies round `c`: `count` dark specks churning in a cloud `radius` wide, its heart dim.
fn swarm(f: &mut Frame, c: Vec3, radius: f32, time: f32, count: usize) {
    for k in 0..count {
        let s = k as f32 * 2.399;
        let r = radius * ((s * 0.37).sin() * 0.5 + 0.5).sqrt();
        let a = s + time * (1.5 + (k % 7) as f32 * 0.3) * if k % 2 == 0 { 1.0 } else { -1.0 };
        let y = (s * 1.3 + time * 2.0).sin() * radius * 0.45;
        let p = c + Vec3::new(a.cos() * r, y, a.sin() * r);
        f.opaque[MeshKind::Cube as usize].push(Instance::new(
            Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.035)),
            lin(FLY),
            1.0,
            Vec3::ZERO,
            SHADE_UNLIT,
        ));
    }
    let m = Mat4::from_translation(c) * Mat4::from_scale(Vec3::splat(radius * 1.2));
    f.transparent[MeshKind::Sphere as usize].push(Instance::new(m, lin(0x0a0606), 0.35, Vec3::ZERO, SHADE_UNLIT));
}

/// Smooth noise over the tiles (0 to 1), in cells three tiles wide.
fn blob(x: i32, y: i32) -> f32 {
    let (fx, fy) = (x as f32 / 3.0, y as f32 / 3.0);
    let (x0, y0) = (fx.floor(), fy.floor());
    let (tx, ty) = (fx - x0, fy - y0);
    let at = |i: f32, j: f32| hash(i as f64 * 7.3, j as f64 * 3.1) as f32;
    let (sx, sy) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
    let top = at(x0, y0) + (at(x0 + 1.0, y0) - at(x0, y0)) * sx;
    let bottom = at(x0, y0 + 1.0) + (at(x0 + 1.0, y0 + 1.0) - at(x0, y0 + 1.0)) * sx;
    top + (bottom - top) * sy
}

/// The Larder turns to flesh toward its north end, in blobs.
fn fleshed(larder: crate::logic::hub::Rect, x: i32, y: i32) -> bool {
    let north = (larder.y1 - y) as f32 / (larder.y1 - larder.y0) as f32;
    north * 1.1 + blob(x, y) * 0.55 > 0.95
}

fn near_to(p: Vec3, hero: Vec3, range: f32) -> bool {
    Vec2::new(p.x - hero.x, p.z - hero.z).length_squared() < range * range
}

/// A floor tile of the realm's layout (for the crumbs: its rows as laid out, before any door opened).
fn s_walkable(r: &Feast, x: i32, y: i32) -> bool {
    r.base.rows.get(y as usize).and_then(|row| row.as_bytes().get(x as usize)).is_some_and(|&c| c == b' ' || c == b'P' || c == b'G')
}

/// The realm's sky, fog and light: warm amber candlelight in the Hall, orange firelight in the
/// Kitchens, a sickly wet pink in the Larder, and the Stomach's orange-green.
pub(super) fn atmosphere(_v: &FeastView, r: &Feast, s: &SceneState) -> Atmosphere {
    let y = s.hero.pos().y;
    // How deep he is, smoothly: a whole step for each zone, eased across the seams.
    let depth = |y: f64| r.dressing_at(y).index() as f32;
    let k = (depth(y - 1.5) + depth(y) + depth(y + 1.5)) / 3.0;
    let presets = [
        (0x140c06u32, 0xffd090u32, 0x7a4a20u32, 0x8a6030u32, 0xfff0c8u32, 18.0f32, 46.0f32),
        (0x180c06, 0xf0b888, 0x6a3a20, 0x7a4a30, 0xffd0a0, 15.0, 40.0),
        (0x140a0c, 0xe8a0a8, 0x5a2a30, 0x6a3a40, 0xf0c0c0, 12.0, 34.0),
        (0x0c0a04, 0xc8d070, 0x5a2a1a, 0x5a5420, 0xd8e0a0, 10.0, 30.0),
    ];
    let (a, b) = (k.floor() as usize, (k.ceil() as usize).min(3));
    let t = k - k.floor();
    let pa = presets[a.min(3)];
    let pb = presets[b];
    let mix = |x: u32, y: u32| lin(x).lerp(lin(y), t);
    let mut at = super::super::vault::vault_atmosphere();
    at.background = mix_hex(pa.0, pb.0, t);
    at.hemi_sky = mix(pa.1, pb.1) * 0.36;
    at.hemi_ground = mix(pa.2, pb.2) * 0.42;
    at.ambient = mix(pa.3, pb.3) * 0.38;
    at.sun = mix(pa.4, pb.4) * (1.0 - 0.12 * k);
    at.fog_near = pa.5 + (pb.5 - pa.5) * t;
    at.fog_far = pa.6 + (pb.6 - pa.6) * t;
    at.bloom = 0.95 + 0.1 * k;
    at.cloud_tint = mix(pa.2, pb.2) * 0.3;
    at
}
