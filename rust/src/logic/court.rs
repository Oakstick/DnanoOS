//! The princes' courts, built to their own plans as their fights come (ROADMAP.md, phases 3 to
//! 9). The first is Greed's: **the Counting House**, where Mammon sits.
//!
//! An octagonal hall about thirty tiles across, floored in gold-veined dark marble with a ring of
//! coin mosaic. A ring of eight black-and-gold pillars stands about its middle; heaps of coins and
//! spilled chests lie against its walls, and counting tables piled with ledgers stand east and
//! west. At the north end Mammon's throne stands on a dais under his giant brass scales. The hero
//! comes in from the south, and the gate he came through locks behind him: the prince's key opens
//! it. (The art is `render::view::court_greed`; the fight, `game::court`.)

use super::level::{LevelData, Theme};
use super::levels::level_for;
use super::sins::Sin;
use super::types::P;

/// The grid's side, and its middle.
pub const SIZE: i32 = 33;
pub const CENTRE: (i32, i32) = (16, 16);
/// The octagon's floor: no farther than this from the middle along either axis, nor than
/// `CORNER` along both together.
pub const HALF: i32 = 13;
pub const CORNER: i32 = 19;
/// The gate in the south wall, and where he comes in, just inside it.
pub const GATE: (i32, i32) = (16, 30);
pub const HERO: (i32, i32) = (16, 26);
/// Where Mammon stands as the fight begins: before his throne.
pub const MAMMON: (i32, i32) = (16, 8);
/// The throne's dais (inclusive, x0..=x1, y0..=y1), against the north wall under the scales.
pub const DAIS: (i32, i32, i32, i32) = (13, 3, 19, 4);
/// Where the great scales stand, behind the throne (a point in the north wall, for the art).
pub const SCALES: (f64, f64) = (16.0, 1.6);
/// The ring of eight black-and-gold pillars.
pub const PILLARS: [(i32, i32); 8] = [(25, 20), (20, 25), (12, 25), (7, 20), (7, 12), (12, 7), (20, 7), (25, 12)];
/// The great heaps of coins in the four corners (the golems climb out of the gold beside them).
pub const HEAPS: [(i32, i32); 4] = [(6, 8), (26, 8), (6, 24), (26, 24)];
/// Chests against the east and west walls, spilling their coins.
pub const CHESTS: [(i32, i32); 4] = [(3, 11), (3, 21), (29, 11), (29, 21)];
/// The counting tables, piled with ledgers: runs of tiles (x0, x1, y).
pub const TABLES: [(i32, i32, i32); 2] = [(5, 6, 16), (26, 27, 16)];
/// The radius of the coin-mosaic ring on the floor.
pub const MOSAIC: f64 = 5.5;

/// What the prince (and what he raises) is multiplied by in a court of `tier`: his health and his
/// damage. The first court the hero comes to is the measure; every key held before it makes the
/// prince stronger, as it does the realms.
pub fn weight(tier: usize) -> (f64, f64) {
    let t = tier.max(1) as f64 - 1.0;
    (1.0 + 0.3 * t, 0.85 + 0.1 * t)
}

/// Is (x, y) inside the octagon's floor?
pub fn in_hall(x: i32, y: i32) -> bool {
    let (dx, dy) = ((x - CENTRE.0).abs(), (y - CENTRE.1).abs());
    dx <= HALF && dy <= HALF && dx + dy <= CORNER
}

/// What stands on the floor at (x, y), by the plan (none: open floor or wall).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Piece {
    Pillar,
    Heap,
    Chest,
    Table,
    Dais,
}

/// The piece standing at (x, y), if any.
pub fn piece(x: i32, y: i32) -> Option<Piece> {
    let at = (x, y);
    if PILLARS.contains(&at) {
        Some(Piece::Pillar)
    } else if HEAPS.contains(&at) {
        Some(Piece::Heap)
    } else if CHESTS.contains(&at) {
        Some(Piece::Chest)
    } else if TABLES.iter().any(|&(x0, x1, ty)| y == ty && (x0..=x1).contains(&x)) {
        Some(Piece::Table)
    } else if (DAIS.0..=DAIS.2).contains(&x) && (DAIS.1..=DAIS.3).contains(&y) {
        Some(Piece::Dais)
    } else {
        None
    }
}

/// The Counting House as a level, as hard as dungeon `tier`.
pub fn counting_house(tier: usize) -> LevelData {
    let base = level_for(tier);
    let mut rows = vec![vec!['#'; SIZE as usize]; SIZE as usize];
    for (y, row) in rows.iter_mut().enumerate() {
        for (x, c) in row.iter_mut().enumerate() {
            let (x, y) = (x as i32, y as i32);
            if in_hall(x, y) && piece(x, y).is_none() {
                *c = ' ';
            }
        }
    }
    rows[GATE.1 as usize][GATE.0 as usize] = 'D';
    rows[HERO.1 as usize][HERO.0 as usize] = 'P';
    rows[MAMMON.1 as usize][MAMMON.0 as usize] = 'G';
    LevelData {
        name: "The Counting House".to_string(),
        rows: rows.iter().map(|r| r.iter().collect()).collect(),
        heights: vec!["0".repeat(SIZE as usize); SIZE as usize],
        hero_speed: base.hero_speed,
        enemy_speed: base.enemy_speed,
        frenzy_seconds: base.frenzy_seconds,
        boss: true,
        theme: Theme::Court(Sin::Greed),
    }
}

/// The open tiles beside a heap where a golem may climb out of the gold, nearest the middle first.
pub fn beside_heap(heap: (i32, i32)) -> P {
    let toward = |a: i32, c: i32| (c - a).signum();
    P::new(heap.0 + toward(heap.0, CENTRE.0), heap.1 + toward(heap.1, CENTRE.1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::parse_level;
    use crate::logic::pathing::reachable;
    use crate::logic::types::Tile;

    #[test]
    fn the_counting_house_is_an_octagon_entered_from_the_south_with_the_gate_behind_him() {
        let level = parse_level(&counting_house(1));
        let g = &level.grid;
        assert_eq!((g.width, g.height), (SIZE, SIZE));
        assert_eq!(level.data.name, "The Counting House");
        assert!(level.data.boss && level.data.theme == Theme::Court(Sin::Greed));
        // About thirty tiles across, its corners cut.
        let row: Vec<i32> = (0..SIZE).filter(|&x| g.get(x, CENTRE.1 - 1) != Tile::Rock).collect();
        assert!(row.len() >= 25 && row.len() <= 30, "{} across", row.len());
        assert!(g.get(CENTRE.0 - HALF, CENTRE.1 - HALF) == Tile::Rock, "a corner cut off");
        // The gate in the south wall, him just inside it facing north, Mammon before his throne to the north.
        assert_eq!(level.gate, Some(P::new(GATE.0, GATE.1)));
        assert_eq!(level.hero_spawn, P::new(HERO.0, HERO.1));
        assert!(level.hero_spawn.y < GATE.1 && GATE.1 - level.hero_spawn.y <= 4);
        assert_eq!(level.enemy_spawn, P::new(MAMMON.0, MAMMON.1));
        assert!(DAIS.3 < MAMMON.1 && (SCALES.1 as i32) < DAIS.1, "the throne behind him, the scales behind it");
        // Every open tile can be walked to, and every piece stands on the hall's floor.
        let seen = reachable(g, level.hero_spawn, false, false);
        for (x, y, t) in g.cells() {
            if t != Tile::Rock {
                assert!(seen.contains(&P::new(x, y)), "{x},{y} cut off");
            }
        }
        for &(x, y) in PILLARS.iter().chain(&HEAPS).chain(&CHESTS) {
            assert!(in_hall(x, y) && g.get(x, y) == Tile::Rock, "{x},{y}");
        }
        assert_eq!(PILLARS.len(), 8);
        // A golem has room to climb out beside each heap.
        for h in HEAPS {
            let p = beside_heap(h);
            assert!(g.is_walkable(p.x, p.y, false), "beside {h:?}");
        }
        // The way from him to Mammon is open.
        assert!(seen.contains(&level.enemy_spawn));
    }
}
