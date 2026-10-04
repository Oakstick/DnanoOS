//! The Ledger of Grudges in the fight (WARBAND.md §8): rivals seated as the leaders of the realms'
//! packs, every foe's part in a fight followed from the war log, deeds that make a foe somebody,
//! a rival's meeting, his barks, his lesson, his weakness, his fall and his trophy.
//!
//! Only the vaults and the realms keep grudges (not the ring, the lab; not a prince's court). A
//! lesson changes a rival's choices, never a move's frames (rule I6): he waits before he strikes,
//! he wears the shield harder, the elixir he has learnt works less on him.

use super::brawl::Field;
use super::Game;
use crate::logic::combat::fsm::Defense;
use crate::logic::combat::MoveId;
use crate::logic::enemy::EnemyKind;
use crate::logic::juice::{DilationKind, Stop, Trauma};
use crate::logic::types::{dist_sq, PotionKind, V};
use crate::logic::war::ledger::{EncounterTracker, Lesson, Personality, Place, RivalId, Side, Situation, Strength, Weakness, SILVER_TAKEN};
use crate::logic::war::WarEvent;

/// Tiles within which a rival is met (his card, his bark).
pub const MEET_RANGE: f64 = 10.0;
/// Deeds that can make rivals in one level: the Ledger grows by the realm, not by the minute.
pub const PROMOTIONS_PER_LEVEL: u32 = 2;
/// A Proud rival stands taunting this long after his blow lands.
pub const PROUD_TAUNT: u16 = 60;
/// A Scarred rival takes this much more from his old wound's side.
pub const SCAR_SHARE: f64 = 1.3;
/// A Coward flees below this share of his health.
pub const COWARD_FLEE: f64 = 0.4;
/// A Schemer calls the next den when hurt below this share, after a horn this long (frames).
pub const SCHEME_AT: f64 = 0.5;
pub const SCHEME_HORN: u16 = 60;

/// The rivals and the trackers of this level.
#[derive(Clone, Debug, Default)]
pub struct Grudges {
    /// Every foe that has taken part in a fight, followed until the fight ends.
    pub trackers: Vec<EncounterTracker>,
    /// The rivals seated here: (duelist id, rival id).
    pub seated: Vec<(u32, RivalId)>,
    /// Seated for this level (so a fall and the dens' return keep the same rivals).
    pub placed: bool,
    /// Rivals met in this life (their card already shown).
    pub met: Vec<RivalId>,
    /// Promotions made in this level.
    pub promoted: u32,
    /// A rival waiting before he strikes (his lesson): (duelist id, frames left).
    pub waiting: Vec<(u32, u16)>,
    /// A Schemer's horn: (duelist id, frames left); and the schemers who have blown theirs.
    pub horns: Vec<(u32, u16)>,
    pub schemed: Vec<u32>,
}

impl Grudges {
    /**
     * Whether rival `id` holds back his `mv` a while yet (his lesson, from `ledger`): against a
     * parrying hero he waits a short or a long beat before his tell (the tell itself is whole: the
     * fill shows the real landing); against a leaping hero (`evading`) he waits out the landing of
     * the leap. (A free function of the grudges, as the brawl's ground is borrowed while foes think.)
     */
    pub fn waits(
        &mut self,
        ledger: &crate::logic::war::ledger::Ledger,
        rng: &mut crate::logic::rng::Rng,
        evading: bool,
        id: u32,
        mv: MoveId,
    ) -> bool {
        let Some(lesson) = self.seated.iter().find(|s| s.0 == id).and_then(|s| ledger.get(s.1)).and_then(|r| r.lesson) else {
            return false;
        };
        if let Some(w) = self.waiting.iter().find(|w| w.0 == id) {
            return w.1 > 0;
        }
        let wait = match lesson {
            Lesson::Parry => {
                let long = rng.next() < 0.5;
                lesson.hold(mv, long)
            }
            Lesson::Leap if evading => lesson.waits_after_leap(),
            _ => 0,
        };
        if wait == 0 {
            return false;
        }
        self.waiting.push((id, wait));
        true
    }
}

impl Game {
    /// Grudges are kept here.
    fn grudges_kept(&self) -> bool {
        self.brawl.as_ref().is_some_and(|b| b.field == Field::Vault) && !self.in_court()
    }

    fn duel_index(&self, id: u32) -> Option<usize> {
        self.enemies.iter().position(|e| e.duel.as_ref().is_some_and(|d| d.id == id))
    }

    /// The rival duelist `id` is, if any.
    pub fn rival_of(&self, id: u32) -> Option<RivalId> {
        self.war.grudges.seated.iter().find(|s| s.0 == id).map(|s| s.1)
    }

    /// The lesson and the traits of the rival duelist `id` is.
    fn rival_traits(&self, id: u32) -> Option<(Option<Lesson>, Strength, Weakness, Personality)> {
        let r = self.ledger().get(self.rival_of(id)?)?;
        Some((r.lesson, r.strength, r.weakness, r.personality))
    }

    /**
     * Seats the Ledger's rivals for this realm visit (once a level): each leads a pack, his own
     * kind among its foes dressed as him, or (if none is) he comes in with it. After a fall the
     * dens begin again, and the same rivals take their places once more.
     */
    pub(super) fn seat_rivals(&mut self) {
        if !self.grudges_kept() {
            return;
        }
        if !self.war.grudges.placed {
            self.war.grudges.placed = true;
            let tier = self.realm_tier();
            let seed = (self.level.grid.width as u32).wrapping_mul(7919) ^ (tier as u32).wrapping_mul(0x9e37_79b9);
            let offered = self.ledger().offer(tier, seed);
            let packs = self.war.packs.len();
            for (k, rival) in offered.into_iter().enumerate() {
                if packs == 0 {
                    break;
                }
                // (Not the first den: the antechamber is no place for a grudge.)
                let p = 1.min(packs - 1) + (seed as usize / 7 + k * 3) % packs.saturating_sub(1).max(1);
                let p = p.min(packs - 1);
                let Some(kind) = self.ledger().get(rival).map(|r| r.kind) else { continue };
                let member =
                    self.war.packs[p].members.iter().find(|m| m.alive && m.kind == kind && self.rival_of(m.id).is_none()).map(|m| m.id);
                let id = match member {
                    Some(id) => Some(id),
                    None => self.bring_rival(p, kind),
                };
                let Some(id) = id else { continue };
                if let (Some(i), Some(r)) = (self.duel_index(id), self.ledger().get(rival).cloned()) {
                    if let Some(d) = self.enemies[i].duel.as_deref_mut() {
                        r.dress(d);
                    }
                    self.enemies[i].b.name = r.full_name().to_uppercase();
                }
                self.war.grudges.seated.push((id, rival));
            }
        } else {
            // After a fall: each takes his armour and poise again (his health was kept).
            let seated = self.war.grudges.seated.clone();
            for (id, rival) in seated {
                let Some(r) = self.ledger().get(rival).cloned() else { continue };
                if let Some(d) = self.duel_index(id).and_then(|i| self.enemies[i].duel.as_deref_mut()) {
                    d.f.armor += r.armor_bonus();
                    d.f.max_poise *= r.poise_scale();
                    d.f.poise = d.f.max_poise;
                }
            }
        }
        // Each leads his pack.
        let seated = self.war.grudges.seated.clone();
        for (id, _) in seated {
            if let Some(p) = self.war.packs.iter_mut().find(|p| p.has(id)) {
                p.leader = id;
            }
        }
    }

    /// A rival of `kind` comes into pack `p`'s den, beside its foes. Returns his duelist's id.
    fn bring_rival(&mut self, p: usize, kind: EnemyKind) -> Option<u32> {
        let other = self.war.packs[p].members.iter().find(|m| m.alive).map(|m| m.id)?;
        let (at, den) = self.duel_index(other).and_then(|i| self.enemies[i].duel.as_deref()).map(|d| (d.pos, d.den))?;
        let spot = V::new(at.x + 0.8, at.y);
        let id = self.spawn_duelist(kind, spot, V::new(0.0, 1.0), Default::default())?;
        if let Some(d) = self.duel_index(id).and_then(|i| self.enemies[i].duel.as_deref_mut()) {
            d.den = den.map(|mut den| {
                den.home = spot;
                den
            });
            d.pos = spot;
        }
        self.war.packs[p].members.push(crate::logic::war::pack::Member {
            id,
            kind,
            role: crate::logic::war::pack::Role::of(kind),
            resting: 0,
            alive: true,
        });
        Some(id)
    }

    /// The war log, as the grudges hear it (from `update_war`): each foe's part followed, a rival
    /// met, a Proud rival's taunt, a fight's end (his fall, the hero's).
    pub(super) fn grudges_hear(&mut self, log: &[WarEvent]) {
        if !self.grudges_kept() {
            return;
        }
        let hero_hp = self.hero.hp as f64;
        for e in log {
            if let Some(foe) = e.foe() {
                if !self.war.grudges.trackers.iter().any(|t| t.foe == foe) {
                    if let Some(kind) = self.duel_index(foe).and_then(|i| self.enemies[i].duel.as_deref()).map(|d| d.breed.kind) {
                        let rival = self.rival_of(foe);
                        self.war.grudges.trackers.push(EncounterTracker::new(foe, kind, rival));
                    }
                }
            }
            let fracs: Vec<f64> = self
                .war
                .grudges
                .trackers
                .iter()
                .map(|t| {
                    self.duel_index(t.foe).and_then(|i| self.enemies[i].duel.as_deref()).map(|d| d.hp / d.max_hp.max(1.0)).unwrap_or(0.0)
                })
                .collect();
            for (t, frac) in self.war.grudges.trackers.iter_mut().zip(fracs) {
                t.observe(e, hero_hp, frac);
            }
            match *e {
                WarEvent::Landed { foe, how: Defense::Open, dmg, .. } if dmg > 0.0 => {
                    if let Some((_, _, Weakness::Proud, _)) = self.rival_traits(foe) {
                        // He stops to gloat: an opening.
                        if let Some(d) = self.duel_index(foe).and_then(|i| self.enemies[i].duel.as_deref_mut()) {
                            d.rest = d.rest.max(PROUD_TAUNT);
                            let p = d.pos;
                            self.floater(p.x, p.y - 1.6, "HE GLOATS", 1.0, Some(0xffc040));
                        }
                    }
                }
                WarEvent::Slain { foe, at, .. } => self.grudge_slain(foe, at),
                _ => {}
            }
        }
        if self.hero.hp <= 0 {
            self.grudges_hero_fell();
        }
    }

    /// Every frame of the brawl (from the packs' frame): rivals met, a coward's flight, a duelist's
    /// pack standing back, a schemer's horn.
    pub(super) fn grudges_frame(&mut self) {
        if !self.grudges_kept() {
            return;
        }
        if !self.war.grudges.placed && self.war.composed {
            self.seat_rivals();
        }
        let hp = self.hero.pos();
        let seated = self.war.grudges.seated.clone();
        for (id, rival) in seated {
            let Some(i) = self.duel_index(id) else { continue };
            let Some(d) = self.enemies[i].duel.as_deref() else { continue };
            if !d.alive() {
                continue;
            }
            let (pos, engaged, frac) = (d.pos, d.engaged(), d.hp / d.max_hp.max(1.0));
            if engaged && dist_sq(pos, hp) < MEET_RANGE * MEET_RANGE && !self.war.grudges.met.contains(&rival) {
                self.war.grudges.met.push(rival);
                self.meet_rival(rival, Some(id), pos);
                self.dilate(DilationKind::RivalMet);
                if let Some(t) = self.ledger().get(rival).map(|r| r.kind) {
                    self.audio.foe_cry(t, crate::audio::Cry::Alert, pos);
                }
            }
            let Some((_, _, _, nature)) = self.rival_traits(id) else { continue };
            match nature {
                Personality::Coward if frac < COWARD_FLEE => {
                    if let Some(d) = self.enemies[i].duel.as_deref_mut() {
                        if let Some(den) = d.den.as_mut().filter(|den| den.awake && !den.returning) {
                            den.returning = true;
                            if let Some(said) = self.rival_bark(rival, Situation::Flee) {
                                self.floater(pos.x, pos.y - 1.8, said, 2.0, Some(0xffc040));
                            }
                        }
                    }
                    if let Some(d) = self.enemies[i].duel.as_deref_mut().filter(|d| d.token) {
                        d.token = false;
                        if let Some(b) = self.brawl.as_mut() {
                            b.threat.release(id);
                        }
                    }
                }
                Personality::Duelist if engaged => {
                    // His pack stands back: the fight is his.
                    let pack: Vec<u32> =
                        self.war.packs.iter().find(|p| p.has(id)).map(|p| p.members.iter().map(|m| m.id).collect()).unwrap_or_default();
                    for w in self.war.directives.iter_mut().filter(|w| w.0 != id && pack.contains(&w.0)) {
                        w.1.ask = false;
                        w.1.back = w.1.back.max(1.5);
                    }
                }
                Personality::Schemer if frac < SCHEME_AT && !self.war.grudges.schemed.contains(&id) => {
                    self.war.grudges.schemed.push(id);
                    self.war.grudges.horns.push((id, SCHEME_HORN));
                    self.floater(pos.x, pos.y - 1.8, "HE SOUNDS HIS HORN", 1.4, Some(0xffc040));
                    self.audio.play(crate::audio::Sfx::Toll);
                }
                _ => {}
            }
        }
        // A schemer's horn: when it has sounded out (and he still stands), the next den wakes.
        let mut sounded = Vec::new();
        for h in self.war.grudges.horns.iter_mut() {
            h.1 = h.1.saturating_sub(1);
            if h.1 == 0 {
                sounded.push(h.0);
            }
        }
        self.war.grudges.horns.retain(|h| h.1 > 0);
        for id in sounded {
            let standing =
                self.duel_index(id).and_then(|i| self.enemies[i].duel.as_deref()).is_some_and(|d| d.alive() && d.f.reeling().is_none());
            if standing {
                self.wake_nearest_den(id);
            }
        }
        for w in self.war.grudges.waiting.iter_mut() {
            w.1 = w.1.saturating_sub(1);
        }
    }

    /// The sleeping den nearest duelist `id` wakes (a schemer's call).
    fn wake_nearest_den(&mut self, id: u32) {
        let Some(from) = self.duel_index(id).and_then(|i| self.enemies[i].duel.as_deref()).map(|d| d.pos) else { return };
        let near = self
            .enemies
            .iter()
            .filter_map(|e| e.duel.as_deref())
            .filter(|d| d.alive() && d.den.is_some_and(|den| !den.awake))
            .min_by(|a, b| dist_sq(a.pos, from).total_cmp(&dist_sq(b.pos, from)))
            .and_then(|d| d.den);
        let Some(den) = near else { return };
        for e in self.enemies.iter_mut() {
            if let Some(d) = e.duel.as_deref_mut().and_then(|d| d.den.as_mut()).filter(|d| d.lo == den.lo && d.hi == den.hi) {
                d.awake = true;
            }
        }
        let c = den.centre();
        self.floater(c.x, c.y, "THE CALL IS ANSWERED", 1.4, Some(0xffc85a));
    }

    /// Rival `id` began a move: his wait is spent.
    pub(super) fn rival_began(&mut self, id: u32) {
        self.war.grudges.waiting.retain(|w| w.0 != id);
    }

    /// The share of shield wear a blow by duelist `by` costs (a rival who learnt the shield wears it harder).
    pub(super) fn rival_wear(&self, by: u32) -> f64 {
        match self.rival_traits(by) {
            Some((Some(l), ..)) => l.shield_wear(),
            _ => 1.0,
        }
    }

    /**
     * What a blow from `from` on rival `id` lands as, against the whole of it: his old wound's
     * side bites deeper; the elixir he has learnt (or is proof against) lifts the hero's blow less.
     */
    pub(super) fn rival_takes(&self, id: u32, from: V) -> f64 {
        let Some(rival) = self.rival_of(id) else { return 1.0 };
        let Some(r) = self.ledger().get(rival) else { return 1.0 };
        let mut share = 1.0;
        if let (Weakness::Scarred(side), Some(d)) = (r.weakness, self.duel_index(id).and_then(|i| self.enemies[i].duel.as_deref())) {
            // His left or right, as he faces.
            let to = V::new(from.x - d.pos.x, from.y - d.pos.y);
            let cross = d.facing.x * to.y - d.facing.y * to.x;
            if (side == Side::Left) == (cross > 0.0) {
                share *= SCAR_SHARE;
            }
        }
        let fx = &self.hero.fx;
        for (on, p, lift) in [(fx.strength > 0.0, PotionKind::Strength, 1.25), (fx.ragnarok > 0.0, PotionKind::Ragnarok, 2.0)] {
            let scale = r.elixir_scale(p);
            if on && scale < 1.0 {
                share *= (1.0 + (lift - 1.0) * scale) / lift;
            }
        }
        share
    }

    /// A foe slain: if it was a rival, his fall (his reward, his trophy, his last words), and its
    /// tracker closed either way.
    fn grudge_slain(&mut self, foe: u32, at: V) {
        let Some(k) = self.war.grudges.trackers.iter().position(|t| t.foe == foe) else { return };
        let t = self.war.grudges.trackers.remove(k);
        let Some(rival) = self.rival_of(foe) else { return };
        let place = self.grudge_place();
        let tier = self.hero_tier();
        let e = t.encounter(place, self.hero_kind, tier, false);
        if let Some(said) = self.rival_bark(rival, Situation::Die) {
            self.floater(at.x, at.y - 1.8, said, 2.4, Some(0xffc040));
        }
        self.rival_fought(rival, e);
        if let Some(reward) = self.warband.save.ledger.slain(rival) {
            self.progress.runes += reward.runes;
            self.progress.silver += reward.silver;
            self.floater(at.x, at.y - 1.2, format!("TROPHY: {}", reward.trophy.name().to_uppercase()), 2.4, Some(0xffd040));
            self.save_warband();
        }
        self.freeze(Stop::Trophy);
        self.dilate(DilationKind::Trophy);
        self.view.trauma(Trauma::Trophy);
        self.war.grudges.seated.retain(|s| s.0 != foe);
        self.sync_trophies();
    }

    /// The hero fell: the foes that brought him down (and those that broke his guard thrice or lived
    /// on low) become rivals, or rise in rank; a killer takes his share of the silver.
    fn grudges_hero_fell(&mut self) {
        let trackers = std::mem::take(&mut self.war.grudges.trackers);
        let place = self.grudge_place();
        let tier = self.hero_tier();
        for t in trackers.iter().filter(|t| t.met) {
            let silver = if t.killed_hero { (self.progress.silver as f64 * SILVER_TAKEN).round() as u32 } else { 0 };
            match t.rival {
                Some(rival) => {
                    if t.killed_hero {
                        self.progress.silver -= silver;
                        self.warband.save.ledger.took_silver(rival, silver);
                        if let Some(said) = self.rival_bark(rival, Situation::Win) {
                            let hp = self.hero.pos();
                            self.floater(hp.x, hp.y - 2.0, said, 2.6, Some(0xff7a5a));
                        }
                    }
                    self.rival_fought(rival, t.encounter(place, self.hero_kind, tier, false));
                }
                None if self.war.grudges.promoted < PROMOTIONS_PER_LEVEL => {
                    let leader = self.war.packs.iter().any(|p| p.leader == t.foe);
                    if let Some(deed) = t.deed(silver, leader, false) {
                        let rival = self.promote_rival(t.kind, deed);
                        self.war.grudges.promoted += 1;
                        if silver > 0 {
                            self.progress.silver -= silver;
                            self.warband.save.ledger.took_silver(rival, silver);
                        }
                        self.rival_fought(rival, t.encounter(place, self.hero_kind, tier, false));
                        if let Some(r) = self.ledger().get(rival) {
                            let text = format!("{} REMEMBERS YOU", r.full_name().to_uppercase());
                            let hp = self.hero.pos();
                            self.floater(hp.x, hp.y - 1.4, text, 2.6, Some(0xffc040));
                        }
                    }
                }
                None => {}
            }
        }
        self.war.grudges.met.clear();
        self.war.grudges.waiting.clear();
    }

    /// Foes whose hunt was called off (he walked away): their part in the fight ends; a leader he
    /// left standing, or one that lived on low, becomes somebody.
    pub(super) fn grudges_left(&mut self, called_off: &[u32]) {
        if !self.grudges_kept() {
            return;
        }
        let place = self.grudge_place();
        let tier = self.hero_tier();
        for &foe in called_off {
            let Some(k) = self.war.grudges.trackers.iter().position(|t| t.foe == foe) else { continue };
            let t = self.war.grudges.trackers.remove(k);
            if !t.met {
                continue;
            }
            match t.rival {
                Some(rival) => {
                    self.rival_fought(rival, t.encounter(place, self.hero_kind, tier, true));
                    self.war.grudges.met.retain(|&r| r != rival);
                }
                None if self.war.grudges.promoted < PROMOTIONS_PER_LEVEL => {
                    let leader = self.war.packs.iter().any(|p| p.leader == t.foe);
                    if let Some(deed) = t.deed(0, leader, true) {
                        let rival = self.promote_rival(t.kind, deed);
                        self.war.grudges.promoted += 1;
                        self.rival_fought(rival, t.encounter(place, self.hero_kind, tier, true));
                    }
                }
                None => {}
            }
        }
    }

    /// Where this fight is, as a rival remembers it.
    fn grudge_place(&self) -> Place {
        use crate::logic::level::Theme;
        match self.level.data.theme {
            Theme::Realm(sin) => Place::Realm(sin),
            Theme::Court(sin) => Place::Court(sin),
            _ => Place::Realm(crate::logic::sins::Sin::Greed),
        }
    }

    /// His blade's tier, for what a rival remembers of it.
    fn hero_tier(&self) -> u8 {
        self.hero.blade
    }

    /// The workshop's trophy wall from the Ledger: every rival slain.
    pub(super) fn sync_trophies(&mut self) {
        let trophies = self
            .ledger()
            .rivals
            .iter()
            .filter(|r| !r.alive)
            .map(|r| super::workshop::Trophy {
                name: format!("{} OF {}", r.trophy.name().to_uppercase(), r.full_name().to_uppercase()),
                line: r.last().map(|e| e.line()).unwrap_or_default(),
                color: 0xffc040,
            })
            .collect();
        self.workshop.trophies = trophies;
    }
}
