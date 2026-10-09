//! Wrath's court, **the Anvil of Wrath** (`logic::court::wrath` lays it out), to the art direction:
//! the board's palette (charcoal, umber, copper, amber) with Wrath's red as its accent, under the
//! light of the fire beneath the floor.
//!
//! A round hall of black basalt, its floor cracked by seams that glow from the fire under it and
//! scorched black round them, ash drifted against the wall, and the weapons of the war's dead stuck
//! in the ground where they fell: swords, spears, axes, a shield face down, a helm. The wall is
//! rough basalt laid in heavy courses, veins of fire running up it, and war banners hang from it on
//! iron poles, red and black with the raven sigil, burning from their tattered hems. Four broken
//! stone pillars stand about the floor, two tiles by two, their drums cracked and scorched (Amon
//! is baited into them). At the north end a stepped dais carries the great anvil, a blade half
//! forged and glowing on its face, chains about its foot; behind it his throne of black iron and
//! bone burns, flanked by two fire bowls, his own banner of the raven and the wolf over it. The
//! gate he came in by is Wrath's portal in the south wall.
//!
//! Each frame: the fires burn and light the hall, embers rise from the seams, and Amon wears his
//! fire: in his throat as he breathes, along his charge, about him as his rage grows, a ring of
//! sparks while he stands dazed. His rift is drawn here too (`emit_rift`): the crack racing out
//! from his axe, the seam glowing as its fuse burns, the lava bursting up and lying in pools.

use super::*;
use crate::logic::combat::prince::wrath::RAGE_MAX;
use crate::logic::combat::prince::{Trap, COIN_FLIGHT};
use crate::logic::combat::{MoveId, Phase, Reel, State};
use crate::logic::court::wrath::{self as plan, Piece, ANVIL, CENTRE, DAIS, GATE, PILLARS, RADIUS, THRONE};
use crate::logic::hub::Gate;

pub(super) mod pal {
    pub const BASALT: u32 = 0x221d1c;
    pub const BASALT_L: u32 = 0x2e2826;
    pub const BASALT_D: u32 = 0x161212;
    pub const SCORCH: u32 = 0x0e0a0a;
    pub const ASH: u32 = 0x5e5652;
    pub const ASH_D: u32 = 0x463f3c;
    pub const STONE: u32 = 0x6e655c;
    pub const STONE_L: u32 = 0x857a6e;
    pub const STONE_D: u32 = 0x4a433d;
    pub const IRON: u32 = 0x34343a;
    pub const IRON_L: u32 = 0x585a62;
    pub const IRON_D: u32 = 0x222226;
    pub const STEEL: u32 = 0x9aa0a8;
    pub const WOOD: u32 = 0x3e2a1c;
    pub const WOOD_C: u32 = 0x1e1410;
    pub const LEATHER: u32 = 0x4a2e1c;
    pub const RED: u32 = 0xa01a14;
    pub const RED_D: u32 = 0x6a100c;
    pub const BLACK: u32 = 0x141010;
    pub const GOLD: u32 = 0xc89a3a;
    pub const BONE: u32 = 0xd8ccb0;
    pub const BONE_D: u32 = 0x9a8c70;
    pub const LAVA_DEEP: u32 = 0x7a2408;
    pub const LAVA: u32 = 0xe8541a;
    pub const LAVA_HOT: u32 = 0xf0a448;
    pub const FLAME: u32 = 0xffb050;
    pub const FLAME_TIP: u32 = 0xff5a18;
    pub const EMBER: u32 = 0xff6a20;
    pub const AMBER: u32 = 0xffa050;
}
use pal::*;

/// The voxel materials' shading modes, for plain boxes: hammered iron, cloth, ivory, dressed stone.
const SHADE_IRON: f32 = 6.0;
const SHADE_CLOTH: f32 = 8.0;
const SHADE_BONE: f32 = 11.0;
const SHADE_STONE: f32 = 18.0;
/// The wall stands this tall (its top jagged about it).
const WALL_H: f32 = 2.5;

/// Dark and hot: the fire under the floor lights the hall from below, the banners and the throne
/// burn amber, the smoke closes in; no sky.
pub fn wrath_atmosphere() -> Atmosphere {
    Atmosphere {
        background: 0x0a0505,
        fog_near: 16.0,
        fog_far: 44.0,
        hemi_sky: lin(0x5a3a30) * 0.45,
        hemi_ground: lin(0x8a3412) * 0.6,
        ambient: lin(0x6a4434) * 0.62,
        sun_dir: Vec3::new(0.3, 1.0, 0.45).normalize(),
        sun: lin(0xffc08a) * 0.75,
        exposure: 1.25,
        night_sky: 0.0,
        bloom: 0.9,
        moon_view: Vec3::new(0.0, 0.3, -1.0).normalize(),
        cloud_tint: lin(0x2a0c06),
    }
}

/// Its ground haze: the smoke of the fires, thick and low.
pub const HAZE: (f32, f32) = (0.018, 1.7);

/// What the view keeps of the court for its moving parts: its fires (where, how big), the glowing
/// points of its seams, its gate, the throne and the anvil's face.
pub struct AnvilView {
    pub fires: Vec<(Vec3, f32)>,
    pub seams: Vec<Vec3>,
    pub gate: Gate,
    pub throne: Vec3,
    pub anvil: Vec3,
}

fn cube(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot_y: f32, color: Vec3, shade: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(scale, Quat::from_rotation_y(rot_y), pos), color, 1.0, Vec3::ZERO, shade));
}

/// A box turned by `rot`, lit from within by `glow`.
fn glowing(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot: Quat, color: Vec3, glow: Vec3, shade: f32) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(scale, rot, pos), color, 1.0, glow, shade));
}

/// A rod from `a` to `b`, `w` thick.
fn rod(out: &mut Vec<Instance>, a: Vec3, b: Vec3, w: f32, color: Vec3, glow: Vec3, shade: f32) {
    let d = b - a;
    let len = d.length().max(1e-4);
    let rot = Quat::from_rotation_arc(Vec3::Y, d / len);
    out.push(Instance::new(Mat4::from_scale_rotation_translation(Vec3::new(w, len, w), rot, (a + b) * 0.5), color, 1.0, glow, shade));
}

fn basalt(h: f64) -> Vec3 {
    let set = [BASALT, BASALT, BASALT_L, BASALT_D];
    lin(set[(h * 4.0) as usize % 4]) * (0.92 + h as f32 * 0.14)
}

/// The `k`-th draw, 0 to 1, from `seed`.
fn rnd(seed: f64, k: u32) -> f32 {
    hash(seed * 1.31 + k as f64 * 7.77 + 0.5, seed * 0.67 - k as f64 * 3.19 + 11.0) as f32
}

/// The seams of fire under the floor: cracks wandering out from the anvil's dais and from the four
/// pillars' feet, branching, each a polyline of points on the floor.
fn seam_paths() -> Vec<Vec<Vec2>> {
    let mut out = Vec::new();
    let open = |p: Vec2| {
        let (x, y) = (p.x.round() as i32, p.y.round() as i32);
        plan::in_hall(x, y) && !matches!(plan::piece(x, y), Some(Piece::Pillar | Piece::Anvil))
    };
    let walk = |from: Vec2, heading: f32, steps: usize, seed: f64, out: &mut Vec<Vec<Vec2>>| {
        let mut path = vec![from];
        let mut p = from;
        let mut a = heading;
        for k in 0..steps {
            a += (rnd(seed, k as u32) - 0.5) * 0.9;
            let next = p + Vec2::new(a.cos(), a.sin()) * 0.55;
            if !open(next) {
                break;
            }
            p = next;
            path.push(p);
        }
        if path.len() > 2 {
            out.push(path);
        }
    };
    // From the anvil's foot, fanning south over the floor, each with a branch.
    let foot = Vec2::new(ANVIL.0 as f32 + 1.0, ANVIL.2 as f32 + 0.8);
    for k in 0..7 {
        let a = PI * 0.5 + (k as f32 - 3.0) * 0.36;
        let seed = 11.0 + k as f64 * 3.7;
        let steps = 14 + (rnd(seed, 99) * 12.0) as usize;
        walk(foot + Vec2::new((k as f32 - 3.0) * 0.4, 0.0), a, steps, seed, &mut out);
        let mid = foot + Vec2::new(a.cos(), a.sin()) * (3.0 + rnd(seed, 50) * 3.0);
        walk(mid, a + if k % 2 == 0 { 0.8 } else { -0.8 }, 8, seed + 0.5, &mut out);
    }
    // From each pillar's foot, a few short cracks out across the floor.
    for (i, &p) in PILLARS.iter().enumerate() {
        let m = plan::pillar_middle(p);
        let c = Vec2::new(m.0 as f32, m.1 as f32);
        for k in 0..4 {
            let a = k as f32 * PI * 0.5 + 0.4 + i as f32;
            walk(c + Vec2::new(a.cos(), a.sin()) * 1.3, a, 6 + k * 2, 70.0 + i as f64 * 9.0 + k as f64, &mut out);
        }
    }
    // A broken ring of them about the middle, where the fighting is thickest.
    for k in 0..9 {
        if k % 3 == 2 {
            continue;
        }
        let a = k as f32 * TAU / 9.0;
        let p = Vec2::new(CENTRE.0 as f32, CENTRE.1 as f32) + Vec2::new(a.cos(), a.sin()) * 4.2;
        walk(p, a + PI * 0.5, 4, 120.0 + k as f64, &mut out);
    }
    out
}

impl WorldView {
    /// The Anvil of Wrath's static scene.
    pub(super) fn build_wrath_court(&mut self, grid: &Grid) {
        let floor_tile = |x: i32, y: i32| plan::in_hall(x, y);
        let gate_tile = |x: i32, y: i32| x == GATE.0 && y == GATE.1;
        let c = Vec3::new(CENTRE.0 as f32, 0.0, CENTRE.1 as f32);
        let mut fires: Vec<(Vec3, f32)> = Vec::new();

        // ---- the floor: slabs of basalt, ash drifted against the wall, a scorched heart
        for y in 0..grid.height {
            for x in 0..grid.width {
                if !floor_tile(x, y) && !gate_tile(x, y) {
                    continue;
                }
                let (xf, yf) = (x as f32, y as f32);
                let h = hash(x as f64 * 1.9 + 5.0, y as f64 * 2.7 + 1.0);
                let out = &mut self.static_cubes;
                let tilt = (h as f32 - 0.5) * 0.04;
                out.push(Instance::new(
                    Mat4::from_scale_rotation_translation(
                        Vec3::new(0.97, 0.14, 0.97),
                        Quat::from_rotation_x(tilt) * Quat::from_rotation_z(-tilt * 0.7),
                        Vec3::new(xf, -0.07 + (h as f32 - 0.5) * 0.025, yf),
                    ),
                    basalt(h),
                    1.0,
                    Vec3::ZERO,
                    SHADE_STONE,
                ));
                let d = Vec2::new(xf - c.x, yf - c.z).length();
                if d > RADIUS as f32 - 1.6 && hash(y as f64 * 3.1, x as f64 * 0.7) > 0.35 {
                    // Ash drifted against the wall, in soft heaps.
                    let k = (d - (RADIUS as f32 - 1.6)) / 1.6;
                    let a = hash(x as f64 * 7.0, y as f64 * 5.0) as f32 * TAU;
                    cube(
                        out,
                        Vec3::new(xf, 0.01 + k * 0.03, yf),
                        Vec3::new(0.8, 0.03 + k * 0.08, 0.6),
                        a,
                        lin(ASH) * (0.85 + k * 0.2),
                        SHADE_STONE,
                    );
                    cube(
                        out,
                        Vec3::new(xf + 0.2, 0.03 + k * 0.06, yf - 0.15),
                        Vec3::new(0.45, 0.04 + k * 0.06, 0.4),
                        a + 0.7,
                        lin(ASH_D),
                        SHADE_STONE,
                    );
                } else if h > 0.86 {
                    // A slab cracked across, its pieces a little apart.
                    let a = hash(x as f64 * 5.0, y as f64 * 9.0) as f32 * PI;
                    cube(out, Vec3::new(xf, 0.005, yf), Vec3::new(0.9, 0.006, 0.03), a, lin(SCORCH), SHADE_LIT);
                }
            }
        }
        // The scorched heart of the ring: a great black burn, a sigil of fire set in it.
        for k in 0..36 {
            let a = k as f32 * TAU / 36.0;
            let r = 2.6 + 0.5 * (a * 3.0).sin();
            let p = c + Vec3::new(a.cos() * r * 0.55, 0.004, a.sin() * r * 0.55);
            cube(&mut self.static_cubes, p, Vec3::new(r * 0.9, 0.006, 0.5), -a, lin(SCORCH), SHADE_LIT);
        }
        // The raven's sigil: a ring, and the three claws of the wolf through it.
        for k in 0..28 {
            let a = k as f32 * TAU / 28.0;
            let p = c + Vec3::new(a.cos() * 1.5, 0.008, a.sin() * 1.5);
            glowing(
                &mut self.static_cubes,
                p,
                Vec3::new(0.34, 0.01, 0.06),
                Quat::from_rotation_y(-a + PI / 2.0),
                lin(LAVA_DEEP),
                lin(LAVA) * 0.55,
                SHADE_LIT,
            );
        }
        for k in -1..=1 {
            let o = c + Vec3::new(k as f32 * 0.42, 0.009, 0.0);
            glowing(
                &mut self.static_cubes,
                o,
                Vec3::new(0.07, 0.01, 2.4),
                Quat::from_rotation_y(k as f32 * 0.22),
                lin(LAVA_DEEP),
                lin(LAVA) * 0.5,
                SHADE_LIT,
            );
        }

        // ---- the seams of fire: a glowing crack in a scorched lip, and points along them for their light
        let mut seams = Vec::new();
        for (i, path) in seam_paths().iter().enumerate() {
            for (k, w) in path.windows(2).enumerate() {
                let (a, b) = (Vec3::new(w[0].x, 0.0, w[0].y), Vec3::new(w[1].x, 0.0, w[1].y));
                let d = b - a;
                let len = d.length();
                let yaw = -d.z.atan2(d.x);
                let mid = (a + b) * 0.5;
                let taper = 1.0 - k as f32 / path.len() as f32 * 0.6;
                let out = &mut self.static_cubes;
                cube(out, mid + Vec3::new(0.0, 0.004, 0.0), Vec3::new(len + 0.06, 0.008, 0.2 * taper), yaw, lin(SCORCH), SHADE_LIT);
                let hot = 0.35 + 0.65 * taper;
                glowing(
                    out,
                    mid + Vec3::new(0.0, 0.009, 0.0),
                    Vec3::new(len + 0.04, 0.008, 0.055 * taper + 0.02),
                    Quat::from_rotation_y(yaw),
                    lin(LAVA_DEEP),
                    lin(LAVA).lerp(lin(LAVA_HOT), taper * 0.5) * hot,
                    SHADE_LIT,
                );
                if k % 7 == 3 && i % 2 == 0 {
                    seams.push(mid + Vec3::new(0.0, 0.15, 0.0));
                }
            }
        }

        // ---- the wall: rough basalt in heavy courses, its top jagged, veins of fire up it
        for (x, y, t) in grid.cells() {
            if t != Tile::Rock || floor_tile(x, y) || gate_tile(x, y) {
                continue;
            }
            let near =
                [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)].iter().any(|&(dx, dy)| floor_tile(x + dx, y + dy));
            if !near {
                continue;
            }
            if y == GATE.1 && (x - GATE.0).abs() <= 1 {
                continue; // the portal's own towers
            }
            let start = self.static_cubes.len();
            let (xf, yf) = (x as f32, y as f32);
            let h = hash(x as f64 * 3.3, y as f64 * 4.1);
            let top = WALL_H + (h as f32 - 0.5) * 0.7;
            let out = &mut self.static_cubes;
            cube(out, Vec3::new(xf, 0.15, yf), Vec3::new(1.04, 0.3, 1.04), 0.0, lin(BASALT_D), SHADE_STONE);
            let mut yb = 0.3;
            let mut course = 0;
            while yb < top - 0.05 {
                let hh = (0.42 + rnd(h, course) * 0.3).min(top - yb);
                let off = (rnd(h, course + 10) - 0.5) * 0.12;
                let col = basalt(rnd(h, course + 20) as f64) * 0.95;
                cube(
                    out,
                    Vec3::new(xf + off, yb + hh / 2.0, yf - off),
                    Vec3::new(1.0 - rnd(h, course + 30) * 0.08, hh - 0.02, 1.0),
                    off * 0.6,
                    col,
                    SHADE_STONE,
                );
                yb += hh;
                course += 1;
            }
            // The jagged crown: a broken block or two standing proud.
            for k in 0..2 {
                if rnd(h, 40 + k) > 0.45 {
                    let s = 0.3 + rnd(h, 42 + k) * 0.3;
                    let o = Vec3::new((rnd(h, 44 + k) - 0.5) * 0.5, top + s * 0.4, (rnd(h, 46 + k) - 0.5) * 0.5);
                    cube(out, Vec3::new(xf, 0.0, yf) + o, Vec3::new(s, s * 0.9, s * 1.1), rnd(h, 48 + k) * PI, lin(BASALT_D), SHADE_STONE);
                }
            }
            // A vein of fire up the face toward the hall, now and then.
            let toward = Vec2::new(c.x - xf, c.z - yf).normalize_or_zero();
            if h > 0.72 {
                let face = Vec3::new(xf, 0.0, yf) + Vec3::new(toward.x, 0.0, toward.y) * 0.52;
                let mut p = face + Vec3::new(0.0, 0.05, 0.0);
                let side = Vec3::new(-toward.y, 0.0, toward.x);
                for k in 0..5 {
                    let q = p + Vec3::new(0.0, 0.4 + rnd(h, 60 + k) * 0.2, 0.0) + side * (rnd(h, 70 + k) - 0.5) * 0.3;
                    let q = Vec3::new(q.x, q.y.min(top - 0.1), q.z);
                    rod(out, p, q, 0.05, lin(LAVA_DEEP), lin(LAVA) * (0.8 - k as f32 * 0.12), SHADE_LIT);
                    p = q;
                }
            }
            let len = self.static_cubes.len() - start;
            self.rock_index.insert(tile_key(x, y), (start, len));
            let shades: Vec<f32> = self.static_cubes[start..start + len].iter().map(|i| i.emissive[3]).collect();
            self.trees.push(Tree {
                range: (start, len),
                base: Vec3::new(xf, 0.0, yf),
                radius: 0.72,
                height: top + 0.3,
                fade: 1.0,
                alive: true,
                shades: Some(shades),
            });
        }

        // ---- the war banners on the wall, burning from their hems (none over the gate or the throne)
        for k in 0..12 {
            let a = k as f32 * TAU / 12.0 + TAU / 24.0;
            let dir = Vec3::new(a.cos(), 0.0, a.sin());
            if dir.z > 0.8 || dir.z < -0.85 {
                continue;
            }
            let at = c + dir * (RADIUS as f32 + 0.35);
            let fire = self.build_war_banner(at, -dir, k as f64);
            fires.push((fire, 0.5));
        }

        // ---- the broken pillars
        for (i, &p) in PILLARS.iter().enumerate() {
            let m = plan::pillar_middle(p);
            let base = Vec3::new(m.0 as f32, 0.0, m.1 as f32);
            let start = self.static_cubes.len();
            let tall = [3.4f32, 1.7, 2.5, 1.2][i % 4];
            self.build_broken_pillar(base, tall, i as f64);
            let len = self.static_cubes.len() - start;
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                self.rock_index.insert(tile_key(p.0 + dx, p.1 + dy), (start, len));
            }
            let shades: Vec<f32> = self.static_cubes[start..start + len].iter().map(|i| i.emissive[3]).collect();
            self.trees.push(Tree {
                range: (start, len),
                base,
                radius: 1.0,
                height: tall + 0.2,
                fade: 1.0,
                alive: true,
                shades: Some(shades),
            });
        }

        // ---- the weapons of the dead, stuck where they fell
        let mut placed = 0;
        for k in 0..400u32 {
            if placed >= 30 {
                break;
            }
            let x = 3 + (rnd(7.0, k) * 27.0) as i32;
            let y = 3 + (rnd(13.0, k) * 27.0) as i32;
            if !floor_tile(x, y) || plan::piece(x, y).is_some() || (x == GATE.0 && y > 22) || (x - 16).abs() <= 1 && (7..=26).contains(&y) {
                continue;
            }
            let near_pillar = PILLARS.iter().any(|&p| (x - p.0).abs() <= 2 && (y - p.1).abs() <= 2);
            if near_pillar && rnd(3.0, k) > 0.4 {
                continue;
            }
            let at = Vec3::new(x as f32 + (rnd(5.0, k) - 0.5) * 0.6, 0.0, y as f32 + (rnd(9.0, k) - 0.5) * 0.6);
            self.build_fallen_weapon(at, rnd(11.0, k) as f64 * 97.0 + k as f64);
            placed += 1;
        }

        // ---- the dais, the anvil, the throne and its fires
        let anvil = self.build_anvil_dais();
        let throne = self.build_wrath_throne();
        for s in [-1.0f32, 1.0] {
            let bowl = Vec3::new(THRONE.0 as f32 + s * 3.6, 0.0, THRONE.1 as f32 + 0.5);
            fires.push((self.build_fire_bowl(bowl), 0.9));
        }
        fires.push((throne + Vec3::new(0.0, 1.9, -0.3), 1.2));
        // Fire baskets on the wall between the banners, low.
        for k in 0..6 {
            let a = PI * 0.5 + (k as f32 - 2.5) * 0.62 + if k >= 3 { PI } else { 0.0 };
            let dir = Vec3::new(a.cos(), 0.0, a.sin());
            if dir.z > 0.75 {
                continue;
            }
            let at = c + dir * (RADIUS as f32 + 0.45) + Vec3::new(0.0, 1.3, 0.0);
            fires.push((self.build_fire_basket(at, -dir), 0.45));
        }

        // ---- the gate: Wrath's portal in the south wall, looking into the hall
        let gate = Gate { tile: P::new(GATE.0, GATE.1), facing: Dir::Up, sin: crate::logic::sins::Sin::Wrath };
        self.build_court_portal(&gate);
        self.anvil = Some(AnvilView { fires, seams, gate, throne, anvil });
    }

    /// A war banner hanging from an iron pole thrust out of the wall at `at`, facing `into` the hall:
    /// red cloth and black, the raven sigil, a gold fringe, its hem tattered and burning. Returns
    /// where its fire burns.
    fn build_war_banner(&mut self, at: Vec3, into: Vec3, seed: f64) -> Vec3 {
        let out = &mut self.static_cubes;
        let yaw = -into.z.atan2(into.x) + PI / 2.0;
        let rot = Quat::from_rotation_y(yaw);
        let side = Vec3::new(into.z, 0.0, -into.x);
        let top = at + Vec3::new(0.0, 2.35, 0.0);
        // The pole out of the wall, a cross-bar, a spike at its end.
        rod(out, top - into * 0.15, top + into * 0.55, 0.06, lin(IRON), Vec3::ZERO, SHADE_IRON);
        rod(out, top + into * 0.4 - side * 0.42, top + into * 0.4 + side * 0.42, 0.045, lin(IRON_L), Vec3::ZERO, SHADE_IRON);
        for s in [-1.0f32, 1.0] {
            glowing(
                out,
                top + into * 0.4 + side * s * 0.45,
                Vec3::splat(0.07),
                rot * Quat::from_rotation_x(PI / 4.0),
                lin(IRON_D),
                Vec3::ZERO,
                SHADE_IRON,
            );
        }
        // The cloth: red with black panels, hanging a little out from the wall, its hem in tatters.
        let hang = top + into * 0.4;
        let length = 1.5 + rnd(seed, 1) * 0.3;
        let strips = 6;
        for k in 0..strips {
            let u = (k as f32 + 0.5) / strips as f32 - 0.5;
            let tatter = rnd(seed, 10 + k) * 0.35;
            let l = length - tatter;
            let p = hang + side * u * 0.8 - Vec3::new(0.0, l / 2.0 + 0.03, 0.0) + into * (0.02 * (k % 2) as f32);
            let col = if k == 0 || k == strips - 1 { lin(BLACK) } else { lin(RED) * (0.9 + rnd(seed, 20 + k) * 0.15) };
            out.push(Instance::new(
                Mat4::from_scale_rotation_translation(Vec3::new(0.8 / strips as f32 + 0.005, l, 0.03), rot, p),
                col,
                1.0,
                Vec3::ZERO,
                SHADE_CLOTH,
            ));
            // The charred hem.
            let hem = hang + side * u * 0.8 - Vec3::new(0.0, l + 0.02, 0.0);
            glowing(out, hem, Vec3::new(0.8 / strips as f32, 0.06, 0.035), rot, lin(BLACK), lin(EMBER) * 0.5, SHADE_LIT);
        }
        // The gold fringe at the top, and the raven's sigil: a black bird, its wings spread.
        cube(out, hang - Vec3::new(0.0, 0.05, 0.0) + into * 0.03, Vec3::new(0.84, 0.06, 0.04), yaw, lin(GOLD), SHADE_IRON);
        let sig = hang - Vec3::new(0.0, 0.62, 0.0) + into * 0.035;
        let bird = |o: Vec3, s: Vec3, r: f32, out: &mut Vec<Instance>| {
            out.push(Instance::new(
                Mat4::from_scale_rotation_translation(s, rot * Quat::from_rotation_z(r), sig + side * o.x + Vec3::new(0.0, o.y, 0.0)),
                lin(BLACK),
                1.0,
                Vec3::ZERO,
                SHADE_CLOTH,
            ));
        };
        bird(Vec3::ZERO, Vec3::new(0.1, 0.26, 0.02), 0.0, out);
        bird(Vec3::new(0.0, 0.17, 0.0), Vec3::new(0.08, 0.08, 0.02), 0.0, out);
        for s in [-1.0f32, 1.0] {
            bird(Vec3::new(s * 0.14, 0.05, 0.0), Vec3::new(0.26, 0.06, 0.02), s * 0.45, out);
            bird(Vec3::new(s * 0.24, -0.05, 0.0), Vec3::new(0.16, 0.05, 0.02), s * 0.9, out);
        }
        hang - Vec3::new(0.0, length - 0.15, 0.0)
    }

    /// A broken stone pillar two tiles square: a stepped plinth, a shaft of cracked, scorched drums
    /// broken off at `tall`, a fallen drum and rubble at its foot (all inside its two tiles).
    fn build_broken_pillar(&mut self, base: Vec3, tall: f32, seed: f64) {
        let out = &mut self.static_cubes;
        let stone = |k: u32| lin(STONE) * (0.85 + rnd(seed, k) * 0.25);
        cube(out, base + Vec3::new(0.0, 0.12, 0.0), Vec3::new(1.9, 0.24, 1.9), 0.0, lin(STONE_D), SHADE_STONE);
        cube(out, base + Vec3::new(0.0, 0.33, 0.0), Vec3::new(1.6, 0.18, 1.6), 0.0, stone(1), SHADE_STONE);
        cube(out, base + Vec3::new(0.0, 0.47, 0.0), Vec3::new(1.35, 0.1, 1.35), PI / 4.0, lin(STONE_L), SHADE_STONE);
        // The shaft: drums of an eight-sided section, each a little out of true.
        let mut y = 0.52;
        let mut k = 0;
        while y < tall - 0.1 {
            let hh = (0.42 + rnd(seed, 10 + k) * 0.12).min(tall - y);
            let off = Vec3::new((rnd(seed, 20 + k) - 0.5) * 0.05, 0.0, (rnd(seed, 30 + k) - 0.5) * 0.05);
            let col = stone(40 + k);
            for r in [0.0, PI / 4.0] {
                cube(
                    out,
                    base + off + Vec3::new(0.0, y + hh / 2.0, 0.0),
                    Vec3::new(1.05, hh - 0.025, 1.05),
                    r + rnd(seed, 50 + k) * 0.1,
                    col,
                    SHADE_STONE,
                );
            }
            // Fluting: dark grooves down each face.
            for f in 0..8 {
                let a = f as f32 * PI / 4.0 + PI / 8.0;
                let p = base + off + Vec3::new(a.cos() * 0.53, y + hh / 2.0, a.sin() * 0.53);
                cube(out, p, Vec3::new(0.05, hh - 0.04, 0.05), -a, lin(STONE_D) * 0.8, SHADE_STONE);
            }
            y += hh;
            k += 1;
        }
        // The break: jagged chunks standing up from the top drum.
        for j in 0..4 {
            let a = j as f32 * PI * 0.5 + rnd(seed, 60 + j) * 0.8;
            let s = 0.25 + rnd(seed, 70 + j) * 0.25;
            let p = base + Vec3::new(a.cos() * 0.28, y + s * 0.35 * rnd(seed, 80 + j), a.sin() * 0.28);
            cube(out, p, Vec3::new(s, s * (0.6 + rnd(seed, 90 + j)), s * 0.8), a, stone(100 + j), SHADE_STONE);
        }
        // A crack up the shaft, fire glowing in it, and the scorch about it.
        let face = rnd(seed, 110) * TAU;
        let n = Vec3::new(face.cos(), 0.0, face.sin());
        let mut p = base + n * 0.56 + Vec3::new(0.0, 0.5, 0.0);
        for j in 0..6 {
            let q = p + Vec3::new(0.0, (y - 0.6) / 6.0, 0.0) + Vec3::new(-n.z, 0.0, n.x) * (rnd(seed, 120 + j) - 0.5) * 0.25;
            rod(out, p, q, 0.035, lin(LAVA_DEEP), lin(LAVA) * 0.6, SHADE_LIT);
            p = q;
        }
        // A drum fallen against the plinth, rubble about it.
        let a = rnd(seed, 130) * TAU;
        let fall = base + Vec3::new(a.cos() * 0.55, 0.3, a.sin() * 0.55);
        out.push(Instance::new(
            Mat4::from_scale_rotation_translation(Vec3::new(0.85, 0.42, 0.85), Quat::from_rotation_y(a) * Quat::from_rotation_z(1.2), fall),
            stone(131),
            1.0,
            Vec3::ZERO,
            SHADE_STONE,
        ));
        for j in 0..7 {
            let b = rnd(seed, 140 + j) * TAU;
            let r = 0.5 + rnd(seed, 150 + j) * 0.4;
            let s = 0.08 + rnd(seed, 160 + j) * 0.14;
            cube(out, base + Vec3::new(b.cos() * r, s * 0.4, b.sin() * r), Vec3::splat(s), b, stone(170 + j), SHADE_STONE);
        }
    }

    /// A weapon of the war's dead, stuck in the floor at `at` (or a shield face down, a helm, a skull).
    fn build_fallen_weapon(&mut self, at: Vec3, seed: f64) {
        let out = &mut self.static_cubes;
        let kind = (rnd(seed, 0) * 6.0) as u32;
        let lean = Quat::from_rotation_y(rnd(seed, 1) * TAU) * Quat::from_rotation_x(0.12 + rnd(seed, 2) * 0.35);
        let up = lean * Vec3::Y;
        let steel = lin(STEEL) * (0.6 + rnd(seed, 3) * 0.3);
        match kind {
            0 | 1 => {
                // A sword: blade in the floor, crossguard, grip and pommel.
                let blade = 0.55 + rnd(seed, 4) * 0.25;
                rod(out, at - up * 0.05, at + up * blade, 0.05, steel, Vec3::ZERO, SHADE_IRON);
                let guard = at + up * blade;
                out.push(Instance::new(
                    Mat4::from_scale_rotation_translation(Vec3::new(0.26, 0.035, 0.045), lean, guard),
                    lin(IRON),
                    1.0,
                    Vec3::ZERO,
                    SHADE_IRON,
                ));
                rod(out, guard, guard + up * 0.16, 0.035, lin(LEATHER), Vec3::ZERO, SHADE_LIT);
                glowing(out, guard + up * 0.18, Vec3::splat(0.05), lean, lin(IRON_L), Vec3::ZERO, SHADE_IRON);
            }
            2 => {
                // A spear, its shaft broken, a rag of pennon on it.
                let l = 0.9 + rnd(seed, 5) * 0.5;
                rod(out, at, at + up * l, 0.035, lin(WOOD), Vec3::ZERO, SHADE_LIT);
                rod(out, at + up * l, at + up * (l + 0.1), 0.05, lin(WOOD_C), Vec3::ZERO, SHADE_LIT);
                let side = lean * Vec3::X;
                out.push(Instance::new(
                    Mat4::from_scale_rotation_translation(Vec3::new(0.16, 0.12, 0.01), lean, at + up * (l - 0.12) + side * 0.09),
                    lin(RED_D),
                    1.0,
                    Vec3::ZERO,
                    SHADE_CLOTH,
                ));
            }
            3 => {
                // An axe bitten into the floor, its haft up.
                rod(out, at + up * 0.08, at + up * 0.75, 0.04, lin(WOOD), Vec3::ZERO, SHADE_LIT);
                out.push(Instance::new(
                    Mat4::from_scale_rotation_translation(Vec3::new(0.05, 0.18, 0.22), lean, at + up * 0.06),
                    steel,
                    1.0,
                    Vec3::ZERO,
                    SHADE_IRON,
                ));
            }
            4 => {
                // A round shield face down, its rim of iron.
                let yaw = rnd(seed, 6) * TAU;
                for (r, col, y) in [(0.32f32, lin(IRON_D), 0.03f32), (0.27, lin(WOOD) * 0.8, 0.045)] {
                    for j in 0..4 {
                        cube(
                            out,
                            at + Vec3::new(0.0, y, 0.0),
                            Vec3::new(r * 2.0, 0.04, r * 0.8),
                            yaw + j as f32 * PI / 4.0,
                            col,
                            SHADE_IRON,
                        );
                    }
                }
                glowing(
                    out,
                    at + Vec3::new(0.0, 0.08, 0.0),
                    Vec3::new(0.12, 0.05, 0.12),
                    Quat::from_rotation_y(yaw),
                    lin(IRON_L),
                    Vec3::ZERO,
                    SHADE_IRON,
                );
            }
            _ => {
                // A helm on its side, and a skull by it.
                let yaw = rnd(seed, 7) * TAU;
                out.push(Instance::new(
                    Mat4::from_scale_rotation_translation(
                        Vec3::new(0.26, 0.22, 0.3),
                        Quat::from_rotation_y(yaw) * Quat::from_rotation_z(1.3),
                        at + Vec3::new(0.0, 0.12, 0.0),
                    ),
                    lin(IRON) * 1.2,
                    1.0,
                    Vec3::ZERO,
                    SHADE_IRON,
                ));
                let s = at + Vec3::new(yaw.cos() * 0.35, 0.09, yaw.sin() * 0.35);
                cube(out, s, Vec3::new(0.17, 0.16, 0.2), yaw, lin(BONE), SHADE_BONE);
                cube(out, s + Vec3::new(0.0, -0.06, 0.0), Vec3::new(0.12, 0.06, 0.15), yaw, lin(BONE_D), SHADE_BONE);
            }
        }
    }

    /// The stepped dais of basalt edged in iron, and on it the great anvil: a block of iron three
    /// tiles wide on a stone footing, its horn to the east, a blade half forged and glowing on its
    /// face, a great hammer against it, chains about its foot. Returns the middle of its face.
    fn build_anvil_dais(&mut self) -> Vec3 {
        let out = &mut self.static_cubes;
        let (x0, y0, x1, y1) = DAIS;
        let mid = Vec3::new((x0 + x1) as f32 / 2.0, 0.0, (y0 + y1) as f32 / 2.0);
        let (w, d) = ((x1 - x0 + 1) as f32, (y1 - y0 + 1) as f32);
        for k in 0..3 {
            let k = k as f32;
            let p = mid + Vec3::new(0.0, 0.06 + k * 0.08, -k * 0.35);
            cube(out, p, Vec3::new(w - k * 1.4, 0.12 + k * 0.0, d - k * 0.7), 0.0, lin(BASALT_D) * (1.0 + k * 0.08), SHADE_STONE);
            cube(
                out,
                p + Vec3::new(0.0, 0.065, (d - k * 0.7) / 2.0 - 0.02),
                Vec3::new(w - k * 1.4, 0.02, 0.05),
                0.0,
                lin(IRON_L),
                SHADE_IRON,
            );
        }
        let foot = Vec3::new(ANVIL.0 as f32 + 1.0, 0.0, ANVIL.2 as f32);
        // The stone footing.
        cube(out, foot + Vec3::new(0.0, 0.25, 0.0), Vec3::new(2.4, 0.5, 1.15), 0.0, lin(STONE_D), SHADE_STONE);
        cube(out, foot + Vec3::new(0.0, 0.53, 0.0), Vec3::new(2.1, 0.08, 0.95), 0.0, lin(STONE), SHADE_STONE);
        // The anvil's foot, waist and body.
        cube(out, foot + Vec3::new(0.0, 0.68, 0.0), Vec3::new(1.9, 0.22, 0.85), 0.0, lin(IRON_D), SHADE_IRON);
        cube(out, foot + Vec3::new(0.0, 0.92, 0.0), Vec3::new(1.15, 0.28, 0.55), 0.0, lin(IRON), SHADE_IRON);
        cube(out, foot + Vec3::new(0.0, 1.2, 0.0), Vec3::new(2.3, 0.3, 0.8), 0.0, lin(IRON), SHADE_IRON);
        // Its face, hammered bright at the middle.
        cube(out, foot + Vec3::new(0.0, 1.37, 0.0), Vec3::new(2.3, 0.05, 0.78), 0.0, lin(IRON_L), SHADE_IRON);
        // The heel (west) square, the horn (east) tapering to a point.
        cube(out, foot + Vec3::new(-1.25, 1.22, 0.0), Vec3::new(0.3, 0.26, 0.7), 0.0, lin(IRON), SHADE_IRON);
        for k in 0..5 {
            let t = k as f32 / 4.0;
            let s = 0.62 * (1.0 - t * 0.85);
            cube(
                out,
                foot + Vec3::new(1.2 + t * 0.9, 1.25 - t * 0.05, 0.0),
                Vec3::new(0.24, s * 0.55, s),
                0.0,
                lin(IRON) * (1.0 + t * 0.1),
                SHADE_IRON,
            );
        }
        // The hardy hole, and the blade on the face, half forged and glowing.
        cube(out, foot + Vec3::new(-0.8, 1.38, 0.0), Vec3::new(0.1, 0.02, 0.1), 0.0, lin(BLACK), SHADE_LIT);
        let blade = foot + Vec3::new(0.15, 1.42, 0.08);
        glowing(out, blade, Vec3::new(1.2, 0.04, 0.14), Quat::from_rotation_y(0.12), lin(LAVA), lin(LAVA_HOT) * 1.6, SHADE_LIT);
        glowing(
            out,
            blade + Vec3::new(0.7, 0.0, 0.08),
            Vec3::new(0.3, 0.035, 0.09),
            Quat::from_rotation_y(0.3),
            lin(LAVA_DEEP),
            lin(LAVA) * 0.9,
            SHADE_LIT,
        );
        // The great hammer leaning against the anvil's east side.
        let head = foot + Vec3::new(1.55, 0.2, 0.55);
        out.push(Instance::new(
            Mat4::from_scale_rotation_translation(Vec3::new(0.42, 0.3, 0.3), Quat::from_rotation_z(-0.3), head),
            lin(IRON_D),
            1.0,
            Vec3::ZERO,
            SHADE_IRON,
        ));
        rod(out, head + Vec3::new(-0.05, 0.12, 0.0), head + Vec3::new(-0.5, 1.15, 0.0), 0.07, lin(WOOD), Vec3::ZERO, SHADE_LIT);
        // Chains about its foot, link by link.
        for s in [-1.0f32, 1.0] {
            for k in 0..9 {
                let t = k as f32 / 8.0;
                let p = foot + Vec3::new(s * (0.7 + t * 0.6), 0.62 - (t * PI).sin() * 0.35 + t * -0.25, 0.48 + t * 0.12);
                out.push(Instance::new(
                    Mat4::from_scale_rotation_translation(
                        Vec3::new(0.12, 0.05, 0.035),
                        Quat::from_rotation_x(if k % 2 == 0 { 0.0 } else { PI / 2.0 }),
                        p.max(foot + Vec3::new(-9.0, 0.03, -9.0)),
                    ),
                    lin(IRON_L) * 0.8,
                    1.0,
                    Vec3::ZERO,
                    SHADE_IRON,
                ));
            }
        }
        foot + Vec3::new(0.0, 1.42, 0.0)
    }

    /// His throne behind the anvil: black iron and bone, its back high and spiked, a wolf's skull
    /// with a raven's beak crowning it, his great banner of the raven and the wolf on the wall over
    /// it. Returns the seat.
    fn build_wrath_throne(&mut self) -> Vec3 {
        let out = &mut self.static_cubes;
        let seat = Vec3::new(THRONE.0 as f32, 0.55, THRONE.1 as f32 + 0.4);
        cube(out, seat - Vec3::new(0.0, 0.3, 0.0), Vec3::new(2.4, 0.5, 1.5), 0.0, lin(BASALT_D), SHADE_STONE);
        cube(out, seat + Vec3::new(0.0, 0.25, 0.0), Vec3::new(2.0, 0.5, 1.3), 0.0, lin(IRON_D), SHADE_IRON);
        cube(out, seat + Vec3::new(0.0, 0.54, 0.05), Vec3::new(1.7, 0.1, 1.1), 0.0, lin(RED_D), SHADE_CLOTH);
        for s in [-1.0f32, 1.0] {
            cube(out, seat + Vec3::new(s * 1.0, 0.8, 0.0), Vec3::new(0.25, 0.5, 1.3), 0.0, lin(IRON), SHADE_IRON);
            // Each arm ends in a skull.
            cube(out, seat + Vec3::new(s * 1.0, 1.1, 0.55), Vec3::new(0.26, 0.24, 0.28), 0.0, lin(BONE), SHADE_BONE);
            cube(out, seat + Vec3::new(s * 1.0, 1.02, 0.66), Vec3::new(0.2, 0.08, 0.1), 0.0, lin(BONE_D), SHADE_BONE);
        }
        // The back: iron plates rising high, spiked like a crown of blades.
        cube(out, seat + Vec3::new(0.0, 1.8, -0.55), Vec3::new(2.2, 2.6, 0.3), 0.0, lin(IRON_D), SHADE_IRON);
        cube(out, seat + Vec3::new(0.0, 1.7, -0.38), Vec3::new(1.6, 2.0, 0.06), 0.0, lin(RED_D) * 0.8, SHADE_CLOTH);
        for k in 0..9 {
            let x = -1.0 + k as f32 * 0.25;
            let tall = 0.6 + (1.0 - (x / 1.1).abs()) * 1.1 + (k % 2) as f32 * 0.2;
            let p = seat + Vec3::new(x, 3.1 + tall / 2.0, -0.55);
            out.push(Instance::new(
                Mat4::from_scale_rotation_translation(
                    Vec3::new(0.09, tall, 0.09),
                    Quat::from_rotation_z(-x * 0.25) * Quat::from_rotation_y(PI / 4.0),
                    p,
                ),
                lin(IRON) * 1.1,
                1.0,
                Vec3::ZERO,
                SHADE_IRON,
            ));
        }
        // The crest: a wolf's skull with a raven's beak, horns curving out.
        let crest = seat + Vec3::new(0.0, 3.2, -0.35);
        cube(out, crest, Vec3::new(0.6, 0.5, 0.55), 0.0, lin(BONE), SHADE_BONE);
        cube(out, crest + Vec3::new(0.0, -0.12, 0.38), Vec3::new(0.3, 0.22, 0.4), 0.0, lin(BONE_D), SHADE_BONE);
        out.push(Instance::new(
            Mat4::from_scale_rotation_translation(
                Vec3::new(0.14, 0.14, 0.42),
                Quat::from_rotation_x(0.5),
                crest + Vec3::new(0.0, -0.32, 0.6),
            ),
            lin(BLACK),
            1.0,
            Vec3::ZERO,
            SHADE_BONE,
        ));
        for s in [-1.0f32, 1.0] {
            glowing(
                out,
                crest + Vec3::new(s * 0.15, 0.06, 0.28),
                Vec3::new(0.08, 0.06, 0.04),
                Quat::IDENTITY,
                lin(EMBER),
                lin(EMBER) * 2.0,
                SHADE_LIT,
            );
            let mut p = crest + Vec3::new(s * 0.3, 0.15, 0.0);
            for k in 0..5 {
                let q = p + Vec3::new(s * 0.18, 0.12 + k as f32 * 0.02, -0.05);
                rod(out, p, q, 0.11 - k as f32 * 0.018, lin(BONE_D), Vec3::ZERO, SHADE_BONE);
                p = q;
            }
        }
        // His great banner on the wall above: the raven over the wolf, red on black, gold-edged.
        let bw = Vec3::new(THRONE.0 as f32, 4.7, 0.55);
        cube(out, bw + Vec3::new(0.0, 0.95, 0.0), Vec3::new(3.4, 0.1, 0.1), 0.0, lin(IRON), SHADE_IRON);
        cube(out, bw, Vec3::new(3.0, 1.9, 0.04), 0.0, lin(BLACK), SHADE_CLOTH);
        cube(out, bw + Vec3::new(0.0, 0.0, 0.025), Vec3::new(2.7, 1.65, 0.02), 0.0, lin(RED), SHADE_CLOTH);
        for (x, y, sx, sy, r) in [
            (0.0f32, 0.3f32, 0.3f32, 0.55f32, 0.0f32),
            (0.0, 0.66, 0.24, 0.24, 0.0),
            (-0.45, 0.4, 0.8, 0.14, 0.4),
            (0.45, 0.4, 0.8, 0.14, -0.4),
            (-0.8, 0.2, 0.5, 0.1, 0.9),
            (0.8, 0.2, 0.5, 0.1, -0.9),
            (0.0, -0.45, 1.0, 0.3, 0.0),
            (0.42, -0.3, 0.3, 0.2, 0.3),
            (-0.6, -0.55, 0.5, 0.08, -0.5),
        ] {
            out.push(Instance::new(
                Mat4::from_scale_rotation_translation(Vec3::new(sx, sy, 0.02), Quat::from_rotation_z(r), bw + Vec3::new(x, y, 0.045)),
                lin(BLACK),
                1.0,
                Vec3::ZERO,
                SHADE_CLOTH,
            ));
        }
        for s in [-1.0f32, 1.0] {
            cube(out, bw + Vec3::new(s * 1.38, 0.0, 0.04), Vec3::new(0.06, 1.7, 0.02), 0.0, lin(GOLD), SHADE_IRON);
        }
        seat
    }

    /// A fire bowl of iron on a stone drum at `at`. Returns where its fire burns.
    fn build_fire_bowl(&mut self, at: Vec3) -> Vec3 {
        let out = &mut self.static_cubes;
        cube(out, at + Vec3::new(0.0, 0.35, 0.0), Vec3::new(0.7, 0.7, 0.7), PI / 4.0, lin(STONE_D), SHADE_STONE);
        cube(out, at + Vec3::new(0.0, 0.8, 0.0), Vec3::new(1.0, 0.22, 1.0), 0.0, lin(IRON_D), SHADE_IRON);
        cube(out, at + Vec3::new(0.0, 0.8, 0.0), Vec3::new(1.0, 0.22, 1.0), PI / 4.0, lin(IRON), SHADE_IRON);
        glowing(out, at + Vec3::new(0.0, 0.92, 0.0), Vec3::new(0.8, 0.04, 0.8), Quat::IDENTITY, lin(LAVA_DEEP), lin(LAVA) * 1.4, SHADE_LIT);
        at + Vec3::new(0.0, 0.95, 0.0)
    }

    /// A fire basket of iron bands on a bracket out of the wall at `at`, facing `into` the hall.
    /// Returns where its fire burns.
    fn build_fire_basket(&mut self, at: Vec3, into: Vec3) -> Vec3 {
        let out = &mut self.static_cubes;
        let p = at + into * 0.35;
        rod(out, at - into * 0.1, p, 0.06, lin(IRON), Vec3::ZERO, SHADE_IRON);
        rod(out, at - Vec3::new(0.0, 0.35, 0.0), p - Vec3::new(0.0, 0.05, 0.0), 0.04, lin(IRON), Vec3::ZERO, SHADE_IRON);
        for k in 0..6 {
            let a = k as f32 * TAU / 6.0;
            rod(
                out,
                p + Vec3::new(a.cos() * 0.08, -0.05, a.sin() * 0.08),
                p + Vec3::new(a.cos() * 0.2, 0.25, a.sin() * 0.2),
                0.03,
                lin(IRON_D),
                Vec3::ZERO,
                SHADE_IRON,
            );
        }
        glowing(out, p + Vec3::new(0.0, 0.05, 0.0), Vec3::new(0.2, 0.1, 0.2), Quat::IDENTITY, lin(LAVA_DEEP), lin(LAVA) * 1.5, SHADE_LIT);
        p + Vec3::new(0.0, 0.12, 0.0)
    }

    /// Each frame: the fires and their light, embers rising from the seams, the seams' glow, Amon's
    /// own fire, his rift's lava, and the gate.
    pub(super) fn emit_wrath_court(&self, s: &SceneState, time: f32, hero: Vec3, f: &mut Frame) {
        let Some(av) = &self.anvil else { return };
        f.bounds = Vec4::new(-0.5, -0.5, s.grid.width as f32 - 0.5, s.grid.height as f32 - 0.5);
        for (i, &(p, size)) in av.fires.iter().enumerate() {
            flames(f, p, size, time, i as f32);
            glow_orb(f, p + Vec3::new(0.0, 0.25 * size, 0.0), 0.9 * size + 0.2, lin(AMBER), 0.16 + 0.03 * (time * 7.0 + i as f32).sin());
        }
        // The nearest fires light the hall; the throne's whatever; the nearest seams glow up from below.
        let points: Vec<Vec3> = av.fires.iter().map(|f| f.0).collect();
        for (i, p) in nearest(&points, hero, 5).into_iter().enumerate() {
            let flick = 5.0 + (time * 6.0 + i as f32 * 1.3).sin() * 0.6 + (time * 15.0 + i as f32).sin() * 0.3;
            f.lights.push(Light { pos: p + Vec3::new(0.0, 0.35, 0.0), color: lin(AMBER) * flick, distance: 8.5 });
        }
        f.lights.push(Light { pos: av.throne + Vec3::new(0.0, 2.4, 1.6), color: lin(FLAME) * 5.0, distance: 8.0 });
        f.lights.push(Light { pos: av.anvil + Vec3::new(0.0, 0.5, 0.6), color: lin(LAVA_HOT) * 3.0, distance: 4.0 });
        for (i, p) in nearest(&av.seams, hero, 3).into_iter().enumerate() {
            let pulse = 2.2 + (time * 1.7 + i as f32 * 2.1).sin() * 0.6;
            f.lights.push(Light { pos: p, color: lin(LAVA) * pulse, distance: 3.5 });
        }
        // Embers rising off the seams and drifting up into the dark.
        for k in 0..48 {
            let seed = k as f64 * 3.17;
            let period = 3.0 + rnd(seed, 1) * 3.0;
            let t = ((time + rnd(seed, 2) * period) / period).fract();
            let from = av.seams.get(k % av.seams.len().max(1)).copied().unwrap_or(Vec3::new(CENTRE.0 as f32, 0.0, CENTRE.1 as f32));
            let drift = Vec3::new((t * 5.0 + k as f32).sin() * 0.5, t * 4.5, (t * 3.7 + k as f32 * 0.5).cos() * 0.5);
            let p = from + Vec3::new((rnd(seed, 3) - 0.5) * 2.0, 0.0, (rnd(seed, 4) - 0.5) * 2.0) + drift;
            let size = 0.035 * (1.0 - t) + 0.01;
            let c = lin(LAVA_HOT).lerp(lin(EMBER), t);
            f.additive.push(Instance::new(
                Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(size)),
                c * 2.2,
                0.85 * (1.0 - t),
                Vec3::ZERO,
                SHADE_UNLIT,
            ));
        }
        // Amon's own fire.
        for e in s.enemies {
            if let Some(d) = e.duel.as_deref().filter(|d| d.alive() && d.breed.kind == EnemyKind::Amon) {
                emit_amon_fire(d, time, f);
            }
        }
        self.emit_rift(s.traps, s.grid, time, f);
        // The gate he came in by.
        let door = s.gate.map(|g| g.door).unwrap_or(0.0);
        self.emit_portal(&av.gate, door, false, time, hero, f);
    }

    /// Amon's rift (his traps, `Shape::TrapLine`): a crack racing out from his axe along the floor,
    /// then a seam that glows hotter as its fuse burns down and flashes as it comes due, then lava
    /// bursting up in a fountain and lying in a pool that keeps burning while it lingers.
    fn emit_rift(&self, traps: &[Trap], grid: &Grid, time: f32, f: &mut Frame) {
        for t in traps {
            let ground = floor_at(grid, t.pos.x as f32, t.pos.y as f32);
            let at = Vec3::new(t.pos.x as f32, ground, t.pos.y as f32);
            let from = Vec3::new(t.from.x as f32, ground, t.from.y as f32);
            let seed = (t.pos.x * 3.7 + t.pos.y * 1.9) as f32;
            let r = t.radius as f32;
            let dir = (at - from).normalize_or_zero();
            let side = Vec3::new(-dir.z, 0.0, dir.x);
            if t.flight > 0 {
                // The crack racing out toward it.
                let k = 1.0 - t.flight as f32 / (COIN_FLIGHT as f32 + 1.0);
                let tip = from.lerp(at, k);
                let n = ((tip - from).length() / 0.3).ceil().max(1.0) as i32;
                for j in 0..n {
                    let p = from.lerp(tip, (j as f32 + 0.5) / n as f32) + side * ((j as f32 * 2.3 + seed).sin() * 0.08);
                    let m = Mat4::from_translation(p + Vec3::new(0.0, 0.03, 0.0))
                        * Mat4::from_rotation_y(-dir.z.atan2(dir.x))
                        * Mat4::from_scale(Vec3::new(0.32, 0.015, 0.06));
                    f.additive.push(Instance::new(m, lin(LAVA) * 0.9, 0.75, Vec3::ZERO, SHADE_UNLIT));
                }
                continue;
            }
            if let Some(b) = t.burst {
                if t.lingering() || b < 4 {
                    // The pool of lava: a glowing disc, a crust darkening over it as it ends.
                    let left = 1.0 - b as f32 / t.linger.max(1) as f32;
                    let pulse = 0.75 + 0.25 * (time * 5.0 + seed).sin();
                    let disc = |f: &mut Frame, rr: f32, c: Vec3, a: f32, y: f32| {
                        let m = Mat4::from_translation(at + Vec3::new(0.0, y, 0.0))
                            * Mat4::from_rotation_x(-PI / 2.0)
                            * Mat4::from_scale(Vec3::new(rr, rr, 1.0));
                        f.transparent[MeshKind::Disc as usize].push(Instance::new(m, c, a, Vec3::ZERO, SHADE_UNLIT));
                    };
                    disc(f, r, lin(LAVA) * (1.4 * pulse), 0.75 * left.max(0.2), 0.025);
                    disc(f, r * 0.6, lin(LAVA_HOT) * (1.6 * pulse), 0.6 * left, 0.03);
                    // Crust plates floating on it.
                    for j in 0..5 {
                        let a = j as f32 * 2.39 + seed * 1.3;
                        let rr = r * (0.25 + 0.5 * ((j * 7 % 5) as f32 / 5.0));
                        let p = at + Vec3::new(a.cos() * rr, 0.04, a.sin() * rr);
                        let sz = (0.1 + 0.08 * ((j * 3 % 4) as f32)) * (1.0 + 0.8 * (1.0 - left));
                        let m = Mat4::from_translation(p)
                            * Mat4::from_rotation_y(a * 1.7 + seed)
                            * Mat4::from_scale(Vec3::new(sz, 0.02, sz * 0.7));
                        f.opaque[MeshKind::Cube as usize].push(Instance::new(m, lin(BASALT_D), 1.0, lin(LAVA_DEEP) * 0.2, SHADE_LIT));
                    }
                    // Bubbles and flickers of flame.
                    for j in 0..3 {
                        let u = ((time * (1.3 + j as f32 * 0.4) + seed + j as f32) * 0.8).fract();
                        let a = j as f32 * 2.1 + seed * 0.3;
                        let p = at + Vec3::new(a.cos() * r * 0.35, 0.05 + u * 0.4, a.sin() * r * 0.35);
                        let sz = 0.08 * (1.0 - u) * left;
                        f.additive.push(Instance::new(
                            Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(sz, sz * 1.8, sz)),
                            lin(FLAME) * 2.0,
                            0.7 * (1.0 - u),
                            Vec3::ZERO,
                            SHADE_UNLIT,
                        ));
                    }
                    f.lights.push(Light { pos: at + Vec3::new(0.0, 0.5, 0.0), color: lin(LAVA) * 2.4 * left.max(0.15), distance: 2.6 });
                }
                if b < 26 {
                    // The burst: a fountain of lava thrown up and falling back.
                    let u = b as f32 / 26.0;
                    for j in 0..10 {
                        let a = j as f32 * 2.39 + seed;
                        let sp = 0.3 + 0.5 * ((j * 37 % 10) as f32 / 10.0);
                        let h = (1.6 + sp) * u * 2.0 - 4.0 * u * u * 1.2;
                        let p = at + Vec3::new(a.cos() * sp * u * r * 1.6, h.max(0.05), a.sin() * sp * u * r * 1.6);
                        let sz = 0.12 * (1.0 - u) + 0.04;
                        let c = lin(LAVA_HOT).lerp(lin(LAVA), u);
                        f.additive.push(Instance::new(
                            Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(sz)),
                            c * 2.4,
                            0.9 * (1.0 - u * 0.6),
                            Vec3::ZERO,
                            SHADE_UNLIT,
                        ));
                    }
                    f.lights.push(Light { pos: at + Vec3::new(0.0, 0.8, 0.0), color: lin(LAVA_HOT) * 6.0 * (1.0 - u), distance: 3.5 });
                }
                continue;
            }
            // The seam glowing as its fuse burns: a crack across it, brighter and wider as it comes
            // due, a ring of scorch showing how far the lava will reach, flashing at the end.
            let fill = t.tell();
            let late = t.fuse <= 8;
            let blink = if late { 0.7 + (time * 40.0).sin().abs() * 0.9 } else { 0.0 };
            let n = ((TAU * r) / 0.2).ceil() as i32;
            for k in 0..n {
                let a = k as f32 * TAU / n as f32;
                let p = at + Vec3::new(a.cos() * r, 0.03, a.sin() * r);
                let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(0.07, 0.012, 0.07));
                f.additive.push(Instance::new(m, lin(LAVA) * (0.6 + fill * 0.8 + blink), 0.7, Vec3::ZERO, SHADE_UNLIT));
            }
            let m = Mat4::from_translation(at + Vec3::new(0.0, 0.022, 0.0))
                * Mat4::from_rotation_x(-PI / 2.0)
                * Mat4::from_scale(Vec3::new(r, r, 1.0));
            f.transparent[MeshKind::Disc as usize].push(Instance::new(
                m,
                lin(LAVA_DEEP) * (0.7 + blink * 0.5),
                0.15 + 0.35 * fill,
                Vec3::ZERO,
                SHADE_UNLIT,
            ));
            for j in -2..=2 {
                let o = dir * (j as f32 * r * 0.38) + side * ((j as f32 * 1.7 + seed).sin() * 0.1);
                let m = Mat4::from_translation(at + o + Vec3::new(0.0, 0.03, 0.0))
                    * Mat4::from_rotation_y(-dir.z.atan2(dir.x) + (j as f32 + seed).sin() * 0.3)
                    * Mat4::from_scale(Vec3::new(r * 0.45, 0.015, 0.04 + 0.08 * fill));
                f.additive.push(Instance::new(m, lin(LAVA_HOT) * (1.0 + 1.5 * fill + blink), 0.9, Vec3::ZERO, SHADE_UNLIT));
            }
            f.lights.push(Light { pos: at + Vec3::new(0.0, 0.3, 0.0), color: lin(LAVA) * (0.8 + 2.5 * fill), distance: 2.2 });
        }
    }
}

/// Tongues of flame at `p`, `size` across, licking up and fading.
fn flames(f: &mut Frame, p: Vec3, size: f32, time: f32, seed: f32) {
    for k in 0..7 {
        let t = (time * 2.0 + seed * 0.37 + k as f32 * 0.143).fract();
        let a = seed * 1.9 + k as f32 * 1.3;
        let spread = 0.18 * size * (1.0 - t);
        let q = p + Vec3::new(a.cos() * spread, (0.05 + t * 0.7) * size, a.sin() * spread);
        let s = (0.2 * (1.0 - t) + 0.04) * size;
        let c = lin(FLAME).lerp(lin(FLAME_TIP), t);
        f.additive.push(Instance::new(
            Mat4::from_translation(q) * Mat4::from_scale(Vec3::new(s, s * 1.8, s)),
            c * 1.4,
            0.65 * (1.0 - t),
            Vec3::ZERO,
            SHADE_UNLIT,
        ));
    }
}

/// Amon wears his fire: in his throat (brighter as his rage grows), gathering at his maw as he
/// draws breath and roaring out in a cone as he breathes; trailing from him as he charges, snorting
/// from his beak as he lowers his head; embers about him as his rage mounts; a ring of sparks over
/// his head while he stands dazed.
fn emit_amon_fire(d: &crate::logic::combat::Duelist, time: f32, f: &mut Frame) {
    let p = Vec3::new(d.pos.x as f32, 0.0, d.pos.y as f32);
    let ahead = Vec3::new(d.facing.x as f32, 0.0, d.facing.y as f32).normalize_or_zero();
    let aim = Vec3::new(d.aim.x as f32, 0.0, d.aim.y as f32).normalize_or_zero();
    let side = Vec3::new(-ahead.z, 0.0, ahead.x);
    let rage = d.lore.rage as f32 / RAGE_MAX as f32;
    let maw = p + ahead * 0.75 + Vec3::new(0.0, 2.45, 0.0);
    // His key light (a warm one before him, a red rim behind) and the fire in his throat.
    f.lights.push(Light { pos: p + ahead * 2.2 + Vec3::new(0.0, 3.4, 0.0), color: lin(0xffc89a) * 5.0, distance: 7.0 });
    f.lights.push(Light { pos: p - ahead * 1.6 + Vec3::new(0.0, 3.0, 0.0), color: lin(0xc0281a) * 4.0, distance: 5.5 });
    f.lights.push(Light { pos: maw, color: lin(EMBER) * (1.0 + 3.0 * rage), distance: 2.5 + rage });
    glow_orb(f, maw, 0.25 + 0.2 * rage, lin(EMBER), 0.2 + 0.25 * rage);
    // Embers about him as his rage mounts.
    for k in 0..(d.lore.rage as usize * 5) {
        let t = ((time * 0.9 + k as f32 * 0.137) % 1.0 + 1.0) % 1.0;
        let a = k as f32 * 2.4 + time * 0.6;
        let q = p + Vec3::new(a.cos() * (0.9 + 0.3 * t), 0.4 + t * 3.0, a.sin() * (0.9 + 0.3 * t));
        let sz = 0.04 * (1.0 - t) + 0.01;
        f.additive.push(Instance::new(
            Mat4::from_translation(q) * Mat4::from_scale(Vec3::splat(sz)),
            lin(LAVA_HOT) * 2.2,
            0.9 * (1.0 - t),
            Vec3::ZERO,
            SHADE_UNLIT,
        ));
    }
    match d.f.state {
        State::Attack { mv: MoveId::FlameBreath, frame } => {
            let data = d.data(MoveId::FlameBreath);
            match data.phase(frame) {
                Phase::Startup => {
                    let u = frame as f32 / data.startup as f32;
                    glow_orb(f, maw, 0.3 + 0.5 * u, lin(FLAME), 0.3 + 0.6 * u);
                    f.lights.push(Light { pos: maw, color: lin(FLAME) * (2.0 + 6.0 * u), distance: 3.5 });
                }
                Phase::Active => {
                    // The cone of fire, roaring out along his aim.
                    let reach = data.reach as f32;
                    let half = data.half_angle as f32;
                    for k in 0..60 {
                        let t = ((time * 3.2 + k as f32 * 0.0167) % 1.0 + 1.0) % 1.0;
                        let a = ((k as f32 * 2.399).sin()) * half * 0.9;
                        let dir = aim * a.cos() + Vec3::new(-aim.z, 0.0, aim.x) * a.sin();
                        let q = maw.lerp(p + dir * reach + Vec3::new(0.0, 0.5, 0.0), t)
                            + Vec3::new(0.0, (t * 9.0 + k as f32).sin() * 0.12, 0.0);
                        let sz = 0.05 + 0.17 * t;
                        let c = lin(FLAME).lerp(lin(FLAME_TIP), t.sqrt());
                        f.additive.push(Instance::new(
                            Mat4::from_translation(q) * Mat4::from_scale(Vec3::new(sz, sz * 0.8, sz)),
                            c * 0.9,
                            0.55 * (1.0 - t * 0.8),
                            Vec3::ZERO,
                            SHADE_UNLIT,
                        ));
                    }
                    f.lights.push(Light { pos: p + aim * reach * 0.6 + Vec3::new(0.0, 1.0, 0.0), color: lin(FLAME) * 9.0, distance: 6.0 });
                }
                Phase::Recovery => {
                    // Smoke curling from his beak.
                    let u = (frame - data.startup - data.active) as f32 / data.recovery as f32;
                    for k in 0..4 {
                        let q = maw + Vec3::new((k as f32 * 1.7).sin() * 0.1, u * 0.8 + k as f32 * 0.15, 0.0);
                        f.transparent[MeshKind::Sphere as usize].push(Instance::new(
                            Mat4::from_translation(q) * Mat4::from_scale(Vec3::splat(0.12 + u * 0.2)),
                            lin(0x3a3230),
                            0.35 * (1.0 - u),
                            Vec3::ZERO,
                            SHADE_LIT,
                        ));
                    }
                }
            }
        }
        State::Attack { mv: MoveId::FieryCharge, frame } => {
            let data = d.data(MoveId::FieryCharge);
            match data.phase(frame) {
                Phase::Startup => {
                    // Snorts of fire from his beak as he paws the floor.
                    if (frame / 10) % 2 == 0 {
                        for s in [-1.0f32, 1.0] {
                            let q = maw + side * s * 0.12 + Vec3::new(0.0, -0.1, 0.0);
                            flames(f, q, 0.35, time * 2.0, s);
                        }
                    }
                }
                Phase::Active => {
                    // Fire streaming back from him as he runs, dust at his feet.
                    for k in 0..24 {
                        let t = ((time * 4.0 + k as f32 * 0.0417) % 1.0 + 1.0) % 1.0;
                        let q = p - aim * (0.6 + t * 2.4)
                            + side * ((k as f32 * 1.9).sin() * 0.7)
                            + Vec3::new(0.0, 0.3 + (k % 5) as f32 * 0.45 + t * 0.6, 0.0);
                        let sz = 0.18 * (1.0 - t) + 0.05;
                        f.additive.push(Instance::new(
                            Mat4::from_translation(q) * Mat4::from_scale(Vec3::new(sz, sz * 1.4, sz)),
                            lin(FLAME).lerp(lin(FLAME_TIP), t) * 1.6,
                            0.7 * (1.0 - t),
                            Vec3::ZERO,
                            SHADE_UNLIT,
                        ));
                    }
                    f.lights.push(Light { pos: p + Vec3::new(0.0, 1.2, 0.0), color: lin(FLAME) * 6.0, distance: 5.0 });
                }
                Phase::Recovery => {}
            }
        }
        State::Reel { kind: Reel::Staggered, left, .. } => {
            // Dazed: sparks wheeling over his head.
            let top = p + Vec3::new(0.0, 3.35, 0.0);
            for k in 0..6 {
                let a = time * 4.0 + k as f32 * TAU / 6.0;
                let q = top + Vec3::new(a.cos() * 0.55, (a * 2.0).sin() * 0.06, a.sin() * 0.55);
                f.additive.push(Instance::new(
                    Mat4::from_translation(q) * Mat4::from_rotation_y(a) * Mat4::from_scale(Vec3::new(0.09, 0.09, 0.02)),
                    lin(0xffe070) * 2.5,
                    0.9 * (left as f32 / 30.0).min(1.0),
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
        }
        _ => {}
    }
}
