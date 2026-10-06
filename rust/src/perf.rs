//! Repeatable performance captures (RP08 in the Technical Art and Rendering Plan):
//!
//!   AdventuresOfTheAncientBlades --perf [all|garden,battle30,...] [--perf-mode throughput|gameplay]
//!                                [--frames N] [--size 1920x1080] [--fx-off shadows,ao] [--lod-bias 1]
//!                                [--out perf]
//!
//! Each scene starts from the same seed and plays its setup script untimed through the driver that
//! captures use (`driver`: the script, the bot that takes the pad at a `bot:K`, one fixed step at a
//! time), so a scene with a bot is a scene with a fight. Then it draws `WARM_FRAMES` frames with the
//! simulation held, so the GPU leaves its idle clocks and the first uploads are done (a fixed count,
//! so the scene measured begins at the same simulated moment every time), and then measures its
//! stated simulated duration (`--frames` overrides the count): its own script, the view emitted, the
//! HUD drawn, the frame recorded and submitted, with at most two frames in flight (no waiting after
//! each one, unlike `--bench`).
//!
//! Two modes. **Throughput** (the default) takes one 1/120 s simulation step per drawn frame: the
//! scene keeps moving (rigs posing, the bot fighting, things uploaded) however fast the renderer
//! draws, which is what the renderer's numbers want; its CPU times are not the game's at any frame
//! rate. **Gameplay** measures each scene twice, as the game's frame loop runs it (`app.rs`'s
//! accumulator): at 60 FPS, two steps per drawn frame, and at 30 FPS, four, reporting update and
//! render CPU per drawn frame at those operating points (frames are not paced to the rate: wall
//! time is still as fast as the machine goes).
//!
//! Every scene states what must have happened in it (`Expect`): a fight's swings, damage both ways,
//! foes engaged and packs slain; a portal's every change of level; for a fight, that no frame
//! stalled (its CPU time within `FIGHT_BUDGET_MS`, no rig meshed and sent up mid-fight) and, where
//! the scene brings one in, that a kind of foe new to the fight came. A scene that falls short is
//! marked FAILED in the report, and the run exits with 3.
//!
//! Wall, CPU and GPU times are reported separately (median, 95th and 99th percentile and worst),
//! with the machine, driver, backend, build and settings, to `perf/` as a Markdown summary plus one
//! CSV of every frame per scene. Compare captures only at equal settings.

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::time::Instant;

use crate::app::STEP;
use crate::capture::{arg, parse_script};
use crate::driver::{Deeds, Driver};
use crate::game::Game;
use crate::render::gpu::Gpu;
use crate::render::hud;
use crate::render::overlay::Overlay;
use crate::render::preset::{self, Preset};
use crate::render::profile::{fx, CpuTimes, FrameCounts, SimTimes};
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

/// What must have happened while a scene was measured.
#[derive(Clone, Copy)]
struct Expect {
    /// A fight: the hero swings and deals damage, foes hunt him and their blows reach him (met on
    /// his guard or not), and at least `packs` packs (encounters) fall to their last foe.
    fight: bool,
    packs: u32,
    /// He takes damage. (Not asked of the expert bot in the vaults, which parries or blocks every
    /// blow there: nothing drew blood in 90 simulated seconds of trying.)
    hurt: bool,
    /// Changes of level shown.
    loads: usize,
    /// No measured frame may take more CPU time than this (ms), nor draw a rig the level had not
    /// made ready (`FrameCounts::rig_misses`): a fight never stutters.
    budget: Option<f64>,
    /// A kind of foe not in the fight as it was measured from comes into it (the preload's test).
    fresh: bool,
}

/// Nothing in particular: a scene for the renderer alone.
const NOTHING: Expect = Expect { fight: false, packs: 0, hurt: false, loads: 0, budget: None, fresh: false };

/**
 * A fight's frame budget: no frame's CPU time (update, emission, HUD, upload and recording) over
 * 8 ms, half a 60 FPS frame. A fight's frames take 1.3 to 2.5 ms of it at the median here, so
 * that is three times the usual frame with room to spare for the GPU's work and the OS; the
 * stalls this catches cost 9 to 26 ms (a kind of foe meshed and sent up as it came, the probes
 * worked out again round a coin taken, a wall broken). Measured after the warm-up, on the dev
 * laptop (RENDERING.md, "No stalls mid-fight").
 */
const FIGHT_BUDGET_MS: f64 = 8.0;

/// What a measured fight showed of its frame budget.
#[derive(Clone, Copy, Debug, Default)]
struct Stalls {
    /// The slowest frame's CPU time (ms) and which it was.
    worst: f64,
    worst_at: usize,
    /// Frames over the budget, and rig meshes drawn unprepared.
    over: usize,
    misses: u32,
}

impl Stalls {
    fn of(samples: &[Sample], budget: f64) -> Self {
        let mut s = Stalls::default();
        for (i, x) in samples.iter().enumerate() {
            let cpu = x.cpu.total();
            if cpu > s.worst {
                (s.worst, s.worst_at) = (cpu, i);
            }
            s.over += usize::from(cpu > budget);
            s.misses += x.counts.rig_misses;
        }
        s
    }

    /// What broke the budget, if anything.
    fn check(&self, budget: f64) -> Vec<String> {
        let mut out = Vec::new();
        if self.over > 0 {
            out.push(format!("{} frames over {budget:.0} ms of CPU (the worst {:.1} ms, frame {})", self.over, self.worst, self.worst_at));
        }
        if self.misses > 0 {
            out.push(format!("{} rigs meshed and sent up mid-fight", self.misses));
        }
        out
    }
}

impl Expect {
    /// What fell short, if anything.
    fn check(&self, d: &Deeds) -> Vec<String> {
        let mut out = Vec::new();
        if self.fight {
            if d.swings == 0 {
                out.push("the hero never swung".to_string());
            }
            if d.hits == 0 || d.dealt <= 0.0 {
                out.push("he dealt no damage".into());
            }
            if d.engaged == 0 {
                out.push("no foe engaged him".into());
            }
            if d.met == 0 {
                out.push("no foe's blow reached him".into());
            }
        }
        if self.hurt && d.taken <= 0.0 {
            out.push("he took no damage".into());
        }
        if d.packs < self.packs {
            out.push(format!("{} of {} packs slain", d.packs, self.packs));
        }
        if d.loads < self.loads {
            out.push(format!("{} of {} changes of level shown", d.loads, self.loads));
        }
        out
    }

    /// What fell short of the frames (`budget`) and of the foes that came (`fresh`): `stalls` as
    /// measured, `fresh` the kinds of foe new to the fight that came into it.
    fn check_frames(&self, stalls: &Stalls, fresh: &[crate::logic::enemy::EnemyKind]) -> Vec<String> {
        let mut out = self.budget.map(|b| stalls.check(b)).unwrap_or_default();
        if self.fresh && fresh.is_empty() {
            out.push("no kind of foe new to the fight came".into());
        }
        out
    }

    fn describe(&self) -> String {
        let mut parts = Vec::new();
        if self.fight {
            parts.push("swings, damage dealt, foes engaged, their blows met".to_string());
        }
        if self.hurt {
            parts.push("damage taken".into());
        }
        if self.packs > 0 {
            parts.push(format!("{} pack{} slain", self.packs, if self.packs == 1 { "" } else { "s" }));
        }
        if self.loads > 0 {
            parts.push(format!("{} changes of level shown", self.loads));
        }
        if self.fresh {
            parts.push("a kind of foe new to the fight".into());
        }
        if let Some(b) = self.budget {
            parts.push(format!("no frame over {b:.0} ms of CPU, no rig meshed mid-fight"));
        }
        if parts.is_empty() {
            "nothing asserted".into()
        } else {
            parts.join("; ")
        }
    }
}

#[derive(Clone, Copy)]
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
    /// The simulated seconds measured.
    secs: f64,
    expect: Expect,
}

/// Frames drawn before measuring, the simulation held.
const WARM_FRAMES: usize = 120;

/// The gameplay mode's simulation steps per drawn frame: the game's at 60 and at 30 FPS, as its
/// frame loop's accumulator takes them (`app.rs`).
const GAMEPLAY_STEPS: [usize; 2] = [2, 4];

/// The plan's scene list: a hero close-up, a dense corridor, a long vista, 30 enemies, transparency,
/// fast rotation, camera switching, destruction (also in the middle of a 30-foe fight) and portal
/// arrival; and three fights the bot plays, in the vaults and in the Pit (one of them on into its
/// next level, where foes of a new kind come).
const SCENES: &[Scene] = &[
    Scene {
        name: "closeup",
        what: "Hero close-up in the Cyclops' meadow",
        setup: "0.1:boss",
        settle: 2.0,
        during: "",
        period: 0.0,
        motion: Motion::Closeup,
        secs: 5.0,
        expect: NOTHING,
    },
    Scene {
        name: "garden",
        what: "The Garden of Krampus, walking north from the gate",
        setup: "0.1:dungeon:0",
        settle: 4.5,
        during: "0.0:+up",
        period: 0.0,
        motion: Motion::Still,
        secs: 5.0,
        expect: NOTHING,
    },
    Scene {
        name: "corridor",
        what: "A dense estate corridor (dungeon II), walking",
        setup: "0.1:dungeon:1",
        settle: 4.5,
        during: "0.0:+left,3.0:-left,3.0:+up",
        period: 0.0,
        motion: Motion::Still,
        secs: 5.0,
        expect: NOTHING,
    },
    Scene {
        name: "vista",
        what: "The Portals of Hell: the long view over the lava",
        setup: "0.1:hub",
        settle: 3.0,
        during: "",
        period: 0.0,
        motion: Motion::Turn(3.0),
        secs: 5.0,
        expect: NOTHING,
    },
    Scene {
        name: "battle30",
        what: "30 stage-2 foes (all three attack kinds) closing on the hero in the meadow",
        setup: "0.1:boss,1.5:foes:5:2",
        settle: 2.0,
        during: "0.0:attack",
        period: 0.6,
        motion: Motion::Still,
        secs: 5.0,
        expect: Expect { budget: Some(FIGHT_BUDGET_MS), ..NOTHING },
    },
    Scene {
        name: "warband",
        what: "The Vaults of Mammon with the expert bot fighting its packs: the Drum, the packs, the threat manager, the juice and the voices (WARBAND.md)",
        setup: "0.1:realm:4,0.5:bot:2",
        settle: 25.0,
        during: "",
        period: 0.0,
        motion: Motion::Still,
        secs: 20.0,
        expect: Expect { fight: true, packs: 1, hurt: false, loads: 0, budget: Some(FIGHT_BUDGET_MS), fresh: false },
    },
    Scene {
        name: "pit",
        what: "The Pit's seventh level with the expert bot fighting its inmates as they come out of the doors",
        setup: "0.1:pit:7,0.3:bot:2",
        settle: 10.0,
        during: "",
        period: 0.0,
        motion: Motion::Still,
        secs: 20.0,
        expect: Expect { fight: true, packs: 1, hurt: true, loads: 0, budget: Some(FIGHT_BUDGET_MS), fresh: false },
    },
    Scene {
        name: "descent",
        what: "The Pit's sixth level with the expert bot fighting, its clock run on to the bell: the platform goes down and the seventh level's giants come out of the doors",
        setup: "0.1:pit:6,0.3:bot:2",
        settle: 8.0,
        during: "0.0:pitclock:112",
        period: 0.0,
        motion: Motion::Still,
        secs: 22.0,
        expect: Expect { fight: true, packs: 0, hurt: false, loads: 0, budget: Some(FIGHT_BUDGET_MS), fresh: true },
    },
    Scene {
        name: "souls",
        what: "Scene I, the souls rise: see-through rigs, mist and glow",
        setup: "0.3:beat:18:2",
        settle: 0.5,
        during: "",
        period: 0.0,
        motion: Motion::Still,
        secs: 5.0,
        expect: NOTHING,
    },
    Scene {
        name: "rotate",
        what: "The Garden, the camera swung round fast",
        setup: "0.1:dungeon:0",
        settle: 4.5,
        during: "",
        period: 0.0,
        motion: Motion::Turn(40.0),
        secs: 5.0,
        expect: NOTHING,
    },
    Scene {
        name: "switch",
        what: "The Garden, switching between the chase and the top-down camera",
        setup: "0.1:dungeon:0",
        settle: 4.5,
        during: "0.5:camera",
        period: 1.0,
        motion: Motion::Still,
        secs: 5.0,
        expect: NOTHING,
    },
    Scene {
        name: "destruction",
        what: "The Garden, a bomb every second (walls breaking)",
        setup: "0.1:dungeon:0",
        settle: 4.5,
        during: "0.0:bomb,0.1:+left,0.5:-left,0.5:+right,0.9:-right",
        period: 1.0,
        motion: Motion::Still,
        secs: 5.0,
        expect: NOTHING,
    },
    Scene {
        name: "siege",
        what: "The Garden with 30 stage-2 foes closing in, a bomb every second and the sword swinging (walls breaking mid-fight)",
        setup: "0.1:dungeon:0,4.0:foes:5:2",
        settle: 5.0,
        during: "0.0:bomb,0.1:+left,0.3:attack,0.5:-left,0.5:+right,0.8:attack,0.9:-right",
        period: 1.0,
        motion: Motion::Still,
        secs: 5.0,
        expect: Expect { budget: Some(FIGHT_BUDGET_MS), ..NOTHING },
    },
    Scene {
        name: "portal",
        what: "Portal arrival: the hub, into Pride's realm and back, twice (cold then warm)",
        setup: "0.1:hub",
        settle: 2.0,
        during: "1.0:realm:0,3.0:hub,5.0:realm:0,7.0:hub",
        period: 0.0,
        motion: Motion::Still,
        secs: 9.0,
        expect: Expect { loads: 4, ..NOTHING },
    },
    Scene {
        name: "start",
        what: "Start: the Garden of Krampus loaded cold, as choosing a hero begins the adventure",
        setup: "0.1:hub",
        settle: 2.0,
        during: "1.0:dungeon:0",
        period: 0.0,
        motion: Motion::Still,
        secs: 4.0,
        expect: Expect { loads: 1, ..NOTHING },
    },
    Scene {
        name: "vault",
        what: "The Vaults of Mammon at the entrance",
        setup: "0.1:realm:4",
        settle: 3.0,
        during: "",
        period: 0.0,
        motion: Motion::Turn(4.0),
        secs: 5.0,
        expect: NOTHING,
    },
];

/// One measured frame.
#[derive(Clone, Copy, Default)]
struct Sample {
    wall: f64,
    cpu: CpuTimes,
    /// The latest GPU frame time read back (from a frame or two earlier), or none yet.
    gpu: Option<f64>,
    /// The two cascades' passes from the same read-back (near; far with its static cache when drawn).
    shadow: Option<(f64, f64)>,
    /// The simulation's sections (RP06), summed over the frame's steps (`SimTimes::NAMES`).
    sim: [f64; 9],
    counts: FrameCounts,
    /// The run's snapshot written in this frame (`game::persist`).
    saves: u32,
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
pub(crate) fn stamp() -> String {
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

fn new_game(w: u32, h: u32, lod_bias: f32) -> Game {
    let mut game = Game::new();
    game.reseed(1);
    game.viewport = (w as f32, h as f32);
    game.view.aspect = w as f32 / h as f32;
    game.view.viewport_h = h as f32;
    game.view.lod_bias = lod_bias;
    game.tour_pending = false;
    // Without the prince's cheats, as the harnesses play (WARBAND.md §10.3): a whisper the bot
    // refuses opens the old man's workshop, and the fight would be measured paused behind it.
    game.voices.deceit.off = true;
    game
}

/// The scene set: a fresh game with its setup script played untimed by the driver, which then has
/// the scene's own script next (and keeps the setup's bot playing).
fn stage(scene: &Scene, (w, h): (u32, u32), lod_bias: f32, juice_off: u32) -> (Game, Driver) {
    let mut game = new_game(w, h, lod_bias);
    game.set_juice_off(juice_off);
    let mut driver = Driver::new(parse_script(scene.setup));
    driver.run(&mut game, scene.settle);
    driver.play(parse_script(scene.during), scene.period);
    // From here on a change of level waits behind the loading screen for the renderer, as in the
    // game (RP03).
    game.await_renderer = true;
    (game, driver)
}

/// The frames a scene's simulated seconds take at `steps` simulation steps a frame.
fn frames_for(scene: &Scene, steps: usize) -> usize {
    (scene.secs / (steps as f64 * STEP)).round().max(1.0) as usize
}

/// The simulation's part of one measured frame: the camera's motion, then `steps` steps of the
/// driver (the scene's script and the bot). Returns the update's sections summed over the steps.
fn advance(game: &mut Game, driver: &mut Driver, scene: &Scene, steps: usize) -> [f64; 9] {
    if let Motion::Turn(px) = scene.motion {
        game.view.look(px, 0.0);
    }
    let mut sim = [0.0; 9];
    for _ in 0..steps {
        driver.step(game);
        for (a, b) in sim.iter_mut().zip(game.sim.values()) {
            *a += b;
        }
    }
    sim
}

/// The close-up's camera, `orbit` radians round the hero.
fn pose(game: &mut Game, scene: &Scene, orbit: f32) {
    if let Motion::Closeup = scene.motion {
        let p = game.hero.pos();
        let y = game.hero.altitude as f32 * crate::render::view::STEP;
        game.view.set_closeup(glam::Vec3::new(p.x as f32, y + 0.62, p.y as f32), orbit, 2.1);
    }
}

/// A scene played as `measure` plays it, without drawing: a renderer that has every level ready at
/// once. What happened in its measured frames.
#[cfg(test)]
fn rehearse(scene: &Scene, steps: usize) -> Deeds {
    rehearse_with_kinds(scene, steps).0
}

/// The kinds of foe that came into the fight while it was measured, not in it at the start.
fn fresh_kinds(start: &[crate::logic::enemy::EnemyKind], game: &Game, seen: &mut Vec<crate::logic::enemy::EnemyKind>) {
    for e in &game.enemies {
        if !start.contains(&e.kind()) && !seen.contains(&e.kind()) {
            seen.push(e.kind());
        }
    }
}

/// `rehearse`, with the kinds of foe new to the fight that came, and every kind the fight showed.
#[cfg(test)]
fn rehearse_with_kinds(scene: &Scene, steps: usize) -> (Deeds, Vec<crate::logic::enemy::EnemyKind>, Vec<crate::logic::enemy::EnemyKind>) {
    let (mut game, mut driver) = stage(scene, (1920, 1080), 1.0, 0);
    driver.recount(&game);
    let start: Vec<_> = game.enemies.iter().map(|e| e.kind()).collect();
    let (mut fresh, mut all) = (Vec::new(), Vec::new());
    for _ in 0..frames_for(scene, steps) {
        advance(&mut game, &mut driver, scene, steps);
        game.finish_loading();
        fresh_kinds(&start, &game, &mut fresh);
        fresh_kinds(&[], &game, &mut all);
    }
    (driver.deeds(&game), fresh, all)
}

struct Run {
    samples: Vec<Sample>,
    /// The first drawn frame after the setup (cold: static upload, first rig meshes), in ms.
    first_frame: f64,
    /// Changes of level while measuring, timed from the load to the first frame shown (RP03).
    transitions: Vec<crate::game::Transition>,
    /// What happened while measuring.
    deeds: Deeds,
    /// Kinds of foe new to the fight that came into it while measuring.
    fresh: Vec<crate::logic::enemy::EnemyKind>,
    /// What preparing the level's rigs cost: meshing them (CPU, ms) and making their buffers
    /// (ms, bytes), at the last preparation.
    prepared: (f64, f64, u64),
}

/// Measures `frames` frames of a scene at `steps` simulation steps a frame, after the warm-up.
fn measure(gpu: &mut Gpu, scene: &Scene, steps: usize, frames: usize, (w, h): (u32, u32), lod_bias: f32) -> Run {
    let (mut game, mut driver) = stage(scene, (w, h), lod_bias, gpu.fx_off);
    let target = gpu.offscreen();
    let mut frame = Frame::default();
    let mut overlay = Overlay::new();
    let logical = (w as f32, h as f32);
    let fps = (1.0 / (steps as f64 * STEP)).round() as u32;
    let mut in_flight: VecDeque<wgpu::SubmissionIndex> = VecDeque::new();
    let mut orbit = 0.0f32;
    let mut first_frame = 0.0;
    let mut samples = Vec::with_capacity(frames);
    let mut last = Instant::now();
    let mut gpu_seen = 0;
    let mut start_kinds = Vec::new();
    let mut new_kinds = Vec::new();
    for i in 0..WARM_FRAMES + frames {
        let warming = i < WARM_FRAMES;
        if i == WARM_FRAMES {
            driver.recount(&game);
            start_kinds = game.enemies.iter().map(|e| e.kind()).collect();
        }
        let writes = game.persist.writes;
        let mut cpu = CpuTimes::default();
        let mut sim = [0.0; 9];
        // The warm-up draws the scene as the setup left it; the measured frames move it on.
        if !warming {
            let s = Instant::now();
            sim = advance(&mut game, &mut driver, scene, steps);
            cpu.update = s.elapsed().as_secs_f64() * 1e3;
            orbit += 0.01;
        }
        pose(&mut game, scene, orbit);
        let s = Instant::now();
        game.view.emit(&game.scene(), &mut frame);
        cpu.emit = s.elapsed().as_secs_f64() * 1e3;
        let s = Instant::now();
        overlay.begin(logical.0, logical.1);
        hud::draw(&mut overlay, &game, fps);
        cpu.hud = s.elapsed().as_secs_f64() * 1e3;
        overlay.map.upload(|r, px| gpu.write_atlas(r, px));
        let s = Instant::now();
        gpu.sync_static(&mut game.view);
        cpu.sync = s.elapsed().as_secs_f64() * 1e3;
        gpu.render_portraits(&game.view, &overlay.portraits.due);
        let s = Instant::now();
        gpu.draw(&target, &game.view, &frame, &overlay.verts, logical);
        if game.loading.is_some() && gpu.level_ready(&game.view) {
            game.finish_loading();
        }
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
                "perf: {} first frame {first_frame:.0} ms (emit {:.0}, hud {:.0}, upload {:.0}, record {:.0}, GPU wait {:.0})",
                scene.name,
                cpu.emit,
                cpu.hud,
                cpu.sync,
                cpu.record,
                s.elapsed().as_secs_f64() * 1e3
            );
        }
        let now = Instant::now();
        let wall = (now - last).as_secs_f64() * 1e3;
        last = now;
        let fresh = gpu.timer.as_ref().filter(|t| t.frames > gpu_seen);
        let gpu_ms = fresh.map(|t| t.total);
        let shadow = fresh.map(|t| {
            let ms = |label: &str| t.passes.iter().filter(|p| p.0 == label).map(|p| p.1).sum::<f64>();
            (ms("shadow near"), ms("shadow far") + ms("shadow cache"))
        });
        if let Some(t) = fresh {
            gpu_seen = t.frames;
        }
        if !warming {
            samples.push(Sample { wall, cpu, gpu: gpu_ms, shadow, sim, counts: gpu.counts, saves: game.persist.writes - writes });
            fresh_kinds(&start_kinds, &game, &mut new_kinds);
        }
    }
    let _ = gpu.device.poll(wgpu::PollType::Wait);
    let deeds = driver.deeds(&game);
    let prepared = (game.view.resident().ms, gpu.resident_cost.0, gpu.resident_cost.1);
    Run { samples, first_frame, transitions: game.transitions, deeds, fresh: new_kinds, prepared }
}

pub fn run(args: &[String]) -> i32 {
    std::env::set_var("VH_NO_AUDIO", "1");
    // Saves are written behind, as the game writes them.
    crate::platform::write_behind();
    // Each scene starts from no save at all, in a folder of its own under VH_SAVE_DIR (or the
    // temporary folder): what one scene saves (progress, the Ledger, a checkpoint) must not change
    // the next, or a scene would play differently alone and in `all`.
    let saves =
        std::env::var_os("VH_SAVE_DIR").map(std::path::PathBuf::from).unwrap_or_else(|| std::env::temp_dir().join("void-harvest-perf"));
    let size = arg(args, "--size")
        .and_then(|s| s.split_once('x'))
        .and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?)))
        .unwrap_or((1920u32, 1080u32));
    // A frame count overrides the scenes' durations (their checks may then fall short).
    let frames_arg: Option<usize> = arg(args, "--frames").and_then(|n| n.parse().ok());
    let gameplay = match arg(args, "--perf-mode") {
        None | Some("throughput") => false,
        Some("gameplay") => true,
        Some(other) => {
            eprintln!("no such mode {other}; the modes are throughput and gameplay");
            return 2;
        }
    };
    let rates: &[usize] = if gameplay { &GAMEPLAY_STEPS } else { &[1] };
    let which = arg(args, "--perf").filter(|s| !s.starts_with("--")).unwrap_or("all");
    let fx_list = arg(args, "--fx-off").unwrap_or("");
    let preset = match arg(args, "--preset") {
        Some(name) => match Preset::named(name) {
            Some(p) => p,
            None => {
                eprintln!("no such preset; the presets are: {}", preset::ALL.map(|p| p.name).join(", "));
                return 2;
            }
        },
        None => Preset::default_for_build(),
    };
    let preset = Preset { gpu_cull: preset.gpu_cull || args.iter().any(|a| a == "--gpu-cull"), ..preset };
    let lod_bias: f32 = arg(args, "--lod-bias").and_then(|b| b.parse().ok()).unwrap_or(preset.lod_bias);
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
    gpu.set_preset(preset);
    gpu.measure_occlusion = args.iter().any(|a| a == "--occlusion");
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
    let s = &preset.shadows;
    let _ = writeln!(
        md,
        "| Preset | {} (for the {}): shadows {} near, {} far, {}+{} filter taps; {} volumetric lamps; scenery culled on the {} |",
        preset.name,
        preset.target,
        s.near_size,
        s.far_size,
        s.search_taps,
        s.filter_taps,
        preset.volumetric_lamps,
        if preset.gpu_cull { "GPU" } else { "CPU" }
    );
    let _ = writeln!(md, "| Level-of-detail bias | {lod_bias} |");
    let _ = writeln!(
        md,
        "| Mode | {} |",
        if gameplay {
            "gameplay: each scene at 60 FPS (two 1/120 s simulation steps per drawn frame) and at 30 FPS (four), as the game's frame loop runs them; frames not paced to the rate"
        } else {
            "throughput: one 1/120 s simulation step per drawn frame (the scene keeps moving however fast it is drawn)"
        }
    );
    let _ = writeln!(
        md,
        "| Frames per scene | {}, after {WARM_FRAMES} frames of warm-up with the simulation held; at most 2 in flight |\n",
        match frames_arg {
            Some(n) => format!("{n} (set by --frames)"),
            None => "the scene's simulated seconds (below) at its steps per frame".into(),
        }
    );
    let _ = writeln!(
        md,
        "Times in ms. Wall is the time between frames, unpaced; CPU is the update (every simulation step of the frame), \
         emission, HUD, upload and recording, and Render is CPU without the update; GPU is from timestamp queries, first pass \
         to last. GPU work overlaps the CPU's.\n"
    );
    let _ = writeln!(
        md,
        "| Scene | Sim | Wall p50 | p95 | p99 | max | CPU p50 | p95 | Update p50 | p95 | Render p50 | p95 | GPU p50 | p95 | p99 | max | First frame | Draws | Tri (M) | Upload kB |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
    );
    let _ = std::fs::create_dir_all(&out_dir);
    let mut cascades = String::new();
    let mut sim_rows = String::new();
    let mut moves = String::new();
    let mut hidden_rows = String::new();
    let mut grain_rows = String::new();
    let mut checks = String::new();
    let mut budget_rows = String::new();
    let mut failed = Vec::new();
    let runs: Vec<(&Scene, usize)> = scenes.iter().flat_map(|s| rates.iter().map(move |&k| (*s, k))).collect();
    for (scene, steps) in runs {
        let fps = (1.0 / (steps as f64 * STEP)).round() as u32;
        let (label, sim_label) = if gameplay {
            (format!("{} @{fps}", scene.name), format!("{fps} FPS, {steps} steps"))
        } else {
            (scene.name.to_string(), "1 step".to_string())
        };
        let frames = frames_arg.unwrap_or_else(|| frames_for(scene, steps));
        let file = if gameplay { format!("{}-{fps}fps", scene.name) } else { scene.name.to_string() };
        let dir = saves.join(format!("perf-{file}"));
        crate::platform::finish_writes();
        let _ = std::fs::remove_dir_all(&dir);
        std::env::set_var("VH_SAVE_DIR", &dir);
        eprintln!("perf: {label} ({frames} frames, {:.1} s simulated) ...", frames as f64 * steps as f64 * STEP);
        let run = measure(&mut gpu, scene, steps, frames, size, lod_bias);
        let s = &run.samples;
        let wall = percentiles(s.iter().map(|x| x.wall).collect());
        let cpu = percentiles(s.iter().map(|x| x.cpu.total()).collect());
        let update = percentiles(s.iter().map(|x| x.cpu.update).collect());
        let render = percentiles(s.iter().map(|x| x.cpu.total() - x.cpu.update).collect());
        let gpu_t = percentiles(s.iter().filter_map(|x| x.gpu).collect());
        let mean = |f: &dyn Fn(&Sample) -> f64| s.iter().map(f).sum::<f64>() / s.len().max(1) as f64;
        let draws = mean(&|x| x.counts.draws as f64);
        let tris = mean(&|x| x.counts.triangles as f64) / 1e6;
        let upload = mean(&|x| x.counts.upload as f64) / 1024.0;
        let line = format!(
            "| {label} | {sim_label} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.1} | {:.0} | {:.2} | {:.0} |",
            wall[0],
            wall[1],
            wall[2],
            wall[3],
            cpu[0],
            cpu[1],
            update[0],
            update[1],
            render[0],
            render[1],
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
        // The cascades against their budgets (RP16). The budgets are for the target GPU; on any
        // other machine the shares only say where the shadow time goes.
        let near = percentiles(s.iter().filter_map(|x| x.shadow.map(|c| c.0)).collect());
        let far = percentiles(s.iter().filter_map(|x| x.shadow.map(|c| c.1)).collect());
        let b = &preset.budget;
        // The fine cascade is not drawn where nothing focuses it (the souls' gallery).
        let near_cells = if near[1] > 0.0 {
            format!(
                "{:.2} | {:.2} | {:.0}% | {:.0}k",
                near[0],
                near[1],
                near[1] / b.shadow_near_ms * 100.0,
                mean(&|x| x.counts.shadow_near as f64) / 1e3
            )
        } else {
            "not drawn | | |".to_string()
        };
        let _ = writeln!(
            cascades,
            "| {label} | {near_cells} | {:.2} | {:.2} | {:.0}% | {:.0}k |",
            far[0],
            far[1],
            far[1] / b.shadow_far_ms * 100.0,
            mean(&|x| (x.counts.shadow_static - x.counts.shadow_near) as f64) / 1e3,
        );
        let mut csv = String::from("frame,wall_ms,cpu_ms,update_ms,emit_ms,hud_ms,upload_ms,record_ms,gpu_ms,shadow_near_ms,shadow_far_ms,draws,instances,triangles,upload_bytes,static_drawn,shadow_static,shadow_near,rigs,rig_voxels,lights,lights_dropped,light_pairs,busiest_cluster,rig_misses,saves");
        for n in SimTimes::NAMES {
            let _ = write!(csv, ",sim_{n}_ms");
        }
        csv.push('\n');
        for (i, x) in s.iter().enumerate() {
            let c = &x.counts;
            let _ = writeln!(
                csv,
                "{i},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                x.wall,
                x.cpu.total(),
                x.cpu.update,
                x.cpu.emit,
                x.cpu.hud,
                x.cpu.sync,
                x.cpu.record,
                x.gpu.map(|g| format!("{g:.3}")).unwrap_or_default(),
                x.shadow.map(|c| format!("{:.3}", c.0)).unwrap_or_default(),
                x.shadow.map(|c| format!("{:.3}", c.1)).unwrap_or_default(),
                c.draws,
                c.instances,
                c.triangles,
                c.upload,
                c.static_drawn,
                c.shadow_static,
                c.shadow_near,
                c.rigs,
                c.rig_voxels,
                c.lights,
                c.lights_dropped,
                c.light_pairs,
                c.busiest_cluster,
                c.rig_misses,
                x.saves
            );
            csv.pop();
            for v in x.sim {
                let _ = write!(csv, ",{v:.3}");
            }
            csv.push('\n');
        }
        let _ = std::fs::write(format!("{out_dir}/{when}-{file}.csv"), csv);
        {
            let by = |f: &dyn Fn(&FrameCounts) -> u64| mean(&|x| f(&x.counts) as f64);
            let (chunk, group, cluster) = (by(&|c| c.faces_by_chunk), by(&|c| c.faces_by_group), by(&|c| c.faces_by_cluster));
            if chunk > 0.0 {
                let _ = writeln!(
                    grain_rows,
                    "| {label} | {:.0}k | {:.0}k ({:.0}%) | {:.0}k ({:.0}%) |",
                    chunk / 1e3,
                    group / 1e3,
                    group / chunk * 100.0,
                    cluster / 1e3,
                    cluster / chunk * 100.0
                );
            }
        }
        if gpu.measure_occlusion {
            let seen = mean(&|x| x.counts.faces_seen as f64);
            let hidden = mean(&|x| x.counts.faces_hidden as f64);
            let _ =
                writeln!(hidden_rows, "| {label} | {:.0}k | {:.0}k | {:.1}% |", seen / 1e3, hidden / 1e3, hidden / seen.max(1.0) * 100.0);
        }
        for tr in &run.transitions {
            let _ = writeln!(
                moves,
                "| {label} | {} | {} | {:.0} | {:.0} |",
                tr.label,
                if tr.cold { "cold" } else { "warm" },
                tr.load_ms,
                tr.ms
            );
        }
        // The update's sections: median, 95th percentile and worst (RP06).
        let mut row = format!("| {label} |");
        for k in 0..SimTimes::NAMES.len() {
            let p = percentiles(s.iter().map(|x| x.sim[k]).collect());
            let _ = write!(row, " {:.2} / {:.2} / {:.1} |", p[0], p[1], p[3]);
        }
        let _ = writeln!(sim_rows, "{row}");
        // The frames against the fight's budget (no stalls), and what the level made ready.
        let budget = scene.expect.budget.unwrap_or(FIGHT_BUDGET_MS);
        let stalls = Stalls::of(s, budget);
        let saves: u32 = s.iter().map(|x| x.saves).sum();
        let _ = writeln!(
            budget_rows,
            "| {label} | {} | {:.1} (frame {}) | {} | {} | {saves} | {:.0} | {:.0} | {:.1} |",
            scene.expect.budget.map(|b| format!("{b:.0} ms")).unwrap_or_else(|| "none".into()),
            stalls.worst,
            stalls.worst_at,
            stalls.over,
            stalls.misses,
            run.prepared.0,
            run.prepared.1,
            run.prepared.2 as f64 / 1e6
        );
        // What happened, against what must have.
        let d = &run.deeds;
        let mut short = scene.expect.check(d);
        short.extend(scene.expect.check_frames(&stalls, &run.fresh));
        let verdict = if short.is_empty() { "passed".to_string() } else { format!("**FAILED**: {}", short.join("; ")) };
        if !short.is_empty() {
            eprintln!("perf: {label} FAILED: {}", short.join("; "));
            failed.push(label.clone());
        }
        let _ = writeln!(
            checks,
            "| {label} | {:.1} | {} | {} | {:.0} | {} | {:.0} | {} | {} | {} | {} | {verdict} |",
            d.steps as f64 * STEP,
            d.swings,
            d.hits,
            d.dealt,
            d.met,
            d.taken,
            d.engaged,
            d.kills,
            d.packs,
            d.loads
        );
    }
    let b = &preset.budget;
    let _ = writeln!(
        md,
        "\n## Shadow cascades\n\nGPU ms per cascade, and the 95th percentile as a share of the preset's budget on its target GPU \
         ({:.1} ms near, {:.1} ms far). Far includes redrawing its static cache when that happens. Casters are static \
         instances (thousands).\n\n| Scene | Near p50 | p95 | of budget | casters | Far p50 | p95 | of budget | casters |\n|---|---|---|---|---|---|---|---|---|\n{cascades}",
        b.shadow_near_ms, b.shadow_far_ms
    );
    let _ = writeln!(
        md,
        "\n## Frame budget\n\nNo stalls mid-fight: each scene's slowest frame of CPU time (ms) and which it was, the frames over \
         its budget (a fight's: {FIGHT_BUDGET_MS:.0} ms), the rig meshes drawn that the level had not made ready as it loaded \
         (each one meshed and sent up in its frame), and the run's snapshot written while measuring. Then what the level's \
         preparation cost behind the loading screen: meshing its rigs (CPU ms), making their buffers (ms) and their size \
         (MB), at the scene's last level.\n\n| Scene | Budget | Worst | Over | Rig misses | Saves | Meshing | Buffers | MB |\n|---|---|---|---|---|---|---|---|---|\n{budget_rows}"
    );
    let _ = writeln!(
        md,
        "\n## Simulation step\n\nWhere the update's time goes, per section, summed over the frame's simulation steps (ms: median / \
         95th percentile / worst). The view section is the camera and animation state; drawing preparation is under CPU emit above.\n\n| Scene | {} |\n|---|{}\n{sim_rows}",
        SimTimes::NAMES.join(" | "),
        "---|".repeat(SimTimes::NAMES.len())
    );
    if !grain_rows.is_empty() {
        let (groups, clusters) = gpu.face_cluster_counts();
        let _ = writeln!(
            md,
            "\n## Culling granularity\n\nStatic faces the camera would draw (mean per frame), culled by whole chunks, by chunk and facing group \
             ({groups} groups in the last level), and by clusters of at most 64 faces with their facing ({clusters} clusters), each 48 bytes \
             on the GPU (RP21).\n\n| Scene | Chunks | Facing groups | 64-face clusters |\n|---|---|---|---|\n{grain_rows}"
        );
    }
    if !hidden_rows.is_empty() {
        let _ = writeln!(
            md,
            "\n## Hidden scenery\n\nStatic faces in the camera's view, and those in chunks wholly hidden behind nearer scenery (an occlusion \
             query per chunk in the depth prepass, nearest first): what chunk-level occlusion culling could skip (RP20).\n\n| Scene | In view | Hidden | Share |\n|---|---|---|---|\n{hidden_rows}"
        );
    }
    if !moves.is_empty() {
        let _ = writeln!(
            md,
            "\n## Changes of level\n\nFrom the load to the first frame shown, in ms, behind the loading screen (RP03). Load is building the \
             level and its view; the rest is the renderer making it ready (uploading the scenery, working out its light). Cold is \
             the level's first load in this run of the game, warm a return to it.\n\n| Scene | Level | | Load | To first frame |\n|---|---|---|---|---|\n{moves}"
        );
    }
    let _ = writeln!(
        md,
        "\n## What was played\n\nWhat happened in each scene's measured frames (the driver's count: swings the hero began, his blows \
         landed, damage dealt, foes' blows that reached him and the damage he took, the most foes hunting him at once, foes and \
         packs slain, changes of level shown), against what the scene must show.\n\n| Scene | Seconds | Swings | Hits | Dealt | Met | Taken | Engaged | Kills | Packs | Loads | Check |\n|---|---|---|---|---|---|---|---|---|---|---|---|\n{checks}"
    );
    let _ = writeln!(md, "\n## Scenes\n");
    for scene in SCENES {
        let _ = writeln!(md, "- **{}** ({:.0} s; {}): {}.", scene.name, scene.secs, scene.expect.describe(), scene.what);
    }
    let path = format!("{out_dir}/{when}.md");
    match std::fs::write(&path, md) {
        Ok(()) if !failed.is_empty() => {
            eprintln!("perf: wrote {path}; FAILED: {}", failed.join(", "));
            3
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn scene(name: &str) -> &'static Scene {
        SCENES.iter().find(|s| s.name == name).expect("a scene of that name")
    }

    #[test]
    fn the_gameplay_mode_takes_the_games_steps_per_frame() {
        for (fps, steps) in [(60.0, GAMEPLAY_STEPS[0]), (30.0, GAMEPLAY_STEPS[1])] {
            assert_eq!(((1.0 / fps) / STEP).round() as usize, steps, "{fps} FPS");
        }
        let garden = scene("garden");
        let (mut game, mut driver) = stage(garden, (640, 360), 1.0, 0);
        for steps in [1, 2, 4] {
            driver.recount(&game);
            advance(&mut game, &mut driver, garden, steps);
            assert_eq!(driver.deeds.steps as usize, steps, "{steps} steps a frame");
            assert_eq!(frames_for(garden, steps) * steps, (garden.secs / STEP).round() as usize, "the same seconds at any rate");
        }
    }

    #[test]
    fn the_warband_scene_fights() {
        let d = rehearse(scene("warband"), 1);
        eprintln!("warband: {d:?}");
        assert_eq!(scene("warband").expect.check(&d), Vec::<String>::new());
    }

    #[test]
    fn the_pit_scene_fights() {
        let d = rehearse(scene("pit"), 2);
        eprintln!("pit: {d:?}");
        assert_eq!(scene("pit").expect.check(&d), Vec::<String>::new());
    }

    #[test]
    fn the_descent_scene_brings_a_new_kind_of_foe_into_the_fight_and_the_pit_made_it_ready() {
        let descent = scene("descent");
        let (d, fresh, all) = rehearse_with_kinds(descent, 2);
        eprintln!("descent: {d:?}, new kinds {fresh:?}");
        assert_eq!(descent.expect.check(&d), Vec::<String>::new());
        assert!(!fresh.is_empty(), "the seventh level's foes came: {all:?}");
        // Every kind the fight showed was made ready as the Pit was loaded.
        let (g, _) = stage(descent, (640, 360), 1.0, 0);
        let ready = g.level_kinds();
        assert!(all.iter().all(|k| ready.contains(k)), "{all:?} against {ready:?}");
    }

    #[test]
    fn a_fight_over_its_budget_or_with_a_rig_meshed_mid_fight_says_so() {
        let frame = |cpu: f64, misses: u32| {
            let mut x = Sample { cpu: CpuTimes { update: cpu, ..Default::default() }, ..Default::default() };
            x.counts.rig_misses = misses;
            x
        };
        let calm = [frame(2.0, 0), frame(3.0, 0)];
        let stalls = Stalls::of(&calm, FIGHT_BUDGET_MS);
        assert!(stalls.check(FIGHT_BUDGET_MS).is_empty());
        assert_eq!((stalls.worst, stalls.worst_at), (3.0, 1));
        let stalled = [frame(2.0, 0), frame(14.0, 1), frame(9.0, 0)];
        let s = Stalls::of(&stalled, FIGHT_BUDGET_MS);
        assert_eq!((s.over, s.misses, s.worst_at), (2, 1, 1));
        assert_eq!(s.check(FIGHT_BUDGET_MS).len(), 2);
        // Each fight scene keeps the budget; the renderer's own scenes do not.
        for name in ["warband", "pit", "battle30", "siege", "descent"] {
            assert_eq!(scene(name).expect.budget, Some(FIGHT_BUDGET_MS), "{name}");
            assert!(scene(name).expect.check_frames(&s, &[]).len() >= 2, "{name}");
        }
        assert!(scene("garden").expect.check_frames(&s, &[]).is_empty());
        assert_eq!(scene("descent").expect.check_frames(&Stalls::of(&calm, FIGHT_BUDGET_MS), &[]).len(), 1, "no new kind came");
    }

    #[test]
    fn the_portal_scene_makes_every_change_of_level() {
        let portal = scene("portal");
        let d = rehearse(portal, 1);
        assert_eq!(d.loads, portal.expect.loads, "{d:?}");
        assert!(portal.expect.check(&d).is_empty());
        // At the old fixed count of 600 frames, its last return was never reached.
        assert!(portal.secs > 7.0 && frames_for(portal, 1) > 600);
    }

    #[test]
    fn a_scene_that_falls_short_says_so() {
        let idle = Deeds::default();
        let short = scene("warband").expect.check(&idle);
        assert_eq!(short.len(), 5, "{short:?}");
        assert_eq!(scene("pit").expect.check(&idle).len(), 6, "and the Pit asks for blood too");
        assert_eq!(scene("portal").expect.check(&idle), vec!["0 of 4 changes of level shown".to_string()]);
        assert!(NOTHING.check(&idle).is_empty());
    }
}
