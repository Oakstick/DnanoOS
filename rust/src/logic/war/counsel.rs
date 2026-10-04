//! The old man's counsel (WARBAND.md §10.1): rules of the form *trigger → line*, each with a
//! priority, a cooldown and, for some, a once-flag kept for the run.
//!
//! Every line that says something about a foe is built from the game's own data: a breed's answer
//! is read from its archetype, its folklore power and its moves (`Answer::of`), a rival's trait from
//! the rival himself (`ledger::counsel_line_for`), a general's name and instrument from what the
//! generals' layer knows (`GeneralIntel`). So the old man is true by construction, and
//! `the_old_man_never_lies` checks it.
//!
//! Each hero hears his own register (the old man calls Ragnar, Mehmed and Miyamoto differently),
//! but the facts in a line are the same for all three.

use std::collections::{BTreeSet, HashMap};

use super::deceit::CheatKind;
use super::ledger::{counsel_line_for, Rival};
use super::names;
use crate::logic::combat::archetype::{Archetype, Power};
use crate::logic::combat::frames::{Shape, UNSTOPPABLE};
use crate::logic::combat::{Breed, MoveId};
use crate::logic::enemy::EnemyKind;
use crate::logic::heroes::HeroKind;
use crate::logic::sins::Sin;

/// At most one line in this many seconds (§10.1).
pub const GAP: f64 = 45.0;
/// A line not yet said is dropped after this long: what he would have said no longer fits.
pub const STALE: f64 = 90.0;

// ------------------------------------------------------------------ a breed's answer

/// The way to beat a breed, as the old man says it. Each is a fact about the breed that the game's
/// data can confirm or deny (`holds`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Answer {
    /// A brute: its blow is slow; wait it out, strike its long recovery.
    WaitAndPunish,
    /// A body foe: it bites from beside him; parry the bite or step out of reach.
    ParryTheBite,
    /// A distance foe: close in between its bolts.
    CloseBetweenBolts,
    /// A blow no shield holds: parry it, or leap.
    ParryOrLeap,
    /// A blow from a mark on the floor: walk out of it, or leap.
    WalkOut,
    /// Tepegöz's ring: only his eye can be hurt.
    StrikeTheEye,
    /// The goryō: nothing touches it but while it rages.
    StrikeWhileItRages,
    /// The ejder's plea: a second blow brings it back.
    StayYourHand,
    /// The elf-maid: hollow behind, and only there to be struck.
    StrikeFromBehind,
    /// The kappa: parried, it bows and spills its dish.
    ParryTheShove,
    /// The kitsune: one blow pops an illusion.
    PopTheIllusions,
    /// The cin: it vanishes when he comes near, but not twice in quick turns.
    CloseAgain,
    /// Hel: the dead she raises fall when she does.
    StrikeTheQueen,
    /// Surtr: his blows leave the floor burning.
    MindTheFire,
    /// The frost giant: the frost bites through the shield.
    FrostThroughShield,
    /// The tülükuş: its cry dooms his next wound.
    AfterTheCry,
    /// The albastı: a shadow until she rears up.
    WatchTheRearing,
    /// The arçura: it grows tall to cast.
    CloseWhenItGrows,
    /// The oni's club: no blow stops it, only a parry.
    ParryTheClub,
    /// The karakoncolos: it roars his name, and dooms him.
    StepOutOfTheCall,
    /// Mammon's hide.
    HideTurns,
    /// Amon: his charge runs straight; run into stone, he is dazed.
    BaitIntoStone,
}

impl Answer {
    pub const ALL: [Answer; 22] = [
        Answer::WaitAndPunish,
        Answer::ParryTheBite,
        Answer::CloseBetweenBolts,
        Answer::ParryOrLeap,
        Answer::WalkOut,
        Answer::StrikeTheEye,
        Answer::StrikeWhileItRages,
        Answer::StayYourHand,
        Answer::StrikeFromBehind,
        Answer::ParryTheShove,
        Answer::PopTheIllusions,
        Answer::CloseAgain,
        Answer::StrikeTheQueen,
        Answer::MindTheFire,
        Answer::FrostThroughShield,
        Answer::AfterTheCry,
        Answer::WatchTheRearing,
        Answer::CloseWhenItGrows,
        Answer::ParryTheClub,
        Answer::StepOutOfTheCall,
        Answer::HideTurns,
        Answer::BaitIntoStone,
    ];

    /// The answers a liar can put in any breed's mouth (the prince's whisper picks a false one).
    pub const PLAUSIBLE: [Answer; 7] = [
        Answer::WaitAndPunish,
        Answer::ParryTheBite,
        Answer::CloseBetweenBolts,
        Answer::ParryOrLeap,
        Answer::ParryTheClub,
        Answer::StayYourHand,
        Answer::StrikeFromBehind,
    ];

    /// The answer to `kind`, read from its data: its folklore power first, then its own moves, then
    /// its archetype.
    pub fn of(kind: EnemyKind) -> Answer {
        let b = Breed::of(kind);
        let by_power = match b.power {
            Power::Ward => Some(Answer::StrikeTheEye),
            Power::Spirit => Some(Answer::StrikeWhileItRages),
            Power::Plea => Some(Answer::StayYourHand),
            Power::Hollow => Some(Answer::StrikeFromBehind),
            Power::Dish => Some(Answer::ParryTheShove),
            Power::Illusions => Some(Answer::PopTheIllusions),
            Power::Vanish => Some(Answer::CloseAgain),
            Power::Necromancy => Some(Answer::StrikeTheQueen),
            Power::Fire => Some(Answer::MindTheFire),
            Power::Rime => Some(Answer::FrostThroughShield),
            Power::Omen => Some(Answer::AfterTheCry),
            Power::Shade => Some(Answer::WatchTheRearing),
            Power::Grow => Some(Answer::CloseWhenItGrows),
            // A prince's hide (for now every prince's power is one).
            Power::GoldHide | Power::Vainglory | Power::Scales | Power::Torpor | Power::Bloat | Power::Allure => Some(Answer::HideTurns),
            // Amon has no hide: his charge is the thing to know.
            Power::Fury => Some(Answer::BaitIntoStone),
            Power::Rattle | Power::None => None,
        };
        if let Some(a) = by_power {
            return a;
        }
        let moves = b.moves();
        if moves.iter().any(|m| m.data().unblockable) {
            Answer::ParryOrLeap
        } else if moves.iter().any(|m| matches!(m.data().shape, Shape::Mark { .. })) {
            Answer::WalkOut
        } else if moves.contains(&MoveId::Call) {
            Answer::StepOutOfTheCall
        } else if moves.iter().any(|m| m.data().armor >= UNSTOPPABLE) {
            Answer::ParryTheClub
        } else {
            match b.arch {
                Archetype::Brute => Answer::WaitAndPunish,
                Archetype::Skirmisher => Answer::ParryTheBite,
                Archetype::Hexer => Answer::CloseBetweenBolts,
                Archetype::Prince => Answer::HideTurns,
            }
        }
    }

    /// Is it true of `kind`? Checked against the frame data and the breed table, not against `of`.
    pub fn holds(self, kind: EnemyKind) -> bool {
        let b = Breed::of(kind);
        let moves = b.moves();
        let any = |f: &dyn Fn(MoveId) -> bool| moves.iter().any(|&m| f(m));
        match self {
            Answer::WaitAndPunish => b.arch == Archetype::Brute && b.main_move().data().recovery >= 30,
            Answer::ParryTheBite => {
                b.arch == Archetype::Skirmisher && any(&|m| m.data().strikes() && !m.data().unblockable && !m.data().projectile)
            }
            Answer::CloseBetweenBolts => b.arch == Archetype::Hexer && any(&|m| m.data().projectile),
            Answer::ParryOrLeap => any(&|m| m.data().unblockable),
            Answer::WalkOut => any(&|m| matches!(m.data().shape, Shape::Mark { .. })),
            Answer::StrikeTheEye => b.power == Power::Ward,
            Answer::StrikeWhileItRages => b.power == Power::Spirit,
            Answer::StayYourHand => b.power == Power::Plea,
            Answer::StrikeFromBehind => b.power == Power::Hollow,
            Answer::ParryTheShove => b.power == Power::Dish,
            Answer::PopTheIllusions => b.power == Power::Illusions,
            Answer::CloseAgain => b.power == Power::Vanish,
            Answer::StrikeTheQueen => b.power == Power::Necromancy,
            Answer::MindTheFire => b.power == Power::Fire,
            Answer::FrostThroughShield => b.power == Power::Rime,
            Answer::AfterTheCry => b.power == Power::Omen,
            Answer::WatchTheRearing => b.power == Power::Shade,
            Answer::CloseWhenItGrows => b.power == Power::Grow,
            Answer::ParryTheClub => any(&|m| m.data().armor >= UNSTOPPABLE),
            Answer::StepOutOfTheCall => moves.contains(&MoveId::Call),
            Answer::HideTurns => b.power.is_hide(),
            Answer::BaitIntoStone => b.power == Power::Fury && any(&|m| matches!(m.data().shape, Shape::Charge { .. })),
        }
    }

    /// The words that name it in a line (lower case; no answer's are inside another's line).
    pub fn keyword(self) -> &'static str {
        match self {
            Answer::WaitAndPunish => "swings slow",
            Answer::ParryTheBite => "parry the bite",
            Answer::CloseBetweenBolts => "close in between",
            Answer::ParryOrLeap => "no shield holds",
            Answer::WalkOut => "walk out of",
            Answer::StrikeTheEye => "only the eye",
            Answer::StrikeWhileItRages => "while it rages",
            Answer::StayYourHand => "stay your hand",
            Answer::StrikeFromBehind => "hollow behind",
            Answer::ParryTheShove => "spills the dish",
            Answer::PopTheIllusions => "one blow pops",
            Answer::CloseAgain => "vanishes in smoke",
            Answer::StrikeTheQueen => "the dead fall with",
            Answer::MindTheFire => "floor burning",
            Answer::FrostThroughShield => "frost bites through",
            Answer::AfterTheCry => "the cry of the",
            Answer::WatchTheRearing => "rears up",
            Answer::CloseWhenItGrows => "grows tall",
            Answer::ParryTheClub => "only a parry stops",
            Answer::StepOutOfTheCall => "roars your name",
            Answer::HideTurns => "turns your blade",
            Answer::BaitIntoStone => "into stone",
        }
    }

    /// The fact, said of `kind` (one or two sentences; the keyword in it).
    pub fn body(self, kind: EnemyKind) -> String {
        let b = names::breed_name(kind);
        let she = names::feminine(kind);
        let (it, its) = if she { ("she", "her") } else { ("it", "its") };
        match self {
            Answer::WaitAndPunish => format!("The {b} swings slow. Wait for the blow, then strike while {it} recovers."),
            Answer::ParryTheBite => format!("The {b} bites from beside you. Parry the bite, or step out of {its} reach."),
            Answer::CloseBetweenBolts => format!("The {b} strikes from afar. Close in between {its} bolts, and never stand still."),
            Answer::ParryOrLeap => format!("No shield holds the blows of the {b}. Parry them, or leap."),
            Answer::WalkOut => format!("The {b} strikes from under the floor. Watch the mark, and walk out of it, or leap."),
            Answer::StrikeTheEye => format!("No blade cuts the {b}; only the eye. Strike when {it} is spent from a blow, or reeling."),
            Answer::StrikeWhileItRages => format!("Nothing touches the {b} while it waits. Strike while it rages."),
            Answer::StayYourHand => format!("Fell the {b} and {it} begs for a second blow. Stay your hand; restraint kills {it}."),
            Answer::StrikeFromBehind => format!("No blow tells on the {b} from the front. Go round: {it} is hollow behind."),
            Answer::ParryTheShove => {
                format!("Parry the {b}'s shove: {it} bows and spills the dish on {its} head, and is weak a while. Strike then.")
            }
            Answer::PopTheIllusions => format!("When the {b} splits in three, one blow pops a false one. Find the true one."),
            Answer::CloseAgain => {
                format!("Come near the {b} and {it} vanishes in smoke, but not twice in quick turns. Close in again at once.")
            }
            Answer::StrikeTheQueen => format!("Strike the {b}, not what {it} raises: the dead fall with their mistress."),
            Answer::MindTheFire => format!("The {b}'s blows leave the floor burning. Never fight in the fire; draw {it} out of it."),
            Answer::FrostThroughShield => format!("The {b}'s frost bites through the shield. Do not block {it}; parry, or step away."),
            Answer::AfterTheCry => {
                format!("Mind the cry of the {b}: it dooms your next wound. Let {it} drop from the air, then parry.")
            }
            Answer::WatchTheRearing => format!("The {b} is a shadow with red eyes until {it} rears up. Then parry the fall."),
            Answer::CloseWhenItGrows => format!("The {b} grows tall to cast and shrinks to slip away. Close in when {it} grows."),
            Answer::ParryTheClub => format!("No blow stops the {b}'s club; only a parry stops it. Keep back from the ground it shakes."),
            Answer::StepOutOfTheCall => {
                format!("When the {b} roars your name, your next wound cuts deeper. Get away as the roar begins.")
            }
            Answer::HideTurns => format!("The {b}'s hide turns your blade. Strike in {its} recoveries, or while {it} reels."),
            Answer::BaitIntoStone => {
                format!("The {b} charges straight. Stand with stone at your back and step aside at the last: run into stone, {it} stands dazed.")
            }
        }
    }
}

// ------------------------------------------------------------------ the generals (W6's hook)

/// What the old man knows of a realm's general, when the generals' layer (`logic::war::general`,
/// W6) knows it: his name, one of his instruments, and where it stands. Every field is read from
/// the general's own tables, so a line built from it is true.
#[derive(Clone, Debug, PartialEq)]
pub struct GeneralIntel {
    /// "The Assessor".
    pub name: String,
    /// "counting-bell".
    pub instrument: String,
    /// "in the hall east of the treasury".
    pub place: String,
}

// ------------------------------------------------------------------ the registers

/// What a line is for: its frame differs by hero.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Frame {
    Death,
    Room,
    Rival,
    Realm,
    Gear,
    Praise,
}

/// How the old man addresses him.
pub fn address(hero: HeroKind) -> &'static str {
    match hero {
        HeroKind::Ragnar => "Ragnar",
        HeroKind::Mehmed => "janissary",
        HeroKind::Miyamoto => "swordsman",
    }
}

/// `body` in the hero's register: Ragnar is told plainly, by name; Mehmed formally, as a soldier;
/// Miyamoto briefly, as a student of the sword. The body (the facts) is never changed.
pub fn framed(hero: HeroKind, frame: Frame, body: &str) -> String {
    let a = address(hero);
    match (hero, frame) {
        (HeroKind::Ragnar, Frame::Death) => format!("Up, Ragnar. {body}"),
        (HeroKind::Mehmed, Frame::Death) => format!("Rise, janissary, and mark this. {body}"),
        (HeroKind::Miyamoto, Frame::Death) => format!("Breathe, swordsman. {body}"),
        (HeroKind::Ragnar, Frame::Room) => {
            format!("Three times in that room, Ragnar. {body} Come back to me. I'll mend the shield, and we'll talk.")
        }
        (HeroKind::Mehmed, Frame::Room) => {
            format!("Three times that room has thrown you back, my lion. {body} Come back to me. I'll mend the shield, and we'll talk.")
        }
        (HeroKind::Miyamoto, Frame::Room) => {
            format!("Three falls in one room, swordsman. {body} Come back to me. I'll mend the shield, and we'll talk.")
        }
        (HeroKind::Ragnar, Frame::Rival) => format!("{body} Remember it, Ragnar."),
        (HeroKind::Mehmed, Frame::Rival) => format!("Your enemy has a name, janissary. {body}"),
        (HeroKind::Miyamoto, Frame::Rival) => format!("Know your opponent, swordsman. {body}"),
        (_, Frame::Realm) => format!("{body} Go carefully, {a}."),
        (HeroKind::Ragnar, Frame::Gear) => body.to_string(),
        (_, Frame::Gear) => format!("{body} Do not wait, {a}."),
        (HeroKind::Ragnar, Frame::Praise) => format!("Ha! {body} Quitmer smelled it too."),
        (HeroKind::Mehmed, Frame::Praise) => format!("Well seen, janissary. {body} Quitmer knew it too."),
        (HeroKind::Miyamoto, Frame::Praise) => format!("Your eye is clear, swordsman. {body} Quitmer knew it too."),
    }
}

/// His counsel on a breed, for this hero.
pub fn breed_line(kind: EnemyKind, hero: HeroKind) -> String {
    framed(hero, Frame::Death, &Answer::of(kind).body(kind))
}

/// What he says of a rival met, if the hero has paid for (or found) one of his traits; nothing if
/// not (he only names what is known).
pub fn rival_line(r: &Rival, hero: HeroKind) -> Option<String> {
    (r.known != 0).then(|| framed(hero, Frame::Rival, &counsel_line_for(r)))
}

/// What he says as the hero enters a realm: the general's name and an instrument when the generals'
/// layer knows them, otherwise only what is sure (the prince keeps a general there).
pub fn realm_line(sin: Sin, hero: HeroKind, intel: Option<&GeneralIntel>, first: bool) -> String {
    let realm = sin.realm();
    let prince = sin.prince();
    let mut body = match intel {
        Some(g) => format!("{realm}. {} holds it for {prince}. His {} stands {}. Break it.", g.name, g.instrument, g.place),
        None => format!("{realm}. {prince} keeps a general there; I have not learned his name. Listen for horns."),
    };
    if first {
        body.push_str(" And whatever the prince offers you, it costs more than he says.");
    }
    framed(hero, Frame::Realm, &body)
}

/// The gear's warnings, in his voice instead of a floater.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum GearWarning {
    BladeDulling,
    BladeRusted,
    ShieldCracking,
    ShieldBroken,
}

pub fn gear_line(w: GearWarning, hero: HeroKind) -> String {
    let body = match w {
        GearWarning::BladeDulling => "Your edge is going. Bring it to my whetstone before it rusts.",
        GearWarning::BladeRusted => "That blade is rust now; it glances off them. Call me, and I'll sharpen it.",
        GearWarning::ShieldCracking => "Your shield is cracking. Mend it before it breaks.",
        GearWarning::ShieldBroken => "Your shield is gone. I can forge it anew; call me.",
    };
    framed(hero, Frame::Gear, body)
}

/// His praise when a cheat of the prince's is seen through.
pub fn praise_line(cheat: CheatKind, hero: HeroKind) -> String {
    let body = match cheat {
        CheatKind::Whisper => "That was not my voice down there. I said no such thing.",
        CheatKind::CrookedBargain => "You refused his bargain. It cost more than he said; it always does.",
        CheatKind::LoadedDice => "A chest that breathes is no chest.",
        CheatKind::TurnedSign => "You followed my lamp, not his sign.",
    };
    framed(hero, Frame::Praise, body)
}

/// What he says when asked about a whisper: the denial, and the truth in its place.
pub fn denial(truth: &str) -> String {
    format!("I said no such thing. {truth}")
}

/// What he remarks at the workshop on the lies seen through this run.
pub fn lies_remark(lies: u32, hero: HeroKind) -> Option<String> {
    let a = address(hero);
    match lies {
        0 => None,
        1 => Some(format!("You caught the prince in a lie, {a}. He has more. Keep your ears open.")),
        n => Some(format!("{n} lies seen through, {a}. The prince is running short of tricks; he has not run out.")),
    }
}

// ------------------------------------------------------------------ the rules

/// What can set him talking.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Rule {
    /// First time a breed kills him: its counter (once per breed, for the run).
    FirstDeath,
    /// Three deaths in the same room: a tip, and "come back to me" (once per room).
    RoomDeaths,
    /// A rival met, whose trait is known (once per rival).
    RivalMet,
    /// Into a realm: its general, or what is sure.
    RealmEntered,
    /// The blade dulls or rusts, the shield cracks or breaks.
    Gear,
    /// A prince's cheat seen through: praise (and Quitmer barks).
    SeenThrough,
}

impl Rule {
    /// Higher speaks first.
    pub fn priority(self) -> u8 {
        match self {
            Rule::SeenThrough => 6,
            Rule::RoomDeaths => 5,
            Rule::FirstDeath => 4,
            Rule::RivalMet => 3,
            Rule::Gear => 2,
            Rule::RealmEntered => 1,
        }
    }

    /// Seconds before the same rule may speak again (on top of the gap).
    pub fn cooldown(self) -> f64 {
        match self {
            Rule::Gear => 120.0,
            Rule::RealmEntered => 60.0,
            _ => 0.0,
        }
    }
}

/// What happened, as the counsel hears it.
#[derive(Clone, Copy, Debug)]
pub enum Trigger<'a> {
    /// He fell: to what (if a foe), and in which room (`room` is the caller's key for it).
    Died {
        by: Option<EnemyKind>,
        room: u64,
    },
    RivalMet(&'a Rival),
    RealmEntered {
        sin: Sin,
        intel: Option<&'a GeneralIntel>,
    },
    Gear(GearWarning),
    SeenThrough(CheatKind),
}

/// What a line is about: what `the_old_man_never_lies` checks it against.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum About {
    Breed(EnemyKind, Answer),
    /// A room he keeps falling in, with nothing known of what fells him there.
    Room,
    Rival,
    Realm(Sin),
    Gear(GearWarning),
    Praise(CheatKind),
}

/// A line he will say.
#[derive(Clone, PartialEq, Debug)]
pub struct Line {
    pub rule: Rule,
    pub text: String,
    pub about: About,
    /// When it was heard (the game's time).
    pub at: f64,
}

/// The old man's memory: what he has said for good (the once-flags), the deaths he has seen,
/// what he means to say, and when he last spoke.
#[derive(Clone, Debug, Default)]
pub struct Counsel {
    /// Once-flags kept for the run: "d<kind>" a breed's counter given, "m<rival>" a rival named,
    /// "bargain" the prince's rule told. (Rooms are this visit's only, and not kept.)
    pub flags: BTreeSet<String>,
    rooms: HashMap<u64, u32>,
    pub pending: Vec<Line>,
    /// When he last spoke, and when each rule last did.
    pub last: Option<f64>,
    said: HashMap<Rule, f64>,
}

impl Counsel {
    /// The flags as the save keeps them ("d3,m12,bargain"; no '|').
    pub fn encode(&self) -> String {
        self.flags.iter().filter(|f| !f.starts_with('r')).cloned().collect::<Vec<_>>().join(",")
    }

    pub fn decode(text: &str) -> Counsel {
        let flags = text.split(',').map(str::trim).filter(|s| !s.is_empty() && s.is_ascii()).map(str::to_string).collect();
        Counsel { flags, ..Counsel::default() }
    }

    /// A new place: the rooms counted begin again (the flags stay).
    pub fn new_visit(&mut self) {
        self.rooms.clear();
        self.flags.retain(|f| !f.starts_with('r'));
    }

    /// Hears what happened at `now`; whatever he means to say about it waits in `pending`.
    pub fn hear(&mut self, t: Trigger, hero: HeroKind, now: f64) {
        let push = |s: &mut Counsel, rule: Rule, text: String, about: About, flag: Option<String>| {
            if let Some(f) = &flag {
                if s.flags.contains(f) {
                    return;
                }
            }
            if s.said.get(&rule).is_some_and(|&at| now - at < rule.cooldown()) {
                return;
            }
            if s.pending.iter().any(|l| l.text == text) {
                return;
            }
            if let Some(f) = flag {
                s.flags.insert(f);
            }
            s.pending.push(Line { rule, text, about, at: now });
        };
        match t {
            Trigger::Died { by, room } => {
                let n = self.rooms.entry(room).or_insert(0);
                *n += 1;
                let third = *n == 3;
                if let Some(kind) = by {
                    let answer = Answer::of(kind);
                    if third {
                        let text = framed(hero, Frame::Room, &answer.body(kind));
                        push(self, Rule::RoomDeaths, text, About::Breed(kind, answer), Some(format!("r{room}")));
                    } else {
                        let text = framed(hero, Frame::Death, &answer.body(kind));
                        push(self, Rule::FirstDeath, text, About::Breed(kind, answer), Some(format!("d{}", kind_index(kind))));
                    }
                } else if third {
                    let text = framed(hero, Frame::Room, "Whatever is in there, it is not going anywhere.");
                    push(self, Rule::RoomDeaths, text, About::Room, Some(format!("r{room}")));
                }
            }
            Trigger::RivalMet(r) => {
                if let Some(text) = rival_line(r, hero) {
                    push(self, Rule::RivalMet, text, About::Rival, Some(format!("m{}", r.id.0)));
                }
            }
            Trigger::RealmEntered { sin, intel } => {
                let first = !self.flags.contains("bargain");
                let text = realm_line(sin, hero, intel, first);
                if first {
                    self.flags.insert("bargain".into());
                }
                push(self, Rule::RealmEntered, text, About::Realm(sin), None);
            }
            Trigger::Gear(w) => push(self, Rule::Gear, gear_line(w, hero), About::Gear(w), None),
            Trigger::SeenThrough(c) => push(self, Rule::SeenThrough, praise_line(c, hero), About::Praise(c), None),
        }
    }

    /// The line to say now, if any: the highest priority (the oldest of equals), and only when he
    /// may speak (`open`) and has kept quiet `GAP` seconds. Stale lines are dropped.
    pub fn next(&mut self, now: f64, open: bool) -> Option<Line> {
        self.pending.retain(|l| now - l.at < STALE);
        if !open || self.last.is_some_and(|t| now - t < GAP) {
            return None;
        }
        let best = (0..self.pending.len()).max_by(|&a, &b| {
            let (la, lb) = (&self.pending[a], &self.pending[b]);
            la.rule.priority().cmp(&lb.rule.priority()).then(lb.at.total_cmp(&la.at))
        })?;
        let line = self.pending.remove(best);
        self.last = Some(now);
        self.said.insert(line.rule, now);
        Some(line)
    }
}

/// A kind's number in the save (the Ledger's order).
pub fn kind_index(kind: EnemyKind) -> usize {
    super::ledger::KINDS.iter().position(|&k| k == kind).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::heroes::ROSTER;
    use crate::logic::war::ledger::{Ledger, Trait, KINDS};
    use crate::logic::war::names::Deed;

    #[test]
    fn every_breed_has_a_true_answer_read_from_its_data() {
        for &kind in KINDS {
            let a = Answer::of(kind);
            assert!(a.holds(kind), "{kind:?}: {a:?} does not hold");
        }
        // The answers the tables call for: the brute waits, the biter is parried, the caster closed
        // on, and the legends each by their own rule.
        use EnemyKind as K;
        for (kind, want) in [
            (K::Blemmyes, Answer::WaitAndPunish),
            (K::Werewolf, Answer::ParryTheBite),
            (K::Witch, Answer::CloseBetweenBolts),
            (K::Oni, Answer::ParryTheClub),
            (K::Gashadokuro, Answer::ParryOrLeap),
            (K::Nidhogg, Answer::WalkOut),
            (K::Karakoncolos, Answer::StepOutOfTheCall),
            (K::Tepegoz, Answer::StrikeTheEye),
            (K::Ejder, Answer::StayYourHand),
            (K::Kappa, Answer::ParryTheShove),
            (K::Mammon, Answer::HideTurns),
        ] {
            assert_eq!(Answer::of(kind), want, "{kind:?}");
        }
    }

    #[test]
    fn the_old_man_never_lies() {
        // No answer's keyword turns up in another answer's line, for any breed.
        for &kind in KINDS {
            for a in Answer::ALL {
                let line = a.body(kind).to_lowercase();
                assert!(line.contains(a.keyword()), "{a:?} on {kind:?}: {line}");
                for b in Answer::ALL.iter().filter(|&&b| b != a) {
                    assert!(!line.contains(b.keyword()), "{a:?} on {kind:?} names {b:?}: {line}");
                }
            }
        }
        // Every breed's counsel, in every hero's register, names one answer, and it is true.
        for &kind in KINDS {
            for hero in ROSTER {
                let line = breed_line(kind, hero).to_lowercase();
                let named: Vec<Answer> = Answer::ALL.iter().copied().filter(|a| line.contains(a.keyword())).collect();
                assert_eq!(named.len(), 1, "{kind:?}/{hero:?}: {line}");
                assert!(named[0].holds(kind), "the old man lied about the {kind:?}: {line}");
                assert!(names::drawable(&line), "{line}");
            }
        }
        // Every rival he names, he names truly, and only once something about him is known.
        let all = Trait::all();
        let mut l = Ledger::default();
        for seed in 0..120u32 {
            let kind = KINDS[seed as usize % KINDS.len()];
            let hero = ROSTER[seed as usize % 3];
            let id = l.promote(kind, Deed::ALL[seed as usize % 5], hero, seed.wrapping_mul(2_654_435_761));
            l.get_mut(id).unwrap().known = (seed % 8) as u8;
            let r = l.get(id).unwrap();
            match rival_line(r, hero) {
                None => assert_eq!(r.known, 0, "he keeps quiet only when nothing is known"),
                Some(line) => {
                    let named: Vec<&Trait> = all.iter().filter(|t| line.contains(&t.keyword())).collect();
                    assert_eq!(named.len(), 1, "{line}");
                    assert!(r.true_traits().contains(named[0]) && r.knows(*named[0]), "lied about {}: {line}", r.name);
                }
            }
        }
        // A general's name and instrument are said as the intel has them, and never made up.
        let intel = GeneralIntel { name: "The Assessor".into(), instrument: "counting-bell".into(), place: "in the east hall".into() };
        let line = realm_line(Sin::Greed, HeroKind::Mehmed, Some(&intel), false);
        assert!(line.contains("The Assessor") && line.contains("counting-bell") && line.contains("in the east hall"));
        let blind = realm_line(Sin::Greed, HeroKind::Mehmed, None, false);
        assert!(!blind.contains("Assessor") && blind.contains("Mammon"), "{blind}");
    }

    #[test]
    fn each_hero_hears_his_own_register_of_the_same_facts() {
        let lines: Vec<String> = ROSTER.iter().map(|&h| breed_line(EnemyKind::Oni, h)).collect();
        let body = Answer::ParryTheClub.body(EnemyKind::Oni);
        for l in &lines {
            assert!(l.contains(&body), "the facts are the same: {l}");
        }
        assert!(lines[0] != lines[1] && lines[1] != lines[2] && lines[0] != lines[2], "{lines:?}");
        assert!(breed_line(EnemyKind::Oni, HeroKind::Ragnar).contains("Ragnar"));
        assert!(breed_line(EnemyKind::Oni, HeroKind::Mehmed).contains("janissary"));
        assert!(breed_line(EnemyKind::Oni, HeroKind::Miyamoto).contains("swordsman"));
    }

    #[test]
    fn counsel_keeps_its_gap_its_priorities_and_its_once_flags() {
        let mut c = Counsel::default();
        let hero = HeroKind::Ragnar;
        c.hear(Trigger::Gear(GearWarning::BladeDulling), hero, 0.0);
        c.hear(Trigger::Died { by: Some(EnemyKind::Oni), room: 7 }, hero, 1.0);
        // Closed (a peak): nothing.
        assert!(c.next(2.0, false).is_none());
        // Open: the death's counsel first (higher priority), then quiet for the gap.
        let first = c.next(3.0, true).unwrap();
        assert_eq!(first.about, About::Breed(EnemyKind::Oni, Answer::ParryTheClub));
        assert!(c.next(3.0 + GAP - 1.0, true).is_none(), "one line in 45 s at most");
        let second = c.next(3.0 + GAP, true).unwrap();
        assert_eq!(second.rule, Rule::Gear);
        // Once per breed for the run; the third death in a room gets its own line.
        c.hear(Trigger::Died { by: Some(EnemyKind::Oni), room: 7 }, hero, 60.0);
        assert!(c.pending.is_empty(), "the oni's counter is said once");
        c.hear(Trigger::Died { by: Some(EnemyKind::Oni), room: 7 }, hero, 61.0);
        assert_eq!(c.pending.len(), 1);
        assert!(c.pending[0].text.contains("Come back to me"), "{}", c.pending[0].text);
        // Stale lines go unsaid.
        let mut d = Counsel::default();
        d.hear(Trigger::Gear(GearWarning::ShieldBroken), hero, 0.0);
        assert!(d.next(STALE + 1.0, true).is_none());
        // The flags survive the save; the rooms do not.
        let back = Counsel::decode(&c.encode());
        assert!(back.flags.contains(&format!("d{}", kind_index(EnemyKind::Oni))));
        assert!(back.flags.iter().all(|f| !f.starts_with('r')));
        assert!(!c.encode().contains('|'));
    }
}
