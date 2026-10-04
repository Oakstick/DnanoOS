//! Gluttony's court, built to its own plan: **The Banquet of Flies**, where Beelzebub sits.
//!
//! A long hall about thirty tiles across, floored in stained flags under a living dark of flies. A
//! great table runs round the floor in a U, open to the south: its two arms down the east and west
//! of the hall and its head across the north, broken in the middle where his way to the floor
//! runs. Platters of food grey with mould stand on it (four; six when the feast has been laid);
//! behind its arms run narrow lanes with cauldrons over cold fires; at the north end, behind the
//! head of the table, his throne of piled plates stands on a low dais. The hero comes in from the
//! south, and the gate he came through locks behind him: the prince's key opens it. (The art is
//! `render::view::court_gluttony`; the fight, `game::court` and `game::court::gluttony`.)

use crate::logic::level::{LevelData, Theme};
use crate::logic::levels::level_for;
use crate::logic::sins::Sin;
use crate::logic::types::{P, V};

/// The Banquet of Flies is built: Beelzebub fights in it as himself.
pub const BUILT: bool = true;
/// The court's name.
pub const NAME: &str = "The Banquet of Flies";
/// What Beelzebub raises against the hero.
pub const ADDS: &str = "swarms";

/// The grid's side, and its middle.
pub const SIZE: i32 = 33;
pub const CENTRE: (i32, i32) = (16, 16);
/// The hall's floor: no farther than this from the middle along either axis (a long hall, its
/// corners square).
pub const HALF: i32 = 13;
/// The gate in the south wall, and where he comes in, just inside it.
pub const GATE: (i32, i32) = (16, 30);
pub const HERO: (i32, i32) = (16, 26);
/// Where Beelzebub stands as the fight begins: before his throne, behind the head of the table.
pub const PRINCE: (i32, i32) = (16, 6);
/// The throne's dais (inclusive, x0..=x1, y0..=y1), against the north wall.
pub const DAIS: (i32, i32, i32, i32) = (14, 3, 18, 4);
/// The great table in a U: its arms (x, y0..=y1), and its head (x0..=x1, y) in two halves either
/// side of the gap he comes through.
pub const ARMS: [(i32, i32, i32); 2] = [(6, 8, 23), (26, 8, 23)];
pub const HEAD: [(i32, i32, i32); 2] = [(6, 12, 8), (20, 26, 8)];
/// The platters on the table: the first four always (two on each arm), the last two (on the head)
/// only when his general's decree has laid the feast.
pub const PLATTERS: [(i32, i32); 6] = [(6, 12), (26, 12), (6, 19), (26, 19), (10, 8), (22, 8)];
/// How many platters the table bears: four, or six with the feast laid.
pub const PLATTERS_SET: usize = 4;
pub const PLATTERS_LAID: usize = 6;
/// The cauldrons over their cold fires: two in the lanes behind the arms, two by the gate.
pub const CAULDRONS: [(i32, i32); 4] = [(4, 15), (28, 15), (6, 27), (26, 27)];

/// Is (x, y) inside the hall's floor?
pub fn in_hall(x: i32, y: i32) -> bool {
    (x - CENTRE.0).abs() <= HALF && (y - CENTRE.1).abs() <= HALF
}

/// What stands on the floor at (x, y), by the plan (none: open floor or wall).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Piece {
    /// The table (a platter stands on some of its tiles: `PLATTERS`).
    Table,
    Cauldron,
    Dais,
}

/// The piece standing at (x, y), if any.
pub fn piece(x: i32, y: i32) -> Option<Piece> {
    if ARMS.iter().any(|&(ax, y0, y1)| x == ax && (y0..=y1).contains(&y))
        || HEAD.iter().any(|&(x0, x1, hy)| y == hy && (x0..=x1).contains(&x))
    {
        Some(Piece::Table)
    } else if CAULDRONS.contains(&(x, y)) {
        Some(Piece::Cauldron)
    } else if (DAIS.0..=DAIS.2).contains(&x) && (DAIS.1..=DAIS.3).contains(&y) {
        Some(Piece::Dais)
    } else {
        None
    }
}

/// Where he stands to eat from platter `at`: on the floor inside the U, against the table's edge
/// (his middle a body's width off it).
pub fn platter_stand(at: (i32, i32)) -> V {
    let reach = 0.5 + 0.95 + 0.1;
    if at.1 == HEAD[0].2 {
        V::new(at.0 as f64, at.1 as f64 + reach)
    } else if at.0 < CENTRE.0 {
        V::new(at.0 as f64 + reach, at.1 as f64)
    } else {
        V::new(at.0 as f64 - reach, at.1 as f64)
    }
}

/// The open tile beside platter `at` on the floor's side (where a swarm rises from its food).
pub fn beside_platter(at: (i32, i32)) -> P {
    if at.1 == HEAD[0].2 {
        P::new(at.0, at.1 + 1)
    } else if at.0 < CENTRE.0 {
        P::new(at.0 + 1, at.1)
    } else {
        P::new(at.0 - 1, at.1)
    }
}

/// The court as a level, as hard as dungeon `tier`: the Banquet of Flies.
pub fn level(tier: usize) -> LevelData {
    banquet(tier)
}

/// The Banquet of Flies as a level, as hard as dungeon `tier`.
pub fn banquet(tier: usize) -> LevelData {
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
    rows[PRINCE.1 as usize][PRINCE.0 as usize] = 'G';
    LevelData {
        name: NAME.to_string(),
        rows: rows.iter().map(|r| r.iter().collect()).collect(),
        heights: vec!["0".repeat(SIZE as usize); SIZE as usize],
        hero_speed: base.hero_speed,
        enemy_speed: base.enemy_speed,
        frenzy_seconds: base.frenzy_seconds,
        boss: true,
        theme: Theme::Court(Sin::Gluttony),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::parse_level;
    use crate::logic::pathing::reachable;
    use crate::logic::types::Tile;

    #[test]
    fn gluttonys_court_parses_and_every_tile_of_it_can_be_reached() {
        super::super::tests::court_is_sound(Sin::Gluttony);
    }

    #[test]
    fn the_banquet_is_a_long_hall_entered_from_the_south_with_a_table_in_a_u_round_its_floor() {
        let level = parse_level(&banquet(1));
        let g = &level.grid;
        assert_eq!((g.width, g.height), (SIZE, SIZE));
        assert_eq!(level.data.name, "The Banquet of Flies");
        assert!(level.data.boss && level.data.theme == Theme::Court(Sin::Gluttony));
        // About thirty tiles across.
        let across = (0..SIZE).filter(|&x| in_hall(x, CENTRE.1)).count();
        assert!((25..=30).contains(&across), "{across} across");
        // The gate in the south wall, him just inside it, Beelzebub before his throne to the north.
        assert_eq!(level.gate, Some(P::new(GATE.0, GATE.1)));
        assert_eq!(level.hero_spawn, P::new(HERO.0, HERO.1));
        assert!(level.hero_spawn.y < GATE.1 && GATE.1 - level.hero_spawn.y <= 4);
        assert_eq!(level.enemy_spawn, P::new(PRINCE.0, PRINCE.1));
        assert!(DAIS.3 < PRINCE.1, "the throne behind him");
        // Every open tile can be walked to.
        let seen = reachable(g, level.hero_spawn, false, false);
        for (x, y, t) in g.cells() {
            if t != Tile::Rock {
                assert!(seen.contains(&P::new(x, y)), "{x},{y} cut off");
            }
        }
        assert!(seen.contains(&level.enemy_spawn), "the way to him is open");
        // The table: a U open to the south, its head broken where his way to the floor runs.
        for &(x, y0, y1) in &ARMS {
            for y in y0..=y1 {
                assert_eq!(g.get(x, y), Tile::Rock, "the arm at {x},{y}");
            }
            assert!(g.is_walkable(x, y1 + 1, false), "open to the south below {x}");
        }
        assert!((13..=19).all(|x| g.is_walkable(x, HEAD[0].2, false)), "his way through the head of the table, his bulk and more wide");
        assert!(g.get(HEAD[0].0, HEAD[0].2) == Tile::Rock && g.get(HEAD[1].1, HEAD[1].2) == Tile::Rock);
        // Every platter stands on the table, and he can stand against it to eat; a swarm can rise beside it.
        for &p in &PLATTERS {
            assert_eq!(piece(p.0, p.1), Some(Piece::Table), "{p:?} on the table");
            let s = platter_stand(p);
            assert!(g.is_walkable(s.x.round() as i32, s.y.round() as i32, false), "beside {p:?}");
            assert!(seen.contains(&beside_platter(p)));
            assert!((s.x - p.0 as f64).abs() + (s.y - p.1 as f64).abs() < 1.6, "against its edge");
        }
        assert_eq!((PLATTERS_SET, PLATTERS_LAID), (4, 6));
        // The cauldrons in the hall, on its floor.
        for &(x, y) in &CAULDRONS {
            assert!(in_hall(x, y) && g.get(x, y) == Tile::Rock, "{x},{y}");
        }
        // Lanes run behind the arms: the hero can go round the table.
        assert!(g.is_walkable(4, 10, false) && g.is_walkable(28, 22, false));
    }
}
