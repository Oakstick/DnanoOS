//! The Burning Field as the view draws it (Wrath's realm; laid out in `logic::realms::field`).
//!
//! A lava sea under an ash sky. The slabs of scorched earth that drift on it are drawn every frame
//! where the drift has carried them (those near him): their tops of churned earth, ash, blood-mud
//! and obsidian glass where the ground melted, their sides down into the lava glowing at the
//! waterline, the blocks planted in them (`decor`) and the litter of the field; the cracks of the
//! blows they have taken glow with lava, and a piece going down tilts and sinks. The isles that
//! never drift are part of the static scene: the war camp he arrives in, with its tents and fire;
//! the herald's isle with its braziers, the burnt gallows-tree and the raven-headed herald; the
//! hill's terrace with the truce stone; and behind it Amon's banner-hill, his great banner on it.
//!
//! The lava glows from below and lights the field; the ash fog is short. Every flash of lightning
//! whitens the sky and the sun for a moment, outlines every slab's edges in white and throws true
//! shadows (the only moment the field reads true), and a bolt strikes down; a ghost army wheels in
//! the storm overhead, seen faintly, and clearly in the flash. The slain lie where they fell,
//! glowing as the next flash nears. In the horn's call the lava rises; the truce stone's ring
//! counts out the seconds he holds, and opens in a column of white light.

mod decor;

use super::super::vault_decor::{merge, push_merged, Merged};
use super::super::*;
use crate::logic::realms::field::{Battlefield, Block, Ground, Slab, SlabKind, FLASH_SECONDS, TRUCE_RADIUS, TRUCE_SECONDS};
use decor::pal::*;
use decor::{bx, plain, rnd, SH_METAL, SH_WOOD};

/// How brightly his lantern burns here: the lava lights him.
pub(super) const LANTERN: f32 = 0.6;
/// The lava's surface (the low shelf's top is at 0).
const LAVA_Y: f32 = -0.42;
/// Slabs are drawn this near him (tiles, from their edges).
const RANGE: f32 = 22.0;
/// The voxel of the herald and the horn.
const HERALD_VOXEL: f32 = 0.042;
const HORN_VOXEL: f32 = 0.047;

/// What the view keeps of the realm: every slab's boxes (in its own frame), the horn's model and
/// where its cord hangs, and the fires burning on the isles.
pub(in crate::render::view) struct FieldView {
    slabs: HashMap<u32, SlabArt>,
    horn: Merged,
    cord: Vec3,
    fires: Vec<Vec3>,
}

/// A slab's boxes, about its local tile (0, 0) with its top at y = 0, and the spots that smoulder
/// on it; built for its shape (`key`: id, size, blocks), rebuilt when that changes.
struct SlabArt {
    key: (u32, i32, i32, usize),
    cubes: Vec<Instance>,
    fires: Vec<Vec3>,
}

fn slab_key(s: &Slab) -> (u32, i32, i32, usize) {
    (s.gen, s.w, s.h, s.blocks.len())
}

/// Smooth value noise, 0 to 1.
fn vnoise(x: f32, z: f32) -> f32 {
    let (xi, zi) = (x.floor(), z.floor());
    let (fx, fz) = (x - xi, z - zi);
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let h = |a: f32, b: f32| hash(a as f64 * 1.37 + 5.0, b as f64 * 1.71 + 9.0) as f32;
    let (a, b, c, d) = (h(xi, zi), h(xi + 1.0, zi), h(xi, zi + 1.0), h(xi + 1.0, zi + 1.0));
    let (u, v) = (s(fx), s(fz));
    a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v
}

/// The lava's colour and glow at a point: rivers of heat between cooling crust.
fn lava_look(x: f32, z: f32) -> (Vec3, Vec3) {
    // Pools of heat, and between them a crust broken by thin glowing veins.
    let heat = 0.62 * vnoise(x * 0.13, z * 0.13) + 0.38 * vnoise(x * 0.37 + 7.0, z * 0.37 + 3.0);
    let vein = 1.0 - ((vnoise(x * 0.45 + 3.0, z * 0.45 - 2.0) - 0.5).abs() * 12.0).min(1.0);
    // (More of it crusted over than not, as the Portals' lava: rivers of heat, not a sea of it.)
    let hot = ((heat - 0.58) / 0.22).clamp(0.0, 1.0).max(vein * 0.5);
    let color = lin(CRUST).lerp(lin(LAVA_DEEP), hot * hot);
    let glow = lin(LAVA).lerp(lin(LAVA_HOT), ((hot - 0.85) / 0.15).max(0.0)) * (hot * hot * 0.16 + 0.02);
    (color, glow)
}

fn glowing(out: &mut Vec<Instance>, at: Vec3, size: Vec3, color: Vec3, glow: Vec3) {
    out.push(Instance::new(Mat4::from_scale_rotation_translation(size, Quat::IDENTITY, at), color, 1.0, glow, SHADE_LIT));
}

/// One tile of scorched earth, its top at `at`: churned earth, a drift of ash, glossy blood-mud,
/// or black obsidian glass where the ground melted (a red glint in its cracks).
fn earth(out: &mut Vec<Instance>, root: Mat4, at: Vec3, seed: f64) {
    let h = hash(seed * 1.7 + 3.0, seed * 0.31 + 1.0);
    let k = (hash(seed, seed * 2.0) * 4.0) as usize % 4;
    let jitter = (hash(seed * 3.1, 7.0) as f32 - 0.5) * 0.05;
    let top = at + Vec3::new(0.0, jitter, 0.0);
    let base = lin(EARTH[k]) * (0.93 + h as f32 * 0.1);
    let block = |out: &mut Vec<Instance>, c: Vec3, shade: f32| {
        bx(out, root, top - Vec3::new(0.0, 0.09, 0.0), Vec3::new(1.0, 0.18, 1.0), Quat::IDENTITY, c, Vec3::ZERO, shade);
    };
    if h < 0.05 {
        // Obsidian glass.
        block(out, lin(OBSIDIAN), SHADE_LIT);
        let a = (hash(seed, 9.0) as f32) * PI;
        bx(
            out,
            root,
            top + Vec3::new(0.0, 0.004, 0.0),
            Vec3::new(0.55, 0.01, 0.025),
            Quat::from_rotation_y(a),
            lin(LAVA_DEEP),
            lin(LAVA) * 0.9,
            SHADE_LIT,
        );
        for j in 0..3 {
            let o = Vec3::new((hash(seed, 20.0 + j as f64) as f32 - 0.5) * 0.6, 0.008, (hash(seed, 30.0 + j as f64) as f32 - 0.5) * 0.6);
            bx(
                out,
                root,
                top + o,
                Vec3::new(0.18, 0.016, 0.12),
                Quat::from_rotation_y(a + j as f32 * 1.3),
                lin(OBSIDIAN_L),
                lin(0x6a3a8a) * 0.12,
                SHADE_LIT,
            );
        }
    } else if h < 0.12 {
        // Blood-mud.
        block(out, base * 0.8, SHADE_LIT);
        let a = (hash(seed, 4.0) as f32) * PI;
        bx(
            out,
            root,
            top + Vec3::new(0.0, 0.004, 0.0),
            Vec3::new(0.7, 0.012, 0.55),
            Quat::from_rotation_y(a),
            lin(MUD),
            lin(MUD) * 0.1,
            SHADE_LIT,
        );
        bx(
            out,
            root,
            top + Vec3::new(0.1, 0.008, 0.05),
            Vec3::new(0.3, 0.012, 0.25),
            Quat::from_rotation_y(a * 1.3),
            lin(MUD_L),
            lin(MUD_L) * 0.08,
            SHADE_LIT,
        );
    } else if h < 0.32 {
        // A drift of ash.
        let a = (hash(seed, 5.0) as f32) * PI;
        // (A grey cast over the tile, and grains of it.)
        let c = base.lerp(lin(ASH), 0.22 + (hash(seed, 6.0) as f32) * 0.12);
        block(out, c, SHADE_LIT);
        for j in 0..5 {
            let o = Vec3::new((hash(seed, 7.0 + j as f64) as f32 - 0.5) * 0.8, 0.012, (hash(seed, 8.0 + j as f64) as f32 - 0.5) * 0.8);
            bx(
                out,
                root,
                top + o,
                Vec3::splat(0.07),
                Quat::from_rotation_y(a + j as f32),
                lin(ASH) * (0.8 + j as f32 * 0.08),
                Vec3::ZERO,
                SHADE_LIT,
            );
        }
    } else {
        block(out, base, SHADE_LIT);
        let a = (hash(seed, 12.0) as f32) * PI;
        if h < 0.45 {
            // A scorch mark, black at its heart.
            bx(
                out,
                root,
                top + Vec3::new(0.05, 0.004, -0.04),
                Vec3::new(0.62, 0.01, 0.46),
                Quat::from_rotation_y(a),
                base.lerp(lin(CHAR), 0.55),
                Vec3::ZERO,
                SHADE_LIT,
            );
            bx(
                out,
                root,
                top + Vec3::new(0.0, 0.008, 0.0),
                Vec3::new(0.32, 0.012, 0.24),
                Quat::from_rotation_y(a + 0.7),
                lin(CHAR),
                Vec3::ZERO,
                SHADE_LIT,
            );
        } else if h < 0.55 {
            // Spatters of blood.
            for c in 0..4 {
                let o =
                    Vec3::new((hash(seed + c as f64, 13.0) as f32 - 0.5) * 0.8, 0.005, (hash(seed - c as f64, 14.0) as f32 - 0.5) * 0.8);
                let w = 0.05 + (hash(seed, 15.0 + c as f64) as f32) * 0.08;
                bx(
                    out,
                    root,
                    top + o,
                    Vec3::new(w, 0.008, w * 0.8),
                    Quat::from_rotation_y(a + c as f32),
                    lin(MUD_L),
                    Vec3::ZERO,
                    SHADE_LIT,
                );
            }
        } else if h < 0.68 {
            // Stones half sunk in the earth.
            for c in 0..2 {
                let o = Vec3::new((hash(seed + c as f64, 16.0) as f32 - 0.5) * 0.7, 0.02, (hash(seed - c as f64, 17.0) as f32 - 0.5) * 0.7);
                let w = 0.1 + (hash(seed, 18.0 + c as f64) as f32) * 0.1;
                bx(
                    out,
                    root,
                    top + o,
                    Vec3::new(w, 0.06, w * 0.8),
                    Quat::from_rotation_y(a * 2.0 + c as f32),
                    lin(0x5a5450),
                    Vec3::ZERO,
                    SHADE_LIT,
                );
            }
        }
        if h > 0.8 {
            // Clods thrown up by hooves and feet.
            for c in 0..3 {
                let ox = (hash(seed + c as f64, 1.0) as f32 - 0.5) * 0.7;
                let oz = (hash(seed - c as f64, 2.0) as f32 - 0.5) * 0.7;
                bx(
                    out,
                    root,
                    top + Vec3::new(ox, 0.03, oz),
                    Vec3::new(0.1, 0.07, 0.09),
                    Quat::from_rotation_y(ox * 5.0),
                    lin(EARTH_D),
                    Vec3::ZERO,
                    SHADE_LIT,
                );
            }
        }
    }
}

/// The body of a slab or an isle of `w` by `h` tiles (local tile (0, 0) at the origin, its top at
/// 0) down into the lava `lava` below its top: earth over rock in strata, rocks jutting from its
/// sides, and the lava licking at it in a glowing line.
fn body(out: &mut Vec<Instance>, root: Mat4, w: i32, h: i32, lava: f32, seed: f64) {
    let (cx, cz) = ((w - 1) as f32 / 2.0, (h - 1) as f32 / 2.0);
    let (fw, fh) = (w as f32, h as f32);
    let deep = lava - 0.6;
    // Earth over rock: the soil the top was churned from, then darker rock down into the lava.
    let soil = (-0.55f32).max(lava + 0.05);
    plain(out, root, Vec3::new(cx, (-0.18 + soil) / 2.0, cz), Vec3::new(fw - 0.02, -0.18 - soil, fh - 0.02), 0x5a4232);
    plain(out, root, Vec3::new(cx, (soil + deep) / 2.0, cz), Vec3::new(fw - 0.03, soil - deep, fh - 0.03), 0x3a2c26);
    // Strata: a band of paler earth, a band of ash, a band of dark rock.
    let band = |out: &mut Vec<Instance>, y: f32, t: f32, grow: f32, c: u32| {
        plain(out, root, Vec3::new(cx, y, cz), Vec3::new(fw + grow, t, fh + grow), c);
    };
    band(out, -0.27, 0.09, 0.02, 0x6a5038);
    band(out, -0.4, 0.04, 0.015, 0x6a625e);
    if lava < -0.7 {
        band(out, (lava - 0.26) * 0.55, 0.16, 0.04, 0x2a2220);
    }
    // Rocks jutting along every side.
    let mut perim: Vec<(f32, f32, f32, f32)> = Vec::new();
    for x in 0..w {
        perim.push((x as f32, -0.5, 0.0, -1.0));
        perim.push((x as f32, fh - 0.5, 0.0, 1.0));
    }
    for z in 0..h {
        perim.push((-0.5, z as f32, -1.0, 0.0));
        perim.push((fw - 0.5, z as f32, 1.0, 0.0));
    }
    for (k, &(x, z, nx, nz)) in perim.iter().enumerate() {
        let r = |j: u32| rnd(seed + k as f64 * 0.37, j);
        if r(0) < 0.3 {
            continue;
        }
        if r(0) > 0.94 {
            // Something of the battle buried in the bank: a skull, or a sword's hilt.
            let at = Vec3::new(x + nx * 0.02, -0.3 - r(1) * 0.25, z + nz * 0.02);
            if r(2) < 0.5 {
                decor::skull(out, root, at, nz.atan2(nx) - PI / 2.0 + PI, 0.9);
            } else {
                let q = Quat::from_rotation_arc(Vec3::Y, Vec3::new(nx, 0.3, nz).normalize());
                bx(out, root, at, Vec3::new(0.04, 0.28, 0.04), q, lin(STEEL_D), Vec3::ZERO, SH_METAL);
                bx(out, root, at + q * Vec3::new(0.0, 0.1, 0.0), Vec3::new(0.16, 0.03, 0.04), q, lin(RUST), Vec3::ZERO, SH_METAL);
            }
            continue;
        }
        let tall = 0.2 + r(1) * (-lava).min(1.4) * 0.6;
        let y = -0.12 - r(2) * (-lava - tall).max(0.0) - tall / 2.0;
        let along = 0.35 + r(3) * 0.5;
        let size = if nx != 0.0 { Vec3::new(0.16, tall, along) } else { Vec3::new(along, tall, 0.16) };
        let c = lin([0x4a3a30, 0x3e322a, 0x5a4a3e, 0x34302c][(r(4) * 4.0) as usize % 4]) * (0.85 + r(5) * 0.3);
        bx(
            out,
            root,
            Vec3::new(x + nx * 0.03, y, z + nz * 0.03),
            size,
            Quat::from_rotation_y((r(6) - 0.5) * 0.3),
            c,
            Vec3::ZERO,
            SHADE_LIT,
        );
        // The crumbled lip of the top.
        if r(7) < 0.4 {
            bx(
                out,
                root,
                Vec3::new(x - nx * 0.12, 0.02, z - nz * 0.12),
                Vec3::new(0.18, 0.06, 0.16),
                Quat::from_rotation_y(r(1) * 3.0),
                lin(EARTH_D),
                Vec3::ZERO,
                SHADE_LIT,
            );
        }
    }
    // The lava licking at the waterline, all round.
    let wl = lava + 0.04;
    for (at, size) in [
        (Vec3::new(cx, wl, -0.53), Vec3::new(fw + 0.06, 0.07, 0.04)),
        (Vec3::new(cx, wl, fh - 0.47), Vec3::new(fw + 0.06, 0.07, 0.04)),
        (Vec3::new(-0.53, wl, cz), Vec3::new(0.04, 0.07, fh + 0.06)),
        (Vec3::new(fw - 0.47, wl, cz), Vec3::new(0.04, 0.07, fh + 0.06)),
    ] {
        bx(out, root, at, size, Quat::IDENTITY, lin(LAVA_DEEP), lin(LAVA) * 1.3, SHADE_LIT);
    }
    // Veins of heat up the sides.
    for k in 0..((w + h) / 4) {
        let r = |j: u32| rnd(seed - 50.0 - k as f64, j);
        let (x, z, size) = match k % 4 {
            0 => (r(0) * (fw - 1.0), -0.52, Vec3::new(0.05, -lava * 0.6, 0.02)),
            1 => (r(0) * (fw - 1.0), fh - 0.48, Vec3::new(0.05, -lava * 0.6, 0.02)),
            2 => (-0.52, r(0) * (fh - 1.0), Vec3::new(0.02, -lava * 0.6, 0.05)),
            _ => (fw - 0.48, r(0) * (fh - 1.0), Vec3::new(0.02, -lava * 0.6, 0.05)),
        };
        bx(out, root, Vec3::new(x, lava * 0.7, z), size, Quat::IDENTITY, lin(LAVA_DEEP), lin(LAVA) * 0.8, SHADE_LIT);
    }
}

/// The boxes of slab `s`, in its own frame.
fn build_slab(s: &Slab) -> SlabArt {
    let mut cubes = Vec::new();
    let mut fires = Vec::new();
    let top = s.elevation() as f32 * STEP;
    let lava = LAVA_Y - top;
    // (Seeded by where its tiles lay at home, so a piece keeps its look when the slab splits.)
    let (hx, hz) = (s.home.x.round() as i32, s.home.y.round() as i32);
    let id = Mat4::IDENTITY;
    for z in 0..s.h {
        for x in 0..s.w {
            let seed = ((hx + x) * 131 + (hz + z) * 71) as f64;
            earth(&mut cubes, id, Vec3::new(x as f32, 0.0, z as f32), seed);
            let l = P::new(x, z);
            if !s.blocked(l) && hash(seed * 0.7, 5.5) < 0.13 {
                let root = Mat4::from_translation(Vec3::new(x as f32, 0.0, z as f32));
                if let Some(fire) = decor::litter(&mut cubes, root, seed) {
                    fires.push(fire + Vec3::new(x as f32, 0.0, z as f32));
                }
            }
        }
    }
    body(&mut cubes, id, s.w, s.h, lava, (hx * 17 + hz * 29) as f64);
    for &(at, b) in &s.blocks {
        let seed = ((hx + at.x) * 37 + (hz + at.y) * 53) as f64;
        let base = Vec3::new(at.x as f32, 0.0, at.y as f32);
        match b {
            Block::Blades => decor::blades(&mut cubes, Mat4::from_translation(base), seed),
            Block::Palisade => decor::palisade(&mut cubes, Mat4::from_translation(base), seed, at.y == 0 || at.y == s.h - 1),
            Block::Tower => decor::tower(&mut cubes, Mat4::from_translation(base + Vec3::new(0.5, 0.0, 0.5)), seed),
            Block::Wagon => decor::wagon(&mut cubes, Mat4::from_translation(base + Vec3::new(0.5, 0.0, 0.0)), seed),
            Block::Banner => decor::banner(&mut cubes, Mat4::from_translation(base), seed),
        }
    }
    SlabArt { key: slab_key(s), cubes, fires }
}

/// Where slab `s` stands this frame: its drift, a slight bob, and (going down or coming up) its
/// sinking and tilt.
fn slab_root(s: &Slab, time: f32) -> Mat4 {
    let top = s.elevation() as f32 * STEP;
    let depth = top - LAVA_Y + 1.2;
    let bob = (time * 0.8 + s.id as f32 * 1.7).sin() * 0.012;
    let mut drop = (1.0 - s.risen as f32).powi(2) * depth;
    let mut tilt = Quat::IDENTITY;
    if let Some(k) = s.sinking {
        let p = k.progress() as f32;
        let slow = s.kind == SlabKind::Moat(4);
        drop += if slow { p.powi(3) } else { p * p } * depth;
        let lean = if slow { p.powi(3) * 0.12 } else { p * 0.2 };
        tilt = if s.part.x != 0.0 {
            Quat::from_rotation_z(-lean * s.part.x.signum() as f32)
        } else if s.part.y != 0.0 {
            Quat::from_rotation_x(lean * s.part.y.signum() as f32)
        } else {
            Quat::from_rotation_x(lean)
        };
    }
    let centre = Vec3::new((s.w - 1) as f32 / 2.0, 0.0, (s.h - 1) as f32 / 2.0);
    Mat4::from_translation(Vec3::new(s.origin.x as f32, top + bob - drop, s.origin.y as f32))
        * Mat4::from_translation(centre)
        * Mat4::from_quat(tilt)
        * Mat4::from_translation(-centre)
}

impl WorldView {
    /// The realm's static geometry: the lava sea, the isles and what stands on them, the hill and
    /// its banner, wrecks far out in the lava; and every slab's boxes, to be drawn where it drifts.
    pub(super) fn build_field(&mut self, grid: &Grid, r: &Battlefield) -> FieldView {
        self.build_field_lava(grid);
        let mut fires = Vec::new();
        for (rect, e) in [(r.camp, 0), (r.isle, 1), (r.terrace, 0)] {
            let root = Mat4::from_translation(Vec3::new(rect.x0 as f32, e as f32 * STEP, rect.y0 as f32));
            let (w, h) = (rect.x1 - rect.x0 + 1, rect.y1 - rect.y0 + 1);
            for p in rect.tiles() {
                let seed = (p.x * 131 + p.y * 71) as f64;
                earth(&mut self.static_cubes, root, Vec3::new((p.x - rect.x0) as f32, 0.0, (p.y - rect.y0) as f32), seed);
            }
            body(&mut self.static_cubes, root, w, h, LAVA_Y - e as f32 * STEP, (rect.x0 * 7 + rect.y0) as f64);
        }
        self.build_field_camp(r, &mut fires);
        let cord = self.build_field_isle(r, &mut fires);
        self.build_field_hill(r, &mut fires);
        self.build_field_far();
        let horn = merge(&decor::horn(), HORN_VOXEL);
        let slabs = r.slabs.iter().map(|s| (s.id, build_slab(s))).collect();
        FieldView { slabs, horn, cord, fires }
    }

    /// Lava under and round everything: a block a tile under the field and a margin round it, and
    /// coarser out to the fog.
    fn build_field_lava(&mut self, grid: &Grid) {
        let out = &mut self.static_cubes;
        let (gw, gh) = (grid.width as f32, grid.height as f32);
        for (step, reach, inner) in [(0.5f32, 6.0f32, -1.0f32), (1.0, 16.0, 6.0), (3.0, 46.0, 16.0)] {
            let mut z = -reach;
            while z <= gh - 1.0 + reach {
                let mut x = -reach;
                while x <= gw - 1.0 + reach {
                    let inside_inner = inner >= 0.0 && x > -inner - 0.5 && x < gw - 0.5 + inner && z > -inner - 0.5 && z < gh - 0.5 + inner;
                    if !inside_inner {
                        let (c, e) = lava_look(x, z);
                        let lift = if e.x < 0.3 { 0.025 } else { 0.0 };
                        glowing(out, Vec3::new(x, LAVA_Y - 0.06 + lift, z), Vec3::new(step, 0.12, step), c, e);
                    }
                    x += step;
                }
                z += step;
            }
        }
    }

    /// The war camp he arrives in: tents of grey and red cloth, a fire, racks of spears, banners,
    /// stakes at the corners, a palisade's stumps along its edges.
    fn build_field_camp(&mut self, r: &Battlefield, fires: &mut Vec<Vec3>) {
        let c = r.camp;
        let out = &mut self.static_cubes;
        let at = |x: i32, z: i32| Mat4::from_translation(Vec3::new(x as f32, 0.0, z as f32));
        // Tents: an A-frame of cloth over a ridge-pole.
        for (k, &(x, z, yaw)) in
            [(c.x0 + 2, c.y0 + 3, 0.2f32), (c.x1 - 2, c.y0 + 2, -0.3), (c.x0 + 3, c.y1 - 3, 1.5), (c.x1 - 3, c.y1 - 4, 1.4)]
                .iter()
                .enumerate()
        {
            let root = at(x, z) * Mat4::from_rotation_y(yaw);
            let cloth = if k % 2 == 0 { 0x8a8478 } else { RED_D };
            for s in [-1.0f32, 1.0] {
                bx(
                    out,
                    root,
                    Vec3::new(s * 0.27, 0.36, 0.0),
                    Vec3::new(0.05, 0.86, 1.25),
                    Quat::from_rotation_z(s * 0.62),
                    lin(cloth),
                    Vec3::ZERO,
                    SHADE_LIT,
                );
                // A hem burnt black, and a patch.
                bx(
                    out,
                    root,
                    Vec3::new(s * 0.5, 0.04, 0.0),
                    Vec3::new(0.06, 0.08, 1.26),
                    Quat::from_rotation_z(s * 0.62),
                    lin(CHAR),
                    Vec3::ZERO,
                    SHADE_LIT,
                );
                bx(
                    out,
                    root,
                    Vec3::new(s * 0.3, 0.38, 0.25),
                    Vec3::new(0.055, 0.25, 0.3),
                    Quat::from_rotation_z(s * 0.62),
                    lin(cloth) * 0.7,
                    Vec3::ZERO,
                    SHADE_LIT,
                );
            }
            plain(out, root, Vec3::new(0.0, 0.72, 0.0), Vec3::new(0.05, 0.05, 1.45), WOOD);
            for zz in [-0.66f32, 0.66] {
                plain(out, root, Vec3::new(0.0, 0.37, zz), Vec3::new(0.05, 0.74, 0.05), WOOD_C);
            }
            plain(out, root, Vec3::new(0.0, 0.22, 0.63), Vec3::new(0.3, 0.42, 0.02), CHAR);
        }
        // The camp fire, ringed with stones.
        let (fx, fz) = ((c.x0 + c.x1) / 2 - 3, c.y0 + 5);
        let fire = at(fx, fz);
        for k in 0..8 {
            let a = k as f32 / 8.0 * TAU;
            plain(out, fire, Vec3::new(a.cos() * 0.38, 0.06, a.sin() * 0.38), Vec3::new(0.16, 0.12, 0.16), 0x4a4440);
        }
        for k in 0..3 {
            bx(
                out,
                fire,
                Vec3::new(0.0, 0.08, 0.0),
                Vec3::new(0.55, 0.08, 0.09),
                Quat::from_rotation_y(k as f32),
                lin(CHAR),
                Vec3::ZERO,
                SH_WOOD,
            );
        }
        bx(out, fire, Vec3::new(0.0, 0.14, 0.0), Vec3::new(0.25, 0.1, 0.25), Quat::IDENTITY, lin(EMBER), lin(EMBER) * 2.5, SHADE_LIT);
        fires.push(Vec3::new(fx as f32, 0.5, fz as f32));
        // Racks of spears, banners, and the litter of a camp.
        for &(x, z) in &[(c.x0 + 6, c.y0 + 1), (c.x1 - 6, c.y1 - 1)] {
            let root = at(x, z);
            plain(out, root, Vec3::new(0.0, 0.7, 0.0), Vec3::new(1.0, 0.06, 0.06), WOOD);
            for s in [-0.45f32, 0.45] {
                plain(out, root, Vec3::new(s, 0.4, 0.0), Vec3::new(0.06, 0.8, 0.06), WOOD_C);
            }
            for k in 0..6 {
                let rot = Quat::from_rotation_x(0.2);
                bx(
                    out,
                    root,
                    Vec3::new(-0.4 + k as f32 * 0.16, 0.75, 0.05),
                    Vec3::new(0.03, 1.5, 0.03),
                    rot,
                    lin(WOOD_L),
                    Vec3::ZERO,
                    SH_WOOD,
                );
                bx(
                    out,
                    root,
                    Vec3::new(-0.4 + k as f32 * 0.16, 1.55, 0.2),
                    Vec3::new(0.05, 0.16, 0.015),
                    rot,
                    lin(STEEL),
                    Vec3::ZERO,
                    SH_METAL,
                );
            }
        }
        for &(x, z) in &[(c.x0 + 1, c.y0), (c.x1 - 1, c.y0), (c.x0 + 8, c.y1 - 2)] {
            decor::banner(out, at(x, z), (x * 3 + z) as f64);
        }
        for p in [P::new(c.x0, c.y0 + 2), P::new(c.x1, c.y0 + 4)] {
            decor::palisade(out, at(p.x, p.y), (p.x + p.y) as f64, false);
        }
        for k in 0..12 {
            let x = c.x0 + 1 + (hash(k as f64 * 3.3, 1.0) * (c.x1 - c.x0 - 1) as f64) as i32;
            let z = c.y0 + 1 + (hash(k as f64 * 1.7, 2.0) * (c.y1 - c.y0 - 2) as f64) as i32;
            if let Some(f) = decor::litter(out, at(x, z), (x * 11 + z * 13) as f64 + 0.5) {
                fires.push(f + Vec3::new(x as f32, 0.0, z as f32));
            }
        }
    }

    /// The herald's isle: the gallows-tree and its noose, the herald beside it, braziers at its
    /// corners, a ring of shields planted by the horn, banners. Returns where the horn's cord hangs.
    fn build_field_isle(&mut self, r: &Battlefield, fires: &mut Vec<Vec3>) -> Vec3 {
        let i = r.isle;
        let top = STEP;
        let out = &mut self.static_cubes;
        let at = |x: f32, z: f32| Mat4::from_translation(Vec3::new(x, top, z));
        let tree = Vec3::new(r.horn.x as f32, top, r.horn.y as f32 - 0.1);
        let cord = decor::gallows(out, Mat4::from_translation(tree));
        let herald = merge(&decor::herald(), HERALD_VOXEL);
        // (Turned toward the horn, so that his raven's beak shows from the south.)
        push_merged(out, &herald, at(r.herald.x as f32, r.herald.y as f32) * Mat4::from_rotation_y(-PI / 2.0 + 0.45), 0.0);
        // Braziers: an iron bowl of fire on a tripod.
        for (x, z) in [(i.x0 + 1, i.y0 + 1), (i.x1 - 1, i.y0 + 1), (i.x0 + 1, i.y1 - 1), (i.x1 - 1, i.y1 - 1)] {
            let root = at(x as f32, z as f32);
            for k in 0..3 {
                let a = k as f32 / 3.0 * TAU;
                let rot = Quat::from_rotation_y(-a) * Quat::from_rotation_z(0.25);
                bx(
                    out,
                    root,
                    Vec3::new(a.cos() * 0.12, 0.42, a.sin() * 0.12),
                    Vec3::new(0.04, 0.9, 0.04),
                    rot,
                    lin(0x2a2a2e),
                    Vec3::ZERO,
                    SH_METAL,
                );
            }
            plain(out, root, Vec3::new(0.0, 0.88, 0.0), Vec3::new(0.42, 0.12, 0.42), 0x2a2a2e);
            bx(out, root, Vec3::new(0.0, 0.97, 0.0), Vec3::new(0.32, 0.08, 0.32), Quat::IDENTITY, lin(EMBER), lin(EMBER) * 2.6, SHADE_LIT);
            fires.push(Vec3::new(x as f32, top + 1.3, z as f32));
        }
        // Shields planted in a ring before the horn, and banners.
        for k in 0..9 {
            let a = PI * 0.15 + k as f32 / 8.0 * PI * 0.7;
            let p = Vec3::new(r.horn.x as f32 + a.cos() * 2.6, top + 0.2, r.horn.y as f32 + 0.6 + a.sin() * 2.2);
            if (p.x - r.herald.x as f32).abs() < 0.8 && (p.z - r.herald.y as f32).abs() < 0.8 {
                continue;
            }
            decor::shield(out, Mat4::IDENTITY, p, Quat::from_rotation_y(-a + PI / 2.0) * Quat::from_rotation_x(-0.25), k as f64 * 3.1);
        }
        for (x, z) in [(i.x0 + 3, i.y0), (i.x1 - 3, i.y0), (i.x0 + 6, i.y1)] {
            decor::banner(out, at(x as f32, z as f32), (x * 5 + z) as f64);
        }
        for k in 0..10 {
            let x = i.x0 + 1 + (hash(k as f64 * 2.9, 4.0) * (i.x1 - i.x0 - 1) as f64) as i32;
            let z = i.y0 + 2 + (hash(k as f64 * 1.3, 6.0) * (i.y1 - i.y0 - 3) as f64) as i32;
            if (x - r.rest.x).abs() + (z - r.rest.y).abs() <= 1 {
                continue;
            }
            decor::litter(out, at(x as f32, z as f32), (x * 17 + z * 5) as f64 + 0.25);
        }
        cord
    }

    /// The hill's terrace (the truce stone on it, braziers, paving round the stone) and Amon's
    /// banner-hill rising behind it: a mound of earth, ash and bones, broken blades planted on its
    /// slopes, burnt trees, and his great banner at the top.
    fn build_field_hill(&mut self, r: &Battlefield, fires: &mut Vec<Vec3>) {
        let t = r.terrace;
        let out = &mut self.static_cubes;
        let stone = Vec3::new(r.stone.x as f32, 0.0, r.stone.y as f32);
        decor::truce_stone(out, Mat4::from_translation(stone));
        for k in 0..20 {
            let a = k as f32 / 20.0 * TAU;
            let p = stone + Vec3::new(a.cos() * 1.6, 0.012, a.sin() * 1.3);
            bx(out, Mat4::IDENTITY, p, Vec3::new(0.5, 0.025, 0.36), Quat::from_rotation_y(-a), lin(STONE_D) * 0.35, Vec3::ZERO, SHADE_LIT);
        }
        for (x, z) in [(t.x0 + 1, t.y1 - 1), (t.x1 - 1, t.y1 - 1)] {
            let root = Mat4::from_translation(Vec3::new(x as f32, 0.0, z as f32));
            plain(out, root, Vec3::new(0.0, 0.45, 0.0), Vec3::new(0.14, 0.9, 0.14), 0x2a2a2e);
            plain(out, root, Vec3::new(0.0, 0.92, 0.0), Vec3::new(0.44, 0.12, 0.44), 0x2a2a2e);
            // (Unlit: the portal's own braziers light the terrace.)
            bx(out, root, Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.32, 0.08, 0.32), Quat::IDENTITY, lin(EMBER), lin(EMBER) * 2.6, SHADE_LIT);
        }
        for p in [P::new(t.x0, t.y0), P::new(t.x1, t.y0)] {
            decor::banner(out, Mat4::from_translation(Vec3::new(p.x as f32, 0.0, p.y as f32)), (p.x * 3) as f64);
        }
        // The mound, a column of earth a tile, highest behind the portal.
        let gate = r.base.exit.tile;
        let peak = Vec3::new(gate.x as f32, 0.0, -3.0);
        for z in -16..t.y0 {
            for x in -6..66 {
                if (x - gate.x).abs() <= 1 && z >= gate.y - 1 {
                    continue; // (the portal's own)
                }
                let (dx, dz) = ((x as f32 - peak.x) / 24.0, (z as f32 - peak.z) / 11.0);
                let d = (dx * dx + dz * dz).sqrt();
                let n = vnoise(x as f32 * 0.4, z as f32 * 0.4);
                let mut hgt = (1.0 - d).max(0.0).powf(0.7) * 6.2 + n * 0.8 - 0.3;
                if z >= gate.y - 1 {
                    hgt = hgt.max(2.6 + n * 0.5);
                }
                if hgt <= 0.0 {
                    continue;
                }
                let seed = (x * 131 + z * 71) as f64;
                let k = (hash(seed, 2.0) * 4.0) as usize % 4;
                let c = if hgt > 4.8 { lin(ASH_D) } else { lin(EARTH[k]) * (0.8 + n * 0.3) };
                let foot = LAVA_Y - 0.4;
                bx(
                    out,
                    Mat4::IDENTITY,
                    Vec3::new(x as f32, (hgt + foot) / 2.0, z as f32),
                    Vec3::new(1.0, hgt - foot, 1.0),
                    Quat::IDENTITY,
                    c,
                    Vec3::ZERO,
                    SHADE_LIT,
                );
                let on = Mat4::from_translation(Vec3::new(x as f32, hgt, z as f32));
                let pick = hash(seed, 9.0);
                if pick < 0.07 {
                    decor::blades(out, on, seed);
                } else if pick < 0.14 {
                    decor::litter(out, on, seed);
                } else if pick < 0.16 {
                    // A burnt tree, bare and split.
                    for s in 0..3 {
                        let rot = Quat::from_euler(EulerRot::XYZ, (s as f32 - 1.0) * 0.5, s as f32 * 2.0, 0.3 * (s as f32 - 1.0));
                        bx(
                            out,
                            on,
                            rot * Vec3::new(0.0, 0.7, 0.0),
                            Vec3::new(0.12, 1.4 - s as f32 * 0.3, 0.12),
                            rot,
                            lin(CHAR),
                            Vec3::ZERO,
                            SH_WOOD,
                        );
                    }
                }
            }
        }
        let top = Vec3::new(gate.x as f32, 6.0, -3.0);
        decor::great_banner(out, Mat4::from_translation(top));
        fires.push(top + Vec3::new(0.0, 7.4, 0.3));
    }

    /// Far out in the lava: spires of obsidian, a siege tower sunk to its middle, slabs tipped on
    /// end, glowing where the lava eats them.
    fn build_field_far(&mut self) {
        let out = &mut self.static_cubes;
        for k in 0..40 {
            let side = k % 3;
            let a = hash(k as f64 * 4.1, 3.0) as f32;
            let b = hash(k as f64 * 2.3, 8.0) as f32;
            let (x, z) = match side {
                0 => (-4.0 - a * 14.0, b * 118.0),
                1 => (63.0 + a * 14.0, b * 118.0),
                _ => (a * 60.0, 121.0 + b * 10.0),
            };
            let kind = hash(k as f64, 5.0);
            if kind < 0.45 {
                let tall = 1.5 + b * 3.5;
                for s in 0..4 {
                    let t = s as f32 / 4.0;
                    let w = (0.9 - t * 0.6) * (0.7 + a * 0.6);
                    bx(
                        out,
                        Mat4::IDENTITY,
                        Vec3::new(x + t * 0.2, LAVA_Y + tall * (t + 0.12), z),
                        Vec3::new(w, tall / 4.0 + 0.05, w * 0.8),
                        Quat::from_rotation_y(a * 3.0 + t),
                        lin(OBSIDIAN) * (1.0 + t * 0.5),
                        Vec3::ZERO,
                        SH_METAL,
                    );
                }
                glowing(out, Vec3::new(x, LAVA_Y + 0.05, z), Vec3::new(1.3, 0.08, 1.1), lin(LAVA_DEEP), lin(LAVA) * 1.4);
            } else if kind < 0.65 {
                let root = Mat4::from_translation(Vec3::new(x, LAVA_Y - 1.2, z)) * Mat4::from_rotation_z(0.3 * (a - 0.5));
                decor::tower(out, root, k as f64);
            } else {
                let root =
                    Mat4::from_translation(Vec3::new(x, LAVA_Y, z)) * Mat4::from_rotation_y(a * TAU) * Mat4::from_rotation_x(0.9 + b * 0.5);
                plain(out, root, Vec3::new(0.0, 0.0, 0.0), Vec3::new(3.0 + a * 3.0, 0.5, 2.0 + b * 2.0), 0x2e221c);
                plain(out, root, Vec3::new(0.0, 0.26, 0.0), Vec3::new(2.9 + a * 3.0, 0.04, 1.9 + b * 2.0), EARTH[1]);
            }
        }
    }

    /// The realm's own animation, one step: slabs whose shape has changed are built again.
    pub(super) fn update_field_view(&mut self, v: &mut FieldView, r: &Battlefield, _s: &SceneState, _dt: f32) {
        for s in &r.slabs {
            let stale = v.slabs.get(&s.id).is_none_or(|a| a.key != slab_key(s));
            if stale {
                v.slabs.insert(s.id, build_slab(s));
            }
        }
        if v.slabs.len() > r.slabs.len() {
            v.slabs.retain(|id, _| r.slabs.iter().any(|s| s.id == *id));
        }
    }

    /// What moves, glows and lights in the realm.
    pub(super) fn emit_field(&self, v: &FieldView, r: &Battlefield, s: &SceneState, time: f32, hero: Vec3, f: &mut Frame) {
        let cube = MeshKind::Cube as usize;
        let flash = r.flash_light() as f32;
        let mut lit: Vec<(Vec3, Vec3, f32)> = Vec::new();
        // ---- the slabs where they drift
        for sl in &r.slabs {
            if sl.risen <= 0.0 || sl.sinking.is_some_and(|k| k.t > k.over + 0.9) {
                continue;
            }
            let Some(art) = v.slabs.get(&sl.id) else { continue };
            let (lo, hi) = sl.bounds();
            let dx = (lo.x as f32 - hero.x).max(hero.x - hi.x as f32).max(0.0);
            let dz = (lo.y as f32 - hero.z).max(hero.z - hi.y as f32).max(0.0);
            if dx > RANGE || dz > RANGE {
                continue;
            }
            let root = slab_root(sl, time);
            let out = &mut f.opaque[cube];
            out.reserve(art.cubes.len());
            for inst in &art.cubes {
                let m = root * Mat4::from_cols_array(&inst.m);
                out.push(Instance { m: m.to_cols_array(), ..*inst });
            }
            self.emit_cracks(sl, root, f);
            if flash > 0.0 {
                emit_outline(sl, root, flash, f);
            }
            for fire in &art.fires {
                let p = root.transform_point3(*fire);
                lit.push((p, lin(EMBER) * 1.2, 3.5));
            }
            if sl.risen < 1.0 || sl.sinking.is_some() {
                // Lava pouring off a slab coming up, or seething round one going down.
                let c = root.transform_point3(Vec3::new((sl.w - 1) as f32 / 2.0, 0.0, (sl.h - 1) as f32 / 2.0));
                glow_orb(f, Vec3::new(c.x, LAVA_Y + 0.3, c.z), (sl.w.max(sl.h) as f32) * 0.6, lin(LAVA), 0.35);
            }
        }
        self.emit_corpses(r, time, f);
        self.emit_field_lava(r, s, time, hero, &mut lit, f);
        self.emit_horn(v, r, time, f);
        self.emit_truce(r, time, f);
        self.emit_storm(r, time, hero, f);
        self.emit_fury(r, s, time, hero, f);
        // ---- the fires, the nearest few
        for p in &v.fires {
            if p.distance_squared(hero) < 16.0 * 16.0 {
                lit.push((*p, lin(0xff8a3a) * 1.6, 6.0));
            }
        }
        lit.sort_by(|a, b| a.0.distance_squared(hero).total_cmp(&b.0.distance_squared(hero)));
        for (k, (p, c, d)) in lit.into_iter().take(12).enumerate() {
            let flick = 0.85 + (time * 9.0 + k as f32 * 1.3).sin() * 0.1 + (time * 23.0 + k as f32).sin() * 0.05;
            f.lights.push(Light { pos: p, color: c * flick, distance: d });
        }
    }

    /// A slab's cracks: from every blow it has taken, a jagged line glowing with the lava under it,
    /// longer and brighter at every crack.
    fn emit_cracks(&self, sl: &Slab, root: Mat4, f: &mut Frame) {
        if sl.scars.is_empty() {
            return;
        }
        let stage = sl.cracks() as f32;
        let heat = 0.6 + stage * 0.5 + if sl.sinking.is_some() { 1.0 } else { 0.0 };
        for (k, sc) in sl.scars.iter().enumerate() {
            let seed = k as f64 * 3.7 + sl.id as f64;
            let mut p = Vec3::new(sc.x as f32, 0.0, sc.y as f32);
            let mut a = rnd(seed, 0) * TAU;
            let n = 2 + stage as u32;
            for j in 0..n {
                let len = 0.3 + rnd(seed, 1 + j) * 0.35;
                a += (rnd(seed, 10 + j) - 0.5) * 1.4;
                let d = Vec3::new(a.cos(), 0.0, a.sin()) * len;
                let mid = p + d * 0.5;
                if mid.x < -0.45 || mid.z < -0.45 || mid.x > sl.w as f32 - 0.55 || mid.z > sl.h as f32 - 0.55 {
                    break;
                }
                let rot = Quat::from_rotation_y(-a);
                let m = |w: f32, y: f32| {
                    root * Mat4::from_scale_rotation_translation(Vec3::new(len, 0.02, w), rot, mid + Vec3::new(0.0, y, 0.0))
                };
                f.opaque[MeshKind::Cube as usize].push(Instance::new(m(0.08, 0.004), lin(CHAR), 1.0, Vec3::ZERO, SHADE_LIT));
                f.opaque[MeshKind::Cube as usize].push(Instance::new(m(0.03, 0.012), lin(LAVA_DEEP), 1.0, lin(LAVA) * heat, SHADE_LIT));
                p += d;
            }
        }
    }

    /// The slain where they lie: a heap of armour and bones, a cold light gathering in it as the
    /// next flash nears.
    fn emit_corpses(&self, r: &Battlefield, time: f32, f: &mut Frame) {
        let soon = (1.0 - r.next_flash as f32 / 1.5).clamp(0.0, 1.0);
        for (k, c) in r.corpses.iter().enumerate() {
            let Some(p) = r.ground_pos(c.at) else { continue };
            let y = match c.at {
                Ground::Slab(id, _) => r.slab(id).map(|s| s.elevation()).unwrap_or(0),
                Ground::Isle(_) => {
                    if r.isle.contains(P::new(p.x.round() as i32, p.y.round() as i32)) {
                        1
                    } else {
                        0
                    }
                }
            } as f32
                * STEP;
            let at = Vec3::new(p.x as f32, y, p.y as f32);
            let yaw = k as f32 * 2.3;
            let root = Mat4::from_translation(at) * Mat4::from_rotation_y(yaw);
            let out = &mut f.opaque[MeshKind::Cube as usize];
            plain(out, root, Vec3::new(0.0, 0.06, 0.0), Vec3::new(0.6, 0.12, 0.3), 0x3a3634);
            plain(out, root, Vec3::new(0.18, 0.13, 0.02), Vec3::new(0.2, 0.06, 0.24), STEEL_D);
            decor::skull(out, root, Vec3::new(-0.32, 0.08, 0.0), 0.4, 0.9);
            plain(out, root, Vec3::new(0.05, 0.03, 0.22), Vec3::new(0.45, 0.04, 0.05), BONE_D);
            let pulse = 0.25 + soon * 0.9 + (time * 3.0 + k as f32).sin() * 0.08;
            glow_orb(f, at + Vec3::new(0.0, 0.35, 0.0), 0.5 + soon * 0.4, lin(0x9ab0ff), pulse);
        }
    }

    /// The lava: risen over the low shelf in the horn's call, glowing up through the gaps near him,
    /// embers rising off it and ash drifting down.
    fn emit_field_lava(&self, r: &Battlefield, _s: &SceneState, time: f32, hero: Vec3, lit: &mut Vec<(Vec3, Vec3, f32)>, f: &mut Frame) {
        let (hx, hz) = (hero.x.round() as i32, hero.z.round() as i32);
        let rise = r.lava as f32 * 0.55;
        if rise > 0.01 {
            let y = LAVA_Y + rise;
            for z in hz - 16..=hz + 8 {
                for x in hx - 14..=hx + 14 {
                    if r.on_isle(V::new(x as f64, z as f64)) {
                        continue;
                    }
                    // (The crust drifts slowly over the risen lava.)
                    let (col, glow) = lava_look(x as f32 + (time * 0.3).floor(), z as f32);
                    let m = Mat4::from_scale_rotation_translation(
                        Vec3::new(1.0, 0.1, 1.0),
                        Quat::IDENTITY,
                        Vec3::new(x as f32, y - 0.05, z as f32),
                    );
                    f.opaque[MeshKind::Cube as usize].push(Instance::new(m, col * 0.8, 1.0, glow * 0.6, SHADE_LIT));
                }
            }
        }
        // The lava's own light, up through the nearest gaps.
        let mut gaps = 0;
        for k in 0..49 {
            let (gx, gz) = (k % 7 - 3, k / 7 - 4);
            let t = P::new((hx / 3 + gx) * 3, (hz / 3 + gz) * 3);
            if !r.lava_at(t) || gaps >= 7 {
                continue;
            }
            gaps += 1;
            lit.push((Vec3::new(t.x as f32, LAVA_Y + 0.2, t.y as f32), lin(0xe8661c) * 0.7, 4.5));
        }
        // Embers rising, and ash falling.
        for k in 0..36 {
            let a = hash(k as f64 * 1.9, 3.0) as f32;
            let b = hash(k as f64 * 2.7, 5.0) as f32;
            let life = (time * (0.25 + a * 0.2) + b) % 1.0;
            let p = Vec3::new(hero.x - 10.0 + a * 20.0 + (time + k as f32).sin() * 0.3, LAVA_Y + life * 4.5, hero.z - 12.0 + b * 18.0);
            let s = 0.04 * (1.0 - life);
            f.additive.push(Instance::new(
                Mat4::from_scale_rotation_translation(Vec3::splat(s), Quat::IDENTITY, p),
                lin(EMBER),
                1.0 - life,
                Vec3::ZERO,
                SHADE_UNLIT,
            ));
        }
        for k in 0..60 {
            let a = hash(k as f64 * 3.1, 7.0) as f32;
            let b = hash(k as f64 * 1.3, 9.0) as f32;
            let fall = (time * (0.08 + a * 0.05) + b) % 1.0;
            let p = Vec3::new(
                hero.x - 9.0 + a * 18.0 + (time * 0.7 + k as f32).sin() * 0.6,
                hero.y + 5.0 - fall * 6.0,
                hero.z - 11.0 + b * 16.0,
            );
            let m = Mat4::from_scale_rotation_translation(Vec3::new(0.05, 0.012, 0.04), Quat::from_rotation_y(time + k as f32), p);
            f.transparent[MeshKind::Cube as usize].push(Instance::new(m, lin(0x8a8280), 0.6, Vec3::ZERO, SHADE_UNLIT));
        }
    }

    /// The horn on its cord, swinging after it has sounded, its call rolling out in rings.
    fn emit_horn(&self, v: &FieldView, r: &Battlefield, time: f32, f: &mut Frame) {
        let age = r.horn_age as f32;
        let swing = if age < 12.0 { (age * 5.0).sin() * (-age * 0.5).exp() * 0.7 } else { 0.0 } + (time * 0.9).sin() * 0.05;
        let root = Mat4::from_translation(v.cord) * Mat4::from_rotation_x(swing);
        let cube = MeshKind::Cube as usize;
        f.opaque[cube].push(Instance::new(
            root * Mat4::from_scale_rotation_translation(Vec3::new(0.025, 0.5, 0.025), Quat::IDENTITY, Vec3::new(0.0, -0.25, 0.0)),
            lin(ROPE),
            1.0,
            Vec3::ZERO,
            SHADE_LIT,
        ));
        let horn = root * Mat4::from_translation(Vec3::new(0.0, -0.95, 0.0)) * Mat4::from_rotation_y(0.15);
        push_merged(&mut f.opaque[cube], &v.horn, horn, if r.called.is_some() { 0.08 } else { 0.0 });
        if age < 3.0 {
            for k in 0..3 {
                let t = (age - k as f32 * 0.4).max(0.0);
                if t <= 0.0 {
                    continue;
                }
                let radius = 0.6 + t * 4.0;
                let m = Mat4::from_translation(v.cord + Vec3::new(0.0, -0.5, 0.0))
                    * Mat4::from_rotation_x(PI / 2.0)
                    * Mat4::from_scale(Vec3::new(radius, radius, 1.0));
                f.transparent[MeshKind::Annulus as usize].push(Instance::new(
                    m,
                    lin(0xffd8a0),
                    (1.0 - t / 3.0) * 0.5,
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
        }
    }

    /// The truce stone's ring: a faint circle on the terrace, lit round as he holds the truce, and,
    /// once it opens, a column of white light from the stone.
    fn emit_truce(&self, r: &Battlefield, time: f32, f: &mut Frame) {
        let stone = Vec3::new(r.stone.x as f32, 0.0, r.stone.y as f32);
        let n = 48;
        let held = if r.opened { 1.0 } else { (r.truce / TRUCE_SECONDS) as f32 };
        let radius = TRUCE_RADIUS as f32;
        for k in 0..n {
            let a = k as f32 / n as f32 * TAU - PI / 2.0;
            let p = stone + Vec3::new(a.cos() * radius, 0.02, a.sin() * radius);
            let on = (k as f32 + 0.5) / n as f32 <= held;
            let w = if on { 0.08 } else { 0.05 };
            let m = Mat4::from_scale_rotation_translation(Vec3::new(w, 0.02, radius * TAU / n as f32 * 0.75), Quat::from_rotation_y(-a), p);
            if on {
                f.additive.push(Instance::new(m, Vec3::new(0.75, 0.75, 0.8), 1.0, Vec3::ZERO, SHADE_UNLIT));
            } else {
                f.transparent[MeshKind::Cube as usize].push(Instance::new(m, Vec3::splat(0.35), 0.3, Vec3::ZERO, SHADE_UNLIT));
            }
        }
        if r.opened {
            let rise = (r.open_age as f32 * 0.8).min(1.0);
            let pulse = 0.8 + (time * 3.0).sin() * 0.2;
            for (w, k) in [(0.9f32, 0.18f32), (0.45, 0.4)] {
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::new(w, 12.0 * rise, w),
                    Quat::IDENTITY,
                    stone + Vec3::new(0.0, 6.0 * rise, 0.0),
                );
                f.additive.push(Instance::new(m, Vec3::new(1.0, 0.98, 0.94) * k * pulse, 1.0, Vec3::ZERO, SHADE_UNLIT));
            }
            glow_orb(f, stone + Vec3::new(0.0, 1.4, 0.0), 1.4, Vec3::ONE, 0.45 * pulse);
            f.lights.push(Light { pos: stone + Vec3::new(0.0, 2.0, 0.0), color: Vec3::splat(2.2) * pulse, distance: 8.0 });
        } else if held > 0.0 {
            glow_orb(f, stone + Vec3::new(0.0, 1.1, 0.0), 0.5 + held * 0.5, Vec3::ONE, 0.1 + held * 0.25);
        }
    }

    /// The storm: the ghost army wheeling overhead (faint, and clear in the flash), and the bolt of
    /// each flash striking down with its light.
    fn emit_storm(&self, r: &Battlefield, time: f32, hero: Vec3, f: &mut Frame) {
        let flash = r.flash_light() as f32;
        let n = 16;
        let mut riders = Vec::new();
        for k in 0..n {
            let a = time * 0.11 + k as f32 / n as f32 * TAU;
            let radius = 11.0 + (k % 3) as f32 * 1.6;
            let centre = Vec3::new(hero.x, hero.y + 3.4 + (k % 2) as f32 * 0.7 + (time * 0.5 + k as f32).sin() * 0.25, hero.z - 3.0);
            let p = centre + Vec3::new(a.cos() * radius, 0.0, a.sin() * radius * 0.75);
            let yaw = -(a + PI / 2.0);
            let root = Mat4::from_translation(p) * Mat4::from_rotation_y(yaw) * Mat4::from_scale(Vec3::splat(0.9));
            decor::rider(&mut riders, root, time * 7.0 + k as f32 * 1.1);
        }
        let alpha = 0.07 + flash * 0.4;
        let color = lin(0xc8d8ff) * (0.6 + flash * 0.9);
        for mut inst in riders {
            inst.color = [color.x, color.y, color.z, alpha];
            f.transparent[MeshKind::Cube as usize].push(inst);
        }
        if flash <= 0.0 || r.flash >= FLASH_SECONDS {
            return;
        }
        // The bolt: a jagged line from the clouds down to where it strikes.
        let ground = Vec3::new(r.bolt.x as f32, 0.6, r.bolt.y as f32);
        let mut p = ground + Vec3::new(0.0, 14.0, 0.0);
        let seed = r.flashes as f64 * 7.3;
        for k in 0..9 {
            let t = (k + 1) as f32 / 9.0;
            let next = ground.lerp(ground + Vec3::new(0.0, 14.0, 0.0), 1.0 - t)
                + Vec3::new((rnd(seed, k) - 0.5) * 1.6 * (1.0 - t), 0.0, (rnd(seed, k + 20) - 0.5) * 1.2 * (1.0 - t));
            let d = next - p;
            let len = d.length();
            let rot = Quat::from_rotation_arc(Vec3::Y, d / len);
            let m = Mat4::from_scale_rotation_translation(Vec3::new(0.07, len, 0.07), rot, p + d * 0.5);
            f.additive.push(Instance::new(m, Vec3::new(0.9, 0.95, 1.0) * 1.6 * flash, 1.0, Vec3::ZERO, SHADE_UNLIT));
            p = next;
        }
        glow_orb(f, ground, 1.4, Vec3::new(0.85, 0.9, 1.0), 0.7 * flash);
        f.lights.push(Light { pos: ground + Vec3::new(0.0, 2.0, 0.0), color: Vec3::new(4.0, 4.4, 5.0) * 2.5 * flash, distance: 14.0 });
    }

    /// His fury: embers of it circling him, one for each stack, and in Berserkergang a red rage
    /// about him.
    fn emit_fury(&self, r: &Battlefield, s: &SceneState, time: f32, hero: Vec3, f: &mut Frame) {
        if !s.hero.alive {
            return;
        }
        for k in 0..r.fury {
            let a = time * 3.0 + k as f32 / r.fury.max(1) as f32 * TAU;
            let p = hero + Vec3::new(a.cos() * 0.55, 0.8 + (time * 4.0 + k as f32).sin() * 0.15, a.sin() * 0.55);
            f.additive.push(Instance::new(
                Mat4::from_scale_rotation_translation(Vec3::splat(0.06), Quat::IDENTITY, p),
                lin(0xff4a2a) * 2.0,
                1.0,
                Vec3::ZERO,
                SHADE_UNLIT,
            ));
        }
        if r.berserk > 0.0 {
            let pulse = 0.7 + (time * 10.0).sin() * 0.3;
            glow_orb(f, hero + Vec3::new(0.0, 0.7, 0.0), 1.5, lin(0xff2a1a), 0.6 * pulse);
            let m = Mat4::from_translation(hero + Vec3::new(0.0, 0.05, 0.0))
                * Mat4::from_rotation_x(PI / 2.0)
                * Mat4::from_scale(Vec3::new(1.1 * pulse, 1.1 * pulse, 1.0));
            f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, lin(0xff3a2a) * 1.5, 0.6, Vec3::ZERO, SHADE_UNLIT));
            f.lights.push(Light { pos: hero + Vec3::new(0.0, 1.0, 0.0), color: lin(0xff3020) * 3.0 * pulse, distance: 4.0 });
        }
    }
}

/// In the flash, every edge of a slab outlined in white: round its top and down its corners into
/// the lava.
fn emit_outline(sl: &Slab, root: Mat4, flash: f32, f: &mut Frame) {
    let (w, h) = (sl.w as f32, sl.h as f32);
    let (x0, z0, x1, z1) = (-0.5, -0.5, w - 0.5, h - 0.5);
    let drop = LAVA_Y - sl.elevation() as f32 * STEP;
    let white = Vec3::new(1.0, 1.0, 1.05) * 2.4 * flash;
    let mut line = |at: Vec3, size: Vec3| {
        f.additive.push(Instance::new(
            root * Mat4::from_scale_rotation_translation(size, Quat::IDENTITY, at),
            white,
            1.0,
            Vec3::ZERO,
            SHADE_UNLIT,
        ));
    };
    let t = 0.06;
    line(Vec3::new((x0 + x1) / 2.0, 0.02, z0), Vec3::new(w + t, t, t));
    line(Vec3::new((x0 + x1) / 2.0, 0.02, z1), Vec3::new(w + t, t, t));
    line(Vec3::new(x0, 0.02, (z0 + z1) / 2.0), Vec3::new(t, t, h + t));
    line(Vec3::new(x1, 0.02, (z0 + z1) / 2.0), Vec3::new(t, t, h + t));
    for (x, z) in [(x0, z0), (x1, z0), (x0, z1), (x1, z1)] {
        line(Vec3::new(x, drop / 2.0, z), Vec3::new(t, -drop, t));
    }
}

/// The realm's sky, fog and light: an ash-dark red, short fog, the lava lighting from below and a
/// dim sun through the smoke; the lightning whitens it all for a moment and throws hard shadows;
/// in the horn's call the air reddens.
pub(super) fn atmosphere(_v: &FieldView, r: &Battlefield, _s: &SceneState) -> Atmosphere {
    let flash = r.flash_light() as f32;
    let call = r.lava as f32;
    let k = flash * flash;
    let bg = mix_hex(mix_hex(0x1a0806, 0x2e0a06, call), 0xc8ccff, k * 0.7);
    let sun_dir = Vec3::new(0.25, 1.0, 0.35).normalize().lerp(Vec3::new(-0.35, 1.0, 0.15).normalize(), k).normalize();
    Atmosphere {
        background: bg,
        fog_near: 12.0 + k * 10.0,
        fog_far: 30.0 + k * 24.0,
        hemi_sky: (lin(0x8a7a70) * 1.0).lerp(lin(0xdde4ff) * 2.2, k),
        hemi_ground: lin(0xd0702e) * (0.55 + call * 0.35),
        ambient: (lin(0x5a4c46) * 0.9).lerp(lin(0xc8ccff) * 1.5, k),
        sun_dir,
        sun: (lin(0xf0bc90) * (1.5 + call * 0.2)).lerp(lin(0xeef2ff) * 4.5, k),
        exposure: 1.25,
        night_sky: 0.0,
        bloom: 1.0,
        moon_view: Vec3::new(0.0, 0.3, -1.0).normalize(),
        cloud_tint: lin(0x2a0a08),
    }
}
