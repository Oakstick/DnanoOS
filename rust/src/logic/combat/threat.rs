//! The threat manager (COMBAT.md §4): who may attack, and when, and where the rest wait.
//!
//! Attack **tokens** come out of a pool of threat points, no faster than the cadence, and only
//! to a foe whose strike would land far enough from every other's. A token holder may begin its
//! move only if its first active frame clears every committed strike (the commit gate), so no
//! two attacks land inside the reaction window. Foes without a token hold **sectors** of a
//! ring around the hero, to his sides and front.

use super::math::{angle, from_angle, scale};
use crate::logic::types::V;
use std::f64::consts::{PI, TAU};

/// Threat points in the pool.
pub const CAPACITY: u8 = 3;
/// Frames between two grants, at least.
pub const CADENCE: u32 = 30;
/// Frames between any two strikes' first active frames, at least (300 ms).
pub const SPACING: u32 = 18;
/// Frames a token may go unspent past its projected strike before it returns (at least `TIMEOUT` in all).
pub const TIMEOUT: u32 = 150;
pub const LATE: u32 = 60;
/// Frames a foe waits before asking again when its token is taken back.
pub const REVOKE_REST: u16 = 45;
/// The ring is cut into this many sectors.
pub const SECTORS: usize = 8;
/// Extra cost (radians) of a sector in the hero's back arc: more than any turn, so his back fills last.
pub const BACK_PENALTY: f64 = 3.2;
/// A foe keeps its sector unless another is this much (radians) better.
pub const STICKY: f64 = 0.3;

/// A foe asking to attack.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Request {
    pub id: u32,
    pub cost: u8,
    /// Frames until its strike would land: the walk into range plus its startup.
    pub eta: u32,
    pub dist: f64,
    /// How far in front of the hero it stands: his aim against the way to it, -1 (behind) to 1.
    pub front: f64,
    /// Frames it has waited.
    pub wait: u16,
}

impl Request {
    /// Closeness, standing in front of the hero (readable), and patience (flanks happen, eventually).
    pub fn score(&self) -> f64 {
        1.0 * (1.0 - self.dist / 10.0).clamp(0.0, 1.0) + 0.8 * (1.0 + self.front) / 2.0 + 0.5 * (self.wait as f64 / 240.0).min(1.0)
    }
}

#[derive(Clone, Copy, Debug)]
struct Hold {
    id: u32,
    cost: u8,
    /// The frame its strike lands: projected until it commits, then fixed.
    active_at: u32,
    committed: bool,
    /// The frame by which it must have struck, or the token returns.
    deadline: u32,
    /// Given back, but its strike is still on its way (a bolt in flight): it no longer takes up
    /// the pool, and its landing frame still keeps the others clear until it has passed.
    spent: bool,
}

#[derive(Clone, Debug)]
pub struct ThreatManager {
    pub capacity: u8,
    pub frame: u32,
    holds: Vec<Hold>,
    last_grant: Option<u32>,
}

impl Default for ThreatManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ThreatManager {
    pub fn new() -> Self {
        Self { capacity: CAPACITY, frame: 0, holds: Vec::new(), last_grant: None }
    }

    pub fn tick(&mut self) {
        self.frame += 1;
        let frame = self.frame;
        self.holds.retain(|h| !h.spent || h.active_at + SPACING > frame);
    }

    /// Threat points in use.
    pub fn used(&self) -> u8 {
        self.holds.iter().filter(|h| !h.spent).map(|h| h.cost).sum()
    }

    pub fn holds(&self, id: u32) -> bool {
        self.holds.iter().any(|h| h.id == id && !h.spent)
    }

    /// The frames the committed strikes land on.
    pub fn committed(&self) -> Vec<u32> {
        self.holds.iter().filter(|h| h.committed).map(|h| h.active_at).collect()
    }

    /// Grants at most one token this frame: to the best-scored request that fits the pool and
    /// whose strike would land clear of every other's.
    pub fn grant(&mut self, requests: &[Request]) -> Option<u32> {
        if self.last_grant.is_some_and(|t| self.frame - t < CADENCE) {
            return None;
        }
        let mut order: Vec<&Request> = requests.iter().filter(|r| !self.holds(r.id)).collect();
        order.sort_by(|a, b| b.score().total_cmp(&a.score()).then(a.id.cmp(&b.id)));
        let used = self.used();
        for r in order {
            if used + r.cost > self.capacity {
                continue;
            }
            let at = self.frame + r.eta;
            if !self.holds.iter().all(|h| at.abs_diff(h.active_at) >= SPACING) {
                continue;
            }
            let deadline = self.frame + TIMEOUT.max(r.eta + LATE);
            self.holds.push(Hold { id: r.id, cost: r.cost, active_at: at, committed: false, deadline, spent: false });
            self.last_grant = Some(self.frame);
            return Some(r.id);
        }
        None
    }

    /// A holder still on its way: its strike now lands in `eta` frames (planning only).
    pub fn project(&mut self, id: u32, eta: u32) {
        let frame = self.frame;
        if let Some(h) = self.holds.iter_mut().find(|h| h.id == id && !h.committed && !h.spent) {
            h.active_at = frame + eta;
        }
    }

    /// The commit gate: may the holder begin a move that lands `lands` frames from now (its
    /// startup, and a bolt's flight)?
    pub fn may_commit(&self, id: u32, lands: u16) -> bool {
        let at = self.frame + lands as u32;
        self.holds.iter().filter(|h| (h.id != id || h.spent) && h.committed).all(|h| at.abs_diff(h.active_at) >= SPACING)
    }

    /// The holder began its move: its strike lands on a fixed frame.
    pub fn commit(&mut self, id: u32, lands: u16) {
        let at = self.frame + lands as u32;
        if let Some(h) = self.holds.iter_mut().find(|h| h.id == id && !h.spent) {
            h.active_at = at;
            h.committed = true;
        }
    }

    /// The holder gives its token back. A strike still on its way keeps its landing frame clear.
    pub fn release(&mut self, id: u32) {
        let frame = self.frame;
        self.holds.retain(|h| h.id != id || (h.committed && h.active_at + SPACING > frame));
        for h in self.holds.iter_mut().filter(|h| h.id == id) {
            h.spent = true;
        }
    }

    /// A token granted and not spent in time (its walk in, plus `LATE`; never under `TIMEOUT`).
    pub fn expired(&self, id: u32) -> bool {
        self.holds.iter().any(|h| h.id == id && !h.committed && !h.spent && self.frame > h.deadline)
    }

    pub fn clear(&mut self) {
        self.holds.clear();
        self.last_grant = None;
    }
}

/// A waiting foe, for the sectors: who, where, the radius of its ring, and its sector last time.
#[derive(Clone, Copy, Debug)]
pub struct Waiter {
    pub id: u32,
    pub pos: V,
    pub ring: f64,
    pub prev: Option<usize>,
}

/// Where a waiting foe should stand: its sector, and the point on its ring.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slot {
    pub id: u32,
    pub sector: usize,
    pub target: V,
}

fn wrap(a: f64) -> f64 {
    let mut a = (a + PI).rem_euclid(TAU) - PI;
    if a <= -PI {
        a += TAU;
    }
    a
}

/// The angle of sector `k` (world-fixed, so the slots stay put while the hero turns).
pub fn sector_angle(k: usize) -> f64 {
    k as f64 * TAU / SECTORS as f64
}

/**
 * Hands each waiting foe a sector of its ring: the nearest foes choose first, each the free
 * sector closest to its bearing, keeping its old one unless another is clearly better. Sectors
 * in the hero's back arc cost extra, and so does one whose point the arena (`lo`..`hi`) cuts off.
 * Melee rings (under 5 tiles) and the ranged ring share no sectors among themselves.
 */
pub fn assign_slots(hero: V, aim: V, waiters: &[Waiter], lo: V, hi: V) -> Vec<Slot> {
    let mut order: Vec<&Waiter> = waiters.iter().collect();
    let d2 = |w: &Waiter| (w.pos.x - hero.x).powi(2) + (w.pos.y - hero.y).powi(2);
    order.sort_by(|a, b| d2(a).total_cmp(&d2(b)).then(a.id.cmp(&b.id)));
    let back = angle(aim) + PI;
    let mut taken = [[false; SECTORS]; 2];
    let mut out = Vec::new();
    for w in order {
        let band = usize::from(w.ring >= 5.0);
        let bearing = (w.pos.y - hero.y).atan2(w.pos.x - hero.x);
        let point = |k: usize| {
            let p = scale(from_angle(sector_angle(k)), w.ring);
            V::new(hero.x + p.x, hero.y + p.y)
        };
        let cost = |k: usize| {
            let p = point(k);
            let mut c = wrap(sector_angle(k) - bearing).abs();
            if wrap(sector_angle(k) - back).abs() <= PI / 4.0 + 1e-6 {
                c += BACK_PENALTY;
            }
            if p.x < lo.x || p.x > hi.x || p.y < lo.y || p.y > hi.y {
                c += 1.0;
            }
            if w.prev == Some(k) {
                c -= STICKY;
            }
            c
        };
        let best = (0..SECTORS).filter(|&k| !taken[band][k]).min_by(|&a, &b| cost(a).total_cmp(&cost(b)));
        let Some(k) = best else { continue };
        taken[band][k] = true;
        let p = point(k);
        out.push(Slot { id: w.id, sector: k, target: V::new(p.x.clamp(lo.x, hi.x), p.y.clamp(lo.y, hi.y)) });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(id: u32, cost: u8, eta: u32) -> Request {
        Request { id, cost, eta, dist: 3.0, front: 0.0, wait: 0 }
    }

    #[test]
    fn the_pool_holds_three_points_and_tokens_come_no_faster_than_the_cadence() {
        let mut t = ThreatManager::new();
        assert_eq!(t.grant(&[req(1, 2, 40)]), Some(1), "the Brute");
        assert_eq!(t.grant(&[req(2, 1, 100)]), None, "not within the cadence");
        for _ in 0..CADENCE {
            t.tick();
        }
        assert_eq!(t.grant(&[req(2, 1, 100)]), Some(2));
        for _ in 0..CADENCE {
            t.tick();
        }
        assert_eq!(t.grant(&[req(3, 1, 200)]), None, "3 of 3 points in use");
        t.release(1);
        assert_eq!(t.grant(&[req(3, 1, 200)]), Some(3));
        assert_eq!(t.used(), 2);
    }

    #[test]
    fn no_token_goes_to_a_strike_that_would_land_on_top_of_another() {
        let mut t = ThreatManager::new();
        t.grant(&[req(1, 1, 50)]);
        for _ in 0..CADENCE {
            t.tick();
        }
        // The first lands on frame 50; a strike landing on frame 30 + 30 = 60 is too near.
        assert_eq!(t.grant(&[req(2, 1, 30)]), None);
        assert_eq!(t.grant(&[req(2, 1, 38)]), Some(2), "frame 68 is clear of 50");
        // The commit gate: a holder in range waits until its strike clears the committed one.
        let mut t = ThreatManager::new();
        t.grant(&[req(1, 1, 18)]);
        for _ in 0..CADENCE {
            t.tick();
        }
        assert_eq!(t.grant(&[req(2, 1, 60)]), Some(2));
        t.commit(1, 18);
        assert!(!t.may_commit(2, 18), "both would land on frame 48");
        for _ in 0..SPACING - 1 {
            t.tick();
        }
        assert!(!t.may_commit(2, 18));
        t.tick();
        assert!(t.may_commit(2, 18), "eighteen frames later it clears");
        // A bolt still in flight when its caster gives the token back keeps its landing clear.
        let mut t = ThreatManager::new();
        t.grant(&[req(1, 1, 40)]);
        t.commit(1, 90); // startup and a long flight
        t.release(1);
        assert_eq!(t.used(), 0, "the caster's point is free again");
        for _ in 0..CADENCE {
            t.tick();
        }
        t.grant(&[req(2, 1, 200)]);
        assert!(!t.may_commit(2, 60), "frame 90 is the bolt's");
        assert!(t.may_commit(2, 60 + SPACING as u16));
        for _ in 0..90 + SPACING - CADENCE {
            t.tick();
        }
        assert!(t.committed().is_empty(), "landed, and forgotten");
    }

    #[test]
    fn the_foe_in_front_and_near_gets_the_token_first() {
        let near_front = Request { id: 1, cost: 1, eta: 20, dist: 2.0, front: 1.0, wait: 0 };
        let near_behind = Request { id: 2, cost: 1, eta: 20, dist: 2.0, front: -1.0, wait: 0 };
        let far_patient = Request { id: 3, cost: 1, eta: 60, dist: 8.0, front: -1.0, wait: 600 };
        assert!(near_front.score() > near_behind.score());
        assert!(near_behind.score() > far_patient.score() - 0.5);
        let mut t = ThreatManager::new();
        assert_eq!(t.grant(&[near_behind, far_patient, near_front]), Some(1));
        // An unspent token comes back.
        for _ in 0..TIMEOUT {
            t.tick();
        }
        assert!(!t.expired(1));
        t.tick();
        assert!(t.expired(1));
        // A far walk in earns a longer leash: its walk, and LATE frames more.
        let mut t = ThreatManager::new();
        t.grant(&[Request { id: 9, cost: 2, eta: 300, dist: 9.0, front: 1.0, wait: 0 }]);
        for _ in 0..300 + LATE {
            t.tick();
        }
        assert!(!t.expired(9));
        t.tick();
        assert!(t.expired(9));
    }

    #[test]
    fn waiting_foes_take_different_sectors_away_from_his_back() {
        let hero = V::new(10.0, 10.0);
        let aim = V::new(1.0, 0.0); // facing east; his back is west
        let lo = V::new(0.0, 0.0);
        let hi = V::new(20.0, 20.0);
        let waiters: Vec<Waiter> =
            (0..4).map(|i| Waiter { id: i, pos: V::new(6.0, 10.0 + i as f64 * 0.1), ring: 4.0, prev: None }).collect();
        let slots = assign_slots(hero, aim, &waiters, lo, hi);
        assert_eq!(slots.len(), 4);
        let mut sectors: Vec<usize> = slots.iter().map(|s| s.sector).collect();
        sectors.sort();
        sectors.dedup();
        assert_eq!(sectors.len(), 4, "one foe to a sector");
        for s in &slots {
            let dx = s.target.x - hero.x;
            assert!(dx > -2.0, "no one waits square behind him: {s:?}");
            assert!(((s.target.x - hero.x).powi(2) + (s.target.y - hero.y).powi(2)).sqrt() > 3.9);
        }
        // A ranged foe shares a bearing with a melee one: different rings.
        let two = [
            Waiter { id: 1, pos: V::new(14.0, 10.0), ring: 4.0, prev: None },
            Waiter { id: 2, pos: V::new(16.0, 10.0), ring: 6.0, prev: None },
        ];
        let slots = assign_slots(hero, aim, &two, lo, hi);
        assert_eq!(slots[0].sector, slots[1].sector);
    }
}
