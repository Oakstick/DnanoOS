//! Krampus (FINALE.md §3): St Nicholas's dark companion of the Alpine winter, who comes on
//! Krampusnacht, the fifth of December, with chains and bells, a bundle of birch switches and a
//! basket on his back to carry bad children away. He waits at the bottom of the Pit with the friend
//! he took at the graveyard in a cage, and he is the last fight of the game.
//!
//! He fights as a prince does (the fourth archetype, COMBAT.md §13), but he is no sin's prince:
//! his phases fall at 70% and 40% of his health (`PHASE_AT`), not at the thirds, so his third, the
//! longest and the fiercest, comes sooner. The Bells lash with the birch, sweep the chain and toll;
//! the Chains lash the chain along a line, charge (stone dazes him: `DAZE`) and call his Perchten;
//! Krampusnacht flings live coals that burn on, swings his sack, and goes to the cage to rattle it
//! at the child, open all the while (`game::lair`). His pelt is a hide as Mammon's gold.

use super::MoveId;

/// His health's shares below which his second and third phases begin.
pub const PHASE_AT: [f64; 2] = [0.7, 0.4];

/// The phase of Krampus with `hp` of `max` health: 1, 2 or 3.
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

/// Frames he stands dazed after his charge has run into stone (staggered: every blow lands whole,
/// and more).
pub const DAZE: u16 = 150;
/// His Perchten, called as the second phase begins (once in a fight), and the frames they take to
/// come out of the dark.
pub const PERCHTEN: u8 = 2;
pub const PERCHT_RISE: u16 = 60;
/// What his Perchten are worth beside him: a share of a Percht's own health and blows.
pub const PERCHT_HEALTH: f64 = 1.0;
pub const PERCHT_DAMAGE: f64 = 0.8;
/// The cage taunt: frames into Krampusnacht before the first, frames between one and the next
/// (from the end of the last), the most frames he walks to the cage before giving it up, and how
/// many a fight holds at most.
pub const TAUNT_FIRST: u32 = 240;
pub const TAUNT_EVERY: u32 = 900;
pub const TAUNT_WALK: u32 = 480;
pub const TAUNTS: u32 = 3;
/// The bells on his chains (his death silences them one by one).
pub const BELLS: u8 = 7;

/// His moves in `phase`, in the order he prefers them: the Bells' toll, sweep and lash; the
/// Chains' lash along a line, charge, sweep and lash; Krampusnacht's coals, charge, chain and sack.
pub fn table(phase: u8) -> &'static [MoveId] {
    match phase {
        0 | 1 => &[MoveId::TollOfBells, MoveId::ChainSweep, MoveId::BirchLash],
        2 => &[MoveId::ChainLash, MoveId::HoofCharge, MoveId::ChainSweep, MoveId::BirchLash],
        _ => &[MoveId::CoalFire, MoveId::HoofCharge, MoveId::ChainLash, MoveId::SackSwing],
    }
}

/// His pace in `phase` (a multiple of his walk) and his rest between moves (a multiple of his
/// archetype's): the Chains a little quicker, Krampusnacht quicker still.
pub fn pace(phase: u8) -> (f64, f64) {
    match phase {
        0 | 1 => (1.0, 1.0),
        2 => (1.1, 0.85),
        _ => (1.3, 0.62),
    }
}

/// The title of `phase`, for the banner and the boss bar.
pub fn phase_name(phase: u8) -> &'static str {
    match phase {
        0 | 1 => "THE BELLS",
        2 => "THE CHAINS",
        _ => "KRAMPUSNACHT",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::combat::archetype::{Archetype, Breed, Power};
    use crate::logic::combat::frames::{Shape, REACTION};
    use crate::logic::combat::Duelist;
    use crate::logic::enemy::EnemyKind;
    use crate::logic::types::V;

    #[test]
    fn his_phases_fall_at_seventy_and_forty() {
        assert_eq!(phase_of(1000.0, 1000.0), 1);
        assert_eq!(phase_of(701.0, 1000.0), 1);
        assert_eq!(phase_of(700.0, 1000.0), 2);
        assert_eq!(phase_of(401.0, 1000.0), 2);
        assert_eq!(phase_of(400.0, 1000.0), 3);
        assert_eq!(phase_of(0.0, 1000.0), 3);
        assert_eq!(super::super::phase_for(EnemyKind::Krampus, 650.0, 1000.0), 2, "his own thresholds");
        assert_eq!(super::super::phase_for(EnemyKind::Mammon, 650.0, 1000.0), 2);
        assert_eq!(super::super::phase_for(EnemyKind::Mammon, 690.0, 1000.0), 1, "the princes' own");
    }

    #[test]
    fn krampus_fights_by_three_tables_and_every_move_keeps_the_rules() {
        let mut k = Breed::of(EnemyKind::Krampus);
        assert_eq!((k.arch, k.power, k.cost), (Archetype::Prince, Power::Pelt, 3));
        assert!(k.power.is_hide(), "his pelt is a hide");
        for prince in [EnemyKind::Mammon, EnemyKind::Amon, EnemyKind::Leviathan, EnemyKind::Beelzebub] {
            assert!(k.hp > Breed::of(prince).hp && k.radius >= Breed::of(prince).radius, "larger and longer than {prince:?}");
        }
        let mut seen = Vec::new();
        for phase in 1..=3 {
            k.phase = phase;
            let moves = k.moves();
            assert_eq!(moves, table(phase));
            assert!(moves.len() <= 4, "a cooldown for each");
            assert!(k.main_move().data().strikes() && !k.main_move().data().projectile, "he closes in to strike");
            for &mv in moves {
                let d = mv.data();
                assert!(MoveId::FOE.contains(&mv), "{}", d.name);
                // Readable, punishable (a prince's 30 frames).
                assert!(d.startup >= 30 && d.recovery >= 30, "{}", d.name);
                if !seen.contains(&mv) {
                    seen.push(mv);
                }
            }
        }
        assert_eq!(seen.len(), 7, "lash, sweep, toll, chain lash, charge, coals, sack: {seen:?}");
        // The taunt: no blow, readable, and a long open recovery.
        let t = MoveId::CageRattle.data();
        assert!(!t.strikes() && t.startup >= 30 && t.recovery >= 120 && MoveId::FOE.contains(&MoveId::CageRattle));
        // The bell toll shakes the floor round him; the sack swing too.
        assert!(matches!(MoveId::TollOfBells.data().shape, Shape::Quake { .. }));
        assert!(matches!(MoveId::SackSwing.data().shape, Shape::Quake { .. }));
        // The chain lash: a line on the floor, locked well before it flies; only the shield holds it.
        let lash = MoveId::ChainLash.data();
        assert!(matches!(lash.shape, Shape::Beam { .. }) && lash.blockable() && !lash.parriable());
        assert!(lash.startup - lash.track >= 20);
        // The charge runs its length in its active frames, its aim locked before he sets off.
        let c = MoveId::HoofCharge.data();
        let Shape::Charge { length, speed } = c.shape else { panic!("a charge") };
        assert!(length / speed * 60.0 <= c.active as f64, "the run fits its active frames");
        assert!(c.startup - c.track >= 12 && c.parriable() && c.blockable());
        // The coals: a row toward him that burns on, and lands late enough to walk off.
        let (count, radius, fuse, linger) = MoveId::CoalFire.data().traps().expect("a row of coals");
        assert!(matches!(MoveId::CoalFire.data().shape, Shape::TrapLine { .. }));
        assert!(count >= 4 && linger >= 120 && fuse > REACTION * 2 && radius < 1.0);
        // His reach: the birch past an arm, the chain's sweep farther and wider.
        let d = Duelist::bred(1, Breed::of(EnemyKind::Krampus), V::new(0.0, 0.0), V::new(0.0, 1.0));
        let (birch, sweep) = (d.data(MoveId::BirchLash), d.data(MoveId::ChainSweep));
        assert!(birch.reach > 2.0 && sweep.reach > birch.reach && sweep.half_angle > birch.half_angle);
        // Krampusnacht is quicker; still slower than the hero (6.5 x 0.8 = 5.2 t/s).
        k.phase = 1;
        let calm = k.stats();
        k.phase = 3;
        let night = k.stats();
        assert!(night.walk > calm.walk && night.rest < calm.rest);
        assert!(night.walk < 6.5 * 0.8 * 0.75, "{}", night.walk);
    }

    #[test]
    fn his_perchten_are_brutes_with_switches_lighter_than_him() {
        let p = Breed::of(EnemyKind::Percht);
        assert_eq!((p.arch, p.cost), (Archetype::Brute, 1));
        assert!(p.stats().hp < Breed::of(EnemyKind::Krampus).stats().hp / 5.0);
        assert!(p.moves().iter().all(|m| m.data().startup >= 18));
    }
}
