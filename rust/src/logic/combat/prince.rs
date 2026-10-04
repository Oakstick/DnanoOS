//! The fourth archetype: a sin's prince (COMBAT.md §13). The same fighter, frames, guard, parry
//! and threat rules as every foe; what makes him a boss is a few numbers more:
//!
//! - **Phases by health.** Above two thirds he fights with one table of moves, down to a third
//!   with a second, below it with a third, quicker on his feet and quicker to strike again
//!   (`phase_of`, `table`, `pace`). Each change is a cue for the game: a banner, a breather.
//! - **Armour windows.** Mammon's hide is sewn with coins: outside his recoveries it turns most of
//!   a blow, damage and poise alike (`HIDE_SHARE`), as Tepegöz's ring does.
//! - **A token of three.** He fills the threat manager's pool by himself; what he raises shares
//!   the rest.
//!
//! Mammon, the prince of Greed, fights so: the Banker sweeps his sceptre and tosses coins that lie
//! on the floor and burst into gold spikes (`Trap`); the Collector raises two gold golems from his
//! heaps and grasps, holding him and draining his silver until he is struck; the Hoard rains coins
//! from above, flings them from afar, and brings his hoard down on the floor.

use super::frames::MoveId;
use super::math::{add, from_angle, len, scale, sub};
use crate::logic::types::V;
use std::f64::consts::TAU;

/// His health's shares below which the second and the third phase begin.
pub const PHASE_AT: [f64; 2] = [2.0 / 3.0, 1.0 / 3.0];
/// The share of a blow his hide lets through outside his recoveries.
pub const HIDE_SHARE: f64 = 0.3;
/// Frames his fist may hold the hero before it lets go of itself, and how often it drains him:
/// the silver it takes each time, and the share of his whole health each draught gives back.
pub const GRASP_HOLD: u16 = 300;
pub const DRAIN_EVERY: u16 = 30;
pub const DRAIN_SILVER: u32 = 10;
pub const DRAIN_HEAL: f64 = 0.012;
/// How far from his middle the held hero stands: in his fist, within reach of a blow.
pub const FIST: f64 = 0.75;
/// The gold golems raised from his heaps as the second phase begins (once in a fight), and the
/// frames they take to climb out of the gold.
pub const GOLEMS: u8 = 2;
pub const GOLEM_RISE: u16 = 60;
/// Frames a coin takes to fly from his hand to the floor (or to fall from above).
pub const COIN_FLIGHT: u16 = 16;
/// Frames the gold spikes stand after a coin bursts (for the view).
pub const SPIKES: u16 = 24;

/// The phase of a prince with `hp` of `max` health: 1, 2 or 3.
pub fn phase_of(hp: f64, max: f64) -> u8 {
    let share = hp / max.max(1e-9);
    if share > PHASE_AT[0] {
        1
    } else if share > PHASE_AT[1] {
        2
    } else {
        3
    }
}

/// His moves in `phase`, in the order he prefers them: the Banker's sceptre and coin toss; the
/// Collector's grasp besides; the Hoard's rain of coins, his slam and volley, and the sceptre.
pub fn table(phase: u8) -> &'static [MoveId] {
    match phase {
        0 | 1 => &[MoveId::CoinToss, MoveId::Sceptre],
        2 => &[MoveId::Grasp, MoveId::CoinToss, MoveId::Sceptre],
        _ => &[MoveId::CoinRain, MoveId::HoardSlam, MoveId::CoinVolley, MoveId::Sceptre],
    }
}

/// His pace in `phase` (a multiple of his walk) and his rest between moves (a multiple of his
/// archetype's): the Hoard comes on quicker, and strikes again sooner.
pub fn pace(phase: u8) -> (f64, f64) {
    if phase >= 3 {
        (1.35, 0.6)
    } else {
        (1.0, 1.0)
    }
}

/// The title of `phase`, for the banner and the boss bar.
pub fn phase_name(phase: u8) -> &'static str {
    match phase {
        0 | 1 => "THE BANKER",
        2 => "THE COLLECTOR",
        _ => "THE HOARD",
    }
}

/// A coin on the floor that will burst into gold spikes: where, frames until it lands and then
/// until it bursts, how wide, how hard, who threw it (its throw's coins burst together, and land
/// as one strike), and frames since it burst (the spikes standing, for the view).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trap {
    pub pos: V,
    /// Where it was thrown from (or high above, for the rain), for its flight.
    pub from: V,
    pub flight: u16,
    pub fuse: u16,
    /// Its fuse when it landed, for the view's tell filling.
    pub full: u16,
    pub radius: f64,
    pub damage: f64,
    pub owner: u32,
    pub volley: u32,
    pub burst: Option<u16>,
    /// It fell from above (the rain) rather than flew from his hand.
    pub fell: bool,
}

impl Trap {
    /// One frame: it flies, then its fuse burns. True on the frame it bursts.
    pub fn step(&mut self) -> bool {
        if let Some(b) = self.burst.as_mut() {
            *b += 1;
            return false;
        }
        if self.flight > 0 {
            self.flight -= 1;
            return false;
        }
        if self.fuse > 0 {
            self.fuse -= 1;
        }
        if self.fuse == 0 {
            self.burst = Some(0);
            return true;
        }
        false
    }

    /// Still to burst, or its spikes still standing.
    pub fn live(&self) -> bool {
        self.burst.is_none_or(|b| b < SPIKES)
    }

    /// Lying on the floor with its fuse burning.
    pub fn armed(&self) -> bool {
        self.flight == 0 && self.burst.is_none()
    }

    /// Does its burst reach a body of `radius` at `p`?
    pub fn reaches(&self, p: V, radius: f64) -> bool {
        len(sub(p, self.pos)) <= self.radius + radius
    }

    /// 0 to 1 through its fuse once landed.
    pub fn tell(&self) -> f32 {
        if self.flight > 0 {
            0.0
        } else {
            1.0 - self.fuse as f32 / self.full.max(1) as f32
        }
    }
}

/// Where `count` coins land about the hero at `hero`: the first where he stands, the rest in a
/// ring round him (`spread` tiles out), turned by `turn` so no two throws lie alike. A rain's
/// coins lie in two rings, near and far.
pub fn scatter(count: u8, hero: V, spread: f64, turn: f64, rain: bool) -> Vec<V> {
    let mut out = vec![hero];
    let n = count.saturating_sub(1) as usize;
    for k in 0..n {
        let a = turn + k as f64 * TAU / n.max(1) as f64;
        let r = if rain && k % 2 == 1 { spread * 1.7 } else { spread };
        out.push(add(hero, scale(from_angle(a), r)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::combat::archetype::{Archetype, Breed, Power};
    use crate::logic::combat::frames::{Rider, Shape};
    use crate::logic::enemy::EnemyKind;

    #[test]
    fn his_phases_change_at_two_thirds_and_one_third() {
        assert_eq!(phase_of(900.0, 900.0), 1);
        assert_eq!(phase_of(601.0, 900.0), 1);
        assert_eq!(phase_of(600.0, 900.0), 2);
        assert_eq!(phase_of(301.0, 900.0), 2);
        assert_eq!(phase_of(300.0, 900.0), 3);
        assert_eq!(phase_of(0.0, 900.0), 3);
    }

    #[test]
    fn the_prince_is_a_fourth_archetype_with_a_table_for_every_phase() {
        let mut m = Breed::of(EnemyKind::Mammon);
        assert_eq!((m.arch, m.power, m.cost, m.phase), (Archetype::Prince, Power::GoldHide, 3, 1));
        assert_eq!(Archetype::Prince.stats().cost, 3, "his token fills the pool");
        let calm = m.stats();
        let mut seen = Vec::new();
        for phase in 1..=3 {
            m.phase = phase;
            let moves = m.moves();
            assert_eq!(moves, table(phase));
            assert!(moves.len() <= 4, "a cooldown for each");
            assert!(moves.iter().all(|mv| MoveId::FOE.contains(mv)));
            assert!(m.main_move().data().strikes() && !m.main_move().data().projectile, "he closes in to strike");
            seen.extend_from_slice(moves);
        }
        // The Banker's coins are traps, the Collector grasps, the Hoard rains coins and slams.
        assert!(matches!(MoveId::CoinToss.data().shape, Shape::Traps { .. }));
        assert_eq!(MoveId::Grasp.data().rider, Rider::Grasp);
        assert!(!MoveId::Grasp.data().blockable() && MoveId::Grasp.data().parriable());
        assert!(matches!(MoveId::CoinRain.data().shape, Shape::Traps { .. }));
        assert!(matches!(MoveId::HoardSlam.data().shape, Shape::Quake { .. }));
        assert!(MoveId::CoinVolley.data().projectile);
        for mv in [MoveId::Sceptre, MoveId::CoinToss, MoveId::Grasp, MoveId::CoinVolley, MoveId::CoinRain, MoveId::HoardSlam] {
            assert!(seen.contains(&mv), "{mv:?} in a table");
        }
        // The Hoard is quicker on his feet and quicker to strike again.
        let hoard = m.stats();
        assert!(hoard.walk > calm.walk && hoard.rest < calm.rest);
        // Still slower than the hero (6.5 x 0.8 = 5.2 t/s): he can always walk away.
        assert!(hoard.walk < 6.5 * 0.8 * 0.6);
    }

    #[test]
    fn coins_land_on_him_and_round_him_and_burst_after_their_fuse() {
        let hero = V::new(10.0, 10.0);
        let spots = scatter(5, hero, 1.8, 0.3, false);
        assert_eq!(spots.len(), 5);
        assert_eq!(spots[0], hero);
        assert!(spots[1..].iter().all(|p| (len(sub(*p, hero)) - 1.8).abs() < 1e-9));
        let rain = scatter(7, hero, 1.6, 0.0, true);
        assert!(rain.iter().any(|p| len(sub(*p, hero)) > 2.5), "a far ring too");
        let mut t = Trap {
            pos: hero,
            from: hero,
            flight: 3,
            fuse: 5,
            full: 5,
            radius: 0.8,
            damage: 10.0,
            owner: 1,
            volley: 1,
            burst: None,
            fell: false,
        };
        let bursts: Vec<bool> = (0..8).map(|_| t.step()).collect();
        assert_eq!(bursts.iter().filter(|b| **b).count(), 1);
        assert_eq!(bursts.iter().position(|b| *b), Some(7), "after its flight and its fuse");
        assert!(t.reaches(V::new(10.9, 10.0), 0.3) && !t.reaches(V::new(11.2, 10.0), 0.3));
        for _ in 0..SPIKES {
            t.step();
        }
        assert!(!t.live());
    }
}
