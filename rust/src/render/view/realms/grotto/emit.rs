//! The grotto each frame: the water's two surfaces, the coils, the rafts and ribs, the pools and
//! geysers, the beads and the phantoms, the caustics, the Eye, the bell, and the light.

use super::super::super::vault_decor::push_merged;
use super::decor::pal as art_pal;
use super::*;

/// The serpent's scales, by turns.
fn scale(k: usize) -> Vec3 {
    lin([0x2a5a44, 0x1e4636, 0x356a50][k % 3])
}

/// The bronze of the bell as it heats: verdigris-dark bronze, dull red, orange, nearly white.
fn heat_color(heat: f32) -> Vec3 {
    let stops = [lin(0x8a5c2c), lin(0x9a2a10), lin(0xff6a1a), lin(0xfff0c0)];
    let t = heat.clamp(0.0, 1.0) * 3.0;
    let i = (t as usize).min(2);
    stops[i].lerp(stops[i + 1], t - i as f32)
}

/// A box along the direction `dir` in the ground plane.
fn along(f: &mut Vec<Instance>, at: Vec3, dir: Vec2, size: Vec3, color: Vec3, glow: Vec3, shade: f32) {
    let yaw = dir.y.atan2(dir.x);
    f.push(Instance::new(Mat4::from_scale_rotation_translation(size, Quat::from_rotation_y(-yaw), at), color, 1.0, glow, shade));
}

/// A point along a coil's spine, `u` tiles from its first tile (past either end, straight on).
fn spine_at(spine: &[P], u: f32) -> Vec2 {
    let n = spine.len();
    if n == 1 {
        return Vec2::new(spine[0].x as f32, spine[0].y as f32);
    }
    let i = (u.floor() as i32).clamp(0, n as i32 - 2) as usize;
    let a = Vec2::new(spine[i].x as f32, spine[i].y as f32);
    let b = Vec2::new(spine[i + 1].x as f32, spine[i + 1].y as f32);
    a + (b - a) * (u - i as f32)
}

impl WorldView {
    pub(super) fn emit_grotto_frame(&self, v: &GrottoView, r: &Grotto, s: &SceneState, time: f32, hero: Vec3, f: &mut Frame) {
        let drowned = layer_of(s.hero.occupied()) == Layer::Drowned;
        self.emit_water(drowned, time, f);
        if !drowned {
            self.emit_deep(r, s.grid, hero, time, f);
        }
        self.emit_coils(r, drowned, hero, time, f);
        self.emit_rafts(v, r, drowned, hero, time, f);
        self.emit_crossings(r, s, hero, time, f);
        self.emit_treasure(v, r, drowned, hero, time, f);
        self.emit_caustics(r, s.grid, drowned, hero, time, f);
        self.emit_eye(r, s, time, f);
        self.emit_bell(v, r, s, time, f);
        self.emit_lights(v, r, drowned, hero, time, f);
    }

    /// The Waterline's jade surface over its half of the map; seen from below, the same surface
    /// rippling overhead on the Drowned Floor.
    fn emit_water(&self, drowned: bool, _time: f32, f: &mut Frame) {
        let cube_i = MeshKind::Cube as usize;
        let (w, h) = (REGION_W as f32, HEIGHT as f32);
        let m = Mat4::from_translation(Vec3::new(w / 2.0 - 0.5, WATER_Y, h / 2.0 - 0.5)) * Mat4::from_scale(Vec3::new(w, 0.002, h));
        f.transparent[cube_i].push(Instance::new(m, lin(0x0e3a40) * 0.6, 0.1, Vec3::ZERO, SHADE_UNLIT));
        if drowned {
            let o = above_offset();
            let m = Mat4::from_translation(Vec3::new(w / 2.0 - 0.5 + o.x, WATER_Y + o.y, h / 2.0 - 0.5))
                * Mat4::from_scale(Vec3::new(w, 0.002, h));
            f.transparent[cube_i].push(Instance::new(m, lin(WATER_BELOW) * 0.5, 0.14, Vec3::new(-0.04, 0.03, 0.0), SHADE_WATER));
        }
    }

    /// The body of the deep water under the Waterline's surface: a cold teal over what lies below.
    fn emit_deep(&self, r: &Grotto, grid: &Grid, hero: Vec3, time: f32, f: &mut Frame) {
        let reach = RANGE as i32;
        let (hx, hz) = (hero.x.round() as i32, hero.z.round() as i32);
        for z in hz - reach..=hz + reach {
            for x in (hx - reach).max(0)..=(hx + reach).min(REGION_W - 1) {
                let p = P::new(x, z);
                if !grid.in_bounds(x, z) {
                    continue;
                }
                let bridge = grid.is_bridge(x, z);
                let bell_down = matches!(r.bell.phase, BellPhase::Down | BellPhase::Lifted);
                let deep = grid.get(x, z) == Tile::Pit || r.pool_at(p).is_some() || (r.bell.pool.contains(&p) && !bell_down) || bridge;
                if !deep {
                    continue;
                }
                if bridge {
                    let m =
                        Mat4::from_translation(Vec3::new(x as f32, WATER_Y - 0.5, z as f32)) * Mat4::from_scale(Vec3::new(1.0, 0.002, 1.0));
                    f.transparent[MeshKind::Cube as usize].push(Instance::new(m, lin(0x041a18), 0.7, Vec3::ZERO, SHADE_UNLIT));
                    continue;
                }
                let pool = r.pool_at(p).is_some() || r.bell.pool.contains(&p);
                let m =
                    Mat4::from_translation(Vec3::new(x as f32, WATER_Y - 0.45, z as f32)) * Mat4::from_scale(Vec3::new(1.0, 0.002, 1.0));
                let (c, a) = if pool { (lin(0x02140e), 0.5) } else { (lin(0x0a3a34), 0.14) };
                f.transparent[MeshKind::Cube as usize].push(Instance::new(m, c, a, Vec3::ZERO, SHADE_UNLIT));
                // The surface over the deep: jade, a ring spreading on it now and then.
                let m =
                    Mat4::from_translation(Vec3::new(x as f32, WATER_Y + 0.004, z as f32)) * Mat4::from_scale(Vec3::new(1.0, 0.002, 1.0));
                f.transparent[MeshKind::Cube as usize].push(Instance::new(m, lin(WATER) * 0.7, 0.18, Vec3::ZERO, SHADE_UNLIT));
                let seed = hash(x as f64 * 2.3, z as f64 * 3.1) as f32;
                let t = (time * 0.35 + seed * 7.0).fract();
                if seed > 0.55 {
                    let rr = 0.08 + t * 0.45;
                    let m = Mat4::from_translation(Vec3::new(
                        x as f32 + (seed - 0.75) * 0.6,
                        WATER_Y + 0.012,
                        z as f32 + (seed * 5.0).fract() * 0.5 - 0.25,
                    )) * Mat4::from_rotation_x(-PI / 2.0)
                        * Mat4::from_scale(Vec3::new(rr, rr, 1.0));
                    f.transparent[MeshKind::Annulus as usize].push(Instance::new(
                        m,
                        lin(0xbfffe0),
                        0.3 * (1.0 - t),
                        Vec3::ZERO,
                        SHADE_UNLIT,
                    ));
                }
            }
        }
    }

    /// Leviathan's coils: a hump arching out of the deep water (flat-backed where it bears him), a
    /// wall of coils heaped across a passage below; each drawn too where the other surface shows it.
    fn emit_coils(&self, r: &Grotto, drowned: bool, hero: Vec3, time: f32, f: &mut Frame) {
        for coil in &r.coils {
            let s = coil.start(r.time as f32 as f64) as f32;
            let len = coil.len as f32;
            let mean = coil.cross.iter().fold(Vec2::ZERO, |a, &(x, y)| a + Vec2::new(x as f32, y as f32)) / coil.cross.len() as f32;
            let thick = 0.55 + 0.6 * (coil.cross.len() as f32 - 1.0);
            let first = spine_at(&coil.spine, 0.0);
            let real = Vec3::new(first.x, 0.0, first.y);
            let seen = match (coil.kind, drowned) {
                (CoilKind::Hump, false) | (CoilKind::Wall, true) => Some(Vec3::ZERO),
                (CoilKind::Hump, true) => Some(above_offset()),
                (CoilKind::Wall, false) => Some(copy_offset()),
            };
            let Some(off) = seen else { continue };
            let mid = real + off + Vec3::new(coil.spine.len() as f32 / 2.0, 0.0, 0.0);
            if Vec2::new(mid.x - hero.x, mid.z - hero.z).length() > RANGE + coil.spine.len() as f32 {
                continue;
            }
            let dir = {
                let a = spine_at(&coil.spine, 0.0);
                let b = spine_at(&coil.spine, 1.0);
                (b - a).normalize_or_zero()
            };
            match coil.kind {
                CoilKind::Hump => {
                    let (lo, hi) = (s - 0.3, s + len - 1.0 + 0.3);
                    let mut u = s - 1.9;
                    let mut k = 0;
                    while u <= s + len - 1.0 + 1.9 {
                        let out = if u < lo {
                            lo - u
                        } else if u > hi {
                            u - hi
                        } else {
                            0.0
                        };
                        let top = WATER_Y + 0.02 - (out / 1.3).powi(2) * 0.6 + (time * 2.0 + u).sin() * 0.015;
                        if top > WATER_Y - 0.4 {
                            let p = spine_at(&coil.spine, u) + mean;
                            let at = Vec3::new(p.x, top - 0.5, p.y) + off;
                            if drowned {
                                // Seen from below: a dark shape passing over the surface.
                                let m = Mat4::from_scale_rotation_translation(
                                    Vec3::new(0.4, 0.4, thick),
                                    Quat::from_rotation_y(-dir.y.atan2(dir.x)),
                                    at + Vec3::new(0.0, 0.25, 0.0),
                                );
                                f.opaque[MeshKind::Cube as usize].push(Instance::new(m, scale(k) * 0.5, 0.55, Vec3::ZERO, SHADE_DITHER));
                            } else {
                                self.coil_segment(f, at, dir, thick, k, true);
                            }
                        }
                        u += 0.24;
                        k += 1;
                    }
                    // Foam rings where it breaks the surface, before and behind.
                    for end in [lo - 0.9, hi + 0.9].into_iter().filter(|_| !drowned) {
                        let p = spine_at(&coil.spine, end) + mean;
                        for j in 0..2 {
                            let rr = thick * 0.5 + 0.15 + j as f32 * 0.15 + (time * 3.0 + end + j as f32).sin() * 0.04;
                            let m = Mat4::from_translation(Vec3::new(p.x, WATER_Y + 0.012, p.y) + off)
                                * Mat4::from_rotation_x(-PI / 2.0)
                                * Mat4::from_scale(Vec3::new(rr, rr, 1.0));
                            f.transparent[MeshKind::Annulus as usize].push(Instance::new(
                                m,
                                lin(0xd8fff0),
                                0.35 - j as f32 * 0.12,
                                Vec3::ZERO,
                                SHADE_UNLIT,
                            ));
                        }
                    }
                }
                CoilKind::Wall => {
                    let mut u = s - 0.6;
                    let mut k = 0;
                    while u <= s + len - 1.0 + 0.6 {
                        let p = spine_at(&coil.spine, u) + mean;
                        for layer in 0..3 {
                            let wob = (time * 1.6 + u * 1.7 + layer as f32 * 2.1).sin() * 0.1;
                            let side = Vec2::new(-dir.y, dir.x) * wob;
                            let at = Vec3::new(p.x + side.x, 0.05 + layer as f32 * 0.44, p.y + side.y) + off;
                            self.coil_segment(f, at, dir, thick + 0.2, k + layer * 2, layer == 2);
                        }
                        u += 0.24;
                        k += 1;
                    }
                }
            }
        }
    }

    /// A length of the serpent's body along `dir` (its base at `at`): a broad back of wet scales,
    /// rounded over, its pale belly showing at the sides, every other one a verdigris fin.
    fn coil_segment(&self, f: &mut Frame, at: Vec3, dir: Vec2, girth: f32, k: usize, fin: bool) {
        self.coil_length(f, at, dir, girth, 0.36, k, fin);
    }

    /// A length `long` of the serpent's body (see `coil_segment`).
    #[allow(clippy::too_many_arguments)]
    fn coil_length(&self, f: &mut Frame, at: Vec3, dir: Vec2, girth: f32, long: f32, k: usize, fin: bool) {
        let cube_i = MeshKind::Cube as usize;
        let side = Vec3::new(-dir.y, 0.0, dir.x);
        let g = girth / 0.85;
        along(
            &mut f.opaque[cube_i],
            at + Vec3::new(0.0, 0.22 * g, 0.0),
            dir,
            Vec3::new(long, 0.42 * g, girth),
            scale(k),
            Vec3::ZERO,
            SHADE_LIT,
        );
        along(
            &mut f.opaque[cube_i],
            at + Vec3::new(0.0, 0.46 * g, 0.0),
            dir,
            Vec3::new(long * 0.94, 0.1 * g, girth * 0.7),
            scale(k + 1) * 1.25,
            Vec3::ZERO,
            SHADE_LIT,
        );
        along(
            &mut f.opaque[cube_i],
            at + Vec3::new(0.0, 0.06 * g, 0.0),
            dir,
            Vec3::new(long * 0.9, 0.1 * g, girth * 1.04),
            lin(0xa49c78),
            Vec3::ZERO,
            SHADE_BONE,
        );
        if k.is_multiple_of(3) {
            // A row of lighter scales across the back.
            along(
                &mut f.opaque[cube_i],
                at + Vec3::new(0.0, 0.43 * g, 0.0),
                dir,
                Vec3::new(0.08, 0.06, girth * 0.95),
                lin(0x2f6a52),
                Vec3::ZERO,
                SHADE_LIT,
            );
        }
        if fin && k.is_multiple_of(2) {
            along(
                &mut f.opaque[cube_i],
                at + Vec3::new(0.0, 0.6 * g, 0.0),
                dir,
                Vec3::new(0.26, 0.2 * g, 0.05),
                lin(VERDIGRIS),
                Vec3::ZERO,
                SHADE_BRONZE,
            );
            along(
                &mut f.opaque[cube_i],
                at + Vec3::new(0.0, 0.6 * g + 0.16, 0.0) - side * 0.0,
                dir,
                Vec3::new(0.12, 0.14, 0.04),
                lin(VERDIGRIS) * 1.2,
                Vec3::ZERO,
                SHADE_BRONZE,
            );
        }
    }

    /// The bone rafts bobbing on the Waterline; seen from below, their dark undersides, gold heaped
    /// on them (gold that is not there), and the rib-bridges overhead. What hangs over him is cut
    /// away round him.
    fn emit_rafts(&self, v: &GrottoView, r: &Grotto, drowned: bool, hero: Vec3, time: f32, f: &mut Frame) {
        let cube_i = MeshKind::Cube as usize;
        let off = if drowned { above_offset() } else { Vec3::ZERO };
        let clear = |at: Vec3| Vec2::new(at.x - hero.x, at.z - hero.z).length();
        for (k, p) in r.rafts.iter().enumerate() {
            let fk = k as f32 * 1.7;
            let at = Vec3::new(p.x as f32, WATER_Y - 0.06 + (time * 1.2 + fk).sin() * 0.03, p.y as f32) + off;
            let d = clear(at);
            if d > RANGE || (drowned && d < 3.2) {
                continue;
            }
            let m = Mat4::from_translation(at)
                * Mat4::from_rotation_y(fk + (time * 0.3 + fk).sin() * 0.1)
                * Mat4::from_rotation_z((time * 1.1 + fk).sin() * 0.04);
            push_merged(&mut f.opaque[cube_i], &v.art.raft, m, 0.0);
            if drowned {
                // From below: heaped with gold, glinting.
                push_merged(&mut f.opaque[cube_i], &v.art.gold, m * Mat4::from_translation(Vec3::new(0.0, 0.2, 0.0)), 0.25);
                let g = ((time * 1.9 + fk).sin() * 0.5 + 0.5).powi(6);
                glow_orb(f, at + Vec3::new(0.0, 0.5, 0.0), 0.5, lin(GOLD), 0.18 + 0.2 * g);
            }
        }
        if drowned {
            for &(at, along_x) in &v.ribs {
                let at = at + off + Vec3::new(0.0, -0.05, 0.0);
                let d = clear(at);
                if !(3.2..RANGE).contains(&d) {
                    continue;
                }
                let size = if along_x { Vec3::new(1.0, 0.12, 0.85) } else { Vec3::new(0.85, 0.12, 1.0) };
                let alpha = ((d - 3.2) / 3.0).clamp(0.35, 1.0);
                let m = Mat4::from_scale_rotation_translation(size, Quat::IDENTITY, at);
                let shade = if alpha < 1.0 { SHADE_DITHER } else { SHADE_BONE };
                f.opaque[cube_i].push(Instance::new(m, lin(BONE_D) * 0.6, alpha, Vec3::ZERO, shade));
            }
        }
    }

    /// The deep pools' swirl (only down), the vents' bubbles (only up), and a geyser's column as it
    /// carries him up, the Waterline welling where it will set him down.
    fn emit_crossings(&self, r: &Grotto, _s: &SceneState, hero: Vec3, time: f32, f: &mut Frame) {
        let near = |at: Vec3| Vec2::new(at.x - hero.x, at.z - hero.z).length() < RANGE;
        for (i, q) in r.pools.iter().enumerate() {
            let c = Vec3::new(q.at.x as f32, WATER_Y + 0.01, q.at.y as f32);
            if !near(c) {
                continue;
            }
            for k in 0..12 {
                let a = time * 1.4 + k as f32 * TAU / 12.0 + i as f32;
                let rr = 0.18 + 0.22 * ((k as f32 * 0.37 + time * 0.2).fract());
                let m = Mat4::from_translation(c + Vec3::new(a.cos() * rr, 0.0, a.sin() * rr))
                    * Mat4::from_rotation_y(-a)
                    * Mat4::from_scale(Vec3::new(0.12, 0.01, 0.03));
                f.additive.push(Instance::new(m, lin(ALGAE), 0.2, Vec3::ZERO, SHADE_UNLIT));
            }
            // Bubbles drawn down into the deep.
            for k in 0..6 {
                let t = (time * 0.5 + k as f32 / 6.0 + i as f32 * 0.3).fract();
                let a = k as f32 * 2.3 + t * 4.0;
                let at = c + Vec3::new(a.cos() * 0.2 * (1.0 - t), -t * 2.2, a.sin() * 0.2 * (1.0 - t) + t * REFRACTION as f32);
                f.additive.push(Instance::new(
                    Mat4::from_translation(at) * Mat4::from_scale(Vec3::splat(0.05)),
                    lin(0xd8fff0),
                    0.4 * (1.0 - t),
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
        }
        for (i, vent) in r.vents.iter().enumerate() {
            let riding = r.ride.filter(|rd| rd.0 == i).map(|rd| rd.1 as f32);
            let c = Vec3::new(vent.at.x as f32, 0.0, vent.at.y as f32);
            if near(c) {
                let n = if riding.is_some() { 16 } else { 6 };
                for k in 0..n {
                    let t = (time * if riding.is_some() { 1.4 } else { 0.4 } + k as f32 / n as f32).fract();
                    let a = k as f32 * 2.7;
                    let at = c + Vec3::new(a.cos() * 0.15, t * DEPTH, a.sin() * 0.15);
                    f.additive.push(Instance::new(
                        Mat4::from_translation(at) * Mat4::from_scale(Vec3::splat(0.04 + t * 0.05)),
                        lin(0xe8fff4),
                        0.45 * (1.0 - t),
                        Vec3::ZERO,
                        SHADE_UNLIT,
                    ));
                }
                if let Some(t) = riding {
                    let rise = (t / RIDE_SECONDS as f32).min(1.0);
                    let top = DEPTH * rise + 0.3;
                    let m = Mat4::from_translation(c + Vec3::new(0.0, top / 2.0, 0.0)) * Mat4::from_scale(Vec3::new(0.5, top, 0.5));
                    f.transparent[MeshKind::Cylinder as usize].push(Instance::new(
                        m,
                        lin(0xd8fff0),
                        0.6,
                        Vec3::new(i as f32, 0.0, 0.0),
                        SHADE_JET,
                    ));
                    glow_orb(f, c + Vec3::new(0.0, top, 0.0), 0.7, lin(0xe8fff4), 0.25);
                }
            }
            if let Some(t) = riding {
                // Where it will break the surface above.
                let up = Vec3::new(vent.to.x as f32, WATER_Y + 0.02, vent.to.y as f32);
                for k in 0..10 {
                    let a = k as f32 / 10.0 * TAU + time * 2.0;
                    let rr = 0.3 + 0.1 * (time * 6.0 + k as f32).sin();
                    let m = Mat4::from_translation(up + Vec3::new(a.cos() * rr, 0.0, a.sin() * rr))
                        * Mat4::from_scale(Vec3::new(0.12, 0.02, 0.12));
                    f.additive.push(Instance::new(m, lin(0xe8fff4), 0.25 * (t / RIDE_SECONDS as f32), Vec3::ZERO, SHADE_UNLIT));
                }
            }
        }
    }

    /// Nazar beads (real, on their own surface, and those below seen through the water casting a
    /// shadow in the caustics) and the phantoms: chests and beads under the water at the wrong depth.
    fn emit_treasure(&self, v: &GrottoView, r: &Grotto, drowned: bool, hero: Vec3, time: f32, f: &mut Frame) {
        let near = |at: Vec3| Vec2::new(at.x - hero.x, at.z - hero.z).length() < RANGE;
        for (k, b) in r.beads.iter().enumerate() {
            if b.taken {
                continue;
            }
            let bob = (time * 2.0 + k as f32).sin() * 0.05;
            let at = Vec3::new(b.at.x as f32, 0.42 + bob, b.at.y as f32);
            if near(at) {
                self.nazar(f, at, 1.0, time + k as f32);
            }
            if layer_of(b.at) == Layer::Drowned && !drowned {
                let shown = at + copy_offset();
                if near(shown) {
                    self.nazar(f, shown, 1.0, time + k as f32);
                    // A real thing casts its shadow in the caustics.
                    let m = Mat4::from_translation(Vec3::new(shown.x, -DEPTH + 0.02, shown.z))
                        * Mat4::from_rotation_x(-PI / 2.0)
                        * Mat4::from_scale(Vec3::new(0.3, 0.3, 1.0));
                    f.transparent[MeshKind::Disc as usize].push(Instance::new(m, Vec3::ZERO, 0.55, Vec3::ZERO, SHADE_UNLIT));
                }
            }
        }
        if drowned {
            return;
        }
        // The phantoms: on the line from where the camera stood a moment ago to where they seem to
        // lie, drawn just under the surface (and as much smaller as that is nearer the camera).
        let a = v.anchor;
        let cube_i = MeshKind::Cube as usize;
        for (k, ph) in r.phantoms.iter().enumerate() {
            if ph.kind == PhantomKind::Gold {
                continue;
            }
            let floor = Vec3::new(ph.at.x as f32, 0.0, ph.at.y as f32) + copy_offset();
            if !near(floor) {
                continue;
            }
            let y = WATER_Y - 0.3;
            let t = ((y - a.y) / (floor.y - a.y)).clamp(0.05, 1.0);
            let at = a + (floor - a) * t;
            let fk = k as f32 * 2.3;
            match ph.kind {
                PhantomKind::Chest => {
                    let m = Mat4::from_translation(at) * Mat4::from_rotation_y(fk) * Mat4::from_scale(Vec3::splat(t));
                    push_merged(&mut f.opaque[cube_i], &v.art.chest, m, 0.35);
                    let g = ((time * 1.6 + fk).sin() * 0.5 + 0.5).powi(4);
                    glow_orb(f, at + Vec3::new(0.0, 0.4 * t, 0.0), 0.7 * t, lin(GOLD), 0.22 + 0.25 * g);
                    for s in 0..3 {
                        let an = time * 1.2 + s as f32 * TAU / 3.0 + fk;
                        let sp = at + Vec3::new(an.cos() * 0.35, 0.45 + (an * 1.3).sin() * 0.12, an.sin() * 0.35) * t;
                        f.additive.push(Instance::new(
                            Mat4::from_translation(sp) * Mat4::from_scale(Vec3::splat(0.035 * t)),
                            lin(0xfff0a0),
                            0.8,
                            Vec3::ZERO,
                            SHADE_UNLIT,
                        ));
                    }
                }
                PhantomKind::Bead => self.nazar(f, at + Vec3::new(0.0, 0.4 * t, 0.0), t, time + fk),
                PhantomKind::Gold => {}
            }
        }
    }

    /// A nazar bead: rings of dark blue, white and pale blue round a black eye, turned to the camera.
    fn nazar(&self, f: &mut Frame, at: Vec3, s: f32, time: f32) {
        let disc = MeshKind::Disc as usize;
        let toward = f.facing * Vec3::Z;
        for (k, (r, c)) in [(0.17f32, NAZAR), (0.12, 0xf4f4ff), (0.085, NAZAR_L), (0.04, 0x050505)].iter().enumerate() {
            let m = Mat4::from_scale_rotation_translation(Vec3::new(r * s, r * s, 1.0), f.facing, at + toward * (k as f32 * 0.006));
            let col = lin(*c);
            f.opaque[disc].push(Instance::new(m, col, 1.0, col * 0.35, SHADE_LIT));
        }
        glow_orb(f, at, 0.35 * s, lin(NAZAR_L), 0.18 + 0.05 * (time * 3.0).sin());
    }

    /// Light crawling over the floor below and up the walls: on the Drowned Floor where he walks it,
    /// or as it shows beneath the Waterline's deep water when he is above.
    fn emit_caustics(&self, r: &Grotto, grid: &Grid, drowned: bool, hero: Vec3, time: f32, f: &mut Frame) {
        let reach = 11;
        let (hx, hz) = (hero.x.round() as i32, hero.z.round() as i32);
        let bar = |f: &mut Frame, at: Vec3, seed: f32, strength: f32| {
            for k in 0..2 {
                let ph = seed * 7.3 + k as f32 * 3.1;
                let off = Vec3::new((time * 0.55 + ph).sin() * 0.35, 0.0, (time * 0.47 + ph * 1.7).cos() * 0.35);
                let a = ph + time * 0.25;
                let alpha = (0.07 + 0.06 * (time * 1.3 + ph * 2.0).sin()) * strength;
                let m = Mat4::from_translation(at + off) * Mat4::from_rotation_y(a) * Mat4::from_scale(Vec3::new(0.55, 0.004, 0.035));
                f.additive.push(Instance::new(m, lin(0xd8ffb0), alpha.max(0.0), Vec3::ZERO, SHADE_UNLIT));
            }
        };
        for z in hz - reach..=hz + reach {
            for x in hx - reach..=hx + reach {
                if !grid.in_bounds(x, z) {
                    continue;
                }
                let seed = hash(x as f64 * 1.9, z as f64 * 2.7) as f32;
                if drowned {
                    if grid.get(x, z) == Tile::Rock {
                        continue;
                    }
                    bar(f, Vec3::new(x as f32, 0.012, z as f32), seed, 1.0);
                } else {
                    let p = P::new(x, z);
                    let deep = grid.get(x, z) == Tile::Pit || r.pool_at(p).is_some() || r.bell.pool.contains(&p);
                    if deep && layer_of(p) == Layer::Waterline {
                        let below_tile = P::new(x + OX, (z as f32 - REFRACTION as f32).round() as i32);
                        if grid.get(below_tile.x, below_tile.y) != Tile::Rock {
                            bar(f, Vec3::new(x as f32, -DEPTH + 0.012, z as f32), seed, 1.3);
                        }
                    }
                    // And up the Waterline's walls, from the water.
                    if grid.get(x, z) == Tile::Rock && layer_of(p) == Layer::Waterline {
                        for d in CARDINALS {
                            let n = p.step(d);
                            if grid.get(n.x, n.y) == Tile::Rock || seed < 0.4 {
                                continue;
                            }
                            let (dx, dz) = d.vec();
                            let up = 0.15 + ((time * 0.3 + seed * 5.0).sin() * 0.5 + 0.5) * 0.5;
                            let at = Vec3::new(x as f32 + dx as f32 * 0.515, up, z as f32 + dz as f32 * 0.515);
                            let size = if dx == 0 { Vec3::new(0.5, 0.05, 0.004) } else { Vec3::new(0.004, 0.05, 0.5) };
                            let alpha = 0.06 + 0.05 * (time * 1.7 + seed * 11.0).sin();
                            f.additive.push(Instance::new(
                                Mat4::from_scale_rotation_translation(size, Quat::IDENTITY, at),
                                lin(0xc8ffd0),
                                alpha.max(0.0),
                                Vec3::ZERO,
                                SHADE_UNLIT,
                            ));
                        }
                    }
                }
            }
        }
    }

    /// The Eye, open in the water of his cavern: a great almond, its slit pupil turned the way its
    /// cone sweeps; the cone a green wash over the floor; a green light on him while it holds him.
    fn emit_eye(&self, r: &Grotto, s: &SceneState, time: f32, f: &mut Frame) {
        let Some((_, t)) = r.gaze() else { return };
        if matches!(r.bell.phase, BellPhase::Down | BellPhase::Lifted) {
            return;
        }
        let o = s.hero.occupied();
        let Some((eye, angle)) = r.eye_for(o) else { return };
        let drowned = layer_of(o) == Layer::Drowned;
        let y = if drowned { 1.5 } else { WATER_Y + 0.015 };
        let floor_y = if drowned { 0.02 } else { SHELF_Y + 0.02 };
        let c = Vec3::new(eye.x as f32, y, eye.y as f32);
        let t = t as f32;
        let open = (t / 0.08).min(1.0).min((1.0 - t) / 0.08).clamp(0.0, 1.0);
        let open = open * open * (3.0 - 2.0 * open);
        let cube_i = MeshKind::Cube as usize;
        let (a, b) = (1.5f32, 0.75f32);
        let look = Vec2::new((angle as f32).cos(), (angle as f32).sin());
        let iris = look * 0.28;
        let cell = 0.13;
        for cz in -7i32..=7 {
            for cx in -12i32..=12 {
                let (px, pz) = (cx as f32 * cell, cz as f32 * cell);
                let u = px / a;
                if u.abs() > 1.0 {
                    continue;
                }
                let lid = (1.0 - u * u).max(0.0).powf(0.65) * b;
                let inside = pz.abs() <= lid * open;
                let rim = !inside && pz.abs() <= lid + cell * 1.2;
                if !inside && !rim {
                    continue;
                }
                let at = c + Vec3::new(px, 0.0, pz);
                let (col, glow, shade) = if rim {
                    (scale((cx + cz).unsigned_abs() as usize), Vec3::ZERO, SHADE_WET)
                } else {
                    let d = Vec2::new(px - iris.x, pz - iris.y);
                    if d.x.abs() < 0.07 && d.y.abs() < 0.42 {
                        (lin(0x020402), Vec3::ZERO, SHADE_LIT)
                    } else if d.length() < 0.5 {
                        let streak = ((d.y.atan2(d.x) * 9.0).sin() * 0.5 + 0.5) * 0.4 + 0.6;
                        let col = lin(ALGAE).lerp(lin(GOLD), (d.length() / 0.5).powi(2)) * streak;
                        (col * 0.3, col * 0.25, SHADE_LIGHT)
                    } else {
                        let col = lin(EYE) * (0.55 - pz.abs() * 0.3);
                        (col * 0.22, col * 0.12, SHADE_LIGHT)
                    }
                };
                f.opaque[cube_i].push(Instance::new(
                    Mat4::from_scale_rotation_translation(Vec3::new(cell, 0.03, cell), Quat::IDENTITY, at),
                    col,
                    1.0,
                    glow,
                    shade,
                ));
            }
        }
        glow_orb(f, c + Vec3::new(0.0, 0.2, 0.0), 1.8, lin(EYE), 0.12 * open);
        // The cone: a wash of green over the floor, brighter along its edges.
        let half = EYE_HALF as f32;
        let mut d = 1.0;
        while d < EYE_RANGE as f32 {
            for k in -3i32..=3 {
                let an = angle as f32 + half * k as f32 / 3.0;
                let at = Vec3::new(c.x + an.cos() * d, floor_y, c.z + an.sin() * d);
                let edge = k.abs() == 3;
                let w = (d * half * 2.0 / 6.0).max(0.25) * if edge { 0.25 } else { 1.0 };
                let alpha = (if edge { 0.16 } else { 0.05 }) * (1.0 - d / EYE_RANGE as f32) * open;
                let m = Mat4::from_translation(at) * Mat4::from_rotation_y(-an) * Mat4::from_scale(Vec3::new(0.85, 0.004, w));
                f.additive.push(Instance::new(m, lin(0x8aff5a), alpha, Vec3::ZERO, SHADE_UNLIT));
            }
            d += 0.8;
        }
        f.lights.push(Light { pos: c + Vec3::new(0.0, 0.8, 0.0), color: lin(EYE) * 3.0 * open, distance: 7.0 });
        if r.seen {
            let hp = s.hero.pos();
            let at = Vec3::new(hp.x as f32, s.hero.altitude as f32 * STEP + 0.6, hp.y as f32);
            glow_orb(f, at, 0.9, lin(0x8aff5a), 0.2 + 0.08 * (time * 8.0).sin());
        }
    }

    /// Kiyohime's bell: hanging over the black pool (faded when he is near), or down over him, the
    /// serpent coiled round it, the bronze heating, the scorch and the steam; lifted on its seam.
    fn emit_bell(&self, v: &GrottoView, r: &Grotto, s: &SceneState, time: f32, f: &mut Frame) {
        let bell = &r.bell;
        let c = Vec3::new(bell.center.x as f32, 0.0, bell.center.y as f32);
        let hp = s.hero.pos();
        if Vec2::new(c.x - hp.x as f32, c.z - hp.y as f32).length() > RANGE + 6.0 || layer_of(s.hero.occupied()) == Layer::Drowned {
            return;
        }
        match bell.phase {
            BellPhase::Hanging | BellPhase::Won => {
                self.bell_shape(v, f, c + Vec3::new(0.0, 2.4, 0.0), 4.1, 4.5, lin(0x8a5c2c), Vec3::ZERO, false, v.bell_fade, None);
                // Through the black pool: the court's portal glowing below.
                let shown = Vec3::new(r.base.exit.tile.x as f32, 1.3, r.base.exit.tile.y as f32) + copy_offset();
                glow_orb(f, shown, 2.2, lin(ALGAE), 0.3 + 0.06 * (time * 1.5).sin());
            }
            BellPhase::Down | BellPhase::Lifted => {
                let heat = bell.heat() as f32;
                let col = heat_color(heat);
                let glow = col * heat * heat * 1.4;
                let lift = if bell.phase == BellPhase::Lifted {
                    let k = (bell.clock as f32 / LIFT_SECONDS as f32).clamp(0.0, 1.0);
                    1.7 * (k * (2.0 - k))
                } else {
                    0.0
                };
                let radius = bell.disc as f32 + 0.62;
                self.bell_shape(v, f, c + Vec3::new(0.0, -0.3 + lift, 0.0), radius, 2.6, col, glow, true, 1.0, Some(bell.head as f32));
                self.serpent_round_bell(v, f, c, radius, 2.3 + lift, bell.head as f32, lift, time);
                self.bell_floor(f, r);
                // The white-hot patch where the head presses.
                let patch = bell.patch();
                let pa = Vec3::new(patch.x as f32, 0.7 + lift, patch.y as f32);
                glow_orb(f, pa, 0.9, lin(0xfff8e8), 0.45 + 0.15 * (time * 9.0).sin());
                f.lights.push(Light { pos: pa, color: lin(0xfff0d0) * 4.0, distance: 4.0 });
                f.lights.push(Light { pos: c + Vec3::new(0.0, 2.0, 0.0), color: col * (1.0 + heat * 5.0), distance: 8.0 });
                // The scorch, outside the safe circle.
                let safe = bell.safe() as f32;
                let n = (bell.disc.ceil()) as i32;
                let steam = bell.steam();
                for dz in -n..=n {
                    for dx in -n..=n {
                        let d = ((dx * dx + dz * dz) as f32).sqrt();
                        if d > bell.disc as f32 {
                            continue;
                        }
                        let at = c + Vec3::new(dx as f32, SHELF_Y + 0.02, dz as f32);
                        if d > safe {
                            let pulse = 0.35 + 0.15 * (time * 6.0 + d).sin();
                            let m = Mat4::from_translation(at) * Mat4::from_scale(Vec3::new(0.96, 0.01, 0.96));
                            f.additive.push(Instance::new(m, lin(0xff5a1a), pulse, Vec3::ZERO, SHADE_UNLIT));
                        }
                        if let Some((ring, burst)) = steam {
                            if d.round() as u32 != ring {
                                continue;
                            }
                            let seed = hash(dx as f64 * 3.1, dz as f64 * 1.7) as f32;
                            if burst {
                                let m =
                                    Mat4::from_translation(at + Vec3::new(0.0, 1.1, 0.0)) * Mat4::from_scale(Vec3::new(0.55, 2.2, 0.55));
                                f.additive.push(Instance::new(m, lin(0xf4f8ff), 0.32, Vec3::ZERO, SHADE_UNLIT));
                                glow_orb(f, at + Vec3::new(0.0, 0.8, 0.0), 0.7, lin(0xffffff), 0.2);
                            } else {
                                for k in 0..2 {
                                    let tt = (time * 1.5 + seed * 3.0 + k as f32 * 0.5).fract();
                                    let m = Mat4::from_translation(at + Vec3::new((seed - 0.5) * 0.4, tt * 0.8, k as f32 * 0.2 - 0.1))
                                        * Mat4::from_scale(Vec3::splat(0.12 + tt * 0.15));
                                    f.additive.push(Instance::new(m, lin(0xe8f0f4), 0.22 * (1.0 - tt), Vec3::ZERO, SHADE_UNLIT));
                                }
                            }
                        }
                    }
                }
                if bell.phase == BellPhase::Lifted {
                    // The seam: the floor opening under the bell, the real portal's green below.
                    let m = Mat4::from_translation(c + Vec3::new(0.0, SHELF_Y + 0.03, 0.0))
                        * Mat4::from_rotation_x(-PI / 2.0)
                        * Mat4::from_scale(Vec3::new(SEAM_RADIUS as f32, SEAM_RADIUS as f32, 1.0));
                    f.transparent[MeshKind::Disc as usize].push(Instance::new(m, lin(0x02100a), 0.85, Vec3::ZERO, SHADE_UNLIT));
                    for k in 0..16 {
                        let a = time * 3.0 + k as f32 * TAU / 16.0;
                        let rr = SEAM_RADIUS as f32 * (0.4 + 0.6 * ((k as f32 * 0.31 + time).fract()));
                        let m = Mat4::from_translation(c + Vec3::new(a.cos() * rr, SHELF_Y + 0.05, a.sin() * rr))
                            * Mat4::from_rotation_y(-a)
                            * Mat4::from_scale(Vec3::new(0.3, 0.01, 0.05));
                        f.additive.push(Instance::new(m, lin(ALGAE), 0.6, Vec3::ZERO, SHADE_UNLIT));
                    }
                    glow_orb(f, c + Vec3::new(0.0, 0.2, 0.0), 1.8, lin(ALGAE), 0.5);
                    f.lights.push(Light { pos: c + Vec3::new(0.0, 0.3, 0.0), color: lin(ALGAE) * 5.0, distance: 5.0 });
                }
            }
        }
    }

    /// The bell itself: a cylinder of bronze staves flaring at the lip and rounding at the shoulder,
    /// its bands, its four panels of bosses, two striking-seats, the dome and its dragon loop.
    /// Down over him, the half toward the camera is cut away above its rim. `fade` below 1 dithers
    /// it out of his way; `hot` whitens the staves where the serpent's head presses.
    #[allow(clippy::too_many_arguments)]
    fn bell_shape(
        &self,
        v: &GrottoView,
        f: &mut Frame,
        rim: Vec3,
        radius: f32,
        height: f32,
        color: Vec3,
        glow: Vec3,
        cut: bool,
        fade: f32,
        hot: Option<f32>,
    ) {
        let cube_i = MeshKind::Cube as usize;
        let shade = if fade < 0.999 { SHADE_DITHER } else { SHADE_BRONZE };
        let bands = 11;
        let staves = 44;
        let profile = |t: f32| {
            let mut r = radius * (1.0 - 0.07 * t);
            if t < 0.12 {
                r *= 1.0 + 0.07 * (1.0 - t / 0.12);
            }
            if t > 0.82 {
                let u = (t - 0.82) / 0.18;
                r *= (1.0 - u * u * 0.85).max(0.1).sqrt();
            }
            r
        };
        let push = |f: &mut Frame, m: Mat4, col: Vec3, gl: Vec3| f.opaque[cube_i].push(Instance::new(m, col, fade, gl, shade));
        for b in 0..bands {
            let t = (b as f32 + 0.5) / bands as f32;
            let r = profile(t);
            let y = rim.y + t * height;
            let raised = matches!(b, 1 | 5 | 6 | 10);
            for i in 0..staves {
                let a = i as f32 / staves as f32 * TAU;
                if cut && b >= 2 && a.sin() > 0.15 {
                    continue;
                }
                let mut col = color * if raised { 0.85 } else { 1.0 } * (0.95 + 0.1 * ((i * 7 + b * 3) % 5) as f32 / 5.0);
                let mut gl = glow;
                if let Some(h) = hot {
                    let mut off = (a - h).rem_euclid(TAU);
                    if off > PI {
                        off = TAU - off;
                    }
                    if off < 0.35 {
                        let k = 1.0 - off / 0.35;
                        col = col.lerp(lin(0xffe8b0), k);
                        gl = gl.lerp(lin(0xffd890) * 0.9, k);
                    }
                }
                let w = TAU * r / staves as f32 * 1.1;
                let thick = if raised { 0.3 } else { 0.22 };
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::new(w, height / bands as f32 * 1.04, thick),
                    Quat::from_rotation_y(-(PI / 2.0 + a)),
                    Vec3::new(rim.x + a.cos() * r, y, rim.z + a.sin() * r),
                );
                push(f, m, col, gl);
                // The bosses, in the upper panels; the vertical bands between the panels.
                let quarter = (a / (PI / 2.0)).fract();
                if (0.62..0.8).contains(&t) && (0.2..0.8).contains(&quarter) && i % 2 == 0 {
                    let m = Mat4::from_scale_rotation_translation(
                        Vec3::splat(0.16),
                        Quat::from_rotation_y(-a),
                        Vec3::new(rim.x + a.cos() * (r + 0.13), y, rim.z + a.sin() * (r + 0.13)),
                    );
                    push(f, m, color * 1.1, gl);
                }
                if quarter < 0.04 || quarter > 0.96 {
                    let m = Mat4::from_scale_rotation_translation(
                        Vec3::new(0.2, height / bands as f32, 0.12),
                        Quat::from_rotation_y(-(PI / 2.0 + a)),
                        Vec3::new(rim.x + a.cos() * (r + 0.12), y, rim.z + a.sin() * (r + 0.12)),
                    );
                    push(f, m, color * 0.8, gl);
                }
            }
        }
        // The two striking-seats, lotus medallions low on the bell.
        for a in [0.0f32, PI] {
            if cut && a.sin() > 0.15 {
                continue;
            }
            let t = 0.28;
            let r = profile(t) + 0.14;
            let center = Vec3::new(rim.x + a.cos() * r, rim.y + t * height, rim.z + a.sin() * r);
            for k in 0..8 {
                let pa = k as f32 / 8.0 * TAU;
                let tangent = Vec3::new(-a.sin(), 0.0, a.cos());
                let at = center + tangent * pa.cos() * 0.32 + Vec3::new(0.0, pa.sin() * 0.32, 0.0);
                push(f, Mat4::from_scale_rotation_translation(Vec3::splat(0.14), Quat::from_rotation_y(-a), at), color * 1.15, glow);
            }
            push(f, Mat4::from_scale_rotation_translation(Vec3::splat(0.2), Quat::from_rotation_y(-a), center), color * 1.2, glow);
        }
        // The dome and its dragon loop, and the chain it hangs by.
        let top = rim.y + height;
        if !cut {
            for k in 0..3 {
                let rr = radius * 0.62 * (1.0 - k as f32 * 0.3);
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::new(rr * 2.0, 0.16, rr * 2.0),
                    Quat::from_rotation_y(k as f32 * 0.4),
                    Vec3::new(rim.x, top + 0.08 + k as f32 * 0.16, rim.z),
                );
                push(f, m, color * 0.95, glow);
            }
            if fade >= 0.999 {
                push_merged(
                    &mut f.opaque[cube_i],
                    &v.art.crown,
                    Mat4::from_translation(Vec3::new(rim.x, top + 0.5, rim.z)) * Mat4::from_scale(Vec3::splat(1.4)),
                    0.0,
                );
            }
            for k in 0..8 {
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::new(0.14, 0.5, 0.36),
                    Quat::from_rotation_y(if k % 2 == 0 { 0.0 } else { PI / 2.0 }),
                    Vec3::new(rim.x, top + 2.4 + k as f32 * 0.45, rim.z),
                );
                push(f, m, lin(0x3a3a34), Vec3::ZERO);
            }
        }
    }

    /// The serpent coiled round the bell's shoulder, its head reared over the rim on the far side,
    /// pressing on the bronze and looking in; a bell lifted, it loosens and slides down.
    #[allow(clippy::too_many_arguments)]
    fn serpent_round_bell(&self, v: &GrottoView, f: &mut Frame, c: Vec3, radius: f32, height: f32, head: f32, lift: f32, time: f32) {
        let cube_i = MeshKind::Cube as usize;
        let rc = radius * 0.98 + lift * 0.5;
        let turns = 1.6 * TAU;
        let mut u = 0.5;
        let mut k = 0;
        while u < turns {
            let a = head - u;
            // The half toward the camera stays clear, as the bell's own is cut away.
            if a.sin() < 0.1 {
                let y = height - 0.1 + (u / turns) * 0.7 - lift * 0.8 + (time * 1.5 + u * 2.0).sin() * 0.04;
                let at = Vec3::new(c.x + a.cos() * rc, y, c.z + a.sin() * rc);
                let tangent = Vec2::new(a.sin(), -a.cos());
                let girth = 1.15 * (1.0 - u / turns * 0.35);
                self.coil_length(f, at, tangent, girth, 0.6, k, true);
            }
            u += 0.13;
            k += 1;
        }
        // The neck, rising from the coils to the head over the rim.
        let to = Vec3::new(-head.cos(), 0.0, -head.sin());
        let rim = Vec3::new(c.x + head.cos() * radius, height + 0.3 - lift * 0.6, c.z + head.sin() * radius);
        for j in 0..5 {
            let t = j as f32 / 4.0;
            let at = rim - to * (0.9 - t * 0.6) + Vec3::new(0.0, -0.3 + t * 0.5, 0.0);
            self.coil_length(f, at, Vec2::new(to.x, to.z), 1.0, 0.5, j, false);
        }
        let yaw = to.x.atan2(to.z);
        let m = Mat4::from_translation(rim - to * 0.6 + Vec3::new(0.0, 0.1, 0.0))
            * Mat4::from_rotation_y(yaw)
            * Mat4::from_rotation_x(0.45)
            * Mat4::from_scale(Vec3::splat(1.0));
        push_merged(&mut f.opaque[cube_i], &v.art.head, m, 0.0);
        let eye = rim + to * 0.3 + Vec3::new(0.0, 0.25, 0.0);
        glow_orb(f, eye, 0.7, lin(art_pal::EYE & 0xffffff), 0.25 + 0.05 * (time * 4.0).sin());
    }

    /// Shallows over the black pool while the bell is down on it (but where its seam opens).
    fn bell_floor(&self, f: &mut Frame, r: &Grotto) {
        let cube_i = MeshKind::Cube as usize;
        let bell = &r.bell;
        for p in &bell.pool {
            let d = bell.dist(p.v());
            if bell.phase == BellPhase::Lifted && d <= SEAM_RADIUS {
                continue;
            }
            let h = hash(p.x as f64 * 3.1, p.y as f64 * 2.3) as f32;
            let m = Mat4::from_scale_rotation_translation(
                Vec3::new(0.98, 0.1, 0.98),
                Quat::IDENTITY,
                Vec3::new(p.x as f32, SHELF_Y - 0.05, p.y as f32),
            );
            f.opaque[cube_i].push(Instance::new(m, lin(SHELF_L) * (0.9 + h * 0.3), 1.0, Vec3::ZERO, SHADE_LIT));
        }
    }

    /// The lanterns of the Drowned Floor (where he walks it, or as they glow up through the water),
    /// the algae round the pools, and a warm haze from below.
    fn emit_lights(&self, v: &GrottoView, _r: &Grotto, drowned: bool, hero: Vec3, time: f32, f: &mut Frame) {
        let lamps = if drowned { &v.lamps } else { &v.copy_lamps };
        let near: Vec<Vec3> = lamps.iter().copied().filter(|l| Vec2::new(l.x - hero.x, l.z - hero.z).length() < RANGE).collect();
        for (i, l) in near.iter().enumerate() {
            let flick = 0.04 * (time * 5.0 + i as f32 * 1.3).sin();
            glow_orb(f, *l, if drowned { 0.45 } else { 0.7 }, lin(LAMP), if drowned { 0.2 + flick } else { 0.07 + flick * 0.3 });
        }
        for (i, p) in nearest(&near, hero, 5).into_iter().enumerate() {
            let flick = (time * 6.0 + i as f32 * 1.7).sin() * 0.3;
            let (power, reach) = if drowned { (4.0, 7.0) } else { (2.2, 4.0) };
            f.lights.push(Light { pos: p + Vec3::new(0.0, 0.25, 0.0), color: lin(0xf0dc96) * (power + flick), distance: reach });
        }
        if !drowned {
            let algae: Vec<Vec3> = v.algae.iter().copied().filter(|l| Vec2::new(l.x - hero.x, l.z - hero.z).length() < RANGE).collect();
            for l in &algae {
                glow_orb(f, *l, 0.5, lin(ALGAE), 0.08);
            }
            for p in nearest(&algae, hero, 2) {
                f.lights.push(Light { pos: p, color: lin(ALGAE) * 1.4, distance: 3.5 });
            }
        }
    }
}
