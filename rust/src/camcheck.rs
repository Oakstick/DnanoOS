//! Camera validation (RP02 in the Technical Art and Rendering Plan): both gameplay cameras, in
//! every kind of level, checked against the scenery the level actually builds.
//!
//!   AdventuresOfTheAncientBlades --camera-check [all|dungeon0,hub,...] [--stride 2] [--walk 20]
//!                                               [--out perf]
//!
//! Two passes per level and camera:
//!
//! - **Survey.** Ragnar is set down on every `stride`-th open tile with the world paused; the chase
//!   camera is turned to eight headings at three pitches (rest, looking down hard, looking up),
//!   the top camera taken as it is. Each pose is held long enough for the scenery to fade out of
//!   the way, then checked.
//! - **Walk.** He wanders the level for `walk` seconds with the foes about, the camera swung by
//!   the mouse now and then and switched between the two views every few seconds, checked every
//!   frame. This catches what the survey's settled poses cannot: the camera's easing toward its
//!   goal, and switching.
//!
//! What is checked, against the level's static cubes (the solid ones: opaque, not fading, not
//! water, glow, cloud or grass):
//!
//! - **Clipping:** the eye's near-plane corners inside a solid cube (a wall cut open on screen).
//! - **Hidden hero:** all three points on him (shin, waist, chest) behind solid cubes, after the
//!   fade has had time to clear the view. In the walk, only a stretch longer than 0.3 s counts.
//! - **Off screen:** his waist outside the middle 90% of the screen.
//! - **Holes:** a view ray below the horizon that meets nothing where it crosses ground he walks
//!   on (not a pit, a well or a chasm, which are holes by design, nor a seam between slabs): the
//!   void showing through the level. Rays that run past the level's edge
//!   into its backdrop (the cavern, the sky, the lava) are counted apart, for the art review of
//!   vistas (TA06), not failed: the backdrop is the plan's background layer.
//! - **Depth of field:** the blur on any foe within 12 m of him (the plan: gameplay depth of field
//!   must not hide enemies), in pixels at 1080p; more than 1 px counts.
//! - **Level of detail:** the level his size on screen would call for under each camera at
//!   1080p. He is always drawn whole (`emit`), so this is reported, not failed.
//!
//! The report goes to `perf/camera-<time>.md`, with the worst poses to reproduce in `--capture`.
//! It exits non-zero when any check fails.

use std::collections::HashMap;
use std::fmt::Write as _;

use glam::{Mat4, Vec2, Vec3, Vec4};

use crate::app::STEP;
use crate::capture::arg;
use crate::game::{Game, Screen, Stage};
use crate::input::Action;
use crate::logic::rng::Rng;
use crate::logic::sins::SINS;
use crate::logic::types::{Dir, Tile, P};
use crate::render::gpu::{DOF_CLEAR, DOF_FULL, DOF_PX};
use crate::render::view::{CameraMode, Frame, Instance, WorldView, STEP as RISE};

/// The near plane's corners lie this far from the eye (0.1 m away, 50° tall, 16:9): a cube
/// closer than this cuts into the picture.
const NEAR_REACH: f32 = 0.14;
/// The blur a foe near the hero may carry, in pixels at 1080p.
const DOF_LIMIT: f32 = 1.0;
/// Foes this close to him are in play and must stay sharp.
const THREAT_RANGE: f32 = 12.0;
/// In the walk, being hidden for less than this is the fade catching up, not a failure.
const HIDDEN_GRACE: f64 = 0.3;
/// Seconds a surveyed pose is held before it is checked (the fade's full swing takes 0.13 s).
const SETTLE: f64 = 0.25;

/// The levels the check knows, by name.
fn stages() -> Vec<(String, Stage)> {
    let mut v: Vec<(String, Stage)> = Vec::new();
    for n in 0..3 {
        v.push((format!("dungeon{n}"), Stage::Dungeon(n)));
        v.push((format!("arena{n}"), Stage::Arena(n)));
    }
    v.push(("hub".into(), Stage::Hub));
    for (i, s) in SINS.iter().enumerate() {
        v.push((format!("realm{i}"), Stage::Realm(*s)));
        v.push((format!("court{i}"), Stage::Court(*s)));
    }
    v.push(("gym".into(), Stage::Gym));
    v
}

/// Solid static cubes, bucketed by the 1 m columns they cover, for quick ray and point tests.
struct Solids {
    inv: Vec<Mat4>,
    m: Vec<Mat4>,
    cols: HashMap<(i32, i32), Vec<u32>>,
    /// Each cube's index into the view's static cubes, to see whether it has faded since.
    src: Vec<u32>,
    /// Each column's lowest and highest extent.
    top: HashMap<(i32, i32), (f32, f32)>,
}

fn solid_kind(i: &Instance) -> bool {
    let shade = i.emissive[3];
    // Grass, cloud, water, jets and glows never hide anything.
    !(shade == 4.0 || shade == 5.0 || shade >= 20.0) && i.color[3] > 0.0
}

impl Solids {
    fn new(cubes: &[Instance]) -> Self {
        Self::of(cubes, solid_kind)
    }

    /// Everything drawn, see-through or not (what a view ray can meet).
    fn everything(cubes: &[Instance]) -> Self {
        Self::of(cubes, |c| c.color[3] > 0.0)
    }

    fn of(cubes: &[Instance], keep: impl Fn(&Instance) -> bool) -> Self {
        let mut s = Solids { inv: Vec::new(), m: Vec::new(), cols: HashMap::new(), src: Vec::new(), top: HashMap::new() };
        for (k, c) in cubes.iter().enumerate() {
            if !keep(c) {
                continue;
            }
            let m = Mat4::from_cols_array(&c.m);
            // Its footprint: the eight corners' bounds.
            let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for corner in 0..8 {
                let l = Vec3::new(
                    if corner & 1 == 0 { -0.5 } else { 0.5 },
                    if corner & 2 == 0 { -0.5 } else { 0.5 },
                    if corner & 4 == 0 { -0.5 } else { 0.5 },
                );
                let w = m.transform_point3(l);
                lo = lo.min(w);
                hi = hi.max(w);
            }
            // Specks (petals, pebbles) neither block a view nor cut the near plane visibly.
            if (hi - lo).max_element() < 0.12 {
                continue;
            }
            let id = s.m.len() as u32;
            s.m.push(m);
            s.inv.push(m.inverse());
            s.src.push(k as u32);
            for x in lo.x.floor() as i32..=hi.x.floor() as i32 {
                for z in lo.z.floor() as i32..=hi.z.floor() as i32 {
                    s.cols.entry((x, z)).or_default().push(id);
                    let t = s.top.entry((x, z)).or_insert((f32::MAX, f32::MIN));
                    *t = (t.0.min(lo.y), t.1.max(hi.y));
                }
            }
        }
        s
    }

    /// Does the ray from `eye` along `dir` meet anything within `reach`? Walked column by column.
    fn ray_hits(&self, eye: Vec3, dir: Vec3, reach: f32) -> bool {
        let end = eye + dir * reach;
        let n = (reach * 2.0).ceil() as i32;
        let mut seen = std::collections::HashSet::new();
        let mut last = (i32::MIN, i32::MIN);
        for i in 0..=n {
            let p = eye.lerp(end, i as f32 / n as f32);
            let col = (p.x.floor() as i32, p.z.floor() as i32);
            if col == last {
                continue;
            }
            last = col;
            for dx in -1..=1 {
                for dz in -1..=1 {
                    let c = (col.0 + dx, col.1 + dz);
                    let Some(&(bot, top)) = self.top.get(&c) else { continue };
                    // The ray's height over this stretch.
                    let (y0, y1) = (eye.y.min(end.y), eye.y.max(end.y));
                    if top < y0 || bot > y1 {
                        continue;
                    }
                    for &id in self.cols.get(&c).into_iter().flatten() {
                        if !seen.insert(id) {
                            continue;
                        }
                        let inv = &self.inv[id as usize];
                        if slab(inv.transform_point3(eye), inv.transform_point3(end)) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Is cube `id` solid right now (not faded out of the way)?
    fn standing(&self, id: u32, cubes: &[Instance]) -> bool {
        cubes.get(self.src[id as usize] as usize).is_some_and(|c| c.color[3] >= 0.999)
    }

    /// Does any cube of these, at any height, lie within `r` of `p` across the ground (a floor
    /// below or above the plane `p` is on; a seam between floor slabs does not count as open)?
    fn covers_ground(&self, p: Vec3, r: f32) -> bool {
        for x in (p.x - r).floor() as i32..=(p.x + r).floor() as i32 {
            for z in (p.z - r).floor() as i32..=(p.z + r).floor() as i32 {
                for &id in self.cols.get(&(x, z)).into_iter().flatten() {
                    let m = &self.m[id as usize];
                    let q = Vec3::new(p.x, m.w_axis.y, p.z);
                    let l = self.inv[id as usize].transform_point3(q).clamp(Vec3::splat(-0.5), Vec3::splat(0.5));
                    let c = m.transform_point3(l);
                    if Vec2::new(c.x - q.x, c.z - q.z).length() < r {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Does any cube of these (faded or not) come within `r` of `p`?
    fn near_any(&self, p: Vec3, r: f32) -> bool {
        for x in (p.x - r).floor() as i32..=(p.x + r).floor() as i32 {
            for z in (p.z - r).floor() as i32..=(p.z + r).floor() as i32 {
                for &id in self.cols.get(&(x, z)).into_iter().flatten() {
                    let l = self.inv[id as usize].transform_point3(p).clamp(Vec3::splat(-0.5), Vec3::splat(0.5));
                    if self.m[id as usize].transform_point3(l).distance(p) < r {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// The nearest a solid cube comes to `p`, if within `r`.
    fn near(&self, p: Vec3, r: f32, cubes: &[Instance]) -> bool {
        let mut seen = std::collections::HashSet::new();
        for x in (p.x - r).floor() as i32..=(p.x + r).floor() as i32 {
            for z in (p.z - r).floor() as i32..=(p.z + r).floor() as i32 {
                for &id in self.cols.get(&(x, z)).into_iter().flatten() {
                    if !seen.insert(id) || !self.standing(id, cubes) {
                        continue;
                    }
                    let l = self.inv[id as usize].transform_point3(p).clamp(Vec3::splat(-0.5), Vec3::splat(0.5));
                    if self.m[id as usize].transform_point3(l).distance(p) < r {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Does a solid cube cut the segment from `a` to `b` (stopping a little short of `b`)?
    fn blocks(&self, a: Vec3, b: Vec3, cubes: &[Instance]) -> bool {
        let b = a.lerp(b, 0.95);
        let mut seen = std::collections::HashSet::new();
        let (lo, hi) = (a.min(b), a.max(b));
        for x in lo.x.floor() as i32..=hi.x.floor() as i32 {
            for z in lo.z.floor() as i32..=hi.z.floor() as i32 {
                for &id in self.cols.get(&(x, z)).into_iter().flatten() {
                    if !seen.insert(id) || !self.standing(id, cubes) {
                        continue;
                    }
                    let inv = &self.inv[id as usize];
                    if slab(inv.transform_point3(a), inv.transform_point3(b)) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// Does the segment from `a` to `b` (in the cube's own frame) pass through the unit cube?
fn slab(a: Vec3, b: Vec3) -> bool {
    let d = b - a;
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for k in 0..3 {
        if d[k].abs() < 1e-7 {
            if a[k].abs() > 0.5 {
                return false;
            }
            continue;
        }
        let (mut ta, mut tb) = ((-0.5 - a[k]) / d[k], (0.5 - a[k]) / d[k]);
        if ta > tb {
            std::mem::swap(&mut ta, &mut tb);
        }
        t0 = t0.max(ta);
        t1 = t1.min(tb);
        if t0 > t1 {
            return false;
        }
    }
    true
}

/// The near plane's corners, to test against the scenery (the eye alone misses a wall it is
/// pressed against).
fn near_points(view: &WorldView) -> [Vec3; 5] {
    let (eye, fwd, right, up, _) = view.camera_basis();
    let near = 0.1;
    let h = near * (crate::render::view::FOV_DEGREES.to_radians() * 0.5).tan();
    let w = h * view.aspect;
    let c = eye + fwd * near;
    [eye, c + right * w + up * h, c - right * w + up * h, c + right * w - up * h, c - right * w - up * h]
}

#[derive(Default, Clone)]
struct Tally {
    poses: u32,
    clip: u32,
    hidden: u32,
    offscreen: u32,
    out_of_bounds: u32,
    backdrop: u32,
    /// The most blur on a foe in play, in pixels at 1080p, and how many frames went over the limit.
    dof_worst: f32,
    dof_over: u32,
    /// The coarsest level he was drawn at.
    hero_lod: u8,
    /// In the walk: the longest he stayed hidden, in seconds, and the switches made.
    longest_hidden: f64,
    switches: u32,
    /// The first few failures, to look at again.
    examples: Vec<String>,
}

impl Tally {
    fn fails(&self) -> bool {
        self.clip > 0 || self.hidden > 0 || self.offscreen > 0 || self.out_of_bounds > 0 || self.dof_over > 0
    }

    fn note(&mut self, what: String) {
        if self.examples.len() < 6 {
            self.examples.push(what);
        }
    }
}

/// What one frame's camera shows, checked.
struct Look {
    clip: bool,
    hidden: bool,
    offscreen: bool,
    out_of_bounds: bool,
    /// Rays past the level's edge into its backdrop (not a failure; for the vista review).
    backdrop: bool,
    dof: f32,
    hero_lod: u8,
}

fn hero_feet(game: &Game) -> Vec3 {
    let p = game.hero.pos();
    Vec3::new(p.x as f32, game.hero.altitude as f32 * RISE, p.y as f32)
}

/// The view rays below the horizon (a grid across the lower screen) that meet nothing: whether
/// one of them went through the level's floor plan (a hole), and whether one ran past its edge.
fn empty_rays(view: &WorldView, grid: &crate::logic::grid::Grid, seen: &Solids) -> (bool, bool) {
    let inv = view.view_proj().inverse();
    let eye = view.camera_position();
    let (w, h) = view.level_size();
    let (mut hole, mut past) = (false, false);
    for j in 0..4 {
        for i in 0..7 {
            let (x, y) = (-0.9 + 1.8 * i as f32 / 6.0, -0.9 + 1.0 * j as f32 / 3.0);
            let far = inv.project_point3(Vec3::new(x, y, 1.0));
            let near = inv.project_point3(Vec3::new(x, y, 0.0));
            let dir = (far - near).normalize_or_zero();
            if dir.y > -0.03 {
                continue; // at or above the horizon: sky
            }
            let reach = crate::render::view::FAR.min((eye.y + 6.0) / -dir.y);
            if !seen.ray_hits(eye, dir, reach) {
                // Where it passes the floor's height.
                let g = eye + dir * (eye.y / -dir.y).min(reach);
                // (Inside the outer wall's ring: past it is the backdrop.)
                // Only ground he walks on counts: a pit, a well or a chasm (a tile he cannot walk)
                // is a hole by design, and a seam between floor slabs is not one.
                let pit = matches!(grid.get(g.x.round() as i32, g.z.round() as i32), Tile::Pit | Tile::Rock) || seen.covers_ground(g, 0.15);
                if g.x > 0.5 && g.z > 0.5 && g.x < w - 1.5 && g.z < h - 1.5 && !pit {
                    hole = true;
                } else {
                    past = true;
                }
            }
        }
    }
    (hole, past)
}

fn look(game: &Game, solids: &Solids, seen: &Solids, edges: bool) -> Look {
    let view = &game.view;
    let cubes = &view.static_cubes;
    let eye = view.camera_position();
    let clip = near_points(view).iter().any(|&p| solids.near(p, 0.02, cubes)) || solids.near(eye, NEAR_REACH * 0.5, cubes);
    let feet = hero_feet(game);
    let marks = [feet + Vec3::Y * 0.25, feet + Vec3::Y * 0.6, feet + Vec3::Y * 1.0];
    let hidden = marks.iter().all(|&m| solids.blocks(eye, m, cubes));
    let vp = view.view_proj();
    let ndc = |p: Vec3| {
        let c = vp * Vec4::new(p.x, p.y, p.z, 1.0);
        (c.w > 1e-4).then(|| (c.x / c.w, c.y / c.w))
    };
    let offscreen = ndc(marks[1]).is_none_or(|(x, y)| x.abs() > 0.9 || y.abs() > 0.9);
    let (out_of_bounds, backdrop) = if edges { empty_rays(view, &game.level.grid, seen) } else { (false, false) };
    // The composite's depth of field (`blur_amount` in post.wgsl).
    let mut dof = 0.0f32;
    if let Some(focus) = view.focus_depth() {
        let (_, fwd, ..) = view.camera_basis();
        for e in game.enemies.iter().filter(|e| e.is_dangerous()) {
            let p = e.pos();
            let at = Vec3::new(p.x as f32, feet.y + 0.5, p.y as f32);
            if at.distance(feet) > THREAT_RANGE {
                continue;
            }
            let d = (at - eye).dot(fwd);
            let x = ((d - focus - DOF_CLEAR) / (DOF_FULL - DOF_CLEAR)).clamp(0.0, 1.0);
            dof = dof.max(x * x * (3.0 - 2.0 * x) * DOF_PX);
        }
    }
    // What he would be drawn at by size alone; he is held whole whatever this says (emit), so it
    // is reported, not failed.
    let lod = view.lod_policy();
    let hero_lod = lod.level(lod.pixels(view.hero_voxel(), marks[1]), None);
    Look { clip, hidden, offscreen, out_of_bounds, backdrop, dof, hero_lod }
}

/// Plays `secs` of the game, keeping him alive.
fn play(game: &mut Game, secs: f64) {
    let mut t = 0.0;
    while t < secs {
        game.hero.invuln = game.hero.invuln.max(1.0);
        game.update(STEP);
        t += STEP;
    }
}

/// Into a level, playing, the camera chosen.
fn enter(game: &mut Game, stage: Stage, mode: CameraMode) -> bool {
    game.map_jump(stage);
    play(game, 4.0);
    if game.screen != Screen::Playing {
        // The hub's first visit runs its tour; any other screen is pressed through.
        game.tour = None;
        for _ in 0..4 {
            game.input.inject(Action::Confirm);
            play(game, 1.0);
            if game.screen == Screen::Playing {
                break;
            }
        }
    }
    game.view.camera_mode = mode;
    game.view.snap_camera();
    game.screen == Screen::Playing
}

fn record(t: &mut Tally, l: &Look, what: impl Fn() -> String) {
    t.poses += 1;
    t.backdrop += u32::from(l.backdrop);
    t.hero_lod = t.hero_lod.max(l.hero_lod);
    t.dof_worst = t.dof_worst.max(l.dof);
    if l.dof > DOF_LIMIT {
        t.dof_over += 1;
    }
    for (bad, name, n) in
        [(l.clip, "clips", &mut t.clip), (l.offscreen, "off screen", &mut t.offscreen), (l.out_of_bounds, "hole", &mut t.out_of_bounds)]
    {
        if bad {
            *n += 1;
        }
        let _ = name;
    }
    if l.clip || l.offscreen || l.out_of_bounds || l.hidden {
        let mut tags = Vec::new();
        if l.clip {
            tags.push("clips");
        }
        if l.hidden {
            tags.push("hides him");
        }
        if l.offscreen {
            tags.push("off screen");
        }
        if l.out_of_bounds {
            tags.push("shows a hole into the void");
        }
        t.note(format!("{}: {}", tags.join(", "), what()));
    }
}

fn survey(game: &mut Game, solids: &Solids, seen: &Solids, mode: CameraMode, stride: i32) -> Tally {
    let mut t = Tally::default();
    // Every prop he cannot stand in: opaque or see-through, not water, glow, cloud or grass.
    let props = Solids::of(&game.view.static_cubes, |c| solid_kind(c) && c.emissive[3] != crate::render::view::SHADE_GRASS);
    // The world holds still: no foes, no clock. The view keeps updating.
    game.input.inject(Action::Pause);
    game.update(STEP);
    let paused = game.screen == Screen::Paused;
    let grid = game.level.grid.clone();
    let pitches = [crate::render::view::REST_PITCH, -1.15, 0.05];
    for z in (0..grid.height).step_by(stride as usize) {
        for x in (0..grid.width).step_by(stride as usize) {
            if matches!(grid.get(x, z), Tile::Rock | Tile::Pit) {
                continue;
            }
            // Only where he can stand: some levels set props on tiles their grid leaves open
            // (the Hall of Mirrors' frames), which he walks round, never into.
            let body = Vec3::new(x as f32, grid.elevation(x, z) as f32 * RISE, z as f32);
            if [0.3, 0.7, 1.1].iter().any(|&y| props.near_any(body + Vec3::Y * y, 0.12)) {
                continue;
            }
            game.hero.place(P::new(x, z));
            // (Set down, he stands at the tile's height, as walking there would leave him.)
            game.hero.altitude = grid.elevation(x, z) as f64;
            let poses: Vec<Option<(f32, f32)>> = if mode == CameraMode::Third {
                (0..8).flat_map(|k| pitches.iter().map(move |&p| Some((k as f32 * std::f32::consts::TAU / 8.0, p)))).collect()
            } else {
                vec![None]
            };
            for pose in poses {
                if let Some((yaw, pitch)) = pose {
                    game.view.cam_yaw = yaw;
                    game.view.cam_pitch = pitch;
                    game.view.mouse_look = true;
                }
                game.view.snap_camera();
                let mut s = 0.0;
                while s < SETTLE {
                    if !paused {
                        game.hero.invuln = game.hero.invuln.max(1.0);
                        game.hero.place(P::new(x, z));
                    }
                    game.update(STEP);
                    s += STEP;
                }
                let l = look(game, solids, seen, true);
                if l.hidden {
                    t.hidden += 1;
                }
                record(&mut t, &l, || match pose {
                    Some((yaw, pitch)) => format!("at:{x}:{z},aim:{:.0}:{:.2}", yaw.to_degrees(), pitch),
                    None => format!("at:{x}:{z}"),
                });
            }
        }
    }
    game.view.mouse_look = false;
    if paused {
        game.input.inject(Action::Pause);
        game.update(STEP);
    }
    t
}

fn walk(game: &mut Game, solids: &Solids, seen: &Solids, secs: f64, seed: u32) -> Tally {
    let mut t = Tally::default();
    let mut rng = Rng::new(seed);
    let dirs = [Dir::Up, Dir::Down, Dir::Left, Dir::Right];
    let mut dir = Dir::Up;
    let (mut turn_at, mut switch_at, mut sweep) = (0.0, 4.0, 0.0f64);
    let mut hidden_for = 0.0;
    let mut last = game.hero.pos();
    let mut still = 0.0;
    let mut clock = 0.0;
    while clock < secs {
        if game.screen != Screen::Playing {
            break;
        }
        if clock >= turn_at || still > 0.4 {
            game.input.hold_dir(dir, false);
            dir = dirs[(rng.next() * 4.0) as usize % 4];
            game.input.hold_dir(dir, true);
            turn_at = clock + 0.6 + rng.next() * 1.4;
            still = 0.0;
        }
        if clock >= switch_at {
            game.input.inject(Action::Camera);
            t.switches += 1;
            switch_at = clock + 3.0 + rng.next() * 2.0;
        }
        // Now and then the player swings the view round with the mouse.
        if sweep <= 0.0 && rng.next() < 0.004 {
            sweep = 1.0;
        }
        if sweep > 0.0 {
            game.view.look(9.0, if rng.next() < 0.5 { -2.0 } else { 2.0 });
            sweep -= STEP;
        }
        game.hero.invuln = game.hero.invuln.max(1.0);
        game.update(STEP);
        clock += STEP;
        let p = game.hero.pos();
        still = if (p.x - last.x).abs() + (p.y - last.y).abs() < 1e-6 { still + STEP } else { 0.0 };
        last = p;
        // (The edge of the world is looked for every tenth frame: it is the slow check.)
        let frame = (clock / STEP).round() as u64;
        let l = look(game, solids, seen, frame.is_multiple_of(10));
        hidden_for = if l.hidden { hidden_for + STEP } else { 0.0 };
        t.longest_hidden = t.longest_hidden.max(hidden_for);
        if hidden_for > HIDDEN_GRACE && hidden_for - STEP <= HIDDEN_GRACE {
            t.hidden += 1;
        }
        let mode = game.view.camera_mode;
        let (cx, cz) = (p.x.round() as i32, p.y.round() as i32);
        record(&mut t, &Look { hidden: hidden_for > HIDDEN_GRACE, ..l }, || format!("{mode:?} walk at {clock:.1} s, near at:{cx}:{cz}"));
    }
    game.input.hold_dir(dir, false);
    t
}

pub fn run(args: &[String]) -> i32 {
    std::env::set_var("VH_NO_AUDIO", "1");
    if std::env::var_os("VH_SAVE_DIR").is_none() {
        let dir = std::env::temp_dir().join("void-harvest-camcheck");
        let _ = std::fs::remove_dir_all(&dir);
        std::env::set_var("VH_SAVE_DIR", dir);
    }
    let which = arg(args, "--camera-check").filter(|s| !s.starts_with("--")).unwrap_or("all");
    let stride: i32 = arg(args, "--stride").and_then(|s| s.parse().ok()).unwrap_or(2).max(1);
    let walk_secs: f64 = arg(args, "--walk").and_then(|s| s.parse().ok()).unwrap_or(20.0);
    let out_dir = arg(args, "--out").unwrap_or("perf").to_string();
    let chosen: Vec<(String, Stage)> = stages().into_iter().filter(|(n, _)| which == "all" || which.split(',').any(|w| w == n)).collect();
    if chosen.is_empty() {
        eprintln!("no such level; the levels are: {}", stages().iter().map(|s| s.0.clone()).collect::<Vec<_>>().join(", "));
        return 2;
    }
    let mut md = String::from("# Camera validation (RP02)\n\n");
    let _ = writeln!(
        md,
        "Survey: every {stride} tile(s), chase camera at 8 headings × 3 pitches, top camera as is, each held {SETTLE} s. \
         Walk: {walk_secs} s wandering with foes about, mouse sweeps, the view switched every 3 to 5 s. At 1920×1080.\n"
    );
    let _ = writeln!(
        md,
        "| Level | Camera | Pass | Poses | Clips | Hides him | Off screen | Holes | Backdrop shown | Foe blur (worst px) | His LOD by size | Longest hidden |\n|---|---|---|---|---|---|---|---|---|---|---|---|"
    );
    let mut examples = String::new();
    let mut failed = false;
    for (name, stage) in &chosen {
        for mode in [CameraMode::Third, CameraMode::Top] {
            let mut game = Game::new();
            game.reseed(1);
            game.viewport = (1920.0, 1080.0);
            game.view.aspect = 16.0 / 9.0;
            game.view.viewport_h = 1080.0;
            game.tour_pending = false;
            if !enter(&mut game, *stage, mode) {
                eprintln!("camera-check: {name}: could not start playing (screen {:?})", game.screen);
                let _ = writeln!(md, "| {name} | {mode:?} | - | could not start playing | | | | | | | | |");
                failed = true;
                continue;
            }
            let solids = Solids::new(&game.view.static_cubes);
            // What a view ray can meet: the static cubes, and what the level draws each frame or
            // keeps on the GPU as sets (the meadow's turf, the hub's islands), from one frame.
            let mut drawn = game.view.static_cubes.clone();
            {
                let mut frame = Frame::default();
                game.view.emit(&game.scene(), &mut frame);
                for v in frame.opaque.iter().chain(frame.transparent.iter()) {
                    drawn.extend_from_slice(v);
                }
                drawn.extend_from_slice(&frame.transparent_early);
                for set in frame.sets.iter().chain(frame.sets_blend.iter()) {
                    drawn.extend_from_slice(&set.instances);
                }
            }
            let seen = Solids::everything(&drawn);
            let started = std::time::Instant::now();
            let s = survey(&mut game, &solids, &seen, mode, stride);
            // The walk starts in this camera; it switches as it goes.
            game.view.camera_mode = mode;
            game.view.snap_camera();
            let w = walk(&mut game, &solids, &seen, walk_secs, 7);
            for (pass, t) in [("survey", &s), ("walk", &w)] {
                let line = format!(
                    "| {name} | {mode:?} | {pass} | {} | {} | {} | {} | {} | {} | {:.2}{} | {} | {} |",
                    t.poses,
                    t.clip,
                    t.hidden,
                    t.offscreen,
                    t.out_of_bounds,
                    t.backdrop,
                    t.dof_worst,
                    if t.dof_over > 0 { format!(" ({} over)", t.dof_over) } else { String::new() },
                    t.hero_lod,
                    if pass == "walk" { format!("{:.2} s ({} switches)", t.longest_hidden, t.switches) } else { String::new() },
                );
                eprintln!("{line}");
                let _ = writeln!(md, "{line}");
                if t.fails() {
                    failed = true;
                }
                for e in &t.examples {
                    let _ = writeln!(examples, "- {name}, {mode:?} {pass}: {e}");
                }
            }
            eprintln!("camera-check: {name} {mode:?} took {:.1} s", started.elapsed().as_secs_f64());
        }
    }
    if !examples.is_empty() {
        let _ = writeln!(md, "\n## Examples\n\n{examples}");
    }
    let _ = std::fs::create_dir_all(&out_dir);
    let path = format!("{out_dir}/camera-{}.md", crate::perf::stamp());
    let _ = std::fs::write(&path, md);
    eprintln!("camera-check: wrote {path}");
    i32::from(failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_slab_test_finds_segments_through_the_unit_cube_only() {
        assert!(slab(Vec3::new(-2.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0)));
        assert!(!slab(Vec3::new(-2.0, 0.6, 0.0), Vec3::new(2.0, 0.6, 0.0)));
        assert!(!slab(Vec3::new(-2.0, 0.0, 0.0), Vec3::new(-0.6, 0.0, 0.0)));
        assert!(slab(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0)));
    }

    #[test]
    fn solids_see_a_wall_between_two_points_and_a_point_pressed_against_it() {
        let wall = Instance::new(
            Mat4::from_scale_rotation_translation(Vec3::new(0.2, 2.0, 4.0), glam::Quat::IDENTITY, Vec3::new(5.0, 1.0, 5.0)),
            Vec3::ONE,
            1.0,
            Vec3::ZERO,
            0.0,
        );
        let cubes = vec![wall];
        let s = Solids::new(&cubes);
        assert!(s.blocks(Vec3::new(3.0, 1.0, 5.0), Vec3::new(7.0, 1.0, 5.0), &cubes));
        assert!(!s.blocks(Vec3::new(3.0, 1.0, 5.0), Vec3::new(3.0, 1.0, 9.0), &cubes));
        assert!(s.near(Vec3::new(4.85, 1.0, 5.0), 0.1, &cubes));
        assert!(!s.near(Vec3::new(4.5, 1.0, 5.0), 0.1, &cubes));
        // A faded wall no longer blocks.
        let mut faded = cubes.clone();
        faded[0].color[3] = 0.5;
        assert!(!s.blocks(Vec3::new(3.0, 1.0, 5.0), Vec3::new(7.0, 1.0, 5.0), &faded));
    }
}
