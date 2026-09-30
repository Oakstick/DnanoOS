//! The camera: the chase view behind the hero and the top view over him, looking around,
//! the close-ups the tour and captures use, the map view, and the projections for the GPU.

use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CameraMode {
    Third,
    Top,
}

/// Over-the-shoulder chase camera tuning (see the web build for the reasoning).
pub(super) mod chase {
    pub const DISTANCE: f32 = 3.4;
    pub const SHOULDER: f32 = 0.3;
    pub const PIVOT_HEIGHT: f32 = 1.7;
    pub const REST_PITCH: f32 = -0.44;
    pub const LOOK_AHEAD: f32 = 2.2;
    pub const MIN_PITCH: f32 = -1.2;
    pub const MAX_PITCH: f32 = 0.1;
    pub const MIN_DISTANCE: f32 = 0.9;
    pub const POS_LERP: f32 = 16.0;
    /// Radians of turn per pixel of mouse movement.
    pub const SENSITIVITY: f32 = 0.0026;
    /// Without the mouse captured the view drifts back behind him at this rate.
    pub const FOLLOW_LERP: f32 = 3.5;
    /// How quickly the view swings to keep a boss in frame.
    pub const FOCUS_LERP: f32 = 2.2;
}

/// Undergrowth is at least this tall, which is what the camera has to squeeze past.
const WALL_TOP: f32 = 0.7;

/// Top-down follow camera: high and slightly south of the hero, looking north.
const TOP_HEIGHT: f32 = 8.2;

const TOP_BACK: f32 = 5.0;

const TOP_LOOK_AHEAD: f32 = 0.4;

const TOP_LERP: f32 = 5.0;

pub const FOV_DEGREES: f32 = 50.0;

pub const NEAR: f32 = 0.1;

pub const FAR: f32 = 80.0;

/**
 * How far back the camera can sit before undergrowth comes between it and
 * Ragnar. Walls are whole tiles, so the ray is walked against the grid.
 */
fn clear_behind(grid: &Grid, pivot: Vec3, dir: Vec3) -> f32 {
    let steps = 14;
    for i in 1..=steps {
        let d = chase::DISTANCE * i as f32 / steps as f32;
        let x = js_round((pivot.x - dir.x * d) as f64) as i32;
        let z = js_round((pivot.z - dir.z * d) as f64) as i32;
        let y = pivot.y - dir.y * d;
        if grid.get(x, z) != Tile::Rock {
            continue;
        }
        if y > grid.elevation(x, z) as f32 * STEP + WALL_TOP {
            continue;
        }
        return chase::MIN_DISTANCE.max(d - chase::DISTANCE / steps as f32);
    }
    chase::DISTANCE
}

/// The direction from `target` with the most open ground, so a close camera sits over a road.
fn open_angle(grid: &Grid, target: Vec3) -> f32 {
    let mut best = (0, PI / 2.0);
    for k in 0..16 {
        let a = k as f32 / 16.0 * TAU;
        let mut open = 0;
        for d in [0.8f32, 1.6, 2.4, 3.2] {
            let x = js_round((target.x + a.cos() * d) as f64) as i32;
            let z = js_round((target.z + a.sin() * d) as f64) as i32;
            if grid.get(x, z) != Tile::Rock {
                open += 1;
            } else {
                break;
            }
        }
        // Prefer looking in from the south (the usual view) when it is a tie.
        let score = open * 10 + if a.sin() > 0.3 { 1 } else { 0 };
        if score > best.0 {
            best = (score, a);
        }
    }
    best.1
}

impl WorldView {
    pub(super) fn update_camera(&mut self, dt: f32, grid: &Grid, hero: &Hero) {
        let hp = hero.pos();
        let (hx, hz) = (hp.x as f32, hp.y as f32);
        // Follow the floor under him, not the jump arc, so the view does not bounce on every hop.
        let focus = Vec3::new(hx, floor_at(grid, hx, hz), hz);
        self.level_dims = Vec2::new(grid.width as f32, grid.height as f32);
        if self.showcase != self.was_showcase || self.forge != self.was_forge || self.select.is_some() != self.was_select || self.story.is_some() != self.was_story {
            self.was_showcase = self.showcase;
            self.was_forge = self.forge;
            self.was_select = self.select.is_some();
            self.was_story = self.story.is_some();
            self.initialized = false;
        }
        if self.tour.is_none() {
            self.tour_index = None;
        }
        let story_cam = self.story.map(|sv| self.story_camera(sv));
        let (desired, look, pos_lerp) = if let Some((eye, look, _)) = story_cam {
            // The story's shots are placed exactly, cut to cut.
            self.initialized = false;
            (eye, look, 1.0)
        } else if let Some((target, t, index)) = self.tour {
            // A dramatic swoop: in from high and wide, then a slow close orbit.
            if self.tour_index != Some(index) {
                self.tour_index = Some(index);
                self.tour_angle = open_angle(grid, target);
            }
            let e = 1.0 - (1.0 - (t / 0.7).min(1.0)).powi(3);
            let a = self.tour_angle + t * 0.22;
            let r = 2.5 + 3.6 * (1.0 - e);
            let h = 2.15 + 3.0 * (1.0 - e);
            (target + Vec3::new(a.cos() * r, h, a.sin() * r), target + Vec3::new(0.0, 0.35, 0.0), 7.0 + 6.0 * e)
        } else if let Some(sel) = self.select {
            // The character screen: a low, heroic view of the line-up, drifting toward whoever is picked.
            let goal = select::slot_feet(sel.index, 1.0).x - select::SELECT_STAGE.x;
            self.select_cam += (goal * 0.32 - self.select_cam) * (1.0 - (-3.0 * dt).exp());
            self.showcase_angle += dt;
            let sway = (self.showcase_angle * 0.31).sin() * 0.07;
            let base = select::SELECT_STAGE;
            (base + Vec3::new(self.select_cam + sway, 1.0, 3.35), base + Vec3::new(self.select_cam * 1.15, 0.78, 0.0), 6.0)
        } else if self.forge {
            // The old man's crypt fills the left of the screen; his wares sit on the right.
            self.showcase_angle += dt;
            let (eye, look) = self.wares_camera(self.showcase_angle);
            (eye, look, 6.0)
        } else if self.showcase {
            // The title diorama: the camera sways gently in front of Ragnar.
            self.showcase_angle += dt;
            let a = (self.showcase_angle * 0.23).sin() * 0.5;
            (STAGE + Vec3::new(a * 1.1, 1.2, 3.9), STAGE + Vec3::new(a * 0.3, 0.72, -0.2), 6.0)
        } else if self.camera_mode == CameraMode::Third {
            if self.recentering {
                let k = 1.0 - (-10.0 * dt).exp();
                let goal = dir_to_yaw(hero.facing);
                self.cam_yaw = lerp_angle(self.cam_yaw, goal, k);
                self.cam_pitch += (chase::REST_PITCH - self.cam_pitch) * k;
                let mut off = (self.cam_yaw - goal).rem_euclid(TAU);
                if off > PI {
                    off = TAU - off;
                }
                if off < 0.01 && (self.cam_pitch - chase::REST_PITCH).abs() < 0.01 {
                    self.recentering = false;
                }
            } else if !self.mouse_look {
                // In a boss fight the view turns toward the boss rather than trailing his heading.
                let target = self.focus.map(|p| p - focus).filter(|d| d.x * d.x + d.z * d.z > 2.5);
                match target {
                    Some(d) => self.cam_yaw = lerp_angle(self.cam_yaw, d.z.atan2(d.x), 1.0 - (-chase::FOCUS_LERP * dt).exp()),
                    None if hero.free_move => {
                        // Free movement: trail where he runs, unless he is backing toward the camera.
                        let goal = hero.heading as f32;
                        let mut off = (goal - self.cam_yaw).rem_euclid(TAU);
                        if off > PI {
                            off = TAU - off;
                        }
                        if hero.vel > 0.3 && off < 1.75 && self.follow_heading {
                            self.cam_yaw = lerp_angle(self.cam_yaw, goal, 1.0 - (-chase::FOLLOW_LERP * 0.6 * dt).exp());
                        }
                    }
                    None => self.cam_yaw = lerp_angle(self.cam_yaw, dir_to_yaw(hero.facing), 1.0 - (-chase::FOLLOW_LERP * dt).exp()),
                }
            }
            let cp = self.cam_pitch.cos();
            let dir = Vec3::new(cp * self.cam_yaw.cos(), self.cam_pitch.sin(), cp * self.cam_yaw.sin());
            let right = Vec3::new(-self.cam_yaw.sin(), 0.0, self.cam_yaw.cos());
            let pivot = focus + Vec3::new(0.0, chase::PIVOT_HEIGHT, 0.0) + right * chase::SHOULDER;
            let desired = pivot - dir * clear_behind(grid, pivot, dir);
            let look = pivot + dir * chase::LOOK_AHEAD;
            (desired, look, chase::POS_LERP)
        } else {
            (focus + Vec3::new(0.0, TOP_HEIGHT, TOP_BACK), focus + Vec3::new(0.0, 0.0, -TOP_LOOK_AHEAD), TOP_LERP)
        };
        self.focus_point = if let Some((_, _, focus)) = story_cam {
            focus
        } else if let Some((target, _, _)) = self.tour {
            target
        } else if let Some(sel) = self.select {
            select::slot_feet(sel.index, 1.0) + Vec3::new(0.0, 0.7, 0.0)
        } else if self.forge {
            story::GATEWAY + Vec3::new(-0.4, 0.8, -0.6)
        } else if self.showcase {
            STAGE + Vec3::new(0.0, 0.6, 0.3)
        } else {
            focus + Vec3::new(0.0, 0.5, 0.0)
        };
        if !self.initialized {
            self.cam_pos = desired;
            self.cam_look = look;
            self.initialized = true;
        } else {
            let k = 1.0 - (-pos_lerp * dt).exp();
            self.cam_pos = self.cam_pos.lerp(desired, k);
            self.cam_look = self.cam_look.lerp(look, k);
        }
    }

    /// Turns the camera by a mouse movement in pixels.
    pub fn look(&mut self, dx: f64, dy: f64) {
        if dx != 0.0 || dy != 0.0 {
            self.recentering = false; // the player took the camera back
        }
        self.cam_yaw += dx as f32 * chase::SENSITIVITY;
        let p = self.cam_pitch - dy as f32 * chase::SENSITIVITY;
        self.cam_pitch = p.clamp(chase::MIN_PITCH, chase::MAX_PITCH);
    }

    /// Swings the chase camera back behind Ragnar (R3); it follows him again afterwards.
    pub fn recenter(&mut self) {
        self.recentering = true;
    }

    /// Points the camera along a grid direction, for a fresh life or dungeon.
    pub fn aim_along(&mut self, dir: Dir) {
        self.cam_yaw = dir_to_yaw(dir);
        self.cam_pitch = chase::REST_PITCH;
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.cam_pos + self.shake_offset, self.cam_look + self.shake_offset * 0.6, Vec3::Y)
    }

    /// Freezes the camera on a close portrait of a point, from `angle` radians around it.
    pub fn set_closeup(&mut self, target: Vec3, angle: f32, distance: f32) {
        self.focus_point = target;
        self.cam_look = target;
        self.cam_pos = target + Vec3::new(angle.sin() * distance, distance * 0.28, angle.cos() * distance);
        self.shake_offset = Vec3::ZERO;
    }

    /// A close view like `set_closeup`, from `rise` times the distance above the point.
    pub fn set_closeup_rise(&mut self, target: Vec3, angle: f32, distance: f32, rise: f32) {
        self.set_closeup(target, angle, distance);
        self.cam_pos = target + Vec3::new(angle.sin() * distance, distance * rise, angle.cos() * distance);
    }

    /// Freezes the camera into a classic isometric view of the whole grid.
    pub fn set_map_view(&mut self, grid: &Grid) {
        let (w, h) = (grid.width as f32, grid.height as f32);
        let center = Vec3::new(w / 2.0 - 0.5, 0.0, h / 2.0 - 0.5);
        // 45 degrees around, 35.26 degrees down: the true isometric angle.
        let dir = Vec3::ONE.normalize();
        self.cam_look = center;
        self.cam_pos = center + dir * 80.0;
        self.shake_offset = Vec3::ZERO;
        // The diamond of the map is (w + h) / sqrt(2) wide and about half that tall on screen.
        let across = (w + h) / 2f32.sqrt();
        self.map_view = Some((across / self.aspect.max(0.01)).max(across * 0.5 + 2.0) * 0.53);
    }

    /// The sky's view rays: forward, and right and up scaled to the screen edges. The pitch is
    /// held a little above the horizon, so the stars stay up there even under a map camera.
    pub fn sky_basis(&self) -> (Vec3, Vec3, Vec3) {
        let d = (self.cam_look - self.cam_pos).normalize_or(Vec3::NEG_Z);
        let yaw = d.z.atan2(d.x);
        let pitch = d.y.clamp(-1.0, 1.0).asin().max(0.15);
        let fwd = Vec3::new(pitch.cos() * yaw.cos(), pitch.sin(), pitch.cos() * yaw.sin());
        let right = fwd.cross(Vec3::Y).normalize();
        let up = right.cross(fwd);
        let t = (FOV_DEGREES.to_radians() * 0.5).tan();
        (fwd, right * t * self.aspect.max(0.01), up * t)
    }

    /// The camera for reconstructing world positions from depth: position, forward, and right
    /// and up scaled to the screen edges (per unit of depth, or in world units when orthographic).
    pub fn camera_basis(&self) -> (Vec3, Vec3, Vec3, Vec3, bool) {
        let pos = self.cam_pos + self.shake_offset;
        let fwd = (self.cam_look + self.shake_offset * 0.6 - pos).normalize_or(Vec3::NEG_Z);
        let right = fwd.cross(Vec3::Y).normalize_or(Vec3::X);
        let up = right.cross(fwd);
        let aspect = self.aspect.max(0.01);
        match self.map_view {
            Some(half) => (pos, fwd, right * half * aspect, up * half, true),
            None => {
                let t = (FOV_DEGREES.to_radians() * 0.5).tan();
                (pos, fwd, right * t * aspect, up * t, false)
            }
        }
    }

    /// How far in front of the camera the in-focus plane lies; None turns depth of field off.
    pub fn focus_depth(&self) -> Option<f32> {
        if self.map_view.is_some() {
            return None;
        }
        let (pos, fwd, ..) = self.camera_basis();
        Some((self.focus_point - pos).dot(fwd).max(0.5))
    }

    /// Where hero `i` of the character screen's line-up stands, his feet in the world.
    pub fn select_feet(&self, i: usize) -> Vec3 {
        select::slot_feet(i, self.select_step[i.min(2)])
    }

    /// The region the shadow map covers: its center and radius.
    /// Where the fine shadow cascade centers: on the ground a little past where the
    /// camera looks. None where the coarse one is enough (the map, the title, the shop).
    pub fn shadow_focus(&self) -> Option<Vec3> {
        if self.forge || self.showcase || self.select.is_some() || self.story.is_some() || self.map_view.is_some() {
            return None;
        }
        let fwd = self.cam_look - self.cam_pos;
        let flat = Vec3::new(fwd.x, 0.0, fwd.z).normalize_or_zero();
        Some(Vec3::new(self.cam_look.x, self.focus_point.y, self.cam_look.z) + flat * 3.0)
    }

    pub fn shadow_box(&self) -> (Vec3, f32) {
        if let Some(sv) = self.story {
            self.story_shadow_box(sv)
        } else if self.select.is_some() {
            (select::SELECT_STAGE + Vec3::new(0.0, 0.6, -0.6), 6.5)
        } else if self.forge {
            (story::GATEWAY + Vec3::new(-0.5, 1.0, -0.5), 5.5)
        } else if self.showcase {
            (STAGE, 5.0)
        } else {
            let d = self.level_dims;
            (Vec3::new(d.x / 2.0 - 0.5, 0.5, d.y / 2.0 - 0.5), d.length() / 2.0 + 2.5)
        }
    }

    pub fn projection(&self) -> Mat4 {
        if let Some(half) = self.map_view {
            let hw = half * self.aspect.max(0.01);
            return Mat4::orthographic_rh(-hw, hw, -half, half, 1.0, 200.0);
        }
        Mat4::perspective_rh(FOV_DEGREES.to_radians(), self.aspect.max(0.01), NEAR, FAR)
    }

    pub fn view_proj(&self) -> Mat4 {
        self.projection() * self.view_matrix()
    }

    pub fn camera_position(&self) -> Vec3 {
        self.cam_pos
    }

    /// World point to window coordinates (logical pixels), or None if behind the camera.
    pub fn project(&self, p: Vec3, width: f32, height: f32) -> Option<(f32, f32)> {
        let clip = self.view_proj() * Vec4::new(p.x, p.y, p.z, 1.0);
        if clip.w <= 1e-5 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        if ndc.z > 1.0 {
            return None;
        }
        Some(((ndc.x + 1.0) / 2.0 * width, (1.0 - ndc.y) / 2.0 * height))
    }
}
