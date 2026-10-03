//! wgpu backend: WebGPU in the browser, Direct3D 12 / Vulkan / Metal on desktop.
//!
//! A frame is: a shadow map from the moon (or sun); the scene, into color plus
//! a buffer of normals and depth; contact shading from that buffer; a glow
//! chain; a composite with background blur and grading onto the screen; and
//! the HUD on top.

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};
use wgpu::util::DeviceExt;

use super::faces::StaticFaces;
use super::overlay::{build_atlas, OVertex, ATLAS_H, ATLAS_W};
use super::profile::{fx, FrameCounts, GpuTimer};
use super::view::{Frame, Instance, InstanceSet, Light, MeshKind, RigDraw, WorldView, FAR, MESH_KINDS};
use super::voxels::{RigMesh, RigVertex};

const MAX_LIGHTS: usize = 32;
const SAMPLES: u32 = 4;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// The scene's color: light itself (linear HDR, times the exposure), tone mapped by the composite.
const COLOR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// The scene shader's tone curve (ACES filmic as Three.js fits it), exposure 1, for linear light.
fn aces_filmic(c: Vec3) -> Vec3 {
    let input = glam::Mat3::from_cols(
        Vec3::new(0.59719, 0.07600, 0.02840),
        Vec3::new(0.35458, 0.90834, 0.13383),
        Vec3::new(0.04823, 0.01566, 0.83777),
    );
    let output = glam::Mat3::from_cols(
        Vec3::new(1.60475, -0.10208, -0.00327),
        Vec3::new(-0.53108, 1.10813, -0.07276),
        Vec3::new(-0.07367, -0.00605, 1.07602),
    );
    let v = input * (c / 0.6);
    let a = v * (v + Vec3::splat(0.0245786)) - Vec3::splat(0.000090537);
    let b = v * (v * 0.983729 + Vec3::splat(0.432951)) + Vec3::splat(0.238081);
    (output * (a / b)).clamp(Vec3::ZERO, Vec3::ONE)
}

/// The light that tone maps to linear display colour `t` (as the shader's `untonemap`).
fn untonemap(t: Vec3) -> Vec3 {
    let goal = t.clamp(Vec3::ZERO, Vec3::splat(0.97));
    let mut x = goal;
    for _ in 0..24 {
        x *= (goal / aces_filmic(x).max(Vec3::splat(1e-5))).clamp(Vec3::splat(0.25), Vec3::splat(4.0));
    }
    x
}
/// World normal and view depth, and the glow chain.
const HDR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const AO: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;
const SHADOW_SIZE: u32 = if cfg!(target_arch = "wasm32") { 2048 } else { 4096 };
/// Half the width of the fine shadow cascade, in world units.
const NEAR_CASCADE: f32 = 10.0;
const BLOOM_LEVELS: usize = 5;
/// Static scenery is cut into square chunks this many world units across, culled as a whole.
const CHUNK: f32 = 8.0;
/// Spare static slots for cubes a level adds as it is played (floor patches, scorch marks).
const STATIC_SPARE: usize = 8192;

/// A cube's open faces (bits from `StaticFaces`) as face records: its slot times 8 plus the face.
fn push_faces(out: &mut Vec<u32>, slot: u32, open: u8) {
    for f in 0..6 {
        if open & (1 << f) != 0 {
            out.push(slot << 3 | f);
        }
    }
}

/// A run of static instances near one another, the run of their open faces, and the box they fill.
#[derive(Clone, Copy, Debug)]
struct StaticChunk {
    start: u32,
    count: u32,
    faces: u32,
    face_count: u32,
    min: Vec3,
    max: Vec3,
}

/// The six planes of a view-projection's clip volume (depth 0..1), facing inward.
struct Frustum([Vec4; 6]);

impl Frustum {
    fn new(m: Mat4) -> Self {
        let r = [m.row(0), m.row(1), m.row(2), m.row(3)];
        Self([r[3] + r[0], r[3] - r[0], r[3] + r[1], r[3] - r[1], r[2], r[3] - r[2]])
    }

    /// False only when the box lies wholly outside one plane.
    fn sees(&self, min: Vec3, max: Vec3) -> bool {
        self.0.iter().all(|p| {
            let v = Vec3::new(
                if p.x >= 0.0 { max.x } else { min.x },
                if p.y >= 0.0 { max.y } else { min.y },
                if p.z >= 0.0 { max.z } else { min.z },
            );
            p.truncate().dot(v) + p.w >= 0.0
        })
    }
}

/// An instanced unit cube's center and half extent along each world axis.
fn cube_bounds(c: &Instance) -> (Vec3, Vec3) {
    let m = &c.m;
    let a = |i: usize| Vec3::new(m[i].abs(), m[i + 1].abs(), m[i + 2].abs());
    (Vec3::new(m[12], m[13], m[14]), (a(0) + a(4) + a(8)) * 0.5)
}

/// Orders the static instances chunk by chunk (stably, so a tile's run stays a run) and bounds
/// each chunk. Instances too big for a chunk share one of their own; hidden ones (scaled to
/// nothing) widen no box.
fn chunk_static(cubes: &[Instance]) -> (Vec<u32>, Vec<StaticChunk>) {
    let mut keyed: Vec<((i32, i32), u32)> = cubes
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let (at, half) = cube_bounds(c);
            let key = if half.x.max(half.z) > CHUNK * 0.5 {
                (i32::MAX, i32::MAX)
            } else {
                ((at.x / CHUNK).floor() as i32, (at.z / CHUNK).floor() as i32)
            };
            (key, i as u32)
        })
        .collect();
    keyed.sort_by_key(|k| k.0);
    let mut chunks: Vec<StaticChunk> = Vec::new();
    let mut last = None;
    for (slot, (key, i)) in keyed.iter().enumerate() {
        if last != Some(*key) {
            last = Some(*key);
            chunks.push(StaticChunk {
                start: slot as u32,
                count: 0,
                faces: 0,
                face_count: 0,
                min: Vec3::INFINITY,
                max: Vec3::NEG_INFINITY,
            });
        }
        let ch = chunks.last_mut().unwrap();
        ch.count += 1;
        let (at, half) = cube_bounds(&cubes[*i as usize]);
        if half.max_element() > 0.0 {
            ch.min = ch.min.min(at - half);
            ch.max = ch.max.max(at + half);
        }
    }
    chunks.retain(|c| c.min.x <= c.max.x);
    (keyed.into_iter().map(|k| k.1).collect(), chunks)
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuLight {
    pos: [f32; 4],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    view_proj: [f32; 16],
    light_vp: [f32; 16],
    light_vp_near: [f32; 16],
    cam_pos: [f32; 4],
    cam_fwd: [f32; 4],
    fog: [f32; 4],
    params: [f32; 4],
    hemi_sky: [f32; 4],
    hemi_ground: [f32; 4],
    ambient: [f32; 4],
    moon_dir: [f32; 4],
    moon_color: [f32; 4],
    sky_fwd: [f32; 4],
    sky_right: [f32; 4],
    sky_up: [f32; 4],
    sky_moon: [f32; 4],
    bounds: [f32; 4],
    shadow: [f32; 4],
    lights: [GpuLight; MAX_LIGHTS],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PostUniform {
    view_proj: [f32; 16],
    cam_pos: [f32; 4],
    cam_fwd: [f32; 4],
    cam_r: [f32; 4],
    cam_u: [f32; 4],
    params: [f32; 4],
    params2: [f32; 4],
    moon: [f32; 4],
}

struct MeshBuf {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
}

/// A rig's mesh on the GPU, and the frame it was last drawn in.
struct RigBuf {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
    used: u64,
}

/// A prebuilt instance set on the GPU, and the frame it was last drawn in.
struct SetBuf {
    buf: wgpu::Buffer,
    count: u32,
    used: u64,
}

/// Rig meshes not drawn for this many frames are let go.
const RIG_IDLE_FRAMES: u64 = 600;
/// The most rigs one frame can draw (the draw-id buffer's length).
const MAX_RIG_DRAWS: u32 = 4096;

/// A growable GPU buffer of instances or vertices.
struct Dyn {
    buf: wgpu::Buffer,
    cap: u64,
    usage: wgpu::BufferUsages,
    label: &'static str,
}

impl Dyn {
    fn new(device: &wgpu::Device, label: &'static str, usage: wgpu::BufferUsages, cap: u64) -> Self {
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: cap,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { buf, cap, usage, label }
    }

    /// Makes room for at least `bytes`, dropping what was there if it must grow.
    fn reserve(&mut self, device: &wgpu::Device, bytes: u64) {
        if bytes > self.cap {
            *self = Dyn::new(device, self.label, self.usage, bytes.next_power_of_two());
        }
    }

    fn write(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        if bytes.len() as u64 > self.cap {
            let cap = (bytes.len() as u64).next_power_of_two();
            *self = Dyn::new(device, self.label, self.usage, cap);
        }
        queue.write_buffer(&self.buf, 0, bytes);
    }
}

/// Everything sized to the window, rebuilt on resize.
struct Targets {
    msaa_color: wgpu::TextureView,
    msaa_gbuf: wgpu::TextureView,
    depth: wgpu::TextureView,
    color: wgpu::TextureView,
    gbuf: wgpu::TextureView,
    ao: wgpu::TextureView,
    ao_blur: wgpu::TextureView,
    bloom: Vec<wgpu::TextureView>,
    ssao_bind: wgpu::BindGroup,
    blur_bind: wgpu::BindGroup,
    rays_bind: wgpu::BindGroup,
    /// [0] reads the scene color; [i] reads bloom[i - 1].
    down_binds: Vec<wgpu::BindGroup>,
    /// [i] reads bloom[i + 1], to add into bloom[i].
    up_binds: Vec<wgpu::BindGroup>,
    composite_bind: wgpu::BindGroup,
}

/// What the post passes share: their bind group layout, uniform, sampler and a blank texture.
struct PostShared {
    layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
    sampler: wgpu::Sampler,
    blank: wgpu::TextureView,
}

impl PostShared {
    fn bind(&self, device: &wgpu::Device, label: &str, t: [&wgpu::TextureView; 4]) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.uniform.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(t[0]) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(t[1]) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(t[2]) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(t[3]) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        })
    }
}

fn texture(
    device: &wgpu::Device,
    label: &str,
    (w, h): (u32, u32),
    format: wgpu::TextureFormat,
    samples: u32,
    sampled: bool,
) -> wgpu::TextureView {
    let mut usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
    if sampled {
        usage |= wgpu::TextureUsages::TEXTURE_BINDING;
    }
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: samples,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

impl Targets {
    fn new(device: &wgpu::Device, post: &PostShared, w: u32, h: u32) -> Self {
        let full = (w, h);
        let half = ((w / 2).max(1), (h / 2).max(1));
        let color = texture(device, "color", full, COLOR, 1, true);
        let gbuf = texture(device, "gbuf", full, HDR, 1, true);
        let ao = texture(device, "ao", half, AO, 1, true);
        let ao_blur = texture(device, "ao blur", half, AO, 1, true);
        let bloom: Vec<_> =
            (0..BLOOM_LEVELS).map(|i| texture(device, "bloom", ((w >> (i + 1)).max(1), (h >> (i + 1)).max(1)), HDR, 1, true)).collect();
        let b = &post.blank;
        let ssao_bind = post.bind(device, "ssao", [&gbuf, b, b, b]);
        let blur_bind = post.bind(device, "blur", [&ao, &gbuf, b, b]);
        let rays_bind = post.bind(device, "rays", [&color, &gbuf, b, b]);
        let mut down_binds = vec![post.bind(device, "bloom pre", [&color, b, b, b])];
        for i in 1..BLOOM_LEVELS {
            down_binds.push(post.bind(device, "bloom down", [&bloom[i - 1], b, b, b]));
        }
        let up_binds = (0..BLOOM_LEVELS - 1).map(|i| post.bind(device, "bloom up", [&bloom[i + 1], b, b, b])).collect();
        let composite_bind = post.bind(device, "composite", [&color, &gbuf, &ao_blur, &bloom[0]]);
        Self {
            msaa_color: texture(device, "msaa color", full, COLOR, SAMPLES, false),
            msaa_gbuf: texture(device, "msaa gbuf", full, HDR, SAMPLES, false),
            depth: texture(device, "depth", full, DEPTH, SAMPLES, false),
            color,
            gbuf,
            ao,
            ao_blur,
            bloom,
            ssao_bind,
            blur_bind,
            rays_bind,
            down_binds,
            up_binds,
            composite_bind,
        }
    }
}

pub struct Gpu {
    pub surface: Option<wgpu::Surface<'static>>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub format: wgpu::TextureFormat,
    rt: Targets,
    post: PostShared,
    /// One layer of the shadow map per cascade: the fine one round Ragnar, then the whole level.
    shadow_layers: [wgpu::TextureView; 2],
    shadow_tex: wgpu::Texture,
    /// The coarse cascade holding only the static scenery, kept while the light, the level and the
    /// cascade's box stay the same (the static generation and the matrix it was drawn with).
    shadow_cache: wgpu::Texture,
    shadow_cache_view: wgpu::TextureView,
    shadow_cache_key: Option<(u64, [f32; 16])>,
    shadow_pipe: wgpu::RenderPipeline,
    shadow_binds: [wgpu::BindGroup; 2],
    cascade_bufs: [wgpu::Buffer; 2],
    scene_opaque: wgpu::RenderPipeline,
    scene_prepass: wgpu::RenderPipeline,
    rig_opaque: wgpu::RenderPipeline,
    rig_blend: wgpu::RenderPipeline,
    rig_prepass: wgpu::RenderPipeline,
    rig_shadow: wgpu::RenderPipeline,
    rig_layout: wgpu::BindGroupLayout,
    /// The resident rig meshes by rig id, this frame's draw records and part matrices, and
    /// 0, 1, 2, ... as an instance stream so each draw finds its record.
    rig_meshes: std::collections::HashMap<u64, RigBuf>,
    rig_draw_buf: Dyn,
    rig_part_buf: Dyn,
    rig_ids: wgpu::Buffer,
    frame_no: u64,
    /// Prebuilt instance sets by id (`view::InstanceSet`).
    sets: std::collections::HashMap<u64, SetBuf>,
    scene_blend: wgpu::RenderPipeline,
    scene_add: wgpu::RenderPipeline,
    sky_pipe: wgpu::RenderPipeline,
    ssao_pipe: wgpu::RenderPipeline,
    blur_pipe: wgpu::RenderPipeline,
    bloom_pre_pipe: wgpu::RenderPipeline,
    bloom_down_pipe: wgpu::RenderPipeline,
    bloom_up_pipe: wgpu::RenderPipeline,
    rays_pipe: wgpu::RenderPipeline,
    composite_pipe: wgpu::RenderPipeline,
    overlay_pipe: wgpu::RenderPipeline,
    globals: wgpu::Buffer,
    scene_bind: wgpu::BindGroup,
    screen: wgpu::Buffer,
    overlay_bind: wgpu::BindGroup,
    meshes: Vec<MeshBuf>,
    static_buf: Dyn,
    static_count: u32,
    /// The static cubes' open faces (slot times 8 plus the face), chunk by chunk; the store's bind
    /// group; one quad's indices; and the pipelines that draw faces.
    face_buf: Dyn,
    /// The open-face index of the static cubes, the chunked faces as first sent, and the faces
    /// opened or added since (drawn as one more run, after the chunks), with the box they lie in.
    static_faces: Option<StaticFaces>,
    faces_main: Vec<u32>,
    extra_faces: Vec<u32>,
    extra_box: (Vec3, Vec3),
    static_layout: wgpu::BindGroupLayout,
    static_bind: Option<wgpu::BindGroup>,
    quad: wgpu::Buffer,
    face_prepass: wgpu::RenderPipeline,
    face_opaque: wgpu::RenderPipeline,
    face_shadow: wgpu::RenderPipeline,
    static_chunks: Vec<StaticChunk>,
    /// Where each of the view's static instances sits in the chunk-ordered buffer.
    static_slot: Vec<u32>,
    /// Bumped on every full static upload.
    static_gen: u64,
    /// Bytes uploaded since the last frame was drawn.
    pending_upload: u64,
    /// GPU time per pass, when the adapter can time them.
    pub timer: Option<GpuTimer>,
    /// What the last frame drew.
    pub counts: FrameCounts,
    /// Effects switched off (`profile::fx` bits), to see what each costs and adds.
    pub fx_off: u32,
    /// The last frame handed to the queue, for pacing frames in flight.
    pub last_submit: Option<wgpu::SubmissionIndex>,
    dyn_buf: Dyn,
    overlay_buf: Dyn,
    pub backend: String,
    pub adapter: wgpu::AdapterInfo,
}

impl Gpu {
    /// Opens the GPU on a window surface. `size` is in physical pixels.
    pub async fn new(window: Option<Arc<winit::window::Window>>, size: (u32, u32)) -> Result<Self, String> {
        // Direct3D 12 first on Windows: it is what the Steam overlay and capture
        // tools know best. Anything else (or WGPU_BACKEND) as the fallback.
        let preferred = if cfg!(windows) { wgpu::Backends::DX12 } else { wgpu::Backends::all() };
        let order = match wgpu::Backends::from_env() {
            Some(b) => vec![b],
            None => vec![preferred, wgpu::Backends::all()],
        };
        let mut found = None;
        for backends in order {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor { backends, ..Default::default() });
            let surface = match &window {
                Some(w) => match instance.create_surface(w.clone()) {
                    Ok(s) => Some(s),
                    Err(_) => continue,
                },
                None => None,
            };
            let request = instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: surface.as_ref(),
                force_fallback_adapter: false,
            });
            if let Ok(adapter) = request.await {
                found = Some((surface, adapter));
                break;
            }
        }
        let (surface, adapter) = found.ok_or_else(|| "no GPU adapter supports this window".to_string())?;
        let info = adapter.get_info();
        let backend = format!("{:?} {}", info.backend, info.name);
        let adapter_info = info.clone();
        let limits = if cfg!(target_arch = "wasm32") {
            wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())
        } else {
            wgpu::Limits::default().using_resolution(adapter.limits())
        };
        // Timestamps for the profiler, where the adapter has them.
        let features = adapter.features() & wgpu::Features::TIMESTAMP_QUERY;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("void-harvest"),
                required_features: features,
                required_limits: limits,
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|e| format!("device: {e}"))?;

        // The shader encodes sRGB itself (as Three.js does), so the target must not.
        let format = match &surface {
            Some(s) => {
                let caps = s.get_capabilities(&adapter);
                caps.formats.iter().copied().find(|f| !f.is_srgb()).unwrap_or(caps.formats[0].remove_srgb_suffix())
            }
            None => wgpu::TextureFormat::Rgba8Unorm,
        };
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.0.max(1),
            height: size.1.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };
        if let Some(s) = &surface {
            s.configure(&device, &config);
        }

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shaders"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders.wgsl").into()),
        });
        let post_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("post"),
            source: wgpu::ShaderSource::Wgsl(include_str!("post.wgsl").into()),
        });

        // ---- scene
        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_entry = |binding, visibility| wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
            count: None,
        };
        let shadow_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow map"),
            size: wgpu::Extent3d { width: SHADOW_SIZE, height: SHADOW_SIZE, depth_or_array_layers: 2 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let shadow_cache = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("static shadow cache"),
            size: wgpu::Extent3d { width: SHADOW_SIZE, height: SHADOW_SIZE, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let shadow_cache_view = shadow_cache.create_view(&Default::default());
        let shadow_view = shadow_tex
            .create_view(&wgpu::TextureViewDescriptor { dimension: Some(wgpu::TextureViewDimension::D2Array), ..Default::default() });
        let layer = |i: u32| {
            shadow_tex.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: i,
                array_layer_count: Some(1),
                ..Default::default()
            })
        };
        let shadow_layers = [layer(0), layer(1)];
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let scene_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene"),
            entries: &[
                uniform_entry(0, wgpu::ShaderStages::VERTEX_FRAGMENT),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });
        let scene_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene"),
            layout: &scene_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: globals.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&shadow_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&shadow_sampler) },
            ],
        });
        let scene_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene"),
            bind_group_layouts: &[&scene_layout],
            push_constant_ranges: &[],
        });
        // The shadow pass cannot bind the map it draws into: it gets its cascade's matrix alone.
        let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow"),
            entries: &[uniform_entry(3, wgpu::ShaderStages::VERTEX)],
        });
        let cascade_buf = || {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("cascade"),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let cascade_bufs = [cascade_buf(), cascade_buf()];
        let shadow_bind = |buf: &wgpu::Buffer| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("shadow"),
                layout: &shadow_layout,
                entries: &[wgpu::BindGroupEntry { binding: 3, resource: buf.as_entire_binding() }],
            })
        };
        let shadow_binds = [shadow_bind(&cascade_bufs[0]), shadow_bind(&cascade_bufs[1])];
        let shadow_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow"),
            bind_group_layouts: &[&shadow_layout],
            push_constant_ranges: &[],
        });
        let vertex_layouts = [
            wgpu::VertexBufferLayout {
                array_stride: 12,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x3],
            },
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Instance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![1 => Float32x4, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4],
            },
        ];
        let shadow_pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow"),
            layout: Some(&shadow_pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_shadow"),
                buffers: &vertex_layouts,
                compilation_options: Default::default(),
            },
            fragment: None,
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: wgpu::DepthBiasState { constant: 2, slope_scale: 2.5, clamp: 0.0 },
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        // Color, then normals and depth; blended passes leave the latter alone.
        let scene_targets = |blend: Option<wgpu::BlendState>, gbuf: bool| {
            [
                Some(wgpu::ColorTargetState { format: COLOR, blend, write_mask: wgpu::ColorWrites::ALL }),
                Some(wgpu::ColorTargetState {
                    format: HDR,
                    blend: None,
                    write_mask: if gbuf { wgpu::ColorWrites::ALL } else { wgpu::ColorWrites::empty() },
                }),
            ]
        };
        let scene_pipe = |label: &str, blend: Option<wgpu::BlendState>, depth_write: bool, fs: &str, writes: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&scene_pl),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_scene"),
                    buffers: &vertex_layouts,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fs),
                    targets: &if writes {
                        scene_targets(blend, depth_write)
                    } else {
                        [COLOR, HDR]
                            .map(|format| Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::empty() }))
                    },
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH,
                    depth_write_enabled: depth_write,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState { count: SAMPLES, ..Default::default() },
                multiview: None,
                cache: None,
            })
        };
        let alpha = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let additive = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        };

        // ---- resident rigs: their meshes, plus a record per draw and the parts' matrices
        let storage_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let rig_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rigs"),
            entries: &[storage_entry(0), storage_entry(1)],
        });
        let rig_scene_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rig scene"),
            bind_group_layouts: &[&scene_layout, &rig_layout],
            push_constant_ranges: &[],
        });
        let rig_shadow_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rig shadow"),
            bind_group_layouts: &[&shadow_layout, &rig_layout],
            push_constant_ranges: &[],
        });
        let rig_vertex_layouts = [
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<RigVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Snorm8x4, 2 => Float32x4, 3 => Uint32],
            },
            wgpu::VertexBufferLayout {
                array_stride: 4,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![4 => Uint32],
            },
        ];
        let rig_pipe = |label: &str, blend: Option<wgpu::BlendState>, depth_write: bool, fs: &str, writes: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&rig_scene_pl),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_rig"),
                    buffers: &rig_vertex_layouts,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fs),
                    targets: &if writes {
                        scene_targets(blend, depth_write)
                    } else {
                        [COLOR, HDR]
                            .map(|format| Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::empty() }))
                    },
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH,
                    depth_write_enabled: depth_write,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState { count: SAMPLES, ..Default::default() },
                multiview: None,
                cache: None,
            })
        };
        let rig_shadow = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rig shadow"),
            layout: Some(&rig_shadow_pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_rig_shadow"),
                buffers: &rig_vertex_layouts,
                compilation_options: Default::default(),
            },
            fragment: None,
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: wgpu::DepthBiasState { constant: 2, slope_scale: 2.5, clamp: 0.0 },
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        // ---- static scenery, face by face (see `open_faces`)
        let static_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("static store"), entries: &[storage_entry(0)] });
        let face_scene_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("face scene"),
            bind_group_layouts: &[&scene_layout, &static_layout],
            push_constant_ranges: &[],
        });
        let face_shadow_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("face shadow"),
            bind_group_layouts: &[&shadow_layout, &static_layout],
            push_constant_ranges: &[],
        });
        let face_layouts = [wgpu::VertexBufferLayout {
            array_stride: 4,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &wgpu::vertex_attr_array![0 => Uint32],
        }];
        let face_pipe = |label: &str, fs: &str, writes: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&face_scene_pl),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_face"),
                    buffers: &face_layouts,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fs),
                    targets: &if writes {
                        scene_targets(None, true)
                    } else {
                        [COLOR, HDR]
                            .map(|format| Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::empty() }))
                    },
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState { count: SAMPLES, ..Default::default() },
                multiview: None,
                cache: None,
            })
        };
        let face_shadow = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("face shadow"),
            layout: Some(&face_shadow_pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_face_shadow"),
                buffers: &face_layouts,
                compilation_options: Default::default(),
            },
            fragment: None,
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: wgpu::DepthBiasState { constant: 2, slope_scale: 2.5, clamp: 0.0 },
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        // Every pipeline that shades the scene compiles the whole scene shader, which takes the
        // Direct3D 12 shader compiler a good while each; they are built side by side.
        let (scene_opaque, scene_blend, scene_add, scene_prepass, rig_opaque, rig_blend, rig_prepass, face_opaque, face_prepass) =
            std::thread::scope(|s| {
                let a = s.spawn(|| scene_pipe("opaque", None, true, "fs_scene", true));
                let b = s.spawn(|| scene_pipe("transparent", Some(alpha), false, "fs_scene", true));
                let c = s.spawn(|| scene_pipe("additive", Some(additive), false, "fs_scene", true));
                let d = s.spawn(|| scene_pipe("depth prepass", None, true, "fs_prepass", false));
                let e = s.spawn(|| rig_pipe("rig opaque", None, true, "fs_scene", true));
                let f = s.spawn(|| rig_pipe("rig transparent", Some(alpha), false, "fs_scene", true));
                let g = s.spawn(|| rig_pipe("rig depth prepass", None, true, "fs_prepass", false));
                let h = s.spawn(|| face_pipe("face opaque", "fs_scene", true));
                let i = s.spawn(|| face_pipe("face depth prepass", "fs_prepass", false));
                let j = |t: std::thread::ScopedJoinHandle<'_, wgpu::RenderPipeline>| t.join().expect("pipeline thread");
                (j(a), j(b), j(c), j(d), j(e), j(f), j(g), j(h), j(i))
            });
        let quad = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad"),
            contents: bytemuck::cast_slice(&[0u16, 1, 2, 0, 2, 3]),
            usage: wgpu::BufferUsages::INDEX,
        });
        let ids: Vec<u32> = (0..MAX_RIG_DRAWS).collect();
        let rig_ids = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rig draw ids"),
            contents: bytemuck::cast_slice(&ids),
            usage: wgpu::BufferUsages::VERTEX,
        });
        // The sky: one full-screen triangle behind everything, no depth.
        let sky_pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sky"),
            layout: Some(&scene_pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_sky"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_sky"),
                targets: &scene_targets(None, true),
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState { count: SAMPLES, ..Default::default() },
            multiview: None,
            cache: None,
        });

        // ---- post
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
        let post_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post"),
            entries: &[
                uniform_entry(0, wgpu::ShaderStages::FRAGMENT),
                tex_entry(1),
                tex_entry(2),
                tex_entry(3),
                tex_entry(4),
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let post_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("post"),
            bind_group_layouts: &[&post_layout],
            push_constant_ranges: &[],
        });
        let post_pipe = |entry: &str, format: wgpu::TextureFormat, blend: Option<wgpu::BlendState>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(&post_pl),
                vertex: wgpu::VertexState {
                    module: &post_shader,
                    entry_point: Some("vs_post"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &post_shader,
                    entry_point: Some(entry),
                    targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask: wgpu::ColorWrites::ALL })],
                    compilation_options: Default::default(),
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        };
        let add_all = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        };
        let (ssao_pipe, blur_pipe, bloom_pre_pipe, bloom_down_pipe, bloom_up_pipe, rays_pipe, composite_pipe) = std::thread::scope(|s| {
            let a = s.spawn(|| post_pipe("fs_ssao", AO, None));
            let b = s.spawn(|| post_pipe("fs_blur", AO, None));
            let c = s.spawn(|| post_pipe("fs_bloom_pre", HDR, None));
            let d = s.spawn(|| post_pipe("fs_bloom_down", HDR, None));
            let e = s.spawn(|| post_pipe("fs_bloom_up", HDR, Some(add_all)));
            let f = s.spawn(|| post_pipe("fs_rays", HDR, Some(add_all)));
            let g = s.spawn(|| post_pipe("fs_composite", format, None));
            let j = |t: std::thread::ScopedJoinHandle<'_, wgpu::RenderPipeline>| t.join().expect("pipeline thread");
            (j(a), j(b), j(c), j(d), j(e), j(f), j(g))
        });
        let post = PostShared {
            layout: post_layout,
            uniform: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("post"),
                size: std::mem::size_of::<PostUniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("post"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                ..Default::default()
            }),
            blank: texture(&device, "blank", (1, 1), COLOR, 1, true),
        };

        // ---- overlay
        let screen = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screen"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let atlas = device.create_texture_with_data(
            &queue,
            &wgpu::TextureDescriptor {
                label: Some("font"),
                size: wgpu::Extent3d { width: ATLAS_W, height: ATLAS_H, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &build_atlas(),
        );
        let atlas_view = atlas.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("font"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let overlay_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("overlay"),
            entries: &[
                uniform_entry(0, wgpu::ShaderStages::VERTEX),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let overlay_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("overlay"),
            layout: &overlay_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: screen.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&atlas_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });
        let overlay_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("overlay"),
            bind_group_layouts: &[&overlay_layout],
            push_constant_ranges: &[],
        });
        let overlay_pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("overlay"),
            layout: Some(&overlay_pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_overlay"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<OVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_overlay"),
                targets: &[Some(wgpu::ColorTargetState { format, blend: Some(alpha), write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        let meshes = build_meshes(&device);
        let inst = wgpu::BufferUsages::VERTEX;
        let rt = Targets::new(&device, &post, config.width, config.height);
        let timer = GpuTimer::new(&device, &queue);
        let rig_draw_buf = Dyn::new(&device, "rig draws", wgpu::BufferUsages::STORAGE, 1 << 14);
        let rig_part_buf = Dyn::new(&device, "rig parts", wgpu::BufferUsages::STORAGE, 1 << 16);
        Ok(Self {
            shadow_tex,
            shadow_cache,
            shadow_cache_view,
            shadow_cache_key: None,
            static_chunks: Vec::new(),
            face_buf: Dyn::new(&device, "static faces", wgpu::BufferUsages::VERTEX, 1 << 20),
            static_faces: None,
            faces_main: Vec::new(),
            extra_faces: Vec::new(),
            extra_box: (Vec3::INFINITY, Vec3::NEG_INFINITY),
            static_layout,
            static_bind: None,
            quad,
            face_prepass,
            face_opaque,
            face_shadow,
            static_slot: Vec::new(),
            static_gen: 0,
            pending_upload: 0,
            timer,
            counts: FrameCounts::default(),
            fx_off: 0,
            last_submit: None,
            surface,
            static_buf: Dyn::new(&device, "static", inst | wgpu::BufferUsages::STORAGE, 1 << 20),
            dyn_buf: Dyn::new(&device, "dynamic", inst, 1 << 20),
            overlay_buf: Dyn::new(&device, "overlay", inst, 1 << 18),
            device,
            queue,
            config,
            format,
            rt,
            post,
            shadow_layers,
            shadow_pipe,
            shadow_binds,
            cascade_bufs,
            scene_opaque,
            scene_prepass,
            rig_opaque,
            rig_blend,
            rig_prepass,
            rig_shadow,
            rig_layout,
            rig_meshes: std::collections::HashMap::new(),
            rig_draw_buf,
            rig_part_buf,
            rig_ids,
            frame_no: 0,
            sets: std::collections::HashMap::new(),
            scene_blend,
            scene_add,
            sky_pipe,
            ssao_pipe,
            blur_pipe,
            bloom_pre_pipe,
            bloom_down_pipe,
            bloom_up_pipe,
            rays_pipe,
            composite_pipe,
            overlay_pipe,
            globals,
            scene_bind,
            screen,
            overlay_bind,
            meshes,
            static_count: 0,
            backend,
            adapter: adapter_info,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let (w, h) = (width.max(1), height.max(1));
        if w == self.config.width && h == self.config.height {
            return;
        }
        self.config.width = w;
        self.config.height = h;
        if let Some(s) = &self.surface {
            s.configure(&self.device, &self.config);
        }
        self.rt = Targets::new(&self.device, &self.post, w, h);
    }

    pub fn set_vsync(&mut self, on: bool) {
        self.config.present_mode = if on { wgpu::PresentMode::AutoVsync } else { wgpu::PresentMode::AutoNoVsync };
        if let Some(s) = &self.surface {
            s.configure(&self.device, &self.config);
        }
    }

    /// Uploads the level's static geometry when it changed. A new level goes up whole, chunk by
    /// chunk (see `chunk_static`), with the open faces of each chunk (`faces::StaticFaces`). Later
    /// changes go up as they come: rewritten cubes in place, cubes added into the spare slots,
    /// and the faces a hidden cube had covered as a short extra run. Only a cube that moves, or
    /// more added cubes than there is room for, sends the whole level up again.
    pub fn sync_static(&mut self, view: &mut WorldView) {
        if !view.static_dirty && !self.sync_static_changes(view) {
            view.static_dirty = true;
        }
        if !view.static_dirty {
            return;
        }
        let (order, mut chunks) = chunk_static(&view.static_cubes);
        self.static_slot = vec![0; order.len()];
        let mut sorted = Vec::with_capacity(order.len());
        for (slot, &i) in order.iter().enumerate() {
            sorted.push(view.static_cubes[i as usize]);
            self.static_slot[i as usize] = slot as u32;
        }
        let index = StaticFaces::build(&sorted);
        let mut faces: Vec<u32> = Vec::new();
        for ch in chunks.iter_mut() {
            ch.faces = faces.len() as u32;
            for slot in ch.start..ch.start + ch.count {
                push_faces(&mut faces, slot, index.open[slot as usize]);
            }
            ch.face_count = faces.len() as u32 - ch.faces;
        }
        if std::env::var_os("VH_FACE_STATS").is_some() {
            let mut by: std::collections::BTreeMap<i32, (usize, usize)> = Default::default();
            for (c, bits) in sorted.iter().zip(&index.open) {
                let e = by.entry(c.emissive[3] as i32).or_default();
                e.0 += 1;
                e.1 += bits.count_ones() as usize;
            }
            for (shade, (n, f)) in by {
                eprintln!("faces: shade {shade:>2}: {n} cubes, {f} open faces ({:.0}%)", f as f64 * 100.0 / (n * 6) as f64);
            }
        }
        // Room for cubes added later (floor patches, scorch marks) without sending it all again.
        let stride = std::mem::size_of::<Instance>();
        self.static_buf.reserve(&self.device, ((sorted.len() + STATIC_SPARE) * stride) as u64);
        let bytes: &[u8] = bytemuck::cast_slice(&sorted);
        self.static_buf.write(&self.device, &self.queue, bytes);
        self.face_buf.write(&self.device, &self.queue, bytemuck::cast_slice(&faces));
        self.pending_upload += (bytes.len() + faces.len() * 4) as u64;
        self.static_bind = (!sorted.is_empty()).then(|| {
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("static store"),
                layout: &self.static_layout,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: self.static_buf.buf.as_entire_binding() }],
            })
        });
        self.static_count = sorted.len() as u32;
        self.static_chunks = chunks;
        self.static_faces = Some(index);
        self.faces_main = faces;
        self.extra_faces.clear();
        self.extra_box = (Vec3::INFINITY, Vec3::NEG_INFINITY);
        self.static_gen += 1;
        view.static_dirty = false;
        view.static_patches.clear();
    }

    /// The changes since the last upload, sent as they are. False when they need the whole level sent again.
    fn sync_static_changes(&mut self, view: &mut WorldView) -> bool {
        let Some(index) = self.static_faces.as_mut() else { return view.static_cubes.is_empty() };
        let stride = std::mem::size_of::<Instance>() as u64;
        let mut extras_changed = false;
        let grow = |b: &mut (Vec3, Vec3), c: &Instance| {
            let (at, half) = cube_bounds(c);
            // Its neighbours lie within one of its own sizes.
            b.0 = b.0.min(at - half * 3.0);
            b.1 = b.1.max(at + half * 3.0);
        };
        // Cubes added since: into the spare slots, with their open faces in the extra run.
        while self.static_slot.len() < view.static_cubes.len() {
            let slot = self.static_count;
            if (slot as u64 + 1) * stride > self.static_buf.cap {
                return false;
            }
            let c = view.static_cubes[self.static_slot.len()];
            self.queue.write_buffer(&self.static_buf.buf, slot as u64 * stride, bytemuck::bytes_of(&c));
            self.pending_upload += stride;
            self.static_slot.push(slot);
            self.static_count += 1;
            let open = index.add(slot, &c);
            push_faces(&mut self.extra_faces, slot, open);
            grow(&mut self.extra_box, &c);
            extras_changed = true;
        }
        // Rewritten cubes go up run by run, wherever the chunk order put them; a hidden one opens
        // its neighbours' faces.
        for (start, len) in std::mem::take(&mut view.static_patches) {
            let mut i = start;
            while i < start + len {
                let first = self.static_slot[i];
                let mut j = i + 1;
                while j < start + len && self.static_slot[j] == first + (j - i) as u32 {
                    j += 1;
                }
                let bytes: &[u8] = bytemuck::cast_slice(&view.static_cubes[i..j]);
                self.queue.write_buffer(&self.static_buf.buf, first as u64 * stride, bytes);
                self.pending_upload += bytes.len() as u64;
                for k in i..j {
                    let slot = self.static_slot[k];
                    let Some(opened) = index.update(slot, &view.static_cubes[k]) else { return false };
                    if !opened.is_empty() {
                        // The neighbours stand in the hidden cube's chunk, or just over its edge.
                        let at = self.static_chunks.partition_point(|c| c.start <= slot).saturating_sub(1);
                        if let Some(ch) = self.static_chunks.get(at) {
                            self.extra_box.0 = self.extra_box.0.min(ch.min - Vec3::ONE);
                            self.extra_box.1 = self.extra_box.1.max(ch.max + Vec3::ONE);
                        }
                        self.extra_faces.extend(opened);
                        extras_changed = true;
                    }
                }
                i = j;
            }
        }
        if extras_changed {
            // Shadows change too: the cached cascade is drawn again.
            self.static_gen += 1;
            let main = self.faces_main.len();
            let needed = ((main + self.extra_faces.len()) * 4) as u64;
            if needed > self.face_buf.cap {
                let all: Vec<u32> = self.faces_main.iter().chain(&self.extra_faces).copied().collect();
                self.face_buf.write(&self.device, &self.queue, bytemuck::cast_slice(&all));
            } else {
                self.queue.write_buffer(&self.face_buf.buf, main as u64 * 4, bytemuck::cast_slice(&self.extra_faces));
            }
            self.pending_upload += self.extra_faces.len() as u64 * 4;
        }
        true
    }

    /// Draws one frame into `target`: shadows, the scene, post-processing, then the overlay on top.
    pub fn draw(&mut self, target: &wgpu::TextureView, view: &WorldView, frame: &Frame, overlay: &[OVertex], logical: (f32, f32)) {
        let mut counts = FrameCounts { upload: std::mem::take(&mut self.pending_upload), ..Default::default() };
        if let Some(t) = self.timer.as_mut() {
            t.collect(&self.device);
            t.begin();
        }
        // ---- uniforms
        let mut lights = [GpuLight::zeroed(); MAX_LIGHTS];
        let mut chosen: Vec<&Light> = frame.lights.iter().collect();
        counts.lights = chosen.len() as u32;
        if chosen.len() > MAX_LIGHTS {
            let hero = frame.lights[0].pos;
            chosen.sort_by(|a, b| a.pos.distance_squared(hero).partial_cmp(&b.pos.distance_squared(hero)).unwrap());
            chosen.truncate(MAX_LIGHTS);
        }
        counts.lights_dropped = counts.lights - chosen.len() as u32;
        for (i, l) in chosen.iter().enumerate() {
            lights[i] = GpuLight { pos: [l.pos.x, l.pos.y, l.pos.z, l.distance], color: [l.color.x, l.color.y, l.color.z, 0.0] };
        }
        let atm = frame.atmosphere;
        let fog = super::overlay::rgba(atm.background, atm.fog_near);
        let v4 = |v: Vec3, w: f32| [v.x, v.y, v.z, w];
        // The fog is light: as much as tone maps to the fog colour.
        let fog_light = untonemap(super::voxels::lin(atm.background));
        let (cam, fwd, cam_r, cam_u, ortho) = view.camera_basis();
        let (sky_fwd, sky_right, sky_up) = view.sky_basis();
        // Two shadow cascades look down the light: a fine one over the ground ahead of
        // Ragnar, stepped a whole texel at a time so its edges never crawl, and a coarse
        // one over the whole level (or the diorama on show).
        let (center, radius) = view.shadow_box();
        let l = atm.sun_dir.normalize_or(Vec3::Y);
        let up = if l.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
        let light_view = Mat4::look_at_rh(center + l * 90.0, center, up);
        let light_vp = Mat4::orthographic_rh(-radius, radius, -radius, radius, 1.0, 200.0) * light_view;
        let near_r = NEAR_CASCADE;
        let light_vp_near = match view.shadow_focus() {
            Some(focus) => {
                let f = light_view.transform_point3(focus);
                let texel = 2.0 * near_r / SHADOW_SIZE as f32;
                let (cx, cy) = ((f.x / texel).round() * texel, (f.y / texel).round() * texel);
                Some(Mat4::orthographic_rh(cx - near_r, cx + near_r, cy - near_r, cy + near_r, 1.0, 200.0) * light_view)
            }
            None => None,
        };
        let view_proj = view.view_proj();
        let g = Globals {
            view_proj: view_proj.to_cols_array(),
            light_vp: light_vp.to_cols_array(),
            light_vp_near: light_vp_near.unwrap_or(light_vp).to_cols_array(),
            cam_pos: v4(view.camera_position(), if ortho { 0.0 } else { 1.0 }),
            cam_fwd: v4(fwd, 1.0 / SHADOW_SIZE as f32),
            fog,
            params: [atm.fog_far.min(FAR), atm.exposure, frame.coin_pulse, chosen.len() as f32],
            hemi_sky: v4(atm.hemi_sky, 0.0),
            hemi_ground: v4(atm.hemi_ground, fog_light.x),
            ambient: v4(atm.ambient, fog_light.y),
            moon_dir: v4(atm.sun_dir, fog_light.z),
            moon_color: v4(atm.sun, 0.0),
            sky_fwd: v4(sky_fwd, atm.night_sky),
            sky_right: v4(sky_right, frame.time),
            sky_up: v4(sky_up, 0.0),
            sky_moon: v4(atm.moon_view, 0.0),
            bounds: frame.bounds.to_array(),
            shadow: [near_r, radius, if light_vp_near.is_some() { 1.0 } else { 0.0 }, (self.fx_off & (fx::SCENE | fx::BACKFACE)) as f32],
            lights,
        };
        self.queue.write_buffer(&self.cascade_bufs[0], 0, bytemuck::cast_slice(&light_vp_near.unwrap_or(light_vp).to_cols_array()));
        self.queue.write_buffer(&self.cascade_bufs[1], 0, bytemuck::cast_slice(&light_vp.to_cols_array()));
        self.queue.write_buffer(&self.globals, 0, bytemuck::bytes_of(&g));
        let focus = view.focus_depth();
        let post = PostUniform {
            view_proj: view_proj.to_cols_array(),
            cam_pos: v4(cam, if ortho { 1.0 } else { 0.0 }),
            cam_fwd: v4(fwd, focus.unwrap_or(0.0)),
            cam_r: v4(cam_r, 5.0),
            cam_u: v4(cam_u, 20.0),
            params: [
                0.32,
                if self.fx_off & fx::AO != 0 { 0.0 } else { 1.4 },
                if self.fx_off & fx::BLOOM != 0 { 0.0 } else { 0.45 * atm.bloom },
                if focus.is_some() && self.fx_off & fx::DOF == 0 { 3.5 } else { 0.0 },
            ],
            params2: [0.6, 0.28, frame.time, if self.fx_off & fx::GRADE != 0 { 0.0 } else { 1.0 }],
            moon: moon_on_screen(sky_fwd, sky_right, sky_up, atm.moon_view, atm.night_sky),
        };
        self.queue.write_buffer(&self.post.uniform, 0, bytemuck::bytes_of(&post));
        self.queue.write_buffer(&self.screen, 0, bytemuck::cast_slice(&[logical.0, logical.1, 0.0, 0.0]));
        counts.upload += (std::mem::size_of::<Globals>() + std::mem::size_of::<PostUniform>() + 128 + 16) as u64;

        // ---- instances: opaque by mesh, the early see-through ones, see-through by mesh, additive
        let order_t =
            [MeshKind::Disc, MeshKind::Annulus, MeshKind::Ring, MeshKind::Cube, MeshKind::Sphere, MeshKind::Tube, MeshKind::Cylinder];
        let total = frame.opaque.iter().chain(frame.transparent.iter()).map(|l| l.len()).sum::<usize>()
            + frame.transparent_early.len()
            + frame.additive.len()
            + frame.glow.len();
        let mut all: Vec<Instance> = Vec::with_capacity(total);
        // (stage, mesh, start, count): 0 opaque, 1 the early see-through, 3 see-through, 2 additive.
        let mut ranges: Vec<(u8, usize, u32, u32)> = Vec::new();
        for k in 0..MESH_KINDS {
            let list = &frame.opaque[k];
            if !list.is_empty() {
                ranges.push((0, k, all.len() as u32, list.len() as u32));
                all.extend_from_slice(list);
            }
        }
        if !frame.transparent_early.is_empty() {
            ranges.push((1, MeshKind::Cube as usize, all.len() as u32, frame.transparent_early.len() as u32));
            all.extend_from_slice(&frame.transparent_early);
        }
        for k in order_t {
            let list = &frame.transparent[k as usize];
            if !list.is_empty() {
                ranges.push((3, k as usize, all.len() as u32, list.len() as u32));
                all.extend_from_slice(list);
            }
        }
        if !frame.additive.is_empty() {
            ranges.push((2, MeshKind::Cube as usize, all.len() as u32, frame.additive.len() as u32));
            all.extend_from_slice(&frame.additive);
        }
        if !frame.glow.is_empty() {
            ranges.push((2, MeshKind::Disc as usize, all.len() as u32, frame.glow.len() as u32));
            all.extend_from_slice(&frame.glow);
        }
        // ---- rigs: the solid ones, then the see-through, as one list of records and matrices
        self.frame_no += 1;
        let rig_bind = self.upload_rigs(frame, &mut counts);
        self.upload_sets(frame, &mut counts);
        let rig_split = frame.rigs.len() as u32;
        let rig_total = rig_split + frame.rigs_blend.len() as u32;
        self.dyn_buf.write(&self.device, &self.queue, bytemuck::cast_slice(&all));
        self.overlay_buf.write(&self.device, &self.queue, bytemuck::cast_slice(overlay));
        counts.upload += (std::mem::size_of_val(all.as_slice()) + std::mem::size_of_val(overlay)) as u64;

        // ---- the static chunks each view sees: the camera's front to back, then each cascade's casters
        let use_static = self.static_count > 0 && !frame.skip_static;
        let visible = |m: Mat4| -> Vec<(u32, u32)> {
            if !use_static {
                return Vec::new();
            }
            let fr = Frustum::new(m);
            let mut runs: Vec<(u32, u32)> =
                self.static_chunks.iter().filter(|c| fr.sees(c.min, c.max)).map(|c| (c.faces, c.face_count)).collect();
            if !self.extra_faces.is_empty() && fr.sees(self.extra_box.0, self.extra_box.1) {
                runs.push((self.faces_main.len() as u32, self.extra_faces.len() as u32));
            }
            runs
        };
        let mut main_chunks: Vec<(f32, u32, u32)> = Vec::new();
        if use_static {
            let fr = Frustum::new(view_proj);
            let eye = view.camera_position();
            for c in self.static_chunks.iter().filter(|c| fr.sees(c.min, c.max)) {
                let near = eye.clamp(c.min, c.max).distance_squared(eye);
                main_chunks.push((near, c.faces, c.face_count));
                counts.static_drawn += c.count as u64;
            }
            main_chunks.sort_by(|a, b| a.0.total_cmp(&b.0));
        }
        let mut main_faces: Vec<(u32, u32)> = main_chunks.iter().map(|c| (c.1, c.2)).collect();
        if use_static && !self.extra_faces.is_empty() && Frustum::new(view_proj).sees(self.extra_box.0, self.extra_box.1) {
            main_faces.push((self.faces_main.len() as u32, self.extra_faces.len() as u32));
        }
        let near_chunks = light_vp_near.map(&visible).unwrap_or_default();
        // The coarse cascade's static part is drawn once and copied in, until the light, the
        // level or the box changes.
        let cache_key = (self.static_gen, light_vp.to_cols_array());
        let cache_fresh = use_static && self.shadow_cache_key == Some(cache_key);
        counts.chunks = self.static_chunks.len() as u32;
        counts.static_total = self.static_count as u64;
        counts.chunks_drawn = main_chunks.len() as u32;

        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        if use_static && !cache_fresh {
            let far = visible(light_vp);
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow cache"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_cache_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: self.timer.as_mut().and_then(|t| t.pass("shadow cache")),
                occlusion_query_set: None,
            });
            pass.set_bind_group(0, &self.shadow_binds[1], &[]);
            self.draw_faces(&mut pass, &self.face_shadow, &far, &mut counts);
            counts.shadow_static += far.iter().map(|c| c.1 as u64).sum::<u64>();
            drop(pass);
            self.shadow_cache_key = Some(cache_key);
        }
        if use_static {
            enc.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.shadow_cache,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &self.shadow_tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x: 0, y: 0, z: 1 },
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d { width: SHADOW_SIZE, height: SHADOW_SIZE, depth_or_array_layers: 1 },
            );
        }

        // ---- shadow maps, fine cascade then coarse (the fine one is skipped when unused)
        for c in 0..2 {
            if c == 0 && light_vp_near.is_none() {
                continue;
            }
            let label = if c == 0 { "shadow near" } else { "shadow far" };
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_layers[c],
                    depth_ops: Some(wgpu::Operations {
                        load: if c == 1 && use_static { wgpu::LoadOp::Load } else { wgpu::LoadOp::Clear(1.0) },
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: self.timer.as_mut().and_then(|t| t.pass(label)),
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.shadow_pipe);
            pass.set_bind_group(0, &self.shadow_binds[c], &[]);
            if c == 0 && !near_chunks.is_empty() {
                self.draw_faces(&mut pass, &self.face_shadow, &near_chunks, &mut counts);
                counts.shadow_static += near_chunks.iter().map(|c| c.1 as u64).sum::<u64>();
            }
            if !all.is_empty() {
                pass.set_pipeline(&self.shadow_pipe);
                pass.set_vertex_buffer(1, self.dyn_buf.buf.slice(..));
                for &(_, mesh, start, count) in ranges.iter().filter(|r| r.0 == 0) {
                    let m = &self.meshes[mesh];
                    pass.set_vertex_buffer(0, m.vertices.slice(..));
                    pass.set_index_buffer(m.indices.slice(..), wgpu::IndexFormat::Uint16);
                    pass.draw_indexed(0..m.count, 0, start..start + count);
                    counts.draw(count as u64, m.count);
                }
            }
            self.draw_sets(&mut pass, &frame.sets, &mut counts);
            if let Some(bind) = rig_bind.as_ref().filter(|_| rig_split > 0) {
                pass.set_pipeline(&self.rig_shadow);
                pass.set_bind_group(1, bind, &[]);
                // The near cascade shades the rigs on screen, so it keeps their very level (self-shadow
                // stays aligned); the coarse far cascade takes each a level coarser.
                if c == 0 {
                    self.draw_rigs(&mut pass, &frame.rigs.meshes, 0, &mut counts);
                } else {
                    self.draw_rigs(&mut pass, &frame.rigs.far_meshes, rig_total, &mut counts);
                }
            }
        }

        // ---- scene
        {
            // The clear shows where nothing is drawn: the light that tone maps to the background colour.
            let bg = [fog_light.x, fog_light.y, fog_light.z, 1.0];
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.rt.msaa_color,
                        resolve_target: Some(&self.rt.color),
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color { r: bg[0] as f64, g: bg[1] as f64, b: bg[2] as f64, a: 1.0 }),
                            store: wgpu::StoreOp::Discard,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.rt.msaa_gbuf,
                        resolve_target: Some(&self.rt.gbuf),
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 1.0, b: 0.0, a: 10000.0 }),
                            store: wgpu::StoreOp::Discard,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.rt.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }),
                    stencil_ops: None,
                }),
                timestamp_writes: self.timer.as_mut().and_then(|t| t.pass("main")),
                occlusion_query_set: None,
            });
            pass.set_bind_group(0, &self.scene_bind, &[]);
            if atm.night_sky > 0.0 {
                pass.set_pipeline(&self.sky_pipe);
                pass.draw(0..3, 0..1);
                counts.draw(1, 3);
            }

            // The depth of every opaque thing first (nearest static chunks first), then the shading.
            let prepass = self.fx_off & fx::PREPASS == 0;
            if prepass && !main_faces.is_empty() {
                self.draw_faces(&mut pass, &self.face_prepass, &main_faces, &mut counts);
            }
            if prepass && !all.is_empty() {
                pass.set_pipeline(&self.scene_prepass);
                pass.set_vertex_buffer(1, self.dyn_buf.buf.slice(..));
                for &(_, mesh, start, count) in ranges.iter().filter(|r| r.0 == 0) {
                    let m = &self.meshes[mesh];
                    pass.set_vertex_buffer(0, m.vertices.slice(..));
                    pass.set_index_buffer(m.indices.slice(..), wgpu::IndexFormat::Uint16);
                    pass.draw_indexed(0..m.count, 0, start..start + count);
                    counts.draw(count as u64, m.count);
                }
            }
            if prepass && !frame.sets.is_empty() {
                pass.set_pipeline(&self.scene_prepass);
                self.draw_sets(&mut pass, &frame.sets, &mut counts);
            }
            if let Some(bind) = rig_bind.as_ref() {
                pass.set_bind_group(1, bind, &[]);
            }
            if prepass && rig_split > 0 {
                pass.set_pipeline(&self.rig_prepass);
                self.draw_rigs(&mut pass, &frame.rigs.meshes, 0, &mut counts);
            }
            if !main_faces.is_empty() {
                self.draw_faces(&mut pass, &self.face_opaque, &main_faces, &mut counts);
            }
            if let Some(bind) = rig_bind.as_ref() {
                pass.set_bind_group(1, bind, &[]);
            }
            if !frame.sets.is_empty() {
                pass.set_pipeline(&self.scene_opaque);
                self.draw_sets(&mut pass, &frame.sets, &mut counts);
            }
            // The solid rigs with the solid things; the see-through sets and rigs where see-through cubes go.
            let mut rigs_done = rig_split == 0;
            let mut blend_done = rig_total == rig_split && frame.sets_blend.is_empty();
            for &(pipe, mesh, start, count) in &ranges {
                if pipe > 0 && !rigs_done {
                    pass.set_pipeline(&self.rig_opaque);
                    self.draw_rigs(&mut pass, &frame.rigs.meshes, 0, &mut counts);
                    rigs_done = true;
                }
                // See-through sets and rigs come at the end of the see-through cubes, where they were listed
                // (over the water, under the spheres and veils of falling water).
                let early = [MeshKind::Disc, MeshKind::Annulus, MeshKind::Ring, MeshKind::Cube].iter().any(|k| *k as usize == mesh);
                if (pipe == 2 || (pipe == 3 && !early)) && !blend_done {
                    self.draw_blend_sets_and_rigs(&mut pass, frame, rig_split, &mut counts);
                    blend_done = true;
                }
                pass.set_pipeline(match pipe {
                    0 => &self.scene_opaque,
                    1 | 3 => &self.scene_blend,
                    _ => &self.scene_add,
                });
                let m = &self.meshes[mesh];
                pass.set_vertex_buffer(0, m.vertices.slice(..));
                pass.set_index_buffer(m.indices.slice(..), wgpu::IndexFormat::Uint16);
                pass.set_vertex_buffer(1, self.dyn_buf.buf.slice(..));
                pass.draw_indexed(0..m.count, 0, start..start + count);
                counts.draw(count as u64, m.count);
            }
            if !rigs_done {
                pass.set_pipeline(&self.rig_opaque);
                self.draw_rigs(&mut pass, &frame.rigs.meshes, 0, &mut counts);
            }
            if !blend_done {
                self.draw_blend_sets_and_rigs(&mut pass, frame, rig_split, &mut counts);
            }
        }

        // ---- post
        #[allow(clippy::too_many_arguments)]
        fn fullscreen(
            enc: &mut wgpu::CommandEncoder,
            label: &'static str,
            target: &wgpu::TextureView,
            pipe: &wgpu::RenderPipeline,
            bind: &wgpu::BindGroup,
            clear: bool,
            timer: &mut Option<GpuTimer>,
            counts: &mut FrameCounts,
        ) {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: if clear { wgpu::LoadOp::Clear(wgpu::Color::BLACK) } else { wgpu::LoadOp::Load },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: timer.as_mut().and_then(|t| t.pass(label)),
                occlusion_query_set: None,
            });
            pass.set_pipeline(pipe);
            pass.set_bind_group(0, bind, &[]);
            pass.draw(0..3, 0..1);
            counts.draw(1, 3);
        }
        let rt = &self.rt;
        let (timer, c) = (&mut self.timer, &mut counts);
        fullscreen(&mut enc, "ssao", &rt.ao, &self.ssao_pipe, &rt.ssao_bind, true, timer, c);
        fullscreen(&mut enc, "ssao", &rt.ao_blur, &self.blur_pipe, &rt.blur_bind, true, timer, c);
        fullscreen(&mut enc, "bloom", &rt.bloom[0], &self.bloom_pre_pipe, &rt.down_binds[0], true, timer, c);
        for i in 1..BLOOM_LEVELS {
            fullscreen(&mut enc, "bloom", &rt.bloom[i], &self.bloom_down_pipe, &rt.down_binds[i], true, timer, c);
        }
        for i in (0..BLOOM_LEVELS - 1).rev() {
            fullscreen(&mut enc, "bloom", &rt.bloom[i], &self.bloom_up_pipe, &rt.up_binds[i], false, timer, c);
        }
        if post.moon[2] > 0.0 {
            fullscreen(&mut enc, "shafts", &rt.bloom[0], &self.rays_pipe, &rt.rays_bind, false, timer, c);
        }
        fullscreen(&mut enc, "composite", target, &self.composite_pipe, &rt.composite_bind, true, timer, c);

        // ---- HUD
        if !overlay.is_empty() {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("overlay"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: self.timer.as_mut().and_then(|t| t.pass("hud")),
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.overlay_pipe);
            pass.set_bind_group(0, &self.overlay_bind, &[]);
            pass.set_vertex_buffer(0, self.overlay_buf.buf.slice(..));
            pass.draw(0..overlay.len() as u32, 0..1);
            counts.draw(1, overlay.len() as u32);
        }
        if let Some(t) = self.timer.as_mut() {
            t.resolve(&mut enc);
        }
        self.last_submit = Some(self.queue.submit([enc.finish()]));
        if let Some(t) = self.timer.as_mut() {
            t.submitted();
        }
        self.counts = counts;
    }

    /// Makes sure every rig this frame draws has its mesh on the GPU, uploads the frame's
    /// draw records and part matrices, and lets go of meshes long unused. The bind group
    /// for the records, if any rig is drawn.
    fn upload_rigs(&mut self, frame: &Frame, counts: &mut FrameCounts) -> Option<wgpu::BindGroup> {
        let lists = [&frame.rigs, &frame.rigs_blend];
        let total: usize = lists.iter().map(|l| l.len()).sum();
        let now = self.frame_no;
        if self.frame_no.is_multiple_of(60) {
            self.rig_meshes.retain(|_, b| now - b.used < RIG_IDLE_FRAMES);
        }
        if total == 0 {
            return None;
        }
        let mut draws: Vec<RigDraw> = Vec::with_capacity(total);
        let mut parts: Vec<[f32; 16]> = Vec::new();
        // Records: the solid rigs, the see-through ones, then the solid ones again a level coarser
        // for the far shadow cascade (sharing the solid rigs' matrices).
        for list in lists {
            let base = parts.len() as u32;
            draws.extend(list.draws.iter().map(|d| RigDraw { part_base: d.part_base + base, ..*d }));
            parts.extend_from_slice(&list.parts);
            for (k, n) in list.levels.iter().enumerate() {
                counts.rig_levels[k] += n;
            }
        }
        draws.extend_from_slice(&frame.rigs.far_draws);
        for mesh in frame.rigs.meshes.iter().chain(&frame.rigs_blend.meshes).chain(&frame.rigs.far_meshes) {
            {
                let device = &self.device;
                let buf = self.rig_meshes.entry(mesh.id).or_insert_with(|| {
                    counts.upload +=
                        (std::mem::size_of_val(mesh.vertices.as_slice()) + std::mem::size_of_val(mesh.indices.as_slice())) as u64;
                    rig_buf(device, mesh)
                });
                buf.used = now;
            }
        }
        for mesh in frame.rigs.meshes.iter().chain(&frame.rigs_blend.meshes) {
            counts.rigs += 1;
            counts.rig_voxels += mesh.voxels as u64;
        }
        draws.truncate(MAX_RIG_DRAWS as usize);
        let (d, p): (&[u8], &[u8]) = (bytemuck::cast_slice(&draws), bytemuck::cast_slice(&parts));
        self.rig_draw_buf.write(&self.device, &self.queue, d);
        self.rig_part_buf.write(&self.device, &self.queue, p);
        counts.upload += (d.len() + p.len()) as u64;
        Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rigs"),
            layout: &self.rig_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.rig_draw_buf.buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: self.rig_part_buf.buf.as_entire_binding() },
            ],
        }))
    }

    /// Draws runs of static faces (start, count) with `pipe`, which reads the static store.
    fn draw_faces(&self, pass: &mut wgpu::RenderPass<'_>, pipe: &wgpu::RenderPipeline, runs: &[(u32, u32)], counts: &mut FrameCounts) {
        let Some(bind) = &self.static_bind else { return };
        pass.set_pipeline(pipe);
        pass.set_bind_group(1, bind, &[]);
        pass.set_vertex_buffer(0, self.face_buf.buf.slice(..));
        pass.set_index_buffer(self.quad.slice(..), wgpu::IndexFormat::Uint16);
        for &(start, count) in runs {
            if count > 0 {
                pass.draw_indexed(0..6, 0, start..start + count);
                counts.draw(count as u64, 6);
            }
        }
    }

    /// Puts every instance set this frame draws on the GPU (once), and lets go of those long unused.
    fn upload_sets(&mut self, frame: &Frame, counts: &mut FrameCounts) {
        let now = self.frame_no;
        if now % 60 == 30 {
            self.sets.retain(|_, b| now - b.used < RIG_IDLE_FRAMES);
        }
        for set in frame.sets.iter().chain(&frame.sets_blend) {
            if set.is_empty() {
                continue;
            }
            let device = &self.device;
            let buf = self.sets.entry(set.id).or_insert_with(|| {
                counts.upload += std::mem::size_of_val(set.instances.as_slice()) as u64;
                SetBuf {
                    buf: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("instance set"),
                        contents: bytemuck::cast_slice(set.instances.as_slice()),
                        usage: wgpu::BufferUsages::VERTEX,
                    }),
                    count: set.len() as u32,
                    used: 0,
                }
            });
            buf.used = now;
        }
    }

    /// Draws instance sets as cubes, with the pipeline already set.
    fn draw_sets(&self, pass: &mut wgpu::RenderPass<'_>, sets: &[InstanceSet], counts: &mut FrameCounts) {
        let m = &self.meshes[MeshKind::Cube as usize];
        pass.set_vertex_buffer(0, m.vertices.slice(..));
        pass.set_index_buffer(m.indices.slice(..), wgpu::IndexFormat::Uint16);
        for set in sets {
            let Some(buf) = self.sets.get(&set.id) else { continue };
            pass.set_vertex_buffer(1, buf.buf.slice(..));
            pass.draw_indexed(0..m.count, 0, 0..buf.count);
            counts.draw(buf.count as u64, m.count);
        }
    }

    /// The see-through sets (mist), then the see-through rigs.
    fn draw_blend_sets_and_rigs(&self, pass: &mut wgpu::RenderPass<'_>, frame: &Frame, rig_split: u32, counts: &mut FrameCounts) {
        if !frame.sets_blend.is_empty() && self.fx_off & fx::MIST == 0 {
            pass.set_pipeline(&self.scene_blend);
            self.draw_sets(pass, &frame.sets_blend, counts);
        }
        if !frame.rigs_blend.is_empty() {
            pass.set_pipeline(&self.rig_blend);
            self.draw_rigs(pass, &frame.rigs_blend.meshes, rig_split, counts);
        }
    }

    /// Draws a list of rigs with the pipeline already set; `first` is the list's first record.
    fn draw_rigs(&self, pass: &mut wgpu::RenderPass<'_>, meshes: &[std::sync::Arc<RigMesh>], first: u32, counts: &mut FrameCounts) {
        pass.set_vertex_buffer(1, self.rig_ids.slice(..));
        for (k, mesh) in meshes.iter().enumerate() {
            let id = first + k as u32;
            let Some(buf) = self.rig_meshes.get(&mesh.id).filter(|_| id < MAX_RIG_DRAWS) else { continue };
            pass.set_vertex_buffer(0, buf.vertices.slice(..));
            pass.set_index_buffer(buf.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..buf.count, 0, id..id + 1);
            counts.draw(1, buf.count);
        }
    }

    /// A texture the size of the screen to draw into without a window.
    pub fn offscreen(&self) -> wgpu::TextureView {
        self.device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("offscreen"),
                size: wgpu::Extent3d { width: self.config.width, height: self.config.height, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&Default::default())
    }

    /// Draws the same frame `n` times offscreen, waiting for each; the mean milliseconds per frame.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn bench(&mut self, view: &WorldView, frame: &Frame, overlay: &[OVertex], logical: (f32, f32), n: u32) -> f64 {
        let (w, h) = (self.config.width, self.config.height);
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bench"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let tv = tex.create_view(&Default::default());
        // A second of drawing first, so a laptop GPU has left its idle clocks.
        let warm = std::time::Instant::now();
        while warm.elapsed().as_secs_f64() < 1.0 {
            self.draw(&tv, view, frame, overlay, logical);
            let _ = self.device.poll(wgpu::PollType::Wait);
        }
        let start = std::time::Instant::now();
        let mut cpu = 0.0;
        for _ in 0..n {
            let t = std::time::Instant::now();
            self.draw(&tv, view, frame, overlay, logical);
            cpu += t.elapsed().as_secs_f64();
            let _ = self.device.poll(wgpu::PollType::Wait);
        }
        eprintln!("bench: {:.2} ms of it recording and uploading on the CPU", cpu * 1000.0 / n.max(1) as f64);
        start.elapsed().as_secs_f64() * 1000.0 / n.max(1) as f64
    }

    /// Renders into an offscreen texture and reads it back as tightly packed RGBA8.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn capture(&mut self, view: &WorldView, frame: &Frame, overlay: &[OVertex], logical: (f32, f32)) -> Vec<u8> {
        let (w, h) = (self.config.width, self.config.height);
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("capture"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let tv = tex.create_view(&Default::default());
        self.draw(&tv, view, frame, overlay, logical);
        let row = (w * 4).div_ceil(256) * 256;
        let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (row * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = self.device.create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(h) },
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        self.queue.submit([enc.finish()]);
        let slice = buf.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.device.poll(wgpu::PollType::Wait);
        let data = slice.get_mapped_range();
        let bgra = matches!(self.format, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb);
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            let line = &data[(y * row) as usize..(y * row + w * 4) as usize];
            for px in line.chunks(4) {
                if bgra {
                    out.extend_from_slice(&[px[2], px[1], px[0], 255]);
                } else {
                    out.extend_from_slice(&[px[0], px[1], px[2], 255]);
                }
            }
        }
        out
    }
}

/**
 * Where the moon (or at dusk the sun) is drawn on screen, in uv, and how
 * strongly it should shine past the things in front of it: nothing when it
 * is behind the camera or there is no sky, fading as it leaves the screen.
 * The sky is drawn from its own view rays (`WorldView::sky_basis`), so the
 * moon's place is found the same way.
 */
fn moon_on_screen(fwd: Vec3, right: Vec3, up: Vec3, moon: Vec3, night_sky: f32) -> [f32; 4] {
    let m = moon.normalize_or(Vec3::Y);
    let k = m.dot(fwd) / fwd.length_squared();
    if night_sky <= 0.0 || k <= 0.05 {
        return [0.0; 4];
    }
    let x = m.dot(right) / (k * right.length_squared());
    let y = m.dot(up) / (k * up.length_squared());
    let off = (x.abs().max(y.abs()) - 1.0).max(0.0);
    let strength = (1.0 - off / 0.6).clamp(0.0, 1.0) * if night_sky > 1.5 { 0.9 } else { 0.55 };
    [x * 0.5 + 0.5, 0.5 - y * 0.5, strength, 0.0]
}

// ------------------------------------------------------------------ meshes

fn build_meshes(device: &wgpu::Device) -> Vec<MeshBuf> {
    let mut out = Vec::new();
    for kind in 0..MESH_KINDS {
        let (v, i) = match kind {
            0 => cube(),
            1 => sphere(16, 12),
            2 => disc(12),
            3 => ring(0.45, 0.85, 16, 0.2 * std::f32::consts::PI, 0.6 * std::f32::consts::PI),
            4 => cylinder(0.34, 0.4, 0.12, 8),
            5 => tube(1.0, 1.12, 1.0, 24),
            _ => ring(0.82, 1.0, 32, 0.0, std::f32::consts::TAU),
        };
        out.push(MeshBuf {
            vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("mesh vertices"),
                contents: bytemuck::cast_slice(&v),
                usage: wgpu::BufferUsages::VERTEX,
            }),
            indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("mesh indices"),
                contents: bytemuck::cast_slice(&pad_indices(i.clone())),
                usage: wgpu::BufferUsages::INDEX,
            }),
            count: i.len() as u32,
        });
    }
    out
}

fn rig_buf(device: &wgpu::Device, mesh: &RigMesh) -> RigBuf {
    let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("rig vertices"),
        contents: bytemuck::cast_slice(&mesh.vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("rig indices"),
        contents: bytemuck::cast_slice(&mesh.indices),
        usage: wgpu::BufferUsages::INDEX,
    });
    RigBuf { vertices, indices, count: mesh.indices.len() as u32, used: 0 }
}

/// Index buffers must be a multiple of four bytes.
fn pad_indices(mut i: Vec<u16>) -> Vec<u16> {
    if i.len() % 2 == 1 {
        i.push(0);
    }
    i
}

type MeshData = (Vec<[f32; 3]>, Vec<u16>);

/// Unit cube centered on the origin.
fn cube() -> MeshData {
    let mut v = Vec::new();
    for i in 0..8 {
        v.push([if i & 1 == 0 { -0.5 } else { 0.5 }, if i & 2 == 0 { -0.5 } else { 0.5 }, if i & 4 == 0 { -0.5 } else { 0.5 }]);
    }
    let faces: [[u16; 4]; 6] = [[0, 2, 3, 1], [4, 5, 7, 6], [0, 1, 5, 4], [2, 6, 7, 3], [0, 4, 6, 2], [1, 3, 7, 5]];
    let mut i = Vec::new();
    for f in faces {
        i.extend_from_slice(&[f[0], f[1], f[2], f[0], f[2], f[3]]);
    }
    (v, i)
}

/// Unit sphere.
fn sphere(ws: u32, hs: u32) -> MeshData {
    let mut v = Vec::new();
    for y in 0..=hs {
        let phi = y as f32 / hs as f32 * std::f32::consts::PI;
        for x in 0..=ws {
            let theta = x as f32 / ws as f32 * std::f32::consts::TAU;
            v.push([-theta.cos() * phi.sin(), phi.cos(), theta.sin() * phi.sin()]);
        }
    }
    let mut i = Vec::new();
    let row = ws + 1;
    for y in 0..hs {
        for x in 0..ws {
            let a = (y * row + x) as u16;
            let b = a + row as u16;
            i.extend_from_slice(&[a, b, a + 1, b, b + 1, a + 1]);
        }
    }
    (v, i)
}

/// Unit disc in the XY plane.
fn disc(segments: u32) -> MeshData {
    let mut v = vec![[0.0, 0.0, 0.0]];
    for s in 0..=segments {
        let a = s as f32 / segments as f32 * std::f32::consts::TAU;
        v.push([a.cos(), a.sin(), 0.0]);
    }
    let mut i = Vec::new();
    for s in 1..=segments as u16 {
        i.extend_from_slice(&[0, s, s + 1]);
    }
    (v, i)
}

/// A flat ring sector in the XY plane.
fn ring(inner: f32, outer: f32, segments: u32, start: f32, length: f32) -> MeshData {
    let mut v = Vec::new();
    for s in 0..=segments {
        let a = start + s as f32 / segments as f32 * length;
        v.push([inner * a.cos(), inner * a.sin(), 0.0]);
        v.push([outer * a.cos(), outer * a.sin(), 0.0]);
    }
    let mut i = Vec::new();
    for s in 0..segments as u16 {
        let a = s * 2;
        i.extend_from_slice(&[a, a + 1, a + 3, a, a + 3, a + 2]);
    }
    (v, i)
}

/// Open tube along y, centered on the origin: no caps.
fn tube(r_top: f32, r_bottom: f32, h: f32, segments: u32) -> MeshData {
    let mut v = Vec::new();
    for s in 0..=segments {
        let a = s as f32 / segments as f32 * std::f32::consts::TAU;
        v.push([r_top * a.sin(), h / 2.0, r_top * a.cos()]);
        v.push([r_bottom * a.sin(), -h / 2.0, r_bottom * a.cos()]);
    }
    let mut i = Vec::new();
    for s in 0..segments as u16 {
        let (t0, b0, t1, b1) = (s * 2, s * 2 + 1, s * 2 + 2, s * 2 + 3);
        i.extend_from_slice(&[t0, b0, b1, t0, b1, t1]);
    }
    (v, i)
}

/// Capped cylinder along y, centered on the origin.
fn cylinder(r_top: f32, r_bottom: f32, h: f32, segments: u32) -> MeshData {
    let mut v = vec![[0.0, h / 2.0, 0.0], [0.0, -h / 2.0, 0.0]];
    for s in 0..segments {
        let a = s as f32 / segments as f32 * std::f32::consts::TAU;
        v.push([r_top * a.sin(), h / 2.0, r_top * a.cos()]);
        v.push([r_bottom * a.sin(), -h / 2.0, r_bottom * a.cos()]);
    }
    let mut i = Vec::new();
    let n = segments as u16;
    for s in 0..n {
        let t0 = 2 + s * 2;
        let b0 = t0 + 1;
        let t1 = 2 + ((s + 1) % n) * 2;
        let b1 = t1 + 1;
        i.extend_from_slice(&[t0, b0, b1, t0, b1, t1, 0, t0, t1, 1, b1, b0]);
    }
    (v, i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn untonemap_finds_the_light_behind_a_display_colour() {
        for hex in [0x06100b, 0x2a0a0e, 0x767676, 0xc49358, 0x567770, 0xb6b39a, 0x202040] {
            let t = super::super::voxels::lin(hex);
            let back = aces_filmic(untonemap(t));
            assert!((back - t).abs().max_element() < 1e-3, "{hex:06x}: {t} came back as {back}");
        }
    }

    #[test]
    fn frustum_keeps_what_it_sees() {
        let vp = Mat4::perspective_rh(1.0, 1.0, 0.1, 100.0) * Mat4::look_at_rh(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
        let f = Frustum::new(vp);
        assert!(f.sees(Vec3::new(-1.0, -1.0, -11.0), Vec3::new(1.0, 1.0, -9.0)));
        assert!(!f.sees(Vec3::new(-1.0, -1.0, 9.0), Vec3::new(1.0, 1.0, 11.0)), "behind");
        assert!(!f.sees(Vec3::new(50.0, -1.0, -11.0), Vec3::new(52.0, 1.0, -9.0)), "off to the side");
    }
}
