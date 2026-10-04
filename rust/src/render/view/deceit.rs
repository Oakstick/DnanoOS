//! The prince's cheats in the world (WARBAND.md §10.3): the false gatekeeper in his side room,
//! the old man's own sculpt, with a lantern burning in the sin's colour (the real one's burns warm
//! white) and a shadow of a dog at his feet that never wags (the real Quitmer is never still); and
//! the mimic chest, the vaults' own chest, breathing, its coins dull.

use std::sync::OnceLock;

use super::vault_decor::{push_merged, Merged};
use super::*;

/// What the renderer needs of the cheats standing in the world (set by the game each frame).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DeceitView {
    /// The false gatekeeper: where he stands (logic x, y), the way he faces, his prince's colour.
    pub keeper: Option<(V, V, u32)>,
    /// A mimic chest: where it stands, the way it faces.
    pub chest: Option<(V, V)>,
}

/// The vaults' chest and its lid, made once.
fn chest_art() -> &'static (Merged, Merged) {
    static ART: OnceLock<(Merged, Merged)> = OnceLock::new();
    ART.get_or_init(super::vault::chest)
}

impl WorldView {
    /// The false gatekeeper and the mimic chest, where they stand.
    pub(super) fn emit_deceits(&self, s: &SceneState, f: &mut Frame, time: f32) {
        let d = self.deceit;
        let ground = |p: V| {
            let (x, z) = (p.x as f32, p.y as f32);
            Vec3::new(x, floor_at(s.grid, x, z), z)
        };
        if let Some((at, facing, color)) = d.keeper {
            let feet = ground(at);
            let yaw = (facing.x as f32).atan2(facing.y as f32);
            let rig = &self.models.old_man;
            let mut pose = Pose::rest(rig);
            if let Some(i) = rig.index("head") {
                // He nods along to his own patter, too eagerly.
                pose.rot[i].x = 0.08 + (time * 2.4).sin() * 0.06;
            }
            let root = Mat4::from_translation(feet) * Mat4::from_rotation_y(yaw);
            emit_rig(&mut f.rigs, rig, &pose, root, Vec3::splat(0.92), 1.0, Vec3::ZERO);
            // His lantern in the sin's colour, hung at his side, and its light on the floor.
            let side = Vec3::new(yaw.cos(), 0.0, -yaw.sin());
            let lamp = feet + side * 0.42 + Vec3::Y * 0.85;
            let c = lin(color);
            let flick = 0.85 + (time * 9.0).sin() * 0.08 + (time * 13.7).sin() * 0.05;
            let cube = MeshKind::Cube as usize;
            f.opaque[cube].push(Instance::new(
                Mat4::from_scale_rotation_translation(Vec3::new(0.12, 0.16, 0.12), Quat::IDENTITY, lamp),
                lin(0x2a2420),
                1.0,
                c * 2.2 * flick,
                SHADE_LIT,
            ));
            glow_orb(f, lamp, 0.45, c, 0.5 * flick);
            f.lights.push(Light { pos: lamp + Vec3::Y * 0.1, color: c * 3.5 * flick, distance: 5.0 });
            // A shadow of a dog at his feet: Quitmer's shape, dark, and still.
            let dog = &self.models.dog;
            let mut still = Pose::rest(dog);
            for (name, x) in [("foreA", -1.5), ("foreB", -1.5), ("hindA", -1.35), ("hindB", -1.35)] {
                if let Some(i) = dog.index(name) {
                    still.rot[i].x = x;
                }
            }
            let dog_at = feet - side * 0.55 + Vec3::new(facing.x as f32, 0.0, facing.y as f32) * 0.25;
            let dog_root = Mat4::from_translation(dog_at) * Mat4::from_rotation_y(yaw + 0.6);
            emit_rig(&mut f.rigs, dog, &still, dog_root, Vec3::splat(0.12), 1.0, c * 0.05);
        }
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
