//! The scripted driver that `--capture` and `--perf` share: it plays a script (`seconds:command`
//! pairs, `capture::parse_script`) on the game's clock, lets a bot take the pad from a `bot:K` on,
//! steps the game one fixed simulation step at a time, and keeps count of what the hero did, so a
//! benchmark can say what its scene really played.
//!
//! One step is what the game's frame loop does once per `STEP` of time owed (`app.rs`): the
//! script's commands that are due fire, the bot (if one plays) sets the pad, and the game updates.

use crate::app::STEP;
use crate::capture::{exec, Cmd};
use crate::game::bot::Bot;
use crate::game::Game;
use crate::logic::combat::State;

/// What happened while the driver stepped the game, counted from the last `Driver::recount`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Deeds {
    /// Simulation steps taken.
    pub steps: u32,
    /// Swings the hero began in the fight by frame data (a chain's every link counts).
    pub swings: u32,
    /// His blows that landed and the damage they did; foes' blows that reached him (however he
    /// met them: parried, blocked, evaded or not) and the damage he took.
    pub hits: u32,
    pub dealt: f64,
    pub met: u32,
    pub taken: f64,
    /// Foes slain, and packs whose last foe fell (encounters won).
    pub kills: u32,
    pub packs: u32,
    /// The most foes hunting him at once.
    pub engaged: usize,
    /// Changes of level shown (loaded, and made ready by the renderer when the game waits for one).
    pub loads: usize,
}

/// The counters the driver reads from the game after each step, to count their growth.
#[derive(Clone, Copy, Default)]
struct Seen {
    hits: u32,
    dealt: f64,
    met: u32,
    taken: f64,
    kills: u32,
    packs: u32,
    loads: usize,
    /// His swing as last seen: its move and frame.
    swing: Option<(crate::logic::combat::MoveId, u16)>,
}

impl Seen {
    fn of(game: &Game) -> Self {
        let stats = game.brawl.as_ref().map(|b| b.stats).unwrap_or_default();
        let swing = game.brawl.as_ref().and_then(|b| match b.hero.state {
            State::Attack { mv, frame } => Some((mv, frame)),
            _ => None,
        });
        Seen {
            hits: stats.hits,
            dealt: stats.dealt,
            met: stats.landings,
            taken: stats.taken,
            kills: stats.kills,
            packs: game.war.tally.packs_cleared,
            loads: game.transitions.len(),
            swing,
        }
    }
}

/// How much a counter grew from `was` to `now`; one that went back (the fight or the level begun
/// again) counts from zero.
fn grew<T: PartialOrd + std::ops::Sub<Output = T> + Copy>(was: T, now: T) -> T {
    if now >= was {
        now - was
    } else {
        now
    }
}

/// A script played on the game, step by step, with the bot it starts and the deeds it counts.
pub struct Driver {
    script: Vec<(f64, Cmd)>,
    next: usize,
    /// Seconds into the script (back to the start at each `period`).
    pub t: f64,
    /// The script begins again every `period` seconds; 0 plays it once.
    period: f64,
    /// The bot that took the pad at a `bot:K`, kept from one script to the next.
    pub bot: Option<Bot>,
    pub deeds: Deeds,
    seen: Seen,
}

impl Driver {
    /// A driver for `script`, played once from its start.
    pub(crate) fn new(script: Vec<(f64, Cmd)>) -> Self {
        Driver { script, next: 0, t: 0.0, period: 0.0, bot: None, deeds: Deeds::default(), seen: Seen::default() }
    }

    /// Plays `script` from its start, again every `period` seconds if that is above zero. The bot,
    /// if one plays, plays on.
    pub(crate) fn play(&mut self, script: Vec<(f64, Cmd)>, period: f64) {
        self.script = script;
        self.next = 0;
        self.t = 0.0;
        self.period = period;
    }

    /// Every command of a script played once has fired.
    pub fn done(&self) -> bool {
        self.period <= 0.0 && self.next >= self.script.len()
    }

    /// Counts the deeds from here on, from zero.
    pub fn recount(&mut self, game: &Game) {
        self.deeds = Deeds::default();
        self.seen = Seen::of(game);
    }

    /// One simulation step: the commands that are due fire, the bot (if one plays) sets the pad,
    /// and the game updates by `STEP`.
    pub fn step(&mut self, game: &mut Game) {
        while self.next < self.script.len() && self.script[self.next].0 <= self.t {
            exec(game, &self.script[self.next].1, &mut self.bot);
            self.next += 1;
        }
        if let Some(b) = self.bot.as_mut() {
            let pad = b.act(game);
            game.input.set_pad(Some(pad));
        }
        game.update(STEP);
        self.t += STEP;
        if self.period > 0.0 && self.t >= self.period {
            self.t -= self.period;
            self.next = 0;
        }
        self.count(game);
    }

    /// The deeds so far, with the changes of level shown since the last step (a renderer may have
    /// made one ready after it).
    pub fn deeds(&self, game: &Game) -> Deeds {
        Deeds { loads: self.deeds.loads + grew(self.seen.loads, game.transitions.len()), ..self.deeds }
    }

    /// Steps until the script's clock reaches `until` seconds.
    pub fn run(&mut self, game: &mut Game, until: f64) {
        while self.t < until {
            self.step(game);
        }
    }

    /// Notes what the step did.
    fn count(&mut self, game: &Game) {
        let now = Seen::of(game);
        let (was, d) = (self.seen, &mut self.deeds);
        d.steps += 1;
        d.hits += grew(was.hits, now.hits);
        d.dealt += grew(was.dealt, now.dealt);
        d.met += grew(was.met, now.met);
        d.taken += grew(was.taken, now.taken);
        d.kills += grew(was.kills, now.kills);
        d.packs += grew(was.packs, now.packs);
        d.loads += grew(was.loads, now.loads);
        // A swing begins when he was not swinging, or the frame went back (the next link of a chain).
        if let Some((mv, frame)) = now.swing {
            if was.swing.is_none_or(|(m, f)| m != mv || frame < f) {
                d.swings += 1;
            }
        }
        let hunting = game.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.engaged()).count();
        d.engaged = d.engaged.max(hunting);
        self.seen = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::parse_script;
    use crate::game::Screen;
    use crate::game::Stage;

    #[test]
    fn a_scripts_commands_fire_at_their_times() {
        let mut g = Game::new();
        g.reseed(1);
        g.tour_pending = false;
        g.map_jump(Stage::Dungeon(0));
        let mut d = Driver::new(parse_script("0.5:coins:123,1.0:coins:456"));
        d.run(&mut g, 0.49);
        assert_ne!(g.progress.silver, 123, "not before its time");
        d.run(&mut g, 0.5 + STEP);
        assert_eq!(g.progress.silver, 123, "at its time");
        assert!(!d.done());
        d.run(&mut g, 1.0 + STEP);
        assert_eq!(g.progress.silver, 456);
        assert!(d.done(), "every command has fired");
        assert_eq!(d.deeds.steps as f64, (d.t / STEP).round(), "one step a STEP");
    }

    #[test]
    fn a_repeating_script_begins_again_each_period() {
        let mut g = Game::new();
        g.reseed(1);
        g.tour_pending = false;
        g.map_jump(Stage::Dungeon(0));
        let mut d = Driver::new(Vec::new());
        d.play(parse_script("0.1:coins:7"), 0.5);
        d.run(&mut g, 0.2);
        assert_eq!(g.progress.silver, 7);
        g.progress.silver = 0;
        // Past the period the clock wraps, and the command fires again 0.1 s in.
        for _ in 0..(0.45 / STEP) as usize {
            d.step(&mut g);
        }
        assert_eq!(g.progress.silver, 7, "fired again in the next period");
        assert!(!d.done(), "a repeating script never is");
    }

    #[test]
    fn the_bots_pad_is_applied_every_step() {
        let fresh = || {
            let mut g = Game::new();
            g.reseed(1);
            g.tour_pending = false;
            g
        };
        let script = "0.1:realm:4,0.5:bot:2";
        let mut g = fresh();
        let mut d = Driver::new(parse_script(script));
        d.run(&mut g, 0.5);
        assert!(d.bot.is_none(), "no bot before its command");
        d.run(&mut g, 0.5 + STEP);
        assert!(d.bot.is_some() && g.hero.free_move, "the bot plays from its command on");
        let start = g.hero.pos();
        d.run(&mut g, 8.0);
        // The same script played by hand, the bot's pad set before every update, ends in the same
        // place; and the bot did send him somewhere.
        let mut h = fresh();
        let (mut bot, mut t, mut next) = (None, 0.0, 0);
        let cmds = parse_script(script);
        while t < 8.0 {
            while next < cmds.len() && cmds[next].0 <= t {
                exec(&mut h, &cmds[next].1, &mut bot);
                next += 1;
            }
            if let Some(b) = bot.as_mut() {
                let pad = b.act(&h);
                h.input.set_pad(Some(pad));
            }
            h.update(STEP);
            t += STEP;
        }
        assert_eq!(g.screen, Screen::Playing);
        assert_eq!(g.hero.pos(), h.hero.pos(), "the driver steps as the hand-played loop does");
        assert_ne!(g.hero.pos(), start, "the bot's pad moved him");
        assert!((d.deeds.steps as f64 - 8.0 / STEP).abs() <= 1.0, "one step a STEP");
    }
}
