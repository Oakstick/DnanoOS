//! Which faces of the static scenery can be seen (RP11 in the Technical Art and Rendering Plan).
//!
//! The level is built of instanced cubes. A cube's face is shut when a neighbour of exactly the
//! same shape and turn sits one cube over on that side (so the face is wholly covered) and that
//! neighbour hides what is behind it (`Cover`). Everything else stays open, and only open faces
//! are drawn. The index is kept after the level is built, so a cube hidden later (a wall blown
//! up, a coin taken, a hedge burnt) reopens just its neighbours' faces, and a cube added later
//! (the floor patch under the wall) finds its own, without going over the level again.

use glam::{Mat3, Vec3};

use super::view::Instance;

/// A cheap hasher for the index's integer keys.
#[derive(Default)]
pub struct KeyHasher(u64);

impl std::hash::Hasher for KeyHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = (self.0 ^ *b as u64).wrapping_mul(0x100_0000_01b3);
        }
    }

    fn write_u32(&mut self, v: u32) {
        self.0 = (self.0.rotate_left(17) ^ v as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    }

    fn write_i32(&mut self, v: i32) {
        self.write_u32(v as u32);
    }
}

type KeyMap<K, V> = std::collections::HashMap<K, V, std::hash::BuildHasherDefault<KeyHasher>>;

/// How a cube hides its neighbours' faces: not at all (see-through, water, mist, or hidden),
/// always, or only those of cubes that fade with it (a fading tree's own voxels).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cover {
    None,
    Solid,
    Fading,
}

pub fn cover(c: &Instance) -> Cover {
    let shade = c.emissive[3];
    if (shade > 4.5 && shade < 5.5) || shade > 19.5 {
        Cover::None
    } else if shade > 2.5 && shade < 3.5 {
        // Dithered (a tree out of the camera's way): its alpha comes and goes, its cover stays.
        Cover::Fading
    } else if c.color[3] < 0.999 {
        Cover::None
    } else {
        Cover::Solid
    }
}

/// A cube's turn and scale (quantised) and its place in units of itself (neighbours 64 apart).
type Shape = ([i32; 9], [i32; 3]);

fn shape(c: &Instance) -> Option<Shape> {
    let m = &c.m;
    let basis = Mat3::from_cols_array(&[m[0], m[1], m[2], m[4], m[5], m[6], m[8], m[9], m[10]]);
    if cover(c) == Cover::None || basis.determinant().abs() < 1e-12 {
        return None;
    }
    let cols = basis.to_cols_array();
    let q: [i32; 9] = std::array::from_fn(|k| (cols[k] * 4096.0).round() as i32);
    let p = basis.inverse() * Vec3::new(m[12], m[13], m[14]) * 64.0;
    Some((q, [p.x.round() as i32, p.y.round() as i32, p.z.round() as i32]))
}

/// One cube over, face by face: +x, -x, +y, -y, +z, -z (the opposite face is `f ^ 1`).
const STEPS: [[i32; 3]; 6] = [[64, 0, 0], [-64, 0, 0], [0, 64, 0], [0, -64, 0], [0, 0, 64], [0, 0, -64]];

pub struct StaticFaces {
    lattices: KeyMap<[i32; 9], u32>,
    at: KeyMap<(u32, i32, i32, i32), u32>,
    keys: Vec<Option<(u32, [i32; 3])>>,
    covers: Vec<Cover>,
    /// Each cube's open faces, a bit each.
    pub open: Vec<u8>,
}

/// Runs `f` over `items` in a few even parts on every core, keeping the order.
fn par_map<T: Sync, R: Send>(items: &[T], f: impl Fn(usize, &T) -> R + Sync) -> Vec<R> {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).clamp(1, 16);
    let span = items.len().div_ceil(threads).max(1);
    let f = &f;
    std::thread::scope(|s| {
        let parts: Vec<_> = items
            .chunks(span)
            .enumerate()
            .map(|(k, part)| s.spawn(move || part.iter().enumerate().map(|(i, t)| f(k * span + i, t)).collect::<Vec<R>>()))
            .collect();
        parts.into_iter().flat_map(|h| h.join().expect("face thread")).collect()
    })
}

impl StaticFaces {
    pub fn build(cubes: &[Instance]) -> Self {
        let shapes = par_map(cubes, |_, c| shape(c));
        let mut index = Self {
            lattices: KeyMap::default(),
            at: KeyMap::default(),
            keys: Vec::with_capacity(cubes.len()),
            covers: par_map(cubes, |_, c| cover(c)),
            open: Vec::new(),
        };
        index.at.reserve(cubes.len());
        for (i, s) in shapes.into_iter().enumerate() {
            let key = s.map(|s| index.place(i as u32, s));
            index.keys.push(key);
        }
        let open = par_map(&index.keys, |i, _| index.open_of(i as u32));
        index.open = open;
        index
    }

    fn place(&mut self, slot: u32, (q, p): Shape) -> (u32, [i32; 3]) {
        let next = self.lattices.len() as u32;
        let id = *self.lattices.entry(q).or_insert(next);
        self.at.insert((id, p[0], p[1], p[2]), slot);
        (id, p)
    }

    fn neighbour(&self, (id, p): (u32, [i32; 3]), f: usize) -> Option<u32> {
        let s = STEPS[f];
        self.at.get(&(id, p[0] + s[0], p[1] + s[1], p[2] + s[2])).copied()
    }

    fn open_of(&self, slot: u32) -> u8 {
        let Some(key) = self.keys[slot as usize] else { return 0x3f };
        let mine = self.covers[slot as usize];
        (0..6)
            .filter(|&f| {
                let shut = self.neighbour(key, f).is_some_and(|j| match self.covers[j as usize] {
                    Cover::Solid => true,
                    Cover::Fading => mine == Cover::Fading,
                    Cover::None => false,
                });
                !shut
            })
            .fold(0, |bits, f| bits | 1 << f)
    }

    /// A cube added in the next free slot: its open faces.
    pub fn add(&mut self, slot: u32, c: &Instance) -> u8 {
        debug_assert_eq!(slot as usize, self.keys.len());
        self.covers.push(cover(c));
        let key = shape(c).map(|s| self.place(slot, s));
        self.keys.push(key);
        let open = self.open_of(slot);
        self.open.push(open);
        open
    }

    /// The cube in `slot` was rewritten. If only its look changed, nothing to do; if it was
    /// hidden, the faces of its neighbours that it covered are opened, and returned (slot times 8
    /// plus the face). None if it moved, which this index cannot follow.
    pub fn update(&mut self, slot: u32, c: &Instance) -> Option<Vec<u32>> {
        let s = slot as usize;
        let now = shape(c);
        let was = self.keys[s];
        match (was, now) {
            (Some((id, p)), Some((q, np))) if self.lattices.get(&q) == Some(&id) && p == np => {
                self.covers[s] = cover(c);
                Some(Vec::new())
            }
            (None, None) => Some(Vec::new()),
            (Some(key), None) => {
                self.at.remove(&(key.0, key.1[0], key.1[1], key.1[2]));
                self.keys[s] = None;
                self.covers[s] = Cover::None;
                let mut opened = Vec::new();
                for f in 0..6 {
                    if let Some(j) = self.neighbour(key, f) {
                        let back = f ^ 1;
                        if self.open[j as usize] & (1 << back) == 0 {
                            self.open[j as usize] |= 1 << back;
                            opened.push(j << 3 | back as u32);
                        }
                    }
                }
                Some(opened)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::view::SHADE_LIT;
    use glam::Mat4;

    fn cube(at: Vec3, size: f32) -> Instance {
        Instance::new(
            Mat4::from_scale_rotation_translation(Vec3::splat(size), glam::Quat::IDENTITY, at),
            Vec3::ONE,
            1.0,
            Vec3::ZERO,
            SHADE_LIT,
        )
    }

    #[test]
    fn touching_cubes_shut_the_faces_between_them() {
        let cubes = [cube(Vec3::ZERO, 0.5), cube(Vec3::new(0.5, 0.0, 0.0), 0.5), cube(Vec3::new(3.0, 0.0, 0.0), 0.5)];
        let f = StaticFaces::build(&cubes);
        assert_eq!(f.open[0], 0x3f & !1, "+x shut");
        assert_eq!(f.open[1], 0x3f & !2, "-x shut");
        assert_eq!(f.open[2], 0x3f);
        // A cube of another size beside it covers nothing.
        let f = StaticFaces::build(&[cube(Vec3::ZERO, 0.5), cube(Vec3::new(0.75, 0.0, 0.0), 1.0)]);
        assert_eq!(f.open, vec![0x3f, 0x3f]);
    }

    #[test]
    fn hiding_a_cube_reopens_its_neighbours_and_adding_one_finds_its_faces() {
        let cubes = [cube(Vec3::ZERO, 0.5), cube(Vec3::new(0.5, 0.0, 0.0), 0.5)];
        let mut f = StaticFaces::build(&cubes);
        let mut gone = cubes[1];
        gone.m = (Mat4::from_translation(Vec3::new(0.0, -50.0, 0.0)) * Mat4::from_scale(Vec3::ZERO)).to_cols_array();
        assert_eq!(f.update(1, &gone), Some(vec![0 << 3]));
        assert_eq!(f.open[0], 0x3f);
        // A fade changes the look, not the shape.
        let mut faded = cubes[0];
        faded.color[3] = 0.4;
        faded.emissive[3] = 3.0;
        assert_eq!(f.update(0, &faded), Some(vec![]));
        // Moved: the index cannot follow.
        assert_eq!(f.update(0, &cube(Vec3::new(9.0, 0.0, 0.0), 0.5)), None);
        let mut f = StaticFaces::build(&cubes[..1]);
        assert_eq!(f.add(1, &cubes[1]), 0x3f & !2);
    }
}
