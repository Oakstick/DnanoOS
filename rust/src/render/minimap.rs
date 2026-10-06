//! The minimap's data (UI.md §7): the level drawn as an image at a few pixels a tile, under the fog
//! of war (`logic::explore`), and the markers to draw on top of it.
//!
//! `MapImage` is built once a level from its grid, in the level's own colours muted so the markers
//! read on them: floor (lighter the higher it stands, a dark lip where it drops), walls with a darker
//! outline, water, lava or a chasm, bridges and ramps. Rock that touches no floor is left clear, so
//! the level reads as its own shape. Tiles not yet seen are clear too, and the edge of what has been
//! seen is soft. As the fog lifts and the ground changes (a gate opening, a causeway rising) the image
//! is patched where it changed, and `dirty` says which pixels, so the uploader sends only those rows.
//!
//! `markers` lists what to draw on it from the game: the hero, the foes by rank, the archangels'
//! shrines, keys, loot and chests, the gates and portals, the Pit's doors and platform, the old man's
//! lamp, the general's instruments and post. All of it reads the game and nothing writes to it.
//!
//! `debug_picture` puts the two together on the CPU for the capture tool (`--minimap`).

use crate::game::Game;
use crate::logic::angels::{Archangel, ARCHANGELS};
use crate::logic::enemy::EnemyMode;
use crate::logic::explore::Explore;
use crate::logic::garden::GardenPlan;
use crate::logic::grid::Grid;
use crate::logic::items::DiamondKind;
use crate::logic::level::Theme;
use crate::logic::sins::Sin;
use crate::logic::types::{Tile, P, V};

/// The image is never wider or taller than this, in pixels.
pub const MAX_SIDE: u32 = 1024;
/// Pixels a tile, at most (a small level), and at least.
const MOST_PER_TILE: u32 = 8;
const LEAST_PER_TILE: u32 = 2;
/// Rows of the grid looked over each sync for ground that changed under the image.
const SCAN_ROWS: i32 = 4;
/// How much lighter the floor is drawn for each level it stands up.
const LIFT: f32 = 0.08;
/// Foes are shown this far off (in tiles) where the level is revealed whole and sight is not asked.
const FOE_RANGE: f64 = 2.0 * crate::logic::explore::SIGHT as f64;

// ---------------------------------------------------------------------------------- the looks

/// Which look the map takes: one for each kind of level.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style {
    /// Dungeon I, the moonlit landscape garden.
    Garden,
    /// Dungeon II, the flooded Cisterns, and the stone dungeons after it.
    Cisterns,
    Crypt,
    /// The Cyclops' sunny meadow.
    Meadow,
    /// The Portals of Hell, and the gym over its lava.
    Hub,
    Gym,
    /// The Vaults of Mammon.
    Vault,
    /// A sin's realm after the vaults, and a prince's court.
    Realm(Sin),
    Court(Sin),
    Pit,
    Lair,
}

impl Style {
    /// The look of a level of `theme` (a boss's arena or not), dungeon `index` of the estates.
    pub fn of(theme: Theme, boss: bool, index: usize) -> Style {
        match theme {
            Theme::Estate if boss => Style::Meadow,
            Theme::Estate if index == 0 => Style::Garden,
            Theme::Estate if index == 1 => Style::Cisterns,
            Theme::Estate => Style::Crypt,
            Theme::Meadow => Style::Meadow,
            Theme::Hell => Style::Hub,
            Theme::Gym => Style::Gym,
            Theme::Realm(Sin::Greed) => Style::Vault,
            Theme::Realm(s) => Style::Realm(s),
            Theme::Court(s) => Style::Court(s),
            Theme::Pit => Style::Pit,
            Theme::Lair => Style::Lair,
        }
    }

    pub fn of_game(g: &Game) -> Style {
        Style::of(g.theme(), g.level.data.boss, g.dungeon)
    }
}

/// What a hole in the floor (`Tile::Pit`) is, in a look.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Hole {
    Lava,
    Water,
    Chasm,
}

/// A look's colours (muted already), as 0xRRGGBB.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Palette {
    pub floor: u32,
    pub wall: u32,
    /// The outline of the walls, where they meet the floor.
    pub edge: u32,
    pub water: u32,
    pub bridge: u32,
    /// The hollow foes come out of, and the Pit's platform.
    pub special: u32,
    pub hole: Hole,
    /// The holes' colour, and the flecks on it (embers, glints).
    pub pit: u32,
    pub fleck: u32,
}

fn mix(a: u32, b: u32, t: f32) -> u32 {
    let ch = |s: u32| -> u32 {
        let (x, y) = (((a >> s) & 0xff) as f32, ((b >> s) & 0xff) as f32);
        ((x + (y - x) * t).round().clamp(0.0, 255.0) as u32) << s
    };
    ch(16) | ch(8) | ch(0)
}

/// `c` lightened (f > 1) or darkened (f < 1).
fn scale(c: u32, f: f32) -> u32 {
    let ch = |s: u32| ((((c >> s) & 0xff) as f32 * f).round().clamp(0.0, 255.0) as u32) << s;
    ch(16) | ch(8) | ch(0)
}

/// A colour of the level taken a step toward grey and down, so the markers stand out on it.
fn mute(c: u32) -> u32 {
    let (r, g, b) = (((c >> 16) & 0xff) as f32, ((c >> 8) & 0xff) as f32, (c & 0xff) as f32);
    let l = (0.3 * r + 0.59 * g + 0.11 * b).round() as u32;
    scale(mix(c, l << 16 | l << 8 | l, 0.3), 0.86)
}

impl Palette {
    pub fn of(style: Style) -> Palette {
        // Floor, wall, edge, water, bridge, special, the holes and their flecks.
        let raw = |floor, wall, edge, water, bridge, special, hole, pit, fleck| Palette {
            floor,
            wall,
            edge,
            water,
            bridge,
            special,
            hole,
            pit,
            fleck,
        };
        let p = match style {
            Style::Garden => raw(0x4c6440, 0x86906e, 0x121810, 0x3a72a0, 0x9a7448, 0x5a4632, Hole::Chasm, 0x16140f, 0x2a261c),
            Style::Cisterns => raw(0x4a5856, 0x8c9690, 0x101615, 0x2e6a82, 0x7a5a3c, 0x4a3430, Hole::Water, 0x24566a, 0x6aa8c0),
            Style::Crypt => raw(0x544c44, 0x938a7c, 0x14110e, 0x2e5a68, 0x76583a, 0x523430, Hole::Chasm, 0x110f0e, 0x26201c),
            Style::Meadow => raw(0x5a8c3a, 0xaaa496, 0x2a2822, 0x3f8fc8, 0x9a7a52, 0x5a4a30, Hole::Chasm, 0x1a1410, 0x2a2018),
            Style::Hub | Style::Gym => raw(0x4e4240, 0x82726a, 0x100a08, 0x2a4a6a, 0x84603e, 0x4a3a34, Hole::Lava, 0x8a3412, 0xd8702a),
            Style::Vault => raw(0x6a5a3e, 0xb49e6c, 0x18100a, 0x2e5a6a, 0x7a5a36, 0x6a5430, Hole::Chasm, 0x140e08, 0x5a4420),
            Style::Realm(s) => {
                let (hole, pit, fleck) = match s {
                    Sin::Envy => (Hole::Water, 0x2a6a5e, 0x6ac8a8),
                    Sin::Sloth => (Hole::Water, 0x3e4e34, 0x7a8a52),
                    Sin::Gluttony => (Hole::Lava, 0x7a2c10, 0xc8642a),
                    Sin::Pride => (Hole::Water, 0x262a44, 0x8a8ac8),
                    _ => (Hole::Chasm, mix(0x100e10, s.color(), 0.08), mix(0x2a2428, s.color(), 0.2)),
                };
                raw(
                    mix(0x4e4a44, s.color(), 0.2),
                    mix(0x8a8478, s.color(), 0.18),
                    0x121010,
                    0x2e5a6a,
                    0x7a5a3a,
                    mix(0x4a3a30, s.color(), 0.3),
                    hole,
                    pit,
                    fleck,
                )
            }
            Style::Court(s) => {
                let (hole, pit, fleck) = match s {
                    Sin::Wrath => (Hole::Lava, 0x8a3412, 0xd8702a),
                    Sin::Envy => (Hole::Water, 0x2a6a5e, 0x6ac8a8),
                    Sin::Lust => (Hole::Water, 0x1a2a3e, 0x6a7aa8),
                    _ => (Hole::Chasm, 0x120e0e, mix(0x2a2428, s.color(), 0.25)),
                };
                raw(
                    mix(0x5e5a52, s.color(), 0.16),
                    mix(0x9a9488, s.color(), 0.14),
                    0x141210,
                    0x2e5a6a,
                    0x7a5a3a,
                    mix(0x5a4a3a, s.color(), 0.3),
                    hole,
                    pit,
                    fleck,
                )
            }
            Style::Pit => raw(0x45433e, 0x7c7870, 0x0e0d0c, 0x2e5a6a, 0x6a5038, 0x77736a, Hole::Chasm, 0x0c0b0a, 0x1c1a18),
            Style::Lair => raw(0x363a54, 0x62646e, 0x060608, 0x22364a, 0x5a4232, 0x4a3a3a, Hole::Chasm, 0x08080a, 0x2a1414),
        };
        Palette {
            floor: mute(p.floor),
            wall: mute(p.wall),
            edge: p.edge,
            // (Where the holes are water, the water is theirs.)
            water: mute(if p.hole == Hole::Water { p.pit } else { p.water }),
            bridge: mute(p.bridge),
            special: mute(p.special),
            hole: p.hole,
            // (Lava keeps its heat, darkened: it is the one hazard that must read at a glance.)
            pit: if p.hole == Hole::Lava { p.pit } else { mute(p.pit) },
            fleck: if p.hole == Hole::Lava { p.fleck } else { mute(p.fleck) },
        }
    }
}

// ---------------------------------------------------------------------------------- the image

/// What each tile is drawn as.
pub mod class {
    /// Rock touching no floor: left clear.
    pub const VOID: u8 = 0;
    pub const WALL: u8 = 1;
    pub const FLOOR: u8 = 2;
    pub const RAMP: u8 = 3;
    pub const BRIDGE: u8 = 4;
    pub const WATER: u8 = 5;
    /// A hole in the floor: lava, a chasm (or water, in the looks whose holes are water).
    pub const HOLE: u8 = 6;
    /// The hollow, the Pit's platform.
    pub const SPECIAL: u8 = 7;
}

/// A rectangle of the image's pixels.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl PixelRect {
    pub fn union(self, o: PixelRect) -> PixelRect {
        let (x0, y0) = (self.x.min(o.x), self.y.min(o.y));
        let (x1, y1) = ((self.x + self.w).max(o.x + o.w), (self.y + self.h).max(o.y + o.h));
        PixelRect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
    }
}

/// The level as an RGBA image (rows top to bottom, the grid's north at the top), under its fog.
#[derive(Clone, Debug)]
pub struct MapImage {
    pub w: u32,
    pub h: u32,
    pub px_per_tile: u32,
    pub rgba: Vec<u8>,
    /// The pixels changed since the uploader last took them (`take_dirty`); the whole image after
    /// a build.
    pub dirty: Option<PixelRect>,
    pub style: Style,
    pub palette: Palette,
    /// The grid's size, in tiles.
    tiles: (i32, i32),
    /// Each tile's class and floor level as drawn (`class | level << 4`).
    keys: Vec<u16>,
    /// The garden's or the arena's watercourses (rock in the grid).
    water: Vec<bool>,
    /// The ground without the fog.
    base: Vec<u8>,
    /// The fog's level count as built, and how much of its log is drawn.
    epoch: u32,
    fog_read: usize,
    /// The next row the ground is looked over at.
    scan: i32,
}

/// Pixels a tile for a level of `w` x `h` tiles: as many as fit `MAX_SIDE`, up to 8.
pub fn px_per_tile(w: i32, h: i32) -> u32 {
    (MAX_SIDE / w.max(h).max(1) as u32).clamp(LEAST_PER_TILE, MOST_PER_TILE)
}

/// A small, steady hash of a pixel: the grain on the floor, the glints on water.
fn grain(x: u32, y: u32) -> u32 {
    let mut h = x.wrapping_mul(0x9e37_79b1) ^ y.wrapping_mul(0x85eb_ca77);
    h ^= h >> 15;
    h = h.wrapping_mul(0xc2b2_ae3d);
    h ^ (h >> 13)
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl MapImage {
    /// The map of the level the game is in, under its fog.
    pub fn build(game: &Game) -> MapImage {
        Self::build_from(&game.level.grid, Style::of_game(game), game.garden.as_ref(), &game.explore)
    }

    /// The map of `grid` in `style`, its watercourses from the garden's plan, under `fog`.
    pub fn build_from(grid: &Grid, style: Style, garden: Option<&GardenPlan>, fog: &Explore) -> MapImage {
        let (tw, th) = (grid.width, grid.height);
        let s = px_per_tile(tw, th);
        let (w, h) = (tw as u32 * s, th as u32 * s);
        let n = (tw * th) as usize;
        let mut water = vec![false; n];
        if let Some(plan) = garden {
            for p in plan.water().map(|r| r.p).chain(plan.fountain.map(|f| f.p)) {
                if grid.in_bounds(p.x, p.y) {
                    water[(p.y * tw + p.x) as usize] = true;
                }
            }
        }
        let mut m = MapImage {
            w,
            h,
            px_per_tile: s,
            rgba: vec![0; (w * h * 4) as usize],
            dirty: Some(PixelRect { x: 0, y: 0, w, h }),
            style,
            palette: Palette::of(style),
            tiles: (tw, th),
            keys: vec![0; n],
            water,
            base: vec![0; (w * h * 4) as usize],
            epoch: fog.epoch(),
            fog_read: fog.revealed(),
            scan: 0,
        };
        for y in 0..th {
            for x in 0..tw {
                m.keys[(y * tw + x) as usize] = m.key_of(grid, x, y);
            }
        }
        for y in 0..th {
            for x in 0..tw {
                m.paint(x, y);
            }
        }
        m.compose(fog, 0, 0, tw - 1, th - 1);
        m
    }

    /// Brings the image up to date with the game: built afresh for a new level, patched where the
    /// fog has lifted and where the ground has changed (a few rows of the grid looked over each time).
    pub fn sync(&mut self, game: &Game) {
        if game.explore.epoch() != self.epoch || (game.level.grid.width, game.level.grid.height) != self.tiles {
            *self = Self::build(game);
            return;
        }
        self.sync_from(&game.level.grid, &game.explore);
    }

    /// `sync` for a grid and its fog (the same level as built).
    pub fn sync_from(&mut self, grid: &Grid, fog: &Explore) {
        let tw = self.tiles.0;
        // The fog: the tiles it has revealed since, and the soft edge round them.
        let new = fog.revealed_since(self.fog_read);
        if !new.is_empty() {
            let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
            for &i in new {
                let (x, y) = (i as i32 % tw, i as i32 / tw);
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
            }
            self.compose(fog, x0 - 2, y0 - 2, x1 + 2, y1 + 2);
        }
        self.fog_read = fog.revealed();
        // The ground: a few rows looked over, and any tile changed drawn again with those about it.
        let mut changed: Option<(i32, i32, i32, i32)> = None;
        for _ in 0..SCAN_ROWS.min(self.tiles.1) {
            let y = self.scan;
            self.scan = (self.scan + 1) % self.tiles.1.max(1);
            for x in 0..tw {
                let k = self.key_of(grid, x, y);
                let i = (y * tw + x) as usize;
                if self.keys[i] != k {
                    self.keys[i] = k;
                    changed = Some(match changed {
                        None => (x, y, x, y),
                        Some((a, b, c, d)) => (a.min(x), b.min(y), c.max(x), d.max(y)),
                    });
                }
            }
        }
        if let Some((x0, y0, x1, y1)) = changed {
            for y in (y0 - 1).max(0)..=(y1 + 1).min(self.tiles.1 - 1) {
                for x in (x0 - 1).max(0)..=(x1 + 1).min(tw - 1) {
                    self.paint(x, y);
                }
            }
            self.compose(fog, x0 - 2, y0 - 2, x1 + 2, y1 + 2);
        }
    }

    /// The pixels changed since last asked, for the uploader to send.
    pub fn take_dirty(&mut self) -> Option<PixelRect> {
        self.dirty.take()
    }

    /// Where a point of the world (in tiles; their centres are whole numbers) falls on the image.
    pub fn to_pixel(&self, at: V) -> (f32, f32) {
        let s = self.px_per_tile as f32;
        ((at.x as f32 + 0.5) * s, (at.y as f32 + 0.5) * s)
    }

    /// The class a tile is drawn as (`class`), and its floor level.
    pub fn class_at(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x >= self.tiles.0 || y >= self.tiles.1 {
            return class::VOID;
        }
        (self.keys[(y * self.tiles.0 + x) as usize] & 0xf) as u8
    }

    fn level_at(&self, x: i32, y: i32) -> i32 {
        if x < 0 || y < 0 || x >= self.tiles.0 || y >= self.tiles.1 {
            return 0;
        }
        (self.keys[(y * self.tiles.0 + x) as usize] >> 4) as i32
    }

    /// One pixel, as RGBA.
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.w + x) * 4) as usize;
        [self.rgba[i], self.rgba[i + 1], self.rgba[i + 2], self.rgba[i + 3]]
    }

    fn water_at(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.tiles.0 && y < self.tiles.1 && self.water[(y * self.tiles.0 + x) as usize]
    }

    /// How a tile is drawn: its class and floor level.
    fn key_of(&self, grid: &Grid, x: i32, y: i32) -> u16 {
        use class::*;
        let c = if grid.is_bridge(x, y) {
            BRIDGE
        } else if self.water_at(x, y) {
            WATER
        } else {
            match grid.get(x, y) {
                Tile::Rock => {
                    let open = (-1..=1).any(|dy| {
                        (-1..=1).any(|dx| {
                            let (nx, ny) = (x + dx, y + dy);
                            grid.in_bounds(nx, ny) && (grid.get(nx, ny) != Tile::Rock || self.water_at(nx, ny))
                        })
                    });
                    if open {
                        WALL
                    } else {
                        VOID
                    }
                }
                Tile::Pit if self.palette.hole == Hole::Water => WATER,
                Tile::Pit => HOLE,
                _ if grid.is_ramp(x, y) => RAMP,
                Tile::Nest => SPECIAL,
                _ if self.style == Style::Pit && crate::logic::pit::on_platform_tile(P::new(x, y)) => SPECIAL,
                _ => FLOOR,
            }
        };
        c as u16 | (grid.elevation(x, y).clamp(0, 15) as u16) << 4
    }

    /// Paints one tile's pixels of the ground (no fog).
    fn paint(&mut self, x: i32, y: i32) {
        use class::*;
        let s = self.px_per_tile as i32;
        let pal = self.palette;
        let c = self.class_at(x, y);
        let lv = self.level_at(x, y);
        let walk = |c: u8| matches!(c, FLOOR | RAMP | BRIDGE | SPECIAL);
        let open = |c: u8| c != VOID && c != WALL;
        // The neighbours: west, east, north, south.
        let n = [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)];
        let nc = n.map(|(a, b)| self.class_at(a, b));
        let nl = n.map(|(a, b)| self.level_at(a, b));
        // Floor rises lighter level by level.
        let lift = 1.0 + LIFT * lv as f32;
        // A ramp climbs the way its neighbours' floors differ; a bridge is crossed the way the
        // floor meets it.
        let ramp_along_x = nl[0] != nl[1] && (walk(nc[0]) || walk(nc[1]));
        let bridge_along_x = walk(nc[0]) || walk(nc[1]);
        let line = (s / 6).max(1);
        for j in 0..s {
            for i in 0..s {
                let (px, py) = ((x * s + i) as u32, (y * s + j) as u32);
                let g = grain(px, py);
                let jitter = 1.0 + ((g & 0xff) as f32 / 255.0 - 0.5) * 0.07;
                // Which borders this pixel lies on: west, east, north, south.
                let on = [i < line, i >= s - line, j < line, j >= s - line];
                let mut a = 255u8;
                let rgb = match c {
                    VOID => {
                        a = 0;
                        0
                    }
                    WALL => {
                        let mut edge = (0..4).any(|k| on[k] && open(nc[k]));
                        // A corner of floor touching the wall only diagonally.
                        if !edge && (on[0] || on[1]) && (on[2] || on[3]) {
                            let dx = if on[0] { -1 } else { 1 };
                            let dy = if on[2] { -1 } else { 1 };
                            edge = open(self.class_at(x + dx, y + dy));
                        }
                        if edge {
                            pal.edge
                        } else {
                            scale(pal.wall, jitter)
                        }
                    }
                    WATER => {
                        if g.is_multiple_of(23) {
                            scale(pal.water, 1.35)
                        } else {
                            scale(pal.water, 1.0 + ((g & 0xff) as f32 / 255.0 - 0.5) * 0.05)
                        }
                    }
                    HOLE => match pal.hole {
                        Hole::Lava if g.is_multiple_of(29) => pal.fleck,
                        Hole::Lava => scale(pal.pit, jitter),
                        _ if g.is_multiple_of(31) => pal.fleck,
                        _ => pal.pit,
                    },
                    _ => {
                        let base = match c {
                            BRIDGE => pal.bridge,
                            SPECIAL => pal.special,
                            _ => pal.floor,
                        };
                        let mut col = scale(base, lift * jitter);
                        if c == RAMP {
                            // Steps across the slope.
                            let k = if ramp_along_x { i } else { j };
                            if (k / line.max(1)) % 2 == 0 {
                                col = scale(col, 1.14);
                            }
                        }
                        if c == BRIDGE {
                            // Planks across the way over.
                            let k = if bridge_along_x { i } else { j };
                            if k % 3 == 2 {
                                col = scale(col, 0.72);
                            }
                        }
                        for k in 0..4 {
                            if !on[k] {
                                continue;
                            }
                            if walk(nc[k]) && nl[k] < lv && c != RAMP && nc[k] != RAMP {
                                // The lip of a drop.
                                col = scale(col, 0.55);
                            } else if nc[k] == HOLE || nc[k] == WATER {
                                col = scale(col, 0.75);
                            }
                        }
                        col
                    }
                };
                let o = ((py * self.w + px) * 4) as usize;
                self.base[o] = (rgb >> 16) as u8;
                self.base[o + 1] = (rgb >> 8) as u8;
                self.base[o + 2] = rgb as u8;
                self.base[o + 3] = a;
            }
        }
    }

    /// Lays the fog over the ground for tiles `x0..=x1`, `y0..=y1` (clamped), and marks them dirty:
    /// each pixel as seen as the tiles about it (blended between their middles), so the edge of what
    /// has been seen fades over half a tile, on the seen side.
    fn compose(&mut self, fog: &Explore, x0: i32, y0: i32, x1: i32, y1: i32) {
        let (tw, th) = self.tiles;
        let (x0, y0, x1, y1) = (x0.max(0), y0.max(0), x1.min(tw - 1), y1.min(th - 1));
        if x0 > x1 || y0 > y1 {
            return;
        }
        let s = self.px_per_tile as i32;
        // How seen each tile is, over the rectangle and a tile round it (beyond the grid, as its
        // edge). Clear rock beside a seen tile counts as seen: it is clear anyway, and so only the
        // edge of the explored ground fades, never the outside of the walls that bound it.
        let (lw, lh) = (x1 - x0 + 3, y1 - y0 + 3);
        let mut lit = vec![0.0f32; (lw * lh) as usize];
        let seen = |x: i32, y: i32| fog.seen(x.clamp(0, tw - 1), y.clamp(0, th - 1));
        for ly in 0..lh {
            for lx in 0..lw {
                let (x, y) = (x0 - 1 + lx, y0 - 1 + ly);
                let (cx, cy) = (x.clamp(0, tw - 1), y.clamp(0, th - 1));
                let on = seen(cx, cy)
                    || (self.keys[(cy * tw + cx) as usize] & 0xf) as u8 == class::VOID
                        && (-1..=1).any(|dy| (-1..=1).any(|dx| (dx, dy) != (0, 0) && seen(cx + dx, cy + dy)));
                lit[(ly * lw + lx) as usize] = if on { 1.0 } else { 0.0 };
            }
        }
        let at = |x: i32, y: i32| lit[((y - y0 + 1) * lw + (x - x0 + 1)) as usize];
        for py in (y0 * s)..((y1 + 1) * s) {
            let v = (py as f32 + 0.5) / s as f32 - 0.5;
            let ty = v.floor() as i32;
            let fy = v - ty as f32;
            for px in (x0 * s)..((x1 + 1) * s) {
                let u = (px as f32 + 0.5) / s as f32 - 0.5;
                let tx = u.floor() as i32;
                let fx = u - tx as f32;
                let (a, b, c, d) = (at(tx, ty), at(tx + 1, ty), at(tx, ty + 1), at(tx + 1, ty + 1));
                let shown = if a == b && b == c && c == d {
                    a
                } else {
                    let top = a + (b - a) * fx;
                    let bottom = c + (d - c) * fx;
                    smoothstep(0.3, 1.0, top + (bottom - top) * fy)
                };
                let o = ((py as u32 * self.w + px as u32) * 4) as usize;
                self.rgba[o..o + 3].copy_from_slice(&self.base[o..o + 3]);
                self.rgba[o + 3] = (self.base[o + 3] as f32 * shown).round() as u8;
            }
        }
        let r = PixelRect { x: (x0 * s) as u32, y: (y0 * s) as u32, w: ((x1 - x0 + 1) * s) as u32, h: ((y1 - y0 + 1) * s) as u32 };
        self.dirty = Some(self.dirty.map_or(r, |d| d.union(r)));
    }
}

// ---------------------------------------------------------------------- the image on the GPU

/**
 * The minimap's image as the screen shows it (UI.md §7): owned beside the overlay
 * (`Overlay::map`), brought up to date as the HUD draws (`sync`), and what changed sent to the UI
 * atlas's `MAP_REGION` by the app after the HUD is laid out (`upload`). A new level's image goes
 * up whole, with the rest of the region cleared (so nothing of the last level shows past its
 * edge); after that only the rectangle the fog or the ground changed.
 */
#[derive(Default)]
pub struct MapLayer {
    pub image: Option<MapImage>,
    /// What to send, in the image's pixels: the whole region (a new level), or a rectangle.
    pending: Option<Pending>,
    scratch: Vec<u8>,
    /// Bytes sent by the last upload, and how many uploads in all (for the readout and tests).
    pub sent_last: usize,
    pub uploads: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Pending {
    Whole,
    Rect(PixelRect),
}

impl MapLayer {
    /// Brings the image up to date with the level the game is in (built the first time and for
    /// each new level, patched otherwise), and notes what the GPU must be sent.
    pub fn sync(&mut self, game: &Game) {
        let img = match &mut self.image {
            Some(img) => {
                img.sync(game);
                img
            }
            None => self.image.insert(MapImage::build(game)),
        };
        let Some(d) = img.take_dirty() else { return };
        let whole = d == PixelRect { x: 0, y: 0, w: img.w, h: img.h };
        self.pending = Some(match (self.pending, whole) {
            (_, true) | (Some(Pending::Whole), _) => Pending::Whole,
            (Some(Pending::Rect(r)), false) => Pending::Rect(r.union(d)),
            (None, false) => Pending::Rect(d),
        });
    }

    /// Whether something waits to be sent.
    pub fn dirty(&self) -> bool {
        self.pending.is_some()
    }

    /**
     * Sends what changed to the atlas through `write` (`Gpu::write_atlas`): tightly packed RGBA
     * rows of a part of `MAP_REGION`. Returns the bytes sent.
     */
    pub fn upload(&mut self, mut write: impl FnMut(crate::render::overlay::AtlasRegion, &[u8])) -> usize {
        use crate::render::overlay::MAP_REGION;
        let (Some(p), Some(img)) = (self.pending.take(), &self.image) else { return 0 };
        let (r, from) = match p {
            Pending::Whole => (MAP_REGION, PixelRect { x: 0, y: 0, w: MAP_REGION.w, h: MAP_REGION.h }),
            Pending::Rect(d) => (MAP_REGION.sub(d.x, d.y, d.w, d.h), d),
        };
        let row = (r.w * 4) as usize;
        self.scratch.clear();
        self.scratch.resize(row * r.h as usize, 0);
        for y in 0..r.h {
            let iy = from.y + y;
            if iy >= img.h || from.x >= img.w {
                continue;
            }
            let n = (r.w.min(img.w - from.x) * 4) as usize;
            let src = ((iy * img.w + from.x) * 4) as usize;
            let dst = y as usize * row;
            self.scratch[dst..dst + n].copy_from_slice(&img.rgba[src..src + n]);
        }
        write(r, &self.scratch);
        self.sent_last = self.scratch.len();
        self.uploads += 1;
        self.sent_last
    }

    /// Where a point of the image (in its pixels) is in the atlas (0..1).
    pub fn atlas_uv(px: f32, py: f32) -> [f32; 2] {
        use crate::render::overlay::{ATLAS_H, ATLAS_W, MAP_REGION};
        [(MAP_REGION.x as f32 + px) / ATLAS_W as f32, (MAP_REGION.y as f32 + py) / ATLAS_H as f32]
    }
}

// -------------------------------------------------------------------------------- the markers

/// How much a foe matters, for its mark: red dots for the plain, gold for a pack's leader and a
/// rival, a crown for a general, a skull for a boss or a prince.
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum FoeRank {
    Plain,
    Elite,
    Rival,
    General,
    Boss,
}

/// What a marker shows.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MarkerKind {
    /// The hero, and the way he faces (an angle in the grid's x/y plane, y down: 0 east, π/2 south).
    Hero {
        heading: f32,
    },
    Foe(FoeRank),
    /// An archangel's shrine: the angel it shows now, and whether he has given here.
    Shrine {
        angel: Archangel,
        spent: bool,
    },
    /// A golden key (a nugget) of the estates, the Cyclops' red key, a remedy (the green cross),
    /// a bottle lying in the first dungeon, an item shrine.
    Key,
    RedKey,
    Remedy,
    Potion,
    ItemShrine,
    /// Gold, gems and items lying in the vaults, and a treasure chest.
    Loot,
    Chest,
    /// The gate to the boss, or a realm's far portal: shut, or open.
    Gate {
        open: bool,
    },
    /// The portal he came in by.
    Entry,
    /// A portal of the hub: its sin, and whether its prince is slain.
    Portal {
        sin: Sin,
        conquered: bool,
    },
    /// The House's door in the hub, open once the seal is broken.
    HouseDoor {
        open: bool,
    },
    /// One of the Pit's iron doors, and the platform it rides down on (half its side, in tiles).
    PitDoor {
        open: bool,
    },
    Platform {
        half: f32,
    },
    /// Where the old man's lamp points: the realm's far portal (always true).
    Lamp,
    /// One of the general's instruments (its order, 0..2), and his command post.
    Instrument {
        order: usize,
        broken: bool,
    },
    Post,
    /// The cage in Krampus's lair, and whether the boy is out of it.
    Cage {
        open: bool,
    },
}

/// One thing to draw on the map.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Marker {
    pub kind: MarkerKind,
    /// Where it is, in tiles (their centres are whole numbers).
    pub at: V,
    /// Whether the map may show it: seen (or in sight, for what moves), or always shown.
    pub known: bool,
}

impl Marker {
    /// Its colour under the colour-blind palette the player chose (`render::palette`).
    pub fn color(&self) -> u32 {
        crate::render::palette::marker(self, crate::render::hud::prefs().colour_blind)
    }

    /// Its own colour, the palette aside.
    pub fn own_color(&self) -> u32 {
        match self.kind {
            MarkerKind::Hero { .. } => 0xfff4dc,
            MarkerKind::Foe(FoeRank::Plain) => 0xe8382a,
            MarkerKind::Foe(FoeRank::Elite) => 0xeab55e,
            MarkerKind::Foe(FoeRank::Rival) => 0xffd04a,
            MarkerKind::Foe(FoeRank::General) => 0xffc040,
            MarkerKind::Foe(FoeRank::Boss) => 0xf4ece0,
            MarkerKind::Shrine { spent: true, .. } => 0x8a8478,
            MarkerKind::Shrine { angel, .. } => angel.color(),
            MarkerKind::Key => 0xffd84a,
            MarkerKind::RedKey => 0xff2a44,
            MarkerKind::Remedy => 0x4ae070,
            MarkerKind::Potion => 0x9ad1ff,
            MarkerKind::ItemShrine => 0xe8e0ff,
            MarkerKind::Loot => 0xf2c53d,
            MarkerKind::Chest => 0xd09a3a,
            MarkerKind::Gate { open: true } => 0x7af0a0,
            MarkerKind::Gate { open: false } => 0xb06a44,
            MarkerKind::Entry => 0x8ab4d8,
            MarkerKind::Portal { conquered: true, .. } => 0x8a8478,
            MarkerKind::Portal { sin, .. } => sin.color(),
            MarkerKind::HouseDoor { open: true } => 0xff5a2a,
            MarkerKind::HouseDoor { open: false } => 0x6a3a2a,
            MarkerKind::PitDoor { open: true } => 0xff8a3a,
            MarkerKind::PitDoor { open: false } => 0x6a6460,
            MarkerKind::Platform { .. } => 0xd7ccb8,
            MarkerKind::Lamp => 0xffb84a,
            MarkerKind::Instrument { broken: true, .. } => 0x6a6a72,
            MarkerKind::Instrument { .. } => 0xffc040,
            MarkerKind::Post => 0xeab55e,
            MarkerKind::Cage { open: true } => 0x7af0a0,
            MarkerKind::Cage { open: false } => 0xc8b89a,
        }
    }
}

/// Everything to mark on the map, from the game as it stands: the places first, then the foes, the
/// hero last (drawn in this order, he is on top).
pub fn markers(game: &Game) -> Vec<Marker> {
    let fog = &game.explore;
    let seen = |p: P| fog.seen(p.x, p.y);
    let mut out = Vec::new();
    let mut put = |kind: MarkerKind, at: V, known: bool| out.push(Marker { kind, at, known });
    let g = &game.level.grid;

    // The keys and remedies lying in the level.
    for (x, y, t) in g.cells() {
        match t {
            Tile::Nodule => put(MarkerKind::Key, P::new(x, y).v(), fog.seen(x, y)),
            Tile::Charge => put(MarkerKind::Remedy, P::new(x, y).v(), fog.seen(x, y)),
            _ => {}
        }
    }
    if let Some(k) = game.red_key {
        put(MarkerKind::RedKey, k.v(), seen(k));
    }
    for p in game.potions.iter().filter(|p| !p.taken) {
        put(MarkerKind::Potion, p.tile.v(), seen(p.tile));
    }
    // The archangels' shrines (and the item shrines of the first dungeon).
    let spent = game.angels_spent();
    for d in &game.diamonds {
        match d.kind {
            DiamondKind::Angel => {
                let angel = ARCHANGELS[d.index % ARCHANGELS.len()];
                put(MarkerKind::Shrine { angel, spent: !d.active || spent[angel.index()] }, d.tile.v(), seen(d.tile));
            }
            DiamondKind::Item if d.active => put(MarkerKind::ItemShrine, d.tile.v(), seen(d.tile)),
            DiamondKind::Item => {}
        }
    }
    // The vaults' gold, gems, chests and what the chests threw out.
    if let Some(v) = &game.vault {
        for l in v.loot.iter().filter(|l| !l.taken) {
            let kind = if l.kind == crate::logic::vault::LootKind::Chest { MarkerKind::Chest } else { MarkerKind::Loot };
            put(kind, l.at.v(), seen(l.at));
        }
        for d in v.drops.iter().filter(|d| !d.taken) {
            put(MarkerKind::Loot, d.at.v(), seen(d.at));
        }
        put(MarkerKind::Entry, v.entry_gate.v(), seen(v.entry_gate));
    }
    if let Some(r) = &game.run {
        let e = r.base().entry.tile;
        put(MarkerKind::Entry, e.v(), seen(e));
    }
    // The gate, and in a realm the old man's lamp toward its far portal (always shown, always true).
    if let Some(gate) = game.gate {
        put(MarkerKind::Gate { open: game.gate_open }, gate.v(), seen(gate));
        if game.in_vault() || game.in_run() {
            put(MarkerKind::Lamp, gate.v(), true);
        }
    }
    // The hub's portals, and the House's door.
    if let Some(h) = &game.hub {
        for gate in &h.gates {
            let conquered = game.sins_cleared & gate.sin.bit() != 0;
            put(MarkerKind::Portal { sin: gate.sin, conquered }, gate.tile.v(), true);
        }
        let door = crate::logic::hub::house_door();
        put(MarkerKind::HouseDoor { open: h.seal >= 1.0 }, door.v(), true);
    }
    // The Pit's doors and its platform.
    if let Some(p) = &game.pit {
        for (i, d) in crate::logic::pit::doors().iter().enumerate() {
            put(MarkerKind::PitDoor { open: p.doors[i] > 0.0 }, d.tile.v(), true);
        }
        let r = crate::logic::pit::platform();
        let (cx, cy) = ((r.x0 + r.x1) as f64 / 2.0, (r.y0 + r.y1) as f64 / 2.0);
        put(MarkerKind::Platform { half: crate::logic::pit::PLATFORM as f32 / 2.0 }, V::new(cx, cy), true);
    }
    // The cage in the lair.
    if let Some(l) = &game.lair {
        let (x, y) = crate::logic::lair::CAGE_MIDDLE;
        put(MarkerKind::Cage { open: l.cage.broken || l.cage.door > 0.0 }, V::new(x, y), true);
    }
    // The general's instruments and his post (known once seen, or marked on a caught herald's map).
    let general = game.general.as_ref();
    if let Some(r) = general {
        for (k, i) in r.command.instruments.iter().enumerate() {
            put(MarkerKind::Instrument { order: k, broken: i.broken }, i.tile.v(), seen(i.tile));
        }
        if !r.command.slain {
            put(MarkerKind::Post, r.posts.post.v(), r.post_known || seen(r.posts.post));
        }
    }

    // The foes: those in his sight (or near, where the level is revealed whole); a boss always.
    let hp = game.hero.pos();
    let near = |at: V| {
        let t = P::new(at.x.round() as i32, at.y.round() as i32);
        fog.visible(t.x, t.y) || (fog.whole() && crate::logic::types::dist_sq(at, hp) <= FOE_RANGE * FOE_RANGE)
    };
    let boss_id = game.court.as_ref().map(|c| c.prince).or(game.lair.as_ref().map(|l| l.krampus));
    let general_id = general.and_then(|r| r.id);
    for e in &game.enemies {
        if matches!(e.mode(), EnemyMode::Eaten | EnemyMode::Nest) || e.is_ally() {
            continue;
        }
        let rank = match e.duel.as_deref() {
            Some(d) if !d.alive() => continue,
            Some(d) if Some(d.id) == boss_id || d.arch == crate::logic::combat::Archetype::Prince => FoeRank::Boss,
            Some(d) if Some(d.id) == general_id => FoeRank::General,
            Some(d) if game.rival_of(d.id).is_some() => FoeRank::Rival,
            Some(d) if game.leads_a_pack(d.id) => FoeRank::Elite,
            _ => FoeRank::Plain,
        };
        let at = e.pos();
        put(MarkerKind::Foe(rank), at, rank == FoeRank::Boss || near(at));
    }
    if let Some(b) = game.boss.as_ref().filter(|b| b.alive()) {
        put(MarkerKind::Foe(FoeRank::Boss), b.pos, true);
    }
    put(MarkerKind::Hero { heading: game.hero.heading as f32 }, hp, true);
    out
}

// ---------------------------------------------------------------------- the debug picture

/// A small canvas for the debug picture.
struct Canvas {
    w: u32,
    h: u32,
    px: Vec<u8>,
}

impl Canvas {
    fn blend(&mut self, x: i32, y: i32, c: u32, a: f32) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 || a <= 0.0 {
            return;
        }
        let o = ((y as u32 * self.w + x as u32) * 4) as usize;
        for (k, s) in [16u32, 8, 0].into_iter().enumerate() {
            let v = ((c >> s) & 0xff) as f32;
            self.px[o + k] = (self.px[o + k] as f32 + (v - self.px[o + k] as f32) * a.min(1.0)).round() as u8;
        }
        self.px[o + 3] = 255;
    }

    /// A filled disc, its edge smoothed.
    fn disc(&mut self, cx: f32, cy: f32, r: f32, c: u32, a: f32) {
        let (x0, x1, y0, y1) = ((cx - r - 1.0) as i32, (cx + r + 1.0) as i32, (cy - r - 1.0) as i32, (cy + r + 1.0) as i32);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
                self.blend(x, y, c, a * (r - d + 0.5).clamp(0.0, 1.0));
            }
        }
    }

    fn ring(&mut self, cx: f32, cy: f32, r: f32, width: f32, c: u32, a: f32) {
        let (x0, x1, y0, y1) = ((cx - r - width) as i32, (cx + r + width) as i32 + 1, (cy - r - width) as i32, (cy + r + width) as i32 + 1);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
                self.blend(x, y, c, a * (width / 2.0 - (d - r).abs() + 0.5).clamp(0.0, 1.0));
            }
        }
    }

    /// A filled convex polygon (its points in order), its edge smoothed a little.
    fn poly(&mut self, pts: &[(f32, f32)], c: u32, a: f32) {
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for &(x, y) in pts {
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
        }
        for y in (y0 as i32 - 1)..=(y1 as i32 + 1) {
            for x in (x0 as i32 - 1)..=(x1 as i32 + 1) {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                // The least signed distance inside every edge (the points wound either way).
                let mut inside = f32::MAX;
                let mut wind = 0.0;
                for k in 0..pts.len() {
                    let (ax, ay) = pts[k];
                    let (bx, by) = pts[(k + 1) % pts.len()];
                    wind += (bx - ax) * (by + ay);
                }
                for k in 0..pts.len() {
                    let (ax, ay) = pts[k];
                    let (bx, by) = pts[(k + 1) % pts.len()];
                    let (ex, ey) = (bx - ax, by - ay);
                    let len = (ex * ex + ey * ey).sqrt().max(1e-4);
                    let mut d = ((px - ax) * ey - (py - ay) * ex) / len;
                    if wind < 0.0 {
                        d = -d;
                    }
                    inside = inside.min(d);
                }
                self.blend(x, y, c, a * (inside + 0.5).clamp(0.0, 1.0));
            }
        }
    }

    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: u32, a: f32) {
        self.poly(&[(x, y), (x + w, y), (x + w, y + h), (x, y + h)], c, a);
    }

    fn line(&mut self, a: (f32, f32), b: (f32, f32), width: f32, c: u32, alpha: f32) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l = (dx * dx + dy * dy).sqrt().max(1e-4);
        let (nx, ny) = (-dy / l * width / 2.0, dx / l * width / 2.0);
        self.poly(&[(a.0 + nx, a.1 + ny), (b.0 + nx, b.1 + ny), (b.0 - nx, b.1 - ny), (a.0 - nx, a.1 - ny)], c, alpha);
    }

    fn diamond(&mut self, cx: f32, cy: f32, r: f32, c: u32, a: f32) {
        self.poly(&[(cx, cy - r), (cx + r, cy), (cx, cy + r), (cx - r, cy)], c, a);
    }
}

const SHADOW: u32 = 0x0a0c0c;

/// Draws one marker on the canvas, `t` pixels a tile.
fn draw_marker(cv: &mut Canvas, m: &Marker, x: f32, y: f32, t: f32) {
    let col = m.color();
    match m.kind {
        MarkerKind::Hero { heading } => {
            let (ux, uy) = (heading.cos(), heading.sin());
            let (px, py) = (-uy, ux);
            let r = t * 0.95;
            let tip = (x + ux * r, y + uy * r);
            let back = (x - ux * r * 0.6, y - uy * r * 0.6);
            let l = (back.0 + px * r * 0.65 - ux * r * 0.15, back.1 + py * r * 0.65 - uy * r * 0.15);
            let rr = (back.0 - px * r * 0.65 - ux * r * 0.15, back.1 - py * r * 0.65 - uy * r * 0.15);
            let grow = |p: (f32, f32), k: f32| (x + (p.0 - x) * k, y + (p.1 - y) * k);
            cv.disc(x, y, r * 1.2, 0xfff4dc, 0.16);
            cv.poly(&[grow(tip, 1.3), grow(l, 1.3), grow(back, 1.3), grow(rr, 1.3)], SHADOW, 0.9);
            cv.poly(&[tip, l, back, rr], col, 1.0);
            cv.poly(&[tip, l, back], 0xeab55e, 1.0);
        }
        MarkerKind::Foe(rank) => {
            let r = match rank {
                FoeRank::Plain => t * 0.38,
                FoeRank::Elite | FoeRank::Rival => t * 0.48,
                FoeRank::General => t * 0.6,
                FoeRank::Boss => t * 0.85,
            };
            match rank {
                FoeRank::General => {
                    // A crown: a band and three points.
                    let (w, h) = (r * 2.0, r * 1.5);
                    let (x0, y0) = (x - w / 2.0, y - h / 2.0);
                    let crown = [
                        (x0, y0 + h),
                        (x0, y0),
                        (x0 + w * 0.25, y0 + h * 0.45),
                        (x, y0 - h * 0.1),
                        (x0 + w * 0.75, y0 + h * 0.45),
                        (x0 + w, y0),
                        (x0 + w, y0 + h),
                    ];
                    cv.disc(x, y, r * 1.25, SHADOW, 0.85);
                    // (Not convex: drawn as its band and three triangles.)
                    cv.rect(x0, y0 + h * 0.45, w, h * 0.55, col, 1.0);
                    cv.poly(&[crown[1], crown[2], (x0, y0 + h * 0.5)], col, 1.0);
                    cv.poly(&[(x0 + w * 0.25, y0 + h * 0.5), crown[3], (x0 + w * 0.75, y0 + h * 0.5)], col, 1.0);
                    cv.poly(&[crown[4], crown[5], (x0 + w, y0 + h * 0.5)], col, 1.0);
                }
                FoeRank::Boss => {
                    // A skull: the dome, the jaw, two eyes.
                    cv.disc(x, y, r * 1.2, SHADOW, 0.85);
                    cv.disc(x, y - r * 0.15, r * 0.85, col, 1.0);
                    cv.rect(x - r * 0.45, y + r * 0.3, r * 0.9, r * 0.5, col, 1.0);
                    cv.disc(x - r * 0.33, y - r * 0.12, r * 0.22, SHADOW, 1.0);
                    cv.disc(x + r * 0.33, y - r * 0.12, r * 0.22, SHADOW, 1.0);
                    cv.rect(x - r * 0.08, y + r * 0.3, r * 0.16, r * 0.5, SHADOW, 0.8);
                }
                _ => {
                    cv.disc(x, y, r + t * 0.14, SHADOW, 0.85);
                    cv.disc(x, y, r, col, 1.0);
                    if rank == FoeRank::Rival {
                        cv.ring(x, y, r * 0.55, (t * 0.16).max(1.0), 0xc8202a, 1.0);
                    }
                }
            }
        }
        MarkerKind::Shrine { spent, .. } => {
            let r = t * 0.62;
            if !spent {
                cv.disc(x, y, r * 2.0, col, 0.18);
                cv.ring(x, y, r * 1.45, (t * 0.12).max(1.0), col, 0.8);
            }
            cv.diamond(x, y, r * 1.2, SHADOW, 0.9);
            cv.diamond(x, y, r, col, 1.0);
            cv.diamond(x, y, r * 0.4, 0xffffff, if spent { 0.2 } else { 0.8 });
        }
        MarkerKind::Key | MarkerKind::RedKey => {
            cv.diamond(x, y, t * 0.56, SHADOW, 0.85);
            cv.diamond(x, y, t * 0.42, col, 1.0);
        }
        MarkerKind::Remedy => {
            let (a, b) = (t * 0.36, t * 0.12);
            cv.rect(x - a - 1.0, y - b - 1.0, 2.0 * a + 2.0, 2.0 * b + 2.0, SHADOW, 0.8);
            cv.rect(x - b - 1.0, y - a - 1.0, 2.0 * b + 2.0, 2.0 * a + 2.0, SHADOW, 0.8);
            cv.rect(x - a, y - b, 2.0 * a, 2.0 * b, col, 1.0);
            cv.rect(x - b, y - a, 2.0 * b, 2.0 * a, col, 1.0);
        }
        MarkerKind::Potion | MarkerKind::Loot => {
            cv.disc(x, y, t * 0.28, SHADOW, 0.8);
            cv.disc(x, y, t * 0.2, col, 1.0);
        }
        MarkerKind::ItemShrine => {
            cv.diamond(x, y, t * 0.5, SHADOW, 0.85);
            cv.diamond(x, y, t * 0.38, col, 1.0);
        }
        MarkerKind::Chest => {
            let (w, h) = (t * 0.8, t * 0.55);
            cv.rect(x - w / 2.0 - 1.5, y - h / 2.0 - 1.5, w + 3.0, h + 3.0, SHADOW, 0.9);
            cv.rect(x - w / 2.0, y - h / 2.0, w, h, col, 1.0);
            cv.rect(x - w / 2.0, y - h * 0.05, w, h * 0.12, 0x5a3a1a, 1.0);
        }
        MarkerKind::Gate { open } => {
            let (w, h) = (t * 1.3, t * 1.1);
            cv.rect(x - w / 2.0 - 1.5, y - h / 2.0 - 1.5, w + 3.0, h + 3.0, SHADOW, 0.9);
            cv.rect(x - w / 2.0, y - h / 2.0, w, h, col, 1.0);
            if !open {
                cv.rect(x - t * 0.06, y - h / 2.0, t * 0.12, h, SHADOW, 0.8);
            } else {
                cv.rect(x - w * 0.3, y - h * 0.3, w * 0.6, h * 0.6, 0x0a1a10, 0.8);
            }
        }
        MarkerKind::Entry => {
            cv.ring(x, y, t * 0.5, (t * 0.18).max(1.0), SHADOW, 0.8);
            cv.ring(x, y, t * 0.5, (t * 0.1).max(1.0), col, 1.0);
        }
        MarkerKind::Portal { conquered, sin } => {
            cv.disc(x, y, t * 1.0, SHADOW, 0.85);
            cv.disc(x, y, t * 0.82, col, 1.0);
            cv.disc(x, y, t * 0.5, if conquered { 0x3a3632 } else { 0x140c0a }, 1.0);
            if conquered {
                cv.ring(x, y, t * 0.82, (t * 0.14).max(1.0), sin.color(), 1.0);
            }
        }
        MarkerKind::HouseDoor { open } => {
            let (w, h) = (t * 1.6, t * 1.4);
            cv.rect(x - w / 2.0 - 1.5, y - h / 2.0 - 1.5, w + 3.0, h + 3.0, SHADOW, 0.9);
            cv.rect(x - w / 2.0, y - h / 2.0, w, h, col, 1.0);
            cv.rect(x - w * 0.25, y - h * 0.2, w * 0.5, h * 0.7, if open { 0x1a0806 } else { 0x2a1a14 }, 1.0);
        }
        MarkerKind::PitDoor { open } => {
            let r = t * 0.55;
            cv.rect(x - r - 1.0, y - r - 1.0, 2.0 * r + 2.0, 2.0 * r + 2.0, SHADOW, 0.9);
            cv.rect(x - r, y - r, 2.0 * r, 2.0 * r, col, 1.0);
            if open {
                cv.disc(x, y, r * 1.8, col, 0.2);
            }
        }
        MarkerKind::Platform { half } => {
            let r = half * t;
            let w = (t * 0.2).max(1.5);
            cv.line((x - r, y - r), (x + r, y - r), w, col, 0.7);
            cv.line((x + r, y - r), (x + r, y + r), w, col, 0.7);
            cv.line((x + r, y + r), (x - r, y + r), w, col, 0.7);
            cv.line((x - r, y + r), (x - r, y - r), w, col, 0.7);
        }
        MarkerKind::Lamp => {
            // The old man's lamp: an amber diamond's outline round a flame, and a soft glow.
            let s = t * 0.9;
            cv.disc(x, y, s * 1.6, col, 0.15);
            for (a, b) in [((x, y - s), (x + s, y)), ((x + s, y), (x, y + s)), ((x, y + s), (x - s, y)), ((x - s, y), (x, y - s))] {
                cv.line(a, b, (t * 0.32).max(2.0), SHADOW, 0.7);
                cv.line(a, b, (t * 0.16).max(1.2), col, 1.0);
            }
            cv.disc(x, y, s * 0.3, col, 1.0);
        }
        MarkerKind::Instrument { order, broken } => {
            let r = t * 0.48;
            cv.disc(x, y, r * 1.3, SHADOW, 0.85);
            cv.disc(x, y, r, col, 1.0);
            cv.disc(x, y, r * 0.6, 0x14100c, 1.0);
            for k in 0..=order {
                cv.disc(x - r * 0.4 + k as f32 * r * 0.4, y, r * 0.14, col, 1.0);
            }
            if broken {
                cv.line((x - r, y - r), (x + r, y + r), (t * 0.12).max(1.0), 0xc8382a, 1.0);
            }
        }
        MarkerKind::Post => {
            let (w, h) = (t * 0.9, t * 1.2);
            cv.rect(x - w / 2.0 - 1.0, y - h / 2.0 - 1.0, w + 2.0, h + 2.0, SHADOW, 0.85);
            cv.poly(
                &[
                    (x - w / 2.0, y - h / 2.0),
                    (x + w / 2.0, y - h / 2.0),
                    (x + w / 2.0, y + h / 2.0),
                    (x, y + h * 0.2),
                    (x - w / 2.0, y + h / 2.0),
                ],
                col,
                1.0,
            );
        }
        MarkerKind::Cage { open } => {
            let (w, h) = (t * 2.6, t * 2.6);
            cv.rect(x - w / 2.0, y - h / 2.0, w, h, SHADOW, 0.6);
            for k in 0..5 {
                let bx = x - w / 2.0 + w * k as f32 / 4.0;
                cv.line((bx, y - h / 2.0), (bx, y + h / 2.0), (t * 0.14).max(1.0), col, if open && k == 2 { 0.2 } else { 1.0 });
            }
        }
    }
}

/// The map as the minimap would show it, drawn naively on the CPU: the image over a dark ground,
/// enlarged to be looked at, every known marker on top. Width, height and RGBA.
pub fn debug_picture(game: &Game) -> (u32, u32, Vec<u8>) {
    let img = MapImage::build(game);
    let k = (1200 / img.w.max(img.h)).clamp(1, 4);
    let margin = 12 * k;
    let (w, h) = (img.w * k + 2 * margin, img.h * k + 2 * margin);
    let mut cv = Canvas { w, h, px: vec![0; (w * h * 4) as usize] };
    // The panel behind the map: ink, a faint grid every eight tiles so its scale reads.
    for y in 0..h {
        for x in 0..w {
            let o = ((y * w + x) * 4) as usize;
            let grid = img.px_per_tile * k * 8;
            let on = x >= margin && y >= margin && ((x - margin).is_multiple_of(grid) || (y - margin).is_multiple_of(grid));
            let c: [u8; 3] = if on { [26, 32, 32] } else { [12, 17, 17] };
            cv.px[o..o + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    for y in 0..img.h * k {
        for x in 0..img.w * k {
            let p = img.pixel(x / k, y / k);
            if p[3] > 0 {
                let c = (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32;
                cv.blend((x + margin) as i32, (y + margin) as i32, c, p[3] as f32 / 255.0);
            }
        }
    }
    let t = (img.px_per_tile * k) as f32;
    for m in markers(game).iter().filter(|m| m.known) {
        let (x, y) = img.to_pixel(m.at);
        draw_marker(&mut cv, m, x * k as f32 + margin as f32, y * k as f32 + margin as f32, t);
    }
    (w, h, cv.px)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Stage;
    use crate::logic::sins::Sin;

    fn at(stage: Stage) -> Game {
        let mut g = Game::new();
        g.map_jump(stage);
        g
    }

    /// The colour of a tile's middle pixel, before the fog.
    fn ground(img: &MapImage, x: i32, y: i32) -> u32 {
        let s = img.px_per_tile as i32;
        let o = (((y * s + s / 2) as u32 * img.w + (x * s + s / 2) as u32) * 4) as usize;
        (img.base[o] as u32) << 16 | (img.base[o + 1] as u32) << 8 | img.base[o + 2] as u32
    }

    fn near(a: u32, b: u32, tol: i32) -> bool {
        [16, 8, 0].iter().all(|&s| (((a >> s) & 0xff) as i32 - ((b >> s) & 0xff) as i32).abs() <= tol)
    }

    /// The image of the level the game is in: its size, its look, its floor and walls in that
    /// look's colours, and the fog as the level's kind wants it.
    fn check(g: &Game, style: Style, whole: bool) -> MapImage {
        let img = MapImage::build(g);
        let (tw, th) = (g.level.grid.width, g.level.grid.height);
        assert_eq!(img.style, style, "{}", g.level.data.name);
        assert_eq!((img.w, img.h), (tw as u32 * img.px_per_tile, th as u32 * img.px_per_tile));
        assert!(img.w <= MAX_SIDE && img.h <= MAX_SIDE && img.px_per_tile >= 2);
        assert_eq!(img.rgba.len(), (img.w * img.h * 4) as usize);
        let pal = Palette::of(style);
        // His own tile is floor in the look's colour (lighter as it stands higher), and seen.
        let hp = g.hero.pos();
        let (hx, hy) = (hp.x.round() as i32, hp.y.round() as i32);
        let c = img.class_at(hx, hy);
        assert!(matches!(c, class::FLOOR | class::SPECIAL | class::RAMP | class::BRIDGE), "{}: he stands on class {c}", g.level.data.name);
        if c == class::FLOOR {
            let lift = 1.0 + LIFT * g.level.grid.elevation(hx, hy).clamp(0, 15) as f32;
            assert!(
                near(ground(&img, hx, hy), scale(pal.floor, lift), 14),
                "{}: floor {:06x} against {:06x}",
                g.level.data.name,
                ground(&img, hx, hy),
                pal.floor
            );
        }
        assert_eq!(img.pixel(img.to_pixel(hp).0 as u32, img.to_pixel(hp).1 as u32)[3], 255, "his tile is seen");
        // Some wall in the wall's colour, and its outline where it meets the floor (the gym has
        // none: its platforms stand in lava).
        let s = img.px_per_tile as i32;
        if g.level.grid.count(Tile::Rock) > 0 {
            let walls: Vec<(i32, i32)> =
                (0..th).flat_map(|y| (0..tw).map(move |x| (x, y))).filter(|&(x, y)| img.class_at(x, y) == class::WALL).collect();
            assert!(!walls.is_empty(), "{}: no walls", g.level.data.name);
            assert!(walls.iter().any(|&(x, y)| near(ground(&img, x, y), pal.wall, 14)));
            let outlined = walls.iter().any(|&(x, y)| {
                img.class_at(x + 1, y) == class::FLOOR && {
                    let o = (((y * s + s / 2) as u32 * img.w + ((x + 1) * s - 1) as u32) * 4) as usize;
                    (img.base[o] as u32) << 16 | (img.base[o + 1] as u32) << 8 | img.base[o + 2] as u32 == pal.edge
                }
            });
            assert!(outlined, "{}: no wall outlined", g.level.data.name);
        }
        // The fog: the whole level opaque where it is drawn, or only round him.
        let drawn: Vec<(i32, i32)> =
            (0..th).flat_map(|y| (0..tw).map(move |x| (x, y))).filter(|&(x, y)| img.class_at(x, y) != class::VOID).collect();
        let opaque = drawn.iter().filter(|&&(x, y)| img.pixel((x * s + s / 2) as u32, (y * s + s / 2) as u32)[3] == 255).count();
        if whole {
            assert_eq!(opaque, drawn.len(), "{}: revealed whole", g.level.data.name);
        } else {
            assert!(opaque > 20 && opaque < drawn.len() / 2, "{}: {opaque} of {} tiles shown", g.level.data.name, drawn.len());
        }
        img
    }

    #[test]
    fn the_image_has_the_size_and_colours_of_every_kind_of_level() {
        // The garden (Dungeon I), explored.
        let g = at(Stage::Dungeon(0));
        let img = check(&g, Style::Garden, false);
        // Its river and bridges are drawn as water and planks.
        let (tw, th) = (g.level.grid.width, g.level.grid.height);
        let classes: Vec<u8> = (0..th).flat_map(|y| (0..tw).map(move |x| (x, y))).map(|(x, y)| img.class_at(x, y)).collect();
        assert!(classes.contains(&class::WATER) && classes.contains(&class::BRIDGE));
        // The stone dungeons after it.
        check(&at(Stage::Dungeon(1)), Style::Cisterns, false);
        check(&at(Stage::Dungeon(2)), Style::Crypt, false);
        // The Cyclops' meadow, revealed whole.
        check(&at(Stage::Arena(0)), Style::Meadow, true);
        // The vaults, explored.
        check(&at(Stage::Realm(Sin::Greed)), Style::Vault, false);
        // A realm after them.
        check(&at(Stage::Realm(Sin::Envy)), Style::Realm(Sin::Envy), false);
        check(&at(Stage::Realm(Sin::Sloth)), Style::Realm(Sin::Sloth), false);
        // The hub: its lava drawn hot.
        let hub = at(Stage::Hub);
        let img = check(&hub, Style::Hub, true);
        let (tw, th) = (hub.level.grid.width, hub.level.grid.height);
        assert!((0..th).any(|y| (0..tw).any(|x| img.class_at(x, y) == class::HOLE)));
        assert_eq!(img.palette.hole, Hole::Lava);
        // A court, the gym, the Pit (its platform marked out) and the lair: revealed whole.
        check(&at(Stage::Court(Sin::Greed)), Style::Court(Sin::Greed), true);
        check(&at(Stage::Court(Sin::Wrath)), Style::Court(Sin::Wrath), true);
        check(&at(Stage::Gym), Style::Gym, true);
        let pit = at(Stage::Pit(1));
        let img = check(&pit, Style::Pit, true);
        let mid = crate::logic::pit::SIZE / 2;
        assert_eq!(img.class_at(mid - 2, mid - 2), class::SPECIAL);
        check(&at(Stage::Lair), Style::Lair, true);
        // Every look its own floor.
        let floors: Vec<u32> = [
            Style::Garden,
            Style::Crypt,
            Style::Meadow,
            Style::Hub,
            Style::Vault,
            Style::Realm(Sin::Envy),
            Style::Court(Sin::Greed),
            Style::Pit,
            Style::Lair,
        ]
        .iter()
        .map(|&s| Palette::of(s).floor)
        .collect();
        for (i, a) in floors.iter().enumerate() {
            for b in &floors[i + 1..] {
                assert!(!near(*a, *b, 6), "{a:06x} and {b:06x}");
            }
        }
    }

    #[test]
    fn a_patch_covers_only_the_tiles_that_changed() {
        let mut g = at(Stage::Dungeon(0));
        let mut img = MapImage::build(&g);
        assert!(img.take_dirty().is_some());
        // Nothing changed: nothing to send.
        img.sync(&g);
        assert_eq!(img.take_dirty(), None);
        // He walks a few tiles: only the tiles about his new sight are sent, and what they show is
        // what a fresh build shows.
        let hp = g.hero.pos();
        let start = P::new(hp.x.round() as i32, hp.y.round() as i32);
        let to = [P::new(start.x + 3, start.y), P::new(start.x - 3, start.y), P::new(start.x, start.y + 3), P::new(start.x, start.y - 3)]
            .into_iter()
            .find(|p| g.level.grid.is_walkable(p.x, p.y, false))
            .expect("somewhere to walk");
        let before = g.explore.revealed();
        g.explore.observe(&g.level.grid, to.v());
        let fresh: Vec<u32> = g.explore.revealed_since(before).to_vec();
        assert!(!fresh.is_empty());
        img.sync(&g);
        let d = img.take_dirty().expect("a patch");
        let s = img.px_per_tile;
        let tw = g.level.grid.width;
        let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
        for &i in &fresh {
            let (x, y) = ((i as i32 % tw) as u32, (i as i32 / tw) as u32);
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
        }
        // The new tiles and two about them (the soft edge, and the clear rock that may now count as
        // seen), clamped to the image.
        let want = PixelRect {
            x: x0.saturating_sub(2) * s,
            y: y0.saturating_sub(2) * s,
            w: ((x1 + 3).min(tw as u32) - x0.saturating_sub(2)) * s,
            h: ((y1 + 3).min(g.level.grid.height as u32) - y0.saturating_sub(2)) * s,
        };
        assert_eq!(d, want);
        assert!(d.w * d.h < img.w * img.h / 4, "a patch, not the whole image");
        assert_eq!(img.rgba, MapImage::build(&g).rgba, "patched as built");
        // Ground that changes (a wall opened) is found by the scan and patched round that tile.
        let wall = (0..g.level.grid.height)
            .flat_map(|y| (0..tw).map(move |x| (x, y)))
            .find(|&(x, y)| img.class_at(x, y) == class::WALL && x > 2 && y > 2 && x < tw - 3 && y < g.level.grid.height - 3)
            .unwrap();
        g.level.grid.set(wall.0, wall.1, Tile::Open);
        let mut found = None;
        for _ in 0..g.level.grid.height {
            img.sync(&g);
            if let Some(d) = img.take_dirty() {
                found = Some(d);
                break;
            }
        }
        let d = found.expect("the change is found");
        assert!(d.x <= wall.0 as u32 * s && d.x + d.w >= (wall.0 as u32 + 1) * s && d.w <= 5 * s && d.h <= 5 * s, "{d:?}");
        // A new level: built afresh.
        g.map_jump(Stage::Hub);
        img.sync(&g);
        assert_eq!((img.style, img.dirty.map(|d| (d.w, d.h))), (Style::Hub, Some((img.w, img.h))));
    }

    #[test]
    fn the_markers_include_the_shrines_the_gate_and_the_foes_in_the_garden() {
        let mut g = at(Stage::Dungeon(0));
        let m = markers(&g);
        let shrines = m.iter().filter(|m| matches!(m.kind, MarkerKind::Shrine { .. })).count();
        assert_eq!(shrines, g.level.angel_shrines.len());
        assert!(shrines > 0);
        assert!(m.iter().any(|m| matches!(m.kind, MarkerKind::Gate { open: false })));
        assert_eq!(m.iter().filter(|m| m.kind == MarkerKind::Key).count() as u32, g.keys_total);
        let hero = m.last().unwrap();
        assert!(matches!(hero.kind, MarkerKind::Hero { .. }) && hero.known);
        // The foes: once out of the hollow, each is marked, and known only while in his sight.
        for e in g.enemies.iter_mut() {
            e.b.mode = EnemyMode::Chase;
        }
        let near = g.hero.pos();
        let far = (0..g.level.grid.height)
            .flat_map(|y| (0..g.level.grid.width).map(move |x| P::new(x, y)))
            .find(|p| g.level.grid.is_walkable(p.x, p.y, false) && !g.explore.seen(p.x, p.y))
            .unwrap();
        g.enemies[0].m.tile = P::new(near.x.round() as i32, near.y.round() as i32);
        g.enemies[1].m.tile = far;
        let m = markers(&g);
        let foes: Vec<&Marker> = m.iter().filter(|m| matches!(m.kind, MarkerKind::Foe(_))).collect();
        assert_eq!(foes.len(), g.enemies.len());
        assert!(foes.iter().all(|f| f.kind == MarkerKind::Foe(FoeRank::Plain)));
        assert!(foes[0].known && !foes[1].known);
        // The shrines and the gate are known once seen: those far away are not yet.
        let unseen_shrine = m.iter().any(|m| matches!(m.kind, MarkerKind::Shrine { .. }) && !m.known);
        assert!(unseen_shrine);
        // In a realm, the old man's lamp always shows where the far portal is.
        let r = at(Stage::Realm(Sin::Greed));
        let m = markers(&r);
        let lamp = m.iter().find(|m| m.kind == MarkerKind::Lamp).expect("the lamp");
        assert!(lamp.known && lamp.at == r.gate.unwrap().v());
        assert!(m.iter().any(|m| m.kind == MarkerKind::Chest));
        // The hub's seven portals, the Pit's eight doors and platform, a court's prince.
        let hub = markers(&at(Stage::Hub));
        assert_eq!(hub.iter().filter(|m| matches!(m.kind, MarkerKind::Portal { .. })).count(), 7);
        let pit = markers(&at(Stage::Pit(1)));
        assert_eq!(pit.iter().filter(|m| matches!(m.kind, MarkerKind::PitDoor { .. })).count(), 8);
        assert!(pit.iter().any(|m| matches!(m.kind, MarkerKind::Platform { .. })));
        let court = markers(&at(Stage::Court(Sin::Greed)));
        assert!(court.iter().any(|m| m.kind == MarkerKind::Foe(FoeRank::Boss) && m.known));
    }

    #[test]
    fn the_fog_never_changes_the_game() {
        use crate::app::STEP;
        use crate::capture::parse_script;
        use crate::driver::Driver;
        // The expert bot fights through the vaults and walks the garden, once with the fog and once
        // without: everything the game did is the same.
        for script in ["0.1:realm:4,0.5:bot:2", "0.1:dungeon:0,0.15:camera,2.3:+up,4:-up,4:+left,6:-left,6:+down,9:-down"] {
            let play = |fog: bool| {
                let mut g = Game::new();
                g.reseed(1);
                g.tour_pending = false;
                g.explore.off = !fog;
                let mut d = Driver::new(parse_script(script));
                d.run(&mut g, 30.0);
                let foes: Vec<(i64, i64)> = g.enemies.iter().map(|e| ((e.pos().x * 1e6) as i64, (e.pos().y * 1e6) as i64)).collect();
                (d.deeds(&g), g.hero.pos(), g.hero.hp, g.keys_got, g.score.score, foes, g.screen, g.explore.revealed())
            };
            let (with, without) = (play(true), play(false));
            assert!(with.7 > without.7 + 50, "{script}: the fog lifted in one only ({} and {})", with.7, without.7);
            assert_eq!(with.0, without.0, "{script}: the same deeds");
            assert_eq!((with.1, with.2, with.3, with.4), (without.1, without.2, without.3, without.4), "{script}");
            assert_eq!((&with.5, with.6), (&without.5, without.6), "{script}: the foes where they were");
            assert!(with.0.steps as f64 >= 30.0 / STEP - 1.0);
        }
    }

    #[test]
    fn building_the_largest_map_and_a_step_of_the_fog_are_quick() {
        // Every kind of level: the largest is the one timed.
        let stages = [
            Stage::Dungeon(0),
            Stage::Dungeon(2),
            Stage::Hub,
            Stage::Realm(Sin::Greed),
            Stage::Realm(Sin::Wrath),
            Stage::Realm(Sin::Gluttony),
            Stage::Realm(Sin::Sloth),
        ];
        let mut g = stages.iter().map(|&s| at(s)).max_by_key(|g| g.level.grid.width * g.level.grid.height).unwrap();
        let (tw, th) = (g.level.grid.width, g.level.grid.height);
        let t0 = web_time::Instant::now();
        let img = MapImage::build(&g);
        let build_ms = t0.elapsed().as_secs_f64() * 1e3;
        // A step of the fog: a look from each walkable tile in turn (every one a new tile).
        let walk: Vec<P> = (0..th)
            .flat_map(|y| (0..tw).map(move |x| P::new(x, y)))
            .filter(|p| g.level.grid.is_walkable(p.x, p.y, false))
            .take(400)
            .collect();
        let t0 = web_time::Instant::now();
        for p in &walk {
            g.explore.observe(&g.level.grid, p.v());
        }
        let step_ms = t0.elapsed().as_secs_f64() * 1e3 / walk.len() as f64;
        let mut patched = img.clone();
        let t0 = web_time::Instant::now();
        patched.sync(&g);
        let patch_ms = t0.elapsed().as_secs_f64() * 1e3;
        // A sync with nothing new (the fog still, a few rows of ground looked over): every frame's cost.
        let t0 = web_time::Instant::now();
        for _ in 0..100 {
            patched.sync(&g);
        }
        let idle_ms = t0.elapsed().as_secs_f64() * 1e3 / 100.0;
        let t0 = web_time::Instant::now();
        let n = markers(&g).len();
        let markers_ms = t0.elapsed().as_secs_f64() * 1e3;
        println!(
            "minimap: {} ({tw}x{th} tiles) built as {}x{} at {} px a tile in {build_ms:.2} ms; a step of the fog {:.4} ms; \
             400 looks patched in {patch_ms:.2} ms; an idle sync {idle_ms:.4} ms; {n} markers listed in {markers_ms:.3} ms",
            g.level.data.name, img.w, img.h, img.px_per_tile, step_ms
        );
        assert!(step_ms < 0.1, "a step of the fog took {step_ms} ms");
        assert!(idle_ms < 0.1, "an idle sync took {idle_ms} ms");
    }
}
