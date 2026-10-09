//! The scenes at Hızır's on screen (B07, `logic::after`): his workshop as the wares show it (the
//! hero's own dressing of it, its fire and lamps), the three boys round the anvil with Toby's thing
//! lying on it, each with his warrior's soul standing behind him in spirit light, Hızır across the
//! anvil from them leaning on his staff, Quitmer on the floor by the hearth. The cast play by their
//! clips (Scene I's: `cast_clips`); the camera cuts by the beat and the speaker, as Scene I's does.

use super::cast_clips::{DogAct, OldAct};
use super::story::{kid_root, Act, StoryView};
use super::story_camera::{Dof, Lens, StoryCam};
use super::toby::token_model;
use super::workshop::{ANVIL, ANVIL_TOP, WORKSHOP};
use super::*;
use crate::game::workshop::{QuitmerMood, Station, WorkshopView, TROPHY_SLOTS};
use crate::logic::after::{kid_of_role, stage};
use crate::logic::gear::BELT_SLOTS;
use crate::logic::heroes::HeroKind;
use crate::logic::story::{Scene, Shot, Speaker};
use crate::logic::tale::{soul_color, warrior_of, Token, TOBY_COLOR};

/// Where Toby's thing lies: on the anvil's face (workshop coordinates).
const TOKEN_AT: Vec3 = Vec3::new(ANVIL.x, ANVIL_TOP + 0.02, ANVIL.z + 0.02);
/// Hızır, across the anvil from the boys, his back to the racks.
const HIZIR_AT: Vec3 = Vec3::new(-4.3, 0.0, -3.78);
/// The three boys (kids 1, 2, 3, left to right as they landed in Scene I) round the anvil.
const BOYS_AT: [Vec3; 3] = [Vec3::new(-4.98, 0.0, -2.08), Vec3::new(-4.0, 0.0, -1.76), Vec3::new(-3.02, 0.0, -2.08)];
/// Quitmer on the floor by the hearth.
const DOG_AT: Vec3 = Vec3::new(-5.75, 0.0, -3.05);
/// The souls stand this far behind their boys.
const SOUL_BACK: f32 = 0.62;

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn yaw_to(from: Vec3, to: Vec3) -> f32 {
    (to.x - from.x).atan2(to.z - from.z)
}

/// Where kid `k` (1 to 3) stands.
fn boy_at(k: usize) -> Vec3 {
    BOYS_AT[(k.clamp(1, 3)) - 1]
}

/// A boy's face as he stands, and Hızır's (workshop coordinates).
fn boy_face(k: usize) -> Vec3 {
    boy_at(k) + Vec3::new(0.0, 0.6, 0.0)
}
const HIZIR_FACE: Vec3 = Vec3::new(HIZIR_AT.x, 0.9, HIZIR_AT.z + 0.2);

/// The middle of the three, for facing them.
const BOYS_MID: Vec3 = Vec3::new(-4.0, 0.0, -1.95);

impl WorldView {
    /// The workshop's room made ready for a scene with `hero` played (its dressing is his).
    pub fn prepare_after(&mut self, hero: HeroKind) {
        self.after_hero = hero;
        self.prepare_workshop_room(hero);
    }

    /// Who speaks now, as the kid he is (`None` for Hızır and the action).
    fn after_speaker(sv: StoryView, hero: HeroKind) -> (Option<usize>, bool) {
        let beats = sv.scene.beats();
        let b = &beats[sv.beat.min(beats.len() - 1)];
        match b.speaker {
            Speaker::Boy(role) => (Some(kid_of_role(hero, role)), false),
            Speaker::OldMan => (None, true),
            _ => (None, false),
        }
    }

    /// The scene at Hızır's, this frame.
    pub(super) fn emit_after(&self, sv: StoryView, s: &SceneState, f: &mut Frame) {
        let Scene::After(sin) = sv.scene else { return };
        let time = s.time as f32;
        let hero = s.hero_kind;
        let at = |p: Vec3| WORKSHOP + p;
        let room = WorkshopView {
            station: Station::Anvil,
            hero,
            owned: [1, 0, 1, 1],
            wield: 1,
            carry: None,
            belt: [None; BELT_SLOTS],
            work: None,
            mood: QuitmerMood::Calm,
            trophies: [None; TROPHY_SLOTS],
            sigils: [None; 7],
            news: 0,
        };
        self.emit_workshop_room(f, room, time);
        f.haze = Haze::default();
        let shot = sv.shot();
        let t = sv.t;
        let (speaker, hizir_speaks) = Self::after_speaker(sv, hero);
        let gather = sv.since(Shot::Gather);
        let lines_before = sv.scene.beats()[..sv.beat.min(sv.scene.beats().len())].iter().filter(|b| b.speaker == Speaker::OldMan).count();

        // ---- Toby's thing on the anvil, its warm glow swelling as the scene comes up
        let token = Token::of(sin);
        let swell = smooth(stage::FADE_IN, stage::GLOW + 0.8, gather).max(if gather > 100.0 { 1.0 } else { 0.0 });
        let show = if shot == Shot::Show { smooth(0.0, 0.8, t) } else { 0.0 };
        // (At the cast's size: the boys are children, three quarters of a metre tall.)
        let root = Mat4::from_translation(at(TOKEN_AT)) * Mat4::from_rotation_y(0.35) * Mat4::from_scale(Vec3::splat(0.62));
        token_model(f, token, root, 0.03 + 0.05 * show);
        let warm = lin(TOBY_COLOR);
        glow_orb(f, at(TOKEN_AT + Vec3::Y * 0.08), 0.45, warm, (0.05 + 0.06 * show) * swell + 0.015 * (time * 2.5).sin());
        f.lights.push(Light { pos: at(TOKEN_AT + Vec3::new(0.0, 0.6, 0.4)), color: warm * (0.6 + 0.8 * show), distance: 2.0 });

        // ---- Hızır, across the anvil, leaning on his staff
        let look_at = match speaker {
            Some(k) => boy_face(k),
            None if shot == Shot::Show => TOKEN_AT,
            None => BOYS_MID + Vec3::Y * 0.6,
        };
        let om_yaw = yaw_to(HIZIR_AT, BOYS_MID);
        let head = (yaw_to(HIZIR_AT, look_at) - om_yaw).clamp(-0.8, 0.8);
        let act = if hizir_speaks { OldAct::Talk { t, line: lines_before } } else { OldAct::Stand { doze: 0.0 } };
        if !self.emit_old_man_at(f, act, at(HIZIR_AT), om_yaw, head, 0.0, time) {
            let talk = hizir_speaks.then_some(t);
            self.emit_old_man(f, at(HIZIR_AT), om_yaw, head, 0.0, 0.0, talk, time);
        }

        // ---- Quitmer, watching from his place by the hearth, his tail going now and then
        let wag = 0.3 + 0.5 * smooth(1.0, 1.6, gather) * (1.0 - smooth(2.5, 3.5, gather)).max(0.4);
        let dog_yaw = yaw_to(DOG_AT, BOYS_MID) - 0.3;
        self.emit_dog(f, at(DOG_AT), dog_yaw, 0.0, 0.0, 1.0, 0.0, 0.0, wag, time, Some(DogAct::Lie { sleep: 0.0, wag }));

        // ---- the three boys, and their warriors' souls behind them
        let leave = sv.since(Shot::Leave);
        for k in 1..=3usize {
            let feet = boy_at(k);
            let face_anvil = yaw_to(feet, TOKEN_AT);
            let turning = smooth(stage::TURN, stage::TURN + 1.2, leave).min(1.0) * if (0.0..100.0).contains(&leave) { 1.0 } else { 0.0 };
            let yaw = lerp_angle(face_anvil, yaw_to(feet, Vec3::new(1.2, 0.0, 5.0)), turning * 0.6);
            let speaking = speaker == Some(k);
            let act = if speaking { Act::Talk(t) } else { Act::Stand };
            let aim = match speaker {
                Some(o) if o != k => Some(boy_face(o)),
                _ if hizir_speaks => Some(HIZIR_FACE),
                _ if shot == Shot::Show || shot == Shot::Gather => Some(TOKEN_AT),
                _ => None,
            };
            self.emit_kid(f, k, kid_root(at(feet), yaw), act, time, 0.0, Vec3::ZERO, aim.map(at));
            // His warrior's soul, standing behind him in spirit light.
            let away = (feet - TOKEN_AT) * Vec3::new(1.0, 0.0, 1.0);
            let soul = feet + away.normalize_or(Vec3::Z) * SOUL_BACK;
            // (Softer than as they rose in Scene I: here they stand by, and the boys are the subject.)
            let tint = lin(soul_color(warrior_of(k))) * 0.6;
            let rise = smooth(0.3, 1.8, gather).max(if gather > 100.0 { 1.0 } else { 0.0 });
            self.emit_soul(f, warrior_of(k), at(soul), face_anvil, rise, 0.92, tint, time + k as f32 * 1.3);
        }
        // A warm key on the faces from the fire's side, and a cool fill from the door.
        f.lights.push(Light { pos: at(Vec3::new(-4.0, 1.8, -0.6)), color: lin(0xffd8b0) * 2.6, distance: 4.5 });
        f.lights.push(Light { pos: at(Vec3::new(-4.3, 1.7, -2.9)), color: lin(0xffc89a) * 1.6, distance: 3.0 });
    }

    /// The scene's camera: a wide as it comes up, over the boys' shoulders on Hızır, from beside
    /// him on the boy who speaks, close on Toby's thing as its finder speaks of it, and drawing
    /// back to the room as they go.
    pub(super) fn after_camera(&self, sv: StoryView, hero: HeroKind) -> StoryCam {
        let t = sv.t;
        let v = Vec3::new;
        let (speaker, hizir) = Self::after_speaker(sv, hero);
        let medium = Dof::behind(1.4, 6.0, 4.5);
        let shallow = Dof::behind(0.45, 2.4, 6.0);
        let (eye, look, focus, fov, dof) = match sv.shot() {
            Shot::Gather => {
                // From behind the anvil, over Hızır's shoulder: the boys' faces, their souls behind them.
                let u = smooth(0.0, 4.2, t);
                (v(-2.75, 1.8, -4.35).lerp(v(-3.05, 1.6, -4.25), u), v(-4.15, 0.6, -1.7), BOYS_MID, 46.0, medium)
            }
            Shot::Show => {
                let u = smooth(0.0, 5.0, t);
                let eye = TOKEN_AT + v(0.45, 0.4, 0.95).lerp(v(0.36, 0.33, 0.75), u);
                (eye, TOKEN_AT + v(0.0, 0.05, 0.0), TOKEN_AT, 30.0, shallow)
            }
            Shot::Leave => {
                // Drawing back and up from behind the anvil, the room and the fire round them.
                let u = smooth(0.0, 3.0, t);
                (v(-2.6, 1.6, -4.3).lerp(v(-1.2, 2.5, -4.4), u), v(-4.1, 0.55, -1.9), BOYS_MID, 46.0, medium)
            }
            _ if hizir => {
                // Between the boys' heads, on him.
                let drift = (t * 0.2).min(1.0) * 0.08;
                (v(-3.62, 1.02, -1.38 - drift), HIZIR_FACE + v(0.0, -0.04, 0.0), HIZIR_FACE, 34.0, shallow)
            }
            _ => {
                let k = speaker.unwrap_or(2);
                let face = boy_face(k);
                // From beside Hızır, a little to the anvil's other side from the boy.
                let to = (face - HIZIR_FACE) * v(1.0, 0.0, 1.0);
                let dir = to.normalize_or(Vec3::Z);
                let side = v(dir.z, 0.0, -dir.x);
                let eye = face - dir * 1.6 + side * 0.4 + v(0.0, 0.24, 0.0);
                // (Looking a little under the face, so it sits above the dialogue box.)
                (eye, face + v(0.0, -0.12, 0.0), face, 36.0, shallow)
            }
        };
        let sway = Vec3::new((sv.age * 0.53).sin() * 0.012, (sv.age * 0.71).sin() * 0.008, (sv.age * 0.37).cos() * 0.008);
        StoryCam {
            eye: WORKSHOP + eye + sway,
            look: WORKSHOP + look + sway * 0.5,
            focus: WORKSHOP + focus,
            lens: Lens { fov, roll: 0.0, dof },
        }
    }

    /// The shadow map's reach in the scene: the anvil and those round it.
    pub(super) fn after_shadow_box(&self) -> (Vec3, f32) {
        (WORKSHOP + Vec3::new(-4.1, 0.8, -2.5), 3.6)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::sins::SINS;

    /// Every moment of every scene: the camera stays in the room and on its subject, in front of it.
    #[test]
    fn the_camera_keeps_to_the_room_and_looks_at_the_one_who_speaks() {
        let view = WorldView::new();
        for sin in SINS {
            let scene = Scene::After(sin);
            for hero in crate::logic::heroes::ROSTER {
                for (beat, b) in scene.beats().iter().enumerate() {
                    for k in 0..4 {
                        let t = k as f32 * 1.5;
                        let sv = StoryView { scene, beat, t, age: t, shown: 1e3, souls: 0 };
                        let cam = view.after_camera(sv, hero);
                        let local = cam.eye - WORKSHOP;
                        assert!(local.x.abs() < 6.8 && local.z.abs() < 4.8 && local.y > 0.3 && local.y < 3.0, "{sin:?} {b:?}: {local}");
                        assert!(cam.eye.distance(cam.look) > 0.3 && cam.eye.distance(cam.focus) < 7.0, "{sin:?} beat {beat}");
                    }
                }
            }
        }
    }
}
