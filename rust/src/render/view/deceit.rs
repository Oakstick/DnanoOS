//! The prince's cheats in the world (WARBAND.md §10.3): the mimic chest, the vaults' own chest,
//! breathing, its coins dull. (The old man is never one of them.)

use std::sync::OnceLock;

use super::vault_decor::{push_merged, Merged};
use super::*;

/// What the renderer needs of the cheats standing in the world (set by the game each frame).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DeceitView {
    /// A mimic chest: where it stands, the way it faces.
    pub chest: Option<(V, V)>,
}

/// The vaults' chest and its lid, made once.
fn chest_art() -> &'static (Merged, Merged) {
    static ART: OnceLock<(Merged, Merged)> = OnceLock::new();
    ART.get_or_init(super::vault::chest)
}

impl WorldView {
    /// The mimic chest, where it stands.
    pub(super) fn emit_deceits(&self, s: &SceneState, f: &mut Frame, time: f32) {
        let d = self.deceit;
        let ground = |p: V| {
            let (x, z) = (p.x as f32, p.y as f32);
            Vec3::new(x, floor_at(s.grid, x, z), z)
        };
        if let Some((at, facing)) = d.chest {
            let base = ground(at);
            let yaw = (facing.x as f32).atan2(facing.y as f32);
            // It breathes: a slow swell, and the lid lifting a crack on each breath.
            let breath = ((time * 1.4).sin() * 0.5 + 0.5).powf(1.5);
            let m = Mat4::from_translation(base)
                * Mat4::from_rotation_y(yaw)
                * Mat4::from_scale(Vec3::new(1.0 + breath * 0.04, 1.0 + breath * 0.07, 1.0 + breath * 0.04));
            let (chest, lid) = chest_art();
            let cube = MeshKind::Cube as usize;
            push_merged(&mut f.opaque[cube], chest, m, 0.0);
            push_merged(&mut f.opaque[cube], lid, super::vault::lid_matrix(m, -0.12 * breath), 0.0);
            // (No glint, no light from its gold: a real chest's gold shines.)
        }
    }
}
