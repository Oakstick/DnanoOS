//! The portraits' models (UI.md §6, `render::portrait`): whose rig, posed how, and the camera that
//! frames its head and shoulders, for `Gpu::render_portraits`.

use super::*;
use crate::render::portrait::{Figure, PortraitId, Pose as PortraitPose};

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
/// How fast a bestiary page's figure turns (radians a second): once round in about fourteen seconds.
const FIGURE_TURN: f32 = 0.45;

thread_local! {
    /// Each figure's framing (its middle and the half-height it is framed to), made once a rig.
    static FRAMES: std::cell::RefCell<HashMap<usize, (Vec3, f32)>> = std::cell::RefCell::new(HashMap::new());
}

/// The mimic chest as a bestiary page shows it: the vaults' chest with its lid thrown back, a
/// mouth inside, teeth along both rims, a tongue over the front and eyes glowing under the lid.
fn mimic() -> &'static Rig {
    static RIG: std::sync::OnceLock<Rig> = std::sync::OnceLock::new();
    RIG.get_or_init(|| {
        use crate::render::characters::{mat, Sculpt};
        let (wood, wood_l) = (0x5a3a22 | mat::WOOD, 0x7a5232 | mat::WOOD);
        let gold = 0xf2c53d | mat::GOLD;
        let (mouth, throat) = (0x5a1018 | mat::SKIN, 0x2a0408 | mat::SKIN);
        let (tooth, tongue, eye) = (0xeee6cc | mat::IVORY, 0xc84a64 | mat::SKIN, 0xffd040 | mat::GLOW);
        let mut s = Sculpt::new(14, 18, 13);
        // The box: wood, darker gold bands round it, the lock plate on its front.
        s.cuboid(0, 0, 3, 13, 6, 12, wood);
        for x in [0, 13] {
            s.cuboid(x, 0, 3, x, 6, 12, wood_l);
        }
        for x in [3, 10] {
            s.cuboid(x, 0, 3, x, 6, 12, gold);
        }
        s.cuboid(8, 3, 12, 9, 5, 12, gold);
        // The mouth: the box's top a throat, its rim red flesh, teeth standing on the front rim.
        s.cuboid(1, 6, 4, 12, 6, 11, throat);
        for x in 1..13 {
            s.set(x, 6, 11, mouth);
            s.set(x, 6, 4, mouth);
            if x % 2 == 1 {
                s.set(x, 7, 11, tooth);
            }
        }
        // The tongue, lolling out over the front.
        s.cuboid(4, 7, 6, 5, 7, 11, tongue);
        s.cuboid(4, 4, 12, 5, 6, 12, tongue);
        // The lid, thrown back and leaning, red inside, teeth hanging from its rim, two eyes.
        for y in 7..18 {
            let z = 3 - (y - 7) / 4;
            s.cuboid(0, y, z - 1, 13, y, z - 1, wood);
            s.cuboid(1, y, z, 12, y, z, mouth);
            for x in [3, 10] {
                s.set(x, y, z - 1, gold);
            }
        }
        for x in 1..13 {
            if x % 2 == 0 {
                s.set(x, 8, 4, tooth);
            }
        }
        for x in [4, 5, 8, 9] {
            s.set(x, 13, 2, eye);
        }
        build_rig(&s.finish(0.0), &[], 0.05)
    })
}

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
            // (The Peerless is his own reflection: the hero as he is played.)
            PortraitId::Figure(Figure::Foe(EnemyKind::Peerless)) => &self.models.hero,
            PortraitId::Figure(Figure::Foe(kind)) => self.models.rig(animation::rig_for(kind).0),
            PortraitId::Figure(Figure::Cyclops) => &self.models.cyclops,
            PortraitId::Figure(Figure::Mimic) => mimic(),
        }
    }

    /**
     * A bestiary page's figure: `id`'s whole model at rest, breathing, seen from a camera that
     * goes slowly round it (the model turning, as the portrait's light goes with the camera), its
     * framing fixed for the model (made once a rig: it never swims as the figure turns).
     */
    fn figure_rigs(&self, id: PortraitId, pose: &PortraitPose, out: &mut RigList) -> PortraitCamera {
        let rig = self.portrait_rig(id);
        let p = Pose::rest(rig);
        let t = pose.time;
        let root = Mat4::from_rotation_x(0.015 * (t * 1.6).sin());
        out.lod.bias = 0.0;
        let base = out.parts.len();
        out.push(rig, &p, root, Vec3::ONE, 1.0, Vec3::ZERO, rig_look::TINT_ALL, 0.0);
        let key = rig as *const Rig as usize;
        let (centre, half) = FRAMES.with(|f| {
            *f.borrow_mut().entry(key).or_insert_with(|| {
                // The posed voxels: their box, and how far they reach from the upright through its middle.
                let mut world: Vec<Vec3> = Vec::new();
                for (i, part) in rig.parts.iter().enumerate() {
                    let m = out.parts.get(base + i).map(Mat4::from_cols_array).unwrap_or(Mat4::IDENTITY);
                    world.extend(part.cubes.iter().map(|(at, _)| m.transform_point3(*at)));
                }
                let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
                for w in &world {
                    lo = lo.min(*w);
                    hi = hi.max(*w);
                }
                if lo.x > hi.x {
                    return (Vec3::new(0.0, 0.5, 0.0), 0.6);
                }
                let mid = (lo + hi) * 0.5;
                let reach = world.iter().map(|w| Vec2::new(w.x - mid.x, w.z - mid.z).length()).fold(0.0, f32::max);
                (mid, ((hi.y - lo.y) * 0.5).max(reach) * 1.08 + rig.size * 2.0)
            })
        });
        let dist = half / (FOV.to_radians() * 0.5).tan();
        let a = TURN + t * FIGURE_TURN;
        let eye = centre + Vec3::new(a.sin(), 0.16, a.cos()).normalize() * dist;
        let view = Mat4::look_at_rh(eye, centre, Vec3::Y);
        let proj = Mat4::perspective_rh(FOV.to_radians(), 1.0, (dist * 0.2).max(0.01), dist * 4.0 + 2.0);
        PortraitCamera { view_proj: proj * view, eye, fwd: (centre - eye).normalize() }
    }

    /**
     * Poses `id`'s model at the origin, facing +z, into `out` (drawn whole, at its finest), and
     * frames its head and shoulders: its "head" part if it has one, else the top third of it.
     * The pose breathes with `pose.time`; the old man's head and free hand move while he talks.
     */
    pub fn portrait_rigs(&self, id: PortraitId, pose: &PortraitPose, out: &mut RigList) -> PortraitCamera {
        if matches!(id, PortraitId::Figure(_)) {
            return self.figure_rigs(id, pose, out);
        }
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
