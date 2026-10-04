//! The heroes' mastery at run time (WARBAND.md §11.1, §12): deeds counted from the war log,
//! what each rank does in the fight, and the old man's trials.
//!
//! * **Deeds**: `count_deeds` reads the war log once a frame (from `update_war`): parries, blocks
//!   and leaps from `Landed`; punishes (a blow into an opening the last frame saw), poise breaks,
//!   finishers and the halberd's tip from `HeroHit`. The rank whose deeds are met raises the toast.
//! * **The ranks in the fight**: what goes on the kit comes through `mastered` (the integrator's
//!   one call in `Game::hero_kit`); the rest are rules kept here, each bounded: stamina given back,
//!   Hold the Line's shove, the unbroken guard, the War Cry, the gold glint, the white edge.
//! * **Trials**: from the training yard, a bout of the Sparring Ring with a deed to do; then back
//!   to the yard, its card on the board and the rank granted on a pass.

use super::brawl::Field;
use super::save::Stage;
use super::sparring::{BoutPhase, AFTERMATH};
use super::workshop::Station;
use super::{Game, RowKind, Screen, ShopState};
use crate::audio::Sfx;
use crate::input::Action;
use crate::logic::combat::duelist::KNOCK_KEEP;
use crate::logic::combat::fsm::{Defense, Effect, CHAIN_GRACE};
use crate::logic::combat::{math, Attack, Kit, MoveId, Phase, Reel, State};
use crate::logic::gym;
use crate::logic::heroes::HeroKind;
use crate::logic::progression::STAMINA_MAX;
use crate::logic::story::roman;
use crate::logic::types::{Dir, V};
use crate::logic::war::mastery::{self as m, Feat, Rule, Stance, Temper, Trial, TrialResult, WAR_CRY};
use crate::logic::war::WarEvent;

/// Seconds the "MASTERY RISES" toast stays up.
pub const TOAST_SECONDS: f64 = 4.0;
/// The gold of a spent blow's glint.
const GLINT: u32 = 0xffd040;

/// A trial under way: which, its deeds so far, and what broke it (a rule), if anything.
#[derive(Clone, Debug)]
pub struct TrialRun {
    pub trial: &'static Trial,
    pub count: u8,
    pub broke: Option<&'static str>,
    pub over: bool,
}

/// The mastery's state on the game.
#[derive(Clone, Debug, Default)]
pub struct MasteryRun {
    pub temper: Temper,
    /// The brawl's frame as last read (it starts again with every fight).
    clock: u32,
    /// Foes open to a punish as the last frame ended (in their recovery, or reeling from a parry),
    /// and those already punished in it.
    recovering: Vec<u32>,
    punished: Vec<u32>,
    /// His swing as last read, and the frame he last swung on.
    swing: Option<(MoveId, u16)>,
    swung_at: u32,
    /// The War Cry under way: the frame it began, and whether it has sounded; the frame the next may.
    pub cry: Option<(u32, bool)>,
    cry_ready: u32,
    /// Seconds left of the toast, and the rank whose trial it announces.
    pub toast: f64,
    pub toast_rank: u8,
    pub trial: Option<TrialRun>,
    /// The last trial's card, shown in the yard.
    pub last_trial: Option<TrialResult>,
    /// Where a trial returns him: the checkpoint, and whether he had called the old man.
    back_to: Option<(Stage, bool)>,
}

impl Game {
    /// What his mastery makes of his kit now (rank 0 and nothing charged: the kit itself). The
    /// integrator calls it last in `Game::hero_kit`.
    pub fn mastered(&self, k: Kit) -> Kit {
        k.with_mastery(&self.stance())
    }

    /// His rank, and what his temper has charged.
    pub fn stance(&self) -> Stance {
        let rank = self.progress.hero(self.hero_kind).mastery.rank;
        self.mastery.temper.stance(self.hero_kind, rank)
    }

    /// The last guard frame that parries, when the white edge shows it (Miyamoto from rank 1, in a fight).
    pub fn parry_edge(&self) -> Option<u16> {
        let b = self.brawl.as_ref()?;
        (self.hero_kind == HeroKind::Miyamoto && self.progress.hero(self.hero_kind).mastery.rank >= 1 && self.brawling())
            .then_some(b.hero.parry_to)
    }

    /// Once a frame, after the war log is read (`update_war`): the deeds, the ranks' rules, the
    /// War Cry, the toast, the trial.
    pub(super) fn count_deeds(&mut self, dt: f64) {
        self.mastery.toast = (self.mastery.toast - dt).max(0.0);
        let Some(frame) = self.brawl.as_ref().map(|b| b.stats.frames) else { return };
        if frame < self.mastery.clock {
            // A fresh fight: nothing charged carries over.
            let keep = (self.mastery.toast, self.mastery.toast_rank);
            self.mastery.temper = Temper::default();
            self.mastery.recovering.clear();
            self.mastery.punished.clear();
            self.mastery.swing = None;
            self.mastery.swung_at = 0;
            self.mastery.cry = None;
            self.mastery.cry_ready = 0;
            (self.mastery.toast, self.mastery.toast_rank) = keep;
        }
        self.mastery.clock = frame;
        let hero = self.hero_kind;
        let rank = self.progress.hero(hero).mastery.rank;
        let events = std::mem::take(&mut self.war.read);
        for e in &events {
            self.hear_deed(e, hero, rank, frame);
        }
        self.war.read = events;
        self.watch_swing(frame);
        self.watch_recovery(hero, rank);
        self.war_cry(hero, rank, frame);
        self.trial_frame();
    }

    /// One event of the war log, as his mastery hears it.
    fn hear_deed(&mut self, e: &WarEvent, hero: HeroKind, rank: u8, frame: u32) {
        match *e {
            WarEvent::Landed { foe, how, lands, dmg, .. } => match how {
                Defense::Parried if lands => {
                    self.deed(Feat::Parry);
                    self.mastery.temper.parry(frame);
                }
                Defense::Blocked if lands => {
                    self.deed(Feat::Block);
                    self.mastery.temper.block();
                    if hero == HeroKind::Mehmed && rank >= 3 {
                        self.hold_the_line(foe);
                    }
                }
                Defense::Evaded if lands => {
                    self.deed(Feat::Evade);
                    if hero == HeroKind::Miyamoto && rank >= 4 {
                        self.refund(m::EVADE_REFUND);
                    }
                }
                Defense::GuardBroken => {
                    self.mastery.temper.broken();
                    self.trial_breach(true);
                    if hero != HeroKind::Miyamoto && rank >= m::UNBROKEN_RANK && !self.mastery.temper.unbroken_spent {
                        self.stand_unbroken();
                    }
                }
                Defense::Open if dmg > 0.0 => {
                    self.mastery.temper.wound();
                    self.trial_breach(false);
                }
                _ => {}
            },
            WarEvent::HeroHit { foe, effect, finisher, at, warded, .. } => {
                if self.mastery.recovering.contains(&foe) && !self.mastery.punished.contains(&foe) {
                    self.mastery.punished.push(foe);
                    self.deed(Feat::Punish);
                }
                if effect == Effect::Broken {
                    self.deed(Feat::Break);
                }
                if finisher {
                    self.deed(Feat::Finisher);
                }
                if hero == HeroKind::Mehmed {
                    if let Some(poise) = self.tip_hit(foe, at) {
                        self.deed(Feat::Tip);
                        if rank >= 2 {
                            self.tip_poise(foe, poise * (1.0 - warded));
                        }
                    }
                }
            }
            WarEvent::Slain { last_of_pack, .. } => {
                if hero == HeroKind::Ragnar && rank >= 3 && self.time_scale() < 1.0 {
                    self.refund(m::DILATION_REFUND);
                }
                if last_of_pack {
                    self.mastery.temper.unbroken_spent = false;
                    // The deeds are kept as each pack falls.
                    self.save_progress();
                }
            }
            _ => {}
        }
    }

    /// A deed done: his mastery counts it (the toast when it meets a rank), and so does the trial.
    fn deed(&mut self, feat: Feat) {
        let hero = self.hero_kind;
        let mastery = &mut self.progress.hero_mut(hero).mastery;
        if mastery.count(hero, feat) {
            self.mastery.toast = TOAST_SECONDS;
            self.mastery.toast_rank = mastery.rank + 1;
            self.audio.play(Sfx::Milestone);
            self.save_progress();
        }
        if let Some(t) = self.mastery.trial.as_mut().filter(|t| !t.over && t.trial.feat == feat) {
            t.count = t.count.saturating_add(1);
        }
    }

    fn refund(&mut self, stamina: f64) {
        self.hero.stamina = (self.hero.stamina + stamina).min(STAMINA_MAX);
    }

    /// Hold the Line (Mehmed 3): the body foe whose blow he blocked is shoved half a tile back (its
    /// knock, as a flinch's; it is not slowed).
    fn hold_the_line(&mut self, foe: u32) {
        let hp = self.hero.pos();
        let push = m::HOLD_THE_LINE * crate::logic::combat::FPS * (1.0 - KNOCK_KEEP);
        if let Some(du) = self
            .enemies
            .iter_mut()
            .filter_map(|e| e.duel.as_deref_mut())
            .find(|d| d.id == foe && d.alive() && d.breed.attack() == Attack::Body)
        {
            let away = math::norm(math::sub(du.pos, hp));
            du.knock = math::add(du.knock, math::scale(away, push));
        }
    }

    /// A blow of his swing on `foe` at `at`: was it the tip of his reach? Its poise if so.
    fn tip_hit(&self, foe: u32, at: V) -> Option<f64> {
        let b = self.brawl.as_ref()?;
        let (mv, _, _) = b.hero.attack()?;
        let d = b.hero.data(mv);
        let du = self.enemies.iter().filter_map(|e| e.duel.as_deref()).find(|d| d.id == foe)?;
        let reached = math::len(math::sub(at, self.hero.pos())) - du.radius();
        (!d.projectile && reached >= d.reach - m::TIP).then_some(d.poise)
    }

    /// The tip breaks harder (Mehmed 2): a share of the blow's poise again.
    fn tip_poise(&mut self, foe: u32, poise: f64) {
        let extra = poise * m::TIP_POISE;
        let Some(du) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == foe && d.alive()) else {
            return;
        };
        if du.f.shake(extra) {
            let p = du.pos;
            self.floater(p.x, p.y - 1.0, "BREAK!", 0.8, Some(0xffd040));
        }
    }

    /// Once a pack (Ragnar 5, Mehmed 5): the guard that broke stands.
    fn stand_unbroken(&mut self) {
        let Some(b) = self.brawl.as_mut() else { return };
        if !matches!(b.hero.state, State::Reel { kind: Reel::GuardBroken, .. }) {
            return;
        }
        b.hero.state = State::Guard { frame: b.hero.parry_to + 1 };
        self.mastery.temper.unbroken_spent = true;
        let hp = self.hero.pos();
        self.floater(hp.x, hp.y - 1.2, "UNBROKEN", 1.0, Some(0xffe070));
        self.audio.play(Sfx::Clang);
    }

    /// His swing as it goes: a new one is charged by his temper; a chain's grace gone, it rests.
    fn watch_swing(&mut self, frame: u32) {
        let now = self.brawl.as_ref().and_then(|b| b.hero.attack()).map(|(mv, _, f)| (mv, f));
        let began = match (now, self.mastery.swing) {
            (Some((mv, f)), Some((was, g))) => mv != was || f < g,
            (Some(_), None) => true,
            _ => false,
        };
        if began {
            self.mastery.temper.swing(frame);
        }
        if now.is_some() {
            self.mastery.swung_at = frame;
        } else if frame.saturating_sub(self.mastery.swung_at) > CHAIN_GRACE as u32 {
            self.mastery.temper.rest();
        }
        self.mastery.swing = now;
    }

    /// The foes open to a punish (a recovery, a parry's reel); the gold glint as one opens (Ragnar 1).
    fn watch_recovery(&mut self, hero: HeroKind, rank: u8) {
        let now: Vec<(u32, V)> = self
            .enemies
            .iter()
            .filter_map(|e| e.duel.as_deref())
            .filter(|d| d.alive() && (matches!(d.f.attack(), Some((_, Phase::Recovery, _))) || d.f.reeling() == Some(Reel::Deflected)))
            .map(|d| (d.id, d.pos))
            .collect();
        if hero == HeroKind::Ragnar && rank >= 1 {
            for &(id, p) in &now {
                if !self.mastery.recovering.contains(&id) {
                    self.view.sparkle(&self.level.grid, p.x, p.y, 1.3, GLINT, 12);
                }
            }
        }
        self.mastery.punished.retain(|id| now.iter().any(|(n, _)| n == id));
        self.mastery.recovering = now.into_iter().map(|(id, _)| id).collect();
    }

    /**
     * The War Cry (Ragnar 4; F or L3, where it is not the blink): his fighter is held for its
     * frames (struck in its startup, the cry is lost); on its first active frame every body foe
     * about him staggers.
     */
    fn war_cry(&mut self, hero: HeroKind, rank: u8, frame: u32) {
        let live = self.brawl.as_ref().is_some_and(|b| b.live || b.field == Field::Vault);
        if hero != HeroKind::Ragnar || rank < 4 || !self.brawling() || !live {
            self.mastery.cry = None;
            return;
        }
        let total = WAR_CRY.total();
        match self.mastery.cry {
            None => {
                if self.input.was_pressed(&[Action::Step]) && self.war_cry_ready() {
                    let Some(b) = self.brawl.as_mut() else { return };
                    b.hero.state = State::Reel { kind: Reel::Hitstun, left: total, total };
                    self.hero.exert(WAR_CRY.stamina);
                    self.mastery.cry = Some((frame, false));
                    self.mastery.cry_ready = frame + WAR_CRY.rest;
                    self.audio.play(Sfx::Growl);
                }
            }
            Some((at, sounded)) => {
                let Some(b) = self.brawl.as_ref() else { return };
                let el = frame.saturating_sub(at);
                // Held still in the cry's own frames; a blow's hitstun in its place ends it.
                let held = matches!(b.hero.state, State::Reel { kind: Reel::Hitstun, left, total: t }
                    if t == total && (left as u32 + el).abs_diff(total as u32) <= 2);
                if !held || el >= total as u32 {
                    self.mastery.cry = None;
                    return;
                }
                if !sounded && el >= WAR_CRY.startup as u32 {
                    self.mastery.cry = Some((at, true));
                    self.sound_war_cry();
                }
            }
        }
    }

    /// The cry sounds: every body foe within its reach staggers.
    fn sound_war_cry(&mut self) {
        let hp = self.hero.pos();
        let mut shaken = Vec::new();
        for e in self.enemies.iter_mut() {
            let Some(du) = e.duel.as_deref_mut() else { continue };
            if du.alive() && du.breed.attack() == Attack::Body && math::len(math::sub(du.pos, hp)) <= WAR_CRY.radius + du.radius() {
                let max = du.f.max_poise;
                if du.f.shake(max) {
                    shaken.push(du.pos);
                }
            }
        }
        self.audio.play(Sfx::BossRoar);
        self.view.shake(0.25);
        self.view.dust(&self.level.grid, hp.x, hp.y, 0xe8d0a0, 36, 4.0);
        self.floater(hp.x, hp.y - 1.4, "WAR CRY", 1.2, Some(0xffb040));
        for p in shaken {
            self.floater(p.x, p.y - 0.9, "STAGGERED", 0.9, Some(0xffd040));
        }
    }

    /// Could he cry the War Cry now (Ragnar 4: in a fight, free, his breath and its rest)?
    pub fn war_cry_ready(&self) -> bool {
        let Some(b) = self.brawl.as_ref() else { return false };
        self.hero_kind == HeroKind::Ragnar
            && self.progress.hero(self.hero_kind).mastery.rank >= 4
            && self.brawling()
            && self.mastery.cry.is_none()
            && b.stats.frames >= self.mastery.cry_ready
            && matches!(b.hero.state, State::Neutral | State::Guard { .. })
            && !self.hero.airborne()
            && self.hero.stamina + 1e-9 >= WAR_CRY.stamina
    }

    // ---- the trials

    /// The trial set for his next rank (its deeds met), if any.
    pub fn trial_ready(&self) -> Option<&'static Trial> {
        let hero = self.hero_kind;
        self.progress.hero(hero).mastery.ready().map(|r| m::trial(hero, r))
    }

    /// From the yard: into the Sparring Ring for the trial that is set. (The checkpoint is kept:
    /// he comes back to it, at the yard, when the trial is over.)
    pub fn begin_trial(&mut self) -> bool {
        let Some(tr) = self.trial_ready() else { return false };
        self.begin_trial_of(tr);
        true
    }

    /// Into the ring for `tr` whatever the deeds (captures and tests).
    pub fn begin_trial_of(&mut self, tr: &'static Trial) {
        let stage = self.saved_run.as_ref().map(|r| r.stage).unwrap_or(Stage::Shop(0));
        self.mastery.back_to = Some((stage, self.shop.called));
        self.audio.play(Sfx::Whoosh);
        self.load_gym();
        self.enter(Screen::Playing);
        self.hero.place(gym::ring_start());
        self.hero.face(Dir::Left);
        if let Some(s) = self.spar.as_mut() {
            s.bout = tr.bout;
            s.label = Some(format!("TRIAL {}: {}", roman(tr.rank as usize), tr.name));
        }
        self.mastery.trial = Some(TrialRun { trial: tr, count: 0, broke: None, over: false });
        self.start_bout();
        self.juice.fade = 1.0;
        for a in [Action::Confirm, Action::Jump, Action::Attack, Action::Tap] {
            self.input.consume(a);
        }
    }

    /// A rule of the trial broken: a guard broken (`guard`) or a wound.
    fn trial_breach(&mut self, guard: bool) {
        let Some(t) = self.mastery.trial.as_mut().filter(|t| !t.over) else { return };
        match t.trial.rule {
            Rule::NoGuardBreak if guard => t.broke = Some("YOUR GUARD WAS BROKEN"),
            Rule::NoWound => t.broke = Some("YOU WERE WOUNDED"),
            _ => {}
        }
    }

    /// Has the trial been passed or failed yet?
    fn trial_frame(&mut self) {
        let Some(run) = self.mastery.trial.as_ref().filter(|t| !t.over) else { return };
        let tr = run.trial;
        let secs = self.brawl.as_ref().map(|b| b.stats.frames as f64 / 60.0).unwrap_or(0.0);
        let outcome = if run.count >= tr.count {
            Some((true, ""))
        } else if let Some(why) = run.broke {
            Some((false, why))
        } else {
            match self.spar.as_ref().map(|s| s.phase) {
                Some(BoutPhase::Won(_)) => Some((false, "ALL DOWN BEFORE THE DEEDS WERE DONE")),
                Some(BoutPhase::Lost(_)) => Some((false, "DEFEATED")),
                Some(BoutPhase::Fight) if secs > tr.limit => Some((false, "OUT OF TIME")),
                Some(BoutPhase::Idle) | None => Some((false, "YOU LEFT THE RING")),
                _ => None,
            }
        };
        if let Some((passed, why)) = outcome {
            self.end_trial(passed, why, secs);
        }
    }

    /// The trial is over: the card, the rank if it was passed, and the aftermath before the yard.
    fn end_trial(&mut self, passed: bool, why: &'static str, secs: f64) {
        let hero = self.hero_kind;
        let Some(run) = self.mastery.trial.as_mut() else { return };
        run.over = true;
        let (tr, count) = (run.trial, run.count);
        let granted = self.progress.hero_mut(hero).mastery.tried(tr.rank, count, passed);
        self.mastery.last_trial =
            Some(TrialResult { hero, rank: tr.rank, passed, granted, count: count.min(tr.count), of: tr.count, secs, why });
        self.save_progress();
        let hp = self.hero.pos();
        if let Some(b) = self.brawl.as_mut() {
            b.live = false;
        }
        let phase = self.spar.as_ref().map(|s| s.phase);
        match phase {
            Some(BoutPhase::Idle) | None => {
                // He walked out: straight back to the old man.
                self.trial_over();
                return;
            }
            Some(BoutPhase::Won(t) | BoutPhase::Lost(t)) => {
                if let Some(s) = self.spar.as_mut() {
                    s.phase = if passed { BoutPhase::Won(t) } else { BoutPhase::Lost(t) };
                }
            }
            _ => {
                if let Some(s) = self.spar.as_mut() {
                    s.phase = if passed { BoutPhase::Won(AFTERMATH) } else { BoutPhase::Lost(AFTERMATH) };
                }
            }
        }
        if passed {
            self.audio.play(Sfx::Win);
            self.floater(hp.x, hp.y - 1.4, "TRIAL PASSED", 2.0, Some(0xffd040));
            if granted {
                self.floater(hp.x, hp.y - 0.9, format!("MASTERY RANK {}", roman(tr.rank as usize)), 2.0, Some(0xffe8b0));
            }
        } else {
            self.audio.play(Sfx::Denied);
            self.floater(hp.x, hp.y - 1.4, "TRIAL FAILED", 2.0, Some(0xff6a4a));
        }
    }

    /// The trial's aftermath is over: back where he was, at the old man's yard, the trials' row
    /// chosen so the card shows.
    pub(super) fn trial_over(&mut self) {
        self.mastery.trial = None;
        let (stage, called) = self.mastery.back_to.take().unwrap_or((Stage::Shop(0), false));
        self.map_jump(stage);
        self.shop = ShopState { called, ..ShopState::default() };
        self.go_to_station(Station::Yard);
        if let Some(r) = self.shop_rows().iter().position(|r| r.kind == RowKind::Trials) {
            self.shop.row = r;
        }
        self.enter(Screen::Shop);
        self.juice.fade = 1.0;
    }

    /// A trial is under way (the ring's aftermath goes back to the yard).
    pub fn in_trial(&self) -> bool {
        self.mastery.trial.is_some()
    }

    // ---- capture hooks

    /// Plays hero `k`: 0 Ragnar, 1 Mehmed, 2 Miyamoto.
    pub fn debug_hero(&mut self, k: usize) {
        self.hero_kind = [HeroKind::Ragnar, HeroKind::Mehmed, HeroKind::Miyamoto][k.min(2)];
        self.equip_from_progress();
    }

    /// His mastery at rank `n`, the next rank's deeds part done (or its trial set, at an even rank).
    pub fn debug_mastery(&mut self, n: u8) {
        let hero = self.hero_kind;
        let rank = n.min(m::RANKS);
        let mst = &mut self.progress.hero_mut(hero).mastery;
        mst.rank = rank;
        let need = m::DEEDS_FOR[rank.min(m::RANKS - 1) as usize];
        let have = if rank.is_multiple_of(2) { need } else { need * 3 / 5 };
        let kinds = m::way(hero).feats.len() as u32;
        for (k, d) in mst.deeds.iter_mut().enumerate().take(kinds as usize) {
            *d = have / kinds + u32::from((k as u32) < have % kinds);
        }
        for k in 0..rank as usize {
            mst.tries[k] = 1 + (k as u8 % 2);
            mst.best[k] = m::trial(hero, k as u8 + 1).count;
        }
    }

    /// The training yard, its trials' card up.
    pub fn debug_yard(&mut self) {
        if self.screen != Screen::Shop {
            self.open_shop();
        }
        self.go_to_station(Station::Yard);
        if let Some(r) = self.shop_rows().iter().position(|r| r.kind == RowKind::Trials) {
            self.shop.row = r;
        }
    }

    /// His trial of rank `n`, in the ring.
    pub fn debug_trial(&mut self, n: u8) {
        self.begin_trial_of(m::trial(self.hero_kind, n));
    }

    /// The toast, as if a rank's deeds were just met.
    pub fn debug_toast(&mut self) {
        self.mastery.toast = TOAST_SECONDS;
        self.mastery.toast_rank = self.progress.hero(self.hero_kind).mastery.rank + 1;
    }
}

#[cfg(test)]
mod tests {
    use super::super::bot::{fresh_game, Bot, Loadout, Skill};
    use super::*;
    use crate::logic::enemy::EnemyKind;
    use crate::logic::war::mastery::{Mastery, DEEDS_FOR};

    fn as_hero(hero: HeroKind) -> Game {
        let mut g = fresh_game(1);
        Loadout { hero, blade: 1, shield: 1, path: 0 }.equip(&mut g);
        g
    }

    fn ready(g: &mut Game, rank: u8) {
        let hero = g.hero_kind;
        let mst = &mut g.progress.hero_mut(hero).mastery;
        *mst = Mastery { rank: rank - 1, ..Mastery::default() };
        mst.deeds[0] = DEEDS_FOR[(rank - 1) as usize];
    }

    #[test]
    fn deeds_are_counted_from_the_war_log() {
        let mut g = as_hero(HeroKind::Ragnar);
        g.debug_ring(1);
        let v = V::new(1.0, 0.0);
        let foe = g.enemies.iter().find_map(|e| e.duel.as_ref().map(|d| d.id)).unwrap();
        // The foe was in its recovery when the last frame ended: the first blow into it is a punish.
        g.mastery.recovering = vec![foe];
        let hit = |effect, finisher| WarEvent::HeroHit {
            foe,
            kind: EnemyKind::Blemmyes,
            effect,
            dmg: 10.0,
            finisher,
            dir: v,
            at: v,
            warded: 0.0,
        };
        g.war.read = vec![hit(Effect::Flinched(10), false), hit(Effect::Broken, false), hit(Effect::Held, true)];
        g.count_deeds(0.0);
        let mst = g.progress.hero(HeroKind::Ragnar).mastery;
        assert_eq!(mst.deeds, [1, 1, 1], "one punish (not three), a break, a finisher");
        // Blocks and parries are not the berserker's way.
        g.war.read = vec![WarEvent::Landed { foe, kind: None, how: Defense::Parried, dmg: 0.0, dir: v, lands: true, bolt: false }];
        g.count_deeds(0.0);
        assert_eq!(g.progress.hero(HeroKind::Ragnar).mastery.total(), 3);
    }

    #[test]
    fn the_deeds_that_meet_a_rank_raise_the_toast() {
        let mut g = as_hero(HeroKind::Miyamoto);
        g.debug_ring(1);
        g.progress.hero_mut(HeroKind::Miyamoto).mastery.deeds = [DEEDS_FOR[0] - 1, 0, 0];
        let v = V::new(1.0, 0.0);
        g.war.read = vec![WarEvent::Landed { foe: 1, kind: None, how: Defense::Evaded, dmg: 0.0, dir: v, lands: true, bolt: false }];
        g.count_deeds(0.0);
        assert!(g.mastery.toast > 0.0 && g.mastery.toast_rank == 1, "MASTERY RISES");
        assert_eq!(g.trial_ready().map(|t| t.name), Some("THREE BLOWS PARRIED"));
    }

    #[test]
    fn a_trial_from_the_yard_is_fought_in_the_ring_and_returns_to_the_yard_with_its_card() {
        let mut g = as_hero(HeroKind::Miyamoto);
        ready(&mut g, 1);
        g.open_shop();
        g.go_to_station(Station::Yard);
        let rows = g.shop_rows();
        let r = rows.iter().position(|r| r.kind == RowKind::Trials).expect("the trials' row");
        assert!(rows[r].buttons[0].enabled, "its trial is set");
        g.shop.row = r;
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        assert!(g.in_trial() && g.in_ring() && g.screen == Screen::Playing, "into the ring");
        assert_eq!(g.spar.as_ref().unwrap().title(), "TRIAL I: THREE BLOWS PARRIED");
        let mut bot = Bot::new(Skill::Expert, 1);
        for _ in 0..((m::TRIAL_LIMIT + 10.0) / crate::app::STEP) as usize {
            let pad = bot.act(&g);
            g.input.set_pad(Some(pad));
            g.update(crate::app::STEP);
            if g.screen == Screen::Shop {
                break;
            }
        }
        g.input.set_pad(None);
        let card = g.mastery.last_trial.clone().expect("a card");
        assert!(card.passed && card.granted && card.count == 3, "{card:?}");
        assert_eq!(g.progress.hero(HeroKind::Miyamoto).mastery.rank, 1);
        assert_eq!((g.screen, g.station()), (Screen::Shop, Station::Yard), "back at the yard");
        assert!(!g.in_trial());
    }

    #[test]
    fn a_trial_is_failed_by_its_rule_and_grants_nothing() {
        let mut g = as_hero(HeroKind::Mehmed);
        ready(&mut g, 1);
        assert!(g.begin_trial());
        let v = V::new(1.0, 0.0);
        g.war.read = vec![WarEvent::Landed { foe: 1, kind: None, how: Defense::GuardBroken, dmg: 5.0, dir: v, lands: true, bolt: false }];
        g.count_deeds(0.0);
        let card = g.mastery.last_trial.clone().expect("a card");
        assert!(!card.passed && card.why == "YOUR GUARD WAS BROKEN");
        assert_eq!(g.progress.hero(HeroKind::Mehmed).mastery.rank, 0);
        assert_eq!(g.progress.hero(HeroKind::Mehmed).mastery.tries[0], 1);
        assert!(matches!(g.spar.as_ref().unwrap().phase, BoutPhase::Lost(_)));
    }

    #[test]
    fn the_unbroken_guard_stands_once_a_pack_and_hold_the_line_shoves_a_body_foe() {
        let mut g = as_hero(HeroKind::Mehmed);
        g.progress.hero_mut(HeroKind::Mehmed).mastery.rank = 5;
        g.debug_ring(2); // the pack: two wolves, body foes
        let v = V::new(1.0, 0.0);
        let broken = WarEvent::Landed { foe: 1, kind: None, how: Defense::GuardBroken, dmg: 5.0, dir: v, lands: true, bolt: false };
        let reel = State::Reel { kind: Reel::GuardBroken, left: 40, total: 48 };
        g.brawl.as_mut().unwrap().hero.state = reel;
        g.war.read = vec![broken];
        g.count_deeds(0.0);
        assert!(matches!(g.brawl.as_ref().unwrap().hero.state, State::Guard { .. }), "guarded");
        g.brawl.as_mut().unwrap().hero.state = reel;
        g.count_deeds(0.0);
        assert_eq!(g.brawl.as_ref().unwrap().hero.state, reel, "once a pack");
        // A blocked bite: the wolf goes back half a tile.
        let (id, before) = g.enemies.iter().find_map(|e| e.duel.as_ref().map(|d| (d.id, d.knock))).unwrap();
        assert_eq!(before, V::new(0.0, 0.0));
        g.war.read = vec![WarEvent::Landed { foe: id, kind: None, how: Defense::Blocked, dmg: 0.0, dir: v, lands: true, bolt: false }];
        g.count_deeds(0.0);
        let k = g.enemies.iter().find_map(|e| e.duel.as_ref().filter(|d| d.id == id).map(|d| d.knock)).unwrap();
        let tiles = math::len(k) / crate::logic::combat::FPS / (1.0 - KNOCK_KEEP);
        assert!((tiles - m::HOLD_THE_LINE).abs() < 1e-9, "{tiles}");
    }

    /// Mastery rewards skill (WARBAND.md §11.1): at rank 5 the expert deals its damage faster than
    /// at rank 0, by more than the masher gains (who gains next to nothing), for every hero; a masher
    /// beats no Brute he could not beat before; the expert still wins all. (Its kit needs `mastered` in `hero_kit`.)
    #[test]
    #[ignore = "needs the integrator's `self.mastered(k)` at the end of Game::hero_kit (game/brawl.rs)"]
    fn mastery_grows_the_experts_margin_more_than_the_mashers() {
        use super::super::bot::{mastery_line, play_bout_mastered};
        use crate::logic::heroes::ROSTER;
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        let seeds = [1, 2];
        let mut total = [0.0; 2];
        for hero in ROSTER {
            let l = Some(Loadout::all(hero, 1));
            let mut gain = [0.0; 2];
            for (s, skill) in [Skill::Novice, Skill::Expert].into_iter().enumerate() {
                for bout in [1, 3] {
                    let (w0, _, dps0) = mastery_line(bout, skill, &seeds, l, 0);
                    let (w5, _, dps5) = mastery_line(bout, skill, &seeds, l, m::RANKS);
                    gain[s] += dps5 / dps0 - 1.0;
                    if skill == Skill::Expert {
                        assert!(w0 == 1.0 && w5 == 1.0, "{hero:?} the expert wins bout {bout} at both ranks");
                    }
                }
            }
            assert!(gain[1] >= gain[0] - 0.02, "{hero:?}: the expert gains {:.0}%, the masher {:.0}%", gain[1] * 50.0, gain[0] * 50.0);
            assert!(gain[0] <= 0.05, "{hero:?}: mashing earns little from mastery ({:.0}%)", gain[0] * 50.0);
            total[0] += gain[0];
            total[1] += gain[1];
            // A masher the Brute beats at rank 0 it beats at rank 5.
            if !play_bout_mastered(0, Skill::Novice, 1, l, 0).won {
                assert!(!play_bout_mastered(0, Skill::Novice, 1, l, m::RANKS).won, "{hero:?}: mastery made a masher beat the Brute");
            }
        }
        assert!(total[1] > total[0] + 0.1, "the expert's margin grows: {total:?}");
    }

    #[test]
    fn the_war_cry_holds_him_for_its_frames_and_staggers_the_body_foes_about_him() {
        let mut g = as_hero(HeroKind::Ragnar);
        g.progress.hero_mut(HeroKind::Ragnar).mastery.rank = 4;
        g.debug_ring(2);
        g.hero.free_move = true;
        // Through the countdown, the wolves brought up close.
        while !g.brawl.as_ref().unwrap().live {
            g.update(crate::app::STEP);
        }
        let wolf = g.enemies.iter().find_map(|e| e.duel.as_ref().map(|d| d.pos)).unwrap();
        g.hero.place(crate::logic::types::P::new(wolf.x.round() as i32 + 1, wolf.y.round() as i32));
        g.input.inject(Action::Step);
        g.update(crate::app::STEP);
        assert!(g.mastery.cry.is_some(), "he cries");
        let mut staggered = false;
        for _ in 0..(WAR_CRY.startup as usize + 4) * 2 {
            g.update(crate::app::STEP);
            staggered |= g.enemies.iter().filter_map(|e| e.duel.as_deref()).any(|d| d.f.reeling() == Some(Reel::Staggered));
        }
        // (Unless a bite caught him first, which ends the cry.)
        assert!(staggered || g.mastery.cry.is_none(), "the wolves stagger");
        assert!(staggered, "a wolf about him staggered");
    }
}
