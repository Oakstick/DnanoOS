//! The QA pass's soak (QA.md): every stop on the LEVELS map entered as a player enters it (the
//! map opened from the title, the stop picked), then played for a while by a bot and by a monkey
//! that presses whatever comes to hand (the keyboard, the mouse on the menus, the controller's
//! buttons), the menus and the pages opened and shut over every moment of it. Each stop is played
//! in its own game, a panic caught and told, and what a reviewer would call broken is watched for:
//!
//! * a panic, and where it came;
//! * a screen held past its time (a countdown, a fanfare, a death, a choice) with keys pressed;
//! * the hero standing in rock, or off the level, or at a position that is not a number;
//! * a foe standing in rock (stuck in a wall) for seconds on end;
//! * the lists that grow with play (floaters, projectiles, foes) past what any fight needs.
//!
//! `--monkey SEED SECS [cheats]` plays every stop SECS seconds (the testing keys pressed too with
//! `cheats`), and prints the findings, one line each, and each stop's figures.

use super::bot::{Bot, Skill};
use super::map::map_stops;
use super::{Game, Screen};
use crate::input::{PadButton, PadState};
use crate::logic::rng::Rng;
use crate::logic::types::{Tile, P};
use std::fmt::Write;
use winit::keyboard::KeyCode;

/// The keys a player presses: moving, fighting, the menus and the pages.
const KEYS: [KeyCode; 24] = [
    KeyCode::Escape,
    KeyCode::Enter,
    KeyCode::Space,
    KeyCode::KeyE,
    KeyCode::KeyB,
    KeyCode::KeyH,
    KeyCode::KeyM,
    KeyCode::Tab,
    KeyCode::KeyQ,
    KeyCode::KeyR,
    KeyCode::KeyC,
    KeyCode::KeyF,
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::ArrowUp,
    KeyCode::ArrowDown,
    KeyCode::ArrowLeft,
    KeyCode::ArrowRight,
    KeyCode::KeyW,
    KeyCode::KeyA,
    KeyCode::KeyS,
    KeyCode::KeyD,
    KeyCode::ShiftLeft,
    KeyCode::Backspace,
];

/// The testing keys (shipped in every build): F1 the Cyclops, F2 the gate, F4 the grid, F5 god
/// mode, F6 gold, F7 and F9 the ring's bouts, F10 a rival, 0 the old man, F12 the map.
const CHEATS: [KeyCode; 10] = [
    KeyCode::F1,
    KeyCode::F2,
    KeyCode::F4,
    KeyCode::F5,
    KeyCode::F6,
    KeyCode::F7,
    KeyCode::F9,
    KeyCode::F10,
    KeyCode::Digit0,
    KeyCode::F12,
];

/// Seconds a screen that ends by itself, or on a press, may hold before it counts as stuck.
const SCREEN_HOLD: f64 = 120.0;
/// Seconds in rock before the hero, or a foe, counts as stuck in a wall.
const IN_ROCK: f64 = 3.0;

/// One thing found, at a stop, so many seconds into it.
#[derive(Clone, Debug)]
pub struct Finding {
    pub stop: String,
    pub t: f64,
    pub kind: &'static str,
    pub what: String,
}

/// How one stop went: its findings, and the most the growing lists held.
#[derive(Clone, Debug, Default)]
pub struct StopReport {
    pub stop: String,
    pub findings: Vec<Finding>,
    pub screens: Vec<(Screen, f64)>,
    pub most: (usize, usize, usize, usize),
    pub panicked: bool,
}

/// The monkey's hands: keys held until a time, and what it does next.
struct Hands {
    rng: Rng,
    held: Vec<(KeyCode, f64)>,
    cheats: bool,
}

impl Hands {
    fn chance(&mut self, p: f64) -> bool {
        self.rng.next() < p
    }

    fn pick<T: Copy>(&mut self, list: &[T]) -> T {
        list[((self.rng.next() * list.len() as f64) as usize).min(list.len() - 1)]
    }

    /// One update's presses: a key now and then (held a moment), a click on the screen, a
    /// controller button.
    fn press(&mut self, g: &mut Game, t: f64) {
        self.held.retain(|&(k, until)| {
            if t >= until {
                g.input.key_up(k);
                false
            } else {
                true
            }
        });
        if self.chance(0.08) {
            let k = self.pick(&KEYS);
            if !self.held.iter().any(|h| h.0 == k) {
                g.input.key_down(k, false);
                let hold = if matches!(k, KeyCode::KeyW | KeyCode::KeyA | KeyCode::KeyS | KeyCode::KeyD) { 0.6 } else { 0.05 };
                self.held.push((k, t + hold));
            }
        }
        if self.cheats && self.chance(0.004) {
            let k = self.pick(&CHEATS);
            g.input.key_down(k, false);
            g.input.key_up(k);
        }
        if self.chance(0.01) {
            let (w, h) = g.viewport;
            let (x, y) = ((self.rng.next() * w as f64) as f32, (self.rng.next() * h as f64) as f32);
            g.input.cursor_moved_to(x, y);
            g.input.mouse_button(winit::event::MouseButton::Left, true);
            g.input.mouse_button(winit::event::MouseButton::Left, false);
        }
        if self.chance(0.01) {
            let b = self.pick(&PadButton::ALL);
            g.input.set_pad(Some(PadState {
                buttons: [b].into_iter().collect(),
                kind: Some(crate::input::Device::Xbox),
                ..Default::default()
            }));
        }
    }

    fn let_go(&mut self, g: &mut Game) {
        for (k, _) in self.held.drain(..) {
            g.input.key_up(k);
        }
    }
}

/// Into stop `index` of the LEVELS map as a player goes: the map opened from the title, the stop
/// highlighted, Enter.
fn enter_stop(g: &mut Game, index: usize) {
    if g.screen != Screen::Title {
        g.enter(Screen::Title);
    }
    g.open_map();
    g.map_index = index;
    g.input.inject(crate::input::Action::Confirm);
    g.update(crate::app::STEP);
}

/// Stop `index` played for `secs` seconds by the bot and the monkey in turns (ten seconds each,
/// and ten idle), from a game of its own seeded `seed`.
pub fn soak_stop(index: usize, seed: u32, secs: f64, cheats: bool) -> StopReport {
    let stops = map_stops();
    let label = stops[index].label.clone();
    let mut rep = StopReport { stop: label.clone(), ..Default::default() };
    let mut g = Game::new();
    g.reseed(seed);
    g.tour_pending = false;
    let step = crate::app::STEP;
    let mut bot = Bot::new(Skill::Expert, seed);
    let mut hands = Hands { rng: Rng::new(seed.wrapping_mul(7919).wrapping_add(index as u32)), held: Vec::new(), cheats };
    let found = |rep: &mut StopReport, t: f64, kind: &'static str, what: String| {
        if rep.findings.iter().filter(|f| f.kind == kind).count() < 4 {
            rep.findings.push(Finding { stop: label.clone(), t, kind, what });
        }
    };
    let entered = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| enter_stop(&mut g, index)));
    if let Err(e) = entered {
        found(&mut rep, 0.0, "panic", format!("entering: {}", panic_text(&e)));
        rep.panicked = true;
        return rep;
    }
    let (mut t, mut on_screen, mut screen) = (0.0, 0.0, g.screen);
    let mut hero_rock = 0.0;
    let mut foe_rock: std::collections::HashMap<usize, f64> = Default::default();
    rep.screens.push((screen, 0.0));
    while t < secs {
        let phase = ((t / 10.0) as usize) % 3;
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            match phase {
                // The bot plays (a press of Enter now and then off the field, to go on).
                0 => {
                    let pad = if g.screen == Screen::Playing { bot.act(&g) } else { PadState::default() };
                    g.input.set_pad(Some(pad));
                    if g.screen != Screen::Playing && ((t / step) as u64).is_multiple_of(40) {
                        g.input.inject(crate::input::Action::Confirm);
                    }
                }
                // The monkey.
                1 => {
                    g.input.set_pad(None);
                    hands.press(&mut g, t);
                }
                // Hands off.
                _ => {
                    hands.let_go(&mut g);
                    g.input.set_pad(None);
                }
            }
            g.update(step);
        }));
        if let Err(e) = r {
            found(&mut rep, t, "panic", format!("on {:?}: {}", g.screen, panic_text(&e)));
            rep.panicked = true;
            return rep;
        }
        t += step;
        // (The monkey quits now and then: the game goes on from the title, back into the stop.)
        if g.quit_requested {
            g.quit_requested = false;
        }
        if g.screen == Screen::Title || g.screen == Screen::GameOver && phase != 1 {
            hands.let_go(&mut g);
            g.input.set_pad(None);
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| enter_stop(&mut g, index)));
            if let Err(e) = r {
                found(&mut rep, t, "panic", format!("back in: {}", panic_text(&e)));
                rep.panicked = true;
                return rep;
            }
        }
        if g.screen != screen {
            screen = g.screen;
            on_screen = 0.0;
            if rep.screens.len() < 60 {
                rep.screens.push((screen, t));
            }
        } else {
            on_screen += step;
        }
        let holds = !matches!(screen, Screen::Playing | Screen::Paused | Screen::Title | Screen::Shop | Screen::Story | Screen::Select);
        if holds && on_screen > SCREEN_HOLD && on_screen - step <= SCREEN_HOLD {
            found(&mut rep, t, "stuck screen", format!("{screen:?} for {SCREEN_HOLD:.0} s"));
        }
        if g.screen == Screen::Playing {
            let hp = g.hero.pos();
            let grid = &g.level.grid;
            if !hp.x.is_finite() || !hp.y.is_finite() {
                found(&mut rep, t, "hero nan", format!("{hp:?}"));
            } else {
                let tile = P::new(hp.x.round() as i32, hp.y.round() as i32);
                if !grid.in_bounds(tile.x, tile.y) {
                    found(&mut rep, t, "hero off level", format!("at ({:.1}, {:.1})", hp.x, hp.y));
                }
                if grid.get(tile.x, tile.y) == Tile::Rock && !g.hero.airborne() {
                    hero_rock += step;
                    if hero_rock > IN_ROCK && hero_rock - step <= IN_ROCK {
                        found(&mut rep, t, "hero in rock", format!("at ({:.2}, {:.2})", hp.x, hp.y));
                    }
                } else {
                    hero_rock = 0.0;
                }
            }
            for (i, e) in g.enemies.iter().enumerate() {
                let (pos, alive) = match e.duel.as_deref() {
                    Some(d) => (d.pos, d.alive()),
                    None => (e.pos(), e.is_targetable() || e.is_ally()),
                };
                let tile = P::new(pos.x.round() as i32, pos.y.round() as i32);
                let rock = alive && pos.x.is_finite() && grid.get(tile.x, tile.y) == Tile::Rock;
                let k = foe_rock.entry(i).or_default();
                if rock {
                    *k += step;
                    if *k > IN_ROCK && *k - step <= IN_ROCK {
                        let kind = e
                            .duel
                            .as_deref()
                            .map(|d| {
                                format!("{:?} {:?} den {:?} puppet {}", d.breed.kind, d.f.state, d.den.as_ref().map(|n| n.home), d.puppet)
                            })
                            .unwrap_or_else(|| format!("{:?}", e.kind()));
                        let near: String = (-1..=1)
                            .map(|dy| {
                                (-1..=1)
                                    .map(|dx| if grid.get(tile.x + dx, tile.y + dy) == Tile::Rock { '#' } else { '.' })
                                    .collect::<String>()
                            })
                            .collect::<Vec<_>>()
                            .join("/");
                        let what = format!(
                            "{kind} at ({:.2}, {:.2}) [{near}] hero at ({:.1}, {:.1})",
                            pos.x,
                            pos.y,
                            g.hero.pos().x,
                            g.hero.pos().y
                        );
                        found(&mut rep, t, "foe in rock", what);
                    }
                } else {
                    *k = 0.0;
                }
                if alive && !pos.x.is_finite() {
                    found(&mut rep, t, "foe nan", format!("{:?}", e.kind()));
                }
            }
        }
        let most = &mut rep.most;
        most.0 = most.0.max(g.floaters.len());
        most.1 = most.1.max(g.projectiles.len());
        most.2 = most.2.max(g.enemies.len());
        most.3 = most.3.max(g.fires.len());
    }
    rep
}

fn panic_text(e: &Box<dyn std::any::Any + Send>) -> String {
    e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_else(|| "?".into())
}

/// Every stop, `secs` seconds each: the findings first, then each stop's figures.
pub fn monkey(seed: u32, secs: f64, cheats: bool, only: Option<usize>) -> String {
    let n = map_stops().len();
    let reports: Vec<StopReport> =
        (0..n).filter(|i| only.is_none_or(|o| o == *i)).map(|i| soak_stop(i, seed.wrapping_add(i as u32 * 101), secs, cheats)).collect();
    let mut out = String::new();
    let all: Vec<&Finding> = reports.iter().flat_map(|r| r.findings.iter()).collect();
    let _ = writeln!(out, "THE MONKEY seed {seed}, {secs:.0} s a stop{}: {} findings", if cheats { ", cheats" } else { "" }, all.len());
    for f in &all {
        let _ = writeln!(out, "  [{}] {} at {:.1} s: {}", f.kind, f.stop, f.t, f.what);
    }
    for r in &reports {
        let path: Vec<String> = r.screens.iter().take(12).map(|(s, t)| format!("{s:?}@{t:.0}")).collect();
        let _ = writeln!(
            out,
            "{:<44} most floaters {:>3} shots {:>3} foes {:>3} fires {:>3}{} | {}",
            r.stop,
            r.most.0,
            r.most.1,
            r.most.2,
            r.most.3,
            if r.panicked { " PANIC" } else { "" },
            path.join(" ")
        );
    }
    out
}
