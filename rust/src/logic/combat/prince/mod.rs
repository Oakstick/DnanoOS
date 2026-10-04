//! The fourth archetype: a sin's prince (COMBAT.md §13). The same fighter, frames, guard, parry
//! and threat rules as every foe; what makes him a boss is a few numbers more:
//!
//! - **Phases by health.** Above two thirds he fights with one table of moves, down to a third
//!   with a second, below it with a third, quicker on his feet and quicker to strike again
//!   (`phase_of`, `table`, `pace`). Each change is a cue for the game: a banner, a breather.
//! - **Armour windows.** A prince's hide (Mammon's is sewn with coins) turns most of a blow
//!   outside his recoveries, damage and poise alike (`HIDE_SHARE`), as Tepegöz's ring does.
//! - **A token of three.** He fills the threat manager's pool by himself; what he raises shares
//!   the rest.
//!
//! What is shared lives here: the phases, the hide, the coins on the floor (`Trap`, `scatter`).
//! Each prince's own table, pace and phase names are his module's: `greed` (Mammon), `pride`
//! (Lucifer), `envy` (Leviathan), `wrath` (Amon), `sloth` (Belphegor), `gluttony` (Beelzebub)
//! and `lust` (Asmodeus); `table`, `pace` and `phase_name` here choose by the prince's kind.

pub mod envy;
pub mod gluttony;
pub mod greed;
pub mod lust;
pub mod pride;
pub mod sloth;
pub mod wrath;

pub use greed::{COIN_FLIGHT, DRAIN_EVERY, DRAIN_HEAL, DRAIN_SILVER, FIST, GOLEMS, GOLEM_RISE, GRASP_HOLD, SPIKES};

use super::frames::MoveId;
use super::math::{add, from_angle, len, scale, sub};
use crate::logic::enemy::EnemyKind;
use crate::logic::types::V;
use std::f64::consts::TAU;

/// A lingering trap's ground hurts him once every this many frames while he stands in it, for
/// this share of the trap's damage (it never slows him).
pub const LINGER_EVERY: u16 = 30;
pub const LINGER_SHARE: f64 = 0.25;

/// His health's shares below which the second and the third phase begin.
pub const PHASE_AT: [f64; 2] = [2.0 / 3.0, 1.0 / 3.0];
/// The share of a blow his hide lets through outside his recoveries.
pub const HIDE_SHARE: f64 = 0.3;
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

/// The prince of `kind`'s moves in `phase`, in the order he prefers them (each prince's table is
/// his own module's).
pub fn table(kind: EnemyKind, phase: u8) -> &'static [MoveId] {
    match kind {
        EnemyKind::Lucifer => pride::table(phase),
        EnemyKind::Leviathan => envy::table(phase),
        EnemyKind::Amon => wrath::table(phase),
        EnemyKind::Belphegor => sloth::table(phase),
        EnemyKind::Beelzebub => gluttony::table(phase),
        EnemyKind::Asmodeus => lust::table(phase),
        _ => greed::table(phase),
    }
}

/// The prince of `kind`'s pace in `phase` (a multiple of his walk) and his rest between moves (a
/// multiple of his archetype's).
pub fn pace(kind: EnemyKind, phase: u8) -> (f64, f64) {
    match kind {
        EnemyKind::Lucifer => pride::pace(phase),
        EnemyKind::Leviathan => envy::pace(phase),
        EnemyKind::Amon => wrath::pace(phase),
        EnemyKind::Belphegor => sloth::pace(phase),
        EnemyKind::Beelzebub => gluttony::pace(phase),
        EnemyKind::Asmodeus => lust::pace(phase),
        _ => greed::pace(phase),
    }
}

/// The title of the prince of `kind`'s `phase`, for the banner and the boss bar.
pub fn phase_name(kind: EnemyKind, phase: u8) -> &'static str {
    match kind {
        EnemyKind::Lucifer => pride::phase_name(phase),
        EnemyKind::Leviathan => envy::phase_name(phase),
        EnemyKind::Amon => wrath::phase_name(phase),
        EnemyKind::Belphegor => sloth::phase_name(phase),
        EnemyKind::Beelzebub => gluttony::phase_name(phase),
        EnemyKind::Asmodeus => lust::phase_name(phase),
        _ => greed::phase_name(phase),
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
    /// Frames after it bursts that its ground goes on hurting him while he stands in it (0: none).
    pub linger: u16,
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

    /// Still to burst, or its spikes still standing (or its ground still hurting).
    pub fn live(&self) -> bool {
        self.burst.is_none_or(|b| b < SPIKES.max(self.linger))
    }

    /// Burst, and its ground still hurting (a lingering trap's).
    pub fn lingering(&self) -> bool {
        self.burst.is_some_and(|b| b > 0 && b < self.linger)
    }

    /// The frame its lingering ground hurts again (every `LINGER_EVERY` after the burst).
    pub fn stings(&self) -> bool {
        self.lingering() && self.burst.is_some_and(|b| b.is_multiple_of(LINGER_EVERY))
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
            assert_eq!(moves, table(EnemyKind::Mammon, phase));
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
            linger: 0,
        };
        let bursts: Vec<bool> = (0..8).map(|_| t.step()).collect();
        assert_eq!(bursts.iter().filter(|b| **b).count(), 1);
        assert_eq!(bursts.iter().position(|b| *b), Some(7), "after its flight and its fuse");
        assert!(t.reaches(V::new(10.9, 10.0), 0.3) && !t.reaches(V::new(11.2, 10.0), 0.3));
        for _ in 0..SPIKES {
            t.step();
        }
        assert!(!t.live());
        assert!(!t.lingering() && !t.stings(), "the coins do not linger");
    }

    #[test]
    fn a_lingering_traps_ground_stings_every_so_often_after_it_bursts_then_is_gone() {
        let mut t = Trap {
            pos: V::new(5.0, 5.0),
            from: V::new(5.0, 5.0),
            flight: 0,
            fuse: 2,
            full: 2,
            radius: 0.8,
            damage: 10.0,
            owner: 1,
            volley: 1,
            burst: None,
            fell: false,
            linger: 100,
        };
        let mut stings = 0;
        let mut frames = 0;
        while t.live() {
            t.step();
            stings += t.stings() as u32;
            frames += 1;
            assert!(frames < 500);
        }
        assert_eq!(stings, (100 - 1) / LINGER_EVERY as u32, "once every {LINGER_EVERY} frames while it lingers");
        assert!(frames > 100 && !t.lingering());
    }
}
