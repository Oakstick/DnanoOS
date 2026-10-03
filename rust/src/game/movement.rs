//! How the hero's input becomes movement: the corridor he means on the grid, the free-movement wish,
//! the chase camera's facing, and his footsteps.

use super::Game;
use crate::audio::Sfx;
use crate::logic::types::{dist_sq, relative_dir, Dir, CARDINALS, V};
use crate::render::view::{dir_to_yaw, CameraMode};
use std::f64::consts::PI;

/// Tiles walked between footsteps.
const STRIDE_TILES: f64 = 0.5;

impl Game {
    /**
     * Movement input. In third person a key means "relative to where the camera
     * looks": W runs into the view, A and D step to either side, S backs out.
     */
    pub(super) fn resolve_wanted_dir(&mut self) -> Dir {
        let raw = self.input.direction();
        if self.view.camera_mode != CameraMode::Third || raw == Dir::None {
            self.latch = None;
            return raw;
        }
        // When the camera trails his heading, a turn on the spot swings the view,
        // which would re-map the still-held key and spin him again. So a held key
        // keeps the corridor it meant when pressed. With the mouse steering, the
        // camera does not follow him, and running where it looks stays live.
        if !self.view.mouse_look {
            if let Some((key, abs)) = self.latch {
                if key == raw {
                    return abs;
                }
            }
        }
        let abs = relative_dir(raw, self.camera_facing());
        self.latch = Some((raw, abs));
        abs
    }

    /// Free movement's direction in grid units: the held keys, turned to match the camera behind him.
    pub(super) fn free_wish(&mut self) -> V {
        let (rx, ry) = self.input.move_vector();
        // The chase camera swings round behind him only while he runs (nearly) straight ahead;
        // running diagonally, sideways or back it holds still, so a held direction stays put.
        self.view.follow_heading = ry < -0.97;
        if self.view.camera_mode != CameraMode::Third {
            return V::new(rx, ry);
        }
        let yaw = self.view.cam_yaw as f64;
        let (fx, fy) = (yaw.cos(), yaw.sin());
        let (sx, sy) = (-fy, fx);
        V::new(fx * -ry + sx * rx, fy * -ry + sy * rx)
    }

    /// The corridor the camera is pointing down, with hysteresis near the diagonals.
    pub(super) fn camera_facing(&mut self) -> Dir {
        let yaw = self.view.cam_yaw as f64;
        let off = |d: Dir| {
            let mut a = (yaw - dir_to_yaw(d) as f64) % (PI * 2.0);
            if a > PI {
                a -= PI * 2.0;
            }
            if a < -PI {
                a += PI * 2.0;
            }
            a.abs()
        };
        if self.cam_dir != Dir::None && off(self.cam_dir) < PI * 0.32 {
            return self.cam_dir;
        }
        let mut best = Dir::Down;
        let mut best_off = f64::INFINITY;
        for d in CARDINALS {
            let a = off(d);
            if a < best_off {
                best_off = a;
                best = d;
            }
        }
        self.cam_dir = best;
        best
    }

    /// Footsteps while running, a thud and a puff of dust on landing.
    pub(super) fn footsteps(&mut self, before: V) {
        let airborne = self.hero.airborne();
        if self.was_airborne && !airborne {
            let hp = self.hero.pos();
            self.audio.play(Sfx::Land);
            self.view.dust(&self.level.grid, hp.x, hp.y, 0x4a3620, 12, 1.8);
            self.view.shake(0.07);
            self.step_dist = 0.0;
        }
        self.was_airborne = airborne;
        if airborne || !self.hero.alive {
            return;
        }
        let after = self.hero.pos();
        let moved = dist_sq(before, after).sqrt();
        if moved <= 0.0 || moved > 1.5 {
            return; // standing, or wrapping through a tunnel
        }
        self.step_dist += moved;
        if self.step_dist >= STRIDE_TILES {
            self.step_dist -= STRIDE_TILES;
            let speed = if self.hero.m.speed > 0.0 { (self.hero.vel / self.hero.m.speed).clamp(0.3, 1.0) } else { 0.5 };
            let slimy = self.hero.slimed();
            self.audio.play_step(speed, slimy);
            let color = if slimy { 0x5cff6a } else { 0x4a3620 };
            self.view.dust(&self.level.grid, after.x, after.y, color, 2, 0.7);
        }
    }
}
