//! The Grey Mire: Sloth's realm (REALMS.md §4). A fog-drowned peat bog that is a sleeping giant:
//! his head, shoulder, back, hip, knee and foot lie under the moss as low hills, curled round his
//! own open hand. On the palm, ringed by a moat of deep bog, stands the Stopped Hourglass, the court
//! door at its foot and Hel's bed Kör on the threshold-stone before it. Five fingers reach from the
//! palm across the moat to the shore: the hand curls toward a hero who stands still (each joint
//! rising out of the black water) and withdraws from one who hurries (`Finger`).
//!
//! The rest of the bog: black peat pools and the rotten causeways that loop across them, deep mud,
//! duckweed hiding bog holes (lanterns of the lygtemænd hover over every one), moss beds as soft as
//! pillows, half-sunk cottages, a drowned chapel, willows and a dead oak; and Belphegor's three
//! shortcuts toward the hourglass (a rail cart, a rope ferry, a clockwork lift), each stopping
//! halfway on a moss island (`Ride`).
//!
//! The land turns: when the giant is woken a little (a loud fight by his head) he rolls over in his
//! sleep, and everything but the palm swings a quarter turn round the hourglass. So the layout is
//! kept as the giant lies (`cells`, "canonical"), and the world is that turned `turn` times
//! (`to_world`); the palm and the moat about it are the same either way. The rules that play it are
//! in `game::realms::mire`.

use std::collections::{BinaryHeap, HashSet, VecDeque};

use super::{gate, pick_native, Base, Pack};
use crate::logic::enemy::EnemyKind;
use crate::logic::hub::{Gate, Rect};
use crate::logic::rng::Rng;
use crate::logic::sins::Sin;
use crate::logic::types::{Dir, CARDINALS, P, V};

/// The layout's seed: the realm is the same every time (its packs vary with the tier).
pub const SEED: u32 = 7404;
/// The bog is a square this many tiles across, the hourglass in its middle.
pub const SIZE: i32 = 65;
pub const CENTRE: i32 = 32;
/// The palm (the threshold island), the moat's outer shore, and the land's edge, as radii.
pub const PALM_R: f64 = 5.5;
pub const MOAT_R: f64 = 12.5;
pub const LAND_R: f64 = 28.5;
/// The giant's breath: the tide comes and goes this often (seconds).
pub const BREATH: f64 = 12.0;
/// After this long moving, the hand withdraws (tiles a second); standing still, it curls toward him.
pub const HASTE_GRACE: f64 = 3.0;
pub const WITHDRAW: f64 = 1.0;
pub const CURL: f64 = 1.5;
/// Torpor: full at this; how it fills standing still, wading, lying in moss; how it drains moving
/// on firm ground and with each kill.
pub const TORPOR_MAX: f64 = 100.0;
pub const STILL_RATE: f64 = 10.0;
pub const WADE_RATE: f64 = 5.0;
pub const MOSS_RATE: f64 = 25.0;
pub const FIRM_DRAIN: f64 = 8.0;
pub const KILL_DRAIN: f64 = 20.0;
/// He is lying in moss once he has stood still on it this long.
pub const LIE_DOWN: f64 = 0.6;
/// A moss bed heals this much a second; every so many seconds rested, the mire grows older.
pub const MOSS_HEAL: f64 = 5.0;
pub const REST_AGE: f64 = 10.0;
/// The Long Sleep: the sand's seconds, a yawn every so often, how far it pushes him (braced, less),
/// and the Torpor it brings.
pub const SAND: f64 = 60.0;
pub const YAWN_EVERY: f64 = 5.0;
pub const YAWN_PUSH: f64 = 3.0;
pub const BRACED_PUSH: f64 = 1.0;
pub const YAWN_TORPOR: f64 = 15.0;
/// In the Long Sleep, standing still longer than this fills Torpor twice as fast.
pub const LULL: f64 = 2.0;
/// A shortcut must be wound this long, standing still, to go on; it rides this fast.
pub const WIND: f64 = 20.0;
pub const RIDE_SPEED: f64 = 2.5;
/// The land turns over this long (the fog surging white), at most this often, when a fight this
/// near the head grows loud.
pub const TURN_SECONDS: f64 = 3.0;
pub const TURN_REST: f64 = 20.0;
pub const HEAD_RANGE: f64 = 11.0;
/// The giant snores this often while he sleeps.
pub const SNORE_EVERY: f64 = 4.2;

/// The parts of the giant that rise above the bog as hills.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Part {
    Head,
    /// His other hand, curled under his cheek.
    Cheek,
    Shoulder,
    Back,
    Hip,
    Knee,
    Foot,
}

/// What a tile of the bog is.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Cell {
    /// Beyond the world: deep bog and reed beds.
    Deep,
    /// Low peat ground: the tide floods it.
    Peat,
    /// The giant's body, a level up (its edges slope: `Mire::ramp`).
    Hill(Part),
    /// A moss bed: it heals, and it brings sleep.
    Moss,
    /// Deep mud: half his pace.
    Mud,
    /// Duckweed over a bog hole: it looks like firm ground.
    Hole,
    /// Black peat water.
    Pool,
    /// The deep bog round the palm.
    Moat,
    /// A causeway plank over the water (it may rot: `Mire::rotten`).
    Plank,
    /// A joint of finger `k`, the `i`th from the palm.
    Finger(u8, u8),
    /// The way of shortcut `k`, its `i`th step (water, but while it runs).
    Track(u8, u16),
    /// Where a shortcut waits for him.
    Station(u8),
    /// The giant's palm, the island round the hourglass.
    Palm,
    /// The threshold-stone over the bog hole, and on it the bed Kör.
    Threshold,
    /// The Stopped Hourglass's tower, and the court door in its foot.
    Tower,
    Door,
    /// A half-sunk cottage (its index), the drowned chapel, a willow, the dead oak.
    Cottage(u8),
    Chapel,
    Willow,
    Oak,
}

impl Cell {
    /// He can stand here (a finger's joint and a shortcut's way only as the game raises them).
    pub fn firm(self) -> bool {
        matches!(
            self,
            Cell::Peat
                | Cell::Hill(_)
                | Cell::Moss
                | Cell::Mud
                | Cell::Hole
                | Cell::Plank
                | Cell::Station(_)
                | Cell::Palm
                | Cell::Threshold
        )
    }

    /// Solid: rock to the level (a building, a tree, the tower, the world's edge).
    pub fn solid(self) -> bool {
        matches!(self, Cell::Deep | Cell::Tower | Cell::Door | Cell::Cottage(_) | Cell::Chapel | Cell::Willow | Cell::Oak)
    }
}

/// One of the giant's fingers across the moat: its joints from the palm out, and how far it
/// reaches (joints risen; the `i`th is firm once `reach >= i + 1`).
#[derive(Clone, Debug)]
pub struct Finger {
    pub joints: Vec<P>,
    pub reach: f64,
}

impl Finger {
    pub fn len(&self) -> usize {
        self.joints.len()
    }

    pub fn is_empty(&self) -> bool {
        self.joints.is_empty()
    }

    /// Joint `i` is up out of the water, firm to stand on.
    pub fn up(&self, i: usize) -> bool {
        self.reach >= i as f64 + 1.0
    }

    /// How far joint `i` has risen, 0 to 1.
    pub fn rise(&self, i: usize) -> f64 {
        (self.reach - i as f64).clamp(0.0, 1.0)
    }
}

/// Belphegor's shortcuts.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RideKind {
    Cart,
    Ferry,
    Lift,
}

impl RideKind {
    pub fn name(self) -> &'static str {
        match self {
            RideKind::Cart => "THE RAIL CART",
            RideKind::Ferry => "THE ROPE FERRY",
            RideKind::Lift => "THE CLOCKWORK LIFT",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RideState {
    /// Waiting at its station.
    Waiting,
    /// Running toward step `to` of its way.
    Running { to: usize },
    /// Stopped halfway, on its moss island, to be wound.
    Stranded,
    /// At the end of its way, by the moat.
    Spent,
}

/// A shortcut: its way from its station (`path[0]`) by its moss island (`path[stop]`) to the
/// moat's shore (the last step), where it is along it, and how far it has been wound.
#[derive(Clone, Debug)]
pub struct Ride {
    pub kind: RideKind,
    pub path: Vec<P>,
    pub stop: usize,
    pub pos: f64,
    pub state: RideState,
    pub wound: f64,
}

impl Ride {
    /// Its way's step `i` is firm now (the station, the island and the shore always; the water
    /// between while it runs over it).
    pub fn open(&self, i: usize) -> bool {
        match self.state {
            RideState::Running { to } => {
                let from = if to == self.stop { 0 } else { self.stop };
                (from..=to).contains(&i)
            }
            _ => false,
        }
    }

    /// Where it is, as a point along its way (canonical).
    pub fn at(&self) -> V {
        let n = self.path.len();
        let i = (self.pos.floor() as usize).min(n - 1);
        let j = (i + 1).min(n - 1);
        let t = self.pos - i as f64;
        let (a, b) = (self.path[i].v(), self.path[j].v());
        V::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
    }
}

/// The Long Sleep, as far as it has gone.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Phase {
    /// The bed has not been reached.
    Wander,
    /// Kör's offer is before him.
    Asking,
    /// The hourglass has turned: `left` seconds of sand, the next yawn in `yawn`.
    Sand { left: f64, yawn: f64 },
    /// The sand has run out and the door stands open (he yawns on: the next in `yawn`).
    Done { yawn: f64 },
}

#[derive(Clone, Debug)]
pub struct Mire {
    pub base: Base,
    /// The bog as the giant lies, row by row (`SIZE` squared), and its floors (0 or 1) and slopes.
    pub cells: Vec<Cell>,
    pub level: Vec<u8>,
    pub ramp: Vec<bool>,
    /// Planks rotted away (by index into `cells`).
    pub rotten: Vec<bool>,
    pub fingers: Vec<Finger>,
    pub rides: Vec<Ride>,
    /// Where the giant's head lies (its middle), and where the bed stands.
    pub head: P,
    pub bed: P,
    /// Where things stand that dress the bog (canonical): each cottage's tiles and how it leans,
    /// the chapel's tiles, the gears sunk in the mud.
    pub cottages: Vec<(Rect, f32)>,
    pub chapel: Rect,
    pub gears: Vec<P>,
    /// The arrival, the portal behind him, his checkpoint, and the packs, as the giant lies.
    pub canon_arrival: P,
    pub canon_entry: Gate,
    pub canon_checkpoint: P,
    pub canon_packs: Vec<Pack>,
    /// Quarter turns the land has made (clockwise as seen from above).
    pub turn: u8,
    /// Times he has slept the Long Sleep (three hundred years each), and notches the tide has risen.
    pub age: u32,
    pub notch: u32,
    // ---- the state of its rules (`game::realms::mire`)
    pub torpor: f64,
    /// Seconds he has stood still, and moved, without a break.
    pub still: f64,
    pub moving: f64,
    /// The giant's breath, seconds into it.
    pub breath: f64,
    /// Seconds rested in moss, toward the next ageing.
    pub rested: f64,
    pub heal_carry: f64,
    /// The kills already counted against his Torpor.
    pub kills_seen: u32,
    /// The last firm place he stood (world), where the bog gives him back, and the last he stood
    /// on beyond the moat (canonical: his checkpoint once he reaches the palm).
    pub last_safe: V,
    pub last_land: P,
    pub snore: f64,
    /// The land turning: seconds in, and whether it has swung yet; the rest before it may again.
    pub turning: Option<(f64, bool)>,
    pub turn_rest: f64,
    pub phase: Phase,
    /// A yawn's push still to come (tiles), and its way.
    pub push: f64,
    pub push_dir: V,
    /// The wave of the last yawn, seconds since (for the view), and its centre.
    pub wave: f64,
    /// Servants that climbed out of the bog (their duelists' ids), and the next climbing.
    pub raised: Vec<u32>,
    pub rise_timer: f64,
    /// His patron's wait and his blessing's charges last tick: a fall in either is a blessing used.
    pub prev_wait: f64,
    pub prev_charges: u8,
    /// Falling asleep: seconds in.
    pub dozing: Option<f64>,
    /// A warning given while Torpor is high.
    pub warned: bool,
    /// The hourglass turning over (seconds since it began; for the view).
    pub turned_at: Option<f64>,
    pub clock: f64,
}

/// The tile's index in the square.
pub fn idx(p: P) -> usize {
    (p.y * SIZE + p.x) as usize
}

pub fn inside(p: P) -> bool {
    p.x >= 0 && p.y >= 0 && p.x < SIZE && p.y < SIZE
}

/// Distance from the hourglass.
pub fn radius(p: P) -> f64 {
    (((p.x - CENTRE) * (p.x - CENTRE) + (p.y - CENTRE) * (p.y - CENTRE)) as f64).sqrt()
}

/// On the palm (which never turns).
pub fn in_palm(p: P) -> bool {
    radius(p) <= PALM_R
}

/// A quarter turn clockwise round the hourglass (as seen from above: north to east).
pub fn turn_p(p: P) -> P {
    P::new(CENTRE - (p.y - CENTRE), CENTRE + (p.x - CENTRE))
}

pub fn unturn_p(p: P) -> P {
    P::new(CENTRE + (p.y - CENTRE), CENTRE - (p.x - CENTRE))
}

pub fn turn_v(v: V) -> V {
    let c = CENTRE as f64;
    V::new(c - (v.y - c), c + (v.x - c))
}

pub fn turn_dir(d: Dir) -> Dir {
    match d {
        Dir::Up => Dir::Right,
        Dir::Right => Dir::Down,
        Dir::Down => Dir::Left,
        Dir::Left => Dir::Up,
        Dir::None => Dir::None,
    }
}

pub fn turn_rect(r: Rect) -> Rect {
    let (a, b) = (turn_p(P::new(r.x0, r.y0)), turn_p(P::new(r.x1, r.y1)));
    Rect { x0: a.x.min(b.x), y0: a.y.min(b.y), x1: a.x.max(b.x), y1: a.y.max(b.y) }
}

/// A point at angle `deg` (0 east, 90 south) and `r` tiles from the hourglass.
fn polar(deg: f64, r: f64) -> V {
    let a = deg.to_radians();
    V::new(CENTRE as f64 + a.cos() * r, CENTRE as f64 + a.sin() * r)
}

fn tile(v: V) -> P {
    P::new(v.x.round() as i32, v.y.round() as i32)
}

/// Smooth value noise in about 0..1.
fn noise(x: f64, y: f64, seed: u32) -> f64 {
    let h = |i: i32, j: i32| {
        let mut n = (i as u32).wrapping_mul(0x8da6_b343) ^ (j as u32).wrapping_mul(0xd816_3841) ^ seed.wrapping_mul(0xcb1a_b31f);
        n ^= n >> 13;
        n = n.wrapping_mul(0x85eb_ca6b);
        n ^= n >> 16;
        (n & 0xffff) as f64 / 65535.0
    };
    let (i, j) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - i as f64, y - j as f64);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let top = h(i, j) + (h(i + 1, j) - h(i, j)) * sx;
    let bot = h(i, j + 1) + (h(i + 1, j + 1) - h(i, j + 1)) * sx;
    top + (bot - top) * sy
}

/// The tiles of a line from `a` to `b`, each beside the last (four ways).
fn walk(a: V, b: V) -> Vec<P> {
    let n = ((b.x - a.x).abs().max((b.y - a.y).abs()) * 3.0).ceil().max(1.0) as usize;
    let mut out: Vec<P> = Vec::new();
    for k in 0..=n {
        let t = k as f64 / n as f64;
        let p = tile(V::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t));
        if let Some(&last) = out.last() {
            if last == p {
                continue;
            }
            if last.x != p.x && last.y != p.y {
                // Round the corner by way of whichever side lies nearer the line.
                out.push(P::new(p.x, last.y));
            }
        }
        out.push(p);
    }
    out
}

/// A gentle curve from `a` to `b`, bowed `bow` tiles to one side.
fn bowed(a: V, b: V, bow: f64) -> Vec<P> {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    let (nx, ny) = (-dy / len, dx / len);
    let steps = 6;
    let mut out: Vec<P> = Vec::new();
    for k in 0..steps {
        let at = |k: usize| {
            let t = k as f64 / steps as f64;
            let off = bow * 4.0 * t * (1.0 - t);
            V::new(a.x + dx * t + nx * off, a.y + dy * t + ny * off)
        };
        for p in walk(at(k), at(k + 1)) {
            if out.last() != Some(&p) {
                out.push(p);
            }
        }
    }
    out
}

impl Mire {
    /// The realm laid out for `tier`.
    pub fn generate(tier: usize) -> Mire {
        Mire::generate_aged(tier, 0)
    }

    /// The realm laid out for `tier`, `age` Long Sleeps on: its causeways rotting, its servants
    /// more, its tide higher.
    pub fn generate_aged(tier: usize, age: u32) -> Mire {
        let mut rng = Rng::new(SEED);
        let n = (SIZE * SIZE) as usize;
        let mut cells = vec![Cell::Deep; n];
        let mut level = vec![0u8; n];
        let all = || (0..SIZE).flat_map(|y| (0..SIZE).map(move |x| P::new(x, y)));

        // ---- the ground: the palm, the moat, the land to its ragged edge
        for p in all() {
            let r = radius(p);
            let a = ((p.y - CENTRE) as f64).atan2((p.x - CENTRE) as f64);
            let edge = LAND_R + (noise(a * 3.0 + 10.0, 0.5, SEED) - 0.5) * 2.4;
            // The palm stands a level over the bog, above the tide; its rim slopes down to the fingers.
            level[idx(p)] = u8::from(r <= PALM_R);
            cells[idx(p)] = if r <= PALM_R {
                Cell::Palm
            } else if r <= MOAT_R {
                Cell::Moat
            } else if r <= edge && p.x > 1 && p.y > 1 && p.x < SIZE - 2 && p.y < SIZE - 2 {
                Cell::Peat
            } else {
                Cell::Deep
            };
        }
        // He arrives from the west, the portal in the reeds behind him.
        let canon_arrival = P::new(CENTRE - 27, CENTRE);
        let entry_tile = P::new(CENTRE - 29, CENTRE);
        for y in CENTRE - 3..=CENTRE + 3 {
            for x in canon_arrival.x - 1..=canon_arrival.x + 3 {
                cells[idx(P::new(x, y))] = Cell::Peat;
            }
            cells[idx(P::new(entry_tile.x, y))] = Cell::Deep;
            cells[idx(P::new(entry_tile.x - 1, y))] = Cell::Deep;
        }
        cells[idx(P::new(entry_tile.x + 1, CENTRE))] = Cell::Peat;

        // ---- the giant: hills of moss over his body, curled round his hand
        let shore = |p: P| radius(p) < MOAT_R + 2.6;
        let parts: [(Part, V, f64, f64, f64); 8] = [
            (Part::Head, polar(-135.0, 19.0), 4.4, 4.0, 0.8),
            (Part::Cheek, polar(-106.0, 17.5), 3.2, 2.2, 0.3),
            (Part::Shoulder, polar(-63.0, 19.5), 5.2, 3.3, 0.5),
            (Part::Back, polar(-18.0, 20.5), 5.5, 2.6, 1.3),
            (Part::Back, polar(10.0, 20.5), 5.0, 2.6, 1.8),
            (Part::Hip, polar(56.0, 20.0), 4.8, 4.4, 0.0),
            (Part::Knee, polar(112.0, 20.0), 4.6, 3.2, 2.0),
            (Part::Foot, polar(148.0, 21.5), 2.8, 2.4, 0.4),
        ];
        for &(part, c, rx, ry, rot) in &parts {
            let (cs, sn) = (rot.cos(), rot.sin());
            for p in all() {
                let (dx, dy) = (p.x as f64 - c.x, p.y as f64 - c.y);
                let (u, w) = (dx * cs + dy * sn, -dx * sn + dy * cs);
                if (u / rx).powi(2) + (w / ry).powi(2) <= 1.0 && cells[idx(p)] == Cell::Peat && !shore(p) {
                    cells[idx(p)] = Cell::Hill(part);
                    level[idx(p)] = 1;
                }
            }
        }
        let head = tile(parts[0].1);

        // ---- the shortcuts: a canal of black water from a station out on the land to the shore,
        // a moss island in it halfway, the way laid over the water
        let mut rides: Vec<Ride> = Vec::new();
        let mut isle = vec![false; n];
        for (k, (kind, deg, bow)) in
            [(RideKind::Cart, 31.0, 1.2), (RideKind::Ferry, 165.0, -1.0), (RideKind::Lift, -86.0, 0.9)].into_iter().enumerate()
        {
            let (from, mid, to) = (polar(deg, 26.0), polar(deg + 1.5, 19.5), polar(deg, 13.6));
            let (a, b) = (bowed(from, mid, bow), bowed(mid, to, -bow));
            let mut path: Vec<P> = a.clone();
            let stop = path.len() - 1;
            path.extend(b.iter().skip(1));
            let island = path[stop];
            // The canal: water within two and a half tiles of the way (but out on the land's rim and
            // at the shore), the island moss in it, a tile of water round the island.
            for p in all() {
                let r = radius(p);
                if !(14.6..=25.6).contains(&r) || cells[idx(p)] == Cell::Deep {
                    continue;
                }
                let near = path.iter().any(|q| (((q.x - p.x).pow(2) + (q.y - p.y).pow(2)) as f64) <= 6.3);
                if near {
                    cells[idx(p)] = Cell::Pool;
                    level[idx(p)] = 0;
                }
            }
            for p in all() {
                let d = (((p.x - island.x).pow(2) + (p.y - island.y).pow(2)) as f64).sqrt();
                if d <= 1.9 {
                    cells[idx(p)] = Cell::Moss;
                    level[idx(p)] = 0;
                    isle[idx(p)] = true;
                } else if d <= 3.0 && cells[idx(p)] != Cell::Deep {
                    cells[idx(p)] = Cell::Pool;
                    level[idx(p)] = 0;
                    isle[idx(p)] = true;
                }
            }
            // A tile of water between the island and the land on either side: a leap off.
            let (dx, dy) = ((path[stop + 1].x - path[stop - 1].x) as f64, (path[stop + 1].y - path[stop - 1].y) as f64);
            let l = (dx * dx + dy * dy).sqrt().max(1e-6);
            for s in [-1.0, 1.0] {
                let (nx, ny) = (-dy / l * s, dx / l * s);
                for d in 3..6 {
                    let q = tile(V::new(island.x as f64 + nx * d as f64, island.y as f64 + ny * d as f64));
                    if inside(q) && cells[idx(q)] != Cell::Deep && !shore(q) {
                        cells[idx(q)] = Cell::Peat;
                        level[idx(q)] = 0;
                    }
                }
            }
            for (i, &p) in path.iter().enumerate() {
                cells[idx(p)] = if i == 0 || i == stop {
                    Cell::Station(k as u8)
                } else if i + 1 == path.len() {
                    Cell::Peat
                } else {
                    Cell::Track(k as u8, i as u16)
                };
                level[idx(p)] = 0;
            }
            // The station stands on firm ground.
            for d in CARDINALS {
                let q = path[0].step(d);
                if cells[idx(q)] == Cell::Deep || cells[idx(q)] == Cell::Pool && radius(q) > radius(path[0]) {
                    cells[idx(q)] = Cell::Peat;
                }
            }
            rides.push(Ride { kind, path, stop, pos: 0.0, state: RideState::Waiting, wound: 0.0 });
        }

        // ---- the pools: black water wherever the peat dips, but on the giant and the shore
        for p in all() {
            if cells[idx(p)] != Cell::Peat || shore(p) || radius(p) > LAND_R - 1.0 {
                continue;
            }
            let near_hill = false;
            let v = noise(p.x as f64 / 3.0, p.y as f64 / 3.0, SEED + 1) * 0.65 + noise(p.x as f64 / 1.6, p.y as f64 / 1.6, SEED + 2) * 0.35;
            let by_arrival = (p.x - canon_arrival.x).abs() <= 4 && (p.y - canon_arrival.y).abs() <= 3;
            // Between the shore and the land's rim lies the bog proper: water, but for ridges of peat
            // and the giant (with a margin of peat round him); elsewhere the odd pool.
            let r = radius(p);
            let pool = if (15.6..25.4).contains(&r) { v < 0.58 } else { v > 0.68 };
            if pool && !near_hill && !by_arrival {
                cells[idx(p)] = Cell::Pool;
            }
        }

        // ---- the buildings and the trees: the drowned chapel half in a pool, half-sunk cottages,
        // willows by the water, the dead oak near the shore
        let chapel = {
            let c = tile(polar(-160.0, 24.0));
            Rect { x0: c.x - 1, y0: c.y - 2, x1: c.x + 2, y1: c.y + 2 }
        };
        for p in chapel.tiles() {
            cells[idx(p)] = Cell::Chapel;
            level[idx(p)] = 0;
        }
        for x in chapel.x0 - 1..=chapel.x1 + 1 {
            for y in [chapel.y0 - 1, chapel.y0 - 2] {
                let q = P::new(x, y);
                if cells[idx(q)] == Cell::Peat {
                    cells[idx(q)] = Cell::Pool;
                }
            }
        }
        let mut cottages = Vec::new();
        for (k, (deg, r, w, h)) in
            [(-20.0, 25.0, 2, 3), (80.0, 23.5, 3, 2), (125.0, 25.0, 2, 2), (-50.0, 25.5, 2, 2)].into_iter().enumerate()
        {
            let c = tile(polar(deg, r));
            let rect = Rect { x0: c.x, y0: c.y, x1: c.x + w - 1, y1: c.y + h - 1 };
            for p in rect.tiles() {
                cells[idx(p)] = Cell::Cottage(k as u8);
                level[idx(p)] = 0;
            }
            cottages.push((rect, (rng.next() as f32 - 0.5) * 0.3));
        }
        let oak = tile(polar(136.0, 15.6));
        cells[idx(oak)] = Cell::Oak;
        let mut willows = 0;
        let mut tries = 0;
        while willows < 14 && tries < 4000 {
            tries += 1;
            let p = P::new(3 + rng.index((SIZE - 6) as usize) as i32, 3 + rng.index((SIZE - 6) as usize) as i32);
            let r = radius(p);
            if cells[idx(p)] != Cell::Peat || !(15.5..=26.5).contains(&r) {
                continue;
            }
            let by_water = CARDINALS.iter().any(|&d| cells[idx(p.step(d))] == Cell::Pool);
            let crowded = (-2..=2).any(|dy| (-2..=2).any(|dx| cells[idx(P::new(p.x + dx, p.y + dy))].solid() && (dx, dy) != (0, 0)));
            let open_round = CARDINALS.iter().filter(|&&d| cells[idx(p.step(d))].firm()).count() >= 2;
            if by_water && !crowded && open_round && (p.x - canon_arrival.x).abs() + (p.y - canon_arrival.y).abs() > 5 {
                cells[idx(p)] = Cell::Willow;
                willows += 1;
            }
        }

        // ---- every scrap of land joined to the arrival: causeways of planks across the water
        // where it is not (and the scraps too small to matter sink)
        let walkable = |cells: &[Cell], p: P| inside(p) && cells[idx(p)].firm();
        for _ in 0..400 {
            let main = flood(&cells, canon_arrival, &isle);
            let stray = all().find(|&p| walkable(&cells, p) && !isle[idx(p)] && !in_palm(p) && !main.contains(&p));
            let Some(s) = stray else { break };
            let part = flood(&cells, s, &isle);
            let station = part.iter().any(|&p| matches!(cells[idx(p)], Cell::Station(_)));
            let way = if part.len() >= 3 || station { causeway(&cells, &part, &main, &isle) } else { None };
            match way {
                Some(way) if !way.is_empty() => {
                    for p in way {
                        cells[idx(p)] = Cell::Plank;
                        level[idx(p)] = 0;
                    }
                }
                _ if station => {
                    // A station must be reached: a straight way of planks to the nearest firm ground joined to him.
                    let from = part.iter().copied().find(|&p| matches!(cells[idx(p)], Cell::Station(_))).unwrap_or(s);
                    let to = main.iter().copied().min_by_key(|q| (q.x - from.x).pow(2) + (q.y - from.y).pow(2)).unwrap_or(canon_arrival);
                    for p in walk(from.v(), to.v()) {
                        if !cells[idx(p)].firm() && !matches!(cells[idx(p)], Cell::Track(..)) {
                            cells[idx(p)] = Cell::Plank;
                            level[idx(p)] = 0;
                        }
                    }
                }
                _ => {
                    for p in part {
                        cells[idx(p)] = Cell::Pool;
                        level[idx(p)] = 0;
                    }
                }
            }
        }
        // Loops: causeways across pools where the way round is long.
        let mut loops = 0;
        let mut tries = 0;
        while loops < 16 && tries < 2000 {
            tries += 1;
            let a = P::new(3 + rng.index((SIZE - 6) as usize) as i32, 3 + rng.index((SIZE - 6) as usize) as i32);
            let ang = rng.next() * std::f64::consts::TAU;
            let dist = 5.0 + rng.next() * 4.0;
            let b = tile(V::new(a.x as f64 + ang.cos() * dist, a.y as f64 + ang.sin() * dist));
            if !inside(b) || !walkable(&cells, a) || !walkable(&cells, b) || isle[idx(a)] || isle[idx(b)] {
                continue;
            }
            if radius(a) < MOAT_R + 1.0 || radius(b) < MOAT_R + 1.0 {
                continue;
            }
            let round = steps(&cells, a, b, &isle);
            if round.is_some_and(|s| s as f64 <= dist * 1.7) {
                continue;
            }
            if let Some(way) = causeway(&cells, &HashSet::from([a]), &HashSet::from([b]), &isle) {
                if way.len() >= 2 && way.len() <= 10 {
                    for p in way {
                        cells[idx(p)] = Cell::Plank;
                    }
                    loops += 1;
                }
            }
        }

        // ---- deep mud, moss beds, and the bog holes under their duckweed
        let main = flood(&cells, canon_arrival, &isle);
        let mut gears = Vec::new();
        for _ in 0..18 {
            let c = P::new(3 + rng.index((SIZE - 6) as usize) as i32, 3 + rng.index((SIZE - 6) as usize) as i32);
            if !main.contains(&c)
                || cells[idx(c)] != Cell::Peat
                || radius(c) < MOAT_R + 2.5
                || (c.x - canon_arrival.x).abs() + (c.y - canon_arrival.y).abs() < 6
            {
                continue;
            }
            let r = 1.2 + rng.next() * 1.3;
            for p in all() {
                let clear = (p.x - canon_arrival.x).abs() + (p.y - canon_arrival.y).abs() > 5;
                if cells[idx(p)] == Cell::Peat && clear && (((p.x - c.x).pow(2) + (p.y - c.y).pow(2)) as f64) <= r * r {
                    cells[idx(p)] = Cell::Mud;
                }
            }
            gears.push(c);
        }
        for k in 0..7 {
            let c = if k == 0 {
                // His pillow: a moss bed behind his head, away from his face.
                let back = V::new((head.x - CENTRE) as f64, (head.y - CENTRE) as f64);
                let l = (back.x * back.x + back.y * back.y).sqrt().max(1.0);
                tile(V::new(head.x as f64 + back.x / l * 2.4, head.y as f64 + back.y / l * 2.4))
            } else {
                P::new(3 + rng.index((SIZE - 6) as usize) as i32, 3 + rng.index((SIZE - 6) as usize) as i32)
            };
            let by_arrival = (c.x - canon_arrival.x).abs() + (c.y - canon_arrival.y).abs() < 7;
            if !main.contains(&c) || !matches!(cells[idx(c)], Cell::Peat | Cell::Hill(_)) || radius(c) < MOAT_R + 2.0 || by_arrival {
                continue;
            }
            for p in all() {
                if matches!(cells[idx(p)], Cell::Peat | Cell::Hill(_)) && (((p.x - c.x).pow(2) + (p.y - c.y).pow(2)) as f64) <= 1.6 * 1.6 {
                    cells[idx(p)] = Cell::Moss;
                }
            }
        }
        let mut holes = 0;
        let mut tries = 0;
        while holes < 21 && tries < 6000 {
            tries += 1;
            let p = P::new(3 + rng.index((SIZE - 6) as usize) as i32, 3 + rng.index((SIZE - 6) as usize) as i32);
            if !hole_fits(&cells, p, canon_arrival) {
                continue;
            }
            // Three in a row, like lanterns along a path.
            let d = CARDINALS[rng.index(4)];
            let mut chain = vec![p];
            let mut q = p;
            for _ in 0..2 {
                q = P::new(q.x + 2 * d.vec().0 + rng.index(3) as i32 - 1, q.y + 2 * d.vec().1);
                if inside(q) && hole_fits(&cells, q, canon_arrival) {
                    chain.push(q);
                }
            }
            for c in chain {
                cells[idx(c)] = Cell::Hole;
                holes += 1;
            }
        }

        // ---- the fingers across the moat, the palm: the tower, its door, the bed before it
        let mut fingers = Vec::new();
        for (deg, curl) in [(-80.0, 9.0), (-14.0, 7.0), (44.0, 8.0), (112.0, -7.0), (176.0, 9.0)] {
            let mut joints: Vec<P> = Vec::new();
            for k in 0..=40 {
                let t = k as f64 / 40.0;
                let r = PALM_R + 0.3 + t * (MOAT_R - PALM_R);
                let v = polar(deg + curl * t * t, r);
                let p = tile(v);
                if let Some(&last) = joints.last() {
                    if last == p {
                        continue;
                    }
                    if last.x != p.x && last.y != p.y {
                        let side = P::new(p.x, last.y);
                        if cells[idx(side)] == Cell::Moat {
                            joints.push(side);
                        } else {
                            joints.push(P::new(last.x, p.y));
                        }
                    }
                }
                joints.push(p);
            }
            joints.retain(|&p| cells[idx(p)] == Cell::Moat);
            joints.dedup();
            for (i, &p) in joints.iter().enumerate() {
                cells[idx(p)] = Cell::Finger(fingers.len() as u8, i as u8);
            }
            // The shore it reaches is firm.
            if let Some(&tip) = joints.last() {
                for d in CARDINALS {
                    let q = tip.step(d);
                    if radius(q) > MOAT_R && !cells[idx(q)].firm() {
                        cells[idx(q)] = Cell::Peat;
                        level[idx(q)] = 0;
                    }
                }
            }
            fingers.push(Finger { joints, reach: 0.0 });
        }
        for y in CENTRE - 3..=CENTRE - 1 {
            for x in CENTRE - 1..=CENTRE + 1 {
                cells[idx(P::new(x, y))] = Cell::Tower;
            }
        }
        let door = P::new(CENTRE, CENTRE - 1);
        cells[idx(door)] = Cell::Door;
        let bed = P::new(CENTRE, CENTRE + 2);
        cells[idx(bed)] = Cell::Threshold;

        // ---- the slopes round every hill
        let mut ramp = vec![false; n];
        for p in all() {
            if level[idx(p)] == 1 && CARDINALS.iter().any(|&d| level[idx(p.step(d))] == 0 && !cells[idx(p.step(d))].solid()) {
                ramp[idx(p)] = true;
            }
        }

        let canon_entry = gate(Sin::Sloth, entry_tile, Dir::Right);
        let mut m = Mire {
            base: Base {
                sin: Sin::Sloth,
                width: SIZE,
                height: SIZE,
                rows: Vec::new(),
                heights: Vec::new(),
                arrival: canon_arrival,
                entry: canon_entry,
                exit: gate(Sin::Sloth, door, Dir::Down),
                packs: Vec::new(),
                checkpoint: canon_arrival,
                tier,
            },
            rotten: vec![false; n],
            cells,
            level,
            ramp,
            fingers,
            rides,
            head,
            bed,
            cottages,
            chapel,
            gears,
            canon_arrival,
            canon_entry,
            canon_checkpoint: canon_arrival,
            canon_packs: Vec::new(),
            turn: 0,
            age,
            notch: age,
            torpor: 0.0,
            still: 0.0,
            moving: 0.0,
            breath: 0.0,
            rested: 0.0,
            heal_carry: 0.0,
            kills_seen: 0,
            last_safe: canon_arrival.v(),
            last_land: canon_arrival,
            snore: 1.0,
            turning: None,
            turn_rest: 0.0,
            phase: Phase::Wander,
            push: 0.0,
            push_dir: V::new(0.0, 0.0),
            wave: 99.0,
            raised: Vec::new(),
            rise_timer: 2.0,
            prev_wait: 0.0,
            prev_charges: 0,
            dozing: None,
            warned: false,
            turned_at: None,
            clock: 0.0,
        };
        m.canon_packs = m.lay_packs(tier, age);
        // As he first comes in, the hand lies open toward him: the finger nearest the arrival reaches
        // the shore (and draws back once he hurries). After a Long Sleep it is shut.
        if age == 0 {
            if let Some(k) = m.nearest_finger(canon_arrival.v()) {
                m.fingers[k].reach = m.fingers[k].len() as f64;
            }
        }
        // Three hundred years have rotted the causeways.
        if age > 0 {
            let share = (0.3 * age as f64).min(0.55);
            let planks: Vec<P> = all().filter(|&p| m.cells[idx(p)] == Cell::Plank).collect();
            let want = (planks.len() as f64 * share) as usize;
            let mut order = planks.clone();
            rng.shuffle(&mut order);
            let mut gone = 0;
            for p in order {
                if gone >= want {
                    break;
                }
                if m.may_rot(p) {
                    m.rotten[idx(p)] = true;
                    gone += 1;
                }
            }
        }
        m.sync_base();
        m
    }

    /// The packs, at clearings round the bog (one by the giant's head): the natives of the tier,
    /// Hel's servants among them, more of those the older the mire.
    fn lay_packs(&self, tier: usize, age: u32) -> Vec<Pack> {
        let mut rng = Rng::new(SEED ^ (tier as u32).wrapping_mul(0x9e37_79b1) ^ age.wrapping_mul(0x85eb_ca77));
        let isle = vec![false; self.cells.len()];
        let main = flood(&self.cells, self.canon_arrival, &isle);
        let mut packs = Vec::new();
        let size = match tier {
            0..=2 => 3,
            3..=5 => 4,
            _ => 5,
        };
        for (k, (deg, r)) in [
            (-122.0, 23.5),
            (-40.0, 23.0),
            (5.0, 25.0),
            (70.0, 16.5),
            (102.0, 24.0),
            (146.0, 17.0),
            (-172.0, 17.0),
            (-100.0, 25.5),
            (22.0, 16.0),
        ]
        .into_iter()
        .enumerate()
        {
            let c = tile(polar(deg, r));
            let den = Rect { x0: c.x - 3, y0: c.y - 3, x1: c.x + 3, y1: c.y + 3 };
            let mut spots: Vec<P> = den
                .tiles()
                .filter(|&p| {
                    inside(p)
                        && main.contains(&p)
                        && matches!(self.cells[idx(p)], Cell::Peat | Cell::Hill(_) | Cell::Mud)
                        && radius(p) > MOAT_R + 1.5
                })
                .collect();
            if spots.len() < 6 {
                continue;
            }
            rng.shuffle(&mut spots);
            let servants = usize::from(k % 2 == 0) + (age as usize).min(3);
            let mut foes = Vec::new();
            let mut posts: Vec<P> = Vec::new();
            for p in spots {
                if foes.len() >= size + (age as usize).min(3) {
                    break;
                }
                if posts.iter().any(|q| (q.x - p.x).abs() + (q.y - p.y).abs() < 2) {
                    continue;
                }
                let (kind, stage) = if foes.len() < servants { (EnemyKind::Ganglot, 0) } else { pick_native(&mut rng, Sin::Sloth, tier) };
                posts.push(p);
                foes.push((kind, stage, p));
            }
            packs.push(Pack { den, foes });
        }
        packs
    }

    /// The cell at world tile `w`.
    pub fn cell(&self, w: P) -> Cell {
        if !inside(w) {
            return Cell::Deep;
        }
        self.cells[idx(self.to_canon(w))]
    }

    /// The floor (0 or 1) and slope at world tile `w`.
    pub fn floor_at(&self, w: P) -> (u8, bool) {
        if !inside(w) {
            return (0, false);
        }
        let c = idx(self.to_canon(w));
        (self.level[c], self.ramp[c])
    }

    /// Where a tile of the bog as the giant lies is now.
    pub fn to_world(&self, p: P) -> P {
        if in_palm(p) {
            return p;
        }
        (0..self.turn).fold(p, |q, _| turn_p(q))
    }

    pub fn to_canon(&self, w: P) -> P {
        if in_palm(w) {
            return w;
        }
        (0..self.turn).fold(w, |q, _| unturn_p(q))
    }

    /// A point as the giant lies, where it is now.
    pub fn v_world(&self, v: V) -> V {
        if in_palm(tile(v)) {
            return v;
        }
        (0..self.turn).fold(v, |q, _| turn_v(q))
    }

    /// A turn as the giant lies, as it is now.
    pub fn dir_world(&self, d: Dir) -> Dir {
        (0..self.turn).fold(d, |q, _| turn_dir(q))
    }

    /// The giant's head, where it lies now.
    pub fn head_world(&self) -> P {
        self.to_world(self.head)
    }

    /// The plank at canonical `p` may rot without leaving a gap too wide to leap (no rotten plank
    /// beside it).
    pub fn may_rot(&self, p: P) -> bool {
        self.cells[idx(p)] == Cell::Plank
            && !self.rotten[idx(p)]
            && CARDINALS.iter().all(|&d| {
                let q = p.step(d);
                !(inside(q) && self.rotten[idx(q)])
            })
    }

    /// World tile `w` is firm underfoot now (a causeway's rotten plank and a finger's sunk joint are
    /// water; a shortcut's way is firm while it runs).
    pub fn firm(&self, w: P) -> bool {
        if !inside(w) {
            return false;
        }
        let c = self.to_canon(w);
        match self.cells[idx(c)] {
            Cell::Plank => !self.rotten[idx(c)],
            Cell::Finger(k, i) => self.fingers[k as usize].up(i as usize),
            Cell::Track(k, i) => self.rides[k as usize].open(i as usize),
            other => other.firm(),
        }
    }

    /// The level's rows and heights for the bog as it lies now (see `logic::level`).
    pub fn rows(&self) -> (Vec<String>, Vec<String>) {
        let mut rows = Vec::with_capacity(SIZE as usize);
        let mut heights = Vec::with_capacity(SIZE as usize);
        let arrival = self.to_world(self.canon_arrival);
        let spawn = P::new(CENTRE, CENTRE + 5);
        for y in 0..SIZE {
            let mut row = String::with_capacity(SIZE as usize);
            let mut hrow = String::with_capacity(SIZE as usize);
            for x in 0..SIZE {
                let w = P::new(x, y);
                let c = self.cell(w);
                let ch = if w == arrival {
                    'P'
                } else if w == spawn {
                    'G'
                } else {
                    match c {
                        Cell::Door => 'D',
                        c if c.solid() => '#',
                        Cell::Plank | Cell::Finger(..) | Cell::Track(..) => {
                            if self.firm(w) {
                                '='
                            } else {
                                '~'
                            }
                        }
                        Cell::Pool | Cell::Moat => '~',
                        _ => ' ',
                    }
                };
                row.push(ch);
                let (lv, ramp) = self.floor_at(w);
                hrow.push(if ramp {
                    '/'
                } else if lv == 1 {
                    '1'
                } else {
                    '0'
                });
            }
            rows.push(row);
            heights.push(hrow);
        }
        (rows, heights)
    }

    /// The shared layout (`Base`) as the land lies now: its rows, the arrival and the portal behind
    /// him, his checkpoint, the packs.
    pub fn sync_base(&mut self) {
        let (rows, heights) = self.rows();
        self.base.rows = rows;
        self.base.heights = heights;
        self.base.arrival = self.to_world(self.canon_arrival);
        self.base.entry =
            Gate { tile: self.to_world(self.canon_entry.tile), facing: self.dir_world(self.canon_entry.facing), sin: Sin::Sloth };
        self.base.checkpoint = self.to_world(self.canon_checkpoint);
        let turned: Vec<Pack> = self
            .canon_packs
            .iter()
            .map(|pk| {
                let den = (0..self.turn).fold(pk.den, |r, _| turn_rect(r));
                Pack { den, foes: pk.foes.iter().map(|&(k, s, p)| (k, s, self.to_world(p))).collect() }
            })
            .collect();
        self.base.packs = turned;
    }

    /// How high the tide stands, 0 (drained) to 1 (flooded), `breath` seconds into the giant's breath.
    pub fn flood(&self) -> f64 {
        let t = self.breath.rem_euclid(BREATH);
        let rise = |a: f64, b: f64| ((t - a) / (b - a)).clamp(0.0, 1.0);
        let s = |x: f64| x * x * (3.0 - 2.0 * x);
        s(rise(5.0, 7.0)) * (1.0 - s(rise(10.0, 12.0)))
    }

    /// The water's height (world units) now: below the peat at low tide, over it at high tide, and
    /// higher with every notch it has risen.
    pub fn water_y(&self) -> f64 {
        -0.22 + 0.37 * self.flood() + 0.12 * self.notch as f64
    }

    /// World tile `w` is under the tide now (a hill only once the tide has risen far).
    pub fn flooded(&self, w: P) -> bool {
        let c = self.cell(w);
        if !matches!(c, Cell::Peat | Cell::Hill(_) | Cell::Moss | Cell::Mud | Cell::Hole | Cell::Station(_)) {
            return false;
        }
        let (lv, ramp) = self.floor_at(w);
        let floor = if ramp { 0.25 } else { lv as f64 * 0.55 };
        self.water_y() > floor + 0.05
    }

    /// The finger whose tip is nearest world point `v`.
    pub fn nearest_finger(&self, v: V) -> Option<usize> {
        (0..self.fingers.len()).filter(|&k| !self.fingers[k].is_empty()).min_by(|&a, &b| {
            let d = |k: usize| {
                let tip = self.to_world(*self.fingers[k].joints.last().unwrap()).v();
                crate::logic::types::dist_sq(tip, v)
            };
            d(a).total_cmp(&d(b))
        })
    }

    /// In the Long Sleep (the sand running).
    pub fn in_sand(&self) -> bool {
        matches!(self.phase, Phase::Sand { .. })
    }

    /// Seconds of sand left, while it runs.
    pub fn sand_left(&self) -> Option<f64> {
        match self.phase {
            Phase::Sand { left, .. } => Some(left),
            _ => None,
        }
    }

    /// How white the fog has surged as the land turns, 0 to 1.
    pub fn surge(&self) -> f64 {
        let Some((t, _)) = self.turning else { return 0.0 };
        let s = |x: f64| x.clamp(0.0, 1.0);
        s(t / (TURN_SECONDS * 0.4)) * (1.0 - s((t - TURN_SECONDS * 0.6) / (TURN_SECONDS * 0.4)))
    }
}

/// The firm ground joined to `from` (skipping the shortcuts' islands).
fn flood(cells: &[Cell], from: P, isle: &[bool]) -> HashSet<P> {
    let mut seen = HashSet::from([from]);
    let mut queue = VecDeque::from([from]);
    while let Some(p) = queue.pop_front() {
        for d in CARDINALS {
            let q = p.step(d);
            if inside(q) && cells[idx(q)].firm() && !isle[idx(q)] && seen.insert(q) {
                queue.push_back(q);
            }
        }
    }
    seen
}

/// Steps on foot from `a` to `b` (none: no way).
fn steps(cells: &[Cell], a: P, b: P, isle: &[bool]) -> Option<usize> {
    let mut dist = vec![usize::MAX; cells.len()];
    dist[idx(a)] = 0;
    let mut queue = VecDeque::from([a]);
    while let Some(p) = queue.pop_front() {
        if p == b {
            return Some(dist[idx(p)]);
        }
        for d in CARDINALS {
            let q = p.step(d);
            if inside(q) && cells[idx(q)].firm() && !isle[idx(q)] && dist[idx(q)] == usize::MAX {
                dist[idx(q)] = dist[idx(p)] + 1;
                queue.push_back(q);
            }
        }
    }
    None
}

/// The cheapest way of planks over the pools from `from` to `to` (the pools' tiles to plank; firm
/// ground on the way costs nothing), winding a little.
fn causeway(cells: &[Cell], from: &HashSet<P>, to: &HashSet<P>, isle: &[bool]) -> Option<Vec<P>> {
    #[derive(PartialEq, Eq)]
    struct Node(u32, P);
    impl Ord for Node {
        fn cmp(&self, o: &Self) -> std::cmp::Ordering {
            o.0.cmp(&self.0).then_with(|| (self.1.x, self.1.y).cmp(&(o.1.x, o.1.y)))
        }
    }
    impl PartialOrd for Node {
        fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(o))
        }
    }
    let n = cells.len();
    let mut cost = vec![u32::MAX; n];
    let mut back: Vec<Option<P>> = vec![None; n];
    let mut heap = BinaryHeap::new();
    for &p in from {
        cost[idx(p)] = 0;
        heap.push(Node(0, p));
    }
    let near_isle = |p: P| CARDINALS.iter().any(|&d| inside(p.step(d)) && isle[idx(p.step(d))]) || isle[idx(p)];
    while let Some(Node(c, p)) = heap.pop() {
        if c > cost[idx(p)] {
            continue;
        }
        if to.contains(&p) {
            let mut way = Vec::new();
            let mut q = p;
            while let Some(prev) = back[idx(q)] {
                if cells[idx(q)] == Cell::Pool {
                    way.push(q);
                }
                q = prev;
            }
            return Some(way);
        }
        for d in CARDINALS {
            let q = p.step(d);
            if !inside(q) || near_isle(q) || radius(q) <= MOAT_R + 0.5 {
                continue;
            }
            let step = match cells[idx(q)] {
                Cell::Pool => 10 + (noise(q.x as f64 / 2.0, q.y as f64 / 2.0, SEED + 9) * 14.0) as u32,
                c if c.firm() => 1,
                _ => continue,
            };
            let nc = c + step;
            if nc < cost[idx(q)] {
                cost[idx(q)] = nc;
                back[idx(q)] = Some(p);
                heap.push(Node(nc, q));
            }
        }
    }
    None
}

/// A bog hole may lie at `p`: low peat with firm ground all round it (so there is always a way
/// past), away from the arrival and the shore.
fn hole_fits(cells: &[Cell], p: P, arrival: P) -> bool {
    inside(p)
        && cells[idx(p)] == Cell::Peat
        && radius(p) > MOAT_R + 3.0
        && (p.x - arrival.x).abs() + (p.y - arrival.y).abs() > 5
        && CARDINALS.iter().all(|&d| matches!(cells[idx(p.step(d))], Cell::Peat | Cell::Mud | Cell::Moss | Cell::Plank))
        && CARDINALS.iter().any(|&d| {
            let q = p.step(d).step(d);
            inside(q) && matches!(cells[idx(q)], Cell::Pool | Cell::Plank)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::parse_level;
    use crate::logic::levels::level_for;
    use crate::logic::pathing::reachable;
    use crate::logic::types::Tile;

    /// The bog drawn in letters (for a look at the layout: `cargo test mire_map -- --nocapture`).
    fn map(m: &Mire) -> String {
        let mut out = String::new();
        for y in 0..SIZE {
            for x in 0..SIZE {
                let c = m.cells[idx(P::new(x, y))];
                out.push(match c {
                    Cell::Deep => ' ',
                    Cell::Peat => '.',
                    Cell::Hill(_) if m.ramp[idx(P::new(x, y))] => '/',
                    Cell::Hill(_) => '^',
                    Cell::Moss => 'm',
                    Cell::Mud => ',',
                    Cell::Hole => 'o',
                    Cell::Pool => '~',
                    Cell::Moat => '-',
                    Cell::Plank => '=',
                    Cell::Finger(..) => 'f',
                    Cell::Track(..) => '+',
                    Cell::Station(_) => 'S',
                    Cell::Palm => '_',
                    Cell::Threshold => 'K',
                    Cell::Tower => 'T',
                    Cell::Door => 'D',
                    Cell::Cottage(_) => 'C',
                    Cell::Chapel => 'H',
                    Cell::Willow => 'Y',
                    Cell::Oak => 'O',
                });
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn mire_map() {
        println!("{}", map(&Mire::generate(1)));
    }

    #[test]
    fn the_mire_is_one_bog_round_the_palm_and_the_hand_reaches_the_shore() {
        let mut m = Mire::generate(1);
        let level = parse_level(&m.base.level(&level_for(1)));
        assert_eq!(level.hero_spawn, m.base.arrival);
        assert_eq!(level.gate, Some(m.base.exit.tile));
        assert_eq!((m.base.width, m.base.height), (SIZE, SIZE));
        // Laid out, the hand lies open toward the arrival: one finger reaches the shore.
        let k = m.nearest_finger(m.base.arrival.v()).unwrap();
        assert!(m.fingers.iter().enumerate().all(|(i, f)| (i == k) == (f.reach == f.len() as f64)));
        assert!(reachable(&level.grid, m.base.arrival, false, false).contains(&m.bed));
        // The hand withdrawn, the palm is cut off by the moat.
        for f in m.fingers.iter_mut() {
            f.reach = 0.0;
        }
        m.sync_base();
        let level = parse_level(&m.base.level(&level_for(1)));
        let seen = reachable(&level.grid, m.base.arrival, false, false);
        assert!(!seen.contains(&m.bed), "no way onto the palm while the hand is withdrawn");
        // Every finger's tip touches the shore he can walk to.
        for (k, f) in m.fingers.iter().enumerate() {
            assert!(f.len() >= 5, "finger {k} has {} joints", f.len());
            let tip = *f.joints.last().unwrap();
            assert!(CARDINALS.iter().any(|&d| seen.contains(&tip.step(d))), "finger {k}'s tip is on the shore");
            assert!(CARDINALS.iter().any(|&d| in_palm(f.joints[0].step(d))), "finger {k} grows from the palm");
        }
        // Curled toward him, a finger is a way onto the palm, to the bed and the door.
        m.fingers[2].reach = m.fingers[2].len() as f64;
        m.sync_base();
        let level = parse_level(&m.base.level(&level_for(1)));
        let seen = reachable(&level.grid, m.base.arrival, false, false);
        assert!(seen.contains(&m.bed), "the bed");
        assert!(seen.contains(&m.base.exit.front()), "the door");
        assert_eq!(level.grid.get(m.base.entry.tile.x, m.base.entry.tile.y), Tile::Rock);
        assert!(seen.contains(&m.base.entry.front()));
        // Every station can be reached on foot, and every shortcut's way ends on the shore.
        for r in &m.rides {
            assert!(seen.contains(&r.path[0]), "{:?}'s station", r.kind);
            assert!(seen.contains(r.path.last().unwrap()), "{:?} ends where he can walk", r.kind);
            assert!(r.stop > 2 && r.stop + 3 < r.path.len());
            assert!(!seen.contains(&r.path[r.stop]), "{:?}'s island is cut off but for a leap", r.kind);
            assert!(CARDINALS.iter().filter(|&&d| m.cells[idx(r.path[r.stop].step(d))] == Cell::Moss).count() >= 2, "a lovely moss island");
        }
    }

    #[test]
    fn the_bog_has_its_pools_causeways_holes_and_moss_and_packs_in_their_dens() {
        let m = Mire::generate(3);
        let count = |f: fn(Cell) -> bool| m.cells.iter().filter(|&&c| f(c)).count();
        assert!(count(|c| c == Cell::Pool) > 400, "pools: {}", count(|c| c == Cell::Pool));
        assert!(count(|c| c == Cell::Plank) >= 12, "causeways: {}", count(|c| c == Cell::Plank));
        assert!(count(|c| c == Cell::Hole) >= 12, "holes: {}", count(|c| c == Cell::Hole));
        assert!(count(|c| c == Cell::Moss) >= 20, "moss: {}", count(|c| c == Cell::Moss));
        assert!(count(|c| c == Cell::Mud) >= 12, "mud: {}", count(|c| c == Cell::Mud));
        assert!(count(|c| matches!(c, Cell::Hill(Part::Head))) >= 30, "the head");
        assert!(count(|c| c == Cell::Willow) >= 8);
        let level = parse_level(&m.base.level(&level_for(3)));
        let seen = reachable(&level.grid, m.base.arrival, false, false);
        assert!(m.base.packs.len() >= 6, "{} packs", m.base.packs.len());
        let mut servants = 0;
        for pk in &m.base.packs {
            assert!(pk.foes.len() >= 3);
            for &(kind, stage, post) in &pk.foes {
                assert!(pk.den.contains(post) && seen.contains(&post), "{kind:?} at {post:?}");
                assert_ne!(m.cells[idx(post)], Cell::Hole);
                if kind == EnemyKind::Ganglot {
                    servants += 1;
                    assert_eq!(stage, 0);
                }
            }
        }
        assert!(servants >= 3, "Hel's servants among the natives");
        // A pack lies by the giant's head, where a fight wakes him.
        let head = m.head_world().v();
        assert!(m.base.packs.iter().any(|pk| {
            let (cx, cy) = pk.den.center();
            crate::logic::types::dist_sq(V::new(cx as f64, cy as f64), head) < 64.0
        }));
        // The holes always leave a way past.
        for p in (0..SIZE).flat_map(|y| (0..SIZE).map(move |x| P::new(x, y))) {
            if m.cells[idx(p)] == Cell::Hole {
                assert!(CARDINALS.iter().filter(|&&d| m.cells[idx(p.step(d))].firm()).count() >= 3);
            }
        }
    }

    #[test]
    fn the_land_turns_round_the_hourglass_and_the_palm_stays_put() {
        let mut m = Mire::generate(1);
        for f in m.fingers.iter_mut() {
            f.reach = 0.0;
        }
        m.sync_base();
        let before = m.base.rows.clone();
        let (head, arrival, entry, packs) = (m.head_world(), m.base.arrival, m.base.entry, m.base.packs.clone());
        m.turn = 1;
        m.sync_base();
        // North goes east: the head swings a quarter round, the arrival too, and the portal turns with it.
        assert_eq!(m.head_world(), turn_p(head));
        assert_eq!(m.base.arrival, turn_p(arrival));
        assert_eq!(m.base.entry.facing, turn_dir(entry.facing));
        assert_eq!(m.base.entry.front(), turn_p(entry.front()));
        for (a, b) in packs.iter().zip(&m.base.packs) {
            assert_eq!(b.den, turn_rect(a.den));
            assert!(a.foes.iter().zip(&b.foes).all(|(f, g)| g.2 == turn_p(f.2) && b.den.contains(g.2)));
        }
        // The palm, the tower, the door and the bed stay where they were.
        let ch = |rows: &Vec<String>, p: P| rows[p.y as usize].chars().nth(p.x as usize).unwrap();
        for y in 0..SIZE {
            for x in 0..SIZE {
                let p = P::new(x, y);
                if radius(p) <= MOAT_R && !matches!(m.cell(p), Cell::Finger(..)) {
                    assert_eq!(ch(&before, p), ch(&m.base.rows, p), "{p:?}");
                }
            }
        }
        assert_eq!(m.base.exit.tile, P::new(CENTRE, CENTRE - 1));
        let level = parse_level(&m.base.level(&level_for(1)));
        assert_eq!(level.gate, Some(m.base.exit.tile));
        assert_eq!(level.hero_spawn, m.base.arrival);
        // Four turns and it lies as it did.
        m.turn = 0;
        m.sync_base();
        assert_eq!(m.base.rows, before);
    }

    #[test]
    fn an_older_mire_has_rotten_causeways_more_servants_and_a_higher_tide() {
        let young = Mire::generate(2);
        let old = Mire::generate_aged(2, 1);
        assert_eq!(young.cells, old.cells, "the same bog");
        let rotten = old.rotten.iter().filter(|&&r| r).count();
        let planks = old.cells.iter().filter(|&&c| c == Cell::Plank).count();
        assert!(rotten * 5 >= planks, "{rotten} of {planks} planks rotten");
        // Never two rotten side by side: every gap can be leapt.
        for p in (0..SIZE).flat_map(|y| (0..SIZE).map(move |x| P::new(x, y))) {
            if old.rotten[idx(p)] {
                assert!(CARDINALS.iter().all(|&d| !old.rotten[idx(p.step(d))]));
            }
        }
        let servants = |m: &Mire| m.base.packs.iter().flat_map(|p| &p.foes).filter(|f| f.0 == EnemyKind::Ganglot).count();
        assert!(servants(&old) >= servants(&young) * 2, "{} servants, then {}", servants(&young), servants(&old));
        assert!(old.water_y() > young.water_y());
    }

    #[test]
    fn the_tide_comes_with_the_giants_breath() {
        let mut m = Mire::generate(1);
        let low = P::new(CENTRE - 26, CENTRE);
        assert_eq!(m.cell(low), Cell::Peat);
        m.breath = 1.0;
        assert!(!m.flooded(low) && m.flood() == 0.0);
        m.breath = 8.5;
        assert!(m.flooded(low) && m.flood() == 1.0, "high tide floods the low peat");
        let hill = (0..SIZE)
            .flat_map(|y| (0..SIZE).map(move |x| P::new(x, y)))
            .find(|&p| matches!(m.cell(p), Cell::Hill(_)) && !m.ramp[idx(p)])
            .unwrap();
        assert!(!m.flooded(hill), "but not the giant");
        m.breath = 12.5;
        assert!(!m.flooded(low), "and it drains");
    }
}
