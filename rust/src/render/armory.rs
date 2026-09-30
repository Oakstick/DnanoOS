//! The armory in 3D: every weapon a hero can hold and every shield he can
//! sling on his back, sculpted from its form and colour in the grid of the
//! detailed hero models (60 wide, 92 tall, 36 deep; the right fist round
//! x 48..55, y 26..33; blades rise along y and face +z). Weapons are laid out
//! in units of two voxels (a 30 by 46 by 18 plan), so the blocky parts are
//! written simply, while blades, curves and edges are traced at the full
//! resolution. Weapons are models of their own, layered onto the sword arm
//! (and, for pairs, the shield arm too); shields are carved into the body's back.

use super::characters::{mat, Sculpt};
use super::voxels::Collected;
use crate::logic::heroes::HeroKind;
use crate::logic::progression::Form;

/// The plan's size, in units of two voxels.
const W: i32 = 30;
const H: i32 = 46;
const D: i32 = 18;
/// Voxels to a unit of the plan.
const K: i32 = 2;

const HAFT: u32 = 0x6b3f1d | mat::WOOD;
const HAFT_D: u32 = 0x4a2a12 | mat::WOOD;
const WRAP: u32 = 0x3a2418 | mat::LEATHER;
const WRAP_L: u32 = 0xb89a68 | mat::LEATHER;
const STEEL: u32 = 0xc8d0da;
const IRON: u32 = 0x5a616b | mat::METAL;
const GOLD: u32 = 0xf2c53d | mat::GOLD;

fn mix(a: u32, b: u32, t: f32) -> u32 {
    let ch = |s: u32| {
        let (x, y) = (((a >> s) & 255) as f32, ((b >> s) & 255) as f32);
        ((x + (y - x) * t).round().clamp(0.0, 255.0) as u32) << s
    };
    ch(16) | ch(8) | ch(0)
}

/// Metal tinted with the weapon's colour: its body, a dark shade and its bright edge.
fn metal(col: u32) -> (u32, u32, u32) {
    let body = mix(STEEL, col, 0.45);
    (body | mat::METAL, mix(body, 0x000000, 0.35) | mat::METAL, mix(col, 0xffffff, 0.55) | mat::METAL)
}

/// Draws into one hand: the right, or (mirrored) the left for the second of a pair.
/// Coordinates are in plan units unless a method says it works in voxels.
struct Hand<'a> {
    s: &'a mut Sculpt,
    left: bool,
}

impl Hand<'_> {
    /// One voxel, in voxels.
    fn fine(&mut self, x: i32, y: i32, z: i32, c: u32) {
        let x = if self.left { W * K - 1 - x } else { x };
        self.s.set(x, y, z, c);
    }
    /// One plan unit: a block of two voxels a side.
    fn set(&mut self, x: i32, y: i32, z: i32, c: u32) {
        self.cuboid(x, y, z, x, y, z, c);
    }
    #[allow(clippy::too_many_arguments)]
    fn cuboid(&mut self, x0: i32, y0: i32, z0: i32, x1: i32, y1: i32, z1: i32, c: u32) {
        for y in y0 * K..(y1 + 1) * K {
            for z in z0 * K..(z1 + 1) * K {
                for x in x0 * K..(x1 + 1) * K {
                    self.fine(x, y, z, c);
                }
            }
        }
    }
    /// The haft or grip in the fist, from y0 to y1: rounded off at its corners.
    fn grip(&mut self, y0: i32, y1: i32, c: u32) {
        for y in y0 * K..(y1 + 1) * K {
            for z in 16..=19 {
                for x in 52..=55 {
                    let corner = (x == 52 || x == 55) && (z == 16 || z == 19);
                    if !corner {
                        self.fine(x, y, z, c);
                    }
                }
            }
        }
    }
    /**
     * A blade rising from y0 to y1 (plan units): its centre line (in z) and
     * half-width at each height, traced voxel by voxel. It is ground thinner
     * toward the edge, which is a bright line along the front (and the tip).
     */
    fn blade(&mut self, y0: i32, y1: i32, center: impl Fn(f32) -> f32, half: impl Fn(f32) -> f32, body: u32, edge: u32) {
        let (fy0, fy1) = (y0 * K, (y1 + 1) * K - 1);
        for y in fy0..=fy1 {
            let t = (y - fy0) as f32 / (fy1 - fy0).max(1) as f32;
            let (c, h) = (center(t) * K as f32 + 0.5, half(t) * K as f32 + 0.5);
            let (z0, z1) = ((c - h).round() as i32, (c + h).round() as i32);
            for z in z0..=z1 {
                let bevel = z1 - z < 2 || z - z0 < 1;
                let col = if z == z1 || y == fy1 { edge } else { body };
                let xs = if bevel { 53..=54 } else { 52..=55 };
                for x in xs {
                    self.fine(x, y, z, col);
                }
            }
        }
    }
}

/// Tapers a blade's last `n` steps of its length to a point.
fn taper(t: f32, w: f32, from: f32) -> f32 {
    if t < from {
        w
    } else {
        w * (1.0 - (t - from) / (1.0 - from)).max(0.12)
    }
}

/// One weapon of the form in one hand.
fn weapon(h: &mut Hand, form: Form, col: u32) {
    let (body, dark, edge) = metal(col);
    match form {
        Form::Knife => {
            h.grip(10, 17, HAFT);
            h.cuboid(26, 9, 8, 27, 9, 9, dark);
            h.cuboid(25, 18, 6, 28, 18, 11, dark);
            h.blade(19, 29, |_| 8.5, |t| taper(t, 1.5, 0.6), body, edge);
        }
        Form::Sword => {
            for y in 9..=17 {
                h.grip(y, y, if y % 2 == 0 { WRAP } else { WRAP_L });
            }
            h.cuboid(25, 7, 7, 28, 8, 10, dark);
            h.cuboid(25, 18, 5, 28, 18, 12, dark);
            h.cuboid(26, 18, 4, 27, 18, 13, GOLD);
            h.blade(19, 41, |_| 8.5, |t| taper(t, 1.6, 0.8), body, edge);
            for y in 20..=36 {
                h.cuboid(26, y, 8, 27, y, 9, dark);
            }
        }
        Form::Saber => {
            for y in 9..=17 {
                h.grip(y, y, if y % 3 == 0 { GOLD } else { WRAP });
            }
            h.cuboid(25, 18, 6, 28, 18, 11, GOLD);
            h.blade(19, 39, |t| 8.5 + 3.6 * t * t, |t| taper(t, 1.3, 0.75), body, edge);
        }
        Form::Katana => {
            for y in 8..=17 {
                h.grip(y, y, if y % 2 == 0 { 0x1c1a1e | mat::LEATHER } else { 0xeeeae0 | mat::CLOTH });
            }
            for (dx, dz) in [
                (0, 0),
                (1, 0),
                (-1, 0),
                (2, 0),
                (0, 1),
                (1, 1),
                (0, -1),
                (1, -1),
                (0, 2),
                (1, 2),
                (0, -2),
                (1, -2),
                (-1, 1),
                (2, 1),
                (-1, -1),
                (2, -1),
            ] {
                h.cuboid(26 + dx, 18, 8 + dz, 26 + dx, 18, 9 + dz, dark);
            }
            h.blade(19, 43, |t| 8.5 + 2.0 * t * t, |t| taper(t, 1.0, 0.85), body, edge);
        }
        Form::Axe => {
            h.grip(6, 36, HAFT);
            let profile = [(10, 29, 36), (11, 28, 36), (12, 27, 37), (13, 26, 37), (14, 25, 37), (15, 24, 37), (16, 24, 37)];
            for (i, &(z, y0, y1)) in profile.iter().enumerate() {
                for y in y0..=y1 {
                    h.cuboid(26, y, z, 27, y, z, if i == profile.len() - 1 { edge } else { body });
                }
            }
            h.cuboid(25, 31, 7, 28, 35, 10, dark);
            for z in 4..=6 {
                h.cuboid(26, 32, z, 27, 33, z, dark);
            }
            // Runes cut into the blade.
            for (y, z) in [(30, 12), (32, 13), (34, 12), (33, 14)] {
                h.set(26, y, z, edge);
            }
        }
        Form::TwinAxes => weapon(h, Form::Axe, col),
        Form::TwinBlades => weapon(h, Form::Saber, col),
        Form::Wand => {
            h.grip(5, 21, 0x3a2a3a | mat::WOOD);
            for y in [8, 20] {
                h.cuboid(25, y, 7, 28, y, 10, GOLD);
            }
            h.blade(22, 36, |_| 8.5, |t| taper(t, 1.1, 0.7), body, edge);
            // A flame-crystal at the tip.
            for y in 37..=44 {
                let r = 2.2 * (1.0 - ((y as f32 - 40.0) / 4.2).powi(2)).max(0.0);
                let ri = r.round() as i32;
                for z in (8 - ri)..=(9 + ri) {
                    h.cuboid(26 - (ri / 2), y, z, 27 + (ri / 2), y, z, if y > 41 { mix(col, 0xffffff, 0.6) } else { col });
                }
            }
        }
        Form::Spear => {
            h.grip(2, 40, HAFT);
            for y in [12, 17, 39] {
                h.cuboid(25, y, 7, 28, y, 10, GOLD);
            }
            let halves = [0.9f32, 1.7, 1.9, 1.6, 1.0, 0.4];
            for (i, &w) in halves.iter().enumerate() {
                let y = 40 + i as i32;
                let (z0, z1) = ((8.5 - w).round() as i32, (8.5 + w).round() as i32);
                for z in z0..=z1 {
                    h.cuboid(26, y, z, 27, y, z, if z == z1 || i == halves.len() - 1 { edge } else { body });
                }
            }
        }
        Form::Hammer => {
            for y in 8..=24 {
                h.grip(y, y, if (10..=18).contains(&y) && y % 2 == 0 { WRAP } else { HAFT_D });
            }
            h.cuboid(26, 6, 8, 27, 7, 9, dark);
            h.cuboid(23, 25, 4, 29, 31, 13, body);
            h.cuboid(23, 25, 4, 29, 25, 13, dark);
            h.cuboid(23, 31, 4, 29, 31, 13, dark);
            for (y, z) in [(27, 4), (28, 4), (29, 4), (28, 5), (27, 13), (28, 13), (29, 13), (28, 12)] {
                h.cuboid(23, y, z, 29, y, z, edge);
            }
            for y in 26..=30 {
                for z in 5..=12 {
                    if (y + z) % 3 == 0 {
                        h.set(23, y, z, edge);
                        h.set(29, y, z, edge);
                    }
                }
            }
        }
        Form::Mace => {
            h.grip(6, 30, HAFT);
            h.cuboid(25, 29, 7, 28, 30, 10, GOLD);
            for y in 31..=37 {
                let r = 3.2 * (1.0 - ((y as f32 - 34.0) / 3.6).powi(2)).max(0.0);
                let ri = r.round() as i32;
                h.cuboid(27 - ri, y, 9 - ri, 26 + ri, y, 8 + ri, body);
            }
            for (x0, x1, z0, z1) in [(26, 27, 3, 4), (26, 27, 13, 14), (22, 22, 8, 9), (31, 31, 8, 9)] {
                h.cuboid(x0.min(29), 32, z0, x1.min(29), 36, z1, edge);
            }
            h.cuboid(26, 38, 8, 27, 39, 9, edge);
        }
        Form::Bow => {
            for y in 3..=39 {
                let t = (y as f32 - 21.0) / 18.0;
                let z = (8.5 + 4.0 * (1.0 - t * t)).round() as i32;
                h.cuboid(26, y, z, 27, y, z, if (y - 21).abs() < 3 { WRAP } else { mix(col, 0x000000, 0.1) | mat::GOLD });
            }
            for y in 3..=39 {
                h.set(26, y, 8, 0xe8e0d0 | mat::CLOTH);
            }
        }
        Form::Sickle => {
            h.grip(8, 24, HAFT);
            for i in 0..=18 {
                let a = (90.0 - 150.0 * i as f32 / 18.0).to_radians();
                let (y, z) = ((25.0 + 4.5 * a.sin()).round() as i32, (12.0 + 4.5 * a.cos()).round() as i32);
                h.cuboid(26, y, z, 27, y, z, if i > 12 { edge } else { body });
            }
            for y in 1..=7 {
                h.set(26 + (y % 2), y, 9, IRON);
            }
        }
        Form::Gun => {
            // A flintlock pistol held muzzle up: a curved walnut stock in the fist, a brass-bound
            // lock, a long iron barrel.
            for y in 6..=17 {
                let z0 = if y < 10 { 6 + (10 - y) / 2 } else { 8 };
                h.cuboid(26, y, z0.min(8), 27, y, 9, HAFT);
            }
            h.cuboid(26, 5, 5, 27, 6, 8, GOLD);
            h.cuboid(25, 16, 6, 28, 19, 11, HAFT_D);
            h.cuboid(25, 17, 10, 28, 18, 12, GOLD);
            h.cuboid(28, 18, 11, 28, 20, 11, IRON);
            h.set(28, 21, 10, IRON);
            for y in 20..=34 {
                h.cuboid(26, y, 8, 27, y, 9, if y % 5 == 0 { GOLD } else { body });
            }
            h.cuboid(26, 35, 8, 27, 35, 9, edge);
            h.cuboid(26, 21, 7, 26, 30, 7, WRAP);
        }
        Form::Forked => {
            // A double-pointed greatsword: a long grip, broad guard, a broad blade split at the tip.
            for y in 7..=17 {
                h.grip(y, y, if y % 2 == 0 { WRAP } else { GOLD });
            }
            h.cuboid(25, 6, 7, 28, 6, 10, GOLD);
            h.cuboid(25, 18, 4, 28, 18, 13, GOLD);
            h.blade(19, 38, |_| 8.5, |_| 2.2, body, edge);
            for (y, dz) in [(39, 0), (40, 0), (41, 1), (42, 1), (43, 2)] {
                h.cuboid(26, y, 6 - dz, 27, y, 7 - dz, edge);
                h.cuboid(26, y, 10 + dz, 27, y, 11 + dz, edge);
            }
            for y in 20..=36 {
                h.cuboid(26, y, 8, 27, y, 9, dark);
            }
        }
        // Shields, brews and runes are not held.
        _ => {}
    }
}

/// How far a hero's fist is from Ragnar's (right hand, or left), in voxels of his own grid.
fn grip_shift(hero: HeroKind, left: bool) -> (i32, i32, i32) {
    match (hero, left) {
        (HeroKind::Ragnar, _) => (0, 0, 0),
        (_, false) => (22, -4, 2),
        (_, true) => (14, -4, 2),
    }
}

/**
 * A weapon of `form` and `col` in `hero`'s hand, in his grid: sculpted in
 * Ragnar's and carried over to the other heroes' fists. Each hero's fist is
 * his own, sculpted round where the grip goes.
 */
pub fn weapon_for(hero: HeroKind, form: Form, col: u32, left: bool) -> Collected {
    let mut c = hero_weapon(form, col, left);
    let (dx, dy, dz) = grip_shift(hero, left);
    for v in c.voxels.iter_mut() {
        // Held at the ready: leaning forward from the fist, so the blade clears the arm.
        let lean = ((v.y - FIST_Y) as f32 * LEAN).round() as i32;
        v.x += dx;
        v.y += dy;
        v.z += dz + lean;
    }
    c
}

/// The height of the middle of Ragnar's fist, in voxels, and how far a held weapon
/// leans forward for each voxel it rises above it.
const FIST_Y: i32 = 30;
const LEAN: f32 = 0.42;

/// A weapon of `form` and `col` as a model of its own, in Ragnar's grid: in his right
/// hand, or the second of a pair in his left.
pub fn hero_weapon(form: Form, col: u32, left: bool) -> Collected {
    let mut s = Sculpt::new(W * K, H * K, D * K);
    weapon(&mut Hand { s: &mut s, left }, form, col);
    s.finish(0.03)
}

/// Does this form come as a pair, one in each hand?
pub fn paired(form: Form) -> bool {
    matches!(form, Form::TwinAxes | Form::TwinBlades)
}

/**
 * The shield of `form` and `col` slung on his back: its face at z 0..1, rim and
 * boss standing out; wood for the lower tiers, metal higher up.
 */
pub fn back_shield(s: &mut Sculpt, form: Form, col: u32, tier: u8) {
    back_shield_at(s, form, col, tier, (0, 0, 0));
}

/**
 * The same shield carved `at` voxels away from where Ragnar's hangs (for
 * another hero's back), traced voxel by voxel: its face two voxels thick at
 * z 2..3, the rim and boss standing out to z 0.
 */
pub fn back_shield_at(s: &mut Sculpt, form: Form, col: u32, tier: u8, at: (i32, i32, i32)) {
    let (ox, oy, oz) = at;
    let stuff = if tier <= 4 && !matches!(form, Form::Mirror) { mat::WOOD } else { mat::METAL };
    let face = col | stuff;
    let face_d = mix(col, 0x000000, 0.28) | stuff;
    let face_l = mix(col, 0xffffff, 0.35) | stuff;
    let rim = if tier >= 8 { GOLD } else { IRON };
    let rim_l = if tier >= 8 { mix(0xf2c53d, 0xffffff, 0.4) | mat::GOLD } else { 0x8a929c | mat::METAL };
    let rivet = 0xe4e9ee | mat::METAL;
    // Its middle and scale in voxels (the plan's shield is 18 units across, centred at 14.5, 21).
    let (cx, cy) = (29.5f32, 42.5f32);
    for y in 16..=69 {
        for x in 6..=53 {
            // Distance from the middle, in plan units.
            let (dx, dy) = ((x as f32 - cx) / K as f32, (y as f32 - cy) / K as f32);
            let r = (dx * dx + dy * dy).sqrt();
            let a = dy.atan2(dx);
            let (c, raised) = match form {
                Form::Round | Form::Spiral | Form::Mirror => {
                    if r > 9.0 {
                        continue;
                    }
                    let edge = r > 8.1;
                    let boss = r < 2.4;
                    let c = if edge {
                        let studded = (a / std::f32::consts::TAU * 24.0).rem_euclid(1.0) < 0.2 && r > 8.4;
                        if form == Form::Mirror {
                            GOLD
                        } else if studded {
                            rivet
                        } else if r > 8.7 {
                            rim
                        } else {
                            rim_l
                        }
                    } else if boss {
                        if form == Form::Mirror {
                            face_l
                        } else if r < 1.1 {
                            rim_l
                        } else {
                            mix(STEEL, col, 0.3) | mat::METAL
                        }
                    } else {
                        match form {
                            Form::Spiral => {
                                if (a / std::f32::consts::TAU + r / 3.2).rem_euclid(1.0) < 0.3 {
                                    face_d
                                } else {
                                    face
                                }
                            }
                            Form::Mirror => {
                                if (dx + dy).abs() < 0.9 && r < 6.5 {
                                    0xffffff | mat::METAL
                                } else {
                                    face
                                }
                            }
                            _ => {
                                // Planks behind a riveted iron ring.
                                if (r - 5.4).abs() < 0.3 {
                                    if (a / std::f32::consts::TAU * 16.0).rem_euclid(1.0) < 0.25 {
                                        rivet
                                    } else {
                                        rim
                                    }
                                } else if (x - 6) % 8 == 0 {
                                    face_d
                                } else {
                                    face
                                }
                            }
                        }
                    };
                    (c, edge || boss)
                }
                Form::Kite => {
                    let half = if dy >= 1.0 { 9.0 } else { 9.0 * (dy + 13.0) / 14.0 + 0.8 };
                    if dx.abs() > half || dy > 12.5 {
                        continue;
                    }
                    let edge = dx.abs() > half - 1.1 || dy > 11.6;
                    let c = if edge {
                        if (x + y) % 5 == 0 {
                            rivet
                        } else {
                            rim
                        }
                    } else if dx.abs() < 0.6 || (dy - 5.0).abs() < 0.5 {
                        face_l
                    } else if (dx >= 0.0) != (dy >= 5.0) {
                        face
                    } else {
                        face_d
                    };
                    (c, edge)
                }
                _ => {
                    // A standing shield: planks between iron bands, a crest in the middle.
                    if !(-9.5..=9.5).contains(&dx) || !(-13.0..=13.0).contains(&dy) {
                        continue;
                    }
                    let edge = dx.abs() > 8.8 || dy.abs() > 12.3;
                    let band = (dy + 9.0).abs() < 0.5 || (dy - 9.0).abs() < 0.5;
                    let crest = r < 2.6;
                    let c = if edge || band {
                        if (x + y) % 6 == 0 {
                            rivet
                        } else {
                            rim
                        }
                    } else if crest {
                        if r < 1.4 {
                            rim_l
                        } else {
                            face_l
                        }
                    } else if (x - 10) % 10 == 0 {
                        face_d
                    } else {
                        face
                    };
                    (c, edge || crest)
                }
            };
            for z in 2..=3 {
                s.set(x + ox, y + oy, z + oz, c);
            }
            if raised {
                for z in 0..=1 {
                    s.set(x + ox, y + oy, z + oz, c);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::heroes::ROSTER;
    use crate::logic::progression::{node, Tree, TIERS};

    #[test]
    fn every_weapon_and_shield_of_every_hero_has_a_model() {
        for hero in ROSTER {
            for tier in 1..=TIERS {
                let n = node(hero, Tree::Armory, tier);
                let w = hero_weapon(n.form, n.color, false);
                assert!(w.voxels.len() > 60, "{} has too few voxels: {}", n.name, w.voxels.len());
                let n = node(hero, Tree::Shield, tier);
                let mut s = Sculpt::new(W * K, H * K, D * K);
                back_shield(&mut s, n.form, n.color, tier);
                assert!(s.finish(0.0).voxels.len() > 150, "{} is too small", n.name);
            }
        }
    }
}
