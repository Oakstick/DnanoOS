//! Shared vocabulary: tiles, directions, grid points and continuous positions.

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Tile {
    Rock,
    Open,
    Nodule,
    Charge,
    Nest,
    Tunnel,
    /// A hole in the floor: impassable on foot, jumpable, flyable.
    Pit,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Default)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
    #[default]
    None,
}

impl Dir {
    /// The one of the four closest to the way `v` points (Down for a zero vector).
    pub fn nearest(v: V) -> Dir {
        if v.x.abs() >= v.y.abs() && v.x != 0.0 {
            if v.x > 0.0 { Dir::Right } else { Dir::Left }
        } else if v.y < 0.0 {
            Dir::Up
        } else {
            Dir::Down
        }
    }

    pub fn vec(self) -> (i32, i32) {
        match self {
            Dir::Up => (0, -1),
            Dir::Down => (0, 1),
            Dir::Left => (-1, 0),
            Dir::Right => (1, 0),
            Dir::None => (0, 0),
        }
    }

    pub fn vecf(self) -> V {
        let (x, y) = self.vec();
        V::new(x as f64, y as f64)
    }

    pub fn opposite(self) -> Dir {
        match self {
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
            Dir::Left => Dir::Right,
            Dir::Right => Dir::Left,
            Dir::None => Dir::None,
        }
    }

    /// 'none' faces down, as it does everywhere a facing is needed.
    pub fn or_down(self) -> Dir {
        if self == Dir::None {
            Dir::Down
        } else {
            self
        }
    }
}

/// Search order used by every chooser; ties go to the earlier entry.
pub const CARDINALS: [Dir; 4] = [Dir::Up, Dir::Left, Dir::Down, Dir::Right];

/// Integer tile coordinate.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord, Default)]
pub struct P {
    pub x: i32,
    pub y: i32,
}

impl P {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
    pub fn v(self) -> V {
        V::new(self.x as f64, self.y as f64)
    }
    pub fn step(self, d: Dir) -> P {
        let (dx, dy) = d.vec();
        P::new(self.x + dx, self.y + dy)
    }
}

/// Continuous position in tile units; tile centers are integers.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct V {
    pub x: f64,
    pub y: f64,
}

impl V {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

pub fn dist_sq(a: V, b: V) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

pub fn manhattan(a: V, b: V) -> f64 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

/// JavaScript's Math.round: halves go up. Kept so generated layouts match the web build.
pub fn js_round(v: f64) -> f64 {
    (v + 0.5).floor()
}

/**
 * A screen-relative input (up = forward) mapped to a grid direction given
 * the direction the camera faces. Used by the third-person camera.
 */
pub fn relative_dir(input: Dir, forward: Dir) -> Dir {
    if input == Dir::None {
        return Dir::None;
    }
    let (fx, fy) = forward.or_down().vec();
    let right = (-fy, fx); // right of forward, with +y pointing down the grid
    let v = match input {
        Dir::Up => (fx, fy),
        Dir::Down => (-fx, -fy),
        Dir::Right => right,
        Dir::Left => (-right.0, -right.1),
        Dir::None => return Dir::None,
    };
    if v.0 == 1 {
        Dir::Right
    } else if v.0 == -1 {
        Dir::Left
    } else if v.1 == 1 {
        Dir::Down
    } else {
        Dir::Up
    }
}

/// Potions: the ten brews of a hero's elixir tree (tier 1 first) and an extra life.
/// Found in the first dungeon, bought from the old gatekeeper, carried on the belt.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum PotionKind {
    Health,
    Stamina,
    Strength,
    Haste,
    Bark,
    Fury,
    Fire,
    Invulnerability,
    Ragnarok,
    Spirit,
    Life,
}

impl PotionKind {
    /// Level glyphs. Only three bottle marks exist in the layouts (kept for
    /// parity with the web build); the game assigns the real brews on load.
    pub fn from_glyph(c: char) -> Option<PotionKind> {
        match c {
            'r' => Some(PotionKind::Life),
            'b' => Some(PotionKind::Health),
            'v' => Some(PotionKind::Haste),
            _ => None,
        }
    }
    pub fn glyph(self) -> char {
        match self {
            PotionKind::Life => 'r',
            PotionKind::Haste => 'v',
            _ => 'b',
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_dir_turns_with_the_camera() {
        assert_eq!(relative_dir(Dir::Up, Dir::Right), Dir::Right);
        assert_eq!(relative_dir(Dir::Right, Dir::Right), Dir::Down);
        assert_eq!(relative_dir(Dir::Left, Dir::Down), Dir::Right);
        assert_eq!(relative_dir(Dir::Down, Dir::Up), Dir::Down);
    }
}
