//! The HUD's look (UI.md §4): dark glass panels with a soft inner gradient, a thin copper rule and
//! notched corners, laid on a soft shadow so they hold over the vaults' gold and the ending's white;
//! bars with the trail of what a blow cost and a glow at their edge; pills, badges, dividers; and
//! the unit every size is measured in, with the safe area round the screen's edge.
//!
//! The panels are 9-slice art, drawn here as shapes into the UI atlas's `ICON_REGION` at start-up
//! (`art::bake`, written by `Gpu::new`): any size from one picture. The archangels' wings, their
//! halo, a soft glow, a crisp disc, the whisper's smoke and a fade are baked beside them.
//!
//! ```ignore
//! let t = unit(w, h);                       // the HUD unit, with the player's UI scale
//! style::panel_glass(o, [x, y, pw, ph], t, 1.0);
//! style::bar(o, [x, y, bw, bh], hp, trail, 0x5cff6a, 1.0);
//! let (alpha, slide) = style::appear(age, left);   // in and out over 0.15 s
//! ```

use std::f32::consts::{FRAC_PI_2, TAU};
use std::sync::atomic::{AtomicU32, Ordering};

use super::{mix_rgb, COPPER, GOLD, INK, SAND, TEXT};
use crate::render::overlay::{rgba, Align, AtlasRegion, Overlay, Style, ICON_REGION};

// ------------------------------------------------------------------ the unit

/// The player's UI scale (80% to 150%), as f32 bits; the settings page sets it (`set_ui_scale`).
static UI_SCALE: AtomicU32 = AtomicU32::new(0x3f80_0000);

/// The UI scale's range.
pub const UI_SCALE_MIN: f32 = 0.8;
pub const UI_SCALE_MAX: f32 = 1.5;

/// Sets the UI scale (clamped to its range); every size in the HUD follows from the next frame.
pub fn set_ui_scale(scale: f32) {
    UI_SCALE.store(scale.clamp(UI_SCALE_MIN, UI_SCALE_MAX).to_bits(), Ordering::Relaxed);
}

/// The UI scale now (1 unless the settings have changed it).
pub fn ui_scale() -> f32 {
    f32::from_bits(UI_SCALE.load(Ordering::Relaxed))
}

/// The layout's grid: the unit is the screen's width over this many, or its height over that many,
/// whichever is less.
const COLS: f32 = 64.0;
const ROWS: f32 = 36.0;
/// The unit's least (a small window) and where it stops growing with the window up to 1080p
/// (28 px); past that it grows on in proportion, so 1440p and 4K keep 1080p's proportions.
const MIN_UNIT: f32 = 12.0;
const UNIT_1080: f32 = 28.0;

/// The HUD unit for a `w` by `h` screen at `scale` (see `unit`).
pub fn unit_at(w: f32, h: f32, scale: f32) -> f32 {
    let base = (w / COLS).min(h / ROWS);
    let t = if base <= 30.0 { base.clamp(MIN_UNIT, UNIT_1080) } else { UNIT_1080 * base / 30.0 };
    t * scale
}

/// The HUD unit for a `w` by `h` screen (logical pixels): 20 px at 720p, 28 at 1080p, 56 at
/// 2160p, times the player's UI scale. Every size in the HUD is a share of it.
pub fn unit(w: f32, h: f32) -> f32 {
    unit_at(w, h, ui_scale())
}

/// The safe area's margin: 5% of the shorter edge (UI.md §2), nothing of the HUD outside it.
pub fn safe(w: f32, h: f32) -> f32 {
    0.05 * w.min(h)
}

/// The type's sizes, in units: what is read is never under 18 px at 1080p.
pub mod size {
    /// Body text: the old man's words, the objective.
    pub const BODY: f32 = 0.72;
    /// Labels and figures in the panels.
    pub const LABEL: f32 = 0.64;
    /// Names in the display serif.
    pub const NAME: f32 = 0.78;
    /// Small marks (a slot's number, a timer under an icon).
    pub const MARK: f32 = 0.5;
}

/// The alert red (UI.md §4), and the warm white of a trail.
pub const ALERT: u32 = 0xe8413a;
pub const TRAIL: u32 = 0xf4ead8;

// ------------------------------------------------------------- the atlas art

/// The pictures in `ICON_REGION`, made from shapes at start-up.
pub mod art {
    use super::*;

    const fn at(x: u32, y: u32, w: u32, h: u32) -> AtlasRegion {
        AtlasRegion::new(ICON_REGION.x + x, ICON_REGION.y + y, w, h)
    }

    /// The glass: ink, lighter at the top, its corners cut (9-slice, `GLASS_BORDER`).
    pub const GLASS: AtlasRegion = at(0, 0, 96, 96);
    /// The rule round the glass, white (tinted copper, amber, a sin's colour), the cuts brighter.
    pub const FRAME: AtlasRegion = at(96, 0, 96, 96);
    pub const GLASS_BORDER: u32 = 32;
    /// The notch's size in the art, in texels of the 96.
    pub const NOTCH: f32 = 12.0;
    /// A soft shadow round a panel (9-slice, `SHADOW_BORDER`; the panel's edge `SHADOW_INSET` in).
    pub const SHADOW: AtlasRegion = at(192, 0, 128, 128);
    pub const SHADOW_BORDER: u32 = 56;
    pub const SHADOW_INSET: f32 = 40.0;
    /// A soft round glow, white.
    pub const GLOW: AtlasRegion = at(320, 0, 64, 64);
    /// A halo: a ring with its glow, white.
    pub const HALO: AtlasRegion = at(384, 0, 128, 128);
    /// Two wings spread from the middle of their foot, white, their feathers shaded.
    pub const WINGS: AtlasRegion = at(0, 128, 256, 160);
    /// A wisp of smoke, white.
    pub const SMOKE: AtlasRegion = at(256, 128, 128, 128);
    /// Opaque at the top, clear at the bottom.
    pub const FADE: AtlasRegion = at(384, 128, 32, 128);
    /// A crisp disc, white.
    pub const DISC: AtlasRegion = at(416, 128, 96, 96);

    /// Every picture's place, for the test that they keep apart.
    pub const ALL: [AtlasRegion; 9] = [GLASS, FRAME, SHADOW, GLOW, HALO, WINGS, SMOKE, FADE, DISC];

    fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
        let k = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
        k * k * (3.0 - 2.0 * k)
    }

    /// How much of a texel a shape covers, from its signed distance (negative inside), in texels.
    fn cover(d: f32) -> f32 {
        (0.5 - d).clamp(0.0, 1.0)
    }

    /// The glass's edge as a signed distance (texels; negative inside) at `(x, y)` of a 96 square.
    pub(super) fn notched(x: f32, y: f32) -> f32 {
        let (dx, dy) = ((x - 48.0).abs(), (y - 48.0).abs());
        (dx - 48.0).max(dy - 48.0).max((dx + dy - (96.0 - NOTCH)) * std::f32::consts::FRAC_1_SQRT_2)
    }

    /// A segment's distance (a capsule of radius `r` from `a` to `b`), negative inside.
    fn capsule(p: [f32; 2], a: [f32; 2], b: [f32; 2], ra: f32, rb: f32) -> (f32, f32) {
        let (pax, pay) = (p[0] - a[0], p[1] - a[1]);
        let (bax, bay) = (b[0] - a[0], b[1] - a[1]);
        let h = ((pax * bax + pay * bay) / (bax * bax + bay * bay)).clamp(0.0, 1.0);
        let (qx, qy) = (pax - bax * h, pay - bay * h);
        ((qx * qx + qy * qy).sqrt() - (ra + (rb - ra) * h), h)
    }

    /// A hash of a lattice point, 0..1.
    fn hash(x: i32, y: i32) -> f32 {
        let mut n = (x.wrapping_mul(374_761_393) ^ y.wrapping_mul(668_265_263)) as u32;
        n = (n ^ (n >> 13)).wrapping_mul(1_274_126_177);
        ((n ^ (n >> 16)) & 0xffff) as f32 / 65535.0
    }

    fn noise(x: f32, y: f32) -> f32 {
        let (xi, yi) = (x.floor() as i32, y.floor() as i32);
        let (fx, fy) = (x - x.floor(), y - y.floor());
        let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
        let a = hash(xi, yi) + (hash(xi + 1, yi) - hash(xi, yi)) * sx;
        let b = hash(xi, yi + 1) + (hash(xi + 1, yi + 1) - hash(xi, yi + 1)) * sx;
        a + (b - a) * sy
    }

    /**
     * The art, `ICON_REGION`'s RGBA rows (sRGB, straight alpha), for `Gpu::write_atlas`. Each
     * picture is drawn from shapes by their distances, so its edges are smooth at the size made.
     */
    pub fn bake() -> Vec<u8> {
        let (rw, rh) = (ICON_REGION.w as usize, ICON_REGION.h as usize);
        let mut px = vec![0u8; rw * rh * 4];
        let mut paint = |r: AtlasRegion, f: &dyn Fn(f32, f32) -> [f32; 4]| {
            let (ox, oy) = ((r.x - ICON_REGION.x) as usize, (r.y - ICON_REGION.y) as usize);
            for y in 0..r.h as usize {
                for x in 0..r.w as usize {
                    let c = f(x as f32 + 0.5, y as f32 + 0.5);
                    let i = ((oy + y) * rw + ox + x) * 4;
                    for k in 0..4 {
                        px[i + k] = (c[k].clamp(0.0, 1.0) * 255.0).round() as u8;
                    }
                }
            }
        };
        // The glass: ink with a teal cast, lit a little from above, darker below; a faint sheen
        // along the inside of its top edge.
        paint(GLASS, &|x, y| {
            let d = notched(x, y);
            let k = y / 96.0;
            let lift = 0.034 * (1.0 - smoothstep(0.0, 0.75, k)) - 0.012 * smoothstep(0.55, 1.0, k);
            let ink = [12.0 / 255.0 + lift, 17.0 / 255.0 + lift * 1.1, 17.0 / 255.0 + lift];
            let sheen = (1.0 - ((d + 3.5).abs() / 1.5)).clamp(0.0, 1.0) * smoothstep(48.0, 20.0, y) * 0.07;
            let a = 0.86 + 0.06 * k;
            [ink[0] + sheen, ink[1] + sheen, ink[2] + sheen, cover(d) * a]
        });
        // The rule: a line two texels wide just inside the edge, the corners' cuts brighter.
        paint(FRAME, &|x, y| {
            let d = notched(x, y);
            let line = (1.25 - (d + 1.6).abs()).clamp(0.0, 1.0);
            let (dx, dy) = ((x - 48.0).abs(), (y - 48.0).abs());
            let on_cut = dx + dy > 96.0 - NOTCH - 3.0 && dx > 48.0 - NOTCH - 1.0 && dy > 48.0 - NOTCH - 1.0;
            let l = if on_cut { 1.0 } else { 0.82 };
            [l, l, l, line]
        });
        // The shadow: a soft dark ring falling off outside the panel's edge (40 texels in).
        paint(SHADOW, &|x, y| {
            let half = 64.0 - SHADOW_INSET;
            let (dx, dy) = (((x - 64.0).abs() - half).max(0.0), ((y - 64.0).abs() - half).max(0.0));
            let d = (dx * dx + dy * dy).sqrt();
            let a = 0.62 * (-(d / 15.0).powi(2)).exp();
            [0.0, 0.0, 0.0, a]
        });
        paint(GLOW, &|x, y| {
            let r2 = ((x - 32.0).powi(2) + (y - 32.0).powi(2)) / (32.0 * 32.0);
            let a = (-r2 * 3.6).exp() * (1.0 - r2.min(1.0)).powi(2);
            [1.0, 1.0, 1.0, a]
        });
        paint(HALO, &|x, y| {
            let r = ((x - 64.0).powi(2) + (y - 64.0).powi(2)).sqrt();
            let ring = cover((r - 44.0).abs() - 4.0);
            let glow = (-((r - 44.0) / 12.0).powi(2)).exp() * 0.45 * (1.0 - (r / 64.0).min(1.0)).sqrt();
            let lit = 0.86 + 0.14 * cover((r - 45.5).abs() - 1.2);
            [lit, lit, lit, ring.max(glow)]
        });
        paint(WINGS, &|x, y| wing_texel(x, y));
        paint(SMOKE, &|x, y| {
            let (u, v) = ((x - 64.0) / 64.0, (y - 64.0) / 64.0);
            let r = (u * u + v * v).sqrt();
            let n = 0.55 * noise(x / 22.0, y / 22.0) + 0.3 * noise(x / 9.0 + 7.0, y / 9.0) + 0.15 * noise(x / 4.0, y / 4.0 + 3.0);
            let a = smoothstep(0.42, 0.78, n) * (1.0 - smoothstep(0.35, 1.0, r));
            [1.0, 1.0, 1.0, a * 0.9]
        });
        paint(FADE, &|_, y| [1.0, 1.0, 1.0, smoothstep(127.0, 0.0, y)]);
        paint(DISC, &|x, y| {
            let r = ((x - 48.0).powi(2) + (y - 48.0).powi(2)).sqrt();
            [1.0, 1.0, 1.0, cover(r - 46.0)]
        });
        px
    }

    /**
     * The wings at a texel of their 256 by 160: from shoulders by the middle of the foot, each
     * wing a fan of primaries sweeping out and up, coverts over their roots, each feather shaded
     * along its length with a darker edge, so the tint reads as feathers.
     */
    fn wing_texel(x: f32, y: f32) -> [f32; 4] {
        // Laid feather over feather, each a little darker at its own edge, so the seams show.
        let (mut shade, mut alpha) = (1.0f32, 0.0f32);
        let mut lay = |d: f32, light: f32| {
            let c = cover(d);
            if c > 0.0 {
                let edge = (-d / 1.8).clamp(0.0, 1.0);
                shade += (light * (0.7 + 0.3 * edge) - shade) * c;
                alpha = alpha.max(c);
            }
        };
        for side in [-1.0f32, 1.0] {
            // The left wing in the picture's own terms, mirrored for the right.
            let px = if side < 0.0 { x } else { 256.0 - x };
            let p = [px, y];
            let (shoulder, wrist) = ([116.0, 100.0], [58.0, 30.0]);
            let along = |u: f32| {
                let bulge = (u * std::f32::consts::PI).sin() * 10.0;
                [shoulder[0] + (wrist[0] - shoulder[0]) * u - bulge * 0.4, shoulder[1] + (wrist[1] - shoulder[1]) * u - bulge]
            };
            // The flight feathers, from the shoulder's short ones to the wrist's long primaries,
            // the inner first so the outer lie over them.
            for i in 0..10 {
                let u = i as f32 / 9.0;
                let base = along(u);
                let (dx, dy) = (-0.12 - 0.76 * u, 1.0 - 0.48 * u);
                let l = (dx * dx + dy * dy).sqrt();
                let len = 36.0 + 24.0 * u;
                let tip = [base[0] + dx / l * len, base[1] + dy / l * len];
                let (d, h) = capsule(p, base, tip, 7.0, 3.6);
                lay(d, 0.96 - 0.2 * h);
            }
            // The arm of the wing: coverts along its leading edge, lighter.
            for i in 0..6 {
                let u = i as f32 / 5.0;
                let base = along(u);
                let tip = [base[0] - 6.0 - 4.0 * u, base[1] + 16.0 - 4.0 * u];
                let (d, _) = capsule(p, base, tip, 8.0, 6.0);
                lay(d, 1.0);
            }
        }
        [shade, shade, shade, alpha]
    }
}

// ------------------------------------------------------------------ shapes

/// A soft round glow of `color` with `r` its reach.
pub fn glow(o: &mut Overlay, cx: f32, cy: f32, r: f32, color: [f32; 4]) {
    o.region(art::GLOW, cx - r, cy - r, r * 2.0, r * 2.0, color);
}

/// A crisp disc (smooth at any size, where `Overlay::circle` shows its sides when large).
pub fn disc(o: &mut Overlay, cx: f32, cy: f32, r: f32, color: [f32; 4]) {
    let r = r * 48.0 / 46.0;
    o.region(art::DISC, cx - r, cy - r, r * 2.0, r * 2.0, color);
}

/// A ring's arc from `a0` to `a1` radians (clockwise from the right, y down), `width` wide, in
/// enough steps to look round, without the round caps `Overlay::arc` puts on every step.
#[allow(clippy::too_many_arguments)]
pub fn ring(o: &mut Overlay, cx: f32, cy: f32, r: f32, width: f32, a0: f32, a1: f32, color: [f32; 4]) {
    let steps = (((a1 - a0).abs() / TAU) * 72.0).ceil().max(2.0) as usize;
    let (ri, ro) = (r - width / 2.0, r + width / 2.0);
    let at = |a: f32, rr: f32| [cx + a.cos() * rr, cy + a.sin() * rr];
    for i in 0..steps {
        let (u, v) = (a0 + (a1 - a0) * i as f32 / steps as f32, a0 + (a1 - a0) * (i + 1) as f32 / steps as f32);
        o.triangle(at(u, ri), at(u, ro), at(v, ro), color);
        o.triangle(at(u, ri), at(v, ro), at(v, ri), color);
    }
}

/// A slice of a disc from `a0` to `a1` (a cooldown's radial fill over an icon).
pub fn pie(o: &mut Overlay, cx: f32, cy: f32, r: f32, a0: f32, a1: f32, color: [f32; 4]) {
    let steps = (((a1 - a0).abs() / TAU) * 48.0).ceil().max(1.0) as usize;
    for i in 0..steps {
        let (u, v) = (a0 + (a1 - a0) * i as f32 / steps as f32, a0 + (a1 - a0) * (i + 1) as f32 / steps as f32);
        o.triangle([cx, cy], [cx + u.cos() * r, cy + u.sin() * r], [cx + v.cos() * r, cy + v.sin() * r], color);
    }
}

/// A cooldown over a square icon: the part still to come back darkened, sweeping clockwise from
/// the top as it does (`left` 1 just used, 0 ready).
pub fn cooldown(o: &mut Overlay, x: f32, y: f32, s: f32, left: f32) {
    if left <= 0.0 {
        return;
    }
    let (cx, cy) = (x + s / 2.0, y + s / 2.0);
    let a0 = -FRAC_PI_2 + (1.0 - left.min(1.0)) * TAU;
    // (A disc wide enough to cover the square's corners, kept inside it by drawing a square's worth.)
    let r = s * 0.5;
    pie(o, cx, cy, r, a0, -FRAC_PI_2 + TAU, [0.0, 0.0, 0.0, 0.62]);
    ring(o, cx, cy, r * 0.96, (s * 0.06).max(1.5), a0, -FRAC_PI_2 + TAU, [1.0, 1.0, 1.0, 0.35]);
}

/// A small diamond, filled.
pub fn diamond(o: &mut Overlay, cx: f32, cy: f32, r: f32, color: [f32; 4]) {
    o.triangle([cx, cy - r], [cx + r, cy], [cx, cy + r], color);
    o.triangle([cx, cy - r], [cx, cy + r], [cx - r, cy], color);
}

// ------------------------------------------------------------------ panels

/// The panel's edge art drawn this many units wide at each corner.
const EDGE: f32 = 0.62;

/// The shadow under a panel: soft, dark, a little below it.
pub fn shadow(o: &mut Overlay, r: [f32; 4], t: f32, alpha: f32) {
    let s = t * 0.55;
    let edge = s * art::SHADOW_BORDER as f32 / art::SHADOW_INSET;
    o.nine_slice(
        art::SHADOW,
        art::SHADOW_BORDER,
        r[0] - s,
        r[1] - s + t * 0.12,
        r[2] + 2.0 * s,
        r[3] + 2.0 * s,
        edge,
        [1.0, 1.0, 1.0, alpha],
    );
}

/// The glass alone (no shadow, no rule), `alpha` its opacity.
pub fn glass(o: &mut Overlay, r: [f32; 4], t: f32, alpha: f32) {
    o.nine_slice(art::GLASS, art::GLASS_BORDER, r[0], r[1], r[2], r[3], t * EDGE, [1.0, 1.0, 1.0, alpha]);
}

/// The rule round a panel, in `color`.
pub fn frame(o: &mut Overlay, r: [f32; 4], t: f32, color: [f32; 4]) {
    o.nine_slice(art::FRAME, art::GLASS_BORDER, r[0], r[1], r[2], r[3], t * EDGE, color);
}

/// A dark glass panel (UI.md §4): its shadow, the glass, the copper rule, the notched corners.
pub fn panel_glass(o: &mut Overlay, r: [f32; 4], t: f32, alpha: f32) {
    panel_ruled(o, r, t, alpha, COPPER);
}

/// A glass panel with its rule in `rule`'s colour (amber for the old man, a sin's for a whisper).
pub fn panel_ruled(o: &mut Overlay, r: [f32; 4], t: f32, alpha: f32, rule: u32) {
    if r[2] <= 0.0 || r[3] <= 0.0 {
        return;
    }
    shadow(o, r, t, alpha);
    glass(o, r, t, alpha);
    frame(o, r, t, rgba(rule, 0.78 * alpha));
}

/// A panel's heading: a name in the display serif at its top left and a divider under it. The y
/// below it.
pub fn panel_header(o: &mut Overlay, r: [f32; 4], title: &str, t: f32, color: u32, alpha: f32) -> f32 {
    let pad = t * 0.55;
    let s = Style::display(t * size::LABEL, rgba(color, alpha)).tracking(0.06).shadow(t * 0.05, t * 0.08);
    o.text_styled(title, r[0] + pad, r[1] + t * 0.7, &s);
    divider(o, r[0] + pad, r[1] + t * 1.25, r[2] - pad * 2.0, t, rgba(COPPER, 0.7 * alpha));
    r[1] + t * 1.5
}

/// A divider: a thin line fading out at both ends, a diamond at its middle.
pub fn divider(o: &mut Overlay, x: f32, y: f32, w: f32, t: f32, color: [f32; 4]) {
    let lw = (t * 0.045).max(1.0);
    let n = 6;
    for i in 0..n {
        let k = i as f32 / n as f32;
        let a = color[3] * smooth(((k + 0.5 / n as f32) * 2.0 - 1.0).abs());
        let seg = w / n as f32;
        o.rect(x + seg * i as f32, y - lw / 2.0, seg + 0.5, lw, [color[0], color[1], color[2], a]);
    }
    diamond(o, x + w / 2.0, y, t * 0.12, color);
}

/// 1 in the middle, easing to 0 at the ends (`k` 0 middle .. 1 end).
fn smooth(k: f32) -> f32 {
    let k = 1.0 - k.clamp(0.0, 1.0);
    k * k * (3.0 - 2.0 * k)
}

/**
 * A bar (health, a boss's, a shield's): a dark track, the pale trail of what the last blow cost,
 * the fill in `color` lit along its top and shaded below, and a glow at its leading edge.
 * `frac` and `trail` run 0..1.
 */
pub fn bar(o: &mut Overlay, r: [f32; 4], frac: f32, trail: f32, color: u32, alpha: f32) {
    let [x, y, w, h] = r;
    let frac = frac.clamp(0.0, 1.0);
    let b = (h * 0.12).clamp(1.0, 2.5);
    o.rect(x - b, y - b, w + 2.0 * b, h + 2.0 * b, [0.0, 0.0, 0.0, 0.72 * alpha]);
    o.rect(x, y, w, h, [INK[0] * 2.4, INK[1] * 2.4, INK[2] * 2.4, 0.92 * alpha]);
    o.rect(x, y + h * 0.6, w, h * 0.4, [0.0, 0.0, 0.0, 0.25 * alpha]);
    let trail = trail.clamp(0.0, 1.0);
    if trail > frac {
        o.rect(x + w * frac, y, w * (trail - frac), h, rgba(TRAIL, 0.78 * alpha));
    }
    if frac <= 0.0 {
        return;
    }
    let fw = w * frac;
    o.rect(x, y, fw, h, rgba(color, alpha));
    o.rect(x, y, fw, h * 0.38, rgba(mix_rgb(color, 0xffffff, 0.45), 0.55 * alpha));
    o.rect(x, y + h * 0.72, fw, h * 0.28, [0.0, 0.0, 0.0, 0.22 * alpha]);
    // The glow at its edge: a soft light, and a bright line.
    let ex = x + fw;
    glow(o, ex, y + h / 2.0, h * 1.6, rgba(mix_rgb(color, 0xffffff, 0.35), 0.55 * alpha));
    o.rect(ex - (h * 0.12).max(1.0), y, (h * 0.12).max(1.0), h, rgba(mix_rgb(color, 0xffffff, 0.7), 0.9 * alpha));
}

/// Notches across a bar at its shares `at` (a boss's thirds).
pub fn notches(o: &mut Overlay, r: [f32; 4], at: &[f32], t: f32, color: [f32; 4]) {
    for &k in at {
        let x = r[0] + r[2] * k;
        o.rect(x - 1.0, r[1] - t * 0.1, 2.0, r[3] + t * 0.2, color);
    }
}

/// A pill: text in a rounded dark band. Its width. `x` is its left, middle or right by `style`.
pub fn pill(o: &mut Overlay, text: &str, x: f32, y: f32, style: &Style, back: [f32; 4], edge: Option<[f32; 4]>) -> f32 {
    let tw = o.measure_styled(text, style);
    let h = style.size * 1.15;
    let w = tw + h * 0.9;
    let left = match style.align {
        Align::Left => x,
        Align::Center => x - w / 2.0,
        Align::Right => x - w,
    };
    super::glyphs::rounded_rect(o, left, y - h / 2.0, w, h, h / 2.0, back);
    if let Some(e) = edge {
        super::glyphs::rounded_rect(o, left, y + h / 2.0 - 1.5, w, 1.5, 0.75, e);
    }
    o.text_styled(text, left + h * 0.45, y, &style.align(Align::Left));
    w
}

/// A badge: a number in a disc of `color` with a dark ring.
pub fn badge(o: &mut Overlay, text: &str, cx: f32, cy: f32, r: f32, color: u32, alpha: f32) {
    disc(o, cx, cy, r * 1.14, [0.0, 0.0, 0.0, 0.75 * alpha]);
    disc(o, cx, cy, r, rgba(color, alpha));
    let s = Style::body(r * 1.25, rgba(0x14100c, alpha)).bold().centered();
    o.text_styled(text, cx, cy, &s);
}

/// The fade (opaque to clear) over `[x, y, w, h]`, its opaque side toward `side`: 0 up, 1 right,
/// 2 down, 3 left.
pub fn fade_quad(o: &mut Overlay, r: [f32; 4], side: u8, color: [f32; 4]) {
    let [u0, v0, u1, v1] = art::FADE.uv();
    let [x, y, w, h] = r;
    let corners = [[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
    // Each corner's v: opaque (v0) on the named side.
    let (o_, c_) = (v0, v1);
    let v = match side {
        0 => [o_, o_, c_, c_],
        1 => [c_, o_, o_, c_],
        2 => [c_, c_, o_, o_],
        _ => [o_, c_, c_, o_],
    };
    let uv = [[u0, v[0]], [u1, v[1]], [u1, v[2]], [u0, v[3]]];
    o.textured_quad(corners, uv, color, 0);
}

/// A soft edge of `color` round the whole screen, `depth` deep (the hurt red, a hot flash).
pub fn vignette(o: &mut Overlay, depth: f32, color: [f32; 4]) {
    let (w, h) = (o.width, o.height);
    fade_quad(o, [0.0, 0.0, w, depth], 0, color);
    fade_quad(o, [0.0, h - depth, w, depth], 2, color);
    fade_quad(o, [0.0, 0.0, depth, h], 3, color);
    fade_quad(o, [w - depth, 0.0, depth, h], 1, color);
}

/// A band of dark glass across the screen at `y` (its middle), `bh` tall, softening above and
/// below, a thin rule of `rule` along each edge (a banner's ground).
pub fn band(o: &mut Overlay, y: f32, bh: f32, t: f32, rule: u32, alpha: f32) {
    let w = o.width;
    let ink = [INK[0], INK[1], INK[2], 0.74 * alpha];
    let soft = t * 0.9;
    o.rect(0.0, y - bh / 2.0, w, bh, ink);
    fade_quad(o, [0.0, y - bh / 2.0 - soft, w, soft], 2, ink);
    fade_quad(o, [0.0, y + bh / 2.0, w, soft], 0, ink);
    let lw = (t * 0.05).max(1.0);
    for k in [-1.0f32, 1.0] {
        let ry = y + k * bh / 2.0 - lw / 2.0;
        // The rule brightest in the middle, gone at the screen's sides.
        let n = 8;
        for i in 0..n {
            let kk = (i as f32 + 0.5) / n as f32;
            let a = smooth((kk * 2.0 - 1.0).abs()) * 0.7 * alpha;
            o.rect(w * i as f32 / n as f32, ry, w / n as f32 + 0.5, lw, rgba(rule, a));
        }
    }
}

/// Draws `content` (which returns how far down it reached), then puts `back` (drawn for that
/// height) under it: for a panel as tall as what goes in it.
pub fn under(o: &mut Overlay, content: impl FnOnce(&mut Overlay) -> f32, back: impl FnOnce(&mut Overlay, f32)) -> f32 {
    let start = o.verts.len();
    let bottom = content(o);
    let drawn: Vec<_> = o.verts.drain(start..).collect();
    back(o, bottom);
    o.verts.extend(drawn);
    bottom
}

/// Seconds a panel takes to come in or go.
pub const APPEAR: f32 = 0.15;

/**
 * How far in a panel is, `age` seconds after it came and `left` before it goes: its opacity and
 * how far it still has to slide (0..1 of its slide), eased, over `APPEAR` each way.
 */
pub fn appear(age: f32, left: f32) -> (f32, f32) {
    let ease = |k: f32| {
        let k = k.clamp(0.0, 1.0);
        1.0 - (1.0 - k).powi(3)
    };
    let a = ease(age / APPEAR).min(ease(left / APPEAR));
    (a, 1.0 - a)
}

/// Body text's style: the sans at `size` units, with a short shadow that holds over bright ground.
pub fn body(t: f32, units: f32, color: [f32; 4]) -> Style {
    Style::body(t * units, color).shadow((t * 0.05).max(1.0), (t * 0.06).max(1.0))
}

/// A label's style: the sans, a little heavier, spaced capitals.
pub fn label(t: f32, units: f32, color: [f32; 4]) -> Style {
    Style::body(t * units, color).weight(0.3).shadow((t * 0.05).max(1.0), (t * 0.06).max(1.0))
}

/// A name's style: the display serif, spaced, with a dark outline so it reads on anything.
pub fn name(t: f32, units: f32, color: [f32; 4]) -> Style {
    Style::display(t * units, color).tracking(0.05).outline((t * 0.05).max(1.0), [0.0, 0.0, 0.0, 0.55 * color[3]])
}

/// The text colours, for the modules that use the look.
pub const TEXT_MAIN: u32 = TEXT;
pub const TEXT_SAND: u32 = SAND;
pub const TEXT_GOLD: u32 = GOLD;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_unit_keeps_1080p_and_grows_in_proportion_past_it() {
        assert_eq!(unit_at(1280.0, 720.0, 1.0), 20.0);
        assert_eq!(unit_at(1920.0, 1080.0, 1.0), 28.0);
        assert!((unit_at(3840.0, 2160.0, 1.0) - 56.0).abs() < 1e-3);
        assert!((unit_at(2560.0, 1440.0, 1.0) - 28.0 * 40.0 / 30.0).abs() < 1e-3);
        assert_eq!(unit_at(640.0, 360.0, 1.0), 12.0);
        assert_eq!(unit_at(1920.0, 1080.0, 1.25), 35.0);
        // Grows without a jump as the window does.
        let mut last = unit_at(640.0, 360.0, 1.0);
        for h in (360..2400).step_by(8) {
            let u = unit_at(h as f32 * 16.0 / 9.0, h as f32, 1.0);
            assert!(u >= last && u - last < 1.0, "{h}: {last} to {u}");
            last = u;
        }
    }

    #[test]
    fn the_art_keeps_inside_its_region_and_apart() {
        for (i, a) in art::ALL.iter().enumerate() {
            assert!(a.x >= ICON_REGION.x, "{a:?}");
            assert!(a.x + a.w <= ICON_REGION.x + ICON_REGION.w && a.y + a.h <= ICON_REGION.y + ICON_REGION.h, "{a:?}");
            for b in &art::ALL[i + 1..] {
                assert!(a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y, "{a:?} and {b:?}");
            }
        }
        let px = art::bake();
        assert_eq!(px.len(), (ICON_REGION.w * ICON_REGION.h * 4) as usize);
        let alpha =
            |r: AtlasRegion, x: u32, y: u32| px[(((r.y - ICON_REGION.y + y) * ICON_REGION.w + r.x - ICON_REGION.x + x) * 4 + 3) as usize];
        // The glass: solid in the middle, clear in its cut corners.
        assert!(alpha(art::GLASS, 48, 48) > 200);
        assert_eq!(alpha(art::GLASS, 1, 1), 0);
        // The rule: on the edge, not in the middle.
        assert!(alpha(art::FRAME, 48, 1) > 150);
        assert_eq!(alpha(art::FRAME, 48, 48), 0);
        // The wings have feathers on both sides of the middle, none at its foot.
        assert!(alpha(art::WINGS, 70, 60) > 0 && alpha(art::WINGS, 186, 60) > 0);
        assert_eq!(alpha(art::WINGS, 128, 158), 0);
        assert!(alpha(art::DISC, 48, 48) == 255 && alpha(art::DISC, 1, 1) == 0);
    }

    #[test]
    fn a_panel_comes_and_goes_in_a_blink() {
        assert_eq!(appear(0.0, 5.0).0, 0.0);
        assert_eq!(appear(APPEAR, 5.0), (1.0, 0.0));
        assert!(appear(APPEAR * 0.5, 5.0).0 > 0.5);
        assert_eq!(appear(3.0, 0.0).0, 0.0);
    }
}
