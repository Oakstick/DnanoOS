//! Leviathan, the prince of Envy, under the drowned bell (PRINCES.md §2).
//!
//! - **The Crooked Serpent** lashes his tail wide round him, spits brine in a fan of three, and
//!   dives: he sinks out of sight and comes up under the hero (a mark that follows him, then
//!   erupts), and lies there long in his recovery.
//! - **The Evil Eye** covets: he copies the archangel's power the hero last used, in green
//!   (`covet`): Michael's fire a volley of four, Gabriel's thunder three marks round him,
//!   Raphael's beam a beam, Uriel's wave the floor shaking round Leviathan (Raphael's, if the hero
//!   has used none). Two serpentlings rise from the water as it begins.
//! - **The Bell** coils round the bell (`COIL`), quicker: he tolls it, and the floor shakes in a
//!   ring round it; fire bursts at points round it (`fire_spots`). Coiled round the hot bell, his
//!   scales are burnt off on the side he turns to it (`burnt_side`): a blow from there lands whole.
//!
//! His scales are a hide as Mammon's gold (`prince::HIDE_SHARE`) outside his recoveries.

use super::MoveId;
use crate::logic::angels::Archangel;
use crate::logic::combat::math::{add, dot, from_angle, len, norm, scale, sub};
use crate::logic::types::V;
use std::f64::consts::TAU;

/// The serpentlings that rise from the water as his second phase begins (once in a fight), and
/// the frames they take to rise.
pub const SERPENTLINGS: u8 = 2;
pub const SERPENTLING_RISE: u16 = 50;
/// In his third phase he keeps this near the bell's middle (coiled round it), and swims to it
/// this fast (tiles a second) when he is farther.
pub const COIL: f64 = 3.3;
pub const SWIM: f64 = 7.0;
/// Coiled round the hot bell, a blow from no more than this far (radians) off the way from him to
/// the bell lands whole: his scales are burnt off on that side.
pub const BURNT: f64 = 100.0 * std::f64::consts::PI / 180.0;
/// Gabriel's thunder coveted: the two marks besides the first stand this far from it, behind the
/// hero either side (the way out is toward Leviathan, or straight back between them).
pub const THUNDER_SPREAD: f64 = 1.8;
/// The burning coil: this many fires in a ring this far round the bell's middle, and one on the
/// hero; each this wide.
pub const FIRES: usize = 6;
pub const FIRE_RING: f64 = 2.8;
pub const FIRE_RADIUS: f64 = 0.85;
/// Frames a fire's flames stand after it bursts (for the view).
pub const FLAMES: u16 = 30;

/// His moves in `phase`, in the order he prefers them: the Crooked Serpent's spit, lash and dive;
/// the Evil Eye's covet besides the spit and the lash; the Bell's toll, burning coil and lash.
pub fn table(phase: u8) -> &'static [MoveId] {
    match phase {
        0 | 1 => &[MoveId::BrineSpit, MoveId::CoilLash, MoveId::Dive],
        2 => &[MoveId::Covet, MoveId::BrineSpit, MoveId::CoilLash],
        _ => &[MoveId::BellToll, MoveId::BurningCoil, MoveId::CoilLash],
    }
}

/// His pace in `phase` (a multiple of his walk) and his rest between moves (a multiple of his
/// archetype's): coiled round the bell he moves quicker, and strikes again sooner.
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
        0 | 1 => "THE CROOKED SERPENT",
        2 => "THE EVIL EYE",
        _ => "THE BELL",
    }
}

/// What his covet becomes when the hero last used `angel`'s power (Raphael's if none).
pub fn covet(angel: Option<Archangel>) -> MoveId {
    match angel.unwrap_or(Archangel::Raphael) {
        Archangel::Michael => MoveId::CovetFire,
        Archangel::Gabriel => MoveId::CovetThunder,
        Archangel::Raphael => MoveId::CovetBeam,
        Archangel::Uriel => MoveId::CovetWave,
    }
}

/// The archangel whose power a coveted move copies.
pub fn coveted(mv: MoveId) -> Option<Archangel> {
    match mv {
        MoveId::CovetFire => Some(Archangel::Michael),
        MoveId::CovetThunder => Some(Archangel::Gabriel),
        MoveId::CovetBeam => Some(Archangel::Raphael),
        MoveId::CovetWave => Some(Archangel::Uriel),
        _ => None,
    }
}

/// Is a blow on him at `pos` from `from` on the side he turns to the bell at `bell`?
pub fn burnt_side(pos: V, from: V, bell: V) -> bool {
    let (to, toward) = (sub(from, pos), sub(bell, pos));
    if len(to) < 1e-9 || len(toward) < 1e-9 {
        return false;
    }
    dot(norm(to), norm(toward)) >= BURNT.cos()
}

/// Where Gabriel's coveted thunder strikes: the mark that followed the hero, and two more behind
/// him either side (as seen from Leviathan at `from`).
pub fn thunder_marks(mark: V, from: V) -> [V; 3] {
    let toward = sub(from, mark);
    let base = if len(toward) < 1e-9 { 0.0 } else { toward.y.atan2(toward.x) };
    let at = |a: f64| add(mark, scale(from_angle(base + a), THUNDER_SPREAD));
    [mark, at(TAU / 3.0), at(-TAU / 3.0)]
}

/// Where the burning coil's fires burst, the hero having stood at `hero` as it began: a ring of
/// `FIRES` round the bell, turned so that his bearing from it falls between two of them, and one
/// where he stood (unless one of the ring is already on him). (The game and the view both reckon
/// them from where he stood: `Lore::mark`.)
pub fn fire_spots(bell: V, hero: V) -> Vec<V> {
    let to = sub(hero, bell);
    let turn = if len(to) < 1e-9 { 0.0 } else { to.y.atan2(to.x) } + std::f64::consts::PI / FIRES as f64;
    let mut out: Vec<V> = (0..FIRES).map(|k| add(bell, scale(from_angle(turn + k as f64 * TAU / FIRES as f64), FIRE_RING))).collect();
    if out.iter().all(|p| len(sub(*p, hero)) > FIRE_RADIUS) {
        out.push(hero);
    }
    out
}

/// Where he coils in his third phase: as near `pos` as `COIL` from the bell lets him be.
pub fn coiled(pos: V, bell: V) -> V {
    let to = sub(pos, bell);
    if len(to) <= COIL {
        pos
    } else {
        add(bell, scale(norm(to), COIL))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::combat::archetype::{Archetype, Breed, Power};
    use crate::logic::combat::frames::{Shape, REACTION};
    use crate::logic::combat::fsm::GuardRule;
    use crate::logic::enemy::EnemyKind;
    use std::f64::consts::PI;

    #[test]
    fn leviathan_is_a_prince_with_a_table_for_every_phase_and_his_moves_keep_the_rules() {
        let mut b = Breed::of(EnemyKind::Leviathan);
        assert_eq!((b.arch, b.power, b.cost), (Archetype::Prince, Power::Scales, 3));
        assert!(b.power.is_hide(), "his scales are a hide");
        let calm = b.stats();
        let mut seen = Vec::new();
        for phase in 1..=3 {
            b.phase = phase;
            let moves = b.moves();
            assert_eq!(moves, table(phase));
            assert!(moves.len() <= 4, "a cooldown for each");
            assert!(moves.iter().all(|mv| MoveId::FOE.contains(mv)));
            assert_eq!(b.main_move(), MoveId::CoilLash, "he closes in to lash");
            seen.extend_from_slice(moves);
        }
        assert_eq!(phase_name(1), "THE CROOKED SERPENT");
        assert_eq!(phase_name(2), "THE EVIL EYE");
        assert_eq!(phase_name(3), "THE BELL");
        // The Bell is quicker on his feet and quicker to strike again, still slower than the hero.
        let bell = b.stats();
        assert!(bell.walk > calm.walk && bell.rest < calm.rest);
        assert!(bell.walk < 6.5 * 0.8 * 0.6);
        // Every move he has, the coveted four with them: readable, punishable, guarded as it should be.
        let all = [
            MoveId::CoilLash,
            MoveId::BrineSpit,
            MoveId::Dive,
            MoveId::Covet,
            MoveId::CovetFire,
            MoveId::CovetThunder,
            MoveId::CovetBeam,
            MoveId::CovetWave,
            MoveId::BellToll,
            MoveId::BurningCoil,
        ];
        for mv in all {
            let d = mv.data();
            assert!(MoveId::FOE.contains(&mv), "{mv:?}");
            assert!(d.startup >= 30, "{}: {} f is quicker than a prince may be", d.name, d.startup);
            assert!(d.recovery >= 30, "{}: {} f of recovery", d.name, d.recovery);
            assert!(d.strikes(), "{}", d.name);
            assert!(mv == MoveId::Covet || mv == MoveId::BurningCoil || d.shape != Shape::Spell, "{}", d.name);
        }
        for mv in [MoveId::Covet, MoveId::BrineSpit, MoveId::CoilLash, MoveId::Dive, MoveId::BellToll, MoveId::BurningCoil] {
            assert!(seen.contains(&mv), "{mv:?} in a table");
        }
        // The coveted four start up as the covet does: he becomes one of them on its first frame.
        for mv in [MoveId::CovetFire, MoveId::CovetThunder, MoveId::CovetBeam, MoveId::CovetWave] {
            assert_eq!(mv.data().startup, MoveId::Covet.data().startup, "{mv:?}");
        }
        // What the guard can do: the lash and the wave block and parry; the beam only blocks; the
        // dive, the thunder and the toll come from the ground.
        assert_eq!(MoveId::CoilLash.data().guard_rule(), GuardRule::Full);
        assert!(MoveId::CoilLash.data().half_angle * 2.0 >= 120f64.to_radians(), "a wide sweep");
        assert_eq!(MoveId::CovetWave.data().guard_rule(), GuardRule::Full);
        assert_eq!(MoveId::CovetBeam.data().guard_rule(), GuardRule::BlockOnly);
        for mv in [MoveId::Dive, MoveId::CovetThunder, MoveId::BellToll] {
            assert_eq!(mv.data().guard_rule(), GuardRule::Bare, "{mv:?}");
        }
        assert!(matches!(MoveId::BrineSpit.data().shape, Shape::Volley { count: 3, .. }) && MoveId::BrineSpit.data().projectile);
        assert!(matches!(MoveId::CovetFire.data().shape, Shape::Volley { count: 4, .. }) && MoveId::CovetFire.data().projectile);
        assert!(matches!(MoveId::CovetWave.data().shape, Shape::Quake { .. }));
        assert!(matches!(MoveId::CovetBeam.data().shape, Shape::Beam { .. }));
    }

    #[test]
    fn a_beam_and_the_fires_give_time_to_step_out_with_a_fifth_to_spare() {
        let walk = 6.5 * crate::logic::levels::SPEED_SCALE;
        // The beam's line locks as its tracking ends; then he steps out of its half-width.
        let d = MoveId::CovetBeam.data();
        let Shape::Beam { width, .. } = d.shape else { panic!() };
        let gained = (d.startup - d.track - REACTION) as f64 * walk / 60.0 - 0.25;
        assert!(gained >= (width / 2.0 + 0.3) * 1.2, "{gained:.2}");
        // The fires show where they will burst from the coil's first frame.
        let d = MoveId::BurningCoil.data();
        let gained = (d.startup - REACTION) as f64 * walk / 60.0 - 0.25;
        assert!(gained >= (FIRE_RADIUS + 0.3) * 1.2, "{gained:.2}");
        // The second and third thunder marks follow the first.
        let d = MoveId::CovetThunder.data();
        let Shape::Mark { radius } = d.shape else { panic!() };
        assert!(THUNDER_SPREAD > radius, "a way out between them");
    }

    #[test]
    fn he_covets_the_angel_last_called_and_raphael_if_none() {
        assert_eq!(covet(None), MoveId::CovetBeam);
        for (angel, mv) in [
            (Archangel::Michael, MoveId::CovetFire),
            (Archangel::Gabriel, MoveId::CovetThunder),
            (Archangel::Raphael, MoveId::CovetBeam),
            (Archangel::Uriel, MoveId::CovetWave),
        ] {
            assert_eq!(covet(Some(angel)), mv);
            assert_eq!(coveted(mv), Some(angel));
        }
        assert_eq!(coveted(MoveId::CoilLash), None);
    }

    #[test]
    fn coiled_round_the_bell_his_scales_are_burnt_on_its_side() {
        let bell = V::new(16.0, 16.0);
        let pos = V::new(16.0, 13.0);
        // From the bell's side, or across his flank toward it: whole.
        assert!(burnt_side(pos, V::new(16.0, 14.5), bell));
        assert!(burnt_side(pos, V::new(17.5, 13.6), bell));
        assert!(burnt_side(pos, V::new(14.5, 13.0), bell), "at his flank, a little past square");
        // From the side away from it: his scales.
        assert!(!burnt_side(pos, V::new(16.0, 11.5), bell));
        assert!(!burnt_side(pos, V::new(15.0, 11.8), bell));
        let edge = add(pos, from_angle(PI / 2.0 + BURNT - 0.05));
        assert!(burnt_side(pos, edge, bell));
        // He coils within reach of it.
        assert_eq!(coiled(V::new(16.0, 14.0), bell), V::new(16.0, 14.0));
        let far = coiled(V::new(16.0, 4.0), bell);
        assert!((len(sub(far, bell)) - COIL).abs() < 1e-9 && far.y < bell.y);
    }

    #[test]
    fn the_fires_ring_the_bell_and_one_finds_the_hero_and_the_thunder_falls_round_him() {
        let bell = V::new(16.0, 16.0);
        let spots = fire_spots(bell, V::new(16.0, 24.0));
        assert_eq!(spots.len(), FIRES + 1);
        assert!(spots[..FIRES].iter().all(|p| (len(sub(*p, bell)) - FIRE_RING).abs() < 1e-9));
        assert_eq!(spots[FIRES], V::new(16.0, 24.0));
        // Standing on the ring as it begins, he is between two of its fires, and one more is on him.
        let on = add(bell, scale(from_angle(0.4), FIRE_RING));
        let spots = fire_spots(bell, on);
        assert_eq!(spots.len(), FIRES + 1);
        assert!(spots[..FIRES].iter().all(|p| len(sub(*p, on)) > FIRE_RADIUS * 1.5));
        // Three thunder marks: the first where it followed him, two behind him from Leviathan.
        let m = thunder_marks(V::new(10.0, 10.0), V::new(10.0, 4.0));
        assert_eq!(m[0], V::new(10.0, 10.0));
        for p in &m[1..] {
            assert!((len(sub(*p, m[0])) - THUNDER_SPREAD).abs() < 1e-9);
            assert!(p.y > 10.0, "behind him: {p:?}");
        }
    }
}
