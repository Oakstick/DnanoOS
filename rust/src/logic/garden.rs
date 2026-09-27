//! Dungeon I as an English landscape garden. A river winds through the
//! hedges from a spring at one edge of the island to a waterfall off the
//! other, crossed by bridges where it meets the paths. Around it lie a
//! churchyard, walled flower gardens, parterres, classical temples on the
//! larger lawns and small wayside shrines.
//!
//! The plan is worked out from the maze itself (the layout never changes), so
//! the river always crosses paths square-on and never blocks one: every
//! crossing becomes a bridge. Bridges are walkable by everyone but slimes.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

use super::grid::Grid;
use super::types::{Dir, Tile, CARDINALS, P};

/// One tile of the river, in flow order, and the floor level its water lies at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RiverTile {
    pub p: P,
    pub level: i32,
}

#[derive(Clone, Debug, Default)]
pub struct GardenPlan {
    /// From the spring (first) to the waterfall off the edge (last). Includes bridged tiles.
    pub river: Vec<RiverTile>,
    pub bridges: Vec<P>,
    /// Wall tiles of the churchyard: graves, an iron railing, a dead tree, a tomb.
    pub graveyard: Vec<P>,
    /// Wall tiles given over to flower beds inside box hedges.
    pub flowers: Vec<P>,
    /// Small blocks (2x2 lawns) each crowned by a round temple: the block's tiles.
    pub temples: Vec<Vec<P>>,
    /// Single wall tiles with a wayside shrine.
    pub shrines: Vec<P>,
}

impl GardenPlan {
    pub fn is_water(&self, p: P) -> bool {
        self.river.iter().any(|r| r.p == p)
    }

    /// The churchyard's middle, for its thicker mist.
    pub fn graveyard_center(&self) -> Option<(f32, f32)> {
        if self.graveyard.is_empty() {
            return None;
        }
        let n = self.graveyard.len() as f32;
        Some((self.graveyard.iter().map(|p| p.x as f32).sum::<f32>() / n, self.graveyard.iter().map(|p| p.y as f32).sum::<f32>() / n))
    }
}

/// Deterministic 0..1 noise for a tile.
fn h(x: i32, y: i32, salt: u32) -> f64 {
    let mut v = (x as u32).wrapping_mul(0x9e37_79b9) ^ (y as u32).wrapping_mul(0x85eb_ca6b) ^ salt.wrapping_mul(0xc2b2_ae35);
    v ^= v >> 15;
    v = v.wrapping_mul(0x2c1b_3c6d);
    v ^= v >> 12;
    (v & 0xffff) as f64 / 65535.0
}

fn open_floor(grid: &Grid, x: i32, y: i32) -> bool {
    matches!(grid.get(x, y), Tile::Open | Tile::Nodule | Tile::Charge)
}

/// Rock next to the enemies' hollow stays as it is.
fn near_nest(grid: &Grid, x: i32, y: i32) -> bool {
    (-1..=1).any(|dy| (-1..=1).any(|dx| grid.get(x + dx, y + dy) == Tile::Nest))
}

/**
 * The garden for this grid, or None if no river course exists. `keep` lists
 * tiles the river must not take (spawns, shrines, potions).
 */
pub fn plan(grid: &Grid, keep: &[P]) -> Option<GardenPlan> {
    let (river, bridges) = river_course(grid, keep)?;
    let mut plan = GardenPlan { river, bridges, ..Default::default() };
    place_features(grid, &mut plan);
    Some(plan)
}

/**
 * A boss arena's stream as a river: its water tiles (pits) and the bridges
 * across it, in one row, flowing east to west. No scenery beyond that.
 */
pub fn stream_plan(grid: &Grid) -> Option<GardenPlan> {
    let mut rows: HashMap<i32, Vec<i32>> = HashMap::new();
    for (x, y, t) in grid.cells() {
        if t == Tile::Pit {
            rows.entry(y).or_default().push(x);
        }
    }
    let (&y, xs) = rows.iter().max_by_key(|(_, xs)| xs.len())?;
    let (x0, x1) = (*xs.iter().min()?, *xs.iter().max()?);
    let mut river = vec![];
    let mut bridges = vec![];
    for x in (x0..=x1).rev() {
        let t = grid.get(x, y);
        if t == Tile::Pit {
            river.push(RiverTile { p: P::new(x, y), level: grid.elevation(x, y) });
        } else if t != Tile::Rock {
            river.push(RiverTile { p: P::new(x, y), level: grid.elevation(x, y) });
            bridges.push(P::new(x, y));
        }
    }
    Some(GardenPlan { river, bridges, ..Default::default() })
}

/// Marks the bridges and protects the scenery from bombs.
pub fn apply(grid: &mut Grid, plan: &GardenPlan) {
    for b in &plan.bridges {
        grid.set_bridge(b.x, b.y, true);
    }
    let scenery = plan.river.iter().map(|r| r.p).chain(plan.graveyard.iter().copied()).chain(plan.temples.iter().flatten().copied());
    for p in scenery {
        if grid.get(p.x, p.y) == Tile::Rock {
            grid.set_landmark(p.x, p.y, true);
        }
    }
}

/**
 * The river: a cheapest course from the east edge to the west edge that runs
 * through wall tiles and crosses paths only straight across a north-south
 * corridor (walls either side along its flow), preferring the middle rows.
 */
fn river_course(grid: &Grid, keep: &[P]) -> Option<(Vec<RiverTile>, Vec<P>)> {
    let (w, hgt) = (grid.width, grid.height);
    let keep: HashSet<P> = keep.iter().copied().collect();
    let mid = hgt as f64 / 2.0 - 0.5;
    // Costs in hundredths, so the heap orders exactly.
    let tile_cost = |x: i32, y: i32| -> Option<u32> {
        let band = ((y as f64 - mid).abs() * 20.0) as u32;
        if x == 0 {
            return (y > 0 && y < hgt - 1 && grid.get(x, y) == Tile::Rock).then_some(100 + band);
        }
        if x == w - 1 || y <= 0 || y >= hgt - 1 {
            return None;
        }
        let p = P::new(x, y);
        match grid.get(x, y) {
            Tile::Rock if !near_nest(grid, x, y) => Some(100 + band),
            _ if keep.contains(&p) => None,
            // A crossing: a north-south corridor with wall either side of it.
            _ if (open_floor(grid, x, y) || grid.is_ramp(x, y))
                && grid.get(x - 1, y) == Tile::Rock
                && grid.get(x + 1, y) == Tile::Rock
                && grid.get(x, y - 1) != Tile::Rock
                && grid.get(x, y + 1) != Tile::Rock =>
            {
                Some(600 + band)
            }
            _ => None,
        }
    };
    let mut dist: HashMap<P, u32> = HashMap::new();
    let mut prev: HashMap<P, P> = HashMap::new();
    let mut heap = BinaryHeap::new();
    for y in 1..hgt - 1 {
        if grid.get(w - 1, y) == Tile::Rock {
            let p = P::new(w - 1, y);
            dist.insert(p, 0);
            heap.push(Reverse((0u32, y, w - 1)));
        }
    }
    let mut goal = None;
    while let Some(Reverse((d, y, x))) = heap.pop() {
        let p = P::new(x, y);
        if dist.get(&p).is_some_and(|&best| d > best) {
            continue;
        }
        if x == 0 {
            goal = Some(p);
            break;
        }
        let crossing = grid.get(x, y) != Tile::Rock;
        for dir in CARDINALS {
            // Across a path the river keeps straight on, east to west.
            if crossing && dir != Dir::Left {
                continue;
            }
            let (dx, dy) = dir.vec();
            let (nx, ny) = (x + dx, y + dy);
            if !grid.in_bounds(nx, ny) {
                continue;
            }
            let Some(mut c) = tile_cost(nx, ny) else { continue };
            let entering_path = grid.get(nx, ny) != Tile::Rock;
            if entering_path && dir != Dir::Left {
                continue;
            }
            if dy != 0 {
                c += 60; // a bend costs a little: long straight reaches read as a river
            }
            let n = P::new(nx, ny);
            let nd = d + c;
            if dist.get(&n).is_none_or(|&best| nd < best) {
                dist.insert(n, nd);
                prev.insert(n, p);
                heap.push(Reverse((nd, ny, nx)));
            }
        }
    }
    let goal = goal?;
    // Walk back from the west edge: that list runs west to east, so reverse it to flow order.
    let mut course = vec![goal];
    let mut at = goal;
    while let Some(&p) = prev.get(&at) {
        course.push(p);
        at = p;
    }
    // `course` now runs from the west edge back to the spring; flow is east to west.
    course.reverse();
    let bridges: Vec<P> = course.iter().copied().filter(|p| grid.get(p.x, p.y) != Tile::Rock).collect();

    // Water lies at the lowest floor beside it; where that steps, it falls over a weir.
    let mut levels: Vec<Option<i32>> = course
        .iter()
        .map(|p| {
            CARDINALS
                .iter()
                .filter_map(|&d| {
                    let (dx, dy) = d.vec();
                    let (nx, ny) = (p.x + dx, p.y + dy);
                    let t = grid.get(nx, ny);
                    let walk = t != Tile::Rock && t != Tile::Pit && !course.contains(&P::new(nx, ny));
                    walk.then(|| grid.elevation(nx, ny))
                })
                .min()
        })
        .collect();
    let first = levels.iter().flatten().copied().next().unwrap_or(0);
    let mut last = first;
    for l in levels.iter_mut() {
        let v = l.unwrap_or(last);
        *l = Some(v);
        last = v;
    }
    let river = course.iter().zip(levels).map(|(&p, l)| RiverTile { p, level: l.unwrap_or(0) }).collect();
    Some((river, bridges))
}

/// Groups of wall tiles joined edge to edge.
fn wall_blocks(grid: &Grid, skip: &HashSet<P>) -> Vec<Vec<P>> {
    let mut seen: HashSet<P> = HashSet::new();
    let mut blocks = Vec::new();
    for (x, y, t) in grid.cells() {
        let p = P::new(x, y);
        let border = x == 0 || y == 0 || x == grid.width - 1 || y == grid.height - 1;
        if t != Tile::Rock || border || skip.contains(&p) || near_nest(grid, x, y) || seen.contains(&p) {
            continue;
        }
        let mut block = vec![];
        let mut queue = VecDeque::from([p]);
        seen.insert(p);
        while let Some(q) = queue.pop_front() {
            block.push(q);
            for d in CARDINALS {
                let (dx, dy) = d.vec();
                let n = P::new(q.x + dx, q.y + dy);
                let nborder = n.x <= 0 || n.y <= 0 || n.x >= grid.width - 1 || n.y >= grid.height - 1;
                if grid.get(n.x, n.y) == Tile::Rock && !nborder && !skip.contains(&n) && !near_nest(grid, n.x, n.y) && seen.insert(n) {
                    queue.push_back(n);
                }
            }
        }
        blocks.push(block);
    }
    blocks
}

fn center(tiles: &[P]) -> (f64, f64) {
    let n = tiles.len() as f64;
    (tiles.iter().map(|p| p.x as f64).sum::<f64>() / n, tiles.iter().map(|p| p.y as f64).sum::<f64>() / n)
}

fn is_square(block: &[P]) -> bool {
    if block.len() != 4 {
        return false;
    }
    let (x0, y0) = (block.iter().map(|p| p.x).min().unwrap(), block.iter().map(|p| p.y).min().unwrap());
    block.iter().all(|p| p.x - x0 <= 1 && p.y - y0 <= 1)
}

fn faces_path(grid: &Grid, p: P) -> bool {
    CARDINALS.iter().any(|d| {
        let (dx, dy) = d.vec();
        let t = grid.get(p.x + dx, p.y + dy);
        t != Tile::Rock && t != Tile::Nest
    })
}

fn place_features(grid: &Grid, plan: &mut GardenPlan) {
    let water: HashSet<P> = plan.river.iter().map(|r| r.p).collect();
    let mut blocks = wall_blocks(grid, &water);
    blocks.sort_by_key(|b| (b[0].y, b[0].x));
    let hero_side = |b: &[P]| center(b);

    // The churchyard: the biggest walled enclosure, away from the middle.
    let mut big: Vec<usize> = (0..blocks.len()).filter(|&i| (8..=30).contains(&blocks[i].len())).collect();
    big.sort_by(|&a, &b| blocks[b].len().cmp(&blocks[a].len()).then(hero_side(&blocks[a]).0.total_cmp(&hero_side(&blocks[b]).0).reverse()));
    let mut used: HashSet<usize> = HashSet::new();
    if let Some(&g) = big.first() {
        plan.graveyard = blocks[g].clone();
        used.insert(g);
    }
    // The other big enclosures become walled flower gardens.
    for &i in big.iter().skip(1) {
        plan.flowers.extend(blocks[i].iter().copied());
        used.insert(i);
    }

    // Temples on two square lawns, far from each other and from the churchyard.
    let squares: Vec<usize> = (0..blocks.len()).filter(|&i| !used.contains(&i) && is_square(&blocks[i])).collect();
    let far = |c: (f64, f64), others: &[(f64, f64)]| others.iter().map(|o| (c.0 - o.0).powi(2) + (c.1 - o.1).powi(2)).fold(f64::INFINITY, f64::min);
    let mut anchors: Vec<(f64, f64)> = vec![];
    if !plan.graveyard.is_empty() {
        anchors.push(center(&plan.graveyard));
    }
    let big_estate = grid.width * grid.height > 1500;
    let (temple_count, shrine_count) = if big_estate { (4, 14) } else { (2, 6) };
    for _ in 0..temple_count {
        let pick = squares.iter().copied().filter(|i| !used.contains(i)).max_by(|&a, &b| {
            let fa = far(center(&blocks[a]), &anchors) + h(a as i32, 0, 7) * 0.5;
            let fb = far(center(&blocks[b]), &anchors) + h(b as i32, 0, 7) * 0.5;
            fa.total_cmp(&fb)
        });
        if let Some(i) = pick {
            anchors.push(center(&blocks[i]));
            plan.temples.push(blocks[i].clone());
            used.insert(i);
        }
    }
    // Parterres: about a third of the remaining square lawns.
    for &i in &squares {
        if !used.contains(&i) && h(blocks[i][0].x, blocks[i][0].y, 3) < 0.34 {
            plan.flowers.extend(blocks[i].iter().copied());
            used.insert(i);
        }
    }
    // Wayside shrines: wall tiles by the paths, spread across the garden.
    let mut candidates: Vec<P> = (0..blocks.len()).filter(|i| !used.contains(i)).flat_map(|i| blocks[i].iter().copied()).filter(|&p| faces_path(grid, p)).collect();
    candidates.sort_by_key(|p| (p.y, p.x));
    let mut spots: Vec<(f64, f64)> = anchors.clone();
    for _ in 0..shrine_count {
        let pick = candidates.iter().copied().filter(|p| !plan.shrines.contains(p)).max_by(|a, b| {
            let fa = far((a.x as f64, a.y as f64), &spots) + h(a.x, a.y, 11) * 2.0;
            let fb = far((b.x as f64, b.y as f64), &spots) + h(b.x, b.y, 11) * 2.0;
            fa.total_cmp(&fb)
        });
        if let Some(p) = pick {
            spots.push((p.x as f64, p.y as f64));
            plan.shrines.push(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::parse_level;
    use crate::logic::levels::level_for;
    use crate::logic::pathing::reachable;

    fn garden() -> (Grid, GardenPlan, P) {
        let level = parse_level(&level_for(0));
        let mut keep = vec![level.hero_spawn, level.enemy_spawn];
        keep.extend(level.weapon_shrines.iter().chain(&level.item_shrines).copied());
        keep.extend(level.potions.iter().map(|(p, _)| *p));
        let plan = plan(&level.grid, &keep).expect("a river course");
        let mut grid = level.grid.clone();
        apply(&mut grid, &plan);
        (grid, plan, level.hero_spawn)
    }

    #[test]
    fn the_river_runs_edge_to_edge_and_is_bridged_at_every_path() {
        let (grid, plan, _) = garden();
        let first = plan.river.first().unwrap().p;
        let last = plan.river.last().unwrap().p;
        assert_eq!(first.x, grid.width - 1, "it rises at the east edge");
        assert_eq!(last.x, 0, "it falls off the west edge");
        for pair in plan.river.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            assert_eq!((a.p.x - b.p.x).abs() + (a.p.y - b.p.y).abs(), 1, "the river is continuous");
            assert!((b.level - a.level).abs() <= 1, "the water steps at most one level at a time");
        }
        assert!(plan.bridges.len() >= 2, "it crosses some paths: {:?}", plan.bridges);
        for r in &plan.river {
            let t = grid.get(r.p.x, r.p.y);
            assert!(t == Tile::Rock || grid.is_bridge(r.p.x, r.p.y), "{:?} is water on a path", r.p);
        }
    }

    #[test]
    fn bridges_keep_the_garden_whole_for_the_hero() {
        let (grid, _, spawn) = garden();
        let level = parse_level(&level_for(0));
        let before = reachable(&level.grid, spawn, true, false);
        let after = reachable(&grid, spawn, true, false);
        assert_eq!(before, after);
    }

    #[test]
    fn slimes_turn_back_at_bridges() {
        use crate::logic::mover::Mover;
        let (grid, plan, _) = garden();
        let b = plan.bridges[0];
        // Stand next to a bridge, north of it: the step south is refused for a slime only.
        let from = P::new(b.x, b.y - 1);
        let mut m = Mover::new(from, 1.0);
        assert!(m.can_go(&grid, Dir::Down, false) || grid.elevation(b.x, b.y) > grid.elevation(from.x, from.y));
        m.avoids_bridges = true;
        assert!(!m.can_go(&grid, Dir::Down, false));
    }

    #[test]
    fn scenery_is_placed_and_bombproof() {
        let (grid, plan, _) = garden();
        assert!(plan.graveyard.len() >= 8, "a churchyard: {:?}", plan.graveyard);
        assert_eq!(plan.temples.len(), 4);
        assert!(!plan.flowers.is_empty());
        assert_eq!(plan.shrines.len(), 14);
        for p in plan.graveyard.iter().chain(plan.river.iter().map(|r| &r.p)) {
            if grid.get(p.x, p.y) == Tile::Rock {
                assert!(grid.is_landmark(p.x, p.y));
            }
        }
    }

    #[test]
    #[ignore]
    fn print_plan() {
        let (grid, plan, _) = garden();
        for y in 0..grid.height {
            let row: String = (0..grid.width)
                .map(|x| {
                    let p = P::new(x, y);
                    if plan.bridges.contains(&p) {
                        '='
                    } else if plan.is_water(p) {
                        '~'
                    } else if plan.graveyard.contains(&p) {
                        '+'
                    } else if plan.temples.iter().any(|t| t.contains(&p)) {
                        'T'
                    } else if plan.flowers.contains(&p) {
                        '*'
                    } else if plan.shrines.contains(&p) {
                        's'
                    } else {
                        match grid.get(x, y) {
                            Tile::Rock => '#',
                            Tile::Nest => 'N',
                            Tile::Pit => 'o',
                            _ => ' ',
                        }
                    }
                })
                .collect();
            println!("{row}");
        }
        println!("{:?}", plan.river.iter().map(|r| (r.p.x, r.p.y, r.level)).collect::<Vec<_>>());
    }
}
