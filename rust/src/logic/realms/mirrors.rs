//! The Hall of Mirrors: Pride's realm (REALMS.md §1), the Spire of the Morning Star. Seven
//! galleries climb toward a white sun. The first six are octagonal rings round an open
//! light-well; the seventh is the throne, a round mirror floor before one vast mirror.
//!
//! There are no stairs. Each gallery below the throne has three full-height mirror-doors in its
//! north wall, and walking into one carries him, with no fade, out of a mirror of another
//! gallery:
//! - one door is true and climbs to the next gallery;
//! - the other two are vain and loop him one or two galleries down (from the first gallery, back
//!   to the portal he came in by);
//! - no door works both ways: the mirror he steps out of never leads back where he came from.
//!
//! A statue of him stands before every mirror-door, and relay mirrors on stands by the well
//! throw the Morning Star's light across the floor as burning beams. The court's portal is in the
//! first gallery, beside the one he came in by, and stays sealed until the Peerless falls.
//!
//! The galleries lie side by side on one map, rock between them; the view stacks them into a
//! spire. The realm's rules (the doors, Glory, the beams, the echo) are `game::realms::mirrors`;
//! the state they keep lives here.

use std::collections::VecDeque;

use super::{gate, pick_native, Base, Canvas, Pack};
use crate::logic::combat::fsm::State;
use crate::logic::combat::MoveId;
use crate::logic::hub::Rect;
use crate::logic::rng::Rng;
use crate::logic::sins::Sin;
use crate::logic::types::{Dir, P, V};

/// The layout's seed: the realm is the same every time.
pub const SEED: u32 = 7101;
/// Seven galleries; the seventh (`THRONE`) is the top of the spire.
pub const GALLERIES: usize = 7;
pub const THRONE: usize = 6;
/// Mirror-doors in each gallery below the throne, and where each stands along the north wall
/// (tiles east of the gallery's centre). Each is three tiles wide.
pub const DOORS: usize = 3;
pub const DOOR_DX: [i32; DOORS] = [-5, 0, 5];
/// A gallery's floor: |dx| and |dy| at most `OUTER` and |dx| + |dy| at most `CUT` from its centre
/// (an octagon 23 tiles across); the light-well in its middle the same with `WELL` and `WELL_CUT`.
pub const OUTER: i32 = 11;
pub const CUT: i32 = 18;
pub const WELL: i32 = 4;
pub const WELL_CUT: i32 = 6;
/// The plan: a gallery to each cell, four cells to a row, rock between.
pub const CELL: i32 = 34;
pub const MARGIN: i32 = 2;
const COLS: i32 = 4;
/// The row of a gallery's mirror-doors, north of its centre.
pub const DOOR_ROW: i32 = OUTER + 1;
/// The throne: its floor a disc this wide round its centre, the dais in the middle, a landing
/// north of the disc before the vast mirror (`THRONE_DOOR_ROW` north of the centre, 13 tiles wide).
pub const THRONE_RADIUS: f64 = 10.0;
pub const DAIS_RADIUS: f64 = 1.6;
pub const THRONE_DOOR_ROW: i32 = 15;
pub const LANDING_HALF: i32 = 6;
/// The rings of the throne's floor that fall, one for each blow the Peerless lands: everything
/// farther from the centre than each of these.
pub const RINGS: [f64; 3] = [8.0, 6.0, 4.0];
/// Blows a statue of him takes before it shatters.
pub const STATUE_BLOWS: u8 = 3;
/// Glory: his strike power per stack; from `GLORY_TRACKS` stacks the beams track him and the
/// tengu press harder; from `GLORY_FAKES` the vain mirrors fake the cracks.
pub const GLORY_MIGHT: f64 = 0.08;
pub const GLORY_TRACKS: u32 = 3;
pub const GLORY_FAKES: u32 = 5;
/// Seconds a Splinter swaps the faces of the true and vain mirrors.
pub const SPLINTER_SECONDS: f64 = 6.0;
/// The beams: one every `BEAM_EVERY` seconds in his gallery, telegraphed for `BEAM_TELL`, burning
/// for `BEAM_BURN`. Standing in one (not leaping) burns `BEAM_DAMAGE` at once, then `BEAM_BITE`
/// every `BEAM_BITE_EVERY` seconds.
pub const BEAM_EVERY: f64 = 4.5;
pub const BEAM_TELL: f64 = 1.1;
pub const BEAM_BURN: f64 = 1.2;
pub const BEAM_WIDTH: f64 = 0.42;
pub const BEAM_DAMAGE: f64 = 10.0;
pub const BEAM_BITE: f64 = 5.0;
pub const BEAM_BITE_EVERY: f64 = 0.4;
/// A tracking beam turns after him for this long into its telegraph, then holds.
pub const BEAM_TRACK: f64 = 0.6;
/// The echo: frames behind him, and what each Glory stack adds to its damage and health.
pub const ECHO_DELAY: u32 = 24;
pub const ECHO_GLORY: f64 = 0.1;
/// The reflection leans this far out of the glass toward him, and comes no nearer than `ECHO_APART`.
pub const ECHO_CLOSE: f64 = 1.0;
pub const ECHO_APART: f64 = 0.8;
/// Frames it takes to rise out of the floor, and that it stands posing when he drinks.
pub const RISE_FRAMES: u16 = 40;
pub const POSE_FRAMES: u16 = 40;
/// Seconds of the fall down the spire's length.
pub const FALL_SECONDS: f64 = 1.8;
/// Frames of his history kept for the echo.
const SNAPS_KEPT: usize = 96;

/// One gallery: its centre, its floor's bounds (its pack's den), its mirror-doors, the relay
/// mirrors that throw the beams (stands by the well, rock tiles, each with the way it throws), and
/// the plinths where statues of his glory rise (rock tiles along its walls).
#[derive(Clone, Debug)]
pub struct Gallery {
    pub index: usize,
    pub center: P,
    pub floor: Rect,
    pub doors: Vec<usize>,
    pub relays: Vec<(P, Dir)>,
    pub plinths: Vec<P>,
}

/// A mirror-door: its recess (walkable tiles, west to east, the glass along their north edge),
/// whether it is true, the gallery it leads to and the door he steps out of there (none: the
/// arrival by the portal), and the statue of him before it (a rock tile until smashed) with the
/// blows it has left.
#[derive(Clone, Debug)]
pub struct Door {
    pub gallery: usize,
    pub tiles: Vec<P>,
    pub truth: bool,
    pub to: usize,
    pub out: Option<usize>,
    pub statue: Option<P>,
    pub statue_hp: u8,
}

impl Door {
    /// The middle tile of its recess.
    pub fn center(&self) -> P {
        self.tiles[self.tiles.len() / 2]
    }

    /// Where its glass stands: the north edge of the recess (world z, in tiles).
    pub fn glass(&self) -> f64 {
        self.center().y as f64 - 0.5
    }

    /// Where he steps out of it, coming through in column `col` (-1 west, 0, 1 east of its middle):
    /// before the glass, beside the statue while it stands.
    pub fn front(&self, col: i32) -> P {
        let c = self.center();
        let col = if col == 0 && self.statue_hp > 0 && self.statue.is_some() { -1 } else { col.clamp(-1, 1) };
        P::new(c.x + col, c.y + 1)
    }
}

/// His visit to the gallery he is in: unhurt so far, its clearing already counted, and the
/// wounds and bottles he had when it began (to tell a hit or a drink since).
#[derive(Clone, Copy, Debug, Default)]
pub struct Visit {
    pub clean: bool,
    pub awarded: bool,
    pub hurt: u32,
    pub belt: usize,
}

/// A beam of the Morning Star thrown by a relay mirror: from where to where, how long since its
/// telegraph began, whether it has burned him yet and when it bites again.
#[derive(Clone, Copy, Debug)]
pub struct Beam {
    pub relay: usize,
    pub from: V,
    pub to: V,
    pub age: f64,
    pub touched: bool,
    pub bite: f64,
}

impl Beam {
    /// Burning (past its telegraph).
    pub fn burning(&self) -> bool {
        self.age >= BEAM_TELL && self.age < BEAM_TELL + BEAM_BURN
    }

    /// Does it cover a body of `radius` at `p`?
    pub fn covers(&self, p: V, radius: f64) -> bool {
        let (dx, dy) = (self.to.x - self.from.x, self.to.y - self.from.y);
        let l2 = dx * dx + dy * dy;
        let t = if l2 > 1e-9 { (((p.x - self.from.x) * dx + (p.y - self.from.y) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
        let (cx, cy) = (self.from.x + dx * t, self.from.y + dy * t);
        (p.x - cx).hypot(p.y - cy) <= BEAM_WIDTH + radius
    }
}

/// What he did on one frame of the fight: where he stood and faced, the move he began, whether he
/// leapt, whether his guard was held, how high he was.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Snap {
    pub frame: u32,
    pub pos: V,
    pub aim: V,
    pub began: Option<MoveId>,
    pub evade: bool,
    pub guard: bool,
    pub altitude: f64,
}

/// The echo: his recent frames, his fighter's state on the last one, the last frame seen and the
/// last replayed, his swings counted; and the Peerless's fight (under way, frames left rising,
/// the wounds he had taken when it began).
#[derive(Clone, Debug, Default)]
pub struct Echo {
    pub snaps: VecDeque<Snap>,
    pub last: Option<State>,
    pub frame: u32,
    pub played: u32,
    pub swing: u32,
    pub fight: bool,
    pub rise: u32,
    pub hurt: u32,
}

impl Echo {
    /// His frame `f`, as recorded.
    pub fn snap(&self, f: u32) -> Option<&Snap> {
        self.snaps.iter().rev().find(|s| s.frame == f)
    }

    /// Remembers a frame, forgetting the oldest.
    pub fn record(&mut self, s: Snap) {
        self.snaps.push_back(s);
        while self.snaps.len() > SNAPS_KEPT {
            self.snaps.pop_front();
        }
    }
}

/// The fall down the spire: seconds into it, and whether it is the fall of the victor (to the
/// court's portal) or of the beaten (to the first gallery).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fall {
    pub t: f64,
    pub win: bool,
}

#[derive(Clone, Debug)]
pub struct Mirrors {
    pub base: Base,
    pub galleries: Vec<Gallery>,
    pub doors: Vec<Door>,
    /// The throne: the centre of its floor, the dais, every tile of its floor, and where he
    /// stands when he steps out of the vast mirror.
    pub throne: P,
    pub dais: Vec<P>,
    pub throne_floor: Vec<P>,
    pub landing: P,
    /// The gallery he is in, the vain loops he has taken (the spire grows by one for each), and
    /// every warp (the view snaps its camera on each).
    pub gallery: usize,
    pub loops: u32,
    pub warps: u32,
    /// Glory stacks, the Splinter's seconds left, and his visit to this gallery.
    pub glory: u32,
    pub splinter: f64,
    pub visit: Visit,
    /// The beam in his gallery, seconds since the last, and whose turn it is.
    pub beam: Option<Beam>,
    pub beam_clock: f64,
    pub beam_turn: usize,
    /// The echo and the Peerless's fight; the throne's rings fallen and seconds since the last.
    pub echo: Echo,
    pub rings: u8,
    pub ring_age: f64,
    /// The fall down the spire, while it lasts; the Peerless shattered; his last statue blow's swing.
    pub fall: Option<Fall>,
    pub won: bool,
    pub statue_swing: u32,
    /// The Morning Star has spoken on this visit to the throne.
    pub spoken: bool,
}

/// Is (dx, dy) on a gallery's octagon?
pub fn on_octagon(dx: i32, dy: i32, outer: i32, cut: i32) -> bool {
    dx.abs() <= outer && dy.abs() <= outer && dx.abs() + dy.abs() <= cut
}

impl Mirrors {
    /// The realm laid out for `tier`.
    pub fn generate(tier: usize) -> Mirrors {
        let mut rng = Rng::new(SEED);
        let (w, h) = (MARGIN * 2 + COLS * CELL, MARGIN * 2 + 2 * CELL);
        let mut c = Canvas::new(w, h);
        let centre = |g: usize| {
            let (col, row) = (g as i32 % COLS, g as i32 / COLS);
            P::new(MARGIN + col * CELL + CELL / 2, MARGIN + row * CELL + CELL / 2 + 1)
        };

        // ---- the six galleries: an octagonal ring round the well, three doors in the north wall
        let mut galleries = Vec::new();
        let mut doors = Vec::new();
        for g in 0..THRONE {
            let o = centre(g);
            for dy in -OUTER..=OUTER {
                for dx in -OUTER..=OUTER {
                    if on_octagon(dx, dy, OUTER, CUT) {
                        let well = on_octagon(dx, dy, WELL, WELL_CUT);
                        c.set(P::new(o.x + dx, o.y + dy), if well { '~' } else { ' ' });
                    }
                }
            }
            let mut ids = Vec::new();
            for dx in DOOR_DX {
                let tiles: Vec<P> = (-1..=1).map(|k| P::new(o.x + dx + k, o.y - DOOR_ROW)).collect();
                for &t in &tiles {
                    c.set(t, ' ');
                }
                let statue = P::new(o.x + dx, o.y - DOOR_ROW + 1);
                c.set(statue, '#');
                ids.push(doors.len());
                doors.push(Door { gallery: g, tiles, truth: false, to: 0, out: None, statue: Some(statue), statue_hp: STATUE_BLOWS });
            }
            let relays = vec![
                (P::new(o.x, o.y + WELL + 1), Dir::Down),
                (P::new(o.x + WELL + 1, o.y), Dir::Right),
                (P::new(o.x - WELL - 1, o.y), Dir::Left),
                (P::new(o.x, o.y - WELL - 1), Dir::Up),
            ];
            let plinths = vec![
                P::new(o.x - 6, o.y + OUTER),
                P::new(o.x + 6, o.y + OUTER),
                P::new(o.x - 2, o.y + OUTER),
                P::new(o.x + 2, o.y + OUTER),
                P::new(o.x - OUTER, o.y + 4),
                P::new(o.x + OUTER, o.y + 4),
                P::new(o.x - OUTER, o.y - 4),
                P::new(o.x + OUTER, o.y - 4),
            ];
            for &(p, _) in &relays {
                c.set(p, '#');
            }
            for &p in &plinths {
                c.set(p, '#');
            }
            let floor = Rect { x0: o.x - OUTER, y0: o.y - OUTER, x1: o.x + OUTER, y1: o.y + OUTER };
            galleries.push(Gallery { index: g, center: o, floor, doors: ids, relays, plinths });
        }

        // ---- the throne: a disc round the dais, a landing before the vast mirror north of it
        let t = centre(THRONE);
        let throne = P::new(t.x, t.y + 1);
        let mut throne_floor = Vec::new();
        let r = THRONE_RADIUS;
        for dy in -THRONE_DOOR_ROW + 1..=r as i32 {
            for dx in -(r as i32)..=r as i32 {
                let disc = ((dx * dx + dy * dy) as f64).sqrt() <= r;
                let landing = dx.abs() <= LANDING_HALF && (-THRONE_DOOR_ROW + 1..=-(r as i32) + 2).contains(&dy);
                if disc || landing {
                    let p = P::new(throne.x + dx, throne.y + dy);
                    c.set(p, ' ');
                    throne_floor.push(p);
                }
            }
        }
        let dais: Vec<P> = throne_floor
            .iter()
            .copied()
            .filter(|p| (((p.x - throne.x).pow(2) + (p.y - throne.y).pow(2)) as f64).sqrt() <= DAIS_RADIUS)
            .collect();
        let vast: Vec<P> = (-LANDING_HALF..=LANDING_HALF).map(|dx| P::new(throne.x + dx, throne.y - THRONE_DOOR_ROW)).collect();
        for &p in &vast {
            c.set(p, ' ');
        }
        let landing = P::new(throne.x, throne.y - THRONE_DOOR_ROW + 1);
        let throne_rect =
            Rect { x0: throne.x - r as i32, y0: throne.y - THRONE_DOOR_ROW, x1: throne.x + r as i32, y1: throne.y + r as i32 };
        let throne_door = doors.len();
        doors.push(Door { gallery: THRONE, tiles: vast, truth: false, to: THRONE - 2, out: None, statue: None, statue_hp: 0 });
        galleries.push(Gallery {
            index: THRONE,
            center: throne,
            floor: throne_rect,
            doors: vec![throne_door],
            relays: Vec::new(),
            plinths: Vec::new(),
        });
        c.set(throne, 'G');

        // ---- the mirrors: one true door to each gallery, climbing; the rest vain, looping down
        for g in 0..THRONE {
            let truth = rng.index(DOORS);
            for k in 0..DOORS {
                let d = &mut doors[galleries[g].doors[k]];
                d.truth = k == truth;
                d.to = if d.truth { g + 1 } else { g.saturating_sub(1 + rng.index(2)) };
            }
        }
        // He steps out of the destination's door in the same place along its wall, unless that
        // mirror would lead straight back; then out of another (there is always one that does not).
        for i in 0..doors.len() {
            let (g, to) = (doors[i].gallery, doors[i].to);
            if to == g {
                continue; // the first gallery's vain doors: back to the portal
            }
            let k = galleries[g].doors.iter().position(|&d| d == i).unwrap_or(0);
            let there = &galleries[to].doors;
            let same = there.get(k).copied();
            let order: Vec<usize> = same.into_iter().chain(there.iter().copied().filter(|&d| Some(d) != same)).collect();
            let out = order.into_iter().find(|&o| doors[o].to != g);
            doors[i].out = out;
        }

        // ---- the portals: in by the first gallery's west wall, out by its east wall
        let o = galleries[0].center;
        let entry = gate(Sin::Pride, P::new(o.x - OUTER - 1, o.y), Dir::Right);
        let exit = gate(Sin::Pride, P::new(o.x + OUTER + 1, o.y), Dir::Left);
        let arrival = P::new(o.x - OUTER + 1, o.y);
        c.set(arrival, 'P');
        c.set(exit.tile, 'D');

        // ---- the natives: a pack to every gallery below the throne, more the higher it is, its den
        // the south of the ring (it stirs as he comes round the well, not as he steps out of a
        // mirror), clear of the beams' stands and well clear of the portal he arrives by
        let mut packs = Vec::new();
        for g in 0..THRONE {
            let o = galleries[g].center;
            let count = [2, 3, 3, 4, 4, 5][g];
            let den = Rect { x0: o.x - OUTER, y0: o.y + 2, x1: o.x + OUTER, y1: o.y + OUTER };
            let mut spots: Vec<P> = Vec::new();
            for p in den.tiles() {
                let far = ((p.x - arrival.x).pow(2) + (p.y - arrival.y).pow(2)) as f64 > 64.0;
                if c.get(p) == ' ' && p.y > den.y0 && p.y < den.y1 && far {
                    spots.push(p);
                }
            }
            rng.shuffle(&mut spots);
            let mut foes = Vec::new();
            for p in spots {
                if foes.len() >= count {
                    break;
                }
                if foes.iter().any(|f: &(_, _, P)| (f.2.x - p.x).abs() + (f.2.y - p.y).abs() < 3) {
                    continue;
                }
                let (kind, stage) = pick_native(&mut rng, Sin::Pride, tier);
                foes.push((kind, stage, p));
            }
            packs.push(Pack { den, foes });
        }

        let (rows, heights) = c.finish();
        let base = Base { sin: Sin::Pride, width: w, height: h, rows, heights, arrival, entry, exit, packs, checkpoint: arrival, tier };
        Mirrors {
            base,
            galleries,
            doors,
            throne,
            dais,
            throne_floor,
            landing,
            gallery: 0,
            loops: 0,
            warps: 0,
            glory: 0,
            splinter: 0.0,
            visit: Visit::default(),
            beam: None,
            beam_clock: 0.0,
            beam_turn: 0,
            echo: Echo::default(),
            rings: 0,
            ring_age: 99.0,
            fall: None,
            won: false,
            statue_swing: 0,
            spoken: false,
        }
    }

    /// The gallery a tile lies in.
    pub fn gallery_at(&self, p: P) -> Option<usize> {
        self.galleries.iter().position(|g| g.floor.contains(p) || g.doors.iter().any(|&d| self.doors[d].tiles.contains(&p)))
    }

    /// The mirror-door whose recess holds `p`.
    pub fn door_at(&self, p: P) -> Option<usize> {
        self.doors.iter().position(|d| d.tiles.contains(&p))
    }

    /// The galleries the light-well shows above him: those truly above, and one more for every
    /// vain loop (the spire grows as he is flattered).
    pub fn shown_above(&self) -> u32 {
        (THRONE.saturating_sub(self.gallery)) as u32 + self.loops
    }

    /// The face a mirror shows: true (his real look, the cracks) or vain (crowned, gilded, whole).
    /// A Splinter swaps them.
    pub fn shows_true(&self, door: usize) -> bool {
        self.doors[door].truth != (self.splinter > 0.0)
    }

    /// Does its glass show cracks? The true face does; from `GLORY_FAKES` stacks, the vain fake them.
    pub fn cracked(&self, door: usize) -> bool {
        self.shows_true(door) || self.glory >= GLORY_FAKES
    }

    /// His strike power's multiple from Glory.
    pub fn might(&self) -> f64 {
        1.0 + GLORY_MIGHT * self.glory as f64
    }

    /// `p` reflected across the throne.
    pub fn across_throne(&self, p: V) -> V {
        let c = self.throne.v();
        V::new(2.0 * c.x - p.x, 2.0 * c.y - p.y)
    }

    /// Which of the throne's rings a tile of its floor falls with (0 first; 3: only when the whole
    /// floor gives way).
    pub fn ring_of(&self, p: P) -> u8 {
        let d = (((p.x - self.throne.x).pow(2) + (p.y - self.throne.y).pow(2)) as f64).sqrt();
        RINGS.iter().position(|&r| d > r).unwrap_or(RINGS.len()) as u8
    }

    /// Standing on the dais.
    pub fn on_dais(&self, p: P) -> bool {
        self.dais.contains(&p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::parse_level;
    use crate::logic::levels::level_for;
    use crate::logic::pathing::reachable;
    use std::collections::HashSet;

    #[test]
    fn seven_galleries_each_below_the_throne_with_one_true_mirror_of_three() {
        let m = Mirrors::generate(1);
        assert_eq!(m.galleries.len(), GALLERIES);
        for g in &m.galleries[..THRONE] {
            assert_eq!(g.doors.len(), DOORS);
            let truths: Vec<&Door> = g.doors.iter().map(|&d| &m.doors[d]).filter(|d| d.truth).collect();
            assert_eq!(truths.len(), 1, "gallery {}", g.index);
            assert_eq!(truths[0].to, g.index + 1, "the true mirror climbs");
            for &d in &g.doors {
                let door = &m.doors[d];
                if !door.truth {
                    assert!(door.to < g.index || (g.index == 0 && door.to == 0), "a vain mirror loops down");
                    assert!(g.index.saturating_sub(door.to) <= 2, "one or two galleries down");
                }
                assert!(door.statue.is_some(), "a statue of him before every door");
            }
        }
        let vast = &m.doors[m.galleries[THRONE].doors[0]];
        assert!(!vast.truth && vast.to < THRONE, "the vast mirror's back face is vain too");
        assert_eq!(vast.tiles.len(), 2 * LANDING_HALF as usize + 1);
    }

    #[test]
    fn no_mirror_works_both_ways_and_the_mirrors_reach_the_throne() {
        let m = Mirrors::generate(1);
        for (i, d) in m.doors.iter().enumerate() {
            match d.out {
                Some(o) => {
                    assert_eq!(m.doors[o].gallery, d.to, "door {i} steps out in its own destination");
                    assert_ne!(m.doors[o].to, d.gallery, "door {i}: the mirror he steps out of never leads back");
                    assert_ne!(d.to, d.gallery);
                }
                None => assert_eq!((d.gallery, d.to), (0, 0), "only the first gallery's vain doors lead to the portal"),
            }
        }
        // The mirror graph: from the first gallery every gallery can be reached, and the true
        // mirrors alone climb to the throne.
        let mut seen = HashSet::from([0usize]);
        let mut queue = vec![0usize];
        while let Some(g) = queue.pop() {
            for &d in &m.galleries[g].doors {
                if seen.insert(m.doors[d].to) {
                    queue.push(m.doors[d].to);
                }
            }
        }
        assert_eq!(seen.len(), GALLERIES);
        let mut g = 0;
        for _ in 0..THRONE {
            let d = m.galleries[g].doors.iter().copied().find(|&d| m.doors[d].truth).unwrap();
            g = m.doors[d].to;
        }
        assert_eq!(g, THRONE, "the true path climbs from the arrival to the top");
    }

    #[test]
    fn every_gallery_is_walkable_round_its_well_and_its_doors_and_dais_can_be_reached() {
        let m = Mirrors::generate(3);
        let level = parse_level(&m.base.level(&level_for(1)));
        let grid = &level.grid;
        assert_eq!(level.hero_spawn, m.base.arrival);
        assert_eq!(m.gallery_at(m.base.arrival), Some(0));
        for g in &m.galleries {
            let from = if g.index == THRONE { m.landing } else { m.doors[g.doors[0]].front(-1) };
            assert!(grid.is_walkable(from.x, from.y, false), "gallery {}", g.index);
            let seen = reachable(grid, from, false, false);
            for &d in &g.doors {
                let door = &m.doors[d];
                assert!(door.tiles.iter().all(|t| seen.contains(t)), "gallery {}: a door out of reach", g.index);
                for col in [-1, 1] {
                    assert!(seen.contains(&door.front(col)), "gallery {}: where he steps out", g.index);
                }
            }
            if g.index == THRONE {
                assert!(m.dais.iter().all(|p| seen.contains(p)));
                assert!(m.dais.contains(&m.throne));
            } else {
                // The ring goes all the way round the well; the well itself is a void.
                let o = g.center;
                for p in [P::new(o.x, o.y + OUTER - 1), P::new(o.x - OUTER + 1, o.y), P::new(o.x + OUTER - 1, o.y)] {
                    assert!(seen.contains(&p), "gallery {}: {p:?}", g.index);
                }
                assert!(!grid.is_walkable(o.x, o.y, false) && grid.get(o.x, o.y) == crate::logic::types::Tile::Pit);
                for &(r, _) in &g.relays {
                    assert!(!grid.is_walkable(r.x, r.y, false));
                }
                let pack = &m.base.packs[g.index];
                assert!(g.floor.contains(P::new(pack.den.x0, pack.den.y0)) && g.floor.contains(P::new(pack.den.x1, pack.den.y1)));
                assert!(!pack.den.contains(m.doors[g.doors[0]].front(-1)), "it sleeps on as he steps out of a mirror");
                assert!(pack.foes.iter().all(|f| seen.contains(&f.2)), "gallery {}: its pack can reach him", g.index);
            }
        }
        // The throne's rings: the outer falls first, the dais never by the rings alone.
        assert_eq!(m.ring_of(m.landing), 0);
        assert_eq!(m.ring_of(m.throne), 3);
        assert!(m.throne_floor.iter().any(|&p| m.ring_of(p) == 1) && m.throne_floor.iter().any(|&p| m.ring_of(p) == 2));
    }
}
