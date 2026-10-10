//! Envy's court, built to its own plan: **the Drowned Bell**, where Leviathan waits.
//!
//! A drowned temple: a round hall about twenty-eight tiles across, walled in green-black stone
//! with serpent bones set in it. A ring of shallow green water runs round its central floor (the
//! water is only looks: it never slows the hero), six broken columns standing in it. In the middle
//! a great bronze bell hangs from a broken beam, its mouth an arm's height above the floor: no one
//! walks under it. At the north end, against the wall, the drowned altar where the serpent sleeps;
//! Leviathan rises before it. The hero comes in from the south through the water, and the gate he
//! came through locks behind him: the prince's key opens it. (The art is
//! `render::view::court_envy`; the fight, `game::court::envy`.)

use crate::logic::level::{LevelData, Theme};
use crate::logic::levels::level_for;
use crate::logic::sins::Sin;
use crate::logic::types::{P, V};

/// The Drowned Bell is built: Leviathan fights in it as himself.
pub const BUILT: bool = true;
/// The court's name.
pub const NAME: &str = "The Drowned Bell";
/// What Leviathan raises against the hero.
pub const ADDS: &str = "serpentlings";

/// The grid's side, and its middle.
pub const SIZE: i32 = 33;
pub const CENTRE: (i32, i32) = (16, 16);
/// The round hall's floor: no farther than this from the middle.
pub const RADIUS: f64 = 13.9;
/// The ring of shallow water round the central floor: from this far out to this far.
pub const WATER: (f64, f64) = (9.2, 12.3);
/// The gate in the south wall, and where he comes in, just inside it (in the water).
pub const GATE: (i32, i32) = (16, 30);
pub const HERO: (i32, i32) = (16, 26);
/// Where Leviathan rises as the fight begins: before his altar.
pub const LEVIATHAN: (i32, i32) = (16, 7);
/// Where the prince stands as the fight begins (Leviathan, before his altar).
pub const PRINCE: (i32, i32) = LEVIATHAN;
/// The drowned altar against the north wall (inclusive, x0..=x1, y0..=y1).
pub const ALTAR: (i32, i32, i32, i32) = (14, 3, 18, 4);
/// The bell's tiles (inclusive): three by three about the middle, its rim on the floor's stone.
pub const BELL_TILES: (i32, i32, i32, i32) = (15, 15, 17, 17);
/// The bell's middle, and how far its rim reaches from it.
pub const BELL: V = V { x: 16.0, y: 16.0 };
pub const BELL_RIM: f64 = 1.45;
/// The six broken columns standing in the water.
pub const PILLARS: [(i32, i32); 6] = [(27, 16), (21, 25), (11, 25), (5, 16), (11, 7), (21, 7)];
/// Where the water stirs: the serpentlings rise from these, between the columns.
pub const SPRINGS: [(i32, i32); 6] = [(25, 21), (16, 27), (7, 21), (7, 11), (16, 5), (25, 11)];

/// How far (x, y) lies from the hall's middle.
fn from_middle(x: f64, y: f64) -> f64 {
    ((x - CENTRE.0 as f64).powi(2) + (y - CENTRE.1 as f64).powi(2)).sqrt()
}

/// Is (x, y) inside the round hall's floor?
pub fn in_hall(x: i32, y: i32) -> bool {
    from_middle(x as f64, y as f64) <= RADIUS
}

/// Is (x, y) (a point on the floor) in the ring of shallow water?
pub fn in_water(x: f64, y: f64) -> bool {
    let r = from_middle(x, y);
    r >= WATER.0 && r <= WATER.1
}

/// What stands on the floor at (x, y), by the plan (none: open floor or wall).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Piece {
    Bell,
    Pillar,
    Altar,
}

/// The piece standing at (x, y), if any.
pub fn piece(x: i32, y: i32) -> Option<Piece> {
    let within = |r: (i32, i32, i32, i32)| (r.0..=r.2).contains(&x) && (r.1..=r.3).contains(&y);
    if within(BELL_TILES) {
        Some(Piece::Bell)
    } else if PILLARS.contains(&(x, y)) {
        Some(Piece::Pillar)
    } else if within(ALTAR) {
        Some(Piece::Altar)
    } else {
        None
    }
}

/// The court as a level, as hard as dungeon `tier`: the Drowned Bell.
pub fn level(tier: usize) -> LevelData {
    drowned_bell(tier)
}

/// The Drowned Bell as a level, as hard as dungeon `tier`.
pub fn drowned_bell(tier: usize) -> LevelData {
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
    rows[LEVIATHAN.1 as usize][LEVIATHAN.0 as usize] = 'G';
    LevelData {
        name: NAME.to_string(),
        rows: rows.iter().map(|r| r.iter().collect()).collect(),
        heights: vec!["0".repeat(SIZE as usize); SIZE as usize],
        hero_speed: base.hero_speed,
        enemy_speed: base.enemy_speed,
        frenzy_seconds: base.frenzy_seconds,
        boss: true,
        theme: Theme::Court(Sin::Envy),
    }
}

/// The springs in the water nearest `at`, nearest first (where the serpentlings rise).
pub fn springs_near(at: V) -> Vec<P> {
    let mut out: Vec<P> = SPRINGS.iter().map(|&(x, y)| P::new(x, y)).collect();
    let d = |p: &P| (p.x as f64 - at.x).powi(2) + (p.y as f64 - at.y).powi(2);
    out.sort_by(|a, b| d(a).total_cmp(&d(b)).then(a.y.cmp(&b.y)).then(a.x.cmp(&b.x)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::parse_level;
    use crate::logic::pathing::reachable;
    use crate::logic::types::Tile;

    #[test]
    fn the_drowned_bell_is_a_round_hall_round_a_bell_entered_from_the_south_through_the_water() {
        let level = parse_level(&drowned_bell(1));
        let g = &level.grid;
        assert_eq!((g.width, g.height), (SIZE, SIZE));
        assert_eq!(level.data.name, "The Drowned Bell");
        assert!(level.data.boss && level.data.theme == Theme::Court(Sin::Envy));
        // About thirty tiles across, and round: its corners are rock.
        let row: Vec<i32> = (0..SIZE).filter(|&x| g.get(x, CENTRE.1 - 2) != Tile::Rock).collect();
        assert!(row.len() >= 25 && row.len() <= 30, "{} across", row.len());
        for (x, y) in [(3, 3), (29, 3), (3, 29), (29, 29)] {
            assert_eq!(g.get(x, y), Tile::Rock, "a round hall: {x},{y}");
        }
        // The gate in the south wall, him just inside it (in the water) facing north, Leviathan
        // before his altar to the north.
        assert_eq!(level.gate, Some(P::new(GATE.0, GATE.1)));
        assert!((GATE.0 - 1..=GATE.0 + 1).filter(|&x| x != GATE.0).all(|x| g.get(x, GATE.1) == Tile::Rock), "in the wall");
        assert_eq!(level.hero_spawn, P::new(HERO.0, HERO.1));
        assert!(level.hero_spawn.y < GATE.1 && GATE.1 - level.hero_spawn.y <= 4);
        assert!(in_water(HERO.0 as f64, HERO.1 as f64), "he comes in through the water");
        assert_eq!(level.enemy_spawn, P::new(LEVIATHAN.0, LEVIATHAN.1));
        assert!(ALTAR.3 < LEVIATHAN.1 && ALTAR.1 > 2, "the altar behind him, against the north wall");
        // The bell in the middle: no one walks under it, and its rim is where the plan says.
        for x in BELL_TILES.0..=BELL_TILES.2 {
            for y in BELL_TILES.1..=BELL_TILES.3 {
                assert_eq!(g.get(x, y), Tile::Rock, "the bell at {x},{y}");
            }
        }
        assert_eq!((BELL.x as i32, BELL.y as i32), CENTRE);
        const { assert!(BELL_RIM >= 1.4 && BELL_RIM <= 1.6) };
        // Open floor all round the bell, wide enough for him to coil round it and the hero to walk it.
        for k in 0..32 {
            let a = k as f64 * std::f64::consts::TAU / 32.0;
            for r in [2.5, 3.5, 4.5, 6.0] {
                let p = P::new((BELL.x + a.cos() * r).round() as i32, (BELL.y + a.sin() * r).round() as i32);
                assert!(g.is_walkable(p.x, p.y, false), "round the bell at {r}: {p:?}");
            }
        }
        // Every open tile can be walked to; every piece stands on the hall's floor; the columns
        // stand in the water, and the springs are open water.
        let seen = reachable(g, level.hero_spawn, false, false);
        for (x, y, t) in g.cells() {
            if t != Tile::Rock {
                assert!(seen.contains(&P::new(x, y)), "{x},{y} cut off");
            }
        }
        for &(x, y) in &PILLARS {
            assert!(in_hall(x, y) && g.get(x, y) == Tile::Rock && in_water(x as f64, y as f64), "column {x},{y}");
        }
        for &(x, y) in &SPRINGS {
            assert!(g.is_walkable(x, y, false) && in_water(x as f64, y as f64), "spring {x},{y}");
        }
        assert_eq!(springs_near(V::new(16.0, 28.0))[0], P::new(16, 27));
        // The way from him to Leviathan is open.
        assert!(seen.contains(&level.enemy_spawn));
        super::super::tests::court_is_sound(Sin::Envy);
    }

    #[test]
    fn envys_court_parses_and_every_tile_of_it_can_be_reached() {
        super::super::tests::court_is_sound(Sin::Envy);
    }
}
