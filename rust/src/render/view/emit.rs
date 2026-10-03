//! Each frame's draw list: the hero, the foes, pickups, shots and bombs, the boss, the level's
//! own life (water, lava, the vaults' gold), the sea of cloud, and the lights nearest the hero.

use super::*;

const LIGHT_POOL_TORCHES: usize = 3;

const LIGHT_POOL_REMEDIES: usize = 3;

const LIGHT_POOL_SHRINES: usize = 2;

/// The dungeon floats on a sea of dark clouds: stacked mist layers under and around it. The
/// upper layers only show outside the level and swallow its sides; the lowest hides the void.
/// How far outside the island each rank of the smoke wall stands.
const SMOKE_RANKS: [f32; 5] = [0.35, 1.3, 2.6, 4.2, 6.5];

/// The smoke wall's sheets reach from down in the cloud sea to above its highest crest.
const SMOKE_BOTTOM: f32 = -2.6;

const SMOKE_TOP: f32 = 7.4;

/**
 * The cloud sea the island floats on, and the smoke wall round its edges:
 * ranks of billowing curtains that rise to about five times Ragnar's
 * height. A rank thins away as the camera comes up to it, so the view out
 * from near the edge stays clear, and from far outside (the map) the near
 * ranks are left out altogether.
 */
fn emit_cloud_sea(grid: &Grid, f: &mut Frame, eye: Vec3) {
    let tint = f.atmosphere.cloud_tint;
    let (w, h) = (grid.width as f32, grid.height as f32);
    f.bounds = Vec4::new(-0.5, -0.5, w - 0.5, h - 0.5);
    let center = Vec3::new(w / 2.0 - 0.5, 0.0, h / 2.0 - 0.5);
    let size = (w.max(h) + 70.0) * 2.0;
    // (height, opacity, seed), lowest first so they blend back to front.
    let layers = [
        (-1.6, 1.0, 0.0),
        (-1.2, 0.9, 1.0),
        (-0.8, 0.85, 2.0),
        (-0.45, 0.8, 3.0),
        (-0.1, 0.8, 4.0),
        (0.15, 0.75, 5.0),
        (0.4, 0.65, 6.0),
        (0.7, 0.5, 7.0),
        (1.0, 0.35, 8.0),
    ];
    for (y, alpha, seed) in layers {
        let m = Mat4::from_translation(center + Vec3::new(0.0, y, 0.0)) * Mat4::from_scale(Vec3::new(size, 0.002, size));
        f.transparent[MeshKind::Cube as usize].push(Instance::new(m, tint, alpha, Vec3::new(seed, y, 0.0), SHADE_CLOUD));
    }

    // The smoke wall: north, south, west and east, each rank a curtain the length of the side and then some.
    let b = f.bounds;
    let mid_y = (SMOKE_BOTTOM + SMOKE_TOP) / 2.0;
    let tall = SMOKE_TOP - SMOKE_BOTTOM;
    let mut curtains: Vec<(f32, Instance)> = vec![];
    for (rank, &out) in SMOKE_RANKS.iter().enumerate() {
        for side in 0..4 {
            let along_x = side < 2;
            let len = if along_x { w } else { h } + 2.0 * out + 6.0;
            let (pos, eye_out) = match side {
                0 => (Vec3::new(center.x, mid_y, b.y - out), b.y - eye.z),
                1 => (Vec3::new(center.x, mid_y, b.w + out), eye.z - b.w),
                2 => (Vec3::new(b.x - out, mid_y, center.z), b.x - eye.x),
                _ => (Vec3::new(b.z + out, mid_y, center.z), eye.x - b.z),
            };
            let k = ((out - eye_out) / 1.5).clamp(0.0, 1.0);
            if k <= 0.0 {
                continue;
            }
            let scale = if along_x { Vec3::new(len, tall, 0.002) } else { Vec3::new(0.002, tall, len) };
            let m = Mat4::from_translation(pos) * Mat4::from_scale(scale);
            curtains.push((out - eye_out, Instance::new(m, tint, k, Vec3::new(30.0 + rank as f32, out, side as f32), SHADE_CLOUD)));
        }
    }
    // Farthest first, so the nearer ranks blend over the ones behind them.
    curtains.sort_by(|a, b| b.0.total_cmp(&a.0));
    f.transparent_early.extend(curtains.into_iter().map(|(_, i)| i));
}

/// Slime's wet material: glossy, a sharp highlight (voxel material 9).
const SHADE_GOO: f32 = 14.0;

/**
 * One tile of slime trail: a lumpy splat of glossy goo with droplets flung
 * around it, a smear joining it to the next trail tile, and bubbles that swell
 * and pop. `life` runs 1 (fresh) to 0 (dried up); the goo shrinks as it dries.
 */
fn emit_goo(f: &mut Frame, tile: P, floor: f32, life: f32, time: f32, trail: &std::collections::HashSet<P>) {
    let disc = MeshKind::Disc as usize;
    let k = (life / 0.4).min(1.0).sqrt();
    if k <= 0.02 {
        return;
    }
    let (tx, tz) = (tile.x as f32, tile.y as f32);
    let (hx, hy) = (tile.x as f64, tile.y as f64);
    let goo = lin(0x46d23c);
    let goo_l = lin(0x9cf06a);
    let glow = lin(0x5cff6a) * 0.14;
    let mut lift = 0.012;
    let mut blob = |f: &mut Frame, x: f32, z: f32, rx: f32, rz: f32, rot: f32, color: Vec3| {
        let m = Mat4::from_translation(Vec3::new(x, floor + lift, z))
            * Mat4::from_rotation_y(rot)
            * Mat4::from_rotation_x(-PI / 2.0)
            * Mat4::from_scale(Vec3::new(rx, rz, 1.0));
        f.opaque[disc].push(Instance::new(m, color, 1.0, glow, SHADE_GOO));
        lift += 0.0015;
    };
    let wob = 1.0 + (time * 2.1 + tx * 1.3 + tz * 0.7).sin() * 0.05;
    let jx = (hash(hx * 3.0, hy * 5.0) as f32 - 0.5) * 0.12;
    let jz = (hash(hy * 3.0, hx * 5.0) as f32 - 0.5) * 0.12;
    let spin = hash(hx, hy) as f32 * TAU;
    // The main splat: two overlapping lobes.
    blob(f, tx + jx, tz + jz, 0.25 * k * wob, 0.19 * k / wob, spin, goo);
    blob(f, tx + jx + spin.cos() * 0.08, tz + jz + spin.sin() * 0.08, 0.15 * k / wob, 0.12 * k * wob, spin + 1.3, goo_l);
    // Droplets flung around it.
    for i in 0..5 {
        let hi = hash(hx * 7.0 + i as f64, hy * 11.0) as f32;
        let a = spin + i as f32 * 1.3 + hi;
        let d = 0.24 + hi * 0.14;
        let r = (0.035 + hi * 0.045) * k;
        blob(f, tx + jx + a.cos() * d, tz + jz + a.sin() * d, r, r * 0.85, a, if i % 2 == 0 { goo } else { goo_l });
    }
    // A smear toward the next tile of the trail, so it reads as one track.
    for (dx, dz) in [(1, 0), (0, 1)] {
        if trail.contains(&P::new(tile.x + dx, tile.y + dz)) {
            let (rx, rz) = if dx != 0 { (0.36, 0.1) } else { (0.1, 0.36) };
            blob(f, tx + dx as f32 * 0.5, tz + dz as f32 * 0.5, rx * k, rz * k, 0.0, goo);
        }
    }
    // Bubbles: swell, then pop.
    for b in 0..2 {
        let hb = hash(hx * 13.0 + b as f64, hy * 17.0) as f32;
        let phase = (time * (0.35 + hb * 0.3) + hb * 5.0).fract();
        let r = phase.powf(0.7) * 0.04 * k;
        if phase < 0.95 && r > 0.004 {
            let p = Vec3::new(tx + jx + (hb - 0.5) * 0.24, floor + 0.012 + r * 0.6, tz + jz + (0.5 - hb) * 0.2);
            let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(r));
            f.opaque[MeshKind::Sphere as usize].push(Instance::new(m, goo_l, 1.0, glow, SHADE_GOO));
        }
    }
}

impl WorldView {
    /// Everything that moves or animates, plus the light list, for one rendered frame.
    pub fn emit(&self, s: &SceneState, f: &mut Frame) {
        f.clear();
        {
            let fwd = (self.cam_look - self.cam_pos).normalize_or(Vec3::NEG_Z);
            let right = fwd.cross(Vec3::Y).normalize_or(Vec3::X);
            let up = right.cross(fwd);
            f.facing = Quat::from_mat3(&glam::Mat3::from_cols(right, up, -fwd));
        }
        f.skip_static = self.showcase || self.forge || self.select.is_some() || self.story.is_some();
        f.time = s.time as f32;
        let lod = self.lod_policy();
        f.rigs.lod = lod;
        f.rigs_blend.lod = lod;
        if self.lookdev.is_some() {
            self.emit_lookdev(s, f);
            return;
        }
        if let Some(sv) = self.story {
            self.emit_story(sv, s, f);
            return;
        }
        if self.forge {
            self.emit_wares(s, f);
            return;
        }
        if let Some(sel) = self.select {
            self.emit_select(sel, s, f);
            return;
        }
        f.atmosphere = if self.showcase {
            Atmosphere::forest()
        } else if self.vault.is_some() {
            vault::vault_atmosphere()
        } else if let Some(a) = self.run_atmosphere(s) {
            a
        } else if self.hell {
            hell::hell_atmosphere()
        } else if self.meadow {
            meadow_atmosphere()
        } else {
            Atmosphere::forest()
        };
        if let Some(c) = self.tint.filter(|_| !self.showcase) {
            f.atmosphere = f.atmosphere.tinted(c);
        }
        f.haze = self.level_haze();
        // Seer's Sight (path 3) sees through the dark.
        let nv = if s.hero.sees() { 0.6 } else { 0.0 };
        if nv > 0.0 && !self.meadow && !self.hell && !self.showcase && self.vault.is_none() && self.run_view.is_none() {
            let a = &mut f.atmosphere;
            a.ambient *= 1.0 + 3.0 * nv;
            a.hemi_sky *= 1.0 + 1.4 * nv;
            a.hemi_ground *= 1.0 + 2.5 * nv;
            a.sun *= 1.0 + 1.0 * nv;
            a.fog_near += 10.0 * nv;
            a.fog_far += 20.0 * nv;
        }
        if self.showcase {
            self.emit_stage(s, f);
            return;
        }
        let time = s.time as f32;
        f.coin_pulse = 0.35 + (time * 3.0).sin() * 0.2;

        let hero_pos = self.emit_hero(s, f);
        self.emit_foes(s, f);
        self.emit_duels(s, f);
        let (remedy_points, shrine_points) = self.emit_pickups(s, f);
        self.emit_transients(s, f);
        self.emit_world(s, f, hero_pos);
        self.emit_light_pools(f, time, Vec3::new(hero_pos.x, 0.0, hero_pos.z), &remedy_points, &shrine_points);
    }

    /// Each kind of level's ground haze (RP19): a low night mist in the dungeons, a thin dusk haze in
    /// the meadow, heat over the lava, dust in the vaults, and each realm its own (the Grey Mire
    /// thickest). Kept thin enough that a foe charging from across a room stays plain to see.
    fn level_haze(&self) -> Haze {
        let haze = |density, falloff| Haze { density, floor: 0.0, falloff };
        if self.showcase {
            Haze::default()
        } else if self.vault.is_some() {
            haze(0.008, 2.5)
        } else if let Some(r) = &self.run_view {
            match r {
                realms::RunView::Sloth(_) => haze(0.06, 1.2),
                _ => haze(0.025, 1.0),
            }
        } else if self.hell {
            Haze { density: 0.03, floor: -1.0, falloff: 1.2 }
        } else if self.meadow {
            haze(0.018, 1.5)
        } else {
            haze(0.035, 0.8)
        }
    }

    /// The hero: his body, his gifts' glow, his lantern, the trail of his swing and his shadow.
    /// Returns where he stands, for what is drawn around him.
    fn emit_hero(&self, s: &SceneState, f: &mut Frame) -> Vec3 {
        let time = s.time as f32;
        let grid = s.grid;
        let hero = s.hero;
        let hp = hero.pos();
        let (hx, hz) = (hp.x as f32, hp.y as f32);
        let bob = if hero.airborne() { 0.0 } else { hero.motion as f32 * self.hero_phase.cos().abs() * 0.05 };
        let hero_pos = Vec3::new(hx, hero.altitude as f32 * STEP + bob, hz);
        let hero_group = Mat4::from_translation(hero_pos) * Mat4::from_rotation_y(self.hero_yaw);
        let blink = hero.invulnerable() && !hero.invincible() && ((time * 16.0).floor() as i64).rem_euclid(2) == 0;
        let ghost = hero.invisible();
        let (mut emissive, light_color) = if hero.invincible() {
            (lin(INVINCIBLE) * (0.45 + (time * 12.0).sin() * 0.25), lin(INVINCIBLE))
        } else {
            (Vec3::ZERO, lin(HERO_LIGHT))
        };
        if hero.fx.strength > 0.0 {
            emissive += lin(0xff2a1a) * (0.12 + (time * 6.0).sin().abs() * 0.12);
        }
        // A shield potion's bark greys his skin a little; frostbite blues it, venom greens it.
        let mut tint = if hero.bark > 0 { Vec3::new(0.82, 0.84, 0.88) } else { Vec3::ONE };
        let ails = s.ails;
        if ails.frost > 0 {
            tint *= Vec3::new(0.72, 0.88, 1.2);
            emissive += lin(0x6ab8ff) * (0.1 + (time * 4.0).sin().abs() * 0.06);
        }
        if ails.venom > 0 {
            emissive += lin(0x6ad040) * (0.08 + (time * 5.0).sin().abs() * 0.1);
        }
        if hero.fx.ragnarok > 0.0 {
            emissive += lin(0x9a2aff) * (0.1 + (time * 5.0).sin().abs() * 0.1);
        }
        if hero.alive && !blink {
            let alpha = if ghost { 0.3 + (time * 6.0).sin() * 0.08 } else { 1.0 };
            let root = hero_group * Mat4::from_translation(Vec3::new(0.0, 0.0, self.hero_lunge)) * euler(self.hero_root_rot);
            let target = if ghost { &mut f.rigs_blend } else { &mut f.rigs };
            let rust = ((1.0 - hero.edge) as f32 * 0.9).clamp(0.0, 0.85);
            // He is always drawn whole: under the top camera he covers under 2 px a voxel at 1080p,
            // where foes go to half detail, but he is the one the player reads (RP02).
            let bias = target.lod.bias;
            target.lod.bias = 0.0;
            match (&self.models.raised, hero.blocking) {
                (Some((bare, shield)), true) => {
                    // The shield comes off his back and up before him, on his shield arm's side.
                    emit_hero_rig(target, bare, &self.hero_pose, root, tint, alpha, emissive, rust);
                    let side = self.hero_pose.pos.get(3).map(|p| p.x * 0.55).unwrap_or(-0.12);
                    let up = root * Mat4::from_translation(Vec3::new(side, 0.06, 0.12)) * Mat4::from_rotation_y(PI);
                    emit_rig(target, shield, &Pose::rest(shield), up, tint, alpha, emissive);
                }
                _ => emit_hero_rig(target, self.hero_rig(hero), &self.hero_pose, root, tint, alpha, emissive, rust),
            }
            target.lod.bias = bias;
        }
        self.emit_hero_fx(hero, hero_pos, time, f);
        self.emit_ails(&ails, hero_pos, time, f);
        if let (true, Some(sh)) = (hero.blocking && hero.alive, hero.shield) {
            // A faint ward of its colour about the raised shield.
            let color = lin(progression::node(s.hero_kind, progression::Tree::Shield, sh.tier).color);
            let pulse = 0.9 + (time * 7.0).sin() * 0.1;
            let ward =
                hero_group * Mat4::from_translation(Vec3::new(0.0, 0.5, 0.5)) * Mat4::from_scale(Vec3::new(0.5 * pulse, 0.55 * pulse, 1.0));
            f.transparent[MeshKind::Annulus as usize].push(Instance::new(ward, color * 1.4, 0.35, Vec3::ZERO, SHADE_UNLIT));
        }
        let mut hero_light = if hero.alive { 8.0 } else { 0.0 };
        if ghost {
            hero_light = 3.0;
        }
        if self.meadow {
            hero_light *= 0.25; // broad daylight; the lantern barely shows
        }
        if self.hell {
            hero_light *= 0.45; // the lava lights him from below
        }
        if self.vault.is_some() {
            hero_light *= 0.5; // candlelight everywhere
        }
        if let Some(k) = self.run_lantern() {
            hero_light *= k; // each realm has its own light
        }
        // His lantern rides well above the horned helmet, so it lights him without bleaching him.
        let to_cam = {
            let d = self.cam_pos - hero_pos;
            let flat = Vec3::new(d.x, 0.0, d.z);
            if flat.length_squared() > 1e-4 {
                flat.normalize()
            } else {
                Vec3::Z
            }
        };
        f.lights.push(Light {
            pos: hero_pos + Vec3::new(0.0, 2.3, 0.0) + to_cam * 0.9,
            color: light_color * hero_light * 2.2,
            distance: 8.5,
        });

        if self.slash_t >= 0.0 {
            // The blade's trail: a bright leading arc and fading echoes behind it, swept across the front.
            let p = self.slash_p();
            let q = ((p - 0.17) / 0.33).clamp(0.0, 1.0);
            let env = if p < 0.15 { 0.0 } else { (1.0 - ((p - 0.15) / 0.55)).clamp(0.0, 1.0) };
            let side = self.slash_side;
            // The trail grows with the blade's reach and takes its colour.
            // (The ring is drawn at 1.08 across; a blade's swing lands wherever the trail sweeps, see `in_blade_arc`.)
            let reach = progression::blade(s.hero_kind, hero.blade).swing.map(|s| s.0).unwrap_or(1.2);
            let size = (systems::sweep_radius(reach) / 1.08) as f32;
            let axe_color = if hero.blade <= 1 {
                lin(SLASH)
            } else {
                lin(progression::node(s.hero_kind, crate::logic::progression::Tree::Armory, hero.blade).color).lerp(Vec3::ONE, 0.35)
            };
            for k in 0..5 {
                let qk = (q - k as f32 * 0.09).clamp(0.0, 1.0);
                let sweep = side * (1.15 - 2.3 * (1.0 - (1.0 - qk).powi(3)));
                let alpha = env * (0.85 - k as f32 * 0.16);
                if alpha <= 0.01 {
                    continue;
                }
                let m = hero_group
                    * Mat4::from_translation(Vec3::new(0.0, 0.45 - k as f32 * 0.02, 0.12))
                    * Mat4::from_rotation_y(sweep)
                    * Mat4::from_rotation_z(side * 0.32)
                    * euler(Vec3::new(-PI / 2.0, 0.0, PI))
                    * Mat4::from_scale(Vec3::splat((1.08 + k as f32 * 0.03) * size));
                let color = if k == 0 { lin(0xffffff) } else { axe_color };
                f.transparent[MeshKind::Ring as usize].push(Instance::new(m, color, alpha, Vec3::ZERO, SHADE_UNLIT));
            }
        }

        if hero.alive {
            // A drop shadow that stays on the floor while he jumps, so height reads clearly.
            let ground_h = floor_at(grid, hx, hz);
            let lift = (hero.altitude as f32 * STEP - ground_h).max(0.0);
            let sh = (1.0 - lift * 0.5).max(0.5);
            let m = Mat4::from_translation(Vec3::new(hx, ground_h + 0.015, hz))
                * Mat4::from_rotation_x(-PI / 2.0)
                * Mat4::from_scale(Vec3::new(0.32 * sh, 0.32 * sh, 1.0));
            f.transparent[MeshKind::Disc as usize].push(Instance::new(m, Vec3::ZERO, 0.45 * sh, Vec3::ZERO, SHADE_UNLIT));
        }
        hero_pos
    }

    /// The foes near enough to see: their bodies, the glow under them, a halo over the turned and
    /// stars round the befuddled.
    fn emit_foes(&self, s: &SceneState, f: &mut Frame) {
        let view_proj = self.view_proj();
        let time = s.time as f32;
        for (i, v) in self.enemy_views.iter().enumerate() {
            if i >= s.enemies.len() {
                break;
            }
            if v.group_pos.distance(self.cam_pos) > f.atmosphere.fog_far + 4.0 {
                continue;
            }
            // The Peerless wears his own likeness: the Hall of Mirrors draws it.
            if v.kind == EnemyKind::Peerless {
                continue;
            }
            // Nor those out of the camera's view (with a margin for their size and the light they cast).
            let clip = view_proj * (v.group_pos + Vec3::new(0.0, 0.5, 0.0)).extend(1.0);
            if clip.w <= 0.0 || clip.x.abs() > clip.w * 1.25 + 1.0 || clip.y.abs() > clip.w * 1.25 + 1.0 {
                continue;
            }
            let group = Mat4::from_translation(v.group_pos) * Mat4::from_rotation_y(v.yaw);
            if v.visible {
                let root = group * euler(v.root_rot) * Mat4::from_scale(v.root_scale);
                let rig = self.models.rig(v.rig);
                emit_rig(&mut f.rigs, rig, &v.pose, root, v.mat_color, 1.0, v.emissive);
                if v.eye > 0.0 {
                    if let Some(h) = rig.index("head") {
                        let pivot = Vec3::from(crate::render::foes6::TEPEGOZ_RIG[4].pivot);
                        let eye = (Vec3::from(crate::render::foes6::TEPEGOZ_EYE) - pivot) * 0.02;
                        let p = (root * Mat4::from_translation(v.pose.pos[h]) * euler(v.pose.rot[h])).transform_point3(eye);
                        let pulse = 0.8 + (time * 10.0).sin().abs() * 0.4;
                        glow_orb(f, p, 0.35, lin(0xfff0a0), 0.9 * pulse * v.eye);
                        f.lights.push(Light { pos: p, color: lin(0xffe080) * 3.0 * v.eye, distance: 2.5 });
                    }
                }
            }
            if v.light_intensity > 0.0 {
                // Inside the body, so it lights the ground round the foe and not the foe; a draugr's is low,
                // under his open ribs.
                let lift = match v.kind {
                    EnemyKind::Skeleton | EnemyKind::Helmeted | EnemyKind::Minion => 0.2,
                    EnemyKind::Bat | EnemyKind::LargeBat => 0.13,
                    EnemyKind::Vampire => 0.16,
                    EnemyKind::Slime | EnemyKind::Werewolf => 0.3,
                    EnemyKind::Witch => 0.4,
                    EnemyKind::Blemmyes => 0.5,
                    EnemyKind::Tengu => 0.45,
                    EnemyKind::Rasetsu => 0.55,
                    EnemyKind::DireWolf | EnemyKind::Cobra => 0.3,
                    EnemyKind::Jorogumo => 0.35,
                    EnemyKind::Kappa | EnemyKind::Kitsune => 0.3,
                    EnemyKind::Tulukus | EnemyKind::Rokurokubi | EnemyKind::Cin | EnemyKind::Albasti => 0.4,
                    EnemyKind::Oni | EnemyKind::Karakoncolos | EnemyKind::Arcura | EnemyKind::Goryo | EnemyKind::Hel => 0.6,
                    EnemyKind::Tepegoz
                    | EnemyKind::Gashadokuro
                    | EnemyKind::Ejder
                    | EnemyKind::FrostGiant
                    | EnemyKind::Surtr
                    | EnemyKind::Nidhogg => 1.0,
                    EnemyKind::Peerless | EnemyKind::Ganglot | EnemyKind::Gaki | EnemyKind::Ellepige => 0.45,
                };
                f.lights.push(Light {
                    pos: v.group_pos + Vec3::new(0.0, lift, 0.0),
                    color: v.light_color * v.light_intensity,
                    distance: 4.0,
                });
            }
            if s.enemies[i].is_ally() && v.visible {
                // A halo over the turned.
                let top = crown(v.kind).unwrap_or(match v.kind {
                    EnemyKind::Skeleton => 1.18,
                    EnemyKind::Witch => 1.3,
                    EnemyKind::Helmeted | EnemyKind::Blemmyes => 1.05,
                    EnemyKind::Slime => 0.78,
                    EnemyKind::Werewolf | EnemyKind::Minion => 0.68,
                    _ => 0.32,
                });
                let blue = lin(Archangel::Michael.color());
                let m = Mat4::from_translation(v.group_pos + Vec3::new(0.0, top, 0.0))
                    * Mat4::from_rotation_x(-PI / 2.0)
                    * Mat4::from_scale(Vec3::new(0.2, 0.2, 1.0));
                f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, blue * 2.4, 0.85, Vec3::ZERO, SHADE_UNLIT));
            }
            if s.enemies[i].is_confused() && v.visible {
                for k in 0..3 {
                    let a = time * 5.0 + k as f32 * TAU / 3.0;
                    let p = v.group_pos + Vec3::new(a.cos() * 0.28, 1.0 + (time * 7.0 + k as f32).sin() * 0.04, a.sin() * 0.28);
                    f.additive.push(Instance::new(
                        Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.05)),
                        lin(0xc89aff) * 1.6,
                        0.9,
                        Vec3::ZERO,
                        SHADE_UNLIT,
                    ));
                }
            }
        }
    }

    /// What lies about to be picked up or visited: green crosses, shrines, the hidden relic, a
    /// decoy, the spirits and fires, bottles. Returns the remedies and the lit shrines, whose light
    /// the nearest few cast.
    fn emit_pickups(&self, s: &SceneState, f: &mut Frame) -> (Vec<Vec3>, Vec<(Vec3, Vec3)>) {
        let time = s.time as f32;
        let grid = s.grid;
        let cube = MeshKind::Cube as usize;
        // --- remedies
        let mut remedy_points = Vec::new();
        let remedy_glow = lin(REMEDY) * 0.75;
        for (p, base) in &self.charges {
            let pos = Vec3::new(base.x, floor_at(grid, p.x as f32, p.y as f32) + 0.15 + (time * 3.0).sin() * 0.05, base.z);
            let m = Mat4::from_translation(pos) * Mat4::from_rotation_y(time * 1.5);
            emit_mesh(&mut f.opaque[cube], &self.models.medkit, m, Vec3::ONE, 1.0, remedy_glow);
            remedy_points.push(pos);
        }

        // --- shrines
        let mut shrine_points = Vec::new();
        for (i, d) in s.diamonds.iter().enumerate() {
            if d.kind == DiamondKind::Angel {
                if let Some(lit) = self.emit_angel_shrine(d, grid, &s.angels_spent, f) {
                    shrine_points.push(lit);
                }
                continue;
            }
            let base = Vec3::new(d.tile.x as f32, floor_at(grid, d.tile.x as f32, d.tile.y as f32), d.tile.y as f32);
            let (stone, runes) = &self.models.pedestals[1];
            let rune = lin(0xffc84a);
            let rune_glow = if d.active { 0.6 + (time * 2.5 + d.tile.x as f32).sin().abs() * 0.6 } else { 0.1 };
            emit_mesh(&mut f.opaque[cube], stone, Mat4::from_translation(base), Vec3::ONE, 1.0, Vec3::ZERO);
            emit_mesh(&mut f.opaque[cube], runes, Mat4::from_translation(base), Vec3::ONE, 1.0, rune * rune_glow);
            if !d.active {
                continue;
            }
            let hex = d.current().color();
            let spin = self.diamond_spin.get(i).copied().unwrap_or(0.0);
            let m = Mat4::from_translation(base + Vec3::new(0.0, 0.45 + (time * 2.0 + d.tile.x as f32).sin() * 0.06, 0.0))
                * Mat4::from_rotation_y(spin);
            emit_mesh(&mut f.opaque[cube], &self.models.diamond, m, lin(hex), 1.0, lin(hex) * 0.3);
            shrine_points.push((base, lin(hex)));
        }

        // --- the hidden shrine: a pedestal with a rune stone turning over it, and with Seer's Sight a beam through the walls
        if let Some(t) = s.relic {
            let base = Vec3::new(t.x as f32, floor_at(grid, t.x as f32, t.y as f32), t.y as f32);
            let (stone, runes) = &self.models.pedestals[0];
            let rune = lin(0x9ad1ff);
            emit_mesh(&mut f.opaque[cube], stone, Mat4::from_translation(base), Vec3::ONE, 1.0, Vec3::ZERO);
            emit_mesh(
                &mut f.opaque[cube],
                runes,
                Mat4::from_translation(base),
                Vec3::ONE,
                1.0,
                rune * (0.8 + (time * 3.0).sin().abs() * 0.8),
            );
            let m = Mat4::from_translation(base + Vec3::new(0.0, 0.55 + (time * 2.0).sin() * 0.07, 0.0))
                * Mat4::from_rotation_y(time * 1.4)
                * Mat4::from_scale(Vec3::new(0.2, 0.26, 0.1));
            f.opaque[cube].push(Instance::new(m, lin(0x8a8e96), 1.0, rune * 0.5, SHADE_LIT));
            glow_orb(f, base + Vec3::new(0.0, 0.6, 0.0), 0.7, rune, 0.35);
            f.lights.push(Light { pos: base + Vec3::new(0.0, 0.9, 0.0), color: rune * 5.0, distance: 3.5 });
            if s.hero.sees() {
                let beam = Mat4::from_translation(base + Vec3::new(0.0, 3.0, 0.0)) * Mat4::from_scale(Vec3::new(0.12, 6.0, 0.12));
                f.additive.push(Instance::new(beam, rune, 0.35 + 0.1 * (time * 4.0).sin(), Vec3::ZERO, SHADE_UNLIT));
            }
        }
        if let Some(d) = s.decoy {
            let at = Vec3::new(d.x as f32, floor_at(grid, d.x as f32, d.y as f32), d.y as f32);
            self.emit_decoy(at, time, f);
        }
        self.emit_spirits(s, time, f);
        self.emit_fires(s, time, f);
        self.emit_angel_fx(time, f);

        // --- potions
        for p in s.potions {
            if p.taken {
                continue;
            }
            let hex = p.def(s.hero_kind).color;
            let pos = Vec3::new(
                p.tile.x as f32,
                floor_at(grid, p.tile.x as f32, p.tile.y as f32) + 0.05 + (time * 2.5 + p.tile.x as f32).sin() * 0.04,
                p.tile.y as f32,
            );
            let m = Mat4::from_translation(pos) * Mat4::from_rotation_y(time * 1.2);
            if let Some(mesh) = self.models.bottles.get(&hex) {
                emit_mesh(&mut f.opaque[cube], mesh, m, Vec3::ONE, 1.0, lin(hex) * 0.5);
            }
        }
        (remedy_points, shrine_points)
    }

    /// What is in flight or fading: bombs and their fuses, shots, slime trails, debris and blasts,
    /// sparks, and the tour's spotlight.
    fn emit_transients(&self, s: &SceneState, f: &mut Frame) {
        let time = s.time as f32;
        let grid = s.grid;
        let cube = MeshKind::Cube as usize;
        // --- bombs
        for b in s.bombs {
            let on = b.is_flash_on(s.time);
            let pos = Vec3::new(b.at.x as f32, floor_at(grid, b.tile.x as f32, b.tile.y as f32), b.at.y as f32);
            let tint = if on { Vec3::ONE } else { lin(0x9a9a9a) };
            let glow = if on { Vec3::splat(0.9) } else { Vec3::ZERO };
            emit_mesh(&mut f.opaque[cube], &self.models.bomb, Mat4::from_translation(pos), tint, 1.0, glow);
            // The fuse sputters: a few sparks jumping off its tip.
            let tip = pos + Vec3::new(0.07, 0.53, 0.0);
            for k in 0..4 {
                let ph = (s.time as f32 * 9.0 + k as f32 * 0.25).fract();
                let a = k as f32 * 1.9 + (s.time as f32 * 3.0).floor();
                let p = tip + Vec3::new(a.cos() * ph * 0.12, ph * 0.12, a.sin() * ph * 0.12);
                f.additive.push(Instance::new(
                    Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.03)),
                    lin(0xffc040) * 1.8,
                    1.0 - ph,
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
        }

        // --- projectiles
        for p in s.projectiles {
            let (px, pz) = (p.pos.x as f32, p.pos.y as f32);
            let pos = Vec3::new(px, floor_at(grid, px, pz) + 0.45, pz);
            let yaw = (p.heading.x as f32).atan2(p.heading.y as f32);
            if self.emit_forged_shot(p.kind, pos, yaw, time, f) {
                continue;
            }
            let group = Mat4::from_translation(pos) * Mat4::from_rotation_y(yaw);
            let color = lin(p.def.color);
            let (mesh, spin, glow) = match p.kind {
                RangedKind::Boomerang => (&self.models.boomerang, 14.0, Vec3::ZERO),
                RangedKind::Fireball => (&self.models.fireball, 6.0, color * 1.2),
                RangedKind::Crescent => (&self.models.crescent, 0.0, color * 0.9),
                _ => (&self.models.arrow, 0.0, Vec3::ZERO),
            };
            let inner = group * Mat4::from_rotation_y(time * spin);
            emit_mesh(&mut f.opaque[cube], mesh, inner, Vec3::ONE, 1.0, glow);
            if p.kind == RangedKind::Fireball {
                f.lights.push(Light { pos: pos + Vec3::new(0.0, 0.15, 0.0), color: color * 5.0, distance: 4.0 });
            }
        }

        // --- slime trails: glossy splats joined tile to tile, bubbling, drying away
        let trail: std::collections::HashSet<P> = s.puddles.iter().map(|(p, _)| p).collect();
        for (tile, life) in s.puddles.iter() {
            emit_goo(f, tile, floor_at(grid, tile.x as f32, tile.y as f32), life as f32 / PUDDLE_SECONDS as f32, time, &trail);
        }

        // --- debris and blasts
        for d in &self.debris {
            let m = Mat4::from_translation(d.pos) * euler(d.rot) * Mat4::from_scale(Vec3::splat(0.16));
            f.opaque[cube].push(Instance::new(m, d.color, 1.0, Vec3::ZERO, SHADE_LIT));
        }
        for b in &self.blasts {
            let t = 1.0 - b.life / b.seconds;
            let m = Mat4::from_translation(b.pos) * Mat4::from_scale(Vec3::splat(b.radius * (1.0 + t * b.scale)));
            f.transparent[MeshKind::Sphere as usize].push(Instance::new(m, b.color, 0.9 * (1.0 - t), Vec3::ZERO, SHADE_UNLIT));
        }

        // --- impact particles
        for p in &self.particles {
            let k = (p.life / p.max).clamp(0.0, 1.0);
            let m = Mat4::from_translation(p.pos) * Mat4::from_scale(Vec3::splat(p.size * k.sqrt().max(0.2)));
            if p.glow {
                f.additive.push(Instance::new(m, p.color, k, Vec3::ZERO, SHADE_UNLIT));
            } else {
                f.opaque[cube].push(Instance::new(m, p.color, 1.0, Vec3::ZERO, SHADE_LIT));
            }
        }

        // --- a spotlight on whatever the tour is showing
        if let Some((target, t, _)) = self.tour {
            let k = (t / 0.6).min(1.0);
            f.lights.push(Light { pos: target + Vec3::new(0.4, 3.0, 0.6), color: lin(0xffe6b8) * 5.0 * k, distance: 6.0 });
        }
    }

    /// The boss, and the level's own life: the meadow, the lava, the vaults' gold or the sea of
    /// cloud, the garden and its fountain, the gate and its keys, smoke and fireflies.
    fn emit_world(&self, s: &SceneState, f: &mut Frame, hero_pos: Vec3) {
        let time = s.time as f32;
        let grid = s.grid;
        // --- the boss and what it throws
        if let Some(b) = s.boss {
            self.emit_boss(b, hero_pos, time, f);
        }
        self.emit_boss_shots(s.boss_shots, hero_pos, time, f);

        let focus_hint = Vec3::new(hero_pos.x, 0.0, hero_pos.z);
        // --- fireflies by night; water, butterflies and pollen by day
        if self.meadow {
            self.emit_meadow(time, f);
        }
        if self.hell {
            // Lava below instead of the sea of cloud.
            f.bounds = Vec4::new(-0.5, -0.5, grid.width as f32 - 0.5, grid.height as f32 - 0.5);
            self.emit_hell(s, time, hero_pos, f);
        } else if self.vault.is_some() {
            // Underground: no sea of cloud, and no fireflies.
            f.bounds = Vec4::new(-0.5, -0.5, grid.width as f32 - 0.5, grid.height as f32 - 0.5);
            self.emit_vault(s, time, hero_pos, f);
        } else if self.run_view.is_some() {
            // A realm after the vaults: its own sky and life, no sea of cloud, no fireflies.
            f.bounds = Vec4::new(-0.5, -0.5, grid.width as f32 - 0.5, grid.height as f32 - 0.5);
            self.emit_run(s, time, hero_pos, f);
        } else {
            emit_cloud_sea(grid, f, self.cam_pos);
        }
        self.emit_garden(time, f);
        self.emit_fountain(time, f);
        self.emit_gate_and_keys(s, time, hero_pos, f);
        self.emit_smoke(time, f);
        let mote_color = lin(MOTE);
        for (i, b) in self.mote_base.iter().enumerate() {
            if self.meadow || self.hell || self.vault.is_some() || self.run_view.is_some() {
                break;
            }
            let fi = i as f32;
            let p = Vec3::new(
                b.x + (time * 0.5 + fi).sin() * 0.4,
                b.y + (time * 0.9 + fi * 0.7).sin() * 0.2,
                b.z + (time * 0.4 + fi * 1.3).cos() * 0.4,
            );
            // Each firefly flashes in its own rhythm: a quick glow, then a long dim spell.
            let phase = (time * (0.35 + (fi * 0.618).fract() * 0.3) + fi * 0.37).fract();
            let flash = (1.0 - (phase * 3.0 - 0.5).abs() * 2.0).max(0.0);
            glow_orb(f, p, 0.04 + 0.015 * flash, mote_color, 0.35 + 0.65 * flash * flash);
            if flash > 0.3 && p.distance_squared(focus_hint) < 144.0 {
                glow_orb(f, p, 0.16 + 0.1 * flash, mote_color, 0.28 * flash);
            }
        }
    }

    /// Light pools, moved to the sources nearest the hero: lanterns and candles (with halos),
    /// remedies, shrines, and whatever effects are burning.
    fn emit_light_pools(&self, f: &mut Frame, time: f32, focus: Vec3, remedy_points: &[Vec3], shrine_points: &[(Vec3, Vec3)]) {
        let torch_points: Vec<Vec3> = self.torches.values().map(|(p, _)| *p).collect();
        for (i, p) in nearest(&torch_points, focus, LIGHT_POOL_TORCHES).into_iter().enumerate() {
            let fi = i as f32;
            let intensity = 4.5 + (time * 3.0 + fi * 2.3).sin() * 0.5 + (time * 7.0 + fi).sin() * 0.3;
            f.lights.push(Light { pos: p + Vec3::new(0.0, 0.5, 0.2), color: lin(TORCH_LIGHT) * intensity, distance: 7.0 });
        }
        for (i, p) in nearest(&self.candles, focus, 5).into_iter().enumerate() {
            let flicker = 1.6 + (time * 9.0 + i as f32 * 1.7).sin() * 0.25 + (time * 23.0 + i as f32).sin() * 0.12;
            f.lights.push(Light { pos: p, color: lin(0xffb060) * flicker, distance: 3.2 });
        }
        // Halos round the lamps and candles nearby, as if lighting the damp air.
        for (i, p) in nearest(&torch_points, focus, 10).into_iter().enumerate() {
            let flick = 0.9 + (time * 3.0 + i as f32 * 2.3).sin() * 0.06;
            glow_orb(f, p + Vec3::new(0.0, 0.42, 0.0), 0.42, lin(TORCH_LIGHT), 0.16 * flick);
        }
        for (i, p) in nearest(&self.candles, focus, 14).into_iter().enumerate() {
            let flick = 0.85 + (time * 9.0 + i as f32 * 1.7).sin() * 0.1;
            glow_orb(f, p, 0.24, lin(0xffb060), 0.2 * flick);
        }
        for p in nearest(remedy_points, focus, LIGHT_POOL_REMEDIES) {
            f.lights.push(Light {
                pos: p + Vec3::new(0.0, 0.45, 0.0),
                color: lin(REMEDY) * (4.0 + (time * 5.0).sin() * 1.5),
                distance: 5.0,
            });
        }
        let shrine_pos: Vec<Vec3> = shrine_points.iter().map(|(p, _)| *p).collect();
        for p in nearest(&shrine_pos, focus, LIGHT_POOL_SHRINES) {
            let color = shrine_points.iter().find(|(q, _)| *q == p).map(|(_, c)| *c).unwrap_or(Vec3::ONE);
            f.lights.push(Light { pos: p + Vec3::new(0.0, 0.9, 0.0), color: color * 4.0, distance: 5.0 });
        }
        for l in &self.effect_lights {
            if l.intensity > 0.0 {
                f.lights.push(Light { pos: l.pos, color: l.color * l.intensity, distance: l.distance });
            }
        }
    }
}

impl WorldView {
    /// What the legends have done to him, shown about him: frost's motes drifting off him, the
    /// omen's dark sigil over his head, venom's bubbles.
    fn emit_ails(&self, ails: &crate::logic::combat::ails::Ails, at: Vec3, time: f32, f: &mut Frame) {
        if ails.frost > 0 {
            for k in 0..4 {
                let t = (time * 0.7 + k as f32 / 4.0).fract();
                let a = k as f32 * TAU / 4.0 + time * 0.8;
                let p = at + Vec3::new(a.cos() * 0.26, 0.3 + t * 1.0, a.sin() * 0.26);
                let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.04 * (1.0 - t * 0.4)));
                f.additive.push(Instance::new(m, lin(0xc8f0ff) * 1.6, 0.8 * (1.0 - t), Vec3::ZERO, SHADE_UNLIT));
            }
        }
        if ails.doom > 0 {
            let m = Mat4::from_translation(at + Vec3::new(0.0, 1.45 + (time * 3.0).sin() * 0.04, 0.0))
                * Mat4::from_rotation_y(time * 2.0)
                * Mat4::from_rotation_z(PI / 4.0)
                * Mat4::from_scale(Vec3::splat(0.11));
            f.additive.push(Instance::new(m, lin(0x9a5aff) * 2.0, 0.85, Vec3::ZERO, SHADE_UNLIT));
        }
        if ails.venom > 0 {
            for k in 0..3 {
                let t = (time * 0.9 + k as f32 / 3.0).fract();
                let p = at + Vec3::new((k as f32 * 2.1).cos() * 0.2, 0.5 + t * 0.9, (k as f32 * 2.1).sin() * 0.2);
                let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.045 * (1.0 - t * 0.5)));
                f.additive.push(Instance::new(m, lin(0x8ad04a) * 1.8, 0.8 * (1.0 - t), Vec3::ZERO, SHADE_UNLIT));
            }
        }
    }
}

/// The hero's rig: the tint falls on all of him, and `rust` (0 to 1) reddens the metal voxels of
/// his blades (the "blade" and "offhand" parts).
#[allow(clippy::too_many_arguments)]
fn emit_hero_rig(out: &mut RigList, rig: &Rig, pose: &Pose, root: Mat4, tint: Vec3, alpha: f32, emissive: Vec3, rust: f32) {
    out.push(rig, pose, root, tint, alpha, emissive, rig_look::TINT_ALL, rust);
}
