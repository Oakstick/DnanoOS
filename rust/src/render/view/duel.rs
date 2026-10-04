//! The Sparring Ring's duelists on screen (COMBAT.md §1.3): each stands where its mind put it and
//! faces its own way; its move plays on top of its gait (the wind-up is the tell, the strike
//! snaps, the recovery hangs open), its colours tell its state, and the floor shows where its
//! attack will land, filling as the startup runs and flashing in the commit frames: a cone (a
//! small one at a body foe's jaws: it bites from beside him), a fan, a line the way a bolt will
//! fly, a ring on the floor where a burrow will burst up (COMBAT.md §11). Token holders wear a red
//! pip; every caster's bolt has its own look (`BoltLook`): the bog slime's dripping glob of spit,
//! the witch's violet hex star, the jorogumo's ball of fire, the kitsune's foxfire wisps, the cin's
//! tall smokeless flame, the arçura's thorny root dart, the goryō's jagged lightning, Hel's cold
//! drain with its motes; gold, once parried back. The legends leave their marks for a moment
//! (lightning, roots, ice, an eruption, smoke, a splash, the ground shaking, a grave), and Surtr's
//! fire burns on.

use super::animation::{follow, EJDER_NECKS};
use super::*;
use crate::logic::combat::ails::{Flash, FlashKind};
use crate::logic::combat::duelist::{Bolt, BoltLook, ALOFT, FIRE_TINT};
use crate::logic::combat::frames::Shape;
use crate::logic::combat::fsm::PARRY_FROM;
use crate::logic::combat::{Archetype, Duelist, MoveId, Phase, Reel, State};

/// The commit flash: the last frames of a startup, when the aim no longer follows.
const COMMIT_FRAMES: u16 = 8;

/// An archetype's telegraph colour.
fn tell(arch: Archetype) -> u32 {
    match arch {
        Archetype::Brute => 0xff5a2a,
        Archetype::Skirmisher => 0xff3a6a,
        Archetype::Hexer => 0xb066ff,
        Archetype::Prince => 0xffa030,
    }
}

/// A duelist's telegraph colour: its archetype's, the jorogumo's fire, or what its move is: the
/// legends' in their own colours (roots green, lightning pale, the owl's talons amber, the burrow
/// sickly, ice cyan), a grab no shield holds in bone white.
fn tell_of(d: &Duelist) -> u32 {
    match d.f.attack().map(|a| a.0) {
        Some(MoveId::Roots) => return 0x6ad050,
        Some(MoveId::Thunder) => return 0xa8c8ff,
        Some(MoveId::Talons) => return 0xffb060,
        Some(MoveId::Burrow) => return 0x9ad04a,
        Some(MoveId::FrostSlam) => return 0x9adcff,
        Some(MoveId::Grab | MoveId::Grasp) => return 0xe8f0d0,
        Some(MoveId::CoinToss | MoveId::CoinRain | MoveId::CoinVolley) => return 0xffd860,
        Some(MoveId::Call) => return 0x9ac8ff,
        Some(MoveId::Raise) => return 0x6a8cff,
        Some(MoveId::Swarm | MoveId::Scatter | MoveId::BileSpew) => return 0x9ab030,
        _ => {}
    }
    if d.breed.kind == EnemyKind::Jorogumo {
        FIRE_TINT
    } else {
        tell(d.arch)
    }
}

/// How high over the floor a third-stage foe's head is (the others keep their archetype's
/// heights): its token pip floats there, its bars over that.
pub fn crown(kind: EnemyKind) -> Option<f32> {
    match kind {
        EnemyKind::Tengu => Some(1.2),
        EnemyKind::Rasetsu => Some(1.5),
        EnemyKind::DireWolf => Some(0.9),
        EnemyKind::Jorogumo => Some(1.05),
        EnemyKind::Cobra => Some(1.08),
        EnemyKind::Kappa => Some(0.98),
        EnemyKind::Rokurokubi => Some(1.75),
        EnemyKind::Kitsune => Some(0.92),
        EnemyKind::Oni => Some(1.78),
        EnemyKind::Tulukus => Some(1.05),
        EnemyKind::Albasti => Some(1.42),
        EnemyKind::Cin => Some(1.36),
        EnemyKind::Arcura => Some(1.6),
        EnemyKind::Karakoncolos => Some(1.75),
        EnemyKind::FrostGiant => Some(2.82),
        EnemyKind::Surtr => Some(3.0),
        EnemyKind::Nidhogg => Some(2.05),
        EnemyKind::Hel => Some(1.88),
        EnemyKind::Tepegoz => Some(2.72),
        EnemyKind::Gashadokuro => Some(3.05),
        EnemyKind::Ejder => Some(2.3),
        EnemyKind::Goryo => Some(1.62),
        EnemyKind::Mammon => Some(3.25),
        EnemyKind::GoldGolem => Some(1.7),
        // Pride's court
        EnemyKind::Lucifer => Some(3.25),
        EnemyKind::Reflection => Some(1.5),
        // Envy's court
        EnemyKind::Leviathan => Some(3.25),
        EnemyKind::Serpentling => Some(1.5),
        // Wrath's court
        EnemyKind::Amon => Some(3.25),
        EnemyKind::Raven => Some(1.5),
        // Sloth's court
        EnemyKind::Belphegor => Some(3.25),
        EnemyKind::Nightmare => Some(1.5),
        // Gluttony's court
        EnemyKind::Beelzebub => Some(3.25),
        EnemyKind::FlySwarm => Some(1.5),
        // Lust's court
        EnemyKind::Asmodeus => Some(3.25),
        _ => None,
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
        self.overshoot_duel(i, d);
        let v = &mut self.enemy_views[i];
        v.visible = d.alive() && d.lore.vanish == 0;
        let glow = lin(tell_of(d));
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
        if d.arch == Archetype::Prince {
            // A prince's tell is his pose and the floor's: a glow from within would wash out a figure
            // so large and dark, so only a breath of it shows.
            v.emissive *= 0.02;
            v.light_intensity = 0.0;
        }
        if d.flash > 0 {
            let k = d.flash as f32 / 6.0;
            v.emissive += Vec3::splat(1.3 * k);
            v.mat_color *= 1.0 + 0.4 * k;
        }
        // Tepegöz's eye lights up while it is within reach; the goryō is a cold ghost but in its wrath.
        v.eye = if d.breed.power == crate::logic::combat::archetype::Power::Ward && d.exposed() { 1.0 } else { 0.0 };
        if d.breed.power == crate::logic::combat::archetype::Power::Spirit {
            if d.untouchable() {
                v.mat_color *= Vec3::new(0.62, 0.72, 1.0);
                v.emissive += lin(0x4a6aff) * (0.12 + (time * 2.0 + i as f32).sin().abs() * 0.06);
            } else {
                v.emissive += lin(0xa8c8ff) * (0.3 + (time * 30.0).sin().abs() * 0.2);
            }
        }
        if d.lore.illusion {
            // An illusion casts no light, and wavers.
            v.light_intensity = 0.0;
            v.mat_color *= 0.86 + 0.14 * (time * 9.0 + i as f32).sin();
        }
        if d.breed.power == crate::logic::combat::archetype::Power::Shade
            && !matches!(d.f.attack(), Some((MoveId::Smother, Phase::Startup | Phase::Active, _)))
        {
            // The albastı: a shadow, red eyes in it, until she falls on him.
            v.mat_color *= 0.16;
            v.light_color = lin(0xc0283a);
            v.light_intensity = 1.2;
        }
        if d.f.attack().is_some_and(|a| a.0 == MoveId::Burrow) {
            // Níðhöggr under the floor: nothing shows but the earth heaving over it.
            let f = d.f.attack().map(|a| a.2).unwrap_or(0);
            let burrow = MoveId::Burrow.data();
            if f >= crate::logic::combat::duelist::BURROW_SINK && f < burrow.startup {
                v.visible = false;
                v.light_intensity = 0.0;
            }
        }
    }

    /// Its move or its reel, over its gait.
    fn pose_duel(&mut self, i: usize, d: &Duelist, time: f32) {
        let rig = self.models.rig(self.enemy_views[i].rig);
        let v = &mut self.enemy_views[i];
        let cobra = v.kind == EnemyKind::Cobra;
        v.root_rot.y = 0.0;
        v.root_scale = Vec3::ONE;
        if let Some(w) = rig.index("water").filter(|_| d.lore.spilled > 0) {
            // The kappa's dish spilled: no water in it until it fills again.
            v.pose.pos[w].y -= 0.3;
        }
        if d.begging() {
            // The dragon felled, begging: slumped over, its heads hung to the floor.
            for name in EJDER_NECKS {
                v.pose.rot_x(rig, name, 1.0);
            }
            v.root_rot = Vec3::new(0.12, 0.0, 0.1 + (time * 3.0).sin() * 0.02);
            return;
        }
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
                    MoveId::Shove => {
                        // Down in a sumo crouch, then the shove: leaning in, both palms thrust out.
                        if strike == 0.0 {
                            v.root_scale = Vec3::new(1.0 + 0.06 * wind, 1.0 - 0.16 * wind, 1.0);
                            v.root_rot.x = 0.18 * wind;
                            v.pose.rot_x(rig, "leftArm", -0.9 * wind);
                            v.pose.rot_x(rig, "rightArm", -0.9 * wind);
                        } else {
                            let out = 1.0 - rec;
                            v.root_rot.x = 0.3 * out;
                            v.pose.rot_x(rig, "leftArm", -1.6 * out);
                            v.pose.rot_x(rig, "rightArm", -1.6 * out);
                            v.pose.rot_x(rig, "leftLeg", 0.5 * out);
                            v.pose.rot_x(rig, "rightLeg", -0.6 * out);
                        }
                        follow(rig, &mut v.pose, "head", "water");
                    }
                    MoveId::NeckBite => {
                        // Her neck draws back, then bows forward a little way and her head bites:
                        // it stays on her neck, and her neck its own length.
                        let bow = if strike == 0.0 { -0.18 * wind } else { 0.42 * snap * (1.0 - rec) };
                        v.pose.rot_x(rig, "neck", bow);
                        let head = rig.index("head").map(|h| v.pose.rot[h]).unwrap_or_default();
                        follow(rig, &mut v.pose, "neck", "head");
                        if let Some(h) = rig.index("head") {
                            v.pose.rot[h] += head + Vec3::new(0.35 * snap * (1.0 - rec), 0.0, 0.0);
                        }
                    }
                    MoveId::Foxfire => {
                        // The fox's head lifts and its tail flicks as the foxfire gathers, then it looses.
                        v.pose.rot_x(rig, "head", -0.35 * wind * (1.0 - rec));
                        if let Some(t) = rig.index("tail") {
                            v.pose.rot[t].x = -0.5 * wind * (1.0 - rec) + 0.4 * snap * (1.0 - rec);
                        }
                    }
                    MoveId::Slam | MoveId::Kanabo | MoveId::FrostSlam => {
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
                    MoveId::Talons => {
                        // Straight up on spread wings over him, out of reach, and straight down talons first.
                        let u = (f / s).min(1.0);
                        let (up, down) = (ALOFT.0 as f32 + 6.0, ALOFT.1 as f32);
                        if strike == 0.0 {
                            let high = if f < down { ease(f / up) } else { 1.0 - ease((f - down) / (s - down)) };
                            v.group_pos.y += 1.5 * high;
                            v.root_rot.x = if f >= down { 0.6 } else { -0.15 };
                        }
                        let spread = if rec > 0.0 { 1.35 * (1.0 - rec) } else { 1.35 + (time * 16.0).sin() * 0.35 * (1.0 - u) };
                        for (name, side) in [("leftWing", -1.0f32), ("rightWing", 1.0)] {
                            if let Some(w) = rig.index(name) {
                                v.pose.rot[w] = Vec3::new(0.0, 0.0, side * spread);
                            }
                        }
                        v.pose.rot_x(rig, "leftLeg", -0.8 * wind * (1.0 - rec));
                        v.pose.rot_x(rig, "rightLeg", -0.8 * wind * (1.0 - rec));
                    }
                    MoveId::Smother => {
                        // She rears up tall beside him, arms spread over him, then falls forward on him.
                        if strike == 0.0 {
                            v.root_scale = Vec3::new(1.0 - 0.04 * wind, 1.0 + 0.12 * wind, 1.0);
                            v.root_rot.x = -0.2 * wind;
                            v.pose.rot_x(rig, "leftArm", -2.4 * wind);
                            v.pose.rot_x(rig, "rightArm", -2.4 * wind);
                        } else {
                            let out = 1.0 - rec;
                            v.root_rot.x = -0.2 + 0.65 * snap * out + 0.2 * rec;
                            v.pose.rot_x(rig, "leftArm", (-2.4 + 1.0 * snap) * out);
                            v.pose.rot_x(rig, "rightArm", (-2.4 + 1.0 * snap) * out);
                        }
                    }
                    MoveId::Roots => {
                        // Grown as tall as a tree to call the roots, arms raised; then back to his size.
                        let grow = 1.0 + 0.45 * wind * (1.0 - rec);
                        v.root_scale = Vec3::splat(grow);
                        v.pose.rot_x(rig, "leftArm", -2.4 * wind * (1.0 - rec));
                        v.pose.rot_x(rig, "rightArm", -2.4 * wind * (1.0 - rec));
                    }
                    MoveId::Call => {
                        // Its head thrown back, arms spread wide: the call in a loved one's voice.
                        let open = wind * (1.0 - rec);
                        v.pose.rot_x(rig, "head", -0.45 * open);
                        if let Some(l) = rig.index("leftArm") {
                            v.pose.rot[l].z = -0.9 * open;
                        }
                        if let Some(r) = rig.index("rightArm") {
                            v.pose.rot[r].z = 0.9 * open;
                        }
                    }
                    MoveId::Burrow => {
                        // Down into the floor, along under it, and up out of it beneath him.
                        let sink = crate::logic::combat::duelist::BURROW_SINK as f32;
                        if f < sink {
                            v.group_pos.y -= 2.2 * (f / sink);
                            v.root_rot.x = 0.5 * (f / sink);
                        } else if strike > 0.0 && rec == 0.0 {
                            v.group_pos.y -= 1.6 * (1.0 - snap);
                            v.pose.rot_x(rig, "neck", -0.3 * (1.0 - snap));
                        }
                    }
                    MoveId::Bite if cobra || v.kind == EnemyKind::Nidhogg => {
                        if strike == 0.0 {
                            // Drawn back on its coil, the hood high.
                            v.pose.rot_x(rig, "neck", -0.4 * wind);
                            v.pose.rot_x(rig, "head", 0.25 * wind);
                            v.root_scale = Vec3::new(1.0, 1.0 - 0.06 * wind, 1.0);
                        } else {
                            // The strike: down off the coil onto him, its length its own.
                            let out = snap * (1.0 - rec);
                            v.pose.rot_x(rig, "neck", 0.85 * out);
                            v.pose.rot_x(rig, "head", -0.3 * out);
                        }
                        if v.kind == EnemyKind::Nidhogg {
                            let head = rig.index("head").map(|h| v.pose.rot[h]).unwrap_or_default();
                            follow(rig, &mut v.pose, "neck", "head");
                            if let Some(h) = rig.index("head") {
                                v.pose.rot[h] += head;
                            }
                        }
                    }
                    MoveId::Bite => {
                        if strike == 0.0 {
                            // Hackles up, head drawn back, weight on the haunches.
                            v.root_scale = Vec3::new(1.0 + 0.05 * wind, 1.0 - 0.12 * wind, 1.0);
                            v.root_rot.x = -0.12 * wind;
                            v.pose.rot_x(rig, "head", -0.35 * wind);
                        } else {
                            // The snap of the jaws where it stands: the head thrown down at him.
                            let out = snap * (1.0 - rec);
                            v.root_rot.x = 0.2 * out;
                            v.pose.rot_x(rig, "frontLeftLeg", -0.5 * out);
                            v.pose.rot_x(rig, "frontRightLeg", -0.5 * out);
                            v.pose.rot_x(rig, "head", 0.45 * out);
                        }
                    }
                    MoveId::Grab => {
                        // Two great hands lifted and reaching, the jaw gaping; then they close on him.
                        let reach = if strike == 0.0 { -1.3 * wind } else { -1.3 + 0.5 * snap };
                        let hold = if rec > 0.0 { 1.0 - rec * 0.6 } else { 1.0 };
                        v.pose.rot_x(rig, "leftArm", reach * hold);
                        v.pose.rot_x(rig, "rightArm", reach * hold);
                        if let (Some(l), Some(r)) = (rig.index("leftArm"), rig.index("rightArm")) {
                            v.pose.rot[l].z = 0.3 * snap * hold;
                            v.pose.rot[r].z = -0.3 * snap * hold;
                        }
                        follow(rig, &mut v.pose, "head", "jaw");
                        if let Some(j) = rig.index("jaw") {
                            v.pose.rot[j].x += 0.6 * wind * (1.0 - rec) + if rec > 0.0 { (time * 12.0).sin().abs() * 0.3 } else { 0.0 };
                        }
                    }
                    MoveId::Heads => {
                        // Every head drawn back, then the heads strike in turn, two by two, the middle last.
                        const BITES: [&[usize]; 4] = [&[2, 4], &[1, 5], &[0, 6], &[3]];
                        let per = (a / 4.0).max(1.0);
                        for (b, necks) in BITES.iter().enumerate() {
                            let start = s + b as f32 * per;
                            let k = if f < start { 0.0 } else { ((f - start) / per).min(2.0) };
                            let out = if k <= 1.0 { k } else { 2.0 - k };
                            for &n in necks.iter() {
                                v.pose.rot_x(rig, EJDER_NECKS[n], -0.35 * wind * (1.0 - out) + 0.9 * out * (1.0 - rec));
                            }
                        }
                    }
                    MoveId::Thunder => {
                        // Both hands raised to the sky as the storm gathers; flung down as it strikes.
                        let up = if strike == 0.0 { -2.7 * wind } else { -2.7 + 1.6 * snap };
                        v.pose.rot_x(rig, "leftArm", up * (1.0 - rec));
                        v.pose.rot_x(rig, "rightArm", up * (1.0 - rec));
                        v.pose.rot_x(rig, "head", -0.3 * wind * (1.0 - rec));
                    }
                    MoveId::Raise => {
                        // Both arms up, the staff high: the dead are called out of the floor.
                        let up = if strike == 0.0 { -2.6 * wind } else { -2.6 * (1.0 - rec) };
                        v.pose.rot_x(rig, "rightArm", up);
                        v.pose.rot_x(rig, "leftArm", up * 0.8);
                    }
                    MoveId::Sceptre if v.kind == EnemyKind::Mammon => {
                        // The sceptre drawn back over his right shoulder as he winds round, then swept
                        // right across before him, and his weight following it.
                        v.root_rot.y = if strike == 0.0 {
                            -0.75 * wind
                        } else if rec == 0.0 {
                            -0.75 + 1.9 * snap
                        } else {
                            1.15 * (1.0 - rec)
                        };
                        let arm = if strike == 0.0 { -2.3 * wind } else { -2.3 + 1.1 * snap };
                        v.pose.rot_x(rig, "rightArm", arm * (1.0 - rec));
                        if let Some(r) = rig.index("rightArm") {
                            v.pose.rot[r].z = 0.5 * wind * (1.0 - rec);
                        }
                        v.pose.rot_x(rig, "leftArm", 0.35 * wind * (1.0 - rec));
                    }
                    MoveId::CoinToss | MoveId::CoinVolley => {
                        // His left hand down into the sack, then the coins flung up (or flat) at him.
                        let fling = if mv == MoveId::CoinToss { -2.3 } else { -1.5 };
                        let arm = if strike == 0.0 { 0.55 * wind } else { 0.55 + (fling - 0.55) * snap };
                        v.pose.rot_x(rig, "leftArm", arm * (1.0 - rec));
                        if let Some(l) = rig.index("leftArm") {
                            v.pose.rot[l].z = -0.25 * wind * (1.0 - rec);
                        }
                        v.root_rot.y = if strike == 0.0 { 0.3 * wind } else { 0.3 - 0.6 * snap * (1.0 - rec) };
                        v.root_rot.x = -0.08 * wind * (1.0 - rec);
                    }
                    MoveId::Grasp => {
                        // He draws back and leans in, his clawed left hand thrust out; while it holds
                        // him the fist stays closed and out.
                        let held = d.lore.hold > 0;
                        let out = if held { 1.0 } else { 1.0 - rec };
                        let arm = if strike == 0.0 { 0.4 * wind } else { 0.4 - 1.55 * snap };
                        v.pose.rot_x(rig, "leftArm", arm * out);
                        if let Some(l) = rig.index("leftArm") {
                            // (In across his front, down to where the hero stands.)
                            v.pose.rot[l].z = if strike == 0.0 { -0.3 * wind } else { 0.38 * snap * out };
                        }
                        v.root_rot.x = if strike == 0.0 { -0.12 * wind } else { 0.26 * out };
                        v.pose.rot_x(rig, "rightArm", 0.3 * wind * out);
                        if held {
                            v.root_rot.z = (time * 14.0).sin() * 0.02;
                        }
                    }
                    MoveId::CoinRain => {
                        // Both arms raised to the vault overhead, his head thrown back; then flung wide.
                        let up = if strike == 0.0 { -2.8 * wind } else { -2.8 * (1.0 - rec) };
                        v.pose.rot_x(rig, "leftArm", up);
                        v.pose.rot_x(rig, "rightArm", up);
                        v.pose.rot_x(rig, "head", -0.4 * wind * (1.0 - rec));
                        if let (Some(l), Some(r)) = (rig.index("leftArm"), rig.index("rightArm")) {
                            v.pose.rot[l].z = -0.6 * snap * (1.0 - rec);
                            v.pose.rot[r].z = 0.6 * snap * (1.0 - rec);
                        }
                    }
                    MoveId::HoardSlam => {
                        // The sceptre in both fists high over his crown, leaning back; down like a hammer.
                        let arms = if strike == 0.0 {
                            -3.0 * wind
                        } else if rec == 0.0 {
                            -3.0 + 3.6 * snap
                        } else {
                            0.6 * (1.0 - rec)
                        };
                        v.pose.rot_x(rig, "leftArm", arms);
                        v.pose.rot_x(rig, "rightArm", arms);
                        v.root_rot.x = if strike == 0.0 { -0.16 * wind } else { 0.28 * (1.0 - rec) };
                        v.root_scale = Vec3::new(1.0, 1.0 - 0.05 * snap * (1.0 - rec), 1.0);
                    }
                    MoveId::Cleaver => {
                        // The cleaver raised high over his right shoulder, his bulk leaning back; then
                        // brought down like a butcher's on the block.
                        let arm = if strike == 0.0 {
                            -2.7 * wind
                        } else if rec == 0.0 {
                            -2.7 + 3.3 * snap
                        } else {
                            0.6 * (1.0 - rec)
                        };
                        v.pose.rot_x(rig, "rightArm", arm);
                        if let Some(r) = rig.index("rightArm") {
                            v.pose.rot[r].z = 0.25 * wind * (1.0 - rec);
                        }
                        v.root_rot.x = if strike == 0.0 { -0.1 * wind } else { 0.2 * (1.0 - rec) };
                        v.root_rot.y = if strike == 0.0 { -0.3 * wind } else { (-0.3 + 0.6 * snap) * (1.0 - rec) };
                    }
                    MoveId::Swarm => {
                        // His arms flung wide, his head thrown back, the flies pouring from him at the throw.
                        if let (Some(l), Some(r)) = (rig.index("leftArm"), rig.index("rightArm")) {
                            let open = if strike == 0.0 { wind } else { 1.0 - rec };
                            v.pose.rot[l].z = -0.9 * open;
                            v.pose.rot[r].z = 0.9 * open;
                            v.pose.rot[l].x = -0.6 * open;
                            v.pose.rot[r].x = -0.6 * open;
                        }
                        v.pose.rot_x(rig, "head", if strike == 0.0 { -0.45 * wind } else { (-0.45 + 0.75 * snap) * (1.0 - rec) });
                        v.root_rot.x = if strike == 0.0 { -0.08 * wind } else { 0.12 * (1.0 - rec) };
                    }
                    MoveId::Gorge => {
                        // Bent over the food, his head down in it and bobbing, the drumstick to his mouth.
                        let bent = if strike == 0.0 { wind.min(1.0) } else { 1.0 - rec };
                        v.root_rot.x = 0.3 * bent;
                        v.pose.rot_x(rig, "head", (0.55 + (time * 11.0).sin() * 0.18) * bent);
                        v.pose.rot_x(rig, "leftArm", (-1.5 + (time * 5.5).sin() * 0.3) * bent);
                        v.pose.rot_x(rig, "rightArm", -0.5 * bent);
                        v.root_scale = Vec3::new(1.0 + 0.03 * (time * 11.0).sin().abs() * bent, 1.0, 1.0);
                    }
                    MoveId::Scatter => {
                        // He crouches as he bursts into flies, and lands squashed, belly first.
                        if strike == 0.0 {
                            let c = (frame as f32 / 12.0).min(1.0);
                            v.root_scale = Vec3::new(1.0 + 0.1 * c, 1.0 - 0.18 * c, 1.0 + 0.1 * c);
                        } else {
                            let squash = (1.0 - rec) * 0.2;
                            v.root_scale = Vec3::new(1.0 + squash * 0.6, 1.0 - squash, 1.0 + squash * 0.6);
                            v.pose.rot_x(rig, "leftArm", -1.2 * (1.0 - rec));
                            v.pose.rot_x(rig, "rightArm", -1.2 * (1.0 - rec));
                        }
                    }
                    MoveId::BellyRoll => {
                        // Rocked back, arms tucked; then over and over on his belly; then up, dizzy.
                        if strike == 0.0 {
                            v.root_rot.x = -0.25 * wind;
                            v.pose.rot_x(rig, "leftArm", -0.8 * wind);
                            v.pose.rot_x(rig, "rightArm", -0.8 * wind);
                        } else if rec == 0.0 {
                            v.root_rot.x = (f - s) * 0.32;
                            v.root_scale = Vec3::new(1.08, 0.86, 1.08);
                            v.pose.rot_x(rig, "leftArm", -1.6);
                            v.pose.rot_x(rig, "rightArm", -1.6);
                            v.pose.rot_x(rig, "head", 0.8);
                        } else {
                            v.root_rot.z = (time * 7.0).sin() * 0.12 * (1.0 - rec);
                            v.pose.rot_x(rig, "head", 0.3 * (1.0 - rec));
                        }
                    }
                    MoveId::BileSpew => {
                        // His head drawn back, his belly heaving; then the bile spewed out over the floor.
                        v.pose.rot_x(rig, "head", if strike == 0.0 { -0.5 * wind } else { (-0.5 + 1.0 * snap) * (1.0 - rec) });
                        v.root_rot.x = if strike == 0.0 { -0.12 * wind } else { 0.22 * (1.0 - rec) };
                        let heave = if strike == 0.0 { (time * 14.0).sin().abs() * 0.05 * wind } else { 0.0 };
                        v.root_scale = Vec3::new(1.0 + heave, 1.0 - heave * 0.5, 1.0 + heave);
                    }
                    MoveId::BellySlam => {
                        // Up on his toes, arms high, leaning back; then down on the floor belly first.
                        let arms = if strike == 0.0 { -2.6 * wind } else { -2.6 * (1.0 - snap) * (1.0 - rec) };
                        v.pose.rot_x(rig, "leftArm", arms);
                        v.pose.rot_x(rig, "rightArm", arms);
                        v.root_rot.x = if strike == 0.0 { -0.18 * wind } else { 0.32 * (1.0 - rec) };
                        if strike == 0.0 {
                            v.group_pos.y += 0.25 * wind * wind;
                        }
                        v.root_scale =
                            Vec3::new(1.0 + 0.1 * snap * (1.0 - rec), 1.0 - 0.12 * snap * (1.0 - rec), 1.0 + 0.1 * snap * (1.0 - rec));
                    }
                    MoveId::Hex | MoveId::Smokeless => {
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
            State::Evade { frame } if !d.puppet => {
                // A rival swaying back out of his finisher: low, leaning away, then up again.
                let u = (frame as f32 / d.f.evade_total.max(1) as f32).min(1.0);
                let arc = (u * PI).sin();
                v.root_rot.x = -0.45 * arc;
                v.root_scale = Vec3::new(1.0 + 0.06 * arc, 1.0 - 0.14 * arc, 1.0 + 0.06 * arc);
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
                let top = crown(d.breed.kind).unwrap_or(match d.arch {
                    Archetype::Brute => 1.25,
                    Archetype::Skirmisher => 0.95,
                    Archetype::Hexer => 1.55,
                    Archetype::Prince => 3.25,
                });
                let m = Mat4::from_translation(base + Vec3::new(0.0, top + (time * 4.0).sin() * 0.04, 0.0))
                    * Mat4::from_rotation_y(time * 3.0)
                    * Mat4::from_rotation_z(PI / 4.0)
                    * Mat4::from_scale(Vec3::splat(0.09));
                f.additive.push(Instance::new(m, lin(0xff3a2a) * 2.2, 0.95, Vec3::ZERO, SHADE_UNLIT));
            }
            let State::Attack { mv, frame } = d.f.state else { continue };
            let data = d.data(mv);
            let phase = data.phase(frame);
            if phase == Phase::Recovery {
                continue;
            }
            if mv == MoveId::Burrow && frame >= crate::logic::combat::duelist::BURROW_SINK && phase == Phase::Startup {
                // The earth heaving over it as it goes.
                for k in 0..5 {
                    let a = k as f32 * 1.3 + time * 3.0;
                    let p = base + Vec3::new(a.cos() * 0.3, 0.06 + 0.05 * (time * 9.0 + k as f32).sin().abs(), a.sin() * 0.3);
                    let m = Mat4::from_translation(p) * Mat4::from_rotation_y(a) * Mat4::from_scale(Vec3::new(0.32, 0.12, 0.26));
                    f.opaque[MeshKind::Cube as usize].push(Instance::new(m, lin(0x4a3a28), 1.0, Vec3::ZERO, SHADE_LIT));
                }
            }
            let color = lin(tell_of(d));
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
            let ring = |dot: &mut dyn FnMut(Vec3, bool), at: Vec3, r: f32| {
                // A ring of dots on the floor, an inner one, and its middle, lit as the fill grows inward.
                for (rr, n) in [(r, ((TAU * r) / 0.24).ceil() as i32), (r * 0.55, ((TAU * r * 0.55) / 0.26).ceil() as i32)] {
                    for k in 0..n {
                        let a = k as f32 * TAU / n as f32;
                        dot(at + Vec3::new(a.cos() * rr, 0.0, a.sin() * rr), rr >= r * (1.0 - fill));
                    }
                }
                dot(at, fill >= 0.95);
            };
            match data.shape {
                Shape::Mark { radius } => {
                    // Where the burrow will burst up: a ring following him, then holding.
                    let m = d.lore.mark;
                    let at = Vec3::new(m.x as f32, 0.0, m.y as f32) - Vec3::new(base.x, 0.0, base.z);
                    ring(&mut dot, at, radius as f32);
                    continue;
                }
                Shape::Volley { count, spread } => {
                    // A fan of lines, the way each foxfire will fly; the fire gathering at its tail.
                    for k in 0..count {
                        let a = -spread as f32 + 2.0 * spread as f32 * k as f32 / (count.max(2) - 1) as f32;
                        let dir = aim * a.cos() + side * a.sin();
                        for j in 1..=10 {
                            dot(dir * (0.5 + j as f32 * 0.32), j as f32 / 10.0 <= fill);
                        }
                    }
                    let orb = base + aim * -0.3 + Vec3::new(0.0, 0.7, 0.0);
                    let m =
                        Mat4::from_translation(orb) * Mat4::from_rotation_y(time * 6.0) * Mat4::from_scale(Vec3::splat(0.04 + 0.12 * fill));
                    f.additive.push(Instance::new(m, color * 2.4, 0.9, Vec3::ZERO, SHADE_UNLIT));
                    continue;
                }
                Shape::Spell => {
                    // A call: pale rings rolling out from it toward him; the dead: a cold swirl round her.
                    let n = 3;
                    for k in 0..n {
                        let r = ((time * 1.4 + k as f32 / n as f32).fract() * 2.6 + 0.4) * if mv == MoveId::Call { 1.0 } else { 0.5 };
                        let m = Mat4::from_translation(base + Vec3::new(0.0, 0.05, 0.0))
                            * Mat4::from_rotation_x(-PI / 2.0)
                            * Mat4::from_scale(Vec3::new(r, r, 1.0));
                        f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, color * 1.6, 0.5 * fill, Vec3::ZERO, SHADE_UNLIT));
                    }
                    continue;
                }
                Shape::Quake { radius, .. } => {
                    // The ring the ground will shake in, round the cone.
                    let n = ((TAU * radius as f32) / 0.3).ceil() as i32;
                    for k in 0..n {
                        let a = k as f32 * TAU / n as f32;
                        dot(Vec3::new(a.cos(), 0.0, a.sin()) * radius as f32, fill >= 0.7);
                    }
                }
                Shape::Traps { .. } => {
                    // Where the coins will land: a ring about him, the gold gathering in the prince's hand.
                    let hp = s.hero.pos();
                    let at = Vec3::new(hp.x as f32, 0.0, hp.y as f32) - Vec3::new(base.x, 0.0, base.z);
                    ring(&mut dot, at, if mv == MoveId::CoinRain { 2.6 } else { 1.8 });
                    let hand = base + Vec3::new(0.0, 2.0, 0.0) + side * -0.9;
                    let m = Mat4::from_translation(hand)
                        * Mat4::from_rotation_y(time * 6.0)
                        * Mat4::from_scale(Vec3::splat(0.05 + 0.16 * fill));
                    f.additive.push(Instance::new(m, color * 2.4, 0.9, Vec3::ZERO, SHADE_UNLIT));
                    continue;
                }
                Shape::Beam { length, width } => {
                    // The line the beam will run along, as far as the rock lets it, and its edges.
                    let span = if d.lore.span > 0.0 { d.lore.span as f32 } else { length as f32 };
                    let half = width as f32 / 2.0;
                    let n = (span / 0.3).ceil().max(1.0) as i32;
                    for k in 1..=n {
                        let along = k as f32 * span / n as f32;
                        let lit = along / span <= fill;
                        dot(aim * along, lit);
                        if k % 2 == 0 {
                            dot(aim * along + side * half, lit);
                            dot(aim * along - side * half, lit);
                        }
                    }
                    continue;
                }
                Shape::Charge { length, .. } => {
                    // The way he will run, filling from him outward.
                    let length = length as f32;
                    let n = (length / 0.35).ceil().max(1.0) as i32;
                    for k in 1..=n {
                        let along = k as f32 * length / n as f32;
                        dot(aim * along, along / length <= fill);
                    }
                    continue;
                }
                Shape::TrapLine { count, radius, spacing, .. } => {
                    // Where the row of traps will lie: a ring for each along his aim.
                    let first = d.radius() as f32 + radius as f32;
                    for k in 0..count {
                        ring(&mut dot, aim * (first + k as f32 * spacing as f32), radius as f32);
                    }
                    continue;
                }
                Shape::Cone => {}
            }
            match mv {
                _ if data.projectile => {
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
                    // His mastery's white edge: raised now, the guard would meet it in the parry window.
                    let left = data.startup.saturating_sub(frame);
                    if s.parry_edge.is_some_and(|to| phase == Phase::Startup && (PARRY_FROM..=to).contains(&left)) {
                        let n = ((2.0 * half * (reach + 0.12)) / 0.12).ceil().max(2.0) as i32;
                        for k in 0..=n {
                            let a = -half + 2.0 * half * k as f32 / n as f32;
                            let dir = aim * a.cos() + side * a.sin();
                            let m = Mat4::from_translation(base + dir * (reach + 0.12) + Vec3::new(0.0, 0.03, 0.0))
                                * Mat4::from_scale(Vec3::new(0.07, 0.014, 0.07));
                            f.additive.push(Instance::new(m, Vec3::splat(3.2), 0.95, Vec3::ZERO, SHADE_UNLIT));
                        }
                    }
                }
            }
        }
        for fl in s.flashes {
            self.emit_flash(fl, time, f);
        }
        if self.court == Some(Sin::Gluttony) {
            // (Beelzebub's bile, not Mammon's coins.)
            self.emit_bile(s.traps, s.grid, time, f);
        } else {
            self.emit_traps(s.traps, s.grid, time, f);
        }
        // Surtr's fire on the floor: tongues of flame flickering up, and their light.
        for h in s.hazards {
            let c = Vec3::new(h.pos.x as f32, floor_at(s.grid, h.pos.x as f32, h.pos.y as f32), h.pos.y as f32);
            let dying = (h.life as f32 / 30.0).min(1.0);
            for k in 0..7 {
                let a = k as f32 * 2.4 + h.pos.x as f32 * 3.0;
                let r = h.radius as f32 * (0.25 + 0.6 * ((k * 37 % 10) as f32 / 10.0));
                let flick = ((time * (7.0 + k as f32)) + a).sin() * 0.5 + 0.5;
                let tall = (0.25 + 0.35 * flick) * dying;
                let p = c + Vec3::new(a.cos() * r, tall * 0.5, a.sin() * r);
                let m = Mat4::from_translation(p) * Mat4::from_rotation_y(a) * Mat4::from_scale(Vec3::new(0.12, tall, 0.12));
                let col = if k % 3 == 0 { lin(0xffe070) } else { lin(0xff6a1a) };
                f.additive.push(Instance::new(m, col * 2.2, 0.85 * dying, Vec3::ZERO, SHADE_UNLIT));
            }
            f.lights.push(Light { pos: c + Vec3::new(0.0, 0.4, 0.0), color: lin(0xff7a2a) * 3.0 * dying, distance: 3.0 });
        }
        for b in s.bolts {
            emit_bolt(b, time, f);
        }
    }

    /// A legend's mark on the scene, as it ages: lightning striking down, roots bursting up and
    /// sinking back, spikes of ice rising along their lane, the floor bursting, smoke, a splash,
    /// the ground's tremor rolling out, a grave opening.
    fn emit_flash(&self, fl: &Flash, time: f32, f: &mut Frame) {
        let u = fl.progress();
        let at = Vec3::new(fl.pos.x as f32, 0.0, fl.pos.y as f32);
        let cube = |f: &mut Frame, p: Vec3, size: Vec3, color: Vec3, alpha: f32, spin: f32| {
            let m = Mat4::from_translation(p) * Mat4::from_rotation_y(spin) * Mat4::from_scale(size);
            f.additive.push(Instance::new(m, color, alpha, Vec3::ZERO, SHADE_UNLIT));
        };
        match fl.kind {
            FlashKind::Lightning => {
                // A jagged bolt from the sky to the mark, white at its heart; then only the glow.
                let fade = 1.0 - u;
                let mut p = at + Vec3::new(0.0, 7.0, 0.0);
                for k in 0..14 {
                    let q = at
                        + Vec3::new(
                            ((k * 7919) % 13) as f32 / 13.0 * 0.5 - 0.25,
                            7.0 - (k + 1) as f32 * 0.5,
                            ((k * 104729) % 11) as f32 / 11.0 * 0.5 - 0.25,
                        );
                    let q = if k == 13 { at } else { q };
                    let mid = (p + q) * 0.5;
                    let len = (q - p).length();
                    let dir = (q - p).normalize_or_zero();
                    let rot = Quat::from_rotation_arc(Vec3::Y, dir);
                    let m = Mat4::from_rotation_translation(rot, mid) * Mat4::from_scale(Vec3::new(0.09, len, 0.09));
                    f.additive.push(Instance::new(m, lin(0xe8f0ff) * 3.0, 0.95 * fade, Vec3::ZERO, SHADE_UNLIT));
                    p = q;
                }
                let m = Mat4::from_translation(at + Vec3::new(0.0, 0.03, 0.0))
                    * Mat4::from_rotation_x(-PI / 2.0)
                    * Mat4::from_scale(Vec3::new(1.0 + u, 1.0 + u, 1.0));
                f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, lin(0xa8c8ff) * 2.0, 0.8 * fade, Vec3::ZERO, SHADE_UNLIT));
                f.lights.push(Light { pos: at + Vec3::new(0.0, 1.0, 0.0), color: lin(0xc8d8ff) * 9.0 * fade, distance: 6.0 });
            }
            FlashKind::Roots => {
                // Gnarled roots thrust up out of the floor, hold, and sink back.
                let rise = if u < 0.12 {
                    u / 0.12
                } else if u > 0.75 {
                    (1.0 - u) / 0.25
                } else {
                    1.0
                };
                for k in 0..9 {
                    let a = k as f32 * 2.17 + fl.pos.y as f32;
                    let r = 0.15 + 0.7 * ((k * 31 % 9) as f32 / 9.0);
                    let tall = (0.5 + 0.5 * ((k * 17 % 7) as f32 / 7.0)) * rise;
                    let base = at + Vec3::new(a.cos() * r, tall * 0.5, a.sin() * r);
                    let m = Mat4::from_translation(base)
                        * Mat4::from_rotation_y(a)
                        * Mat4::from_rotation_z(0.25 * (a * 3.0).sin())
                        * Mat4::from_scale(Vec3::new(0.09, tall.max(0.01), 0.09));
                    let col = if k % 3 == 0 { lin(0x4a8a30) } else { lin(0x5a3a1e) };
                    f.opaque[MeshKind::Cube as usize].push(Instance::new(m, col, 1.0, Vec3::ZERO, SHADE_LIT));
                }
            }
            FlashKind::Ice => {
                // Spikes of ice rising one after another down the lane, then melting away.
                let dir = Vec3::new(fl.dir.x as f32, 0.0, fl.dir.y as f32).normalize_or_zero();
                let side = Vec3::new(-dir.z, 0.0, dir.x);
                let n = (fl.length as f32 / 0.35).ceil() as i32;
                for k in 1..=n {
                    let along = k as f32 * fl.length as f32 / n as f32;
                    let born = 0.12 * along / fl.length as f32;
                    if u < born {
                        continue;
                    }
                    let grow = ((u - born) / 0.06).min(1.0) * (1.0 - ((u - 0.7) / 0.3).clamp(0.0, 1.0));
                    for w in [-0.25f32, 0.2] {
                        let tall = (0.35 + 0.3 * ((k * 13 % 5) as f32 / 5.0)) * grow;
                        let p = at + dir * along + side * w + Vec3::new(0.0, tall * 0.5, 0.0);
                        let m = Mat4::from_translation(p)
                            * Mat4::from_rotation_y(k as f32)
                            * Mat4::from_rotation_x(0.2 * w.signum())
                            * Mat4::from_scale(Vec3::new(0.12, tall.max(0.01), 0.12));
                        f.transparent[MeshKind::Cube as usize].push(Instance::new(
                            m,
                            lin(0xc8f0ff),
                            0.75,
                            lin(0x6ab8ff) * 0.4,
                            SHADE_UNLIT,
                        ));
                    }
                }
            }
            FlashKind::Erupt => {
                // The floor bursting: clods thrown up and falling back, a ring rolling out.
                for k in 0..10 {
                    let a = k as f32 * TAU / 10.0;
                    let up = (u * 3.0).min(1.0);
                    let p = at + Vec3::new(a.cos() * (0.3 + u * 1.2), up * 1.2 - u * u * 1.6, a.sin() * (0.3 + u * 1.2));
                    let m = Mat4::from_translation(p.max(Vec3::new(f32::MIN, 0.05, f32::MIN)))
                        * Mat4::from_rotation_y(a + time)
                        * Mat4::from_scale(Vec3::splat(0.12));
                    f.opaque[MeshKind::Cube as usize].push(Instance::new(m, lin(0x4a3a28), 1.0, Vec3::ZERO, SHADE_LIT));
                }
                let r = 0.4 + u * 1.6;
                let m = Mat4::from_translation(at + Vec3::new(0.0, 0.04, 0.0))
                    * Mat4::from_rotation_x(-PI / 2.0)
                    * Mat4::from_scale(Vec3::new(r, r, 1.0));
                f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, lin(0x9ad04a), 0.6 * (1.0 - u), Vec3::ZERO, SHADE_UNLIT));
            }
            FlashKind::Smoke => {
                // A puff of grey smoke, swelling and thinning.
                for k in 0..5 {
                    let a = k as f32 * 1.3;
                    let p = at + Vec3::new(a.cos() * 0.25 * (1.0 + u), 0.3 + u * 0.9 + k as f32 * 0.08, a.sin() * 0.25 * (1.0 + u));
                    let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.22 + u * 0.35));
                    f.transparent[MeshKind::Sphere as usize].push(Instance::new(
                        m,
                        lin(0xb8b4c8),
                        0.55 * (1.0 - u),
                        Vec3::ZERO,
                        SHADE_UNLIT,
                    ));
                }
            }
            FlashKind::Splash => {
                // The water of the dish thrown out in drops, falling.
                for k in 0..12 {
                    let a = k as f32 * TAU / 12.0;
                    let p = at + Vec3::new(a.cos() * u * 0.9, 1.0 + u * 0.8 - u * u * 2.2, a.sin() * u * 0.9);
                    if p.y > 0.0 {
                        cube(f, p, Vec3::splat(0.05), lin(0x8ad8ff) * 1.8, 0.9, 0.0);
                    }
                }
            }
            FlashKind::Quake => {
                // The tremor rolling out over the floor.
                let r = 0.3 + u * fl.length.max(1.0) as f32;
                for (rr, alpha) in [(r, 0.7), (r * 0.7, 0.4)] {
                    let m = Mat4::from_translation(at + Vec3::new(0.0, 0.04, 0.0))
                        * Mat4::from_rotation_x(-PI / 2.0)
                        * Mat4::from_scale(Vec3::new(rr, rr, 1.0));
                    f.transparent[MeshKind::Annulus as usize].push(Instance::new(
                        m,
                        lin(0xc8a878),
                        alpha * (1.0 - u),
                        Vec3::ZERO,
                        SHADE_UNLIT,
                    ));
                }
            }
            FlashKind::Grave => {
                // A dark patch where the floor opens for the dead.
                let r = 0.55 * (1.0 - u * 0.3);
                let m = Mat4::from_translation(at + Vec3::new(0.0, 0.02, 0.0))
                    * Mat4::from_rotation_x(-PI / 2.0)
                    * Mat4::from_scale(Vec3::new(r, r, 1.0));
                f.transparent[MeshKind::Disc as usize].push(Instance::new(m, lin(0x0a0810), 0.75 * (1.0 - u), Vec3::ZERO, SHADE_UNLIT));
                cube(f, at + Vec3::new(0.0, 0.1, 0.0), Vec3::new(0.5, 0.02, 0.5), lin(0x6a8cff), 0.4 * (1.0 - u), time);
            }
        }
    }
}

/**
 * A bolt in flight, as its caster's kind looses it (in its colour: gold once it flies at the foes):
 * the bog slime's glob of spit wobbling and dripping, the witch's violet star in a spinning ring,
 * the jorogumo's ball of fire with flames streaming up off it, the kitsune's foxfire wisp on a
 * long wavering tail, the cin's tall pale flame with no smoke behind it, the arçura's thorny dart
 * shedding leaves, the goryō's jagged lightning, Hel's dark drain with cold motes about it, and his
 * own blade spinning. Each lights the floor under it its own way.
 */
fn emit_bolt(b: &Bolt, time: f32, f: &mut Frame) {
    let p = Vec3::new(b.pos.x as f32, 1.0, b.pos.y as f32);
    let v = Vec3::new(b.vel.x as f32, 0.0, b.vel.y as f32).normalize_or_zero();
    let side = Vec3::new(-v.z, 0.0, v.x);
    let color = lin(b.tint);
    let seed = (b.pos.x * 7.3 + b.pos.y * 3.1) as f32;
    let yaw = v.x.atan2(v.z);
    let add = |f: &mut Frame, at: Vec3, rot: Quat, size: Vec3, c: Vec3, alpha: f32| {
        f.additive.push(Instance::new(Mat4::from_scale_rotation_translation(size, rot, at), c, alpha, Vec3::ZERO, SHADE_UNLIT));
    };
    let (light, reach) = match b.look {
        BoltLook::Spit => {
            // A glob of green bog spit, wobbling as it flies, dripping onto the floor behind it.
            let wob = (time * 18.0).sin() * 0.04;
            let m = Mat4::from_scale_rotation_translation(
                Vec3::new(0.2 + wob, 0.16 - wob, 0.24),
                Quat::from_rotation_y(yaw),
                p - Vec3::new(0.0, 0.1, 0.0),
            );
            f.transparent[MeshKind::Sphere as usize].push(Instance::new(m, color * 0.9, 0.85, color * 0.35, SHADE_UNLIT));
            for k in 1..4 {
                let t = ((time * 2.5 + k as f32 * 0.33).fract()) * 0.5;
                let at = p - v * (0.18 + 0.12 * k as f32) - Vec3::new(0.0, 0.1 + t * 1.6, 0.0);
                let m = Mat4::from_scale_rotation_translation(Vec3::splat(0.05 - k as f32 * 0.008), Quat::IDENTITY, at);
                f.transparent[MeshKind::Sphere as usize].push(Instance::new(m, color, 0.8, color * 0.3, SHADE_UNLIT));
            }
            (color * 1.2, 2.2)
        }
        BoltLook::Hex => {
            // A violet star (two cubes crossed) turning in a ring of the same light, sparks behind.
            let spin = Quat::from_rotation_y(time * 9.0) * Quat::from_rotation_x(time * 7.0);
            add(f, p, spin, Vec3::splat(0.13), color * 2.6, 0.95);
            add(f, p, spin * Quat::from_rotation_z(PI / 4.0) * Quat::from_rotation_x(PI / 4.0), Vec3::splat(0.13), color * 2.2, 0.9);
            let ring = Mat4::from_translation(p)
                * Mat4::from_rotation_y(yaw)
                * Mat4::from_rotation_z(time * 5.0)
                * Mat4::from_scale(Vec3::new(0.24, 0.24, 1.0));
            f.transparent[MeshKind::Annulus as usize].push(Instance::new(ring, color * 2.0, 0.8, Vec3::ZERO, SHADE_UNLIT));
            for k in 1..4 {
                add(
                    f,
                    p - v * k as f32 * 0.16,
                    Quat::from_rotation_y(time * 9.0 + k as f32),
                    Vec3::splat(0.09 - k as f32 * 0.02),
                    color * (2.0 - k as f32 * 0.45),
                    0.8 - k as f32 * 0.2,
                );
            }
            (color * 2.5, 3.0)
        }
        BoltLook::Fire => {
            // A ball of fire, a yellow heart in it, flames streaming up and back off it.
            glow_orb(f, p, 0.55, color, 1.0);
            add(f, p, Quat::from_rotation_y(time * 11.0) * Quat::from_rotation_x(time * 8.0), Vec3::splat(0.2), color * 2.4, 0.95);
            add(f, p, Quat::from_rotation_x(time * 13.0), Vec3::splat(0.11), lin(0xffe070) * 3.0, 1.0);
            for k in 0..6 {
                let t = (time * 3.0 + k as f32 / 6.0).fract();
                let wave = side * ((time * 9.0 + k as f32 * 2.1).sin() * 0.08);
                let at = p - v * (0.12 + t * 0.6) + Vec3::new(0.0, t * 0.45, 0.0) + wave;
                let c = if k % 2 == 0 { lin(0xffd050) } else { color };
                add(f, at, Quat::from_rotation_y(t * 6.0 + k as f32), Vec3::splat(0.12 * (1.0 - t) + 0.02), c * 2.2, 0.9 * (1.0 - t));
            }
            (color * 4.0, 4.0)
        }
        BoltLook::Foxfire => {
            // A pale wisp, bobbing as it comes, on a long wavering tail.
            let bob = Vec3::new(0.0, (time * 7.0 + seed).sin() * 0.08, 0.0);
            glow_orb(f, p + bob, 0.32, color, 0.9);
            add(f, p + bob, Quat::from_rotation_y(time * 6.0), Vec3::new(0.08, 0.14, 0.08), Vec3::ONE.lerp(color, 0.4) * 2.6, 0.95);
            for k in 1..9 {
                let back = k as f32 * 0.11;
                let sway = side * ((time * 8.0 - k as f32 * 0.7 + seed).sin() * 0.06 * k as f32 / 4.0);
                let at = p + bob * (1.0 - k as f32 / 9.0) - v * back + sway + Vec3::new(0.0, k as f32 * 0.015, 0.0);
                add(
                    f,
                    at,
                    Quat::from_rotation_y(k as f32),
                    Vec3::splat(0.07 * (1.0 - k as f32 / 10.0)),
                    color * (2.0 - k as f32 * 0.18),
                    0.85 - k as f32 * 0.08,
                );
            }
            (color * 2.0, 2.6)
        }
        BoltLook::Smokeless => {
            // A tall pale flame standing straight up as it flies, flickering; nothing trails it.
            let flick = 1.0 + (time * 23.0 + seed).sin() * 0.15 + (time * 37.0).sin() * 0.08;
            add(
                f,
                p + Vec3::new(0.0, 0.05, 0.0),
                Quat::from_rotation_y(time * 4.0),
                Vec3::new(0.12, 0.42 * flick, 0.12),
                color * 2.2,
                0.85,
            );
            add(f, p, Quat::from_rotation_y(-time * 5.0), Vec3::new(0.06, 0.26 * flick, 0.06), Vec3::splat(3.0), 1.0);
            glow_orb(f, p, 0.4, Vec3::ONE.lerp(color, 0.5), 0.6);
            (Vec3::ONE.lerp(color, 0.5) * 3.2, 3.4)
        }
        BoltLook::Thorn => {
            // A dart of root and thorn, rolling as it flies, leaves shed behind it.
            let roll = Quat::from_rotation_y(yaw) * Quat::from_rotation_z(time * 12.0);
            let m = Mat4::from_scale_rotation_translation(Vec3::new(0.07, 0.07, 0.46), roll, p);
            f.opaque[MeshKind::Cube as usize].push(Instance::new(m, lin(0x6a4a24), 1.0, Vec3::ZERO, SHADE_LIT));
            for k in 0..4 {
                let a = k as f32 * PI / 2.0;
                let off = roll * Vec3::new(a.cos() * 0.07, a.sin() * 0.07, 0.12 - k as f32 * 0.08);
                let m = Mat4::from_scale_rotation_translation(Vec3::new(0.025, 0.025, 0.1), roll * Quat::from_rotation_x(0.9), p + off);
                f.opaque[MeshKind::Cube as usize].push(Instance::new(m, lin(0x3a5a1a), 1.0, Vec3::ZERO, SHADE_LIT));
            }
            add(f, p + v * 0.25, roll, Vec3::new(0.04, 0.04, 0.08), lin(0x9adf5a) * 1.8, 0.9);
            for k in 1..5 {
                let t = (time * 2.0 + k as f32 * 0.25).fract();
                let at = p - v * (0.3 + t * 0.7) + side * ((k as f32 * 1.7 + time * 3.0).sin() * 0.12) - Vec3::new(0.0, t * 0.4, 0.0);
                add(
                    f,
                    at,
                    Quat::from_rotation_y(time * 5.0 + k as f32) * Quat::from_rotation_x(k as f32),
                    Vec3::new(0.07, 0.01, 0.04),
                    lin(0x6ad050) * 1.6,
                    0.8 * (1.0 - t),
                );
            }
            (lin(0x6ad050) * 1.2, 2.0)
        }
        BoltLook::Lightning => {
            // A jagged fork of lightning, struck anew every few frames, white at the head.
            let n = 5;
            let jag = (time * 30.0).floor();
            let mut a = p;
            for k in 0..n {
                let h = ((jag + k as f32 * 7.0) * 12.9898).sin() * 43758.547;
                let r = (h - h.floor()) * 2.0 - 1.0;
                let b2 = p - v * (k + 1) as f32 * 0.2 + side * r * 0.14 + Vec3::new(0.0, r * 0.06, 0.0);
                let mid = (a + b2) * 0.5;
                let d = b2 - a;
                let len = d.length().max(0.01);
                let rot = Quat::from_rotation_arc(Vec3::Z, d / len);
                add(f, mid, rot, Vec3::new(0.035, 0.035, len * 0.5 + 0.02), Vec3::ONE.lerp(color, 0.5) * 3.2, 1.0 - k as f32 * 0.12);
                a = b2;
            }
            glow_orb(f, p, 0.45, color, 1.1);
            add(f, p, Quat::from_rotation_y(time * 20.0), Vec3::splat(0.09), Vec3::splat(3.5), 1.0);
            let flick = 0.7 + ((time * 41.0).sin() * 0.5 + 0.5) * 0.6;
            (color * 4.5 * flick, 4.0)
        }
        BoltLook::Drain => {
            // A dark heart of cold, motes of pale blue circling it, a ghost of it fading behind.
            let m = Mat4::from_scale_rotation_translation(Vec3::splat(0.12), Quat::IDENTITY, p);
            f.opaque[MeshKind::Sphere as usize].push(Instance::new(m, lin(0x101828), 1.0, color * 0.15, SHADE_UNLIT));
            glow_orb(f, p, 0.42, color, 0.7);
            for k in 0..3 {
                let a = time * 8.0 + k as f32 * TAU / 3.0;
                let at = p + side * a.cos() * 0.22 + Vec3::new(0.0, a.sin() * 0.22, 0.0);
                add(f, at, Quat::from_rotation_y(a), Vec3::splat(0.045), lin(0xc8dcff) * 2.6, 0.95);
            }
            for k in 1..4 {
                glow_orb(f, p - v * k as f32 * 0.22, 0.3 - k as f32 * 0.06, color, 0.45 - k as f32 * 0.1);
            }
            (color * 2.5, 3.0)
        }
        BoltLook::Coin => {
            // A gold coin tumbling end over end, a glint of it trailing behind.
            let spin = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(time * 22.0 + seed);
            let m = Mat4::from_scale_rotation_translation(Vec3::new(0.16, 0.035, 0.16), spin, p);
            f.opaque[MeshKind::Cylinder as usize].push(Instance::new(m, color, 1.0, color * 0.35, 13.0));
            for k in 1..4 {
                add(f, p - v * k as f32 * 0.16, spin, Vec3::new(0.08, 0.08, 0.02), color * (1.8 - k as f32 * 0.4), 0.7 - k as f32 * 0.18);
            }
            (color * 2.0, 2.5)
        }
        BoltLook::Flies => {
            // A buzzing knot of black flies, each on its own tight loop, wings blurred pale, a sickly
            // green sheen on them, and a few stragglers trailing.
            for k in 0..14 {
                let kf = k as f32;
                let a = time * (17.0 + kf * 1.9) + kf * 2.3 + seed;
                let r = 0.08 + 0.13 * ((k * 7 % 5) as f32 / 4.0);
                let trail = if k >= 11 { (kf - 10.0) * 0.18 } else { 0.0 };
                let at = p - v * trail
                    + side * (a.cos() * r)
                    + Vec3::new(0.0, (a * 1.3).sin() * r * 0.8 - 0.15, 0.0)
                    + v * ((a * 0.7).sin() * r);
                let body = Mat4::from_scale_rotation_translation(Vec3::new(0.045, 0.035, 0.06), Quat::from_rotation_y(a), at);
                f.opaque[MeshKind::Cube as usize].push(Instance::new(body, lin(0x141010), 1.0, lin(0x2a3a10) * 0.6, SHADE_LIT));
                add(f, at + Vec3::new(0.0, 0.03, 0.0), Quat::from_rotation_y(a * 3.0), Vec3::new(0.09, 0.006, 0.04), lin(0xc8d0b0), 0.35);
            }
            glow_orb(f, p - Vec3::new(0.0, 0.1, 0.0), 0.32, color, 0.12);
            (color * 0.6, 1.6)
        }
        BoltLook::Blade => {
            // His blade, spinning flat.
            let spin = Quat::from_rotation_y(time * 24.0);
            add(f, p, spin, Vec3::new(0.42, 0.025, 0.07), color * 2.6, 0.95);
            add(f, p - v * 0.15, Quat::from_rotation_y(time * 24.0 - 0.5), Vec3::new(0.36, 0.02, 0.05), color * 1.4, 0.5);
            (color * 2.5, 3.0)
        }
    };
    f.lights.push(Light { pos: p, color: light, distance: reach });
}
