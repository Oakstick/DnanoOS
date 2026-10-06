//! The engine's loading screen, shown from the moment the window opens until the renderer is
//! ready. The shaders take seconds to compile on DX12, so the screen draws itself on its own
//! thread meanwhile, while the renderer reports which step it is on through a `Progress`.
//!
//! Its backdrop is the controller legend (UI.md §9): the game's title in Cinzel across the top,
//! the gamepad drawn large left of the middle with every part called out to what it does from the
//! saved bindings (`vh.bindings`, read from the settings file as `engine_start_secs` is; the
//! defaults if none), the keyboard and mouse's column to its right, and the device used last
//! (`vh.last_device`: the controller until one is known) lit. The legend's data is
//! `render::legend`'s, the in-game loading card's too. It is set once on the CPU with ab_glyph in
//! the game's two faces, on the screen's own thread before its first frame (a few milliseconds),
//! into a layer the size of the window. Live over it:
//! - a fade in, and a dark gradient (or, should `assets/engine/splash.png` be there, that art,
//!   dimmed under the legend);
//! - a progress bar with its percentage, timed against how long the last start took;
//! - the step it is on, and the checklist of steps: done, under way (a spinner), to come.
//!
//! `capture` draws one frame of it into a picture (`--splash out.png`).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ab_glyph::{point, Font, FontRef, GlyphId, PxScale, ScaleFont, VariableFont};

use crate::input::{Bindings, Control, Device, MouseButton, PadButton};
use crate::render::legend::{self, Part};

/// The engine's name, in small type under the steps.
const ENGINE: &str = "Distinguisher Nano  ·  real-time authoring engine";
/// The game's title, across the top.
const TITLE: &str = "ADVENTURES OF THE ANCIENT BLADES";
/// The steps the renderer goes through, as the checklist names them.
const STEPS: [&str; 3] = ["Initializing Graphics Device", "Preparing Runtime", "Compiling Shaders"];

/// The colour the window shows before the screen fades in, as displayed (the target is not sRGB).
pub const BG: wgpu::Color = wgpu::Color { r: 0.035, g: 0.05, b: 0.065, a: 1.0 };

/// Where the logo sits in the backdrop art, as uv: its centre and half size.
const LOGO: [f32; 4] = [0.5, 0.33, 0.085, 0.135];
/// The layout, as fractions of the window: the bar's ends and height, and the status line and
/// the checklist's heights. Sizes are in pixels at a 941-pixel-high window, scaled with it.
const BAR: (f32, f32, f32) = (0.32, 0.682, 0.7525);
const STATUS_Y: f32 = 0.794;
const STEPS_Y: f32 = 0.843;
const REF_H: f32 = 941.0;
/// What the layer's dynamic band covers (the rest of it, the legend, is drawn once).
const BAND: (f32, f32) = (0.725, 0.875);

/// How long a start takes before any has been timed, in seconds.
const FIRST_GUESS: f32 = 10.0;
const TIMED_KEY: &str = "engine_start_secs";

const WHITE: [f32; 3] = [0.9, 0.92, 0.95];
const SOFT: [f32; 3] = [0.72, 0.76, 0.8];
const DIM: [f32; 3] = [0.42, 0.47, 0.53];
const CYAN: [f32; 3] = [0.38, 0.78, 1.0];
const DONE: [f32; 3] = [0.6, 0.78, 0.92];

/// A colour as the layer takes it (display values: the target is not sRGB).
fn hex(c: u32) -> [f32; 3] {
    [((c >> 16) & 255) as f32 / 255.0, ((c >> 8) & 255) as f32 / 255.0, (c & 255) as f32 / 255.0]
}

/// The legend's colours: the HUD's gold, sand and type; the pad's plastic.
const GOLD: u32 = 0xeab55e;
const SAND: u32 = 0xd7ccb8;
const TEXT: u32 = 0xe6e0d2;
const TEXT_DIM: u32 = 0x9a9488;
const COPPER: u32 = 0xb06a44;
const PAD_BODY: u32 = 0x2a2e38;
const PAD_EDGE: u32 = 0x626a7c;
const PAD_DARK: u32 = 0x15171d;
const PAD_CAP: u32 = 0x3e4452;
const PAD_SHINE: u32 = 0x8a93a6;
const PAD_LINE: u32 = 0x9aa3b8;
const KEYCAP: u32 = 0xe8e0cc;
const KEY_LABEL: u32 = 0x1a1612;

/// Which step the renderer is on (an index into the checklist), shared with the loading screen.
#[derive(Clone, Default)]
pub struct Progress(Arc<AtomicU32>);

impl Progress {
    pub fn step(&self, n: u32) {
        self.0.store(n, Ordering::Relaxed);
    }
    fn get(&self) -> u32 {
        self.0.load(Ordering::Relaxed)
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    /// Seconds in, progress (0-1), the window's aspect, the art's aspect.
    a: [f32; 4],
    /// The window's size, whether there is art, and the pixel scale.
    res: [f32; 4],
    logo: [f32; 4],
    /// The bar in pixels: its two ends, its height, its half thickness.
    bar: [f32; 4],
    /// The spinner in pixels: its centre, its radius, and whether it shows.
    spin: [f32; 4],
    bg: [f32; 4],
}

/// Text and icons, drawn on the CPU into a layer the size of the window (premultiplied).
struct Layer {
    w: usize,
    h: usize,
    px: Vec<[f32; 4]>,
}

impl Layer {
    fn put(&mut self, x: i32, y: i32, rgb: [f32; 3], a: f32) {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h || a <= 0.0 {
            return;
        }
        let p = &mut self.px[y as usize * self.w + x as usize];
        let a = a.min(1.0);
        for k in 0..3 {
            p[k] = rgb[k] * a + p[k] * (1.0 - a);
        }
        p[3] = a + p[3] * (1.0 - a);
    }

    fn clear_rows(&mut self, y0: usize, y1: usize) {
        for p in &mut self.px[y0 * self.w..y1.min(self.h) * self.w] {
            *p = [0.0; 4];
        }
    }

    /// Fills by a signed distance over a box: `dist` in pixels, negative inside.
    #[allow(clippy::too_many_arguments)]
    fn shape(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, rgb: [f32; 3], alpha: f32, dist: impl Fn(f32, f32) -> f32) {
        for y in y0.floor() as i32..y1.ceil() as i32 {
            for x in x0.floor() as i32..x1.ceil() as i32 {
                let d = dist(x as f32 + 0.5, y as f32 + 0.5);
                self.put(x, y, rgb, (0.5 - d).clamp(0.0, 1.0) * alpha);
            }
        }
    }

    fn ring(&mut self, cx: f32, cy: f32, r: f32, thick: f32, rgb: [f32; 3], alpha: f32) {
        let e = r + thick + 1.0;
        self.shape(cx - e, cy - e, cx + e, cy + e, rgb, alpha, |x, y| ((x - cx).hypot(y - cy) - r).abs() - thick / 2.0);
    }

    fn disc(&mut self, cx: f32, cy: f32, r: f32, rgb: [f32; 3], alpha: f32) {
        let e = r + 1.0;
        self.shape(cx - e, cy - e, cx + e, cy + e, rgb, alpha, |x, y| (x - cx).hypot(y - cy) - r);
    }

    /// A box with rounded corners (radius `r`).
    #[allow(clippy::too_many_arguments)]
    fn round_rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, rgb: [f32; 3], alpha: f32) {
        let r = r.min(w / 2.0).min(h / 2.0);
        let (cx, cy, hw, hh) = (x + w / 2.0, y + h / 2.0, w / 2.0 - r, h / 2.0 - r);
        self.shape(x - 1.0, y - 1.0, x + w + 1.0, y + h + 1.0, rgb, alpha, |px, py| {
            let (qx, qy) = (((px - cx).abs() - hw).max(0.0), ((py - cy).abs() - hh).max(0.0));
            qx.hypot(qy) - r
        });
    }

    fn segment(&mut self, a: (f32, f32), b: (f32, f32), thick: f32, rgb: [f32; 3], alpha: f32) {
        let e = thick + 1.0;
        let (x0, x1) = (a.0.min(b.0) - e, a.0.max(b.0) + e);
        let (y0, y1) = (a.1.min(b.1) - e, a.1.max(b.1) + e);
        self.shape(x0, y0, x1, y1, rgb, alpha, |x, y| {
            let (px, py, dx, dy) = (x - a.0, y - a.1, b.0 - a.0, b.1 - a.1);
            let t = ((px * dx + py * dy) / (dx * dx + dy * dy).max(1e-6)).clamp(0.0, 1.0);
            (px - dx * t).hypot(py - dy * t) - thick / 2.0
        });
    }
}

/// The game's two faces.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Face {
    Body,
    Display,
}

/// Type, set with ab_glyph in the game's faces (`assets/fonts/`): Alegreya Sans for the words,
/// Cinzel for the title; what one lacks, the other lends.
struct Type {
    body: FontRef<'static>,
    display: FontRef<'static>,
}

impl Type {
    fn new() -> Option<Self> {
        let body = FontRef::try_from_slice(crate::render::text::BODY_TTF).ok()?;
        let mut display = FontRef::try_from_slice(crate::render::text::DISPLAY_TTF).ok()?;
        display.set_variation(b"wght", 700.0);
        Some(Self { body, display })
    }

    /// The font that has `c` (the face asked for first) and its glyph.
    fn glyph(&self, face: Face, c: char) -> (&FontRef<'static>, GlyphId) {
        let (first, other) = match face {
            Face::Body => (&self.body, &self.display),
            Face::Display => (&self.display, &self.body),
        };
        let id = first.glyph_id(c);
        if id.0 == 0 && other.glyph_id(c).0 != 0 {
            (other, other.glyph_id(c))
        } else {
            (first, id)
        }
    }

    fn width(&self, face: Face, text: &str, size: f32, tracking: f32) -> f32 {
        let mut w = 0.0;
        let mut prev: Option<(*const FontRef<'static>, GlyphId)> = None;
        for c in text.chars() {
            let (font, id) = self.glyph(face, c);
            let sf = font.as_scaled(PxScale::from(size));
            if let Some((pf, p)) = prev {
                if std::ptr::eq(pf, font) {
                    w += sf.kern(p, id);
                }
            }
            w += sf.h_advance(id) + tracking;
            prev = Some((font as *const _, id));
        }
        (w - tracking).max(0.0)
    }

    /// Sets `text` from `x`, centred on the height `cy`; returns where it ends.
    #[allow(clippy::too_many_arguments)]
    fn draw(&self, layer: &mut Layer, face: Face, text: &str, size: f32, x: f32, cy: f32, tracking: f32, rgb: [f32; 3], alpha: f32) -> f32 {
        let scale = PxScale::from(size);
        // The cap height's middle on `cy`.
        let first = match face {
            Face::Body => &self.body,
            Face::Display => &self.display,
        };
        let baseline = cy + first.as_scaled(scale).ascent() * 0.36;
        let mut x = x;
        let mut prev: Option<(*const FontRef<'static>, GlyphId)> = None;
        for c in text.chars() {
            let (font, id) = self.glyph(face, c);
            let sf = font.as_scaled(scale);
            if let Some((pf, p)) = prev {
                if std::ptr::eq(pf, font) {
                    x += sf.kern(p, id);
                }
            }
            if let Some(o) = font.outline_glyph(id.with_scale_and_position(scale, point(x, baseline))) {
                let b = o.px_bounds();
                o.draw(|gx, gy, cov| layer.put(b.min.x as i32 + gx as i32, b.min.y as i32 + gy as i32, rgb, cov * alpha));
            }
            x += sf.h_advance(id) + tracking;
            prev = Some((font as *const _, id));
        }
        x
    }

    #[allow(clippy::too_many_arguments)]
    fn centred(&self, layer: &mut Layer, face: Face, text: &str, size: f32, cx: f32, cy: f32, tracking: f32, rgb: [f32; 3], alpha: f32) {
        let w = self.width(face, text, size, tracking);
        self.draw(layer, face, text, size, cx - w / 2.0, cy, tracking, rgb, alpha);
    }

    /// Sets `text` ending at `x`; returns where it starts.
    #[allow(clippy::too_many_arguments)]
    fn right(&self, layer: &mut Layer, face: Face, text: &str, size: f32, x: f32, cy: f32, rgb: [f32; 3], alpha: f32) -> f32 {
        let w = self.width(face, text, size, 0.0);
        self.draw(layer, face, text, size, x - w, cy, 0.0, rgb, alpha);
        x - w
    }
}

/// The loading screen, ready to draw.
pub struct Splash {
    pipe: wgpu::RenderPipeline,
    bind: wgpu::BindGroup,
    uniforms: wgpu::Buffer,
    layer_tex: wgpu::Texture,
    layer: Layer,
    kind: Type,
    img_aspect: f32,
    has_art: bool,
    size: (u32, u32),
    /// Where the spinner goes (the step under way's icon), in pixels.
    spin: [f32; 4],
    /// The bindings and the device used last, as saved, for the legend.
    bindings: Bindings,
    device: Device,
    /// How long the legend took to set, in milliseconds (the start's cost of it).
    pub legend_ms: f64,
}

/// Paints `surface` plain `BG`.
pub fn clear(device: &wgpu::Device, queue: &wgpu::Queue, surface: &wgpu::Surface) {
    let Ok(frame) = surface.get_current_texture() else { return };
    let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("splash clear") });
    enc.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("splash clear"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            resolve_target: None,
            ops: wgpu::Operations { load: wgpu::LoadOp::Clear(BG), store: wgpu::StoreOp::Store },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    queue.submit([enc.finish()]);
    frame.present();
}

/// Where the backdrop art is looked for, in order.
fn art_paths() -> Vec<PathBuf> {
    let rel = PathBuf::from("assets").join("engine").join("splash.png");
    let mut out = Vec::new();
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(PathBuf::from)) {
        out.push(dir.join(&rel));
    }
    out.push(rel.clone());
    out.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(&rel));
    out
}

/// The backdrop art as RGBA8: its width, height and pixels.
fn load_art() -> Option<(u32, u32, Vec<u8>)> {
    let bytes = art_paths().into_iter().find_map(|p| std::fs::read(p).ok())?;
    let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
    dec.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = dec.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf.as_chunks::<3>().0.iter().flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => buf.as_chunks::<2>().0.iter().flat_map(|c| [c[0], c[0], c[0], c[1]]).collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    Some((info.width, info.height, rgba))
}

fn texture(device: &wgpu::Device, label: &str, w: u32, h: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        // (Display values in, display values out: no sRGB decode.)
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn upload(queue: &wgpu::Queue, tex: &wgpu::Texture, w: u32, y0: u32, rows: u32, rgba: &[u8]) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: tex,
            mip_level: 0,
            origin: wgpu::Origin3d { x: 0, y: y0, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * w), rows_per_image: Some(rows) },
        wgpu::Extent3d { width: w, height: rows, depth_or_array_layers: 1 },
    );
}

impl Splash {
    /// The loading screen for a `size` target of `format` (None if its type will not load). Its
    /// legend is set later, on its own thread (`prepare`).
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat, size: (u32, u32)) -> Option<Self> {
        let kind = Type::new()?;
        let (aw, ah, art) = load_art().unwrap_or((1, 1, vec![0, 0, 0, 255]));
        let has_art = aw > 1;
        let art_tex = texture(device, "splash art", aw, ah);
        upload(queue, &art_tex, aw, 0, ah, &art);
        let (w, h) = (size.0.max(1), size.1.max(1));
        let layer_tex = texture(device, "splash layer", w, h);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("splash"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("splash"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let tex_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("splash"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                tex_entry(1),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                tex_entry(3),
            ],
        });
        let art_view = art_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let layer_view = layer_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("splash"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniforms.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&art_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&layer_view) },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("splash"),
            source: wgpu::ShaderSource::Wgsl(include_str!("splash.wgsl").into()),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("splash"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("splash"),
            layout: Some(&pl),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs"), compilation_options: Default::default(), buffers: &[] },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            multiview: None,
            cache: None,
        });
        Some(Self {
            pipe,
            bind,
            uniforms,
            layer_tex,
            layer: Layer { w: w as usize, h: h as usize, px: vec![[0.0; 4]; (w * h) as usize] },
            kind,
            img_aspect: aw as f32 / ah as f32,
            has_art,
            size: (w, h),
            spin: [0.0; 4],
            bindings: legend::saved_bindings(),
            device: legend::saved_device(),
            legend_ms: 0.0,
        })
    }

    /// The legend set and the whole layer sent up, at `step` and `percent` (before the first frame).
    fn prepare(&mut self, queue: &wgpu::Queue, step: u32, percent: u32) {
        let at = Instant::now();
        self.draw_legend();
        self.legend_ms = at.elapsed().as_secs_f64() * 1e3;
        self.draw_band(step, percent);
        let (w, h) = self.size;
        let all: Vec<u8> = self.layer.px.iter().flat_map(|p| p.map(|c| (c.clamp(0.0, 1.0) * 255.0 + 0.5) as u8)).collect();
        upload(queue, &self.layer_tex, w, 0, h, &all);
    }

    fn scale(&self) -> f32 {
        self.size.1 as f32 / REF_H
    }

    /// The legend (UI.md §9), set once: the title, the pad and its callouts, the keyboard and
    /// mouse's column, the device used last lit, and the engine's name under the steps.
    fn draw_legend(&mut self) {
        let (w, h, s) = (self.size.0 as f32, self.size.1 as f32, self.scale());
        let kind = &self.kind;
        let layer = &mut self.layer;
        // The title, with a copper rule and a diamond under it.
        kind.centred(layer, Face::Display, TITLE, 44.0 * s, w / 2.0, h * 0.075, 5.0 * s, hex(GOLD), 1.0);
        let (ry, rw) = (h * 0.118, 260.0 * s);
        layer.segment((w / 2.0 - rw, ry), (w / 2.0 - 10.0 * s, ry), 1.2 * s, hex(COPPER), 0.8);
        layer.segment((w / 2.0 + 10.0 * s, ry), (w / 2.0 + rw, ry), 1.2 * s, hex(COPPER), 0.8);
        let d = 5.0 * s;
        layer.shape(w / 2.0 - d - 1.0, ry - d - 1.0, w / 2.0 + d + 1.0, ry + d + 1.0, hex(GOLD), 1.0, |x, y| {
            (x - w / 2.0).abs() + (y - ry).abs() - d
        });

        // The pad left of the middle (as large as its labels leave room for), the keyboard and
        // mouse's column to its right.
        let pad_last = self.device.is_pad();
        let pad_device = if pad_last { self.device } else { Device::Xbox };
        let head = 15.0 * s;
        let lit = |on: bool| if on { (hex(GOLD), 1.0) } else { (hex(TEXT_DIM), 0.85) };
        let kw = (w * 0.22).min(300.0 * s);
        let kx = w - kw - 36.0 * s;
        let top = h * 0.165;
        let area = [30.0 * s, top + 22.0 * s, kx - 70.0 * s, h * 0.655 - top - 22.0 * s];
        let [cx, pad_top] = draw_pad(layer, kind, &self.bindings, pad_device, area, 17.5 * s);
        let (c, a) = lit(pad_last);
        let pad_head = if pad_last { "CONTROLLER  ·  USED LAST" } else { "CONTROLLER" };
        kind.centred(layer, Face::Body, pad_head, head, cx, pad_top - 14.0 * s, 2.5 * s, c, a);
        let (c, a) = lit(!pad_last);
        let key_head = if pad_last { "KEYBOARD AND MOUSE" } else { "KEYBOARD AND MOUSE  ·  USED LAST" };
        kind.draw(layer, Face::Body, key_head, head, kx, top, 2.5 * s, c, a);
        layer.segment((kx, top + 13.0 * s), (kx + kw, top + 13.0 * s), 1.0 * s, hex(COPPER), 0.7);
        draw_keys(layer, kind, &self.bindings, kx, top + 24.0 * s, kw, 16.5 * s);

        // The engine's name, small, under the checklist.
        kind.centred(layer, Face::Body, ENGINE, 12.5 * s, w / 2.0, h * 0.93, 1.5 * s, SOFT, 0.55);
    }

    /// The band under the bar: its percentage, the step under way, and the checklist.
    fn draw_band(&mut self, step: u32, percent: u32) {
        let (w, h, s) = (self.size.0 as f32, self.size.1 as f32, self.scale());
        self.layer.clear_rows((h * BAND.0) as usize, (h * BAND.1) as usize);
        let kind = &self.kind;
        let layer = &mut self.layer;
        kind.draw(layer, Face::Body, &format!("{percent}%"), 15.0 * s, w * (BAR.1 + 0.014), h * BAR.2, 0.0, SOFT, 1.0);
        let status = match STEPS.get(step as usize) {
            Some(name) => format!("{name}..."),
            None => "Ready".to_string(),
        };
        kind.centred(layer, Face::Body, &status, 17.0 * s, w / 2.0, h * STATUS_Y, 0.4 * s, WHITE, 1.0);
        // The checklist: each step's icon and name, a short rule between them.
        let (text, r, gap, rule) = (13.5 * s, 11.0 * s, 10.0 * s, 22.0 * s);
        let widths: Vec<f32> = STEPS.iter().map(|t| 2.0 * r + gap + kind.width(Face::Body, t, text, 0.0)).collect();
        let total = widths.iter().sum::<f32>() + (STEPS.len() - 1) as f32 * (rule + 2.0 * gap * 1.4);
        let mut x = w / 2.0 - total / 2.0;
        let cy = h * STEPS_Y;
        self.spin = [0.0; 4];
        for (i, name) in STEPS.iter().enumerate() {
            let i = i as u32;
            let (icon, label, alpha) = if i < step {
                (DONE, SOFT, 0.95)
            } else if i == step {
                (CYAN, CYAN, 1.0)
            } else {
                (DIM, DIM, 0.85)
            };
            let cx = x + r;
            layer.ring(cx, cy, r, 1.4 * s, icon, alpha * if i == step { 0.45 } else { 1.0 });
            if i < step {
                // A tick.
                layer.segment((cx - r * 0.42, cy + r * 0.02), (cx - r * 0.1, cy + r * 0.34), 1.5 * s, icon, 1.0);
                layer.segment((cx - r * 0.1, cy + r * 0.34), (cx + r * 0.45, cy - r * 0.3), 1.5 * s, icon, 1.0);
            } else if i == step {
                self.spin = [cx, cy, r, 1.0];
            }
            let end = kind.draw(layer, Face::Body, name, text, cx + r + gap, cy, 0.0, label, alpha);
            x = end;
            if (i as usize) < STEPS.len() - 1 {
                let (a, b) = (x + gap * 1.4, x + gap * 1.4 + rule);
                layer.segment((a, cy), (b, cy), 1.0 * s, DIM, 0.6);
                x = b + gap * 1.4;
            }
        }
    }

    fn upload_band(&self, queue: &wgpu::Queue) {
        let h = self.size.1 as f32;
        let (y0, y1) = ((h * BAND.0) as usize, ((h * BAND.1) as usize).min(self.layer.h));
        let w = self.layer.w;
        let rows: Vec<u8> = self.layer.px[y0 * w..y1 * w].iter().flat_map(|p| p.map(|c| (c.clamp(0.0, 1.0) * 255.0 + 0.5) as u8)).collect();
        upload(queue, &self.layer_tex, w as u32, y0 as u32, (y1 - y0) as u32, &rows);
    }

    fn write_uniforms(&self, queue: &wgpu::Queue, time: f32, progress: f32) {
        let (w, h, s) = (self.size.0 as f32, self.size.1 as f32, self.scale());
        let u = Uniforms {
            a: [time, progress, w / h, self.img_aspect],
            res: [w, h, if self.has_art { 1.0 } else { 0.0 }, s],
            logo: LOGO,
            bar: [w * BAR.0, w * BAR.1, h * BAR.2, 2.2 * s],
            spin: self.spin,
            bg: [BG.r as f32, BG.g as f32, BG.b as f32, 1.0],
        };
        queue.write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&u));
    }

    /// Records the screen's one pass into `view`.
    fn pass(&self, enc: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("splash"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(BG), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipe);
        pass.set_bind_group(0, &self.bind, &[]);
        pass.draw(0..3, 0..1);
    }

    /// Draws one frame at `time` seconds in, `progress` along. False when the surface had no
    /// frame to give.
    fn draw(&self, device: &wgpu::Device, queue: &wgpu::Queue, surface: &wgpu::Surface, time: f32, progress: f32) -> bool {
        let Ok(frame) = surface.get_current_texture() else { return false };
        self.write_uniforms(queue, time, progress);
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("splash") });
        self.pass(&mut enc, &view);
        queue.submit([enc.finish()]);
        frame.present();
        true
    }

    /// Runs the loading screen on its own thread, drawing into `surface` until `finish`;
    /// `painted` is called once its first frame is up (or it gives up on one).
    pub fn run(
        mut self,
        surface: wgpu::Surface<'static>,
        device: wgpu::Device,
        queue: wgpu::Queue,
        config: wgpu::SurfaceConfiguration,
        progress: Progress,
        painted: impl FnOnce() + Send + 'static,
    ) -> Running {
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let handle = std::thread::spawn(move || {
            let start = Instant::now();
            // The legend, set here rather than on the renderer's thread, which goes on to the shaders.
            self.prepare(&queue, progress.get(), 0);
            // The bar is timed against the last start (the compile is one long step, so the
            // steps alone cannot move it smoothly), never passing 96% until it is done.
            let expected = crate::platform::load(TIMED_KEY).and_then(|v| v.parse::<f32>().ok()).filter(|t| *t > 0.5).unwrap_or(FIRST_GUESS);
            let mut painted = Some(painted);
            let mut shown = 0.0f32;
            let mut done_at: Option<Instant> = None;
            let mut drawn = (progress.get(), 0);
            loop {
                let done = stopping.load(Ordering::Relaxed);
                if done && done_at.is_none() {
                    done_at = Some(Instant::now());
                    // Remember how long it took, for the next start's bar.
                    let took = start.elapsed().as_secs_f32();
                    crate::platform::save(TIMED_KEY, &format!("{:.2}", (expected + took) * 0.5));
                }
                let t = start.elapsed().as_secs_f32();
                let target = if done { 1.0 } else { (t / expected).min(0.96) };
                shown += (target - shown) * if done { 0.3 } else { 0.1 };
                let step = if done { STEPS.len() as u32 } else { progress.get() };
                let percent = (shown * 100.0).round() as u32;
                if drawn != (step, percent) {
                    self.draw_band(step, percent);
                    self.upload_band(&queue);
                    drawn = (step, percent);
                }
                if self.draw(&device, &queue, &surface, t, shown) {
                    if let Some(p) = painted.take() {
                        p();
                    }
                } else {
                    // Lost or outdated: make it again (the renderer resizes it once it is ready).
                    surface.configure(&device, &config);
                    std::thread::sleep(Duration::from_millis(8));
                }
                if painted.is_some() && start.elapsed() > Duration::from_secs(2) {
                    if let Some(p) = painted.take() {
                        p();
                    }
                }
                // Done: a last moment with the bar full and every step ticked.
                if done_at.is_some_and(|d| d.elapsed() > Duration::from_millis(350)) {
                    break;
                }
            }
            if let Some(p) = painted.take() {
                p();
            }
            surface
        });
        Running { stop, handle }
    }
}

/// A point of the pad (pad units about its centre) on screen.
fn at(cx: f32, cy: f32, u: f32, p: [f32; 2]) -> (f32, f32) {
    (cx + p[0] * u, cy + p[1] * u)
}

/// The pad as `device` marks it, each part called out to what it does from `bindings` in text
/// `size` tall, the whole fitted into `area` (the overlay's `draw_pad_legend`, set with the CPU's
/// raster). Its centre's x and its top.
fn draw_pad(layer: &mut Layer, kind: &Type, bindings: &Bindings, device: Device, area: [f32; 4], size: f32) -> [f32; 2] {
    let callouts = legend::callouts(bindings);
    let glyph = size * 1.3;
    let gap = size * 0.5;
    let label = |parts: &[Part]| {
        parts
            .iter()
            .filter_map(|&p| callouts.iter().find(|c| c.part == p).map(|c| (p, c)))
            .map(|(p, c)| kind.width(Face::Body, &c.does, size, 0.0) + 2.0 * gap + part_width(kind, p, device, glyph))
            .fold(0.0f32, f32::max)
    };
    let (lw_text, rw_text) = (label(&Part::LEFT), label(&Part::RIGHT));
    let [ax, ay, aw, ah] = area;
    let u = ((aw - lw_text - rw_text) / 22.4).min(ah / 13.6).max(4.0);
    let cx = ax + lw_text + 11.2 * u + ((aw - lw_text - rw_text - 22.4 * u) / 2.0).max(0.0);
    let cy = ay + ah / 2.0 + 0.4 * u;
    let lw = (u * 0.22).max(1.5);
    let p = |q: [f32; 2]| at(cx, cy, u, q);
    // Triggers and bumpers behind the body, named on them.
    for left in [true, false] {
        for (r, label_y, trigger) in [(legend::TRIGGER, -5.55, true), (legend::BUMPER, -4.25, false)] {
            let x = if left { r[0] } else { -r[0] - r[2] };
            let (px, py) = p([x, r[1]]);
            let (pw, ph) = (r[2] * u, r[3] * u);
            layer.round_rect(px - lw, py - lw, pw + 2.0 * lw, ph + 2.0 * lw, ph / 2.0 + lw, hex(PAD_EDGE), 1.0);
            layer.round_rect(px, py, pw, ph, ph / 2.0, hex(PAD_CAP), 1.0);
            let b = match (trigger, left) {
                (true, true) => PadButton::LeftTrigger,
                (true, false) => PadButton::RightTrigger,
                (false, true) => PadButton::LeftBumper,
                (false, false) => PadButton::RightBumper,
            };
            let name = crate::input::control_name(Control::Pad(b), device);
            kind.centred(layer, Face::Body, &name, u * 0.62, cx + (x + r[2] / 2.0) * u, cy + label_y * u, 0.0, hex(PAD_SHINE), 1.0);
        }
    }
    // The body: lobes, grips and slab, rimmed and then filled.
    for (grow, color) in [(lw, PAD_EDGE), (0.0, PAD_BODY)] {
        for &(x, y, r) in &legend::BODY_CIRCLES {
            let (sx, sy) = p([x, y]);
            layer.disc(sx, sy, r * u + grow, hex(color), 1.0);
        }
        let b = legend::BODY_SLAB;
        let (rx, ry) = p([b[0], b[1]]);
        layer.round_rect(rx - grow, ry - grow, b[2] * u + 2.0 * grow, b[3] * u + 2.0 * grow, grow.max(1.0), hex(color), 1.0);
    }
    let (hx, hy) = p([-4.0, -4.0]);
    layer.round_rect(hx, hy, 8.0 * u, 0.18 * u, 0.09 * u, hex(PAD_SHINE), 0.35);

    // ---- the callouts, under the buttons so each line meets its button's edge
    let (top, bottom) = (cy - 6.4 * u, cy + 5.6 * u);
    for (side, parts) in [(-1.0f32, &Part::LEFT[..]), (1.0, &Part::RIGHT[..])] {
        let edge = cx + side * 11.2 * u;
        let n = parts.len();
        for (i, &part) in parts.iter().enumerate() {
            let Some(c) = callouts.iter().find(|c| c.part == part) else { continue };
            let y = top + (bottom - top) * (i as f32 + 0.5) / n as f32;
            let a = p(part.anchor());
            layer.disc(a.0, a.1, (u * 0.13).max(2.0), hex(PAD_LINE), 0.95);
            let elbow = (edge - side * 0.8 * u, y);
            layer.segment(a, elbow, (u * 0.07).max(1.0), hex(PAD_LINE), 0.6);
            layer.segment(elbow, (edge, y), (u * 0.07).max(1.0), hex(PAD_LINE), 0.6);
            let gap = size * 0.5;
            let gw = part_width(kind, part, device, glyph);
            let dim = c.actions.is_empty() && !matches!(part, Part::LeftStick | Part::RightStick);
            let color = hex(if dim { TEXT_DIM } else { TEXT });
            if side < 0.0 {
                let gx = edge - gap - gw;
                part_glyph(layer, kind, part, device, gx, y, glyph);
                kind.right(layer, Face::Body, &c.does, size, gx - gap, y, color, 1.0);
            } else {
                let gx = edge + gap;
                part_glyph(layer, kind, part, device, gx, y, glyph);
                kind.draw(layer, Face::Body, &c.does, size, gx + gw + gap, y, 0.0, color, 1.0);
            }
        }
    }

    // ---- the buttons: sticks, D-pad, face buttons, the three in the middle
    for s in [p(legend::LEFT_STICK), p(legend::RIGHT_STICK)] {
        layer.disc(s.0, s.1, 1.55 * u + lw * 0.6, hex(PAD_EDGE), 1.0);
        layer.disc(s.0, s.1, 1.55 * u, hex(PAD_DARK), 1.0);
        layer.disc(s.0, s.1, 1.1 * u + lw * 0.4, hex(PAD_SHINE), 1.0);
        layer.disc(s.0, s.1, 1.1 * u, hex(PAD_CAP), 1.0);
        layer.disc(s.0, s.1, 0.35 * u, hex(PAD_DARK), 1.0);
    }
    let dp = p(legend::DPAD);
    layer.disc(dp.0, dp.1, 1.75 * u + lw * 0.4, hex(PAD_EDGE), 1.0);
    layer.disc(dp.0, dp.1, 1.75 * u, hex(PAD_DARK), 1.0);
    layer.round_rect(dp.0 - 1.35 * u, dp.1 - 0.45 * u, 2.7 * u, 0.9 * u, 0.45 * u, hex(PAD_CAP), 1.0);
    layer.round_rect(dp.0 - 0.45 * u, dp.1 - 1.35 * u, 0.9 * u, 2.7 * u, 0.45 * u, hex(PAD_CAP), 1.0);
    layer.disc(dp.0, dp.1, 0.25 * u, hex(PAD_DARK), 1.0);
    for b in [PadButton::North, PadButton::East, PadButton::South, PadButton::West] {
        let (bx, by) = p(Part::Button(b).anchor());
        face_button(layer, kind, b, device, bx, by, 0.68 * u);
    }
    let (home, view, menu) = (p(legend::HOME), p(legend::VIEW), p(legend::MENU));
    layer.disc(home.0, home.1, 0.7 * u + lw * 0.5, hex(PAD_SHINE), 1.0);
    layer.disc(home.0, home.1, 0.7 * u, hex(PAD_DARK), 1.0);
    for b in [view, menu] {
        layer.disc(b.0, b.1, 0.42 * u + lw * 0.5, hex(PAD_EDGE), 1.0);
        layer.disc(b.0, b.1, 0.42 * u, hex(PAD_DARK), 1.0);
    }
    let thin = (u * 0.06).max(1.0);
    let (vx, vy) = (view.0 - 0.2 * u, view.1 - 0.14 * u);
    for (a, b) in [
        ((vx, vy), (vx + 0.3 * u, vy)),
        ((vx + 0.3 * u, vy), (vx + 0.3 * u, vy + 0.22 * u)),
        ((vx, vy + 0.22 * u), (vx + 0.3 * u, vy + 0.22 * u)),
        ((vx, vy), (vx, vy + 0.22 * u)),
    ] {
        layer.segment(a, b, thin, hex(PAD_SHINE), 1.0);
    }
    for k in 0..3 {
        let y = menu.1 - 0.12 * u + k as f32 * 0.12 * u;
        layer.segment((menu.0 - 0.18 * u, y), (menu.0 + 0.18 * u, y), thin, hex(PAD_SHINE), 1.0);
    }
    [cx, cy - 6.6 * u]
}

/// A face button, `r` in radius about (`cx`, `cy`): Xbox's coloured letter, PlayStation's symbol
/// on dark plastic.
fn face_button(layer: &mut Layer, kind: &Type, b: PadButton, device: Device, cx: f32, cy: f32, r: f32) {
    let color = hex(crate::render::hud::glyphs::face_color(b, device));
    if device == Device::PlayStation {
        layer.disc(cx, cy, r, hex(PAD_SHINE), 1.0);
        layer.disc(cx, cy, r - (r * 0.12).max(1.0), hex(0x2a2c33), 1.0);
        let (m, sw) = (r * 0.48, (r * 0.16).max(1.2));
        match b {
            PadButton::South => {
                layer.segment((cx - m, cy - m), (cx + m, cy + m), sw, color, 1.0);
                layer.segment((cx - m, cy + m), (cx + m, cy - m), sw, color, 1.0);
            }
            PadButton::East => layer.ring(cx, cy, m, sw, color, 1.0),
            PadButton::West => {
                let q = m * 0.85;
                for (a, z) in [((-q, -q), (q, -q)), ((q, -q), (q, q)), ((q, q), (-q, q)), ((-q, q), (-q, -q))] {
                    layer.segment((cx + a.0, cy + a.1), (cx + z.0, cy + z.1), sw, color, 1.0);
                }
            }
            _ => {
                let (a, b2, c2) = ((cx, cy - m * 1.05), (cx - m * 1.05, cy + m * 0.75), (cx + m * 1.05, cy + m * 0.75));
                layer.segment(a, b2, sw, color, 1.0);
                layer.segment(b2, c2, sw, color, 1.0);
                layer.segment(c2, a, sw, color, 1.0);
            }
        }
    } else {
        layer.disc(cx, cy, r, hex(0x101114), 1.0);
        layer.disc(cx, cy, r - (r * 0.1).max(1.0), color, 1.0);
        let letter = match b {
            PadButton::South => "A",
            PadButton::East => "B",
            PadButton::West => "X",
            _ => "Y",
        };
        kind.centred(layer, Face::Body, letter, r * 1.25, cx, cy, 0.0, hex(0x101114), 1.0);
    }
}

/// A part's glyph's name: the button's as `device` names it, the sticks' letters, the D-pad's.
fn part_name(part: Part, device: Device) -> String {
    match part {
        Part::Button(b) => crate::input::control_name(Control::Pad(b), device).into_owned(),
        Part::LeftStick => "L".into(),
        Part::RightStick => "R".into(),
        Part::DPad => "D-pad".into(),
    }
}

fn part_width(kind: &Type, part: Part, device: Device, h: f32) -> f32 {
    match part {
        Part::Button(PadButton::North | PadButton::East | PadButton::South | PadButton::West) | Part::LeftStick | Part::RightStick => h,
        _ => kind.width(Face::Body, &part_name(part, device), h * 0.5, 0.0) + h * 0.7,
    }
}

/// A part's glyph `h` tall, its left edge at `x`, centred on `y`.
#[allow(clippy::too_many_arguments)]
fn part_glyph(layer: &mut Layer, kind: &Type, part: Part, device: Device, x: f32, y: f32, h: f32) {
    match part {
        Part::Button(b @ (PadButton::North | PadButton::East | PadButton::South | PadButton::West)) => {
            face_button(layer, kind, b, device, x + h / 2.0, y, h / 2.0);
        }
        Part::LeftStick | Part::RightStick => {
            let (cx, r) = (x + h / 2.0, h / 2.0);
            layer.disc(cx, y, r, hex(PAD_SHINE), 1.0);
            layer.disc(cx, y, r - (r * 0.12).max(1.0), hex(0x2a2c33), 1.0);
            kind.centred(layer, Face::Body, &part_name(part, device), h * 0.45, cx, y, 0.0, hex(0xf0ece0), 1.0);
        }
        _ => {
            let w = part_width(kind, part, device, h);
            layer.round_rect(x, y - h * 0.4, w, h * 0.8, h * 0.3, hex(PAD_SHINE), 1.0);
            layer.round_rect(x + 1.2, y - h * 0.4 + 1.2, w - 2.4, h * 0.8 - 2.4, h * 0.3 - 1.2, hex(0x2a2c33), 1.0);
            kind.centred(layer, Face::Body, &part_name(part, device), h * 0.5, x + w / 2.0, y, 0.0, hex(0xf0ece0), 1.0);
        }
    }
}

/// A key or mouse control as a glyph `h` tall from `x`, centred on `y`: its width.
fn key_glyph(layer: &mut Layer, kind: &Type, c: Control, x: f32, y: f32, h: f32) -> f32 {
    match c {
        Control::Key(k) => {
            let name = crate::input::key_name(k);
            let w = (kind.width(Face::Body, &name, h * 0.5, 0.0) + h * 0.55).max(h * 0.9);
            layer.round_rect(x, y - h / 2.0, w, h, h * 0.18, hex(0x8a7a5a), 1.0);
            layer.round_rect(x + 1.2, y - h / 2.0 + 1.2, w - 2.4, h - 3.4, h * 0.16, hex(KEYCAP), 1.0);
            kind.centred(layer, Face::Body, &name, h * 0.5, x + w / 2.0, y - h * 0.04, 0.0, hex(KEY_LABEL), 1.0);
            w
        }
        Control::Mouse(_) | Control::WheelUp | Control::WheelDown => {
            mouse_glyph(layer, x, y, h, Some(c));
            h * 0.72
        }
        Control::Pad(_) => 0.0,
    }
}

/// The mouse, its button (or wheel) lit, or none: it moves.
fn mouse_glyph(layer: &mut Layer, x: f32, y: f32, h: f32, lit: Option<Control>) {
    let (w, top) = (h * 0.72, y - h / 2.0);
    layer.round_rect(x, top, w, h, w * 0.45, hex(0x8a7a5a), 1.0);
    layer.round_rect(x + 1.2, top + 1.2, w - 2.4, h - 2.4, w * 0.45 - 1.2, hex(KEYCAP), 1.0);
    let gold = hex(0xffc35a);
    match lit {
        Some(Control::Mouse(MouseButton::Left)) => layer.round_rect(x + 1.2, top + 1.2, w / 2.0 - 1.2, h * 0.42, w * 0.3, gold, 1.0),
        Some(Control::Mouse(MouseButton::Right)) => layer.round_rect(x + w / 2.0, top + 1.2, w / 2.0 - 1.2, h * 0.42, w * 0.3, gold, 1.0),
        _ => {}
    }
    layer.segment((x + w / 2.0, top + 1.0), (x + w / 2.0, top + h * 0.42), 1.0, hex(0x8a7a5a), 1.0);
    let wheel = matches!(lit, Some(Control::WheelUp | Control::WheelDown | Control::Mouse(MouseButton::Middle)));
    layer.round_rect(x + w * 0.4, top + h * 0.14, w * 0.2, h * 0.2, w * 0.1, if wheel { gold } else { hex(0x8a7a5a) }, 1.0);
}

/// The keyboard and mouse's column from (`x`, `y`) down, `width` wide, text `size` tall.
fn draw_keys(layer: &mut Layer, kind: &Type, bindings: &Bindings, x: f32, y: f32, width: f32, size: f32) {
    let glyph = size * 1.35;
    let row_h = glyph * 1.3;
    let label_x = x + width * 0.42;
    let text = hex(TEXT);
    let mut y = y + row_h * 0.5;
    mouse_glyph(layer, x, y, glyph, None);
    kind.draw(layer, Face::Body, "Camera", size, label_x, y, 0.0, text, 1.0);
    y += row_h;
    for row in legend::key_rows(bindings) {
        let mut pen = x;
        let range = row.actions.len() == 5 && row.controls.len() == 2;
        for (k, &c) in row.controls.iter().enumerate() {
            if k > 0 && range {
                pen = kind.draw(layer, Face::Body, "-", size, pen + size * 0.2, y, 0.0, hex(SAND), 1.0) + size * 0.2;
            }
            if pen + glyph > label_x - size * 0.4 {
                break;
            }
            pen += key_glyph(layer, kind, c, pen, y, glyph) + size * 0.25;
        }
        kind.draw(layer, Face::Body, &row.does, size, label_x, y, 0.0, text, 1.0);
        y += row_h;
    }
}

/// One frame of the loading screen, `size` pixels, at `step` of the checklist with the bar
/// `progress` along and `time` seconds in, read back as tightly packed RGBA8 (`--splash`). The
/// legend shows `device` when given, else the device used last as saved. With the legend's
/// setting time, in milliseconds.
pub fn capture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    size: (u32, u32),
    step: u32,
    progress: f32,
    time: f32,
    shown: Option<Device>,
) -> Option<(Vec<u8>, f64)> {
    let (w, h) = (size.0.max(1), size.1.max(1));
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut s = Splash::new(device, queue, format, (w, h))?;
    if let Some(d) = shown {
        s.device = d;
    }
    s.prepare(queue, step, (progress * 100.0).round() as u32);
    s.write_uniforms(queue, time, progress);
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("splash capture"),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = tex.create_view(&Default::default());
    let row = (w * 4).div_ceil(256) * 256;
    let buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("splash readback"),
        size: (row * h) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&Default::default());
    s.pass(&mut enc, &view);
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        wgpu::TexelCopyBufferInfo {
            buffer: &buf,
            layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(h) },
        },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
    queue.submit([enc.finish()]);
    let slice = buf.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    let _ = device.poll(wgpu::PollType::Wait);
    let data = slice.get_mapped_range();
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        let line = &data[(y * row) as usize..(y * row + w * 4) as usize];
        for px in line.chunks(4) {
            out.extend_from_slice(&[px[0], px[1], px[2], 255]);
        }
    }
    Some((out, s.legend_ms))
}

/// The loading screen, drawing on its own thread.
pub struct Running {
    stop: Arc<AtomicBool>,
    handle: std::thread::JoinHandle<wgpu::Surface<'static>>,
}

impl Running {
    /// Ends it (after a last moment with its bar full) and hands the surface back.
    pub fn finish(self) -> wgpu::Surface<'static> {
        self.stop.store(true, Ordering::Relaxed);
        self.handle.join().expect("the loading screen's thread")
    }
}
