//! The portraits' models (UI.md §6, `render::portrait`): whose rig, posed how, and the camera that
//! frames its head and shoulders, for `Gpu::render_portraits`.

use super::*;
use crate::render::portrait::{PortraitId, Pose as PortraitPose};

/// How a portrait is seen: its view and projection, the eye and the way it looks.
#[derive(Clone, Copy, Debug)]
pub struct PortraitCamera {
    pub view_proj: Mat4,
    pub eye: Vec3,
    pub fwd: Vec3,
}

/// The portrait camera's field of view, in degrees, and how far round from the model's front it
/// stands (a three-quarter view, turned to the model's right).
const FOV: f32 = 24.0;
const TURN: f32 = 0.38;
/// A head's share of a model's height (a man's is about a seventh; the cast's are bigger).
const HEAD: f32 = 0.2;

impl WorldView {
    /// The rig `id` stands for, and whether it is a hero (whose rig changes with his kit).
    fn portrait_rig(&self, id: PortraitId) -> &Rig {
        match id {
            PortraitId::OldMan => &self.models.old_man,
            PortraitId::Hero(kind) if kind == self.models.hero_key.0 => &self.models.hero,
            PortraitId::Hero(kind) => {
                let k = ROSTER.iter().position(|&r| r == kind).unwrap_or(0);
                self.portrait_heroes[k].get_or_init(|| forge::hero_rigs(kind, forge::Kit::plain(1, None)).0)
            }
            PortraitId::Foe(kind) => self.models.rig(animation::rig_for(kind).0),
            PortraitId::Prince(sin) => self.models.rig(match sin {
                Sin::Pride => RigId::Lucifer,
                Sin::Envy => RigId::Leviathan,
                Sin::Wrath => RigId::Amon,
                Sin::Sloth => RigId::Belphegor,
                Sin::Greed => RigId::Mammon,
                Sin::Gluttony => RigId::Beelzebub,
                Sin::Lust => RigId::Asmodeus,
            }),
        }
    }

    /**
     * Poses `id`'s model at the origin, facing +z, into `out` (drawn whole, at its finest), and
     * frames its head and shoulders: its "head" part if it has one, else the top third of it.
     * The pose breathes with `pose.time`; the old man's head and free hand move while he talks.
     */
    pub fn portrait_rigs(&self, id: PortraitId, pose: &PortraitPose, out: &mut RigList) -> PortraitCamera {
        let rig = self.portrait_rig(id);
        let mut p = Pose::rest(rig);
        let idx = |n: &str| rig.index(n);
        let t = pose.time;
        if let Some(hd) = idx("head") {
            p.rot[hd].x = (t * 1.6).sin() * 0.02;
            p.rot[hd].y = (t * 0.45).sin() * 0.08;
            if let Some(since) = pose.talking {
                p.rot[hd].x += (since * 5.5).sin() * 0.05;
            }
        }
        if let (PortraitId::OldMan, Some(since), Some(free)) = (id, pose.talking, idx("freeArm")) {
            p.rot[free].x = -0.35 - (since * 2.7).sin() * 0.2;
        }
        let root = Mat4::from_rotation_x(0.02 * (t * 1.6).sin());
        out.lod.bias = 0.0;
        let base = out.parts.len();
        out.push(rig, &p, root, Vec3::ONE, 1.0, Vec3::ZERO, rig_look::TINT_ALL, 0.0);
        // The posed voxels: the model's extent, then the top of its head part (or of it all, for a
        // rig with none) and the voxels in the band below that top, a head's height deep.
        let head = idx("head");
        let mut world: Vec<(Vec3, bool)> = Vec::new();
        for (i, part) in rig.parts.iter().enumerate() {
            let m = out.parts.get(base + i).map(Mat4::from_cols_array).unwrap_or(Mat4::IDENTITY);
            world.extend(part.cubes.iter().map(|(at, _)| (m.transform_point3(*at), Some(i) == head)));
        }
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        let mut crown = f32::MIN;
        for &(w, in_head) in &world {
            lo = lo.min(w);
            hi = hi.max(w);
            if in_head || head.is_none() {
                crown = crown.max(w.y);
            }
        }
        if lo.x > hi.x {
            (lo, hi, crown) = (Vec3::ZERO, Vec3::ONE, 1.0);
        }
        let height = hi.y - lo.y;
        let head_h = height * HEAD;
        let (mut blo, mut bhi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for &(w, in_head) in &world {
            if (in_head || head.is_none()) && w.y > crown - head_h {
                blo = blo.min(w);
                bhi = bhi.max(w);
            }
        }
        let mid = if blo.x <= bhi.x { (blo + bhi) * 0.5 } else { (lo + hi) * 0.5 };
        // Head and shoulders: a little over the crown down to the chest (the old man closer, his
        // face and beard filling the frame: he speaks from it).
        let (top, bottom) = if id == PortraitId::OldMan {
            (crown + head_h * 0.18, crown - head_h * 1.9)
        } else {
            (crown + head_h * 0.25, crown - head_h * 2.3)
        };
        let target = Vec3::new(mid.x, (top + bottom) * 0.5, mid.z);
        // (Wide enough for what stands out from the head: horns, a hat's brim.)
        let span = if blo.x <= bhi.x { (bhi.x - blo.x).max(bhi.z - blo.z) } else { 0.0 };
        let half = ((top - bottom) * 0.5).max(span * 0.56);
        let dist = half / (FOV.to_radians() * 0.5).tan();
        let eye = target + Vec3::new(TURN.sin(), 0.12, TURN.cos()).normalize() * dist;
        let view = Mat4::look_at_rh(eye, target, Vec3::Y);
        let proj = Mat4::perspective_rh(FOV.to_radians(), 1.0, (dist * 0.2).max(0.01), dist * 4.0 + 2.0);
        PortraitCamera { view_proj: proj * view, eye, fwd: (target - eye).normalize() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::enemy::EnemyKind;
    use crate::logic::heroes::HeroKind;

    #[test]
    fn every_portrait_frames_its_model_in_front_of_the_camera() {
        let view = WorldView::new();
        for id in [
            PortraitId::OldMan,
            PortraitId::Hero(HeroKind::Ragnar),
            PortraitId::Hero(HeroKind::Miyamoto),
            PortraitId::Foe(EnemyKind::Skeleton),
            PortraitId::Prince(Sin::Greed),
        ] {
            let mut rigs = RigList::default();
            let cam = view.portrait_rigs(id, &PortraitPose::still(), &mut rigs);
            assert_eq!(rigs.len(), 1, "{id:?}");
            // The framed point projects to the middle of the square.
            let target = cam.eye + cam.fwd;
            let c = cam.view_proj.project_point3(target);
            assert!(c.x.abs() < 0.05 && c.y.abs() < 0.05 && c.z > 0.0 && c.z < 1.0, "{id:?}: {c}");
        }
    }
}
