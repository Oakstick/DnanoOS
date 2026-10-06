//! The player's settings that are not kept elsewhere (UI.md §8): the shield's hold or toggle, the
//! mouse and the sticks, the four volumes, the quality preset, the UI's scale and the size of its
//! prose, and the accessibility options. Each is a `vh.*` key in the settings file (`platform`), read
//! forgivingly: a value missing or not understood is its default.
//!
//! The settings kept where they always were stay there: the camera (`vh.camera`), the minimap's
//! turning (`vh.minimap_turn`, `game::fullmap`), free movement
//! (`vh.free_move`), slow motion and the shake (`game::Feel`), the rumble (`vh.rumble`), the full
//! screen (`vh.fullscreen`), mute (`vh.muted`) and the bindings (`vh.bindings`). The settings page
//! shows them all together. Nothing here is read by the simulation: they change what is drawn and
//! heard, and how the controls are read, never the rules.

use crate::platform;

/// Colour-blind palettes for the tells and markers (red and green swapped for colours each kind
/// of colour blindness tells apart).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum ColourBlind {
    #[default]
    Off,
    /// Green weak (the commonest): red and green read alike.
    Deuteranopia,
    /// Red weak: red reads dark as well as like green.
    Protanopia,
    /// Blue weak: blue and yellow read alike.
    Tritanopia,
}

impl ColourBlind {
    pub const ALL: [ColourBlind; 4] = [ColourBlind::Off, ColourBlind::Deuteranopia, ColourBlind::Protanopia, ColourBlind::Tritanopia];

    pub fn name(self) -> &'static str {
        match self {
            ColourBlind::Off => "off",
            ColourBlind::Deuteranopia => "deuteranopia",
            ColourBlind::Protanopia => "protanopia",
            ColourBlind::Tritanopia => "tritanopia",
        }
    }

    /// As the settings page writes it.
    pub fn label(self) -> &'static str {
        match self {
            ColourBlind::Off => "Off",
            ColourBlind::Deuteranopia => "Deuteranopia",
            ColourBlind::Protanopia => "Protanopia",
            ColourBlind::Tritanopia => "Tritanopia",
        }
    }
}

/// The size of the prose: the dialogue, the old man's guide, the tips.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum TextSize {
    Small,
    #[default]
    Normal,
    Large,
}

impl TextSize {
    pub const ALL: [TextSize; 3] = [TextSize::Small, TextSize::Normal, TextSize::Large];

    pub fn name(self) -> &'static str {
        match self {
            TextSize::Small => "small",
            TextSize::Normal => "normal",
            TextSize::Large => "large",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            TextSize::Small => "Small",
            TextSize::Normal => "Normal",
            TextSize::Large => "Large",
        }
    }

    /// What the prose's size is multiplied by.
    pub fn scale(self) -> f32 {
        match self {
            TextSize::Small => 0.88,
            TextSize::Normal => 1.0,
            TextSize::Large => 1.22,
        }
    }
}

/// The settings (see the module's words), as the player has them.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// The shield is raised by a press and lowered by the next (else it is held).
    pub shield_toggle: bool,
    /// The mouse's turn of the camera, in percent.
    pub mouse_sensitivity: u8,
    /// Up and down turned over: for the mouse, and for the right stick, apart.
    pub invert_mouse_y: bool,
    pub invert_stick_y: bool,
    /// How far a stick must be pushed before it counts, in percent: the left (running) and the
    /// right (the camera).
    pub move_deadzone: u8,
    pub look_deadzone: u8,
    /// The four volumes, in percent: everything, the music, the effects, the voices.
    pub master: u8,
    pub music: u8,
    pub effects: u8,
    pub voices: u8,
    /// The quality preset's name (`render::preset`).
    pub preset: &'static str,
    /// The UI's scale in percent (80 to 150): the HUD's unit is multiplied by it.
    pub ui_scale: u8,
    pub text_size: TextSize,
    pub colour_blind: ColourBlind,
    /// How dark the backing behind subtitles and dialogue is, in percent.
    pub backing: u8,
    /// Flashes and white-outs dimmed, nothing strobing.
    pub reduce_flashing: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            shield_toggle: false,
            mouse_sensitivity: 100,
            invert_mouse_y: false,
            invert_stick_y: false,
            move_deadzone: 40,
            look_deadzone: 16,
            master: 100,
            music: 100,
            effects: 100,
            voices: 100,
            preset: crate::render::preset::Preset::default_for_build().name,
            ui_scale: 100,
            text_size: TextSize::Normal,
            colour_blind: ColourBlind::Off,
            backing: 85,
            reduce_flashing: false,
        }
    }
}

/// A setting's range: its least, its most, and the step the page moves it by.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Range {
    pub min: u8,
    pub max: u8,
    pub step: u8,
}

impl Range {
    /// `v` stepped `by` steps (either way), kept in the range.
    pub fn stepped(self, v: u8, by: i32) -> u8 {
        (v as i32 + by * self.step as i32).clamp(self.min as i32, self.max as i32) as u8
    }

    /// How far along the range `v` is, 0 to 1 (a slider's fill).
    pub fn frac(self, v: u8) -> f32 {
        (v.saturating_sub(self.min)) as f32 / (self.max - self.min).max(1) as f32
    }

    /// The value at `frac` along the range, on a step (a slider clicked).
    pub fn at(self, frac: f32) -> u8 {
        let v = self.min as f32 + frac.clamp(0.0, 1.0) * (self.max - self.min) as f32;
        let k = ((v - self.min as f32) / self.step as f32).round() as i32;
        self.stepped(self.min, k)
    }
}

pub const SENSITIVITY: Range = Range { min: 25, max: 250, step: 25 };
pub const MOVE_DEADZONE: Range = Range { min: 10, max: 70, step: 5 };
pub const LOOK_DEADZONE: Range = Range { min: 4, max: 50, step: 2 };
pub const VOLUME: Range = Range { min: 0, max: 100, step: 5 };
pub const UI_SCALE: Range = Range { min: 80, max: 150, step: 10 };
pub const BACKING: Range = Range { min: 0, max: 100, step: 5 };

/// The keys, each once (the round trip's test walks them).
pub const KEYS: [&str; 16] = [
    "vh.shield_toggle",
    "vh.mouse_sensitivity",
    "vh.invert_mouse_y",
    "vh.invert_stick_y",
    "vh.deadzone_move",
    "vh.deadzone_look",
    "vh.volume_master",
    "vh.volume_music",
    "vh.volume_effects",
    "vh.volume_voices",
    "vh.preset",
    "vh.ui_scale",
    "vh.text_size",
    "vh.colour_blind",
    "vh.backing",
    "vh.reduce_flashing",
];

fn flag(v: bool) -> String {
    if v { "1" } else { "0" }.to_string()
}

impl Settings {
    /// Each setting as its key and saved value, in `KEYS`' order.
    pub fn entries(&self) -> Vec<(&'static str, String)> {
        let values = [
            flag(self.shield_toggle),
            self.mouse_sensitivity.to_string(),
            flag(self.invert_mouse_y),
            flag(self.invert_stick_y),
            self.move_deadzone.to_string(),
            self.look_deadzone.to_string(),
            self.master.to_string(),
            self.music.to_string(),
            self.effects.to_string(),
            self.voices.to_string(),
            self.preset.to_string(),
            self.ui_scale.to_string(),
            self.text_size.name().to_string(),
            self.colour_blind.name().to_string(),
            self.backing.to_string(),
            flag(self.reduce_flashing),
        ];
        KEYS.iter().copied().zip(values).collect()
    }

    /// Read through `get` (a key's saved value, if any): a value missing, out of its range or not
    /// understood is its default.
    pub fn read(get: impl Fn(&str) -> Option<String>) -> Self {
        let d = Self::default();
        let flag = |k: &str, def: bool| match get(k).as_deref().map(str::trim) {
            Some("1") => true,
            Some("0") => false,
            _ => def,
        };
        let num = |k: &str, r: Range, def: u8| {
            get(k).and_then(|v| v.trim().parse::<u8>().ok()).filter(|v| (r.min..=r.max).contains(v)).unwrap_or(def)
        };
        let preset = get("vh.preset").and_then(|n| crate::render::preset::Preset::named(n.trim())).map(|p| p.name).unwrap_or(d.preset);
        let text_size = get("vh.text_size").and_then(|v| TextSize::ALL.into_iter().find(|t| t.name() == v.trim())).unwrap_or_default();
        let colour_blind =
            get("vh.colour_blind").and_then(|v| ColourBlind::ALL.into_iter().find(|c| c.name() == v.trim())).unwrap_or_default();
        Self {
            shield_toggle: flag("vh.shield_toggle", d.shield_toggle),
            mouse_sensitivity: num("vh.mouse_sensitivity", SENSITIVITY, d.mouse_sensitivity),
            invert_mouse_y: flag("vh.invert_mouse_y", d.invert_mouse_y),
            invert_stick_y: flag("vh.invert_stick_y", d.invert_stick_y),
            move_deadzone: num("vh.deadzone_move", MOVE_DEADZONE, d.move_deadzone),
            look_deadzone: num("vh.deadzone_look", LOOK_DEADZONE, d.look_deadzone),
            master: num("vh.volume_master", VOLUME, d.master),
            music: num("vh.volume_music", VOLUME, d.music),
            effects: num("vh.volume_effects", VOLUME, d.effects),
            voices: num("vh.volume_voices", VOLUME, d.voices),
            preset,
            ui_scale: num("vh.ui_scale", UI_SCALE, d.ui_scale),
            text_size,
            colour_blind,
            backing: num("vh.backing", BACKING, d.backing),
            reduce_flashing: flag("vh.reduce_flashing", d.reduce_flashing),
        }
    }

    /// As saved (tests keep to the defaults).
    pub fn load() -> Self {
        if cfg!(test) {
            return Self::default();
        }
        Self::read(platform::load)
    }

    /// Saved, every key in one write (never from tests; captures save into their own folder).
    pub fn save(&self) {
        if cfg!(test) {
            return;
        }
        self.save_now();
    }

    /// Saved, whatever is running (the round trip's test, into its own folder).
    pub fn save_now(&self) {
        let entries = self.entries();
        let pairs: Vec<(&str, &str)> = entries.iter().map(|(k, v)| (*k, v.as_str())).collect();
        if let Err(e) = platform::save_all(&pairs) {
            platform::warn(&format!("could not save the settings: {e}"));
        }
    }

    /// What the HUD needs of them as it draws.
    pub fn ui(&self) -> UiPrefs {
        UiPrefs {
            prose: self.text_size.scale(),
            colour_blind: self.colour_blind,
            backing: self.backing as f32 / 100.0,
            reduce_flashing: self.reduce_flashing,
        }
    }
}

/// The settings the HUD reads as it draws (`render::hud::prefs`; the UI's scale is
/// `hud::style`'s): the prose's size, the colour-blind palette, the backing's darkness and the
/// flashes dimmed.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct UiPrefs {
    pub prose: f32,
    pub colour_blind: ColourBlind,
    pub backing: f32,
    pub reduce_flashing: bool,
}

impl Default for UiPrefs {
    fn default() -> Self {
        Settings::default().ui()
    }
}

/// The settings key for the device used last (`input::Device`), for the engine's start screen.
pub const LAST_DEVICE_KEY: &str = "vh.last_device";

/// The device used last as saved (a controller, until one is known: the start screen's legend
/// shows the pad).
pub fn device_name(d: crate::input::Device) -> &'static str {
    use crate::input::Device;
    match d {
        Device::KeyboardMouse => "kbm",
        Device::Xbox => "xbox",
        Device::PlayStation => "ps",
        Device::Generic => "pad",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_setting_round_trips_through_the_settings_file() {
        let dir = std::env::temp_dir().join(format!("vh-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        platform::use_dir_on_this_thread(dir.clone());
        // Nothing saved: the defaults.
        assert_eq!(Settings::read(platform::load), Settings::default());
        // Every one changed from its default, saved, and read back from the file.
        let s = Settings {
            shield_toggle: true,
            mouse_sensitivity: 175,
            invert_mouse_y: true,
            invert_stick_y: true,
            move_deadzone: 25,
            look_deadzone: 30,
            master: 55,
            music: 20,
            effects: 75,
            voices: 0,
            preset: "low",
            ui_scale: 130,
            text_size: TextSize::Large,
            colour_blind: ColourBlind::Tritanopia,
            backing: 40,
            reduce_flashing: true,
        };
        for (k, v) in Settings::default().entries() {
            let changed = s.entries().into_iter().find(|(k2, _)| *k2 == k).unwrap().1;
            assert_ne!(v, changed, "{k} is not changed by the test");
        }
        s.save_now();
        let text = std::fs::read_to_string(dir.join("settings.txt")).unwrap();
        for k in KEYS {
            assert!(text.contains(&format!("{k}=")), "{k} is in the file");
        }
        assert_eq!(Settings::read(platform::load), s);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn values_out_of_range_or_unknown_are_the_defaults() {
        let bad = |k: &str| -> Option<String> {
            Some(
                match k {
                    "vh.ui_scale" => "300",
                    "vh.volume_master" => "-4",
                    "vh.preset" => "ultra",
                    "vh.text_size" => "huge",
                    "vh.colour_blind" => "plaid",
                    "vh.shield_toggle" => "yes",
                    "vh.deadzone_move" => "2",
                    _ => "junk",
                }
                .to_string(),
            )
        };
        assert_eq!(Settings::read(bad), Settings::default());
        let ok = |k: &str| (k == "vh.ui_scale").then(|| " 120 ".to_string());
        assert_eq!(Settings::read(ok).ui_scale, 120);
    }

    #[test]
    fn ranges_step_and_clamp() {
        assert_eq!(VOLUME.stepped(100, 1), 100);
        assert_eq!(VOLUME.stepped(0, -1), 0);
        assert_eq!(VOLUME.stepped(50, 2), 60);
        assert_eq!(UI_SCALE.stepped(100, 1), 110);
        assert_eq!(UI_SCALE.at(0.0), 80);
        assert_eq!(UI_SCALE.at(1.0), 150);
        assert_eq!(VOLUME.at(0.51), 50);
        assert!((UI_SCALE.frac(115) - 0.5).abs() < 1e-6);
        // Every default sits in its range, on a step.
        let d = Settings::default();
        for (r, v) in [
            (SENSITIVITY, d.mouse_sensitivity),
            (MOVE_DEADZONE, d.move_deadzone),
            (LOOK_DEADZONE, d.look_deadzone),
            (VOLUME, d.master),
            (UI_SCALE, d.ui_scale),
            (BACKING, d.backing),
        ] {
            assert!((r.min..=r.max).contains(&v) && (v - r.min) % r.step == 0, "{r:?} {v}");
        }
    }
}
