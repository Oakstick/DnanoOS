# The Great Adventures of Sebastian (Rust + wgpu)

The game rebuilt in Rust from the TypeScript/Three.js version one folder up, feature for feature. It uses one codebase for two targets:

| Target | Graphics | How to build | Output |
|---|---|---|---|
| Windows executable (Steam) | wgpu over Direct3D 12, Vulkan as the fallback | `powershell -File build-windows.ps1` | `dist/windows/The Great Adventures of Sebastian.exe` |
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
| Aim camera | Mouse (click to capture) | Right stick (the camera stays where you aim it; click R3 to swing it back behind Sebastian) |
| Attack / fire | **Left click**, E | X, right trigger |
| Bomb | **Right click**, B, Shift | B, left bumper |
| Jump | Space | A |
| Camera mode | C | Y |
| Pause | Esc | Start, Back |
| Mute / FPS / Fullscreen | M / F1 / F11, Alt+Enter | |

## What changed from the web version

- **Sebastian the viking** replaces Popolon. He has a horned iron helmet with a bronze band, a braided ginger beard with gold beads, a fur mantle and red cloak, and a painted round shield on his back, and he swings a bearded axe. The foes are the **Draugr** (an undead viking, the old skeleton), the **Raven** and **Night Raven** (the old bats) and the **Bog Slime**. All of them are sculpted in code (`src/render/characters.rs`) at about twice the old resolution.
- **Start page:** a 3D diorama with Sebastian on a stone dais among his foes, braziers and fireflies, under a swaying camera. On top sit the gold "The Great Adventures of Sebastian" logo, a quest panel, a foes panel and the menu.

- **Mouse buttons:** left click attacks (sword swing or the held shot), right click drops a bomb. The first left click in play also captures the mouse for aiming.
- **HUD:** health in the top-left corner (portrait, lives, HP bar, weapon, potion effects), score in the top-right corner (gold, nuggets, best). Bombs and the dungeon name sit in the bottom-right.
- **Music tempo:** every 5 nuggets the backing track speeds up by 5% (104 BPM at the start, about 126 BPM at 20 nuggets). Collecting all 25 plays a congratulations fanfare: a brass run, a snare roll, then a ringing chord with sparkles.
- **Menus:** the title screen has Play / Quit Game buttons and the pause screen has Resume / Quit to Title / Quit Game. Use the mouse (hover and click), the arrows or W/S with Enter or Space, or the d-pad or stick with A or Start. The web build leaves out Quit Game.
- **Start:** Popolon begins at the center of the dungeon, facing its longest corridor, after a 3-2-1-GO countdown with beeps.
- **Sword:** each swing has a wind-up, a snapping strike with the hips turning into it and a small lunge, then a follow-through. Swings alternate forehand and backhand, with a sweeping trail.
- **Impact feedback:** micro dust bursts on kills, hits, shots striking walls, landings, footsteps and blasts. Screen shake scales with the impact (a tap for a wall hit, a jolt for a bomb at your feet), and kills get a brief freeze-frame.
- **Loadout panel:** the bottom-middle panel shows the held weapon with ammo pips, shield charges, and each running potion with a bar that drains as it wears off.
- **UI visibility:** larger text, drop shadows on all text, darker bordered panels, a trailing damage chunk on the health bar, and a red screen-edge flash when hurt.
- **Dynamic audio:** every effect varies a little in pitch and volume. Kill streaks climb in pitch, forehand and backhand swishes differ, and footsteps alternate (they squelch in slime). The music layers in extra hats and ghost kicks as enemies close in, and it muffles when health runs low.
- **Juiced rewards:** collected nuggets fly into the HUD counter, which punches in. The score rolls up, floating text pops in, and every five nuggets gets a banner and a chime.
- **Soundtrack:** an upbeat 8-bar track at 104 BPM, with four-on-the-floor kicks, claps, open hats, a bouncing octave bassline, a sixteenth-note arpeggio and a two-phrase lead melody. The first pass builds up layer by layer, every fourth bar ends in a snare fill, and alternate passes ornament the melody.
- **Turning on the spot:** standing still, tapping a direction turns Popolon to face it without stepping, and holding it walks. A blocked direction (wall, pit) just turns him, so you can aim a swing, a shot or a jump anywhere.
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
