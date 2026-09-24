# Void Harvest (Rust + wgpu)

The game rebuilt in Rust from the TypeScript/Three.js version one folder up, feature for feature. It uses one codebase for two targets:

| Target | Graphics | How to build | Output |
|---|---|---|---|
| Windows executable (Steam) | wgpu over Direct3D 12, Vulkan as the fallback | `powershell -File build-windows.ps1` | `dist/windows/VoidHarvest.exe` |
| Web | WebAssembly over WebGPU, WebGL2 as the fallback | `powershell -File build-web.ps1` | `dist/web/` |

wgpu is Rust's implementation of the WebGPU API. The browser build uses the browser's own WebGPU, and the desktop build maps the same calls onto the native APIs. The shaders are WGSL either way.

## Requirements

- Rust stable (`rustup`), plus `rustup target add wasm32-unknown-unknown` for the web build
- `cargo install wasm-bindgen-cli --version 0.2.128` (it must match the `wasm-bindgen` version in `Cargo.lock`)
- On Windows, the Visual Studio C++ build tools (the Windows SDK's `rc.exe` embeds the icon)

## Running

```
cargo run -- --windowed          # debug build in a window
cargo run --release              # borderless fullscreen (F11 or Alt+Enter toggles)
cargo test                       # rules, level generation, movement, audio, input
```

Serve `web/` over http (for example `python -m http.server 5174 --directory web`) after `build-web.ps1`. Browsers will not load wasm from `file://`.

## Controls

| | Keyboard and mouse | Controller |
|---|---|---|
| Move | WASD / arrows (relative to the camera) | Left stick, d-pad |
| Aim camera | Mouse (click to capture) | Right stick |
| Attack / fire | **Left click**, E | X, right trigger |
| Bomb | **Right click**, B, Shift | B, left bumper |
| Jump | Space | A |
| Camera mode | C | Y |
| Pause | Esc | Start, Back |
| Mute / FPS / Fullscreen | M / F1 / F11, Alt+Enter | |

## What changed from the web version

- **Mouse buttons:** left click attacks (sword swing or the held shot), right click drops a bomb. The first left click in play also captures the mouse for aiming.
- **HUD:** health in the top-left corner (portrait, lives, HP bar, weapon, potion effects), score in the top-right corner (gold, nuggets, best). Bombs and the dungeon name sit in the bottom-right.
- **Music tempo:** every 5 nuggets the backing track speeds up by 5% (78 BPM at the start, about 95 BPM at 20 nuggets). Collecting all 25 plays a congratulations fanfare: a brass run, a snare roll, then a ringing chord with sparkles.
- **Menus:** the title screen has Play / Quit Game buttons and the pause screen has Resume / Quit to Title / Quit Game. Use the mouse (hover and click), the arrows or W/S with Enter or Space, or the d-pad or stick with A or Start. The web build leaves out Quit Game.
- **Weight:** Popolon no longer snaps to full speed. Holding a direction accelerates him to the same top speed as before in 0.2 s. Letting go coasts him to rest on the next tile center under friction. Reversing brakes before he turns, and corners bleed a little speed. Hitboxes, `occupied` tiles and collisions are unchanged (`src/logic/hero.rs`, covered by tests).

## Layout

```
src/logic/     rules: grid, level generation, hero, enemies, items, projectiles, systems
src/game.rs    screens, the round, and the rules that tie entities together
src/render/    view.rs (scene -> instances + lights), gpu.rs (wgpu), shaders.wgsl, hud.rs, overlay.rs, voxels.rs
src/audio/     software synth, lo-fi music sequencer, sound effects
src/input.rs   keyboard, mouse, touch and gamepad mapped to actions
src/app.rs     winit window, event loop, fixed 120 Hz step
src/capture.rs headless screenshots: --capture out.png --time 6 --script "0.1:confirm,3:+up,4:bomb"
steam/         SteamPipe build scripts and the release checklist
```

The dungeon generator uses the same seeded RNG and hash as the TypeScript version, and a test checks that the generated layout matches the web build tile for tile. Lighting follows Three.js (Lambert, hemisphere, physically based point-light falloff, ACES filmic, fog after sRGB encode). Sampled pixels match the Three.js build to within one or two levels.
