//! The Burning Field in play (Wrath's realm, REALMS.md §3; laid out in `logic::realms::field`).
//!
//! Every tick the slabs drift, each band at its pace, and the grid follows them: a slab's tiles are
//! its floor at its band's height, and the gaps are lava, walkable so that a short leap lands in it.
//! The slab under him carries him (and every foe on a slab is carried by it); off every slab and
//! isle he is in the lava, which costs him a third of his health and puts him back on the last
//! ground he stood on. A foe walks no further than its slab's edge, but one knocked off it, or
//! whose poise breaks within a tile of an edge, topples in for good.
//!
//! Fury: every blow he lands adds a stack, each biting deeper and quickening him, until he stops
//! swinging for two seconds; at ten, Berserkergang locks his controls on the nearest foe for six
//! seconds, swinging, with no guard. Every blow, his or theirs, cracks the slab it lands on, and at
//! the third crack it splits and the smaller piece sinks. Lightning strikes every eight seconds or
//! so, and at each flash the slain rise where they fell, unless their bodies went into the lava.
//!
//! The horn on the herald's isle calls the whole field (the climax, `sound_horn`); the truce stone on
//! the hill's terrace opens the far portal once he has held five seconds without a swing near it.
//! F2 brings him to the horn.

use super::super::{Banner, Game};
use super::Meter;
use crate::audio::Sfx;
use crate::input::Action;
use crate::logic::combat::ails::{Flash, FlashKind};
use crate::logic::combat::duelist::RISE_FRAMES;
use crate::logic::combat::{math, Phase, Reel};
use crate::logic::enemy::{EnemyKind, EnemyMode};
use crate::logic::hub::Rect;
use crate::logic::realms::field::*;
use crate::logic::realms::{Pack, Run};
use crate::logic::types::{dist_sq, Dir, P, V};

/// The field's colours: Wrath's red, the lava's orange, the lightning's white.
const RED: u32 = 0xff3a2a;
const LAVA: u32 = 0xff7a20;
const WHITE: u32 = 0xf4f0ff;

/// Floats over pits and lava: the ravens, and the wrathful dead riding the storm.
fn floats(kind: EnemyKind) -> bool {
    kind.flies() || kind == EnemyKind::Goryo
}

impl Game {
    fn field(&self) -> Option<&Battlefield> {
        match self.run.as_ref() {
            Some(Run::Wrath(f)) => Some(f.as_ref()),
            _ => None,
        }
    }

    fn field_mut(&mut self) -> Option<&mut Battlefield> {
        match self.run.as_mut() {
            Some(Run::Wrath(f)) => Some(f.as_mut()),
            _ => None,
        }
    }

    /// As the realm is entered (its packs raised, the camera above): the far portal stays sealed
    /// until the truce stone opens it.
    pub(in crate::game) fn enter_field(&mut self) {
        self.field_mark();
    }

    /// The realm's own rules each tick.
    pub(in crate::game) fn update_field(&mut self, dt: f64) {
        if self.field().is_none() {
            return;
        }
        self.field_survey();
        self.field_drift(dt);
        self.field_foes();
        self.field_footing();
        if !self.hero.alive {
            return;
        }
        self.field_fury(dt);
        self.field_storm(dt);
        self.field_horn(dt);
        self.field_record();
    }

    /// The controls as the realm leaves them (`Game::steer_run`): his fury quickens him, and in
    /// Berserkergang it takes the controls, running him at the nearest foe and swinging, his guard
    /// and his leap gone.
    pub(in crate::game) fn steer_field(&mut self, wanted: Dir) -> Dir {
        let Some(f) = self.field() else { return wanted };
        let (fury, berserk) = (f.fury, f.berserk);
        self.hero.pace = 1.0 + FURY_SPEED * fury as f64;
        if berserk <= 0.0 || !self.hero.alive {
            return wanted;
        }
        self.input.inject_release(Action::Block);
        self.input.consume(Action::Jump);
        let hp = self.hero.pos();
        let Some(target) = self.nearest_foe(hp) else { return wanted };
        let to = math::sub(target, hp);
        let dist = math::len(to);
        self.hero.heading = math::angle(to);
        self.hero.facing = Dir::nearest(to);
        if dist <= 1.9 {
            self.input.inject(Action::Attack);
        }
        if dist > 1.1 {
            self.hero.wish = to;
            Dir::nearest(to)
        } else {
            self.hero.wish = V::new(0.0, 0.0);
            Dir::None
        }
    }

    /// The nearest foe standing (not one still climbing out of the ground, nor one out of reach of
    /// any blade).
    fn nearest_foe(&self, from: V) -> Option<V> {
        self.enemies
            .iter()
            .filter_map(|e| e.duel.as_deref())
            .filter(|d| d.alive() && !d.untouchable() && d.lore.rise == 0)
            .map(|d| d.pos)
            .filter(|p| dist_sq(*p, from) < 30.0 * 30.0)
            .min_by(|a, b| dist_sq(*a, from).total_cmp(&dist_sq(*b, from)))
    }

    /// After a fall, as he rises at the checkpoint (and as the realm is first entered): the field as
    /// it was laid out (its slabs whole, its moat sunk, its packs at their posts, no fury), but where
    /// he rises and the stone, if it has opened, kept.
    pub(in crate::game) fn reset_field(&mut self) {
        let hits = self.brawl.as_ref().map(|b| (b.stats.hits, b.stats.hurt + b.stats.blocks + b.stats.parries)).unwrap_or((0, 0));
        let Some(f) = self.field_mut() else { return };
        let mut fresh = f.restored();
        (fresh.seen_hits, fresh.seen_struck) = hits;
        let welcome = !fresh.welcomed;
        fresh.welcomed = true;
        let (packs, tier, opened) = (fresh.base.packs.clone(), fresh.base.tier, fresh.opened);
        *f = fresh;
        if let Some(mut b) = self.brawl.take() {
            self.enemies = self.raise_packs(&packs, tier, &mut b);
            self.brawl = Some(b);
        }
        self.refresh_foe_views();
        self.hero.pace = 1.0;
        self.gate_open |= opened;
        self.field_mark();
        if welcome {
            self.juice.banner = Some(Banner {
                text: "THE BURNING FIELD".into(),
                sub: "EVERY BLOW CRACKS THE GROUND   MICHAEL'S FIRE IS ITS BEST BLESSING: LET THEM FEUD".into(),
                color: RED,
                life: 4.5,
                max: 4.5,
            });
        }
    }

    /// F2: straight to the climax, before the horn on its gallows-tree, facing it.
    pub(in crate::game) fn cheat_field(&mut self) {
        let Some(f) = self.field_mut() else { return };
        let at = P::new(f.horn.x, f.horn.y + 1);
        f.base.checkpoint = f.rest;
        f.safe = Ground::Isle(at.v());
        f.on = None;
        self.hero.place(at);
        self.hero.face(Dir::Up);
        self.view.aim_along(Dir::Up);
        self.field_mark();
    }

    /// What the HUD shows of the realm's rules: his fury (Berserkergang's time while it runs), the
    /// causeway's time while he has yet to cross it, and the truce stone's count.
    pub(in crate::game) fn field_meters(&self) -> Vec<Meter> {
        let Some(f) = self.field() else { return Vec::new() };
        let mut m = Vec::new();
        if f.berserk > 0.0 {
            m.push(Meter::bar("BERSERKERGANG", f.berserk as f32, BERSERK_SECONDS as f32, RED).warning(true));
        } else {
            m.push(Meter::bar("FURY", f.fury as f32, FURY_MAX as f32, RED));
        }
        if f.called.is_some() && !f.crossed && !f.opened {
            let left = f.causeway_left();
            m.push(Meter::bar("CAUSEWAY", left as f32, CAUSEWAY_SECONDS as f32, LAVA).warning(left < 15.0));
        }
        if !f.opened && (f.truce > 0.0 || f.terrace.contains(self.hero.occupied())) {
            m.push(Meter::bar("TRUCE", f.truce as f32, TRUCE_SECONDS as f32, WHITE));
        }
        m
    }

    /// The slabs into the grid, the one he stands on first.
    fn field_mark(&mut self) {
        if let Some(Run::Wrath(f)) = self.run.as_mut() {
            let prefer = f.on;
            f.mark(&mut self.level.grid, prefer);
        }
    }

    /**
     * The fight since the last tick: his blows landed stack his fury; every blow, his or theirs,
     * cracks the slab it landed on; a foe struck while he is furious takes his fury's share on top;
     * and those slain lie where they fell (unless the lava took them).
     */
    fn field_survey(&mut self) {
        let Some(b) = self.brawl.as_ref() else { return };
        let (hits, struck) = (b.stats.hits, b.stats.hurt + b.stats.blocks + b.stats.parries);
        let Some(f) = self.field() else { return };
        let (new_hits, new_struck) = (hits.saturating_sub(f.seen_hits), struck.saturating_sub(f.seen_struck));
        let (stacks, weight) = (f.fury, if f.called.is_some() && !f.opened { 2 } else { 1 });
        let seen = f.seen.clone();
        let toppled = f.toppled.clone();
        let mut blows: Vec<V> = Vec::new();
        let mut slain: Vec<(EnemyKind, V)> = Vec::new();
        let mut felled: Vec<usize> = Vec::new();
        let mut bites: Vec<(V, f64)> = Vec::new();
        for s in &seen {
            let i = self.enemies.iter().position(|e| e.duel.as_ref().is_some_and(|d| d.id == s.id));
            let Some(d) = i.and_then(|i| self.enemies[i].duel.as_deref_mut()).filter(|d| d.alive()) else {
                if !toppled.contains(&s.id) {
                    slain.push((s.kind, s.pos));
                }
                continue;
            };
            if d.hp < s.hp - 1e-6 {
                blows.push(d.pos);
                let bonus = (s.hp - d.hp) * FURY_DAMAGE * stacks as f64;
                if bonus > 0.0 {
                    d.hp -= bonus;
                    let p = d.pos;
                    if d.hp <= 0.0 {
                        felled.push(i.unwrap_or(0));
                    }
                    if bonus >= 0.5 {
                        bites.push((p, bonus));
                    }
                }
            }
        }
        for (p, bonus) in bites {
            self.floater(p.x + 0.35, p.y - 1.0, format!("+{}", bonus.round()), 0.7, Some(RED));
        }
        for i in felled {
            if let Some(p) = self.fury_fells(i) {
                slain.push((self.enemies[i].kind(), p));
            }
        }
        let hp = self.hero.pos();
        let Some(f) = self.field_mut() else { return };
        (f.seen_hits, f.seen_struck) = (hits, struck);
        // The ground keeps score.
        let mut cracked: Vec<(V, bool)> = Vec::new();
        let on_him = (0..new_struck).map(|_| hp);
        for at in blows.into_iter().chain(on_him) {
            let Some((id, before)) = f.slab_under(at, 0.3, None).map(|s| (s.id, s.cracks())) else { continue };
            let split = f.strike(id, at, weight).is_some() || f.slab(id).is_some_and(|s| s.sinking.is_some());
            let after = f.slab(id).map(|s| s.cracks()).unwrap_or(0);
            if split {
                cracked.push((at, true));
            } else if after > before {
                cracked.push((at, false));
            }
        }
        // His fury.
        let mut rage = false;
        if new_hits > 0 {
            f.idle = 0.0;
            if f.berserk <= 0.0 {
                f.fury = (f.fury + new_hits).min(FURY_MAX);
                if f.fury >= FURY_MAX {
                    f.berserk = BERSERK_SECONDS;
                    rage = true;
                }
            }
        }
        // The slain lie where they fell, on the ground under them.
        let mut lost = 0;
        for (kind, p) in slain {
            match f.ground_at(p, 0.3, None) {
                Some(at) => f.corpses.push(Corpse { kind, at }),
                None => lost += 1,
            }
        }
        for (at, split) in cracked {
            if split {
                self.audio.play(Sfx::DoorGrind);
                self.view.shake(0.35);
                self.view.dust(&self.level.grid, at.x, at.y, 0x4a3426, 22, 2.4);
                self.floater(at.x, at.y - 0.9, "THE GROUND SPLITS", 1.3, Some(LAVA));
            } else {
                self.audio.play(Sfx::Clang);
                self.view.dust(&self.level.grid, at.x, at.y, 0x5a4434, 8, 1.2);
                self.floater(at.x, at.y - 0.6, "CRACK", 0.7, Some(0xffa060));
            }
        }
        if lost > 0 {
            self.floater(hp.x, hp.y - 1.6, "THE LAVA KEEPS ITS DEAD", 1.2, Some(LAVA));
        }
        if rage {
            self.audio.play(Sfx::Howl);
            self.audio.play(Sfx::BossRoar);
            self.view.shake(0.5);
            self.view.sparkle(&self.level.grid, hp.x, hp.y, 1.0, RED, 40);
            self.juice.banner = Some(Banner {
                text: "BERSERKERGANG".into(),
                sub: "WRATH TAKES THE CONTROLS   NO GUARD".into(),
                color: RED,
                life: 2.2,
                max: 2.2,
            });
        }
    }

    /// Foe `i` falls to his fury's share of a blow: slain as by the blow itself. Returns where.
    fn fury_fells(&mut self, i: usize) -> Option<V> {
        let d = self.enemies.get_mut(i)?.duel.as_deref_mut()?;
        if !d.alive() {
            return None;
        }
        d.hp = 0.0;
        d.f.kill();
        let p = d.pos;
        let color = self.enemies[i].b.color;
        self.enemies[i].b.mode = EnemyMode::Eaten;
        if let Some(b) = self.brawl.as_mut() {
            b.stats.kills += 1;
        }
        self.kill_spoils(p, crate::logic::systems::POINTS_KILL);
        self.view.dust(&self.level.grid, p.x, p.y, color, 18, 2.6);
        self.view.shake(0.18);
        Some(p)
    }

    /// The slabs drift (sinking, rising), the grid follows, and they carry whoever stands on them.
    fn field_drift(&mut self, dt: f64) {
        let riders = self.field_riders();
        let Some(f) = self.field_mut() else { return };
        let moved = f.drift(dt);
        f.rise(dt);
        f.sink(dt);
        let target = if f.called.is_some() && !f.opened { 1.0 } else { 0.0 };
        f.lava += (target - f.lava).clamp(-dt / LAVA_SECONDS, dt / LAVA_SECONDS);
        self.field_mark();
        self.field_carry(&riders, &moved);
    }

    /// Who stands on which slab now: the foes (by index), and him.
    fn field_riders(&self) -> Vec<(usize, u32)> {
        let Some(f) = self.field() else { return Vec::new() };
        self.enemies
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let d = e.duel.as_deref().filter(|d| d.alive())?;
                f.slab_under(d.pos, 0.3, None).map(|s| (i, s.id))
            })
            .collect()
    }

    /// The slabs that moved (`moved`, by id) carry him and the foes on them (`riders`), and the
    /// foes' dens with them.
    fn field_carry(&mut self, riders: &[(usize, u32)], moved: &[(u32, V)]) {
        let delta = |id: u32| moved.iter().find(|m| m.0 == id).map(|m| m.1);
        for &(i, id) in riders {
            let Some(d) = delta(id) else { continue };
            let Some(du) = self.enemies.get_mut(i).and_then(|e| e.duel.as_deref_mut()) else { continue };
            du.pos = math::add(du.pos, d);
            if let Some(den) = du.den.as_mut() {
                den.lo = math::add(den.lo, d);
                den.hi = math::add(den.hi, d);
                den.home = math::add(den.home, d);
            }
            let p = du.pos;
            self.enemies[i].m.tile = P::new(p.x.round() as i32, p.y.round() as i32);
        }
        let Some(f) = self.field() else { return };
        let Some((d, now)) = f.on.and_then(|id| Some((delta(id)?, f.slab(id)?.origin))) else { return };
        if self.hero.airborne() {
            return; // (in the air the field drifts on under him)
        }
        if self.hero.free_move {
            self.hero.shove(&self.level.grid, d);
        } else {
            // On the grid he goes with it a whole tile at a time.
            let was = math::sub(now, d);
            let step = P::new(now.x.round() as i32 - was.x.round() as i32, now.y.round() as i32 - was.y.round() as i32);
            if step != P::new(0, 0) {
                let o = self.hero.occupied();
                self.hero.place(P::new(o.x + step.x, o.y + step.y));
            }
        }
    }

    /// The foes keep to their ground: one that walked off an edge is held at it, one knocked off it
    /// (or left standing in the lava as its slab went under) topples in, and so does one whose poise
    /// breaks within a tile of an edge. The ravens and the goryō float over it all.
    fn field_foes(&mut self) {
        let Some(f) = self.field() else { return };
        let mut topple: Vec<usize> = Vec::new();
        let mut held: Vec<(usize, V)> = Vec::new();
        for (i, e) in self.enemies.iter().enumerate() {
            let Some(d) = e.duel.as_deref().filter(|d| d.alive()) else { continue };
            if floats(e.kind()) || d.lore.rise > 0 {
                continue;
            }
            let broken = matches!(d.f.reeling(), Some(Reel::Staggered | Reel::GuardBroken));
            if f.ground_at(d.pos, 0.25, None).is_some() {
                if broken && f.edge_near(d.pos, EDGE_TOPPLE) {
                    topple.push(i);
                }
                continue;
            }
            let was = f.seen.iter().find(|s| s.id == d.id).and_then(|s| s.ground).and_then(|g| f.ground_pos(g));
            match was {
                Some(p) if d.f.reeling().is_none() && math::len(d.knock) < 0.5 => held.push((i, p)),
                _ => topple.push(i),
            }
        }
        // One held at an edge he could leap to (it thinks), a band below him: it taunts him across.
        let hp = self.hero.pos();
        let below = |p: V| f.slab_under(p, 0.3, None).map(|s| s.elevation()).unwrap_or(0);
        let mine = f.on.and_then(|id| f.slab(id)).map(|s| s.elevation()).unwrap_or(0);
        let bait =
            (f.taunt_rest <= 0.0).then(|| held.iter().map(|&(_, p)| p).find(|&p| below(p) < mine && dist_sq(p, hp) < 7.0 * 7.0)).flatten();
        for (i, p) in held {
            if let Some(d) = self.enemies[i].duel.as_deref_mut() {
                d.pos = p;
            }
        }
        if let Some(p) = bait {
            const TAUNTS: [&str; 4] = ["COME DOWN AND FIGHT!", "ONE LEAP, COWARD!", "IT IS BUT A STEP!", "HERE, WARRIOR! HERE!"];
            let k = self.field().map(|f| f.taunts as usize).unwrap_or(0);
            if let Some(f) = self.field_mut() {
                f.taunt_rest = 5.0;
                f.taunts += 1;
            }
            self.floater(p.x, p.y - 1.3, TAUNTS[k % TAUNTS.len()], 1.6, Some(0xffc8a0));
            self.audio.play(Sfx::Growl);
        }
        if topple.is_empty() {
            return;
        }
        topple.sort_unstable();
        for &i in topple.iter().rev() {
            let e = self.enemies.remove(i);
            let Some(d) = e.duel.as_deref() else { continue };
            let p = d.pos;
            if let Some(b) = self.brawl.as_mut() {
                b.threat.release(d.id);
                b.stats.kills += 1;
            }
            if let Some(f) = self.field_mut() {
                f.toppled.push(d.id);
            }
            self.kill_spoils(p, crate::logic::systems::POINTS_KILL);
            self.audio.play(Sfx::Explode);
            self.view.sparkle(&self.level.grid, p.x, p.y, 0.2, LAVA, 26);
            self.view.dust(&self.level.grid, p.x, p.y, 0xff5a10, 16, 2.2);
            self.floater(p.x, p.y - 0.9, "INTO THE LAVA, FOR GOOD", 1.2, Some(LAVA));
        }
        self.refresh_foe_views();
    }

    /// Where he stands: a slab (the last safe spot on it kept), an isle (the herald's is a
    /// checkpoint; the terrace is the far side of the moat), or the lava.
    fn field_footing(&mut self) {
        if !self.hero.alive || self.hero.airborne() {
            return;
        }
        let p = self.hero.pos();
        let slack = if self.hero.free_move { SLACK } else { 0.5 };
        let Some(f) = self.field_mut() else { return };
        let on = f.on;
        match f.ground_at(p, slack, on) {
            Some(Ground::Slab(id, _)) => {
                f.on = Some(id);
                let safe = f.slab(id).and_then(|s| {
                    let lt = s.local_tile(p);
                    let inside = lt.x >= 1 && lt.y >= 1 && lt.x < s.w - 1 && lt.y < s.h - 1;
                    (s.sinking.is_none() && inside && s.open_at(lt)).then_some(V::new(lt.x as f64, lt.y as f64))
                });
                if let Some(l) = safe {
                    f.safe = Ground::Slab(id, l);
                }
            }
            Some(Ground::Isle(_)) => {
                f.on = None;
                let t = P::new(p.x.round() as i32, p.y.round() as i32);
                f.safe = Ground::Isle(t.v());
                let first = f.isle.contains(t) && f.base.checkpoint != f.rest;
                if first {
                    f.base.checkpoint = f.rest;
                }
                let crossed = f.terrace.contains(t) && f.called.is_some() && !f.crossed;
                if crossed {
                    f.crossed = true;
                }
                if first {
                    self.floater(p.x, p.y - 1.2, "THE HERALD'S ISLE: HE WILL RISE HERE", 1.6, Some(0xffd8a0));
                    self.audio.play(Sfx::Milestone);
                }
                if crossed {
                    self.floater(p.x, p.y - 1.2, "ACROSS: NOW STOP", 1.6, Some(WHITE));
                }
            }
            None => self.field_fall(),
        }
    }

    /// Into the lava: a third of his health, and back on the last safe ground (in the climax, the
    /// herald's isle).
    fn field_fall(&mut self) {
        let p = self.hero.pos();
        let loss = (self.hero.max_hp as f64 * LAVA_SHARE).ceil();
        self.audio.play(Sfx::Explode);
        self.audio.play(Sfx::Hurt);
        self.view.shake(0.45);
        self.view.sparkle(&self.level.grid, p.x, p.y, 0.3, LAVA, 40);
        self.view.dust(&self.level.grid, p.x, p.y, 0xff5a10, 24, 2.6);
        self.floater(p.x, p.y - 1.0, "INTO THE LAVA", 1.3, Some(LAVA));
        self.wound_hero(loss, LAVA);
        if !self.hero.alive {
            return;
        }
        let Some(f) = self.field_mut() else { return };
        // The lava quenches his rage, and his fury with it.
        let quenched = f.berserk > 0.0;
        (f.berserk, f.fury, f.fade) = (0.0, 0, 0.0);
        let mut back = if f.called.is_some() && !f.crossed && !f.opened { Ground::Isle(f.rest.v()) } else { f.safe };
        if let Ground::Slab(id, _) = back {
            // (His last spot went down with a piece of its slab: the nearest sound ground instead.)
            if f.slab(id).is_none_or(|s| s.sinking.is_some() || !s.solid()) {
                back = f.sound_ground_near(p).unwrap_or(Ground::Isle(f.base.checkpoint.v()));
            }
        }
        let (to, on) = match f.ground_pos(back) {
            Some(q) => (q, if let Ground::Slab(id, _) = back { Some(id) } else { None }),
            None => (f.base.checkpoint.v(), None),
        };
        f.on = on;
        if on.is_none() {
            f.safe = Ground::Isle(to);
        }
        self.field_mark();
        self.hero.place(P::new(to.x.round() as i32, to.y.round() as i32));
        self.hero.invuln = self.hero.invuln.max(1.2);
        self.hero.pace = 1.0;
        if quenched {
            self.floater(to.x, to.y - 1.2, "THE LAVA QUENCHES HIS RAGE", 1.4, Some(0xffb0a0));
        }
    }

    /// Fury holds while he swings and fades once he has stopped; Berserkergang runs out.
    fn field_fury(&mut self, dt: f64) {
        let swinging = self.brawl.as_ref().is_some_and(|b| b.hero.attack().is_some());
        let hp = self.hero.pos();
        let Some(f) = self.field_mut() else { return };
        f.taunt_rest = (f.taunt_rest - dt).max(0.0);
        f.idle = if swinging { 0.0 } else { f.idle + dt };
        let mut passed = false;
        if f.berserk > 0.0 {
            f.berserk -= dt;
            if f.berserk <= 0.0 {
                (f.berserk, f.fury, f.fade) = (0.0, 0, 0.0);
                passed = true;
            }
        } else if f.idle > FURY_HOLD && f.fury > 0 {
            f.fade += dt;
            while f.fade >= FURY_FADE && f.fury > 0 {
                f.fade -= FURY_FADE;
                f.fury -= 1;
            }
        } else {
            f.fade = 0.0;
        }
        let berserk = f.berserk > 0.0;
        if berserk {
            // (The swings it presses are its own, not a held button.)
            self.input.inject_release(Action::Attack);
        }
        if passed {
            self.hero.pace = 1.0;
            self.floater(hp.x, hp.y - 1.2, "THE RAGE PASSES", 1.2, Some(0xffb0a0));
        }
    }

    /// Lightning, about every eight seconds (four once the horn has called): a flash, and the
    /// slain rising where they fell.
    fn field_storm(&mut self, dt: f64) {
        let hp = self.hero.pos();
        let Some(f) = self.field_mut() else { return };
        f.clock += dt;
        f.flash += dt;
        f.next_flash -= dt;
        if f.next_flash > 0.0 {
            return;
        }
        f.flash = 0.0;
        f.flashes += 1;
        f.next_flash = f.flash_wait();
        f.bolt = f.bolt_near(hp);
        let risers = f.risers();
        let dens: Vec<(EnemyKind, V, Rect)> = risers.iter().map(|&(i, p)| (f.corpses[i].kind, p, f.den_at(p))).collect();
        f.corpses.clear();
        self.audio.play(Sfx::GabrielThunder);
        self.view.shake(0.18);
        if dens.is_empty() {
            return;
        }
        for (kind, p, den) in dens {
            self.field_raise(kind, p, den, dist_sq(p, hp) < 12.0 * 12.0);
        }
        self.refresh_foe_views();
        self.audio.play(Sfx::Soul);
        self.floater(hp.x, hp.y - 1.6, "THE SLAIN RISE", 1.4, Some(0x9ab0ff));
    }

    /// One of the slain rises at `at`, climbing out of the ground, its den the ground it rose on
    /// (awake at once if he is `near`).
    fn field_raise(&mut self, kind: EnemyKind, at: V, den: Rect, near: bool) {
        let Some(tier) = self.field().map(|f| f.base.tier) else { return };
        let post = P::new(at.x.round() as i32, at.y.round() as i32);
        let pack = Pack { den, foes: vec![(kind, kind.stage().max(1), post)] };
        let Some(mut b) = self.brawl.take() else { return };
        let mut raised = self.raise_packs(&[pack], tier, &mut b);
        b.flashes.push(Flash::new(FlashKind::Grave, at));
        self.brawl = Some(b);
        for e in raised.iter_mut() {
            if let Some(d) = e.duel.as_deref_mut() {
                d.pos = at;
                d.lore.rise = RISE_FRAMES;
                if let Some(den) = d.den.as_mut() {
                    den.home = at;
                    den.awake = near;
                }
            }
        }
        self.view.dust(&self.level.grid, at.x, at.y, 0x6a5a48, 14, 1.4);
        self.enemies.extend(raised);
    }

    /**
     * The herald speaks as he comes near; a swing at the horn sounds it; the horn's call runs on (the
     * causeway sinking under its time, the call failing if it goes under before he is across); and
     * the truce stone counts the seconds he holds without a swing near it, and opens the way.
     */
    fn field_horn(&mut self, dt: f64) {
        let hp = self.hero.pos();
        let attack = self.brawl.as_ref().and_then(|b| b.hero.attack());
        let Some(f) = self.field_mut() else { return };
        f.horn_age += dt;
        if f.opened {
            f.open_age += dt;
        }
        let herald = f.herald.v();
        let near_herald = dist_sq(hp, herald) < HERALD_RANGE * HERALD_RANGE;
        let speak = near_herald && !f.heard && f.called.is_none() && !f.opened;
        if speak {
            f.heard = true;
        } else if !near_herald && dist_sq(hp, herald) > (HERALD_RANGE + 4.0).powi(2) {
            f.heard = false;
        }
        let blow = f.called.is_none()
            && !f.opened
            && matches!(attack, Some((_, Phase::Active, _)))
            && dist_sq(hp, f.horn.v()) <= HORN_REACH * HORN_REACH;
        // The call runs on.
        let mut failed = false;
        if let Some(t) = f.called.as_mut() {
            *t += dt;
        }
        if f.called.is_some() && !f.opened {
            if let Some(c) = f.slabs.iter_mut().find(|s| s.kind == SlabKind::Moat(4)) {
                if c.risen >= 1.0 && c.sinking.is_none() {
                    c.sinking = Some(Sink::new(CAUSEWAY_SECONDS));
                }
            }
            if !f.crossed && f.causeway_left() <= 0.0 {
                f.recall();
                failed = true;
            }
        }
        // The truce stone.
        let stone = f.stone.v();
        let inside = !f.opened && dist_sq(hp, stone) <= TRUCE_RADIUS * TRUCE_RADIUS;
        let broke = inside && f.truce > 0.6 && attack.is_some();
        if inside && attack.is_none() {
            f.truce += dt;
        } else {
            f.truce = 0.0;
        }
        let open = !f.opened && f.truce >= TRUCE_SECONDS;
        if speak {
            self.audio.play(Sfx::Caw);
            self.floater(herald.x, herald.y - 2.2, "\"SOUND THE HORN AND THE BATTLE ENDS.\"", 3.2, Some(0xffe0c8));
            self.floater(herald.x, herald.y - 1.7, "THE HERALD OF AMON", 3.2, Some(RED));
        }
        if blow {
            self.sound_horn();
        }
        if failed {
            self.audio.play(Sfx::Denied);
            self.juice.banner = Some(Banner {
                text: "THE CAUSEWAY IS GONE".into(),
                sub: "THE HORN STILL HANGS ON THE GALLOWS-TREE".into(),
                color: LAVA,
                life: 3.0,
                max: 3.0,
            });
        }
        if broke {
            self.audio.play(Sfx::Denied);
            self.floater(stone.x, stone.y - 1.6, "THE TRUCE IS BROKEN", 1.2, Some(WHITE));
        }
        if open {
            self.open_truce();
        }
    }

    /// The horn sounds: the whole field answers. Every band lurches, the lava rises over the low
    /// shelf, the moat's five slabs rise (the causeway sinking from the moment it is up), every pack
    /// wakes, and the lightning strikes at once and every four seconds after.
    fn sound_horn(&mut self) {
        let riders = self.field_riders();
        let Some(f) = self.field_mut() else { return };
        f.call();
        f.base.checkpoint = f.rest;
        f.next_flash = 0.0;
        let moved = f.lurch();
        let herald = f.herald.v();
        self.field_mark();
        self.field_carry(&riders, &moved);
        for e in self.enemies.iter_mut() {
            if let Some(den) = e.duel.as_deref_mut().and_then(|d| d.den.as_mut()) {
                den.awake = true;
                den.returning = false;
            }
        }
        self.audio.play(Sfx::Toll);
        self.audio.play(Sfx::BossRoar);
        self.view.shake(0.8);
        self.floater(herald.x, herald.y - 2.2, "\"THE BATTLE NEVER ENDS.\"", 2.6, Some(0xffe0c8));
        self.juice.banner = Some(Banner {
            text: "THE HORN CALLS THE WHOLE FIELD".into(),
            sub: "THE CAUSEWAY SINKS: SIXTY SECONDS TO THE HILL".into(),
            color: RED,
            life: 3.5,
            max: 3.5,
        });
    }

    /// The truce stone opens the way: the far portal beside it unseals, and the field calms.
    fn open_truce(&mut self) {
        let Some(f) = self.field_mut() else { return };
        f.opened = true;
        f.open_age = 0.0;
        let stone = f.stone.v();
        self.gate_open = true;
        self.audio.play(Sfx::Win);
        self.audio.play(Sfx::Bells);
        self.view.sparkle(&self.level.grid, stone.x, stone.y, 1.4, WHITE, 60);
        self.juice.banner = Some(Banner {
            text: "THE TRUCE STONE OPENS THE WAY".into(),
            sub: "NO HORN ENDS A BATTLE: ONLY STOPPING DOES".into(),
            color: WHITE,
            life: 3.5,
            max: 3.5,
        });
    }

    /// What the next tick measures against: every foe standing, where and on what, and its health.
    fn field_record(&mut self) {
        let seen: Vec<(u32, EnemyKind, V, f64)> =
            self.enemies.iter().filter_map(|e| e.duel.as_deref().filter(|d| d.alive()).map(|d| (d.id, e.kind(), d.pos, d.hp))).collect();
        let Some(f) = self.field_mut() else { return };
        f.seen = seen.into_iter().map(|(id, kind, pos, hp)| Seen { id, kind, pos, hp, ground: f.ground_at(pos, 0.25, None) }).collect();
        f.toppled.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::realms::tests::{in_realm, STEP};
    use crate::game::Screen;
    use crate::logic::sins::Sin;

    fn field(g: &Game) -> &Battlefield {
        g.field().unwrap()
    }

    fn run(g: &mut Game, seconds: f64) {
        for _ in 0..(seconds / STEP) as usize {
            g.update(STEP);
        }
    }

    /// A game on the Burning Field with no foes, he moving freely.
    fn alone() -> Game {
        let mut g = in_realm(Sin::Wrath);
        g.enemies.clear();
        g.field_mut().unwrap().seen.clear();
        g.refresh_foe_views();
        g.hero.free_move = true;
        g
    }

    /// Puts him in the middle of a drifting slab of `band` (the first of the field's), and returns its id.
    fn onto(g: &mut Game, band: Band) -> u32 {
        let (id, at) = {
            let s = field(g).slabs.iter().find(|s| s.kind == SlabKind::Field && s.band == band).unwrap();
            let mut l = P::new(s.w / 2, s.h / 2);
            while s.blocked(l) {
                l.x += 1;
            }
            (s.id, s.tile_of(l))
        };
        g.field_mut().unwrap().on = Some(id);
        g.field_mark();
        g.hero.place(at);
        g.update(STEP);
        assert_eq!(field(g).on, Some(id), "on the slab");
        id
    }

    #[test]
    fn the_far_portal_stays_sealed_and_the_slabs_drift_under_him_and_carry_him() {
        let mut g = alone();
        assert!(!g.gate_open, "sealed until the truce stone opens it");
        let id = onto(&mut g, Band::Field);
        let local = |g: &Game| {
            let s = field(g).slab(id).unwrap();
            s.local(g.hero.pos())
        };
        let (start, at) = (field(&g).slab(id).unwrap().origin, local(&g));
        run(&mut g, 3.0);
        let s = field(&g).slab(id).unwrap();
        assert!(math::len(math::sub(s.origin, start)) > 0.3, "the slab drifted");
        assert!(math::len(math::sub(local(&g), at)) < 0.05, "and carried him with it: {:?} then {:?}", at, local(&g));
        assert_eq!(field(&g).on, Some(id));
        assert_eq!(g.hero.hp, g.hero.max_hp, "never in the lava");
    }

    #[test]
    fn the_lava_burns_a_third_of_his_health_and_puts_him_back_on_his_last_slab() {
        let mut g = alone();
        let id = onto(&mut g, Band::Field);
        let full = g.hero.hp;
        // Off its edge, into the lava between the lanes.
        let s = field(&g).slab(id).unwrap().clone();
        let off = (1..4).map(|k| s.tile_of(P::new(s.w / 2, -k))).find(|&t| field(&g).lava_at(t)).expect("lava north of it");
        g.hero.place(off);
        g.update(STEP);
        assert_eq!(g.hero.hp, full - (full as f64 / 3.0).ceil() as i32, "a third of his health");
        assert!(g.hero.alive);
        let s = field(&g).slab(id).unwrap();
        assert!(s.contains(g.hero.pos(), 0.0), "back on the slab he stood on");
        assert!(g.hero.invulnerable());
        // Three falls, and the lava has him.
        for _ in 0..3 {
            g.hero.invuln = 0.0;
            let s = field(&g).slab(id).unwrap().clone();
            let off = (1..4).map(|k| s.tile_of(P::new(s.w / 2, -k))).find(|&t| field(&g).lava_at(t)).expect("lava north of it");
            g.hero.place(off);
            g.update(STEP);
        }
        assert_eq!(g.screen, Screen::Dying);
    }

    #[test]
    fn fury_stacks_with_his_blows_quickens_him_and_berserkergang_takes_his_controls() {
        let mut g = in_realm(Sin::Wrath);
        g.hero.free_move = true;
        // Nine blows landed: nine stacks, and he is quicker.
        g.brawl.as_mut().unwrap().stats.hits += 9;
        g.update(STEP);
        g.update(STEP);
        assert_eq!(field(&g).fury, 9);
        assert!((g.hero.pace - (1.0 + 9.0 * FURY_SPEED)).abs() < 1e-9, "{}", g.hero.pace);
        // It holds two seconds without a swing, then fades.
        run(&mut g, 1.5);
        assert_eq!(field(&g).fury, 9);
        run(&mut g, 1.2);
        assert!(field(&g).fury < 9);
        // The tenth: Berserkergang. His controls run him at the nearest foe, swinging, with no guard.
        g.field_mut().unwrap().fury = 9;
        g.brawl.as_mut().unwrap().stats.hits += 1;
        g.update(STEP);
        assert!(field(&g).berserk > 0.0, "berserk");
        let hp = g.hero.pos();
        let foe = g.nearest_foe(hp).expect("a foe on the field");
        g.input.inject(Action::Block);
        g.hero.wish = V::new(-1.0, 0.0);
        let wanted = g.steer_field(Dir::Left);
        let to = math::sub(foe, hp);
        assert_eq!(wanted, Dir::nearest(to), "steered at the foe");
        assert!(math::dot(math::norm(g.hero.wish), math::norm(to)) > 0.99, "his wish is the rage's");
        assert!(!g.input.held(Action::Block), "no guard");
        // Six seconds, and the rage passes, spending his fury.
        g.enemies.clear();
        g.field_mut().unwrap().seen.clear();
        g.refresh_foe_views();
        run(&mut g, BERSERK_SECONDS + 0.2);
        assert_eq!((field(&g).berserk, field(&g).fury), (0.0, 0));
    }

    #[test]
    fn blows_crack_the_slab_they_land_on_and_split_it() {
        let mut g = alone();
        let id = onto(&mut g, Band::Ridge);
        let w = field(&g).slab(id).unwrap().w;
        // Blows on him: every one cracks the slab under him.
        for k in 1..=STRAIN_PER_CRACK * CRACKS_TO_SPLIT {
            g.hero.invuln = 1.0;
            g.brawl.as_mut().unwrap().stats.hurt += 1;
            g.update(STEP);
            if k < STRAIN_PER_CRACK * CRACKS_TO_SPLIT {
                assert_eq!(field(&g).slab(id).unwrap().cracks(), k / STRAIN_PER_CRACK);
            }
        }
        let pieces: Vec<&Slab> =
            field(&g).slabs.iter().filter(|s| s.home.y == field(&g).slab(id).unwrap().home.y && s.band == Band::Ridge).collect();
        assert!(pieces.iter().any(|s| s.sinking.is_some()), "a piece sinks");
        let a = field(&g).slab(id).unwrap();
        assert!(a.w < w, "the slab is smaller");
    }

    #[test]
    fn the_slain_rise_at_the_next_flash_unless_the_lava_took_them() {
        let mut g = in_realm(Sin::Wrath);
        g.hero.free_move = true;
        run(&mut g, 0.2);
        // Two foes slain: one where it stands, one over the lava.
        let n = g.enemies.len();
        let (a, b) = (0, 1);
        let lava = (5..HEIGHT).flat_map(|y| (0..WIDTH).map(move |x| P::new(x, y))).find(|&t| field(&g).lava_at(t)).unwrap();
        let ka = g.enemies[a].kind();
        let pa = g.enemies[a].duel.as_ref().unwrap().pos;
        let idb = g.enemies[b].duel.as_ref().unwrap().id;
        g.field_mut().unwrap().seen.iter_mut().find(|s| s.id == idb).unwrap().pos = lava.v();
        for i in [b, a] {
            g.enemies.remove(i);
        }
        g.refresh_foe_views();
        g.update(STEP);
        assert_eq!(field(&g).corpses.len(), 1, "one body lies on the field; the lava kept the other");
        assert_eq!(g.enemies.len(), n - 2);
        // The flash: it rises where it fell.
        g.field_mut().unwrap().next_flash = STEP / 2.0;
        g.update(STEP);
        assert_eq!(g.enemies.len(), n - 1);
        let risen = g.enemies.last().unwrap();
        assert_eq!(risen.kind(), ka);
        let d = risen.duel.as_ref().unwrap();
        assert!(math::len(math::sub(d.pos, pa)) < 1.0, "where it fell (as its slab carried it)");
        assert!(d.lore.rise > 0, "climbing out of the ground");
        assert!(field(&g).corpses.is_empty());
    }

    #[test]
    fn a_foe_whose_poise_breaks_at_an_edge_topples_into_the_lava_for_good() {
        let mut g = in_realm(Sin::Wrath);
        run(&mut g, 0.1);
        let i = g.enemies.iter().position(|e| !floats(e.kind())).unwrap();
        let id = g.enemies[i].duel.as_ref().unwrap().id;
        // At the very edge of its slab, its poise broken.
        let p = g.enemies[i].duel.as_ref().unwrap().pos;
        let s = field(&g).slab_under(p, 0.3, None).unwrap().clone();
        let edge = s.world(V::new(0.0, s.local(p).y));
        let d = g.enemies[i].duel.as_deref_mut().unwrap();
        d.pos = edge;
        d.f.state = crate::logic::combat::State::Reel { kind: Reel::Staggered, left: 30, total: 30 };
        let kills = g.brawl.as_ref().unwrap().stats.kills;
        g.update(STEP);
        assert!(g.enemies.iter().all(|e| e.duel.as_ref().is_none_or(|d| d.id != id)), "gone into the lava");
        assert_eq!(g.brawl.as_ref().unwrap().stats.kills, kills + 1);
        g.update(STEP);
        assert!(field(&g).corpses.is_empty(), "and no body to rise");
    }

    #[test]
    fn the_horn_calls_the_field_and_the_truce_stone_opens_only_after_five_seconds_without_a_swing() {
        let mut g = alone();
        g.input.key_down(winit::keyboard::KeyCode::F2, false);
        g.update(STEP);
        let f = field(&g);
        assert_eq!(g.hero.occupied(), P::new(f.horn.x, f.horn.y + 1), "F2: before the horn");
        assert!(f.called.is_none());
        let shelf: Vec<u32> = f.slabs.iter().filter(|s| s.band == Band::Shelf).map(|s| s.id).collect();
        // The herald speaks; a swing at the horn sounds it.
        g.update(STEP);
        assert!(g.floaters.iter().any(|fl| fl.text.contains("SOUND THE HORN")));
        g.input.inject(Action::Attack);
        run(&mut g, 0.5);
        let f = field(&g);
        assert!(f.called.is_some(), "the horn calls the field");
        assert_eq!(f.base.checkpoint, f.rest, "he rises on the herald's isle");
        assert!(f.flashes >= 1, "the lightning strikes at once");
        run(&mut g, RISE_SECONDS + 0.5);
        let f = field(&g);
        assert!(f.slabs.iter().filter(|s| matches!(s.kind, SlabKind::Moat(_))).all(|s| s.solid()), "the moat's five rise");
        assert!(f.causeway().unwrap().sinking.is_some(), "and the causeway sinks");
        assert!(f.lava > 0.9, "the lava rises");
        assert!(shelf.iter().all(|&id| f.slab(id).is_none_or(|s| !s.solid())), "and drowns the low shelf");
        assert!(f.next_flash <= CALLED_FLASH_EVERY + 1e-9);
        let left = f.causeway_left();
        assert!(left < CAUSEWAY_SECONDS && left > CAUSEWAY_SECONDS - 3.0);
        // Across, to the stone: five seconds without a swing open it, and any swing starts again.
        let stone = field(&g).stone;
        g.hero.place(P::new(stone.x, stone.y + 2));
        run(&mut g, 3.0);
        assert!(field(&g).truce > 2.5 && !g.gate_open);
        g.hero.invuln = 1.0;
        g.input.inject(Action::Attack);
        run(&mut g, 0.3);
        assert!(field(&g).truce < 0.3, "a swing breaks the truce: {}", field(&g).truce);
        run(&mut g, 2.0);
        assert!(!g.gate_open);
        run(&mut g, 3.5);
        assert!(field(&g).opened && g.gate_open, "the stone opens the far portal");
    }

    #[test]
    fn the_hill_cannot_be_reached_without_the_horn() {
        let g = alone();
        let f = field(&g);
        // The moat is all lava and wider than a leap.
        for y in f.terrace.y1 + 1..f.isle.y0 {
            assert!(f.lava_at(P::new(30, y)), "lava at row {y}");
        }
        assert!(f.isle.y0 - f.terrace.y1 - 1 > 2);
    }

    #[test]
    fn a_fall_restores_the_field_but_keeps_his_checkpoint() {
        let mut g = alone();
        g.input.key_down(winit::keyboard::KeyCode::F2, false);
        g.update(STEP);
        g.input.inject(Action::Attack);
        run(&mut g, 0.5);
        assert!(field(&g).called.is_some());
        g.hero.hp = 1;
        g.wound_hero(5.0, 0xffffff);
        run(&mut g, 3.0);
        let f = field(&g);
        assert!(f.called.is_none() && f.lava == 0.0, "the field is as it was laid out");
        assert_eq!(f.base.checkpoint, f.rest);
        assert!(f.isle.contains(g.hero.occupied()), "he rises on the herald's isle");
        assert!(!g.enemies.is_empty(), "the packs back at their posts");
    }
}
