//! The boys' faces (ANIMATION.md §12.3, "their mouths"): the sculpted face does not move, so the
//! part of it round the mouth and the part round the eyes are cut out of the head and drawn in a
//! few versions, one at a time, riding the head: the mouth shut (as sculpted), parted as he talks,
//! or wide open in a shout (a dark mouth with his upper teeth and his tongue, the chin dropped a
//! little); the eyes open, or shut in a blink. The view picks the version each frame (`Face`) and
//! shrinks the others away into the head.

use std::collections::HashMap;

use glam::{Mat4, Vec3};

use crate::render::cast;
use crate::render::voxels::{lin, Rig, RigPart};

/// A boy's face as the view sets it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Face {
    /// 0 shut, 1 parted (talking), 2 wide open (a shout).
    pub mouth: u8,
    /// Shut in a blink.
    pub blink: bool,
}

/// The versions' parts, after the head's bones (`faced`): the mouth's three, then the eyes' two.
pub const MOUTHS: [&str; 3] = ["mouth.shut", "mouth.talk", "mouth.open"];
pub const EYES: [&str; 2] = ["eyes.open", "eyes.shut"];

/// A voxel of the grid, its linear colour and its shade mode.
type Cells = HashMap<(i32, i32, i32), (Vec3, f32)>;

/// The skin's shade mode (5 + `mat::SKIN`), the teeth's (ivory).
const SKIN: f32 = 10.0;
const IVORY: f32 = 11.0;

/// The rows round the mouth (the sculpt's mouth is the row at y 38, the nose at 40 and 41), and
/// round the eyes (43 and 44, the lid's line at 45; the brows at 47 stay with the head), across
/// the face; only the face's front (z 14 and up) is cut.
const MOUTH_BOX: ([i32; 2], [i32; 2]) = ([13, 22], [36, 41]);
const EYES_BOX: ([i32; 2], [i32; 2]) = ([10, 25], [42, 45]);
const FRONT: i32 = 14;

/// Where the boy's eyes are, along x (each three voxels wide, at y 43 and 44).
const EYE_XS: [[i32; 3]; 2] = [[12, 13, 14], [21, 22, 23]];

/**
 * A boy's jointed rig (`CastRig::Kid(..).jointed()`) with his faces: the voxels of the head in the
 * mouth's and the eyes' boxes cut out into a part for each version (`MOUTHS`, `EYES`), each riding
 * the head (its joint the head's, its parent the head). The head's own voxels round them are not
 * culled against them (they are other parts), so whichever is shown closes the face.
 */
pub fn faced(mut rig: Rig) -> Rig {
    let Some(head) = rig.index("head") else { return rig };
    let size = rig.size;
    let (w, _, d) = cast::KID_SIZE;
    let (ox, oz) = ((w - 1) as f32 / 2.0, (d - 1) as f32 / 2.0);
    let rest = rig.parts[head].rest;
    let to_grid = |off: Vec3| {
        let at = rest + off;
        ((ox + at.x / size).round() as i32, (at.y / size - 0.5).round() as i32, (at.z / size + oz).round() as i32)
    };
    let to_off = |(x, y, z): (i32, i32, i32)| Vec3::new((x as f32 - ox) * size, (y as f32 + 0.5) * size, (z as f32 - oz) * size) - rest;
    let inside = |(x, y, z): (i32, i32, i32), b: ([i32; 2], [i32; 2])| {
        (b.0[0]..=b.0[1]).contains(&x) && (b.1[0]..=b.1[1]).contains(&y) && z >= FRONT
    };
    let (mut mouth, mut eyes) = (Cells::new(), Cells::new());
    let part = &mut rig.parts[head];
    let mut keep = (Vec::new(), Vec::new());
    for (j, &(off, c)) in part.cubes.iter().enumerate() {
        let g = to_grid(off);
        let shade = part.shade.get(j).copied().unwrap_or(0.0);
        if inside(g, MOUTH_BOX) {
            mouth.insert(g, (c, shade));
        } else if inside(g, EYES_BOX) {
            eyes.insert(g, (c, shade));
        } else {
            keep.0.push((off, c));
            keep.1.push(shade);
        }
    }
    if mouth.is_empty() || eyes.is_empty() {
        return rig;
    }
    (part.cubes, part.shade) = keep;
    let skin = commonest(&eyes, SKIN).unwrap_or(lin(0xf0c8a0));
    let make = |name: &'static str, cells: &Cells| {
        let mut list: Vec<_> = cells.iter().collect();
        list.sort_unstable_by_key(|(g, _)| **g);
        let cubes: Vec<(Vec3, Vec3)> = list.iter().map(|(g, (c, _))| (to_off(**g), *c)).collect();
        let shade = list.iter().map(|(_, (_, s))| *s).collect();
        let min_y = cubes.iter().map(|(p, _)| p.y - size / 2.0).fold(f32::INFINITY, f32::min);
        RigPart { name, rest, cubes, shade, min_y: if min_y.is_finite() { min_y } else { 0.0 }, parent: Some(head) }
    };
    let versions = [mouth.clone(), parted(&mouth, skin, false), parted(&mouth, skin, true)];
    for (name, cells) in MOUTHS.iter().zip(&versions) {
        rig.parts.push(make(name, cells));
    }
    rig.parts.push(make(EYES[0], &eyes));
    rig.parts.push(make(EYES[1], &blinked(&eyes, skin)));
    rig
}

/// The commonest colour among the voxels of shade mode `shade`.
fn commonest(cells: &Cells, shade: f32) -> Option<Vec3> {
    let mut count: HashMap<[u32; 3], (usize, Vec3)> = HashMap::new();
    for (c, s) in cells.values() {
        if (*s - shade).abs() < 0.5 {
            count.entry(c.to_array().map(f32::to_bits)).or_insert((0, *c)).0 += 1;
        }
    }
    count.into_values().max_by_key(|(n, _)| *n).map(|(_, c)| c)
}

/// The frontmost voxel's depth at (x, y).
fn front(cells: &Cells, x: i32, y: i32) -> Option<i32> {
    cells.keys().filter(|g| g.0 == x && g.1 == y).map(|g| g.2).max()
}

/**
 * The mouth parted: talking, a dark slot four voxels wide where the lips were and two at its middle
 * over it; shouting (`wide`), a mouth eight voxels wide and three rows tall, dark inside, his upper
 * teeth under the lip, his tongue on its floor, the lower lip dropped below it.
 */
fn parted(shut: &Cells, skin: Vec3, wide: bool) -> Cells {
    let mut c = shut.clone();
    let maw = lin(0x2a0f0d);
    let tongue = lin(0x9a4440);
    let teeth = lin(0xece6d8);
    let lip = lin(0xb0584c);
    type Row = (i32, std::ops::RangeInclusive<i32>);
    let rows: &[Row] = if wide { &[(38, 14..=21), (39, 14..=21), (40, 15..=20)] } else { &[(38, 16..=19), (39, 17..=18)] };
    if wide {
        // (The tip of his nose drawn up out of the way: the sculpt's nose stands out a voxel at y 40.)
        for x in 17..=18 {
            if let (Some(z), Some(face)) = (front(&c, x, 40), front(&c, 16, 40)) {
                if z > face {
                    c.remove(&(x, 40, z));
                }
            }
        }
    }
    for (y, xs) in rows {
        for x in xs.clone() {
            let Some(z) = front(&c, x, *y) else { continue };
            let middle = (16..=19).contains(&x);
            c.remove(&(x, *y, z));
            let back = match (*y, middle) {
                // His upper teeth under the lip, his tongue on the mouth's floor.
                (40, true) => (teeth, IVORY),
                (38, true) if wide => (tongue, SKIN),
                _ => (maw, SKIN),
            };
            c.insert((x, *y, z - 1), back);
        }
    }
    if wide {
        // The chin dropped: the lower lip under the open mouth, over the scarf.
        for x in 14..=21 {
            if let Some(z) = front(shut, x, 38) {
                let edge = x == 14 || x == 21;
                c.insert((x, 37, z), (if edge { skin * 0.86 } else { lip }, SKIN));
            }
        }
    }
    c
}

/// The eyes shut: the lids down over them (the skin a shade darker), the lashes a line beneath.
fn blinked(open: &Cells, skin: Vec3) -> Cells {
    let mut c = open.clone();
    for xs in EYE_XS {
        for x in xs {
            for (y, col) in [(44, skin * 0.84), (43, lin(0x2a1c16))] {
                if let Some(z) = front(&c, x, y) {
                    c.insert((x, y, z), (col, SKIN));
                }
            }
        }
    }
    c
}

/**
 * The faces' matrices for `face`: the versions shown ride the head (`mats[head]`); the others are
 * shrunk to nothing at its joint, inside it. Leaves `mats` alone for a rig without faces.
 */
pub fn wear(rig: &Rig, mats: &mut [Mat4], face: Face) {
    let Some(head) = rig.index("head") else { return };
    let Some(first) = rig.index(MOUTHS[0]) else { return };
    let on = mats[head];
    let off = on * Mat4::from_scale(Vec3::splat(1e-4));
    for (k, m) in mats.iter_mut().enumerate().skip(first).take(MOUTHS.len() + EYES.len()) {
        let i = k - first;
        let shown = if i < MOUTHS.len() { i == face.mouth.min(2) as usize } else { (i - MOUTHS.len() == 1) == face.blink };
        *m = if shown { on } else { off };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::anim::cast_rigs::CastRig;

    #[test]
    fn every_boys_face_opens_and_blinks_and_keeps_the_head_whole() {
        for k in 0..4 {
            let plain = CastRig::Kid(k).jointed();
            let rig = faced(CastRig::Kid(k).jointed());
            let head = rig.index("head").unwrap();
            assert_eq!(rig.parts.len(), plain.parts.len() + MOUTHS.len() + EYES.len());
            // The head with its shut mouth and open eyes is the head as sculpted.
            let n = |name: &str| rig.parts[rig.index(name).unwrap()].cubes.len();
            assert_eq!(n("head") + n(MOUTHS[0]) + n(EYES[0]), plain.parts[head].cubes.len(), "kid {k}");
            assert!(n(MOUTHS[0]) > 40 && n(EYES[0]) > 40, "kid {k}: {} {}", n(MOUTHS[0]), n(EYES[0]));
            // The versions differ: dark inside an open mouth, the eyes covered in a blink.
            let dark = |name: &str| rig.parts[rig.index(name).unwrap()].cubes.iter().filter(|(_, c)| c.length() < 0.05).count();
            assert!(dark(MOUTHS[2]) > dark(MOUTHS[1]) && dark(MOUTHS[1]) > dark(MOUTHS[0]), "kid {k}");
            let white = |name: &str| rig.parts[rig.index(name).unwrap()].cubes.iter().filter(|(_, c)| c.min_element() > 0.8).count();
            assert!(white(EYES[0]) >= 4 && white(EYES[1]) == 0, "kid {k}: the whites of his eyes");
            for name in MOUTHS.iter().chain(&EYES) {
                assert_eq!(rig.parts[rig.index(name).unwrap()].parent, Some(head));
            }
            // One mouth and one pair of eyes worn at a time, on the head.
            let mut mats = vec![Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0)); rig.parts.len()];
            wear(&rig, &mut mats, Face { mouth: 2, blink: true });
            let shown: Vec<&str> = rig
                .parts
                .iter()
                .zip(&mats)
                .filter(|(p, m)| p.parent == Some(head) && m.x_axis.length() > 0.5)
                .map(|(p, _)| p.name)
                .collect();
            assert_eq!(shown, vec![MOUTHS[2], EYES[1]]);
        }
    }
}
