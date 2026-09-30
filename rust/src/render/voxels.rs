//! Voxel model data, sprites, and the cutting of characters into rigged parts.
//!
//! `layers` run from the bottom of the model to the top; each layer is a list
//! of rows from back (-z) to front (+z); each row is a string of palette keys
//! along x. '.' is empty. Models face +z.

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
    let mut pixels = Vec::new();
    for y in (0..c.height).rev() {
        let mut row = Vec::new();
        for x in 0..c.width {
            let best = c.voxels.iter().filter(|v| v.x == x && v.y == y).max_by_key(|v| v.z);
            row.push(best.map(|v| v.c & 0xff_ffff));
        }
        pixels.push(row);
    }
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
}

impl Rig {
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
        Rig { size, parts }
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
    Rig { size, parts }
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
    fn rigs_claim_every_voxel_once() {
        let model = crate::render::characters::ragnar();
        let rig = build_rig(&model, crate::render::characters::RAGNAR_RIG, 0.05);
        assert_eq!(rig.parts.len(), 6);
        let total: usize = rig.parts.iter().map(|p| p.cubes.len()).sum();
        assert_eq!(total, model.voxels.len());
    }

    #[test]
    fn portrait_is_the_front_view() {
        let s = sprite_from(&crate::render::characters::ragnar());
        assert_eq!(s.height, 23);
        assert_eq!(s.width, 15);
    }
}
