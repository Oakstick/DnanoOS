//! The Burning Field: Wrath's realm (REALMS.md §3). For now a plain hall stands in for it (`super::placeholder`).

use super::{placeholder, Base};
use crate::logic::sins::Sin;

/// The layout's seed: the realm is the same every time.
pub const SEED: u32 = 7303;

#[derive(Clone, Debug)]
pub struct Battlefield {
    pub base: Base,
}

impl Battlefield {
    /// The realm laid out for `tier`.
    pub fn generate(tier: usize) -> Battlefield {
        Battlefield { base: placeholder(Sin::Wrath, tier, SEED) }
    }
}
