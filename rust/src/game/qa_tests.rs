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
        let run = Run { hero, stage, score: 100 + k as u32, lives: 1 + (k % 3) as u32, belt: belt.clone(), sins, ng: 0 };
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
        ng: 0,
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

/**
 * Away from the window as play is about to begin (alt-tab during the countdown, the loading or
 * the old man's door): the countdown ends on the pause menu, not in a fight he cannot see. Back,
 * he resumes as ever; a controller's press while the window is behind another one counts as
 * being back (QA.md Q1).
 */
#[test]
fn play_never_begins_while_he_is_away() {
    let _dir = own_folder("focus");
    let mut g = Game::new();
    g.map_jump(Stage::Realm(Sin::Greed));
    assert_eq!(g.screen, Screen::Ready);
    g.focus_lost();
    for _ in 0..((super::READY_SECONDS + 1.0) / STEP) as usize {
        g.update(STEP);
    }
    assert_eq!(g.screen, Screen::Paused, "the countdown ends on the pause menu");
    g.focus_gained();
    g.input.inject(Action::Pause);
    g.update(STEP);
    assert_eq!(g.screen, Screen::Playing, "back, Esc resumes");
    // Away mid-fight, the game pauses (as it did); a controller's Start while away resumes it.
    g.focus_lost();
    assert_eq!(g.screen, Screen::Paused);
    g.input.inject(Action::Pause);
    g.update(STEP);
    assert_eq!(g.screen, Screen::Playing, "a press while away: he is there");
    // The old man called, and left while away: his door opens on the pause menu too.
    g.call_old_man();
    assert_eq!(g.screen, Screen::Shop);
    g.focus_lost();
    g.input.inject(Action::Pause);
    g.update(STEP);
    g.focused = false;
    g.input.inject(Action::Pause);
    g.update(STEP);
    g.focused = false;
    for _ in 0..((super::READY_SECONDS + 1.0) / STEP) as usize {
        g.update(STEP);
    }
    assert_eq!(g.screen, Screen::Paused, "the countdown after the wares ends on the pause menu");
}

/**
 * A fall brings the realm's foes home whole; a foe whose den the realm has since turned to rock
 * (the Rotting Feast's teeth closed on its gatehouse while it chased him through) rises where it
 * stood, not inside the rock (QA.md Q3).
 */
#[test]
fn a_foe_whose_den_turned_to_rock_goes_home_where_it_stood() {
    let _dir = own_folder("den-rock");
    let mut g = Game::new();
    g.map_jump(Stage::Realm(Sin::Greed));
    g.enter(Screen::Playing);
    g.update(STEP);
    let grid = g.level.grid.clone();
    let open = |p: crate::logic::types::P| grid.is_walkable(p.x, p.y, false);
    let (i, home, stood) = g
        .enemies
        .iter()
        .enumerate()
        .find_map(|(i, e)| {
            let home = e.duel.as_deref()?.den?.home;
            let h = crate::logic::types::P::new(home.x.round() as i32, home.y.round() as i32);
            let near = [(2, 0), (-2, 0), (0, 2), (0, -2)]
                .iter()
                .map(|&(dx, dy)| crate::logic::types::P::new(h.x + dx, h.y + dy))
                .find(|p| open(*p))?;
            Some((i, h, near))
        })
        .expect("a foe at its den with floor beside it");
    g.enemies[i].duel.as_mut().unwrap().pos = stood.v();
    g.level.grid.set(home.x, home.y, crate::logic::types::Tile::Rock);
    g.reset_round();
    let d = g.enemies[i].duel.as_deref().unwrap();
    assert_eq!((d.pos.x, d.pos.y), (stood.x as f64, stood.y as f64), "home where it stood");
    assert!(d.den.is_some_and(|den| den.contains(d.pos)), "its den goes with it");
}

/**
 * The controller in his hands unplugged (or its battery dead) mid-fight: the game pauses, and
 * the countdown after it ends on the pause menu; a controller unplugged while he plays on the
 * keyboard changes nothing (QA.md Q5).
 */
#[test]
fn a_controller_lost_mid_fight_pauses_the_game() {
    let _dir = own_folder("pad-lost");
    let mut g = Game::new();
    g.map_jump(Stage::Court(Sin::Greed));
    g.enter(Screen::Playing);
    g.input.set_last_device(crate::input::Device::Xbox);
    g.controller_lost();
    assert_eq!(g.screen, Screen::Paused, "a controller lost mid-fight pauses");
    // On the keyboard, a controller pulled out of the machine is nothing to him.
    g.input.inject(Action::Pause);
    g.update(STEP);
    assert_eq!(g.screen, Screen::Playing);
    g.input.set_last_device(crate::input::Device::KeyboardMouse);
    g.controller_lost();
    assert_eq!(g.screen, Screen::Playing, "playing on the keyboard");
    // Lost during the countdown: it ends on the pause menu.
    g.input.set_last_device(crate::input::Device::Xbox);
    g.map_jump(Stage::Realm(Sin::Pride));
    g.controller_lost();
    for _ in 0..((super::READY_SECONDS + 1.0) / STEP) as usize {
        g.update(STEP);
    }
    assert_eq!(g.screen, Screen::Paused);
}

/// F2 (the testing cheat) sets him before the gate on its inner side, in a court's south wall as in
/// the garden's north hedge, never inside the wall (QA.md Q4).
#[test]
fn the_gate_cheat_sets_him_on_floor_before_every_gate() {
    let _dir = own_folder("gate-cheat");
    let mut stages = vec![Stage::Dungeon(0), Stage::Arena(0)];
    stages.extend(SINS.iter().map(|&s| Stage::Court(s)));
    for stage in stages {
        let mut g = Game::new();
        g.map_jump(stage);
        g.enter(Screen::Playing);
        if matches!(stage, Stage::Arena(_)) {
            g.debug_wound_boss(100_000);
        }
        g.cheat_to_gate();
        let p = g.hero.pos();
        assert!(g.level.grid.is_walkable(p.x.round() as i32, p.y.round() as i32, false), "{stage:?}: at ({:.1}, {:.1})", p.x, p.y);
    }
}

/// Every page that opens over the pause menu, opened and shut in turn (each as a player opens it,
/// and put away with Esc), then play resumed.
fn every_page_over_the_pause(g: &mut Game, guide: bool) {
    g.input.inject(Action::Pause);
    g.update(STEP);
    assert_eq!(g.screen, Screen::Paused, "Esc pauses");
    type Opens = fn(&mut Game);
    let pages: [(&str, Opens); 6] = [
        ("bestiary", Game::open_bestiary),
        ("settings", Game::open_settings),
        ("controls", Game::open_controls),
        ("full map", Game::open_full_map),
        ("ledger", Game::open_ledger),
        ("levels", Game::open_map),
    ];
    for (name, open) in pages {
        open(g);
        for _ in 0..3 {
            g.update(STEP);
        }
        draw_hud(g);
        g.input.inject(Action::Pause);
        g.update(STEP);
        g.update(STEP);
        assert_eq!(g.screen, Screen::Paused, "Esc puts the {name} away, the game still paused");
    }
    g.input.inject(Action::Pause);
    g.update(STEP);
    assert_eq!(g.screen, Screen::Playing, "Esc resumes");
    // The old man's guide (B, from play): up, and away again, play going on. (Not while the
    // Pit's platform goes down: nothing in play answers then.)
    if !guide {
        return;
    }
    g.input.inject(Action::Guide);
    g.update(STEP);
    assert!(g.guide_open && g.screen == Screen::Paused, "B: the guide");
    draw_hud(g);
    g.input.inject(Action::Guide);
    g.update(STEP);
    assert_eq!(g.screen, Screen::Playing, "B again: play goes on");
}

/// Plays until `done` holds (Enter pressed now and then, as a player goes on), at most `secs`.
fn play_until(g: &mut Game, secs: f64, done: &dyn Fn(&Game) -> bool) -> bool {
    let mut t = 0.0;
    while t < secs {
        if done(g) {
            return true;
        }
        if g.screen == Screen::Ready || g.screen == Screen::View {
            g.enter(Screen::Playing);
        }
        if ((t / STEP) as u64).is_multiple_of(30) && matches!(g.screen, Screen::Won) {
            g.input.inject(Action::Confirm);
        }
        g.update(STEP);
        t += STEP;
    }
    done(g)
}

/**
 * The pause, every page over it, and the old man called, in the middle of the game's special
 * moments: the Pit's platform going down, a prince's change of phase, Krampus at the cage and
 * his fall, a banner up, the old man's counsel speaking. Each moment goes on as it should when
 * play resumes (the platform reaches the next level, the prince fights on, the end begins).
 */
#[test]
fn every_special_moment_survives_the_pause_and_the_old_man() {
    let _dir = own_folder("moments");
    // The Pit's descent: paused and the old man called as the platform goes down.
    let mut g = Game::new();
    g.map_jump(Stage::Pit(2));
    g.enter(Screen::Playing);
    g.update(STEP);
    g.debug_pit_clock(crate::logic::pit::LEVEL_SECONDS - 0.05);
    for _ in 0..30 {
        g.update(STEP);
    }
    assert!(g.pit_view().is_some_and(|p| p.descent.is_some()), "the platform goes down");
    every_page_over_the_pause(&mut g, false);
    g.call_old_man();
    assert_eq!(g.screen, Screen::Shop);
    g.input.inject(Action::Pause);
    g.update(STEP);
    g.input.inject(Action::Pause);
    g.update(STEP);
    assert!(play_until(&mut g, 30.0, &|g| g.pit.as_ref().is_some_and(|p| p.level == 3)), "the platform reaches level 3");
    assert_eq!(g.saved_run.as_ref().map(|r| r.stage), Some(Stage::Pit(3)));

    // A prince's change of phase, in every court: paused at it, and he fights on to his fall.
    for sin in SINS {
        let mut g = Game::new();
        g.map_jump(Stage::Court(sin));
        g.enter(Screen::Playing);
        for _ in 0..30 {
            g.update(STEP);
        }
        let max = g.prince().map(|d| d.max_hp).expect("the prince");
        g.debug_wound_boss((max * 0.35) as i32);
        for _ in 0..3 {
            g.update(STEP);
        }
        every_page_over_the_pause(&mut g, true);
        g.call_old_man();
        g.input.inject(Action::Pause);
        g.update(STEP);
        g.input.inject(Action::Pause);
        g.update(STEP);
        assert!(play_until(&mut g, 10.0, &|g| g.screen == Screen::Playing), "{sin:?}: back to the fight");
        g.debug_wound_boss(100_000);
        assert!(play_until(&mut g, 20.0, &|g| g.court.as_ref().is_some_and(|c| c.slain)), "{sin:?}: he falls");
        draw_hud(&g);
    }

    // Krampus at the cage, then his fall: paused at each, and the end begins.
    let mut g = Game::new();
    g.map_jump(Stage::Lair);
    g.enter(Screen::Playing);
    for _ in 0..30 {
        g.update(STEP);
    }
    g.debug_taunt();
    for _ in 0..60 {
        g.update(STEP);
    }
    every_page_over_the_pause(&mut g, true);
    g.debug_fell();
    for _ in 0..20 {
        g.update(STEP);
    }
    every_page_over_the_pause(&mut g, true);
    g.call_old_man();
    g.input.inject(Action::Pause);
    g.update(STEP);
    g.input.inject(Action::Pause);
    g.update(STEP);
    assert!(play_until(&mut g, 60.0, &|g| g.story.as_ref().is_some_and(|s| s.scene == Scene::Two)), "the end begins (on {:?})", g.screen);
}

/**
 * Dying anywhere (on his feet or mid-leap; in the garden, a meadow, the hub, every realm and court,
 * the Pit and the lair): he rises at the place's start on floor he can stand on (not in rock, a pit
 * or the Burning Field's lava), and the last life lost goes to GAME OVER, then the title, where
 * Continue comes back to the same place.
 */
#[test]
fn dying_anywhere_brings_him_back_on_floor_and_the_last_life_to_the_title() {
    let _dir = own_folder("dying");
    let mut stages = vec![Stage::Dungeon(0), Stage::Arena(0), Stage::Hub, Stage::Pit(1), Stage::Pit(6), Stage::Lair];
    for sin in SINS {
        stages.extend([Stage::Realm(sin), Stage::Court(sin)]);
    }
    for stage in stages {
        let mut g = Game::new();
        g.map_jump(stage);
        g.enter(Screen::Playing);
        for _ in 0..30 {
            g.update(STEP);
        }
        let mut deaths = 0;
        loop {
            assert!(deaths < 8, "{stage:?}: the lives never run out");
            if deaths % 2 == 0 {
                g.input.inject(Action::Jump);
                g.update(STEP);
            }
            if g.brawling() {
                g.wound_hero(1.0e6, 0xffffff);
            } else {
                g.hero.invuln = 0.0;
                g.hero.shield = None;
                g.hero.hp = 1;
                g.debug_hurt();
            }
            deaths += 1;
            let mut t = 0.0;
            while !matches!(g.screen, Screen::Ready | Screen::GameOver) && t < 10.0 {
                g.update(STEP);
                t += STEP;
            }
            if g.screen == Screen::GameOver {
                break;
            }
            assert_eq!(g.screen, Screen::Ready, "{stage:?}: he rises again (death {deaths})");
            assert!(until(&mut g, Screen::Playing, 5.0), "{stage:?}");
            let p = g.hero.pos();
            let t = crate::logic::types::P::new(p.x.round() as i32, p.y.round() as i32);
            assert!(p.x.is_finite() && super::bot::Bot::floor(&g, t), "{stage:?}: he rises on floor, at ({:.1}, {:.1})", p.x, p.y);
        }
        g.input.inject(Action::Confirm);
        g.update(STEP);
        assert_eq!(g.screen, Screen::Title, "{stage:?}: GAME OVER, then the title");
        assert!(g.can_continue() && g.saved_run.as_ref().is_some_and(|r| r.stage == stage), "{stage:?}: Continue comes back here");
    }
}

/// The old man speaking as the game is paused: his panel is not drawn under the pause menu, where
/// the road down and the score sat over its words and its "Enter skip" read as the menu's (QA.md Q15).
#[test]
fn the_pause_hides_the_line_being_spoken() {
    let _dir = own_folder("pause-line");
    let mut g = Game::new();
    g.map_jump(Stage::Realm(Sin::Greed));
    g.enter(Screen::Playing);
    for _ in 0..30 {
        g.update(STEP);
    }
    g.debug_counsel();
    g.update(STEP);
    assert!(g.voices_line().is_some(), "he speaks");
    g.enter(Screen::Paused);
    let verts = |g: &Game| {
        let mut o = Overlay::new();
        o.begin(1920.0, 1080.0);
        crate::render::hud::draw(&mut o, g, 60);
        o.verts.len()
    };
    let speaking = verts(&g);
    g.voices.spoken = None;
    assert_eq!(speaking, verts(&g), "nothing of his panel under the pause");
}

/**
 * The bestiary (1.0.19) from every door: the title, the pause menu, the old man's guide (Back goes
 * back to his words), every group and page turned, and a saved book broken, old or tampered with
 * (cut short, numbers swapped for awkward ones, bytes flipped): nothing panics, the HUD draws it
 * at four screen shapes, and Back always closes it (QA.md).
 */
#[test]
fn the_bestiary_opens_everywhere_and_a_spoiled_book_never_stops_it() {
    use crate::logic::bestiary::{all_entries, Book};
    let _dir = own_folder("bestiary");
    let mut g = Game::new();
    // A book well filled: every entry met, some felled, rivals remembered.
    let mut book = Book::default();
    for (k, e) in all_entries().into_iter().enumerate() {
        book.met(e);
        if k % 2 == 0 {
            book.felled(e);
        }
        if k % 5 == 0 {
            book.felled_by(e);
        }
    }
    book.remember_rival("Thibault of Graufels", crate::logic::enemy::EnemyKind::Blemmyes, 3, true);
    let valid = book.encode();
    assert_eq!(Book::decode(&valid).encode(), valid, "it round-trips");
    let turn = |g: &mut Game| {
        for k in 0..12 {
            g.input.inject(if k % 3 == 0 { Action::TabNext } else { Action::Confirm });
            g.input.nav = Some(if k % 2 == 0 { crate::logic::types::Dir::Down } else { crate::logic::types::Dir::Right });
            g.update(STEP);
            draw_hud(g);
        }
    };
    // From the title.
    g.bestiary.book = book.clone();
    g.open_bestiary();
    assert!(g.bestiary.open);
    turn(&mut g);
    g.input.inject(Action::Pause);
    g.update(STEP);
    assert!(!g.bestiary.open && g.screen == Screen::Title, "Back closes it, on the title");
    // From the pause menu, and from the guide.
    g.map_jump(Stage::Realm(Sin::Lust));
    g.enter(Screen::Playing);
    g.update(STEP);
    g.input.inject(Action::Pause);
    g.update(STEP);
    g.open_bestiary();
    turn(&mut g);
    g.input.inject(Action::Pause);
    g.update(STEP);
    assert!(!g.bestiary.open && g.screen == Screen::Paused, "Back closes it, on the pause menu");
    g.input.inject(Action::Pause);
    g.update(STEP);
    g.input.inject(Action::Guide);
    g.update(STEP);
    assert!(g.guide_open);
    g.open_bestiary_from_guide();
    turn(&mut g);
    g.input.inject(Action::Pause);
    g.update(STEP);
    assert!(!g.bestiary.open && g.guide_open, "Back goes back to the old man's words");
    // A spoiled book.
    let mut rng = Rng::new(7);
    for (k, text) in spoiled(&valid, &mut rng, 80).into_iter().enumerate() {
        g.bestiary.book = Book::decode(&text);
        if k % 10 == 0 {
            g.enter(Screen::Paused);
            g.open_bestiary();
            turn(&mut g);
            g.input.inject(Action::Pause);
            g.update(STEP);
        }
        let _ = g.bestiary.book.encode();
        draw_hud(&g);
    }
}

/// Esc (Back) on the camera choice closes it: the camera in use stays, whatever was highlighted,
/// and play goes on to the countdown (QA.md Q13).
#[test]
fn back_closes_the_camera_choice_and_keeps_the_camera() {
    use crate::render::view::CameraMode;
    let _dir = own_folder("view-back");
    for before in [CameraMode::Third, CameraMode::Top] {
        let mut g = Game::new();
        g.view.camera_mode = before;
        g.saved_run = Some(Run { hero: HeroKind::Ragnar, stage: Stage::Dungeon(0), score: 0, lives: 3, belt: Vec::new(), sins: 0, ng: 0 });
        g.continue_game();
        assert_eq!(g.screen, Screen::View);
        // Another card highlighted, then Esc.
        g.input.nav = Some(crate::logic::types::Dir::Right);
        g.update(STEP);
        g.input.nav = Some(crate::logic::types::Dir::Right);
        g.update(STEP);
        g.input.inject(Action::Back);
        g.input.inject(Action::Pause);
        g.update(STEP);
        assert_eq!(g.screen, Screen::Ready, "Esc closes the choice ({before:?})");
        assert_eq!(g.view.camera_mode, before, "and keeps the camera in use");
        assert!(until(&mut g, Screen::Playing, 10.0), "play goes on");
    }
}

/// What a testing key could change, to see that none did.
fn testing_marks(g: &Game) -> String {
    let p = g.hero.pos();
    format!(
        "{:?} realm {:?} dungeon {} boss {} silver {} runes {} immortal {} map {} dev {} keys {} at ({:.2},{:.2}) foes {} readout {:?} bout {:?}",
        g.screen,
        g.realm,
        g.dungeon,
        g.boss.is_some(),
        g.progress.silver,
        g.progress.runes,
        g.immortal,
        g.show_map,
        g.dev,
        g.keys_got,
        p.x,
        p.y,
        g.enemies.len(),
        g.brawl.as_ref().map(|b| b.readout),
        g.spar.as_ref().map(|s| s.bout),
    )
}

/**
 * The testing keys (F1 the Cyclops, F2 the gate, F5 god mode, F6 gold, F7-F10 the ring and a rival,
 * F12 the developer's map, 0 the old man's wares) work only in developer builds: in a
 * distributable (`Game::testing_keys` off, as `DEV_BUILD` leaves it there) each, pressed on the
 * keyboard on the title, in play, in the vaults' brawl and in the Sparring Ring, does nothing. F3,
 * F4 and F11 (the frame counter, the grid, the full screen) are the player's in every build, and
 * the testing keys stay out of the bindable set (the user's call, 2026-10-09).
 */
#[test]
fn testing_keys_are_inert_outside_developer_builds() {
    use crate::input::bindings::{fixed_key, reserved, Control};
    use winit::keyboard::KeyCode;
    let _dir = own_folder("testing-keys");
    let testing = [
        Action::Cheat,
        Action::KeyCheat,
        Action::Immortal,
        Action::Gold,
        Action::NextStageBout,
        Action::Readout,
        Action::NextBout,
        Action::MakeRival,
        Action::DevMap,
        Action::ShopCheat,
    ];
    let keys: Vec<KeyCode> = [
        KeyCode::F1,
        KeyCode::F2,
        KeyCode::F5,
        KeyCode::F6,
        KeyCode::F7,
        KeyCode::F8,
        KeyCode::F9,
        KeyCode::F10,
        KeyCode::F12,
        KeyCode::Digit0,
        KeyCode::Numpad0,
    ]
    .into_iter()
    .collect();
    for k in &keys {
        assert!(fixed_key(*k).is_some_and(|a| testing.contains(&a)), "{k:?} is a testing key");
        assert!(reserved(Control::Key(*k)), "{k:?} stays out of the bindable set");
    }
    let press = |g: &mut Game, k: KeyCode| {
        g.input.key_down(k, false);
        g.update(STEP);
        g.input.key_up(k);
        for _ in 0..3 {
            g.update(STEP);
        }
    };
    let places: [(&str, Option<Stage>); 4] =
        [("title", None), ("garden", Some(Stage::Dungeon(0))), ("vaults", Some(Stage::Realm(Sin::Greed))), ("ring", Some(Stage::Gym))];
    for (name, stage) in places {
        for &k in &keys {
            let mut g = Game::new();
            g.testing_keys = false;
            g.dev = false;
            if let Some(stage) = stage {
                g.map_jump(stage);
                g.enter(Screen::Playing);
                g.hero.invuln = 1e9;
            }
            // Still for a moment, so only the key can change what is marked.
            g.input.blur();
            let before = testing_marks(&g);
            press(&mut g, k);
            g.input.blur();
            let after = testing_marks(&g);
            let still = |s: &str| s.split(" at (").next().unwrap_or("").to_string();
            assert_eq!(still(&after), still(&before), "{name}: {k:?} did something in a distributable");
        }
    }
    // (And where the testing keys work, they do: F6 in the garden.)
    let mut g = Game::new();
    g.map_jump(Stage::Dungeon(0));
    g.enter(Screen::Playing);
    let silver = g.progress.silver;
    press(&mut g, KeyCode::F6);
    assert_eq!(g.progress.silver, silver + super::CHEAT_COINS, "a developer build takes F6");
}

/**
 * `--dev` on the command line is heeded only in developer builds (`DEV_BUILD`): in a distributable
 * it leaves the developer's map hidden and the testing keys inert (F6 gives no silver), in a
 * developer build it turns both on (the user's call, 2026-10-09).
 */
#[test]
fn dev_on_the_command_line_is_ignored_outside_developer_builds() {
    use winit::keyboard::KeyCode;
    let _dir = own_folder("dev-flag");
    for dev_build in [false, true] {
        let mut g = Game::new();
        // (A distributable's game, as `Game::new` makes it there.)
        g.testing_keys = false;
        g.dev = false;
        g.ask_dev(true, dev_build);
        assert_eq!(
            (g.dev, g.testing_keys),
            (dev_build, dev_build),
            "--dev in a {} build",
            if dev_build { "developer" } else { "distributable" }
        );
        g.map_jump(Stage::Dungeon(0));
        g.enter(Screen::Playing);
        let silver = g.progress.silver;
        g.input.key_down(KeyCode::F6, false);
        g.update(STEP);
        g.input.key_up(KeyCode::F6);
        g.update(STEP);
        let gained = g.progress.silver - silver;
        assert_eq!(gained, if dev_build { super::CHEAT_COINS } else { 0 }, "F6 after --dev (developer build: {dev_build})");
    }
    // (Without --dev nothing changes either way.)
    let mut g = Game::new();
    g.testing_keys = false;
    g.dev = false;
    g.ask_dev(false, true);
    assert!(!g.dev && !g.testing_keys);
}

/**
 * After the end the finished run is closed (the user's call, 2026-10-09): the title offers no
 * Continue (now, and when the game is started again), Start begins a new adventure at once (no
 * "start over?", nothing to lose), in the Garden with nothing of the old run (no keys, no purse,
 * no unlocks, an empty Ledger), and the bestiary, its pages and the rivals it remembers, is intact.
 */
#[test]
fn after_the_end_start_begins_anew_and_the_bestiary_stays() {
    use crate::logic::bestiary::{all_entries, Book, BOOK_KEY};
    use crate::logic::enemy::EnemyKind;
    use crate::logic::progression::Tree;
    let _dir = own_folder("after-the-end");
    let mut book = Book::default();
    for e in all_entries().into_iter().take(12) {
        book.met(e);
        book.felled(e);
    }
    book.remember_rival("Thibault of Graufels", EnemyKind::Blemmyes, 3, false);
    {
        // A whole run behind him: at Krampus's lair, every key, a full purse, unlocks bought.
        let mut g = launched();
        g.bestiary.book = book.clone();
        g.bestiary.live = true;
        g.bestiary.dirty = true;
        g.saved_run = Some(Run {
            hero: HeroKind::Mehmed,
            stage: Stage::Lair,
            score: 9000,
            lives: 2,
            belt: vec![PotionKind::Health],
            sins: ALL_SINS,
            ng: 0,
        });
        g.progress.silver = 1500;
        g.progress.runes = 40;
        let _ = g.progress.unlock(HeroKind::Mehmed, Tree::Armory, 1);
        g.warband.save.ledger.promote(EnemyKind::Blemmyes, crate::logic::war::names::Deed::KillingBlow, HeroKind::Mehmed, 7);
        assert!(!g.ledger().rivals.is_empty());
        g.mark_saves();
        g.flush_saves();
        g.continue_game();
        assert!(g.in_lair());
        assert!(until(&mut g, Screen::Playing, 30.0));
        // Krampus falls and the friend reaches him: the end.
        g.lair_won();
        assert_eq!(g.screen, Screen::Story);
        assert!(g.finished);
        g.input.inject(Action::Pause);
        assert!(until(&mut g, Screen::Title, 10.0), "Esc skips the end to the title");
        assert!(!g.title_items().contains(&super::MenuItem::Continue), "no Continue after the end");
        assert_eq!(g.saved_run, None);
        assert_eq!(g.progress, Progress::default(), "the purse and the unlocks went with the run");
        assert!(g.ledger().rivals.is_empty(), "the Ledger is the run's");
        assert_eq!(g.bestiary.book, book, "the bestiary stays");
        g.flush_saves();
    }
    // The game started again: still no Continue; the book as it was saved.
    let mut g = launched();
    assert_eq!(g.saved_run, None);
    assert!(!g.title_items().contains(&super::MenuItem::Continue), "no Continue when started again");
    assert_eq!(platform::load(BOOK_KEY).map(|s| Book::decode(&s)), Some(book.clone()), "the saved book stays");
    g.bestiary.book = Book::decode(&platform::load(BOOK_KEY).unwrap_or_default());
    // Start: a new adventure at once (nothing to ask about), from the heroes.
    g.menu_index = g.title_items().iter().position(|&m| m == super::MenuItem::Play).unwrap();
    g.input.inject(Action::Confirm);
    g.update(STEP);
    assert!(!g.confirm_reset, "nothing to lose: Start does not ask");
    assert_eq!(g.screen, Screen::Select);
    g.input.inject(Action::Confirm);
    g.update(STEP);
    assert!(until(&mut g, Screen::Playing, 30.0));
    assert_eq!((g.dungeon, g.realm, g.sins_cleared, g.keys_got), (0, None, 0, 0), "a fresh run, in the Garden");
    assert_eq!(g.progress, Progress::default());
    assert_eq!(g.saved_run.as_ref().map(|r| r.stage), Some(Stage::Dungeon(0)), "saved as a new run");
    assert_eq!(g.bestiary.book.rival("Thibault of Graufels").map(|r| r.name.as_str()), Some("Thibault of Graufels"));
    assert!(all_entries().into_iter().take(12).all(|e| g.bestiary.book.unlocked(e)), "every page still open");
}

/**
 * New Game+ (the user's call, 2026-10-09): once the tale is told the title offers NEW GAME+ beside
 * Start (and when the game is started again). Taken, the adventure begins from the heroes with the
 * finished run's kit and purse, its living rivals back in the Ledger, every realm a tier harder
 * (the Garden a stage deeper), the bestiary intact; its checkpoint carries the level and uses the
 * offer up. Finished again, it offers NG+2. Start beside it still begins a fresh adventure.
 */
#[test]
fn after_the_end_new_game_plus_keeps_the_kit_a_tier_harder() {
    use super::MenuItem;
    use crate::logic::bestiary::{all_entries, Book};
    use crate::logic::enemy::EnemyKind;
    use crate::logic::progression::Tree;
    use crate::logic::war::names::Deed;
    let _dir = own_folder("new-game-plus");
    let mut book = Book::default();
    for e in all_entries().into_iter().take(8) {
        book.met(e);
    }
    let (kit, rivals) = {
        // A whole run behind him, as the end begins: a purse, unlocks, two rivals (one slain).
        let mut g = launched();
        g.bestiary.book = book.clone();
        g.saved_run = Some(Run {
            hero: HeroKind::Mehmed,
            stage: Stage::Lair,
            score: 9000,
            lives: 2,
            belt: vec![PotionKind::Health],
            sins: ALL_SINS,
            ng: 0,
        });
        g.progress.silver = 1500;
        g.progress.runes = 40;
        let _ = g.progress.unlock(HeroKind::Mehmed, Tree::Armory, 1);
        let alive = g.warband.save.ledger.promote(EnemyKind::Blemmyes, Deed::KillingBlow, HeroKind::Mehmed, 7);
        let slain = g.warband.save.ledger.promote(EnemyKind::Oni, Deed::KillingBlow, HeroKind::Mehmed, 9);
        if let Some(r) = g.warband.save.ledger.get_mut(slain) {
            r.alive = false;
        }
        g.mark_saves();
        g.flush_saves();
        g.continue_game();
        assert!(until(&mut g, Screen::Playing, 30.0));
        let kit = g.progress;
        let name = g.ledger().get(alive).map(|r| r.full_name()).unwrap();
        g.lair_won();
        g.input.inject(Action::Pause);
        assert!(until(&mut g, Screen::Title, 10.0));
        assert!(g.title_items().contains(&MenuItem::NewGamePlus), "the title offers New Game+ once the tale is told");
        assert!(g.title_items().contains(&MenuItem::Play), "...beside Start");
        assert_eq!(g.ng_offer.as_ref().map(|o| o.level), Some(1));
        g.flush_saves();
        (kit, name)
    };
    // Started again: the offer stands. New Game+: nothing under way to lose, so it begins at once.
    let mut g = launched();
    g.bestiary.book = book.clone();
    assert!(!g.title_items().contains(&MenuItem::Continue));
    g.menu_index = g.title_items().iter().position(|&m| m == MenuItem::NewGamePlus).expect("offered when started again");
    g.input.inject(Action::Confirm);
    g.update(STEP);
    assert!(!g.confirm_reset);
    assert_eq!(g.screen, Screen::Select);
    g.select_index = HeroKind::Mehmed.slot();
    g.input.inject(Action::Confirm);
    g.update(STEP);
    assert!(until(&mut g, Screen::Playing, 30.0));
    assert_eq!((g.dungeon, g.sins_cleared, g.keys_got), (0, 0, 0), "from the start, in the Garden");
    assert_eq!(g.progress, kit, "the kit and the purse kept");
    assert!(g.progress.hero(HeroKind::Mehmed).has(Tree::Armory, 1));
    assert_eq!(g.ledger().rivals.iter().map(|r| r.full_name()).collect::<Vec<_>>(), vec![rivals.clone()], "the living rival is back");
    assert_eq!(g.ng_plus, 1);
    assert_eq!(g.realm_tier(), 2, "a tier harder: the first realm as the second");
    assert_eq!(g.foe_stage(), 2, "the Garden a stage deeper");
    assert_eq!(g.saved_run.as_ref().map(|r| (r.stage, r.ng)), Some((Stage::Dungeon(0), 1)), "the checkpoint carries it");
    assert!(g.ng_offer.is_none(), "the offer is used up");
    assert!(all_entries().into_iter().take(8).all(|e| g.bestiary.book.unlocked(e)), "the bestiary intact");
    g.flush_saves();
    // Started again mid-way: Continue says so, and the level comes back with the run.
    let mut g = launched();
    assert_eq!(g.ng_plus, 1);
    assert!(g.continue_caption().is_some_and(|c| c.contains("New Game+1")));
    assert!(!g.title_items().contains(&MenuItem::NewGamePlus), "no offer while it is under way");
    // Finished again: NG+2 is offered.
    g.saved_run.as_mut().unwrap().stage = Stage::Lair;
    g.saved_run.as_mut().unwrap().sins = ALL_SINS;
    g.continue_game();
    assert!(until(&mut g, Screen::Playing, 30.0));
    assert_eq!(g.realm_tier(), 9);
    g.lair_won();
    assert_eq!(g.ng_offer.as_ref().map(|o| o.level), Some(2));
    g.input.inject(Action::Pause);
    assert!(until(&mut g, Screen::Title, 10.0));
    // Start beside it: a fresh adventure, nothing kept, a first run's tiers.
    g.menu_index = g.title_items().iter().position(|&m| m == MenuItem::Play).unwrap();
    g.input.inject(Action::Confirm);
    g.update(STEP);
    g.input.inject(Action::Confirm);
    g.update(STEP);
    assert!(until(&mut g, Screen::Playing, 30.0));
    assert_eq!(g.progress, Progress::default());
    assert!(g.ledger().rivals.is_empty());
    assert_eq!((g.ng_plus, g.realm_tier(), g.foe_stage()), (0, 1, 1));
    assert_eq!(g.saved_run.as_ref().map(|r| r.ng), Some(0));
    assert!(g.ng_offer.is_some(), "the offer stays on the title for another time");
}

/**
 * The GPU lost mid-fight (QA.md Q2; the app drops the renderer and starts a new one): whatever save
 * is marked is on disk before the renderer goes, play waits on the pause menu (and does not begin
 * again by itself), the level is marked to go up whole to the new renderer, and a line says what
 * happened; play goes on as before once he resumes.
 */
#[test]
fn a_lost_gpu_saves_first_and_waits_on_the_pause_menu() {
    let _dir = own_folder("gpu-lost");
    let mut g = launched();
    g.saved_run =
        Some(Run { hero: HeroKind::Ragnar, stage: Stage::Realm(Sin::Greed), score: 10, lives: 3, belt: Vec::new(), sins: 0, ng: 0 });
    g.continue_game();
    assert!(until(&mut g, Screen::Playing, 30.0));
    for _ in 0..60 {
        g.update(STEP);
    }
    // Silver earned since the checkpoint, its save marked but not yet written.
    g.progress.silver += 77;
    g.mark_saves();
    let silver = g.progress.silver;
    assert_ne!(super::persist::read_progress().silver, silver, "(not yet on disk)");
    g.view.static_dirty = false;
    g.renderer_lost();
    assert_eq!(super::persist::read_progress().silver, silver, "saved before the renderer goes");
    assert_eq!(super::save::read_run().map(|r| r.stage), Some(Stage::Realm(Sin::Greed)));
    assert_eq!(g.screen, Screen::Paused, "play waits for him");
    assert!(g.view.static_dirty, "the level goes up whole to the new renderer");
    assert!(g.juice.banner.as_ref().is_some_and(|b| b.text.contains("SCREEN")));
    let at = g.hero.pos();
    for _ in 0..(3.0 / STEP) as usize {
        g.update(STEP);
    }
    assert_eq!((g.screen, g.hero.pos()), (Screen::Paused, at), "nothing goes on meanwhile");
    g.focus_gained();
    g.input.inject(Action::Pause);
    g.update(STEP);
    assert_eq!(g.screen, Screen::Playing, "and on when he resumes");
}
