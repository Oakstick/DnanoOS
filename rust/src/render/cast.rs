//! The cast of Scene I, sculpted voxel by voxel like the heroes: the four
//! friends, Krampus as a shadow out of the fog, the old gatekeeper and his
//! dog Quitmer. Each is cut into parts that the story animates. Models face
//! +z; coordinates are voxels, y up.

use glam::Vec3;

use super::characters::{mat, Sculpt};
use super::voxels::{Collected, PartDef};

fn nz(x: i32, y: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9e37_79b1) ^ (y as u32).wrapping_mul(0x85eb_ca77) ^ (z as u32).wrapping_mul(0xc2b2_ae3d);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    (h & 0xffff) as f32 / 65535.0
}

fn ball(s: &mut Sculpt, c: Vec3, r: Vec3, color: u32) {
    s.ellipsoid((c.x, c.y, c.z), (r.x, r.y, r.z), 0, color);
}

/// A limb, tail or horn along a path of (point, radius) knots.
fn limb(s: &mut Sculpt, knots: &[(Vec3, f32)], c: u32) {
    for w in knots.windows(2) {
        let (a, b) = (w[0].0, w[1].0);
        s.capsule((a.x, a.y, a.z), (b.x, b.y, b.z), w[0].1, w[1].1, c);
    }
}

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

/// The frontmost filled z at (x, y), if any: where a face's features go on a round head.
fn front(s: &Sculpt, x: i32, y: i32, d: i32) -> Option<i32> {
    (0..d).rev().find(|&z| s.get(x, y, z).is_some())
}

/// Paints the frontmost voxel at (x, y), or sets one `out` voxels in front of it.
fn face(s: &mut Sculpt, x: i32, y: i32, d: i32, out: i32, c: u32) {
    if let Some(z) = front(s, x, y, d) {
        s.set(x, y, z + out, c);
    }
}

/// A rosier version of a skin tone, for cheeks.
fn blush(c: u32) -> u32 {
    let (r, g, b) = ((c >> 16) & 255, (c >> 8) & 255, c & 255);
    ((r + 12).min(255) << 16) | ((g * 82 / 100) << 8) | (b * 78 / 100)
}

/// Shaggy fur: tufts standing out of every face of `coat` and hanging down from it.
fn shag(s: &mut Sculpt, dims: (i32, i32, i32), coat: u32, tuft: u32, density: f32) {
    let (w, h, d) = dims;
    let mut add = vec![];
    for y in 1..h {
        for z in 0..d {
            for x in 0..w {
                if s.get(x, y, z) != Some(coat) {
                    continue;
                }
                for (dx, dy, dz) in [(1, 0, 0), (-1, 0, 0), (0, 0, 1), (0, 0, -1), (0, -1, 0)] {
                    if s.get(x + dx, y + dy, z + dz).is_none() && nz(x * 3 + dx, y * 5 + dy, z * 7 + dz) > 1.0 - density {
                        add.push((x + dx, y + dy, z + dz));
                        if nz(x, y - 1, z + 11) > 0.45 {
                            add.push((x + dx, y + dy - 1, z + dz));
                        }
                    }
                }
            }
        }
    }
    for (x, y, z) in add {
        if s.get(x, y, z).is_none() {
            s.set(x, y, z, tuft);
        }
    }
}

// --------------------------------------------------------------- the boys

/// What a boy wears on his head.
#[derive(Clone, Copy, Debug)]
enum Hat {
    /// A knitted cap with a turned-up brim, and a bobble if `pom` is some colour.
    Knit { color: u32, band: u32, pom: Option<u32> },
    /// A flat cloth cap with a short peak.
    Flat(u32),
    /// Nothing: just his hair.
    Bare,
}

/// How one of the four friends looks.
#[derive(Clone, Copy, Debug)]
struct Kid {
    skin: u32,
    hair: u32,
    coat: u32,
    trim: u32,
    scarf: u32,
    scarf_band: u32,
    breeches: u32,
    socks: u32,
    shoes: u32,
    hat: Hat,
    freckles: bool,
}

const KIDS: [Kid; 4] = [
    // The one Krampus takes: a red bobble cap and a red scarf over a blue coat.
    Kid {
        skin: 0xf0c8a0,
        hair: 0x6a4024,
        coat: 0x34588e,
        trim: 0x284470,
        scarf: 0xc8322a,
        scarf_band: 0xf0e6d0,
        breeches: 0x5e5248,
        socks: 0xb8b0a4,
        shoes: 0x3a2618,
        hat: Hat::Knit { color: 0xc8322a, band: 0xa0241e, pom: Some(0xf4f0e6) },
        freckles: false,
    },
    // A flat cap, a yellow scarf, an olive coat.
    Kid {
        skin: 0xe2b088,
        hair: 0x1e1612,
        coat: 0x5e7a34,
        trim: 0x485e26,
        scarf: 0xe8b83c,
        scarf_band: 0xc88e24,
        breeches: 0x3e3a36,
        socks: 0x7a2e2a,
        shoes: 0x2a1c12,
        hat: Hat::Flat(0x6a4a2c),
        freckles: false,
    },
    // Bare-headed and fair, in a brick-red coat and a grey scarf.
    Kid {
        skin: 0xf6d6b8,
        hair: 0xe6be5e,
        coat: 0x923a2c,
        trim: 0x6e2a20,
        scarf: 0x9aa0a8,
        scarf_band: 0x72787e,
        breeches: 0x4e4a5e,
        socks: 0xd8d0c4,
        shoes: 0x4a2e1a,
        hat: Hat::Bare,
        freckles: false,
    },
    // Freckled and ginger, a teal cap and scarf, a plum coat.
    Kid {
        skin: 0xf0c4a2,
        hair: 0xc4602a,
        coat: 0x5a3e6e,
        trim: 0x442e54,
        scarf: 0x2e9a8c,
        scarf_band: 0x227266,
        breeches: 0x6a5a44,
        socks: 0x4a6a8a,
        shoes: 0x2e2014,
        hat: Hat::Knit { color: 0x2e9a8c, band: 0x227266, pom: None },
        freckles: true,
    },
];

/// The friends' grid: 18 wide, 29 tall, 13 deep.
pub const KID_SIZE: (i32, i32, i32) = (18, 29, 13);

/**
 * One of the four friends (0 is the one Krampus takes): a big round head
 * under a cap or a mop of hair, a scarf wound twice round the neck with its
 * end hanging, a buttoned coat, breeches, long socks and boots.
 */
pub fn kid(which: usize) -> Collected {
    let k = KIDS[which % KIDS.len()];
    let (w, h, d) = KID_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let cloth = |c: u32| c | mat::CLOTH;
    let (coat, trim) = (cloth(k.coat), cloth(k.trim));
    let skin = k.skin | mat::SKIN;
    let skin_d = scale(k.skin, 0.84) | mat::SKIN;
    let hair = k.hair | mat::FUR;

    // ---- boots, socks, breeches
    for (x0, x1) in [(4, 7), (10, 13)] {
        s.cuboid(x0, 0, 4, x1, 1, 9, k.shoes | mat::LEATHER);
        s.cuboid(x0, 0, 4, x1, 0, 9, scale(k.shoes, 0.7) | mat::LEATHER);
        s.cuboid(x0 + 1, 2, 5, x1 - 1, 4, 7, cloth(k.socks));
        s.cuboid(x0, 5, 4, x1, 8, 8, cloth(k.breeches));
        s.cuboid(x0, 5, 4, x1, 5, 8, cloth(scale(k.breeches, 0.8)));
    }
    // ---- the coat: a skirt flaring front and back, the body, a row of buttons, cuffed sleeves
    s.cuboid(4, 8, 2, 13, 10, 10, coat);
    s.cuboid(4, 11, 3, 13, 16, 9, coat);
    s.cuboid(4, 8, 2, 13, 8, 10, trim);
    s.cuboid(8, 8, 10, 9, 10, 10, trim);
    s.cuboid(8, 11, 9, 9, 16, 9, trim);
    for y in [9, 12, 14] {
        s.set(8, y, if y < 11 { 11 } else { 10 }, 0xd8c8a0 | mat::GOLD);
    }
    s.cuboid(4, 11, 3, 13, 11, 9, cloth(scale(k.trim, 0.9))); // a belt of the darker cloth
    for (x0, x1, hand) in [(1, 3, (1, 2)), (14, 16, (15, 16))] {
        s.cuboid(x0, 11, 4, x1, 16, 8, coat);
        s.cuboid(x0, 10, 4, x1, 10, 8, trim);
        s.cuboid(hand.0, 8, 5, hand.1, 9, 7, skin);
    }
    // ---- the scarf: twice round the neck, one end hanging down the front
    let (scarf, band) = (cloth(k.scarf), cloth(k.scarf_band));
    s.cuboid(4, 16, 3, 13, 17, 10, scarf);
    s.paint(|x, y, _z, c| if c == scarf && y == 17 && x % 3 == 0 { Some(band) } else { None });
    for y in 12..=15 {
        s.cuboid(5, y, 10, 6, y, 10, if y % 3 == 0 { band } else { scarf });
    }
    s.cuboid(5, 11, 11, 6, 11, 11, band);

    // ---- the head: big and round, ears, eyes, a small nose, rosy cheeks
    s.ellipsoid((8.5, 22.5, 6.5), (5.4, 5.6, 4.9), 18, skin);
    for x in [3, 14] {
        s.cuboid(x, 21, 6, x, 22, 7, skin_d);
    }
    // Eyes (a glint in each), a button nose, a small mouth, rosy cheeks.
    let eye = 0x241c18;
    for x in [6, 11] {
        face(&mut s, x, 22, d, 0, eye);
        face(&mut s, x, 23, d, 0, eye);
    }
    face(&mut s, 7, 23, d, 0, 0xf4f0e8);
    face(&mut s, 10, 23, d, 0, 0xf4f0e8);
    for x in [8, 9] {
        face(&mut s, x, 21, d, 1, skin_d);
        face(&mut s, x, 19, d, 0, 0x9a3a32 | mat::SKIN);
    }
    for x in [5, 12] {
        face(&mut s, x, 20, d, 0, blush(k.skin) | mat::SKIN);
    }
    if k.freckles {
        for (x, y) in [(5, 21), (6, 20), (11, 20), (12, 21), (7, 21), (10, 21)] {
            face(&mut s, x, y, d, 0, 0xc88a5a | mat::SKIN);
        }
    }
    // Brows and a fringe of hair over the forehead, the rest round the back.
    for x in [5, 6, 11, 12] {
        face(&mut s, x, 25, d, 0, hair);
    }
    s.paint(|x, y, z, c| {
        if c != skin {
            return None;
        }
        let back = z <= 5 && y >= 19;
        let top = y >= 26;
        let side = (x <= 4 || x >= 13) && y >= 23;
        let fringe = z >= 9 && y >= 26;
        if back || top || side || fringe {
            Some(hair)
        } else {
            None
        }
    });
    match k.hat {
        Hat::Knit { color, band, pom } => {
            let (color, band) = (cloth(color), cloth(band));
            s.ellipsoid((8.5, 24.5, 6.5), (5.9, 4.3, 5.3), 24, color);
            s.paint(|_x, y, _z, c| if c == color && y <= 25 { Some(band) } else { None });
            s.paint(|x, y, z, c| if c == color && (x + z) % 2 == 0 && y % 2 == 0 { Some(scale(color & 0xff_ffff, 0.9) | mat::CLOTH) } else { None });
            if let Some(p) = pom {
                ball(&mut s, v(8.5, 27.6, 6.5), Vec3::splat(1.6), p | mat::FUR);
            }
        }
        Hat::Flat(c) => {
            let c = cloth(c);
            s.ellipsoid((8.5, 25.8, 6.2), (5.8, 2.6, 5.4), 25, c);
            s.cuboid(4, 25, 10, 13, 25, 12, c);
            s.cuboid(5, 25, 12, 12, 25, 12, cloth(scale(c & 0xff_ffff, 0.8)));
        }
        Hat::Bare => {
            // A tousled mop: tufts sticking up and out.
            for (x, y, z) in [(6, 28, 6), (9, 28, 5), (11, 28, 7), (4, 27, 8), (13, 27, 5), (8, 28, 9), (7, 27, 10), (10, 27, 10)] {
                s.set(x, y, z, hair);
            }
            s.paint(|x, y, z, c| if c == hair && nz(x, y, z) > 0.8 { Some(scale(k.hair, 1.12) | mat::FUR) } else { None });
        }
    }
    s.finish(0.03)
}

/// A boy's joints: legs from the hips, arms from the shoulders, the head (with its cap).
pub const KID_RIG: &[PartDef] = &[
    PartDef { name: "legA", boxes: &[([3, 0, 0], [8, 7, 12])], pivot: [5.5, 8.0, 6.5] },
    PartDef { name: "legB", boxes: &[([9, 0, 0], [14, 7, 12])], pivot: [11.5, 8.0, 6.5] },
    PartDef { name: "armA", boxes: &[([0, 8, 0], [3, 16, 12])], pivot: [2.5, 15.5, 6.0] },
    PartDef { name: "armB", boxes: &[([14, 8, 0], [17, 16, 12])], pivot: [15.0, 15.5, 6.0] },
    PartDef { name: "head", boxes: &[([0, 18, 0], [17, 28, 12])], pivot: [8.5, 18.0, 6.5] },
];

/// A boy's middle, in his grid: where Krampus's claws close on him.
pub const KID_WAIST: [f32; 3] = [8.5, 12.0, 6.5];

/// Scales a colour's brightness (keeping its material byte).
fn scale(c: u32, k: f32) -> u32 {
    let ch = |s: u32| ((((c >> s) & 255) as f32 * k).clamp(0.0, 255.0) as u32) << s;
    ch(16) | ch(8) | ch(0) | (c & 0xff00_0000)
}

// ----------------------------------------------------------- the old man

mod old {
    use super::mat;
    pub const ROBE: u32 = 0x3c4658 | mat::CLOTH;
    pub const ROBE_D: u32 = 0x2c3444 | mat::CLOTH;
    pub const ROBE_L: u32 = 0x4a566a | mat::CLOTH;
    pub const PATCH: u32 = 0x6a5236 | mat::CLOTH;
    pub const STITCH: u32 = 0xb8a47a | mat::CLOTH;
    pub const HOOD: u32 = 0x323a4a | mat::CLOTH;
    pub const ROPE: u32 = 0x9a8660 | mat::CLOTH;
    pub const ROPE_D: u32 = 0x76663e | mat::CLOTH;
    pub const SKIN: u32 = 0xdcae8e | mat::SKIN;
    pub const SKIN_D: u32 = 0xba8a6c | mat::SKIN;
    pub const NOSE: u32 = 0xd08a76 | mat::SKIN;
    pub const BEARD: u32 = 0xe8e6de | mat::FUR;
    pub const BEARD_D: u32 = 0xc4c2ba | mat::FUR;
    pub const BEARD_L: u32 = 0xf8f6f0 | mat::FUR;
    pub const EYE: u32 = 0x2a2420;
    pub const IRON: u32 = 0x4a4c50 | mat::METAL;
    pub const IRON_L: u32 = 0x7a7e84 | mat::METAL;
    pub const STAFF: u32 = 0x5a3e22 | mat::WOOD;
    pub const STAFF_D: u32 = 0x3e2a16 | mat::WOOD;
    pub const SANDAL: u32 = 0x5a3a1e | mat::LEATHER;
}

/// The old man's grid: 30 wide, 52 tall (his staff tops his head), 20 deep.
pub const OLD_MAN_SIZE: (i32, i32, i32) = (30, 52, 20);

/**
 * The old gatekeeper: a stooped figure in a long patched robe of slate-blue
 * wool, its hood up round a long weathered face (a big red nose under
 * shaggy white brows, heavy-lidded eyes), a white beard down to his rope
 * belt, which is hung with great iron keys. His left hand leans on a
 * gnarled staff taller than he is.
 */
pub fn old_man() -> Collected {
    use old::*;
    let (w, h, d) = OLD_MAN_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let (cx, cz) = (14.5f32, 9.5f32);

    // ---- the robe: a long bell of wool, its hem ragged, folds down it, a patch
    for y in 0..=31 {
        let t = y as f32 / 31.0;
        let rx = 7.6 - 1.9 * t + if y > 26 { (y - 26) as f32 * 0.25 } else { 0.0 };
        let rz = 5.6 - 1.2 * t;
        for z in 0..d {
            for x in 0..w {
                let (dx, dz) = ((x as f32 - cx) / rx, (z as f32 - cz) / rz);
                if dx * dx + dz * dz <= 1.0 {
                    s.set(x, y, z, ROBE);
                }
            }
        }
    }
    // A ragged hem: a few voxels missing along the bottom.
    for z in 0..d {
        for x in 0..w {
            if s.get(x, 0, z).is_some() && nz(x, 0, z) > 0.72 {
                s.clear(x, 0, z);
            }
        }
    }
    s.paint(|x, y, z, c| {
        if c != ROBE {
            return None;
        }
        let a = ((z as f32 - cz).atan2(x as f32 - cx) * 5.0).sin();
        if a > 0.75 && y < 26 {
            return Some(ROBE_D);
        }
        if a < -0.85 && y < 26 {
            return Some(ROBE_L);
        }
        let n = nz(x, y / 2, z);
        Some(if n < 0.06 { ROBE_D } else { ROBE })
    });
    // A square patch on the front, stitched round.
    for y in 6..=10 {
        for x in 16..=19 {
            let z = (0..d).rev().find(|&z| s.get(x, y, z).is_some()).unwrap_or(0);
            let edge = y == 6 || y == 10 || x == 16 || x == 19;
            s.set(x, y, z, if edge && (x + y) % 2 == 0 { STITCH } else { PATCH });
        }
    }
    // Sandal toes peeping out under the hem.
    for x0 in [10, 16] {
        s.cuboid(x0, 0, 14, x0 + 3, 1, 16, SANDAL);
        s.cuboid(x0, 1, 15, x0 + 3, 1, 15, SKIN_D);
    }

    // ---- shoulders, and the hood fallen down his back in folds
    ball(&mut s, v(cx, 29.5, cz), v(7.8, 2.4, 5.0), ROBE);
    ball(&mut s, v(cx, 31.0, 6.0), v(6.0, 2.2, 2.4), HOOD);

    // ---- the rope belt, its knot and ends, and the ring of keys at his right hip
    for z in 0..d {
        for x in 0..w {
            for y in [17, 18] {
                if s.get(x, y, z).is_some() && (s.get(x + 1, y, z).is_none() || s.get(x - 1, y, z).is_none() || s.get(x, y, z + 1).is_none() || s.get(x, y, z - 1).is_none()) {
                    s.set(x, y, z, if (x + z + y) % 3 == 0 { ROPE_D } else { ROPE });
                }
            }
        }
    }
    let front = |s: &Sculpt, x: i32, y: i32| (0..20).rev().find(|&z| s.get(x, y, z).is_some()).unwrap_or(10);
    let kz = front(&s, 14, 17) + 1;
    s.cuboid(13, 16, kz, 15, 18, kz, ROPE_D);
    for (x, len) in [(13, 5), (15, 7)] {
        for y in (17 - len)..16 {
            s.set(x, y, kz, if y % 2 == 0 { ROPE } else { ROPE_D });
        }
    }
    // The ring hangs off his left side (the viewer's right), the keys from it.
    let (rx, ry, rz) = (21.5f32, 13.0f32, 12.0f32);
    for k in 0..16 {
        let a = k as f32 / 16.0 * std::f32::consts::TAU;
        s.set((rx + a.cos() * 2.0).round() as i32, (ry + a.sin() * 2.0).round() as i32, rz as i32, IRON);
    }
    for (i, dx) in [-1.5f32, 0.0, 1.5].into_iter().enumerate() {
        let x = (rx + dx).round() as i32;
        let top = ry as i32 - 2;
        let len = 5 + i as i32;
        for y in (top - len)..top {
            s.set(x, y, rz as i32 + 1, IRON_L);
        }
        s.cuboid(x, top - len - 1, rz as i32 + 1, x + 1, top - len, rz as i32 + 1, IRON);
        s.cuboid(x - 1, top, rz as i32 + 1, x + 1, top + 1, rz as i32 + 1, IRON);
    }

    // ---- arms: wide sleeves, old knotted hands; the left grips the staff, the right hangs
    limb(&mut s, &[(v(7.2, 30.0, 10.0), 2.6), (v(5.8, 25.0, 11.5), 2.8), (v(5.0, 23.0, 15.0), 2.9)], ROBE);
    ball(&mut s, v(5.0, 23.0, 16.2), v(1.5, 1.6, 1.3), SKIN);
    limb(&mut s, &[(v(22.4, 30.0, 10.0), 2.6), (v(23.6, 24.0, 10.8), 2.9), (v(23.8, 21.0, 11.0), 3.1)], ROBE);
    ball(&mut s, v(23.8, 19.6, 11.2), v(1.5, 1.7, 1.4), SKIN);
    s.paint(|x, y, z, c| if c == ROBE && (x <= 8 || x >= 21) && y >= 20 && y <= 22 && nz(x, y, z) > 0.4 { Some(ROBE_D) } else { None });
    // The staff: gnarled, a knot here and there, a knob on top worn smooth.
    for y in 0..=48 {
        let wob = ((y as f32) * 0.35).sin() * 0.5;
        let x = (5.0 + wob).round() as i32;
        let c = if y % 7 == 3 { STAFF_D } else { STAFF };
        s.set(x, y, 16, c);
        s.set(x, y, 17, c);
        if y > 30 {
            s.set(x - 1, y, 16, STAFF_D);
        }
    }
    ball(&mut s, v(5.0, 49.0, 16.5), v(1.9, 1.6, 1.8), STAFF);
    s.set(5, 50, 17, STAFF_D);

    // ---- the head: a long face in the shadow of his hood, heavy white brows, a big red nose
    s.cuboid(12, 32, 8, 17, 33, 12, SKIN_D);
    ball(&mut s, v(cx, 38.0, 11.2), v(4.6, 5.4, 4.4), SKIN);
    for x in [10, 19] {
        s.cuboid(x, 36, 10, x, 37, 11, SKIN_D);
    }
    // The hood, up: a cowl round the head, falling onto the shoulders, the face left open.
    ball(&mut s, v(cx, 39.0, 10.0), v(6.1, 6.6, 5.6), HOOD);
    for y in 31..=44 {
        for z in 0..d {
            for x in 0..w {
                let (dx, dy) = ((x as f32 - cx) / 3.9, (y as f32 - 38.0) / 5.6);
                let opening = z as f32 > 12.0 && dx * dx + dy * dy < 1.0;
                if s.get(x, y, z) == Some(HOOD) && opening {
                    s.clear(x, y, z);
                }
            }
        }
    }
    // Put back the face the cowl covered.
    for y in 33..=43 {
        for z in 12..d {
            for x in 0..w {
                let (dx, dy, dz) = ((x as f32 - cx) / 4.6, (y as f32 - 38.0) / 5.4, (z as f32 - 11.2) / 4.4);
                if dx * dx + dy * dy + dz * dz <= 1.0 && s.get(x, y, z).is_none() {
                    s.set(x, y, z, SKIN);
                }
            }
        }
    }
    s.paint(|x, y, z, c| {
        if c != HOOD {
            return None;
        }
        let rim = z >= 12;
        Some(if rim { ROBE_L } else if (x + y + z) % 5 == 0 { ROBE_D } else { HOOD })
    });
    // White hair escaping at the temples.
    for (x, y) in [(11, 41), (18, 41), (10, 40), (19, 40), (10, 39), (19, 39)] {
        face(&mut s, x, y, d, 0, BEARD);
    }
    // Wrinkles across the brow; eyes half shut under heavy lids.
    for x in [12, 14, 16] {
        face(&mut s, x, 42, d, 0, SKIN_D);
    }
    for x in [12, 13, 16, 17] {
        face(&mut s, x, 38, d, 0, EYE);
        face(&mut s, x, 39, d, 0, SKIN_D);
    }
    // Shaggy white brows jutting out over them.
    for x in [11, 12, 13, 14, 15, 16, 17, 18] {
        if x != 14 && x != 15 {
            face(&mut s, x, 40, d, 1, BEARD_L);
        }
    }
    for x in [11, 18] {
        face(&mut s, x, 41, d, 1, BEARD);
    }
    // The nose: big, bulbous, reddened; ruddy cheeks either side.
    for y in 35..=37 {
        for x in [14, 15] {
            face(&mut s, x, y, d, 1, NOSE);
        }
    }
    face(&mut s, 14, 35, d, 1, NOSE);
    face(&mut s, 15, 35, d, 1, NOSE);
    for x in [11, 18] {
        face(&mut s, x, 36, d, 0, NOSE);
    }

    // ---- the beard: from his chin down to his belt, a drooping moustache over it
    limb(&mut s, &[(v(cx, 33.2, 14.2), 3.8), (v(cx, 28.0, 15.0), 3.9), (v(cx, 22.0, 15.4), 2.8), (v(cx, 18.6, 15.6), 1.2)], BEARD);
    for y in 35..45 {
        for z in 0..d {
            for x in 0..w {
                if s.get(x, y, z) == Some(BEARD) {
                    s.clear(x, y, z);
                }
            }
        }
    }
    for x in 11..=18 {
        face(&mut s, x, 34, d, 1, BEARD_L);
    }
    for (x, y) in [(11, 33), (18, 33), (11, 32), (18, 32)] {
        face(&mut s, x, y, d, 1, BEARD_L);
    }
    s.paint(|x, y, z, c| {
        if c != BEARD {
            return None;
        }
        let n = nz(x, y / 3, z);
        Some(if n < 0.3 { BEARD_D } else if n > 0.75 { BEARD_L } else { BEARD })
    });
    s.finish(0.03)
}

/// The old man's joints: the head (and beard), the staff arm, the free arm.
pub const OLD_MAN_RIG: &[PartDef] = &[
    PartDef { name: "head", boxes: &[([7, 32, 3], [22, 47, 19]), ([9, 19, 15], [20, 31, 19])], pivot: [14.5, 32.0, 11.0] },
    PartDef { name: "staffArm", boxes: &[([2, 19, 6], [7, 31, 19]), ([2, 0, 14], [7, 51, 19])], pivot: [7.5, 30.0, 10.5] },
    PartDef { name: "freeArm", boxes: &[([22, 16, 5], [27, 32, 17])], pivot: [21.5, 30.0, 10.5] },
];

// ---------------------------------------------------------------- Quitmer

mod hound {
    use super::mat;
    pub const COAT: u32 = 0x4a3a2e | mat::FUR;
    pub const COAT_D: u32 = 0x2a2019 | mat::FUR;
    pub const COAT_L: u32 = 0x6a5644 | mat::FUR;
    pub const MASK: u32 = 0x1c1614 | mat::FUR;
    pub const NOSE: u32 = 0x0e0c0c | mat::LEATHER;
    pub const GUM: u32 = 0x7a2a30 | mat::SKIN;
    pub const TOOTH: u32 = 0xeae2cc | mat::IVORY;
    pub const EYE: u32 = 0xffb030;
    pub const SCAR: u32 = 0x8a6a64 | mat::SKIN;
    pub const COLLAR: u32 = 0x1e1614 | mat::LEATHER;
    pub const SPIKE: u32 = 0xa8acb2 | mat::METAL;
    pub const RING: u32 = 0x5a5c62 | mat::METAL;
    pub const CLAW: u32 = 0x3a342e | mat::IVORY;
}

/// Quitmer's grid: 24 wide, 36 tall, 58 long (tail at z = 0, nose at the far end).
pub const DOG_SIZE: (i32, i32, i32) = (24, 36, 58);
/// Where his chain fastens (the ring under his collar), in his grid.
pub const DOG_COLLAR_RING: [f32; 3] = [11.5, 19.0, 45.5];
/// His eyes, in his grid (they glow).
pub const DOG_EYES: [[f32; 3]; 2] = [[8.0, 27.5, 52.5], [15.0, 27.5, 52.5]];

/**
 * Quitmer, the gatekeeper's dog: a mastiff the size of a pony, dark brindle
 * with a black mask, a scarred brow, cropped ears standing up, glowing amber
 * eyes, heavy jowls over his fangs and a collar of black leather set with
 * iron spikes, its ring waiting for the chain.
 */
pub fn dog() -> Collected {
    use hound::*;
    let (w, h, d) = DOG_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let cx = 11.5f32;

    // ---- the body: haunches, a long back, a deep chest, a hump of shoulder
    ball(&mut s, v(cx, 17.0, 26.0), v(7.4, 6.8, 15.5), COAT);
    ball(&mut s, v(cx, 16.5, 36.5), v(8.2, 8.4, 7.2), COAT);
    ball(&mut s, v(cx, 20.5, 38.5), v(7.6, 4.8, 6.0), COAT);
    ball(&mut s, v(cx, 18.0, 15.5), v(7.6, 6.4, 6.0), COAT);
    // ---- hind legs: heavy hams, the hock angled back, big paws
    for side in [-1.0f32, 1.0] {
        let x = cx + side * 4.8;
        ball(&mut s, v(x, 14.5, 15.0), v(3.2, 5.4, 4.8), COAT);
        limb(&mut s, &[(v(x, 11.0, 13.5), 2.5), (v(x, 6.0, 12.0), 2.0), (v(x, 2.0, 14.5), 1.9)], COAT);
        ball(&mut s, v(x, 1.2, 15.6), v(2.3, 1.3, 2.9), COAT_D);
        // ---- forelegs: straight columns of muscle down to broad paws
        limb(&mut s, &[(v(x, 15.0, 39.5), 3.0), (v(x, 8.0, 40.0), 2.5), (v(x, 3.0, 41.0), 2.2)], COAT);
        ball(&mut s, v(x, 1.3, 42.2), v(2.5, 1.4, 3.1), COAT_D);
        for k in [-1.0f32, 0.0, 1.0] {
            s.set((x + k).round() as i32, 0, 45, CLAW);
            s.set((x + k).round() as i32, 0, 19, CLAW);
        }
    }
    // ---- the neck, thick and short, and the collar round it
    limb(&mut s, &[(v(cx, 21.0, 41.0), 5.8), (v(cx, 24.5, 46.0), 5.2)], COAT);
    for y in 14..30 {
        for z in 42..47 {
            for x in 0..w {
                // A band round the neck's axis: leather on the outside, spikes set in it.
                let t = ((z as f32 - 41.0) / 5.0).clamp(0.0, 1.0);
                let (ay, az) = (21.0 + 3.5 * t, 41.0 + 5.0 * t);
                let r = ((x as f32 - cx).powi(2) + (y as f32 - ay).powi(2)).sqrt();
                if (43..=44).contains(&z) && r >= 5.0 && r <= 6.4 && (az - z as f32).abs() < 3.0 {
                    s.set(x, y, z, COLLAR);
                }
            }
        }
    }
    for k in 0..10 {
        let a = k as f32 / 10.0 * std::f32::consts::TAU;
        let (x, y) = (cx + a.cos() * 7.2, 23.2 + a.sin() * 7.2);
        s.set(x.round() as i32, y.round() as i32, 44, SPIKE);
    }
    for (x, y) in [(10, 17), (11, 16), (12, 16), (13, 17), (11, 18), (12, 18)] {
        s.set(x, y, 45, RING);
    }

    // ---- the head: a broad skull, heavy jowls, a short square muzzle
    ball(&mut s, v(cx, 27.5, 48.0), v(6.2, 5.6, 5.6), COAT);
    ball(&mut s, v(cx, 24.2, 51.0), v(5.4, 3.4, 4.2), COAT);
    s.cuboid(8, 22, 51, 15, 26, 56, MASK);
    ball(&mut s, v(cx, 24.5, 55.0), v(3.8, 2.4, 1.8), MASK);
    s.cuboid(10, 25, 57, 13, 26, 57, NOSE);
    s.set(10, 24, 57, NOSE);
    s.set(13, 24, 57, NOSE);
    // Upper fangs under the lip.
    for x in [9, 14] {
        s.set(x, 22, 56, TOOTH);
    }
    s.cuboid(10, 22, 56, 13, 22, 56, GUM);
    // The lower jaw (a part of its own, so it can snap).
    s.cuboid(9, 19, 49, 14, 21, 55, MASK);
    for x in [9, 14] {
        s.set(x, 21, 55, TOOTH);
    }
    s.cuboid(10, 21, 54, 13, 21, 54, GUM);
    // Eyes under a heavy brow, a pale scar across the left.
    for e in DOG_EYES {
        let (x, y, z) = (e[0] as i32, e[1] as i32, e[2] as i32);
        s.cuboid(x, y, z, x, y, z + 1, EYE);
        s.set(x, y + 1, z + 1, COAT_D);
    }
    s.cuboid(7, 29, 51, 16, 29, 52, COAT_D);
    for (x, y, z) in [(6, 31, 50), (7, 30, 51), (7, 28, 52), (8, 26, 53)] {
        s.set(x, y, z, SCAR);
    }
    // Cropped ears, standing up.
    for side in [-1.0f32, 1.0] {
        let x = cx + side * 4.3;
        limb(&mut s, &[(v(x, 31.5, 47.0), 1.8), (v(x + side * 0.6, 34.0, 46.2), 0.9), (v(x + side * 0.8, 35.4, 45.8), 0.4)], COAT_D);
    }
    // ---- a tail curling up
    limb(&mut s, &[(v(cx, 20.5, 10.5), 2.0), (v(cx, 23.0, 5.5), 1.6), (v(cx, 26.5, 2.5), 1.3), (v(cx, 29.0, 2.0), 0.9)], COAT);

    // ---- brindle stripes over it all, darker down the spine, the belly lighter
    s.paint(|x, y, z, c| {
        if c != COAT {
            return None;
        }
        let stripe = ((z as f32 * 0.9 + y as f32 * 0.5 + (x as f32 * 0.7).sin() * 1.5) * 0.9).sin();
        let n = nz(x, y, z);
        if y > 22 && (x as f32 - cx).abs() < 2.0 && z < 44 {
            return Some(COAT_D);
        }
        if y < 12 && z > 20 && z < 36 {
            return Some(COAT_L);
        }
        Some(if stripe > 0.6 { COAT_D } else if n > 0.9 { COAT_L } else { COAT })
    });
    shag(&mut s, (w, h, d), COAT, COAT_D, 0.1);
    s.finish(0.035)
}

/// Quitmer's joints. The jaw is listed before the head so its voxels go to it.
pub const DOG_RIG: &[PartDef] = &[
    PartDef { name: "jaw", boxes: &[([7, 17, 48], [16, 21, 57])], pivot: [11.5, 22.0, 49.0] },
    PartDef { name: "head", boxes: &[([2, 22, 46], [21, 35, 57]), ([2, 26, 45], [21, 35, 46])], pivot: [11.5, 24.0, 45.0] },
    PartDef { name: "foreA", boxes: &[([2, 0, 35], [9, 8, 47])], pivot: [6.7, 8.5, 40.5] },
    PartDef { name: "foreB", boxes: &[([14, 0, 35], [21, 8, 47])], pivot: [16.3, 8.5, 40.5] },
    PartDef { name: "hindA", boxes: &[([2, 0, 8], [9, 8, 21])], pivot: [6.7, 8.5, 13.5] },
    PartDef { name: "hindB", boxes: &[([14, 0, 8], [21, 8, 21])], pivot: [16.3, 8.5, 13.5] },
    PartDef { name: "tail", boxes: &[([7, 19, 0], [16, 31, 9])], pivot: [11.5, 20.5, 10.0] },
];

// ---------------------------------------------------------------- Krampus

mod shade {
    use super::mat;
    pub const FUR: u32 = 0x1a151d | mat::FUR;
    pub const FUR_D: u32 = 0x0e0b10 | mat::FUR;
    pub const FUR_L: u32 = 0x2e2533 | mat::FUR;
    pub const HIDE: u32 = 0x2c2428 | mat::SKIN;
    pub const HIDE_D: u32 = 0x1e181c | mat::SKIN;
    pub const HORN: u32 = 0x2e2820 | mat::IVORY;
    pub const HORN_D: u32 = 0x1e1a14 | mat::IVORY;
    pub const HORN_TIP: u32 = 0x5a4e3a | mat::IVORY;
    pub const HOOF: u32 = 0x141210 | mat::IVORY;
    pub const CLAW: u32 = 0x8a7c66 | mat::IVORY;
    pub const TONGUE: u32 = 0x9a1a2a | mat::SKIN;
    pub const MAW: u32 = 0x0a0606;
    pub const FANG: u32 = 0xe4d8bc | mat::IVORY;
    pub const EYE: u32 = 0xff3a1a;
    pub const WICKER: u32 = 0x4a3622 | mat::WOOD;
    pub const WICKER_D: u32 = 0x33251a | mat::WOOD;
    pub const BIRCH: u32 = 0x8a7a66 | mat::WOOD;
    pub const CHAIN: u32 = 0x3c3e44 | mat::METAL;
    pub const BELL: u32 = 0x8e7034 | mat::GOLD;
    pub const STRAP: u32 = 0x2a1e16 | mat::LEATHER;
}

/// Krampus's grid: 46 wide, 64 tall (horns and all), 42 deep.
pub const KRAMPUS_SIZE: (i32, i32, i32) = (46, 64, 42);
/// His eyes, in his grid.
pub const KRAMPUS_EYES: [[f32; 3]; 2] = [[20.5, 49.0, 34.5], [25.5, 49.0, 34.5]];
/// The claws of his right hand (on the viewer's left as he faces you), where he holds what he takes.
pub const KRAMPUS_HAND: [f32; 3] = [9.0, 23.0, 30.5];

/**
 * Krampus as the boys see him: a shadow come out of the fog, black shaggy
 * fur on a hunched man-goat, long arms with hooked claws hanging almost to
 * his knees, cloven hooves, a goat's face thrust forward with a long red
 * tongue lolling out, ember eyes, great curling horns, chains wound round
 * him hung with bells, and a wicker basket on his back for children.
 */
pub fn krampus_shade() -> Collected {
    use shade::*;
    let (w, h, d) = KRAMPUS_SIZE;
    let mut s = Sculpt::new(w, h, d);
    let cx = 23.0f32;

    // ---- goat legs: hooves, thin cannons back up to the hock, shaggy thighs forward to the hip
    for side in [-1.0f32, 1.0] {
        let x = cx + side * 5.0;
        ball(&mut s, v(x, 1.6, 22.5), v(2.4, 1.8, 3.0), HOOF);
        for y in 0..3 {
            for z in 23..27 {
                s.clear(x.round() as i32, y, z);
            }
        }
        limb(&mut s, &[(v(x, 3.0, 21.5), 1.6), (v(x, 12.0, 16.5), 1.5)], HIDE_D);
        ball(&mut s, v(x, 12.5, 16.2), Vec3::splat(2.2), FUR);
        limb(&mut s, &[(v(x, 13.0, 16.0), 2.8), (v(x + side * 0.4, 18.5, 19.0), 3.9), (v(x + side * 0.2, 23.0, 20.0), 4.3)], FUR);
        ball(&mut s, v(x, 4.5, 21.0), v(2.4, 1.4, 2.4), FUR); // fetlock tuft
    }
    // ---- hips, belly, chest, and heavy shoulders, all hunched forward
    ball(&mut s, v(cx, 25.5, 18.5), v(8.4, 4.6, 6.2), FUR);
    ball(&mut s, v(cx, 31.0, 19.5), v(7.8, 6.2, 6.4), FUR);
    ball(&mut s, v(cx, 37.0, 21.5), v(9.2, 6.0, 6.6), FUR);
    ball(&mut s, v(cx, 40.0, 22.5), v(11.2, 3.6, 5.6), FUR);
    // ---- the basket on his back, bristling with birch switches, and its straps
    let (bx0, bx1) = (16, 30);
    for y in 29..=45 {
        for z in 7..=14 {
            for x in bx0..=bx1 {
                let wall = z == 7 || z == 14 || x == bx0 || x == bx1 || y == 29;
                if wall {
                    let band = y == 29 || y == 37 || y == 45;
                    s.set(x, y, z, if band { STRAP } else if (x + y + z) % 2 == 0 { WICKER } else { WICKER_D });
                }
            }
        }
    }
    for i in 0..7 {
        let fi = i as f32;
        let base = v(18.0 + fi * 1.7, 40.0, 10.5 + (i % 2) as f32);
        let tip = v(17.0 + fi * 2.0, 52.0 + (i % 3) as f32, 8.5 + (i % 3) as f32);
        s.capsule((base.x, base.y, base.z), (tip.x, tip.y, tip.z), 0.5, 0.3, BIRCH);
    }
    for side in [-1.0f32, 1.0] {
        limb(&mut s, &[(v(cx + side * 5.0, 44.0, 14.0), 0.8), (v(cx + side * 6.0, 43.5, 20.0), 0.8), (v(cx + side * 6.0, 36.0, 27.5), 0.8)], STRAP);
    }
    // ---- a chain round his waist and another across his chest, hung with bells
    for (cy, r, tilt) in [(27.0f32, 9.2f32, 0.0f32), (36.0, 10.0, 0.35)] {
        for i in 0..64 {
            let a = i as f32 / 64.0 * std::f32::consts::TAU;
            if i % 3 == 2 {
                continue;
            }
            let (x, z) = (cx + a.cos() * r, 19.5 + a.sin() * (r * 0.72));
            let y = cy + (a * 2.0).sin() * 0.6 + a.cos() * tilt * 6.0;
            s.set(x.round() as i32, y.round() as i32, z.round() as i32, CHAIN);
        }
    }
    for (dx, y) in [(-4.0f32, 24.5f32), (3.5, 24.0), (0.0, 23.2), (-6.5, 33.0), (6.0, 36.5)] {
        ball(&mut s, v(cx + dx, y, 26.6), v(1.4, 1.6, 1.4), BELL);
        s.set((cx + dx).round() as i32, y as i32 - 2, 27, HORN_D);
    }

    // ---- arms: long and hanging forward, shaggy to the elbow, clawed hands
    for side in [-1.0f32, 1.0] {
        let sx = cx + side * 10.5;
        let elbow = v(cx + side * 13.0, 33.0, 25.0);
        let wrist = v(cx + side * 14.0, 25.5, 28.5);
        limb(&mut s, &[(v(sx, 40.5, 22.0), 3.2), (elbow, 2.7)], FUR);
        ball(&mut s, elbow, Vec3::splat(2.9), FUR_L);
        limb(&mut s, &[(elbow, 2.3), (wrist, 1.9)], HIDE_D);
        ball(&mut s, wrist + v(0.0, -1.2, 0.6), v(2.0, 2.2, 1.8), HIDE);
        for (i, off) in [(-1.2f32, 0.4f32), (0.0, 0.8), (1.2, 0.4), (side * -1.8, -0.6)].into_iter().enumerate() {
            let base = wrist + v(off.0, -2.8, 1.0 + off.1);
            let tip = base + v(off.0 * 0.3, -3.2, 1.8 - i as f32 * 0.3);
            s.capsule((base.x, base.y, base.z), (tip.x, tip.y, tip.z), 0.7, 0.35, CLAW);
        }
    }

    // ---- neck and head, thrust forward and low
    limb(&mut s, &[(v(cx, 39.5, 23.0), 4.2), (v(cx, 44.0, 27.0), 3.6)], FUR);
    ball(&mut s, v(cx, 46.5, 24.0), v(6.2, 6.2, 4.6), FUR); // the mane behind the head
    ball(&mut s, v(cx, 47.5, 29.0), v(4.8, 4.6, 4.8), HIDE); // skull
    ball(&mut s, v(cx, 45.2, 33.4), v(3.1, 2.7, 3.4), HIDE); // muzzle
    ball(&mut s, v(cx, 42.4, 32.6), v(2.9, 1.3, 3.2), HIDE_D); // the jaw, hanging open
    for z in 31..=36 {
        for x in (cx - 1.5) as i32..=(cx + 1.5) as i32 {
            s.set(x, 43, z, MAW);
        }
    }
    for side in [-1.0f32, 1.0] {
        let x = (cx + side * 1.6).round() as i32;
        s.set(x, 43, 36, FANG);
        s.set(x, 43, 35, FANG);
        s.set(x, 42, 36, FANG);
        s.set((cx + side * 0.6).round() as i32, 46, 37, MAW); // nostrils
    }
    // The long tongue lolling out and down over his chest.
    limb(&mut s, &[(v(cx, 43.0, 34.5), 1.3), (v(cx, 41.5, 37.0), 1.1), (v(cx + 0.6, 38.5, 38.0), 0.9), (v(cx, 35.0, 38.2), 0.7)], TONGUE);
    ball(&mut s, v(cx, 49.8, 32.0), v(4.4, 1.1, 1.6), HIDE_D); // a heavy brow
    for e in KRAMPUS_EYES {
        s.cuboid(e[0].round() as i32 - 1, e[1] as i32, 33, e[0].round() as i32, e[1] as i32, 34, EYE);
    }
    for side in [-1.0f32, 1.0] {
        // Goat's ears sticking out.
        limb(&mut s, &[(v(cx + side * 4.2, 48.5, 27.0), 1.4), (v(cx + side * 6.8, 49.8, 26.0), 0.9), (v(cx + side * 8.8, 50.8, 25.0), 0.4)], HIDE_D);
        // Great horns: up and back, curling round and forward at the tips, ridged along their length.
        let k = |dx: f32, y: f32, z: f32, r: f32| (v(cx + side * dx, y, z), r);
        let knots = [k(2.6, 51.0, 28.5, 2.1), k(4.8, 55.0, 26.5, 1.9), k(7.8, 58.5, 23.5, 1.6), k(11.0, 60.5, 21.5, 1.35), k(14.0, 60.0, 21.0, 1.1), k(16.2, 57.0, 22.4, 0.9), k(16.5, 54.0, 24.8, 0.65), k(15.2, 52.4, 26.8, 0.45)];
        let mut run = 0.0;
        for wk in knots.windows(2) {
            let len = wk[0].0.distance(wk[1].0);
            let c = if run > 17.0 { HORN_TIP } else if ((run / 1.4) as i32) % 2 == 0 { HORN } else { HORN_D };
            s.capsule((wk[0].0.x, wk[0].0.y, wk[0].0.z), (wk[1].0.x, wk[1].0.y, wk[1].0.z), wk[0].1, wk[1].1, c);
            run += len;
        }
    }
    limb(&mut s, &[(v(cx, 41.0, 31.0), 1.8), (v(cx, 38.0, 30.6), 1.1), (v(cx, 36.0, 29.8), 0.5)], FUR_L); // a goatee

    // ---- shaggy fur over it all, blacker in the hollows
    s.paint(|x, y, z, c| {
        if c != FUR {
            return None;
        }
        let n = nz(x, y / 2, z);
        Some(if n < 0.25 { FUR_D } else if n > 0.86 { FUR_L } else { FUR })
    });
    shag(&mut s, (w, h, d), FUR, FUR_D, 0.24);
    shag(&mut s, (w, h, d), FUR_D, FUR, 0.12);
    s.finish(0.04)
}

/// Krampus's joints: head and horns, both arms from the shoulder, both legs from the hip.
pub const KRAMPUS_RIG: &[PartDef] = &[
    PartDef { name: "head", boxes: &[([0, 44, 15], [45, 63, 41]), ([15, 34, 29], [31, 43, 41])], pivot: [23.0, 41.0, 24.0] },
    PartDef { name: "armR", boxes: &[([0, 14, 17], [13, 42, 41])], pivot: [12.5, 40.5, 22.0] },
    PartDef { name: "armL", boxes: &[([33, 14, 17], [45, 42, 41])], pivot: [33.5, 40.5, 22.0] },
    PartDef { name: "legR", boxes: &[([12, 0, 8], [22, 21, 32])], pivot: [18.0, 22.0, 19.0] },
    PartDef { name: "legL", boxes: &[([24, 0, 8], [34, 21, 32])], pivot: [28.0, 22.0, 19.0] },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::voxels::build_rig;

    #[test]
    fn the_cast_is_cut_into_its_parts() {
        let models: [(&str, Collected, &[PartDef]); 7] = [
            ("kid 0", kid(0), KID_RIG),
            ("kid 1", kid(1), KID_RIG),
            ("kid 2", kid(2), KID_RIG),
            ("kid 3", kid(3), KID_RIG),
            ("old man", old_man(), OLD_MAN_RIG),
            ("dog", dog(), DOG_RIG),
            ("krampus", krampus_shade(), KRAMPUS_RIG),
        ];
        for (name, m, defs) in models {
            let rig = build_rig(&m, defs, 0.025);
            assert_eq!(rig.parts.len(), defs.len() + 1, "{name}");
            let total: usize = rig.parts.iter().map(|p| p.cubes.len()).sum();
            assert_eq!(total, m.voxels.len(), "{name}: every voxel in exactly one part");
            for p in &rig.parts {
                assert!(!p.cubes.is_empty(), "{name}: part {} is empty", p.name);
            }
        }
    }
}
