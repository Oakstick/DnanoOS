//! The Garden of Krampus (dungeon I) and the title stage, to the art direction: the board's
//! palette laid over the garden's own scenery rather than replacing it. The garden keeps its
//! hedges, woods, fountain, channels, churchyard and temples; a grade pulls them into the board:
//! greens to moss and olive, chalk-white paving down to pale stone (and dressed-stone
//! material), loud blossoms quieter. Its lanterns and fireflies burn amber, and the night is
//! the board's mist teal under the moon.

use super::*;

pub(super) mod pal {
    /// The garden's lantern glass and light, and its fireflies: the board's amber.
    pub const LANTERN: u32 = 0xffb050;
    pub const FIREFLY: u32 = 0xf0c070;
    /// The night: a teal-charcoal dark, the moon's light cold mist teal.
    pub const NIGHT: u32 = 0x0b1416;
    pub const MOONLIGHT: u32 = 0xa8c8c4;
    pub const MIST_TEAL: u32 = 0x6e9a90;
    /// The ground mist over the walks: the night's mist teal, greyed.
    pub const MIST: u32 = 0x7a8c88;
}
use pal::*;

fn to_srgb(x: f32) -> f32 {
    if x <= 0.003_130_8 {
        x * 12.92
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    }
}

fn to_linear(x: f32) -> f32 {
    if x <= 0.040_45 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}

/// Hue (degrees), saturation and value of a display colour.
fn hsv(c: Vec3) -> (f32, f32, f32) {
    let (mx, mn) = (c.max_element(), c.min_element());
    let d = mx - mn;
    let h = if d <= 1e-6 {
        0.0
    } else if mx == c.x {
        60.0 * ((c.y - c.z) / d).rem_euclid(6.0)
    } else if mx == c.y {
        60.0 * ((c.z - c.x) / d + 2.0)
    } else {
        60.0 * ((c.x - c.y) / d + 4.0)
    };
    (h, if mx <= 1e-6 { 0.0 } else { d / mx }, mx)
}

fn from_hsv(h: f32, s: f32, v: f32) -> Vec3 {
    let c = v * s;
    let hp = h.rem_euclid(360.0) / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Vec3::new(r, g, b) + Vec3::splat(v - c)
}

/// Grades one (linear) albedo into the board, and says whether it reads as stone.
pub(super) fn grade(c: Vec3) -> (Vec3, bool) {
    grade_keeping(c, None)
}

/// How far apart two hues are, in degrees.
fn hue_gap(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

/// As `grade`, but a colour near the hue of `accent` (a realm's sin) keeps its saturation, a
/// little deepened: the accent the realm is known by survives the grade.
pub(super) fn grade_keeping(c: Vec3, accent: Option<f32>) -> (Vec3, bool) {
    let s = Vec3::new(to_srgb(c.x.clamp(0.0, 1.0)), to_srgb(c.y.clamp(0.0, 1.0)), to_srgb(c.z.clamp(0.0, 1.0)));
    let (mut h, mut sat, mut v) = hsv(s);
    let mut stone = false;
    if let Some(a) = accent.filter(|&a| sat >= 0.2 && hue_gap(h, a) < 28.0) {
        h = a + (h - a) * 0.6;
        sat = (sat * 0.92).min(0.9);
        v = v.min(0.9) * 0.95;
    } else if sat < 0.2 {
        // Stone and greys: chalk down to the board's pale stone, warmed a touch.
        if v > 0.45 {
            v = 0.45 + (v - 0.45) * 0.5;
        }
        h = 42.0;
        sat = sat * 0.5 + 0.07;
        stone = v > 0.25;
    } else if (60.0..=175.0).contains(&h) {
        // Greens: toward moss and olive, quieter and deeper; the brightest leaves held down most.
        h = 98.0 + (h - 98.0) * 0.5;
        sat *= 0.68;
        v *= 0.95;
        if v > 0.5 {
            v = 0.5 + (v - 0.5) * 0.5;
        }
    } else if (15.0..60.0).contains(&h) && v < 0.6 {
        // Wood and earth: toward umber.
        sat *= 0.85;
    } else {
        // Blossoms, berries, banners: still there, but quieter.
        sat *= 0.75;
        v *= 0.86;
    }
    let g = from_hsv(h, sat, v);
    (Vec3::new(to_linear(g.x), to_linear(g.y), to_linear(g.z)), stone)
}

/// The garden's night: the moon over a mist-teal dark.
pub fn garden_atmosphere() -> Atmosphere {
    Atmosphere {
        background: NIGHT,
        fog_near: 14.0,
        fog_far: 34.0,
        hemi_sky: lin(MIST_TEAL) * 0.75,
        hemi_ground: lin(0x161c16),
        ambient: lin(0x26302e) * 0.75,
        sun_dir: Vec3::new(-6.0, 14.0, 8.0).normalize(),
        sun: lin(MOONLIGHT) * 1.1,
        exposure: 1.2,
        night_sky: 1.0,
        bloom: 1.0,
        moon_view: Vec3::new(-1.103, 0.534, -0.311).normalize(),
        cloud_tint: lin(0x121a1c),
    }
}

/// The Garden of Krampus at night, in the Cisterns' key: darker than the title's night, the
/// moon a cool, weak rim, the ambient low, so the lanterns' amber pools carry the frame and the
/// teal is left to the shadows and the mist.
pub fn garden_night() -> Atmosphere {
    Atmosphere {
        hemi_sky: lin(MIST_TEAL) * 0.4,
        hemi_ground: lin(0x141a16) * 0.8,
        ambient: lin(0x26302e) * 0.42,
        sun: lin(0x98bcc8) * 0.55,
        exposure: 1.15,
        bloom: 1.05,
        ..garden_atmosphere()
    }
}

impl WorldView {
    /// Grades the level's static scenery (the garden, its gate) into the board. Glowing pieces
    /// (lantern glass, candles) keep their colour; plain stone takes the dressed-stone material.
    pub(super) fn grade_estate(&mut self) {
        self.grade_scenery(None);
    }

    /// Grades the level's static scenery into the board, keeping the hue of `accent` (a realm's
    /// sin colour) as its accent.
    pub(super) fn grade_scenery(&mut self, accent: Option<u32>) {
        let accent = accent.map(|a| hsv(Vec3::new(((a >> 16) & 255) as f32, ((a >> 8) & 255) as f32, (a & 255) as f32) / 255.0).0);
        // (Trees are not stone, however pale a birch's bark.)
        let mut wood = vec![false; self.static_cubes.len()];
        for t in &self.trees {
            for w in wood.iter_mut().skip(t.range.0).take(t.range.1) {
                *w = true;
            }
        }
        // (What was built in the board's palette already keeps its colours.)
        let mut keep = vec![false; self.static_cubes.len()];
        for &(start, len) in &self.authored {
            for k in keep.iter_mut().skip(start).take(len) {
                *k = true;
            }
        }
        for ((c, &wood), &keep) in self.static_cubes.iter_mut().zip(&wood).zip(&keep) {
            if keep {
                continue;
            }
            let lit = c.emissive[3] == SHADE_LIT;
            let glowing = c.emissive[0] + c.emissive[1] + c.emissive[2] > 0.0;
            if glowing {
                continue;
            }
            let (g, stone) = grade_keeping(Vec3::new(c.color[0], c.color[1], c.color[2]), accent);
            c.color[0] = g.x;
            c.color[1] = g.y;
            c.color[2] = g.z;
            if stone && lit && !wood {
                c.emissive[3] = crypt::SHADE_STONE;
            }
        }
    }
}

/// A mesh with its colours graded into the board (the title stage's trees).
pub(super) fn graded(m: &Mesh) -> Mesh {
    Mesh { size: m.size, cubes: m.cubes.iter().map(|(p, c)| (*p, grade(*c).0)).collect() }
}
