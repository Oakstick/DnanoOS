//! `--export-rig HERO OUT.glb`: a jointed hero as a binary glTF for Blender. His voxels go out as a
//! mesh per bone (each voxel's open faces, culled as the resident rig culls them, `voxels::RigMesh`;
//! vertex colours are the voxels' linear colours), every vertex bound wholly to its one bone (a
//! skin: the voxels stay rigid), with the armature in its rest pose and the bind.
//!
//! Axes: glTF's are the engine's. Y is up, he faces +Z, and +X is his own left; a metre is a tile.
//! Blender turns that into its own Z up on import (he faces -Y there, his left at +X).

use glam::{Mat4, Vec3};

use super::skeleton::{BONES, BONE_COUNT, PARENTS};
use crate::render::voxels::Rig;

/// The six faces' outward axes, as `voxels::RigMesh` numbers them, and two axes across each
/// (`u × v` is the outward axis, so the corners run anticlockwise seen from outside).
const FACES: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
    ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
    ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
    ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
    ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
    ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
    ([0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
];

/// The binary buffer as it is laid out, and the JSON of its views and accessors.
#[derive(Default)]
struct Bin {
    data: Vec<u8>,
    views: Vec<String>,
    accessors: Vec<String>,
}

impl Bin {
    /// Adds `bytes` as a buffer view (`target` 34962 for vertices, 34963 for indices, 0 for none)
    /// and an accessor of `count` elements over it; returns the accessor's index.
    fn add(&mut self, bytes: &[u8], target: u32, component: u32, kind: &str, count: usize, minmax: Option<(Vec3, Vec3)>) -> usize {
        while !self.data.len().is_multiple_of(4) {
            self.data.push(0);
        }
        let view = self.views.len();
        let target = if target == 0 { String::new() } else { format!(",\"target\":{target}") };
        self.views.push(format!("{{\"buffer\":0,\"byteOffset\":{},\"byteLength\":{}{target}}}", self.data.len(), bytes.len()));
        self.data.extend_from_slice(bytes);
        let mm = minmax
            .map(|(lo, hi)| format!(",\"min\":[{},{},{}],\"max\":[{},{},{}]", lo.x, lo.y, lo.z, hi.x, hi.y, hi.z))
            .unwrap_or_default();
        self.accessors.push(format!("{{\"bufferView\":{view},\"componentType\":{component},\"count\":{count},\"type\":\"{kind}\"{mm}}}"));
        self.accessors.len() - 1
    }
}

fn floats(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// The jointed rig `rig` (its parts in the order of `BONES`) as a GLB file.
pub fn rig_glb(rig: &Rig, name: &str) -> Vec<u8> {
    assert_eq!(rig.parts.len(), BONE_COUNT, "a jointed rig has a part for every bone");
    let mesh = rig.mesh();
    let h = rig.size / 2.0;
    let mut bin = Bin::default();
    // Each bone's faces.
    let mut meshes = Vec::new();
    let mut mesh_nodes = Vec::new();
    for (b, part) in rig.parts.iter().enumerate() {
        let (mut pos, mut nrm, mut col, mut joints, mut weights, mut idx) = (vec![], vec![], vec![], vec![], vec![], vec![]);
        for f in mesh.faces.iter().filter(|f| (f.part_face & 0xff) as usize == b) {
            let (n, u, v) = FACES[(f.part_face >> 8) as usize];
            let (n, u, v) = (Vec3::from(n), Vec3::from(u), Vec3::from(v));
            let c = part.rest + Vec3::from(f.pos) + n * h;
            let base = (pos.len() / 3) as u32;
            for (a, bb) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                let p = c + (u * a + v * bb) * h;
                pos.extend_from_slice(&p.to_array());
                nrm.extend_from_slice(&n.to_array());
                col.extend_from_slice(&f.color[..3]);
                joints.extend_from_slice(&[b as u8, 0, 0, 0]);
                weights.extend_from_slice(&[1.0f32, 0.0, 0.0, 0.0]);
            }
            idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        if idx.is_empty() {
            continue;
        }
        let count = pos.len() / 3;
        let lo = pos.chunks(3).fold(Vec3::splat(f32::INFINITY), |m, p| m.min(Vec3::from_slice(p)));
        let hi = pos.chunks(3).fold(Vec3::splat(f32::NEG_INFINITY), |m, p| m.max(Vec3::from_slice(p)));
        let a_pos = bin.add(&floats(&pos), 34962, 5126, "VEC3", count, Some((lo, hi)));
        let a_nrm = bin.add(&floats(&nrm), 34962, 5126, "VEC3", count, None);
        let a_col = bin.add(&floats(&col), 34962, 5126, "VEC3", count, None);
        let a_jnt = bin.add(&joints, 34962, 5121, "VEC4", count, None);
        let a_wgt = bin.add(&floats(&weights), 34962, 5126, "VEC4", count, None);
        let ib: Vec<u8> = idx.iter().flat_map(|i| i.to_le_bytes()).collect();
        let a_idx = bin.add(&ib, 34963, 5125, "SCALAR", idx.len(), None);
        meshes.push(format!(
            "{{\"name\":\"{}\",\"primitives\":[{{\"attributes\":{{\"POSITION\":{a_pos},\"NORMAL\":{a_nrm},\"COLOR_0\":{a_col},\"JOINTS_0\":{a_jnt},\"WEIGHTS_0\":{a_wgt}}},\"indices\":{a_idx},\"material\":0}}]}}",
            BONES[b]
        ));
        mesh_nodes.push((b, meshes.len() - 1));
    }
    // The bind: each joint's rest in the model, inverted (the bones are not turned at rest).
    let ibm: Vec<f32> = rig.parts.iter().flat_map(|p| Mat4::from_translation(-p.rest).to_cols_array()).collect();
    let a_ibm = bin.add(&floats(&ibm), 0, 5126, "MAT4", BONE_COUNT, None);

    // Nodes: the bones first (each placed relative to its parent), then a node for each mesh.
    let mut nodes = Vec::new();
    for (b, part) in rig.parts.iter().enumerate() {
        let t = part.rest - PARENTS[b].map(|p| rig.parts[p].rest).unwrap_or(Vec3::ZERO);
        let children: Vec<String> = (0..BONE_COUNT).filter(|&c| PARENTS[c] == Some(b)).map(|c| c.to_string()).collect();
        let children = if children.is_empty() { String::new() } else { format!(",\"children\":[{}]", children.join(",")) };
        nodes.push(format!("{{\"name\":\"{}\",\"translation\":[{},{},{}]{children}}}", BONES[b], t.x, t.y, t.z));
    }
    let mut roots = vec!["0".to_string()];
    for (b, m) in &mesh_nodes {
        roots.push(nodes.len().to_string());
        nodes.push(format!("{{\"name\":\"{}_voxels\",\"mesh\":{m},\"skin\":0}}", BONES[*b]));
    }
    let joints: Vec<String> = (0..BONE_COUNT).map(|b| b.to_string()).collect();
    let json = format!(
        concat!(
            "{{\"asset\":{{\"version\":\"2.0\",\"generator\":\"Adventures of the Ancient Blades --export-rig\"}},",
            "\"scene\":0,\"scenes\":[{{\"name\":\"{name}\",\"nodes\":[{roots}]}}],",
            "\"nodes\":[{nodes}],",
            "\"skins\":[{{\"name\":\"{name}\",\"joints\":[{joints}],\"skeleton\":0,\"inverseBindMatrices\":{ibm}}}],",
            "\"meshes\":[{meshes}],",
            "\"materials\":[{{\"name\":\"voxel\",\"pbrMetallicRoughness\":{{\"baseColorFactor\":[1,1,1,1],\"metallicFactor\":0,\"roughnessFactor\":0.9}}}}],",
            "\"accessors\":[{accessors}],\"bufferViews\":[{views}],\"buffers\":[{{\"byteLength\":{len}}}]}}"
        ),
        name = name,
        roots = roots.join(","),
        nodes = nodes.join(","),
        joints = joints.join(","),
        ibm = a_ibm,
        meshes = meshes.join(","),
        accessors = bin.accessors.join(","),
        views = bin.views.join(","),
        len = bin.data.len(),
    );
    glb(json.into_bytes(), bin.data)
}

/// Writes hero `name`'s jointed rig (his first blade, his first shield on his arm) to `path`.
pub fn export_rig(name: &str, path: &str) -> Result<String, String> {
    use crate::logic::heroes::HeroKind;
    let hero = match name.to_ascii_lowercase().as_str() {
        "ragnar" => HeroKind::Ragnar,
        "mehmed" => HeroKind::Mehmed,
        "miyamoto" => HeroKind::Miyamoto,
        other => return Err(format!("no hero {other}: ragnar, mehmed or miyamoto")),
    };
    let rig =
        crate::render::view::jointed_hero(hero, 1, Some(1)).ok_or_else(|| format!("{name} has no joint table yet (anim::skeleton)"))?;
    let glb = rig_glb(&rig, &name.to_ascii_lowercase());
    std::fs::write(path, &glb).map_err(|e| format!("{path}: {e}"))?;
    let voxels: usize = rig.parts.iter().map(|p| p.cubes.len()).sum();
    Ok(format!("wrote {path}: {} bones, {voxels} voxels, {} faces, {} KB", BONE_COUNT, rig.mesh().faces.len(), glb.len() / 1024))
}

/// What the clips cost: reading them (once), and a frame of the animator and the hierarchy for
/// the hero, timed over many frames of a run, a swing, a guard and a leap.
pub fn bench() -> String {
    use super::hero::{Animator, Drive, Strike};
    use super::library;
    use crate::logic::heroes::HeroKind;
    let mut out = String::new();
    let t = std::time::Instant::now();
    let Some(clips) = library::clips(HeroKind::Ragnar) else {
        return "no clips for Ragnar
"
        .into();
    };
    let first = t.elapsed().as_secs_f64() * 1e3;
    out += &format!(
        "clips: {} read in {:.2} ms ({} embedded: {})
",
        clips.clips.len(),
        clips.load_ms,
        library::embedded(HeroKind::Ragnar).len(),
        library::embedded(HeroKind::Ragnar).join(", ")
    );
    out += &format!(
        "clips: first call {first:.2} ms; {} errors
",
        clips.errors.len()
    );
    for c in &clips.clips {
        out += &format!(
            "  {:12} {:3} frames {:.2} s, markers {:?}
",
            c.name,
            c.frame_count(),
            c.duration(),
            c.markers.marks
        );
    }
    let bytes: usize = super::library::EMBEDDED.iter().map(|e| e.2.len() + e.3.len()).sum();
    out += &format!(
        "embedded: {} KB of GLB and JSON
",
        bytes / 1024
    );
    let rig = crate::render::view::jointed_hero(HeroKind::Ragnar, 1, Some(1)).expect("Ragnar's joints");
    let joints: [Vec3; BONE_COUNT] = std::array::from_fn(|i| rig.parts[i].rest);
    let mut a = Animator::default();
    let frames = 20_000;
    let t = std::time::Instant::now();
    let mut sink = 0.0f32;
    for f in 0..frames {
        let k = f % 240;
        let strike = (60..90).contains(&k).then(|| Strike {
            t: (k - 60) as f32 / 60.0,
            bounds: [0.0, 0.1, 0.167, 0.4],
            side: if f % 480 < 240 { 1.0 } else { -1.0 },
            serial: (f / 240) as u32,
            heavy: f % 720 >= 480,
        });
        let d = Drive {
            dt: 1.0 / 60.0,
            speed: if k < 60 { 5.0 } else { 0.0 },
            motion: if k < 60 { 1.0 } else { 0.0 },
            airborne: (120..145).contains(&k).then(|| (k - 120) as f32 / 25.0),
            strike,
            blocking: (160..200).contains(&k),
            springs: true,
            ..Default::default()
        };
        if let Some((locals, extra)) = a.step(clips, &d) {
            let m = clips.skeleton.matrices(&locals, &joints, &extra);
            sink += m[10].w_axis.x;
        }
    }
    let us = t.elapsed().as_secs_f64() * 1e6 / frames as f64;
    out += &format!(
        "a frame: {us:.2} µs to sample, blend and pose his {} bones (checksum {sink:.1})
",
        BONE_COUNT
    );
    out
}

/// The GLB container: its header, the JSON chunk (padded with spaces) and the binary chunk (with zeros).
fn glb(mut json: Vec<u8>, mut bin: Vec<u8>) -> Vec<u8> {
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    let total = 12 + 8 + json.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&json);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(&bin);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::anim::clip::Clip;

    #[test]
    fn an_exported_rig_reads_back_with_its_bones_and_voxels() {
        let rig = crate::render::view::jointed_hero(crate::logic::heroes::HeroKind::Ragnar, 1, Some(1)).expect("Ragnar has joints");
        let glb = rig_glb(&rig, "ragnar");
        let doc = gltf::Gltf::from_slice(&glb).expect("a valid GLB");
        let blob = doc.blob.as_deref().unwrap();
        // Every bone, in order, its parent the skeleton's, at its joint.
        let skin = doc.skins().next().unwrap();
        let joints: Vec<_> = skin.joints().collect();
        assert_eq!(joints.len(), BONE_COUNT);
        for (b, j) in joints.iter().enumerate() {
            assert_eq!(j.name(), Some(BONES[b]));
        }
        // Each mesh is bound wholly to its own bone, and the faces are the resident rig's.
        let mut faces = 0;
        for node in doc.nodes().filter(|n| n.mesh().is_some()) {
            let bone = BONES.iter().position(|b| node.name() == Some(&format!("{b}_voxels"))).unwrap();
            for prim in node.mesh().unwrap().primitives() {
                let r = prim.reader(|_| Some(blob));
                assert!(r.read_joints(0).unwrap().into_u16().all(|j| j[0] as usize == bone));
                let n = r.read_positions().unwrap().count();
                assert_eq!(n % 4, 0);
                faces += n / 4;
                // Every corner lies on its voxel's face, within the bone's voxels.
                let part = &rig.parts[bone];
                let lo = part.cubes.iter().fold(Vec3::INFINITY, |m, (o, _)| m.min(part.rest + *o)) - Vec3::splat(rig.size);
                let hi = part.cubes.iter().fold(Vec3::NEG_INFINITY, |m, (o, _)| m.max(part.rest + *o)) + Vec3::splat(rig.size);
                assert!(r.read_positions().unwrap().all(|p| Vec3::from(p).cmpge(lo).all() && Vec3::from(p).cmple(hi).all()));
            }
        }
        assert_eq!(faces, rig.mesh().faces.len());
        // As a clip's skeleton, its rest puts every part on its joint.
        let clip = Clip::from_glb("rest", &glb, r#"{"markers":{}}"#).unwrap();
        let joints: Vec<Vec3> = rig.parts.iter().map(|p| p.rest).collect();
        let m = clip.skeleton.matrices(&clip.skeleton.rest, &joints, &[glam::Quat::IDENTITY; BONE_COUNT]);
        for (b, m) in m.iter().enumerate() {
            assert!(m.abs_diff_eq(Mat4::from_translation(joints[b]), 1e-5));
        }
        assert_eq!(clip.frame_count(), 1);
    }
}
