//! The Prince's Hands: the generals (WARBAND.md §9). Each realm has a general, a named commander
//! serving its prince, who runs the realm against the hero by orders the player can see and
//! counter. This module is the general's mind and his realm's plan, free of the game: who the
//! seven are, the four kinds of order and the instruments they work through, when an order may be
//! given (only while the Drum builds, one at a time, each announced a second or more before it
//! bites), the counter-order he answers a lost instrument with, what his decree and his death do
//! to the prince's fight, where his post and his instruments stand, and what the run keeps of him.
//!
//! `game::general` plays it: the general himself and his guard, the patrols, the mustered packs,
//! the kindled floor, the decree's carriers, the instruments broken by the hero's blows.

use std::collections::{HashSet, VecDeque};

use super::super::enemy::EnemyKind;
use super::super::grid::Grid;
use super::super::hub::Rect;
use super::super::rng::Rng;
use super::super::sins::{Sin, SINS};
use super::super::types::{CARDINALS, P};

/// No order bites less than this many frames after it is announced (I7: a second, by eye and ear).
pub const MIN_LEAD: u32 = 60;
/// Frames of the Drum's build before a general gives his first order.
pub const FIRST_ORDER: u32 = 240;
/// Frames he waits after an order is over before he gives the next.
pub const COOLDOWN: u32 = 1500;
/// Frames a counter-order is announced before it is carried out.
pub const COUNTER_LEAD: u32 = 90;
/// Blows of his that break an instrument.
pub const BLOWS: u8 = 3;
/// A kindled hazard is marked on the floor this long before it bursts (three seconds).
pub const KINDLE_FUSE: u32 = 180;
/// Frames between one kindled volley's marking and the next.
pub const KINDLE_EVERY: u32 = 150;
/// The radius of one kindled mark (tiles), and how far from the zone's middle they fall.
pub const KINDLE_RADIUS: f64 = 0.95;
pub const KINDLE_ZONE: f64 = 4.5;
/// Marks in a volley.
pub const KINDLE_MARKS: usize = 5;
/// A patrol sees him this close (tiles); its whistle blows this long, then wakes the dens this near.
pub const PATROL_SIGHT: f64 = 7.0;
pub const WHISTLE: u32 = 60;
pub const WHISTLE_WAKES: f64 = 10.0;
/// Frames he stands at a glowing gate to bar it.
pub const BAR_FRAMES: u32 = 90;
/// The decree's carriers walk this fast (tiles a second); the decree has at least this long.
pub const CARRIER_PACE: f64 = 1.6;
pub const DECREE_LEAST: u32 = 1800;
/// The general's health over his breed's (and the realm's weight on it), but never past a lesser
/// prince's (§9.2: about 300 before the weight).
pub const GENERAL_HEALTH: f64 = 2.5;
pub const GENERAL_HP: f64 = 300.0;

/// What the general's breed's health is multiplied by.
pub fn general_health(body: EnemyKind) -> f64 {
    let base = crate::logic::combat::Breed::of(body).stats().hp.max(1.0);
    GENERAL_HEALTH.min(GENERAL_HP / base).max(1.0)
}
/// A mustered pack (more for each gate doubled), and the watch's patrol.
pub const MUSTER: usize = 2;
pub const PATROL: usize = 3;
/// Id numbers of the kindled volleys in the threat manager (above every duelist's).
pub const HAZARD_IDS: u32 = 0xf000_0000;

/// The four kinds of order (§9.1).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum OrderKind {
    /// Patrols walk between rooms; one that sees him whistles, and the dens near wake.
    Watch,
    /// A horn sounds, a gate glows, a pack marches in.
    Muster,
    /// The realm's hazard wakes on the floor about him, marked before it bursts.
    Kindle,
    /// A scheme carried to the court on a timer: whatever it finishes, the prince fights with.
    Decree,
}

impl OrderKind {
    /// What the card says he does.
    pub fn verb(self) -> &'static str {
        match self {
            OrderKind::Watch => "SETS THE WATCH",
            OrderKind::Muster => "MUSTERS",
            OrderKind::Kindle => "KINDLES",
            OrderKind::Decree => "DECREES",
        }
    }
    /// One letter for its icon on the banner.
    pub fn letter(self) -> &'static str {
        match self {
            OrderKind::Watch => "W",
            OrderKind::Muster => "M",
            OrderKind::Kindle => "K",
            OrderKind::Decree => "D",
        }
    }
    /// Frames from its announcement to its bite: the whistle, the horn's four seconds, the
    /// bellows' wheeze (then each mark its own three seconds), the herald's cry.
    pub fn lead(self) -> u32 {
        match self {
            OrderKind::Watch => 90,
            OrderKind::Muster => 240,
            OrderKind::Kindle => 120,
            OrderKind::Decree => 120,
        }
    }
    /// Frames it runs once it bites (the decree's own time is its carriers' walk).
    pub fn live(self) -> u32 {
        match self {
            OrderKind::Watch => 2400,
            OrderKind::Muster => 90,
            OrderKind::Kindle => KINDLE_EVERY * 3 + KINDLE_FUSE / 2,
            OrderKind::Decree => DECREE_LEAST,
        }
    }
}

/// Where an order stands.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum OrderStage {
    /// Standing ready: he may give it.
    #[default]
    Ready,
    /// Given and announced: it bites in `left` frames.
    Announced { left: u32 },
    /// Under way, for `left` frames more.
    Live { left: u32 },
    /// Its instrument is broken (or he is slain): never again.
    Broken,
    /// The decree carried to the court: done for good.
    Done,
}

/// The things in the realm an order works through, which the hero can break.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum InstrumentKind {
    /// A brazier on a post: the watch's fire.
    WatchFire,
    /// A war horn on its post.
    WarHorn,
    /// A bell in a frame (the counting-bell, the drowned bell).
    Bell,
    /// The bellows (or valve) that wakes the realm's hazard.
    Bellows,
    /// A chain on its winch: the decree's way to the court.
    Chain,
}

impl InstrumentKind {
    /// The one each kind of order works through.
    pub fn of(order: OrderKind, bell: bool) -> InstrumentKind {
        match order {
            OrderKind::Watch => InstrumentKind::WatchFire,
            OrderKind::Muster if bell => InstrumentKind::Bell,
            OrderKind::Muster => InstrumentKind::WarHorn,
            OrderKind::Kindle => InstrumentKind::Bellows,
            OrderKind::Decree => InstrumentKind::Chain,
        }
    }
}

/// One instrument in the realm: which order it serves (an index into the general's three), where it
/// stands, the blows it has left, and whether it is broken or (the bellows) turned on his foes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Instrument {
    pub id: u16,
    pub kind: InstrumentKind,
    pub order: usize,
    pub tile: P,
    pub blows: u8,
    pub broken: bool,
    pub turned: bool,
}

/// One of the seven (§9.3): his realm, his name, the body he wears and its colour, his three
/// orders and what each one's instrument is called, the realm's hazard, and his decree.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct General {
    pub sin: Sin,
    pub name: &'static str,
    pub body: EnemyKind,
    /// His cloak's colour (and his banner's field).
    pub cloak: u32,
    pub orders: [OrderKind; 3],
    /// What each order's instrument is called, in the order of `orders`.
    pub instruments: [&'static str; 3],
    /// The realm's hazard his kindling wakes, and its colour.
    pub hazard: &'static str,
    pub hazard_color: u32,
    /// What his decree does (the card's words), and what it does to the prince's fight.
    pub decree: &'static str,
    pub decree_done: &'static str,
}

impl General {
    /// The general of `sin`'s realm.
    pub fn of(sin: Sin) -> General {
        use EnemyKind as K;
        use OrderKind::*;
        match sin {
            Sin::Greed => General {
                sin,
                name: "THE ASSESSOR",
                body: K::Blemmyes,
                cloak: 0xd8a830,
                orders: [Muster, Kindle, Decree],
                instruments: ["THE COUNTING-BELL", "THE COIN-BELLOWS", "THE TREASURY CHAIN"],
                hazard: "COIN TRAPS",
                hazard_color: 0xf2c53d,
                decree: "THE TREASURERS EMPTY THE CHESTS INTO THE COURT",
                decree_done: "MAMMON'S GOLEMS RISE AT ONCE",
            },
            Sin::Pride => General {
                sin,
                name: "THE CHAMBERLAIN OF GLASS",
                body: K::Tengu,
                cloak: 0xf0e8d0,
                orders: [Watch, Kindle, Decree],
                instruments: ["THE MIRROR-FIRE", "THE LENS-BELLOWS", "THE THRONE CHAIN"],
                hazard: "LIGHT BEAMS",
                hazard_color: 0xfff4d0,
                decree: "THE SERVANTS POLISH THE THRONE-MIRROR",
                decree_done: "THE PEERLESS GAINS HIS CROWN",
            },
            Sin::Envy => General {
                sin,
                name: "THE BELL-WARDEN",
                body: K::Kappa,
                cloak: 0x4a9a7a,
                orders: [Kindle, Muster, Decree],
                instruments: ["THE GEYSER VALVE", "THE DROWNED BELL", "THE BELL CHAIN"],
                hazard: "GEYSERS",
                hazard_color: 0x5ad8c0,
                decree: "THE BELL IS LOWERED INTO THE DEEP",
                decree_done: "THE COURT'S WATER RISES",
            },
            Sin::Wrath => General {
                sin,
                name: "MARSHAL GOREBRAND",
                body: K::Oni,
                cloak: 0x9a1a10,
                orders: [Muster, Kindle, Decree],
                instruments: ["THE WAR HORN", "THE LAVA SLUICE", "THE BRIDGE CHAIN"],
                hazard: "LAVA SLUICES",
                hazard_color: 0xff6a20,
                decree: "THE SAPPERS BURN THE BRIDGES",
                decree_done: "AMON FIGHTS WITH THE BRIDGES BURNT",
            },
            Sin::Sloth => General {
                sin,
                name: "THE LAMPLIGHTER",
                body: K::Karakoncolos,
                cloak: 0x5a6a8a,
                orders: [Watch, Kindle, Decree],
                instruments: ["THE LANTERN-FIRE", "THE LAMP-BELLOWS", "THE HOURGLASS CHAIN"],
                hazard: "SMOULDERING LAMPS",
                hazard_color: 0x9ab0e0,
                decree: "THE HOURGLASS IS WOUND",
                decree_done: "BELPHEGOR WAKES RESTED",
            },
            Sin::Gluttony => General {
                sin,
                name: "THE STEWARD OF THE LAST COURSE",
                body: K::Gaki,
                cloak: 0xd8c8a0,
                orders: [Muster, Kindle, Decree],
                instruments: ["THE DINNER GONG", "THE ACID VALVE", "THE LARDER CHAIN"],
                hazard: "ACID TIDES",
                hazard_color: 0x9ae040,
                decree: "THE COOKS LAY THE PRINCE'S FEAST",
                decree_done: "BEELZEBUB EATS AND IS HEALED",
            },
            Sin::Lust => General {
                sin,
                name: "THE BRIDESMAID",
                body: K::Albasti,
                cloak: 0xf0d0e0,
                orders: [Watch, Kindle, Decree],
                instruments: ["THE PROCESSION'S TORCH", "THE CENSER", "THE VOW-CHAIN"],
                hazard: "CENSER SMOKE",
                hazard_color: 0xff7ab0,
                decree: "THE VOWS ARE SPOKEN",
                decree_done: "THE BLACK THORN'S GATE SEALS",
            },
        }
    }

    /// His muster sounds a bell (or a gong), not a horn.
    pub fn bell(&self) -> bool {
        matches!(self.sin, Sin::Greed | Sin::Envy | Sin::Gluttony)
    }

    /// The card's line for an order of his: "THE ASSESSOR MUSTERS".
    pub fn card(&self, order: OrderKind) -> String {
        format!("{} {}", self.name, order.verb())
    }

    /// The order he gives by index, and where it is in his three, for `kind`.
    pub fn index_of(&self, kind: OrderKind) -> Option<usize> {
        self.orders.iter().position(|&o| o == kind)
    }
}

/// What the prince's fight is, after his general's decree and his death (§9.2, §17 decision 5).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PrinceTerms {
    /// His adds come as his fight begins (his general's decree done): Mammon's golems climb out
    /// of the gold at once.
    pub adds_at_start: bool,
    /// His second phase brings its adds (his general lives).
    pub phase_two_adds: bool,
    /// His health as a multiple (a prince still stood in by a Cyclops takes his general's decree
    /// and death this way: a decree done raises it, a general slain lowers it).
    pub health: f64,
}

impl PrinceTerms {
    pub const WHOLE: PrinceTerms = PrinceTerms { adds_at_start: false, phase_two_adds: true, health: 1.0 };
}

/// The prince of `sin`'s fight, his general's decree done or not, his general slain or not. In a
/// court built to its own plan the decree brings his adds at once and his general's death loses
/// him those of his second phase; a prince stood in by a Cyclops takes them on his health.
pub fn terms(sin: Sin, decree_done: bool, slain: bool) -> PrinceTerms {
    if crate::logic::court::built(sin) {
        return PrinceTerms { adds_at_start: decree_done, phase_two_adds: !slain, health: 1.0 };
    }
    let health = 1.0 + if decree_done { 0.25 } else { 0.0 } - if slain { 0.2 } else { 0.0 };
    PrinceTerms { adds_at_start: false, phase_two_adds: true, health }
}

/// How the general answers the loss of an instrument (once for each).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Counter {
    /// A guard pack goes from his post to the instrument at this index, and holds it.
    GuardTo(usize),
    /// The muster's gates are doubled: every pack it brings is bigger by this many.
    DoubleMuster,
}

/// What the general's mind tells the game this frame.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cue {
    /// Order `order` is given: its horn, its card, its ping; it bites in `lead` frames.
    Announce { order: usize, lead: u32 },
    /// It bites now.
    Bite { order: usize },
    /// It is over, standing at `stage` (ready again, broken or done).
    Over { order: usize, stage: OrderStage },
    /// A counter-order is announced, to be carried out in `lead` frames.
    Answer { counter: Counter, lead: u32 },
    /// The counter-order is carried out.
    Answered { counter: Counter },
}

/// The general's mind through a realm: his orders, his instruments, when he may give the next
/// order, and the counter-orders he owes. One step a frame; deterministic per seed.
#[derive(Clone, Debug)]
pub struct Command {
    pub general: General,
    pub stages: [OrderStage; 3],
    /// Times each order has been given, and has bitten.
    pub given: [u32; 3],
    pub instruments: [Instrument; 3],
    /// The order under way (announced or live), if any: one at a time.
    pub running: Option<usize>,
    /// Frames of the build before he may give the next order.
    pub wait: u32,
    pub slain: bool,
    pub decree_done: bool,
    /// Frames his decree has (its carriers' walk to the court).
    pub decree_frames: u32,
    /// The counter-order announced and not yet carried out, and those carried out.
    pub answer: Option<(Counter, u32)>,
    pub answers: u32,
    /// Packs the muster brings bigger, by the gates doubled.
    pub muster_extra: usize,
    /// The order he gives next, as he goes round his three.
    next: usize,
    pub frame: u32,
}

impl Command {
    /// The general of `sin` with his instruments at `tiles` (in the order of his orders).
    pub fn new(sin: Sin, tiles: [P; 3]) -> Command {
        let general = General::of(sin);
        let instruments = std::array::from_fn(|k| Instrument {
            id: k as u16,
            kind: InstrumentKind::of(general.orders[k], general.bell()),
            order: k,
            tile: tiles[k],
            blows: BLOWS,
            broken: false,
            turned: false,
        });
        Command {
            general,
            stages: [OrderStage::Ready; 3],
            given: [0; 3],
            instruments,
            running: None,
            wait: FIRST_ORDER,
            slain: false,
            decree_done: false,
            decree_frames: DECREE_LEAST,
            answer: None,
            answers: 0,
            muster_extra: 0,
            next: 0,
            frame: 0,
        }
    }

    /// The frames from order `k`'s announcement to its bite.
    pub fn lead(&self, k: usize) -> u32 {
        self.general.orders[k].lead().max(MIN_LEAD)
    }

    fn live(&self, k: usize) -> u32 {
        match self.general.orders[k] {
            OrderKind::Decree => self.decree_frames.max(DECREE_LEAST),
            o => o.live(),
        }
    }

    /// He may give order `k` now (its instrument whole, a decree not yet done).
    pub fn can_give(&self, k: usize) -> bool {
        !self.slain && self.stages[k] == OrderStage::Ready && !self.instruments[k].broken
    }

    /**
     * One frame. `build`: the Drum is in its build, the only phase in which a new order may be
     * given (§6.2); an order already announced runs on whatever the Drum does.
     */
    pub fn step(&mut self, build: bool) -> Vec<Cue> {
        self.step_with(build, [true; 3])
    }

    /// One frame, as `step`, where the game says which orders the field has room for now (`room`:
    /// a muster that would crowd the fight past its bounds waits).
    pub fn step_with(&mut self, build: bool, room: [bool; 3]) -> Vec<Cue> {
        let mut cues = Vec::new();
        self.frame += 1;
        if self.slain {
            return cues;
        }
        if let Some((c, left)) = self.answer.as_mut() {
            *left = left.saturating_sub(1);
            if *left == 0 {
                let counter = *c;
                self.answer = None;
                self.answers += 1;
                if counter == Counter::DoubleMuster {
                    self.muster_extra += MUSTER;
                }
                cues.push(Cue::Answered { counter });
            }
        }
        match self.running {
            Some(k) => match self.stages[k] {
                OrderStage::Announced { left } if left <= 1 => {
                    self.stages[k] = OrderStage::Live { left: self.live(k) };
                    cues.push(Cue::Bite { order: k });
                }
                OrderStage::Announced { left } => self.stages[k] = OrderStage::Announced { left: left - 1 },
                OrderStage::Live { left } if left <= 1 => {
                    let stage = if self.general.orders[k] == OrderKind::Decree {
                        self.decree_done = true;
                        OrderStage::Done
                    } else {
                        OrderStage::Ready
                    };
                    cues.push(self.end(k, stage));
                }
                OrderStage::Live { left } => self.stages[k] = OrderStage::Live { left: left - 1 },
                _ => self.running = None,
            },
            None => {
                if build {
                    self.wait = self.wait.saturating_sub(1);
                }
                if build && self.wait == 0 {
                    if let Some(k) = (0..3).map(|i| (self.next + i) % 3).find(|&k| room[k] && self.can_give(k)) {
                        self.next = (k + 1) % 3;
                        let lead = self.lead(k);
                        self.stages[k] = OrderStage::Announced { left: lead };
                        self.given[k] += 1;
                        self.running = Some(k);
                        cues.push(Cue::Announce { order: k, lead });
                    }
                }
            }
        }
        cues
    }

    /// Order `k` is over, standing at `stage`; he rests before the next.
    fn end(&mut self, k: usize, stage: OrderStage) -> Cue {
        self.stages[k] = stage;
        if self.running == Some(k) {
            self.running = None;
            self.wait = COOLDOWN;
        }
        Cue::Over { order: k, stage }
    }

    /// The game saw order `k` fail (its whistler slain, its gate barred, its carriers cut down): it
    /// is over, and may be given again.
    pub fn fail(&mut self, k: usize) -> Option<Cue> {
        matches!(self.stages[k], OrderStage::Announced { .. } | OrderStage::Live { .. }).then(|| self.end(k, OrderStage::Ready))
    }

    /// The game saw order `k` done early (the decree's carriers all reached the court).
    pub fn finish(&mut self, k: usize) -> Option<Cue> {
        if !matches!(self.stages[k], OrderStage::Live { .. }) {
            return None;
        }
        let decree = self.general.orders[k] == OrderKind::Decree;
        self.decree_done |= decree;
        Some(self.end(k, if decree { OrderStage::Done } else { OrderStage::Ready }))
    }

    /**
     * A blow of his lands on instrument `k`. When it breaks, its order is broken for good (ended at
     * once if it was under way), and he owes one counter-order: a guard pack to the next instrument
     * still whole, or, if that is the muster's, its gates doubled. Returns what to tell the game.
     */
    pub fn strike(&mut self, k: usize) -> Vec<Cue> {
        let mut cues = Vec::new();
        let i = &mut self.instruments[k];
        if i.broken {
            return cues;
        }
        i.blows = i.blows.saturating_sub(1);
        if i.blows > 0 {
            return cues;
        }
        i.broken = true;
        if self.stages[k] != OrderStage::Done {
            if matches!(self.stages[k], OrderStage::Announced { .. } | OrderStage::Live { .. }) {
                cues.push(self.end(k, OrderStage::Broken));
            } else {
                self.stages[k] = OrderStage::Broken;
            }
        }
        if self.slain {
            return cues;
        }
        let whole = (1..3).map(|d| (k + d) % 3).find(|&j| !self.instruments[j].broken && self.stages[j] != OrderStage::Done);
        if let Some(j) = whole {
            let counter = if self.general.orders[j] == OrderKind::Muster { Counter::DoubleMuster } else { Counter::GuardTo(j) };
            // (A second loss before the first answer is carried out replaces it: one answer at a time.)
            self.answer = Some((counter, COUNTER_LEAD));
            cues.push(Cue::Answer { counter, lead: COUNTER_LEAD });
        }
        cues
    }

    /// The bellows turned on his own: its kindling burns his foes from now on (it stays whole).
    pub fn turn(&mut self, k: usize) -> bool {
        let i = &mut self.instruments[k];
        if i.broken || i.turned || i.kind != InstrumentKind::Bellows {
            return false;
        }
        i.turned = true;
        true
    }

    /// He is slain: every order still standing ends at once, and no answer comes.
    pub fn slay(&mut self) -> Vec<Cue> {
        let mut cues = Vec::new();
        if self.slain {
            return cues;
        }
        if let Some(k) = self.running {
            cues.push(self.end(k, OrderStage::Broken));
        }
        self.slain = true;
        self.answer = None;
        cues
    }

    /// Instruments broken.
    pub fn broken(&self) -> usize {
        self.instruments.iter().filter(|i| i.broken).count()
    }

    /// What the prince fights with, as things stand.
    pub fn terms(&self) -> PrinceTerms {
        terms(self.general.sin, self.decree_done, self.slain)
    }

    /// What the run keeps of him.
    pub fn record(&self) -> Record {
        let broken = self.instruments.iter().enumerate().filter(|(_, i)| i.broken).fold(0u8, |a, (k, _)| a | 1 << k);
        Record { met: true, broken, decree_done: self.decree_done, slain: self.slain }
    }

    /// As the run left him: instruments broken stay broken, a decree done stays done.
    pub fn restore(&mut self, r: Record) {
        for k in 0..3 {
            if r.broken & (1 << k) != 0 {
                self.instruments[k].broken = true;
                self.instruments[k].blows = 0;
                self.stages[k] = OrderStage::Broken;
            }
        }
        if r.decree_done {
            self.decree_done = true;
            if let Some(k) = self.general.index_of(OrderKind::Decree) {
                self.stages[k] = OrderStage::Done;
            }
        }
        self.slain = r.slain;
    }
}

// ------------------------------------------------------------------ the run's record

/// What the run keeps of one realm's general (`vh.warband`, its `generals:` section).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Record {
    /// He has been met (his realm entered).
    pub met: bool,
    /// His instruments broken, a bit each (in the order of his orders).
    pub broken: u8,
    pub decree_done: bool,
    pub slain: bool,
}

/// Every realm's general, as the run keeps them (in the order of `SINS`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Records(pub [Record; 7]);

impl Records {
    pub fn get(&self, sin: Sin) -> Record {
        self.0[sin.index()]
    }
    pub fn set(&mut self, sin: Sin, r: Record) {
        self.0[sin.index()] = r;
    }

    /// The section's text: `I.B.D.S` for each realm whose general has been met (I the realm's
    /// index, B the broken bits, D and S 0 or 1), joined by commas. No '|' and no ':'.
    pub fn encode(&self) -> String {
        self.0
            .iter()
            .enumerate()
            .filter(|(_, r)| r.met)
            .map(|(i, r)| format!("{i}.{}.{}.{}", r.broken, u8::from(r.decree_done), u8::from(r.slain)))
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Reads a section back; what it cannot read is left out.
    pub fn decode(text: &str) -> Records {
        let mut out = Records::default();
        for entry in text.split(',').filter(|e| !e.is_empty()) {
            let n: Vec<u8> = entry.split('.').filter_map(|v| v.parse().ok()).collect();
            if let [i, broken, d, s] = n[..] {
                if let Some(r) = out.0.get_mut(i as usize) {
                    *r = Record { met: true, broken: broken & 7, decree_done: d == 1, slain: s == 1 };
                }
            }
        }
        out
    }
}

// ------------------------------------------------------------------ the realm's plan

/// Where a general's realm puts him and his instruments (§9.3: "the realm's posts").
#[derive(Clone, Debug, PartialEq)]
pub struct Posts {
    /// His command post: where he stands, in the room he holds.
    pub post: P,
    pub room: Rect,
    /// Each instrument's tile, in the order of his orders.
    pub instruments: [P; 3],
    /// The gates a muster marches in through.
    pub gates: [P; 2],
    /// The watch's round from its fire and back, tile by tile (empty if he sets no watch).
    pub round: Vec<P>,
    /// The decree's way from his post to the court's portal, tile by tile.
    pub court: Vec<P>,
}

/// The shortest walk from `a` to `b` over open floor, both ends included.
pub fn walk(grid: &Grid, a: P, b: P) -> Option<Vec<P>> {
    let mut prev: std::collections::HashMap<P, P> = std::collections::HashMap::from([(a, a)]);
    let mut queue = VecDeque::from([a]);
    while let Some(p) = queue.pop_front() {
        if p == b {
            let mut path = vec![b];
            let mut at = b;
            while at != a {
                at = prev[&at];
                path.push(at);
            }
            path.reverse();
            return Some(path);
        }
        for d in CARDINALS {
            if grid.can_move(p, d, false, false) {
                let n = grid.neighbor(p, d);
                if let std::collections::hash_map::Entry::Vacant(e) = prev.entry(n) {
                    e.insert(p);
                    queue.push_back(n);
                }
            }
        }
    }
    None
}

/// The open tile in `room` nearest its middle (pulled toward `toward` by `pull` tiles), off `busy`.
fn spot_in(grid: &Grid, room: Rect, reach: &HashSet<P>, busy: &HashSet<P>, pull: (f64, f64)) -> Option<P> {
    let (cx, cy) = room.center();
    let (gx, gy) = (cx as f64 + pull.0, cy as f64 + pull.1);
    // Clear of walls on every side if it can be, so a prop and a fight both have room (a narrow
    // path's tiles have floor on two sides at least).
    let open = |p: &P| CARDINALS.iter().filter(|&&d| grid.is_walkable(p.step(d).x, p.step(d).y, false)).count();
    room.tiles().filter(|p| reach.contains(p) && grid.is_walkable(p.x, p.y, false) && !busy.contains(p) && open(p) >= 2).min_by(|a, b| {
        let da = (a.x as f64 - gx).powi(2) + (a.y as f64 - gy).powi(2) + if open(a) < 4 { 400.0 } else { 0.0 };
        let db = (b.x as f64 - gx).powi(2) + (b.y as f64 - gy).powi(2) + if open(b) < 4 { 400.0 } else { 0.0 };
        da.total_cmp(&db).then(a.y.cmp(&b.y)).then(a.x.cmp(&b.x))
    })
}

/// The floor in `reach` cut into squares of about 14 tiles, those with floor enough to fight on.
fn squares(reach: &HashSet<P>) -> Vec<Rect> {
    const SIDE: i32 = 14;
    let (Some(x0), Some(y0)) = (reach.iter().map(|p| p.x).min(), reach.iter().map(|p| p.y).min()) else { return Vec::new() };
    let (x1, y1) = (reach.iter().map(|p| p.x).max().unwrap_or(x0), reach.iter().map(|p| p.y).max().unwrap_or(y0));
    let mut out = Vec::new();
    for cy in 0..=(y1 - y0) / SIDE {
        for cx in 0..=(x1 - x0) / SIDE {
            let r = Rect { x0: x0 + cx * SIDE, y0: y0 + cy * SIDE, x1: x0 + cx * SIDE + SIDE - 1, y1: y0 + cy * SIDE + SIDE - 1 };
            if r.tiles().filter(|p| reach.contains(p)).count() >= 40 {
                out.push(r);
            }
        }
    }
    out
}

/// The floor a realm's fights are on: what he walks to from where he arrives, and what its foes
/// walk to from their posts (a realm whose parts are joined by other ways than floor, as Pride's
/// galleries by their mirrors, has more than one).
pub fn ground(grid: &Grid, arrival: P, foes: &[P]) -> HashSet<P> {
    let mut reach = super::super::pathing::reachable(grid, arrival, false, false);
    for &p in foes {
        if !reach.contains(&p) && grid.is_walkable(p.x, p.y, false) {
            reach.extend(super::super::pathing::reachable(grid, p, false, false));
        }
    }
    reach
}

/**
 * Picks the general's post and his instruments in a realm, by `seed`: his post in a room past the
 * middle of the way (not the last, where the court's portal is), each instrument in a room of its
 * own, the muster's gates on open floor between rooms, the watch's round through two rooms and
 * back, and the decree's way to the court's portal (`court`, the tile before it). Every one is
 * reached from `arrival`. `rooms` are the realm's rooms (or its dens), the entry's left out.
 */
pub fn place(grid: &Grid, arrival: P, court: P, rooms: &[Rect], foes: &[P], orders: [OrderKind; 3], seed: u32) -> Option<Posts> {
    let mut rng = Rng::new(seed ^ 0x9e3b_1c55);
    let reach = ground(grid, arrival, foes);
    let far = |r: &Rect| {
        let (x, y) = r.center();
        (x as f64 - arrival.x as f64).hypot(y as f64 - arrival.y as f64)
    };
    let near_court = |r: &Rect| {
        let (x, y) = r.center();
        (x as f64 - court.x as f64).hypot(y as f64 - court.y as f64)
    };
    let span = (court.x as f64 - arrival.x as f64).hypot(court.y as f64 - arrival.y as f64).max(1.0);
    let mut rooms: Vec<Rect> = rooms.iter().copied().filter(|r| r.tiles().any(|p| reach.contains(&p))).collect();
    if rooms.len() < 4 {
        // (Too few rooms on foot from where he arrives: a realm whose other parts are reached some
        // other way, as Pride's galleries by their mirrors. The floor he can walk is cut into
        // squares instead, and those with floor enough serve as rooms.)
        rooms = squares(&reach);
    }
    if rooms.len() < 4 {
        return None;
    }
    // (Sorted by how far along the way they lie, so the seed picks the same way every time.)
    rooms.sort_by(|a, b| far(a).total_cmp(&far(b)).then(a.x0.cmp(&b.x0)).then(a.y0.cmp(&b.y0)));
    let mut busy: HashSet<P> = HashSet::from([arrival, court]);
    // His post: past the middle of the way, not the court's own room.
    // (Of those, the smaller half: a general's room holds a fight, not a pile-on.)
    let mut posts: Vec<Rect> = rooms.iter().copied().filter(|r| far(r) >= span * 0.45 && near_court(r) >= span * 0.2).collect();
    let area = |r: &Rect| (r.x1 - r.x0 + 1) * (r.y1 - r.y0 + 1);
    posts.sort_by(|a, b| area(a).cmp(&area(b)).then(far(a).total_cmp(&far(b))).then(a.x0.cmp(&b.x0)));
    posts.truncate(posts.len().div_ceil(2).max(1));
    let pool = if posts.is_empty() { &rooms } else { &posts };
    let room = pool[rng.index(pool.len())];
    // He stands where one of the room's own foes stood (the realm keeps its foes' posts on its
    // ground, however that ground moves), else on its open floor.
    let (cx, cy) = room.center();
    let mut theirs: Vec<P> = foes.iter().copied().filter(|p| room.contains(*p) && reach.contains(p)).collect();
    theirs.sort_by(|a, b| {
        let d = |p: &P| (p.x as f32 - cx).powi(2) + (p.y as f32 - cy).powi(2);
        d(a).total_cmp(&d(b)).then(a.y.cmp(&b.y)).then(a.x.cmp(&b.x))
    });
    let post = match theirs.first() {
        Some(&p) => p,
        None => spot_in(grid, room, &reach, &busy, (0.0, 0.0))?,
    };
    busy.insert(post);
    busy.extend(foes.iter().copied());
    // Each instrument in a room of its own, spread along the way.
    let mut others: Vec<Rect> = rooms.iter().copied().filter(|r| *r != room).collect();
    rng.shuffle(&mut others);
    let mut chosen: Vec<Rect> = Vec::new();
    for r in &others {
        if chosen.len() == 3 {
            break;
        }
        let (x, y) = r.center();
        let apart = chosen.iter().all(|c| {
            let (cx, cy) = c.center();
            (cx - x).hypot(cy - y) >= 14.0
        });
        if apart {
            chosen.push(*r);
        }
    }
    for r in &others {
        if chosen.len() < 3 && !chosen.contains(r) {
            chosen.push(*r);
        }
    }
    if chosen.len() < 3 {
        return None;
    }
    let mut tiles = [post; 3];
    for k in 0..3 {
        let pull = (rng.range(-2.0, 2.0), rng.range(-2.0, 2.0));
        tiles[k] = spot_in(grid, chosen[k], &reach, &busy, pull)?;
        busy.insert(tiles[k]);
    }
    // The muster's gates: open floor outside every room, a door's step from one, far from where he
    // arrives and from each other.
    let inside = |p: &P| rooms.iter().any(|r| r.contains(*p));
    let mut doors: Vec<P> = reach
        .iter()
        .copied()
        .filter(|p| !inside(p) && !busy.contains(p))
        .filter(|p| CARDINALS.iter().any(|&d| inside(&p.step(d))))
        .filter(|p| (p.x - arrival.x).abs() + (p.y - arrival.y).abs() >= 16)
        .collect();
    if doors.len() < 2 {
        // (A realm of open ground: any floor far enough from him will do.)
        doors = reach.iter().copied().filter(|p| !busy.contains(p) && (p.x - arrival.x).abs() + (p.y - arrival.y).abs() >= 16).collect();
    }
    doors.sort_by_key(|p| (p.y, p.x));
    if doors.is_empty() {
        return None;
    }
    let first = doors[rng.index(doors.len())];
    let second = doors
        .iter()
        .copied()
        .filter(|p| (p.x - first.x).abs() + (p.y - first.y).abs() >= 20)
        .collect::<Vec<_>>()
        .get(0..)
        .and_then(|v| if v.is_empty() { None } else { Some(v[rng.index(v.len())]) })
        .unwrap_or(first);
    let gates = [first, second];
    // The watch's round: from its fire through the two nearest other rooms and back.
    let round = match orders.iter().position(|&o| o == OrderKind::Watch) {
        Some(w) => {
            let fire = tiles[w];
            let mut stops: Vec<Rect> = rooms.iter().copied().filter(|r| !r.contains(fire) && *r != room).collect();
            stops.sort_by(|a, b| {
                let d = |r: &Rect| {
                    let (x, y) = r.center();
                    (x as f64 - fire.x as f64).hypot(y as f64 - fire.y as f64)
                };
                d(a).total_cmp(&d(b)).then(a.x0.cmp(&b.x0))
            });
            let own = super::super::pathing::reachable(grid, fire, false, false);
            let mut way = vec![fire];
            let mut at = fire;
            let mut legs = 0;
            for r in &stops {
                if legs == 2 {
                    break;
                }
                // (Only rooms on the fire's own floor.)
                let Some(p) = spot_in(grid, *r, &own, &HashSet::new(), (0.0, 0.0)) else { continue };
                let Some(leg) = walk(grid, at, p) else { continue };
                way.extend(leg.into_iter().skip(1));
                at = p;
                legs += 1;
            }
            if legs == 0 {
                // (No other room on its floor: the watch walks to the far end of it and back.)
                let far_end = own.iter().copied().max_by_key(|p| ((p.x - fire.x).pow(2) + (p.y - fire.y).pow(2), p.y, p.x));
                if let Some(leg) = far_end.and_then(|p| walk(grid, fire, p)) {
                    at = *leg.last().unwrap_or(&fire);
                    way.extend(leg.into_iter().skip(1));
                }
            }
            let home = walk(grid, at, fire)?;
            way.extend(home.into_iter().skip(1));
            way
        }
        None => Vec::new(),
    };
    // (Where the court cannot be walked to from his post, as from a gallery of Pride's spire, the
    // decree is carried to the far end of his own floor: the throne-mirror, the altar.)
    let court_way = match walk(grid, post, court) {
        Some(w) => w,
        None => {
            let own = super::super::pathing::reachable(grid, post, false, false);
            let far_end = own.iter().copied().max_by_key(|p| ((p.x - post.x).pow(2) + (p.y - post.y).pow(2), p.y, p.x))?;
            walk(grid, post, far_end)?
        }
    };
    Some(Posts { post, room, instruments: tiles, gates, round, court: court_way })
}

/// The rooms of the vaults a general may hold or set an instrument in (all but the entry).
pub fn vault_rooms(v: &super::super::vault::Vault) -> Vec<Rect> {
    v.rooms.iter().filter(|r| r.kind != super::super::vault::RoomKind::Entry).map(|r| r.rect).collect()
}

/// The rooms of one of the six realms: its dens.
pub fn realm_rooms(b: &super::super::realms::Base) -> Vec<Rect> {
    let mut out: Vec<Rect> = Vec::new();
    for p in &b.packs {
        if !out.contains(&p.den) {
            out.push(p.den);
        }
    }
    out
}

/// The seed a realm's general is placed by: its sin and its tier.
pub fn seed_of(sin: Sin, tier: usize) -> u32 {
    0x6e6e_0000 ^ (sin.index() as u32 * 977) ^ (tier as u32).wrapping_mul(0x9e37)
}

/// The seven, in the order of `SINS`.
pub fn all() -> [General; 7] {
    SINS.map(General::of)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(c: &mut Command, frames: u32, build: bool) -> Vec<(u32, Cue)> {
        let mut out = Vec::new();
        for _ in 0..frames {
            for q in c.step(build) {
                out.push((c.frame, q));
            }
        }
        out
    }

    #[test]
    fn the_seven_generals_follow_the_plans_table() {
        let g = all();
        let names: Vec<&str> = g.iter().map(|g| g.name).collect();
        assert!(names.contains(&"THE ASSESSOR") && names.contains(&"MARSHAL GOREBRAND") && names.contains(&"THE BRIDESMAID"));
        for gen in g {
            // Three orders, never two of a kind, a kindling and a decree in each realm.
            let o = gen.orders;
            assert!(o[0] != o[1] && o[1] != o[2] && o[0] != o[2], "{:?}", gen.sin);
            assert!(o.contains(&OrderKind::Kindle) && o.contains(&OrderKind::Decree), "{:?}", gen.sin);
            assert!(gen.name.is_ascii() && gen.decree.is_ascii() && gen.instruments.iter().all(|s| s.is_ascii()));
        }
        assert_eq!(General::of(Sin::Greed).body, EnemyKind::Blemmyes);
        assert_eq!(General::of(Sin::Greed).orders, [OrderKind::Muster, OrderKind::Kindle, OrderKind::Decree]);
        assert_eq!(General::of(Sin::Greed).card(OrderKind::Muster), "THE ASSESSOR MUSTERS");
    }

    #[test]
    fn orders_only_come_in_the_build() {
        let mut c = Command::new(Sin::Greed, [P::new(1, 1); 3]);
        // A long rest, a peak, a release: nothing.
        assert!(run(&mut c, 5000, false).is_empty());
        // The build: after its first stretch, an order.
        let cues = run(&mut c, FIRST_ORDER + 1, true);
        assert!(matches!(cues.first(), Some((_, Cue::Announce { order: 0, .. }))), "{cues:?}");
        // Announced, it bites and ends whatever the Drum does after.
        let after = run(&mut c, 2000, false);
        assert!(after.iter().any(|q| matches!(q.1, Cue::Bite { order: 0 })));
        assert!(after.iter().any(|q| matches!(q.1, Cue::Over { order: 0, stage: OrderStage::Ready })));
        assert!(!after.iter().any(|q| matches!(q.1, Cue::Announce { .. })), "no new order outside the build");
        // One at a time, round his three.
        let mut c = Command::new(Sin::Greed, [P::new(1, 1); 3]);
        let cues = run(&mut c, 20_000, true);
        let given: Vec<usize> =
            cues.iter().filter_map(|q| if let Cue::Announce { order, .. } = q.1 { Some(order) } else { None }).collect();
        assert!(given.len() >= 4 && given[..3] == [0, 1, 2], "{given:?}");
        let mut live: Option<usize> = None;
        for (_, q) in &cues {
            match *q {
                Cue::Announce { order, .. } => {
                    assert!(live.is_none(), "one order at a time");
                    live = Some(order);
                }
                Cue::Over { order, .. } => {
                    assert_eq!(live, Some(order));
                    live = None;
                }
                _ => {}
            }
        }
        // The decree, once done, is never given again.
        assert!(c.decree_done && c.stages[2] == OrderStage::Done);
        assert_eq!(c.given[2], 1);
    }

    #[test]
    fn every_order_is_announced_a_second_before_it_bites() {
        for sin in SINS {
            let mut c = Command::new(sin, [P::new(1, 1); 3]);
            let cues = run(&mut c, 30_000, true);
            let mut announced: Option<(usize, u32)> = None;
            let mut bites = 0;
            for (f, q) in cues {
                match q {
                    Cue::Announce { order, lead } => {
                        assert!(lead >= MIN_LEAD, "{sin:?}");
                        announced = Some((order, f));
                    }
                    Cue::Bite { order } => {
                        let (o, at) = announced.expect("a bite without its announcement");
                        assert_eq!(o, order);
                        assert!(f - at >= MIN_LEAD, "{sin:?}: order {order} bit {} frames after its horn", f - at);
                        bites += 1;
                    }
                    Cue::Answer { lead, .. } => assert!(lead >= MIN_LEAD),
                    _ => {}
                }
            }
            assert!(bites >= 3, "{sin:?}");
            // A kindled mark lies three seconds before it bursts.
            const { assert!(KINDLE_FUSE >= MIN_LEAD && WHISTLE >= MIN_LEAD) };
        }
    }

    #[test]
    fn a_broken_instrument_ends_its_order_and_the_general_answers_once() {
        let mut c = Command::new(Sin::Greed, [P::new(1, 1), P::new(2, 2), P::new(3, 3)]);
        run(&mut c, FIRST_ORDER + 1, true);
        assert_eq!(c.running, Some(0));
        assert!(c.strike(0).is_empty() && c.strike(0).is_empty(), "three blows break it");
        let cues = c.strike(0);
        assert!(cues.contains(&Cue::Over { order: 0, stage: OrderStage::Broken }));
        // The next whole one is the bellows: a guard pack goes to it.
        assert!(cues.contains(&Cue::Answer { counter: Counter::GuardTo(1), lead: COUNTER_LEAD }));
        assert_eq!(c.stages[0], OrderStage::Broken);
        let later = run(&mut c, COUNTER_LEAD, false);
        assert!(later.iter().any(|q| q.1 == Cue::Answered { counter: Counter::GuardTo(1) }));
        // Never given again; a struck broken instrument does nothing.
        assert!(c.strike(0).is_empty());
        let cues = run(&mut c, 30_000, true);
        assert!(!cues.iter().any(|q| q.1 == Cue::Announce { order: 0, lead: c.lead(0) }));
        // The muster whole and the last other one broken: its gates are doubled.
        let mut c = Command::new(Sin::Greed, [P::new(1, 1), P::new(2, 2), P::new(3, 3)]);
        for _ in 0..BLOWS {
            c.strike(2);
        }
        for _ in 0..BLOWS - 1 {
            c.strike(1);
        }
        let cues = c.strike(1);
        assert!(cues.contains(&Cue::Answer { counter: Counter::DoubleMuster, lead: COUNTER_LEAD }));
        run(&mut c, COUNTER_LEAD, false);
        assert_eq!(c.muster_extra, MUSTER);
        // The bellows may be turned (once, while whole); nothing else may.
        let mut c = Command::new(Sin::Greed, [P::new(1, 1); 3]);
        assert!(!c.turn(0) && c.turn(1) && !c.turn(1));
    }

    #[test]
    fn the_decree_and_the_generals_death_change_the_princes_fight() {
        assert_eq!(terms(Sin::Greed, false, false), PrinceTerms::WHOLE);
        let t = terms(Sin::Greed, true, false);
        assert!(t.adds_at_start && t.phase_two_adds, "the treasurers' decree done: the golems come sooner");
        let t = terms(Sin::Greed, false, true);
        assert!(!t.adds_at_start && !t.phase_two_adds, "the Assessor slain: no golems in phase two");
        for sin in SINS.into_iter().filter(|&s| !crate::logic::court::built(s)) {
            assert!(terms(sin, true, false).health > 1.0 && terms(sin, false, true).health < 1.0, "{sin:?}");
        }
        // Slain, every order ends at once and nothing more is given.
        let mut c = Command::new(Sin::Wrath, [P::new(1, 1); 3]);
        run(&mut c, FIRST_ORDER + 1, true);
        let cues = c.slay();
        assert!(cues.iter().any(|q| matches!(q, Cue::Over { stage: OrderStage::Broken, .. })));
        assert!(run(&mut c, 20_000, true).is_empty());
        assert!(c.terms().health < 1.0);
        // A failed order (the gate barred) may be given again after his rest.
        let mut c = Command::new(Sin::Greed, [P::new(1, 1); 3]);
        run(&mut c, FIRST_ORDER + 1, true);
        assert!(c.fail(0).is_some() && c.stages[0] == OrderStage::Ready && c.running.is_none());
        assert_eq!(c.wait, COOLDOWN);
    }

    /// Each realm laid out at `tier`, as a grid, with where he arrives, the court's step and the rooms.
    fn realm(sin: Sin, tier: usize) -> (Grid, P, P, Vec<Rect>, Vec<P>) {
        use crate::logic::level::parse_level;
        use crate::logic::levels::level_for;
        if sin == Sin::Greed {
            let v = crate::logic::vault::Vault::generate(crate::logic::vault::SEED, tier);
            let level = parse_level(&v.level(&level_for(1)));
            let court = P::new(v.exit_gate.x - 1, v.exit_gate.y);
            (level.grid, v.arrival, court, vault_rooms(&v), v.packs.iter().flat_map(|p| p.foes.iter().map(|f| f.2)).collect())
        } else {
            let run = crate::logic::realms::Run::generate(sin, tier).unwrap();
            let b = run.base();
            let level = parse_level(&b.level(&level_for(1)));
            (level.grid, b.arrival, b.exit.front(), realm_rooms(b), b.packs.iter().flat_map(|p| p.foes.iter().map(|f| f.2)).collect())
        }
    }

    #[test]
    fn every_instrument_can_be_reached_and_broken() {
        for sin in SINS {
            for tier in [1, 4, 7] {
                let (grid, arrival, court, rooms, foes) = realm(sin, tier);
                let gen = General::of(sin);
                let posts =
                    place(&grid, arrival, court, &rooms, &foes, gen.orders, seed_of(sin, tier)).unwrap_or_else(|| panic!("{sin:?} {tier}"));
                assert_eq!(
                    place(&grid, arrival, court, &rooms, &foes, gen.orders, seed_of(sin, tier)).as_ref(),
                    Some(&posts),
                    "the same every time"
                );
                let seen = ground(&grid, arrival, &foes);
                let mut all = vec![posts.post];
                all.extend(posts.instruments);
                all.extend(posts.gates);
                for p in &all {
                    assert!(seen.contains(p), "{sin:?} at tier {tier}: {p:?} cannot be reached");
                }
                // Each instrument stands clear, with floor all round it to strike it from.
                for (k, p) in posts.instruments.iter().enumerate() {
                    let sides = CARDINALS.iter().filter(|&&d| seen.contains(&p.step(d))).count();
                    assert!(sides >= 2, "{sin:?}: instrument {k} has no floor to strike it from");
                    assert!(!posts.room.contains(*p), "{sin:?}: an instrument in his own room");
                }
                assert!(posts.instruments[0] != posts.instruments[1] && posts.instruments[1] != posts.instruments[2]);
                // His post is past the middle of the way; the decree walks from it to the court.
                assert_eq!(posts.court.first(), Some(&posts.post));
                if crate::logic::pathing::reachable(&grid, posts.post, false, false).contains(&court) {
                    assert_eq!(posts.court.last(), Some(&court));
                }
                assert!(posts.court.windows(2).all(|w| (w[0].x - w[1].x).abs() + (w[0].y - w[1].y).abs() == 1));
                if gen.orders.contains(&OrderKind::Watch) {
                    let w = gen.index_of(OrderKind::Watch).unwrap();
                    assert!(posts.round.len() > 10, "{sin:?}: the watch's round");
                    assert_eq!((posts.round.first(), posts.round.last()), (Some(&posts.instruments[w]), Some(&posts.instruments[w])));
                    assert!(posts.round.iter().all(|p| seen.contains(p)));
                } else {
                    assert!(posts.round.is_empty());
                }
                // Three blows break each.
                let mut c = Command::new(sin, posts.instruments);
                for k in 0..3 {
                    for _ in 0..BLOWS {
                        c.strike(k);
                    }
                }
                assert_eq!(c.broken(), 3);
            }
        }
    }

    #[test]
    fn the_generals_record_survives_the_save() {
        let mut c = Command::new(Sin::Envy, [P::new(1, 1); 3]);
        for _ in 0..BLOWS {
            c.strike(1);
        }
        c.decree_done = true;
        let mut all = Records::default();
        all.set(Sin::Envy, c.record());
        all.set(Sin::Lust, Record { met: true, slain: true, ..Record::default() });
        let text = all.encode();
        assert!(!text.contains('|') && !text.contains(':'), "{text}");
        assert_eq!(Records::decode(&text), all);
        let mut back = Command::new(Sin::Envy, [P::new(1, 1); 3]);
        back.restore(Records::decode(&text).get(Sin::Envy));
        assert!(back.instruments[1].broken && back.stages[1] == OrderStage::Broken && back.decree_done);
        assert_eq!(back.stages[2], OrderStage::Done);
        assert_eq!(Records::decode("x,9.1.1.1,2.1"), Records::default(), "rubbish is left out");
    }
}
