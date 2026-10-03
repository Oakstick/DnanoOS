//! The Garden of Thorns as the view draws it (Lust's realm; laid out in `logic::realms::thorns`).
//!
//! Moonlight on a crimson pleasure garden: rose hedges as tall as walls, their lovely faces turned
//! to the camera and black bramble behind; pale rose-marble paths strewn with petals under rose
//! arches whose lanterns lead the eye along them, toward her; the court's marble and the heart's
//! crimson carpet; the koi in their marble fountains; the bridal bowers with the dead grooms in
//! them; perfume mist in the hollows; and over the bramble the Black Thorn, a silhouette at the
//! gate. The Bride stands ahead in a moonbeam, the brightest thing in the garden, rising out of the
//! roses; on the wedding night she walks the ground. What changes is drawn each frame or patched
//! into the static scene: Raphael's burnt paths, the facades turning to glass in the smoke.

mod decor;

use std::collections::HashMap;

use super::super::*;
use crate::logic::realms::thorns::{Ground, Thorns, CHARM_MAX, SMOKE_REVEAL, SMOKE_SECONDS, WEDDING_SECONDS};
use decor::pal::*;
use decor::{bx, h, Face, HEDGE_H};

/// How brightly his lantern burns here.
pub(super) const LANTERN: f32 = 0.55;

/// The Bride is the elf-maid this many times over.
const BRIDE_SCALE: f32 = 2.6;
/// She appears over this long where she has gone on to.
const APPEAR: f32 = 0.6;
/// Lanterns, koi and the like are drawn only this near him (tiles).
const NEAR: f32 = 26.0;

/// What the view keeps of the realm: each hedge tile's boxes and its fading tree (to burn and to
/// glass), how far each glassed tree has gone, how many burnt tiles are drawn, and where the
/// lanterns, the fountains' spouts, the censers and the thorn paths are.
pub(in crate::render::view) struct ThornsView(Box<ThornsState>);

/// (Kept boxed: the view of a realm being played is one of six.)
pub(in crate::render::view) struct ThornsState {
    hedges: HashMap<P, (usize, usize)>,
    tree_at: HashMap<P, usize>,
    hedge_trees: Vec<(usize, Vec2)>,
    glass: HashMap<usize, f32>,
    burnt: usize,
    lanterns: Vec<Vec3>,
    spouts: Vec<(Vec3, Vec3)>,
    thorns: Vec<Vec3>,
    time: f32,
}

impl WorldView {
    /// The realm's static geometry.
    pub(super) fn build_thorns(&mut self, grid: &Grid, r: &Thorns) -> ThornsView {
        let rock = |p: P| !grid.in_bounds(p.x, p.y) || grid.get(p.x, p.y) == Tile::Rock;
        let solid = |p: P| rock(p) || r.facades.contains(&p);
        let gates: Vec<P> = [r.base.entry, r.base.exit].iter().flat_map(|g| [g.tile, g.towers()[0], g.towers()[1]]).collect();
        let censers = r.censers.clone();
        // Fruit trees stand in the bramble beside the open ground, either side of each glade.
        let mut trees: Vec<(P, bool)> = Vec::new();
        for (i, g) in std::iter::once(r.court).chain(r.clearings.iter().copied()).chain(std::iter::once(r.thorn_glade)).enumerate() {
            for (k, p) in [P::new(g.x1 + 1, g.y0 + 1), P::new(g.x0 - 1, g.y1 - 1), P::new(g.x1 + 1, g.y1 - 1)].into_iter().enumerate() {
                if rock(p) && !gates.contains(&p) && !censers.contains(&p) && p.x < W_LIMIT && (i + k) % 3 != 2 {
                    trees.push((p, (i + k) % 2 == 0));
                }
            }
        }
        // The Black Thorn stands in a niche of pale bone gravel in the bramble, where it reads.
        let bt = P::new(r.black_thorn.x.round() as i32, r.black_thorn.y.round() as i32);
        let roots: Vec<P> = (bt.x - 1..=bt.x + 1)
            .flat_map(|x| (bt.y - 1..=bt.y).map(move |y| P::new(x, y)))
            .filter(|p| rock(*p) && !gates.contains(p))
            .collect();
        let tree_tiles: Vec<P> = trees.iter().map(|t| t.0).chain(roots.iter().copied()).collect();
        let mut view = ThornsState {
            hedges: HashMap::new(),
            tree_at: HashMap::new(),
            hedge_trees: Vec::new(),
            glass: HashMap::new(),
            burnt: 0,
            lanterns: Vec::new(),
            spouts: Vec::new(),
            thorns: Vec::new(),
            time: 0.0,
        };

        // ---- the ground
        let ponds: Vec<P> = r.ponds.iter().flat_map(|p| p.tiles.iter().copied()).collect();
        for (x, y, _) in grid.cells() {
            let p = P::new(x, y);
            if (rock(p) && !r.facades.contains(&p) && !censers.contains(&p) && !tree_tiles.contains(&p)) || ponds.contains(&p) {
                continue;
            }
            self.thorns_floor(r, p);
            if r.ground_at(p) == Ground::Thorn {
                view.thorns.push(Vec3::new(x as f32, 0.0, y as f32));
            }
        }
        // Thresholds under the portals, walkable when their doors open.
        for g in [r.base.entry, r.base.exit] {
            let (dx, dy) = g.facing.vec();
            bx(
                &mut self.static_cubes,
                Vec3::new(g.tile.x as f32 - dx as f32 * 0.5, -0.06, g.tile.y as f32 - dy as f32 * 0.5),
                Vec3::new(1.0, 0.1, 1.0),
                Quat::IDENTITY,
                MARBLE_D,
                SHADE_LIT,
            );
        }

        // ---- the hedges: a block on every tile of hedge that faces open ground (and on the hidden
        // paths' facades), each a tree that fades out of the camera's way
        for (x, y, _) in grid.cells() {
            let p = P::new(x, y);
            if !solid(p) || gates.contains(&p) || censers.contains(&p) || tree_tiles.contains(&p) {
                continue;
            }
            let faces = [Dir::Up, Dir::Left, Dir::Down, Dir::Right].map(|d| {
                let n = p.step(d);
                let face = if solid(n) || censers.contains(&n) {
                    Face::None
                } else {
                    match r.ground_at(n) {
                        Ground::Thorn => Face::Bramble,
                        Ground::Bramble => Face::None,
                        _ => Face::Lovely,
                    }
                };
                (d, face)
            });
            if faces.iter().all(|f| f.1 == Face::None) && !r.facades.contains(&p) {
                continue; // the bramble within, capped below
            }
            let start = self.static_cubes.len();
            decor::hedge(&mut self.static_cubes, p, faces);
            let len = self.static_cubes.len() - start;
            view.hedges.insert(p, (start, len));
            let i = self.trees.len();
            self.trees.push(Tree {
                range: (start, len),
                base: Vec3::new(x as f32, 0.0, y as f32),
                radius: 0.72,
                height: HEDGE_H + 0.3,
                fade: 1.0,
                alive: true,
                shades: None,
            });
            view.tree_at.insert(p, i);
            view.hedge_trees.push((i, Vec2::new(x as f32, y as f32)));
        }
        // The bramble within the thickest hedge, black from above: one flat top for each run of it
        // along a row, thorny sprigs and a few skulls on it.
        let inner = |p: P| {
            solid(p)
                && !r.facades.contains(&p)
                && !view.hedges.contains_key(&p)
                && !gates.contains(&p)
                && !censers.contains(&p)
                && !tree_tiles.contains(&p)
        };
        for y in 0..grid.height {
            let mut x = 0;
            while x < grid.width {
                if !inner(P::new(x, y)) {
                    x += 1;
                    continue;
                }
                let x0 = x;
                while x < grid.width && inner(P::new(x, y)) {
                    x += 1;
                }
                let n = (x - x0) as f32;
                bx(
                    &mut self.static_cubes,
                    Vec3::new(x0 as f32 + n / 2.0 - 0.5, HEDGE_H / 2.0 - 0.02, y as f32),
                    Vec3::new(n, HEDGE_H - 0.04, 1.0),
                    Quat::IDENTITY,
                    BRAMBLE,
                    SHADE_LIT,
                );
                for gx in x0..x {
                    decor::bramble_top(&mut self.static_cubes, P::new(gx, y));
                }
            }
        }

        // ---- rose arches over the paths, every few tiles, their lanterns leading the eye on
        for path in &r.paths {
            let (x0, y0, x1, y1) = (path.x0, path.y0, path.x1, path.y1);
            let spots: Vec<(Vec3, Vec3)> = match path.ahead {
                Dir::Up => (y0 + 4..=y1 - 4).step_by(6).map(|y| (Vec3::new(x0 as f32 + 0.5, 0.0, y as f32), Vec3::X)).collect(),
                Dir::Down => (y0 + 4..=path.block - 3).step_by(6).map(|y| (Vec3::new(x1 as f32 - 0.5, 0.0, y as f32), Vec3::X)).collect(),
                Dir::Right => (x0 + 4..=x1 - 4).step_by(6).map(|x| (Vec3::new(x as f32, 0.0, y0 as f32 + 0.5), Vec3::Z)).collect(),
                _ => (x0 + 4..=x1 - 4).step_by(6).map(|x| (Vec3::new(x as f32, 0.0, y1 as f32 - 0.5), Vec3::Z)).collect(),
            };
            for (c, across) in spots {
                let tile = P::new(c.x.round() as i32, c.z.round() as i32);
                if r.ground_at(tile) != Ground::Rose {
                    continue;
                }
                let start = self.static_cubes.len();
                let lamp = decor::arch(&mut self.static_cubes, c, across, (tile.x * 7 + tile.y * 3) as f32);
                view.lanterns.push(lamp);
                let len = self.static_cubes.len() - start;
                self.trees.push(Tree { range: (start, len), base: c, radius: 1.1, height: 2.7, fade: 1.0, alive: true, shades: None });
            }
        }
        // The rose arch from the court into the spiral.
        let (_, _, sx1, sy1) = crate::logic::realms::thorns::ring_bounds(0);
        let gate_arch = Vec3::new(sx1 as f32 + 1.5, 0.0, sy1 as f32 - 1.5);
        for (dx, across) in [(-0.5f32, Vec3::Z), (0.5, Vec3::Z)] {
            let lamp = decor::arch(&mut self.static_cubes, gate_arch + Vec3::new(dx, 0.0, 0.0), across, 3.0 + dx);
            view.lanterns.push(lamp);
        }

        // ---- the props: censers, fountains, bowers, fruit trees, the pavilion, the fairy ring, the Thorn
        for c in &r.censers {
            let at = Vec3::new(c.x as f32, 0.0, c.y as f32);
            let top = decor::censer(&mut self.static_cubes, at);
            self.smoke.push((top, 0.9));
        }
        for pd in &r.ponds {
            let spout = decor::fountain(&mut self.static_cubes, &pd.tiles, pd.golden);
            let c = pd.center();
            view.spouts.push((spout, Vec3::new(c.x as f32, -0.08, c.y as f32)));
        }
        for (i, b) in r.bowers.iter().enumerate() {
            decor::bower(&mut self.static_cubes, Vec3::new(b.at.x as f32, 0.0, b.at.y as f32), i as f32 * 1.7 + 0.3);
        }
        for (k, &(p, peach)) in trees.iter().enumerate() {
            let base = Vec3::new(p.x as f32, 0.0, p.y as f32);
            let start = self.static_cubes.len();
            decor::fruit_tree(&mut self.static_cubes, base, peach, k as f32 * 2.3 + 0.7);
            let len = self.static_cubes.len() - start;
            self.trees.push(Tree { range: (start, len), base, radius: 0.9, height: 2.4, fade: 1.0, alive: true, shades: None });
        }
        decor::pavilion(&mut self.static_cubes, Vec3::new(r.home.x as f32, 0.0, r.home.y as f32));
        for c in &r.dances {
            decor::fairy_ring(&mut self.static_cubes, Vec3::new(c.x as f32, 0.0, c.y as f32), 2.3);
        }
        decor::black_thorn(&mut self.static_cubes, Vec3::new(r.black_thorn.x as f32, 0.0, r.black_thorn.y as f32));

        // ---- perfume mist pooled in the hollows: rose-tinted sheets, drawn every frame
        let mut mist: Vec<&P> = r.mist.iter().collect();
        mist.sort();
        for p in mist {
            for (k, &(lift, alpha)) in [(0.08f32, 0.3f32), (0.24, 0.24), (0.46, 0.15), (0.72, 0.08)].iter().enumerate() {
                let m = Mat4::from_translation(Vec3::new(p.x as f32, lift, p.y as f32)) * Mat4::from_scale(Vec3::new(1.0, 0.002, 1.0));
                self.mist.push(Instance::new(m, lin(0xffb8dc), alpha, Vec3::new(20.0 + k as f32, lift, 0.0), SHADE_CLOUD));
            }
        }
        ThornsView(Box::new(view))
    }

    /// The floor of an open tile (and of a hidden path under its facade): rose-marble flags strewn
    /// with petals on the rose paths, black thorn runners on the thorn paths, marble in the court,
    /// crimson in the heart, moss in the clearings.
    fn thorns_floor(&mut self, r: &Thorns, p: P) {
        let out = &mut self.static_cubes;
        let (x, y) = (p.x, p.y);
        let c = Vec3::new(x as f32, 0.0, y as f32);
        let j = h(x, y, 1);
        let bone_yard = r.thorn_glade.contains(p) || (p.y < r.thorn_glade.y0 && (p.x - r.black_thorn.x.round() as i32).abs() <= 1);
        if bone_yard {
            // Pale gravel and the bones of grooms about the Black Thorn's foot.
            bx(
                out,
                c + Vec3::new(0.0, -0.06, 0.0),
                Vec3::new(1.0, 0.1, 1.0),
                Quat::IDENTITY,
                if j < 0.5 { 0x6a6068 } else { 0x5c525a },
                SHADE_LIT,
            );
            for k in 0..3 {
                let a = h(x, y, 90 + k) * TAU;
                let at = c + Vec3::new(a.cos() * 0.35, 0.01, a.sin() * 0.3);
                bx(out, at, Vec3::new(0.12, 0.04, 0.1), Quat::from_rotation_y(a), [0x7a7078, 0x48404a, 0x8a8088][k as usize], SHADE_LIT);
            }
            if j > 0.8 {
                decor::skull(out, c + Vec3::new(-0.2, 0.0, 0.15), j * 30.0, 0.15);
            } else if j < 0.1 {
                decor::groom(out, c, j * 40.0);
            }
            return;
        }
        match r.ground_at(p) {
            Ground::Rose => {
                bx(out, c + Vec3::new(0.0, -0.06, 0.0), Vec3::new(1.0, 0.1, 1.0), Quat::IDENTITY, GROUT, SHADE_LIT);
                for (k, (sx, sz)) in [(-0.25f32, -0.25f32), (0.25, -0.25), (-0.25, 0.25), (0.25, 0.25)].into_iter().enumerate() {
                    let jj = h(x, y, 2 + k as u32);
                    let col = if jj < 0.3 {
                        FLAG_D
                    } else if jj > 0.85 {
                        MARBLE_W
                    } else {
                        FLAG
                    };
                    bx(out, c + Vec3::new(sx, -0.005, sz), Vec3::new(0.46, 0.02, 0.46), Quat::IDENTITY, col, SHADE_LIT);
                }
                for k in 0..(j * 4.0) as u32 {
                    let a = h(x, y, 10 + k) * TAU;
                    let at = c + Vec3::new(a.cos() * 0.35 * h(x, y, 20 + k), 0.012, a.sin() * 0.35);
                    bx(
                        out,
                        at,
                        Vec3::new(0.08, 0.008, 0.055),
                        Quat::from_rotation_y(a * 3.0),
                        [CRIMSON, RED, PINK][k as usize % 3],
                        SHADE_LIT,
                    );
                }
            }
            Ground::Thorn => {
                bx(
                    out,
                    c + Vec3::new(0.0, -0.06, 0.0),
                    Vec3::new(1.0, 0.1, 1.0),
                    Quat::IDENTITY,
                    if j < 0.5 { EARTH } else { EARTH_D },
                    SHADE_LIT,
                );
                for k in 0..2 {
                    let a = h(x, y, 30 + k) * PI;
                    let at = c + Vec3::new((h(x, y, 40 + k) - 0.5) * 0.5, 0.02, (h(x, y, 50 + k) - 0.5) * 0.5);
                    bx(out, at, Vec3::new(0.85, 0.035, 0.04), Quat::from_rotation_y(a), BRAMBLE_L, SHADE_LIT);
                    for t in [-0.3f32, 0.0, 0.3] {
                        let q = at + Quat::from_rotation_y(a) * Vec3::new(t, 0.03, 0.0);
                        bx(out, q, Vec3::new(0.02, 0.07, 0.02), Quat::from_rotation_z(0.3), THORN, SHADE_LIT);
                    }
                }
                if j > 0.94 {
                    decor::skull(out, c + Vec3::new(0.2, 0.0, -0.1), j * 20.0, 0.15);
                } else if j < 0.03 {
                    decor::groom(out, c, j * 50.0);
                }
            }
            Ground::Glade if r.court.contains(p) => {
                let black = (x + y) % 2 == 0;
                bx(
                    out,
                    c + Vec3::new(0.0, -0.06, 0.0),
                    Vec3::new(0.98, 0.1, 0.98),
                    Quat::IDENTITY,
                    if black { MARBLE_D } else { MARBLE },
                    SHADE_LIT,
                );
                if x == r.court.x0 || x == r.court.x1 || y == r.court.y0 || y == r.court.y1 {
                    bx(
                        out,
                        c + Vec3::new(0.0, -0.005, 0.0),
                        Vec3::new(0.7, 0.012, 0.7),
                        Quat::from_rotation_y(PI / 4.0),
                        GOLD_D,
                        SHADE_LIT,
                    );
                }
            }
            Ground::Glade if r.heart.contains(p) => {
                bx(out, c + Vec3::new(0.0, -0.06, 0.0), Vec3::new(1.0, 0.1, 1.0), Quat::IDENTITY, CARPET, SHADE_LIT);
                for k in 0..3 {
                    let a = h(x, y, 60 + k) * TAU;
                    let at = c + Vec3::new(a.cos() * 0.38 * h(x, y, 70 + k), 0.002, a.sin() * 0.38);
                    bx(out, at, Vec3::new(0.09, 0.01, 0.06), Quat::from_rotation_y(a), [RED, PINK, WHITE][k as usize % 3], SHADE_LIT);
                }
            }
            Ground::Glade => {
                out.push(Instance::new(
                    Mat4::from_scale_rotation_translation(Vec3::new(1.0, 0.1, 1.0), Quat::IDENTITY, c + Vec3::new(0.0, -0.05, 0.0)),
                    lin(MOSS) * (0.9 + j * 0.2),
                    1.0,
                    Vec3::ZERO,
                    SHADE_GRASS,
                ));
                if j > 0.7 {
                    bx(
                        out,
                        c + Vec3::new(j - 0.85, 0.01, 0.2),
                        Vec3::new(0.08, 0.008, 0.06),
                        Quat::from_rotation_y(j * 9.0),
                        PINK,
                        SHADE_LIT,
                    );
                }
            }
            Ground::Ash | Ground::Bramble => {
                bx(out, c + Vec3::new(0.0, -0.06, 0.0), Vec3::new(1.0, 0.1, 1.0), Quat::IDENTITY, EARTH_D, SHADE_LIT);
            }
        }
    }

    /// The realm's own animation, one step: Raphael's burnt paths leave the static scene; the
    /// facades round the smoke (and round him in true sight) turn to glass and back.
    pub(super) fn update_thorns_view(&mut self, v: &mut ThornsView, r: &Thorns, s: &SceneState, dt: f32) {
        v.time += dt;
        // Burnt hedge: gone for good, ash and embers where it stood.
        while v.burnt < r.burnt.len() {
            let p = r.burnt[v.burnt];
            v.burnt += 1;
            if let Some(range) = v.hedges.remove(&p) {
                self.hide_static(range);
            }
            if let Some(i) = v.tree_at.remove(&p) {
                self.trees[i].alive = false;
                v.glass.remove(&i);
            }
            let c = Vec3::new(p.x as f32, 0.0, p.y as f32);
            let out = &mut self.static_cubes;
            bx(out, c + Vec3::new(0.0, -0.06, 0.0), Vec3::new(1.0, 0.1, 1.0), Quat::IDENTITY, 0x241c1c, SHADE_LIT);
            for k in 0..3 {
                let a = h(p.x, p.y, 400 + k) * TAU;
                let at = c + Vec3::new(a.cos() * 0.3, 0.02, a.sin() * 0.3);
                decor::bxg(out, at, Vec3::new(0.12, 0.03, 0.08), Quat::from_rotation_y(a), 0xff6a2a, 0.5);
                decor::beam(out, at, at + Vec3::new(0.05, 0.25, 0.02), 0.04, 0x0e0a0a);
            }
            self.static_dirty = true;
        }
        // Glass: where the smoke is (and, in true sight, round him), the lovely faces thin away.
        let hero = s.hero.pos();
        let mut centres: Vec<(Vec2, f32, f32)> = Vec::new();
        if let Some(sm) = r.smoke {
            let k = ((sm.left as f32) / 0.8).min(((SMOKE_SECONDS - sm.left) as f32) / 0.8).clamp(0.0, 1.0);
            centres.push((Vec2::new(sm.at.x as f32, sm.at.y as f32), SMOKE_REVEAL as f32, k));
        }
        if r.sight > 0.0 {
            centres.push((Vec2::new(hero.x as f32, hero.y as f32), 7.0, (r.sight as f32 / 0.8).min(1.0)));
        }
        let mut want: HashMap<usize, f32> = HashMap::new();
        for &(i, at) in &v.hedge_trees {
            if !self.trees[i].alive && !v.glass.contains_key(&i) {
                continue;
            }
            let g = centres
                .iter()
                .map(|&(c, reach, k)| k * (1.0 - ((at - c).length() - reach * 0.6) / (reach * 0.4)).clamp(0.0, 1.0))
                .fold(0.0, f32::max);
            if g > 0.0 || v.glass.contains_key(&i) {
                want.insert(i, g);
            }
        }
        for (i, goal) in want {
            let cur = v.glass.get(&i).copied().unwrap_or(0.0);
            let next = if goal > cur { (cur + dt * 3.0).min(goal) } else { (cur - dt * 3.0).max(goal) };
            let t = &mut self.trees[i];
            let (start, len) = t.range;
            if next <= 0.001 {
                v.glass.remove(&i);
                t.alive = true;
                let fade = t.fade;
                for inst in &mut self.static_cubes[start..start + len] {
                    inst.color[3] = fade;
                    inst.emissive[3] = if fade >= 0.999 { SHADE_LIT } else { SHADE_DITHER };
                }
            } else {
                v.glass.insert(i, next);
                t.alive = false;
                let alpha = (1.0 - 0.82 * next).min(t.fade);
                for inst in &mut self.static_cubes[start..start + len] {
                    inst.color[3] = alpha;
                    inst.emissive[3] = SHADE_DITHER;
                }
            }
            self.static_patches.push((start, len));
        }
    }

    /// What moves, glows and lights in the realm: the Bride and her moonbeam, the wedding night's
    /// crawling hedges, the dancing elf-maids, the fox's wedding, the koi and the fountains, the
    /// smoke and the honest paths it lights, the lanterns, the bowers' last gear, charm's petals.
    pub(super) fn emit_thorns(&self, v: &ThornsView, r: &Thorns, s: &SceneState, time: f32, hero: Vec3, f: &mut Frame) {
        let cube = MeshKind::Cube as usize;
        let near = |p: Vec3| Vec2::new(p.x - hero.x, p.z - hero.z).length_squared() < NEAR * NEAR;
        let w = |p: crate::logic::types::V| Vec3::new(p.x as f32, 0.0, p.y as f32);

        // ---- the Bride
        if !r.unmade {
            self.emit_bride(r, s, time, f);
        }
        // Where she was: a burst of petals as she goes.
        if r.bride_moved < 0.9 && !r.unmade {
            let at = w(r.bride_from);
            let t = r.bride_moved as f32 / 0.9;
            for k in 0..16 {
                let a = k as f32 * 2.399;
                let p = at + Vec3::new(a.cos() * (0.4 + 1.6 * t), 1.0 + (k % 5) as f32 * 0.5 + t * 0.8, a.sin() * (0.4 + 1.6 * t));
                f.additive.push(Instance::new(
                    Mat4::from_translation(p) * Mat4::from_rotation_y(a + time * 3.0) * Mat4::from_scale(Vec3::new(0.09, 0.02, 0.06)),
                    lin(if k % 3 == 0 { WHITE } else { PINK }),
                    0.8 * (1.0 - t),
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
        }
        // Unmade: her petals settle where she fell, and her light goes out slowly.
        if r.unmade && r.bride_moved < 3.0 {
            let at = w(r.bride_from);
            let t = r.bride_moved as f32 / 3.0;
            for k in 0..40 {
                let a = k as f32 * 2.399;
                let rr = (k as f32 * 0.07) * (0.6 + t);
                let p = at + Vec3::new(a.cos() * rr, (3.0 - k as f32 * 0.07).max(0.02) * (1.0 - t) + 0.02, a.sin() * rr);
                f.additive.push(Instance::new(
                    Mat4::from_translation(p) * Mat4::from_rotation_y(a) * Mat4::from_scale(Vec3::new(0.1, 0.02, 0.07)),
                    lin(if k % 4 == 0 { WHITE } else { PINK }),
                    1.0 - t,
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
        }

        // ---- the wedding night: the garden collapses toward her
        if let Some(wd) = r.wedding {
            let k = (wd.t / WEDDING_SECONDS) as f32;
            let her = w(r.bride);
            let g = r.thorn_glade;
            for i in 0..14 {
                let a = i as f32 / 14.0 * TAU + 0.3;
                let from = Vec3::new((g.x0 + g.x1) as f32 / 2.0 + a.cos() * 7.5, 0.0, (g.y0 + g.y1) as f32 / 2.0 + 2.0 + a.sin() * 6.0);
                let at = from.lerp(her, (k * 0.7).min(0.7)) + Vec3::new((time * 3.0 + i as f32).sin() * 0.06, 0.0, 0.0);
                let lean = Quat::from_rotation_y(a) * Quat::from_rotation_z((time * 4.0 + i as f32).sin() * 0.12);
                for (c, (dx, dy, dz)) in [
                    (LEAF_D, (0.0f32, 0.45f32, 0.0f32)),
                    (LEAF, (0.2, 0.7, 0.1)),
                    (LEAF_L, (-0.2, 0.75, -0.1)),
                    (LEAF_MOON, (0.0, 1.0, 0.05)),
                ] {
                    f.opaque[cube].push(Instance::new(
                        Mat4::from_scale_rotation_translation(Vec3::new(0.62, 0.55, 0.6), lean, at + lean * Vec3::new(dx, dy, dz)),
                        lin(c),
                        1.0,
                        Vec3::ZERO,
                        SHADE_LIT,
                    ));
                }
                for (j, c) in [CRIMSON, PINK].into_iter().enumerate() {
                    let p = at + lean * Vec3::new(0.25 - j as f32 * 0.5, 0.95, 0.3);
                    f.opaque[cube].push(Instance::new(
                        Mat4::from_scale_rotation_translation(Vec3::splat(0.14), lean, p),
                        lin(c),
                        1.0,
                        lin(c) * 0.15,
                        SHADE_LIT,
                    ));
                }
                // Roots wriggling under it, dragging it on.
                for j in 0..4 {
                    let ra = a + j as f32 * 1.57 + (time * 6.0 + i as f32 + j as f32).sin() * 0.4;
                    let tip = at + Vec3::new(ra.cos() * 0.55, 0.03, ra.sin() * 0.55);
                    let mid = (at + tip) * 0.5 + Vec3::new(0.0, 0.08, 0.0);
                    let d = tip - at;
                    f.opaque[cube].push(Instance::new(
                        Mat4::from_scale_rotation_translation(
                            Vec3::new(0.05, 0.05, d.length()),
                            Quat::from_rotation_arc(Vec3::Z, d.normalize_or(Vec3::Z)),
                            mid,
                        ),
                        lin(BRAMBLE),
                        1.0,
                        Vec3::ZERO,
                        SHADE_LIT,
                    ));
                }
            }
        }

        // ---- Elveskud: elf-maids dancing in a ring, their lovely faces outward, beckoning
        let maid = &self.models.foes_far[RigId::Ellepige as usize];
        for c in &r.dances {
            let centre = w(*c);
            if !near(centre) {
                continue;
            }
            for k in 0..7 {
                let a = time * 0.55 + k as f32 / 7.0 * TAU;
                let at = centre + Vec3::new(a.cos() * 1.7, (time * 4.0 + k as f32).sin().abs() * 0.06, a.sin() * 1.7);
                let yaw = a.cos().atan2(a.sin());
                let mut pose = Pose::rest(maid);
                for (name, side) in [("leftArm", 1.0f32), ("rightArm", -1.0)] {
                    if let Some(i) = maid.index(name) {
                        pose.rot[i].z = side * (1.2 + (time * 3.0 + k as f32).sin() * 0.15);
                    }
                }
                pose.rot_x(maid, "leftLeg", (time * 4.0 + k as f32).sin() * 0.3);
                pose.rot_x(maid, "rightLeg", -(time * 4.0 + k as f32).sin() * 0.3);
                let root = Mat4::from_translation(at) * Mat4::from_rotation_y(yaw);
                emit_rig(&mut f.opaque[cube], maid, &pose, root, Vec3::splat(0.9), 1.0, lin(0xf0e0ea) * 0.03);
            }
            glow_orb(f, centre + Vec3::new(0.0, 0.3, 0.0), 2.4, lin(0xd8c8ff), 0.06 + 0.02 * (time * 2.0).sin());
            f.lights.push(Light { pos: centre + Vec3::new(0.0, 1.6, 0.0), color: lin(0xd8c8ff) * 1.2, distance: 4.5 });
        }

        // ---- the fox's wedding: a foxfire procession crossing the court, now and then
        let fox = &self.models.foes_far[RigId::Kitsune as usize];
        let cycle = (time / 34.0).fract() * 34.0;
        if cycle < 16.0 {
            let (a, b) = (w(r.procession.0), w(r.procession.1));
            for k in 0..7 {
                let t = (cycle - k as f32 * 0.9) / 14.0;
                if !(0.0..=1.0).contains(&t) {
                    continue;
                }
                let at = a.lerp(b, t);
                if !near(at) {
                    continue;
                }
                let dir = (b - a).normalize_or(Vec3::X);
                let fade = (t * 8.0).min((1.0 - t) * 8.0).min(1.0);
                let mut pose = Pose::rest(fox);
                let step = time * 9.0 + k as f32;
                pose.rot_x(fox, "leftLeg", step.sin() * 0.5);
                pose.rot_x(fox, "rightLeg", -step.sin() * 0.5);
                let root =
                    Mat4::from_translation(at) * Mat4::from_rotation_y(dir.x.atan2(dir.z)) * Mat4::from_scale(Vec3::splat(0.6 * fade));
                emit_rig(&mut f.opaque[cube], fox, &pose, root, Vec3::ONE, 1.0, lin(0xffa040) * 0.15);
                let flame = at + Vec3::new(0.0, 0.85 + (time * 3.0 + k as f32).sin() * 0.08, 0.0);
                glow_orb(f, flame, 0.32, lin(0x7ae8ff), 0.5 * fade);
                f.additive.push(Instance::new(
                    Mat4::from_translation(flame) * Mat4::from_scale(Vec3::new(0.07, 0.12, 0.07)),
                    lin(0xb8f4ff),
                    0.9 * fade,
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
                if k % 2 == 0 {
                    f.lights.push(Light { pos: flame, color: lin(0x7ae8ff) * 2.0 * fade, distance: 3.0 });
                }
            }
        }

        // ---- the koi, the fountains' jets, and the lanterns mirrored in the water (but never her)
        for (i, pd) in r.ponds.iter().enumerate() {
            let (spout, mid) = v.spouts[i];
            if !near(mid) {
                continue;
            }
            for k in 0..12 {
                let t = ((time * 0.9 + k as f32 / 12.0) % 1.0).abs();
                let p = spout.lerp(mid, t) + Vec3::new(0.0, (t * PI).sin() * 0.5 - t * 0.6, 0.0);
                f.additive.push(Instance::new(
                    Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.035)),
                    lin(0xcfe4ff),
                    0.45,
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
            let fish = if pd.empty > 0.0 { 1 } else { 3 };
            for k in 0..fish {
                let a = time * (0.6 + k as f32 * 0.2) + k as f32 * 2.1 + i as f32;
                let rr = 0.35 + k as f32 * 0.12;
                let p = mid + Vec3::new(a.cos() * rr, 0.025, a.sin() * rr);
                let yaw = (-a.sin()).atan2(a.cos()) + PI / 2.0;
                let golden = pd.golden && !r.golden_taken && k == 0;
                let (body, spot) = if golden {
                    (0xffc830, 0xfff0a0)
                } else if k == 1 {
                    (0xf4f0ea, 0xff6a2a)
                } else {
                    (0xff7a2a, 0xf8f0e8)
                };
                let rot = Quat::from_rotation_y(yaw);
                let glow = if golden { 0.8 } else { 0.1 };
                f.opaque[cube].push(Instance::new(
                    Mat4::from_scale_rotation_translation(Vec3::new(0.08, 0.04, 0.2), rot, p),
                    lin(body),
                    1.0,
                    lin(body) * glow,
                    SHADE_LIT,
                ));
                f.opaque[cube].push(Instance::new(
                    Mat4::from_scale_rotation_translation(Vec3::new(0.06, 0.045, 0.07), rot, p + rot * Vec3::new(0.0, 0.004, 0.03)),
                    lin(spot),
                    1.0,
                    lin(spot) * glow,
                    SHADE_LIT,
                ));
                let tail = rot * Quat::from_rotation_y((time * 8.0 + k as f32).sin() * 0.5);
                f.opaque[cube].push(Instance::new(
                    Mat4::from_scale_rotation_translation(Vec3::new(0.1, 0.012, 0.07), tail, p - rot * Vec3::new(0.0, 0.0, 0.13)),
                    lin(body),
                    1.0,
                    lin(body) * glow,
                    SHADE_LIT,
                ));
                if golden {
                    glow_orb(f, p, 0.4, lin(0xffd84a), 0.35);
                    f.lights.push(Light { pos: p + Vec3::new(0.0, 0.4, 0.0), color: lin(0xffd84a) * 2.0, distance: 3.0 });
                }
            }
            for l in nearest(&v.lanterns, mid, 2) {
                if (l - mid).length() < 5.0 {
                    let mirrored = Vec3::new(l.x * 0.25 + mid.x * 0.75, -0.075, l.z * 0.25 + mid.z * 0.75);
                    glow_orb(f, mirrored, 0.18, lin(ROSE_GLASS), 0.25);
                }
            }
        }

        // ---- the smoke of a burnt fish, and the honest paths it lights
        if let Some(sm) = r.smoke {
            let at = w(sm.at);
            let k = ((sm.left as f32) / 1.0).min(((SMOKE_SECONDS - sm.left) as f32) / 0.6).clamp(0.0, 1.0);
            let sphere = MeshKind::Sphere as usize;
            for i in 0..30 {
                let fi = i as f32;
                let t = (time * 0.16 + fi / 30.0).fract();
                let a = fi * 2.399 + time * 0.25;
                let rr = 0.3 + (fi * 0.37).fract() * 1.5;
                let p = at + Vec3::new(a.cos() * rr, 0.9 + t * 2.2, a.sin() * rr);
                let size = 0.25 + t * 0.45;
                let alpha = 0.09 * k * (PI * t).sin();
                f.transparent[sphere].push(Instance::new(
                    Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(size)),
                    lin(0xd8d0e6),
                    alpha,
                    Vec3::ZERO,
                    SHADE_LIT,
                ));
            }
            for (i, p) in v.thorns.iter().enumerate() {
                let d = Vec2::new(p.x - at.x, p.z - at.z).length();
                if d > SMOKE_REVEAL as f32 + 6.0 {
                    continue;
                }
                let pulse = 0.5 + 0.5 * (time * 3.0 - d * 0.6).sin();
                let reach = (1.0 - (d / (SMOKE_REVEAL as f32 + 6.0))).clamp(0.0, 1.0);
                let fi = i as f32;
                for e in 0..3 {
                    let fe = fi * 3.0 + e as f32;
                    let q = *p
                        + Vec3::new((fe * 0.73).fract() - 0.5, 0.06 + (time * 0.8 + fe).sin().abs() * 0.08, (fe * 0.37).fract() - 0.5)
                            * 0.8;
                    f.additive.push(Instance::new(
                        Mat4::from_translation(q) * Mat4::from_scale(Vec3::splat(0.05)),
                        lin(0xffc070) * 1.5,
                        0.8 * k * reach * (0.4 + pulse),
                        Vec3::ZERO,
                        SHADE_UNLIT,
                    ));
                }
            }
            f.lights.push(Light { pos: at + Vec3::new(0.0, 1.5, 0.0), color: lin(0xffc8a0) * 3.0 * k, distance: 6.0 });
        }

        // ---- the lanterns: rose glass glowing along the paths; the nearest light them
        let mut lit: Vec<Vec3> = Vec::new();
        for (i, l) in v.lanterns.iter().enumerate() {
            if !near(*l) {
                continue;
            }
            lit.push(*l);
            glow_orb(f, *l, 0.42, lin(ROSE_GLASS), 0.22 + 0.03 * (time * 2.3 + i as f32).sin());
        }
        for (i, p) in nearest(&lit, hero, 6).into_iter().enumerate() {
            let flick = 2.8 + (time * 5.0 + i as f32 * 1.3).sin() * 0.25;
            f.lights.push(Light { pos: p, color: lin(ROSE_GLASS) * flick, distance: 5.0 });
        }

        // ---- the grooms' last gear, glowing in the bowers until it is taken
        for b in &r.bowers {
            if b.taken {
                continue;
            }
            let at = Vec3::new(b.at.x as f32, 0.45 + (time * 2.0 + b.at.x as f32).sin() * 0.04, b.at.y as f32);
            if !near(at) {
                continue;
            }
            let c = lin(b.rarity.color());
            glow_orb(f, at, 0.3, c, 0.3);
            f.additive.push(Instance::new(
                Mat4::from_translation(at + Vec3::new(0.0, 1.2, 0.0)) * Mat4::from_scale(Vec3::new(0.05, 2.4, 0.05)),
                c,
                0.1,
                Vec3::ZERO,
                SHADE_UNLIT,
            ));
        }

        // ---- charm: petals drawn from him toward her, the more the stronger; a whirl when enthralled
        let charm = (r.charm / CHARM_MAX) as f32;
        if !r.unmade && charm > 0.15 {
            let her = w(r.bride) + Vec3::new(0.0, 1.6, 0.0);
            let from = hero + Vec3::new(0.0, 0.8, 0.0);
            let n = (charm * 18.0) as usize;
            for k in 0..n {
                let t = (time * 0.35 + k as f32 / n as f32).fract();
                let p = from.lerp(her, t * 0.6)
                    + Vec3::new((time * 2.0 + k as f32).sin() * 0.3, (t * PI).sin() * 0.5, (time * 1.7 + k as f32).cos() * 0.3);
                f.additive.push(Instance::new(
                    Mat4::from_translation(p)
                        * Mat4::from_rotation_y(time * 4.0 + k as f32)
                        * Mat4::from_scale(Vec3::new(0.08, 0.015, 0.055)),
                    lin(PINK),
                    0.7 * charm * (1.0 - t),
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
        }
        if r.enthralled > 0.0 {
            for k in 0..20 {
                let a = time * 3.0 + k as f32 * 0.314;
                let p = hero + Vec3::new(a.cos() * 0.8, 0.3 + (k % 5) as f32 * 0.3, a.sin() * 0.8);
                f.additive.push(Instance::new(
                    Mat4::from_translation(p) * Mat4::from_rotation_y(a) * Mat4::from_scale(Vec3::new(0.09, 0.02, 0.06)),
                    lin(PINK),
                    0.85,
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
            }
            glow_orb(f, hero + Vec3::new(0.0, 0.7, 0.0), 1.3, lin(PINK), 0.25);
        }
    }

    /// The Bride: the elf-maid many times over, veiled in white, in her moonbeam. In the garden she
    /// rises out of the roses ahead of him, singing, her lovely front to the camera (in true sight,
    /// her hollow back); on the wedding night she walks the ground toward him, arms out.
    fn emit_bride(&self, r: &Thorns, s: &SceneState, time: f32, f: &mut Frame) {
        let cube = MeshKind::Cube as usize;
        let rig = &self.models.ellepige;
        let wedding = r.wedding.is_some();
        // On open ground (her dais in the court) she stands on a pale round dais; in the garden she
        // rises out of the roses of the hedge.
        let tile = P::new(r.bride.x.round() as i32, r.bride.y.round() as i32);
        let dais = !wedding && s.grid.is_walkable(tile.x, tile.y, false);
        let home = !wedding && (r.bride.x - r.home.x).abs() < 0.01 && (r.bride.y - r.home.y).abs() < 0.01;
        let feet = if wedding {
            0.0
        } else if home {
            HEDGE_H + 0.26
        } else if dais {
            0.36
        } else {
            0.25
        };
        let at = Vec3::new(r.bride.x as f32, feet, r.bride.y as f32);
        let facing = if wedding {
            r.bride_facing
        } else if r.sight > 0.0 {
            crate::logic::types::V::new(0.0, -1.0)
        } else {
            r.bride_facing
        };
        let yaw = (facing.x as f32).atan2(facing.y as f32);
        let appear = if wedding { 1.0 } else { (r.bride_moved as f32 / APPEAR).min(1.0) };
        if dais {
            for (k, (w, y)) in [(1.5f32, 0.06f32), (1.25, 0.18), (1.0, 0.3)].into_iter().enumerate() {
                for turn in [0.0f32, PI / 4.0] {
                    let m = Mat4::from_scale_rotation_translation(
                        Vec3::new(w, 0.12, w),
                        Quat::from_rotation_y(turn),
                        at - Vec3::new(0.0, feet - y, 0.0),
                    );
                    let c = lin([MARBLE_D, MARBLE, MARBLE_W][k]);
                    f.transparent[cube].push(Instance::new(m, c, appear, c * 0.08, SHADE_LIT));
                }
            }
        }
        let mut pose = Pose::rest(rig);
        if wedding {
            // Walking up to him, her arms held out for him.
            let step = time * 5.0;
            pose.rot_x(rig, "leftLeg", step.sin() * 0.25);
            pose.rot_x(rig, "rightLeg", -step.sin() * 0.25);
            pose.rot_x(rig, "leftArm", -1.25 + (time * 1.3).sin() * 0.08);
            pose.rot_x(rig, "rightArm", -1.25 - (time * 1.3).sin() * 0.08);
        } else {
            // Singing: her arms a little open, swaying, her head tilting to the song.
            for (name, side) in [("leftArm", -1.0f32), ("rightArm", 1.0)] {
                if let Some(i) = rig.index(name) {
                    pose.rot[i].x = -0.45 + (time * 0.9 + side).sin() * 0.1;
                    pose.rot[i].z = -side * (0.35 + (time * 0.7).sin() * 0.06);
                }
            }
            if let Some(i) = rig.index("head") {
                pose.rot[i].z = (time * 0.6).sin() * 0.12;
                pose.rot[i].x = -0.12;
            }
        }
        let sway = Mat4::from_rotation_z((time * 0.8).sin() * 0.03);
        let root = Mat4::from_translation(at) * Mat4::from_rotation_y(yaw) * sway * Mat4::from_scale(Vec3::splat(BRIDE_SCALE));
        let glow = if r.sight > 0.0 && !wedding { 0.0 } else { 0.035 };
        let target = if appear >= 1.0 { &mut f.opaque[cube] } else { &mut f.transparent[cube] };
        emit_rig(target, rig, &pose, root, Vec3::splat(0.86), appear, lin(0xfff0f6) * glow * appear);
        // The veil: sheer white from her crown, trailing down her back to the ground behind her.
        let head = root * Mat4::from_translation(Vec3::new(0.0, 1.12, 0.0));
        for k in 0..5 {
            let x = (k as f32 - 2.0) * 0.045;
            let len = 1.12 - (k as f32 - 2.0).abs() * 0.05;
            let m = head
                * Mat4::from_translation(Vec3::new(x * 0.8, -0.03, -0.07))
                * Mat4::from_rotation_x(-0.18 - (time * 0.9 + k as f32).sin() * 0.02)
                * Mat4::from_translation(Vec3::new(0.0, -len / 2.0, -0.02))
                * Mat4::from_scale(Vec3::new(0.05 + len * 0.02, len, 0.008));
            f.transparent[cube].push(Instance::new(m, lin(0xfaf6fa), 0.26 * appear, lin(0xfaf6fa) * 0.06, SHADE_LIT));
        }
        // The moonbeam she stands in, its pool of light, and her own light: the brightest in the garden.
        let beam_k = if wedding { 0.5 } else { 1.0 } * appear;
        // (The beam comes down to her crown: it lights her, it does not wash her out.)
        for (wd, a) in [(1.6f32, 0.03f32), (0.7, 0.045)] {
            let m = Mat4::from_translation(at + Vec3::new(0.0, 3.3 + 13.0, 0.0)) * Mat4::from_scale(Vec3::new(wd, 26.0, wd));
            f.additive.push(Instance::new(m, lin(0xd0dcff), a * beam_k, Vec3::ZERO, SHADE_UNLIT));
        }
        let ring = Mat4::from_translation(at + Vec3::new(0.0, if wedding || dais || home { 0.03 } else { HEDGE_H - 0.2 }, 0.0))
            * Mat4::from_rotation_x(-PI / 2.0)
            * Mat4::from_scale(Vec3::splat(1.1));
        f.transparent[MeshKind::Ring as usize].push(Instance::new(ring, lin(0xdde6ff), 0.35 * beam_k, Vec3::ZERO, SHADE_UNLIT));
        glow_orb(f, at + Vec3::new(0.0, 3.5, 0.0), 1.2, lin(0xffeef8), 0.05 * appear);
        f.lights.push(Light { pos: at + Vec3::new(0.0, 3.0, 2.6), color: lin(0xfff0f8) * 1.6 * appear, distance: 6.5 });
        // Petals drifting down round her as she sings.
        for k in 0..10 {
            let t = (time * 0.15 + k as f32 / 10.0).fract();
            let a = k as f32 * 2.399 + time * 0.4;
            let p = at + Vec3::new(a.cos() * 1.1, 3.4 - t * 3.2, a.sin() * 1.1);
            f.additive.push(Instance::new(
                Mat4::from_translation(p) * Mat4::from_rotation_y(a * 2.0) * Mat4::from_scale(Vec3::new(0.09, 0.02, 0.06)),
                lin(if k % 3 == 0 { WHITE } else { PINK }),
                0.7 * appear * (1.0 - t),
                Vec3::ZERO,
                SHADE_UNLIT,
            ));
        }
    }
}

impl std::ops::Deref for ThornsView {
    type Target = ThornsState;
    fn deref(&self) -> &ThornsState {
        &self.0
    }
}

impl std::ops::DerefMut for ThornsView {
    fn deref_mut(&mut self) -> &mut ThornsState {
        &mut self.0
    }
}

/// Fruit trees keep off the garden's eastern edge.
const W_LIMIT: i32 = crate::logic::realms::thorns::W - 2;

/// The realm's sky, fog and light: silver moonlight on crimson; darker on the wedding night, when
/// the moon goes behind the Thorn; clearer in the smoke and in true sight.
pub(super) fn atmosphere(_v: &ThornsView, r: &Thorns, _s: &SceneState) -> Atmosphere {
    let mut a = Atmosphere {
        background: 0x1a0a16,
        fog_near: 16.0,
        fog_far: 44.0,
        hemi_sky: lin(0x8a98d0) * 0.5,
        hemi_ground: lin(0x4a1a30) * 0.42,
        ambient: lin(0x50304c) * 0.5,
        sun_dir: Vec3::new(-0.35, 1.0, 0.55).normalize(),
        sun: lin(0xc8d4ff) * 1.15,
        exposure: 1.18,
        night_sky: 1.0,
        bloom: 1.25,
        moon_view: Vec3::new(-0.4, 0.5, -1.0).normalize(),
        cloud_tint: lin(0x24101c),
    };
    if r.wedding.is_some() {
        a.sun *= 0.55;
        a.hemi_sky *= 0.6;
        a.background = 0x140610;
        a.ambient = lin(0x5a1a34) * 0.45;
    }
    if r.smoke.is_some() || r.sight > 0.0 {
        a.ambient *= 1.25;
        a.fog_far += 8.0;
    }
    a
}
