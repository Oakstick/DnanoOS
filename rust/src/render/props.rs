//! Every item and object in the world, sculpted in voxels at three to four
//! times the resolution of the old text models but at the same world size:
//! nuggets, crosses, bombs, arrows, boomerangs, fireballs, crescents,
//! potion bottles, shrine gems and pedestals, lanterns and the forest's pines.

use super::characters::Sculpt;
use super::voxels::{lin, Collected, Mesh};

/// A sculpted model as a mesh: centered on x/z, base at y = 0, `size` per voxel.
pub fn mesh_from(c: &Collected, size: f32) -> Mesh {
    let ox = (c.width - 1) as f32 / 2.0;
    let oz = (c.depth - 1) as f32 / 2.0;
    let cubes = c
        .voxels
        .iter()
        .map(|v| (glam::Vec3::new((v.x as f32 - ox) * size, (v.y as f32 + 0.5) * size, (v.z as f32 - oz) * size), lin(v.c)))
        .collect();
    Mesh { size, cubes }
}

fn noise(x: i32, y: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841) ^ (z as u32).wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 16;
    (h & 0xffff) as f32 / 65535.0
}

// ------------------------------------------------------------------ nugget

/// A lumpy gold nugget: bright on top, deep gold beneath, with glints.
pub fn nugget() -> Mesh {
    const GOLD: u32 = 0xf2c53d;
    let mut s = Sculpt::new(11, 8, 9);
    s.ellipsoid((5.0, 2.4, 4.0), (3.8, 2.5, 3.3), 0, GOLD);
    s.ellipsoid((2.8, 3.2, 3.0), (2.2, 2.0, 2.3), 0, GOLD);
    s.ellipsoid((7.6, 2.8, 5.4), (2.5, 2.2, 2.2), 0, GOLD);
    s.ellipsoid((5.2, 4.8, 4.2), (2.2, 2.0, 1.9), 0, GOLD);
    s.paint(|x, y, z, _| {
        let n = noise(x, y, z);
        Some(if n < 0.1 {
            0xfff6c8
        } else if y >= 5 {
            0xffe27a
        } else if y <= 1 {
            0xb07c1a
        } else if n > 0.85 {
            0xd49a24
        } else {
            GOLD
        })
    });
    mesh_from(&s.finish(0.06), 0.04)
}

// ------------------------------------------------------------------ remedy

/// The green cross: rounded arms, a pale bevel and a white plus on its face.
pub fn medkit() -> Mesh {
    const GREEN: u32 = 0x2fcf4a;
    let mut s = Sculpt::new(15, 15, 5);
    s.cuboid(5, 0, 0, 9, 14, 4, GREEN);
    s.cuboid(0, 5, 0, 14, 9, 4, GREEN);
    // Round off the arm ends.
    for (x, y) in [(5, 0), (9, 0), (5, 14), (9, 14), (0, 5), (0, 9), (14, 5), (14, 9)] {
        for z in 0..5 {
            s.clear(x, y, z);
        }
    }
    s.paint(|x, y, z, _| {
        let plus = (z == 4) && ((x == 7 && (2..=12).contains(&y)) || (y == 7 && (2..=12).contains(&x)) || ((6..=8).contains(&x) && (6..=8).contains(&y)));
        let edge = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| {
            let (nx, ny) = (x + dx, y + dy);
            let inside = ((5..=9).contains(&nx) && (0..=14).contains(&ny)) || ((0..=14).contains(&nx) && (5..=9).contains(&ny));
            !inside
        });
        Some(if plus {
            0xf4fff6
        } else if edge && (z == 4 || z == 0) {
            0x9dffab
        } else if z == 0 {
            0x1f9a33
        } else {
            GREEN
        })
    });
    mesh_from(&s.finish(0.03), 0.034)
}

// -------------------------------------------------------------------- bomb

/// A round iron bomb: riveted band, cap, and a rope fuse with a glowing tip.
pub fn bomb() -> Mesh {
    const IRON: u32 = 0x2c3038;
    let mut s = Sculpt::new(13, 15, 13);
    s.ellipsoid((6.0, 5.6, 6.0), (5.6, 5.6, 5.6), 0, IRON);
    s.paint(|x, y, z, _| {
        let (dx, dy, dz) = (x as f32 - 6.0, y as f32 - 5.6, z as f32 - 6.0);
        let r = (dx * dx + dy * dy + dz * dz).sqrt().max(0.01);
        // Light from the upper left front.
        let lit = (-dx * 0.4 + dy * 0.7 + dz * 0.5) / r;
        Some(if (y == 5 || y == 6) && r > 4.5 {
            if (x + z) % 3 == 0 { 0xa8b0bc } else { 0x4a505c }
        } else if lit > 0.85 {
            0xb8c0cc
        } else if lit > 0.55 {
            0x5a6270
        } else if lit > 0.1 {
            0x3a404a
        } else {
            IRON
        })
    });
    s.cuboid(5, 11, 5, 7, 11, 7, 0x6a707c);
    s.cuboid(5, 10, 5, 7, 10, 7, 0x4a505c);
    // The fuse: a curling rope ending in a hot tip.
    for (x, y, z) in [(6, 12, 6), (6, 13, 6), (7, 13, 6), (7, 14, 6)] {
        s.set(x, y, z, 0xc8a060);
    }
    s.set(8, 14, 6, 0xff7a20);
    mesh_from(&s.finish(0.04), 0.036)
}

// ------------------------------------------------------------------ arrow

/// An arrow lying along +z: fletching, a painted wooden shaft, a steel broadhead.
pub fn arrow() -> Mesh {
    let mut s = Sculpt::new(5, 3, 18);
    for z in 1..=13 {
        s.set(2, 1, z, if z % 4 == 0 { 0xa87444 } else { 0x8a5a2e });
    }
    s.set(2, 1, 0, 0x2a1a10); // nock
    for z in 1..=4 {
        let c = if z % 2 == 0 { 0xf4f4f8 } else { 0xc8262e };
        s.set(2, 0, z, c);
        s.set(2, 2, z, c);
        s.set(0, 1, z, c);
        s.set(1, 1, z, c);
        s.set(3, 1, z, c);
        s.set(4, 1, z, c);
    }
    // Broadhead.
    for (z, half) in [(14, 1), (15, 1), (16, 1), (17, 0)] {
        for x in (2 - half)..=(2 + half) {
            s.set(x, 1, z, if x == 2 { 0xf4f8fc } else { 0xc8d0da });
        }
    }
    s.set(2, 0, 15, 0xa8b0bc);
    s.set(2, 2, 15, 0xa8b0bc);
    mesh_from(&s.finish(0.03), 0.032)
}

// --------------------------------------------------------------- boomerang

/// A carved boomerang lying flat: a curved V of banded wood with pale tips.
pub fn boomerang() -> Mesh {
    let mut s = Sculpt::new(15, 2, 15);
    let seg = |px: f32, pz: f32, a: (f32, f32), b: (f32, f32)| {
        let (vx, vz) = (b.0 - a.0, b.1 - a.1);
        let t = (((px - a.0) * vx + (pz - a.1) * vz) / (vx * vx + vz * vz)).clamp(0.0, 1.0);
        let (cx, cz) = (a.0 + vx * t, a.1 + vz * t);
        (((px - cx).powi(2) + (pz - cz).powi(2)).sqrt(), t)
    };
    for z in 0..15 {
        for x in 0..15 {
            let (px, pz) = (x as f32, z as f32);
            let (da, ta) = seg(px, pz, (1.5, 13.0), (7.0, 2.5));
            let (db, tb) = seg(px, pz, (7.0, 2.5), (12.5, 13.0));
            let (d, t) = if da < db { (da, ta) } else { (db, 1.0 - tb) };
            // Arms taper toward the tips.
            if d > 1.3 + 0.6 * t {
                continue;
            }
            let c = if t < 0.14 {
                0xe8d8b0
            } else if ((t * 12.0) as i32) % 3 == 0 {
                0x6b3f1d
            } else {
                0xb07a44
            };
            s.set(x, 0, z, c);
            if d < 1.0 {
                s.set(x, 1, z, if c == 0x6b3f1d { c } else { 0xc98f4a });
            }
        }
    }
    mesh_from(&s.finish(0.04), 0.026)
}

// ---------------------------------------------------------------- fireball

/// A ball of fire: white-hot core, yellow, orange and a ragged red rim.
pub fn fireball() -> Mesh {
    let mut s = Sculpt::new(11, 11, 11);
    s.ellipsoid((5.0, 5.0, 5.0), (5.2, 5.2, 5.2), 0, 0xff8a2a);
    s.paint(|x, y, z, _| {
        let r = ((x as f32 - 5.0).powi(2) + (y as f32 - 5.0).powi(2) + (z as f32 - 5.0).powi(2)).sqrt();
        let n = noise(x, y, z);
        Some(if r < 2.2 {
            0xfff6c0
        } else if r < 3.4 {
            0xffd23a
        } else if r < 4.4 || n < 0.5 {
            0xff8a2a
        } else {
            0xe0401a
        })
    });
    // Ragged edges: bite out some of the rim.
    for y in 0..11 {
        for z in 0..11 {
            for x in 0..11 {
                let r = ((x as f32 - 5.0).powi(2) + (y as f32 - 5.0).powi(2) + (z as f32 - 5.0).powi(2)).sqrt();
                if r > 4.3 && noise(x + 7, y, z) < 0.35 {
                    s.clear(x, y, z);
                }
            }
        }
    }
    mesh_from(&s.finish(0.05), 0.03)
}

// ---------------------------------------------------------------- crescent

/// A crescent blade of cold light lying flat, sharp edge forward.
pub fn crescent() -> Mesh {
    let mut s = Sculpt::new(19, 2, 9);
    for z in 0..9 {
        for x in 0..19 {
            let (dx, dz) = (x as f32 - 9.0, z as f32 + 3.0);
            let r = (dx * dx + dz * dz).sqrt();
            if !(8.0..=11.5).contains(&r) {
                continue;
            }
            let c = if r > 10.6 { 0xf4ffff } else if r < 8.8 { 0x5ab8e0 } else { 0x8fe7ff };
            s.set(x, 0, z, c);
            if r > 8.8 && r < 10.8 {
                s.set(x, 1, z, 0xbff4ff);
            }
        }
    }
    mesh_from(&s.finish(0.03), 0.028)
}

// ------------------------------------------------------------------ bottle

/// Placeholder liquid colors, recolored per potion: body, shadow, surface.
pub const LIQUID: u32 = 0x010203;
pub const LIQUID_DARK: u32 = 0x010204;
pub const LIQUID_LIGHT: u32 = 0x010205;

/// A round glass flask, half full, with a neck, a lip and a cork.
pub fn bottle(liquid: u32) -> Mesh {
    const GLASS: u32 = 0xd0e8f2;
    let mut s = Sculpt::new(11, 17, 11);
    s.ellipsoid((5.0, 4.6, 5.0), (4.6, 4.5, 4.6), 0, GLASS);
    s.paint(|x, y, z, _| {
        Some(if (2..=3).contains(&x) && (3..=6).contains(&y) && z >= 7 {
            0xffffff // a glint on the glass
        } else if y <= 1 {
            LIQUID_DARK
        } else if y == 7 {
            LIQUID_LIGHT
        } else if y < 7 {
            LIQUID
        } else {
            GLASS
        })
    });
    s.cuboid(4, 9, 4, 6, 12, 6, GLASS);
    s.cuboid(3, 12, 3, 7, 12, 7, 0xe8f6fc);
    s.cuboid(4, 13, 4, 6, 15, 6, 0xa87a4a);
    s.cuboid(4, 15, 4, 6, 15, 6, 0xc89a64);
    s.cuboid(4, 10, 4, 6, 10, 6, 0xc8a060); // a twine tie
    let mut m = mesh_from(&s.finish(0.0), 0.03);
    let (body, dark, light) = (lin(liquid), lin(liquid) * 0.55, lin(liquid).lerp(glam::Vec3::ONE, 0.35));
    let glass = lin(0xd0e8f2).lerp(lin(liquid), 0.3);
    for (_, c) in m.cubes.iter_mut() {
        if (*c - lin(0xd0e8f2)).length() < 0.08 {
            *c = glass;
        } else if (*c - lin(LIQUID)).length() < 1e-5 {
            *c = body;
        } else if (*c - lin(LIQUID_DARK)).length() < 1e-5 {
            *c = dark;
        } else if (*c - lin(LIQUID_LIGHT)).length() < 1e-5 {
            *c = light;
        }
    }
    m
}

// ------------------------------------------------------------------ shrines

/// A cut gem for the shrines: pale facets, lit differently so it reads cut; tinted by the offer.
pub fn gem() -> Mesh {
    let mut s = Sculpt::new(13, 21, 13);
    for y in 0..21 {
        let r = if y <= 12 { y as f32 / 2.0 } else { (20 - y) as f32 * 6.0 / 8.0 };
        for z in 0..13 {
            for x in 0..13 {
                let (dx, dz) = (x as f32 - 6.0, z as f32 - 6.0);
                if dx.abs() + dz.abs() > r + 0.3 {
                    continue;
                }
                let up = y > 12;
                let facet = match (dx >= 0.0, dz >= 0.0) {
                    (true, true) => 0xffffff,
                    (true, false) => 0xc8c8d4,
                    (false, true) => 0x9a9aac,
                    (false, false) => 0x74748a,
                };
                let c = if y == 12 {
                    0xffffff
                } else if up {
                    facet
                } else {
                    let v = facet & 0xff;
                    let k = (v as f32 * 0.62) as u32;
                    (k << 16) | (k << 8) | k
                };
                s.set(x, y, z, c);
            }
        }
    }
    mesh_from(&s.finish(0.02), 0.034)
}

/// A carved stone pedestal and, separately, its ring of runes (which glow).
pub fn pedestal(rune: u32) -> (Mesh, Mesh) {
    let all = pedestal_mesh(rune);
    let key = lin(rune);
    let (runes, stone): (Vec<_>, Vec<_>) = all.cubes.into_iter().partition(|(_, c)| (*c - key).length() < 1e-3);
    (Mesh { size: all.size, cubes: stone }, Mesh { size: all.size, cubes: runes })
}

fn pedestal_mesh(rune: u32) -> Mesh {
    const STONE: u32 = 0x66686c;
    const STONE_D: u32 = 0x4a4c50;
    const STONE_L: u32 = 0x86888c;
    let mut s = Sculpt::new(15, 9, 15);
    let oct = |x: i32, z: i32, r: i32| (x - 7).abs() <= r && (z - 7).abs() <= r && (x - 7).abs() + (z - 7).abs() <= r + r / 2;
    for y in 0..9 {
        let r = match y {
            0..=1 => 7,
            2 => 6,
            3..=6 => 4,
            7 => 6,
            _ => 5,
        };
        for z in 0..15 {
            for x in 0..15 {
                if oct(x, z, r) {
                    let c = match y {
                        0 => STONE_D,
                        1 | 7 => STONE_L,
                        _ => STONE,
                    };
                    s.set(x, y, z, c);
                }
            }
        }
    }
    // Runes: bright marks around the column.
    s.paint(|x, y, z, c| {
        if (4..=5).contains(&y) && c == STONE && ((x - 7).abs() == 4 || (z - 7).abs() == 4) && (x + z + y) % 3 != 0 {
            Some(rune)
        } else if noise(x, y, z) < 0.12 && c == STONE {
            Some(0x5f8f3a) // a little moss
        } else {
            None
        }
    });
    // No jitter, so the rune color stays exact for splitting.
    let mut c = s.finish(0.0);
    for v in c.voxels.iter_mut() {
        if v.c != rune {
            let k = 0.92 + noise(v.x, v.y, v.z) * 0.16;
            let ch = |sh: u32| ((((v.c >> sh) & 255) as f32 * k).min(255.0) as u32) << sh;
            v.c = ch(16) | ch(8) | ch(0);
        }
    }
    mesh_from(&c, 0.045)
}

// ------------------------------------------------------------------ lantern

/// A lantern on a short post: the frame (no glow) and its glass (glows).
pub fn lantern() -> (Mesh, Mesh) {
    const IRON: u32 = 0x2e2a26;
    let mut frame = Sculpt::new(9, 20, 9);
    frame.cuboid(4, 0, 4, 5, 11, 5, 0x6b3f1d);
    frame.cuboid(2, 12, 2, 7, 12, 7, IRON);
    for (x, z) in [(2, 2), (2, 7), (7, 2), (7, 7)] {
        frame.cuboid(x, 13, z, x, 16, z, IRON);
    }
    frame.cuboid(2, 17, 2, 7, 17, 7, IRON);
    frame.cuboid(3, 18, 3, 6, 18, 6, IRON);
    frame.cuboid(4, 19, 4, 5, 19, 5, 0x5a5048);
    let mut glass = Sculpt::new(9, 20, 9);
    glass.cuboid(3, 13, 3, 6, 16, 6, 0xd6ff5a);
    glass.cuboid(4, 14, 4, 5, 15, 5, 0xffffc0);
    (mesh_from(&frame.finish(0.04), 0.045), mesh_from(&glass.finish(0.0), 0.045))
}

// -------------------------------------------------------------------- tree

/// A Norse pine: a short trunk with roots and three tiers of needles, lit on top.
pub fn pine() -> Mesh {
    const DARK: u32 = 0x1f4a1c;
    const MID: u32 = 0x2f6b2a;
    const LIGHT: u32 = 0x4c8f3c;
    let mut s = Sculpt::new(7, 13, 7);
    s.cuboid(3, 0, 3, 3, 3, 3, 0x4a2f16);
    for (x, z) in [(2, 3), (4, 3), (3, 2), (3, 4)] {
        s.set(x, 0, z, 0x3a2410);
    }
    for (y0, y1, r0, r1) in [(2, 6, 3.4f32, 1.4f32), (5, 9, 2.8, 0.9), (8, 12, 2.0, 0.2)] {
        for y in y0..=y1 {
            let t = (y - y0) as f32 / (y1 - y0) as f32;
            let r = r0 + (r1 - r0) * t;
            for z in 0..7 {
                for x in 0..7 {
                    let (dx, dz) = (x as f32 - 3.0, z as f32 - 3.0);
                    if dx * dx + dz * dz <= r * r + 0.3 {
                        s.set(x, y, z, MID);
                    }
                }
            }
        }
    }
    s.paint(|x, y, z, c| {
        if c != MID {
            return None;
        }
        let n = noise(x, y, z);
        // The tier edges catch the light; the undersides sit in shade.
        let below = y == 2 || y == 5 || y == 8;
        Some(if below {
            DARK
        } else if n < 0.3 || y >= 11 {
            LIGHT
        } else if n > 0.8 {
            DARK
        } else {
            MID
        })
    });
    s.set(3, 12, 3, LIGHT);
    mesh_from(&s.finish(0.06), 0.078)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_prop_is_sculpted_at_its_old_size() {
        let size = |m: &Mesh| {
            let (mut lo, mut hi) = (glam::Vec3::splat(f32::MAX), glam::Vec3::splat(f32::MIN));
            for (p, _) in &m.cubes {
                lo = lo.min(*p);
                hi = hi.max(*p);
            }
            hi - lo + glam::Vec3::splat(m.size)
        };
        for (name, m, min_cubes) in [
            ("nugget", nugget(), 60),
            ("medkit", medkit(), 100),
            ("bomb", bomb(), 150),
            ("arrow", arrow(), 20),
            ("boomerang", boomerang(), 40),
            ("fireball", fireball(), 100),
            ("crescent", crescent(), 40),
            ("bottle", bottle(0xff0000), 100),
            ("gem", gem(), 150),
            ("pedestal", pedestal(0x00ffff).0, 150),
            ("pine", pine(), 60),
        ] {
            assert!(m.cubes.len() >= min_cubes, "{name}: only {} cubes", m.cubes.len());
            let s = size(&m);
            assert!(s.max_element() < 1.1, "{name} grew too big: {s}");
        }
        // The pine keeps the old tree's footprint, which the camera fade relies on.
        let p = size(&pine());
        assert!((p.x - 0.55).abs() < 0.1 && (p.y - 1.0).abs() < 0.15, "pine is {p}");
        // The bottle's liquid takes its potion's color.
        assert!(bottle(0xff0000).cubes.iter().any(|(_, c)| (*c - lin(0xff0000)).length() < 1e-4));
        assert!(!pedestal(0x00ffff).1.cubes.is_empty(), "the runes split out");
    }
}
