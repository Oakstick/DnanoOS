//! The colour-blind palettes (UI.md §2.6, §4): the colours that tell danger from safety, swapped
//! for each kind of colour blindness for ones it tells apart. They reach the fight's tells (a blade
//! glinting red for a blow no shield holds), the minimap's foes and the places it marks green, and
//! the health bars. With the setting off, every colour is the game's own.
//!
//! Red and green read alike in deuteranopia and protanopia (protanopia darkens red as well): there
//! danger turns to a bright orange and safety to blue. In tritanopia red and green are safe but
//! blue reads like green and yellow like pink: danger stays red, leaning crimson, safety turns teal,
//! and the middle of the health bar white.

use crate::render::minimap::{FoeRank, Marker, MarkerKind};
use crate::settings::ColourBlind;

/// The game's own danger red (a peril's glint, a foe's dot) and safety green.
pub const DANGER: u32 = 0xff2a1a;
pub const SAFE: u32 = 0x5cff6a;

/// A blow no shield holds: its weapon's glint.
pub fn peril(mode: ColourBlind) -> u32 {
    match mode {
        ColourBlind::Off => DANGER,
        ColourBlind::Deuteranopia => 0xff8c00,
        ColourBlind::Protanopia => 0xffb000,
        ColourBlind::Tritanopia => 0xff1f5a,
    }
}

/// What is safe, open or good (a gate open, a remedy, a cage opened).
pub fn safe(mode: ColourBlind) -> u32 {
    match mode {
        ColourBlind::Off => SAFE,
        ColourBlind::Deuteranopia | ColourBlind::Protanopia => 0x3d9bff,
        ColourBlind::Tritanopia => 0x2fd0c8,
    }
}

/// A plain foe's dot on the map.
pub fn foe(mode: ColourBlind) -> u32 {
    match mode {
        ColourBlind::Off => 0xe8382a,
        _ => peril(mode),
    }
}

/// The health bar's colour at `frac` of full: high, middling, low.
pub fn health(mode: ColourBlind, frac: f32) -> u32 {
    let (high, mid, low) = match mode {
        ColourBlind::Off => (SAFE, 0xeab55e, 0xd8262e),
        ColourBlind::Deuteranopia => (0x3d9bff, 0xe8e2d4, 0xff8c00),
        ColourBlind::Protanopia => (0x3d9bff, 0xe8e2d4, 0xffb000),
        ColourBlind::Tritanopia => (0x2fd0c8, 0xf2f2f2, 0xff1f5a),
    };
    if frac > 0.5 {
        high
    } else if frac > 0.25 {
        mid
    } else {
        low
    }
}

/// A map marker's colour under the palette: the foes' red dots and the green of what is open or
/// healing swapped; everything else as it is.
pub fn marker(m: &Marker, mode: ColourBlind) -> u32 {
    if mode == ColourBlind::Off {
        return m.own_color();
    }
    match m.kind {
        MarkerKind::Foe(FoeRank::Plain) => foe(mode),
        MarkerKind::RedKey => peril(mode),
        MarkerKind::Remedy | MarkerKind::Gate { open: true } | MarkerKind::Cage { open: true } => safe(mode),
        _ => m.own_color(),
    }
}

/// How far two colours are apart (each channel 0 to 255, summed), for the tests.
#[cfg(test)]
fn apart(a: u32, b: u32) -> i32 {
    (0..3).map(|k| ((a >> (8 * k)) as i32 & 255) - ((b >> (8 * k)) as i32 & 255)).map(i32::abs).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::types::V;

    /// A colour as a deuteranope sees it (Machado, Oliveira and Fernandes 2009, severity 1, on
    /// linear light), roughly: enough to tell whether two colours stay apart.
    fn as_deuteranope(c: u32) -> [f32; 3] {
        let lin = |v: u32| ((v & 255) as f32 / 255.0).powf(2.2);
        let (r, g, b) = (lin(c >> 16), lin(c >> 8), lin(c));
        [0.367 * r + 0.861 * g - 0.228 * b, 0.280 * r + 0.673 * g + 0.047 * b, -0.012 * r + 0.043 * g + 0.969 * b]
    }

    #[test]
    fn colour_blind_swaps_the_tells_and_the_markers() {
        for mode in ColourBlind::ALL {
            let swapped = mode != ColourBlind::Off;
            assert_eq!(peril(mode) != DANGER, swapped, "{mode:?}");
            assert_eq!(safe(mode) != SAFE, swapped, "{mode:?}");
            // The two ends stay far apart, and both stand off from the plain white glint.
            assert!(apart(peril(mode), safe(mode)) > 200, "{mode:?}");
            assert!(apart(peril(mode), 0xfff2c8) > 150, "{mode:?}");
        }
        let foe_dot = Marker { kind: MarkerKind::Foe(FoeRank::Plain), at: V::new(0.0, 0.0), known: true };
        let key = Marker { kind: MarkerKind::Key, at: V::new(0.0, 0.0), known: true };
        assert_eq!(marker(&foe_dot, ColourBlind::Off), foe_dot.own_color());
        assert_ne!(marker(&foe_dot, ColourBlind::Deuteranopia), foe_dot.own_color());
        assert_eq!(marker(&key, ColourBlind::Protanopia), key.own_color(), "what is not red or green keeps its colour");
        // The health bar's high and low ends, which a deuteranope sees as near alike in the game's
        // own colours, are told apart under the palette.
        let seen = |c: u32| as_deuteranope(c);
        let gap = |a: [f32; 3], b: [f32; 3]| (0..3).map(|k| (a[k] - b[k]).abs()).sum::<f32>();
        let own = gap(seen(health(ColourBlind::Off, 1.0)), seen(health(ColourBlind::Off, 0.1)));
        let swapped = gap(seen(health(ColourBlind::Deuteranopia, 1.0)), seen(health(ColourBlind::Deuteranopia, 0.1)));
        assert!(swapped > own, "{swapped} > {own}");
    }
}
