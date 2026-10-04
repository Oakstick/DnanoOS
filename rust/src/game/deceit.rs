//! The prince's cheats at run time (WARBAND.md §10.3; their rules in `logic::war::deceit`).
//!
//! Each realm visit deals two (`deceit::deal`). Two are places: **the false gatekeeper**, an old
//! man in a side room selling at half price (no Quitmer, a shadow dog that never wags, a lantern in
//! the sin's colour), and **loaded dice**, a mimic chest (it breathes, its coins never glint, its
//! hinge creaks). Three are moments, played one at a time in Rest or Build: **the whisper** (false
//! counsel in the old man's words, in the sin's colour, with a reversed echo and no bark), **the
//! crooked bargain** (an offer through the prince's offer screen, with an honest hidden price), and
//! **the turned sign** (the realm's compass needle turned toward a den for 20 s, flickering in the
//! sin's colour, while the old man's amber lamp stays true).
//!
//! Nothing here takes what the hero owns: the false wares are lent, and gone with the realm.

use super::counsel::{toward, Speaker, AMBER};
use super::{Banner, Game, MenuItem, Offer, Screen};
use crate::audio::Sfx;
use crate::logic::combat::duelist::Lore;
use crate::logic::combat::Phase;
use crate::logic::enemy::EnemyKind;
use crate::logic::hub::Rect;
use crate::logic::realms::{deepest_stage, weight};
use crate::logic::sins::Sin;
use crate::logic::types::{dist_sq, Dir, PotionKind, P, V};
use crate::logic::war::deceit::{
    self, keeper_wares, may_begin, wares_worth, CheatKind, Ware, WareKind, Whisper, BLADE_BLOWS, BLADE_MIGHT, SHATTER_MIGHT,
    SHATTER_SECONDS, SIGN_SECONDS, TURN_MIGHT, TURN_SECONDS,
};
use crate::logic::war::WarEvent;

/// Seconds into a visit before the first moment of deceit, and between them.
const FIRST_AT: f64 = 14.0;
const BETWEEN: f64 = 60.0;
/// A swing meets the false gatekeeper or a chest within this reach, in front of him.
const STRIKE_REACH: f64 = 1.7;
/// Walking up to the false gatekeeper this close puts his wares to him (and he must step this far
/// off before they are put again).
const TRADE_AT: f64 = 1.0;
const REARM_AT: f64 = 2.6;
/// ...and not again for this long (he may stand by a corridor the hero walks often).
const REASK: f64 = 20.0;
/// The silver a chest is worth outside the vaults, twice over when struck first.
const CHEST_SILVER: u32 = 40;
/// The bargain's hidden price: this share of his health.
pub const PRICE_SHARE: f64 = 1.0 / 3.0;
/// The elite demon's health over its stage's.
const ELITE: f64 = 1.5;

/// A whisper in the air: what it said, and when (the visit's clock).
#[derive(Clone, Debug, PartialEq)]
pub struct LiveWhisper {
    pub w: Whisper,
    pub since: f64,
}

/// The false gatekeeper in his side room.
#[derive(Clone, Debug, PartialEq)]
pub struct Keeper {
    pub at: V,
    /// The way he faces (into his room).
    pub facing: V,
    pub wares: [Ware; 2],
    pub sold: [bool; 2],
    /// He will put his wares to the hero when next he walks up (once he has been away a while).
    pub armed: bool,
    /// When he last put them (the visit's clock).
    pub asked_at: f64,
    /// Struck: the demon he was, and whether it has fallen.
    pub demon: Option<u32>,
    pub slain: bool,
}

/// A mimic chest.
#[derive(Clone, Debug, PartialEq)]
pub struct Mimic {
    pub at: V,
    /// Struck first (double loot), or opened (it bit): either way, done.
    pub done: bool,
    pub biter: Option<u32>,
    creak: f64,
}

/// The realm's compass turned.
#[derive(Clone, Debug, PartialEq)]
pub struct Sign {
    pub left: f64,
    /// Where the turned needle points: the den it would lead him into.
    pub toward: V,
    pub den: Rect,
    pub entered: bool,
}

/// What the prince's offer screen is asking.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Asking {
    Keeper,
    Bargain,
}

/// The false wares he carries: bottles that will turn, a blade's blows left, and how long the turn
/// and the sting of a shattered blade last. `applied`/`expect` keep the multiple on his blows apart
/// from the realm's own (Pride's Glory).
#[derive(Clone, Debug, PartialEq)]
pub struct Tainted {
    pub elixirs: Vec<PotionKind>,
    pub blade: Option<u32>,
    pub turned: f64,
    pub shattered: f64,
    applied: f64,
    expect: f64,
}

impl Default for Tainted {
    fn default() -> Self {
        Tainted { elixirs: Vec::new(), blade: None, turned: 0.0, shattered: 0.0, applied: 1.0, expect: f64::NAN }
    }
}

impl Tainted {
    /// What the false wares do to his blows now.
    pub fn might(&self) -> f64 {
        let mut m = 1.0;
        if self.blade.is_some() {
            m *= BLADE_MIGHT;
        }
        if self.turned > 0.0 {
            m *= TURN_MIGHT;
        }
        if self.shattered > 0.0 {
            m *= SHATTER_MIGHT;
        }
        m
    }

    pub fn any(&self) -> bool {
        !self.elixirs.is_empty() || self.blade.is_some() || self.turned > 0.0 || self.shattered > 0.0
    }
}

/// The prince's cheats in the realm he is in.
#[derive(Clone, Debug, Default)]
pub struct Deceit {
    /// The realm they were dealt for.
    pub sin: Option<Sin>,
    pub dealt: Vec<CheatKind>,
    /// Those played (or met) this visit.
    pub played: Vec<CheatKind>,
    /// Seconds of play this visit, and when the next moment may come.
    pub clock: f64,
    next_at: f64,
    pub whisper: Option<LiveWhisper>,
    pub keeper: Option<Keeper>,
    pub mimic: Option<Mimic>,
    pub sign: Option<Sign>,
    pub asking: Option<Asking>,
    pub tainted: Tainted,
    /// His swing was in its active frames last frame (each swing counts once).
    swinging: bool,
    visits: u32,
    /// No cheats at all (the bots' harnesses, `VH_NO_DECEIT`).
    pub off: bool,
    /// Deal them even in the tests (which otherwise deal their own).
    pub force: bool,
}

impl Game {
    /// Deals the realm's cheats as he enters it: the places (the false gatekeeper's room, a mimic
    /// chest) are set now, the moments wait for their time.
    pub(super) fn deal_deceit(&mut self, sin: Sin) {
        static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        let d = &mut self.voices.deceit;
        // (Tests deal their own, so the realms' own tests play as they always did.)
        if d.off || (cfg!(test) && !d.force) || *OFF.get_or_init(|| std::env::var_os("VH_NO_DECEIT").is_some()) {
            return;
        }
        d.visits += 1;
        // From the realm's layout and the visit, not from the game's dice (nothing else shifts).
        let mut seed = d.visits.wrapping_mul(7919) ^ (sin.index() as u32) << 20;
        for b in self.level.data.name.bytes().chain(self.level.grid.width.to_le_bytes()).chain(self.level.grid.height.to_le_bytes()) {
            seed = seed.rotate_left(5) ^ b as u32;
        }
        seed ^= self.warband.save.lies.wrapping_mul(31) ^ (self.realm_tier() as u32) << 8;
        let dealt = deceit::deal(sin, seed);
        let d = &mut self.voices.deceit;
        d.sin = Some(sin);
        d.dealt = dealt.clone();
        d.played.clear();
        d.clock = 0.0;
        d.next_at = FIRST_AT;
        if dealt.contains(&CheatKind::FalseGatekeeper) {
            self.place_keeper(seed);
        }
        if dealt.contains(&CheatKind::LoadedDice) {
            self.place_mimic(seed.rotate_left(11));
        }
    }

    /// He left the realm: the false wares crumble (the bottles off his belt, the blade gone, his
    /// blows his own again), and every cheat with it.
    pub(super) fn leave_deceit(&mut self) {
        let t = std::mem::take(&mut self.voices.deceit.tainted);
        for k in &t.elixirs {
            if let Some(i) = self.belt.iter().rposition(|b| b == k) {
                self.belt.remove(i);
            }
        }
        if let Some(b) = self.brawl.as_mut() {
            if (b.might - t.expect).abs() < 1e-12 {
                b.might /= t.applied;
            }
        }
        if t.any() {
            self.belt_index = self.belt_index.min(self.belt.len().saturating_sub(1));
            let hp = self.hero.pos();
            self.floater(hp.x, hp.y - 1.0, "THE PRINCE'S WARES CRUMBLE TO ASH", 2.2, Some(0x9a8a7a));
        }
        let d = &mut self.voices.deceit;
        let (visits, off, force) = (d.visits, d.off, d.force);
        *d = Deceit { visits, off, force, ..Deceit::default() };
    }

    /// The false gatekeeper's room: in the vaults a chamber with no den in it; in a realm open floor
    /// well away from the way in, the far portal and the dens.
    fn place_keeper(&mut self, seed: u32) {
        let Some(spot) = self.side_spot(seed, 1) else { return };
        let price = crate::logic::progression::bottle_price(PotionKind::Strength);
        let facing = toward(spot, self.hero.pos());
        self.voices.deceit.keeper = Some(Keeper {
            at: spot,
            facing,
            wares: keeper_wares(price, 120),
            sold: [false; 2],
            armed: true,
            asked_at: f64::NEG_INFINITY,
            demon: None,
            slain: false,
        });
    }

    fn place_mimic(&mut self, seed: u32) {
        let keeper = self.voices.deceit.keeper.as_ref().map(|k| k.at);
        let Some(mut spot) = self.side_spot(seed, 1) else { return };
        if keeper.is_some_and(|k| dist_sq(k, spot) < 36.0) {
            match self.side_spot(seed.rotate_left(7) ^ 0x55, 1) {
                Some(s) if keeper.is_none_or(|k| dist_sq(k, s) >= 36.0) => spot = s,
                _ => return,
            }
        }
        self.voices.deceit.mimic = Some(Mimic { at: spot, done: false, biter: None, creak: 0.0 });
    }

    /// A quiet floor tile for a cheat to stand on: open ground `clear` tiles about, out of every
    /// den, away from where he came in and the far portal. Chosen by `seed`. (In the vaults every
    /// chamber holds a pack: there he keeps to an alcove of the corridors.)
    fn side_spot(&self, seed: u32, clear: i32) -> Option<V> {
        let grid = &self.level.grid;
        let mut dens: Vec<Rect> = Vec::new();
        let mut rooms: Vec<Rect> = Vec::new();
        let mut clear = clear;
        if let Some(v) = &self.vault {
            dens.extend(v.packs.iter().map(|p| p.den));
            rooms.extend(v.corridors.iter().copied());
            clear = 0;
        }
        if let Some(run) = &self.run {
            dens.extend(run.base().packs.iter().map(|p| p.den));
        }
        let start = self.level.hero_spawn;
        let exit = self.gate;
        let far = |p: P, q: P, d: i32| (p.x - q.x).abs() + (p.y - q.y).abs() >= d;
        let mut spots: Vec<P> = Vec::new();
        for y in clear.max(1)..grid.height - clear.max(1) {
            for x in clear.max(1)..grid.width - clear.max(1) {
                let p = P::new(x, y);
                if !far(p, start, 12) || exit.is_some_and(|e| !far(p, e, 8)) {
                    continue;
                }
                let margin = if self.vault.is_some() { 1 } else { 2 };
                if dens.iter().any(|r| x >= r.x0 - margin && x <= r.x1 + margin && y >= r.y0 - margin && y <= r.y1 + margin) {
                    continue;
                }
                if !rooms.is_empty() && !rooms.iter().any(|r| r.contains(p)) {
                    continue;
                }
                let open = (-clear..=clear).all(|dy| (-clear..=clear).all(|dx| grid.is_walkable(x + dx, y + dy, false)));
                if open {
                    spots.push(p);
                }
            }
        }
        if spots.is_empty() {
            return None;
        }
        let p = spots[(seed as usize).wrapping_mul(2_654_435_761) % spots.len()];
        Some(p.v())
    }

    /// Once a frame of play in a realm with cheats dealt.
    pub(super) fn update_deceit(&mut self, dt: f64) {
        let Some(sin) = self.voices.deceit.sin else { return };
        self.voices.deceit.clock += dt;
        let swing = self.new_swing();
        self.update_tainted(dt, sin);
        self.update_keeper(sin, swing);
        self.update_mimic(sin, swing, dt);
        self.update_sign(dt);
        let d = &mut self.voices.deceit;
        if d.whisper.as_ref().is_some_and(|w| d.clock - w.since > deceit::ASK_WINDOW) {
            // Unasked: the lie stands (what it costs is what he does with it).
            d.whisper = None;
        }
        // The next moment of deceit: in Rest or Build, one at a time, the panel free.
        let d = &self.voices.deceit;
        let busy = d.whisper.is_some() || d.sign.is_some() || d.asking.is_some();
        if busy || d.clock < d.next_at || !may_begin(self.war.drum.phase) || self.voices_line().is_some() {
            return;
        }
        let Some(next) = d
            .dealt
            .iter()
            .copied()
            .find(|c| matches!(c, CheatKind::Whisper | CheatKind::TurnedSign | CheatKind::CrookedBargain) && !d.played.contains(c))
        else {
            return;
        };
        self.voices.deceit.played.push(next);
        self.voices.deceit.next_at = self.voices.deceit.clock + BETWEEN;
        match next {
            CheatKind::Whisper => self.begin_whisper(sin),
            CheatKind::TurnedSign => self.begin_sign(),
            CheatKind::CrookedBargain => self.begin_bargain(),
            _ => {}
        }
    }

    /// A new swing of his reached its active frames this frame: where he stands and where it goes.
    fn new_swing(&mut self) -> Option<(V, V)> {
        let b = self.brawl.as_ref()?;
        let active = matches!(b.hero.attack(), Some((_, Phase::Active, _)));
        let aim = b.aim;
        let fresh = active && !self.voices.deceit.swinging;
        self.voices.deceit.swinging = active;
        fresh.then(|| (self.hero.pos(), aim))
    }

    /// Does the swing meet something at `at`?
    fn swing_meets(swing: Option<(V, V)>, at: V) -> bool {
        swing.is_some_and(|(from, aim)| {
            let d = dist_sq(from, at).sqrt();
            let to = toward(from, at);
            d <= STRIKE_REACH && (d < 0.4 || aim.x * to.x + aim.y * to.y > 0.2)
        })
    }

    // ------------------------------------------------------------------ the whisper

    fn begin_whisper(&mut self, sin: Sin) {
        let hero = self.hero_kind;
        let hp = self.hero.pos();
        let seed = (self.voices.deceit.clock * 1000.0) as u32 ^ self.voices.deceit.visits;
        // Of a foe near (the one he will meet next), else a rival, else the way out.
        let near = self
            .enemies
            .iter()
            .filter_map(|e| e.duel.as_deref())
            .filter(|d| d.alive() && dist_sq(d.pos, hp) < 16.0 * 16.0)
            .min_by(|a, b| dist_sq(a.pos, hp).total_cmp(&dist_sq(b.pos, hp)))
            .map(|d| d.breed.kind);
        let w = if let Some(kind) = near {
            deceit::whisper_breed(kind, sin, hero, seed)
        } else if let Some(r) = self.ledger().live().next() {
            deceit::whisper_rival(r, sin, hero, seed)
        } else {
            deceit::whisper_way(self.way_out(), sin, hero)
        };
        self.speak(Speaker::Whisper(sin), w.text.clone());
        let since = self.voices.deceit.clock;
        self.voices.deceit.whisper = Some(LiveWhisper { w, since });
    }

    /// Where the far portal lies from him, by the nearest of the four ways.
    pub(super) fn way_out(&self) -> Dir {
        let Some(g) = self.gate else { return Dir::Right };
        let hp = self.hero.pos();
        let (dx, dy) = (g.x as f64 - hp.x, g.y as f64 - hp.y);
        if dx.abs() >= dy.abs() {
            if dx >= 0.0 {
                Dir::Right
            } else {
                Dir::Left
            }
        } else if dy >= 0.0 {
            Dir::Down
        } else {
            Dir::Up
        }
    }

    // ------------------------------------------------------------------ the turned sign

    /// The dens about him, with a foe still asleep in them.
    fn sleeping_dens(&self) -> Vec<Rect> {
        let mut dens: Vec<Rect> = Vec::new();
        if let Some(v) = &self.vault {
            dens.extend(v.packs.iter().map(|p| p.den));
        }
        if let Some(run) = &self.run {
            dens.extend(run.base().packs.iter().map(|p| p.den));
        }
        dens.retain(|r| {
            self.enemies.iter().filter_map(|e| e.duel.as_deref()).any(|d| {
                let p = P::new(d.pos.x.round() as i32, d.pos.y.round() as i32);
                d.alive() && d.den.is_some_and(|n| !n.awake) && r.contains(p)
            })
        });
        dens
    }

    fn begin_sign(&mut self) {
        let hp = self.hero.pos();
        let exit = self.gate.map(|g| toward(hp, g.v())).unwrap_or(V::new(1.0, 0.0));
        // The den most out of his way, within reach.
        let den = self
            .sleeping_dens()
            .into_iter()
            .map(|r| {
                let (cx, cy) = r.center();
                (r, V::new(cx as f64, cy as f64))
            })
            .filter(|(_, c)| dist_sq(*c, hp) < 45.0 * 45.0)
            .min_by(|a, b| {
                let off = |c: V| {
                    let t = toward(hp, c);
                    t.x * exit.x + t.y * exit.y
                };
                off(a.1).total_cmp(&off(b.1))
            });
        let Some((den, c)) = den else { return };
        self.voices.deceit.sign = Some(Sign { left: SIGN_SECONDS, toward: c, den, entered: false });
        self.audio.play(Sfx::Whisper);
    }

    fn update_sign(&mut self, dt: f64) {
        let p = self.hero.occupied();
        let Some(s) = self.voices.deceit.sign.as_mut() else { return };
        s.left -= dt;
        s.entered |= s.den.contains(p);
        if s.left <= 0.0 {
            let followed = s.entered;
            self.voices.deceit.sign = None;
            if !followed {
                self.seen_through(CheatKind::TurnedSign);
            }
        }
    }

    /// Where the realm's compass needle points now, and whether it has been turned.
    pub fn compass(&self) -> Option<(V, bool)> {
        match &self.voices.deceit.sign {
            Some(s) => Some((s.toward, true)),
            None => self.gate.map(|g| (g.v(), false)),
        }
    }

    // ------------------------------------------------------------------ the bargain

    fn begin_bargain(&mut self) {
        if self.run.is_none() || self.gate_open {
            return;
        }
        self.voices.deceit.asking = Some(Asking::Bargain);
        self.ask_offer();
    }

    /// The offer the prince's screen shows for a cheat, if one is being asked.
    pub fn deceit_offer(&self) -> Option<Offer> {
        let d = &self.voices.deceit;
        let sin = d.sin?;
        match d.asking? {
            Asking::Keeper => {
                let k = d.keeper.as_ref()?;
                let ware = |i: usize| {
                    let w = &k.wares[i];
                    if k.sold[i] {
                        format!("{}: SOLD", w.name())
                    } else {
                        format!("{}: {} SILVER (HE ASKS {})", w.name(), w.price(), w.full)
                    }
                };
                Some(Offer {
                    title: "THE OLD MAN".into(),
                    lines: vec![deceit::lie(CheatKind::FalseGatekeeper, sin), ware(0), ware(1), "Quitmer? Asleep in the back.".into()],
                    choices: vec![MenuItem::Choice("BUY THE ELIXIR"), MenuItem::Choice("BUY THE BLADE"), MenuItem::Choice("WALK AWAY")],
                    color: sin.color(),
                })
            }
            Asking::Bargain => Some(Offer {
                title: format!("{}'S BARGAIN", sin.prince().to_uppercase()),
                lines: vec![deceit::lie(CheatKind::CrookedBargain, sin), "Accept, and you stand at the end of the road.".into()],
                choices: vec![MenuItem::Choice("ACCEPT"), MenuItem::Choice("REFUSE")],
                color: sin.color(),
            }),
        }
    }

    /// His answer on the prince's screen.
    pub(super) fn answer_deceit(&mut self, choice: MenuItem) {
        let asking = self.voices.deceit.asking.take();
        self.enter(Screen::Playing);
        let sin = self.voices.deceit.sin.unwrap_or(Sin::Greed);
        match asking {
            Some(Asking::Keeper) => match choice {
                MenuItem::Choice("BUY THE ELIXIR") => self.buy_false_ware(0, sin),
                MenuItem::Choice("BUY THE BLADE") => self.buy_false_ware(1, sin),
                _ => {}
            },
            Some(Asking::Bargain) if choice == MenuItem::Choice("ACCEPT") => {
                // The gain is real: the end of the road. So is the price he did not name.
                self.cheat_run();
                let loss = ((self.hero.max_hp as f64 * PRICE_SHARE).round() as i32).min(self.hero.hp - 1).max(0);
                self.hero.hp -= loss;
                self.audio.play(Sfx::Toll);
                self.view.shake(0.4);
                self.juice.banner = Some(Banner {
                    text: "THE END OF THE ROAD".into(),
                    sub: format!("AND THE PRICE HE DID NOT NAME: {loss} OF YOUR HEALTH"),
                    color: sin.color(),
                    life: 3.5,
                    max: 3.5,
                });
            }
            Some(Asking::Bargain) => {
                let hp = self.hero.pos();
                self.audio.play(Sfx::Nodule);
                self.floater(hp.x, hp.y - 1.0, "REFUSED", 1.4, Some(AMBER));
            }
            None => {}
        }
    }

    // ------------------------------------------------------------------ the false gatekeeper

    fn update_keeper(&mut self, sin: Sin, swing: Option<(V, V)>) {
        let hp = self.hero.pos();
        let slain_now = {
            let read = &self.war.read;
            self.voices
                .deceit
                .keeper
                .as_ref()
                .and_then(|k| k.demon.filter(|_| !k.slain))
                .is_some_and(|id| read.iter().any(|e| matches!(e, WarEvent::Slain { foe, .. } if *foe == id)))
        };
        if slain_now {
            let worth = self.voices.deceit.keeper.as_ref().map(|k| wares_worth(&k.wares)).unwrap_or(0);
            if let Some(k) = self.voices.deceit.keeper.as_mut() {
                k.slain = true;
            }
            self.progress.silver += worth;
            self.save_progress();
            self.floater(hp.x, hp.y - 1.6, format!("HIS WARES' PRICE: +{worth} SILVER"), 2.4, Some(0xd8d8e0));
            self.seen_through(CheatKind::FalseGatekeeper);
            return;
        }
        let Some(k) = self.voices.deceit.keeper.as_mut() else { return };
        if k.demon.is_some() {
            return;
        }
        let d = dist_sq(k.at, hp).sqrt();
        if Self::swing_meets(swing, k.at) {
            let at = k.at;
            let facing = toward(at, hp);
            self.reveal_keeper(sin, at, facing);
            return;
        }
        if d > REARM_AT && self.voices.deceit.clock - k.asked_at > REASK {
            k.armed = true;
        }
        let swinging = self.voices.deceit.swinging;
        let Some(k) = self.voices.deceit.keeper.as_mut() else { return };
        if k.armed && d < TRADE_AT && !swinging && self.screen == Screen::Playing {
            k.armed = false;
            k.asked_at = self.voices.deceit.clock;
            if !self.voices.deceit.played.contains(&CheatKind::FalseGatekeeper) {
                self.voices.deceit.played.push(CheatKind::FalseGatekeeper);
            }
            self.voices.deceit.asking = Some(Asking::Keeper);
            self.ask_offer();
        }
    }

    /// Struck, the old man is a demon: an elite of the realm's deepest stage, where he stood.
    fn reveal_keeper(&mut self, sin: Sin, at: V, facing: V) {
        let stage = deepest_stage(self.realm_tier());
        let kind = demon_of(stage);
        let id = self.spawn_duelist(kind, at, facing, Lore::default());
        if let Some(id) = id {
            self.weigh_spawned(id, stage, ELITE);
        }
        if let Some(k) = self.voices.deceit.keeper.as_mut() {
            k.demon = Some(id.unwrap_or(u32::MAX));
            k.slain = id.is_none();
        }
        if !self.voices.deceit.played.contains(&CheatKind::FalseGatekeeper) {
            self.voices.deceit.played.push(CheatKind::FalseGatekeeper);
        }
        self.audio.play(Sfx::Stinger);
        self.view.shake(0.45);
        self.view.dust(&self.level.grid, at.x, at.y, sin.color(), 24, 2.5);
        self.floater(at.x, at.y - 1.4, "NO OLD MAN: A DEMON", 2.0, Some(sin.color()));
    }

    /// A spawned foe weighed as the realm weighs its own, times `extra` on its health.
    fn weigh_spawned(&mut self, id: u32, stage: u8, extra: f64) {
        let tier = self.realm_tier();
        let (hp, damage) = weight(stage, tier);
        if let Some(d) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == id) {
            d.max_hp *= hp * extra;
            d.hp = d.max_hp;
            d.breed.damage *= damage;
        }
    }

    fn buy_false_ware(&mut self, i: usize, sin: Sin) {
        let Some(k) = self.voices.deceit.keeper.as_ref() else { return };
        let (ware, sold) = (k.wares[i], k.sold[i]);
        let hp = self.hero.pos();
        if sold || self.progress.silver < ware.price() {
            self.audio.play(Sfx::Denied);
            return;
        }
        if matches!(ware.kind, WareKind::Elixir(_)) && self.belt.len() >= crate::logic::gear::BELT_SLOTS {
            self.audio.play(Sfx::Denied);
            self.floater(hp.x, hp.y - 1.0, "YOUR BELT IS FULL", 1.4, Some(0xff6a6a));
            return;
        }
        self.progress.silver -= ware.price();
        self.save_progress();
        if let Some(k) = self.voices.deceit.keeper.as_mut() {
            k.sold[i] = true;
        }
        match ware.kind {
            WareKind::Elixir(p) => {
                self.belt.push(p);
                self.voices.deceit.tainted.elixirs.push(p);
            }
            WareKind::Blade => self.voices.deceit.tainted.blade = Some(BLADE_BLOWS),
        }
        self.audio.play(Sfx::Buy);
        self.floater(hp.x, hp.y - 1.0, ware.name(), 1.6, Some(sin.color()));
    }

    /// The false wares at work: a tainted bottle drunk turns (his blows weak for a while), the
    /// bright blade counts its blows and shatters; their multiple on his blows kept up to date.
    fn update_tainted(&mut self, dt: f64, sin: Sin) {
        let hp = self.hero.pos();
        let events = self.war.read.clone();
        for e in events {
            match e {
                WarEvent::Drank { potion } => {
                    let t = &mut self.voices.deceit.tainted;
                    if let Some(i) = t.elixirs.iter().position(|&k| k == potion) {
                        t.elixirs.remove(i);
                        t.turned = TURN_SECONDS;
                        self.audio.play(Sfx::Denied);
                        self.floater(hp.x, hp.y - 1.2, "THE ELIXIR TURNS: YOUR ARMS GO WEAK", 2.0, Some(sin.color()));
                    }
                }
                WarEvent::HeroHit { .. } => {
                    let t = &mut self.voices.deceit.tainted;
                    if let Some(n) = t.blade.as_mut() {
                        *n = n.saturating_sub(1);
                        if *n == 0 {
                            t.blade = None;
                            t.shattered = SHATTER_SECONDS;
                            self.audio.play(Sfx::ShieldBreak);
                            self.view.shake(0.3);
                            self.floater(hp.x, hp.y - 1.2, "THE BRIGHT BLADE SHATTERS", 2.0, Some(sin.color()));
                        }
                    }
                }
                _ => {}
            }
        }
        let t = &mut self.voices.deceit.tainted;
        t.turned = (t.turned - dt).max(0.0);
        t.shattered = (t.shattered - dt).max(0.0);
        let m = t.might();
        if let Some(b) = self.brawl.as_mut() {
            // (If the realm set his might itself since, that is the base now.)
            let base = if (b.might - t.expect).abs() < 1e-12 { b.might / t.applied } else { b.might };
            b.might = base * m;
            t.applied = m;
            t.expect = b.might;
        }
    }

    // ------------------------------------------------------------------ loaded dice

    fn update_mimic(&mut self, sin: Sin, swing: Option<(V, V)>, dt: f64) {
        let hp = self.hero.pos();
        let airborne = self.hero.airborne();
        let Some(m) = self.voices.deceit.mimic.as_mut() else { return };
        if m.done {
            return;
        }
        let d = dist_sq(m.at, hp).sqrt();
        let at = m.at;
        // Its hinge creaks when he is near.
        m.creak -= dt;
        let creak = d < 6.0 && m.creak <= 0.0;
        if creak {
            m.creak = 2.6;
        }
        if Self::swing_meets(swing, at) {
            m.done = true;
            self.chest_struck(sin, at);
            return;
        }
        if d < 0.6 && !airborne {
            m.done = true;
            self.chest_bites(sin, at);
            return;
        }
        if creak {
            self.audio.play(Sfx::Creak);
        }
    }

    /// Struck first: it dies with a shriek, and gives twice what a chest gives.
    fn chest_struck(&mut self, sin: Sin, at: V) {
        if !self.voices.deceit.played.contains(&CheatKind::LoadedDice) {
            self.voices.deceit.played.push(CheatKind::LoadedDice);
        }
        self.audio.play(Sfx::Shriek);
        self.view.sparkle(&self.level.grid, at.x, at.y, 0.8, 0xffd860, 40);
        let tile = P::new(at.x.round() as i32, at.y.round() as i32);
        if self.vault.is_some() {
            let worth = crate::logic::vault::LootKind::Chest.worth() * 2;
            self.hoard.gold += worth;
            self.hoard.chests += 2;
            self.open_chest(tile);
            self.open_chest(tile);
            self.floater(at.x, at.y - 0.6, format!("THE MIMIC'S HOARD: DOUBLE +{worth}"), 2.0, Some(0xffd860));
        } else {
            let worth = CHEST_SILVER * 2;
            self.progress.silver += worth;
            self.save_progress();
            self.floater(at.x, at.y - 0.6, format!("THE MIMIC'S HOARD: DOUBLE +{worth} SILVER"), 2.0, Some(0xffd860));
        }
        let _ = sin;
        self.seen_through(CheatKind::LoadedDice);
    }

    /// Opened: it was a mouth. A body foe of the realm comes out of it biting (a duelist, so the
    /// threat manager plans its bite like any other).
    fn chest_bites(&mut self, sin: Sin, at: V) {
        if !self.voices.deceit.played.contains(&CheatKind::LoadedDice) {
            self.voices.deceit.played.push(CheatKind::LoadedDice);
        }
        let stage = deepest_stage(self.realm_tier());
        let kind = if sin == Sin::Gluttony { EnemyKind::Gaki } else { biter_of(stage) };
        let facing = toward(at, self.hero.pos());
        let id = self.spawn_duelist(kind, at, facing, Lore::default());
        if let Some(id) = id {
            self.weigh_spawned(id, if kind.realm_own() { 0 } else { stage }, 1.0);
        }
        if let Some(m) = self.voices.deceit.mimic.as_mut() {
            m.biter = id;
        }
        self.audio.play(Sfx::Shriek);
        self.audio.play(Sfx::Growl);
        self.view.shake(0.3);
        self.floater(at.x, at.y - 0.8, "THE CHEST BITES", 1.6, Some(sin.color()));
    }

    // ------------------------------------------------------------------ capture hooks

    /// Capture hook (`whisper`): the prince whispers now (in the realm he is in, or Greed's).
    pub fn debug_whisper(&mut self) {
        let sin = self.voices.deceit.sin.or(self.realm).unwrap_or(Sin::Greed);
        self.voices.deceit.sin.get_or_insert(sin);
        self.voices.spoken = None;
        self.begin_whisper(sin);
    }

    /// Capture hook (`gatekeeper`): the false gatekeeper in this realm (dealt if he was not), and
    /// the hero a few steps from him, facing him.
    pub fn debug_gatekeeper(&mut self) {
        let sin = self.realm.unwrap_or(Sin::Greed);
        if self.voices.deceit.keeper.is_none() {
            self.voices.deceit.sin = Some(sin);
            for s in 0..8 {
                self.place_keeper(1234 + s * 977);
                if self.voices.deceit.keeper.is_some() {
                    break;
                }
            }
        }
        let Some(k) = self.voices.deceit.keeper.as_mut() else { return };
        let at = k.at;
        let mut stand = V::new(at.x, at.y + 2.4);
        if !self.level.grid.is_walkable(stand.x.round() as i32, stand.y.round() as i32, false) {
            stand = V::new(at.x, at.y - 2.4);
        }
        k.facing = toward(at, stand);
        k.armed = false;
        self.hero.place(P::new(stand.x.round() as i32, stand.y.round() as i32));
        let face = if stand.y > at.y { Dir::Up } else { Dir::Down };
        self.hero.face(face);
        self.view.aim_along(face);
    }

    /// Capture hook (`sign`): the realm's compass turned toward a den now.
    pub fn debug_sign(&mut self) {
        let sin = self.voices.deceit.sin.or(self.realm).unwrap_or(Sin::Greed);
        self.voices.deceit.sin.get_or_insert(sin);
        self.begin_sign();
    }

    /// Capture hook (`mimic`): a mimic chest a few steps before him.
    pub fn debug_mimic(&mut self) {
        let sin = self.voices.deceit.sin.or(self.realm).unwrap_or(Sin::Greed);
        self.voices.deceit.sin.get_or_insert(sin);
        let hp = self.hero.occupied();
        let spot = [P::new(hp.x, hp.y - 2), P::new(hp.x + 2, hp.y), P::new(hp.x - 2, hp.y), P::new(hp.x, hp.y + 2)]
            .into_iter()
            .find(|p| self.level.grid.is_walkable(p.x, p.y, false))
            .unwrap_or(hp);
        self.voices.deceit.mimic = Some(Mimic { at: spot.v(), done: false, biter: None, creak: 0.0 });
    }
}

/// The demon behind the false gatekeeper: the melee body of the realm's deepest stage.
pub fn demon_of(stage: u8) -> EnemyKind {
    use EnemyKind as K;
    [K::Skeleton, K::Blemmyes, K::Rasetsu, K::Oni, K::Karakoncolos, K::Gashadokuro, K::Surtr][(stage.clamp(1, 7) - 1) as usize]
}

/// What comes out of a mimic chest: a body foe of the stage, never a giant.
pub fn biter_of(stage: u8) -> EnemyKind {
    use EnemyKind as K;
    [K::Bat, K::Werewolf, K::DireWolf, K::Kappa, K::Albasti, K::Albasti, K::Albasti][(stage.clamp(1, 7) - 1) as usize]
}

#[cfg(test)]
mod tests {
    use super::super::realms::tests::{in_realm, STEP};
    use super::*;
    use crate::logic::progression::TREES;
    use crate::logic::war::drum::DrumPhase;

    /// A realm with its cheats dealt (the ones asked for), play under way, the Drum at rest.
    fn dealt(sin: Sin, cheats: &[CheatKind]) -> Game {
        let mut g = in_realm(sin);
        g.update(STEP);
        g.leave_deceit();
        g.voices.deceit.sin = Some(sin);
        g.voices.deceit.dealt = cheats.to_vec();
        for (k, c) in cheats.iter().enumerate() {
            match c {
                CheatKind::FalseGatekeeper => g.place_keeper(99 + k as u32),
                CheatKind::LoadedDice => g.place_mimic(7 + k as u32),
                _ => {}
            }
        }
        g.war.drum.phase = DrumPhase::Rest;
        // (The realm's own greeting off the panel.)
        g.voices.spoken = None;
        g.voices.counsel.pending.clear();
        g
    }

    /// Owned tiers of every tree, relics, lives: what the prince must never take.
    fn owned(g: &Game) -> (Vec<u8>, u32, u32, Vec<u8>) {
        let h = g.progress.hero(g.hero_kind);
        (TREES.iter().map(|&t| h.owned(t)).collect(), g.score.lives, g.progress.runes, vec![g.sins_cleared])
    }

    #[test]
    fn the_prince_never_takes_what_the_hero_owns() {
        let mut g = dealt(Sin::Gluttony, &[CheatKind::FalseGatekeeper, CheatKind::LoadedDice]);
        g.progress.silver = 1000;
        let before = owned(&g);
        // Believe the false gatekeeper: buy both, drink the bottle, swing the blade to pieces.
        let k = g.voices.deceit.keeper.clone().expect("a side room for him");
        g.voices.deceit.asking = Some(Asking::Keeper);
        g.answer_deceit(MenuItem::Choice("BUY THE ELIXIR"));
        g.voices.deceit.asking = Some(Asking::Keeper);
        g.answer_deceit(MenuItem::Choice("BUY THE BLADE"));
        assert_eq!(g.progress.silver, 1000 - k.wares[0].price() - k.wares[1].price(), "half price");
        assert_eq!(g.voices.deceit.tainted.blade, Some(BLADE_BLOWS));
        let bottles = |g: &Game| g.belt.iter().filter(|&&p| p == PotionKind::Strength).count();
        let on_belt = bottles(&g);
        assert!(on_belt >= 1, "the bottle on his belt");
        let base = g.brawl.as_ref().unwrap().might;
        g.update_deceit(STEP);
        assert!((g.brawl.as_ref().unwrap().might - base * BLADE_MIGHT).abs() < 1e-9, "the bright blade bites");
        // A second bottle of his, drunk: it turns.
        g.belt.push(PotionKind::Strength);
        g.voices.deceit.tainted.elixirs.push(PotionKind::Strength);
        g.belt.pop();
        g.war.read = vec![WarEvent::Drank { potion: PotionKind::Strength }];
        g.update_deceit(STEP);
        assert!(g.voices.deceit.tainted.turned > 0.0, "the elixir turns");
        assert!((g.brawl.as_ref().unwrap().might - base * BLADE_MIGHT * TURN_MIGHT).abs() < 1e-9);
        let hit = WarEvent::HeroHit {
            foe: 1,
            kind: EnemyKind::Gaki,
            effect: crate::logic::combat::fsm::Effect::Flinched(10),
            dmg: 1.0,
            finisher: false,
            dir: V::new(1.0, 0.0),
            at: V::new(0.0, 0.0),
            warded: 0.0,
        };
        g.war.read = vec![hit; BLADE_BLOWS as usize];
        g.update_deceit(STEP);
        assert!(g.voices.deceit.tainted.blade.is_none() && g.voices.deceit.tainted.shattered > 0.0, "it shatters after 30 blows");
        // Believe the dice: walk onto the chest, and it bites (a duelist, through the threat manager).
        let m = g.voices.deceit.mimic.clone().expect("a mimic");
        let n = g.enemies.len();
        g.hero.place(P::new(m.at.x.round() as i32, m.at.y.round() as i32));
        g.update_deceit(STEP);
        assert_eq!(g.enemies.len(), n + 1, "it bites");
        assert!(g.enemies.last().unwrap().duel.is_some());
        // Leave the realm: every false ware is gone, and nothing he owns went with it.
        g.war.read.clear();
        assert_eq!(g.voices.deceit.tainted.elixirs.len(), 1, "the bottle he bought, not yet drunk");
        g.leave_deceit();
        assert_eq!(bottles(&g), on_belt - 1, "the false bottle is gone from his belt");
        assert!(!g.voices.deceit.tainted.any());
        assert!((g.brawl.as_ref().unwrap().might - base).abs() < 1e-9, "his blows his own again");
        assert_eq!(owned(&g), before, "no tier, relic, rune or life taken");
    }

    #[test]
    fn striking_the_false_gatekeeper_reveals_a_demon_whose_death_pays_his_wares() {
        let mut g = dealt(Sin::Envy, &[CheatKind::FalseGatekeeper]);
        let k = g.voices.deceit.keeper.clone().expect("a side room for him");
        let lies = g.warband.save.lies;
        let silver = g.progress.silver;
        let n = g.enemies.len();
        g.hero.place(P::new(k.at.x.round() as i32, k.at.y.round() as i32 + 1));
        g.voices.deceit.swinging = false;
        let swing = Some((g.hero.pos(), toward(g.hero.pos(), k.at)));
        g.update_keeper(Sin::Envy, swing);
        assert_eq!(g.enemies.len(), n + 1, "a demon where he stood");
        let id = g.voices.deceit.keeper.as_ref().unwrap().demon.unwrap();
        let demon = g.enemies.last().unwrap().duel.as_ref().unwrap();
        assert_eq!((demon.id, demon.breed.kind), (id, demon_of(deepest_stage(g.realm_tier()))));
        g.war.read = vec![WarEvent::Slain { foe: id, kind: demon.breed.kind, at: k.at, dir: V::new(1.0, 0.0), last_of_pack: true }];
        g.update_keeper(Sin::Envy, None);
        assert_eq!(g.progress.silver, silver + wares_worth(&k.wares), "his wares' price in silver");
        assert_eq!(g.warband.save.lies, lies + 1, "a lie seen through");
    }

    #[test]
    fn a_whisper_is_seen_through_by_asking_the_old_man() {
        let mut g = dealt(Sin::Pride, &[CheatKind::Whisper]);
        g.voices.deceit.next_at = 0.0;
        g.voices.spoken = None;
        g.time += 100.0;
        g.update_deceit(STEP);
        let live = g.voices.deceit.whisper.clone().expect("the prince whispers at rest");
        let s = g.voices_line().expect("in the panel").clone();
        assert_eq!(s.who, Speaker::Whisper(Sin::Pride));
        assert_eq!(s.color(), Sin::Pride.color(), "in the sin's colour, not amber");
        let runes = g.progress.runes;
        g.call_old_man();
        assert_eq!(g.warband.save.lies, 1);
        assert_eq!(g.progress.runes, runes + deceit::SEEN_RUNES);
        let said = g.workshop_counsel().unwrap();
        assert!(said.starts_with("I said no such thing.") && said.contains(&live.w.truth), "{said}");
    }

    #[test]
    fn the_bargain_gives_what_it_says_and_takes_what_it_does_not() {
        let mut g = dealt(Sin::Sloth, &[CheatKind::CrookedBargain]);
        g.voices.deceit.next_at = 0.0;
        g.update_deceit(STEP);
        assert_eq!(g.screen, Screen::Toll, "the offer is put to him");
        let offer = g.run_offer().expect("through the offer screen");
        assert!(offer.title.contains("BELPHEGOR") && !offer.lines.join(" ").to_lowercase().contains("health"), "the price unsaid");
        let (hp, max) = (g.hero.hp, g.hero.max_hp);
        g.answer_deceit(MenuItem::Choice("ACCEPT"));
        assert_eq!(g.screen, Screen::Playing);
        assert!(g.hero.hp < hp && g.hero.hp >= 1, "the hidden price");
        assert!(hp - g.hero.hp <= (max as f64 * PRICE_SHARE).round() as i32 + 1, "bounded");
    }

    #[test]
    fn the_turned_sign_points_at_a_den_and_the_old_mans_lamp_stays_true() {
        let mut g = dealt(Sin::Lust, &[CheatKind::TurnedSign]);
        let (true_way, turned) = g.compass().unwrap();
        assert!(!turned);
        assert_eq!(Some(true_way), g.gate.map(|p| p.v()));
        g.debug_sign();
        let (way, turned) = g.compass().unwrap();
        assert!(turned && way != true_way, "turned toward a den");
        let lies = g.warband.save.lies;
        for _ in 0..((SIGN_SECONDS + 0.5) / STEP) as usize {
            g.update_sign(STEP);
        }
        assert!(g.voices.deceit.sign.is_none());
        assert_eq!(g.warband.save.lies, lies + 1, "not followed: seen through");
    }

    #[test]
    fn striking_a_mimic_first_doubles_its_loot() {
        let mut g = dealt(Sin::Gluttony, &[CheatKind::LoadedDice]);
        g.debug_mimic();
        let hp = g.hero.pos();
        g.voices.deceit.mimic.as_mut().unwrap().at = V::new(hp.x, hp.y - 1.2);
        let m = g.voices.deceit.mimic.clone().unwrap();
        let silver = g.progress.silver;
        let swing = Some((g.hero.pos(), toward(g.hero.pos(), m.at)));
        g.update_mimic(Sin::Gluttony, swing, STEP);
        assert!(g.voices.deceit.mimic.as_ref().unwrap().done);
        assert_eq!(g.progress.silver, silver + 2 * CHEST_SILVER);
        assert_eq!(g.warband.save.lies, 1);
    }

    #[test]
    fn the_bot_refuses_whispers_and_strikes_the_false_gatekeeper_and_the_mimic() {
        use crate::game::bot::{Bot, Skill};
        let mut g = dealt(Sin::Greed, &[CheatKind::FalseGatekeeper]);
        g.hero.free_move = true;
        // (Nothing hunting it: it sees through cheats between fights.)
        g.enemies.clear();
        g.debug_gatekeeper();
        let mut bot = Bot::new(Skill::Expert, 3);
        for _ in 0..(6.0 / STEP) as usize {
            if g.screen != Screen::Playing {
                break;
            }
            let pad = bot.act(&g);
            g.input.set_pad(Some(pad));
            g.update(STEP);
            if g.voices.deceit.keeper.as_ref().is_some_and(|k| k.demon.is_some()) {
                break;
            }
        }
        assert!(g.voices.deceit.keeper.as_ref().unwrap().demon.is_some(), "it struck him rather than trade");
        assert_eq!(g.screen, Screen::Playing, "and never stood to hear his wares");
        // A mimic before it: struck, never opened.
        let mut g = dealt(Sin::Gluttony, &[CheatKind::LoadedDice]);
        g.hero.free_move = true;
        g.enemies.clear();
        g.debug_mimic();
        let n = g.enemies.len();
        for _ in 0..(6.0 / STEP) as usize {
            let pad = bot.act(&g);
            g.input.set_pad(Some(pad));
            g.update(STEP);
            if g.voices.deceit.mimic.as_ref().is_some_and(|m| m.done) {
                break;
            }
        }
        assert!(g.voices.deceit.mimic.as_ref().unwrap().done && g.enemies.len() == n, "struck first: no bite");
        // A whisper in the air: it asks the old man.
        let mut g = dealt(Sin::Pride, &[CheatKind::Whisper]);
        g.enemies.clear();
        g.debug_whisper();
        let pad = bot.act(&g);
        g.input.set_pad(Some(pad));
        g.update(STEP);
        assert_eq!((g.screen, g.warband.save.lies), (Screen::Shop, 1), "called him, and he denied it");
    }

    #[test]
    fn every_realm_has_a_side_room_for_the_false_gatekeeper_and_a_spot_for_a_chest() {
        for sin in crate::logic::sins::SINS.into_iter().filter(|&s| CheatKind::FalseGatekeeper.fits(s)) {
            let mut g = dealt(sin, &[CheatKind::FalseGatekeeper, CheatKind::LoadedDice]);
            let k = g.voices.deceit.keeper.clone().unwrap_or_else(|| panic!("{sin:?}: no room for him"));
            let t = P::new(k.at.x.round() as i32, k.at.y.round() as i32);
            assert!(g.level.grid.is_walkable(t.x, t.y, false), "{sin:?}");
            if CheatKind::LoadedDice.fits(sin) {
                assert!(g.voices.deceit.mimic.is_some(), "{sin:?}: a chest");
            }
            // He is still there after a few seconds of the realm's own rules.
            for _ in 0..(3.0 / STEP) as usize {
                g.update(STEP);
            }
            assert!(g.voices.deceit.keeper.is_some(), "{sin:?}");
        }
    }
}
