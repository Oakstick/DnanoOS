//! 2D overlay batcher: rectangles, lines, arcs, text in the game's type (`text`), textured quads
//! from the UI atlas (pictures, portraits, the minimap's level, 9-slice panels), in logical
//! pixels, blended in sRGB like a canvas.
//!
//! Every vertex says how its texel is read (`OVertex::extra`, the `FLAG_*` bits): straight from
//! the RGBA atlas (solids, the pictures, the old pixel font), as a signed distance field from the
//! type's atlas (text, with its outline, shadow and glow passes), premultiplied (the portraits the
//! renderer draws), and masked to a circle (the minimap).
//!
//! The RGBA atlas is `ATLAS_W` by `ATLAS_H`. Its corner (`LEGACY`) keeps the old pixel font's
//! cells, the solid texel and the pictures; the rest is set out in regions for the builders to
//! come: `ICON_REGION` (redrawn icons), `MAP_REGION` (the minimap's level and fog) and the portrait
//! slots (`portrait_slot`), each written on the GPU (`Gpu::write_atlas`, `Gpu::render_portraits`).

use bytemuck::{Pod, Zeroable};

use super::text::{fonts, Family, CAP, SDF_EM, SDF_W, SPREAD};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct OVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    /// `[flags, a, b, c]`: how the texel is read (`FLAG_*`), and its parameters: for a field, the
    /// edge's shift (positive grows the shape) and the softness, both in field units; for the
    /// circle mask, the vertex's place in the circle's square (-1..1, -1..1).
    pub extra: [f32; 4],
}

/// The texel is a signed distance field from the type's atlas.
pub const FLAG_SDF: u32 = 1;
/// Masked to the circle inscribed in the quad (`extra[1..3]` run -1..1 over it).
pub const FLAG_CIRCLE: u32 = 2;
/// The texel is premultiplied by its alpha (the renderer's portraits).
pub const FLAG_PREMULTIPLIED: u32 = 4;

pub const ATLAS_W: u32 = 2048;
pub const ATLAS_H: u32 = 2048;
/// The old atlas, kept in the new one's corner: the pixel font's cells, the solid texel, the pictures.
pub const LEGACY: u32 = 512;
const CELL: u32 = 32;
const COLS: u32 = 16;
/// Glyph cell used for solid fills (a fully set block).
const SOLID: u32 = 0;
/// Glyph cell holding the heart.
const HEART: u32 = 127;

const HEART_BITS: [u8; 8] = [0b0011_0110, 0b0111_1111, 0b0111_1111, 0b0111_1111, 0b0011_1110, 0b0001_1100, 0b0000_1000, 0b0000_0000];

/// Letters past ASCII, in the atlas from cell 128 on: Turkish's, and the long o of Japanese names
/// (the later stages' foes are the Tülükuş, the Arçura, Tepegöz, the Goryō...).
const EXTRA: [char; 12] = ['Ç', 'ç', 'Ö', 'ö', 'Ü', 'ü', 'Ş', 'ş', 'İ', 'ı', 'Ō', 'ō'];
const EXTRA_CELL: u32 = 128;

/// A rectangle of the RGBA atlas, in texels.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AtlasRegion {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl AtlasRegion {
    pub const fn new(x: u32, y: u32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }

    /// Its corners in the atlas's 0..1 (u0, v0, u1, v1), half a texel in from its edges so the
    /// filter never reaches its neighbour.
    pub fn uv(self) -> [f32; 4] {
        let (aw, ah) = (ATLAS_W as f32, ATLAS_H as f32);
        [
            (self.x as f32 + 0.5) / aw,
            (self.y as f32 + 0.5) / ah,
            ((self.x + self.w) as f32 - 0.5) / aw,
            ((self.y + self.h) as f32 - 0.5) / ah,
        ]
    }

    /// A part of it, in texels from its own corner.
    pub fn sub(self, x: u32, y: u32, w: u32, h: u32) -> Self {
        Self { x: self.x + x, y: self.y + y, w: w.min(self.w.saturating_sub(x)), h: h.min(self.h.saturating_sub(y)) }
    }
}

/// Free for the HUD's redrawn icons (UI.md §4), written with `Gpu::write_atlas`.
pub const ICON_REGION: AtlasRegion = AtlasRegion::new(512, 0, 512, 512);
/// Free for the minimap's level and its fog (UI.md §7), written with `Gpu::write_atlas`.
pub const MAP_REGION: AtlasRegion = AtlasRegion::new(1024, 0, 1024, 1024);
/// The portraits' slots: `PORTRAIT_SLOTS` squares `PORTRAIT_SIZE` across, from row 1024 down.
pub const PORTRAIT_SIZE: u32 = 256;
pub const PORTRAIT_SLOTS: usize = 32;

/// Portrait slot `k`'s square in the atlas.
pub const fn portrait_slot(k: usize) -> AtlasRegion {
    let per_row = (ATLAS_W / PORTRAIT_SIZE) as usize;
    AtlasRegion::new((k % per_row) as u32 * PORTRAIT_SIZE, 1024 + (k / per_row) as u32 * PORTRAIT_SIZE, PORTRAIT_SIZE, PORTRAIT_SIZE)
}

/// An extra letter's glyph: font8x8's own where it has one, else its base letter with a cedilla
/// under it, its dot taken off or put on, or a bar over it.
fn extra_glyph(c: char) -> [u8; 8] {
    use font8x8::legacy::{BASIC_LEGACY as BASIC, LATIN_LEGACY as LATIN};
    let latin = |c: char| LATIN[c as usize - 0xa0];
    let with = |base: char, row: usize, bits: u8| {
        let mut g = BASIC[base as usize];
        g[row] = bits;
        g
    };
    match c {
        'Ç' | 'ç' | 'Ö' | 'ö' | 'Ü' | 'ü' => latin(c),
        'Ş' => with('S', 7, 0x0c),
        'ş' => with('s', 7, 0x0c),
        'İ' => [0x0c, 0x00, 0x1e, 0x0c, 0x0c, 0x0c, 0x1e, 0x00],
        'ı' => with('i', 0, 0x00),
        'Ō' => [0x3e, 0x00, 0x1c, 0x36, 0x63, 0x63, 0x36, 0x1c],
        'ō' => with('o', 0, 0x1e),
        _ => BASIC['?' as usize],
    }
}

/// A picture kept in the atlas under the glyphs: a screenshot for the camera choice.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Picture {
    /// The Garden seen from above.
    ViewTop,
    /// The Garden over Ragnar's shoulder.
    ViewFollow,
}

/// Every picture's size in texels, and the atlas row they start on (below the last glyph cell).
const PICTURE_W: u32 = 256;
const PICTURE_H: u32 = 144;
const PICTURE_Y: u32 = 288;

impl Picture {
    const ALL: [Picture; 2] = [Picture::ViewTop, Picture::ViewFollow];

    /// Its place in the atlas.
    pub fn region(self) -> AtlasRegion {
        match self {
            Picture::ViewTop => AtlasRegion::new(0, PICTURE_Y, PICTURE_W, PICTURE_H),
            Picture::ViewFollow => AtlasRegion::new(PICTURE_W, PICTURE_Y, PICTURE_W, PICTURE_H),
        }
    }

    /// Its texels, raw RGB rows (taken with `--capture --no-hud` and boxed down to size).
    fn rgb(self) -> &'static [u8] {
        match self {
            Picture::ViewTop => include_bytes!("../../assets/ui/view_top.rgb"),
            Picture::ViewFollow => include_bytes!("../../assets/ui/view_follow.rgb"),
        }
    }
}

/// The atlas's corner (`LEGACY` square), four bytes a texel: every ASCII glyph of the public-domain
/// font8x8, magnified 4x, in white with its coverage as alpha, then the extra letters, then the
/// pictures. (The rest of the atlas starts clear and is written on the GPU.)
pub fn build_atlas() -> Vec<u8> {
    let mut px = vec![0u8; (LEGACY * LEGACY) as usize];
    let scale = CELL / 8;
    let put = |px: &mut Vec<u8>, cell: u32, bits: &[u8; 8]| {
        let cx = (cell % COLS) * CELL;
        let cy = (cell / COLS) * CELL;
        for (row, byte) in bits.iter().enumerate() {
            for col in 0..8u32 {
                if byte & (1 << col) == 0 {
                    continue;
                }
                for dy in 0..scale {
                    for dx in 0..scale {
                        let x = cx + col * scale + dx;
                        let y = cy + row as u32 * scale + dy;
                        px[(y * LEGACY + x) as usize] = 255;
                    }
                }
            }
        }
    };
    for (i, glyph) in font8x8::legacy::BASIC_LEGACY.iter().enumerate() {
        if i as u32 == SOLID || i as u32 == HEART {
            continue;
        }
        put(&mut px, i as u32, glyph);
    }
    put(&mut px, HEART, &HEART_BITS);
    put(&mut px, SOLID, &[0xff; 8]);
    for (i, &c) in EXTRA.iter().enumerate() {
        put(&mut px, EXTRA_CELL + i as u32, &extra_glyph(c));
    }
    // The glyphs white, their coverage as alpha; the pictures opaque.
    let mut out: Vec<u8> = px.iter().flat_map(|&a| [255, 255, 255, a]).collect();
    for p in Picture::ALL {
        let r = p.region();
        for (k, [cr, cg, cb]) in p.rgb().as_chunks::<3>().0.iter().enumerate() {
            let (x, y) = (r.x + k as u32 % PICTURE_W, r.y + k as u32 / PICTURE_W);
            let i = ((y * LEGACY + x) * 4) as usize;
            out[i..i + 4].copy_from_slice(&[*cr, *cg, *cb, 255]);
        }
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Align {
    Left,
    Center,
    Right,
}

pub fn rgba(hex: u32, alpha: f32) -> [f32; 4] {
    [((hex >> 16) & 255) as f32 / 255.0, ((hex >> 8) & 255) as f32 / 255.0, (hex & 255) as f32 / 255.0, alpha]
}

/// A soft copy of text drawn behind it: offset, blurred, in its own colour (a drop shadow, or with
/// no offset a glow).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Shadow {
    pub dx: f32,
    pub dy: f32,
    /// How far it softens, in pixels (up to an eighth of the em; more is clipped).
    pub blur: f32,
    pub color: [f32; 4],
}

/**
 * How a run of text is drawn (UI.md §4). `size` keeps the old pixel font's meaning (capitals 0.7
 * of it tall) and `y` is the middle of the capitals, as the old `text` placed them; text is laid
 * out by the faces' real advances and kerning.
 *
 * ```ignore
 * let title = Style::display(t * 1.3, rgba(GOLD, 1.0)).centered().shadow(2.0, 3.0);
 * o.text_styled("THE LEDGER", w / 2.0, y, &title);
 * let body = Style::body(t * 0.6, rgba(TEXT, 1.0));
 * let below = o.wrap_styled(&line, x, y, width, t * 0.8, &body);
 * ```
 */
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Style {
    pub family: Family,
    pub size: f32,
    pub color: [f32; 4],
    pub align: Align,
    /// -1 (thin) .. 0 (the face's own) .. 1 (heavy), by moving the field's edge.
    pub weight: f32,
    /// An outline this many pixels wide (up to an eighth of the em), in its own colour.
    pub outline: Option<(f32, [f32; 4])>,
    pub shadow: Option<Shadow>,
    /// Extra space between letters, in ems (spaced capitals for titles).
    pub tracking: f32,
    /// Figures all as wide as the widest, so counting numbers do not jitter.
    pub tabular: bool,
}

impl Style {
    pub fn new(family: Family, size: f32, color: [f32; 4]) -> Self {
        Self { family, size, color, align: Align::Left, weight: 0.0, outline: None, shadow: None, tracking: 0.0, tabular: false }
    }

    /// The humanist sans, for what is read.
    pub fn body(size: f32, color: [f32; 4]) -> Self {
        Self::new(Family::Body, size, color)
    }

    /// The display serif, for titles and names.
    pub fn display(size: f32, color: [f32; 4]) -> Self {
        Self::new(Family::Display, size, color)
    }

    pub fn align(self, align: Align) -> Self {
        Self { align, ..self }
    }

    pub fn centered(self) -> Self {
        self.align(Align::Center)
    }

    pub fn right(self) -> Self {
        self.align(Align::Right)
    }

    pub fn weight(self, weight: f32) -> Self {
        Self { weight: weight.clamp(-1.0, 1.0), ..self }
    }

    pub fn bold(self) -> Self {
        self.weight(0.6)
    }

    pub fn color(self, color: [f32; 4]) -> Self {
        Self { color, ..self }
    }

    pub fn size(self, size: f32) -> Self {
        Self { size, ..self }
    }

    pub fn outline(self, width: f32, color: [f32; 4]) -> Self {
        Self { outline: Some((width, color)), ..self }
    }

    /// A dark drop shadow `offset` down and right, softened by `blur` pixels.
    pub fn shadow(self, offset: f32, blur: f32) -> Self {
        Self { shadow: Some(Shadow { dx: offset, dy: offset, blur, color: [0.0, 0.0, 0.0, 0.8] }), ..self }
    }

    /// A glow of `color` round the letters, `radius` pixels.
    pub fn glow(self, radius: f32, color: [f32; 4]) -> Self {
        Self { shadow: Some(Shadow { dx: 0.0, dy: 0.0, blur: radius, color }), ..self }
    }

    pub fn tracking(self, ems: f32) -> Self {
        Self { tracking: ems, ..self }
    }

    pub fn tabular(self) -> Self {
        Self { tabular: true, ..self }
    }
}

pub struct Overlay {
    pub verts: Vec<OVertex>,
    pub width: f32,
    pub height: f32,
    /// Multiplies every alpha, like canvas globalAlpha.
    pub alpha: f32,
    /// Drop-shadow offset for text drawn with the old `text` in pixels; 0 draws none.
    pub shadow: f32,
    /// The portraits the HUD drew this frame, for the renderer to make (`portrait`).
    pub portraits: super::portrait::PortraitBook,
    /// The minimap's image of the level, kept up to date as the HUD draws and sent to
    /// `MAP_REGION` by the app (`minimap::MapLayer::upload`).
    pub map: super::minimap::MapLayer,
}

/// Share of the font size a glyph cell is tall; keeps capitals near a system font's cap height.
const GLYPH: f32 = 0.8;
/// Glyphs are drawn condensed, so lines of pixel text run about as long as proportional text.
const CONDENSE: f32 = 0.78;
/// The old `text`'s bold lines this many times the HUD's unit or more are titles: the display
/// serif. (The unit is the height over 36, as `hud::tile_for` makes it on a wide screen.)
const DISPLAY_FROM: f32 = 0.85;

impl Overlay {
    pub fn new() -> Self {
        Self {
            verts: Vec::with_capacity(16_384),
            width: 1.0,
            height: 1.0,
            alpha: 1.0,
            shadow: 0.0,
            portraits: Default::default(),
            map: Default::default(),
        }
    }

    pub fn begin(&mut self, width: f32, height: f32) {
        self.verts.clear();
        self.width = width;
        self.height = height;
        self.alpha = 1.0;
        self.shadow = 0.0;
        self.portraits.begin_frame();
    }

    /// Where the atlas's solid texel is, for a shape drawn with `raw_triangle`.
    pub fn solid_uv() -> [f32; 2] {
        let c = (SOLID % COLS) * CELL + CELL / 2;
        let r = (SOLID / COLS) * CELL + CELL / 2;
        [c as f32 / ATLAS_W as f32, r as f32 / ATLAS_H as f32]
    }

    fn tri(&mut self, a: [f32; 2], b: [f32; 2], c: [f32; 2], color: [f32; 4]) {
        let uv = Self::solid_uv();
        let color = [color[0], color[1], color[2], color[3] * self.alpha];
        for p in [a, b, c] {
            self.verts.push(OVertex { pos: p, uv, color, extra: [0.0; 4] });
        }
    }

    fn quad(&mut self, p: [[f32; 2]; 4], uv: [[f32; 2]; 4], color: [f32; 4]) {
        self.quad_ex(p, uv, color, [[0.0; 4]; 4]);
    }

    fn quad_ex(&mut self, p: [[f32; 2]; 4], uv: [[f32; 2]; 4], color: [f32; 4], extra: [[f32; 4]; 4]) {
        let color = [color[0], color[1], color[2], color[3] * self.alpha];
        for i in [0, 1, 2, 0, 2, 3] {
            self.verts.push(OVertex { pos: p[i], uv: uv[i], color, extra: extra[i] });
        }
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let uv = Self::solid_uv();
        self.quad([[x, y], [x + w, y], [x + w, y + h], [x, y + h]], [uv; 4], color);
    }

    /// A picture from the atlas, stretched over `w` by `h`.
    pub fn picture(&mut self, p: Picture, x: f32, y: f32, w: f32, h: f32, alpha: f32) {
        self.region(p.region(), x, y, w, h, [1.0, 1.0, 1.0, alpha]);
    }

    // ------------------------------------------------------------ textured quads

    /**
     * The low-level textured quad: four corners on screen (clockwise from the top left), their
     * places in the atlas (0..1), a tint, and how the texel is read (`FLAG_*`). With
     * `FLAG_CIRCLE` the quad is masked to the circle inscribed in it (an ellipse if it is not
     * square), feathered by a pixel.
     */
    pub fn textured_quad(&mut self, corners: [[f32; 2]; 4], uvs: [[f32; 2]; 4], tint: [f32; 4], flags: u32) {
        let f = flags as f32;
        let local = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
        let extra = local.map(|l| [f, l[0], l[1], 0.0]);
        self.quad_ex(corners, uvs, tint, extra);
    }

    /// An atlas region over the rectangle `x, y, w, h`, tinted.
    pub fn region(&mut self, r: AtlasRegion, x: f32, y: f32, w: f32, h: f32, tint: [f32; 4]) {
        let [u0, v0, u1, v1] = r.uv();
        self.textured_quad([[x, y], [x + w, y], [x + w, y + h], [x, y + h]], [[u0, v0], [u1, v0], [u1, v1], [u0, v1]], tint, 0);
    }

    /// An atlas region `w` by `h` centred on `cx, cy`, turned `angle` radians clockwise (y down).
    pub fn region_rotated(&mut self, r: AtlasRegion, cx: f32, cy: f32, w: f32, h: f32, angle: f32, tint: [f32; 4]) {
        let [u0, v0, u1, v1] = r.uv();
        let (s, c) = angle.sin_cos();
        let corner = |dx: f32, dy: f32| [cx + dx * c - dy * s, cy + dx * s + dy * c];
        let (hw, hh) = (w / 2.0, h / 2.0);
        self.textured_quad(
            [corner(-hw, -hh), corner(hw, -hh), corner(hw, hh), corner(-hw, hh)],
            [[u0, v0], [u1, v0], [u1, v1], [u0, v1]],
            tint,
            0,
        );
    }

    /**
     * A round window of `radius` pixels at `cx, cy` onto the atlas (the minimap): it shows the
     * square of the atlas centred on `uv` (0..1) with `half` its half-width in uv, turned `angle`
     * radians (the view turns clockwise as `angle` grows: north-up is 0). Keep what it shows
     * inside your region: past its edge it reads the neighbours.
     */
    pub fn region_circle(&mut self, uv: [f32; 2], half: f32, angle: f32, cx: f32, cy: f32, radius: f32, tint: [f32; 4]) {
        let (s, c) = (-angle).sin_cos();
        let at = |dx: f32, dy: f32| [uv[0] + (dx * c - dy * s) * half, uv[1] + (dx * s + dy * c) * half];
        let r = radius;
        self.textured_quad(
            [[cx - r, cy - r], [cx + r, cy - r], [cx + r, cy + r], [cx - r, cy + r]],
            [at(-1.0, -1.0), at(1.0, -1.0), at(1.0, 1.0), at(-1.0, 1.0)],
            tint,
            FLAG_CIRCLE,
        );
    }

    /**
     * A 9-slice panel from an atlas region whose corners are `border` texels square: the corners
     * drawn `edge` pixels square, the edges stretched along, the middle filled (UI.md §4: any size
     * from one picture).
     */
    #[allow(clippy::too_many_arguments)]
    pub fn nine_slice(&mut self, r: AtlasRegion, border: u32, x: f32, y: f32, w: f32, h: f32, edge: f32, tint: [f32; 4]) {
        let b = border.min(r.w / 2).min(r.h / 2);
        let e = edge.min(w / 2.0).min(h / 2.0);
        let xs = [x, x + e, x + w - e, x + w];
        let ys = [y, y + e, y + h - e, y + h];
        let cols = [(0, b), (b, r.w - 2 * b), (r.w - b, b)];
        let rows = [(0, b), (b, r.h - 2 * b), (r.h - b, b)];
        for (j, &(ty, th)) in rows.iter().enumerate() {
            for (i, &(tx, tw)) in cols.iter().enumerate() {
                if tw == 0 || th == 0 {
                    continue;
                }
                let part = r.sub(tx, ty, tw, th);
                self.region(part, xs[i], ys[j], xs[i + 1] - xs[i], ys[j + 1] - ys[j], tint);
            }
        }
    }

    /**
     * A portrait made by the renderer (UI.md §6), drawn over `x, y, w, h`: asks for it (made this
     * frame if it is new or due, see `portrait::PortraitBook`) and draws its slot, tinted (white
     * for its own colours). Masked round with `round`.
     */
    #[allow(clippy::too_many_arguments)]
    pub fn portrait(
        &mut self,
        id: super::portrait::PortraitId,
        pose: super::portrait::Pose,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        round: bool,
        tint: [f32; 4],
    ) {
        let px = w.max(h).round() as u32;
        let Some((slot, size)) = self.portraits.request(id, pose, px) else { return };
        let [u0, v0, u1, v1] = portrait_slot(slot).sub(0, 0, size, size).uv();
        let flags = FLAG_PREMULTIPLIED | if round { FLAG_CIRCLE } else { 0 };
        self.textured_quad([[x, y], [x + w, y], [x + w, y + h], [x, y + h]], [[u0, v0], [u1, v0], [u1, v1], [u0, v1]], tint, flags);
    }

    // ------------------------------------------------------------ shapes

    pub fn rect_outline(&mut self, x: f32, y: f32, w: f32, h: f32, lw: f32, color: [f32; 4]) {
        let hw = lw / 2.0;
        self.rect(x - hw, y - hw, w + lw, lw, color);
        self.rect(x - hw, y + h - hw, w + lw, lw, color);
        self.rect(x - hw, y + hw, lw, h - lw, color);
        self.rect(x + w - hw, y + hw, lw, h - lw, color);
    }

    /// A thick segment with round-ish caps.
    pub fn line(&mut self, a: [f32; 2], b: [f32; 2], width: f32, color: [f32; 4]) {
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len = (dx * dx + dy * dy).sqrt().max(1e-4);
        let (nx, ny) = (-dy / len * width / 2.0, dx / len * width / 2.0);
        let uv = Self::solid_uv();
        self.quad([[a[0] + nx, a[1] + ny], [b[0] + nx, b[1] + ny], [b[0] - nx, b[1] - ny], [a[0] - nx, a[1] - ny]], [uv; 4], color);
        self.circle(a[0], a[1], width / 2.0, color);
        self.circle(b[0], b[1], width / 2.0, color);
    }

    pub fn circle(&mut self, cx: f32, cy: f32, r: f32, color: [f32; 4]) {
        let n = 16;
        for i in 0..n {
            let a0 = i as f32 / n as f32 * std::f32::consts::TAU;
            let a1 = (i + 1) as f32 / n as f32 * std::f32::consts::TAU;
            self.tri([cx, cy], [cx + r * a0.cos(), cy + r * a0.sin()], [cx + r * a1.cos(), cy + r * a1.sin()], color);
        }
    }

    /**
     * One triangle of vertices as they are (their alpha times the overlay's), for what the other
     * calls do not make: a window onto the atlas clipped to a region, a gradient (the minimap).
     */
    pub fn raw_triangle(&mut self, v: [OVertex; 3]) {
        for mut p in v {
            p.color[3] *= self.alpha;
            self.verts.push(p);
        }
    }

    /// A filled triangle.
    pub fn triangle(&mut self, a: [f32; 2], b: [f32; 2], c: [f32; 2], color: [f32; 4]) {
        self.tri(a, b, c, color);
    }

    /// An arc from `a0` to `a1` radians (canvas convention, y down).
    pub fn arc(&mut self, cx: f32, cy: f32, r: f32, a0: f32, a1: f32, width: f32, color: [f32; 4]) {
        let n = 12;
        let mut prev = [cx + r * a0.cos(), cy + r * a0.sin()];
        for i in 1..=n {
            let a = a0 + (a1 - a0) * i as f32 / n as f32;
            let p = [cx + r * a.cos(), cy + r * a.sin()];
            self.line(prev, p, width, color);
            prev = p;
        }
    }

    // ------------------------------------------------------------ text

    /// The style the old `text` call stands for: the body sans, or for bold lines of title size
    /// the display serif (bold small lines stay the sans, heavier); the old drop shadow, softened.
    pub fn legacy_style(&self, size: f32, color: [f32; 4], align: Align, bold: bool) -> Style {
        let unit = self.height / 36.0;
        let mut s = if bold && size >= unit * DISPLAY_FROM { Style::display(size, color) } else { Style::body(size, color) };
        if bold && s.family == Family::Body {
            s = s.weight(0.25);
        }
        s.align = align;
        if self.shadow > 0.0 {
            // (Never more than an eighth of the letters' height: on small text a long shadow reads as
            // a second letter.)
            let d = self.shadow.max(size * 0.07).min((size * 0.12).max(1.0));
            s.shadow = Some(Shadow { dx: d * 0.8, dy: d, blur: d * 0.3, color: [0.0, 0.0, 0.0, color[3] * 0.7] });
        }
        s
    }

    /// The width of `text` at `size`, in the type the old `text` draws regular lines in.
    pub fn measure(&self, text: &str, size: f32) -> f32 {
        self.measure_styled(text, &Style::body(size, [1.0; 4]))
    }

    /// The width of `text` as `text` would draw it, bold or not.
    pub fn measure_as(&self, text: &str, size: f32, bold: bool) -> f32 {
        self.measure_styled(text, &self.legacy_style(size, [1.0; 4], Align::Left, bold))
    }

    /// Text in the game's type, centred vertically on `y` (the middle of the capitals, like canvas
    /// `textBaseline = 'middle'`): the body sans, or for `bold` titles the display serif.
    pub fn text(&mut self, text: &str, x: f32, y: f32, size: f32, color: [f32; 4], align: Align, bold: bool) {
        let s = self.legacy_style(size, color, align, bold);
        self.text_styled(text, x, y, &s);
    }

    /// The old pixel font (font8x8 at 4x, monospaced), for where it is part of the art.
    #[allow(clippy::too_many_arguments)]
    pub fn pixel_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: [f32; 4], align: Align, bold: bool) {
        let cell = size * GLYPH;
        let cw = cell * CONDENSE;
        let w = Self::pixel_measure(text, size);
        let mut pen = match align {
            Align::Left => x,
            Align::Center => x - w / 2.0,
            Align::Right => x - w,
        };
        let top = y - cell * 0.44;
        let passes: &[f32] = if bold { &[0.0, 0.0] } else { &[0.0] };
        for ch in text.chars() {
            let code = match ch {
                '♥' => HEART,
                c if (c as u32) < 128 => c as u32,
                c => EXTRA.iter().position(|&e| e == c).map(|i| EXTRA_CELL + i as u32).unwrap_or('?' as u32),
            };
            if ch != ' ' && code != SOLID {
                // Inset by a texel so bilinear sampling never bleeds in the neighboring glyph.
                let u0 = ((code % COLS) * CELL) as f32 / ATLAS_W as f32 + 1.0 / ATLAS_W as f32;
                let v0 = ((code / COLS) * CELL) as f32 / ATLAS_H as f32 + 1.0 / ATLAS_H as f32;
                let du = (CELL - 2) as f32 / ATLAS_W as f32;
                let dv = (CELL - 2) as f32 / ATLAS_H as f32;
                for (k, _) in passes.iter().enumerate() {
                    let ox = if k == 1 { (cw / 12.0).max(0.6) } else { 0.0 };
                    let x0 = pen + ox;
                    self.quad(
                        [[x0, top], [x0 + cw, top], [x0 + cw, top + cell], [x0, top + cell]],
                        [[u0, v0], [u0 + du, v0], [u0 + du, v0 + dv], [u0, v0 + dv]],
                        color,
                    );
                }
            }
            pen += cw;
        }
    }

    /// The old pixel font's width for `text`: every letter alike.
    pub fn pixel_measure(text: &str, size: f32) -> f32 {
        text.chars().count() as f32 * size * GLYPH * CONDENSE
    }

    /// Each letter's pen position and glyph for `text` in `style`, from 0; and the whole width.
    fn layout(text: &str, style: &Style) -> (Vec<(f32, char)>, f32) {
        let face = fonts().face(style.family);
        let em = face.em(style.size);
        let figure = if style.tabular { (b'0'..=b'9').map(|d| face.glyph(d as char).advance).fold(0.0, f32::max) } else { 0.0 };
        // Optical spacing: capitals set apart a little, and small text opened up as it shrinks.
        let caps = text.chars().any(char::is_alphabetic) && !text.chars().any(char::is_lowercase);
        let small = ((22.0 - em) / 22.0).clamp(0.0, 1.0) * 0.05;
        let tracking = style.tracking + small + if caps { 0.035 } else { 0.0 };
        let word = if caps { 0.08 } else { 0.0 };
        let mut pen = 0.0f32;
        let mut out = Vec::with_capacity(text.len());
        let mut prev: Option<char> = None;
        for c in text.chars() {
            if let Some(p) = prev {
                pen += face.kern(p, c) * em;
            }
            let g = face.glyph(c);
            if style.tabular && c.is_ascii_digit() {
                out.push((pen + (figure - g.advance) * em / 2.0, c));
                pen += figure * em;
            } else {
                out.push((pen, c));
                pen += g.advance * em;
            }
            pen += tracking * em;
            if c == ' ' {
                pen += word * em;
            }
            prev = Some(c);
        }
        if !out.is_empty() {
            pen -= tracking * em;
        }
        (out, pen.max(0.0))
    }

    /// Where each letter of `text` in `style` starts, from 0 (left-aligned), and the whole width:
    /// for drawing a word letter by letter (the title's bobbing letters).
    pub fn pens(&self, text: &str, style: &Style) -> (Vec<f32>, f32) {
        let (pens, width) = Self::layout(text, style);
        (pens.into_iter().map(|p| p.0).collect(), width)
    }

    /// The width of `text` in `style`, by its glyphs' real advances and kerning.
    pub fn measure_styled(&self, text: &str, style: &Style) -> f32 {
        Self::layout(text, style).1
    }

    /**
     * Draws `text` in `style` with `y` the middle of its capitals and `x` its left, middle or right
     * by `style.align`. The shadow (or glow) first, then the outline, then the letters, each a
     * pass over the type's field. Returns its width.
     */
    pub fn text_styled(&mut self, text: &str, x: f32, y: f32, style: &Style) -> f32 {
        let (pens, width) = Self::layout(text, style);
        let face = fonts().face(style.family);
        let em = face.em(style.size);
        let x0 = match style.align {
            Align::Left => x,
            Align::Center => x - width / 2.0,
            Align::Right => x - width,
        };
        let baseline = y + style.size * CAP / 2.0;
        // Field units: one stored texel is 1 / (2 SPREAD); a screen pixel is SDF_EM / em texels.
        let per_px = SDF_EM / em.max(1.0) / (2.0 * SPREAD);
        let grow = style.weight * 0.1;
        let outline = style.outline.map(|(w, c)| ((w * per_px).min(0.45), c));
        let reach = outline.map(|o| o.0).unwrap_or(0.0);
        let mut passes: Vec<(f32, f32, [f32; 2], [f32; 4])> = Vec::with_capacity(3);
        if let Some(s) = style.shadow {
            let soft = (s.blur * per_px).min(0.45);
            // (A glow spreads as far as it softens; a shadow keeps to the letters' shape.)
            let spread = if s.dx == 0.0 && s.dy == 0.0 { soft * 0.5 } else { 0.0 };
            // (The soft edge stops short of the field's floor, or the whole quad would glow.)
            let shift = (grow + reach + spread).min(0.4);
            passes.push((shift, soft.min(0.46 - shift), [s.dx, s.dy], s.color));
        }
        if let Some((w, c)) = outline {
            passes.push(((grow + w).min(0.48), 0.0, [0.0, 0.0], c));
        }
        passes.push((grow, 0.0, [0.0, 0.0], style.color));
        for (shift, soft, [dx, dy], color) in passes {
            for &(pen, c) in &pens {
                let g = face.glyph(c);
                if !g.ink {
                    continue;
                }
                let [px0, py0, px1, py1] = g.plane;
                let (gx0, gy0) = (x0 + pen + px0 * em + dx, baseline + py0 * em + dy);
                let (gx1, gy1) = (x0 + pen + px1 * em + dx, baseline + py1 * em + dy);
                let [u0, v0, u1, v1] = g.uv;
                let e = [FLAG_SDF as f32, shift, soft, 0.0];
                self.quad_ex([[gx0, gy0], [gx1, gy0], [gx1, gy1], [gx0, gy1]], [[u0, v0], [u1, v0], [u1, v1], [u0, v1]], color, [e; 4]);
            }
        }
        width
    }

    /// `text` broken into lines no wider than `max_width` in `style`, at spaces (a word longer than
    /// the line stands alone); explicit newlines break too.
    pub fn wrap_lines(&self, text: &str, max_width: f32, style: &Style) -> Vec<String> {
        let mut lines = Vec::new();
        for para in text.split('\n') {
            let mut line = String::new();
            for word in para.split(' ') {
                let trial = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
                if self.measure_styled(&trial, style) > max_width && !line.is_empty() {
                    lines.push(std::mem::replace(&mut line, word.to_string()));
                } else {
                    line = trial;
                }
            }
            lines.push(line);
        }
        lines
    }

    /// Word-wrapped text in `style` from `y` (the first line's middle), each line `line_height`
    /// below the last, aligned on `x` by `style.align`. Returns the y just below the last line.
    #[allow(clippy::too_many_arguments)]
    pub fn wrap_styled(&mut self, text: &str, x: f32, mut y: f32, max_width: f32, line_height: f32, style: &Style) -> f32 {
        for line in self.wrap_lines(text, max_width, style) {
            if !line.is_empty() {
                self.text_styled(&line, x, y, style);
            }
            y += line_height;
        }
        y
    }

    /// Word-wrapped centered text. Returns the y just below the last line.
    #[allow(clippy::too_many_arguments)]
    pub fn wrap_text(&mut self, text: &str, cx: f32, y: f32, max_width: f32, line_height: f32, size: f32, color: [f32; 4]) -> f32 {
        let s = self.legacy_style(size, color, Align::Center, false);
        self.wrap_styled(text, cx, y, max_width, line_height, &s)
    }
}

impl Default for Overlay {
    fn default() -> Self {
        Self::new()
    }
}

/// The type atlas's size, for the renderer.
pub fn sdf_size() -> (u32, u32) {
    (SDF_W, fonts().height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_later_stages_names_draw_in_their_own_letters() {
        assert_eq!(build_atlas().len(), (LEGACY * LEGACY * 4) as usize);
        assert!(EXTRA_CELL + EXTRA.len() as u32 <= (LEGACY / CELL) * (LEGACY / CELL), "room in the atlas");
        let unknown = extra_glyph('x');
        for c in EXTRA {
            assert!(extra_glyph(c) != unknown && extra_glyph(c).iter().any(|&b| b != 0), "{c}");
        }
        let f = fonts();
        for name in ["TÜLÜKUŞ", "ARÇURA", "TEPEGÖZ", "GORYŌ", "YEDİ BAŞLI EJDER", "Albastı", "ZEMHERİ NIGHTS", "RAGNARÖK"] {
            for face in &f.faces {
                assert!(name.chars().all(|c| face.has(c)), "{name}");
            }
        }
    }

    #[test]
    fn the_regions_keep_apart() {
        let mut all = vec![AtlasRegion::new(0, 0, LEGACY, LEGACY), ICON_REGION, MAP_REGION];
        all.extend((0..PORTRAIT_SLOTS).map(portrait_slot));
        for (i, a) in all.iter().enumerate() {
            assert!(a.x + a.w <= ATLAS_W && a.y + a.h <= ATLAS_H, "{a:?} inside");
            for b in &all[i + 1..] {
                let apart = a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y;
                assert!(apart, "{a:?} and {b:?}");
            }
        }
    }

    #[test]
    fn measure_agrees_with_what_is_drawn() {
        let mut o = Overlay::new();
        o.begin(1280.0, 720.0);
        for style in [Style::body(20.0, [1.0; 4]), Style::display(28.0, [1.0; 4]).tracking(0.05), Style::body(14.0, [1.0; 4]).tabular()] {
            for text in ["The Garden of Krampus", "WAVE 10/10", "Tülükuş", "AV To."] {
                let w = o.measure_styled(text, &style);
                o.verts.clear();
                let drawn = o.text_styled(text, 100.0, 50.0, &style);
                assert_eq!(w, drawn);
                // The ink sits within the measured run (the padding of the field aside).
                let face = fonts().face(style.family);
                let pad = SPREAD / SDF_EM * face.em(style.size);
                let xs: Vec<f32> = o.verts.iter().map(|v| v.pos[0]).collect();
                let (lo, hi) = (xs.iter().cloned().fold(f32::MAX, f32::min), xs.iter().cloned().fold(f32::MIN, f32::max));
                assert!(lo >= 100.0 - pad - style.size * 0.15, "{text}: starts at {lo}");
                assert!(hi <= 100.0 + w + pad + style.size * 0.15, "{text}: ends at {hi} past {w}");
                // Centred and right-aligned runs end where they should.
                o.verts.clear();
                o.text_styled(text, 500.0, 50.0, &style.right());
                let hi = o.verts.iter().map(|v| v.pos[0]).fold(f32::MIN, f32::max);
                assert!((hi - pad - 500.0).abs() < style.size * 0.3, "{text}: right edge {hi}");
            }
        }
        // Proportional: an i is narrower than an M; kerning tucks AV in.
        let s = Style::body(20.0, [1.0; 4]);
        assert!(o.measure_styled("iiii", &s) < o.measure_styled("MMMM", &s) * 0.5);
    }

    #[test]
    fn wrapping_keeps_inside_the_width() {
        let o = Overlay::new();
        let s = Style::body(18.0, [1.0; 4]);
        let text = "Up, Ragnar. The bog slime strikes from afar; close the distance before it spits again.";
        let lines = o.wrap_lines(text, 220.0, &s);
        assert!(lines.len() >= 3);
        for l in &lines {
            assert!(o.measure_styled(l, &s) <= 220.0, "{l}");
        }
        assert_eq!(lines.join(" "), text);
        assert_eq!(o.wrap_lines("one\ntwo", 1000.0, &s), vec!["one", "two"]);
    }
}
