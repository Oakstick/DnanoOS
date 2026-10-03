//! The Burning Field: Wrath's realm (REALMS.md §3), Amon's battlefield broken into slabs of scorched
//! earth adrift on a lava sea.
//!
//! He comes through the portal onto the war camp at the south end; Amon's banner-hill rises at the
//! north end. Between them the field drifts in lanes running west to east, each lane in one of three
//! height bands (the low shelf, the field, the high ridge), and each band at its own pace, so the gaps
//! between lanes open and close like ice floes. The slabs carry fields of blades, palisades, siege
//! towers, war wagons and torn banners, and packs of Amon's natives hold the larger ones.
//!
//! The ground keeps score: every blow cracks the slab it lands on, and at the third crack it splits,
//! the smaller piece sinking. Lightning strikes every eight seconds or so, and at every flash the slain
//! rise where they fell, unless their bodies went into the lava.
//!
//! At the foot of the hill, on the herald's isle, a war horn hangs on a burnt gallows-tree, and a
//! raven-headed herald says: "Sound the horn and the battle ends." It is a lie. The horn calls the
//! whole field: the bands lurch, the lava rises over the low shelf, the flashes come every four
//! seconds, and five slabs rise out of the moat, the last of them, the causeway to the hill, sinking
//! from the moment it rises. On the hill's terrace stands the truce stone, the real gate: it opens
//! after five seconds in which he swings at nothing within four tiles of it.
//!
//! This module is the layout and the slabs' geometry (`game::realms::field` plays the rules, and
//! `render::view::realms::field` draws it all).

use super::{gate, pick_native, Base, Canvas, Pack};
use crate::logic::enemy::EnemyKind;
use crate::logic::grid::Grid;
use crate::logic::hub::Rect;
use crate::logic::rng::Rng;
use crate::logic::sins::Sin;
use crate::logic::types::{Dir, Tile, P, V};

/// The layout's seed: the realm is the same every time.
pub const SEED: u32 = 7303;
/// The field's size in tiles.
pub const WIDTH: i32 = 60;
pub const HEIGHT: i32 = 118;

/// Health a fall into the lava costs him: a third of what he has when whole.
pub const LAVA_SHARE: f64 = 1.0 / 3.0;
/// Fury: a stack for every blow he lands, up to ten, each adding to his blows and his pace.
pub const FURY_MAX: u32 = 10;
pub const FURY_DAMAGE: f64 = 0.06;
pub const FURY_SPEED: f64 = 0.04;
/// Fury holds this long after his last swing, and then a stack goes every `FURY_FADE` seconds.
pub const FURY_HOLD: f64 = 2.0;
pub const FURY_FADE: f64 = 0.5;
/// Berserkergang, at full fury: this long with his controls locked on the nearest foe, and no guard.
pub const BERSERK_SECONDS: f64 = 6.0;
/// Blows a slab takes for each crack, the cracks it takes before it splits, and how long the smaller
/// piece takes to sink.
pub const STRAIN_PER_CRACK: u32 = 3;
pub const CRACKS_TO_SPLIT: u32 = 3;
pub const SINK_SECONDS: f64 = 4.0;
/// A piece no longer than this along its long side sinks whole rather than splitting.
pub const MIN_PIECE: i32 = 2;
/// How fast a sinking piece drifts off the slab it split from (tiles a second).
pub const PART: f64 = 0.35;
/// Lightning: about every eight seconds (every four once the horn has called the field), a flash this long.
pub const FLASH_EVERY: f64 = 8.0;
pub const CALLED_FLASH_EVERY: f64 = 4.0;
pub const FLASH_SECONDS: f64 = 0.3;
/// The first flash after he arrives (or rises again).
pub const FIRST_FLASH: f64 = 5.0;
/// The horn: the causeway's time, how long the five slabs take to rise, and the lava's rise.
pub const CAUSEWAY_SECONDS: f64 = 60.0;
pub const RISE_SECONDS: f64 = 2.5;
pub const LAVA_SECONDS: f64 = 3.0;
/// The shelf drowns over this long once the lava rises.
pub const DROWN_SECONDS: f64 = 3.0;
/// The drift runs this much faster once the horn has called the field, and every band lurches by this.
pub const CALLED_PACE: f64 = 1.6;
pub const LURCH: f64 = 0.7;
/// The truce stone: seconds of no swinging, within this many tiles of it.
pub const TRUCE_SECONDS: f64 = 5.0;
pub const TRUCE_RADIUS: f64 = 4.0;
/// A foe whose poise breaks this near an edge topples into the lava.
pub const EDGE_TOPPLE: f64 = 1.0;
/// He blows the horn with a swing from this near it.
pub const HORN_REACH: f64 = 1.8;
/// The herald speaks when he comes this near.
pub const HERALD_RANGE: f64 = 5.0;
/// How far past a slab's edge he may stand before he is in the lava (free movement; on the grid, half a tile).
pub const SLACK: f64 = 0.15;
/// A tile under the edge of a slab counts as its floor when this much of it is covered (each way).
const COVER: f64 = 0.1;

/// The three height bands: their floor elevation and how fast they drift.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Band {
    /// The low shelf, a raft barely above the lava: the fastest.
    Shelf,
    /// The field.
    Field,
    /// The high ridge: the slowest.
    Ridge,
}

impl Band {
    pub const ALL: [Band; 3] = [Band::Shelf, Band::Field, Band::Ridge];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn elevation(self) -> i32 {
        self as i32
    }

    /// The band's drift, radians of its phase a second.
    pub fn pace(self) -> f64 {
        match self {
            Band::Shelf => 0.52,
            Band::Field => 0.37,
            Band::Ridge => 0.25,
        }
    }
}

/// What stands planted in a slab, impassable: a thicket of broken swords and spears (a field of
/// blades), a charred palisade, a siege tower (two by two), a war wagon (two long, along x), or a
/// torn banner on its pole amid stakes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Block {
    Blades,
    Palisade,
    Tower,
    Wagon,
    Banner,
}

impl Block {
    /// The tiles it covers, from its anchor.
    pub fn cover(self, at: P) -> Vec<P> {
        match self {
            Block::Tower => vec![at, P::new(at.x + 1, at.y), P::new(at.x, at.y + 1), P::new(at.x + 1, at.y + 1)],
            Block::Wagon => vec![at, P::new(at.x + 1, at.y)],
            _ => vec![at],
        }
    }
}

/// How a slab drifts: `amp` tiles either way along x and y, at its band's pace times `rate`, from `phase`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Drift {
    pub amp: V,
    pub rate: V,
    pub phase: V,
}

impl Drift {
    pub fn offset(&self, band_phase: f64) -> V {
        V::new(self.amp.x * (band_phase * self.rate.x + self.phase.x).sin(), self.amp.y * (band_phase * self.rate.y + self.phase.y).sin())
    }
}

/// What a slab is: one of the field's, or one of the five that rise out of the moat at the horn's
/// call (`k` from the herald's isle; the fifth is the causeway to the hill).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SlabKind {
    Field,
    Moat(u8),
}

/// A slab going down into the lava: seconds since it began, and how long it takes.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Sink {
    pub t: f64,
    pub over: f64,
}

impl Sink {
    pub fn new(over: f64) -> Self {
        Self { t: 0.0, over }
    }

    /// 0 as it begins to go down, 1 when it is gone.
    pub fn progress(&self) -> f64 {
        (self.t / self.over).clamp(0.0, 1.0)
    }

    pub fn gone(&self) -> bool {
        self.t >= self.over
    }
}

/// A slab of scorched earth: `w` by `h` tiles, its local tile (0, 0) centred on `origin` (drifting
/// about `home`). `gen` changes whenever its shape does (a split), for the view.
#[derive(Clone, Debug)]
pub struct Slab {
    pub id: u32,
    pub gen: u32,
    pub kind: SlabKind,
    pub band: Band,
    pub w: i32,
    pub h: i32,
    pub home: V,
    pub drift: Drift,
    pub origin: V,
    /// Its blocks, by local anchor tile.
    pub blocks: Vec<(P, Block)>,
    /// Blows it has taken, and where they landed (local).
    pub strain: u32,
    pub scars: Vec<V>,
    pub sinking: Option<Sink>,
    /// 0 sunk in the moat to 1 risen (the field's slabs are always 1).
    pub risen: f64,
    /// A sinking piece drifts away from the slab it split from, this many tiles a second.
    pub part: V,
}

impl Slab {
    /// Its corners as continuous positions: the low one and the high one.
    pub fn bounds(&self) -> (V, V) {
        (V::new(self.origin.x - 0.5, self.origin.y - 0.5), V::new(self.origin.x + self.w as f64 - 0.5, self.origin.y + self.h as f64 - 0.5))
    }

    /// `p` lies on it, or no more than `slack` past its edge.
    pub fn contains(&self, p: V, slack: f64) -> bool {
        let (lo, hi) = self.bounds();
        p.x >= lo.x - slack && p.x <= hi.x + slack && p.y >= lo.y - slack && p.y <= hi.y + slack
    }

    /// How far `p` is from its nearest edge (negative outside).
    pub fn edge_dist(&self, p: V) -> f64 {
        let (lo, hi) = self.bounds();
        (p.x - lo.x).min(hi.x - p.x).min(p.y - lo.y).min(hi.y - p.y)
    }

    pub fn local(&self, p: V) -> V {
        V::new(p.x - self.origin.x, p.y - self.origin.y)
    }

    pub fn world(&self, local: V) -> V {
        V::new(self.origin.x + local.x, self.origin.y + local.y)
    }

    /// The grid tile under its local tile `l` now.
    pub fn tile_of(&self, l: P) -> P {
        P::new(self.origin.x.round() as i32 + l.x, self.origin.y.round() as i32 + l.y)
    }

    /// The local tile under `p` (which may be off it).
    pub fn local_tile(&self, p: V) -> P {
        P::new(p.x.round() as i32 - self.origin.x.round() as i32, p.y.round() as i32 - self.origin.y.round() as i32)
    }

    pub fn elevation(&self) -> i32 {
        self.band.elevation()
    }

    /// It is ground underfoot: risen, and not yet gone under.
    pub fn solid(&self) -> bool {
        self.risen >= 1.0 && !self.sinking.is_some_and(|s| s.gone())
    }

    /// Something impassable stands on its local tile `l`.
    pub fn blocked(&self, l: P) -> bool {
        self.blocks.iter().any(|&(at, b)| b.cover(at).contains(&l))
    }

    /// Its local tile `l` is open ground on it.
    pub fn open_at(&self, l: P) -> bool {
        l.x >= 0 && l.y >= 0 && l.x < self.w && l.y < self.h && !self.blocked(l)
    }

    /// The cracks it shows: one for every `STRAIN_PER_CRACK` blows.
    pub fn cracks(&self) -> u32 {
        (self.strain / STRAIN_PER_CRACK).min(CRACKS_TO_SPLIT)
    }

    /// It drifts (the causeway to the hill lies still against it).
    pub fn drifts(&self) -> bool {
        self.drift.amp.x != 0.0 || self.drift.amp.y != 0.0
    }
}

/// Ground under someone: a slab (by id) and the point on it, or a fixed isle and the point.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Ground {
    Slab(u32, V),
    Isle(V),
}

/// One of the slain, lying where it fell until the next flash raises it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Corpse {
    pub kind: EnemyKind,
    pub at: Ground,
}

/// A foe as the field last saw it: to know where the slain fell.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Seen {
    pub id: u32,
    pub kind: EnemyKind,
    pub pos: V,
    pub hp: f64,
    pub ground: Option<Ground>,
}

/// The Burning Field: its layout, its drifting slabs and the state of its rules.
#[derive(Clone, Debug)]
pub struct Battlefield {
    pub base: Base,
    pub slabs: Vec<Slab>,
    next_id: u32,
    /// Each band's drift: an angle running on at the band's pace.
    pub phase: [f64; 3],
    /// Seconds since he arrived (or rose again).
    pub clock: f64,
    /// Seconds since the last flash, until the next, where its bolt struck, and how many there have been.
    pub flash: f64,
    pub next_flash: f64,
    pub bolt: V,
    pub flashes: u32,
    /// 0 to 1: the lava risen over the low shelf.
    pub lava: f64,
    /// The isles that never drift: the war camp he arrives on, the herald's isle, the hill's terrace.
    pub camp: Rect,
    pub isle: Rect,
    pub terrace: Rect,
    /// The gallows-tree the horn hangs from, the herald, the truce stone (all on rock), and where he
    /// rises on the herald's isle.
    pub horn: P,
    pub herald: P,
    pub stone: P,
    pub rest: P,
    /// Seconds since the horn was blown (none: silent), whether he has reached the terrace since, the
    /// seconds of truce he has held, and whether the stone has opened the way.
    pub called: Option<f64>,
    pub crossed: bool,
    pub truce: f64,
    pub opened: bool,
    /// Seconds since the horn last sounded (for its swing), and since the stone opened.
    pub horn_age: f64,
    pub open_age: f64,
    /// Fury: stacks, seconds since his last swing, the fading's clock, seconds of Berserkergang left.
    pub fury: u32,
    pub idle: f64,
    pub fade: f64,
    pub berserk: f64,
    /// The slain, until they rise.
    pub corpses: Vec<Corpse>,
    /// The slab he stands on (none: an isle, the air or the lava), and the last safe spot he stood on.
    pub on: Option<u32>,
    pub safe: Ground,
    /// The tiles the slabs drift over, and those of them that are lava now.
    sea: Vec<bool>,
    lava_tiles: Vec<bool>,
    rng: Rng,
    /// The fight as last seen: the foes, his blows landed and blows on him, the foes toppled into the lava.
    pub seen: Vec<Seen>,
    pub seen_hits: u32,
    pub seen_struck: u32,
    pub toppled: Vec<u32>,
    /// He has been welcomed, and the herald has spoken to him (since he last came near).
    pub welcomed: bool,
    pub heard: bool,
    /// Seconds until a foe held at an edge may taunt him again, and how many have.
    pub taunt_rest: f64,
    pub taunts: u32,
}

/// A lane of the field: its rows, band, drift and the slabs along it (first column and width).
struct Lane {
    y0: i32,
    h: i32,
    band: Band,
    amp: (f64, f64),
    rate: (f64, f64),
    phase: (f64, f64),
    segs: &'static [(i32, i32)],
}

/// The field's lanes, north to south: from the herald's isle down to the war camp. Neighbouring
/// lanes are never more than one band apart going north, so a leap always climbs far enough.
const LANES: [Lane; 6] = [
    Lane { y0: 50, h: 7, band: Band::Ridge, amp: (4.0, 0.5), rate: (1.0, 1.7), phase: (0.3, 1.1), segs: &[(8, 12), (24, 13), (41, 11)] },
    Lane { y0: 59, h: 8, band: Band::Field, amp: (5.0, 0.5), rate: (1.0, 1.9), phase: (2.0, 0.2), segs: &[(7, 15), (26, 9), (39, 14)] },
    Lane { y0: 69, h: 7, band: Band::Shelf, amp: (6.0, 0.5), rate: (1.0, 1.6), phase: (4.1, 2.6), segs: &[(9, 10), (23, 16), (43, 10)] },
    Lane { y0: 78, h: 9, band: Band::Field, amp: (4.0, 0.5), rate: (1.2, 1.5), phase: (5.0, 3.9), segs: &[(6, 18), (28, 8), (40, 14)] },
    Lane { y0: 89, h: 7, band: Band::Ridge, amp: (4.0, 0.5), rate: (1.1, 2.1), phase: (1.4, 4.4), segs: &[(10, 13), (27, 10), (41, 12)] },
    Lane { y0: 98, h: 6, band: Band::Field, amp: (5.0, 0.5), rate: (0.9, 1.8), phase: (3.3, 5.5), segs: &[(8, 12), (24, 12), (40, 12)] },
];

/// The five slabs of the moat, from the herald's isle north to the hill: rows, columns, band and
/// drift. The fifth is the causeway, lying still against the terrace.
const MOAT: [(i32, i32, i32, i32, Band, f64); 5] = [
    (36, 3, 26, 9, Band::Ridge, 3.0),
    (30, 3, 24, 8, Band::Field, 4.0),
    (25, 3, 28, 8, Band::Ridge, 3.5),
    (20, 3, 25, 8, Band::Field, 4.0),
    (15, 3, 21, 18, Band::Field, 0.0),
];

impl Battlefield {
    /// The realm laid out for `tier`.
    pub fn generate(tier: usize) -> Battlefield {
        let mut rng = Rng::new(SEED);
        let camp = Rect { x0: 22, y0: 105, x1: 37, y1: 114 };
        let isle = Rect { x0: 20, y0: 40, x1: 39, y1: 48 };
        let terrace = Rect { x0: 21, y0: 5, x1: 38, y1: 14 };
        let (horn, herald, stone, rest) = (P::new(29, 40), P::new(33, 40), P::new(30, 9), P::new(30, 45));
        // He comes in through a portal in the camp's west side (the camera looks north from the south:
        // nothing tall stands south of him).
        let arrival = P::new(camp.x0, (camp.y0 + camp.y1) / 2 + 1);
        let entry = gate(Sin::Wrath, P::new(camp.x0 - 1, arrival.y), Dir::Right);
        let exit = gate(Sin::Wrath, P::new(30, terrace.y0 - 1), Dir::Down);

        // The slabs: the field's lanes, then the moat's five (sunk until the horn calls them).
        let mut slabs = Vec::new();
        let mut next_id = 1;
        for (li, lane) in LANES.iter().enumerate() {
            for (si, &(x0, w)) in lane.segs.iter().enumerate() {
                // Each slab of a lane drifts with it, a little of its own phase across the lane's
                // y-drift, so that a lane's gaps breathe too.
                let drift = Drift {
                    amp: V::new(lane.amp.0, lane.amp.1),
                    rate: V::new(lane.rate.0, lane.rate.1),
                    phase: V::new(lane.phase.0, lane.phase.1 + si as f64 * 0.15 * (li % 2) as f64),
                };
                slabs.push(Slab::new(next_id, SlabKind::Field, lane.band, x0, lane.y0, w, lane.h, drift));
                next_id += 1;
            }
        }
        for (k, &(y0, h, x0, w, band, ax)) in MOAT.iter().enumerate() {
            let drift = if ax > 0.0 {
                Drift { amp: V::new(ax, 0.4), rate: V::new(1.3 + 0.2 * k as f64, 2.2), phase: V::new(k as f64 * 1.9, k as f64 * 2.7 + 0.5) }
            } else {
                Drift::default()
            };
            let mut s = Slab::new(next_id, SlabKind::Moat(k as u8), band, x0, y0, w, h, drift);
            s.risen = 0.0;
            slabs.push(s);
            next_id += 1;
        }
        for s in slabs.iter_mut() {
            plant(s, &mut rng);
        }

        // The ground: lava everywhere but the isles; the hill's rock along the north.
        let mut c = Canvas::new(WIDTH, HEIGHT);
        c.fill(0, 5, WIDTH - 1, HEIGHT - 1, ' ');
        c.fill(0, 0, WIDTH - 1, 4, '#');
        for (r, h) in [(camp, '0'), (isle, '1'), (terrace, '0')] {
            for p in r.tiles() {
                c.set_height(p, h);
            }
        }
        // The portal's wall beside the camp, and its towers.
        c.fill(camp.x0 - 1, arrival.y - 1, camp.x0 - 1, arrival.y + 1, '#');
        for s in slabs.iter().filter(|s| s.risen >= 1.0) {
            for ly in 0..s.h {
                for lx in 0..s.w {
                    let t = s.tile_of(P::new(lx, ly));
                    c.set_height(t, char::from(b'0' + s.elevation() as u8));
                    if s.blocked(P::new(lx, ly)) {
                        c.set(t, '#');
                    }
                }
            }
        }
        for p in [horn, herald, stone] {
            c.set(p, '#');
        }
        // A few stakes about the camp and the terrace, standing in the way.
        for p in
            [P::new(camp.x0, camp.y0 + 2), P::new(camp.x1, camp.y0 + 4), P::new(terrace.x0, terrace.y0), P::new(terrace.x1, terrace.y0)]
        {
            c.set(p, '#');
        }
        c.set(arrival, 'P');
        c.set(exit.tile, 'D');
        c.set(P::new(camp.x1 - 1, camp.y1 - 1), 'G');

        // The packs: on the larger slabs of the field, and Amon's guard on the terrace.
        let mut packs = Vec::new();
        for s in slabs.iter().filter(|s| s.kind == SlabKind::Field && s.w >= 10) {
            // (Not every lane's every slab: about two in three.)
            if rng.next() > 0.68 {
                continue;
            }
            let n = 2 + (tier >= 3) as usize + (rng.next() < 0.4) as usize;
            let posts = posts_on(s, n, &mut rng);
            if posts.is_empty() {
                continue;
            }
            let den = Rect {
                x0: s.tile_of(P::new(0, 0)).x,
                y0: s.tile_of(P::new(0, 0)).y,
                x1: s.tile_of(P::new(s.w - 1, 0)).x,
                y1: s.tile_of(P::new(0, s.h - 1)).y,
            };
            let foes = posts
                .into_iter()
                .map(|l| {
                    let (kind, stage) = pick_native(&mut rng, Sin::Wrath, tier);
                    (kind, stage, s.tile_of(l))
                })
                .collect();
            packs.push(Pack { den, foes });
        }
        let guard = [P::new(25, 8), P::new(35, 8), P::new(27, 12)];
        packs.push(Pack {
            den: terrace,
            foes: guard
                .into_iter()
                .map(|p| {
                    let (kind, stage) = pick_native(&mut rng, Sin::Wrath, tier);
                    (kind, stage, p)
                })
                .collect(),
        });

        let (rows, heights) = c.finish();
        let base =
            Base { sin: Sin::Wrath, width: WIDTH, height: HEIGHT, rows, heights, arrival, entry, exit, packs, checkpoint: arrival, tier };
        let mut sea = vec![false; (WIDTH * HEIGHT) as usize];
        for y in 5..HEIGHT {
            for x in 0..WIDTH {
                let p = P::new(x, y);
                let fixed = camp.contains(p) || isle.contains(p) || terrace.contains(p) || (x == camp.x0 - 1 && (y - arrival.y).abs() <= 1);
                sea[(y * WIDTH + x) as usize] = !fixed;
            }
        }
        let mut f = Battlefield {
            base,
            slabs,
            next_id,
            phase: [0.0; 3],
            clock: 0.0,
            flash: 99.0,
            next_flash: FIRST_FLASH,
            bolt: V::new(30.0, 100.0),
            flashes: 0,
            lava: 0.0,
            camp,
            isle,
            terrace,
            horn,
            herald,
            stone,
            rest,
            called: None,
            crossed: false,
            truce: 0.0,
            opened: false,
            horn_age: 99.0,
            open_age: 0.0,
            fury: 0,
            idle: 0.0,
            fade: 0.0,
            berserk: 0.0,
            corpses: Vec::new(),
            on: None,
            safe: Ground::Isle(arrival.v()),
            sea,
            lava_tiles: vec![false; (WIDTH * HEIGHT) as usize],
            rng: Rng::new(SEED ^ 0x5eed),
            seen: Vec::new(),
            seen_hits: 0,
            seen_struck: 0,
            toppled: Vec::new(),
            welcomed: false,
            heard: false,
            taunt_rest: 0.0,
            taunts: 0,
        };
        f.place_all();
        f
    }

    /// The field as it was laid out, its slabs whole and its moat sunk, but what he has earned kept:
    /// where he rises, the stone opened, his welcome.
    pub fn restored(&self) -> Battlefield {
        let mut f = Battlefield::generate(self.base.tier);
        f.base.checkpoint = self.base.checkpoint;
        // (New pieces never take an old piece's id: the view knows its slabs by them.)
        f.next_id = f.next_id.max(self.next_id);
        f.opened = self.opened;
        f.welcomed = self.welcomed;
        f.safe = Ground::Isle(self.base.checkpoint.v());
        f
    }

    /// A fresh id for a slab.
    fn fresh_id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id - 1
    }

    pub fn slab(&self, id: u32) -> Option<&Slab> {
        self.slabs.iter().find(|s| s.id == id)
    }

    pub fn slab_mut(&mut self, id: u32) -> Option<&mut Slab> {
        self.slabs.iter_mut().find(|s| s.id == id)
    }

    /// Where `slab` stands for the bands' phases `phase`.
    fn origin_at(slab: &Slab, phase: &[f64; 3]) -> V {
        let o = slab.drift.offset(phase[slab.band.index()]);
        V::new(slab.home.x + o.x, slab.home.y + o.y)
    }

    /// Every slab where the bands' phases put it.
    pub fn place_all(&mut self) {
        let phase = self.phase;
        for s in self.slabs.iter_mut() {
            s.origin = Self::origin_at(s, &phase);
        }
    }

    /// The bands run on `dt` seconds (faster once the horn has called the field): each slab's new
    /// origin, and how far each moved (by id).
    pub fn drift(&mut self, dt: f64) -> Vec<(u32, V)> {
        let pace = if self.called.is_some() && !self.opened { CALLED_PACE } else { 1.0 };
        for b in Band::ALL {
            self.phase[b.index()] += b.pace() * pace * dt;
        }
        for s in self.slabs.iter_mut().filter(|s| s.sinking.is_some()) {
            s.home = V::new(s.home.x + s.part.x * dt, s.home.y + s.part.y * dt);
        }
        self.shift_all()
    }

    /// Every band jolts forward (the horn's call).
    pub fn lurch(&mut self) -> Vec<(u32, V)> {
        for p in self.phase.iter_mut() {
            *p += LURCH;
        }
        self.shift_all()
    }

    fn shift_all(&mut self) -> Vec<(u32, V)> {
        let phase = self.phase;
        let mut moved = Vec::new();
        for s in self.slabs.iter_mut() {
            let to = Self::origin_at(s, &phase);
            let d = V::new(to.x - s.origin.x, to.y - s.origin.y);
            s.origin = to;
            if d.x != 0.0 || d.y != 0.0 {
                moved.push((s.id, d));
            }
        }
        moved
    }

    /// The solid slab under `p` (no more than `slack` past its edge), the one he stands on first if
    /// it is `prefer`, else the highest.
    pub fn slab_under(&self, p: V, slack: f64, prefer: Option<u32>) -> Option<&Slab> {
        if let Some(s) = prefer.and_then(|id| self.slab(id)).filter(|s| s.solid() && s.contains(p, slack)) {
            return Some(s);
        }
        self.slabs.iter().filter(|s| s.solid() && s.contains(p, slack)).max_by_key(|s| s.elevation())
    }

    /// `p` is on one of the isles that never drift (or the hill's rock).
    pub fn on_isle(&self, p: V) -> bool {
        let t = P::new(p.x.round() as i32, p.y.round() as i32);
        t.x >= 0 && t.y >= 0 && t.x < WIDTH && t.y < HEIGHT && !self.sea[(t.y * WIDTH + t.x) as usize]
    }

    /// The tile is lava now (a tile of the sea no slab covers).
    pub fn lava_at(&self, t: P) -> bool {
        t.x >= 0 && t.y >= 0 && t.x < WIDTH && t.y < HEIGHT && self.lava_tiles[(t.y * WIDTH + t.x) as usize]
    }

    /// Lava lies within `reach` tiles of `p` (an edge is near).
    pub fn edge_near(&self, p: V, reach: f64) -> bool {
        let steps = (reach * 2.0).ceil() as i32;
        (-steps..=steps).any(|ky| {
            (-steps..=steps).any(|kx| {
                let q = V::new(p.x + kx as f64 * reach / steps as f64, p.y + ky as f64 * reach / steps as f64);
                self.lava_at(P::new(q.x.round() as i32, q.y.round() as i32)) && self.slab_under(q, 0.0, None).is_none()
            })
        })
    }

    /// Where someone standing at `p` is: on a slab (and where on it), on an isle, or nowhere (the lava).
    pub fn ground_at(&self, p: V, slack: f64, prefer: Option<u32>) -> Option<Ground> {
        if let Some(s) = self.slab_under(p, slack, prefer) {
            return Some(Ground::Slab(s.id, s.local(p)));
        }
        self.on_isle(p).then_some(Ground::Isle(p))
    }

    /// The open tile nearest `p`, a tile in from the edges of a slab that is not going down.
    pub fn sound_ground_near(&self, p: V) -> Option<Ground> {
        self.slabs
            .iter()
            .filter(|s| s.solid() && s.sinking.is_none() && s.w >= 3 && s.h >= 3)
            .flat_map(|s| (1..s.h - 1).flat_map(move |y| (1..s.w - 1).map(move |x| (s, P::new(x, y)))))
            .filter(|(s, l)| s.open_at(*l))
            .map(|(s, l)| (s.id, l.v(), crate::logic::types::dist_sq(s.world(l.v()), p)))
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(id, l, _)| Ground::Slab(id, l))
    }

    /// Where a ground is now (none: its slab is gone).
    pub fn ground_pos(&self, g: Ground) -> Option<V> {
        match g {
            Ground::Isle(p) => Some(p),
            Ground::Slab(id, l) => self.slab(id).filter(|s| s.solid()).map(|s| s.world(l)),
        }
    }

    /**
     * The slabs as tiles: every tile of the sea is lava (open floor at elevation 0, walkable so that
     * a misjudged leap lands in it) unless a solid slab covers it, and then it is that slab's floor at
     * its band's elevation, or rock where something stands planted in it. A tile only partly under a
     * slab's edge counts as its floor too, so that the floor under him never ends before the slab he
     * sees does; where two slabs cover a tile, the one he stands on (`prefer`) wins, then one that
     * covers the tile's middle, then the higher.
     */
    pub fn mark(&mut self, grid: &mut Grid, prefer: Option<u32>) {
        let n = (WIDTH * HEIGHT) as usize;
        // (priority, elevation, rock) for every tile.
        let mut best: Vec<(u8, i32, bool)> = vec![(0, 0, false); n];
        for s in self.slabs.iter().filter(|s| s.solid()) {
            let (lo, hi) = s.bounds();
            let e = s.elevation();
            for ty in (lo.y.floor() as i32).max(0)..=(hi.y.ceil() as i32).min(HEIGHT - 1) {
                let oy = (hi.y.min(ty as f64 + 0.5) - lo.y.max(ty as f64 - 0.5)).max(0.0);
                if oy <= COVER {
                    continue;
                }
                for tx in (lo.x.floor() as i32).max(0)..=(hi.x.ceil() as i32).min(WIDTH - 1) {
                    let ox = (hi.x.min(tx as f64 + 0.5) - lo.x.max(tx as f64 - 0.5)).max(0.0);
                    if ox <= COVER {
                        continue;
                    }
                    let strict = tx as f64 >= lo.x && (tx as f64) < hi.x && ty as f64 >= lo.y && (ty as f64) < hi.y;
                    let pri = if prefer == Some(s.id) {
                        3
                    } else if strict {
                        2
                    } else {
                        1
                    };
                    let i = (ty * WIDTH + tx) as usize;
                    if (pri, e) > (best[i].0, best[i].1) {
                        best[i] = (pri, e, best[i].2);
                    }
                }
            }
            for &(at, b) in &s.blocks {
                for l in b.cover(at) {
                    let t = s.tile_of(l);
                    if t.x >= 0 && t.y >= 0 && t.x < WIDTH && t.y < HEIGHT {
                        let i = (t.y * WIDTH + t.x) as usize;
                        best[i] = (best[i].0.max(2), e, true);
                    }
                }
            }
        }
        for (i, &(pri, e, rock)) in best.iter().enumerate() {
            if !self.sea[i] {
                continue;
            }
            let (x, y) = (i as i32 % WIDTH, i as i32 / WIDTH);
            self.lava_tiles[i] = pri == 0;
            grid.set(x, y, if rock { Tile::Rock } else { Tile::Open });
            grid.set_elevation(x, y, e);
        }
    }

    /**
     * A blow lands on `id` at `at` (`weight` blows' worth): it strains, and at its third crack it
     * splits across its long side through the blow, the smaller piece sinking (a piece too small to
     * split sinks whole). Returns the new piece's id if it split.
     */
    pub fn strike(&mut self, id: u32, at: V, weight: u32) -> Option<u32> {
        let s = self.slab_mut(id)?;
        if s.sinking.is_some() || !s.solid() {
            return None;
        }
        s.strain += weight;
        let l = s.local(at);
        if s.scars.len() < 24 {
            s.scars.push(l);
        }
        if s.strain < STRAIN_PER_CRACK * CRACKS_TO_SPLIT {
            return None;
        }
        self.split(id, l)
    }

    /// Splits slab `id` across its long side at local `l`: the smaller piece sinks.
    fn split(&mut self, id: u32, l: V) -> Option<u32> {
        let new_id = self.fresh_id();
        let s = self.slab_mut(id)?;
        let along_x = s.w >= s.h;
        let len = if along_x { s.w } else { s.h };
        if len < 2 * MIN_PIECE {
            s.sinking = Some(Sink::new(SINK_SECONDS));
            s.gen += 1;
            return None;
        }
        let at = if along_x { l.x } else { l.y };
        let cut = (at.round() as i32).clamp(MIN_PIECE, len - MIN_PIECE);
        // The piece below the cut (local 0..cut) and the piece above it (cut..len).
        let low_smaller = cut * 2 <= len;
        let mut piece = s.clone();
        piece.id = new_id;
        piece.gen = 0;
        piece.strain = 0;
        piece.sinking = Some(Sink::new(SINK_SECONDS));
        // It drifts off the way it lies from the cut.
        let away = if low_smaller { -PART } else { PART };
        piece.part = if along_x { V::new(away, 0.0) } else { V::new(0.0, away) };
        let shift = |p: P, by: i32| if along_x { P::new(p.x - by, p.y) } else { P::new(p.x, p.y - by) };
        let shiftv = |v: V, by: i32| if along_x { V::new(v.x - by as f64, v.y) } else { V::new(v.x, v.y - by as f64) };
        let low = |p: P| if along_x { p.x < cut } else { p.y < cut };
        let lowv = |v: V| if along_x { v.x < cut as f64 - 0.5 } else { v.y < cut as f64 - 0.5 };
        let (keep_low, by_keep, by_piece) = if low_smaller { (false, cut, 0) } else { (true, 0, cut) };
        let mut kept = s.clone();
        let (kept_len, piece_len) = if keep_low { (cut, len - cut) } else { (len - cut, cut) };
        if along_x {
            (kept.w, piece.w) = (kept_len, piece_len);
        } else {
            (kept.h, piece.h) = (kept_len, piece_len);
        }
        kept.blocks = s.blocks.iter().filter(|(p, _)| low(*p) == keep_low).map(|&(p, b)| (shift(p, by_keep), b)).collect();
        piece.blocks = s.blocks.iter().filter(|(p, _)| low(*p) != keep_low).map(|&(p, b)| (shift(p, by_piece), b)).collect();
        kept.scars = s.scars.iter().filter(|v| lowv(**v) == keep_low).map(|&v| shiftv(v, by_keep)).collect();
        piece.scars = s.scars.iter().filter(|v| lowv(**v) != keep_low).map(|&v| shiftv(v, by_piece)).collect();
        let off = |by: i32| if along_x { V::new(by as f64, 0.0) } else { V::new(0.0, by as f64) };
        kept.home = V::new(s.home.x + off(by_keep).x, s.home.y + off(by_keep).y);
        kept.origin = V::new(s.origin.x + off(by_keep).x, s.origin.y + off(by_keep).y);
        piece.home = V::new(s.home.x + off(by_piece).x, s.home.y + off(by_piece).y);
        piece.origin = V::new(s.origin.x + off(by_piece).x, s.origin.y + off(by_piece).y);
        kept.strain = 0;
        kept.gen = s.gen + 1;
        // (Blocks cut through stay with the piece their anchor is on, as far as it reaches.)
        let (kw, kh) = (kept.w, kept.h);
        kept.blocks.retain(|&(p, b)| b.cover(p).iter().all(|q| q.x >= 0 && q.y >= 0 && q.x < kw && q.y < kh));
        let (pw, ph) = (piece.w, piece.h);
        piece.blocks.retain(|&(p, b)| b.cover(p).iter().all(|q| q.x >= 0 && q.y >= 0 && q.x < pw && q.y < ph));
        *s = kept;
        self.slabs.push(piece);
        // Whatever lay on it (his safe spot, the slain, the foes as last seen) lies on its piece now.
        let remap = |g: Ground| match g {
            Ground::Slab(sid, l) if sid == id => {
                if lowv(l) == keep_low {
                    Ground::Slab(id, shiftv(l, by_keep))
                } else {
                    Ground::Slab(new_id, shiftv(l, by_piece))
                }
            }
            g => g,
        };
        self.safe = remap(self.safe);
        for c in self.corpses.iter_mut() {
            c.at = remap(c.at);
        }
        for f in self.seen.iter_mut() {
            f.ground = f.ground.map(remap);
        }
        Some(new_id)
    }

    /// Sinking slabs go on down, `dt` seconds; those gone under are taken away (a moat slab goes back
    /// to lying sunk, to rise again at the horn's next call). Returns the ids of those gone.
    pub fn sink(&mut self, dt: f64) -> Vec<u32> {
        let mut gone = Vec::new();
        for s in self.slabs.iter_mut() {
            if let Some(k) = s.sinking.as_mut() {
                let was = k.gone();
                k.t += dt;
                if k.gone() && !was {
                    gone.push(s.id);
                }
            }
        }
        // Gone for good, but for the moat's, which lie sunk again and whole.
        self.slabs.retain(|s| !(s.sinking.is_some_and(|k| k.t >= k.over + 1.0) && s.kind == SlabKind::Field));
        for s in self.slabs.iter_mut().filter(|s| matches!(s.kind, SlabKind::Moat(_))) {
            if s.sinking.is_some_and(|k| k.t >= k.over + 1.0) {
                s.sinking = None;
                s.risen = 0.0;
                s.strain = 0;
                s.scars.clear();
                s.gen += 1;
            }
        }
        gone
    }

    /// The moat's slabs rise, `dt` seconds of their rising.
    pub fn rise(&mut self, dt: f64) {
        for s in self.slabs.iter_mut().filter(|s| matches!(s.kind, SlabKind::Moat(_)) && s.sinking.is_none()) {
            if s.risen > 0.0 && s.risen < 1.0 {
                s.risen = (s.risen + dt / RISE_SECONDS).min(1.0);
            }
        }
    }

    /// The horn calls the field: the moat's five begin to rise, the low shelf to drown.
    pub fn call(&mut self) {
        self.called = Some(0.0);
        self.crossed = false;
        self.horn_age = 0.0;
        for s in self.slabs.iter_mut() {
            match s.kind {
                SlabKind::Moat(_) if s.risen <= 0.0 => s.risen = 1e-3,
                SlabKind::Field if s.band == Band::Shelf && s.sinking.is_none() => s.sinking = Some(Sink::new(DROWN_SECONDS)),
                _ => {}
            }
        }
    }

    /// The causeway: the moat's fifth slab.
    pub fn causeway(&self) -> Option<&Slab> {
        self.slabs.iter().find(|s| s.kind == SlabKind::Moat(4))
    }

    /// Seconds the causeway has left (the full time until it has risen).
    pub fn causeway_left(&self) -> f64 {
        match self.causeway() {
            Some(s) if s.risen < 1.0 => CAUSEWAY_SECONDS,
            Some(s) => s.sinking.map(|k| (k.over - k.t).max(0.0)).unwrap_or(CAUSEWAY_SECONDS),
            None => 0.0,
        }
    }

    /// The horn's call is over, failed: the moat's slabs go down again and the lava falls.
    pub fn recall(&mut self) {
        self.called = None;
        self.crossed = false;
        for s in self.slabs.iter_mut().filter(|s| matches!(s.kind, SlabKind::Moat(_))) {
            if s.risen > 0.0 && s.sinking.is_none() {
                s.sinking = Some(Sink::new(SINK_SECONDS));
            }
        }
    }

    /// The next flash's wait: about eight seconds, four once the horn has called the field.
    pub fn flash_wait(&mut self) -> f64 {
        if self.called.is_some() && !self.opened {
            CALLED_FLASH_EVERY
        } else {
            FLASH_EVERY + self.rng.range(-1.0, 1.0)
        }
    }

    /// Somewhere for a bolt to strike, near `near`.
    pub fn bolt_near(&mut self, near: V) -> V {
        V::new(near.x + self.rng.range(-7.0, 7.0), near.y + self.rng.range(-6.0, 3.0))
    }

    /// The lightning is up (its first `FLASH_SECONDS`): 1 at the strike, fading to 0.
    pub fn flash_light(&self) -> f64 {
        if self.flash < FLASH_SECONDS {
            1.0 - self.flash / FLASH_SECONDS
        } else {
            0.0
        }
    }

    /// Where the slain can be raised now: the corpses lying on ground that still stands, with where.
    pub fn risers(&self) -> Vec<(usize, V)> {
        self.corpses.iter().enumerate().filter_map(|(i, c)| self.ground_pos(c.at).map(|p| (i, p))).collect()
    }

    /// The den for one raised at `p`: its slab or its isle.
    pub fn den_at(&self, p: V) -> Rect {
        if let Some(s) = self.slab_under(p, 0.3, None) {
            let (a, b) = (s.tile_of(P::new(0, 0)), s.tile_of(P::new(s.w - 1, s.h - 1)));
            return Rect { x0: a.x, y0: a.y, x1: b.x, y1: b.y };
        }
        let t = P::new(p.x.round() as i32, p.y.round() as i32);
        [self.camp, self.isle, self.terrace].into_iter().find(|r| r.contains(t)).unwrap_or(Rect {
            x0: t.x - 3,
            y0: t.y - 3,
            x1: t.x + 3,
            y1: t.y + 3,
        })
    }
}

impl Slab {
    #[allow(clippy::too_many_arguments)]
    fn new(id: u32, kind: SlabKind, band: Band, x0: i32, y0: i32, w: i32, h: i32, drift: Drift) -> Self {
        let home = V::new(x0 as f64, y0 as f64);
        let o = drift.offset(0.0);
        Slab {
            id,
            gen: 0,
            kind,
            band,
            w,
            h,
            home,
            drift,
            origin: V::new(home.x + o.x, home.y + o.y),
            blocks: Vec::new(),
            strain: 0,
            scars: Vec::new(),
            sinking: None,
            risen: 1.0,
            part: V::default(),
        }
    }
}

/// Every open tile of the slab can be walked to from every other.
fn connected(s: &Slab) -> bool {
    let open: Vec<P> = (0..s.h).flat_map(|y| (0..s.w).map(move |x| P::new(x, y))).filter(|&p| s.open_at(p)).collect();
    let Some(&first) = open.first() else { return false };
    let mut seen = vec![first];
    let mut queue = vec![first];
    while let Some(p) = queue.pop() {
        for d in [Dir::Up, Dir::Down, Dir::Left, Dir::Right] {
            let n = p.step(d);
            if s.open_at(n) && !seen.contains(&n) {
                seen.push(n);
                queue.push(n);
            }
        }
    }
    seen.len() == open.len()
}

/// Plants a slab: thickets of blades, a run of palisade along its north or south edge, a siege tower
/// and a war wagon on the larger ones, a banner or two, none of it cutting the slab in two.
fn plant(s: &mut Slab, rng: &mut Rng) {
    let moat = matches!(s.kind, SlabKind::Moat(_));
    let area = s.w * s.h;
    let mut tries: Vec<(P, Block)> = Vec::new();
    // A siege tower and a war wagon on the larger slabs of the field.
    if !moat && area >= 90 && rng.next() < 0.6 {
        tries.push((P::new(1 + rng.index((s.w - 3) as usize) as i32, 1 + rng.index((s.h - 3) as usize) as i32), Block::Tower));
    }
    if !moat && area >= 60 && rng.next() < 0.5 {
        tries.push((P::new(1 + rng.index((s.w - 3) as usize) as i32, 1 + rng.index((s.h - 2) as usize) as i32), Block::Wagon));
    }
    // A palisade along part of an edge (never the whole of it).
    if !moat && s.w >= 10 && rng.next() < 0.55 {
        let y = if rng.next() < 0.5 { 0 } else { s.h - 1 };
        let run = 3 + rng.index(4) as i32;
        let x0 = rng.index((s.w - run) as usize) as i32;
        for x in x0..x0 + run {
            tries.push((P::new(x, y), Block::Palisade));
        }
    }
    // Thickets of broken blades: random walks of a few tiles.
    let thickets = if moat { rng.index(2) } else { (area / 40) as usize + rng.index(2) };
    for _ in 0..thickets {
        let mut p = P::new(rng.index(s.w as usize) as i32, rng.index(s.h as usize) as i32);
        for _ in 0..(2 + rng.index(4)) {
            tries.push((p, Block::Blades));
            let d = rng.pick(&[Dir::Up, Dir::Down, Dir::Left, Dir::Right]);
            let n = p.step(d);
            if n.x >= 0 && n.y >= 0 && n.x < s.w && n.y < s.h {
                p = n;
            }
        }
    }
    for _ in 0..(1 + rng.index(2)) {
        tries.push((P::new(rng.index(s.w as usize) as i32, rng.index(s.h as usize) as i32), Block::Banner));
    }
    for (at, b) in tries {
        let cover = b.cover(at);
        if cover.iter().any(|q| q.x < 0 || q.y < 0 || q.x >= s.w || q.y >= s.h || s.blocked(*q)) {
            continue;
        }
        s.blocks.push((at, b));
        // Never so much that it cuts the slab in two, or leaves it too little room.
        let open = (0..s.h).flat_map(|y| (0..s.w).map(move |x| P::new(x, y))).filter(|&p| s.open_at(p)).count() as i32;
        if !connected(s) || open * 10 < area * 7 {
            s.blocks.pop();
        }
    }
}

/// Up to `n` posts for a pack on the slab: open tiles a tile in from its edges and clear of blocks,
/// two apart.
fn posts_on(s: &Slab, n: usize, rng: &mut Rng) -> Vec<P> {
    let mut spots: Vec<P> = (1..s.h - 1)
        .flat_map(|y| (1..s.w - 1).map(move |x| P::new(x, y)))
        .filter(|&p| [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)].iter().all(|&(dx, dy)| s.open_at(P::new(p.x + dx, p.y + dy))))
        .collect();
    rng.shuffle(&mut spots);
    let mut out: Vec<P> = Vec::new();
    for p in spots {
        if out.len() >= n {
            break;
        }
        if out.iter().all(|q| (q.x - p.x).abs() + (q.y - p.y).abs() >= 3) {
            out.push(p);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::parse_level;
    use crate::logic::levels::level_for;

    /// The two slabs (or a slab and an isle's edge) are within a leap: their spans along x share
    /// two tiles, the lava between them along y is narrow enough to clear, and the far one is no
    /// more than a band up.
    fn leapable(a: (V, V, i32), b: (V, V, i32)) -> bool {
        let share = a.1.x.min(b.1.x) - a.0.x.max(b.0.x);
        let gap = (b.0.y - a.1.y).max(a.0.y - b.1.y);
        share >= 2.0 && gap <= 1.5 && b.2 - a.2 <= 1
    }

    fn span(s: &Slab) -> (V, V, i32) {
        let (lo, hi) = s.bounds();
        (lo, hi, s.elevation())
    }

    fn rect_span(r: Rect, e: i32) -> (V, V, i32) {
        (V::new(r.x0 as f64 - 0.5, r.y0 as f64 - 0.5), V::new(r.x1 as f64 + 0.5, r.y1 as f64 + 0.5), e)
    }

    #[test]
    fn the_field_drifts_in_lanes_of_three_bands_between_the_camp_and_the_herald_s_isle() {
        let f = Battlefield::generate(1);
        let level = parse_level(&f.base.level(&level_for(1)));
        assert_eq!(level.hero_spawn, f.base.arrival);
        assert_eq!(f.base.entry.front(), f.base.arrival, "he arrives before the portal he came through");
        assert_eq!(f.base.exit.front().y, f.terrace.y0, "the far portal opens on the terrace");
        for b in Band::ALL {
            assert!(f.slabs.iter().any(|s| s.band == b && s.kind == SlabKind::Field), "{b:?}");
        }
        for s in &f.slabs {
            assert!(s.w >= 8 && s.w <= 20 || matches!(s.kind, SlabKind::Moat(_)), "slab {} is {} across", s.id, s.w);
            assert!(connected(s), "slab {} is cut in two by its blocks", s.id);
        }
        // Neighbouring lanes going north never climb more than a band: a leap reaches.
        for pair in LANES.windows(2) {
            assert!(pair[0].band.elevation() - pair[1].band.elevation() <= 1);
        }
        assert!(f.slabs.iter().filter(|s| matches!(s.kind, SlabKind::Moat(_))).all(|s| !s.solid()), "the moat lies sunk");
    }

    #[test]
    fn the_slabs_join_the_camp_to_the_horn_at_some_phase_of_their_drift() {
        let mut f = Battlefield::generate(1);
        // Step by step from the camp north: some slab of the next lane comes within a leap of one of
        // this lane's slabs at some moment of the drift.
        let lanes: Vec<Vec<u32>> = LANES
            .iter()
            .map(|l| f.slabs.iter().filter(|s| s.kind == SlabKind::Field && s.home.y as i32 == l.y0).map(|s| s.id).collect())
            .collect();
        let mut links = [false; 7];
        for _ in 0..6000 {
            f.drift(0.05);
            let spans = |ids: &Vec<u32>| ids.iter().map(|&id| span(f.slab(id).unwrap())).collect::<Vec<_>>();
            let camp = vec![rect_span(f.camp, 0)];
            let isle = vec![rect_span(f.isle, 1)];
            let mut steps = vec![camp];
            for l in lanes.iter().rev() {
                steps.push(spans(l));
            }
            steps.push(isle);
            for k in 0..7 {
                links[k] |= steps[k].iter().any(|&a| steps[k + 1].iter().any(|&b| leapable(a, b)));
            }
        }
        assert!(links.iter().all(|&l| l), "every step north is a leap at some moment: {links:?}");
        // And the moat is wider than any leap until the horn calls its slabs.
        assert!(f.isle.y0 - f.terrace.y1 - 1 > 20);
    }

    #[test]
    fn the_packs_stand_in_their_dens_on_open_ground() {
        let f = Battlefield::generate(3);
        let level = parse_level(&f.base.level(&level_for(1)));
        assert!(f.base.packs.len() >= 6, "{} packs", f.base.packs.len());
        for pk in &f.base.packs {
            assert!(!pk.foes.is_empty());
            for &(kind, _, post) in &pk.foes {
                assert!(pk.den.contains(post), "{kind:?} at {post:?}");
                assert!(level.grid.is_walkable(post.x, post.y, false), "{kind:?} on rock at {post:?}");
                assert!(super::super::natives(Sin::Wrath).iter().any(|n| n.0 == kind));
            }
        }
    }

    #[test]
    fn the_grid_follows_the_slabs_and_lava_fills_the_gaps() {
        let mut f = Battlefield::generate(1);
        let mut grid = parse_level(&f.base.level(&level_for(1))).grid;
        for _ in 0..40 {
            f.drift(0.25);
        }
        f.mark(&mut grid, None);
        for s in f.slabs.iter().filter(|s| s.solid()) {
            for ly in 0..s.h {
                for lx in 0..s.w {
                    let l = P::new(lx, ly);
                    let t = s.tile_of(l);
                    if !(0..WIDTH).contains(&t.x) {
                        continue;
                    }
                    assert_eq!(grid.elevation(t.x, t.y), s.elevation(), "slab {} at {l:?}", s.id);
                    assert_eq!(grid.get(t.x, t.y) == Tile::Rock, s.blocked(l), "slab {} at {l:?}", s.id);
                    assert!(!f.lava_at(t));
                }
            }
        }
        // Between the slabs, lava: open floor at the bottom, and no slab over it.
        let lava: Vec<P> = (5..HEIGHT).flat_map(|y| (0..WIDTH).map(move |x| P::new(x, y))).filter(|&t| f.lava_at(t)).collect();
        assert!(lava.len() > 2000, "{} tiles of lava", lava.len());
        for t in lava {
            assert!(grid.get(t.x, t.y) == Tile::Open && grid.elevation(t.x, t.y) == 0);
            assert!(f.slab_under(t.v(), 0.0, None).is_none(), "{t:?} lies under a slab");
        }
        // The moat is lava, all of it, until the horn.
        assert!((16..38).all(|y| f.lava_at(P::new(30, y))));
    }

    #[test]
    fn a_slab_split_at_its_third_crack_keeps_its_larger_piece_and_sinks_the_smaller() {
        let mut f = Battlefield::generate(1);
        let id = f.slabs.iter().find(|s| s.kind == SlabKind::Field && s.w >= 14).unwrap().id;
        let (w, h) = (f.slab(id).unwrap().w, f.slab(id).unwrap().h);
        let near_end = f.slab(id).unwrap().world(V::new(w as f64 - 3.0, 2.0));
        // A body lying near the far end, and his safe spot near the near end.
        f.corpses.push(Corpse { kind: EnemyKind::Skeleton, at: Ground::Slab(id, V::new(w as f64 - 1.0, 1.0)) });
        f.safe = Ground::Slab(id, V::new(1.0, 1.0));
        let (body, spot) = (f.slab(id).unwrap().world(V::new(w as f64 - 1.0, 1.0)), f.slab(id).unwrap().world(V::new(1.0, 1.0)));
        let mut split = None;
        for k in 0..STRAIN_PER_CRACK * CRACKS_TO_SPLIT {
            assert_eq!(f.slab(id).unwrap().cracks(), k / STRAIN_PER_CRACK);
            split = f.strike(id, near_end, 1);
        }
        let piece = split.expect("split at the third crack");
        let (a, b) = (f.slab(id).unwrap(), f.slab(piece).unwrap());
        assert_eq!(a.w + b.w, w);
        assert_eq!((a.h, b.h), (h, h));
        assert!(a.w > b.w && a.sinking.is_none() && b.sinking.is_some(), "the smaller piece sinks");
        assert_eq!(a.strain, 0);
        assert!((b.origin.x - (a.origin.x + a.w as f64)).abs() < 1e-9, "side by side where the slab was");
        // What lay on it lies on its piece, where it lay.
        assert!(matches!(f.corpses[0].at, Ground::Slab(p, _) if p == piece));
        assert!(matches!(f.safe, Ground::Slab(p, _) if p == id));
        let near = |g: Ground, at: V| crate::logic::types::dist_sq(f.ground_pos(g).unwrap(), at) < 1e-9;
        assert!(near(f.corpses[0].at, body) && near(f.safe, spot));
        for _ in 0..((SINK_SECONDS + 1.5) / 0.1) as usize {
            f.sink(0.1);
        }
        assert!(f.slab(piece).is_none(), "gone under");
    }
}
