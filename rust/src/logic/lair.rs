//! Krampus's lair (FINALE.md §3): the bottom of the Pit, where the stone platform the hero rode
//! down through the seven levels comes to rest. A frozen hollow under the earth about thirty tiles
//! across, its floor black stone dusted with snow; icicles hang from the dark above, iron chains
//! hang down out of it with bells on them, and about the floor lie bundles of birch switches, his
//! sacks and his basket, broken toys half buried in the snow. Four columns of ice, frozen from the
//! roof to the floor, stand about the hollow: Krampus's charge runs straight, and a charge that
//! ends in stone (an ice column, the cage, the wall) leaves him dazed (`game::lair`). Two braziers
//! of his coal burn red either side of the cage.
//!
//! At the north end the iron cage hangs from a great chain, low over the floor, and in it the
//! friend he took at the graveyard. The platform (`PLATFORM`, eleven tiles by eleven) lies in the
//! south of the hollow, the hero standing in its middle as it comes to rest; Krampus waits between
//! it and the cage. There is no gate: the way out is the way the platform came, up.
//!
//! (The fight is `game::lair`, its art `render::view::lair`.)

use super::level::{LevelData, Theme};
use super::levels::level_for;
use super::types::P;

/// The lair's name.
pub const NAME: &str = "Krampus's Lair";
/// What Krampus raises against the hero.
pub const ADDS: &str = "Perchten";

/// The grid's side, and its middle.
pub const SIZE: i32 = 33;
pub const CENTRE: (i32, i32) = (16, 16);
/// The hollow's floor: no farther than this from the middle.
pub const RADIUS: f64 = 14.2;
/// The platform, come to rest (inclusive, x0..=x1, y0..=y1): eleven tiles by eleven, its top flush
/// with the floor.
pub const PLATFORM: (i32, i32, i32, i32) = (11, 18, 21, 28);
/// Where the hero stands as the platform comes to rest: its middle.
pub const HERO: (i32, i32) = (16, 23);
/// Where Krampus stands as the fight begins: between the platform and the cage.
pub const KRAMPUS: (i32, i32) = (16, 9);
/// The iron cage, hanging low over the floor at the north end (inclusive tiles, solid: nothing
/// walks under it or into it), and its middle.
pub const CAGE: (i32, i32, i32, i32) = (15, 2, 17, 4);
pub const CAGE_MIDDLE: (f64, f64) = (16.0, 3.0);
/// Where Krampus stands to rattle the cage: before its door, on open floor.
pub const CAGE_STAND: (f64, f64) = (16.0, 6.0);
/// The four columns of ice, each two tiles by two: their north-west tiles.
pub const COLUMNS: [(i32, i32); 4] = [(10, 10), (21, 10), (7, 16), (24, 16)];
/// The braziers of his coal, either side of the cage (solid).
pub const BRAZIERS: [(i32, i32); 2] = [(12, 5), (20, 5)];
/// His great basket and his sacks, against the wall (solid).
pub const BASKET: (i32, i32) = (25, 6);
pub const SACKS: [(i32, i32); 3] = [(6, 8), (27, 22), (5, 24)];
/// Bundles of birch switches lying on the floor (walked over).
pub const BIRCH: [(f64, f64); 5] = [(8.4, 6.2), (23.2, 4.6), (4.2, 13.6), (28.4, 15.2), (24.6, 26.4)];
/// Broken toys half buried in the snow (walked over): a wooden horse, a doll, a drum, a top.
pub const TOYS: [(f64, f64); 6] = [(9.6, 4.8), (21.8, 7.4), (4.8, 19.6), (27.6, 11.4), (8.2, 27.2), (26.4, 24.2)];
/// The chains hanging out of the dark above near the wall, a bell on each (over the floor; walked
/// under).
pub const CHAINS: [(f64, f64); 10] =
    [(6.0, 9.0), (26.0, 9.0), (3.6, 16.0), (28.4, 16.0), (5.0, 22.5), (27.0, 22.5), (9.0, 4.6), (23.0, 4.6), (10.0, 27.6), (22.0, 27.6)];
/// Where his Perchten come out of the dark: two clefts in the wall, west and east.
pub const CLEFTS: [(i32, i32); 2] = [(3, 12), (29, 12)];

/// What stands on the floor at (x, y), by the plan.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Piece {
    /// A column of ice from the roof to the floor (solid): a charge that runs into it ends dazed.
    Column,
    /// The cage (solid).
    Cage,
    /// A brazier of coals (solid).
    Brazier,
    /// His basket, or a sack (solid).
    Basket,
    /// The platform's slab (open floor).
    Platform,
}

impl Piece {
    /// Nothing walks through it.
    pub fn solid(self) -> bool {
        self != Piece::Platform
    }
}

/// Is (x, y) inside the hollow's floor?
pub fn in_hollow(x: i32, y: i32) -> bool {
    let (dx, dy) = ((x - CENTRE.0) as f64, (y - CENTRE.1) as f64);
    dx * dx + dy * dy <= RADIUS * RADIUS
}

/// The column standing at (x, y), if any: its north-west tile.
pub fn column_at(x: i32, y: i32) -> Option<(i32, i32)> {
    COLUMNS.iter().copied().find(|&(px, py)| (px..=px + 1).contains(&x) && (py..=py + 1).contains(&y))
}

/// A column's middle (between its four tiles).
pub fn column_middle(p: (i32, i32)) -> (f64, f64) {
    (p.0 as f64 + 0.5, p.1 as f64 + 0.5)
}

/// Is (x, y) on the platform?
pub fn on_platform(x: i32, y: i32) -> bool {
    (PLATFORM.0..=PLATFORM.2).contains(&x) && (PLATFORM.1..=PLATFORM.3).contains(&y)
}

/// The piece standing at (x, y), if any.
pub fn piece(x: i32, y: i32) -> Option<Piece> {
    if column_at(x, y).is_some() {
        Some(Piece::Column)
    } else if (CAGE.0..=CAGE.2).contains(&x) && (CAGE.1..=CAGE.3).contains(&y) {
        Some(Piece::Cage)
    } else if BRAZIERS.contains(&(x, y)) {
        Some(Piece::Brazier)
    } else if BASKET == (x, y) || SACKS.contains(&(x, y)) {
        Some(Piece::Basket)
    } else if on_platform(x, y) {
        Some(Piece::Platform)
    } else {
        None
    }
}

/// The lair as a level, as hard as dungeon `tier`.
pub fn level(tier: usize) -> LevelData {
    let base = level_for(tier);
    let mut rows = vec![vec!['#'; SIZE as usize]; SIZE as usize];
    for (y, row) in rows.iter_mut().enumerate() {
        for (x, c) in row.iter_mut().enumerate() {
            let (x, y) = (x as i32, y as i32);
            if in_hollow(x, y) && !piece(x, y).is_some_and(Piece::solid) {
                *c = ' ';
            }
        }
    }
    rows[HERO.1 as usize][HERO.0 as usize] = 'P';
    rows[KRAMPUS.1 as usize][KRAMPUS.0 as usize] = 'G';
    LevelData {
        name: NAME.to_string(),
        rows: rows.iter().map(|r| r.iter().collect()).collect(),
        heights: vec!["0".repeat(SIZE as usize); SIZE as usize],
        hero_speed: base.hero_speed,
        enemy_speed: base.enemy_speed,
        frenzy_seconds: base.frenzy_seconds,
        boss: true,
        theme: Theme::Lair,
    }
}

/// Where a Percht comes out of a cleft: the nearest open floor to it toward the middle.
pub fn out_of_cleft(cleft: (i32, i32)) -> P {
    let toward = |a: i32, c: i32| (c - a).signum();
    let mut p = P::new(cleft.0, cleft.1);
    for _ in 0..6 {
        if in_hollow(p.x, p.y) && !piece(p.x, p.y).is_some_and(Piece::solid) {
            break;
        }
        p = P::new(p.x + toward(p.x, CENTRE.0), p.y + toward(p.y, CENTRE.1));
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::parse_level;
    use crate::logic::pathing::reachable;
    use crate::logic::types::Tile;

    #[test]
    fn the_lair_is_a_round_hollow_with_the_platform_in_the_south_and_the_cage_in_the_north() {
        let level = parse_level(&level(8));
        let g = &level.grid;
        assert_eq!((g.width, g.height), (SIZE, SIZE));
        assert_eq!(level.data.name, "Krampus's Lair");
        assert!(level.data.boss && level.data.theme == Theme::Lair);
        // About thirty tiles across, round.
        let row: Vec<i32> = (0..SIZE).filter(|&x| g.get(x, CENTRE.1) != Tile::Rock || column_at(x, CENTRE.1).is_some()).collect();
        assert!(row.len() >= 27 && row.len() <= 31, "{} across", row.len());
        assert_eq!(g.get(CENTRE.0 - 12, CENTRE.1 - 12), Tile::Rock, "round: the corners are wall");
        // The platform: eleven by eleven, open, in the south half; the hero in its middle.
        assert_eq!((PLATFORM.2 - PLATFORM.0 + 1, PLATFORM.3 - PLATFORM.1 + 1), (11, 11));
        for y in PLATFORM.1..=PLATFORM.3 {
            for x in PLATFORM.0..=PLATFORM.2 {
                assert!(g.is_walkable(x, y, false), "the platform at {x},{y}");
            }
        }
        assert_eq!(level.hero_spawn, P::new(HERO.0, HERO.1));
        assert_eq!(((PLATFORM.0 + PLATFORM.2) / 2, (PLATFORM.1 + PLATFORM.3) / 2), HERO, "in its middle");
        assert!(HERO.1 > CENTRE.1, "in the south");
        assert!(level.gate.is_none(), "no gate: the way out is up");
        // The cage at the north end, solid; Krampus between it and the platform; his stand before
        // its door open, with room for him.
        assert_eq!(level.enemy_spawn, P::new(KRAMPUS.0, KRAMPUS.1));
        assert!(CAGE.3 < KRAMPUS.1 && KRAMPUS.1 < PLATFORM.1, "the cage, Krampus, the platform, north to south");
        for y in CAGE.1..=CAGE.3 {
            for x in CAGE.0..=CAGE.2 {
                assert!(in_hollow(x, y) && g.get(x, y) == Tile::Rock, "the cage at {x},{y}");
            }
        }
        assert!(CAGE_MIDDLE.1 < 5.0 && (CAGE_MIDDLE.0 - CENTRE.0 as f64).abs() < 0.1);
        let stand = P::new(CAGE_STAND.0.round() as i32, CAGE_STAND.1.round() as i32);
        for (dx, dy) in [(0, 0), (-1, 0), (1, 0), (0, 1), (-1, 1), (1, 1)] {
            assert!(g.is_walkable(stand.x + dx, stand.y + dy, false), "room before the cage at {dx},{dy}");
        }
        assert!(CAGE_STAND.1 - CAGE.3 as f64 - 0.5 >= 1.0, "his body clears the bars");
        // Every open tile can be walked to.
        let seen = reachable(g, level.hero_spawn, false, false);
        for (x, y, t) in g.cells() {
            if t != Tile::Rock {
                assert!(seen.contains(&P::new(x, y)), "{x},{y} cut off");
            }
        }
        // Four columns of ice, two by two, with open ground all round each, each within a charge's
        // run of the middle: there is always stone to bait him into.
        for &c in &COLUMNS {
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                assert_eq!(g.get(c.0 + dx, c.1 + dy), Tile::Rock, "{c:?}");
            }
            for (dx, dy) in [(-2, 0), (3, 0), (0, -2), (0, 3), (-2, 1), (3, 1), (1, -2), (1, 3)] {
                assert!(g.is_walkable(c.0 + dx, c.1 + dy, false), "room round {c:?}");
            }
            let m = column_middle(c);
            let from_middle = ((m.0 - CENTRE.0 as f64).powi(2) + (m.1 - CENTRE.1 as f64).powi(2)).sqrt();
            assert!(from_middle > 5.0 && from_middle < 9.0, "{c:?}: {from_middle}");
            assert!(!on_platform(c.0, c.1) && !on_platform(c.0 + 1, c.1 + 1), "off the platform");
        }
        // The way from the platform to Krampus and on to the cage's stand is open.
        assert!((stand.y..=HERO.1).all(|y| g.is_walkable(HERO.0, y, false)));
        // The props stand on the floor, in the hollow, off the platform; a Percht comes out of each cleft.
        for p in BRAZIERS.iter().chain(SACKS.iter()).chain(std::iter::once(&BASKET)) {
            assert!(in_hollow(p.0, p.1) && g.get(p.0, p.1) == Tile::Rock && !on_platform(p.0, p.1), "{p:?}");
        }
        for p in BIRCH.iter().chain(TOYS.iter()) {
            let t = P::new(p.0.round() as i32, p.1.round() as i32);
            assert!(g.is_walkable(t.x, t.y, false) && !on_platform(t.x, t.y), "{p:?}");
        }
        for c in CLEFTS {
            let at = out_of_cleft(c);
            assert!(g.is_walkable(at.x, at.y, false), "out of {c:?}");
        }
    }
}
