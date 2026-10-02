//! The Grey Mire: Sloth's realm (REALMS.md §4). For now a plain hall stands in for it (`super::placeholder`).

use super::{placeholder, Base};
use crate::logic::sins::Sin;

/// The layout's seed: the realm is the same every time.
pub const SEED: u32 = 7404;

#[derive(Clone, Debug)]
pub struct Mire {
    pub base: Base,
}

impl Mire {
    /// The realm laid out for `tier`.
    pub fn generate(tier: usize) -> Mire {
        Mire { base: placeholder(Sin::Sloth, tier, SEED) }
    }
}
