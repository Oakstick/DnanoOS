//! The juice, applied (WARBAND.md §4.1-§4.2): the freeze on a hit by the table and within its
//! budget, the slowing of time, and the two settings that temper the feel (slow motion on kills,
//! the screen's shake). The numbers are `logic::juice`'s; the camera, the chips and the shatter,
//! and the springs are the view's (`render::view::juice`).
//!
//! With the juice switched off (`--fx-off juice`, as the harnesses run) the freezes are the brawl's
//! as they always were, with no budget, and time never slows: the bots' numbers do not move with
//! the feel.

use super::{Game, MenuItem, Screen};
use crate::input::Action;
use crate::logic::juice::{budgeted, Dilation, DilationKind, Stop, BUDGET_WINDOW, DILATION_EVERY, FPS};
use crate::platform;
use crate::render::profile::fx;

/// The settings keys: slow motion on kills ("1" or "0"), and the screen's shake in percent.
const SLOW_MOTION_KEY: &str = "vh.slow_motion";
const SHAKE_KEY: &str = "vh.shake";

/// The steps the shake setting goes through, from the pause menu.
pub const SHAKE_STEPS: [u8; 5] = [100, 75, 50, 25, 0];

/// The player's settings for the feel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Feel {
    /// Time slows for a beat on the kills that end something (on by default).
    pub slow_motion: bool,
    /// How much the screen shakes, in percent (accessibility; 100 by default).
    pub shake: u8,
}

impl Default for Feel {
    fn default() -> Self {
        Self { slow_motion: true, shake: 100 }
    }
}

impl Feel {
    /// As saved (tests keep to the defaults).
    pub fn load() -> Self {
        if cfg!(test) {
            return Self::default();
        }
        let slow_motion = platform::load(SLOW_MOTION_KEY).map(|v| v.trim() != "0").unwrap_or(true);
        let shake = platform::load(SHAKE_KEY).and_then(|v| v.trim().parse::<u8>().ok()).map(|v| v.min(100)).unwrap_or(100);
        Self { slow_motion, shake }
    }

    fn save(&self) {
        if cfg!(test) {
            return;
        }
        platform::save(SLOW_MOTION_KEY, if self.slow_motion { "1" } else { "0" });
        platform::save(SHAKE_KEY, &self.shake.to_string());
    }
}

/// The game's clock under the juice: the freezes of the last window (when, in seconds of real
/// time, and how many frames each held), the slowing of time under way (each with its age in
/// seconds of real time out of a freeze), when time last slowed, the time scale, the pace, and the
/// presses made while the world was frozen, kept for the step that ends the freeze.
#[derive(Clone, Debug)]
pub struct Tempo {
    pub(super) freezes: Vec<(f64, f64)>,
    pub(super) dilations: Vec<(Dilation, f64)>,
    pub(super) last_dilation: Option<f64>,
    pub(super) scale: f64,
    pub(super) pace: f64,
    pub(super) kept: Vec<Action>,
}

impl Default for Tempo {
    fn default() -> Self {
        Self { freezes: Vec::new(), dilations: Vec::new(), last_dilation: None, scale: 1.0, pace: 1.0, kept: Vec::new() }
    }
}

impl Game {
    /**
     * The world holds for a beat (WARBAND.md §4.1): `stop`'s frames by the table, within the
     * budget (at most 14 frames of freeze in any 60; past it, each new freeze is halved). A
     * freeze that falls inside one already holding only holds as long as the longer of the two.
     */
    pub fn freeze(&mut self, stop: Stop) {
        let frames = if self.juice_off & fx::HITSTOP != 0 { stop.legacy_frames() } else { stop.frames() };
        self.freeze_frames(frames as f64);
    }

    /// A freeze of `frames` (60 Hz), within the budget: for the freezes outside the brawl's table
    /// (the garden's blows, the Cyclops, a BRUTAL streak).
    pub fn freeze_frames(&mut self, frames: f64) {
        if frames <= 0.0 {
            return;
        }
        if self.juice_off & fx::HITSTOP != 0 {
            self.hitstop = self.hitstop.max(frames / FPS);
            return;
        }
        let now = self.time;
        let window = BUDGET_WINDOW / FPS;
        self.tempo.freezes.retain(|&(t, _)| now - t < window);
        let used: f64 = self.tempo.freezes.iter().map(|f| f.1).sum();
        let held = budgeted(used, frames);
        let added = held - self.hitstop.max(0.0) * FPS;
        if added > 0.0 {
            self.tempo.freezes.push((now, added));
            self.hitstop = held / FPS;
        }
    }

    /**
     * Time slows for a beat (WARBAND.md §4.2): `kind`'s scale, held, then eased back. At most once
     * every three seconds; never with the setting off, out of play, or under a story's or a tour's
     * camera. Returns whether it slowed.
     */
    pub fn dilate(&mut self, kind: DilationKind) -> bool {
        if self.juice_off & fx::DILATION != 0 || !self.feel.slow_motion || self.screen != Screen::Playing || self.view.cinematic() {
            return false;
        }
        if self.tempo.last_dilation.is_some_and(|t| self.time - t < DILATION_EVERY) {
            return false;
        }
        self.tempo.last_dilation = Some(self.time);
        self.tempo.dilations.push((kind.dilation(), 0.0));
        true
    }

    /// The time scale now (1 at full speed; below it while time is slowed). The audio pitches its
    /// effects by it (`logic::juice::pitch_of`).
    pub fn time_scale(&self) -> f64 {
        self.tempo.scale
    }

    /// The share of real time the world ran in the last step: 0 in a freeze or the pause, the time
    /// scale while time is slowed, else 1.
    pub fn pace(&self) -> f64 {
        self.tempo.pace
    }

    /// Switches juice off (`fx` bits; `fx::JUICE` for all of it): the harnesses run without it.
    pub fn set_juice_off(&mut self, off: u32) {
        self.juice_off = off & fx::JUICE;
        if self.juice_off & fx::DILATION != 0 {
            self.tempo.dilations.clear();
            self.tempo.scale = 1.0;
        }
    }

    /**
     * One step of play: frozen while a freeze holds (the world waits, the effects and the camera
     * go on), else the world runs at the time scale. The slowing of time ages on real time, but
     * only out of a freeze (a kill's freeze, then its slow motion).
     *
     * A button pressed in a freeze is not lost: no step reads it while the world waits, so it is
     * kept and handed to the step that ends the freeze, as if pressed then (once, however often it
     * was pressed in the freeze; a held button, the guard, is read as it stands). The fighter's
     * buffer ages on the fight's frames and none pass in a freeze, so a press at its very start
     * reaches the buffer as fresh as one at its end, and waits there `fsm::BUFFER` frames as any
     * other press does. The kept presses are the step's alone: what reads the buttons after it
     * (and already saw them in the freeze) does not see them again.
     */
    pub(super) fn play_step(&mut self, dt: f64) {
        if self.hitstop > 0.0 {
            self.hitstop -= dt;
            self.tempo.pace = 0.0;
            for a in self.input.presses() {
                if !self.tempo.kept.contains(&a) {
                    self.tempo.kept.push(a);
                }
            }
            return;
        }
        if self.view.cinematic() {
            self.tempo.dilations.clear();
        }
        let mut scale = 1.0f64;
        self.tempo.dilations.retain_mut(|(d, age)| match d.scale_at(*age) {
            Some(s) => {
                scale = scale.min(s);
                *age += dt;
                true
            }
            None => false,
        });
        self.tempo.scale = scale;
        self.tempo.pace = scale;
        let kept: Vec<Action> = std::mem::take(&mut self.tempo.kept).into_iter().filter(|&a| self.input.repress(a)).collect();
        self.tick(dt * scale);
        for a in kept {
            self.input.consume(a);
        }
    }

    /// Out of play the world's pace is the screen's: still under the pause, else on.
    pub(super) fn settle_pace(&mut self) {
        self.tempo.pace = if self.screen == Screen::Paused { 0.0 } else { 1.0 };
    }

    /// A new level (or round): no freeze, no slowing of time, no press kept from a freeze carried
    /// into it.
    pub(super) fn reset_tempo(&mut self) {
        self.hitstop = 0.0;
        self.tempo.kept.clear();
        self.tempo.freezes.clear();
        self.tempo.dilations.clear();
        self.tempo.scale = 1.0;
    }

    /// The settings and switches the view reads each step.
    pub(super) fn sync_feel(&mut self) {
        self.view.set_feel(self.feel.shake as f32 / 100.0, self.juice_off);
    }

    /// The pause menu's buttons for the feel, showing the settings as they stand.
    pub(super) fn feel_items(&self) -> [MenuItem; 2] {
        [MenuItem::SlowMotion(self.feel.slow_motion), MenuItem::Shake(self.feel.shake)]
    }

    /// Slow motion on kills, on or off (and remembered).
    pub(super) fn toggle_slow_motion(&mut self) {
        self.feel.slow_motion = !self.feel.slow_motion;
        if !self.feel.slow_motion {
            self.tempo.dilations.clear();
        }
        self.feel.save();
    }

    /// The screen's shake one step down (from none, back to full), and remembered.
    pub(super) fn step_shake(&mut self) {
        let k = SHAKE_STEPS.iter().position(|&s| s == self.feel.shake).map(|k| (k + 1) % SHAKE_STEPS.len()).unwrap_or(0);
        self.feel.shake = SHAKE_STEPS[k];
        self.feel.save();
    }
}

/// The juice the capture tool's `juice:K` fires, in turn (see `Game::debug_juice`).
pub const JUICE_EVENTS: [&str; 19] = [
    "kick",
    "fov",
    "chips flesh",
    "chips bone",
    "chips iron",
    "chips stone",
    "chips feather",
    "chips smoke",
    "chips ember",
    "chips ice",
    "chips gold",
    "chips slime",
    "chips wood",
    "chips chitin",
    "shatter",
    "dilation",
    "sparks block",
    "recoil",
    "shield jolt",
];

impl Game {
    /**
     * Fires juice event `k` of `JUICE_EVENTS` (for captures, in the ring, on its nearest foe): the
     * camera kicked along a blow from the foe, a FOV punch, chips of each matter, the foe shattered,
     * the foe shattered in slow motion, sparks off his shield, the foe's flinch, his shield jolted.
     */
    pub fn debug_juice(&mut self, k: usize) {
        use crate::logic::juice::{Matter, Trauma, FOV_FINISHER, KICK_STRUCK};
        let hp = self.hero.pos();
        let foe = (0..self.enemies.len()).filter(|&i| self.enemies[i].duel.as_ref().is_some_and(|d| d.alive())).min_by(|&a, &b| {
            let d = |i: usize| crate::logic::types::dist_sq(self.enemies[i].duel.as_ref().unwrap().pos, hp);
            d(a).total_cmp(&d(b))
        });
        let at = foe.and_then(|i| self.enemies[i].duel.as_ref()).map(|d| d.pos).unwrap_or(hp);
        let id = foe.and_then(|i| self.enemies[i].duel.as_ref()).map(|d| d.id).unwrap_or(0);
        let away = crate::logic::combat::math::norm(crate::logic::combat::math::sub(at, hp));
        let toward = crate::logic::types::V::new(-away.x, -away.y);
        let grid = &self.level.grid;
        match k {
            0 => {
                self.view.trauma(Trauma::Struck { dmg: 30.0 });
                self.view.kick(toward, KICK_STRUCK);
            }
            1 => {
                self.view.trauma(Trauma::Finisher);
                self.view.fov_punch(FOV_FINISHER.0 * 3.0, 0.5);
            }
            2..=13 => self.view.chips(grid, at, away, Matter::ALL[k - 2], 10),
            14 | 15 => {
                if let Some(i) = foe {
                    self.view.shatter(i, away);
                    if let Some(d) = self.enemies[i].duel.as_mut() {
                        d.hp = 0.0;
                        d.f.kill();
                    }
                    self.enemies[i].b.mode = crate::logic::enemy::EnemyMode::Eaten;
                    self.freeze(crate::logic::juice::Stop::Kill);
                    if k == 15 {
                        self.dilate(crate::logic::juice::DilationKind::Trophy);
                    }
                }
            }
            16 => {
                self.view.sparks_block(grid, hp, toward);
                self.view.shield_jolt();
            }
            17 => self.view.recoil(id, away),
            _ => self.view.shield_jolt(),
        }
    }
}
