//! The foes by their clips (ANIMATION.md §11): after the code has posed a foe (which stays the
//! fallback and the comparison), its animator (`anim::foe`) is told what it is doing (its pace, its
//! move and the move's frames, its reel) and gives the pose of its jointed rig; the clip carries its
//! own lean, twist and bob, so the code's are taken back, and the juice (the anticipation, the
//! flinch springs, the slam's squash) is laid on top as before. Nothing here is read by the game.
//!
//! A foe far from the camera is posed every few frames (`FAR_EVERY`), one off screen not at all;
//! a slain draugr's scatter bursts its jointed bones (all nineteen of them) as it did its six parts.

use super::animation::Pose;
use super::*;
use crate::logic::combat::{Archetype, MoveId, Reel, State};
use crate::render::anim::clip::Trs;
use crate::render::anim::foe::{FoeAnimator, FoeDrive, FoeReel, FoeStrike, WALK_TO_RUN};
use crate::render::anim::foe_rigs::FoeRig;
use crate::render::anim::library::{self, Mode};
use crate::render::anim::warp::move_bounds;

/// Beyond this distance from the camera (tiles), where its rig is drawn at its coarsest, a foe's
/// clips are sampled every `FAR_EVERY` frames.
pub(super) const FAR: f32 = 16.0;
const FAR_EVERY: u8 = 3;
/// A prince's roar as a phase begins, played at the rally's frames (startup, active, recovery).
const ROAR: (u16, u16, u16) = (40, 1, 36);
/// The moves that are a run at him: their active frames are run on the foe's feet (its run clip).
const CHARGES: [MoveId; 3] = [MoveId::FieryCharge, MoveId::BullCharge, MoveId::HoofCharge];

/// Each bone's flight as a body bursts apart: its velocity and its spin.
type Burst = Vec<(Vec3, Vec3)>;

/// A foe's clips at work: its animator, its pose and its parts' matrices.
pub(super) struct FoeClips {
    pub(super) foe: FoeRig,
    anim: FoeAnimator,
    locals: Vec<Trs>,
    pub(super) mats: Vec<Mat4>,
    /// Its clips posed it this frame (else the code's rig and pose are drawn).
    pub(super) posed: bool,
    /// The move under way and its last frame, and how many moves it has begun.
    last_move: Option<(MoveId, u16)>,
    serial: u32,
    /// Its jointed bones bursting apart and coming back (a slain draugr), and their flight.
    scatter: Option<(Pose, Option<Burst>)>,
    /// Frames since it was last posed, and the time they took.
    skipped: u8,
    owed: f32,
    /// A prince's phase as last seen, and his roar as a new one begins (seconds into it).
    phase: u8,
    roar: Option<f32>,
}

impl FoeClips {
    /// A foe of `kind`'s clips, if it has a jointed rig and clips; `phase` starts its cycles apart
    /// from its kin's.
    pub(super) fn of(kind: EnemyKind, phase: f32) -> Option<FoeClips> {
        let foe = FoeRig::of(kind)?;
        library::foe_clips(foe)?;
        Some(FoeClips {
            foe,
            anim: FoeAnimator::new(phase),
            locals: Vec::new(),
            mats: Vec::new(),
            posed: false,
            last_move: None,
            serial: 0,
            scatter: None,
            skipped: 0,
            owed: 0.0,
            phase: 0,
            roar: None,
        })
    }
}

/// The clips of a move: its own, by the move's name in snake case (`neck_bite` for NeckBite, `coin_toss`
/// for CoinToss), and its kind's where a foe has none of its own (a slam for the kanabo's blow and
/// the frost slam, a hex for a bolt, a bite for the neck's).
pub(super) fn move_clips(mv: MoveId) -> [&'static str; 2] {
    let kind = match mv {
        MoveId::Slam | MoveId::Kanabo | MoveId::FrostSlam | MoveId::HoardSlam | MoveId::HourglassSlam | MoveId::BellySlam => "slam",
        MoveId::Sweep => "sweep",
        MoveId::Hex | MoveId::Smokeless | MoveId::Thunder | MoveId::Foxfire => "hex",
        MoveId::Bite | MoveId::NeckBite => "bite",
        MoveId::Rally => "rally",
        _ => "-",
    };
    [snake(mv), kind]
}

/// A move's name in snake case, made once per move.
fn snake(mv: MoveId) -> &'static str {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static NAMES: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();
    let debug = format!("{mv:?}");
    let mut names = NAMES.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap_or_else(|e| e.into_inner());
    if let Some(n) = names.get(&debug) {
        return n;
    }
    let mut out = String::new();
    for (k, ch) in debug.chars().enumerate() {
        if ch.is_ascii_uppercase() && k > 0 {
            out.push('_');
        }
        out.push(ch.to_ascii_lowercase());
    }
    let leaked: &'static str = Box::leak(out.into_boxed_str());
    names.insert(debug, leaked);
    leaked
}

impl WorldView {
    /**
     * Foe `i` by its clips, after the code posed it: `base` is where it stands this frame before
     * the code's bob and sway (the clip carries its own). Leaves its clips unposed (the code's pose
     * drawn) when it has none, when `VH_ANIM=code`, or for a moment its clips lack.
     */
    pub(super) fn animate_foe_clips(&mut self, i: usize, e: &Enemy, base: Vec3, dt: f32) {
        let mode = self.anim_mode;
        let (cam, view_proj) = (self.cam_pos, self.view_proj());
        let models = &self.models;
        let rng = &mut self.rng;
        let Some(v) = self.enemy_views.get_mut(i) else { return };
        let Some(c) = v.clip.as_mut() else { return };
        c.posed = false;
        if mode == Mode::Code {
            return;
        }
        let rig = models.jointed_foe(c.foe);
        // A slain draugr: its bones burst and come back together (as the code's six parts do).
        if e.mode() == EnemyMode::Scattered {
            let (pose, burst) = c.scatter.get_or_insert_with(|| (Pose::rest(rig), None));
            super::animation::scatter_step(rig, pose, burst, e.scatter_progress(), dt, rng);
            c.mats.clear();
            c.mats.extend(pose.pos.iter().zip(&pose.rot).map(|(p, r)| Mat4::from_translation(*p) * euler(*r)));
            c.posed = true;
            c.anim.reset();
            return;
        }
        c.scatter = None;
        let Some(clips) = library::foe_clips(c.foe) else { return };
        let duel = e.duel.as_deref();
        // (A rival swaying back, the dragon felled and begging, Belphegor asleep: the code's.)
        if duel.is_some_and(|d| matches!(d.f.state, State::Evade { .. }) || d.begging() || d.lore.slumber.depth() > 0.0) {
            // (A rival swaying back out of the hero's finisher: the code's.)
            c.anim.reset();
            return;
        }
        // Off screen it is not drawn: nothing to pose (it picks up where its clock has run to).
        let at = view_proj * (v.group_pos + Vec3::new(0.0, 0.5, 0.0)).extend(1.0);
        let shown = at.w > 0.0 && at.x.abs() <= at.w * 1.25 + 1.0 && at.y.abs() <= at.w * 1.25 + 1.0;
        let far = v.group_pos.distance(cam) > FAR;
        c.owed += dt;
        if !shown || (far && c.skipped + 1 < FAR_EVERY && !c.mats.is_empty()) {
            c.skipped += 1;
            // (Far, its last pose is drawn again.)
            c.posed = shown && !c.mats.is_empty();
            if c.posed {
                settle_root(v, base, duel.and_then(|d| d.f.attack()).map(|a| a.0));
            }
            return;
        }
        let dt = std::mem::take(&mut c.owed);
        c.skipped = 0;
        let mut drive = FoeDrive { dt, ..Default::default() };
        match duel {
            Some(d) => {
                // A prince roars as each phase begins, over the breather the game gives him (it has
                // no pose of its own in the code: his roar is heard and the hall shakes).
                if d.arch == Archetype::Prince
                    && c.phase != 0
                    && d.breed.phase > c.phase
                    && ["roar", "rally"].iter().any(|n| clips.get(n).is_some())
                {
                    c.roar = Some(0.0);
                    c.serial = c.serial.wrapping_add(1);
                }
                c.phase = d.breed.phase;
                drive.speed = (d.vel.x * d.vel.x + d.vel.y * d.vel.y).sqrt() as f32;
                drive.moving = drive.speed > 0.3;
                match d.f.state {
                    State::Attack { mv, frame } => {
                        if c.last_move.is_none_or(|(m, f)| m != mv || frame < f) {
                            c.serial = c.serial.wrapping_add(1);
                        }
                        c.last_move = Some((mv, frame));
                        let data = d.data(mv);
                        let secs = |f: u16| f as f32 / 60.0;
                        let running = CHARGES.contains(&mv) && frame >= data.startup && frame < data.startup + data.active;
                        if running {
                            // A charge: its wind-up and its pulling up the move's clip, its run between
                            // them its feet's, at full tilt.
                            drive.moving = true;
                            drive.speed = drive.speed.max(WALK_TO_RUN[1]);
                        } else {
                            drive.strike = Some(FoeStrike {
                                clips: move_clips(mv),
                                t: secs(frame),
                                bounds: move_bounds(secs(data.startup), secs(data.active), secs(data.recovery)),
                                serial: c.serial,
                            });
                        }
                    }
                    State::Reel { kind, left, total } => {
                        let p = 1.0 - left as f32 / total.max(1) as f32;
                        let how = match kind {
                            Reel::Hitstun | Reel::Blockstun => FoeReel::Hit,
                            Reel::Staggered | Reel::GuardBroken => FoeReel::Stagger,
                            Reel::Deflected => FoeReel::Deflected,
                        };
                        drive.reel = Some((how, p));
                    }
                    _ => {}
                }
                if !matches!(d.f.state, State::Neutral) {
                    c.roar = None;
                }
                if let Some(t) = c.roar {
                    let secs = |f: u16| f as f32 / 60.0;
                    let (s, a, r) = ROAR;
                    if t > secs(s + a + r) {
                        c.roar = None;
                    } else {
                        drive.strike = Some(FoeStrike {
                            clips: ["roar", "rally"],
                            t,
                            bounds: move_bounds(secs(s), secs(a), secs(r)),
                            serial: c.serial,
                        });
                        c.roar = Some(t + dt);
                    }
                }
                if !matches!(d.f.state, State::Attack { .. }) {
                    c.last_move = None;
                }
            }
            None => {
                drive.moving = e.m.dir != Dir::None && e.mode() != EnemyMode::Nest;
                drive.speed = if drive.moving { e.m.speed as f32 } else { 0.0 };
            }
        }
        if !c.anim.step(clips, c.foe.def(), &drive, &mut c.locals) {
            c.anim.reset();
            return;
        }
        let joints: Vec<Vec3> = rig.parts.iter().map(|p| p.rest).collect();
        clips.skeleton.matrices_into(&c.locals, &joints, &[], &mut c.mats);
        // The kappa's dish spilled: no water in it until it fills again (as the code sinks it).
        if let (Some(w), Some(d)) = (rig.index("water"), duel) {
            if d.lore.spilled > 0 {
                c.mats[w] = Mat4::from_translation(Vec3::new(0.0, -0.3, 0.0)) * c.mats[w];
            }
        }
        // Krampus's sack taken off his hip by its neck and swung from his fist (as the code hangs it
        // there): its neck carried to his right hand and back.
        if let (FoeRig::Krampus, Some(d)) = (c.foe, duel) {
            if let (Some((MoveId::SackSwing, _, frame)), Some(sk), Some(h)) = (d.f.attack(), rig.index("sack"), rig.index("hand.R")) {
                let data = d.data(MoveId::SackSwing);
                let (s, a, r) = (data.startup as f32, data.active as f32, data.recovery as f32);
                let f = frame as f32;
                let wind = ease(f / s);
                let out = 1.0 - if f < s + a { 0.0 } else { ease((f - s - a) / r) };
                let k = ease(wind * 1.6).min(ease(out * 1.6));
                let held = c.mats[h] * Mat4::from_translation(rig.parts[h].rest - rig.parts[sk].rest);
                let (s0, r0, t0) = c.mats[sk].to_scale_rotation_translation();
                let (s1, r1, t1) = held.to_scale_rotation_translation();
                c.mats[sk] = Mat4::from_scale_rotation_translation(s0.lerp(s1, k), r0.slerp(r1, k), t0.lerp(t1, k));
            }
        }
        // Lucifer's white six while he stands, the burnt six when he has fallen (as the code sinks
        // the others far under the floor).
        if c.foe == FoeRig::Lucifer {
            let fallen = duel.is_some_and(|d| d.breed.phase >= 3);
            for (name, shown) in [("wings.L", !fallen), ("wings.R", !fallen), ("burnt.L", fallen), ("burnt.R", fallen)] {
                if let (Some(k), false) = (rig.index(name), shown) {
                    c.mats[k] = Mat4::from_translation(Vec3::new(0.0, -40.0, 0.0)) * c.mats[k];
                }
            }
        }
        c.posed = true;
        settle_root(v, base, duel.and_then(|d| d.f.attack()).map(|a| a.0));
    }

    /// Foe `i` as its clips pose it this frame (its jointed rig and its parts' matrices in the
    /// model), unless the code poses it.
    pub(super) fn foe_clip_pose(&self, i: usize) -> Option<(&Rig, &[Mat4])> {
        if self.anim_mode == Mode::Code {
            return None;
        }
        let c = self.enemy_views.get(i)?.clip.as_ref().filter(|c| c.posed)?;
        Some((self.models.jointed_foe(c.foe), c.mats.as_slice()))
    }

    /// Whether foe `i` was posed by its clips this frame (tests, captures).
    pub fn foe_posed_by_clips(&self, i: usize) -> bool {
        self.foe_clip_pose(i).is_some()
    }

    /// Whether foe `i` has clips at all (captures).
    pub fn foe_has_clips(&self, i: usize) -> bool {
        self.enemy_views.get(i).is_some_and(|v| v.clip.is_some())
    }
}

/// The code's lean, sway, twist, squash and bob taken back: the clip carries its own. (The juice
/// is laid on after, `overshoot_duel`.) What the code does to the whole body for a folklore move
/// stays the code's: the arçura grown tall as a tree for her roots, the tülükuş up on its wings and
/// down on its talons, Níðhöggr sunk under the floor, Leviathan under the water, Beelzebub's roll.
fn settle_root(v: &mut EnemyView, base: Vec3, mv: Option<MoveId>) {
    match mv {
        Some(MoveId::Burrow) => {}
        // (Beelzebub rolling over and over on his belly; squashed as he bursts into flies; up on his
        // toes before his belly slam.)
        Some(MoveId::BellyRoll) => v.group_pos = base,
        Some(MoveId::Scatter) => {
            v.root_rot = Vec3::ZERO;
            v.group_pos = base;
        }
        Some(MoveId::BellySlam) => {
            v.root_rot = Vec3::ZERO;
            v.root_scale = Vec3::ONE;
        }
        Some(MoveId::Roots) => {
            v.root_rot = Vec3::ZERO;
            v.group_pos = base;
        }
        Some(MoveId::Talons | MoveId::Dive) => {
            v.root_rot = Vec3::ZERO;
            v.root_scale = Vec3::ONE;
        }
        _ => {
            v.root_rot = Vec3::ZERO;
            v.root_scale = Vec3::ONE;
            v.group_pos = base;
        }
    }
}

fn ease(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}
