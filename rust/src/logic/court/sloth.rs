//! Sloth's court: **The Hall of Stopped Clocks**, where Belphegor dozes.
//!
//! A grey bedchamber the size of a hall, about thirty tiles across, its corners cut: a floor of
//! worn grey boards under a fog that lies on it, walls of faded grey-blue hangings between timber
//! posts, great beams overhead hung with cobwebs. Stopped clocks of every size stand round the
//! walls (the tall longcase clocks are pieces of the floor; the small ones hang on the walls). At
//! the north end Belphegor's great chair stands against the wall, and before it on the floor the
//! giant stopped hourglass; a canopied bed stands in the north-east corner, unslept in, and a drift
//! of sand has run out across the south-west. Four timber posts hold the beams up over the floor.
//! The hero comes in from the south, and the gate locks behind him: the prince's key opens it.
//! (The art is `render::view::court_sloth`; the fight, `game::court` and `game::court::sloth`.)

use crate::logic::level::{LevelData, Theme};
use crate::logic::levels::level_for;
use crate::logic::sins::Sin;
use crate::logic::types::P;

/// The Hall of Stopped Clocks is built: Belphegor fights in it as himself.
pub const BUILT: bool = true;
/// The court's name.
pub const NAME: &str = "The Hall of Stopped Clocks";
/// What Belphegor raises against the hero.
pub const ADDS: &str = "nightmares";

/// The grid's side, and its middle.
pub const SIZE: i32 = 33;
pub const CENTRE: (i32, i32) = (16, 16);
/// The hall's floor: no farther than this from the middle along either axis, nor than `CORNER`
/// along both together (its corners cut).
pub const HALF: i32 = 13;
pub const CORNER: i32 = 24;
/// The gate in the south wall, and where he comes in, just inside it.
pub const GATE: (i32, i32) = (16, 30);
pub const HERO: (i32, i32) = (16, 26);
/// Where Belphegor sits as the fight begins: in his great chair, before its back.
pub const PRINCE: (i32, i32) = (16, 8);
/// The great chair against the north wall (inclusive, x0..=x1, y0..=y1): its high back and its
/// arms; he sits on the front of its seat.
pub const CHAIR: (i32, i32, i32, i32) = (15, 3, 17, 6);
/// The giant hourglass, stopped, on the floor before the chair (off to its right, out of the way
/// between him and the gate).
pub const HOURGLASS: (i32, i32) = (20, 10);
/// The canopied bed in the north-east corner (inclusive).
pub const BED: (i32, i32, i32, i32) = (25, 4, 27, 6);
/// The tall stopped clocks standing against the walls.
pub const CLOCKS: [(i32, i32); 12] =
    [(3, 9), (3, 15), (3, 21), (29, 9), (29, 15), (29, 21), (7, 3), (10, 3), (21, 3), (9, 29), (23, 29), (3, 25)];
/// The four timber posts that hold up the beams.
pub const POSTS: [(i32, i32); 4] = [(9, 11), (23, 11), (9, 21), (23, 21)];
/// The drift of sand run out across the south-west floor: its middle and its reach (no piece:
/// it is walked over).
pub const DRIFT: (f64, f64, f64) = (8.5, 23.5, 3.2);

/// Is (x, y) inside the hall's floor?
pub fn in_hall(x: i32, y: i32) -> bool {
    let (dx, dy) = ((x - CENTRE.0).abs(), (y - CENTRE.1).abs());
    dx <= HALF && dy <= HALF && dx + dy <= CORNER
}

/// What stands on the floor at (x, y), by the plan (none: open floor or wall).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Piece {
    Chair,
    Hourglass,
    Bed,
    Clock,
    Post,
}

fn in_rect(r: (i32, i32, i32, i32), x: i32, y: i32) -> bool {
    (r.0..=r.2).contains(&x) && (r.1..=r.3).contains(&y)
}

/// The piece standing at (x, y), if any.
pub fn piece(x: i32, y: i32) -> Option<Piece> {
    let at = (x, y);
    if in_rect(CHAIR, x, y) {
        Some(Piece::Chair)
    } else if at == HOURGLASS {
        Some(Piece::Hourglass)
    } else if in_rect(BED, x, y) {
        Some(Piece::Bed)
    } else if CLOCKS.contains(&at) {
        Some(Piece::Clock)
    } else if POSTS.contains(&at) {
        Some(Piece::Post)
    } else {
        None
    }
}

/// The court as a level, as hard as dungeon `tier`: the Hall of Stopped Clocks.
pub fn level(tier: usize) -> LevelData {
    hall_of_stopped_clocks(tier)
}

/// The Hall of Stopped Clocks as a level, as hard as dungeon `tier`.
pub fn hall_of_stopped_clocks(tier: usize) -> LevelData {
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
        theme: Theme::Court(Sin::Sloth),
    }
}

/// Where his nightmares gather out of the fog: either side of him, across the way he faces, as
/// near as the floor lets (`open` says which tiles are floor).
pub fn nightmare_spots(prince: (f64, f64), facing: (f64, f64), open: impl Fn(P) -> bool) -> Vec<(f64, f64)> {
    let (fx, fy) = facing;
    let n = (fx * fx + fy * fy).sqrt().max(1e-9);
    let across = (-fy / n, fx / n);
    let mut out = Vec::new();
    for side in [-1.0, 1.0] {
        let spot = [2.4, 1.8, 3.0, 1.4].iter().map(|d| (prince.0 + across.0 * side * d, prince.1 + across.1 * side * d)).find(|p| {
            let t = P::new(p.0.round() as i32, p.1.round() as i32);
            open(t)
        });
        out.push(spot.unwrap_or(prince));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::parse_level;
    use crate::logic::pathing::reachable;
    use crate::logic::types::Tile;

    #[test]
    fn the_hall_of_stopped_clocks_is_entered_from_the_south_and_belphegor_sits_in_his_chair_to_the_north() {
        let level = parse_level(&hall_of_stopped_clocks(1));
        let g = &level.grid;
        assert_eq!((g.width, g.height), (SIZE, SIZE));
        assert_eq!(level.data.name, "The Hall of Stopped Clocks");
        assert!(level.data.boss && level.data.theme == Theme::Court(Sin::Sloth));
        // About thirty tiles across, its corners cut.
        let row: Vec<i32> = (0..SIZE).filter(|&x| g.get(x, CENTRE.1 + 2) != Tile::Rock).collect();
        assert!(row.len() >= 25 && row.len() <= 30, "{} across", row.len());
        assert!(g.get(CENTRE.0 - HALF, CENTRE.1 - HALF) == Tile::Rock, "a corner cut off");
        // The gate in the south wall, him just inside it, Belphegor before his chair to the north.
        assert_eq!(level.gate, Some(P::new(GATE.0, GATE.1)));
        assert_eq!(level.hero_spawn, P::new(HERO.0, HERO.1));
        assert!(level.hero_spawn.y < GATE.1 && GATE.1 - level.hero_spawn.y <= 4);
        assert_eq!(level.enemy_spawn, P::new(PRINCE.0, PRINCE.1));
        assert!(CHAIR.3 < PRINCE.1 && PRINCE.1 - CHAIR.3 <= 2 && (CHAIR.0..=CHAIR.2).contains(&PRINCE.0), "his chair right behind him");
        // The hourglass on the floor before the chair, near him.
        assert!(HOURGLASS.1 > CHAIR.3 && (HOURGLASS.0 - PRINCE.0).abs() + (HOURGLASS.1 - PRINCE.1).abs() <= 6);
        assert_eq!(piece(HOURGLASS.0, HOURGLASS.1), Some(Piece::Hourglass));
        // Every open tile can be walked to, and every piece stands on the hall's floor.
        let seen = reachable(g, level.hero_spawn, false, false);
        for (x, y, t) in g.cells() {
            if t != Tile::Rock {
                assert!(seen.contains(&P::new(x, y)), "{x},{y} cut off");
            }
        }
        for &(x, y) in CLOCKS.iter().chain(&POSTS).chain([&HOURGLASS]) {
            assert!(in_hall(x, y) && g.get(x, y) == Tile::Rock, "{x},{y}");
        }
        // The tall clocks stand against the walls: each has the wall at its back.
        for &(x, y) in &CLOCKS {
            let walled = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| !in_hall(x + dx, y + dy));
            assert!(walled, "{x},{y} stands out on the floor");
        }
        // The drift of sand lies on open floor, away from the way in.
        let (dx, dy, _) = DRIFT;
        assert!(g.is_walkable(dx.round() as i32, dy.round() as i32, false));
        // His nightmares have room either side of him.
        let spots = nightmare_spots((PRINCE.0 as f64, PRINCE.1 as f64), (0.0, 1.0), |p| g.is_walkable(p.x, p.y, false));
        assert_eq!(spots.len(), 2);
        assert!(spots.iter().all(|s| (s.0 - PRINCE.0 as f64).abs() > 1.0), "{spots:?}");
        // The way from him to Belphegor is open.
        assert!(seen.contains(&level.enemy_spawn));
        super::super::tests::court_is_sound(Sin::Sloth);
    }
}
