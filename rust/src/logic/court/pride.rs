//! Pride's court, built to its own plan: **the Throne of the Morning Star**, where Lucifer waits.
//!
//! A round hall about thirty tiles across, floored in white marble veined with gold and polished
//! to a mirror. Six tall mirrors in gilt frames stand round its wall, three either side; in the
//! north, high steps go up to an empty throne whose back is the seventh mirror, under a great
//! window shaped like a star, and shafts of light fall from it onto the floor. Nothing stands on
//! the floor: it is his stage, and his lance of light crosses it from wall to wall. The hero comes
//! in from the south, and the gate he came through locks behind him: the prince's key opens it.
//! (The art is `render::view::court_pride`; the fight, `game::court` and `game::court::pride`.)

use crate::logic::level::{LevelData, Theme};
use crate::logic::levels::level_for;
use crate::logic::sins::Sin;
use crate::logic::types::{P, V};

/// The Throne of the Morning Star is built: Lucifer fights in it as himself.
pub const BUILT: bool = true;
/// The court's name.
pub const NAME: &str = "The Throne of the Morning Star";
/// What Lucifer raises against the hero.
pub const ADDS: &str = "reflections";

/// The grid's side, and its middle.
pub const SIZE: i32 = 33;
pub const CENTRE: (i32, i32) = (16, 16);
/// The round floor: no farther than this from the middle (squared, in tiles).
pub const RADIUS_SQ: i32 = 204;
/// The gate in the south wall, and where he comes in, just inside it.
pub const GATE: (i32, i32) = (16, 31);
pub const HERO: (i32, i32) = (16, 27);
/// Where Lucifer stands as the fight begins: at the foot of his steps.
pub const LUCIFER: (i32, i32) = (16, 8);
/// Where the prince stands as the fight begins (Lucifer, at the foot of his steps).
pub const PRINCE: (i32, i32) = LUCIFER;
/// The high steps up to the throne (inclusive, x0..=x1, y0..=y1, as far as the round wall lets
/// them run): a block against the north wall.
pub const STEPS: (i32, i32, i32, i32) = (11, 2, 21, 5);
/// The empty throne at the top of the steps, its back the seventh mirror, and the star window
/// high in the wall over it (points, for the art).
pub const THRONE: (f64, f64) = (16.0, 2.6);
pub const WINDOW: (f64, f64) = (16.0, 0.9);
/// The six tall mirrors round the wall, three either side: each stands on a tile of the floor's
/// edge, facing the middle.
pub const MIRRORS: [(i32, i32); 6] = [(27, 7), (30, 17), (25, 27), (7, 27), (2, 17), (5, 7)];
/// How far before a mirror (toward the middle) one who steps out of it stands.
pub const MIRROR_STEP_OUT: f64 = 2.0;
/// The shafts of light from the star window: where each falls on the floor and how wide.
pub const SHAFTS: [(f64, f64, f64); 5] = [(16.0, 13.0, 2.4), (11.5, 18.0, 1.4), (20.5, 18.0, 1.4), (13.0, 23.0, 1.1), (19.0, 23.0, 1.1)];

/// Is (x, y) inside the round floor?
pub fn in_hall(x: i32, y: i32) -> bool {
    let (dx, dy) = (x - CENTRE.0, y - CENTRE.1);
    dx * dx + dy * dy <= RADIUS_SQ
}

/// What stands on the floor at (x, y), by the plan (none: open floor or wall).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Piece {
    Mirror,
    Steps,
}

/// The piece standing at (x, y), if any.
pub fn piece(x: i32, y: i32) -> Option<Piece> {
    if MIRRORS.contains(&(x, y)) {
        Some(Piece::Mirror)
    } else if (STEPS.0..=STEPS.2).contains(&x) && (STEPS.1..=STEPS.3).contains(&y) && in_hall(x, y) {
        Some(Piece::Steps)
    } else {
        None
    }
}

/// The court as a level, as hard as dungeon `tier`: the Throne of the Morning Star.
pub fn level(tier: usize) -> LevelData {
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
    rows[LUCIFER.1 as usize][LUCIFER.0 as usize] = 'G';
    LevelData {
        name: NAME.to_string(),
        rows: rows.iter().map(|r| r.iter().collect()).collect(),
        heights: vec!["0".repeat(SIZE as usize); SIZE as usize],
        hero_speed: base.hero_speed,
        enemy_speed: base.enemy_speed,
        frenzy_seconds: base.frenzy_seconds,
        boss: true,
        theme: Theme::Court(Sin::Pride),
    }
}

/// The middle of the hall, as a point.
pub fn centre() -> V {
    P::new(CENTRE.0, CENTRE.1).v()
}

/// Where one who steps out of the mirror at `m` stands: before it, toward the middle.
pub fn before_mirror(m: (i32, i32)) -> V {
    let at = P::new(m.0, m.1).v();
    let c = centre();
    let (dx, dy) = (c.x - at.x, c.y - at.y);
    let l = (dx * dx + dy * dy).sqrt().max(1e-9);
    V::new(at.x + dx / l * MIRROR_STEP_OUT, at.y + dy / l * MIRROR_STEP_OUT)
}

/// Every place one may step out of a mirror: before each of the six round the wall, and at the
/// foot of the steps (the throne's mirror).
pub fn mirror_doors() -> [V; 7] {
    let mut out = [P::new(LUCIFER.0, LUCIFER.1 - 1).v(); 7];
    for (k, &m) in MIRRORS.iter().enumerate() {
        out[k] = before_mirror(m);
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
    fn the_throne_of_the_morning_star_is_a_round_hall_entered_from_the_south() {
        let level = parse_level(&level(1));
        let g = &level.grid;
        assert_eq!((g.width, g.height), (SIZE, SIZE));
        assert_eq!(level.data.name, "The Throne of the Morning Star");
        assert!(level.data.boss && level.data.theme == Theme::Court(Sin::Pride));
        // About thirty tiles across, and round: its corners are wall.
        let row: Vec<i32> = (0..SIZE).filter(|&x| g.get(x, CENTRE.1) != Tile::Rock).collect();
        assert!(row.len() >= 26 && row.len() <= 31, "{} across", row.len());
        for (x, y) in [(3, 3), (29, 3), (3, 29), (29, 29)] {
            assert_eq!(g.get(x, y), Tile::Rock, "{x},{y}: a round hall has no corners");
        }
        // The gate in the south wall, him just inside it, Lucifer at the foot of his steps to the north.
        assert_eq!(level.gate, Some(P::new(GATE.0, GATE.1)));
        assert_eq!(level.hero_spawn, P::new(HERO.0, HERO.1));
        assert!(level.hero_spawn.y < GATE.1 && GATE.1 - level.hero_spawn.y <= 4);
        assert_eq!(level.enemy_spawn, P::new(LUCIFER.0, LUCIFER.1));
        assert!(
            STEPS.3 < LUCIFER.1 && (THRONE.1 as i32) <= STEPS.3 && WINDOW.1 < THRONE.1,
            "the steps behind him, the throne up them, the window over it"
        );
        // Every open tile can be walked to; the steps and the mirrors stand on the hall's floor.
        let seen = reachable(g, level.hero_spawn, false, false);
        for (x, y, t) in g.cells() {
            if t != Tile::Rock {
                assert!(seen.contains(&P::new(x, y)), "{x},{y} cut off");
            }
        }
        assert_eq!(MIRRORS.len(), 6, "six round the wall, the seventh the throne's");
        for (x, y) in MIRRORS {
            assert!(in_hall(x, y) && g.get(x, y) == Tile::Rock, "{x},{y}");
            // On the floor's edge: the next tile out is the wall.
            let (dx, dy) = ((x - CENTRE.0).signum(), (y - CENTRE.1).signum());
            assert!(!in_hall(x + dx, y) || !in_hall(x, y + dy), "{x},{y} stands against the wall");
        }
        assert_eq!(g.get(16, 4), Tile::Rock, "the steps");
        // Before every mirror there is floor to step out onto, open all round.
        for door in mirror_doors() {
            let p = P::new(door.x.round() as i32, door.y.round() as i32);
            for (dx, dy) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
                assert!(g.is_walkable(p.x + dx, p.y + dy, false), "before a mirror at {door:?}");
            }
        }
        // Nothing stands between him and Lucifer: the floor is open, for the lance.
        for y in LUCIFER.1..HERO.1 {
            assert!(g.is_walkable(CENTRE.0, y, false));
        }
        assert!(SHAFTS.iter().all(|s| in_hall(s.0 as i32, s.1 as i32)));
        super::super::tests::court_is_sound(Sin::Pride);
    }
}
