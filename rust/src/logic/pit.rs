//! The Pit (FINALE.md §1, §2): the last level, a vertical prison after the film *El hoyo*. Each of
//! its seven levels is the same square stone cell with a square hole in its middle, and in the hole
//! a stone platform that fills it exactly; the cell's floor round the platform is the ledge, and in
//! its walls are the iron doors the level's inmates come out of, and barred cells between them.
//!
//! Every level lasts two minutes. The foes of its stage (level *n* holds stage *n*) come out of the
//! doors in waves over the first hundred seconds, never more than ten standing at once; at zero the
//! platform sinks to the next level, and every foe standing on it rides down with the hero. What
//! stands on the ledge stays behind; he always goes down (from the ledge he leaps after it, and the
//! landing costs him a tenth of his health, never his life).
//!
//! Laid out and ruled here, free of rendering: the cell as a level, which tiles are the platform,
//! the doors, each level's stage, waves and light, and the rules as pure functions. `game::pit`
//! plays it, `render::view::pit` draws it.

use super::combat::{Attack, Breed};
use super::enemy::EnemyKind;
use super::hub::Rect;
use super::level::{LevelData, Theme};
use super::levels::SPEED_SCALE;
use super::types::{Dir, P, V};

/// The cell is this many tiles on a side, its walls included.
pub const SIZE: i32 = 25;
/// The platform's side, and the hole it fills.
pub const PLATFORM: i32 = 11;
/// The first tile of the hole along either axis (the hole is centred).
pub const HOLE_AT: i32 = (SIZE - PLATFORM) / 2;
/// The levels, top to bottom.
pub const LEVELS: u8 = 7;
/// Seconds a level lasts before the platform sinks.
pub const LEVEL_SECONDS: f64 = 120.0;
/// The last seconds, when the bell tolls each second and the platform's edge glows.
pub const BELL_SECONDS: f64 = 15.0;
/// After this the doors stay shut: what has not come out by then does not come.
pub const DOORS_CLOSE: f64 = 100.0;
/// The descent: a shudder as the platform lets go (he leaps after it now, if he must), then the
/// sinking itself. The fight is still all the while.
pub const SHUDDER_SECONDS: f64 = 0.7;
pub const DESCENT_SECONDS: f64 = 5.0;
/// World units from one level's floor to the next one's.
pub const LEVEL_DEPTH: f32 = 9.0;
/// The most foes standing in the cell at once (those that rode down count too).
pub const MAX_STANDING: usize = 10;
/// The share of his health the leap after the platform costs him.
pub const LEAP_SHARE: f64 = 0.1;
/// Seconds into the level that each wave's doors open.
pub const WAVE_TIMES: [f64; 4] = [3.0, 26.0, 50.0, 74.0];

/// The platform's tiles: the hole in the middle of the cell.
pub const fn platform() -> Rect {
    Rect { x0: HOLE_AT, y0: HOLE_AT, x1: HOLE_AT + PLATFORM - 1, y1: HOLE_AT + PLATFORM - 1 }
}

/// The long table across the middle of the platform: stone and solid, the feast laid on it.
pub const fn table() -> Rect {
    let mid = HOLE_AT + PLATFORM / 2;
    Rect { x0: mid - 3, y0: mid, x1: mid + 3, y1: mid }
}

/// Is the tile part of the platform?
pub fn on_platform_tile(p: P) -> bool {
    platform().contains(p)
}

/// Is a body standing at `p` (in tiles, free movement) on the platform? Its tiles reach half a tile
/// either way of their centres.
pub fn on_platform(p: V) -> bool {
    let r = platform();
    let (lo, hi) = (r.x0 as f64 - 0.5, r.x1 as f64 + 0.5);
    let (lo_y, hi_y) = (r.y0 as f64 - 0.5, r.y1 as f64 + 0.5);
    p.x >= lo && p.x <= hi && p.y >= lo_y && p.y <= hi_y
}

/// Is the tile inside the walls (the ledge or the platform)?
pub fn inside(p: P) -> bool {
    p.x >= 1 && p.y >= 1 && p.x <= SIZE - 2 && p.y <= SIZE - 2
}

/// Is the tile the ledge: inside the walls, off the platform?
pub fn on_ledge(p: P) -> bool {
    inside(p) && !on_platform_tile(p)
}

/// An iron door in a wall: its tile in the wall, the way it opens (into the cell), and the ledge
/// tile before it, where its inmates step out.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Door {
    pub tile: P,
    pub facing: Dir,
}

impl Door {
    pub fn front(&self) -> P {
        let (dx, dy) = self.facing.vec();
        P::new(self.tile.x + dx, self.tile.y + dy)
    }

    /// The ledge tiles a pack spreads over as it comes out: the one before the door, and either side
    /// along the wall.
    pub fn mouth(&self) -> [P; 3] {
        let f = self.front();
        let (dx, dy) = self.facing.vec();
        [f, P::new(f.x - dy, f.y - dx), P::new(f.x + dy, f.y + dx)]
    }
}

/// Along each wall, where its two doors stand (from its west or north end); its barred cells stand
/// at `CELLS`, and the level's number is carved in the north wall's middle.
pub const DOOR_AT: [i32; 2] = [6, 18];
pub const CELLS: [i32; 4] = [3, 9, 15, 21];

/// The eight doors: north, east, south and west walls, two each.
pub fn doors() -> [Door; 8] {
    let last = SIZE - 1;
    std::array::from_fn(|i| {
        let along = DOOR_AT[i % 2];
        match i / 2 {
            0 => Door { tile: P::new(along, 0), facing: Dir::Down },
            1 => Door { tile: P::new(last, along), facing: Dir::Left },
            2 => Door { tile: P::new(along, last), facing: Dir::Up },
            _ => Door { tile: P::new(0, along), facing: Dir::Right },
        }
    })
}

/// The barred cells in the walls (scenery): their tiles, and the way each looks into the cell.
pub fn cells() -> Vec<(P, Dir)> {
    let last = SIZE - 1;
    let mut out = Vec::new();
    for &a in &CELLS {
        out.push((P::new(a, 0), Dir::Down));
        out.push((P::new(last, a), Dir::Left));
        out.push((P::new(a, last), Dir::Up));
        out.push((P::new(0, a), Dir::Right));
    }
    out
}

/// Where he comes in at the first level: on the ledge at the platform's south lip, facing it.
pub fn arrival() -> P {
    P::new(SIZE / 2, platform().y1 + 2)
}

/// Where he leaps to from the lip as the first level begins, and where he rises after a fall: on
/// the platform, south of the table.
pub fn landing() -> P {
    P::new(SIZE / 2, platform().y1 - 1)
}

/// The green cross laid on the platform as each level begins: north of the table.
pub fn cross() -> P {
    P::new(SIZE / 2, platform().y0 + 2)
}

/**
 * The cell as a level: rock walls round a floor ('#' and ' '), the table on the platform rock, he
 * arrives on the lip ('P'), and every level names a spawn ('G': here the middle of the north ledge,
 * though the inmates come out of the doors). The grid is the same at every level.
 */
pub fn cell() -> LevelData {
    let n = SIZE as usize;
    let mut rows = vec![vec!['#'; n]; n];
    for y in 1..SIZE - 1 {
        for x in 1..SIZE - 1 {
            rows[y as usize][x as usize] = ' ';
        }
    }
    for p in table().tiles() {
        rows[p.y as usize][p.x as usize] = '#';
    }
    let a = arrival();
    rows[a.y as usize][a.x as usize] = 'P';
    rows[2][(SIZE / 2) as usize] = 'G';
    LevelData {
        name: "The Pit".to_string(),
        rows: rows.iter().map(|r| r.iter().collect()).collect(),
        heights: vec!["0".repeat(n); n],
        hero_speed: 6.5 * SPEED_SCALE,
        enemy_speed: 5.4 * SPEED_SCALE,
        frenzy_seconds: 7.0,
        boss: false,
        theme: Theme::Pit,
    }
}

// ------------------------------------------------------------------ the levels

/// The stage of foe level `level` (1 to 7) holds: its own number.
pub fn stage_of(level: u8) -> u8 {
    level.clamp(1, LEVELS)
}

/// What the old prisoners call each level, by the foes it holds.
pub fn stage_name(level: u8) -> &'static str {
    match stage_of(level) {
        1 => "THE RESTLESS DEAD",
        2 => "THE WITCH'S BROOD",
        3 => "THE DEMONS OF THE MOUNTAIN",
        4 => "THE YOKAI",
        5 => "THE ANATOLIAN NIGHT",
        6 => "THE LEGENDS",
        _ => "RAGNAROK",
    }
}

/// The colour of each level's light, after its stage: the graveyard's pale green, the witch's
/// violet, the mountain's red, foxfire blue, the night's deep teal, legend's gold, the end's fire.
pub fn light(level: u8) -> u32 {
    match stage_of(level) {
        1 => 0x9ad8a0,
        2 => 0xa88aff,
        3 => 0xff5a4a,
        4 => 0x6ab8ff,
        5 => 0x3ad0c0,
        6 => 0xffc04a,
        _ => 0xff6a2a,
    }
}

/// How much of the feast on the table is left at each level: all of it at the first, nothing but
/// bones and an overturned cup at the seventh.
pub fn feast_left(level: u8) -> f32 {
    1.0 - (stage_of(level) - 1) as f32 / (LEVELS - 1) as f32
}

/// One foe coming out of a door: when (seconds into the level), which door, what, and its stage.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Arrival {
    pub at: u32,
    pub door: usize,
    pub kind: EnemyKind,
    pub stage: u8,
}

/// Which two doors each wave comes out of: a pair on facing walls, a different pair each time.
const WAVE_DOORS: [[usize; 2]; 4] = [[0, 5], [2, 7], [1, 4], [3, 6]];

/// How many foes a pack of level `level` holds: three, one fighting with its body, one in melee,
/// one from afar.
pub fn pack_size(_level: u8) -> usize {
    3
}

/**
 * Every foe of level `level`, as it comes: four waves, each out of two doors on facing walls, each
 * door's pack mixing the three ways of fighting (dealt by turns, as the realms deal theirs). The
 * same every time.
 */
pub fn arrivals(level: u8) -> Vec<Arrival> {
    let stage = stage_of(level);
    let kinds: Vec<EnemyKind> = EnemyKind::of_stage(stage).iter().copied().filter(|&k| k != EnemyKind::Minion).collect();
    let of_way = |way: Attack| -> Vec<EnemyKind> { kinds.iter().copied().filter(|&k| Breed::of(k).attack() == way).collect() };
    let ways = [Attack::Body, Attack::Melee, Attack::Distance];
    let queues: Vec<Vec<EnemyKind>> = ways.iter().map(|&w| of_way(w)).collect();
    let mut dealt = [0usize; 3];
    let mut turn = 0usize;
    let mut draw = || {
        for _ in 0..3 {
            let k = turn % 3;
            turn += 1;
            if !queues[k].is_empty() {
                let f = queues[k][dealt[k] % queues[k].len()];
                dealt[k] += 1;
                return f;
            }
        }
        kinds[turn % kinds.len()]
    };
    let mut out = Vec::new();
    for (w, &at) in WAVE_TIMES.iter().enumerate() {
        for &door in &WAVE_DOORS[w] {
            for _ in 0..pack_size(level) {
                out.push(Arrival { at: at as u32, door, kind: draw(), stage });
            }
        }
    }
    out
}

/// Seconds left on the level's clock (never below zero).
pub fn time_left(clock: f64) -> f64 {
    (LEVEL_SECONDS - clock).max(0.0)
}

/// The clock as the HUD shows it: whole minutes and seconds left, rounded up (so it reads 0:00 only
/// at zero).
pub fn mmss(clock: f64) -> String {
    let left = time_left(clock).ceil() as u32;
    format!("{}:{:02}", left / 60, left % 60)
}

/// Does the bell toll between `before` and `after` (seconds on the clock)? Once for each whole
/// second of the last fifteen that the step crosses.
pub fn tolls(before: f64, after: f64) -> bool {
    let (a, b) = (time_left(before), time_left(after));
    b < BELL_SECONDS && a.ceil() != b.ceil() && b > 0.0
}

/// What the leap after the sinking platform costs him: a tenth of his health, rounded up, but never
/// his last point.
pub fn leap_cost(hp: i32, max_hp: i32) -> i32 {
    let cost = (max_hp as f64 * LEAP_SHARE).ceil() as i32;
    cost.min(hp - 1).max(0)
}

/// How far below its level the platform has sunk `t` seconds into the descent (0 through the shudder,
/// then eased down to a whole level).
pub fn sunk(t: f64) -> f32 {
    let k = ((t - SHUDDER_SECONDS) / (DESCENT_SECONDS - SHUDDER_SECONDS)).clamp(0.0, 1.0) as f32;
    let e = k * k * (3.0 - 2.0 * k);
    e * LEVEL_DEPTH
}

/// The level's name for the HUD and the floaters: "LEVEL 3" and so on.
pub fn level_label(level: u8) -> String {
    format!("LEVEL {}", stage_of(level))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::parse_level;
    use crate::logic::pathing::reachable;
    use crate::logic::types::Tile;

    #[test]
    fn the_platform_fills_the_hole_and_the_ledge_runs_round_it_inside_the_walls() {
        let level = parse_level(&cell());
        let g = &level.grid;
        assert_eq!((g.width, g.height), (SIZE, SIZE));
        let r = platform();
        assert_eq!((r.x1 - r.x0 + 1, r.y1 - r.y0 + 1), (PLATFORM, PLATFORM));
        // Centred: as many ledge tiles on every side.
        assert_eq!(r.x0 - 1, SIZE - 2 - r.x1);
        assert_eq!(r.y0 - 1, SIZE - 2 - r.y1);
        let ledge_width = r.x0 - 1;
        assert!((5..=6).contains(&ledge_width), "a ledge five or six tiles wide, not {ledge_width}");
        for (x, y, t) in g.cells() {
            let p = P::new(x, y);
            if !inside(p) {
                assert_eq!(t, Tile::Rock, "the wall at {p:?}");
            } else if table().contains(p) {
                assert_eq!(t, Tile::Rock, "the table at {p:?}");
                assert!(on_platform_tile(p));
            } else {
                assert!(g.is_walkable(x, y, false), "open floor at {p:?}");
                assert_ne!(on_platform_tile(p), on_ledge(p), "{p:?} is one or the other");
            }
        }
        // The platform's edge meets the ledge with no gap: every tile round it is ledge.
        for x in r.x0 - 1..=r.x1 + 1 {
            assert!(on_ledge(P::new(x, r.y0 - 1)) && on_ledge(P::new(x, r.y1 + 1)));
        }
        // A body on a platform tile, or half a tile over its edge, is on it; a step further is not.
        assert!(on_platform(V::new(r.x0 as f64 - 0.45, r.y0 as f64)));
        assert!(!on_platform(V::new(r.x0 as f64 - 0.6, r.y0 as f64)));
        assert!(on_platform(landing().v()) && on_platform(cross().v()));
        assert!(!on_platform(arrival().v()) && on_ledge(arrival()));
        assert_eq!(level.hero_spawn, arrival());
        assert_eq!(level.data.theme, Theme::Pit);
    }

    #[test]
    fn every_open_tile_can_be_reached_and_every_door_opens_onto_the_ledge() {
        let level = parse_level(&cell());
        let g = &level.grid;
        let seen = reachable(g, arrival(), false, false);
        for (x, y, _) in g.cells() {
            if g.is_walkable(x, y, false) {
                assert!(seen.contains(&P::new(x, y)), "{x},{y} cannot be reached");
            }
        }
        let doors = doors();
        for d in doors {
            assert!(!inside(d.tile), "{d:?} stands in the wall");
            for m in d.mouth() {
                assert!(on_ledge(m) && g.is_walkable(m.x, m.y, false), "{d:?}: its mouth {m:?} on the ledge");
            }
        }
        // Two to a wall, none sharing a tile with another or with a barred cell.
        for (i, a) in doors.iter().enumerate() {
            assert!(doors.iter().skip(i + 1).all(|b| b.tile != a.tile));
            assert!(cells().iter().all(|c| c.0 != a.tile));
        }
        for (p, _) in cells() {
            assert!(!inside(p));
        }
    }

    #[test]
    fn each_level_brings_twenty_to_thirty_of_its_own_stage_in_four_waves_over_the_first_hundred_seconds() {
        for level in 1..=LEVELS {
            let a = arrivals(level);
            assert!((20..=30).contains(&a.len()), "level {level}: {} foes", a.len());
            assert_eq!(a, arrivals(level), "the same every time");
            assert!(a.iter().all(|x| x.stage == level && EnemyKind::of_stage(level).contains(&x.kind) && x.kind != EnemyKind::Minion));
            assert!(a.iter().all(|x| (x.at as f64) < DOORS_CLOSE - 10.0), "level {level}: room to come out before the doors shut");
            let mut times: Vec<u32> = a.iter().map(|x| x.at).collect();
            times.dedup();
            assert!(times.len() >= 4, "level {level}: in waves");
            // Every kind of the stage comes, and each door's pack fights all three ways (where the
            // stage has them).
            for &k in EnemyKind::of_stage(level).iter().filter(|&&k| k != EnemyKind::Minion) {
                assert!(a.iter().any(|x| x.kind == k), "level {level}: no {k:?}");
            }
            if pack_size(level) == 3 {
                for chunk in a.chunks(3) {
                    let ways: Vec<Attack> = chunk.iter().map(|x| Breed::of(x.kind).attack()).collect();
                    assert!(ways.contains(&Attack::Body) && ways.contains(&Attack::Melee) && ways.contains(&Attack::Distance), "{chunk:?}");
                }
            }
            assert!(a.iter().all(|x| x.door < 8));
        }
        assert!(feast_left(1) == 1.0 && feast_left(7) == 0.0 && feast_left(4) < feast_left(3));
        assert_eq!(stage_name(7), "RAGNAROK");
    }

    #[test]
    fn the_clock_reads_down_and_the_bell_tolls_each_of_the_last_fifteen_seconds() {
        assert_eq!(mmss(0.0), "2:00");
        assert_eq!(mmss(0.4), "2:00");
        assert_eq!(mmss(61.0), "0:59");
        assert_eq!(mmss(119.9), "0:01");
        assert_eq!(mmss(120.0), "0:00");
        assert_eq!(mmss(130.0), "0:00");
        let step = 1.0 / 120.0;
        let mut t = 0.0;
        let mut rung = Vec::new();
        while t < LEVEL_SECONDS {
            if tolls(t, t + step) {
                rung.push(time_left(t + step).ceil() as u32);
            }
            t += step;
        }
        assert_eq!(rung, (1..=15).rev().collect::<Vec<_>>(), "once a second, from 0:15 down to 0:01");
    }

    #[test]
    fn the_leap_after_the_platform_costs_a_tenth_of_his_health_and_never_his_life() {
        assert_eq!(leap_cost(200, 200), 20);
        assert_eq!(leap_cost(150, 155), 16, "rounded up");
        assert_eq!(leap_cost(12, 200), 11, "never his last point");
        assert_eq!(leap_cost(1, 200), 0);
        assert_eq!(sunk(0.0), 0.0);
        assert_eq!(sunk(SHUDDER_SECONDS), 0.0);
        assert!((sunk(DESCENT_SECONDS) - LEVEL_DEPTH).abs() < 1e-5);
        assert!(sunk(2.5) > 0.0 && sunk(2.5) < LEVEL_DEPTH);
    }
}
