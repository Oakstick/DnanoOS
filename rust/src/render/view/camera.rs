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

/// The chase camera's resting pitch (for tools that pose it).
pub const REST_PITCH: f32 = chase::REST_PITCH;

/// A jump in where the hero is, in one frame, that the camera cuts to instead of following.
const JUMP_CUT: f32 = 4.0;

/// Top-down follow camera: high and slightly south of the hero, looking north.
const TOP_HEIGHT: f32 = 8.2;

const TOP_BACK: f32 = 5.0;

const TOP_LOOK_AHEAD: f32 = 0.4;

const TOP_LERP: f32 = 5.0;

/// How long a switch between the two views blends its gaze onto him.
const SWITCH_SECONDS: f32 = 0.45;

/// How fast the top view's gaze follows him (its eye follows at `TOP_LERP`).
const TOP_LOOK_LERP: f32 = 14.0;

pub const FOV_DEGREES: f32 = 50.0;

pub const NEAR: f32 = 0.1;

pub const FAR: f32 = 80.0;

/// How far from the middle of the view he may sit, in radians (18 degrees).
const FRAME_ANGLE: f32 = 0.314;

/// The highest the chase camera may look when pressed in as close as it goes.
const CLOSE_PITCH: f32 = -1.0;

/// Room the camera keeps from the scenery: its near plane's corners reach this far (0.1 m out,
/// 50 degrees tall, 16:9), plus a little.
const EYE_ROOM: f32 = 0.18;

/// The solid scenery over each tile of a level, as the spans of height it fills (bottom, top),
/// for the chase camera to keep clear of: every opaque cube (water, glows, clouds, grass and
/// specks left out, and the cubes of trees, which fade from the view instead). Spans, not a
/// height, so the camera fits under an overhang: a gallery, a bridge, an arch (RP02).
#[derive(Default)]
pub(super) struct Spans {
    w: usize,
    h: usize,
    /// Everything the near plane must keep out of, and the opaque part alone (what hides him:
    /// he shows through glass).
    cols: Vec<Vec<(f32, f32)>>,
    opaque: Vec<Vec<(f32, f32)>>,
}

impl Spans {
    pub(super) fn build(cubes: &[Instance], skip: &[(usize, usize)], w: usize, h: usize) -> Spans {
        let mut cols: Vec<Vec<(f32, f32)>> = vec![Vec::new(); w * h];
        let mut opaque: Vec<Vec<(f32, f32)>> = vec![Vec::new(); w * h];
        let mut skipped = vec![false; cubes.len()];
        for &(start, len) in skip {
            for s in skipped.iter_mut().skip(start).take(len) {
                *s = true;
            }
        }
        for (c, _) in cubes.iter().zip(&skipped).filter(|(_, &s)| !s) {
            let shade = c.emissive[3];
            if shade == SHADE_GRASS || shade == SHADE_CLOUD || shade >= SHADE_WATER || c.color[3] <= 0.0 {
                continue;
            }
            let m = Mat4::from_cols_array(&c.m);
            let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for k in 0..8 {
                let l = Vec3::new(
                    if k & 1 == 0 { -0.5 } else { 0.5 },
                    if k & 2 == 0 { -0.5 } else { 0.5 },
                    if k & 4 == 0 { -0.5 } else { 0.5 },
                );
                let p = m.transform_point3(l);
                lo = lo.min(p);
                hi = hi.max(p);
            }
            if (hi - lo).max_element() < 0.12 {
                continue;
            }
            // The tiles it covers (tile x spans x - 0.5 to x + 0.5), not counting a sliver over the edge.
            let x0 = (lo.x + 0.55).floor().max(0.0) as usize;
            let x1 = ((hi.x + 0.45).floor().max(-1.0) as isize).min(w as isize - 1);
            let z0 = (lo.z + 0.55).floor().max(0.0) as usize;
            let z1 = ((hi.z + 0.45).floor().max(-1.0) as isize).min(h as isize - 1);
            if x1 < 0 || z1 < 0 {
                continue;
            }
            let clear = c.color[3] < 0.999;
            for z in z0..=z1 as usize {
                for x in x0..=x1 as usize {
                    cols[z * w + x].push((lo.y, hi.y));
                    if !clear {
                        opaque[z * w + x].push((lo.y, hi.y));
                    }
                }
            }
        }
        // Each column's spans merged where they touch.
        for col in cols.iter_mut().chain(opaque.iter_mut()) {
            col.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut merged: Vec<(f32, f32)> = Vec::with_capacity(col.len().min(4));
            for &(lo, hi) in col.iter() {
                match merged.last_mut() {
                    Some(last) if lo <= last.1 + 0.02 => last.1 = last.1.max(hi),
                    _ => merged.push((lo, hi)),
                }
            }
            *col = merged;
        }
        Spans { w, h, cols, opaque }
    }

    /// Is height `y` at (`x`, `z`) within `room` of the scenery there (of its opaque part, with
    /// `opaque_only`)?
    fn solid(&self, x: f32, z: f32, y: f32, room: f32, opaque_only: bool) -> bool {
        let (tx, tz) = (js_round(x as f64) as i64, js_round(z as f64) as i64);
        if tx < 0 || tz < 0 || tx >= self.w as i64 || tz >= self.h as i64 {
            return false;
        }
        let cols = if opaque_only { &self.opaque } else { &self.cols };
        cols[tz as usize * self.w + tx as usize].iter().any(|&(lo, hi)| y > lo - room && y < hi + room)
    }

    /// Would the camera's near plane at `p` cut into the scenery?
    fn cramped(&self, p: Vec3) -> bool {
        [(0.0, 0.0), (EYE_ROOM, 0.0), (-EYE_ROOM, 0.0), (0.0, EYE_ROOM), (0.0, -EYE_ROOM)]
            .iter()
            .any(|&(ox, oz)| self.solid(p.x + ox, p.z + oz, p.y, EYE_ROOM, false))
    }

    fn len(&self) -> usize {
        self.cols.len()
    }
}

/**
 * How far back the chase camera can sit before the scenery cuts into its near plane or comes
 * between it and Ragnar (`marks` on him), and whether he is seen from there: the ray back from
 * the pivot is walked in short steps against the scenery's spans (RP02).
 */
fn clear_behind(spans: &Spans, pivot: Vec3, dir: Vec3, marks: &[Vec3]) -> Behind {
    let sees = |p: Vec3| {
        marks.iter().any(|&m| {
            // (Fine steps, with a little to spare, so a line grazing a wall's top counts as blocked.)
            let n = (p.distance(m) / 0.1).ceil().max(1.0) as usize;
            !(1..n).any(|k| {
                let q = p.lerp(m, k as f32 / n as f32);
                q.distance(m) > 0.12 && spans.solid(q.x, q.z, q.y, 0.04, true)
            })
        })
    };
    let cramped = |p: Vec3| spans.cramped(p);
    let steps = 34;
    let mut last = 0.0;
    for i in 1..=steps {
        let d = chase::DISTANCE * i as f32 / steps as f32;
        let p = pivot - dir * d;
        // Each sight line to him, stopping just short of him.
        if cramped(p) || !sees(p) {
            let d = chase::MIN_DISTANCE.max(last);
            let at = pivot - dir * d;
            return Behind { d, seen: sees(at), fits: !cramped(at) };
        }
        last = d;
    }
    Behind { d: chase::DISTANCE, seen: true, fits: true }
}

/// Where the chase camera can sit along one heading: how far back, whether he is in sight from
/// there, and whether its near plane is clear of the scenery there.
#[derive(Clone, Copy)]
struct Behind {
    d: f32,
    seen: bool,
    fits: bool,
}

/**
 * The chase camera's eye and the point it looks at, for a heading and pitch, kept out of the
 * scenery with him in sight (RP02):
 *
 * - it backs off only as far as the scenery allows;
 * - pressed in close, it cannot tip up as far (or he would drop out of the bottom of the frame);
 * - where the heading asked for has no room or no sight of him at all (stood in a doorway,
 *   looking into the wall), it swings to the nearest heading that has, up to a quarter turn;
 * - and failing that it ducks under a low beam, or rises over what is in the way, and looks at
 *   him.
 *
 * The player's own heading and pitch are kept, for when there is room again.
 */
fn chase_pose(spans: &Spans, focus: Vec3, yaw: f32, pitch: f32) -> (Vec3, Vec3) {
    let marks = [focus + Vec3::Y * 0.6, focus + Vec3::Y * 1.0];
    let along = |yaw: f32, pitch: f32| Vec3::new(pitch.cos() * yaw.cos(), pitch.sin(), pitch.cos() * yaw.sin());
    let attempt = |yaw: f32| {
        let right = Vec3::new(-yaw.sin(), 0.0, yaw.cos());
        let pivot = focus + Vec3::new(0.0, chase::PIVOT_HEIGHT, 0.0) + right * chase::SHOULDER;
        let mut dir = along(yaw, pitch);
        let mut b = clear_behind(spans, pivot, dir, &marks);
        let close = 1.0 - ((b.d - chase::MIN_DISTANCE) / (chase::DISTANCE - chase::MIN_DISTANCE)).clamp(0.0, 1.0);
        let cap = chase::MAX_PITCH + close * (CLOSE_PITCH - chase::MAX_PITCH);
        if pitch > cap {
            dir = along(yaw, cap);
            b = clear_behind(spans, pivot, dir, &marks);
        }
        (pivot, dir, b)
    };
    let (mut pivot, mut dir, mut b) = attempt(yaw);
    if !(b.seen && b.fits) {
        'swing: for k in 1..=6 {
            for side in [1.0, -1.0] {
                let (p, d, c) = attempt(yaw + side * k as f32 * (PI / 12.0));
                if c.seen && c.fits {
                    (pivot, dir, b) = (p, d, c);
                    break 'swing;
                }
            }
        }
    }
    let mut look = pivot + dir * chase::LOOK_AHEAD;
    if !(b.seen && b.fits) {
        // (Down first, under a low beam; then up, over something low.)
        for lift in [-0.5, 0.6, 1.2, 1.8] {
            let raised = pivot + Vec3::Y * lift;
            let c = clear_behind(spans, raised, dir, &marks);
            if c.seen && c.fits {
                (pivot, b) = (raised, c);
                look = focus + Vec3::Y * 0.8;
                break;
            }
        }
    }
    let eye = pivot - dir * b.d;
    (eye, frame_him(eye, look, focus + Vec3::Y * 0.6))
}

/// The point to look at from `eye`, turned from `look` toward `him` just far enough that he sits
/// within `FRAME_ANGLE` of the middle of the view (the frame is 25 degrees to its top and bottom
/// edges), so the chase camera never leaves him at or past an edge (RP02).
fn frame_him(eye: Vec3, look: Vec3, him: Vec3) -> Vec3 {
    let (to_look, to_him) = ((look - eye).normalize_or_zero(), (him - eye).normalize_or_zero());
    let off = to_look.dot(to_him).clamp(-1.0, 1.0).acos();
    if off <= FRAME_ANGLE || to_look == Vec3::ZERO || to_him == Vec3::ZERO {
        return look;
    }
    let turned = Quat::from_rotation_arc(to_look, to_him);
    let (axis, _) = turned.to_axis_angle();
    eye + Quat::from_axis_angle(axis, off - FRAME_ANGLE) * (look - eye)
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
        // He went through a tunnel to the far side, or was carried off: the camera cuts to him
        // (easing there would sweep it through the whole level's walls, RP02).
        if self.last_focus.is_some_and(|f| f.distance(focus) > JUMP_CUT) {
            self.initialized = false;
        }
        self.last_focus = Some(focus);
        self.level_dims = Vec2::new(grid.width as f32, grid.height as f32);
        if self.showcase != self.was_showcase
            || self.forge != self.was_forge
            || self.select.is_some() != self.was_select
            || self.story.is_some() != self.was_story
        {
            self.was_showcase = self.showcase;
            self.was_forge = self.forge;
            self.was_select = self.select.is_some();
            self.was_story = self.story.is_some();
            self.initialized = false;
        }
        if self.tour.is_none() {
            self.tour_index = None;
        }
        if self.tops_dirty || self.tops.len() != grid.width as usize * grid.height as usize {
            let trees: Vec<(usize, usize)> = self.trees.iter().map(|t| t.range).collect();
            self.tops = Spans::build(&self.static_cubes, &trees, grid.width as usize, grid.height as usize);
            self.tops_dirty = false;
            self.fit_trees();
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
            // The old man's workshop: the station he stands at fills the left of the screen, the
            // wares sit on the right; from station to station the camera glides.
            self.showcase_angle += dt;
            let (eye, look) = self.workshop_camera(self.showcase_angle);
            (eye, look, 3.2)
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
            let (desired, look) = chase_pose(&self.tops, focus, self.cam_yaw, self.cam_pitch);
            (desired, look, chase::POS_LERP)
        } else {
            // High above him, and higher still over a tower that would cut into its view.
            let mut eye = focus + Vec3::new(0.0, TOP_HEIGHT, TOP_BACK);
            let mut lift = 0.0;
            while self.tops.cramped(eye) && lift < 4.0 {
                eye.y += 0.25;
                lift += 0.25;
            }
            (eye, focus + Vec3::new(0.0, 0.0, -TOP_LOOK_AHEAD), TOP_LERP)
        };
        self.focus_point = if let Some((_, _, focus)) = story_cam {
            focus
        } else if let Some((target, _, _)) = self.tour {
            target
        } else if let Some(sel) = self.select {
            select::slot_feet(sel.index, 1.0) + Vec3::new(0.0, 0.7, 0.0)
        } else if self.forge {
            self.workshop_focus()
        } else if self.showcase {
            STAGE + Vec3::new(0.0, 0.6, 0.3)
        } else {
            focus + Vec3::new(0.0, 0.5, 0.0)
        };
        // Just switched between the chase and the top view: while the eye swings over, the gaze
        // holds on him, so he never drops out of the frame on the way (RP02).
        if self.camera_mode != self.mode_was {
            self.mode_was = self.camera_mode;
            self.switching = 1.0;
        }
        self.switching = (self.switching - dt / SWITCH_SECONDS).max(0.0);
        let gameplay = story_cam.is_none() && self.tour.is_none() && self.select.is_none() && !self.forge && !self.showcase;
        let look = if gameplay && self.switching > 0.0 {
            let k = self.switching * self.switching * (3.0 - 2.0 * self.switching);
            look.lerp(focus + Vec3::Y * 0.8, k)
        } else {
            look
        };
        if !self.initialized {
            self.cam_pos = desired;
            self.cam_look = look;
            self.initialized = true;
        } else {
            let k = 1.0 - (-pos_lerp * dt).exp();
            self.cam_pos = self.cam_pos.lerp(desired, k);
            // The top view's gaze keeps up with him faster than its eye does, so a switch from
            // the chase view swings over without losing him off the frame (RP02).
            let look_rate = if gameplay && self.camera_mode == CameraMode::Top { pos_lerp.max(TOP_LOOK_LERP) } else { pos_lerp };
            self.cam_look = self.cam_look.lerp(look, 1.0 - (-look_rate * dt).exp());
        }
        // Easing toward its goal, the chase camera may pass where he would leave the frame (round
        // a corner, swinging after a switch): its gaze is held on him all the way (RP02).
        // Where easing would carry the eye through the scenery (either view), it cuts to its
        // goal, which is clear.
        if gameplay && self.map_view.is_none() && self.tops.cramped(self.cam_pos) {
            self.cam_pos = desired;
        }
        if gameplay && self.camera_mode == CameraMode::Third && self.map_view.is_none() {
            self.cam_look = frame_him(self.cam_pos, self.cam_look, focus + Vec3::Y * 0.6);
        }
    }

    /// A composed shot rather than play (the story, the title, the character screen, the forge,
    /// the tour): no foes to keep sharp in it.
    pub fn cinematic(&self) -> bool {
        self.story.is_some() || self.showcase || self.select.is_some() || self.forge || self.tour.is_some()
    }

    /// Puts the camera straight at its goal on the next update instead of easing there (a cut).
    pub fn snap_camera(&mut self) {
        self.initialized = false;
    }

    /// The hero's voxel size, for how big he is on screen.
    pub fn hero_voxel(&self) -> f32 {
        self.models.hero.size
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

    /// The eye this frame, shaken, kicked and rolled (WARBAND.md §4.3): where it is, what it looks
    /// at, and which way is up.
    fn eye(&self) -> (Vec3, Vec3, Vec3) {
        let kick = self.kick_offset();
        let pos = self.cam_pos + self.shake_offset + kick;
        let look = self.cam_look + self.shake_offset * 0.6 + kick * 0.5;
        let fwd = (look - pos).normalize_or(Vec3::NEG_Z);
        let up = if self.juice.roll != 0.0 { Quat::from_axis_angle(fwd, self.juice.roll) * Vec3::Y } else { Vec3::Y };
        (pos, look, up)
    }

    pub fn view_matrix(&self) -> Mat4 {
        let (pos, look, up) = self.eye();
        Mat4::look_at_rh(pos, look, up)
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
        let t = (self.fov_degrees().to_radians() * 0.5).tan();
        (fwd, right * t * self.aspect.max(0.01), up * t)
    }

    /// The camera for reconstructing world positions from depth: position, forward, and right
    /// and up scaled to the screen edges (per unit of depth, or in world units when orthographic).
    pub fn camera_basis(&self) -> (Vec3, Vec3, Vec3, Vec3, bool) {
        let (pos, look, sky) = self.eye();
        let fwd = (look - pos).normalize_or(Vec3::NEG_Z);
        let right = fwd.cross(sky).normalize_or(Vec3::X);
        let up = right.cross(fwd);
        let aspect = self.aspect.max(0.01);
        match self.map_view {
            Some(half) => (pos, fwd, right * half * aspect, up * half, true),
            None => {
                let t = (self.fov_degrees().to_radians() * 0.5).tan();
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
        if self.forge || self.showcase || self.select.is_some() || self.story.is_some() || self.map_view.is_some() || self.lookdev.is_some()
        {
            return None;
        }
        let fwd = self.cam_look - self.cam_pos;
        let flat = Vec3::new(fwd.x, 0.0, fwd.z).normalize_or_zero();
        Some(Vec3::new(self.cam_look.x, self.focus_point.y, self.cam_look.z) + flat * 3.0)
    }

    pub fn shadow_box(&self) -> (Vec3, f32) {
        if self.lookdev.is_some() {
            (super::lookdev::LOOKDEV + Vec3::new(0.0, 1.0, 0.0), 6.5)
        } else if let Some(sv) = self.story {
            self.story_shadow_box(sv)
        } else if self.select.is_some() {
            (select::SELECT_STAGE + Vec3::new(0.0, 0.6, -0.6), 6.5)
        } else if self.forge {
            self.workshop_shadow_box()
        } else if self.showcase {
            (STAGE, 5.0)
        } else {
            let d = self.level_dims;
            (Vec3::new(d.x / 2.0 - 0.5, 0.5, d.y / 2.0 - 0.5), d.length() / 2.0 + 2.5)
        }
    }

    /// The level's size in tiles (x, z), for what covers it (the probes).
    pub fn level_size(&self) -> (f32, f32) {
        (self.level_dims.x, self.level_dims.y)
    }

    /// How rigs choose their level of detail under this camera (see `LodPolicy`).
    pub fn lod_policy(&self) -> LodPolicy {
        let half_px = self.viewport_h.max(1.0) * 0.5;
        let (px_per_unit, ortho) = match self.map_view {
            Some(half) => (half_px / half.max(0.01), true),
            None => (half_px / (FOV_DEGREES.to_radians() * 0.5).tan(), false),
        };
        LodPolicy { eye: self.camera_position(), px_per_unit, ortho, bias: self.lod_bias, debug: self.lod_debug }
    }

    pub fn projection(&self) -> Mat4 {
        if let Some(half) = self.map_view {
            let hw = half * self.aspect.max(0.01);
            return Mat4::orthographic_rh(-hw, hw, -half, half, 1.0, 200.0);
        }
        Mat4::perspective_rh(self.fov_degrees().to_radians(), self.aspect.max(0.01), NEAR, FAR)
    }

    pub fn view_proj(&self) -> Mat4 {
        self.projection() * self.view_matrix()
    }

    pub fn camera_position(&self) -> Vec3 {
        self.cam_pos
    }

    /// The camera's roll now (radians), for tools and tests.
    pub fn camera_roll(&self) -> f32 {
        self.juice.roll
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
