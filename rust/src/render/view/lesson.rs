//! Hızır and Quitmer in the Garden while he teaches his lesson (B04): come as he does when called,
//! in a breath of light, standing by to watch the duel, Quitmer lying at his heel.

use super::cast_clips::{DogAct, OldAct};
use super::frame::Frame;
use super::{floor_at, glow_orb, lin, SceneState, WorldView};
use glam::Vec3;

/// The lesson as the view needs it: where Hızır and Quitmer stand (tiles), which way Hızır faces
/// (radians in the ground plane), seconds since they came, and seconds into his line (if speaking).
#[derive(Clone, Copy, Debug)]
pub struct LessonView {
    pub hizir: (f64, f64),
    pub dog: (f64, f64),
    pub facing: f64,
    pub age: f32,
    pub talk: Option<f32>,
}

impl WorldView {
    pub(super) fn emit_lesson(&self, s: &SceneState, time: f32, f: &mut Frame) {
        let Some(l) = s.lesson else { return };
        let at = |p: (f64, f64)| Vec3::new(p.0 as f32, floor_at(s.grid, p.0 as f32, p.1 as f32), p.1 as f32);
        let (dx, dz) = (l.facing.cos() as f32, l.facing.sin() as f32);
        let yaw = dx.atan2(dz);
        let feet = at(l.hizir);
        let act = match l.talk {
            Some(t) => OldAct::Talk { t, line: 0 },
            None => OldAct::Stand { doze: 0.0 },
        };
        if !self.emit_old_man_at(f, act, feet, yaw, 0.0, 0.0, time) {
            self.emit_old_man(f, feet, yaw, 0.0, 0.0, 0.0, l.talk, time);
        }
        let dog = at(l.dog);
        self.emit_dog(f, dog, yaw, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, time, Some(DogAct::Lie { sleep: 0.0, wag: 0.5 }));
        // The light he came in, fading; a little of it stays about him while he teaches.
        let come = (1.0 - l.age / 1.2).max(0.0);
        glow_orb(f, feet + Vec3::Y * 0.8, 0.9 + come, lin(0xffd890), 0.12 + 0.5 * come);
    }
}
