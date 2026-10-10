//! The Grey Mire in play (Sloth's realm, REALMS.md §4; laid out in `logic::realms::mire`).
//!
//! Its trap is Torpor: standing still fills it, and so do wading through mud and the tide and lying
//! in moss; moving on firm ground and every kill drain it; full, he lies down and sleeps. Yet the
//! hourglass rewards idling: the giant's hand curls toward him while he stands still, a finger
//! rising joint by joint across the moat, and withdraws once he has hurried for three seconds.
//! Moss heals, but every ten seconds rested the mire grows older (the tide a notch higher, a plank
//! rotted away, a servant of Hel climbed out of the bog). Duckweed hides bog holes, and the bog
//! gives him back where he last stood firm, poorer in health and wearier. A loud fight by the
//! giant's head (three foes on him, or a blessing called) makes him roll over in his sleep: under a
//! white surge of fog the whole land swings a quarter turn round the hourglass, and only the palm
//! stays put. Belphegor's shortcuts carry him, beautifully, halfway, and wait to be wound.
//!
//! At the hourglass's foot Hel's bed Kör makes the prince's offer: "Lie down. The door opens when
//! you wake." Lie down, or let Torpor fill, and he wakes at the realm's edge three hundred years
//! later, the door open but the mire older (`long_sleep`). Stand, and the hourglass turns: sixty
//! seconds of sand, the fingers withdrawn, a yawn every five seconds pushing him toward the bed
//! (a leap clears its crest; a raised guard holds him to a tile), and the servants wading in from
//! every side. When the sand runs out, the giant's hand opens the door. F2 brings him to the bed.

use super::super::brawl::{Brawl, Field};
use super::super::save::Stage;
use super::super::{Banner, Game, MenuItem, Screen};
use super::{Meter, Offer};
use crate::audio::Sfx;
use crate::logic::combat::ails::{Flash, FlashKind};
use crate::logic::combat::duelist::{Lore, RISE_FRAMES};
use crate::logic::combat::{Breed, Duelist};
use crate::logic::enemy::{spec_of, Enemy, EnemyKind, EnemyMode};
use crate::logic::level::parse_level;
use crate::logic::realms::mire::{
    in_palm, inside, radius, turn_dir, turn_p, turn_v, Cell, Mire, Phase, RideState, BRACED_PUSH, CURL, FIRM_DRAIN, HASTE_GRACE,
    HEAD_RANGE, KILL_DRAIN, LIE_DOWN, LULL, MOAT_R, MOSS_HEAL, MOSS_RATE, REST_AGE, RIDE_SPEED, SAND, SNORE_EVERY, STILL_RATE, TORPOR_MAX,
    TURN_REST, TURN_SECONDS, WADE_RATE, WIND, WITHDRAW, YAWN_EVERY, YAWN_PUSH, YAWN_TORPOR,
};
use crate::logic::realms::{weight, Run};
use crate::logic::sins::Sin;
use crate::logic::types::{dist_sq, Dir, Tile, CARDINALS, P, V};

/// Slate blue, Belphegor's colour; the sand's gold; the marsh-lights' blue.
const SLATE: u32 = 0x7a9ac8;
const SAND_GOLD: u32 = 0xe8c070;
const BOG: u32 = 0x3a3226;
/// He is standing still once he has stopped this long.
const SETTLE: f64 = 0.25;
/// Seconds he takes to fall asleep once Torpor is full.
const DOZE: f64 = 1.4;
/// A yawn carries him this fast (tiles a second).
const PUSH_SPEED: f64 = 7.5;
/// A bog hole takes this much health; the bog's Torpor.
const SINK_WOUND: f64 = 10.0;
const SINK_TORPOR: f64 = 15.0;
/// At most this many servants wade in at once in the Long Sleep.
const SERVANTS_AT_ONCE: usize = 6;

/// What one tick of the mire's rules leaves for the game to do.
#[derive(Default)]
struct Outcome {
    heal: i32,
    age: bool,
    sink: bool,
    hole: bool,
    snore: Option<f64>,
    turn_now: bool,
    turn_begin: bool,
    raise: bool,
    yawn: Option<bool>,
    sand_out: bool,
    ask: bool,
    sleep: bool,
    doze: bool,
    warn: bool,
    kills: u32,
    tiles: Vec<(P, bool)>,
    boarded: Option<usize>,
    stopped: Option<(usize, bool)>,
    wound: Option<usize>,
    carry: Option<V>,
    checkpoint: bool,
}

impl Game {
    fn mire(&self) -> Option<&Mire> {
        match self.run.as_ref() {
            Some(Run::Sloth(m)) => Some(m),
            _ => None,
        }
    }

    fn mire_mut(&mut self) -> Option<&mut Mire> {
        match self.run.as_mut() {
            Some(Run::Sloth(m)) => Some(m),
            _ => None,
        }
    }

    /// As the realm is entered (its packs raised, the camera above): Hel's servants named, the
    /// kills so far counted.
    pub(in crate::game) fn enter_mire(&mut self) {
        self.name_servants();
        let kills = self.brawl.as_ref().map(|b| b.stats.kills).unwrap_or(0);
        let (wait, charges) = (self.patron_wait, self.blessing.map(|b| b.charges).unwrap_or(0));
        if let Some(m) = self.mire_mut() {
            m.kills_seen = kills;
            m.prev_wait = wait;
            m.prev_charges = charges;
        }
    }

    /// Ganglati and Ganglöt, by turns ("slow-walker", both).
    fn name_servants(&mut self) {
        for (k, e) in self.enemies.iter_mut().filter(|e| e.kind() == EnemyKind::Ganglot).enumerate() {
            e.b.name = format!("{} {}", if k % 2 == 0 { "Ganglati" } else { "Ganglöt" }, k + 1);
        }
    }

    /// The realm's own rules each tick.
    pub(in crate::game) fn update_mire(&mut self, dt: f64) {
        if self.mire().is_none() {
            return;
        }
        let hp = self.hero.pos();
        let tile = self.hero.occupied();
        let airborne = self.hero.airborne();
        let blocking = self.hero.blocking;
        let kills = self.brawl.as_ref().map(|b| b.stats.kills).unwrap_or(0);
        let walkable_here = self.level.grid.is_walkable(tile.x, tile.y, false);
        let wait = self.patron_wait;
        let charges = self.blessing.map(|b| b.charges).unwrap_or(0);
        let gate_open = self.gate_open;
        let vel = self.hero.vel;
        let engaged = self.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.engaged() && dist_sq(d.pos, hp) < 64.0).count();
        let alive_raised = |m: &Mire, enemies: &[Enemy]| {
            enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.alive() && m.raised.contains(&d.id)).count()
        };
        let raised_now = self.mire().map(|m| alive_raised(m, &self.enemies)).unwrap_or(0);
        let tier = self.realm_tier();
        let mut out = Outcome::default();
        {
            let Some(m) = self.mire_mut() else { return };
            m.clock += dt;
            m.breath += dt;
            m.turn_rest -= dt;
            m.wave += dt;
            let riding = m.rides.iter().any(|r| matches!(r.state, RideState::Running { .. }));
            if let Some(t) = m.dozing.as_mut() {
                *t += dt;
                out.sleep = *t >= DOZE;
            }
            // ---- moving or standing still (being carried is standing still)
            // (A moment's pause, turning on his heel, is not yet standing still.)
            let stepping = !riding && m.dozing.is_none() && (airborne || vel > 0.4);
            if stepping {
                m.moving += dt;
                m.still = 0.0;
            } else {
                m.still += dt;
                if m.still >= SETTLE {
                    m.moving = 0.0;
                } else {
                    m.moving += dt;
                }
            }
            let moving = m.still < SETTLE;
            let cell = m.cell(tile);
            let wading = cell == Cell::Mud || m.flooded(tile);
            let lying = cell == Cell::Moss && !moving && !airborne && m.still >= LIE_DOWN && !riding;
            // ---- Torpor
            let lull = if m.in_sand() && m.still > LULL { 2.0 } else { 1.0 };
            let rate = if lying {
                MOSS_RATE
            } else if !stepping && moving {
                0.0
            } else if !moving {
                STILL_RATE * lull
            } else if wading && !airborne {
                WADE_RATE
            } else {
                -FIRM_DRAIN
            };
            m.torpor = (m.torpor + rate * dt).clamp(0.0, TORPOR_MAX);
            if kills > m.kills_seen {
                out.kills = kills - m.kills_seen;
                m.torpor = (m.torpor - KILL_DRAIN * out.kills as f64).max(0.0);
            }
            m.kills_seen = kills;
            if m.torpor >= 70.0 && !m.warned {
                m.warned = true;
                out.warn = true;
            } else if m.torpor < 50.0 {
                m.warned = false;
            }
            if m.torpor >= TORPOR_MAX && m.dozing.is_none() {
                m.dozing = Some(0.0);
                out.doze = true;
            }
            // ---- moss: it heals, and time passes
            if lying {
                m.heal_carry += MOSS_HEAL * dt;
                out.heal = m.heal_carry.floor() as i32;
                m.heal_carry -= out.heal as f64;
                m.rested += dt;
                if m.rested >= REST_AGE {
                    m.rested -= REST_AGE;
                    out.age = true;
                }
            }
            // ---- the hand: it curls toward the still, withdraws from the hasty, and is drawn in
            // altogether while the sand runs
            let before: Vec<Vec<bool>> = m.fingers.iter().map(|f| (0..f.len()).map(|i| f.up(i)).collect()).collect();
            if m.in_sand() {
                for f in m.fingers.iter_mut() {
                    f.reach = (f.reach - WITHDRAW * 4.0 * dt).max(0.0);
                }
            } else {
                if !moving && !riding && m.dozing.is_none() {
                    if let Some(k) = m.nearest_finger(hp) {
                        let f = &mut m.fingers[k];
                        f.reach = (f.reach + CURL * dt).min(f.len() as f64);
                    }
                }
                if m.moving > HASTE_GRACE {
                    for f in m.fingers.iter_mut() {
                        f.reach = (f.reach - WITHDRAW * dt).max(0.0);
                    }
                }
            }
            for (k, f) in m.fingers.iter().enumerate() {
                for (i, &joint) in f.joints.iter().enumerate() {
                    if f.up(i) != before[k][i] {
                        out.tiles.push((m.to_world(joint), f.up(i)));
                    }
                }
            }
            // ---- the bog takes him: a hole under the duckweed, or water where a plank or a joint was
            let on_hole = cell == Cell::Hole && dist_sq(hp, tile.v()) < 0.38 * 0.38;
            if !airborne && !riding && (on_hole || !walkable_here) {
                out.sink = true;
                out.hole = on_hole;
                m.torpor = (m.torpor + SINK_TORPOR).min(TORPOR_MAX);
            } else if !airborne && !riding && m.firm(tile) && cell != Cell::Hole {
                m.last_safe = tile.v();
                if radius(tile) > MOAT_R + 0.5 && !matches!(cell, Cell::Finger(..)) {
                    m.last_land = m.to_canon(tile);
                }
            }
            // His checkpoint moves to the shore he crossed from, once he sets foot on the palm.
            if in_palm(tile) && m.canon_checkpoint != m.last_land && !m.in_sand() {
                m.canon_checkpoint = m.last_land;
                out.checkpoint = true;
            }
            // ---- the giant snores, from his head, while he sleeps
            if matches!(m.phase, Phase::Wander | Phase::Asking) {
                m.snore -= dt;
                if m.snore <= 0.0 {
                    m.snore = SNORE_EVERY;
                    out.snore = Some(dist_sq(hp, m.head_world().v()).sqrt());
                }
            }
            // ---- a loud fight by his head turns him in his sleep
            let blessed = wait > m.prev_wait + 0.5 || charges < m.prev_charges;
            m.prev_wait = wait;
            m.prev_charges = charges;
            if m.turning.is_none() && m.turn_rest <= 0.0 && m.phase == Phase::Wander {
                let near_head = dist_sq(hp, m.head_world().v()) < HEAD_RANGE * HEAD_RANGE;
                if near_head && (engaged >= 3 || blessed) {
                    m.turning = Some((0.0, false));
                    out.turn_begin = true;
                }
            }
            if let Some((t, swung)) = m.turning.as_mut() {
                *t += dt;
                if !*swung && *t >= TURN_SECONDS * 0.5 {
                    *swung = true;
                    out.turn_now = true;
                }
                if *t >= TURN_SECONDS {
                    m.turning = None;
                    m.turn_rest = TURN_REST;
                }
            }
            // ---- Belphegor's shortcuts
            for k in 0..m.rides.len() {
                let station = m.to_world(m.rides[k].path[0]);
                let island = m.to_world(m.rides[k].path[m.rides[k].stop]);
                match m.rides[k].state {
                    RideState::Waiting if tile == station && !airborne && m.dozing.is_none() => {
                        m.rides[k].state = RideState::Running { to: m.rides[k].stop };
                        out.boarded = Some(k);
                    }
                    RideState::Running { to } => {
                        let r = &mut m.rides[k];
                        r.pos = (r.pos + RIDE_SPEED * dt).min(to as f64);
                        let at = r.at();
                        if r.pos >= to as f64 {
                            let stranded = to == r.stop;
                            r.state = if stranded { RideState::Stranded } else { RideState::Spent };
                            out.stopped = Some((k, stranded));
                        }
                        out.carry = Some(m.v_world(at));
                    }
                    RideState::Stranded if !moving && !airborne && dist_sq(hp, island.v()) < 1.6 * 1.6 => {
                        let r = &mut m.rides[k];
                        r.wound += dt;
                        if r.wound >= WIND {
                            r.state = RideState::Running { to: r.path.len() - 1 };
                            out.wound = Some(k);
                        }
                    }
                    _ => {}
                }
            }
            // ---- Kör, and the Long Sleep
            let to_bed = dist_sq(hp, m.bed.v()).sqrt();
            match m.phase {
                Phase::Wander if !gate_open && to_bed < 1.3 && !airborne && m.dozing.is_none() => {
                    m.phase = Phase::Asking;
                    out.ask = true;
                }
                Phase::Sand { left, yawn } => {
                    let (left, yawn) = (left - dt, yawn - dt);
                    m.phase = Phase::Sand { left, yawn: if yawn <= 0.0 { yawn + YAWN_EVERY } else { yawn } };
                    if yawn <= 0.0 {
                        out.yawn = Some(airborne);
                    }
                    m.rise_timer -= dt;
                    if m.rise_timer <= 0.0 {
                        m.rise_timer = (5.0 - 0.3 * tier as f64).max(2.8);
                        out.raise = raised_now < SERVANTS_AT_ONCE;
                    }
                    if left <= 0.0 {
                        m.phase = Phase::Done { yawn: yawn.max(0.5) };
                        out.sand_out = true;
                    }
                }
                Phase::Done { yawn } if in_palm(tile) => {
                    let yawn = yawn - dt;
                    m.phase = Phase::Done { yawn: if yawn <= 0.0 { yawn + YAWN_EVERY } else { yawn } };
                    if yawn <= 0.0 {
                        out.yawn = Some(airborne);
                    }
                }
                _ => {}
            }
            if let Some(over) = out.yawn {
                m.wave = 0.0;
                if in_palm(tile) && !over {
                    let to = V::new(m.bed.x as f64 - hp.x, m.bed.y as f64 - hp.y);
                    let l = (to.x * to.x + to.y * to.y).sqrt().max(1e-6);
                    m.push = if blocking { BRACED_PUSH } else { YAWN_PUSH };
                    m.push_dir = V::new(to.x / l, to.y / l);
                    m.torpor = (m.torpor + YAWN_TORPOR).min(TORPOR_MAX);
                }
            }
            // In the Long Sleep the bed is a fall into the bog hole beneath it: pushed or walked onto, he sleeps.
            if matches!(m.phase, Phase::Sand { .. } | Phase::Done { .. }) && to_bed < 0.55 && !airborne && m.dozing.is_none() {
                m.dozing = Some(DOZE);
                out.sleep = true;
            }
        }
        self.apply_mire(out, dt);
    }

    /// What a tick of the rules leaves to be done: the world changed, the feedback given.
    fn apply_mire(&mut self, out: Outcome, dt: f64) {
        let hp = self.hero.pos();
        for &(w, up) in &out.tiles {
            self.level.grid.set(w.x, w.y, if up { Tile::Open } else { Tile::Pit });
            self.level.grid.set_bridge(w.x, w.y, up);
        }
        if out.heal > 0 {
            self.hero.heal(out.heal);
        }
        if out.kills > 0 {
            self.floater(hp.x, hp.y - 1.7, format!("-{} TORPOR", out.kills as f64 * KILL_DRAIN), 1.0, Some(SLATE));
        }
        if out.warn {
            self.floater(hp.x, hp.y - 1.2, "HIS EYELIDS GROW HEAVY", 1.6, Some(SLATE));
            self.audio.play(Sfx::Soul);
        }
        if out.checkpoint {
            if let Some(m) = self.mire_mut() {
                m.sync_base();
            }
        }
        if out.sink {
            self.sink(out.hole);
        }
        if let Some(d) = out.snore {
            if d < 20.0 {
                // The snores come from the head: the nearer, the louder and the more the ground shakes.
                self.audio.play(Sfx::Growl);
                self.view.shake((0.14 * (1.0 - d / 20.0)) as f32);
                if d < 12.0 {
                    if let Some(h) = self.mire().map(|m| m.head_world()) {
                        self.floater(h.x as f64, h.y as f64 - 1.0, "SNNRRK...", 1.4, Some(0xa8b0b8));
                    }
                }
            }
        }
        if out.turn_begin {
            self.audio.play(Sfx::DoorGrind);
            self.audio.play(Sfx::Growl);
            self.view.shake(0.5);
            self.juice.banner = Some(Banner {
                text: "THE GIANT TURNS IN HIS SLEEP".into(),
                sub: "THE LAND SWINGS ROUND THE HOURGLASS".into(),
                color: SLATE,
                life: 2.8,
                max: 2.8,
            });
        }
        if out.turn_now {
            self.turn_land();
        }
        if out.age {
            self.mire_ages();
        }
        if let Some(k) = out.boarded {
            self.open_ride(k);
            let name = self.mire().map(|m| m.rides[k].kind.name()).unwrap_or("");
            self.floater(hp.x, hp.y - 1.3, format!("{name}: TO THE HOURGLASS!"), 1.8, Some(0xd8c8a0));
            self.audio.play(Sfx::Chain);
        }
        if let Some(to) = out.carry {
            let step = V::new(to.x - hp.x, to.y - hp.y);
            if self.hero.free_move {
                self.hero.shove(&self.level.grid, step);
            } else {
                self.hero.place(P::new(to.x.round() as i32, to.y.round() as i32));
            }
        }
        if let Some((k, stranded)) = out.stopped {
            self.open_ride(k);
            let hp = self.hero.pos();
            if stranded {
                self.floater(hp.x, hp.y - 1.4, "IT STOPS: A LOVELY MOSS ISLAND, OUT OF THE WIND", 2.4, Some(0x9ab08a));
                self.floater(hp.x, hp.y - 0.8, "WIND IT: TWENTY SECONDS STANDING STILL", 2.4, Some(0xd8c8a0));
                self.audio.play(Sfx::Denied);
            } else {
                self.floater(hp.x, hp.y - 1.2, "THE END OF THE LINE", 1.6, Some(0xd8c8a0));
            }
        }
        if let Some(k) = out.wound {
            self.open_ride(k);
            self.floater(hp.x, hp.y - 1.3, "WOUND: IT RUNS ON", 1.6, Some(0xd8c8a0));
            self.audio.play(Sfx::Chain);
        }
        if let Some(over) = out.yawn {
            self.audio.play(Sfx::Wind);
            self.view.shake(0.3);
            if over {
                self.floater(hp.x, hp.y - 1.2, "OVER THE CREST", 1.0, Some(0xd8e8ff));
            } else if self.mire().is_some_and(|m| m.push > 0.0) {
                self.floater(hp.x, hp.y - 1.2, if self.hero.blocking { "BRACED" } else { "THE GIANT YAWNS" }, 1.0, Some(SLATE));
            }
        }
        // The yawn's push, carried out over a moment.
        let push = self.mire().map(|m| (m.push, m.push_dir)).unwrap_or((0.0, V::new(0.0, 0.0)));
        if push.0 > 0.0 {
            let step = push.0.min(PUSH_SPEED * dt);
            if self.hero.free_move {
                self.hero.shove(&self.level.grid, V::new(push.1.x * step, push.1.y * step));
            } else if push.0 - step <= 0.0 {
                let whole = self.mire().map(|m| if self.hero.blocking { BRACED_PUSH } else { YAWN_PUSH.min(m.push + 2.0) }).unwrap_or(1.0);
                let to = V::new(hp.x + push.1.x * whole, hp.y + push.1.y * whole);
                let t = P::new(to.x.round() as i32, to.y.round() as i32);
                if self.level.grid.is_walkable(t.x, t.y, false) {
                    self.hero.place(t);
                }
            }
            if let Some(m) = self.mire_mut() {
                m.push = (m.push - step).max(0.0);
            }
        }
        if out.raise {
            if let Some(at) = self.servant_spot(true) {
                self.raise_servant(at);
            }
        }
        if out.sand_out {
            self.gate_open = true;
            self.audio.play(Sfx::Win);
            self.view.shake(0.4);
            self.juice.banner = Some(Banner {
                text: "THE SAND RUNS OUT".into(),
                sub: "THE GIANT'S HAND OPENS THE DOOR   GO, BEFORE HE YAWNS AGAIN".into(),
                color: SAND_GOLD,
                life: 3.5,
                max: 3.5,
            });
        }
        if out.ask {
            self.audio.play(Sfx::Bells);
            self.ask_offer();
        }
        if out.doze {
            self.audio.play(Sfx::Soul);
            self.juice.banner =
                Some(Banner { text: "HE LIES DOWN TO SLEEP".into(), sub: "JUST FOR A MOMENT".into(), color: SLATE, life: 2.0, max: 2.0 });
        }
        if out.sleep && self.mire().and_then(|m| m.dozing).is_some_and(|t| t >= DOZE) {
            self.long_sleep();
        }
    }

    /// The bog takes him: health and wakefulness lost, and he is given back where he last stood firm.
    fn sink(&mut self, hole: bool) {
        let hp = self.hero.pos();
        let back = self.mire().map(|m| m.last_safe).unwrap_or(hp);
        let checkpoint = self.mire().map(|m| m.base.checkpoint).unwrap_or(self.level.hero_spawn);
        let t = P::new(back.x.round() as i32, back.y.round() as i32);
        let to = if self.level.grid.is_walkable(t.x, t.y, false) { t } else { checkpoint };
        self.audio.play(Sfx::Drip);
        self.audio.play(Sfx::Hurt);
        self.view.dust(&self.level.grid, hp.x, hp.y, BOG, 18, 1.6);
        self.floater(hp.x, hp.y - 1.0, if hole { "A BOG HOLE UNDER THE DUCKWEED" } else { "THE BOG TAKES HIM" }, 1.6, Some(0x8ab0d8));
        self.wound_hero(SINK_WOUND, 0x6a8aa0);
        if self.hero.alive {
            self.hero.place(to);
        }
    }

    /// Ten minutes pass in the mire (he has rested ten seconds in moss): the tide rises a notch, a
    /// plank rots away, and a servant climbs out of the bog.
    fn mire_ages(&mut self) {
        let hp = self.hero.pos();
        let mut rot = None;
        if let Some(m) = self.mire_mut() {
            m.notch += 1;
            let planks: Vec<P> = (0..m.cells.len())
                .map(|i| P::new(i as i32 % crate::logic::realms::mire::SIZE, i as i32 / crate::logic::realms::mire::SIZE))
                .filter(|&p| m.may_rot(p))
                .collect();
            if !planks.is_empty() {
                rot = Some(planks[self.rng.index(planks.len())]);
            }
        }
        if let Some(p) = rot {
            if let Some(m) = self.mire_mut() {
                m.rotten[crate::logic::realms::mire::idx(p)] = true;
                let w = m.to_world(p);
                self.level.grid.set(w.x, w.y, Tile::Pit);
                self.level.grid.set_bridge(w.x, w.y, false);
                if let Some(m) = self.mire_mut() {
                    m.sync_base();
                }
            }
        }
        self.floater(hp.x, hp.y - 1.5, "TEN MINUTES PASS IN THE MIRE", 2.0, Some(0xa8b0b8));
        self.floater(hp.x, hp.y - 0.9, "THE TIDE RISES   A PLANK ROTS   A SERVANT CLIMBS OUT", 2.2, Some(SLATE));
        self.audio.play(Sfx::Bells);
        if let Some(at) = self.servant_spot(false) {
            self.raise_servant(at);
        }
    }

    /// Where a servant of Hel may climb out of the bog: in the Long Sleep at the palm's edge, away
    /// from him; else at the water's edge a little way off, out of sight in the fog.
    fn servant_spot(&mut self, palm: bool) -> Option<P> {
        let m = self.mire()?;
        let hp = self.hero.pos();
        let grid = &self.level.grid;
        let mut spots: Vec<P> = Vec::new();
        for y in 0..grid.height {
            for x in 0..grid.width {
                let p = P::new(x, y);
                if !grid.is_walkable(x, y, false) || m.cell(p) == Cell::Hole || m.cell(p) == Cell::Threshold {
                    continue;
                }
                let d = dist_sq(p.v(), hp).sqrt();
                let ok = if palm {
                    in_palm(p) && radius(p) >= 4.2 && d > 2.0
                } else {
                    (5.0..=9.0).contains(&d)
                        && !in_palm(p)
                        && CARDINALS.iter().any(|&dd| {
                            let q = p.step(dd);
                            inside(q) && matches!(m.cell(q), Cell::Pool | Cell::Moat)
                        })
                };
                if ok {
                    spots.push(p);
                }
            }
        }
        if spots.is_empty() {
            return None;
        }
        Some(spots[self.rng.index(spots.len())])
    }

    /// A servant of Hel climbs out of the bog at `at`, and wades toward him: slow, and it never stops.
    fn raise_servant(&mut self, at: P) {
        let tier = self.realm_tier();
        let hp = self.hero.pos();
        let Some(brawl) = self.brawl.as_mut() else { return };
        let id = brawl.next_id();
        let spec = spec_of(&self.level.grid, EnemyKind::Ganglot);
        let mut e = Enemy::new(&spec, at, self.level.data.enemy_speed);
        e.b.mode = EnemyMode::Chase;
        e.b.name = format!("{} {}", if id % 2 == 0 { "Ganglati" } else { "Ganglöt" }, id);
        let (fx, fy) = (hp.x - at.x as f64, hp.y - at.y as f64);
        let l = (fx * fx + fy * fy).sqrt().max(1e-6);
        let mut d = Duelist::bred(id, Breed::of(EnemyKind::Ganglot), at.v(), V::new(fx / l, fy / l));
        let (h, damage) = super::super::newgame::with_foe_weight(weight(0, tier), self.ng_plus);
        d.max_hp *= h;
        d.hp = d.max_hp;
        d.breed.damage *= damage;
        d.lore = Lore { rise: RISE_FRAMES, ..Lore::default() };
        d.rest = 60;
        e.duel = Some(Box::new(d));
        brawl.flashes.push(Flash::new(FlashKind::Grave, at.v()));
        self.view.add_enemy(&e);
        self.enemies.push(e);
        self.view.dust(&self.level.grid, at.x as f64, at.y as f64, BOG, 16, 1.4);
        if let Some(m) = self.mire_mut() {
            m.raised.push(id);
        }
    }

    /// Shortcut `k`'s way over the water firm where it runs now, water again where it has stopped.
    fn open_ride(&mut self, k: usize) {
        let Some(m) = self.mire() else { return };
        let r = &m.rides[k];
        let changes: Vec<(P, bool)> = (1..r.path.len() - 1).filter(|&i| i != r.stop).map(|i| (m.to_world(r.path[i]), r.open(i))).collect();
        for (w, open) in changes {
            self.level.grid.set(w.x, w.y, if open { Tile::Open } else { Tile::Pit });
            self.level.grid.set_bridge(w.x, w.y, open);
        }
    }

    /// The giant rolls over: everything but the palm swings a quarter turn round the hourglass,
    /// he and every foe with it, under the fog.
    fn turn_land(&mut self) {
        let Some(m) = self.mire_mut() else { return };
        m.turn = (m.turn + 1) % 4;
        if !in_palm(P::new(m.last_safe.x.round() as i32, m.last_safe.y.round() as i32)) {
            m.last_safe = turn_v(m.last_safe);
        }
        m.sync_base();
        let here = self.hero.occupied();
        if !in_palm(here) {
            let facing = turn_dir(self.hero.facing);
            self.hero.place(turn_p(here));
            self.hero.face(facing);
        }
        for e in self.enemies.iter_mut() {
            if let Some(d) = e.duel.as_deref_mut() {
                if !in_palm(P::new(d.pos.x.round() as i32, d.pos.y.round() as i32)) {
                    d.pos = turn_v(d.pos);
                    d.facing = V::new(-d.facing.y, d.facing.x);
                }
                if let Some(den) = d.den.as_mut() {
                    let (a, b) = (turn_v(den.lo), turn_v(den.hi));
                    den.lo = V::new(a.x.min(b.x), a.y.min(b.y));
                    den.hi = V::new(a.x.max(b.x), a.y.max(b.y));
                    den.home = turn_v(den.home);
                }
                let at = P::new(d.pos.x.round() as i32, d.pos.y.round() as i32);
                e.m.teleport(at);
            }
            if let Some(post) = e.post.filter(|&p| !in_palm(p)) {
                e.post = Some(turn_p(post));
            }
        }
        if let Some(b) = self.brawl.as_mut() {
            b.bolts.clear();
            b.hazards.clear();
            b.flashes.clear();
        }
        self.projectiles.clear();
        self.bombs.clear();
        self.relevel();
        self.rebuild_mire_view();
    }

    /// The level's grid made anew from the mire as it lies now (its door kept as it stands).
    fn relevel(&mut self) {
        let Some(m) = self.mire() else { return };
        let parsed = parse_level(&m.base.level(&self.level.data));
        let checkpoint = m.base.checkpoint;
        self.level.grid = parsed.grid;
        self.level.hero_spawn = checkpoint;
        if let Some(g) = self.gate {
            for dx in -1..=1 {
                self.level.grid.set_landmark(g.x + dx, g.y, true);
            }
            if self.door >= 1.0 {
                self.level.grid.set(g.x, g.y, Tile::Open);
                self.level.grid.set_elevation(g.x, g.y, self.level.grid.elevation(g.x, g.y + 1));
            }
        }
    }

    /// The whole scene built again (the land has turned, or aged).
    fn rebuild_mire_view(&mut self) {
        let started = web_time::Instant::now();
        self.rebuild_mire_view_now();
        self.begin_loading(self.level.data.name.clone(), started);
    }

    fn rebuild_mire_view_now(&mut self) {
        self.view.build_level(
            &self.level.grid,
            &self.enemies,
            &self.diamonds,
            self.theme(),
            self.garden.as_ref(),
            self.level.gate,
            self.vault.as_ref(),
            self.run.as_ref(),
        );
    }

    /// He sleeps the Long Sleep, and wakes at the realm's edge three hundred years later: the mire
    /// laid out again, older (its causeways rotten, its servants more, its tide higher), and the
    /// door open, far away. Mammon's restart, with interest.
    fn long_sleep(&mut self) {
        let age = self.mire().map(|m| m.age + 1).unwrap_or(1);
        self.audio.play(Sfx::Bells);
        self.audio.play(Sfx::Soul);
        self.load_realm(Sin::Sloth);
        self.age_mire(age);
        self.checkpoint(Stage::Realm(Sin::Sloth));
        self.juice.fade = 1.0;
        self.juice.banner = Some(Banner {
            text: if age == 1 { "THREE HUNDRED YEARS LATER".into() } else { format!("{} YEARS LATER", age * 300) },
            sub: "HE WAKES AT THE EDGE OF THE MIRE   THE DOOR STANDS OPEN, FAR AWAY".into(),
            color: SLATE,
            life: 4.5,
            max: 4.5,
        });
        self.enter(Screen::Ready);
    }

    /// The mire laid out `age` Long Sleeps on, its packs raised again, its door open (the sand ran
    /// out while he slept).
    fn age_mire(&mut self, age: u32) {
        let tier = self.realm_tier();
        let mut m = Mire::generate_aged(tier, age);
        m.phase = Phase::Done { yawn: YAWN_EVERY };
        let packs = m.base.packs.clone();
        self.run = Some(Run::Sloth(Box::new(m)));
        self.relevel();
        let mut brawl = self.brawl.take().unwrap_or_else(|| Brawl::new(Field::Vault));
        brawl.reset(self.hero_kit());
        self.enemies = self.raise_packs(&packs, tier, &mut brawl);
        self.brawl = Some(brawl);
        self.enter_mire();
        self.gate_open = true;
        self.rebuild_mire_view();
        self.reset_round();
    }

    /// The controls as the realm leaves them (`Game::steer_run`): mud and the tide slow him; carried
    /// by a shortcut, or falling asleep, he does not walk.
    pub(in crate::game) fn steer_mire(&mut self, wanted: Dir) -> Dir {
        let Some(m) = self.mire() else { return wanted };
        let tile = self.hero.occupied();
        let pace = if m.cell(tile) == Cell::Mud {
            0.5
        } else if m.flooded(tile) {
            0.6
        } else {
            1.0
        };
        let held = m.dozing.is_some() || m.rides.iter().any(|r| matches!(r.state, RideState::Running { .. }));
        self.hero.pace = pace;
        if held {
            self.hero.wish = V::new(0.0, 0.0);
            return Dir::None;
        }
        wanted
    }

    /// After a fall, as he rises at the checkpoint (and as the realm is first entered): wide awake,
    /// the hand withdrawn, the Long Sleep undone if it had begun, and the servants that climbed out
    /// sunk back. (What time has done to the mire stays done.)
    pub(in crate::game) fn reset_mire(&mut self) {
        let kills = self.brawl.as_ref().map(|b| b.stats.kills).unwrap_or(0);
        let gate_open = self.gate_open;
        let Some(m) = self.mire_mut() else { return };
        m.torpor = 0.0;
        m.still = 0.0;
        m.moving = 0.0;
        m.push = 0.0;
        m.dozing = None;
        m.warned = false;
        m.kills_seen = kills;
        m.wave = 99.0;
        if matches!(m.phase, Phase::Asking | Phase::Sand { .. }) {
            m.phase = Phase::Wander;
            m.turned_at = None;
        }
        if gate_open && m.phase == Phase::Wander {
            m.phase = Phase::Done { yawn: YAWN_EVERY };
        }
        if m.turning.is_some() {
            m.turning = None;
            m.turn_rest = TURN_REST * 0.5;
        }
        if m.clock > 0.0 {
            // (On first coming in the hand lies as it was laid out, open toward him.)
            for f in m.fingers.iter_mut() {
                f.reach = 0.0;
            }
        }
        for r in m.rides.iter_mut() {
            if let RideState::Running { to } = r.state {
                r.pos = to as f64;
                r.state = if to == r.stop { RideState::Stranded } else { RideState::Spent };
            }
        }
        m.last_safe = m.base.checkpoint.v();
        let raised = std::mem::take(&mut m.raised);
        m.sync_base();
        if !raised.is_empty() {
            self.enemies.retain(|e| e.duel.as_ref().is_none_or(|d| !raised.contains(&d.id)));
            self.refresh_foe_views();
        }
        self.hero.pace = 1.0;
        self.relevel();
    }

    /// F2: straight to the bed at the hourglass's foot, the hand curled to the palm.
    pub(in crate::game) fn cheat_mire(&mut self) {
        let Some(m) = self.mire_mut() else { return };
        for f in m.fingers.iter_mut() {
            f.reach = f.len() as f64;
        }
        m.torpor = 0.0;
        let bed = m.bed;
        m.sync_base();
        self.relevel();
        self.hero.place(P::new(bed.x, bed.y + 1));
        self.hero.face(Dir::Up);
        self.view.aim_along(Dir::Up);
    }

    /// What the HUD shows of the realm's rules: his Torpor, the sand while it runs, a shortcut's
    /// winding, and the years he has slept.
    pub(in crate::game) fn mire_meters(&self) -> Vec<Meter> {
        let Some(m) = self.mire() else { return Vec::new() };
        let mut out = vec![Meter::bar("TORPOR", m.torpor as f32, TORPOR_MAX as f32, SLATE).warning(m.torpor >= 70.0)];
        if let Some(left) = m.sand_left() {
            out.push(Meter::count("SAND", left.max(0.0).ceil() as u32, SAND_GOLD).warning(left <= 10.0));
        }
        let hp = self.hero.pos();
        if let Some(r) =
            m.rides.iter().find(|r| r.state == RideState::Stranded && r.wound > 0.0 && dist_sq(m.to_world(r.path[r.stop]).v(), hp) < 9.0)
        {
            out.push(Meter::bar("WINDING", r.wound as f32, WIND as f32, 0xc8a870));
        }
        if m.age > 0 {
            out.push(Meter::count("YEARS SLEPT", m.age * 300, 0xa8b0b8));
        }
        out
    }

    /// The offer standing before him (the bed at the hourglass's foot), if one is.
    pub(in crate::game) fn mire_offer(&self) -> Option<Offer> {
        let m = self.mire()?;
        (m.phase == Phase::Asking).then(|| Offer {
            title: "KÖR, THE SICK-BED".into(),
            lines: vec![
                "Lie down. The door opens when you wake.".into(),
                "The curtains gleam, the pillows are soft, and the sand has stopped.".into(),
            ],
            choices: vec![MenuItem::Choice("LIE DOWN"), MenuItem::Choice("STAND")],
            color: SLATE,
        })
    }

    /// His answer to it. Lie down: the hourglass turns over, and he sleeps. Stand: the hourglass
    /// turns over all the same, and the Long Sleep begins.
    pub(in crate::game) fn answer_mire(&mut self, choice: MenuItem) {
        self.enter(Screen::Playing);
        let clock = self.mire().map(|m| m.clock).unwrap_or(0.0);
        if choice == MenuItem::Choice("LIE DOWN") {
            if let Some(m) = self.mire_mut() {
                m.phase = Phase::Wander;
                m.turned_at = Some(clock);
            }
            self.long_sleep();
            return;
        }
        if let Some(m) = self.mire_mut() {
            m.phase = Phase::Sand { left: SAND, yawn: YAWN_EVERY };
            m.turned_at = Some(clock);
            m.rise_timer = 1.5;
            m.still = 0.0;
        }
        self.audio.play(Sfx::Toll);
        self.audio.play(Sfx::Stinger);
        self.view.shake(0.45);
        self.juice.banner = Some(Banner {
            text: "THE HOURGLASS TURNS OVER".into(),
            sub: "THE DOOR OPENS WHEN THE SAND RUNS OUT   KEEP MOVING   STAY AWAKE".into(),
            color: SAND_GOLD,
            life: 3.5,
            max: 3.5,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::realms::tests::{in_realm, STEP};
    use crate::game::READY_SECONDS;
    use crate::input::Action;
    use crate::logic::realms::mire::{Phase, PALM_R, SIZE};

    fn mire(g: &Game) -> &Mire {
        g.mire().expect("the Grey Mire")
    }

    fn run(g: &mut Game, seconds: f64) {
        for _ in 0..(seconds / STEP).round() as usize {
            g.update(STEP);
        }
    }

    /// The mire with no foes in it, so nothing interrupts what is being tried.
    fn quiet() -> Game {
        let mut g = in_realm(Sin::Sloth);
        g.enemies.clear();
        g.refresh_foe_views();
        g
    }

    /// Firm low peat by the arrival, with room to pace east and west.
    fn peat_walk(g: &Game) -> (P, P) {
        let a = mire(g).base.arrival;
        let (p, q) = (P::new(a.x + 2, a.y), P::new(a.x + 3, a.y));
        assert!((-1..=4).all(|dx| mire(g).cell(P::new(a.x + dx, a.y)) == Cell::Peat), "peat by the arrival");
        (p, q)
    }

    #[test]
    fn the_grey_mire_is_entered_with_its_packs_asleep_and_the_hand_open_toward_him() {
        let g = in_realm(Sin::Sloth);
        let m = mire(&g);
        assert_eq!(g.hero.occupied(), m.base.arrival);
        assert!(!g.gate_open, "sealed until the Long Sleep is survived");
        let open = m.nearest_finger(m.base.arrival.v()).unwrap();
        assert_eq!(m.fingers[open].reach, m.fingers[open].len() as f64, "the nearest finger reaches the shore");
        assert!(g.enemies.len() >= 18, "{} foes", g.enemies.len());
        assert!(g
            .enemies
            .iter()
            .any(|e| e.kind() == EnemyKind::Ganglot && (e.b.name.starts_with("Ganglati") || e.b.name.starts_with("Ganglöt"))));
        assert_eq!(g.run_meters()[0].label, "TORPOR");
        for (k, f) in m.fingers.iter().enumerate() {
            for &j in &f.joints {
                let w = m.to_world(j);
                let t = if k == open { Tile::Open } else { Tile::Pit };
                assert_eq!(g.level.grid.get(w.x, w.y), t, "a withdrawn finger is under the water");
            }
        }
    }

    /// Paces him back and forth (free movement) for `seconds`, turning every half second.
    fn pace(g: &mut Game, seconds: f64) {
        g.hero.free_move = true;
        for i in 0..(seconds / STEP).round() as usize {
            let right = ((i as f64 * STEP / 0.5) as usize).is_multiple_of(2);
            g.input.hold_dir(Dir::Right, right);
            g.input.hold_dir(Dir::Left, !right);
            g.update(STEP);
        }
        g.input.clear_direction();
    }

    #[test]
    fn torpor_fills_while_he_stands_still_and_drains_while_he_moves_or_kills() {
        let mut g = quiet();
        let (a, _) = peat_walk(&g);
        g.hero.place(a);
        run(&mut g, 3.0);
        let t = mire(&g).torpor;
        assert!((25.0..=31.0).contains(&t), "still for 3 s: {t}");
        // Pacing on firm peat (the tide out) drains it.
        g.mire_mut().unwrap().breath = 0.0;
        pace(&mut g, 2.0);
        let after = mire(&g).torpor;
        assert!(after < t - 8.0, "moving drains it: {t} then {after}");
        // Kills drain it by twenty each.
        g.mire_mut().unwrap().torpor = 60.0;
        let kills = g.brawl.as_ref().unwrap().stats.kills;
        g.brawl.as_mut().unwrap().stats.kills = kills + 2;
        g.update(STEP);
        let t = mire(&g).torpor;
        assert!((19.0..=22.0).contains(&t), "two kills: {t}");
    }

    #[test]
    fn full_torpor_is_the_long_sleep_and_he_wakes_at_the_edge_in_an_older_mire() {
        let mut g = quiet();
        let arrival = mire(&g).base.arrival;
        let (a, _) = peat_walk(&g);
        g.hero.place(a);
        g.mire_mut().unwrap().torpor = 97.0;
        run(&mut g, 0.6);
        assert!(mire(&g).dozing.is_some(), "he lies down");
        run(&mut g, DOZE + 0.2);
        assert_eq!(g.screen, Screen::Ready, "he sleeps, and wakes");
        let m = mire(&g);
        assert_eq!(m.age, 1, "three hundred years later");
        assert!(g.gate_open, "the door stands open");
        assert!(m.rotten.iter().any(|&r| r), "the causeways rot");
        assert_eq!(g.hero.occupied(), arrival, "at the edge of the mire");
        assert_eq!(m.torpor, 0.0);
        assert!(g.run_meters().iter().any(|mt| mt.label == "YEARS SLEPT" && mt.value == 300.0));
        let servants = g.enemies.iter().filter(|e| e.kind() == EnemyKind::Ganglot).count();
        let young = Mire::generate(g.realm_tier()).base.packs.iter().flat_map(|p| &p.foes).filter(|f| f.0 == EnemyKind::Ganglot).count();
        assert!(servants >= young * 2, "{servants} servants now, {young} before");
        // And again: it only grows worse.
        run(&mut g, READY_SECONDS + 0.1);
        g.enemies.clear();
        g.refresh_foe_views();
        g.mire_mut().unwrap().torpor = 99.9;
        run(&mut g, 0.3 + DOZE + 0.2);
        assert_eq!(mire(&g).age, 2);
        assert!(mire(&g).notch >= 2);
    }

    #[test]
    fn the_hand_curls_toward_the_still_and_withdraws_from_the_hasty() {
        let mut g = quiet();
        for f in g.mire_mut().unwrap().fingers.iter_mut() {
            f.reach = 0.0;
        }
        g.mire_mut().unwrap().sync_base();
        g.relevel();
        // He stands on the shore by the third finger's tip.
        let (k, shore) = {
            let m = mire(&g);
            let k = 2;
            let tip = m.to_world(*m.fingers[k].joints.last().unwrap());
            let shore = CARDINALS.iter().map(|&d| tip.step(d)).find(|&q| radius(q) > MOAT_R && m.firm(q)).unwrap();
            (k, shore)
        };
        g.hero.place(shore);
        run(&mut g, 2.0);
        let r = mire(&g).fingers[k].reach;
        assert!((2.6..=3.1).contains(&r), "curled 1.5 joints a second: {r}");
        assert!(mire(&g).fingers.iter().enumerate().all(|(i, f)| i == k || f.reach == 0.0), "only the finger nearest him");
        let n = mire(&g).fingers[k].len();
        g.mire_mut().unwrap().torpor = 0.0;
        run(&mut g, n as f64 / 1.5);
        assert_eq!(mire(&g).fingers[k].reach, n as f64, "it reaches him");
        let first = mire(&g).to_world(mire(&g).fingers[k].joints[0]);
        assert_eq!(g.level.grid.get(first.x, first.y), Tile::Open, "a way of joints over the moat");
        let seen = crate::logic::pathing::reachable(&g.level.grid, shore, false, false);
        assert!(seen.contains(&mire(&g).bed), "to the bed");
        // Hurrying: three seconds' grace, then the hand withdraws a joint a second.
        g.mire_mut().unwrap().torpor = 0.0;
        pace(&mut g, 5.0);
        let r = mire(&g).fingers[k].reach;
        assert!(r < n as f64 - 1.0 && r > n as f64 - 3.5, "withdrawn about two joints: {r} of {n}");
        let tip = mire(&g).to_world(*mire(&g).fingers[k].joints.last().unwrap());
        assert_eq!(g.level.grid.get(tip.x, tip.y), Tile::Pit, "its tip back under the water");
    }

    #[test]
    fn a_loud_fight_by_his_head_turns_the_land_round_the_hourglass_which_stays_put() {
        let mut g = in_realm(Sin::Sloth);
        let head = mire(&g).head_world();
        let bed = mire(&g).bed;
        let exit = mire(&g).base.exit;
        // Beside the head, three foes on him.
        let spot = (0..SIZE)
            .flat_map(|y| (0..SIZE).map(move |x| P::new(x, y)))
            .filter(|&p| g.level.grid.is_walkable(p.x, p.y, false) && mire(&g).cell(p) != Cell::Hole)
            .min_by_key(|p| (p.x - head.x).pow(2) + (p.y - head.y).pow(2) + if mire(&g).cell(*p) == Cell::Moss { 1000 } else { 0 })
            .unwrap();
        g.hero.place(spot);
        let mut woken = 0;
        for e in g.enemies.iter_mut() {
            if woken < 3 {
                if let Some(d) = e.duel.as_deref_mut() {
                    d.pos = V::new(spot.x as f64 + 1.0, spot.y as f64);
                    if let Some(den) = d.den.as_mut() {
                        den.awake = true;
                        den.lo = V::new(spot.x as f64 - 3.0, spot.y as f64 - 3.0);
                        den.hi = V::new(spot.x as f64 + 3.0, spot.y as f64 + 3.0);
                    }
                    woken += 1;
                }
            }
        }
        let far = g.enemies.iter().filter_map(|e| e.duel.as_deref()).map(|d| d.pos).next_back().unwrap();
        g.hero.invuln = 99.0;
        g.update(STEP);
        assert!(mire(&g).turning.is_some(), "the giant stirs");
        // (Blows landing hold the world a beat: run till the land has swung.)
        for _ in 0..(6.0 / STEP) as usize {
            if mire(&g).turn == 1 {
                break;
            }
            g.hero.invuln = 99.0;
            g.hero.hp = g.hero.max_hp;
            g.update(STEP);
        }
        let m = mire(&g);
        assert_eq!(m.turn, 1, "a quarter turn");
        assert_eq!(m.head_world(), turn_p(head), "the head swung round");
        assert_eq!((m.bed, m.base.exit), (bed, exit), "the hourglass, its door and the bed stay put");
        assert_eq!(g.level.gate, Some(exit.tile));
        let me = g.hero.occupied();
        assert!((me.x - turn_p(spot).x).abs() <= 1 && (me.y - turn_p(spot).y).abs() <= 1, "he swung with the land: {me:?}");
        assert!(g.level.grid.is_walkable(me.x, me.y, false));
        let last = g.enemies.iter().filter_map(|e| e.duel.as_deref()).map(|d| d.pos).next_back().unwrap();
        assert!(dist_sq(last, turn_v(far)) < 4.0, "and every foe");
        // The grid is the turned mire.
        let (rows, _) = mire(&g).rows();
        for (y, row) in rows.iter().enumerate() {
            for (x, c) in row.chars().enumerate() {
                let t = g.level.grid.get(x as i32, y as i32);
                let firm = matches!(c, ' ' | '=' | 'P' | 'G');
                assert_eq!(firm, t != Tile::Rock && t != Tile::Pit, "{x},{y}");
            }
        }
        run(&mut g, TURN_SECONDS);
        assert!(mire(&g).turning.is_none());
    }

    #[test]
    fn a_blessing_by_his_head_turns_the_land_too() {
        let mut g = quiet();
        let head = mire(&g).head_world();
        let spot = (0..SIZE)
            .flat_map(|y| (0..SIZE).map(move |x| P::new(x, y)))
            .filter(|&p| g.level.grid.is_walkable(p.x, p.y, false) && mire(&g).cell(p) == Cell::Peat)
            .min_by_key(|p| (p.x - head.x).pow(2) + (p.y - head.y).pow(2))
            .unwrap();
        g.hero.place(spot);
        g.update(STEP);
        assert!(mire(&g).turning.is_none(), "a quiet hero lets him sleep");
        g.patron_wait = 30.0;
        g.update(STEP);
        assert!(mire(&g).turning.is_some(), "a blessing wakes him");
    }

    #[test]
    fn moss_heals_but_ages_the_mire() {
        let mut g = quiet();
        let moss = (0..SIZE)
            .flat_map(|y| (0..SIZE).map(move |x| P::new(x, y)))
            .find(|&p| mire(&g).cell(p) == Cell::Moss && g.level.grid.is_walkable(p.x, p.y, false) && !in_palm(p) && !mire(&g).flooded(p))
            .unwrap();
        g.hero.place(moss);
        g.hero.hp = 20;
        let notch = mire(&g).notch;
        let n = g.enemies.len();
        run(&mut g, 3.0);
        assert!(g.hero.hp >= 30, "it heals: {}", g.hero.hp);
        let t = mire(&g).torpor;
        assert!(t > 55.0, "and lying in moss brings sleep fast: {t}");
        // Ten seconds rested, in all: the tide rises, a plank rots, a servant climbs out.
        g.mire_mut().unwrap().torpor = 0.0;
        g.mire_mut().unwrap().rested = REST_AGE - 0.05;
        run(&mut g, 0.2);
        let m = mire(&g);
        assert_eq!(m.notch, notch + 1);
        assert_eq!(m.rotten.iter().filter(|&&r| r).count(), 1);
        assert_eq!(g.enemies.len(), n + 1);
        assert_eq!(g.enemies.last().unwrap().kind(), EnemyKind::Ganglot);
        assert_eq!(m.raised.len(), 1);
        let rot = (0..m.rotten.len()).find(|&i| m.rotten[i]).unwrap();
        let w = m.to_world(P::new(rot as i32 % SIZE, rot as i32 / SIZE));
        assert_eq!(g.level.grid.get(w.x, w.y), Tile::Pit, "the plank is gone");
    }

    #[test]
    fn a_bog_hole_under_the_duckweed_takes_health_and_gives_him_back() {
        let mut g = quiet();
        let hole = (0..SIZE).flat_map(|y| (0..SIZE).map(move |x| P::new(x, y))).find(|&p| mire(&g).cell(p) == Cell::Hole).unwrap();
        let beside = CARDINALS.iter().map(|&d| hole.step(d)).find(|&q| mire(&g).firm(q) && mire(&g).cell(q) != Cell::Hole).unwrap();
        assert!(g.level.grid.is_walkable(hole.x, hole.y, false), "it looks like firm ground");
        g.hero.place(beside);
        g.update(STEP);
        let hp = g.hero.hp;
        g.hero.place(hole);
        g.update(STEP);
        assert_eq!(g.hero.occupied(), beside, "given back where he stood firm");
        assert!(g.hero.hp < hp);
        assert!(mire(&g).torpor >= 15.0);
    }

    #[test]
    fn a_shortcut_carries_him_halfway_and_must_be_wound() {
        let mut g = quiet();
        g.hero.free_move = true;
        let (station, island, end) = {
            let m = mire(&g);
            let r = &m.rides[0];
            (m.to_world(r.path[0]), m.to_world(r.path[r.stop]), m.to_world(*r.path.last().unwrap()))
        };
        g.hero.place(station);
        g.update(STEP);
        assert!(matches!(mire(&g).rides[0].state, RideState::Running { .. }), "aboard");
        run(&mut g, 1.0);
        assert!(mire(&g).torpor > 6.0, "a ride is a rest");
        for _ in 0..(12.0 / STEP) as usize {
            g.mire_mut().unwrap().torpor = 0.0;
            g.update(STEP);
        }
        assert_eq!(mire(&g).rides[0].state, RideState::Stranded, "it stops halfway");
        let me = g.hero.pos();
        assert!(dist_sq(me, island.v()) < 1.0, "on the moss island: {me:?} {island:?}");
        // Winding it is standing still: twenty seconds of it (wakefulness kept up here by hand; time
        // passing in the moss brings servants out of the bog, kept off him here).
        for _ in 0..((WIND + 0.5) / STEP) as usize {
            g.mire_mut().unwrap().torpor = 0.0;
            g.hero.invuln = 99.0;
            g.hero.hp = g.hero.max_hp;
            g.update(STEP);
        }
        assert!(matches!(mire(&g).rides[0].state, RideState::Running { .. } | RideState::Spent), "wound, it runs on");
        for _ in 0..(12.0 / STEP) as usize {
            g.mire_mut().unwrap().torpor = 0.0;
            g.hero.invuln = 99.0;
            g.hero.hp = g.hero.max_hp;
            g.update(STEP);
        }
        assert_eq!(mire(&g).rides[0].state, RideState::Spent);
        assert_eq!(g.screen, Screen::Playing);
        assert!(dist_sq(g.hero.pos(), end.v()) < 1.5, "by the moat: {:?} {:?} {:?}", g.hero.pos(), end, mire(&g).rides[0].at());
    }

    #[test]
    fn the_bed_asks_lie_down_first_and_lying_down_is_the_long_sleep() {
        let mut g = in_realm(Sin::Sloth);
        g.cheat_to_gate();
        g.update(STEP);
        assert_eq!(g.screen, Screen::Toll, "before the bed, the question");
        let offer = g.run_offer().expect("Kör's offer");
        assert_eq!(offer.lines[0], "Lie down. The door opens when you wake.");
        assert_eq!(g.menu_items(), vec![MenuItem::Choice("LIE DOWN"), MenuItem::Choice("STAND")]);
        assert_eq!(g.menu_index, 0, "lying down is what it offers first");
        g.input.inject(Action::Confirm);
        g.update(STEP);
        assert_eq!(g.screen, Screen::Ready);
        assert_eq!(mire(&g).age, 1, "he slept three hundred years");
        assert!(g.gate_open);
        assert_eq!(g.hero.occupied(), mire(&g).base.arrival);
    }

    /// The Long Sleep under way, foes cleared, he on the palm (free movement).
    fn in_the_sand() -> Game {
        let mut g = quiet();
        g.hero.free_move = true;
        g.cheat_to_gate();
        g.update(STEP);
        assert_eq!(g.screen, Screen::Toll);
        g.input.nav = Some(Dir::Down);
        g.update(STEP);
        g.input.inject(Action::Confirm);
        g.update(STEP);
        assert_eq!(g.screen, Screen::Playing);
        assert!(mire(&g).in_sand(), "the hourglass turns over");
        g
    }

    /// Runs `seconds`, keeping him awake and unhurt, no servant climbing out.
    fn awake(g: &mut Game, seconds: f64) {
        for _ in 0..(seconds / STEP).round() as usize {
            g.mire_mut().unwrap().torpor = 0.0;
            g.mire_mut().unwrap().rise_timer = 99.0;
            g.hero.invuln = 99.0;
            g.update(STEP);
        }
    }

    #[test]
    fn standing_turns_the_hourglass_and_the_yawns_push_him_toward_the_bed() {
        let mut g = in_the_sand();
        assert!(g.run_meters().iter().any(|m| m.label == "SAND" && m.value == 60.0));
        // He stands east of the bed: a yawn carries him three tiles toward it.
        let bed = mire(&g).bed;
        let at = P::new(bed.x + 4, bed.y);
        g.hero.place(at);
        g.hero.invuln = 99.0;
        let wait = match mire(&g).phase {
            Phase::Sand { yawn, .. } => yawn,
            _ => unreachable!(),
        };
        awake(&mut g, wait - 0.05);
        while mire(&g).push == 0.0 {
            g.mire_mut().unwrap().torpor = 0.0;
            g.update(STEP);
        }
        assert!(mire(&g).torpor >= YAWN_TORPOR, "a yawn brings sleep");
        awake(&mut g, 0.6);
        let x = g.hero.pos().x;
        assert!((x - (bed.x as f64 + 1.0)).abs() < 0.35, "pushed three tiles: {x}");
        // Braced on guard, he gives a tile only.
        g.hero.place(at);
        g.input.inject(Action::Block);
        awake(&mut g, YAWN_EVERY);
        let x = g.hero.pos().x;
        assert!(x > bed.x as f64 + 2.4, "braced: {x}");
        g.input.inject_release(Action::Block);
        // Fingers drawn in: the island is cut off.
        assert!(mire(&g).fingers.iter().all(|f| f.reach == 0.0));
    }

    #[test]
    fn a_leap_clears_the_yawns_crest() {
        let mut g = in_the_sand();
        let bed = mire(&g).bed;
        let at = P::new(bed.x, bed.y + 3);
        g.hero.place(at);
        g.hero.face(Dir::Down);
        g.hero.invuln = 99.0;
        let wait = match mire(&g).phase {
            Phase::Sand { yawn, .. } => yawn,
            _ => unreachable!(),
        };
        awake(&mut g, wait - 0.1);
        assert!(g.hero.try_jump(&g.level.grid));
        awake(&mut g, 0.3);
        assert_eq!(mire(&g).push, 0.0, "airborne, the wave passes under him");
    }

    #[test]
    fn surviving_the_sand_opens_the_door_and_servants_wade_in_meanwhile() {
        let mut g = in_the_sand();
        let bed = mire(&g).bed;
        let door = mire(&g).base.exit.tile;
        g.hero.place(P::new(bed.x - 3, bed.y - 1));
        for _ in 0..((SAND + 12.0) / STEP) as usize {
            if g.gate_open {
                break;
            }
            g.hero.invuln = 99.0;
            g.hero.hp = g.hero.max_hp;
            {
                let m = g.mire_mut().unwrap();
                m.torpor = 0.0;
                m.push = 0.0;
            }
            g.update(STEP);
        }
        assert!(g.gate_open, "the giant's hand opens the door: {:?} {:?} {:?}", g.screen, mire(&g).phase, g.hero.pos());
        assert!(matches!(mire(&g).phase, Phase::Done { .. }));
        let servants = g.enemies.iter().filter(|e| e.kind() == EnemyKind::Ganglot && e.duel.as_ref().is_some_and(|d| d.alive())).count();
        assert!(servants >= 3, "{servants} servants waded in");
        assert!(g
            .enemies
            .iter()
            .filter_map(|e| e.duel.as_deref())
            .all(|d| radius(P::new(d.pos.x.round() as i32, d.pos.y.round() as i32)) <= PALM_R + 1.0));
        // Through the door to Belphegor's court.
        g.enemies.clear();
        g.refresh_foe_views();
        g.hero.place(P::new(door.x, door.y + 1));
        run(&mut g, 3.0);
        assert!(g.door >= 1.0);
        g.hero.m.teleport(door);
        g.update(STEP);
        assert_eq!(g.screen, Screen::Cleared);
    }

    #[test]
    fn falling_in_the_sand_undoes_it_and_he_rises_on_the_shore() {
        let mut g = in_the_sand();
        let lives = g.score.lives;
        g.wound_hero(9999.0, 0xffffff);
        run(&mut g, 2.6);
        assert_eq!(g.score.lives, lives - 1);
        let m = mire(&g);
        assert_eq!(m.phase, Phase::Wander, "the hourglass stands still again");
        assert!(m.raised.is_empty());
        assert!(!in_palm(g.hero.occupied()), "he rises off the palm");
        assert_eq!(m.torpor, 0.0);
    }
}
