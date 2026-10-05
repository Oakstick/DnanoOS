//! The run's save as one snapshot: the checkpoint (`vh.run`), the purse and the unlocks
//! (`vh.progress`), the Ledger of Grudges (`vh.warband`) and a new high score, written together in
//! one write of the settings file, so a rival's death is never saved without his reward (or the
//! reward without his death).
//!
//! What changes them only marks the snapshot (`mark_saves`); it is written at a calm moment (no
//! freeze-frame, no foe in the middle of a blow, or a second after it was marked at the latest),
//! and at once at a checkpoint, on every change of screen (the pause, the shop, the title, a new
//! level's Ready) and as the game closes (`flush_saves`). One kill is one write at most. A write
//! that fails is told to the console, the snapshot stays marked and is tried again a few seconds
//! later; after several failures the player is told too.

use super::ledger::WARBAND_KEY;
use super::save::{Run, RUN_KEY};
use super::{Game, Screen};
use crate::logic::combat::fsm::State;
use crate::logic::progression::Progress;
use crate::platform;

/// Where the silver, the runes and the unlocks are saved.
pub const PROGRESS_KEY: &str = "vh.progress";
/// The longest a marked snapshot waits for a calm moment, in seconds.
const MAX_WAIT: f64 = 1.0;
/// Seconds before a failed write is tried again.
const RETRY: f64 = 5.0;
/// Failed writes in a row before the player is told.
const TELL_AFTER: u32 = 3;

/// The snapshot's state: marked or written, and how its writes went.
#[derive(Clone, Debug)]
pub struct Persist {
    /// Writes reach the save. Off in tests, which keep off the player's save; a test that means to
    /// save turns it on under a save folder of its own (`platform::use_dir_on_this_thread`).
    pub live: bool,
    /// Something in the snapshot changed since it was last written.
    pub dirty: bool,
    /// Seconds it has been waiting for a calm moment.
    waited: f64,
    /// Seconds before a failed write is tried again.
    retry: f64,
    /// Failed writes in a row.
    pub failures: u32,
    /// Snapshots written, and how long the last and the slowest took (milliseconds).
    pub writes: u32,
    pub last_ms: f64,
    pub worst_ms: f64,
}

impl Default for Persist {
    fn default() -> Self {
        Self { live: !cfg!(test), dirty: false, waited: 0.0, retry: 0.0, failures: 0, writes: 0, last_ms: 0.0, worst_ms: 0.0 }
    }
}

/// The silver, the runes and the unlocks the save holds.
pub(super) fn read_progress() -> Progress {
    platform::load(PROGRESS_KEY).and_then(|s| Progress::decode(&s)).unwrap_or_default()
}

impl Game {
    /// Something of the run's snapshot changed: it is written at the next calm moment.
    pub(super) fn mark_saves(&mut self) {
        self.persist.dirty = true;
    }

    /// The snapshot, if marked (or a new high score is waiting), is written now: at a checkpoint,
    /// a change of screen, and as the game closes.
    pub fn flush_saves(&mut self) {
        let high = self.score.high_to_save();
        if !self.persist.dirty && high.is_none() {
            return;
        }
        if !self.persist.live {
            self.persist.dirty = false;
            self.persist.waited = 0.0;
            return;
        }
        let mut entries = self.snapshot();
        entries.extend(high);
        let pairs: Vec<(&str, &str)> = entries.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let started = web_time::Instant::now();
        let written = platform::save_all(&pairs);
        let ms = started.elapsed().as_secs_f64() * 1e3;
        let p = &mut self.persist;
        p.last_ms = ms;
        p.worst_ms = p.worst_ms.max(ms);
        match written {
            Ok(()) => {
                p.dirty = false;
                p.waited = 0.0;
                p.retry = 0.0;
                p.failures = 0;
                p.writes += 1;
                self.score.high_saved();
            }
            Err(e) => self.save_failed(&e),
        }
    }

    /// A write failed (now, or behind: `platform::take_write_failure`): the snapshot is kept marked
    /// and tried again in a while, and after several failures the player is told.
    fn save_failed(&mut self, e: &str) {
        let p = &mut self.persist;
        p.dirty = true;
        p.retry = RETRY;
        p.failures += 1;
        platform::warn(&format!("could not save the run (try {}): {e}", p.failures));
        if p.failures == TELL_AFTER && self.screen == Screen::Playing {
            let at = self.hero.pos();
            self.floater(at.x, at.y - 2.2, "THE GAME COULD NOT BE SAVED", 3.0, Some(0xff7a5a));
        }
    }

    /// Every update: a marked snapshot is written once the moment is calm, or a second on.
    pub(super) fn tick_saves(&mut self, dt: f64) {
        // A write behind that failed is tried again, as one that failed in place.
        if let Some(e) = platform::take_write_failure() {
            self.save_failed(&e);
        }
        if !self.persist.dirty {
            return;
        }
        self.persist.waited += dt;
        if self.persist.retry > 0.0 {
            self.persist.retry -= dt;
            if self.persist.retry > 0.0 {
                return;
            }
        }
        if self.save_calm() || self.persist.waited >= MAX_WAIT {
            self.flush_saves();
        }
    }

    /// Out of play, or in it with no freeze-frame held and no foe in the middle of a blow.
    fn save_calm(&self) -> bool {
        if self.screen != Screen::Playing {
            return true;
        }
        let striking =
            self.enemies.iter().filter_map(|e| e.duel.as_deref()).any(|d| d.alive() && matches!(d.f.state, State::Attack { .. }));
        self.hitstop <= 0.0 && !striking
    }

    /// The run's keys and values, as they stand. The Ledger goes with a run under way (and is
    /// emptied with it).
    fn snapshot(&self) -> Vec<(&'static str, String)> {
        let run = self.saved_run.as_ref().map(Run::encode).unwrap_or_default();
        let war = if self.saved_run.is_some() { self.warband.save.encode() } else { String::new() };
        vec![(RUN_KEY, run), (PROGRESS_KEY, self.progress.encode()), (WARBAND_KEY, war)]
    }

    /// The run as the save holds it: the checkpoint, the purse and the Ledger (`Game::new` reads
    /// them so, outside tests).
    #[cfg(test)]
    fn read_saved(&mut self) {
        self.saved_run = super::save::read_run();
        self.progress = read_progress();
        self.warband = super::ledger::Warband::read();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::STEP;
    use crate::logic::sins::Sin;
    use crate::logic::war::names::Deed;
    use crate::logic::war::WarEvent;
    use std::path::PathBuf;

    /// A save folder of this test's own, empty, that this thread's saves go to.
    fn own_folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vh-persist-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        platform::use_dir_on_this_thread(dir.clone());
        dir
    }

    /// In the vaults of Greed with a run under way, saving on: a rival seated at a duelist's place.
    fn vault_with_a_rival(name: &str) -> (Game, u32, crate::logic::war::ledger::RivalId, PathBuf) {
        let dir = own_folder(name);
        let mut g = super::super::bot::fresh_game(3);
        g.map_jump(super::super::save::Stage::Realm(Sin::Greed));
        g.enter(Screen::Playing);
        for _ in 0..10 {
            g.update(STEP);
        }
        let d = g.enemies.iter().filter_map(|e| e.duel.as_deref()).next().expect("a duelist in the vaults");
        let (id, kind) = (d.id, d.breed.kind);
        let rival = g.promote_rival(kind, Deed::KillingBlow);
        // He took silver from the hero once, so his fall gives silver back as well as runes.
        g.warband.save.ledger.took_silver(rival, 200);
        g.war.grudges.seated.push((id, rival));
        g.persist.live = true;
        g.flush_saves();
        assert!(!g.persist.dirty);
        (g, id, rival, dir)
    }

    /// The rival duelist `id` falls, as the war log tells it to the grudges.
    fn heard_slain(g: &mut Game, id: u32) {
        let d = g.enemies.iter().filter_map(|e| e.duel.as_deref()).find(|d| d.id == id).unwrap();
        let (kind, at) = (d.breed.kind, d.pos);
        g.grudges_hear(&[WarEvent::Slain { foe: id, kind, at, dir: at, last_of_pack: false }]);
    }

    /// A fresh game reading what the save folder holds.
    fn reloaded() -> Game {
        let mut g = Game::new();
        g.read_saved();
        g
    }

    #[test]
    fn a_rival_slain_and_the_game_closed_at_once_keeps_his_death_and_his_reward() {
        let (mut g, id, rival, _dir) = vault_with_a_rival("quit");
        let (silver, runes) = (g.progress.silver, g.progress.runes);
        heard_slain(&mut g, id);
        assert!(g.progress.runes > runes && g.progress.silver > silver, "his reward");
        assert!(g.persist.dirty, "marked, not written on the kill");
        // Closed at once (the window's exit flushes, App::exiting).
        g.flush_saves();
        let back = reloaded();
        assert_eq!((back.progress.silver, back.progress.runes), (g.progress.silver, g.progress.runes), "the reward was saved");
        let r = back.ledger().get(rival).expect("he is in the saved Ledger");
        assert!(!r.alive, "and so was his death");
        assert_eq!(back.saved_run, g.saved_run);
    }

    #[test]
    fn a_kill_is_one_write_at_a_calm_moment() {
        let (mut g, id, rival, _dir) = vault_with_a_rival("one-write");
        let before = platform::writes_on_this_thread();
        // He falls as a blow fells him: his fall is heard on the next frame, with the freeze it
        // brings.
        let i = g.enemies.iter().position(|e| e.duel.as_ref().is_some_and(|d| d.id == id)).unwrap();
        g.slay(i);
        assert_eq!(platform::writes_on_this_thread(), before, "nothing written as he falls");
        for _ in 0..(1.5 / STEP) as usize {
            g.update(STEP);
        }
        assert!(!g.persist.dirty);
        assert!(g.ledger().get(rival).is_some_and(|r| !r.alive), "he fell");
        assert_eq!(platform::writes_on_this_thread() - before, 1, "his fight, his fall and his reward: one write");
        assert!(g.persist.last_ms > 0.0);
    }

    #[test]
    fn a_broken_save_is_read_from_its_copy() {
        let dir = own_folder("backup");
        let mut g = Game::new();
        g.persist.live = true;
        g.progress.silver = 100;
        g.mark_saves();
        g.flush_saves();
        g.progress.silver = 200;
        g.mark_saves();
        g.flush_saves();
        assert_eq!(read_progress().silver, 200);
        // Torn: the settings file loses its tail mid-line.
        let main = dir.join("settings.txt");
        let text = std::fs::read_to_string(&main).unwrap();
        std::fs::write(&main, text[..text.len() / 2].replace('=', " ")).unwrap();
        assert_eq!(read_progress().silver, 100, "the copy before the last write");
        // Missing altogether: the copy again.
        std::fs::remove_file(&main).unwrap();
        assert_eq!(read_progress().silver, 100);
        // The next write is whole again, and the good copy was not replaced by the broken file.
        g.progress.silver = 300;
        g.mark_saves();
        g.flush_saves();
        assert_eq!(read_progress().silver, 300);
        std::fs::remove_file(&main).unwrap();
        assert_eq!(read_progress().silver, 100, "the good copy was kept");
    }

    #[test]
    fn a_failed_write_stays_marked_and_is_tried_again() {
        let dir = own_folder("fails");
        // The save folder cannot be made: a file stands where it should be.
        std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
        std::fs::write(&dir, "in the way").unwrap();
        platform::use_dir_on_this_thread(dir.join("saves"));
        let mut g = Game::new();
        g.persist.live = true;
        g.progress.silver = 70;
        g.mark_saves();
        g.flush_saves();
        assert!(g.persist.dirty, "kept marked");
        assert_eq!((g.persist.failures, g.persist.writes), (1, 0));
        // Not tried again every frame...
        g.update(STEP);
        assert_eq!(g.persist.failures, 1);
        // ...but once the way is clear, a few seconds on, it is written.
        std::fs::remove_file(&dir).unwrap();
        for _ in 0..((RETRY + 0.5) / STEP) as usize {
            g.update(STEP);
        }
        assert!(!g.persist.dirty);
        assert_eq!((g.persist.failures, g.persist.writes), (0, 1));
        assert_eq!(read_progress().silver, 70);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tests_write_nothing_unless_they_ask() {
        let _dir = own_folder("off");
        let before = platform::writes_on_this_thread();
        let mut g = Game::new();
        g.progress.silver = 5;
        g.mark_saves();
        g.flush_saves();
        g.enter(Screen::Paused);
        assert!(!g.persist.dirty);
        assert_eq!(platform::writes_on_this_thread(), before);
    }

    /// What a write costs (run with `--nocapture` to see it): the snapshot of a run well under way,
    /// against the two whole-file saves a rival's fall made before.
    #[test]
    fn the_cost_of_a_save() {
        let _dir = own_folder("cost");
        let mut g = Game::new();
        g.persist.live = true;
        g.debug_ledger();
        g.saved_run = Some(Run {
            hero: crate::logic::heroes::HeroKind::Ragnar,
            stage: super::super::save::Stage::Realm(Sin::Greed),
            score: 12_345,
            lives: 2,
            belt: Vec::new(),
            sins: 1,
        });
        g.progress.silver = 4321;
        let n = 40;
        let mut times = Vec::new();
        for k in 0..n {
            g.progress.runes = k;
            g.mark_saves();
            g.flush_saves();
            times.push(g.persist.last_ms);
        }
        let war = g.warband.save.encode();
        let mut old = Vec::new();
        for _ in 0..n {
            let t = web_time::Instant::now();
            platform::save(WARBAND_KEY, &war);
            platform::save(WARBAND_KEY, &war);
            old.push(t.elapsed().as_secs_f64() * 1e3);
        }
        let stats = |v: &mut Vec<f64>| {
            v.sort_by(f64::total_cmp);
            (v.iter().sum::<f64>() / v.len() as f64, v[v.len() / 2], v[v.len() - 1])
        };
        let (mean, median, worst) = stats(&mut times);
        let (old_mean, old_median, old_worst) = stats(&mut old);
        eprintln!(
            "save: snapshot {mean:.2} ms mean, {median:.2} median, {worst:.2} worst; a kill's two saves before {old_mean:.2} mean, {old_median:.2} median, {old_worst:.2} worst"
        );
        assert_eq!(g.persist.writes, n);
    }
}
