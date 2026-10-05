//! Animation by clips (ANIMATION.md): the heroes' jointed skeleton cut from their sculpts, glTF
//! clips made in Blender (from motion of any source) read once and sampled at 60 Hz, warped onto
//! the frame data, blended, and the juice laid on top; and the jointed rig's export for Blender.
//! The code's animation (`view::animation`) stays as the fallback and the comparison.

pub mod clip;
pub mod cloth;
#[cfg(not(target_arch = "wasm32"))]
pub mod export;
pub mod hero;
pub mod library;
pub mod skeleton;
pub mod warp;
