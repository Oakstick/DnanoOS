//! The gym: a training ground with one of every foe, reached from the map.
//! A cut-down Portals of Hell: one platform for each stage of foes in a row
//! over the lava, joined by wooden bridges, the hero arriving on the first.
//! Stage 1's foes stand on the first platform, stage 2's on the second, and
//! so on as stages are added. Slain foes pull themselves together again at
//! their posts, so the gym never empties.
//!
//! West of the arrival platform lies the Sparring Ring, the combat lab of
//! COMBAT.md: step in and a bout begins, first of the three archetypes, then
//! of the later stages' foes, two bouts and more a stage.

use super::enemy::EnemyKind;
use super::hub::Rect;
use super::level::{LevelData, Theme};
use super::levels::SPEED_SCALE;
use super::types::{P, V};

/// How many stages of foes the gym holds (one platform each).
pub const STAGES: usize = 7;
pub const PLATFORM: i32 = 13;
pub const GAP: i32 = 7;
pub const MARGIN: i32 = 6;
/// The Sparring Ring's side.
pub const RING: i32 = 17;

/// The grid's width and height.
pub fn size() -> (i32, i32) {
    (MARGIN * 2 + RING + GAP + STAGES as i32 * PLATFORM + (STAGES as i32 - 1) * GAP, MARGIN * 2 + RING)
}

/// The Sparring Ring, west of the stages.
pub fn ring() -> Rect {
    Rect { x0: MARGIN, y0: MARGIN, x1: MARGIN + RING - 1, y1: MARGIN + RING - 1 }
}

/// The ring's floor as the duelists know it: from edge to edge of its tiles.
pub fn ring_bounds() -> (V, V) {
    let r = ring();
    (V::new(r.x0 as f64 - 0.45, r.y0 as f64 - 0.45), V::new(r.x1 as f64 + 0.45, r.y1 as f64 + 0.45))
}

/// The platform of stage `k + 1`, beside the ring's middle.
pub fn platform(k: usize) -> Rect {
    let x0 = MARGIN + RING + GAP + k as i32 * (PLATFORM + GAP);
    let y0 = MARGIN + (RING - PLATFORM) / 2;
    Rect { x0, y0, x1: x0 + PLATFORM - 1, y1: y0 + PLATFORM - 1 }
}

/// The bridges: from the ring to the first platform, then between platforms, three tiles wide across their middles.
pub fn bridges() -> Vec<Rect> {
    let cy = MARGIN + RING / 2;
    let mut out = vec![Rect { x0: ring().x1 + 1, y0: cy - 1, x1: platform(0).x0 - 1, y1: cy + 1 }];
    out.extend((1..STAGES).map(|k| {
        let (a, b) = (platform(k - 1), platform(k));
        Rect { x0: a.x1 + 1, y0: cy - 1, x1: b.x0 - 1, y1: cy + 1 }
    }));
    out
}

/// Where the hero arrives: the west end of the first platform, facing the foes (the ring behind him).
pub fn arrival() -> P {
    let r = platform(0);
    P::new(r.x0 + 1, (r.y0 + r.y1) / 2)
}

/// Where stage `k + 1`'s foes stand: two rows across the east half of their platform; a stage
/// with giants in it stands them in a wide square instead, room to turn round in.
pub fn posts(k: usize) -> Vec<P> {
    let r = platform(k);
    let cy = (r.y0 + r.y1) / 2;
    let giants = EnemyKind::of_stage(k as u8 + 1).iter().any(|&kind| crate::logic::combat::Breed::of(kind).radius > 0.7);
    let mut out = Vec::new();
    if giants {
        for dy in [-3, 3] {
            for x in [r.x0 + 4, r.x1 - 3] {
                out.push(P::new(x, cy + dy));
            }
        }
        return out;
    }
    for dy in [-2, 2] {
        for x in (r.x0 + 5..=r.x1 - 1).step_by(2) {
            out.push(P::new(x, cy + dy));
        }
    }
    out
}

/// A bout of the Sparring Ring: its name, its foes (their bodies, each fighting as its breed),
/// the seconds a skilled player should take to win it (COMBAT.md §2.1, give or take a fifth),
/// and the tier of kit those seconds are measured with (1: the training sword; the later stages
/// want a hero who has bought his way up the trees).
#[derive(Clone, Copy, Debug)]
pub struct Bout {
    pub name: &'static str,
    pub foes: &'static [EnemyKind],
    pub target: (f64, f64),
    pub tier: u8,
}

impl Bout {
    /// The stage it draws its foes from (2 for the lab's: the archetypes' own bodies).
    pub fn stage(&self) -> u8 {
        self.foes.iter().map(|k| k.stage()).max().unwrap_or(2)
    }
}

use super::enemy::EnemyKind as K;

/// The bouts, in order: the lab's five, the archetypes in their own bodies (COMBAT.md §5); the
/// third stage's three, its blades, its jaws and its fire (§10); then two for each stage
/// after it, a pair and the whole cast (§11).
pub const BOUTS: [Bout; 16] = [
    Bout { name: "THE BRUTE", foes: &[K::Blemmyes], target: (8.0, 14.0), tier: 1 },
    Bout { name: "THE PACK", foes: &[K::Werewolf, K::Werewolf], target: (5.0, 10.0), tier: 1 },
    Bout { name: "THE HEXER'S GUARD", foes: &[K::Witch, K::Werewolf], target: (5.0, 10.0), tier: 1 },
    Bout { name: "THE TRIO", foes: &[K::Blemmyes, K::Werewolf, K::Witch], target: (13.0, 24.0), tier: 1 },
    Bout { name: "THE GAUNTLET", foes: &[K::Blemmyes, K::Werewolf, K::Werewolf, K::Witch, K::Witch], target: (20.0, 32.0), tier: 1 },
    Bout { name: "STEEL", foes: &[K::Tengu, K::Rasetsu], target: (20.0, 32.0), tier: 1 },
    Bout { name: "FANG AND FIRE", foes: &[K::DireWolf, K::Cobra, K::Jorogumo], target: (10.0, 16.0), tier: 1 },
    Bout { name: "THE NIGHT PARADE", foes: &[K::Tengu, K::Rasetsu, K::DireWolf, K::Cobra, K::Jorogumo], target: (29.0, 45.0), tier: 1 },
    Bout { name: "THE IRON CLUB", foes: &[K::Oni, K::Kappa], target: (14.0, 30.0), tier: 3 },
    Bout { name: "THE FOX'S WEDDING", foes: &[K::Oni, K::Kappa, K::Rokurokubi, K::Kitsune], target: (25.0, 50.0), tier: 3 },
    Bout { name: "THE CALL IN THE COLD", foes: &[K::Karakoncolos, K::Tulukus], target: (14.0, 30.0), tier: 5 },
    Bout { name: "ZEMHERİ NIGHTS", foes: &[K::Karakoncolos, K::Tulukus, K::Albasti, K::Cin, K::Arcura], target: (30.0, 60.0), tier: 5 },
    Bout { name: "THE ONE EYE", foes: &[K::Tepegoz], target: (10.0, 24.0), tier: 7 },
    Bout { name: "THE OLD TALES", foes: &[K::Tepegoz, K::Gashadokuro, K::Ejder, K::Goryo], target: (30.0, 60.0), tier: 7 },
    Bout { name: "FIRE AND ICE", foes: &[K::FrostGiant, K::Surtr], target: (20.0, 40.0), tier: 9 },
    Bout { name: "RAGNARÖK", foes: &[K::FrostGiant, K::Surtr, K::Nidhogg, K::Hel], target: (30.0, 60.0), tier: 9 },
];

/// The first of the third stage's bouts.
pub const STAGE_THREE_BOUT: usize = 5;

/// The bouts of `stage` (3 and on), in order.
pub fn bouts_of(stage: u8) -> std::ops::Range<usize> {
    let first = BOUTS.iter().position(|b| b.stage() == stage).unwrap_or(BOUTS.len());
    let last = BOUTS.iter().rposition(|b| b.stage() == stage).map(|k| k + 1).unwrap_or(first);
    first..last
}

/// The first bout of the stage after bout `k`'s (the lab's first after the last stage's).
pub fn next_stage_bout(k: usize) -> usize {
    let stage = BOUTS[k % BOUTS.len()].stage();
    BOUTS.iter().position(|b| b.stage() > stage).unwrap_or(0)
}

/// Where a bout's foes enter: in a file west of the ring's middle, facing east, six tiles from the hero.
pub fn bout_spawns(n: usize) -> Vec<V> {
    let r = ring();
    let (cx, cy) = ((r.x0 + r.x1) as f64 / 2.0, (r.y0 + r.y1) as f64 / 2.0);
    (0..n).map(|i| V::new(cx - 3.0 - (i % 2) as f64 * 1.5, cy + (i as f64 - (n as f64 - 1.0) / 2.0) * 2.4)).collect()
}

/// Where the hero stands for a bout: east of the ring's middle.
pub fn ring_start() -> P {
    let r = ring();
    P::new((r.x0 + r.x1) / 2 + 3, (r.y0 + r.y1) / 2)
}

/// The foes of the gym, stage by stage, with their posts.
pub fn garrison() -> Vec<(EnemyKind, u8, P)> {
    (0..STAGES)
        .flat_map(|k| {
            let stage = k as u8 + 1;
            EnemyKind::of_stage(stage).iter().zip(posts(k)).map(move |(&kind, post)| (kind, stage, post))
        })
        .collect()
}

/// The gym as a level: platforms of ' ' in a sea of '~', '=' for the bridges.
pub fn gym() -> LevelData {
    let (w, h) = size();
    let mut rows = vec![vec!['~'; w as usize]; h as usize];
    let mut put = |p: P, c: char| rows[p.y as usize][p.x as usize] = c;
    for k in 0..STAGES {
        for p in platform(k).tiles() {
            put(p, ' ');
        }
    }
    for p in ring().tiles() {
        put(p, ' ');
    }
    for b in bridges() {
        for p in b.tiles() {
            put(p, '=');
        }
    }
    put(arrival(), 'P');
    // Every level names a foe spawn: the far end of the last platform (the gym's foes stand at their posts instead).
    let last = platform(STAGES - 1);
    put(P::new(last.x1, (last.y0 + last.y1) / 2), 'G');
    LevelData {
        name: "The Gym".to_string(),
        rows: rows.iter().map(|r| r.iter().collect()).collect(),
        heights: vec!["0".repeat(w as usize); h as usize],
        hero_speed: 6.5 * SPEED_SCALE,
        enemy_speed: 5.4 * SPEED_SCALE,
        frenzy_seconds: 7.0,
        boss: false,
        theme: Theme::Gym,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::parse_level;
    use crate::logic::pathing::reachable;

    #[test]
    fn every_post_stands_on_its_platform_and_the_bridges_join_them() {
        let level = parse_level(&gym());
        let seen = reachable(&level.grid, arrival(), false, false);
        for (k, (kind, stage, post)) in garrison().into_iter().enumerate() {
            assert!(platform(stage as usize - 1).contains(post), "{kind:?} ({k}) stands on stage {stage}'s platform");
            assert!(level.grid.is_walkable(post.x, post.y, false));
            assert!(seen.contains(&post), "and he can walk to it");
        }
        let all: usize = (1..=STAGES as u8).map(|s| EnemyKind::of_stage(s).len()).sum();
        assert_eq!(garrison().len(), all, "every stage's whole cast at its posts");
        for k in 0..STAGES {
            assert!(posts(k).len() >= EnemyKind::of_stage(k as u8 + 1).len(), "room on stage {}'s platform", k + 1);
        }
    }

    #[test]
    fn the_labs_bouts_are_the_archetypes_own_bodies_and_every_later_stage_brings_all_its_foes() {
        use crate::logic::combat::{Archetype, Attack, Breed};
        for b in &BOUTS[..STAGE_THREE_BOUT] {
            assert!(b.foes.iter().all(|&k| Archetype::ALL.iter().any(|a| a.kind() == k)), "{}", b.name);
            assert_eq!(b.tier, 1);
        }
        assert_eq!(bouts_of(3).start, STAGE_THREE_BOUT);
        let mut tier = 1;
        for stage in 3..=EnemyKind::STAGES {
            let bouts = bouts_of(stage);
            assert!(bouts.len() >= 2, "stage {stage} has its bouts");
            let foes: Vec<EnemyKind> = BOUTS[bouts.clone()].iter().flat_map(|b| b.foes.iter().copied()).collect();
            for &k in EnemyKind::of_stage(stage) {
                assert!(foes.contains(&k), "{k:?} has a bout");
            }
            assert!(foes.iter().all(|k| k.stage() == stage), "stage {stage}'s bouts are its own foes'");
            // Each stage's last bout brings every way of attacking at once; each wants as good a kit as the last.
            let last = BOUTS[bouts.end - 1];
            for way in [Attack::Melee, Attack::Body, Attack::Distance] {
                assert!(last.foes.iter().any(|&k| Breed::of(k).attack() == way), "{}: no {way:?}", last.name);
            }
            for b in &BOUTS[bouts] {
                assert!(b.tier >= tier, "{}", b.name);
                tier = b.tier;
            }
        }
        assert_eq!(next_stage_bout(0), STAGE_THREE_BOUT);
        assert_eq!(next_stage_bout(BOUTS.len() - 1), 0, "round to the lab again");
    }

    #[test]
    fn the_ring_lies_over_the_first_bridge_and_holds_every_bouts_foes() {
        let level = parse_level(&gym());
        let seen = reachable(&level.grid, arrival(), false, false);
        assert!(seen.contains(&ring_start()), "he can walk into the ring");
        let (lo, hi) = ring_bounds();
        for b in BOUTS {
            for p in bout_spawns(b.foes.len()) {
                assert!(p.x > lo.x + 1.0 && p.x < hi.x - 1.0 && p.y > lo.y + 1.0 && p.y < hi.y - 1.0, "{}: {p:?}", b.name);
            }
        }
        assert!(!ring().contains(arrival()));
    }
}
