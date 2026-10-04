//! What the Warband keeps for the run (WARBAND.md §14): one settings key, `vh.warband`, written
//! beside the run's checkpoint (`vh.run`), wiped by Start, kept by Continue.
//!
//! The line is sectioned, so each phase can add its own without breaking the others:
//!
//! ```text
//! v1 ledger:<the Ledger>|generals:<...>|lies:<N>|counsel:<...>
//! ```
//!
//! A section this build does not know is kept as it was and written back, so an older build
//! never loses what a newer one saved.

use super::ledger::Ledger;

/// The run's Warband, as saved.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct WarSave {
    /// The Ledger of Grudges.
    pub ledger: Ledger,
    /// Each realm's general (W6): instruments broken, decrees done, slain. Kept as written.
    pub generals: String,
    /// The prince's lies seen through this run (W7).
    pub lies: u32,
    /// The old man's once-flags (W7). Kept as written.
    pub counsel: String,
    /// Sections this build does not know, in the order they came.
    pub other: Vec<(String, String)>,
}

impl WarSave {
    /// One line for the settings file (no newline, and no '|' inside a section).
    pub fn encode(&self) -> String {
        let mut sections = vec![
            format!("ledger:{}", self.ledger.encode()),
            format!("generals:{}", self.generals),
            format!("lies:{}", self.lies),
            format!("counsel:{}", self.counsel),
        ];
        sections.extend(self.other.iter().map(|(k, v)| format!("{k}:{v}")));
        format!("v1 {}", sections.join("|"))
    }

    /// Reads a line back; `None` for anything that is not a v1 line (a wiped save is empty).
    /// A section it cannot read is left at its default rather than losing the rest.
    pub fn decode(text: &str) -> Option<WarSave> {
        let body = text.trim().strip_prefix("v1 ")?;
        let mut out = WarSave::default();
        for section in body.split('|').filter(|s| !s.is_empty()) {
            let (key, value) = section.split_once(':').unwrap_or((section, ""));
            match key {
                "ledger" => out.ledger = Ledger::decode(value).unwrap_or_default(),
                "generals" => out.generals = value.to_string(),
                "lies" => out.lies = value.parse().unwrap_or(0),
                "counsel" => out.counsel = value.to_string(),
                _ => out.other.push((key.to_string(), value.to_string())),
            }
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::enemy::EnemyKind;
    use crate::logic::heroes::HeroKind;
    use crate::logic::war::names::Deed;

    #[test]
    fn the_warband_survives_the_save_file() {
        let mut s = WarSave::default();
        let id = s.ledger.promote(EnemyKind::Albasti, Deed::TookSilver, HeroKind::Mehmed, 42);
        s.ledger.get_mut(id).unwrap().name = "Sarı Gülsüm".to_string();
        s.ledger.promote(EnemyKind::Tepegoz, Deed::KillingBlow, HeroKind::Mehmed, 43);
        s.lies = 3;
        s.counsel = "first-death,met-rival".to_string();
        s.generals = "g4:110".to_string();
        let text = s.encode();
        assert!(text.starts_with("v1 ledger:") && !text.contains('\n'));
        assert_eq!(WarSave::decode(&text), Some(s.clone()));
        // Sections from a later build are kept and written back.
        let later = format!("{text}|mastery:r3,m2|drum:steady");
        let back = WarSave::decode(&later).unwrap();
        assert_eq!(back.other, vec![("mastery".to_string(), "r3,m2".to_string()), ("drum".to_string(), "steady".to_string())]);
        assert_eq!(back.encode(), later);
        assert_eq!(back.ledger, s.ledger);
        // A wiped save, or rubbish, holds nothing; a broken section loses only itself.
        assert_eq!(WarSave::decode(""), None);
        assert_eq!(WarSave::decode("v2 ledger:"), None);
        let broken = WarSave::decode("v1 ledger:x|lies:5").unwrap();
        assert_eq!((broken.ledger, broken.lies), (Ledger::default(), 5));
    }
}
