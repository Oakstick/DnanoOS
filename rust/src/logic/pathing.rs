use std::collections::{HashMap, HashSet, VecDeque};

use super::grid::Grid;
use super::types::{Dir, CARDINALS, P};

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
