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

/// The tiles a search has reached, and the first step that reached each: a byte per tile of the
/// grid (and a map for any off it). The same search as with a hash map, at a fraction of the cost:
/// hashing was most of the foes' time in a 30-foe fight in the Garden's maze (up to 6 ms a frame).
struct Firsts {
    w: i32,
    h: i32,
    steps: Vec<u8>,
    off: HashMap<P, Dir>,
}

const UNSEEN: u8 = u8::MAX;

impl Firsts {
    fn new(grid: &Grid, from: P) -> Self {
        let (w, h) = (grid.width.max(0), grid.height.max(0));
        let mut f = Self { w, h, steps: vec![UNSEEN; (w * h) as usize], off: HashMap::new() };
        f.insert(from, Dir::None);
        f
    }

    fn index(&self, p: P) -> Option<usize> {
        (p.x >= 0 && p.y >= 0 && p.x < self.w && p.y < self.h).then(|| (p.y * self.w + p.x) as usize)
    }

    fn get(&self, p: P) -> Option<Dir> {
        match self.index(p) {
            Some(i) => match self.steps[i] {
                UNSEEN => None,
                0 => Some(Dir::Up),
                1 => Some(Dir::Down),
                2 => Some(Dir::Left),
                3 => Some(Dir::Right),
                _ => Some(Dir::None),
            },
            None => self.off.get(&p).copied(),
        }
    }

    fn insert(&mut self, p: P, d: Dir) {
        match self.index(p) {
            Some(i) => {
                self.steps[i] = match d {
                    Dir::Up => 0,
                    Dir::Down => 1,
                    Dir::Left => 2,
                    Dir::Right => 3,
                    Dir::None => 4,
                }
            }
            None => {
                self.off.insert(p, d);
            }
        }
    }
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
    let mut first = Firsts::new(grid, from);
    let mut queue = VecDeque::from([from]);
    let mut best = (score(from), Dir::None);
    while let Some(p) = queue.pop_front() {
        let opening = first.get(p).unwrap_or(Dir::None);
        for d in CARDINALS {
            if !grid.can_move(p, d, allow_nest, flying) {
                continue;
            }
            let n = grid.neighbor(p, d);
            if first.get(n).is_some() || (avoid_bridges && grid.is_bridge(n.x, n.y)) {
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
    let mut first = Firsts::new(grid, from);
    let mut queue = VecDeque::from([from]);
    while let Some(p) = queue.pop_front() {
        let opening = first.get(p).unwrap_or(Dir::None);
        for d in CARDINALS {
            if !grid.can_move(p, d, allow_nest, flying) {
                continue;
            }
            let n = grid.neighbor(p, d);
            if first.get(n).is_some() {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The search as it was, on a hash map, to hold the dense one to.
    fn route_by_map(grid: &Grid, from: P, target: V, flying: bool, allow_nest: bool, avoid_bridges: bool) -> Option<Dir> {
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

    #[test]
    fn the_dense_search_finds_the_same_ways_as_the_hash_map_did() {
        for index in 0..3 {
            let level = crate::logic::level::parse_level(&crate::logic::levels::level_for(index));
            let g = &level.grid;
            let open: Vec<P> = g.cells().filter(|c| g.is_walkable(c.0, c.1, false)).map(|c| P::new(c.0, c.1)).collect();
            for (k, &from) in open.iter().enumerate().step_by(7) {
                let to = open[(k * 31 + 5) % open.len()];
                for (flying, nest, bridges) in [(false, false, false), (true, false, false), (false, true, true)] {
                    let target = V::new(to.x as f64 + 0.3, to.y as f64 - 0.2);
                    assert_eq!(
                        route_toward(g, from, target, flying, nest, bridges),
                        route_by_map(g, from, target, flying, nest, bridges),
                        "level {index}, {from:?} to {to:?}"
                    );
                }
                let goal = open[(k * 17 + 3) % open.len()];
                assert_eq!(
                    step_toward(g, from, goal, false, true).is_some(),
                    from != goal && reachable(g, from, false, true).contains(&goal)
                );
            }
        }
    }
}
