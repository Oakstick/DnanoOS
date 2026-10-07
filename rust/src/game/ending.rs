//! The end (FINALE.md §4). Krampus is dead and the friend he took has reached the hero at the
//! bottom of the Pit: `begin_ending` plays Scene II in the story's own player (`story`), with
//! the hymn under it (`audio::hymn`), then the words and the credits, then the title. Esc skips
//! it as it skips Scene I. The save remembers that the game was finished, and the title shows a
//! small mark for it.

use super::story::{Story, SKIP_FADE};
use super::{Game, Screen};
use crate::audio::hymn;
use crate::audio::{Sfx, Soundtrack};
use crate::input::Action;
use crate::logic::story::{ascent, Scene, Shot};
use crate::platform;

/// Where the save keeps that the game was finished.
pub const FINISHED_KEY: &str = "vh.finished";

/// Seconds the picture takes to go to black when the end is skipped (a little longer than a
/// skip of Scene I: the hymn is let go over it).
const ENDING_SKIP_FADE: f64 = 1.1;

/// Has this player finished the game (on this machine, or this browser)?
pub fn finished_saved() -> bool {
    platform::load(FINISHED_KEY).is_some_and(|v| v.trim() == "1")
}

/// Writes into the save that the game was finished. (The tests keep off the player's save: they
/// call this alone, under a save folder of their own.)
fn remember_finished() {
    platform::save(FINISHED_KEY, "1");
}

/// Seconds into the credits at which their song comes in, and the hymn's fade under it.
const CREDITS_SONG_IN: f32 = 0.05;
const CREDITS_HANDOVER: f64 = 4.0;
/// Seconds before the credits end at which their song begins to fade.
const CREDITS_SONG_OUT: f32 = 4.0;

/// How long the credits roll.
fn credits_hold() -> f32 {
    let beats = Scene::Two.beats();
    crate::logic::story::first(Shot::Credits).map_or(44.0, |i| beats[i].hold as f32)
}

/// Seconds into Scene II at which the hymn's first bar sounds.
pub fn hymn_start() -> f64 {
    Scene::Two.time_at(crate::logic::story::first(Shot::Light).unwrap_or(1), ascent::VOICES as f64)
}

/// The bar of the hymn that begins between scene times `prev` (exclusive) and `now`
/// (inclusive), if one does.
pub fn bar_between(prev: f64, now: f64) -> Option<usize> {
    let start = hymn_start();
    let bar = |t: f64| ((t - start) / hymn::BAR).floor();
    let b = bar(now);
    let begun = now >= start && (prev < start || bar(prev) < b);
    (begun && (b as usize) < hymn::SCORE.len()).then_some(b as usize)
}

impl Game {
    /**
     * The end begins: from wherever the lair leaves them (the hero and the freed boy together
     * at the bottom of the Pit), Scene II plays. The game's music stops for the hymn, the save
     * marks the game as finished, and the fight's leftovers are cleared away.
     */
    pub fn begin_ending(&mut self) {
        self.finished = true;
        if !cfg!(test) {
            remember_finished();
        }
        self.play_ending();
    }

    /// Scene II from its start, without marking anything (the Map, the captures).
    pub(super) fn play_ending(&mut self) {
        self.guide_open = false;
        self.show_map = false;
        self.show_controls = false;
        self.floaters.clear();
        self.juice.banner = None;
        self.story = Some(Story::of(Scene::Two));
        self.enter(Screen::Story);
        self.audio.unlock();
        self.audio.hush();
        self.audio.set_threat(0.0);
        self.audio.set_danger(0.0);
        self.audio.set_time_scale(1.0);
        for a in [Action::Confirm, Action::Jump, Action::Attack, Action::Tap, Action::Pause] {
            self.input.consume(a);
        }
    }

    /// Jumps Scene II to `secs` seconds from its start (captures, tests), beginning it first if
    /// it is not playing.
    pub fn ending_jump(&mut self, secs: f64) {
        if self.story.as_ref().is_none_or(|s| s.scene != Scene::Two) || self.screen != Screen::Story {
            self.play_ending();
        }
        let (beat, t) = Scene::Two.beat_at(secs);
        self.story_jump(beat, t);
    }

    /// Sounds pinned to moments of Scene II, and the hymn's bars as they come.
    pub(super) fn ending_cues(&mut self, shot: Shot, prev: f64, t: f64) {
        let at = |when: f32| prev < when as f64 && t >= when as f64;
        match shot {
            // The cold of the bottom: one breath of wind; then quiet.
            Shot::Embrace if at(0.3) => self.audio.play(Sfx::Wind),
            Shot::Light if at(ascent::LIGHT_TOUCH) => self.audio.play(Sfx::Soul),
            // The credits roll to their own song: the hymn is let go as it comes in, and it fades
            // out with the last of them before the title.
            Shot::Credits if at(CREDITS_SONG_IN) => {
                self.audio.hymn_release(CREDITS_HANDOVER);
                self.audio.set_soundtrack(Some(Soundtrack::Credits));
            }
            Shot::Credits if at(credits_hold() - CREDITS_SONG_OUT) => self.audio.hush(),
            _ => {}
        }
        if let Some(st) = &self.story {
            let now = st.scene_time();
            let before = now - (t - prev);
            if let Some(bar) = bar_between(before, now) {
                self.audio.hymn_bar(bar);
            }
        }
    }

    /// Esc during the end: the picture goes to black, the hymn is let go, and the title follows.
    pub(super) fn skip_ending(&mut self) {
        if let Some(st) = self.story.as_mut() {
            st.ending = Some((ENDING_SKIP_FADE.max(SKIP_FADE), ENDING_SKIP_FADE.max(SKIP_FADE)));
        }
        self.audio.hymn_release(ENDING_SKIP_FADE * 1.5);
        self.audio.hush();
    }

    /// The end is over: back to the title, its music, and its mark.
    pub(super) fn finish_ending(&mut self) {
        self.audio.hymn_release(2.5);
        self.enter(Screen::Title);
        self.juice.fade = 1.0;
        for a in [Action::Confirm, Action::Jump, Action::Attack, Action::Tap, Action::Pause] {
            self.input.consume(a);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::STEP;
    use crate::logic::story::{SCENE_TWO, WORDS};

    #[test]
    fn the_hymn_begins_with_the_voices_and_sounds_each_bar_once() {
        let start = hymn_start();
        let embrace = SCENE_TWO[0].hold;
        assert!((start - (embrace + ascent::VOICES as f64)).abs() < 1e-9);
        assert_eq!(bar_between(start - 0.01, start), Some(0));
        assert_eq!(bar_between(start, start + 0.01), None);
        assert_eq!(bar_between(start + hymn::BAR - 0.01, start + hymn::BAR + 0.01), Some(1));
        // Bar by bar, frame by frame, each heard once and in order, and all of them before the credits end.
        let mut heard = Vec::new();
        let total = Scene::Two.time_at(SCENE_TWO.len(), 0.0);
        let mut t = 0.0;
        while t < total {
            if let Some(b) = bar_between(t, t + STEP) {
                heard.push(b);
            }
            t += STEP;
        }
        assert_eq!(heard, (0..hymn::SCORE.len()).collect::<Vec<_>>());
    }

    #[test]
    fn the_end_plays_through_its_beats_to_the_credits_and_then_the_title() {
        let mut g = Game::new();
        g.begin_ending();
        assert_eq!(g.screen, Screen::Story);
        assert!(g.finished, "the game is marked finished as the end begins");
        let mut seen: Vec<Shot> = Vec::new();
        let mut starts: Vec<f64> = Vec::new();
        let mut clock = 0.0;
        for _ in 0..(200.0 / STEP) as usize {
            if g.screen != Screen::Story {
                break;
            }
            let st = g.story.as_ref().unwrap();
            assert_eq!(st.scene, Scene::Two);
            if seen.last() != Some(&st.beat().shot) {
                seen.push(st.beat().shot);
                starts.push(clock);
            }
            g.update(STEP);
            clock += STEP;
        }
        assert_eq!(g.screen, Screen::Title, "the title follows the credits");
        assert!(g.story.is_none());
        assert_eq!(seen, vec![Shot::Embrace, Shot::Light, Shot::Rise, Shot::White, Shot::Words, Shot::Credits]);
        // Each beat begins when the ones before it have run their time.
        for (i, &at) in starts.iter().enumerate() {
            let due = Scene::Two.time_at(i, 0.0);
            assert!((at - due).abs() < 3.0 * STEP, "beat {i} began at {at:.2}s, not {due:.2}s");
        }
        assert!(WORDS.iter().all(|w| w.is_ascii()));
    }

    #[test]
    fn esc_skips_the_end_to_the_title_and_a_press_lets_the_credits_go() {
        let mut g = Game::new();
        g.begin_ending();
        for _ in 0..30 {
            g.update(STEP);
        }
        g.input.inject(Action::Pause);
        g.update(STEP);
        for _ in 0..(2.0 / STEP) as usize {
            g.update(STEP);
        }
        assert_eq!(g.screen, Screen::Title, "Esc skips the end");
        assert!(g.finished, "a skipped end still counts as finished");

        // In the credits, Enter lets them go (once they have been up a moment).
        let credits = Scene::Two.time_at(crate::logic::story::first(Shot::Credits).unwrap(), 3.0);
        g.ending_jump(credits);
        assert_eq!(g.story.as_ref().map(|s| s.beat().shot), Some(Shot::Credits));
        g.input.inject(Action::Confirm);
        g.update(STEP);
        for _ in 0..(2.0 / STEP) as usize {
            g.update(STEP);
        }
        assert_eq!(g.screen, Screen::Title);
    }

    #[test]
    fn the_finished_mark_is_saved_and_read_back() {
        // A save folder of this test's own (the process-wide `VH_SAVE_DIR` is shared with every
        // other test running at the same time).
        platform::use_dir_on_this_thread(std::env::temp_dir().join("vh-test-finished"));
        platform::save(FINISHED_KEY, "0");
        assert!(!finished_saved());
        remember_finished();
        assert!(finished_saved(), "the mark is read back from the save");
        // In play the game sets it as the end begins (and reads it as it starts: `Game::new`).
        let mut g = Game::new();
        assert!(!g.finished);
        g.begin_ending();
        assert!(g.finished);
    }
}
