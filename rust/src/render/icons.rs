//! Pixel-art icons for the HUD and the old man's wares: every entry of every
//! hero's four trees (weapons, shields, elixirs and the rune stones of the
//! path), the shrine weapons, silver, rune fragments, keys and bombs. Drawn
//! procedurally on a 16x16 canvas with a dark outline, the way hand-made
//! 16-bit item icons look.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::voxels::Sprite;
use crate::logic::heroes::{HeroKind, ROSTER};
use crate::logic::items::RangedKind;
use crate::logic::progression::{self, Form, Tree, TIERS, TREES};
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
    /// Tier n of a hero's tree.
    Node(HeroKind, Tree, u8),
    /// A potion on a hero's belt (his own brews, or an extra life).
    Potion(HeroKind, PotionKind),
    /// A shrine's ammo weapon.
    Pickup(RangedKind),
    Coin,
    Rune,
    Lock,
    Bomb,
    Key,
}

pub fn icon(i: Icon) -> &'static Sprite {
    static CACHE: OnceLock<HashMap<Icon, Sprite>> = OnceLock::new();
    let map = CACHE.get_or_init(|| {
        let mut m = HashMap::new();
        for hero in ROSTER {
            for tree in TREES {
                for tier in 1..=TIERS {
                    let n = progression::node(hero, tree, tier);
                    m.insert(Icon::Node(hero, tree, tier), item(n.form, n.color));
                }
            }
            for kind in crate::logic::gear::ALL_POTIONS {
                let sprite = match progression::elixir_tier(kind) {
                    Some(tier) => {
                        let n = progression::node(hero, Tree::Elixir, tier);
                        item(n.form, n.color)
                    }
                    None => life(),
                };
                m.insert(Icon::Potion(hero, kind), sprite);
            }
        }
        for k in crate::logic::items::WEAPON_ORDER {
            m.insert(Icon::Pickup(k), pickup(k));
        }
        m.insert(Icon::Coin, coin());
        m.insert(Icon::Rune, rune_fragment());
        m.insert(Icon::Lock, lock());
        m.insert(Icon::Bomb, bomb());
        m.insert(Icon::Key, key());
        m
    });
    &map[&i]
}

// ------------------------------------------------------------------ weapons

const STEEL: u32 = 0xc8d0da;
const STEEL_D: u32 = 0x8a929e;
const WOOD: u32 = 0x8a5a2e;
const WOOD_L: u32 = 0xb07a44;
const GOLD: u32 = 0xffd84a;
const GOLD_D: u32 = 0xc8942a;
const FIRE: u32 = 0xff7a2a;
const FIRE_L: u32 = 0xffd23a;
const RED: u32 = 0xc8262e;

/// Blends two colours: 0 is `a`, 1 is `b`.
fn mix(a: u32, b: u32, t: f32) -> u32 {
    let ch = |s: u32| {
        let (x, y) = (((a >> s) & 255) as f32, ((b >> s) & 255) as f32);
        ((x + (y - x) * t).round().clamp(0.0, 255.0) as u32) << s
    };
    ch(16) | ch(8) | ch(0)
}

/// An entry's colour, a darker and a lighter shade of it.
fn shades(col: u32) -> (u32, u32, u32) {
    (col, scale(col, 0.6), mix(col, 0xffffff, 0.5))
}

/// Points along a gently curved blade from `a` to `b`, bowed `bow` pixels to its right.
fn curve(a: (f32, f32), b: (f32, f32), bow: f32) -> Vec<(i32, i32)> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let (nx, ny) = (-dy / len, dx / len);
    (0..=24)
        .map(|i| {
            let t = i as f32 / 24.0;
            let k = bow * (std::f32::consts::PI * t).sin();
            ((a.0 + dx * t - nx * k).round() as i32, (a.1 + dy * t - ny * k).round() as i32)
        })
        .collect()
}

/// A handle and cross-guard at the lower left, for a blade rising to the upper right.
fn hilt(c: &mut Canvas, guard: u32) {
    c.line((1, 14), (3, 12), WOOD);
    c.line((2, 14), (4, 12), WOOD_L);
    c.set(1, 15, guard);
    c.line((2, 9), (6, 13), guard);
}

/// The icon of a tree entry: its form, in its colour.
fn item(form: Form, col: u32) -> Sprite {
    let mut c = Canvas::new();
    let (m, d, l) = shades(col);
    let steel = mix(STEEL, col, 0.45);
    match form {
        Form::Knife => {
            c.line((2, 13), (4, 11), WOOD);
            c.line((3, 13), (5, 11), WOOD_L);
            c.line((3, 9), (7, 13), d);
            c.line((5, 10), (10, 5), steel);
            c.line((6, 10), (11, 5), m);
            c.line((6, 11), (12, 5), l);
            c.set(12, 4, l);
        }
        Form::Sword => {
            hilt(&mut c, d);
            c.line((5, 10), (13, 2), steel);
            c.line((5, 11), (14, 2), m);
            c.line((6, 11), (14, 3), l);
            c.set(14, 1, l);
            c.set(3, 11, GOLD);
        }
        Form::Saber => {
            hilt(&mut c, d);
            for (x, y) in curve((5.0, 10.0), (14.0, 1.0), 2.2) {
                c.set(x, y, m);
                c.set(x + 1, y, l);
            }
            c.set(3, 11, GOLD);
        }
        Form::Katana => {
            for (i, (x, y)) in [(1, 14), (2, 13), (3, 12), (1, 15), (2, 14), (3, 13)].into_iter().enumerate() {
                c.set(x, y, if i % 2 == 0 { 0x2a2a30 } else { 0xe8e0d0 });
            }
            c.disc(4.5, 10.5, 1.4, d);
            for (x, y) in curve((5.0, 10.0), (15.0, 0.0), 1.2) {
                c.set(x, y, m);
                c.set(x, y + 1, l);
            }
        }
        Form::Axe => {
            c.line((3, 14), (10, 4), WOOD);
            c.line((4, 14), (11, 4), WOOD_L);
            c.rect(9, 2, 12, 7, m);
            c.rect(12, 1, 13, 8, m);
            c.line((14, 1), (14, 8), l);
            c.rect(9, 6, 10, 8, d);
            c.set(10, 3, l);
        }
        Form::TwinAxes => {
            c.line((2, 14), (11, 3), WOOD);
            c.line((13, 14), (4, 3), WOOD_L);
            c.rect(1, 1, 4, 5, m);
            c.line((0, 1), (0, 5), l);
            c.rect(11, 1, 14, 5, m);
            c.line((15, 1), (15, 5), l);
            c.rect(3, 4, 4, 6, d);
            c.rect(11, 4, 12, 6, d);
        }
        Form::TwinBlades => {
            for (x, y) in curve((3.0, 12.0), (14.0, 1.0), 1.6) {
                c.set(x, y, m);
                c.set(x + 1, y, l);
            }
            for (x, y) in curve((12.0, 12.0), (1.0, 1.0), -1.6) {
                c.set(x, y, m);
                c.set(x - 1, y, l);
            }
            c.line((1, 10), (4, 13), d);
            c.line((11, 13), (14, 10), d);
            c.line((1, 14), (2, 13), WOOD);
            c.line((14, 14), (13, 13), WOOD);
        }
        Form::Wand => {
            c.line((3, 15), (10, 6), 0x3a2a3a);
            c.line((4, 15), (11, 6), 0x5a3a5a);
            c.disc(12.0, 4.0, 2.8, m);
            c.disc(12.0, 4.0, 1.6, l);
            for (x, y) in [(12, 0), (14, 1), (10, 1), (15, 3)] {
                c.set(x, y, m);
            }
            c.set(12, 4, 0xffffff);
        }
        Form::Spear => {
            c.line((1, 15), (10, 6), WOOD);
            c.line((2, 15), (11, 6), WOOD_L);
            c.set(10, 7, d);
            c.set(11, 7, d);
            c.line((11, 5), (14, 2), m);
            c.line((12, 6), (15, 3), m);
            c.line((11, 4), (13, 2), m);
            c.line((12, 5), (15, 2), l);
            c.set(15, 1, l);
        }
        Form::Hammer => {
            c.rect(7, 8, 8, 14, WOOD);
            c.rect(8, 8, 8, 14, WOOD_L);
            c.rect(6, 15, 9, 15, d);
            c.rect(2, 2, 13, 7, m);
            c.rect(2, 2, 13, 2, l);
            c.rect(2, 7, 13, 7, d);
            c.rect(2, 3, 3, 6, l);
            c.line((7, 3), (8, 6), l);
            c.set(6, 5, l);
            c.set(9, 4, l);
        }
        Form::Mace => {
            c.rect(7, 8, 8, 15, WOOD);
            c.set(8, 12, GOLD);
            c.disc(7.5, 5.0, 3.8, m);
            for (x, y) in [(7, 0), (8, 0), (2, 5), (13, 5), (3, 1), (12, 1), (3, 8), (12, 8)] {
                c.set(x, y, d);
            }
            c.disc(6.5, 4.0, 1.3, l);
        }
        Form::Bow => {
            for i in 0..=40 {
                let a = (120.0 + 120.0 * i as f32 / 40.0).to_radians();
                c.set((14.0 + 9.0 * a.cos()).round() as i32, (8.0 + 7.6 * a.sin()).round() as i32, m);
                c.set((14.0 + 8.0 * a.cos()).round() as i32, (8.0 + 6.8 * a.sin()).round() as i32, d);
            }
            c.line((10, 1), (10, 15), 0xe8e0d0);
            c.line((3, 8), (14, 8), WOOD_L);
            c.set(2, 8, l);
            c.set(3, 7, l);
            c.set(3, 9, l);
            c.set(14, 7, RED);
            c.set(14, 9, RED);
        }
        Form::Sickle => {
            c.line((2, 14), (6, 10), WOOD);
            c.line((3, 14), (7, 10), WOOD_L);
            for i in 0..=30 {
                let a = (180.0 + 170.0 * i as f32 / 30.0).to_radians();
                c.set((8.5 + 4.5 * a.cos()).round() as i32, (6.0 + 4.5 * a.sin()).round() as i32, m);
                c.set((8.5 + 3.5 * a.cos()).round() as i32, (6.0 + 3.5 * a.sin()).round() as i32, l);
            }
            for (x, y) in [(5, 15), (7, 14), (9, 15), (11, 14)] {
                c.set(x, y, STEEL_D);
            }
            c.disc(13.5, 13.0, 1.5, STEEL_D);
        }
        Form::Round => {
            c.disc(7.5, 7.5, 7.0, 0x5a616b);
            c.disc(7.5, 7.5, 5.8, m);
            c.paint(|x, _, p| if p == m && x % 3 == 0 { Some(d) } else { None });
            c.disc(7.5, 7.5, 1.8, l);
            c.set(7, 7, 0xffffff);
        }
        Form::Kite => {
            for y in 1..15 {
                let half = if y < 8 { 6.5 } else { 6.5 * (15 - y) as f32 / 7.0 };
                for x in 0..N {
                    let dx = (x as f32 - 7.5).abs();
                    if dx > half {
                        continue;
                    }
                    let rim = dx > half - 1.2 || y == 1;
                    c.set(x, y, if rim { d } else if dx < 0.8 || y == 6 { l } else { m });
                }
            }
        }
        Form::Spiral => {
            c.disc(7.5, 7.5, 7.0, d);
            c.disc(7.5, 7.5, 5.8, m);
            c.paint(|x, y, p| {
                let (dx, dy) = (x as f32 - 7.5, y as f32 - 7.5);
                let r = dx.hypot(dy);
                let a = dy.atan2(dx) / std::f32::consts::TAU;
                if p == m && (a + r / 2.4).rem_euclid(1.0) < 0.28 {
                    Some(l)
                } else {
                    None
                }
            });
            c.disc(7.5, 7.5, 1.5, STEEL);
        }
        Form::Tate => {
            c.rect(2, 1, 13, 14, m);
            for x in [5, 9] {
                c.line((x, 1), (x, 14), d);
            }
            c.rect(2, 3, 13, 3, d);
            c.rect(2, 12, 13, 12, d);
            c.disc(7.5, 7.5, 1.8, l);
            c.set(7, 7, 0xffffff);
        }
        Form::Mirror => {
            c.disc(7.5, 7.5, 7.0, GOLD_D);
            c.disc(7.5, 7.5, 5.8, m);
            c.line((4, 7), (7, 4), 0xffffff);
            c.line((5, 9), (9, 5), l);
            for (x, y) in [(7, 0), (0, 7), (15, 8), (8, 15)] {
                c.set(x, y, GOLD);
            }
        }
        Form::Flask => {
            c.rect(6, 0, 9, 1, 0xa87a4a);
            c.rect(6, 2, 9, 4, 0xc8dce8);
            c.disc(7.5, 10.0, 5.2, 0xc8dce8);
            c.disc(7.5, 10.2, 4.4, m);
            c.rect(7, 3, 8, 5, m);
            c.paint(|x, y, p| if p == m && y > 11 && (x + y) % 2 == 0 { Some(d) } else { None });
            c.set(5, 8, 0xffffff);
            c.set(5, 9, 0xffffff);
        }
        Form::Vial => {
            c.rect(6, 0, 9, 1, 0xa87a4a);
            c.rect(5, 2, 10, 14, 0xc8dce8);
            c.rect(6, 5, 9, 14, m);
            c.rect(6, 15, 9, 15, 0xc8dce8);
            c.paint(|x, y, p| if p == m && y > 10 && (x + y) % 2 == 0 { Some(d) } else { None });
            c.line((6, 3), (6, 7), 0xffffff);
        }
        Form::Gourd => {
            c.rect(7, 0, 8, 1, 0xa87a4a);
            c.disc(7.5, 4.5, 2.8, mix(m, 0xd8b878, 0.4));
            c.disc(7.5, 11.0, 4.6, mix(m, 0xd8b878, 0.25));
            c.rect(4, 7, 11, 7, 0x8a2a2a);
            c.disc(6.0, 10.0, 1.2, l);
            c.paint(|x, y, p| if p != 0x8a2a2a && y > 12 && (x + y) % 2 == 0 { Some(d) } else { None });
        }
        Form::Rune(n) => {
            c.disc(7.5, 8.0, 7.0, 0x4a4e56);
            c.disc(7.0, 7.5, 6.0, 0x7a7e86);
            c.paint(|x, y, p| if p == 0x7a7e86 && (x * 7 + y * 3) % 11 == 0 { Some(0x6a6e76) } else { None });
            let glyphs: [&[(i32, i32, i32, i32)]; 10] = [
                &[(6, 3, 6, 12), (6, 5, 10, 3), (6, 8, 10, 6)],
                &[(5, 12, 5, 3), (5, 3, 10, 6), (10, 6, 10, 12)],
                &[(6, 3, 6, 12), (6, 5, 10, 7), (10, 7, 6, 9)],
                &[(6, 3, 6, 12), (6, 3, 10, 6), (6, 6, 10, 9)],
                &[(6, 3, 6, 12), (6, 3, 10, 5), (10, 5, 6, 8), (6, 8, 10, 12)],
                &[(10, 3, 6, 7), (6, 7, 10, 12)],
                &[(5, 3, 10, 12), (10, 3, 5, 12)],
                &[(7, 3, 7, 12), (7, 7, 4, 3), (7, 7, 10, 3)],
                &[(7, 3, 7, 12), (7, 3, 4, 6), (7, 3, 10, 6)],
                &[(7, 3, 10, 7), (10, 7, 7, 11), (7, 11, 4, 7), (4, 7, 7, 3)],
            ];
            for &(x0, y0, x1, y1) in glyphs[(n as usize).min(9)] {
                c.line((x0, y0), (x1, y1), l);
            }
            c.paint(|x, y, p| if p == l && (x + y) % 3 == 0 { Some(m) } else { None });
        }
    }
    c.outlined()
}

/// An extra life: a rose bottle marked with a heart.
fn life() -> Sprite {
    let mut c = Canvas::new();
    let col = 0xff6d8d;
    c.rect(6, 0, 9, 1, 0xa87a4a);
    c.rect(6, 2, 9, 4, 0xc8dce8);
    c.disc(7.5, 10.0, 5.2, 0xc8dce8);
    c.disc(7.5, 10.2, 4.4, col);
    c.rect(7, 3, 8, 5, col);
    let w = 0xffffff;
    c.rect(5, 8, 6, 9, w);
    c.rect(9, 8, 10, 9, w);
    c.rect(5, 10, 10, 10, w);
    c.rect(6, 11, 9, 11, w);
    c.rect(7, 12, 8, 12, w);
    c.outlined()
}

/// A rune fragment: a broken shard of stone with a glowing line of a rune.
fn rune_fragment() -> Sprite {
    let mut c = Canvas::new();
    for y in 1..15 {
        for x in 2..14 {
            let inside = (x - 2) + (y - 1) >= 3 && (13 - x) + (14 - y) >= 2 && x + (14 - y) >= 4 && (13 - x) + y >= 3;
            if inside {
                c.set(x, y, if (x * 5 + y * 3) % 7 == 0 { 0x6a6e76 } else { 0x8a8e96 });
            }
        }
    }
    c.line((7, 3), (7, 12), 0x9ad1ff);
    c.line((7, 5), (10, 3), 0x9ad1ff);
    c.line((7, 8), (10, 6), 0xd8f0ff);
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

fn pickup(kind: RangedKind) -> Sprite {
    match kind {
        RangedKind::Arrow => {
            let mut c = Canvas::new();
            arrow(&mut c, 0, STEEL, 0xd8d0c0, RED);
            c.outlined()
        }
        RangedKind::Boomerang => {
            let mut c = Canvas::new();
            for i in 0..8 {
                c.rect(2 + i, 3 + i, 3 + i, 4 + i, 0xc98f4a);
                c.rect(9 + i.min(5), 10 - i, 10 + i.min(5), 11 - i, 0xc98f4a);
            }
            c.paint(|x, y, _| if (x + y) % 5 == 0 { Some(0x8a5a2e) } else { None });
            c.outlined()
        }
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

/// A golden key: an ornate ring bow, a shaft, two teeth.
fn key() -> Sprite {
    let mut c = Canvas::new();
    c.disc(4.5, 4.5, 4.0, GOLD_D);
    c.disc(4.5, 4.5, 3.2, GOLD);
    c.disc(4.5, 4.5, 1.4, 0x1a1408);
    c.line((7, 7), (14, 14), GOLD);
    c.line((8, 7), (14, 13), GOLD_D);
    c.rect(11, 12, 12, 15, GOLD);
    c.rect(13, 10, 15, 11, GOLD);
    c.set(3, 2, 0xfff6c0);
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
    fn every_entry_of_a_tree_has_a_distinct_icon() {
        for hero in ROSTER {
            for tree in TREES {
                let mut seen: Vec<&Vec<Vec<Option<u32>>>> = Vec::new();
                for tier in 1..=TIERS {
                    let s = icon(Icon::Node(hero, tree, tier));
                    assert!(s.pixels.iter().flatten().filter(|p| p.is_some()).count() > 20, "{hero:?} {tree:?} {tier} is nearly empty");
                    assert!(!seen.contains(&&s.pixels), "{hero:?} {tree:?} {tier} looks like another icon");
                    seen.push(&s.pixels);
                }
            }
        }
    }
}
