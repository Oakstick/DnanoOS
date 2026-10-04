use super::grid::Grid;
use super::sins::Sin;
use super::types::{js_round, PotionKind, Tile, P};

/**
 * ASCII legend for `rows`:
 *   #  rock          .  nodule        o  remedy (green cross)   ~  pit
 *   ' ' open floor   P  hero spawn    G  enemy spawn (hollow exit)
 *   N  hollow interior (enemies only)  -  tunnel (wraps horizontally)
 *   W  weapon shrine (open tile)     I  item shrine (open tile)
 *   r  life potion   b  shield potion   v  invisibility potion (open tiles)
 *
 * `heights` rows run parallel to `rows`:
 *   0 1 2  floor elevation in levels     /  ramp joining two levels
 *   anything else = level 0
 */
#[derive(Clone, Debug)]
pub struct LevelData {
    pub name: String,
    pub rows: Vec<String>,
    pub heights: Vec<String>,
    pub enemy_speed: f64,
    pub hero_speed: f64,
    pub frenzy_seconds: f64,
    /// A boss arena: no nuggets, no hollow, one big foe.
    pub boss: bool,
    /// How it looks, and what lives there.
    pub theme: Theme,
}

/// A level's look: the moonlit estates and dungeons, the Cyclops' meadow, the lava of the Portals
/// of Hell, and a sin's realm and its prince's court (for now the estate and the meadow under the
/// sin's colour).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Theme {
    #[default]
    Estate,
    Meadow,
    Hell,
    Realm(Sin),
    Court(Sin),
    /// The gym: platforms over lava, one of every foe (drawn like the Portals of Hell).
    Gym,
    /// Krampus's lair at the bottom of the Pit (FINALE.md §3, `logic::lair`).
    Lair,
}

impl Theme {
    /// The sin whose realm or court this is.
    pub fn sin(self) -> Option<Sin> {
        match self {
            Theme::Realm(s) | Theme::Court(s) => Some(s),
            _ => None,
        }
    }

    /// A boss arena's sunny meadow: the Cyclops', or a prince's court still standing in for his own.
    pub fn is_meadow(self) -> bool {
        matches!(self, Theme::Meadow | Theme::Court(_)) && !self.is_princes_court()
    }

    /// A prince's court built to its own plan (`court`), its prince fought as himself: Greed's
    /// Counting House, and each other court once its module is `BUILT`.
    pub fn is_princes_court(self) -> bool {
        matches!(self, Theme::Court(s) if super::court::built(s))
    }

    /// Platforms over a sea of lava: the Portals of Hell, or the gym.
    pub fn is_hell(self) -> bool {
        matches!(self, Theme::Hell | Theme::Gym)
    }

    /// The Portals of Hell themselves (not the gym).
    pub fn is_hub(self) -> bool {
        self == Theme::Hell
    }

    /// The gym, with one of every foe.
    pub fn is_gym(self) -> bool {
        self == Theme::Gym
    }

    /// The Vaults of Mammon: Greed's realm, the one built so far.
    pub fn is_vault(self) -> bool {
        self == Theme::Realm(Sin::Greed)
    }
}

#[derive(Clone, Debug)]
pub struct ParsedLevel {
    pub grid: Grid,
    pub hero_spawn: P,
    pub enemy_spawn: P,
    pub nest_center: P,
    pub angel_shrines: Vec<P>,
    pub item_shrines: Vec<P>,
    pub potions: Vec<(P, PotionKind)>,
    /// The gate to the boss, set in the outer wall; it opens when every key is found.
    pub gate: Option<P>,
    /// The hub's portal gates ('1' to '7'): where each stands, and its sin's index.
    pub portals: Vec<(P, usize)>,
    pub data: LevelData,
}

pub fn parse_level(data: &LevelData) -> ParsedLevel {
    let height = data.rows.len() as i32;
    let width = data.rows[0].chars().count() as i32;
    let mut tiles = Vec::new();
    let mut heights = Vec::new();
    let mut ramps = Vec::new();
    let mut hero_spawn = None;
    let mut enemy_spawn = None;
    let mut nest_tiles = Vec::new();
    let mut angel_shrines = Vec::new();
    let mut item_shrines = Vec::new();
    let mut potions = Vec::new();
    let mut gate = None;
    let mut portals = Vec::new();
    let mut bridges = Vec::new();

    assert!(data.heights.is_empty() || data.heights.len() as i32 == height, "level heights row count mismatch");

    for (y, row) in data.rows.iter().enumerate() {
        let y = y as i32;
        let cells: Vec<char> = row.chars().collect();
        assert_eq!(cells.len() as i32, width, "level row {y} has the wrong width");
        let hrow: Vec<char> = data.heights.get(y as usize).map(|h| h.chars().collect()).unwrap_or_default();
        for (x, &c) in cells.iter().enumerate() {
            let x = x as i32;
            let p = P::new(x, y);
            let t = match c {
                '#' => Tile::Rock,
                'D' => {
                    gate = Some(p);
                    Tile::Rock
                }
                '1'..='7' => {
                    portals.push((p, c as usize - '1' as usize));
                    Tile::Rock
                }
                // A wooden bridge over lava or water.
                '=' => {
                    bridges.push(p);
                    Tile::Open
                }
                '.' => Tile::Nodule,
                'o' => Tile::Charge,
                ' ' => Tile::Open,
                '~' => Tile::Pit,
                '-' => Tile::Tunnel,
                'P' => {
                    hero_spawn = Some(p);
                    Tile::Open
                }
                'G' => {
                    enemy_spawn = Some(p);
                    Tile::Open
                }
                'N' => {
                    nest_tiles.push(p);
                    Tile::Nest
                }
                'W' => {
                    angel_shrines.push(p);
                    Tile::Open
                }
                'I' => {
                    item_shrines.push(p);
                    Tile::Open
                }
                other => match PotionKind::from_glyph(other) {
                    Some(kind) => {
                        potions.push((p, kind));
                        Tile::Open
                    }
                    None => panic!("unknown tile {other:?} at {x},{y} in {}", data.name),
                },
            };
            tiles.push(t);
            let h = hrow.get(x as usize).copied().unwrap_or('0');
            ramps.push(h == '/');
            heights.push(h.to_digit(10).map(|d| d as i32).unwrap_or(0));
        }
    }

    let hero_spawn = hero_spawn.expect("level has no hero spawn (P)");
    let enemy_spawn = enemy_spawn.expect("level has no enemy spawn (G)");

    // Ramps take the lower of the floors they join (spreading inward through wide ramp blocks);
    // rock takes its tallest walkable neighbor's floor so walls rise from the raised ground around them.
    let idx = |x: i32, y: i32| (y * width + x) as usize;
    let inb = |x: i32, y: i32| x >= 0 && y >= 0 && x < width && y < height;
    let neighbors = |x: i32, y: i32| [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)];
    let mut settled = vec![false; tiles.len()];
    for y in 0..height {
        for x in 0..width {
            if !ramps[idx(x, y)] {
                continue;
            }
            let around: Vec<i32> = neighbors(x, y)
                .iter()
                .filter(|&&(nx, ny)| inb(nx, ny) && tiles[idx(nx, ny)] != Tile::Rock && !ramps[idx(nx, ny)])
                .map(|&(nx, ny)| heights[idx(nx, ny)])
                .collect();
            if let Some(&m) = around.iter().min() {
                heights[idx(x, y)] = m;
                settled[idx(x, y)] = true;
            }
        }
    }
    for _ in 0..(width + height) {
        let mut changed = false;
        for y in 0..height {
            for x in 0..width {
                let i = idx(x, y);
                if !ramps[i] || settled[i] {
                    continue;
                }
                let near: Vec<i32> = neighbors(x, y)
                    .iter()
                    .filter(|&&(nx, ny)| inb(nx, ny) && ramps[idx(nx, ny)] && settled[idx(nx, ny)])
                    .map(|&(nx, ny)| heights[idx(nx, ny)])
                    .collect();
                if let Some(&m) = near.iter().min() {
                    heights[i] = m;
                    settled[i] = true;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    for y in 0..height {
        for x in 0..width {
            if tiles[idx(x, y)] != Tile::Rock {
                continue;
            }
            let around = neighbors(x, y)
                .iter()
                .filter(|&&(nx, ny)| inb(nx, ny) && tiles[idx(nx, ny)] != Tile::Rock)
                .map(|&(nx, ny)| heights[idx(nx, ny)])
                .max();
            if let Some(m) = around {
                heights[idx(x, y)] = m;
            }
        }
    }

    let nest_center = if nest_tiles.is_empty() {
        enemy_spawn
    } else {
        let n = nest_tiles.len() as f64;
        let sx: f64 = nest_tiles.iter().map(|p| p.x as f64).sum();
        let sy: f64 = nest_tiles.iter().map(|p| p.y as f64).sum();
        P::new(js_round(sx / n) as i32, js_round(sy / n) as i32)
    };

    let mut grid = Grid::new(width, height, tiles, heights, ramps);
    for b in bridges {
        grid.set_bridge(b.x, b.y, true);
    }
    ParsedLevel { grid, hero_spawn, enemy_spawn, nest_center, angel_shrines, item_shrines, potions, gate, portals, data: data.clone() }
}
