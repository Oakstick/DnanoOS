//! Animation: the hero's stride, swing and jump, and each foe's gait, wingbeat, hop, scuttle or
//! sway, how it turns, and a slain draugr falling apart and pulling itself together.

use super::*;

pub(super) const TURN_RATE_HERO: f32 = 14.0;

const TURN_RATE_ENEMY: f32 = 10.0;

const REASSEMBLE_START: f64 = 0.7;

/// How long the sword's wind-up, strike and follow-through take on screen.
pub const SLASH_SECONDS: f32 = 0.3;

#[derive(Clone)]
pub(super) struct Pose {
    pub(super) pos: Vec<Vec3>,
    pub(super) rot: Vec<Vec3>,
}

impl Pose {
    pub(super) fn rest(rig: &Rig) -> Self {
        Self { pos: rig.parts.iter().map(|p| p.rest).collect(), rot: vec![Vec3::ZERO; rig.parts.len()] }
    }
    pub(super) fn rot_x(&mut self, rig: &Rig, name: &str, angle: f32) {
        if let Some(i) = rig.index(name) {
            self.rot[i].x = angle;
        }
    }

    /// Weapons at the ready, for a hero standing (the title stage, the character screen): the
    /// shield forearm turned so its shield's face looks forward and out (strapped edge-on to his
    /// front, it showed only its rim), and the sword arm a little forward, the blade clear of it.
    /// The held parts turn with their arms.
    pub(super) fn ready(&mut self, rig: &Rig) {
        for name in ["shieldArm", "shield", "offhand"] {
            if let Some(i) = rig.index(name) {
                self.rot[i].x = SHIELD_FORWARD;
                self.rot[i].y = SHIELD_TURN;
            }
        }
        for name in ["swordArm", "blade"] {
            if let Some(i) = rig.index(name) {
                self.rot[i].x = SWORD_FORWARD;
            }
        }
    }
}

/// The ready stance: how far the shield forearm turns its shield toward the front, and how far
/// each arm is carried forward of hanging straight.
pub(super) const SHIELD_TURN: f32 = -0.7;
pub(super) const SHIELD_FORWARD: f32 = -0.15;
pub(super) const SWORD_FORWARD: f32 = -0.3;

pub(super) struct EnemyView {
    pub(super) rig: RigId,
    pub(super) pose: Pose,
    pub(super) kind: EnemyKind,
    pub(super) hover: f32,
    pub(super) phase: f32,
    /// Smoothed model heading.
    pub(super) yaw: f32,
    pub(super) group_pos: Vec3,
    pub(super) root_rot: Vec3,
    pub(super) root_scale: Vec3,
    pub(super) visible: bool,
    pub(super) base_color: Vec3,
    pub(super) mat_color: Vec3,
    pub(super) emissive: Vec3,
    pub(super) light_color: Vec3,
    pub(super) light_intensity: f32,
    pub(super) scatter: Option<Vec<(Vec3, Vec3)>>,
    /// Tepegöz's eye, lit (0 to 1) while it is within reach.
    pub(super) eye: f32,
}

impl WorldView {
    /// Drops every enemy's view (the capture's line-up of foes replaces them).
    pub fn clear_enemies(&mut self) {
        self.enemy_views.clear();
    }

    /// Adds a view for an enemy; call again for enemies that arrive mid-level. Order must match the game's list.
    pub fn add_enemy(&mut self, e: &Enemy) {
        let (rig, hover) = match e.kind() {
            EnemyKind::Skeleton => (RigId::Skeleton, 0.0),
            EnemyKind::Bat => (RigId::Bat, 0.55),
            EnemyKind::Vampire => (RigId::Vampire, 0.65),
            EnemyKind::Slime => (RigId::Slime, 0.0),
            EnemyKind::Witch => (RigId::Witch, 0.0),
            EnemyKind::Minion => (RigId::Minion, 0.0),
            EnemyKind::Helmeted => (RigId::Helmeted, 0.0),
            EnemyKind::Werewolf => (RigId::Werewolf, 0.0),
            EnemyKind::Blemmyes => (RigId::Blemmyes, 0.0),
            EnemyKind::LargeBat => (RigId::LargeBat, 0.6),
            EnemyKind::Tengu => (RigId::Tengu, 0.0),
            EnemyKind::Rasetsu => (RigId::Rasetsu, 0.0),
            EnemyKind::DireWolf => (RigId::DireWolf, 0.0),
            EnemyKind::Jorogumo => (RigId::Jorogumo, 0.0),
            EnemyKind::Cobra => (RigId::Cobra, 0.0),
            EnemyKind::Kappa => (RigId::Kappa, 0.0),
            EnemyKind::Rokurokubi => (RigId::Rokurokubi, 0.0),
            EnemyKind::Kitsune => (RigId::Kitsune, 0.0),
            EnemyKind::Oni => (RigId::Oni, 0.0),
            EnemyKind::Tulukus => (RigId::Tulukus, 0.0),
            EnemyKind::Albasti => (RigId::Albasti, 0.0),
            EnemyKind::Cin => (RigId::Cin, 0.0),
            EnemyKind::Arcura => (RigId::Arcura, 0.0),
            EnemyKind::Karakoncolos => (RigId::Karakoncolos, 0.0),
            EnemyKind::FrostGiant => (RigId::FrostGiant, 0.0),
            EnemyKind::Surtr => (RigId::Surtr, 0.0),
            EnemyKind::Nidhogg => (RigId::Nidhogg, 0.0),
            EnemyKind::Hel => (RigId::Hel, 0.0),
            EnemyKind::Tepegoz => (RigId::Tepegoz, 0.0),
            EnemyKind::Gashadokuro => (RigId::Gashadokuro, 0.0),
            EnemyKind::Ejder => (RigId::Ejder, 0.0),
            // (The ghost floats.)
            EnemyKind::Goryo => (RigId::Goryo, 0.12),
            // The realms' own: the Peerless is drawn by the Hall of Mirrors in his own likeness
            // (this is only its stand-in); Hel's servants, the gaki and the elf-maids have their own.
            EnemyKind::Peerless => (RigId::Helmeted, 0.0),
            EnemyKind::Ganglot => (RigId::Ganglot, 0.0),
            EnemyKind::Gaki => (RigId::Gaki, 0.0),
            EnemyKind::Ellepige => (RigId::Ellepige, 0.0),
            EnemyKind::Mammon => (RigId::Mammon, 0.0),
            EnemyKind::GoldGolem => (RigId::GoldGolem, 0.0),
        };
        let pose = Pose::rest(self.models.rig(rig));
        // Its levels of detail and their meshes, made now rather than in the middle of a fight
        // (once per kind: every later foe of it shares them).
        let model = self.models.rig(rig);
        for level in 0..LOD_LEVELS {
            model.lod(level).mesh();
        }
        let base = lin(e.b.color);
        self.enemy_views.push(EnemyView {
            rig,
            pose,
            kind: e.kind(),
            hover,
            phase: self.rng.nextf() * 6.0,
            yaw: 0.0,
            group_pos: Vec3::ZERO,
            root_rot: Vec3::ZERO,
            root_scale: Vec3::ONE,
            visible: false,
            base_color: base,
            mat_color: Vec3::ONE,
            emissive: Vec3::ZERO,
            light_color: base,
            light_intensity: 0.0,
            scatter: None,
            eye: 0.0,
        });
    }

    /// Starts the sword's slash animation; swings alternate forehand and backhand.
    pub fn sword_slash(&mut self) {
        self.slash_t = 0.0;
        self.slash_side = -self.slash_side;
        self.slash_timing = None;
    }

    /// A slash keyed to a move's frame data: the wind-up fills its startup, the strike its active
    /// frames, the follow-through its recovery (seconds each).
    pub fn sword_slash_timed(&mut self, startup: f32, active: f32, recovery: f32) {
        self.sword_slash();
        self.slash_timing = Some([startup.max(1e-3), active.max(1e-3), recovery.max(1e-3)]);
    }

    /// How long the slash under way lasts.
    pub(super) fn slash_len(&self) -> f32 {
        self.slash_timing.map(|[s, a, r]| s + a + r).unwrap_or(SLASH_SECONDS)
    }

    /// The slash's progress through its keys, 0 to 1: the wind-up ends at 0.2, the strike at 0.47.
    pub(super) fn slash_p(&self) -> f32 {
        let t = self.slash_t.max(0.0);
        match self.slash_timing {
            None => (t / SLASH_SECONDS).clamp(0.0, 1.0),
            Some([s, a, r]) => {
                if t < s {
                    0.2 * t / s
                } else if t < s + a {
                    0.2 + 0.27 * (t - s) / a
                } else {
                    (0.47 + 0.53 * (t - s - a) / r).min(1.0)
                }
            }
        }
    }

    pub(super) fn update_enemy(&mut self, i: usize, e: &Enemy, s: &SceneState, dt: f32, time: f32) {
        if let Some(d) = e.duel.as_deref() {
            self.update_duelist(i, e, d, s, dt, time);
            return;
        }
        let grid = s.grid;
        let frenzy = s.frenzy;
        let ep = e.pos();
        let ground = if e.m.flying {
            floor_at(grid, ep.x as f32, ep.y as f32)
        } else {
            let mut m = e.m.clone();
            m.elevation_on(grid) as f32 * STEP
        };
        {
            let v = &mut self.enemy_views[i];
            v.group_pos = Vec3::new(ep.x as f32, ground + v.hover + (time * 5.0 + i as f32 * 1.7).sin() * 0.05, ep.y as f32);
            if e.m.dir != Dir::None {
                v.yaw = lerp_angle(v.yaw, dir_to_rotation(e.m.dir), 1.0 - (-TURN_RATE_ENEMY * dt).exp());
            }
        }
        self.animate_enemy(i, e, dt, time);
        let v = &mut self.enemy_views[i];
        // Waiting in the hollow they are out of sight, except when the tour shows them off.
        let hidden = e.mode() == EnemyMode::Eaten || (e.mode() == EnemyMode::Nest && self.tour.is_none());
        v.visible = !hidden;
        if e.mode() == EnemyMode::Scattered {
            v.light_intensity = 0.6;
            v.light_color = v.base_color;
            v.mat_color = Vec3::ONE;
            v.emissive = Vec3::ZERO;
            return;
        }
        if e.is_ally() {
            // Turned by Michael's fire: washed in blue light.
            let blue = lin(Archangel::Michael.color());
            v.mat_color = Vec3::new(0.72, 0.84, 1.3);
            v.emissive = blue * (0.3 + (time * 4.0).sin() * 0.08);
            v.light_color = blue;
        } else if e.is_stunned() {
            let warn = frenzy.warning() && ((time * 8.0).floor() as i64).rem_euclid(2) == 0;
            v.mat_color = lin(if warn { STUN_WARN } else { STUN });
            v.emissive = lin(STUN) * (0.5 + (time * 12.0).sin() * 0.3);
            v.light_color = lin(STUN);
        } else {
            v.mat_color = Vec3::ONE;
            v.emissive = Vec3::ZERO;
            v.light_color = v.base_color;
        }
        v.light_intensity = if hidden { 0.0 } else { 2.5 };
    }

    /// Legs and arms swing in opposition while moving; the sword arm does the slash; a tuck in the air.
    pub(super) fn animate_hero(&mut self, hero: &Hero) {
        let rig = &self.models.hero;
        let p = &mut self.hero_pose;
        let motion = hero.motion as f32;
        let stride = self.hero_phase.sin() * motion;
        if hero.airborne() {
            let tuck = (hero.jump_progress() as f32 * PI).sin();
            p.rot_x(rig, "leftLeg", -0.9 * tuck);
            p.rot_x(rig, "rightLeg", -0.9 * tuck);
            p.rot_x(rig, "shieldArm", -0.6 * tuck);
            p.rot_x(rig, "swordArm", 0.4 * tuck);
            p.rot_x(rig, "head", -0.15 * tuck);
            self.hero_root_rot.x = 0.2 * tuck;
            self.sync_hands();
            return;
        }
        p.rot_x(rig, "leftLeg", stride * 0.75);
        p.rot_x(rig, "rightLeg", -stride * 0.75);
        // Behind a raised shield his arm comes up across his front; otherwise both arms are at
        // the ready (see `Pose::ready`), swinging with his stride.
        p.rot_x(rig, "shieldArm", if hero.blocking { -1.35 } else { SHIELD_FORWARD - stride * 0.35 });
        if let Some(i) = rig.index("shieldArm") {
            p.rot[i].y = if hero.blocking { 0.0 } else { SHIELD_TURN };
        }
        let swing = if hero.swinging() { (hero.swing_progress() as f32 * PI).sin() * 1.7 } else { SWORD_FORWARD + stride * 0.35 };
        p.rot_x(rig, "swordArm", swing);
        p.rot_x(rig, "head", motion * 0.08 + if hero.swinging() { 0.15 } else { 0.0 });
        self.hero_root_rot.x = motion * if hero.hasted() { 0.14 } else { 0.07 };
        self.hero_root_rot.y = 0.0;
        self.hero_lunge = 0.0;
        if self.slash_t >= 0.0 {
            self.pose_slash();
        }
        self.sync_hands();
    }

    /// The weapons (layered parts of his rig) move with the arms that hold them.
    pub(super) fn sync_hands(&mut self) {
        let rig = &self.models.hero;
        for (arm, held) in [("swordArm", "blade"), ("shieldArm", "offhand"), ("shieldArm", "shield")] {
            if let (Some(a), Some(b)) = (rig.index(arm), rig.index(held)) {
                self.hero_pose.rot[b] = self.hero_pose.rot[a];
            }
        }
    }

    /**
     * The sword slash, keyed like a hand animation: a quick wind-up that lifts
     * the blade back over the shoulder, a fast strike that sweeps it across
     * and down with the hips turning into it and a small lunge, then an eased
     * follow-through back to guard. Mirrored on alternate swings.
     */
    pub(super) fn pose_slash(&mut self) {
        // (time, arm pitch, arm roll, body twist, lunge); roll and twist mirror with the side.
        const KEYS: [(f32, f32, f32, f32, f32); 4] =
            [(0.0, 0.0, 0.0, 0.0, 0.0), (0.2, -2.3, 0.55, 0.38, -0.03), (0.47, 1.55, -0.75, -0.5, 0.1), (1.0, 0.0, 0.0, 0.0, 0.0)];
        let p = self.slash_p();
        let k = KEYS.iter().rposition(|k| k.0 <= p).unwrap_or(0).min(KEYS.len() - 2);
        let (a, b) = (KEYS[k], KEYS[k + 1]);
        let u = ((p - a.0) / (b.0 - a.0)).clamp(0.0, 1.0);
        // The strike snaps (ease-out cubic); wind-up and recovery ease in and out. The blade
        // follows through past the strike's end pose before it comes back (WARBAND.md §4.5).
        let mut e = if k == 1 { 1.0 - (1.0 - u).powi(3) } else { u * u * (3.0 - 2.0 * u) };
        if k == 2 && self.juice.off & crate::render::profile::fx::SPRINGS == 0 {
            e -= crate::logic::juice::BLADE_OVERSHOOT * (PI * (u / 0.4).min(1.0)).sin();
        }
        let mix = |x: f32, y: f32| x + (y - x) * e;
        let side = self.slash_side;
        let rig = &self.models.hero;
        if let Some(i) = rig.index("swordArm") {
            self.hero_pose.rot[i].x = mix(a.1, b.1);
            self.hero_pose.rot[i].z = mix(a.2, b.2) * side;
        }
        let guard = (p * PI).sin();
        self.hero_pose.rot_x(rig, "shieldArm", -0.55 * guard);
        self.hero_pose.rot_x(rig, "head", 0.12 * guard);
        self.hero_root_rot.y = mix(a.3, b.3) * side;
        self.hero_lunge = mix(a.4, b.4);
    }

    pub(super) fn animate_enemy(&mut self, i: usize, e: &Enemy, dt: f32, time: f32) {
        if e.mode() == EnemyMode::Scattered {
            self.animate_scatter(i, e, dt);
            return;
        }
        let models = &self.models;
        let v = &mut self.enemy_views[i];
        let rig = models.rig(v.rig);
        if v.scatter.is_some() {
            // Reassembled: snap every bone home.
            v.pose = Pose::rest(rig);
            v.scatter = None;
        }
        // (A duelist walks freely: its gait follows its velocity.)
        let (moving, pace) = match e.duel.as_deref() {
            Some(d) => {
                let sp = (d.vel.x * d.vel.x + d.vel.y * d.vel.y).sqrt() as f32;
                (sp > 0.3, sp)
            }
            None => (e.m.dir != Dir::None && e.mode() != EnemyMode::Nest, e.m.speed as f32),
        };
        if moving {
            // (A giant's legs are long: it steps slower for the same pace.)
            let cadence = if crate::logic::combat::Breed::of(v.kind).radius > 0.7 { 0.55 } else { 1.0 };
            v.phase += dt * pace * 2.2 * cadence;
        }
        match v.kind {
            EnemyKind::Skeleton
            | EnemyKind::Helmeted
            | EnemyKind::Witch
            | EnemyKind::Blemmyes
            | EnemyKind::Tengu
            | EnemyKind::Rasetsu
            | EnemyKind::Oni
            | EnemyKind::Kappa
            | EnemyKind::Karakoncolos
            | EnemyKind::Albasti
            | EnemyKind::Cin
            | EnemyKind::Arcura
            | EnemyKind::Tepegoz
            | EnemyKind::Gashadokuro
            | EnemyKind::FrostGiant
            | EnemyKind::Surtr
            | EnemyKind::Hel
            | EnemyKind::Peerless
            | EnemyKind::Ganglot
            | EnemyKind::Gaki
            | EnemyKind::Ellepige
            | EnemyKind::Mammon
            | EnemyKind::GoldGolem => {
                // A walk: the blemmyes, the rasetsu and the oni lumber, the witch and the tengu stride,
                // the kappa waddles, the skeletons stalk.
                let (stride, swing) = match v.kind {
                    EnemyKind::Blemmyes | EnemyKind::Rasetsu | EnemyKind::Oni | EnemyKind::Karakoncolos => (0.6, 0.35),
                    EnemyKind::Tepegoz | EnemyKind::Gashadokuro | EnemyKind::FrostGiant | EnemyKind::Surtr => (0.5, 0.3),
                    // (Mammon waddles under his robe and his belly; the golems stump.)
                    EnemyKind::Mammon => (0.3, 0.16),
                    EnemyKind::GoldGolem => (0.5, 0.3),
                    EnemyKind::Witch | EnemyKind::Tengu | EnemyKind::Arcura => (0.7, 0.3),
                    EnemyKind::Albasti => (0.6, 0.22),
                    // (Under a long robe: small steps.)
                    EnemyKind::Cin | EnemyKind::Hel | EnemyKind::Goryo | EnemyKind::Ellepige => (0.25, 0.15),
                    // (Hel's servants shuffle.)
                    EnemyKind::Ganglot => (0.3, 0.12),
                    EnemyKind::Kappa => (0.5, 0.2),
                    // (The gaki shamble on their sticks of legs, grasping hands held out.)
                    EnemyKind::Gaki => (0.45, 0.12),
                    _ => (0.85, 0.5),
                };
                let s = v.phase.sin();
                v.pose.rot_x(rig, "leftLeg", s * stride);
                v.pose.rot_x(rig, "rightLeg", -s * stride);
                v.pose.rot_x(rig, "leftArm", -s * swing);
                v.pose.rot_x(rig, "rightArm", s * swing);
                v.pose.rot_x(rig, "head", (v.phase * 2.0).sin() * 0.1);
                v.root_rot.z = v.phase.sin()
                    * match v.kind {
                        EnemyKind::Blemmyes | EnemyKind::Rasetsu | EnemyKind::Oni => 0.1,
                        EnemyKind::Kappa => 0.14,
                        EnemyKind::Gaki => 0.12,
                        _ => 0.06,
                    };
                if let Some(k) = rig.index("tail") {
                    v.pose.rot[k].y = (time * 2.6 + i as f32).sin() * 0.3;
                }
                // The water in the kappa's dish goes with its head; the gashadokuro's jaw too, hanging
                // open and rattling.
                follow(rig, &mut v.pose, "head", "water");
                follow(rig, &mut v.pose, "head", "jaw");
                if let Some(j) = rig.index("jaw") {
                    v.pose.rot[j].x += 0.14 + (time * 7.0 + i as f32).sin().abs() * 0.08;
                }
            }
            EnemyKind::Goryo => {
                // It floats, bobbing; its sleeves drift, its head tilts.
                let t = time * 1.2 + i as f32;
                v.group_pos.y += t.sin() * 0.05;
                v.pose.rot_x(rig, "leftArm", (t * 0.8).sin() * 0.08 - 0.05);
                v.pose.rot_x(rig, "rightArm", (t * 0.8 + 1.3).sin() * 0.08 - 0.05);
                v.pose.rot_x(rig, "head", (t * 0.6).sin() * 0.05);
                v.root_rot.z = (t * 0.5).sin() * 0.04;
                v.root_rot.x = if moving { 0.12 } else { 0.0 };
            }
            EnemyKind::Ejder => {
                // Seven necks weaving out of time with one another; the legs plod; the tail sweeps.
                let t = time * 1.3 + i as f32;
                for k in 0..7 {
                    if let Some(n) = rig.index(EJDER_NECKS[k]) {
                        let p = t + k as f32 * 0.9;
                        v.pose.rot[n] = Vec3::new(p.sin() * 0.08, 0.0, (p * 0.7).sin() * 0.06);
                    }
                }
                let s = v.phase.sin();
                let step = if moving { 0.4 } else { 0.0 };
                v.pose.rot_x(rig, "frontLeftLeg", s * step);
                v.pose.rot_x(rig, "backRightLeg", s * step);
                v.pose.rot_x(rig, "frontRightLeg", -s * step);
                v.pose.rot_x(rig, "backLeftLeg", -s * step);
                if let Some(k) = rig.index("tail") {
                    v.pose.rot[k].y = (time * 1.6 + i as f32).sin() * 0.25;
                }
                v.root_rot.z = if moving { s * 0.04 } else { 0.0 };
            }
            EnemyKind::Rokurokubi => {
                // She kneels still; her long neck weaves, her head turning on it.
                let t = time * 1.1 + i as f32;
                for name in ["neck", "head"] {
                    if let Some(k) = rig.index(name) {
                        v.pose.pos[k] = rig.parts[k].rest;
                    }
                }
                if let Some(k) = rig.index("neck") {
                    v.pose.rot[k] = Vec3::new((t * 0.7).sin() * 0.04, 0.0, t.sin() * 0.06);
                }
                if let Some(k) = rig.index("head") {
                    v.pose.rot[k] = Vec3::new((t * 0.5).sin() * 0.06, (t * 0.8).sin() * 0.15, -t.sin() * 0.08);
                }
                v.pose.rot_x(rig, "leftArm", (time * 0.9).sin() * 0.04);
                v.pose.rot_x(rig, "rightArm", (time * 0.9 + 1.0).sin() * 0.04);
                v.root_rot.z = if moving { (v.phase * 2.0).sin() * 0.04 } else { 0.0 };
            }
            EnemyKind::Minion => {
                // Scurries in hops, its hat bobbing.
                let t = if moving { v.phase * 1.6 } else { time * 3.0 + i as f32 };
                let hop = t.sin();
                v.root_scale = Vec3::new(1.0 - hop * 0.06, 1.0 + hop * 0.1, 1.0 - hop * 0.06);
                v.root_rot.z = (t * 0.5).sin() * 0.12;
                if moving {
                    v.group_pos.y += hop.max(0.0) * 0.12;
                }
            }
            EnemyKind::Werewolf | EnemyKind::DireWolf | EnemyKind::Kitsune => {
                // A gallop: the front legs together, the back legs half a beat behind, the head low, the tail streaming.
                let t = v.phase * 1.2;
                let (front, back) = (t.sin(), (t - 1.2).sin());
                v.pose.rot_x(rig, "frontLeftLeg", front * 0.7);
                v.pose.rot_x(rig, "frontRightLeg", (t + 0.4).sin() * 0.7);
                v.pose.rot_x(rig, "backLeftLeg", -back * 0.7);
                v.pose.rot_x(rig, "backRightLeg", -(t - 0.8).sin() * 0.7);
                v.pose.rot_x(rig, "head", if moving { 0.15 + front * 0.08 } else { (time * 1.5).sin() * 0.05 });
                if let Some(k) = rig.index("tail") {
                    v.pose.rot[k].y = (time * 6.0 + i as f32).sin() * 0.3;
                }
                v.root_rot.x = if moving { front * 0.05 } else { 0.0 };
                if moving {
                    v.group_pos.y += front.max(0.0) * 0.08;
                }
            }
            EnemyKind::Tulukus => {
                // The owl stands and stalks on its talons, wings folded, its head turning on its neck.
                let t = if moving { v.phase * 1.4 } else { time * 0.8 + i as f32 };
                v.pose.rot_x(rig, "leftLeg", if moving { t.sin() * 0.45 } else { 0.0 });
                v.pose.rot_x(rig, "rightLeg", if moving { -t.sin() * 0.45 } else { 0.0 });
                if let Some(h) = rig.index("head") {
                    v.pose.rot[h] = Vec3::new(0.0, (time * 0.5 + i as f32).sin() * 0.6, 0.0);
                }
                for (name, side) in [("leftWing", -1.0f32), ("rightWing", 1.0)] {
                    if let Some(w) = rig.index(name) {
                        v.pose.rot[w] = Vec3::new(0.0, 0.0, side * (time * 2.0 + i as f32).sin().max(0.0) * 0.05);
                    }
                }
                v.root_rot.z = if moving { t.sin() * 0.08 } else { 0.0 };
            }
            EnemyKind::Bat | EnemyKind::Vampire | EnemyKind::LargeBat => {
                let beat = time * if v.kind == EnemyKind::LargeBat { 9.0 } else { 15.0 } + i as f32 * 2.0;
                let flap = beat.sin() * 0.8;
                if let Some(l) = rig.index("leftWing") {
                    v.pose.rot[l].z = -flap;
                }
                if let Some(r) = rig.index("rightWing") {
                    v.pose.rot[r].z = flap;
                }
                v.root_rot.x = if moving { 0.3 } else { 0.05 };
                v.group_pos.y += beat.sin() * 0.035;
            }
            EnemyKind::Jorogumo => {
                // Eight legs in two alternating fours, each lifting as it swings forward; her body
                // sways over them, her arms poised.
                let t = if moving { v.phase * 1.7 } else { time * 0.9 + i as f32 };
                for (k, name) in crate::render::foes3::JORO_LEGS.iter().enumerate() {
                    let side = if k.is_multiple_of(2) { -1.0 } else { 1.0 };
                    let ph = t + ((k / 2 + k % 2) % 2) as f32 * PI;
                    let (swing, lift) = if moving { (ph.sin() * 0.32, ph.cos().max(0.0) * 0.22) } else { ((ph * 0.5).sin() * 0.03, 0.0) };
                    if let Some(j) = rig.index(name) {
                        v.pose.rot[j].y = swing;
                        v.pose.rot[j].z = side * lift;
                    }
                }
                v.pose.rot_x(rig, "head", (time * 1.3 + i as f32).sin() * 0.05);
                v.pose.rot_x(rig, "leftArm", (time * 1.1).sin() * 0.08);
                v.pose.rot_x(rig, "rightArm", (time * 1.1 + 1.0).sin() * 0.08);
                v.root_rot.z = if moving { (t * 2.0).sin() * 0.03 } else { 0.0 };
                if moving {
                    v.group_pos.y += (t * 2.0).sin().abs() * 0.015;
                }
            }
            EnemyKind::Nidhogg => {
                // The dragon sways on its coil, its neck weaving, its head riding on it; its tail lashes.
                let t = if moving { v.phase * 1.1 } else { time * 1.1 + i as f32 };
                if let Some(k) = rig.index("neck") {
                    v.pose.rot[k] = Vec3::new((t * 0.5).sin() * 0.04, 0.0, t.sin() * if moving { 0.1 } else { 0.06 });
                }
                if let Some(k) = rig.index("head") {
                    v.pose.pos[k] = rig.parts[k].rest;
                }
                follow(rig, &mut v.pose, "neck", "head");
                if let Some(k) = rig.index("head") {
                    v.pose.rot[k] += Vec3::new((t * 0.7).sin() * 0.06, 0.0, -t.sin() * 0.05);
                }
                if let Some(k) = rig.index("tail") {
                    v.pose.rot[k].y = (time * 2.0 + i as f32).sin() * 0.3;
                }
                v.root_rot.x = 0.0;
                v.root_rot.y = if moving { (t * 0.8).sin() * 0.1 } else { 0.0 };
            }
            EnemyKind::Cobra => {
                // It sways where it stands, hood and head weaving; moving, it slides from side to side.
                let t = if moving { v.phase * 1.3 } else { time * 1.5 + i as f32 };
                if let Some(k) = rig.index("neck") {
                    v.pose.rot[k] = Vec3::new(0.0, 0.0, t.sin() * if moving { 0.12 } else { 0.07 });
                }
                if let Some(k) = rig.index("head") {
                    v.pose.rot[k] = Vec3::new((t * 0.5).sin() * 0.05, 0.0, -t.sin() * 0.09);
                }
                if let Some(k) = rig.index("tail") {
                    v.pose.rot[k].y = (time * 2.5 + i as f32).sin() * 0.3;
                }
                v.root_rot.x = 0.0;
                v.root_rot.y = if moving { (t * 0.8).sin() * 0.12 } else { 0.0 };
            }
            EnemyKind::Slime => {
                let t = if moving { v.phase } else { time * 2.5 + i as f32 };
                let sq = t.sin();
                v.root_scale = Vec3::new(1.0 - sq * 0.14, 1.0 + sq * 0.3, 1.0 - sq * 0.14);
                if moving {
                    v.group_pos.y += sq.max(0.0) * 0.2;
                }
                v.root_rot.x = if moving { 0.12 } else { 0.0 };
            }
        }
    }

    /// Bones burst outward and tumble to the floor, lie still, then glide back to their rest pose.
    pub(super) fn animate_scatter(&mut self, i: usize, e: &Enemy, dt: f32) {
        let models = &self.models;
        let rng = &mut self.rng;
        let v = &mut self.enemy_views[i];
        let rig = models.rig(v.rig);
        v.root_rot = Vec3::ZERO;
        v.root_scale = Vec3::ONE;
        if v.scatter.is_none() {
            v.scatter = Some(
                rig.parts
                    .iter()
                    .map(|p| {
                        let a = rng.nextf() * TAU;
                        let sp = if p.name == "body" { 0.3 } else { 0.8 + rng.nextf() * 1.1 };
                        (
                            Vec3::new(a.cos() * sp, 2.5 + rng.nextf() * 2.5, a.sin() * sp),
                            Vec3::new((rng.nextf() - 0.5) * 16.0, (rng.nextf() - 0.5) * 16.0, (rng.nextf() - 0.5) * 16.0),
                        )
                    })
                    .collect(),
            );
        }
        let progress = e.scatter_progress();
        let scatter = v.scatter.as_mut().unwrap();
        if progress < REASSEMBLE_START {
            for (pi, part) in rig.parts.iter().enumerate() {
                let (vel, spin) = &mut scatter[pi];
                let floor_y = -part.min_y + 0.02;
                vel.y -= GRAVITY * dt;
                v.pose.pos[pi] += *vel * dt;
                if v.pose.pos[pi].y < floor_y {
                    v.pose.pos[pi].y = floor_y;
                    vel.y *= -0.3;
                    vel.x *= 0.5;
                    vel.z *= 0.5;
                    *spin *= 0.4;
                }
                v.pose.rot[pi] += *spin * dt;
            }
        } else {
            let t = ((progress - REASSEMBLE_START) / (1.0 - REASSEMBLE_START)) as f32;
            let k = 1.0 - (-dt * (6.0 + t * 30.0)).exp();
            for (pi, part) in rig.parts.iter().enumerate() {
                v.pose.pos[pi] = v.pose.pos[pi].lerp(part.rest, k);
                v.pose.rot[pi] *= 1.0 - k;
            }
            v.root_rot.z = (t * 60.0).sin() * 0.04 * (1.0 - t);
        }
    }
}

/// `child` (a part sculpted apart from `parent` but riding on it: the water in the kappa's dish)
/// turned with `parent` about `parent`'s joint.
pub(super) fn follow(rig: &Rig, pose: &mut Pose, parent: &str, child: &str) {
    let (Some(p), Some(c)) = (rig.index(parent), rig.index(child)) else { return };
    let r = Quat::from_euler(EulerRot::XYZ, pose.rot[p].x, pose.rot[p].y, pose.rot[p].z);
    pose.pos[c] = pose.pos[p] + r * (rig.parts[c].rest - rig.parts[p].rest);
    pose.rot[c] = pose.rot[p];
}

/// The seven-headed dragon's necks, left to right.
pub(super) const EJDER_NECKS: [&str; 7] = ["neck0", "neck1", "neck2", "neck3", "neck4", "neck5", "neck6"];
