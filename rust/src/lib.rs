//! Adventures of the Ancient Blades (the crate keeps its first name, Void Harvest).
//!
//! One Rust codebase for the desktop build (a standalone executable over
//! Direct3D 12 / Vulkan / Metal through wgpu) and the web build (WebAssembly
//! over WebGPU, with WebGL2 as a fallback).

pub mod app;
pub mod audio;
pub mod game;
pub mod input;
pub mod logic;
pub mod platform;
pub mod render;
#[cfg(not(target_arch = "wasm32"))]
pub mod capture;
#[cfg(feature = "steam")]
pub mod steam;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    app::run(app::Options::default());
}
