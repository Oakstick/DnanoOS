//! Greed's court in play: the Counting House (`logic::court::greed` lays it out) and Mammon, its
//! prince. As the second phase of his health begins, two gold golems climb out of his heaps (at
//! once, if his general's decree was done); as he falls, his hoard spills.

use crate::audio::Sfx;
use crate::game::Game;
use crate::logic::combat::duelist::Lore;
use crate::logic::combat::math;
use crate::logic::combat::prince::{GOLEMS, GOLEM_RISE};
use crate::logic::court::{adds_weight, beside_heap, HEAPS};
use crate::logic::enemy::EnemyKind;
use crate::logic::types::{dist_sq, V};

impl Game {
    /// As Mammon is set before his throne.
    pub(super) fn greed_enter(&mut self, _prince: u32) {}

    /// Each tick of the fight in the Counting House.
    pub(super) fn greed_tick(&mut self, _dt: f64) {}

    /// As his health falls into `phase`: as the second begins, his golems climb out of the gold
    /// (unless they already have, or his general's death has lost them).
    pub(super) fn greed_phase(&mut self, prince: u32, at: V, phase: u8) {
        if phase >= 2 && self.adds_unsummoned(prince) {
            self.raise_golems(prince, at);
        }
    }

    /// After a fall (his golems are already gone).
    pub(super) fn greed_reset(&mut self) {}

    /// As he falls: his hoard spills.
    pub(super) fn greed_slain(&mut self, at: V) {
        self.floater(at.x, at.y - 1.8, "THE HOARD SPILLS", 2.0, Some(0xf2c53d));
    }

    /// The phase banner's line under his name, and its colour.
    pub(super) fn greed_banner(phase: u8) -> (&'static str, u32) {
        match phase {
            2 => ("HE RAISES HIS GOLD AGAINST YOU   STRIKE HIM TO BREAK HIS GRASP", 0xd8a040),
            _ => ("COINS FALL FROM ABOVE   STEP OFF THEM, THEN STRIKE", 0xff8a3a),
        }
    }

    /// His adds at once (his general's decree done): the golems.
    pub(super) fn greed_raise(&mut self, prince: u32, at: V) {
        self.raise_golems(prince, at);
    }

    /// Two gold golems climb out of the heaps nearest the hero (in the ring, out of the floor
    /// either side of the prince); they fall when he does.
    fn raise_golems(&mut self, id: u32, at: V) {
        let hp = self.hero.pos();
        let spots: Vec<V> = if self.in_court() {
            let mut heaps = HEAPS.to_vec();
            heaps.sort_by(|a, b| dist_sq(beside_heap(*a).v(), hp).total_cmp(&dist_sq(beside_heap(*b).v(), hp)));
            heaps.iter().take(GOLEMS as usize).map(|h| beside_heap(*h).v()).collect()
        } else {
            let across = math::norm(V::new(-(hp.y - at.y), hp.x - at.x));
            [-1.0, 1.0].iter().map(|s| math::add(at, math::scale(across, 2.2 * s))).collect()
        };
        let tier = self.realm_tier();
        let (weight_hp, weight_damage) = if self.in_court() { adds_weight(crate::logic::sins::Sin::Greed, tier) } else { (1.0, 1.0) };
        for p in spots {
            let facing = math::norm(math::sub(hp, p));
            let lore = Lore { master: Some(id), rise: GOLEM_RISE, ..Lore::default() };
            if let Some(gid) = self.spawn_duelist(EnemyKind::GoldGolem, p, facing, lore) {
                if let Some(g) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == gid) {
                    g.max_hp *= weight_hp;
                    g.hp = g.max_hp;
                    g.breed.damage *= weight_damage;
                }
            }
            self.view.dust(&self.level.grid, p.x, p.y, 0xe8b030, 24, 1.8);
            self.view.sparkle(&self.level.grid, p.x, p.y, 1.2, 0xffe070, 30);
        }
        self.floater(hp.x, hp.y - 1.5, "THE GOLD RISES", 1.6, Some(0xf2c53d));
        self.audio.play(Sfx::DoorGrind);
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{self, run};
    use super::super::{COURT_CADENCE, COURT_CAPACITY};
    use super::*;
    use crate::game::{save, Screen};
    use crate::logic::combat::archetype::Archetype;
    use crate::logic::combat::prince::{phase_of, GRASP_HOLD};
    use crate::logic::combat::{Duelist, MoveId, State};
    use crate::logic::sins::Sin;
    use crate::logic::types::{Tile, P};

    fn court() -> Game {
        testing::court(Sin::Greed)
    }

    fn mammon(g: &mut Game) -> &mut Duelist {
        testing::prince(g)
    }

    #[test]
    fn the_court_of_greed_is_the_counting_house_and_mammon_fights_there_by_frame_data() {
        let g = court();
        assert_eq!(g.level.data.name, "The Counting House");
        assert_eq!(g.realm, Some(Sin::Greed));
        assert!(g.boss.is_none(), "no Cyclops");
        assert!(g.arena(), "a boss's arena all the same");
        assert!(g.brawling(), "fought by frame data");
        let b = g.brawl.as_ref().unwrap();
        assert_eq!((b.threat.capacity, b.threat.cadence), (COURT_CAPACITY, COURT_CADENCE));
        let d = g.prince().expect("Mammon stands");
        assert_eq!((d.breed.kind, d.arch, d.breed.phase), (EnemyKind::Mammon, Archetype::Prince, 1));
        assert_eq!(d.stats().cost, 3);
        assert_eq!(g.enemies.len(), 1, "he waits alone");
        let bar = g.prince_bar().unwrap();
        assert_eq!((bar.name.as_str(), bar.title), ("MAMMON", "THE BANKER"));
        assert_eq!(g.keys_total, 1, "one key opens the gate");
    }

    #[test]
    fn his_phases_change_at_the_thresholds_and_the_golems_rise_once() {
        let mut g = court();
        let max = mammon(&mut g).max_hp;
        mammon(&mut g).rest = u16::MAX;
        let from = mammon(&mut g).pos;
        // A blow that leaves him above two thirds: still the Banker.
        mammon(&mut g).hp = max * 0.75;
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        let golems = |g: &Game| g.enemies.iter().filter(|e| e.kind() == EnemyKind::GoldGolem).count();
        // (His hide took most of it.)
        let hp = mammon(&mut g).hp;
        assert_eq!(mammon(&mut g).breed.phase, phase_of(hp, max));
        assert_eq!(golems(&g), 0);
        // Under two thirds: the Collector, his golems, a green cross.
        mammon(&mut g).hp = max * 0.6;
        mammon(&mut g).rest = u16::MAX;
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        assert_eq!(mammon(&mut g).breed.phase, 2);
        assert_eq!(g.court.as_ref().unwrap().phase, 2);
        assert_eq!(golems(&g), GOLEMS as usize, "two golems out of the gold");
        assert!(g.level.grid.count(Tile::Charge) >= 1, "a breather");
        assert!(g.juice.banner.as_ref().is_some_and(|b| b.text.contains("THE COLLECTOR")));
        assert_eq!(mammon(&mut g).breed.moves()[0], MoveId::Grasp);
        // Under a third: the Hoard. The golems are not raised again.
        mammon(&mut g).hp = max * 0.3;
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        assert_eq!(mammon(&mut g).breed.phase, 3);
        assert_eq!(golems(&g), GOLEMS as usize, "raised once");
        assert_eq!(mammon(&mut g).breed.moves()[0], MoveId::CoinRain);
    }

    #[test]
    fn his_hide_turns_a_blow_but_not_in_his_recovery() {
        let mut g = court();
        let from = mammon(&mut g).pos;
        mammon(&mut g).rest = u16::MAX;
        let before = mammon(&mut g).hp;
        g.call_duel_hit(0, from);
        let warded = before - mammon(&mut g).hp;
        let d = mammon(&mut g);
        d.f.state = State::Attack { mv: MoveId::Sceptre, frame: 60 };
        let before = d.hp;
        g.call_duel_hit(0, from);
        let open = before - mammon(&mut g).hp;
        assert!(warded > 0.0 && open > warded * 1.5, "warded {warded}, open {open}");
    }

    #[test]
    fn his_grasp_holds_him_and_drains_his_silver_until_he_strikes() {
        let mut g = court();
        g.progress.silver = 500;
        let hp = g.hero.pos();
        let d = mammon(&mut g);
        d.pos = V::new(hp.x, hp.y - 1.9);
        d.facing = V::new(0.0, 1.0);
        d.rest = u16::MAX;
        d.breed.phase = 2;
        d.hp = d.max_hp * 0.5;
        d.token = true;
        assert!(d.begin(MoveId::Grasp, hp));
        let mut held = false;
        for _ in 0..400 {
            g.update(crate::app::STEP);
            if g.brawl.as_ref().unwrap().held.is_some() {
                held = true;
                break;
            }
        }
        assert!(held, "the grasp closes on him");
        let hp0 = mammon(&mut g).hp;
        let at = g.hero.pos();
        run(&mut g, 1.2);
        assert!(g.progress.silver < 500, "his silver drains: {}", g.progress.silver);
        assert!(mammon(&mut g).hp > hp0, "and heals the prince");
        assert!(math::len(math::sub(g.hero.pos(), at)) < 0.05, "held where he is");
        assert!(mammon(&mut g).lore.hold > 0 && mammon(&mut g).lore.hold < GRASP_HOLD);
        // A blow breaks the grasp.
        let from = g.hero.pos();
        g.call_duel_hit(0, from);
        assert!(g.brawl.as_ref().unwrap().held.is_none(), "free");
        assert_eq!(mammon(&mut g).lore.hold, 0);
        let silver = g.progress.silver;
        run(&mut g, 1.0);
        assert_eq!(g.progress.silver, silver, "no more drained");
    }

    #[test]
    fn his_coins_lie_on_the_floor_then_burst_into_spikes() {
        let mut g = court();
        let hp = g.hero.pos();
        let d = mammon(&mut g);
        d.pos = V::new(hp.x, hp.y - 4.0);
        d.facing = V::new(0.0, 1.0);
        d.rest = u16::MAX;
        d.token = true;
        assert!(d.begin(MoveId::CoinToss, hp));
        let start = g.hero.hp;
        let mut seen = 0;
        let mut burst = false;
        for _ in 0..400 {
            g.update(crate::app::STEP);
            let b = g.brawl.as_ref().unwrap();
            seen = seen.max(b.traps.len());
            burst |= b.traps.iter().any(|t| t.burst.is_some());
            if burst {
                break;
            }
        }
        assert_eq!(seen, 5, "five coins on the floor");
        assert!(burst, "they burst");
        run(&mut g, 0.2);
        assert!(g.hero.hp < start, "standing on one, he is hurt: {} of {start}", g.hero.hp);
    }

    #[test]
    fn mammon_slain_drops_the_key_of_greed_and_seals_greed() {
        let mut g = court();
        let from = mammon(&mut g).pos;
        mammon(&mut g).hp = 1.0;
        mammon(&mut g).f.state = State::Attack { mv: MoveId::Sceptre, frame: 60 };
        mammon(&mut g).breed.phase = 3;
        g.call_duel_hit(0, from);
        assert!(g.prince().is_none(), "he has fallen");
        assert!(g.court.as_ref().unwrap().slain);
        assert!(g.red_key.is_some(), "the Key of Greed lies on the floor");
        assert!(g.sins_cleared & Sin::Greed.bit() != 0, "Greed's bit is set");
        assert_eq!(g.key_name(), "THE KEY OF GREED");
        assert!(g.saved_run.as_ref().is_some_and(|r| r.stage == save::Stage::SinShop(Sin::Greed)), "and the checkpoint is the old man's");
        // He takes the key, the gate opens, and through it he goes on to the old man.
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
    fn a_fall_brings_mammon_back_with_his_wounds_and_continue_comes_back_to_his_court() {
        let mut g = court();
        let max = mammon(&mut g).max_hp;
        mammon(&mut g).hp = max * 0.5;
        mammon(&mut g).breed.phase = 2;
        g.reset_round();
        let d = mammon(&mut g);
        assert_eq!((d.hp, d.breed.phase), (max * 0.5, 2), "his wounds kept (WARBAND.md W10)");
        // As the fight begins again, his golems climb out of the gold once more.
        g.enter(Screen::Playing);
        run(&mut g, 0.05);
        assert_eq!(g.enemies.iter().filter(|e| e.kind() == EnemyKind::GoldGolem).count(), GOLEMS as usize);
        // Saved at the court's door, Continue loads it again.
        g.checkpoint(save::Stage::Court(Sin::Greed));
        let run = g.saved_run.clone().unwrap();
        let mut h = Game::new();
        h.saved_run = Some(run);
        h.continue_game();
        assert!(h.in_court() && h.prince().is_some_and(|d| d.hp == d.max_hp), "a game over gives him back his health");
        assert_eq!(h.level.data.name, "The Counting House");
    }
}
