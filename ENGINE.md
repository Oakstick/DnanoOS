# Distinguisher Nano: working on the engine

This repository holds **Adventures of the Ancient Blades**, a voxel action-adventure in Rust + wgpu. It is the first game on the way to an engine called **Distinguisher Nano** ("Initializing real-time authoring engine"). The game is the engine's proving ground: everything the engine becomes must still build and play the game, unchanged.

This page is the starting point for engine work on any machine, by a person or by Claude Code.

---

## 1. Getting set up

### Tools
- **Git** and **Rust** via rustup (stable, MSVC on Windows), with the Visual Studio C++ Build Tools.
- **Python 3** with `numpy` and `soundfile`, for the music tools in `rust/tools/`.
- **Blender**, only for the animation pipeline (`rust/tools/blender/`, ANIMATION.md).
- **Claude Code**, optional; see §6 for how the work has been run.

### The code
- Clone: `git clone https://github.com/Oakstick/DnanoOS.git void-harvest`.
- Branch `rust-port` is the main line, with full history **and** the assets.
- Branch `code-only` is this snapshot without the assets: code and docs only.

### The assets (not in `code-only`)
The game embeds its assets at build time (`include_bytes!` in `rust/build.rs`, `audio/track.rs`, `render/anim/library.rs`), so **it won't build without them**. Get them in one of these ways:
- Check out `rust-port`, which has `rust/assets/` in full.
- Or copy `rust/assets/` (about 52 MB: `music/` 40 MB, `anim/` 12 MB, `fonts/`, `ui/`, `icon.ico`) from a machine that has it into the same place.

Not in git at all; copy these by hand if needed:
- `C:\Users\<you>\Games\mixamo\` (about 194 MB): Mixamo source clips, needed only to re-make animations.
- `rust/assets/engine/splash.png`: the engine's loading-screen art, placed by hand.
- Saves and settings: `%APPDATA%\VoidHarvest\`.
- Claude Code's memory notes: `C:\Users\<you>\.claude\projects\<project-folder>\memory\`. Copy them into the matching folder on the new machine after opening Claude Code there once. Keeping the same project path (`...\Games\void-harvest`) keeps the folder name the same.

### First build
From `rust/`:
- `cargo test --profile quick -- --test-threads=4`: about 950 tests, about 8 minutes.
- `cargo run --release`: play.
- `bash tools/release.sh`: a release build copied to the Desktop, with the developer menu (LEVELS) on.

---

## 2. What is here: the code map

About 201k lines of Rust in `rust/src` (2026-10-10).

| Area | Lines | What it is | Engine or game |
|---|---:|---|---|
| `platform.rs`, `app.rs`, `driver.rs`, `main.rs`, `settings.rs` | ~2.0k | window and event loop (winit), the fixed **120 Hz** simulation step, the settings file, the command line | **engine core** |
| `input/` | 2.5k | actions, two-slot rebinding, keyboard/mouse/pad (gilrs), device-aware button glyphs, rumble | **engine** |
| `audio/` | 7.1k | mixer, Ogg streaming, music with measured tempo/downbeat, procedural SFX, a formant speech synth, war drum, choir | **engine** (the tracks and voices are game) |
| `render/` core: `gpu.rs`, `voxels.rs`, `clusters.rs`, `probes.rs`, `brdf.rs`, `preset.rs`, `profile.rs`, `text.rs`, `splash.rs`, `*.wgsl` | ~25k | wgpu renderer: voxel meshing, PBR materials (RP14), AO (RP15), clustered lights (RP17), light probes (RP18), haze (RP19), quality presets (RP16), HDR and ACES, two shadow cascades, SSAO, bloom, DOF, SDF text, the engine splash | **engine** |
| `render/anim/` | 8.0k | clip runtime: GLB clips retargeted onto voxel rigs, warped by gameplay frame data, cloth | **engine** (the rigs are game) |
| `render/view/` | 52.8k | every place's sets, lights and dressing (the Garden, the Cisterns, the hub, the realms, the courts...) and the camera | **game**, with reusable builders (`crypt.rs`, `story_sets.rs`, `hell_art.rs`) |
| `render/hud/`, `hud.rs`, `overlay.rs`, `minimap.rs`, `legend.rs`, `portrait.rs` | ~12k | HUD, menus, minimap, bestiary book, panels | **game**, with reusable widgets |
| `render/` sculpts: `heroes.rs`, `foes*.rs`, `cast.rs`, `characters.rs`, `props.rs`, `foliage.rs` | ~15k | voxel characters and props sculpted in code | **game** content |
| `logic/` | 38.4k | the simulation: grid, movement, pathing, the **frame-data combat** (FSM, threat manager, archetypes), the warband AI (packs, generals, rivals, the Drum), realms, progression | **game**. `logic/combat` and `logic/war` could become an engine "action combat" module |
| `game/` | 46.8k | the game's flow: levels, story, shops, saves, courts, the bots, harnesses | **game** |
| `perf.rs`, `capture.rs`, `camcheck.rs` | ~3.2k | perf harness, headless captures, camera checker | **engine tools** |

**Determinism is a feature.** The simulation is deterministic at 120 Hz, and bots (novice, adept, expert) play whole runs headless. Use the harness to prove a refactor changed nothing:
- `--run SKILL SEED ORDER`
- `--court`, `--pit 2 8 1`, `--lair`, `--balance 2`
- `--garden`, `--meadow`
- `--camera-check`, `--perf`, `--bench`, `--capture`

---

## 3. The engine plan (proposed, to decide)

A suggested path. Each step keeps the game building and its harness output **bit-identical**, which is the refactor's safety net.

| Step | What | Done when |
|---|---|---|
| **E0** | **Workspace split.** One cargo workspace: `dnano-core` (platform, loop, settings, time), `dnano-input`, `dnano-audio`, `dnano-render`, `dnano-anim`, `dnano-tools` (perf, capture, camera check), and the game crate `ancient-blades` on top. Move code without changing it. | The game builds from the workspace; every harness is identical; the tests pass. |
| **E1** | **Assets at run time.** Replace `include_bytes!` with an asset manifest and loader (dev: from disk with hot reload; release: one packed archive). | The game runs from loose files in dev and from a pack in release; music and clips reload live. |
| **E2** | **Scene model.** Decide between keeping the bespoke structs and adopting an ECS (`hecs` or `bevy_ecs`). The simulation must stay deterministic. | A decision and a small prototype. |
| **E3** | **The authoring editor**, the "real-time authoring" in the strapline: an in-engine editor (e.g. egui) to inspect entities, edit voxel grids and level layouts, place lights, scrub animations and play from any point. Today all of this is code. | The Garden can be edited and played without recompiling. |
| **E4** | **Physics for presentation** with Rapier (ragdolls, props, debris), chosen 2026-10-06. Gameplay stays kinematic and frame-data driven. | Ragdolls on deaths behind a setting; the harness unchanged. |
| **E5** | **Platforms.** Linux/Steam Deck builds; measured presets on modest GPUs (backlog B29). | 60 FPS at Low on a Steam Deck profile. |
| **E6** | **A second, tiny game** on the engine (a sample project), to prove the split. | It builds against the engine crates only. |

**Decided:**
- The web build is dropped (2026-10-10); WebGL2 can't read the rigs' storage buffers.
- Physics will be Rapier, not Jolt.

**Open:**
- the engine's own repository (this one, or a new one);
- ECS or not;
- the editor's UI library;
- asset formats;
- the licence.

---

## 4. Rules that keep the project healthy

- **The simulation is sacred.** No rendering, UI, audio or refactor may change the simulation unless that is the point and it's stated. Prove it with the harness before and after.
- **Checks before every commit that touches code:**
  - `cargo fmt`
  - `cargo clippy --profile quick --all-targets -- -D warnings`
  - `cargo test --profile quick -- --test-threads=4`
  - The wasm clippy step ends with backlog B27.
- **Memory:** this machine ran out of commit memory with more than about 4 test/harness processes at once. Keep heavy processes low.
- **Windows quirks:**
  - Write files with LF (Python: `open(..., newline='\n')`); CRLF breaks `cargo fmt --check` and shell scripts.
  - Bash heredocs over about 200 lines fail; write big files with an editor or tool.
- **Releases:**
  - Bump the version in `rust/Cargo.toml` for every release build, then run `bash tools/release.sh`.
  - Distributables (`build-windows.ps1`) only on request.
- **Every new level or scene goes on the LEVELS map** (`game/map.rs`), the developer's jump menu.
- **Every improvement idea goes into `rust/BACKLOG.md` first;** the owner decides what to build.
- **Story rules:**
  - The old gatekeeper (Hızır) always leads to good.
  - No Devil/Satan figure anywhere.
  - The tone is a family adventure.

---

## 5. The documents

All in `rust/`:

| File | What |
|---|---|
| `README.md` | the game, controls, how to build and play |
| `BACKLOG.md` | **the work list**: every item, its status, and the owner's decisions |
| `COMBAT.md` | frame-data combat: the rules, the numbers, the harness |
| `WARBAND.md` | enemy AI (packs, generals, rivals, the Drum), the bots, every tuning log (W9–W12) |
| `REALMS.md`, `PRINCES.md`, `PRINCES-EPIC.md` | the seven realms, their princes, and the planned epic courts |
| `STORY.md`, `FINALE.md` | the cast (Toby, Hızır, Magnus, Emre, Kenji), every line, the ending |
| `RENDERING.md` | the renderer's plan and status, and each place's art passes |
| `ANIMATION.md` | the clip pipeline (Mixamo → Blender → GLB → voxel rigs) |
| `UI.md` | HUD, menus, type, input and glyphs, accessibility |
| `QA.md` | the QA passes, issues, untested items |
| `ROADMAP.md` | the game's original roadmap |

---

## 6. How the work has been run (with Claude Code)

- **One coordinator, many jobs.** The main session keeps the backlog and the owner's decisions, and starts background agents, one job each (e.g. "the realm redesign", "the jump fix").
- **Each job in its own git worktree** (`../vh-<job>`, branch `<job>`), committing as it goes, never merging itself.
- **Merging:** the coordinator reviews the job's report and captures, merges into `rust-port`, adds the job's open items to the backlog, then removes the worktree.
- **At most about 3 jobs at once**, each with at most 2 heavy processes.
- **The GPU lock:** jobs that time frames take `../gpu.lock` only around a timing run, and never time while the owner's game is running.
- **The shared git stash:** worktrees share one stash; never use a bare `git stash pop`.
- **Briefs:** each job's brief names its backlog items, the files it owns, the agents beside it, the checks, and the report it owes: what changed, harness before/after, captures, what still reads wrong.

Start engine work by reading this page, then `rust/BACKLOG.md` and `rust/RENDERING.md`. Then propose E0 as the first job.
