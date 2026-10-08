//! The QA pass's tests (QA.md): Continue at every kind of checkpoint, a save file that is broken,
//! old or tampered with, and the story skipped at every beat.

use super::save::{Run, Stage};
use super::{Game, Screen};
use crate::app::STEP;
use crate::input::Action;
use crate::logic::heroes::{HeroKind, ROSTER};
use crate::logic::progression::Progress;
use crate::logic::rng::Rng;
use crate::logic::sins::{Sin, ALL_SINS, SINS};
use crate::logic::story::Scene;
use crate::logic::types::PotionKind;
use crate::logic::war::save::WarSave;
use crate::platform;
use crate::render::overlay::Overlay;
use std::path::PathBuf;

/// A save folder of this test's own, empty, that this thread's saves go to.
fn own_folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vh-qa-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    platform::use_dir_on_this_thread(dir.clone());
    dir
}

/// A game as the player starts it: what the save folder holds read in, saving on.
fn launched() -> Game {
    let mut g = Game::new();
    g.read_saved();
    g.persist.live = true;
    g
}

/// Updates until `screen` comes (pressing Enter now and then, as a player does to go on), at most
/// `secs` seconds; whether it came.
fn until(g: &mut Game, screen: Screen, secs: f64) -> bool {
    let mut t = 0.0;
    while t < secs {
        if g.screen == screen {
            return true;
        }
        if ((t / STEP) as u64).is_multiple_of(30) && matches!(g.screen, Screen::View | Screen::Won | Screen::GameOver) {
            g.input.inject(Action::Confirm);
        }
        g.update(STEP);
        t += STEP;
    }
    g.screen == screen
}

/// The HUD drawn over the game as it stands (on the CPU: what it lays out must not panic).
fn draw_hud(g: &Game) {
    let mut o = Overlay::new();
    for (w, h) in [(1600.0, 900.0), (1440.0, 900.0), (2560.0, 1080.0), (1024.0, 768.0)] {
        o.begin(w, h);
        crate::render::hud::draw(&mut o, g, 60);
    }
}

/// Every kind of checkpoint the game writes, as the save file holds it.
fn every_stage() -> Vec<Stage> {
    let mut stages = vec![Stage::Dungeon(0), Stage::Arena(0), Stage::Shop(0), Stage::Hub, Stage::Gym, Stage::Lair];
    for sin in SINS {
        stages.extend([Stage::Realm(sin), Stage::Court(sin), Stage::SinShop(sin)]);
    }
    stages.extend((1..=crate::logic::pit::LEVELS).map(Stage::Pit));
    stages.extend([Stage::Dungeon(1), Stage::Dungeon(2), Stage::Arena(1), Stage::Shop(1)]);
    stages
}

/// The keys a run at `stage` holds, as play would have left them.
fn sins_at(stage: Stage, k: usize) -> u8 {
    match stage {
        Stage::Pit(_) | Stage::Lair => ALL_SINS,
        Stage::SinShop(s) => s.bit() | SINS[(s.index() + 1) % 7].bit(),
        Stage::Realm(s) | Stage::Court(s) => SINS[(s.index() + 1 + k % 3) % 7].bit(),
        Stage::Hub => 0b000_0101,
        _ => 0,
    }
}

/**
 * Quit and Continue at every kind of checkpoint: the game closed with the run saved there (each
 * of the three heroes in turn, a belt of bottles, lives and keys), started again, the title
 * offers Continue and names the place, Continue puts him back at that place's start with what
 * he had, and play goes on from there: the countdown, or the old man's wares and the way on.
 */
#[test]
fn continue_comes_back_to_every_kind_of_checkpoint() {
    let _dir = own_folder("continue");
    for (k, stage) in every_stage().into_iter().enumerate() {
        let hero = ROSTER[k % ROSTER.len()];
        let sins = sins_at(stage, k);
        let belt = vec![PotionKind::Health, PotionKind::Stamina][..k % 3].to_vec();
        let run = Run { hero, stage, score: 100 + k as u32, lives: 1 + (k % 3) as u32, belt: belt.clone(), sins };
        {
            let mut g = launched();
            g.saved_run = Some(run.clone());
            g.progress.silver = 500;
            g.mark_saves();
            g.flush_saves();
        }
        // The game starts again.
        let mut g = launched();
        assert_eq!(g.saved_run.as_ref(), Some(&run), "{stage:?}: the checkpoint is read back");
        assert!(g.title_items().contains(&super::MenuItem::Continue), "{stage:?}: the title offers Continue");
        assert!(g.continue_caption().is_some_and(|c| c.starts_with(hero.name())), "{stage:?}");
        g.continue_game();
        assert_eq!((g.hero_kind, g.sins_cleared, g.score.lives), (hero, sins, run.lives), "{stage:?}: what he had");
        assert_eq!(g.belt, belt, "{stage:?}: the belt");
        match stage {
            Stage::Shop(_) | Stage::SinShop(_) => {
                assert_eq!(g.screen, Screen::Shop, "{stage:?}: back at the old man's");
                draw_hud(&g);
                // Out of the wares: on to the hub (with the sin's key set in the seal) or the next dungeon.
                g.input.inject(Action::Pause);
                g.update(STEP);
                g.input.inject(Action::Pause);
                g.update(STEP);
                assert_ne!(g.screen, Screen::Shop, "{stage:?}: Esc twice leaves the wares");
                match stage {
                    Stage::SinShop(s) => {
                        assert!(g.hub.is_some(), "{stage:?}: back in the Portals of Hell");
                        assert!(g.sins_cleared & s.bit() != 0);
                        assert_eq!(g.saved_run.as_ref().map(|r| r.stage), Some(Stage::Hub), "{stage:?}: saved there");
                    }
                    Stage::Shop(0) => assert!(g.hub.is_some(), "after the Garden's Cyclops, the hub"),
                    Stage::Shop(n) => assert_eq!(g.dungeon, n + 1, "off the road, the next dungeon"),
                    _ => unreachable!(),
                }
            }
            _ => {
                assert_eq!(g.screen, Screen::View, "{stage:?}: the camera is chosen first");
                match stage {
                    Stage::Realm(s) => assert!(g.realm == Some(s) && !g.arena(), "{stage:?}"),
                    Stage::Court(s) => assert!(g.realm == Some(s) && g.arena() && g.prince().is_some(), "{stage:?}"),
                    Stage::Pit(n) => assert_eq!(g.pit.as_ref().map(|p| p.level), Some(n), "{stage:?}"),
                    Stage::Lair => assert!(g.in_lair(), "{stage:?}"),
                    Stage::Hub => assert!(g.hub.is_some(), "{stage:?}"),
                    Stage::Arena(n) => assert!(g.boss.is_some() && g.dungeon == n, "{stage:?}"),
                    Stage::Dungeon(n) => assert_eq!(g.dungeon, n, "{stage:?}"),
                    Stage::Gym => assert!(g.theme().is_gym(), "{stage:?}"),
                    _ => {}
                }
            }
        }
        assert!(until(&mut g, Screen::Playing, 30.0), "{stage:?}: play goes on (on {:?})", g.screen);
        for _ in 0..(3.0 / STEP) as usize {
            g.update(STEP);
        }
        draw_hud(&g);
    }
}

/// The digit runs of `text` (where its numbers are), by byte range.
fn numbers(text: &str) -> Vec<(usize, usize)> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_digit() {
            let s = i;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            out.push((s, i));
        } else {
            i += 1;
        }
    }
    out
}

/// Ways a save line goes bad: cut short anywhere, each number replaced by an awkward one, bytes
/// flipped, and plain noise.
fn spoiled(valid: &str, rng: &mut Rng, most: usize) -> Vec<String> {
    const AWKWARD: [&str; 10] = ["0", "1", "7", "255", "256", "65535", "65536", "4294967295", "4294967296", "-1"];
    let mut out = vec![String::new(), " ".into(), "v1".into(), "v2".into(), "v3".into(), "\u{feff}".into(), "\0\0".into()];
    let step = (valid.len() / 60).max(1);
    for i in (0..valid.len()).step_by(step) {
        if valid.is_char_boundary(i) {
            out.push(valid[..i].to_string());
        }
    }
    let runs = numbers(valid);
    let mut picked: Vec<usize> = (0..runs.len()).collect();
    while picked.len() > most {
        let k = rng.index(picked.len());
        picked.swap_remove(k);
    }
    for &k in &picked {
        let (a, b) = runs[k];
        for v in AWKWARD {
            out.push(format!("{}{v}{}", &valid[..a], &valid[b..]));
        }
    }
    for _ in 0..40 {
        let mut bytes = valid.as_bytes().to_vec();
        for _ in 0..1 + rng.index(4) {
            if bytes.is_empty() {
                break;
            }
            let i = rng.index(bytes.len());
            bytes[i] = b" |,:;-0123456789xv"[rng.index(18)];
        }
        out.push(String::from_utf8_lossy(&bytes).into_owned());
    }
    out
}

/// A purse and unlocks well into a run: every hero part way up his trees, a patron, an outfit, a
/// broken shield, blunted blades, mastery under way.
fn rich_progress() -> Progress {
    let mut p = Progress { silver: 1234, runes: 17, ..Default::default() };
    for (k, hero) in ROSTER.into_iter().enumerate() {
        let h = p.hero_mut(hero);
        h.owned = [4 + k as u8, 3, 5, 2];
        h.relics = [1, 0, 2, 0];
        h.wield = 3;
        h.carry = 4;
        h.patron = Some(crate::logic::angels::ARCHANGELS[k]);
        h.outfit = k as u8;
        h.broken = 1 << 2;
        h.edge[1] = 40;
        h.mastery.rank = k as u8 + 1;
    }
    p
}

/**
 * A save file broken, old or tampered with never stops the game: each line it keeps (the run's
 * checkpoint, the purse and unlocks, the Warband) cut short, its numbers swapped for awkward ones
 * (0, 255, 65536, past u32, negative), bytes flipped. Whatever reads back is played: the wares
 * opened on every tab and the HUD drawn for a purse, Continue for a checkpoint, the Ledger's
 * board and a realm entered for a Warband.
 */
#[test]
fn a_spoiled_save_never_stops_the_game() {
    let _dir = own_folder("spoiled");
    let mut rng = Rng::new(42);
    let mut g = Game::new();
    // The purse and the unlocks.
    let valid = rich_progress().encode();
    assert_eq!(Progress::decode(&valid), Some(rich_progress()));
    for (k, text) in spoiled(&valid, &mut rng, 120).into_iter().enumerate() {
        let Some(p) = Progress::decode(&text) else { continue };
        g.hero_kind = ROSTER[k % ROSTER.len()];
        g.progress = p;
        g.equip_from_progress();
        g.open_shop();
        for _ in 0..8 {
            g.input.inject(Action::TabNext);
            g.update(STEP);
        }
        draw_hud(&g);
    }
    // The run's checkpoint.
    let run = Run {
        hero: HeroKind::Mehmed,
        stage: Stage::Court(Sin::Wrath),
        score: 4321,
        lives: 2,
        belt: vec![PotionKind::Health, PotionKind::Life],
        sins: 0b11,
    };
    let valid = run.encode();
    g.progress = Progress::default();
    for text in spoiled(&valid, &mut rng, 40).into_iter().chain(["v1 0 d 2 450 3 -".to_string(), "v1 2 a 9 0 0 9,9".to_string()]) {
        let Some(r) = Run::decode(&text) else { continue };
        g.enter(Screen::Title);
        g.saved_run = Some(r);
        g.continue_game();
        for _ in 0..30 {
            g.update(STEP);
        }
        draw_hud(&g);
    }
    // The Warband: the Ledger, the generals, the lies, the old man's once-flags.
    g.enter(Screen::Title);
    g.debug_ledger();
    g.warband.save.generals = g.general_records().encode();
    g.warband.save.lies = 3;
    let valid = g.warband.save.encode();
    assert!(WarSave::decode(&valid).is_some());
    let spoilt = spoiled(&valid, &mut rng, 60);
    for (k, text) in spoilt.iter().enumerate() {
        let Some(w) = WarSave::decode(text) else { continue };
        g.enter(Screen::Paused);
        g.warband.save = w;
        g.open_ledger();
        g.update(STEP);
        draw_hud(&g);
        g.warband.board_open = false;
        // (Into a realm with it now and then: its rivals seated, its general raised.)
        if k % 15 == 0 {
            g.map_jump(Stage::Realm(SINS[k % 7]));
            g.enter(Screen::Playing);
            for _ in 0..20 {
                g.update(STEP);
            }
            draw_hud(&g);
        }
    }
}

/**
 * The story skipped at every beat of Scene I (Esc), from Play on the title: the hero's choice
 * follows, Esc there goes back to the title, and Start again plays the story from its beginning.
 * Scene II skipped at every beat goes to the title. Nothing waits for ever.
 */
#[test]
fn the_story_skips_from_every_beat() {
    let _dir = own_folder("story");
    for scene in [Scene::One, Scene::Two] {
        for beat in 0..scene.beats().len() {
            let mut g = Game::new();
            g.intro = true;
            match scene {
                Scene::One => {
                    g.play_from_title();
                    assert_eq!(g.screen, Screen::Story);
                    g.story_jump(beat, 0.5);
                }
                Scene::Two => g.ending_jump(Scene::Two.time_at(beat, 0.5)),
            }
            g.update(STEP);
            g.input.inject(Action::Pause);
            let next = if scene == Scene::One { Screen::Select } else { Screen::Title };
            assert!(until(&mut g, next, 5.0), "{scene:?} skipped at beat {beat}: on {:?}", g.screen);
            if scene == Scene::One {
                // Back to the title, and Start plays the story again from its start.
                g.input.inject(Action::Pause);
                g.update(STEP);
                assert_eq!(g.screen, Screen::Title, "Esc on the heroes goes back");
                g.update(STEP);
                g.input.inject(Action::Confirm);
                g.update(STEP);
                assert_eq!(g.screen, Screen::Story, "Start plays it again");
                assert_eq!(g.story.as_ref().map(|s| s.beat), Some(0), "from the start");
            }
        }
    }
}

/// Scene I played through by a player who presses Enter whenever a line waits: the heroes come,
/// the hero is chosen, the camera, and the Garden's countdown (in the time the scene takes and a
/// little more).
#[test]
fn the_story_plays_through_to_the_garden() {
    let _dir = own_folder("story-through");
    let mut g = Game::new();
    g.intro = true;
    g.play_from_title();
    let total: f64 = Scene::One.beats().iter().map(|b| b.hold + 30.0).sum();
    let mut t = 0.0;
    while g.screen == Screen::Story && t < total {
        if ((t / STEP) as u64).is_multiple_of(45) {
            g.input.inject(Action::Confirm);
        }
        g.update(STEP);
        t += STEP;
    }
    assert_eq!(g.screen, Screen::Select, "the heroes, after {t:.0} s");
    g.update(STEP);
    g.input.inject(Action::Confirm);
    g.update(STEP);
    assert!(until(&mut g, Screen::Playing, 20.0), "into play: on {:?}", g.screen);
    assert_eq!(g.dungeon, 0);
}
