//! The Serpent Grotto: Envy's realm (REALMS.md §2). A flooded sea-cave cathedral: six caverns
//! ring the Bell Cavern, where a drowned temple's bronze bell hangs over a black pool.
//!
//! It has two surfaces, laid side by side in one map:
//! - the Waterline (west half, `x < REGION_W`), where he arrives: wading shallows, rib-bridges of
//!   dead sea-serpents over deep water, bone rafts;
//! - the Drowned Floor (east half, `OX` tiles on): the grotto's sunken twin, the same caverns two
//!   storeys down, always to be seen through the clear water (the view draws it beneath the
//!   Waterline).
//!
//! Each surface is whole only where the other is broken, so the true way zigzags between them:
//! deep pools only go down (a dive), geyser vents only go up (an exposed ride). Four crossings
//! bring him from the Mouth to the Bell Cavern: dive in the Rib Chasm, ride up in the Nacre
//! Hall, dive at the Ejder's Spring, ride up in the Jade Well. Every other pool is a lure, under
//! the phantom treasure the water shows.
//!
//! Leviathan's coils thread both surfaces: a hump moving through the deep water above (a bridge
//! that comes and goes), a wall sliding across a passage below. The Eye opens in the water now
//! and then and sweeps the cavern; nazar beads ward it. At the bell the water whispers *"All you
//! want lies below"*, and the bell drops over him (see `Bell` and `game::realms::grotto`).
//!
//! The level's rows keep the grotto as it stood before it flooded: its fallen rib-bridges whole
//! and the shaft beneath the bell open, so the way to the far portal can be checked on foot.
//! Play floods them (`Grotto::flood`), and from then on only the crossings join the surfaces.

use std::collections::HashSet;
use std::f64::consts::{PI, TAU};

use super::{gate, pick_native, Base, Canvas, Pack};
use crate::logic::angels::Archangel;
use crate::logic::enemy::EnemyKind;
use crate::logic::grid::Grid;
use crate::logic::hub::Rect;
use crate::logic::rng::Rng;
use crate::logic::sins::Sin;
use crate::logic::types::{Dir, PotionKind, Tile, CARDINALS, P, V};

/// The layout's seed: the realm is the same every time.
pub const SEED: u32 = 7202;
/// Each surface is a 3x3 of cells this big round the Bell Cavern, inside a margin of rock.
pub const MARGIN: i32 = 2;
pub const CELL_W: i32 = 20;
pub const CELL_H: i32 = 15;
pub const REGION_W: i32 = MARGIN * 2 + 3 * CELL_W;
/// Rock between the two surfaces, and how far east the Drowned Floor's twin of a tile lies.
pub const GAP: i32 = 4;
pub const OX: i32 = REGION_W + GAP;
pub const WIDTH: i32 = OX + REGION_W;
pub const HEIGHT: i32 = MARGIN * 2 + 3 * CELL_H;
/// The refraction lie: from the Waterline the Drowned Floor shows this many tiles toward the
/// camera (south) of where it lies, so a dive lands as far beyond what he aimed at.
pub const REFRACTION: f64 = 1.5;
/// A geyser's ride up, in seconds (he is held on the vent, open to blows, all the while).
pub const RIDE_SECONDS: f64 = 4.0;
/// Seconds after a crossing before another can begin (he must step off and on again).
pub const CROSS_LULL: f64 = 0.6;
/// The Eye: when it first opens, how often, for how long; its reach, the half-width of its cone,
/// and how far either side of south it sweeps (radians).
pub const EYE_FIRST: f64 = 12.0;
pub const EYE_EVERY: f64 = 20.0;
pub const EYE_OPEN: f64 = 6.0;
pub const EYE_RANGE: f64 = 13.0;
pub const EYE_HALF: f64 = 0.34;
pub const EYE_SWEEP: f64 = 1.2;
/// Frames of doom the Eye's gaze lays on him (renewed while it holds him).
pub const GAZE_DOOM: u16 = 420;
/// The bell: the radius of the disc of shallows it first drops on, seconds before the safe circle
/// shrinks by a ring, the scorch (health every so many seconds) on the tiles outside it, the blows
/// on the serpent's head that lift it, how long it stays lifted, how far from its middle the seam
/// under it opens, and the share of his health the bell takes when it crushes him.
pub const BELL_DISC: f64 = 3.5;
pub const RING_SECONDS: f64 = 6.0;
pub const SCORCH_EVERY: f64 = 20.0 / 60.0;
pub const SCORCH_DAMAGE: f64 = 5.0;
pub const HEAD_BLOWS: u32 = 3;
pub const LIFT_SECONDS: f64 = 1.0;
pub const SEAM_RADIUS: f64 = 1.6;
pub const CRUSH_SHARE: f64 = 0.45;
/// Steam vents: a ring of the disc hisses for `STEAM_WARN` seconds, then bursts for
/// `STEAM_BURST`, every `STEAM_EVERY`; a burst scalds him unless he is in the air.
pub const STEAM_EVERY: f64 = 2.2;
pub const STEAM_WARN: f64 = 0.9;
pub const STEAM_BURST: f64 = 0.3;
pub const STEAM_DAMAGE: f64 = 8.0;
/// Where round the bell the serpent's head presses, by turns (radians in the grid's plane; all
/// on its far side, north, where the bronze stands whole above the cut-away the camera sees in by).
pub const HEAD_ANGLES: [f64; 5] = [-2.2, -0.9, -1.6, -2.7, -0.45];
/// How close to the glowing bronze (tiles) his heavy blow must land, and how far off his aim.
pub const HEAD_REACH: f64 = 2.0;
pub const HEAD_ANGLE: f64 = 1.0;

/// The two surfaces of the grotto.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Layer {
    Waterline,
    Drowned,
}

/// Which surface a tile is on.
pub fn layer_of(p: P) -> Layer {
    if p.x >= REGION_W {
        Layer::Drowned
    } else {
        Layer::Waterline
    }
}

/// The drowned twin of a tile of the Waterline.
pub fn below(p: P) -> P {
    P::new(p.x + OX, p.y)
}

/// The Waterline over a tile of the Drowned Floor.
pub fn above(p: P) -> P {
    P::new(p.x - OX, p.y)
}

/// A tile as the Waterline's plan has it (the Drowned Floor's folded back onto it).
pub fn plan(p: P) -> P {
    if layer_of(p) == Layer::Drowned {
        above(p)
    } else {
        p
    }
}

/// Where a point of the Drowned Floor shows when seen through the water from the Waterline:
/// over its twin, shifted toward the camera by the refraction.
pub fn apparent(drowned: V) -> V {
    V::new(drowned.x - OX as f64, drowned.y + REFRACTION)
}

/// A cavern: its name, its middle and cell (on the Waterline's plan), and where the Eye opens in
/// its water.
#[derive(Clone, Debug)]
pub struct Cavern {
    pub name: &'static str,
    pub center: P,
    pub cell: Rect,
    pub eye: P,
}

/// A deep pool on the Waterline: where it is, where a dive from it lands on the Drowned Floor
/// (its twin), and the crossing of the true way it makes (none for a lure).
#[derive(Clone, Copy, Debug)]
pub struct Pool {
    pub at: P,
    pub to: P,
    pub stage: Option<u8>,
}

/// A geyser vent on the Drowned Floor: where it is, where its ride sets him down on the
/// Waterline, and the crossing it makes (none for the way back from a lure).
#[derive(Clone, Copy, Debug)]
pub struct Vent {
    pub at: P,
    pub to: P,
    pub stage: Option<u8>,
}

/// A nazar bead lying in a cavern, until he takes it.
#[derive(Clone, Copy, Debug)]
pub struct Bead {
    pub at: P,
    pub taken: bool,
}

/// Treasure the water shows that is not there.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PhantomKind {
    Chest,
    Bead,
    Gold,
}

/// A phantom: what it seems to be, and where it seems to lie (on the Drowned Floor, for one seen
/// from above; on the Waterline, for one seen from below).
#[derive(Clone, Copy, Debug)]
pub struct Phantom {
    pub at: P,
    pub kind: PhantomKind,
}

/// One of Leviathan's coils: a hump through the deep water above (where it is, the water bears
/// him) or a wall sliding across the floor below (where it is, nothing passes).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CoilKind {
    Hump,
    Wall,
}

/// A coil moves to and fro along its spine, `len` tiles of it at a time, `cross` tiles thick.
#[derive(Clone, Debug)]
pub struct Coil {
    pub kind: CoilKind,
    pub spine: Vec<P>,
    pub cross: Vec<(i32, i32)>,
    pub len: usize,
    /// Tiles a second, and where along its round it starts (seconds).
    pub speed: f64,
    pub phase: f64,
}

impl Coil {
    /// How far along its spine it has come at `t` (the first tile it covers, between whole tiles).
    pub fn start(&self, t: f64) -> f64 {
        let span = self.spine.len().saturating_sub(self.len) as f64;
        if span <= 0.0 {
            return 0.0;
        }
        let s = (t * self.speed + self.phase * self.speed).rem_euclid(2.0 * span);
        if s > span {
            2.0 * span - s
        } else {
            s
        }
    }

    /// Every tile it covers at `t`.
    pub fn covered(&self, t: f64) -> Vec<P> {
        let s = (self.start(t).round() as usize).min(self.spine.len().saturating_sub(self.len));
        self.spine[s..(s + self.len).min(self.spine.len())]
            .iter()
            .flat_map(|p| self.cross.iter().map(move |&(dx, dy)| P::new(p.x + dx, p.y + dy)))
            .collect()
    }

    /// Every tile it can ever cover.
    pub fn track(&self) -> Vec<P> {
        self.spine.iter().flat_map(|p| self.cross.iter().map(move |&(dx, dy)| P::new(p.x + dx, p.y + dy))).collect()
    }
}

/// The bell, from hanging to lifted for good.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BellPhase {
    /// Over the black pool, the portal glowing beneath.
    Hanging,
    /// Down over him, heating, the serpent coiled round it.
    Down,
    /// Lifted for a moment: the seam under it is open.
    Lifted,
    /// He dived through; the bell hangs again, and the portal is real.
    Won,
}

/// Kiyohime's bell (the climax): where it hangs, the black pool under it, where he surfaces if it
/// crushes him, where he comes down on the Drowned Floor through its seam; and, once it has
/// dropped, how it stands: the disc of shallows under it, the serpent's head pressing on it, and
/// the tiles it covered.
#[derive(Clone, Debug)]
pub struct Bell {
    pub center: P,
    pub pool: Vec<P>,
    pub mouth: P,
    pub landing: P,
    pub phase: BellPhase,
    /// Seconds since it dropped (or lifted).
    pub clock: f64,
    /// The radius of the disc of shallows it holds him in (smaller each time it drops again).
    pub disc: f64,
    pub drops: u32,
    /// Heavy blows on the serpent's head since it dropped, and where round the bell the head presses.
    pub blows: u32,
    pub head: f64,
    /// The tiles under it before it dropped.
    pub saved: Vec<(P, Tile)>,
    /// Seconds to the next scorch.
    pub scorch: f64,
}

impl Bell {
    /// How far (tiles) from its middle a point is.
    pub fn dist(&self, at: V) -> f64 {
        let (dx, dy) = (at.x - self.center.x as f64, at.y - self.center.y as f64);
        (dx * dx + dy * dy).sqrt()
    }

    /// The radius of the safe circle now: a ring less every `RING_SECONDS` (below zero, it is gone).
    pub fn safe(&self) -> f64 {
        self.disc - 1.0 - (self.clock / RING_SECONDS).floor()
    }

    /// How hot the bronze is, 0 to 1 (1 as the last of the safe circle goes).
    pub fn heat(&self) -> f64 {
        let whole = (self.disc - 1.0 + 1.0) * RING_SECONDS;
        (self.clock / whole).clamp(0.0, 1.0)
    }

    /// Where the head presses now: the next of `HEAD_ANGLES` for every blow and every drop.
    pub fn press(&mut self) {
        self.head = HEAD_ANGLES[(self.blows + self.drops * 2) as usize % HEAD_ANGLES.len()];
    }

    /// The white-hot patch where the serpent's head presses on the bronze.
    pub fn patch(&self) -> V {
        let r = self.disc + 0.6;
        V::new(self.center.x as f64 + self.head.cos() * r, self.center.y as f64 + self.head.sin() * r)
    }

    /// The tiles of the disc of shallows (radius `disc`) and of the bronze rim round it.
    pub fn disc_tiles(&self, disc: f64) -> (Vec<P>, Vec<P>) {
        let (mut inside, mut rim) = (Vec::new(), Vec::new());
        let r = (disc + 2.0).ceil() as i32;
        for dy in -r..=r {
            for dx in -r..=r {
                let d = ((dx * dx + dy * dy) as f64).sqrt();
                let p = P::new(self.center.x + dx, self.center.y + dy);
                if d <= disc {
                    inside.push(p);
                } else if d <= disc + 1.25 {
                    rim.push(p);
                }
            }
        }
        (inside, rim)
    }

    /// The steam now, if any: the ring (whole tiles from the middle) that hisses, and whether it
    /// bursts this moment (else it only warns).
    pub fn steam(&self) -> Option<(u32, bool)> {
        if self.phase != BellPhase::Down {
            return None;
        }
        let t = self.clock.rem_euclid(STEAM_EVERY);
        let k = (self.clock / STEAM_EVERY) as u32;
        let rings = self.disc.floor() as u32 + 1;
        // Outer and inner by turns, so standing still never answers it.
        let ring = (k * 2 + rings - 1) % rings;
        if t < STEAM_WARN {
            Some((ring, false))
        } else if t < STEAM_WARN + STEAM_BURST {
            Some((ring, true))
        } else {
            None
        }
    }
}

/// What the grotto took from him under the Eye: a bottle, a blessing's charge, or his patron's call.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stolen {
    Potion(PotionKind),
    Blessing(Archangel),
    Patron,
}

/// A native keeping what was stolen (by its duelist's id), until it falls.
#[derive(Clone, Copy, Debug)]
pub struct Theft {
    pub by: u32,
    pub what: Stolen,
}

#[derive(Clone, Debug)]
pub struct Grotto {
    pub base: Base,
    pub caverns: Vec<Cavern>,
    pub pools: Vec<Pool>,
    pub vents: Vec<Vent>,
    pub beads: Vec<Bead>,
    pub phantoms: Vec<Phantom>,
    pub coils: Vec<Coil>,
    /// Bone rafts floating in the deep water of the Waterline (scenery).
    pub rafts: Vec<P>,
    /// Rib-bridges that have fallen into the water, and the shaft under the bell, closed in play.
    pub fallen: Vec<P>,
    pub sealed: Vec<P>,
    pub bell: Bell,
    // ---- in play
    /// Seconds in the realm, and on the Eye's clock (both from his last rising).
    pub time: f64,
    pub eye_clock: f64,
    /// The Eye's opening a bead has warded, and whether the Eye holds him now.
    pub warded: Option<u32>,
    pub seen: bool,
    /// Nazar beads on his belt.
    pub nazar: u32,
    /// A geyser carrying him up: which vent, seconds into the ride.
    pub ride: Option<(usize, f64)>,
    pub thefts: Vec<Theft>,
    /// The crossings of the true way made, and seconds before another crossing may begin.
    pub stage: u8,
    pub lull: f64,
    /// The water has whispered at the bell.
    pub whispered: bool,
    /// His heavy blow now swinging has been counted on the serpent's head.
    pub heavy_counted: bool,
    /// He was being shoved last tick (a kappa's charge begins as the shove does).
    pub shoved: bool,
    /// The Eye's last opening announced, and the last that doomed him.
    pub announced: Option<u32>,
    pub marked: Option<u32>,
    /// The bell's last steam burst that scalded him, and the rings of its safe circle gone.
    pub steam_hit: Option<u32>,
    pub rings_lost: u32,
    /// He has been down to the Drowned Floor (its name is shown the first time).
    pub dived: bool,
    /// What each coil's track is when no coil lies on it, and what each covers now.
    pub coil_base: Vec<Vec<(P, Tile, bool)>>,
    pub covered: Vec<Vec<P>>,
}

/// The cells of the ring, clockwise from the Mouth (where he arrives), and the Bell Cavern's.
const CELLS: [(i32, i32); 7] = [(0, 1), (0, 0), (1, 0), (2, 0), (2, 1), (2, 2), (1, 1)];
const NAMES: [&str; 7] =
    ["The Mouth", "The Rib Chasm", "The Nacre Hall", "The Ejder's Spring", "The Jade Well", "The Coral Choir", "The Bell Cavern"];
/// The Bell Cavern's index in `caverns`.
pub const BELL: usize = 6;

/// The tiles of a cell (on the Waterline's plan).
pub fn cell_rect(col: i32, row: i32) -> Rect {
    let (x0, y0) = (MARGIN + col * CELL_W, MARGIN + row * CELL_H);
    Rect { x0, y0, x1: x0 + CELL_W - 1, y1: y0 + CELL_H - 1 }
}

fn cell_center(col: i32, row: i32) -> P {
    P::new(MARGIN + col * CELL_W + CELL_W / 2, MARGIN + row * CELL_H + CELL_H / 2)
}

/// A cavern's floor: an ellipse round `c`, its edge rippled, kept a tile inside its cell.
fn blob(rng: &mut Rng, c: P, rx: f64, ry: f64, cell: Rect) -> Vec<P> {
    let ph: [f64; 3] = [rng.range(0.0, TAU), rng.range(0.0, TAU), rng.range(0.0, TAU)];
    let mut out = Vec::new();
    for y in cell.y0 + 1..cell.y1 {
        for x in cell.x0 + 1..cell.x1 {
            let (dx, dy) = ((x - c.x) as f64 / rx, (y - c.y) as f64 / ry);
            let a = dy.atan2(dx);
            let r = 1.0 + 0.09 * (2.0 * a + ph[0]).sin() + 0.06 * (3.0 * a + ph[1]).sin() + 0.04 * (5.0 * a + ph[2]).sin();
            if dx * dx + dy * dy <= r * r {
                out.push(P::new(x, y));
            }
        }
    }
    out
}

/// A disc of tiles round `c`.
fn disc(c: P, r: f64) -> Vec<P> {
    let n = r.ceil() as i32;
    let mut out = Vec::new();
    for dy in -n..=n {
        for dx in -n..=n {
            if ((dx * dx + dy * dy) as f64) <= r * r {
                out.push(P::new(c.x + dx, c.y + dy));
            }
        }
    }
    out
}

/// A straight band between `a` and `b` (in a row or a column), `half` tiles either side, every
/// rock tile of it set to `glyph`; the tiles changed.
fn passage(c: &mut Canvas, a: P, b: P, half: i32, glyph: char) -> Vec<P> {
    let mut out = Vec::new();
    let (x0, x1, y0, y1) = if a.y == b.y {
        (a.x.min(b.x), a.x.max(b.x), a.y - half, a.y + half)
    } else {
        (a.x - half, a.x + half, a.y.min(b.y), a.y.max(b.y))
    };
    for y in y0..=y1 {
        for x in x0..=x1 {
            let p = P::new(x, y);
            if c.get(p) == '#' {
                c.set(p, glyph);
                out.push(p);
            }
        }
    }
    out
}

impl Grotto {
    /// The realm laid out for `tier`.
    pub fn generate(tier: usize) -> Grotto {
        let mut rng = Rng::new(SEED);
        let mut c = Canvas::new(WIDTH, HEIGHT);
        let d = |p: P| below(p);

        // ---- the caverns: each the same shape on both surfaces (the sunken twin), the Bell
        // Cavern a round hall on each
        let mut caverns = Vec::new();
        let mut floors: Vec<Vec<P>> = Vec::new();
        for (k, &(col, row)) in CELLS.iter().enumerate() {
            let cell = cell_rect(col, row);
            let center = cell_center(col, row);
            let tiles = if k == BELL {
                disc(center, 6.5)
            } else {
                let (rx, ry) = if k == 3 { (8.0, 6.0) } else { (7.6, 5.6) };
                blob(&mut rng, center, rx, ry, cell)
            };
            for &p in &tiles {
                c.set(p, ' ');
                if k != BELL {
                    c.set(d(p), ' ');
                }
            }
            caverns.push(Cavern { name: NAMES[k], center, cell, eye: P::new(center.x, center.y - 4) });
            floors.push(tiles);
        }
        let bell_center = caverns[BELL].center;
        // The Drowned Floor of the Bell Cavern: a round hall with the court's portal standing in
        // it, directly beneath the bell.
        for p in disc(bell_center, 6.0) {
            c.set(d(p), ' ');
        }
        let ctr = |k: usize| caverns[k].center;

        // ---- the Waterline's passages (the Drowned Floor has its own)
        let mut bridges_w: Vec<P> = Vec::new();
        passage(&mut c, ctr(0), ctr(1), 1, ' ');
        passage(&mut c, P::new(ctr(1).x, 9), P::new(ctr(2).x, 9), 1, ' ');
        // The Nacre Hall to the Spring: a rib-bridge over deep water, whole.
        for p in passage(&mut c, P::new(ctr(2).x, 6), P::new(ctr(3).x, 6), 1, '~') {
            if (p.x - (ctr(2).x + ctr(3).x) / 2).abs() <= 8 {
                c.set(p, '=');
                bridges_w.push(p);
            }
        }
        passage(&mut c, ctr(4), ctr(BELL), 1, ' ');
        passage(&mut c, ctr(4), ctr(5), 1, ' ');
        // The Spring to the Jade Well: its ribs have fallen.
        let mut fallen: Vec<P> = Vec::new();
        for p in passage(&mut c, ctr(3), ctr(4), 1, '~') {
            if p.x == ctr(3).x {
                c.set(p, '=');
                fallen.push(p);
            }
        }
        // His way in: the portal in the Mouth's west wall, a short hall to the cavern.
        let entry = gate(Sin::Envy, P::new(MARGIN, ctr(0).y), Dir::Right);
        let arrival = P::new(MARGIN + 1, ctr(0).y);
        passage(&mut c, arrival, ctr(0), 1, ' ');

        // ---- where the Waterline is broken
        // The Rib Chasm: deep water from wall to wall, its ribs fallen.
        for &p in &floors[1] {
            if p.x == ctr(1).x + 3 || p.x == ctr(1).x + 4 {
                if (8..=10).contains(&p.y) {
                    c.set(p, '=');
                    fallen.push(p);
                } else {
                    c.set(p, '~');
                }
            }
        }
        // The Nacre Hall: a channel cutting off a nook at its south (the hump crosses it).
        for &p in &floors[2] {
            if p.y == 10 || p.y == 11 {
                c.set(p, '~');
            }
        }
        // The Ejder's Spring: the spring's channel across the cavern, its ribs fallen.
        for &p in &floors[3] {
            if p.y == 10 || p.y == 11 {
                if (47..=49).contains(&p.x) {
                    c.set(p, '=');
                    fallen.push(p);
                } else {
                    c.set(p, '~');
                }
            }
        }
        // The Jade Well: a deep channel under a whole rib-bridge.
        for &p in &floors[4] {
            if p.x == ctr(4).x - 5 || p.x == ctr(4).x - 4 {
                if (23..=25).contains(&p.y) {
                    c.set(p, '=');
                    bridges_w.push(p);
                } else {
                    c.set(p, '~');
                }
            }
        }
        // Ponds in the Mouth and the Coral Choir, each round an island with a bead on it.
        let isle = [P::new(8, 20), P::new(56, 37)];
        for &i in &isle {
            for p in disc(i, 2.3) {
                if c.get(p) == ' ' {
                    c.set(p, '~');
                }
            }
            c.set(i, ' ');
        }
        // The black pool under the bell.
        let bell_pool: Vec<P> = disc(bell_center, 2.6);

        // ---- the Drowned Floor's passages: from the Rib Chasm to the Nacre Hall, from the Spring
        // to the Jade Well, from the Jade Well to the Coral Choir (nowhere else)
        passage(&mut c, d(P::new(ctr(1).x, 9)), d(P::new(ctr(2).x, 9)), 1, ' ');
        passage(&mut c, d(ctr(3)), d(ctr(4)), 1, ' ');
        passage(&mut c, d(ctr(4)), d(ctr(5)), 1, ' ');

        // ---- Leviathan's coils
        let row = |y: i32, x0: i32, x1: i32| (x0..=x1).map(|x| P::new(x, y)).collect::<Vec<P>>();
        let col = |x: i32, y0: i32, y1: i32| (y0..=y1).map(|y| P::new(x, y)).collect::<Vec<P>>();
        let mut coils = vec![
            // The Mouth: a hump through the pond, the only way onto the island.
            Coil { kind: CoilKind::Hump, spine: row(20, 6, 10), cross: vec![(0, 0)], len: 2, speed: 0.8, phase: 0.0 },
            // The Rib Chasm to the Nacre Hall, below: a wall across the passage.
            Coil {
                kind: CoilKind::Wall,
                spine: col(d(P::new(20, 0)).x, 3, 15),
                cross: vec![(0, 0), (1, 0)],
                len: 3,
                speed: 1.1,
                phase: 3.0,
            },
            // The Nacre Hall: a hump through the channel to the nook.
            Coil { kind: CoilKind::Hump, spine: row(10, 25, 39), cross: vec![(0, 0), (0, 1)], len: 2, speed: 1.2, phase: 2.0 },
            // The Ejder's Spring: a hump along the spring's channel.
            Coil { kind: CoilKind::Hump, spine: row(10, 45, 59), cross: vec![(0, 0), (0, 1)], len: 2, speed: 1.3, phase: 5.0 },
            // The Spring to the Jade Well, below: a wall across the passage.
            Coil {
                kind: CoilKind::Wall,
                spine: row(16, d(P::new(46, 0)).x, d(P::new(58, 0)).x),
                cross: vec![(0, 0), (0, 1)],
                len: 3,
                speed: 1.0,
                phase: 1.0,
            },
            // The Coral Choir: a hump through the pond to its island.
            Coil { kind: CoilKind::Hump, spine: row(37, 53, 59), cross: vec![(0, 0)], len: 2, speed: 0.9, phase: 4.0 },
        ];
        // A wall's track runs into the rock either side of its passage: hollow it out.
        for coil in coils.iter().filter(|k| k.kind == CoilKind::Wall) {
            for p in coil.track() {
                if c.get(p) == '#' {
                    c.set(p, ' ');
                }
            }
        }
        // A hump only moves through water (and over the islands it reaches).
        for coil in coils.iter_mut().filter(|k| k.kind == CoilKind::Hump) {
            coil.spine.retain(|p| matches!(c.get(*p), '~' | '=') || isle.contains(p));
        }

        // ---- the crossings
        let pool = |at: P, stage: Option<u8>| Pool { at, to: d(at), stage };
        let pools = vec![pool(P::new(15, 27), None), pool(P::new(9, 6), Some(1)), pool(P::new(54, 7), Some(3)), pool(P::new(48, 41), None)];
        let vent = |at: P, stage: Option<u8>| Vent { at: d(at), to: at, stage };
        let vents =
            vec![vent(P::new(9, 26), None), vent(P::new(34, 8), Some(2)), vent(P::new(55, 27), Some(4)), vent(P::new(55, 41), None)];
        for p in pools.iter().flat_map(|q| [q.at, q.to]).chain(vents.iter().flat_map(|v| [v.at, v.to])) {
            c.set(p, ' ');
        }

        // ---- nazar beads, about one to a cavern, on either surface
        let beads: Vec<Bead> = [isle[0], d(P::new(6, 11)), P::new(32, 13), P::new(55, 13), d(P::new(49, 22)), isle[1]]
            .into_iter()
            .map(|at| Bead { at, taken: false })
            .collect();

        // ---- deep water opening in the Waterline's shelves (windows on the twin below), wherever
        // it cuts no way off: a cavern's floor must hang together as it did
        let fallen_set: HashSet<P> = fallen.iter().copied().collect();
        let mut keep_dry: HashSet<P> =
            pools.iter().map(|q| q.at).chain(vents.iter().map(|v| v.to)).chain(beads.iter().map(|b| b.at)).collect();
        keep_dry.extend(coils.iter().flat_map(|k| k.track()));
        keep_dry.extend(isle);
        for k in 0..BELL {
            let cell = caverns[k].cell;
            let walk = |c: &Canvas, p: P, whole: bool| {
                matches!(c.get(p), ' ' | 'P' | 'G') || (c.get(p) == '=' && (whole || !fallen_set.contains(&p)))
            };
            let pieces = |c: &Canvas, whole: bool| {
                let mut seen: HashSet<P> = HashSet::new();
                let mut n = 0;
                for y in cell.y0 - 1..=cell.y1 + 1 {
                    for x in cell.x0 - 1..=cell.x1 + 1 {
                        let p = P::new(x, y);
                        if !walk(c, p, whole) || !seen.insert(p) {
                            continue;
                        }
                        n += 1;
                        let mut stack = vec![p];
                        while let Some(q) = stack.pop() {
                            for dd in CARDINALS {
                                let r = q.step(dd);
                                if r.x >= cell.x0 - 1
                                    && r.x <= cell.x1 + 1
                                    && r.y >= cell.y0 - 1
                                    && r.y <= cell.y1 + 1
                                    && walk(c, r, whole)
                                    && seen.insert(r)
                                {
                                    stack.push(r);
                                }
                            }
                        }
                    }
                }
                n
            };
            let before = (pieces(&c, false), pieces(&c, true));
            let mut tries: Vec<P> = floors[k].clone();
            rng.shuffle(&mut tries);
            let mut made = 0;
            for at in tries {
                if made >= 3 {
                    break;
                }
                let spot = disc(at, 1.7);
                let near_dry = spot.iter().any(|p| keep_dry.iter().any(|q| (q.x - p.x).abs() <= 1 && (q.y - p.y).abs() <= 1));
                if near_dry || spot.iter().any(|p| c.get(*p) != ' ' || !floors[k].contains(p)) {
                    continue;
                }
                for &p in &spot {
                    c.set(p, '~');
                }
                if (pieces(&c, false), pieces(&c, true)) == before {
                    made += 1;
                } else {
                    for &p in &spot {
                        c.set(p, ' ');
                    }
                }
            }
        }

        // ---- the phantoms: chests and beads shown below where nothing lies, gold heaped on the
        // rafts seen from below, the treasure of the lures most of all
        let mut phantoms = vec![
            Phantom { at: d(P::new(14, 24)), kind: PhantomKind::Chest },
            Phantom { at: d(P::new(16, 25)), kind: PhantomKind::Chest },
            Phantom { at: d(P::new(13, 28)), kind: PhantomKind::Bead },
            Phantom { at: d(P::new(30, 6)), kind: PhantomKind::Chest },
            Phantom { at: d(P::new(57, 13)), kind: PhantomKind::Bead },
            Phantom { at: d(P::new(47, 39)), kind: PhantomKind::Chest },
            Phantom { at: d(P::new(50, 43)), kind: PhantomKind::Bead },
            Phantom { at: d(P::new(46, 42)), kind: PhantomKind::Bead },
        ];
        // Bone rafts adrift in the deep water, every one of them golden from below.
        let tracks: HashSet<P> = coils.iter().flat_map(|k| k.track()).collect();
        let mut deep: Vec<P> = (0..HEIGHT)
            .flat_map(|y| (0..REGION_W).map(move |x| P::new(x, y)))
            .filter(|&p| c.get(p) == '~' && !tracks.contains(&p))
            .filter(|&p| CARDINALS.iter().filter(|&&dd| c.get(p.step(dd)) == '~').count() >= 2)
            .collect();
        rng.shuffle(&mut deep);
        let mut rafts: Vec<P> = Vec::new();
        for p in deep {
            if rafts.len() >= 12 {
                break;
            }
            if rafts.iter().all(|r| (r.x - p.x).abs() + (r.y - p.y).abs() >= 4) {
                rafts.push(p);
                phantoms.push(Phantom { at: p, kind: PhantomKind::Gold });
            }
        }

        // ---- the bell, and the court's portal on the Drowned Floor beneath it
        let exit = gate(Sin::Envy, d(P::new(bell_center.x, bell_center.y - 2)), Dir::Down);
        for t in exit.towers() {
            c.set(t, '#');
        }
        c.set(exit.tile, 'D');
        // The shaft under the bell, down to the court (closed in play): through the rock south of
        // both halls, along the margin.
        let mut sealed: Vec<P> = Vec::new();
        let foot = HEIGHT - MARGIN;
        sealed.extend(passage(&mut c, P::new(bell_center.x, bell_center.y + 6), P::new(bell_center.x, foot), 0, ' '));
        sealed.extend(passage(&mut c, P::new(bell_center.x, foot), d(P::new(bell_center.x, foot)), 0, ' '));
        sealed.extend(passage(&mut c, d(P::new(bell_center.x, foot)), d(P::new(bell_center.x, bell_center.y + 5)), 0, ' '));
        let bell = Bell {
            center: bell_center,
            pool: bell_pool,
            mouth: P::new(bell_center.x + 6, bell_center.y),
            landing: d(P::new(bell_center.x, bell_center.y + 1)),
            phase: BellPhase::Hanging,
            clock: 0.0,
            disc: BELL_DISC,
            drops: 0,
            blows: 0,
            head: HEAD_ANGLES[0],
            saved: Vec::new(),
            scorch: 0.0,
        };

        // ---- the Drowned Floor broken where the Waterline is whole: rockfalls in its caverns
        let keep: HashSet<P> = pools
            .iter()
            .map(|q| q.to)
            .chain(vents.iter().map(|v| v.at))
            .chain(beads.iter().map(|b| b.at))
            .chain(tracks.iter().copied())
            .chain(phantoms.iter().map(|p| p.at))
            .collect();
        let near_keep = |p: P| keep.iter().any(|k| (k.x - p.x).abs() <= 2 && (k.y - p.y).abs() <= 2);
        for k in [0usize, 2, 4, 5] {
            for _ in 0..3 {
                let f = &floors[k];
                let p = d(f[rng.index(f.len())]);
                if near_keep(p) {
                    continue;
                }
                // Never across a cavern's passages: only well inside it.
                let inner = |q: P| CARDINALS.iter().all(|&dd| c.get(q.step(dd)) != '#') && floors[k].contains(&above(q));
                if inner(p) && (p.x - d(ctr(k)).x).abs() >= 2 {
                    c.set(p, '#');
                }
            }
        }

        // ---- the packs: on both surfaces, in every cavern but the Mouth's shallows and the bell's
        let mut packs = Vec::new();
        let mut busy: HashSet<P> = keep.clone();
        busy.insert(arrival);
        let plan_dens: [(usize, Layer, usize); 11] = [
            (0, Layer::Drowned, 2),
            (1, Layer::Waterline, 2),
            (1, Layer::Drowned, 3),
            (2, Layer::Drowned, 3),
            (2, Layer::Waterline, 2),
            (3, Layer::Waterline, 3),
            (3, Layer::Drowned, 3),
            (4, Layer::Drowned, 3),
            (4, Layer::Waterline, 3),
            (5, Layer::Waterline, 2),
            (5, Layer::Drowned, 3),
        ];
        for &(k, layer, count) in &plan_dens {
            let shift = if layer == Layer::Drowned { OX } else { 0 };
            let cell = caverns[k].cell;
            let den = Rect { x0: cell.x0 + shift, y0: cell.y0, x1: cell.x1 + shift, y1: cell.y1 };
            let mut spots: Vec<P> = floors[k]
                .iter()
                .map(|&p| P::new(p.x + shift, p.y))
                .filter(|&p| c.get(p) == ' ' && !busy.contains(&p) && !near_keep(p))
                .filter(|&p| CARDINALS.iter().all(|&dd| c.get(p.step(dd)) == ' '))
                .collect();
            rng.shuffle(&mut spots);
            let mut foes = Vec::new();
            for p in spots {
                if foes.len() >= count {
                    break;
                }
                if foes.iter().any(|f: &(EnemyKind, u8, P)| (f.2.x - p.x).abs() + (f.2.y - p.y).abs() < 3) {
                    continue;
                }
                let (kind, stage) = pick_native(&mut rng, Sin::Envy, tier);
                busy.insert(p);
                foes.push((kind, stage, p));
            }
            if !foes.is_empty() {
                packs.push(Pack { den, foes });
            }
        }

        // ---- the level's marks: him, the hollow's spawn (required, unused here), the far portal
        c.set(arrival, 'P');
        c.set(P::new(arrival.x + 2, arrival.y + 1), 'G');
        let (rows, heights) = c.finish();
        let base =
            Base { sin: Sin::Envy, width: WIDTH, height: HEIGHT, rows, heights, arrival, entry, exit, packs, checkpoint: arrival, tier };
        let covered = vec![Vec::new(); coils.len()];
        Grotto {
            base,
            caverns,
            pools,
            vents,
            beads,
            phantoms,
            coils,
            rafts,
            fallen,
            sealed,
            bell,
            time: 0.0,
            eye_clock: 0.0,
            warded: None,
            seen: false,
            nazar: 0,
            ride: None,
            thefts: Vec::new(),
            stage: 0,
            lull: 0.0,
            whispered: false,
            heavy_counted: false,
            shoved: false,
            announced: None,
            marked: None,
            steam_hit: None,
            rings_lost: 0,
            dived: false,
            coil_base: Vec::new(),
            covered,
        }
    }

    /// Floods the grotto as play begins: the fallen ribs are deep water, the bell's shaft rock.
    pub fn flood(&self, grid: &mut Grid) {
        for p in &self.fallen {
            grid.set(p.x, p.y, Tile::Pit);
            grid.set_bridge(p.x, p.y, false);
        }
        for p in &self.sealed {
            grid.set(p.x, p.y, Tile::Rock);
        }
    }

    /// The cavern a tile lies in (either surface), if any.
    pub fn cavern_at(&self, p: P) -> Option<usize> {
        let q = plan(p);
        self.caverns.iter().position(|c| c.cell.contains(q))
    }

    /// The deep pool on tile `p`, if there is one.
    pub fn pool_at(&self, p: P) -> Option<usize> {
        self.pools.iter().position(|q| q.at == p)
    }

    /// The geyser vent on tile `p`, if there is one.
    pub fn vent_at(&self, p: P) -> Option<usize> {
        self.vents.iter().position(|v| v.at == p)
    }

    /// Where a fall into the deep water at `p` (on the Waterline) comes down: its twin, or the
    /// floor nearest it in the same cavern (never in the Bell Cavern's, which only the bell's seam reaches).
    pub fn sink_to(&self, grid: &Grid, p: P) -> P {
        let want = below(plan(p));
        let ok = |q: P| grid.is_walkable(q.x, q.y, false) && layer_of(q) == Layer::Drowned && self.cavern_at(q).is_some_and(|k| k != BELL);
        if ok(want) {
            return want;
        }
        for r in 1..8 {
            for dy in -r..=r {
                for dx in -r..=r {
                    let q = P::new(want.x + dx, want.y + dy);
                    if (dx.abs() == r || dy.abs() == r) && ok(q) {
                        return q;
                    }
                }
            }
        }
        self.pools.iter().find(|q| q.stage.is_some()).map(|q| q.to).unwrap_or(want)
    }

    /// The Eye's opening under way and how far through it is (0 to 1), if it is open.
    pub fn gaze(&self) -> Option<(u32, f64)> {
        gaze(self.eye_clock)
    }

    /// Where the Eye looks from, in the cavern of tile `p` (on that tile's surface; in the Bell
    /// Cavern, from the bell's sound-hole), and which way its cone points now.
    pub fn eye_for(&self, p: P) -> Option<(V, f64)> {
        let (_, t) = self.gaze()?;
        let k = self.cavern_at(p)?;
        let e = self.caverns[k].eye;
        let e = if layer_of(p) == Layer::Drowned { below(e) } else { e };
        Some((e.v(), PI / 2.0 + EYE_SWEEP * (2.0 * t - 1.0)))
    }
}

/// The Eye on its clock: the opening under way and how far through it is, if it is open.
pub fn gaze(clock: f64) -> Option<(u32, f64)> {
    if clock < EYE_FIRST {
        return None;
    }
    let c = clock - EYE_FIRST;
    let t = c.rem_euclid(EYE_EVERY);
    (t < EYE_OPEN).then(|| ((c / EYE_EVERY) as u32, t / EYE_OPEN))
}

/// Seconds until the Eye opens next (0 while it is open).
pub fn until_gaze(clock: f64) -> f64 {
    if clock < EYE_FIRST {
        return EYE_FIRST - clock;
    }
    let t = (clock - EYE_FIRST).rem_euclid(EYE_EVERY);
    if t < EYE_OPEN {
        0.0
    } else {
        EYE_EVERY - t
    }
}

/// Does the Eye at `eye`, looking along `angle`, see `at`?
pub fn in_cone(eye: V, angle: f64, at: V) -> bool {
    let (dx, dy) = (at.x - eye.x, at.y - eye.y);
    let d = (dx * dx + dy * dy).sqrt();
    if d > EYE_RANGE {
        return false;
    }
    if d < 0.8 {
        return true;
    }
    let mut off = (dy.atan2(dx) - angle).rem_euclid(TAU);
    if off > PI {
        off = TAU - off;
    }
    off <= EYE_HALF
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::parse_level;
    use crate::logic::levels::level_for;
    use std::collections::VecDeque;

    fn flooded() -> (Grotto, Grid) {
        let g = Grotto::generate(1);
        let mut grid = parse_level(&g.base.level(&level_for(1))).grid;
        g.flood(&mut grid);
        (g, grid)
    }

    /// The fewest crossings that bring him from `from` to each tile, walking where he can, a hump
    /// counted as there when it comes and a wall as gone when it goes.
    fn crossings(g: &Grotto, grid: &Grid, from: P) -> std::collections::HashMap<P, u32> {
        let mut grid = grid.clone();
        for k in &g.coils {
            for p in k.track() {
                grid.set(p.x, p.y, Tile::Open);
            }
        }
        let mut best = std::collections::HashMap::from([(from, 0u32)]);
        let mut queue = VecDeque::from([(from, 0u32)]);
        while let Some((p, n)) = queue.pop_front() {
            if best.get(&p).is_some_and(|&b| b < n) {
                continue;
            }
            let mut next: Vec<(P, u32)> =
                CARDINALS.iter().filter(|&&dd| grid.can_move(p, dd, false, false)).map(|&dd| (grid.neighbor(p, dd), n)).collect();
            if let Some(i) = g.pool_at(p) {
                next.push((g.pools[i].to, n + 1));
            }
            if let Some(i) = g.vent_at(p) {
                next.push((g.vents[i].to, n + 1));
            }
            for (q, m) in next {
                if best.get(&q).is_none_or(|&b| m < b) {
                    best.insert(q, m);
                    if m == n {
                        queue.push_front((q, m));
                    } else {
                        queue.push_back((q, m));
                    }
                }
            }
        }
        best
    }

    #[test]
    fn two_surfaces_side_by_side_the_drowned_floor_a_twin_of_the_waterline() {
        let g = Grotto::generate(1);
        assert_eq!((g.base.width, g.base.height), (WIDTH, HEIGHT));
        assert_eq!(g.caverns.len(), 7);
        let level = parse_level(&g.base.level(&level_for(1)));
        assert_eq!(level.hero_spawn, g.base.arrival);
        assert_eq!(layer_of(g.base.arrival), Layer::Waterline);
        assert_eq!(layer_of(g.base.exit.tile), Layer::Drowned, "the court's portal is on the Drowned Floor");
        // The portal shows through the black pool, under the bell, as the refraction places it.
        let seen = apparent(g.base.exit.front().v());
        assert!(g.bell.dist(seen) <= 2.6, "the portal glows under the bell: {seen:?}");
        // The twins: most of each cavern's floor is floor on both surfaces.
        for (k, cav) in g.caverns.iter().enumerate().filter(|(k, _)| *k != BELL) {
            let (mut both, mut all) = (0, 0);
            for p in cav.cell.tiles() {
                if level.grid.get(p.x, p.y) != Tile::Rock {
                    all += 1;
                    if level.grid.get(p.x + OX, p.y) != Tile::Rock {
                        both += 1;
                    }
                }
            }
            assert!(both * 10 >= all * 7, "{}: {both} of {all}", g.caverns[k].name);
        }
    }

    #[test]
    fn four_crossings_bring_him_to_the_bell_and_only_its_seam_to_the_portal() {
        let (g, grid) = flooded();
        let best = crossings(&g, &grid, g.base.arrival);
        let ledge = g.bell.mouth;
        assert_eq!(best.get(&ledge), Some(&4), "the true way zigzags: four crossings to the bell");
        assert!(!best.contains_key(&g.base.exit.front()), "the portal is reached through the bell, nowhere else");
        // On foot alone the Waterline never gets there (each surface is broken where the other is whole).
        let walk = crate::logic::pathing::reachable(&grid, g.base.arrival, false, false);
        assert!(!walk.contains(&ledge));
        // Every crossing is reachable, and goes one way: down from a pool, up from a vent.
        for q in &g.pools {
            assert!(best.contains_key(&q.at), "pool at {:?}", q.at);
            assert_eq!((layer_of(q.at), layer_of(q.to)), (Layer::Waterline, Layer::Drowned));
            assert_eq!(q.to, below(q.at));
            assert!(grid.is_walkable(q.to.x, q.to.y, false));
        }
        for v in &g.vents {
            assert_eq!((layer_of(v.at), layer_of(v.to)), (Layer::Drowned, Layer::Waterline));
            assert!(grid.is_walkable(v.at.x, v.at.y, false) && grid.is_walkable(v.to.x, v.to.y, false));
        }
        // The lures cost a crossing and the way back.
        let stages: Vec<u8> = g.pools.iter().filter_map(|q| q.stage).chain(g.vents.iter().filter_map(|v| v.stage)).collect();
        assert_eq!(stages.len(), 4);
        assert!(g.pools.iter().any(|q| q.stage.is_none()));
    }

    #[test]
    fn beads_lie_about_one_to_a_cavern_and_the_packs_keep_to_their_dens_on_both_surfaces() {
        let (g, grid) = flooded();
        let best = crossings(&g, &grid, g.base.arrival);
        assert!(g.beads.len() >= 5);
        for b in &g.beads {
            assert!(grid.is_walkable(b.at.x, b.at.y, false), "bead at {:?}", b.at);
            assert!(best.contains_key(&b.at), "bead at {:?} out of reach", b.at);
        }
        assert!(g.beads.iter().any(|b| layer_of(b.at) == Layer::Waterline) && g.beads.iter().any(|b| layer_of(b.at) == Layer::Drowned));
        assert!(g.phantoms.iter().filter(|p| p.kind != PhantomKind::Gold).all(|p| layer_of(p.at) == Layer::Drowned));
        assert!(g.phantoms.iter().filter(|p| p.kind == PhantomKind::Gold).all(|p| g.rafts.contains(&p.at)));
        for layer in [Layer::Waterline, Layer::Drowned] {
            assert!(g.base.packs.iter().any(|pk| layer_of(pk.foes[0].2) == layer), "packs on the {layer:?}");
        }
        for pk in &g.base.packs {
            for &(kind, _, post) in &pk.foes {
                assert!(pk.den.contains(post) && grid.is_walkable(post.x, post.y, false), "{kind:?} at {post:?}");
                assert!(crate::logic::realms::natives(Sin::Envy).iter().any(|n| n.0 == kind));
            }
        }
        // Every cavern pair has a coil.
        for k in 0..BELL {
            assert!(
                g.coils
                    .iter()
                    .any(|c| c.spine.iter().any(|p| g.cavern_at(*p) == Some(k)) || c.track().iter().any(|p| g.cavern_at(*p) == Some(k))),
                "{} has no coil",
                g.caverns[k].name
            );
        }
    }

    #[test]
    fn a_coil_moves_to_and_fro_and_the_eye_opens_on_its_clock() {
        let g = Grotto::generate(1);
        let k = &g.coils[1];
        let span = (k.spine.len() - k.len) as f64;
        let (mut lo, mut hi) = (f64::MAX, f64::MIN);
        for i in 0..400 {
            let s = k.start(i as f64 * 0.1);
            lo = lo.min(s);
            hi = hi.max(s);
            assert_eq!(k.covered(i as f64 * 0.1).len(), k.len * k.cross.len());
        }
        assert!(lo < 0.5 && hi > span - 0.5, "it runs the length of its track");
        assert!(gaze(EYE_FIRST - 0.1).is_none());
        assert_eq!(gaze(EYE_FIRST + 3.0).map(|g| g.0), Some(0));
        assert!(gaze(EYE_FIRST + EYE_OPEN + 1.0).is_none());
        assert_eq!(gaze(EYE_FIRST + EYE_EVERY + 1.0).map(|g| g.0), Some(1));
        assert!((until_gaze(EYE_FIRST + EYE_OPEN) - (EYE_EVERY - EYE_OPEN)).abs() < 1e-9);
        let eye = V::new(10.0, 10.0);
        assert!(in_cone(eye, PI / 2.0, V::new(10.0, 18.0)));
        assert!(!in_cone(eye, PI / 2.0, V::new(18.0, 10.0)));
        assert!(!in_cone(eye, PI / 2.0, V::new(10.0, 10.0 + EYE_RANGE + 1.0)));
    }
}
