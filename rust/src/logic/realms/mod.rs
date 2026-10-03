//! The six realms built after the vaults (REALMS.md has their designs): the Hall of Mirrors
//! (Pride), the Serpent Grotto (Envy), the Burning Field (Wrath), the Grey Mire (Sloth), the
//! Rotting Feast (Gluttony) and the Garden of Thorns (Lust).
//!
//! Each is laid out and kept here, free of rendering: its ground as a level, its foes in packs at
//! their dens (dealt by how far he has come, `Dealer`, whichever realm it is), and the state of its own rules (a meter, a layer, a drifting slab). What every
//! realm shares is its `Base`; what is its own lives in its module. `game::realms` plays them and
//! `render::view::realms` draws them.

pub mod feast;
pub mod field;
pub mod grotto;
pub mod mire;
pub mod mirrors;
pub mod thorns;

use super::combat::{Attack, Breed};
use super::enemy::EnemyKind;
use super::hub::Gate;
use super::level::{LevelData, Theme};
use super::sins::Sin;
use super::types::{Dir, P};
pub use super::vault::Pack;

/// What every realm has: its ground (rows of tiles and heights, as `logic::level` reads them),
/// where he arrives and the portal behind him, the portal to the prince's court, its packs, and
/// where he rises after a fall.
#[derive(Clone, Debug)]
pub struct Base {
    pub sin: Sin,
    pub width: i32,
    pub height: i32,
    pub rows: Vec<String>,
    pub heights: Vec<String>,
    /// Where he arrives (the level's `P`), and the portal he came through behind him.
    pub arrival: P,
    pub entry: Gate,
    /// The portal to the prince's court (the level's `D`): sealed until the realm's last trap has
    /// been survived (`Game::gate_open`).
    pub exit: Gate,
    /// Its natives, each pack at its den.
    pub packs: Vec<Pack>,
    /// Where he rises after a fall: the arrival, until the realm's rules move it on.
    pub checkpoint: P,
    /// How hard it was laid out (`Game::realm_tier`).
    pub tier: usize,
}

impl Base {
    /// The realm as a level, at the pace of `base`.
    pub fn level(&self, base: &LevelData) -> LevelData {
        LevelData {
            name: self.sin.realm().to_string(),
            rows: self.rows.clone(),
            heights: self.heights.clone(),
            hero_speed: base.hero_speed,
            enemy_speed: base.enemy_speed,
            frenzy_seconds: base.frenzy_seconds,
            boss: false,
            theme: Theme::Realm(self.sin),
        }
    }
}

/// A realm being played: its layout and the state of its rules.
#[derive(Clone, Debug)]
pub enum Run {
    Pride(Box<mirrors::Mirrors>),
    Envy(Box<grotto::Grotto>),
    Wrath(Box<field::Battlefield>),
    Sloth(Box<mire::Mire>),
    Gluttony(Box<feast::Feast>),
    Lust(Box<thorns::Thorns>),
}

impl Run {
    /// The realm of `sin` laid out for `tier` (none for Greed: the vaults are `logic::vault`).
    pub fn generate(sin: Sin, tier: usize) -> Option<Run> {
        Some(match sin {
            Sin::Pride => Run::Pride(Box::new(mirrors::Mirrors::generate(tier))),
            Sin::Envy => Run::Envy(Box::new(grotto::Grotto::generate(tier))),
            Sin::Wrath => Run::Wrath(Box::new(field::Battlefield::generate(tier))),
            Sin::Sloth => Run::Sloth(Box::new(mire::Mire::generate(tier))),
            Sin::Gluttony => Run::Gluttony(Box::new(feast::Feast::generate(tier))),
            Sin::Lust => Run::Lust(Box::new(thorns::Thorns::generate(tier))),
            Sin::Greed => return None,
        })
    }

    pub fn sin(&self) -> Sin {
        self.base().sin
    }

    pub fn base(&self) -> &Base {
        match self {
            Run::Pride(r) => &r.base,
            Run::Envy(r) => &r.base,
            Run::Wrath(r) => &r.base,
            Run::Sloth(r) => &r.base,
            Run::Gluttony(r) => &r.base,
            Run::Lust(r) => &r.base,
        }
    }

    pub fn base_mut(&mut self) -> &mut Base {
        match self {
            Run::Pride(r) => &mut r.base,
            Run::Envy(r) => &mut r.base,
            Run::Wrath(r) => &mut r.base,
            Run::Sloth(r) => &mut r.base,
            Run::Gluttony(r) => &mut r.base,
            Run::Lust(r) => &mut r.base,
        }
    }
}

/// The realm of `sin` as a level, laid out for `tier` (none for Greed).
pub fn level(sin: Sin, tier: usize, base: &LevelData) -> Option<LevelData> {
    Run::generate(sin, tier).map(|r| r.base().level(base))
}

/// The deepest stage of foe a realm of `tier` sends: one past its tier, from the second to the seventh.
pub fn deepest_stage(tier: usize) -> u8 {
    (tier as u8).saturating_add(1).clamp(2, 7)
}

/// The stages a realm of `tier` brings in fresh, whose foes lead its packs: the deepest it sends,
/// and in the last realm (every other key held) the two deepest.
pub fn fresh_stages(tier: usize) -> std::ops::RangeInclusive<u8> {
    let deepest = deepest_stage(tier);
    let from = if tier >= 7 { deepest - 1 } else { deepest };
    from..=deepest
}

/// Every foe a realm of `tier` sends, with its stage: all those met so far (the Garden's first, then
/// one stage more for every realm taken, whichever it was), the witch's minions aside (they come
/// with the witches).
pub fn roster(tier: usize) -> Vec<(EnemyKind, u8)> {
    (1..=deepest_stage(tier))
        .flat_map(|s| EnemyKind::of_stage(s).iter().map(move |&k| (k, s)))
        .filter(|&(k, _)| k != EnemyKind::Minion)
        .collect()
}

/// A foe to deal, and the stage it was built for.
type Foe = (EnemyKind, u8);

/// For one kind of fighting: the first round (fresh, then old), and the turns after it.
type Queue = (Vec<Foe>, Vec<Foe>);

/**
 * Deals the foes of a realm of `tier` to its packs, the same way every time (`roster`): by turns
 * one that fights with its body, one in melee and one from afar, so every pack mixes the three.
 * Of each kind of fighting it deals first every foe of the fresh stages (`fresh_stages`), then
 * every one met before, the latest first, so all of them are in a realm with room for them (and
 * where there is less, the oldest and weakest are left out); after that, by turns one fresh and
 * one old.
 */
pub struct Dealer {
    queues: [Queue; 3],
    dealt: [usize; 3],
    turn: usize,
}

impl Dealer {
    pub fn new(tier: usize) -> Self {
        let fresh = fresh_stages(tier);
        let all = roster(tier);
        let queues = [Attack::Body, Attack::Melee, Attack::Distance].map(|way| {
            let fights = |&&(k, _): &&(EnemyKind, u8)| Breed::of(k).attack() == way;
            // (The latest stage first, in both.)
            let mut new: Vec<(EnemyKind, u8)> = all.iter().filter(fights).filter(|n| fresh.contains(&n.1)).copied().collect();
            let mut old: Vec<(EnemyKind, u8)> = all.iter().filter(fights).filter(|n| !fresh.contains(&n.1)).copied().collect();
            new.sort_by_key(|n| std::cmp::Reverse(n.1));
            old.sort_by_key(|n| std::cmp::Reverse(n.1));
            let first: Vec<(EnemyKind, u8)> = new.iter().chain(&old).copied().collect();
            let mut turns = Vec::new();
            for i in 0..new.len().max(old.len()) {
                if !new.is_empty() {
                    turns.push(new[i % new.len()]);
                }
                if let Some(&o) = old.get(i) {
                    turns.push(o);
                }
            }
            (first, turns)
        });
        Self { queues, dealt: [0; 3], turn: 0 }
    }

    /// The next foe, and the stage it was built for.
    pub fn draw(&mut self) -> Foe {
        // (Every stage has all three kinds of fighting, so no queue is empty; this is a guard.)
        for _ in 0..3 {
            let k = self.turn % 3;
            self.turn += 1;
            let (first, turns) = &self.queues[k];
            let n = self.dealt[k];
            if let Some(&f) = first.get(n) {
                self.dealt[k] += 1;
                return f;
            }
            if !turns.is_empty() {
                self.dealt[k] += 1;
                return turns[(n - first.len()) % turns.len()];
            }
        }
        (EnemyKind::Skeleton, 1)
    }
}

/// What a foe built for `stage` (0 for a realm's own) is multiplied by in a realm of `tier`: its
/// health and its damage (on top of the brawl's weight for the realms, `brawl::VAULT_DAMAGE`).
/// Natives fight as the vaults' packs do; a realm's own new foes grow with the tier, since they come
/// at every depth.
pub fn weight(stage: u8, tier: usize) -> (f64, f64) {
    if stage == 0 {
        let t = tier.max(1) as f64;
        (0.7 + 0.25 * t, 0.9 + 0.12 * t)
    } else {
        (0.85, 1.0)
    }
}

/// A plan for a realm's ground: tiles and heights as characters (see `logic::level`), every tile
/// rock until carved.
#[derive(Clone, Debug)]
pub struct Canvas {
    pub width: i32,
    pub height: i32,
    pub tiles: Vec<Vec<char>>,
    pub heights: Vec<Vec<char>>,
}

impl Canvas {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            tiles: vec![vec!['#'; width as usize]; height as usize],
            heights: vec![vec!['0'; width as usize]; height as usize],
        }
    }

    pub fn inside(&self, p: P) -> bool {
        p.x >= 0 && p.y >= 0 && p.x < self.width && p.y < self.height
    }

    pub fn get(&self, p: P) -> char {
        if self.inside(p) {
            self.tiles[p.y as usize][p.x as usize]
        } else {
            '#'
        }
    }

    pub fn set(&mut self, p: P, c: char) {
        if self.inside(p) {
            self.tiles[p.y as usize][p.x as usize] = c;
        }
    }

    /// Floor elevation (0 to 9), or `/` for a ramp.
    pub fn set_height(&mut self, p: P, h: char) {
        if self.inside(p) {
            self.heights[p.y as usize][p.x as usize] = h;
        }
    }

    /// Every tile of the inclusive rectangle set to `c`.
    pub fn fill(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: char) {
        for y in y0.min(y1)..=y0.max(y1) {
            for x in x0.min(x1)..=x0.max(x1) {
                self.set(P::new(x, y), c);
            }
        }
    }

    /// A disc of `c` round `(cx, cy)`.
    pub fn disc(&mut self, cx: f64, cy: f64, r: f64, c: char) {
        let (x0, x1) = ((cx - r).floor() as i32, (cx + r).ceil() as i32);
        let (y0, y1) = ((cy - r).floor() as i32, (cy + r).ceil() as i32);
        for y in y0..=y1 {
            for x in x0..=x1 {
                if (x as f64 - cx).powi(2) + (y as f64 - cy).powi(2) <= r * r {
                    self.set(P::new(x, y), c);
                }
            }
        }
    }

    /// The rows and heights, for a level.
    pub fn finish(&self) -> (Vec<String>, Vec<String>) {
        (self.tiles.iter().map(|r| r.iter().collect()).collect(), self.heights.iter().map(|r| r.iter().collect()).collect())
    }
}

/// A portal set in a wall: on `tile`, opening toward `facing` (the way he stands before it).
pub fn gate(sin: Sin, tile: P, facing: Dir) -> Gate {
    Gate { tile, facing, sin }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::parse_level;
    use crate::logic::levels::level_for;
    use crate::logic::pathing::reachable;
    use crate::logic::sins::SINS;

    #[test]
    fn every_realm_but_the_vaults_is_laid_out_and_its_far_portal_can_be_reached() {
        for sin in SINS {
            let Some(run) = Run::generate(sin, 1) else {
                assert_eq!(sin, Sin::Greed);
                continue;
            };
            let b = run.base();
            assert_eq!(run.sin(), sin);
            let level = parse_level(&b.level(&level_for(1)));
            assert_eq!(level.hero_spawn, b.arrival, "{sin:?}");
            assert_eq!(level.gate, Some(b.exit.tile), "{sin:?}");
            assert_eq!(level.data.name, sin.realm());
            assert_eq!(level.data.theme, Theme::Realm(sin));
            let seen = reachable(&level.grid, b.arrival, false, false);
            assert!(seen.contains(&b.exit.front()), "{sin:?}: the way to the far portal");
            for pk in &b.packs {
                for &(kind, _, post) in &pk.foes {
                    assert!(pk.den.contains(post), "{sin:?}: {kind:?} at {post:?} outside its den");
                    assert!(level.grid.is_walkable(post.x, post.y, false), "{sin:?}: {kind:?} on rock at {post:?}");
                }
            }
        }
    }

    #[test]
    fn every_realm_holds_every_foe_of_the_stage_it_brings_in() {
        for sin in SINS.into_iter().filter(|&s| s != Sin::Greed) {
            for tier in 1..=7 {
                let run = Run::generate(sin, tier).unwrap();
                let held: Vec<EnemyKind> = run.base().packs.iter().flat_map(|p| p.foes.iter().map(|f| f.0)).collect();
                for &kind in EnemyKind::of_stage(deepest_stage(tier)).iter().filter(|&&k| k != EnemyKind::Minion) {
                    assert!(held.contains(&kind), "{sin:?} at tier {tier} has no {kind:?}");
                }
            }
        }
    }

    #[test]
    fn each_realm_taken_brings_in_a_stage_more_dealt_by_turns_and_the_same_every_time() {
        assert_eq!((deepest_stage(1), deepest_stage(3), deepest_stage(9)), (2, 4, 7));
        assert_eq!((fresh_stages(1), fresh_stages(6), fresh_stages(7)), (2..=2, 7..=7, 6..=7));
        for tier in 1..=7 {
            let all = roster(tier);
            let deal = |n: usize| -> Vec<(EnemyKind, u8)> {
                let mut d = Dealer::new(tier);
                (0..n).map(|_| d.draw()).collect()
            };
            let dealt = deal(200);
            assert_eq!(dealt, deal(200), "tier {tier}: dealt the same every time");
            // Every foe met so far comes, and none deeper.
            for n in &all {
                assert!(dealt.contains(n), "tier {tier}: {n:?} never dealt");
            }
            assert!(dealt.iter().all(|n| all.contains(n) && n.1 <= deepest_stage(tier)));
            // Every three in a row fight all three ways.
            for three in dealt.as_chunks::<3>().0 {
                let ways: Vec<Attack> = three.iter().map(|n| Breed::of(n.0).attack()).collect();
                assert!(ways.contains(&Attack::Body) && ways.contains(&Attack::Melee) && ways.contains(&Attack::Distance));
            }
            // The fresh stages' foes come first, every one of them in the first dozen.
            let fresh: Vec<_> = all.iter().filter(|n| fresh_stages(tier).contains(&n.1)).collect();
            assert!(fresh.iter().all(|n| dealt[..12].contains(n)), "tier {tier}: {:?}", &dealt[..12]);
            // Every foe met so far within the first round.
            for n in &all {
                assert!(dealt[..all.len() + 6].contains(n), "tier {tier}: {n:?} late");
            }
        }
        let (hp1, dmg1) = weight(0, 1);
        let (hp7, dmg7) = weight(0, 7);
        assert!(hp7 > hp1 && dmg7 > dmg1, "a realm's own foes grow with the tier");
    }
}
