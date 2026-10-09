//! Wrath's court in play: the Anvil of Wrath (`logic::court::wrath` lays it out) and Amon, its
//! prince (PRINCES.md §3). Wrath has no hide: every blow he takes quickens him (his rage, kept by
//! `brawl::duel_hit` in `Lore::rage`), and what spends it is stone. His charge runs straight along
//! its locked aim; one that runs into a pillar, the anvil or the wall leaves him dazed (staggered,
//! every blow landing whole and more) with his rage gone. The Fury charges twice over: a charge
//! that ends on open floor is followed at once by a second (`Lore::twin`), as the threat manager
//! allows. As the second phase begins his ravens come down from the burning banners (at once, if
//! his general's decree was done; never, if his general fell).

use crate::audio::{Cry, Sfx};
use crate::game::Game;
use crate::logic::combat::duelist::Lore;
use crate::logic::combat::fsm::Reel;
use crate::logic::combat::math;
use crate::logic::combat::prince::wrath::{DAZE, RAVENS, RAVEN_DESCENT, TWIN_REST};
use crate::logic::combat::{MoveId, State};
use crate::logic::court::adds_weight;
use crate::logic::court::wrath::{below_perch, PERCHES};
use crate::logic::enemy::EnemyKind;
use crate::logic::types::{dist_sq, V};

/// Fire's colours, for his court's floaters and sparks.
const EMBER: u32 = 0xff7a2a;
const FLAME: u32 = 0xffb050;

impl Game {
    /// As Amon is set before his anvil.
    pub(super) fn wrath_enter(&mut self, _prince: u32) {}

    /// Each tick of the fight at the Anvil: a charge run into stone dazes him and spends his rage;
    /// the Fury's charge that ends on open floor has a second follow it.
    pub(super) fn wrath_tick(&mut self, _dt: f64) {
        let Some(id) = self.court.as_ref().filter(|c| !c.slain).map(|c| c.prince) else { return };
        let Some(d) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == id && d.alive()) else { return };
        let charging = matches!(d.f.state, State::Attack { mv: MoveId::FieryCharge, .. });
        if charging && d.crashed {
            // Into stone: he reels, dazed, his rage gone out of him.
            d.f.state = State::Reel { kind: Reel::Staggered, left: DAZE, total: DAZE };
            d.lore.rage = 0;
            d.lore.twin = 0;
            d.vel = V::new(0.0, 0.0);
            let at = d.pos;
            self.floater(at.x, at.y - 2.2, "DAZED: STRIKE NOW", 1.6, Some(FLAME));
            self.audio.play(Sfx::BossRoar);
            self.view.shake(0.4);
            self.view.sparkle(&self.level.grid, at.x, at.y, 1.2, 0xffe070, 26);
            return;
        }
        // The Fury's two charges: 1 the first under way, 2 the second due, 3 the second under way.
        let phase = d.breed.phase;
        match (d.f.state, d.lore.twin) {
            (State::Attack { mv: MoveId::FieryCharge, .. }, 0) => d.lore.twin = 1,
            (State::Attack { mv: MoveId::FieryCharge, .. }, 2) => d.lore.twin = 3,
            (State::Attack { .. }, 2) => d.lore.twin = 0,
            (State::Neutral, 1) if phase >= 3 => {
                // The first ran its length: the second is due at once (the threat manager still plans it).
                if let Some(k) = d.breed.moves().iter().position(|&m| m == MoveId::FieryCharge) {
                    if let Some(c) = d.cool.get_mut(k) {
                        *c = 0;
                    }
                }
                d.rest = d.rest.min(TWIN_REST);
                d.lore.twin = 2;
            }
            (State::Neutral | State::Reel { .. }, 1 | 3) => d.lore.twin = 0,
            _ => {}
        }
    }

    /// As his health falls into `phase`: as the second begins, his ravens come down (unless they
    /// already have, or his general's death has lost them).
    pub(super) fn wrath_phase(&mut self, prince: u32, at: V, phase: u8) {
        if phase >= 2 && self.adds_unsummoned(prince) {
            self.raise_ravens(prince, at);
        }
    }

    /// After a fall (his ravens are already gone; his rage went with his health).
    pub(super) fn wrath_reset(&mut self) {}

    /// As he falls: the fire in his throat goes out.
    pub(super) fn wrath_slain(&mut self, at: V) {
        self.floater(at.x, at.y - 1.8, "THE FIRE GOES OUT OF HIM", 2.0, Some(EMBER));
        self.view.dust(&self.level.grid, at.x, at.y, 0x2a2420, 30, 3.0);
    }

    /// The phase banner's line under his name, and its colour.
    pub(super) fn wrath_banner(phase: u8) -> (&'static str, u32) {
        match phase {
            2 => ("HIS RAVENS COME   THE FLOOR SPLITS: STEP OFF THE GLOWING SEAM", 0xe0602a),
            _ => ("HE CHARGES TWICE   STAND BEFORE STONE, THEN STEP ASIDE", 0xff3a1a),
        }
    }

    /// His adds at once (his general's decree done): the ravens.
    pub(super) fn wrath_raise(&mut self, prince: u32, at: V) {
        self.raise_ravens(prince, at);
    }

    /// His ravens come down from the burning banners nearest the hero (not on top of him; in the
    /// ring, out of the air either side of the prince); they fall when he does.
    fn raise_ravens(&mut self, id: u32, at: V) {
        let hp = self.hero.pos();
        let spots: Vec<V> = if self.in_court() {
            let mut perches: Vec<V> = PERCHES.iter().map(|&p| below_perch(p).v()).filter(|p| dist_sq(*p, hp) > 2.5 * 2.5).collect();
            perches.sort_by(|a, b| dist_sq(*a, hp).total_cmp(&dist_sq(*b, hp)));
            perches.into_iter().take(RAVENS as usize).collect()
        } else {
            let across = math::norm(V::new(-(hp.y - at.y), hp.x - at.x));
            [-1.0, 1.0, 0.0].iter().map(|s| math::add(at, math::scale(across, 2.2 * s + 0.01))).take(RAVENS as usize).collect()
        };
        let tier = self.realm_tier();
        let (weight_hp, weight_damage) =
            if self.in_court() { self.ng_foe(adds_weight(crate::logic::sins::Sin::Wrath, tier)) } else { (1.0, 1.0) };
        for p in spots {
            let facing = math::norm(math::sub(hp, p));
            let lore = Lore { master: Some(id), rise: RAVEN_DESCENT, ..Lore::default() };
            if let Some(rid) = self.spawn_duelist(EnemyKind::Raven, p, facing, lore) {
                if let Some(r) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == rid) {
                    r.max_hp *= weight_hp;
                    r.hp = r.max_hp;
                    r.breed.damage *= weight_damage;
                }
            }
            self.view.dust(&self.level.grid, p.x, p.y, 0x2a2030, 18, 1.6);
            self.view.sparkle(&self.level.grid, p.x, p.y, 1.0, EMBER, 18);
            self.audio.foe_cry(EnemyKind::Raven, Cry::Alert, p);
        }
        self.floater(hp.x, hp.y - 1.5, "THE RAVENS COME", 1.6, Some(EMBER));
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{self, run};
    use super::super::{COURT_CADENCE, COURT_CAPACITY};
    use super::*;
    use crate::game::{save, Screen};
    use crate::logic::combat::archetype::Archetype;
    use crate::logic::combat::prince::wrath::RAGE_MAX;
    use crate::logic::combat::prince::{phase_of, LINGER_EVERY};
    use crate::logic::combat::Duelist;
    use crate::logic::court::wrath::{pillar_middle, PILLARS, PRINCE};
    use crate::logic::sins::Sin;
    use crate::logic::types::{Tile, P};
    use crate::logic::war::general;

    fn court() -> Game {
        testing::court(Sin::Wrath)
    }

    fn amon(g: &mut Game) -> &mut Duelist {
        testing::prince(g)
    }

    fn ravens(g: &Game) -> usize {
        g.enemies.iter().filter(|e| e.kind() == EnemyKind::Raven && e.duel.as_ref().is_some_and(|d| d.alive())).count()
    }

    /// Amon, still and out of the threat manager's way, at `at` facing `facing`, about to begin
    /// `mv` on the hero.
    fn set(g: &mut Game, at: V, facing: V) {
        let d = amon(g);
        d.pos = at;
        d.facing = facing;
        d.rest = u16::MAX;
        d.token = true;
        d.f.state = State::Neutral;
    }

    #[test]
    fn the_court_of_wrath_is_the_anvil_and_amon_fights_there_by_frame_data() {
        let g = court();
        assert_eq!(g.level.data.name, "The Anvil of Wrath");
        assert_eq!(g.realm, Some(Sin::Wrath));
        assert!(g.boss.is_none(), "no Cyclops");
        assert!(g.arena(), "a boss's arena all the same");
        assert!(g.brawling(), "fought by frame data");
        let b = g.brawl.as_ref().unwrap();
        assert_eq!((b.threat.capacity, b.threat.cadence), (COURT_CAPACITY, COURT_CADENCE));
        let d = g.prince().expect("Amon stands");
        assert_eq!((d.breed.kind, d.arch, d.breed.phase), (EnemyKind::Amon, Archetype::Prince, 1));
        assert_eq!(d.stats().cost, 3);
        assert_eq!(d.pos, P::new(PRINCE.0, PRINCE.1).v());
        assert_eq!(g.enemies.len(), 1, "he waits alone");
        let bar = g.prince_bar().unwrap();
        assert_eq!((bar.name.as_str(), bar.title), ("AMON", "THE WOLF"));
        assert_eq!(g.keys_total, 1, "one key opens the gate");
    }

    #[test]
    fn his_phases_change_at_the_thresholds_and_the_ravens_come_once() {
        let mut g = court();
        let max = amon(&mut g).max_hp;
        amon(&mut g).rest = u16::MAX;
        let from = amon(&mut g).pos;
        amon(&mut g).hp = max * 0.67 + 30.0;
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        let hp = amon(&mut g).hp;
        assert_eq!(amon(&mut g).breed.phase, phase_of(hp, max));
        assert_eq!(ravens(&g), 0);
        // Under two thirds: the Raven, his ravens, a green cross.
        amon(&mut g).hp = max * 0.6;
        amon(&mut g).rest = u16::MAX;
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        assert_eq!(amon(&mut g).breed.phase, 2);
        assert_eq!(g.court.as_ref().unwrap().phase, 2);
        assert_eq!(ravens(&g), RAVENS as usize, "his ravens come down");
        assert!(g.level.grid.count(Tile::Charge) >= 1, "a breather");
        assert!(g.juice.banner.as_ref().is_some_and(|b| b.text.contains("THE RAVEN")));
        assert_eq!(amon(&mut g).breed.moves()[0], MoveId::Rift);
        // Under a third: the Fury. The ravens are not called again.
        amon(&mut g).hp = max * 0.3;
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        assert_eq!(amon(&mut g).breed.phase, 3);
        assert_eq!(ravens(&g), RAVENS as usize, "called once");
        assert_eq!(amon(&mut g).breed.moves(), &[MoveId::FieryCharge, MoveId::Eruption, MoveId::FlameBreath]);
        assert!(g.juice.banner.as_ref().is_some_and(|b| b.text.contains("THE FURY") && b.sub.contains("TWICE")));
    }

    #[test]
    fn wrath_has_no_hide_every_blow_lands_whole_and_quickens_him_up_to_five_steps() {
        let mut g = court();
        let from = amon(&mut g).pos;
        amon(&mut g).rest = u16::MAX;
        let walk = amon(&mut g).stats().walk;
        let before = amon(&mut g).hp;
        g.call_duel_hit(0, from);
        let neutral = before - amon(&mut g).hp;
        assert_eq!(amon(&mut g).lore.rage, 1, "a blow, a step of rage");
        assert!(amon(&mut g).stats().walk > walk, "and he is quicker for it");
        // In a recovery a blow lands as a punish, no more: there is no hide to open.
        let d = amon(&mut g);
        d.f = d.breed.fighter();
        d.f.state = State::Attack { mv: MoveId::FlameBreath, frame: 60 };
        let before = d.hp;
        g.call_duel_hit(0, from);
        let open = before - amon(&mut g).hp;
        assert!(neutral > 0.0 && open < neutral * 1.6, "neutral {neutral}, open {open}");
        for _ in 0..8 {
            let d = amon(&mut g);
            d.f = d.breed.fighter();
            g.call_duel_hit(0, from);
        }
        assert_eq!(amon(&mut g).lore.rage, RAGE_MAX, "five steps and no more");
        // A fall: whole again, and calm.
        g.reset_round();
        assert_eq!(amon(&mut g).lore.rage, 0);
    }

    /// Amon west of a pillar, the hero standing before its stone; he charges, the hero steps aside
    /// as the aim locks, and the charge runs on into the stone.
    #[test]
    fn a_charge_run_into_stone_dazes_him_and_spends_his_rage_and_on_open_floor_it_does_not() {
        let mut g = court();
        let pillar = PILLARS[1];
        let m = pillar_middle(pillar);
        let stand = P::new(pillar.0 - 1, pillar.1);
        g.hero.place(stand);
        let hp = g.hero.pos();
        set(&mut g, V::new(m.0 - 6.5, hp.y), V::new(1.0, 0.0));
        amon(&mut g).lore.rage = 4;
        assert!(amon(&mut g).begin(MoveId::FieryCharge, hp));
        // Through his tracking frames, then aside (north, out of his run) as the aim holds still.
        let track = MoveId::FieryCharge.data().track;
        for _ in 0..(track as usize + 2) * 2 {
            g.update(crate::app::STEP);
        }
        g.hero.place(P::new(stand.x - 1, stand.y - 3));
        let mut dazed = false;
        for _ in 0..240 {
            g.update(crate::app::STEP);
            if amon(&mut g).f.reeling() == Some(Reel::Staggered) {
                dazed = true;
                break;
            }
        }
        assert!(dazed, "into the stone, and dazed: {:?}", amon(&mut g).pos);
        let d = amon(&mut g);
        assert!(d.exposed() && d.lore.rage == 0, "dazed, open, his rage spent");
        assert!(d.pos.x < m.0 - 1.0, "stopped short of the pillar: {:?}", d.pos);
        // Dazed a long while: every blow lands, and more than whole.
        let frames = match d.f.state {
            State::Reel { left, .. } => left,
            _ => 0,
        };
        assert!(frames > 120, "{frames}");
        assert!(d.f.vulnerability() > 1.0);

        // On open floor the charge runs its length and he recovers, undazed.
        let mut g = court();
        g.hero.place(P::new(16, 20));
        let hp = g.hero.pos();
        set(&mut g, V::new(16.0, 13.0), V::new(0.0, 1.0));
        assert!(amon(&mut g).begin(MoveId::FieryCharge, hp));
        let track = MoveId::FieryCharge.data().track;
        for _ in 0..(track as usize + 2) * 2 {
            g.update(crate::app::STEP);
        }
        g.hero.place(P::new(12, 23));
        g.hero.hp = 1000;
        run(&mut g, 1.6);
        let d = amon(&mut g);
        assert!(d.f.reeling().is_none() && !d.crashed, "{:?}", d.f.state);
        assert!(d.pos.y > 18.0, "he ran on: {:?}", d.pos);
    }

    #[test]
    fn the_fury_charges_twice_over() {
        let mut g = court();
        g.hero.hp = 1000;
        g.hero.place(P::new(16, 22));
        let hp = g.hero.pos();
        let d = amon(&mut g);
        d.breed.phase = 3;
        d.hp = d.max_hp * 0.3;
        d.lore.summoned = true;
        set(&mut g, V::new(16.0, 12.0), V::new(0.0, 1.0));
        assert!(amon(&mut g).begin(MoveId::FieryCharge, hp));
        // Off his line at once: the charge runs past him.
        g.hero.place(P::new(11, 18));
        let mut charges = 0;
        let mut was = false;
        for _ in 0..(6.0 / crate::app::STEP) as usize {
            g.update(crate::app::STEP);
            let now = matches!(amon(&mut g).f.state, State::Attack { mv: MoveId::FieryCharge, .. });
            if now && !was {
                charges += 1;
            }
            was = now;
            if charges >= 2 {
                break;
            }
        }
        assert!(charges >= 2, "a second charge follows the first");
    }

    #[test]
    fn the_rift_glows_late_enough_to_step_off_and_its_lava_stings_while_he_stands_in_it() {
        let mut g = court();
        g.hero.hp = 1000;
        g.hero.place(P::new(16, 20));
        let hp = g.hero.pos();
        set(&mut g, V::new(16.0, 14.0), V::new(0.0, 1.0));
        amon(&mut g).breed.phase = 2;
        amon(&mut g).lore.summoned = true;
        assert!(amon(&mut g).begin(MoveId::Rift, hp));
        // The seam is laid along his aim, toward him.
        let mut laid = Vec::new();
        for _ in 0..400 {
            g.update(crate::app::STEP);
            let b = g.brawl.as_ref().unwrap();
            if b.traps.iter().any(|t| t.armed()) {
                laid = b.traps.clone();
                break;
            }
        }
        assert!(laid.len() >= 4, "a row of lava: {}", laid.len());
        assert!(laid.iter().all(|t| (t.pos.x - 16.0).abs() < 0.3 && t.pos.y > 14.0), "along his aim");
        let on = laid.iter().find(|t| t.reaches(hp, 0.3)).expect("the seam runs under him");
        // As it lands it glows, and gives time to walk off it with a fifth to spare.
        let walk = 6.5 * crate::logic::levels::SPEED_SCALE;
        let gained = on.fuse.saturating_sub(crate::logic::combat::frames::REACTION) as f64 * walk / 60.0 - 0.25;
        assert!(gained >= (on.radius + 0.3) * 1.2, "{gained}");
        // Stepped off at once: untouched by the burst.
        let start = g.hero.hp;
        g.hero.place(P::new(18, 20));
        let mut burst = false;
        for _ in 0..300 {
            g.update(crate::app::STEP);
            if g.brawl.as_ref().unwrap().traps.iter().any(|t| t.burst.is_some()) {
                burst = true;
                break;
            }
        }
        assert!(burst);
        run(&mut g, 0.2);
        assert_eq!(g.hero.hp, start, "off the seam, untouched");
        // Stood in the lava as it lingers, it stings again and again (and never holds him).
        let t = g.brawl.as_ref().unwrap().traps[2];
        g.hero.place(P::new(t.pos.x.round() as i32, t.pos.y.round() as i32));
        amon(&mut g).rest = u16::MAX;
        amon(&mut g).token = false;
        let before = g.hero.hp;
        run(&mut g, LINGER_EVERY as f64 * 3.0 / 60.0);
        assert!(g.hero.hp < before, "stung: {} of {before}", g.hero.hp);
        assert_eq!(g.hero.root, 1.0, "never slowed");
    }

    #[test]
    fn amon_slain_drops_the_key_of_wrath_and_the_way_on_is_open() {
        let mut g = court();
        let from = amon(&mut g).pos;
        amon(&mut g).hp = 1.0;
        amon(&mut g).f.state = State::Attack { mv: MoveId::AxeCleave, frame: 60 };
        amon(&mut g).breed.phase = 3;
        g.call_duel_hit(0, from);
        assert!(g.prince().is_none(), "he has fallen");
        assert!(g.court.as_ref().unwrap().slain);
        assert!(g.red_key.is_some(), "the Key of Wrath lies on the floor");
        assert!(g.sins_cleared & Sin::Wrath.bit() != 0, "Wrath's bit is set");
        assert_eq!(g.key_name(), "THE KEY OF WRATH");
        assert!(g.saved_run.as_ref().is_some_and(|r| r.stage == save::Stage::SinShop(Sin::Wrath)), "the checkpoint is the old man's");
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
    fn a_fall_brings_amon_back_with_his_wounds_and_calm_and_continue_comes_back_to_his_court() {
        let mut g = court();
        let max = amon(&mut g).max_hp;
        amon(&mut g).hp = max * 0.5;
        amon(&mut g).breed.phase = 2;
        amon(&mut g).lore.rage = 3;
        amon(&mut g).pos = V::new(10.0, 18.0);
        g.reset_round();
        let d = amon(&mut g);
        assert_eq!((d.hp, d.breed.phase, d.lore.rage), (max * 0.5, 2, 0), "his wounds kept (WARBAND.md W10), his rage gone");
        assert_eq!(d.pos, P::new(PRINCE.0, PRINCE.1).v(), "back before his anvil");
        g.checkpoint(save::Stage::Court(Sin::Wrath));
        let saved = g.saved_run.clone().unwrap();
        let mut h = Game::new();
        h.saved_run = Some(saved);
        h.continue_game();
        assert!(h.in_court() && h.prince().is_some_and(|d| d.hp == d.max_hp), "a game over gives him back his health");
        assert_eq!(h.level.data.name, "The Anvil of Wrath");
    }

    #[test]
    fn his_general_slain_no_ravens_come_and_his_decree_done_they_come_at_once() {
        let at_court = |record: general::Record| {
            let mut g = court();
            let mut all = g.general_records();
            all.set(Sin::Wrath, record);
            g.warband.save.generals = all.encode();
            g.map_jump(save::Stage::Court(Sin::Wrath));
            g.enter(Screen::Playing);
            g.hero.free_move = true;
            g
        };
        // Marshal Gorebrand slain: at two thirds no ravens.
        let mut g = at_court(general::Record { met: true, slain: true, ..Default::default() });
        assert!(amon(&mut g).lore.summoned, "his ravens are lost");
        let max = amon(&mut g).max_hp;
        let from = amon(&mut g).pos;
        amon(&mut g).hp = max * 0.6;
        g.call_duel_hit(0, from);
        run(&mut g, 0.1);
        assert_eq!(amon(&mut g).breed.phase, 2);
        assert_eq!(ravens(&g), 0, "no ravens");
        // His decree done (the bridges burnt): the ravens come as the fight begins, and not again.
        let mut g = at_court(general::Record { met: true, decree_done: true, ..Default::default() });
        run(&mut g, 0.1);
        assert_eq!(ravens(&g), RAVENS as usize, "they come at once");
        let from = amon(&mut g).pos;
        amon(&mut g).hp = max * 0.6;
        g.call_duel_hit(0, from);
        run(&mut g, 0.1);
        assert_eq!(ravens(&g), RAVENS as usize, "and not again");
    }

    /// PRINCES.md §0: Amon in his court by the bots: with the kit a hero has on reaching him
    /// (tiers 1 and 2) the expert brings him down through all three phases with lives to spare;
    /// a masher loses every life to him.
    #[test]
    fn the_expert_beats_amon_in_his_court_and_a_masher_does_not() {
        use crate::game::bot::{court_fight, Loadout, Skill};
        use crate::logic::heroes::HeroKind;
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        for (tier, seed) in [(1, 1), (2, 2)] {
            let kit = (tier > 1).then(|| Loadout::all(HeroKind::Ragnar, tier));
            let f = court_fight(Sin::Wrath, Skill::Expert, seed, kit, 240.0);
            assert!(f.slain && f.deaths <= 1 && f.phase == 3, "tier {tier}: {f:?}");
            assert_eq!(f.stats.overlaps, 0, "{f:?}");
        }
        // The masher where the run meets Amon, the fourth court in either order (WARBAND.md W10: his
        // blows tempered for the adept, a masher with three lives fells him at the first court's
        // weight, where no run meets him).
        let (slain, _, _, _, lost) =
            crate::game::whole_run::court_at(Sin::Wrath, Skill::Novice, 1, Loadout::all(HeroKind::Ragnar, 2), 3, 240.0);
        assert!(!slain && lost == 3, "{slain} {lost}");
    }
}
