//! Dungeon I as an English landscape garden. In the middle of the estate,
//! where the two long walls that quarter it cross, stands a fountain with a
//! statue of Krampus; four channels run from its basin straight out along
//! those walls, north, east, south and west, under a bridge wherever a path
//! crosses. Three pour off the island's edge into the clouds; the northern
//! one ends in a pool short of the gate. Around them lie a churchyard, walled
//! flower gardens, parterres, classical temples on the larger lawns and small
//! wayside shrines.
//!
//! The plan is worked out from the maze itself (the layout never changes), so
//! the water always crosses paths square-on and never blocks one: every
//! crossing becomes a bridge. Bridges are walkable by everyone but slimes.

use std::collections::{HashMap, HashSet, VecDeque};

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
    /// The Krampus fountain in the middle of the estate, and the floor level its basin stands on.
    pub fountain: Option<RiverTile>,
    /// Watercourses, each in flow order: from its source (beside the fountain) to where it
    /// ends (off the island's edge, or in a pool). Bridged tiles are included.
    pub channels: Vec<Vec<RiverTile>>,
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
    /// Every tile of water, channel by channel.
    pub fn water(&self) -> impl Iterator<Item = &RiverTile> {
        self.channels.iter().flatten()
    }

    pub fn is_water(&self, p: P) -> bool {
        self.water().any(|r| r.p == p)
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
 * The garden for this grid, or None if its middle has no room for the
 * fountain and its channels. `keep` lists tiles the water must not take
 * (spawns, shrines, potions, the gate).
 */
pub fn plan(grid: &Grid, keep: &[P]) -> Option<GardenPlan> {
    let (fountain, channels, bridges) = fountain_channels(grid, keep)?;
    let mut plan = GardenPlan { fountain: Some(fountain), channels, bridges, ..Default::default() };
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
    Some(GardenPlan { channels: vec![river], bridges, ..Default::default() })
}

/// Marks the bridges and protects the scenery from bombs.
pub fn apply(grid: &mut Grid, plan: &GardenPlan) {
    for b in &plan.bridges {
        grid.set_bridge(b.x, b.y, true);
    }
    let scenery = plan.water().map(|r| r.p).chain(plan.fountain.map(|f| f.p)).chain(plan.graveyard.iter().copied()).chain(plan.temples.iter().flatten().copied());
    for p in scenery {
        if grid.get(p.x, p.y) == Tile::Rock {
            grid.set_landmark(p.x, p.y, true);
        }
    }
}

/**
 * The fountain in the middle of the grid and its four channels. Each runs
 * straight out along the middle row or column, through wall and across paths
 * square-on (a bridge each time), until it pours off the island's edge. One
 * that meets a tile it must keep clear of (the gate) ends short of it, in
 * open water rather than under a bridge.
 */
fn fountain_channels(grid: &Grid, keep: &[P]) -> Option<(RiverTile, Vec<Vec<RiverTile>>, Vec<P>)> {
    let c = P::new((grid.width - 1) / 2, (grid.height - 1) / 2);
    if grid.get(c.x, c.y) != Tile::Rock || near_nest(grid, c.x, c.y) {
        return None;
    }
    let keep: HashSet<P> = keep.iter().copied().collect();
    let mut courses: Vec<Vec<P>> = vec![];
    for d in [Dir::Up, Dir::Right, Dir::Down, Dir::Left] {
        let (dx, dy) = d.vec();
        let mut course = vec![];
        let mut p = c;
        loop {
            let n = P::new(p.x + dx, p.y + dy);
            if !grid.in_bounds(n.x, n.y) || keep.contains(&n) {
                break;
            }
            if grid.get(n.x, n.y) == Tile::Rock {
                if near_nest(grid, n.x, n.y) {
                    break;
                }
                course.push(n);
                if n.x == 0 || n.y == 0 || n.x == grid.width - 1 || n.y == grid.height - 1 {
                    break; // over the edge
                }
                p = n;
            } else if let Some(run) = crossing(grid, n, d) {
                p = *run.last().unwrap();
                course.extend(run);
            } else {
                break;
            }
        }
        // A channel never ends under a bridge.
        while course.last().is_some_and(|q| grid.get(q.x, q.y) != Tile::Rock) {
            course.pop();
        }
        if course.len() < 2 {
            return None;
        }
        courses.push(course);
    }
    let taken: HashSet<P> = courses.iter().flatten().copied().chain([c]).collect();
    let bridges = courses.iter().flatten().copied().filter(|p| grid.get(p.x, p.y) != Tile::Rock).collect();
    // The basin stands as high as the paths around it; from there the water only runs down.
    let top = CARDINALS
        .iter()
        .map(|&d| {
            let (dx, dy) = d.vec();
            grid.elevation(c.x + dx, c.y + dy)
        })
        .max()
        .unwrap_or(0);
    let channels = courses.iter().map(|course| water_levels(grid, course, &taken, top)).collect();
    Some((RiverTile { p: c, level: top }, channels, bridges))
}

/**
 * The path water running `d` from `p` would cross square-on: a run of floor
 * tiles (as wide as the path) each with the path going on to either side, wall
 * before the run and after it. None if the water cannot cross there.
 */
fn crossing(grid: &Grid, p: P, d: Dir) -> Option<Vec<P>> {
    let (dx, dy) = d.vec();
    let path = |x: i32, y: i32| grid.is_walkable(x, y, false);
    if grid.get(p.x - dx, p.y - dy) != Tile::Rock {
        return None;
    }
    let mut run = vec![];
    let mut q = p;
    while (open_floor(grid, q.x, q.y) || grid.is_ramp(q.x, q.y)) && path(q.x - dy, q.y - dx) && path(q.x + dy, q.y + dx) {
        run.push(q);
        if run.len() > 3 {
            return None;
        }
        q = P::new(q.x + dx, q.y + dy);
    }
    (!run.is_empty() && grid.get(q.x, q.y) == Tile::Rock).then_some(run)
}

/// Water lies at the lowest floor beside it, never above the water upstream; where it steps down, it falls over a weir.
fn water_levels(grid: &Grid, course: &[P], taken: &HashSet<P>, source: i32) -> Vec<RiverTile> {
    let mut last = source;
    course
        .iter()
        .map(|&p| {
            let beside = CARDINALS
                .iter()
                .filter_map(|&d| {
                    let (dx, dy) = d.vec();
                    let n = P::new(p.x + dx, p.y + dy);
                    (grid.is_walkable(n.x, n.y, false) && !taken.contains(&n)).then(|| grid.elevation(n.x, n.y))
                })
                .min();
            last = beside.unwrap_or(last).min(last);
            RiverTile { p, level: last }
        })
        .collect()
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
    let water: HashSet<P> = plan.water().map(|r| r.p).chain(plan.fountain.map(|f| f.p)).collect();
    let mut blocks = wall_blocks(grid, &water);
    blocks.sort_by_key(|b| (b[0].y, b[0].x));
    let hero_side = |b: &[P]| center(b);

    // The churchyard: the biggest walled enclosure, away from the middle. (With corridors two
    // tiles wide the enclosures are longer; the long walls quartering the estate are longer still.)
    let mut big: Vec<usize> = (0..blocks.len()).filter(|&i| (10..=50).contains(&blocks[i].len())).collect();
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
        keep.extend(level.gate);
        let plan = plan(&level.grid, &keep).expect("room for the fountain");
        let mut grid = level.grid.clone();
        apply(&mut grid, &plan);
        (grid, plan, level.hero_spawn)
    }

    #[test]
    fn four_channels_run_from_the_fountain_and_are_bridged_at_every_path() {
        let (grid, plan, _) = garden();
        let f = plan.fountain.expect("a fountain");
        let (w, h) = (grid.width, grid.height);
        let mid = P::new((w - 1) / 2, (h - 1) / 2);
        assert_eq!(f.p, mid, "in the middle of the estate");
        assert_eq!(plan.channels.len(), 4);
        // North stops short of the gate, below the corridor in front of it; east, south and west pour off the edge.
        let ends: Vec<P> = plan.channels.iter().map(|c| c.last().unwrap().p).collect();
        assert_eq!(ends, vec![P::new(mid.x, 3), P::new(w - 1, mid.y), P::new(mid.x, h - 1), P::new(0, mid.y)]);
        for c in &plan.channels {
            let first = c[0];
            assert_eq!((first.p.x - f.p.x).abs() + (first.p.y - f.p.y).abs(), 1, "it leaves the basin");
            assert!(first.level <= f.level);
            for pair in c.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                assert_eq!((a.p.x - b.p.x).abs() + (a.p.y - b.p.y).abs(), 1, "the channel is continuous");
                assert!(b.level <= a.level && a.level - b.level <= 1, "water runs down a level at a time");
            }
            assert_eq!(grid.get(c.last().unwrap().p.x, c.last().unwrap().p.y), Tile::Rock, "it never ends under a bridge");
        }
        assert!(plan.bridges.len() >= 12, "it crosses the paths: {:?}", plan.bridges);
        for r in plan.water() {
            let t = grid.get(r.p.x, r.p.y);
            assert!(t == Tile::Rock || grid.is_bridge(r.p.x, r.p.y), "{:?} is water on a path", r.p);
            if grid.is_bridge(r.p.x, r.p.y) {
                assert_eq!(r.level, grid.elevation(r.p.x, r.p.y), "the water under {:?} lies at the path's level", r.p);
            }
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
        for &b in &plan.bridges {
            // Stand on the path beside a bridge: the step onto it is refused for a slime only.
            for d in CARDINALS {
                let (dx, dy) = d.vec();
                let from = P::new(b.x - dx, b.y - dy);
                if !grid.is_walkable(from.x, from.y, false) || grid.is_bridge(from.x, from.y) || grid.elevation(from.x, from.y) != grid.elevation(b.x, b.y) {
                    continue;
                }
                let mut m = Mover::new(from, 1.0);
                assert!(m.can_go(&grid, d, false), "{from:?} to the bridge at {b:?}");
                m.avoids_bridges = true;
                assert!(!m.can_go(&grid, d, false));
            }
        }
    }

    #[test]
    fn scenery_is_placed_and_bombproof() {
        let (grid, plan, _) = garden();
        assert!(plan.graveyard.len() >= 8, "a churchyard: {:?}", plan.graveyard);
        assert_eq!(plan.temples.len(), 4);
        assert!(!plan.flowers.is_empty());
        assert_eq!(plan.shrines.len(), 14);
        for p in plan.graveyard.iter().chain(plan.water().map(|r| &r.p)).chain(plan.fountain.iter().map(|f| &f.p)) {
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
                    if plan.fountain.is_some_and(|f| f.p == p) {
                        'K'
                    } else if plan.bridges.contains(&p) {
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
        for c in &plan.channels {
            println!("{:?}", c.iter().map(|r| (r.p.x, r.p.y, r.level)).collect::<Vec<_>>());
        }
    }
}
