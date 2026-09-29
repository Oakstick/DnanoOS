//! The Portals of Hell: the hub's layout. Eight stone platforms float over a
//! sea of lava in a 3x3 grid round the volcano, where the House of the Devil
//! stands. The south platform is the entrance, reached along a ledge from the
//! cliffs outside; each of the other seven has a portal gate to one sin's
//! realm. Wooden bridges join each platform to its two neighbours in a ring,
//! all running straight north-south or east-west. Four causeways to the
//! volcano lie sunk in the lava until the seal of the seven sins is broken.

use super::level::{LevelData, Theme};
use super::levels::SPEED_SCALE;
use super::sins::{Sin, SINS};
use super::types::{Dir, P};

/// The hub is this many tiles on a side.
pub const HUB_SIZE: i32 = 65;
/// A platform's side, the lava between platforms, and the lava round the edge.
pub const PLATFORM: i32 = 13;
pub const GAP: i32 = 7;
pub const MARGIN: i32 = 6;
/// Bridges and causeways are three tiles wide: the middle row or column of a side, and one either way.
pub const SPAN_HALF: i32 = 1;

/// A rectangle of tiles, inclusive.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl Rect {
    pub fn contains(&self, p: P) -> bool {
        p.x >= self.x0 && p.x <= self.x1 && p.y >= self.y0 && p.y <= self.y1
    }
    /// Longer from west to east than from north to south.
    pub fn along_x(&self) -> bool {
        self.x1 - self.x0 > self.y1 - self.y0
    }
    pub fn center(&self) -> (f32, f32) {
        ((self.x0 + self.x1) as f32 / 2.0, (self.y0 + self.y1) as f32 / 2.0)
    }
    pub fn tiles(&self) -> impl Iterator<Item = P> + '_ {
        (self.y0..=self.y1).flat_map(move |y| (self.x0..=self.x1).map(move |x| P::new(x, y)))
    }
}

/// The square of the 3x3 at column `col`, row `row`.
pub fn cell(col: i32, row: i32) -> Rect {
    let (x0, y0) = (MARGIN + col * (PLATFORM + GAP), MARGIN + row * (PLATFORM + GAP));
    Rect { x0, y0, x1: x0 + PLATFORM - 1, y1: y0 + PLATFORM - 1 }
}

/// Where each platform sits in the 3x3, clockwise from the entrance (south).
const RING: [(i32, i32); 8] = [(1, 2), (0, 2), (0, 1), (0, 0), (1, 0), (2, 0), (2, 1), (2, 2)];

/// A platform: its tiles, and the sin whose portal stands on it (none for the entrance).
#[derive(Clone, Copy, Debug)]
pub struct Platform {
    pub rect: Rect,
    pub col: i32,
    pub row: i32,
    pub sin: Option<Sin>,
}

/// The eight platforms, clockwise from the entrance.
pub fn platforms() -> [Platform; 8] {
    std::array::from_fn(|i| {
        let (col, row) = RING[i];
        Platform { rect: cell(col, row), col, row, sin: if i == 0 { None } else { Some(SINS[i - 1]) } }
    })
}

/// The volcano's square in the middle.
pub fn volcano() -> Rect {
    cell(1, 1)
}

/// A portal gate: its tile in the platform's outer edge, the way it faces (into the platform), its sin.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Gate {
    pub tile: P,
    pub facing: Dir,
    pub sin: Sin,
}

impl Gate {
    /// The tile before the doors, where he stands to go through.
    pub fn front(&self) -> P {
        let (dx, dy) = self.facing.vec();
        P::new(self.tile.x + dx, self.tile.y + dy)
    }
    /// The towers either side of the doors.
    pub fn towers(&self) -> [P; 2] {
        let (dx, dy) = self.facing.vec();
        [P::new(self.tile.x - dy, self.tile.y - dx), P::new(self.tile.x + dy, self.tile.y + dx)]
    }
}

/// The seven gates, one on the outer edge of each sin's platform: the top row's in their north
/// edges, the west column's in their west edges, the east column's in their east edges.
pub fn gates() -> [Gate; 7] {
    let p = platforms();
    std::array::from_fn(|i| {
        let pl = p[i + 1];
        let (cx, cy) = ((pl.rect.x0 + pl.rect.x1) / 2, (pl.rect.y0 + pl.rect.y1) / 2);
        let (tile, facing) = if pl.row == 0 {
            (P::new(cx, pl.rect.y0), Dir::Down)
        } else if pl.col == 0 {
            (P::new(pl.rect.x0, cy), Dir::Right)
        } else {
            (P::new(pl.rect.x1, cy), Dir::Left)
        };
        Gate { tile, facing, sin: pl.sin.expect("a sin's platform") }
    })
}

/// The lava between two squares side by side, three tiles wide across their middles.
fn span_between(a: Rect, b: Rect) -> Rect {
    let (acx, acy) = ((a.x0 + a.x1) / 2, (a.y0 + a.y1) / 2);
    if a.y0 == b.y0 {
        // Side by side, west and east.
        let (w, e) = if a.x0 < b.x0 { (a, b) } else { (b, a) };
        Rect { x0: w.x1 + 1, y0: acy - SPAN_HALF, x1: e.x0 - 1, y1: acy + SPAN_HALF }
    } else {
        let (n, s) = if a.y0 < b.y0 { (a, b) } else { (b, a) };
        Rect { x0: acx - SPAN_HALF, y0: n.y1 + 1, x1: acx + SPAN_HALF, y1: s.y0 - 1 }
    }
}

/// The eight wooden bridges of the ring, each joining a platform to the next one clockwise.
pub fn bridges() -> [Rect; 8] {
    let p = platforms();
    std::array::from_fn(|i| span_between(p[i].rect, p[(i + 1) % 8].rect))
}

/// The four causeways from the north, east, south and west platforms to the volcano, sunk in the lava until the seal breaks.
pub fn causeways() -> [Rect; 4] {
    let v = volcano();
    [cell(1, 0), cell(2, 1), cell(1, 2), cell(0, 1)].map(|c| span_between(c, v))
}

/// The ledge from the cliffs outside onto the entrance platform, and where he comes in on it.
pub fn ledge() -> Rect {
    let e = cell(1, 2);
    let cx = (e.x0 + e.x1) / 2;
    Rect { x0: cx - SPAN_HALF, y0: e.y1 + 1, x1: cx + SPAN_HALF, y1: HUB_SIZE - 2 }
}

pub fn arrival() -> P {
    let l = ledge();
    P::new((l.x0 + l.x1) / 2, l.y1 - 2)
}

/// The Seal of the Seven: an altar in the middle of the entrance platform.
pub fn altar() -> P {
    let e = cell(1, 2);
    P::new((e.x0 + e.x1) / 2, (e.y0 + e.y1) / 2)
}

/// The door of the House of the Devil, in the volcano's south face, where the south causeway ends.
pub fn house_door() -> P {
    let v = volcano();
    P::new((v.x0 + v.x1) / 2, v.y1)
}

/// Broken columns stand in each platform's corners, a tile in from the edges.
pub fn columns(rect: Rect) -> [P; 4] {
    [P::new(rect.x0 + 1, rect.y0 + 1), P::new(rect.x1 - 1, rect.y0 + 1), P::new(rect.x0 + 1, rect.y1 - 1), P::new(rect.x1 - 1, rect.y1 - 1)]
}

/**
 * The hub as a level: lava ('~') everywhere but the platforms, the bridges
 * ('='), the ledge in from outside and the volcano. Each gate is its sin's
 * digit ('1' for Pride to '7' for Lust) with rock towers either side; the
 * corner columns, the altar and the volcano are rock.
 */
pub fn portals_of_hell() -> LevelData {
    let n = HUB_SIZE as usize;
    let mut rows = vec![vec!['~'; n]; n];
    let mut put = |p: P, c: char| rows[p.y as usize][p.x as usize] = c;
    for pl in platforms() {
        for p in pl.rect.tiles() {
            put(p, ' ');
        }
        for p in columns(pl.rect) {
            put(p, '#');
        }
    }
    for p in volcano().tiles() {
        put(p, '#');
    }
    for b in bridges() {
        for p in b.tiles() {
            put(p, '=');
        }
    }
    for p in ledge().tiles() {
        put(p, ' ');
    }
    // The cliffs he came down from close the ledge's end.
    let l = ledge();
    for x in l.x0 - 1..=l.x1 + 1 {
        put(P::new(x, HUB_SIZE - 1), '#');
    }
    for g in gates() {
        put(g.tile, char::from(b'1' + g.sin.index() as u8));
        for t in g.towers() {
            put(t, '#');
        }
    }
    put(altar(), '#');
    put(arrival(), 'P');
    // No foes come here, but every level names a spawn: the middle of the ledge.
    put(P::new(arrival().x, arrival().y - 2), 'G');
    LevelData {
        name: "The Portals of Hell".to_string(),
        rows: rows.iter().map(|r| r.iter().collect()).collect(),
        heights: vec!["0".repeat(n); n],
        hero_speed: 6.5 * SPEED_SCALE,
        enemy_speed: 5.4 * SPEED_SCALE,
        frenzy_seconds: 7.0,
        boss: false,
        theme: Theme::Hell,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::parse_level;
    use crate::logic::pathing::reachable;
    use crate::logic::types::Tile;

    #[test]
    fn eight_platforms_in_a_ring_of_bridges_round_the_sealed_volcano() {
        let level = parse_level(&portals_of_hell());
        let g = &level.grid;
        assert_eq!((g.width, g.height), (HUB_SIZE, HUB_SIZE));
        assert_eq!(level.hero_spawn, arrival());
        // Seven gates, one per sin, on seven different platforms.
        assert_eq!(level.portals.len(), 7);
        for gate in gates() {
            assert!(level.portals.contains(&(gate.tile, gate.sin.index())), "{:?}'s gate", gate.sin);
            assert_eq!(g.get(gate.tile.x, gate.tile.y), Tile::Rock, "shut until its doors open");
            assert!(g.is_walkable(gate.front().x, gate.front().y, false), "room before {:?}'s doors", gate.sin);
        }
        // From where he comes in, every gate's threshold and every platform can be reached...
        let seen = reachable(g, level.hero_spawn, true, false);
        for gate in gates() {
            assert!(seen.contains(&gate.front()), "{:?} unreachable", gate.sin);
        }
        // ...but only by the bridges: with them gone, the entrance is an island.
        let mut cut = g.clone();
        for b in bridges() {
            for p in b.tiles() {
                assert!(g.is_bridge(p.x, p.y), "bridge tile {p:?}");
                cut.set(p.x, p.y, Tile::Pit);
            }
        }
        let island = reachable(&cut, level.hero_spawn, true, false);
        assert!(gates().iter().all(|gate| !island.contains(&gate.front())), "no way round the bridges");
        // The volcano stands in lava; nothing reaches it until the causeways rise.
        let door = house_door();
        assert!(!seen.iter().any(|p| (p.x - door.x).abs() + (p.y - door.y).abs() <= 1), "the House is sealed off");
        for c in causeways() {
            assert!(c.tiles().all(|p| g.get(p.x, p.y) == Tile::Pit), "causeway {c:?} is still under the lava");
        }
        // The lava gaps are wider than a jump (two tiles).
        assert!(GAP > 2);
    }

    #[test]
    fn every_bridge_and_causeway_runs_straight() {
        for r in bridges().iter().chain(causeways().iter()) {
            let (len, wide) = if r.along_x() { (r.x1 - r.x0 + 1, r.y1 - r.y0 + 1) } else { (r.y1 - r.y0 + 1, r.x1 - r.x0 + 1) };
            assert_eq!((len, wide), (GAP, 2 * SPAN_HALF + 1), "{r:?}");
        }
    }
}
