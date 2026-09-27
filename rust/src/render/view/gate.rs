//! The estate's golden keys and the gate they unlock, and the smoke that
//! rises from its braziers and from every candle in the garden.
//!
//! The gatehouse stands in the north wall: two stone towers with battlements
//! either side of a pointed arch, oak doors banded with iron, and above the
//! arch eight lock lamps, one lit for every key found. With all eight, a
//! beacon of light rises from it; as Sebastian comes up, the doors swing in on
//! a golden glow.

use super::*;
use crate::render::characters::Sculpt;
use crate::render::props::mesh_from;

mod pal {
    pub const STONE: [u32; 3] = [0x8e8a7c, 0x7c786c, 0x9e9a8c];
    pub const STONE_D: u32 = 0x5e5a50;
    pub const SLATE: u32 = 0x3e424c;
    pub const OAK: [u32; 3] = [0x5a3a1e, 0x4e3218, 0x664424];
    pub const IRON: u32 = 0x26282c;
    pub const GOLD: u32 = 0xf2c53d;
    pub const VOID: u32 = 0x060606;
    pub const EMBER: u32 = 0xff8a30;
    pub const SMOKE: u32 = 0x7e848c;
}
use pal::*;

/// A golden key standing on its tip: a ring bow with a cross in it, a collar, a shaft, two teeth.
pub fn golden_key() -> Mesh {
    const G: u32 = 0xf2c53d;
    const G_D: u32 = 0xc8942a;
    let mut s = Sculpt::new(13, 30, 3);
    for y in 18..30 {
        for x in 0..13 {
            let d = ((x as f32 - 6.0).powi(2) + (y as f32 - 23.5).powi(2)).sqrt();
            if (3.0..=5.6).contains(&d) {
                for z in 0..3 {
                    s.set(x, y, z, if d > 5.0 { G_D } else { G });
                }
            }
        }
    }
    s.cuboid(6, 20, 1, 6, 27, 1, G);
    s.cuboid(3, 23, 1, 9, 23, 1, G);
    s.cuboid(4, 16, 0, 8, 17, 2, G_D);
    s.cuboid(5, 3, 1, 7, 15, 1, G);
    s.cuboid(8, 0, 1, 11, 2, 1, G);
    s.cuboid(8, 5, 1, 10, 6, 1, G);
    s.cuboid(5, 0, 1, 7, 2, 1, G_D);
    mesh_from(&s.finish(0.04), 0.022)
}

fn cube(out: &mut Vec<Instance>, pos: Vec3, scale: Vec3, rot: Quat, color: Vec3) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(scale, rot, pos), color, 1.0, Vec3::ZERO, SHADE_LIT));
}

fn stone(h: f64) -> Vec3 {
    lin(STONE[(h * 3.0) as usize % 3]) * (0.94 + h as f32 * 0.12)
}

/// Where the lock lamps sit on the gate's face, relative to its foot.
fn lock_spot(i: u32, n: u32) -> Vec3 {
    let a = PI * (0.15 + 0.7 * (i as f32 + 0.5) / n as f32);
    Vec3::new(-a.cos() * 0.52, 1.66 + a.sin() * 0.3, 0.14)
}

impl WorldView {
    /// The gatehouse's stonework (the doors and lamps move, so they are drawn each frame).
    pub(super) fn build_gate(&mut self, grid: &Grid, gate: P) {
        let floor = grid.elevation(gate.x, gate.y + 1) as f32 * STEP;
        let o = Vec3::new(gate.x as f32, floor, gate.y as f32);
        let q = Quat::IDENTITY;
        let out = &mut self.static_cubes;
        // Threshold and a dark passage behind the doors.
        cube(out, o + Vec3::new(0.0, -0.04, 0.05), Vec3::new(1.0, 0.1, 0.9), q, lin(STONE_D));
        cube(out, o + Vec3::new(0.0, 0.8, -0.42), Vec3::new(0.9, 1.7, 0.08), q, lin(VOID));
        // Towers: coursed stone, a battlemented top, a slate spire.
        for side in [-1.0f32, 1.0] {
            let t = o + Vec3::new(side * 0.98, 0.0, 0.0);
            let base = -0.3;
            for k in 0..7 {
                let h = hash(gate.x as f64 + side as f64 * 3.0, k as f64);
                let y0 = base + k as f32 * 0.38;
                let w = if k == 0 { 0.82 } else { 0.72 };
                cube(out, t + Vec3::new(0.0, y0 + 0.19, 0.0), Vec3::new(w, 0.37, w), q, stone(h));
            }
            let top = base + 7.0 * 0.38;
            cube(out, t + Vec3::new(0.0, top + 0.04, 0.0), Vec3::new(0.84, 0.08, 0.84), q, lin(STONE_D));
            for (mx, mz) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0), (0.0, 1.0), (0.0, -1.0), (1.0, 0.0), (-1.0, 0.0)] {
                cube(out, t + Vec3::new(mx * 0.33, top + 0.19, mz * 0.33), Vec3::new(0.16, 0.22, 0.16), q, stone(hash(mx as f64, mz as f64 + side as f64)));
            }
            for (k, w) in [0.5f32, 0.4, 0.3, 0.2, 0.11].iter().enumerate() {
                cube(out, t + Vec3::new(0.0, top + 0.15 + k as f32 * 0.16, 0.0), Vec3::new(*w, 0.16, *w), Quat::from_rotation_y(0.785), lin(SLATE));
            }
            cube(out, t + Vec3::new(0.0, top + 1.0, 0.0), Vec3::new(0.05, 0.12, 0.05), q, lin(GOLD));
            // Arrow slits.
            for y in [0.9f32, 1.6] {
                cube(out, t + Vec3::new(0.0, y, 0.365), Vec3::new(0.05, 0.22, 0.01), q, lin(VOID));
            }
            // A brazier before each tower.
            let b = o + Vec3::new(side * 0.98, 0.0, 0.62);
            cube(out, b + Vec3::new(0.0, 0.25, 0.0), Vec3::new(0.14, 0.5, 0.14), q, stone(0.4));
            cube(out, b + Vec3::new(0.0, 0.54, 0.0), Vec3::new(0.3, 0.08, 0.3), q, lin(IRON));
            cube(out, b + Vec3::new(0.0, 0.6, 0.0), Vec3::new(0.24, 0.05, 0.24), q, lin(EMBER) * 0.6);
            self.smoke.push((b + Vec3::new(0.0, 0.75, 0.0), 1.0));
        }
        // The arch: piers, a pointed ring of voussoirs, the wall above it, a cornice and merlons.
        let out = &mut self.static_cubes;
        for side in [-1.0f32, 1.0] {
            cube(out, o + Vec3::new(side * 0.53, 0.66, 0.0), Vec3::new(0.2, 1.32, 0.34), q, stone(0.2));
        }
        for side in [-1.0f32, 1.0] {
            for k in 0..6 {
                let t = k as f32 / 5.0;
                // A pointed (two-centred) arch: each side an arc about the far springing point.
                let a = t * 0.95;
                let p = Vec3::new(side * (0.8 * a.cos() - 0.4), 1.32 + 0.8 * a.sin(), 0.02);
                cube(out, o + p, Vec3::new(0.14, 0.12, 0.36), Quat::from_rotation_z(-side * a), stone(hash(k as f64, side as f64)));
            }
        }
        cube(out, o + Vec3::new(0.0, 2.08, 0.02), Vec3::new(0.16, 0.18, 0.37), q, lin(GOLD)); // keystone
        for (x0, x1, y0, y1) in [(-0.62f32, -0.22f32, 1.32f32, 1.9f32), (0.22, 0.62, 1.32, 1.9), (-0.62, 0.62, 1.95, 2.3)] {
            cube(out, o + Vec3::new((x0 + x1) / 2.0, (y0 + y1) / 2.0, -0.02), Vec3::new(x1 - x0, y1 - y0, 0.3), q, stone(0.6));
        }
        // An oak tympanum fills the arch above the doors.
        for k in 0..4 {
            let w = 0.78 - k as f32 * 0.18;
            cube(out, o + Vec3::new(0.0, 1.37 + k as f32 * 0.1, -0.02), Vec3::new(w, 0.1, 0.06), q, lin(OAK[k % 3]));
        }
        cube(out, o + Vec3::new(0.0, 2.36, 0.0), Vec3::new(1.3, 0.08, 0.42), q, lin(STONE_D));
        for k in 0..5 {
            cube(out, o + Vec3::new(-0.52 + k as f32 * 0.26, 2.52, 0.0), Vec3::new(0.14, 0.24, 0.3), q, stone(hash(k as f64, 9.0)));
        }
        // Dark sockets for the lock lamps.
        for i in 0..8 {
            cube(out, o + lock_spot(i, 8) - Vec3::new(0.0, 0.0, 0.03), Vec3::new(0.1, 0.1, 0.04), q, lin(IRON));
        }
    }

    /// Doors, lock lamps, braziers, the beacon and the glow; and the golden keys still lying about.
    pub(super) fn emit_gate_and_keys(&self, s: &SceneState, time: f32, hero: Vec3, f: &mut Frame) {
        let cube_i = MeshKind::Cube as usize;
        // ---- keys: spinning, bobbing, haloed, a faint beam to find them by
        let mut lit: Vec<Vec3> = vec![];
        for p in &self.keys {
            if s.grid.get(p.x, p.y) != Tile::Nodule {
                continue;
            }
            let base = Vec3::new(p.x as f32, floor_at(s.grid, p.x as f32, p.y as f32), p.y as f32);
            let bob = (time * 2.0 + p.x as f32).sin() * 0.06;
            let at = base + Vec3::new(0.0, 0.34 + bob, 0.0);
            let spin = Quat::from_rotation_y(time * 1.8 + p.y as f32) * Quat::from_rotation_z(0.25);
            let m = &self.models.key;
            for (off, c) in &m.cubes {
                let pm = Mat4::from_scale_rotation_translation(Vec3::splat(m.size), spin, at + spin * *off - Vec3::new(0.0, 0.33, 0.0));
                f.opaque[cube_i].push(Instance::new(pm, *c, 1.0, lin(GOLD) * 0.35, 13.0));
            }
            let halo = Mat4::from_translation(at) * Mat4::from_scale(Vec3::splat(0.36 + (time * 3.0).sin() * 0.03));
            f.additive.push(Instance::new(halo, lin(GOLD), 0.12, Vec3::ZERO, SHADE_UNLIT));
            let beam = Mat4::from_translation(base + Vec3::new(0.0, 3.0, 0.0)) * Mat4::from_scale(Vec3::new(0.07, 6.0, 0.07));
            f.additive.push(Instance::new(beam, lin(GOLD), 0.1 + (time * 2.0 + p.x as f32).sin() * 0.03, Vec3::ZERO, SHADE_UNLIT));
            for k in 0..4 {
                let a = time * 1.5 + k as f32 * TAU / 4.0;
                let sp = at + Vec3::new(a.cos() * 0.3, (a * 1.7).sin() * 0.15, a.sin() * 0.3);
                f.additive.push(Instance::new(Mat4::from_translation(sp) * Mat4::from_scale(Vec3::splat(0.035)), lin(0xfff0a0), 0.8, Vec3::ZERO, SHADE_UNLIT));
            }
            lit.push(at);
        }
        for p in nearest(&lit, hero, 3) {
            f.lights.push(Light { pos: p + Vec3::new(0.0, 0.3, 0.0), color: lin(0xffd060) * 3.5, distance: 3.5 });
        }

        // ---- the gate
        let Some(g) = s.gate else { return };
        let floor = s.grid.elevation(g.tile.x, g.tile.y + 1) as f32 * STEP;
        let o = Vec3::new(g.tile.x as f32, floor, g.tile.y as f32);
        let open = g.keys >= g.total && g.total > 0;
        // Doors: planks, iron straps and studs, a ring pull; they swing in, away from you.
        let swing = g.door.clamp(0.0, 1.0);
        let angle = (swing * swing * (3.0 - 2.0 * swing)) * 1.75;
        for side in [-1.0f32, 1.0] {
            let hinge = o + Vec3::new(side * 0.42, 0.0, 0.1);
            let rot = Quat::from_rotation_y(-side * angle);
            let leaf = |f: &mut Frame, local: Vec3, scale: Vec3, color: Vec3| {
                let p = hinge + rot * Vec3::new(-side * local.x, local.y, local.z);
                f.opaque[cube_i].push(Instance::new(Mat4::from_scale_rotation_translation(scale, rot, p), color, 1.0, Vec3::ZERO, SHADE_LIT));
            };
            for k in 0..3 {
                leaf(f, Vec3::new(0.07 + k as f32 * 0.14, 0.66, 0.0), Vec3::new(0.135, 1.32, 0.06), lin(OAK[k]));
            }
            for y in [0.25f32, 1.05] {
                leaf(f, Vec3::new(0.21, y, 0.035), Vec3::new(0.42, 0.06, 0.02), lin(IRON));
                for k in 0..3 {
                    leaf(f, Vec3::new(0.07 + k as f32 * 0.14, y, 0.05), Vec3::splat(0.03), lin(0x4a4c52));
                }
            }
            leaf(f, Vec3::new(0.36, 0.66, 0.06), Vec3::new(0.06, 0.08, 0.02), lin(GOLD));
        }
        // Lock lamps: one lit for every key found.
        for i in 0..g.total.min(8) {
            let lit = i < g.keys;
            let p = o + lock_spot(i, g.total.min(8));
            let pulse = if open { 0.75 + (time * 4.0 + i as f32).sin() * 0.25 } else { 1.0 };
            let (c, e) = if lit { (lin(GOLD), lin(0xffc040) * 2.2 * pulse) } else { (lin(0x3a3830), Vec3::ZERO) };
            f.opaque[cube_i].push(Instance::new(Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.075)), c, 1.0, e, SHADE_LIT));
            if lit {
                f.additive.push(Instance::new(Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.16)), lin(0xffc040), 0.18 * pulse, Vec3::ZERO, SHADE_UNLIT));
            }
        }
        // Brazier flames.
        for side in [-1.0f32, 1.0] {
            let b = o + Vec3::new(side * 0.98, 0.64, 0.62);
            for k in 0..6 {
                let fk = k as f32 + side * 7.0;
                let t = (time * 2.2 + fk * 0.37).fract();
                let p = b + Vec3::new((fk * 1.9).sin() * 0.07, t * 0.35, (fk * 2.7).cos() * 0.07);
                let size = 0.1 * (1.0 - t) + 0.02;
                let color = lin(if t < 0.4 { 0xffd060 } else { 0xff6a20 });
                f.additive.push(Instance::new(Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(size)), color, 0.8 * (1.0 - t), Vec3::ZERO, SHADE_UNLIT));
            }
            let flick = 3.2 + (time * 11.0 + side).sin() * 0.4 + (time * 23.0).sin() * 0.2;
            f.lights.push(Light { pos: b + Vec3::new(0.0, 0.35, 0.25), color: lin(0xff9a40) * flick * 0.7, distance: 4.0 });
        }
        // Unlocked: a beacon over the gate, seen from across the estate; and the glow through the doors.
        if open {
            let beam = Mat4::from_translation(o + Vec3::new(0.0, 7.0, -0.1)) * Mat4::from_scale(Vec3::new(0.45, 14.0, 0.45));
            f.additive.push(Instance::new(beam, lin(GOLD), 0.1 + (time * 1.5).sin() * 0.03, Vec3::ZERO, SHADE_UNLIT));
            f.lights.push(Light { pos: o + Vec3::new(0.0, 2.9, 0.4), color: lin(0xffc860) * 1.6, distance: 5.0 });
        }
        if swing > 0.0 {
            let glow = Mat4::from_translation(o + Vec3::new(0.0, 0.8, -0.36)) * Mat4::from_scale(Vec3::new(0.84, 1.6, 0.02));
            f.additive.push(Instance::new(glow, lin(0xffd070), swing, Vec3::ZERO, SHADE_UNLIT));
            f.lights.push(Light { pos: o + Vec3::new(0.0, 0.9, 0.2), color: lin(0xffd070) * 8.0 * swing, distance: 6.0 });
        }
    }

    /// Smoke curling up from braziers and candles: puffs that rise, swell, drift and thin away.
    pub(super) fn emit_smoke(&self, time: f32, f: &mut Frame) {
        let sphere = MeshKind::Sphere as usize;
        for (i, &(base, strength)) in self.smoke.iter().enumerate() {
            let n = (8.0 * strength).ceil() as usize;
            for k in 0..n {
                let fk = k as f32 + i as f32 * 3.1;
                let t = (time * (0.18 + 0.06 * strength) + k as f32 / n as f32).fract();
                let drift = Vec3::new(0.35 + (fk * 1.3).sin() * 0.1, 0.0, 0.12);
                let p = base + drift * t * (1.0 + strength) + Vec3::new((time * 0.8 + fk).sin() * 0.06 * t, t * (0.9 + 1.4 * strength), (time * 0.6 + fk * 1.7).cos() * 0.06 * t);
                let r = (0.04 + t * 0.2) * strength.sqrt();
                let a = 0.16 * (PI * t).sin().powf(0.8) * strength.sqrt();
                let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(r));
                f.transparent[sphere].push(Instance::new(m, lin(SMOKE), a, Vec3::ZERO, SHADE_LIT));
            }
        }
    }
}
