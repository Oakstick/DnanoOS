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
use crate::logic::combat::{MoveId, Reel, State};
use crate::render::anim::clip::Trs;
use crate::render::anim::foe::{FoeAnimator, FoeDrive, FoeReel, FoeStrike};
use crate::render::anim::foe_rigs::FoeRig;
use crate::render::anim::library::{self, Mode};
use crate::render::anim::warp::move_bounds;

/// Beyond this distance from the camera (tiles), where its rig is drawn at its coarsest, a foe's
/// clips are sampled every `FAR_EVERY` frames.
pub(super) const FAR: f32 = 16.0;
const FAR_EVERY: u8 = 3;

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
        // (A rival swaying back, and the dragon felled and begging: the code's.)
        if duel.is_some_and(|d| matches!(d.f.state, State::Evade { .. }) || d.begging()) {
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
                        drive.strike = Some(FoeStrike {
                            clips: move_clips(mv),
                            t: secs(frame),
                            bounds: move_bounds(secs(data.startup), secs(data.active), secs(data.recovery)),
                            serial: c.serial,
                        });
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
/// down on its talons, Níðhöggr sunk under the floor, Leviathan under the water.
fn settle_root(v: &mut EnemyView, base: Vec3, mv: Option<MoveId>) {
    match mv {
        Some(MoveId::Burrow) => {}
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
