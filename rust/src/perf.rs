//! Repeatable performance captures (RP08 in the Technical Art and Rendering Plan):
//!
//!   AdventuresOfTheAncientBlades --perf [all|garden,battle30,...] [--frames 600] [--size 1920x1080]
//!                                [--fx-off shadows,ao] [--out perf]
//!
//! Each scene starts from the same seed and script, runs a second of untimed frames so the GPU
//! leaves its idle clocks and the first uploads are done, then plays `--frames` frames as the
//! game would: one simulation step per frame, the view emitted, the HUD drawn, the frame
//! recorded and submitted, with at most two frames in flight (no waiting after each one, unlike
//! `--bench`). Wall, CPU and GPU times are reported separately (median, 95th and 99th percentile
//! and worst), with the machine, driver, backend, build and settings, to `perf/` as a Markdown
//! summary plus one CSV of every frame per scene. Compare captures only at equal settings.

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::time::Instant;

use crate::app::STEP;
use crate::capture::{arg, exec, parse_script, Cmd};
use crate::game::Game;
use crate::render::gpu::Gpu;
use crate::render::hud;
use crate::render::overlay::Overlay;
use crate::render::profile::{fx, CpuTimes, FrameCounts};
use crate::render::view::Frame;

/// How the camera moves while a scene is measured.
#[derive(Clone, Copy)]
enum Motion {
    Still,
    /// A mouse turn of this many pixels a frame (fast rotation).
    Turn(f64),
    /// A portrait of the hero from close by, circling him slowly.
    Closeup,
}

struct Scene {
    name: &'static str,
    what: &'static str,
    /// The script that sets the scene, and how many seconds of it to play untimed.
    setup: &'static str,
    settle: f64,
    /// A script run while measuring, from the first measured frame, repeating every `period` seconds (0: once).
    during: &'static str,
    period: f64,
    motion: Motion,
}

/// The plan's scene list: a hero close-up, a dense corridor, a long vista, 30 enemies, transparency,
/// fast rotation, camera switching, destruction and portal arrival.
const SCENES: &[Scene] = &[
    Scene {
        name: "closeup",
        what: "Hero close-up in the Cyclops' meadow",
        setup: "0.1:boss",
        settle: 2.0,
        during: "",
        period: 0.0,
        motion: Motion::Closeup,
    },
    Scene {
        name: "garden",
        what: "The Garden of Krampus, walking north from the gate",
        setup: "0.1:dungeon:0",
        settle: 4.5,
        during: "0.0:+up",
        period: 0.0,
        motion: Motion::Still,
    },
    Scene {
        name: "corridor",
        what: "A dense estate corridor (dungeon II), walking",
        setup: "0.1:dungeon:1",
        settle: 4.5,
        during: "0.0:+left,3.0:-left,3.0:+up",
        period: 0.0,
        motion: Motion::Still,
    },
    Scene {
        name: "vista",
        what: "The Portals of Hell: the long view over the lava",
        setup: "0.1:hub",
        settle: 3.0,
        during: "",
        period: 0.0,
        motion: Motion::Turn(3.0),
    },
    Scene {
        name: "battle30",
        what: "30 stage-2 foes (all three attack kinds) closing on the hero in the meadow",
        setup: "0.1:boss,1.5:foes:5:2",
        settle: 2.0,
        during: "0.0:attack",
        period: 0.6,
        motion: Motion::Still,
    },
    Scene {
        name: "souls",
        what: "Scene I, the souls rise: see-through rigs, mist and glow",
        setup: "0.3:beat:18:2",
        settle: 0.5,
        during: "",
        period: 0.0,
        motion: Motion::Still,
    },
    Scene {
        name: "rotate",
        what: "The Garden, the camera swung round fast",
        setup: "0.1:dungeon:0",
        settle: 4.5,
        during: "",
        period: 0.0,
        motion: Motion::Turn(40.0),
    },
    Scene {
        name: "switch",
        what: "The Garden, switching between the chase and the top-down camera",
        setup: "0.1:dungeon:0",
        settle: 4.5,
        during: "0.5:camera",
        period: 1.0,
        motion: Motion::Still,
    },
    Scene {
        name: "destruction",
        what: "The Garden, a bomb every second (walls breaking)",
        setup: "0.1:dungeon:0",
        settle: 4.5,
        during: "0.0:bomb,0.1:+left,0.5:-left,0.5:+right,0.9:-right",
        period: 1.0,
        motion: Motion::Still,
    },
    Scene {
        name: "portal",
        what: "Portal arrival: the hub, into Pride's realm and back, twice (cold then warm)",
        setup: "0.1:hub",
        settle: 2.0,
        during: "1.0:realm:0,3.0:hub,5.0:realm:0,7.0:hub",
        period: 0.0,
        motion: Motion::Still,
    },
    Scene {
        name: "vault",
        what: "The Vaults of Mammon at the entrance",
        setup: "0.1:realm:4",
        settle: 3.0,
        during: "",
        period: 0.0,
        motion: Motion::Turn(4.0),
    },
];

/// One measured frame.
#[derive(Clone, Copy, Default)]
struct Sample {
    wall: f64,
    cpu: CpuTimes,
    /// The latest GPU frame time read back (from a frame or two earlier), or none yet.
    gpu: Option<f64>,
    counts: FrameCounts,
}

fn percentiles(mut v: Vec<f64>) -> [f64; 4] {
    if v.is_empty() {
        return [0.0; 4];
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let at = |q: f64| v[((v.len() - 1) as f64 * q).round() as usize];
    [at(0.5), at(0.95), at(0.99), v[v.len() - 1]]
}

/// What the machine is, as best the game can tell.
fn machine(gpu: &Gpu) -> Vec<(String, String)> {
    let info = &gpu.adapter;
    let mut out = vec![
        ("GPU".into(), info.name.clone()),
        ("Backend".into(), format!("{:?}", info.backend)),
        ("Driver".into(), format!("{} {}", info.driver, info.driver_info).trim().to_string()),
    ];
    #[cfg(windows)]
    {
        let reg = std::process::Command::new("reg")
            .args(["query", r"HKLM\HARDWARE\DESCRIPTION\System\CentralProcessor\0", "/v", "ProcessorNameString"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| {
                s.lines().find(|l| l.contains("ProcessorNameString")).map(|l| l.split("REG_SZ").last().unwrap_or("").trim().to_string())
            });
        out.push(("CPU".into(), reg.unwrap_or_else(|| std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_default())));
    }
    out.push(("Logical cores".into(), std::thread::available_parallelism().map(|n| n.to_string()).unwrap_or_default()));
    out.push(("Build".into(), format!("{} {}", env!("CARGO_PKG_VERSION"), option_env!("VH_BUILD").unwrap_or("unknown"))));
    out.push(("Profile".into(), if cfg!(debug_assertions) { "debug".into() } else { "optimised".into() }));
    out
}

/// UTC now as 2026-10-03-141502.
fn stamp() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Days since 1970 to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}-{:02}{:02}{:02}", rem / 3600, rem / 60 % 60, rem % 60)
}

fn new_game(w: u32, h: u32) -> Game {
    let mut game = Game::new();
    game.reseed(1);
    game.viewport = (w as f32, h as f32);
    game.view.aspect = w as f32 / h as f32;
    game.tour_pending = false;
    game
}

/// Plays `script` from `*t` until `until`, without drawing.
fn play(game: &mut Game, script: &[(f64, Cmd)], next: &mut usize, t: &mut f64, until: f64) {
    let mut bot = None;
    while *t < until {
        while *next < script.len() && script[*next].0 <= *t {
            exec(game, &script[*next].1, &mut bot);
            *next += 1;
        }
        game.update(STEP);
        *t += STEP;
    }
}

struct Run {
    samples: Vec<Sample>,
    /// The first drawn frame after the setup (cold: static upload, first rig meshes), in ms.
    first_frame: f64,
}

fn measure(gpu: &mut Gpu, scene: &Scene, frames: usize, (w, h): (u32, u32)) -> Run {
    let mut game = new_game(w, h);
    let setup = parse_script(scene.setup);
    let (mut next, mut t) = (0, 0.0);
    play(&mut game, &setup, &mut next, &mut t, scene.settle);

    let target = gpu.offscreen();
    let mut frame = Frame::default();
    let mut overlay = Overlay::new();
    let logical = (w as f32, h as f32);
    let mut in_flight: VecDeque<wgpu::SubmissionIndex> = VecDeque::new();
    let during = parse_script(scene.during);
    let mut during_next = 0;
    let mut during_t = 0.0;
    let mut orbit = 0.0f32;
    let mut bot = None;
    let mut first_frame = 0.0;
    let mut samples = Vec::with_capacity(frames);
    let warm = Instant::now();
    let mut warming = true;
    let mut last = Instant::now();
    let mut gpu_seen = 0;
    let mut i = 0;
    while samples.len() < frames {
        let mut cpu = CpuTimes::default();
        // The scene's own goings-on, on the simulation's clock (not during the warm-up).
        if !warming {
            while during_next < during.len() && during[during_next].0 <= during_t {
                exec(&mut game, &during[during_next].1, &mut bot);
                during_next += 1;
            }
            during_t += STEP;
            if scene.period > 0.0 && during_t >= scene.period {
                during_t -= scene.period;
                during_next = 0;
            }
        }
        match scene.motion {
            Motion::Turn(px) => game.view.look(px, 0.0),
            Motion::Closeup | Motion::Still => {}
        }
        let s = Instant::now();
        game.update(STEP);
        cpu.update = s.elapsed().as_secs_f64() * 1e3;
        if let Motion::Closeup = scene.motion {
            orbit += 0.01;
            let p = game.hero.pos();
            let y = game.hero.altitude as f32 * crate::render::view::STEP;
            game.view.set_closeup(glam::Vec3::new(p.x as f32, y + 0.62, p.y as f32), orbit, 2.1);
        }
        let s = Instant::now();
        game.view.emit(&game.scene(), &mut frame);
        cpu.emit = s.elapsed().as_secs_f64() * 1e3;
        let s = Instant::now();
        overlay.begin(logical.0, logical.1);
        hud::draw(&mut overlay, &game, 60);
        cpu.hud = s.elapsed().as_secs_f64() * 1e3;
        let s = Instant::now();
        gpu.sync_static(&mut game.view);
        cpu.sync = s.elapsed().as_secs_f64() * 1e3;
        let s = Instant::now();
        gpu.draw(&target, &game.view, &frame, &overlay.verts, logical);
        cpu.record = s.elapsed().as_secs_f64() * 1e3;
        if let Some(idx) = gpu.last_submit.clone() {
            in_flight.push_back(idx);
        }
        // At most two frames queued, as a swapchain would allow.
        while in_flight.len() > 2 {
            let idx = in_flight.pop_front().unwrap();
            let _ = gpu.device.poll(wgpu::PollType::WaitForSubmissionIndex(idx));
        }
        if i == 0 {
            let s = Instant::now();
            let _ = gpu.device.poll(wgpu::PollType::Wait);
            first_frame = last.elapsed().as_secs_f64() * 1e3;
            eprintln!(
                "perf: {} first frame {first_frame:.0} ms (update {:.0}, emit {:.0}, hud {:.0}, upload {:.0}, record {:.0}, GPU wait {:.0})",
                scene.name,
                cpu.update,
                cpu.emit,
                cpu.hud,
                cpu.sync,
                cpu.record,
                s.elapsed().as_secs_f64() * 1e3
            );
        }
        i += 1;
        let now = Instant::now();
        let wall = (now - last).as_secs_f64() * 1e3;
        last = now;
        if warming {
            if warm.elapsed().as_secs_f64() >= 1.0 {
                warming = false;
            }
            continue;
        }
        let gpu_ms = gpu.timer.as_ref().filter(|t| t.frames > gpu_seen).map(|t| {
            gpu_seen = t.frames;
            t.total
        });
        samples.push(Sample { wall, cpu, gpu: gpu_ms, counts: gpu.counts });
    }
    let _ = gpu.device.poll(wgpu::PollType::Wait);
    Run { samples, first_frame }
}

pub fn run(args: &[String]) -> i32 {
    std::env::set_var("VH_NO_AUDIO", "1");
    if std::env::var_os("VH_SAVE_DIR").is_none() {
        let dir = std::env::temp_dir().join("void-harvest-perf");
        let _ = std::fs::remove_dir_all(&dir);
        std::env::set_var("VH_SAVE_DIR", dir);
    }
    let size = arg(args, "--size")
        .and_then(|s| s.split_once('x'))
        .and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?)))
        .unwrap_or((1920u32, 1080u32));
    let frames: usize = arg(args, "--frames").and_then(|n| n.parse().ok()).unwrap_or(600);
    let which = arg(args, "--perf").filter(|s| !s.starts_with("--")).unwrap_or("all");
    let fx_list = arg(args, "--fx-off").unwrap_or("");
    let out_dir = arg(args, "--out").unwrap_or("perf").to_string();
    let scenes: Vec<&Scene> = SCENES.iter().filter(|s| which == "all" || which.split(',').any(|n| n == s.name)).collect();
    if scenes.is_empty() {
        eprintln!("no such scene; the scenes are: {}", SCENES.iter().map(|s| s.name).collect::<Vec<_>>().join(", "));
        return 2;
    }
    let mut gpu = match pollster::block_on(Gpu::new(None, size)) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    gpu.fx_off = fx::parse(fx_list);
    if gpu.timer.is_none() {
        eprintln!("note: this adapter has no timestamp queries; GPU times are not reported");
    }
    let when = stamp();
    let mut md = String::new();
    let _ = writeln!(md, "# Performance capture {when}\n");
    let _ = writeln!(md, "| Setting | Value |\n|---|---|");
    for (k, v) in machine(&gpu) {
        let _ = writeln!(md, "| {k} | {v} |");
    }
    let _ = writeln!(md, "| Resolution | {}x{} native, 4x MSAA, no upscaling |", size.0, size.1);
    let _ = writeln!(md, "| Effects off | {} |", if fx_list.is_empty() { "none" } else { fx_list });
    let _ = writeln!(md, "| Frames per scene | {frames} after 1 s of warm-up, at most 2 in flight |\n");
    let _ = writeln!(
        md,
        "Times in ms. Wall is the time between frames as the game would pace them; CPU is simulation, emission, HUD, upload and \
         recording; GPU is from timestamp queries, first pass to last. GPU work overlaps the CPU's.\n"
    );
    let _ = writeln!(
        md,
        "| Scene | Wall p50 | p95 | p99 | max | CPU p50 | p95 | GPU p50 | p95 | p99 | max | First frame | Draws | Tri (M) | Upload kB |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
    );
    let _ = std::fs::create_dir_all(&out_dir);
    for scene in scenes {
        eprintln!("perf: {} ({frames} frames) ...", scene.name);
        let run = measure(&mut gpu, scene, frames, size);
        let s = &run.samples;
        let wall = percentiles(s.iter().map(|x| x.wall).collect());
        let cpu = percentiles(s.iter().map(|x| x.cpu.total()).collect());
        let gpu_t = percentiles(s.iter().filter_map(|x| x.gpu).collect());
        let mean = |f: &dyn Fn(&Sample) -> f64| s.iter().map(f).sum::<f64>() / s.len().max(1) as f64;
        let draws = mean(&|x| x.counts.draws as f64);
        let tris = mean(&|x| x.counts.triangles as f64) / 1e6;
        let upload = mean(&|x| x.counts.upload as f64) / 1024.0;
        let line = format!(
            "| {} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.1} | {:.0} | {:.2} | {:.0} |",
            scene.name,
            wall[0],
            wall[1],
            wall[2],
            wall[3],
            cpu[0],
            cpu[1],
            gpu_t[0],
            gpu_t[1],
            gpu_t[2],
            gpu_t[3],
            run.first_frame,
            draws,
            tris,
            upload
        );
        eprintln!("{line}");
        let _ = writeln!(md, "{line}");
        let mut csv = String::from("frame,wall_ms,cpu_ms,update_ms,emit_ms,hud_ms,upload_ms,record_ms,gpu_ms,draws,instances,triangles,upload_bytes,static_drawn,shadow_static,rigs,rig_voxels,lights,lights_dropped\n");
        for (i, x) in s.iter().enumerate() {
            let c = &x.counts;
            let _ = writeln!(
                csv,
                "{i},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{},{},{},{},{},{},{},{},{},{},{}",
                x.wall,
                x.cpu.total(),
                x.cpu.update,
                x.cpu.emit,
                x.cpu.hud,
                x.cpu.sync,
                x.cpu.record,
                x.gpu.map(|g| format!("{g:.3}")).unwrap_or_default(),
                c.draws,
                c.instances,
                c.triangles,
                c.upload,
                c.static_drawn,
                c.shadow_static,
                c.rigs,
                c.rig_voxels,
                c.lights,
                c.lights_dropped
            );
        }
        let _ = std::fs::write(format!("{out_dir}/{when}-{}.csv", scene.name), csv);
    }
    let _ = writeln!(md, "\n## Scenes\n");
    for scene in SCENES {
        let _ = writeln!(md, "- **{}**: {}.", scene.name, scene.what);
    }
    let path = format!("{out_dir}/{when}.md");
    match std::fs::write(&path, md) {
        Ok(()) => {
            eprintln!("perf: wrote {path}");
            0
        }
        Err(e) => {
            eprintln!("could not write {path}: {e}");
            1
        }
    }
}
