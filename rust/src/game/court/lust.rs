//! Lust's court in play: the Rose Bower (`logic::court::lust` lays it out) and Asmodeus, its prince
//! (`logic::combat::prince::lust`).
//!
//! - **The censers** (Tobit): a blow on a cold censer lights it, and it smokes for ten seconds.
//!   While any smokes, his allure is gone (his hide turns nothing) and he will not stand within
//!   three tiles of a smoking one: he backs away from it. Each censer lights twice in a fight (once,
//!   if his general's decree was done). Its embers burn in the bowl, on the scene as fire no one can
//!   stand in (a `Hazard` of no width on the censer's own tile), which the court's view reads to draw
//!   the embers and the smoke.
//! - **The blown kiss**: a pink heart, his one bolt. No shield stops it and no parry turns it: it
//!   is leapt, or struck out of the air by the hero's blade. If it strikes him he is smitten for a
//!   moment: his guard will not rise (he still walks, leaps and swings). It is met here, the frame
//!   before it would reach him, as a blow from the ground is (`general_hazard_blow`): the landing
//!   the threat manager planned for it.
//! - **The thorn ring**: what his ring of thorns lays about the hero is laid round him instead.
//! - **The elf-maids** step out from behind the hedges as his second phase begins (once; at once if
//!   the Bridesmaid's vows were spoken; never if she fell). They fall when he does.

use crate::audio::Sfx;
use crate::game::{Game, Screen};
use crate::logic::combat::ails::Hazard;
use crate::logic::combat::archetype::Power;
use crate::logic::combat::duelist::{BoltLook, Flow, GridTerrain, Lore, Terrain, HERO_RADIUS};
use crate::logic::combat::frames::{BOLT_RADIUS, BOLT_SPEED};
use crate::logic::combat::fsm::Defense;
use crate::logic::combat::prince::lust::{
    LIGHTS, LIGHTS_DECREED, MAIDS, MAID_DAMAGE, MAID_HEALTH, MAID_RISE, RING, SHUN, SHUN_PACE, SMITTEN, SMOKE,
};
use crate::logic::combat::prince::Trap;
use crate::logic::combat::{math, MoveId, Phase, State};
use crate::logic::court::lust::{beside_hedge, CENSERS, HEDGES};
use crate::logic::court::weight;
use crate::logic::enemy::EnemyKind;
use crate::logic::sins::Sin;
use crate::logic::types::{dist_sq, P, V};

/// A censer's body, for a blow's reach.
const CENSER_RADIUS: f64 = 0.4;
/// Frames the embers in a smoking censer's bowl are kept on the scene for (renewed every tick).
const EMBERS: u16 = 4;
/// Pink, the colour of his kisses and his court's floaters.
const ROSE: u32 = 0xff6aa8;

/// One of the court's censers: the frame its smoke runs out (none smoking: past), and how many
/// times more a blow will light it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Censer {
    pub until: u32,
    pub lights: u8,
}

/// What the Rose Bower keeps of its fight: its censers, the frame until which the hero is smitten,
/// the last thorn ring laid round the prince, the swing that last struck a censer (one swing
/// strikes one once), whether the smoke was up last tick, and whether the hero has been told.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bower {
    pub censers: [Censer; 4],
    pub smitten: u32,
    ring: u32,
    swing: Option<u32>,
    smoking: bool,
    told: bool,
}

impl Bower {
    /// A fresh fight's: every censer cold, each to light `lights` times.
    fn fresh(lights: u8) -> Self {
        Bower { censers: [Censer { until: 0, lights }; 4], ..Bower::default() }
    }

    /// Is any censer smoking on `frame`?
    fn smokes(&self, frame: u32) -> bool {
        self.censers.iter().any(|c| c.until > frame)
    }
}

impl Game {
    /// The brawl's frame, if it is fought.
    fn bower_frame(&self) -> u32 {
        self.brawl.as_ref().map(|b| b.threat.frame).unwrap_or(0)
    }

    fn bower(&mut self) -> Option<&mut Bower> {
        self.court.as_mut().filter(|c| c.sin == Sin::Lust).map(|c| &mut c.bower)
    }

    /// The Rose Bower's censers: where each stands, whether it smokes, and how many times more it
    /// will light (empty outside his court).
    pub fn censers(&self) -> Vec<(V, bool, u8)> {
        let frame = self.bower_frame();
        let Some(b) = self.court.as_ref().filter(|c| c.sin == Sin::Lust).map(|c| &c.bower) else { return Vec::new() };
        CENSERS.iter().zip(b.censers.iter()).map(|(&(x, y), c)| (P::new(x, y).v(), c.until > frame, c.lights)).collect()
    }

    /// The hero is smitten by a kiss: his guard will not rise.
    pub fn smitten(&self) -> bool {
        let frame = self.bower_frame();
        self.court.as_ref().is_some_and(|c| c.sin == Sin::Lust && c.bower.smitten > frame)
    }

    /// As Asmodeus is set before the bridal arch: the censers cold, each to light twice (once, if
    /// the Bridesmaid's vows were spoken).
    pub(super) fn lust_enter(&mut self, _prince: u32) {
        let lights = if self.prince_terms(Sin::Lust).adds_at_start { LIGHTS_DECREED } else { LIGHTS };
        if let Some(b) = self.bower() {
            *b = Bower::fresh(lights);
        }
    }

    /// Each tick of the fight in the Rose Bower: the censers struck and smoking, his allure with
    /// them, the kisses met, the thorn ring laid round him, the smitten guard.
    pub(super) fn lust_tick(&mut self, dt: f64) {
        let Some(id) = self.court.as_ref().filter(|c| !c.slain).map(|c| c.prince) else { return };
        if self.screen != Screen::Playing || self.prince().is_none() {
            return;
        }
        let frame = self.bower_frame();
        self.tell_of_the_censers();
        self.strike_censers(frame);
        self.censers_smoke(id, frame, dt);
        self.meet_kisses(id, frame);
        self.lay_thorn_ring(id);
        let smitten = self.bower().map(|b| b.smitten > frame).unwrap_or(false);
        if smitten {
            // Smitten: his guard will not rise (nor parry), though he walks, leaps and swings.
            if let Some(b) = self.brawl.as_mut() {
                b.hero.guard_held = false;
            }
            if frame.is_multiple_of(15) {
                let hp = self.hero.pos();
                self.view.sparkle(&self.level.grid, hp.x, hp.y, 1.6, ROSE, 3);
            }
        }
    }

    /// The first time the fight is on: where the censers are, and what they are for.
    fn tell_of_the_censers(&mut self) {
        let Some(b) = self.bower().filter(|b| !b.told) else { return };
        b.told = true;
        let hp = self.hero.pos();
        let near = CENSERS.iter().map(|&(x, y)| P::new(x, y).v()).min_by(|a, b| dist_sq(*a, hp).total_cmp(&dist_sq(*b, hp)));
        if let Some(c) = near {
            self.floater(c.x, c.y - 1.4, "STRIKE A CENSER", 3.0, Some(0xf0d8a0));
        }
    }

    /// A blow of his on a cold censer lights it (one censer for one swing).
    fn strike_censers(&mut self, frame: u32) {
        let Some(b) = self.brawl.as_ref() else { return };
        let Some((mv, Phase::Active, at)) = b.hero.attack() else { return };
        let d = b.hero.data(mv);
        if d.projectile {
            return;
        }
        let swing = frame.saturating_sub(at as u32);
        let (hp, aim) = (self.hero.pos(), b.aim);
        let Some(k) = CENSERS.iter().position(|&(x, y)| math::in_cone(hp, aim, P::new(x, y).v(), CENSER_RADIUS, d.reach, d.half_angle))
        else {
            return;
        };
        let Some(bower) = self.bower().filter(|b| b.swing != Some(swing)) else { return };
        bower.swing = Some(swing);
        let c = &mut bower.censers[k];
        let at = P::new(CENSERS[k].0, CENSERS[k].1).v();
        if c.until > frame {
            return;
        }
        if c.lights == 0 {
            self.floater(at.x, at.y - 1.3, "BURNT OUT", 1.0, Some(0x9a8a80));
            self.audio.play(Sfx::Clang);
            return;
        }
        c.lights -= 1;
        c.until = frame + SMOKE as u32;
        let left = c.lights;
        self.floater(at.x, at.y - 1.5, "THE CENSER SMOKES", 1.6, Some(0xf0d8a0));
        if left == 0 {
            self.floater(at.x, at.y - 1.0, "ITS LAST FIRE", 1.4, Some(0xc8b8a8));
        }
        self.audio.play(Sfx::RaphaelBeam);
        if let Some(b) = self.brawl.as_mut() {
            b.hazards.push(Hazard { pos: at, radius: 0.0, life: EMBERS });
        }
        self.view.sparkle(&self.level.grid, at.x, at.y, 1.2, 0xffd890, 26);
        self.view.dust(&self.level.grid, at.x, at.y, 0xd8d0e0, 14, 1.2);
    }

    /// The smoke: embers in every smoking censer's bowl, his allure gone while any smokes (back as the
    /// last goes cold), and he backs away from each smoking one.
    fn censers_smoke(&mut self, id: u32, frame: u32, dt: f64) {
        let Some(b) = self.bower() else { return };
        let smokes = b.smokes(frame);
        let was = std::mem::replace(&mut b.smoking, smokes);
        let lit: Vec<V> =
            CENSERS.iter().zip(b.censers.iter()).filter(|(_, c)| c.until > frame).map(|(&(x, y), _)| P::new(x, y).v()).collect();
        if let Some(br) = self.brawl.as_mut() {
            // The embers in the bowls of those that smoke (a fire no one can stand in: no width).
            br.hazards.retain(|h| h.radius > 0.0);
            br.hazards.extend(lit.iter().map(|&p| Hazard { pos: p, radius: 0.0, life: EMBERS }));
        }
        let Some(d) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == id && d.alive()) else { return };
        d.breed.power = if smokes { Power::None } else { Power::Allure };
        let at = d.pos;
        if smokes && !matches!(d.f.state, State::Attack { .. }) {
            // He will not stand near the smoke: he backs out of it.
            let flow = Flow::default();
            let terrain = GridTerrain { grid: &self.level.grid, flow: &flow };
            for c in &lit {
                let away = math::sub(d.pos, *c);
                let gap = SHUN - math::len(away);
                if gap > 0.0 {
                    let step = math::scale(math::norm(away), gap.min(SHUN_PACE * dt));
                    d.pos = terrain.slide(d.pos, step, d.radius());
                }
            }
        }
        if smokes && !was {
            self.floater(at.x, at.y - 3.4, "THE SMOKE: HIS ALLURE FAILS", 1.8, Some(0xf0d8a0));
        } else if was && !smokes {
            self.floater(at.x, at.y - 3.4, "THE SMOKE THINS", 1.4, Some(ROSE));
        }
    }

    /// His kisses as they come to the hero: struck out of the air by his blade, passed under in a
    /// leap, or on him: no shield stops a kiss, and a kiss that strikes him leaves him smitten.
    fn meet_kisses(&mut self, id: u32, frame: u32) {
        let hp = self.hero.pos();
        let airborne = self.hero.airborne();
        let Some(b) = self.brawl.as_mut() else { return };
        let blade = match b.hero.attack() {
            Some((mv, Phase::Active, _)) => Some(b.hero.data(mv)).filter(|d| !d.projectile),
            _ => None,
        };
        let aim = b.aim;
        let mut struck = Vec::new();
        let mut landed = None;
        for k in b.bolts.iter_mut().filter(|k| k.owner == id && k.look == BoltLook::Kiss && !k.reflected && !k.touched && k.life > 0) {
            if blade.is_some_and(|d| math::in_cone(hp, aim, k.pos, BOLT_RADIUS, d.reach + 0.2, d.half_angle)) {
                k.life = 0;
                struck.push(k.pos);
                continue;
            }
            // (Met the frame before it would touch him: it is paced to land on its planned frame.)
            let near = BOLT_RADIUS + HERO_RADIUS + math::len(k.vel).max(BOLT_SPEED * 1.6) / 60.0;
            if !airborne && landed.is_none() && math::len(math::sub(k.pos, hp)) <= near {
                k.life = 0;
                k.touched = true;
                landed = Some((math::sub(k.pos, math::scale(k.vel, 1.0 / 60.0)), k.damage));
            }
        }
        for p in struck {
            self.floater(p.x, p.y - 0.9, "STRUCK DOWN", 0.9, Some(ROSE));
            self.view.sparkle(&self.level.grid, p.x, p.y, 1.0, ROSE, 16);
            self.audio.play(Sfx::Clang);
        }
        let Some((from, damage)) = landed else { return };
        if self.general_hazard_blow(from, damage, id) == Defense::Open && self.hero.alive {
            if let Some(b) = self.bower() {
                b.smitten = frame + SMITTEN as u32;
            }
            let hp = self.hero.pos();
            self.floater(hp.x, hp.y - 1.4, "SMITTEN: NO GUARD", 1.4, Some(ROSE));
            self.view.sparkle(&self.level.grid, hp.x, hp.y, 1.4, ROSE, 24);
            self.audio.play(Sfx::Bells);
        }
    }

    /// His thorn ring: what the throw laid about the hero is laid round him instead, as far as the
    /// floor goes (the thorns burst together, on the frame the threat manager planned).
    fn lay_thorn_ring(&mut self, id: u32) {
        let Some(d) = self.prince() else { return };
        if !matches!(d.f.attack(), Some((MoveId::ThornRing, Phase::Active | Phase::Recovery, _))) {
            return;
        }
        let at = d.pos;
        let count = MoveId::ThornRing.data().traps().map(|t| t.0).unwrap_or(8);
        let ring = self.bower().map(|b| b.ring).unwrap_or(0);
        let Some(b) = self.brawl.as_ref() else { return };
        let Some(volley) = b.traps.iter().filter(|t| t.owner == id && t.flight > 0).map(|t| t.volley).max().filter(|&v| v != ring) else {
            return;
        };
        let Some(model) = b.traps.iter().find(|t| t.volley == volley).copied() else { return };
        let turn = volley as f64 * 0.77;
        let spots: Vec<V> = (0..count)
            .map(|k| math::add(at, math::scale(math::from_angle(turn + k as f64 * std::f64::consts::TAU / count as f64), RING)))
            .filter(|p| self.level.grid.is_walkable(p.x.round() as i32, p.y.round() as i32, false))
            .collect();
        if let Some(b) = self.brawl.as_mut() {
            b.traps.retain(|t| t.volley != volley);
            b.traps.extend(spots.into_iter().map(|p| Trap { pos: p, from: at, ..model }));
        }
        if let Some(b) = self.bower() {
            b.ring = volley;
        }
    }

    /// As his health falls into `phase`: as the second begins, the elf-maids step out from behind
    /// the hedges (unless they already have, or the Bridesmaid's fall has lost them).
    pub(super) fn lust_phase(&mut self, prince: u32, at: V, phase: u8) {
        if phase >= 2 && self.adds_unsummoned(prince) {
            self.raise_maids(prince, at);
        }
    }

    /// After a fall: the censers cold and whole again, no kiss on him, his allure back (his
    /// elf-maids are already gone).
    pub(super) fn lust_reset(&mut self) {
        self.lust_enter(0);
        if let Some(d) = self
            .court
            .as_ref()
            .map(|c| c.prince)
            .and_then(|id| self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == id))
        {
            d.breed.power = Power::Allure;
        }
    }

    /// As he falls: the smoke drives him off, as Raphael's drove him to Egypt.
    pub(super) fn lust_slain(&mut self, at: V) {
        if let Some(b) = self.bower() {
            b.smitten = 0;
        }
        self.floater(at.x, at.y - 2.2, "THE SMOKE DRIVES HIM OFF", 2.0, Some(0xf0d8a0));
        self.view.sparkle(&self.level.grid, at.x, at.y, 2.4, ROSE, 40);
    }

    /// The phase banner's line under his name, and its colour.
    pub(super) fn lust_banner(phase: u8) -> (&'static str, u32) {
        match phase {
            2 => ("THE BULL CHARGES, THE RAM BUTTS   LIGHT A CENSER, STRIKE THE MAIDS' BACKS", 0xff7a9a),
            _ => ("THORNS RING HIM   STEP OUT, LIGHT A CENSER, THEN STRIKE", 0xff5a5a),
        }
    }

    /// His adds at once (the Bridesmaid's vows spoken): the elf-maids.
    pub(super) fn lust_raise(&mut self, prince: u32, at: V) {
        self.raise_maids(prince, at);
    }

    /// Two elf-maids step out from behind the hedges nearest the hero (in the ring, beside the
    /// prince); they fall when he does.
    fn raise_maids(&mut self, id: u32, at: V) {
        let hp = self.hero.pos();
        let spots: Vec<V> = if self.in_court() {
            let mut hedges = HEDGES.to_vec();
            hedges.sort_by(|a, b| dist_sq(beside_hedge(*a).v(), hp).total_cmp(&dist_sq(beside_hedge(*b).v(), hp)));
            hedges
                .iter()
                .cycle()
                .take(MAIDS as usize)
                .enumerate()
                .map(|(k, h)| math::add(beside_hedge(*h).v(), V::new(0.0, k as f64 / 2.0)))
                .collect()
        } else {
            let across = math::norm(V::new(-(hp.y - at.y), hp.x - at.x));
            [-1.0, 1.0].iter().map(|s| math::add(at, math::scale(across, 2.2 * s))).collect()
        };
        let tier = self.realm_tier();
        let (weight_hp, weight_damage) = if self.in_court() { weight(tier) } else { (1.0, 1.0) };
        for p in spots {
            let facing = math::norm(math::sub(hp, p));
            let lore = Lore { master: Some(id), rise: MAID_RISE, ..Lore::default() };
            if let Some(mid) = self.spawn_duelist(EnemyKind::Ellepige, p, facing, lore) {
                if let Some(m) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == mid) {
                    m.max_hp *= weight_hp * MAID_HEALTH;
                    m.hp = m.max_hp;
                    m.breed.damage *= weight_damage * MAID_DAMAGE;
                }
            }
            self.view.dust(&self.level.grid, p.x, p.y, 0x5a7a3a, 16, 1.4);
            self.view.sparkle(&self.level.grid, p.x, p.y, 1.2, 0xf0e0ea, 24);
        }
        self.floater(hp.x, hp.y - 1.5, "THE ELF-MAIDS STEP OUT: STRIKE THEIR BACKS", 1.8, Some(0xf0e0ea));
        self.audio.play(Sfx::Whisper);
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{self, run};
    use super::super::{COURT_CADENCE, COURT_CAPACITY};
    use super::*;
    use crate::game::save;
    use crate::input::{Action, PadState};
    use crate::logic::combat::archetype::Archetype;
    use crate::logic::combat::prince::phase_of;
    use crate::logic::combat::Duelist;
    use crate::logic::types::Tile;

    fn court() -> Game {
        testing::court(Sin::Lust)
    }

    fn asmodeus(g: &mut Game) -> &mut Duelist {
        testing::prince(g)
    }

    /// Asmodeus keeps still and strikes nothing of himself.
    fn still(g: &mut Game) {
        let d = asmodeus(g);
        d.rest = u16::MAX;
        d.token = false;
    }

    /// The hero beside censer `k` (south of it), facing it, swings once.
    fn strike_censer(g: &mut Game, k: usize) {
        let (x, y) = CENSERS[k];
        g.hero.place(P::new(x, y + 1));
        g.hero.heading = math::angle(V::new(0.0, -1.0));
        g.hero.stamina = crate::logic::progression::STAMINA_MAX;
        g.brawl_input(true, false, false);
        run(g, 0.45);
    }

    /// Asmodeus `dist` tiles north of the hero, facing him, begins `mv` (in `phase`).
    fn begin(g: &mut Game, mv: MoveId, dist: f64, phase: u8) {
        let hp = g.hero.pos();
        let d = asmodeus(g);
        d.pos = V::new(hp.x, hp.y - dist);
        d.facing = V::new(0.0, 1.0);
        d.rest = u16::MAX;
        d.breed.phase = phase;
        d.token = true;
        assert!(d.begin(mv, hp), "{mv:?}");
    }

    /// One tick of play with `pad` held.
    fn tick(g: &mut Game, pad: &PadState) {
        g.input.set_pad(Some(pad.clone()));
        g.update(crate::app::STEP);
    }

    #[test]
    fn the_court_of_lust_is_the_rose_bower_and_asmodeus_fights_there_by_frame_data() {
        let g = court();
        assert_eq!(g.level.data.name, "The Rose Bower");
        assert_eq!(g.realm, Some(Sin::Lust));
        assert!(g.boss.is_none(), "no Cyclops");
        assert!(g.arena() && g.brawling(), "a boss's arena, fought by frame data");
        let b = g.brawl.as_ref().unwrap();
        assert_eq!((b.threat.capacity, b.threat.cadence), (COURT_CAPACITY, COURT_CADENCE));
        let d = g.prince().expect("Asmodeus stands");
        assert_eq!((d.breed.kind, d.arch, d.breed.phase, d.breed.power), (EnemyKind::Asmodeus, Archetype::Prince, 1, Power::Allure));
        assert_eq!(g.enemies.len(), 1, "he waits alone");
        let bar = g.prince_bar().unwrap();
        assert_eq!((bar.name.as_str(), bar.title), ("ASMODEUS", "THE SUITOR"));
        assert_eq!(g.keys_total, 1, "one key opens the gate");
        // Four cold censers, each to light twice.
        let c = g.censers();
        assert_eq!(c.len(), 4);
        assert!(c.iter().all(|c| !c.1 && c.2 == LIGHTS));
        assert!(!g.smitten());
    }

    #[test]
    fn his_phases_change_at_the_thresholds_and_the_elf_maids_step_out_once() {
        let mut g = court();
        let max = asmodeus(&mut g).max_hp;
        still(&mut g);
        let from = asmodeus(&mut g).pos;
        let maids = |g: &Game| g.enemies.iter().filter(|e| e.kind() == EnemyKind::Ellepige).count();
        asmodeus(&mut g).hp = max * 0.67 + 1.0;
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        let hp = asmodeus(&mut g).hp;
        assert_eq!(asmodeus(&mut g).breed.phase, phase_of(hp, max));
        assert_eq!(maids(&g), 0);
        // Under two thirds: the Three Heads, two elf-maids from the hedges, a green cross.
        asmodeus(&mut g).hp = max * 0.6;
        still(&mut g);
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        assert_eq!(asmodeus(&mut g).breed.phase, 2);
        assert_eq!(maids(&g), MAIDS as usize, "two elf-maids");
        assert!(g.level.grid.count(Tile::Charge) >= 1, "a breather");
        assert!(g.juice.banner.as_ref().is_some_and(|b| b.text.contains("THE THREE HEADS")));
        assert_eq!(asmodeus(&mut g).breed.moves()[0], MoveId::BullCharge);
        // They serve him lighter than in their garden, and fall when he does.
        let maid = g.enemies.iter().filter_map(|e| e.duel.as_deref()).find(|d| d.breed.kind == EnemyKind::Ellepige).unwrap();
        let realm = crate::logic::combat::Breed::of(EnemyKind::Ellepige);
        assert!(maid.max_hp < realm.stats().hp * weight(g.realm_tier()).0 && maid.lore.master.is_some());
        // Under a third: the Bridegroom of Ashes. The maids are not raised again.
        asmodeus(&mut g).hp = max * 0.3;
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        assert_eq!(asmodeus(&mut g).breed.phase, 3);
        assert_eq!(maids(&g), MAIDS as usize, "raised once");
        assert_eq!(asmodeus(&mut g).breed.moves()[0], MoveId::ThornRing);
        assert!(g.prince_bar().unwrap().title == "THE BRIDEGROOM OF ASHES");
    }

    #[test]
    fn his_allure_turns_a_blow_but_not_in_his_recovery_nor_while_a_censer_smokes() {
        let mut g = court();
        still(&mut g);
        let from = asmodeus(&mut g).pos;
        let before = asmodeus(&mut g).hp;
        g.call_duel_hit(0, from);
        let warded = before - asmodeus(&mut g).hp;
        let d = asmodeus(&mut g);
        d.f.state = State::Attack { mv: MoveId::LanceThrust, frame: 60 };
        let before = d.hp;
        g.call_duel_hit(0, from);
        let open = before - asmodeus(&mut g).hp;
        assert!(warded > 0.0 && open > warded * 2.5, "warded {warded}, open {open}");
        // A censer lit: the smoke takes his allure away, and a blow lands whole outside his recoveries.
        asmodeus(&mut g).f.state = State::Neutral;
        strike_censer(&mut g, 0);
        assert!(g.censers()[0].1, "it smokes");
        assert_eq!(asmodeus(&mut g).breed.power, Power::None);
        let d = asmodeus(&mut g);
        d.f.state = State::Neutral;
        assert!(!d.exposed());
        let before = d.hp;
        g.call_duel_hit(0, from);
        let smoked = before - asmodeus(&mut g).hp;
        assert!(smoked > warded * 2.5, "warded {warded}, in the smoke {smoked}");
        // When the smoke is gone, his allure is back.
        run(&mut g, SMOKE as f64 / 60.0 + 0.2);
        assert!(!g.censers()[0].1, "cold again");
        assert_eq!(asmodeus(&mut g).breed.power, Power::Allure);
    }

    #[test]
    fn a_blow_lights_a_censer_for_ten_seconds_twice_and_he_backs_out_of_its_smoke() {
        let mut g = court();
        still(&mut g);
        strike_censer(&mut g, 1);
        let c = g.censers()[1];
        assert!(c.1 && c.2 == LIGHTS - 1, "lit once by one swing: {c:?}");
        assert!(g.brawl.as_ref().unwrap().hazards.iter().any(|h| h.radius == 0.0), "its embers show");
        // He will not stand within three tiles of it: set beside it, he backs away.
        let at = g.censers()[1].0;
        asmodeus(&mut g).pos = math::add(at, V::new(1.6, 0.6));
        still(&mut g);
        run(&mut g, 1.2);
        let gap = math::len(math::sub(asmodeus(&mut g).pos, at));
        assert!(gap >= SHUN - 0.15, "{gap} from the smoke");
        // Ten seconds, then cold; a blow lights it once more, and then it is burnt out.
        run(&mut g, SMOKE as f64 / 60.0 - 2.1);
        assert!(g.censers()[1].1, "still smoking just before ten seconds");
        run(&mut g, 1.0);
        assert!(!g.censers()[1].1, "cold after ten");
        strike_censer(&mut g, 1);
        assert!(g.censers()[1].1 && g.censers()[1].2 == 0);
        run(&mut g, SMOKE as f64 / 60.0 + 0.1);
        strike_censer(&mut g, 1);
        assert!(!g.censers()[1].1, "burnt out: it lights no more");
        // A swing that meets no censer lights none.
        g.hero.place(P::new(16, 26));
        g.brawl_input(true, false, false);
        run(&mut g, 0.45);
        assert!(g.censers().iter().all(|c| !c.1));
    }

    /// Asmodeus blows a kiss at the hero five tiles south of him; `pad` is held as it comes; at
    /// `trigger` tiles off, `then` is pressed once. Returns the health he lost.
    fn a_kiss(g: &mut Game, pad: PadState, trigger: f64, then: Option<Action>) -> i32 {
        still(g);
        begin(g, MoveId::BlownKiss, 5.0, 1);
        let start = g.hero.hp;
        let mut pressed = false;
        for _ in 0..240 {
            let hp = g.hero.pos();
            let near =
                g.brawl.as_ref().unwrap().bolts.iter().any(|b| b.look == BoltLook::Kiss && math::len(math::sub(b.pos, hp)) <= trigger);
            let mut p = pad.clone();
            if near && !pressed {
                pressed = true;
                if let Some(a) = then {
                    p.actions.insert(a);
                }
            }
            tick(g, &p);
            if pressed && g.brawl.as_ref().unwrap().bolts.iter().all(|b| b.look != BoltLook::Kiss) {
                break;
            }
        }
        assert!(pressed, "the kiss came");
        run(g, 0.1);
        start - g.hero.hp
    }

    #[test]
    fn a_blown_kiss_smites_him_through_his_shield_but_is_leapt_or_struck_out_of_the_air() {
        // Guarding does nothing against it: it strikes him, and he is smitten.
        let mut g = court();
        let guard = PadState { actions: [Action::Block].into_iter().collect(), ..Default::default() };
        let lost = a_kiss(&mut g, guard.clone(), 3.0, None);
        assert!(lost > 0 && g.smitten(), "lost {lost}");
        // Smitten, his guard will not rise, though he can walk.
        for _ in 0..20 {
            tick(&mut g, &guard);
        }
        assert!(!matches!(g.brawl.as_ref().unwrap().hero.state, State::Guard { .. }), "no guard while smitten");
        assert!(!g.hero.blocking);
        let at = g.hero.pos();
        let walk = PadState { left: (1.0, 0.0), ..Default::default() };
        for _ in 0..30 {
            tick(&mut g, &walk);
        }
        assert!(math::len(math::sub(g.hero.pos(), at)) > 0.5, "he walks at his pace");
        // It passes: his guard comes back.
        run(&mut g, SMITTEN as f64 / 60.0);
        assert!(!g.smitten());
        for _ in 0..20 {
            tick(&mut g, &guard);
        }
        assert!(matches!(g.brawl.as_ref().unwrap().hero.state, State::Guard { .. }), "his guard rises again");
        // Leapt, it passes under him.
        let mut g = court();
        let lost = a_kiss(&mut g, PadState::default(), 1.6, Some(Action::Jump));
        assert!(lost == 0 && !g.smitten(), "leapt: lost {lost}");
        // Struck out of the air by his blade.
        let mut g = court();
        g.hero.heading = math::angle(V::new(0.0, -1.0));
        let lost = a_kiss(&mut g, PadState::default(), 2.0, Some(Action::Attack));
        assert!(lost == 0 && !g.smitten(), "struck down: lost {lost}");
    }

    #[test]
    fn his_thorns_burst_late_enough_to_step_out_of_and_his_ring_is_round_him() {
        // A row of thorns toward him: stepping aside as he sees them land, he is not hurt.
        let mut g = court();
        begin(&mut g, MoveId::ThornLine, 4.0, 1);
        let start = g.hero.hp;
        let mut landed = None;
        for i in 0..600 {
            let traps = &g.brawl.as_ref().unwrap().traps;
            if landed.is_none() && !traps.is_empty() && traps.iter().all(|t| t.armed()) {
                landed = Some(i);
            }
            let pad = match landed {
                // (A reaction's frames, two ticks each, then off to the side.)
                Some(at) if i >= at + 2 * crate::logic::combat::frames::REACTION as usize => {
                    PadState { left: (1.0, 0.0), ..Default::default() }
                }
                _ => PadState::default(),
            };
            tick(&mut g, &pad);
            if landed.is_some() && g.brawl.as_ref().unwrap().traps.iter().all(|t| t.burst.is_some()) {
                break;
            }
        }
        assert!(landed.is_some(), "the thorns lay in a row");
        run(&mut g, 0.2);
        assert_eq!(g.hero.hp, start, "stepped out of the row");
        // Standing on them, he is hurt.
        let mut g = court();
        begin(&mut g, MoveId::ThornLine, 4.0, 1);
        let start = g.hero.hp;
        run(&mut g, 2.0);
        assert!(g.hero.hp < start, "the thorns burst under him");
        // The ring: every thorn round him; close by, it catches the hero.
        let mut g = court();
        begin(&mut g, MoveId::ThornRing, 1.9, 3);
        let at = asmodeus(&mut g).pos;
        let start = g.hero.hp;
        run(&mut g, 1.0);
        let traps = g.brawl.as_ref().unwrap().traps.clone();
        assert!(traps.len() >= 6, "{} thorns", traps.len());
        assert!(traps.iter().all(|t| (math::len(math::sub(t.pos, at)) - RING).abs() < 0.01), "round him");
        run(&mut g, 1.0);
        assert!(g.hero.hp < start, "standing by him, the ring catches him");
    }

    #[test]
    fn asmodeus_slain_drops_the_key_of_lust_and_the_way_on_is_open() {
        let mut g = court();
        let from = asmodeus(&mut g).pos;
        asmodeus(&mut g).hp = 1.0;
        asmodeus(&mut g).f.state = State::Attack { mv: MoveId::LanceThrust, frame: 60 };
        asmodeus(&mut g).breed.phase = 3;
        g.call_duel_hit(0, from);
        assert!(g.prince().is_none(), "he has fallen");
        assert!(g.court.as_ref().unwrap().slain);
        assert!(g.red_key.is_some(), "the Key of Lust lies on the floor");
        assert!(g.sins_cleared & Sin::Lust.bit() != 0, "Lust's bit is set");
        assert_eq!(g.key_name(), "THE KEY OF LUST");
        assert!(g.saved_run.as_ref().is_some_and(|r| r.stage == save::Stage::SinShop(Sin::Lust)));
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
    fn a_fall_brings_asmodeus_back_whole_with_the_censers_cold_and_continue_comes_back() {
        let mut g = court();
        still(&mut g);
        strike_censer(&mut g, 2);
        let max = asmodeus(&mut g).max_hp;
        asmodeus(&mut g).hp = max * 0.5;
        asmodeus(&mut g).breed.phase = 2;
        g.reset_round();
        let d = asmodeus(&mut g);
        assert_eq!((d.hp, d.breed.phase, d.breed.power), (max, 1, Power::Allure));
        assert!(g.censers().iter().all(|c| !c.1 && c.2 == LIGHTS), "the censers cold and whole");
        g.checkpoint(save::Stage::Court(Sin::Lust));
        let run = g.saved_run.clone().unwrap();
        let mut h = Game::new();
        h.saved_run = Some(run);
        h.continue_game();
        assert!(h.in_court() && h.prince().is_some());
        assert_eq!(h.level.data.name, "The Rose Bower");
    }

    /// The court of Lust with the Bridesmaid's realm come to `record`.
    fn court_after(record: crate::logic::war::general::Record) -> Game {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test-court"));
        let mut g = Game::new();
        let mut all = g.general_records();
        all.set(Sin::Lust, record);
        g.warband.save.generals = all.encode();
        g.map_jump(save::Stage::Court(Sin::Lust));
        g.enter(Screen::Playing);
        g.hero.free_move = true;
        g
    }

    #[test]
    fn the_bridesmaid_slain_he_has_no_maids_and_her_vows_spoken_they_come_at_once_and_the_censers_light_once() {
        use crate::logic::war::general::Record;
        let maids = |g: &Game| g.enemies.iter().filter(|e| e.kind() == EnemyKind::Ellepige).count();
        // Slain: no elf-maids at his second phase.
        let mut g = court_after(Record { met: true, slain: true, ..Default::default() });
        assert!(asmodeus(&mut g).lore.summoned, "his maids are lost");
        let max = asmodeus(&mut g).max_hp;
        let from = asmodeus(&mut g).pos;
        asmodeus(&mut g).hp = max * 0.6;
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        assert_eq!(asmodeus(&mut g).breed.phase, 2);
        assert_eq!(maids(&g), 0, "none come");
        assert!(g.censers().iter().all(|c| c.2 == LIGHTS));
        // The vows spoken: the maids at once, and each censer lights only once.
        let mut g = court_after(Record { met: true, decree_done: true, ..Default::default() });
        run(&mut g, 0.1);
        assert_eq!(maids(&g), MAIDS as usize, "the maids come at once");
        assert!(g.censers().iter().all(|c| c.2 == LIGHTS_DECREED), "{:?}", g.censers());
        still(&mut g);
        strike_censer(&mut g, 0);
        assert_eq!(g.censers()[0].2, 0, "lit once, it is spent");
    }

    /// PRINCES.md §0: Asmodeus in his own court, by the bots: with the kit a hero has on reaching
    /// him (tiers 1 and 2) the expert brings him down through all three phases without falling,
    /// lighting the censers as it goes; a masher loses every life to him.
    #[test]
    fn the_expert_beats_asmodeus_in_his_court_and_a_masher_does_not() {
        use crate::game::bot::{court_fight, Loadout, Skill};
        use crate::logic::heroes::HeroKind;
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        for (tier, seed) in [(1, 1), (2, 2)] {
            let kit = (tier > 1).then(|| Loadout::all(HeroKind::Ragnar, tier));
            let f = court_fight(Sin::Lust, Skill::Expert, seed, kit, 200.0);
            assert!(f.slain && f.deaths == 0 && f.phase == 3, "tier {tier}: {f:?}");
            assert!(f.secs > 60.0 && f.secs < 160.0, "tier {tier}: {f:?}");
            assert_eq!(f.stats.overlaps, 0, "{f:?}");
        }
        let f = court_fight(Sin::Lust, Skill::Novice, 1, Some(Loadout::all(HeroKind::Ragnar, 2)), 200.0);
        assert!(!f.slain && f.game_over, "{f:?}");
    }
}
