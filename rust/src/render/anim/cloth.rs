//! Cloth: a hero's cloak cut out of his jointed rig into a grid of patches that a small cloth
//! simulation carries (`Cloth`). The grid's points hang from his chest, fall under their weight,
//! trail behind him as he runs and turns, swing with his spins and lift in his leaps; they are
//! kept off his back and his thighs. Each patch of the cloak's voxels rides on the four points
//! round it, so the cloak stays voxels and still flows. It is presentation only: nothing in the
//! fight reads it, and it runs on the frame's own time.

use glam::{Mat4, Vec3};

use super::skeleton::{Bone, BONE_COUNT};
use crate::logic::heroes::HeroKind;
use crate::render::voxels::{lin, Rig, RigPart};

/// Patches across the cloak and down it.
pub const COLS: usize = 6;
pub const ROWS: usize = 6;
const CELLS: usize = COLS * ROWS;

/// The patches' part names (a rig part's name is static).
const NAMES: [&str; CELLS] = [
    "cape.0", "cape.1", "cape.2", "cape.3", "cape.4", "cape.5", "cape.6", "cape.7", "cape.8", "cape.9", "cape.10", "cape.11", "cape.12",
    "cape.13", "cape.14", "cape.15", "cape.16", "cape.17", "cape.18", "cape.19", "cape.20", "cape.21", "cape.22", "cape.23", "cape.24",
    "cape.25", "cape.26", "cape.27", "cape.28", "cape.29", "cape.30", "cape.31", "cape.32", "cape.33", "cape.34", "cape.35",
];

/// Which voxels of a hero are his cloak: its colours, in a box of his sculpt's grid (voxel
/// centres, x as sculpted).
pub struct CapeSpec {
    pub colors: &'static [u32],
    pub lo: [f32; 3],
    pub hi: [f32; 3],
}

/// A hero's cloak, if he wears one that flows. (None does now: Ragnar's cloak was taken off on
/// 2026-10-06; the cloth stays for the characters that will wear one: a spec names its colours
/// and the box behind the hero they hang in, as the tests' cloak does.)
pub fn spec(hero: HeroKind) -> Option<&'static CapeSpec> {
    match hero {
        HeroKind::Ragnar | HeroKind::Mehmed | HeroKind::Miyamoto => None,
    }
}

/// The bones the cloak can be cut from (it hangs down his back, past his hips).
const FROM: [Bone; 7] = [Bone::Hips, Bone::Spine, Bone::Chest, Bone::ThighL, Bone::ThighR, Bone::ShinL, Bone::ShinR];

/// The cloak's grid at rest, in the model: `ROWS + 1` lines down it, `COLS + 1` points along each,
/// spread over the cloak's width at that height (it flares toward the hem).
#[derive(Clone, Debug)]
struct Grid {
    pts: Vec<Vec3>,
    top: f32,
    bottom: f32,
}

impl Grid {
    fn of(points: &[Vec3], size: f32) -> Self {
        let top = points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max) + size * 0.5;
        let bottom = points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min) - size * 0.5;
        let band = (top - bottom) / ROWS as f32 * 0.75;
        let mut pts = Vec::with_capacity((ROWS + 1) * (COLS + 1));
        for j in 0..=ROWS {
            let y = top - (top - bottom) * j as f32 / ROWS as f32;
            // The cloak's width and depth about this line (the nearest voxels if none lie in it).
            let near: Vec<Vec3> = points.iter().copied().filter(|p| (p.y - y).abs() <= band).collect();
            let near = if near.is_empty() { points.to_vec() } else { near };
            let x0 = near.iter().map(|p| p.x).fold(f32::INFINITY, f32::min) - size * 0.5;
            let x1 = near.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max) + size * 0.5;
            let z = near.iter().map(|p| p.z).sum::<f32>() / near.len() as f32;
            for i in 0..=COLS {
                pts.push(Vec3::new(x0 + (x1 - x0) * i as f32 / COLS as f32, y, z));
            }
        }
        Self { pts, top, bottom }
    }

    fn at(&self, i: usize, j: usize) -> Vec3 {
        self.pts[j * (COLS + 1) + i]
    }

    /// The patch a point of the cloak at rest falls in.
    fn cell(&self, p: Vec3) -> usize {
        let v = ((self.top - p.y) / (self.top - self.bottom) * ROWS as f32).clamp(0.0, ROWS as f32 - 1e-3);
        let j = v as usize;
        let f = v - j as f32;
        let (a0, a1) = (self.at(0, j).x, self.at(COLS, j).x);
        let (b0, b1) = (self.at(0, j + 1).x, self.at(COLS, j + 1).x);
        let (x0, x1) = (a0 + (b0 - a0) * f, a1 + (b1 - a1) * f);
        let i = ((p.x - x0) / (x1 - x0) * COLS as f32).clamp(0.0, COLS as f32 - 1e-3) as usize;
        j * COLS + i
    }

    /// A patch's corners: top left, top right, bottom left, bottom right (as the grid runs).
    fn corners(cell: usize) -> [usize; 4] {
        let (i, j) = (cell % COLS, cell / COLS);
        let k = |i: usize, j: usize| j * (COLS + 1) + i;
        [k(i, j), k(i + 1, j), k(i, j + 1), k(i + 1, j + 1)]
    }
}

/// A patch's frame from its four corners: centred on them, its x across the cloak, its y up it,
/// stretched as far as the corners are (`rest` gives the lengths that count as unstretched).
fn frame(c: [Vec3; 4], rest: Option<(f32, f32)>) -> (Mat4, f32, f32) {
    let across = (c[1] + c[3] - c[0] - c[2]) * 0.5;
    let up = (c[0] + c[1] - c[2] - c[3]) * 0.5;
    let centre = (c[0] + c[1] + c[2] + c[3]) * 0.25;
    let x = across.normalize_or(Vec3::X);
    let z = x.cross(up).normalize_or(Vec3::Z);
    let y = z.cross(x);
    let (lx, ly) = (across.length(), up.length());
    let (sx, sy) = match rest {
        Some((rx, ry)) => ((lx / rx).clamp(0.8, 1.25), (ly / ry).clamp(0.8, 1.25)),
        None => (1.0, 1.0),
    };
    (Mat4::from_cols((x * sx).extend(0.0), (y * sy).extend(0.0), z.extend(0.0), centre.extend(1.0)), lx, ly)
}

/// A patch's voxels (each place and colour) and their shade modes.
type Patch = (Vec<(Vec3, Vec3)>, Vec<f32>);

/**
 * The jointed rig `rig` with its cloak (the voxels `spec` names, on the bones it hangs down) cut
 * out into `ROWS × COLS` patches, appended after the skeleton's bones as parts named `cape.N`,
 * each hanging from the chest (for the export, which folds them back). `width` and `depth` are the
 * sculpt's grid. A rig with no such voxels comes back as it was.
 */
pub fn split(mut rig: Rig, spec: &CapeSpec, width: i32, depth: i32) -> Rig {
    let size = rig.size;
    let (ox, oz) = ((width - 1) as f32 / 2.0, (depth - 1) as f32 / 2.0);
    let colors: Vec<Vec3> = spec.colors.iter().map(|&c| lin(c)).collect();
    let mut cloak: Vec<(Vec3, Vec3, f32)> = Vec::new();
    for bone in FROM {
        let Some(part) = rig.parts.get_mut(bone.index()) else { continue };
        let rest = part.rest;
        let mut keep = (Vec::new(), Vec::new());
        for (k, (off, c)) in part.cubes.iter().enumerate() {
            let at = rest + *off;
            let g = [ox - at.x / size, at.y / size - 0.5, at.z / size + oz];
            let inside = (0..3).all(|a| g[a] >= spec.lo[a] && g[a] <= spec.hi[a]);
            let shade = part.shade.get(k).copied().unwrap_or(0.0);
            if inside && colors.iter().any(|w| shade_of(*c, *w)) {
                cloak.push((at, *c, shade));
            } else {
                keep.0.push((*off, *c));
                keep.1.push(shade);
            }
        }
        (part.cubes, part.shade) = keep;
        part.min_y = part.cubes.iter().map(|(p, _)| p.y - size / 2.0).fold(f32::INFINITY, f32::min);
        if !part.min_y.is_finite() {
            part.min_y = 0.0;
        }
    }
    if cloak.is_empty() {
        return rig;
    }
    let grid = Grid::of(&cloak.iter().map(|v| v.0).collect::<Vec<_>>(), size);
    let mut cells: Vec<Patch> = vec![(Vec::new(), Vec::new()); CELLS];
    let centres: Vec<Vec3> = (0..CELLS).map(|n| Grid::corners(n).iter().map(|&k| grid.pts[k]).sum::<Vec3>() * 0.25).collect();
    for (at, c, shade) in cloak {
        let n = grid.cell(at);
        cells[n].0.push((at - centres[n], c));
        cells[n].1.push(shade);
    }
    let mut parts = std::mem::take(&mut rig.parts);
    for (n, (cubes, shade)) in cells.into_iter().enumerate() {
        let min_y = cubes.iter().map(|(p, _)| p.y - size / 2.0).fold(f32::INFINITY, f32::min);
        parts.push(RigPart {
            name: NAMES[n],
            rest: centres[n],
            cubes,
            shade,
            min_y: if min_y.is_finite() { min_y } else { 0.0 },
            parent: Some(Bone::Chest.index()),
        });
    }
    Rig::new(size, parts)
}

/// Whether `c` is colour `of` as the sculpt left it: each voxel's colour is scaled a few percent
/// up or down (`characters::Sculpt::finish`), so its channels keep their proportions.
fn shade_of(c: Vec3, of: Vec3) -> bool {
    let r = c / of.max(Vec3::splat(1e-4));
    let mean = (r.x + r.y + r.z) / 3.0;
    (0.6..1.6).contains(&mean) && (r - Vec3::splat(mean)).abs().max_element() < mean * 0.15
}

/// Whether a part is a patch of a cloak (`split`).
pub fn is_patch(part: &RigPart) -> bool {
    part.name.starts_with("cape.")
}

/// How the cloak hangs and what it may not pass through, worked out once from the rig at rest.
#[derive(Clone, Debug)]
pub struct Cape {
    /// The first patch's part.
    first: usize,
    /// Each patch's place at rest.
    centre: Vec<Vec3>,
    rest: Vec<Vec3>,
    /// Each patch's rest frame inverted, times its rest place (its voxels are kept relative to
    /// that), and its unstretched lengths.
    bind: Vec<(Mat4, f32, f32)>,
    /// The top line's points, each held to the chest where it lies at rest.
    pins: Vec<(usize, Vec3)>,
    /// Lengths to keep: points, rest length, stiffness.
    links: Vec<(usize, usize, f32, f32)>,
    /// Each point's body bone and how far behind it the point hangs at rest.
    behind: Vec<(usize, f32)>,
    /// His thighs: joint to knee, and their girth.
    thighs: [(usize, usize); 2],
    girth: f32,
}

impl Cape {
    /// The cloak of a jointed rig cut by `split`, if it has one.
    pub fn of(rig: &Rig) -> Option<Self> {
        let first = rig.parts.iter().position(is_patch)?;
        let size = rig.size;
        let points: Vec<Vec3> = rig.parts[first..].iter().flat_map(|p| p.cubes.iter().map(move |(o, _)| p.rest + *o)).collect();
        if points.is_empty() {
            return None;
        }
        let grid = Grid::of(&points, size);
        let joint = |b: Bone| rig.parts[b.index()].rest;
        let bind = (0..CELLS)
            .map(|n| {
                let c = Grid::corners(n).map(|k| grid.pts[k]);
                let (f, lx, ly) = frame(c, None);
                (f.inverse() * Mat4::from_translation(rig.parts[first + n].rest), lx, ly)
            })
            .collect();
        let pins = (0..=COLS).map(|i| (i, grid.pts[i] - joint(Bone::Chest))).collect::<Vec<_>>();
        let mut links = Vec::new();
        let k = |i: usize, j: usize| j * (COLS + 1) + i;
        for j in 0..=ROWS {
            for i in 0..=COLS {
                let mut link = |a: usize, b: usize, stiff: f32| links.push((a, b, grid.pts[a].distance(grid.pts[b]), stiff));
                if i < COLS {
                    link(k(i, j), k(i + 1, j), 1.0);
                }
                if j < ROWS {
                    link(k(i, j), k(i, j + 1), 1.0);
                }
                if i < COLS && j < ROWS {
                    link(k(i, j), k(i + 1, j + 1), 0.5);
                    link(k(i + 1, j), k(i, j + 1), 0.5);
                }
                // (Bending: a little stiffness over two patches, so it folds rather than crumples.)
                if i + 2 <= COLS {
                    link(k(i, j), k(i + 2, j), 0.15);
                }
                if j + 2 <= ROWS {
                    link(k(i, j), k(i, j + 2), 0.15);
                }
            }
        }
        let behind = grid
            .pts
            .iter()
            .map(|p| {
                let bone = if p.y >= joint(Bone::Chest).y {
                    Bone::Chest
                } else if p.y >= joint(Bone::Spine).y {
                    Bone::Spine
                } else {
                    Bone::Hips
                };
                (bone.index(), (joint(bone).z - p.z).max(size))
            })
            .collect();
        Some(Self {
            first,
            centre: rig.parts[first..first + CELLS].iter().map(|p| p.rest).collect(),
            rest: grid.pts,
            bind,
            pins,
            links,
            behind,
            thighs: [(Bone::ThighL.index(), Bone::ShinL.index()), (Bone::ThighR.index(), Bone::ShinR.index())],
            girth: size * 6.5,
        })
    }

    /// The number of patches.
    pub fn patches(&self) -> usize {
        CELLS
    }

    /// The first patch's part in the rig.
    pub fn first(&self) -> usize {
        self.first
    }
}

/// Seconds of one step of the cloth (it steps at least this finely, however the frames fall).
const STEP: f32 = 1.0 / 120.0;
/// How much of its speed a point keeps each step (a little drag in every direction).
const KEEP: f32 = 0.99;
/// The air's push on the cloak's face, against its speed through it (per second): it catches the
/// air face-on and slips through it edge-on, so it trails at a slant rather than flying flat.
const FACE_DRAG: f32 = 1.2;
/// How much of his travel and turning the cloak leaves behind (0: it follows him rigidly, 1: it
/// feels all of it). A sprint starts at full speed in a frame; a real cloak would not feel that
/// jolt whole.
const INERTIA: f32 = 0.6;
/// Passes over the lengths each step.
const PASSES: usize = 5;
/// The cloth's weight.
const GRAVITY: Vec3 = Vec3::new(0.0, -9.8, 0.0);
/// A jump of the hero further than this in a frame (a portal, a fall back to the start) lays the
/// cloak back at rest about him rather than dragging it across the level.
const TELEPORT: f32 = 1.5;

/// The cloak's points as they move, in the world.
#[derive(Clone, Debug, Default)]
pub struct Cloth {
    pos: Vec<Vec3>,
    prev: Vec<Vec3>,
    root: Option<Mat4>,
    time: f32,
}

impl Cloth {
    /// Lets the cloak fall back to rest about him on the next step.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /**
     * Moves the cloak on by `dt` seconds with the hero placed at `root` and his bones' matrices in
     * the model `bones` (the clip's pose, `anim::Skeleton::matrices`), whose joints at rest are
     * `joints`.
     */
    pub fn step(&mut self, cape: &Cape, bones: &[Mat4], joints: &[Vec3], root: Mat4, dt: f32) {
        let to_world = |p: Vec3| root.transform_point3(p);
        let at = root.w_axis.truncate();
        let fresh = self.pos.len() != cape.rest.len() || self.root.is_none_or(|r| r.w_axis.truncate().distance(at) > TELEPORT);
        if fresh {
            // At rest about him as his chest stands now.
            let chest = bones[Bone::Chest.index()];
            self.pos = cape.rest.iter().map(|p| to_world(chest.transform_point3(*p - joints[Bone::Chest.index()]))).collect();
            self.prev = self.pos.clone();
        } else if let Some(last) = self.root {
            // Carry the cloak along with part of his travel and turning (`INERTIA`).
            let (_, q0, t0) = last.to_scale_rotation_translation();
            let (_, q1, t1) = root.to_scale_rotation_translation();
            let turn = glam::Quat::IDENTITY.slerp(q1 * q0.inverse(), 1.0 - INERTIA);
            let shift = (t1 - t0) * (1.0 - INERTIA);
            for p in self.pos.iter_mut().chain(self.prev.iter_mut()) {
                *p = t0 + turn * (*p - t0) + shift;
            }
        }
        self.root = Some(root);
        let from_world = root.inverse();
        let steps = ((dt / STEP).ceil() as usize).clamp(1, 8);
        let h = dt / steps as f32;
        let w = COLS + 1;
        for _ in 0..steps {
            self.time += h;
            // A breath of wind across his back, never still.
            let wind = root.transform_vector3(Vec3::new((self.time * 1.3).sin() * 0.3, 0.0, -0.1 - (self.time * 2.1).sin() * 0.2));
            let old = self.pos.clone();
            for k in 0..self.pos.len() {
                let p = old[k];
                let v = (p - self.prev[k]) / h;
                // The cloak's face here, from its neighbours.
                let (i, j) = (k % w, k / w);
                let across = old[j * w + (i + 1).min(COLS)] - old[j * w + i.saturating_sub(1)];
                let down = old[(j + 1).min(ROWS) * w + i] - old[j.saturating_sub(1) * w + i];
                let n = across.cross(down).normalize_or_zero();
                let air = -n * (v - wind).dot(n) * FACE_DRAG;
                self.prev[k] = p;
                self.pos[k] = p + v * h * KEEP + (GRAVITY + air) * h * h;
            }
            for _ in 0..PASSES {
                for &(a, b, len, stiff) in &cape.links {
                    let d = self.pos[b] - self.pos[a];
                    let l = d.length();
                    if l < 1e-6 {
                        continue;
                    }
                    let push = d * ((l - len) / l * 0.5 * stiff);
                    self.pos[a] += push;
                    self.pos[b] -= push;
                }
                for &(k, off) in &cape.pins {
                    self.pos[k] = to_world(bones[Bone::Chest.index()].transform_point3(off));
                }
                self.keep_off_him(cape, bones, &from_world, root);
            }
        }
    }

    /// Keeps every point behind his back (as far as it hung at rest) and out of his thighs.
    fn keep_off_him(&mut self, cape: &Cape, bones: &[Mat4], from_world: &Mat4, root: Mat4) {
        let joint = |b: usize| bones[b].transform_point3(Vec3::ZERO);
        for (k, p) in self.pos.iter_mut().enumerate() {
            let mut m = from_world.transform_point3(*p);
            let (bone, behind) = cape.behind[k];
            // (Level: leaning forward his back faces up as well as back, and a point hanging far
            // below it would be pushed up along that, onto his back.)
            let b = bones[bone].transform_vector3(-Vec3::Z);
            let back = Vec3::new(b.x, 0.0, b.z).normalize_or(-Vec3::Z);
            let d = (m - joint(bone)).dot(back);
            if d < behind * 0.85 {
                m += back * (behind * 0.85 - d);
            }
            // A heavy cloak never lifts past about 60 degrees from hanging: each line down it stays
            // half its rest drop below the top (it would otherwise kick up flat when he sets off).
            let row = k / (COLS + 1);
            if row > 0 {
                let drop = cape.rest[0].y - cape.rest[row * (COLS + 1)].y;
                m.y = m.y.min(cape.rest[0].y - drop * 0.5);
            }
            for &(hip, knee) in &cape.thighs {
                let (a, b) = (joint(hip), joint(knee));
                let ab = b - a;
                let t = ((m - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
                let near = a + ab * t;
                let out = m - near;
                let l = out.length();
                if l < cape.girth && l > 1e-6 {
                    m = near + out / l * cape.girth;
                }
            }
            *p = root.transform_point3(m);
        }
    }

    /// Each patch's matrix in the model (to follow the skeleton's parts), for the hero at `root`.
    pub fn matrices(&self, cape: &Cape, root: Mat4) -> Vec<Mat4> {
        let from_world = root.inverse();
        (0..CELLS)
            .map(|n| match self.pos.len() == cape.rest.len() {
                true => {
                    let c = Grid::corners(n).map(|k| from_world.transform_point3(self.pos[k]));
                    let (bind, lx, ly) = cape.bind[n];
                    frame(c, Some((lx, ly))).0 * bind
                }
                false => Mat4::from_translation(cape.centre[n]),
            })
            .collect()
    }
}

/// The matrices of the whole jointed rig: the skeleton's, then the cloak's patches.
pub fn with_cloak(bones: &[Mat4; BONE_COUNT], cloak: Option<(&Cloth, &Cape)>, root: Mat4) -> Vec<Mat4> {
    let mut all = bones.to_vec();
    if let Some((cloth, cape)) = cloak {
        all.resize(cape.first(), Mat4::IDENTITY);
        all.extend(cloth.matrices(cape, root));
    }
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tests' cloak: a red sheet two voxels thick hanging down Ragnar's back from his
    /// shoulders to his knees, flaring toward the hem.
    const RED: u32 = 0x7d2a24;
    const TEST: CapeSpec = CapeSpec { colors: &[RED], lo: [f32::NEG_INFINITY, 0.0, f32::NEG_INFINITY], hi: [f32::INFINITY, 57.0, 9.5] };

    fn cloaked() -> Rig {
        let mut rig = crate::render::view::jointed_hero(HeroKind::Ragnar, 1, Some(1)).expect("Ragnar is jointed");
        let size = rig.size;
        let (w, _, d) = crate::render::heroes::RAGNAR_SIZE;
        let (ox, oz) = ((w - 1) as f32 / 2.0, (d - 1) as f32 / 2.0);
        for y in 20..=56 {
            let spread = (56 - y) as f32 * 0.07;
            for x in (14.0 - spread) as i32..=(45.0 + spread) as i32 {
                for z in [6, 7] {
                    let at = Vec3::new((ox - x as f32) * size, (y as f32 + 0.5) * size, (z as f32 - oz) * size);
                    let bone = if y as f32 >= 45.5 {
                        Bone::Chest
                    } else if y as f32 >= 35.5 {
                        Bone::Spine
                    } else {
                        Bone::Hips
                    };
                    let part = &mut rig.parts[bone.index()];
                    part.cubes.push((at - part.rest, lin(RED)));
                    part.shade.push(0.0);
                }
            }
        }
        split(rig, &TEST, w, d)
    }

    fn rest_bones(rig: &Rig) -> [Mat4; BONE_COUNT] {
        std::array::from_fn(|i| Mat4::from_translation(rig.parts[i].rest))
    }

    #[test]
    fn the_cloak_is_cut_into_patches_and_nothing_is_lost() {
        let rig = cloaked();
        let cape = Cape::of(&rig).expect("his cloak is cut out");
        assert_eq!(cape.first(), BONE_COUNT, "the patches follow the skeleton's bones");
        assert_eq!(rig.parts.len(), BONE_COUNT + CELLS);
        let cloak: usize = rig.parts[BONE_COUNT..].iter().map(|p| p.cubes.len()).sum();
        assert!(cloak > 1000, "the cloak has {cloak} voxels");
        // Nothing red is left on his back.
        let red: Vec<Vec3> = TEST.colors.iter().map(|&c| lin(c)).collect();
        for b in FROM {
            let p = &rig.parts[b.index()];
            let left = p.cubes.iter().filter(|(o, c)| (p.rest + *o).z < -0.1 && red.iter().any(|r| shade_of(*c, *r)));
            assert_eq!(left.count(), 0, "cloak left on {:?}", b);
        }
    }

    #[test]
    fn at_rest_the_patches_sit_where_the_cloak_was() {
        let rig = cloaked();
        let cape = Cape::of(&rig).unwrap();
        let mut cloth = Cloth::default();
        let bones = rest_bones(&rig);
        let joints: Vec<Vec3> = rig.parts.iter().map(|p| p.rest).collect();
        // One tiny step from rest: the points have not moved, so each patch is the plain move to its place.
        cloth.step(&cape, &bones, &joints, Mat4::IDENTITY, 1e-6);
        for (n, m) in cloth.matrices(&cape, Mat4::IDENTITY).iter().enumerate() {
            let want = Mat4::from_translation(rig.parts[cape.first() + n].rest);
            assert!(m.abs_diff_eq(want, 1e-3), "patch {n} moved at rest: {m:?}");
        }
    }

    #[test]
    fn it_hangs_from_his_chest_and_never_passes_through_his_back() {
        let rig = cloaked();
        let cape = Cape::of(&rig).unwrap();
        let bones = rest_bones(&rig);
        let joints: Vec<Vec3> = rig.parts.iter().map(|p| p.rest).collect();
        let mut cloth = Cloth::default();
        // He runs forward hard for two seconds: the cloak streams out behind him, never through him.
        for f in 0..120 {
            let root = Mat4::from_translation(Vec3::new(0.0, 0.0, f as f32 * 0.08));
            cloth.step(&cape, &bones, &joints, root, 1.0 / 60.0);
            for (k, p) in cloth.pos.iter().enumerate() {
                let m = root.inverse().transform_point3(*p);
                let (bone, behind) = cape.behind[k];
                assert!(joints[bone].z - m.z >= behind * 0.85 - 1e-3, "point {k} went into him at frame {f}");
            }
        }
        // The top line stays on his shoulders; the hem has risen behind him.
        let root = Mat4::from_translation(Vec3::new(0.0, 0.0, 119.0 * 0.08));
        let m = |k: usize| root.inverse().transform_point3(cloth.pos[k]);
        assert!(m(0).distance(cape.rest[0]) < 1e-3, "the cloak came off his chest");
        let hem = (ROWS * (COLS + 1) + COLS / 2, cape.rest[ROWS * (COLS + 1) + COLS / 2]);
        assert!(m(hem.0).z < hem.1.z - 0.05 && m(hem.0).y > hem.1.y, "the hem streams out behind: {:?} from {:?}", m(hem.0), hem.1);
        // Standing still again, it falls back and settles.
        let stand = root;
        for _ in 0..600 {
            cloth.step(&cape, &bones, &joints, stand, 1.0 / 60.0);
        }
        let speed: f32 = cloth.pos.iter().zip(&cloth.prev).map(|(a, b)| a.distance(*b)).fold(0.0, f32::max) * 120.0;
        assert!(speed < 0.6, "still swinging at {speed} m/s after ten seconds");
    }

    #[test]
    fn a_portal_lays_it_back_at_rest() {
        let rig = cloaked();
        let cape = Cape::of(&rig).unwrap();
        let bones = rest_bones(&rig);
        let joints: Vec<Vec3> = rig.parts.iter().map(|p| p.rest).collect();
        let mut cloth = Cloth::default();
        cloth.step(&cape, &bones, &joints, Mat4::IDENTITY, 1.0 / 60.0);
        let far = Mat4::from_translation(Vec3::new(40.0, 0.0, 0.0));
        cloth.step(&cape, &bones, &joints, far, 1.0 / 60.0);
        assert!(cloth.pos.iter().all(|p| p.x > 39.0), "the cloak was dragged across the level");
    }
}
