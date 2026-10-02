//! The Garden of Thorns: Lust's realm (REALMS.md §6). For now a plain hall stands in for it (`super::placeholder`).

use super::{placeholder, Base};
use crate::logic::sins::Sin;

/// The layout's seed: the realm is the same every time.
pub const SEED: u32 = 7606;

#[derive(Clone, Debug)]
pub struct Thorns {
    pub base: Base,
}

impl Thorns {
    /// The realm laid out for `tier`.
    pub fn generate(tier: usize) -> Thorns {
        Thorns { base: placeholder(Sin::Lust, tier, SEED) }
    }
}
