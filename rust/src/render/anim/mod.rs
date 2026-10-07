//! Animation by clips (ANIMATION.md): the heroes' jointed skeleton cut from their sculpts, glTF
//! clips made in Blender (from motion of any source) read once and sampled at 60 Hz, warped onto
//! the frame data, blended, the hold of what he carries and the juice laid on top; and the jointed
//! rig's export for Blender. The code's animation (`view::animation`) stays as the fallback and the
//! comparison.

pub mod clip;
pub mod cloth;
#[cfg(not(target_arch = "wasm32"))]
pub mod export;
pub mod foe;
pub mod foe_beasts;
pub mod foe_build;
pub mod foe_rigs;
pub mod foe_stage4;
pub mod foe_stage5;
pub mod foe_stage6;
pub mod hero;
pub mod hold;
pub mod library;
pub mod skeleton;
pub mod warp;
