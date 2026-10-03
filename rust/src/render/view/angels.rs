//! The archangels in the world. Their shrines: an octagonal marble plinth
//! banded in gold, a little marble angel on it with wings half raised,
//! holding Michael's sword, Gabriel's horn, Raphael's staff or Uriel's flame,
//! a halo over it, and the archangel's light baked into the place: the
//! marble glowing faintly in their colour, a soft shaft falling on the statue
//! and a pool of that light on the floor round it, steady as a lamp (all gone
//! grey and dark when that archangel has given this level, or all four
//! have). And their powers:
//! Gabriel's thunderbolts, Raphael's beam and Uriel's wave. Michael's fire
//! flies as a shot (`emit_forged_shot`); the foes it turns glow blue and
//! wear a halo (`update_enemy`, the enemies' emitter).

use super::*;
use crate::logic::angels::{Archangel, WAVE_RADIUS, WAVE_SPEED};
use crate::logic::items::{Diamond, Offer};
use crate::logic::types::V;
use crate::render::carve::{ball, limb, v};
use crate::render::characters::{mat, Sculpt};
use crate::render::voxels::Collected;

/// The shrine's voxels: it stands about 0.9 tall on its tile.
pub(super) const SHRINE_VOXEL: f32 = 0.0175;
const SHRINE_SIZE: (i32, i32, i32) = (32, 52, 28);
/// Where the statue's head is in its grid, for the halo.
const HEAD: [f32; 3] = [15.5, 41.0, 13.8];

const THUNDER_SECONDS: f32 = 0.5;
const BEAM_SECONDS: f32 = 0.75;

/// A thunderbolt out of the sky onto a point.
pub(super) struct Bolt {
    at: Vec3,
    life: f32,
    seed: f32,
    color: Vec3,
}

/// Raphael's beam, from the hero to where it ends.
pub(super) struct Beam {
    a: Vec3,
    b: Vec3,
    life: f32,
    color: Vec3,
}

/// Uriel's wave: where it rolls out from, seconds since, its colour.
pub(super) struct Wave {
    at: Vec3,
    t: f32,
    color: Vec3,
}

mod stone {
    use super::mat;
    pub const MARBLE: u32 = 0xf0ebe0 | mat::IVORY;
    pub const MARBLE_S: u32 = 0xd6cebe | mat::IVORY;
    pub const MARBLE_D: u32 = 0xb2a994 | mat::IVORY;
    pub const GOLD: u32 = 0xe8b84a | mat::GOLD;
    pub const STEEL: u32 = 0xc8ccd4 | mat::METAL;
    pub const WOOD: u32 = 0x7a5230 | mat::WOOD;
    pub const FLAME_Y: u32 = 0xffe070;
    pub const FLAME_O: u32 = 0xff9a2a;
    pub const FLAME_R: u32 = 0xff4a1a;
}

/**
 * A shrine of `angel`: the plinth and the statue on it, holding the
 * archangel's own sign. Faces +z; the halo and the light are drawn apart.
 */
pub fn angel_shrine(angel: Archangel) -> Collected {
    use stone::*;
    let (w, h, d) = SHRINE_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (15.5f32, 13.5f32);
    // ---- the plinth: octagonal steps, a gold band
    let oct = |x: i32, z: i32, r: f32| {
        let (dx, dz) = ((x as f32 - cx).abs(), (z as f32 - cz).abs());
        dx <= r && dz <= r && dx + dz <= r * 1.42
    };
    for y in 0..=9 {
        let (r, c) = match y {
            0..=1 => (13.5, MARBLE_D),
            2 => (12.5, GOLD),
            3..=7 => (10.8, MARBLE),
            8 => (12.2, MARBLE_S),
            _ => (11.6, MARBLE),
        };
        for z in 0..d {
            for x in 0..w {
                if oct(x, z, r) {
                    s.set(x, y, z, c);
                }
            }
        }
    }
    // A gold roundel on its face.
    for y in 3..=7 {
        for x in 13..=18 {
            let r = ((x as f32 - cx).powi(2) + (y as f32 - 5.0).powi(2)).sqrt();
            if r < 2.6 {
                if let Some(z) = (0..d).rev().find(|&z| s.get(x, y, z).is_some()) {
                    s.set(x, y, z + 1, if r < 1.3 { MARBLE } else { GOLD });
                }
            }
        }
    }

    // ---- the statue: a robe falling in folds, a gold girdle, the body, the head
    let y0 = 10.0f32;
    for k in 0..=19 {
        let y = y0 + k as f32;
        let t = k as f32 / 19.0;
        let (rx, rz) = (5.4 - t * 2.0, 4.2 - t * 1.6);
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = ((x as f32 - cx) / rx, (z as f32 - cz) / rz);
                if dx * dx + dz * dz <= 1.0 {
                    let fold = ((z as f32 - cz).atan2(x as f32 - cx) * 7.0).sin();
                    s.set(x, y as i32, z, if fold > 0.55 { MARBLE_S } else { MARBLE });
                }
            }
        }
    }
    for z in 0..d {
        for x in 0..w {
            let (dx, dz) = ((x as f32 - cx) / 3.6, (z as f32 - cz) / 2.8);
            if dx * dx + dz * dz <= 1.0 {
                s.set(x, (y0 + 19.0) as i32, z, GOLD);
            }
        }
    }
    for k in 0..7 {
        let y = y0 + 20.0 + k as f32;
        let chest = (k as f32 / 6.0 * PI).sin();
        ball(&mut s, v(cx, y, cz + 0.2 * chest), v(3.5 + chest * 0.4, 0.8, 2.4 + chest * 0.3), MARBLE);
    }
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 3.2, y0 + 26.0, cz), Vec3::splat(1.6), MARBLE);
    }
    limb(&mut s, &[(v(cx, y0 + 26.5, cz), 1.2), (v(cx, y0 + 28.5, cz + 0.3), 1.1)], MARBLE);
    ball(&mut s, v(HEAD[0], HEAD[1], HEAD[2]), v(2.5, 2.8, 2.5), MARBLE);
    // Curls of hair over the crown and down the back of the neck; a nose.
    s.paint(|x, y, z, c| {
        let (dx, dy, dz) = (x as f32 - HEAD[0], y as f32 - HEAD[1], z as f32 - HEAD[2]);
        let on_head = dx * dx / 7.0 + dy * dy / 8.5 + dz * dz / 7.0 <= 1.2;
        (c == MARBLE && on_head && (dy > 0.8 || dz < -0.8) && (x + y + z) % 2 == 0).then_some(MARBLE_S)
    });
    s.set(16, HEAD[1] as i32, (HEAD[2] + 2.6) as i32, MARBLE_S);

    // ---- wings, half raised behind: feathers in rows along each
    for side in [-1.0f32, 1.0] {
        let root = (cx + side * 1.8, y0 + 25.5);
        let tip = (cx + side * 11.0, y0 + 37.5);
        let low = (cx + side * 6.5, y0 + 9.5);
        for y in (y0 as i32 + 8)..(y0 as i32 + 39) {
            for x in 0..w {
                let (px, py) = (x as f32, y as f32);
                // Inside the triangle root-tip-low, rounded at the tip.
                let edge = |a: (f32, f32), b: (f32, f32)| (b.0 - a.0) * (py - a.1) - (b.1 - a.1) * (px - a.0);
                let (e1, e2, e3) = (edge(root, tip), edge(tip, low), edge(low, root));
                let inside = (e1 >= 0.0 && e2 >= 0.0 && e3 >= 0.0) || (e1 <= 0.0 && e2 <= 0.0 && e3 <= 0.0);
                if !inside {
                    continue;
                }
                let out = (px - cx).abs();
                let z = (cz - 3.0 - out * 0.22).round() as i32;
                let row = ((py - y0) * 0.5 + out * 0.3) as i32;
                let c = if row % 3 == 0 {
                    MARBLE_S
                } else if out > 8.0 && (x % 2 == 0) {
                    MARBLE_D
                } else {
                    MARBLE
                };
                s.set(x, y, z, c);
                s.set(x, y, z - 1, MARBLE_S);
            }
        }
    }

    // ---- arms, and each archangel's sign in their hands
    let arm = |s: &mut Sculpt, side: f32, elbow: Vec3, hand: Vec3| {
        limb(s, &[(v(cx + side * 3.3, y0 + 25.5, cz), 1.3), (elbow, 1.1), (hand, 0.9)], MARBLE);
        ball(s, hand, Vec3::splat(1.1), MARBLE);
    };
    match angel {
        Archangel::Michael => {
            // Both hands on the hilt of a sword held up before him.
            let hilt = v(cx, y0 + 21.0, cz + 4.0);
            for side in [-1.0f32, 1.0] {
                arm(&mut s, side, v(cx + side * 3.6, y0 + 21.5, cz + 2.0), v(cx + side * 0.9, hilt.y, hilt.z));
            }
            limb(&mut s, &[(v(cx, y0 + 18.0, cz + 4.0), 0.7), (v(cx, y0 + 22.5, cz + 4.0), 0.7)], WOOD);
            limb(&mut s, &[(v(cx - 2.8, y0 + 23.0, cz + 4.0), 0.6), (v(cx + 2.8, y0 + 23.0, cz + 4.0), 0.6)], GOLD);
            for y in (y0 as i32 + 24)..=(y0 as i32 + 40) {
                let half = if y > y0 as i32 + 37 { 0 } else { 1 };
                for x in (cx as i32 - half)..=(cx as i32 + half) {
                    s.set(x, y, (cz + 4.0) as i32, STEEL);
                }
            }
            ball(&mut s, v(cx, y0 + 17.6, cz + 4.0), Vec3::splat(0.9), GOLD);
        }
        Archangel::Gabriel => {
            // A long horn raised to the lips, its bell flaring forward.
            let (mouth, bell) = (v(cx + 0.5, HEAD[1] - 1.2, cz + 3.0), v(cx + 2.5, HEAD[1] + 4.0, cz + 11.0));
            arm(&mut s, 1.0, v(cx + 4.0, y0 + 23.0, cz + 2.5), v(cx + 1.6, HEAD[1] - 0.5, cz + 6.0));
            arm(&mut s, -1.0, v(cx - 3.6, y0 + 23.0, cz + 2.5), v(cx + 1.0, HEAD[1] - 2.0, cz + 4.5));
            limb(&mut s, &[(mouth, 0.6), (mouth.lerp(bell, 0.6), 0.7), (bell, 1.2)], GOLD);
            let dir = (bell - mouth).normalize();
            for k in 0..4 {
                ball(&mut s, bell + dir * k as f32 * 0.6, Vec3::splat(1.3 + k as f32 * 0.45), GOLD);
            }
        }
        Archangel::Raphael => {
            // A pilgrim's staff in the right hand, gold at its head; the left hand raised in blessing.
            let staff_x = cx + 5.2;
            arm(&mut s, 1.0, v(cx + 4.8, y0 + 22.0, cz + 1.8), v(staff_x, y0 + 23.0, cz + 2.4));
            arm(&mut s, -1.0, v(cx - 4.2, y0 + 23.5, cz + 1.8), v(cx - 3.6, y0 + 28.0, cz + 3.0));
            limb(&mut s, &[(v(staff_x, y0, cz + 2.4), 0.6), (v(staff_x, y0 + 37.0, cz + 2.4), 0.6)], WOOD);
            ball(&mut s, v(staff_x, y0 + 38.0, cz + 2.4), Vec3::splat(1.2), GOLD);
            limb(&mut s, &[(v(staff_x, y0 + 36.0, cz + 2.4), 0.4), (v(staff_x + 1.4, y0 + 34.0, cz + 2.4), 0.4)], GOLD);
        }
        Archangel::Uriel => {
            // A flame burning on the palm of the raised right hand; the left hand at his heart.
            let palm = v(cx + 4.2, y0 + 33.0, cz + 2.4);
            arm(&mut s, 1.0, v(cx + 5.2, y0 + 27.0, cz + 1.4), palm);
            arm(&mut s, -1.0, v(cx - 3.8, y0 + 21.5, cz + 2.0), v(cx - 1.0, y0 + 24.0, cz + 3.0));
            for (k, c) in [(0, FLAME_R), (1, FLAME_O), (2, FLAME_O), (3, FLAME_Y), (4, FLAME_Y)] {
                let r = 1.6 - k as f32 * 0.28;
                ball(&mut s, palm + v(0.0, 1.4 + k as f32 * 1.1, 0.0), Vec3::splat(r), c);
            }
        }
    }
    s.finish(0.03)
}

impl WorldView {
    /// Gabriel's thunderbolt onto `at` (a tile position).
    pub fn thunder(&mut self, grid: &Grid, at: V, color: u32) {
        let p = Vec3::new(at.x as f32, floor_at(grid, at.x as f32, at.y as f32), at.y as f32);
        let seed = self.rng.nextf() * 100.0;
        self.bolts.push(Bolt { at: p, life: THUNDER_SECONDS, seed, color: lin(color) });
        self.sparkle(grid, at.x, at.y, 0.9, color, 16);
        self.dust(grid, at.x, at.y, 0x3a4a3a, 10, 2.8);
    }

    /// Raphael's beam from `from` to `to` (tile positions), at chest height.
    pub fn beam(&mut self, grid: &Grid, from: V, to: V, color: u32) {
        let lift = |p: V| Vec3::new(p.x as f32, floor_at(grid, p.x as f32, p.y as f32) + 0.55, p.y as f32);
        self.beams.push(Beam { a: lift(from), b: lift(to), life: BEAM_SECONDS, color: lin(color) });
        self.sparkle(grid, to.x, to.y, 1.0, color, 24);
    }

    /// Uriel's wave rolling out from `at`.
    pub fn holy_wave(&mut self, grid: &Grid, at: V, color: u32) {
        let p = Vec3::new(at.x as f32, floor_at(grid, at.x as f32, at.y as f32) + 0.05, at.y as f32);
        self.waves.push(Wave { at: p, t: 0.0, color: lin(color) });
    }

    pub(super) fn update_angel_fx(&mut self, dt: f32) {
        self.bolts.retain_mut(|b| {
            b.life -= dt;
            b.life > 0.0
        });
        self.beams.retain_mut(|b| {
            b.life -= dt;
            b.life > 0.0
        });
        let end = (WAVE_RADIUS / WAVE_SPEED) as f32 + 0.3;
        self.waves.retain_mut(|w| {
            w.t += dt;
            w.t < end
        });
    }

    pub(super) fn emit_angel_fx(&self, time: f32, f: &mut Frame) {
        for b in &self.bolts {
            let k = b.life / THUNDER_SECONDS;
            let flick = (time * 40.0).floor();
            let top = b.at + Vec3::new(1.2, 9.0, 0.6);
            let mut prev = top;
            let steps = 10;
            for i in 1..=steps {
                let t = i as f32 / steps as f32;
                let mut p = top.lerp(b.at, t);
                if i < steps {
                    let h = |n: f32| ((flick + b.seed) * 12.9898 + i as f32 * 4.7 + n * 78.233).sin().fract() - 0.5;
                    p += Vec3::new(h(1.0), 0.0, h(3.0)) * 0.9;
                }
                f.additive.push(Instance::new(
                    forge::segment(prev, p, 0.06),
                    Vec3::ONE.lerp(b.color, 0.45) * 1.6,
                    k,
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
                f.additive.push(Instance::new(forge::segment(prev, p, 0.22), b.color, 0.3 * k, Vec3::ZERO, SHADE_UNLIT));
                prev = p;
            }
            glow_orb(f, b.at + Vec3::new(0.0, 0.3, 0.0), 0.9, b.color, 0.35 * k);
            f.lights.push(Light { pos: b.at + Vec3::new(0.0, 1.0, 0.0), color: b.color * 5.0 * k, distance: 4.0 });
        }
        for b in &self.beams {
            let k = b.life / BEAM_SECONDS;
            let thick = 0.35 * k.sqrt();
            f.additive.push(Instance::new(
                forge::segment(b.a, b.b, thick * 0.45),
                Vec3::ONE.lerp(b.color, 0.3) * 2.6,
                k,
                Vec3::ZERO,
                SHADE_UNLIT,
            ));
            f.additive.push(Instance::new(forge::segment(b.a, b.b, thick * 1.6), b.color, 0.45 * k, Vec3::ZERO, SHADE_UNLIT));
            let len = (b.b - b.a).length();
            let n = (len / 2.5).ceil().max(1.0) as i32;
            for i in 0..=n {
                let p = b.a.lerp(b.b, i as f32 / n as f32);
                f.lights.push(Light { pos: p, color: b.color * 6.0 * k, distance: 3.2 });
            }
            glow_orb(f, b.a, 0.9, b.color, 0.7 * k);
            glow_orb(f, b.b, 1.2, b.color, 0.7 * k);
        }
        for w in &self.waves {
            let r = (w.t * WAVE_SPEED as f32).min(WAVE_RADIUS as f32);
            let fade = (1.0 - r / WAVE_RADIUS as f32).clamp(0.0, 1.0).sqrt();
            // A wall of red light racing outward, and a ring on the ground under it.
            let posts = 72;
            for i in 0..posts {
                let a = i as f32 / posts as f32 * TAU;
                let base = w.at + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
                let tall = 0.9 + ((a * 5.0 + time * 9.0).sin() * 0.5 + 0.5) * 0.6;
                f.additive.push(Instance::new(
                    forge::segment(base, base + Vec3::new(0.0, tall, 0.0), 0.05 + r * 0.006),
                    w.color * 1.1,
                    0.5 * fade,
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
            let m =
                Mat4::from_translation(w.at) * Mat4::from_rotation_x(-PI / 2.0) * Mat4::from_scale(Vec3::new(r.max(0.1), r.max(0.1), 1.0));
            f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, w.color * 1.5, 0.5 * fade, Vec3::ZERO, SHADE_UNLIT));
            for i in 0..6 {
                let a = i as f32 / 6.0 * TAU;
                f.lights.push(Light { pos: w.at + Vec3::new(a.cos() * r, 0.8, a.sin() * r), color: w.color * 4.0 * fade, distance: 4.0 });
            }
        }
    }

    /**
     * An archangel's shrine: the plinth and statue of whoever it honours now
     * (turned to face a path beside it), the halo, and their light baked into
     * the spot: a faint glow in the marble, a shaft from above, a pool on the
     * floor. It does not flicker. Spent, it stands grey and dark. Returns
     * where it stands and its colour, for the light pool and the label.
     */
    pub(super) fn emit_angel_shrine(&self, d: &Diamond, grid: &Grid, spent: &[bool; 4], f: &mut Frame) -> Option<(Vec3, Vec3)> {
        let Offer::Angel(angel) = d.current() else { return None };
        let (x, y) = (d.tile.x, d.tile.y);
        let base = Vec3::new(x as f32, floor_at(grid, x as f32, y as f32), y as f32);
        let lit = d.active && !spent[angel.index()];
        let color = if lit { lin(angel.color()) } else { lin(0x6a6e76) };
        // Face the first open path round it, the south first.
        let yaw = [(0, 1, 0.0f32), (1, 0, PI / 2.0), (-1, 0, -PI / 2.0), (0, -1, PI)]
            .iter()
            .find(|&&(dx, dy, _)| grid.is_walkable(x + dx, y + dy, false))
            .map(|&(_, _, a)| a)
            .unwrap_or(0.0);
        let rig = &self.models.angel_shrines[angel.index()];
        let root = Mat4::from_translation(base) * Mat4::from_rotation_y(yaw);
        let tint = if lit { Vec3::ONE } else { Vec3::splat(0.62) };
        // Lit, the marble itself holds a little of their light.
        let inner = if lit { color * 0.1 } else { Vec3::ZERO };
        emit_rig(&mut f.rigs, rig, &Pose::rest(rig), root, tint, 1.0, inner);
        // The halo over the statue's head.
        let head = root.transform_point3(Vec3::new(0.0, (HEAD[1] + 3.8) * SHRINE_VOXEL, (HEAD[2] - 13.5 - 0.6) * SHRINE_VOXEL));
        let glow = if lit { 1.0 } else { 0.15 };
        let m = Mat4::from_translation(head) * Mat4::from_rotation_x(-PI / 2.0 + 0.35) * Mat4::from_scale(Vec3::new(0.075, 0.075, 1.0));
        f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, color * 2.5 * glow, 0.85, Vec3::ZERO, SHADE_UNLIT));
        if lit {
            // A pool of their light on the floor, brightest at the plinth's foot ...
            let pool = |r: f32| {
                Mat4::from_translation(base + Vec3::new(0.0, 0.03, 0.0))
                    * Mat4::from_rotation_x(-PI / 2.0)
                    * Mat4::from_scale(Vec3::splat(r))
            };
            f.glow.push(Instance::new(pool(1.9), color, 0.45, Vec3::ZERO, SHADE_GLOW));
            f.glow.push(Instance::new(pool(0.9), Vec3::ONE.lerp(color, 0.6), 0.35, Vec3::ZERO, SHADE_GLOW));
            // ... and a soft shaft of it about the statue, fading as it rises.
            for k in 0..7 {
                let fade = 1.0 - k as f32 / 7.0;
                glow_orb(f, base + Vec3::new(0.0, 0.25 + k as f32 * 0.38, 0.0), 0.62, color, 0.16 * fade * fade);
            }
        }
        if lit {
            glow_orb(f, head, 0.35, color, 0.35);
        }
        // Lit, it joins the pool of shrine lights (see `emit`).
        lit.then_some((base, color))
    }
}
