//! Adventures of the Ancient Blades (the crate keeps its first name, Void Harvest).
//!
//! One Rust codebase for the desktop build (a standalone executable over
//! Direct3D 12 / Vulkan / Metal through wgpu) and the web build (WebAssembly
//! over WebGPU, with WebGL2 as a fallback).
//!
//! The layers:
//!
//! - `logic`: the rules as plain data and functions: the grid and movement, level generation
//!   (estates, the arena, the hub, the gym, the Vaults of Mammon), the hero, the foes and their
//!   brains, items, progression, and `combat` (frame data, the fighter state machine, the threat
//!   manager: COMBAT.md). No rendering, audio or input (only `platform`, to keep the high score),
//!   so it is all unit-tested.
//! - `platform`, `input`, `audio`: saves (a file, or browser storage); the keyboard, mouse and
//!   controllers mapped to actions; the synthesized sounds and recorded music.
//! - `render`: `view` turns a `SceneState` into instanced boxes and lights each frame; `gpu` draws
//!   them with wgpu; `hud` draws the overlay; the rest sculpts the voxel models.
//! - `game`: the state of a run and every rule that ties the layers together, one module per part
//!   (see its docs).
//! - `app`: the window and the frame loop; `capture` renders scripted frames headless.
//!
//! One coupling runs both ways: the game owns the view and calls it directly (sparks, shakes,
//! rebuilding a level), and the HUD reads the `Game`. Separating them (the game emitting events
//! the view consumes) is the next architectural step, not yet taken.

pub mod app;
pub mod audio;
#[cfg(not(target_arch = "wasm32"))]
pub mod capture;
pub mod game;
pub mod input;
pub mod logic;
pub mod platform;
pub mod render;
#[cfg(feature = "steam")]
pub mod steam;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    app::run(app::Options::default());
}
