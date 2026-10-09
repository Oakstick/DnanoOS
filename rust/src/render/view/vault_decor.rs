//! The art of the Vaults of Mammon that is not architecture: statues for the
//! wall niches (a gilded king with sceptre and orb, a marble goddess with a
//! gold amphora, a golden lion, a knight in gilded armour leaning on his
//! sword, and Mammon himself hugging a sack of coins), tall gold urns, heaps
//! of treasure, and paintings in gilt frames (Mammon enthroned, a golden city
//! at sunset, a still life of goblets and grapes, a ship at sea, a dragon on
//! its hoard, a noble's portrait).
//!
//! Everything here is merged into boxes: each run of same-coloured voxels
//! along a row becomes one stretched box, so the hundreds of decorations in
//! the vaults cost a fraction of their voxels.

use super::vault_art::{SHADE_COINS, SHADE_GILT};
use super::*;
use crate::render::carve::{ball, limb, v};
use crate::render::characters::{mat, Sculpt};

/// Polished marble, as a voxel material (shader kind 10, shade mode 15).
const MARBLE_MAT: u32 = 10 << 24;

mod pal {
    use super::{mat, MARBLE_MAT};
    // Gold by its reflectance (`vault_art::SHADE_GILT` draws it).
    pub const G: u32 = 0xffd47a | mat::GOLD;
    pub const GD: u32 = 0xd9ab58 | mat::GOLD;
    pub const GL: u32 = 0xffdd94 | mat::GOLD;
    pub const M: u32 = 0xf0e8da | MARBLE_MAT;
    pub const MD: u32 = 0xd2c6b0 | MARBLE_MAT;
    pub const STEEL: u32 = 0xd4d8e0 | mat::METAL;
    pub const RUBY: u32 = 0xe0203a | mat::SLIME;
    pub const SAPPHIRE: u32 = 0x3a6aff | mat::SLIME;
    pub const EMERALD: u32 = 0x2ad06a | mat::SLIME;
    pub const PEARL: u32 = 0xf6f0ff | mat::IVORY;
    /// A heap's coins (`vault_art::SHADE_COINS` draws them).
    pub const COIN: u32 = 0xf2cc78 | mat::GOLD;
    pub const DARK: u32 = 0x1a1410;
    pub const VELVET: u32 = 0x8a1a22 | mat::CLOTH;
    pub const SACK: u32 = 0x7a5a36 | mat::CLOTH;
}
use pal::*;

/// A box of a merged model: its centre and size in the model's own units, its colour, its shade.
#[derive(Clone, Copy, Debug)]
pub struct MBox {
    pub center: Vec3,
    pub size: Vec3,
    pub color: Vec3,
    pub shade: f32,
}

/// A model as merged boxes, centred on x/z with its base at y = 0 (as `mesh_from`).
pub struct Merged {
    pub boxes: Vec<MBox>,
}

impl Merged {
    /// How tall it stands.
    #[cfg(test)]
    pub fn height(&self) -> f32 {
        self.boxes.iter().map(|b| b.center.y + b.size.y / 2.0).fold(0.0, f32::max)
    }
}

fn shade_of(c: u32) -> f32 {
    match c >> 24 {
        0 => SHADE_LIT,
        m => 5.0 + m as f32,
    }
}

/// Merges a sculpt's voxels into boxes: every run of one colour along x becomes one box.
pub fn merge(c: &Collected, size: f32) -> Merged {
    use std::collections::BTreeMap;
    let ox = (c.width - 1) as f32 / 2.0;
    let oz = (c.depth - 1) as f32 / 2.0;
    // (y, z) -> the row's voxels by x.
    let mut rows: BTreeMap<(i32, i32), Vec<(i32, u32)>> = BTreeMap::new();
    for vx in &c.voxels {
        rows.entry((vx.y, vx.z)).or_default().push((vx.x, vx.c));
    }
    let mut boxes = Vec::new();
    for ((y, z), mut row) in rows {
        row.sort_by_key(|r| r.0);
        let mut k = 0;
        while k < row.len() {
            let (x0, col) = row[k];
            let mut x1 = x0;
            while k + 1 < row.len() && row[k + 1].0 == x1 + 1 && row[k + 1].1 == col {
                k += 1;
                x1 += 1;
            }
            let n = (x1 - x0 + 1) as f32;
            boxes.push(MBox {
                center: Vec3::new(((x0 + x1) as f32 / 2.0 - ox) * size, (y as f32 + 0.5) * size, (z as f32 - oz) * size),
                size: Vec3::new(n * size, size, size),
                color: lin(col & 0x00ff_ffff),
                shade: shade_of(col),
            });
            k += 1;
        }
    }
    Merged { boxes }
}

/// Merges a solid model (`Sculpt::finish_solid`, nothing hollow) into boxes layer by layer: each
/// layer's runs of one colour grown into rectangles, so a heap is a few dozen boxes.
pub fn merge_solid(c: &Collected, size: f32) -> Merged {
    use std::collections::BTreeMap;
    let ox = (c.width - 1) as f32 / 2.0;
    let oz = (c.depth - 1) as f32 / 2.0;
    let mut layers: BTreeMap<i32, BTreeMap<(i32, i32), u32>> = BTreeMap::new();
    for vx in &c.voxels {
        layers.entry(vx.y).or_default().insert((vx.z, vx.x), vx.c);
    }
    let mut boxes = Vec::new();
    for (y, mut cells) in layers {
        while let Some((&(z0, x0), &col)) = cells.iter().next() {
            // As far along x as the colour runs, then as many rows on in z as match it whole.
            let mut x1 = x0;
            while cells.get(&(z0, x1 + 1)) == Some(&col) {
                x1 += 1;
            }
            let mut z1 = z0;
            while (x0..=x1).all(|x| cells.get(&(z1 + 1, x)) == Some(&col)) {
                z1 += 1;
            }
            for z in z0..=z1 {
                for x in x0..=x1 {
                    cells.remove(&(z, x));
                }
            }
            let (nx, nz) = ((x1 - x0 + 1) as f32, (z1 - z0 + 1) as f32);
            boxes.push(MBox {
                center: Vec3::new(((x0 + x1) as f32 / 2.0 - ox) * size, (y as f32 + 0.5) * size, ((z0 + z1) as f32 / 2.0 - oz) * size),
                size: Vec3::new(nx * size, size, nz * size),
                color: lin(col & 0x00ff_ffff),
                shade: shade_of(col),
            });
        }
    }
    Merged { boxes }
}

/// Gives the boxes of colour `hex` (as painted, its material bits aside) the shade `shade`.
pub fn reshade(m: &mut Merged, hex: u32, shade: f32) {
    let c = lin(hex & 0x00ff_ffff);
    for b in m.boxes.iter_mut().filter(|b| (b.color - c).abs().max_element() < 1e-4) {
        b.shade = shade;
    }
}

/// Puts a merged model's boxes into `out` under `m`, each glowing a little (`glow` of its colour).
pub fn push_merged(out: &mut Vec<Instance>, model: &Merged, m: Mat4, glow: f32) {
    for b in &model.boxes {
        let bm = m * Mat4::from_scale_rotation_translation(b.size, Quat::IDENTITY, b.center);
        out.push(Instance::new(bm, b.color, 1.0, b.color * glow, b.shade));
    }
}

// ------------------------------------------------------------------ statues

/// The statues' size in voxels, and their voxel: about half a tile across and one and a quarter tall.
const STATUE: (i32, i32, i32) = (20, 46, 15);
pub const STATUE_VOXEL: f32 = 0.027;

/// A cone of robe from `y0` to `y1`, narrowing from (rx0, rz0) to (rx1, rz1), with folds in `fold`.
#[allow(clippy::too_many_arguments)]
fn robe(s: &mut Sculpt, cx: f32, cz: f32, y0: i32, y1: i32, r0: (f32, f32), r1: (f32, f32), c: u32, fold: u32) {
    for y in y0..=y1 {
        let t = (y - y0) as f32 / (y1 - y0).max(1) as f32;
        let (rx, rz) = (r0.0 + (r1.0 - r0.0) * t, r0.1 + (r1.1 - r0.1) * t);
        for z in 0..STATUE.2 {
            for x in 0..STATUE.0 {
                let (dx, dz) = ((x as f32 - cx) / rx, (z as f32 - cz) / rz);
                if dx * dx + dz * dz <= 1.0 {
                    let a = (z as f32 - cz).atan2(x as f32 - cx);
                    s.set(x, y, z, if (a * 6.0).sin() > 0.6 { fold } else { c });
                }
            }
        }
    }
}

/// A ring (a crown, a girdle, a diadem) round (cx, cz) at height y.
fn ring(s: &mut Sculpt, cx: f32, cz: f32, y: i32, r: f32, c: u32) {
    for z in 0..STATUE.2 {
        for x in 0..STATUE.0 {
            let d = ((x as f32 - cx).powi(2) + ((z as f32 - cz) * 1.2).powi(2)).sqrt();
            if (r - 0.8..=r + 0.4).contains(&d) {
                s.set(x, y, z, c);
            }
        }
    }
}

/// A gilded king: robe and cape, a beard, a crown set with rubies, a sceptre and an orb.
fn king() -> Collected {
    let mut s = Sculpt::new(STATUE.0, STATUE.1, STATUE.2);
    let (cx, cz) = (9.5, 7.0);
    robe(&mut s, cx, cz, 0, 22, (7.0, 5.2), (5.0, 3.8), G, GD);
    // A cape falling behind.
    for y in 4..31 {
        let w = 5.5 + (31 - y) as f32 * 0.06;
        for x in (cx - w) as i32..=(cx + w) as i32 {
            s.set(x, y, 2, GD);
            s.set(x, y, 3, GD);
        }
    }
    ball(&mut s, v(cx, 27.0, cz), v(5.2, 5.5, 3.6), G);
    ring(&mut s, cx, cz, 22, 5.0, GL);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 5.0, 31.0, cz), v(2.2, 2.0, 2.2), GL);
        limb(
            &mut s,
            &[(v(cx + side * 5.4, 30.0, cz), 1.6), (v(cx + side * 5.6, 25.0, cz + 1.5), 1.4), (v(cx + side * 4.4, 22.5, cz + 3.4), 1.3)],
            G,
        );
    }
    // Sceptre in the right hand, orb in the left.
    s.capsule((cx + 4.4, 15.0, cz + 3.6), (cx + 4.4, 38.0, cz + 3.6), 0.6, 0.6, GL);
    ball(&mut s, v(cx + 4.4, 39.0, cz + 3.6), Vec3::splat(1.3), RUBY);
    ball(&mut s, v(cx - 4.4, 23.5, cz + 4.4), Vec3::splat(1.8), GL);
    s.cuboid((cx - 4.4) as i32, 25, (cz + 4.4) as i32, (cx - 4.4) as i32, 27, (cz + 4.4) as i32, GL);
    // The head, beard and crown.
    ball(&mut s, v(cx, 35.5, cz + 0.5), v(3.2, 3.6, 3.0), G);
    ball(&mut s, v(cx, 32.6, cz + 2.2), v(2.6, 2.6, 1.5), GD);
    for (dx, c) in [(-1.2f32, DARK), (1.2, DARK)] {
        s.set((cx + dx).round() as i32, 36, (cz + 3.3).round() as i32, c);
    }
    for y in 39..=41 {
        ring(&mut s, cx, cz + 0.5, y, 3.1, if y == 40 { GL } else { G });
    }
    for k in 0..8 {
        let a = k as f32 / 8.0 * TAU;
        let (x, z) = ((cx + a.cos() * 3.0).round() as i32, (cz + 0.5 + a.sin() * 2.6).round() as i32);
        s.set(x, 42, z, GL);
        s.set(x, 43, z, if k % 2 == 0 { RUBY } else { GL });
    }
    s.finish(0.0)
}

/// A marble goddess in a flowing robe, a gold amphora on her shoulder and a gold diadem.
fn goddess() -> Collected {
    let mut s = Sculpt::new(STATUE.0, STATUE.1, STATUE.2);
    let (cx, cz) = (10.0, 7.0);
    robe(&mut s, cx, cz, 0, 27, (5.6, 4.4), (3.6, 2.8), M, MD);
    ring(&mut s, cx, cz, 27, 3.6, G);
    ball(&mut s, v(cx, 30.5, cz + 0.2), v(3.8, 4.0, 2.8), M);
    ball(&mut s, v(cx, 33.6, cz), v(1.2, 1.4, 1.2), M);
    ball(&mut s, v(cx, 36.8, cz + 0.4), v(2.6, 3.0, 2.6), M);
    ball(&mut s, v(cx, 38.0, cz - 2.0), v(1.8, 1.8, 1.6), MD);
    ring(&mut s, cx, cz + 0.4, 38, 2.5, G);
    // The right arm down along the robe; the left raised to the amphora on her shoulder.
    limb(&mut s, &[(v(cx + 3.6, 32.0, cz), 1.2), (v(cx + 4.4, 26.0, cz + 0.8), 1.0), (v(cx + 4.4, 21.0, cz + 1.4), 1.0)], M);
    limb(&mut s, &[(v(cx - 3.6, 32.0, cz), 1.2), (v(cx - 5.4, 36.0, cz + 0.5), 1.0), (v(cx - 4.4, 39.5, cz + 0.5), 1.0)], M);
    ball(&mut s, v(cx - 5.0, 41.5, cz + 0.2), v(2.2, 2.6, 2.0), G);
    s.capsule((cx - 5.0, 43.0, cz + 0.2), (cx - 5.0, 45.0, cz + 0.2), 0.9, 1.2, G);
    for y in [40, 42] {
        s.set((cx - 7.2) as i32, y, cz as i32, GD);
    }
    s.finish(0.0)
}

/// A golden lion sitting up, its mane darker, ruby eyes.
fn lion() -> Collected {
    let mut s = Sculpt::new(STATUE.0, STATUE.1, STATUE.2);
    let (cx, cz) = (10.0, 7.0);
    ball(&mut s, v(cx, 7.0, cz - 1.5), v(6.2, 7.0, 5.4), G);
    ball(&mut s, v(cx, 14.5, cz + 1.0), v(4.6, 7.0, 4.0), G);
    for side in [-1.0f32, 1.0] {
        limb(&mut s, &[(v(cx + side * 2.6, 13.0, cz + 3.0), 1.7), (v(cx + side * 2.6, 2.0, cz + 4.6), 1.5)], G);
        ball(&mut s, v(cx + side * 2.6, 1.2, cz + 5.4), v(1.8, 1.2, 2.0), GL);
        ball(&mut s, v(cx + side * 5.4, 3.0, cz - 0.5), v(2.0, 3.0, 3.4), G);
    }
    // The mane, and the face set in it.
    ball(&mut s, v(cx, 22.5, cz + 0.5), v(5.6, 5.6, 4.6), GD);
    s.paint(|x, y, z, c| (y >= 17 && c == GD && (x + y * 3 + z * 7) % 5 == 0).then_some(G));
    ball(&mut s, v(cx, 22.0, cz + 3.6), v(3.0, 3.2, 2.4), G);
    ball(&mut s, v(cx, 20.4, cz + 5.4), v(1.8, 1.3, 1.4), GL);
    for side in [-1.0f32, 1.0] {
        s.set((cx + side * 1.3).round() as i32, 23, (cz + 5.6).round() as i32, RUBY);
        ball(&mut s, v(cx + side * 3.0, 26.4, cz + 2.0), Vec3::splat(1.1), G);
    }
    s.set(cx as i32, 19, (cz + 6.4) as i32, DARK);
    // The tail curled round beside him.
    limb(&mut s, &[(v(cx + 5.0, 2.0, cz - 4.0), 1.0), (v(cx + 7.4, 1.6, cz + 1.0), 0.9), (v(cx + 7.0, 4.0, cz + 4.0), 1.2)], GD);
    s.finish(0.0)
}

/// A knight in gilded armour, plumed helm, both hands on the pommel of the sword before him.
fn knight() -> Collected {
    let mut s = Sculpt::new(STATUE.0, STATUE.1, STATUE.2);
    let (cx, cz) = (10.0, 6.5);
    for side in [-1.0f32, 1.0] {
        limb(&mut s, &[(v(cx + side * 2.2, 15.0, cz), 2.0), (v(cx + side * 2.3, 2.0, cz + 0.3), 1.7)], GD);
        ball(&mut s, v(cx + side * 2.3, 1.0, cz + 1.2), v(1.8, 1.0, 2.6), G);
        ball(&mut s, v(cx + side * 2.2, 8.0, cz + 1.0), Vec3::splat(1.5), GL);
    }
    ball(&mut s, v(cx, 21.5, cz), v(5.0, 7.5, 3.5), G);
    // A red tabard down the front, with a gold coin on it.
    for y in 11..26 {
        for x in (cx - 2.5) as i32..=(cx + 2.5) as i32 {
            if let Some(z) = (0..STATUE.2).rev().find(|&z| s.get(x, y, z).is_some()) {
                s.set(x, y, z, VELVET);
            }
        }
    }
    ball(&mut s, v(cx, 20.0, cz + 3.6), v(1.4, 1.4, 0.6), GL);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 4.8, 27.0, cz), Vec3::splat(2.4), GL);
        limb(
            &mut s,
            &[(v(cx + side * 5.0, 26.0, cz), 1.5), (v(cx + side * 4.0, 21.0, cz + 2.4), 1.3), (v(cx + side * 1.3, 20.0, cz + 4.4), 1.2)],
            G,
        );
    }
    // The sword, point down before him.
    s.capsule((cx, 1.0, cz + 4.6), (cx, 17.0, cz + 4.6), 0.5, 0.9, STEEL);
    s.cuboid((cx - 3.0) as i32, 17, (cz + 4.6) as i32, (cx + 3.0) as i32, 17, (cz + 4.6) as i32, GD);
    s.capsule((cx, 17.0, cz + 4.6), (cx, 21.0, cz + 4.6), 0.6, 0.6, GD);
    ball(&mut s, v(cx, 21.6, cz + 4.6), Vec3::splat(1.0), RUBY);
    // The helm, its visor slit, and the plume.
    ball(&mut s, v(cx, 32.0, cz), v(3.0, 3.6, 3.0), GL);
    for x in (cx - 1.6) as i32..=(cx + 1.6) as i32 {
        if let Some(z) = (0..STATUE.2).rev().find(|&z| s.get(x, 32, z).is_some()) {
            s.set(x, 32, z, DARK);
        }
    }
    limb(
        &mut s,
        &[(v(cx, 35.0, cz), 1.2), (v(cx, 38.0, cz - 1.0), 1.4), (v(cx, 38.5, cz - 4.0), 1.0), (v(cx, 35.0, cz - 5.0), 0.7)],
        VELVET,
    );
    s.finish(0.0)
}

/// Mammon himself: a fat horned figure in gold, grinning, a sack of coins in his arms.
fn mammon() -> Collected {
    let mut s = Sculpt::new(STATUE.0, STATUE.1, STATUE.2);
    let (cx, cz) = (10.0, 6.5);
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 3.0, 3.0, cz + 0.5), v(2.4, 3.2, 2.6), GD);
    }
    ball(&mut s, v(cx, 13.0, cz + 0.6), v(7.0, 8.4, 5.4), G);
    ball(&mut s, v(cx, 22.5, cz), v(6.2, 4.2, 4.0), G);
    ring(&mut s, cx, cz + 0.6, 11, 6.6, GL);
    // The sack, coins spilling from its mouth.
    ball(&mut s, v(cx, 14.0, cz + 5.8), v(3.6, 3.4, 2.0), SACK);
    for (x, y) in [(-1, 17), (0, 18), (1, 17), (2, 18), (-2, 18), (0, 19)] {
        s.set((cx as i32) + x, y, (cz + 6.0) as i32, GL);
    }
    for side in [-1.0f32, 1.0] {
        limb(
            &mut s,
            &[(v(cx + side * 5.8, 22.0, cz), 1.9), (v(cx + side * 5.6, 16.0, cz + 3.4), 1.6), (v(cx + side * 3.4, 14.5, cz + 5.6), 1.5)],
            G,
        );
    }
    ball(&mut s, v(cx, 29.5, cz + 0.6), v(3.8, 3.8, 3.4), G);
    for x in (cx - 2.0) as i32..=(cx + 2.0) as i32 {
        s.set(x, 28, (cz + 3.8) as i32, DARK);
    }
    for side in [-1.0f32, 1.0] {
        s.set((cx + side * 1.4).round() as i32, 30, (cz + 3.8).round() as i32, RUBY);
        limb(
            &mut s,
            &[(v(cx + side * 2.6, 32.0, cz), 1.1), (v(cx + side * 5.0, 35.5, cz), 0.9), (v(cx + side * 4.6, 39.5, cz - 0.5), 0.5)],
            GD,
        );
    }
    s.finish(0.0)
}

/// A tall gold urn with two handles, its body banded and set with a jewel.
fn urn() -> Collected {
    let mut s = Sculpt::new(STATUE.0, STATUE.1, STATUE.2);
    let (cx, cz) = (10.0, 7.0);
    s.ellipsoid((cx, 2.0, cz), (3.0, 2.0, 3.0), 0, GD);
    s.capsule((cx, 2.0, cz), (cx, 6.0, cz), 1.6, 1.4, G);
    ball(&mut s, v(cx, 15.0, cz), v(6.4, 9.0, 5.6), G);
    for y in [11, 19] {
        ring(&mut s, cx, cz, y, 6.0, GL);
    }
    s.capsule((cx, 22.0, cz), (cx, 29.0, cz), 2.6, 2.4, G);
    ring(&mut s, cx, cz, 29, 3.4, GL);
    ring(&mut s, cx, cz, 30, 3.6, GD);
    for side in [-1.0f32, 1.0] {
        limb(&mut s, &[(v(cx + side * 2.6, 27.0, cz), 0.8), (v(cx + side * 6.0, 27.0, cz), 0.8), (v(cx + side * 6.4, 21.0, cz), 0.8)], GD);
    }
    ball(&mut s, v(cx, 15.0, cz + 5.4), Vec3::splat(1.3), SAPPHIRE);
    s.finish(0.0)
}

/// Gives a merged model's gold the vaults' gilt.
fn gilt(mut m: Merged) -> Merged {
    for c in [G, GD, GL] {
        reshade(&mut m, c, SHADE_GILT);
    }
    m
}

/// Every statue design, then the urn last.
pub fn statues() -> Vec<Merged> {
    [king(), goddess(), lion(), knight(), mammon(), urn()].iter().map(|c| gilt(merge(c, STATUE_VOXEL))).collect()
}

// ------------------------------------------------------------------ treasure

/// A heap of treasure: gold coins (solid, drawn by the shader), a goblet, a crown, a plate and a
/// bar, pearls and gems on its slopes.
pub fn treasure() -> Merged {
    let mut s = Sculpt::new(17, 12, 17);
    s.ellipsoid((8.0, 0.0, 8.0), (8.0, 5.0, 7.0), 0, COIN);
    s.ellipsoid((5.0, 0.5, 10.5), (4.0, 3.5, 3.6), 0, COIN);
    // Gems lying on its slopes (only where nothing lies over them).
    let mut gems = Vec::new();
    for z in 0..17 {
        for x in 0..17 {
            let Some(y) = (0..12).rev().find(|&y| s.get(x, y, z).is_some()) else { continue };
            if y >= 2 && (x * 7 + z * 5 + y * 13) % 23 == 0 {
                gems.push((x, y + 1, z, [RUBY, SAPPHIRE, EMERALD][((x + z) % 3) as usize]));
            }
        }
    }
    for (x, y, z, c) in gems {
        s.set(x, y, z, c);
    }
    // A goblet standing in the heap.
    s.cuboid(11, 4, 6, 11, 6, 6, GD);
    s.ellipsoid((11.0, 8.0, 6.0), (1.8, 1.6, 1.8), 6, GL);
    // A crown tipped on its side.
    for k in 0..10 {
        let a = k as f32 / 10.0 * TAU;
        let (x, y) = ((5.0 + a.cos() * 2.4).round() as i32, (5.5 + a.sin() * 2.4).round() as i32);
        s.set(x, y, 7, G);
        if k % 2 == 0 {
            s.set(x, y, 8, RUBY);
        }
    }
    // A plate leaning on the heap, and a bar on its foot.
    for y in 1..6 {
        for x in 13..16 {
            if (x == 13 || x == 15) && (y == 1 || y == 5) {
                continue;
            }
            s.set(x, y, 12, GD);
        }
    }
    s.cuboid(2, 0, 2, 5, 0, 3, G);
    // A string of pearls over the top.
    for k in 0..9 {
        s.set(6 + k, 5 + ((k as f32 * 0.8).sin() * 1.2).round() as i32, 9 + k / 3, PEARL);
    }
    let mut m = gilt(merge_solid(&s.finish_solid(0.0), 0.05));
    reshade(&mut m, COIN, SHADE_COINS);
    m
}

// ------------------------------------------------------------------ paintings

/// A painting's canvas, in pixels.
pub const CANVAS: (i32, i32) = (24, 16);
pub const CANVAS_PX: f32 = 0.032;

fn mixc(a: u32, b: u32, t: f32) -> u32 {
    mix_hex(a, b, t)
}

/// One painting, drawn pixel by pixel: `paint(x, y)` is the colour at (x, y), y = 0 at the top.
fn canvas(paint: impl Fn(i32, i32) -> u32) -> Merged {
    let (w, h) = CANVAS;
    let mut s = Sculpt::new(w, h, 1);
    for y in 0..h {
        for x in 0..w {
            s.set(x, h - 1 - y, 0, paint(x, y));
        }
    }
    merge(&s.finish(0.0), CANVAS_PX)
}

fn disc(x: i32, y: i32, cx: f32, cy: f32, r: f32) -> bool {
    (x as f32 - cx).powi(2) + (y as f32 - cy).powi(2) <= r * r
}

/// Every painting design.
pub fn paintings() -> Vec<Merged> {
    vec![
        // Mammon enthroned: horned and gold-robed on a gold throne, against dark red.
        canvas(|x, y| {
            let bg = mixc(0x3a0a0e, 0x6a1418, y as f32 / 16.0);
            if (8..=15).contains(&x) && (4..=15).contains(&y) && !(10..=13).contains(&x) {
                return 0xc89a30; // the throne's back and arms
            }
            if disc(x, y, 11.5, 6.0, 2.0) {
                return 0xe8b84a;
            }
            if (y == 3 && (x == 9 || x == 14)) || (y == 2 && (x == 8 || x == 15)) {
                return 0xd8c8a8;
            }
            if (9..=14).contains(&x) && (8..=14).contains(&y) {
                return if (x + y) % 3 == 0 { 0xfff0a0 } else { 0xd8a830 };
            }
            if y == 15 {
                return 0x2a1a10;
            }
            bg
        }),
        // A golden city at sunset over still water.
        canvas(|x, y| {
            let sky = mixc(0xffb050, 0x6a3a8a, y as f32 / 9.0);
            let towers = [2, 5, 9, 12, 16, 20];
            let tall = towers.iter().position(|&t| (x - t).abs() <= 1).map(|i| 4 + (i as i32 * 3) % 4);
            if y < 10 {
                if disc(x, y, 18.0, 6.0, 2.2) {
                    return 0xfff0b0;
                }
                if let Some(t) = tall {
                    if y >= t {
                        return if (x + y) % 4 == 0 { 0xffe080 } else { 0x2a1a30 };
                    }
                }
                if y >= 8 {
                    return 0x3a2440;
                }
                return sky;
            }
            let ref_y = 19 - y;
            if (x - 18).abs() <= 1 && (y % 2 == 0) {
                return 0xffd080;
            }
            mixc(sky, 0x1a1a3a, 0.5 + (ref_y as f32 * 0.05))
        }),
        // A still life: a gold goblet, grapes and coins on a dark table.
        canvas(|x, y| {
            if y >= 12 {
                return if y == 12 { 0x5a3a22 } else { 0x3a2416 };
            }
            if (5..=7).contains(&x) && (4..=11).contains(&y) && (y <= 6 || x == 6 || y == 11) {
                return if x == 5 { 0xfff0a0 } else { 0xd8a830 };
            }
            for (gx, gy) in [(12.0, 9.0), (14.0, 9.0), (13.0, 7.5), (13.0, 10.5), (15.0, 10.5), (11.0, 10.5)] {
                if disc(x, y, gx, gy, 1.1) {
                    return if (x + y) % 3 == 0 { 0x9a5ab8 } else { 0x5a2a7a };
                }
            }
            if y == 11 && (18..=21).contains(&x) {
                return 0xf2c53d;
            }
            mixc(0x1a2a1a, 0x2a3a22, x as f32 / 24.0)
        }),
        // A ship under full sail on a dark sea.
        canvas(|x, y| {
            if y >= 11 {
                return if (x + y * 2) % 5 == 0 { 0x6a9ad8 } else { 0x1a3a6a };
            }
            if (8..=16).contains(&x) && (9..=10).contains(&y) && (y == 10 || (9..=15).contains(&x)) {
                return 0x5a3a22;
            }
            if x == 12 && (1..=9).contains(&y) {
                return 0x3a2416;
            }
            if (9..=11).contains(&x) && (2..=8).contains(&y) || (13..=15).contains(&x) && (3..=8).contains(&y) {
                return 0xf4f0e0;
            }
            if disc(x, y, 4.0, 3.0, 2.0) || disc(x, y, 6.0, 2.5, 1.6) {
                return 0xe8eef8;
            }
            mixc(0x8ab8e8, 0xd8e8f8, y as f32 / 10.0)
        }),
        // A dragon curled on its hoard in a dark cave.
        canvas(|x, y| {
            if y >= 12 || (y >= 10 && (x - 12).abs() < 15 - (y - 10) * 2 && (x - 12).abs() < 8) {
                return if (x * 3 + y) % 4 == 0 { 0xfff0a0 } else { 0xe8b84a };
            }
            if disc(x, y, 10.0, 8.0, 3.6) || disc(x, y, 16.0, 5.0, 2.2) || ((12..=15).contains(&x) && (5..=8).contains(&y)) {
                return if (x + y) % 3 == 0 { 0x3a8a3a } else { 0x2a6a2a };
            }
            if x == 17 && y == 4 {
                return 0xff3a2a;
            }
            if (5..=7).contains(&x) && (3..=6).contains(&y) && x - 5 <= y - 3 {
                return 0x1a4a1a; // a wing
            }
            mixc(0x0a0808, 0x2a1a14, (y as f32 / 16.0).min(1.0))
        }),
        // A noble's portrait: a velvet hat, a gold chain, a red robe.
        canvas(|x, y| {
            if disc(x, y, 12.0, 6.0, 3.0) {
                return if y >= 8 && (11..=13).contains(&x) { 0xc88a6a } else { 0xe8b090 };
            }
            if (8..=16).contains(&x) && (1..=3).contains(&y) {
                return 0x1a1418;
            }
            if y >= 9 {
                let robe = (x - 12).abs() <= 2 + (y - 9);
                if robe {
                    return if y == 10 && (9..=15).contains(&x) { 0xf2c53d } else { 0x8a1a22 };
                }
            }
            if (x == 11 || x == 13) && y == 6 {
                return 0x2a1a10;
            }
            mixc(0x1a1a24, 0x3a3044, (x as f32 / 24.0 + y as f32 / 32.0).min(1.0))
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decorations_merge_into_far_fewer_boxes_than_voxels() {
        let (mut boxes, mut voxels) = (0, 0);
        for c in [king(), goddess(), lion(), knight(), mammon(), urn()] {
            let m = merge(&c, STATUE_VOXEL);
            assert!(m.boxes.len() < c.voxels.len(), "{} boxes for {} voxels", m.boxes.len(), c.voxels.len());
            assert!(m.height() > 0.6 && m.height() <= STATUE.1 as f32 * STATUE_VOXEL + 1e-4);
            boxes += m.boxes.len();
            voxels += c.voxels.len();
        }
        assert!(boxes * 4 < voxels * 3, "the statues merge {voxels} voxels into {boxes} boxes");
        for p in paintings() {
            assert!(p.boxes.len() * 3 < (CANVAS.0 * CANVAS.1) as usize * 2, "a painting merges to {} boxes", p.boxes.len());
        }
    }
}
