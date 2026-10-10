//! The cover in the Cyclops' meadow (B57): the stone pillars and the low walls inside the arena
//! stop what the Cyclops throws, and what it throws wears them down. A bolt chips a tile of cover,
//! a boulder knocks a piece off it, a dive that slams down beside it shakes it; worn through, it
//! comes down in a heap of rubble with a crash, and the tile is open grass. So cover is spent as the
//! fight goes on, and the fight moves round the arena to what still stands. Broken cover stays
//! broken through his falls (the meadow is laid anew when it is entered again).
//!
//! The rules are the meadow's only (a prince's court keeps its pillars); `chip_cover` takes any blow
//! at a tile, so a frame-data Cyclops (B02) can wear the cover by the same call.

use super::Game;
use crate::audio::Sfx;
use crate::logic::levels::ARENA;
use crate::logic::types::{Tile, CARDINALS, P, V};

/// How much a tile of cover takes before it comes down: a pillar standing alone, or a tile of a
/// low wall (the walls are the weaker, and more of them).
pub const PILLAR_HP: u32 = 12;
pub const WALL_HP: u32 = 7;
/// What wears it: a bolt, a boulder, a dive's slam within `SLAM_REACH` tiles of where it lands.
pub const BOLT_WEAR: u32 = 1;
pub const BOULDER_WEAR: u32 = 3;
pub const SLAM_WEAR: u32 = 5;
pub const SLAM_REACH: f64 = 1.8;

/// The wear on the meadow's cover, tile by tile (only the tiles struck so far).
#[derive(Clone, Debug, Default)]
pub struct Cover {
    pub wear: Vec<(P, u32)>,
    /// Tiles brought down so far.
    pub broken: u32,
}

impl Game {
    /// Is the tile at `p` a piece of the meadow's cover (rock inside the arena, in the Cyclops'
    /// meadow, not a prince's court)?
    pub fn is_cover(&self, p: P) -> bool {
        let (x0, y0, x1, y1) = ARENA;
        self.boss.is_some()
            && self.theme() == crate::logic::level::Theme::Meadow
            && p.x >= x0
            && p.x <= x1
            && p.y >= y0
            && p.y <= y1
            && self.level.grid.get(p.x, p.y) == Tile::Rock
    }

    /// Tiles of cover still standing in the Cyclops' meadow (0 anywhere else).
    pub fn meadow_cover_standing(&self) -> u32 {
        let (x0, y0, x1, y1) = ARENA;
        (y0..=y1).flat_map(|y| (x0..=x1).map(move |x| P::new(x, y))).filter(|&p| self.is_cover(p)).count() as u32
    }

    /// How much the cover at `p` takes in all: a lone pillar more than a tile of a wall.
    pub fn cover_hp(&self, p: P) -> u32 {
        let g = &self.level.grid;
        let alone = CARDINALS.iter().all(|&d| {
            let n = g.neighbor(p, d);
            !self.is_cover(n)
        });
        if alone {
            PILLAR_HP
        } else {
            WALL_HP
        }
    }

    /// How worn the cover at `p` is, 0 (whole) to 1 (about to come down).
    pub fn cover_worn(&self, p: P) -> f32 {
        let w = self.cover.wear.iter().find(|(q, _)| *q == p).map(|w| w.1).unwrap_or(0);
        (w as f32 / self.cover_hp(p) as f32).min(1.0)
    }

    /// A blow of `amount` on the cover at `p` (nothing if it is not cover): chips fly; worn
    /// through, it comes down.
    pub fn chip_cover(&mut self, p: P, amount: u32) {
        if amount == 0 || !self.is_cover(p) {
            return;
        }
        let hp = self.cover_hp(p);
        let worn = match self.cover.wear.iter_mut().find(|(q, _)| *q == p) {
            Some(w) => {
                w.1 += amount;
                w.1
            }
            None => {
                self.cover.wear.push((p, amount));
                amount
            }
        };
        let (x, y) = (p.x as f64, p.y as f64);
        // Chips of stone off it, more as it wears.
        self.view.dust(&self.level.grid, x, y, 0x9a9488, 4 + (6 * worn / hp) as usize, 1.2);
        if worn >= hp {
            self.break_cover(p);
        } else if worn * 2 >= hp && (worn - amount) * 2 < hp {
            // Half worn: it cracks, and is heard to.
            self.audio.play(Sfx::Clang);
            self.floater(x, y - 0.6, "CRACKED", 0.8, Some(0xc8bca8));
        }
    }

    /// The cover at `p` comes down: the tile opens to grass, rubble and dust fly, a crash.
    fn break_cover(&mut self, p: P) {
        self.cover.wear.retain(|(q, _)| *q != p);
        self.cover.broken += 1;
        self.level.grid.set(p.x, p.y, Tile::Open);
        self.view.remove_rock(&self.level.grid, p.x, p.y);
        let (x, y) = (p.x as f64, p.y as f64);
        self.view.crumble(&self.level.grid, x, y);
        self.view.dust(&self.level.grid, x, y, 0x8a867c, 24, 2.6);
        let d = crate::logic::types::dist_sq(p.v(), self.hero.pos()).sqrt() as f32;
        self.view.shake(0.3 * (1.0 - d / 14.0).max(0.2));
        self.audio.play(Sfx::Explode);
    }

    /// Capture hook: a boulder hurled at the piece of cover nearest him, from beside him.
    pub fn debug_hurl(&mut self) {
        let hp = self.hero.pos();
        let (x0, y0, x1, y1) = ARENA;
        let near = (y0..=y1)
            .flat_map(|y| (x0..=x1).map(move |x| P::new(x, y)))
            .filter(|&p| self.is_cover(p))
            .min_by(|a, b| crate::logic::types::dist_sq(a.v(), hp).total_cmp(&crate::logic::types::dist_sq(b.v(), hp)));
        if let Some(p) = near {
            let from = V::new(hp.x + 0.6, hp.y);
            let dir = (p.y as f64 - from.y).atan2(p.x as f64 - from.x);
            self.boss_shots.push(crate::logic::boss::BossShot::new(from, dir, 6.0, crate::logic::boss::ShotKind::Boulder));
        }
    }

    /// A dive's slam at `at`: every piece of cover within reach is shaken.
    pub(super) fn slam_cover(&mut self, at: V) {
        let r = SLAM_REACH.ceil() as i32;
        let (cx, cy) = (at.x.round() as i32, at.y.round() as i32);
        for dy in -r..=r {
            for dx in -r..=r {
                let p = P::new(cx + dx, cy + dy);
                if crate::logic::types::dist_sq(p.v(), at) <= SLAM_REACH * SLAM_REACH {
                    self.chip_cover(p, SLAM_WEAR);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::save::Stage;
    use crate::logic::boss::{BossShot, ShotKind};

    fn meadow() -> Game {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        let mut g = Game::new();
        g.map_jump(Stage::Arena(0));
        g.enter(crate::game::Screen::Playing);
        g
    }

    #[test]
    fn what_the_cyclops_throws_wears_the_cover_down_until_it_falls() {
        let mut g = meadow();
        let start = g.meadow_cover_standing();
        assert!(start >= 15, "cover enough to begin with: {start}");
        // A pillar: lone rock inside the arena.
        let (x0, y0, x1, y1) = ARENA;
        let pillar = (y0..=y1)
            .flat_map(|y| (x0..=x1).map(move |x| P::new(x, y)))
            .find(|&p| g.is_cover(p) && g.cover_hp(p) == PILLAR_HP)
            .expect("a pillar");
        // Boulders rolled straight at it, one after another.
        let mut thrown = 0;
        while g.level.grid.get(pillar.x, pillar.y) == Tile::Rock && thrown < 20 {
            let from = V::new(pillar.x as f64, pillar.y as f64 + 3.0);
            g.boss_shots.push(BossShot::new(from, -std::f64::consts::FRAC_PI_2, 4.0, ShotKind::Boulder));
            for _ in 0..90 {
                g.update_boss_shots(crate::app::STEP);
            }
            thrown += 1;
        }
        assert_eq!(thrown, PILLAR_HP.div_ceil(BOULDER_WEAR), "worn through boulder by boulder");
        assert_eq!(g.level.grid.get(pillar.x, pillar.y), Tile::Open, "it came down: open grass");
        assert_eq!(g.meadow_cover_standing(), start - 1);
        // What is thrown now flies on through where it stood.
        let from = V::new(pillar.x as f64, pillar.y as f64 + 2.0);
        g.boss_shots.push(BossShot::new(from, -std::f64::consts::FRAC_PI_2, 6.0, ShotKind::Bolt));
        g.hero.place(P::new(pillar.x + 4, pillar.y + 4));
        let past = (0..(0.5 / crate::app::STEP) as usize).any(|_| {
            g.update_boss_shots(crate::app::STEP);
            g.boss_shots.iter().any(|s| s.pos.y < pillar.y as f64 - 0.5)
        });
        assert!(past, "what is thrown flies on through where it stood");
    }

    #[test]
    fn a_dive_slam_shakes_the_cover_beside_it_and_nothing_breaks_outside_the_meadow() {
        let mut g = meadow();
        let (x0, y0, x1, y1) = ARENA;
        let wall =
            (y0..=y1).flat_map(|y| (x0..=x1).map(move |x| P::new(x, y))).find(|&p| g.is_cover(p) && g.cover_hp(p) == WALL_HP).unwrap();
        g.slam_cover(wall.v());
        assert!(g.cover_worn(wall) > 0.0, "shaken");
        g.slam_cover(wall.v());
        assert_eq!(g.level.grid.get(wall.x, wall.y), Tile::Open, "two slams bring a wall's tile down");
        // In the Garden nothing is cover.
        let garden = super::super::tests::playing();
        assert_eq!(garden.meadow_cover_standing(), 0);
    }
}
