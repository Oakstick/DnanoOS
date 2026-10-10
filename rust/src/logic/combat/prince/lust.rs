//! Asmodeus, the prince of Lust (PRINCES.md §6): the lancer of the rose bower, three heads on him
//! (the man's smiling in the middle, a bull's and a ram's either side), a serpent's tail.
//!
//! - **The Suitor** thrusts his lance, long and narrow; blows a kiss, a pink heart that leaves
//!   whoever it strikes smitten (no guard, no parry, for `SMITTEN` frames: he still walks, leaps and
//!   swings); and makes thorns burst from the ground in a row toward the hero.
//! - **The Three Heads** lowers the bull's head and charges, butts with the ram's (a shove), and
//!   still kisses and thrusts; two elf-maids step from the hedges (once).
//! - **The Bridegroom of Ashes** makes thorns burst in a ring round him, and comes on quicker.
//!
//! His hide, **Allure**, turns most of a blow outside his recoveries, as Mammon's gold does, but
//! not while one of his court's censers smokes (Tobit: the smoke of the fish's heart and liver
//! drove him off). A blow lights a cold censer for `SMOKE` frames, and each lights `LIGHTS` times
//! in a fight (once, if his general's decree was done); while one smokes he will not stand within
//! `SHUN` tiles of it. (The censers are the court's: `game::court::lust`.)

use super::MoveId;

/// Frames a blown kiss leaves him smitten: no guard and no parry (he still walks, leaps and swings).
pub const SMITTEN: u16 = 90;
/// Frames a censer smokes once a blow lights it, and how many times each lights in a fight (once
/// each if his general's decree was done).
pub const SMOKE: u16 = 600;
pub const LIGHTS: u8 = 2;
pub const LIGHTS_DECREED: u8 = 1;
/// He will not stand nearer than this to a smoking censer (tiles, middle to middle), and backs
/// away from one this fast (tiles a second).
pub const SHUN: f64 = 3.0;
pub const SHUN_PACE: f64 = 3.2;
/// The elf-maids who step from the hedges as his second phase begins (once in a fight), and the
/// frames they take to step out.
pub const MAIDS: u8 = 2;
pub const MAID_RISE: u16 = 45;
/// The elf-maids' health and the weight of their blows, as shares of the realm's own (they serve him
/// here, lighter than in their garden).
pub const MAID_HEALTH: f64 = 0.8;
pub const MAID_DAMAGE: f64 = 0.6;
/// How far from his middle the thorns of his ring burst.
pub const RING: f64 = 2.15;

/// His moves in `phase`, in the order he prefers them (each but the lance with a cooldown of its
/// own, so the lance is what he closes in with): the Suitor's kiss, thorns and lance; the Three
/// Heads' charge, kiss, butt and lance; the Bridegroom's ring of thorns, charge, kiss and lance.
pub fn table(phase: u8) -> &'static [MoveId] {
    match phase {
        0 | 1 => &[MoveId::BlownKiss, MoveId::ThornLine, MoveId::LanceThrust],
        2 => &[MoveId::BullCharge, MoveId::BlownKiss, MoveId::RamButt, MoveId::LanceThrust],
        _ => &[MoveId::ThornRing, MoveId::BullCharge, MoveId::BlownKiss, MoveId::LanceThrust],
    }
}

/// His pace in `phase` (a multiple of his walk) and his rest between moves (a multiple of his
/// archetype's): the Bridegroom of Ashes comes on quicker, and strikes again sooner.
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
        0 | 1 => "THE SUITOR",
        2 => "THE THREE HEADS",
        _ => "THE BRIDEGROOM OF ASHES",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::combat::archetype::{Archetype, Breed, Power};
    use crate::logic::combat::frames::{Rider, Shape, REACTION};
    use crate::logic::combat::fsm::GuardRule;
    use crate::logic::enemy::EnemyKind;

    #[test]
    fn asmodeus_fights_with_a_table_for_each_phase_and_every_move_keeps_the_rules() {
        let mut m = Breed::of(EnemyKind::Asmodeus);
        assert_eq!((m.arch, m.power, m.cost), (Archetype::Prince, Power::Allure, 3));
        let mut seen = Vec::new();
        for phase in 1..=3 {
            m.phase = phase;
            let moves = m.moves();
            assert_eq!(moves, table(phase));
            assert!(moves.len() <= 4, "a cooldown for each");
            assert_eq!(m.main_move(), MoveId::LanceThrust, "he closes in with his lance");
            for &mv in moves {
                let d = mv.data();
                assert!(MoveId::FOE.contains(&mv));
                // Readable and punishable, as every prince (PRINCES.md §0).
                assert!(d.startup >= 30 && d.recovery >= 30, "{}", d.name);
                assert!(d.track < d.startup, "{}: his aim commits before the blow", d.name);
                assert!(mv == MoveId::LanceThrust || d.cooldown > 0, "{}", d.name);
            }
            seen.extend_from_slice(moves);
        }
        for mv in [MoveId::LanceThrust, MoveId::BlownKiss, MoveId::ThornLine, MoveId::BullCharge, MoveId::RamButt, MoveId::ThornRing] {
            assert!(seen.contains(&mv), "{mv:?} in a table");
        }
        // The lance: long and narrow. The kiss: one bolt. The thorns: from the ground, in a row and
        // in a ring. The bull charges, guardable; the ram shoves.
        let lance = MoveId::LanceThrust.data();
        assert!(lance.reach >= 2.2 && lance.half_angle < 0.4);
        assert!(MoveId::BlownKiss.data().projectile);
        assert!(matches!(MoveId::ThornLine.data().shape, Shape::TrapLine { .. }));
        assert!(matches!(MoveId::ThornRing.data().shape, Shape::Traps { .. }));
        assert_eq!(MoveId::ThornLine.data().guard_rule(), GuardRule::Bare);
        assert!(matches!(MoveId::BullCharge.data().shape, Shape::Charge { .. }));
        assert_eq!(MoveId::BullCharge.data().guard_rule(), GuardRule::Full);
        assert!(matches!(MoveId::RamButt.data().rider, Rider::Shove(_)));
        // The Bridegroom is quicker, and still slower than the hero.
        m.phase = 1;
        let calm = m.stats();
        m.phase = 3;
        let ash = m.stats();
        assert!(ash.walk > calm.walk && ash.rest < calm.rest && ash.walk < 6.5 * 0.8 * 0.6);
        // His smitten spell is only a while, and longer than a reaction.
        const { assert!(SMITTEN > REACTION * 3 && SMITTEN <= 120) };
        assert_eq!(phase_name(2), "THE THREE HEADS");
    }

    #[test]
    fn his_charge_runs_far_enough_and_its_run_fills_its_active_frames() {
        let d = MoveId::BullCharge.data();
        let Shape::Charge { length, speed } = d.shape else { panic!() };
        assert!(length >= d.range.1, "it reaches as far as he begins it from");
        let run = (length / speed * 60.0).round() as u16;
        assert!(d.active + 1 >= run && d.active <= run + 2, "{} active frames for a {run}-frame run", d.active);
    }
}
