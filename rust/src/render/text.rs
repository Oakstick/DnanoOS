//! The game's type (UI.md §4): two openly licensed families, drawn from a signed-distance-field
//! atlas made at start-up, so text is crisp at any size and outlines, shadows and glows come for
//! free in the overlay's shader.
//!
//! - **Body**: Alegreya Sans Medium, a humanist sans, for everything that is read.
//! - **Display**: Cinzel at its bold weight, a Roman display serif, for titles and names.
//!
//! Both are SIL Open Font Licence 1.1 (`assets/fonts/OFL-*.txt`), embedded with `include_bytes!`
//! (the web build has no file system). Each glyph is rasterised at `RASTER_EM` pixels to the em,
//! its distance to the outline worked out exactly (Felzenszwalb and Huttenlocher's transform, in
//! and out), halved to `SDF_EM`, and packed into one 8-bit texture. The metrics (advances, the
//! glyphs' boxes and the pairs' kerning from the fonts' GPOS tables) stay on the CPU for layout,
//! so `Overlay::measure_styled` agrees with what is drawn to the pixel.
//!
//! Sizes keep the old pixel font's meaning: a `size` of 20 draws capitals 14 px tall (`CAP`), so
//! every layout written for the old font still sits where it did.

use std::collections::HashMap;
use std::sync::OnceLock;

use ab_glyph::{Font, FontRef, PxScale, ScaleFont, VariableFont};

/// The type families.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Family {
    /// Alegreya Sans Medium: everything that is read.
    Body,
    /// Cinzel Bold: titles, names, the big numbers.
    Display,
}

impl Family {
    pub const ALL: [Family; 2] = [Family::Body, Family::Display];

    fn index(self) -> usize {
        self as usize
    }
}

/// Capitals stand this share of a `size` tall (as the old pixel font's did).
pub const CAP: f32 = 0.7;
/// Pixels to the em the glyphs are rasterised at, and stored at (half: the transform is worked
/// out at twice the resolution it is kept at, for clean curves).
const RASTER_EM: f32 = 96.0;
pub const SDF_EM: f32 = RASTER_EM / 2.0;
/// How far the field reaches past the outline, in stored pixels, each way: the widest outline or
/// glow is this share of an em (`SPREAD / SDF_EM`, an eighth).
pub const SPREAD: f32 = 6.0;
/// The atlas's width; its height is what the glyphs need, rounded up to a power of two.
pub const SDF_W: u32 = 1024;

/// The characters every face carries: printable ASCII, Latin-1, Latin Extended-A (the Turkish
/// names, the macrons of the Japanese ones, and room for any name to come), and the marks the
/// game's strings use. What a face lacks it borrows from the other; the heart is drawn.
fn charset() -> Vec<char> {
    let mut set: Vec<char> = (0x20u32..0x7f).chain(0xa0..0x180).filter_map(char::from_u32).collect();
    set.extend("–—‘’“”•…→←↑↓×·♥".chars());
    set.sort_unstable();
    set.dedup();
    set
}

/// Where a glyph is in the atlas, and the box it covers about the pen on the baseline.
#[derive(Clone, Copy, Debug, Default)]
pub struct Glyph {
    /// u0, v0, u1, v1 in the atlas (0..1).
    pub uv: [f32; 4],
    /// x0, y0, x1, y1 in ems from the pen on the baseline (y down), padding included.
    pub plane: [f32; 4],
    /// How far the pen moves, in ems.
    pub advance: f32,
    /// Whether it has anything to draw (a space has not).
    pub ink: bool,
}

/// One face's metrics.
pub struct Face {
    glyphs: HashMap<char, Glyph>,
    /// Kerning of each pair of printable ASCII characters, in ems (95 x 95).
    kern: Vec<f32>,
    /// Capital height, ascender and descender (negative), in ems.
    pub cap: f32,
    pub ascent: f32,
    pub descent: f32,
}

impl Face {
    /// The glyph for `c`, or the question mark's.
    pub fn glyph(&self, c: char) -> &Glyph {
        self.glyphs.get(&c).or_else(|| self.glyphs.get(&'?')).expect("the question mark is always there")
    }

    pub fn has(&self, c: char) -> bool {
        self.glyphs.contains_key(&c)
    }

    /// The kerning between `a` and `b` in ems (printable ASCII only; nothing for the rest).
    pub fn kern(&self, a: char, b: char) -> f32 {
        let (a, b) = (a as u32, b as u32);
        if (0x20..0x7f).contains(&a) && (0x20..0x7f).contains(&b) {
            self.kern[((a - 0x20) * 95 + (b - 0x20)) as usize]
        } else {
            0.0
        }
    }

    /// Pixels to the em for text of `size` (its capitals `CAP` of `size` tall).
    pub fn em(&self, size: f32) -> f32 {
        size * CAP / self.cap
    }
}

/// Both faces and the atlas they share.
pub struct Fonts {
    pub faces: [Face; 2],
    /// The field, one byte a texel (`SDF_W` by `height`): 128 is the outline, more is inside.
    pub pixels: Vec<u8>,
    pub height: u32,
    /// How long the atlas took to make, in ms.
    pub build_ms: f64,
}

impl Fonts {
    pub fn face(&self, f: Family) -> &Face {
        &self.faces[f.index()]
    }
}

static FONTS: OnceLock<Fonts> = OnceLock::new();

/// The faces and their atlas, made the first time they are wanted (by the renderer as it starts,
/// or the first text laid out).
pub fn fonts() -> &'static Fonts {
    FONTS.get_or_init(build)
}

pub(crate) const BODY_TTF: &[u8] = include_bytes!("../../assets/fonts/AlegreyaSans-Medium.ttf");
pub(crate) const DISPLAY_TTF: &[u8] = include_bytes!("../../assets/fonts/Cinzel-Variable.ttf");
/// Cinzel's weight axis is set to this (its bold).
const DISPLAY_WEIGHT: f32 = 700.0;

/// A glyph's field before packing: its stored size, its texels, and its metrics.
struct Raw {
    family: usize,
    c: char,
    w: usize,
    h: usize,
    px: Vec<u8>,
    plane: [f32; 4],
    advance: f32,
}

fn build() -> Fonts {
    let start = web_time::Instant::now();
    let mut body = FontRef::try_from_slice(BODY_TTF).expect("the body face parses");
    let mut display = FontRef::try_from_slice(DISPLAY_TTF).expect("the display face parses");
    display.set_variation(b"wght", DISPLAY_WEIGHT);
    // (The body face is static; a no-op, kept so both are made alike.)
    body.set_variation(b"wght", 500.0);
    let set = charset();
    let mut raws: Vec<Raw> = Vec::new();
    let mut faces = Vec::new();
    for (k, (font, ttf, weight)) in [(&body, BODY_TTF, None), (&display, DISPLAY_TTF, Some(DISPLAY_WEIGHT))].into_iter().enumerate() {
        let upem = font.units_per_em().unwrap_or(1000.0);
        let scale = PxScale::from(RASTER_EM * font.height_unscaled() / upem);
        let scaled = font.as_scaled(scale);
        let mut cap_face = ttf_parser::Face::parse(ttf, 0).expect("the face parses");
        if let Some(w) = weight {
            cap_face.set_variation(ttf_parser::Tag::from_bytes(b"wght"), w);
        }
        let cap = cap_face.capital_height().map(|c| c as f32 / upem).filter(|&c| c > 0.2).unwrap_or(0.7);
        for &c in &set {
            if c == '♥' {
                raws.push(heart(k));
                continue;
            }
            let id = font.glyph_id(c);
            if id.0 == 0 && c != ' ' {
                continue;
            }
            let advance = scaled.h_advance(id) / RASTER_EM;
            raws.push(rasterise(font, k, c, scale, advance));
        }
        faces.push(Face {
            glyphs: HashMap::new(),
            kern: kerning(&cap_face, upem),
            cap,
            ascent: font.ascent_unscaled() / upem,
            descent: font.descent_unscaled() / upem,
        });
    }
    // What one face lacks, the other lends (scaled to its own capitals).
    let (height, placed) = pack(&raws);
    let mut pixels = vec![0u8; (SDF_W * height) as usize];
    for (r, &(x, y)) in raws.iter().zip(&placed) {
        for row in 0..r.h {
            let at = (y as usize + row) * SDF_W as usize + x as usize;
            pixels[at..at + r.w].copy_from_slice(&r.px[row * r.w..(row + 1) * r.w]);
        }
        let (fw, fh) = (SDF_W as f32, height as f32);
        let g = Glyph {
            uv: [x as f32 / fw, y as f32 / fh, (x as f32 + r.w as f32) / fw, (y as f32 + r.h as f32) / fh],
            plane: r.plane,
            advance: r.advance,
            ink: r.w > 0,
        };
        faces[r.family].glyphs.insert(r.c, g);
    }
    for c in set {
        for k in 0..2 {
            if !faces[k].glyphs.contains_key(&c) {
                if let Some(&g) = faces[1 - k].glyphs.get(&c) {
                    let s = faces[k].cap / faces[1 - k].cap;
                    let g = Glyph { plane: g.plane.map(|p| p * s), advance: g.advance * s, ..g };
                    faces[k].glyphs.insert(c, g);
                }
            }
        }
    }
    let [b, d]: [Face; 2] = faces.try_into().ok().expect("two faces");
    Fonts { faces: [b, d], pixels, height, build_ms: start.elapsed().as_secs_f64() * 1e3 }
}

/// Shelf-packs the glyphs (tallest first) into `SDF_W` columns; the height used (a power of two)
/// and each glyph's corner, in the order given.
fn pack(raws: &[Raw]) -> (u32, Vec<(u32, u32)>) {
    let mut order: Vec<usize> = (0..raws.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(raws[i].h));
    let mut placed = vec![(0u32, 0u32); raws.len()];
    let (mut x, mut y, mut shelf) = (0u32, 0u32, 0u32);
    for i in order {
        let (w, h) = (raws[i].w as u32, raws[i].h as u32);
        if w == 0 {
            continue;
        }
        if x + w > SDF_W {
            x = 0;
            y += shelf + 1;
            shelf = 0;
        }
        placed[i] = (x, y);
        x += w + 1;
        shelf = shelf.max(h);
    }
    ((y + shelf).next_power_of_two().max(64), placed)
}

/// The field of one glyph of `font`.
fn rasterise(font: &FontRef, family: usize, c: char, scale: PxScale, advance: f32) -> Raw {
    let glyph = font.glyph_id(c).with_scale_and_position(scale, ab_glyph::point(0.0, 0.0));
    let Some(outline) = font.outline_glyph(glyph) else {
        return Raw { family, c, w: 0, h: 0, px: Vec::new(), plane: [0.0; 4], advance };
    };
    let b = outline.px_bounds();
    let pad = (SPREAD * 2.0) as i32;
    let (bw, bh) = (b.width() as i32, b.height() as i32);
    // Even, so it halves cleanly.
    let w = ((bw + 2 * pad + 1) / 2 * 2) as usize;
    let h = ((bh + 2 * pad + 1) / 2 * 2) as usize;
    let mut cov = vec![0.0f32; w * h];
    outline.draw(|x, y, v| {
        let (x, y) = (x as i32 + pad, y as i32 + pad);
        if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
            cov[y as usize * w + x as usize] = v.min(1.0);
        }
    });
    let px = field(&cov, w, h);
    let plane = [
        (b.min.x - pad as f32) / RASTER_EM,
        (b.min.y - pad as f32) / RASTER_EM,
        (b.min.x - pad as f32 + w as f32) / RASTER_EM,
        (b.min.y - pad as f32 + h as f32) / RASTER_EM,
    ];
    Raw { family, c, w: w / 2, h: h / 2, px, plane, advance }
}

/// The heart, drawn (neither face has one): two lobes and a point, capital-high.
fn heart(family: usize) -> Raw {
    let pad = SPREAD * 2.0;
    let (gw, gh) = (0.78 * RASTER_EM, 0.7 * RASTER_EM);
    let w = ((gw + 2.0 * pad) as usize).div_ceil(2) * 2;
    let h = ((gh + 2.0 * pad) as usize).div_ceil(2) * 2;
    let mut cov = vec![0.0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            // Four samples a pixel of the curve (x² + y² - 1)³ - x² y³ <= 0.
            let mut n = 0;
            for (sx, sy) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
                let u = ((x as f32 + sx - pad) / gw - 0.5) * 2.6;
                let v = -(((y as f32 + sy - pad) / gh) * 2.3 - 1.25);
                let a = u * u + v * v - 1.0;
                if a * a * a - u * u * v * v * v <= 0.0 {
                    n += 1;
                }
            }
            cov[y * w + x] = n as f32 / 4.0;
        }
    }
    let px = field(&cov, w, h);
    let top = -0.7 - pad / RASTER_EM;
    let plane = [0.06 - pad / RASTER_EM, top, 0.06 - pad / RASTER_EM + w as f32 / RASTER_EM, top + h as f32 / RASTER_EM];
    Raw { family, c: '♥', w: w / 2, h: h / 2, px, plane, advance: 0.9 }
}

/// The signed distance field of a coverage image (`w` by `h`, both even), halved and stored as
/// bytes: 128 on the outline, rising inside, `SPREAD` stored pixels to 0 and 255.
fn field(cov: &[f32], w: usize, h: usize) -> Vec<u8> {
    const FAR: f32 = 1e20;
    let inside: Vec<bool> = cov.iter().map(|&c| c >= 0.5).collect();
    let mut to_in: Vec<f32> = inside.iter().map(|&i| if i { 0.0 } else { FAR }).collect();
    let mut to_out: Vec<f32> = inside.iter().map(|&i| if i { FAR } else { 0.0 }).collect();
    edt(&mut to_in, w, h);
    edt(&mut to_out, w, h);
    let sd: Vec<f32> = (0..w * h)
        .map(|i| {
            let c = cov[i];
            if c > 0.02 && c < 0.98 {
                // At the edge the coverage says where the outline crosses better than the grid can.
                0.5 - c
            } else if inside[i] {
                -(to_out[i].sqrt() - 0.5)
            } else {
                to_in[i].sqrt() - 0.5
            }
        })
        .collect();
    let (hw, hh) = (w / 2, h / 2);
    let mut out = vec![0u8; hw * hh];
    for y in 0..hh {
        for x in 0..hw {
            let i = 2 * y * w + 2 * x;
            // Raster pixels to stored ones: half.
            let d = (sd[i] + sd[i + 1] + sd[i + w] + sd[i + w + 1]) / 4.0 / 2.0;
            let v = 0.5 - d / (2.0 * SPREAD);
            out[y * hw + x] = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    out
}

/// Squared Euclidean distance transform in place (Felzenszwalb and Huttenlocher): each texel
/// becomes its squared distance to the nearest texel that was 0.
fn edt(grid: &mut [f32], w: usize, h: usize) {
    let n = w.max(h);
    let (mut f, mut d, mut v, mut z) = (vec![0.0f32; n], vec![0.0f32; n], vec![0usize; n], vec![0.0f32; n + 1]);
    for x in 0..w {
        for y in 0..h {
            f[y] = grid[y * w + x];
        }
        edt_1d(&f[..h], &mut d[..h], &mut v, &mut z);
        for y in 0..h {
            grid[y * w + x] = d[y];
        }
    }
    for y in 0..h {
        f[..w].copy_from_slice(&grid[y * w..(y + 1) * w]);
        edt_1d(&f[..w], &mut d[..w], &mut v, &mut z);
        grid[y * w..(y + 1) * w].copy_from_slice(&d[..w]);
    }
}

fn edt_1d(f: &[f32], d: &mut [f32], v: &mut [usize], z: &mut [f32]) {
    let n = f.len();
    if n == 0 {
        return;
    }
    let mut k = 0usize;
    v[0] = 0;
    z[0] = f32::NEG_INFINITY;
    z[1] = f32::INFINITY;
    let sq = |q: usize| (q * q) as f32;
    for q in 1..n {
        let mut s = ((f[q] + sq(q)) - (f[v[k]] + sq(v[k]))) / (2.0 * q as f32 - 2.0 * v[k] as f32);
        while s <= z[k] {
            k -= 1;
            s = ((f[q] + sq(q)) - (f[v[k]] + sq(v[k]))) / (2.0 * q as f32 - 2.0 * v[k] as f32);
        }
        k += 1;
        v[k] = q;
        z[k] = s;
        z[k + 1] = f32::INFINITY;
    }
    k = 0;
    for (q, out) in d.iter_mut().enumerate().take(n) {
        while z[k + 1] < q as f32 {
            k += 1;
        }
        let dq = q as f32 - v[k] as f32;
        *out = dq * dq + f[v[k]];
    }
}

/// The face's pair kerning for printable ASCII, from the GPOS table's `kern` feature, in ems.
fn kerning(face: &ttf_parser::Face, upem: f32) -> Vec<f32> {
    use ttf_parser::gpos::{PairAdjustment, PositioningSubtable};
    let mut out = vec![0.0f32; 95 * 95];
    let Some(gpos) = face.tables().gpos else { return out };
    let ids: Vec<Option<ttf_parser::GlyphId>> = (0x20u8..0x7f).map(|c| face.glyph_index(c as char)).collect();
    let kern = ttf_parser::Tag::from_bytes(b"kern");
    let mut lookups: Vec<u16> = gpos.features.into_iter().filter(|f| f.tag == kern).flat_map(|f| f.lookup_indices.into_iter()).collect();
    lookups.sort_unstable();
    lookups.dedup();
    for li in lookups {
        let Some(lookup) = gpos.lookups.get(li) else { continue };
        for sub in lookup.subtables.into_iter::<PositioningSubtable>() {
            let PositioningSubtable::Pair(pair) = sub else { continue };
            for (a, first) in ids.iter().enumerate() {
                let Some(first) = *first else { continue };
                let Some(cov) = pair.coverage().get(first) else { continue };
                for (b, second) in ids.iter().enumerate() {
                    let Some(second) = *second else { continue };
                    let slot = &mut out[a * 95 + b];
                    if *slot != 0.0 {
                        // The first subtable to cover a pair decides it.
                        continue;
                    }
                    let adj = match pair {
                        PairAdjustment::Format1 { sets, .. } => sets.get(cov).and_then(|s| s.get(second)),
                        PairAdjustment::Format2 { classes, matrix, .. } => matrix.get((classes.0.get(first), classes.1.get(second))),
                    };
                    if let Some((r, _)) = adj {
                        *slot = r.x_advance as f32 / upem;
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_transform_measures_distance() {
        // One point lit in a row of seven: the squared distances fan out from it.
        let mut g = vec![1e20f32; 7];
        g[3] = 0.0;
        edt(&mut g, 7, 1);
        assert_eq!(g, vec![9.0, 4.0, 1.0, 0.0, 1.0, 4.0, 9.0]);
        let mut g = vec![1e20f32; 9];
        g[0] = 0.0;
        edt(&mut g, 3, 3);
        assert_eq!(g[8], 8.0, "the far corner of a 3x3, diagonally");
    }

    #[test]
    fn both_faces_carry_every_letter_with_ink() {
        let f = fonts();
        for face in &f.faces {
            let set = charset();
            let missing: Vec<char> = set.iter().copied().filter(|&c| !face.has(c)).collect();
            // (Only the odd obsolete letter, such as the apostrophe-n, may be wanting.)
            assert!(missing.len() <= 3, "missing {missing:?}");
            assert!((0x20u8..0x7f).all(|c| face.has(c as char)));
            assert!(face.glyph('A').ink && !face.glyph(' ').ink);
            assert!(face.glyph('W').advance > face.glyph('i').advance, "proportional");
            assert!(face.cap > 0.5 && face.cap < 0.8, "capitals {}", face.cap);
        }
        assert!(f.height <= 2048, "the atlas fits a WebGL2 texture: {}", f.height);
        eprintln!("the type's atlas: {} x {}, made in {:.1} ms", SDF_W, f.height, f.build_ms);
    }

    #[test]
    fn the_atlas_holds_a_field() {
        let f = fonts();
        let g = f.face(Family::Body).glyph('O');
        let (x0, y0) = ((g.uv[0] * SDF_W as f32) as usize, (g.uv[1] * f.height as f32) as usize);
        let (x1, y1) = ((g.uv[2] * SDF_W as f32) as usize, (g.uv[3] * f.height as f32) as usize);
        let at = |x: usize, y: usize| f.pixels[y * SDF_W as usize + x];
        // Its corner is far outside; its ring is inside; its middle is the counter, outside again.
        assert!(at(x0, y0) < 20);
        // The corners of its tight box are outside its round, clearly (no faint coverage there).
        let pad = SPREAD as usize;
        for (x, y) in [(x0 + pad, y0 + pad), (x1 - 1 - pad, y0 + pad), (x0 + pad, y1 - 1 - pad), (x1 - 1 - pad, y1 - 1 - pad)] {
            assert!(at(x, y) < 110, "a corner of the O's box: {}", at(x, y));
        }
        let mid = (y0 + y1) / 2;
        let ring = (x0..x1).map(|x| at(x, mid)).max().unwrap();
        assert!(ring > 150, "the stroke is inside: {ring}");
        assert!(at((x0 + x1) / 2, mid) < 128, "the counter is open");
    }

    /// Every character in the game's string literals (names with their accents included).
    #[test]
    fn every_letter_the_game_writes_is_in_both_faces() {
        fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    out.push(p);
                }
            }
        }
        let mut files = Vec::new();
        walk(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
        let mut used = std::collections::BTreeSet::new();
        for f in files {
            let src = std::fs::read_to_string(&f).unwrap();
            for line in src.lines() {
                let code = line.trim_start();
                if code.starts_with("//") || code.starts_with('*') || code.starts_with("/*") {
                    continue;
                }
                // The insides of the line's string literals.
                let mut inside = false;
                let mut escaped = false;
                for c in line.chars() {
                    if inside {
                        if escaped {
                            escaped = false;
                        } else if c == '\\' {
                            escaped = true;
                        } else if c == '"' {
                            inside = false;
                        } else if !c.is_control() {
                            used.insert(c);
                        }
                    } else if c == '"' {
                        inside = true;
                    }
                }
            }
        }
        assert!(used.contains(&'Ş') && used.contains(&'ō'), "the scan finds the names");
        let fonts = fonts();
        for face in &fonts.faces {
            let missing: String = used.iter().filter(|&&c| !face.has(c)).collect();
            assert!(missing.is_empty(), "missing {missing}");
        }
    }

    #[test]
    fn pairs_kern() {
        let f = fonts();
        let body = f.face(Family::Body);
        assert!(body.kern('A', 'V') < 0.0 || body.kern('T', 'o') < 0.0, "the classic pairs tuck in");
        assert_eq!(body.kern('é', 'A'), 0.0);
    }
}
