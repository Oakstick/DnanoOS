//! The six realms built after the vaults (REALMS.md has their designs): the Hall of Mirrors
//! (Pride), the Serpent Grotto (Envy), the Burning Field (Wrath), the Grey Mire (Sloth), the
//! Rotting Feast (Gluttony) and the Garden of Thorns (Lust).
//!
//! Each is laid out and kept here, free of rendering: its ground as a level, its natives in packs
//! at their dens, and the state of its own rules (a meter, a layer, a drifting slab). What every
//! realm shares is its `Base`; what is its own lives in its module. `game::realms` plays them and
//! `render::view::realms` draws them.

pub mod feast;
pub mod field;
pub mod grotto;
pub mod mire;
pub mod mirrors;
pub mod thorns;

use super::enemy::EnemyKind;
use super::hub::Gate;
use super::level::{LevelData, Theme};
use super::rng::Rng;
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

/// The natives of each realm (REALMS.md), each with the stage it was built for. The realms' own
/// new foes (the servants of Hel, the gaki, the elf-maids, the Peerless) are placed by the realms
/// themselves.
pub fn natives(sin: Sin) -> &'static [(EnemyKind, u8)] {
    use EnemyKind as K;
    match sin {
        // The vain queen's witches and the palace guard; monks too proud for paradise; the cin behind the
        // mirrors; a heart frozen by a shard of the devil's mirror.
        Sin::Pride => &[(K::Helmeted, 2), (K::Witch, 2), (K::Tengu, 3), (K::Cin, 5), (K::FrostGiant, 7)],
        // The grotto's slime and bats; the serpents and the river imp; the dragon on the spring; the malice-striker.
        Sin::Envy => &[(K::Slime, 1), (K::LargeBat, 2), (K::Cobra, 3), (K::Kappa, 4), (K::Ejder, 6), (K::Nidhogg, 7)],
        // Amon's ravens and wolves, the Einherjar, the rasetsu and the oni, the wrathful dead, the fire giant.
        Sin::Wrath => &[
            (K::Bat, 1),
            (K::Skeleton, 1),
            (K::Helmeted, 2),
            (K::Werewolf, 2),
            (K::DireWolf, 3),
            (K::Rasetsu, 3),
            (K::Oni, 4),
            (K::Goryo, 6),
            (K::Surtr, 7),
        ],
        // The bog's slime, the barrow's draugr, the sleeping bats, the silent owl, the night hag, Hel.
        Sin::Sloth => &[(K::Slime, 1), (K::Skeleton, 1), (K::LargeBat, 2), (K::Tulukus, 5), (K::Albasti, 5), (K::Hel, 7)],
        // The digesting slime, the witch of the house of cake, the blemmyes, the skeleton of the starved, the one-eyed glutton.
        Sin::Gluttony => &[(K::Slime, 1), (K::Witch, 2), (K::Blemmyes, 2), (K::Gashadokuro, 6), (K::Tepegoz, 6)],
        // The seductress and the bats; the spider-woman, the fox-wife and the courtesan; the voice of a loved one;
        // the garden's guardian.
        Sin::Lust => {
            &[(K::Witch, 2), (K::LargeBat, 2), (K::Jorogumo, 3), (K::Kitsune, 4), (K::Rokurokubi, 4), (K::Karakoncolos, 5), (K::Arcura, 5)]
        }
        Sin::Greed => &[],
    }
}

/// The deepest stage of foe a realm of `tier` sends: one past its tier, from the second to the seventh.
pub fn deepest_stage(tier: usize) -> u8 {
    (tier as u8).saturating_add(1).clamp(2, 7)
}

/// A native of `sin`'s realm for `tier`: one built for no deeper a stage than the tier allows, the
/// deeper ones more often.
pub fn pick_native(rng: &mut Rng, sin: Sin, tier: usize) -> (EnemyKind, u8) {
    let deepest = deepest_stage(tier);
    let all = natives(sin);
    let mut fit: Vec<(EnemyKind, u8)> = all.iter().copied().filter(|n| n.1 <= deepest).collect();
    if fit.is_empty() {
        // (Every realm has natives from the second stage, so this is only a guard.)
        fit.push(all.iter().copied().min_by_key(|n| n.1).unwrap_or((EnemyKind::Helmeted, 2)));
    }
    let weights: Vec<f64> = fit.iter().map(|n| (n.1 as f64).powf(1.5)).collect();
    let mut r = rng.next() * weights.iter().sum::<f64>();
    for (k, w) in weights.iter().enumerate() {
        if r < *w {
            return fit[k];
        }
        r -= w;
    }
    fit[fit.len() - 1]
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

/// A plain hall standing in for a realm not yet built: the portal in its west wall, the far portal
/// in its east wall, two packs of its natives between.
pub fn placeholder(sin: Sin, tier: usize, seed: u32) -> Base {
    use super::hub::Rect;
    let (w, h) = (44, 30);
    let mut c = Canvas::new(w, h);
    c.fill(3, 3, w - 4, h - 4, ' ');
    let mid = h / 2;
    let entry = gate(sin, P::new(2, mid), Dir::Right);
    let exit = gate(sin, P::new(w - 3, mid), Dir::Left);
    let arrival = P::new(4, mid);
    c.set(arrival, 'P');
    c.set(exit.tile, 'D');
    c.set(P::new(w - 5, mid), 'G');
    let mut rng = Rng::new(seed);
    let mut packs = Vec::new();
    for (den, count) in [(Rect { x0: 12, y0: 6, x1: 20, y1: h - 7 }, 3), (Rect { x0: 24, y0: 6, x1: 34, y1: h - 7 }, 4)] {
        let foes = (0..count)
            .map(|k| {
                let (kind, stage) = pick_native(&mut rng, sin, tier);
                (kind, stage, P::new(den.x0 + 2 + 2 * k, den.y0 + 3 + 3 * k))
            })
            .collect();
        packs.push(Pack { den, foes });
    }
    let (rows, heights) = c.finish();
    Base { sin, width: w, height: h, rows, heights, arrival, entry, exit, packs, checkpoint: arrival, tier }
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
    fn natives_come_by_the_tier_and_every_realm_has_some_for_the_first() {
        let mut rng = Rng::new(3);
        for sin in SINS.into_iter().filter(|&s| s != Sin::Greed) {
            assert!(natives(sin).iter().any(|n| n.1 <= 2), "{sin:?} has natives for the first realm");
            for tier in 1..=7 {
                for _ in 0..40 {
                    let (kind, stage) = pick_native(&mut rng, sin, tier);
                    assert!(stage <= deepest_stage(tier), "{sin:?} tier {tier}: {kind:?} of stage {stage}");
                    assert!(natives(sin).contains(&(kind, stage)));
                }
            }
        }
        assert_eq!((deepest_stage(1), deepest_stage(3), deepest_stage(9)), (2, 4, 7));
        let (hp1, dmg1) = weight(0, 1);
        let (hp7, dmg7) = weight(0, 7);
        assert!(hp7 > hp1 && dmg7 > dmg1, "a realm's own foes grow with the tier");
    }
}
