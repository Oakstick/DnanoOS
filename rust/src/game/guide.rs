//! The old man's guide (B): play pauses, and he tells the hero about the place he stands in: its
//! tale, what he must do, what to watch for (`logic::guide`), and what is so here now (the realm's
//! general, the keys held). B, Esc, Enter or a click put it away and play goes on.

use super::{Game, Screen};
use crate::audio::Sfx;
use crate::input::Action;
use crate::logic::guide::{self, Guide, GuidePlace};
use crate::logic::level::Theme;

impl Game {
    /// Where he stands, as the guide tells it.
    pub fn guide_place(&self) -> GuidePlace {
        match self.level.data.theme {
            Theme::Realm(sin) => GuidePlace::Realm(sin),
            Theme::Court(sin) => GuidePlace::Court(sin),
            Theme::Hell => GuidePlace::Hub,
            Theme::Gym => GuidePlace::Gym,
            Theme::Pit => GuidePlace::Pit(self.pit.as_ref().map(|p| p.level).unwrap_or(1)),
            Theme::Meadow => GuidePlace::Cyclops(self.dungeon.min(2)),
            Theme::Estate => match self.dungeon {
                0 => GuidePlace::Garden,
                1 => GuidePlace::Cisterns,
                _ => GuidePlace::BoneVaults,
            },
        }
    }

    /// What the old man says of the place he stands in, and what is so here now.
    pub fn guide_now(&self) -> (Guide, Vec<String>) {
        let place = self.guide_place();
        let g = guide::guide(place, self.hero_kind);
        let mut now = Vec::new();
        match place {
            GuidePlace::Hub => {
                let keys = self.sins_cleared.count_ones();
                now.push(format!(
                    "You hold {keys} of the seven keys. Each realm you clear makes the next one harder, so choose your road."
                ));
            }
            GuidePlace::Realm(sin) => {
                if let Some(r) = self.general.as_ref().filter(|r| r.sin == sin) {
                    let name = super::general::title_case(&r.name);
                    if r.command.slain {
                        now.push(format!("{name} is slain. His orders died with him, and his prince is weaker for it."));
                    } else {
                        let left = (0..3).filter(|&k| !r.command.instruments[k].broken).count();
                        let standing = match left {
                            3 => "All three of his instruments still stand".to_string(),
                            0 => "Every one of his instruments is broken".to_string(),
                            n => format!("{n} of his three instruments still stand"),
                        };
                        now.push(format!("{name} holds this realm for his prince. {standing}: break them and his orders fail."));
                        if r.post_known {
                            now.push("His herald's map marks his post. Strike him there, and his prince fights the weaker.".into());
                        }
                    }
                }
                let rivals = self.war.grudges.seated.len();
                if rivals > 0 {
                    now.push(format!(
                        "{} you have met before {} here, at the head of a pack. Their names are in the Ledger.",
                        if rivals == 1 { "One" } else { "Some" },
                        if rivals == 1 { "waits" } else { "wait" }
                    ));
                }
            }
            GuidePlace::Pit(_) => {
                if let Some((_, left, on, waiting, came)) = self.pit_status() {
                    let secs = left.ceil() as u32;
                    now.push(match on {
                        0 => format!("{secs} seconds until the platform goes down. Nothing stands on it now."),
                        1 => format!("{secs} seconds until the platform goes down. One foe stands on it: it rides down with you if it is still there."),
                        n => format!("{secs} seconds until the platform goes down. {n} foes stand on it: they ride down with you if they are still there."),
                    });
                    if waiting > 0 {
                        now.push(format!("{waiting} are still behind the doors."));
                    }
                    if came > 0 {
                        now.push(format!("{came} rode down into this level with you."));
                    }
                }
            }
            _ => {}
        }
        (g, now)
    }

    /// B: play pauses, and the old man speaks.
    pub(super) fn open_guide(&mut self) {
        self.guide_open = true;
        self.enter(Screen::Paused);
        self.guide_open = true;
        self.audio.play(Sfx::Bark);
    }

    /// The guide is up: B, Esc, Enter, A or a click put it away, and play goes on.
    pub(super) fn update_guide(&mut self) {
        let back = [Action::Guide, Action::Pause, Action::Confirm, Action::Jump, Action::Tap];
        if !self.input.was_pressed(&back) && self.input.click.is_none() {
            return;
        }
        for a in back {
            self.input.consume(a);
        }
        self.input.consume(Action::Attack);
        self.input.click = None;
        self.guide_open = false;
        self.audio.play(Sfx::Nodule);
        self.enter(Screen::Playing);
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::playing;
    use super::*;

    /// B pauses play under the old man's guide to the place; B again and play goes on. No bomb.
    #[test]
    fn b_opens_the_old_mans_guide_and_closes_it() {
        let mut g = playing();
        let bombs = g.bombs.len();
        g.input.inject(Action::Guide);
        g.update(crate::app::STEP);
        assert!(g.guide_open && g.screen == Screen::Paused, "the guide is up");
        assert_eq!(g.guide_place(), GuidePlace::Garden);
        assert!(!g.guide_now().0.title.is_empty());
        g.input.inject(Action::Guide);
        g.update(crate::app::STEP);
        assert!(!g.guide_open && g.screen == Screen::Playing, "and put away");
        assert_eq!(g.bombs.len(), bombs, "no bomb dropped");
    }
}
