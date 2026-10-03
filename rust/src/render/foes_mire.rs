//! The model of Ganglati and Ganglöt, Hel's shrouded servants in the Grey Mire (REALMS.md §4): the
//! slow-walkers, tall and stooped in a grey burial shroud wound with bands and a cord, its hem
//! torn and black with bog to the knee and hung with weed; the head bowed under a deep hood, the
//! face only a shadow with two cold points of light in it; gaunt grey hands, the left reaching
//! out before it, the right on a long crooked staff of bog-oak from whose crook a lantern hangs,
//! burning blue. It faces +z, in the parts its rig swings (legs that shuffle under the shroud, the
//! arms in their sleeves, the head in its hood).

use glam::Vec3;

use super::carve::{ball, nz, v};
use super::characters::{mat, Sculpt};
use super::voxels::{Collected, PartDef};

/// The model's grid, and the size of one of its voxels in tiles: about a tile and a quarter tall,
/// stooped, and taller than a man with its staff.
pub const GANGLOT_SIZE: (i32, i32, i32) = (46, 100, 48);
pub const GANGLOT_VOXEL: f32 = 0.0125;
const C: (f32, f32) = (21.5, 17.0);

/// Its hips (set low: under the shroud only its shins swing), its shoulders and its neck.
pub const GANGLOT_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[], pivot: [C.0 - 4.0, 24.0, C.1] },
    PartDef { name: "rightLeg", boxes: &[], pivot: [C.0 + 4.0, 24.0, C.1] },
    PartDef { name: "leftArm", boxes: &[], pivot: [C.0 - 8.0, 63.0, C.1 + 8.0] },
    PartDef { name: "rightArm", boxes: &[], pivot: [C.0 + 8.0, 63.0, C.1 + 8.0] },
    PartDef { name: "head", boxes: &[], pivot: [C.0, 65.0, C.1 + 11.0] },
];

mod pal {
    use super::mat;
    pub const SHROUD: u32 = 0xa9a799 | mat::CLOTH;
    pub const SHROUD_L: u32 = 0xc4c2b4 | mat::CLOTH;
    pub const SHROUD_D: u32 = 0x82806f | mat::CLOTH;
    pub const BAND: u32 = 0x66645a | mat::CLOTH;
    pub const STAIN: u32 = 0x4c4436 | mat::CLOTH;
    pub const STAIN_D: u32 = 0x353024 | mat::CLOTH;
    pub const WEED: u32 = 0x56633c | mat::FUR;
    pub const CORD: u32 = 0x6a5a40 | mat::LEATHER;
    pub const SKIN: u32 = 0x8c979b | mat::SKIN;
    pub const SKIN_D: u32 = 0x6b767b | mat::SKIN;
    pub const NAIL: u32 = 0xc8c2ae | mat::IVORY;
    pub const VOID: u32 = 0x0b0b0d | mat::CLOTH;
    pub const EYE: u32 = 0xa8e0ff | mat::GLOW;
    pub const WOOD: u32 = 0x463a2e | mat::WOOD;
    pub const WOOD_L: u32 = 0x5e4f3c | mat::WOOD;
    pub const IRON: u32 = 0x3a3d42 | mat::METAL;
    pub const IRON_L: u32 = 0x585c62 | mat::METAL;
    pub const FLAME: u32 = 0x6ab4ff | mat::GLOW;
    pub const FLAME_CORE: u32 = 0xd8f0ff | mat::GLOW;
    pub const GLASS: u32 = 0x4a6a80 | mat::GLOW;
}
use pal::*;

/// The shroud's colour at height `y` (voxels) and angle `a`: bog-black to the knee, its folds lighter.
fn shroud_at(y: i32, a: f32, x: i32, z: i32) -> u32 {
    let fold = (a * 7.0 + y as f32 * 0.05).sin();
    let stain = 22.0 + nz(x, 0, z) * 8.0;
    if (y as f32) < stain * 0.55 {
        if nz(x, y, z) > 0.5 {
            STAIN_D
        } else {
            STAIN
        }
    } else if (y as f32) < stain {
        if nz(x, y, z) > 0.55 {
            STAIN
        } else {
            SHROUD_D
        }
    } else if nz(x, 0, z * 3) > 0.82 && (y as f32) < 48.0 + nz(z, 1, x) * 14.0 {
        // Bog water dried in streaks down from the knee.
        SHROUD_D
    } else if fold > 0.75 {
        SHROUD_L
    } else if fold < -0.7 {
        SHROUD_D
    } else {
        SHROUD
    }
}

/// A tapered limb from `a` to `b`, radii `ra` to `rb`, in `c`.
fn cap(s: &mut Sculpt, a: Vec3, b: Vec3, ra: f32, rb: f32, c: u32) {
    s.capsule((a.x, a.y, a.z), (b.x, b.y, b.z), ra, rb, c);
}

/// A gaunt hand at `wrist`, reaching along `fwd`: knuckles, four long fingers crooked down, a thumb.
fn hand(s: &mut Sculpt, wrist: Vec3, fwd: Vec3, out: f32, curl: f32) {
    let f = fwd.normalize();
    let lat = Vec3::X * out;
    let down = -Vec3::Y;
    ball(s, wrist + f * 1.2, v(2.0, 1.2, 1.8), SKIN);
    for i in 0..4 {
        let across = -1.2 + i as f32 * 0.8;
        let base = wrist + f * 2.4 + lat * across;
        let k1 = base + (f + down * curl * 0.3).normalize() * 2.2;
        let k2 = k1 + (f * 0.5 + down * curl).normalize() * 1.8;
        cap(s, base, k1, 0.6, 0.55, SKIN_D);
        cap(s, k1, k2, 0.55, 0.45, SKIN);
        s.set(k2.x.round() as i32, k2.y.round() as i32, k2.z.round() as i32, NAIL);
    }
    let t = wrist + f * 1.2 - lat * 2.0;
    cap(s, t, t + f * 1.6 + down * 0.6, 0.6, 0.45, SKIN_D);
}

pub fn ganglot() -> Collected {
    let (w, h, d) = GANGLOT_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = C;
    // The spine leans forward as it rises: where the body's middle is at height `y`.
    let lean = |y: f32| cz + 11.0 * ((y - 20.0).max(0.0) / 46.0).powf(1.8);

    // ---- the shins and bare grey feet, shuffling under the hem
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(1 + i as u8);
        let knee = v(cx + side * 4.0, 22.0, cz + 0.5);
        let ankle = v(cx + side * 4.4, 4.0, cz + 0.5);
        cap(&mut s, knee, ankle, 2.6, 2.0, BAND);
        // Wrapped in strips, like the shroud.
        for y in 6..22 {
            if y % 4 == 0 {
                for (x, z) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let p = v(cx + side * 4.2 + x as f32 * 2.2, y as f32, cz + 0.5 + z as f32 * 2.2);
                    s.set(p.x.round() as i32, y, p.z.round() as i32, SHROUD_D);
                }
            }
        }
        ball(&mut s, v(ankle.x, 2.0, ankle.z + 2.2), v(2.0, 1.8, 3.4), SKIN_D);
        for t in 0..3 {
            s.set((ankle.x - 1.0 + t as f32).round() as i32, 1, (ankle.z + 5.4).round() as i32, NAIL);
        }
    }

    // ---- the shroud: from its torn hem to the shoulders, filled within, bowed forward
    s.part(0);
    for y in 7..=64 {
        let yf = y as f32;
        let t = (yf - 7.0) / 57.0;
        let mid = lean(yf);
        let rx = 9.8 - 3.2 * t + 0.6 * (1.0 - t) * (1.0 - t);
        let (rf, rb) = (7.4 - 2.0 * t, 8.6 - 2.6 * t + if (40.0..62.0).contains(&yf) { 1.5 } else { 0.0 });
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = (x as f32 - cx, z as f32 - mid);
                let a = dz.atan2(dx);
                let k = 1.0 + 0.06 * (a * 9.0 + yf * 0.04).sin() * (1.0 - t * 0.6);
                let rz = if dz > 0.0 { rf } else { rb };
                if (dx / (rx * k)).powi(2) + (dz / (rz * k)).powi(2) > 1.0 {
                    continue;
                }
                // Its hem in tatters: strips of uneven length.
                let strip = ((a + std::f32::consts::PI) / std::f32::consts::TAU * 22.0).floor() as i32;
                if yf < 7.0 + nz(strip, 5, 11) * 7.0 {
                    continue;
                }
                s.set(x, y, z, shroud_at(y, a, x, z));
            }
        }
    }
    // The winding bands round the body, and a cord knotted at the waist, its ends hanging.
    for (y0, c) in [(30, BAND), (37, BAND), (44, CORD), (52, BAND), (59, BAND)] {
        let yf = y0 as f32;
        let mid = lean(yf);
        for z in 0..d {
            for x in 0..w {
                if s.get(x, y0, z).is_some() {
                    let outer = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(ox, oz)| s.get(x + ox, y0, z + oz).is_none());
                    if outer {
                        s.recolor(x, y0, z, c);
                        if c == CORD {
                            s.recolor(x, y0 + 1, z, c);
                        }
                    }
                }
            }
        }
        if c == CORD {
            let knot = v(cx + 2.0, yf, mid + 7.2);
            ball(&mut s, knot, v(1.4, 1.2, 1.0), CORD);
            cap(&mut s, knot, knot + v(0.6, -12.0, 0.8), 0.6, 0.5, CORD);
            cap(&mut s, knot, knot + v(2.4, -9.0, 0.6), 0.6, 0.5, CORD);
        }
    }
    // Weed hanging from the hem, black water dripping.
    for k in 0..14 {
        let a = k as f32 / 14.0 * std::f32::consts::TAU + 0.3;
        let r = 9.4;
        let top = 12.0 + nz(k, 2, 3) * 8.0;
        let p = v(cx + a.cos() * r, top, lean(top) + a.sin() * r * 0.85);
        cap(&mut s, p, p + v(0.0, -(4.0 + nz(k, 4, 1) * 5.0), 0.3), 0.55, 0.4, WEED);
    }
    // The shoulders under the shroud, hunched.
    for side in [-1.0f32, 1.0] {
        ball(&mut s, v(cx + side * 6.2, 61.0, lean(61.0) - 0.5), v(4.4, 3.6, 4.4), SHROUD);
    }
    // The hump of the back, under the shroud.
    ball(&mut s, v(cx, 60.0, lean(60.0) - 3.5), v(6.4, 5.0, 4.6), SHROUD_D);

    // ---- the arms in long sleeves: the left reaching out, the right on the staff
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        s.part(3 + i as u8);
        let shoulder = Vec3::from(GANGLOT_RIG[2 + i].pivot);
        let (elbow, wrist) = if side < 0.0 {
            (v(cx - 9.0, 54.0, cz + 15.5), v(cx - 6.5, 51.0, cz + 23.5))
        } else {
            (v(cx + 9.5, 52.0, cz + 12.0), v(cx + 10.0, 49.0, cz + 17.0))
        };
        cap(&mut s, shoulder, elbow, 3.4, 3.2, SHROUD);
        // The sleeve opens toward the wrist, hanging loose below it, its edge torn.
        let n = 10;
        for k in 0..=n {
            let t = k as f32 / n as f32;
            let c = elbow + (wrist - elbow) * t;
            let r = 3.0 + 1.6 * t * t;
            let sag = v(0.0, -1.4 * t * t, 0.0);
            let col = if t > 0.85 { SHROUD_D } else { SHROUD };
            ball(&mut s, c + sag, v(r, r, r), col);
        }
        // Hollow at the cuff: the hand comes out of shadow.
        ball(&mut s, wrist + (wrist - elbow).normalize() * 0.8, v(2.4, 2.4, 2.4), VOID);
        let fwd = if side < 0.0 { v(0.15, -0.15, 1.0) } else { v(0.0, -0.2, 1.0) };
        hand(&mut s, wrist + (wrist - elbow).normalize() * 1.2, fwd, side, if side < 0.0 { 0.9 } else { 1.6 });
        if side > 0.0 {
            // The staff of bog-oak, gnarled, through its fist from the ground to its crook.
            let (sx, sz) = (wrist.x + 0.5, wrist.z + 3.4);
            for y in 1..=88 {
                let wob = (y as f32 * 0.21).sin() * 0.5;
                let r = if y % 17 == 0 { 1.7 } else { 1.15 };
                for z in (sz - 2.0) as i32..=(sz + 2.0) as i32 {
                    for x in (sx - 2.0) as i32..=(sx + 2.0) as i32 {
                        let (dx, dz) = (x as f32 - sx - wob, z as f32 - sz);
                        if dx * dx + dz * dz <= r * r {
                            s.set(x, y, z, if (x + y) % 5 == 0 { WOOD_L } else { WOOD });
                        }
                    }
                }
            }
            // The crook bends forward and down; the lantern hangs from it on a chain.
            let mut p = v(sx, 88.0, sz);
            for k in 0..9 {
                let a = k as f32 / 8.0 * 2.6;
                let q = v(sx + 0.3 * k as f32 * 0.1, 88.0 + a.sin() * 5.0, sz + (1.0 - a.cos()) * 3.4);
                cap(&mut s, p, q, 1.2, 1.1, WOOD);
                p = q;
            }
            let hook = p;
            for k in 0..5 {
                s.set(hook.x.round() as i32, (hook.y - 1.0 - k as f32).round() as i32, hook.z.round() as i32, IRON_L);
            }
            let lamp = hook + v(0.0, -10.0, 0.0);
            // Its iron cage, smoked glass, the blue flame within.
            for y in -3i32..=3 {
                for z in -2i32..=2 {
                    for x in -2i32..=2 {
                        let edge = x.abs() == 2 && z.abs() == 2 || y.abs() == 3;
                        let glass = !edge && (x.abs() == 2 || z.abs() == 2);
                        let c = if edge {
                            IRON
                        } else if glass {
                            GLASS
                        } else if y.abs() <= 1 && x.abs() <= 1 && z.abs() <= 1 {
                            if y == 0 && x == 0 && z == 0 {
                                FLAME_CORE
                            } else {
                                FLAME
                            }
                        } else {
                            continue;
                        };
                        s.set(
                            (lamp.x + x as f32).round() as i32,
                            (lamp.y + y as f32).round() as i32,
                            (lamp.z + z as f32).round() as i32,
                            c,
                        );
                    }
                }
            }
            for y in 4..=5 {
                s.set(lamp.x.round() as i32, (lamp.y + y as f32).round() as i32, lamp.z.round() as i32, IRON_L);
            }
        }
    }

    // ---- the head, bowed in its hood: a shadow for a face, two cold points of light in it
    s.part(5);
    let neck = Vec3::from(GANGLOT_RIG[4].pivot);
    let head = v(cx, 68.0, neck.z + 4.0);
    // The hood: a deep cowl, open in front on a shadow where the face should be.
    for y in (head.y - 7.0) as i32..=(head.y + 9.0) as i32 {
        for z in (head.z - 9.0) as i32..=(head.z + 9.0) as i32 {
            for x in (head.x - 9.0) as i32..=(head.x + 9.0) as i32 {
                let p = v(x as f32, y as f32, z as f32) - head;
                let r = (p / v(6.8, 7.6, 7.4)).length();
                if r > 1.0 {
                    continue;
                }
                // The opening: an oval in front, under the brow, cut deep into the cowl.
                let face = (p.x / 4.2).powi(2) + ((p.y + 1.0) / 4.6).powi(2) < 1.0 && p.z > 1.0;
                if face && r > 0.62 {
                    continue;
                }
                let c = if face || r < 0.62 {
                    VOID
                } else if (p.x * 1.3 + p.y).sin() > 0.8 {
                    SHROUD_L
                } else if p.y < -4.0 {
                    SHROUD_D
                } else {
                    SHROUD
                };
                s.set(x, y, z, c);
            }
        }
    }
    // The rim of the opening, a fold of linen round the shadow.
    for k in 0..24 {
        let a = k as f32 / 24.0 * std::f32::consts::TAU;
        let p = head + v(a.cos() * 4.4, a.sin() * 4.8 - 1.0, 5.0 + (a.sin() * 0.5 + 0.5) * 1.2);
        s.set(p.x.round() as i32, p.y.round() as i32, p.z.round() as i32, SHROUD_D);
    }
    // The peak of the cowl, sagging forward.
    ball(&mut s, head + v(0.0, 6.0, 2.0), v(4.4, 2.6, 4.6), SHROUD);
    ball(&mut s, head + v(0.0, 4.4, 5.6), v(4.2, 1.6, 2.2), SHROUD_D);
    // The eyes: two cold lights deep in the shadow.
    for side in [-1.0f32, 1.0] {
        let e = head + v(side * 1.6, 0.0, 3.6);
        s.set(e.x.round() as i32, e.y.round() as i32, e.z.round() as i32, EYE);
        s.set(e.x.round() as i32, e.y.round() as i32 - 1, e.z.round() as i32, EYE);
    }
    // The hood's hem falls over the shoulders.
    for k in 0..16 {
        let a = k as f32 / 16.0 * std::f32::consts::TAU;
        let p = head + v(a.cos() * 6.6, -6.4, a.sin() * 6.0 - 1.0);
        if a.sin() > 0.6 {
            continue; // (not across the face)
        }
        cap(&mut s, p, p + v(a.cos() * 1.2, -3.0 - nz(k, 9, 2) * 2.0, a.sin() * 1.0), 1.5, 1.0, if k % 3 == 0 { SHROUD_D } else { SHROUD });
    }
    s.finish(0.03)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hels_servant_is_shrouded_bowed_and_carries_its_lantern() {
        let c = ganglot();
        let (w, h, d) = GANGLOT_SIZE;
        assert!(c.voxels.iter().all(|v| v.x >= 0 && v.x < w && v.y >= 0 && v.y < h && v.z >= 0 && v.z < d));
        // Every part of its rig has its voxels: the shins, the arms, the hooded head.
        for part in 1..=GANGLOT_RIG.len() as u8 {
            assert!(c.voxels.iter().any(|v| v.part == part), "part {part} is sculpted");
        }
        // Its eyes and its lantern burn.
        let glow = c.voxels.iter().filter(|v| v.c >> 24 == mat::GLOW >> 24).count();
        assert!(glow >= 10, "{glow} glowing voxels");
        // It stands about a tile and a quarter tall, its staff higher.
        let top = c.voxels.iter().filter(|v| v.part == 5).map(|v| v.y).max().unwrap();
        assert!((70..90).contains(&top), "the hood's top at {top}");
        assert!(c.voxels.iter().filter(|v| v.part == 4).map(|v| v.y).max().unwrap() > top, "the staff stands over its head");
    }

    /// The model's sheet (front, side, back / top, two three-quarter views), into VH_PREVIEW_DIR.
    #[test]
    #[ignore]
    fn preview_ganglot() {
        println!("{}", super::super::preview::sheet(&ganglot(), "ganglot"));
    }
}
