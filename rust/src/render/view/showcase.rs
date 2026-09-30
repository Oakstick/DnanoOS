//! The title screen's diorama: the three heroes on their stage in the night forest.

use super::*;

/// Where the title stage stands: well away from the dungeon, which is not drawn meanwhile.
pub(super) const STAGE: Vec3 = Vec3::new(-40.0, 0.0, -40.0);

impl WorldView {
    /**
     * The title screen's stage: the three friends still free stand in front,
     * and behind each on a stone dais towers the warrior whose soul he will
     * carry (Mehmed behind the left, Ragnar behind the middle, Miyamoto behind
     * the right), each in a glow of his soul's colour. A ring of trees,
     * braziers and fireflies round them, a warm key light, a cool rim.
     */
    pub(super) fn emit_stage(&self, s: &SceneState, f: &mut Frame) {
        use HeroKind;
        let time = s.time as f32;
        let cube = MeshKind::Cube as usize;
        let cyl = MeshKind::Cylinder as usize;
        let at = |x: f32, y: f32, z: f32| STAGE + Vec3::new(x, y, z);

        // Ground: a wide mossy platform; behind the boys a dais of dressed stone, two steps up.
        let platform = Mat4::from_translation(at(0.0, -0.09, 0.0)) * Mat4::from_scale(Vec3::new(14.0, 1.5, 14.0));
        f.opaque[cyl].push(Instance::new(platform, lin(0x2a4a28), 1.0, Vec3::ZERO, SHADE_LIT));
        for (k, (w, d, y)) in [(4.9f32, 1.9f32, 0.07f32), (4.5, 1.5, 0.2)].into_iter().enumerate() {
            for gx in 0..8 {
                let x = -w / 2.0 + (gx as f32 + 0.5) * w / 8.0;
                let v = hash(gx as f64 + k as f64 * 9.0, 3.0) as f32;
                let m = Mat4::from_translation(at(x, y, -0.75 + k as f32 * 0.0)) * Mat4::from_scale(Vec3::new(w / 8.0 - 0.02, 0.13, d));
                f.opaque[cube].push(Instance::new(m, lin(0x6a6e64) * (0.8 + 0.3 * v), 1.0, Vec3::ZERO, SHADE_LIT));
            }
        }
        // A ring of trees, and two braziers either side of the line-up.
        for i in 0..14 {
            let a = i as f32 / 14.0 * TAU + 0.2;
            // Leave the camera's side of the ring open (the camera sits toward +z).
            if a.sin() > 0.35 {
                continue;
            }
            let r = 4.6 + (i % 2) as f32 * 0.9;
            push_field(
                &mut f.opaque[cube],
                &self.models.tree,
                at(a.cos() * r, 0.0, a.sin() * r),
                a * 3.0,
                1.9 + (i % 3) as f32 * 0.3,
                Vec3::ONE,
                Vec3::ZERO,
                SHADE_LIT,
            );
        }
        let braziers = [at(-2.9, 0.0, 0.2), at(2.9, 0.0, 0.2)];
        for p in braziers {
            let post = Mat4::from_translation(p + Vec3::new(0.0, 0.45, 0.0)) * Mat4::from_scale(Vec3::new(0.12, 0.9, 0.12));
            f.opaque[cube].push(Instance::new(post, lin(0x2e2a26), 1.0, Vec3::ZERO, SHADE_LIT));
            let bowl = Mat4::from_translation(p + Vec3::new(0.0, 0.93, 0.0)) * Mat4::from_scale(Vec3::new(1.2, 0.8, 1.2));
            f.opaque[cyl].push(Instance::new(bowl, lin(0x3a3530), 1.0, Vec3::ZERO, SHADE_LIT));
            for k in 0..5 {
                let a = time * 3.0 + k as f32 * 1.3;
                let fp = p + Vec3::new(a.cos() * 0.08, 1.02 + (time * 5.0 + k as f32).sin().abs() * 0.12, a.sin() * 0.08);
                let m = Mat4::from_translation(fp) * Mat4::from_scale(Vec3::splat(0.07));
                f.additive.push(Instance::new(m, lin(0xffa040) * 1.4, 0.8, Vec3::ZERO, SHADE_UNLIT));
            }
            glow_orb(f, p + Vec3::new(0.0, 1.1, 0.0), 0.6, lin(0xffa050), 0.25);
        }

        // The warriors on the dais, and in front of each the boy who will carry his soul.
        let heroes =
            [(HeroKind::Mehmed, &self.models.mehmed), (HeroKind::Ragnar, &self.models.ragnar), (HeroKind::Miyamoto, &self.models.miyamoto)];
        for (i, (_, rig)) in heroes.iter().enumerate() {
            let x = (i as f32 - 1.0) * 1.3;
            let tint = lin(story::SOUL_TINT[i]);
            let feet = at(x, 0.27, -0.75);
            let breathe = 1.0 + (time * 2.0 + i as f32 * 1.7).sin() * 0.006;
            let root =
                Mat4::from_translation(feet) * Mat4::from_rotation_y(-x * 0.12) * Mat4::from_scale(Vec3::new(1.12, 1.12 * breathe, 1.12));
            let mut pose = Pose::rest(rig);
            if let Some(hd) = rig.index("head") {
                pose.rot[hd].y = (time * 0.5 + i as f32 * 2.0).sin() * 0.12;
            }
            let aura = tint * (0.035 + 0.02 * (time * 1.5 + i as f32).sin());
            emit_rig(&mut f.opaque[cube], rig, &pose, root, Vec3::ONE, 1.0, aura);
            // His soul's light: a halo behind him, a ring at his feet, motes rising.
            glow_orb(f, feet + Vec3::new(0.0, 0.75, -0.35), 1.25, tint, 0.2 + 0.05 * (time * 1.3 + i as f32).sin());
            let ring = Mat4::from_translation(feet + Vec3::new(0.0, 0.01, 0.0))
                * Mat4::from_rotation_x(-PI / 2.0)
                * Mat4::from_scale(Vec3::new(0.5, 0.5, 1.0));
            f.glow.push(Instance::new(ring, tint, 0.3, Vec3::ZERO, SHADE_GLOW));
            for k in 0..8 {
                let fk = k as f32;
                let u = (time * 0.4 + fk * 0.125 + i as f32 * 0.3).fract();
                let a = fk * 2.4 + time * 0.6;
                glow_orb(f, feet + Vec3::new(a.cos() * 0.4, u * 1.6, a.sin() * 0.3), 0.03, tint, 0.6 * (1.0 - u));
            }
            f.lights.push(Light { pos: feet + Vec3::new(0.0, 1.0, -0.6), color: tint * 4.0, distance: 3.0 });

            // The boy, a step in front, looking about; now and then one of them waves.
            let boy = at(x * 0.95, 0.0, 0.95);
            let cycle = (time * 0.25 + i as f32 * 0.37).fract();
            let act = if cycle < 0.18 { story::Act::Talk(cycle * 4.0 / 0.18) } else { story::Act::Stand };
            let look = (time * 0.6 + i as f32 * 2.1).sin() * 0.35;
            self.emit_kid(f, i + 1, story::kid_root(boy, -x * 0.1), act, time, look, Vec3::ZERO);
        }

        // Fireflies.
        for i in 0..50 {
            let fi = i as f32;
            let a = fi * 2.399;
            let r = 1.5 + (fi * 0.37) % 3.2;
            let p = at(a.cos() * r + (time * 0.5 + fi).sin() * 0.3, 0.5 + (fi * 0.61) % 2.0 + (time * 0.9 + fi).sin() * 0.15, a.sin() * r);
            let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.05));
            f.additive.push(Instance::new(m, lin(MOTE), 0.55 + (time * 2.2 + fi).sin() * 0.3, Vec3::ZERO, SHADE_UNLIT));
        }

        // Light: warm from the front for the boys' faces, a cool moon rim from behind, the braziers.
        let flicker = 1.0 + (time * 7.0).sin() * 0.05 + (time * 3.1).sin() * 0.04;
        f.lights.push(Light { pos: at(-1.2, 2.2, 3.0), color: lin(0xffc27a) * 22.0 * flicker, distance: 10.0 });
        f.lights.push(Light { pos: at(2.6, 2.8, -2.6), color: lin(0x8fb4ff) * 22.0, distance: 10.0 });
        f.lights.push(Light { pos: at(0.8, 2.0, 2.2), color: lin(0xfff0e0) * 5.0, distance: 6.0 });
        for p in braziers {
            f.lights.push(Light { pos: p + Vec3::new(0.0, 1.3, 0.0), color: lin(TORCH_LIGHT) * 4.0 * flicker, distance: 5.0 });
        }
        for l in &self.effect_lights {
            if l.intensity > 0.0 {
                f.lights.push(Light { pos: l.pos, color: l.color * l.intensity, distance: l.distance });
            }
        }
    }
}
