//! The old man's workshop (WARBAND.md §12): the hidden place where the hero is armed, counselled
//! and tested, on a stage of its own. One room with six stations round it: the anvil before the
//! weapon racks and the hearth on the back wall, the shield wall beside them, the alchemy bench
//! along the right wall, the training yard before it, the war table by the front wall with the
//! Ledger's pinboard behind it, and the trophy wall on the left. The same layout is dressed for
//! the hero being played: a Norse smithy for Ragnar (log walls, a turf roof, a rune-carved anvil,
//! a stone hearth, axes on the wall), an Ottoman armoury tent for Mehmed (striped cloth, carpets,
//! brass lamps, halberds, a kazan on the fire), a Japanese forge for Miyamoto (tatami, paper
//! screens, a clay forge, a whetstone, swords on a stand).
//!
//! Each dressing is a set built the first time it is wanted. What the hero owns (the chains on
//! the sealed blades and shields, the lit shelf under each blade he has, the bottles on the
//! shelves, his belt on the bench, the generals' sigils, the news pinned up, the trophies) is a
//! second set, rebuilt when it changes. The blades and shields are the armory's own models of the
//! hero being played. The old man walks to the station the hero stands at and works there (three
//! strikes of the hammer at the anvil and the shield wall); Quitmer follows the hero about.

use super::crypt::pal::{AMBER, CHARCOAL, COPPER, IRON, MIST_TEAL, PALE_STONE, UMBER};
use super::*;
use crate::game::workshop::{QuitmerMood, Station, Work, WorkshopView, STRIKES, TROPHY_SLOTS, WORK_SECONDS};
use crate::logic::gear::{self, BELT_SLOTS};
use crate::logic::heroes::HeroKind;
use crate::logic::progression::{self, Tree, TIERS};
use crate::render::characters::Sculpt;
use crate::render::{armory, cast};

/// Where the workshop stands, well away from the level and the other stages.
pub(super) const WORKSHOP: Vec3 = Vec3::new(-240.0, 0.0, -40.0);

/// The armory's models are sculpted at the heroes' voxel size.
const ARMS_VOXEL: f32 = VOXEL / 4.0;

/// Material shade modes for plain blocks (5 + the material's number; stone and flag are the
/// art direction's surfaces).
const SHADE_METAL: f32 = 6.0;
const SHADE_FUR: f32 = 7.0;
const SHADE_CLOTH: f32 = 8.0;
const SHADE_LEATHER: f32 = 9.0;
const SHADE_WOOD: f32 = 12.0;
const SHADE_GOLD: f32 = 13.0;
const SHADE_STONE: f32 = 18.0;
const SHADE_FLAG: f32 = 19.0;

// ------------------------------------------------------------------ the layout

/// The room: half its width (x) and depth (z), and the height of its walls.
const HALF_W: f32 = 7.0;
const HALF_D: f32 = 5.0;
const HIGH: f32 = 3.1;

/// The hearth in the back-left corner, the anvil before the racks, the weapon rack along the
/// back wall (ten slots along x), the shield wall beside it (two rows of five), the block the
/// shields are reforged on, the bench along the right wall (along z), the chalk ring of the
/// yard with its post and dummy, the war table and the pinboard behind it on the front wall,
/// the trophy wall on the left wall (two rows of five), and the door in the front wall.
const HEARTH: Vec3 = Vec3::new(-6.2, 0.0, -4.15);
const ANVIL: Vec3 = Vec3::new(-4.0, 0.0, -3.0);
const ANVIL_TOP: f32 = 0.62;
const RACK: Vec3 = Vec3::new(-4.05, 0.0, -4.6);
const RACK_STEP: f32 = 0.22;
const SHIELDS: Vec3 = Vec3::new(-0.2, 0.0, -4.9);
const SHIELD_STEP: f32 = 0.56;
const SHIELD_ROWS: [f32; 2] = [1.95, 1.2];
const BLOCK: Vec3 = Vec3::new(-2.05, 0.0, -3.7);
const BLOCK_TOP: f32 = 0.55;
const BENCH: Vec3 = Vec3::new(6.45, 0.0, -2.6);
const BENCH_TOP: f32 = 0.88;
const YARD: Vec3 = Vec3::new(3.4, 0.0, 2.3);
const POST: Vec3 = Vec3::new(4.65, 0.0, 2.75);
const DUMMY: Vec3 = Vec3::new(4.0, 0.0, 1.35);
const TABLE: Vec3 = Vec3::new(-4.2, 0.0, 2.5);
const TABLE_TOP: f32 = 0.86;
const BOARD: Vec3 = Vec3::new(-4.2, 0.0, 4.9);
const TROPHY: Vec3 = Vec3::new(-6.9, 0.0, -2.35);
const TROPHY_STEP: f32 = 0.55;
const TROPHY_ROWS: [f32; 2] = [1.95, 1.3];
const DOOR: Vec3 = Vec3::new(1.2, 0.0, HALF_D);
/// The open floor in the middle, which the walkers cross on the way from one side to another.
const MIDDLE: Vec3 = Vec3::new(0.2, 0.0, 0.3);
/// The lamps: on the back wall between the racks and the shields, by the yard, by the war
/// table, by the door, beside the shield wall, and at the end of the bench.
const LAMPS: [Vec3; 6] = [
    Vec3::new(-2.3, 2.15, -4.7),
    Vec3::new(6.7, 2.05, 0.45),
    Vec3::new(-6.7, 2.05, 0.45),
    Vec3::new(-0.1, 2.25, 4.7),
    Vec3::new(1.5, 2.15, -4.7),
    Vec3::new(6.75, 2.0, -4.3),
];

/// One station's place in the room: the camera's eye and what it frames (set on the left of
/// the screen, the wares' panel being on the right), and where the old man, the hero and
/// Quitmer stand, and where the old man works.
#[derive(Clone, Copy)]
struct Spot {
    eye: Vec3,
    subject: Vec3,
    /// How far left of the middle the subject sits, in halves of the screen's width.
    left: f32,
    old_man: Vec3,
    /// The hero and Quitmer, to the camera's right of the subject and nearer the camera.
    hero: (f32, f32),
    dog: (f32, f32),
    /// Where the old man works (his hands go there).
    work: Vec3,
}

impl Spot {
    /// A point on the floor `r` to the camera's right of the subject and `t` nearer the camera.
    fn framed(&self, (r, t): (f32, f32)) -> Vec3 {
        let d = Vec3::new(self.subject.x - self.eye.x, 0.0, self.subject.z - self.eye.z).normalize_or(Vec3::NEG_Z);
        let right = Vec3::new(-d.z, 0.0, d.x);
        Vec3::new(self.subject.x, 0.0, self.subject.z) + right * r - d * t
    }

    fn hero_at(&self) -> Vec3 {
        self.framed(self.hero)
    }

    fn dog_at(&self) -> Vec3 {
        self.framed(self.dog)
    }
}

fn spot(s: Station) -> Spot {
    let v = Vec3::new;
    match s {
        Station::Anvil => Spot {
            eye: v(-3.0, 2.65, 1.9),
            subject: v(-4.15, 0.95, -3.2),
            left: 0.45,
            old_man: ANVIL + v(0.0, 0.0, -0.5),
            hero: (-1.0, 0.55),
            dog: (0.85, 1.35),
            work: ANVIL + v(0.0, ANVIL_TOP, 0.0),
        },
        Station::ShieldWall => Spot {
            eye: v(-0.3, 2.7, 1.9),
            subject: v(-0.75, 1.2, -3.8),
            left: 0.55,
            old_man: BLOCK + v(0.0, 0.0, -0.5),
            hero: (0.75, 1.25),
            dog: (1.2, 2.0),
            work: BLOCK + v(0.0, BLOCK_TOP, 0.0),
        },
        Station::Bench => Spot {
            eye: v(2.6, 2.6, 1.7),
            subject: v(6.15, 1.15, -2.45),
            left: 0.5,
            old_man: BENCH + v(-0.6, 0.0, -0.3),
            hero: (1.0, 1.0),
            dog: (1.35, 1.75),
            work: BENCH + v(0.05, BENCH_TOP + 0.05, -0.1),
        },
        Station::Yard => Spot {
            eye: v(-0.75, 2.95, -1.75),
            subject: v(3.55, 0.75, 1.95),
            left: 0.45,
            old_man: YARD + v(1.35, 0.0, -0.15),
            hero: (0.55, 0.3),
            dog: (-0.55, 1.5),
            work: DUMMY + v(0.0, 1.1, 0.0),
        },
        Station::WarTable => Spot {
            eye: v(-2.2, 2.85, -0.85),
            subject: v(-4.4, 0.95, 3.0),
            left: 0.5,
            old_man: TABLE + v(-0.3, 0.0, 0.85),
            hero: (-0.95, 1.05),
            dog: (-1.2, 1.85),
            work: TABLE + v(0.0, TABLE_TOP + 0.1, 0.0),
        },
        Station::Trophies => Spot {
            eye: v(-2.0, 2.6, 1.3),
            subject: v(-6.5, 1.35, -2.35),
            left: 0.45,
            old_man: TROPHY + v(0.8, 0.0, -1.45),
            hero: (0.55, 1.5),
            dog: (1.05, 2.2),
            work: TROPHY + v(0.1, 1.6, 0.0),
        },
    }
}

// ------------------------------------------------------------------ the dressings

/// Which dressing the room wears: the hero's own.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Dress {
    Norse,
    Ottoman,
    Japanese,
}

impl Dress {
    fn of(hero: HeroKind) -> Self {
        match hero {
            HeroKind::Ragnar => Dress::Norse,
            HeroKind::Mehmed => Dress::Ottoman,
            HeroKind::Miyamoto => Dress::Japanese,
        }
    }

    fn index(self) -> usize {
        self as usize
    }

    /// The wood of the furniture (racks, benches, tables): its colour and a darker one.
    fn wood(self) -> (u32, u32) {
        match self {
            Dress::Norse => (0x6a5038, 0x4a3828),
            Dress::Ottoman => (0x6e4a30, 0x4e3422),
            Dress::Japanese => (0x5a4232, 0x3a2a20),
        }
    }
}

fn h(a: f64, b: f64) -> f32 {
    hash(a, b) as f32
}

/// A colour jittered a little up or down by `k` (0..1).
fn jit(hex: u32, k: f32, amount: f32) -> Vec3 {
    lin(hex) * (1.0 - amount + k * amount * 2.0)
}

/// Blocks laid into a set, in the room's own coordinates.
struct Build {
    out: Vec<Instance>,
}

impl Build {
    fn new() -> Self {
        Self { out: Vec::new() }
    }

    fn put(&mut self, p: Vec3, s: Vec3, yaw: f32, c: Vec3, shade: f32) {
        let m = Mat4::from_scale_rotation_translation(s, Quat::from_rotation_y(yaw), WORKSHOP + p);
        self.out.push(Instance::new(m, c, 1.0, Vec3::ZERO, shade));
    }

    fn hex(&mut self, p: Vec3, s: Vec3, c: u32, shade: f32) {
        self.put(p, s, 0.0, lin(c), shade);
    }

    fn rot(&mut self, p: Vec3, s: Vec3, q: Quat, c: Vec3, shade: f32) {
        let m = Mat4::from_scale_rotation_translation(s, q, WORKSHOP + p);
        self.out.push(Instance::new(m, c, 1.0, Vec3::ZERO, shade));
    }

    fn glow(&mut self, p: Vec3, s: Vec3, c: u32, glow: Vec3) {
        let m = Mat4::from_scale_rotation_translation(s, Quat::IDENTITY, WORKSHOP + p);
        self.out.push(Instance::new(m, lin(c), 1.0, glow, SHADE_LIT));
    }

    /// A beam from `a` to `b`, `thick` across.
    fn seg(&mut self, a: Vec3, b: Vec3, thick: f32, c: Vec3, shade: f32) {
        let m = super::forge::segment(WORKSHOP + a, WORKSHOP + b, thick);
        self.out.push(Instance::new(m, c, 1.0, Vec3::ZERO, shade));
    }

    /// A box standing on the floor at `p`.
    fn stand(&mut self, p: Vec3, s: Vec3, c: u32, shade: f32) {
        self.hex(p + Vec3::new(0.0, s.y / 2.0, 0.0), s, c, shade);
    }
}

/// The set of one dressing: everything that stays put.
fn dressing_set(dress: Dress) -> Vec<Instance> {
    let mut b = Build::new();
    floor(&mut b, dress);
    walls(&mut b, dress);
    ceiling(&mut b, dress);
    hearth(&mut b, dress);
    anvil(&mut b, dress);
    rack(&mut b, dress);
    shield_wall(&mut b, dress);
    bench(&mut b, dress);
    yard(&mut b, dress);
    war_table(&mut b, dress);
    trophy_wall(&mut b, dress);
    door(&mut b, dress);
    lamps(&mut b, dress);
    decor(&mut b, dress);
    b.out
}

fn floor(b: &mut Build, dress: Dress) {
    b.hex(Vec3::new(0.0, -0.16, 0.0), Vec3::new(HALF_W * 2.0 + 0.6, 0.2, HALF_D * 2.0 + 0.6), 0x14110e, SHADE_LIT);
    match dress {
        Dress::Norse => {
            // Wide planks along x, the joints staggered; flagstones round the hearth.
            let mut row = 0;
            let mut z = -HALF_D + 0.15;
            while z < HALF_D {
                let mut x = -HALF_W + h(row as f64, 3.0) * 0.8 - 0.8;
                let mut n = 0;
                while x < HALF_W {
                    let k = h(row as f64 * 1.3, n as f64 * 2.1);
                    let len = 1.3 + k * 1.0;
                    let (a, e) = (x.max(-HALF_W), (x + len).min(HALF_W));
                    let near_hearth = (a + e) / 2.0 < -4.4 && z < -3.0;
                    if e - a > 0.1 && !near_hearth {
                        b.put(
                            Vec3::new((a + e) / 2.0, -0.04 + k * 0.012, z),
                            Vec3::new(e - a - 0.03, 0.08, 0.28),
                            0.0,
                            jit(UMBER[(n + row) % 2], k, 0.12),
                            SHADE_WOOD,
                        );
                    }
                    x += len;
                    n += 1;
                }
                z += 0.3;
                row += 1;
            }
            for gx in 0..5 {
                for gz in 0..4 {
                    let k = h(gx as f64 * 1.7, gz as f64 + 7.0);
                    let p = Vec3::new(-HALF_W + 0.35 + gx as f32 * 0.6, -0.04, -HALF_D + 0.3 + gz as f32 * 0.6);
                    b.put(p, Vec3::new(0.56, 0.1, 0.56), (k - 0.5) * 0.06, jit(CHARCOAL[gx % 3], k, 0.1), SHADE_FLAG);
                }
            }
            // A bear's fur before the war table.
            b.hex(TABLE + Vec3::new(0.0, 0.005, -0.1), Vec3::new(2.3, 0.03, 1.7), 0x5a4636, SHADE_FUR);
            b.hex(TABLE + Vec3::new(0.0, 0.012, -0.1), Vec3::new(1.9, 0.03, 1.3), 0x6a5644, SHADE_FUR);
        }
        Dress::Ottoman => {
            // Beaten earth, and carpets over it.
            for gx in -7..7 {
                for gz in -5..5 {
                    let k = h(gx as f64 * 1.9, gz as f64 * 2.3);
                    b.put(
                        Vec3::new(gx as f32 + 0.5, -0.05 + k * 0.01, gz as f32 + 0.5),
                        Vec3::new(0.99, 0.1, 0.99),
                        0.0,
                        jit(0x5a4a3c, k, 0.08),
                        SHADE_FLAG,
                    );
                }
            }
            for &(c, w, d, field, border) in &[
                (Vec3::new(0.3, 0.0, 0.4), 4.6, 3.0, 0x7a3424, AMBER),
                (TABLE + Vec3::new(0.0, 0.0, -0.1), 2.6, 2.0, 0x3a4a48, COPPER),
                (Vec3::new(5.2, 0.0, -2.5), 1.6, 3.0, 0x5a3a2c, AMBER),
            ] {
                carpet(b, c, w, d, field, border);
            }
        }
        Dress::Japanese => {
            // Dark boards, the beaten-earth forge floor round the hearth and anvil, and tatami.
            let mut n = 0;
            let mut x = -HALF_W + 0.11;
            while x < HALF_W {
                let k = h(n as f64 * 0.7, 5.0);
                b.put(Vec3::new(x, -0.04, 0.0), Vec3::new(0.2, 0.08, HALF_D * 2.0), 0.0, jit(0x4a3a2e, k, 0.1), SHADE_WOOD);
                x += 0.22;
                n += 1;
            }
            b.hex(Vec3::new(-4.6, 0.005, -3.5), Vec3::new(4.6, 0.04, 3.0), 0x4a4038, SHADE_FLAG);
            for i in 0..3 {
                for j in 0..2 {
                    let c = TABLE + Vec3::new(-0.9 + i as f32 * 0.9, 0.0, -0.9 + j as f32 * 1.8);
                    tatami(b, c, Vec3::new(0.88, 0.0, 1.78));
                }
            }
            for i in 0..2 {
                tatami(b, Vec3::new(-0.6 + i as f32 * 1.8, 0.0, 0.2), Vec3::new(1.78, 0.0, 0.88));
            }
        }
    }
}

/// A carpet: its field, a border round it, and a few medallions.
fn carpet(b: &mut Build, c: Vec3, w: f32, d: f32, field: u32, border: u32) {
    b.hex(c + Vec3::new(0.0, 0.012, 0.0), Vec3::new(w, 0.025, d), border, SHADE_CLOTH);
    b.hex(c + Vec3::new(0.0, 0.02, 0.0), Vec3::new(w - 0.2, 0.025, d - 0.2), field, SHADE_CLOTH);
    b.hex(c + Vec3::new(0.0, 0.026, 0.0), Vec3::new(w - 0.42, 0.025, d - 0.42), 0x4a2a20, SHADE_CLOTH);
    b.hex(c + Vec3::new(0.0, 0.032, 0.0), Vec3::new(w - 0.5, 0.025, d - 0.5), field, SHADE_CLOTH);
    for k in 0..3 {
        let x = c.x + (k as f32 - 1.0) * w * 0.28;
        b.rot(Vec3::new(x, c.y + 0.04, c.z), Vec3::new(0.32, 0.02, 0.32), Quat::from_rotation_y(PI / 4.0), lin(border), SHADE_CLOTH);
        b.rot(Vec3::new(x, c.y + 0.045, c.z), Vec3::new(0.16, 0.02, 0.16), Quat::from_rotation_y(PI / 4.0), lin(0x2e4a4a), SHADE_CLOTH);
    }
}

/// A tatami mat (`size` x and z): straw, and its dark cloth edges along the long sides.
fn tatami(b: &mut Build, c: Vec3, size: Vec3) {
    b.hex(c + Vec3::new(0.0, 0.03, 0.0), Vec3::new(size.x - 0.02, 0.06, size.z - 0.02), 0x8a8460, SHADE_CLOTH);
    let along_z = size.z > size.x;
    for s in [-1.0f32, 1.0] {
        let p = if along_z {
            c + Vec3::new(s * (size.x / 2.0 - 0.04), 0.035, 0.0)
        } else {
            c + Vec3::new(0.0, 0.035, s * (size.z / 2.0 - 0.04))
        };
        let sz = if along_z { Vec3::new(0.07, 0.065, size.z - 0.02) } else { Vec3::new(size.x - 0.02, 0.065, 0.07) };
        b.hex(p, sz, 0x2a2a24, SHADE_CLOTH);
    }
}

/// The four walls, each from `from` along `along` for `len`, the room on its `inward` side.
fn walls(b: &mut Build, dress: Dress) {
    let sides = [
        (Vec3::new(-HALF_W, 0.0, -HALF_D), Vec3::X, HALF_W * 2.0, Vec3::Z),
        (Vec3::new(-HALF_W, 0.0, HALF_D), Vec3::X, HALF_W * 2.0, Vec3::NEG_Z),
        (Vec3::new(-HALF_W, 0.0, -HALF_D), Vec3::Z, HALF_D * 2.0, Vec3::X),
        (Vec3::new(HALF_W, 0.0, -HALF_D), Vec3::Z, HALF_D * 2.0, Vec3::NEG_X),
    ];
    for (n, &(from, along, len, inward)) in sides.iter().enumerate() {
        let seed = n as f64 * 13.0;
        let wall = |b: &mut Build, u0: f32, u1: f32, y0: f32, y1: f32, depth: f32, out: f32, c: Vec3, shade: f32| {
            let mid = from + along * ((u0 + u1) / 2.0) + Vec3::new(0.0, (y0 + y1) / 2.0, 0.0) - inward * out;
            let size = along * (u1 - u0) + Vec3::Y * (y1 - y0) + inward.abs() * depth;
            b.put(mid, size, 0.0, c, shade);
        };
        // The door's gap in the front wall.
        let gap = if n == 1 { Some((DOOR.x + HALF_W - 0.75, DOOR.x + HALF_W + 0.75)) } else { None };
        let spans: Vec<(f32, f32)> = match gap {
            Some((a, e)) => vec![(0.0, a), (e, len)],
            None => vec![(0.0, len)],
        };
        match dress {
            Dress::Norse => {
                // A footing of stones, then logs laid along the wall to the roof.
                for &(a, e) in &spans {
                    let mut u = a;
                    let mut k = 0;
                    while u < e {
                        let w = 0.45 + h(u as f64 + seed, 1.0) * 0.3;
                        let e2 = (u + w).min(e);
                        wall(b, u, e2 - 0.03, 0.0, 0.36, 0.3, 0.0, jit(CHARCOAL[k % 3], h(u as f64, seed), 0.12), SHADE_STONE);
                        u += w;
                        k += 1;
                    }
                    let mut y = 0.36;
                    let mut row = 0;
                    while y < HIGH {
                        let k = h(row as f64 + seed, 4.0);
                        let c = jit(if row % 2 == 0 { 0x4e3c2c } else { 0x5a4632 }, k, 0.1);
                        wall(b, a, e, y, y + 0.22, 0.26, 0.0, c, SHADE_WOOD);
                        // Moss and turf pressed in the joints.
                        wall(b, a, e, y + 0.205, y + 0.235, 0.3, 0.0, lin(0x2e3426), SHADE_FUR);
                        y += 0.235;
                        row += 1;
                    }
                }
                for u in posts(len, n) {
                    if gap.is_some_and(|(a, e)| u > a - 0.3 && u < e + 0.3) {
                        continue;
                    }
                    let p = from + along * u + inward * 0.12;
                    b.put(p + Vec3::new(0.0, HIGH / 2.0, 0.0), Vec3::new(0.3, HIGH, 0.3), 0.0, lin(0x3e2e22), SHADE_WOOD);
                }
            }
            Dress::Ottoman => {
                // Cloth hung in stripes of madder red and cream, a dark band along its foot, poles.
                for &(a, e) in &spans {
                    let mut u = a;
                    let mut k = 0;
                    while u < e {
                        let e2 = (u + 0.5).min(e);
                        let c = if k % 2 == 0 { 0x7a3424 } else { 0x9a8e74 };
                        // The cloth bellies a little between the ropes.
                        let sag = 0.04 * (k as f32 * 0.9).sin();
                        wall(b, u, e2, 0.0, HIGH, 0.06, sag, jit(c, h(u as f64, seed), 0.06), SHADE_CLOTH);
                        u = e2;
                        k += 1;
                    }
                    wall(b, a, e, 0.0, 0.42, 0.08, -0.02, lin(0x3a2a24), SHADE_CLOTH);
                    wall(b, a, e, 0.42, 0.5, 0.08, -0.025, lin(AMBER), SHADE_CLOTH);
                    wall(b, a, e, HIGH - 0.35, HIGH - 0.25, 0.08, -0.025, lin(AMBER), SHADE_CLOTH);
                }
                for u in posts(len, n) {
                    if gap.is_some_and(|(a, e)| u > a - 0.3 && u < e + 0.3) {
                        continue;
                    }
                    let p = from + along * u + inward * 0.12;
                    b.put(p + Vec3::new(0.0, HIGH / 2.0, 0.0), Vec3::new(0.12, HIGH, 0.12), 0.0, lin(0x5a3a22), SHADE_WOOD);
                    b.put(p + Vec3::new(0.0, HIGH - 0.05, 0.0), Vec3::new(0.18, 0.12, 0.18), 0.0, lin(0xb08a3c), SHADE_GOLD);
                }
            }
            Dress::Japanese => {
                // Posts and rails of dark wood; paper screens between them, plaster above. The
                // back wall, the forge's side, is boarded instead.
                let boarded = n == 0;
                for &(a, e) in &spans {
                    let panels = ((e - a) / 0.9).round().max(1.0) as i32;
                    let pw = (e - a) / panels as f32;
                    for p in 0..panels {
                        let u0 = a + p as f32 * pw;
                        let u1 = u0 + pw;
                        if boarded {
                            for q in 0..3 {
                                let w = pw / 3.0;
                                let c = jit(0x5a4636, h(u0 as f64 + q as f64, seed), 0.1);
                                wall(b, u0 + q as f32 * w, u0 + (q + 1) as f32 * w - 0.015, 0.0, 1.95, 0.06, 0.0, c, SHADE_WOOD);
                            }
                        } else {
                            wall(b, u0, u1, 0.12, 1.88, 0.03, 0.0, lin(0xc8c0a6), SHADE_CLOTH);
                            for q in 1..3 {
                                let u = u0 + pw * q as f32 / 3.0;
                                wall(b, u - 0.012, u + 0.012, 0.12, 1.88, 0.05, -0.01, lin(0x3a2a20), SHADE_WOOD);
                            }
                            for q in 1..5 {
                                let y = 0.12 + 1.76 * q as f32 / 5.0;
                                wall(b, u0, u1, y - 0.012, y + 0.012, 0.05, -0.01, lin(0x3a2a20), SHADE_WOOD);
                            }
                        }
                        wall(b, u0, u0 + 0.1, 0.0, HIGH, 0.12, -0.03, lin(0x2e2018), SHADE_WOOD);
                    }
                    wall(b, a, e, 0.0, 0.12, 0.1, -0.02, lin(0x2e2018), SHADE_WOOD);
                    wall(b, a, e, 1.88, 2.02, 0.12, -0.03, lin(0x2e2018), SHADE_WOOD);
                    wall(b, a, e, 2.02, HIGH, 0.06, 0.0, jit(PALE_STONE[0], h(a as f64, seed), 0.05), SHADE_STONE);
                }
            }
        }
    }
}

/// Where the posts stand along wall `n` (of `len`): the corners, and between them clear of
/// the stations (thirds along the back and front, the middle of the sides).
fn posts(len: f32, n: usize) -> Vec<f32> {
    if n < 2 {
        vec![0.0, len / 3.0, len * 2.0 / 3.0, len]
    } else {
        vec![0.0, len / 2.0, len]
    }
}

fn ceiling(b: &mut Build, dress: Dress) {
    let (wood, dark) = match dress {
        Dress::Norse => (0x3e2e22, 0x2a3222),
        Dress::Ottoman => (0x5a3a22, 0x6a2c20),
        Dress::Japanese => (0x3a2a20, 0x4a3a2e),
    };
    // Beams across, and the roof's underside over them (turf on boards, the tent's cloth, boards).
    let mut x = -HALF_W + 0.7;
    while x < HALF_W {
        b.hex(Vec3::new(x, HIGH + 0.08, 0.0), Vec3::new(0.2, 0.22, HALF_D * 2.0), wood, SHADE_WOOD);
        x += 1.4;
    }
    for k in 0..10 {
        let z = -HALF_D + 0.5 + k as f32;
        let c = if dress == Dress::Ottoman && k % 2 == 1 { 0x9a8e74 } else { dark };
        let shade = if dress == Dress::Norse {
            SHADE_FUR
        } else if dress == Dress::Ottoman {
            SHADE_CLOTH
        } else {
            SHADE_WOOD
        };
        // A smoke hole over the hearth.
        if dress != Dress::Japanese && k == 0 {
            b.hex(Vec3::new(1.0, HIGH + 0.3, z), Vec3::new(HALF_W * 2.0 - 2.0 + 0.6, 0.1, 0.99), c, shade);
            continue;
        }
        b.hex(Vec3::new(0.0, HIGH + 0.3, z), Vec3::new(HALF_W * 2.0 + 0.6, 0.1, 0.99), c, shade);
    }
}

fn hearth(b: &mut Build, dress: Dress) {
    let c = HEARTH;
    match dress {
        Dress::Norse => {
            // A raised hearth of stones, a fire bed of embers, and a hood of stone over it.
            for i in 0..4 {
                for j in 0..3 {
                    let k = h(i as f64 * 3.1, j as f64 + 1.0);
                    let p = c + Vec3::new(-0.6 + i as f32 * 0.4, 0.18, -0.4 + j as f32 * 0.4);
                    b.put(p, Vec3::new(0.38, 0.36, 0.38), (k - 0.5) * 0.1, jit(CHARCOAL[(i + j) % 3], k, 0.12), SHADE_STONE);
                }
            }
            for k in 0..9 {
                let a = k as f32 * 0.7;
                b.glow(c + Vec3::new(a.cos() * 0.3, 0.4, a.sin() * 0.22), Vec3::splat(0.09), 0x3a1408, lin(0xff5a18) * 0.45);
            }
            for i in 0..3 {
                b.seg(
                    c + Vec3::new(-0.3 + i as f32 * 0.3, 0.42, -0.2),
                    c + Vec3::new(0.2 - i as f32 * 0.2, 0.45, 0.25),
                    0.07,
                    lin(0x2a1a10),
                    SHADE_WOOD,
                );
            }
            b.hex(c + Vec3::new(0.0, 2.35, -0.25), Vec3::new(1.7, 0.9, 1.1), 0x3f4745, SHADE_STONE);
            b.hex(c + Vec3::new(0.0, 1.85, -0.25), Vec3::new(1.85, 0.12, 1.25), 0x2e3432, SHADE_STONE);
            b.hex(c + Vec3::new(0.0, 2.95, -0.55), Vec3::new(0.8, 0.5, 0.6), 0x353c3a, SHADE_STONE);
        }
        Dress::Ottoman => {
            // A ring of stones, the fire, an iron tripod and the great copper kazan on it.
            for k in 0..10 {
                let a = k as f32 / 10.0 * TAU;
                let kk = h(k as f64, 9.0);
                b.put(
                    c + Vec3::new(a.cos() * 0.62, 0.1, a.sin() * 0.62),
                    Vec3::new(0.22, 0.2, 0.2),
                    a,
                    jit(CHARCOAL[k % 3], kk, 0.12),
                    SHADE_STONE,
                );
            }
            for k in 0..7 {
                let a = k as f32 * 0.9;
                b.glow(c + Vec3::new(a.cos() * 0.25, 0.06, a.sin() * 0.2), Vec3::splat(0.1), 0x3a1408, lin(0xff5a18) * 0.45);
            }
            for k in 0..3 {
                let a = k as f32 / 3.0 * TAU + 0.4;
                b.seg(c + Vec3::new(a.cos() * 0.55, 0.0, a.sin() * 0.55), c + Vec3::new(0.0, 1.25, 0.0), 0.05, lin(IRON), SHADE_METAL);
            }
            // The kazan: a deep copper bowl, rings of it narrowing to the base.
            for (y, r) in [(0.42f32, 0.22f32), (0.5, 0.32), (0.6, 0.38), (0.7, 0.4), (0.8, 0.39), (0.88, 0.36)] {
                for k in 0..8 {
                    let a = k as f32 / 8.0 * TAU;
                    b.put(
                        c + Vec3::new(a.cos() * r, y, a.sin() * r),
                        Vec3::new(r * 0.8, 0.1, 0.06),
                        -a + PI / 2.0,
                        jit(COPPER, h(k as f64, y as f64), 0.08),
                        SHADE_GOLD,
                    );
                }
            }
            b.hex(c + Vec3::new(0.0, 0.86, 0.0), Vec3::new(0.62, 0.02, 0.62), 0x5a3a20, SHADE_LIT);
            b.hex(c + Vec3::new(0.0, 0.93, 0.0), Vec3::new(0.82, 0.04, 0.82), 0xb08a3c, SHADE_GOLD);
        }
        Dress::Japanese => {
            // The hodo: a low forge of clay with its mouth glowing, the box bellows beside it.
            b.stand(c, Vec3::new(1.3, 0.55, 0.9), 0x6a5a4a, SHADE_STONE);
            b.stand(c + Vec3::new(0.0, 0.55, 0.0), Vec3::new(1.1, 0.25, 0.75), 0x5a4c3e, SHADE_STONE);
            b.glow(c + Vec3::new(0.0, 0.62, 0.2), Vec3::new(0.5, 0.18, 0.4), 0x3a1408, lin(0xff6a20) * 0.9);
            b.glow(c + Vec3::new(0.0, 0.45, 0.46), Vec3::new(0.3, 0.14, 0.03), 0x3a1408, lin(0xff6a20) * 0.7);
            b.stand(c + Vec3::new(1.05, 0.0, -0.1), Vec3::new(0.55, 0.5, 0.95), 0x4a3426, SHADE_WOOD);
            b.hex(c + Vec3::new(1.05, 0.38, 0.4), Vec3::new(0.06, 0.06, 0.4), 0x2e2018, SHADE_WOOD);
            b.hex(c + Vec3::new(0.0, 2.4, -0.2), Vec3::new(0.5, 1.6, 0.5), 0x4a4038, SHADE_STONE);
        }
    }
}

fn anvil(b: &mut Build, dress: Dress) {
    let c = ANVIL;
    let iron = lin(0x3a3c40);
    // The block it stands on: a stump, a stone, a sunk block.
    match dress {
        Dress::Norse => {
            for k in 0..3 {
                b.put(c + Vec3::new(0.0, 0.2, 0.0), Vec3::new(0.5, 0.4, 0.5), k as f32 * PI / 6.0, lin(0x4a3424), SHADE_WOOD);
            }
            b.hex(c + Vec3::new(0.0, 0.405, 0.0), Vec3::new(0.48, 0.01, 0.48), 0x8a6a4a, SHADE_WOOD);
        }
        Dress::Ottoman => {
            b.stand(c, Vec3::new(0.56, 0.4, 0.5), PALE_STONE[1], SHADE_STONE);
            b.hex(c + Vec3::new(0.0, 0.36, 0.0), Vec3::new(0.6, 0.05, 0.54), 0xb08a3c, SHADE_GOLD);
        }
        Dress::Japanese => {
            b.stand(c, Vec3::new(0.5, 0.4, 0.5), 0x4a3c30, SHADE_WOOD);
        }
    }
    // The anvil: waist, face, horn and heel.
    b.put(c + Vec3::new(0.0, 0.44, 0.0), Vec3::new(0.22, 0.1, 0.18), 0.0, iron, SHADE_METAL);
    b.put(c + Vec3::new(0.0, 0.53, 0.0), Vec3::new(0.16, 0.1, 0.13), 0.0, iron, SHADE_METAL);
    b.put(c + Vec3::new(0.0, 0.59, 0.0), Vec3::new(0.42, 0.06, 0.17), 0.0, lin(0x4a4c52), SHADE_METAL);
    if dress != Dress::Japanese {
        b.seg(c + Vec3::new(0.2, 0.59, 0.0), c + Vec3::new(0.36, 0.57, 0.0), 0.07, iron, SHADE_METAL);
        b.seg(c + Vec3::new(0.34, 0.575, 0.0), c + Vec3::new(0.43, 0.565, 0.0), 0.035, iron, SHADE_METAL);
    }
    b.put(c + Vec3::new(-0.24, 0.59, 0.0), Vec3::new(0.08, 0.05, 0.12), 0.0, iron, SHADE_METAL);
    match dress {
        Dress::Norse => {
            // Runes cut along its waist, a faint amber in them.
            for k in 0..5 {
                let x = -0.08 + k as f32 * 0.04;
                b.glow(c + Vec3::new(x, 0.45 + (k % 2) as f32 * 0.02, 0.092), Vec3::new(0.012, 0.05, 0.004), 0x5a3010, lin(AMBER) * 0.5);
            }
            // Tongs and a hammer laid on the stump.
            b.seg(c + Vec3::new(-0.2, 0.42, 0.18), c + Vec3::new(0.15, 0.42, 0.22), 0.02, iron, SHADE_METAL);
        }
        Dress::Ottoman => {
            b.hex(c + Vec3::new(0.0, 0.63, 0.0), Vec3::new(0.43, 0.012, 0.175), 0x6a6c72, SHADE_METAL);
        }
        Dress::Japanese => {
            // The whetstone on its stand beside it, and a tub of water.
            let w = c + Vec3::new(0.7, 0.0, 0.25);
            b.stand(w, Vec3::new(0.5, 0.3, 0.22), 0x4a3426, SHADE_WOOD);
            b.hex(w + Vec3::new(0.0, 0.33, 0.0), Vec3::new(0.34, 0.06, 0.1), 0x8a8270, SHADE_STONE);
            let tub = c + Vec3::new(0.65, 0.0, -0.4);
            b.stand(tub, Vec3::new(0.34, 0.28, 0.34), 0x5a4232, SHADE_WOOD);
            b.hex(tub + Vec3::new(0.0, 0.27, 0.0), Vec3::new(0.28, 0.02, 0.28), 0x2a3a3c, SHADE_METAL);
        }
    }
}

/// The weapon rack along the back wall: a plank with ten notches and a rail to lean on.
fn rack(b: &mut Build, dress: Dress) {
    let (wood, dark) = dress.wood();
    let c = RACK;
    let len = RACK_STEP * 10.0 + 0.3;
    b.stand(c + Vec3::new(0.0, 0.0, 0.0), Vec3::new(len, 0.12, 0.34), dark, SHADE_WOOD);
    b.hex(c + Vec3::new(0.0, 0.15, 0.0), Vec3::new(len, 0.05, 0.28), wood, SHADE_WOOD);
    for i in 0..=10 {
        let x = c.x - RACK_STEP * 5.0 + i as f32 * RACK_STEP;
        b.hex(Vec3::new(x, 0.2, c.z + 0.08), Vec3::new(0.025, 0.05, 0.12), dark, SHADE_WOOD);
    }
    for s in [-1.0f32, 1.0] {
        b.stand(c + Vec3::new(s * (len / 2.0 - 0.05), 0.0, -0.08), Vec3::new(0.09, 1.25, 0.09), dark, SHADE_WOOD);
    }
    b.hex(c + Vec3::new(0.0, 1.0, -0.12), Vec3::new(len, 0.07, 0.07), wood, SHADE_WOOD);
    if dress == Dress::Ottoman {
        b.hex(c + Vec3::new(0.0, 1.04, -0.12), Vec3::new(len + 0.04, 0.02, 0.09), 0xb08a3c, SHADE_GOLD);
    }
}

/// The shield wall: a backing of boards and two rows of pegs.
fn shield_wall(b: &mut Build, dress: Dress) {
    let (wood, dark) = dress.wood();
    let c = SHIELDS;
    let w = SHIELD_STEP * 5.0 + 0.3;
    b.hex(c + Vec3::new(0.0, 1.55, 0.05), Vec3::new(w, 1.75, 0.06), dark, SHADE_WOOD);
    for (k, &y) in SHIELD_ROWS.iter().enumerate() {
        b.hex(c + Vec3::new(0.0, y + 0.33, 0.09), Vec3::new(w - 0.1, 0.05, 0.04), wood, SHADE_WOOD);
        for i in 0..5 {
            let x = c.x - SHIELD_STEP * 2.0 + i as f32 * SHIELD_STEP;
            b.hex(Vec3::new(x, y + 0.3, c.z + 0.15), Vec3::new(0.04, 0.04, 0.12), 0x2a2624, SHADE_METAL);
            let _ = k;
        }
    }
    // The block shields are reforged on: a small anvil on a stone, tongs, a quench tub.
    let bc = BLOCK;
    b.stand(bc, Vec3::new(0.46, 0.42, 0.42), CHARCOAL[1], SHADE_STONE);
    b.hex(bc + Vec3::new(0.0, 0.47, 0.0), Vec3::new(0.3, 0.1, 0.16), 0x3a3c40, SHADE_METAL);
    b.hex(bc + Vec3::new(0.0, 0.53, 0.0), Vec3::new(0.36, 0.04, 0.18), 0x4a4c52, SHADE_METAL);
    let tub = bc + Vec3::new(-0.55, 0.0, 0.3);
    b.stand(tub, Vec3::new(0.4, 0.34, 0.4), wood, SHADE_WOOD);
    b.hex(tub + Vec3::new(0.0, 0.33, 0.0), Vec3::new(0.34, 0.02, 0.34), 0x22302e, SHADE_METAL);
    b.seg(tub + Vec3::new(-0.1, 0.36, 0.0), tub + Vec3::new(-0.35, 0.75, -0.1), 0.02, lin(0x3a3c40), SHADE_METAL);
}

/// The alchemy bench along the right wall: its top and legs, three shelves over it, a mortar and
/// a copper still. (The bottles are in the wares set.)
fn bench(b: &mut Build, dress: Dress) {
    let (wood, dark) = dress.wood();
    let c = BENCH;
    let len = 2.7;
    b.hex(c + Vec3::new(0.0, BENCH_TOP - 0.04, 0.0), Vec3::new(0.7, 0.08, len), wood, SHADE_WOOD);
    for (dx, dz) in [(-0.3, -1.25), (-0.3, 1.25), (0.3, -1.25), (0.3, 1.25)] {
        b.stand(c + Vec3::new(dx, 0.0, dz), Vec3::new(0.08, BENCH_TOP - 0.08, 0.08), dark, SHADE_WOOD);
    }
    b.hex(c + Vec3::new(0.0, 0.25, 0.0), Vec3::new(0.62, 0.04, len - 0.1), dark, SHADE_WOOD);
    for k in 0..3 {
        let y = 1.4 + k as f32 * 0.42;
        b.hex(Vec3::new(HALF_W - 0.2, y, c.z), Vec3::new(0.3, 0.04, len), wood, SHADE_WOOD);
        for dz in [-1.2f32, 0.0, 1.2] {
            b.hex(Vec3::new(HALF_W - 0.1, y - 0.08, c.z + dz), Vec3::new(0.12, 0.14, 0.03), dark, SHADE_WOOD);
        }
    }
    // A mortar and pestle, and a copper still with its coil.
    let m = c + Vec3::new(-0.1, BENCH_TOP, 0.85);
    b.stand(m, Vec3::new(0.16, 0.1, 0.16), PALE_STONE[0], SHADE_STONE);
    b.seg(m + Vec3::new(0.0, 0.08, 0.0), m + Vec3::new(0.06, 0.24, 0.03), 0.03, lin(PALE_STONE[1]), SHADE_STONE);
    let s = c + Vec3::new(0.05, BENCH_TOP, -0.95);
    b.stand(s, Vec3::new(0.26, 0.26, 0.26), COPPER, SHADE_GOLD);
    b.stand(s + Vec3::new(0.0, 0.26, 0.0), Vec3::new(0.12, 0.14, 0.12), COPPER, SHADE_GOLD);
    b.seg(s + Vec3::new(0.0, 0.42, 0.0), s + Vec3::new(-0.1, 0.36, 0.42), 0.025, lin(COPPER), SHADE_GOLD);
    for k in 0..4 {
        let y = 0.3 - k as f32 * 0.06;
        b.hex(s + Vec3::new(-0.12, y, 0.45), Vec3::new(0.14, 0.02, 0.14), COPPER, SHADE_GOLD);
    }
    // Clay jars along the bench's back and the lowest shelf, and herbs hung to dry.
    for k in 0..7 {
        let kk = h(k as f64, 31.0);
        let z = c.z - 1.2 + k as f32 * 0.4;
        let hgt = 0.14 + kk * 0.12;
        b.stand(
            Vec3::new(HALF_W - 0.28, BENCH_TOP, z),
            Vec3::new(0.13 + kk * 0.05, hgt, 0.13 + kk * 0.05),
            [0x7a5a40, 0x5a4a3c, 0x8a6a4a][k % 3],
            SHADE_STONE,
        );
        if k % 2 == 0 {
            let y = 1.4 + 0.42 * 3.0 - 0.06;
            for j in 0..3 {
                let hz = z + (j as f32 - 1.0) * 0.05;
                b.seg(
                    Vec3::new(HALF_W - 0.3, y - 0.12, hz),
                    Vec3::new(HALF_W - 0.3, y - 0.42 - kk * 0.1, hz + 0.02),
                    0.035,
                    jit(0x5a6a3a, kk, 0.15),
                    SHADE_FUR,
                );
            }
        }
    }
    // The brewing bowl in the middle, where the old man works.
    let bowl = c + Vec3::new(0.05, BENCH_TOP, -0.1);
    b.stand(bowl, Vec3::new(0.26, 0.08, 0.26), 0x5a5c60, SHADE_METAL);
    b.hex(bowl + Vec3::new(0.0, 0.08, 0.0), Vec3::new(0.2, 0.01, 0.2), 0x1a2224, SHADE_METAL);
    let _ = dress;
}

/// The training yard: a ring of chalk on the floor, a sparring post, a straw dummy, and a stand
/// of practice sticks.
fn yard(b: &mut Build, dress: Dress) {
    let (wood, dark) = dress.wood();
    let r = 1.55;
    for k in 0..40 {
        let a = k as f32 / 40.0 * TAU;
        let p = YARD + Vec3::new(a.cos() * r, 0.04, a.sin() * r);
        b.put(p, Vec3::new(0.26, 0.012, 0.05), -a + PI / 2.0, lin(0xc8c2b0) * (0.85 + 0.15 * h(k as f64, 2.0)), SHADE_LIT);
    }
    // The sparring post: a stout pole with arms and a leg of wood to strike against.
    let p = POST;
    b.stand(p, Vec3::new(0.16, 1.55, 0.16), wood, SHADE_WOOD);
    b.stand(p + Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.34, 0.08, 0.34), dark, SHADE_WOOD);
    for (y, yaw, len) in [(1.15f32, 0.5f32, 0.36f32), (1.12, -0.5, 0.36), (0.85, 0.0, 0.3), (0.42, 0.2, 0.34)] {
        let dir = Vec3::new(yaw.sin(), -0.05, yaw.cos()).normalize();
        b.seg(p + Vec3::new(0.0, y, 0.0), p + Vec3::new(0.0, y, 0.0) + dir * len, 0.05, lin(dark), SHADE_WOOD);
    }
    // The dummy: a pole, a cross-bar for arms, a body of straw bound with rope, a sack head.
    let d = DUMMY;
    b.stand(d, Vec3::new(0.08, 1.2, 0.08), dark, SHADE_WOOD);
    b.stand(d, Vec3::new(0.4, 0.06, 0.4), dark, SHADE_WOOD);
    for (y, w) in [(0.72f32, 0.3f32), (0.86, 0.34), (1.0, 0.36), (1.14, 0.33)] {
        b.put(d + Vec3::new(0.0, y, 0.0), Vec3::new(w, 0.15, w * 0.75), 0.3, jit(0x9a8660, h(y as f64, 1.0), 0.08), SHADE_FUR);
    }
    for y in [0.8f32, 1.08] {
        b.put(d + Vec3::new(0.0, y, 0.0), Vec3::new(0.37, 0.03, 0.29), 0.3, lin(0x6a5a3a), SHADE_CLOTH);
    }
    b.put(d + Vec3::new(0.0, 1.12, 0.0), Vec3::new(0.85, 0.06, 0.06), 0.3, lin(dark), SHADE_WOOD);
    b.put(d + Vec3::new(0.0, 1.36, 0.0), Vec3::new(0.2, 0.22, 0.18), 0.3, lin(0x8a7a5a), SHADE_CLOTH);
    // Practice sticks in a stand.
    let s = YARD + Vec3::new(1.4, 0.0, 1.85);
    b.stand(s, Vec3::new(0.4, 0.3, 0.18), dark, SHADE_WOOD);
    for k in 0..4 {
        let x = s.x - 0.12 + k as f32 * 0.08;
        b.seg(Vec3::new(x, 0.25, s.z), Vec3::new(x + 0.05, 1.15, s.z - 0.12), 0.03, lin(wood), SHADE_WOOD);
    }
}

/// The war table: a map of the Portals of Hell, its eight platforms round the volcano in a three
/// by three over the lava, a socket on each sin's platform for its general's sigil; and the
/// Ledger's pinboard behind it on the front wall, its frames empty.
fn war_table(b: &mut Build, dress: Dress) {
    let (wood, dark) = dress.wood();
    let c = TABLE;
    b.hex(c + Vec3::new(0.0, TABLE_TOP - 0.04, 0.0), Vec3::new(1.7, 0.08, 1.2), wood, SHADE_WOOD);
    b.hex(c + Vec3::new(0.0, TABLE_TOP - 0.1, 0.0), Vec3::new(1.6, 0.06, 1.1), dark, SHADE_WOOD);
    for (dx, dz) in [(-0.72, -0.48), (-0.72, 0.48), (0.72, -0.48), (0.72, 0.48)] {
        b.stand(c + Vec3::new(dx, 0.0, dz), Vec3::new(0.1, TABLE_TOP - 0.1, 0.1), dark, SHADE_WOOD);
    }
    // The tray of lava, faintly aglow, and the platforms on it.
    let top = c + Vec3::new(0.0, TABLE_TOP, 0.0);
    b.glow(top + Vec3::new(0.0, 0.015, 0.0), Vec3::new(1.12, 0.03, 1.02), 0x2a0c06, lin(0xff4a1a) * 0.06);
    for col in 0..3 {
        for row in 0..3 {
            let p = top + Vec3::new((col as f32 - 1.0) * 0.36, 0.04, (row as f32 - 1.0) * 0.32);
            if col == 1 && row == 1 {
                // The volcano.
                b.hex(p, Vec3::new(0.24, 0.05, 0.22), 0x2e2a28, SHADE_STONE);
                b.hex(p + Vec3::new(0.0, 0.05, 0.0), Vec3::new(0.16, 0.06, 0.15), 0x3a3230, SHADE_STONE);
                b.hex(p + Vec3::new(0.0, 0.1, 0.0), Vec3::new(0.09, 0.05, 0.08), 0x3a3230, SHADE_STONE);
                b.glow(p + Vec3::new(0.0, 0.13, 0.0), Vec3::new(0.05, 0.02, 0.05), 0x5a1808, lin(0xff5a20) * 1.2);
            } else {
                b.hex(p, Vec3::new(0.28, 0.05, 0.25), PALE_STONE[(col + row) % 2], SHADE_STONE);
            }
        }
    }
    // Sockets for the seven sigils, on the sins' platforms.
    for pl in crate::logic::hub::platforms().iter().filter(|p| p.sin.is_some()) {
        let p = top + Vec3::new((pl.col as f32 - 1.0) * 0.36, 0.075, (pl.row as f32 - 1.0) * 0.32);
        b.hex(p, Vec3::new(0.1, 0.012, 0.1), 0x1e1a18, SHADE_METAL);
    }
    // Two candles at its corners.
    for s in [-1.0f32, 1.0] {
        let p = top + Vec3::new(s * 0.72, 0.0, 0.46);
        b.stand(p, Vec3::new(0.05, 0.12, 0.05), 0xd8ccb0, SHADE_LIT);
    }
    // The pinboard on the front wall: a board in a frame, six empty frames for the Ledger.
    let w = BOARD;
    b.hex(w + Vec3::new(0.0, 1.55, -0.04), Vec3::new(2.1, 1.25, 0.05), 0x5a4436, SHADE_WOOD);
    b.hex(w + Vec3::new(0.0, 1.55, -0.02), Vec3::new(2.22, 1.37, 0.03), dark, SHADE_WOOD);
    for i in 0..3 {
        for j in 0..2 {
            let p = w + Vec3::new(-0.66 + i as f32 * 0.66, 1.84 - j as f32 * 0.58, -0.08);
            for (o, s) in [
                (Vec3::new(0.0, 0.22, 0.0), Vec3::new(0.48, 0.035, 0.02)),
                (Vec3::new(0.0, -0.22, 0.0), Vec3::new(0.48, 0.035, 0.02)),
                (Vec3::new(-0.225, 0.0, 0.0), Vec3::new(0.035, 0.44, 0.02)),
                (Vec3::new(0.225, 0.0, 0.0), Vec3::new(0.035, 0.44, 0.02)),
            ] {
                b.hex(p + o, s, COPPER, SHADE_GOLD);
            }
        }
    }
}

/// The trophy wall on the left wall: ten plaques, each with its hook.
fn trophy_wall(b: &mut Build, dress: Dress) {
    let (wood, dark) = dress.wood();
    let c = TROPHY;
    let w = TROPHY_STEP * 5.0 + 0.3;
    b.hex(c + Vec3::new(0.05, 1.62, 0.0), Vec3::new(0.05, 1.3, w), dark, SHADE_WOOD);
    for &y in &TROPHY_ROWS {
        for i in 0..5 {
            let z = c.z - TROPHY_STEP * 2.0 + i as f32 * TROPHY_STEP;
            let p = Vec3::new(c.x + 0.1, y, z);
            b.hex(p, Vec3::new(0.04, 0.4, 0.34), wood, SHADE_WOOD);
            b.hex(p + Vec3::new(0.0, -0.22, 0.0), Vec3::new(0.04, 0.06, 0.22), wood, SHADE_WOOD);
            b.hex(p + Vec3::new(0.05, 0.12, 0.0), Vec3::new(0.06, 0.03, 0.03), 0x2a2624, SHADE_METAL);
            b.hex(p + Vec3::new(0.09, 0.15, 0.0), Vec3::new(0.02, 0.06, 0.03), 0x2a2624, SHADE_METAL);
        }
    }
}

/// The door in the front wall, shut, a red glow under it from the stair down.
fn door(b: &mut Build, dress: Dress) {
    let (wood, dark) = dress.wood();
    let c = DOOR;
    for s in [-1.0f32, 1.0] {
        b.stand(c + Vec3::new(s * 0.72, 0.0, 0.0), Vec3::new(0.16, 2.3, 0.3), dark, SHADE_WOOD);
    }
    b.hex(c + Vec3::new(0.0, 2.36, 0.0), Vec3::new(1.66, 0.16, 0.32), dark, SHADE_WOOD);
    for k in 0..5 {
        let x = c.x - 0.52 + k as f32 * 0.26;
        b.put(Vec3::new(x, 1.15, c.z + 0.02), Vec3::new(0.25, 2.26, 0.08), 0.0, jit(wood, h(k as f64, 3.0), 0.1) * 0.9, SHADE_WOOD);
    }
    for y in [0.45f32, 1.85] {
        b.hex(Vec3::new(c.x, y, c.z - 0.03), Vec3::new(1.28, 0.08, 0.03), 0x2a2624, SHADE_METAL);
    }
    b.glow(Vec3::new(c.x, 0.012, c.z - 0.06), Vec3::new(1.2, 0.02, 0.04), 0x300800, lin(0xff3a10) * 0.6);
}

/// The lamps' bodies (their light and halos are drawn each frame).
fn lamps(b: &mut Build, dress: Dress) {
    for (n, &p) in LAMPS.iter().enumerate() {
        let wall_out = if p.x.abs() > HALF_W - 0.5 { Vec3::new(p.x.signum(), 0.0, 0.0) } else { Vec3::new(0.0, 0.0, p.z.signum()) };
        match dress {
            Dress::Norse => {
                // An iron cresset on a bracket.
                b.seg(p + wall_out * 0.28 - Vec3::Y * 0.1, p - Vec3::Y * 0.12, 0.03, lin(IRON), SHADE_METAL);
                b.hex(p - Vec3::Y * 0.12, Vec3::new(0.16, 0.06, 0.16), IRON, SHADE_METAL);
                for k in 0..4 {
                    let a = k as f32 * PI / 2.0 + 0.4;
                    b.seg(
                        p - Vec3::Y * 0.1 + Vec3::new(a.cos(), 0.0, a.sin()) * 0.08,
                        p + Vec3::new(a.cos(), 0.0, a.sin()) * 0.11,
                        0.02,
                        lin(IRON),
                        SHADE_METAL,
                    );
                }
            }
            Dress::Ottoman => {
                // A pierced brass lamp hung on a chain.
                b.seg(p + Vec3::Y * 0.25, Vec3::new(p.x, HIGH, p.z), 0.015, lin(0x6a5a3a), SHADE_GOLD);
                b.hex(p + Vec3::Y * 0.18, Vec3::new(0.1, 0.06, 0.1), 0xb08a3c, SHADE_GOLD);
                for (y, r) in [(0.1f32, 0.16f32), (0.0, 0.2), (-0.1, 0.14)] {
                    b.put(p + Vec3::Y * y, Vec3::new(r, 0.08, r), PI / 4.0 * (n % 2) as f32, lin(0xb08a3c), SHADE_GOLD);
                }
                b.hex(p - Vec3::Y * 0.18, Vec3::new(0.04, 0.1, 0.04), 0xb08a3c, SHADE_GOLD);
            }
            Dress::Japanese => {
                // A paper lantern: a frame of dark wood round lit paper.
                let floor_lamp = Vec3::new(p.x, 0.0, p.z) - wall_out * 0.25;
                let lp = floor_lamp + Vec3::Y * 0.55;
                b.glow(lp, Vec3::new(0.26, 0.38, 0.26), 0xe8dcc0, lin(0xffd8a0) * 0.9);
                for (dx, dz) in [(-0.14, -0.14), (-0.14, 0.14), (0.14, -0.14), (0.14, 0.14)] {
                    b.stand(floor_lamp + Vec3::new(dx, 0.0, dz), Vec3::new(0.03, 0.8, 0.03), 0x2e2018, SHADE_WOOD);
                }
                b.hex(lp + Vec3::Y * 0.21, Vec3::new(0.32, 0.03, 0.32), 0x2e2018, SHADE_WOOD);
            }
        }
    }
}

/// Where a lamp's flame (or its paper's middle) is.
fn lamp_light(dress: Dress, p: Vec3) -> Vec3 {
    match dress {
        Dress::Japanese => {
            let wall_out = if p.x.abs() > HALF_W - 0.5 { Vec3::new(p.x.signum(), 0.0, 0.0) } else { Vec3::new(0.0, 0.0, p.z.signum()) };
            Vec3::new(p.x, 0.55, p.z) - wall_out * 0.25
        }
        Dress::Norse => p + Vec3::Y * 0.02,
        Dress::Ottoman => p,
    }
}

/// What each hero's place has on its walls besides the stations.
fn decor(b: &mut Build, dress: Dress) {
    let (wood, dark) = dress.wood();
    match dress {
        Dress::Norse => {
            // Two axes crossed on the back wall over the racks, and a horn on a peg.
            let c = RACK + Vec3::new(0.0, 2.05, 0.38);
            for s in [-1.0f32, 1.0] {
                let a = c + Vec3::new(s * 0.45, -0.4, 0.0);
                let e = c + Vec3::new(-s * 0.45, 0.4, 0.0);
                b.seg(a, e, 0.045, lin(0x5a3a22), SHADE_WOOD);
                let head = e + Vec3::new(s * 0.04, -0.02, 0.0);
                b.put(head, Vec3::new(0.24, 0.16, 0.03), s * 0.7, lin(0x6a6e74), SHADE_METAL);
            }
            b.put(c + Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.1, 0.1, 0.04), PI / 4.0, lin(AMBER), SHADE_GOLD);
            // A drinking horn and furs by the trophy wall; a barrel by the bench.
            b.seg(Vec3::new(-6.85, 1.9, 0.0), Vec3::new(-6.75, 1.65, 0.35), 0.06, lin(0xc8b89a), SHADE_LIT);
            let barrel = Vec3::new(6.35, 0.0, 0.6);
            for k in 0..8 {
                let a = k as f32 / 8.0 * TAU;
                b.put(
                    barrel + Vec3::new(a.cos() * 0.25, 0.4, a.sin() * 0.25),
                    Vec3::new(0.2, 0.8, 0.05),
                    -a + PI / 2.0,
                    jit(0x5a4228, h(k as f64, 1.0), 0.08),
                    SHADE_WOOD,
                );
            }
            for y in [0.15f32, 0.65] {
                b.hex(barrel + Vec3::new(0.0, y, 0.0), Vec3::new(0.56, 0.05, 0.56), 0x2a2624, SHADE_METAL);
            }
        }
        Dress::Ottoman => {
            // Halberds racked by the shield wall, and cushions by the war table.
            let c = Vec3::new(3.6, 0.0, -4.7);
            b.hex(c + Vec3::new(0.35, 0.1, 0.0), Vec3::new(1.0, 0.2, 0.25), dark, SHADE_WOOD);
            b.hex(c + Vec3::new(0.35, 1.6, -0.05), Vec3::new(1.0, 0.06, 0.1), dark, SHADE_WOOD);
            for k in 0..4 {
                let x = c.x + k as f32 * 0.24;
                b.seg(Vec3::new(x, 0.15, c.z), Vec3::new(x, 2.2, c.z - 0.05), 0.035, lin(wood), SHADE_WOOD);
                let top = Vec3::new(x, 2.2, c.z - 0.05);
                b.put(top + Vec3::new(0.06, -0.12, 0.0), Vec3::new(0.14, 0.2, 0.02), 0.0, lin(0x8a9098), SHADE_METAL);
                b.seg(top, top + Vec3::Y * 0.18, 0.02, lin(0x8a9098), SHADE_METAL);
                b.hex(top + Vec3::new(-0.05, -0.08, 0.0), Vec3::new(0.06, 0.06, 0.02), 0x8a9098, SHADE_METAL);
            }
            for (k, &p) in [TABLE + Vec3::new(-1.15, 0.0, 0.2), TABLE + Vec3::new(1.15, 0.0, 0.4)].iter().enumerate() {
                b.stand(p, Vec3::new(0.5, 0.16, 0.5), if k == 0 { 0x7a3424 } else { 0x2e4a4a }, SHADE_CLOTH);
                b.hex(p + Vec3::new(0.0, 0.17, 0.0), Vec3::new(0.08, 0.02, 0.08), AMBER, SHADE_CLOTH);
            }
        }
        Dress::Japanese => {
            // A sword stand on a low chest by the shield wall, and a scroll on the wall.
            let c = Vec3::new(3.9, 0.0, -4.6);
            b.stand(c, Vec3::new(1.2, 0.45, 0.45), dark, SHADE_WOOD);
            for s in [-1.0f32, 1.0] {
                b.stand(c + Vec3::new(s * 0.4, 0.45, 0.0), Vec3::new(0.06, 0.5, 0.2), 0x1e1612, SHADE_WOOD);
            }
            for (k, y) in [0.62f32, 0.78].iter().enumerate() {
                b.put(
                    c + Vec3::new(0.0, *y, 0.0),
                    Vec3::new(0.95, 0.035, 0.04),
                    0.0,
                    lin(if k == 0 { 0x1e1612 } else { 0x2a1a14 }),
                    SHADE_LEATHER,
                );
                b.put(c + Vec3::new(0.3, *y, 0.0), Vec3::new(0.05, 0.05, 0.05), 0.0, lin(0xb08a3c), SHADE_GOLD);
            }
            let scroll = Vec3::new(-1.1, 1.55, -HALF_D + 0.06);
            b.hex(scroll, Vec3::new(0.5, 1.0, 0.02), 0xc8c0a6, SHADE_CLOTH);
            b.hex(scroll + Vec3::new(0.0, 0.52, 0.01), Vec3::new(0.56, 0.04, 0.04), 0x2e2018, SHADE_WOOD);
            for k in 0..4 {
                b.hex(scroll + Vec3::new(0.0, 0.25 - k as f32 * 0.16, 0.012), Vec3::new(0.05, 0.1, 0.01), 0x1a1612, SHADE_LIT);
            }
        }
    }
}

// ------------------------------------------------------------------ what he owns

/// What the wares set shows; rebuilt when it changes.
#[derive(Clone, Copy, PartialEq, Debug)]
struct WaresKey {
    hero: HeroKind,
    owned: [u8; 4],
    belt: [Option<crate::logic::types::PotionKind>; BELT_SLOTS],
    trophies: [Option<u32>; TROPHY_SLOTS],
    sigils: [Option<u32>; 7],
    news: u8,
}

impl WaresKey {
    fn of(w: &WorkshopView) -> Self {
        Self { hero: w.hero, owned: w.owned, belt: w.belt, trophies: w.trophies, sigils: w.sigils, news: w.news }
    }
}

/// A slot's place on the weapon rack (the foot of the blade).
fn rack_slot(tier: u8) -> Vec3 {
    Vec3::new(RACK.x - RACK_STEP * 4.5 + (tier - 1) as f32 * RACK_STEP, 0.2, RACK.z + 0.02)
}

/// A hook's place on the shield wall (the shield's foot hangs from it).
fn shield_slot(tier: u8) -> Vec3 {
    let i = (tier - 1) as usize;
    Vec3::new(SHIELDS.x - SHIELD_STEP * 2.0 + (i % 5) as f32 * SHIELD_STEP, SHIELD_ROWS[i / 5] - 0.33, SHIELDS.z + 0.2)
}

/// A plaque's place on the trophy wall.
fn trophy_slot(i: usize) -> Vec3 {
    Vec3::new(TROPHY.x + 0.14, TROPHY_ROWS[i / 5], TROPHY.z - TROPHY_STEP * 2.0 + (i % 5) as f32 * TROPHY_STEP)
}

/// A chain from `a` to `b`: links of iron, each turned a quarter from the last.
fn chain(b: &mut Build, a: Vec3, e: Vec3) {
    let d = e - a;
    let n = (d.length() / 0.045).ceil().max(1.0) as i32;
    let q = Quat::from_rotation_arc(Vec3::Y, d.normalize_or(Vec3::Y));
    for k in 0..n {
        let p = a + d * ((k as f32 + 0.5) / n as f32);
        let turn = q * Quat::from_rotation_y(if k % 2 == 0 { 0.0 } else { PI / 2.0 });
        b.rot(p, Vec3::new(0.035, 0.05, 0.012), turn, lin(0x3a3c42), SHADE_METAL);
    }
}

/// A rune seal: a disc of pale stone with a rune burning blue in it.
fn seal(b: &mut Build, p: Vec3, facing: Vec3) {
    let q = Quat::from_rotation_arc(Vec3::Z, facing);
    b.rot(p, Vec3::new(0.11, 0.11, 0.03), q * Quat::from_rotation_z(PI / 4.0), lin(PALE_STONE[0]), SHADE_STONE);
    let m = Mat4::from_scale_rotation_translation(Vec3::new(0.016, 0.07, 0.012), q, WORKSHOP + p + facing * 0.018);
    b.out.push(Instance::new(m, lin(0x6ab8ff), 1.0, lin(0x6ab8ff) * 1.6, SHADE_LIT));
    let m = Mat4::from_scale_rotation_translation(Vec3::new(0.05, 0.016, 0.012), q, WORKSHOP + p + facing * 0.018 + Vec3::Y * 0.015);
    b.out.push(Instance::new(m, lin(0x6ab8ff), 1.0, lin(0x6ab8ff) * 1.6, SHADE_LIT));
}

/// A small bottle of `color` standing at `p` (its shape by `kind`, 0..3).
fn bottle(b: &mut Build, p: Vec3, color: u32, kind: usize, scale: f32) {
    let (w, hgt) = [(0.09, 0.11), (0.07, 0.15), (0.11, 0.08), (0.08, 0.12)][kind % 4];
    let (w, hgt) = (w * scale, hgt * scale);
    let glass = lin(color).lerp(lin(0xd0e8f2), 0.25);
    let m = Mat4::from_scale_rotation_translation(Vec3::new(w, hgt, w), Quat::IDENTITY, WORKSHOP + p + Vec3::Y * (hgt / 2.0));
    b.out.push(Instance::new(m, glass, 1.0, lin(color) * 0.35, SHADE_LIT));
    b.hex(p + Vec3::Y * (hgt + 0.025 * scale), Vec3::new(w * 0.4, 0.05 * scale, w * 0.4), 0xd0e8f2, SHADE_LIT);
    b.hex(p + Vec3::Y * (hgt + 0.06 * scale), Vec3::new(w * 0.45, 0.025 * scale, w * 0.45), 0xa87a4a, SHADE_WOOD);
}

/// The set of what he owns and what hangs on the walls.
fn wares_set(key: &WaresKey) -> Vec<Instance> {
    let mut b = Build::new();
    let hero = key.hero;
    let owned = |t: Tree| key.owned[t.index()];
    // The racks: a lit shelf under each blade he has; chains and a seal on the sealed tiers.
    for tier in 1..=TIERS {
        let p = rack_slot(tier);
        if tier <= owned(Tree::Armory) {
            b.glow(p + Vec3::new(0.0, -0.035, 0.1), Vec3::new(0.18, 0.012, 0.03), AMBER, lin(AMBER) * 0.9);
        } else if progression::sealed(tier) {
            chain(&mut b, p + Vec3::new(-0.11, 0.35, 0.08), p + Vec3::new(0.11, 0.75, 0.08));
            chain(&mut b, p + Vec3::new(0.11, 0.35, 0.08), p + Vec3::new(-0.11, 0.75, 0.08));
            seal(&mut b, p + Vec3::new(0.0, 0.55, 0.1), Vec3::Z);
        }
        let s = shield_slot(tier);
        if tier > owned(Tree::Shield) && progression::sealed(tier) {
            chain(&mut b, s + Vec3::new(-0.24, 0.1, 0.06), s + Vec3::new(0.24, 0.52, 0.06));
            chain(&mut b, s + Vec3::new(0.24, 0.1, 0.06), s + Vec3::new(-0.24, 0.52, 0.06));
            seal(&mut b, s + Vec3::new(0.0, 0.31, 0.08), Vec3::Z);
        }
    }
    // The shelves: each elixir learned stands there in its colour; the others are empty flasks,
    // the sealed ones under wax.
    for tier in 1..=TIERS {
        let i = (tier - 1) as usize;
        let shelf = 2 - i / 4;
        let z = BENCH.z - 1.05 + (i % 4) as f32 * 0.7 + if shelf == 0 { 0.35 } else { 0.0 };
        let p = Vec3::new(HALF_W - 0.2, 1.42 + shelf as f32 * 0.42, z);
        let n = progression::node(hero, Tree::Elixir, tier);
        if tier <= owned(Tree::Elixir) {
            bottle(&mut b, p + Vec3::new(0.0, 0.0, -0.12), n.color, i, 1.5);
            bottle(&mut b, p + Vec3::new(0.02, 0.0, 0.12), n.color, i + 1, 1.25);
        } else {
            let m = Mat4::from_scale_rotation_translation(Vec3::new(0.11, 0.14, 0.11), Quat::IDENTITY, WORKSHOP + p + Vec3::Y * 0.07);
            b.out.push(Instance::new(m, lin(0x5a6466), 1.0, Vec3::ZERO, SHADE_METAL));
            if progression::sealed(tier) {
                b.hex(p + Vec3::new(-0.06, 0.08, 0.0), Vec3::new(0.012, 0.06, 0.06), 0x7a2a20, SHADE_LEATHER);
            }
        }
    }
    // His belt laid out on the bench, a bottle in each pocket that holds one.
    let belt = BENCH + Vec3::new(-0.16, BENCH_TOP, 0.55);
    b.hex(belt + Vec3::new(0.0, 0.01, 0.0), Vec3::new(0.08, 0.02, 0.95), 0x4a3020, SHADE_LEATHER);
    b.hex(belt + Vec3::new(0.0, 0.015, 0.5), Vec3::new(0.1, 0.03, 0.08), 0xb08a3c, SHADE_GOLD);
    for (k, slot) in key.belt.iter().enumerate() {
        let p = belt + Vec3::new(0.0, 0.02, -0.38 + k as f32 * 0.19);
        b.hex(p + Vec3::Y * 0.03, Vec3::new(0.11, 0.06, 0.13), 0x5a3a24, SHADE_LEATHER);
        if let Some(kind) = slot {
            bottle(&mut b, p + Vec3::Y * 0.04, gear::potion(hero, *kind).color, k, 0.75);
        }
    }
    // The generals known, a sigil in their sin's colour on their platform.
    for (pl, sigil) in crate::logic::hub::platforms().iter().filter(|p| p.sin.is_some()).zip(key.sigils) {
        if let Some(c) = sigil {
            let p = TABLE + Vec3::new((pl.col as f32 - 1.0) * 0.36, TABLE_TOP + 0.1, (pl.row as f32 - 1.0) * 0.32);
            b.put(p, Vec3::new(0.07, 0.06, 0.07), PI / 4.0, lin(c), SHADE_GOLD);
            b.glow(p + Vec3::Y * 0.04, Vec3::new(0.03, 0.01, 0.03), c, lin(c) * 1.2);
        }
    }
    // News pinned in the Ledger's frames, a note in each.
    for k in 0..key.news.min(6) as usize {
        let p = BOARD + Vec3::new(-0.66 + (k % 3) as f32 * 0.66, 1.84 - (k / 3) as f32 * 0.58, -0.09);
        b.put(p, Vec3::new(0.32, 0.3, 0.01), 0.0, lin(0xd8ccb0), SHADE_CLOTH);
        b.glow(p + Vec3::new(0.0, 0.12, -0.01), Vec3::new(0.03, 0.03, 0.02), 0x8a1a10, lin(0x8a1a10) * 0.2);
    }
    // The trophies on their hooks: a token in each one's colour.
    for (i, t) in key.trophies.iter().enumerate() {
        if let Some(c) = t {
            let p = trophy_slot(i) + Vec3::new(0.06, -0.02, 0.0);
            b.put(p, Vec3::new(0.08, 0.22, 0.16), 0.0, lin(*c), SHADE_METAL);
            b.put(p + Vec3::new(0.02, 0.0, 0.0), Vec3::new(0.06, 0.08, 0.08), PI / 4.0, lin(*c) * 1.2, SHADE_GOLD);
        }
    }
    b.out
}

// ------------------------------------------------------------------ the blades and shields

/// The armory of one hero as models for the racks and the shield wall.
pub(super) struct Racks {
    hero: HeroKind,
    blades: Vec<Rig>,
    shields: Vec<Rig>,
}

impl Racks {
    fn new(hero: HeroKind) -> Self {
        let blades = (1..=TIERS)
            .map(|tier| {
                let n = progression::node(hero, Tree::Armory, tier);
                let mut c = armory::hero_weapon(n.form, n.color, false);
                // Stood on its own: its grip (round x 53.5 of the plan's grid) over the model's middle.
                for v in c.voxels.iter_mut() {
                    v.x -= 24;
                }
                build_rig(&c, &[], ARMS_VOXEL)
            })
            .collect();
        let shields = (1..=TIERS)
            .map(|tier| {
                let n = progression::node(hero, Tree::Shield, tier);
                let mut s = Sculpt::new(60, 92, 36);
                armory::back_shield(&mut s, n.form, n.color, tier);
                let mut c = s.finish(0.03);
                // Off his back: its foot at the model's foot, its thickness about the middle.
                for v in c.voxels.iter_mut() {
                    v.y -= 16;
                    v.z += 15;
                }
                build_rig(&c, &[], ARMS_VOXEL)
            })
            .collect();
        Self { hero, blades, shields }
    }
}

// ------------------------------------------------------------------ the walkers

/// Someone crossing the room: where he is, which way he faces, his stride, and whether he is on
/// his way to the middle first (to go round what stands in the room).
#[derive(Clone, Copy, Debug, Default)]
struct Walker {
    pos: Vec3,
    yaw: f32,
    phase: f32,
    moving: f32,
    via: bool,
    goal: Vec3,
}

impl Walker {
    fn at(pos: Vec3, yaw: f32) -> Self {
        Self { pos, yaw, goal: pos, ..Default::default() }
    }

    /// One step toward `goal` at `speed`, then turning to `face` once there.
    fn step(&mut self, goal: Vec3, face: f32, speed: f32, dt: f32) {
        if goal.distance(self.goal) > 0.01 {
            self.goal = goal;
            // Far, and across the room: by way of the open middle.
            self.via = self.pos.distance(goal) > 3.2 && self.pos.distance(MIDDLE) > 0.6;
        }
        let target = if self.via { MIDDLE } else { goal };
        let d = target - self.pos;
        let dist = d.length();
        if self.via && dist < 0.5 {
            self.via = false;
        }
        if dist > 0.02 {
            let step = (speed * dt).min(dist);
            self.pos += d / dist * step;
            self.yaw = lerp_angle(self.yaw, d.x.atan2(d.z), 1.0 - (-9.0 * dt).exp());
            self.phase += step * 6.5;
            self.moving = (self.moving + dt * 5.0).min(1.0);
        } else {
            self.yaw = lerp_angle(self.yaw, face, 1.0 - (-5.0 * dt).exp());
            self.moving = (self.moving - dt * 4.0).max(0.0);
        }
    }
}

/// The workshop's state in the view: its sets, the armory on its racks, and the three walkers.
#[derive(Default)]
pub(super) struct WorkshopStage {
    sets: [Option<InstanceSet>; 3],
    wares: Option<(WaresKey, InstanceSet)>,
    racks: Option<Racks>,
    old_man: Walker,
    hero: Walker,
    dog: Walker,
    /// Was it up last frame (else the walkers are put straight in their places).
    up: bool,
    /// How settled Quitmer is (lying down), 0..1.
    settle: f32,
}

/// The heading that looks from `from` toward `to`, for a model facing +z.
fn yaw_to(from: Vec3, to: Vec3) -> f32 {
    (to.x - from.x).atan2(to.z - from.z)
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// How high the old man's working arm is raised `t` seconds into a piece of hammering (0 low on
/// the work, 1 over his shoulder), and whether the hammer has just come down (0..1 after a strike).
fn hammer_arm(t: f32) -> f32 {
    let mut raise: f32 = 0.0;
    for (k, &at) in STRIKES.iter().enumerate() {
        let heavy = k == STRIKES.len() - 1;
        let lift = if heavy { 0.5 } else { 0.38 };
        let p = (t - (at - lift)) / lift;
        if (0.0..1.0).contains(&p) {
            // Up slowly, then down fast onto the work.
            let up = smooth(0.0, 0.72, p);
            let down = smooth(0.72, 1.0, p);
            raise = raise.max(up * (1.0 - down) * if heavy { 1.25 } else { 0.95 });
        }
    }
    raise
}

impl WorldView {
    /// The workshop's moving parts: its sets built when first wanted, the armory of the hero
    /// played, the wares, and the old man, the hero and Quitmer walking to their places.
    pub(super) fn update_workshop(&mut self, dt: f32) {
        let Some(w) = self.workshop.filter(|_| self.forge) else {
            self.shop_stage.up = false;
            return;
        };
        let st = &mut self.shop_stage;
        let dress = Dress::of(w.hero);
        if st.sets[dress.index()].is_none() {
            st.sets[dress.index()] = Some(InstanceSet::new(dressing_set(dress)));
        }
        let key = WaresKey::of(&w);
        if st.wares.as_ref().is_none_or(|(k, _)| *k != key) {
            st.wares = Some((key, InstanceSet::new(wares_set(&key))));
        }
        if st.racks.as_ref().is_none_or(|r| r.hero != w.hero) {
            st.racks = Some(Racks::new(w.hero));
        }
        let sp = spot(w.station);
        let face_man = yaw_to(sp.old_man, sp.hero_at());
        let face_hero = yaw_to(sp.hero_at(), (sp.work + sp.old_man) * 0.5);
        let face_dog = yaw_to(sp.dog_at(), sp.hero_at()) - 0.5;
        if !st.up {
            st.up = true;
            st.old_man = Walker::at(sp.old_man, face_man);
            st.hero = Walker::at(sp.hero_at(), face_hero);
            st.dog = Walker::at(sp.dog_at(), face_dog);
            st.settle = 1.0;
        }
        // While he works he turns to the work.
        let face_man = match w.work {
            Some(Work { station, .. }) if station == w.station && st.old_man.pos.distance(sp.old_man) < 0.1 => yaw_to(sp.old_man, sp.work),
            _ => face_man,
        };
        st.old_man.step(sp.old_man, face_man, 2.3, dt);
        st.hero.step(sp.hero_at(), face_hero, 2.8, dt);
        st.dog.step(sp.dog_at(), face_dog, 3.2, dt);
        let lie = st.dog.moving < 0.05 && w.mood == QuitmerMood::Calm;
        st.settle = (st.settle + if lie { dt * 1.2 } else { -dt * 3.0 }).clamp(0.0, 1.0);
    }

    /// The workshop's camera for the station he stands at: framing it on the left of the screen.
    pub(super) fn workshop_camera(&self, t: f32) -> (Vec3, Vec3) {
        let s = self.workshop.map(|w| w.station).unwrap_or_default();
        let sp = spot(s);
        let eye = WORKSHOP + sp.eye + Vec3::new((t * 0.21).sin() * 0.05, (t * 0.27).sin() * 0.025, (t * 0.17).cos() * 0.04);
        let subject = WORKSHOP + sp.subject;
        // Look past the subject to the right, so it sits at 30% of the screen's width.
        let d = subject - eye;
        let right = d.cross(Vec3::Y).normalize_or(Vec3::X);
        let tan_half = (FOV_DEGREES.to_radians() / 2.0).tan() * self.aspect.max(0.5);
        let across = Vec3::new(d.x, 0.0, d.z).length();
        (eye, subject + right * across * tan_half * sp.left)
    }

    /// What the depth of field keeps sharp at the workshop, and the shadow map's reach there.
    pub(super) fn workshop_focus(&self) -> Vec3 {
        WORKSHOP + spot(self.workshop.map(|w| w.station).unwrap_or_default()).subject
    }

    pub(super) fn workshop_shadow_box(&self) -> (Vec3, f32) {
        let s = self.workshop.map(|w| w.station).unwrap_or_default();
        let sp = spot(s);
        (WORKSHOP + (sp.subject + sp.hero_at()) * 0.5 + Vec3::Y * 0.6, 4.6)
    }

    /// The workshop this frame: the set and the wares, the blades and shields, the fire and the
    /// lamps, the old man at his work, the hero and Quitmer.
    pub(super) fn emit_workshop(&self, s: &SceneState, f: &mut Frame) {
        let time = s.time as f32;
        let w = self.workshop.unwrap_or(WorkshopView {
            station: Station::Anvil,
            hero: s.hero_kind,
            owned: [1, 0, 1, 1],
            wield: 1,
            carry: None,
            belt: [None; BELT_SLOTS],
            work: None,
            mood: QuitmerMood::Calm,
            trophies: [None; TROPHY_SLOTS],
            sigils: [None; 7],
            news: 0,
        });
        let st = &self.shop_stage;
        let dress = Dress::of(w.hero);
        let at = |p: Vec3| WORKSHOP + p;
        f.atmosphere = workshop_atmosphere(dress);
        // No haze: its glow round the lamps hung in the air before the stations like ghosts.
        f.haze = Haze::default();
        f.bounds = Vec4::new(WORKSHOP.x - HALF_W, WORKSHOP.z - HALF_D, WORKSHOP.x + HALF_W, WORKSHOP.z + HALF_D);
        if let Some(set) = &st.sets[dress.index()] {
            f.sets.push(set.clone());
        }
        if let Some((_, set)) = &st.wares {
            f.sets.push(set.clone());
        }
        let cube = MeshKind::Cube as usize;

        // The blades on the rack and the shields on the wall: lit if his, in the dark if not.
        if let Some(r) = st.racks.as_ref().filter(|r| r.hero == w.hero) {
            for tier in 1..=TIERS {
                let mine = tier <= w.owned[Tree::Armory.index()];
                // The blade in his hand is not on the rack.
                if !(mine && tier == w.wield) {
                    let p = at(rack_slot(tier));
                    let root = Mat4::from_translation(p) * Mat4::from_rotation_x(-0.1) * Mat4::from_rotation_y(0.5);
                    let tint = if mine { Vec3::ONE } else { Vec3::splat(0.42) };
                    emit_rig(
                        &mut f.rigs,
                        &r.blades[(tier - 1) as usize],
                        &Pose::rest(&r.blades[(tier - 1) as usize]),
                        root,
                        tint,
                        1.0,
                        Vec3::ZERO,
                    );
                }
                let mine = tier <= w.owned[Tree::Shield.index()];
                if !(mine && w.carry == Some(tier)) {
                    let p = at(shield_slot(tier));
                    let root = Mat4::from_translation(p)
                        * Mat4::from_rotation_y(PI)
                        * Mat4::from_rotation_x(0.08)
                        * Mat4::from_scale(Vec3::splat(0.85));
                    let tint = if mine { Vec3::ONE } else { Vec3::splat(0.45) };
                    let rig = &r.shields[(tier - 1) as usize];
                    emit_rig(&mut f.rigs, rig, &Pose::rest(rig), root, tint, 1.0, Vec3::ZERO);
                }
            }
        }

        // The fire.
        let fl = 1.0 + (time * 8.0).sin() * 0.06 + (time * 21.0).sin() * 0.04 + (time * 3.1).sin() * 0.05;
        let fire = at(HEARTH)
            + match dress {
                Dress::Norse => Vec3::new(0.0, 0.45, 0.0),
                Dress::Ottoman => Vec3::new(0.0, 0.12, 0.0),
                Dress::Japanese => Vec3::new(0.0, 0.72, 0.2),
            };
        let flames = if dress == Dress::Japanese { 6 } else { 16 };
        for k in 0..flames {
            let a = time * 3.0 + k as f32 * 1.3;
            let rise = (time * 1.7 + k as f32 * 0.23).fract();
            let spread = if dress == Dress::Japanese { 0.12 } else { 0.22 } * (0.5 + h(k as f64, 5.0));
            let p = fire + Vec3::new(a.cos() * spread * (1.0 - rise * 0.8), rise * 0.45, a.sin() * spread * 0.7 * (1.0 - rise * 0.8));
            let size = 0.07 * (1.0 - rise * 0.7);
            f.additive.push(Instance::new(
                Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(size, size * 1.6, size)),
                lin([0xffc060, 0xff8a30, 0xff6a20][k % 3]) * 0.8,
                0.75 * (1.0 - rise),
                Vec3::ZERO,
                SHADE_UNLIT,
            ));
        }
        glow_orb(f, fire + Vec3::Y * 0.2, 1.1, lin(0xff8a30), 0.32 * fl);
        f.lights.push(Light { pos: fire + Vec3::new(0.5, 0.5, 0.6), color: lin(0xff9a50) * 7.5 * fl, distance: 7.5 });
        // Smoke curling up into the hole in the roof (a cold light falls the other way).
        if dress != Dress::Japanese {
            for k in 0..6 {
                let u = (time * 0.18 + k as f32 / 6.0).fract();
                let p = fire + Vec3::new((time * 0.5 + k as f32).sin() * 0.15, 0.7 + u * 2.0, -u * 0.3);
                f.transparent[cube].push(Instance::new(
                    Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.25 + u * 0.4)),
                    lin(0x3a3a38),
                    0.12 * (1.0 - u),
                    Vec3::ZERO,
                    SHADE_LIT,
                ));
            }
            f.lights.push(Light { pos: at(Vec3::new(-4.5, HIGH - 0.2, -4.4)), color: lin(MIST_TEAL) * 2.0, distance: 5.0 });
        }

        // The lamps.
        for (n, &p) in LAMPS.iter().enumerate() {
            let lp = at(lamp_light(dress, p));
            let flick = 1.0 + (time * 7.0 + n as f32 * 1.9).sin() * 0.05 + (time * 17.0 + n as f32).sin() * 0.03;
            let (c, glow) = match dress {
                Dress::Japanese => (0xffd8a0, 0.22),
                _ => (0xffb060, 0.3),
            };
            if dress != Dress::Japanese {
                f.additive.push(Instance::new(
                    Mat4::from_translation(lp + Vec3::Y * 0.03) * Mat4::from_scale(Vec3::new(0.05, 0.08, 0.05) * flick),
                    lin(0xffd070) * 1.8,
                    0.9,
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
            glow_orb(f, lp, 0.6, lin(c), glow * flick);
            f.lights.push(Light { pos: lp + Vec3::Y * 0.1, color: lin(c) * 4.6 * flick, distance: 6.0 });
        }

        // The old man at the station, the hero and Quitmer.
        let sp = spot(w.station);
        let working = w.work.filter(|k| k.station == w.station && st.old_man.pos.distance(sp.old_man) < 0.15 && st.old_man.moving < 0.2);
        let hand = self.emit_keeper(f, w, st.old_man, working, sp, time);
        if let Some(k) = working {
            self.emit_work(f, k, sp, hand, time);
        }
        self.emit_workshop_hero(f, st.hero, sp, time);
        let (strain, snap, wag, calm) = match w.mood {
            QuitmerMood::Calm => (0.0, 0.0, 0.25, st.settle),
            QuitmerMood::Wag => (0.0, 0.0, 1.0, st.settle * 0.5),
            QuitmerMood::Growl => (0.55, ((time * 3.0).sin() * 0.5 + 0.5) * 0.8, 0.0, 0.0),
        };
        let mut dog_feet = at(st.dog.pos);
        dog_feet.y += (st.dog.phase * 2.0).sin().abs() * 0.025 * st.dog.moving;
        self.emit_dog(f, dog_feet, st.dog.yaw, strain, 0.0, calm, snap, 0.0, wag, time, None);
        if w.mood == QuitmerMood::Growl {
            // Growling at the thing, his hackles up and his eyes hot.
            glow_orb(f, dog_feet + Vec3::new(0.0, 0.7, 0.0), 0.5, lin(0xff5a20), 0.08);
        }

        // A lamp's light over the station's wares, a warm key light on faces from the camera's
        // side, and a cool fill from the door.
        let toward = (sp.eye - sp.subject) * Vec3::new(1.0, 0.0, 1.0);
        let over = at(Vec3::new(sp.subject.x, HIGH - 0.2, sp.subject.z) + toward.normalize_or_zero() * 2.4);
        f.lights.push(Light { pos: over, color: lin(0xffd0a0) * 3.0, distance: 6.0 });
        let key = at(sp.hero_at() + (sp.eye - sp.hero_at()) * 0.4 + Vec3::Y * 1.0);
        f.lights.push(Light { pos: key, color: lin(0xffe0c0) * 2.2, distance: 4.0 });
        f.lights.push(Light { pos: at(DOOR + Vec3::new(0.0, 1.6, -1.2)), color: lin(0x9ab0c8) * 1.6, distance: 5.0 });
    }

    /// The old man: walking (his robe swaying and bobbing), or at his station, talking now and
    /// then, or at his work. Returns where his working hand is.
    fn emit_keeper(&self, f: &mut Frame, w: WorkshopView, me: Walker, work: Option<Work>, sp: Spot, time: f32) -> Vec3 {
        let rig = &self.models.old_man;
        let mut pose = Pose::rest(rig);
        let idx = |n: &str| rig.index(n).unwrap_or(0);
        let (hd, staff, free) = (idx("head"), idx("staffArm"), idx("freeArm"));
        let breathe = (time * 1.6).sin();
        let walk = me.moving;
        // His free arm: lifted for the hammer, reaching over the bench, raised over the yard,
        // swinging as he walks, or talking.
        let (raise, stir) = match work {
            // Level with the anvil's face when the hammer falls.
            Some(k) if k.station.hammers() => (1.16 + hammer_arm(k.t) * 0.8, 0.0),
            Some(k) if k.station == Station::Bench => {
                (0.85 * smooth(0.0, 0.3, k.t) * (1.0 - smooth(WORK_SECONDS - 0.3, WORK_SECONDS, k.t)), 1.0)
            }
            Some(k) => (1.6 * smooth(0.0, 0.4, k.t) * (1.0 - smooth(WORK_SECONDS - 0.4, WORK_SECONDS, k.t)), 0.0),
            None => (0.0, 0.0),
        };
        let at_work = work.is_some();
        pose.rot[free].x = -1.25 * raise + (me.phase).sin() * 0.25 * walk;
        let hammering = work.is_some_and(|k| k.station.hammers());
        pose.rot[free].z = if hammering { 0.04 } else { 0.2 * raise } + stir * (time * 6.0).sin() * 0.15;
        pose.rot[staff].x = breathe * 0.01 - (me.phase).sin() * 0.15 * walk;
        // He looks at the hero, or at his work.
        let to_hero = yaw_to(me.pos, self.shop_stage.hero.pos) - me.yaw;
        let look = if at_work { 0.0 } else { (to_hero + PI).rem_euclid(TAU) - PI };
        pose.rot[hd].y = look.clamp(-0.7, 0.7) * (1.0 - walk);
        pose.rot[hd].x = breathe * 0.02 + if at_work { 0.3 } else { 0.0 };
        let talk = (time % 7.0).min(10.0);
        if !at_work && walk < 0.1 && talk < 4.0 {
            pose.rot[hd].x += (talk * 5.5).sin() * 0.05 * (1.0 - smooth(3.0, 4.0, talk));
            pose.rot[free].x += -0.35 * (1.0 - smooth(3.0, 4.0, talk)) - (talk * 2.7).sin() * 0.2 * (1.0 - smooth(3.0, 4.0, talk));
        }
        let bob = (me.phase * 2.0).sin().abs() * 0.025 * walk;
        let sway = (time * 0.7).sin() * 0.012 + (me.phase).sin() * 0.04 * walk;
        let feet = WORKSHOP + me.pos + Vec3::Y * bob;
        let root = Mat4::from_translation(feet)
            * Mat4::from_rotation_y(me.yaw)
            * Mat4::from_rotation_z(sway)
            * Mat4::from_rotation_x(0.05 + 0.06 * walk);
        emit_rig(&mut f.rigs, rig, &pose, root, Vec3::ONE, 1.0, Vec3::ZERO);
        // His hand, and the hammer in it while he hammers.
        let arm = root * Mat4::from_translation(pose.pos[free]) * euler(pose.rot[free]);
        let pivot = cast::OLD_MAN_RIG[2].pivot;
        let size = super::story::OLD_MAN_VOXEL;
        let local = |p: [f32; 3]| (Vec3::from(p) - Vec3::from(pivot)) * size;
        let hand = arm.transform_point3(local([47.6, 38.0, 24.0]));
        if let Some(k) = work.filter(|k| k.station.hammers()) {
            let _ = k;
            let cube = MeshKind::Cube as usize;
            let grip = local([47.6, 37.0, 24.6]);
            let handle = arm * Mat4::from_translation(grip + Vec3::new(0.0, -0.1, 0.0)) * Mat4::from_scale(Vec3::new(0.025, 0.22, 0.025));
            f.opaque[cube].push(Instance::new(handle, lin(0x5a3e22), 1.0, Vec3::ZERO, SHADE_WOOD));
            let head = arm * Mat4::from_translation(grip + Vec3::new(0.0, -0.22, -0.01)) * Mat4::from_scale(Vec3::new(0.06, 0.06, 0.14));
            f.opaque[cube].push(Instance::new(head, lin(0x3a3c40), 1.0, Vec3::ZERO, SHADE_METAL));
        }
        let _ = (w, sp);
        hand
    }

    /// The work itself: sparks off the anvil at each strike (more on the last), the metal
    /// glowing; a puff of the elixir's colour over the bench; a glint over the yard.
    fn emit_work(&self, f: &mut Frame, k: Work, sp: Spot, hand: Vec3, time: f32) {
        let at = WORKSHOP + sp.work;
        let color = lin(k.color);
        if k.station.hammers() {
            // The metal on the anvil, hot, cooling as the strikes fall.
            let heat = 1.0 - smooth(0.0, WORK_SECONDS, k.t) * 0.6;
            let m = Mat4::from_scale_rotation_translation(Vec3::new(0.24, 0.025, 0.06), Quat::from_rotation_y(0.2), at + Vec3::Y * 0.015);
            f.opaque[MeshKind::Cube as usize].push(Instance::new(m, lin(0x5a2a10), 1.0, lin(0xff7a20) * 1.4 * heat, SHADE_LIT));
            glow_orb(f, at + Vec3::Y * 0.05, 0.3, lin(0xff8a30), 0.15 * heat);
            for (s, &when) in STRIKES.iter().enumerate() {
                let since = k.t - when;
                if !(0.0..0.7).contains(&since) {
                    continue;
                }
                let heavy = s == STRIKES.len() - 1;
                let n = if heavy { 26 } else { 14 };
                for i in 0..n {
                    let seed = (s * 97 + i) as f64;
                    let a = h(seed, 1.0) * TAU;
                    let up = 1.2 + h(seed, 2.0) * 1.6;
                    let out = 0.6 + h(seed, 3.0) * 1.4 * if heavy { 1.4 } else { 1.0 };
                    let life = 0.25 + h(seed, 4.0) * 0.45;
                    if since > life {
                        continue;
                    }
                    let v = Vec3::new(a.cos() * out, up, a.sin() * out);
                    let p = at + Vec3::Y * 0.04 + v * since + Vec3::new(0.0, -4.9 * since * since, 0.0);
                    let fade = 1.0 - since / life;
                    let tail = v.normalize_or(Vec3::Y) * 0.03;
                    f.additive.push(Instance::new(
                        super::forge::segment(p - tail, p + tail, 0.012),
                        lin(if i % 3 == 0 { 0xffe8a0 } else { 0xffa040 }) * 2.0,
                        fade,
                        Vec3::ZERO,
                        SHADE_UNLIT,
                    ));
                }
                let flash = 1.0 - (since / 0.12).min(1.0);
                if flash > 0.0 {
                    glow_orb(f, at + Vec3::Y * 0.06, if heavy { 0.5 } else { 0.35 }, lin(0xffc070), 0.3 * flash);
                    f.lights.push(Light { pos: at + Vec3::new(0.0, 0.3, 0.3), color: lin(0xffb060) * 2.5 * flash, distance: 3.0 });
                }
            }
            f.lights.push(Light { pos: at + Vec3::new(0.0, 0.1, 0.45), color: lin(0xff8a40) * 0.5 * heat, distance: 1.6 });
        } else if k.station == Station::Bench {
            // A puff of the brew's colour rising from the bowl, motes after it.
            let u = smooth(0.0, WORK_SECONDS, k.t);
            glow_orb(f, at + Vec3::Y * (0.08 + u * 0.2), 0.45, color, 0.45 * (1.0 - u));
            for i in 0..10 {
                let fi = i as f32;
                let r = (k.t * 0.8 + fi * 0.1).fract();
                let a = fi * 2.3 + time;
                let p = at + Vec3::new(a.cos() * 0.08, 0.05 + r * 0.6, a.sin() * 0.08);
                glow_orb(f, p, 0.04, color, 0.7 * (1.0 - r) * (1.0 - u));
            }
            f.lights.push(Light { pos: at + Vec3::Y * 0.3, color: color * 3.0 * (1.0 - u), distance: 2.5 });
        } else {
            // A glint at his raised hand.
            let u = smooth(0.0, WORK_SECONDS, k.t);
            glow_orb(f, hand, 0.3, color, 0.5 * (1.0 - u) * smooth(0.0, 0.4, k.t));
        }
    }

    /// The hero: walking to his place at the station, then standing there breathing, his kit on him.
    fn emit_workshop_hero(&self, f: &mut Frame, me: Walker, sp: Spot, time: f32) {
        let rig = &self.models.hero;
        let mut pose = Pose::rest(rig);
        pose.ready(rig);
        let breathe = (time * 1.8).sin();
        if let Some(i) = rig.index("head") {
            pose.rot[i].x = 0.08 + breathe * 0.02;
        }
        let stride = me.phase.sin() * 0.55 * me.moving;
        if let (Some(l), Some(r)) = (rig.index("leftLeg"), rig.index("rightLeg")) {
            pose.rot[l].x = stride;
            pose.rot[r].x = -stride;
        }
        let bob = (me.phase * 2.0).sin().abs() * 0.03 * me.moving;
        let root = Mat4::from_translation(WORKSHOP + me.pos + Vec3::Y * bob)
            * Mat4::from_rotation_y(me.yaw)
            * Mat4::from_scale(Vec3::new(1.0, 1.0 + breathe * 0.006, 1.0));
        emit_rig(&mut f.rigs, rig, &pose, root, Vec3::ONE, 1.0, Vec3::ZERO);
        let _ = sp;
    }
}

/// The workshop's light: no sky, the warm dark of a room lit by its fire and lamps, a little cold
/// light from the smoke hole (none in the Japanese forge, which has a roof of boards).
fn workshop_atmosphere(dress: Dress) -> Atmosphere {
    Atmosphere {
        background: 0x0e0c0a,
        fog_near: 9.0,
        fog_far: 24.0,
        hemi_sky: lin(0x4e5a56) * 0.6,
        hemi_ground: lin(0x2a2018) * 0.3,
        ambient: lin(0x28302e) * 1.15,
        sun_dir: Vec3::new(-0.3, 1.0, -0.2).normalize(),
        sun: lin(MIST_TEAL) * if dress == Dress::Japanese { 0.08 } else { 0.18 },
        exposure: 1.45,
        night_sky: 0.0,
        bloom: 1.0,
        moon_view: Vec3::Y,
        cloud_tint: lin(0x14110e),
    }
}

/// Every station has a place, and the stations are where the six are.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::workshop::STATIONS;

    #[test]
    fn each_station_frames_its_subject_and_keeps_the_cast_in_the_room() {
        for s in STATIONS {
            let sp = spot(s);
            for p in [sp.eye, sp.subject, sp.old_man, sp.hero_at(), sp.dog_at(), sp.work] {
                assert!(p.x.abs() < HALF_W && p.z.abs() < HALF_D, "{s:?}: {p} is outside the room");
            }
            assert!(sp.eye.distance(sp.subject) > 2.5, "{s:?}: the camera stands back");
            assert!(sp.old_man.distance(sp.hero_at()) > 0.5, "{s:?}: the old man and the hero apart");
        }
    }

    #[test]
    fn the_three_dressings_build_moderate_sets() {
        for d in [Dress::Norse, Dress::Ottoman, Dress::Japanese] {
            let n = dressing_set(d).len();
            assert!(n > 500 && n < 9000, "{d:?}: {n} blocks");
        }
        let key = WaresKey {
            hero: HeroKind::Ragnar,
            owned: [3, 2, 1, 4],
            belt: [None; BELT_SLOTS],
            trophies: [None; TROPHY_SLOTS],
            sigils: [None; 7],
            news: 0,
        };
        assert!(wares_set(&key).len() < 2000);
    }

    #[test]
    fn the_hammer_rises_and_falls_three_times() {
        let mut falls = 0;
        let mut was = 0.0;
        let mut rising = false;
        let mut t = 0.0;
        while t < WORK_SECONDS {
            let r = hammer_arm(t);
            if r > was + 1e-4 {
                rising = true;
            } else if rising && r < was - 1e-4 {
                falls += 1;
                rising = false;
            }
            was = r;
            t += 1.0 / 120.0;
        }
        assert_eq!(falls, 3);
    }
}
