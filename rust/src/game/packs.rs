//! The packs at run time (WARBAND.md §7): the dens' foes made into bands, their morale heard from
//! the war log, their leaders' plays turned into wishes the threat manager and the sectors read,
//! their breaking, their affixes and their rallies.

use super::brawl::Field;
use super::Game;
use crate::audio::Sfx;
use crate::logic::combat::MoveId;
use crate::logic::enemy::EnemyKind;
use crate::logic::types::{dist_sq, V};
use crate::logic::war::pack::{self, Affix, PackCue, Role, Seen, Temperament};
use crate::logic::war::WarEvent;

/// What a pack does as it breaks: the words over it, and their colour.
fn broke_line(t: Temperament) -> (&'static str, u32) {
    match t {
        Temperament::Cowardly => ("THE PACK BREAKS AND FLEES", 0x9ad1ff),
        Temperament::Frenzied => ("THE PACK GOES BERSERK", 0xff6a3a),
        Temperament::Stalwart => ("THE PACK STANDS FIRM", 0xc8d0da),
    }
}

impl Game {
    /// The packs fight here: the vaults and the realms (not the ring, the lab; not a prince's court).
    fn packs_fight(&self) -> bool {
        static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if *OFF.get_or_init(|| std::env::var_os("VH_NO_PACKS").is_some()) {
            return false; // (for A/B runs of the harness)
        }
        self.brawl.as_ref().is_some_and(|b| b.field == Field::Vault) && !self.in_court()
    }

    /// Where duelist `id` is in `self.enemies`.
    fn duel_at(&self, id: u32) -> Option<usize> {
        self.enemies.iter().position(|e| e.duel.as_ref().is_some_and(|d| d.id == id))
    }

    /// The dens' foes made into packs (once a level, and again after he falls): roles, a leader, its
    /// affixes by the realm's tier.
    fn compose_packs(&mut self) {
        let tier = self.realm_tier();
        type Den = (V, V, Vec<(u32, EnemyKind, f64)>);
        let mut dens: Vec<Den> = Vec::new();
        for d in self.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.counts() && !d.puppet) {
            let Some(den) = d.den else { continue };
            match dens.iter_mut().find(|k| k.0 == den.lo && k.1 == den.hi) {
                Some(k) => k.2.push((d.id, d.breed.kind, d.max_hp)),
                None => dens.push((den.lo, den.hi, vec![(d.id, d.breed.kind, d.max_hp)])),
            }
        }
        self.war.packs = dens
            .into_iter()
            .map(|(lo, hi, foes)| {
                let seed = (lo.x * 73.0 + lo.y * 131.0 + hi.x * 7.0) as u32 ^ (tier as u32).wrapping_mul(0x9e37_79b9);
                pack::compose(lo, hi, &foes, tier, seed)
            })
            .collect();
        let iron: Vec<u32> = self.war.packs.iter().filter(|p| p.has_affix(Affix::Ironhide)).map(|p| p.leader).collect();
        for id in iron {
            if let Some(d) = self.duel_at(id).and_then(|i| self.enemies[i].duel.as_deref_mut()) {
                d.f.armor += pack::IRONHIDE_ARMOR;
            }
        }
        self.war.composed = true;
    }

    /// The packs are broken up (he fell and the dens begin again, or a new level): a frenzy's pace
    /// given back, to be composed afresh.
    pub(super) fn disband_packs(&mut self) {
        let frenzied: Vec<u32> =
            self.war.packs.iter().filter(|p| p.frenzy > 0).flat_map(|p| p.members.iter().filter(|m| m.alive).map(|m| m.id)).collect();
        for id in frenzied {
            if let Some(d) = self.duel_at(id).and_then(|i| self.enemies[i].duel.as_deref_mut()) {
                d.breed.speed /= pack::FRENZY_PACE;
            }
        }
        self.war.packs.clear();
        self.war.directives.clear();
        self.war.composed = false;
    }

    /// The war log, as each pack hears it: its morale, and its breaking.
    pub(super) fn packs_hear(&mut self, e: &WarEvent) {
        for k in 0..self.war.packs.len() {
            let alive: Vec<u32> = self.war.packs[k].members.iter().filter(|m| m.alive).map(|m| m.id).collect();
            let near = alive.iter().any(|&id| self.duel_at(id).and_then(|i| self.enemies[i].duel.as_deref()).is_some_and(|d| d.engaged()));
            let Some(PackCue::Broke(t)) = self.war.packs[k].hear(e, near) else { continue };
            let alive: Vec<u32> = self.war.packs[k].members.iter().filter(|m| m.alive).map(|m| m.id).collect();
            if t == Temperament::Frenzied {
                for &id in &alive {
                    if let Some(d) = self.duel_at(id).and_then(|i| self.enemies[i].duel.as_deref_mut()) {
                        d.breed.speed *= pack::FRENZY_PACE;
                    }
                }
            }
            if t == Temperament::Cowardly {
                // It turns to flee at once: what it was about to strike with goes back.
                for &id in &alive {
                    let Some(d) = self.duel_at(id).and_then(|i| self.enemies[i].duel.as_deref_mut()) else { continue };
                    if let Some(den) = d.den.as_mut().filter(|den| den.awake) {
                        den.returning = true;
                    }
                    d.token = false;
                    d.slot = None;
                    if let Some(b) = self.brawl.as_mut() {
                        b.threat.release(id);
                    }
                }
            }
            if t != Temperament::Stalwart {
                self.war.drum.routed();
                self.war.tally.routs += 1;
            }
            let at = alive.first().and_then(|&id| self.duel_at(id)).and_then(|i| self.enemies[i].duel.as_deref()).map(|d| d.pos);
            if let Some(p) = at {
                let (text, color) = broke_line(t);
                self.floater(p.x, p.y - 1.5, text, 1.6, Some(color));
            }
        }
    }

    /**
     * One frame of the packs (from the brawl's foes' frame, after the dens): each leader's play
     * becomes its foes' wishes; a routed pack walks home and gives its tokens back; a gilded
     * leader flees when hurt; a frenzy and a vengeful heart quicken their rests; a banner brings
     * poise back; a leader sounds its rally.
     */
    pub(super) fn packs_frame(&mut self) {
        self.war.directives.clear();
        if !self.packs_fight() {
            return;
        }
        if !self.war.composed {
            self.compose_packs();
        }
        let hp = self.hero.pos();
        let aim = self.hero.aim();
        let blocking = self.hero.blocking;
        let frame = self.brawl.as_ref().map(|b| b.threat.frame).unwrap_or(0);
        let mut releases = Vec::new();
        let mut rallies = Vec::new();
        for k in 0..self.war.packs.len() {
            let seen: Vec<Seen> = self.war.packs[k]
                .members
                .iter()
                .filter(|m| m.alive)
                .filter_map(|m| self.duel_at(m.id).and_then(|i| self.enemies[i].duel.as_deref()))
                .map(|d| Seen {
                    id: d.id,
                    pos: d.pos,
                    hp: d.hp / d.max_hp.max(1.0),
                    engaged: d.engaged(),
                    free: d.f.is_neutral() && !d.token && d.rest == 0,
                })
                .collect();
            let was_frenzied = self.war.packs[k].frenzy > 0;
            let (wishes, rally) = self.war.packs[k].frame(&seen, hp, aim, blocking);
            self.war.directives.extend(wishes);
            let p = &self.war.packs[k];
            let (leader, rout, frenzy, vengeance) = (p.leader, p.rout > 0, p.frenzy > 0, p.vengeance > 0);
            let (gilded, banner) = (p.has_affix(Affix::Gilded), p.has_affix(Affix::Bannered));
            let alive: Vec<u32> = p.members.iter().filter(|m| m.alive).map(|m| m.id).collect();
            if rally {
                rallies.push(leader);
            }
            let leader_at = self.duel_at(leader).and_then(|i| self.enemies[i].duel.as_deref()).filter(|d| d.alive()).map(|d| d.pos);
            for &id in &alive {
                let Some(d) = self.duel_at(id).and_then(|i| self.enemies[i].duel.as_deref_mut()) else { continue };
                if was_frenzied && !frenzy {
                    d.breed.speed /= pack::FRENZY_PACE;
                }
                let flees = rout || (gilded && id == leader && d.hp < d.max_hp * pack::GILDED_FLEE);
                if flees {
                    if let Some(den) = d.den.as_mut().filter(|den| den.awake) {
                        den.returning = true;
                    }
                    if d.token {
                        d.token = false;
                        d.slot = None;
                        releases.push(id);
                    }
                }
                if (frenzy || (vengeance && id == leader)) && frame.is_multiple_of(5) {
                    d.rest = d.rest.saturating_sub(1);
                }
                if let (true, Some(at)) = (banner, leader_at) {
                    let near = dist_sq(d.pos, at) <= pack::BANNER_RADIUS * pack::BANNER_RADIUS;
                    if near && d.f.regenerating() && d.f.reeling().is_none() {
                        d.f.poise = (d.f.poise + d.f.max_poise * pack::BANNER_REGEN).min(d.f.max_poise);
                    }
                }
            }
        }
        if let Some(b) = self.brawl.as_mut() {
            for id in releases {
                b.threat.release(id);
            }
        }
        for leader in rallies {
            let Some(i) = self.duel_at(leader) else { continue };
            let Some(d) = self.enemies[i].duel.as_deref_mut() else { continue };
            if d.begin(MoveId::Rally, hp) {
                let (kind, at) = (d.breed.kind, d.pos);
                self.war_note(WarEvent::Tell { foe: leader, kind, mv: MoveId::Rally, at });
                self.audio.play(Sfx::Toll);
            }
        }
    }

    /// A leader's rally sounds (its first active frame): its pack about it takes back a share of
    /// its poise, and heart.
    pub(super) fn rally(&mut self, i: usize) {
        let Some(d) = self.enemies[i].duel.as_deref() else { return };
        let (id, at) = (d.id, d.pos);
        let Some(k) = self.war.packs.iter().position(|p| p.leader == id) else { return };
        self.war.packs[k].rallied();
        self.war.tally.rallies += 1;
        let ids: Vec<u32> = self.war.packs[k].members.iter().filter(|m| m.alive).map(|m| m.id).collect();
        for m in ids {
            if let Some(o) = self.duel_at(m).and_then(|j| self.enemies[j].duel.as_deref_mut()) {
                if dist_sq(o.pos, at) <= pack::RALLY_RADIUS * pack::RALLY_RADIUS {
                    o.f.poise = (o.f.poise + o.f.max_poise * pack::RALLY_POISE).min(o.f.max_poise);
                }
            }
        }
        self.floater(at.x, at.y - 1.5, "RALLY!", 1.2, Some(0xffc040));
        self.view.sparkle(&self.level.grid, at.x, at.y, 1.4, 0xffc040, 24);
    }

    /// The share of a turned-back bolt that reaches duelist `id`: half for a caster near a Warding leader.
    pub(super) fn ward_share(&self, id: u32) -> f64 {
        let Some(at) = self.duel_at(id).and_then(|i| self.enemies[i].duel.as_deref()).map(|d| d.pos) else { return 1.0 };
        let warded = self.war.packs.iter().filter(|p| p.has_affix(Affix::Warding) && p.leader != id).any(|p| {
            p.members.iter().any(|m| m.id == id && m.role == Role::Caller)
                && self
                    .duel_at(p.leader)
                    .and_then(|i| self.enemies[i].duel.as_deref())
                    .is_some_and(|l| l.alive() && dist_sq(l.pos, at) <= pack::WARD_RADIUS * pack::WARD_RADIUS)
        });
        if warded {
            pack::WARD_SHARE
        } else {
            1.0
        }
    }

    /// Duelist `id` leads a Gilded pack: its spoils come twice over.
    pub(super) fn gilded_leader(&self, id: u32) -> bool {
        self.war.packs.iter().any(|p| p.leader == id && p.has_affix(Affix::Gilded))
    }

    /// Duelist `id` leads its pack (the HUD crowns it).
    pub fn leads_a_pack(&self, id: u32) -> bool {
        self.war.packs.iter().any(|p| p.leader == id)
    }
}
