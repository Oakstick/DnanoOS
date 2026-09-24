use std::collections::HashSet;

use super::level::LevelData;
use super::rng::Rng;
use super::types::PotionKind;

/// Everyone moves at this fraction of the original tuning.
pub const SPEED_SCALE: f64 = 0.8;
/// Gold nuggets scattered through each dungeon; collecting them all wins.
pub const NUGGETS_PER_LEVEL: usize = 25;
/// Potion bottles per dungeon: one extra life, two shields, two invisibility.
pub const POTION_PLAN: [PotionKind; 5] =
    [PotionKind::Life, PotionKind::Shield, PotionKind::Shield, PotionKind::Invisible, PotionKind::Invisible];
/// Pits cut across corridors that must be jumped.
pub const PITS_PER_LEVEL: usize = 6;
/// Manhattan distance kept between two pits.
const MIN_PIT_SPACING: i32 = 6;
/// Tiles per authored cell: corridors are this many tiles wide.
pub const WORLD_SCALE: usize = 1;

/**
 * 14 x 12 base quadrant, mirrored into a 27 x 23 dungeon. Corridors are one
 * tile wide. The chamber at rows 7-10 is the enemy hollow where slain foes
 * reform, reached by the passage above its G gate.
 */
const QUADRANT: [&str; 12] = [
    "##############",
    "#o........W.o#",
    "#.##.##.##.#.#",
    "#.##.##.##.#.#",
    "-.P..........#",
    "#.##.##.##.#.#",
    "#.##.##.##.#.#",
    "#.I..##G##...#",
    "#.##.#NNN#.#.#",
    "#.##.#NNN#.#.#",
    "#o...#####..o#",
    "##############",
];

/// Rows of the shared column that open between the left and right halves.
const COLUMN_GATES: [usize; 3] = [1, 4, 10];
/// Columns of the middle row that open between the top and bottom halves.
const ROW_GATES: [usize; 4] = [1, 10, 16, 25];
/// Gates that carry a ramp; every other gate is a one-level ledge you must jump up.
const RAMP_COLUMN_GATES: [usize; 1] = [4];
const RAMP_ROW_GATES: [usize; 2] = [10, 16];

const LEVEL_TL: u32 = 0;
const LEVEL_TR: u32 = 1;
const LEVEL_BL: u32 = 1;
const LEVEL_BR: u32 = 2;

#[derive(Clone, Debug)]
pub struct Layout {
    pub rows: Vec<Vec<char>>,
    pub heights: Vec<Vec<char>>,
}

impl Layout {
    fn strings(g: &[Vec<char>]) -> Vec<String> {
        g.iter().map(|r| r.iter().collect()).collect()
    }
}

/**
 * Mirrors the quadrant right and down into a 27 x 23 dungeon. The shared
 * border column and row are punched through at a few gates. Only the top-left
 * copy keeps its hollow and spawn; the other hollows become walled clearings.
 * Shrines alternate quadrants: weapons top-left and bottom-right, items
 * top-right and bottom-left.
 */
pub fn compose_dungeon(base: &[&str]) -> Layout {
    let top: Vec<Vec<char>> = base
        .iter()
        .map(|row| {
            let chars: Vec<char> = row.chars().collect();
            let mut out = chars.clone();
            let rev: Vec<char> = chars.iter().rev().copied().collect();
            out.extend_from_slice(&rev[1..]);
            out
        })
        .collect();
    let mut grid = top.clone();
    for row in top[..top.len() - 1].iter().rev() {
        grid.push(row.clone());
    }
    let w = grid[0].len();
    let h = grid.len();
    let mid_x = (w - 1) / 2;
    let mid_y = (h - 1) / 2;

    let in_top_left = |x: usize, y: usize| x < mid_x && y < mid_y;
    let quadrant = |x: usize, y: usize| (if x < mid_x { 0 } else { 1 }) + (if y < mid_y { 0 } else { 2 });

    for y in 0..h {
        for x in 0..w {
            match grid[y][x] {
                'N' | 'G' | 'P' => {
                    if !in_top_left(x, y) {
                        grid[y][x] = '.';
                    }
                }
                'W' => {
                    let q = quadrant(x, y);
                    if q != 0 && q != 3 {
                        grid[y][x] = '.';
                    }
                }
                'I' => {
                    let q = quadrant(x, y);
                    if q != 1 && q != 2 {
                        grid[y][x] = '.';
                    }
                }
                '-' => {
                    if x == mid_x {
                        grid[y][x] = '.';
                    }
                }
                _ => {}
            }
        }
    }

    for &y in &COLUMN_GATES {
        grid[y][mid_x] = '.';
        grid[h - 1 - y][mid_x] = '.';
    }
    for &x in &ROW_GATES {
        grid[mid_y][x] = '.';
    }

    // Elevation: quadrant levels; a gate tile takes the lower side so you walk on
    // and jump (or ramp) up; tunnel mouths are ramps so the wrap works both ways.
    let level_of = |x: usize, y: usize| -> u32 {
        if x < mid_x && y < mid_y {
            return LEVEL_TL;
        }
        if x > mid_x && y < mid_y {
            return LEVEL_TR;
        }
        if x < mid_x && y > mid_y {
            return LEVEL_BL;
        }
        if x > mid_x && y > mid_y {
            return LEVEL_BR;
        }
        if x == mid_x {
            return if y < mid_y { LEVEL_TL } else { LEVEL_BL };
        }
        if x < mid_x {
            LEVEL_TL
        } else {
            LEVEL_TR
        }
    };
    let heights: Vec<Vec<char>> = (0..h)
        .map(|y| {
            (0..w)
                .map(|x| {
                    let c = grid[y][x];
                    if c == '-' {
                        return '/';
                    }
                    if x == mid_x && (RAMP_COLUMN_GATES.contains(&y) || RAMP_COLUMN_GATES.contains(&(h - 1 - y))) {
                        return '/';
                    }
                    if y == mid_y && RAMP_ROW_GATES.contains(&x) {
                        return '/';
                    }
                    char::from_digit(level_of(x, y), 10).unwrap()
                })
                .collect()
        })
        .collect();

    Layout { rows: grid, heights }
}

/**
 * Cuts pits into straight corridor segments, away from gates, the spawn, the
 * hollow, and shrines, with both banks at the same level so a two-tile jump
 * always clears them. '~' marks a pit in a left-right corridor, '^' one in an
 * up-down corridor.
 */
pub fn place_pits(layout: &Layout, count: usize, seed: u32) -> Layout {
    let mut grid = layout.rows.clone();
    let w = grid[0].len() as i32;
    let h = grid.len() as i32;
    let mid_x = (w - 1) / 2;
    let mid_y = (h - 1) / 2;
    let heights = &layout.heights;
    let level = |x: i32, y: i32| heights[y as usize][x as usize];

    #[derive(Clone, Copy)]
    struct Cand {
        x: i32,
        y: i32,
        glyph: char,
    }
    let mut candidates: Vec<Cand> = Vec::new();
    {
        let open = |x: i32, y: i32| {
            x >= 0 && y >= 0 && x < w && y < h && matches!(grid[y as usize][x as usize], '.' | ' ')
        };
        let rock = |x: i32, y: i32| x < 0 || y < 0 || x >= w || y >= h || grid[y as usize][x as usize] == '#';
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                if grid[y as usize][x as usize] != '.' {
                    continue;
                }
                if (x - mid_x).abs() <= 3 || (y - mid_y).abs() <= 3 {
                    continue;
                }
                let horizontal = open(x - 1, y)
                    && open(x + 1, y)
                    && rock(x, y - 1)
                    && rock(x, y + 1)
                    && level(x - 1, y) == level(x + 1, y)
                    && level(x - 1, y) == level(x, y);
                let vertical = open(x, y - 1)
                    && open(x, y + 1)
                    && rock(x - 1, y)
                    && rock(x + 1, y)
                    && level(x, y - 1) == level(x, y + 1)
                    && level(x, y - 1) == level(x, y);
                if horizontal {
                    candidates.push(Cand { x, y, glyph: '~' });
                } else if vertical {
                    candidates.push(Cand { x, y, glyph: '^' });
                }
            }
        }
    }

    let mut rng = Rng::new(seed.wrapping_mul(104_729).wrapping_add(3));
    rng.shuffle(&mut candidates);

    // With one-tile corridors a single pit can seal off a whole wing, so a cut is
    // only kept when every tile still walkable from the hollow stays walkable.
    let mut start = (1, 1);
    for (y, row) in grid.iter().enumerate() {
        if let Some(x) = row.iter().position(|&c| c == 'G') {
            start = (x as i32, y as i32);
        }
    }
    let flood_size = |grid: &Vec<Vec<char>>| -> usize {
        let passable = |x: i32, y: i32| {
            x >= 0 && y >= 0 && x < w && y < h && !matches!(grid[y as usize][x as usize], '#' | '~' | '^')
        };
        let mut seen = HashSet::from([start]);
        let mut stack = vec![start];
        let mut n = 0;
        while let Some((px, py)) = stack.pop() {
            n += 1;
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let q = (px + dx, py + dy);
                if seen.contains(&q) || !passable(q.0, q.1) {
                    continue;
                }
                seen.insert(q);
                stack.push(q);
            }
        }
        n
    };

    let mut chosen: Vec<Cand> = Vec::new();
    let mut reach = flood_size(&grid);
    for c in candidates {
        if chosen.len() >= count {
            break;
        }
        if !chosen.iter().all(|p| (p.x - c.x).abs() + (p.y - c.y).abs() >= MIN_PIT_SPACING) {
            continue;
        }
        let was = grid[c.y as usize][c.x as usize];
        grid[c.y as usize][c.x as usize] = c.glyph;
        let now = flood_size(&grid);
        if now + 1 == reach {
            chosen.push(c);
            reach = now;
        } else {
            grid[c.y as usize][c.x as usize] = was; // this cut would strand part of the dungeon
        }
    }
    Layout { rows: grid, heights: layout.heights.clone() }
}

/**
 * Blows the authored layout up by `scale` so every road is that many tiles
 * wide. A pit becomes a one-tile trench across the road so a jump still
 * clears it. Heights fill their block.
 */
pub fn upscale(layout: &Layout, scale: usize) -> Layout {
    let mid = scale / 2;
    let mut rows = Vec::new();
    let mut heights = Vec::new();
    for (y, row) in layout.rows.iter().enumerate() {
        for dy in 0..scale {
            let mut out = Vec::new();
            let mut hout = Vec::new();
            for (x, &c) in row.iter().enumerate() {
                let hch = layout.heights[y][x];
                for dx in 0..scale {
                    let t = match c {
                        '#' | ' ' | 'N' | '-' => c,
                        '~' => {
                            if dx == mid {
                                '~'
                            } else {
                                ' '
                            }
                        }
                        '^' => {
                            if dy == mid {
                                '~'
                            } else {
                                ' '
                            }
                        }
                        _ => {
                            if dx == mid && dy == mid {
                                c
                            } else {
                                ' '
                            }
                        }
                    };
                    out.push(t);
                    hout.push(hch);
                }
            }
            rows.push(out);
            heights.push(hout);
        }
    }
    Layout { rows, heights }
}

/**
 * Replaces the coin carpet with `count` gold nuggets. Every '.' becomes plain
 * floor, then nuggets are placed on open tiles by a seeded shuffle with a
 * minimum spacing that relaxes only if the dungeon cannot fit them all.
 */
pub fn place_nuggets(rows: &[Vec<char>], count: usize, seed: u32) -> Vec<Vec<char>> {
    let mut grid: Vec<Vec<char>> =
        rows.iter().map(|r| r.iter().map(|&c| if c == '.' { ' ' } else { c }).collect()).collect();
    let mut open: Vec<(i32, i32)> = Vec::new();
    for (y, row) in grid.iter().enumerate() {
        for (x, &c) in row.iter().enumerate() {
            if c == ' ' {
                open.push((x as i32, y as i32));
            }
        }
    }
    let mut rng = Rng::new(seed);
    rng.shuffle(&mut open);

    let mut chosen: Vec<usize> = Vec::new();
    let mut min_dist = 5;
    while chosen.len() < count && min_dist >= 0 {
        for (i, p) in open.iter().enumerate() {
            if chosen.len() >= count {
                break;
            }
            let far_enough = chosen.iter().all(|&c| {
                let q = open[c];
                (q.0 - p.0).abs() + (q.1 - p.1).abs() >= min_dist
            });
            if far_enough && !chosen.contains(&i) {
                chosen.push(i);
            }
        }
        min_dist -= 1;
    }
    for i in chosen {
        let (x, y) = open[i];
        grid[y as usize][x as usize] = '.';
    }
    grid
}

/**
 * Drops potion bottles on plain open floor, away from nuggets and each other.
 * Bottles never share a tile with anything else.
 */
pub fn place_potions(rows: &[Vec<char>], plan: &[PotionKind], seed: u32) -> Vec<Vec<char>> {
    let mut grid = rows.to_vec();
    let mut open: Vec<(i32, i32)> = Vec::new();
    let mut busy: Vec<(i32, i32)> = Vec::new();
    for (y, row) in grid.iter().enumerate() {
        for (x, &c) in row.iter().enumerate() {
            let p = (x as i32, y as i32);
            if c == ' ' {
                open.push(p);
            } else if !matches!(c, '#' | '~' | '^') {
                busy.push(p);
            }
        }
    }
    let mut rng = Rng::new(seed.wrapping_mul(7919).wrapping_add(17));
    rng.shuffle(&mut open);

    // Two separate rules: never crowd a nugget or shrine, and keep the bottles
    // well apart from each other.
    let mut chosen: Vec<usize> = Vec::new();
    let mut apart = 8;
    while chosen.len() < plan.len() && apart >= 0 {
        for (i, p) in open.iter().enumerate() {
            if chosen.len() >= plan.len() {
                break;
            }
            if chosen.contains(&i) {
                continue;
            }
            if busy.iter().any(|c| (c.0 - p.0).abs() + (c.1 - p.1).abs() < 2) {
                continue;
            }
            if chosen.iter().any(|&c| {
                let q = open[c];
                (q.0 - p.0).abs() + (q.1 - p.1).abs() < apart
            }) {
                continue;
            }
            chosen.push(i);
        }
        apart -= 1;
    }
    for (k, &i) in chosen.iter().enumerate() {
        let (x, y) = open[i];
        grid[y as usize][x as usize] = plan[k].glyph();
    }
    grid
}

/// Pits, nuggets and potions for a dungeon, seeded so each has its own layout.
pub fn decorate_dungeon(layout: &Layout, seed: u32) -> Layout {
    let pitted = place_pits(layout, PITS_PER_LEVEL, seed);
    let rows = place_potions(&place_nuggets(&pitted.rows, NUGGETS_PER_LEVEL, seed), &POTION_PLAN, seed);
    upscale(&Layout { rows, heights: pitted.heights }, WORLD_SCALE)
}

fn dungeon(name: &str, seed: u32, hero_speed: f64, enemy_speed: f64, frenzy_seconds: f64) -> LevelData {
    let layout = decorate_dungeon(&compose_dungeon(&QUADRANT), seed);
    LevelData {
        name: name.to_string(),
        rows: Layout::strings(&layout.rows),
        heights: Layout::strings(&layout.heights),
        hero_speed,
        enemy_speed,
        frenzy_seconds,
    }
}

pub fn levels() -> Vec<LevelData> {
    vec![
        dungeon("Dungeon I: Sunken Halls", 1, 6.5 * SPEED_SCALE, 5.4 * SPEED_SCALE, 7.0),
        dungeon("Dungeon II: The Cisterns", 2, 7.0 * SPEED_SCALE, 6.3 * SPEED_SCALE, 5.5),
        dungeon("Dungeon III: Bone Vaults", 3, 7.5 * SPEED_SCALE, 7.3 * SPEED_SCALE, 4.0),
    ]
}

/// Beyond the last authored level, speeds keep ramping and hazards land in new spots.
pub fn level_for(index: usize) -> LevelData {
    let all = levels();
    if index < all.len() {
        return all[index].clone();
    }
    let base = all.last().unwrap().clone();
    let extra = (index - all.len() + 1) as f64;
    let layout = decorate_dungeon(&compose_dungeon(&QUADRANT), index as u32 + 1);
    LevelData {
        name: format!("Dungeon {}: Deeper Still", index + 1),
        rows: Layout::strings(&layout.rows),
        heights: Layout::strings(&layout.heights),
        enemy_speed: (base.enemy_speed + extra * 0.3 * SPEED_SCALE).min(9.0 * SPEED_SCALE),
        hero_speed: (base.hero_speed + extra * 0.2 * SPEED_SCALE).min(9.0 * SPEED_SCALE),
        frenzy_seconds: (base.frenzy_seconds - extra * 0.5).max(2.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::parse_level;
    use crate::logic::pathing::reachable;
    use crate::logic::types::Tile;

    #[test]
    fn dungeon_is_27_by_23_with_25_nuggets_and_5_potions() {
        let data = level_for(0);
        assert_eq!(data.rows.len(), 23);
        assert!(data.rows.iter().all(|r| r.chars().count() == 27));
        let level = parse_level(&data);
        assert_eq!(level.grid.count(Tile::Nodule), NUGGETS_PER_LEVEL);
        assert_eq!(level.potions.len(), 5);
        assert_eq!(level.grid.count(Tile::Pit), PITS_PER_LEVEL);
    }

    #[test]
    fn every_pickup_is_reachable_with_jumps() {
        let level = parse_level(&level_for(0));
        let seen = reachable(&level.grid, level.hero_spawn, true, false);
        for (x, y, t) in level.grid.cells() {
            if t == Tile::Nodule || t == Tile::Charge {
                assert!(seen.contains(&crate::logic::types::P::new(x, y)), "pickup at {x},{y} unreachable");
            }
        }
    }

    #[test]
    fn layout_matches_the_web_build() {
        // Row 1 of Dungeon I as generated by the TypeScript build (seed 1).
        let data = level_for(0);
        let expected = include_str!("../../tests/dungeon1.txt");
        let got = data.rows.join("\n");
        assert_eq!(got.trim_end(), expected.trim_end());
    }
}
