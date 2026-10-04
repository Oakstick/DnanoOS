//! Beelzebub, the prince of Gluttony (PRINCES.md §5): the lord of the flies at his banquet.
//!
//! The Lord of Flies chops with his cleaver and flings fans of slow flies; the Swarm bursts into
//! flies and gathers again on the hero, landing with a belly slam, and two fly swarms rise from the
//! food; the Glutton, swollen and quicker, rolls on his belly, spews bile that lies on the floor
//! and burns, and slams his belly down. In every phase, hurt, he goes to the platters on his table
//! to eat (`Gorge`): struck while he eats he chokes, staggered and open, and the platter is
//! spoiled. Each platter feeds him once, and he eats a green cross the hero leaves on the floor.
//! What he does with his food is the court's (`game::court::gluttony`); his numbers are here.

use super::MoveId;

/// He goes to eat once his health is under this share of its whole.
pub const GORGE_BELOW: f64 = 0.85;
/// The share of his whole health a platter gives back, and a green cross.
pub const GORGE_HEAL: f64 = 0.09;
pub const CROSS_HEAL: f64 = 0.05;
/// Frames between his meals (from the end of one to the setting out for the next), and the
/// longest he walks to his food before he gives it up.
pub const GORGE_REST: u32 = 540;
pub const GORGE_WALK: u32 = 600;
/// Frames a green cross must lie on the floor before he goes for it (the hero takes it first).
pub const CROSS_GRACE: u32 = 150;
/// Frames he reels, choking, struck while he eats (his hide open).
pub const CHOKE: u16 = 100;
/// The fly swarms that rise from the food as his second phase begins (once in a fight), and the
/// frames they take to gather.
pub const SWARMS: u8 = 2;
pub const SWARM_RISE: u16 = 50;
/// His flies fly at this share of a bolt's pace (they arrive later, on a frame the threat manager
/// plans; if it cannot, at a bolt's pace).
pub const FLY_PACE: f64 = 0.62;
/// The scatter: the startup frame he bursts into flies, and the frames before his slam that he
/// gathers again on the floor (on his mark).
pub const SCATTER_SINK: u16 = 12;
pub const SCATTER_GATHER: u16 = 8;

/// His moves in `phase`, in the order he prefers them: the Lord of Flies' swarm and cleaver; the
/// Swarm's scatter besides; the Glutton's roll, bile and slam. (His meal, `Gorge`, is in every
/// table, for its place among his moves; the court, not his choice, sends him to his food.)
pub fn table(phase: u8) -> &'static [MoveId] {
    match phase {
        0 | 1 => &[MoveId::Swarm, MoveId::Cleaver, MoveId::Gorge],
        2 => &[MoveId::Scatter, MoveId::Swarm, MoveId::Cleaver, MoveId::Gorge],
        _ => &[MoveId::BellyRoll, MoveId::BileSpew, MoveId::BellySlam, MoveId::Gorge],
    }
}

/// His pace in `phase` (a multiple of his walk) and his rest between moves (a multiple of his
/// archetype's): the Glutton, swollen, comes on quicker and strikes again sooner.
pub fn pace(phase: u8) -> (f64, f64) {
    if phase >= 3 {
        (1.3, 0.65)
    } else {
        (1.0, 1.0)
    }
}

/// The title of `phase`, for the banner and the boss bar.
pub fn phase_name(phase: u8) -> &'static str {
    match phase {
        0 | 1 => "THE LORD OF FLIES",
        2 => "THE SWARM",
        _ => "THE GLUTTON",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::combat::archetype::{Archetype, Breed, Power};
    use crate::logic::combat::frames::{Shape, BOLT_SPEED};
    use crate::logic::combat::prince;
    use crate::logic::enemy::EnemyKind;

    #[test]
    fn beelzebub_is_a_prince_with_a_table_for_every_phase_and_a_meal_in_each() {
        let mut b = Breed::of(EnemyKind::Beelzebub);
        assert_eq!((b.arch, b.power, b.cost), (Archetype::Prince, Power::Bloat, 3));
        assert!(Power::Bloat.is_hide(), "his bloat turns a blow as Mammon's gold does");
        let calm = b.stats();
        for phase in 1..=3 {
            b.phase = phase;
            let moves = b.moves();
            assert_eq!(moves, prince::table(EnemyKind::Beelzebub, phase));
            assert!(moves.len() <= 4, "a cooldown for each");
            assert!(moves.contains(&MoveId::Gorge), "he eats in every phase");
            assert!(moves.iter().all(|mv| MoveId::FOE.contains(mv)));
            let main = b.main_move().data();
            assert!(main.strikes() && !main.projectile, "he closes in to strike");
            assert_eq!(prince::phase_name(EnemyKind::Beelzebub, phase), phase_name(phase));
        }
        // The swarm: a fan of five; the cleaver a cone; the meal a spell no one chooses against him.
        assert!(matches!(MoveId::Swarm.data().shape, Shape::Volley { count: 5, .. }) && MoveId::Swarm.data().projectile);
        assert!(matches!(MoveId::Cleaver.data().shape, Shape::Cone));
        let gorge = MoveId::Gorge.data();
        assert!(!gorge.strikes() && gorge.startup >= 50 && matches!(gorge.shape, Shape::Spell));
        // The scatter falls on a mark; the roll is a charge; the bile lingers; the slam shakes.
        assert!(matches!(MoveId::Scatter.data().shape, Shape::Mark { .. }));
        assert!(
            SCATTER_SINK < MoveId::Scatter.data().track && SCATTER_GATHER < MoveId::Scatter.data().startup - MoveId::Scatter.data().track
        );
        assert!(matches!(MoveId::BellyRoll.data().shape, Shape::Charge { .. }));
        assert!(MoveId::BileSpew.data().traps().is_some_and(|(_, _, _, linger)| linger > 60));
        assert!(matches!(MoveId::BellySlam.data().shape, Shape::Quake { .. }));
        // The Glutton is quicker on his feet and to strike again, and still slower than the hero.
        b.phase = 3;
        let glutton = b.stats();
        assert!(glutton.walk > calm.walk && glutton.rest < calm.rest);
        assert!(glutton.walk < 6.5 * 0.8 * 0.6);
        // His flies are slow, but not so slow a bolt's pacing cannot meet them.
        const { assert!(FLY_PACE >= 0.5 && FLY_PACE < 1.0 && BOLT_SPEED * FLY_PACE > 3.0) };
        const { assert!(GORGE_HEAL > CROSS_HEAL && GORGE_BELOW < 1.0) };
    }
}
