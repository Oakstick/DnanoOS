//! The story's scenes as they play. Scene I, "The Threshold", comes after Play on the title.
//! Action beats run for their time; each line of dialogue types itself out, and Enter, Space, a
//! click or A shows the rest at once and then moves on. Esc (or Start) skips it. Either way the
//! character screen comes next: the old man has just offered the boys the three warriors' souls.
//! Scene II, "The Way Up", plays the same way at the end (`ending`), and the title follows it.

use super::{Game, Screen};
use crate::audio::{Sfx, Speech};
use crate::input::Action;
use crate::logic::story::{first, pause_after, stage, Beat, Place, Scene, Shot, Speaker, SOULS_LINE};
use crate::render::view::StoryView;

/// Letters a second the typewriter shows.
pub const TYPE_RATE: f64 = 42.0;
/// Seconds after a line's beat begins before it starts to type (the camera cuts first).
pub const TYPE_DELAY: f64 = 0.3;
/// Seconds the picture takes to go to black at the end of the scene.
pub const STORY_FADE: f64 = 0.9;
/// Seconds it takes when the scene is skipped.
pub(super) const SKIP_FADE: f64 = 0.45;

/// The scene in progress.
#[derive(Clone, Debug, Default)]
pub struct Story {
    /// Which scene: the first, or the last.
    pub scene: Scene,
    pub beat: usize,
    /// Seconds into the beat.
    pub t: f64,
    /// Seconds since the scene began.
    pub age: f64,
    /// Letters of the line showing (fractional while the typewriter is partway along one).
    pub shown: f64,
    /// Seconds the typewriter waits before its next letter.
    hold: f64,
    /// Going to black before the character screen: seconds left, and the whole of it.
    pub ending: Option<(f64, f64)>,
    /// Seconds to the next gust of wind or drip of water.
    ambience: f64,
    /// Letters typed so far in this line (every other one makes a sound).
    typed: usize,
    /// Which of the three warriors' souls have risen (bits, in the order he names them).
    pub souls: u8,
}

impl Story {
    /// A scene from its first beat.
    pub(super) fn of(scene: Scene) -> Self {
        Self { scene, ambience: 0.5, ..Default::default() }
    }

    pub fn beat(&self) -> &'static Beat {
        let beats = self.scene.beats();
        &beats[self.beat.min(beats.len() - 1)]
    }

    /// Seconds since the scene began, by its beats (`age` also counts a skip's fade).
    pub fn scene_time(&self) -> f64 {
        self.scene.time_at(self.beat, self.t)
    }

    /// Has the whole line typed out?
    pub fn line_done(&self) -> bool {
        self.shown as usize >= self.beat().line.len()
    }

    /// What the view shows of it.
    pub fn view(&self) -> StoryView {
        StoryView {
            scene: self.scene,
            beat: self.beat,
            t: self.t as f32,
            age: self.age as f32,
            shown: self.shown as f32,
            souls: self.souls,
        }
    }

    /// 0..1 how far the picture has gone to black at the end.
    pub fn fade_out(&self) -> f32 {
        self.ending.map(|(left, all)| (1.0 - left / all).clamp(0.0, 1.0) as f32).unwrap_or(0.0)
    }
}

impl Game {
    /// Starts Scene I from its first beat.
    pub(super) fn begin_story(&mut self) {
        let story = Story { ambience: 0.5, ..Default::default() };
        // Its sets are lit by their own probes: the scene waits behind the loading screen until
        // the renderer has worked them out (RP03), never in the middle of it.
        self.view.story = Some(story.view());
        self.view.showcase = false;
        self.story = Some(story);
        self.enter(Screen::Story);
        self.audio.hush();
        self.audio.set_threat(0.12);
        self.view.prepare_cast(false);
        self.begin_loading("Scene I".into(), web_time::Instant::now());
    }

    /// Jumps Scene I to beat `beat`, `t` seconds in, its line showing in full (captures, tests).
    pub fn story_jump(&mut self, beat: usize, t: f64) {
        if self.story.is_none() {
            self.begin_story();
        }
        if let Some(st) = self.story.as_mut() {
            st.beat = beat.min(st.scene.beats().len() - 1);
            st.t = t;
            st.age = st.age.max(1.0) + t;
            st.shown = st.beat().line.len() as f64;
            st.typed = 0;
            st.ending = None;
            if st.scene == Scene::One && first(Shot::Souls).is_some_and(|souls| st.beat >= souls) {
                st.souls = 0b111;
            }
        }
    }

    pub(super) fn update_story(&mut self, dt: f64) {
        let Some(st) = self.story.as_mut() else {
            self.open_select();
            return;
        };
        let scene = st.scene;
        let prev = st.t;
        st.t += dt;
        st.age += dt;
        if let Some((left, _)) = st.ending.as_mut() {
            *left -= dt;
            if *left <= 0.0 {
                self.finish_story();
            }
            return;
        }
        if self.input.was_pressed(&[Action::Pause]) {
            st.ending = Some((SKIP_FADE, SKIP_FADE));
            if scene == Scene::Two {
                self.skip_ending();
            } else {
                self.audio.play(Sfx::Whoosh);
            }
            return;
        }
        let beat = *st.beat();
        let t = st.t;

        // The typewriter: a letter at a time, holding a moment at the stops.
        let len = beat.line.len();
        let mut letters = 0;
        if len > 0 && t > TYPE_DELAY && !st.line_done() {
            if st.hold > 0.0 {
                st.hold -= dt;
            } else {
                let from = st.shown as usize;
                st.shown = (st.shown + dt * TYPE_RATE).min(len as f64);
                for i in from..(st.shown as usize) {
                    if beat.line.as_bytes()[i].is_ascii_alphabetic() {
                        letters += 1;
                    }
                    let pause = pause_after(beat.line, i);
                    if pause > 0.0 {
                        st.shown = (i + 1) as f64;
                        st.hold = pause;
                        break;
                    }
                }
            }
        }
        let pressed = self.input.was_pressed(&[Action::Confirm, Action::Jump, Action::Tap]);
        let mut next = false;
        if beat.waits() {
            if pressed && t > 0.15 {
                if st.line_done() {
                    next = true;
                } else {
                    st.shown = len as f64;
                    st.hold = 0.0;
                }
            }
        } else if t >= beat.hold {
            next = true;
        }
        // At the end, a press once the credits are rolling lets them go.
        let done = beat.shot == Shot::Credits && pressed && t > 2.0;
        if done {
            st.ending = Some((STORY_FADE, STORY_FADE));
        }
        st.ambience -= dt;
        // (Scene II has no wind or water: the voices are the first gentle sound in a long while.)
        let gust = st.ambience <= 0.0 && scene == Scene::One;
        if gust {
            st.ambience = 2.5 + (st.age * 7.31).sin().abs() * 3.0;
        }
        let blips = (st.typed + letters) / 2 != st.typed / 2;
        st.typed += letters;
        // Each warrior's soul rises as the old man names him.
        let mut risen = 0;
        if beat.shot == Shot::Souls {
            for (bit, name) in ["Ragnar", "Mehmed", "Miyamoto"].iter().enumerate() {
                let named = SOULS_LINE.find(name).is_some_and(|i| st.shown as usize > i);
                if named && st.souls & (1 << bit) == 0 {
                    st.souls |= 1 << bit;
                    risen += 1;
                }
            }
        }
        self.input.consume(Action::Attack);

        if blips {
            self.audio.play_voice(match beat.speaker {
                Speaker::Boys => Speech::Child,
                Speaker::OldMan => Speech::Old,
                Speaker::Dog => Speech::Beast,
                Speaker::Narrator => Speech::Narrator,
            });
        }
        if gust {
            // The wind over the graves up above; water dripping in the dungeon below.
            self.audio.play(if beat.shot.place() == Place::Graveyard { Sfx::Wind } else { Sfx::Drip });
        }
        for _ in 0..risen {
            self.audio.play(Sfx::Soul);
        }
        if scene == Scene::Two {
            self.ending_cues(beat.shot, prev, t);
        } else {
            self.story_cues(beat.shot, prev, t);
        }
        if next && !done {
            self.next_beat();
        }
    }

    /// Sounds pinned to moments of the action.
    fn story_cues(&mut self, shot: Shot, prev: f64, t: f64) {
        let at = |when: f32| prev < when as f64 && t >= when as f64;
        match shot {
            Shot::Title if at(0.4) => self.audio.play(Sfx::Toll),
            Shot::Establish => {
                if at(0.9) {
                    self.audio.play(Sfx::Caw);
                }
                if stage::KICKS.iter().chain([&stage::MISKICK]).any(|&k| at(k)) {
                    self.audio.play(Sfx::Kick);
                }
                if at(stage::BALL_AT_GATE) {
                    self.audio.play(Sfx::Clang);
                }
                if at(stage::AT_GATE + 0.5) {
                    self.audio.play(Sfx::Bells);
                }
            }
            Shot::Taken => {
                if at(stage::BELLS) {
                    self.audio.play(Sfx::Bells);
                }
                if at(stage::EYES) {
                    self.audio.play(Sfx::Caw);
                }
                if at(stage::RUSH) {
                    self.audio.play(Sfx::Shriek);
                }
                if at(stage::GATE_BURST) {
                    self.audio.play(Sfx::Clang);
                }
                if at(stage::GRAB) {
                    self.audio.play(Sfx::Stinger);
                }
                if at(stage::TURN + 0.2) {
                    self.audio.play(Sfx::Bells);
                }
            }
            Shot::Pursuit => {
                if at(0.2) {
                    self.audio.play(Sfx::Whoosh);
                }
                if at(stage::FLASH) {
                    self.audio.play(Sfx::Portal);
                }
            }
            Shot::Arrive => {
                if stage::TUMBLE.iter().any(|&k| at(k + 0.25)) {
                    self.audio.play(Sfx::Land);
                }
                if at(stage::CLOSE) {
                    self.audio.play(Sfx::Portal);
                }
                if at(stage::GROWL) {
                    self.audio.play(Sfx::Growl);
                }
                if at(stage::GROWL + 0.3) {
                    self.audio.play(Sfx::Chain);
                }
            }
            Shot::Snarl if at(0.05) => {
                self.audio.play(Sfx::Growl);
                self.audio.play(Sfx::Chain);
            }
            Shot::Easy if at(0.9) => self.audio.play(Sfx::Whimper),
            Shot::Descent => {
                if at(stage::SOULS_TAKEN) {
                    self.audio.play(Sfx::Soul);
                }
                if at(stage::DOOR) {
                    self.audio.play(Sfx::DoorGrind);
                }
            }
            _ => {}
        }
    }

    fn next_beat(&mut self) {
        let Some(st) = self.story.as_mut() else { return };
        st.beat += 1;
        st.t = 0.0;
        st.shown = 0.0;
        st.hold = 0.0;
        st.typed = 0;
        let len = st.scene.beats().len();
        if st.beat >= len {
            st.beat = len - 1;
            st.t = if st.scene == Scene::Two { st.beat().hold } else { 30.0 };
            st.shown = st.beat().line.len() as f64;
            st.ending = Some((STORY_FADE, STORY_FADE));
            return;
        }
        if st.scene == Scene::One && Some(st.beat) == first(Shot::Arrive) {
            self.audio.set_threat(0.25);
        }
    }

    /// Off to the character screen (or, after the last scene, the title).
    fn finish_story(&mut self) {
        let scene = self.story.take().map(|s| s.scene).unwrap_or_default();
        self.audio.set_threat(0.0);
        if scene == Scene::Two {
            self.finish_ending();
            return;
        }
        self.open_select();
        self.juice.fade = 1.0;
    }
}
