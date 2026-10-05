//! Measuring the renderer (RP07 in the Technical Art and Rendering Plan): GPU time per pass from
//! timestamp queries, read back a few frames late so nothing waits on them, and the counts
//! that explain a frame's cost (draws, instances, triangles, bytes sent, lights).

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

/// Room for this many timed passes a frame (two queries each).
const MAX_PASSES: u32 = 32;
/// Frames whose timings may be in flight at once.
const SLOTS: usize = 3;

const FREE: u8 = 0;
const MAPPING: u8 = 1;
const MAPPED: u8 = 2;

struct Slot {
    buf: wgpu::Buffer,
    labels: Vec<&'static str>,
    state: Arc<AtomicU8>,
}

/// GPU timestamps around each render pass. Absent when the adapter cannot time passes.
pub struct GpuTimer {
    set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    slots: Vec<Slot>,
    cur: usize,
    /// Timing this frame: false while every slot is still being read.
    active: bool,
    labels: Vec<&'static str>,
    /// Milliseconds per timestamp tick.
    tick_ms: f64,
    /// The latest frame read back: each label's summed time, and the frame's whole span, in ms.
    pub passes: Vec<(&'static str, f64)>,
    pub total: f64,
    /// How many frames have been read back so far.
    pub frames: u64,
}

impl GpuTimer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        let count = MAX_PASSES * 2;
        let bytes = count as u64 * 8;
        let set = device.create_query_set(&wgpu::QuerySetDescriptor { label: Some("pass times"), ty: wgpu::QueryType::Timestamp, count });
        let resolve = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pass times resolve"),
            size: bytes,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let slots = (0..SLOTS)
            .map(|_| Slot {
                buf: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("pass times readback"),
                    size: bytes,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }),
                labels: Vec::new(),
                state: Arc::new(AtomicU8::new(FREE)),
            })
            .collect();
        Some(Self {
            set,
            resolve,
            slots,
            cur: 0,
            active: false,
            labels: Vec::new(),
            tick_ms: queue.get_timestamp_period() as f64 / 1e6,
            passes: Vec::new(),
            total: 0.0,
            frames: 0,
        })
    }

    /// Starts a frame: it is timed if a readback slot is free.
    pub fn begin(&mut self) {
        self.labels.clear();
        self.active = self.slots[self.cur].state.load(Ordering::Acquire) == FREE;
    }

    /// The timestamp writes for one pass, or none when this frame is not timed.
    pub fn pass(&mut self, label: &'static str) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        if !self.active || self.labels.len() as u32 >= MAX_PASSES {
            return None;
        }
        let i = self.labels.len() as u32 * 2;
        self.labels.push(label);
        Some(wgpu::RenderPassTimestampWrites {
            query_set: &self.set,
            beginning_of_pass_write_index: Some(i),
            end_of_pass_write_index: Some(i + 1),
        })
    }

    /// The same for a compute pass.
    pub fn compute_pass(&mut self, label: &'static str) -> Option<wgpu::ComputePassTimestampWrites<'_>> {
        let w = self.pass(label)?;
        Some(wgpu::ComputePassTimestampWrites {
            query_set: w.query_set,
            beginning_of_pass_write_index: w.beginning_of_pass_write_index,
            end_of_pass_write_index: w.end_of_pass_write_index,
        })
    }

    /// Resolves this frame's queries into its readback slot (before the encoder is finished).
    pub fn resolve(&mut self, enc: &mut wgpu::CommandEncoder) {
        if !self.active || self.labels.is_empty() {
            return;
        }
        let n = self.labels.len() as u32 * 2;
        enc.resolve_query_set(&self.set, 0..n, &self.resolve, 0);
        enc.copy_buffer_to_buffer(&self.resolve, 0, &self.slots[self.cur].buf, 0, n as u64 * 8);
    }

    /// After the frame is submitted: asks for its slot to be mapped, and moves to the next slot.
    pub fn submitted(&mut self) {
        if !self.active || self.labels.is_empty() {
            return;
        }
        let slot = &mut self.slots[self.cur];
        slot.labels = std::mem::take(&mut self.labels);
        slot.state.store(MAPPING, Ordering::Release);
        let state = slot.state.clone();
        let n = slot.labels.len() as u64 * 16;
        slot.buf.slice(..n).map_async(wgpu::MapMode::Read, move |r| {
            state.store(if r.is_ok() { MAPPED } else { FREE }, Ordering::Release);
        });
        self.cur = (self.cur + 1) % SLOTS;
    }

    /// Reads back every frame whose timings have arrived (the newest wins). Never waits.
    pub fn collect(&mut self, device: &wgpu::Device) {
        let _ = device.poll(wgpu::PollType::Poll);
        for slot in &mut self.slots {
            if slot.state.load(Ordering::Acquire) != MAPPED {
                continue;
            }
            let n = slot.labels.len() as u64 * 16;
            {
                let data = slot.buf.slice(..n).get_mapped_range();
                let ticks: &[u64] = bytemuck::cast_slice(&data);
                let mut passes: Vec<(&'static str, f64)> = Vec::new();
                let (mut first, mut last) = (u64::MAX, 0u64);
                for (i, label) in slot.labels.iter().enumerate() {
                    let (a, b) = (ticks[i * 2], ticks[i * 2 + 1]);
                    first = first.min(a);
                    last = last.max(b);
                    let ms = b.saturating_sub(a) as f64 * self.tick_ms;
                    match passes.iter_mut().find(|(l, _)| l == label) {
                        Some(p) => p.1 += ms,
                        None => passes.push((label, ms)),
                    }
                }
                self.passes = passes;
                self.total = last.saturating_sub(first) as f64 * self.tick_ms;
                self.frames += 1;
            }
            slot.buf.unmap();
            slot.state.store(FREE, Ordering::Release);
        }
    }
}

/// Effects that can be switched off one at a time, to capture what each adds and what it costs.
pub mod fx {
    pub const SHADOWS: u32 = 1;
    pub const LIGHTS: u32 = 2;
    pub const GRASS: u32 = 4;
    pub const MATERIALS: u32 = 8;
    pub const FOG: u32 = 16;
    /// The bits the scene shader reads.
    pub const SCENE: u32 = 31;
    pub const AO: u32 = 32;
    pub const BLOOM: u32 = 64;
    pub const DOF: u32 = 128;
    pub const GRADE: u32 = 256;
    /// The opaque depth prepass.
    pub const PREPASS: u32 = 512;
    /// The ground mist (the see-through instance sets).
    pub const MIST: u32 = 1024;
    /// Skipping the static faces turned away from the camera.
    pub const BACKFACE: u32 = 2048;
    /// Not an effect off but a view: each pixel tinted by how many lights its cluster holds.
    pub const LIGHT_HEAT: u32 = 4096;
    /// The ground haze, and the lamps' glow in the air (RP19).
    pub const HAZE: u32 = 8192;
    pub const VOLUMETRIC: u32 = 16384;
    /// The juice (WARBAND.md §4), which the game and the view read rather than the GPU: the
    /// slowing of time; the freeze table and its budget (off, the brawl holds as it always did);
    /// the camera's roll, kick, FOV punch and trauma by source (the plain shake stays); the
    /// chips and the shatter; the springs' overshoot. `juice` is all of them, as the harnesses
    /// run (their numbers must not move with the feel).
    pub const DILATION: u32 = 1 << 15;
    pub const HITSTOP: u32 = 1 << 16;
    pub const SHAKE: u32 = 1 << 17;
    pub const DEBRIS: u32 = 1 << 18;
    pub const SPRINGS: u32 = 1 << 19;
    pub const JUICE: u32 = DILATION | HITSTOP | SHAKE | DEBRIS | SPRINGS;

    pub const NAMES: [(&str, u32); 20] = [
        ("shadows", SHADOWS),
        ("lights", LIGHTS),
        ("grass", GRASS),
        ("materials", MATERIALS),
        ("fog", FOG),
        ("ao", AO),
        ("bloom", BLOOM),
        ("dof", DOF),
        ("grade", GRADE),
        ("prepass", PREPASS),
        ("mist", MIST),
        ("backface", BACKFACE),
        ("haze", HAZE),
        ("volumetric", VOLUMETRIC),
        ("dilation", DILATION),
        ("hitstop", HITSTOP),
        ("shake", SHAKE),
        ("debris", DEBRIS),
        ("springs", SPRINGS),
        ("juice", JUICE),
    ];

    /// "shadows,ao" to its bits; unknown names are ignored.
    pub fn parse(list: &str) -> u32 {
        list.split(',').filter_map(|n| NAMES.iter().find(|(k, _)| *k == n.trim()).map(|(_, b)| *b)).fold(0, |a, b| a | b)
    }
}

/// What one frame asked of the GPU.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameCounts {
    pub draws: u32,
    pub instances: u64,
    pub triangles: u64,
    /// Bytes written to GPU buffers this frame (instances, uniforms, static patches).
    pub upload: u64,
    /// Lights the view asked for, and how many the shader's shortlist dropped.
    pub lights: u32,
    /// Light-in-cluster pairs over a cluster's cap, the pairs kept, and the fullest cluster (RP17).
    pub lights_dropped: u32,
    pub light_pairs: u32,
    pub busiest_cluster: u32,
    /// Static scenery: chunks and instances drawn in the main pass, of how many.
    pub chunks_drawn: u32,
    pub chunks: u32,
    pub static_drawn: u64,
    pub static_total: u64,
    /// Static instances drawn into the shadow cascades (near and far together), and into the near
    /// one alone.
    pub shadow_static: u64,
    pub shadow_near: u64,
    /// Rigs drawn from their resident voxels, and the voxels they cover.
    pub rigs: u32,
    pub rig_voxels: u64,
    /// Rigs drawn whole, at half and at a quarter of their resolution.
    pub rig_levels: [u32; 3],
    /// Rig meshes drawn this frame that the level had not made ready at its load (each one meshed
    /// and sent up in this frame: a stall mid-fight; `WorldView::prepare_rigs`).
    pub rig_misses: u32,
    /// With occlusion measured (`Gpu::measure_occlusion`, RP20): static faces in the camera's
    /// view, and those in chunks wholly hidden behind nearer scenery.
    pub faces_seen: u64,
    pub faces_hidden: u64,
    /// Static faces the camera would draw culled by whole chunks, by chunk and facing group, and by
    /// 64-face clusters with their facing (RP21's comparison; worked out on the CPU each frame).
    pub faces_by_chunk: u64,
    pub faces_by_group: u64,
    pub faces_by_cluster: u64,
}

impl FrameCounts {
    pub fn draw(&mut self, instances: u64, index_count: u32) {
        self.draws += 1;
        self.instances += instances;
        self.triangles += instances * (index_count as u64 / 3);
    }
}

/// Where the last simulation step's time went (RP06's budget split), in ms. Sections are timed in
/// the order the step runs them; a step that leaves early fills only the ones it reached.
#[derive(Clone, Copy, Debug, Default)]
pub struct SimTimes {
    /// The hero: input, movement, pickups, his blows, shrines and potions.
    pub hero: f64,
    /// Projectiles, angels, bombs and their hits.
    pub shots: f64,
    pub boss: f64,
    /// The foes' minds and movement (pathfinding included), and the witches.
    pub foes: f64,
    /// Frame-data fights: the sparring ring, brawls, the vaults, the run.
    pub fights: f64,
    /// New foes summoned, slime trails.
    pub spawns: f64,
    /// Touching foes: hits on the hero and on each other.
    pub contact: f64,
    /// Particles, floaters, the world's effects.
    pub effects: f64,
    /// The view's own update: camera, animation state, fades.
    pub view: f64,
}

impl SimTimes {
    pub const NAMES: [&'static str; 9] = ["hero", "shots", "boss", "foes", "fights", "spawns", "contact", "effects", "view"];

    pub fn values(&self) -> [f64; 9] {
        [self.hero, self.shots, self.boss, self.foes, self.fights, self.spawns, self.contact, self.effects, self.view]
    }
}

/// A stopwatch read lap by lap: each `lap` is the milliseconds since the last.
pub struct Lap(web_time::Instant);

impl Lap {
    pub fn start() -> Self {
        Lap(web_time::Instant::now())
    }

    pub fn lap(&mut self) -> f64 {
        let now = web_time::Instant::now();
        let ms = (now - self.0).as_secs_f64() * 1e3;
        self.0 = now;
        ms
    }
}

/// Where a frame's CPU time went, in milliseconds.
#[derive(Clone, Copy, Debug, Default)]
pub struct CpuTimes {
    pub update: f64,
    pub emit: f64,
    pub hud: f64,
    /// Static geometry upload, and recording plus submitting the frame.
    pub sync: f64,
    pub record: f64,
}

impl CpuTimes {
    pub fn total(&self) -> f64 {
        self.update + self.emit + self.hud + self.sync + self.record
    }
}

/// Lines for the F3 readout.
pub fn readout(cpu: &CpuTimes, counts: &FrameCounts, timer: Option<&GpuTimer>) -> Vec<String> {
    let mut out = vec![
        format!(
            "cpu {:.2} ms: sim {:.2} emit {:.2} hud {:.2} up {:.2} rec {:.2}",
            cpu.total(),
            cpu.update,
            cpu.emit,
            cpu.hud,
            cpu.sync,
            cpu.record
        ),
        format!(
            "{} draws {:.1}k inst {:.2}M tri {:.0} kB up",
            counts.draws,
            counts.instances as f64 / 1e3,
            counts.triangles as f64 / 1e6,
            counts.upload as f64 / 1024.0
        ),
        format!(
            "static {}/{} chunks {:.0}k/{:.0}k inst, shadow {:.0}k",
            counts.chunks_drawn,
            counts.chunks,
            counts.static_drawn as f64 / 1e3,
            counts.static_total as f64 / 1e3,
            counts.shadow_static as f64 / 1e3
        ),
        format!(
            "rigs {} ({:.1}k voxels; detail {}/{}/{})",
            counts.rigs,
            counts.rig_voxels as f64 / 1e3,
            counts.rig_levels[0],
            counts.rig_levels[1],
            counts.rig_levels[2],
        ),
        format!(
            "lights {}: {} in clusters, fullest {}, {} over the cap",
            counts.lights, counts.light_pairs, counts.busiest_cluster, counts.lights_dropped
        ),
    ];
    match timer {
        Some(t) if t.frames > 0 => {
            out.push(format!("gpu {:.2} ms", t.total));
            let mut line = String::new();
            for (i, (label, ms)) in t.passes.iter().enumerate() {
                if i > 0 && i % 4 == 0 {
                    out.push(std::mem::take(&mut line));
                }
                line.push_str(&format!("{label} {ms:.2}  "));
            }
            if !line.is_empty() {
                out.push(line);
            }
        }
        Some(_) => out.push("gpu: waiting for timings".into()),
        None => out.push("gpu: no timestamp queries on this adapter".into()),
    }
    out
}
