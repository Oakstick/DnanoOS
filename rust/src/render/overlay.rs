//! 2D overlay batcher: rectangles, lines, arcs, pixel sprites and a bitmap
//! font, in logical pixels, blended in sRGB like a canvas.

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct OVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

pub const ATLAS_W: u32 = 512;
pub const ATLAS_H: u32 = 256;
const CELL: u32 = 32;
const COLS: u32 = 16;
/// Glyph cell used for solid fills (a fully set block).
const SOLID: u32 = 0;
/// Glyph cell holding the heart.
const HEART: u32 = 127;

const HEART_BITS: [u8; 8] = [
    0b0011_0110,
    0b0111_1111,
    0b0111_1111,
    0b0111_1111,
    0b0011_1110,
    0b0001_1100,
    0b0000_1000,
    0b0000_0000,
];

/// The atlas: every ASCII glyph of the public-domain font8x8, magnified 4x, one byte per texel.
pub fn build_atlas() -> Vec<u8> {
    let mut px = vec![0u8; (ATLAS_W * ATLAS_H) as usize];
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
                        px[(y * ATLAS_W + x) as usize] = 255;
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
    px
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

pub fn rgba(hex: u32, alpha: f32) -> [f32; 4] {
    [((hex >> 16) & 255) as f32 / 255.0, ((hex >> 8) & 255) as f32 / 255.0, (hex & 255) as f32 / 255.0, alpha]
}

pub struct Overlay {
    pub verts: Vec<OVertex>,
    pub width: f32,
    pub height: f32,
    /// Multiplies every alpha, like canvas globalAlpha.
    pub alpha: f32,
}

/// Share of the font size a glyph cell is tall; keeps capitals near a system font's cap height.
const GLYPH: f32 = 0.8;
/// Glyphs are drawn condensed, so lines of pixel text run about as long as proportional text.
const CONDENSE: f32 = 0.78;

impl Overlay {
    pub fn new() -> Self {
        Self { verts: Vec::with_capacity(16_384), width: 1.0, height: 1.0, alpha: 1.0 }
    }

    pub fn begin(&mut self, width: f32, height: f32) {
        self.verts.clear();
        self.width = width;
        self.height = height;
        self.alpha = 1.0;
    }

    fn solid_uv() -> [f32; 2] {
        let c = (SOLID % COLS) * CELL + CELL / 2;
        let r = (SOLID / COLS) * CELL + CELL / 2;
        [c as f32 / ATLAS_W as f32, r as f32 / ATLAS_H as f32]
    }

    fn tri(&mut self, a: [f32; 2], b: [f32; 2], c: [f32; 2], color: [f32; 4]) {
        let uv = Self::solid_uv();
        let color = [color[0], color[1], color[2], color[3] * self.alpha];
        for p in [a, b, c] {
            self.verts.push(OVertex { pos: p, uv, color });
        }
    }

    fn quad(&mut self, p: [[f32; 2]; 4], uv: [[f32; 2]; 4], color: [f32; 4]) {
        let color = [color[0], color[1], color[2], color[3] * self.alpha];
        for i in [0, 1, 2, 0, 2, 3] {
            self.verts.push(OVertex { pos: p[i], uv: uv[i], color });
        }
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let uv = Self::solid_uv();
        self.quad([[x, y], [x + w, y], [x + w, y + h], [x, y + h]], [uv; 4], color);
    }

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
        self.quad(
            [[a[0] + nx, a[1] + ny], [b[0] + nx, b[1] + ny], [b[0] - nx, b[1] - ny], [a[0] - nx, a[1] - ny]],
            [uv; 4],
            color,
        );
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

    pub fn measure(&self, text: &str, size: f32) -> f32 {
        text.chars().count() as f32 * size * GLYPH * CONDENSE
    }

    /// Text centered vertically on `y`, like canvas `textBaseline = 'middle'`.
    pub fn text(&mut self, text: &str, x: f32, y: f32, size: f32, color: [f32; 4], align: Align, bold: bool) {
        let cell = size * GLYPH;
        let cw = cell * CONDENSE;
        let w = self.measure(text, size);
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
                _ => '?' as u32,
            };
            if ch != ' ' && code != SOLID {
                let u0 = ((code % COLS) * CELL) as f32 / ATLAS_W as f32;
                let v0 = ((code / COLS) * CELL) as f32 / ATLAS_H as f32;
                let du = CELL as f32 / ATLAS_W as f32;
                let dv = CELL as f32 / ATLAS_H as f32;
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

    /// Word-wrapped centered text. Returns the y just below the last line.
    pub fn wrap_text(&mut self, text: &str, cx: f32, mut y: f32, max_width: f32, line_height: f32, size: f32, color: [f32; 4]) -> f32 {
        let mut line = String::new();
        for word in text.split(' ') {
            let trial = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if self.measure(&trial, size) > max_width && !line.is_empty() {
                self.text(&line, cx, y, size, color, Align::Center, false);
                y += line_height;
                line = word.to_string();
            } else {
                line = trial;
            }
        }
        if !line.is_empty() {
            self.text(&line, cx, y, size, color, Align::Center, false);
            y += line_height;
        }
        y
    }
}

impl Default for Overlay {
    fn default() -> Self {
        Self::new()
    }
}
