//! Wrath's court, built to its own plan: **the Anvil of Wrath**, where Amon waits.
//!
//! A round hall of black basalt about thirty tiles across, its floor cracked with seams that glow
//! from the fire under it, broken weapons stuck in the ground where they fell. Four broken stone
//! pillars stand about the floor, two by two, and they matter: Amon's charge runs straight, and a
//! charge that ends in stone leaves him dazed (`game::court::wrath`). At the north end a great
//! anvil stands on a dais with his burning throne behind it; war banners burn on the walls. The
//! hero comes in from the south, and the gate he came through locks behind him: the prince's key
//! opens it. (The art is `render::view::court_wrath`; the fight, `game::court::wrath`.)

use crate::logic::level::{LevelData, Theme};
use crate::logic::levels::level_for;
use crate::logic::sins::Sin;
use crate::logic::types::P;

/// The Anvil of Wrath is built: Amon fights in it as himself.
pub const BUILT: bool = true;
/// The court's name.
pub const NAME: &str = "The Anvil of Wrath";
/// What Amon raises against the hero.
pub const ADDS: &str = "ravens";

/// The grid's side, and its middle.
pub const SIZE: i32 = 33;
pub const CENTRE: (i32, i32) = (16, 16);
/// The ring's floor: no farther than this from the middle.
pub const RADIUS: f64 = 13.4;
/// The gate in the south wall, and where he comes in, just inside it.
pub const GATE: (i32, i32) = (16, 30);
pub const HERO: (i32, i32) = (16, 26);
/// Where Amon stands as the fight begins: before his anvil.
pub const PRINCE: (i32, i32) = (16, 8);
/// The dais against the north wall (inclusive, x0..=x1, y0..=y1), the anvil on it, the throne behind.
pub const DAIS: (i32, i32, i32, i32) = (12, 3, 20, 5);
/// The great anvil on the dais: a run of tiles (x0, x1, y), solid.
pub const ANVIL: (i32, i32, i32) = (15, 17, 5);
/// Where his burning throne stands, behind the anvil (a point in the north wall, for the art).
pub const THRONE: (f64, f64) = (16.0, 1.9);
/// The four broken stone pillars, each two tiles by two: their north-west tiles.
pub const PILLARS: [(i32, i32); 4] = [(9, 11), (22, 11), (9, 20), (22, 20)];
/// Where the ravens come down: before the burning banners, near the four quarters of the wall.
pub const PERCHES: [(i32, i32); 6] = [(6, 8), (26, 8), (4, 16), (28, 16), (7, 25), (25, 25)];

/// Is (x, y) inside the ring's floor?
pub fn in_hall(x: i32, y: i32) -> bool {
    let (dx, dy) = ((x - CENTRE.0) as f64, (y - CENTRE.1) as f64);
    dx * dx + dy * dy <= RADIUS * RADIUS
}

/// What stands on the floor at (x, y), by the plan (none: open floor or wall).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Piece {
    /// A broken stone pillar (solid): a charge that runs into it ends dazed.
    Pillar,
    /// The great anvil (solid).
    Anvil,
    /// The dais it stands on (open floor).
    Dais,
}

/// The pillar standing at (x, y), if any: its north-west tile.
pub fn pillar_at(x: i32, y: i32) -> Option<(i32, i32)> {
    PILLARS.iter().copied().find(|&(px, py)| (px..=px + 1).contains(&x) && (py..=py + 1).contains(&y))
}

/// The piece standing at (x, y), if any.
pub fn piece(x: i32, y: i32) -> Option<Piece> {
    if pillar_at(x, y).is_some() {
        Some(Piece::Pillar)
    } else if y == ANVIL.2 && (ANVIL.0..=ANVIL.1).contains(&x) {
        Some(Piece::Anvil)
    } else if (DAIS.0..=DAIS.2).contains(&x) && (DAIS.1..=DAIS.3).contains(&y) {
        Some(Piece::Dais)
    } else {
        None
    }
}

/// A pillar's middle (between its four tiles).
pub fn pillar_middle(p: (i32, i32)) -> (f64, f64) {
    (p.0 as f64 + 0.5, p.1 as f64 + 0.5)
}

/// The court as a level, as hard as dungeon `tier`: the Anvil of Wrath.
pub fn level(tier: usize) -> LevelData {
    anvil_of_wrath(tier)
}

/// The Anvil of Wrath as a level, as hard as dungeon `tier`.
pub fn anvil_of_wrath(tier: usize) -> LevelData {
    let base = level_for(tier);
    let mut rows = vec![vec!['#'; SIZE as usize]; SIZE as usize];
    for (y, row) in rows.iter_mut().enumerate() {
        for (x, c) in row.iter_mut().enumerate() {
            let (x, y) = (x as i32, y as i32);
            let solid = matches!(piece(x, y), Some(Piece::Pillar | Piece::Anvil));
            if in_hall(x, y) && !solid {
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
        theme: Theme::Court(Sin::Wrath),
    }
}

/// Where a raven comes down from its perch: the perch, or the nearest open floor toward the middle.
pub fn below_perch(perch: (i32, i32)) -> P {
    let toward = |a: i32, c: i32| (c - a).signum();
    let mut p = P::new(perch.0, perch.1);
    for _ in 0..4 {
        if in_hall(p.x, p.y) && piece(p.x, p.y).is_none() {
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
    fn the_anvil_of_wrath_is_a_round_hall_entered_from_the_south_with_four_pillars_to_bait_him_into() {
        let level = parse_level(&anvil_of_wrath(1));
        let g = &level.grid;
        assert_eq!((g.width, g.height), (SIZE, SIZE));
        assert_eq!(level.data.name, "The Anvil of Wrath");
        assert!(level.data.boss && level.data.theme == Theme::Court(Sin::Wrath));
        // About thirty tiles across, round.
        let row: Vec<i32> = (0..SIZE).filter(|&x| g.get(x, CENTRE.1) != Tile::Rock).collect();
        assert!(row.len() >= 25 && row.len() <= 30, "{} across", row.len());
        assert!(g.get(CENTRE.0 - 11, CENTRE.1 - 11) == Tile::Rock, "round: the corners are wall");
        // The gate in the south wall, him just inside it, Amon to the north before his anvil.
        assert_eq!(level.gate, Some(P::new(GATE.0, GATE.1)));
        assert_eq!(level.hero_spawn, P::new(HERO.0, HERO.1));
        assert!(level.hero_spawn.y < GATE.1 && GATE.1 - level.hero_spawn.y <= 4);
        assert_eq!(level.enemy_spawn, P::new(PRINCE.0, PRINCE.1));
        assert!(ANVIL.2 < PRINCE.1 && (THRONE.1 as i32) < DAIS.1, "the anvil behind him, the throne behind it");
        assert_eq!(g.get(16, ANVIL.2), Tile::Rock, "the anvil is solid");
        // Every open tile can be walked to.
        let seen = reachable(g, level.hero_spawn, false, false);
        for (x, y, t) in g.cells() {
            if t != Tile::Rock {
                assert!(seen.contains(&P::new(x, y)), "{x},{y} cut off");
            }
        }
        // Four pillars of stone, each two by two, on the floor: open ground all round each (room to
        // stand before it and step aside), and each within a charge's run of the middle and of the
        // others, so there is always stone to bait him into.
        assert_eq!(PILLARS.len(), 4);
        for &p in &PILLARS {
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                assert!(in_hall(p.0 + dx, p.1 + dy) && g.get(p.0 + dx, p.1 + dy) == Tile::Rock, "{p:?}");
            }
            for (dx, dy) in [(-2, 0), (3, 0), (0, -2), (0, 3), (-2, 1), (3, 1), (1, -2), (1, 3)] {
                assert!(g.is_walkable(p.0 + dx, p.1 + dy, false), "room round {p:?}");
            }
            let m = pillar_middle(p);
            let from_middle = ((m.0 - CENTRE.0 as f64).powi(2) + (m.1 - CENTRE.1 as f64).powi(2)).sqrt();
            assert!(from_middle > 5.0 && from_middle < 9.0, "{p:?}: {from_middle}");
        }
        // The way between the hero and Amon is open, and no pillar stands on it.
        assert!((PRINCE.1..=HERO.1).all(|y| g.is_walkable(HERO.0, y, false)));
        // A raven can come down below each perch.
        for p in PERCHES {
            let at = below_perch(p);
            assert!(g.is_walkable(at.x, at.y, false), "below {p:?}");
        }
        super::super::tests::court_is_sound(Sin::Wrath);
    }

    #[test]
    fn wraths_court_parses_and_every_tile_of_it_can_be_reached() {
        super::super::tests::court_is_sound(Sin::Wrath);
        assert!(super::super::built(Sin::Wrath));
        assert_eq!(super::super::court_level(Sin::Wrath, 1).name, NAME);
    }
}
