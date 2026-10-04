//! Lucifer, the prince of Pride (PRINCES.md §1): the morning star fallen, at the foot of his throne.
//!
//! The Morning Star casts his Lance of Dawn, a beam of light along the floor toward the hero
//! (`Shape::Beam`), and sweeps his sword of light wide (Radiant Cut). The Mirror steps into one of
//! his court's mirrors and out of another near the hero, and two cold silver Reflections step out
//! of the glass beside him. The Fallen, his wings burnt black, brings the stars down round the hero
//! (Starfall), and rises on his wings to crash down on a mark that follows him (Pride's Fall).
//!
//! His hide is Vainglory: his light turns most of a blow outside his recoveries, but Pride never
//! looks behind him, and a blow from behind (more than `BEHIND` from where he faces) lands whole.

use super::MoveId;
use crate::logic::combat::math::{dot, len, norm, sub};
use crate::logic::types::V;

/// A blow from farther round than this from where he faces (radians, 100°) is behind him: his
/// vainglory does not turn it.
pub const BEHIND: f64 = 100.0 * std::f64::consts::PI / 180.0;

/// Is a blow from `from` behind one at `pos` facing `facing`?
pub fn behind(pos: V, facing: V, from: V) -> bool {
    let to = sub(from, pos);
    if len(to) < 1e-6 {
        return false;
    }
    dot(norm(to), norm(facing)).clamp(-1.0, 1.0).acos() > BEHIND
}

/// The Reflections that step out of the mirrors as the second phase begins (once in a fight), the
/// frames they take to step out of the glass, and their damage as a share of his.
pub const REFLECTIONS: u8 = 2;
pub const REFLECTION_RISE: u16 = 45;
pub const REFLECTION_DAMAGE: f64 = 0.5;

/// Pride's Fall: from this frame of its startup he is off the floor on his wings, out of reach
/// until he comes down on his mark (its first active frame).
pub const FALL_RISE: u16 = 12;

/// His moves in `phase`, in the order he prefers them: the Morning Star's lance and his cut; the
/// Mirror's step through the glass besides; the Fallen's stars, his fall, and the cut.
pub fn table(phase: u8) -> &'static [MoveId] {
    match phase {
        0 | 1 => &[MoveId::LanceOfDawn, MoveId::RadiantCut],
        2 => &[MoveId::MirrorStep, MoveId::LanceOfDawn, MoveId::RadiantCut],
        _ => &[MoveId::Starfall, MoveId::PridesFall, MoveId::RadiantCut],
    }
}

/// His pace in `phase` (a multiple of his walk) and his rest between moves (a multiple of his
/// archetype's): the Fallen comes on quicker, and strikes again sooner.
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
        0 | 1 => "THE MORNING STAR",
        2 => "THE MIRROR",
        _ => "THE FALLEN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::combat::archetype::{Archetype, Breed, Power};
    use crate::logic::combat::frames::{Shape, REACTION};
    use crate::logic::combat::fsm::GuardRule;
    use crate::logic::enemy::EnemyKind;

    #[test]
    fn lucifer_fights_with_three_tables_each_readable_and_punishable() {
        let mut b = Breed::of(EnemyKind::Lucifer);
        assert_eq!((b.arch, b.power, b.cost), (Archetype::Prince, Power::Vainglory, 3));
        let calm = b.stats();
        for phase in 1..=3 {
            b.phase = phase;
            assert_eq!(b.moves(), table(phase));
            assert!(b.moves().len() <= 4, "a cooldown for each");
            assert_eq!(b.main_move(), MoveId::RadiantCut, "he closes in to cut");
            for &mv in b.moves() {
                let d = mv.data();
                assert!(MoveId::FOE.contains(&mv));
                assert!(d.startup >= 30, "{}: {} f is quicker than a prince may be", d.name, d.startup);
                assert!(d.recovery >= 30, "{}: {} f of recovery", d.name, d.recovery);
            }
        }
        assert_eq!(phase_name(1), "THE MORNING STAR");
        assert_eq!(phase_name(2), "THE MIRROR");
        assert_eq!(phase_name(3), "THE FALLEN");
        // The Fallen is quicker on his feet and quicker to strike again, and still slower than the hero.
        let fallen = b.stats();
        assert!(fallen.walk > calm.walk && fallen.rest < calm.rest);
        assert!(fallen.walk < 6.5 * 0.8 * 0.6);
        // His lance is a beam the shield holds but no parry turns; the stars burst from the floor; he
        // falls on a mark; the mirror step is no blow.
        assert!(matches!(MoveId::LanceOfDawn.data().shape, Shape::Beam { .. }));
        assert_eq!(MoveId::LanceOfDawn.data().guard_rule(), GuardRule::BlockOnly);
        assert!(matches!(MoveId::Starfall.data().shape, Shape::Traps { .. }));
        assert!(matches!(MoveId::PridesFall.data().shape, Shape::Mark { .. }));
        assert!(!MoveId::MirrorStep.data().strikes() && MoveId::MirrorStep.data().shape == Shape::Spell);
        assert_eq!(MoveId::RadiantCut.data().guard_rule(), GuardRule::Full);
        // The mirror step leaves him long in the open as he comes out of the glass.
        assert!(MoveId::MirrorStep.data().recovery >= 60);
    }

    #[test]
    fn the_lance_of_dawn_stops_following_him_in_time_to_step_out_of_its_line() {
        // The line glows from his first startup frame and follows the hero through its tracking;
        // locked, it leaves him (setting out on seeing it lock, at his own pace, less what getting
        // going costs) time to clear its half-width and his own with a fifth to spare.
        let walk = 6.5 * crate::logic::levels::SPEED_SCALE;
        let d = MoveId::LanceOfDawn.data();
        let Shape::Beam { width, length } = d.shape else { panic!("a beam") };
        let after_lock = d.startup.saturating_sub(d.track).saturating_sub(REACTION) as f64;
        let gained = after_lock * walk / 60.0 - 0.25;
        assert!(gained >= (width / 2.0 + 0.3) * 1.2, "{gained:.2} t to clear {:.2} t", width / 2.0);
        assert!(length >= d.range.1 + 1.0, "it reaches past the farthest he casts it from");
    }

    #[test]
    fn a_blow_from_behind_him_is_one_from_more_than_a_hundred_degrees_round() {
        let (pos, facing) = (V::new(5.0, 5.0), V::new(0.0, 1.0));
        assert!(!behind(pos, facing, V::new(5.0, 7.0)), "before him");
        assert!(!behind(pos, facing, V::new(7.0, 5.0)), "beside him");
        assert!(!behind(pos, facing, V::new(7.0, 4.7)), "a little behind his shoulder, still seen");
        assert!(behind(pos, facing, V::new(7.0, 4.0)), "round behind his shoulder");
        assert!(behind(pos, facing, V::new(5.0, 3.0)), "at his back");
    }
}
