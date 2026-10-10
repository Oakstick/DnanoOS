//! B78: one jump everywhere. The leap goes as far in the Garden, the meadow, a realm and a court,
//! follows the stick held at the press, clears a pit in a brawl, plays the jump clip in a fight
//! too, and glows while it is untouchable (not in a bare leap).

use super::save::Stage;
use super::*;
use crate::input::PadState;
use crate::logic::sins::Sin;
use crate::logic::types::{Dir, Tile, P, V};

/// A fresh game at `stage`, play under way, the top view (the stick the world's way round), free
/// movement.
fn place(stage: Stage) -> Game {
    let mut g = super::bot::fresh_game(1);
    g.map_jump(stage);
    g.view.camera_mode = crate::render::view::CameraMode::Top;
    g.enter(Screen::Playing);
    g.hero.free_move = true;
    g.skip_lesson();
    g
}

/// The tile nearest his start with `n` tiles of level open floor beyond it the way `d` (and one
/// behind).
fn runway(g: &Game, d: (i32, i32), n: i32) -> P {
    let gr = &g.level.grid;
    let start = g.hero.occupied();
    let mut best: Option<(i32, P)> = None;
    for y in 1..gr.height - 1 {
        for x in 1..gr.width - 1 {
            let h = gr.elevation(x, y);
            let ok = (-1..=n).all(|k| {
                let (tx, ty) = (x + d.0 * k, y + d.1 * k);
                gr.get(tx, ty) == Tile::Open && gr.elevation(tx, ty) == h && !gr.is_ramp(tx, ty)
            });
            let far = (x - start.x).abs() + (y - start.y).abs();
            if ok && best.is_none_or(|b| far < b.0) {
                best = Some((far, P::new(x, y)));
            }
        }
    }
    best.expect("a runway").1
}

/// How a leap went: how far and which way, the most of the glow on him in the air, the clip his
/// leap was posed by, and whether it was a bare leap.
struct Leapt {
    went: V,
    glow: f32,
    clip: Option<&'static str>,
    bare: bool,
}

/// A press of the jump at `at`, facing down, with `stick` held from a few frames before (and after
/// the press if `hold_after`); with `attack_lead`, an attack pressed that many updates before it.
fn leap_at(g: &mut Game, at: P, stick: (f32, f32), hold_after: bool, attack_lead: Option<usize>) -> Leapt {
    g.hero.m.teleport(at);
    g.hero.face(Dir::Down);
    let pad = |stick: (f32, f32), jump: bool, attack: bool| {
        let mut p = PadState { left: stick, ..Default::default() };
        if jump {
            p.actions.insert(Action::Jump);
        }
        if attack {
            p.actions.insert(Action::Attack);
        }
        p
    };
    for _ in 0..3 {
        g.input.set_pad(Some(pad(stick, false, false)));
        g.update(crate::app::STEP);
    }
    if let Some(n) = attack_lead {
        g.input.set_pad(Some(pad(stick, false, true)));
        g.update(crate::app::STEP);
        for _ in 0..n {
            g.input.set_pad(Some(pad(stick, false, false)));
            g.update(crate::app::STEP);
        }
    }
    let from = g.hero.pos();
    g.input.set_pad(Some(pad(stick, true, false)));
    g.update(crate::app::STEP);
    let after = if hold_after { stick } else { (0.0, 0.0) };
    let (mut flew, mut glow, mut bare) = (false, 0.0f32, false);
    for _ in 0..200 {
        g.input.set_pad(Some(pad(after, false, false)));
        g.update(crate::app::STEP);
        if g.hero.airborne() {
            flew = true;
            glow = glow.max(g.view.leap_glow_state().0);
            bare |= g.brawl.as_ref().is_some_and(|b| b.hero.bare_leap());
        } else if flew {
            break;
        }
    }
    g.input.set_pad(None);
    assert!(flew, "he leapt");
    let to = g.hero.pos();
    Leapt { went: V::new(to.x - from.x, to.y - from.y), glow, clip: g.view.hero_leap_clip(), bare }
}

/// No duelist about (a quiet place to leap in).
fn quiet(g: &mut Game) {
    g.enemies.retain(|e| e.duel.is_none());
}

fn len(v: V) -> f64 {
    (v.x * v.x + v.y * v.y).sqrt()
}

#[test]
fn the_leap_goes_as_far_in_the_garden_the_meadow_a_realm_and_a_court() {
    for (name, stage, brawl) in [
        ("the Garden", Stage::Dungeon(0), true),
        ("the meadow", Stage::Arena(0), false),
        ("the Vaults", Stage::Realm(Sin::Greed), true),
        ("the Hall of Mirrors", Stage::Realm(Sin::Pride), true),
        ("Mammon's court", Stage::Court(Sin::Greed), true),
    ] {
        for (d, stick) in [((1, 0), (1.0, 0.0)), ((0, -1), (0.0, -1.0))] {
            let mut g = place(stage);
            assert_eq!(g.brawling(), brawl, "{name}: a fight by frame data");
            quiet(&mut g);
            let at = runway(&g, d, 3);
            let l = leap_at(&mut g, at, stick, false, None);
            let along = l.went.x * d.0 as f64 + l.went.y * d.1 as f64;
            assert!(
                (along - 2.0).abs() < 0.05 && (len(l.went) - along).abs() < 0.01,
                "{name}: two tiles the stick's way, went {:?}",
                l.went
            );
            if brawl {
                assert!(!l.bare && l.glow > 0.5, "{name}: an evade, glowing while untouchable ({})", l.glow);
            } else {
                assert_eq!(l.glow, 0.0, "{name}: no fight, nothing to be untouchable from");
            }
        }
    }
}

#[test]
fn the_leap_follows_the_stick_held_at_the_press() {
    // Facing down, the stick up and to the right: that way, two tiles, and he turns to it.
    let mut g = place(Stage::Realm(Sin::Greed));
    quiet(&mut g);
    let at = runway(&g, (1, 0), 3);
    let l = leap_at(&mut g, at, (0.6, -0.8), false, None);
    let want = V::new(1.2, -1.6);
    assert!((l.went.x - want.x).abs() < 0.08 && (l.went.y - want.y).abs() < 0.08, "went {:?}", l.went);
    assert!((g.hero.heading - (-0.8f64).atan2(0.6)).abs() < 0.05, "turned to it: {}", g.hero.heading);
    // No stick: the way he faces.
    let mut g = place(Stage::Realm(Sin::Greed));
    quiet(&mut g);
    let at = runway(&g, (0, 1), 3);
    let l = leap_at(&mut g, at, (0.0, 0.0), false, None);
    assert!((l.went.y - 2.0).abs() < 0.05 && l.went.x.abs() < 0.01, "went {:?}", l.went);
    // Pressed in his swing's active frames (two updates a frame), the stick let go at once: the leap waits in the buffer
    // and comes out as they end (bare), still the way the stick pointed at the press.
    let mut g = place(Stage::Realm(Sin::Greed));
    quiet(&mut g);
    let at = runway(&g, (1, 0), 3);
    let l1 = crate::logic::combat::MoveId::L1.data();
    let l = leap_at(&mut g, at, (1.0, 0.0), false, Some((l1.startup as usize + 1) * 2));
    assert!(l.bare, "out of the recovery: a bare leap");
    assert!((l.went.x - 2.0).abs() < 0.08 && l.went.y.abs() < 0.05, "went {:?}", l.went);
    assert_eq!(l.glow, 0.0, "a bare leap is not untouchable, and does not glow");
}

#[test]
fn a_leap_clears_a_pit_in_a_brawl() {
    let mut g = place(Stage::Realm(Sin::Greed));
    quiet(&mut g);
    let at = runway(&g, (1, 0), 3);
    g.level.grid.set(at.x + 1, at.y, Tile::Pit);
    let l = leap_at(&mut g, at, (1.0, 0.0), false, None);
    assert!(g.hero.alive);
    assert!((l.went.x - 2.0).abs() < 0.05, "over the pit: {:?}", l.went);
    assert_eq!(g.hero.occupied(), P::new(at.x + 2, at.y));
}

#[test]
fn the_hero_leaps_by_the_jump_clip_in_a_fight() {
    let mut g = place(Stage::Realm(Sin::Greed));
    quiet(&mut g);
    let at = runway(&g, (1, 0), 3);
    let l = leap_at(&mut g, at, (1.0, 0.0), false, None);
    assert!(g.view.posed_by_clips(), "his clips pose him");
    assert_eq!(l.clip, Some("jump"), "the evade is the jump, not the roll");
}
