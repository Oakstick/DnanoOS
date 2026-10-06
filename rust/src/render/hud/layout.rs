//! Where play's HUD sits (UI.md §5): every corner's rectangle inside the safe area, in the unit.
//!
//! ```text
//! [health: portrait, name, lives, health, stamina]   [boss / prince / Pit / general]   [minimap]
//! [effects]                                                                    [objective, purse]
//!
//!                              [the old man's dialogue]
//! [archangel]                  [loadout and belt]                                    [location]
//! ```
//!
//! The minimap's square is the map builder's to draw in (`minimap`); everything else is drawn by
//! `play`, `angel` and `dialogue` in the rectangles given here.

use super::style;

/// The rectangles of play's HUD, `[x, y, w, h]` in logical pixels, and the unit they are laid in.
#[derive(Clone, Copy, Debug)]
pub struct HudLayout {
    /// The unit (the player's UI scale in it, held down if the screen is too narrow for it).
    pub t: f32,
    /// The safe area's margin.
    pub safe: f32,
    /// Top left: the hero's portrait, name, lives, health and stamina.
    pub health: [f32; 4],
    /// Under it: his running effects, each an icon with its time.
    pub effects: [f32; 4],
    /// Top right: the minimap's square (UI.md §7).
    pub minimap: [f32; 4],
    /// Under the minimap: the objective, the purse and the realm's meters (as much as they need).
    pub purse: [f32; 4],
    /// Top centre: the boss's, a prince's, the Pit's or a general's banner.
    pub top: [f32; 4],
    /// Bottom centre: the weapon, the shield and the belt, with the chosen potion's name over it.
    pub loadout: [f32; 4],
    /// Bottom left: the archangel's emblem and name.
    pub angel: [f32; 4],
    /// Bottom right: where he is, and the guide's control.
    pub location: [f32; 4],
    /// Above the loadout: the old man's dialogue (or a prince's whisper).
    pub dialogue: [f32; 4],
}

/// Sizes, in units.
pub const HEALTH_W: f32 = 15.4;
pub const HEALTH_H: f32 = 3.9;
pub const EFFECTS_H: f32 = 1.9;
pub const MINIMAP: f32 = 7.0;
pub const PURSE_W: f32 = 10.0;
pub const PURSE_H: f32 = 8.0;
pub const TOP_W: f32 = 21.0;
pub const TOP_H: f32 = 4.2;
pub const LOADOUT_W: f32 = 27.0;
pub const LOADOUT_H: f32 = 3.9;
pub const ANGEL_W: f32 = 10.5;
pub const ANGEL_H: f32 = 4.7;
pub const LOCATION_W: f32 = 9.0;
pub const LOCATION_H: f32 = 1.5;
pub const DIALOGUE_W: f32 = 34.0;
pub const DIALOGUE_H: f32 = 8.0;
/// The least room between two parts.
const GAP: f32 = 0.4;
/// The bottom row's width in units with its gaps: what the unit is held to on a narrow screen.
const BOTTOM_ROW: f32 = ANGEL_W + LOADOUT_W + LOCATION_W + 4.0 * GAP;

impl HudLayout {
    /// The layout for a `w` by `h` screen at the player's UI scale.
    pub fn of(w: f32, h: f32) -> Self {
        Self::at(w, h, style::unit(w, h))
    }

    /// The layout for a `w` by `h` screen with the unit `t` (held down to what fits).
    pub fn at(w: f32, h: f32, t: f32) -> Self {
        let safe = style::safe(w, h);
        let (iw, ih) = (w - 2.0 * safe, h - 2.0 * safe);
        // (A large UI scale on a narrow or short screen is held to what fits.)
        let t = t.min(iw / BOTTOM_ROW).min(ih / (MINIMAP + PURSE_H + GAP * 3.0 + DIALOGUE_H + LOADOUT_H)).max(1.0);
        let (l, r, top, bottom) = (safe, w - safe, safe, h - safe);
        let health = [l, top, HEALTH_W * t, HEALTH_H * t];
        let effects = [l, top + (HEALTH_H + GAP) * t, HEALTH_W * t, EFFECTS_H * t];
        let minimap = [r - MINIMAP * t, top, MINIMAP * t, MINIMAP * t];
        let purse = [r - PURSE_W * t, top + (MINIMAP + GAP) * t, PURSE_W * t, PURSE_H * t];
        // The top centre: as wide as the corners beside it leave room for.
        let side = (HEALTH_W + GAP).max(PURSE_W + GAP) * t;
        let tw = (TOP_W * t).min(w - 2.0 * (safe + side));
        let top_c = [w / 2.0 - tw / 2.0, top, tw, TOP_H * t];
        let loadout = [w / 2.0 - LOADOUT_W * t / 2.0, bottom - LOADOUT_H * t, LOADOUT_W * t, LOADOUT_H * t];
        let angel = [l, bottom - ANGEL_H * t, ANGEL_W * t, ANGEL_H * t];
        let location = [r - LOCATION_W * t, bottom - LOCATION_H * t, LOCATION_W * t, LOCATION_H * t];
        // (Clear of the archangel and the location beside the loadout below it.)
        let dw = (DIALOGUE_W * t).min(w - 2.0 * (safe + (ANGEL_W.max(LOCATION_W) + GAP) * t));
        let dialogue = [w / 2.0 - dw / 2.0, loadout[1] - (GAP + DIALOGUE_H) * t, dw, DIALOGUE_H * t];
        Self { t, safe, health, effects, minimap, purse, top: top_c, loadout, angel, location, dialogue }
    }

    /// Every part by name, for the tests and the debug outline.
    pub fn parts(&self) -> [(&'static str, [f32; 4]); 9] {
        [
            ("health", self.health),
            ("effects", self.effects),
            ("minimap", self.minimap),
            ("purse", self.purse),
            ("top", self.top),
            ("loadout", self.loadout),
            ("angel", self.angel),
            ("location", self.location),
            ("dialogue", self.dialogue),
        ]
    }
}

/// The minimap's square for a `w` by `h` screen (UI.md §7: top right, 7 units across, inside the
/// safe area), for the map builder.
pub fn minimap_rect(w: f32, h: f32) -> [f32; 4] {
    HudLayout::of(w, h).minimap
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apart(a: [f32; 4], b: [f32; 4]) -> bool {
        a[0] + a[2] <= b[0] || b[0] + b[2] <= a[0] || a[1] + a[3] <= b[1] || b[1] + b[3] <= a[1]
    }

    #[test]
    fn every_part_keeps_inside_the_safe_area_and_apart_at_every_shape_of_screen() {
        let screens = [
            ("16:9", 1280.0, 720.0),
            ("16:9", 1920.0, 1080.0),
            ("16:9", 3840.0, 2160.0),
            ("16:10", 1280.0, 800.0),
            ("16:10", 1920.0, 1200.0),
            ("21:9", 2560.0, 1080.0),
            ("21:9", 3440.0, 1440.0),
            ("4:3", 1024.0, 768.0),
            ("4:3", 1600.0, 1200.0),
        ];
        for (name, w, h) in screens {
            for scale in [0.8, 1.0, 1.25, 1.5] {
                let l = HudLayout::at(w, h, style::unit_at(w, h, scale));
                let s = l.safe;
                assert!((s - 0.05 * w.min(h)).abs() < 1e-3);
                let parts = l.parts();
                for (i, (a, r)) in parts.iter().enumerate() {
                    let inside = r[0] >= s - 0.01 && r[1] >= s - 0.01 && r[0] + r[2] <= w - s + 0.01 && r[1] + r[3] <= h - s + 0.01;
                    assert!(inside, "{name} {w}x{h} at {scale}: {a} {r:?} leaves the safe area");
                    assert!(r[2] > 0.0 && r[3] > 0.0, "{a}");
                    for (b, q) in &parts[i + 1..] {
                        assert!(apart(*r, *q), "{name} {w}x{h} at {scale}: {a} {r:?} and {b} {q:?} overlap");
                    }
                }
                // The minimap is the square the map builder was promised.
                assert_eq!(l.minimap, [w - s - 7.0 * l.t, s, 7.0 * l.t, 7.0 * l.t]);
                // The top centre keeps a useful width.
                assert!(l.top[2] >= 14.0 * l.t, "{name} {w}x{h} at {scale}: the top centre is {}", l.top[2] / l.t);
            }
        }
    }

    #[test]
    fn the_ui_scale_grows_the_hud_where_there_is_room() {
        let a = HudLayout::at(1920.0, 1080.0, style::unit_at(1920.0, 1080.0, 1.0));
        let b = HudLayout::at(1920.0, 1080.0, style::unit_at(1920.0, 1080.0, 1.25));
        assert!(b.t > a.t * 1.2 && b.health[2] > a.health[2]);
        // 1080p's unit is 28 px: the dialogue's portrait is 196 px, as UI.md §6 has it.
        assert_eq!(a.t, 28.0);
    }
}
