//! Lust's court, **The Rose Bower** (`logic::court::lust` lays it out), where Asmodeus waits: its art.

use super::*;
use crate::logic::combat::prince::{Trap, SPIKES};

/// The court's light (for now the Counting House's).
pub fn lust_atmosphere() -> Atmosphere {
    super::court_greed::counting_atmosphere()
}

/// Its ground haze: density and falloff (for now the Counting House's).
pub const HAZE: (f32, f32) = (0.012, 2.0);

/// What the view keeps of the court for its moving parts.
#[derive(Clone, Debug, Default)]
pub struct BowerView {}

impl WorldView {
    /// The court's static scene, as the level loads (for now none).
    pub(super) fn build_lust_court(&mut self, _grid: &Grid) {
        self.bower = Some(BowerView::default());
    }

    /// The court's moving parts and lights, each frame (for now none).
    pub(super) fn emit_lust_court(&self, _s: &SceneState, _time: f32, _hero: Vec3, _f: &mut Frame) {
        let Some(_view) = &self.bower else { return };
    }

    /// Asmodeus's thorns: where each will burst, a dark stain in the floor with a ring of rose light
    /// closing on it (the tell), then black thorns thrust up out of the ground, and sinking back.
    pub(super) fn emit_bower_thorns(&self, traps: &[Trap], grid: &Grid, time: f32, f: &mut Frame) {
        for t in traps {
            let ground = floor_at(grid, t.pos.x as f32, t.pos.y as f32);
            let at = Vec3::new(t.pos.x as f32, ground, t.pos.y as f32);
            let seed = (t.pos.x * 3.7 + t.pos.y * 1.9) as f32;
            let r = t.radius as f32;
            if let Some(b) = t.burst {
                if b >= SPIKES {
                    continue;
                }
                let u = b as f32 / SPIKES as f32;
                let rise = if u < 0.15 { u / 0.15 } else { 1.0 - ((u - 0.15) / 0.85).powi(2) };
                for k in 0..11 {
                    let a = k as f32 * 2.39 + seed;
                    let rr = if k == 0 { 0.0 } else { r * (0.25 + 0.65 * ((k * 37 % 10) as f32 / 10.0)) };
                    let tall = (0.6 + 0.5 * ((k * 13 % 7) as f32 / 7.0)) * rise;
                    let lean = 0.25 * (a * 2.0).sin();
                    let p = at + Vec3::new(a.cos() * rr, tall * 0.5, a.sin() * rr);
                    let m = Mat4::from_translation(p)
                        * Mat4::from_rotation_y(a)
                        * Mat4::from_rotation_z(lean)
                        * Mat4::from_scale(Vec3::new(0.07, tall.max(0.01), 0.07));
                    f.opaque[MeshKind::Cube as usize].push(Instance::new(m, lin(0x24160f), 1.0, Vec3::ZERO, SHADE_LIT));
                    if k % 3 == 0 {
                        // A rose on the thorn's tip.
                        let tip = p + Vec3::new(-lean * tall * 0.5, tall * 0.5, 0.0);
                        let m = Mat4::from_translation(tip) * Mat4::from_scale(Vec3::splat(0.09 * rise));
                        f.opaque[MeshKind::Sphere as usize].push(Instance::new(m, lin(0xb0122e), 1.0, lin(0x60081a) * 0.4, SHADE_LIT));
                    }
                }
                f.lights.push(Light { pos: at + Vec3::new(0.0, 0.5, 0.0), color: lin(0xff3a5a) * 3.0 * (1.0 - u), distance: 2.6 });
                continue;
            }
            // The tell: a dark stain where the ground will open, a ring of rose light closing in.
            let fill = if t.flight > 0 { 0.0 } else { t.tell() };
            let late = t.flight == 0 && t.fuse <= 8;
            let blink = if late { 0.6 + (time * 40.0).sin().abs() * 0.8 } else { 0.0 };
            let m = Mat4::from_translation(at + Vec3::new(0.0, 0.02, 0.0))
                * Mat4::from_rotation_x(-PI / 2.0)
                * Mat4::from_scale(Vec3::new(r, r, 1.0));
            f.transparent[MeshKind::Disc as usize].push(Instance::new(
                m,
                lin(0x2a0810) * (0.6 + blink),
                0.25 + 0.35 * fill,
                Vec3::ZERO,
                SHADE_UNLIT,
            ));
            let n = ((TAU * r) / 0.22).ceil() as i32;
            for k in 0..n {
                let a = k as f32 * TAU / n as f32 + time * 0.6;
                let p = at + Vec3::new(a.cos() * r, 0.03, a.sin() * r);
                let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(0.08, 0.012, 0.08));
                f.additive.push(Instance::new(m, lin(0xff4a7a) * (0.9 + blink), 0.7, Vec3::ZERO, SHADE_UNLIT));
            }
            let inner = r * (1.0 - fill);
            let m = Mat4::from_translation(at + Vec3::new(0.0, 0.03, 0.0))
                * Mat4::from_rotation_x(-PI / 2.0)
                * Mat4::from_scale(Vec3::new(inner + 0.05, inner + 0.05, 1.0));
            f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, lin(0xff7aa8) * 1.2, 0.5, Vec3::ZERO, SHADE_UNLIT));
            // Thorn tips pricking through the floor as the fuse burns.
            for k in 0..4 {
                let a = seed + k as f32 * 1.7;
                let rr = r * 0.45 * (k as f32 / 4.0 + 0.2);
                let tall = 0.12 * fill;
                let p = at + Vec3::new(a.cos() * rr, tall * 0.5, a.sin() * rr);
                let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(0.04, tall.max(0.005), 0.04));
                f.opaque[MeshKind::Cube as usize].push(Instance::new(m, lin(0x2a1a12), 1.0, Vec3::ZERO, SHADE_LIT));
            }
        }
    }
}
