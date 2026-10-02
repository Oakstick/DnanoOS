//! The Sparring Ring, played through the game (COMBAT.md §5): bouts begin and end, the hero's
//! string lands on its frames, the guard parries and blocks, the evade passes under a slam,
//! and under load the threat manager never lets two strikes land inside the reaction window.

use super::sparring::{BoutPhase, AFTERMATH, COUNTDOWN};
use super::*;
use crate::logic::combat::fsm::Reel;
use crate::logic::combat::threat::CAPACITY;
use crate::logic::combat::{Archetype, MoveId};
use crate::logic::gym;
use crate::logic::types::{Dir, V};
use crate::render::view::CameraMode;

fn gym() -> Game {
    std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
    let mut g = Game::new();
    g.map_jump(save::Stage::Gym);
    g.enter(Screen::Playing);
    g
}

/// Runs the game until `n` more of the ring's 60 Hz frames have passed (hitstop included).
fn frames(g: &mut Game, n: u16) {
    let n = n as u32;
    let goal = g.brawl.as_ref().unwrap().threat.frame + n;
    for _ in 0..(n as usize * 2 + 600) {
        if g.brawl.as_ref().unwrap().threat.frame >= goal {
            return;
        }
        g.update(crate::app::STEP);
    }
    panic!("the ring's clock stalled");
}

/// Runs the game for `secs` seconds of steps.
fn run(g: &mut Game, secs: f64) {
    for _ in 0..(secs / crate::app::STEP) as usize {
        g.update(crate::app::STEP);
    }
}

fn duel(g: &Game, k: usize) -> &crate::logic::combat::Duelist {
    g.enemies.iter().filter_map(|e| e.duel.as_deref()).nth(k).expect("a duelist")
}

fn duel_mut(g: &mut Game, k: usize) -> &mut crate::logic::combat::Duelist {
    g.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).nth(k).expect("a duelist")
}

fn phase(g: &Game) -> BoutPhase {
    g.spar.as_ref().unwrap().phase
}

/// A bout's countdown (its foes standing still) with only its first foe left, `gap` tiles in front
/// of the hero (facing up), facing him, and asking for no token.
fn face_off(n: usize, gap: f64) -> Game {
    let mut g = gym();
    g.debug_ring(n);
    let keep = duel(&g, 0).id;
    g.enemies.retain(|e| e.duel.as_ref().is_none_or(|d| d.id == keep));
    g.view.clear_enemies();
    for e in &g.enemies {
        g.view.add_enemy(e);
    }
    g.hero.face(Dir::Up);
    let hp = g.hero.pos();
    let d = duel_mut(&mut g, 0);
    d.pos = V::new(hp.x, hp.y - gap);
    d.facing = V::new(0.0, 1.0);
    d.rest = u16::MAX;
    g
}

/// The fight is on, and foe 0 begins `mv` at once.
fn strike(g: &mut Game, mv: MoveId) {
    if let Some(s) = g.spar.as_mut() {
        s.phase = BoutPhase::Fight;
    }
    if let Some(b) = g.brawl.as_mut() {
        b.live = true;
    }
    let d = duel_mut(g, 0);
    d.token = true;
    assert!(d.begin(mv));
}

#[test]
fn a_bout_begins_when_he_steps_into_the_ring_and_is_called_off_when_he_leaves() {
    let mut g = gym();
    assert_eq!(phase(&g), BoutPhase::Idle);
    assert!(!g.in_ring(), "he arrives on the first platform");
    let camera = g.view.camera_mode;
    g.hero.place(gym::ring_start());
    g.update(crate::app::STEP);
    assert!(matches!(phase(&g), BoutPhase::Countdown(_)));
    assert_eq!(g.view.camera_mode, CameraMode::Top, "the ring is fought from above");
    let kinds: Vec<Archetype> = g.enemies.iter().filter_map(|e| e.duel.as_deref()).map(|d| d.arch).collect();
    assert_eq!(kinds, gym::BOUTS[0].foes, "bout 1: the Brute");
    run(&mut g, COUNTDOWN + 0.1);
    assert_eq!(phase(&g), BoutPhase::Fight);
    g.hero.place(gym::arrival());
    g.update(crate::app::STEP);
    assert_eq!(phase(&g), BoutPhase::Idle);
    assert!(g.enemies.iter().all(|e| e.duel.is_none()), "its foes leave with him");
    assert_eq!(g.view.camera_mode, camera, "and the camera comes back");
}

#[test]
fn the_heros_string_lands_on_its_active_frames_chains_and_the_finisher_breaks_through_armor() {
    let mut g = face_off(1, 1.4);
    let full = duel(&g, 0).hp;
    g.input.inject(Action::Attack);
    frames(&mut g, 1);
    assert_eq!(g.brawl.as_ref().unwrap().hero.attack().map(|a| a.0), Some(MoveId::L1));
    frames(&mut g, MoveId::L1.data().startup - 1);
    assert_eq!(duel(&g, 0).hp, full, "nothing lands in the startup");
    frames(&mut g, 1);
    assert_eq!(duel(&g, 0).hp, full - 10.0, "the first active frame lands L1");
    assert_eq!(duel(&g, 0).f.reeling(), None, "the Brute's armor holds against a light blow");
    // Mashed: the string comes out on its cancel frames.
    for _ in 0..40 {
        g.input.inject(Action::Attack);
        frames(&mut g, 1);
    }
    let s = g.brawl.as_ref().unwrap();
    assert_eq!(s.stats.hits, 3, "L1, L2, L3");
    assert_eq!(duel(&g, 0).hp, full - 10.0 - 10.0 - 13.0, "11 x 0.92 and 16 x 0.84 of the combo");
    assert!(duel(&g, 0).f.poise < 100.0 - 80.0, "85 poise gone");
    assert!(g.hero.stamina < 100.0 - 47.0, "three swings' stamina");
}

#[test]
fn a_raised_guard_blocks_for_stamina_and_a_late_raise_parries_the_lunge() {
    // Raised long before the lunge: a block.
    let mut g = face_off(2, 2.4);
    g.input.inject(Action::Block);
    frames(&mut g, 20);
    assert!(g.hero.blocking);
    strike(&mut g, MoveId::Lunge);
    frames(&mut g, 30);
    let s = g.brawl.as_ref().unwrap();
    assert_eq!((s.stats.blocks, s.stats.parries), (1, 0), "{:?}", s.stats);
    assert_eq!(g.hero.hp, g.hero.max_hp, "the guard takes it all");
    assert!(g.hero.stamina < 90.0, "for stamina");
    // Raised two frames before its active frames: the bite lands inside the parry window.
    let mut g = face_off(2, 2.4);
    strike(&mut g, MoveId::Lunge);
    frames(&mut g, MoveId::Lunge.data().startup - 2);
    g.input.inject(Action::Block);
    frames(&mut g, 14);
    let s = g.brawl.as_ref().unwrap();
    assert_eq!(s.stats.parries, 1, "parried: {:?}", s.stats);
    assert_eq!(g.hero.hp, g.hero.max_hp);
    assert!(matches!(duel(&g, 0).f.reeling(), Some(Reel::Deflected | Reel::Staggered)), "the Skirmisher reels");
}

#[test]
fn the_evade_passes_under_a_slam_and_an_open_hero_pays_for_it() {
    let mut g = face_off(1, 1.4);
    strike(&mut g, MoveId::Slam);
    let slam = MoveId::Slam.data();
    frames(&mut g, slam.startup - 6);
    g.hero.face(Dir::Down);
    g.input.inject(Action::Jump);
    frames(&mut g, 14);
    let s = g.brawl.as_ref().unwrap();
    assert_eq!(s.stats.evades, 1, "{:?}", s.stats);
    assert_eq!(g.hero.hp, g.hero.max_hp, "untouched in the air");
    // Standing there, it lands.
    let mut g = face_off(1, 1.4);
    strike(&mut g, MoveId::Slam);
    frames(&mut g, slam.startup + 2);
    assert_eq!(g.hero.hp, g.hero.max_hp - 30);
    assert_eq!(g.brawl.as_ref().unwrap().hero.reeling(), Some(Reel::Hitstun));
}

#[test]
fn losing_puts_him_back_on_his_feet_and_winning_brings_the_next_bout() {
    let mut g = face_off(1, 1.4);
    g.hero.hp = 10;
    strike(&mut g, MoveId::Slam);
    frames(&mut g, MoveId::Slam.data().startup + 2);
    assert!(matches!(phase(&g), BoutPhase::Lost(_)));
    assert!(g.hero.alive, "never slain in the ring");
    run(&mut g, AFTERMATH + 0.1);
    assert_eq!(g.hero.hp, g.hero.max_hp);
    assert!(matches!(phase(&g), BoutPhase::Countdown(_)));
    assert_eq!(g.spar.as_ref().unwrap().bout, 0, "the same bout again");
    // Won: on to the next.
    let mut g = face_off(1, 1.4);
    duel_mut(&mut g, 0).hp = 5.0;
    if let Some(s) = g.spar.as_mut() {
        s.phase = BoutPhase::Fight;
    }
    if let Some(b) = g.brawl.as_mut() {
        b.live = true;
    }
    g.input.inject(Action::Attack);
    frames(&mut g, 10);
    assert!(matches!(phase(&g), BoutPhase::Won(_)), "{:?}", phase(&g));
    run(&mut g, AFTERMATH + 0.1);
    assert_eq!(g.spar.as_ref().unwrap().bout, 1);
    let kinds: Vec<Archetype> = g.enemies.iter().filter_map(|e| e.duel.as_deref()).map(|d| d.arch).collect();
    assert_eq!(kinds, gym::BOUTS[1].foes, "the pack");
}

#[test]
fn the_gauntlet_never_lands_two_strikes_inside_the_reaction_window() {
    let mut g = gym();
    g.debug_ring(5);
    run(&mut g, COUNTDOWN + 0.1);
    let mut strikes = 0;
    let mut actives_before = [false; 5];
    for _ in 0..(60.0 / crate::app::STEP) as usize {
        g.hero.hp = g.hero.max_hp; // he stands there and takes it, for a minute
        g.update(crate::app::STEP);
        let s = g.brawl.as_ref().unwrap();
        assert!(s.threat.used() <= CAPACITY);
        for k in 0..5 {
            let on = duel(&g, k).f.active().is_some();
            if on && !actives_before[k] {
                strikes += 1;
            }
            actives_before[k] = on;
        }
    }
    let s = g.brawl.as_ref().unwrap();
    assert_eq!(phase(&g), BoutPhase::Fight);
    assert!(strikes >= 20, "they kept coming: {strikes} strikes in a minute");
    assert_eq!(s.stats.overlaps, 0, "{:?}", s.stats);
    // The waiting stand round him, not stacked on one another.
    for a in 0..5 {
        for b in a + 1..5 {
            let (p, q) = (duel(&g, a).pos, duel(&g, b).pos);
            assert!(((p.x - q.x).powi(2) + (p.y - q.y).powi(2)).sqrt() > 0.5, "{a} and {b} on top of each other");
        }
    }
}

/// The balance targets of COMBAT.md phase 2, played by the bots over three seeds: the expert wins
/// every bout inside its target time, a novice cannot beat the Brute or the Gauntlet, an adept
/// beats the Brute, and no strikes overlap for a player who stands his ground.
#[test]
fn the_bouts_meet_their_balance_targets() {
    use super::bot::{summarize, Skill};
    std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
    let seeds = [1, 2, 3];
    for (k, b) in gym::BOUTS.iter().enumerate() {
        let s = summarize(k, Skill::Expert, &seeds);
        assert_eq!(s.wins, s.runs, "the expert wins {}: {s:?}", b.name);
        assert!(s.ttk >= b.target.0 && s.ttk <= b.target.1, "{} in {:.1} s, not {:?}: {s:?}", b.name, s.ttk, b.target);
        assert_eq!(s.overlaps, 0, "{}: {s:?}", b.name);
    }
    for k in [0, 4] {
        let s = summarize(k, Skill::Novice, &seeds);
        assert_eq!(s.wins, 0, "a masher cannot beat {}: {s:?}", gym::BOUTS[k].name);
    }
    let s = summarize(0, Skill::Adept, &seeds);
    assert_eq!(s.wins, s.runs, "a player who blocks beats the Brute: {s:?}");
}

/// Phase 3 of COMBAT.md, played by the expert over three seeds: every hero's full kit at tier 10
/// wins at least twice as fast as at tier 1, and at both ends the three heroes kill within 1.35x
/// of each other.
#[test]
fn every_heros_kit_grows_and_the_heroes_stay_level() {
    use super::bot::{summarize_as, Loadout, Skill};
    use crate::logic::heroes::ROSTER;
    std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
    let seeds = [1, 2, 3];
    for bout in [0, 3] {
        let mut ends = [[0.0; 3]; 2];
        for (h, hero) in ROSTER.into_iter().enumerate() {
            for (e, tier) in [1, 10].into_iter().enumerate() {
                let s = summarize_as(bout, Skill::Expert, &seeds, Some(Loadout::all(hero, tier)));
                assert_eq!(s.wins, s.runs, "{hero:?} tier {tier} wins {}: {s:?}", gym::BOUTS[bout].name);
                ends[e][h] = s.ttk;
            }
            assert!(
                ends[1][h] * 2.0 <= ends[0][h],
                "{hero:?} on {}: {:.1} s at tier 10 against {:.1} s",
                gym::BOUTS[bout].name,
                ends[1][h],
                ends[0][h]
            );
        }
        for (e, times) in ends.iter().enumerate() {
            let (lo, hi) = times.iter().fold((f64::MAX, 0.0f64), |(a, b), &t| (a.min(t), b.max(t)));
            assert!(
                hi <= lo * 1.35,
                "{} at {}: the heroes take {times:?}",
                gym::BOUTS[bout].name,
                if e == 0 { "tier 1" } else { "tier 10" }
            );
        }
    }
}

/// Phase 4 of COMBAT.md: crawling the Vaults of Mammon, the expert with a tier-4 kit clears every
/// pack without losing all its lives, the adept with a tier-7 kit too, and no strikes overlap.
#[test]
fn the_vaults_are_a_brawl_a_skilled_player_can_clear_with_a_readable_cadence() {
    use super::bot::{crawl, Loadout, Skill};
    use crate::logic::heroes::HeroKind;
    std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
    for (skill, tier) in [(Skill::Expert, 4), (Skill::Adept, 7)] {
        let c = crawl(skill, 1, Some(Loadout::all(HeroKind::Ragnar, tier)), 900.0);
        assert!(c.cleared && !c.game_over, "{skill:?} at tier {tier}: {c:?}");
        assert!(c.overlaps_per_minute() < 0.5, "{skill:?} at tier {tier}: {c:?}");
        assert!(c.peak <= 8, "a pack at a time: {c:?}");
    }
}
