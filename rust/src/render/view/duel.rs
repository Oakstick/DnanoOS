//! The Sparring Ring's duelists on screen (COMBAT.md §1.3): each stands where its mind put it and
//! faces its own way; its move plays on top of its gait (the wind-up is the tell, the strike
//! snaps, the recovery hangs open), its colours tell its state, and the floor shows where its
//! attack will land, filling as the startup runs and flashing in the commit frames. Token
//! holders wear a red pip; hex bolts fly as violet sparks (gold, once parried back).

use super::*;
use crate::logic::combat::{Archetype, Duelist, MoveId, Phase, Reel, State};

/// The commit flash: the last frames of a startup, when the aim no longer follows.
const COMMIT_FRAMES: u16 = 8;

/// An archetype's telegraph colour.
fn tell(arch: Archetype) -> u32 {
    match arch {
        Archetype::Brute => 0xff5a2a,
        Archetype::Skirmisher => 0xff3a6a,
        Archetype::Hexer => 0xb066ff,
    }
}

fn ease(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

impl WorldView {
    /// A duelist's place, heading, pose and colours this frame.
    pub(super) fn update_duelist(&mut self, i: usize, e: &Enemy, d: &Duelist, s: &SceneState, dt: f32, time: f32) {
        let (x, z) = (d.pos.x as f32, d.pos.y as f32);
        let ground = floor_at(s.grid, x, z);
        {
            let v = &mut self.enemy_views[i];
            v.group_pos = Vec3::new(x, ground + v.hover, z);
            let goal = (d.facing.x as f32).atan2(d.facing.y as f32);
            v.yaw = lerp_angle(v.yaw, goal, 1.0 - (-24.0 * dt).exp());
        }
        self.animate_enemy(i, e, dt, time);
        self.pose_duel(i, d, time);
        let v = &mut self.enemy_views[i];
        v.visible = d.alive();
        let glow = lin(tell(d.arch));
        v.mat_color = Vec3::ONE;
        v.emissive = Vec3::ZERO;
        v.light_color = v.base_color;
        v.light_intensity = if v.visible { 2.5 } else { 0.0 };
        match d.f.state {
            State::Attack { mv, frame } => {
                let data = mv.data();
                match data.phase(frame) {
                    Phase::Startup => {
                        let u = frame as f32 / data.startup as f32;
                        let pulse = if data.startup - frame <= COMMIT_FRAMES { 0.9 + (time * 40.0).sin().abs() * 0.9 } else { u * u * 1.2 };
                        v.emissive = glow * pulse;
                        v.light_color = glow;
                        v.light_intensity = 2.5 + pulse * 2.5;
                    }
                    Phase::Active => v.emissive = glow + Vec3::splat(0.5),
                    // Open: it hangs, dimmed.
                    Phase::Recovery => v.mat_color = Vec3::splat(0.55),
                }
            }
            State::Reel { kind: Reel::Staggered | Reel::GuardBroken, .. } => {
                v.mat_color = lin(STUN);
                v.emissive = lin(STUN) * (0.5 + (time * 12.0).sin() * 0.3);
                v.light_color = lin(STUN);
            }
            State::Reel { kind: Reel::Deflected, .. } => {
                v.mat_color = Vec3::new(0.8, 0.9, 1.3);
                v.emissive = lin(0x9ad1ff) * 0.45;
            }
            _ => {}
        }
        if d.flash > 0 {
            let k = d.flash as f32 / 6.0;
            v.emissive += Vec3::splat(1.3 * k);
            v.mat_color *= 1.0 + 0.4 * k;
        }
    }

    /// Its move or its reel, over its gait.
    fn pose_duel(&mut self, i: usize, d: &Duelist, time: f32) {
        let rig = self.models.rig(self.enemy_views[i].rig);
        let v = &mut self.enemy_views[i];
        v.root_rot.y = 0.0;
        v.root_scale = Vec3::ONE;
        if d.arch != Archetype::Skirmisher {
            v.root_rot.x = 0.0; // (the loup-garou's gallop sets its own)
        }
        match d.f.state {
            State::Attack { mv, frame } => {
                let data = mv.data();
                let (s, a, f) = (data.startup as f32, data.active as f32, frame as f32);
                // 0..1 through the wind-up, the strike and the recovery.
                let (wind, strike, rec) = if f < s {
                    (ease(f / s), 0.0, 0.0)
                } else if f < s + a {
                    (1.0, ((f - s + 1.0) / a).min(1.0), 0.0)
                } else {
                    (1.0, 1.0, ease((f - s - a) / data.recovery as f32))
                };
                let snap = 1.0 - (1.0 - strike).powi(3);
                match mv {
                    MoveId::Slam => {
                        // Both fists rise overhead as he leans back, come down like a hammer, and heave up slowly.
                        let arms = if strike == 0.0 {
                            -2.9 * wind
                        } else if rec == 0.0 {
                            -2.9 + 3.8 * snap
                        } else {
                            0.9 * (1.0 - rec)
                        };
                        v.pose.rot_x(rig, "leftArm", arms);
                        v.pose.rot_x(rig, "rightArm", arms);
                        v.root_rot.x = if strike == 0.0 { -0.18 * wind } else { 0.32 * (1.0 - rec) };
                    }
                    MoveId::Sweep => {
                        // Wound round to one side, then a sweep right across.
                        v.root_rot.y = if strike == 0.0 {
                            -0.9 * wind
                        } else if rec == 0.0 {
                            -0.9 + 2.2 * snap
                        } else {
                            1.3 * (1.0 - rec)
                        };
                        v.pose.rot_x(rig, "leftArm", -1.5 * (1.0 - rec));
                        v.pose.rot_x(rig, "rightArm", -1.5 * (1.0 - rec));
                    }
                    MoveId::Lunge => {
                        if strike == 0.0 {
                            // Crouched and coiled, head low.
                            v.root_scale = Vec3::new(1.0 + 0.08 * wind, 1.0 - 0.22 * wind, 1.0 - 0.06 * wind);
                            v.root_rot.x = -0.12 * wind;
                            v.pose.rot_x(rig, "head", 0.35 * wind);
                        } else {
                            // Stretched out in the bite.
                            let out = 1.0 - rec;
                            v.root_scale = Vec3::new(1.0, 1.0, 1.0 + 0.18 * out);
                            v.root_rot.x = 0.22 * out;
                            v.pose.rot_x(rig, "frontLeftLeg", -1.1 * out);
                            v.pose.rot_x(rig, "frontRightLeg", -1.1 * out);
                            v.pose.rot_x(rig, "head", -0.3 * out);
                        }
                    }
                    MoveId::Hex => {
                        // Her right hand high, her left reaching out; then the throw.
                        let right = if strike == 0.0 { -2.8 * wind } else { -1.4 * (1.0 - rec) };
                        v.pose.rot_x(rig, "rightArm", right);
                        v.pose.rot_x(rig, "leftArm", -1.1 * wind * (1.0 - rec));
                    }
                    _ => {}
                }
            }
            State::Reel { kind, left, total } => {
                let k = left as f32 / total.max(1) as f32;
                match kind {
                    Reel::Hitstun | Reel::Blockstun => v.root_rot.x = -0.35 * k,
                    Reel::Deflected => {
                        v.root_rot.x = -0.5 * k;
                        v.pose.rot_x(rig, "leftArm", -2.0 * k);
                        v.pose.rot_x(rig, "rightArm", -2.0 * k);
                    }
                    Reel::Staggered | Reel::GuardBroken => {
                        v.root_rot.z = (time * 9.0).sin() * 0.16;
                        v.root_rot.x = 0.18;
                        v.pose.rot_x(rig, "head", 0.4);
                    }
                }
            }
            _ => {}
        }
    }

    /// The ring's floor tells, token pips and hex bolts.
    pub(super) fn emit_duels(&self, s: &SceneState, f: &mut Frame) {
        let time = s.time as f32;
        for (i, e) in s.enemies.iter().enumerate() {
            let Some(d) = e.duel.as_deref() else { continue };
            if !d.alive() || i >= self.enemy_views.len() {
                continue;
            }
            let base = Vec3::new(d.pos.x as f32, floor_at(s.grid, d.pos.x as f32, d.pos.y as f32), d.pos.y as f32);
            if d.token {
                // A red pip over the one whose turn it is.
                let top = match d.arch {
                    Archetype::Brute => 1.25,
                    Archetype::Skirmisher => 0.95,
                    Archetype::Hexer => 1.55,
                };
                let m = Mat4::from_translation(base + Vec3::new(0.0, top + (time * 4.0).sin() * 0.04, 0.0))
                    * Mat4::from_rotation_y(time * 3.0)
                    * Mat4::from_rotation_z(PI / 4.0)
                    * Mat4::from_scale(Vec3::splat(0.09));
                f.additive.push(Instance::new(m, lin(0xff3a2a) * 2.2, 0.95, Vec3::ZERO, SHADE_UNLIT));
            }
            let State::Attack { mv, frame } = d.f.state else { continue };
            let data = mv.data();
            let phase = data.phase(frame);
            if phase == Phase::Recovery {
                continue;
            }
            let color = lin(tell(d.arch));
            let fill = if phase == Phase::Active { 1.0 } else { frame as f32 / data.startup as f32 };
            let commit = phase == Phase::Startup && data.startup - frame <= COMMIT_FRAMES;
            let blink = if commit { 0.6 + (time * 40.0).sin().abs() * 0.6 } else { 0.0 };
            let aim = Vec3::new(d.aim.x as f32, 0.0, d.aim.y as f32).normalize_or_zero();
            let side = Vec3::new(-aim.z, 0.0, aim.x);
            let mut dot = |p: Vec3, lit: bool| {
                let (glow, alpha) = if phase == Phase::Active {
                    (color * 2.6 + Vec3::splat(0.6), 0.95)
                } else if lit {
                    (color * (1.4 + blink), 0.8)
                } else {
                    (color * (0.5 + blink * 0.6), 0.45)
                };
                let m = Mat4::from_translation(base + p + Vec3::new(0.0, 0.02, 0.0)) * Mat4::from_scale(Vec3::new(0.085, 0.012, 0.085));
                f.additive.push(Instance::new(m, glow, alpha, Vec3::ZERO, SHADE_UNLIT));
            };
            match mv {
                MoveId::Lunge => {
                    // A lane down its aim, as far as the bite will carry.
                    let length = data.travel as f32 + data.reach as f32;
                    let steps = (length / 0.28).ceil() as i32;
                    for k in 1..=steps {
                        let along = k as f32 * length / steps as f32;
                        for w in [-0.32f32, 0.0, 0.32] {
                            dot(aim * along + side * w, along <= length * fill);
                        }
                    }
                }
                MoveId::Hex => {
                    // A line toward him, the way the bolt will fly.
                    for k in 1..=12 {
                        let along = 0.5 + k as f32 * 0.3;
                        dot(aim * along, k as f32 / 12.0 <= fill);
                    }
                    // The bolt gathering in her hand.
                    let orb = base + aim * 0.35 + Vec3::new(0.0, 1.1, 0.0);
                    let m =
                        Mat4::from_translation(orb) * Mat4::from_rotation_y(time * 6.0) * Mat4::from_scale(Vec3::splat(0.04 + 0.1 * fill));
                    f.additive.push(Instance::new(m, color * 2.4, 0.9, Vec3::ZERO, SHADE_UNLIT));
                }
                _ => {
                    // The cone of the blow, ring by ring.
                    let (reach, half) = (data.reach as f32, data.half_angle as f32);
                    let rings = (reach / 0.3).ceil() as i32;
                    for r in 1..=rings {
                        let rr = r as f32 * reach / rings as f32;
                        let n = ((2.0 * half * rr) / 0.3).ceil().max(1.0) as i32;
                        for k in 0..=n {
                            let a = -half + 2.0 * half * k as f32 / n as f32;
                            let dir = aim * a.cos() + side * a.sin();
                            dot(dir * rr, rr <= reach * fill);
                        }
                    }
                }
            }
        }
        // Hex bolts: a spinning spark and its trail, violet (gold once parried back), lighting the floor.
        for b in s.bolts {
            let p = Vec3::new(b.pos.x as f32, 1.0, b.pos.y as f32);
            let v = Vec3::new(b.vel.x as f32, 0.0, b.vel.y as f32).normalize_or_zero();
            let color = lin(if b.reflected { 0xffc850 } else { 0xc070ff });
            for k in 0..4 {
                let m = Mat4::from_translation(p - v * k as f32 * 0.14)
                    * Mat4::from_rotation_y(time * 9.0 + k as f32)
                    * Mat4::from_rotation_x(time * 7.0)
                    * Mat4::from_scale(Vec3::splat(0.13 - k as f32 * 0.025));
                f.additive.push(Instance::new(m, color * (2.6 - k as f32 * 0.5), 0.95 - k as f32 * 0.2, Vec3::ZERO, SHADE_UNLIT));
            }
            f.lights.push(Light { pos: p, color: color * 2.5, distance: 3.0 });
        }
    }
}
