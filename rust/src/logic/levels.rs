use std::collections::HashSet;

use super::level::{LevelData, Theme};
use super::rng::Rng;
use super::sins::Sin;
use super::types::PotionKind;

/// Everyone moves at this fraction of the original tuning.
pub const SPEED_SCALE: f64 = 0.8;
/// Gold nuggets scattered through each classic 27 x 23 dungeon (the boss arenas' base).
pub const NUGGETS_PER_LEVEL: usize = 25;
/// Golden keys hidden across each estate; all of them open the gate to the boss.
pub const KEYS_PER_LEVEL: usize = 8;
/// Potion bottles per dungeon: one extra life, two shields, two invisibility.
pub const POTION_PLAN: [PotionKind; 5] = [PotionKind::Life, PotionKind::Health, PotionKind::Health, PotionKind::Haste, PotionKind::Haste];
/// Pits cut across corridors that must be jumped.
pub const PITS_PER_LEVEL: usize = 6;
/// Manhattan distance kept between two pits.
const MIN_PIT_SPACING: i32 = 6;
/// Tiles per authored cell in the classic dungeon (the boss arenas' base).
pub const WORLD_SCALE: usize = 1;
/// Corridors in an estate are this many tiles wide; the walls between them keep their thickness.
pub const CORRIDOR_WIDTH: usize = 2;

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
                '-' if x == mid_x => {
                    grid[y][x] = '.';
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
        let open = |x: i32, y: i32| x >= 0 && y >= 0 && x < w && y < h && matches!(grid[y as usize][x as usize], '.' | ' ');
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
        let passable = |x: i32, y: i32| x >= 0 && y >= 0 && x < w && y < h && !matches!(grid[y as usize][x as usize], '#' | '~' | '^');
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
 * How many tiles each column and row of an estate becomes: `width` for every
 * one that carries a corridor in the quadrant it is mirrored from (a column
 * where the path runs up and down, a row where it runs across), one for the
 * rest, so the walls between corridors keep their thickness.
 */
pub fn estate_widths(width: usize) -> (Vec<usize>, Vec<usize>) {
    let q: Vec<Vec<char>> = QUADRANT.iter().map(|r| r.chars().collect()).collect();
    let (qw, qh) = (q[0].len(), q.len());
    let open = |x: usize, y: usize| !matches!(q[y][x], '#' | 'N');
    let qcols: Vec<usize> = (0..qw).map(|x| if (0..qh - 1).any(|y| open(x, y) && open(x, y + 1)) { width } else { 1 }).collect();
    let qrows: Vec<usize> = (0..qh).map(|y| if (0..qw - 1).any(|x| open(x, y) && open(x + 1, y)) { width } else { 1 }).collect();
    // A quadrant is mirrored into a dungeon, and the dungeon into the estate, sharing the middle line each time.
    let mirror = |v: &[usize]| -> Vec<usize> { v.iter().chain(v.iter().rev().skip(1)).copied().collect() };
    (mirror(&mirror(&qcols)), mirror(&mirror(&qrows)))
}

/**
 * Widens the corridors of `layout`, repeating each column and row as many
 * times as `cols` and `rows` say (see `estate_widths`). As in `upscale`, a pit
 * becomes a trench right across the road; walls, floor, the hollow and the
 * tunnels fill their widened cell; and a pickup, shrine or spawn sits once in
 * it (the gate too, walled in beside it).
 */
pub fn widen(layout: &Layout, cols: &[usize], rows: &[usize]) -> Layout {
    let src = &layout.rows;
    assert_eq!((cols.len(), rows.len()), (src[0].len(), src.len()), "widths for every column and row");
    let mut out_rows = Vec::new();
    let mut out_heights = Vec::new();
    for (y, row) in src.iter().enumerate() {
        let fy = rows[y];
        for dy in 0..fy {
            let mut out = Vec::new();
            let mut hout = Vec::new();
            for (x, &c) in row.iter().enumerate() {
                let fx = cols[x];
                for dx in 0..fx {
                    let middle = dx == fx / 2 && dy == fy / 2;
                    out.push(match c {
                        '#' | ' ' | 'N' | '-' => c,
                        '~' if dx == fx / 2 => '~',
                        '^' if dy == fy / 2 => '~',
                        '~' | '^' => ' ',
                        'D' if middle => 'D',
                        'D' => '#',
                        _ if middle => c,
                        _ => ' ',
                    });
                    hout.push(layout.heights[y][x]);
                }
            }
            out_rows.push(out);
            out_heights.push(hout);
        }
    }
    Layout { rows: out_rows, heights: out_heights }
}

/**
 * Replaces the coin carpet with `count` gold nuggets. Every '.' becomes plain
 * floor, then nuggets are placed on open tiles by a seeded shuffle with a
 * minimum spacing that relaxes only if the dungeon cannot fit them all.
 */
pub fn place_nuggets(rows: &[Vec<char>], count: usize, seed: u32) -> Vec<Vec<char>> {
    let mut grid: Vec<Vec<char>> = rows.iter().map(|r| r.iter().map(|&c| if c == '.' { ' ' } else { c }).collect()).collect();
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

/**
 * Four classic dungeons joined into one estate four times the size: the
 * dungeon is mirrored right and down again (53 x 45), so floor levels meet
 * across the seams, and passages are opened through the shared wall column
 * and row. One enemy hollow is kept (top-left); the others become walled
 * courtyards. Each quarter keeps one weapon and one item shrine. The shared
 * row across the middle stays wall but for its passages, and in Dungeon I a
 * river takes it.
 */
pub fn compose_estate(base: &[&str]) -> Layout {
    let d = compose_dungeon(base);
    let mirror_x = |rows: &[Vec<char>]| -> Vec<Vec<char>> {
        rows.iter()
            .map(|r| {
                let mut out = r.clone();
                out.extend(r.iter().rev().skip(1));
                out
            })
            .collect()
    };
    let mirror_y = |rows: Vec<Vec<char>>| -> Vec<Vec<char>> {
        let mut out = rows.clone();
        out.extend(rows[..rows.len() - 1].iter().rev().cloned());
        out
    };
    let mut grid = mirror_y(mirror_x(&d.rows));
    let mut heights = mirror_y(mirror_x(&d.heights));
    let (w, h) = (grid[0].len(), grid.len());
    let (mid_x, mid_y) = ((w - 1) / 2, (h - 1) / 2);
    let (dw, dh) = (d.rows[0].len(), d.rows.len());
    for y in 0..h {
        for x in 0..w {
            let home = x < mid_x && y < mid_y;
            // Which quarter of its own dungeon copy this tile lies in.
            let (lx, ly) = (if x < dw { x } else { w - 1 - x }, if y < dh { y } else { h - 1 - y });
            let sub_left = lx < (dw - 1) / 2;
            let sub_top = ly < (dh - 1) / 2;
            match grid[y][x] {
                'N' | 'G' | 'P' if !home => grid[y][x] = '.',
                'W' if !(sub_left && sub_top) => grid[y][x] = '.',
                'I' if !(!sub_left && sub_top) => grid[y][x] = '.',
                '-' if x == mid_x => {
                    grid[y][x] = '.';
                    heights[y][x] = heights[y][x - 1];
                }
                _ => {}
            }
        }
    }
    // Passages through the shared column and row, wherever corridors meet across them.
    let walk = |g: &Vec<Vec<char>>, x: usize, y: usize| g[y][x] != '#';
    for y in 1..h - 1 {
        if grid[y][mid_x] == '#'
            && walk(&grid, mid_x - 1, y)
            && walk(&grid, mid_x + 1, y)
            && [1, 4, 18, 21].contains(&(if y < mid_y { y } else { h - 1 - y }))
        {
            grid[y][mid_x] = '.';
            heights[y][mid_x] = heights[y][mid_x - 1];
        }
    }
    for x in 1..w - 1 {
        let lx = if x < dw { x } else { w - 1 - x };
        if grid[mid_y][x] == '#' && walk(&grid, x, mid_y - 1) && walk(&grid, x, mid_y + 1) && [1, 10, 16, 25].contains(&lx) {
            grid[mid_y][x] = '.';
            heights[mid_y][x] = heights[mid_y - 1][x];
        }
    }
    // The gate to the boss: in the north wall, at the middle, over the passage there.
    grid[0][mid_x] = 'D';
    grid[1][mid_x] = '.';
    heights[1][mid_x] = heights[1][mid_x - 1];
    Layout { rows: grid, heights }
}

/**
 * Hides `count` golden keys: every '.' becomes plain floor, then keys go to
 * open tiles far from the middle (where Ragnar starts), from each other and
 * from the gate, two to each quarter, favoring dead ends.
 */
pub fn place_keys(rows: &[Vec<char>], count: usize, seed: u32) -> Vec<Vec<char>> {
    let mut grid: Vec<Vec<char>> = rows.iter().map(|r| r.iter().map(|&c| if c == '.' { ' ' } else { c }).collect()).collect();
    let (w, h) = (grid[0].len() as i32, grid.len() as i32);
    let (cx, cy) = ((w - 1) / 2, (h - 1) / 2);
    let open = |g: &Vec<Vec<char>>, x: i32, y: i32| {
        x >= 0 && y >= 0 && x < w && y < h && !matches!(g[y as usize][x as usize], '#' | '~' | '^' | 'D')
    };
    let mut rng = Rng::new(seed.wrapping_mul(2654435761).wrapping_add(11));
    let mut cands: Vec<(i32, i32, f64)> = vec![];
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            if grid[y as usize][x as usize] != ' ' || (x - cx).abs() + (y - cy).abs() < 12 || y <= 6 && (x - cx).abs() <= 6 {
                continue;
            }
            let busy = (-1..=1).any(|dy| {
                (-1..=1).any(|dx| {
                    let c = grid[(y + dy) as usize][(x + dx) as usize];
                    !matches!(c, ' ' | '#' | '.')
                })
            });
            if busy {
                continue;
            }
            let exits = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().filter(|(dx, dy)| open(&grid, x + dx, y + dy)).count();
            let bonus = if exits == 1 { 6.0 } else { 0.0 } + rng.next() * 2.0;
            cands.push((x, y, bonus));
        }
    }
    let quarter = |x: i32, y: i32| (x > cx) as usize + 2 * (y > cy) as usize;
    let per_quarter = count.div_ceil(4);
    let mut chosen: Vec<(i32, i32)> = vec![];
    let mut used = [0usize; 4];
    while chosen.len() < count {
        let pick = cands
            .iter()
            .filter(|c| used[quarter(c.0, c.1)] < per_quarter && !chosen.contains(&(c.0, c.1)))
            .max_by(|a, b| {
                let score = |c: &(i32, i32, f64)| {
                    let apart = chosen.iter().map(|q| (q.0 - c.0).abs() + (q.1 - c.1).abs()).min().unwrap_or(40).min(24) as f64;
                    let out = ((c.0 - cx).abs() + (c.1 - cy).abs()).min(30) as f64;
                    apart * 1.5 + out * 0.5 + c.2
                };
                score(a).total_cmp(&score(b))
            })
            .copied();
        let Some((x, y, _)) = pick else { break };
        chosen.push((x, y));
        used[quarter(x, y)] += 1;
    }
    for (x, y) in chosen {
        grid[y as usize][x as usize] = '.';
    }
    grid
}

/// The estate for a dungeon: pits, keys and potions, seeded so each has its own.
pub fn decorate_estate(layout: &Layout, seed: u32) -> Layout {
    // No green crosses in an estate: healing comes from its six potions.
    let mut layout = layout.clone();
    for row in layout.rows.iter_mut() {
        for c in row.iter_mut() {
            if *c == 'o' {
                *c = ' ';
            }
        }
    }
    let pitted = place_pits(&layout, PITS_PER_LEVEL * 3, seed);
    let plan = vec![PotionKind::Health; 6];
    let rows = place_potions(&place_keys(&pitted.rows, KEYS_PER_LEVEL, seed), &plan, seed);
    let (cols, widened) = estate_widths(CORRIDOR_WIDTH);
    widen(&Layout { rows, heights: pitted.heights }, &cols, &widened)
}

fn estate(name: &str, seed: u32, hero_speed: f64, enemy_speed: f64, frenzy_seconds: f64) -> LevelData {
    let layout = decorate_estate(&compose_estate(&QUADRANT), seed);
    LevelData {
        name: name.to_string(),
        rows: Layout::strings(&layout.rows),
        heights: Layout::strings(&layout.heights),
        hero_speed,
        enemy_speed,
        frenzy_seconds,
        boss: false,
        theme: Theme::Estate,
    }
}

/// A classic 27 x 23 dungeon with its nuggets, as the web build made it (the boss arenas are cut from it).
fn dungeon(name: &str, seed: u32, hero_speed: f64, enemy_speed: f64, frenzy_seconds: f64) -> LevelData {
    let layout = decorate_dungeon(&compose_dungeon(&QUADRANT), seed);
    LevelData {
        name: name.to_string(),
        rows: Layout::strings(&layout.rows),
        heights: Layout::strings(&layout.heights),
        hero_speed,
        enemy_speed,
        frenzy_seconds,
        boss: false,
        theme: Theme::Estate,
    }
}

pub fn levels() -> Vec<LevelData> {
    vec![
        estate("The Garden of Krampus", 1, 6.5 * SPEED_SCALE, 5.4 * SPEED_SCALE, 7.0),
        estate("Dungeon II: The Cisterns", 2, 7.0 * SPEED_SCALE, 6.3 * SPEED_SCALE, 5.5),
        estate("Dungeon III: Bone Vaults", 3, 7.5 * SPEED_SCALE, 7.3 * SPEED_SCALE, 4.0),
    ]
}

/// The classic dungeons, one per level, that the boss arenas are cut from.
pub fn classic_dungeon(index: usize) -> LevelData {
    let names = ["Sunken Halls", "The Cisterns", "Bone Vaults"];
    let seed = index as u32 + 1;
    dungeon(names.get(index).copied().unwrap_or("Deeper Still"), seed, 6.5 * SPEED_SCALE, 5.4 * SPEED_SCALE, 7.0)
}

/// Beyond the last authored level, speeds keep ramping and hazards land in new spots.
pub fn level_for(index: usize) -> LevelData {
    let all = levels();
    if index < all.len() {
        return all[index].clone();
    }
    let base = all.last().unwrap().clone();
    let extra = (index - all.len() + 1) as f64;
    let layout = decorate_estate(&compose_estate(&QUADRANT), index as u32 + 1);
    LevelData {
        name: format!("Dungeon {}: Deeper Still", index + 1),
        rows: Layout::strings(&layout.rows),
        heights: Layout::strings(&layout.heights),
        enemy_speed: (base.enemy_speed + extra * 0.3 * SPEED_SCALE).min(9.0 * SPEED_SCALE),
        hero_speed: (base.hero_speed + extra * 0.2 * SPEED_SCALE).min(9.0 * SPEED_SCALE),
        frenzy_seconds: (base.frenzy_seconds - extra * 0.5).max(2.0),
        boss: false,
        theme: Theme::Estate,
    }
}

/// The open middle of a boss arena, inclusive tile bounds: (x0, y0, x1, y1).
pub const ARENA: (i32, i32, i32, i32) = (3, 3, 23, 19);
/// The stream that crosses the arena runs along this row...
pub const STREAM_ROW: i32 = 12;
/// ...and wooden bridges cross it at these columns.
pub const BRIDGES: [i32; 3] = [7, 13, 19];
const PILLARS: [(i32, i32); 6] = [(7, 5), (19, 5), (10, 9), (16, 9), (7, 16), (19, 16)];
/// Low brick walls, as runs of tiles: (x0, x1, y).
const LOW_WALLS: [(i32, i32, i32); 5] = [(4, 6, 8), (20, 22, 8), (11, 12, 15), (14, 15, 15), (12, 14, 4)];
/// The gate out of the arena, in the north hedge: the Cyclops' red diamond key opens it.
pub const ARENA_GATE: (i32, i32) = (13, 2);
/// Where Ragnar and the Cyclops start.
pub const ARENA_HERO: (i32, i32) = (13, 18);
pub const ARENA_BOSS: (i32, i32) = (13, 6);

/**
 * The boss arena after dungeon `index`: the same dungeon's outer ring, with
 * the labyrinth in the middle torn out and turned into a sunny meadow. A
 * stream crosses it under three bridges; stone pillars and low brick walls
 * give cover from what the Cyclops throws. The ring keeps its green crosses,
 * and a weapon shrine and an item shrine flank the start. A gate stands in
 * the north hedge: the Cyclops drops the red diamond key that opens it.
 */
pub fn boss_arena(index: usize) -> LevelData {
    let base = classic_dungeon(index);
    let mut rows: Vec<Vec<char>> = base.rows.iter().map(|r| r.chars().collect()).collect();
    let (x0, y0, x1, y1) = ARENA;
    for (y, row) in rows.iter_mut().enumerate() {
        for (x, c) in row.iter_mut().enumerate() {
            let (x, y) = (x as i32, y as i32);
            if x >= x0 && x <= x1 && y >= y0 && y <= y1 {
                *c = ' ';
                continue;
            }
            // Out in the ring only rock and tunnels stay.
            if !matches!(*c, '#' | '-') {
                *c = ' ';
            }
        }
    }
    for x in x0..=x1 {
        if !BRIDGES.contains(&x) {
            rows[STREAM_ROW as usize][x as usize] = '~';
        }
    }
    for (x, y) in PILLARS {
        rows[y as usize][x as usize] = '#';
    }
    for (a, b, y) in LOW_WALLS {
        for x in a..=b {
            rows[y as usize][x as usize] = '#';
        }
    }
    // The gatehouse in the north hedge, solid hedge beside and behind it.
    let (gx, gy) = (ARENA_GATE.0 as usize, ARENA_GATE.1 as usize);
    for x in gx - 1..=gx + 1 {
        rows[gy][x] = '#';
        rows[gy - 1][x] = '#';
    }
    rows[gy][gx] = 'D';
    rows[ARENA_HERO.1 as usize][ARENA_HERO.0 as usize] = 'P';
    rows[ARENA_BOSS.1 as usize][ARENA_BOSS.0 as usize] = 'G';
    rows[ARENA_HERO.1 as usize][(ARENA_HERO.0 - 4) as usize] = 'W';
    rows[ARENA_HERO.1 as usize][(ARENA_HERO.0 + 4) as usize] = 'I';
    let heights = rows.iter().map(|r| "0".repeat(r.len())).collect();
    LevelData {
        name: if index == 0 { "The Cyclops' Meadow".to_string() } else { format!("The Cyclops' Meadow {}", index + 1) },
        rows: rows.iter().map(|r| r.iter().collect()).collect(),
        heights,
        hero_speed: base.hero_speed,
        enemy_speed: base.enemy_speed,
        frenzy_seconds: base.frenzy_seconds,
        boss: true,
        theme: Theme::Meadow,
    }
}

/**
 * The realm of a sin: for now an estate of its own seed, named for the realm,
 * as hard as dungeon `tier` (one deeper for every sin's key already held).
 * Its own ground, foes and sky come with its phase of the roadmap.
 */
pub fn realm_level(sin: Sin, tier: usize) -> LevelData {
    let base = level_for(tier);
    if sin == Sin::Greed {
        // The Vaults of Mammon (see `vault`).
        return super::vault::Vault::generate(super::vault::SEED, 1).level(&base);
    }
    if let Some(level) = super::realms::level(sin, tier, &base) {
        // The six after them (see `realms`).
        return level;
    }
    let layout = decorate_estate(&compose_estate(&QUADRANT), 20 + sin.index() as u32);
    LevelData {
        name: sin.realm().to_string(),
        rows: Layout::strings(&layout.rows),
        heights: Layout::strings(&layout.heights),
        boss: false,
        theme: Theme::Realm(sin),
        ..base
    }
}

/// The prince's court beyond the realm's gate, each sin's its own (`court`): Greed's is the
/// Counting House; a court not yet built is the Cyclops' meadow under the sin's sky, and the prince
/// a Cyclops in the sin's colour.
pub fn sin_arena(sin: Sin, tier: usize) -> LevelData {
    super::court::court_level(sin, tier)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::parse_level;
    use crate::logic::pathing::reachable;
    use crate::logic::types::Tile;

    #[test]
    fn dungeon_is_27_by_23_with_25_nuggets_and_5_potions() {
        let data = classic_dungeon(0);
        assert_eq!(data.rows.len(), 23);
        assert!(data.rows.iter().all(|r| r.chars().count() == 27));
        let level = parse_level(&data);
        assert_eq!(level.grid.count(Tile::Nodule), NUGGETS_PER_LEVEL);
        assert_eq!(level.potions.len(), 5);
        assert_eq!(level.grid.count(Tile::Pit), PITS_PER_LEVEL);
    }

    #[test]
    fn the_estate_is_four_dungeons_with_eight_keys_and_a_gate() {
        let data = level_for(0);
        // Four 27 x 23 dungeons (53 x 45 joined), every corridor two tiles wide.
        assert_eq!(data.rows.len(), 61);
        assert!(data.rows.iter().all(|r| r.chars().count() == 73));
        let level = parse_level(&data);
        assert_eq!(level.grid.count(Tile::Nodule), KEYS_PER_LEVEL);
        assert_eq!(level.potions.len(), 6);
        assert!(level.potions.iter().all(|(_, k)| *k == PotionKind::Health));
        assert_eq!(level.grid.count(Tile::Charge), 0, "no green crosses");
        assert_eq!(level.angel_shrines.len(), 4);
        assert_eq!(level.item_shrines.len(), 4);
        assert_eq!(level.grid.count(Tile::Nest), 8, "one hollow");
        let gate = level.gate.expect("a gate");
        assert_eq!((gate.x, gate.y), (36, 0));
        assert!(level.grid.is_walkable(gate.x, gate.y + 1, false), "a passage leads to the gate");
        // Two keys to each quarter.
        for q in 0..4 {
            let n = level.grid.cells().filter(|&(x, y, t)| t == Tile::Nodule && (x > 36) as usize + 2 * (y > 30) as usize == q).count();
            assert_eq!(n, 2, "quarter {q}");
        }
    }

    #[test]
    fn every_corridor_is_two_tiles_wide_and_the_walls_keep_their_thickness() {
        let q: Vec<Vec<char>> = QUADRANT.iter().map(|r| r.chars().collect()).collect();
        let (cols, rows) = estate_widths(CORRIDOR_WIDTH);
        assert_eq!((cols.len(), rows.len()), (53, 45));
        // In the quadrant: the corridors running up and down, and across, are the widened ones.
        assert_eq!(cols[..14].iter().enumerate().filter(|(_, &n)| n == 2).map(|(i, _)| i).collect::<Vec<_>>(), vec![1, 4, 7, 10, 12]);
        assert_eq!(rows[..12].iter().enumerate().filter(|(_, &n)| n == 2).map(|(i, _)| i).collect::<Vec<_>>(), vec![1, 4, 7, 10]);
        assert_eq!(q[2][2..4], ['#', '#'], "a wall block's columns stay one tile each");
        // No stretch of open floor in the estate is a single tile wide across its whole run.
        let level = parse_level(&level_for(0));
        let g = &level.grid;
        let floor = |x: i32, y: i32| g.in_bounds(x, y) && g.get(x, y) != Tile::Rock && g.get(x, y) != Tile::Nest;
        for y in 1..g.height - 1 {
            let mut x = 1;
            while x < g.width - 1 {
                // A run across of three or more floor tiles is a corridor: it has floor above or below all along.
                let start = x;
                while floor(x, y) {
                    x += 1;
                }
                if x - start >= 3 {
                    assert!((start..x).all(|i| floor(i, y - 1) || floor(i, y + 1)), "row {y} from {start} to {x} is a lone tile wide");
                }
                x += 1;
            }
        }
    }

    #[test]
    fn every_pickup_and_the_gate_are_reachable_with_jumps() {
        for index in 0..4 {
            let level = parse_level(&level_for(index));
            let seen = reachable(&level.grid, level.hero_spawn, true, false);
            let gate = level.gate.unwrap();
            assert!(seen.contains(&crate::logic::types::P::new(gate.x, gate.y + 1)), "dungeon {index}'s gate");
            for (x, y, t) in level.grid.cells() {
                if t == Tile::Nodule || t == Tile::Charge {
                    assert!(seen.contains(&crate::logic::types::P::new(x, y)), "dungeon {index}: pickup at {x},{y} unreachable");
                }
            }
            for p in level.angel_shrines.iter().chain(&level.item_shrines).chain(level.potions.iter().map(|(p, _)| p)) {
                assert!(seen.contains(p), "dungeon {index}: {p:?} unreachable");
            }
        }
    }

    #[test]
    fn the_boss_arena_is_open_in_the_middle() {
        let level = parse_level(&boss_arena(0));
        let g = &level.grid;
        assert_eq!(g.count(Tile::Nodule), 0);
        assert!(level.data.boss);
        let (x0, y0, x1, y1) = ARENA;
        let rock = (y0..=y1).flat_map(|y| (x0..=x1).map(move |x| (x, y))).filter(|&(x, y)| g.get(x, y) == Tile::Rock).count();
        assert!(rock <= 20, "the middle should be open, found {rock} rock tiles");
        assert_eq!(g.count(Tile::Pit), (x1 - x0 + 1) as usize - BRIDGES.len(), "one stream, bridged");
        // Everywhere open is reachable on foot from the start, over the bridges.
        let seen = reachable(g, level.hero_spawn, false, false);
        for (x, y, t) in g.cells() {
            if matches!(t, Tile::Open | Tile::Charge) && (x0..=x1).contains(&x) && (y0..=y1).contains(&y) {
                assert!(seen.contains(&crate::logic::types::P::new(x, y)), "{x},{y} cut off");
            }
        }
    }

    #[test]
    fn layout_matches_the_web_build() {
        // Dungeon I as generated by the TypeScript build (seed 1): the classic dungeon the estates are built from.
        let data = classic_dungeon(0);
        let expected = include_str!("../../tests/dungeon1.txt");
        let got = data.rows.join("\n");
        assert_eq!(got.trim_end(), expected.trim_end());
    }
}
