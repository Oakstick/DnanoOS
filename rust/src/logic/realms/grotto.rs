//! The Serpent Grotto: Envy's realm (REALMS.md §2). For now a plain hall stands in for it (`super::placeholder`).

use super::{placeholder, Base};
use crate::logic::sins::Sin;

/// The layout's seed: the realm is the same every time.
pub const SEED: u32 = 7202;

#[derive(Clone, Debug)]
pub struct Grotto {
    pub base: Base,
}

impl Grotto {
    /// The realm laid out for `tier`.
    pub fn generate(tier: usize) -> Grotto {
        Grotto { base: placeholder(Sin::Envy, tier, SEED) }
    }
}
