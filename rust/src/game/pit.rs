//! The Pit in play (FINALE.md §1, §2; laid out in `logic::pit`). Through the open door of the House
//! of the Devil there is no throne and no devil, only a square shaft and the platform at its lip;
//! he leaps onto it and the first level's clock begins.
//!
//! Each level's inmates (the foes of its stage) come out of the iron doors in the walls, pack by
//! pack, and fight by frame data as the realms' packs do (`brawl`, under the vaults' rules: his real
//! health, his shield, his blade), never more than ten standing at once: a pack whose doors open
//! while the cell is full waits behind them until there is room, and what has not come out when the
//! doors shut stays in. In the last fifteen seconds the bell tolls each second and the platform's
//! edge glows. At zero the platform sinks: every foe standing on it rides down to join the next
//! level's inmates, what stands on the ledge stays behind, and he goes down whatever happens (from
//! the ledge he leaps after it, and lands hurt). The descent takes five seconds, the fight still.
//!
//! A fall begins the level again, its riders as they came down. After the seventh level the
//! platform sinks once more, to the bottom (`pit_reaches_bottom`).

use super::brawl::{Brawl, Field};
use super::save::Stage;
use super::{Banner, Game, Screen};
use crate::audio::Sfx;
use crate::logic::combat::State;
use crate::logic::enemy::{EnemyKind, EnemyMode};
use crate::logic::hub::Rect;
use crate::logic::pit::{self, Arrival};
use crate::logic::realms::Pack;
use crate::logic::sins::ALL_SINS;
use crate::logic::types::{dist_sq, Dir, Tile, P, V};
use crate::render::view::PitView;

/// The bell's warning, and the green of the cross.
const BELL_RED: u32 = 0xff5a3a;
const GREEN: u32 = 0x5cff6a;
/// Seconds a door stands open once its pack has come out.
const DOOR_OPEN: f64 = 2.5;

/// A foe that rode the platform down: what it is, where it stood on the platform, and its health,
/// so a fall can bring the level back as it began.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rider {
    pub kind: EnemyKind,
    pub stage: u8,
    pub at: V,
    pub hp: f64,
}

/// Where a level stands: the fight, the platform going down (seconds into the descent), or the
/// bottom reached.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PitPhase {
    Fight,
    Descent { t: f64 },
    Bottom,
}

/// One level as it went, for the harness: seconds played, foes slain, those that rode down into it
/// and out of it, those left behind on the ledge, damage taken, falls, and whether he had to leap.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LevelTally {
    pub level: u8,
    pub secs: f64,
    pub kills: u32,
    pub riders_in: u32,
    pub riders_out: u32,
    pub left: u32,
    pub taken: f64,
    pub falls: u32,
    pub leapt: bool,
}

/// The Pit while he is in it.
#[derive(Clone, Debug)]
pub struct PitRun {
    /// The level he is on (1 to 7).
    pub level: u8,
    /// Seconds into the level.
    pub clock: f64,
    pub phase: PitPhase,
    /// This level's foes still behind their doors, in the order they come.
    pending: Vec<Arrival>,
    /// The foes that rode down into this level, as they came (a fall brings them back so).
    pub came: Vec<Rider>,
    /// He is on the lip and leaps onto the platform as play begins (the first level, from the House).
    pub leap_in: bool,
    /// He leapt from the ledge after the sinking platform: the landing costs him once he is down.
    leaping: bool,
    /// Seconds each door has left standing open.
    pub doors: [f64; 8],
    /// The clock when each door began to grind open for the pack due behind it (WARBAND.md I7: a
    /// door is heard and seen a second before anything comes out of it).
    grind: [Option<f64>; 8],
    /// The last-seconds banner has been shown this level.
    warned: bool,
    /// How many rode down at the last descent (for the floater as the next level begins).
    rode: usize,
    /// Each level as it went, and the fight's numbers when this level began.
    pub tallies: Vec<LevelTally>,
    seen: (u32, f64),
}

impl PitRun {
    fn new(level: u8) -> Self {
        Self {
            level,
            clock: 0.0,
            phase: PitPhase::Fight,
            pending: pit::arrivals(level),
            came: Vec::new(),
            leap_in: false,
            leaping: false,
            doors: [0.0; 8],
            grind: [None; 8],
            warned: false,
            rode: 0,
            tallies: vec![LevelTally { level, ..LevelTally::default() }],
            seen: (0, 0.0),
        }
    }

    /// This level's tally.
    fn tally(&mut self) -> &mut LevelTally {
        if self.tallies.is_empty() {
            self.tallies.push(LevelTally { level: self.level, ..LevelTally::default() });
        }
        self.tallies.last_mut().unwrap()
    }

    /// Foes still to come out of the doors this level.
    pub fn waiting(&self) -> usize {
        self.pending.len()
    }
}

/// The den of a door's pack: the ledge before it (its foes hunt from the first moment).
fn door_den(door: usize) -> Rect {
    let m = pit::doors()[door].mouth();
    let (x0, x1) = (m.iter().map(|p| p.x).min().unwrap_or(0), m.iter().map(|p| p.x).max().unwrap_or(0));
    let (y0, y1) = (m.iter().map(|p| p.y).min().unwrap_or(0), m.iter().map(|p| p.y).max().unwrap_or(0));
    Rect { x0, y0, x1, y1 }
}

impl Game {
    /// In the Pit.
    pub fn in_pit(&self) -> bool {
        self.pit.is_some()
    }

    /// Through the door of the House of the Devil: no devil, only the shaft and its platform. He
    /// stands at the lip, and leaps onto it as the first level begins.
    pub fn enter_pit(&mut self) {
        self.audio.play(Sfx::DoorGrind);
        self.audio.play(Sfx::Portal);
        self.load_pit(1);
        self.checkpoint(Stage::Pit(1));
        self.juice.fade = 1.0;
        self.juice.banner =
            Some(Banner { text: "THERE IS NO DEVIL HERE.".into(), sub: "ONLY THE WAY DOWN.".into(), color: 0xe8dcc8, life: 4.5, max: 4.5 });
        self.enter(Screen::Ready);
    }

    /// Level `level` of the Pit (1 to 7), with nothing riding down into it: from the House, from
    /// Continue, or the map. He stands on the lip, and leaps onto the platform as play begins.
    /// Everyone who reaches it holds every sin's key.
    pub fn load_pit(&mut self, level: u8) {
        self.sins_cleared = ALL_SINS;
        let tier = self.realm_tier();
        self.load(pit::cell(), tier);
        self.begin_pit(level.clamp(1, pit::LEVELS), Vec::new());
        if let Some(p) = self.pit.as_mut() {
            p.leap_in = true;
        }
        // The waves of this level and every one below it, made ready with the cell.
        self.prepare_level();
    }

    /// The level's fight laid out: the fight by frame data, its riders on the platform, its inmates
    /// behind their doors, the green cross, and him where he comes in (on the lip at the first level,
    /// else on the platform where it came to rest).
    fn begin_pit(&mut self, level: u8, came: Vec<Rider>) {
        let mut run = PitRun::new(level);
        run.came = came;
        if let Some(b) = self.brawl.as_ref() {
            run.seen = (b.stats.kills, b.stats.taken);
        }
        self.pit = Some(run);
        if self.brawl.as_ref().is_none_or(|b| b.field != Field::Vault) {
            let mut b = Brawl::new(Field::Vault);
            b.reset(self.hero_kit());
            self.brawl = Some(b);
        }
        self.raise_riders();
        self.lay_cross();
    }

    /// The riders of this level back on the platform as they came, hunting at once.
    fn raise_riders(&mut self) {
        let Some(came) = self.pit.as_ref().map(|p| p.came.clone()) else { return };
        let tier = self.realm_tier();
        let Some(mut b) = self.brawl.take() else { return };
        let r = pit::platform();
        for c in came {
            let post = P::new(c.at.x.round() as i32, c.at.y.round() as i32);
            let mut raised = self.raise_packs(&[Pack { den: r, foes: vec![(c.kind, c.stage, post)] }], tier, &mut b);
            for e in raised.iter_mut() {
                if let Some(d) = e.duel.as_deref_mut() {
                    d.pos = c.at;
                    d.hp = c.hp.min(d.max_hp);
                    if let Some(den) = d.den.as_mut() {
                        den.awake = true;
                        den.home = c.at;
                    }
                }
            }
            self.enemies.extend(raised);
        }
        self.brawl = Some(b);
        self.disband_packs();
        self.refresh_foe_views();
    }

    /// The green cross on the platform, as each level begins.
    fn lay_cross(&mut self) {
        let c = pit::cross();
        if self.level.grid.get(c.x, c.y) != Tile::Charge {
            self.level.grid.set(c.x, c.y, Tile::Charge);
            self.view.add_charge(&self.level.grid, c.x, c.y);
        }
    }

    /// After a fall (and as the level is loaded): the level begins again, its riders as they came
    /// down, its inmates back behind their doors, him on the platform.
    pub(super) fn reset_pit(&mut self) {
        let Some(p) = self.pit.as_mut() else { return };
        let level = p.level;
        p.clock = 0.0;
        p.phase = PitPhase::Fight;
        p.pending = pit::arrivals(level);
        p.doors = [0.0; 8];
        p.warned = false;
        p.leaping = false;
        let first = p.leap_in;
        if !first {
            p.tally().falls += 1;
        }
        self.enemies.clear();
        if let Some(b) = self.brawl.as_mut() {
            b.threat.clear();
            b.bolts.clear();
            b.hazards.clear();
        }
        if !first {
            self.hero.place(pit::landing());
            self.hero.face(Dir::Up);
            self.view.aim_along(Dir::Up);
        }
        self.raise_riders();
        self.lay_cross();
    }

    /// The Pit's rules each tick of the fight.
    pub(super) fn update_pit(&mut self, dt: f64) {
        let Some(phase) = self.pit.as_ref().map(|p| p.phase) else { return };
        if phase != PitPhase::Fight {
            return;
        }
        // He leaps from the lip onto the platform as the first level begins.
        if self.pit.as_ref().is_some_and(|p| p.leap_in) && !self.hero.airborne() {
            let to = pit::landing().v();
            self.hero.face(Dir::Up);
            self.hero.leap_to(&self.level.grid, to);
            self.audio.play(Sfx::Jump);
            if let Some(p) = self.pit.as_mut() {
                p.leap_in = false;
            }
        }
        // The fallen are cleared away, as in the realms.
        let mut gone = false;
        for e in self.enemies.iter_mut() {
            if e.mode() == EnemyMode::Scattered && e.b.scatter_timer < 0.4 {
                e.b.mode = EnemyMode::Eaten;
            }
            gone |= e.mode() == EnemyMode::Eaten;
        }
        if gone {
            self.enemies.retain(|e| e.mode() != EnemyMode::Eaten);
            self.refresh_foe_views();
        }
        let (kills, taken) = self.brawl.as_ref().map(|b| (b.stats.kills, b.stats.taken)).unwrap_or((0, 0.0));
        let Some(p) = self.pit.as_mut() else { return };
        let before = p.clock;
        p.clock += dt;
        for d in p.doors.iter_mut() {
            *d = (*d - dt).max(0.0);
        }
        let (killed, hurt) = (kills.saturating_sub(p.seen.0), (taken - p.seen.1).max(0.0));
        p.seen = (kills, taken);
        let t = p.tally();
        t.secs += dt;
        t.kills += killed;
        t.taken += hurt;
        let clock = p.clock;
        self.pit_doors();
        // The last seconds: the bell each second, the banner once.
        if pit::tolls(before, clock) {
            self.audio.play(Sfx::Toll);
        }
        let warn = pit::time_left(clock) <= pit::BELL_SECONDS && self.pit.as_ref().is_some_and(|p| !p.warned);
        if warn {
            if let Some(p) = self.pit.as_mut() {
                p.warned = true;
            }
            self.audio.play(Sfx::Bells);
            self.juice.banner = Some(Banner {
                text: "THE PLATFORM DESCENDS. CLEAR IT.".into(),
                sub: "WHAT STANDS ON IT RIDES DOWN WITH YOU".into(),
                color: BELL_RED,
                life: 3.0,
                max: 3.0,
            });
        }
        if clock >= pit::LEVEL_SECONDS {
            self.begin_descent();
        }
    }

    /// Foes standing in the cell (an illusion does not count; one begging does).
    pub fn pit_standing(&self) -> usize {
        self.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.counts()).count()
    }

    /// The doors whose time has come open, pack by pack, while there is room in the cell (and the
    /// doors not yet shut).
    fn pit_doors(&mut self) {
        let Some(p) = self.pit.as_ref() else { return };
        if p.clock >= pit::DOORS_CLOSE {
            if let Some(p) = self.pit.as_mut() {
                p.pending.clear();
            }
            return;
        }
        let clock = p.clock;
        let mut room = pit::MAX_STANDING.saturating_sub(self.pit_standing());
        let mut out: Vec<(Arrival, usize)> = Vec::new();
        let hp = self.hero.pos();
        let Some(p) = self.pit.as_mut() else { return };
        // The doors whose packs are due within the tell, while there is room for them, grind open
        // first; a pack held back for the cap is told again when its room comes.
        let mut grinding = Vec::new();
        for a in p.pending.iter().take_while(|a| a.at as f64 <= clock + pit::DOOR_TELL).take(room) {
            if p.grind[a.door].is_none() {
                p.grind[a.door] = Some(clock);
                p.doors[a.door] = DOOR_OPEN;
                grinding.push(a.door);
            }
        }
        // (Each door's mouth has three tiles; a pack spreads over them.)
        let mut slot = [0usize; 8];
        while room > 0
            && p.pending
                .first()
                .is_some_and(|a| a.at as f64 <= clock && p.grind[a.door].is_some_and(|t| clock - t >= pit::DOOR_TELL - 1e-9))
        {
            let a = p.pending.remove(0);
            out.push((a, slot[a.door]));
            slot[a.door] += 1;
            room -= 1;
            if !p.pending.iter().any(|b| b.door == a.door && b.at as f64 <= clock) {
                p.grind[a.door] = None;
            }
        }
        if room == 0 {
            // (The rest wait behind shut doors, and grind open again when there is room.)
            p.grind = [None; 8];
        }
        for door in grinding {
            let f = pit::doors()[door].front();
            self.audio.play(Sfx::DoorGrind);
            self.view.dust(&self.level.grid, f.x as f64, f.y as f64, 0x5a5650, 14, 1.6);
            if dist_sq(f.v(), hp) < 14.0 * 14.0 {
                self.floater(f.x as f64, f.y as f64 - 0.8, "THE DOOR OPENS", 1.2, Some(0xc8c0b0));
            }
        }
        if out.is_empty() {
            return;
        }
        let tier = self.realm_tier();
        let Some(mut b) = self.brawl.take() else { return };
        let mut opened = Vec::new();
        for (a, k) in out {
            let door = pit::doors()[a.door];
            let at = door.mouth()[k % 3];
            let mut raised = self.raise_packs(&[Pack { den: door_den(a.door), foes: vec![(a.kind, a.stage, at)] }], tier, &mut b);
            for e in raised.iter_mut() {
                if let Some(d) = e.duel.as_deref_mut() {
                    if let Some(den) = d.den.as_mut() {
                        den.awake = true;
                    }
                    // Out of the door, turning to him.
                    let to = V::new(hp.x - d.pos.x, hp.y - d.pos.y);
                    let len = (to.x * to.x + to.y * to.y).sqrt().max(1e-6);
                    d.facing = V::new(to.x / len, to.y / len);
                }
            }
            self.enemies.extend(raised);
            if !opened.contains(&a.door) {
                opened.push(a.door);
            }
        }
        self.brawl = Some(b);
        for door in opened {
            if let Some(p) = self.pit.as_mut() {
                p.doors[door] = DOOR_OPEN;
            }
        }
        self.disband_packs();
        self.refresh_foe_views();
    }

    /**
     * Zero: the platform lets go. Who rides down is settled now (every foe standing on it); the
     * fight stops (blows in the air gone, every foe's move broken off); and if he stands on the
     * ledge, he leaps after it.
     */
    fn begin_descent(&mut self) {
        let hp = self.hero.pos();
        let on = pit::on_platform(hp);
        let riders = self.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.counts() && pit::on_platform(d.pos)).count();
        let left = self.pit_standing() - riders;
        if let Some(b) = self.brawl.as_mut() {
            b.threat.clear();
            b.bolts.clear();
            b.hazards.clear();
            b.traps.clear();
            b.held = None;
        }
        for d in self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()) {
            if matches!(d.f.state, State::Attack { .. }) {
                d.f.state = State::Neutral;
            }
            d.token = false;
            d.slot = None;
            d.vel = V::new(0.0, 0.0);
            d.knock = V::new(0.0, 0.0);
        }
        // The cross left lying on the platform goes down with it, eaten by the dark.
        let c = pit::cross();
        if self.level.grid.get(c.x, c.y) == Tile::Charge {
            self.level.grid.set(c.x, c.y, Tile::Open);
            self.view.remove_collectible(c.x, c.y);
        }
        let Some(p) = self.pit.as_mut() else { return };
        p.phase = PitPhase::Descent { t: 0.0 };
        p.pending.clear();
        p.rode = riders;
        let t = p.tally();
        t.riders_out = riders as u32;
        t.left = left as u32;
        t.leapt = !on;
        self.audio.play(Sfx::BossSlam);
        self.audio.play(Sfx::Chain);
        self.view.shake(0.45);
        let r = pit::platform();
        let mid = V::new((r.x0 + r.x1) as f64 / 2.0, (r.y0 + r.y1) as f64 / 2.0);
        let words = match riders {
            0 => "THE PLATFORM IS CLEAR".to_string(),
            1 => "1 RIDES DOWN WITH YOU".to_string(),
            n => format!("{n} RIDE DOWN WITH YOU"),
        };
        self.floater(mid.x, mid.y - 1.0, words, 2.6, Some(if riders == 0 { 0xe8dcc8 } else { BELL_RED }));
        if !on && self.hero.alive {
            // From the ledge: after it, whatever the drop. He lands hurt (a tenth), never stranded.
            let to = V::new(hp.x.clamp(r.x0 as f64 + 0.6, r.x1 as f64 - 0.6), hp.y.clamp(r.y0 as f64 + 0.6, r.y1 as f64 - 0.6));
            let to = if pit::table().contains(P::new(to.x.round() as i32, to.y.round() as i32)) { V::new(to.x, to.y + 1.0) } else { to };
            self.hero.leap_to(&self.level.grid, to);
            self.audio.play(Sfx::Jump);
            self.floater(hp.x, hp.y - 1.2, "AFTER IT!", 1.2, Some(0xffd8a0));
            if let Some(p) = self.pit.as_mut() {
                p.leaping = true;
            }
        }
    }

    /**
     * The tick while the platform goes down (or rests at the bottom): nothing fights, nothing walks;
     * only his leap after it finishes, and the floaters drift. The old man's guide (B) opens and the
     * belt is drunk from as in play (QA.md Q12). True while it holds the tick.
     */
    pub(super) fn pit_holds(&mut self, dt: f64) -> bool {
        let Some(phase) = self.pit.as_ref().map(|p| p.phase) else { return false };
        if phase != PitPhase::Fight && self.guide_and_belt() {
            return true;
        }
        let t = match phase {
            PitPhase::Fight => return false,
            PitPhase::Bottom => {
                self.update_effects(dt);
                return true;
            }
            PitPhase::Descent { t } => t,
        };
        let now = t + dt;
        if let Some(p) = self.pit.as_mut() {
            p.phase = PitPhase::Descent { t: now };
        }
        // His leap after the platform plays out; the landing costs him.
        let airborne = self.hero.airborne();
        if airborne {
            self.hero.wish = V::new(0.0, 0.0);
            self.hero.update(dt, &self.level.grid, Dir::None);
        }
        if self.pit.as_ref().is_some_and(|p| p.leaping) && !self.hero.airborne() {
            if let Some(p) = self.pit.as_mut() {
                p.leaping = false;
            }
            let loss = pit::leap_cost(self.hero.hp, self.hero.max_hp);
            let at = self.hero.pos();
            self.audio.play(Sfx::Land);
            self.audio.play(Sfx::Hurt);
            self.view.shake(0.35);
            self.view.dust(&self.level.grid, at.x, at.y, 0x6a645c, 16, 1.6);
            if loss > 0 {
                self.hero.hp -= loss;
                self.juice.hurt = 1.0;
                self.juice.hp_flash = 1.0;
                self.juice.hp_flash_color = 0xff4d4d;
                self.floater(at.x, at.y - 0.6, format!("-{loss}"), 0.9, Some(0xff8a6a));
                if let Some(b) = self.brawl.as_mut() {
                    b.stats.taken += loss as f64;
                }
                if let Some(p) = self.pit.as_mut() {
                    p.tally().taken += loss as f64;
                    p.seen.1 += loss as f64;
                }
            }
        }
        // The stone grinds as it goes, and the levels pass.
        let k = |s: f64| s.floor() as i64;
        if t >= pit::SHUDDER_SECONDS && k(t) != k(now) {
            self.audio.play(Sfx::DoorGrind);
            self.view.shake(0.06);
        }
        if t < pit::SHUDDER_SECONDS && now >= pit::SHUDDER_SECONDS {
            self.audio.play(Sfx::Chain);
        }
        self.update_effects(dt);
        if now >= pit::DESCENT_SECONDS {
            self.end_descent();
        }
        true
    }

    /// The platform comes to rest at the next level: what stood on the ledge above is gone, the
    /// riders stand where they stood, and the next level's clock begins (after the seventh, the
    /// bottom).
    fn end_descent(&mut self) {
        let Some(level) = self.pit.as_ref().map(|p| p.level) else { return };
        // Those left on the ledge above stay there.
        self.enemies.retain(|e| e.duel.as_deref().is_some_and(|d| d.counts() && pit::on_platform(d.pos)));
        let came: Vec<Rider> = self
            .enemies
            .iter()
            .filter_map(|e| e.duel.as_deref())
            .map(|d| Rider { kind: d.breed.kind, stage: d.breed.kind.stage(), at: d.pos, hp: d.hp })
            .collect();
        if level >= pit::LEVELS {
            self.refresh_foe_views();
            self.pit_reaches_bottom();
            return;
        }
        let next = level + 1;
        let tallies = self.pit.as_ref().map(|p| p.tallies.clone()).unwrap_or_default();
        let rode = came.len();
        self.enemies.clear();
        self.begin_pit(next, came);
        if let Some(p) = self.pit.as_mut() {
            let mut all = tallies;
            all.push(LevelTally { level: next, riders_in: rode as u32, ..LevelTally::default() });
            p.tallies = all;
            p.rode = rode;
        }
        self.checkpoint(Stage::Pit(next));
        self.audio.play(Sfx::BossSlam);
        self.view.shake(0.4);
        self.view.snap_camera();
        self.juice.banner = Some(Banner {
            text: pit::level_label(next),
            sub: match rode {
                0 => pit::stage_name(next).to_string(),
                1 => format!("{}   1 RODE DOWN WITH YOU", pit::stage_name(next)),
                n => format!("{}   {n} RODE DOWN WITH YOU", pit::stage_name(next)),
            },
            color: pit::light(next),
            life: 3.0,
            max: 3.0,
        });
        let c = pit::cross();
        self.floater(c.x as f64, c.y as f64 - 0.6, "HEALTH", 1.6, Some(GREEN));
    }

    /**
     * The platform has passed the seventh level and come to rest at the bottom of the Pit: Krampus's
     * lair, where the friend waits in his cage (`lair`).
     */
    pub fn pit_reaches_bottom(&mut self) {
        if let Some(p) = self.pit.as_mut() {
            p.phase = PitPhase::Bottom;
        }
        self.audio.play(Sfx::BossSlam);
        self.enter_lair();
    }

    /// What the view needs of the Pit this frame.
    pub fn pit_view(&self) -> Option<PitView> {
        let p = self.pit.as_ref()?;
        let (descent, sink) = match p.phase {
            PitPhase::Descent { t } => (Some(t as f32), pit::sunk(t)),
            _ => (None, 0.0),
        };
        Some(PitView {
            level: p.level,
            clock: p.clock as f32,
            descent,
            sink,
            bell: p.phase == PitPhase::Fight && pit::time_left(p.clock) <= pit::BELL_SECONDS,
            doors: p.doors.map(|d| (d / DOOR_OPEN).clamp(0.0, 1.0) as f32),
            bottom: p.phase == PitPhase::Bottom,
        })
    }

    /// The level's clock and who stands on the platform, for the HUD: (level, seconds left, foes on
    /// the platform, foes still behind the doors, the riders that came down into it).
    pub fn pit_status(&self) -> Option<(u8, f64, usize, usize, usize)> {
        let p = self.pit.as_ref()?;
        let on = self.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.counts() && pit::on_platform(d.pos)).count();
        Some((p.level, pit::time_left(p.clock), on, p.pending.len(), p.came.len()))
    }

    /// The clock set to `secs` into the level (the capture tool).
    pub fn debug_pit_clock(&mut self, secs: f64) {
        if let Some(p) = self.pit.as_mut().filter(|p| p.phase == PitPhase::Fight) {
            p.clock = secs.clamp(0.0, pit::LEVEL_SECONDS - 0.01);
        }
    }

    /// F2 in the Pit: the clock on to its last twenty seconds.
    pub(super) fn cheat_pit(&mut self) {
        if let Some(p) = self.pit.as_mut().filter(|p| p.phase == PitPhase::Fight) {
            p.clock = p.clock.max(pit::LEVEL_SECONDS - 20.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::READY_SECONDS;
    use crate::logic::combat::Duelist;
    const STEP: f64 = crate::app::STEP;

    /// A game in level `level` of the Pit, play under way (the leap in from the lip done).
    pub(crate) fn in_pit(level: u8) -> Game {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        let mut g = Game::new();
        g.map_jump(Stage::Pit(level));
        for _ in 0..((READY_SECONDS + 0.6) / STEP) as usize {
            g.update(STEP);
        }
        assert_eq!(g.screen, Screen::Playing);
        g
    }

    fn run(g: &mut Game, secs: f64) {
        for _ in 0..(secs / STEP).round() as usize {
            g.update(STEP);
        }
    }

    fn duelists(g: &Game) -> Vec<&Duelist> {
        g.enemies.iter().filter_map(|e| e.duel.as_deref()).collect()
    }

    /// Every foe standing put where it is, at full health and harmless (a test's own stage).
    fn hold_still(g: &mut Game) {
        for d in g.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()) {
            d.puppet = true;
        }
    }

    #[test]
    fn the_first_level_begins_with_the_leap_onto_the_platform_and_its_foes_come_out_of_the_doors() {
        let mut g = in_pit(1);
        assert!(g.in_pit());
        assert_eq!(g.level.data.theme, crate::logic::level::Theme::Pit);
        assert!(g.brawl.as_ref().is_some_and(|b| b.field == Field::Vault), "fought by frame data");
        assert_eq!(g.realm_tier(), 8, "every key held");
        assert!(pit::on_platform(g.hero.pos()), "on the platform once the leap is done: {:?}", g.hero.pos());
        assert_eq!(g.level.grid.get(pit::cross().x, pit::cross().y), Tile::Charge, "a green cross on the platform");
        g.immortal = true;
        run(&mut g, 6.0);
        let doors: Vec<P> = pit::doors().iter().flat_map(|d| d.mouth()).collect();
        assert!(!g.enemies.is_empty(), "the first wave is out");
        for d in duelists(&g) {
            assert!(d.den.is_some_and(|d| d.awake), "hunting from the first moment");
            assert_eq!(d.breed.kind.stage(), 1);
            let home = d.den.map(|d| P::new(d.home.x.round() as i32, d.home.y.round() as i32));
            assert!(home.is_some_and(|h| doors.contains(&h)), "out of a door: {home:?}");
        }
    }

    /// WARBAND.md I7 (W9): every pack out of a door was heard and seen a second before, the door
    /// grinding open; and one held back for the cap is told again when its room comes.
    #[test]
    fn a_door_grinds_open_a_second_before_its_pack_comes_out() {
        let mut g = in_pit(2);
        g.immortal = true;
        let mut told: [Option<f64>; 8] = [None; 8];
        let mut seen: Vec<u32> = duelists(&g).iter().map(|d| d.id).collect();
        let mut out = 0;
        for _ in 0..(100.0 / STEP) as usize {
            g.update(STEP);
            hold_still(&mut g);
            let p = g.pit.as_ref().unwrap();
            for (k, at) in p.grind.iter().enumerate() {
                if at.is_some() && told[k].is_none() {
                    told[k] = *at;
                }
            }
            let clock = p.clock;
            for d in duelists(&g) {
                if seen.contains(&d.id) {
                    continue;
                }
                seen.push(d.id);
                let door =
                    pit::doors().iter().position(|dr| dr.mouth().iter().any(|m| (m.x as f64 - d.pos.x).hypot(m.y as f64 - d.pos.y) < 0.6));
                let Some(door) = door else { continue };
                let since = told[door].map(|t| clock - t).unwrap_or(0.0);
                assert!(since >= pit::DOOR_TELL - 1e-6, "a {:?} out of door {door} {since:.2} s after it began to grind", d.breed.kind);
                out += 1;
            }
            for (k, at) in p.grind.iter().enumerate() {
                if at.is_none() {
                    told[k] = None;
                }
            }
        }
        assert!(out >= pit::MAX_STANDING, "{out} came out");
    }

    #[test]
    fn no_more_than_ten_stand_at_once_and_the_rest_wait_behind_their_doors() {
        let mut g = in_pit(2);
        g.immortal = true;
        let mut most = 0;
        for _ in 0..(95.0 / STEP) as usize {
            g.update(STEP);
            hold_still(&mut g);
            most = most.max(g.pit_standing());
        }
        assert_eq!(most, pit::MAX_STANDING, "the cell fills to its cap and no further");
        assert!(g.pit.as_ref().unwrap().waiting() > 0, "the rest wait behind their doors");
        run(&mut g, 10.0);
        assert_eq!(g.pit.as_ref().unwrap().waiting(), 0, "the doors shut on whoever is left in");
    }

    #[test]
    fn at_zero_the_riders_go_down_the_ledge_stays_and_the_next_level_begins() {
        let mut g = in_pit(3);
        g.immortal = true;
        run(&mut g, 30.0);
        hold_still(&mut g);
        // Two foes onto the platform, the rest left on the ledge.
        let r = pit::platform();
        let spots = [V::new(r.x0 as f64 + 1.0, r.y0 as f64 + 1.0), V::new(r.x1 as f64 - 1.0, r.y1 as f64 - 1.0)];
        let mut ids = Vec::new();
        for (k, d) in g.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).enumerate() {
            if k < 2 {
                d.pos = spots[k];
                ids.push(d.breed.kind);
            } else if pit::on_platform(d.pos) {
                d.pos = V::new(2.0, 2.0);
            }
        }
        let ledge = g.pit_standing() - 2;
        assert!(ledge > 0);
        g.pit.as_mut().unwrap().clock = pit::LEVEL_SECONDS - 0.05;
        run(&mut g, 0.1);
        assert!(matches!(g.pit.as_ref().unwrap().phase, PitPhase::Descent { .. }));
        assert!(
            g.floaters.iter().any(|f| f.text == "2 RIDE DOWN WITH YOU"),
            "{:?}",
            g.floaters.iter().map(|f| &f.text).collect::<Vec<_>>()
        );
        // The fight is still while it sinks: nothing moves, the clock waits.
        let hp = g.hero.hp;
        run(&mut g, 2.0);
        assert_eq!(g.hero.hp, hp);
        assert!(g.pit_view().unwrap().sink > 0.0, "the platform is going down");
        run(&mut g, pit::DESCENT_SECONDS);
        let p = g.pit.as_ref().unwrap();
        assert_eq!(p.level, 4);
        assert_eq!(p.phase, PitPhase::Fight);
        assert_eq!(p.came.len(), 2);
        let kinds: Vec<EnemyKind> = duelists(&g).iter().map(|d| d.breed.kind).collect();
        assert_eq!(kinds, ids, "the riders, and only they, came down");
        assert!(duelists(&g).iter().all(|d| pit::on_platform(d.pos)));
        let t = &p.tallies[p.tallies.len() - 2];
        assert_eq!((t.level, t.riders_out, t.left as usize), (3, 2, ledge));
        assert_eq!(g.saved_run.as_ref().map(|r| r.stage), Some(Stage::Pit(4)), "each level a checkpoint");
    }

    #[test]
    fn from_the_ledge_he_leaps_after_the_platform_and_lands_hurt_but_never_dead() {
        let mut g = in_pit(1);
        g.hero.place(P::new(3, 3));
        g.hero.hp = 5;
        let max = g.hero.max_hp;
        g.pit.as_mut().unwrap().clock = pit::LEVEL_SECONDS - 0.01;
        run(&mut g, 1.5);
        assert!(pit::on_platform(g.hero.pos()), "he went after it: {:?}", g.hero.pos());
        assert_eq!(g.hero.hp, 1, "the landing costs him, but not his last point");
        assert!(g.hero.alive);
        run(&mut g, pit::DESCENT_SECONDS);
        assert_eq!(g.pit.as_ref().unwrap().level, 2, "never stranded");
        // And at full health the landing costs a tenth.
        let mut g = in_pit(1);
        g.hero.place(P::new(20, 20));
        g.hero.hp = g.hero.max_hp;
        g.pit.as_mut().unwrap().clock = pit::LEVEL_SECONDS - 0.01;
        run(&mut g, 1.5);
        assert_eq!(g.hero.max_hp - g.hero.hp, pit::leap_cost(max, max).max(1).min(max));
        assert!(g.pit.as_ref().unwrap().tallies[0].leapt);
    }

    #[test]
    fn a_fall_begins_the_level_again_with_its_riders_as_they_came() {
        let mut g = in_pit(4);
        g.immortal = true;
        run(&mut g, 4.0);
        hold_still(&mut g);
        let r = pit::platform();
        for (k, d) in g.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).enumerate() {
            d.pos = if k < 3 { V::new(r.x0 as f64 + 1.0 + k as f64 * 2.0, r.y0 as f64 + 1.0) } else { V::new(2.0, 22.0) };
        }
        g.pit.as_mut().unwrap().clock = pit::LEVEL_SECONDS - 0.01;
        run(&mut g, pit::DESCENT_SECONDS + 0.5);
        assert_eq!(g.pit.as_ref().unwrap().level, 5);
        let came: Vec<(EnemyKind, V)> = g.pit.as_ref().unwrap().came.iter().map(|r| (r.kind, r.at)).collect();
        assert_eq!(came.len(), 3);
        assert_eq!(duelists(&g).len(), 3, "they came down");
        // Fight on a while, then fall.
        run(&mut g, 20.0);
        g.immortal = false;
        g.hero.hp = 1;
        g.wound_hero(50.0, 0xff0000);
        assert_eq!(g.screen, Screen::Dying);
        run(&mut g, 2.5);
        let p = g.pit.as_ref().unwrap();
        assert_eq!((p.level, p.clock < 1.0), (5, true), "the same level, from the start");
        let now: Vec<(EnemyKind, V)> = duelists(&g).iter().map(|d| (d.breed.kind, d.pos)).collect();
        assert_eq!(now, came, "its riders as they came");
        assert!(pit::on_platform(g.hero.pos()));
        assert_eq!(g.level.grid.get(pit::cross().x, pit::cross().y), Tile::Charge);
        assert_eq!(p.tallies.last().unwrap().falls, 1);
    }

    #[test]
    fn after_the_seventh_level_the_platform_reaches_the_bottom_and_krampus_waits() {
        let mut g = in_pit(7);
        g.immortal = true;
        g.pit.as_mut().unwrap().clock = pit::LEVEL_SECONDS - 0.01;
        run(&mut g, pit::DESCENT_SECONDS + 0.5);
        assert!(g.in_lair() && !g.in_pit(), "at the bottom: Krampus's lair");
        assert_eq!(g.level.data.theme, crate::logic::level::Theme::Lair);
        assert!(g.prince().is_some_and(|d| d.breed.kind == EnemyKind::Krampus), "and Krampus in it");
    }

    #[test]
    fn the_open_door_of_the_house_of_the_devil_leads_down_into_the_pit() {
        use crate::logic::hub;
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        let mut g = Game::new();
        g.sins_cleared = ALL_SINS;
        g.map_jump(Stage::Hub);
        g.enter(Screen::Playing);
        assert_eq!(g.hub.as_ref().unwrap().seal, 1.0, "every key held: the causeways stand");
        let door = hub::house_door();
        g.hero.place(P::new(door.x, door.y + 1));
        g.hero.face(Dir::Up);
        g.input.hold_dir(Dir::Up, true);
        for _ in 0..(0.5 / STEP) as usize {
            g.update(STEP);
            if g.in_pit() {
                break;
            }
        }
        g.input.hold_dir(Dir::Up, false);
        assert!(g.in_pit(), "through the door, into the Pit");
        assert_eq!(g.screen, Screen::Ready);
        assert_eq!(g.pit.as_ref().unwrap().level, 1);
        assert!(g.pit.as_ref().unwrap().leap_in, "he leaps onto the platform as it begins");
        assert_eq!(g.hero.occupied(), pit::arrival(), "on the lip");
        assert_eq!(g.juice.banner.as_ref().map(|b| b.text.as_str()), Some("THERE IS NO DEVIL HERE."));
        assert_eq!(g.saved_run.as_ref().map(|r| r.stage), Some(Stage::Pit(1)));
        assert!(g.hub.is_none());
    }

    #[test]
    fn the_expert_bot_gets_through_the_first_level_and_boards_the_platform_in_time() {
        use crate::game::bot::{pit_ride, Loadout, Skill};
        use crate::logic::heroes::HeroKind;
        let r = pit_ride(Skill::Expert, 3, Some(Loadout::all(HeroKind::Ragnar, 7)), 1, pit::LEVEL_SECONDS + pit::DESCENT_SECONDS + 30.0);
        assert!(r.levels.len() >= 2, "it reached the second level: {:?}", r.levels);
        let first = r.levels[0];
        assert_eq!((first.level, first.falls), (1, 0), "{first:?}");
        assert!(!first.leapt, "on the platform before it sank");
        assert!(first.kills >= 15, "it fought: {first:?}");
        assert_eq!(r.deaths, 0);
    }

    #[test]
    fn continue_brings_him_back_to_the_level_he_reached() {
        let mut g = in_pit(5);
        let saved = g.saved_run.clone().unwrap();
        assert_eq!(saved.stage, Stage::Pit(5));
        g.enter(Screen::Title);
        g.saved_run = Some(saved);
        g.continue_game();
        assert!(g.in_pit());
        assert_eq!(g.pit.as_ref().unwrap().level, 5);
        assert_eq!(g.sins_cleared, ALL_SINS);
    }

    /// While the platform goes down nothing fights, but the old man's guide (B) opens and the belt
    /// is drunk from, as in play; the descent waits while the guide is up (QA.md Q12).
    #[test]
    fn the_guide_and_the_belt_work_while_the_platform_goes_down() {
        use crate::input::Action;
        use crate::logic::types::PotionKind;
        let mut g = in_pit(2);
        g.immortal = true;
        g.belt = vec![PotionKind::Health];
        g.belt_index = 0;
        g.pit.as_mut().unwrap().clock = pit::LEVEL_SECONDS - 0.05;
        run(&mut g, 0.5);
        assert!(matches!(g.pit.as_ref().unwrap().phase, PitPhase::Descent { .. }));
        g.hero.hp = g.hero.max_hp / 4;
        let hp = g.hero.hp;
        g.input.inject(Action::Drink);
        g.update(STEP);
        assert!(g.belt.is_empty() && g.hero.hp > hp, "drunk on the way down: {} -> {}", hp, g.hero.hp);
        // B: the guide opens, and the platform waits for him.
        g.input.inject(Action::Guide);
        g.update(STEP);
        assert!(g.guide_open && g.screen == Screen::Paused, "the guide opens on the way down");
        let PitPhase::Descent { t } = g.pit.as_ref().unwrap().phase else { panic!("still going down") };
        run(&mut g, 1.0);
        assert_eq!(g.pit.as_ref().unwrap().phase, PitPhase::Descent { t }, "the descent waits for him");
        g.input.inject(Action::Guide);
        g.update(STEP);
        assert_eq!(g.screen, Screen::Playing);
        run(&mut g, pit::DESCENT_SECONDS);
        assert_eq!(g.pit.as_ref().unwrap().level, 3, "and goes on down");
    }
}
