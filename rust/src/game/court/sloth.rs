//! Sloth's court in play: the Hall of Stopped Clocks (`logic::court::sloth` lays it out) and
//! Belphegor, its prince (PRINCES.md §4).
//!
//! He sleeps in his great chair as the fight begins (his phase 0): a puppet of his own torpor,
//! doing nothing, his hide open. The first blow lands whole and wakes him (`Cue::Phase(1)`). As his
//! second phase begins he finishes what he is doing and lies down to sleep again, and two
//! nightmares gather out of the fog to fight for him (once in a fight; at once if his general's
//! decree was done, never if his general is slain). Asleep, he mends a little every second, never
//! past the line where his dream began; blows that take a share of his health in one sleep, or a
//! finisher, wake him with a roar, and he fights; some seconds later, still dreaming, he nods off
//! again. At one third he wakes for good and stands. His general's decree done, he wakes rested:
//! he does not sleep at the start (no free blow).
//!
//! His sleep is kept on his `Lore` (`prince::sloth::Slumber`), stepped here once a combat frame.

use crate::audio::{Cry, Sfx};
use crate::game::Game;
use crate::logic::combat::duelist::Lore;
use crate::logic::combat::prince::sloth::{Doze, Slumber, DOZE_AGAIN, MEND, NIGHTMARES, NIGHTMARE_RISE, NOD_FRAMES, WAKE_FRAMES};
use crate::logic::combat::prince::PHASE_AT;
use crate::logic::combat::{math, MoveId};
use crate::logic::court::{sloth::nightmare_spots, weight};
use crate::logic::enemy::EnemyKind;
use crate::logic::sins::Sin;
use crate::logic::types::V;

/// What his sleep did this frame, for the game to show and sound.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Stir {
    /// Struck awake: he rouses himself with a roar.
    Woke,
    /// His eyes close and he lies down.
    NodsOff,
    /// He mends in his sleep.
    Mends,
}

impl Game {
    /// As Belphegor is set down in his chair: asleep, unless his general's decree woke him rested.
    pub(super) fn sloth_enter(&mut self, prince: u32) {
        self.put_belphegor_to_sleep(prince);
    }

    /// Each tick of the fight in the Hall of Stopped Clocks: his sleep moves on a frame at a time.
    pub(super) fn sloth_tick(&mut self, _dt: f64) {
        let Some(id) = self.court.as_ref().filter(|c| !c.slain).map(|c| c.prince) else { return };
        let Some(b) = self.brawl.as_ref() else { return };
        let frame = b.threat.frame;
        let finisher = b.hero.attack().is_some_and(|a| a.0 == MoveId::L3);
        let Some(d) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == id && d.alive()) else {
            return;
        };
        let mut s = d.lore.slumber;
        // (The frames since he was last seen: none the first time, nor after a fall.)
        let steps = if s.seen == 0 || frame < s.seen { 0 } else { frame - s.seen };
        s.seen = frame.max(1);
        let max = d.max_hp;
        let taken = if s.last > 0.0 { (s.last - d.hp).max(0.0) } else { 0.0 };
        let mut stirs = Vec::new();
        if taken > 0.0 && s.wakes(taken, max, finisher) {
            s.doze = Doze::Waking(WAKE_FRAMES);
            stirs.push(Stir::Woke);
        } else if taken > 0.0 && s.open() {
            s.struck += taken;
        }
        // Mending stops at the line where his dream began (he stays in his second phase).
        let cap = max * PHASE_AT[0] - 1.0;
        let mut release = false;
        for _ in 0..steps.min(8) {
            s.doze = match s.doze {
                Doze::Awake => {
                    s.awake += 1;
                    if d.breed.phase == 2 && s.awake >= DOZE_AGAIN {
                        Doze::Drowsy
                    } else {
                        Doze::Awake
                    }
                }
                // He finishes what he is doing, then lies down where he stands.
                Doze::Drowsy if d.f.is_neutral() => {
                    s.struck = 0.0;
                    s.slept = 0;
                    s.chair = false;
                    release |= d.token;
                    d.token = false;
                    stirs.push(Stir::NodsOff);
                    Doze::Nodding(NOD_FRAMES)
                }
                Doze::Drowsy => Doze::Drowsy,
                Doze::Nodding(left) => {
                    s.slept += 1;
                    if left <= 1 {
                        Doze::Asleep
                    } else {
                        Doze::Nodding(left - 1)
                    }
                }
                Doze::Asleep => {
                    s.slept += 1;
                    if d.breed.phase == 2 && d.hp < cap {
                        d.hp = (d.hp + max * MEND / 60.0).min(cap);
                        if s.slept.is_multiple_of(90) {
                            stirs.push(Stir::Mends);
                        }
                    }
                    Doze::Asleep
                }
                Doze::Waking(left) => {
                    if left <= 1 {
                        s.awake = 0;
                        Doze::Awake
                    } else {
                        Doze::Waking(left - 1)
                    }
                }
            };
        }
        if s.doze == Doze::Drowsy {
            // (His eyes closing, he asks for no more turns.)
            d.rest = d.rest.max(30);
        }
        d.puppet = s.open();
        s.last = d.hp;
        d.lore.slumber = s;
        let at = d.pos;
        if release {
            if let Some(b) = self.brawl.as_mut() {
                b.threat.release(id);
            }
        }
        for stir in stirs {
            match stir {
                Stir::Woke => {
                    self.audio.play(Sfx::BossRoar);
                    self.audio.foe_cry(EnemyKind::Belphegor, Cry::Alert, at);
                    self.view.shake(0.35);
                    self.floater(at.x, at.y - 2.2, "STRUCK AWAKE!", 1.4, Some(0xffd060));
                }
                Stir::NodsOff => {
                    self.audio.play(Sfx::Tick);
                    self.floater(at.x, at.y - 2.2, "HE NODS OFF: STRIKE HIM AWAKE", 1.8, Some(0xa8b4e0));
                }
                Stir::Mends => self.floater(at.x, at.y - 2.0, "HE MENDS AS HE SLEEPS", 1.2, Some(0x8ad08a)),
            }
        }
    }

    /// As his health falls into `phase`: the first blow wakes him (the Sleeper); at two thirds he
    /// lies down to dream and his nightmares come (unless they have, or his general's death has
    /// lost them); at one third he wakes for good.
    pub(super) fn sloth_phase(&mut self, prince: u32, at: V, phase: u8) {
        if phase >= 2 && self.adds_unsummoned(prince) {
            self.raise_nightmares(prince, at);
        }
        let Some(d) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == prince) else { return };
        let s = &mut d.lore.slumber;
        match phase {
            2 => {
                s.doze = Doze::Drowsy;
                s.awake = 0;
            }
            _ if s.open() => s.doze = Doze::Waking(WAKE_FRAMES),
            _ => s.doze = Doze::Awake,
        }
    }

    /// After a fall (his nightmares are already gone): back asleep in his chair.
    pub(super) fn sloth_reset(&mut self) {
        if let Some(id) = self.court.as_ref().map(|c| c.prince) {
            self.put_belphegor_to_sleep(id);
        }
    }

    /// As he falls: the stopped clocks start again.
    pub(super) fn sloth_slain(&mut self, at: V) {
        self.floater(at.x, at.y - 1.8, "THE CLOCKS START AGAIN", 2.0, Some(0xc8d4f0));
        self.audio.play(Sfx::Bells);
    }

    /// The phase banner's line under his name, and its colour.
    pub(super) fn sloth_banner(phase: u8) -> (&'static str, u32) {
        match phase {
            0 | 1 => ("HE WAKES   PARRY HIS SWAT, LEAP HIS YAWN, STEP OFF THE SAND", 0x9aa8c8),
            2 => ("HE SLEEPS AND MENDS, AND HIS NIGHTMARES FIGHT   STRIKE HIM AWAKE", 0xa080d0),
            _ => ("HE STANDS AT LAST   STEP BACK FROM HIS LONG REACH, THEN STRIKE", 0xff8a3a),
        }
    }

    /// His adds at once (his general's decree done): his nightmares.
    pub(super) fn sloth_raise(&mut self, prince: u32, at: V) {
        self.raise_nightmares(prince, at);
    }

    /// Belphegor `id` asleep in his chair (his phase 0, his hide open, doing nothing) unless his
    /// general's decree was done: then he wakes rested, and fights from the first.
    fn put_belphegor_to_sleep(&mut self, id: u32) {
        let rested = self.prince_terms(Sin::Sloth).adds_at_start;
        let Some(d) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == id) else { return };
        if rested {
            d.puppet = false;
            d.lore.slumber = Slumber { last: d.hp, ..Slumber::default() };
            return;
        }
        d.breed.phase = 0;
        d.puppet = true;
        d.lore.slumber = Slumber::in_chair(d.hp);
        if let Some(c) = self.court.as_mut() {
            c.phase = 0;
        }
    }

    /// Two nightmares gather out of the fog either side of him; they fall when he does.
    fn raise_nightmares(&mut self, id: u32, at: V) {
        let facing =
            self.enemies.iter().filter_map(|e| e.duel.as_deref()).find(|d| d.id == id).map(|d| d.facing).unwrap_or(V::new(0.0, 1.0));
        let g = &self.level.grid;
        let spots = nightmare_spots((at.x, at.y), (facing.x, facing.y), |p| g.is_walkable(p.x, p.y, false));
        let hp = self.hero.pos();
        let (weight_hp, weight_damage) = if self.in_court() { weight(self.realm_tier()) } else { (1.0, 1.0) };
        for (x, y) in spots.into_iter().take(NIGHTMARES as usize) {
            let p = V::new(x, y);
            let look = math::norm(math::sub(hp, p));
            let lore = Lore { master: Some(id), rise: NIGHTMARE_RISE, ..Lore::default() };
            if let Some(nid) = self.spawn_duelist(EnemyKind::Nightmare, p, look, lore) {
                if let Some(n) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == nid) {
                    n.max_hp *= weight_hp;
                    n.hp = n.max_hp;
                    n.breed.damage *= weight_damage;
                }
            }
            self.view.dust(&self.level.grid, p.x, p.y, 0x6a5a9a, 22, 1.6);
            self.view.sparkle(&self.level.grid, p.x, p.y, 1.1, 0xc080ff, 26);
        }
        self.floater(hp.x, hp.y - 1.5, "HIS NIGHTMARES COME", 1.6, Some(0xb08aff));
        self.audio.play(Sfx::Shriek);
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{self, run};
    use super::super::{COURT_CADENCE, COURT_CAPACITY};
    use super::*;
    use crate::game::{save, Screen};
    use crate::logic::combat::archetype::Archetype;
    use crate::logic::combat::fsm::Hit;
    use crate::logic::combat::prince::phase_of;
    use crate::logic::combat::prince::sloth::WAKE_SHARE;
    use crate::logic::combat::{Duelist, State};
    use crate::logic::types::{Tile, P};
    use crate::logic::war::general;

    fn court() -> Game {
        testing::court(Sin::Sloth)
    }

    fn belphegor(g: &mut Game) -> &mut Duelist {
        testing::prince(g)
    }

    fn nightmares(g: &Game) -> usize {
        g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.alive() && d.breed.kind == EnemyKind::Nightmare).count()
    }

    /// A blow of `damage` on him (as a sword's, from where the hero stands).
    fn strike(g: &mut Game, damage: f64) {
        let id = g.court.as_ref().unwrap().prince;
        let i = g.enemies.iter().position(|e| e.duel.as_ref().is_some_and(|d| d.id == id)).unwrap();
        let from = g.hero.pos();
        g.duel_hit(i, Hit { damage, poise: 0.0, hitstun: 0, ..Default::default() }, 0, from);
    }

    /// The hero a few tiles in front of him, where the fight can find him.
    fn before_him(g: &mut Game, tiles: i32) {
        let p = belphegor(g).pos;
        g.hero.place(P::new(p.x.round() as i32, p.y.round() as i32 + tiles));
    }

    #[test]
    fn the_court_of_sloth_is_the_hall_of_stopped_clocks_and_belphegor_sleeps_in_his_chair() {
        let mut g = court();
        assert_eq!(g.level.data.name, "The Hall of Stopped Clocks");
        assert_eq!(g.realm, Some(Sin::Sloth));
        assert!(g.boss.is_none(), "no Cyclops");
        assert!(g.arena() && g.brawling(), "a boss's arena, fought by frame data");
        let b = g.brawl.as_ref().unwrap();
        assert_eq!((b.threat.capacity, b.threat.cadence), (COURT_CAPACITY, COURT_CADENCE));
        let d = g.prince().expect("Belphegor sits there");
        assert_eq!((d.breed.kind, d.arch, d.breed.phase), (EnemyKind::Belphegor, Archetype::Prince, 0));
        assert!(d.puppet && d.lore.slumber.chair && d.exposed(), "asleep in his chair, his hide open");
        assert_eq!(g.enemies.len(), 1, "he sleeps alone");
        let bar = g.prince_bar().unwrap();
        assert_eq!((bar.name.as_str(), bar.title, bar.open), ("BELPHEGOR", "ASLEEP", true));
        assert_eq!(g.keys_total, 1, "one key opens the gate");
        // Left alone he sleeps on, and does nothing (the hero walks up to him unhurt).
        before_him(&mut g, 3);
        let (at, hp) = (belphegor(&mut g).pos, g.hero.hp);
        run(&mut g, 4.0);
        let d = belphegor(&mut g);
        assert!(d.puppet && d.f.is_neutral() && math::len(math::sub(d.pos, at)) < 0.01, "still asleep where he sat");
        assert_eq!(g.hero.hp, hp);
    }

    #[test]
    fn the_first_blow_lands_whole_and_wakes_him_with_a_roar_and_he_fights() {
        let mut g = court();
        before_him(&mut g, 2);
        let max = belphegor(&mut g).max_hp;
        strike(&mut g, 20.0);
        assert!((max - belphegor(&mut g).hp - 20.0).abs() < 1e-6, "the whole blow lands: {}", max - belphegor(&mut g).hp);
        run(&mut g, 0.05);
        let d = belphegor(&mut g);
        assert_eq!(d.breed.phase, 1);
        assert!(matches!(d.lore.slumber.doze, Doze::Waking(_)) && d.exposed(), "open as he wakes");
        assert!(g.juice.banner.as_ref().is_some_and(|b| b.text.contains("THE SLEEPER")));
        // Awake, his hide turns most of a blow (outside his recoveries).
        run(&mut g, 2.0);
        let d = belphegor(&mut g);
        assert_eq!(d.lore.slumber.doze, Doze::Awake);
        assert!(!d.puppet);
        d.rest = u16::MAX;
        d.token = false;
        let before = d.hp;
        strike(&mut g, 20.0);
        let warded = before - belphegor(&mut g).hp;
        assert!(warded > 0.0 && warded < 10.0, "warded {warded}");
        // And he fights: with the hero before him he begins a move.
        belphegor(&mut g).rest = 0;
        let mut moved = false;
        for _ in 0..(8.0 / crate::app::STEP) as usize {
            g.update(crate::app::STEP);
            if g.prince().is_some_and(|d| matches!(d.f.state, State::Attack { .. })) {
                moved = true;
                break;
            }
        }
        assert!(moved, "he fights once awake");
    }

    #[test]
    fn his_phases_change_at_the_thresholds_he_lies_down_to_dream_and_the_nightmares_rise_once() {
        let mut g = court();
        before_him(&mut g, 4);
        let max = belphegor(&mut g).max_hp;
        strike(&mut g, 1.0);
        run(&mut g, 1.5);
        assert_eq!(belphegor(&mut g).breed.phase, 1);
        // A blow that leaves him just above two thirds: still the Sleeper.
        let d = belphegor(&mut g);
        (d.rest, d.token, d.f.state) = (u16::MAX, false, State::Neutral);
        d.hp = max * 0.67 + 30.0;
        strike(&mut g, 4.0);
        run(&mut g, 0.05);
        let hp = belphegor(&mut g).hp;
        assert_eq!(belphegor(&mut g).breed.phase, phase_of(hp, max));
        assert_eq!(nightmares(&g), 0);
        // Under two thirds: the Dreamer, his nightmares, a green cross; he lies down to sleep.
        belphegor(&mut g).hp = max * 0.6;
        strike(&mut g, 4.0);
        run(&mut g, 0.05);
        assert_eq!(belphegor(&mut g).breed.phase, 2);
        assert_eq!(g.court.as_ref().unwrap().phase, 2);
        assert_eq!(nightmares(&g), NIGHTMARES as usize, "two nightmares out of his dream");
        assert!(g.level.grid.count(Tile::Charge) >= 1, "a breather");
        assert!(g.juice.banner.as_ref().is_some_and(|b| b.text.contains("THE DREAMER")));
        run(&mut g, 1.5);
        let d = belphegor(&mut g);
        assert_eq!(d.lore.slumber.doze, Doze::Asleep, "he lies down where he stands");
        assert!(d.puppet && d.exposed() && !d.lore.slumber.chair);
        // Under a third: the Waking. He stands for good; the nightmares are not raised again.
        belphegor(&mut g).hp = max * 0.3;
        strike(&mut g, 4.0);
        run(&mut g, 0.05);
        assert_eq!(belphegor(&mut g).breed.phase, 3);
        assert!(nightmares(&g) <= NIGHTMARES as usize, "raised once");
        assert_eq!(belphegor(&mut g).breed.moves()[0], MoveId::HourglassSlam);
        run(&mut g, 1.5);
        assert_eq!(belphegor(&mut g).lore.slumber.doze, Doze::Awake);
        assert!(!belphegor(&mut g).puppet);
    }

    /// Belphegor brought to his second phase, asleep in it, the hero a few tiles off.
    fn dreaming() -> Game {
        let mut g = court();
        before_him(&mut g, 5);
        let max = belphegor(&mut g).max_hp;
        strike(&mut g, 1.0);
        run(&mut g, 1.5);
        let d = belphegor(&mut g);
        (d.rest, d.token, d.f.state) = (u16::MAX, false, State::Neutral);
        d.hp = max * 0.62;
        strike(&mut g, 2.0);
        run(&mut g, 1.5);
        assert_eq!(belphegor(&mut g).lore.slumber.doze, Doze::Asleep);
        // (His nightmares kept out of it.)
        g.enemies.retain(|e| e.duel.as_ref().is_none_or(|d| d.breed.kind != EnemyKind::Nightmare));
        g.refresh_foe_views();
        g
    }

    #[test]
    fn asleep_in_his_dream_he_mends_and_blows_enough_wake_him() {
        let mut g = dreaming();
        let max = belphegor(&mut g).max_hp;
        let hp0 = belphegor(&mut g).hp;
        run(&mut g, 5.0);
        let hp1 = belphegor(&mut g).hp;
        assert!(hp1 > hp0 + max * MEND * 3.0, "a hero who waits lets him mend: {hp0} -> {hp1}");
        run(&mut g, 30.0);
        assert!(belphegor(&mut g).hp < max * PHASE_AT[0], "never past the line where his dream began");
        assert_eq!(belphegor(&mut g).breed.phase, 2);
        // A tap does not wake him; blows that take a share of his health do, whole as they land.
        let before = belphegor(&mut g).hp;
        strike(&mut g, 10.0);
        run(&mut g, 0.05);
        assert!((before - belphegor(&mut g).hp - 10.0).abs() < 1.0, "whole");
        assert_eq!(belphegor(&mut g).lore.slumber.doze, Doze::Asleep);
        let mut blows = 0;
        while belphegor(&mut g).lore.slumber.doze == Doze::Asleep {
            strike(&mut g, 10.0);
            run(&mut g, 0.05);
            blows += 1;
            assert!(blows < 20);
        }
        assert!(blows as f64 * 10.0 + 10.0 >= max * WAKE_SHARE - 1.0, "{blows} blows");
        assert!(matches!(belphegor(&mut g).lore.slumber.doze, Doze::Waking(_)));
        run(&mut g, 1.5);
        assert_eq!(belphegor(&mut g).lore.slumber.doze, Doze::Awake);
        // Some seconds later, still dreaming, he nods off again.
        belphegor(&mut g).rest = u16::MAX;
        run(&mut g, DOZE_AGAIN as f64 / 60.0 + 1.5);
        assert!(belphegor(&mut g).lore.slumber.open(), "{:?}", belphegor(&mut g).lore.slumber.doze);
    }

    #[test]
    fn a_finisher_wakes_him_at_once() {
        let mut g = dreaming();
        let b = g.brawl.as_mut().unwrap();
        b.hero.state = State::Attack { mv: MoveId::L3, frame: 10 };
        strike(&mut g, 12.0);
        run(&mut g, 0.02);
        assert!(matches!(belphegor(&mut g).lore.slumber.doze, Doze::Waking(_)));
    }

    #[test]
    fn his_hide_turns_a_blow_when_awake_but_not_in_his_recovery_nor_while_he_sleeps() {
        let mut g = court();
        let before = belphegor(&mut g).hp;
        strike(&mut g, 20.0);
        let asleep = before - belphegor(&mut g).hp;
        run(&mut g, 1.5);
        let d = belphegor(&mut g);
        d.rest = u16::MAX;
        let before = d.hp;
        strike(&mut g, 20.0);
        let warded = before - belphegor(&mut g).hp;
        let d = belphegor(&mut g);
        d.f.state = State::Attack { mv: MoveId::Swat, frame: 60 };
        let before = d.hp;
        strike(&mut g, 20.0);
        let open = before - belphegor(&mut g).hp;
        assert!((asleep - 20.0).abs() < 1e-6, "asleep: {asleep}");
        assert!(warded > 0.0 && open > warded * 2.5, "warded {warded}, open {open}");
    }

    /// Belphegor awake, `dist` tiles north of the hero, facing him, beginning `mv`.
    fn begins(mv: MoveId, dist: f64) -> Game {
        let mut g = court();
        strike(&mut g, 1.0);
        run(&mut g, 1.5);
        let hp = g.hero.pos();
        let d = belphegor(&mut g);
        d.pos = V::new(hp.x, hp.y - dist);
        d.facing = V::new(0.0, 1.0);
        d.rest = u16::MAX;
        d.token = true;
        if mv != MoveId::Yawn && mv != MoveId::Swat && mv != MoveId::SandOfHours {
            d.breed.phase = 3;
        }
        assert!(d.begin(mv, hp));
        g
    }

    #[test]
    fn the_sand_of_hours_lies_on_the_floor_and_bursts_late_enough_to_step_off_it() {
        // Standing still on it, he is hurt.
        let mut g = begins(MoveId::SandOfHours, 5.0);
        let start = g.hero.hp;
        let mut seen = 0;
        for _ in 0..400 {
            g.update(crate::app::STEP);
            let b = g.brawl.as_ref().unwrap();
            seen = seen.max(b.traps.len());
            if b.traps.iter().any(|t| t.burst.is_some()) {
                break;
            }
        }
        assert!((4..=6).contains(&seen), "the sand drifts down in heaps round him: {seen}");
        run(&mut g, 0.2);
        assert!(g.hero.hp < start, "standing in it, he is hurt");
        // Walking off the heap under him as soon as it has landed, he is not.
        let mut g = begins(MoveId::SandOfHours, 5.0);
        let start = g.hero.hp;
        for _ in 0..400 {
            g.update(crate::app::STEP);
            if g.brawl.as_ref().unwrap().traps.iter().any(|t| t.armed()) {
                break;
            }
        }
        let hp = g.hero.pos();
        g.brawl.as_mut().unwrap().traps.retain(|t| t.reaches(hp, 0.3));
        assert_eq!(g.brawl.as_ref().unwrap().traps.len(), 1, "one where he stands");
        // (Reaction time, then off sideways.)
        run(&mut g, crate::logic::combat::frames::REACTION as f64 / 60.0);
        g.input.hold_dir(crate::logic::types::Dir::Left, true);
        run(&mut g, 1.4);
        g.input.hold_dir(crate::logic::types::Dir::Left, false);
        assert_eq!(g.hero.hp, start, "he stepped off it in time");
    }

    #[test]
    fn his_yawn_pushes_the_hero_back_and_hurts_little() {
        let mut g = begins(MoveId::Yawn, 2.2);
        let (start, at) = (g.hero.hp, g.hero.pos());
        run(&mut g, 1.4);
        let gone = math::len(math::sub(g.hero.pos(), at));
        assert!(gone > 1.5, "pushed back {gone}");
        assert!(g.hero.hp < start && start - g.hero.hp < 15, "{} of {start}", g.hero.hp);
        assert!(g.hero.pos().y > at.y, "away from him");
    }

    #[test]
    fn the_clock_hands_reach_far_round_him() {
        // Three tiles off, beside him, the hands still find him...
        let mut g = begins(MoveId::ClockHands, 3.0);
        let start = g.hero.hp;
        run(&mut g, 1.2);
        assert!(g.hero.hp < start, "struck from three tiles");
        // ...but not five.
        let mut g = begins(MoveId::ClockHands, 5.0);
        let start = g.hero.hp;
        run(&mut g, 1.2);
        assert_eq!(g.hero.hp, start);
    }

    #[test]
    fn belphegor_slain_drops_the_key_of_sloth_and_seals_sloth() {
        let mut g = court();
        strike(&mut g, 1.0);
        run(&mut g, 1.5);
        let d = belphegor(&mut g);
        d.hp = 1.0;
        d.f.state = State::Attack { mv: MoveId::Swat, frame: 60 };
        d.breed.phase = 3;
        let from = d.pos;
        g.call_duel_hit(0, from);
        assert!(g.prince().is_none(), "he has fallen");
        assert!(g.court.as_ref().unwrap().slain);
        assert!(g.red_key.is_some(), "the Key of Sloth lies on the floor");
        assert!(g.sins_cleared & Sin::Sloth.bit() != 0, "Sloth's bit is set");
        assert_eq!(g.key_name(), "THE KEY OF SLOTH");
        assert!(g.saved_run.as_ref().is_some_and(|r| r.stage == save::Stage::SinShop(Sin::Sloth)), "the checkpoint is the old man's");
        let k = g.red_key.unwrap();
        g.hero.place(k);
        run(&mut g, 0.3);
        assert!(g.red_key.is_none() && g.gate_open);
        let gate = g.gate.unwrap();
        g.hero.place(P::new(gate.x, gate.y - 1));
        for _ in 0..1200 {
            g.update(crate::app::STEP);
            if g.screen == Screen::Won {
                break;
            }
            let gp = P::new(gate.x, gate.y);
            if g.door >= 1.0 && g.hero.occupied() != gp {
                g.hero.place(gp);
            }
        }
        assert_eq!(g.screen, Screen::Won);
    }

    #[test]
    fn a_fall_brings_belphegor_back_whole_and_asleep_and_continue_comes_back_to_his_court() {
        let mut g = court();
        let max = belphegor(&mut g).max_hp;
        strike(&mut g, 1.0);
        run(&mut g, 1.5);
        belphegor(&mut g).hp = max * 0.5;
        belphegor(&mut g).breed.phase = 2;
        g.reset_round();
        let d = belphegor(&mut g);
        assert_eq!((d.hp, d.breed.phase), (max, 0));
        assert!(d.puppet && d.lore.slumber.chair, "back asleep in his chair");
        assert_eq!(d.pos, P::new(crate::logic::court::sloth::PRINCE.0, crate::logic::court::sloth::PRINCE.1).v());
        g.checkpoint(save::Stage::Court(Sin::Sloth));
        let saved = g.saved_run.clone().unwrap();
        let mut h = Game::new();
        h.saved_run = Some(saved);
        h.continue_game();
        assert!(h.in_court() && h.prince().is_some());
        assert_eq!(h.level.data.name, "The Hall of Stopped Clocks");
    }

    /// The court of Sloth, his general (the Lamplighter) slain or his decree done.
    fn with_general(decree_done: bool, slain: bool) -> Game {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test-court"));
        let mut g = Game::new();
        let mut all = g.general_records();
        all.set(Sin::Sloth, general::Record { met: true, decree_done, slain, ..Default::default() });
        g.warband.save.generals = all.encode();
        g.map_jump(save::Stage::Court(Sin::Sloth));
        g.enter(Screen::Playing);
        g.hero.free_move = true;
        g
    }

    #[test]
    fn his_general_slain_no_nightmares_come_and_his_decree_done_he_wakes_rested_with_them() {
        // The Lamplighter slain: asleep as ever, but no nightmares at his second phase.
        let mut g = with_general(false, true);
        assert!(belphegor(&mut g).puppet);
        let max = belphegor(&mut g).max_hp;
        strike(&mut g, 1.0);
        run(&mut g, 1.5);
        belphegor(&mut g).hp = max * 0.6;
        strike(&mut g, 2.0);
        run(&mut g, 1.0);
        assert_eq!(belphegor(&mut g).breed.phase, 2);
        assert_eq!(nightmares(&g), 0, "his nightmares are lost");
        assert!(belphegor(&mut g).lore.slumber.open(), "he dreams all the same");
        // The hourglass wound (his decree done): he wakes rested (no free blow) and his
        // nightmares come at once.
        let mut g = with_general(true, false);
        run(&mut g, 0.1);
        let d = belphegor(&mut g);
        assert!(!d.puppet && d.breed.phase == 1 && !d.lore.slumber.open(), "awake from the first");
        assert_eq!(nightmares(&g), NIGHTMARES as usize, "the nightmares at once");
        let before = belphegor(&mut g).hp;
        belphegor(&mut g).rest = u16::MAX;
        strike(&mut g, 20.0);
        assert!(before - belphegor(&mut g).hp < 10.0, "no free blow: his hide turns it");
        // ...and none come again at his second phase.
        belphegor(&mut g).hp = max * 0.6;
        strike(&mut g, 2.0);
        run(&mut g, 0.1);
        assert!(nightmares(&g) <= NIGHTMARES as usize);
    }

    /// PRINCES.md §0: by the bots, with the kit a hero has on reaching him (tiers 1 and 2), the
    /// expert brings Belphegor down through all three phases with lives to spare; a masher does not.
    #[test]
    fn the_expert_beats_belphegor_in_his_court_and_a_masher_does_not() {
        use crate::game::bot::{court_fight, Loadout, Skill};
        use crate::logic::heroes::HeroKind;
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        for (tier, seed) in [(1, 1), (2, 2)] {
            let kit = (tier > 1).then(|| Loadout::all(HeroKind::Ragnar, tier));
            let f = court_fight(Sin::Sloth, Skill::Expert, seed, kit, 200.0);
            assert!(f.slain && f.deaths < 3 && f.phase == 3, "tier {tier}: {f:?}");
            assert_eq!(f.stats.overlaps, 0, "{f:?}");
        }
        let f = court_fight(Sin::Sloth, Skill::Novice, 1, Some(Loadout::all(HeroKind::Ragnar, 2)), 200.0);
        assert!(!f.slain && f.game_over, "{f:?}");
    }
}
