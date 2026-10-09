//! The bestiary at run time (`logic::bestiary` holds the book and its words): the book is filled
//! as foes are met, felled and fall on the hero, saved (`vh.bestiary`, beside the run but not part
//! of it: Start Over keeps it), and opened as a page over the title or the pause menu (THE
//! BESTIARY), or from the old man's guide (the menus' secondary), to be read group by group.
//!
//! Nothing here is read by the game: the book only watches (the foes in sight, the war log, the
//! Cyclops, the mimic chest, the hero's falls), so the fight is the same with it or without it.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::{Game, MenuItem, Screen};
use crate::audio::Sfx;
use crate::input::{Action, Control};
use crate::logic::bestiary::{self, Book, Entry, Group, BOOK_KEY, GROUPS};
use crate::logic::boss::BossState;
use crate::logic::enemy::{EnemyKind, EnemyMode};
use crate::logic::types::{dist_sq, Dir, P};
use crate::logic::war::WarEvent;
use crate::platform;
use crate::render::hud;

/// A foe in his sight this near (tiles) is met.
const MEET_RANGE: f64 = 12.0;
/// A foe this near is met, seen or not (round a corner, at arm's length).
const MEET_CLOSE: f64 = 3.0;
/// Seconds between looks round for foes met.
const LOOK_EVERY: f64 = 0.2;
/// A fall is laid on the foe that hurt him last, if it did within this many seconds.
const BLAME_SECONDS: f64 = 3.0;
/// A foe touching him (tiles) when he is hurt with no blow in the war log is the one that hurt him.
const TOUCH: f64 = 1.6;
/// Seconds between lines scrolled while the right stick is held.
const SCROLL_REPEAT: f64 = 0.11;

/// Who a foe on the field is, as the book counts it.
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum Subject {
    Page(Entry),
    /// A rival, by his full name.
    Rival(String),
}

/// One page of a group as the book shows it: a fixed page, or a remembered rival (an index into
/// the book's rivals).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Fixed(Entry),
    Rival(usize),
}

/// What the book keeps an eye on between frames (never read by the game).
#[derive(Debug, Default)]
struct Watch {
    /// The level being watched (the fog's epoch and the level's name).
    level: Option<(u32, String)>,
    /// The duelists met on this level, and who each is.
    duels: HashMap<u32, Subject>,
    /// The foes off the frame data met on this level, by their place in the list.
    grid: HashSet<usize>,
    /// Each foe's kind and mode at the last look (for the falls of those off the frame data).
    modes: Vec<Option<(EnemyKind, EnemyMode)>>,
    /// The Cyclops: met on this level, and alive at the last look.
    cyclops_met: bool,
    cyclops_alive: Option<bool>,
    /// The mimic chest found out at the last look.
    mimic_done: Option<bool>,
    /// The last foe that hurt him, and when.
    blame: Option<(Subject, f64)>,
    /// His health at the last look, and whether his fall was counted.
    hp: i32,
    fallen: bool,
    /// Seconds to the next look round.
    look: f64,
}

/// The bestiary: the book, and the page it is open at.
#[derive(Debug)]
pub struct Bestiary {
    pub book: Book,
    /// The page is up (over the title, the pause menu, or the old man's guide).
    pub open: bool,
    /// The group and the page in it.
    pub group: usize,
    pub index: usize,
    /// Lines of the page's words scrolled past.
    pub scroll: usize,
    /// The most the page can scroll, as last drawn (the HUD measures the words).
    pub max_scroll: AtomicUsize,
    /// Every page open for review (the developer's map): what is saved is untouched.
    pub review: bool,
    /// Opened from the old man's guide: closing goes back to it.
    pub from_guide: bool,
    /// The game's time it opened, and when the page last changed (for the HUD's fades).
    pub since: f64,
    pub turned: f64,
    /// Something new since the book was last written, and whether writes reach the save.
    pub dirty: bool,
    pub live: bool,
    scroll_wait: f64,
    watch: Watch,
}

impl Default for Bestiary {
    fn default() -> Self {
        Self {
            book: Book::default(),
            open: false,
            group: 0,
            index: 0,
            scroll: 0,
            max_scroll: AtomicUsize::new(0),
            review: false,
            from_guide: false,
            since: 0.0,
            turned: 0.0,
            dirty: false,
            live: !cfg!(test),
            scroll_wait: 0.0,
            watch: Watch::default(),
        }
    }
}

impl Bestiary {
    /// The book the save holds (tests and the harnesses start from an empty one that is never written).
    pub fn load() -> Bestiary {
        if cfg!(test) {
            return Bestiary::default();
        }
        let book = platform::load(BOOK_KEY).map(|s| Book::decode(&s)).unwrap_or_default();
        Bestiary { book, ..Bestiary::default() }
    }

    /// A book that is filled but never written (the bots' harnesses: their fights are not the player's).
    pub fn unsaved() -> Bestiary {
        Bestiary { live: false, ..Bestiary::default() }
    }

    /// Is a page open (met, or every page in review)?
    pub fn shown(&self, page: Page) -> bool {
        match page {
            Page::Fixed(e) => self.review || self.book.unlocked(e),
            Page::Rival(_) => true,
        }
    }

    /// The pages of group `g`, in order (the rivals newest first).
    pub fn pages(&self, g: Group) -> Vec<Page> {
        match g {
            Group::Rivals => (0..self.book.rivals.len()).rev().map(Page::Rival).collect(),
            _ => g.entries().into_iter().map(Page::Fixed).collect(),
        }
    }

    /// The group open now.
    pub fn group_now(&self) -> Group {
        GROUPS[self.group.min(GROUPS.len() - 1)]
    }

    /// The page open now, if the group has any.
    pub fn page_now(&self) -> Option<Page> {
        let pages = self.pages(self.group_now());
        pages.get(self.index.min(pages.len().saturating_sub(1))).copied()
    }

    /// Pages open in group `g`, and how many it has.
    pub fn progress(&self, g: Group) -> (usize, usize) {
        let pages = self.pages(g);
        (pages.iter().filter(|&&p| self.shown(p)).count(), pages.len())
    }

    /// Pages open in the whole book (rivals included), and how many fixed pages it has.
    pub fn total(&self) -> (usize, usize) {
        let fixed = bestiary::all_entries();
        let open = fixed.iter().filter(|&&e| self.shown(Page::Fixed(e))).count();
        (open + self.book.rivals.len(), fixed.len() + self.book.rivals.len())
    }

    fn count(&mut self, who: &Subject, what: Count) {
        match who {
            Subject::Page(e) => match what {
                Count::Met => self.book.met(*e),
                Count::Felled => self.book.felled(*e),
                Count::FelledBy => self.book.felled_by(*e),
            },
            Subject::Rival(name) => {
                let Some(r) = self.book.rival_mut(name) else { return };
                let t = &mut r.tally;
                match what {
                    Count::Met => t.met += 1,
                    Count::Felled => {
                        t.felled += 1;
                        t.met = t.met.max(1);
                    }
                    Count::FelledBy => {
                        t.felled_by += 1;
                        t.met = t.met.max(1);
                    }
                }
            }
        }
        self.dirty = true;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Count {
    Met,
    Felled,
    FelledBy,
}

impl Game {
    // ------------------------------------------------------------------ watching the fight

    /// Once a frame of play: the foes met, the foes felled, and what felled him, into the book.
    pub(super) fn observe_bestiary(&mut self, dt: f64) {
        let key = (self.explore.epoch(), self.level.data.name.clone());
        if self.bestiary.watch.level.as_ref() != Some(&key) {
            let hp = self.hero.hp;
            self.bestiary.watch = Watch { level: Some(key), hp, ..Watch::default() };
        }
        // The war log of this frame: who fell, and who hurt him.
        let mut slain: Vec<(u32, EnemyKind)> = Vec::new();
        let mut hurt: Option<(u32, EnemyKind)> = None;
        for e in &self.war.read {
            match *e {
                WarEvent::Slain { foe, kind, .. } => slain.push((foe, kind)),
                WarEvent::Landed { foe, kind: Some(kind), dmg, .. } if dmg > 0.0 => hurt = Some((foe, kind)),
                _ => {}
            }
        }
        for (foe, kind) in slain {
            let who = self.subject_of(foe, kind);
            self.bestiary.count(&who, Count::Felled);
        }
        let now = self.time;
        if let Some((foe, kind)) = hurt {
            let who = self.subject_of(foe, kind);
            self.bestiary.watch.blame = Some((who, now));
        } else if self.hero.hp < self.bestiary.watch.hp {
            // Hurt with no blow of a duelist's: the foe touching him, or the Cyclops' throw.
            let hp = self.hero.pos();
            let touching = self
                .enemies
                .iter()
                .filter(|e| e.duel.is_none() && e.is_dangerous() && dist_sq(e.pos(), hp) <= TOUCH * TOUCH)
                .min_by(|a, b| dist_sq(a.pos(), hp).total_cmp(&dist_sq(b.pos(), hp)))
                .map(|e| Subject::Page(Entry::Foe(e.kind())));
            let who = touching.or_else(|| self.boss.as_ref().filter(|b| b.alive()).map(|_| Subject::Page(Entry::Cyclops)));
            if let Some(who) = who {
                self.bestiary.watch.blame = Some((who, now));
            }
        }
        self.bestiary.watch.hp = self.hero.hp;
        // The falls of the foes off the frame data (the garden's and the dungeons'): from the hunt
        // to the hollow, or to a heap of bones.
        let modes: Vec<Option<(EnemyKind, EnemyMode)>> =
            self.enemies.iter().map(|e| e.duel.is_none().then(|| (e.kind(), e.mode()))).collect();
        let mut fell: Vec<EnemyKind> = Vec::new();
        for (i, m) in modes.iter().enumerate() {
            if let (Some((k, mode)), Some(Some((was_k, was)))) = (m, self.bestiary.watch.modes.get(i)) {
                let hunting = matches!(was, EnemyMode::Scatter | EnemyMode::Chase | EnemyMode::Frenzied);
                if *k == *was_k && hunting && matches!(mode, EnemyMode::Eaten | EnemyMode::Scattered) {
                    fell.push(*k);
                }
            }
        }
        self.bestiary.watch.modes = modes;
        for k in fell {
            self.bestiary.count(&Subject::Page(Entry::Foe(k)), Count::Felled);
        }
        // The Cyclops: met when it comes down, felled when it dies.
        if let Some(b) = &self.boss {
            let (alive, shown) = (b.alive(), !matches!(b.state, BossState::Hidden));
            let w = &mut self.bestiary.watch;
            let met = alive && shown && !w.cyclops_met;
            let felled = w.cyclops_alive == Some(true) && !alive;
            w.cyclops_met |= met;
            w.cyclops_alive = Some(alive);
            if met {
                self.bestiary.count(&Subject::Page(Entry::Cyclops), Count::Met);
            }
            if felled {
                self.bestiary.count(&Subject::Page(Entry::Cyclops), Count::Felled);
            }
        }
        // The mimic chest: found out when struck (felled) or opened.
        let mimic = self.voices.deceit.mimic.as_ref().map(|m| (m.done, m.biter.is_none()));
        match mimic {
            Some((done, struck)) => {
                if done && self.bestiary.watch.mimic_done == Some(false) {
                    self.bestiary.count(&Subject::Page(Entry::Mimic), Count::Met);
                    if struck {
                        self.bestiary.count(&Subject::Page(Entry::Mimic), Count::Felled);
                    }
                }
                self.bestiary.watch.mimic_done = Some(done);
            }
            None => self.bestiary.watch.mimic_done = None,
        }
        // His fall, laid on the foe that hurt him last.
        if self.screen == Screen::Dying && !self.bestiary.watch.fallen {
            self.bestiary.watch.fallen = true;
            if let Some((who, at)) = self.bestiary.watch.blame.take() {
                if now - at <= BLAME_SECONDS {
                    self.bestiary.count(&who, Count::FelledBy);
                }
            }
        } else if self.screen == Screen::Playing {
            self.bestiary.watch.fallen = false;
        }
        // A look round, now and then: the foes in his sight, and the Ledger's rivals.
        self.bestiary.watch.look -= dt;
        if self.bestiary.watch.look <= 0.0 {
            self.bestiary.watch.look = LOOK_EVERY;
            self.look_for_foes();
        }
    }

    /// Who duelist `id` (of `kind`) is: the realm's general, a rival, or a foe of its kind.
    fn subject_of(&mut self, id: u32, kind: EnemyKind) -> Subject {
        if let Some(who) = self.bestiary.watch.duels.get(&id) {
            return who.clone();
        }
        if let Some(g) = self.general.as_ref().filter(|g| g.id == Some(id)) {
            return Subject::Page(Entry::General(g.sin));
        }
        if let Some(r) = self.rival_of(id).and_then(|r| self.ledger().get(r)) {
            let (name, k, rank, alive) = (r.full_name(), r.kind, r.rank, r.alive);
            if self.bestiary.book.remember_rival(&name, k, rank, alive) {
                self.bestiary.dirty = true;
            }
            return Subject::Rival(bestiary_name(&name));
        }
        Subject::Page(Entry::Foe(kind))
    }

    /// The foes in his sight, met (each once a level); the Ledger's rivals, remembered.
    fn look_for_foes(&mut self) {
        let notes: Vec<(String, EnemyKind, u8, bool)> =
            self.ledger().rivals.iter().map(|r| (r.full_name(), r.kind, r.rank, r.alive)).collect();
        for (name, kind, rank, alive) in notes {
            if self.bestiary.book.remember_rival(&name, kind, rank, alive) {
                self.bestiary.dirty = true;
            }
        }
        let hp = self.hero.pos();
        let mut duels: Vec<(u32, EnemyKind)> = Vec::new();
        let mut grid: Vec<(usize, EnemyKind)> = Vec::new();
        for (i, e) in self.enemies.iter().enumerate() {
            if matches!(e.mode(), EnemyMode::Nest | EnemyMode::Eaten) || e.is_ally() {
                continue;
            }
            let at = e.pos();
            let d2 = dist_sq(at, hp);
            if d2 > MEET_RANGE * MEET_RANGE {
                continue;
            }
            let t = P::new(at.x.round() as i32, at.y.round() as i32);
            if d2 > MEET_CLOSE * MEET_CLOSE && !self.explore.visible(t.x, t.y) && !self.explore.whole() {
                continue;
            }
            match e.duel.as_deref() {
                Some(d) if d.alive() && !self.bestiary.watch.duels.contains_key(&d.id) => duels.push((d.id, d.breed.kind)),
                Some(_) => {}
                None if !self.bestiary.watch.grid.contains(&i) => grid.push((i, e.kind())),
                None => {}
            }
        }
        for (id, kind) in duels {
            let who = self.subject_of(id, kind);
            self.bestiary.watch.duels.insert(id, who.clone());
            self.bestiary.count(&who, Count::Met);
        }
        for (i, kind) in grid {
            self.bestiary.watch.grid.insert(i);
            self.bestiary.count(&Subject::Page(Entry::Foe(kind)), Count::Met);
        }
    }

    /// The book to the save, if anything is new (with the run's snapshot: `flush_saves`).
    pub(super) fn flush_bestiary(&mut self) {
        let b = &mut self.bestiary;
        if !b.dirty {
            return;
        }
        b.dirty = false;
        if b.live {
            platform::save(BOOK_KEY, &b.book.encode());
        }
    }

    // ------------------------------------------------------------------ the page

    /// THE BESTIARY, from the title or the pause menu: the book at the first group with a page
    /// open (or where it was left).
    pub fn open_bestiary(&mut self) {
        let b = &mut self.bestiary;
        b.open = true;
        b.from_guide = false;
        b.since = self.time;
        b.turned = self.time;
        b.scroll = 0;
        if b.progress(b.group_now()).0 == 0 {
            if let Some(k) = GROUPS.iter().position(|&g| b.progress(g).0 > 0) {
                b.group = k;
                b.index = 0;
            }
        }
        // The first open page of the group.
        let pages = b.pages(b.group_now());
        if !pages.get(b.index).is_some_and(|&p| b.shown(p)) {
            b.index = pages.iter().position(|&p| b.shown(p)).unwrap_or(0);
        }
        self.audio.play(Sfx::Whoosh);
        for a in [Action::Confirm, Action::Jump, Action::Tap, Action::Attack, Action::Secondary] {
            self.input.consume(a);
        }
        self.input.click = None;
    }

    /// From the old man's guide (the menus' secondary): the book, and Back goes back to his words.
    pub(super) fn open_bestiary_from_guide(&mut self) {
        self.open_bestiary();
        self.bestiary.from_guide = true;
    }

    /// The developer's map's stop: every page open, at `group` and `index` (nothing saved of it).
    pub fn review_bestiary(&mut self, group: usize, index: usize) {
        if !matches!(self.screen, Screen::Title | Screen::Paused) {
            self.enter(Screen::Paused);
        }
        self.bestiary.review = true;
        self.open_bestiary();
        self.bestiary.group = group.min(GROUPS.len() - 1);
        self.bestiary.index = index;
    }

    fn close_bestiary(&mut self) {
        let b = &mut self.bestiary;
        b.open = false;
        b.review = false;
        let back = b.from_guide;
        b.from_guide = false;
        self.audio.play(Sfx::Nodule);
        for a in [Action::Confirm, Action::Jump, Action::Tap, Action::Attack, Action::Back, Action::Pause, Action::Guide] {
            self.input.consume(a);
        }
        self.input.click = None;
        if !back {
            if let Some(i) = self.menu_items().iter().position(|&m| m == MenuItem::Bestiary) {
                self.menu_index = i;
            }
        }
    }

    fn turn_to(&mut self, group: usize, index: usize) {
        let b = &mut self.bestiary;
        let changed = (b.group, b.index) != (group, index);
        b.group = group;
        b.index = index;
        if changed {
            b.scroll = 0;
            b.turned = self.time;
            self.audio.play(Sfx::Nodule);
        }
    }

    /// The page is up: LB / RB (Q / E), the D-pad's (or the arrows') left and right, or a click on
    /// a tab change the group; up and down, or a click, the page; the right stick, the wheel over
    /// the words, or Page Up / Down scroll them; B, Esc or Menu close it.
    pub(super) fn update_bestiary(&mut self, dt: f64) {
        let (w, h) = self.viewport;
        let group = self.bestiary.group_now();
        let pages = self.bestiary.pages(group);
        let n = pages.len();
        let lay = hud::bestiary::layout(w, h, n, self.bestiary.index);
        let inside = |p: (f32, f32), r: &[f32; 4]| p.0 >= r[0] && p.0 <= r[0] + r[2] && p.1 >= r[1] && p.1 <= r[1] + r[3];
        let click = self.input.click;
        if self.input.was_pressed(&[Action::Back, Action::Pause, Action::Guide]) || click.is_some_and(|p| inside(p, &lay.back)) {
            self.close_bestiary();
            return;
        }
        // The group.
        let g = GROUPS.len();
        let mut to_group = None;
        if self.input.was_pressed(&[Action::TabPrev]) || self.input.nav == Some(Dir::Left) || click.is_some_and(|p| inside(p, &lay.prev)) {
            to_group = Some((self.bestiary.group + g - 1) % g);
        }
        if self.input.was_pressed(&[Action::TabNext]) || self.input.nav == Some(Dir::Right) || click.is_some_and(|p| inside(p, &lay.next)) {
            to_group = Some((self.bestiary.group + 1) % g);
        }
        if let Some(p) = click {
            if let Some(k) = lay.tabs.iter().position(|r| inside(p, r)) {
                to_group = Some(k);
            }
        }
        if let Some(k) = to_group {
            let pages = self.bestiary.pages(GROUPS[k]);
            let first = pages.iter().position(|&p| self.bestiary.shown(p)).unwrap_or(0);
            self.turn_to(k, first);
            self.swallow_book();
            return;
        }
        // The wheel over the words scrolls them; elsewhere it moves the highlight (it is the menus' up and down).
        let wheel = self.input.wheel_turned();
        let over_words = self.input.cursor.is_some_and(|p| inside(p, &lay.words));
        let mut scroll: i32 = 0;
        if wheel != 0 && over_words {
            scroll -= wheel * 2;
        }
        // The page.
        if n > 0 && !(wheel != 0 && over_words) {
            let i = self.bestiary.index.min(n - 1);
            match self.input.nav {
                Some(Dir::Up) => self.turn_to(self.bestiary.group, (i + n - 1) % n),
                Some(Dir::Down) => self.turn_to(self.bestiary.group, (i + 1) % n),
                _ => {}
            }
        }
        if self.input.cursor_moved && !self.input.mouse_captured {
            if let Some(p) = self.input.cursor {
                if let Some(&(i, _)) = lay.rows.iter().find(|(_, r)| inside(p, r)) {
                    let group = self.bestiary.group;
                    self.turn_to(group, i);
                }
            }
        }
        if let Some(p) = click {
            if let Some(&(i, _)) = lay.rows.iter().find(|(_, r)| inside(p, r)) {
                let group = self.bestiary.group;
                self.turn_to(group, i);
            }
        }
        // The words: the right stick (held, a line at a time), Page Up and Page Down.
        let ((_, _), (_, ry)) = self.input.sticks();
        self.bestiary.scroll_wait -= dt;
        if ry.abs() > 0.5 {
            if self.bestiary.scroll_wait <= 0.0 {
                scroll += if ry > 0.0 { 1 } else { -1 };
                self.bestiary.scroll_wait = SCROLL_REPEAT;
            }
        } else {
            self.bestiary.scroll_wait = 0.0;
        }
        if self.input.went_down(Control::Key(winit::keyboard::KeyCode::PageDown)) {
            scroll += 4;
        }
        if self.input.went_down(Control::Key(winit::keyboard::KeyCode::PageUp)) {
            scroll -= 4;
        }
        if scroll != 0 {
            let max = self.bestiary.max_scroll.load(Ordering::Relaxed) as i32;
            self.bestiary.scroll = (self.bestiary.scroll as i32 + scroll).clamp(0, max.max(0)) as usize;
        }
        self.swallow_book();
    }

    /// What the page used up this update does nothing else.
    fn swallow_book(&mut self) {
        for a in [Action::Confirm, Action::Jump, Action::Tap, Action::Attack, Action::Secondary, Action::Map] {
            self.input.consume(a);
        }
        self.input.click = None;
    }

    /// Capture hook (`bestiary:G:I`, `bestiarylocked:G:I`): the book open at group G's page I, every
    /// page open for review, or as an empty book would show it.
    pub fn debug_bestiary(&mut self, group: usize, index: usize, locked: bool) {
        if locked {
            self.bestiary.book = Book::default();
            if !matches!(self.screen, Screen::Title | Screen::Paused) {
                self.enter(Screen::Paused);
            }
            self.open_bestiary();
            self.bestiary.group = group.min(GROUPS.len() - 1);
            self.bestiary.index = index;
        } else {
            // A few made-up tallies, so the figures show.
            let b = &mut self.bestiary.book;
            for (k, n) in
                [(EnemyKind::Oni, 3), (EnemyKind::Kappa, 5), (EnemyKind::Tepegoz, 2), (EnemyKind::Hel, 1), (EnemyKind::Skeleton, 9)]
            {
                for _ in 0..n {
                    b.met(Entry::Foe(k));
                    b.felled(Entry::Foe(k));
                }
            }
            b.felled_by(Entry::Foe(EnemyKind::Tepegoz));
            b.remember_rival("Kolgrim, Ragnar's Bane", EnemyKind::Karakoncolos, 3, true);
            b.remember_rival("Kara Ayaz the Shield-Breaker", EnemyKind::Oni, 2, false);
            self.review_bestiary(group, index);
        }
    }
}

/// A name as the book keeps it (the save's separators made spaces).
fn bestiary_name(name: &str) -> String {
    let mut b = Book::default();
    b.remember_rival(name, EnemyKind::Oni, 1, true);
    b.rivals.pop().map(|r| r.name).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::STEP;
    use crate::logic::sins::Sin;
    use crate::logic::types::V;

    fn paused() -> Game {
        let mut g = super::super::tests::playing();
        g.input.inject(Action::Pause);
        g.update(STEP);
        assert_eq!(g.screen, Screen::Paused);
        g
    }

    #[test]
    fn the_bestiary_opens_from_the_pause_menu_and_back_closes_it() {
        let mut g = paused();
        assert!(g.menu_items().contains(&MenuItem::Bestiary));
        g.menu_index = g.menu_items().iter().position(|&m| m == MenuItem::Bestiary).unwrap();
        g.input.inject(Action::Confirm);
        g.update(STEP);
        assert!(g.bestiary.open);
        g.update(STEP);
        assert!(g.bestiary.open, "it stays up");
        // Esc (the pause key) closes the book, not the pause.
        g.input.inject(Action::Pause);
        g.update(STEP);
        assert!(!g.bestiary.open);
        assert_eq!(g.screen, Screen::Paused, "still paused");
        assert_eq!(g.menu_items()[g.menu_index], MenuItem::Bestiary, "THE BESTIARY stays highlighted");
        // B too.
        g.open_bestiary();
        g.input.inject(Action::Back);
        g.update(STEP);
        assert!(!g.bestiary.open);
    }

    #[test]
    fn the_title_shows_the_bestiary_once_a_page_is_open() {
        let mut g = Game::new();
        assert!(!g.menu_items().contains(&MenuItem::Bestiary), "an empty book is not on the title");
        g.bestiary.book.met(Entry::Foe(EnemyKind::Skeleton));
        assert!(g.menu_items().contains(&MenuItem::Bestiary));
        g.menu_index = g.menu_items().iter().position(|&m| m == MenuItem::Bestiary).unwrap();
        g.input.inject(Action::Confirm);
        g.update(STEP);
        assert!(g.bestiary.open);
        assert_eq!(g.bestiary.page_now(), Some(Page::Fixed(Entry::Foe(EnemyKind::Skeleton))), "opened at the page met");
        g.input.inject(Action::Back);
        g.update(STEP);
        assert!(!g.bestiary.open);
        assert_eq!(g.screen, Screen::Title);
    }

    #[test]
    fn the_book_is_read_group_by_group_and_page_by_page() {
        let mut g = paused();
        g.review_bestiary(0, 0);
        assert_eq!(g.bestiary.group_now(), Group::Stage(1));
        // RB: the next group; LB twice: back past the first to the last.
        g.input.inject(Action::TabNext);
        g.update(STEP);
        assert_eq!(g.bestiary.group_now(), Group::Stage(2));
        g.input.inject(Action::TabPrev);
        g.update(STEP);
        g.input.inject(Action::TabPrev);
        g.update(STEP);
        assert_eq!(g.bestiary.group_now(), Group::Rivals);
        // The D-pad's right: the next group again (the first).
        g.input.nav = Some(Dir::Right);
        g.update_bestiary(STEP);
        assert_eq!(g.bestiary.group_now(), Group::Stage(1));
        // Down steps the page, round to the first.
        let n = g.bestiary.pages(Group::Stage(1)).len();
        for k in 1..=n {
            g.input.nav = Some(Dir::Down);
            g.update_bestiary(STEP);
            assert_eq!(g.bestiary.index, k % n);
        }
        g.input.nav = Some(Dir::Up);
        g.update_bestiary(STEP);
        assert_eq!(g.bestiary.index, n - 1);
        // A click on a tab opens its group; on a row, its page.
        let (w, h) = g.viewport;
        let lay = hud::bestiary::layout(w, h, n, 0);
        let r = lay.tabs[10];
        g.input.click = Some((r[0] + r[2] / 2.0, r[1] + r[3] / 2.0));
        g.update_bestiary(STEP);
        assert_eq!(g.bestiary.group_now(), Group::Courts);
        let lay = hud::bestiary::layout(w, h, g.bestiary.pages(Group::Courts).len(), 0);
        let (i, r) = lay.rows[2];
        g.input.click = Some((r[0] + r[2] / 2.0, r[1] + r[3] / 2.0));
        g.update_bestiary(STEP);
        assert_eq!(g.bestiary.index, i);
        assert_eq!(g.bestiary.page_now(), Some(Page::Fixed(Entry::Foe(EnemyKind::Lucifer))));
        // Review shows every page, and closing it ends the review.
        assert!(g.bestiary.shown(Page::Fixed(Entry::Foe(EnemyKind::Krampus))));
        g.input.inject(Action::Back);
        g.update(STEP);
        assert!(!g.bestiary.open && !g.bestiary.review);
        assert!(!g.bestiary.shown(Page::Fixed(Entry::Foe(EnemyKind::Krampus))), "the review left nothing in the book");
    }

    #[test]
    fn the_words_scroll_within_what_the_page_holds() {
        let mut g = paused();
        g.review_bestiary(10, 0);
        g.bestiary.max_scroll.store(5, Ordering::Relaxed);
        g.input.key_down(winit::keyboard::KeyCode::PageDown, false);
        g.update(STEP);
        g.input.key_up(winit::keyboard::KeyCode::PageDown);
        assert_eq!(g.bestiary.scroll, 4);
        g.input.key_down(winit::keyboard::KeyCode::PageDown, false);
        g.update(STEP);
        g.input.key_up(winit::keyboard::KeyCode::PageDown);
        assert_eq!(g.bestiary.scroll, 5, "no further than the page goes");
        // A new page starts at its top.
        g.input.nav = Some(Dir::Down);
        g.update_bestiary(STEP);
        assert_eq!(g.bestiary.scroll, 0);
    }

    #[test]
    fn the_guide_opens_the_book_and_the_book_goes_back_to_the_guide() {
        let mut g = super::super::tests::playing();
        g.open_guide();
        assert!(g.guide_open);
        g.input.inject(Action::Secondary);
        g.update(STEP);
        assert!(g.bestiary.open && g.bestiary.from_guide && g.guide_open);
        g.input.inject(Action::Back);
        g.update(STEP);
        assert!(!g.bestiary.open);
        assert!(g.guide_open, "back to the old man's words");
        assert_eq!(g.screen, Screen::Paused);
    }

    #[test]
    fn foes_met_felled_and_falls_go_into_the_book() {
        let mut g = super::super::tests::playing();
        // A draugr of the garden at arm's length: met, once.
        let i = g.enemies.iter().position(|e| e.kind() == EnemyKind::Skeleton).expect("a draugr in the garden");
        let at = g.hero.pos();
        g.enemies[i].stand_at(P::new(at.x.round() as i32 + 1, at.y.round() as i32));
        g.observe_bestiary(LOOK_EVERY + 0.01);
        assert_eq!(g.bestiary.book.tally(Entry::Foe(EnemyKind::Skeleton)).met, 1);
        g.observe_bestiary(LOOK_EVERY + 0.01);
        assert_eq!(g.bestiary.book.tally(Entry::Foe(EnemyKind::Skeleton)).met, 1, "once a level");
        assert!(g.bestiary.dirty);
        // Struck down: felled.
        g.slay_enemy(i);
        g.observe_bestiary(0.0);
        assert_eq!(g.bestiary.book.tally(Entry::Foe(EnemyKind::Skeleton)).felled, 1);
        // A duelist's slaying and its blow, from the war log; the blow's foe takes his fall.
        g.war.read = vec![
            WarEvent::Slain { foe: 77, kind: EnemyKind::Oni, at: V::new(1.0, 1.0), dir: V::new(1.0, 0.0), last_of_pack: false },
            WarEvent::Landed {
                foe: 78,
                kind: Some(EnemyKind::Hel),
                how: crate::logic::combat::fsm::Defense::Open,
                dmg: 12.0,
                dir: V::new(1.0, 0.0),
                lands: true,
                bolt: true,
            },
        ];
        g.observe_bestiary(0.0);
        assert_eq!(g.bestiary.book.tally(Entry::Foe(EnemyKind::Oni)).felled, 1);
        g.war.read.clear();
        g.screen = Screen::Dying;
        g.observe_bestiary(0.0);
        g.observe_bestiary(0.0);
        assert_eq!(g.bestiary.book.tally(Entry::Foe(EnemyKind::Hel)).felled_by, 1, "one fall, counted once");
        assert!(g.bestiary.book.unlocked(Entry::Foe(EnemyKind::Hel)), "what felled him is met");
    }

    #[test]
    fn the_book_never_changes_the_fight() {
        // The bot through the vaults, watched by the book and not: the same game.
        let run = |watch: bool| {
            let mut g = super::super::bot::fresh_game(5);
            g.map_jump(super::super::save::Stage::Realm(Sin::Greed));
            g.enter(Screen::Playing);
            g.hero.free_move = true;
            let mut bot = super::super::bot::Bot::new(super::super::bot::Skill::Expert, 5);
            for _ in 0..(60 * 20) {
                match g.screen {
                    Screen::Playing => {}
                    Screen::Ready => g.enter(Screen::Playing),
                    _ => break,
                }
                let pad = bot.act(&g);
                g.input.set_pad(Some(pad));
                g.update(STEP);
                if !watch {
                    g.bestiary = Bestiary::unsaved();
                }
            }
            let foes: Vec<(u32, i64, i64)> = g
                .enemies
                .iter()
                .filter_map(|e| e.duel.as_deref())
                .map(|d| (d.id, (d.pos.x * 1000.0) as i64, (d.hp * 1000.0) as i64))
                .collect();
            (g.hero.hp, (g.hero.pos().x * 1000.0) as i64, g.score.score, foes, g.bestiary.book.count())
        };
        let (a, b) = (run(true), run(false));
        assert_eq!((a.0, a.1, a.2, &a.3), (b.0, b.1, b.2, &b.3));
        assert!(a.4 > 0, "the book was filled as it watched");
    }

    #[test]
    fn the_book_is_saved_and_read_back() {
        let dir = std::env::temp_dir().join(format!("vh-bestiary-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        platform::use_dir_on_this_thread(dir.clone());
        let mut g = Game::new();
        g.bestiary.live = true;
        g.bestiary.book.met(Entry::General(Sin::Wrath));
        g.bestiary.dirty = true;
        g.flush_saves();
        assert!(!g.bestiary.dirty);
        let saved = platform::load(BOOK_KEY).expect("the book was written");
        let back = Book::decode(&saved);
        assert!(back.unlocked(Entry::General(Sin::Wrath)));
        // An old save, from before the book: nothing to read, and an empty book.
        platform::save(BOOK_KEY, "");
        assert!(Book::decode(&platform::load(BOOK_KEY).unwrap_or_default()).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
