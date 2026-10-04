//! Amon, the prince of Wrath (PRINCES.md §3): the raven-headed marquis in a wolf's pelt, a
//! serpent's tail behind him, fire in his throat and a great iron axe. The Wolf charges, breathes
//! flame and cleaves; the Raven splits the floor into lava and calls his ravens down; the Fury
//! charges twice over and makes the floor round him erupt.
//!
//! Wrath has no hide. Every blow he takes makes him a little quicker (his rage, up to `RAGE_MAX`
//! steps: `enraged`); a charge that ends against stone leaves him dazed (`DAZE`), his rage spent.

use super::MoveId;
use crate::logic::combat::archetype::Stats;

/// His rage: the steps a blow adds up to, and what each step makes of his pace (a share more on his
/// feet) and of his rest between moves (a share less).
pub const RAGE_MAX: u8 = 5;
pub const RAGE_PACE: f64 = 0.06;
pub const RAGE_REST: f64 = 0.08;
/// Frames he stands dazed after his charge has run into stone (staggered: every blow lands whole,
/// and more).
pub const DAZE: u16 = 150;
/// The ravens he calls down as the second phase begins (once in a fight), and the frames they take
/// to come down from the dark.
pub const RAVENS: u8 = 3;
pub const RAVEN_DESCENT: u16 = 50;
/// Frames after a charge he may ask again for the second of the Fury's two.
pub const TWIN_REST: u16 = 8;

/// His moves in `phase`: the Wolf's charge, axe and breath; the Raven's rift besides; the Fury's
/// charge (twice over, `game::court::wrath`), his eruption and his breath.
pub fn table(phase: u8) -> &'static [MoveId] {
    match phase {
        0 | 1 => &[MoveId::FieryCharge, MoveId::AxeCleave, MoveId::FlameBreath],
        2 => &[MoveId::Rift, MoveId::FieryCharge, MoveId::FlameBreath],
        _ => &[MoveId::FieryCharge, MoveId::Eruption, MoveId::FlameBreath],
    }
}

/// His pace in `phase` (a multiple of his walk) and his rest between moves (a multiple of his
/// archetype's): the Fury comes on quicker, and strikes again sooner.
pub fn pace(phase: u8) -> (f64, f64) {
    if phase >= 3 {
        (1.25, 0.65)
    } else {
        (1.0, 1.0)
    }
}

/// The title of `phase`, for the banner and the boss bar.
pub fn phase_name(phase: u8) -> &'static str {
    match phase {
        0 | 1 => "THE WOLF",
        2 => "THE RAVEN",
        _ => "THE FURY",
    }
}

/// His numbers at `rage` steps of it: quicker on his feet, quicker to strike again (his startups
/// never shorten: every move he makes can still be read).
pub fn enraged(s: Stats, rage: u8) -> Stats {
    let r = rage.min(RAGE_MAX) as f64;
    let feet = 1.0 + RAGE_PACE * r;
    Stats {
        walk: s.walk * feet,
        strafe: s.strafe * feet,
        retreat: s.retreat * feet,
        rest: (s.rest as f64 * (1.0 - RAGE_REST * r)).round() as u16,
        ..s
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
    fn amon_fights_by_three_tables_and_every_move_keeps_the_princes_rules() {
        let mut a = Breed::of(EnemyKind::Amon);
        assert_eq!((a.arch, a.power, a.cost), (Archetype::Prince, Power::Fury, 3));
        assert!(!a.power.is_hide(), "Wrath has no hide");
        assert!(a.hp > Breed::of(EnemyKind::Mammon).hp, "a little more health for the open hide");
        let mut seen = Vec::new();
        for phase in 1..=3 {
            a.phase = phase;
            let moves = a.moves();
            assert_eq!(moves, table(phase));
            assert!(moves.len() <= 4);
            assert!(a.main_move().data().strikes() && !a.main_move().data().projectile, "he closes in to strike");
            for &mv in moves {
                let d = mv.data();
                assert!(MoveId::FOE.contains(&mv));
                // Readable, punishable.
                assert!(d.startup >= 30 && d.recovery >= 30, "{}", d.name);
                if !seen.contains(&mv) {
                    seen.push(mv);
                }
            }
        }
        assert_eq!(seen.len(), 5, "charge, cleave, breath, rift, eruption: {seen:?}");
        // The charge runs up to seven tiles in its active frames, its aim locked before it sets off.
        let c = MoveId::FieryCharge.data();
        let Shape::Charge { length, speed } = c.shape else { panic!("a charge") };
        assert!((length - 7.0).abs() < 1e-9);
        assert!(length / speed * 60.0 <= c.active as f64, "the run fits its active frames");
        assert!(c.startup - c.track >= 12 && c.parriable() && c.blockable());
        // The breath: a cone of fire longer than an arm (2.2 tiles with his arm, 35 degrees either side).
        let d = Duelist::bred(1, Breed::of(EnemyKind::Amon), V::new(0.0, 0.0), V::new(0.0, 1.0));
        let breath = d.data(MoveId::FlameBreath);
        assert!((breath.reach - 2.2).abs() < 1e-6 && (breath.half_angle.to_degrees() - 35.0).abs() < 1e-6);
        // The cleave: narrow and far.
        let cleave = d.data(MoveId::AxeCleave);
        assert!(cleave.reach > breath.reach && cleave.half_angle < breath.half_angle);
        // The rift: a row of lava toward him that lingers, and bursts late enough to walk off.
        let (count, radius, fuse, linger) = MoveId::Rift.data().traps().expect("a row of traps");
        assert!(matches!(MoveId::Rift.data().shape, Shape::TrapLine { .. }));
        assert!(count >= 4 && linger >= 120 && fuse > REACTION * 2 && radius < 1.0);
        // The eruption: the floor all round him.
        assert!(matches!(MoveId::Eruption.data().shape, Shape::Quake { .. }));
        // The Fury is quicker; still slower than the hero (6.5 x 0.8 = 5.2 t/s) at his angriest.
        a.phase = 1;
        let calm = a.stats();
        a.phase = 3;
        let fury = enraged(a.stats(), RAGE_MAX);
        assert!(fury.walk > calm.walk && fury.rest < calm.rest);
        assert!(fury.walk < 6.5 * 0.8 * 0.75, "{}", fury.walk);
    }

    #[test]
    fn rage_quickens_him_a_step_a_blow_up_to_five() {
        let s = Breed::of(EnemyKind::Amon).stats();
        let steps: Vec<f64> = (0..=7).map(|r| enraged(s, r).walk).collect();
        for r in 1..=RAGE_MAX as usize {
            assert!(steps[r] > steps[r - 1]);
        }
        assert_eq!(steps[5], steps[7], "five steps and no more");
        assert_eq!(enraged(s, 0), s);
        let mut d = Duelist::bred(1, Breed::of(EnemyKind::Amon), V::new(0.0, 0.0), V::new(0.0, 1.0));
        let calm = d.stats();
        d.lore.rage = 3;
        assert!(d.stats().walk > calm.walk && d.stats().rest < calm.rest, "his duelist's numbers carry it");
        assert_eq!(d.taken(), 1.0, "every blow lands whole");
    }
}
