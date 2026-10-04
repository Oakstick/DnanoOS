//! Lust's court, built to its own plan: **the Rose Bower**, where Asmodeus waits.
//!
//! A walled garden about thirty tiles across, its walls black thorn hedges grown thick with crimson
//! roses, its corners cut by rose beds. A marble fountain plays in the middle of it; two runs of
//! thorn hedge stand out from the east and west walls (his elf-maids step out from behind them),
//! and four bronze censers stand on plinths round the floor, cold (Tobit: the smoke that drove him
//! off; a blow lights one). At the north end the bridal arch, wound with roses, stands over the
//! bridal bed, and Asmodeus waits before it. The hero comes in from the south, and the gate he came
//! through locks behind him: the prince's key opens it. (The art is `render::view::court_lust`; the
//! fight, `game::court` and `game::court::lust`.)

use crate::logic::level::{LevelData, Theme};
use crate::logic::levels::level_for;
use crate::logic::sins::Sin;
use crate::logic::types::P;

/// The Rose Bower is built: Asmodeus fights in it as himself.
pub const BUILT: bool = true;
/// The court's name.
pub const NAME: &str = "The Rose Bower";
/// What Asmodeus raises against the hero.
pub const ADDS: &str = "elf-maids";

/// The grid's side, and its middle.
pub const SIZE: i32 = 33;
pub const CENTRE: (i32, i32) = (16, 16);
/// The garden's floor: no farther than this from the middle along either axis, nor than `CORNER`
/// along both together (the corners are rose beds behind the hedge).
pub const HALF: i32 = 13;
pub const CORNER: i32 = 22;
/// The gate in the south hedge, and where he comes in, just inside it.
pub const GATE: (i32, i32) = (16, 30);
pub const HERO: (i32, i32) = (16, 26);
/// Where Asmodeus stands as the fight begins: before the bridal arch.
pub const ASMODEUS: (i32, i32) = (16, 8);
/// Where the prince stands as the fight begins.
pub const PRINCE: (i32, i32) = ASMODEUS;
/// The bridal arch's two posts, wound with roses (the way under it stays open), and the bridal
/// bed behind it against the north hedge (x0..=x1 on row y).
pub const ARCH: [(i32, i32); 2] = [(14, 4), (18, 4)];
pub const BED: (i32, i32, i32) = (15, 17, 3);
/// The fountain in the middle: its basin covers the tiles this near its heart.
pub const FOUNTAIN: f64 = 2.2;
/// The four bronze censers on their plinths, round the floor.
pub const CENSERS: [(i32, i32); 4] = [(9, 11), (23, 11), (9, 21), (23, 21)];
/// The runs of thorn hedge standing out from the walls (x, y0..=y1): the elf-maids step out from
/// behind them.
pub const HEDGES: [(i32, i32, i32); 2] = [(5, 13, 19), (27, 13, 19)];
/// Rose beds standing on the floor near the corners.
pub const ROSES: [(i32, i32); 4] = [(6, 6), (26, 6), (6, 26), (26, 26)];

/// Is (x, y) inside the garden's floor?
pub fn in_garden(x: i32, y: i32) -> bool {
    let (dx, dy) = ((x - CENTRE.0).abs(), (y - CENTRE.1).abs());
    dx <= HALF && dy <= HALF && dx + dy <= CORNER
}

/// Is (x, y) under the fountain's basin?
pub fn in_fountain(x: i32, y: i32) -> bool {
    let (dx, dy) = ((x - CENTRE.0) as f64, (y - CENTRE.1) as f64);
    dx * dx + dy * dy <= FOUNTAIN * FOUNTAIN
}

/// What stands on the floor at (x, y), by the plan (none: open floor or the hedge wall).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Piece {
    Fountain,
    Censer,
    Hedge,
    Roses,
    Arch,
    Bed,
}

/// The piece standing at (x, y), if any.
pub fn piece(x: i32, y: i32) -> Option<Piece> {
    let at = (x, y);
    if in_fountain(x, y) {
        Some(Piece::Fountain)
    } else if CENSERS.contains(&at) {
        Some(Piece::Censer)
    } else if HEDGES.iter().any(|&(hx, y0, y1)| x == hx && (y0..=y1).contains(&y)) {
        Some(Piece::Hedge)
    } else if ROSES.contains(&at) {
        Some(Piece::Roses)
    } else if ARCH.contains(&at) {
        Some(Piece::Arch)
    } else if y == BED.2 && (BED.0..=BED.1).contains(&x) {
        Some(Piece::Bed)
    } else {
        None
    }
}

/// The court as a level, as hard as dungeon `tier`: the Rose Bower.
pub fn level(tier: usize) -> LevelData {
    rose_bower(tier)
}

/// The Rose Bower as a level, as hard as dungeon `tier`.
pub fn rose_bower(tier: usize) -> LevelData {
    let base = level_for(tier);
    let mut rows = vec![vec!['#'; SIZE as usize]; SIZE as usize];
    for (y, row) in rows.iter_mut().enumerate() {
        for (x, c) in row.iter_mut().enumerate() {
            let (x, y) = (x as i32, y as i32);
            if in_garden(x, y) && piece(x, y).is_none() {
                *c = ' ';
            }
        }
    }
    rows[GATE.1 as usize][GATE.0 as usize] = 'D';
    rows[HERO.1 as usize][HERO.0 as usize] = 'P';
    rows[ASMODEUS.1 as usize][ASMODEUS.0 as usize] = 'G';
    LevelData {
        name: NAME.to_string(),
        rows: rows.iter().map(|r| r.iter().collect()).collect(),
        heights: vec!["0".repeat(SIZE as usize); SIZE as usize],
        hero_speed: base.hero_speed,
        enemy_speed: base.enemy_speed,
        frenzy_seconds: base.frenzy_seconds,
        boss: true,
        theme: Theme::Court(Sin::Lust),
    }
}

/// Where an elf-maid steps out from behind a hedge: beside its middle, on the garden's side.
pub fn beside_hedge(hedge: (i32, i32, i32)) -> P {
    let (x, y0, y1) = hedge;
    P::new(x + (CENTRE.0 - x).signum(), (y0 + y1) / 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::parse_level;
    use crate::logic::pathing::reachable;
    use crate::logic::types::Tile;

    #[test]
    fn the_rose_bower_is_a_walled_garden_entered_from_the_south_with_the_gate_behind_him() {
        let level = parse_level(&rose_bower(1));
        let g = &level.grid;
        assert_eq!((g.width, g.height), (SIZE, SIZE));
        assert_eq!(level.data.name, "The Rose Bower");
        assert!(level.data.boss && level.data.theme == Theme::Court(Sin::Lust));
        // About thirty tiles across, its corners cut.
        let row: Vec<i32> = (0..SIZE).filter(|&x| g.get(x, CENTRE.1 - 4) != Tile::Rock).collect();
        assert!(row.len() >= 25 && row.len() <= 30, "{} across", row.len());
        assert!(g.get(CENTRE.0 - HALF, CENTRE.1 - HALF) == Tile::Rock, "a corner cut off");
        // The gate in the south hedge, him just inside it, Asmodeus before the arch to the north.
        assert_eq!(level.gate, Some(P::new(GATE.0, GATE.1)));
        assert_eq!(level.hero_spawn, P::new(HERO.0, HERO.1));
        assert!(level.hero_spawn.y < GATE.1 && GATE.1 - level.hero_spawn.y <= 4);
        assert_eq!(level.enemy_spawn, P::new(ASMODEUS.0, ASMODEUS.1));
        assert!(ARCH.iter().all(|a| a.1 < ASMODEUS.1) && BED.2 < ARCH[0].1, "the arch behind him, the bed behind it");
        assert!(g.is_walkable(16, ARCH[0].1, false), "the way under the arch is open");
        // The fountain in the middle stands between him and the prince.
        assert_eq!(g.get(CENTRE.0, CENTRE.1), Tile::Rock);
        assert!(in_fountain(CENTRE.0, CENTRE.1 + 2) && !in_fountain(CENTRE.0, CENTRE.1 + 3));
        // Every open tile can be walked to, and every piece stands on the garden's floor.
        let seen = reachable(g, level.hero_spawn, false, false);
        for (x, y, t) in g.cells() {
            if t != Tile::Rock {
                assert!(seen.contains(&P::new(x, y)), "{x},{y} cut off");
            }
        }
        for &(x, y) in CENSERS.iter().chain(&ROSES).chain(&ARCH) {
            assert!(in_garden(x, y) && g.get(x, y) == Tile::Rock, "{x},{y}");
        }
        // Four censers round the floor, each with open ground about it to stand and strike it from,
        // none nearer than six tiles to another (each one's smoke a place of its own).
        assert_eq!(CENSERS.len(), 4);
        for (i, &(x, y)) in CENSERS.iter().enumerate() {
            let open = [(0, 1), (1, 0), (0, -1), (-1, 0)].iter().filter(|(dx, dy)| g.is_walkable(x + dx, y + dy, false)).count();
            assert_eq!(open, 4, "censer {i}: open all round");
            for &(ox, oy) in &CENSERS[i + 1..] {
                assert!((((x - ox).pow(2) + (y - oy).pow(2)) as f64).sqrt() >= 6.0);
            }
        }
        // An elf-maid has room to step out beside each hedge.
        for h in HEDGES {
            let p = beside_hedge(h);
            assert!(g.is_walkable(p.x, p.y, false), "beside {h:?}");
        }
        assert!(seen.contains(&level.enemy_spawn));
        super::super::tests::court_is_sound(Sin::Lust);
    }
}
