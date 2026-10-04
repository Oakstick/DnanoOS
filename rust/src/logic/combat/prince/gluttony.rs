//! Beelzebub, the prince of Gluttony: the lord of the flies at his banquet.
//!
//! A placeholder until his fight is built (COMBAT.md §13): one move in every phase, Mammon's
//! pace, and names for his three phases.

use super::MoveId;

/// His moves in `phase`, in the order he prefers them (for now his one move in every phase).
pub fn table(_phase: u8) -> &'static [MoveId] {
    &[MoveId::Cleaver]
}

/// His pace in `phase` (a multiple of his walk) and his rest between moves (a multiple of his
/// archetype's): in his last phase he comes on quicker, and strikes again sooner.
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
        0 | 1 => "THE HOST",
        2 => "THE GLUTTON",
        _ => "THE LORD OF FLIES",
    }
}
