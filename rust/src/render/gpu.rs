//! wgpu backend: WebGPU in the browser, Direct3D 12 / Vulkan / Metal on desktop.
//!
//! A frame is: a shadow map from the moon (or sun); the scene, into color plus
//! a buffer of normals and depth; contact shading from that buffer; a glow
//! chain; a composite with background blur and grading onto the screen; and
//! the HUD on top.

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2, Vec3, Vec4};
use wgpu::util::DeviceExt;

use super::clusters;
use super::faces::StaticFaces;
use super::overlay::{build_atlas, OVertex, ATLAS_H, ATLAS_W};
use super::preset::{Preset, Shadows};
use super::probes::{self, ProbeBox, ProbeVolume};
use super::profile::{fx, FrameCounts, GpuTimer};
use super::view::{Frame, Instance, InstanceSet, Light, MeshKind, RigDraw, WorldView, FAR, MESH_KINDS, NEAR};
use super::voxels::RigMesh;

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
    // The lifted toe, as the shaders' `toe`: straight at 0.55 of the light in the darks.
    let aces = output * (a / b);
    let lin = c.max(Vec3::ZERO) * 0.55;
    let h = (Vec3::splat(0.004) - (aces - lin).abs()).max(Vec3::ZERO) / 0.004;
    (aces.max(lin) + h * h * 0.001).clamp(Vec3::ZERO, Vec3::ONE)
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
/// Half the width of the fine shadow cascade, in world units.
const NEAR_CASCADE: f32 = 10.0;
const BLOOM_LEVELS: usize = 5;
/// The depth of field (RP02): nothing blurs until this many metres past what the camera looks at,
/// so a foe within reach of the hero (12 m, `camcheck`) stays sharp, and the blur is full
/// (`DOF_PX` at 1080p) this far past it.
pub const DOF_CLEAR: f32 = 12.0;
pub const DOF_FULL: f32 = 27.0;
pub const DOF_PX: f32 = 3.5;
/// Shots with no foes to keep sharp (the story, the title, the character screen, the forge, the
/// tour) keep the softer depth of field they were composed with.
const CINEMA_DOF_CLEAR: f32 = 5.0;
const CINEMA_DOF_FULL: f32 = 20.0;
/// The environment cube (RP14): its first mip's size, and how many mips (roughness 0 to 1).
const ENV_SIZE: u32 = 128;
const ENV_MIPS: u32 = 6;
/// The environment's render passes: each mip's six faces, the first drawn, the rest prefiltered.
const ENV_PASSES: u32 = ENV_MIPS * 6;
const ENV_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
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

/// Faces within a chunk that all face one way (RP21: clusters with a normal cone, here exact):
/// where they start in the face buffer, how many, the box the faces span, and the way they
/// face (0 to 5 as `FACE_NORMALS`, +x, -x, +y, -y, +z, -z), or 6 for faces of turned cubes and
/// of cubes that can fade, which are never culled by facing.
#[derive(Clone, Copy, Debug)]
struct FaceGroup {
    faces: u32,
    count: u32,
    lo: Vec3,
    hi: Vec3,
    dir: u32,
}

/// The way an unturned cube's face `f` looks, or 6 when the cube is turned or mirrored.
fn face_dir(c: &Instance, f: u32) -> u32 {
    let m = &c.m;
    let square = [m[1], m[2], m[4], m[6], m[8], m[9]].iter().all(|v| v.abs() < 1e-5) && m[0] > 0.0 && m[5] > 0.0 && m[10] > 0.0;
    if square {
        f
    } else {
        6
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

/// The coarse cascade's view down the light (toward `sun_dir`) over the box about `center`,
/// `radius` across each way, and its projection: what keys the cached static part (with the
/// scenery's generation), so it holds as long as the light and the level do.
fn far_light(center: Vec3, radius: f32, sun_dir: Vec3) -> (Mat4, Mat4) {
    let l = sun_dir.normalize_or(Vec3::Y);
    let up = if l.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
    let light_view = Mat4::look_at_rh(center + l * 90.0, center, up);
    (light_view, Mat4::orthographic_rh(-radius, radius, -radius, radius, 1.0, 200.0) * light_view)
}

/// Whether a rig mesh's buffers are kept: while its level lives (`pinned`), or while it was drawn
/// in the last `RIG_IDLE_FRAMES`.
fn rig_kept(pinned: bool, used: u64, now: u64) -> bool {
    pinned || now.saturating_sub(used) < RIG_IDLE_FRAMES
}

/// The texels (x, y, width, height) of a `size`-texel cascade under `light_vp` that what lies in the
/// box `b` can shadow, a texel and a half to spare round it; none when that is over a quarter of
/// the map (drawing it all again is as cheap) or off it.
fn far_part(light_vp: Mat4, b: (Vec3, Vec3), size: u32) -> Option<[u32; 4]> {
    if !(b.0.cmple(b.1).all()) {
        return None;
    }
    let (mut lo, mut hi) = (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY));
    for k in 0..8 {
        let p = Vec3::new(
            if k & 1 == 0 { b.0.x } else { b.1.x },
            if k & 2 == 0 { b.0.y } else { b.1.y },
            if k & 4 == 0 { b.0.z } else { b.1.z },
        );
        let c = light_vp.project_point3(p);
        // (Texels run down from the map's top: its NDC y up.)
        let t = Vec2::new((c.x * 0.5 + 0.5) * size as f32, (0.5 - c.y * 0.5) * size as f32);
        lo = lo.min(t);
        hi = hi.max(t);
    }
    let x0 = (lo.x - 1.5).floor().max(0.0) as u32;
    let y0 = (lo.y - 1.5).floor().max(0.0) as u32;
    let x1 = ((hi.x + 1.5).ceil().max(0.0) as u32).min(size);
    let y1 = ((hi.y + 1.5).ceil().max(0.0) as u32).min(size);
    if x1 <= x0 || y1 <= y0 || (x1 - x0) as u64 * (y1 - y0) as u64 * 4 > size as u64 * size as u64 {
        return None;
    }
    Some([x0, y0, x1 - x0, y1 - y0])
}

/// The cascade's view narrowed to texels `r` of its `size` (light `light_view`, half-width
/// `radius`): to find the casters whose column of the light reaches them.
fn far_part_view(light_view: &Mat4, radius: f32, r: [u32; 4], size: u32) -> Mat4 {
    let s = size as f32;
    let at = |t: u32| t as f32 / s * 2.0 - 1.0;
    let (left, right) = (at(r[0]) * radius, at(r[0] + r[2]) * radius);
    let (bottom, top) = (-at(r[1] + r[3]) * radius, -at(r[1]) * radius);
    Mat4::orthographic_rh(left, right, bottom, top, 1.0, 200.0) * *light_view
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
    cluster: [f32; 4],
    haze: [f32; 4],
    probe_min: [f32; 4],
    probe_size: [f32; 4],
    shadow_q: [f32; 4],
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
    haze: [f32; 4],
    vol: [f32; 4],
}

struct MeshBuf {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
}

/// A rig's mesh on the GPU, the frame it was last drawn in, and whether the level keeps it
/// (made ready at its load: never let go while the level lives).
struct RigBuf {
    /// Its faces, bound for the vertex shader to read (`vs_rig`).
    faces: wgpu::BindGroup,
    count: u32,
    used: u64,
    pinned: bool,
}

/// A prebuilt instance set on the GPU, and the frame it was last drawn in.
struct SetBuf {
    buf: wgpu::Buffer,
    count: u32,
    used: u64,
}

/// Probes refreshed on a worker thread, and those that changed.
#[cfg(not(target_arch = "wasm32"))]
type Refreshed = (ProbeVolume, Option<ProbeBox>);

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
    /// The scene's indirect light, multisampled and resolved (RP15).
    msaa_indirect: wgpu::TextureView,
    indirect: wgpu::TextureView,
    /// Reads the multisampled normals and depth for their resolve (nearest sample, `gbuf.wgsl`).
    gbuf_resolve_bind: wgpu::BindGroup,
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
    /// The scene's lights, for the lamps' glow in the air (RP19).
    lights: wgpu::Buffer,
    gbuf_layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
    sampler: wgpu::Sampler,
    blank: wgpu::TextureView,
}

impl PostShared {
    fn bind(&self, device: &wgpu::Device, label: &str, t: [&wgpu::TextureView; 5]) -> wgpu::BindGroup {
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
                wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(t[4]) },
                wgpu::BindGroupEntry { binding: 7, resource: self.lights.as_entire_binding() },
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
        let indirect = texture(device, "indirect", full, HDR, 1, true);
        let msaa_gbuf = texture(device, "msaa gbuf", full, HDR, SAMPLES, true);
        let gbuf_resolve_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gbuf resolve"),
            layout: &post.gbuf_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&msaa_gbuf) }],
        });
        let ssao_bind = post.bind(device, "ssao", [&gbuf, b, b, b, b]);
        let blur_bind = post.bind(device, "blur", [&ao, &gbuf, b, b, b]);
        let rays_bind = post.bind(device, "rays", [&color, &gbuf, b, b, b]);
        let mut down_binds = vec![post.bind(device, "bloom pre", [&color, b, b, b, b])];
        for i in 1..BLOOM_LEVELS {
            down_binds.push(post.bind(device, "bloom down", [&bloom[i - 1], b, b, b, b]));
        }
        let up_binds = (0..BLOOM_LEVELS - 1).map(|i| post.bind(device, "bloom up", [&bloom[i + 1], b, b, b, b])).collect();
        let composite_bind = post.bind(device, "composite", [&color, &gbuf, &ao_blur, &bloom[0], &indirect]);
        Self {
            msaa_color: texture(device, "msaa color", full, COLOR, SAMPLES, false),
            msaa_gbuf,
            msaa_indirect: texture(device, "msaa indirect", full, HDR, SAMPLES, false),
            indirect,
            gbuf_resolve_bind,
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
    /// One shadow map per cascade, each its own size: the fine one round Ragnar, then the whole level.
    shadow_maps: [wgpu::Texture; 2],
    shadow_views: [wgpu::TextureView; 2],
    /// The coarse cascade holding only the static scenery, kept while the light, the level and the
    /// cascade's box stay the same (the static generation and the matrix it was drawn with).
    shadow_cache: wgpu::Texture,
    shadow_cache_view: wgpu::TextureView,
    shadow_cache_key: Option<(u64, [f32; 16])>,
    /// What changed in the static scenery since the cache was drawn, where that is known and local
    /// (a coin taken, a wall broken): only the cache's texels it can shadow are drawn again.
    far_dirty: Option<(Vec3, Vec3)>,
    /// Clears the depth of a part of a map (the part of the cache drawn again).
    depth_clear: wgpu::RenderPipeline,
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
    rig_face_layout: wgpu::BindGroupLayout,
    /// The resident rig meshes by rig id, this frame's draw records and part matrices, and
    /// 0, 1, 2, ... as an instance stream so each draw finds its record.
    rig_meshes: std::collections::HashMap<u64, RigBuf>,
    /// Which of the view's preparations the pinned rig meshes are (`WorldView::prepare_rigs`), and
    /// what making their buffers last cost (ms, bytes).
    resident_gen: u64,
    pub resident_cost: (f64, u64),
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
    gbuf_resolve_pipe: wgpu::RenderPipeline,
    overlay_pipe: wgpu::RenderPipeline,
    globals: wgpu::Buffer,
    scene_bind: wgpu::BindGroup,
    /// What the scene's bind group holds, to make it again when the probes' textures change.
    scene_inputs: SceneInputs,
    /// The level's probes (RP18): built on the level's first frame, refreshed round broken walls.
    probe_tex: Vec<wgpu::Texture>,
    probes: Option<ProbeVolume>,
    /// Probes of levels seen before, by what they were built from (RP03: a return through a
    /// portal finds its level's light ready), the most recent last.
    probe_cache: Vec<(u64, ProbeVolume)>,
    /// A level's probes being built on a worker thread while the loading screen shows (RP03):
    /// what for (the cache key and the static upload), and where the result comes back.
    #[cfg(not(target_arch = "wasm32"))]
    probe_job: Option<(u64, u64, std::sync::mpsc::Receiver<ProbeVolume>)>,
    /// The probes being refreshed round a change on a worker thread (natively), for which static
    /// upload, and where they come back with the probes that changed. The textures keep the light
    /// as it was meanwhile, a frame or two: a coin taken in the vaults cost 11 to 13 ms in the
    /// frame it went (worked out on the spot, every side of every probe sent up again).
    #[cfg(not(target_arch = "wasm32"))]
    probe_refresh: Option<(u64, std::sync::mpsc::Receiver<Refreshed>)>,
    /// The level (full static upload) the probes were built for, the boxes changed since, and the
    /// bounds of every static cube as last sent (so a hidden one's place is known).
    probe_level: u64,
    probe_dirty: Vec<(Vec3, Vec3)>,
    /// The hash of the static cubes for the probe cache's key, and the upload it was worked out for.
    probe_key: Option<(u64, u64)>,
    static_bounds: Vec<(Vec3, Vec3)>,
    static_level: u64,
    /// Clustered lights: every light, each cluster's run, the runs (`clusters`).
    light_buf: wgpu::Buffer,
    cell_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    /// The environment cube's faces by mip (face + 6 x mip), its passes' bind groups and
    /// pipelines, and the light it was last drawn for.
    env_faces: Vec<wgpu::TextureView>,
    env_globals: wgpu::BindGroup,
    env_src: wgpu::BindGroup,
    env_draw_pipe: wgpu::RenderPipeline,
    env_prefilter_pipe: wgpu::RenderPipeline,
    env_key: Option<Vec<u32>>,
    /// The level (static upload) the environment was last drawn for, and its next pass to draw
    /// (`ENV_PASSES` when it is all drawn).
    env_level: u64,
    env_next: u32,
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
    /// The level's faces by chunk and facing (RP21), as the GPU cull lists them, and the same cut
    /// into clusters of at most 64 faces (for comparing granularities).
    face_groups: Vec<FaceGroup>,
    face_clusters: Vec<FaceGroup>,
    extra_faces: Vec<u32>,
    extra_box: (Vec3, Vec3),
    static_layout: wgpu::BindGroupLayout,
    static_bind: Option<wgpu::BindGroup>,
    /// GPU visibility for the static chunks (RP20), where the adapter can draw indirectly.
    cull: Option<GpuCull>,
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
    /// Measure how much of the static scenery in view is hidden behind nearer scenery (RP20:
    /// whether occlusion culling would pay), with a query per chunk in the depth prepass. Slow
    /// (it waits for the answer each frame); for `--perf --occlusion` only.
    pub measure_occlusion: bool,
    occlusion: Option<(wgpu::QuerySet, wgpu::Buffer, wgpu::Buffer, u32)>,
    /// The quality preset (RP16 and the plan's preset policy); change it with `set_preset`.
    preset: Preset,
    /// The last frame handed to the queue, for pacing frames in flight.
    pub last_submit: Option<wgpu::SubmissionIndex>,
    dyn_buf: Dyn,
    overlay_buf: Dyn,
    pub backend: String,
    pub adapter: wgpu::AdapterInfo,
}

/// The Direct3D 12 shader compiler: Microsoft's DXC, when its two DLLs sit next to the exe
/// (build.rs puts them there), compiles the game's shaders many times faster than FXC, wgpu's
/// default: about a second at start-up instead of ten. Without them, FXC. WGPU_DX12_COMPILER
/// overrides.
fn dx12_compiler() -> wgpu::Dx12Compiler {
    if let Some(c) = wgpu::Dx12Compiler::from_env() {
        return c;
    }
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(std::path::PathBuf::from)) {
        let (dxc, dxil) = (dir.join("dxcompiler.dll"), dir.join("dxil.dll"));
        if dxc.exists() && dxil.exists() {
            return wgpu::Dx12Compiler::DynamicDxc {
                dxc_path: dxc.to_string_lossy().into_owned(),
                dxil_path: dxil.to_string_lossy().into_owned(),
                max_shader_model: wgpu::DxcShaderModel::V6_7,
            };
        }
    }
    wgpu::Dx12Compiler::Fxc
}

/// Graphics APIs to start the renderer on, each with the window's surface (`Gpu::surfaces`).
pub type Surfaces = Vec<(wgpu::Instance, Option<wgpu::Surface<'static>>)>;

impl Gpu {
    /// Opens the GPU on a window surface. `size` is in physical pixels.
    pub async fn new(window: Option<Arc<winit::window::Window>>, size: (u32, u32)) -> Result<Self, String> {
        let candidates = Self::surfaces(window.as_ref());
        Self::start(candidates, size, || ()).await
    }

    /// The graphics APIs to try, in order, each with the window's surface made on it. Called on
    /// the window's own thread: Windows gives out a window's handle only there.
    pub fn surfaces(window: Option<&Arc<winit::window::Window>>) -> Surfaces {
        // Direct3D 12 first on Windows: it is what the Steam overlay and capture
        // tools know best. Anything else (or WGPU_BACKEND) as the fallback.
        let preferred = if cfg!(windows) { wgpu::Backends::DX12 } else { wgpu::Backends::all() };
        let order = match wgpu::Backends::from_env() {
            Some(b) => vec![b],
            None => vec![preferred, wgpu::Backends::all()],
        };
        let mut out = Vec::new();
        for backends in order {
            let backend_options =
                wgpu::BackendOptions { dx12: wgpu::Dx12BackendOptions { shader_compiler: dx12_compiler() }, ..Default::default() };
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor { backends, backend_options, ..Default::default() });
            let surface = match window {
                Some(w) => match instance.create_surface(w.clone()) {
                    Ok(s) => Some(s),
                    Err(_) => continue,
                },
                None => None,
            };
            out.push((instance, surface));
        }
        out
    }

    /// Starts the renderer on the first of `candidates` with a GPU for it (any thread will do).
    /// With a window, the engine's loading screen draws in it while the pipelines compile, and
    /// `painted` is called once it is up (the window may be shown from then on).
    pub async fn start(candidates: Surfaces, size: (u32, u32), painted: impl FnOnce() + Send + 'static) -> Result<Self, String> {
        let mut found = None;
        for (instance, surface) in candidates {
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
        #[allow(unused_mut)]
        let (mut surface, adapter) = found.ok_or_else(|| "no GPU adapter supports this window".to_string())?;
        let info = adapter.get_info();
        let backend = format!("{:?} {}", info.backend, info.name);
        let adapter_info = info.clone();
        let limits = if cfg!(target_arch = "wasm32") {
            wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())
        } else {
            wgpu::Limits::default().using_resolution(adapter.limits())
        };
        // Timestamps for the profiler, where the adapter has them.
        // GPU visibility (RP20) wants indirect multi-draws with a first instance, and counted
        // ones where the adapter has them (DX12, Vulkan 1.2).
        let features = adapter.features()
            & (wgpu::Features::TIMESTAMP_QUERY
                | wgpu::Features::MULTI_DRAW_INDIRECT
                | wgpu::Features::MULTI_DRAW_INDIRECT_COUNT
                | wgpu::Features::INDIRECT_FIRST_INSTANCE);
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
        // The engine's loading screen, while the shaders compile (seconds on DX12): on its own
        // thread, with the surface, until the renderer is ready. (If it cannot be made, the
        // window is painted plain instead.)
        #[cfg(not(target_arch = "wasm32"))]
        let progress = super::splash::Progress::default();
        #[cfg(not(target_arch = "wasm32"))]
        let loading = match surface.take() {
            Some(s) => match super::splash::Splash::new(&device, &queue, format, (config.width, config.height)) {
                Some(screen) => Some(screen.run(s, device.clone(), queue.clone(), config.clone(), progress.clone(), painted)),
                None => {
                    super::splash::clear(&device, &queue, &s);
                    surface = Some(s);
                    painted();
                    None
                }
            },
            None => {
                painted();
                None
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        let step = |n: u32| progress.step(n);
        #[cfg(target_arch = "wasm32")]
        let step = |_: u32| ();
        #[cfg(target_arch = "wasm32")]
        painted();
        // The checklist's steps: the device is up; then the runtime, then the shaders.
        step(1);

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
        let preset = Preset::default_for_build();
        let (shadow_maps, shadow_views, shadow_cache, shadow_cache_view) = shadow_targets(&device, &preset.shadows);
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        // ---- clustered lights (RP17): every light, each cluster's run, and the runs, at fixed sizes
        let light_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let storage = |label: &str, size: u64| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        // The level's probes (RP18): six 3D textures, one per side.
        let probe_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D3,
                multisampled: false,
            },
            count: None,
        };
        let shadow_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Depth,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let light_buf = storage("lights", (clusters::MAX_LIGHTS * std::mem::size_of::<GpuLight>()) as u64);
        let cell_buf = storage("light clusters", clusters::CELLS as u64 * 4);
        let index_buf = storage("light indices", (clusters::CELLS * clusters::CELL_CAP) as u64 * 4);
        let scene_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene"),
            entries: &[
                uniform_entry(0, wgpu::ShaderStages::VERTEX_FRAGMENT),
                shadow_entry(1),
                shadow_entry(16),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                light_entry(7),
                light_entry(8),
                light_entry(9),
                probe_entry(10),
                probe_entry(11),
                probe_entry(12),
                probe_entry(13),
                probe_entry(14),
                probe_entry(15),
            ],
        });
        // ---- the environment (RP14): a cube of the surroundings as light, a mip per roughness,
        // and the split-sum lookup (`brdf`)
        let env_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("environment"),
            size: wgpu::Extent3d { width: ENV_SIZE, height: ENV_SIZE, depth_or_array_layers: 6 },
            mip_level_count: ENV_MIPS,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: ENV_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let cube_view = |base_mip_level: u32, mip_level_count: Option<u32>| {
            env_tex.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::Cube),
                base_mip_level,
                mip_level_count,
                ..Default::default()
            })
        };
        let env_view = cube_view(0, None);
        let env_first = cube_view(0, Some(1));
        let env_faces: Vec<wgpu::TextureView> = (0..ENV_MIPS * 6)
            .map(|k| {
                env_tex.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_mip_level: k / 6,
                    mip_level_count: Some(1),
                    base_array_layer: k % 6,
                    array_layer_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        let env_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("environment"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let lut = super::brdf::lut_texels();
        let lut_view = device
            .create_texture_with_data(
                &queue,
                &wgpu::TextureDescriptor {
                    label: Some("brdf lookup"),
                    size: wgpu::Extent3d { width: super::brdf::LUT_SIZE, height: super::brdf::LUT_SIZE, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba16Float,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::LayerMajor,
                bytemuck::cast_slice(&lut),
            )
            .create_view(&Default::default());
        let scene_inputs = SceneInputs {
            layout: scene_layout.clone(),
            globals: globals.clone(),
            shadow_views: shadow_views.clone(),
            shadow_sampler: shadow_sampler.clone(),
            env_view: env_view.clone(),
            env_sampler: env_sampler.clone(),
            lut_view: lut_view.clone(),
            light_buf: light_buf.clone(),
            cell_buf: cell_buf.clone(),
            index_buf: index_buf.clone(),
        };
        let probe_tex = probe_textures(&device, [1, 1, 1]);
        let scene_bind = scene_inputs.bind(&device, &probe_tex);
        // The environment's own passes see only the globals (the cube they draw into is in the
        // scene's group), and the prefilter its first mip.
        let env_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("environment globals"),
            entries: &[uniform_entry(0, wgpu::ShaderStages::VERTEX_FRAGMENT)],
        });
        let env_src_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("environment source"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let env_globals = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("environment globals"),
            layout: &env_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals.as_entire_binding() }],
        });
        let env_src = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("environment source"),
            layout: &env_src_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&env_first) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&env_sampler) },
            ],
        });
        let env_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("environment"),
            source: wgpu::ShaderSource::Wgsl(concat!(include_str!("shaders.wgsl"), include_str!("env.wgsl")).into()),
        });
        let env_pipe = |label: &str, fs: &str, layouts: &[&wgpu::BindGroupLayout]| {
            let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: layouts,
                push_constant_ranges: &[],
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pl),
                vertex: wgpu::VertexState {
                    module: &env_shader,
                    entry_point: Some("vs_env"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &env_shader,
                    entry_point: Some(fs),
                    targets: &[Some(wgpu::ColorTargetState { format: ENV_FORMAT, blend: None, write_mask: wgpu::ColorWrites::ALL })],
                    compilation_options: Default::default(),
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        };
        let env_draw_pipe = env_pipe("environment", "fs_env", &[&env_layout]);
        let env_prefilter_pipe = env_pipe("environment prefilter", "fs_prefilter", &[&env_layout, &env_src_layout]);
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
        step(2);
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
        // Color, then normals and depth, then indirect light; see-through passes write only the color.
        let scene_targets = |blend: Option<wgpu::BlendState>, gbuf: bool| {
            let solid = if gbuf { wgpu::ColorWrites::ALL } else { wgpu::ColorWrites::empty() };
            [
                Some(wgpu::ColorTargetState { format: COLOR, blend, write_mask: wgpu::ColorWrites::ALL }),
                Some(wgpu::ColorTargetState { format: HDR, blend: None, write_mask: solid }),
                Some(wgpu::ColorTargetState { format: HDR, blend: None, write_mask: solid }),
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
                        [COLOR, HDR, HDR]
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
        // Each rig mesh's faces, read by the vertex shader (six vertices a face, their corners made there).
        let rig_face_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("rig faces"), entries: &[storage_entry(0)] });
        let rig_scene_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rig scene"),
            bind_group_layouts: &[&scene_layout, &rig_layout, &rig_face_layout],
            push_constant_ranges: &[],
        });
        let rig_shadow_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rig shadow"),
            bind_group_layouts: &[&shadow_layout, &rig_layout, &rig_face_layout],
            push_constant_ranges: &[],
        });
        // The draw's record: one instance per rig drawn, its id from the stream 0, 1, 2, ...
        let rig_vertex_layouts = [wgpu::VertexBufferLayout {
            array_stride: 4,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &wgpu::vertex_attr_array![0 => Uint32],
        }];
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
                        [COLOR, HDR, HDR]
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
                        [COLOR, HDR, HDR]
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
        // A part of the cached cascade drawn again: its depth cleared to the far plane first.
        let depth_clear = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("depth clear"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("depth clear"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            })),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_depth_clear"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: None,
            primitive: Default::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: Default::default(),
                bias: Default::default(),
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
                tex_entry(6),
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        // The normals-and-depth resolve: the multisampled buffer, read sample by sample.
        let gbuf_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gbuf resolve"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: true,
                },
                count: None,
            }],
        });
        let cull = GpuCull::new(&device);
        let gbuf_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gbuf resolve"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gbuf.wgsl").into()),
        });
        let gbuf_resolve_pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gbuf resolve"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("gbuf resolve"),
                bind_group_layouts: &[&gbuf_layout],
                push_constant_ranges: &[],
            })),
            vertex: wgpu::VertexState {
                module: &gbuf_shader,
                entry_point: Some("vs_resolve"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &gbuf_shader,
                entry_point: Some("fs_resolve"),
                targets: &[Some(wgpu::ColorTargetState { format: HDR, blend: None, write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
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
            lights: scene_inputs.light_buf.clone(),
            gbuf_layout,
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
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(screen) = loading {
            surface = Some(screen.finish());
        }
        Ok(Self {
            shadow_maps,
            shadow_views,
            preset,
            shadow_cache,
            shadow_cache_view,
            shadow_cache_key: None,
            far_dirty: None,
            depth_clear,
            static_chunks: Vec::new(),
            face_buf: Dyn::new(&device, "static faces", wgpu::BufferUsages::VERTEX, 1 << 20),
            static_faces: None,
            faces_main: Vec::new(),
            face_groups: Vec::new(),
            face_clusters: Vec::new(),
            extra_faces: Vec::new(),
            extra_box: (Vec3::INFINITY, Vec3::NEG_INFINITY),
            static_layout,
            static_bind: None,
            cull,
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
            measure_occlusion: false,
            occlusion: None,
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
            rig_face_layout,
            rig_meshes: std::collections::HashMap::new(),
            resident_gen: 0,
            resident_cost: (0.0, 0),
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
            gbuf_resolve_pipe,
            overlay_pipe,
            globals,
            scene_bind,
            scene_inputs,
            probe_tex,
            probes: None,
            probe_cache: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            probe_job: None,
            #[cfg(not(target_arch = "wasm32"))]
            probe_refresh: None,
            probe_level: 0,
            probe_dirty: Vec::new(),
            probe_key: None,
            static_bounds: Vec::new(),
            static_level: 0,
            light_buf,
            cell_buf,
            index_buf,
            env_faces,
            env_globals,
            env_src,
            env_draw_pipe,
            env_prefilter_pipe,
            env_key: None,
            env_level: 0,
            env_next: ENV_PASSES,
            screen,
            overlay_bind,
            meshes,
            static_count: 0,
            backend,
            adapter: adapter_info,
        })
    }

    /// The view's level is up and lit: its static scenery uploaded and its probes worked out
    /// (RP03: the loading screen waits for this).
    pub fn level_ready(&self, view: &WorldView) -> bool {
        !view.static_dirty
            && self.static_count > 0
            && self.probes_held()
            && self.probe_level == self.static_level
            && self.resident_gen == view.resident().gen
    }

    /// How many facing groups and 64-face clusters the level's static faces make (RP21).
    pub fn face_cluster_counts(&self) -> (usize, usize) {
        (self.face_groups.len(), self.face_clusters.len())
    }

    /// The quality preset in use.
    pub fn preset(&self) -> &Preset {
        &self.preset
    }

    /// Switches the quality preset: the shadow maps are made again when their sizes change (and
    /// the static cascade's cache drawn again).
    pub fn set_preset(&mut self, preset: Preset) {
        if preset.shadows.near_size != self.preset.shadows.near_size || preset.shadows.far_size != self.preset.shadows.far_size {
            let (maps, views, cache, cache_view) = shadow_targets(&self.device, &preset.shadows);
            self.shadow_maps = maps;
            self.shadow_views = views.clone();
            self.shadow_cache = cache;
            self.shadow_cache_view = cache_view;
            self.shadow_cache_key = None;
            self.scene_inputs.shadow_views = views;
            self.scene_bind = self.scene_inputs.bind(&self.device, &self.probe_tex);
        }
        self.preset = preset;
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
        self.sync_resident(view);
        if !view.static_dirty && !self.sync_static_changes(view) {
            view.static_dirty = true;
        }
        if !view.static_dirty {
            return;
        }
        let (order, mut chunks) = chunk_static(&view.static_cubes);
        self.static_slot = Vec::with_capacity(order.len() + STATIC_SPARE);
        self.static_slot.resize(order.len(), 0);
        let mut sorted = Vec::with_capacity(order.len());
        for (slot, &i) in order.iter().enumerate() {
            sorted.push(view.static_cubes[i as usize]);
            self.static_slot[i as usize] = slot as u32;
        }
        let mut index = StaticFaces::build(&sorted);
        // Room for the cubes added later (rubble, patches), so the first of them grows no table
        // of a million entries (copying them cost milliseconds the frame a wall fell).
        index.reserve(STATIC_SPARE);
        // Cubes that can fade keep their back faces: they go with the turned ones.
        let mut fades = vec![false; sorted.len()];
        for (start, len) in view.fade_ranges() {
            for i in start..(start + len).min(self.static_slot.len()) {
                fades[self.static_slot[i] as usize] = true;
            }
        }
        // Each chunk's faces, grouped by the way they face (RP21).
        let mut faces: Vec<u32> = Vec::new();
        let mut groups: Vec<FaceGroup> = Vec::new();
        for ch in chunks.iter_mut() {
            ch.faces = faces.len() as u32;
            let mut by: [Vec<u32>; 7] = Default::default();
            let mut boxes = [(Vec3::INFINITY, Vec3::NEG_INFINITY); 7];
            for slot in ch.start..ch.start + ch.count {
                let c = &sorted[slot as usize];
                let (at, half) = cube_bounds(c);
                let open = index.open[slot as usize];
                for f in 0..6u32 {
                    if open & (1 << f) == 0 {
                        continue;
                    }
                    let d = if fades[slot as usize] { 6 } else { face_dir(c, f) };
                    by[d as usize].push(slot << 3 | f);
                    // The face itself: flat along the way it faces (so the box's sides along that
                    // axis are the face planes, for the facing test), its full size across.
                    let (lo, hi) = if d < 6 {
                        let n = FACE_AXES[d as usize];
                        let centre = at + n * half;
                        let across = half * (Vec3::ONE - n.abs());
                        (centre - across, centre + across)
                    } else {
                        (at - half, at + half)
                    };
                    boxes[d as usize] = (boxes[d as usize].0.min(lo), boxes[d as usize].1.max(hi));
                }
            }
            for (d, list) in by.iter().enumerate() {
                if !list.is_empty() {
                    groups.push(FaceGroup {
                        faces: faces.len() as u32,
                        count: list.len() as u32,
                        lo: boxes[d].0,
                        hi: boxes[d].1,
                        dir: d as u32,
                    });
                    faces.extend_from_slice(list);
                }
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
        self.static_bounds = Vec::with_capacity(sorted.len() + STATIC_SPARE);
        self.static_bounds.extend(sorted.iter().map(|c| {
            let (at, half) = cube_bounds(c);
            (at - half, at + half)
        }));
        self.static_level += 1;
        // The same groups in clusters of at most 64 faces, each with its own box.
        let mut clusters = Vec::new();
        for g in &groups {
            for start in (0..g.count).step_by(64) {
                let n = (g.count - start).min(64);
                let (mut lo, mut hi) = (Vec3::INFINITY, Vec3::NEG_INFINITY);
                for &code in &faces[(g.faces + start) as usize..(g.faces + start + n) as usize] {
                    let (at, half) = cube_bounds(&sorted[(code >> 3) as usize]);
                    let (a, b) = if g.dir < 6 {
                        let ax = FACE_AXES[g.dir as usize];
                        let c = at + ax * half;
                        let across = half * (Vec3::ONE - ax.abs());
                        (c - across, c + across)
                    } else {
                        (at - half, at + half)
                    };
                    lo = lo.min(a);
                    hi = hi.max(b);
                }
                clusters.push(FaceGroup { faces: g.faces + start, count: n, lo, hi, dir: g.dir });
            }
        }
        // The GPU culls by the clusters (RP21): of the three grains they drop the most.
        if let Some(cull) = self.cull.as_mut() {
            cull.upload(&self.device, &self.queue, &clusters);
        }
        self.face_groups = groups;
        self.face_clusters = clusters;
        self.static_chunks = chunks;
        self.static_faces = Some(index);
        self.faces_main = faces;
        self.extra_faces.clear();
        self.extra_box = (Vec3::INFINITY, Vec3::NEG_INFINITY);
        self.static_gen += 1;
        self.far_dirty = None;
        view.static_dirty = false;
        view.static_patches.clear();
    }

    /// The changes since the last upload, sent as they are. False when they need the whole level sent again.
    fn sync_static_changes(&mut self, view: &mut WorldView) -> bool {
        let Some(index) = self.static_faces.as_mut() else { return view.static_cubes.is_empty() };
        let stride = std::mem::size_of::<Instance>() as u64;
        let mut extras_changed = false;
        // (The boxes what changed lay in, as the probes are told of them, and where moved cubes went.)
        let dirt_from = self.probe_dirty.len();
        let mut went: Vec<(Vec3, Vec3)> = Vec::new();
        // A cube hidden or moved changes the shadows too (RP04: a wall blown away no longer
        // casts in the cached cascade), even where no neighbour's face opens.
        let mut moved = false;
        let grow = |b: &mut (Vec3, Vec3), c: &Instance| {
            let (at, half) = cube_bounds(c);
            // Its neighbours lie within one of its own sizes.
            b.0 = b.0.min(at - half * 3.0);
            b.1 = b.1.max(at + half * 3.0);
        };
        // Cubes added since: into the spare slots, with their open faces in the extra run. (They
        // take the next slots in turn, so they go up in one write: a wall's rubble, written cube by
        // cube, cost milliseconds in the frame it fell.)
        let first = self.static_count;
        let added = view.static_cubes.len().saturating_sub(self.static_slot.len());
        if added > 0 && (first as u64 + added as u64) * stride > self.static_buf.cap {
            return false;
        }
        while self.static_slot.len() < view.static_cubes.len() {
            let slot = self.static_count;
            let c = view.static_cubes[self.static_slot.len()];
            let (at, half) = cube_bounds(&c);
            self.static_bounds.push((at - half, at + half));
            self.probe_dirty.push((at - half, at + half));
            self.static_slot.push(slot);
            self.static_count += 1;
            let open = index.add(slot, &c);
            push_faces(&mut self.extra_faces, slot, open);
            grow(&mut self.extra_box, &c);
            extras_changed = true;
        }
        if added > 0 {
            let bytes: &[u8] = bytemuck::cast_slice(&view.static_cubes[view.static_cubes.len() - added..]);
            self.queue.write_buffer(&self.static_buf.buf, first as u64 * stride, bytes);
            self.pending_upload += bytes.len() as u64;
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
                    // A cube hidden (or moved) changes what the probes round it see.
                    let (at, half) = cube_bounds(&view.static_cubes[k]);
                    if let Some(&(lo, hi)) = self.static_bounds.get(slot as usize) {
                        if (lo - (at - half)).length_squared() + (hi - (at + half)).length_squared() > 1e-8 {
                            moved = true;
                            self.probe_dirty.push((lo, hi));
                            if half.max_element() > 0.0 {
                                went.push((at - half, at + half));
                            }
                            self.static_bounds[slot as usize] = (at - half, at + half);
                        }
                    }
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
        if moved || extras_changed {
            // Shadows change too: the cached cascade's part over what changed is drawn again. (The
            // faces a hidden cube opens lie on its own bounds.)
            let mut b = self.far_dirty.unwrap_or((Vec3::INFINITY, Vec3::NEG_INFINITY));
            for &(lo, hi) in self.probe_dirty[dirt_from..].iter().chain(&went) {
                b = (b.0.min(lo), b.1.max(hi));
            }
            self.far_dirty = Some(b);
        }
        if moved && !extras_changed {
            self.static_gen += 1;
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
        // ---- lights, in priority order (the hero's lantern first, then nearest the eye), binned
        // into the view's clusters (RP17)
        let eye = view.camera_position();
        let mut chosen: Vec<&Light> = frame.lights.iter().collect();
        counts.lights = chosen.len() as u32;
        if chosen.len() > 1 {
            chosen[1..].sort_by(|a, b| a.pos.distance_squared(eye).total_cmp(&b.pos.distance_squared(eye)));
        }
        chosen.truncate(clusters::MAX_LIGHTS);
        let gpu_lights: Vec<GpuLight> = chosen
            .iter()
            .map(|l| GpuLight { pos: [l.pos.x, l.pos.y, l.pos.z, l.distance], color: [l.color.x, l.color.y, l.color.z, 0.0] })
            .collect();
        let reach: Vec<clusters::Reach> = chosen.iter().map(|l| clusters::Reach { at: l.pos, range: l.distance }).collect();
        let (_, cam_fwd_unit, _, _, _) = view.camera_basis();
        let grid =
            clusters::build(&reach, &clusters::ClusterView { view_proj: view.view_proj(), eye, fwd: cam_fwd_unit, near: NEAR, far: FAR });
        counts.lights_dropped = grid.dropped;
        counts.light_pairs = grid.pairs;
        counts.busiest_cluster = grid.busiest;
        if !gpu_lights.is_empty() {
            self.queue.write_buffer(&self.light_buf, 0, bytemuck::cast_slice(&gpu_lights));
        }
        self.queue.write_buffer(&self.cell_buf, 0, bytemuck::cast_slice(&grid.cells));
        if !grid.indices.is_empty() {
            self.queue.write_buffer(&self.index_buf, 0, bytemuck::cast_slice(&grid.indices));
        }
        counts.upload += (std::mem::size_of_val(gpu_lights.as_slice()) + grid.cells.len() * 4 + grid.indices.len() * 4) as u64;
        let atm = frame.atmosphere;
        let use_probes = self.sync_probes(view, frame);
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
        let (light_view, light_vp) = far_light(center, radius, atm.sun_dir);
        let near_r = NEAR_CASCADE;
        let light_vp_near = match view.shadow_focus() {
            Some(focus) => {
                let f = light_view.transform_point3(focus);
                let texel = 2.0 * near_r / self.preset.shadows.near_size as f32;
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
            cam_fwd: v4(fwd, 0.0),
            fog,
            params: [atm.fog_far.min(FAR), atm.exposure, frame.coin_pulse, chosen.len() as f32],
            cluster: [self.config.width as f32, self.config.height as f32, NEAR, FAR],
            haze: [frame.haze.density, frame.haze.floor, frame.haze.falloff, 0.0],
            probe_min: [-0.5, probes::Y0 - probes::DY * 0.5, -0.5, if use_probes { 1.0 } else { 0.0 }],
            // (From the textures: the volume itself may be out being refreshed.)
            probe_size: self
                .probe_tex
                .first()
                .map_or([1.0; 4], |t| [t.width() as f32, probes::LAYERS as f32 * probes::DY, t.depth_or_array_layers() as f32, 0.0]),
            shadow_q: [self.preset.shadows.search_taps as f32, self.preset.shadows.filter_taps as f32, 0.0, 0.0],
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
            shadow: [
                near_r,
                radius,
                if light_vp_near.is_some() { 1.0 } else { 0.0 },
                (self.fx_off & (fx::SCENE | fx::BACKFACE | fx::LIGHT_HEAT | fx::HAZE)) as f32,
            ],
        };
        self.queue.write_buffer(&self.cascade_bufs[0], 0, bytemuck::cast_slice(&light_vp_near.unwrap_or(light_vp).to_cols_array()));
        self.queue.write_buffer(&self.cascade_bufs[1], 0, bytemuck::cast_slice(&light_vp.to_cols_array()));
        self.queue.write_buffer(&self.globals, 0, bytemuck::bytes_of(&g));
        let focus = view.focus_depth();
        let post = PostUniform {
            view_proj: view_proj.to_cols_array(),
            cam_pos: v4(cam, if ortho { 1.0 } else { 0.0 }),
            cam_fwd: v4(fwd, focus.unwrap_or(0.0)),
            cam_r: v4(cam_r, if view.cinematic() { CINEMA_DOF_CLEAR } else { DOF_CLEAR }),
            cam_u: v4(cam_u, if view.cinematic() { CINEMA_DOF_FULL } else { DOF_FULL }),
            params: [
                0.32,
                if self.fx_off & fx::AO != 0 { 0.0 } else { 1.4 },
                if self.fx_off & fx::BLOOM != 0 { 0.0 } else { 0.45 * atm.bloom },
                if focus.is_some() && self.fx_off & fx::DOF == 0 { DOF_PX } else { 0.0 },
            ],
            params2: [0.6, 0.28, frame.time, if self.fx_off & fx::GRADE != 0 { 0.0 } else { 1.0 }],
            moon: moon_on_screen(sky_fwd, sky_right, sky_up, atm.moon_view, atm.night_sky),
            // The air: the haze, and a little scattering in the fog itself (so lamps glow in it too).
            haze: [frame.haze.density, frame.haze.floor, frame.haze.falloff, if frame.haze.density > 0.0 { 0.004 } else { 0.0 }],
            vol: [
                if self.fx_off & fx::VOLUMETRIC != 0 || frame.haze.density <= 0.0 {
                    0.0
                } else {
                    // (Not his own lantern: it hangs over him wherever he goes, and its glow read as a
                    // disc stuck to the middle of the screen.)
                    gpu_lights.len().min(self.preset.volumetric_lamps + usize::from(frame.lantern)) as f32
                },
                atm.exposure,
                0.8,
                if frame.lantern { 1.0 } else { 0.0 },
            ],
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
        // RP21's comparison: the faces the camera would draw at each granularity of culling.
        if use_static && self.timer.is_some() {
            let fr = Frustum::new(view_proj);
            let eye = view.camera_position();
            let faces_toward = |g: &FaceGroup| {
                if g.dir >= 6 || self.fx_off & fx::BACKFACE != 0 {
                    return true;
                }
                let n = FACE_AXES[g.dir as usize];
                if ortho {
                    return (-fwd).dot(n) > 0.0;
                }
                let lowest = if n.x + n.y + n.z > 0.0 { g.lo.dot(n) } else { g.hi.dot(n) };
                eye.dot(n) > lowest
            };
            counts.faces_by_chunk = self.static_chunks.iter().filter(|c| fr.sees(c.min, c.max)).map(|c| c.face_count as u64).sum();
            counts.faces_by_group =
                self.face_groups.iter().filter(|g| fr.sees(g.lo, g.hi) && faces_toward(g)).map(|g| g.count as u64).sum();
            counts.faces_by_cluster =
                self.face_clusters.iter().filter(|g| fr.sees(g.lo, g.hi) && faces_toward(g)).map(|g| g.count as u64).sum();
        }
        counts.chunks = self.static_chunks.len() as u32;
        counts.static_total = self.static_count as u64;
        counts.chunks_drawn = main_chunks.len() as u32;

        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        // ---- the static chunks each view sees, worked out on the GPU (RP20)
        let measuring = self.measure_occlusion && use_static && self.fx_off & fx::PREPASS == 0 && !main_faces.is_empty();
        let gpu_cull = use_static && !measuring && self.preset.gpu_cull && self.cull.as_ref().is_some_and(|c| c.chunks > 0);
        if measuring {
            let need = main_faces.len() as u32;
            if self.occlusion.as_ref().is_none_or(|o| o.3 < need) {
                let cap = need.next_power_of_two().max(256);
                let set = self.device.create_query_set(&wgpu::QuerySetDescriptor {
                    label: Some("occlusion"),
                    ty: wgpu::QueryType::Occlusion,
                    count: cap,
                });
                let resolve = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("occlusion resolve"),
                    size: cap as u64 * 8,
                    usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                });
                let read = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("occlusion read"),
                    size: cap as u64 * 8,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                self.occlusion = Some((set, resolve, read, cap));
            }
        }
        if gpu_cull {
            let extra =
                (!self.extra_faces.is_empty()).then_some((self.extra_box, self.faces_main.len() as u32, self.extra_faces.len() as u32));
            let views = [view_proj, light_vp_near.unwrap_or(light_vp), light_vp];
            // Which way each view looks, to drop the face groups turned from it (RP21): the
            // camera from its eye (a point; or along its gaze when orthographic), the cascades
            // along the light. (None when back faces are kept, `--fx-off backface`.)
            let away = -l;
            let eyes = if self.fx_off & fx::BACKFACE != 0 {
                [Vec4::new(0.0, 0.0, 0.0, 2.0); 3]
            } else {
                let camera = if ortho { (-fwd).extend(0.0) } else { view.camera_position().extend(1.0) };
                [camera, (-away).extend(0.0), (-away).extend(0.0)]
            };
            let timer = self.timer.as_mut().and_then(|t| t.compute_pass("cull"));
            if let Some(cull) = self.cull.as_mut() {
                cull.run(&self.queue, &mut enc, &views, &eyes, extra, timer);
            }
        }
        if use_static && !cache_fresh {
            // Where only the scenery changed, and somewhere small (a coin taken, a wall broken), only
            // the cache's texels over it are drawn again: cleared, then every caster whose column
            // of the light reaches them, as a whole redraw would draw them there. Else all of it.
            let size = self.preset.shadows.far_size;
            let part = match (self.shadow_cache_key, self.far_dirty) {
                // (`VH_FAR_WHOLE` draws it all again, to check the part against.)
                (Some((_, m)), Some(b)) if m == cache_key.1 && std::env::var_os("VH_FAR_WHOLE").is_none() => far_part(light_vp, b, size),
                _ => None,
            };
            let far = match part {
                Some(r) => visible(far_part_view(&light_view, radius, r, size)),
                None => visible(light_vp),
            };
            if std::env::var_os("VH_PROBE_STATS").is_some() {
                eprintln!("shadow cache: {part:?} of {size}, {} casters at {}", far.iter().map(|c| c.1).sum::<u32>(), self.frame_no);
            }
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow cache"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_cache_view,
                    depth_ops: Some(wgpu::Operations {
                        load: if part.is_some() { wgpu::LoadOp::Load } else { wgpu::LoadOp::Clear(1.0) },
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: self.timer.as_mut().and_then(|t| t.pass("shadow cache")),
                occlusion_query_set: None,
            });
            if let Some([x, y, w, h]) = part {
                pass.set_scissor_rect(x, y, w, h);
                pass.set_pipeline(&self.depth_clear);
                pass.draw(0..3, 0..1);
            }
            pass.set_bind_group(0, &self.shadow_binds[1], &[]);
            if gpu_cull {
                self.draw_faces_gpu(&mut pass, &self.face_shadow, 2, &mut counts);
            } else {
                self.draw_faces(&mut pass, &self.face_shadow, &far, &mut counts);
            }
            counts.shadow_static += far.iter().map(|c| c.1 as u64).sum::<u64>();
            drop(pass);
            self.shadow_cache_key = Some(cache_key);
            self.far_dirty = None;
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
                    texture: &self.shadow_maps[1],
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                self.shadow_maps[1].size(),
            );
        }

        // ---- the environment, drawn again when the light it shows changes. (It is seen by world
        // direction, not from the camera: keyed by the camera's heading too, it was drawn again,
        // 36 passes and up to 13 ms of the CPU's frame, whenever the chase camera turned.)
        let env_key: Vec<u32> = [atm.hemi_sky, atm.hemi_ground, atm.moon_view]
            .iter()
            .flat_map(|v| v.to_array())
            .chain([atm.night_sky, atm.exposure, atm.background as f32])
            .map(f32::to_bits)
            .collect();
        // A new level's (or the first) is drawn whole at once; the light changing within a level
        // (the Pit's on the way down, a few steps a second) is followed a mip a frame, six passes,
        // so no frame records all 36 (1 to 9 ms of the CPU's frame, several times a descent).
        if self.env_key.as_ref() != Some(&env_key) {
            let fresh = self.env_key.is_none() || self.env_level != self.static_level;
            self.env_key = Some(env_key);
            self.env_level = self.static_level;
            self.env_next = 0;
            if fresh {
                self.env_passes(&mut enc, ENV_PASSES, &mut counts);
            }
        }
        if self.env_next < ENV_PASSES {
            self.env_passes(&mut enc, 6, &mut counts);
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
                    view: &self.shadow_views[c],
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
            if c == 0 && gpu_cull {
                self.draw_faces_gpu(&mut pass, &self.face_shadow, 1, &mut counts);
                let n = near_chunks.iter().map(|c| c.1 as u64).sum::<u64>();
                counts.shadow_static += n;
                counts.shadow_near += n;
            } else if c == 0 && !near_chunks.is_empty() {
                self.draw_faces(&mut pass, &self.face_shadow, &near_chunks, &mut counts);
                let n = near_chunks.iter().map(|c| c.1 as u64).sum::<u64>();
                counts.shadow_static += n;
                counts.shadow_near += n;
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
                    // Kept, not averaged: `gbuf_resolve` picks each pixel's nearest sample after the pass.
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.rt.msaa_gbuf,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 1.0, b: 0.0, a: 10000.0 }),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.rt.msaa_indirect,
                        resolve_target: Some(&self.rt.indirect),
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Discard },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.rt.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }),
                    stencil_ops: None,
                }),
                timestamp_writes: self.timer.as_mut().and_then(|t| t.pass("main")),
                occlusion_query_set: if measuring { self.occlusion.as_ref().map(|o| &o.0) } else { None },
            });
            pass.set_bind_group(0, &self.scene_bind, &[]);
            if atm.night_sky > 0.0 {
                pass.set_pipeline(&self.sky_pipe);
                pass.draw(0..3, 0..1);
                counts.draw(1, 3);
            }

            // The depth of every opaque thing first (nearest static chunks first), then the shading.
            let prepass = self.fx_off & fx::PREPASS == 0;
            if measuring {
                // Nearest first, each chunk counted: a chunk none of whose samples pass lies
                // wholly behind the nearer scenery drawn before it.
                for (i, run) in main_faces.iter().enumerate() {
                    pass.begin_occlusion_query(i as u32);
                    self.draw_faces(&mut pass, &self.face_prepass, std::slice::from_ref(run), &mut counts);
                    pass.end_occlusion_query();
                }
            } else if prepass && gpu_cull {
                self.draw_faces_gpu(&mut pass, &self.face_prepass, 0, &mut counts);
            } else if prepass && !main_faces.is_empty() {
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
            if gpu_cull {
                self.draw_faces_gpu(&mut pass, &self.face_opaque, 0, &mut counts);
            } else if !main_faces.is_empty() {
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
        fullscreen(&mut enc, "gbuf resolve", &rt.gbuf, &self.gbuf_resolve_pipe, &rt.gbuf_resolve_bind, true, timer, c);
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
        if let (true, Some((set, resolve, read, _))) = (measuring, &self.occlusion) {
            let n = main_faces.len() as u32;
            enc.resolve_query_set(set, 0..n, resolve, 0);
            enc.copy_buffer_to_buffer(resolve, 0, read, 0, n as u64 * 8);
        }
        if let Some(t) = self.timer.as_mut() {
            t.resolve(&mut enc);
        }
        self.last_submit = Some(self.queue.submit([enc.finish()]));
        if let (true, Some((_, _, read, _))) = (measuring, &self.occlusion) {
            let n = main_faces.len();
            let slice = read.slice(..n as u64 * 8);
            slice.map_async(wgpu::MapMode::Read, |_| {});
            let _ = self.device.poll(wgpu::PollType::Wait);
            {
                let data = slice.get_mapped_range();
                let samples: &[u64] = bytemuck::cast_slice(&data);
                for (run, &passed) in main_faces.iter().zip(samples) {
                    counts.faces_seen += run.1 as u64;
                    if passed == 0 {
                        counts.faces_hidden += run.1 as u64;
                    }
                }
            }
            read.unmap();
            self.counts = counts;
        }
        if let Some(t) = self.timer.as_mut() {
            t.submitted();
        }
        self.counts = counts;
    }

    /// Makes the buffers of every rig mesh the level keeps (`WorldView::prepare_rigs`) when the view
    /// has prepared a new level, and keeps them (pinned) until the next; the last level's that are
    /// not wanted again are let go at once. (Behind the loading screen: `level_ready` waits for it.)
    fn sync_resident(&mut self, view: &WorldView) {
        let r = view.resident();
        if r.gen == self.resident_gen {
            return;
        }
        let t = web_time::Instant::now();
        let mut bytes = 0;
        for b in self.rig_meshes.values_mut() {
            b.pinned = false;
        }
        for mesh in &r.meshes {
            let (device, layout) = (&self.device, &self.rig_face_layout);
            let buf = self.rig_meshes.entry(mesh.id).or_insert_with(|| {
                bytes += mesh.bytes() as u64;
                rig_buf(device, layout, mesh)
            });
            buf.pinned = true;
        }
        // (What the last frame or two drew stays a while, as before.)
        let now = self.frame_no;
        self.rig_meshes.retain(|_, b| b.pinned || now.saturating_sub(b.used) < 2);
        self.pending_upload += bytes;
        self.resident_gen = r.gen;
        self.resident_cost = (t.elapsed().as_secs_f64() * 1e3, bytes);
    }

    /// Makes sure every rig this frame draws has its mesh on the GPU, uploads the frame's
    /// draw records and part matrices, and lets go of meshes long unused. The bind group
    /// for the records, if any rig is drawn.
    fn upload_rigs(&mut self, frame: &Frame, counts: &mut FrameCounts) -> Option<wgpu::BindGroup> {
        let lists = [&frame.rigs, &frame.rigs_blend];
        let total: usize = lists.iter().map(|l| l.len()).sum();
        let now = self.frame_no;
        if self.frame_no.is_multiple_of(60) {
            self.rig_meshes.retain(|_, b| rig_kept(b.pinned, b.used, now));
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
                let (device, layout) = (&self.device, &self.rig_face_layout);
                let buf = self.rig_meshes.entry(mesh.id).or_insert_with(|| {
                    // Not made ready with the level: meshed (if it was not) and sent up now.
                    counts.rig_misses += 1;
                    if std::env::var_os("VH_RIG_MISSES").is_some() {
                        eprintln!("rig miss: mesh {} ({} voxels)", mesh.id, mesh.voxels);
                    }
                    counts.upload += mesh.bytes() as u64;
                    rig_buf(device, layout, mesh)
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

    /// Builds the level's probes on its first frame, refreshes them round what changed since, and
    /// sends them up when they change. Whether the frame is in a level the probes cover.
    /// The probes for the level now uploaded: from the cache when it was seen before, else built
    /// (on a worker thread, natively: none until it is done).
    fn level_probes(&mut self, view: &WorldView, (w, h): (f32, f32), atm: &super::view::Atmosphere) -> Option<ProbeVolume> {
        use std::hash::{Hash, Hasher};
        // (Worked out once per upload: hashing every cube took 10 ms a frame while the probes were
        // being built behind the loading screen.)
        let cubes = match self.probe_key {
            Some((level, key)) if level == self.static_level => key,
            _ => {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                bytemuck::cast_slice::<_, u8>(&view.static_cubes).hash(&mut hasher);
                let key = hasher.finish();
                self.probe_key = Some((self.static_level, key));
                key
            }
        };
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        cubes.hash(&mut hasher);
        format!("{atm:?} {w} {h}").hash(&mut hasher);
        let key = hasher.finish();
        if let Some(i) = self.probe_cache.iter().position(|c| c.0 == key) {
            let hit = self.probe_cache.remove(i);
            let p = hit.1.clone();
            self.probe_cache.push(hit);
            return Some(p);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some((k, level, rx)) = &self.probe_job {
                if *k == key && *level == self.static_level {
                    let p = rx.try_recv().ok()?;
                    self.probe_job = None;
                    self.remember_probes(key, p.clone());
                    return Some(p);
                }
            }
            let (tx, rx) = std::sync::mpsc::channel();
            let (cubes, atm) = (view.static_cubes.clone(), *atm);
            std::thread::spawn(move || {
                let _ = tx.send(ProbeVolume::build(&cubes, w, h, &atm));
            });
            self.probe_job = Some((key, self.static_level, rx));
            None
        }
        #[cfg(target_arch = "wasm32")]
        {
            let p = ProbeVolume::build(&view.static_cubes, w, h, atm);
            self.remember_probes(key, p.clone());
            Some(p)
        }
    }

    fn remember_probes(&mut self, key: u64, p: ProbeVolume) {
        const KEPT: usize = 8;
        self.probe_cache.push((key, p));
        if self.probe_cache.len() > KEPT {
            self.probe_cache.remove(0);
        }
    }

    /// The level's probes are here (or out being refreshed: their light is on the GPU).
    fn probes_held(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        if self.probe_refresh.as_ref().is_some_and(|r| r.0 == self.static_level) {
            return true;
        }
        self.probes.is_some()
    }

    /// The static cubes that reach into `lo`..`hi` (by their bounds as last sent), for a refresh.
    fn cubes_near(&self, view: &WorldView, lo: Vec3, hi: Vec3) -> Vec<Instance> {
        let (lo, hi) = (lo - Vec3::splat(probes::CELL), hi + Vec3::splat(probes::CELL));
        self.static_slot
            .iter()
            .enumerate()
            .filter(|&(_, &slot)| self.static_bounds.get(slot as usize).is_some_and(|&(a, b)| a.cmple(hi).all() && b.cmpge(lo).all()))
            .filter_map(|(i, _)| view.static_cubes.get(i).copied())
            .collect()
    }

    fn sync_probes(&mut self, view: &WorldView, frame: &Frame) -> bool {
        if frame.skip_static || self.static_count == 0 {
            return false;
        }
        let t = std::time::Instant::now();
        // What to send up: the whole volume, or the probes a refresh changed.
        let mut send: Option<Option<ProbeBox>> = None;
        #[cfg(not(target_arch = "wasm32"))]
        if let Some((level, rx)) = &self.probe_refresh {
            if *level != self.static_level {
                // A new level since: its probes are built afresh, and this one is let go.
                self.probe_refresh = None;
            } else if let Ok((p, changed)) = rx.try_recv() {
                self.probes = Some(p);
                self.probe_refresh = None;
                send = Some(changed);
            } else {
                return true;
            }
        }
        if self.probe_level != self.static_level || !self.probes_held() {
            let (w, h) = view.level_size();
            match self.level_probes(view, (w, h), &frame.atmosphere) {
                Some(p) => {
                    self.probes = Some(p);
                    self.probe_level = self.static_level;
                    self.probe_dirty.clear();
                    send = Some(None);
                }
                // Still being built: the level is not ready, and is drawn without them meanwhile.
                None => return false,
            }
        } else if !self.probe_dirty.is_empty() && self.probes.is_some() {
            let (mut lo, mut hi) = (Vec3::INFINITY, Vec3::NEG_INFINITY);
            for (a, b) in self.probe_dirty.drain(..) {
                lo = lo.min(a);
                hi = hi.max(b);
            }
            let near = self.cubes_near(view, lo, hi);
            let mut p = self.probes.take().unwrap();
            // Natively the work goes to a worker thread, and the light comes a frame or two later;
            // on the web, which has no threads, here and now.
            #[cfg(not(target_arch = "wasm32"))]
            {
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let changed = p.refresh(&near, lo, hi);
                    let _ = tx.send((p, changed));
                });
                self.probe_refresh = Some((self.static_level, rx));
                return true;
            }
            #[cfg(target_arch = "wasm32")]
            {
                let changed = p.refresh(&near, lo, hi);
                self.probes = Some(p);
                send = Some(changed);
            }
        }
        if let Some(changed) = send {
            let p = self.probes.as_ref().unwrap();
            // Only the probes that changed go up (all of them for a new level, or new textures).
            let mut b = changed.unwrap_or(ProbeBox::all(p.dims));
            if self.probe_tex.first().is_none_or(|t| [t.width(), t.height(), t.depth_or_array_layers()] != p.dims) {
                self.probe_tex = probe_textures(&self.device, p.dims);
                self.scene_bind = self.scene_inputs.bind(&self.device, &self.probe_tex);
                b = ProbeBox::all(p.dims);
            }
            let [w, h, d] = b.size();
            let (dx, dy) = (p.dims[0] as usize, p.dims[1] as usize);
            for (s, tex) in self.probe_tex.iter().enumerate() {
                let mut data: Vec<u16> = Vec::with_capacity((w * h * d * 4) as usize);
                for z in b.lo[2]..=b.hi[2] {
                    for y in b.lo[1]..=b.hi[1] {
                        let row = (z as usize * dy + y as usize) * dx;
                        for v in &p.sides[s][row + b.lo[0] as usize..=row + b.hi[0] as usize] {
                            data.extend(v.map(super::brdf::f16));
                        }
                    }
                }
                self.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: tex,
                        mip_level: 0,
                        origin: wgpu::Origin3d { x: b.lo[0], y: b.lo[1], z: b.lo[2] },
                        aspect: wgpu::TextureAspect::All,
                    },
                    bytemuck::cast_slice(&data),
                    wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 8), rows_per_image: Some(h) },
                    wgpu::Extent3d { width: w, height: h, depth_or_array_layers: d },
                );
                self.pending_upload += data.len() as u64 * 2;
            }
            if std::env::var_os("VH_PROBE_STATS").is_some() {
                eprintln!("probes: {:?} of {:?} sent in {:.1} ms at {}", b.size(), p.dims, t.elapsed().as_secs_f64() * 1e3, self.frame_no);
            }
        }
        true
    }

    /// The environment's next `n` passes (of `ENV_PASSES`, in order: the first mip's faces, then
    /// each mip prefiltered from it).
    fn env_passes(&mut self, enc: &mut wgpu::CommandEncoder, n: u32, counts: &mut FrameCounts) {
        let end = (self.env_next + n).min(ENV_PASSES);
        for k in self.env_next..end {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("environment"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.env_faces[k as usize],
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: if k == self.env_next { self.timer.as_mut().and_then(|t| t.pass("environment")) } else { None },
                occlusion_query_set: None,
            });
            pass.set_pipeline(if k < 6 { &self.env_draw_pipe } else { &self.env_prefilter_pipe });
            pass.set_bind_group(0, &self.env_globals, &[]);
            if k >= 6 {
                pass.set_bind_group(1, &self.env_src, &[]);
            }
            pass.draw(0..3, k..k + 1);
            counts.draw(1, 3);
        }
        self.env_next = end;
    }

    /// Draws runs of static faces (start, count) with `pipe`, which reads the static store.
    /// The static chunks view `view` sees, as the cull pass listed them: one indirect multi-draw.
    fn draw_faces_gpu(&self, pass: &mut wgpu::RenderPass<'_>, pipe: &wgpu::RenderPipeline, view: u32, counts: &mut FrameCounts) {
        let (Some(bind), Some(cull)) = (&self.static_bind, &self.cull) else { return };
        pass.set_pipeline(pipe);
        pass.set_bind_group(1, bind, &[]);
        pass.set_vertex_buffer(0, self.face_buf.buf.slice(..));
        pass.set_index_buffer(self.quad.slice(..), wgpu::IndexFormat::Uint16);
        cull.draw(pass, view);
        counts.draws += 1;
    }

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
        pass.set_vertex_buffer(0, self.rig_ids.slice(..));
        for (k, mesh) in meshes.iter().enumerate() {
            let id = first + k as u32;
            let Some(buf) = self.rig_meshes.get(&mesh.id).filter(|b| id < MAX_RIG_DRAWS && b.count > 0) else { continue };
            pass.set_bind_group(2, &buf.faces, &[]);
            pass.draw(0..buf.count * 6, id..id + 1);
            counts.draws += 1;
            counts.instances += 1;
            counts.triangles += buf.count as u64 * 2;
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

/// The probes' six textures (one per side), `dims` probes in x, layers and z.
fn probe_textures(device: &wgpu::Device, dims: [u32; 3]) -> Vec<wgpu::Texture> {
    (0..6)
        .map(|_| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("probes"),
                size: wgpu::Extent3d { width: dims[0], height: dims[1], depth_or_array_layers: dims[2] },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D3,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            })
        })
        .collect()
}

/// The two cascades' maps (each its own size, drawn into and sampled), and the coarse cascade's
/// static cache (the far map's size, copied from).
fn shadow_targets(device: &wgpu::Device, q: &Shadows) -> ([wgpu::Texture; 2], [wgpu::TextureView; 2], wgpu::Texture, wgpu::TextureView) {
    let map = |label, size: u32, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH,
            usage,
            view_formats: &[],
        })
    };
    let drawn = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
    let maps = [map("shadow near", q.near_size, drawn), map("shadow far", q.far_size, drawn)];
    let views = [maps[0].create_view(&Default::default()), maps[1].create_view(&Default::default())];
    let cache = map("static shadow cache", q.far_size, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC);
    let cache_view = cache.create_view(&Default::default());
    (maps, views, cache, cache_view)
}

/// GPU visibility for the static chunks (RP20, `cull.wgsl`): every chunk's bounds kept on the GPU,
/// tested there against the camera and both shadow cascades each frame, and the chunks seen drawn
/// by indirect multi-draws, so the CPU neither tests them nor records a draw per chunk. The
/// faces added since the level loaded (patches, rubble) ride along as one more chunk, its box
/// sent again each frame. Absent where the adapter cannot draw indirectly with a first instance.
struct GpuCull {
    pipe: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
    views: wgpu::Buffer,
    counts: wgpu::Buffer,
    chunk_buf: Option<wgpu::Buffer>,
    args: Option<wgpu::Buffer>,
    bind: Option<wgpu::BindGroup>,
    /// Chunks listed (the level's, and the added faces' one at the end).
    chunks: u32,
    /// Drawn with a count (the seen packed to the front), else every slot drawn.
    counted: bool,
}

/// A chunk as `cull.wgsl` reads it.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuChunk {
    lo: [f32; 4],
    hi: [f32; 4],
    faces: u32,
    count: u32,
    /// The way all its faces look (0 to 5), or 6 for any.
    dir: u32,
    pad: u32,
}

/// Outward normals of a cube's faces, in `FACE_NORMALS` order.
const FACE_AXES: [Vec3; 6] = [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z];

/// Cull views: the camera, the fine shadow cascade, the coarse one's static cache.
const CULL_VIEWS: u32 = 3;

impl GpuCull {
    fn new(device: &wgpu::Device) -> Option<Self> {
        let f = device.features();
        if !f.contains(wgpu::Features::MULTI_DRAW_INDIRECT | wgpu::Features::INDIRECT_FIRST_INSTANCE) {
            return None;
        }
        let storage = |binding, read_only| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cull"),
            entries: &[
                storage(0, true),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                storage(2, false),
                storage(3, false),
            ],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cull"),
            source: wgpu::ShaderSource::Wgsl(include_str!("cull.wgsl").into()),
        });
        let pipe = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("cull"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("cull"),
                bind_group_layouts: &[&layout],
                push_constant_ranges: &[],
            })),
            module: &module,
            entry_point: Some("cs_cull"),
            compilation_options: Default::default(),
            cache: None,
        });
        let views = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cull views"),
            size: 21 * 16 + 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let counts = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cull counts"),
            size: 16,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let counted = f.contains(wgpu::Features::MULTI_DRAW_INDIRECT_COUNT);
        Some(Self { pipe, layout, views, counts, chunk_buf: None, args: None, bind: None, chunks: 0, counted })
    }

    /// A level's face groups (chunk by chunk, by facing), and one slot more for the added faces.
    fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, groups: &[FaceGroup]) {
        let mut list: Vec<GpuChunk> = groups
            .iter()
            .map(|g| GpuChunk {
                lo: g.lo.extend(0.0).to_array(),
                hi: g.hi.extend(0.0).to_array(),
                faces: g.faces,
                count: g.count,
                dir: g.dir,
                pad: 0,
            })
            .collect();
        list.push(GpuChunk::zeroed());
        let n = list.len() as u32;
        if self.chunks != n || self.chunk_buf.is_none() {
            let chunk_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("cull chunks"),
                size: std::mem::size_of_val(list.as_slice()) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let args = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("cull draws"),
                size: (n * CULL_VIEWS) as u64 * 20,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT,
                mapped_at_creation: false,
            });
            self.bind = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("cull"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: chunk_buf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: self.views.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: args.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 3, resource: self.counts.as_entire_binding() },
                ],
            }));
            self.chunk_buf = Some(chunk_buf);
            self.args = Some(args);
        }
        if let Some(b) = &self.chunk_buf {
            queue.write_buffer(b, 0, bytemuck::cast_slice(&list));
        }
        self.chunks = n;
    }

    /// Lists what each view sees: the added faces' box and run sent, the counts cleared, the test run.
    fn run(
        &mut self,
        queue: &wgpu::Queue,
        enc: &mut wgpu::CommandEncoder,
        views: &[Mat4; 3],
        eyes: &[Vec4; 3],
        extra: Option<((Vec3, Vec3), u32, u32)>,
        timestamps: Option<wgpu::ComputePassTimestampWrites<'_>>,
    ) {
        let (Some(chunk_buf), Some(bind)) = (&self.chunk_buf, &self.bind) else { return };
        let last = match extra {
            Some(((lo, hi), faces, count)) => {
                GpuChunk { lo: lo.extend(0.0).to_array(), hi: hi.extend(0.0).to_array(), faces, count, dir: 6, pad: 0 }
            }
            None => GpuChunk::zeroed(),
        };
        queue.write_buffer(chunk_buf, (self.chunks as u64 - 1) * std::mem::size_of::<GpuChunk>() as u64, bytemuck::bytes_of(&last));
        let mut data: Vec<f32> = Vec::with_capacity(21 * 4 + 4);
        for m in views {
            for p in Frustum::new(*m).0 {
                data.extend_from_slice(&p.to_array());
            }
        }
        for e in eyes {
            data.extend_from_slice(&e.to_array());
        }
        let n: [u32; 4] = [self.chunks, CULL_VIEWS, u32::from(self.counted), 0];
        let mut bytes: Vec<u8> = bytemuck::cast_slice(&data).to_vec();
        bytes.extend_from_slice(bytemuck::cast_slice(&n));
        queue.write_buffer(&self.views, 0, &bytes);
        enc.clear_buffer(&self.counts, 0, None);
        let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("cull"), timestamp_writes: timestamps });
        pass.set_pipeline(&self.pipe);
        pass.set_bind_group(0, bind, &[]);
        pass.dispatch_workgroups(self.chunks.div_ceil(64), CULL_VIEWS, 1);
    }

    fn draw(&self, pass: &mut wgpu::RenderPass<'_>, view: u32) {
        let Some(args) = &self.args else { return };
        let offset = (view * self.chunks) as u64 * 20;
        if self.counted {
            pass.multi_draw_indexed_indirect_count(args, offset, &self.counts, view as u64 * 4, self.chunks);
        } else {
            pass.multi_draw_indexed_indirect(args, offset, self.chunks);
        }
    }
}

/// What the scene's bind group (group 0 of every scene pipeline) is made of.
struct SceneInputs {
    layout: wgpu::BindGroupLayout,
    globals: wgpu::Buffer,
    shadow_views: [wgpu::TextureView; 2],
    shadow_sampler: wgpu::Sampler,
    env_view: wgpu::TextureView,
    env_sampler: wgpu::Sampler,
    lut_view: wgpu::TextureView,
    light_buf: wgpu::Buffer,
    cell_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
}

impl SceneInputs {
    /// The scene's bind group, with these probe textures (one per side).
    fn bind(&self, device: &wgpu::Device, probes: &[wgpu::Texture]) -> wgpu::BindGroup {
        let views: Vec<wgpu::TextureView> = probes.iter().map(|t| t.create_view(&Default::default())).collect();
        let mut entries = vec![
            wgpu::BindGroupEntry { binding: 0, resource: self.globals.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&self.shadow_views[0]) },
            wgpu::BindGroupEntry { binding: 16, resource: wgpu::BindingResource::TextureView(&self.shadow_views[1]) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.shadow_sampler) },
            wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&self.env_view) },
            wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(&self.env_sampler) },
            wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(&self.lut_view) },
            wgpu::BindGroupEntry { binding: 7, resource: self.light_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 8, resource: self.cell_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 9, resource: self.index_buf.as_entire_binding() },
        ];
        for (k, v) in views.iter().enumerate() {
            entries.push(wgpu::BindGroupEntry { binding: 10 + k as u32, resource: wgpu::BindingResource::TextureView(v) });
        }
        device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("scene"), layout: &self.layout, entries: &entries })
    }
}

fn rig_buf(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, mesh: &RigMesh) -> RigBuf {
    // (A binding cannot be empty: a rig with no faces keeps one it never draws.)
    let none = [<super::voxels::RigFace as bytemuck::Zeroable>::zeroed()];
    let contents: &[u8] = bytemuck::cast_slice(if mesh.faces.is_empty() { &none[..] } else { &mesh.faces[..] });
    let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("rig faces"),
        contents,
        usage: wgpu::BufferUsages::STORAGE,
    });
    let faces = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("rig faces"),
        layout,
        entries: &[wgpu::BindGroupEntry { binding: 0, resource: buf.as_entire_binding() }],
    });
    RigBuf { faces, count: mesh.faces.len() as u32, used: 0, pinned: false }
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
    fn a_rig_the_level_keeps_is_never_let_go_while_it_lives() {
        // Kept, however long unseen (a kind that comes out of its door minutes in).
        assert!(rig_kept(true, 0, 1_000_000));
        // Not kept, it goes after RIG_IDLE_FRAMES unseen; drawn lately, it stays.
        assert!(!rig_kept(false, 0, RIG_IDLE_FRAMES));
        assert!(rig_kept(false, 100, 100 + RIG_IDLE_FRAMES - 1));
    }

    #[test]
    fn walking_about_the_level_never_moves_the_far_cascade_so_its_cache_holds() {
        let mut g = crate::game::Game::new();
        g.load_level(0);
        let atm = crate::render::view::Atmosphere::forest();
        let mut seen = Vec::new();
        for (x, y) in [(10.0, 10.0), (11.0, 10.0), (30.0, 22.0), (60.0, 55.0), (2.0, 60.0)] {
            g.hero.place(crate::logic::types::P::new(x as i32, y as i32));
            for _ in 0..30 {
                g.update(1.0 / 120.0);
            }
            let (center, radius) = g.view.shadow_box();
            seen.push(far_light(center, radius, atm.sun_dir).1.to_cols_array());
        }
        assert!(seen.windows(2).all(|w| w[0] == w[1]), "the cascade's box follows the level, not him");
    }

    #[test]
    fn a_small_change_redraws_only_its_part_of_the_far_cache() {
        let (center, radius) = (Vec3::new(30.0, 0.5, 30.0), 45.0);
        let (view, vp) = far_light(center, radius, Vec3::new(-6.0, 14.0, 8.0));
        let size = 4096;
        // A coin, a wall: a few dozen texels, a margin round them.
        let coin = (Vec3::new(20.0, 0.1, 25.0), Vec3::new(20.2, 0.3, 25.2));
        let r = far_part(vp, coin, size).expect("a part");
        assert!(r[2] < 40 && r[3] < 40 && r[2] > 3 && r[3] > 3, "{r:?}");
        let wall = (Vec3::new(19.5, 0.0, 24.5), Vec3::new(20.5, 2.5, 25.5));
        assert!(far_part(vp, wall, size).is_some_and(|w| w[2] * w[3] < size * size / 400));
        // The whole level, or nothing: drawn whole.
        assert_eq!(far_part(vp, (Vec3::splat(-10.0), Vec3::splat(80.0)), size), None);
        assert_eq!(far_part(vp, (Vec3::splat(1.0), Vec3::splat(-1.0)), size), None);
        // The part's view sees what stands over its texels, along the light, and not what is beside.
        let part = Frustum::new(far_part_view(&view, radius, r, size));
        assert!(part.sees(coin.0, coin.1));
        let l = Vec3::new(-6.0, 14.0, 8.0).normalize();
        let above = coin.0 + l * 6.0;
        assert!(part.sees(above, above + Vec3::splat(0.1)), "a caster up the light's way");
        assert!(!part.sees(Vec3::new(40.0, 0.0, 40.0), Vec3::new(41.0, 1.0, 41.0)), "one elsewhere");
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
