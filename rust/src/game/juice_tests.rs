//! The juice through the game (WARBAND.md §4): the freeze's budget, the slowing of time and its
//! limits, the frame relations it keeps, the harnesses' switch, and the shatter replacing a body.

use super::sparring::BoutPhase;
use super::*;
use crate::logic::juice::{Dilation, DilationKind, Matter, Stop, Trauma};
use crate::render::profile::fx;

fn gym() -> Game {
    std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
    let mut g = Game::new();
    g.reseed(7);
    g.map_jump(save::Stage::Gym);
    g.enter(Screen::Playing);
    g
}

/// Bout `n` of the ring, its foes fighting at once (no countdown).
fn bout(n: usize) -> Game {
    let mut g = gym();
    g.debug_ring(n);
    if let Some(s) = g.spar.as_mut() {
        s.phase = BoutPhase::Fight;
    }
    if let Some(b) = g.brawl.as_mut() {
        b.live = true;
    }
    g
}

fn frame(g: &Game) -> u32 {
    g.brawl.as_ref().unwrap().threat.frame
}

#[test]
fn the_freeze_follows_the_table_within_its_budget() {
    let mut g = gym();
    g.freeze(Stop::Break);
    assert!((g.hitstop - 8.0 / 60.0).abs() < 1e-9);
    // A freeze inside one already holding holds only as long as the longer.
    g.freeze(Stop::Blocked);
    assert!((g.hitstop - 8.0 / 60.0).abs() < 1e-9);
    // Freeze after freeze in the same second: 8, then 6 to the budget, then halves.
    g.hitstop = 0.0;
    g.freeze(Stop::Break);
    assert!((g.hitstop - 6.0 / 60.0).abs() < 1e-9, "{}", g.hitstop * 60.0);
    g.hitstop = 0.0;
    g.freeze(Stop::Break);
    assert!((g.hitstop - 4.0 / 60.0).abs() < 1e-9, "past the budget, halved: {}", g.hitstop * 60.0);
    // A second later the window has moved on.
    g.time += 1.01;
    g.hitstop = 0.0;
    g.freeze(Stop::Parry { prince: true });
    assert!((g.hitstop - 12.0 / 60.0).abs() < 1e-9);
}

#[test]
fn with_the_juice_off_the_brawl_freezes_as_it_always_did() {
    let mut g = gym();
    g.set_juice_off(fx::JUICE);
    g.freeze(Stop::Blocked);
    assert_eq!(g.hitstop, 0.0, "no freeze on a block before the table");
    g.freeze(Stop::Struck(9));
    assert!((g.hitstop - 9.0 / 60.0).abs() < 1e-9, "the blow's own");
    for _ in 0..5 {
        g.hitstop = 0.0;
        g.freeze(Stop::Break);
        assert!((g.hitstop - 8.0 / 60.0).abs() < 1e-9, "and no budget");
    }
    assert!(!g.dilate(DilationKind::Trophy), "nor does time slow");
}

#[test]
fn time_slows_at_most_once_in_three_seconds_and_eases_back() {
    let mut g = bout(1);
    assert!(g.dilate(DilationKind::LastOfPack));
    assert!(!g.dilate(DilationKind::Trophy), "one slowing in three seconds");
    g.update(crate::app::STEP);
    assert_eq!(g.time_scale(), 0.35);
    assert_eq!(g.pace(), 0.35);
    let d = DilationKind::LastOfPack.dilation();
    for _ in 0..((d.length() / crate::app::STEP) as usize + 2) {
        g.update(crate::app::STEP);
    }
    assert_eq!(g.time_scale(), 1.0, "eased back");
    g.time += 3.0;
    assert!(g.dilate(DilationKind::Trophy), "three seconds on, it may slow again");
    // Not with the setting off, nor under the pause.
    let mut g = bout(1);
    g.toggle_slow_motion();
    assert!(!g.dilate(DilationKind::Trophy));
    let mut g = bout(1);
    g.enter(Screen::Paused);
    assert!(!g.dilate(DilationKind::Trophy));
    g.update(crate::app::STEP);
    assert_eq!(g.pace(), 0.0, "the pause holds the bodies' springs");
}

#[test]
fn a_freeze_then_the_slowing_of_time() {
    // The kill's freeze holds first; the slowing of time begins when it ends.
    let mut g = bout(1);
    g.freeze(Stop::LastOfPack);
    assert!(g.dilate(DilationKind::LastOfPack));
    let f = frame(&g);
    for _ in 0..12 {
        g.update(crate::app::STEP);
        assert_eq!(g.pace(), 0.0);
    }
    assert_eq!(frame(&g), f, "frozen: not a frame passes");
    g.update(crate::app::STEP);
    g.update(crate::app::STEP);
    assert_eq!(g.time_scale(), 0.35, "the whole hold is still to come");
}

/// The fight's story frame by frame: every duelist's state and health, and his.
fn record(g: &Game) -> String {
    let b = g.brawl.as_ref().unwrap();
    let mut s = format!("{} {:?} hp {} |", b.threat.frame, b.hero.state, g.hero.hp);
    for d in g.enemies.iter().filter_map(|e| e.duel.as_deref()) {
        s += &format!(" {} {:?} {:.0} {:.2},{:.2};", d.id, d.f.state, d.hp, d.pos.x, d.pos.y);
    }
    s
}

/// Plays bout `n` for `frames` of its frames at time scale `scale`, the hero pressing attack on
/// a few frames and raising his guard on others, and records every frame.
fn play_at(n: usize, scale: f64, frames: u32) -> Vec<String> {
    let mut g = bout(n);
    g.hero.free_move = true;
    if scale < 1.0 {
        g.tempo.dilations.push((Dilation { scale, hold: 1e9, ease: 0.1 }, 0.0));
    }
    let start = frame(&g);
    let mut out = Vec::new();
    let mut last = start;
    let mut pressed = u32::MAX;
    for _ in 0..(frames as usize * 2 * 4 + 1000) {
        let f = frame(&g) - start;
        if f >= frames || !g.hero.alive {
            break;
        }
        // His buttons go by the fight's frames, never mid-freeze (where they would wait for its end).
        if g.hitstop <= 0.0 && pressed != f && matches!(f % 50, 10 | 11) {
            g.input.inject(crate::input::Action::Attack);
            pressed = f;
        }
        let guard = (f / 70) % 3 == 2;
        if guard && !g.input.held(crate::input::Action::Block) {
            g.input.inject(crate::input::Action::Block);
        } else if !guard && g.input.held(crate::input::Action::Block) {
            g.input.inject_release(crate::input::Action::Block);
        }
        g.update(crate::app::STEP);
        let now = frame(&g);
        assert!(now - last <= 1, "one frame a step at most here");
        if now != last {
            out.push(record(&g));
            last = now;
        }
    }
    out
}

#[test]
fn time_dilation_keeps_every_frame_relation() {
    // The same fight at full speed and at 0.35 lands the same frames in the same order: every
    // telegraph, strike, reel and wound on the same frame number (WARBAND.md I6).
    for n in [1, 4] {
        let full = play_at(n, 1.0, 420);
        let slow = play_at(n, 0.35, 420);
        assert!(full.len() >= 400, "{}", full.len());
        // (A fight, not a stand-off: he swings, they strike, someone is hurt.)
        let (his, theirs): (Vec<&str>, Vec<&str>) = full.iter().map(|r| r.split_once('|').unwrap()).unzip();
        assert!(his.iter().any(|h| h.contains("Attack")), "he never swung");
        assert!(theirs.iter().any(|t| t.contains("Attack")), "they never struck");
        assert_ne!(full.first().map(|r| r.split_once('|').unwrap().1), full.last().map(|r| r.split_once('|').unwrap().1));
        for (k, (a, b)) in full.iter().zip(slow.iter()).enumerate() {
            assert_eq!(a, b, "bout {n}, frame {k} differs");
        }
        assert_eq!(full.len(), slow.len());
    }
}

#[test]
fn a_slain_foe_topples_and_breaks_into_its_voxels() {
    let mut g = bout(1);
    g.update(crate::app::STEP);
    let i = (0..g.enemies.len()).find(|&i| g.enemies[i].duel.is_some()).unwrap();
    let before = g.view.debris_count();
    g.debug_juice(14);
    assert_eq!(g.view.toppling(), 1);
    // The kill's freeze, then six frames of tipping over: the foe itself is gone from the start.
    for _ in 0..12 {
        g.update(crate::app::STEP);
    }
    assert_eq!(g.view.toppling(), 1, "the freeze holds the fall");
    assert!(!g.enemies[i].duel.as_ref().unwrap().alive());
    for _ in 0..16 {
        g.update(crate::app::STEP);
    }
    assert_eq!(g.view.toppling(), 0);
    let made = g.view.debris_count() - before;
    assert!((24..=60).contains(&made), "{made} pieces");
    // Off, nothing breaks.
    let mut g = bout(1);
    g.set_juice_off(fx::DEBRIS);
    g.update(crate::app::STEP);
    g.sync_feel();
    g.debug_juice(14);
    assert_eq!(g.view.toppling(), 0);
}

#[test]
fn chips_and_trauma_stay_in_their_budgets() {
    let mut g = bout(1);
    g.update(crate::app::STEP);
    for k in 0..200 {
        let m = Matter::ALL[k % Matter::ALL.len()];
        g.view.chips(&g.level.grid, g.hero.pos(), crate::logic::types::V::new(1.0, 0.0), m, 10);
        g.view.trauma(Trauma::Blow);
    }
    assert!(g.view.debris_count() <= crate::render::view::effects::MAX_DEBRIS);
    g.update(crate::app::STEP);
    assert!(g.view.camera_roll().abs() <= crate::logic::juice::ROLL_DEGREES.to_radians() * 0.2 * 0.2 + 1e-6);
}

#[test]
fn the_settings_page_steps_the_feels_settings() {
    use super::settings_page::{rows, Row, Tab};
    let mut g = bout(1);
    g.enter(Screen::Paused);
    g.open_settings_on(Tab::Gameplay);
    g.update(crate::app::STEP);
    let row = |r: Row| rows(Tab::Gameplay).iter().position(|&x| x == r).unwrap();
    // The shake: a step down from full, and its value as the page writes it.
    g.settings_page.row = row(Row::Shake);
    g.input.nav = Some(crate::logic::types::Dir::Left);
    g.update(crate::app::STEP);
    assert_eq!(g.feel.shake, 75);
    assert_eq!(g.settings_value(Row::Shake), "75%");
    // Slow motion, off.
    g.settings_page.row = row(Row::SlowMotion);
    g.input.inject(crate::input::Action::Confirm);
    g.update(crate::app::STEP);
    assert!(!g.feel.slow_motion);
    assert_eq!(g.settings_value(Row::SlowMotion), "Off");
    assert_eq!(g.screen, Screen::Paused, "and the game stays paused");
    assert!(g.settings_page.open);
}

/// The debris at its budget (480 pieces) costs the view under 0.3 ms a step, flying and drawn
/// (WARBAND.md §4.4). Timed, so run by hand in an optimised build:
/// `cargo test --profile quick -- --ignored debris_at_its_budget --nocapture`.
#[test]
#[ignore]
fn debris_at_its_budget_is_cheap() {
    use crate::render::view::effects::MAX_DEBRIS;
    let mut g = bout(1);
    g.update(crate::app::STEP);
    let mut frame = crate::render::view::Frame::default();
    let time = |g: &mut Game, frame: &mut crate::render::view::Frame| {
        let n = 3000;
        let t = std::time::Instant::now();
        for _ in 0..n {
            // (A step too short for any piece to land or die: the same 480 all the way.)
            g.update(1e-6);
            g.view.emit(&g.scene(), frame);
        }
        t.elapsed().as_secs_f64() * 1e3 / n as f64
    };
    let bare = time(&mut g, &mut frame);
    let at = g.hero.pos();
    for k in 0..400 {
        g.view.chips(&g.level.grid, at, crate::logic::types::V::new(1.0, 0.3), Matter::ALL[k % 12], 10);
    }
    let count = g.view.debris_count();
    assert_eq!(count, MAX_DEBRIS);
    let full = time(&mut g, &mut frame);
    eprintln!("debris: {count} pieces cost {:.4} ms a step ({bare:.4} ms bare, {full:.4} ms full)", full - bare);
    assert!(full - bare < 0.3);
}

/// Bout 1 with its foes standing by: his fighter alone moves, so what it begins is his doing.
fn still_bout() -> Game {
    let mut g = bout(1);
    g.brawl.as_mut().unwrap().live = false;
    g.update(crate::app::STEP);
    g
}

/// A tap of `a`: pressed and let go before the step reads it.
fn tap(g: &mut Game, a: crate::input::Action) {
    g.input.inject(a);
    g.input.inject_release(a);
}

/// His fighter's state.
fn his(g: &Game) -> crate::logic::combat::fsm::State {
    g.brawl.as_ref().unwrap().hero.state
}

/// What his fighter begins over `steps` updates: each new swing by its move, each new leap "evade".
fn begun(g: &mut Game, steps: usize) -> Vec<String> {
    use crate::logic::combat::fsm::State;
    let mut out = Vec::new();
    let mut last = his(g);
    for _ in 0..steps {
        g.update(crate::app::STEP);
        let now = his(g);
        match (last, now) {
            (State::Attack { mv: a, frame: fa }, State::Attack { mv: b, frame: fb }) if a == b && fb >= fa => {}
            (_, State::Attack { mv, .. }) => out.push(format!("{mv:?}")),
            (State::Evade { frame: fa }, State::Evade { frame: fb }) if fb >= fa => {}
            (_, State::Evade { .. }) => out.push("evade".into()),
            _ => {}
        }
        last = now;
    }
    out
}

#[test]
fn a_press_in_a_freeze_is_kept_for_the_step_that_ends_it() {
    use crate::input::Action;
    use crate::logic::combat::fsm::State;
    for (action, leap) in [(Action::Attack, false), (Action::Jump, true)] {
        // With no freeze, the press begins the move on the fight's next frame.
        let mut g = still_bout();
        assert_eq!(his(&g), State::Neutral);
        tap(&mut g, action);
        let now = begun(&mut g, 2);
        assert_eq!(now.len(), 1, "{action:?} with no freeze: {now:?}");
        assert_eq!(now[0] == "evade", leap);
        let mut more = begun(&mut g, 120);
        assert!(more.is_empty(), "one move a press: {more:?}");
        // In a freeze, pressed at its start, in its middle and on its last frozen step: nothing
        // moves while it holds, then the move begins on the first frame after it, once.
        for at in ["start", "middle", "end"] {
            let mut g = still_bout();
            g.freeze(Stop::Break);
            let steps = (g.hitstop / crate::app::STEP).round() as usize;
            assert!(steps >= 12, "{steps}");
            let mut s = 0;
            while g.hitstop > 0.0 {
                // (The end: the step after which the freeze no longer holds.)
                let last = g.hitstop - crate::app::STEP <= 0.0;
                if (at == "start" && s == 0) || (at == "middle" && s == steps / 2) || (at == "end" && last) {
                    tap(&mut g, action);
                }
                g.update(crate::app::STEP);
                assert_eq!(g.pace(), 0.0, "{at}: step {s} is frozen");
                assert_eq!(his(&g), State::Neutral, "{at}: nothing moves in the freeze");
                s += 1;
            }
            assert!(s >= steps);
            more = begun(&mut g, 2);
            assert_eq!(more, now, "{action:?} pressed at the freeze's {at}");
            let after = begun(&mut g, 120);
            assert!(after.is_empty(), "{action:?} at the freeze's {at}: one move a press, not {after:?}");
        }
    }
}

#[test]
fn a_kept_press_is_the_steps_alone() {
    use crate::input::Action;
    let mut g = still_bout();
    g.freeze(Stop::Break);
    g.input.flush();
    tap(&mut g, Action::Attack);
    g.play_step(crate::app::STEP);
    g.input.flush();
    while g.hitstop > 0.0 {
        g.play_step(crate::app::STEP);
    }
    // The step that ends the freeze reads it, and what reads the buttons after it does not.
    g.play_step(crate::app::STEP);
    assert!(!g.input.was_pressed(&[Action::Attack]));
    assert!(g.tempo.kept.is_empty());
    // A press of the same button on that very step is its own, and stays for the rest.
    let mut g = still_bout();
    g.freeze(Stop::Break);
    tap(&mut g, Action::Attack);
    g.play_step(crate::app::STEP);
    g.input.flush();
    while g.hitstop > 0.0 {
        g.play_step(crate::app::STEP);
    }
    tap(&mut g, Action::Attack);
    g.play_step(crate::app::STEP);
    assert!(g.input.was_pressed(&[Action::Attack]));
    // A new round keeps nothing from a freeze.
    let mut g = still_bout();
    g.freeze(Stop::Break);
    tap(&mut g, Action::Attack);
    g.play_step(crate::app::STEP);
    assert_eq!(g.tempo.kept, vec![Action::Attack]);
    g.reset_tempo();
    assert!(g.tempo.kept.is_empty());
}

#[test]
fn the_guard_holds_through_a_freeze() {
    use crate::input::Action;
    use crate::logic::combat::fsm::State;
    let mut g = still_bout();
    g.freeze(Stop::Break);
    g.input.inject(Action::Block);
    while g.hitstop > 0.0 {
        g.update(crate::app::STEP);
        assert_eq!(his(&g), State::Neutral);
    }
    for _ in 0..4 {
        g.update(crate::app::STEP);
    }
    assert!(matches!(his(&g), State::Guard { .. }), "{:?}", his(&g));
}

#[test]
fn slow_motion_keeps_every_press() {
    use crate::input::Action;
    for action in [Action::Attack, Action::Jump] {
        // On each step of a slowed frame of the fight, those that make no frame of their own too.
        for k in 0..6 {
            let mut g = still_bout();
            g.tempo.dilations.push((Dilation { scale: 0.35, hold: 1e9, ease: 0.1 }, 0.0));
            for _ in 0..k {
                g.update(crate::app::STEP);
            }
            tap(&mut g, action);
            let now = begun(&mut g, 200);
            assert_eq!(now.len(), 1, "{action:?} on step {k} of a slowed frame: {now:?}");
        }
    }
}

#[test]
fn out_of_the_brawl_a_jump_in_a_freeze_comes_after_it() {
    use crate::input::Action;
    let mut g = gym();
    g.update(crate::app::STEP);
    assert!(!g.brawling());
    g.freeze_frames(8.0);
    let mut s = 0;
    while g.hitstop > 0.0 {
        if s == 3 {
            tap(&mut g, Action::Jump);
        }
        g.update(crate::app::STEP);
        assert!(!g.hero.airborne(), "nothing moves in the freeze");
        s += 1;
    }
    g.update(crate::app::STEP);
    assert!(g.hero.airborne(), "the jump, once the freeze ends");
}
