//! The Garden of Thorns: Lust's realm (REALMS.md §6), laid out and kept here.
//!
//! A moonlit pleasure garden of rose hedges as tall as walls. Its rose paths wind inward in a square
//! spiral of four rings to the heart, where seven bridal bowers hold the remains of the grooms who
//! came before him, each with his last gear. The Bride stands always ahead on the rose path he walks
//! (see `Thorns::focus`), and every lovely thing faces the camera; behind the faces is black bramble.
//! The honest way out is the thorn path: straight from the court where he arrives up the eastern
//! bramble to the Black Thorn, the one fixed landmark, at whose foot the gate to Asmodeus' court
//! stands. Two more thorn paths run hidden through the hedges (their tiles drawn as hedge: the
//! facades) from the spiral's rings to it. Along them lie the clearings: the koi ponds and censers
//! of Tobit's cure, a ring of dancing elf-maids, and the natives' dens.
//!
//! What changes in play (charm, koi, smoke, the burnt paths, the wedding night) is kept here too;
//! `game::realms::thorns` plays the rules and `render::view::realms::thorns` draws them.

use std::collections::HashSet;

use super::{gate, Base, Canvas, Dealer, Pack};
use crate::logic::enemy::EnemyKind;
use crate::logic::hub::Rect;
use crate::logic::rng::Rng;
use crate::logic::sins::Sin;
use crate::logic::types::{Dir, P, V};
use crate::logic::vault::{roll_drops, DropItem, Rarity};

/// The layout's seed: the realm is the same every time.
pub const SEED: u32 = 7606;
/// The garden's size in tiles.
pub const W: i32 = 64;
pub const H: i32 = 62;
/// The spiral's square: its corner and side, and its rings (each a rose path two tiles wide inside
/// a hedge two tiles thick).
pub const SPIRAL_X: i32 = 3;
pub const SPIRAL_Y: i32 = 13;
pub const SPIRAL_SIDE: i32 = 45;
pub const RINGS: i32 = 4;
/// The main thorn path's two columns.
pub const THORN_X: i32 = 54;
/// The rows the hidden thorn paths run along: one from the heart, one from the second ring.
pub const HIDDEN_ROWS: [(i32, i32); 2] = [(31, 32), (21, 44)];

/// Charm, from nothing to full: what fills it and what drains it, each a second.
pub const CHARM_MAX: f64 = 100.0;
pub const CHARM_FACING: f64 = 20.0;
pub const CHARM_MIST: f64 = 10.0;
pub const CHARM_DRAIN: f64 = 15.0;
/// How far off her bearing he may face and still be facing her (radians: 45 degrees).
pub const FACING_HALF_ANGLE: f64 = std::f64::consts::FRAC_PI_4;
/// Enthralled: for this long, his controls mirrored and drawn toward her this fast (tiles a second).
pub const ENTHRAL_SECONDS: f64 = 3.0;
pub const ENTHRAL_PULL: f64 = 3.0;
/// Where charm falls back to when the spell breaks.
pub const CHARM_AFTER: f64 = 45.0;
/// The thorns: health a second while he walks them, and the pace they leave him.
pub const THORN_BITE: f64 = 2.0;
pub const THORN_PACE: f64 = 0.8;
/// The rose paths heal this much a second.
pub const ROSE_HEAL: f64 = 2.0;
/// Seconds in a pond to catch a koi; seconds a pond takes to fill again; how many he can carry.
pub const KOI_CATCH: f64 = 2.0;
pub const KOI_RESTOCK: f64 = 15.0;
pub const KOI_MAX: u32 = 3;
/// A burnt fish's smoke: how long, and how far from the censer it blocks charm (4 tiles wide).
pub const SMOKE_SECONDS: f64 = 10.0;
pub const SMOKE_RADIUS: f64 = 2.0;
/// How far round the smoke the facades turn to glass and the honest paths light.
pub const SMOKE_REVEAL: f64 = 9.0;
/// The golden koi's gall: seconds of true sight.
pub const SIGHT_SECONDS: f64 = 20.0;
/// A ring of dancing elf-maids draws him in from this far, this fast, and charms him at its middle.
pub const DANCE_REACH: f64 = 3.6;
pub const DANCE_PULL: f64 = 1.6;
pub const DANCE_CHARM: f64 = 25.0;
/// The wedding night: the gate takes this long to grind open; facing her fills charm this many
/// times as fast; she walks this fast and turns this fast (radians a second); her embrace reaches
/// this far; three blows on her back unmake her.
pub const WEDDING_SECONDS: f64 = 10.0;
pub const WEDDING_CHARM: f64 = 3.0;
pub const BRIDE_WALK: f64 = 1.3;
pub const BRIDE_TURN: f64 = 0.8;
pub const EMBRACE: f64 = 1.0;
pub const BACK_BLOWS: u32 = 3;
/// He comes this near the gate's front and the wedding night begins.
pub const WEDDING_REACH: f64 = 3.5;
/// The eighth groom's wound: this share of his health (never the last of it).
pub const GROOM_WOUND: f64 = 0.4;

/// How far ahead of him the Bride stands on a rose path: farther across the screen than up or
/// down it, so that the camera above, looking north, keeps her in its frame.
pub const AHEAD_ACROSS: f64 = 6.0;
pub const AHEAD_UP: f64 = 4.0;
pub const AHEAD_DOWN: f64 = 3.5;
/// He comes this near (along the path) and she has gone on ahead.
pub const NEAR_HER: f64 = 1.0;

/// What a tile of the garden is underfoot.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ground {
    /// Hedge and bramble (rock).
    Bramble,
    /// A rose path: smooth, perfumed, healing, and always pointing at her.
    Rose,
    /// A thorn path: straight, painful, slow, held by the brides.
    Thorn,
    /// Open ground: the court, the clearings, the heart.
    Glade,
    /// Burnt by Raphael's beam: plain ash, neither rose nor thorn.
    Ash,
}

/// One side of a ring: the way he walks it (`ahead`), the way out from it (`outward`), and the
/// ring's bounds and block, from which the Bride's place along it is worked out.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Path {
    pub ring: i32,
    pub ahead: Dir,
    pub outward: Dir,
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
    /// The block across the ring's east side (its first row).
    pub block: i32,
}

/// A koi pond: its tiles, whether its fish is the golden one, and seconds until it holds a fish again.
#[derive(Clone, Debug)]
pub struct Pond {
    pub tiles: Vec<P>,
    pub golden: bool,
    pub empty: f64,
}

impl Pond {
    pub fn center(&self) -> V {
        let n = self.tiles.len().max(1) as f64;
        V::new(self.tiles.iter().map(|t| t.x as f64).sum::<f64>() / n, self.tiles.iter().map(|t| t.y as f64).sum::<f64>() / n)
    }
}

/// A bridal bower: where it stands, the groom's last gear, and whether he has taken it.
#[derive(Clone, Copy, Debug)]
pub struct Bower {
    pub at: P,
    pub item: DropItem,
    pub rarity: Rarity,
    pub taken: bool,
}

/// A burnt fish's smoke rising from a censer.
#[derive(Clone, Copy, Debug)]
pub struct Smoke {
    pub at: V,
    pub left: f64,
}

/// The wedding night, while it lasts: seconds the gate has ground open, the blows on her back,
/// whether his last swing was counted, and whether he has leapt through her. (The Bride herself,
/// walking up behind him, is `Thorns::bride`.)
#[derive(Clone, Copy, Debug)]
pub struct Wedding {
    pub t: f64,
    pub back_blows: u32,
    pub swung: bool,
    pub passed: bool,
}

#[derive(Clone, Debug)]
pub struct Thorns {
    pub base: Base,
    /// Every tile's ground, row by row.
    pub ground: Vec<Ground>,
    /// The rings' sides, in walking order.
    pub paths: Vec<Path>,
    /// The court he arrives in, the clearings on the thorn path, the Black Thorn's clearing, the heart.
    pub court: Rect,
    pub clearings: Vec<Rect>,
    pub thorn_glade: Rect,
    pub heart: Rect,
    /// The innermost bower, where the eighth groom wakes.
    pub inner: P,
    pub bowers: Vec<Bower>,
    pub ponds: Vec<Pond>,
    /// The censers (rock tiles), the gate's among them.
    pub censers: Vec<P>,
    /// Walkable thorn tiles drawn as hedge: the hidden paths' facades.
    pub facades: HashSet<P>,
    /// Perfume mist, pooled in the hollows.
    pub mist: HashSet<P>,
    /// The dancing rings of elf-maids.
    pub dances: Vec<V>,
    /// The Bride's own pavilion, on the heart's north hedge, and the Black Thorn's root.
    pub home: V,
    pub black_thorn: V,
    /// Where the fox's wedding walks: from, to.
    pub procession: (V, V),

    // ---- what changes in play
    pub charm: f64,
    /// Seconds of enthralment left.
    pub enthralled: f64,
    /// The Bride: where she stands and which way she faces, the path she was set on, and how
    /// long ago she last moved (from where).
    pub bride: V,
    pub bride_facing: V,
    pub bride_path: Option<usize>,
    pub bride_moved: f64,
    pub bride_from: V,
    /// Koi carried, and seconds he has stood fishing.
    pub koi: u32,
    pub fishing: f64,
    pub smoke: Option<Smoke>,
    /// Seconds of true sight left.
    pub sight: f64,
    pub golden_taken: bool,
    /// Hedge burnt away by Raphael's beams.
    pub burnt: Vec<P>,
    pub wedding: Option<Wedding>,
    /// Three blows on her back unmade her.
    pub unmade: bool,
    /// How many times she has taken him, and whether the brides have come in from the sides (once).
    pub grooms: u32,
    pub brides_raised: bool,
    /// Health owed by the thorns and to the roses, not yet whole.
    pub thorn_carry: f64,
    pub rose_carry: f64,
    /// Raphael's charges and his patron's wait as last seen (a cast shows as a drop or a jump).
    pub raphael_seen: u8,
    pub patron_seen: f64,
    /// The hints already given.
    pub hints: u8,
}

/// A rectangle of tiles.
const fn rect(x0: i32, y0: i32, x1: i32, y1: i32) -> Rect {
    Rect { x0, y0, x1, y1 }
}

/// Ring `k`'s bounds: x0, y0, x1, y1 (the heart is ring `RINGS`).
pub fn ring_bounds(k: i32) -> (i32, i32, i32, i32) {
    let o = 4 * k;
    (SPIRAL_X + o, SPIRAL_Y + o, SPIRAL_X + SPIRAL_SIDE - 1 - o, SPIRAL_Y + SPIRAL_SIDE - 1 - o)
}

/// Ring `k`'s block: the first of its two rows across the east side.
pub fn ring_block(k: i32) -> i32 {
    ring_bounds(k).3 - 4
}

impl Thorns {
    /// The realm laid out for `tier`.
    pub fn generate(tier: usize) -> Thorns {
        // Its foes, dealt by how far he has come (`Dealer`).
        let mut deal = Dealer::new(tier);
        let sin = Sin::Lust;
        let mut c = Canvas::new(W, H);
        let mut rose: HashSet<P> = HashSet::new();
        let mut thorn: HashSet<P> = HashSet::new();
        let mut glade: HashSet<P> = HashSet::new();
        let carve = |c: &mut Canvas, set: &mut HashSet<P>, r: Rect| {
            for p in r.tiles() {
                c.set(p, ' ');
                set.insert(p);
            }
        };

        // ---- the spiral: four rings and the heart
        let mut paths = Vec::new();
        for k in 0..RINGS {
            let (x0, y0, x1, y1) = ring_bounds(k);
            for p in rect(x0, y0, x1, y1).tiles() {
                if p.x - x0 < 2 || x1 - p.x < 2 || p.y - y0 < 2 || y1 - p.y < 2 {
                    c.set(p, ' ');
                    rose.insert(p);
                }
            }
            let block = ring_block(k);
            for (ahead, outward) in [(Dir::Left, Dir::Down), (Dir::Up, Dir::Left), (Dir::Right, Dir::Up), (Dir::Down, Dir::Right)] {
                paths.push(Path { ring: k, ahead, outward, x0, y0, x1, y1, block });
            }
        }
        let (hx0, hy0, hx1, hy1) = ring_bounds(RINGS);
        let heart = rect(hx0, hy0, hx1, hy1);
        carve(&mut c, &mut glade, heart);
        for k in 0..RINGS {
            let (_, _, x1, y1) = ring_bounds(k);
            // The block across the east side, and the gap through the hedge into the next ring.
            let block = ring_block(k);
            for p in rect(x1 - 1, block, x1, block + 1).tiles() {
                c.set(p, '#');
                rose.remove(&p);
            }
            for p in rect(x1 - 3, y1 - 6, x1 - 2, y1 - 5).tiles() {
                c.set(p, ' ');
                rose.insert(p);
            }
        }

        // ---- the court, the rose arch into the spiral, the thorn path, its clearings
        let court = rect(50, 54, 60, 58);
        carve(&mut c, &mut glade, court);
        let (_, _, sx1, sy1) = ring_bounds(0);
        for p in rect(sx1 + 1, sy1 - 2, sx1 + 2, sy1 - 1).tiles() {
            c.set(p, ' ');
            rose.insert(p);
        }
        let thorn_glade = rect(50, 3, 60, 9);
        let clearings = vec![rect(51, 41, 58, 46), rect(51, 24, 58, 29), rect(50, 12, 59, 17)];
        carve(&mut c, &mut glade, thorn_glade);
        for r in &clearings {
            carve(&mut c, &mut glade, *r);
        }
        for p in rect(THORN_X, thorn_glade.y1 + 1, THORN_X + 1, court.y0 - 1).tiles() {
            if !glade.contains(&p) {
                c.set(p, ' ');
                thorn.insert(p);
            }
        }
        // The hidden thorn paths: straight through the hedges, east to the thorn path, their tiles
        // in the hedge drawn as hedge.
        let mut facades = HashSet::new();
        for (y, from) in HIDDEN_ROWS {
            for x in from..THORN_X {
                let p = P::new(x, y);
                if c.get(p) == '#' {
                    c.set(p, ' ');
                    thorn.insert(p);
                    facades.insert(p);
                }
            }
        }

        // ---- the bowers, the ponds, the censers, the dancers, the Bride's pavilion, the Thorn
        let inner = P::new((hx0 + hx1) / 2, (hy0 + hy1) / 2);
        let bower_at = [
            inner,
            P::new(inner.x - 4, inner.y - 3),
            P::new(inner.x, inner.y - 4),
            P::new(inner.x + 4, inner.y - 3),
            P::new(inner.x - 4, inner.y + 3),
            P::new(inner.x, inner.y + 4),
            P::new(inner.x + 4, inner.y + 3),
        ];
        let mut rng = Rng::new(SEED ^ 0x0b0e);
        let bowers = bower_at
            .iter()
            .enumerate()
            .map(|(i, &at)| {
                // The innermost groom carried the best of it.
                let (item, rarity) = roll_drops(&mut rng, if i == 0 { 1.0 } else { 0.6 })[0];
                Bower { at, item, rarity, taken: false }
            })
            .collect();
        let pond = |x: i32, y: i32, golden: bool| Pond {
            tiles: vec![P::new(x, y), P::new(x + 1, y), P::new(x, y + 1), P::new(x + 1, y + 1)],
            golden,
            empty: 0.0,
        };
        let ponds = vec![pond(58, 56, false), pond(56, 43, false), pond(52, 26, false), pond(hx0 + 1, inner.y - 1, true)];
        let exit = gate(sin, P::new(THORN_X + 1, 2), Dir::Down);
        let censers = vec![P::new(60, 54), P::new(58, 41), P::new(51, 24), P::new(hx1 - 1, inner.y), P::new(exit.tile.x + 3, 4)];
        for &p in &censers {
            c.set(p, '#');
        }
        let dances = vec![V::new(clearings[2].x0 as f64 + 4.5, (clearings[2].y0 + clearings[2].y1) as f64 / 2.0)];
        let home = V::new(inner.x as f64, hy0 as f64 - 1.5);
        let black_thorn = V::new(exit.tile.x as f64 - 2.6, 2.3);

        // ---- perfume mist in the hollows: the rings' corners and the heart's
        let mut mist = HashSet::new();
        for k in 0..RINGS {
            let (x0, y0, x1, y1) = ring_bounds(k);
            for (cx, cy) in [(x0, y1 - 1), (x0, y0), (x1 - 1, y0)] {
                for p in rect(cx, cy, cx + 1, cy + 1).tiles() {
                    mist.insert(p);
                }
            }
        }
        for (cx, cy) in [(hx0, hy0), (hx1 - 2, hy0), (hx0, hy1 - 2), (hx1 - 2, hy1 - 2)] {
            for p in rect(cx, cy, cx + 2, cy + 2).tiles() {
                if c.get(p) == ' ' {
                    mist.insert(p);
                }
            }
        }

        // ---- the portals, the spawn, Raphael's shrine
        let entry = gate(sin, P::new(court.x1 + 1, (court.y0 + court.y1) / 2), Dir::Left);
        let arrival = entry.front();
        c.set(arrival, 'P');
        c.set(exit.tile, 'D');
        c.set(P::new(thorn_glade.x0 + 1, thorn_glade.y1 - 1), 'G');
        c.set(P::new(court.x0 + 1, court.y1 - 1), 'W');

        // ---- the packs: elf-maids on the thorn path, natives in the clearings
        let mut packs = Vec::new();
        let extra = usize::from(tier >= 4);
        for (y0, y1, count) in [
            (court.y0 - 7, court.y0 - 1, 2 + extra),
            (clearings[1].y1 + 2, clearings[0].y0 - 2, 3 + extra),
            (clearings[2].y1 + 1, clearings[1].y0 - 1, 2),
        ] {
            let den = rect(THORN_X, y0, THORN_X + 1, y1);
            let span = (y1 - y0).max(1);
            let foes = (0..count)
                .map(|k| {
                    let y = y0 + (span * (k as i32 * 2 + 1)) / (count as i32 * 2);
                    (EnemyKind::Ellepige, 0, P::new(THORN_X + (k as i32 % 2), y))
                })
                .collect();
            packs.push(Pack { den, foes });
        }
        let busy: HashSet<P> = ponds.iter().flat_map(|p| p.tiles.iter().copied()).chain(censers.iter().copied()).collect();
        for (r, count) in [(clearings[0], 2 + extra), (clearings[1], 3)] {
            let spots: Vec<P> = [(1, 1), (5, 4), (2, 4), (6, 1), (4, 2)]
                .iter()
                .map(|&(dx, dy)| P::new(r.x0 + dx, r.y0 + dy))
                .filter(|p| !busy.contains(p) && !(p.x == THORN_X || p.x == THORN_X + 1))
                .collect();
            let foes = spots
                .into_iter()
                .take(count)
                .map(|p| {
                    let (kind, stage) = deal.draw();
                    (kind, stage, p)
                })
                .collect();
            packs.push(Pack { den: r, foes });
        }

        let (rows, heights) = c.finish();
        let mut ground = vec![Ground::Bramble; (W * H) as usize];
        for (set, g) in [(&rose, Ground::Rose), (&thorn, Ground::Thorn), (&glade, Ground::Glade)] {
            for p in set.iter() {
                if c.get(*p) != '#' {
                    ground[(p.y * W + p.x) as usize] = g;
                }
            }
        }
        let base = Base { sin, width: W, height: H, rows, heights, arrival, entry, exit, packs, checkpoint: arrival, tier };
        let court_lure = V::new(court.x0 as f64 + 5.0, court.y1 as f64 - 0.5);
        Thorns {
            base,
            ground,
            paths,
            court,
            clearings,
            thorn_glade,
            heart,
            inner,
            bowers,
            ponds,
            censers,
            facades,
            mist,
            dances,
            home,
            black_thorn,
            procession: (V::new(court.x1 as f64 + 1.0, court.y0 as f64 + 0.6), V::new(sx1 as f64 - 3.0, court.y0 as f64 + 1.6)),
            charm: 0.0,
            enthralled: 0.0,
            bride: court_lure,
            bride_facing: V::new(0.0, 1.0),
            bride_path: None,
            bride_moved: 10.0,
            bride_from: court_lure,
            koi: 0,
            fishing: 0.0,
            smoke: None,
            sight: 0.0,
            golden_taken: false,
            burnt: Vec::new(),
            wedding: None,
            unmade: false,
            grooms: 0,
            brides_raised: false,
            thorn_carry: 0.0,
            rose_carry: 0.0,
            raphael_seen: 0,
            patron_seen: 0.0,
            hints: 0,
        }
    }

    pub fn inside(&self, p: P) -> bool {
        p.x >= 0 && p.y >= 0 && p.x < W && p.y < H
    }

    /// The ground underfoot at `p` (bramble outside the garden).
    pub fn ground_at(&self, p: P) -> Ground {
        if self.inside(p) {
            self.ground[(p.y * W + p.x) as usize]
        } else {
            Ground::Bramble
        }
    }

    /// The rose path (a ring's side, by index into `paths`) that `p` lies on, if any. A corner
    /// belongs to the side it turns onto; the gap into a ring and the arch into the first belong
    /// to its southern side.
    pub fn path_at(&self, p: P) -> Option<usize> {
        if self.ground_at(p) != Ground::Rose {
            return None;
        }
        for k in 0..RINGS {
            let (x0, y0, x1, y1) = ring_bounds(k);
            let inside = p.x >= x0 && p.x <= x1 && p.y >= y0 && p.y <= y1;
            // The gap into ring k from the ring outside it (or the arch from the court).
            let gap_in = p.x > x1 && p.x <= x1 + 2 && p.y >= y1 - 2 && p.y < y1;
            if !inside && !gap_in {
                continue;
            }
            if gap_in {
                return Some(self.path_index(k, Dir::Left));
            }
            let ring_tile = p.x - x0 < 2 || x1 - p.x < 2 || p.y - y0 < 2 || y1 - p.y < 2;
            if !ring_tile {
                continue;
            }
            let block = ring_block(k);
            let side = if p.x <= x0 + 1 && p.y >= y0 + 2 {
                Dir::Up
            } else if p.y <= y0 + 1 && p.x <= x1 - 2 {
                Dir::Right
            } else if p.x >= x1 - 1 && p.y < block {
                Dir::Down
            } else {
                Dir::Left
            };
            return Some(self.path_index(k, side));
        }
        // The gap from the last ring into the heart: the end of its east side.
        Some(self.path_index(RINGS - 1, Dir::Down))
    }

    fn path_index(&self, ring: i32, ahead: Dir) -> usize {
        let side = match ahead {
            Dir::Left => 0,
            Dir::Up => 1,
            Dir::Right => 2,
            _ => 3,
        };
        (ring * 4 + side) as usize
    }

    /// Where the Bride stands for him at `hero` on path `i`: on the outer hedge beside it, a little
    /// ahead (farther along the long sides the camera sees across, nearer going south), and never
    /// past its end; at its end, she has gone on round the corner to the next.
    pub fn focus(&self, i: usize, hero: V) -> V {
        let p = self.paths[i];
        let (x0, y0, x1, y1) = (p.x0 as f64, p.y0 as f64, p.x1 as f64, p.y1 as f64);
        let near_end = |left: f64| left < 2.5;
        match p.ahead {
            Dir::Left => {
                let end = x0 - 0.5;
                if near_end(hero.x - x0) {
                    return self.focus(i + 1, V::new(x0 + 0.5, y1 - 0.5));
                }
                V::new((hero.x - AHEAD_ACROSS).max(end), y1 + 1.5)
            }
            Dir::Up => {
                let end = y0 - 0.5;
                if near_end(hero.y - y0) {
                    return self.focus(i + 1, V::new(x0 + 0.5, y0 + 0.5));
                }
                V::new(x0 - 1.5, (hero.y - AHEAD_UP).max(end))
            }
            Dir::Right => {
                let end = x1 + 0.5;
                if near_end(x1 - hero.x) {
                    return self.focus(i + 1, V::new(x1 - 0.5, y0 + 0.5));
                }
                V::new((hero.x + AHEAD_ACROSS).min(end), y0 - 1.5)
            }
            _ => {
                let end = p.block as f64 + 0.5;
                if near_end(p.block as f64 - hero.y) {
                    // Round into the next ring, or at the last into the heart, onto her own pavilion.
                    return if p.ring + 1 < RINGS { self.focus(i + 1, V::new(x1 - 4.0, y1 - 5.5)) } else { self.home };
                }
                V::new(x1 + 1.5, (hero.y + AHEAD_DOWN).min(end))
            }
        }
    }

    /// Where she stands for him when he is on no rose path: on her pavilion while he is in the
    /// heart; in the court on a pale dais before the rose arch, and once he comes near, beyond it;
    /// elsewhere on the hedge by the spiral's east side, level with him.
    pub fn lure(&self, hero: V) -> V {
        let p = P::new(hero.x.round() as i32, hero.y.round() as i32);
        if self.heart.contains(p) {
            return self.home;
        }
        let (_, y0, x1, y1) = ring_bounds(0);
        if self.court.contains(p) {
            let dais = self.court_dais();
            return if hero.x > dais.x + 2.5 { dais } else { V::new(x1 as f64 - 2.0, y1 as f64 + 1.5) };
        }
        V::new(x1 as f64 + 1.5, hero.y.clamp(y0 as f64 + 1.0, y1 as f64 - 1.0))
    }

    /// Her dais in the court, before the rose arch.
    pub fn court_dais(&self) -> V {
        V::new(self.court.x0 as f64 + 5.0, self.court.y1 as f64 - 0.5)
    }

    /// The pond he stands in, if any.
    pub fn pond_at(&self, p: P) -> Option<usize> {
        self.ponds.iter().position(|pd| pd.tiles.contains(&p))
    }

    /// A censer within reach of `at` (beside it).
    pub fn censer_near(&self, at: V) -> Option<usize> {
        self.censers.iter().position(|c| (c.x as f64 - at.x).abs() <= 1.6 && (c.y as f64 - at.y).abs() <= 1.6)
    }

    /// The gate's censer (the last).
    pub fn gate_censer(&self) -> P {
        self.censers[self.censers.len() - 1]
    }

    /// Whether `p` is hedge that Raphael's beam can burn: inside the garden's wall, and not a
    /// censer or the Black Thorn's ground.
    pub fn burnable(&self, p: P) -> bool {
        p.x >= 3 && p.y >= 3 && p.x < W - 3 && p.y < H - 3 && self.ground_at(p) == Ground::Bramble && !self.censers.contains(&p)
    }

    /// Smoke reaching `at`: the burnt fish blocks charm within its 4 tiles.
    pub fn smoked(&self, at: V) -> bool {
        self.smoke.is_some_and(|s| ((s.at.x - at.x).powi(2) + (s.at.y - at.y).powi(2)).sqrt() <= SMOKE_RADIUS)
    }

    /// The Bride's back is toward `at`: he is behind her.
    pub fn behind_her(&self, at: V) -> bool {
        let to = V::new(at.x - self.bride.x, at.y - self.bride.y);
        let len = (to.x * to.x + to.y * to.y).sqrt().max(1e-6);
        (self.bride_facing.x * to.x + self.bride_facing.y * to.y) / len < 0.0
    }
}

/// Whether facing along `aim` from `from` is facing `to` (within `FACING_HALF_ANGLE` of its bearing).
pub fn facing(from: V, aim: V, to: V) -> bool {
    let d = V::new(to.x - from.x, to.y - from.y);
    let (ld, la) = ((d.x * d.x + d.y * d.y).sqrt(), (aim.x * aim.x + aim.y * aim.y).sqrt());
    if ld < 1e-6 || la < 1e-6 {
        return false;
    }
    (d.x * aim.x + d.y * aim.y) / (ld * la) >= FACING_HALF_ANGLE.cos() - 1e-9
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::parse_level;
    use crate::logic::levels::level_for;
    use crate::logic::pathing::reachable;

    /// Steps from `from` to every tile it can walk to.
    fn distances(grid: &crate::logic::grid::Grid, from: P) -> std::collections::HashMap<P, usize> {
        let mut dist = std::collections::HashMap::from([(from, 0)]);
        let mut queue = std::collections::VecDeque::from([from]);
        while let Some(p) = queue.pop_front() {
            for d in crate::logic::types::CARDINALS {
                let n = p.step(d);
                if grid.can_move(p, d, false, false) && !dist.contains_key(&n) {
                    dist.insert(n, dist[&p] + 1);
                    queue.push_back(n);
                }
            }
        }
        dist
    }

    fn garden() -> (Thorns, crate::logic::level::ParsedLevel) {
        let t = Thorns::generate(1);
        let level = parse_level(&t.base.level(&level_for(1)));
        (t, level)
    }

    #[test]
    fn the_rose_paths_spiral_inward_ring_by_ring_to_the_heart() {
        let (t, level) = garden();
        assert_eq!(t.paths.len(), (RINGS * 4) as usize);
        // Only the rose paths and the heart, and only from the court's arch: every ring is walked
        // in turn, each a little deeper, and the heart is reached at the end.
        let mut grid = level.grid.clone();
        for (x, y, _) in level.grid.cells() {
            let g = t.ground_at(P::new(x, y));
            let keep = g == Ground::Rose || t.heart.contains(P::new(x, y));
            if !keep {
                grid.set(x, y, crate::logic::types::Tile::Rock);
            }
        }
        let arch = P::new(ring_bounds(0).2 + 2, ring_bounds(0).3 - 1);
        let seen = reachable(&grid, arch, false, false);
        assert!(seen.contains(&t.inner), "the rose spiral leads to the innermost bower");
        // Walked from the arch, the rings come in order: the deeper the ring, the longer the way.
        let dist = distances(&grid, arch);
        let mut last = 0;
        for k in 0..RINGS {
            let (x0, y0, _, _) = ring_bounds(k);
            let d = dist.get(&P::new(x0, y0)).copied().unwrap_or(usize::MAX);
            assert!(d != usize::MAX && d > last, "ring {k}'s far corner comes after the ring outside it ({d} after {last})");
            last = d;
        }
        assert!(dist[&t.inner] > last, "and the heart last");
        // Every rose tile belongs to a side of a ring, and every side points somewhere.
        for (x, y, _) in level.grid.cells() {
            let p = P::new(x, y);
            if t.ground_at(p) == Ground::Rose {
                assert!(t.path_at(p).is_some(), "rose tile {p:?} on no path");
            }
        }
    }

    #[test]
    fn a_straight_thorn_path_runs_from_the_court_to_the_black_thorn() {
        let (t, level) = garden();
        // Straight up the eastern bramble from the court to the Black Thorn's clearing, all thorn
        // or clearing underfoot, never a rose.
        let front = t.base.exit.front();
        assert_eq!(front.x, THORN_X + 1);
        for y in t.thorn_glade.y1 + 1..t.court.y0 {
            for x in [THORN_X, THORN_X + 1] {
                let g = t.ground_at(P::new(x, y));
                assert!(g == Ground::Thorn || g == Ground::Glade, "({x}, {y}) is {g:?}");
                assert!(level.grid.is_walkable(x, y, false));
            }
        }
        assert!(t.court.contains(t.base.arrival));
        assert!(t.thorn_glade.contains(front));
        let seen = reachable(&level.grid, t.base.arrival, false, false);
        assert!(seen.contains(&front));
        // The hidden paths reach it too, from the heart and from the second ring, drawn as hedge.
        for (y, from) in HIDDEN_ROWS {
            for x in from..THORN_X {
                assert!(level.grid.is_walkable(x, y, false), "hidden path at ({x}, {y})");
            }
            assert!(t.facades.iter().any(|p| p.y == y));
        }
        assert!(t.facades.iter().all(|p| t.ground_at(*p) == Ground::Thorn));
    }

    #[test]
    fn seven_bowers_in_the_heart_and_the_cure_where_it_is_needed() {
        let (t, level) = garden();
        assert_eq!(t.bowers.len(), 7);
        assert_eq!(t.bowers[0].at, t.inner);
        let seen = reachable(&level.grid, t.base.arrival, false, false);
        for b in &t.bowers {
            assert!(t.heart.contains(b.at) && seen.contains(&b.at), "bower at {:?}", b.at);
        }
        assert_eq!(t.ponds.iter().filter(|p| p.golden).count(), 1, "one golden koi");
        for pd in &t.ponds {
            for p in &pd.tiles {
                assert!(seen.contains(p), "pond tile {p:?}");
            }
        }
        for c in &t.censers {
            assert!(!level.grid.is_walkable(c.x, c.y, false));
            assert!(crate::logic::types::CARDINALS.iter().any(|d| seen.contains(&c.step(*d))), "censer {c:?} can be reached");
        }
        // The gate's censer stands in the Black Thorn's clearing.
        assert!(t.thorn_glade.contains(t.gate_censer()));
        assert_eq!(level.angel_shrines.len(), 1, "Raphael's shrine");
        assert!(t.court.contains(level.angel_shrines[0]));
    }

    #[test]
    fn the_packs_hold_the_thorn_paths_and_clearings_never_the_roses() {
        let (t, level) = garden();
        let maids: usize = t.base.packs.iter().flat_map(|p| p.foes.iter()).filter(|f| f.0 == EnemyKind::Ellepige).count();
        assert!(maids >= 6, "{maids} elf-maids");
        for pk in &t.base.packs {
            for &(kind, stage, post) in &pk.foes {
                assert!(pk.den.contains(post) && level.grid.is_walkable(post.x, post.y, false), "{kind:?} at {post:?}");
                assert_ne!(t.ground_at(post), Ground::Rose, "{kind:?} on a rose path");
                if kind == EnemyKind::Ellepige {
                    assert_eq!((stage, t.ground_at(post)), (0, Ground::Thorn));
                }
            }
        }
    }

    #[test]
    fn the_bride_stands_ahead_on_the_outer_hedge_and_goes_on_round_the_corners() {
        let (t, level) = garden();
        for (i, p) in t.paths.iter().enumerate() {
            let (x0, y0, x1, y1) = (p.x0, p.y0, p.x1, p.y1);
            // A tile in the middle of the side.
            let at = match p.ahead {
                Dir::Left => P::new((x0 + x1) / 2, y1),
                Dir::Up => P::new(x0, (y0 + y1) / 2),
                Dir::Right => P::new((x0 + x1) / 2, y0),
                _ => P::new(x1, (y0 + p.block) / 2),
            };
            assert_eq!(t.path_at(at), Some(i), "{at:?}");
            let hero = at.v();
            let her = t.focus(i, hero);
            let (ax, ay) = p.ahead.vec();
            let along = (her.x - hero.x) * ax as f64 + (her.y - hero.y) * ay as f64;
            assert!(along > 2.5, "path {i}: she stands ahead ({along})");
            assert!(facing(hero, p.ahead.vecf(), her), "path {i}: walking it is facing her");
            let tile = P::new(her.x.round() as i32, her.y.round() as i32);
            assert!(!level.grid.is_walkable(tile.x, tile.y, false), "path {i}: she stands on the hedge, out of reach ({tile:?})");
        }
        // At a side's end she has gone on to the next.
        let p = t.paths[0];
        let corner = V::new(p.x0 as f64 + 1.0, p.y1 as f64);
        let her = t.focus(0, corner);
        assert!(her.y < corner.y - 3.0, "round the corner, ahead up the west side: {her:?}");
        assert_eq!(t.lure(t.inner.v()), t.home);
    }

    #[test]
    fn facing_is_within_forty_five_degrees() {
        let o = V::new(0.0, 0.0);
        assert!(facing(o, V::new(1.0, 0.0), V::new(5.0, 0.0)));
        assert!(facing(o, V::new(1.0, 0.0), V::new(5.0, 4.9)));
        assert!(!facing(o, V::new(1.0, 0.0), V::new(5.0, 5.5)));
        assert!(!facing(o, V::new(-1.0, 0.0), V::new(5.0, 0.0)));
    }
}
