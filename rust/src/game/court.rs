//! Greed's court in play: the Counting House (`logic::court` lays it out) and Mammon, its prince.
//! He is a duelist like every foe (`brawl`, COMBAT.md §13): the fourth archetype, a table of moves
//! for each phase of his health, a hide only his recoveries open, a token of three. The court is
//! fought under the vaults' rules (his real health, his shield and his blade), with a deeper pool
//! of threat for the prince and what he raises.
//!
//! As his health falls into a new phase a banner names it and a green cross falls for a breather;
//! as the second begins, two gold golems climb out of his heaps. When he falls he drops the Key of
//! Greed, and the way on is as from any boss: the key opens the gate behind him, then the old
//! man's wares, then the Portals of Hell with Greed sealed. If the hero falls, Mammon is whole
//! again and back before his throne.

use super::brawl::{Brawl, Field};
use super::{Banner, Game};
use crate::audio::Sfx;
use crate::logic::combat::archetype::Archetype;
use crate::logic::combat::duelist::Lore;
use crate::logic::combat::prince::{phase_name, GOLEMS, GOLEM_RISE};
use crate::logic::combat::{math, Breed, Duelist};
use crate::logic::court::{beside_heap, weight, HEAPS, MAMMON};
use crate::logic::enemy::{spec_of, Enemy, EnemyKind, EnemyMode};
use crate::logic::types::{dist_sq, Tile, P, V};

/// The court's pace: threat points (the prince's three: while he strikes, what he raises waits) and
/// frames between attack tokens.
pub const COURT_CAPACITY: u8 = 3;
pub const COURT_CADENCE: u32 = 45;

/// What the court keeps of its fight: who the prince is, his phase as last seen, whether he has
/// fallen, and his health as the bar showed it a moment ago (its pale trail).
#[derive(Clone, Debug, PartialEq)]
pub struct Court {
    pub prince: u32,
    pub phase: u8,
    pub slain: bool,
    pub trail: f32,
    /// His golems are to climb out as his fight begins (his general's decree was done, WARBAND.md §9).
    pub golems_due: bool,
}

/// The prince's bar on the HUD: his name, the phase's title, his health and its trail, and
/// whether his hide is open (`render::hud::court`).
#[derive(Clone, Debug, PartialEq)]
pub struct PrinceBar {
    pub name: String,
    pub phase: u8,
    pub title: &'static str,
    pub hp: f32,
    pub max: f32,
    pub trail: f32,
    pub open: bool,
    pub flash: bool,
}

impl Game {
    /// In Greed's court, the Counting House.
    pub fn in_court(&self) -> bool {
        self.court.is_some()
    }

    /// The prince, while he stands.
    pub fn prince(&self) -> Option<&Duelist> {
        let id = self.court.as_ref()?.prince;
        self.enemies.iter().filter_map(|e| e.duel.as_deref()).find(|d| d.id == id && d.alive())
    }

    /// Where the chase camera turns to in a prince's court: the prince, while he stands and the
    /// fight is on.
    pub(super) fn prince_focus(&self) -> Option<glam::Vec3> {
        if !matches!(self.screen, super::Screen::Playing | super::Screen::Paused) {
            return None;
        }
        self.prince().map(|d| glam::Vec3::new(d.pos.x as f32, 0.0, d.pos.y as f32))
    }

    /// What the HUD shows of the prince's health (none once he has fallen).
    pub fn prince_bar(&self) -> Option<PrinceBar> {
        let c = self.court.as_ref()?;
        let d = self.prince()?;
        Some(PrinceBar {
            name: self.boss_name(),
            phase: d.breed.phase,
            title: phase_name(d.breed.phase),
            hp: d.hp.max(0.0) as f32,
            max: d.max_hp as f32,
            trail: c.trail,
            open: d.exposed(),
            flash: d.flash > 0,
        })
    }

    /// Lays out the court as the level is loaded: the prince before his throne, the fight's rules.
    pub(super) fn enter_court(&mut self) {
        let tier = self.realm_tier();
        let mut brawl = Brawl::new(Field::Vault);
        brawl.reset(self.hero_kit());
        brawl.threat.capacity = COURT_CAPACITY;
        brawl.threat.cadence = COURT_CADENCE;
        brawl.foe_damage = 1.0;
        let id = brawl.next_id();
        let at = P::new(MAMMON.0, MAMMON.1);
        let (hp, damage) = weight(tier);
        let spec = spec_of(&self.level.grid, EnemyKind::Mammon);
        let mut e = Enemy::new(&spec, at, self.level.data.enemy_speed);
        e.post = Some(at);
        e.b.mode = EnemyMode::Chase;
        e.b.name = spec.name.to_uppercase();
        let mut d = Duelist::bred(id, Breed::of(EnemyKind::Mammon), at.v(), V::new(0.0, 1.0));
        d.max_hp *= hp;
        d.hp = d.max_hp;
        d.breed.damage *= damage;
        d.rest = 90;
        // His general's realm weighs on his fight: slain, his phase-two adds are lost; his decree
        // done, his golems come at once.
        let terms = self.prince_terms(crate::logic::sins::Sin::Greed);
        d.lore.summoned = !terms.phase_two_adds;
        e.duel = Some(Box::new(d));
        self.enemies = vec![e];
        self.brawl = Some(brawl);
        let trail = self.enemies[0].duel.as_ref().map(|d| d.max_hp as f32).unwrap_or(0.0);
        self.court = Some(Court { prince: id, phase: 1, slain: false, trail, golems_due: terms.golems_at_start });
    }

    /// After a fall: the prince whole again before his throne, his golems gone (unless he has
    /// already fallen, and his key waits).
    pub(super) fn reset_court(&mut self) {
        let Some(c) = self.court.clone() else { return };
        if c.slain {
            return;
        }
        let keep = c.prince;
        self.enemies.retain(|e| e.duel.as_ref().is_some_and(|d| d.id == keep));
        let home = P::new(MAMMON.0, MAMMON.1).v();
        let terms = self.prince_terms(crate::logic::sins::Sin::Greed);
        if let Some(d) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).next() {
            d.reset_home();
            d.pos = home;
            d.facing = V::new(0.0, 1.0);
            d.rest = 90;
            d.lore.summoned = !terms.phase_two_adds;
        }
        if let Some(c) = self.court.as_mut() {
            c.phase = 1;
            c.golems_due = terms.golems_at_start;
            c.trail = self.enemies.first().and_then(|e| e.duel.as_ref()).map(|d| d.max_hp as f32).unwrap_or(0.0);
        }
        self.refresh_foe_views();
    }

    /// The court each tick: the fallen cleared away, the bar's trail catching up, the key taken.
    pub(super) fn update_court(&mut self, dt: f64) {
        if self.enemies.iter().any(|e| e.mode() == EnemyMode::Eaten) {
            self.enemies.retain(|e| e.mode() != EnemyMode::Eaten);
            self.refresh_foe_views();
        }
        // The golems his general's decree sent ahead, as the fight begins.
        let due = self.court.as_ref().is_some_and(|c| c.golems_due) && self.screen == super::Screen::Playing;
        if let Some((id, pos)) = self.prince().map(|d| (d.id, d.pos)).filter(|_| due) {
            if let Some(c) = self.court.as_mut() {
                c.golems_due = false;
            }
            if let Some(d) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == id) {
                d.lore.summoned = true;
            }
            self.raise_golems(id, pos);
        }
        let hp = self.prince().map(|d| d.hp.max(0.0) as f32);
        if let (Some(c), Some(hp)) = (self.court.as_mut(), hp) {
            // The pale trail of what the last blows cost, catching up after a moment.
            c.trail = if c.trail > hp { (c.trail - (dt as f32) * 120.0).max(hp) } else { hp };
        }
        self.take_red_key();
    }

    /**
     * A prince's health has fallen into `phase` (in his court or in the ring): his roar and its
     * banner, a green cross for a breather, and as the second begins his golems climbing out of
     * the gold.
     */
    pub(super) fn prince_phase(&mut self, i: usize, phase: u8) {
        let Some(du) = self.enemies.get(i).and_then(|e| e.duel.as_deref()) else { return };
        if du.arch != Archetype::Prince {
            return;
        }
        let (id, pos, summoned) = (du.id, du.pos, du.lore.summoned);
        if let Some(c) = self.court.as_mut() {
            c.phase = phase;
        }
        if let Some(b) = self.brawl.as_mut() {
            b.threat.release(id);
        }
        if let Some(d) = self.enemies[i].duel.as_deref_mut() {
            d.token = false;
            d.rest = 75;
            d.lore.summoned |= phase >= 2;
        }
        self.audio.play(Sfx::BossRoar);
        self.audio.set_tempo_step(phase as u32 - 1);
        self.view.shake(0.45);
        self.feel_phase();
        self.view.sparkle(&self.level.grid, pos.x, pos.y, 1.4, 0xf2c53d, 40);
        let sub = match phase {
            2 => "HE RAISES HIS GOLD AGAINST YOU   STRIKE HIM TO BREAK HIS GRASP",
            _ => "COINS FALL FROM ABOVE   STEP OFF THEM, THEN STRIKE",
        };
        self.juice.banner = Some(Banner {
            text: format!("{} - {}", self.boss_name(), phase_name(phase)),
            sub: sub.to_string(),
            color: if phase == 2 { 0xd8a040 } else { 0xff8a3a },
            life: 2.6,
            max: 2.6,
        });
        if phase >= 2 && !summoned {
            self.raise_golems(id, pos);
        }
        self.drop_breather(pos);
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
        let (weight_hp, weight_damage) = if self.in_court() { weight(tier) } else { (1.0, 1.0) };
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

    /// A green cross on open floor a few steps from the hero and away from the prince: a breather
    /// between phases.
    fn drop_breather(&mut self, prince: V) {
        let hp = self.hero.pos();
        let g = &self.level.grid;
        let ring = self.in_ring().then(crate::logic::gym::ring);
        let best = g
            .cells()
            .filter(|&(x, y, t)| t == Tile::Open && g.is_walkable(x, y, false) && ring.is_none_or(|r| r.contains(P::new(x, y))))
            .map(|(x, y, _)| P::new(x, y))
            .filter(|p| p.v() != hp)
            .max_by(|a, b| {
                let score = |p: &P| {
                    let from_hero = dist_sq(p.v(), hp).sqrt();
                    dist_sq(p.v(), prince).sqrt().min(6.0) - (from_hero - 3.5).abs() * 1.5
                };
                score(a).total_cmp(&score(b)).then(b.y.cmp(&a.y)).then(b.x.cmp(&a.x))
            });
        if let Some(p) = best {
            self.level.grid.set(p.x, p.y, Tile::Charge);
            self.view.add_charge(&self.level.grid, p.x, p.y);
            self.view.sparkle(&self.level.grid, p.x as f64, p.y as f64, 0.4, 0x5cff6a, 20);
            self.floater(p.x as f64, p.y as f64, "HEALTH", 1.6, Some(0x5cff6a));
        }
    }

    /// The prince falls in his court: everything a boss's death brings, and his key.
    pub(super) fn prince_slain(&mut self, at: V) {
        if let Some(c) = self.court.as_mut() {
            c.slain = true;
            c.trail = 0.0;
        }
        if let Some(b) = self.brawl.as_mut() {
            b.traps.clear();
            b.bolts.clear();
            b.held = None;
        }
        self.floater(at.x, at.y - 1.8, "THE HOARD SPILLS", 2.0, Some(0xf2c53d));
        self.boss_slain(at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{save, Screen};
    use crate::logic::combat::prince::{phase_of, GRASP_HOLD};
    use crate::logic::combat::{MoveId, State};
    use crate::logic::sins::Sin;

    fn court() -> Game {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test-court"));
        let mut g = Game::new();
        g.map_jump(save::Stage::Court(Sin::Greed));
        g.enter(Screen::Playing);
        g.hero.free_move = true;
        g
    }

    fn mammon(g: &mut Game) -> &mut Duelist {
        let id = g.court.as_ref().unwrap().prince;
        g.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == id).unwrap()
    }

    fn run(g: &mut Game, secs: f64) {
        for _ in 0..(secs / crate::app::STEP) as usize {
            g.update(crate::app::STEP);
        }
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
        // A blow that leaves him just above two thirds: still the Banker.
        mammon(&mut g).hp = max * 0.67 + 1.0;
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
        assert!(warded > 0.0 && open > warded * 2.5, "warded {warded}, open {open}");
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
    fn a_fall_brings_mammon_back_whole_and_continue_comes_back_to_his_court() {
        let mut g = court();
        let max = mammon(&mut g).max_hp;
        mammon(&mut g).hp = max * 0.5;
        mammon(&mut g).breed.phase = 2;
        g.reset_round();
        let d = mammon(&mut g);
        assert_eq!((d.hp, d.breed.phase), (max, 1));
        // Saved at the court's door, Continue loads it again.
        g.checkpoint(save::Stage::Court(Sin::Greed));
        let run = g.saved_run.clone().unwrap();
        let mut h = Game::new();
        h.saved_run = Some(run);
        h.continue_game();
        assert!(h.in_court() && h.prince().is_some());
        assert_eq!(h.level.data.name, "The Counting House");
    }
}
