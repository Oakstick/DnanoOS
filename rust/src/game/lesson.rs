//! Hızır's lesson (B04): early in the Garden, before the first real pack, Hızır comes as he does
//! when called, Quitmer at his heel, and teaches the duel on his old sparring draugr, which cannot
//! kill: the three-hit string, the guard, the parry, the leap, and the three tells. Each step is a
//! line of his in the panel (in his voice) and a card at the top with the buttons as they are bound
//! (`render::hud::lesson`); the step is done when the player does it, and tried again, gently, when
//! it goes wrong. Esc (Start) skips it; it is shown once, and is taught again from his training
//! yard (call him) or from his guide (B) in the Garden. The LEVELS map has a stop for it.

use super::{Game, Screen};
use crate::audio::Sfx;
use crate::logic::combat::duelist::Den;
use crate::logic::combat::fsm::Defense;
use crate::logic::combat::{math, MoveId, State};
use crate::logic::enemy::EnemyKind;
use crate::logic::types::{P, V};
use crate::logic::war::WarEvent;
use crate::platform;

/// The settings key that records the lesson has been seen.
const LESSON_KEY: &str = "vh.lesson_done";
/// Tiles from him the sparring draugr stands, and Hızır off to the side.
const FOE_AHEAD: f64 = 2.0;
const HIZIR_SIDE: f64 = 2.4;
/// Seconds between the draugr's blows while it is his turn to strike.
const CUE: f64 = 1.6;
/// Seconds before a step with nothing done nudges him again.
const NUDGE: f64 = 14.0;
/// Seconds the greeting and the farewell hold.
const GREET: f64 = 4.5;
const FAREWELL: f64 = 5.0;
/// The sparring draugr's health: it is not to be felled.
const SPARRING_HP: f64 = 100_000.0;

/// The lesson's steps.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step {
    Greet,
    Strike,
    Guard,
    Parry,
    Dodge,
    Tells,
    Farewell,
}

impl Step {
    /// The five taught, in order (the greeting and the farewell are not counted).
    pub const TAUGHT: [Step; 5] = [Step::Strike, Step::Guard, Step::Parry, Step::Dodge, Step::Tells];

    /// Its number among the five (1 to 5), if it is one of them.
    pub fn number(self) -> Option<usize> {
        Self::TAUGHT.iter().position(|&s| s == self).map(|k| k + 1)
    }

    pub fn title(self) -> &'static str {
        match self {
            Step::Greet => "HIZIR'S LESSON",
            Step::Strike => "STRIKE",
            Step::Guard => "GUARD",
            Step::Parry => "PARRY",
            Step::Dodge => "DODGE",
            Step::Tells => "THE THREE TELLS",
            Step::Farewell => "WELL DONE",
        }
    }

    /// What Hızır says as the step begins.
    pub fn line(self) -> &'static str {
        match self {
            Step::Greet => "Wait, child. Before the keys, a little lesson. My old sparring draugr cannot truly hurt you.",
            Step::Strike => "Strike him three times running. One, two, three: the third blow is the heavy one.",
            Step::Guard => "Now hold up your shield. A block stops the blow, but you rock back a moment, and it costs you breath.",
            Step::Parry => "Now raise it just as the blow lands. Not early, not late. That is a parry: it throws him open.",
            Step::Dodge => "Leap through the next one. While you are in the air, nothing can touch you.",
            Step::Tells => "Read his signs. A glow as he winds up, a flash as he strikes. A red glint means no shield will hold it: leap!",
            Step::Farewell => "Well done. Now go and find the keys. Call me whenever you need me, and I will come.",
        }
    }

    /// The card's prompt: the buttons, by their tokens (`render::hud::glyphs`).
    pub fn prompt(self) -> &'static str {
        match self {
            Step::Greet => "{Confirm} begin",
            Step::Strike => "{Attack} {Attack} {Attack}  the three-hit string",
            Step::Guard => "hold {Block}  guard",
            Step::Parry => "{Block} as the blow lands  parry",
            Step::Dodge => "{Jump} as he swings  leap",
            Step::Tells => "glow, flash: guard     red glint: {Jump}",
            Step::Farewell => "{Call} call Hızır",
        }
    }

    fn next(self) -> Step {
        match self {
            Step::Greet => Step::Strike,
            Step::Strike => Step::Guard,
            Step::Guard => Step::Parry,
            Step::Parry => Step::Dodge,
            Step::Dodge => Step::Tells,
            Step::Tells | Step::Farewell => Step::Farewell,
        }
    }
}

/// How the player met one of the draugr's blows, as the lesson reads it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Met {
    Done,
    /// Too early, too late, or the wrong answer: the line that says so.
    Again(&'static str),
    /// Nothing to say (a blow that does not count for this step).
    Nothing,
}

/// The lesson under way.
#[derive(Clone, Debug)]
pub struct Lesson {
    pub step: Step,
    /// Seconds into the step, and since the lesson began.
    pub t: f64,
    pub age: f64,
    /// The sparring draugr's duelist id.
    pub foe: u32,
    /// Where Hızır and Quitmer stand, and which way they face (radians in the ground plane).
    pub hizir: V,
    pub dog: V,
    pub facing: f64,
    /// Seconds until the draugr's next blow.
    cue: f64,
    /// Tries that went wrong in this step (the card counts them).
    pub misses: u32,
    /// In the tells: the plain blow has been shown (the red one comes next).
    shown_plain: bool,
    /// Seconds since he last spoke (a nudge waits `NUDGE`).
    quiet: f64,
    /// The last thing that went wrong, for the card.
    pub hint: Option<&'static str>,
}

/// Has this player been taught yet?
pub fn lesson_seen() -> bool {
    platform::load(LESSON_KEY).map(|v| v.trim() == "1").unwrap_or(false)
}

/// Starting over: the lesson is owed again.
pub fn forget_lesson() {
    platform::save(LESSON_KEY, "0");
}

impl Game {
    /// The lesson holds the Garden still (no hunters come while Hızır teaches).
    pub(super) fn lesson_holds(&self) -> bool {
        self.lesson.is_some()
    }

    /// While Hızır teaches, nothing slays him (the sparring draugr's blows still land and hurt).
    pub(super) fn lesson_spares(&self) -> bool {
        self.lesson.is_some()
    }

    /// No lesson (the harnesses, and a player who skips it): it is put away and not owed.
    pub fn skip_lesson(&mut self) {
        self.lesson_pending = false;
        if self.lesson.is_some() {
            self.end_lesson(false);
        }
    }

    /// The lesson again (his training yard, his guide, the LEVELS map): in the Garden's fight only.
    pub fn replay_lesson(&mut self) -> bool {
        if !self.in_garden_fight() {
            return false;
        }
        if self.lesson.is_some() {
            self.end_lesson(false);
        }
        self.lesson_pending = true;
        true
    }

    /// Where the lesson stands: open floor nearest `want`, else his own tile.
    fn lesson_spot(&self, want: V) -> V {
        let g = &self.level.grid;
        let hp = self.hero.occupied();
        let mut best = (f64::MAX, hp.v());
        for dy in -3..=3 {
            for dx in -3..=3 {
                let p = P::new(want.x.round() as i32 + dx, want.y.round() as i32 + dy);
                if !g.is_walkable(p.x, p.y, false) || g.is_bridge(p.x, p.y) || p == hp {
                    continue;
                }
                let d = crate::logic::types::dist_sq(p.v(), want);
                if d < best.0 {
                    best = (d, p.v());
                }
            }
        }
        best.1
    }

    /// Hızır comes, Quitmer at his heel, and his sparring draugr climbs out of the turf before the hero.
    pub(super) fn begin_lesson(&mut self) {
        self.lesson_pending = false;
        let hp = self.hero.pos();
        let aim = self.hero.aim();
        let side = V::new(-aim.y, aim.x);
        let foe_at = self.lesson_spot(math::add(hp, math::scale(aim, FOE_AHEAD)));
        let hizir = self.lesson_spot(math::add(math::add(hp, math::scale(side, HIZIR_SIDE)), math::scale(aim, 0.6)));
        let dog = self.lesson_spot(math::add(hizir, math::scale(aim, -1.0)));
        let facing = math::angle(math::sub(math::scale(math::add(hp, foe_at), 0.5), hizir));
        let to_hero = math::norm(math::sub(hp, foe_at));
        let lore = crate::logic::combat::duelist::Lore { rise: crate::logic::combat::duelist::RISE_FRAMES, ..Default::default() };
        let Some(id) = self.spawn_duelist(EnemyKind::Skeleton, foe_at, to_hero, lore) else { return };
        if let Some(d) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == id) {
            // Its strings are the lesson's: it strikes when he is to meet a blow, and is never felled.
            d.puppet = true;
            d.breed.tell = super::garden_fight::GARDEN_TELL;
            d.f.tell = super::garden_fight::GARDEN_TELL;
            d.breed.damage *= super::garden_fight::GARDEN_DAMAGE;
            (d.max_hp, d.hp) = (SPARRING_HP, SPARRING_HP);
            d.den = None::<Den>;
        }
        if let Some(e) = self.enemies.iter_mut().find(|e| e.duel.as_ref().is_some_and(|d| d.id == id)) {
            e.b.name = "HIZIR'S SPARRING DRAUGR".into();
        }
        self.lesson = Some(Lesson {
            step: Step::Greet,
            t: 0.0,
            age: 0.0,
            foe: id,
            hizir,
            dog,
            facing,
            cue: CUE,
            misses: 0,
            shown_plain: false,
            quiet: 0.0,
            hint: None,
        });
        self.hero.hp = self.hero.max_hp;
        self.audio.play(Sfx::Portal);
        self.audio.play(Sfx::Bark);
        self.view.sparkle(&self.level.grid, hizir.x, hizir.y, 1.2, 0xffd890, 40);
        self.view.dust(&self.level.grid, foe_at.x, foe_at.y, 0x6a5a48, 16, 1.4);
        self.say_lesson(Step::Greet.line());
    }

    /// A line of Hızır's in the panel, in his voice (at once: it is his lesson).
    fn say_lesson(&mut self, line: &str) {
        self.voices.queue.clear();
        self.voices.spoken = Some(super::counsel::Spoken::new(super::counsel::Speaker::OldMan, line.to_string(), self.time));
        if let Some(l) = self.lesson.as_mut() {
            l.quiet = 0.0;
        }
    }

    /// The sparring draugr, if it stands.
    fn lesson_foe(&mut self) -> Option<&mut crate::logic::combat::Duelist> {
        let id = self.lesson.as_ref()?.foe;
        self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == id && d.alive())
    }

    /// The lesson's step: what he did, read from this frame's war log; the draugr kept facing him,
    /// close, whole; its blows when it is his turn to meet one.
    pub(super) fn update_lesson(&mut self, dt: f64) {
        if self.lesson.is_none() {
            if self.lesson_pending && self.screen == Screen::Playing && self.in_garden_fight() {
                self.begin_lesson();
            }
            return;
        }
        let hp = self.hero.pos();
        // The draugr: facing him, a step from him, whole.
        let grid = self.level.grid.clone();
        let Some(foe) = self.lesson_foe() else {
            self.end_lesson(false);
            return;
        };
        let to = math::sub(hp, foe.pos);
        let d = math::len(to);
        if d > 0.05 {
            let dir = math::norm(to);
            if !matches!(foe.f.state, State::Attack { .. }) {
                foe.facing = dir;
                foe.aim = dir;
            }
            let step = math::scale(dir, (d - 1.4).min(2.2 * dt));
            let to = math::add(foe.pos, step);
            let open = grid.is_walkable(to.x.round() as i32, to.y.round() as i32, false);
            if d > 1.5 && open && foe.lore.rise == 0 && matches!(foe.f.state, State::Neutral) {
                foe.vel = math::scale(step, 1.0 / dt.max(1e-6));
                foe.pos = to;
            } else {
                foe.vel = V::new(0.0, 0.0);
            }
        }
        foe.hp = foe.max_hp;
        let foe_id = foe.id;
        let rising = foe.lore.rise > 0;
        let neutral = matches!(foe.f.state, State::Neutral);
        // What he did this frame.
        let events: Vec<WarEvent> = self.war.log.clone();
        let (step, t) = {
            let l = self.lesson.as_mut().unwrap();
            l.t += dt;
            l.age += dt;
            l.quiet += dt;
            (l.step, l.t)
        };
        match step {
            Step::Greet => {
                if t > GREET || self.input.was_pressed(&[crate::input::Action::Confirm]) {
                    self.next_step();
                }
            }
            Step::Strike => {
                let finished = events.iter().any(|e| matches!(e, WarEvent::HeroHit { foe, finisher: true, .. } if *foe == foe_id));
                if finished {
                    self.next_step();
                } else if self.lesson.as_ref().is_some_and(|l| l.quiet > NUDGE) {
                    self.lesson_miss("Keep going. One, two, three, without stopping.");
                }
            }
            Step::Farewell => {
                if t > FAREWELL {
                    self.end_lesson(true);
                }
            }
            _ => {
                // His turn to meet the draugr's blows.
                let mut met = Met::Nothing;
                for e in &events {
                    if let WarEvent::Landed { foe, how, bolt: false, .. } = *e {
                        if foe == foe_id {
                            met = self.lesson_met(step, how);
                        }
                    }
                }
                match met {
                    Met::Done => self.next_step(),
                    Met::Again(line) => self.lesson_miss(line),
                    Met::Nothing => {}
                }
                // The next blow, when the last has run its course.
                let l = self.lesson.as_mut().unwrap();
                if neutral && !rising {
                    l.cue -= dt;
                }
                if l.cue <= 0.0 && neutral && !rising && t > 1.2 {
                    l.cue = CUE;
                    let mv = if step == Step::Tells && l.shown_plain { MoveId::Grab } else { MoveId::Slam };
                    if step == Step::Tells {
                        l.shown_plain = !l.shown_plain;
                    }
                    if let Some(f) = self.lesson_foe() {
                        f.token = true;
                        f.begin(mv, hp);
                    }
                    if let Some(f) = self.lesson_foe() {
                        let (kind, at) = (f.breed.kind, f.pos);
                        let listener = self.audio.listener();
                        self.audio.tell(kind, mv, at, listener);
                    }
                }
            }
        }
    }

    /// How he met the draugr's blow, for this step.
    fn lesson_met(&self, step: Step, how: Defense) -> Met {
        let red = matches!(self.lesson.as_ref(), Some(l) if !l.shown_plain) && step == Step::Tells;
        match (step, how) {
            (_, Defense::Immune) => Met::Nothing,
            (Step::Guard, Defense::Blocked | Defense::Parried) => Met::Done,
            (Step::Guard, _) => Met::Again("Up before the blow falls, and hold it there."),
            (Step::Parry, Defense::Parried) => Met::Done,
            (Step::Parry, Defense::Blocked) => Met::Again("Too soon. Wait for the flash, then raise it."),
            (Step::Parry, Defense::Evaded) => Met::Again("No leaping now. Meet it with the shield, as it lands."),
            (Step::Parry, _) => Met::Again("A breath late. Raise it as the club comes down."),
            (Step::Dodge, Defense::Evaded) => Met::Done,
            (Step::Dodge, _) => Met::Again("Leap as he swings, not before, and not behind your shield."),
            // (The red blow: `shown_plain` has flipped back by the time it lands.)
            (Step::Tells, Defense::Evaded) if red => Met::Done,
            (Step::Tells, _) if red => Met::Again("When you see red, do not guard. Leap!"),
            (Step::Tells, _) => Met::Nothing,
            _ => Met::Nothing,
        }
    }

    /// The step went wrong: a gentle word, and the draugr waits a breath before the next try.
    fn lesson_miss(&mut self, line: &'static str) {
        if let Some(l) = self.lesson.as_mut() {
            l.misses += 1;
            l.hint = Some(line);
            l.cue = CUE + 0.6;
            if l.step == Step::Tells {
                l.shown_plain = false;
            }
        }
        self.hero.hp = self.hero.hp.max(self.hero.max_hp / 2);
        self.audio.play(Sfx::Nodule);
        self.say_lesson(line);
    }

    /// On to the next step: praise, his line, the card turns over.
    fn next_step(&mut self) {
        let Some(l) = self.lesson.as_mut() else { return };
        let next = l.step.next();
        let was = l.step;
        l.step = next;
        l.t = 0.0;
        l.misses = 0;
        l.hint = None;
        l.cue = CUE * 0.5;
        l.shown_plain = false;
        if was != Step::Greet {
            let hp = self.hero.pos();
            self.audio.play(Sfx::Milestone);
            self.floater(hp.x, hp.y - 1.2, "GOOD!", 1.2, Some(0xffd060));
        }
        self.hero.hp = self.hero.max_hp;
        self.hero.stamina = crate::logic::progression::STAMINA_MAX;
        self.say_lesson(next.line());
    }

    /// The lesson ends (taught, or put away): the draugr crumbles, Hızır is gone, the Garden goes on.
    pub(super) fn end_lesson(&mut self, taught: bool) {
        let Some(l) = self.lesson.take() else { return };
        if !taught {
            // (Put away: his line goes with him.)
            self.voices.spoken = None;
        }
        if let Some(i) = self.enemies.iter().position(|e| e.duel.as_ref().is_some_and(|d| d.id == l.foe)) {
            let p = self.enemies[i].pos();
            self.enemies.remove(i);
            self.view.clear_enemies();
            for e in &self.enemies {
                self.view.add_enemy(e);
            }
            self.view.dust(&self.level.grid, p.x, p.y, 0xc8c0b0, 14, 1.6);
        }
        if let Some(b) = self.brawl.as_mut() {
            b.threat.release(l.foe);
            b.bolts.clear();
        }
        self.view.sparkle(&self.level.grid, l.hizir.x, l.hizir.y, 1.2, 0xffd890, 30);
        self.audio.play(Sfx::Whoosh);
        self.spawn_timer = super::SPAWN_FIRST;
        self.hero.hp = self.hero.max_hp;
        // (Taught or skipped, it is not owed again; his yard and his guide teach it again.)
        let _ = taught;
        platform::save(LESSON_KEY, "1");
    }

    /// Hızır and Quitmer as the view draws them.
    pub fn lesson_view(&self) -> Option<crate::render::view::lesson::LessonView> {
        let l = self.lesson.as_ref()?;
        let talk = self.voices.spoken.as_ref().filter(|s| s.who == super::counsel::Speaker::OldMan && !s.done(self.time));
        Some(crate::render::view::lesson::LessonView {
            hizir: (l.hizir.x, l.hizir.y),
            dog: (l.dog.x, l.dog.y),
            facing: l.facing,
            age: l.age as f32,
            talk: talk.map(|s| (self.time - s.since) as f32),
        })
    }

    /// Capture hook: the lesson (begun if it is not) at step `n` (0 the greeting .. 6 the farewell).
    pub fn debug_lesson(&mut self, n: usize) {
        if self.lesson.is_none() && self.in_garden_fight() && self.screen == Screen::Playing {
            self.begin_lesson();
        }
        let steps = [Step::Greet, Step::Strike, Step::Guard, Step::Parry, Step::Dodge, Step::Tells, Step::Farewell];
        let step = steps[n.min(6)];
        if let Some(l) = self.lesson.as_mut() {
            l.step = step;
            l.t = 0.0;
            l.cue = 0.3;
        }
        self.say_lesson(step.line());
    }

    /// Esc (Start) during the lesson: it is put away (and not owed again).
    pub(super) fn lesson_wants_skip(&self) -> bool {
        self.lesson.is_some() && self.screen == Screen::Playing
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Action;
    const STEP: f64 = crate::app::STEP;

    /// The Garden with the lesson begun.
    fn taught() -> Game {
        let mut g = super::super::tests::playing();
        assert!(g.in_garden_fight());
        g.lesson_pending = true;
        g.update(STEP);
        assert!(g.lesson.is_some(), "the lesson begins");
        g
    }

    fn step_of(g: &Game) -> Step {
        g.lesson.as_ref().unwrap().step
    }

    fn frames(g: &mut Game, secs: f64) {
        for _ in 0..(secs / STEP) as usize {
            g.update(STEP);
        }
    }

    #[test]
    fn hizir_comes_with_his_sparring_draugr_and_esc_puts_the_lesson_away() {
        let mut g = taught();
        let l = g.lesson.clone().unwrap();
        assert_eq!(l.step, Step::Greet);
        let d = g.enemies.iter().filter_map(|e| e.duel.as_deref()).find(|d| d.id == l.foe).expect("his sparring draugr");
        assert!(d.puppet && d.breed.kind == EnemyKind::Skeleton);
        assert!(g.voices.spoken.as_ref().is_some_and(|s| s.text == Step::Greet.line()), "he speaks");
        assert!(g.lesson_view().is_some(), "and is drawn, Quitmer by him");
        // No hunter comes while he teaches.
        frames(&mut g, 30.0);
        assert!(g.enemies.iter().filter_map(|e| e.duel.as_deref()).all(|d| d.den.is_some() || d.puppet), "no hunters");
        assert!(g.lesson.is_some(), "still teaching");
        g.input.inject(Action::Pause);
        g.update(STEP);
        assert!(g.lesson.is_none() && !g.lesson_pending, "put away");
        assert_eq!(g.screen, Screen::Playing, "not paused");
        assert!(g.enemies.iter().filter_map(|e| e.duel.as_deref()).all(|d| !d.puppet), "the draugr is gone");
        // His training yard (or his guide) teaches it again, in the Garden.
        assert!(g.replay_lesson());
        g.update(STEP);
        assert!(g.lesson.is_some());
    }

    #[test]
    fn the_steps_go_on_as_he_does_them_and_a_wrong_answer_is_tried_again() {
        let mut g = taught();
        frames(&mut g, GREET + 0.2);
        assert_eq!(step_of(&g), Step::Strike);
        // The three-hit string, landed.
        for _ in 0..240 {
            g.input.inject(Action::Attack);
            g.update(STEP);
            if step_of(&g) != Step::Strike {
                break;
            }
        }
        assert_eq!(step_of(&g), Step::Guard, "the finisher lands");
        // Open to the blow: the guard is asked again; behind his shield: done.
        let hp = g.hero.hp;
        frames(&mut g, 8.0);
        assert!(g.lesson.as_ref().unwrap().misses >= 1 && step_of(&g) == Step::Guard, "struck open, tried again");
        assert!(g.hero.alive && g.hero.hp > 0, "it cannot slay him ({hp})");
        for _ in 0..(5.0 / STEP) as usize {
            g.input.inject(Action::Block);
            g.update(STEP);
            if step_of(&g) != Step::Guard {
                break;
            }
        }
        assert_eq!(step_of(&g), Step::Parry, "blocked");
        // The parry and the tells read their answers.
        assert_eq!(g.lesson_met(Step::Parry, Defense::Blocked), Met::Again("Too soon. Wait for the flash, then raise it."));
        assert_eq!(g.lesson_met(Step::Parry, Defense::Parried), Met::Done);
        assert_eq!(g.lesson_met(Step::Dodge, Defense::Evaded), Met::Done);
        assert!(matches!(g.lesson_met(Step::Dodge, Defense::Blocked), Met::Again(_)));
    }

    #[test]
    fn nothing_slays_him_while_he_is_taught() {
        let mut g = taught();
        g.hero.hp = 1;
        g.wound_hero(50.0, 0xff0000);
        assert!(g.hero.alive && g.hero.hp == 1 && g.screen == Screen::Playing);
    }
}
