//! The Sparring Ring (COMBAT.md §5): the combat lab west of the gym's arrival platform.
//!
//! Inside the ring the brawl's rules hold (`brawl`): the hero fights by frame data and the foes
//! are duelists whose attacks the threat manager paces. This module is the bouts on top: step in
//! and one begins; lose and it begins again; win and the next one comes. Nothing here touches his
//! shield, his lives or his gold.

pub use super::brawl::BoutStats;
use super::brawl::Field;
use super::Game;
use crate::audio::Sfx;
use crate::logic::combat::{Breed, Duelist, Reel, State};
use crate::logic::enemy::{spec_of, Enemy, EnemyKind, EnemyMode};
use crate::logic::gym;
use crate::logic::types::{P, V};
use crate::render::view::CameraMode;

/// Seconds of the countdown before a bout, and of the pause after one is won or lost.
pub const COUNTDOWN: f64 = 2.0;
/// (Long enough to read the results card.)
pub const AFTERMATH: f64 = 4.5;
/// The combat log: one line per bout, in the save folder (the web build keeps the last lines in storage).
pub const LOG_FILE: &str = "combat_log.csv";
const LOG_HEADER: &str = "time,hero,tier,bout,result,secs,dealt,taken,hits,hurt,parries,blocks,evades,breaks,caught,guard_breaks,overlaps";

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BoutPhase {
    /// The hero is outside the ring.
    Idle,
    /// Seconds before the foes move.
    Countdown(f64),
    Fight,
    /// Seconds of the pause after a bout won, or lost.
    Won(f64),
    Lost(f64),
}

#[derive(Clone, Debug)]
pub struct Sparring {
    /// Which of `gym::BOUTS`.
    pub bout: usize,
    pub phase: BoutPhase,
    /// The last bout won, with its numbers, shown until the next is won.
    pub last: Option<(usize, BoutStats)>,
    /// The camera before the ring turned it to the top view (the ring is fought from above, like the vaults).
    pub camera_before: Option<CameraMode>,
    /// A name in place of the bout's (a trial of mastery's).
    pub label: Option<String>,
}

impl Default for Sparring {
    fn default() -> Self {
        Self::new()
    }
}

impl Sparring {
    pub fn new() -> Self {
        Self { bout: 0, phase: BoutPhase::Idle, last: None, camera_before: None, label: None }
    }

    /// The bout's name, numbered.
    pub fn title(&self) -> String {
        if let Some(l) = &self.label {
            return l.clone();
        }
        format!("BOUT {}: {}", self.bout + 1, gym::BOUTS[self.bout].name)
    }
}

impl Game {
    /// The hero stands in the Sparring Ring.
    pub fn in_ring(&self) -> bool {
        self.spar.is_some() && gym::ring().contains(self.hero.occupied())
    }

    /// Takes the bout's foes off the ring.
    fn clear_duelists(&mut self) {
        if self.enemies.iter().any(|e| e.duel.is_some()) {
            self.enemies.retain(|e| e.duel.is_none());
            self.view.clear_enemies();
            for e in &self.enemies {
                self.view.add_enemy(e);
            }
        }
        if let Some(b) = self.brawl.as_mut() {
            let kit = b.kit;
            b.reset(kit);
        }
    }

    /// The bout's foes take their places along the ring's west side, and the countdown starts.
    pub(super) fn start_bout(&mut self) {
        let Some(spar) = self.spar.as_ref() else { return };
        self.start_bout_of(gym::BOUTS[spar.bout].foes);
    }

    /// A bout of the ring against `foes` (the bout's own, or a capture's line-up).
    fn start_bout_of(&mut self, foes: &[EnemyKind]) {
        self.clear_duelists();
        let kit = self.hero_kit();
        let (Some(spar), Some(b)) = (self.spar.as_mut(), self.brawl.as_mut()) else { return };
        let mut fresh = Vec::new();
        for (&kind, at) in foes.iter().zip(gym::bout_spawns(foes.len())) {
            let breed = Breed::of(kind);
            let spec = spec_of(&self.level.grid, kind);
            let mut e = Enemy::new(&spec, P::new(at.x.round() as i32, at.y.round() as i32), self.level.data.enemy_speed);
            e.b.mode = EnemyMode::Chase;
            // (The lab's foes go by their archetypes, a stage's by their own names.)
            let lab = breed.arch.kind() == kind && breed.arch != crate::logic::combat::Archetype::Prince;
            e.b.name = if lab { breed.arch.name().to_string() } else { spec.name.to_uppercase() };
            e.duel = Some(Box::new(Duelist::bred(b.next_id(), breed, at, V::new(1.0, 0.0))));
            fresh.push(e);
        }
        spar.phase = BoutPhase::Countdown(COUNTDOWN);
        if spar.camera_before.is_none() {
            spar.camera_before = Some(self.view.camera_mode);
            self.view.camera_mode = CameraMode::Top;
        }
        b.reset(kit);
        b.live = false;
        let title = spar.title();
        for e in fresh {
            self.view.add_enemy(&e);
            self.enemies.push(e);
        }
        self.hero.hp = self.hero.max_hp;
        let hp = self.hero.pos();
        self.floater(hp.x, hp.y - 1.4, title, COUNTDOWN, Some(0xffc860));
        self.audio.play(Sfx::Tick);
    }

    /// Out of the ring: the bout is called off.
    fn end_bout(&mut self) {
        self.clear_duelists();
        if let Some(b) = self.brawl.as_mut() {
            b.live = false;
        }
        if let Some(s) = self.spar.as_mut() {
            s.phase = BoutPhase::Idle;
            if let Some(mode) = s.camera_before.take() {
                self.view.camera_mode = mode;
            }
        }
        self.hero.root = 1.0;
        self.hero.blocking = false;
    }

    /// F9: on to the next bout.
    pub(super) fn next_bout(&mut self) {
        let inside = self.in_ring();
        let Some(s) = self.spar.as_mut() else { return };
        s.bout = (s.bout + 1) % gym::BOUTS.len();
        if inside {
            self.start_bout();
        }
    }

    /// F7: on to the first bout of the next stage's foes.
    pub(super) fn next_stage_bout(&mut self) {
        let inside = self.in_ring();
        let Some(s) = self.spar.as_mut() else { return };
        s.bout = gym::next_stage_bout(s.bout);
        if inside {
            self.start_bout();
        }
    }

    /// The ring's real-time flow: bouts begin, end, start over (the brawl runs their frames).
    pub(super) fn update_sparring(&mut self, dt: f64) {
        let inside = self.in_ring();
        let Some(phase) = self.spar.as_ref().map(|s| s.phase) else { return };
        match phase {
            BoutPhase::Idle => {
                if inside {
                    self.start_bout();
                }
            }
            BoutPhase::Countdown(_) | BoutPhase::Fight if !inside => self.end_bout(),
            BoutPhase::Countdown(t) => {
                let left = t - dt;
                if let Some(s) = self.spar.as_mut() {
                    s.phase = if left <= 0.0 { BoutPhase::Fight } else { BoutPhase::Countdown(left) };
                }
                if left <= 0.0 {
                    if let Some(b) = self.brawl.as_mut() {
                        b.live = true;
                    }
                    self.audio.play(Sfx::Go);
                }
            }
            BoutPhase::Fight => {}
            BoutPhase::Won(t) | BoutPhase::Lost(t) => {
                let left = t - dt;
                if left > 0.0 {
                    if let Some(s) = self.spar.as_mut() {
                        s.phase = if matches!(phase, BoutPhase::Won(_)) { BoutPhase::Won(left) } else { BoutPhase::Lost(left) };
                    }
                } else if self.in_trial() {
                    // A trial's aftermath: back to the old man's yard.
                    self.trial_over();
                } else {
                    if let Some(s) = self.spar.as_mut() {
                        if matches!(phase, BoutPhase::Won(_)) {
                            s.bout = (s.bout + 1) % gym::BOUTS.len();
                        }
                        s.phase = BoutPhase::Idle;
                    }
                    self.hero.hp = self.hero.max_hp;
                    self.clear_duelists();
                    if inside {
                        self.start_bout();
                    }
                }
            }
        }
    }

    /**
     * The bout is over: won (every foe down) or lost (his health gone). The results card shows
     * its numbers through the aftermath, and a line goes into the combat log (`combat_log.csv`
     * by the saves) so playtests leave data to tune from (COMBAT.md phase 2).
     */
    pub(super) fn finish_bout(&mut self, won: bool) {
        let hp = self.hero.pos();
        let (hero, tier) = (self.hero_kind, self.hero.blade);
        let (Some(s), Some(b)) = (self.spar.as_mut(), self.brawl.as_mut()) else { return };
        if b.field != Field::Ring {
            return;
        }
        s.phase = if won { BoutPhase::Won(AFTERMATH) } else { BoutPhase::Lost(AFTERMATH) };
        b.live = false;
        if won {
            s.last = Some((s.bout, b.stats));
        } else {
            b.hero.state = State::Reel { kind: Reel::Staggered, left: 90, total: 90 };
        }
        let st = b.stats;
        let line = format!(
            "{},{},{},{},{},{:.2},{:.0},{:.0},{},{},{},{},{},{},{},{},{}",
            crate::platform::now_secs(),
            hero.name(),
            tier,
            s.bout + 1,
            if won { "won" } else { "lost" },
            st.frames as f64 / 60.0,
            st.dealt,
            st.taken,
            st.hits,
            st.hurt,
            st.parries,
            st.blocks,
            st.evades,
            st.breaks,
            st.caught,
            st.guard_breaks,
            st.overlaps
        );
        crate::platform::append_line(LOG_FILE, LOG_HEADER, &line);
        if won {
            self.audio.play(Sfx::Win);
        } else {
            self.audio.play(Sfx::Death);
            self.floater(hp.x, hp.y - 1.4, "DEFEATED", 1.5, Some(0xff6a4a));
        }
    }

    /// Into the Sparring Ring (from anywhere), for bout `n` (1-based; 0 keeps the one it is on): the
    /// bout's countdown starts at once. (Captures and tests.)
    pub fn debug_ring(&mut self, n: usize) {
        if !self.theme().is_gym() {
            self.map_jump(super::save::Stage::Gym);
        }
        self.enter(super::Screen::Playing);
        self.hero.place(gym::ring_start());
        self.hero.face(crate::logic::types::Dir::Left);
        if let Some(s) = self.spar.as_mut() {
            if n > 0 {
                s.bout = (n - 1) % gym::BOUTS.len();
            }
        }
        self.start_bout();
    }

    /// The ring against `foes` (captures: a pack of one kind, for its animation).
    pub fn debug_ring_of(&mut self, foes: &[EnemyKind]) {
        self.debug_ring(0);
        self.start_bout_of(foes);
    }
}
