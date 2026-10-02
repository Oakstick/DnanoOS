//! The Hall of Mirrors: Pride's realm (REALMS.md §1). For now a plain hall stands in for it (`super::placeholder`).

use super::{placeholder, Base};
use crate::logic::sins::Sin;

/// The layout's seed: the realm is the same every time.
pub const SEED: u32 = 7101;

#[derive(Clone, Debug)]
pub struct Mirrors {
    pub base: Base,
}

impl Mirrors {
    /// The realm laid out for `tier`.
    pub fn generate(tier: usize) -> Mirrors {
        Mirrors { base: placeholder(Sin::Pride, tier, SEED) }
    }
}
