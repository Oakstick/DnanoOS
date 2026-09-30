//! Bushes and hedges, sculpted in voxels: a body of leaves built up from
//! overlapping clumps, a ragged fringe of single leaves standing out from it,
//! and colour laid on by the light (sunlit crowns, shaded flanks, a dark
//! hollow at the foot) with a speckle of bright leaves and, on some, berries
//! or blossoms. The meadow's hedge ring and its bushes, and the forest's
//! darker bushes, are all made here.

use super::characters::Sculpt;
use super::props::mesh_from;
use super::voxels::Mesh;

/// A bush's colours, from the hollow at its foot to the sunlit leaf tips.
#[derive(Clone, Copy)]
pub struct Foliage {
    pub deep: u32,
    pub dark: u32,
    pub mid: u32,
    pub light: u32,
    pub sun: u32,
}

/// The meadow's bright bushes.
pub const MEADOW: Foliage = Foliage { deep: 0x2a6a22, dark: 0x3f962f, mid: 0x52b03c, light: 0x66c64a, sun: 0x94e466 };
/// The meadow's hedge ring: deeper, clipped box green.
pub const HEDGE: Foliage = Foliage { deep: 0x1e4a1a, dark: 0x2e6e26, mid: 0x3a852f, light: 0x4a9a38, sun: 0x74c252 };
/// The forest's bushes, in the gloom of the dungeons.
pub const FOREST: Foliage = Foliage { deep: 0x1a3a16, dark: 0x255a20, mid: 0x2f6b2a, light: 0x3a7a2e, sun: 0x5a9a3a };

/// Berries and blossoms a bush may wear.
const FRUIT: [u32; 4] = [0xd8262e, 0xf8f8f0, 0xffb0d8, 0xffe066];

fn noise(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343)
        ^ (y as u32).wrapping_mul(0xd816_3841)
        ^ (z as u32).wrapping_mul(0xcb1a_b31f)
        ^ seed.wrapping_mul(0x9e37_79b9);
    h ^= h >> 13;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 16;
    (h & 0xffff) as f32 / 65535.0
}

const SIDES: [(i32, i32, i32); 6] = [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)];

/**
 * A bush `w` voxels across and up to `h` tall. `boxy` gives a clipped hedge
 * block with square shoulders (so neighbours join into one hedge); otherwise
 * it is a round mound. `clumps` lumps of leaves are heaped over the body,
 * and `fruit` dots it with berries or blossoms.
 */
fn sculpt(seed: u32, w: i32, h: i32, boxy: bool, clumps: usize, pal: Foliage, fruit: Option<u32>) -> Sculpt {
    let mut s = Sculpt::new(w, h, w);
    let c = (w - 1) as f32 / 2.0;
    let r = w as f32 / 2.0;
    let body_top = if boxy { h as f32 * 0.78 } else { h as f32 * 0.72 };
    // The body.
    for y in 0..h {
        for z in 0..w {
            for x in 0..w {
                let (dx, dz) = ((x as f32 - c) / r, (z as f32 - c) / r);
                let yf = y as f32;
                let inside = if boxy {
                    // A squarish footprint (a superellipse), shoulders rounding off near the top.
                    let shoulder = ((yf - (body_top - 2.5)) / 2.5).clamp(0.0, 1.0);
                    let k = 4.0 - 2.0 * shoulder;
                    let reach = 1.0 - 0.25 * shoulder * shoulder;
                    yf <= body_top && dx.abs().powf(k) + dz.abs().powf(k) <= reach.powf(k)
                } else {
                    // A dome: widest a little above the ground, pinched in at the foot.
                    let t = yf / body_top;
                    let width = if t < 0.2 { 0.72 + t * 1.2 } else { (1.0 - ((t - 0.2) / 0.8).powi(2)).max(0.0).sqrt() * 0.96 };
                    t <= 1.0 && dx * dx + dz * dz <= width * width
                };
                if inside {
                    s.set(x, y, z, pal.mid);
                }
            }
        }
    }
    // Clumps heaped over the crown and shoulders, so no two bushes have the same outline.
    for k in 0..clumps {
        let a = noise(k as i32, 1, 2, seed) * std::f32::consts::TAU;
        let out = 0.25 + noise(k as i32, 3, 4, seed) * 0.5;
        let cx = c + a.cos() * r * out;
        let cz = c + a.sin() * r * out;
        let cy = body_top - 1.5 + noise(k as i32, 5, 6, seed) * 2.0 - out * 2.0;
        let rad = r * (0.34 + noise(k as i32, 7, 8, seed) * 0.16);
        s.ellipsoid((cx, cy, cz), (rad, rad * 0.8, rad), 0, pal.mid);
    }
    // A fringe of single leaves standing proud of the surface.
    let mut fringe = Vec::new();
    for y in 1..h {
        for z in 0..w {
            for x in 0..w {
                if s.get(x, y, z).is_none() {
                    continue;
                }
                for (i, &(dx, dy, dz)) in SIDES.iter().enumerate() {
                    if dy < 0 {
                        continue;
                    }
                    let (nx, ny, nz) = (x + dx, y + dy, z + dz);
                    if nx < 0 || nz < 0 || nx >= w || nz >= w || ny >= h || s.get(nx, ny, nz).is_some() {
                        continue;
                    }
                    if noise(x * 7 + i as i32, y * 3, z * 5, seed) < 0.09 {
                        fringe.push((nx, ny, nz));
                    }
                }
            }
        }
    }
    for (x, y, z) in fringe {
        s.set(x, y, z, pal.light);
    }
    // Light: sunlit where open to the sky, shaded down the flanks, dark at the foot; bright speckles; fruit.
    let snapshot: Vec<(i32, i32, i32, bool, bool)> = {
        let mut v = Vec::new();
        for y in 0..h {
            for z in 0..w {
                for x in 0..w {
                    if s.get(x, y, z).is_some() {
                        let open_above = (y + 1..h).all(|yy| s.get(x, yy, z).is_none());
                        let surface = SIDES.iter().any(|&(dx, dy, dz)| {
                            let (nx, ny, nz) = (x + dx, y + dy, z + dz);
                            ny >= 0 && (nx < 0 || nz < 0 || nx >= w || nz >= w || ny >= h || s.get(nx, ny, nz).is_none())
                        });
                        v.push((x, y, z, open_above, surface));
                    }
                }
            }
        }
        v
    };
    for (x, y, z, open_above, surface) in snapshot {
        let n = noise(x, y, z, seed);
        let height = y as f32 / h as f32;
        let color = if y == 0 || (height < 0.18 && n < 0.7) {
            pal.deep
        } else if open_above {
            if n < 0.22 {
                pal.sun
            } else if n < 0.75 {
                pal.light
            } else {
                pal.mid
            }
        } else if n < 0.08 {
            pal.light
        } else if height < 0.45 {
            if n < 0.55 {
                pal.dark
            } else {
                pal.deep
            }
        } else if n < 0.6 {
            pal.mid
        } else {
            pal.dark
        };
        let color = match fruit {
            Some(f) if surface && height > 0.3 && noise(x * 3, y * 5, z * 7, seed ^ 0x55) < 0.035 => f,
            _ => color,
        };
        s.recolor(x, y, z, color);
    }
    s
}

/// A hedge block a little over a tile across (neighbours overlap into one hedge), in one of four shapes.
pub fn hedge_block(variant: u32) -> Mesh {
    let h = 10 + (variant % 3) as i32;
    let fruit = (variant == 3).then_some(FRUIT[1]);
    mesh_from(&sculpt(0x4ed6e + variant * 7919, 12, h, true, 7, HEDGE, fruit).finish_grounded(0.05), HEDGE_VOXEL)
}

/// A voxel of the hedge ring, in world units.
pub const HEDGE_VOXEL: f32 = 0.1;

/// A round leafy bush about 0.7 across (scaled where it is placed), in one of four shapes;
/// two of them wear berries or blossoms.
pub fn bush(variant: u32, pal: Foliage) -> Mesh {
    let fruit = match variant {
        1 => Some(FRUIT[0]),
        3 => Some(FRUIT[2 + (pal.mid == FOREST.mid) as usize]),
        _ => None,
    };
    let h = 7 + (variant % 2) as i32;
    mesh_from(&sculpt(0xb057 + variant * 104_729, 10, h, false, 5, pal, fruit).finish_grounded(0.05), 0.07)
}

/// The same bush sculpted coarser, for the dungeons' walls where a thousand of them grow
/// and they are mostly seen from afar: the same size, about half the voxels.
pub fn small_bush(variant: u32, pal: Foliage) -> Mesh {
    let fruit = (variant == 1).then_some(FRUIT[0]);
    let h = 5 + (variant % 2) as i32;
    mesh_from(&sculpt(0xb057 + variant * 104_729, 7, h, false, 4, pal, fruit).finish_grounded(0.05), 0.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bushes_are_leafy_and_not_too_heavy() {
        for v in 0..4 {
            let hedge = hedge_block(v);
            let top = hedge.cubes.iter().map(|(p, _)| p.y).fold(0.0f32, f32::max);
            assert!((0.8..1.3).contains(&top), "a hedge about a tile high: {top}");
            assert!(hedge.cubes.len() < 1400, "hedge {v}: {} voxels", hedge.cubes.len());
            let b = bush(v, MEADOW);
            assert!(b.cubes.len() > 120 && b.cubes.len() < 700, "bush {v}: {} voxels", b.cubes.len());
            // More than a box: several shades of green.
            let mut shades: Vec<[u32; 3]> =
                b.cubes.iter().map(|(_, c)| [(c.x * 64.0) as u32, (c.y * 64.0) as u32, (c.z * 64.0) as u32]).collect();
            shades.sort();
            shades.dedup();
            assert!(shades.len() >= 5, "bush {v} has {} shades", shades.len());
        }
    }
}
