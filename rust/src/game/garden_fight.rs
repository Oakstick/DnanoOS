//! The Garden of Krampus fights by frame data (B01): the brawl of the vaults (`brawl`, COMBAT.md
//! §9) on a beginner's terms.
//!
//! - **Its foes are gentle duelists:** the first stage's four (the draugr a brute, the two ravens
//!   skirmishers, the bog slime a hexer), their wind-ups drawn out by `GARDEN_TELL` frames, their
//!   blows at `GARDEN_DAMAGE` of a breed's, their bodies at `GARDEN_HP`.
//! - **Few at once:** the threat manager lets two points of foes strike in turn (`GARDEN_CAPACITY`,
//!   one token every `GARDEN_CADENCE` frames): at most two foes attacking, never together.
//! - **Packs by the keys:** each golden key has a den near it, asleep until he comes close, as the
//!   vaults' packs are (`garden_dens`). The first three, nearest his start, hold one foe each and ask
//!   the three questions in turn: the brute (can you wait?), the skirmisher (can you read the
//!   timing?), the hexer (can you close the distance?). Farther out they come in twos.
//! - **The Garden takes every key badly** (its escalation, as ever): hunters come for him on the old
//!   schedule (`Game::spawn_every`, sooner with every key, and one at once after each key), a
//!   draugr clawing out of a grave or a raven out of the hollow; more of them may hunt him at once
//!   as the keys mount up (`hunters_allowed`). They hunt until slain.
//! - The keys, the north gate, the layout, the tour and the story beats are as they were; the
//!   ground's terraces hold the foes to the ramps as they hold him (`Flow::climbing`).

use super::brawl::{Brawl, Field};
use super::Game;
use crate::audio::Sfx;
use crate::logic::combat::duelist::{Den, Lore, RISE_FRAMES};
use crate::logic::combat::{Breed, Duelist};
use crate::logic::enemy::{spec_of, Enemy, EnemyKind, EnemyMode};
use crate::logic::grid::Grid;
use crate::logic::hub::Rect;
use crate::logic::types::{Tile, CARDINALS, P, V};
use crate::logic::vault::Pack;
use std::collections::{HashMap, VecDeque};

/// The Garden's pace: threat points, and frames between attack tokens (the vaults' are 2 and 60).
pub const GARDEN_CAPACITY: u8 = 2;
pub const GARDEN_CADENCE: u32 = 75;
/// Frames added to every wind-up of the Garden's foes.
pub const GARDEN_TELL: u16 = 12;
/// What the Garden's foes' blows are multiplied by (the vaults' 0.8), and their health (0.85).
pub const GARDEN_DAMAGE: f64 = 0.6;
pub const GARDEN_HP: f64 = 0.8;
/// No den nearer his start than this many steps, nor in a straight line than this many tiles
/// (nothing wakes while Hızır teaches).
const DEN_FROM_START: u32 = 14;
const DEN_FROM_START_LINE: f64 = 9.0;
/// A den's post lies this many steps from its key, at most.
const DEN_FROM_KEY: u32 = 4;
/// Tiles a den's room reaches either way from its post.
const DEN_HALF: i32 = 3;
/// The packs, nearest key first: three lone foes asking the three questions, then twos.
const PACKS: [&[EnemyKind]; 8] = [
    &[EnemyKind::Skeleton],
    &[EnemyKind::Bat],
    &[EnemyKind::Slime],
    &[EnemyKind::Vampire],
    &[EnemyKind::Skeleton, EnemyKind::Bat],
    &[EnemyKind::Slime, EnemyKind::Vampire],
    &[EnemyKind::Skeleton, EnemyKind::Slime],
    &[EnemyKind::Bat, EnemyKind::Vampire],
];
/// The hunters, in turn (the old reinforcements' order).
const HUNTERS: [EnemyKind; 4] = [EnemyKind::Bat, EnemyKind::Slime, EnemyKind::Vampire, EnemyKind::Skeleton];

/// Steps over the ground from `from` (as he walks it: no climbs but up the ramps).
pub(super) fn walk_steps(grid: &Grid, from: P) -> HashMap<P, u32> {
    let mut dist = HashMap::from([(from, 0)]);
    let mut queue = VecDeque::from([from]);
    while let Some(p) = queue.pop_front() {
        let d = dist[&p];
        for dir in CARDINALS {
            if !grid.can_move(p, dir, false, false) {
                continue;
            }
            let n = grid.neighbor(p, dir);
            if let std::collections::hash_map::Entry::Vacant(e) = dist.entry(n) {
                e.insert(d + 1);
                queue.push_back(n);
            }
        }
    }
    dist
}

/// How many hunters may be out for him at once with `keys` of the Garden's keys taken.
pub fn hunters_allowed(keys: u32) -> usize {
    1 + keys as usize / 3
}

impl Game {
    /// In the Garden of Krampus, fought by frame data.
    pub fn in_garden_fight(&self) -> bool {
        self.brawl.as_ref().is_some_and(|b| b.garden)
    }

    /// The Garden's fight as its level is loaded: the gentle brawl, its dens asleep by the keys.
    pub(super) fn enter_garden(&mut self) {
        let mut brawl = Brawl::new(Field::Vault);
        brawl.garden = true;
        brawl.threat.capacity = GARDEN_CAPACITY;
        brawl.threat.cadence = GARDEN_CADENCE;
        brawl.foe_damage = GARDEN_DAMAGE;
        brawl.reset(self.hero_kit());
        let packs = self.garden_dens();
        self.enemies = self.garden_garrison(&packs, &mut brawl);
        self.brawl = Some(brawl);
    }

    /// Where the Garden's packs sleep: a den by each golden key (nearest his start first), its post
    /// a few steps from the key on the roomiest floor, clear of his start, the shrines, the bottles,
    /// the gate and the hollow.
    pub(super) fn garden_dens(&self) -> Vec<Pack> {
        let g = &self.level.grid;
        let spawn = self.level.hero_spawn;
        let from_start = walk_steps(g, spawn);
        let mut keys: Vec<(u32, P)> = g
            .cells()
            .filter(|&(_, _, t)| t == Tile::Nodule)
            .map(|(x, y, _)| P::new(x, y))
            .map(|p| (from_start.get(&p).copied().unwrap_or(u32::MAX), p))
            .collect();
        keys.sort_by_key(|&(d, p)| (d, p.y, p.x));
        let mut avoid: Vec<P> = self.level.angel_shrines.iter().chain(&self.level.item_shrines).copied().collect();
        avoid.extend(self.level.potions.iter().map(|(p, _)| *p));
        avoid.extend(self.level.gate);
        avoid.push(self.level.enemy_spawn);
        avoid.push(self.level.nest_center);
        let open = |p: P| g.get(p.x, p.y) == Tile::Open && !g.is_bridge(p.x, p.y) && !g.is_ramp(p.x, p.y);
        let room =
            |p: P| (-2..=2).flat_map(|dy| (-2..=2).map(move |dx| (dx, dy))).filter(|&(dx, dy)| open(P::new(p.x + dx, p.y + dy))).count();
        let mut posts: Vec<P> = Vec::new();
        let mut out = Vec::new();
        for (k, &(_, key)) in keys.iter().enumerate().take(PACKS.len()) {
            let near = walk_steps(g, key);
            let post = near
                .iter()
                .filter(|&(&p, &d)| (1..=DEN_FROM_KEY).contains(&d) && open(p))
                .filter(|&(p, _)| from_start.get(p).is_some_and(|&s| s >= DEN_FROM_START))
                .filter(|&(p, _)| crate::logic::types::dist_sq(p.v(), spawn.v()) >= DEN_FROM_START_LINE * DEN_FROM_START_LINE)
                .filter(|&(p, _)| avoid.iter().all(|a| (a.x - p.x).abs() + (a.y - p.y).abs() >= 3))
                .filter(|&(p, _)| posts.iter().all(|q| (q.x - p.x).abs() + (q.y - p.y).abs() >= 6))
                .max_by_key(|&(p, &d)| (room(*p), std::cmp::Reverse(d.abs_diff(2)), std::cmp::Reverse((p.y, p.x))))
                .map(|(&p, _)| p);
            let Some(post) = post else { continue };
            posts.push(post);
            let den = Rect {
                x0: (post.x - DEN_HALF).max(0),
                y0: (post.y - DEN_HALF).max(0),
                x1: (post.x + DEN_HALF).min(g.width - 1),
                y1: (post.y + DEN_HALF).min(g.height - 1),
            };
            // The first at the post, a second beside it.
            let mut spots = vec![post];
            spots.extend(
                CARDINALS.iter().filter(|&&d| g.can_move(post, d, false, false)).map(|&d| g.neighbor(post, d)).filter(|&p| open(p)),
            );
            let foes = PACKS[k].iter().enumerate().map(|(i, &kind)| (kind, 1u8, spots[i.min(spots.len() - 1)])).collect();
            out.push(Pack { den, foes });
        }
        out
    }

    /// A gentle duelist of `kind` at `at` facing `facing` (its body and its blows the Garden's).
    fn garden_duelist(&self, brawl: &mut Brawl, kind: EnemyKind, at: P, facing: V, name: String) -> Enemy {
        let mut spec = spec_of(&self.level.grid, kind);
        spec.name = name;
        let mut e = Enemy::new(&spec, at, self.level.data.enemy_speed);
        e.b.mode = EnemyMode::Chase;
        let mut breed = Breed::of(kind);
        breed.tell = GARDEN_TELL;
        let mut d = Duelist::bred(brawl.next_id(), breed, at.v(), facing);
        // (New Game+: heavier, as everywhere.)
        let (hp, damage) = self.ng_foe((GARDEN_HP, 1.0));
        d.max_hp *= hp;
        d.hp = d.max_hp;
        d.breed.damage *= damage;
        e.duel = Some(Box::new(d));
        e
    }

    /// The packs: every foe at its post in its den, asleep there.
    fn garden_garrison(&self, packs: &[Pack], brawl: &mut Brawl) -> Vec<Enemy> {
        let mut out = Vec::new();
        for pack in packs {
            let r = pack.den;
            let (cx, cy) = r.center();
            for &(kind, _, post) in &pack.foes {
                let facing = V::new(cx as f64 - post.x as f64 + 0.01, cy as f64 - post.y as f64 + 0.01);
                let name = format!("{} {}", spec_of(&self.level.grid, kind).name, out.len() + 1);
                let mut e = self.garden_duelist(brawl, kind, post, facing, name);
                e.post = Some(post);
                if let Some(d) = e.duel.as_deref_mut() {
                    let lo = V::new(r.x0 as f64 - 0.5, r.y0 as f64 - 0.5);
                    let hi = V::new(r.x1 as f64 + 0.5, r.y1 as f64 + 0.5);
                    d.den = Some(Den { lo, hi, home: post.v(), awake: false, returning: false });
                }
                out.push(e);
            }
        }
        out
    }

    /// The hunters out for him now (no den: they hunt until slain).
    fn garden_hunters(&self) -> usize {
        self.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.counts() && d.den.is_none() && !d.puppet).count()
    }

    /// The Garden's rules each tick: the slain cleared away, and the hunters on their schedule.
    pub(super) fn update_garden(&mut self, dt: f64) {
        if !self.in_garden_fight() {
            return;
        }
        if self.enemies.iter().any(|e| e.mode() == EnemyMode::Eaten) {
            self.enemies.retain(|e| e.mode() != EnemyMode::Eaten);
            self.view.clear_enemies();
            for e in &self.enemies {
                self.view.add_enemy(e);
            }
        }
        if self.lesson_holds() {
            return;
        }
        self.spawn_timer -= dt;
        if self.spawn_timer > 0.0 || self.garden_hunters() >= hunters_allowed(self.keys_got) {
            return;
        }
        self.spawn_timer = self.spawn_every();
        self.raise_hunter();
    }

    /// One hunter comes for him: a draugr out of a grave, or a raven or the slime out of the hollow.
    fn raise_hunter(&mut self) {
        let kind = HUNTERS[self.spawned % HUNTERS.len()];
        let graves = self.grave_spawns();
        let hp = self.hero.pos();
        let grave = (kind == EnemyKind::Skeleton && !graves.is_empty()).then(|| graves[self.rng.index(graves.len())]);
        let at = grave.unwrap_or_else(|| self.hollow_door());
        let Some(mut brawl) = self.brawl.take() else { return };
        let name = format!("{} {}", spec_of(&self.level.grid, kind).name, self.spawned / HUNTERS.len() + 2);
        let facing = V::new(hp.x - at.x as f64, hp.y - at.y as f64 + 0.01);
        let mut e = self.garden_duelist(&mut brawl, kind, at, facing, name);
        if let (Some(d), true) = (e.duel.as_deref_mut(), grave.is_some()) {
            d.lore = Lore { rise: RISE_FRAMES, ..Lore::default() };
        }
        self.brawl = Some(brawl);
        self.view.add_enemy(&e);
        self.enemies.push(e);
        self.spawned += 1;
        self.audio.play(Sfx::Fuse);
        if grave.is_some() {
            self.view.dust(&self.level.grid, at.x as f64, at.y as f64, 0x6a5a48, 16, 1.4);
            self.floater(at.x as f64, at.y as f64 - 0.6, "THE DEAD RISE", 1.6, Some(0xe9e9f2));
        } else {
            self.floater(at.x as f64, at.y as f64 - 1.0, "SOMETHING STIRS", 1.6, Some(0xb06cff));
        }
    }

    /// The open floor at the hollow's door, where what lives in it comes out.
    fn hollow_door(&self) -> P {
        let g = &self.level.grid;
        let door = self.level.enemy_spawn;
        if g.is_walkable(door.x, door.y, false) {
            return door;
        }
        let steps = walk_steps(g, self.level.hero_spawn);
        steps.keys().copied().min_by_key(|p| ((p.x - door.x).pow(2) + (p.y - door.y).pow(2), p.y, p.x)).unwrap_or(door)
    }

    /// After a fall: the hunters are gone (the dens wake again from sleep), the schedule begins again.
    pub(super) fn reset_garden(&mut self) {
        if !self.in_garden_fight() {
            return;
        }
        let before = self.enemies.len();
        self.enemies.retain(|e| e.duel.as_ref().is_none_or(|d| d.den.is_some() || d.puppet));
        if self.enemies.len() != before {
            self.view.clear_enemies();
            for e in &self.enemies {
                self.view.add_enemy(e);
            }
        }
        self.spawn_timer = super::SPAWN_FIRST;
    }
}
