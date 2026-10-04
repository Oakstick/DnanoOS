//! The Grey Mire as the view draws it (Sloth's realm; laid out in `logic::realms::mire`): a peat
//! bog drowned in grey fog, seen about eight tiles round him. Black peat and grey bog grass on the
//! low ground, the giant's body raised under moss (his face on the head: brow, shut eyes, a nose
//! that snores mist), pillowy moss beds, glossy mud with gears sunk in it, duckweed over the bog
//! holes and a blue marsh-light over every one, rotten causeways over black water, reeds, willows,
//! half-sunk cottages, the drowned chapel. On the palm the slate tower of the Stopped Hourglass,
//! its sand frozen mid-fall and faintly gold, the court door at its foot and Hel's bed Kör before
//! it, its curtains gleaming. The light is flat and grey and casts no shadows; the hourglass's glow
//! shows through the fog, the one thing to steer by.
//!
//! What changes is drawn each frame: the tide (one sheet of water over the whole bog), the
//! fingers rising and sinking, the palm (it tilts in the Long Sleep), the hourglass (it turns over),
//! the shortcuts' cart, raft and cage, the marsh-lights, the mist about him, the snores, the yawns.
//! When the land turns the game builds the whole scene again under the fog.

mod decor;

use std::collections::{HashMap, HashSet};

use super::super::*;
use crate::logic::realms::mire::{self as mr, Cell, Mire, Phase, RideKind, RideState};
use decor::*;

/// How brightly his lantern burns here: dimmed in the fog.
pub(super) const LANTERN: f32 = 0.6;

/// The mire's colours.
mod pal {
    pub const PEAT_L: u32 = 0x4a4337;
    pub const PEAT_D: u32 = 0x2c2822;
    pub const PEAT_SIDE: u32 = 0x1e1a16;
    pub const GRASS: u32 = 0x8a9086;
    pub const GRASS_D: u32 = 0x6a7068;
    pub const MOSS: u32 = 0x5d6650;
    pub const MOSS_L: u32 = 0x737e62;
    pub const MOSS_D: u32 = 0x474e3d;
    pub const PILLOW: u32 = 0x687652;
    pub const PILLOW_L: u32 = 0x7a8862;
    pub const FLOWER: u32 = 0xe8e4d8;
    pub const MUD: u32 = 0x2a2219;
    pub const MUD_L: u32 = 0x3a2f22;
    pub const DUCKWEED: u32 = 0x5e6c3a;
    pub const DUCKWEED_L: u32 = 0x748448;
    pub const WATER: u32 = 0x1e2428;
    pub const PLANK: u32 = 0x5e5040;
    pub const PLANK_D: u32 = 0x3e3429;
    pub const PLANK_L: u32 = 0x75674f;
    pub const REED: u32 = 0x8c9184;
    pub const REED_L: u32 = 0xa8aa98;
    pub const REED_D: u32 = 0x5e6458;
    pub const CATTAIL: u32 = 0x5a3e28;
    pub const STONE: u32 = 0x6f706a;
    pub const STONE_L: u32 = 0x85857c;
    pub const STONE_D: u32 = 0x55574f;
    pub const CHAPEL: u32 = 0x7a7c78;
    pub const CHAPEL_L: u32 = 0x8f918a;
    pub const CHAPEL_D: u32 = 0x5c5e5a;
    pub const THATCH: u32 = 0x5e5238;
    pub const THATCH_L: u32 = 0x74674a;
    pub const THATCH_D: u32 = 0x463c2a;
    pub const SLATE: u32 = 0x5a6270;
    pub const SLATE_L: u32 = 0x6e7888;
    pub const SLATE_D: u32 = 0x404652;
    pub const GLASS_D: u32 = 0x262c34;
    pub const VOID: u32 = 0x0c0c0e;
    pub const BARK: u32 = 0x4a4038;
    pub const BARK_D: u32 = 0x2e2824;
    pub const LEAF: u32 = 0x6d7a5e;
    pub const LEAF_L: u32 = 0x87957a;
    pub const LEAF_D: u32 = 0x56624a;
    pub const WEB: u32 = 0xa8acb0;
    pub const ROPE: u32 = 0x8a7a5a;
    pub const IRON: u32 = 0x3c3f44;
    pub const RUST: u32 = 0x7a4a2a;
    pub const BRASS: u32 = 0xa88a4a;
    pub const BRASS_D: u32 = 0x7a6234;
    pub const BRONZE: u32 = 0x8a6a3a;
    pub const BONE: u32 = 0xd8d0bc;
    pub const SIGN: u32 = 0x6a5236;
    pub const BEDWOOD: u32 = 0x231c18;
    pub const BEDWOOD_L: u32 = 0x3a302a;
    pub const LINEN: u32 = 0xd0ccc0;
    pub const LINEN_D: u32 = 0xa8a498;
    pub const COVER: u32 = 0x4a4258;
    pub const COVER_L: u32 = 0x5e5670;
    pub const CURTAIN: u32 = 0xf2ead2;
    pub const CURTAIN_D: u32 = 0x8a8270;
    pub const SAND: u32 = 0xe8c070;
    pub const GLASS: u32 = 0x8a96a2;
    pub const MARSH: u32 = 0x7ab8ff;
    pub const SKIN: u32 = 0x8a8070;
    pub const SKIN_D: u32 = 0x6a6256;
    pub const NAIL: u32 = 0xb8b2a2;
    pub const FOG: u32 = 0x5c6866;
    pub const SURGE: u32 = 0xe4e8ec;
}
use pal::*;

/// Mist is drawn this near him (tiles).
const MIST_RANGE: i32 = 11;
/// Marsh-lights and mud bubbles are drawn this near him.
const NEAR: f32 = 15.0;

/// What the view keeps of the realm (boxed: the realms' views share an enum).
pub(in crate::render::view) struct MireView(Box<MireArt>);

impl std::ops::Deref for MireView {
    type Target = MireArt;
    fn deref(&self) -> &MireArt {
        &self.0
    }
}

impl std::ops::DerefMut for MireView {
    fn deref_mut(&mut self) -> &mut MireArt {
        &mut self.0
    }
}

/// The realm's own scene: its planks, marsh-lights, mud, the giant's nostrils, the palm, the tower and the bed.
pub(in crate::render::view) struct MireArt {
    /// Each causeway plank's boxes (world tile), and those hidden since (rotted away).
    planks: HashMap<P, (usize, usize)>,
    hidden: HashSet<P>,
    /// Over each bog hole a marsh-light (world).
    holes: Vec<Vec3>,
    /// Mud, for the bubbles.
    mud: Vec<Vec3>,
    /// The giant's nostrils, whence the snores puff.
    nostrils: Vec<Vec3>,
    /// The palm's tiles, each with its look.
    palm: Vec<(P, f32)>,
    tower: Vec3,
    bed: Vec3,
}

/// The floor's height at a world tile, as the view draws it.
fn floor_y(r: &Mire, w: P) -> f32 {
    let (lv, ramp) = r.floor_at(w);
    if ramp {
        0.0
    } else {
        lv as f32 * STEP
    }
}

impl WorldView {
    /// The realm's static geometry.
    pub(super) fn build_mire(&mut self, grid: &Grid, r: &Mire) -> MireView {
        let mut planks = HashMap::new();
        let mut holes = Vec::new();
        let mut mud = Vec::new();
        let mut palm = Vec::new();
        let mut done_cottage: HashSet<u8> = HashSet::new();
        let mut chapel_done = false;
        let tower = Vec3::new(mr::CENTRE as f32, 0.0, mr::CENTRE as f32 - 2.0);
        let bed = Vec3::new(r.bed.x as f32, 0.0, r.bed.y as f32);
        for (x, y, _) in grid.cells() {
            let w = P::new(x, y);
            let c = r.cell(w);
            let (xf, zf) = (x as f32, y as f32);
            let hh = hash(x as f64 * 1.7 + 3.0, y as f64 * 2.3) as f32;
            let seed = (x * 31 + y * 17) as f32;
            let start = self.static_cubes.len();
            match c {
                Cell::Peat | Cell::Station(_) => {
                    let col = earth(PEAT_D, PEAT_L, x, y, 0.0);
                    self.ground(xf, zf, 0.0, col);
                    let out = &mut self.static_cubes;
                    // Bog grass in tussocks, sphagnum flecks, clods of peat, a puddle, a stone.
                    let sx = (hash(y as f64, x as f64) as f32 - 0.5) * 0.6;
                    if hh > 0.3 {
                        tuft(out, Vec3::new(xf + (hh - 0.5) * 0.6, 0.0, zf + sx), seed, if hh > 0.7 { GRASS } else { GRASS_D });
                    }
                    if hh > 0.75 {
                        tuft(out, Vec3::new(xf - sx, 0.0, zf - (hh - 0.8) * 2.0), seed + 5.0, GRASS_D);
                    }
                    let fleck = smooth(x as f32 * 0.31 + 7.0, y as f32 * 0.31);
                    if fleck > 0.7 {
                        for k in 0..2 {
                            let fk = seed + k as f32 * 2.7;
                            let (dx, dz) = ((hash(fk as f64, 3.0) as f32 - 0.5) * 0.8, (hash(fk as f64, 4.0) as f32 - 0.5) * 0.8);
                            bx(
                                out,
                                Vec3::new(xf + dx, 0.012, zf + dz),
                                Vec3::new(0.2, 0.025, 0.16),
                                fk,
                                if k == 0 { MOSS_L } else { MOSS },
                                SHADE_FUR,
                            );
                        }
                    }
                    if hh < 0.1 {
                        bx(out, Vec3::new(xf + 0.2, 0.004, zf - 0.15), Vec3::new(0.42, 0.01, 0.32), seed, WATER, SHADE_LIT);
                    } else if hh > 0.94 {
                        bx(out, Vec3::new(xf - 0.15, 0.05, zf + 0.2), Vec3::new(0.22, 0.1, 0.18), seed, STONE_D, SHADE_LIT);
                    } else if (0.5..0.58).contains(&hh) {
                        bx(out, Vec3::new(xf + sx, 0.04, zf - sx), Vec3::new(0.18, 0.08, 0.14), seed, PEAT_D, SHADE_LIT);
                    }
                    self.water_edge(r, w);
                }
                Cell::Hill(_) => {
                    let (_, ramp) = r.floor_at(w);
                    let col = earth(MOSS_D, MOSS_L, x, y, 3.0);
                    if ramp {
                        self.slope(r, w, col);
                    } else {
                        self.ground(xf, zf, STEP, col);
                        let out = &mut self.static_cubes;
                        // Soft lumps of moss over the giant's skin.
                        if hh > 0.72 {
                            let (dx, dz) = ((hh - 0.86) * 2.0, (hash(y as f64 * 3.0, x as f64) as f32 - 0.5) * 0.6);
                            bxc(
                                out,
                                Vec3::new(xf + dx, STEP + 0.04, zf + dz),
                                Vec3::new(0.5, 0.09, 0.42),
                                seed,
                                earth(MOSS, PILLOW, x, y, 9.0),
                                SHADE_FUR,
                            );
                            bxc(
                                out,
                                Vec3::new(xf + dx, STEP + 0.1, zf + dz),
                                Vec3::new(0.3, 0.06, 0.26),
                                seed,
                                earth(MOSS_L, PILLOW_L, x, y, 9.0),
                                SHADE_FUR,
                            );
                        }
                        if hh < 0.45 {
                            tuft(out, Vec3::new(xf + (hh - 0.22) * 2.0, STEP, zf - 0.2), seed, if hh < 0.2 { GRASS } else { GRASS_D });
                        }
                        if (0.6..0.66).contains(&hh) {
                            bx(out, Vec3::new(xf - 0.2, STEP + 0.05, zf + 0.25), Vec3::new(0.26, 0.1, 0.2), seed, STONE, SHADE_LIT);
                        }
                    }
                    self.water_edge(r, w);
                }
                Cell::Moss => {
                    let y0 = floor_y(r, w);
                    self.ground(xf, zf, y0, lin(MOSS_D));
                    let out = &mut self.static_cubes;
                    // Cushions of moss, soft as pillows, three or four to a tile.
                    for (k, (ox, oz)) in [(-0.22f32, -0.2f32), (0.22, -0.18), (-0.18, 0.22), (0.2, 0.2)].into_iter().enumerate() {
                        let fk = seed + k as f32 * 3.3;
                        if k == 3 && hh < 0.4 {
                            continue;
                        }
                        let j = (hash(fk as f64, 7.0) as f32 - 0.5) * 0.12;
                        let size = 0.5 + hash(fk as f64, 8.0) as f32 * 0.14;
                        let c = earth(PILLOW, PILLOW_L, x + k as i32, y, 4.0);
                        for (l, (w, hgt)) in [(1.0f32, 0.07f32), (0.82, 0.07), (0.56, 0.05)].into_iter().enumerate() {
                            let yy = y0 + 0.035 + l as f32 * 0.06;
                            bxc(
                                out,
                                Vec3::new(xf + ox + j, yy, zf + oz - j),
                                Vec3::new(size * w, hgt, size * w * 0.9),
                                fk + l as f32 * 0.4,
                                c * (1.0 + l as f32 * 0.06),
                                SHADE_FUR,
                            );
                        }
                    }
                    for k in 0..3 {
                        let fk = seed + k as f32;
                        let a = fk * 2.3;
                        bx(
                            out,
                            Vec3::new(xf + a.cos() * 0.3, y0 + 0.2 - k as f32 * 0.04, zf + a.sin() * 0.3),
                            Vec3::splat(0.05),
                            a,
                            FLOWER,
                            SHADE_LIT,
                        );
                    }
                    self.water_edge(r, w);
                }
                Cell::Mud => {
                    let out = &mut self.static_cubes;
                    bx(out, Vec3::new(xf, -0.4, zf), Vec3::new(1.0, 0.72, 1.0), 0.0, PEAT_SIDE, SHADE_LIT);
                    bx(out, Vec3::new(xf, -0.035, zf), Vec3::new(1.0, 0.03, 1.0), 0.0, if hh > 0.5 { MUD } else { MUD_L }, SHADE_LIT);
                    if hh > 0.6 {
                        bx(out, Vec3::new(xf + 0.2, -0.015, zf - 0.1), Vec3::new(0.3, 0.012, 0.22), seed, MUD_L, SHADE_LIT);
                    }
                    mud.push(Vec3::new(xf, -0.02, zf));
                }
                Cell::Hole => {
                    // Duckweed, a bright mat floating flush with the peat round it: it looks like firm ground.
                    let out = &mut self.static_cubes;
                    bx(out, Vec3::new(xf, -0.02, zf), Vec3::new(1.0, 0.03, 1.0), 0.0, DUCKWEED, SHADE_LIT);
                    for k in 0..5 {
                        let fk = seed + k as f32 * 1.9;
                        let (dx, dz) = ((hash(fk as f64, 1.0) as f32 - 0.5) * 0.8, (hash(fk as f64, 2.0) as f32 - 0.5) * 0.8);
                        bx(out, Vec3::new(xf + dx, -0.002, zf + dz), Vec3::new(0.16, 0.012, 0.12), fk, DUCKWEED_L, SHADE_LIT);
                    }
                    holes.push(Vec3::new(xf, 0.0, zf));
                }
                Cell::Plank => {
                    if r.firm(w) {
                        self.plank(r, w, seed);
                        planks.insert(w, (start, self.static_cubes.len() - start));
                    }
                }
                Cell::Deep => {
                    // Reed beds where the world ends.
                    let by_land = CARDINALS.iter().chain(&[Dir::None]).any(|&d| {
                        let q = w.step(d);
                        (-1..=1).any(|o| {
                            let n = P::new(q.x + o, q.y);
                            mr::inside(n) && r.cell(n).firm()
                        })
                    });
                    if by_land {
                        let out = &mut self.static_cubes;
                        for k in 0..3 {
                            let fk = seed + k as f32 * 3.1;
                            let at = Vec3::new(
                                xf + (hash(fk as f64, 4.0) as f32 - 0.5) * 0.8,
                                -0.25,
                                zf + (hash(fk as f64, 5.0) as f32 - 0.5) * 0.8,
                            );
                            reeds(out, at, fk, 1.5);
                        }
                    }
                }
                Cell::Palm | Cell::Threshold => palm.push((w, hh)),
                Cell::Cottage(k) => {
                    if done_cottage.insert(k) {
                        if let Some(&(rect, lean)) = r.cottages.get(k as usize) {
                            let t = (0..r.turn).fold(rect, |q, _| mr::turn_rect(q));
                            let s0 = self.static_cubes.len();
                            for p in t.tiles() {
                                self.ground(p.x as f32, p.y as f32, 0.0, lin(PEAT_D));
                            }
                            cottage(&mut self.static_cubes, t.x0, t.y0, t.x1, t.y1, lean, k as f32 * 13.0);
                            let (cx, cz) = ((t.x0 + t.x1) as f32 / 2.0, (t.y0 + t.y1) as f32 / 2.0);
                            self.fade_range(s0, Vec3::new(cx, 0.0, cz), 1.6, 2.6);
                        }
                    }
                }
                Cell::Chapel => {
                    if !chapel_done {
                        chapel_done = true;
                        let t = (0..r.turn).fold(r.chapel, |q, _| mr::turn_rect(q));
                        let s0 = self.static_cubes.len();
                        let toward =
                            r.v_world(crate::logic::types::V::new((r.chapel.x0 + r.chapel.x1) as f64 / 2.0, r.chapel.y0 as f64 - 3.0));
                        chapel(&mut self.static_cubes, t.x0, t.y0, t.x1, t.y1, Vec3::new(toward.x as f32, 0.0, toward.y as f32));
                        let (cx, cz) = ((t.x0 + t.x1) as f32 / 2.0, (t.y0 + t.y1) as f32 / 2.0);
                        self.fade_range(s0, Vec3::new(cx, 0.0, cz), 2.6, 4.6);
                    }
                }
                Cell::Willow => {
                    self.ground(xf, zf, 0.0, lin(PEAT_D));
                    let s0 = self.static_cubes.len();
                    willow(&mut self.static_cubes, Vec3::new(xf, 0.0, zf), seed);
                    self.fade_range(s0, Vec3::new(xf, 0.0, zf), 1.2, 2.9);
                }
                Cell::Oak => {
                    self.ground(xf, zf, 0.0, lin(PEAT_D));
                    let s0 = self.static_cubes.len();
                    oak(&mut self.static_cubes, Vec3::new(xf, 0.0, zf));
                    self.fade_range(s0, Vec3::new(xf, 0.0, zf), 1.3, 3.0);
                }
                Cell::Pool | Cell::Moat | Cell::Finger(..) | Cell::Track(..) | Cell::Tower | Cell::Door => {}
            }
        }
        self.giant_face(r);
        // Gears sunk in the mud.
        for (k, g) in r.gears.iter().enumerate() {
            let w = r.to_world(*g);
            if r.cell(w) == Cell::Mud {
                gear(&mut self.static_cubes, Vec3::new(w.x as f32, -0.1, w.y as f32), k as f32 * 3.7 + 1.0);
            }
        }
        // Belphegor's shortcuts: their ways and stations.
        for ride in &r.rides {
            let way: Vec<Vec3> = ride
                .path
                .iter()
                .map(|&p| {
                    let w = r.to_world(p);
                    Vec3::new(w.x as f32, 0.0, w.y as f32)
                })
                .collect();
            ride_way(&mut self.static_cubes, ride.kind, &way, ride.stop);
        }
        // The tower of the Stopped Hourglass (its door is the framework's portal arch).
        let s0 = self.static_cubes.len();
        decor::tower(&mut self.static_cubes, tower);
        self.fade_range(s0, tower, 1.7, decor::HOURGLASS_Y + 2.0);
        // The nostrils, for the snores.
        let head = r.head_world();
        let face = face_dir(head);
        let hv = Vec3::new(head.x as f32, STEP, head.y as f32);
        let side = Vec3::new(-face.z, 0.0, face.x);
        let nostrils = vec![hv + face * 2.6 + side * 0.35 + Vec3::Y * 0.45, hv + face * 2.6 - side * 0.35 + Vec3::Y * 0.45];
        MireView(Box::new(MireArt { planks, hidden: HashSet::new(), holes, mud, nostrils, palm, tower, bed }))
    }

    /// A tile of ground: a block of peat from below the water to `top`, capped in `color`.
    fn ground(&mut self, x: f32, z: f32, top: f32, color: Vec3) {
        let out = &mut self.static_cubes;
        let depth = 0.75 + top;
        bx(out, Vec3::new(x, top - depth / 2.0 - 0.04, z), Vec3::new(1.0, depth - 0.08, 1.0), 0.0, PEAT_SIDE, SHADE_LIT);
        bxc(out, Vec3::new(x, top - 0.04, z), Vec3::new(1.0, 0.08, 1.0), 0.0, color, SHADE_LIT);
    }

    /// A slope up onto the giant: three steps of moss rising toward the higher ground.
    fn slope(&mut self, r: &Mire, w: P, color: Vec3) {
        let up = CARDINALS.iter().copied().find(|&d| {
            let q = w.step(d);
            let (lv, ramp) = r.floor_at(q);
            lv == 1 && !ramp && r.cell(q).firm()
        });
        let (xf, zf) = (w.x as f32, w.y as f32);
        let Some(d) = up else {
            self.ground(xf, zf, STEP * 0.5, color);
            return;
        };
        let (dx, dz) = d.vec();
        let out = &mut self.static_cubes;
        bx(out, Vec3::new(xf, -0.4, zf), Vec3::new(1.0, 0.72, 1.0), 0.0, PEAT_SIDE, SHADE_LIT);
        for k in 0..3 {
            let t = (k as f32 + 0.5) / 3.0 - 0.5;
            let top = STEP * (k as f32 + 1.0) / 3.0;
            let c = Vec3::new(xf + dx as f32 * t, top / 2.0, zf + dz as f32 * t);
            let s = if dx != 0 { Vec3::new(1.0 / 3.0, top, 1.0) } else { Vec3::new(1.0, top, 1.0 / 3.0) };
            bxc(out, c, s, 0.0, if k == 1 { color * 1.12 } else { color }, SHADE_LIT);
        }
    }

    /// Reeds where the land meets the water.
    fn water_edge(&mut self, r: &Mire, w: P) {
        let (xf, zf) = (w.x as f32, w.y as f32);
        for d in CARDINALS {
            let q = w.step(d);
            if !matches!(r.cell(q), Cell::Pool | Cell::Moat) {
                continue;
            }
            let hh = hash(q.x as f64 * 3.1 + w.x as f64, q.y as f64 * 1.3 + w.y as f64) as f32;
            if hh > 0.45 {
                let (dx, dz) = d.vec();
                let at = Vec3::new(xf + dx as f32 * 0.45, floor_y(r, w) - 0.1, zf + dz as f32 * 0.45);
                reeds(&mut self.static_cubes, at, hh * 50.0 + xf, 0.9);
            }
        }
    }

    /// A causeway plank: rotten boards across the way over the water, some gone, some askew, a
    /// stake now and then.
    fn plank(&mut self, r: &Mire, w: P, seed: f32) {
        let (xf, zf) = (w.x as f32, w.y as f32);
        let joined = |d: Dir| {
            let q = w.step(d);
            r.cell(q).firm() || matches!(r.cell(q), Cell::Plank)
        };
        let along_x = (joined(Dir::Left) || joined(Dir::Right)) && !(joined(Dir::Up) && joined(Dir::Down));
        let out = &mut self.static_cubes;
        for k in 0..5 {
            let fk = seed + k as f32 * 1.3;
            let hk = hash(fk as f64, 9.0) as f32;
            if hk < 0.12 {
                continue;
            }
            let off = (k as f32 - 2.0) * 0.2;
            let skew = (hk - 0.5) * 0.25;
            let (pos, yaw) = if along_x {
                (Vec3::new(xf + off, -0.03, zf + skew * 0.3), PI / 2.0 + skew)
            } else {
                (Vec3::new(xf + skew * 0.3, -0.03, zf + off), skew)
            };
            let c = [PLANK, PLANK_D, PLANK_L, PLANK][(hk * 4.0) as usize % 4];
            bx(out, pos, Vec3::new(0.92, 0.06, 0.16), yaw, c, SHADE_WOOD);
        }
        // Rails of half-sunk logs either side.
        for s in [-0.47f32, 0.47] {
            let (a, b) = if along_x {
                (Vec3::new(xf - 0.5, -0.07, zf + s), Vec3::new(xf + 0.5, -0.07, zf + s))
            } else {
                (Vec3::new(xf + s, -0.07, zf - 0.5), Vec3::new(xf + s, -0.07, zf + 0.5))
            };
            beam(out, a, b, 0.09, PLANK_D, SHADE_WOOD);
        }
        if hash(seed as f64, 3.0) > 0.7 {
            let s = if hash(seed as f64, 4.0) > 0.5 { 0.48 } else { -0.48 };
            let at = if along_x { Vec3::new(xf, 0.0, zf + s) } else { Vec3::new(xf + s, 0.0, zf) };
            beam(out, at - Vec3::Y * 0.5, at + Vec3::Y * 0.35, 0.08, PLANK_D, SHADE_WOOD);
        }
    }

    /// The boxes from `start` to now dissolve when they stand between the camera and him.
    fn fade_range(&mut self, start: usize, base: Vec3, radius: f32, height: f32) {
        let len = self.static_cubes.len() - start;
        let shades: Vec<f32> = self.static_cubes[start..start + len].iter().map(|i| i.emissive[3]).collect();
        self.trees.push(Tree { range: (start, len), base, radius, height, fade: 1.0, alive: true, shades: Some(shades) });
    }

    /// The giant's face on his head: a brow of stones, his shut eyes, a great nose with its
    /// nostrils toward his hand, lips in the moss, a beard of reeds; and under his cheek his
    /// other hand, curled, its knuckles and nails showing.
    fn giant_face(&mut self, r: &Mire) {
        let head = r.head_world();
        let face = face_dir(head);
        let side = Vec3::new(-face.z, 0.0, face.x);
        let c = Vec3::new(head.x as f32, STEP, head.y as f32);
        let yaw = face.x.atan2(face.z);
        let out = &mut self.static_cubes;
        let at = |f: f32, s: f32, up: f32| c + face * f + side * s + Vec3::Y * up;
        let skin = |out: &mut Vec<Instance>, p: Vec3, size: Vec3, color: u32| bx(out, p, size, yaw, color, SHADE_SKIN);
        // The brow: a ridge of grey boulders across the face, moss in its cracks.
        for k in -4..=4 {
            let t = k as f32 / 4.0;
            let p = at(0.6 - t * t * 0.5, t * 2.2, 0.16 - t * t * 0.08);
            bx(out, p, Vec3::new(0.62, 0.3, 0.46), yaw + t * 0.4, if k % 2 == 0 { STONE_D } else { STONE }, SHADE_LIT);
        }
        // The eyes, shut: a lid of grey skin on each, its seam a dark crease, grass for lashes.
        for s in [-1.0f32, 1.0] {
            for (k, (w, hgt)) in [(1.7f32, 0.12f32), (1.3, 0.1), (0.8, 0.08)].into_iter().enumerate() {
                skin(out, at(1.45, s * 1.15, 0.06 + k as f32 * 0.09), Vec3::new(w, hgt, w * 0.62), if k == 1 { SKIN } else { SKIN_D });
            }
            let mut prev: Option<Vec3> = None;
            for k in 0..7 {
                let t = k as f32 / 6.0 - 0.5;
                let p = at(1.75 - t * t * 0.9, s * 1.15 + t * 1.5, 0.27 - t * t * 0.3);
                if let Some(q) = prev {
                    beam(out, q, p, 0.08, VOID, SHADE_LIT);
                }
                if k % 2 == 1 {
                    tuft(out, p + face * 0.08, s * 9.0 + k as f32, GRASS_D);
                }
                prev = Some(p);
            }
        }
        // The nose: a long ridge of skin toward his hand, its nostrils dark.
        for k in 0..6 {
            let t = k as f32 / 5.0;
            skin(
                out,
                at(1.6 + t * 1.2, 0.0, 0.14 + t * 0.2 - t * t * 0.06),
                Vec3::new(0.55 + t * 0.3, 0.3 + t * 0.12, 0.42),
                if k % 2 == 0 { SKIN } else { SKIN_D },
            );
        }
        for s in [-1.0f32, 1.0] {
            bx(out, at(2.85, s * 0.26, 0.36), Vec3::new(0.16, 0.12, 0.1), yaw, VOID, SHADE_LIT);
        }
        // The lips, parted a crack, and a beard of reeds falling into the bog.
        for k in -3..=3 {
            let t = k as f32 / 3.0;
            let lift = -(t * t) * 0.06;
            skin(out, at(3.2 - t * t * 0.25, t * 1.25, 0.06 + lift), Vec3::new(0.42, 0.12, 0.2), SKIN_D);
            skin(out, at(3.5 - t * t * 0.25, t * 1.2, 0.03 + lift), Vec3::new(0.42, 0.1, 0.2), SKIN_D);
            bx(out, at(3.36 - t * t * 0.25, t * 1.22, 0.05 + lift), Vec3::new(0.42, 0.04, 0.06), yaw, VOID, SHADE_LIT);
        }
        for k in -3..=3 {
            reeds(out, at(4.0, k as f32 * 0.55, -0.5), k as f32 * 7.0 + 3.0, 1.1);
        }
        // An ear, curled, on the far side of his head.
        for k in 0..7 {
            let a = k as f32 / 6.0 * PI * 1.3 - 0.3;
            skin(out, at(0.4 + a.cos() * 0.6, -2.7, 0.12 + a.sin() * 0.3), Vec3::new(0.26, 0.22, 0.34), SKIN_D);
        }
        // His other hand, curled under his cheek: four long fingers and their nails, a thumb.
        let hand = at(1.5, 3.2, 0.0);
        for k in 0..4 {
            let base = hand + face * (k as f32 * 0.62 - 0.9);
            for j in 0..3 {
                let p = base - side * (j as f32 * 0.55) + Vec3::Y * (0.12 - j as f32 * 0.03);
                skin(out, p, Vec3::new(0.58, 0.42 - j as f32 * 0.05, 0.52), if j == 1 { SKIN } else { SKIN_D });
            }
            let tip = base - side * 1.45 + Vec3::Y * 0.12;
            bx(out, tip, Vec3::new(0.3, 0.06, 0.36), yaw, NAIL, SHADE_LIT);
        }
        skin(out, hand + face * 1.6 + side * 0.4, Vec3::new(0.5, 0.4, 0.9), SKIN_D);
    }

    /// The realm's own animation, one step: planks that rotted away leave the scene.
    pub(super) fn update_mire_view(&mut self, v: &mut MireView, r: &Mire, _s: &SceneState, _dt: f32) {
        let gone: Vec<(P, (usize, usize))> =
            v.planks.iter().filter(|(w, _)| !v.hidden.contains(*w) && !r.firm(**w)).map(|(w, rg)| (*w, *rg)).collect();
        for (w, range) in gone {
            v.hidden.insert(w);
            self.hide_static(range);
        }
    }

    /// What moves, glows and lights in the realm.
    pub(super) fn emit_mire(&self, v: &MireView, r: &Mire, _s: &SceneState, time: f32, hero: Vec3, f: &mut Frame) {
        let cube = MeshKind::Cube as usize;
        // ---- the tide: one sheet of black water over the whole bog, rising and falling with his breath
        let wy = r.water_y() as f32;
        let n = mr::SIZE as f32;
        let m =
            Mat4::from_translation(Vec3::new(n / 2.0 - 0.5, wy, n / 2.0 - 0.5)) * Mat4::from_scale(Vec3::new(n + 30.0, 0.002, n + 30.0));
        f.transparent_early.push(Instance::new(m, lin(WATER), 0.94, Vec3::ZERO, SHADE_LIT));
        // Rings where something stirs under the black water about him.
        let (hx0, hz0) = (hero.x.round() as i32, hero.z.round() as i32);
        for k in 0..10 {
            let fk = k as f32 * 2.39;
            let t = (time * 0.35 + fk * 0.31).fract();
            let cycle = (time * 0.35 + fk * 0.31).floor();
            let (dx, dz) = (((hash(fk as f64, cycle as f64) * 17.0) as i32) - 8, ((hash(cycle as f64, fk as f64) * 13.0) as i32) - 6);
            let w = P::new(hx0 + dx, hz0 + dz);
            if mr::inside(w) && matches!(r.cell(w), Cell::Pool | Cell::Moat | Cell::Deep) {
                let rad = 0.1 + t * 0.6;
                let mm = Mat4::from_translation(Vec3::new(w.x as f32, wy + 0.005, w.y as f32))
                    * Mat4::from_rotation_x(-PI / 2.0)
                    * Mat4::from_scale(Vec3::new(rad, rad, 1.0));
                f.transparent[MeshKind::Annulus as usize].push(Instance::new(mm, lin(0x8a949c), 0.35 * (1.0 - t), Vec3::ZERO, SHADE_UNLIT));
            }
        }
        // ---- the palm, tilting in the Long Sleep, and the bed on it
        let tilt = match r.phase {
            Phase::Sand { .. } | Phase::Done { .. } => {
                let since = r.turned_at.map(|t| (r.clock - t) as f32).unwrap_or(9.0);
                0.045 * (since / 2.0).min(1.0) + 0.008 * (time * 0.7).sin()
            }
            _ => 0.0,
        };
        let pivot = v.bed;
        let palm_m = Mat4::from_translation(pivot)
            * Mat4::from_rotation_x(tilt)
            * Mat4::from_rotation_z(tilt * 0.4)
            * Mat4::from_translation(-pivot);
        for &(w, hh) in &v.palm {
            let (xf, zf) = (w.x as f32, w.y as f32);
            let (_, rim) = r.floor_at(w);
            let top = if rim { STEP * 0.5 } else { STEP };
            let at = palm_m.transform_point3(Vec3::new(xf, top, zf));
            let col = earth(SKIN_D, MOSS, w.x, w.y, 21.0)
                .lerp(lin(SKIN), (smooth(w.x as f32 * 0.4, w.y as f32 * 0.4 + 3.0) - 0.55).max(0.0) * 1.6);
            let depth = top + 0.7;
            let m = palm_m
                * Mat4::from_scale_rotation_translation(
                    Vec3::new(1.0, depth, 1.0),
                    Quat::IDENTITY,
                    Vec3::new(xf, top - depth / 2.0 - 0.03, zf),
                );
            f.opaque[cube].push(Instance::new(m, lin(PEAT_SIDE), 1.0, Vec3::ZERO, SHADE_LIT));
            let m =
                palm_m * Mat4::from_scale_rotation_translation(Vec3::new(1.0, 0.06, 1.0), Quat::IDENTITY, Vec3::new(xf, top - 0.03, zf));
            f.opaque[cube].push(Instance::new(m, col, 1.0, Vec3::ZERO, SHADE_SKIN));
            if hh > 0.8 && w != r.bed {
                tuft_frame(f, at, hh * 40.0);
            }
        }
        // The palm's lines, creased across it: the heart line, the head line, the life line.
        for (k, &(a0, a1, r0, r1)) in [(-2.6f32, -1.2f32, 3.6f32, 3.2f32), (2.8, 1.6, 3.4, 2.9), (0.4, 1.4, 4.2, 3.0)].iter().enumerate() {
            let mut prev: Option<Vec3> = None;
            for j in 0..7 {
                let t = j as f32 / 6.0;
                let a = a0 + (a1 - a0) * t;
                let rr = r0 + (r1 - r0) * t + (t * PI).sin() * 0.3;
                let p = palm_m.transform_point3(Vec3::new(
                    mr::CENTRE as f32 + a.cos() * rr,
                    STEP + 0.004,
                    mr::CENTRE as f32 + 1.0 + a.sin() * rr * 0.8 + k as f32 * 0.2,
                ));
                if let Some(q) = prev {
                    let d = p - q;
                    let mm = Mat4::from_scale_rotation_translation(
                        Vec3::new(0.07, 0.01, d.length() + 0.04),
                        Quat::from_rotation_y(d.x.atan2(d.z)),
                        (p + q) / 2.0,
                    );
                    f.opaque[cube].push(Instance::new(mm, lin(SKIN_D) * 0.7, 1.0, Vec3::ZERO, SHADE_LIT));
                }
                prev = Some(p);
            }
        }
        // The bog hole under the threshold: black water round the stone.
        let hole =
            palm_m * Mat4::from_scale_rotation_translation(Vec3::new(1.7, 0.02, 1.8), Quat::IDENTITY, pivot + Vec3::new(0.0, 0.004, 0.0));
        f.opaque[cube].push(Instance::new(hole, lin(VOID), 1.0, Vec3::ZERO, SHADE_LIT));
        let mut bed_out = Vec::new();
        let bed_root = palm_m * Mat4::from_translation(pivot + Vec3::new(0.0, STEP + 0.06, 0.0));
        decor::bed(&mut bed_out, bed_root);
        f.opaque[cube].extend(bed_out);
        // Its curtains, gleaming, gathered at the posts and tied back, stirring though there is no wind.
        let gleam = 0.1 + 0.06 * (time * 1.3).sin();
        let cloth = lin(CURTAIN);
        for (k, (px, pz)) in [(-0.45f32, -0.55f32), (0.45, -0.55), (-0.45, 0.55), (0.45, 0.55)].into_iter().enumerate() {
            let inward = Vec3::new(-px.signum(), 0.0, 0.0);
            for j in 0..3 {
                let fold = j as f32 - 1.0;
                let sway = (time * 0.8 + k as f32 * 1.9 + j as f32 * 0.7).sin() * 0.025;
                // Down from the canopy, pinched at the tie, flaring to the floor.
                for (y0, y1, spread) in [(1.5f32, 0.95f32, 0.12f32), (0.95, 0.85, 0.05), (0.85, 0.12, 0.16)] {
                    let mid = (y0 + y1) / 2.0;
                    let off = Vec3::new(px, mid, pz) + inward * (0.06 + spread * 0.5) + Vec3::new(0.0, 0.0, fold * spread * 0.6 + sway);
                    let mm = bed_root
                        * Mat4::from_scale_rotation_translation(
                            Vec3::new(0.05 + spread * 0.3, y0 - y1, 0.06 + spread * 0.25),
                            Quat::IDENTITY,
                            off,
                        );
                    f.opaque[cube].push(Instance::new(mm, cloth * (0.92 + 0.06 * fold), 1.0, cloth * gleam, SHADE_CLOTH));
                }
            }
            // The tie, a cord of tarnished gold.
            let tie = bed_root
                * Mat4::from_scale_rotation_translation(Vec3::new(0.16, 0.04, 0.16), Quat::IDENTITY, Vec3::new(px, 0.9, pz) + inward * 0.1);
            f.opaque[cube].push(Instance::new(tie, lin(BRASS), 1.0, lin(BRASS) * 0.1, SHADE_BRASS));
        }
        for (x, z, sx, sz) in [(0.0f32, -0.6f32, 1.0f32, 0.04f32), (0.0, 0.6, 1.0, 0.04), (-0.5, 0.0, 0.04, 1.2), (0.5, 0.0, 0.04, 1.2)] {
            let n = 6;
            for j in 0..n {
                let t = (j as f32 + 0.5) / n as f32 - 0.5;
                let drop = if j % 2 == 0 { 0.16 } else { 0.11 };
                let pos = Vec3::new(x + t * sx * (sx > 0.5) as i32 as f32, 1.5 - drop / 2.0, z + t * sz * (sz > 0.5) as i32 as f32);
                let size = Vec3::new(if sx > 0.5 { sx / n as f32 } else { 0.04 }, drop, if sz > 0.5 { sz / n as f32 } else { 0.04 });
                let mm = bed_root * Mat4::from_scale_rotation_translation(size, Quat::IDENTITY, pos);
                f.opaque[cube].push(Instance::new(mm, cloth, 1.0, cloth * gleam, SHADE_CLOTH));
            }
        }
        let bed_light = bed_root.transform_point3(Vec3::new(0.0, 1.2, 0.6));
        // ---- the fingers, joint by joint, rising out of the moat toward the still
        for fing in &r.fingers {
            let joints: Vec<Vec3> = fing
                .joints
                .iter()
                .map(|&j| {
                    let w = r.to_world(j);
                    Vec3::new(w.x as f32, 0.0, w.y as f32)
                })
                .collect();
            for (i, &p) in joints.iter().enumerate() {
                let rise = fing.rise(i) as f32;
                if rise <= 0.0 {
                    continue;
                }
                let prev = if i == 0 { Vec3::new(mr::CENTRE as f32, 0.0, mr::CENTRE as f32) } else { joints[i - 1] };
                let next = joints.get(i + 1).copied().unwrap_or(p + (p - prev));
                let dir = (next - prev).normalize_or(Vec3::X);
                let yaw = dir.x.atan2(dir.z);
                let sink = (1.0 - rise) * 0.75;
                let top = -0.02 - sink;
                let rot = Quat::from_rotation_y(yaw);
                let knuckle = i % 3 == 2;
                let c = if knuckle { SKIN } else { SKIN_D };
                f.opaque[cube].push(Instance::new(
                    Mat4::from_scale_rotation_translation(Vec3::new(0.94, 0.7, 1.02), rot, p + Vec3::new(0.0, top - 0.35, 0.0)),
                    lin(c),
                    1.0,
                    Vec3::ZERO,
                    SHADE_SKIN,
                ));
                f.opaque[cube].push(Instance::new(
                    Mat4::from_scale_rotation_translation(Vec3::new(0.7, 0.06, 0.8), rot, p + Vec3::new(0.0, top + 0.01, 0.0)),
                    lin(if i % 2 == 0 { MOSS } else { MOSS_D }),
                    1.0,
                    Vec3::ZERO,
                    SHADE_FUR,
                ));
                if knuckle {
                    f.opaque[cube].push(Instance::new(
                        Mat4::from_scale_rotation_translation(Vec3::new(1.0, 0.18, 0.3), rot, p + Vec3::new(0.0, top + 0.02, 0.0)),
                        lin(SKIN),
                        1.0,
                        Vec3::ZERO,
                        SHADE_SKIN,
                    ));
                }
                if i + 1 == joints.len() {
                    // The nail at the fingertip.
                    f.opaque[cube].push(Instance::new(
                        Mat4::from_scale_rotation_translation(
                            Vec3::new(0.6, 0.06, 0.4),
                            rot,
                            p + dir * 0.25 + Vec3::new(0.0, top + 0.05, 0.0),
                        ),
                        lin(NAIL),
                        1.0,
                        Vec3::ZERO,
                        SHADE_LIT,
                    ));
                }
                if rise < 1.0 {
                    // Rising or sinking: the bog runs off it.
                    for k in 0..4 {
                        let t = (time * 2.0 + k as f32 * 0.25 + i as f32 * 0.3).fract();
                        let a = k as f32 * 1.6 + i as f32;
                        let at = p + Vec3::new(a.cos() * 0.45, top - t * 0.4, a.sin() * 0.45);
                        f.opaque[MeshKind::Sphere as usize].push(Instance::new(
                            Mat4::from_translation(at) * Mat4::from_scale(Vec3::splat(0.05 * (1.0 - t) + 0.01)),
                            lin(MUD_L),
                            1.0,
                            Vec3::ZERO,
                            SHADE_WET,
                        ));
                    }
                }
            }
        }
        // ---- the shortcuts' cart, raft and cage, where each stands
        for ride in &r.rides {
            self.emit_vehicle(r, ride, time, f);
        }
        // ---- the hourglass: frozen sand until it turns over, then running out
        self.emit_hourglass(v, r, time, f);
        // ---- marsh-lights over the bog holes: the lygtemænd's false lanterns
        let mut marsh: Vec<Vec3> = Vec::new();
        for (k, h) in v.holes.iter().enumerate() {
            if Vec2::new(h.x - hero.x, h.z - hero.z).length_squared() > NEAR * NEAR {
                continue;
            }
            let fk = k as f32 * 1.37;
            let at = *h + Vec3::new((time * 0.5 + fk).sin() * 0.12, 0.85 + (time * 1.3 + fk).sin() * 0.1, (time * 0.4 + fk).cos() * 0.12);
            let pulse = 0.8 + 0.2 * (time * 2.7 + fk).sin();
            glow_orb(f, at, 0.9 * pulse, lin(MARSH), 0.5 * pulse);
            glow_orb(f, at, 0.35, lin(MARSH), 0.9);
            glow_orb(f, at, 0.08, lin(0xc8e4ff), 0.7);
            for j in 1..4 {
                let q = at + Vec3::new((time * 3.0 + j as f32 + fk).sin() * 0.06, -(j as f32) * 0.07, 0.0);
                f.additive.push(Instance::new(
                    Mat4::from_translation(q) * Mat4::from_scale(Vec3::splat(0.03)),
                    lin(MARSH),
                    0.5 / j as f32,
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
            marsh.push(at);
        }
        for p in nearest(&marsh, hero, 3) {
            f.lights.push(Light { pos: p, color: lin(MARSH) * 1.6, distance: 3.6 });
        }
        // ---- the hourglass's glow through the fog: the one thing to steer by
        let top = v.tower + Vec3::new(0.0, decor::HOURGLASS_Y, 0.0);
        let to = Vec3::new(v.tower.x - hero.x, 0.0, v.tower.z - hero.z);
        let dist = to.length();
        if dist > 7.0 {
            // Low and near enough to be in sight whichever way it lies (the camera looks down from the
            // south): a warm patch of the fog toward it.
            let dir = to / dist;
            let k = ((dist - 7.0) / 4.0).min(1.0);
            let pulse = 0.85 + 0.15 * (time * 0.8).sin();
            glow_orb(f, hero + dir * 5.0 + Vec3::new(0.0, 1.3, 0.0), 2.2, lin(SAND), 0.24 * pulse * k);
            glow_orb(f, hero + dir * 6.5 + Vec3::new(0.0, 1.8, 0.0), 3.4, lin(SAND), 0.13 * pulse * k);
        }
        f.lights.push(Light { pos: top, color: lin(SAND) * 5.0, distance: 9.0 });
        f.lights.push(Light { pos: bed_light, color: lin(CURTAIN) * 1.2, distance: 3.0 });
        // ---- mist lying over the bog about him
        let (hx, hz) = (hero.x.round() as i32, hero.z.round() as i32);
        let mut y = hz - MIST_RANGE;
        while y <= hz + MIST_RANGE {
            let mut x = hx - MIST_RANGE;
            while x <= hx + MIST_RANGE {
                let (gx, gz) = (x.div_euclid(2) * 2, y.div_euclid(2) * 2);
                if gx == x && gz == y {
                    let w = P::new(gx, gz);
                    let base = if mr::inside(w) { floor_y(r, w).max(wy) } else { wy };
                    for (k, (lift, alpha)) in [(0.18f32, 0.3f32), (0.5, 0.2)].into_iter().enumerate() {
                        let m = Mat4::from_translation(Vec3::new(gx as f32 + 0.5, base + lift, gz as f32 + 0.5))
                            * Mat4::from_scale(Vec3::new(2.0, 0.002, 2.0));
                        f.transparent[cube].push(Instance::new(
                            m,
                            lin(0xb8c0c8),
                            alpha,
                            Vec3::new(20.0 + k as f32, lift, 0.0),
                            SHADE_CLOUD,
                        ));
                    }
                }
                x += 2;
            }
            y += 2;
        }
        // ---- the snores: mist puffing from his nostrils
        if matches!(r.phase, Phase::Wander | Phase::Asking) {
            let since = (mr::SNORE_EVERY - r.snore) as f32;
            if since < 2.4 {
                for (k, n) in v.nostrils.iter().enumerate() {
                    for j in 0..4 {
                        let t = (since / 2.4 + j as f32 * 0.08).min(1.0);
                        let face = face_dir(r.head_world());
                        let at = *n + face * (t * 1.6) + Vec3::new((k as f32 - 0.5) * t * 0.5, t * 0.8 + j as f32 * 0.05, 0.0);
                        let m = Mat4::from_translation(at) * Mat4::from_scale(Vec3::splat(0.1 + t * 0.35));
                        f.transparent[MeshKind::Sphere as usize].push(Instance::new(
                            m,
                            lin(0xc8d0d8),
                            0.3 * (1.0 - t),
                            Vec3::ZERO,
                            SHADE_LIT,
                        ));
                    }
                }
            }
        }
        // ---- the yawn's wave, closing over the island toward the hole
        if r.wave < 1.0 {
            let t = r.wave as f32;
            let rad = (mr::PALM_R as f32 + 0.5) * (1.0 - t);
            if rad > 0.2 {
                let m = Mat4::from_translation(v.bed + Vec3::new(0.0, 0.12, 0.0))
                    * Mat4::from_rotation_x(-PI / 2.0)
                    * Mat4::from_scale(Vec3::new(rad, rad, 1.0));
                f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, lin(0xdce6ee), 0.55 * (1.0 - t), Vec3::ZERO, SHADE_UNLIT));
                for k in 0..20 {
                    let a = k as f32 / 20.0 * TAU + t;
                    let at = v.bed + Vec3::new(a.cos() * rad, STEP + 0.15 + (k % 3) as f32 * 0.08, a.sin() * rad);
                    f.additive.push(Instance::new(
                        Mat4::from_translation(at) * Mat4::from_scale(Vec3::splat(0.06)),
                        lin(0xe8f0f8),
                        0.5 * (1.0 - t),
                        Vec3::ZERO,
                        SHADE_UNLIT,
                    ));
                }
            }
        }
        // ---- bubbles rising in the mud near him
        for (k, m) in v.mud.iter().enumerate() {
            if Vec2::new(m.x - hero.x, m.z - hero.z).length_squared() > 100.0 {
                continue;
            }
            let t = (time * 0.6 + k as f32 * 0.37).fract();
            if t < 0.3 {
                let at = *m + Vec3::new((k as f32 * 1.3).sin() * 0.3, 0.0, (k as f32 * 2.1).cos() * 0.3);
                let sc = 0.04 + t * 0.25;
                let mm = Mat4::from_translation(at) * Mat4::from_scale(Vec3::new(sc, sc * 0.6, sc));
                f.opaque[MeshKind::Sphere as usize].push(Instance::new(mm, lin(MUD_L), 1.0, Vec3::ZERO, SHADE_WET));
            }
        }
    }

    /// A shortcut's carriage where it stands on its way: the rail cart, the ferry's raft, the lift's
    /// cage; stranded, its winding key turns as he winds it.
    fn emit_vehicle(&self, r: &Mire, ride: &mr::Ride, time: f32, f: &mut Frame) {
        let cube = MeshKind::Cube as usize;
        let at = r.v_world(ride.at());
        let p = Vec3::new(at.x as f32, 0.0, at.y as f32);
        let n = ride.path.len();
        let i = (ride.pos.floor() as usize).min(n - 1);
        let (a, b) = (r.to_world(ride.path[i.saturating_sub(1).min(n - 2)]), r.to_world(ride.path[(i + 1).min(n - 1)]));
        let dir = Vec3::new((b.x - a.x) as f32, 0.0, (b.y - a.y) as f32).normalize_or(Vec3::Z);
        let running = matches!(ride.state, RideState::Running { .. });
        let bob = if running { (time * 9.0).sin() * 0.02 } else { 0.0 };
        let root = Mat4::from_translation(p + Vec3::new(0.0, bob, 0.0)) * Mat4::from_rotation_y(dir.x.atan2(dir.z));
        let mut out = Vec::new();
        match ride.kind {
            RideKind::Cart => {
                rbx(&mut out, root, Vec3::new(0.0, 0.42, 0.0), Vec3::new(0.7, 0.42, 0.9), PLANK_D, SHADE_WOOD);
                rbx(&mut out, root, Vec3::new(0.0, 0.6, 0.0), Vec3::new(0.62, 0.1, 0.82), VOID, SHADE_LIT);
                for (x, z) in [(-0.3f32, -0.3f32), (0.3, -0.3), (-0.3, 0.3), (0.3, 0.3)] {
                    let spin = if running { time * 8.0 } else { 0.0 };
                    let m = root
                        * Mat4::from_translation(Vec3::new(x, 0.18, z))
                        * Mat4::from_rotation_x(spin)
                        * Mat4::from_scale(Vec3::new(0.06, 0.26, 0.26));
                    out.push(Instance::new(m, lin(IRON), 1.0, Vec3::ZERO, SHADE_IRON));
                }
                for z in [-0.42f32, 0.42] {
                    rbx(&mut out, root, Vec3::new(0.0, 0.45, z), Vec3::new(0.74, 0.06, 0.04), IRON, SHADE_IRON);
                }
            }
            RideKind::Ferry => {
                for k in 0..5 {
                    rbx(
                        &mut out,
                        root,
                        Vec3::new(-0.4 + k as f32 * 0.2, 0.0, 0.0),
                        Vec3::new(0.18, 0.1, 1.1),
                        if k % 2 == 0 { PLANK } else { PLANK_D },
                        SHADE_WOOD,
                    );
                }
                rbx(&mut out, root, Vec3::new(0.45, 0.4, 0.0), Vec3::new(0.08, 0.8, 0.08), PLANK_D, SHADE_WOOD);
                rbx(&mut out, root, Vec3::new(0.45, 0.85, 0.0), Vec3::new(0.12, 0.12, 0.3), BRONZE, SHADE_BRASS);
            }
            RideKind::Lift => {
                rbx(&mut out, root, Vec3::new(0.0, 0.03, 0.0), Vec3::new(0.9, 0.06, 0.9), IRON, SHADE_IRON);
                rbx(&mut out, root, Vec3::new(0.0, 1.45, 0.0), Vec3::new(0.9, 0.06, 0.9), IRON, SHADE_IRON);
                for (x, z) in [(-0.43f32, -0.43f32), (0.43, -0.43), (-0.43, 0.43), (0.43, 0.43), (0.0, -0.43), (0.0, 0.43), (-0.43, 0.0)] {
                    rbx(&mut out, root, Vec3::new(x, 0.74, z), Vec3::new(0.035, 1.42, 0.035), BRASS_D, SHADE_BRASS);
                }
                rbx(&mut out, root, Vec3::new(0.0, 2.0, 0.0), Vec3::new(0.04, 1.0, 0.04), IRON, SHADE_IRON);
                rbx(&mut out, root, Vec3::new(0.0, 2.48, 0.0), Vec3::new(0.3, 0.1, 0.2), IRON, SHADE_IRON);
            }
        }
        // The winding key: it turns as he winds, and sticks up gold while it waits to be wound.
        if ride.state == RideState::Stranded {
            let turn = ride.wound as f32 * 2.0;
            let key = root * Mat4::from_translation(Vec3::new(0.0, 1.0, -0.3)) * Mat4::from_rotation_z(turn);
            out.push(Instance::new(key * Mat4::from_scale(Vec3::new(0.06, 0.4, 0.06)), lin(BRASS), 1.0, lin(BRASS) * 0.2, SHADE_BRASS));
            out.push(Instance::new(
                key * Mat4::from_translation(Vec3::new(0.0, 0.25, 0.0)) * Mat4::from_scale(Vec3::new(0.36, 0.1, 0.06)),
                lin(BRASS),
                1.0,
                lin(BRASS) * 0.2,
                SHADE_BRASS,
            ));
            glow_orb(f, root.transform_point3(Vec3::new(0.0, 1.3, -0.3)), 0.4, lin(SAND), 0.08 + 0.04 * (time * 3.0).sin());
        }
        f.opaque[cube].extend(out);
    }

    /// The hourglass on its tower: a bronze frame, two bulbs of smoked glass, the sand frozen in its
    /// fall; once it turns over, the sand runs.
    fn emit_hourglass(&self, v: &MireView, r: &Mire, time: f32, f: &mut Frame) {
        let cube = MeshKind::Cube as usize;
        let centre = v.tower + Vec3::new(0.0, decor::HOURGLASS_Y, -0.25);
        let since = r.turned_at.map(|t| (r.clock - t) as f32);
        let flip = since.map(|t| {
            let k = (t / 1.6).clamp(0.0, 1.0);
            k * k * (3.0 - 2.0 * k) * PI
        });
        let root = Mat4::from_translation(centre) * Mat4::from_rotation_z(flip.unwrap_or(0.0));
        // How much sand is above: frozen half-fallen until it turns; then its sixty seconds.
        let above = match (r.phase, flip) {
            (Phase::Sand { left, .. }, Some(_)) => (left / mr::SAND).clamp(0.0, 1.0) as f32,
            (Phase::Done { .. }, _) => 0.0,
            _ => 0.55,
        };
        // (Turned over, the sand that was below is above: the glass is drawn in its own frame.)
        let (upper, lower) = (above, 1.0 - above);
        let mut out = Vec::new();
        for s in [-1.0f32, 1.0] {
            rbx(&mut out, root, Vec3::new(0.0, s * 1.42, 0.0), Vec3::new(1.5, 0.14, 1.5), BRONZE, SHADE_BRASS);
            rbx(&mut out, root, Vec3::new(0.0, s * 1.52, 0.0), Vec3::new(1.2, 0.08, 1.2), SLATE_D, SHADE_LIT);
        }
        for k in 0..3 {
            let a = k as f32 / 3.0 * TAU + 0.5;
            rbx(&mut out, root, Vec3::new(a.cos() * 0.62, 0.0, a.sin() * 0.62), Vec3::new(0.1, 2.8, 0.1), BRONZE, SHADE_BRASS);
        }
        // The sand: a heap in the lower bulb, what is left in the upper, the stream between.
        let sand = lin(SAND);
        for k in 0..4 {
            let t = k as f32 / 4.0;
            let w = (0.75 - t * 0.5) * lower.sqrt().max(0.15);
            let m = root
                * Mat4::from_scale_rotation_translation(
                    Vec3::new(w, 0.16, w),
                    Quat::IDENTITY,
                    Vec3::new(0.0, -1.25 + t * 0.18 * lower.max(0.2) * 3.0, 0.0),
                );
            out.push(Instance::new(m, sand, 1.0, sand * 0.35, SHADE_LIT));
        }
        if upper > 0.02 {
            for k in 0..4 {
                let t = k as f32 / 4.0;
                let w = 0.25 + t * 0.45 * upper.sqrt();
                let m = root
                    * Mat4::from_scale_rotation_translation(
                        Vec3::new(w, 0.16, w),
                        Quat::IDENTITY,
                        Vec3::new(0.0, 0.25 + t * upper * 0.9, 0.0),
                    );
                out.push(Instance::new(m, sand, 1.0, sand * 0.35, SHADE_LIT));
            }
        }
        let running = matches!(r.phase, Phase::Sand { .. }) && flip.is_some_and(|a| a >= PI - 0.01);
        for k in 0..12 {
            let base_t = k as f32 / 12.0;
            // Frozen: grains hang in mid-fall. Running: they pour down.
            let t = if running { (base_t + time * 1.6).fract() } else { base_t };
            let m = root
                * Mat4::from_scale_rotation_translation(
                    Vec3::splat(0.05),
                    Quat::IDENTITY,
                    Vec3::new(((k * 7) % 3) as f32 * 0.01, 0.15 - t * 1.25, 0.0),
                );
            out.push(Instance::new(m, sand, 1.0, sand * 0.8, SHADE_UNLIT));
        }
        f.opaque[cube].extend(out);
        // The bulbs of smoked glass.
        for s in [-1.0f32, 1.0] {
            for k in 0..9 {
                let t = k as f32 / 8.0;
                let w = 0.22 + (PI * (0.12 + t * 0.88)).sin() * 0.9;
                let y = s * (0.08 + t * 1.24);
                let turn = if k % 2 == 0 { 0.0 } else { PI / 4.0 };
                let m = root
                    * Mat4::from_scale_rotation_translation(Vec3::new(w, 0.15, w), Quat::from_rotation_y(turn), Vec3::new(0.0, y, 0.0));
                f.transparent[cube].push(Instance::new(m, lin(GLASS), 0.16, lin(SAND) * 0.04, SHADE_LIT));
            }
        }
        let pulse = 0.85 + 0.15 * (time * 1.1).sin();
        glow_orb(f, centre, 1.7, lin(SAND), 0.16 * pulse);
        glow_orb(f, centre + Vec3::new(0.0, -0.9, 0.0), 0.7, lin(SAND), 0.25 * pulse);
    }
}

/// A tuft of grass drawn this frame (on the tilting palm).
fn tuft_frame(f: &mut Frame, at: Vec3, seed: f32) {
    let mut out = Vec::new();
    tuft(&mut out, at, seed, GRASS_D);
    f.opaque[MeshKind::Cube as usize].extend(out);
}

/// Smooth value noise in 0..1 over the bog.
fn smooth(x: f32, y: f32) -> f32 {
    let (i, j) = (x.floor(), y.floor());
    let (fx, fy) = (x - i, y - j);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let h = |a: f32, b: f32| hash(a as f64 * 0.917 + 11.0, b as f64 * 1.311 + 5.0) as f32;
    let top = h(i, j) + (h(i + 1.0, j) - h(i, j)) * sx;
    let bot = h(i, j + 1.0) + (h(i + 1.0, j + 1.0) - h(i, j + 1.0)) * sx;
    top + (bot - top) * sy
}

/// The ground's colour at a tile: between `a` and `b` in broad soft patches, a little uneven tile to tile.
fn earth(a: u32, b: u32, x: i32, y: i32, seed: f32) -> Vec3 {
    let n = smooth(x as f32 * 0.21 + seed, y as f32 * 0.21 - seed) * 0.75 + smooth(x as f32 * 0.6, y as f32 * 0.6 + seed) * 0.25;
    let j = 0.96 + hash(x as f64 * 2.7, y as f64 * 3.1) as f32 * 0.08;
    lin(a).lerp(lin(b), n.clamp(0.0, 1.0)) * j
}

/// Which way the giant's face looks from his head: toward his own hand.
fn face_dir(head: P) -> Vec3 {
    Vec3::new((mr::CENTRE - head.x) as f32, 0.0, (mr::CENTRE - head.y) as f32).normalize_or(Vec3::Z)
}

/// The realm's sky, fog and light: flat, grey and shadowless, the fog close about him (closer as
/// he grows drowsy), surging white as the land turns.
pub(super) fn atmosphere(_v: &MireView, r: &Mire, _s: &SceneState) -> Atmosphere {
    let surge = r.surge() as f32;
    let drowse = ((r.torpor - 40.0) / 60.0).clamp(0.0, 1.0) as f32;
    let sand = r.in_sand();
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let mut a = Atmosphere {
        background: mix_hex(FOG, SURGE, surge),
        fog_near: lerp(lerp(7.5, 5.5, drowse), 0.5, surge),
        fog_far: lerp(lerp(17.5, 13.0, drowse), 3.5, surge),
        // A flat mist-teal grey from everywhere, a little umber off the peat.
        hemi_sky: lin(0xb4c6c2) * 0.85,
        hemi_ground: lin(0x5c5a50) * 0.8,
        ambient: lin(0x6c7674) * 0.75,
        sun_dir: Vec3::new(0.15, 1.0, 0.25).normalize(),
        sun: lin(0xd8dde2) * 0.22,
        exposure: lerp(1.15, 0.85, drowse),
        night_sky: 0.0,
        bloom: 0.85,
        moon_view: Vec3::new(0.0, 0.4, -1.0).normalize(),
        cloud_tint: lin(0x84908e),
    };
    if sand {
        a.hemi_sky = a.hemi_sky.lerp(lin(SAND) * 0.8, 0.15);
    }
    a
}
