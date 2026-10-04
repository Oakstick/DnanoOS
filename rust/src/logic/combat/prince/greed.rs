//! Mammon, the prince of Greed (COMBAT.md §13): the Banker sweeps his sceptre and tosses coins
//! that lie on the floor and burst into gold spikes (`super::Trap`); the Collector raises two gold
//! golems from his heaps and grasps, holding him and draining his silver until he is struck; the
//! Hoard rains coins from above, flings them from afar, and brings his hoard down on the floor.

use super::MoveId;

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
