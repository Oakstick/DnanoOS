//! The Portals of Hell's architecture at the Cisterns' bar (see `hell_art`): the seven portals as
//! monumental arches, the House in the volcano's face, and the Seal of the Seven.
//!
//! - **A portal**: piers of chunky dressed courses (basalt and umber, pale quoins) on pale plinths
//!   under pale imposts; three receding orders of voussoirs, the middle one pale with the sin's
//!   gem for its keystone; a lintel and a stepped tympanum over the doors, a gold ring round the
//!   sin's gem in it; a pale frieze with the sin's name in gold letters, a cornice, a stepped crest
//!   with ivory horns and the sin's gem for a beacon; copper braziers on pale pedestals before it,
//!   burning in the sin's colour (drawn live, `emit_portal`). The doors, the swirl and the rune
//!   ring are drawn live as before.
//! - **The House**: its walls the crypt's dressed courses round lancet windows burning ember-red,
//!   pale string courses and cornices, its roofs and spires polished obsidian, ivory horns, an iron
//!   door under a pointed arch of glowing stones, copper braziers either side. Its name and the
//!   story's words are untouched, and nothing on it depicts a devil.
//! - **The Seal**: pale dressed steps with ember runes, iron sockets, a polished obsidian monolith.

use super::crypt::pal::CHARCOAL;
use super::crypt::{arch_cubes, block, block_m, brazier_cubes, jit, masonry, Arch, Wall, SHADE_IRON, SHADE_STONE};
use super::hell::{gate_root, glowing, skull, SEAL_RING};
use super::hell_art::art::*;
use super::hell_art::{as_stone, placed, tame_flames, SHADE_GLOSS, SHADE_GOLD, SHADE_HORN};
use super::*;
use crate::logic::hub::{self, Gate};
use crate::logic::sins::SINS;

/// Where a hub portal's braziers stand (across from the middle, out in front), and their coals'
/// height (`emit_portal` draws their flames there).
pub(super) const PORTAL_BRAZIER: (f32, f32) = (1.3, 0.72);
pub(super) const PORTAL_COALS: f32 = 0.37 + 0.77 * 0.75;

/// The name over a hub portal, this much larger than a realm's.
const NAME_SCALE: f32 = 1.25;
/// Obsidian, polished; the windows' ember-red; ivory.
const OBSIDIAN: [u32; 2] = [0x221c22, 0x2e2630];
const WINDOW: u32 = 0xff4a14;
const IVORY: [u32; 2] = [0xd8c8a8, 0x5a4a3a];
const GOLD: u32 = 0xe8b84a;

/// A horn of ivory rising from `base`, sweeping out to `side` (-1 or 1), darkening to its tip.
fn ivory_horn(out: &mut Vec<Instance>, base: Vec3, side: f32, size: f32) {
    let mut p = base;
    for k in 0..8 {
        let t = (k + 1) as f32 / 8.0;
        let q = base + Vec3::new(side * (0.55 * t + 0.25 * t * t) * size, (0.9 * t - 0.2 * t * t * t) * size, -0.1 * t * size);
        let thick = 0.26 * size * (1.0 - t * 0.8);
        let c = lin(IVORY[0]).lerp(lin(IVORY[1]), (t - 0.5).max(0.0) * 1.6);
        block_m(out, forge::segment(p, q, thick), c, SHADE_HORN);
        p = q;
    }
}

/// A face of the House's dressed stone (see `crypt::masonry`).
#[allow(clippy::too_many_arguments)]
fn wall(
    centre: Vec3,
    dir: (i32, i32),
    half: f32,
    base: f32,
    height: f32,
    courses: usize,
    face: f32,
    seed: f64,
    openings: &[(f32, f32, f32, f32, f32)],
) -> Wall<'_> {
    Wall {
        centre,
        dir,
        half,
        base,
        height,
        courses,
        seed: (seed, seed * 1.3 + 2.0),
        len: (0.36, 0.32),
        depth: (0.1, 0.08),
        face,
        wet: false,
        ruined: false,
        foot: None,
        openings,
    }
}

/// A long course laid as a run of dressed blocks of uneven length along x (one block that long
/// would stretch the stone's tooling into a grain), each a touch proud or back.
fn run(out: &mut Vec<Instance>, centre: Vec3, size: Vec3, colour: Vec3, seed: f64) {
    let mut x = -size.x / 2.0;
    let mut k = 0;
    while x < size.x / 2.0 - 0.01 {
        let h = hash(seed + k as f64 * 1.9, seed * 0.7 + 3.0);
        let mut w = 0.45 + h as f32 * 0.35;
        if size.x / 2.0 - (x + w) < 0.25 {
            w = size.x / 2.0 - x;
        }
        let proud = (h as f32 - 0.5) * 0.03;
        let c = colour * (0.9 + 0.2 * hash(h * 13.0, seed) as f32);
        block(out, centre + Vec3::new(x + w / 2.0, 0.0, proud), Vec3::new(w - 0.025, size.y, size.z), 0.0, c, SHADE_STONE);
        x += w;
        k += 1;
    }
}

/// A sin's gem: a diamond of its colour, glowing.
fn gem(out: &mut Vec<Instance>, at: Vec3, size: f32, color: Vec3, glow: f32) {
    let m = Mat4::from_translation(at) * Mat4::from_rotation_z(PI / 4.0) * Mat4::from_scale(Vec3::splat(size));
    out.push(Instance::new(m, color, 1.0, color * glow, SHADE_LIT));
}

/// Recolours a range of cubes built by the crypt's arch: the voussoirs in `stone`, alternating.
fn recolour(out: &mut [Instance], stone: &dyn Fn(usize) -> Vec3) {
    for (k, c) in out.iter_mut().enumerate() {
        let s = stone(k);
        c.color[0] = s.x;
        c.color[1] = s.y;
        c.color[2] = s.z;
    }
}

impl WorldView {
    /// A sin's portal: a monumental arch of dressed stone round its doors (see the module).
    pub(super) fn hub_portal(&mut self, g: &Gate) {
        let root = gate_root(g);
        let color = lin(g.sin.color());
        let (hx, hy) = (g.tile.x as f64, g.tile.y as f64);
        let pale = |k: usize, h: f64| jit(PALE[k % 2], h, 0.06) * 0.85;
        let basalt = |k: usize, h: f64| jit(BASALT[k % 3], h, 0.1);
        let mut l: Vec<Instance> = Vec::new();
        // The threshold, a line of the sin's colour inlaid at its lip, and the ring of rune stones
        // in the tile before the doors, where he stands to go through.
        run(&mut l, Vec3::new(0.0, 0.03, 0.12), Vec3::new(1.5, 0.1, 0.95), pale(0, hx) * 0.85, hx);
        let trim = color * 0.55;
        let glow = color * 0.25;
        l.push(Instance::new(
            Mat4::from_scale_rotation_translation(Vec3::new(1.3, 0.02, 0.05), Quat::IDENTITY, Vec3::new(0.0, 0.085, 0.55)),
            trim,
            1.0,
            glow,
            SHADE_LIT,
        ));
        for k in 0..10 {
            let a = k as f32 / 10.0 * TAU;
            let m = Mat4::from_scale_rotation_translation(
                Vec3::new(0.14, 0.02, 0.05),
                Quat::from_rotation_y(-a - PI / 2.0),
                Vec3::new(a.cos() * 0.4, 0.008, 1.0 + a.sin() * 0.4),
            );
            l.push(Instance::new(m, trim, 1.0, glow, SHADE_LIT));
        }
        // Dark behind the doors (set back, so the leaves swing clear of it).
        l.push(Instance::new(
            Mat4::from_scale_rotation_translation(Vec3::new(1.42, 2.7, 0.1), Quat::IDENTITY, Vec3::new(0.0, 1.35, -0.78)),
            lin(0x050305),
            1.0,
            Vec3::ZERO,
            SHADE_LIT,
        ));
        // The piers: a pale plinth, five chunky courses (a block through, then a pale quoin
        // within and basalt without), a pale impost.
        for side in [-1.0f32, 1.0] {
            let x = side * 1.02;
            block(&mut l, Vec3::new(x, 0.15, -0.05), Vec3::new(0.86, 0.3, 1.02), 0.0, pale(1, hx + side as f64) * 0.85, SHADE_STONE);
            let mut y = 0.3;
            for c in 0..5 {
                let ch = 0.46;
                let hk = hash(hx * 3.0 + c as f64 * 1.7 + side as f64, hy * 2.0 + c as f64);
                let askew = (hk as f32 - 0.5) * 0.04;
                let proud = (hash(hk * 7.0, hx) as f32 - 0.5) * 0.06;
                if c % 2 == 0 {
                    let col = if hk < 0.3 { jit(UMBER[c % 2], hk, 0.08) } else { basalt(c, hk) };
                    block(&mut l, Vec3::new(x, y + ch / 2.0, -0.05 + proud), Vec3::new(0.72, ch - 0.03, 0.9), askew, col, SHADE_STONE);
                } else {
                    block(
                        &mut l,
                        Vec3::new(x - side * 0.19, y + ch / 2.0, -0.04 + proud),
                        Vec3::new(0.34, ch - 0.03, 0.94),
                        askew,
                        pale(c, hk),
                        SHADE_STONE,
                    );
                    block(
                        &mut l,
                        Vec3::new(x + side * 0.19, y + ch / 2.0, -0.06),
                        Vec3::new(0.34, ch - 0.03, 0.86),
                        -askew,
                        basalt(c + 1, hk),
                        SHADE_STONE,
                    );
                }
                y += ch;
            }
            block(
                &mut l,
                Vec3::new(x - side * 0.02, 2.68, -0.02),
                Vec3::new(0.88, 0.16, 1.04),
                0.0,
                pale(0, hy + side as f64) * 0.95,
                SHADE_STONE,
            );
        }
        // Behind the orders, the wall they are set in: two courses of blocks.
        for (row, y) in [(0usize, 3.1f32), (1, 3.72)] {
            let mut x = -1.48 + if row == 1 { 0.3 } else { 0.0 };
            let mut k = 0;
            while x < 1.48 {
                let hk = hash(hx * 5.0 + k as f64 + row as f64 * 9.0, hy * 3.0);
                let w = (0.55 + hk as f32 * 0.4).min(1.48 - x);
                block(&mut l, Vec3::new(x + w / 2.0, y, -0.32), Vec3::new(w - 0.03, 0.6, 0.5), 0.0, basalt(k, hk) * 0.85, SHADE_STONE);
                x += w;
                k += 1;
            }
        }
        // The lintel over the doors, and the tympanum stepped up under the inner order.
        run(&mut l, Vec3::new(0.0, 2.55, -0.06), Vec3::new(1.42, 0.24, 0.66), pale(1, hx * 2.0) * 0.9, hy + 5.0);
        for (k, w) in [1.32f32, 1.18, 0.96, 0.6].into_iter().enumerate() {
            block(&mut l, Vec3::new(0.0, 2.76 + k as f32 * 0.18, -0.2), Vec3::new(w, 0.18, 0.3), 0.0, basalt(k, hy) * 0.7, SHADE_STONE);
        }
        // A gold ring round the sin's gem, set in the tympanum.
        for k in 0..10 {
            let a = k as f32 / 10.0 * TAU;
            let m = Mat4::from_scale_rotation_translation(
                Vec3::new(0.09, 0.035, 0.04),
                Quat::from_rotation_z(a + PI / 2.0),
                Vec3::new(a.cos() * 0.22, 3.04 + a.sin() * 0.22, -0.03),
            );
            l.push(Instance::new(m, lin(GOLD), 1.0, lin(GOLD) * 0.12, SHADE_GOLD));
        }
        gem(&mut l, Vec3::new(0.0, 3.04, -0.02), 0.12, color, 1.0);
        // Three orders of voussoirs, each larger and further forward: basalt, pale, umber.
        let orders = [(0.8f32, -0.15f32, 0.6f32), (1.06, 0.12, 0.4), (1.3, 0.3, 0.3)];
        for (k, &(r, z, depth)) in orders.iter().enumerate() {
            let from = l.len();
            let crown = arch_cubes(
                &mut l,
                &Arch {
                    centre: Vec3::new(0.0, 0.0, z),
                    across_x: true,
                    r,
                    rise: r,
                    spring: 2.66,
                    depth,
                    piers: false,
                    lintel: false,
                    seed: (hx + k as f64 * 3.0, hy),
                },
            );
            let tone = move |j: usize| -> Vec3 {
                let h = hash(hx + j as f64 * 1.3, hy + k as f64 * 7.0);
                match k {
                    0 => jit(BASALT[j % 3], h, 0.1),
                    1 => jit(PALE[j % 2], h, 0.06) * 0.85,
                    _ => jit(if j.is_multiple_of(2) { UMBER[0] } else { BASALT[1] }, h, 0.08),
                }
            };
            recolour(&mut l[from..], &tone);
            if k == 1 {
                // The keystone carries the sin's gem.
                gem(&mut l, crown + Vec3::new(0.0, 0.04, depth / 2.0 + 0.03), 0.13, color, 1.2);
            }
        }
        // The frieze with the sin's name in gold, a cornice, a stepped crest, horns, the beacon.
        run(&mut l, Vec3::new(0.0, 4.37, 0.0), Vec3::new(3.1, 0.44, 1.0), basalt(1, hy * 3.0) * 0.6, hx * 3.0);
        for y in [4.17f32, 4.57] {
            run(&mut l, Vec3::new(0.0, y, 0.03), Vec3::new(3.16, 0.06, 1.02), pale(0, hy * 3.0 + y as f64) * 0.8, hy + y as f64 * 7.0);
        }
        run(&mut l, Vec3::new(0.0, 4.64, 0.04), Vec3::new(3.32, 0.1, 1.1), pale(1, hx * 3.0) * 0.8, hx + 11.0);
        run(&mut l, Vec3::new(0.0, 4.8, -0.02), Vec3::new(2.2, 0.22, 0.72), basalt(0, hx) * 0.9, hy + 13.0);
        block(&mut l, Vec3::new(0.0, 4.99, -0.02), Vec3::new(1.2, 0.16, 0.56), 0.0, basalt(2, hy) * 0.9, SHADE_STONE);
        block(&mut l, Vec3::new(0.0, 5.1, -0.02), Vec3::new(0.5, 0.08, 0.42), 0.0, pale(0, hy) * 0.9, SHADE_STONE);
        gem(&mut l, Vec3::new(0.0, 5.42, -0.02), 0.24, color, 1.2);
        for side in [-1.0f32, 1.0] {
            ivory_horn(&mut l, Vec3::new(side * 1.38, 4.68, -0.02), side, 0.85);
        }
        let name = &self.models.portal_names[g.sin.index()];
        let at = Mat4::from_translation(Vec3::new(0.0, 4.37, 0.505)) * Mat4::from_scale(Vec3::splat(NAME_SCALE));
        for (off, c) in &name.cubes {
            l.push(Instance::new(voxel_matrix(&at, name.size * NAME_SCALE, *off), *c, 1.0, *c * 0.2, SHADE_GOLD));
        }
        // Copper braziers on pale pedestals before the piers (their flames are drawn live).
        for side in [-1.0f32, 1.0] {
            let at = Vec3::new(side * PORTAL_BRAZIER.0, 0.0, PORTAL_BRAZIER.1);
            block(&mut l, at + Vec3::Y * 0.17, Vec3::new(0.26, 0.34, 0.26), 0.0, pale(1, hy + side as f64) * 0.85, SHADE_STONE);
            block(&mut l, at + Vec3::Y * 0.35, Vec3::new(0.32, 0.04, 0.32), 0.0, pale(0, hx + side as f64) * 0.8, SHADE_STONE);
            brazier_cubes(&mut l, at + Vec3::Y * 0.37, 0.75, false);
        }
        // What stands above the imposts fades out of the way of the view of him (from the top
        // camera, a side portal stands between it and the walk along the edge north of it).
        let (low, high): (Vec<Instance>, Vec<Instance>) = l.into_iter().partition(|c| c.m[13] < 2.6);
        placed(&mut self.static_cubes, root, &low);
        let start = self.static_cubes.len();
        placed(&mut self.static_cubes, root, &high);
        let len = self.static_cubes.len() - start;
        let shades = self.static_cubes[start..].iter().map(|c| c.emissive[3]).collect();
        self.trees.push(Tree {
            range: (start, len),
            base: root.transform_point3(Vec3::new(0.0, 2.6, 0.0)),
            radius: 1.0,
            height: 3.0,
            fade: 1.0,
            alive: true,
            shades: Some(shades),
        });
    }

    /// The House in the volcano's south face (see the module), on the old one's plan.
    pub(super) fn hub_house(&mut self, cx: f32, face: f32) {
        let mut torch_points = Vec::new();
        let mut smokes = Vec::new();
        let out = &mut self.static_cubes;
        let from = out.len();
        let obsidian = |k: usize| lin(OBSIDIAN[k % 2]);
        // Plinth and steps down to where the causeway will arrive.
        block(out, Vec3::new(cx, -0.2, face - 1.6), Vec3::new(6.2, 0.5, 3.8), 0.0, jit(BASALT[1], 0.4, 0.0) * 0.85, SHADE_STONE);
        for k in 0..3 {
            block(
                out,
                Vec3::new(cx, -0.04 - k as f32 * 0.12, face + 0.12 + k as f32 * 0.18),
                Vec3::new(2.6 - k as f32 * 0.2, 0.1, 0.2),
                0.0,
                jit(PALE[k % 2], k as f64 * 0.3, 0.05) * 0.8,
                SHADE_STONE,
            );
        }
        // The hall: a dark core, its front in dressed courses round the door and four lancets.
        block(out, Vec3::new(cx, 1.6, face - 1.6), Vec3::new(3.8, 3.2, 3.0), 0.0, lin(BASALT_D), SHADE_STONE);
        let windows = [(-1.35f32, 0.85f32), (1.35, 0.85), (-1.35, 2.05), (1.35, 2.05)];
        let mut openings = vec![(0.0f32, 0.74f32, 0.0f32, 2.15f32, 0.75f32)];
        for &(x, y) in &windows {
            openings.push((x, 0.13, y - 0.05, y + 0.42, 0.12));
        }
        masonry(out, &wall(Vec3::new(cx, 0.0, face - 1.6), (0, 1), 1.9, 0.05, 3.15, 7, 1.42, 5.0, &openings));
        for &(x, y) in &windows {
            glowing(out, Vec3::new(cx + x, y + 0.2, face - 0.17), Vec3::new(0.2, 0.5, 0.04), 0.0, lin(0x3a0a04), lin(WINDOW) * 1.8);
            block(out, Vec3::new(cx + x, y - 0.1, face - 0.1), Vec3::new(0.36, 0.06, 0.16), 0.0, jit(PALE[0], 0.3, 0.0) * 0.8, SHADE_STONE);
            block(
                out,
                Vec3::new(cx + x, y + 0.55, face - 0.1),
                Vec3::new(0.32, 0.08, 0.14),
                PI / 4.0,
                jit(PALE[1], 0.6, 0.0) * 0.8,
                SHADE_STONE,
            );
        }
        // A pale cornice; the roof stepped up in polished obsidian, iron spikes along its edge.
        block(out, Vec3::new(cx, 3.24, face - 1.58), Vec3::new(4.0, 0.12, 3.12), 0.0, jit(PALE[1], 0.2, 0.0) * 0.82, SHADE_STONE);
        for k in 0..4 {
            let w = 3.9 - k as f32 * 0.8;
            block(
                out,
                Vec3::new(cx, 3.45 + k as f32 * 0.3, face - 1.6),
                Vec3::new(w, 0.3, 3.1 - k as f32 * 0.5),
                0.0,
                obsidian(k),
                SHADE_GLOSS,
            );
        }
        for k in 0..7 {
            let x = cx - 1.8 + k as f32 * 0.6;
            block_m(out, forge::segment(Vec3::new(x, 3.3, face - 0.1), Vec3::new(x, 3.8, face - 0.1), 0.07), lin(IRON), SHADE_IRON);
        }
        // The towers: dressed courses on their faces, a lancet, a pale cornice, an obsidian
        // spire, a great ivory horn.
        for side in [-1.0f32, 1.0] {
            let tx = cx + side * 2.35;
            let tz = face - 0.9;
            block(out, Vec3::new(tx, 2.5, tz), Vec3::new(1.1, 5.0, 1.1), 0.0, lin(BASALT_D), SHADE_STONE);
            let lancet = [(0.0f32, 0.12f32, 3.3f32, 3.85f32, 0.1f32)];
            masonry(out, &wall(Vec3::new(tx, 0.0, tz), (0, 1), 0.6, 0.0, 5.0, 11, 0.5, 11.0 + side as f64, &lancet));
            masonry(out, &wall(Vec3::new(tx, 0.0, tz), (side as i32, 0), 0.6, 0.0, 5.0, 11, 0.5, 13.0 + side as f64, &[]));
            glowing(out, Vec3::new(tx, 3.6, tz + 0.5), Vec3::new(0.16, 0.55, 0.04), 0.0, lin(0x3a0a04), lin(WINDOW) * 1.8);
            block(out, Vec3::new(tx, 5.07, tz), Vec3::new(1.4, 0.14, 1.4), 0.0, jit(PALE[0], 0.5, 0.0) * 0.82, SHADE_STONE);
            for k in 0..6 {
                let s = 1.0 - k as f32 * 0.16;
                block(out, Vec3::new(tx, 5.3 + k as f32 * 0.26, tz), Vec3::new(0.9 * s, 0.27, 0.9 * s), PI / 4.0, obsidian(k), SHADE_GLOSS);
            }
            ivory_horn(out, Vec3::new(tx + side * 0.4, 4.9, tz), side, 1.0);
        }
        // The central spire behind, horned, its round window burning.
        let (sx, sz) = (cx, face - 2.4);
        block(out, Vec3::new(sx, 4.9, sz), Vec3::new(1.4, 3.4, 1.4), 0.0, obsidian(0), SHADE_GLOSS);
        block(out, Vec3::new(sx, 6.62, sz), Vec3::new(1.5, 0.1, 1.5), 0.0, jit(PALE[1], 0.7, 0.0) * 0.8, SHADE_STONE);
        for k in 0..7 {
            let s = 1.0 - k as f32 * 0.13;
            block(out, Vec3::new(sx, 6.8 + k as f32 * 0.3, sz), Vec3::new(1.1 * s, 0.31, 1.1 * s), PI / 4.0, obsidian(k + 1), SHADE_GLOSS);
        }
        for side in [-1.0f32, 1.0] {
            ivory_horn(out, Vec3::new(sx + side * 0.55, 6.2, sz), side, 1.6);
        }
        let m = Mat4::from_scale_rotation_translation(
            Vec3::new(0.5, 0.5, 0.04),
            Quat::from_rotation_z(PI / 4.0),
            Vec3::new(sx, 5.4, sz + 0.72),
        );
        out.push(Instance::new(m, lin(0x3a0a04), 1.0, lin(WINDOW) * 2.0, SHADE_LIT));
        // The door: a pointed arch of glowing stones, iron leaves studded, a skull above.
        let door_c = Vec3::new(cx, 0.0, face);
        for side in [-1.0f32, 1.0] {
            glowing(out, door_c + Vec3::new(side * 0.72, 1.1, 0.02), Vec3::new(0.16, 2.2, 0.1), 0.0, lin(0x4a1408), lin(EMBER) * 0.7);
            for k in 0..5 {
                let t = k as f32 / 4.0;
                let a = t * 0.95;
                let p = Vec3::new(side * (0.95 * a.cos() - 0.23), 2.2 + 0.7 * a.sin(), 0.02);
                out.push(Instance::new(
                    Mat4::from_translation(door_c + p) * Mat4::from_rotation_z(-side * a) * Mat4::from_scale(Vec3::new(0.14, 0.12, 0.12)),
                    lin(0x4a1408),
                    1.0,
                    lin(EMBER) * 0.7,
                    SHADE_LIT,
                ));
            }
            block(out, door_c + Vec3::new(side * 0.33, 1.1, -0.04), Vec3::new(0.64, 2.2, 0.08), 0.0, lin(IRON), SHADE_IRON);
            for k in 0..4 {
                for j in 0..2 {
                    block(
                        out,
                        door_c + Vec3::new(side * (0.18 + j as f32 * 0.28), 0.35 + k as f32 * 0.5, 0.02),
                        Vec3::splat(0.06),
                        PI / 4.0,
                        lin(0x4a4650),
                        SHADE_IRON,
                    );
                }
            }
        }
        let skull_at = out.len();
        skull(out, door_c + Vec3::new(0.0, 2.75, 0.05), 1.4);
        as_stone(&mut out[skull_at..], 0.95);
        // Copper braziers either side of the door, on pale pedestals.
        for side in [-1.0f32, 1.0] {
            let b = door_c + Vec3::new(side * 1.25, 0.0, 0.4);
            block(out, b + Vec3::Y * 0.15, Vec3::new(0.42, 0.3, 0.42), 0.0, jit(PALE[0], 0.1, 0.0) * 0.8, SHADE_STONE);
            let bowl = out.len();
            brazier_cubes(out, b + Vec3::Y * 0.3, 0.9, true);
            tame_flames(&mut out[bowl..]);
            torch_points.push(b + Vec3::Y * (0.3 + 0.7 - 0.35));
            smokes.push((b + Vec3::Y * 1.2, 0.9));
        }
        // (`masonry` lays the crypt's verdigris now and then: none here.)
        for c in out[from..].iter_mut() {
            if c.color[1] > c.color[0] * 1.02 && c.emissive[3] == SHADE_STONE {
                let u = lin(CHARCOAL[1]);
                c.color[0] = u.x * 0.9;
                c.color[1] = u.y * 0.85;
                c.color[2] = u.z * 0.85;
            }
        }
        let len = out.len() - from;
        self.smoke.extend(smokes);
        for (k, p) in torch_points.into_iter().enumerate() {
            self.torches.insert(P::new(-200 - k as i32, -200), (p, (from, len)));
        }
    }

    /// The Seal of the Seven on its dais in the entrance's middle (as before, in dressed stone).
    pub(super) fn hub_altar(&mut self) {
        let p = hub::altar();
        let c = Vec3::new(p.x as f32, 0.0, p.y as f32);
        let out = &mut self.static_cubes;
        // Three round steps of pale dressed stone, each four slabs turned through the eighth.
        for (k, (r, top, h)) in [(1.35f32, 0.04f32, 0.04f32), (0.95, 0.09, 0.05), (0.62, 0.17, 0.08)].iter().enumerate() {
            for s in 0..4 {
                let turn = s as f32 * PI / 4.0;
                block(
                    out,
                    c + Vec3::new(0.0, top - h / 2.0, 0.0),
                    Vec3::new(r * 2.0, *h, r * 0.83),
                    turn,
                    jit(PALE[k % 2], s as f64 * 0.2, 0.05) * (0.85 - k as f32 * 0.05),
                    SHADE_STONE,
                );
            }
        }
        // A ring of ember runes in the second step, socket to socket.
        for k in 0..16 {
            let a = k as f32 / 16.0 * TAU;
            let m = Mat4::from_scale_rotation_translation(
                Vec3::new(0.24, 0.012, 0.03),
                Quat::from_rotation_y(-a - PI / 2.0),
                c + Vec3::new(a.cos() * 0.8, 0.095, a.sin() * 0.8),
            );
            out.push(Instance::new(m, lin(0x2a0a04), 1.0, lin(EMBER) * 0.4, SHADE_LIT));
        }
        // The seven sockets: an obsidian block, an iron collar, a dark hole for the gem.
        for i in 0..SINS.len() {
            let a = i as f32 / 7.0 * TAU - PI / 2.0;
            let s = c + Vec3::new(a.cos() * SEAL_RING, 0.17, a.sin() * SEAL_RING);
            block(out, s + Vec3::new(0.0, 0.07, 0.0), Vec3::new(0.15, 0.14, 0.15), a, lin(OBSIDIAN[1]), SHADE_GLOSS);
            block(out, s + Vec3::new(0.0, 0.15, 0.0), Vec3::new(0.13, 0.02, 0.13), PI / 4.0, lin(0x4a4650), SHADE_IRON);
            block(out, s + Vec3::new(0.0, 0.14, 0.0), Vec3::new(0.06, 0.03, 0.06), PI / 4.0, lin(0x0a0608), SHADE_LIT);
        }
        // The seal stone: a polished obsidian monolith with ember rune bands, a cap, two horns.
        block(out, c + Vec3::new(0.0, 0.72, 0.0), Vec3::new(0.5, 1.1, 0.5), 0.0, lin(OBSIDIAN[0]), SHADE_GLOSS);
        for y in [0.55f32, 0.95] {
            for f in 0..4 {
                let a = f as f32 * PI / 2.0;
                let n = Vec3::new(a.cos(), 0.0, a.sin());
                glowing(
                    out,
                    c + Vec3::new(0.0, y, 0.0) + n * 0.255,
                    Vec3::new(0.3, 0.05, 0.02),
                    a + PI / 2.0,
                    lin(0x4a1406),
                    lin(EMBER) * 0.9,
                );
            }
        }
        // The seal itself on the south face: seven dim gems round an ember eye.
        for (i, sin) in SINS.iter().enumerate() {
            let a = i as f32 / 7.0 * TAU - PI / 2.0;
            let g = c + Vec3::new(a.cos() * 0.14, 0.75 + a.sin() * 0.14, 0.26);
            out.push(Instance::new(
                Mat4::from_scale_rotation_translation(Vec3::splat(0.035), Quat::from_rotation_z(PI / 4.0), g),
                lin(sin.color()) * 0.6,
                1.0,
                lin(sin.color()) * 0.35,
                SHADE_LIT,
            ));
        }
        glowing(out, c + Vec3::new(0.0, 0.75, 0.26), Vec3::new(0.06, 0.06, 0.02), PI / 4.0, lin(0x3a0a04), lin(EMBER) * 1.4);
        block(out, c + Vec3::new(0.0, 1.32, 0.0), Vec3::new(0.64, 0.1, 0.64), 0.0, jit(PALE[0], 0.4, 0.0) * 0.8, SHADE_STONE);
        block(out, c + Vec3::new(0.0, 1.5, 0.0), Vec3::new(0.4, 0.28, 0.4), 0.0, lin(OBSIDIAN[1]), SHADE_GLOSS);
        for side in [-1.0f32, 1.0] {
            ivory_horn(out, c + Vec3::new(side * 0.16, 1.6, 0.0), side, 0.55);
        }
    }
}
