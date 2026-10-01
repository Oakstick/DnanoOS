//! The Vaults of Mammon: the realm of Greed. A dungeon of rooms and corridors
//! in the manner of Diablo, twice the size of one of its maps: chambers round a
//! great treasury, joined by wide corridors, in gold and polished marble. The
//! hero comes in through a portal in the west wall and the portal to Mammon's
//! court stands in the east wall at the far end. Every room but the first is
//! held by a pack of foes that idles there until he comes near, and every room
//! is strewn with gold: coins, heaps of them, treasure heaps, chests stacked
//! against the walls and rare gems, all of which he may take. A chest also
//! throws out one to three items as it opens, of four rarities as in Diablo
//! (see `roll_drops`). At the far portal Mammon takes his toll (see
//! `game::vault`): only those who leave his gold go through, and they keep
//! what his chests gave them.

use std::collections::HashSet;

use super::enemy::EnemyKind;
use super::hub::Rect;
use super::items::{item, weapon, ItemKind, RangedKind};
use super::level::{LevelData, Theme};
use super::rng::Rng;
use super::sins::Sin;
use super::types::{PotionKind, CARDINALS, P};

/// The layout's seed: the vault is the same every time.
pub const SEED: u32 = 4404;
/// Slots in the plan: rooms sit in a grid of cells this big, walls between.
pub const COLS: i32 = 6;
pub const ROWS: i32 = 4;
pub const CELL_W: i32 = 22;
pub const CELL_H: i32 = 24;
pub const MARGIN: i32 = 3;
/// Corridors are this many tiles wide.
pub const CORRIDOR: i32 = 3;
/// A pack wakes when he comes this close to one of its foes (tiles), or steps into its room.
pub const WAKE_RANGE: f64 = 8.0;
/// A hunting foe gives up and goes home when he is this far from its room.
pub const LEASH: f64 = 26.0;

/// What a piece of loot is worth, in gold.
pub const COIN_WORTH: u32 = 10;
pub const PILE_WORTH: u32 = 50;
pub const TREASURE_WORTH: u32 = 150;
pub const CHEST_WORTH: u32 = 200;
pub const GEM_WORTH: u32 = 500;
/// Seconds an item takes to fly from its chest to where it lands (it can be taken once it has).
pub const DROP_FLIGHT: f64 = 0.55;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RoomKind {
    /// Where he arrives: no foes.
    Entry,
    /// A chamber off the corridors.
    Chamber,
    /// The great treasury in the middle.
    Hall,
    /// The last room, with the portal to the court in its east wall.
    Exit,
}

#[derive(Clone, Debug)]
pub struct Room {
    pub rect: Rect,
    pub kind: RoomKind,
    /// 0 at the entry, 1 at the farthest room (by rooms passed through).
    pub depth: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LootKind {
    /// A few gold coins.
    Coin,
    /// A heap of them.
    Pile,
    /// A treasure heap: gold coins with goblets, a crown and jewels in it.
    Treasure,
    /// A treasure chest.
    Chest,
    /// A cut gem: ruby, sapphire or emerald (its colour).
    Gem(u32),
}

impl LootKind {
    pub fn worth(self) -> u32 {
        match self {
            LootKind::Coin => COIN_WORTH,
            LootKind::Pile => PILE_WORTH,
            LootKind::Treasure => TREASURE_WORTH,
            LootKind::Chest => CHEST_WORTH,
            LootKind::Gem(_) => GEM_WORTH,
        }
    }
}

/// The gems' colours: ruby, sapphire, emerald.
pub const GEMS: [u32; 3] = [0xe0203a, 0x3a6aff, 0x2ad06a];

#[derive(Clone, Copy, Debug)]
pub struct Loot {
    pub at: P,
    pub kind: LootKind,
    pub taken: bool,
}

/// How rare an item from a chest is, with Diablo's colours: white, blue, yellow, orange.
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum Rarity {
    Common,
    Magic,
    Rare,
    Legendary,
}

impl Rarity {
    pub fn color(self) -> u32 {
        match self {
            Rarity::Common => 0xf0ece0,
            Rarity::Magic => 0x6a8cff,
            Rarity::Rare => 0xffe24a,
            Rarity::Legendary => 0xff8a2a,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Rarity::Common => "COMMON",
            Rarity::Magic => "MAGIC",
            Rarity::Rare => "RARE",
            Rarity::Legendary => "LEGENDARY",
        }
    }
}

/// An item a chest throws out: a bottle for the belt, a ranged weapon, a shrine's boon, rune
/// fragments, or bombs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DropItem {
    Potion(PotionKind),
    Weapon(RangedKind),
    Boon(ItemKind),
    Runes(u32),
    Bombs(u32),
}

impl DropItem {
    /// What it is called on the ground, in capitals (bottles are named with the hero's brews).
    pub fn label(self, potion_name: impl Fn(PotionKind) -> &'static str) -> String {
        match self {
            DropItem::Potion(k) => potion_name(k).to_uppercase(),
            DropItem::Weapon(w) => weapon(w).name.to_uppercase(),
            DropItem::Boon(b) => item(b).name.to_uppercase(),
            DropItem::Runes(1) => "A RUNE FRAGMENT".to_string(),
            DropItem::Runes(n) => format!("{n} RUNE FRAGMENTS"),
            DropItem::Bombs(1) => "A BOMB".to_string(),
            DropItem::Bombs(n) => format!("{n} BOMBS"),
        }
    }
}

/// An item lying where a chest threw it (or still in the air).
#[derive(Clone, Copy, Debug)]
pub struct Drop {
    /// Where it lands, and the chest it flew from.
    pub at: P,
    pub from: P,
    pub item: DropItem,
    pub rarity: Rarity,
    /// Seconds since it was thrown.
    pub age: f64,
    pub taken: bool,
}

impl Drop {
    /// On the ground, ready to be taken.
    pub fn landed(&self) -> bool {
        self.age >= DROP_FLIGHT
    }
}

/// How rare a chest's item is: the deeper the room (`depth` 0 to 1), the better the odds.
pub fn roll_rarity(rng: &mut Rng, depth: f64) -> Rarity {
    let d = depth.clamp(0.0, 1.0);
    let weights = [52.0 - 22.0 * d, 30.0, 14.0 + 12.0 * d, 4.0 + 10.0 * d];
    let mut r = rng.next() * weights.iter().sum::<f64>();
    for (k, w) in weights.iter().enumerate() {
        if r < *w {
            return [Rarity::Common, Rarity::Magic, Rarity::Rare, Rarity::Legendary][k];
        }
        r -= w;
    }
    Rarity::Legendary
}

/// An item of `rarity`.
pub fn roll_item(rng: &mut Rng, rarity: Rarity) -> DropItem {
    use DropItem::*;
    let table: &[DropItem] = match rarity {
        Rarity::Common => &[Potion(PotionKind::Health), Potion(PotionKind::Stamina), Bombs(1), Weapon(RangedKind::Arrow)],
        Rarity::Magic => &[
            Potion(PotionKind::Strength),
            Potion(PotionKind::Haste),
            Potion(PotionKind::Bark),
            Weapon(RangedKind::Double),
            Weapon(RangedKind::Boomerang),
            Boon(ItemKind::Speed),
            Bombs(2),
        ],
        Rarity::Rare => &[
            Potion(PotionKind::Fury),
            Potion(PotionKind::Fire),
            Potion(PotionKind::Spirit),
            Weapon(RangedKind::Trishot),
            Weapon(RangedKind::Fireball),
            Weapon(RangedKind::Crescent),
            Boon(ItemKind::Shield),
            Runes(2),
        ],
        Rarity::Legendary => &[
            Potion(PotionKind::Invulnerability),
            Potion(PotionKind::Ragnarok),
            Potion(PotionKind::Life),
            Boon(ItemKind::Invincible),
            Runes(5),
        ],
    };
    rng.pick(table)
}

/// What a chest throws out as it opens: one item, sometimes two, now and then three.
pub fn roll_drops(rng: &mut Rng, depth: f64) -> Vec<(DropItem, Rarity)> {
    let count = 1 + usize::from(rng.next() < 0.35) + usize::from(rng.next() < 0.1);
    (0..count)
        .map(|_| {
            let rarity = roll_rarity(rng, depth);
            (roll_item(rng, rarity), rarity)
        })
        .collect()
}

/// A pack of foes: the room it holds, and each foe's kind, stage and post.
#[derive(Clone, Debug)]
pub struct Pack {
    pub den: Rect,
    pub foes: Vec<(EnemyKind, u8, P)>,
}

#[derive(Clone, Debug)]
pub struct Vault {
    pub width: i32,
    pub height: i32,
    pub rooms: Vec<Room>,
    /// The corridors, as rectangles of floor.
    pub corridors: Vec<Rect>,
    /// Columns standing in the rooms (rock tiles).
    pub pillars: Vec<P>,
    pub loot: Vec<Loot>,
    /// Items thrown out of the chests he has opened.
    pub drops: Vec<Drop>,
    pub packs: Vec<Pack>,
    /// Where he arrives, and the portal behind him in the west wall.
    pub arrival: P,
    pub entry_gate: P,
    /// The portal to the court, in the east wall of the last room ('D').
    pub exit_gate: P,
    /// Mammon's idol, on its plinth in the middle of the treasury (a rock tile).
    pub idol: P,
    pub rows: Vec<String>,
}

/// The slot each room takes (inclusive cell ranges), and its kind.
const PLAN: [((i32, i32, i32, i32), RoomKind); 19] = [
    ((0, 1, 0, 2), RoomKind::Entry),
    ((2, 1, 3, 2), RoomKind::Hall),
    ((5, 1, 5, 2), RoomKind::Exit),
    ((0, 0, 0, 0), RoomKind::Chamber),
    ((1, 0, 1, 0), RoomKind::Chamber),
    ((2, 0, 2, 0), RoomKind::Chamber),
    ((3, 0, 3, 0), RoomKind::Chamber),
    ((4, 0, 4, 0), RoomKind::Chamber),
    ((5, 0, 5, 0), RoomKind::Chamber),
    ((1, 1, 1, 1), RoomKind::Chamber),
    ((1, 2, 1, 2), RoomKind::Chamber),
    ((4, 1, 4, 1), RoomKind::Chamber),
    ((4, 2, 4, 2), RoomKind::Chamber),
    ((0, 3, 0, 3), RoomKind::Chamber),
    ((1, 3, 1, 3), RoomKind::Chamber),
    ((2, 3, 2, 3), RoomKind::Chamber),
    ((3, 3, 3, 3), RoomKind::Chamber),
    ((4, 3, 4, 3), RoomKind::Chamber),
    ((5, 3, 5, 3), RoomKind::Chamber),
];

/// The tiles a range of cells covers.
fn cells(c: (i32, i32, i32, i32)) -> Rect {
    Rect { x0: MARGIN + c.0 * CELL_W, y0: MARGIN + c.1 * CELL_H, x1: MARGIN + (c.2 + 1) * CELL_W - 1, y1: MARGIN + (c.3 + 1) * CELL_H - 1 }
}

fn overlap(a0: i32, a1: i32, b0: i32, b1: i32) -> Option<(i32, i32)> {
    let (lo, hi) = (a0.max(b0), a1.min(b1));
    (hi >= lo).then_some((lo, hi))
}

impl Vault {
    /// The vault, laid out from `seed`.
    pub fn generate(seed: u32) -> Vault {
        let mut rng = Rng::new(seed);
        let width = MARGIN * 2 + COLS * CELL_W;
        let height = MARGIN * 2 + ROWS * CELL_H;

        // ---- rooms: each somewhere in its slot, walls kept round it
        let mut rooms: Vec<Room> = PLAN
            .iter()
            .map(|&(c, kind)| {
                let slot = cells(c);
                let (sw, sh) = (slot.x1 - slot.x0 + 1, slot.y1 - slot.y0 + 1);
                let (w, h) = match kind {
                    RoomKind::Hall => (sw - 8, sh - 8),
                    RoomKind::Entry | RoomKind::Exit => (sw - 8, sh - 14),
                    RoomKind::Chamber => (11 + rng.index(7) as i32, 11 + rng.index(9) as i32),
                };
                let (w, h) = (w.min(sw - 4), h.min(sh - 4));
                let x0 = slot.x0 + 2 + rng.index((sw - 4 - w + 1) as usize) as i32;
                let y0 = slot.y0 + 2 + rng.index((sh - 4 - h + 1) as usize) as i32;
                Room { rect: Rect { x0, y0, x1: x0 + w - 1, y1: y0 + h - 1 }, kind, depth: 0.0 }
            })
            .collect();
        let slot_of: Vec<(i32, i32, i32, i32)> = PLAN.iter().map(|p| p.0).collect();
        let in_slot = |i: usize, cx: i32, cy: i32| {
            let s = slot_of[i];
            cx >= s.0 && cx <= s.2 && cy >= s.1 && cy <= s.3
        };
        let room_at = |cx: i32, cy: i32| (0..rooms.len()).find(|&i| in_slot(i, cx, cy));

        // ---- the graph: neighbours across a cell edge; a spanning tree first, then loops
        let mut edges: Vec<(usize, usize)> = Vec::new();
        for cy in 0..ROWS {
            for cx in 0..COLS {
                let Some(a) = room_at(cx, cy) else { continue };
                for (nx, ny) in [(cx + 1, cy), (cx, cy + 1)] {
                    if let Some(b) = room_at(nx, ny) {
                        if a != b && !edges.contains(&(a, b)) && !edges.contains(&(b, a)) {
                            edges.push((a, b));
                        }
                    }
                }
            }
        }
        rng.shuffle(&mut edges);
        let n = rooms.len();
        let mut group: Vec<usize> = (0..n).collect();
        fn find(g: &mut [usize], i: usize) -> usize {
            let mut r = i;
            while g[r] != r {
                r = g[r];
            }
            let mut c = i;
            while g[c] != r {
                let next = g[c];
                g[c] = r;
                c = next;
            }
            r
        }
        let mut kept: Vec<(usize, usize)> = Vec::new();
        let mut extra: Vec<(usize, usize)> = Vec::new();
        for (a, b) in edges {
            let (ra, rb) = (find(&mut group, a), find(&mut group, b));
            if ra != rb {
                group[ra] = rb;
                kept.push((a, b));
            } else {
                extra.push((a, b));
            }
        }
        // Loops: about half the spare doors, so there is more than one way round.
        for e in extra {
            if rng.next() < 0.55 {
                kept.push(e);
            }
        }

        // ---- carve
        let mut tiles = vec![vec!['#'; width as usize]; height as usize];
        let carve = |r: Rect, tiles: &mut Vec<Vec<char>>| {
            for p in r.tiles() {
                tiles[p.y as usize][p.x as usize] = ' ';
            }
        };
        for r in &rooms {
            carve(r.rect, &mut tiles);
        }
        let mut corridors: Vec<Rect> = Vec::new();
        for &(a, b) in &kept {
            let (ra, rb) = (rooms[a].rect, rooms[b].rect);
            let (left, right) = if ra.x1 < rb.x0 {
                (ra, rb)
            } else if rb.x1 < ra.x0 {
                (rb, ra)
            } else {
                (ra, rb)
            };
            let side_by_side = left.x1 < right.x0;
            let (up, down) = if ra.y1 < rb.y0 { (ra, rb) } else { (rb, ra) };
            let pieces: Vec<Rect> = if side_by_side
                && overlap(left.y0 + 1, left.y1 - 1, right.y0 + 1, right.y1 - 1).is_some_and(|(lo, hi)| hi - lo + 1 >= CORRIDOR)
            {
                let (lo, hi) = overlap(left.y0 + 1, left.y1 - 1, right.y0 + 1, right.y1 - 1).unwrap();
                let y0 = lo + rng.index((hi - lo + 2 - CORRIDOR) as usize) as i32;
                vec![Rect { x0: left.x1 + 1, y0, x1: right.x0 - 1, y1: y0 + CORRIDOR - 1 }]
            } else if !side_by_side
                && overlap(up.x0 + 1, up.x1 - 1, down.x0 + 1, down.x1 - 1).is_some_and(|(lo, hi)| hi - lo + 1 >= CORRIDOR)
            {
                let (lo, hi) = overlap(up.x0 + 1, up.x1 - 1, down.x0 + 1, down.x1 - 1).unwrap();
                let x0 = lo + rng.index((hi - lo + 2 - CORRIDOR) as usize) as i32;
                vec![Rect { x0, y0: up.y1 + 1, x1: x0 + CORRIDOR - 1, y1: down.y0 - 1 }]
            } else if side_by_side {
                // An elbow: out of the left room's east wall, then up or down into the right room.
                let y = (left.y0 + left.y1) / 2;
                let x = (right.x0 + right.x1) / 2;
                let ya = y.min(if right.y0 > y { right.y0 - 1 } else { right.y1 + 1 });
                let yb = y.max(if right.y0 > y { right.y0 - 1 } else { right.y1 + 1 });
                vec![Rect { x0: left.x1 + 1, y0: y - 1, x1: x + 1, y1: y + 1 }, Rect { x0: x - 1, y0: ya - 1, x1: x + 1, y1: yb + 1 }]
            } else {
                let x = (up.x0 + up.x1) / 2;
                let y = (down.y0 + down.y1) / 2;
                let xa = x.min(if down.x0 > x { down.x0 - 1 } else { down.x1 + 1 });
                let xb = x.max(if down.x0 > x { down.x0 - 1 } else { down.x1 + 1 });
                vec![Rect { x0: x - 1, y0: up.y1 + 1, x1: x + 1, y1: y + 1 }, Rect { x0: xa - 1, y0: y - 1, x1: xb + 1, y1: y + 1 }]
            };
            for r in pieces {
                let r = Rect { x0: r.x0.max(1), y0: r.y0.max(1), x1: r.x1.min(width - 2), y1: r.y1.min(height - 2) };
                carve(r, &mut tiles);
                corridors.push(r);
            }
        }

        // ---- columns: a ring in the hall, four in the larger chambers
        let mut pillars: Vec<P> = Vec::new();
        for r in &rooms {
            let rect = r.rect;
            let (w, h) = (rect.x1 - rect.x0 + 1, rect.y1 - rect.y0 + 1);
            match r.kind {
                RoomKind::Hall => {
                    for (fx, fy) in
                        [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75), (0.5, 0.25), (0.5, 0.75), (0.25, 0.5), (0.75, 0.5)]
                    {
                        pillars
                            .push(P::new(rect.x0 + (fx * (w - 1) as f32).round() as i32, rect.y0 + (fy * (h - 1) as f32).round() as i32));
                    }
                }
                RoomKind::Chamber if w >= 14 && h >= 14 => {
                    for (fx, fy) in [(0.3, 0.3), (0.7, 0.3), (0.3, 0.7), (0.7, 0.7)] {
                        pillars
                            .push(P::new(rect.x0 + (fx * (w - 1) as f32).round() as i32, rect.y0 + (fy * (h - 1) as f32).round() as i32));
                    }
                }
                _ => {}
            }
        }
        let hall = rooms.iter().find(|r| r.kind == RoomKind::Hall).unwrap().rect;
        let idol = P::new((hall.x0 + hall.x1) / 2, (hall.y0 + hall.y1) / 2);
        pillars.push(idol);
        for p in &pillars {
            tiles[p.y as usize][p.x as usize] = '#';
        }

        // ---- the portals: in the entry's west wall and the exit's east wall
        let entry = rooms.iter().position(|r| r.kind == RoomKind::Entry).unwrap();
        let exit = rooms.iter().position(|r| r.kind == RoomKind::Exit).unwrap();
        let er = rooms[entry].rect;
        let xr = rooms[exit].rect;
        let entry_gate = P::new(er.x0 - 1, (er.y0 + er.y1) / 2);
        let arrival = P::new(er.x0 + 1, entry_gate.y);
        let exit_gate = P::new(xr.x1 + 1, (xr.y0 + xr.y1) / 2);
        tiles[arrival.y as usize][arrival.x as usize] = 'P';
        tiles[exit_gate.y as usize][exit_gate.x as usize] = 'D';
        let spawn = P::new(xr.x1 - 1, exit_gate.y);
        tiles[spawn.y as usize][spawn.x as usize] = 'G';

        // ---- depth: rooms passed through from the entry
        let mut dist = vec![usize::MAX; n];
        dist[entry] = 0;
        let mut queue = std::collections::VecDeque::from([entry]);
        while let Some(i) = queue.pop_front() {
            for &(a, b) in &kept {
                let o = if a == i {
                    b
                } else if b == i {
                    a
                } else {
                    continue;
                };
                if dist[o] == usize::MAX {
                    dist[o] = dist[i] + 1;
                    queue.push_back(o);
                }
            }
        }
        let deepest = dist.iter().filter(|&&d| d != usize::MAX).max().copied().unwrap_or(1).max(1) as f64;
        for (i, r) in rooms.iter_mut().enumerate() {
            r.depth = if dist[i] == usize::MAX { 1.0 } else { dist[i] as f64 / deepest };
        }

        // ---- loot and packs
        let mut loot: Vec<Loot> = Vec::new();
        let mut packs: Vec<Pack> = Vec::new();
        let mut busy: HashSet<P> = HashSet::from([arrival, spawn]);
        busy.extend(pillars.iter().copied());
        let floor = |tiles: &Vec<Vec<char>>, p: P| tiles[p.y as usize][p.x as usize] == ' ';
        for r in &rooms {
            let rect = r.rect;
            let inner = Rect { x0: rect.x0 + 1, y0: rect.y0 + 1, x1: rect.x1 - 1, y1: rect.y1 - 1 };
            let area = ((inner.x1 - inner.x0 + 1) * (inner.y1 - inner.y0 + 1)) as usize;
            // Coins, gems and the packs keep off the room's edge; chests and heaps stand there, against the walls.
            let mut spots: Vec<P> = inner.tiles().filter(|&p| floor(&tiles, p) && !busy.contains(&p)).collect();
            rng.shuffle(&mut spots);
            let mut edge: Vec<P> = rect.tiles().filter(|&p| !inner.contains(p) && floor(&tiles, p) && !busy.contains(&p)).collect();
            rng.shuffle(&mut edge);
            // Against the walls (a doorway's tile touches no wall on that side, so chests never block one).
            let by_wall = |p: P| CARDINALS.iter().any(|&d| tiles[p.step(d).y as usize][p.step(d).x as usize] == '#');
            // Gold everywhere: the treasury is carpeted with it.
            let (coins, piles, treasures, chests, gems) = match r.kind {
                RoomKind::Entry => (area / 8, 2, 0, 2, 0),
                RoomKind::Chamber => (area / 4, 3 + rng.index(3), 1 + rng.index(2), 3 + rng.index(3), usize::from(rng.next() < 0.5)),
                RoomKind::Hall => (area / 4, 14, 8, 16, 5),
                RoomKind::Exit => (area / 4, 6, 3, 7, 2),
            };
            let take = |want: usize,
                        wall: bool,
                        kind: &mut dyn FnMut(&mut Rng) -> LootKind,
                        rng: &mut Rng,
                        spots: &mut Vec<P>,
                        loot: &mut Vec<Loot>,
                        busy: &mut HashSet<P>| {
                let mut got = 0;
                let mut i = 0;
                while got < want && i < spots.len() {
                    let p = spots[i];
                    if wall && !by_wall(p) {
                        i += 1;
                        continue;
                    }
                    spots.remove(i);
                    busy.insert(p);
                    loot.push(Loot { at: p, kind: kind(rng), taken: false });
                    got += 1;
                }
            };
            // Chests stand in rows of two or three along the walls.
            let mut placed = 0;
            let mut i = 0;
            while placed < chests && i < edge.len() {
                let p = edge[i];
                if !by_wall(p) || busy.contains(&p) {
                    i += 1;
                    continue;
                }
                let along = if p.x == rect.x0 || p.x == rect.x1 { (0, 1) } else { (1, 0) };
                let row = 2 + rng.index(2);
                for k in 0..row as i32 {
                    let q = P::new(p.x + along.0 * k, p.y + along.1 * k);
                    if placed < chests && rect.contains(q) && !inner.contains(q) && floor(&tiles, q) && !busy.contains(&q) && by_wall(q) {
                        busy.insert(q);
                        loot.push(Loot { at: q, kind: LootKind::Chest, taken: false });
                        placed += 1;
                    }
                }
                i += 1;
            }
            edge.retain(|p| !busy.contains(p));
            take(gems, false, &mut |rng: &mut Rng| LootKind::Gem(rng.pick(&GEMS)), &mut rng, &mut spots, &mut loot, &mut busy);
            take(treasures, true, &mut |_| LootKind::Treasure, &mut rng, &mut edge, &mut loot, &mut busy);
            take(piles, true, &mut |_| LootKind::Pile, &mut rng, &mut edge, &mut loot, &mut busy);
            take(piles, false, &mut |_| LootKind::Pile, &mut rng, &mut spots, &mut loot, &mut busy);
            // The packs take their posts before the coins carpet the rest of the floor.
            let mut posts_free: Vec<P> = spots.clone();
            take(coins, false, &mut |_| LootKind::Coin, &mut rng, &mut spots, &mut loot, &mut busy);
            posts_free.retain(|p| !busy.contains(p));
            // The pack: more, and more of the second stage, the deeper the room.
            let count = match r.kind {
                RoomKind::Entry => 0,
                RoomKind::Chamber => 3 + (r.depth * 3.0).round() as usize,
                RoomKind::Hall => 8,
                RoomKind::Exit => 6,
            };
            if count == 0 {
                continue;
            }
            let stage_two = (r.depth * 0.9).min(0.85);
            let mut foes = Vec::new();
            let mut posts: Vec<P> = Vec::new();
            for p in posts_free.iter().copied() {
                if foes.len() >= count {
                    break;
                }
                if posts.iter().any(|q| (q.x - p.x).abs() + (q.y - p.y).abs() < 2) {
                    continue;
                }
                let (kind, stage) = if rng.next() < stage_two {
                    (rng.pick(&[EnemyKind::Helmeted, EnemyKind::LargeBat, EnemyKind::Witch, EnemyKind::Werewolf, EnemyKind::Blemmyes]), 2)
                } else {
                    (rng.pick(&EnemyKind::STAGE_ONE), 1)
                };
                posts.push(p);
                foes.push((kind, stage, p));
            }
            packs.push(Pack { den: rect, foes });
        }

        let rows = tiles.iter().map(|r| r.iter().collect()).collect();
        Vault { width, height, rooms, corridors, pillars, loot, drops: Vec::new(), packs, arrival, entry_gate, exit_gate, idol, rows }
    }

    /// The vault as a level, at the pace of `base`.
    pub fn level(&self, base: &LevelData) -> LevelData {
        LevelData {
            name: Sin::Greed.realm().to_string(),
            rows: self.rows.clone(),
            heights: vec!["0".repeat(self.width as usize); self.height as usize],
            hero_speed: base.hero_speed,
            enemy_speed: base.enemy_speed,
            frenzy_seconds: base.frenzy_seconds,
            boss: false,
            theme: Theme::Realm(Sin::Greed),
        }
    }

    /// The room a tile lies in.
    pub fn room_at(&self, p: P) -> Option<&Room> {
        self.rooms.iter().find(|r| r.rect.contains(p))
    }

    /// What all the loot is worth together.
    pub fn worth(&self) -> u32 {
        self.loot.iter().map(|l| l.kind.worth()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::parse_level;
    use crate::logic::levels::level_for;
    use crate::logic::pathing::reachable;

    #[test]
    fn the_vault_is_twice_a_diablo_map_of_rooms_and_every_room_can_be_reached() {
        let v = Vault::generate(SEED);
        assert_eq!((v.width, v.height), (138, 102));
        assert_eq!(v.rooms.len(), 19);
        let level = parse_level(&v.level(&level_for(1)));
        assert_eq!(level.hero_spawn, v.arrival);
        assert_eq!(level.gate, Some(v.exit_gate));
        let seen = reachable(&level.grid, v.arrival, false, false);
        for r in &v.rooms {
            let (cx, cy) = r.rect.center();
            let c = P::new(cx as i32, cy as i32);
            let any = r.rect.tiles().any(|p| seen.contains(&p));
            assert!(any, "room {:?} at {c:?} cut off", r.kind);
        }
        assert!(seen.contains(&P::new(v.exit_gate.x - 1, v.exit_gate.y)), "the way to the far portal");
        for l in &v.loot {
            assert!(seen.contains(&l.at), "{:?} at {:?} out of reach", l.kind, l.at);
        }
        for pk in &v.packs {
            for &(kind, _, post) in &pk.foes {
                assert!(pk.den.contains(post) && seen.contains(&post), "{kind:?} at {post:?}");
            }
        }
        // Rooms are walled: no room touches another.
        for (i, a) in v.rooms.iter().enumerate() {
            for b in v.rooms.iter().skip(i + 1) {
                assert!(a.rect.x1 + 1 < b.rect.x0 || b.rect.x1 + 1 < a.rect.x0 || a.rect.y1 + 1 < b.rect.y0 || b.rect.y1 + 1 < a.rect.y0);
            }
        }
    }

    #[test]
    fn the_vault_is_full_of_gold_and_held_by_packs_of_both_stages() {
        let v = Vault::generate(SEED);
        let count = |k: fn(LootKind) -> bool| v.loot.iter().filter(|l| k(l.kind)).count();
        assert!(count(|k| k == LootKind::Coin) >= 800, "coins: {}", count(|k| k == LootKind::Coin));
        assert!(count(|k| k == LootKind::Chest) >= 60, "chests: {}", count(|k| k == LootKind::Chest));
        assert!(count(|k| k == LootKind::Treasure) >= 20, "treasure heaps");
        assert!(count(|k| matches!(k, LootKind::Gem(_))) >= 8, "gems");
        assert!(v.worth() > 30_000);
        // Chests stand against the walls, most of them beside another.
        let chests: HashSet<P> = v.loot.iter().filter(|l| l.kind == LootKind::Chest).map(|l| l.at).collect();
        let paired = chests.iter().filter(|p| CARDINALS.iter().any(|&d| chests.contains(&p.step(d)))).count();
        assert!(paired * 3 >= chests.len() * 2, "{paired} of {} chests stand in rows", chests.len());
        // Foes never start on gold: they stand on bare floor.
        let gold: HashSet<P> = v.loot.iter().map(|l| l.at).collect();
        assert!(v.packs.iter().flat_map(|p| p.foes.iter()).all(|f| !gold.contains(&f.2)));
        let foes: Vec<(EnemyKind, u8)> = v.packs.iter().flat_map(|p| p.foes.iter().map(|f| (f.0, f.1))).collect();
        assert!(foes.len() >= 60, "{} foes", foes.len());
        assert!(foes.iter().any(|f| f.1 == 1) && foes.iter().any(|f| f.1 == 2));
        assert!(v.packs.iter().all(|p| !p.foes.is_empty()));
        let entry = v.rooms.iter().find(|r| r.kind == RoomKind::Entry).unwrap();
        assert!(v.packs.iter().all(|p| p.den != entry.rect), "no foes where he arrives");
        // The deeper rooms lean on the second stage.
        let deep: Vec<&Pack> = v.packs.iter().filter(|p| v.room_at(p.foes[0].2).unwrap().depth > 0.7).collect();
        let two = deep.iter().flat_map(|p| p.foes.iter()).filter(|f| f.1 == 2).count();
        let all = deep.iter().map(|p| p.foes.len()).sum::<usize>();
        assert!(two * 2 >= all, "{two} of {all} deep foes are of the second stage");
    }

    #[test]
    fn chests_throw_out_items_and_the_deep_rooms_give_rarer_ones() {
        let mut rng = Rng::new(7);
        let mut legendary = [0usize; 2];
        for (k, depth) in [0.0, 1.0].into_iter().enumerate() {
            for _ in 0..4000 {
                let drops = roll_drops(&mut rng, depth);
                assert!((1..=3).contains(&drops.len()));
                legendary[k] += drops.iter().filter(|d| d.1 == Rarity::Legendary).count();
            }
        }
        assert!(legendary[1] > legendary[0] * 2, "legendary at the entry {}, deep {}", legendary[0], legendary[1]);
        // Every rarity has its own table, and every item has a name.
        for r in [Rarity::Common, Rarity::Magic, Rarity::Rare, Rarity::Legendary] {
            for _ in 0..50 {
                let it = roll_item(&mut rng, r);
                assert!(!it.label(|_| "Health").is_empty());
            }
        }
        assert_eq!(DropItem::Runes(5).label(|_| ""), "5 RUNE FRAGMENTS");
        assert_eq!(DropItem::Potion(PotionKind::Health).label(|_| "Minor Health Potion"), "MINOR HEALTH POTION", "named once");
    }
}
