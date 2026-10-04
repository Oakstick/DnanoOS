//! The engine's loading screen, shown from the moment the window opens until the renderer is
//! ready. The shaders take seconds to compile on DX12, so the screen draws itself on its own
//! thread meanwhile, while the renderer reports which step it is on through a `Progress`.
//!
//! The backdrop is `assets/engine/splash.png` (16:9 art with the logo and title in it), read
//! at start-up from next to the executable, else the working directory, else the source tree,
//! so it can be replaced without a rebuild. Without it, a dark gradient with the engine's name
//! set in type stands in. Over either, live:
//! - a fade in, a slow push in, and the logo breathing light with a sheen sweeping over it;
//! - a progress bar with its percentage, timed against how long the last start took;
//! - the step it is on, and the checklist of steps: done, under way (a spinner), to come.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ab_glyph::{point, Font, FontRef, PxScale, ScaleFont};

/// The engine's name, set in type when there is no backdrop art.
const ENGINE: (&str, &str) = ("Distinguisher", "Nano");
const STRAPLINE: &str = "INITIALIZING REAL-TIME AUTHORING ENGINE";
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
/// What the layer's dynamic band covers (the rest of it, the title, is drawn once).
const BAND: (f32, f32) = (0.725, 0.875);

/// How long a start takes before any has been timed, in seconds.
const FIRST_GUESS: f32 = 10.0;
const TIMED_KEY: &str = "engine_start_secs";

const WHITE: [f32; 3] = [0.9, 0.92, 0.95];
const SOFT: [f32; 3] = [0.72, 0.76, 0.8];
const DIM: [f32; 3] = [0.42, 0.47, 0.53];
const CYAN: [f32; 3] = [0.38, 0.78, 1.0];
const DONE: [f32; 3] = [0.6, 0.78, 0.92];

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

/// Type, set with ab_glyph.
struct Type {
    font: FontRef<'static>,
}

impl Type {
    fn width(&self, text: &str, size: f32, tracking: f32) -> f32 {
        let sf = self.font.as_scaled(PxScale::from(size));
        let mut w = 0.0;
        let mut prev = None;
        for c in text.chars() {
            let id = self.font.glyph_id(c);
            if let Some(p) = prev {
                w += sf.kern(p, id);
            }
            w += sf.h_advance(id) + tracking;
            prev = Some(id);
        }
        w - tracking
    }

    /// Sets `text` from `x`, centred on the height `cy`; returns where it ends.
    #[allow(clippy::too_many_arguments)]
    fn draw(&self, layer: &mut Layer, text: &str, size: f32, x: f32, cy: f32, tracking: f32, rgb: [f32; 3], alpha: f32) -> f32 {
        let scale = PxScale::from(size);
        let sf = self.font.as_scaled(scale);
        // The cap height's middle on `cy`.
        let baseline = cy + sf.ascent() * 0.36;
        let mut x = x;
        let mut prev = None;
        for c in text.chars() {
            let id = self.font.glyph_id(c);
            if let Some(p) = prev {
                x += sf.kern(p, id);
            }
            if let Some(o) = self.font.outline_glyph(id.with_scale_and_position(scale, point(x, baseline))) {
                let b = o.px_bounds();
                o.draw(|gx, gy, cov| layer.put(b.min.x as i32 + gx as i32, b.min.y as i32 + gy as i32, rgb, cov * alpha));
            }
            x += sf.h_advance(id) + tracking;
            prev = Some(id);
        }
        x
    }

    #[allow(clippy::too_many_arguments)]
    fn centred(&self, layer: &mut Layer, text: &str, size: f32, cx: f32, cy: f32, tracking: f32, rgb: [f32; 3], alpha: f32) {
        let w = self.width(text, size, tracking);
        self.draw(layer, text, size, cx - w / 2.0, cy, tracking, rgb, alpha);
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
    /// The loading screen for a `size` target of `format` (None if its type will not load).
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat, size: (u32, u32)) -> Option<Self> {
        let font = FontRef::try_from_slice(epaint_default_fonts::UBUNTU_LIGHT).ok()?;
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
        let mut splash = Self {
            pipe,
            bind,
            uniforms,
            layer_tex,
            layer: Layer { w: w as usize, h: h as usize, px: vec![[0.0; 4]; (w * h) as usize] },
            kind: Type { font },
            img_aspect: aw as f32 / ah as f32,
            has_art,
            size: (w, h),
            spin: [0.0; 4],
        };
        // The engine's name, set once, when there is no art with it in.
        if !has_art {
            splash.draw_title();
        }
        splash.draw_band(0, 0);
        let all: Vec<u8> = splash.layer.px.iter().flat_map(|p| p.map(|c| (c.clamp(0.0, 1.0) * 255.0 + 0.5) as u8)).collect();
        upload(queue, &splash.layer_tex, w, 0, h, &all);
        Some(splash)
    }

    fn scale(&self) -> f32 {
        self.size.1 as f32 / REF_H
    }

    fn draw_title(&mut self) {
        let (w, h, s) = (self.size.0 as f32, self.size.1 as f32, self.scale());
        let size = 66.0 * s;
        let (a, b) = ENGINE;
        let gap = size * 0.25;
        let total = self.kind.width(a, size, 0.0) + gap + self.kind.width(b, size, 0.0);
        let x = w / 2.0 - total / 2.0;
        let end = self.kind.draw(&mut self.layer, a, size, x, h * 0.555, 0.0, WHITE, 1.0);
        self.kind.draw(&mut self.layer, b, size, end + gap, h * 0.555, 0.0, [0.5, 0.74, 1.0], 1.0);
        self.kind.centred(&mut self.layer, STRAPLINE, 13.0 * s, w / 2.0, h * 0.618, 13.0 * s * 0.5, SOFT, 0.85);
    }

    /// The band under the bar: its percentage, the step under way, and the checklist.
    fn draw_band(&mut self, step: u32, percent: u32) {
        let (w, h, s) = (self.size.0 as f32, self.size.1 as f32, self.scale());
        self.layer.clear_rows((h * BAND.0) as usize, (h * BAND.1) as usize);
        let kind = &self.kind;
        let layer = &mut self.layer;
        kind.draw(layer, &format!("{percent}%"), 15.0 * s, w * (BAR.1 + 0.014), h * BAR.2, 0.0, SOFT, 1.0);
        let status = match STEPS.get(step as usize) {
            Some(name) => format!("{name}..."),
            None => "Ready".to_string(),
        };
        kind.centred(layer, &status, 17.0 * s, w / 2.0, h * STATUS_Y, 0.4 * s, WHITE, 1.0);
        // The checklist: each step's icon and name, a short rule between them.
        let (text, r, gap, rule) = (13.5 * s, 11.0 * s, 10.0 * s, 22.0 * s);
        let widths: Vec<f32> = STEPS.iter().map(|t| 2.0 * r + gap + kind.width(t, text, 0.0)).collect();
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
            let end = kind.draw(layer, name, text, cx + r + gap, cy, 0.0, label, alpha);
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

    /// Draws one frame at `time` seconds in, `progress` along. False when the surface had no
    /// frame to give.
    fn draw(&self, device: &wgpu::Device, queue: &wgpu::Queue, surface: &wgpu::Surface, time: f32, progress: f32) -> bool {
        let Ok(frame) = surface.get_current_texture() else { return false };
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
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("splash") });
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("splash"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
            // The bar is timed against the last start (the compile is one long step, so the
            // steps alone cannot move it smoothly), never passing 96% until it is done.
            let expected = crate::platform::load(TIMED_KEY).and_then(|v| v.parse::<f32>().ok()).filter(|t| *t > 0.5).unwrap_or(FIRST_GUESS);
            let mut painted = Some(painted);
            let mut shown = 0.0f32;
            let mut done_at: Option<Instant> = None;
            let mut drawn = (u32::MAX, u32::MAX);
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
