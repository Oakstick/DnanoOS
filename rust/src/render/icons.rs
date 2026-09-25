//! Pixel-art icons for the HUD and the blacksmith: every weapon at every
//! level, the four shields and the potions. Drawn procedurally on a 16x16
//! canvas with a dark outline, the way hand-made 16-bit item icons look.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::voxels::Sprite;
use crate::logic::gear::{potion, Line, ShieldKind};
use crate::logic::items::RangedKind;
use crate::logic::types::PotionKind;

const N: i32 = 16;
const OUTLINE: u32 = 0x160e0a;

struct Canvas {
    px: Vec<Option<u32>>,
}

impl Canvas {
    fn new() -> Self {
        Self { px: vec![None; (N * N) as usize] }
    }
    fn set(&mut self, x: i32, y: i32, c: u32) {
        if (0..N).contains(&x) && (0..N).contains(&y) {
            self.px[(y * N + x) as usize] = Some(c);
        }
    }
    fn get(&self, x: i32, y: i32) -> Option<u32> {
        if (0..N).contains(&x) && (0..N).contains(&y) {
            self.px[(y * N + x) as usize]
        } else {
            None
        }
    }
    fn rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: u32) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.set(x, y, c);
            }
        }
    }
    /// A line of single pixels from a to b.
    fn line(&mut self, a: (i32, i32), b: (i32, i32), c: u32) {
        let steps = (b.0 - a.0).abs().max((b.1 - a.1).abs()).max(1);
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = a.0 as f32 + (b.0 - a.0) as f32 * t;
            let y = a.1 as f32 + (b.1 - a.1) as f32 * t;
            self.set(x.round() as i32, y.round() as i32, c);
        }
    }
    fn disc(&mut self, cx: f32, cy: f32, r: f32, c: u32) {
        for y in 0..N {
            for x in 0..N {
                let (dx, dy) = (x as f32 - cx, y as f32 - cy);
                if dx * dx + dy * dy <= r * r {
                    self.set(x, y, c);
                }
            }
        }
    }
    /// Paints `f`'s color wherever it returns one, over filled pixels only.
    fn paint(&mut self, f: impl Fn(i32, i32, u32) -> Option<u32>) {
        for y in 0..N {
            for x in 0..N {
                if let Some(c) = self.get(x, y) {
                    if let Some(n) = f(x, y, c) {
                        self.set(x, y, n);
                    }
                }
            }
        }
    }
    /// A one-pixel dark outline around everything drawn so far.
    fn outlined(mut self) -> Sprite {
        let filled: Vec<bool> = self.px.iter().map(|p| p.is_some()).collect();
        for y in 0..N {
            for x in 0..N {
                if filled[(y * N + x) as usize] {
                    continue;
                }
                let touches = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| {
                    let (nx, ny) = (x + dx, y + dy);
                    (0..N).contains(&nx) && (0..N).contains(&ny) && filled[(ny * N + nx) as usize]
                });
                if touches {
                    self.set(x, y, OUTLINE);
                }
            }
        }
        let pixels = (0..N).map(|y| (0..N).map(|x| self.get(x, y)).collect()).collect();
        Sprite { width: N, height: N, pixels }
    }
}

/// Everything that has an icon.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Icon {
    Weapon(Line, u8),
    /// A shrine's ammo weapon.
    Pickup(RangedKind),
    Shield(ShieldKind),
    Potion(PotionKind),
    Coin,
    Lock,
    Bomb,
}

pub fn icon(i: Icon) -> &'static Sprite {
    static CACHE: OnceLock<HashMap<Icon, Sprite>> = OnceLock::new();
    let map = CACHE.get_or_init(|| {
        let mut m = HashMap::new();
        for line in crate::logic::gear::LINES {
            for level in 1..=3 {
                m.insert(Icon::Weapon(line, level), weapon(line, level));
            }
        }
        for k in crate::logic::items::WEAPON_ORDER {
            m.insert(Icon::Pickup(k), pickup(k));
        }
        for k in crate::logic::gear::SHIELDS {
            m.insert(Icon::Shield(k), shield(k));
        }
        for k in crate::logic::gear::POTIONS {
            m.insert(Icon::Potion(k), bottle(k));
        }
        m.insert(Icon::Coin, coin());
        m.insert(Icon::Lock, lock());
        m.insert(Icon::Bomb, bomb());
        m
    });
    &map[&i]
}

// ------------------------------------------------------------------ weapons

const STEEL: u32 = 0xc8d0da;
const STEEL_D: u32 = 0x8a929e;
const EDGE: u32 = 0xf4f8fc;
const WOOD: u32 = 0x8a5a2e;
const WOOD_L: u32 = 0xb07a44;
const GOLD: u32 = 0xffd84a;
const GOLD_D: u32 = 0xc8942a;
const FIRE: u32 = 0xff7a2a;
const FIRE_L: u32 = 0xffd23a;
const RED: u32 = 0xc8262e;

fn weapon(line: Line, level: u8) -> Sprite {
    let mut c = Canvas::new();
    match (line, level) {
        (Line::Axe, 1) => {
            c.line((3, 14), (11, 3), WOOD);
            c.line((4, 14), (12, 3), WOOD_L);
            // Bearded blade on the upper right.
            c.rect(9, 2, 12, 7, STEEL);
            c.rect(12, 1, 13, 8, STEEL);
            c.line((14, 1), (14, 8), EDGE);
            c.rect(9, 6, 10, 8, STEEL_D);
        }
        (Line::Axe, 2) => {
            c.line((2, 15), (11, 3), WOOD);
            c.line((3, 15), (12, 3), WOOD_L);
            for y in [10, 12] {
                c.line((5, y), (7, y - 2), RED);
            }
            c.rect(8, 1, 12, 8, STEEL_D);
            c.rect(12, 0, 14, 9, STEEL_D);
            c.line((15, 0), (15, 9), EDGE);
            c.rect(6, 3, 8, 4, STEEL_D); // back spike
            c.set(5, 3, STEEL);
        }
        (Line::Axe, _) => {
            c.line((2, 15), (8, 5), 0x3e2412);
            c.line((3, 15), (9, 5), 0x5a3418);
            c.set(4, 12, GOLD);
            c.set(6, 9, GOLD);
            // Two blades, gilded edges.
            c.disc(4.5, 4.0, 3.6, STEEL);
            c.disc(12.0, 4.0, 3.6, STEEL);
            c.rect(6, 2, 10, 6, STEEL_D);
            c.paint(|x, y, col| {
                let (a, b) = ((x as f32 - 4.5).hypot(y as f32 - 4.0), (x as f32 - 12.0).hypot(y as f32 - 4.0));
                if col == STEEL && (a > 2.8 && x < 5 || b > 2.8 && x > 11) {
                    Some(GOLD)
                } else {
                    None
                }
            });
            c.set(8, 1, GOLD);
        }
        (Line::Arrow, 1) => arrow(&mut c, 0, STEEL, 0xd8d0c0, RED),
        (Line::Arrow, 2) => {
            for off in [-3, 0, 3] {
                arrow(&mut c, off, FIRE, WOOD_L, FIRE);
            }
            for (x, y) in [(13, 1), (14, 3), (12, 0), (15, 2)] {
                c.set(x, y, FIRE_L);
            }
        }
        (Line::Arrow, _) => {
            // A golden arrow in a burst of light.
            for (a, b) in [((11, 0), (11, 8)), ((7, 4), (15, 4)), ((8, 1), (14, 7)), ((14, 1), (8, 7))] {
                c.line(a, b, 0xfff6c0);
            }
            arrow(&mut c, 0, GOLD, GOLD_D, 0xffffff);
            c.disc(11.5, 4.0, 1.6, 0xffffff);
        }
        (Line::Knife, 1) => knife(&mut c, (3, 13), STEEL),
        (Line::Knife, 2) => {
            knife(&mut c, (1, 12), STEEL);
            knife(&mut c, (5, 15), EDGE);
        }
        (Line::Knife, _) => {
            // Three knives fanned out, trailing blue.
            for (dx, dy) in [(-2, 2), (1, 0), (4, -2)] {
                c.line((1 + dx, 12 + dy), (4 + dx, 9 + dy), 0x6aa8ff);
            }
            knife(&mut c, (1, 14), 0x9ad1ff);
            knife(&mut c, (4, 12), 0x9ad1ff);
            knife(&mut c, (7, 10), 0x9ad1ff);
        }
        (Line::Boomerang, level) => {
            let (col, dark) = match level {
                1 => (0xc98f4a, 0x8a5a2e),
                2 => (0xb8c8d8, 0x6e7888),
                _ => (FIRE, 0xb83a1a),
            };
            for i in 0..8 {
                c.rect(2 + i, 3 + i, 3 + i, 4 + i, col);
                c.rect(9 + i.min(5), 10 - i, 10 + i.min(5), 11 - i, col);
            }
            c.paint(|x, y, _| if (x + y) % 5 == 0 { Some(dark) } else { None });
            if level == 3 {
                for (x, y) in [(3, 1), (5, 2), (14, 3), (12, 2), (1, 6)] {
                    c.set(x, y, FIRE_L);
                }
            }
        }
    }
    c.outlined()
}

/// A diagonal arrow, shifted `off` pixels across its line.
fn arrow(c: &mut Canvas, off: i32, head: u32, shaft: u32, fletch: u32) {
    let (ox, oy) = (off, off);
    c.line((2 + ox, 13 + oy), (11 + ox, 4 + oy), shaft);
    c.rect(11 + ox, 2 + oy, 12 + ox, 5 + oy, head);
    c.rect(10 + ox, 3 + oy, 13 + ox, 4 + oy, head);
    c.set(13 + ox, 2 + oy, head);
    c.set(2 + ox, 11 + oy, fletch);
    c.set(4 + ox, 13 + oy, fletch);
    c.set(3 + ox, 11 + oy, fletch);
    c.set(4 + ox, 12 + oy, fletch);
}

/// A throwing knife with its handle at `base`, blade up and to the right.
fn knife(c: &mut Canvas, base: (i32, i32), blade: u32) {
    let (x, y) = base;
    c.line((x, y), (x + 2, y - 2), WOOD);
    c.set(x + 3, y - 3, GOLD_D);
    c.set(x + 2, y - 4, GOLD_D);
    c.set(x + 4, y - 2, GOLD_D);
    c.line((x + 3, y - 4), (x + 7, y - 8), blade);
    c.line((x + 4, y - 4), (x + 8, y - 8), EDGE);
}

fn pickup(kind: RangedKind) -> Sprite {
    match kind {
        RangedKind::Arrow => weapon(Line::Arrow, 1),
        RangedKind::Boomerang => weapon(Line::Boomerang, 1),
        _ => {
            let mut c = Canvas::new();
            match kind {
                RangedKind::Double => {
                    arrow(&mut c, -2, STEEL, 0xd8d0c0, RED);
                    arrow(&mut c, 2, STEEL, 0xd8d0c0, RED);
                }
                RangedKind::Trishot => {
                    c.line((8, 14), (8, 3), 0xd8d0c0);
                    c.line((2, 8), (14, 8), 0xd8d0c0);
                    for (x, y) in [(8, 1), (1, 8), (15, 8)] {
                        c.disc(x as f32, y as f32, 1.3, 0x9ad1ff);
                    }
                }
                RangedKind::Fireball => {
                    c.disc(8.0, 8.5, 5.5, FIRE);
                    c.disc(7.0, 7.5, 3.2, FIRE_L);
                    c.disc(6.5, 7.0, 1.4, 0xffffff);
                }
                _ => {
                    // Crescent.
                    c.disc(8.0, 8.0, 6.0, 0x8fe7ff);
                    for y in 0..N {
                        for x in 0..N {
                            if (x as f32 - 10.5).hypot(y as f32 - 6.5) < 5.2 {
                                c.px[(y * N + x) as usize] = None;
                            }
                        }
                    }
                }
            }
            c.outlined()
        }
    }
}

// ------------------------------------------------------------------ shields

fn shield(kind: ShieldKind) -> Sprite {
    let mut c = Canvas::new();
    match kind {
        ShieldKind::Round | ShieldKind::IronRound => {
            c.disc(7.5, 7.5, 7.0, 0x5a616b);
            let iron = kind == ShieldKind::IronRound;
            c.disc(7.5, 7.5, 5.8, if iron { 0xb8c0cc } else { 0x9a6a3a });
            if !iron {
                c.paint(|x, _, col| if col == 0x9a6a3a && x % 3 == 0 { Some(0x7a522a) } else { None });
            } else {
                c.paint(|x, y, col| {
                    let r = (x as f32 - 7.5).hypot(y as f32 - 7.5);
                    if col == 0xb8c0cc && (r - 4.4).abs() < 0.5 && (x + y) % 3 == 0 {
                        Some(0xf4f8fc)
                    } else if col == 0xb8c0cc && x + y < 12 {
                        Some(0xd0d8e2)
                    } else {
                        None
                    }
                });
            }
            c.disc(7.5, 7.5, 1.8, if iron { GOLD } else { 0x8a929e });
            c.set(7, 7, 0xffffff);
        }
        ShieldKind::Kite | ShieldKind::Dreadnought => {
            let dread = kind == ShieldKind::Dreadnought;
            for y in 1..15 {
                let half = if y < 8 { 6.5 } else { 6.5 * (15 - y) as f32 / 7.0 };
                for x in 0..N {
                    let dx = (x as f32 - 7.5).abs();
                    if dx > half {
                        continue;
                    }
                    let rim = dx > half - 1.2 || y == 1;
                    let col = if dread {
                        if rim {
                            GOLD
                        } else if (x + y) % 4 == 0 && dx > half - 2.5 {
                            0xf4f8fc
                        } else if dx < 0.8 || y == 6 {
                            0x6e7888
                        } else {
                            0xa8b4c4
                        }
                    } else if rim {
                        0x5a616b
                    } else if (x < 8) != (y >= 7) {
                        0xd8262e
                    } else {
                        0xece0c0
                    };
                    c.set(x, y, col);
                }
            }
        }
    }
    c.outlined()
}

// ------------------------------------------------------------------ potions

fn bottle(kind: PotionKind) -> Sprite {
    let mut c = Canvas::new();
    let col = potion(kind).color;
    let dark = scale(col, 0.65);
    // Cork, neck, round belly, a glint.
    c.rect(6, 0, 9, 1, 0xa87a4a);
    c.rect(6, 2, 9, 4, 0xc8dce8);
    c.disc(7.5, 10.0, 5.2, 0xc8dce8);
    c.disc(7.5, 10.2, 4.4, col);
    c.rect(7, 3, 8, 5, col);
    c.paint(|x, y, p| if p == col && y > 11 && (x + y) % 2 == 0 { Some(dark) } else { None });
    c.set(5, 8, 0xffffff);
    c.set(5, 9, 0xffffff);
    // A symbol for what it does.
    let w = 0xffffff;
    match kind {
        PotionKind::Health => {
            c.rect(7, 8, 8, 12, w);
            c.rect(5, 9, 10, 10, w);
        }
        PotionKind::Regeneration => {
            c.line((6, 12), (9, 8), 0x2a8a2a);
            c.rect(8, 8, 9, 9, 0x2a8a2a);
        }
        PotionKind::Swiftness => {
            c.line((5, 11), (10, 11), w);
            c.line((6, 9), (11, 9), w);
        }
        PotionKind::NightVision | PotionKind::TrueSight => {
            c.rect(5, 9, 10, 11, w);
            c.rect(7, 9, 8, 11, 0x14081a);
            c.set(5, 9, col);
            c.set(10, 11, col);
        }
        PotionKind::Strength => {
            c.rect(6, 9, 9, 12, 0xffd0c0);
            c.rect(6, 8, 7, 8, 0xffd0c0);
        }
        PotionKind::IronSkin => {
            c.rect(6, 8, 9, 11, 0xe8eef4);
            c.rect(7, 12, 8, 12, 0xe8eef4);
        }
        PotionKind::Arcane => {
            c.set(7, 8, w);
            c.rect(6, 10, 9, 10, w);
            c.rect(7, 9, 8, 11, w);
            c.set(8, 12, w);
        }
        PotionKind::Lightning => {
            c.line((9, 7), (6, 10), FIRE_L);
            c.line((6, 10), (9, 10), FIRE_L);
            c.line((9, 10), (6, 13), FIRE_L);
        }
        PotionKind::Invulnerability => {
            c.set(7, 7, w);
            c.rect(5, 9, 10, 9, w);
            c.rect(6, 10, 9, 11, w);
            c.set(6, 12, w);
            c.set(9, 12, w);
        }
        PotionKind::Life => {
            c.rect(5, 8, 6, 9, w);
            c.rect(9, 8, 10, 9, w);
            c.rect(5, 10, 10, 10, w);
            c.rect(6, 11, 9, 11, w);
            c.rect(7, 12, 8, 12, w);
        }
    }
    c.outlined()
}

fn coin() -> Sprite {
    let mut c = Canvas::new();
    c.disc(7.5, 7.5, 7.0, GOLD_D);
    c.disc(7.0, 7.0, 5.8, GOLD);
    c.rect(6, 4, 8, 11, GOLD_D);
    c.rect(5, 5, 9, 5, GOLD_D);
    c.rect(5, 10, 9, 10, GOLD_D);
    c.set(4, 4, 0xfff6c0);
    c.outlined()
}

fn bomb() -> Sprite {
    let mut c = Canvas::new();
    c.disc(7.0, 9.0, 6.0, 0x2c3038);
    c.disc(5.5, 7.5, 2.4, 0x5a6270);
    c.disc(5.0, 7.0, 1.0, 0xb8c0cc);
    c.rect(1, 9, 13, 9, 0x6a707c);
    c.rect(6, 2, 8, 3, 0x6a707c);
    c.line((8, 1), (10, 0), 0xc8a060);
    c.set(11, 0, 0xff7a20);
    c.set(12, 1, 0xffd23a);
    c.outlined()
}

fn lock() -> Sprite {
    let mut c = Canvas::new();
    c.rect(4, 7, 11, 14, 0x8a929e);
    c.rect(5, 2, 6, 7, 0x6e7888);
    c.rect(9, 2, 10, 7, 0x6e7888);
    c.rect(5, 1, 10, 2, 0x6e7888);
    c.rect(7, 9, 8, 12, 0x2a2a30);
    c.outlined()
}

fn scale(c: u32, k: f32) -> u32 {
    let ch = |s: u32| ((((c >> s) & 255) as f32 * k).clamp(0.0, 255.0) as u32) << s;
    ch(16) | ch(8) | ch(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_item_has_a_distinct_icon() {
        let mut seen: Vec<&Vec<Vec<Option<u32>>>> = Vec::new();
        for line in crate::logic::gear::LINES {
            for level in 1..=3 {
                let s = icon(Icon::Weapon(line, level));
                assert!(s.pixels.iter().flatten().filter(|p| p.is_some()).count() > 20, "{line:?} {level} is nearly empty");
                assert!(!seen.contains(&&s.pixels), "{line:?} {level} looks like another icon");
                seen.push(&s.pixels);
            }
        }
        for k in crate::logic::gear::POTIONS {
            let s = icon(Icon::Potion(k));
            assert!(!seen.contains(&&s.pixels), "{k:?} looks like another icon");
            seen.push(&s.pixels);
        }
    }
}
