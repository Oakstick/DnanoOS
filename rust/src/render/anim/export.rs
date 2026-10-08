//! `--export-rig HERO OUT.glb`: a jointed hero as a binary glTF for Blender. His voxels go out as a
//! mesh per bone (each voxel's open faces, culled as the resident rig culls them, `voxels::RigMesh`;
//! vertex colours are the voxels' linear colours), every vertex bound wholly to its one bone (a
//! skin: the voxels stay rigid), with the armature in its rest pose and the bind.
//!
//! Axes: glTF's are the engine's. Y is up, he faces +Z, and +X is his own left; a metre is a tile.
//! Blender turns that into its own Z up on import (he faces -Y there, his left at +X).

use glam::{Mat4, Vec3};

use super::skeleton::{SkeletonDef, BONE_COUNT, HUMANOID};
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

/// The jointed hero `rig` (its parts in the order of `BONES`) as a GLB file.
pub fn rig_glb(rig: &Rig, name: &str) -> Vec<u8> {
    rig_glb_of(&HUMANOID, rig, name)
}

/// A jointed rig of skeleton `def` (its parts in the order of its bones) as a GLB file.
pub fn rig_glb_of(def: &SkeletonDef, rig: &Rig, name: &str) -> Vec<u8> {
    let (bone_n, names, parents) = (def.len(), def.bones, def.parents);
    assert_eq!(rig.parts.len(), bone_n, "a jointed rig has a part for every bone");
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
            names[b]
        ));
        mesh_nodes.push((b, meshes.len() - 1));
    }
    // The bind: each joint's rest in the model, inverted (the bones are not turned at rest).
    let ibm: Vec<f32> = rig.parts.iter().flat_map(|p| Mat4::from_translation(-p.rest).to_cols_array()).collect();
    let a_ibm = bin.add(&floats(&ibm), 0, 5126, "MAT4", bone_n, None);

    // Nodes: the bones first (each placed relative to its parent), then a node for each mesh.
    let mut nodes = Vec::new();
    for (b, part) in rig.parts.iter().enumerate() {
        let t = part.rest - parents[b].map(|p| rig.parts[p].rest).unwrap_or(Vec3::ZERO);
        let children: Vec<String> = (0..bone_n).filter(|&c| parents[c] == Some(b)).map(|c| c.to_string()).collect();
        let children = if children.is_empty() { String::new() } else { format!(",\"children\":[{}]", children.join(",")) };
        nodes.push(format!("{{\"name\":\"{}\",\"translation\":[{},{},{}]{children}}}", names[b], t.x, t.y, t.z));
    }
    let mut roots = vec!["0".to_string()];
    for (b, m) in &mesh_nodes {
        roots.push(nodes.len().to_string());
        nodes.push(format!("{{\"name\":\"{}_voxels\",\"mesh\":{m},\"skin\":0}}", names[*b]));
    }
    let joints: Vec<String> = (0..bone_n).map(|b| b.to_string()).collect();
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

/// Writes hero `name`'s jointed rig (his first blade, his first shield on his arm), or a foe's
/// (`anim::foe_rigs`: `draugr`, `bat`, ... as their clips' folders are named), to `path`.
pub fn export_rig(name: &str, path: &str) -> Result<String, String> {
    use crate::logic::heroes::HeroKind;
    let lower = name.to_ascii_lowercase();
    if let Some(foe) = super::foe_rigs::FoeRig::ALL.iter().find(|f| f.dir() == lower) {
        let rig = foe.jointed();
        let glb = rig_glb_of(foe.def(), &rig, &lower);
        std::fs::write(path, &glb).map_err(|e| format!("{path}: {e}"))?;
        let voxels: usize = rig.parts.iter().map(|p| p.cubes.len()).sum();
        return Ok(format!(
            "wrote {path}: {} bones, {voxels} voxels, {} faces, {} KB",
            foe.def().len(),
            rig.mesh().faces.len(),
            glb.len() / 1024
        ));
    }
    if let Some(c) = super::cast_rigs::CastRig::named(&lower) {
        // The story's cast (`kid0` .. `kid3`, `oldman`, `dog`, `krampus_shade`).
        let rig = c.jointed();
        let def = c.joints().def;
        let glb = rig_glb_of(def, &rig, &lower);
        std::fs::write(path, &glb).map_err(|e| format!("{path}: {e}"))?;
        let voxels: usize = rig.parts.iter().map(|p| p.cubes.len()).sum();
        return Ok(format!(
            "wrote {path}: {} bones, {voxels} voxels, {} faces, {} KB",
            def.len(),
            rig.mesh().faces.len(),
            glb.len() / 1024
        ));
    }
    let hero = match lower.as_str() {
        "ragnar" => HeroKind::Ragnar,
        "mehmed" => HeroKind::Mehmed,
        "miyamoto" => HeroKind::Miyamoto,
        other => {
            let foes: Vec<&str> = super::foe_rigs::FoeRig::ALL.iter().map(|f| f.dir()).collect();
            return Err(format!("no hero or foe {other}: ragnar, mehmed, miyamoto, {}", foes.join(", ")));
        }
    };
    let rig =
        crate::render::view::jointed_hero(hero, 1, Some(1)).ok_or_else(|| format!("{name} has no joint table yet (anim::skeleton)"))?;
    let glb = rig_glb(&rig, &name.to_ascii_lowercase());
    std::fs::write(path, &glb).map_err(|e| format!("{path}: {e}"))?;
    let voxels: usize = rig.parts.iter().map(|p| p.cubes.len()).sum();
    Ok(format!("wrote {path}: {} bones, {voxels} voxels, {} faces, {} KB", BONE_COUNT, rig.mesh().faces.len(), glb.len() / 1024))
}

/// What the clips cost, for each hero: reading them (once), and a frame of the animator and the
/// hierarchy, timed over many frames of a run, a swing, a guard and a leap (with the hold of what he
/// carries, as he holds it).
pub fn bench() -> String {
    use crate::logic::heroes::HeroKind;
    let mut out = String::new();
    for hero in [HeroKind::Ragnar, HeroKind::Mehmed, HeroKind::Miyamoto] {
        out += &bench_hero(hero);
    }
    for foe in super::foe_rigs::FoeRig::ALL {
        out += &bench_foe(foe);
    }
    out += &bench_cast();
    let bytes: usize = super::library::EMBEDDED.iter().map(|e| e.2.len() + e.3.len()).sum();
    out += &format!("embedded: {} clips, {} KB of GLB and JSON\n", super::library::EMBEDDED.len(), bytes / 1024);
    out
}

fn bench_hero(hero: crate::logic::heroes::HeroKind) -> String {
    use super::hero::{Animator, Drive, Strike};
    use super::library;
    let mut out = String::new();
    let t = std::time::Instant::now();
    let Some(clips) = library::clips(hero) else {
        return format!("{hero:?}: no clips\n");
    };
    let first = t.elapsed().as_secs_f64() * 1e3;
    out += &format!(
        "{hero:?}: {} clips read in {:.2} ms (first call {first:.2} ms, {} errors): {}\n",
        clips.clips.len(),
        clips.load_ms,
        clips.errors.len(),
        library::embedded(hero).join(", ")
    );
    for c in &clips.clips {
        out += &format!("  {:12} {:3} frames {:.2} s, markers {:?}\n", c.name, c.frame_count(), c.duration(), c.markers.marks);
    }
    let rig = crate::render::view::jointed_hero(hero, 1, Some(1)).expect("every hero is jointed");
    let joints: [Vec3; BONE_COUNT] = std::array::from_fn(|i| rig.parts[i].rest);
    let held = hero == crate::logic::heroes::HeroKind::Miyamoto;
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
            two_hands: held,
            slung: held,
            ..Default::default()
        };
        if let Some((locals, extra)) = a.step(clips, &d) {
            let m = clips.skeleton.matrices(&locals, &joints, &extra);
            sink += m[10].w_axis.x;
        }
    }
    let us = t.elapsed().as_secs_f64() * 1e6 / frames as f64;
    out += &format!("  a frame: {us:.2} µs to sample, blend, hold and pose his {BONE_COUNT} bones (checksum {sink:.1})\n");
    out
}

/// What a foe's clips cost: reading them (once), and a frame of its animator and its bones, timed
/// over many frames of its gait, its moves and its reels.
fn bench_foe(foe: super::foe_rigs::FoeRig) -> String {
    use super::foe::{FoeAnimator, FoeDrive, FoeReel, FoeStrike};
    let t = std::time::Instant::now();
    let Some(clips) = super::library::foe_clips(foe) else {
        return format!(
            "{foe:?}: no clips
"
        );
    };
    let first = t.elapsed().as_secs_f64() * 1e3;
    let mut out = format!(
        "{foe:?}: {} clips read in {:.2} ms (first call {first:.2} ms, {} errors): {}
",
        clips.clips.len(),
        clips.load_ms,
        clips.errors.len(),
        super::library::embedded_in(foe.dir()).join(", ")
    );
    let rig = foe.jointed();
    let joints: Vec<Vec3> = rig.parts.iter().map(|p| p.rest).collect();
    let moves: Vec<&'static str> = ["slam", "sweep", "hex", "bite", "rally"].into_iter().filter(|m| clips.get(m).is_some()).collect();
    let mut a = FoeAnimator::new(0.3);
    let (mut locals, mut mats) = (Vec::new(), Vec::new());
    let frames = 20_000;
    let t = std::time::Instant::now();
    let mut sink = 0.0f32;
    for f in 0..frames {
        let k = f % 240;
        let strike = (60..150).contains(&k).then(|| FoeStrike {
            clips: [moves.get((f / 240) % moves.len().max(1)).copied().unwrap_or("-"); 2],
            t: (k - 60) as f32 / 60.0,
            bounds: [0.0, 0.5, 0.6, 1.5],
            serial: (f / 240) as u32,
        });
        let reel = (180..200).contains(&k).then(|| (if f % 480 < 240 { FoeReel::Hit } else { FoeReel::Stagger }, (k - 180) as f32 / 20.0));
        let d = FoeDrive { dt: 1.0 / 60.0, speed: if k < 60 { 2.5 } else { 0.0 }, moving: k < 60, strike, reel };
        if a.step(clips, foe.def(), &d, &mut locals) {
            clips.skeleton.matrices_into(&locals, &joints, &[], &mut mats);
            sink += mats[1].w_axis.x;
        }
    }
    let us = t.elapsed().as_secs_f64() * 1e6 / frames as f64;
    out += &format!(
        "  a frame: {us:.2} µs to sample, blend and pose its {} bones (checksum {sink:.1})
",
        foe.def().len()
    );
    out
}

/// What the story's cast's clips cost: reading each set (once), and a frame of a member's animator
/// (a base clip and one laid over it, a crossfade now and then) and its matrices.
fn bench_cast() -> String {
    use super::cast::{matrices, CastAnimator, Layer, Mask, Play};
    use super::cast_rigs::CastRig;
    let mut out = String::new();
    let plays: [(CastRig, [&str; 2]); 4] = [
        (CastRig::Kid(0), ["run", "terrified"]),
        (CastRig::OldMan, ["sit_talk", "sit_point"]),
        (CastRig::Dog, ["idle", "snarl"]),
        (CastRig::Krampus, ["run", "hold_up"]),
    ];
    for (c, [base, over]) in plays {
        let t = std::time::Instant::now();
        let Some(set) = super::library::cast_clips(c) else {
            out += &format!(
                "{c:?}: no clips
"
            );
            continue;
        };
        let first = t.elapsed().as_secs_f64() * 1e3;
        let lair = (c == CastRig::Krampus).then(|| super::library::foe_clips(super::foe_rigs::FoeRig::Krampus)).flatten();
        let mut sources = vec![(set, 1.0)];
        if let Some(l) = lair {
            sources.push((l, crate::render::cast::KRAMPUS_VOXEL / crate::render::foes_lair::KRAMPUS_VOXEL));
        }
        out += &format!(
            "{c:?}: {} clips read in {:.2} ms (first call {first:.2} ms, {} errors)
",
            set.clips.len(),
            set.load_ms,
            set.errors.len()
        );
        let def = c.joints().def;
        let rig = c.jointed();
        let joints: Vec<Vec3> = rig.parts.iter().map(|p| p.rest).collect();
        let (mut a, mut mats) = (CastAnimator::default(), Vec::new());
        let frames = 20_000;
        let t = std::time::Instant::now();
        let mut sink = 0.0f32;
        for f in 0..frames {
            let s = f as f32 / 60.0;
            let play = Play::base(Layer::cycle(base, s * 0.7), (f / 120) as u32 % 2).with(Layer::of(over, s).on(Mask::Upper).weight(0.6));
            if let Some(p) = a.step(&sources, def, &play, Some(1.0 / 60.0)) {
                matrices(def, &joints, p, &mut mats);
                sink += mats[1].w_axis.x;
            }
        }
        let us = t.elapsed().as_secs_f64() * 1e6 / frames as f64;
        out += &format!(
            "  a frame: {us:.2} µs to sample two clips, blend and pose its {} bones (checksum {sink:.1})
",
            def.len()
        );
    }
    out
}

/// The chase camera at rest, from the hero (the model's axes, metres): behind him, above, over his
/// right shoulder (`camera::chase`: its pivot 1.7 up and 0.3 to his right, 3.4 back along a pitch of
/// -0.44).
const CHASE_EYE: Vec3 = Vec3::new(-0.3, 1.7 + 3.4 * 0.425_939, -3.4 * 0.904_752);

/**
 * `--blade-view`: for each hero's strikes, the share of their active frames (from `windup_end` to
 * `active_end`) on which his blade can be seen from the chase camera at rest. A frame counts when at
 * least a quarter of the points along the blade (every few of its voxels, his first blade) can be
 * reached by a ray from the eye that strikes none of his other voxels first. He is posed by the clip
 * with the hold he fights with (both fists on the grip and the shield slung, for Miyamoto).
 */
pub fn blade_view() -> String {
    use crate::logic::heroes::HeroKind;
    let mut out = String::new();
    for hero in [HeroKind::Ragnar, HeroKind::Mehmed, HeroKind::Miyamoto] {
        let (Some(clips), Some(rig)) = (super::library::clips(hero), crate::render::view::jointed_hero(hero, 1, Some(1))) else {
            continue;
        };
        let joints: Vec<Vec3> = rig.parts.iter().map(|p| p.rest).collect();
        let blade = super::skeleton::Bone::Blade.index();
        for name in ["slash_fore", "slash_back", "slash_heavy"] {
            let Some(clip) = clips.get(name) else { continue };
            let (a, b) = (clip.markers.get("windup_end").unwrap_or(0.0), clip.markers.get("active_end").unwrap_or(0.0));
            let (mut seen, mut total) = (0, 0);
            for f in a.round() as usize..=b.round() as usize {
                let mut pose = clip.sample(f as f32 / super::clip::FPS);
                if hero == HeroKind::Miyamoto {
                    super::hold::two_hands(&clips.skeleton, &mut pose);
                    super::hold::sling(&clips.skeleton, &mut pose, 0.0);
                }
                let m = clips.skeleton.matrices(&pose, &joints, &[glam::Quat::IDENTITY; BONE_COUNT]);
                total += 1;
                if blade_seen(&rig, &m, blade) {
                    seen += 1;
                }
            }
            out += &format!("{hero:?} {name:12} the blade seen on {seen} of {total} active frames ({}%)\n", 100 * seen / total.max(1));
        }
    }
    out
}

/// Whether a quarter of the points along part `blade` of `rig` posed by `m` are in plain sight of
/// `CHASE_EYE`.
fn blade_seen(rig: &Rig, m: &[Mat4], blade: usize) -> bool {
    let cubes = &rig.parts[blade].cubes;
    let step = (cubes.len() / 48).max(1);
    let points: Vec<Vec3> = cubes.iter().step_by(step).map(|(o, _)| m[blade].transform_point3(*o)).collect();
    let h = rig.size * 0.5;
    // Each other part's voxels, and the box round them, in its own frame.
    let parts: Vec<(usize, Vec3, Vec3)> = rig
        .parts
        .iter()
        .enumerate()
        .filter(|(i, p)| *i != blade && !p.cubes.is_empty())
        .map(|(i, p)| {
            let lo = p.cubes.iter().fold(Vec3::INFINITY, |a, (o, _)| a.min(*o)) - Vec3::splat(h);
            let hi = p.cubes.iter().fold(Vec3::NEG_INFINITY, |a, (o, _)| a.max(*o)) + Vec3::splat(h);
            (i, lo, hi)
        })
        .collect();
    let clear = points
        .iter()
        .filter(|&&p| {
            parts.iter().all(|&(i, lo, hi)| {
                let inv = m[i].inverse();
                let (o, e) = (inv.transform_point3(CHASE_EYE), inv.transform_point3(p));
                // (Stopped a voxel and a half short of the point, so the blade's own fist never counts.)
                let d = e - o;
                let len = d.length();
                let d = d / len;
                let reach = len - rig.size * 1.5;
                !hits_box(o, d, lo, hi, reach)
                    || !rig.parts[i].cubes.iter().any(|(c, _)| hits_box(o, d, *c - Vec3::splat(h), *c + Vec3::splat(h), reach))
            })
        })
        .count();
    clear * 4 >= points.len()
}

/// Whether the ray from `o` along `d` (a unit vector) meets the box `lo..hi` within `reach`.
fn hits_box(o: Vec3, d: Vec3, lo: Vec3, hi: Vec3, reach: f32) -> bool {
    let inv = d.recip();
    let (a, b) = ((lo - o) * inv, (hi - o) * inv);
    let (near, far) = (a.min(b).max_element(), a.max(b).min_element());
    near <= far && far >= 0.0 && near <= reach
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
            assert_eq!(j.name(), Some(HUMANOID.bones[b]));
        }
        // Each mesh is bound wholly to its own bone, and the faces are the resident rig's.
        let mut faces = 0;
        for node in doc.nodes().filter(|n| n.mesh().is_some()) {
            let bone = HUMANOID.bones.iter().position(|b| node.name() == Some(&format!("{b}_voxels"))).unwrap();
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
