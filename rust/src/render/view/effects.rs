//! Effects: sparks, dust and sparkles, bomb blasts and the debris of broken walls, the lights
//! they cast, and the screen shake.

use super::*;

const BLAST_SECONDS: f32 = 0.45;

const DEBRIS_SECONDS: f32 = 1.4;
/// The most flying chunks at once (RP04): a chain of blasts in a 30-foe fight retires the oldest
/// first rather than piling up.
pub const MAX_DEBRIS: usize = 160;

pub(super) const GRAVITY: f32 = 14.0;

/// Particles alive at once; the oldest make way.
const MAX_PARTICLES: usize = 700;

/// Screen shake: trauma decays at this rate per second; offset scales with trauma squared.
const TRAUMA_DECAY: f32 = 1.9;

const SHAKE_DISTANCE: f32 = 0.13;

/// One speck of dust or spark.
pub(super) struct Particle {
    pub(super) pos: Vec3,
    pub(super) vel: Vec3,
    pub(super) color: Vec3,
    pub(super) life: f32,
    pub(super) max: f32,
    pub(super) size: f32,
    pub(super) gravity: f32,
    pub(super) floor: f32,
    pub(super) glow: bool,
}

pub(super) struct Debris {
    pub(super) pos: Vec3,
    pub(super) vel: Vec3,
    pub(super) rot: Vec3,
    pub(super) spin: Vec3,
    pub(super) life: f32,
    pub(super) floor_y: f32,
    pub(super) color: Vec3,
}

pub(super) struct Blast {
    pub(super) id: u64,
    pub(super) light: usize,
    pub(super) pos: Vec3,
    pub(super) color: Vec3,
    pub(super) life: f32,
    pub(super) seconds: f32,
    pub(super) intensity: f32,
    pub(super) scale: f32,
    pub(super) radius: f32,
}

#[derive(Clone, Copy, Default)]
pub(super) struct EffectLight {
    pub(super) pos: Vec3,
    pub(super) color: Vec3,
    pub(super) intensity: f32,
    pub(super) distance: f32,
    pub(super) owner: u64,
}

impl WorldView {
    /// Small light pop where a shot lands or connects.
    pub fn spark(&mut self, grid: &Grid, x: f64, y: f64, color: u32) {
        self.burst(grid, x as f32, y as f32, color, 14.0, 3.0, 0.18, 0.6, 0.12);
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn burst(
        &mut self,
        grid: &Grid,
        x: f32,
        y: f32,
        color: u32,
        intensity: f32,
        radius: f32,
        seconds: f32,
        scale: f32,
        start_radius: f32,
    ) {
        let h = floor_at(grid, x, y);
        let li = self.effect_cursor % LIGHT_POOL_EFFECTS;
        self.effect_cursor += 1;
        let id = self.next_blast_id;
        self.next_blast_id += 1;
        self.effect_lights[li] = EffectLight { pos: Vec3::new(x, h + 0.8, y), color: lin(color), intensity, distance: radius, owner: id };
        self.blasts.push(Blast {
            id,
            light: li,
            pos: Vec3::new(x, h + 0.5, y),
            color: lin(color),
            life: seconds,
            seconds,
            intensity,
            scale,
            radius: start_radius,
        });
    }

    /// Flash, shockwave sphere, and flying chunks at a blast site.
    pub fn explode(&mut self, grid: &Grid, x: f64, y: f64, broken_tiles: usize) {
        let (x, y) = (x as f32, y as f32);
        self.burst(grid, x, y, 0xffb060, 120.0, 9.0, BLAST_SECONDS, 3.5, 0.5);
        let h = floor_at(grid, x, y);
        let count = 10 + broken_tiles * 4;
        for _ in 0..count {
            let r = &mut self.rng;
            let shade = r.nextf();
            // Torn undergrowth: leaves and clods of earth.
            let color = lin(if shade < 0.6 { STONE_A } else { DIRT });
            let pos = Vec3::new(x + (r.nextf() - 0.5) * 0.6, h + 0.4 + r.nextf() * 0.5, y + (r.nextf() - 0.5) * 0.6);
            let a = r.nextf() * TAU;
            let s = 2.0 + r.nextf() * 3.0;
            let vel = Vec3::new(a.cos() * s, 4.0 + r.nextf() * 5.0, a.sin() * s);
            let spin = Vec3::new(r.nextf() * 8.0, r.nextf() * 8.0, r.nextf() * 8.0);
            let life = DEBRIS_SECONDS * (0.7 + r.nextf() * 0.5);
            if self.debris.len() >= MAX_DEBRIS {
                // The one nearest its end goes (the list is kept in no order; this is rare).
                if let Some(i) = self.debris.iter().enumerate().min_by(|a, b| a.1.life.total_cmp(&b.1.life)).map(|(i, _)| i) {
                    self.debris.swap_remove(i);
                }
            }
            self.debris.push(Debris { pos, vel, rot: Vec3::ZERO, spin, life, floor_y: h + 0.08, color });
        }
    }

    /// Trauma-based shake: a few incommensurate sines, so it never loops visibly.
    pub(super) fn update_shake(&mut self, dt: f32) {
        self.trauma = (self.trauma - TRAUMA_DECAY * dt).max(0.0);
        self.shake_clock += dt;
        let t = self.shake_clock;
        let s = self.trauma * self.trauma * SHAKE_DISTANCE;
        self.shake_offset = Vec3::new(
            (t * 47.0).sin() * 0.6 + (t * 83.0).sin() * 0.4,
            ((t * 59.0).sin() * 0.6 + (t * 97.0).cos() * 0.4) * 0.7,
            (t * 53.0).cos() * 0.6 + (t * 71.0).sin() * 0.4,
        ) * s;
    }

    /// Adds impact to the screen shake (0.05 a tap, 0.7 a bomb at your feet).
    pub fn shake(&mut self, amount: f32) {
        self.trauma = (self.trauma + amount).min(1.0);
    }

    pub(super) fn push_particle(&mut self, p: Particle) {
        if self.particles.len() >= MAX_PARTICLES {
            self.particles.remove(0);
        }
        self.particles.push(p);
    }

    /// Micro dust kicked up at a point on the floor: small lit cubes that tumble and settle.
    pub fn dust(&mut self, grid: &Grid, x: f64, y: f64, color: u32, count: usize, speed: f32) {
        let (x, z) = (x as f32, y as f32);
        let floor = floor_at(grid, x, z);
        let base = lin(color);
        for _ in 0..count {
            let r = &mut self.rng;
            let a = r.nextf() * TAU;
            let sp = speed * (0.4 + r.nextf() * 0.6);
            let shade = 0.75 + r.nextf() * 0.5;
            let p = Particle {
                pos: Vec3::new(x + (r.nextf() - 0.5) * 0.3, floor + 0.04 + r.nextf() * 0.15, z + (r.nextf() - 0.5) * 0.3),
                vel: Vec3::new(a.cos() * sp, speed * (0.4 + r.nextf() * 0.8), a.sin() * sp),
                color: base * shade,
                life: 0.3 + r.nextf() * 0.4,
                max: 0.7,
                size: 0.03 + r.nextf() * 0.035,
                gravity: 7.0,
                floor: floor + 0.02,
                glow: false,
            };
            self.push_particle(p);
        }
    }

    /// Glowing motes that burst upward from a point: pickups, kills, heals.
    pub fn sparkle(&mut self, grid: &Grid, x: f64, y: f64, height: f32, color: u32, count: usize) {
        let (x, z) = (x as f32, y as f32);
        let floor = floor_at(grid, x, z);
        let base = lin(color);
        for _ in 0..count {
            let r = &mut self.rng;
            let a = r.nextf() * TAU;
            let sp = 0.6 + r.nextf() * 1.6;
            let life = 0.4 + r.nextf() * 0.45;
            let p = Particle {
                pos: Vec3::new(x, floor + height, z),
                vel: Vec3::new(a.cos() * sp, 1.2 + r.nextf() * 2.6, a.sin() * sp),
                color: base * (1.2 + r.nextf() * 0.8),
                life,
                max: life,
                size: 0.035 + r.nextf() * 0.03,
                gravity: 4.0,
                floor: floor + 0.02,
                glow: true,
            };
            self.push_particle(p);
        }
    }

    pub(super) fn update_effects(&mut self, dt: f32, _s: &SceneState) {
        let drag = 1.0 - (2.4 * dt).min(0.9);
        self.particles.retain_mut(|p| {
            p.life -= dt;
            if p.life <= 0.0 {
                return false;
            }
            p.vel.y -= p.gravity * dt;
            p.vel.x *= drag;
            p.vel.z *= drag;
            p.pos += p.vel * dt;
            if p.pos.y < p.floor {
                p.pos.y = p.floor;
                p.vel.y *= -0.3;
                p.vel.x *= 0.5;
                p.vel.z *= 0.5;
            }
            true
        });
        self.debris.retain_mut(|d| {
            d.life -= dt;
            if d.life <= 0.0 {
                return false;
            }
            d.vel.y -= GRAVITY * dt;
            d.pos += d.vel * dt;
            if d.pos.y < d.floor_y {
                d.pos.y = d.floor_y;
                d.vel.y *= -0.35;
                d.vel.x *= 0.6;
                d.vel.z *= 0.6;
            }
            d.rot += d.spin * dt;
            true
        });
        let lights = &mut self.effect_lights;
        self.blasts.retain_mut(|b| {
            b.life -= dt;
            let owns = lights[b.light].owner == b.id;
            if b.life <= 0.0 {
                if owns {
                    lights[b.light].intensity = 0.0;
                }
                return false;
            }
            let t = 1.0 - b.life / b.seconds;
            if owns {
                lights[b.light].intensity = b.intensity * (1.0 - t);
            }
            true
        });
    }
}
