//! The Rotting Feast in play (Gluttony's realm, REALMS.md §5; laid out in `logic::realms::feast`).
//!
//! Here he starves: every few seconds hunger takes a point of his maximum health, down to 40% of
//! it. Food lies everywhere and walking over it eats it: a bite gives back maximum health, heals
//! him and adds a stack of Bloat. Bloat slows his walk, lengthens his leap's landing, wakes the gaki
//! (from 3), bars the narrow arches to him (from 6) and breaks the Kitchens' crust floors under him
//! (from 8), dropping him into the Stomach. Every L3 that lands burns a stack; a purge font takes
//! five, and twenty health, and stuns him.
//!
//! The seams between the zones close behind him once he is through, the zone behind digested and
//! its foes with it; he rises at the mouth of the zone he has reached. The floors drift him toward
//! the stomach, faster in each zone.
//!
//! The Last Course: at the Stomach's heart a single place is set under a silver dome. Walking up to
//! it lifts the dome on an empty plate, and the Stomach convulses: the walls close in on the
//! pylorus over 45 seconds (caught by them, he is digested and wakes in the Hall, hungry: the realm
//! begins again), acid floods in from the rim, the swarm hunts him as a cloud, and the gut hurls
//! food at him (each hit a stack of Bloat; parried, it flies into the gaki instead). The pylorus's
//! teeth part for two seconds in every ten, and only for a hero of 3 Bloat or less.
//!
//! On a fall the food he ate stays eaten and the zones behind stay digested; his Bloat and hunger
//! go with the life he lost, and a Last Course under way is called off (the dome set again).

use super::super::save::Stage;
use super::super::{Banner, Game, Screen};
use super::Meter;
use crate::audio::Sfx;
use crate::logic::combat::fsm::EVADE_AIR;
use crate::logic::combat::{MoveId, Reel, State};
use crate::logic::enemy::{EnemyKind, EnemyMode};
use crate::logic::hub::Rect;
use crate::logic::realms::feast::*;
use crate::logic::realms::{Pack, Run};
use crate::logic::sins::Sin;
use crate::logic::types::{dist_sq, Dir, Tile, P, V};

/// The realm's colours on the HUD and in its floaters.
const ORANGE: u32 = 0xff8a2a;
const ACID: u32 = 0xb8e04a;
const PALE: u32 = 0xe8d8b0;
const SWARM: u32 = 0x8a7a6a;
/// How near the dome he must come to lift it (tiles from its table's centre).
const DOME_REACH: f64 = 1.6;
/// The swarm's pace in the Last Course: tiles a second, and what each second of it adds.
const SWARM_PACE: (f64, f64) = (1.2, 0.02);

fn len(v: V) -> f64 {
    (v.x * v.x + v.y * v.y).sqrt()
}

fn toward(from: V, to: V) -> V {
    let d = V::new(to.x - from.x, to.y - from.y);
    let l = len(d).max(1e-6);
    V::new(d.x / l, d.y / l)
}

impl Game {
    fn feast(&self) -> Option<&Feast> {
        match self.run.as_ref()? {
            Run::Gluttony(f) => Some(f),
            _ => None,
        }
    }

    fn feast_mut(&mut self) -> Option<&mut Feast> {
        match self.run.as_mut()? {
            Run::Gluttony(f) => Some(f),
            _ => None,
        }
    }

    /// As the realm is entered (its packs raised, the camera above, the pylorus sealed).
    pub(in crate::game) fn enter_feast(&mut self) {}

    /// The realm's own rules each tick.
    pub(in crate::game) fn update_feast(&mut self, dt: f64) {
        if self.feast().is_none() || !self.hero.alive {
            return;
        }
        self.feast_hunger(dt);
        self.feast_bloat(dt);
        self.feast_eat();
        self.feast_font();
        self.feast_crust();
        self.feast_burn();
        self.feast_bar();
        self.feast_seams(dt);
        self.feast_drift(dt);
        self.feast_smell();
        self.feast_dome();
        if self.screen == Screen::Playing && self.hero.alive {
            self.feast_climax(dt);
        }
    }

    /// The controls as the realm leaves them (`Game::steer_run`): untouched (a purge's stun holds
    /// him through his pace, and keeps him from swinging through his fighter).
    pub(in crate::game) fn steer_feast(&mut self, wanted: Dir) -> Dir {
        wanted
    }

    /// After a fall, as he rises at the checkpoint (and as the realm is first entered): a fresh
    /// body (no Bloat, no hunger), the dome set again and the pylorus shut; the food he ate stays
    /// eaten, and the seams behind him stay closed.
    pub(in crate::game) fn reset_feast(&mut self) {
        let hits = self.brawl.as_ref().map(|b| b.stats.hits).unwrap_or(0);
        let Some(f) = self.feast_mut() else { return };
        f.bloat = 0;
        f.hunger_clock = 0.0;
        f.stun = 0.0;
        f.climax = None;
        f.lifted = false;
        f.barred = false;
        f.hint = 0.0;
        f.hits_seen = hits;
        f.l3_burned = false;
        let narrow: Vec<P> = f.seams.iter().filter(|s| !s.closed).flat_map(|s| s.narrow.tiles.clone()).collect();
        let exit = f.base.exit.tile;
        for p in narrow {
            self.level.grid.set(p.x, p.y, Tile::Open);
        }
        self.level.grid.set(exit.x, exit.y, Tile::Rock);
        self.gate_open = false;
        self.door = 0.0;
        self.door_opening = false;
        self.hero.hunger = 0;
        self.hero.pace = 1.0;
        self.hero.max_hp = self.hero.health_cap();
        self.hero.hp = self.hero.max_hp;
    }

    /// F2: straight to the Last Course: every seam behind him, the dome lifted before him.
    pub(in crate::game) fn cheat_feast(&mut self) {
        let Some(f) = self.feast() else { return };
        let open: Vec<usize> = (0..f.seams.len()).filter(|&i| !f.seams[i].closed).collect();
        let dome = f.dome;
        for i in open {
            self.close_seam(i);
        }
        self.hero.place(P::new(dome.x, dome.y + 2));
        self.hero.face(Dir::Up);
        self.view.aim_along(Dir::Up);
        self.spring_climax();
    }

    /// What the HUD shows of the realm's rules: his Bloat and his hunger, and in the Last Course the
    /// pylorus's teeth and the walls.
    pub(in crate::game) fn feast_meters(&self) -> Vec<Meter> {
        let Some(f) = self.feast() else { return Vec::new() };
        let full = self.hero.health_cap() + self.hero.hunger;
        let limit = Feast::hunger_limit(full).max(1);
        let mut m = vec![
            Meter::bar("BLOAT", f.bloat as f32, MAX_BLOAT as f32, ORANGE).warning(f.bloat >= BLOAT_WIDE),
            Meter::bar("HUNGER", self.hero.hunger as f32, limit as f32, PALE).warning(self.hero.hunger >= limit),
        ];
        if let Some(c) = &f.climax {
            if c.open() {
                let left = (PYLORUS_PERIOD - c.t.rem_euclid(PYLORUS_PERIOD)) as f32;
                let label = if f.bloat <= BLOAT_LEAN { "THE TEETH PART" } else { "TOO FULL TO PASS" };
                m.push(Meter::bar(label, left, PYLORUS_OPEN as f32, ORANGE).warning(true));
            } else {
                m.push(Meter::count("TEETH PART IN", c.until_open().ceil() as u32, ORANGE));
            }
            m.push(Meter::count("THE WALLS CLOSE IN", (CLIMAX_SECONDS - c.t).max(0.0).ceil() as u32, 0xff5a4a).warning(c.t > 35.0));
        }
        m
    }

    // ------------------------------------------------------------------ the rules

    /// Hunger takes a point of his maximum health every few seconds, down to 40% of it.
    fn feast_hunger(&mut self, dt: f64) {
        let full = self.hero.health_cap() + self.hero.hunger;
        let limit = Feast::hunger_limit(full);
        let Some(f) = self.feast_mut() else { return };
        f.hunger_clock += dt;
        let mut pangs = 0;
        while f.hunger_clock >= HUNGER_EVERY {
            f.hunger_clock -= HUNGER_EVERY;
            pangs += 1;
        }
        if pangs > 0 {
            self.hero.hunger = (self.hero.hunger + pangs).min(limit.max(self.hero.hunger));
            self.hero.max_hp = self.hero.health_cap();
            self.hero.hp = self.hero.hp.min(self.hero.max_hp);
        }
    }

    /// What his Bloat costs him each tick: his walk, and a frame of his leap's landing for every
    /// stack; and the purge's stun wearing off (it holds him where he stands while it lasts).
    fn feast_bloat(&mut self, dt: f64) {
        let Some(f) = self.feast_mut() else { return };
        f.stun = (f.stun - dt).max(0.0);
        f.hint = (f.hint - dt).max(0.0);
        let (bloat, stunned) = (f.bloat, f.stun > 0.0);
        self.hero.pace = if stunned { 0.0 } else { bloat_pace(bloat) };
        if let Some(b) = self.brawl.as_mut() {
            b.hero.evade_total = EVADE_AIR + b.kit.evade_land + bloat as u16;
        }
    }

    /// Food underfoot is eaten: maximum health given back, health healed, a stack of Bloat.
    fn feast_eat(&mut self) {
        if self.hero.airborne() {
            return;
        }
        let o = self.hero.occupied();
        let Some(f) = self.feast_mut() else { return };
        let Some(i) = f.food.iter().position(|k| !k.eaten && k.at == o) else { return };
        f.food[i].eaten = true;
        let kind = f.food[i].kind;
        let before = f.bloat;
        f.bloat = (f.bloat + 1).min(MAX_BLOAT);
        let bloat = f.bloat;
        self.hero.hunger = (self.hero.hunger - BITE_RESTORE).max(0);
        self.hero.max_hp = self.hero.health_cap();
        self.hero.heal(BITE_HEAL);
        self.audio.play(Sfx::Eat);
        let (x, y) = (o.x as f64, o.y as f64);
        self.view.sparkle(&self.level.grid, x, y, 0.5, ORANGE, 10);
        self.floater(x, y - 0.5, format!("{}  +{}   BLOAT {}", kind.name(), BITE_HEAL, bloat), 1.2, Some(ORANGE));
        let warn = |at: u32| before < at && bloat >= at;
        let line = if warn(BLOAT_CRUST) {
            Some("TOO HEAVY FOR THE CRUST FLOORS")
        } else if warn(BLOAT_WIDE) {
            Some("TOO FULL FOR THE NARROW ARCHES")
        } else if warn(BLOAT_SMELL) {
            Some("THE GAKI SMELL THE FEAST ON HIM")
        } else {
            None
        };
        if let Some(line) = line {
            self.audio.play(Sfx::Stinger);
            self.floater(x, y - 1.2, line, 1.8, Some(0xff5a3a));
        }
    }

    /// A purge font: five stacks of Bloat and twenty health gone, and he stands stunned.
    fn feast_font(&mut self) {
        if self.hero.airborne() {
            return;
        }
        let o = self.hero.occupied();
        let Some(f) = self.feast_mut() else { return };
        let Some(i) = f.fonts.iter().position(|k| !k.used && k.at == o) else { return };
        if f.bloat == 0 {
            return;
        }
        f.fonts[i].used = true;
        let took = f.bloat.min(PURGE_BLOAT);
        f.bloat -= took;
        f.stun = PURGE_STUN;
        let frames = (PURGE_STUN * 60.0) as u16;
        if let Some(b) = self.brawl.as_mut() {
            b.hero.state = State::Reel { kind: Reel::Hitstun, left: frames, total: frames };
        }
        let (x, y) = (o.x as f64, o.y as f64);
        self.audio.play(Sfx::Drip);
        self.audio.play(Sfx::Hurt);
        self.view.dust(&self.level.grid, x, y, 0x9ac84a, 24, 1.6);
        self.view.shake(0.3);
        self.floater(x, y - 1.0, format!("PURGED  -{took} BLOAT"), 1.6, Some(ACID));
        self.wound_hero(PURGE_HEALTH, ACID);
    }

    /// From 8 Bloat the Kitchens' crust gives way under him, and he drops into the Stomach.
    fn feast_crust(&mut self) {
        if self.hero.airborne() {
            return;
        }
        let o = self.hero.occupied();
        let Some(f) = self.feast_mut() else { return };
        if f.bloat < BLOAT_CRUST || f.crust_at(o).is_none() {
            return;
        }
        let mut broke = Vec::new();
        for k in f.crust.iter_mut().filter(|k| !k.broken && (k.at.x - o.x).abs() <= 1 && (k.at.y - o.y).abs() <= 1) {
            k.broken = true;
            broke.push(k.at);
        }
        let skipped: Vec<usize> = (0..f.seams.len()).filter(|&i| !f.seams[i].closed).collect();
        let landing = f.stomach_landing();
        for p in broke {
            self.level.grid.set(p.x, p.y, Tile::Pit);
        }
        self.audio.play(Sfx::DoorGrind);
        self.audio.play(Sfx::Explode);
        self.view.dust(&self.level.grid, o.x as f64, o.y as f64, 0xd8a050, 30, 2.2);
        for i in skipped {
            self.close_seam(i);
        }
        self.hero.place(landing);
        self.hero.altitude = 0.0;
        self.view.shake(0.7);
        self.juice.banner = Some(Banner {
            text: "THE CRUST GIVES WAY".into(),
            sub: "TOO HEAVY: STRAIGHT DOWN INTO THE STOMACH".into(),
            color: ORANGE,
            life: 3.0,
            max: 3.0,
        });
        self.wound_hero(ACID_DAMAGE * 3.0, ACID);
    }

    /// Every L3 that lands burns a stack of Bloat (one for the swing, however many it strikes).
    fn feast_burn(&mut self) {
        let Some(b) = self.brawl.as_ref() else { return };
        let hits = b.stats.hits;
        let in_l3 = matches!(b.hero.state, State::Attack { mv: MoveId::L3, .. });
        let Some(f) = self.feast_mut() else { return };
        let landed = hits > f.hits_seen;
        f.hits_seen = hits;
        if !in_l3 {
            f.l3_burned = false;
            return;
        }
        if landed && !f.l3_burned && f.bloat > 0 {
            f.l3_burned = true;
            f.bloat -= 1;
            let hp = self.hero.pos();
            self.floater(hp.x, hp.y - 1.0, "-1 BLOAT", 0.9, Some(ORANGE));
        }
    }

    /// Too full for the narrow arches: from 6 Bloat they are shut to him (never while he is in one).
    fn feast_bar(&mut self) {
        let o = self.hero.occupied();
        let hp = self.hero.pos();
        let Some(f) = self.feast_mut() else { return };
        let want = f.bloat >= BLOAT_WIDE;
        let mut set: Vec<(P, Tile)> = Vec::new();
        let mut near = false;
        for s in f.seams.iter().filter(|s| !s.closed) {
            if want && s.narrow.contains(o) {
                continue;
            }
            let t = if want { Tile::Rock } else { Tile::Open };
            set.extend(s.narrow.tiles.iter().map(|&p| (p, t)));
            near |= s.narrow.south.iter().any(|m| dist_sq(m.v(), hp) < 2.5 * 2.5);
        }
        f.barred = want;
        let hint = want && near && f.hint <= 0.0;
        if hint {
            f.hint = 3.0;
        }
        for (p, t) in set {
            self.level.grid.set(p.x, p.y, t);
        }
        if hint {
            self.audio.play(Sfx::Denied);
            self.floater(hp.x, hp.y - 1.0, "TOO FULL TO FIT: TAKE THE WIDE ARCH", 1.6, Some(ORANGE));
        }
    }

    /// Through a seam: its teeth close behind him once he is past the hand-off.
    fn feast_seams(&mut self, dt: f64) {
        let hy = self.hero.pos().y;
        let Some(f) = self.feast_mut() else { return };
        for s in f.seams.iter_mut().filter(|s| s.closed) {
            s.since += dt;
        }
        let past: Vec<usize> =
            (0..f.seams.len()).filter(|&i| !f.seams[i].closed && hy < (f.seams[i].top - HANDOFF - 1) as f64 - 0.5).collect();
        for i in past {
            self.close_seam(i);
        }
    }

    /// Seam `i` closes: its arches become rock, the zone behind is digested and its foes with it,
    /// and he will rise past it from now on.
    fn close_seam(&mut self, i: usize) {
        let Some(f) = self.feast_mut() else { return };
        let Some(s) = f.seams.get_mut(i) else { return };
        if s.closed {
            return;
        }
        s.closed = true;
        s.since = 0.0;
        let tiles: Vec<P> = s.narrow.tiles.iter().chain(&s.wide.tiles).copied().collect();
        let (top, landing, into) = (s.top, s.landing, s.from.next());
        f.base.checkpoint = landing;
        let deeper = into > f.zone;
        f.zone = f.zone.max(into);
        for p in &tiles {
            self.level.grid.set(p.x, p.y, Tile::Rock);
        }
        // The zone behind is digested, and whatever stayed in it.
        let mut gone = Vec::new();
        for e in self.enemies.iter_mut() {
            let Some(d) = e.duel.as_deref_mut() else { continue };
            if d.pos.y.round() as i32 >= top {
                d.f.kill();
                e.b.mode = EnemyMode::Eaten;
                gone.push(d.id);
            }
        }
        if let Some(b) = self.brawl.as_mut() {
            for id in gone {
                b.threat.release(id);
            }
        }
        self.audio.play(Sfx::DoorGrind);
        self.audio.play(Sfx::BossSlam);
        self.view.shake(0.35);
        let hp = self.hero.pos();
        self.floater(hp.x, hp.y + 1.2, "THE TEETH CLOSE BEHIND HIM", 1.6, Some(0xffd0b0));
        if deeper {
            self.juice.banner = Some(if into == Zone::Stomach {
                Banner {
                    text: "\"FINISH YOUR PLATE, AND YOU MAY LEAVE.\"".into(),
                    sub: "A SINGLE PLACE IS SET AT THE STOMACH'S HEART".into(),
                    color: ORANGE,
                    life: 4.0,
                    max: 4.0,
                }
            } else {
                Banner { text: into.name().into(), sub: "THE WAY BEHIND IS DIGESTED".into(), color: ORANGE, life: 2.6, max: 2.6 }
            });
        }
    }

    /// The floor carries him toward the stomach, faster in each zone, and the hunting foes with him.
    fn feast_drift(&mut self, dt: f64) {
        let Some(f) = self.feast() else { return };
        let rate = f.drift_at(self.hero.pos().y);
        let rates: Vec<f64> = self.enemies.iter().map(|e| e.duel.as_ref().map(|d| f.drift_at(d.pos.y)).unwrap_or(0.0)).collect();
        if rate > 0.0 && !self.hero.airborne() {
            self.hero.shove(&self.level.grid, V::new(0.0, -rate * dt));
        }
        let grid = &self.level.grid;
        for (e, rate) in self.enemies.iter_mut().zip(rates) {
            let Some(d) = e.duel.as_deref_mut() else { continue };
            if rate <= 0.0 || !d.alive() || !d.den.is_some_and(|den| den.awake) {
                continue;
            }
            let to = V::new(d.pos.x, d.pos.y - rate * dt);
            let lead = (to.y - d.radius() * 0.9).round() as i32;
            if grid.is_walkable(to.x.round() as i32, lead, false) {
                d.pos = to;
            }
        }
    }

    /// From 3 Bloat the gaki smell the feast on him and come.
    fn feast_smell(&mut self) {
        let Some(f) = self.feast() else { return };
        if f.bloat < BLOAT_SMELL {
            return;
        }
        let hp = self.hero.pos();
        let mut woke = false;
        for e in self.enemies.iter_mut().filter(|e| e.kind() == EnemyKind::Gaki) {
            let Some(d) = e.duel.as_deref_mut() else { continue };
            let pos = d.pos;
            if let Some(den) = d.den.as_mut().filter(|den| !den.awake) {
                if dist_sq(pos, hp) < SMELL_RANGE * SMELL_RANGE {
                    den.awake = true;
                    den.returning = false;
                    woke = true;
                }
            }
        }
        if woke {
            self.audio.play(Sfx::Shriek);
            self.floater(hp.x, hp.y - 1.4, "THE GAKI SMELL THE FEAST ON HIM", 1.6, Some(0xc8b49a));
        }
    }

    /// Up to the table at the Stomach's heart: the dome is lifted.
    fn feast_dome(&mut self) {
        let Some(f) = self.feast() else { return };
        if f.lifted || dist_sq(self.hero.pos(), f.dome.v()) > DOME_REACH * DOME_REACH {
            return;
        }
        self.spring_climax();
    }

    /// The dome is lifted on an empty plate (the meal is him), and the Stomach convulses.
    fn spring_climax(&mut self) {
        let Some(f) = self.feast_mut() else { return };
        if f.lifted {
            return;
        }
        f.lifted = true;
        f.zone = Zone::Stomach;
        f.base.checkpoint = f.stomach_landing();
        let swarm = V::new(f.stomach.x, f.stomach.y + f.stomach_radius - 1.0);
        f.climax = Some(Climax::new(swarm));
        let dome = f.dome;
        // Everything in the Stomach wakes.
        for e in self.enemies.iter_mut() {
            if let Some(den) = e.duel.as_deref_mut().and_then(|d| d.den.as_mut()) {
                den.awake = true;
                den.returning = false;
            }
        }
        self.audio.play(Sfx::BossRoar);
        self.audio.play(Sfx::BossSlam);
        self.view.shake(0.8);
        self.view.sparkle(&self.level.grid, dome.x as f64, dome.y as f64, 1.0, 0xe8ecf4, 30);
        self.juice.banner = Some(Banner {
            text: "THE PLATE IS EMPTY".into(),
            sub: "THE MEAL IS HE   THE STOMACH CONVULSES".into(),
            color: ORANGE,
            life: 3.5,
            max: 3.5,
        });
    }

    /// The Last Course, one step: the walls, the acid, the swarm, the food in the air, the gaki the
    /// Stomach sends, the foes squeezed, and the pylorus's teeth.
    fn feast_climax(&mut self, dt: f64) {
        let Some(mut c) = self.feast_mut().and_then(|f| f.climax.take()) else { return };
        c.t += dt;
        let (squeeze, stomach, radius, bloat, exit) = {
            let f = self.feast().unwrap();
            (f.squeeze, f.stomach, f.stomach_radius, f.bloat, f.base.exit.tile)
        };
        let hp = self.hero.pos();
        let from_door = len(V::new(hp.x - squeeze.x, hp.y - squeeze.y));
        if from_door > c.walls() || c.t >= CLIMAX_END {
            self.digest();
            return;
        }
        let airborne = self.hero.airborne();
        let mut wounds: Vec<(f64, u32)> = Vec::new();
        // The acid, flooding in from the rim.
        if from_door > c.dry() && !airborne {
            c.acid_clock += dt;
            if c.acid_clock >= ACID_EVERY {
                c.acid_clock -= ACID_EVERY;
                wounds.push((ACID_DAMAGE, ACID));
            }
        } else {
            c.acid_clock = 0.0;
        }
        // The swarm, hunting him as a cloud.
        let pace = SWARM_PACE.0 + SWARM_PACE.1 * c.t;
        let dir = toward(c.swarm, hp);
        let gap = len(V::new(hp.x - c.swarm.x, hp.y - c.swarm.y));
        let step = (pace * dt).min(gap);
        c.swarm = V::new(c.swarm.x + dir.x * step, c.swarm.y + dir.y * step);
        if gap < SWARM_RADIUS {
            c.sting_clock += dt;
            if c.sting_clock >= ACID_EVERY {
                c.sting_clock -= ACID_EVERY;
                wounds.push((SWARM_DAMAGE, SWARM));
            }
        } else {
            c.sting_clock = 0.0;
        }
        // A point on the rim, inside the walls.
        let rim = |a: f64, c: &Climax| -> V {
            let p = V::new(stomach.x + a.cos() * (radius - 1.0), stomach.y + a.sin() * (radius - 1.0));
            let d = len(V::new(p.x - squeeze.x, p.y - squeeze.y));
            if d > c.walls() - 0.6 {
                let k = (c.walls() - 0.6).max(0.5) / d.max(1e-6);
                V::new(squeeze.x + (p.x - squeeze.x) * k, squeeze.y + (p.y - squeeze.y) * k)
            } else {
                p
            }
        };
        // The gut hurls food at him.
        c.throw_clock -= dt;
        if c.throw_clock <= 0.0 {
            c.throw_clock = 1.7 - 0.8 * (c.t / CLIMAX_SECONDS).min(1.0);
            let from = rim(self.rng.next() * std::f64::consts::TAU, &c);
            let aim = toward(from, hp);
            let kind = self.rng.pick(&FoodKind::ALL);
            c.morsels.push(Morsel { pos: from, vel: V::new(aim.x * MORSEL_SPEED, aim.y * MORSEL_SPEED), kind, age: 0.0, reflected: false });
        }
        let (parrying, guarding) =
            self.brawl.as_ref().map(|b| (b.hero.parrying(), matches!(b.hero.state, State::Guard { .. }))).unwrap_or((false, false));
        let reach = MORSEL_REACH * (1.0 + 0.08 * bloat as f64) + 0.3;
        let mut fed = 0;
        let mut struck: Vec<usize> = Vec::new();
        let mut refused = false;
        let mut blocked = false;
        let duelists: Vec<(usize, V, f64, bool)> = self
            .enemies
            .iter()
            .enumerate()
            .filter_map(|(i, e)| e.duel.as_ref().filter(|d| d.alive()).map(|d| (i, d.pos, d.radius(), e.kind() == EnemyKind::Gaki)))
            .collect();
        c.morsels.retain_mut(|m| {
            m.pos = V::new(m.pos.x + m.vel.x * dt, m.pos.y + m.vel.y * dt);
            m.age += dt;
            if m.age > 4.0 {
                return false;
            }
            if !m.reflected {
                if airborne || len(V::new(m.pos.x - hp.x, m.pos.y - hp.y)) > reach {
                    return true;
                }
                if parrying {
                    // Refused: it flies back at the gaki (at any foe, if no gaki is left).
                    m.reflected = true;
                    m.age = 0.0;
                    let target = duelists
                        .iter()
                        .filter(|d| d.3)
                        .chain(duelists.iter())
                        .min_by(|a, b| dist_sq(a.1, m.pos).total_cmp(&dist_sq(b.1, m.pos)))
                        .map(|d| d.1);
                    let aim = target.map(|t| toward(m.pos, t)).unwrap_or(V::new(-m.vel.x, -m.vel.y));
                    let aim = toward(V::new(0.0, 0.0), aim);
                    m.vel = V::new(aim.x * MORSEL_SPEED * 1.6, aim.y * MORSEL_SPEED * 1.6);
                    refused = true;
                    return true;
                }
                if guarding {
                    blocked = true;
                } else {
                    fed += 1;
                }
                return false;
            }
            if let Some(d) = duelists.iter().find(|d| len(V::new(d.1.x - m.pos.x, d.1.y - m.pos.y)) < d.2 + 0.35) {
                struck.push(d.0);
                return false;
            }
            true
        });
        // The Stomach sends its gaki.
        c.gaki_clock -= dt;
        let gaki_alive = duelists.iter().filter(|d| d.3).count();
        let mut spawn = None;
        if c.gaki_clock <= 0.0 {
            c.gaki_clock = GAKI_EVERY;
            if gaki_alive < CLIMAX_GAKI {
                let at = rim(self.rng.next() * std::f64::consts::TAU, &c);
                spawn = Some(P::new(at.x.round() as i32, at.y.round() as i32));
            }
        }
        // The walls squeeze the foes in with him (and digest any with nowhere to go).
        let walls = c.walls();
        let grid = &self.level.grid;
        let mut crushed = Vec::new();
        for e in self.enemies.iter_mut() {
            let Some(d) = e.duel.as_deref_mut().filter(|d| d.alive()) else { continue };
            let off = V::new(d.pos.x - squeeze.x, d.pos.y - squeeze.y);
            let l = len(off);
            if l > walls - 0.4 && l > 1e-6 && dist_sq(d.pos, stomach) <= (radius + 1.5) * (radius + 1.5) {
                let k = (walls - 0.4).max(0.3) / l;
                let to = V::new(squeeze.x + off.x * k, squeeze.y + off.y * k);
                if grid.is_walkable(to.x.round() as i32, to.y.round() as i32, false) {
                    d.pos = to;
                } else {
                    d.f.kill();
                    e.b.mode = EnemyMode::Eaten;
                    crushed.push(d.id);
                }
            }
        }
        if let Some(b) = self.brawl.as_mut() {
            for id in crushed {
                b.threat.release(id);
            }
        }
        // The pylorus: its teeth part for two seconds in ten, and let through only the lean.
        let open = c.open();
        let lean = bloat <= BLOAT_LEAN;
        let target = if open {
            if lean {
                1.0
            } else {
                0.3
            }
        } else {
            0.0
        };
        c.teeth += (target - c.teeth) * (dt * 8.0).min(1.0);
        let opened = open && !c.was_open;
        c.was_open = open;
        c.admitted = open && lean;
        let on_door = self.hero.occupied() == exit;
        if c.admitted {
            self.gate_open = true;
            self.door = 1.0;
            self.door_opening = true;
            let h = self.level.grid.elevation(exit.x, exit.y + 1);
            self.level.grid.set(exit.x, exit.y, Tile::Open);
            self.level.grid.set_elevation(exit.x, exit.y, h);
        } else {
            self.gate_open = false;
            self.door = 0.0;
            self.door_opening = false;
            if !on_door {
                self.level.grid.set(exit.x, exit.y, Tile::Rock);
            }
        }
        if let Some(f) = self.feast_mut() {
            f.bloat = (f.bloat + fed).min(MAX_BLOAT);
            f.climax = Some(c);
        }
        // What came of it.
        let hp = self.hero.pos();
        if opened {
            self.audio.play(Sfx::DoorGrind);
            let (text, color) = if lean {
                ("THE TEETH PART: THROUGH, NOW!".to_string(), ORANGE)
            } else {
                (format!("TOO FULL TO PASS: BLOAT {bloat}, 3 OR LESS GOES THROUGH"), 0xff5a3a)
            };
            self.floater(exit.x as f64, exit.y as f64 + 1.2, text, 1.8, Some(color));
        }
        if fed > 0 {
            self.audio.play(Sfx::Eat);
            self.floater(hp.x, hp.y - 0.8, format!("FORCE-FED  +{fed} BLOAT"), 1.0, Some(0xff5a3a));
        }
        if refused {
            self.audio.play(Sfx::Clang);
            self.floater(hp.x, hp.y - 0.8, "REFUSED", 0.8, Some(PALE));
        }
        if blocked {
            self.audio.play(Sfx::Kick);
        }
        for i in struck {
            self.stray_blow(i);
        }
        if let Some(at) = spawn {
            self.feast_raise_gaki(at);
        }
        for (loss, color) in wounds {
            if self.screen != Screen::Playing || !self.hero.alive {
                break;
            }
            self.wound_hero(loss, color);
        }
    }

    /// A gaki crawls out of the Stomach's wall at `at`, already hunting.
    fn feast_raise_gaki(&mut self, at: P) {
        let Some(f) = self.feast() else { return };
        if !self.level.grid.is_walkable(at.x, at.y, false) {
            return;
        }
        let (tier, s, r) = (f.base.tier, f.stomach, f.stomach_radius as i32);
        let den = Rect { x0: s.x as i32 - r, y0: s.y as i32 - r, x1: s.x as i32 + r, y1: s.y as i32 + r };
        let Some(mut brawl) = self.brawl.take() else { return };
        let mut raised = self.raise_packs(&[Pack { den, foes: vec![(EnemyKind::Gaki, 0, at)] }], tier, &mut brawl);
        self.brawl = Some(brawl);
        for e in raised.iter_mut() {
            if let Some(den) = e.duel.as_deref_mut().and_then(|d| d.den.as_mut()) {
                den.awake = true;
            }
        }
        self.enemies.extend(raised);
        self.refresh_foe_views();
        self.view.dust(&self.level.grid, at.x as f64, at.y as f64, 0xc8b49a, 14, 1.4);
    }

    /// Caught by the closing walls: digested. He wakes in the Hall of Courses at the top of the
    /// gullet, hungry, and the realm begins again.
    fn digest(&mut self) {
        self.audio.play(Sfx::BossSlam);
        self.audio.play(Sfx::Denied);
        self.view.shake(0.8);
        self.load_realm(Sin::Gluttony);
        self.checkpoint(Stage::Realm(Sin::Gluttony));
        self.hero.hunger = WAKE_HUNGER;
        self.hero.max_hp = self.hero.health_cap();
        self.hero.hp = self.hero.hp.min(self.hero.max_hp);
        self.juice.fade = 1.0;
        self.juice.banner = Some(Banner {
            text: "DIGESTED".into(),
            sub: "HE WAKES IN THE HALL OF COURSES, HUNGRY".into(),
            color: 0xff5a3a,
            life: 4.0,
            max: 4.0,
        });
        self.enter(Screen::Ready);
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{in_realm, STEP};
    use super::*;
    use crate::game::{CLEARED_SECONDS, READY_SECONDS};

    /// The Rotting Feast with no foes in it, play under way.
    fn feast_alone() -> Game {
        let mut g = in_realm(Sin::Gluttony);
        g.enemies.clear();
        g.refresh_foe_views();
        g
    }

    fn feast(g: &Game) -> &Feast {
        g.feast().expect("the Rotting Feast")
    }

    fn run(g: &mut Game, seconds: f64) {
        for _ in 0..(seconds / STEP).round() as usize {
            g.update(STEP);
        }
    }

    #[test]
    fn hunger_takes_his_maximum_health_down_to_two_fifths_and_food_gives_it_back() {
        let mut g = feast_alone();
        let full = g.hero.max_hp;
        run(&mut g, HUNGER_EVERY + 0.1);
        assert_eq!(g.hero.hunger, 1);
        assert_eq!(g.hero.max_hp, full - 1, "a point of maximum health every four seconds");
        // A long fast: no lower than 40%.
        g.feast_mut().unwrap().hunger_clock = HUNGER_EVERY * 500.0;
        g.update(STEP);
        assert_eq!(g.hero.max_hp, full - Feast::hunger_limit(full), "down to 40%");
        assert!(g.hero.hp <= g.hero.max_hp);
        // A bite: four back, ten healed, a stack of Bloat.
        let food = feast(&g).food.iter().find(|k| feast(&g).zone_at(k.at.y as f64) == Zone::Hall).copied().unwrap();
        let (hunger, hp) = (g.hero.hunger, g.hero.hp);
        g.hero.place(food.at);
        g.update(STEP);
        let f = feast(&g);
        assert!(f.food.iter().any(|k| k.at == food.at && k.eaten));
        assert_eq!(f.bloat, 1);
        assert_eq!(g.hero.hunger, hunger - BITE_RESTORE);
        assert_eq!(g.hero.hp, (hp + BITE_HEAL).min(g.hero.max_hp));
        // Walking over it again eats nothing.
        g.update(STEP);
        assert_eq!(feast(&g).bloat, 1);
    }

    #[test]
    fn bloat_slows_him_and_lengthens_his_landing_and_six_bars_the_narrow_arches() {
        let mut g = feast_alone();
        g.feast_mut().unwrap().bloat = 4;
        g.update(STEP);
        assert!((g.hero.pace - 0.8).abs() < 1e-9, "5% a stack");
        let b = g.brawl.as_ref().unwrap();
        assert_eq!(b.hero.evade_total, EVADE_AIR + b.kit.evade_land + 4, "a frame a stack on his landing");
        let narrow = feast(&g).seams[0].narrow.tiles.clone();
        assert!(narrow.iter().all(|p| g.level.grid.is_walkable(p.x, p.y, false)), "at 4 he fits");
        g.feast_mut().unwrap().bloat = BLOAT_WIDE;
        g.update(STEP);
        assert!(narrow.iter().all(|p| g.level.grid.get(p.x, p.y) == Tile::Rock), "at 6 he does not");
        let wide = feast(&g).seams[0].wide.tiles.clone();
        assert!(wide.iter().all(|p| g.level.grid.is_walkable(p.x, p.y, false)), "the wide arch stays open");
        g.feast_mut().unwrap().bloat = BLOAT_WIDE - 1;
        g.update(STEP);
        assert!(narrow.iter().all(|p| g.level.grid.is_walkable(p.x, p.y, false)), "lean again, he fits again");
    }

    #[test]
    fn at_eight_the_kitchens_crust_breaks_under_him_and_drops_him_into_the_stomach() {
        let mut g = feast_alone();
        let crust = feast(&g).crust[0].at;
        g.feast_mut().unwrap().bloat = BLOAT_CRUST - 1;
        g.hero.place(crust);
        g.update(STEP);
        assert_eq!(g.hero.occupied(), crust, "at 7 it bears him");
        g.feast_mut().unwrap().bloat = BLOAT_CRUST;
        let hp = g.hero.hp;
        g.update(STEP);
        let f = feast(&g);
        assert_eq!(g.hero.occupied(), f.stomach_landing(), "straight down into the Stomach");
        assert_eq!(g.level.grid.get(crust.x, crust.y), Tile::Pit);
        assert!(f.crust.iter().any(|k| k.at == crust && k.broken));
        assert!(f.seams.iter().all(|s| s.closed), "the zones he fell past are digested");
        assert_eq!((f.zone, f.base.checkpoint), (Zone::Stomach, f.stomach_landing()));
        assert!(g.hero.hp < hp, "into the acid");
    }

    #[test]
    fn a_seam_closes_behind_him_and_digests_the_zone_he_left() {
        let mut g = in_realm(Sin::Gluttony);
        let s = feast(&g).seams[0].clone();
        let behind = g.enemies.iter().filter(|e| e.duel.as_ref().is_some_and(|d| d.pos.y.round() as i32 >= s.top)).count();
        assert!(behind > 0);
        // In the hand-off rows the arch still stands open behind him.
        g.hero.place(P::new(s.narrow.north[0].x, s.top - 1));
        g.update(STEP);
        assert!(!feast(&g).seams[0].closed);
        g.hero.place(s.landing);
        g.hero.invuln = 1.0;
        g.update(STEP);
        let f = feast(&g);
        assert!(f.seams[0].closed && !f.seams[1].closed);
        assert!(s.narrow.tiles.iter().chain(&s.wide.tiles).all(|p| g.level.grid.get(p.x, p.y) == Tile::Rock), "its teeth shut");
        assert_eq!((f.zone, f.base.checkpoint), (Zone::Kitchens, s.landing));
        run(&mut g, 0.2);
        let left = g.enemies.iter().filter(|e| e.duel.as_ref().is_some_and(|d| d.pos.y.round() as i32 >= s.top)).count();
        assert_eq!(left, 0, "the Hall's foes digested with it");
        // A fall now: he rises past the seam, a fresh body.
        g.feast_mut().unwrap().bloat = 5;
        g.hero.hunger = 12;
        g.reset_round();
        assert_eq!(g.hero.occupied(), s.landing);
        assert_eq!((feast(&g).bloat, g.hero.hunger), (0, 0));
        assert!(feast(&g).seams[0].closed, "the zone behind stays digested");
    }

    #[test]
    fn the_floors_drift_him_toward_the_stomach_faster_in_each_zone() {
        let mut g = feast_alone();
        g.hero.free_move = true;
        let f = feast(&g).clone();
        let larder = f.rooms[2];
        let start = P::new(larder.x0 + 2, (larder.y0 + larder.y1) / 2);
        g.hero.place(start);
        run(&mut g, 1.0);
        let moved = start.y as f64 - g.hero.pos().y;
        assert!((moved - DRIFT[2]).abs() < 0.15, "a tile a second in the Larder: {moved}");
        let hall = f.rooms[0];
        let start = P::new(hall.x0 + 1, hall.y1 - 3);
        g.hero.place(start);
        run(&mut g, 1.0);
        let moved = start.y as f64 - g.hero.pos().y;
        assert!((moved - DRIFT[0]).abs() < 0.08, "a fifth of a tile in the Hall: {moved}");
    }

    #[test]
    fn every_l3_that_lands_burns_a_stack_once() {
        let mut g = feast_alone();
        g.feast_mut().unwrap().bloat = 3;
        let b = g.brawl.as_mut().unwrap();
        b.hero.state = State::Attack { mv: MoveId::L3, frame: 10 };
        b.stats.hits += 2;
        g.update_feast(STEP);
        assert_eq!(feast(&g).bloat, 2);
        g.brawl.as_mut().unwrap().stats.hits += 1;
        g.update_feast(STEP);
        assert_eq!(feast(&g).bloat, 2, "one stack a swing");
        // An L1 that lands burns nothing; the next L3 does.
        let b = g.brawl.as_mut().unwrap();
        b.hero.state = State::Attack { mv: MoveId::L1, frame: 6 };
        b.stats.hits += 1;
        g.update_feast(STEP);
        assert_eq!(feast(&g).bloat, 2);
        let b = g.brawl.as_mut().unwrap();
        b.hero.state = State::Attack { mv: MoveId::L3, frame: 10 };
        b.stats.hits += 1;
        g.update_feast(STEP);
        assert_eq!(feast(&g).bloat, 1);
    }

    #[test]
    fn the_gaki_lurk_while_he_is_lean_and_come_when_he_smells_of_food() {
        let mut g = in_realm(Sin::Gluttony);
        let den = GAKI_DENS[0];
        let gaki: Vec<usize> =
            (0..g.enemies.len()).filter(|&i| g.enemies[i].kind() == EnemyKind::Gaki && den.contains(g.enemies[i].post.unwrap())).collect();
        assert!(gaki.len() >= 3);
        let asleep = |g: &Game, i: usize| g.enemies[i].duel.as_ref().and_then(|d| d.den).is_some_and(|d| !d.awake);
        // Lean, he walks up the Hall past them, and they sleep on.
        g.hero.place(P::new(23, 100));
        g.hero.invuln = 99.0;
        run(&mut g, 1.0);
        assert!(gaki.iter().all(|&i| asleep(&g, i)), "they do not smell him yet");
        g.feast_mut().unwrap().bloat = BLOAT_SMELL;
        g.update(STEP);
        assert!(gaki.iter().all(|&i| !asleep(&g, i)), "at 3 Bloat they come");
        assert!(g.floaters.iter().any(|f| f.text == "THE GAKI SMELL THE FEAST ON HIM"));
    }

    #[test]
    fn a_purge_font_takes_five_stacks_and_twenty_health_and_stuns_him_once() {
        let mut g = feast_alone();
        let font = feast(&g).fonts[0].at;
        g.feast_mut().unwrap().bloat = 7;
        let hp = g.hero.hp;
        g.hero.place(font);
        g.update(STEP);
        let f = feast(&g);
        assert_eq!(f.bloat, 2);
        assert!(f.fonts[0].used && f.stun > 0.0);
        assert_eq!(g.hero.hp, hp - PURGE_HEALTH as i32);
        assert!(matches!(g.brawl.as_ref().unwrap().hero.state, State::Reel { kind: Reel::Hitstun, .. }), "stunned");
        g.feast_mut().unwrap().bloat = 6;
        g.update(STEP);
        assert_eq!(feast(&g).bloat, 6, "the font is dry");
    }

    #[test]
    fn lifting_the_dome_springs_the_last_course_and_the_pylorus_admits_only_the_lean() {
        let mut g = feast_alone();
        g.cheat_feast();
        assert!(feast(&g).lifted && feast(&g).climax.is_some(), "F2: the Last Course under way");
        let mut g = feast_alone();
        let dome = feast(&g).dome;
        for i in 0..3 {
            g.close_seam(i);
        }
        g.hero.place(P::new(dome.x, dome.y + 2));
        g.update(STEP);
        assert!(feast(&g).climax.is_none(), "not yet");
        g.hero.place(P::new(dome.x, dome.y + 1));
        g.update(STEP);
        assert!(feast(&g).lifted && feast(&g).climax.is_some(), "the dome is lifted");
        let exit = feast(&g).base.exit;
        g.hero.place(exit.front());
        g.hero.invuln = 99.0;
        // Shut: nothing passes.
        g.feast_mut().unwrap().climax.as_mut().unwrap().t = 3.0;
        g.update(STEP);
        assert!(!g.gate_open && g.level.grid.get(exit.tile.x, exit.tile.y) == Tile::Rock);
        // Open, but he is too full.
        g.feast_mut().unwrap().bloat = BLOAT_LEAN + 1;
        g.feast_mut().unwrap().climax.as_mut().unwrap().t = PYLORUS_PERIOD - 1.5;
        g.update(STEP);
        assert!(!g.gate_open && g.level.grid.get(exit.tile.x, exit.tile.y) == Tile::Rock, "too full to pass");
        // Open, and lean enough.
        g.feast_mut().unwrap().bloat = BLOAT_LEAN;
        g.update(STEP);
        assert!(g.gate_open && g.door >= 1.0 && g.level.grid.is_walkable(exit.tile.x, exit.tile.y, false));
        g.hero.m.teleport(exit.tile);
        g.update(STEP);
        assert_eq!(g.screen, Screen::Cleared, "through the pylorus");
        run(&mut g, CLEARED_SECONDS + 0.1);
        assert_eq!((g.screen, g.realm, g.boss.is_some()), (Screen::Ready, Some(Sin::Gluttony), true), "Beelzebub's court");
        assert!(g.run.is_none());
    }

    #[test]
    fn the_closing_walls_digest_him_and_he_wakes_in_the_hall_hungry() {
        let mut g = feast_alone();
        g.cheat_feast();
        let f = feast(&g).clone();
        g.feast_mut().unwrap().climax.as_mut().unwrap().t = 30.0;
        // The acid burns where the floor floods.
        let dry = acid_radius(30.0);
        let wet = P::new(f.squeeze.x as i32, (f.squeeze.y + dry + 1.0).round() as i32);
        assert!(g.level.grid.is_walkable(wet.x, wet.y, false));
        g.hero.place(wet);
        let hp = g.hero.hp;
        run(&mut g, ACID_EVERY * 2.0 + 0.05);
        assert!(g.hero.hp <= hp - 2 * ACID_DAMAGE as i32 + 2, "acid: {hp} to {}", g.hero.hp);
        // Caught by the walls: digested.
        g.hero.place(f.stomach_landing());
        g.update(STEP);
        assert_eq!(g.screen, Screen::Ready);
        assert_eq!(g.hero.occupied(), f.base.arrival, "at the top of the gullet");
        assert_eq!(g.hero.hunger, WAKE_HUNGER, "hungry");
        let fresh = feast(&g);
        assert!(!fresh.lifted && fresh.seams.iter().all(|s| !s.closed) && fresh.food.iter().all(|k| !k.eaten), "the feast laid again");
        run(&mut g, READY_SECONDS + 0.1);
        assert_eq!(g.screen, Screen::Playing);
    }

    #[test]
    fn food_the_gut_hurls_feeds_him_unless_he_parries_it_into_the_gaki() {
        let mut g = feast_alone();
        g.cheat_feast();
        g.hero.invuln = 99.0;
        let hp = g.hero.pos();
        let throw = |g: &mut Game| {
            let c = g.feast_mut().unwrap().climax.as_mut().unwrap();
            c.throw_clock = 99.0;
            c.morsels.push(Morsel {
                pos: V::new(hp.x + 0.2, hp.y),
                vel: V::new(-1.0, 0.0),
                kind: FoodKind::Pie,
                age: 0.0,
                reflected: false,
            });
        };
        throw(&mut g);
        let bloat = feast(&g).bloat;
        g.update_feast(STEP);
        assert_eq!(feast(&g).bloat, bloat + 1, "force-fed");
        // Parried, it flies back at a gaki.
        g.feast_raise_gaki(P::new(hp.x as i32 - 4, hp.y as i32));
        let gaki = g.enemies.iter().position(|e| e.kind() == EnemyKind::Gaki).unwrap();
        let life = g.enemies[gaki].duel.as_ref().unwrap().hp;
        throw(&mut g);
        g.brawl.as_mut().unwrap().hero.state = State::Guard { frame: 4 };
        g.update_feast(STEP);
        assert_eq!(feast(&g).bloat, bloat + 1, "refused");
        assert!(feast(&g).climax.as_ref().unwrap().morsels.iter().any(|m| m.reflected));
        for _ in 0..120 {
            g.update_feast(STEP);
        }
        assert!(g.enemies[gaki].duel.as_ref().unwrap().hp < life, "into the gaki");
    }
}
