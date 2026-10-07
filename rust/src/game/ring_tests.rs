//! The Sparring Ring, played through the game (COMBAT.md §5): bouts begin and end, the hero's
//! string lands on its frames, the guard parries and blocks, the evade passes under a slam,
//! and under load the threat manager never lets two strikes land inside the reaction window.

use super::sparring::{BoutPhase, AFTERMATH, COUNTDOWN};
use super::*;
use crate::logic::combat::duelist::{PLEA_FRAMES, SPILL_FRAMES, WARD_SHARE};
use crate::logic::combat::fsm::Reel;
use crate::logic::combat::threat::CAPACITY;
use crate::logic::combat::{Breed, Duelist, MoveId};
use crate::logic::enemy::EnemyKind;
use crate::logic::gym;
use crate::logic::sins::Sin;
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
    let hp = g.hero.pos();
    let d = duel_mut(g, 0);
    d.token = true;
    assert!(d.begin(mv, hp));
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
    let kinds: Vec<EnemyKind> = g.enemies.iter().filter_map(|e| e.duel.as_deref()).map(|d| d.breed.kind).collect();
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
fn a_raised_guard_blocks_for_stamina_and_a_late_raise_parries_the_bite() {
    // Raised long before the bite: a block.
    let mut g = face_off(2, 1.2);
    g.input.inject(Action::Block);
    frames(&mut g, 20);
    assert!(g.hero.blocking);
    strike(&mut g, MoveId::Bite);
    frames(&mut g, 30);
    let s = g.brawl.as_ref().unwrap();
    assert_eq!((s.stats.blocks, s.stats.parries), (1, 0), "{:?}", s.stats);
    assert_eq!(g.hero.hp, g.hero.max_hp, "the guard takes it all");
    assert!(g.hero.stamina < 90.0, "for stamina");
    // Raised four frames before its active frames: the bite lands inside the parry window.
    let mut g = face_off(2, 1.2);
    strike(&mut g, MoveId::Bite);
    frames(&mut g, MoveId::Bite.data().startup - 4);
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

/// Walks him to the right for `n` frames (the stick held over); how far he went.
fn walk(g: &mut Game, n: u16, guard: bool) -> f64 {
    let from = g.hero.pos();
    let goal = g.brawl.as_ref().unwrap().threat.frame + n as u32;
    while g.brawl.as_ref().unwrap().threat.frame < goal {
        let mut pad = crate::input::PadState { left: (1.0, 0.0), ..Default::default() };
        if guard {
            pad.actions.insert(Action::Block);
        }
        g.input.set_pad(Some(pad));
        g.update(crate::app::STEP);
    }
    g.input.set_pad(None);
    crate::logic::combat::math::len(crate::logic::combat::math::sub(g.hero.pos(), from))
}

#[test]
fn no_foes_blow_slows_him_or_roots_him_struck_blocking_or_with_his_guard_broken() {
    // Free, as a yardstick: how far he walks in 16 frames.
    let mut g = face_off(1, 1.4);
    g.hero.free_move = true;
    let free = walk(&mut g, 16, false);
    assert!(free > 0.8, "{free}");
    // Struck open by the slam: in his hitstun he walks at his whole pace.
    let mut g = face_off(1, 1.4);
    g.hero.free_move = true;
    strike(&mut g, MoveId::Slam);
    frames(&mut g, MoveId::Slam.data().startup + 1);
    assert_eq!(g.brawl.as_ref().unwrap().hero.reeling(), Some(Reel::Hitstun));
    assert_eq!(g.hero.root, 1.0, "not rooted");
    let gone = walk(&mut g, 16, false);
    assert!(gone > free * 0.9, "struck, he walked {gone} of {free}");
    // Blocked: in his blockstun he walks as he does with his shield up, no slower.
    let mut g = face_off(1, 1.4);
    g.hero.free_move = true;
    let shielded = walk(&mut g, 16, true);
    let mut g = face_off(1, 1.4);
    g.hero.free_move = true;
    g.input.inject(Action::Block);
    frames(&mut g, 20);
    strike(&mut g, MoveId::Slam);
    while g.brawl.as_ref().unwrap().hero.reeling().is_none() {
        let pad = crate::input::PadState { actions: [Action::Block].into_iter().collect(), ..Default::default() };
        g.input.set_pad(Some(pad));
        frames(&mut g, 1);
    }
    assert_eq!(g.brawl.as_ref().unwrap().hero.reeling(), Some(Reel::Blockstun));
    assert_eq!(g.hero.root, 1.0, "not rooted");
    let gone = walk(&mut g, 16, true);
    assert!(gone > shielded * 0.9, "blocking, he walked {gone} of {shielded}");
    // His guard broken ("BREAK!"): no stamina to hold it, and still he walks at his whole pace.
    let mut g = face_off(1, 1.4);
    g.hero.free_move = true;
    g.input.inject(Action::Block);
    frames(&mut g, 20);
    strike(&mut g, MoveId::Slam);
    while g.brawl.as_ref().unwrap().hero.reeling().is_none() {
        g.hero.stamina = 0.0;
        let pad = crate::input::PadState { actions: [Action::Block].into_iter().collect(), ..Default::default() };
        g.input.set_pad(Some(pad));
        frames(&mut g, 1);
    }
    assert_eq!(g.brawl.as_ref().unwrap().hero.reeling(), Some(Reel::GuardBroken));
    assert_eq!(g.brawl.as_ref().unwrap().stats.guard_breaks, 1);
    assert_eq!(g.hero.root, 1.0, "not rooted");
    let gone = walk(&mut g, 16, false);
    assert!(gone > free * 0.9, "his guard broken, he walked {gone} of {free}");
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
    let kinds: Vec<EnemyKind> = g.enemies.iter().filter_map(|e| e.duel.as_deref()).map(|d| d.breed.kind).collect();
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
/// every bout inside its target time (with the kit of the bout's tier), a novice cannot beat the
/// Brute or the Gauntlet, an adept beats the Brute, and no strikes overlap for a player who stands
/// his ground.
#[test]
fn the_bouts_meet_their_balance_targets() {
    use super::bot::{summarize, summarize_as, Loadout, Skill};
    use crate::logic::heroes::HeroKind;
    std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
    let seeds = [1, 2, 3];
    for (k, b) in gym::BOUTS.iter().enumerate() {
        let kit = (b.tier > 1).then(|| Loadout::all(HeroKind::Ragnar, b.tier));
        let s = summarize_as(k, Skill::Expert, &seeds, kit);
        assert_eq!(s.wins, s.runs, "the expert wins {}: {s:?}", b.name);
        assert!(s.ttk >= b.target.0 && s.ttk <= b.target.1, "{} in {:.1} s, not {:?}: {s:?}", b.name, s.ttk, b.target);
        assert_eq!(s.overlaps, 0, "{}: {s:?}", b.name);
    }
    // (Nor the prince of Greed, with the kit a hero has on reaching him.)
    for (k, tier) in [(0, 1), (4, 1), (gym::BOUTS.len() - 1, 2)] {
        let s = summarize_as(k, Skill::Novice, &seeds, (tier > 1).then(|| Loadout::all(HeroKind::Ragnar, tier)));
        assert_eq!(s.wins, 0, "a masher cannot beat {}: {s:?}", gym::BOUTS[k].name);
    }
    let s = summarize(0, Skill::Adept, &seeds);
    assert_eq!(s.wins, s.runs, "a player who blocks beats the Brute: {s:?}");
}

/// COMBAT.md §10: the third stage is stronger. With the training sword a masher beats none of its
/// bouts (only the expert wins them, above); with a tier-4 kit a careful player wins them all.
#[test]
fn the_third_stage_wants_skill_or_better_gear() {
    use super::bot::{summarize, summarize_as, Loadout, Skill};
    use crate::logic::heroes::HeroKind;
    std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
    let seeds = [1, 2, 3];
    for k in gym::bouts_of(3) {
        let name = gym::BOUTS[k].name;
        let s = summarize(k, Skill::Novice, &seeds);
        assert_eq!(s.wins, 0, "a masher cannot beat {name} with the training sword: {s:?}");
        let s = summarize_as(k, Skill::Adept, &seeds, Some(Loadout::all(HeroKind::Ragnar, 4)));
        assert_eq!(s.wins, s.runs, "a careful player with tier-4 gear wins {name}: {s:?}");
        assert_eq!(s.overlaps, 0, "{name}: {s:?}");
    }
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

/// COMBAT.md §11: every later stage wants its kit. At the tier its bouts are measured with, a
/// masher beats none of them; with three tiers more (or the last), a careful player wins them all,
/// and no strikes overlap.
#[test]
fn every_later_stage_wants_skill_and_its_kit() {
    use super::bot::{summarize_as, Loadout, Skill};
    use crate::logic::heroes::HeroKind;
    std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
    let seeds = [1, 2, 3];
    for stage in 4..=EnemyKind::STAGES {
        for k in gym::bouts_of(stage) {
            let b = gym::BOUTS[k];
            let s = summarize_as(k, Skill::Novice, &seeds, Some(Loadout::all(HeroKind::Ragnar, b.tier)));
            assert_eq!(s.wins, 0, "a masher with a tier-{} kit cannot beat {}: {s:?}", b.tier, b.name);
            let better = (b.tier + 3).min(crate::logic::progression::TIERS);
            let s = summarize_as(k, Skill::Adept, &seeds, Some(Loadout::all(HeroKind::Ragnar, better)));
            assert_eq!(s.wins, s.runs, "a careful player with a tier-{better} kit wins {}: {s:?}", b.name);
            assert_eq!(s.overlaps, 0, "{}: {s:?}", b.name);
        }
    }
}

/// Bout 1's countdown with its foe replaced by a fresh `kind`, `gap` tiles in front of the hero
/// (facing up), facing him, asking for no token; the fight on.
fn legend(kind: EnemyKind, gap: f64) -> Game {
    let mut g = face_off(1, gap);
    g.hero.free_move = true;
    let hp = g.hero.pos();
    let d = duel_mut(&mut g, 0);
    let id = d.id;
    *d = Duelist::bred(id, Breed::of(kind), V::new(hp.x, hp.y - gap), V::new(0.0, 1.0));
    d.rest = u16::MAX;
    let k = (0..g.enemies.len()).find(|&k| g.enemies[k].duel.is_some()).unwrap();
    g.enemies[k].b.kind = kind;
    g.view.clear_enemies();
    for e in &g.enemies {
        g.view.add_enemy(e);
    }
    if let Some(s) = g.spar.as_mut() {
        s.phase = BoutPhase::Fight;
    }
    if let Some(b) = g.brawl.as_mut() {
        b.live = true;
    }
    g
}

fn ails(g: &Game) -> crate::logic::combat::ails::Ails {
    g.brawl.as_ref().unwrap().ails
}

#[test]
fn bowed_to_the_kappa_spills_its_dish_and_the_oni_shakes_the_ground_round_him() {
    // A parried sumo shove: the kappa bows back, and its dish spills.
    let mut g = legend(EnemyKind::Kappa, 1.2);
    strike(&mut g, MoveId::Shove);
    frames(&mut g, MoveId::Shove.data().startup - 4);
    g.input.inject(Action::Block);
    frames(&mut g, 14);
    assert_eq!(g.brawl.as_ref().unwrap().stats.parries, 1);
    let d = duel(&g, 0);
    assert!(d.lore.spilled > SPILL_FRAMES - 30 && d.taken() > 1.0, "spilled: {:?}", d.lore);
    // Blocked, the shove still shoves him back.
    let mut g = legend(EnemyKind::Kappa, 1.2);
    g.input.inject(Action::Block);
    frames(&mut g, 20);
    let hp = g.hero.pos();
    let y0 = hp.y;
    let d = duel_mut(&mut g, 0);
    (d.pos, d.facing) = (V::new(hp.x, hp.y - 1.2), V::new(0.0, 1.0)); // (it strayed toward its post meanwhile)
    strike(&mut g, MoveId::Shove);
    frames(&mut g, 50);
    assert_eq!(g.brawl.as_ref().unwrap().stats.blocks, 1);
    assert!(g.hero.pos().y > y0 + 0.4, "shoved back: {} from {y0}", g.hero.pos().y);
    // The oni's club: behind him, out of its cone once its aim is set, the ground still throws him.
    let mut g = legend(EnemyKind::Oni, 1.4);
    let club = MoveId::Kanabo.data();
    strike(&mut g, MoveId::Kanabo);
    frames(&mut g, club.track + 1);
    let at = duel(&g, 0).pos;
    g.hero.place(P::new(at.x.round() as i32, (at.y - 1.6).round() as i32));
    frames(&mut g, club.startup - club.track + 1);
    let lost = g.hero.max_hp - g.hero.hp;
    assert!(lost > 0 && lost < 25, "a share of the blow reached him: {lost}");
}

/// After a blow he walks at his whole pace: nothing a legend does holds him.
fn free_after_the_blow(g: &mut Game, what: &str) {
    frames(g, 50);
    assert!(g.brawl.as_ref().unwrap().hero.reeling().is_none(), "{what}: still reeling");
    assert_eq!(g.hero.root, 1.0, "{what}: slowed");
}

#[test]
fn the_albastis_weight_and_the_gashadokuros_grab_land_hard_all_at_once_and_let_him_go() {
    for (kind, mv, gap) in [(EnemyKind::Albasti, MoveId::Smother, 1.2), (EnemyKind::Gashadokuro, MoveId::Grab, 1.4)] {
        let mut g = legend(kind, gap);
        strike(&mut g, mv);
        frames(&mut g, mv.data().startup + mv.data().active + 2);
        let lost = g.hero.max_hp - g.hero.hp;
        assert!(lost as f64 >= mv.data().damage * 0.6, "{kind:?}: the whole blow at once: {lost}");
        free_after_the_blow(&mut g, mv.data().name);
        assert_eq!(g.hero.max_hp - g.hero.hp, lost, "{kind:?}: nothing more bites him after");
    }
}

#[test]
fn the_dragon_begs_a_second_blow_brings_it_back_and_restraint_kills_it() {
    let mut g = legend(EnemyKind::Ejder, 1.4);
    duel_mut(&mut g, 0).hp = 4.0;
    g.input.inject(Action::Attack);
    frames(&mut g, 10);
    let d = duel(&g, 0);
    assert!(d.alive() && d.begging(), "felled, it begs");
    assert!(matches!(phase(&g), BoutPhase::Fight), "the bout is not won while it begs");
    // Struck again, it rises with a head grown back.
    frames(&mut g, 30);
    g.input.inject(Action::Attack);
    frames(&mut g, 10);
    let d = duel(&g, 0);
    assert!(!d.begging() && d.hp > 0.0, "it rose: {:?}", d.lore);
    // Felled again and left alone, it dies.
    let hp = g.hero.pos();
    let d = duel_mut(&mut g, 0);
    d.hp = 4.0;
    d.pos = V::new(hp.x, hp.y - 1.4);
    d.slot = None;
    frames(&mut g, 40);
    let d = duel_mut(&mut g, 0);
    d.pos = V::new(hp.x, hp.y - 1.4);
    g.input.inject(Action::Attack);
    frames(&mut g, 10);
    assert!(duel(&g, 0).begging());
    frames(&mut g, PLEA_FRAMES + 2);
    // (A slain duelist leaves the ring at once.)
    assert!(g.enemies.iter().all(|e| e.duel.is_none()), "struck once, it dies");
    assert!(matches!(phase(&g), BoutPhase::Won(_)));
}

#[test]
fn tepegoz_takes_a_share_of_a_blow_but_the_whole_of_it_on_his_eye() {
    let mut g = legend(EnemyKind::Tepegoz, 1.6);
    let full = duel(&g, 0).hp;
    g.input.inject(Action::Attack);
    frames(&mut g, MoveId::L1.data().startup + 1);
    let warded = full - duel(&g, 0).hp;
    assert!(warded > 0.0 && warded <= (10.0 * WARD_SHARE).ceil(), "warded: {warded}");
    // Bent over after his slam, his eye in reach.
    let mut g = legend(EnemyKind::Tepegoz, 1.6);
    g.hero.place(P::new(g.hero.occupied().x + 3, g.hero.occupied().y));
    strike(&mut g, MoveId::Slam);
    frames(&mut g, MoveId::Slam.data().startup + 8);
    let hp = g.hero.pos();
    let at = duel(&g, 0).pos;
    g.hero.face(if at.x < hp.x { crate::logic::types::Dir::Left } else { crate::logic::types::Dir::Right });
    let d = duel_mut(&mut g, 0);
    d.pos = V::new(hp.x + if at.x < hp.x { -1.2 } else { 1.2 }, hp.y);
    let before = d.hp;
    g.input.inject(Action::Attack);
    frames(&mut g, MoveId::L1.data().startup + 1);
    assert!(before - duel(&g, 0).hp >= 10.0, "the whole blow: {}", before - duel(&g, 0).hp);
}

#[test]
fn the_cins_fire_and_the_root_dart_only_hurt_and_the_frost_giants_cold_bites_on_without_slowing_him() {
    let mut g = legend(EnemyKind::Cin, 4.0);
    strike(&mut g, MoveId::Smokeless);
    frames(&mut g, 80);
    assert!(g.hero.hp < g.hero.max_hp);
    free_after_the_blow(&mut g, "smokeless fire");
    // The root dart flies at him as any bolt does, and bursts into roots where it strikes.
    let mut g = legend(EnemyKind::Arcura, 4.0);
    strike(&mut g, MoveId::Roots);
    frames(&mut g, MoveId::Roots.data().startup + 2);
    assert_eq!(g.hero.hp, g.hero.max_hp, "nothing from the floor: the dart is in flight");
    assert_eq!(g.brawl.as_ref().unwrap().bolts.len(), 1);
    frames(&mut g, 40);
    assert!(g.hero.hp < g.hero.max_hp, "the dart strikes him");
    assert!(g.brawl.as_ref().unwrap().flashes.iter().any(|f| f.kind == crate::logic::combat::ails::FlashKind::Roots));
    free_after_the_blow(&mut g, "root dart");
    // The goryō's thunderbolt flies too, and comes down as lightning on him.
    let mut g = legend(EnemyKind::Goryo, 4.0);
    strike(&mut g, MoveId::Thunder);
    frames(&mut g, MoveId::Thunder.data().startup + 40);
    assert!(g.hero.hp < g.hero.max_hp, "the bolt strikes him");
    assert!(g.brawl.as_ref().unwrap().flashes.iter().any(|f| f.kind == crate::logic::combat::ails::FlashKind::Lightning));
    // Frostbite: struck by the giant, the cold keeps biting while he walks at his whole pace.
    let mut g = legend(EnemyKind::FrostGiant, 1.6);
    strike(&mut g, MoveId::Slam);
    frames(&mut g, MoveId::Slam.data().startup + 2);
    assert!(ails(&g).frost > 0, "frostbitten");
    let after_blow = g.hero.hp;
    free_after_the_blow(&mut g, "frostbite");
    assert!(ails(&g).frost > 0 && g.hero.hp < after_blow, "the cold bites on: {} from {after_blow}", g.hero.hp);
}

#[test]
fn surtr_leaves_the_floor_burning_hel_raises_the_dead_and_they_fall_with_her() {
    let mut g = legend(EnemyKind::Surtr, 2.0);
    strike(&mut g, MoveId::Slam);
    frames(&mut g, MoveId::Slam.data().startup + 1);
    assert!(!g.brawl.as_ref().unwrap().hazards.is_empty(), "fire on the floor");
    let mut g = legend(EnemyKind::Hel, 5.0);
    strike(&mut g, MoveId::Raise);
    frames(&mut g, MoveId::Raise.data().startup + 2);
    let raised: Vec<u32> = g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.lore.master.is_some()).map(|d| d.id).collect();
    assert_eq!(raised.len(), 2, "two draugr climb out of the floor");
    duel_mut(&mut g, 0).hp = 1.0;
    let hel = duel(&g, 0).pos;
    g.hero.place(P::new(hel.x.round() as i32, (hel.y + 1.0).round() as i32));
    g.hero.face(crate::logic::types::Dir::Up);
    g.input.inject(Action::Attack);
    frames(&mut g, 12);
    // (A slain duelist leaves the ring at once.)
    assert!(g.enemies.iter().all(|e| e.duel.is_none()), "Hel falls, and her dead with her");
    assert!(matches!(phase(&g), BoutPhase::Won(_)));
}

#[test]
fn the_kitsune_conjures_illusions_a_blow_undoes_and_the_karakoncolos_roar_dooms_him() {
    let mut g = legend(EnemyKind::Kitsune, 1.4);
    let max = duel(&g, 0).max_hp;
    duel_mut(&mut g, 0).hp = max * 0.72;
    g.input.inject(Action::Attack);
    frames(&mut g, MoveId::L1.data().startup + 1);
    let illusions = g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.lore.illusion && d.alive()).count();
    assert_eq!(illusions, 2, "two illusions");
    let fake = g.enemies.iter().filter_map(|e| e.duel.as_deref()).find(|d| d.lore.illusion).map(|d| d.id).unwrap();
    let k = (0..g.enemies.len()).find(|&k| g.enemies[k].duel.as_ref().is_some_and(|d| d.id == fake)).unwrap();
    let hp = g.hero.pos();
    g.call_duel_hit(k, hp);
    assert!(!g.enemies[k].duel.as_ref().unwrap().alive(), "one blow and it is gone");
    // The karakoncolos roars his name: it dooms him, and he stays where he stands.
    let mut g = legend(EnemyKind::Karakoncolos, 5.0);
    let at = g.hero.pos();
    strike(&mut g, MoveId::Call);
    frames(&mut g, MoveId::Call.data().startup + 30);
    assert!(ails(&g).doom > 0, "doomed");
    assert!(
        crate::logic::combat::math::len(crate::logic::combat::math::sub(g.hero.pos(), at)) < 0.05,
        "not drawn in: {:?} from {at:?}",
        g.hero.pos()
    );
    assert_eq!(g.hero.root, 1.0);
}

/// COMBAT.md §13: Mammon in his own court, the Counting House, by the bots: with the kit a hero
/// has on reaching him (tiers 1 and 2) the expert brings him down through all three phases without
/// falling; a masher loses every life to him in his first.
#[test]
fn the_expert_beats_mammon_in_his_court_and_a_masher_does_not() {
    use super::bot::{court_fight, Loadout, Skill};
    use crate::logic::heroes::HeroKind;
    std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
    for (tier, seed) in [(1, 1), (2, 2)] {
        let kit = (tier > 1).then(|| Loadout::all(HeroKind::Ragnar, tier));
        let f = court_fight(Sin::Greed, Skill::Expert, seed, kit, 200.0);
        assert!(f.slain && f.deaths == 0 && f.phase == 3, "tier {tier}: {f:?}");
        assert_eq!(f.stats.overlaps, 0, "{f:?}");
    }
    let f = court_fight(Sin::Greed, Skill::Novice, 1, Some(Loadout::all(HeroKind::Ragnar, 2)), 200.0);
    assert!(!f.slain && f.game_over, "{f:?}");
}

/// The war log (WARBAND.md §3) hears of every strike once: an expert plays the Gauntlet, and what
/// the log counted (landings and how he met them, his blows, the kills) is what the bout counted.
#[test]
fn the_war_log_reports_every_strike_once() {
    use super::bot::{fresh_game, Bot, Skill};
    let mut g = fresh_game(3);
    g.debug_ring(5);
    g.hero.free_move = true;
    let mut bot = Bot::new(Skill::Expert, 3);
    let mut done = false;
    for _ in 0..(150.0 / crate::app::STEP) as usize {
        let pad = bot.act(&g);
        g.input.set_pad(Some(pad));
        g.update(crate::app::STEP);
        if matches!(phase(&g), BoutPhase::Won(_) | BoutPhase::Lost(_)) {
            done = true;
            break;
        }
    }
    assert!(done, "the bout ended");
    let (s, t) = (g.brawl.as_ref().unwrap().stats, g.war.tally);
    assert!(s.landings > 10 && s.hits > 10, "a real fight: {s:?}");
    assert_eq!(t.landings, s.landings, "{t:?} {s:?}");
    assert_eq!((t.parried, t.blocked, t.evaded, t.broken), (s.parries, s.blocks, s.evades, s.guard_breaks), "{t:?} {s:?}");
    assert_eq!(t.hero_hits, s.hits, "{t:?} {s:?}");
    assert_eq!(t.kills, s.kills, "{t:?} {s:?}");
    assert!(t.tells >= t.landings, "every landing was told first: {t:?}");
    if matches!(phase(&g), BoutPhase::Won(_)) {
        assert_eq!(t.packs_cleared, 1, "the last foe of the bout clears it: {t:?}");
    }
}

/// The packs of the vaults (WARBAND.md §7), played by the expert for four minutes: the leaders
/// make their plays, no play brings two strikes down inside the reaction window, and a routed
/// pack holds no token while it flees.
#[test]
fn the_packs_play_together_inside_the_reaction_window_and_a_rout_releases_the_pressure() {
    use super::bot::{fresh_game, Bot, Loadout, Skill};
    use crate::logic::war::pack::Play;
    let mut g = fresh_game(2);
    g.map_jump(save::Stage::Realm(crate::logic::sins::Sin::Greed));
    Loadout::all(crate::logic::heroes::HeroKind::Ragnar, 4).equip(&mut g);
    g.enter(Screen::Playing);
    let kit = g.hero_kit();
    g.brawl.as_mut().unwrap().reset(kit);
    g.hero.free_move = true;
    let mut bot = Bot::new(Skill::Expert, 2);
    let mut plays = std::collections::HashSet::new();
    for _ in 0..(240.0 / crate::app::STEP) as usize {
        if g.screen != Screen::Playing {
            g.enter(Screen::Playing);
        }
        g.hero.hp = g.hero.max_hp;
        let pad = bot.act(&g);
        g.input.set_pad(Some(pad));
        g.update(crate::app::STEP);
        for p in &g.war.packs {
            plays.insert(p.play);
            if p.rout > 0 {
                let held = p.members.iter().filter(|m| m.alive).any(|m| g.brawl.as_ref().unwrap().threat.holds(m.id));
                assert!(!held, "a fleeing pack holds no token");
            }
        }
    }
    assert!(!g.war.packs.is_empty(), "the dens are packs");
    assert!(plays.contains(&Play::Pincer) || plays.contains(&Play::Screen), "the leaders played: {plays:?}");
    let s = g.brawl.as_ref().unwrap().stats;
    assert!(s.kills > 10, "{s:?}");
    assert_eq!(s.overlaps, 0, "{s:?}");
}

/// The Ledger in the fight (WARBAND.md §8): the foe that brings him down in the vaults becomes a
/// rival and takes a share of his silver; on his next visit that rival leads a pack, his kind,
/// his name, dressed by his rank.
#[test]
fn the_foe_that_slays_him_becomes_a_rival_and_leads_a_pack_next_time() {
    use super::bot::{fresh_game, Bot, Skill};
    let mut g = fresh_game(5);
    g.map_jump(save::Stage::Realm(crate::logic::sins::Sin::Greed));
    g.enter(Screen::Playing);
    g.hero.free_move = true;
    g.progress.silver = 1000;
    let mut bot = Bot::new(Skill::Expert, 5);
    // Walk in until a pack hunts him, then stand there with a sliver of health.
    let mut fell = false;
    for _ in 0..(240.0 / crate::app::STEP) as usize {
        let hunting = g.enemies.iter().filter_map(|e| e.duel.as_deref()).any(|d| d.engaged());
        if hunting && g.hero.hp > 1 && !fell {
            g.hero.hp = 1;
        }
        let pad = if hunting { Default::default() } else { bot.act(&g) };
        g.input.set_pad(Some(pad));
        g.update(crate::app::STEP);
        if g.screen != Screen::Playing {
            fell = true;
            break;
        }
    }
    assert!(fell, "he fell");
    let rival = g.ledger().live().next().cloned().expect("his killer is a rival now");
    assert!(g.progress.silver < 1000 && rival.silver_held > 0, "it took its share: {} {}", g.progress.silver, rival.silver_held);
    // The next visit: he waits as a pack's leader.
    g.map_jump(save::Stage::Realm(crate::logic::sins::Sin::Greed));
    g.enter(Screen::Playing);
    for _ in 0..30 {
        g.update(crate::app::STEP);
    }
    let seated = g.war.grudges.seated.clone();
    assert_eq!(seated.len(), 1, "one rival a visit at tier 1");
    let (id, rid) = seated[0];
    assert_eq!(rid, rival.id);
    assert!(g.leads_a_pack(id), "he leads");
    let d = g.enemies.iter().filter_map(|e| e.duel.as_deref()).find(|d| d.id == id).unwrap();
    assert_eq!(d.breed.kind, rival.kind);
}

/// Crossfire (WARBAND.md §7.2): in a realm of the fourth tier two casters of a pack land their
/// bolts on one frame from two sides, as one strike; the reaction window holds round the pair.
#[test]
fn two_casters_cross_their_fire_as_one_strike() {
    use super::bot::{fresh_game, Bot, Loadout, Skill};
    let mut g = fresh_game(4);
    g.sins_cleared = 0b0111; // three keys held: the fourth tier
    g.map_jump(save::Stage::Realm(crate::logic::sins::Sin::Greed));
    // (A kit that lets the fights last: with the sixth tier's blade kept keen (W9) its casters fell
    // before two of them were ever free together.)
    Loadout::all(crate::logic::heroes::HeroKind::Ragnar, 3).equip(&mut g);
    g.enter(Screen::Playing);
    let kit = g.hero_kit();
    g.brawl.as_mut().unwrap().reset(kit);
    g.hero.free_move = true;
    let mut bot = Bot::new(Skill::Expert, 4);
    for _ in 0..(600.0 / crate::app::STEP) as usize {
        if g.screen != Screen::Playing {
            g.enter(Screen::Playing);
        }
        g.hero.hp = g.hero.max_hp;
        let pad = bot.act(&g);
        g.input.set_pad(Some(pad));
        g.update(crate::app::STEP);
    }
    assert!(g.war.tally.crossfires > 0, "they crossed their fire: {:?}", g.war.tally);
    let s = g.brawl.as_ref().unwrap().stats;
    assert_eq!(s.overlaps, 0, "{s:?}");
}

/// A rival who has learnt his finisher (WARBAND.md §8.3) sways back out of it as it begins:
/// untouchable through the sway's first frames, carried a step back, the finisher passing him by.
#[test]
fn a_rival_who_knows_his_finisher_sways_out_of_it() {
    use super::bot::fresh_game;
    use crate::logic::war::ledger::Lesson;
    use crate::logic::war::names::Deed;
    let mut g = fresh_game(9);
    g.map_jump(save::Stage::Realm(crate::logic::sins::Sin::Greed));
    g.enter(Screen::Playing);
    for _ in 0..10 {
        g.update(crate::app::STEP);
    }
    g.hero.face(Dir::Up);
    let hp = g.hero.pos();
    let (id, kind) = {
        let d = g.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).next().unwrap();
        // (Out of the string's first blows' reach, inside the sway's: a staggered rival sways from nothing.)
        d.pos = V::new(hp.x, hp.y - 2.4);
        d.facing = V::new(0.0, 1.0);
        d.rest = u16::MAX;
        if let Some(den) = d.den.as_mut() {
            den.awake = true;
        }
        (d.id, d.breed.kind)
    };
    let rival = g.promote_rival(kind, Deed::KillingBlow);
    if let Some(r) = g.warband.save.ledger.get_mut(rival) {
        r.rank = 3;
        r.lesson = Some(Lesson::Finisher);
    }
    g.war.grudges.seated.push((id, rival));
    let mut swayed = false;
    for _ in 0..(3.0 / crate::app::STEP) as usize {
        g.hero.hp = g.hero.max_hp;
        // (He holds his ground at the string's edge until the finisher comes.)
        if !g.brawl.as_ref().unwrap().hero.attack().is_some_and(|a| a.0 == MoveId::L3) {
            let d = g.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == id).unwrap();
            d.pos = V::new(hp.x, hp.y - 2.4);
        }
        g.input.inject(crate::input::Action::Attack);
        g.update(crate::app::STEP);
        let d = g.enemies.iter().filter_map(|e| e.duel.as_deref()).find(|d| d.id == id).unwrap();
        if matches!(d.f.state, crate::logic::combat::fsm::State::Evade { .. }) {
            swayed = true;
            assert!(g.brawl.as_ref().unwrap().hero.attack().is_some_and(|a| a.0 == MoveId::L3), "it sways as the finisher comes");
            break;
        }
    }
    assert!(swayed, "he read the finisher");
}

/// Nothing in the simulation reads the animation (ANIMATION.md): a bout played with the hero
/// animated by his clips and by the code comes out the same, step for step, his swings, guards and
/// leaps and the foes' answers alike; for each of the three heroes (Miyamoto with his fists on the
/// grip and his shield slung).
#[test]
fn the_clips_leave_the_fight_untouched() {
    for hero in 0..3 {
        clips_leave_the_fight_untouched(hero);
    }
}

fn clips_leave_the_fight_untouched(hero: usize) {
    let play = |code: bool| {
        let mut g = gym();
        g.view.set_anim_mode(code);
        g.debug_hero(hero);
        g.debug_ring(1);
        let (mut trace, mut posed) = (Vec::new(), 0);
        for step in 0..1500 {
            let k = step % 150;
            g.brawl_input(k == 10 || k == 34 || k == 58, k == 100, (120..140).contains(&k));
            g.update(crate::app::STEP);
            posed += usize::from(g.view.posed_by_clips());
            let p = g.hero.pos();
            let state = g.brawl.as_ref().map(|b| b.hero.state);
            trace.push(format!("{:.9} {:.9} {} {:.6} {:.6} {state:?}", p.x, p.y, g.hero.hp, g.hero.stamina, g.hero.altitude));
            for d in g.enemies.iter().filter_map(|e| e.duel.as_deref()) {
                trace.push(format!("{:.9} {:.9} {:.6} {:?}", d.pos.x, d.pos.y, d.hp, d.f.state));
            }
        }
        (trace, posed)
    };
    let ((code, by_code), (clips, by_clips)) = (play(true), play(false));
    assert_eq!(by_code, 0, "the code animated hero {hero}");
    assert!(by_clips > 1000, "hero {hero}'s clips posed him ({by_clips} steps)");
    assert_eq!(code.len(), clips.len());
    for (k, (a, b)) in code.iter().zip(&clips).enumerate() {
        assert_eq!(a, b, "hero {hero}: line {k} differs");
    }
}

/// The foes' clips leave the fight untouched too (ANIMATION.md §11): a bout against a pack of each
/// foe of the first three stages that has clips, played with the foes posed by their clips and by
/// the code, comes out the same, step for step.
#[test]
fn the_foes_clips_leave_the_fight_untouched() {
    use crate::logic::enemy::EnemyKind as K;
    use crate::render::anim::foe_rigs::FoeRig;
    let kinds: Vec<K> = [
        &K::STAGE_ONE[..],
        &K::STAGE_TWO,
        &K::STAGE_THREE,
        &K::STAGE_FOUR,
        &K::STAGE_FIVE,
        &K::STAGE_SIX,
        &K::STAGE_SEVEN,
        &K::REALMS_OWN,
        K::COURTS_OWN,
        &K::LAIR_OWN,
    ]
    .concat()
    .into_iter()
    .filter(|&k| FoeRig::of(k).is_some())
    .collect();
    assert!(!kinds.is_empty());
    for kind in kinds {
        let play = |code: bool| {
            let mut g = gym();
            g.view.set_anim_mode(code);
            g.debug_ring_of(&[kind, kind, kind]);
            let (mut trace, mut posed) = (Vec::new(), 0);
            for step in 0..1200 {
                let k = step % 150;
                g.brawl_input(k == 10 || k == 34 || k == 58, k == 100, (120..140).contains(&k));
                g.update(crate::app::STEP);
                posed += (0..g.enemies.len()).filter(|&i| g.view.foe_posed_by_clips(i)).count();
                let p = g.hero.pos();
                trace.push(format!("{:.9} {:.9} {} {:.6}", p.x, p.y, g.hero.hp, g.hero.stamina));
                for d in g.enemies.iter().filter_map(|e| e.duel.as_deref()) {
                    trace.push(format!("{:.9} {:.9} {:.6} {:?}", d.pos.x, d.pos.y, d.hp, d.f.state));
                }
            }
            (trace, posed)
        };
        let ((code, by_code), (clips, by_clips)) = (play(true), play(false));
        assert_eq!(by_code, 0, "{kind:?}: the code animated them");
        assert!(by_clips > 1000, "{kind:?}: their clips posed them ({by_clips} foe-steps)");
        assert_eq!(code.len(), clips.len());
        for (k, (a, b)) in code.iter().zip(&clips).enumerate() {
            assert_eq!(a, b, "{kind:?}: line {k} differs");
        }
    }
}
