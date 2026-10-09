//! Belphegor, the prince of Sloth (PRINCES.md §4): the dozing lord among his stopped clocks.
//!
//! He sleeps in his great chair as the fight begins (his phase 0, ASLEEP): his hide is open, and
//! the first blow lands whole and wakes him. The Sleeper yawns the hero back, swats with his great
//! hand and lets the sand of hours drift onto the floor to burst. At two thirds he lies down to
//! sleep again (the Dreamer): asleep his hide is open, he mends slowly, and two nightmares come
//! out of his dream to fight for him; struck hard enough (a share of his health, or a finisher)
//! he wakes with a roar and fights, and in a while he nods off again. At one third he stands up
//! for good (the Waking): slow, but his reach is vast: the hourglass brought down on the floor,
//! the clock's hands swept all the way round him.
//!
//! What his sleep keeps is `Slumber` (on his `Lore`); the game moves it on
//! (`game::court::sloth`): it puts him to sleep, mends him, and wakes him.

use super::MoveId;

/// The share of his whole health that blows on him must take off in one sleep to wake him (a
/// finisher wakes him at once).
pub const WAKE_SHARE: f64 = 0.06;
/// Frames he takes to rouse himself, open, before he fights; and to lie down to sleep.
pub const WAKE_FRAMES: u16 = 54;
pub const NOD_FRAMES: u16 = 40;
/// The share of his whole health he mends each second he sleeps in his second phase (never past
/// the line where it began; WARBAND.md W10: halved from 0.008, which a hero with a light blade could
/// not outstrike).
pub const MEND: f64 = 0.004;
/// Frames awake in his second phase before he nods off again.
pub const DOZE_AGAIN: u32 = 540;
/// The nightmares that come out of his dream as his second phase begins (once in a fight), and
/// the frames they take to gather out of the fog.
pub const NIGHTMARES: u8 = 2;
pub const NIGHTMARE_RISE: u16 = 40;

/// Where he is in his sleep.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Doze {
    /// Awake and fighting.
    #[default]
    Awake,
    /// His eyes closing: he finishes what he is doing and lies down.
    Drowsy,
    /// Lying down to sleep, frames left.
    Nodding(u16),
    /// Fast asleep.
    Asleep,
    /// Rousing himself, frames left.
    Waking(u16),
}

/// His sleep, as his fight keeps it (on his `Lore`, so a fall resets it with the rest).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Slumber {
    pub doze: Doze,
    /// Frames since he fell asleep (his breathing, for the view).
    pub slept: u32,
    /// What blows have taken off him in this sleep (his waking's measure).
    pub struck: f64,
    /// His health as last seen (to see a blow land, or his mending).
    pub last: f64,
    /// Frames awake since he last woke.
    pub awake: u32,
    /// Asleep in his great chair (the fight's first sleep), not on the floor.
    pub chair: bool,
    /// The combat frame he was last seen on (his sleep moves on a frame at a time; 0: not yet).
    pub seen: u32,
}

impl Slumber {
    /// Asleep in his chair as the fight begins.
    pub fn in_chair(hp: f64) -> Slumber {
        Slumber { doze: Doze::Asleep, last: hp, chair: true, ..Slumber::default() }
    }

    /// Lying down, asleep or waking: his hide is open, and he does nothing of his own.
    pub fn open(&self) -> bool {
        matches!(self.doze, Doze::Nodding(_) | Doze::Asleep | Doze::Waking(_))
    }

    /// How deep asleep he is, for the view: 0 awake, 1 fast asleep (rising as he lies down,
    /// falling as he wakes).
    pub fn depth(&self) -> f32 {
        match self.doze {
            Doze::Awake | Doze::Drowsy => 0.0,
            Doze::Nodding(left) => 1.0 - left as f32 / NOD_FRAMES as f32,
            Doze::Asleep => 1.0,
            Doze::Waking(left) => left as f32 / WAKE_FRAMES as f32,
        }
    }

    /// A blow that took `taken` off him while he sleeps, as a finisher or not: does it wake him?
    pub fn wakes(&self, taken: f64, max: f64, finisher: bool) -> bool {
        matches!(self.doze, Doze::Asleep | Doze::Nodding(_)) && (finisher || self.struck + taken >= max * WAKE_SHARE)
    }
}

/// His moves in `phase`, in the order he prefers them: asleep, none of them (the Sleeper's, never
/// used); the Sleeper yawns, swats and lets the sand drift; the Dreamer, woken, the same; the
/// Waking brings down the hourglass and sweeps the clock's hands round him, and yawns and swats.
pub fn table(phase: u8) -> &'static [MoveId] {
    match phase {
        0..=2 => &[MoveId::Yawn, MoveId::Swat, MoveId::SandOfHours],
        _ => &[MoveId::HourglassSlam, MoveId::ClockHands, MoveId::Yawn, MoveId::Swat],
    }
}

/// His pace in `phase` (a multiple of his walk) and his rest between moves (a multiple of his
/// archetype's): slothful always, and slowest when he stands at last (his reach makes up for it).
pub fn pace(phase: u8) -> (f64, f64) {
    if phase >= 3 {
        (0.75, 0.9)
    } else {
        (0.85, 1.1)
    }
}

/// The title of `phase`, for the banner and the boss bar.
pub fn phase_name(phase: u8) -> &'static str {
    match phase {
        0 => "ASLEEP",
        1 => "THE SLEEPER",
        2 => "THE DREAMER",
        _ => "THE WAKING",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::combat::archetype::{Archetype, Breed, Power};
    use crate::logic::combat::frames::{Rider, Shape};
    use crate::logic::enemy::EnemyKind;

    #[test]
    fn belphegor_is_a_prince_whose_tables_yawn_swat_drift_sand_then_slam_and_sweep_from_far() {
        let mut b = Breed::of(EnemyKind::Belphegor);
        assert_eq!((b.arch, b.power, b.cost), (Archetype::Prince, Power::Torpor, 3));
        let calm = b.stats();
        for phase in 0..=3 {
            b.phase = phase;
            let moves = b.moves();
            assert_eq!(moves, table(phase));
            assert!(!moves.is_empty() && moves.len() <= 4, "a cooldown for each");
            assert!(moves.iter().all(|mv| MoveId::FOE.contains(mv)));
            assert!(b.main_move().data().strikes() && !b.main_move().data().projectile, "he closes in to strike");
        }
        assert!(matches!(MoveId::Yawn.data().rider, Rider::Shove(_)), "the yawn pushes him back");
        assert!(MoveId::Yawn.data().damage < MoveId::Swat.data().damage * 0.5, "and hurts little");
        assert!(matches!(MoveId::SandOfHours.data().shape, Shape::Traps { .. }));
        assert!(matches!(MoveId::HourglassSlam.data().shape, Shape::Quake { .. }));
        // The clock's hands: the widest, farthest blow of his.
        let hands = MoveId::ClockHands.data();
        assert!(hands.half_angle > MoveId::Swat.data().half_angle && hands.reach > 2.5);
        // The Waking is slower on his feet than the Sleeper, and every phase slower than Mammon.
        b.phase = 3;
        assert!(b.stats().walk < calm.walk);
        assert!(calm.walk < Breed::of(EnemyKind::Mammon).stats().walk);
        assert_eq!(phase_name(0), "ASLEEP");
    }

    #[test]
    fn asleep_his_hide_is_open_and_a_share_of_his_health_or_a_finisher_wakes_him() {
        let mut s = Slumber::in_chair(900.0);
        assert!(s.open() && s.depth() == 1.0 && s.chair);
        assert!(!s.wakes(10.0, 900.0, false), "a tap does not");
        assert!(s.wakes(10.0, 900.0, true), "a finisher does");
        s.struck = 900.0 * WAKE_SHARE - 5.0;
        assert!(s.wakes(10.0, 900.0, false), "blows enough do");
        s.doze = Doze::Waking(WAKE_FRAMES / 2);
        assert!(s.open() && (s.depth() - 0.5).abs() < 0.05, "open as he wakes");
        assert!(!s.wakes(100.0, 900.0, true), "already waking");
        s.doze = Doze::Drowsy;
        assert!(!s.open() && s.depth() == 0.0, "still fighting while his eyes close");
        assert!(!Slumber::default().open());
    }
}
