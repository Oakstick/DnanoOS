//! The fog of war (UI.md §7): which tiles of the level the hero has seen. They are revealed in a
//! radius round him as he goes, by line of sight (a tile behind a wall stays dark until he comes
//! round it), and remembered while the level lives: kept through a fall and through a reload of the
//! same level, forgotten when another is loaded. The hub, the boss arenas and courts, the gym, the
//! Pit and the lair are small and walled by lava or stone, and stand revealed from the start.
//!
//! Presentation only: nothing in the game reads it (the minimap draws it, `render::minimap`). It
//! keeps a log of the tiles revealed in the order they were, so the map's image can patch exactly
//! those and send only the changed rows.

use super::grid::Grid;
use super::level::{ParsedLevel, Theme};
use super::types::{Tile, P, V};

/// How far he sees, in tiles.
pub const SIGHT: i32 = 9;

/// Looks again at least this often (in looks asked for) while he stands on one tile, so a door that
/// opens before him shows what lies past it.
const LOOK_EVERY: u32 = 20;

#[derive(Clone, Debug, Default)]
pub struct Explore {
    /// Switched off (a test proving the game plays the same without it): nothing is revealed.
    pub off: bool,
    width: i32,
    height: i32,
    seen: Vec<bool>,
    /// In his sight at the last look, and the same as a list (to clear it quickly).
    visible: Vec<bool>,
    in_sight: Vec<u32>,
    /// Every tile revealed since the level began, in order (each once).
    log: Vec<u32>,
    /// Counts the levels loaded: the map's image is built afresh when it changes.
    epoch: u32,
    /// The level's name and layout as a number, to tell a reload of the same level from another.
    signature: u64,
    /// Revealed whole from the start.
    whole: bool,
    /// The tile he last looked from, and the looks asked for since.
    from: Option<P>,
    idle: u32,
    /// The offsets within his sight, nearest first.
    reach: Vec<(i32, i32)>,
}

/// Is a level of this kind revealed whole as it loads (a boss's arena, a court, the hub, the gym,
/// the Pit, the lair)? The estates and the realms are explored.
pub fn revealed_whole(theme: Theme, boss: bool) -> bool {
    boss || matches!(theme, Theme::Hell | Theme::Gym | Theme::Pit | Theme::Lair | Theme::Meadow | Theme::Court(_))
}

/// A level's name and layout (which tiles are rock) as one number.
fn signature(level: &ParsedLevel) -> u64 {
    // FNV-1a: stable, and enough to tell two layouts apart.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |b: u8| {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    };
    for b in level.data.name.bytes() {
        eat(b);
    }
    let g = &level.grid;
    for b in g.width.to_le_bytes().into_iter().chain(g.height.to_le_bytes()) {
        eat(b);
    }
    for (_, _, t) in g.cells() {
        eat((t == Tile::Rock) as u8);
    }
    h
}

impl Explore {
    pub fn new() -> Self {
        let mut reach: Vec<(i32, i32)> = Vec::new();
        for dy in -SIGHT..=SIGHT {
            for dx in -SIGHT..=SIGHT {
                if dx * dx + dy * dy <= SIGHT * SIGHT + SIGHT {
                    reach.push((dx, dy));
                }
            }
        }
        reach.sort_by_key(|&(dx, dy)| dx * dx + dy * dy);
        Self { reach, ..Default::default() }
    }

    /// A level has been loaded: the fog is kept if it is the one he was in (a restart), and
    /// otherwise begins again (all revealed, for the kinds that are).
    pub fn enter(&mut self, level: &ParsedLevel) {
        self.epoch = self.epoch.wrapping_add(1);
        self.from = None;
        self.clear_sight();
        let sig = signature(level);
        let (w, h) = (level.grid.width, level.grid.height);
        if sig == self.signature && w == self.width && h == self.height {
            return;
        }
        self.signature = sig;
        self.width = w;
        self.height = h;
        let n = (w * h) as usize;
        self.whole = revealed_whole(level.data.theme, level.data.boss);
        self.seen = vec![self.whole; n];
        self.visible = vec![false; n];
        self.log.clear();
    }

    /// Looks round from `at` (where he stands, in tiles): every tile in his sight is revealed. Only
    /// when he has come onto another tile (or now and then while he stands), so it costs nothing
    /// while he stands still.
    pub fn observe(&mut self, grid: &Grid, at: V) {
        if self.off || self.reach.is_empty() {
            return;
        }
        if grid.width != self.width || grid.height != self.height {
            // (A level that never passed through `enter`: a test's own grid.)
            self.width = grid.width;
            self.height = grid.height;
            let n = (grid.width * grid.height) as usize;
            self.seen = vec![self.whole; n];
            self.visible = vec![false; n];
            self.in_sight.clear();
            self.log.clear();
        }
        let tile = P::new(at.x.round() as i32, at.y.round() as i32);
        if self.from == Some(tile) && self.idle < LOOK_EVERY {
            self.idle += 1;
            return;
        }
        self.from = Some(tile);
        self.idle = 0;
        self.look(grid, tile);
    }

    fn clear_sight(&mut self) {
        for &i in &self.in_sight {
            if let Some(v) = self.visible.get_mut(i as usize) {
                *v = false;
            }
        }
        self.in_sight.clear();
    }

    fn mark(&mut self, x: i32, y: i32) {
        let i = (y * self.width + x) as usize;
        if !self.visible[i] {
            self.visible[i] = true;
            self.in_sight.push(i as u32);
        }
        if !self.seen[i] {
            self.seen[i] = true;
            self.log.push(i as u32);
        }
    }

    /// Everything in sight of `from`: the floor his eye reaches past no rock (nor between two rocks
    /// meeting at a corner), and the walls about that floor.
    fn look(&mut self, grid: &Grid, from: P) {
        self.clear_sight();
        if !grid.in_bounds(from.x, from.y) {
            return;
        }
        let reach = std::mem::take(&mut self.reach);
        for &(dx, dy) in &reach {
            let (x, y) = (from.x + dx, from.y + dy);
            if grid.in_bounds(x, y) && clear_line(grid, from, P::new(x, y)) {
                self.mark(x, y);
            }
        }
        self.reach = reach;
        // The walls round the floor in sight (seen face-on or not, they bound what he saw).
        let floor: Vec<u32> =
            self.in_sight.iter().copied().filter(|&i| grid.get(i as i32 % self.width, i as i32 / self.width) != Tile::Rock).collect();
        for i in floor {
            let (x, y) = (i as i32 % self.width, i as i32 / self.width);
            for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1), (x - 1, y - 1), (x + 1, y - 1), (x - 1, y + 1), (x + 1, y + 1)]
            {
                if grid.in_bounds(nx, ny) && grid.get(nx, ny) == Tile::Rock {
                    self.mark(nx, ny);
                }
            }
        }
    }

    /// Has he seen this tile (or is the level revealed whole)?
    pub fn seen(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.width && y < self.height && self.seen[(y * self.width + x) as usize]
    }

    /// Was the tile in his sight at the last look?
    pub fn visible(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.width && y < self.height && self.visible[(y * self.width + x) as usize]
    }

    /// The level is revealed whole.
    pub fn whole(&self) -> bool {
        self.whole
    }

    /// The tiles revealed since the level began, in order, as indices (`y * width + x`); a reader
    /// keeps how many it has taken and asks for the rest (`revealed_since`).
    pub fn revealed(&self) -> usize {
        self.log.len()
    }

    pub fn revealed_since(&self, n: usize) -> &[u32] {
        self.log.get(n..).unwrap_or(&[])
    }

    /// Changes whenever a level is loaded (a reload of the same one included).
    pub fn epoch(&self) -> u32 {
        self.epoch
    }

    pub fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }
}

/// Can the eye pass from `a` to `b`? Along the line between their middles no tile but the ends is
/// rock, and no diagonal step slips between two rocks touching at a corner.
fn clear_line(grid: &Grid, a: P, b: P) -> bool {
    let (dx, dy) = ((b.x - a.x).abs(), -(b.y - a.y).abs());
    let (sx, sy) = (if a.x < b.x { 1 } else { -1 }, if a.y < b.y { 1 } else { -1 });
    let (mut x, mut y) = (a.x, a.y);
    let mut err = dx + dy;
    let rock = |x: i32, y: i32| grid.get(x, y) == Tile::Rock;
    while (x, y) != (b.x, b.y) {
        let e2 = 2 * err;
        let (px, py) = (x, y);
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
        if px != x && py != y && rock(x, py) && rock(px, y) {
            return false;
        }
        if (x, y) != (b.x, b.y) && rock(x, y) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::{parse_level, LevelData};

    fn level(rows: &[&str], theme: Theme) -> ParsedLevel {
        parse_level(&LevelData {
            name: "test".into(),
            rows: rows.iter().map(|r| r.to_string()).collect(),
            heights: Vec::new(),
            enemy_speed: 1.0,
            hero_speed: 1.0,
            frenzy_seconds: 1.0,
            boss: false,
            theme,
        })
    }

    /// A long hall, a wall down the middle with a room behind it, and a door at the far end.
    const ROOMS: [&str; 7] = [
        "####################",
        "#P                G#",
        "#                  #",
        "############### ####",
        "#                  #",
        "#                  #",
        "####################",
    ];

    #[test]
    fn the_fog_reveals_round_him_and_not_through_walls() {
        let l = level(&ROOMS, Theme::Estate);
        let mut f = Explore::new();
        f.enter(&l);
        assert!(!f.whole());
        f.observe(&l.grid, V::new(3.0, 1.0));
        // Round him, in the hall: seen and in sight; the wall below the hall too.
        assert!(f.seen(3, 1) && f.visible(3, 1) && f.seen(6, 2) && f.seen(3, 3));
        // Beyond his sight down the hall: not yet.
        assert!(!f.seen(18, 1));
        // The room behind the wall, right below him: dark.
        assert!(!f.seen(3, 4) && !f.seen(5, 5));
        // He walks to the doorway at the hall's end and looks through it into the room.
        f.observe(&l.grid, V::new(15.0, 2.0));
        assert!(f.seen(18, 1));
        assert!(f.seen(15, 4) && f.seen(15, 5));
        // Still not the far west of the room, round the corner of the wall from the door.
        assert!(!f.seen(7, 4));
        // What he saw first stays seen, though out of his sight now.
        assert!(f.seen(3, 1) && !f.visible(3, 1));
    }

    #[test]
    fn the_fog_is_kept_through_a_reload_of_the_same_level_and_forgotten_for_another() {
        let l = level(&ROOMS, Theme::Estate);
        let mut f = Explore::new();
        f.enter(&l);
        f.observe(&l.grid, V::new(3.0, 1.0));
        let n = f.revealed();
        assert!(n > 20);
        let e = f.epoch();
        f.enter(&l);
        assert!(f.seen(3, 1) && f.revealed() == n && f.epoch() != e);
        let mut other = ROOMS;
        other[2] = "#        #         #";
        f.enter(&level(&other, Theme::Estate));
        assert!(!f.seen(3, 1) && f.revealed() == 0);
        // A hub, a gym, the Pit, the lair, an arena or a court: revealed whole as it loads.
        f.enter(&level(&ROOMS, Theme::Hell));
        assert!(f.whole() && f.seen(10, 5));
        assert!(revealed_whole(Theme::Court(crate::logic::sins::Sin::Greed), true));
        assert!(revealed_whole(Theme::Pit, false) && revealed_whole(Theme::Lair, true) && revealed_whole(Theme::Gym, false));
        assert!(!revealed_whole(Theme::Realm(crate::logic::sins::Sin::Greed), false) && !revealed_whole(Theme::Estate, false));
    }

    #[test]
    fn its_log_holds_each_tile_once_in_the_order_revealed() {
        let l = level(&ROOMS, Theme::Estate);
        let mut f = Explore::new();
        f.enter(&l);
        f.observe(&l.grid, V::new(3.0, 1.0));
        let first = f.revealed();
        f.observe(&l.grid, V::new(3.0, 1.0));
        assert_eq!(f.revealed(), first, "standing still reveals nothing new");
        f.observe(&l.grid, V::new(12.0, 1.0));
        let more = f.revealed_since(first).to_vec();
        assert!(!more.is_empty());
        let mut all = f.revealed_since(0).to_vec();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), f.revealed());
        for i in more {
            assert!(f.seen(i as i32 % 20, i as i32 / 20));
        }
        // Switched off, it reveals nothing.
        let mut off = Explore::new();
        off.off = true;
        off.enter(&l);
        off.observe(&l.grid, V::new(3.0, 1.0));
        assert_eq!(off.revealed(), 0);
    }
}
