//! The Rotting Feast: Gluttony's realm (REALMS.md §5). A banquet palace that is a gullet, running
//! north from the portal he comes through to the pylorus, the ring of teeth that is the door to
//! Beelzebub's court, in four zones: the Hall of Courses (marble, linen, long tables), the Kitchens
//! (greasy tile, cauldrons on open fires, ovens like mouths, thin crust floors), the Larder (salt
//! turning to wet ribbed flesh) and the Stomach (a round cavity of flesh round acid pools, a single
//! place set under a silver dome at its heart).
//!
//! The zones are joined by seams: bands of rock pierced by a narrow arch (short and unguarded) and
//! a wide one (a long gatehouse, garrisoned). They read as doorways, but they are valves: once he is
//! through, the teeth close behind him and the zone he left is digested. Each zone is narrower than
//! the last and its floor drifts faster toward the stomach.
//!
//! The realm's rules (hunger, Bloat, the crust, the fonts, the Last Course) are played in
//! `game::realms::feast`; their state is kept here with the layout.

use super::{deepest_stage, gate, pick_native, Base, Canvas, Pack};
use crate::logic::enemy::EnemyKind;
use crate::logic::hub::Rect;
use crate::logic::rng::Rng;
use crate::logic::sins::Sin;
use crate::logic::types::{Dir, P, V};
use std::collections::HashSet;

/// The layout's seed: the realm is the same every time.
pub const SEED: u32 = 7505;
/// The map's size in tiles.
pub const WIDTH: i32 = 41;
pub const HEIGHT: i32 = 116;
/// How many rows past a seam still wear the old zone's dressing (the hidden hand-off).
pub const HANDOFF: i32 = 2;
/// How fast each zone's floor carries him toward the stomach (tiles a second; the floors tilt 1°,
/// 3° and 6°; the Stomach's is flat).
pub const DRIFT: [f64; 4] = [0.2, 0.5, 1.0, 0.0];
/// Bloat's ceiling, and what it costs: walk speed per stack, the frames it adds to his leap's landing.
pub const MAX_BLOAT: u32 = 10;
pub const BLOAT_PACE: f64 = 0.05;
/// From this much Bloat he no longer fits the narrow arches; from this much the Kitchens' crust
/// floors break under him; from this much the gaki smell him.
pub const BLOAT_WIDE: u32 = 6;
pub const BLOAT_CRUST: u32 = 8;
pub const BLOAT_SMELL: u32 = 3;
/// The pylorus admits only a hero this lean.
pub const BLOAT_LEAN: u32 = 3;
/// Hunger: a point of maximum health every so many seconds, down to this share of it.
pub const HUNGER_EVERY: f64 = 4.0;
pub const HUNGER_FLOOR: f64 = 0.4;
/// A bite: maximum health given back, health healed, Bloat added.
pub const BITE_RESTORE: i32 = 4;
pub const BITE_HEAL: i32 = 10;
/// A purge font: Bloat taken, health taken, seconds he stands stunned.
pub const PURGE_BLOAT: u32 = 5;
pub const PURGE_HEALTH: f64 = 20.0;
pub const PURGE_STUN: f64 = 3.0;
/// How near the gaki smell him once he smells of food (tiles).
pub const SMELL_RANGE: f64 = 14.0;
/// The Last Course: the walls close over this long (and finish the job in the seconds after it);
/// the pylorus opens for `PYLORUS_OPEN` seconds at the end of every `PYLORUS_PERIOD`.
pub const CLIMAX_SECONDS: f64 = 45.0;
pub const CLIMAX_END: f64 = 48.0;
pub const PYLORUS_PERIOD: f64 = 10.0;
pub const PYLORUS_OPEN: f64 = 2.0;
/// The walls' reach from the pylorus at the start and at the end of the squeeze, and the acid's
/// depth inside them at its fullest.
pub const WALL_START: f64 = 21.0;
pub const WALL_LAST: f64 = 2.5;
pub const ACID_BAND: f64 = 3.0;
/// Acid: health it burns, every so many seconds. The swarm: its sting, how often, its radius.
pub const ACID_DAMAGE: f64 = 3.0;
pub const ACID_EVERY: f64 = 0.5;
pub const SWARM_DAMAGE: f64 = 2.0;
pub const SWARM_RADIUS: f64 = 1.7;
/// Food the gut hurls: how fast it flies, how near counts as a hit (grown by his Bloat).
pub const MORSEL_SPEED: f64 = 5.5;
pub const MORSEL_REACH: f64 = 0.45;
/// How many gaki the Stomach keeps on him in the Last Course, and how often it sends one.
pub const CLIMAX_GAKI: usize = 5;
pub const GAKI_EVERY: f64 = 7.0;
/// Where he wakes after the Stomach has digested him: hungry.
pub const WAKE_HUNGER: i32 = 30;

/// Where each zone's gaki lurk: a corner of it, away from the way through.
pub const GAKI_DENS: [Rect; 4] = [
    Rect { x0: 31, y0: 102, x1: 35, y1: 107 },
    Rect { x0: 28, y0: 76, x1: 32, y1: 81 },
    Rect { x0: 11, y0: 46, x1: 14, y1: 50 },
    Rect { x0: 9, y0: 8, x1: 14, y1: 14 },
];

/// The four zones of the gullet, from the portal to the pylorus.
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord, Hash)]
pub enum Zone {
    Hall,
    Kitchens,
    Larder,
    Stomach,
}

impl Zone {
    pub const ALL: [Zone; 4] = [Zone::Hall, Zone::Kitchens, Zone::Larder, Zone::Stomach];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        match self {
            Zone::Hall => "THE HALL OF COURSES",
            Zone::Kitchens => "THE KITCHENS",
            Zone::Larder => "THE LARDER",
            Zone::Stomach => "THE STOMACH",
        }
    }

    /// The next zone down the gullet.
    pub fn next(self) -> Zone {
        Zone::ALL[(self.index() + 1).min(3)]
    }
}

/// An archway through a seam: every tile carved for it, its gatehouse (a wide arch's den), and the
/// tiles of its mouths on the old zone's side and the new one's.
#[derive(Clone, Debug)]
pub struct Arch {
    pub wide: bool,
    pub tiles: Vec<P>,
    pub den: Option<Rect>,
    pub south: Vec<P>,
    pub north: Vec<P>,
}

impl Arch {
    pub fn contains(&self, p: P) -> bool {
        self.tiles.contains(&p)
    }
}

/// A seam between two zones: a band of rock from row `top` to row `bottom`, the zone `from` south
/// of it, its narrow and wide arches, where he rises once it is behind him, and whether its teeth
/// have closed (and how long ago).
#[derive(Clone, Debug)]
pub struct Seam {
    pub from: Zone,
    pub top: i32,
    pub bottom: i32,
    pub narrow: Arch,
    pub wide: Arch,
    pub landing: P,
    pub closed: bool,
    pub since: f64,
}

impl Seam {
    /// A tile of either arch.
    pub fn arch_at(&self, p: P) -> Option<&Arch> {
        [&self.narrow, &self.wide].into_iter().find(|a| a.contains(p))
    }
}

/// What lies on the tables and the floors to be eaten.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FoodKind {
    /// A roast fowl, glazed too glossy to be real.
    Roast,
    /// A ham on the bone.
    Ham,
    /// A pie with a lattice crust.
    Pie,
    /// A tiered cake.
    Cake,
    /// A bowl of fruit and grapes.
    Fruit,
    /// A loaf.
    Bread,
    /// A pudding in a mould.
    Pudding,
    /// Strings of sausages.
    Sausages,
    /// A wheel of cheese.
    Cheese,
    /// A gingerbread house's worth of gingerbread (the witch's bait).
    Gingerbread,
}

impl FoodKind {
    pub const ALL: [FoodKind; 10] = [
        FoodKind::Roast,
        FoodKind::Ham,
        FoodKind::Pie,
        FoodKind::Cake,
        FoodKind::Fruit,
        FoodKind::Bread,
        FoodKind::Pudding,
        FoodKind::Sausages,
        FoodKind::Cheese,
        FoodKind::Gingerbread,
    ];

    pub fn name(self) -> &'static str {
        match self {
            FoodKind::Roast => "ROAST FOWL",
            FoodKind::Ham => "HAM",
            FoodKind::Pie => "PIE",
            FoodKind::Cake => "CAKE",
            FoodKind::Fruit => "FRUIT",
            FoodKind::Bread => "BREAD",
            FoodKind::Pudding => "PUDDING",
            FoodKind::Sausages => "SAUSAGES",
            FoodKind::Cheese => "CHEESE",
            FoodKind::Gingerbread => "GINGERBREAD",
        }
    }

    /// What each zone serves.
    fn menu(zone: Zone) -> &'static [FoodKind] {
        use FoodKind::*;
        match zone {
            Zone::Hall => &[Roast, Pie, Cake, Fruit, Pudding, Bread],
            Zone::Kitchens => &[Roast, Pie, Bread, Sausages, Gingerbread, Gingerbread],
            Zone::Larder => &[Ham, Cheese, Sausages, Bread, Ham],
            Zone::Stomach => &[Roast, Ham, Fruit, Cake],
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Food {
    pub at: P,
    pub kind: FoodKind,
    pub eaten: bool,
}

/// A purge font: a basin sunk in the floor, good for one purging.
#[derive(Clone, Copy, Debug)]
pub struct Font {
    pub at: P,
    pub used: bool,
}

/// A tile of the Kitchens' thin crust floor, over the Stomach.
#[derive(Clone, Copy, Debug)]
pub struct Crust {
    pub at: P,
    pub broken: bool,
}

/// Food the gut hurls in the Last Course: where it is, where it flies, how long it has flown, and
/// whether he parried it back (then it flies at the gaki).
#[derive(Clone, Copy, Debug)]
pub struct Morsel {
    pub pos: V,
    pub vel: V,
    pub kind: FoodKind,
    pub age: f64,
    pub reflected: bool,
}

/// The Last Course under way: seconds since the dome was lifted, the food in the air, the swarm's
/// cloud, and the clocks of the gut's throws, the gaki it sends, the acid's and the swarm's bites;
/// how far the pylorus's teeth stand apart (0 to 1), and whether they last opened for him.
#[derive(Clone, Debug)]
pub struct Climax {
    pub t: f64,
    pub morsels: Vec<Morsel>,
    pub swarm: V,
    pub throw_clock: f64,
    pub gaki_clock: f64,
    pub acid_clock: f64,
    pub sting_clock: f64,
    pub teeth: f64,
    pub admitted: bool,
    pub was_open: bool,
}

impl Climax {
    pub fn new(swarm: V) -> Self {
        Self {
            t: 0.0,
            morsels: Vec::new(),
            swarm,
            throw_clock: 1.5,
            gaki_clock: 2.5,
            acid_clock: 0.0,
            sting_clock: 0.0,
            teeth: 0.0,
            admitted: false,
            was_open: false,
        }
    }

    /// How far from the pylorus the closing walls stand.
    pub fn walls(&self) -> f64 {
        wall_radius(self.t)
    }

    /// How far from the pylorus the floor is still dry.
    pub fn dry(&self) -> f64 {
        acid_radius(self.t)
    }

    /// The pylorus's teeth part now.
    pub fn open(&self) -> bool {
        pylorus_open(self.t)
    }

    /// Seconds until they next part (0 while they stand open).
    pub fn until_open(&self) -> f64 {
        if self.open() {
            0.0
        } else {
            let phase = self.t.rem_euclid(PYLORUS_PERIOD);
            PYLORUS_PERIOD - PYLORUS_OPEN - phase
        }
    }
}

/// How far from the pylorus the walls stand `t` seconds into the Last Course: closing steadily
/// over `CLIMAX_SECONDS`, then shutting altogether.
pub fn wall_radius(t: f64) -> f64 {
    if t <= CLIMAX_SECONDS {
        WALL_START + (WALL_LAST - WALL_START) * (t / CLIMAX_SECONDS)
    } else {
        WALL_LAST * (1.0 - ((t - CLIMAX_SECONDS) / (CLIMAX_END - CLIMAX_SECONDS)).min(1.0))
    }
}

/// How far from the pylorus the floor is dry: the acid floods in from the rim, a band inside the
/// walls that deepens over the first seconds.
pub fn acid_radius(t: f64) -> f64 {
    wall_radius(t) - (0.6 + t * 0.3).min(ACID_BAND)
}

/// The pylorus's teeth part for the last `PYLORUS_OPEN` seconds of every `PYLORUS_PERIOD`.
pub fn pylorus_open(t: f64) -> bool {
    t.rem_euclid(PYLORUS_PERIOD) >= PYLORUS_PERIOD - PYLORUS_OPEN
}

/// The share of his walk Bloat leaves him.
pub fn bloat_pace(bloat: u32) -> f64 {
    1.0 - BLOAT_PACE * bloat.min(MAX_BLOAT) as f64
}

#[derive(Clone, Debug)]
pub struct Feast {
    pub base: Base,
    /// The three rooms (the Hall, the Kitchens, the Larder) and the Stomach's round cavity.
    pub rooms: [Rect; 3],
    pub stomach: V,
    pub stomach_radius: f64,
    /// The seams, from the Hall's to the Stomach's.
    pub seams: Vec<Seam>,
    /// The Hall's long tables, the Kitchens' hearths (each a cauldron on a fire, 2 by 2) and
    /// chopping blocks, the Larder's rib pillars and barrels: rock tiles.
    pub tables: Vec<P>,
    pub hearths: Vec<P>,
    pub blocks: Vec<P>,
    pub pillars: Vec<P>,
    pub barrels: Vec<P>,
    /// The Kitchens' ovens (wall tiles, and the way each mouth faces), and the Stomach's acid pools.
    pub ovens: Vec<(P, Dir)>,
    pub pools: Vec<P>,
    /// The table at the Stomach's heart where the single place is set under its dome.
    pub dome: P,
    /// The point the walls close on in the Last Course: before the pylorus.
    pub squeeze: V,
    pub food: Vec<Food>,
    pub fonts: Vec<Font>,
    pub crust: Vec<Crust>,

    // ---- the state of its rules
    pub bloat: u32,
    pub hunger_clock: f64,
    /// The deepest zone he has reached.
    pub zone: Zone,
    /// The narrow arches stand barred to him (too full to fit).
    pub barred: bool,
    /// Seconds he stands stunned after a purge.
    pub stun: f64,
    /// His blows landed (the brawl's count) as last seen, and whether this L3 has burned its Bloat.
    pub hits_seen: u32,
    pub l3_burned: bool,
    /// The dome has been lifted, and the Last Course is under way.
    pub lifted: bool,
    pub climax: Option<Climax>,
    /// Seconds since the last "too full" hint.
    pub hint: f64,
}

impl Feast {
    /// The realm laid out for `tier`.
    pub fn generate(tier: usize) -> Feast {
        let sin = Sin::Gluttony;
        let mut c = Canvas::new(WIDTH, HEIGHT);
        let mut rng = Rng::new(SEED);
        let hall = Rect { x0: 5, y0: 90, x1: 35, y1: 111 };
        let kitchens = Rect { x0: 8, y0: 62, x1: 32, y1: 81 };
        let larder = Rect { x0: 11, y0: 36, x1: 30, y1: 53 };
        let stomach = V::new(20.0, 16.0);
        let stomach_radius = 11.0;
        for r in [hall, kitchens, larder] {
            c.fill(r.x0, r.y0, r.x1, r.y1, ' ');
        }
        c.disc(stomach.x, stomach.y, stomach_radius, ' ');

        // ---- the seams: (the zone south of it, its band, the narrow arch's column, the gatehouse
        // (its chamber, the column its south mouth opens at, the column its north mouth opens at))
        let plan = [
            (Zone::Hall, 82, 89, 13, Rect { x0: 18, y0: 84, x1: 31, y1: 87 }, 29, 18),
            (Zone::Kitchens, 54, 61, 27, Rect { x0: 9, y0: 56, x1: 22, y1: 59 }, 9, 20),
            (Zone::Larder, 28, 35, 17, Rect { x0: 19, y0: 30, x1: 29, y1: 33 }, 27, 21),
        ];
        let mut seams = Vec::new();
        for (from, top, bottom, xn, chamber, xs, xw) in plan {
            // The narrow arch: one tile wide, straight through, on into the zone beyond if the band
            // does not reach it.
            let mut narrow = Vec::new();
            for y in (top..=bottom).rev() {
                narrow.push(P::new(xn, y));
            }
            carve_on(&c, P::new(xn, top - 1), &mut narrow);
            for &p in &narrow {
                c.set(p, ' ');
            }
            // The wide arch: in at one end of the gatehouse, across it, and out at the other.
            let mut wide: Vec<P> = chamber.tiles().collect();
            for x in xs..xs + 3 {
                for y in chamber.y1 + 1..=bottom {
                    wide.push(P::new(x, y));
                }
            }
            for x in xw..xw + 3 {
                for y in top..chamber.y0 {
                    wide.push(P::new(x, y));
                }
                carve_on(&c, P::new(x, top - 1), &mut wide);
            }
            for &p in &wide {
                c.set(p, ' ');
            }
            let landing = P::new(xn, top - 1 - HANDOFF - 1);
            seams.push(Seam {
                from,
                top,
                bottom,
                narrow: Arch { wide: false, south: vec![P::new(xn, bottom)], north: vec![P::new(xn, top)], tiles: narrow, den: None },
                wide: Arch {
                    wide: true,
                    south: (xs..xs + 3).map(|x| P::new(x, bottom)).collect(),
                    north: (xw..xw + 3).map(|x| P::new(x, top)).collect(),
                    tiles: wide,
                    den: Some(chamber),
                },
                landing,
                closed: false,
                since: 0.0,
            });
        }

        // ---- the Hall: four long tables running north, a gap halfway along each to cross by
        let mut tables = Vec::new();
        for x in [10, 16, 24, 30] {
            for y in 93..=106 {
                if !(99..=100).contains(&y) {
                    tables.push(P::new(x, y));
                }
            }
        }
        // ---- the Kitchens: hearths, chopping blocks, ovens in the walls, the thin crust
        let mut hearths = Vec::new();
        for (x, y) in [(12, 64), (27, 65), (19, 72)] {
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                hearths.push(P::new(x + dx, y + dy));
            }
        }
        let blocks = vec![P::new(15, 77), P::new(25, 72), P::new(29, 79), P::new(11, 74)];
        let mut ovens = Vec::new();
        for y in [66, 71, 76] {
            ovens.push((P::new(kitchens.x0 - 1, y), Dir::Right));
            ovens.push((P::new(kitchens.x1 + 1, y + 1), Dir::Left));
        }
        let mut crust = Vec::new();
        for (x0, y0, x1, y1) in [(9, 69, 17, 70), (22, 75, 31, 76), (22, 79, 27, 80)] {
            for y in y0..=y1 {
                for x in x0..=x1 {
                    crust.push(Crust { at: P::new(x, y), broken: false });
                }
            }
        }
        // ---- the Larder: rib pillars down its middle, barrels against its walls
        let pillars = vec![P::new(15, 40), P::new(26, 40), P::new(15, 47), P::new(26, 47)];
        let barrels = vec![P::new(11, 44), P::new(11, 45), P::new(30, 38), P::new(30, 50), P::new(30, 51), P::new(12, 52)];
        // ---- the Stomach: acid pools, and the table at its heart
        let pools_at: [(f64, f64, f64); 3] = [(13.0, 12.0, 1.6), (27.5, 19.5, 1.7), (14.0, 21.0, 1.2)];
        let mut pools = Vec::new();
        for &(px, py, r) in &pools_at {
            let (x0, x1) = ((px - r).floor() as i32, (px + r).ceil() as i32);
            let (y0, y1) = ((py - r).floor() as i32, (py + r).ceil() as i32);
            for y in y0..=y1 {
                for x in x0..=x1 {
                    if (x as f64 - px).powi(2) + (y as f64 - py).powi(2) <= r * r {
                        pools.push(P::new(x, y));
                    }
                }
            }
        }
        let dome = P::new(stomach.x as i32, stomach.y as i32);
        for &p in tables.iter().chain(&hearths).chain(&blocks).chain(&pillars).chain(&barrels) {
            c.set(p, '#');
        }
        for &p in &pools {
            c.set(p, '~');
        }
        c.set(dome, '#');

        // ---- the portals, the arrival, the pylorus
        let entry = gate(sin, P::new(20, hall.y1 + 1), Dir::Up);
        // (Well in from the portal, so its arch does not stand between the camera and him.)
        let arrival = P::new(20, hall.y1 - 5);
        let exit = gate(sin, P::new(20, (stomach.y - stomach_radius) as i32 - 1), Dir::Down);
        c.set(arrival, 'P');
        c.set(exit.tile, 'D');
        let spawn = P::new(20, (stomach.y + stomach_radius) as i32 - 2);
        c.set(spawn, 'G');
        let squeeze = V::new(exit.front().x as f64, exit.front().y as f64 + 1.0);

        // ---- the fonts
        let fonts = vec![
            Font { at: P::new(31, 63), used: false },
            Font { at: P::new(12, 37), used: false },
            Font { at: P::new(27, 11), used: false },
        ];

        // ---- what must stay clear: the way in and out, the arches, the fonts, the crust, the landings
        let mut busy: HashSet<P> = HashSet::from([arrival, spawn, exit.front(), entry.front()]);
        // (Food keeps out of the arches altogether; a garrison may stand in its gatehouse.)
        let mut arches: HashSet<P> = HashSet::new();
        for s in &seams {
            arches.extend(s.narrow.tiles.iter().chain(&s.wide.tiles).copied());
            busy.extend(s.narrow.tiles.iter().copied());
            busy.extend(s.wide.tiles.iter().copied().filter(|&p| !s.wide.den.is_some_and(|d| d.contains(p))));
            busy.insert(s.landing);
            // The hand-off rows past each arch stay clear too, so the way through is never blocked.
            for p in s.narrow.north.iter().chain(&s.wide.north) {
                for k in 1..=HANDOFF + 1 {
                    busy.insert(P::new(p.x, p.y - k));
                }
            }
        }
        busy.extend(fonts.iter().map(|f| f.at));
        busy.extend(crust.iter().map(|k| k.at));
        for dy in -2..=2 {
            for dx in -2..=2 {
                busy.insert(P::new(dome.x + dx, dome.y + dy));
            }
        }
        let open = |c: &Canvas, p: P| c.get(p) == ' ';

        // ---- the food: everywhere, the Hall's laid along its tables
        let zone_spots = |zone: Zone, c: &Canvas| -> Vec<P> {
            (0..HEIGHT)
                .flat_map(|y| (0..WIDTH).map(move |x| P::new(x, y)))
                .filter(|&p| open(c, p) && zone_of(&seams_rows(&plan), p.y) == zone)
                .collect()
        };
        let mut food = Vec::new();
        for (zone, count) in [(Zone::Hall, 18), (Zone::Kitchens, 14), (Zone::Larder, 11), (Zone::Stomach, 9)] {
            let mut spots: Vec<P> = zone_spots(zone, &c).into_iter().filter(|p| !busy.contains(p) && !arches.contains(p)).collect();
            rng.shuffle(&mut spots);
            if zone == Zone::Hall {
                // Beside the tables first: platters set out along them.
                spots.sort_by_key(|p| !tables.iter().any(|t| (t.x - p.x).abs() == 1 && t.y == p.y));
            }
            let mut placed: Vec<P> = Vec::new();
            for p in spots {
                if placed.len() >= count {
                    break;
                }
                if placed.iter().any(|q| (q.x - p.x).abs() + (q.y - p.y).abs() < 3) {
                    continue;
                }
                placed.push(p);
                busy.insert(p);
                food.push(Food { at: p, kind: rng.pick(FoodKind::menu(zone)), eaten: false });
            }
        }

        // ---- the packs: each zone's natives and gaki, a garrison in every gatehouse; the witch's
        // kin run the Kitchens; Tepegöz or the gashadokuro keep the Stomach at the deep tiers
        let more = tier / 3;
        let mut packs = Vec::new();
        let native = |rng: &mut Rng, n: usize| -> Vec<(EnemyKind, u8)> { (0..n).map(|_| pick_native(rng, sin, tier)).collect() };
        let gaki = |n: usize| -> Vec<(EnemyKind, u8)> { vec![(EnemyKind::Gaki, 0); n] };
        // (The gaki lurk in corners off the way through, so that only the smell of food brings them.)
        let mut dens: Vec<(Rect, Vec<(EnemyKind, u8)>)> = vec![
            (Rect { x0: 5, y0: 90, x1: 15, y1: 98 }, native(&mut rng, 2 + more)),
            (Rect { x0: 25, y0: 90, x1: 35, y1: 98 }, native(&mut rng, 2 + more)),
            (GAKI_DENS[0], gaki(3)),
            (
                Rect { x0: 8, y0: 62, x1: 20, y1: 68 },
                vec![(EnemyKind::Witch, 2), (EnemyKind::Witch, 2), (EnemyKind::Minion, 2), (EnemyKind::Minion, 2)],
            ),
            (Rect { x0: 21, y0: 68, x1: 32, y1: 74 }, native(&mut rng, 2 + more)),
            (GAKI_DENS[1], gaki(3 + more)),
            (Rect { x0: 11, y0: 36, x1: 20, y1: 44 }, native(&mut rng, 2 + more)),
            (GAKI_DENS[2], gaki(4 + more)),
            (Rect { x0: 21, y0: 42, x1: 30, y1: 49 }, native(&mut rng, 2)),
            (GAKI_DENS[3], gaki(4 + more)),
        ];
        // The Stomach's tenant: a giant of the sixth stage where the tier sends them, else a native.
        let tenant =
            if deepest_stage(tier) >= 6 { vec![(rng.pick(&[EnemyKind::Tepegoz, EnemyKind::Gashadokuro]), 6)] } else { native(&mut rng, 2) };
        dens.push((Rect { x0: 12, y0: 6, x1: 28, y1: 14 }, tenant));
        for s in &seams {
            let n = 3 + usize::from(s.from != Zone::Hall) + more;
            dens.push((s.wide.den.unwrap(), native(&mut rng, n)));
        }
        for (den, foes) in dens {
            let mut spots: Vec<P> = den.tiles().filter(|&p| open(&c, p) && !busy.contains(&p)).collect();
            rng.shuffle(&mut spots);
            let mut posts: Vec<(EnemyKind, u8, P)> = Vec::new();
            for (kind, stage) in foes {
                let Some(&p) = spots.iter().find(|p| posts.iter().all(|q| (q.2.x - p.x).abs() + (q.2.y - p.y).abs() >= 2)) else { break };
                posts.push((kind, stage, p));
            }
            if !posts.is_empty() {
                packs.push(Pack { den, foes: posts });
            }
        }

        let (rows, heights) = c.finish();
        let base = Base { sin, width: WIDTH, height: HEIGHT, rows, heights, arrival, entry, exit, packs, checkpoint: arrival, tier };
        Feast {
            base,
            rooms: [hall, kitchens, larder],
            stomach,
            stomach_radius,
            seams,
            tables,
            hearths,
            blocks,
            pillars,
            barrels,
            ovens,
            pools,
            dome,
            squeeze,
            food,
            fonts,
            crust,
            bloat: 0,
            hunger_clock: 0.0,
            zone: Zone::Hall,
            barred: false,
            stun: 0.0,
            hits_seen: 0,
            l3_burned: false,
            lifted: false,
            climax: None,
            hint: 0.0,
        }
    }

    /// The zone a row lies in (a seam's band belongs to the zone south of it).
    pub fn zone_at(&self, y: f64) -> Zone {
        zone_of(&self.seams.iter().map(|s| s.top).collect::<Vec<_>>(), y.round() as i32)
    }

    /// The zone whose dressing a row wears: the first rows past a seam still wear the old one's.
    pub fn dressing_at(&self, y: f64) -> Zone {
        self.zone_at(y + HANDOFF as f64)
    }

    /// How fast the floor carries him toward the stomach at row `y`.
    pub fn drift_at(&self, y: f64) -> f64 {
        DRIFT[self.dressing_at(y).index()]
    }

    /// Inside the Stomach's cavity.
    pub fn in_stomach(&self, p: V) -> bool {
        let (dx, dy) = (p.x - self.stomach.x, p.y - self.stomach.y);
        dx * dx + dy * dy <= self.stomach_radius * self.stomach_radius
    }

    /// The seam whose band holds row `y`, if any.
    pub fn seam_at(&self, y: i32) -> Option<usize> {
        self.seams.iter().position(|s| (s.top..=s.bottom).contains(&y))
    }

    /// The crust tile at `p`, if it is one and whole.
    pub fn crust_at(&self, p: P) -> Option<usize> {
        self.crust.iter().position(|k| k.at == p && !k.broken)
    }

    /// Where he lands in the Stomach (from the Larder's seam, or through the crust).
    pub fn stomach_landing(&self) -> P {
        self.seams[2].landing
    }

    /// What he can bear to lose of his maximum health to hunger, from a cap of `full`.
    pub fn hunger_limit(full: i32) -> i32 {
        (full as f64 * (1.0 - HUNGER_FLOOR)).floor() as i32
    }
}

/// The seams' first rows, in order, from the plan.
fn seams_rows(plan: &[(Zone, i32, i32, i32, Rect, i32, i32); 3]) -> Vec<i32> {
    plan.iter().map(|s| s.1).collect()
}

/// The zone row `y` lies in, given the seams' first rows from the Hall's to the Stomach's.
fn zone_of(tops: &[i32], y: i32) -> Zone {
    for (k, &top) in tops.iter().enumerate() {
        if y >= top {
            return Zone::ALL[k];
        }
    }
    Zone::Stomach
}

/// Carves on north from `from` until it meets floor already carved (or the map's edge), into `out`.
fn carve_on(c: &Canvas, from: P, out: &mut Vec<P>) {
    let mut p = from;
    while c.inside(p) && p.y > 1 && c.get(p) == '#' {
        out.push(p);
        p = P::new(p.x, p.y - 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::parse_level;
    use crate::logic::levels::level_for;
    use crate::logic::pathing::reachable;
    use crate::logic::types::Tile;

    #[test]
    fn the_gullet_runs_north_in_four_zones_each_narrower_than_the_last() {
        let f = Feast::generate(1);
        let [hall, kitchens, larder] = f.rooms;
        // In order, from the portal to the pylorus: each zone north of the one before.
        assert!(hall.y0 > kitchens.y1 && kitchens.y0 > larder.y1 && larder.y0 as f64 > f.stomach.y + f.stomach_radius);
        let widths = [hall.x1 - hall.x0 + 1, kitchens.x1 - kitchens.x0 + 1, larder.x1 - larder.x0 + 1];
        assert!(widths[1] as f64 <= widths[0] as f64 * 0.85 && widths[2] as f64 <= widths[1] as f64 * 0.85, "{widths:?}");
        assert!((f.stomach_radius * 2.0) < widths[0] as f64, "the stomach is a cavity, not another hall");
        // The zones by row, and their drift quickening toward the stomach.
        assert_eq!(f.zone_at(f.base.arrival.y as f64), Zone::Hall);
        assert_eq!(f.zone_at(70.0), Zone::Kitchens);
        assert_eq!(f.zone_at(45.0), Zone::Larder);
        assert_eq!(f.zone_at(f.stomach.y), Zone::Stomach);
        const { assert!(DRIFT[0] < DRIFT[1] && DRIFT[1] < DRIFT[2]) };
        // The seams in order, their bands between the zones.
        assert_eq!(f.seams.iter().map(|s| s.from).collect::<Vec<_>>(), vec![Zone::Hall, Zone::Kitchens, Zone::Larder]);
        // The first rows past a seam wear the old zone's dressing.
        let s = &f.seams[0];
        assert_eq!(f.dressing_at((s.top - 1) as f64), Zone::Hall);
        assert_eq!(f.dressing_at((s.top - HANDOFF - 1) as f64), Zone::Kitchens);
        // The pylorus is the level's far portal, on the Stomach's north rim.
        assert!(f.in_stomach(f.base.exit.front().v()));
        assert_eq!(f.base.exit.facing, Dir::Down);
    }

    #[test]
    fn every_seam_has_a_short_narrow_arch_and_a_long_garrisoned_wide_one() {
        let f = Feast::generate(1);
        let level = parse_level(&f.base.level(&level_for(1)));
        for (i, s) in f.seams.iter().enumerate() {
            assert!(!s.narrow.wide && s.wide.wide);
            assert!(s.narrow.tiles.len() * 2 <= s.wide.tiles.len(), "seam {i}: the wide arch is the long way round");
            assert!(s.narrow.tiles.iter().all(|p| level.grid.is_walkable(p.x, p.y, false)));
            assert!(s.narrow.tiles.iter().all(|p| !s.wide.contains(*p)), "seam {i}: two separate arches");
            // Only the arches pierce the band.
            for y in s.top..=s.bottom {
                for x in 0..WIDTH {
                    let p = P::new(x, y);
                    if level.grid.is_walkable(x, y, false) {
                        assert!(s.arch_at(p).is_some(), "seam {i}: {p:?} is open but in no arch");
                    }
                }
            }
            // The wide arch's gatehouse is garrisoned; the narrow arch is not.
            let den = s.wide.den.unwrap();
            assert!(f.base.packs.iter().any(|pk| pk.den == den && pk.foes.len() >= 3), "seam {i}: garrisoned");
            assert!(f.base.packs.iter().all(|pk| pk.foes.iter().all(|foe| !s.narrow.contains(foe.2))));
            // Each arch on its own leads through: bar the other and the way still goes.
            for (shut, open) in [(&s.narrow, &s.wide), (&s.wide, &s.narrow)] {
                let mut g = level.grid.clone();
                for p in &shut.tiles {
                    g.set(p.x, p.y, Tile::Rock);
                }
                let seen = reachable(&g, open.south[0], false, false);
                assert!(seen.contains(&s.landing), "seam {i}: through the {} arch alone", if open.wide { "wide" } else { "narrow" });
            }
        }
    }

    #[test]
    fn the_dome_can_be_reached_from_the_arrival_and_the_food_and_packs_lie_where_they_should() {
        let f = Feast::generate(1);
        let level = parse_level(&f.base.level(&level_for(1)));
        let seen = reachable(&level.grid, f.base.arrival, false, false);
        let near_dome = [Dir::Up, Dir::Down, Dir::Left, Dir::Right].iter().any(|&d| seen.contains(&f.dome.step(d)));
        assert!(near_dome, "the way to the Last Course");
        assert!(seen.contains(&f.base.exit.front()), "and to the pylorus");
        for food in &f.food {
            assert!(seen.contains(&food.at), "{:?} at {:?} out of reach", food.kind, food.at);
            assert!(f.crust_at(food.at).is_none() && f.seams.iter().all(|s| s.arch_at(food.at).is_none()));
        }
        for z in Zone::ALL {
            assert!(f.food.iter().filter(|k| f.zone_at(k.at.y as f64) == z).count() >= 8, "{z:?} is laid with food");
        }
        for font in &f.fonts {
            assert!(seen.contains(&font.at));
        }
        assert_eq!(f.fonts.iter().filter(|k| f.in_stomach(k.at.v())).count(), 1, "the Stomach's one purge font");
        assert!(f.crust.iter().all(|k| f.zone_at(k.at.y as f64) == Zone::Kitchens && level.grid.is_walkable(k.at.x, k.at.y, false)));
        for pk in &f.base.packs {
            for &(kind, _, post) in &pk.foes {
                assert!(pk.den.contains(post) && seen.contains(&post), "{kind:?} at {post:?}");
            }
        }
        let kinds: Vec<EnemyKind> = f.base.packs.iter().flat_map(|p| p.foes.iter().map(|k| k.0)).collect();
        assert!(kinds.iter().filter(|&&k| k == EnemyKind::Gaki).count() >= 12, "the gaki in every zone");
        assert!(kinds.contains(&EnemyKind::Witch), "the witch's kin run the Kitchens");
        let deep = Feast::generate(6);
        let kinds: Vec<EnemyKind> = deep.base.packs.iter().flat_map(|p| p.foes.iter().map(|k| k.0)).collect();
        assert!(kinds.iter().any(|k| matches!(k, EnemyKind::Tepegoz | EnemyKind::Gashadokuro)), "the Stomach's tenant");
    }

    #[test]
    fn the_last_course_closes_its_walls_floods_its_rim_and_opens_its_teeth_two_seconds_in_ten() {
        assert!((wall_radius(0.0) - WALL_START).abs() < 1e-9);
        assert!(wall_radius(CLIMAX_SECONDS * 0.5) < WALL_START && wall_radius(CLIMAX_SECONDS) > 0.0);
        assert_eq!(wall_radius(CLIMAX_END), 0.0);
        for t in [0.0, 10.0, 30.0] {
            assert!(acid_radius(t) < wall_radius(t), "the acid lies inside the walls");
        }
        let open: Vec<bool> = (0..100).map(|k| pylorus_open(k as f64 * 0.1)).collect();
        assert_eq!(open.iter().filter(|&&o| o).count(), 20, "two seconds in every ten");
        assert!(!pylorus_open(7.9) && pylorus_open(8.1) && !pylorus_open(10.1));
        let mut c = Climax::new(V::new(0.0, 0.0));
        c.t = 3.0;
        assert!((c.until_open() - 5.0).abs() < 1e-9);
        assert!((bloat_pace(10) - 0.5).abs() < 1e-9 && bloat_pace(0) == 1.0);
        assert_eq!(Feast::hunger_limit(100), 60);
    }
}
