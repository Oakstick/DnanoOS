//! The juice on screen (WARBAND.md §4.3-§4.5): the camera's roll, its kick along a blow and its
//! FOV punch, trauma by source; chips struck off a body by what it is made of, sparks off a
//! shield's rim, and a slain foe tipping over and breaking into its voxels; and the springs that
//! give blows weight (a foe's flinch, the shield knocked back, the leap's landing).
//!
//! What the camera does runs on real time (the eye keeps moving through a freeze); what the
//! bodies do runs on the game's pace, so a hitstop or a slowing of time holds it too. Every
//! random draw here is the view's own (presentation only, WARBAND.md I5).

use super::animation::Pose;
use super::effects::Debris;
use super::models::RigId;
use super::spring::{squash, Spring, Spring3};
use super::*;
use crate::logic::juice::{self, Matter, Trauma};
use crate::render::profile::fx;

/// A slain foe, tipping along the blow that felled it before it breaks: its body as last drawn.
pub(super) struct Topple {
    rig: RigId,
    pose: Pose,
    /// Its body as its clips last posed it, where they did: its jointed rig and its bones' matrices.
    clip: Option<(crate::render::anim::foe_rigs::FoeRig, Vec<Mat4>)>,
    pos: Vec3,
    floor: f32,
    yaw: f32,
    root_rot: Vec3,
    root_scale: Vec3,
    tint: Vec3,
    dir: Vec3,
    matter: Matter,
    giant: bool,
    /// Seconds of the game's time since it fell.
    age: f32,
}

/// The view's juice: switches, settings, the camera's springs, the foes' flinches and the bodies
/// falling.
pub(super) struct JuiceView {
    /// Juice switched off (`fx::DILATION` and the rest; see `render::profile::fx`).
    pub(super) off: u32,
    /// The player's "screen shake" setting, 0 to 1, applied after everything is summed.
    pub(super) shake_scale: f32,
    /// The share of real time the game ran this step (0 frozen), and its time scale (below 1 while
    /// time is slowed; a freeze leaves it be).
    pub(super) pace: f32,
    pub(super) slow: f32,
    kick: Spring3,
    /// A FOV punch: degrees, seconds, seconds since.
    fov: Option<(f32, f32, f32)>,
    /// The camera's roll now (radians), and the phase its noise starts from (seeded).
    pub(super) roll: f32,
    phase: f32,
    /// Each flinching duelist's spring, by its id, and the way it was struck (world, flat).
    recoil: HashMap<u32, (Vec3, Spring)>,
    shield: Spring,
    /// Seconds (game time) since his leap landed; whether he was in the air last step.
    landed: f32,
    airborne: bool,
    pub(super) topples: Vec<Topple>,
}

impl Default for JuiceView {
    fn default() -> Self {
        Self {
            off: 0,
            shake_scale: 1.0,
            pace: 1.0,
            slow: 1.0,
            kick: Spring3::settling(juice::KICK_ZETA, juice::KICK_SETTLE),
            fov: None,
            roll: 0.0,
            phase: 0.0,
            recoil: HashMap::new(),
            shield: Spring::settling(juice::SHIELD_ZETA, juice::SHIELD_SETTLE),
            landed: f32::INFINITY,
            airborne: false,
            topples: Vec::new(),
        }
    }
}

impl JuiceView {
    /// A new level: nothing toppling, flinching or kicked (the switches and settings kept).
    pub(super) fn reset(&mut self) {
        let (off, shake_scale, phase) = (self.off, self.shake_scale, self.phase);
        *self = Self { off, shake_scale, phase, ..Self::default() };
    }
}

/// The colours of what flies off a body of `matter` when it is struck.
fn palette(matter: Matter) -> &'static [u32] {
    match matter {
        Matter::Flesh => &[0x7a1010, 0x9a2020, 0x5a0a0a],
        Matter::Bone => &[0xe8dcc0, 0xcfc2a2, 0xb8a888],
        Matter::Iron => &[0x8a9098, 0x6a7078, 0xa8b0b8],
        Matter::Stone => &[0x6a6460, 0x58524e, 0x7a7470],
        Matter::Feather => &[0x2a2a30, 0x6a5a48, 0xd8d0c0],
        Matter::Smoke => &[0x3a3a42, 0x55555e, 0x6a6a72],
        Matter::Ember => &[0xff7a20, 0xffb040, 0xff4010],
        Matter::Ice => &[0xbfe8ff, 0x8ac8f0, 0xe8f8ff],
        Matter::Gold => &[0xf2c53d, 0xd8a020, 0xffe080],
        Matter::Slime => &[0x5ac040, 0x3a9a30, 0x8ae060],
        Matter::Wood => &[0x6a4a2a, 0x8a6a3a, 0x3a5a2a],
        Matter::Chitin => &[0x2a3a20, 0x4a3a2a, 0x3a3020],
    }
}

/// A grid direction (x, y) as a flat world direction (x, z), of length 1 (or none).
fn flat(dir: V) -> Vec3 {
    Vec3::new(dir.x as f32, 0.0, dir.y as f32).normalize_or_zero()
}

impl WorldView {
    // ---------------------------------------------------------------- the camera

    /// Trauma from a source (WARBAND.md §4.3): it adds its amount but never pushes the total past
    /// its own cap. (With the juice's shake off, the plain shake of the same amount.)
    pub fn trauma(&mut self, source: Trauma) {
        if self.juice.off & fx::SHAKE != 0 {
            self.shake(source.amount());
        } else {
            self.trauma = source.add_to(self.trauma);
        }
    }

    /// The camera kicked `amount` world units along `dir` (the way the blow travelled), springing
    /// back past rest: the player feels where the hit came from.
    pub fn kick(&mut self, dir: V, amount: f32) {
        if self.juice.off & fx::SHAKE != 0 {
            return;
        }
        let d = flat(dir);
        let speed = self.juice.kick.x.speed_for(amount);
        self.juice.kick.kick(d * speed);
    }

    /// The field of view punched by `deg` degrees (negative narrows) for `secs` seconds.
    pub fn fov_punch(&mut self, deg: f32, secs: f32) {
        if self.juice.off & fx::SHAKE != 0 {
            return;
        }
        self.juice.fov = Some((deg, secs.max(1e-3), 0.0));
    }

    /// The field of view this frame (degrees): the camera's own, and the punch under way.
    pub fn fov_degrees(&self) -> f32 {
        let punch = self.juice.fov.map(|(deg, secs, age)| {
            let u = (age / secs).clamp(0.0, 1.0);
            deg * (1.0 - u * u * (3.0 - 2.0 * u))
        });
        FOV_DEGREES + punch.unwrap_or(0.0) * self.juice.shake_scale
    }

    /// Where the kick has the camera now (world units), scaled by the shake setting.
    pub(super) fn kick_offset(&self) -> Vec3 {
        self.juice.kick.value() * self.juice.shake_scale
    }

    /// The player's "screen shake" setting (0 to 1) and the juice switched off (`fx` bits).
    pub fn set_feel(&mut self, shake: f32, off: u32) {
        self.juice.shake_scale = shake.clamp(0.0, 1.0);
        self.juice.off = off;
    }

    /// Seeds the camera's noise (the shake's and the roll's), so captures stay comparable.
    pub(super) fn seed_noise(&mut self) {
        self.juice.phase = self.rng.nextf() * 100.0;
        self.shake_clock = self.juice.phase;
    }

    // ---------------------------------------------------------------- the bodies

    /// Duelist `id` flinches, pushed back along `dir` (the way the blow travelled), and springs
    /// back past where it stood.
    pub fn recoil(&mut self, id: u32, dir: V) {
        if self.juice.off & fx::SPRINGS != 0 {
            return;
        }
        let d = flat(dir);
        let entry = self.juice.recoil.entry(id).or_insert((d, Spring::settling(juice::FLINCH_ZETA, juice::FLINCH_SETTLE)));
        entry.0 = d;
        let speed = entry.1.speed_for(juice::FLINCH_PUSH);
        entry.1.kick(speed);
    }

    /// His raised shield knocked back by a blow it took, and springing up again.
    pub fn shield_jolt(&mut self) {
        if self.juice.off & fx::SPRINGS != 0 {
            return;
        }
        let speed = self.juice.shield.speed_for(juice::SHIELD_JOLT);
        self.juice.shield.kick(speed);
    }

    /// How far duelist `id` is pushed by its flinch now (world offset), and its lean (x, z radians,
    /// in its own frame, which faces `yaw`).
    pub(super) fn flinch(&self, id: u32, yaw: f32) -> Option<(Vec3, Vec3)> {
        let (d, s) = self.juice.recoil.get(&id)?;
        let local = Quat::from_rotation_y(-yaw) * *d;
        Some((*d * s.x, Vec3::new(local.z * s.x * 2.4, 0.0, -local.x * s.x * 2.4)))
    }

    /// His shield's tilt back from a blow (radians).
    pub(super) fn shield_tilt(&self) -> f32 {
        self.juice.shield.x
    }

    /// His landing squash (y scale; x and z keep the volume).
    pub(super) fn hero_squash(&self) -> Vec3 {
        if self.juice.off & fx::SPRINGS != 0 || !self.juice.landed.is_finite() {
            return Vec3::ONE;
        }
        let z = juice::zeta_for_overshoot((juice::SLAM_PEAK - 1.0) / (1.0 - juice::SLAM_SQUASH));
        let y = squash(self.juice.landed, juice::LAND_SQUASH, z, 0.22);
        let xz = 1.0 / y.max(0.5).sqrt();
        Vec3::new(xz, y, xz)
    }

    // ---------------------------------------------------------------- matter

    /// Chips struck off a body of `matter` at `at` by a blow travelling along `dir`: about `n`
    /// (±25%), from the struck side, thrown along the blow and fanned across it as the blade's arc
    /// would throw them. Iron throws sparks as well; flesh, drops of ichor.
    pub fn chips(&mut self, grid: &Grid, at: V, dir: V, matter: Matter, n: usize) {
        if self.juice.off & fx::DEBRIS != 0 || n == 0 {
            return;
        }
        let (x, z) = (at.x as f32, at.y as f32);
        let floor = floor_at(grid, x, z);
        let d = flat(dir);
        let side = Vec3::new(-d.z, 0.0, d.x);
        let origin = Vec3::new(x, floor + 0.7, z) - d * 0.22;
        let g = matter.grain();
        let colors = palette(matter);
        let n = self.vary(n);
        let n = self.room_for(n);
        for _ in 0..n {
            let r = &mut self.rng;
            let along = 1.4 + r.nextf() * 2.2;
            let across = (r.nextf() - 0.5) * 3.0;
            let up = 1.0 + r.nextf() * 2.2;
            let vel = d * along + side * across + Vec3::Y * up;
            let pos = origin + Vec3::new((r.nextf() - 0.5) * 0.2, (r.nextf() - 0.5) * 0.3, (r.nextf() - 0.5) * 0.2);
            let size = g.chip * (0.7 + r.nextf() * 0.6);
            let color = lin(colors[(r.nextf() * colors.len() as f32) as usize % colors.len()]) * (0.85 + r.nextf() * 0.3);
            let spin = Vec3::new(r.nextf() * 14.0 - 7.0, r.nextf() * 14.0 - 7.0, r.nextf() * 14.0 - 7.0);
            let life = 0.9 + r.nextf() * 0.5;
            self.debris.push(Debris::of(pos, vel, spin, life, floor + size * 0.5, color, size, &g, 0.5));
        }
        let (sparks, drops) = match matter {
            Matter::Iron => (8, 0),
            Matter::Flesh => (0, 6),
            Matter::Gold => (4, 0),
            _ => (0, 0),
        };
        for k in 0..sparks + drops {
            let r = &mut self.rng;
            let spark = k < sparks;
            let vel = d * (2.0 + r.nextf() * 3.0) + side * (r.nextf() - 0.5) * 4.0 + Vec3::Y * (1.0 + r.nextf() * 2.5);
            let life = if spark { 0.18 + r.nextf() * 0.2 } else { 0.35 + r.nextf() * 0.3 };
            let p = Particle {
                pos: origin,
                vel,
                color: if spark { lin(0xffd080) * 3.0 } else { lin(0x6a0a0a) },
                life,
                max: life,
                size: if spark { 0.02 + r.nextf() * 0.015 } else { 0.03 + r.nextf() * 0.02 },
                gravity: if spark { 9.0 } else { 12.0 },
                floor: floor + 0.02,
                glow: spark,
            };
            self.push_particle(p);
        }
    }

    /// A fan of sparks off his shield's rim, where a blow travelling along `dir` met it at `at`.
    pub fn sparks_block(&mut self, grid: &Grid, at: V, dir: V) {
        if self.juice.off & fx::DEBRIS != 0 {
            return;
        }
        let (x, z) = (at.x as f32, at.y as f32);
        let floor = floor_at(grid, x, z);
        let d = flat(dir);
        // The rim faces the blow: the sparks fly back the way it came, fanned out to either side.
        let back = if d == Vec3::ZERO { Vec3::Z } else { -d };
        let rim = Vec3::new(x, floor + 0.62, z) + back * 0.32;
        for _ in 0..22 {
            let r = &mut self.rng;
            let a = (r.nextf() - 0.5) * 2.4;
            let fan = Quat::from_rotation_y(a) * back;
            let vel = fan * (2.5 + r.nextf() * 3.5) + Vec3::Y * (0.8 + r.nextf() * 2.4);
            let life = 0.15 + r.nextf() * 0.22;
            let p = Particle {
                pos: rim + Vec3::Y * (r.nextf() - 0.5) * 0.3,
                vel,
                color: lin(if r.nextf() < 0.7 { 0xffe0a0 } else { 0xffffff }) * 3.2,
                life,
                max: life,
                size: 0.018 + r.nextf() * 0.016,
                gravity: 8.0,
                floor: floor + 0.02,
                glow: true,
            };
            self.push_particle(p);
        }
    }

    /**
     * Foe `i` (its index in the game's list) is slain by a blow travelling along `dir`: its body,
     * as last drawn, tips along the blow for `TOPPLE_FRAMES` and breaks into the cubes of its coarse
     * rig (`Rig::lod` at a quarter), 24 to 60 of them (a giant's 90, and a ring of dust), each its
     * own voxels' colour, flying along the blow, out from the body and up, bouncing, resting, and
     * sinking away. Call it where the foe is slain: from the next frame the foe itself is no longer
     * drawn (a duelist that is not alive is hidden), and this takes its place.
     */
    pub fn shatter(&mut self, i: usize, dir: V) {
        if self.juice.off & fx::DEBRIS != 0 {
            return;
        }
        let Some(v) = self.enemy_views.get(i) else { return };
        if !v.visible {
            return;
        }
        let clip = v.clip.as_ref().filter(|c| c.posed).map(|c| (c.foe, c.mats.clone()));
        let topple = Topple {
            rig: v.rig,
            pose: v.pose.clone(),
            clip,
            pos: v.group_pos,
            floor: v.group_pos.y - v.hover,
            yaw: v.yaw,
            root_rot: v.root_rot,
            root_scale: v.root_scale,
            tint: v.mat_color,
            dir: flat(dir),
            matter: juice::matter_of(v.kind),
            giant: juice::is_giant(v.kind),
            age: if self.juice.off & fx::SPRINGS != 0 { juice::TOPPLE_FRAMES / 60.0 } else { 0.0 },
        };
        self.juice.topples.push(topple);
    }

    /// Bodies still tipping over (for tests and tools).
    pub fn toppling(&self) -> usize {
        self.juice.topples.len()
    }

    /// A count drawn ±25% about `n` (presentation rng).
    fn vary(&mut self, n: usize) -> usize {
        ((n as f32) * (0.75 + self.rng.nextf() * 0.5)).round().max(1.0) as usize
    }

    /// Room for `n` more pieces of debris: past half the budget the counts scale down, and past
    /// the whole of it the oldest go first. Returns how many to make.
    fn room_for(&mut self, n: usize) -> usize {
        let full = self.debris.len() as f32 / effects::MAX_DEBRIS as f32;
        let n = if full > 0.5 { ((n as f32) * (1.5 - full).clamp(0.4, 1.0)).round() as usize } else { n }.max(1);
        let over = (self.debris.len() + n).saturating_sub(effects::MAX_DEBRIS);
        if over > 0 {
            self.debris.drain(..over.min(self.debris.len()));
        }
        n.min(effects::MAX_DEBRIS)
    }

    /// The root a toppling body is drawn from, `age` seconds after it fell.
    fn topple_root(t: &Topple) -> Mat4 {
        let k = (t.age * 60.0 / juice::TOPPLE_FRAMES).clamp(0.0, 1.0);
        let tip = juice::TOPPLE_ANGLE * k * k;
        let axis = Vec3::Y.cross(t.dir);
        let tilt = if axis.length_squared() > 1e-6 { Mat4::from_axis_angle(axis.normalize(), tip) } else { Mat4::IDENTITY };
        Mat4::from_translation(t.pos) * tilt * Mat4::from_rotation_y(t.yaw) * euler(t.root_rot) * Mat4::from_scale(t.root_scale)
    }

    /// A toppled body breaks into its coarse voxels.
    fn break_up(&mut self, t: &Topple) {
        let root = Self::topple_root(t);
        let rig = match &t.clip {
            Some((foe, _)) => self.models.jointed_foe(*foe).lod(2),
            None => self.models.rig(t.rig).lod(2),
        };
        let voxel = rig.size * t.root_scale.max_element();
        let mut cubes: Vec<(Vec3, Vec3)> = Vec::new();
        for (p, part) in rig.parts.iter().enumerate() {
            let pm = match &t.clip {
                Some((_, mats)) => root * mats[p],
                None => root * Mat4::from_translation(t.pose.pos[p]) * euler(t.pose.rot[p]),
            };
            cubes.extend(part.cubes.iter().map(|(off, c)| (pm.transform_point3(*off), *c * t.tint)));
        }
        if cubes.is_empty() {
            return;
        }
        let n = juice::shatter_cubes(cubes.len(), t.giant);
        let n = self.room_for(n);
        let center = cubes.iter().map(|c| c.0).sum::<Vec3>() / cubes.len() as f32;
        // Each piece stands for several voxels: as big as they were together, near enough.
        let size = (voxel * (cubes.len() as f32 / n as f32).cbrt() * 0.8).clamp(0.05, if t.giant { 0.34 } else { 0.2 });
        let g = t.matter.grain();
        let stride = cubes.len() as f32 / n as f32;
        for k in 0..n {
            let (pos, color) = cubes[((k as f32 + 0.5) * stride) as usize % cubes.len()];
            let r = &mut self.rng;
            let out = Vec3::new(pos.x - center.x, 0.0, pos.z - center.z).normalize_or_zero();
            let vel = t.dir * juice::SHATTER_ALONG * (0.75 + r.nextf() * 0.5)
                + out * juice::SHATTER_RADIAL * (0.75 + r.nextf() * 0.5)
                + Vec3::Y * juice::SHATTER_UP * (0.6 + r.nextf() * 0.8);
            let spin = Vec3::new(r.nextf() * 12.0 - 6.0, r.nextf() * 12.0 - 6.0, r.nextf() * 12.0 - 6.0);
            let life = juice::SHATTER_FLIGHT * (0.8 + r.nextf() * 0.5) + juice::SHATTER_FADE;
            let s = size * (0.8 + r.nextf() * 0.4);
            let d = Debris::of(pos, vel, spin, life, t.floor + s * 0.5, color, s, &g, juice::SHATTER_FADE);
            self.debris.push(d);
        }
        if t.giant {
            // The ground takes the fall: a ring of dust rolling out from it.
            let base = Vec3::new(t.pos.x, t.floor + 0.05, t.pos.z);
            for k in 0..48 {
                let r = &mut self.rng;
                let a = k as f32 / 48.0 * TAU + r.nextf() * 0.1;
                let dir = Vec3::new(a.cos(), 0.0, a.sin());
                let life = 0.6 + r.nextf() * 0.5;
                let p = Particle {
                    pos: base + dir * 0.8,
                    vel: dir * (3.0 + r.nextf() * 2.0) + Vec3::Y * (0.4 + r.nextf() * 0.8),
                    color: lin(0x8a7a62) * (0.8 + r.nextf() * 0.4),
                    life,
                    max: life,
                    size: 0.06 + r.nextf() * 0.05,
                    gravity: 2.0,
                    floor: t.floor + 0.02,
                    glow: false,
                };
                self.push_particle(p);
            }
        }
    }

    // ---------------------------------------------------------------- each step

    /// The juice's step: the camera's springs and roll on real time (`dt`), the bodies' on the
    /// game's (`dt` times the pace; `slow` is the time scale). `airborne`: whether the hero is in the air.
    pub(super) fn update_juice(&mut self, dt: f32, pace: f32, slow: f32, airborne: bool) {
        self.juice.pace = pace;
        self.juice.slow = slow;
        let game_dt = dt * pace;
        self.juice.kick.step(dt);
        if let Some((_, secs, age)) = self.juice.fov.as_mut() {
            *age += dt;
            if *age >= *secs {
                self.juice.fov = None;
            }
        }
        let t = self.shake_clock;
        let noise = (t * 31.0).sin() * 0.6 + (t * 53.0 + 1.3).sin() * 0.4;
        self.juice.roll = if self.juice.off & fx::SHAKE != 0 {
            0.0
        } else {
            juice::ROLL_DEGREES.to_radians() * self.trauma * self.trauma * noise * self.juice.shake_scale
        };
        for (_, s) in self.juice.recoil.values_mut() {
            s.step(game_dt);
        }
        self.juice.recoil.retain(|_, (_, s)| !s.settled());
        self.juice.shield.step(game_dt);
        if self.juice.airborne && !airborne {
            self.juice.landed = 0.0;
        } else if self.juice.landed.is_finite() {
            self.juice.landed += game_dt;
            if self.juice.landed > 0.6 {
                self.juice.landed = f32::INFINITY;
            }
        }
        self.juice.airborne = airborne;
        for t in self.juice.topples.iter_mut() {
            t.age += game_dt;
        }
        self.break_toppled();
    }

    /// Breaks the bodies that have finished tipping over.
    fn break_toppled(&mut self) {
        let end = juice::TOPPLE_FRAMES / 60.0;
        let (ready, keep): (Vec<Topple>, Vec<Topple>) = std::mem::take(&mut self.juice.topples).into_iter().partition(|t| t.age >= end);
        self.juice.topples = keep;
        for t in &ready {
            self.break_up(t);
        }
    }

    /// The bodies still tipping over, and a glow on the floor under fallen embers.
    pub(super) fn emit_juice(&self, f: &mut Frame) {
        for t in &self.juice.topples {
            match &t.clip {
                Some((foe, mats)) => {
                    let rig = self.models.jointed_foe(*foe);
                    f.rigs.push_matrices(rig, mats, Self::topple_root(t), t.tint, 1.0, Vec3::ZERO, rig_look::PLAIN, 0.0);
                }
                None => emit_rig(&mut f.rigs, self.models.rig(t.rig), &t.pose, Self::topple_root(t), t.tint, 1.0, Vec3::ZERO),
            }
        }
        let (mut sum, mut n) = (Vec3::ZERO, 0);
        for d in self.debris.iter().filter(|d| d.glow > 1.0) {
            sum += d.pos;
            n += 1;
        }
        if n > 0 {
            f.lights.push(Light {
                pos: sum / n as f32 + Vec3::Y * 0.3,
                color: lin(0xff8a30) * (1.5 + n as f32 * 0.1).min(5.0),
                distance: 3.0,
            });
        }
    }
}

impl WorldView {
    /**
     * A duelist's overshoot on top of its pose (WARBAND.md §4.5), by its fighter's frame: its
     * flinch (kicked back along the blow, springing past where it stood); its anticipation (it
     * pulls back in the first third of its startup and holds there until it strikes, when the
     * pull springs forward past rest); a slam's landing (squashed on its first active frame,
     * springing past its height). None of it touches the colours: the commit flash shows whole.
     */
    pub(super) fn overshoot_duel(&mut self, i: usize, d: &crate::logic::combat::Duelist) {
        use crate::logic::combat::{MoveId, State};
        if self.juice.off & fx::SPRINGS != 0 {
            return;
        }
        let flinch = self.flinch(d.id, self.enemy_views[i].yaw);
        let v = &mut self.enemy_views[i];
        if let Some((push, lean)) = flinch {
            v.group_pos += push;
            v.root_rot += lean;
        }
        let State::Attack { mv, frame } = d.f.state else { return };
        let data = d.data(mv);
        let (s, f) = (data.startup as f32, frame as f32);
        let height = super::duel::crown(v.kind).unwrap_or(1.2);
        let back = -flat(d.aim);
        let pull = juice::ANTICIPATION * height;
        if f < s {
            let u = (f / (s * juice::ANTICIPATION_BY)).min(1.0);
            let k = u * u * (3.0 - 2.0 * u);
            v.group_pos += back * pull * k;
            v.root_scale.y *= 1.0 - juice::ANTICIPATION * 0.5 * k;
        } else {
            // Released: it springs forward through rest and back (seconds of the move's frames).
            let t = (f - s) / 60.0;
            let k = juice::stiffness_for(0.45, 0.2);
            let left = 1.0 - super::spring::step_response(t, k, 0.45);
            v.group_pos += back * pull * left;
            if matches!(mv, MoveId::Slam | MoveId::Kanabo | MoveId::FrostSlam | MoveId::HoardSlam) {
                let z = juice::zeta_for_overshoot((juice::SLAM_PEAK - 1.0) / (1.0 - juice::SLAM_SQUASH));
                let y = squash(t, juice::SLAM_SQUASH, z, 0.25);
                let xz = 1.0 / y.max(0.5).sqrt();
                v.root_scale *= Vec3::new(xz, y, xz);
            }
        }
    }
}
