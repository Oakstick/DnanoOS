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
}

pub struct Rig {
    pub size: f32,
    /// 'body' first, then the named parts in definition order.
    pub parts: Vec<RigPart>,
    /// Names this rig's GPU mesh, which is built the first time it is drawn (see `Rig::mesh`).
    id: u64,
    mesh: OnceLock<Arc<RigMesh>>,
    whole: OnceLock<Arc<RigMesh>>,
}

static NEXT_RIG: AtomicU64 = AtomicU64::new(1);

impl Rig {
    pub fn new(size: f32, parts: Vec<RigPart>) -> Self {
        Self { size, parts, id: NEXT_RIG.fetch_add(1, Ordering::Relaxed), mesh: OnceLock::new(), whole: OnceLock::new() }
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
                RigPart { name: p.name, rest: p.rest, cubes, shade, min_y: if min_y.is_finite() { min_y } else { 0.0 } }
            })
            .collect();
        Rig::new(size, parts)
    }
}

// ------------------------------------------------------------ GPU meshes

/// One corner of a voxel face in a rig's GPU mesh.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct RigVertex {
    /// The voxel's centre relative to the part's joint, in world units (the shader adds the
    /// corner, so a face's four corners agree exactly on where the face is).
    pub pos: [f32; 3],
    /// The corner in the voxel's own unit cube, as -1 or 1 (halved by the shader), for per-voxel
    /// detail; then which face it is on (0 to 5: +x, -x, +y, -y, +z, -z).
    pub local: [i8; 4],
    /// Linear colour and the shade mode. (Full floats: the shader seeds each voxel's detail from
    /// its exact colour, so rounding it would change the pattern.)
    pub color: [f32; 4],
    /// Which part (and so which joint matrix) it belongs to.
    pub part: u32,
}

/**
 * A rig as the GPU draws it: only the voxel faces that nothing in the same part covers,
 * each still its own voxel's square (no merging), so materials shade as they did on whole
 * cubes. Faces between parts are kept, since a joint can swing them into view.
 */
pub struct RigMesh {
    pub id: u64,
    pub vertices: Vec<RigVertex>,
    pub indices: Vec<u32>,
    pub voxels: u32,
    /// Parts whose metal rusts with a dull blade (the held weapon and shield), one bit each.
    pub rust_mask: u32,
}

/// The six faces of a unit cube: the outward axis, then the four corners in order round the face.
const FACES: [([i32; 3], [[i8; 3]; 4]); 6] = [
    ([1, 0, 0], [[1, -1, -1], [1, 1, -1], [1, 1, 1], [1, -1, 1]]),
    ([-1, 0, 0], [[-1, -1, -1], [-1, -1, 1], [-1, 1, 1], [-1, 1, -1]]),
    ([0, 1, 0], [[-1, 1, -1], [-1, 1, 1], [1, 1, 1], [1, 1, -1]]),
    ([0, -1, 0], [[-1, -1, -1], [1, -1, -1], [1, -1, 1], [-1, -1, 1]]),
    ([0, 0, 1], [[-1, -1, 1], [1, -1, 1], [1, 1, 1], [-1, 1, 1]]),
    ([0, 0, -1], [[-1, -1, -1], [-1, 1, -1], [1, 1, -1], [1, -1, -1]]),
];

impl RigMesh {
    fn build(rig: &Rig, cull: bool) -> Self {
        let (mut vertices, mut indices) = (Vec::new(), Vec::new());
        let mut voxels = 0;
        let mut rust_mask = 0;
        // Offsets sit on a grid of `size` shifted by the joint's fraction: doubled and rounded, neighbours are 2 apart.
        let key = |p: Vec3| {
            let k = (p / rig.size * 2.0).round();
            (k.x as i32, k.y as i32, k.z as i32)
        };
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
                for (fi, (dir, corners)) in FACES.into_iter().enumerate() {
                    if cull && filled.contains(&(k.0 + dir[0] * 2, k.1 + dir[1] * 2, k.2 + dir[2] * 2)) {
                        continue;
                    }
                    let base = vertices.len() as u32;
                    for q in corners {
                        vertices.push(RigVertex {
                            pos: off.to_array(),
                            local: [q[0] * 127, q[1] * 127, q[2] * 127, fi as i8],
                            color,
                            part: pi as u32,
                        });
                    }
                    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                }
            }
        }
        Self { id: rig.id * 2 + u64::from(!cull), vertices, indices, voxels, rust_mask }
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
        RigPart { name, rest, cubes, shade, min_y: if min_y.is_finite() { min_y } else { 0.0 } }
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
    fn a_rig_mesh_keeps_only_the_open_faces() {
        let rig = build_rig(&crate::render::heroes::ragnar_hd(None), crate::render::heroes::RAGNAR_HD_RIG, 0.0125);
        let mesh = rig.mesh();
        let voxels: usize = rig.parts.iter().map(|p| p.cubes.len()).sum();
        assert_eq!(mesh.voxels as usize, voxels);
        assert_eq!(mesh.indices.len() % 6, 0);
        let faces = mesh.indices.len() / 6;
        // The sculpts are hollow already, so about half their faces stay open.
        assert!(faces * 5 < voxels * 6 * 3, "{faces} faces left of {}", voxels * 6);
        assert!(mesh.vertices.iter().all(|v| (v.part as usize) < rig.parts.len()));
        // A lone voxel keeps all six; two touching keep ten.
        let part = |cubes: Vec<(Vec3, Vec3)>| RigPart { name: "body", rest: Vec3::ZERO, shade: vec![0.0; cubes.len()], cubes, min_y: 0.0 };
        let one = Rig::new(0.5, vec![part(vec![(Vec3::ZERO, Vec3::ONE)])]);
        assert_eq!(one.mesh().indices.len(), 36);
        assert!(Arc::ptr_eq(&one.mesh(), &one.mesh()), "built once");
        assert_ne!(one.mesh().id, one.mesh_whole().id);
        assert_eq!(rig.mesh_whole().indices.len(), voxels * 36);
        let two = Rig::new(0.5, vec![part(vec![(Vec3::new(0.25, 0.25, 0.0), Vec3::ONE), (Vec3::new(0.75, 0.25, 0.0), Vec3::ONE)])]);
        assert_eq!(two.mesh().indices.len(), 60);
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
