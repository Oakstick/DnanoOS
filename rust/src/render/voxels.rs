//! Voxel model data, sprites, and the cutting of characters into rigged parts.
//!
//! `layers` run from the bottom of the model to the top; each layer is a list
//! of rows from back (-z) to front (+z); each row is a string of palette keys
//! along x. '.' is empty. Models face +z.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use bytemuck::{Pod, Zeroable};
use glam::Vec3;

pub struct VoxelModel {
    pub palette: &'static [(char, u32)],
    pub layers: &'static [&'static [&'static str]],
}

#[derive(Clone, Copy, Debug)]
pub struct Voxel {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub c: u32,
    /// The rig part it was sculpted for (1 is the first `PartDef`), or 0 to cut it by boxes.
    pub part: u8,
}

pub struct Collected {
    pub voxels: Vec<Voxel>,
    pub width: i32,
    pub depth: i32,
    pub height: i32,
}

pub fn collect(model: &VoxelModel) -> Collected {
    let mut voxels = Vec::new();
    let mut width = 0;
    let mut depth = 0;
    for (y, rows) in model.layers.iter().enumerate() {
        depth = depth.max(rows.len() as i32);
        for (z, row) in rows.iter().enumerate() {
            width = width.max(row.chars().count() as i32);
            for (x, key) in row.chars().enumerate() {
                if key == '.' || key == ' ' {
                    continue;
                }
                let c = model
                    .palette
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, c)| *c)
                    .unwrap_or_else(|| panic!("voxel palette missing key {key:?}"));
                voxels.push(Voxel { x: x as i32, y: y as i32, z: z as i32, c, part: 0 });
            }
        }
    }
    Collected { voxels, width, depth, height: model.layers.len() as i32 }
}

/// sRGB hex to linear RGB, as Three.js stores colors.
pub fn lin(hex: u32) -> Vec3 {
    let f = |c: u32| {
        let c = c as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    Vec3::new(f((hex >> 16) & 255), f((hex >> 8) & 255), f(hex & 255))
}

/// A model's cubes centered on x/z with its base at y = 0, in world units.
pub struct Mesh {
    pub size: f32,
    pub cubes: Vec<(Vec3, Vec3)>,
}

// ---------------------------------------------------------------- sprites

/// Front view of a model as a 2D sprite: for each column and height, the
/// frontmost voxel's color. Rows run from the top of the model down.
pub struct Sprite {
    pub width: i32,
    pub height: i32,
    pub pixels: Vec<Vec<Option<u32>>>,
}

/// Front view of any voxel model (text-authored or sculpted).
pub fn sprite_from(c: &Collected) -> Sprite {
    // One pass over the voxels: each column keeps its frontmost (the last of equals, as before).
    let (w, h) = (c.width.max(0) as usize, c.height.max(0) as usize);
    let mut front: Vec<Option<(i32, u32)>> = vec![None; w * h];
    for v in &c.voxels {
        if v.x < 0 || v.y < 0 || v.x as usize >= w || v.y as usize >= h {
            continue;
        }
        let cell = &mut front[v.y as usize * w + v.x as usize];
        if cell.is_none_or(|(z, _)| v.z >= z) {
            *cell = Some((v.z, v.c));
        }
    }
    let pixels = (0..h).rev().map(|y| (0..w).map(|x| front[y * w + x].map(|(_, c)| c & 0xff_ffff)).collect()).collect();
    Sprite { width: c.width, height: c.height, pixels }
}

// ------------------------------------------------------------------- rigs

/// Inclusive voxel box.
pub type VoxelBox = ([i32; 3], [i32; 3]);

/// A named, independently rotating part of a model, cut from its voxels by boxes
/// (or claimed by the voxels sculpted for it: see `Sculpt::part`).
pub struct PartDef {
    pub name: &'static str,
    pub boxes: &'static [VoxelBox],
    /// Rotation pivot in voxel units of the model grid.
    pub pivot: [f32; 3],
}

pub struct RigPart {
    pub name: &'static str,
    /// Rest position of the joint, relative to the model root.
    pub rest: Vec3,
    /// Cubes relative to the joint.
    pub cubes: Vec<(Vec3, Vec3)>,
    /// Each cube's shade mode: plain lit, or a voxel material (see `characters::mat`).
    pub shade: Vec<f32>,
    /// Lowest cube face below the joint (negative when the part hangs below it).
    pub min_y: f32,
    /// The part it hangs from, when it has one (always an earlier part): it then turns with that
    /// part about its own joint (see `RigList::push`). The foes, the cast and the heroes as the code
    /// animates them have none; the jointed heroes do (`anim::skeleton`).
    pub parent: Option<usize>,
}

pub struct Rig {
    pub size: f32,
    /// 'body' first, then the named parts in definition order.
    pub parts: Vec<RigPart>,
    /// Names this rig's GPU mesh, which is built the first time it is drawn (see `Rig::mesh`).
    id: u64,
    mesh: OnceLock<Arc<RigMesh>>,
    whole: OnceLock<Arc<RigMesh>>,
    /// The same rig at half and a quarter of the resolution, made the first time they are wanted.
    lods: [OnceLock<Box<Rig>>; 2],
}

/// Levels of detail a rig has: itself, then at half and a quarter of the resolution.
pub const LOD_LEVELS: u8 = 3;

static NEXT_RIG: AtomicU64 = AtomicU64::new(1);

impl Rig {
    pub fn new(size: f32, parts: Vec<RigPart>) -> Self {
        Self {
            size,
            parts,
            id: NEXT_RIG.fetch_add(1, Ordering::Relaxed),
            mesh: OnceLock::new(),
            whole: OnceLock::new(),
            lods: [OnceLock::new(), OnceLock::new()],
        }
    }

    /// Names the rig (for caches and the level-of-detail memory).
    pub fn id(&self) -> u64 {
        self.id
    }

    /// The rig at level of detail `level`: 0 itself, 1 at half the resolution, 2 at a quarter
    /// (see `Rig::coarse`). Made once, on first use; the parts and joints stay the same, so the
    /// same pose drives every level.
    pub fn lod(&self, level: u8) -> &Rig {
        match level {
            0 => self,
            1 => self.lods[0].get_or_init(|| Box::new(self.coarse())),
            _ => self.lods[1].get_or_init(|| Box::new(self.lod(1).coarse())),
        }
    }

    /// The rig's surface as the GPU keeps it, built once on first use. A rig must not
    /// change after it has been drawn.
    pub fn mesh(&self) -> Arc<RigMesh> {
        self.mesh.get_or_init(|| Arc::new(RigMesh::build(self, true))).clone()
    }

    /// The rig with every face of every voxel, for drawing it see-through or dithered, where the
    /// faces inside show (and their layers are part of the look, as on the warriors' souls).
    pub fn mesh_whole(&self) -> Arc<RigMesh> {
        self.whole.get_or_init(|| Arc::new(RigMesh::build(self, false))).clone()
    }
    pub fn index(&self, name: &str) -> Option<usize> {
        self.parts.iter().position(|p| p.name == name)
    }

    /**
     * The same rig at half the resolution, for when its detail cannot be seen:
     * every cell two voxels a side becomes one voxel of their average colour
     * (and the commonest material among them), about a quarter as many cubes.
     */
    pub fn coarse(&self) -> Rig {
        let size = self.size * 2.0;
        let parts = self
            .parts
            .iter()
            .map(|p| {
                // Each cell's summed colour, how many voxels fell in it, and their materials.
                type Cell = (Vec3, u32, Vec<f32>);
                let mut cells: std::collections::HashMap<(i32, i32, i32), Cell> = std::collections::HashMap::new();
                for (j, (off, c)) in p.cubes.iter().enumerate() {
                    let key = ((off.x / size).floor() as i32, (off.y / size).floor() as i32, (off.z / size).floor() as i32);
                    let cell = cells.entry(key).or_insert((Vec3::ZERO, 0, Vec::new()));
                    cell.0 += *c;
                    cell.1 += 1;
                    cell.2.push(p.shade.get(j).copied().unwrap_or(0.0));
                }
                let mut keys: Vec<_> = cells.keys().copied().collect();
                keys.sort_unstable();
                let (mut cubes, mut shade) = (Vec::new(), Vec::new());
                for k in keys {
                    let (sum, n, shades) = &cells[&k];
                    let at = (Vec3::new(k.0 as f32, k.1 as f32, k.2 as f32) + Vec3::splat(0.5)) * size;
                    cubes.push((at, *sum / *n as f32));
                    let common = shades.iter().copied().max_by_key(|m| shades.iter().filter(|o| *o == m).count()).unwrap_or(0.0);
                    shade.push(common);
                }
                let min_y = cubes.iter().map(|(q, _)| q.y - size / 2.0).fold(f32::INFINITY, f32::min);
                RigPart { name: p.name, rest: p.rest, cubes, shade, min_y: if min_y.is_finite() { min_y } else { 0.0 }, parent: p.parent }
            })
            .collect();
        Rig::new(size, parts)
    }
}

// ------------------------------------------------------------ GPU meshes

/// One open voxel face in a rig's GPU mesh, drawn as a quad whose corners the shader makes (as
/// static faces are: `FACE_CORNERS`), as the shader's `RigFace` reads it. 32 bytes a face, where
/// four corners of 36 bytes and six 4-byte indices took 168: Surtr's three levels went from 20 MB
/// to under 4.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct RigFace {
    /// The voxel's centre relative to the part's joint, in world units (the shader adds the
    /// corner, so a face's four corners agree exactly on where the face is).
    pub pos: [f32; 3],
    /// Which part (and so which joint matrix) it belongs to, in the low byte; which face it is in
    /// the next (0 to 5: +x, -x, +y, -y, +z, -z).
    pub part_face: u32,
    /// Linear colour and the shade mode. (Full floats: the shader seeds each voxel's detail from
    /// its exact colour, so rounding it would change the pattern.)
    pub color: [f32; 4],
}

/**
 * A rig as the GPU draws it: only the voxel faces that nothing in the same part covers,
 * each still its own voxel's square (no merging), so materials shade as they did on whole
 * cubes. Faces between parts are kept, since a joint can swing them into view.
 */
pub struct RigMesh {
    pub id: u64,
    pub faces: Vec<RigFace>,
    pub voxels: u32,
    /// Parts whose metal rusts with a dull blade (the held weapon and shield), one bit each.
    pub rust_mask: u32,
}

impl RigMesh {
    /// Its size on the GPU, in bytes.
    pub fn bytes(&self) -> usize {
        std::mem::size_of_val(self.faces.as_slice())
    }
}

/// The six faces of a unit cube by their outward axis (the shader's `FACE_CORNERS` walks each
/// one's corners).
const FACES: [[i32; 3]; 6] = [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1]];

impl RigMesh {
    fn build(rig: &Rig, cull: bool) -> Self {
        let mut faces = Vec::new();
        let mut voxels = 0;
        let mut rust_mask = 0;
        // Offsets sit on a grid of `size` shifted by the joint's fraction: doubled and rounded, neighbours are 2 apart.
        let key = |p: Vec3| {
            let k = (p / rig.size * 2.0).round();
            (k.x as i32, k.y as i32, k.z as i32)
        };
        assert!(rig.parts.len() <= 256, "a rig's part must fit in a byte");
        for (pi, part) in rig.parts.iter().enumerate() {
            if pi < 32 && matches!(part.name, "blade" | "offhand") {
                rust_mask |= 1 << pi;
            }
            let filled: std::collections::HashSet<(i32, i32, i32)> = part.cubes.iter().map(|(o, _)| key(*o)).collect();
            for (j, (off, c)) in part.cubes.iter().enumerate() {
                voxels += 1;
                let shade = part.shade.get(j).copied().filter(|&m| m > 0.0).unwrap_or(0.0);
                let color = [c.x, c.y, c.z, shade];
                let k = key(*off);
                for (fi, dir) in FACES.into_iter().enumerate() {
                    if cull && filled.contains(&(k.0 + dir[0] * 2, k.1 + dir[1] * 2, k.2 + dir[2] * 2)) {
                        continue;
                    }
                    faces.push(RigFace { pos: off.to_array(), part_face: pi as u32 | (fi as u32) << 8, color });
                }
            }
        }
        Self { id: rig.id * 2 + u64::from(!cull), faces, voxels, rust_mask }
    }
}

fn in_box(v: &Voxel, b: &VoxelBox) -> bool {
    v.x >= b.0[0] && v.x <= b.1[0] && v.y >= b.0[1] && v.y <= b.1[1] && v.z >= b.0[2] && v.z <= b.1[2]
}

/// Splits a model into pivoted parts; rotating a part swings it around its joint.
pub fn build_rig(c: &Collected, defs: &[PartDef], size: f32) -> Rig {
    let ox = (c.width - 1) as f32 / 2.0;
    let oz = (c.depth - 1) as f32 / 2.0;
    let mut buckets: Vec<Vec<Voxel>> = vec![Vec::new(); defs.len()];
    let mut body = Vec::new();
    for v in &c.voxels {
        let tagged = (v.part as usize).checked_sub(1).filter(|&i| i < defs.len());
        match tagged.or_else(|| defs.iter().position(|d| d.boxes.iter().any(|b| in_box(v, b)))) {
            Some(i) => buckets[i].push(*v),
            None => body.push(*v),
        }
    }
    let make = |name: &'static str, list: &[Voxel], pivot: [f32; 3]| make_part(c, name, list, pivot, size);
    let mut parts = vec![make("body", &body, [ox, 0.0, oz])];
    for (i, d) in defs.iter().enumerate() {
        parts.push(make(d.name, &buckets[i], d.pivot));
    }
    Rig::new(size, parts)
}

/**
 * A rig whose extra parts are whole models of their own, sculpted in the same
 * grid as the main one: for parts that overlap the body in space (an iris on
 * an eyeball, lids around it) and so cannot be cut out with boxes.
 */
pub fn build_layered_rig(main: &Collected, defs: &[PartDef], layers: &[(&'static str, &Collected, [f32; 3])], size: f32) -> Rig {
    let mut rig = build_rig(main, defs, size);
    for (name, c, pivot) in layers {
        rig.parts.push(make_part(main, name, &c.voxels, *pivot, size));
    }
    rig
}

fn make_part(c: &Collected, name: &'static str, list: &[Voxel], pivot: [f32; 3], size: f32) -> RigPart {
    let ox = (c.width - 1) as f32 / 2.0;
    let oz = (c.depth - 1) as f32 / 2.0;
    {
        let rest = Vec3::new((pivot[0] - ox) * size, pivot[1] * size, (pivot[2] - oz) * size);
        let cubes: Vec<(Vec3, Vec3)> = list
            .iter()
            .map(|v| {
                (Vec3::new((v.x as f32 - pivot[0]) * size, (v.y as f32 + 0.5 - pivot[1]) * size, (v.z as f32 - pivot[2]) * size), lin(v.c))
            })
            .collect();
        let min_y = cubes.iter().map(|(p, _)| p.y - size / 2.0).fold(f32::INFINITY, f32::min);
        // The top byte of a voxel's color names its material; 0 is plain lit.
        let shade = list.iter().map(|v| if v.c >> 24 == 0 { 0.0 } else { 5.0 + (v.c >> 24) as f32 }).collect();
        RigPart { name, rest, cubes, shade, min_y: if min_y.is_finite() { min_y } else { 0.0 }, parent: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_coarse_rig_keeps_its_parts_with_far_fewer_cubes() {
        let fine = build_rig(&crate::render::heroes::ragnar_hd(None), crate::render::heroes::RAGNAR_HD_RIG, 0.0125);
        let coarse = fine.coarse();
        assert_eq!(coarse.size, 0.025);
        assert_eq!(coarse.parts.len(), fine.parts.len());
        let count = |r: &Rig| r.parts.iter().map(|p| p.cubes.len()).sum::<usize>();
        assert!(count(&coarse) * 3 < count(&fine), "{} coarse against {} fine", count(&coarse), count(&fine));
        for (a, b) in coarse.parts.iter().zip(&fine.parts) {
            assert_eq!(a.rest, b.rest);
            assert_eq!(a.cubes.len(), a.shade.len());
            assert!(!a.cubes.is_empty(), "{} lost its cubes", a.name);
        }
    }

    #[test]
    fn a_rigs_levels_of_detail_keep_its_parts_and_are_made_once() {
        let rig = build_rig(&crate::render::heroes::ragnar_hd(None), crate::render::heroes::RAGNAR_HD_RIG, 0.0125);
        assert!(std::ptr::eq(rig.lod(0), &rig));
        let (half, quarter) = (rig.lod(1), rig.lod(2));
        assert!(std::ptr::eq(half, rig.lod(1)), "made once");
        assert_eq!((half.size, quarter.size), (0.025, 0.05));
        let count = |r: &Rig| r.parts.iter().map(|p| p.cubes.len()).sum::<usize>();
        assert!(count(quarter) < count(half) && count(half) < count(&rig));
        for (a, b) in quarter.parts.iter().zip(&rig.parts) {
            assert_eq!((a.name, a.rest), (b.name, b.rest));
        }
        assert_ne!(rig.mesh().id, half.mesh().id);
    }

    #[test]
    fn a_rig_mesh_keeps_only_the_open_faces() {
        let rig = build_rig(&crate::render::heroes::ragnar_hd(None), crate::render::heroes::RAGNAR_HD_RIG, 0.0125);
        let mesh = rig.mesh();
        let voxels: usize = rig.parts.iter().map(|p| p.cubes.len()).sum();
        assert_eq!(mesh.voxels as usize, voxels);
        let faces = mesh.faces.len();
        // The sculpts are hollow already, so about half their faces stay open.
        assert!(faces * 5 < voxels * 6 * 3, "{faces} faces left of {}", voxels * 6);
        assert!(mesh.faces.iter().all(|f| ((f.part_face & 0xff) as usize) < rig.parts.len() && f.part_face >> 8 < 6));
        // A lone voxel keeps all six; two touching keep ten.
        let part = |cubes: Vec<(Vec3, Vec3)>| RigPart {
            name: "body",
            rest: Vec3::ZERO,
            shade: vec![0.0; cubes.len()],
            cubes,
            min_y: 0.0,
            parent: None,
        };
        let one = Rig::new(0.5, vec![part(vec![(Vec3::ZERO, Vec3::ONE)])]);
        assert_eq!(one.mesh().faces.len(), 6);
        assert!(Arc::ptr_eq(&one.mesh(), &one.mesh()), "built once");
        assert_ne!(one.mesh().id, one.mesh_whole().id);
        assert_eq!(rig.mesh_whole().faces.len(), voxels * 6);
        let two = Rig::new(0.5, vec![part(vec![(Vec3::new(0.25, 0.25, 0.0), Vec3::ONE), (Vec3::new(0.75, 0.25, 0.0), Vec3::ONE)])]);
        assert_eq!(two.mesh().faces.len(), 10);
    }

    #[test]
    fn rigs_claim_every_voxel_once() {
        let model = crate::render::characters::ragnar();
        let rig = build_rig(&model, crate::render::characters::RAGNAR_RIG, 0.05);
        assert_eq!(rig.parts.len(), 6);
        let total: usize = rig.parts.iter().map(|p| p.cubes.len()).sum();
        assert_eq!(total, model.voxels.len());
    }

    #[test]
    fn a_sprite_is_each_columns_frontmost_voxel() {
        let model = crate::render::heroes::ragnar_hd(None);
        let s = sprite_from(&model);
        for (row, y) in s.pixels.iter().zip((0..model.height).rev()) {
            for (x, px) in row.iter().enumerate() {
                let best = model.voxels.iter().filter(|v| v.x == x as i32 && v.y == y).max_by_key(|v| v.z);
                assert_eq!(*px, best.map(|v| v.c & 0xff_ffff));
            }
        }
    }

    #[test]
    fn portrait_is_the_front_view() {
        let s = sprite_from(&crate::render::characters::ragnar());
        assert_eq!(s.height, 23);
        assert_eq!(s.width, 15);
    }
}
