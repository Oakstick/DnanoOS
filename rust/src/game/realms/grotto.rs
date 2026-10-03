//! The Serpent Grotto in play (Envy's realm, REALMS.md §2; laid out in `logic::realms::grotto`).
//!
//! - **Crossings.** A step into a deep pool on the Waterline is a dive: he comes down on the
//!   Drowned Floor at the pool's twin (1.5 tiles beyond where it showed through the water). A step
//!   onto a geyser vent below holds him there, open to blows, for a four-second ride up to the
//!   Waterline. Deep water he falls into (a hump moving on under him, a kappa's charge) takes him
//!   down too. Each crossing of the true way moves his checkpoint on.
//! - **Coils.** Leviathan's humps come and go through the deep water above, bridges while they
//!   pass; his walls slide across the passages below, and batter him aside if he stands in the way.
//! - **The Eye** opens every twenty seconds and sweeps the cavern for six. Seen, he is doomed (his
//!   next wound ×1.3); and while it watches, a bottle, a blessing or his patron's call he reaches
//!   for is taken, and the nearest native keeps it until he kills it. A nazar bead from his belt
//!   takes one gaze whole, and cracks.
//! - **Kiyohime's Bell.** In the Bell Cavern the court's portal glows beneath the black pool and the
//!   water whispers *"All you want lies below."* He dives, and the bell drops over him on a disc of
//!   shallows: the serpent coils it, the bronze heats, the tiles outside the safe circle scorch and
//!   the circle shrinks a ring every six seconds, steam bursts in rings, and the Eye watches through
//!   the sound-hole. Three heavy blows where the head presses (the bronze glowing white) and the
//!   serpent uncoils: the bell lifts for a second, and a dive through the seam beneath brings him to
//!   the real portal, open now. Miss it and the bell drops again on a smaller disc; let the circle
//!   go and it crushes him, and he surfaces at the cavern's mouth.

use super::super::{Banner, Game};
use super::Meter;
use crate::audio::Sfx;
use crate::input::Action;
use crate::logic::angels::Blessing;
use crate::logic::combat::{MoveId, State};
use crate::logic::enemy::EnemyKind;
use crate::logic::gear::{self, BELT_SLOTS};
use crate::logic::realms::grotto::*;
use crate::logic::realms::Run;
use crate::logic::types::{Dir, Tile, CARDINALS, P, V};

/// The realm's green, and the Eye's.
const GREEN: u32 = 0x3ad06a;
const EYE_GREEN: u32 = 0x8aff5a;
const NAZAR_BLUE: u32 = 0x4a8cff;
const BRONZE_HOT: u32 = 0xff7a2a;
/// The crossings of the true way made once he is at the bell, and once he is through its seam.
const AT_THE_BELL: u8 = 5;
const THROUGH: u8 = 6;
/// What a coil's wall takes from him as it batters him aside.
const BATTER: f64 = 6.0;

impl Game {
    fn grotto(&self) -> Option<&Grotto> {
        match self.run.as_ref() {
            Some(Run::Envy(g)) => Some(g),
            _ => None,
        }
    }

    fn grotto_mut(&mut self) -> Option<&mut Grotto> {
        match self.run.as_mut() {
            Some(Run::Envy(g)) => Some(g),
            _ => None,
        }
    }

    /// As the realm is entered (its packs raised, the camera above): the grotto floods, and the
    /// coils take their places.
    pub(in crate::game) fn enter_grotto(&mut self) {
        let Some(Run::Envy(g)) = self.run.as_mut() else { return };
        g.flood(&mut self.level.grid);
        let grid = &self.level.grid;
        g.coil_base =
            g.coils.iter().map(|k| k.track().into_iter().map(|p| (p, grid.get(p.x, p.y), grid.is_bridge(p.x, p.y))).collect()).collect();
        g.covered = vec![Vec::new(); g.coils.len()];
        self.grotto_coils();
    }

    /// The realm's own rules each tick.
    pub(in crate::game) fn update_grotto(&mut self, dt: f64) {
        let Some(g) = self.grotto_mut() else { return };
        g.time += dt;
        g.eye_clock += dt;
        g.lull = (g.lull - dt).max(0.0);
        self.grotto_coils();
        self.grotto_ride(dt);
        self.grotto_crossings();
        self.grotto_beads();
        self.grotto_eye();
        self.grotto_thefts();
        self.grotto_bell(dt);
    }

    /// The controls as the realm leaves them (`Game::steer_run`): held fast on a geyser's ride, and
    /// under the Eye what he reaches for is taken.
    pub(in crate::game) fn steer_grotto(&mut self, wanted: Dir) -> Dir {
        self.hero.pace = 1.0;
        let Some(g) = self.grotto() else { return wanted };
        let riding = g.ride.is_some();
        let watched = self.grotto_watched();
        if riding {
            self.hero.pace = 0.0;
            self.hero.wish = V::new(0.0, 0.0);
            self.input.consume(Action::Jump);
        }
        if watched && self.hero.alive {
            self.grotto_snatch();
        }
        if riding {
            Dir::None
        } else {
            wanted
        }
    }

    /// After a fall, as he rises at the checkpoint (and as the realm is first entered): the bell
    /// hangs again (unless he is through it), the ride and the Eye begin afresh.
    pub(in crate::game) fn reset_grotto(&mut self) {
        self.lift_bell_off();
        self.hero.pace = 1.0;
        let Some(g) = self.grotto_mut() else { return };
        if g.bell.phase != BellPhase::Won {
            g.bell.phase = BellPhase::Hanging;
        }
        g.ride = None;
        g.lull = 0.0;
        g.eye_clock = 0.0;
        g.warded = None;
        g.seen = false;
        g.announced = None;
        g.marked = None;
        g.steam_hit = None;
        g.heavy_counted = false;
        g.shoved = false;
    }

    /// F2: straight to the last trap: the bell drops over him (or, once he is through it, he
    /// stands before the open portal).
    pub(in crate::game) fn cheat_grotto(&mut self) {
        let Some(g) = self.grotto_mut() else { return };
        if g.bell.phase == BellPhase::Won {
            self.cheat_to_far_portal();
            return;
        }
        g.whispered = true;
        g.stage = g.stage.max(AT_THE_BELL);
        let mouth = g.bell.mouth;
        g.base.checkpoint = mouth;
        self.level.hero_spawn = mouth;
        self.lift_bell_off();
        if let Some(g) = self.grotto_mut() {
            g.bell.phase = BellPhase::Hanging;
        }
        self.drop_bell();
    }

    /// What the HUD shows of the realm's rules: his nazar beads, the Eye, and under the bell its heat
    /// and the blows on the serpent's head.
    pub(in crate::game) fn grotto_meters(&self) -> Vec<Meter> {
        let Some(g) = self.grotto() else { return Vec::new() };
        let mut out = vec![Meter::count("NAZAR", g.nazar, NAZAR_BLUE)];
        match g.bell.phase {
            BellPhase::Down | BellPhase::Lifted => {
                let heat = g.bell.heat() as f32;
                out.push(Meter::bar("BRONZE", heat, 1.0, BRONZE_HOT).warning(heat > 0.6));
                out.push(Meter::count("HEAD", g.bell.blows, 0xfff4e0).warning(g.bell.phase == BellPhase::Lifted));
                out.push(Meter::bar("THE EYE", 1.0, 1.0, EYE_GREEN).warning(true));
            }
            _ => match g.gaze() {
                Some((_, t)) => out.push(Meter::bar("THE EYE", (1.0 - t) as f32, 1.0, EYE_GREEN).warning(true)),
                None => {
                    let left = until_gaze(g.eye_clock);
                    let shut = (EYE_EVERY - EYE_OPEN).max(EYE_FIRST);
                    out.push(Meter::bar("THE EYE", (1.0 - left / shut).clamp(0.0, 1.0) as f32, 1.0, 0x4a7a5a).warning(left < 2.0));
                }
            },
        }
        if !g.thefts.is_empty() {
            out.push(Meter::count("STOLEN", g.thefts.len() as u32, GREEN).warning(true));
        }
        out
    }

    // ------------------------------------------------------------------ coils

    /// The coils move on: a hump's tiles bear him while it lies on them, a wall's are rock. A wall
    /// moving onto him batters him aside; a foe in its way is pushed aside too.
    fn grotto_coils(&mut self) {
        let Some(Run::Envy(g)) = self.run.as_mut() else { return };
        if g.coil_base.len() != g.coils.len() {
            return;
        }
        let grid = &mut self.level.grid;
        for k in 0..g.coils.len() {
            let now = g.coils[k].covered(g.time);
            if now == g.covered[k] {
                continue;
            }
            for &(p, t, bridge) in &g.coil_base[k] {
                if g.covered[k].contains(&p) && !now.contains(&p) {
                    grid.set(p.x, p.y, t);
                    grid.set_bridge(p.x, p.y, bridge);
                }
            }
            for &p in &now {
                match g.coils[k].kind {
                    CoilKind::Hump => {
                        grid.set(p.x, p.y, Tile::Open);
                        grid.set_bridge(p.x, p.y, true);
                    }
                    CoilKind::Wall => grid.set(p.x, p.y, Tile::Rock),
                }
            }
            g.covered[k] = now;
        }
        // Battered aside by a wall.
        let o = self.hero.occupied();
        if self.level.grid.get(o.x, o.y) == Tile::Rock && self.hero.alive {
            if let Some(q) = self.free_beside(o) {
                self.hero.place(q);
                self.view.shake(0.3);
                self.audio.play(Sfx::BossSlam);
                self.wound_hero(BATTER, GREEN);
                let hp = self.hero.pos();
                self.floater(hp.x, hp.y - 0.9, "THE COIL BATTERS YOU ASIDE", 1.2, Some(GREEN));
            }
        }
        for i in 0..self.enemies.len() {
            let Some(at) = self.enemies[i].duel.as_ref().map(|d| d.pos) else { continue };
            let t = P::new(at.x.round() as i32, at.y.round() as i32);
            if self.level.grid.get(t.x, t.y) == Tile::Rock {
                if let Some(q) = self.free_beside(t) {
                    if let Some(d) = self.enemies[i].duel.as_deref_mut() {
                        d.pos = q.v();
                    }
                }
            }
        }
    }

    /// The nearest floor round `p` (within three tiles).
    fn free_beside(&self, p: P) -> Option<P> {
        let g = &self.level.grid;
        for r in 1..=3 {
            for d in CARDINALS {
                let (dx, dy) = d.vec();
                let q = P::new(p.x + dx * r, p.y + dy * r);
                if g.is_walkable(q.x, q.y, false) {
                    return Some(q);
                }
            }
            for (dx, dy) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
                let q = P::new(p.x + dx * r, p.y + dy * r);
                if g.is_walkable(q.x, q.y, false) {
                    return Some(q);
                }
            }
        }
        None
    }

    // ------------------------------------------------------------------ crossings

    /// A geyser carrying him up: held on its vent, rising, until it sets him down on the Waterline.
    fn grotto_ride(&mut self, dt: f64) {
        let Some(g) = self.grotto_mut() else { return };
        let Some((i, t)) = g.ride else { return };
        let vent = g.vents[i];
        let t = t + dt;
        if t < RIDE_SECONDS {
            g.ride = Some((i, t));
            if self.hero.occupied() != vent.at {
                self.hero.place(vent.at);
            }
            self.hero.altitude = 4.0 * (t / RIDE_SECONDS);
            return;
        }
        g.ride = None;
        g.lull = CROSS_LULL;
        self.reach_stage(vent.stage, vent.to);
        self.hero.place(vent.to);
        self.hero.altitude = 0.0;
        let grid = &self.level.grid;
        self.view.sparkle(grid, vent.to.x as f64, vent.to.y as f64, 0.4, 0xd8fff0, 30);
        self.view.dust(grid, vent.to.x as f64, vent.to.y as f64, 0xcfeee0, 18, 1.6);
        self.audio.play(Sfx::Whoosh);
        self.floater(vent.to.x as f64, vent.to.y as f64 - 0.8, "THE GEYSER SETS YOU ON THE WATERLINE", 1.4, Some(0xcfeee0));
    }

    /// What his feet find: the bell's pool, the open seam under the bell, a deep pool, deep water, a
    /// vent; or a kappa's charge drags him under.
    fn grotto_crossings(&mut self) {
        if !self.hero.alive || self.hero.airborne() {
            return;
        }
        let o = self.hero.occupied();
        let shoving = self.brawl.as_ref().is_some_and(|b| b.ails.shove.x != 0.0 || b.ails.shove.y != 0.0);
        let Some(Run::Envy(g)) = self.run.as_mut() else { return };
        let began_shove = shoving && !g.shoved;
        g.shoved = shoving;
        if g.ride.is_some() || g.lull > 0.0 {
            return;
        }
        let bell = &g.bell;
        if bell.phase == BellPhase::Hanging && bell.pool.contains(&o) {
            self.drop_bell();
            return;
        }
        if bell.phase == BellPhase::Lifted && bell.dist(o.v()) <= SEAM_RADIUS {
            self.through_the_seam();
            return;
        }
        if let Some(i) = g.pool_at(o) {
            let q = g.pools[i];
            self.dive(q.to, q.stage, "YOU DIVE");
            return;
        }
        if layer_of(o) == Layer::Waterline && self.level.grid.get(o.x, o.y) == Tile::Pit {
            let to = g.sink_to(&self.level.grid, o);
            self.dive(to, None, "THE DEEP WATER TAKES YOU DOWN");
            return;
        }
        if let Some(i) = g.vent_at(o) {
            g.ride = Some((i, 0.0));
            self.audio.play(Sfx::Wind);
            let grid = &self.level.grid;
            self.view.dust(grid, o.x as f64, o.y as f64, 0xd8fff0, 24, 2.4);
            self.floater(o.x as f64, o.y as f64 - 0.8, "THE GEYSER RISES", 1.2, Some(0xd8fff0));
            return;
        }
        if began_shove && layer_of(o) == Layer::Waterline {
            let hp = self.hero.pos();
            let kappa = self.enemies.iter().filter_map(|e| e.duel.as_ref().map(|d| (e.kind(), d.pos))).any(|(k, at)| {
                let (dx, dy) = (at.x - hp.x, at.y - hp.y);
                k == EnemyKind::Kappa && dx * dx + dy * dy < 2.6 * 2.6
            });
            if kappa {
                let Some(g) = self.grotto() else { return };
                let to = g.sink_to(&self.level.grid, o);
                self.dive(to, None, "THE KAPPA DRAGS YOU UNDER");
            }
        }
    }

    /// Down through the water to `to` on the Drowned Floor: he sinks the two storeys as he comes down.
    fn dive(&mut self, to: P, stage: Option<u8>, why: &str) {
        let from = self.hero.pos();
        let first = match self.grotto_mut() {
            Some(g) => {
                g.lull = CROSS_LULL;
                let first = !g.dived;
                g.dived = true;
                first
            }
            None => return,
        };
        self.reach_stage(stage, to);
        let grid = &self.level.grid;
        self.view.sparkle(grid, from.x, from.y, 0.2, 0xd8fff0, 26);
        self.view.dust(grid, from.x, from.y, 0xbfe8d8, 16, 1.4);
        self.hero.place(to);
        self.hero.altitude = 4.0;
        self.audio.play(Sfx::Drip);
        self.audio.play(Sfx::Whoosh);
        self.floater(to.x as f64, to.y as f64 - 0.8, why, 1.2, Some(0xbfe8d8));
        if first {
            self.juice.banner = Some(Banner {
                text: "THE DROWNED FLOOR".into(),
                sub: "IT LOOKED RICHER FROM ABOVE   GEYSERS ONLY GO UP".into(),
                color: 0xe8d870,
                life: 3.0,
                max: 3.0,
            });
        }
    }

    /// A crossing of the true way: his checkpoint moves on to where it set him down.
    fn reach_stage(&mut self, stage: Option<u8>, at: P) {
        let Some(s) = stage else { return };
        let Some(g) = self.grotto_mut() else { return };
        if s > g.stage {
            g.stage = s;
            g.base.checkpoint = at;
            self.level.hero_spawn = at;
        }
    }

    /// Beads underfoot go onto his belt.
    fn grotto_beads(&mut self) {
        if self.hero.airborne() || !self.hero.alive {
            return;
        }
        let o = self.hero.occupied();
        let Some(g) = self.grotto_mut() else { return };
        let Some(i) = g.beads.iter().position(|b| !b.taken && b.at == o) else { return };
        g.beads[i].taken = true;
        g.nazar += 1;
        let n = g.nazar;
        self.audio.play(Sfx::Milestone);
        self.view.sparkle(&self.level.grid, o.x as f64, o.y as f64, 0.5, NAZAR_BLUE, 24);
        self.floater(o.x as f64, o.y as f64 - 0.6, format!("A NAZAR BEAD   {n} ON YOUR BELT"), 1.4, Some(NAZAR_BLUE));
    }

    // ------------------------------------------------------------------ the Eye

    /// The Eye opens and sweeps: seen, he is doomed, unless a bead takes the gaze.
    fn grotto_eye(&mut self) {
        let hp = self.hero.pos();
        let o = self.hero.occupied();
        let hidden = self.hero.invisible() || !self.hero.alive;
        let Some(g) = self.grotto_mut() else { return };
        let Some((n, _)) = g.gaze() else {
            g.seen = false;
            return;
        };
        let opened = g.announced != Some(n);
        g.announced = Some(n);
        let in_bell = matches!(g.bell.phase, BellPhase::Down | BellPhase::Lifted);
        let seen = !hidden && !in_bell && g.eye_for(o).is_some_and(|(eye, angle)| in_cone(eye, angle, hp));
        let warded = g.warded == Some(n);
        let mut cracked = false;
        let mut doomed = false;
        if seen && !warded {
            if g.nazar > 0 {
                g.nazar -= 1;
                g.warded = Some(n);
                cracked = true;
            } else {
                doomed = g.marked != Some(n);
                g.marked = Some(n);
            }
        }
        g.seen = seen && g.warded != Some(n);
        let gazing = g.seen;
        if opened {
            self.audio.play(Sfx::Soul);
            self.floater(hp.x, hp.y - 1.4, "THE EYE OPENS IN THE WATER", 1.6, Some(EYE_GREEN));
        }
        if cracked {
            self.audio.play(Sfx::ShieldBreak);
            self.view.sparkle(&self.level.grid, hp.x, hp.y, 0.9, NAZAR_BLUE, 30);
            self.floater(hp.x, hp.y - 1.0, "THE NAZAR TAKES THE GAZE, AND CRACKS", 1.6, Some(NAZAR_BLUE));
        }
        if gazing {
            if let Some(b) = self.brawl.as_mut() {
                b.ails.doom = b.ails.doom.max(GAZE_DOOM);
            }
        }
        if doomed {
            self.audio.play(Sfx::Stinger);
            self.floater(hp.x, hp.y - 1.0, "THE EYE SEES YOU: DOOMED", 1.6, Some(EYE_GREEN));
        }
    }

    /// The Eye watches him now: its cone holds him (no bead has warded this gaze), or he is under the
    /// bell, where it looks through the sound-hole.
    fn grotto_watched(&self) -> bool {
        let Some(g) = self.grotto() else { return false };
        matches!(g.bell.phase, BellPhase::Down | BellPhase::Lifted) || g.seen
    }

    /// He reaches for a bottle, a blessing or his patron while the Eye watches: the grotto takes it.
    fn grotto_snatch(&mut self) {
        let slot = if self.input.was_pressed(&[Action::Drink]) {
            Some(self.belt_index)
        } else {
            (0..BELT_SLOTS).find(|&s| self.input.was_pressed(&[Action::Slot(s as u8)]))
        };
        if let Some(slot) = slot.filter(|&s| s < self.belt.len()) {
            self.input.consume(Action::Drink);
            for s in 0..BELT_SLOTS {
                self.input.consume(Action::Slot(s as u8));
            }
            let kind = self.belt.remove(slot);
            if self.belt_index >= self.belt.len() {
                self.belt_index = self.belt.len().saturating_sub(1);
            }
            self.steal(Stolen::Potion(kind));
        }
        if self.input.was_pressed(&[Action::Angel]) {
            if let Some(mut b) = self.blessing.filter(|b| b.charges > 0) {
                self.input.consume(Action::Angel);
                b.charges -= 1;
                self.blessing = (b.charges > 0).then_some(b);
                self.steal(Stolen::Blessing(b.angel));
            } else if let Some(angel) = self.progress.hero(self.hero_kind).patron.filter(|_| self.patron_wait <= 0.0) {
                self.input.consume(Action::Angel);
                self.patron_wait = angel.recharge();
                self.steal(Stolen::Patron);
            }
        }
    }

    /// The nearest native takes what he reached for, and grows stronger for it, until he kills it.
    pub(in crate::game) fn steal(&mut self, what: Stolen) {
        let hp = self.hero.pos();
        let name = self.stolen_name(what);
        let thief =
            self.enemies.iter().filter_map(|e| e.duel.as_ref().filter(|d| !d.f.is_dead() && d.hp > 0.0).map(|d| (d.id, d.pos))).min_by(
                |a, b| {
                    let da = (a.1.x - hp.x).powi(2) + (a.1.y - hp.y).powi(2);
                    let db = (b.1.x - hp.x).powi(2) + (b.1.y - hp.y).powi(2);
                    da.total_cmp(&db)
                },
            );
        self.audio.play(Sfx::Denied);
        self.audio.play(Sfx::Cackle);
        self.floater(hp.x, hp.y - 1.2, format!("THE EYE WATCHES: THE GROTTO TAKES YOUR {name}"), 1.8, Some(GREEN));
        let Some((id, at)) = thief else { return };
        for e in self.enemies.iter_mut() {
            if let Some(d) = e.duel.as_deref_mut().filter(|d| d.id == id) {
                d.hp = d.max_hp;
                d.breed.damage *= 1.2;
            }
        }
        if let Some(g) = self.grotto_mut() {
            g.thefts.push(Theft { by: id, what });
        }
        self.view.sparkle(&self.level.grid, at.x, at.y, 1.0, GREEN, 30);
        self.floater(at.x, at.y - 1.2, format!("IT KEEPS YOUR {name}"), 2.0, Some(GREEN));
    }

    fn stolen_name(&self, what: Stolen) -> String {
        match what {
            Stolen::Potion(k) => gear::potion(self.hero_kind, k).name.to_uppercase(),
            Stolen::Blessing(a) => format!("{}'S BLESSING", a.name().to_uppercase()),
            Stolen::Patron => "PATRON'S CALL".into(),
        }
    }

    /// A thief has fallen: what it kept comes back to him.
    fn grotto_thefts(&mut self) {
        let Some(g) = self.grotto() else { return };
        if g.thefts.is_empty() {
            return;
        }
        let alive = |id: u32| self.enemies.iter().any(|e| e.duel.as_ref().is_some_and(|d| d.id == id && !d.f.is_dead() && d.hp > 0.0));
        let back: Vec<Theft> = g.thefts.iter().copied().filter(|t| !alive(t.by)).collect();
        if back.is_empty() {
            return;
        }
        if let Some(g) = self.grotto_mut() {
            g.thefts.retain(|t| !back.iter().any(|b| b.by == t.by && b.what == t.what));
        }
        let hp = self.hero.pos();
        for t in back {
            let name = self.stolen_name(t.what);
            match t.what {
                Stolen::Potion(k) => {
                    if self.belt.len() < BELT_SLOTS {
                        self.belt.push(k);
                    } else {
                        continue;
                    }
                }
                Stolen::Blessing(a) => match self.blessing.as_mut() {
                    Some(b) if b.angel == a => b.charges += 1,
                    None => self.blessing = Some(Blessing { angel: a, charges: 1 }),
                    Some(_) => continue,
                },
                Stolen::Patron => self.patron_wait = 0.0,
            }
            self.audio.play(Sfx::Heal);
            self.floater(hp.x, hp.y - 1.2, format!("YOUR {name} COMES BACK TO YOU"), 1.8, Some(0xe8ffd0));
        }
    }

    // ------------------------------------------------------------------ the bell

    /// Kiyohime's bell, each tick: the whisper as he comes into its cavern; down over him, the
    /// heat, the shrinking circle, the steam and the serpent's head; lifted, the moment to dive.
    fn grotto_bell(&mut self, dt: f64) {
        let hp = self.hero.pos();
        let o = self.hero.occupied();
        let airborne = self.hero.airborne();
        let Some(g) = self.grotto_mut() else { return };
        match g.bell.phase {
            BellPhase::Hanging => {
                if !g.whispered && g.cavern_at(o) == Some(BELL) && layer_of(o) == Layer::Waterline {
                    g.whispered = true;
                    let mouth = g.bell.mouth;
                    let stage = Some(AT_THE_BELL);
                    self.reach_stage(stage, mouth);
                    self.audio.play(Sfx::Bells);
                    self.juice.banner = Some(Banner {
                        text: "ALL YOU WANT LIES BELOW".into(),
                        sub: "THE WATER WHISPERS   THE COURT'S PORTAL GLOWS BENEATH THE BELL".into(),
                        color: GREEN,
                        life: 4.0,
                        max: 4.0,
                    });
                }
            }
            BellPhase::Down => {
                g.bell.clock += dt;
                let bell = g.bell.clone();
                let safe = bell.safe();
                if safe < 0.0 {
                    self.crush();
                    return;
                }
                let lost = (bell.clock / RING_SECONDS).floor() as u32;
                let ring_closed = lost > g.rings_lost;
                g.rings_lost = lost;
                // The scorch: outside the safe circle the shallows boil on the bronze.
                let here = bell.dist(o.v());
                let mut wounds: Vec<(f64, &str)> = Vec::new();
                if here > safe + 0.01 && !airborne {
                    g.bell.scorch -= dt;
                    if g.bell.scorch <= 0.0 {
                        g.bell.scorch = SCORCH_EVERY;
                        wounds.push((SCORCH_DAMAGE, ""));
                    }
                } else {
                    g.bell.scorch = 0.0;
                }
                // The steam: a burst in his ring scalds him, unless he is in the air.
                if let Some((ring, true)) = bell.steam() {
                    let burst = (bell.clock / STEAM_EVERY) as u32;
                    if g.steam_hit != Some(burst) && !airborne && here.round() as u32 == ring {
                        g.steam_hit = Some(burst);
                        wounds.push((STEAM_DAMAGE, "SCALDED: LEAP THE STEAM"));
                    }
                }
                if ring_closed {
                    self.audio.play(Sfx::Clang);
                    self.view.shake(0.25);
                    self.floater(hp.x, hp.y - 1.4, "THE BRONZE CLOSES IN", 1.2, Some(BRONZE_HOT));
                }
                for (loss, why) in wounds {
                    let doom = self.brawl.as_mut().map(|b| b.ails.spend_doom()).unwrap_or(1.0);
                    self.wound_hero((loss * doom).round(), BRONZE_HOT);
                    if !why.is_empty() {
                        self.audio.play(Sfx::Fuse);
                        self.floater(hp.x, hp.y - 1.0, why, 1.0, Some(0xf0f0f0));
                    }
                }
                self.strike_the_head();
            }
            BellPhase::Lifted => {
                g.bell.clock += dt;
                if g.bell.clock > LIFT_SECONDS {
                    self.drop_bell_again();
                }
            }
            BellPhase::Won => {}
        }
    }

    /// His heavy blow lands where the serpent's head presses on the bronze: it recoils, and after
    /// three it uncoils and the bell lifts.
    fn strike_the_head(&mut self) {
        let Some(b) = self.brawl.as_ref() else { return };
        let heavy = matches!(b.hero.state, State::Attack { mv: MoveId::L3, .. });
        let landing = b.hero.active() == Some(MoveId::L3);
        let (hp, aim) = (self.hero.pos(), self.hero.aim());
        let Some(g) = self.grotto_mut() else { return };
        if !heavy {
            g.heavy_counted = false;
            return;
        }
        if !landing || g.heavy_counted {
            return;
        }
        g.heavy_counted = true;
        let patch = g.bell.patch();
        let (dx, dy) = (patch.x - hp.x, patch.y - hp.y);
        let d = (dx * dx + dy * dy).sqrt();
        let along = if d > 1e-6 { (dx * aim.x + dy * aim.y) / d } else { 1.0 };
        if d > HEAD_REACH || along < HEAD_ANGLE.cos() {
            return;
        }
        g.bell.blows += 1;
        let blows = g.bell.blows;
        // The head slides round to press somewhere else.
        g.bell.press();
        self.audio.play(Sfx::BossHit);
        self.view.shake(0.35);
        self.view.spark(&self.level.grid, patch.x, patch.y, 0xffffff);
        self.view.sparkle(&self.level.grid, patch.x, patch.y, 1.0, 0xfff4e0, 30);
        if blows >= HEAD_BLOWS {
            if let Some(g) = self.grotto_mut() {
                g.bell.phase = BellPhase::Lifted;
                g.bell.clock = 0.0;
            }
            self.audio.play(Sfx::BossRoar);
            self.juice.banner = Some(Banner {
                text: "THE SERPENT UNCOILS".into(),
                sub: "THE BELL LIFTS   DIVE THROUGH THE SEAM BENEATH IT, NOW".into(),
                color: 0xfff4e0,
                life: 1.6,
                max: 1.6,
            });
        } else {
            self.floater(patch.x, patch.y - 1.0, format!("IT RECOILS   {blows}/{HEAD_BLOWS}"), 1.2, Some(0xfff4e0));
        }
    }

    /// The bell drops over him: a disc of shallows inside, its bronze rim round it.
    fn drop_bell(&mut self) {
        let Some(g) = self.grotto_mut() else { return };
        let bell = &mut g.bell;
        bell.phase = BellPhase::Down;
        bell.clock = 0.0;
        bell.disc = BELL_DISC;
        bell.drops = 0;
        bell.blows = 0;
        bell.press();
        bell.scorch = 0.0;
        g.rings_lost = 0;
        g.steam_hit = None;
        let center = g.bell.center;
        self.set_bell_down(BELL_DISC);
        self.hero.place(center);
        self.hero.face(Dir::Up);
        self.view.shake(0.8);
        self.audio.play(Sfx::Bells);
        self.audio.play(Sfx::BossSlam);
        self.juice.banner = Some(Banner {
            text: "KIYOHIME'S BELL".into(),
            sub: "THE BELL FALLS, THE SERPENT COILS ROUND IT, AND THE BRONZE BEGINS TO HEAT".into(),
            color: BRONZE_HOT,
            life: 3.0,
            max: 3.0,
        });
    }

    /// Under the bell: the tiles of a disc of `disc` turned to shallows, its rim to bronze (each
    /// tile's ground kept to put back).
    fn set_bell_down(&mut self, disc: f64) {
        let Some(Run::Envy(g)) = self.run.as_mut() else { return };
        let grid = &mut self.level.grid;
        let (inside, rim) = g.bell.disc_tiles(disc);
        for &p in inside.iter().chain(rim.iter()) {
            if !g.bell.saved.iter().any(|s| s.0 == p) {
                g.bell.saved.push((p, grid.get(p.x, p.y)));
            }
        }
        for p in inside {
            grid.set(p.x, p.y, Tile::Open);
        }
        for p in rim {
            grid.set(p.x, p.y, Tile::Rock);
        }
    }

    /// The ground under the bell put back as it was.
    fn lift_bell_off(&mut self) {
        let Some(Run::Envy(g)) = self.run.as_mut() else { return };
        for (p, t) in g.bell.saved.drain(..) {
            self.level.grid.set(p.x, p.y, t);
        }
    }

    /// He missed the moment: the bell drops again, on a smaller disc.
    fn drop_bell_again(&mut self) {
        self.lift_bell_off();
        let Some(g) = self.grotto_mut() else { return };
        let bell = &mut g.bell;
        bell.phase = BellPhase::Down;
        bell.drops += 1;
        bell.disc = (BELL_DISC - bell.drops as f64).max(1.5);
        bell.clock = 0.0;
        bell.blows = 0;
        bell.press();
        bell.scorch = 0.0;
        let (disc, center) = (bell.disc, bell.center);
        g.rings_lost = 0;
        g.steam_hit = None;
        self.set_bell_down(disc);
        if self.grotto().is_some_and(|g| g.bell.dist(self.hero.pos()) > disc) {
            self.hero.place(center);
        }
        self.view.shake(0.7);
        self.audio.play(Sfx::BossSlam);
        self.juice.banner =
            Some(Banner { text: "THE BELL DROPS AGAIN".into(), sub: "ON A SMALLER DISC".into(), color: BRONZE_HOT, life: 2.0, max: 2.0 });
    }

    /// The safe circle is gone: the bell crushes him, and he surfaces at the mouth of its cavern.
    fn crush(&mut self) {
        self.lift_bell_off();
        let Some(g) = self.grotto_mut() else { return };
        g.bell.phase = BellPhase::Hanging;
        let mouth = g.bell.mouth;
        self.hero.place(mouth);
        self.hero.face(Dir::Left);
        self.view.shake(1.0);
        self.audio.play(Sfx::BossSlam);
        self.audio.play(Sfx::Bells);
        self.juice.banner = Some(Banner {
            text: "THE BELL CRUSHES YOU".into(),
            sub: "YOU SURFACE AT THE MOUTH OF THE CAVERN".into(),
            color: 0xff4d4d,
            life: 3.0,
            max: 3.0,
        });
        let loss = (self.hero.max_hp as f64 * CRUSH_SHARE).round();
        self.wound_hero(loss, 0xff4d4d);
    }

    /// Through the seam in the moment the bell was lifted: down to the Drowned Floor before the
    /// court's portal, real now, and open.
    fn through_the_seam(&mut self) {
        self.lift_bell_off();
        let Some(g) = self.grotto_mut() else { return };
        g.bell.phase = BellPhase::Won;
        let landing = g.bell.landing;
        g.thefts.clear();
        self.reach_stage(Some(THROUGH), landing);
        self.hero.place(landing);
        self.hero.altitude = 4.0;
        self.hero.face(Dir::Up);
        self.gate_open = true;
        self.audio.play(Sfx::Win);
        self.audio.play(Sfx::Portal);
        self.view.sparkle(&self.level.grid, landing.x as f64, landing.y as f64, 1.0, GREEN, 50);
        self.juice.banner = Some(Banner {
            text: "THE PORTAL IS REAL".into(),
            sub: "THROUGH THE SEAM UNDER THE BELL   LEVIATHAN'S COURT LIES BEYOND".into(),
            color: GREEN,
            life: 3.5,
            max: 3.5,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::realms::tests::{in_realm, STEP};
    use crate::game::Screen;
    use crate::logic::sins::Sin;
    use crate::logic::types::PotionKind;

    fn grotto() -> Game {
        let mut g = in_realm(Sin::Envy);
        // Nobody to fight unless a test wants them.
        g.enemies.clear();
        g.refresh_foe_views();
        g
    }

    fn run(g: &Game) -> &Grotto {
        g.grotto().unwrap()
    }

    fn step(g: &mut Game, seconds: f64) {
        for _ in 0..(seconds / STEP).round() as usize {
            g.update(STEP);
            g.hero.invuln = 1.0;
        }
    }

    #[test]
    fn the_grotto_floods_as_he_comes_in_and_the_far_portal_is_sealed() {
        let g = grotto();
        let r = run(&g);
        assert!(!g.gate_open, "sealed until the bell is survived");
        assert_eq!(g.hero.occupied(), r.base.arrival);
        for p in &r.fallen {
            assert_eq!(g.level.grid.get(p.x, p.y), Tile::Pit, "the ribs at {p:?} have fallen");
        }
        for p in &r.sealed {
            assert_eq!(g.level.grid.get(p.x, p.y), Tile::Rock);
        }
        assert_eq!(g.grotto_meters()[0], Meter::count("NAZAR", 0, NAZAR_BLUE));
    }

    #[test]
    fn a_dive_lands_beyond_where_the_drowned_floor_showed_and_moves_the_checkpoint_on() {
        let mut g = grotto();
        let pool = run(&g).pools.iter().copied().find(|q| q.stage == Some(1)).unwrap();
        g.hero.place(pool.at);
        g.update(STEP);
        let landed = g.hero.occupied();
        assert_eq!(landed, pool.to);
        assert_eq!(layer_of(landed), Layer::Drowned);
        // What showed beneath the pool lies REFRACTION tiles nearer the camera than where he lands.
        let shown = apparent(landed.v());
        assert_eq!((shown.x, shown.y - REFRACTION), (pool.at.x as f64, pool.at.y as f64));
        assert!((shown.y - pool.at.y as f64 - REFRACTION).abs() < 1e-9, "he lands {REFRACTION} tiles beyond what he saw");
        assert_eq!((run(&g).stage, run(&g).base.checkpoint), (1, pool.to));
        assert_eq!(g.level.hero_spawn, pool.to);
        // A lure moves nothing on.
        let lure = run(&g).pools.iter().copied().find(|q| q.stage.is_none()).unwrap();
        step(&mut g, 1.0);
        g.hero.place(lure.at);
        g.update(STEP);
        assert_eq!(g.hero.occupied(), lure.to);
        assert_eq!(run(&g).base.checkpoint, pool.to);
    }

    #[test]
    fn a_geyser_holds_him_four_seconds_and_sets_him_on_the_waterline() {
        let mut g = grotto();
        let vent = run(&g).vents.iter().copied().find(|v| v.stage == Some(2)).unwrap();
        g.hero.place(vent.at);
        g.update(STEP);
        assert!(run(&g).ride.is_some(), "the ride begins");
        g.input.hold_dir(Dir::Left, true);
        step(&mut g, RIDE_SECONDS - 0.5);
        assert_eq!(g.hero.occupied(), vent.at, "held fast on the vent, open to blows");
        step(&mut g, 0.7);
        g.input.hold_dir(Dir::Left, false);
        assert!(run(&g).ride.is_none());
        let up = g.hero.occupied();
        assert_eq!(layer_of(up), Layer::Waterline);
        assert!((up.x - vent.to.x).abs() <= 1 && (up.y - vent.to.y).abs() <= 1, "{up:?} by {:?}", vent.to);
        assert_eq!(run(&g).stage, 2);
    }

    #[test]
    fn falling_off_a_hump_into_deep_water_takes_him_down() {
        let mut g = grotto();
        let r = run(&g);
        let pit = *r.fallen.first().unwrap();
        g.hero.place(pit);
        g.update(STEP);
        assert_eq!(layer_of(g.hero.occupied()), Layer::Drowned);
        assert!(g.level.grid.is_walkable(g.hero.occupied().x, g.hero.occupied().y, false));
    }

    #[test]
    fn a_coil_wall_comes_and_goes_and_batters_him_aside() {
        let mut g = grotto();
        let k = run(&g).coils.iter().position(|c| c.kind == CoilKind::Wall).unwrap();
        let track = run(&g).coils[k].track();
        let mut seen_rock = false;
        let mut seen_open = false;
        for _ in 0..(20.0 / STEP) as usize {
            g.update(STEP);
            let t = track[track.len() / 2];
            match g.level.grid.get(t.x, t.y) {
                Tile::Rock => seen_rock = true,
                _ => seen_open = true,
            }
        }
        assert!(seen_rock && seen_open, "the passage is shut and open by turns");
        // Standing where it comes: he is put beside it.
        let covered = run(&g).covered[k].clone();
        let at = covered[2];
        g.hero.place(at);
        g.hero.invuln = 0.0;
        let hp = g.hero.hp;
        g.update(STEP);
        assert_ne!(g.hero.occupied(), at);
        assert!(g.level.grid.is_walkable(g.hero.occupied().x, g.hero.occupied().y, false));
        assert!(g.hero.hp < hp, "battered");
    }

    /// He stands where the Eye of his cavern looks, as it opens.
    fn under_the_eye(g: &mut Game) {
        let r = run(g);
        let eye = r.caverns[0].eye;
        let spot = P::new(eye.x, eye.y + 4);
        g.hero.place(spot);
        g.grotto_mut().unwrap().eye_clock = EYE_FIRST + EYE_OPEN / 2.0 - STEP;
        g.update(STEP);
    }

    #[test]
    fn the_eye_dooms_him_and_a_bead_takes_one_gaze_whole() {
        let mut g = grotto();
        under_the_eye(&mut g);
        assert!(run(&g).seen, "the Eye sees him");
        assert!(g.brawl.as_ref().unwrap().ails.doom > 0, "doomed");
        // With a bead on his belt the next gaze is taken by it, and it cracks.
        let mut g = grotto();
        g.grotto_mut().unwrap().nazar = 1;
        under_the_eye(&mut g);
        assert_eq!(run(&g).nazar, 0, "the bead cracked");
        assert!(!run(&g).seen);
        assert_eq!(g.brawl.as_ref().unwrap().ails.doom, 0);
        step(&mut g, 1.0);
        assert_eq!(g.brawl.as_ref().unwrap().ails.doom, 0, "it took the whole gaze");
        // A bead underfoot goes onto his belt.
        let bead = run(&g).beads[0].at;
        g.hero.place(bead);
        g.update(STEP);
        assert_eq!(run(&g).nazar, 1);
    }

    #[test]
    fn what_he_drinks_under_the_eye_a_native_keeps_until_he_kills_it() {
        let mut g = in_realm(Sin::Envy);
        g.belt = vec![PotionKind::Health];
        g.belt_index = 0;
        under_the_eye(&mut g);
        let hp = g.hero.hp;
        g.input.inject(Action::Drink);
        g.update(STEP);
        assert!(g.belt.is_empty(), "the bottle is gone");
        assert_eq!(g.hero.hp, hp, "and it did him no good");
        let theft = run(&g).thefts[0];
        assert_eq!(theft.what, Stolen::Potion(PotionKind::Health));
        // Slay the thief: the bottle comes back.
        let i = g.enemies.iter().position(|e| e.duel.as_ref().is_some_and(|d| d.id == theft.by)).unwrap();
        g.enemies[i].duel.as_mut().unwrap().hp = 1.0;
        g.stray_blow(i);
        step(&mut g, 0.5);
        assert_eq!(g.belt, vec![PotionKind::Health]);
        assert!(run(&g).thefts.is_empty());
    }

    #[test]
    fn diving_for_the_portal_drops_the_bell_and_three_heavy_blows_lift_it() {
        let mut g = grotto();
        let bell = run(&g).bell.clone();
        g.hero.place(bell.mouth);
        g.update(STEP);
        assert!(run(&g).whispered, "the water whispers");
        g.hero.place(bell.pool[0]);
        g.update(STEP);
        assert_eq!(run(&g).bell.phase, BellPhase::Down, "the bell drops over him");
        assert_eq!(g.hero.occupied(), bell.center);
        let (inside, rim) = run(&g).bell.disc_tiles(BELL_DISC);
        assert_eq!(inside.len(), 37, "a disc seven tiles across");
        assert!(rim.iter().all(|p| g.level.grid.get(p.x, p.y) == Tile::Rock), "sealed in bronze");
        // Three heavy blows on the white-hot patch.
        for n in 1..=HEAD_BLOWS {
            let patch = run(&g).bell.patch();
            let c = bell.center.v();
            let (dx, dy) = (patch.x - c.x, patch.y - c.y);
            let l = (dx * dx + dy * dy).sqrt();
            let stand = P::new((c.x + dx / l * (BELL_DISC - 0.7)).round() as i32, (c.y + dy / l * (BELL_DISC - 0.7)).round() as i32);
            g.hero.place(stand);
            g.hero.hp = g.hero.max_hp;
            g.hero.free_move = true;
            g.hero.heading = (patch.y - stand.y as f64).atan2(patch.x - stand.x as f64);
            let b = g.brawl.as_mut().unwrap();
            let startup = b.hero.table[2].startup;
            b.hero.state = State::Attack { mv: MoveId::L3, frame: startup };
            g.update(STEP);
            assert_eq!(run(&g).bell.blows, n, "blow {n} lands on the head");
            step(&mut g, 0.6);
        }
        assert_eq!(run(&g).bell.phase, BellPhase::Lifted, "the serpent uncoils and the bell lifts");
        // Dive through the seam in that second: the portal is real.
        g.hero.place(bell.center);
        g.update(STEP);
        assert_eq!(run(&g).bell.phase, BellPhase::Won);
        assert!(g.gate_open, "the far portal opens");
        assert_eq!(g.hero.occupied(), bell.landing);
        let exit = run(&g).base.exit;
        assert!((g.hero.occupied().y - exit.front().y).abs() <= 3 && g.hero.occupied().x == exit.front().x, "before the portal");
        assert_eq!(g.level.grid.get(bell.mouth.x, bell.mouth.y), Tile::Open, "the bell hangs again");
    }

    #[test]
    fn missing_the_moment_drops_it_again_smaller_and_the_lost_circle_crushes_him() {
        let mut g = grotto();
        g.cheat_grotto();
        assert_eq!(run(&g).bell.phase, BellPhase::Down, "F2: straight under the bell");
        {
            let r = g.grotto_mut().unwrap();
            r.bell.phase = BellPhase::Lifted;
            r.bell.clock = 0.0;
        }
        // He stands off the seam, at the edge of the disc, and the moment passes.
        let c = run(&g).bell.center;
        g.hero.place(P::new(c.x + 3, c.y));
        step(&mut g, LIFT_SECONDS + 0.2);
        assert_eq!(run(&g).bell.phase, BellPhase::Down);
        assert_eq!(run(&g).bell.disc, BELL_DISC - 1.0, "smaller");
        assert!(run(&g).bell.dist(g.hero.pos()) <= run(&g).bell.disc + 0.01, "inside it");
        // The scorch outside the safe circle.
        g.hero.place(P::new(c.x + 2, c.y));
        let hp = g.hero.hp;
        g.update(STEP);
        assert!(g.hero.hp < hp, "scorched on the outer tile");
        // Wait for the circle to go: the bell crushes him, and he surfaces at the mouth.
        g.hero.place(c);
        g.hero.hp = g.hero.max_hp;
        let max = g.hero.max_hp;
        let mut crushed = false;
        for _ in 0..(20.0 / STEP) as usize {
            g.update(STEP);
            if run(&g).bell.phase == BellPhase::Hanging {
                crushed = true;
                break;
            }
            // (Steam and scorch aside: only the crushing counts here.)
            g.hero.hp = max;
        }
        assert!(crushed);
        assert_eq!(g.hero.occupied(), run(&g).bell.mouth, "he surfaces at the mouth");
        assert!(g.hero.hp <= max - (max as f64 * CRUSH_SHARE).round() as i32 + 1, "a heavy blow");
        assert!(!g.gate_open);
        assert_eq!(g.screen, Screen::Playing);
        assert!(run(&g).bell.saved.is_empty(), "the ground under it put back");
    }

    #[test]
    fn steam_scalds_the_ring_it_bursts_in_but_a_leap_clears_it() {
        let mut g = grotto();
        g.cheat_grotto();
        let c = run(&g).bell.center;
        // Wait for a burst, standing in its ring.
        let mut scalded = false;
        for _ in 0..(STEAM_EVERY * 2.0 / STEP) as usize {
            if let Some((ring, _)) = run(&g).bell.steam() {
                g.hero.place(P::new(c.x, c.y + ring as i32));
            }
            let hp = g.hero.hp;
            g.update(STEP);
            if run(&g).steam_hit.is_some() {
                scalded = g.hero.hp < hp || scalded;
                break;
            }
        }
        assert!(scalded, "a burst in his ring scalds him");
        // The next burst: in the air as it bursts, he is not touched.
        let mut cleared = false;
        let first = run(&g).steam_hit;
        for _ in 0..(STEAM_EVERY * 3.0 / STEP) as usize {
            let before = run(&g).steam_hit;
            let index = Some((run(&g).bell.clock / STEAM_EVERY) as u32);
            if let Some((ring, _)) = run(&g).bell.steam().filter(|_| index != first) {
                let into = run(&g).bell.clock.rem_euclid(STEAM_EVERY);
                if into < STEAM_WARN - 0.1 {
                    g.hero.place(P::new(c.x, c.y + ring as i32));
                    g.hero.face(Dir::Down);
                } else if !g.hero.airborne() && g.hero.try_jump(&g.level.grid) {
                    // Off the floor just as the hiss ends.
                    let hp = g.hero.hp;
                    let burst_at = (run(&g).bell.clock / STEAM_EVERY) as u32;
                    while run(&g).bell.clock.rem_euclid(STEAM_EVERY) < STEAM_WARN + STEAM_BURST && g.hero.airborne() {
                        g.update(STEP);
                        g.hero.hp = g.hero.hp.max(hp - SCORCH_DAMAGE as i32 * 4);
                    }
                    assert_ne!(run(&g).steam_hit, Some(burst_at), "the leap cleared the burst");
                    cleared = true;
                    break;
                }
            }
            g.update(STEP);
            assert!(run(&g).steam_hit == before || run(&g).steam_hit.is_some());
        }
        assert!(cleared, "a burst came while he could leap it");
    }
}
