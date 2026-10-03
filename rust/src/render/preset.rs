//! Quality presets (RP16 and the preset policy in the Technical Art and Rendering Plan). The two
//! target machines get different presets, not one scaled by a factor: the RTX 3060 at 60 FPS and
//! the GTX 1080 at 30 FPS, both at native 1080p. The policy is to give up distant shadow quality,
//! reflection work and volumetric detail before anything that keeps the hero readable or a
//! material recognisable, and to keep the fog's look apart from the volumetric lighting's quality
//! (the ground haze stays on every preset; only the lamps' glow in it is scaled).

/// The two shadow cascades, each with its own map size, and the soft-shadow filter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadows {
    /// Texels a side of the fine cascade round the hero.
    pub near_size: u32,
    /// Texels a side of the coarse cascade over the whole level.
    pub far_size: u32,
    /// Taps looking for what lies between a point and the light (how soft to go).
    pub search_taps: u32,
    /// Taps of the filter itself.
    pub filter_taps: u32,
}

/// What a preset may spend, in milliseconds of GPU time at the 95th percentile on its target GPU.
/// The cascades are budgeted apart (RP16): the fine one redraws its casters every frame, the
/// coarse one copies its static part in and draws only what moves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Budget {
    pub frame_ms: f64,
    pub shadow_near_ms: f64,
    pub shadow_far_ms: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Preset {
    pub name: &'static str,
    /// The machine it is meant for.
    pub target: &'static str,
    pub shadows: Shadows,
    /// Lamps lighting the haze (RP19), first in priority order.
    pub volumetric_lamps: usize,
    /// Rig level-of-detail bias (RP12): above 1 drops to coarser levels sooner.
    pub lod_bias: f32,
    pub budget: Budget,
}

/// The RTX 3060 at 60 FPS: the full look.
pub const HIGH: Preset = Preset {
    name: "high",
    target: "RTX 3060, 60 FPS",
    shadows: Shadows { near_size: 4096, far_size: 4096, search_taps: 8, filter_taps: 16 },
    volumetric_lamps: 8,
    lod_bias: 1.0,
    budget: Budget { frame_ms: 1000.0 / 60.0, shadow_near_ms: 1.0, shadow_far_ms: 1.0 },
};

/// The GTX 1080 at 30 FPS, the reduced preset. The fine cascade keeps a 1 cm texel round the hero;
/// the level-wide one halves, and the filter takes half the taps.
pub const LOW: Preset = Preset {
    name: "low",
    target: "GTX 1080, 30 FPS",
    shadows: Shadows { near_size: 2048, far_size: 2048, search_taps: 4, filter_taps: 8 },
    volumetric_lamps: 4,
    // Not raised: at 1.25 Ragnar drops to his half level at play distance, which costs readability.
    lod_bias: 1.0,
    budget: Budget { frame_ms: 1000.0 / 30.0, shadow_near_ms: 2.0, shadow_far_ms: 1.5 },
};

/// The browser build (no longer a target, kept compiling): WebGL2's smaller maps.
pub const WEB: Preset = Preset {
    name: "web",
    target: "browser",
    shadows: Shadows { near_size: 2048, far_size: 2048, search_taps: 8, filter_taps: 16 },
    volumetric_lamps: 8,
    lod_bias: 1.0,
    budget: HIGH.budget,
};

pub const ALL: [Preset; 2] = [HIGH, LOW];

impl Preset {
    /// A preset by name ("high", "low").
    pub fn named(name: &str) -> Option<Preset> {
        ALL.into_iter().find(|p| p.name.eq_ignore_ascii_case(name))
    }

    /// The preset a build starts with.
    pub fn default_for_build() -> Preset {
        if cfg!(target_arch = "wasm32") {
            WEB
        } else {
            HIGH
        }
    }
}

impl Default for Preset {
    fn default() -> Self {
        Preset::default_for_build()
    }
}

// The reduced preset never draws the level wider than the full one, and the haze keeps some lamps.
const _: () = assert!(LOW.shadows.far_size <= HIGH.shadows.far_size && LOW.volumetric_lamps > 0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_found_by_name_and_the_low_one_gives_up_distance_first() {
        assert_eq!(Preset::named("LOW"), Some(LOW));
        assert_eq!(Preset::named("high"), Some(HIGH));
        assert_eq!(Preset::named("ultra"), None);
        // The fine cascade round the hero keeps at least the far one's resolution.
        for p in ALL {
            assert!(p.shadows.near_size >= p.shadows.far_size, "{}", p.name);
            assert!(p.shadows.filter_taps >= p.shadows.search_taps, "{}", p.name);
            assert!(p.budget.shadow_near_ms + p.budget.shadow_far_ms < p.budget.frame_ms * 0.2, "{}", p.name);
        }
    }
}
