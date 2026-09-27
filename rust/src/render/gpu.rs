//! wgpu backend: WebGPU in the browser, Direct3D 12 / Vulkan / Metal on desktop.

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;

use super::overlay::{build_atlas, OVertex, ATLAS_H, ATLAS_W};
use super::view::{Frame, Instance, Light, MeshKind, WorldView, FAR, MESH_KINDS};

const MAX_LIGHTS: usize = 32;
const SAMPLES: u32 = 4;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

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
    cam_pos: [f32; 4],
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
    lights: [GpuLight; MAX_LIGHTS],
}

struct MeshBuf {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
}

/// A growable GPU buffer of instances or vertices.
struct Dyn {
    buf: wgpu::Buffer,
    cap: u64,
    usage: wgpu::BufferUsages,
    label: &'static str,
}

impl Dyn {
    fn new(device: &wgpu::Device, label: &'static str, usage: wgpu::BufferUsages, cap: u64) -> Self {
        let buf = device.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size: cap, usage: usage | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        Self { buf, cap, usage, label }
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

pub struct Gpu {
    pub surface: Option<wgpu::Surface<'static>>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub format: wgpu::TextureFormat,
    msaa: wgpu::TextureView,
    depth: wgpu::TextureView,
    scene_opaque: wgpu::RenderPipeline,
    scene_blend: wgpu::RenderPipeline,
    scene_add: wgpu::RenderPipeline,
    sky_pipe: wgpu::RenderPipeline,
    overlay_pipe: wgpu::RenderPipeline,
    globals: wgpu::Buffer,
    scene_bind: wgpu::BindGroup,
    screen: wgpu::Buffer,
    overlay_bind: wgpu::BindGroup,
    meshes: Vec<MeshBuf>,
    static_buf: Dyn,
    static_count: u32,
    dyn_buf: Dyn,
    overlay_buf: Dyn,
    pub backend: String,
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
        let limits = if cfg!(target_arch = "wasm32") {
            wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())
        } else {
            wgpu::Limits::default().using_resolution(adapter.limits())
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("void-harvest"),
                required_features: wgpu::Features::empty(),
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

        // ---- scene
        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let scene_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let scene_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene"),
            layout: &scene_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals.as_entire_binding() }],
        });
        let scene_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene"),
            bind_group_layouts: &[&scene_layout],
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
        let scene_pipe = |label: &str, blend: Option<wgpu::BlendState>, depth_write: bool| {
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
                    entry_point: Some("fs_scene"),
                    targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask: wgpu::ColorWrites::ALL })],
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
        let scene_opaque = scene_pipe("opaque", None, true);
        let scene_blend = scene_pipe("transparent", Some(alpha), false);
        let scene_add = scene_pipe("additive", Some(additive), false);
        // The sky: one full-screen triangle behind everything, no depth.
        let sky_pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sky"),
            layout: Some(&scene_pl),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_sky"), buffers: &[], compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_sky"),
                targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
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
                format: wgpu::TextureFormat::R8Unorm,
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
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
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

        let meshes = build_meshes(&device);
        let inst = wgpu::BufferUsages::VERTEX;
        let (msaa, depth) = targets(&device, format, config.width, config.height);
        Ok(Self {
            surface,
            static_buf: Dyn::new(&device, "static", inst, 1 << 20),
            dyn_buf: Dyn::new(&device, "dynamic", inst, 1 << 20),
            overlay_buf: Dyn::new(&device, "overlay", inst, 1 << 18),
            device,
            queue,
            config,
            format,
            msaa,
            depth,
            scene_opaque,
            scene_blend,
            scene_add,
            sky_pipe,
            overlay_pipe,
            globals,
            scene_bind,
            screen,
            overlay_bind,
            meshes,
            static_count: 0,
            backend,
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
        let (msaa, depth) = targets(&self.device, self.format, w, h);
        self.msaa = msaa;
        self.depth = depth;
    }

    pub fn set_vsync(&mut self, on: bool) {
        self.config.present_mode = if on { wgpu::PresentMode::AutoVsync } else { wgpu::PresentMode::AutoNoVsync };
        if let Some(s) = &self.surface {
            s.configure(&self.device, &self.config);
        }
    }

    /// Uploads the level's static geometry when it changed.
    pub fn sync_static(&mut self, view: &mut WorldView) {
        if view.static_dirty {
            self.static_buf.write(&self.device, &self.queue, bytemuck::cast_slice(&view.static_cubes));
            self.static_count = view.static_cubes.len() as u32;
            view.static_dirty = false;
            view.static_patches.clear();
            return;
        }
        // Small rewrites (fading trees) go up range by range.
        let stride = std::mem::size_of::<Instance>();
        for (start, len) in view.static_patches.drain(..) {
            let bytes: &[u8] = bytemuck::cast_slice(&view.static_cubes[start..start + len]);
            self.queue.write_buffer(&self.static_buf.buf, (start * stride) as u64, bytes);
        }
    }

    /// Draws one frame into `target`: the scene, then the overlay on top.
    pub fn draw(&mut self, target: &wgpu::TextureView, view: &WorldView, frame: &Frame, overlay: &[OVertex], logical: (f32, f32)) {
        // ---- uniforms
        let mut lights = [GpuLight::zeroed(); MAX_LIGHTS];
        let mut chosen: Vec<&Light> = frame.lights.iter().collect();
        if chosen.len() > MAX_LIGHTS {
            let hero = frame.lights[0].pos;
            chosen.sort_by(|a, b| a.pos.distance_squared(hero).partial_cmp(&b.pos.distance_squared(hero)).unwrap());
            chosen.truncate(MAX_LIGHTS);
        }
        for (i, l) in chosen.iter().enumerate() {
            lights[i] = GpuLight { pos: [l.pos.x, l.pos.y, l.pos.z, l.distance], color: [l.color.x, l.color.y, l.color.z, 0.0] };
        }
        let atm = frame.atmosphere;
        let fog = super::overlay::rgba(atm.background, atm.fog_near);
        let v4 = |v: Vec3, w: f32| [v.x, v.y, v.z, w];
        let cam = view.camera_position();
        let (sky_fwd, sky_right, sky_up) = view.sky_basis();
        let g = Globals {
            view_proj: view.view_proj().to_cols_array(),
            cam_pos: v4(cam, 1.0),
            fog,
            params: [atm.fog_far.min(FAR), atm.exposure, frame.coin_pulse, chosen.len() as f32],
            hemi_sky: v4(atm.hemi_sky, 0.0),
            hemi_ground: v4(atm.hemi_ground, 0.0),
            ambient: v4(atm.ambient, 0.0),
            moon_dir: v4(atm.sun_dir, 0.0),
            moon_color: v4(atm.sun, 0.0),
            sky_fwd: v4(sky_fwd, atm.night_sky),
            sky_right: v4(sky_right, frame.time),
            sky_up: v4(sky_up, 0.0),
            sky_moon: v4(atm.moon_view, 0.0),
            lights,
        };
        self.queue.write_buffer(&self.globals, 0, bytemuck::bytes_of(&g));
        self.queue.write_buffer(&self.screen, 0, bytemuck::cast_slice(&[logical.0, logical.1, 0.0, 0.0]));

        // ---- instances: opaque by mesh, transparent by mesh, additive
        let order_t = [MeshKind::Disc, MeshKind::Ring, MeshKind::Cube, MeshKind::Sphere, MeshKind::Cylinder];
        let mut all: Vec<Instance> = Vec::new();
        let mut ranges: Vec<(u8, usize, u32, u32)> = Vec::new(); // (pipeline, mesh, start, count)
        for k in 0..MESH_KINDS {
            let list = &frame.opaque[k];
            if !list.is_empty() {
                ranges.push((0, k, all.len() as u32, list.len() as u32));
                all.extend_from_slice(list);
            }
        }
        for k in order_t {
            let list = &frame.transparent[k as usize];
            if !list.is_empty() {
                ranges.push((1, k as usize, all.len() as u32, list.len() as u32));
                all.extend_from_slice(list);
            }
        }
        if !frame.additive.is_empty() {
            ranges.push((2, MeshKind::Cube as usize, all.len() as u32, frame.additive.len() as u32));
            all.extend_from_slice(&frame.additive);
        }
        self.dyn_buf.write(&self.device, &self.queue, bytemuck::cast_slice(&all));
        self.overlay_buf.write(&self.device, &self.queue, bytemuck::cast_slice(overlay));

        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        {
            let bg = super::overlay::rgba(atm.background, 1.0);
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa,
                    resolve_target: Some(target),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: bg[0] as f64, g: bg[1] as f64, b: bg[2] as f64, a: 1.0 }),
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_bind_group(0, &self.scene_bind, &[]);
            if atm.night_sky > 0.0 {
                pass.set_pipeline(&self.sky_pipe);
                pass.draw(0..3, 0..1);
            }

            // Static level geometry.
            if self.static_count > 0 && !frame.skip_static {
                let m = &self.meshes[MeshKind::Cube as usize];
                pass.set_pipeline(&self.scene_opaque);
                pass.set_vertex_buffer(0, m.vertices.slice(..));
                pass.set_index_buffer(m.indices.slice(..), wgpu::IndexFormat::Uint16);
                pass.set_vertex_buffer(1, self.static_buf.buf.slice(..));
                pass.draw_indexed(0..m.count, 0, 0..self.static_count);
            }
            if !all.is_empty() {
                pass.set_vertex_buffer(1, self.dyn_buf.buf.slice(..));
                for (pipe, mesh, start, count) in ranges {
                    pass.set_pipeline(match pipe {
                        0 => &self.scene_opaque,
                        1 => &self.scene_blend,
                        _ => &self.scene_add,
                    });
                    let m = &self.meshes[mesh];
                    pass.set_vertex_buffer(0, m.vertices.slice(..));
                    pass.set_index_buffer(m.indices.slice(..), wgpu::IndexFormat::Uint16);
                    pass.draw_indexed(0..m.count, 0, start..start + count);
                }
            }
            if !overlay.is_empty() {
                pass.set_pipeline(&self.overlay_pipe);
                pass.set_bind_group(0, &self.overlay_bind, &[]);
                pass.set_vertex_buffer(0, self.overlay_buf.buf.slice(..));
                pass.draw(0..overlay.len() as u32, 0..1);
            }
        }
        self.queue.submit([enc.finish()]);
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

fn targets(device: &wgpu::Device, format: wgpu::TextureFormat, w: u32, h: u32) -> (wgpu::TextureView, wgpu::TextureView) {
    let make = |format, label| {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: SAMPLES,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&Default::default())
    };
    (make(format, "msaa"), make(DEPTH, "depth"))
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
            _ => cylinder(0.34, 0.4, 0.12, 8),
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
