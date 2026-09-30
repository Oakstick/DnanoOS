use std::collections::{HashMap, HashSet, VecDeque};

use super::grid::Grid;
use super::types::{Dir, CARDINALS, P};
use crate::logic::types::V;

/**
 * Every tile a walker can reach from `start`. With `can_jump`, jump landings
 * count as moves too, which is how the hero crosses pits and climbs ledges.
 */
pub fn reachable(grid: &Grid, start: P, can_jump: bool, allow_nest: bool) -> HashSet<P> {
    let mut seen = HashSet::from([start]);
    let mut queue = VecDeque::from([start]);
    while let Some(p) = queue.pop_front() {
        for d in CARDINALS {
            if grid.can_move(p, d, allow_nest, false) {
                let n = grid.neighbor(p, d);
                if seen.insert(n) {
                    queue.push_back(n);
                }
            }
            if can_jump {
                if let Some(land) = grid.jump_landing(p, d) {
                    if seen.insert(land) {
                        queue.push_back(land);
                    }
                }
            }
        }
    }
    seen
}

/**
 * First step of the shortest path from `from` toward `target`: to its tile when
 * that can be reached, else to the reachable tile nearest it (a target inside
 * a wall or across water). `avoid_bridges` keeps a slime off the bridges.
 * None when there is nowhere better to be. Hunters use this, so they search
 * the maze for the hero rather than circling the blocks between them.
 */
pub fn route_toward(grid: &Grid, from: P, target: V, flying: bool, allow_nest: bool, avoid_bridges: bool) -> Option<Dir> {
    let goal = P::new(target.x.round() as i32, target.y.round() as i32);
    let score = |p: P| (p.x - goal.x).pow(2) + (p.y - goal.y).pow(2);
    let mut first: HashMap<P, Dir> = HashMap::from([(from, Dir::None)]);
    let mut queue = VecDeque::from([from]);
    let mut best = (score(from), Dir::None);
    while let Some(p) = queue.pop_front() {
        let opening = first[&p];
        for d in CARDINALS {
            if !grid.can_move(p, d, allow_nest, flying) {
                continue;
            }
            let n = grid.neighbor(p, d);
            if first.contains_key(&n) || (avoid_bridges && grid.is_bridge(n.x, n.y)) {
                continue;
            }
            let step = if opening == Dir::None { d } else { opening };
            let s = score(n);
            if s < best.0 {
                best = (s, step);
                if s == 0 {
                    return Some(step);
                }
            }
            first.insert(n, step);
            queue.push_back(n);
        }
    }
    (best.1 != Dir::None).then_some(best.1)
}

/**
 * First step of a shortest path from `from` to `goal`, or None when no route
 * exists. A slain enemy uses this to find its way home: the hollow has a
 * single entrance, and walking greedily towards it can circle a block forever.
 */
pub fn step_toward(grid: &Grid, from: P, goal: P, flying: bool, allow_nest: bool) -> Option<Dir> {
    if from == goal {
        return None;
    }
    let mut first: HashMap<P, Dir> = HashMap::from([(from, Dir::None)]);
    let mut queue = VecDeque::from([from]);
    while let Some(p) = queue.pop_front() {
        let opening = first[&p];
        for d in CARDINALS {
            if !grid.can_move(p, d, allow_nest, flying) {
                continue;
            }
            let n = grid.neighbor(p, d);
            if first.contains_key(&n) {
                continue;
            }
            let step = if opening == Dir::None { d } else { opening };
            if n == goal {
                return Some(step);
            }
            first.insert(n, step);
            queue.push_back(n);
        }
    }
    None
}
