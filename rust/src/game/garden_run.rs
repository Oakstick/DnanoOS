//! The first hour's harness (B01, B57): the Garden of Krampus and the Cyclops' meadow played by a
//! bot, as `--run` plays the realms. `--garden SKILL SEED` walks the Garden key by key to its north
//! gate; `--meadow SKILL SEED` fights the first Cyclops on the lives a player brings.
//!
//! Where the fight goes by frame data (the Garden since B01) the brawl's own bot fights (`Bot`),
//! and between fights it walks to the nearest golden key still lying, then to the gate (`garden_goal`).
//! Where the old rules hold (the Cyclops' meadow; the Garden before B01) a plainer player stands in
//! (`OldBot`): it walks to its goal and swings at whatever comes within reach; the adept and the
//! expert also leap out of a foe's way when their swing is not ready, step off the line of what the
//! Cyclops throws, and drink from the belt when low. The novice only walks and swings.

use super::bot::{fresh_game, Bot, Skill};
use super::save::Stage;
use super::{Game, Screen};
use crate::input::{Action, PadState};
use crate::logic::combat::math;
use crate::logic::rng::Rng;
use crate::logic::types::{Tile, P, V};

/// How a walk through the Garden went.
#[derive(Clone, Copy, Debug, Default)]
pub struct GardenRun {
    /// Through the north gate (every key taken, the gate reached).
    pub done: bool,
    pub game_over: bool,
    pub secs: f64,
    pub keys: u32,
    pub keys_total: u32,
    pub deaths: u32,
    pub kills: u32,
    /// Health lost, all told.
    pub taken: f64,
    /// Foes standing at the end.
    pub left: u32,
    /// The most foes awake and hunting him at once, and the most strikes reaching him within a
    /// reaction window of each other (the frame data's overlaps; none under the old rules).
    pub most_hunting: u32,
    pub overlaps: u32,
    /// Foes' strikes that reached him (however met), and those that hurt him.
    pub landings: u32,
    pub hurt: u32,
}

/// How a fight with the Cyclops went.
#[derive(Clone, Copy, Debug, Default)]
pub struct MeadowRun {
    pub slain: bool,
    pub game_over: bool,
    pub secs: f64,
    pub deaths: u32,
    pub taken: f64,
    /// The Cyclops' health left (of its whole), and the cover still standing at the end (of all).
    pub boss_hp: (i32, i32),
    pub cover: (u32, u32),
}

/// The golden key nearest him (by the way he must walk) still lying in the Garden, or, with every
/// key taken, the gate.
pub fn garden_goal(g: &Game) -> Option<V> {
    let grid = &g.level.grid;
    let hp = g.hero.pos();
    let steps = super::garden_fight::walk_steps(grid, P::new(hp.x.round() as i32, hp.y.round() as i32));
    let key = grid
        .cells()
        .filter(|&(_, _, t)| t == Tile::Nodule)
        .map(|(x, y, _)| P::new(x, y))
        .min_by_key(|p| (steps.get(p).copied().unwrap_or(u32::MAX), p.y, p.x));
    key.map(|p| p.v()).or_else(|| g.gate.map(|p| p.v()))
}

/// A player of the old contact rules (see the module's notes).
pub struct OldBot {
    pub skill: Skill,
    walker: Bot,
    rng: Rng,
    attack_down: bool,
    sip: u16,
}

impl OldBot {
    pub fn new(skill: Skill, seed: u32) -> Self {
        Self { skill, walker: Bot::new(skill, seed), rng: Rng::new(seed ^ 0x5eed), attack_down: false, sip: 0 }
    }

    fn mash(&mut self, pad: &mut PadState) {
        self.attack_down = !self.attack_down;
        if self.attack_down {
            pad.actions.insert(Action::Attack);
        }
    }

    /// The Cyclops' shots on their way to him (within `ahead` seconds): the way across the soonest's line.
    fn sidestep(g: &Game, hp: V, ahead: f64) -> Option<V> {
        let mut best: Option<(f64, V)> = None;
        for s in &g.boss_shots {
            let speed = math::len(s.vel);
            if speed < 0.1 {
                continue;
            }
            let dir = math::scale(s.vel, 1.0 / speed);
            let to = math::sub(hp, s.pos);
            let along = math::dot(dir, to);
            let across = math::len(math::sub(to, math::scale(dir, along)));
            let t = along / speed;
            if along > 0.0 && across < s.kind.radius() + 0.55 && t < ahead && best.is_none_or(|b| t < b.0) {
                let side = V::new(-dir.y, dir.x);
                let away = if math::dot(side, to) >= 0.0 { side } else { math::scale(side, -1.0) };
                best = Some((t, away));
            }
        }
        best.map(|b| b.1)
    }

    /// This update's stick and buttons.
    pub fn act(&mut self, g: &Game) -> PadState {
        let mut pad = PadState::default();
        let hp = g.hero.pos();
        let careful = self.skill != Skill::Novice;
        self.sip = self.sip.saturating_sub(1);
        if careful && self.sip == 0 && !g.belt.is_empty() && (g.hero.hp as f64) < g.hero.max_hp as f64 * 0.4 {
            self.sip = 240;
            pad.actions.insert(Action::Drink);
        }
        let set = |pad: &mut PadState, d: V| pad.left = (d.x as f32, d.y as f32);
        // The Cyclops' meadow.
        if let Some(b) = g.boss.as_ref() {
            if !b.alive() {
                return pad;
            }
            let ahead = if self.skill == Skill::Expert { 0.5 } else { 0.35 };
            if careful {
                if let Some(away) = Self::sidestep(g, hp, ahead) {
                    set(&mut pad, away);
                    return pad;
                }
            }
            let to = math::sub(b.pos, hp);
            let d = math::len(to);
            let dir = if d > 0.05 { math::norm(to) } else { V::new(0.0, -1.0) };
            // A blade's reach and the eye's body; touching it hurts while it is not dazed.
            let reach = crate::logic::boss::BODY_RADIUS + 1.1;
            let harmful = b.harmful() && b.hover < 1.3;
            if careful && harmful && d < crate::logic::boss::CONTACT_RADIUS + 0.3 {
                set(&mut pad, math::scale(dir, -1.0));
                return pad;
            }
            let stand = if careful && harmful { 1.6 } else { reach - 0.4 };
            if d > stand {
                let w = self.walker.way(g, hp, b.pos);
                set(&mut pad, w);
            } else {
                set(&mut pad, math::scale(dir, 0.3));
            }
            if d <= reach {
                self.mash(&mut pad);
            }
            return pad;
        }
        // The Garden: the nearest foe in reach is struck; the careful leap out of its way.
        let foe = g
            .enemies
            .iter()
            .filter(|e| e.is_dangerous() && e.duel.is_none())
            .map(|e| (math::len(math::sub(e.pos(), hp)), e.pos()))
            .min_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((d, at)) = foe.filter(|f| f.0 < 1.8) {
            let to = math::norm(math::sub(at, hp));
            if !careful {
                set(&mut pad, to);
                self.mash(&mut pad);
                return pad;
            }
            // The careful swing only when the blow is ready and the foe in reach, and keep out of
            // its claws meanwhile: back off (the expert leaps clear).
            let ready = g.hero.attack_cooldown <= 0.0 && g.hero.stamina >= crate::logic::progression::STAMINA_PER_BLOW;
            if ready && d < 1.45 {
                set(&mut pad, math::scale(to, 0.3));
                self.mash(&mut pad);
                return pad;
            }
            if d < 1.1 {
                let away = math::scale(to, -1.0);
                set(&mut pad, away);
                if self.skill == Skill::Expert && d < 0.8 && !g.hero.airborne() && self.rng.next() < 0.5 {
                    pad.actions.insert(Action::Jump);
                }
                return pad;
            }
        }
        if let Some(goal) = garden_goal(g) {
            let w = self.walker.way(g, hp, goal);
            set(&mut pad, w);
        }
        pad
    }
}

/// The Garden ready for a bot: a fresh game in the Garden, play under way, the top view (its stick
/// the world's way round), free movement.
fn garden_game(seed: u32) -> Game {
    let mut g = fresh_game(seed);
    g.map_jump(Stage::Dungeon(0));
    g.view.camera_mode = crate::render::view::CameraMode::Top;
    g.enter(Screen::Playing);
    g.hero.free_move = true;
    g.skip_lesson();
    g
}

/// A bot walks the Garden to its north gate (or its lives run out, or `limit` seconds pass).
pub fn garden_run(skill: Skill, seed: u32, limit: f64) -> GardenRun {
    let mut g = garden_game(seed);
    let kit = g.hero_kit();
    if let Some(b) = g.brawl.as_mut() {
        b.reset(kit);
    }
    let mut bot = Bot::new(skill, seed);
    let mut old = OldBot::new(skill, seed);
    let lives = g.score.lives;
    let step = crate::app::STEP;
    let mut out = GardenRun { keys_total: g.keys_total, ..GardenRun::default() };
    let mut t = 0.0;
    let mut hp = g.hero.hp;
    let mut seen_dead: Vec<usize> = Vec::new();
    let trace = std::env::var_os("VH_TRACE_GARDEN").is_some();
    // (`VH_TRACE_GARDEN=FROM:EVERY`: from second FROM, every EVERY steps.)
    let (from, every) = std::env::var("VH_TRACE_GARDEN")
        .ok()
        .and_then(|v| v.split_once(':').map(|(a, b)| (a.parse().unwrap_or(0.0), b.parse().unwrap_or(300))))
        .unwrap_or((0.0, 300u64));
    if trace {
        // The ground as the bots read it: rock #, pit ~, nest N, key K, a ramp /, else its height.
        let gr = &g.level.grid;
        for y in 0..gr.height {
            let row: String = (0..gr.width)
                .map(|x| match gr.get(x, y) {
                    Tile::Rock => '#',
                    Tile::Pit => '~',
                    Tile::Nest => 'N',
                    Tile::Nodule => 'K',
                    _ if gr.is_ramp(x, y) => '/',
                    _ => char::from_digit(gr.elevation(x, y).clamp(0, 9) as u32, 10).unwrap_or('?'),
                })
                .collect();
            eprintln!("{y:>2} {row}");
        }
        let steps = super::garden_fight::walk_steps(gr, g.hero.occupied());
        for (x, y, t) in gr.cells() {
            if t == Tile::Nodule {
                eprintln!("key {x},{y}: {:?} steps", steps.get(&P::new(x, y)));
            }
        }
        eprintln!("gate {:?}: {:?}", g.gate, g.gate.and_then(|p| steps.get(&p)));
        for e in &g.enemies {
            if let Some(d) = e.duel.as_deref() {
                eprintln!("den foe {:?} at {:.0},{:.0}", d.breed.kind, d.pos.x, d.pos.y);
            }
        }
    }
    while t < limit {
        let playing = g.screen == Screen::Playing;
        let pad = if !playing {
            PadState::default()
        } else if g.brawling() {
            bot.act(&g)
        } else {
            old.act(&g)
        };
        let stick = pad.left;
        g.input.set_pad(Some(pad));
        let before: Vec<bool> = g.enemies.iter().map(|e| e.is_dangerous()).collect();
        g.update(step);
        t += step;
        // (Under the old rules a kill is a foe that was dangerous and is not: its number kept.)
        if !g.brawling() && g.enemies.len() == before.len() {
            for (i, e) in g.enemies.iter().enumerate() {
                if before[i] && !e.is_dangerous() && e.duel.is_none() && !seen_dead.contains(&i) {
                    out.kills += 1;
                    seen_dead.push(i);
                }
                if e.is_dangerous() {
                    seen_dead.retain(|&k| k != i);
                }
            }
        }
        if g.hero.hp < hp {
            out.taken += (hp - g.hero.hp) as f64;
        }
        hp = g.hero.hp;
        let hunting = g.enemies.iter().filter(|e| e.duel.as_ref().is_some_and(|d| d.engaged())).count() as u32;
        if trace && ((t / step) as u64).is_multiple_of(every) && t >= from {
            let foes: Vec<String> = g
                .enemies
                .iter()
                .filter_map(|e| e.duel.as_deref())
                .filter(|d| d.engaged())
                .map(|d| format!("{:?}@{:.1},{:.1}", d.breed.kind, d.pos.x, d.pos.y))
                .collect();
            eprintln!(
                "t {t:.0} stick {:.2},{:.2} at {:.1},{:.1} goal {:?} keys {} hp {} hunting {hunting} {:?} {foes:?}",
                stick.0,
                stick.1,
                g.hero.pos().x,
                g.hero.pos().y,
                garden_goal(&g),
                g.keys_got,
                g.hero.hp,
                g.screen
            );
        }
        out.most_hunting = out.most_hunting.max(hunting);
        match g.screen {
            Screen::GameOver => {
                out.game_over = true;
                break;
            }
            Screen::Ready => g.enter(Screen::Playing),
            Screen::Cleared => {
                out.done = true;
                break;
            }
            _ => {}
        }
    }
    out.secs = t;
    out.keys = g.keys_got;
    out.deaths = lives.saturating_sub(g.score.lives) + u32::from(out.game_over);
    if let Some(b) = g.brawl.as_ref() {
        out.kills = b.stats.kills;
        out.overlaps = b.stats.overlaps;
        out.landings = b.stats.landings;
        out.hurt = b.stats.hurt;
        out.taken = b.stats.taken.max(out.taken);
    }
    out.left = g.enemies.iter().filter(|e| e.is_dangerous() || e.duel.as_ref().is_some_and(|d| d.counts())).count() as u32;
    out
}

/// A bot fights the first Cyclops (the lives a fresh adventure brings, the kit it starts with).
pub fn meadow_run(skill: Skill, seed: u32, limit: f64) -> MeadowRun {
    let mut g = fresh_game(seed);
    g.map_jump(Stage::Arena(0));
    g.view.camera_mode = crate::render::view::CameraMode::Top;
    g.enter(Screen::Playing);
    g.hero.free_move = true;
    // (As the countdown would: down out of the sky it comes.)
    let wake = |g: &mut Game| {
        if let Some(b) = g.boss.as_mut() {
            b.wake();
        }
    };
    wake(&mut g);
    let mut old = OldBot::new(skill, seed);
    let lives = g.score.lives;
    let step = crate::app::STEP;
    let mut out = MeadowRun { cover: (0, g.meadow_cover_standing()), ..MeadowRun::default() };
    let mut t = 0.0;
    let mut hp = g.hero.hp;
    while t < limit {
        let pad = if g.screen == Screen::Playing { old.act(&g) } else { PadState::default() };
        g.input.set_pad(Some(pad));
        g.update(step);
        t += step;
        if g.hero.hp < hp {
            out.taken += (hp - g.hero.hp) as f64;
        }
        hp = g.hero.hp;
        match g.screen {
            Screen::GameOver => {
                out.game_over = true;
                break;
            }
            Screen::Ready => {
                g.enter(Screen::Playing);
                wake(&mut g);
            }
            _ => {}
        }
        if g.boss.as_ref().is_some_and(|b| !b.alive()) {
            out.slain = true;
            break;
        }
    }
    out.secs = t;
    out.deaths = lives.saturating_sub(g.score.lives) + u32::from(out.game_over);
    out.boss_hp = g.boss.as_ref().map(|b| (b.hp, b.max_hp)).unwrap_or((0, 0));
    out.cover.0 = g.meadow_cover_standing();
    out
}

/// `--garden` and `--meadow`'s tables: one line per seed, and the means.
pub fn first_hour_report(meadow: bool, skill: Skill, seeds: &[u32]) -> String {
    let mut s = String::new();
    use std::fmt::Write;
    if meadow {
        let mut won = 0;
        let mut secs = 0.0;
        for &seed in seeds {
            let r = meadow_run(skill, seed, 600.0);
            won += r.slain as u32;
            secs += r.secs;
            let _ = writeln!(
                s,
                "meadow {} seed {seed:>2}: {} {:>5.0} s  deaths {}  taken {:>4.0}  cyclops {}/{}  cover {}/{}",
                skill.name(),
                if r.slain {
                    "SLAIN"
                } else if r.game_over {
                    "LOST "
                } else {
                    "TIME "
                },
                r.secs,
                r.deaths,
                r.taken,
                r.boss_hp.0,
                r.boss_hp.1,
                r.cover.0,
                r.cover.1
            );
        }
        let _ = writeln!(s, "meadow {}: {won}/{} slain, mean {:.0} s", skill.name(), seeds.len(), secs / seeds.len().max(1) as f64);
    } else {
        let mut won = 0;
        let mut secs = 0.0;
        for &seed in seeds {
            let r = garden_run(skill, seed, 1200.0);
            won += r.done as u32;
            secs += r.secs;
            let _ = writeln!(
                s,
                "garden {} seed {seed:>2}: {} {:>5.0} s  keys {}/{}  deaths {}  kills {:>3}  taken {:>5.0}  left {:>2}  most hunting {:>2}  overlaps {}  landings {:>3}  hurt {:>3}",
                skill.name(),
                if r.done { "DONE " } else if r.game_over { "LOST " } else { "TIME " },
                r.secs,
                r.keys,
                r.keys_total,
                r.deaths,
                r.kills,
                r.taken,
                r.left,
                r.most_hunting,
                r.overlaps,
                r.landings,
                r.hurt
            );
        }
        let _ = writeln!(s, "garden {}: {won}/{} through, mean {:.0} s", skill.name(), seeds.len(), secs / seeds.len().max(1) as f64);
    }
    s
}
