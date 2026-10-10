//! The Ledger of Grudges (WARBAND.md §8): foes that hurt the hero, or get away from him, become
//! somebody. Named, ranked and remembered, they remember him too, and from rank 3 each carries a
//! lesson drawn from the hero's most used answer to him.
//!
//! Everything here is numbers and choices, free of the fight's frames: the brawl feeds an
//! `EncounterTracker` from the war log, the Ledger promotes, ranks up, records and offers, and a
//! rival's lesson only ever picks *which* move, *when*, and how hard an elixir bites (rule I6).
//! It never touches a move's frame data.

use super::names::{self, Deed};
use super::WarEvent;
use crate::logic::combat::frames::Shape;
use crate::logic::combat::fsm::Defense;
use crate::logic::combat::{Duelist, MoveId};
use crate::logic::enemy::EnemyKind;
use crate::logic::heroes::{HeroKind, ROSTER};
use crate::logic::progression;
use crate::logic::rng::Rng;
use crate::logic::sins::{Sin, SINS};
use crate::logic::types::PotionKind;

/// At most this many rivals live at once; a tenth replaces the lowest rank.
pub const MAX_LIVE: usize = 9;
/// The highest rank.
pub const MAX_RANK: u8 = 5;
/// Encounters a rival remembers.
pub const MEMORY: usize = 8;
/// Slain rivals kept (for the trophy wall and the avengers), oldest forgotten first.
pub const MAX_FALLEN: usize = 16;
/// A lesson from this rank.
pub const LESSON_FROM: u8 = 3;
/// A lesson needs the hero to have answered him this often (an elixir counts `ELIXIR_WEIGHT`
/// for each encounter it was drunk in).
pub const LESSON_MIN: u32 = 3;
pub const ELIXIR_WEIGHT: u32 = 4;
/// His health grows this share per rank.
pub const HEALTH_PER_RANK: f64 = 0.08;
/// Unflinching: armor added (a brute's finisher must still flinch him below 45: the integrator's test).
pub const UNFLINCHING_ARMOR: f64 = 15.0;
/// Patient: frames his tracking runs longer, never closer to the blow than `COMMIT_FOR`.
pub const PATIENT_TRACK: u16 = 4;
pub const COMMIT_FOR: u16 = 12;
/// Elixir-proof, and the antidote: that elixir's effect on him is scaled by this.
pub const ELIXIR_PROOF: f64 = 0.5;
/// Pack-lord: the pack he leads.
pub const PACK_LORD: usize = 5;
/// Proud: frames he stands open, taunting, after his blow lands.
pub const PROUD_OPEN: u16 = 60;
/// Scarred: blows from the side of his old wound land this much harder.
pub const SCAR_DAMAGE: f64 = 1.3;
/// Greedy: frames he stops for dropped silver.
pub const GREEDY_OPEN: u16 = 50;
/// Old wound: his poise is scaled by this.
pub const OLD_WOUND_POISE: f64 = 0.7;
/// Coward: he routs below this share of his health.
pub const COWARD_ROUT: f64 = 0.4;
/// Schemer: the horn's tell, frames.
pub const HORN_TELL: u16 = 60;
/// The lesson of the parry: the second startup is this much longer than the first (longer than the
/// parry window, so a parry timed by rhythm misses one of the two).
pub const PARRY_SPREAD: u16 = 10;
/// The lesson of the leap: he waits out the leap's landing.
pub const LEAP_LANDING: u16 = 6;
/// The lesson of the shield: his blows wear the shield this much more.
pub const SHIELD_WEAR: f64 = 1.3;
/// A foe's attack never starts up quicker than this (COMBAT.md §0): every lesson keeps it.
pub const FOE_FLOOR: u16 = 18;
pub const MELEE_FLOOR: u16 = 17;
/// Below this share of its health a foe that lives on has *survived* (a deed).
pub const LOW: f64 = 0.25;
/// Guard breaks in one fight that make a Shield-Breaker.
pub const GUARD_BREAKS: u16 = 3;
/// A rival who kills him takes this share of the silver he carries (WARBAND.md §17, decision 2),
/// and gives it back this many times over when slain.
pub const SILVER_TAKEN: f64 = 0.15;
pub const SILVER_BACK: f64 = 1.5;
/// A killing blow that took at least this much silver makes a Silver-Drinker, not a Bane.
pub const SILVER_DEED: u32 = 100;
/// Runes a slain rival gives, per rank.
pub const RUNES_PER_RANK: u32 = 1;

/// Every kind, in the order the save names them (append only).
pub const KINDS: &[EnemyKind] = &{
    use EnemyKind as K;
    [
        K::Skeleton,
        K::Bat,
        K::Vampire,
        K::Slime,
        K::Witch,
        K::Minion,
        K::Helmeted,
        K::Werewolf,
        K::Blemmyes,
        K::LargeBat,
        K::Tengu,
        K::Rasetsu,
        K::DireWolf,
        K::Jorogumo,
        K::Cobra,
        K::Kappa,
        K::Rokurokubi,
        K::Kitsune,
        K::Oni,
        K::Tulukus,
        K::Albasti,
        K::Cin,
        K::Arcura,
        K::Karakoncolos,
        K::Tepegoz,
        K::Gashadokuro,
        K::Ejder,
        K::Goryo,
        K::FrostGiant,
        K::Surtr,
        K::Nidhogg,
        K::Hel,
        K::Peerless,
        K::Ganglot,
        K::Gaki,
        K::Ellepige,
        K::Mammon,
        K::GoldGolem,
        // The six other princes' courts
        K::Lucifer,
        K::Reflection,
        K::Leviathan,
        K::Serpentling,
        K::Amon,
        K::Raven,
        K::Belphegor,
        K::Nightmare,
        K::Beelzebub,
        K::FlySwarm,
        K::Asmodeus,
    ]
};

/// Every brew, in the order the save names them (as `game::save`'s belt does).
pub const ELIXIRS: [PotionKind; 11] = [
    PotionKind::Health,
    PotionKind::Stamina,
    PotionKind::Strength,
    PotionKind::Haste,
    PotionKind::Bark,
    PotionKind::Fury,
    PotionKind::Fire,
    PotionKind::Invulnerability,
    PotionKind::Ragnarok,
    PotionKind::Spirit,
    PotionKind::Life,
];

/// The elixirs whose effect reaches a foe (a rival can be proof against them, or bring their antidote).
pub const BITING: [PotionKind; 7] = [
    PotionKind::Strength,
    PotionKind::Haste,
    PotionKind::Fury,
    PotionKind::Fire,
    PotionKind::Ragnarok,
    PotionKind::Spirit,
    PotionKind::Bark,
];

/// An elixir as the rivals and the old man say it.
pub fn elixir_name(p: PotionKind) -> &'static str {
    match p {
        PotionKind::Health => "health",
        PotionKind::Stamina => "stamina",
        PotionKind::Strength => "strength",
        PotionKind::Haste => "haste",
        PotionKind::Bark => "bark",
        PotionKind::Fury => "fury",
        PotionKind::Fire => "fire",
        PotionKind::Invulnerability => "the unbreakable draught",
        PotionKind::Ragnarok => "Ragnarök",
        PotionKind::Spirit => "spirit",
        PotionKind::Life => "life",
    }
}

fn index_of<T: PartialEq>(list: &[T], x: &T) -> usize {
    list.iter().position(|y| y == x).unwrap_or(0)
}

// ------------------------------------------------------------------ the traits

/// A weapon family, as weaknesses and the old man's counsel name them.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Family {
    Steel,
    Fire,
    Thrown,
}

impl Family {
    pub const ALL: [Family; 3] = [Family::Steel, Family::Fire, Family::Thrown];

    /// The family of tier `tier` of `hero`'s armory: thrown (or shot), burning, or plain steel.
    pub fn of(hero: HeroKind, tier: u8) -> Family {
        let b = progression::blade(hero, tier);
        if b.throw.is_some() || b.arrows > 0 {
            Family::Thrown
        } else if b.burn > 0.0 || b.fire_trail || b.flash > 0.0 {
            Family::Fire
        } else {
            Family::Steel
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Family::Steel => "steel",
            Family::Fire => "fire",
            Family::Thrown => "the thrown blade",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub fn name(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Right => "right",
        }
    }
}

/// What a rival is good at (WARBAND.md §8.2).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Strength {
    /// +15 armor.
    Unflinching,
    /// Tracks 4 f longer, still committing for the last 12.
    Patient,
    /// One elixir's effect on him halved.
    ElixirProof(PotionKind),
    /// Leads a pack of 5.
    PackLord,
    /// Enraged when his pack dies.
    Vengeful,
}

/// Where a rival can be had.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Weakness {
    /// After he lands a blow he taunts, open for 60 f.
    Proud,
    /// Blows from the side of his old wound land ×1.3.
    Scarred(Side),
    /// That family staggers him at once.
    Fears(Family),
    /// Stops to pick up dropped silver (a 50 f opening).
    Greedy,
    /// Poise −30%.
    OldWound,
}

/// How a rival carries himself.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Personality {
    /// Taunts by name, by memory.
    Braggart,
    /// Routs at 40% health and runs to rank up.
    Coward,
    /// His pack stands back; the threat pool is his alone (a 1-on-1 the player can accept).
    Duelist,
    /// When hurt, blows a horn that wakes the next den (60 f tell; interrupt it).
    Schemer,
    /// Hunts the hero if he slew a rival of the same breed (his "brother").
    Avenger,
}

/// One of a rival's three traits, as the card, the board and the old man name it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Trait {
    Strength(Strength),
    Weakness(Weakness),
    Personality(Personality),
}

/// The `known` bit of each trait.
pub const KNOW_STRENGTH: u8 = 1;
pub const KNOW_WEAKNESS: u8 = 2;
pub const KNOW_PERSONALITY: u8 = 4;

impl Trait {
    /// Every trait there is.
    pub fn all() -> Vec<Trait> {
        let mut v: Vec<Trait> = vec![
            Trait::Strength(Strength::Unflinching),
            Trait::Strength(Strength::Patient),
            Trait::Strength(Strength::PackLord),
            Trait::Strength(Strength::Vengeful),
        ];
        v.extend(BITING.iter().map(|&p| Trait::Strength(Strength::ElixirProof(p))));
        v.extend([Weakness::Proud, Weakness::Greedy, Weakness::OldWound].map(Trait::Weakness));
        v.extend([Side::Left, Side::Right].map(|s| Trait::Weakness(Weakness::Scarred(s))));
        v.extend(Family::ALL.map(|f| Trait::Weakness(Weakness::Fears(f))));
        v.extend(
            [Personality::Braggart, Personality::Coward, Personality::Duelist, Personality::Schemer, Personality::Avenger]
                .map(Trait::Personality),
        );
        v
    }

    /// Its `known` bit.
    pub fn bit(self) -> u8 {
        match self {
            Trait::Strength(_) => KNOW_STRENGTH,
            Trait::Weakness(_) => KNOW_WEAKNESS,
            Trait::Personality(_) => KNOW_PERSONALITY,
        }
    }

    /// Its name on the card and the board.
    pub fn name(self) -> String {
        match self {
            Trait::Strength(s) => match s {
                Strength::Unflinching => "Unflinching".into(),
                Strength::Patient => "Patient".into(),
                Strength::ElixirProof(p) => format!("Proof against {}", elixir_name(p)),
                Strength::PackLord => "Pack-lord".into(),
                Strength::Vengeful => "Vengeful".into(),
            },
            Trait::Weakness(w) => match w {
                Weakness::Proud => "Proud".into(),
                Weakness::Scarred(side) => format!("Scarred on the {}", side.name()),
                Weakness::Fears(f) => format!("Fears {}", f.name()),
                Weakness::Greedy => "Greedy".into(),
                Weakness::OldWound => "Old wound".into(),
            },
            Trait::Personality(p) => match p {
                Personality::Braggart => "Braggart".into(),
                Personality::Coward => "Coward".into(),
                Personality::Duelist => "Duelist".into(),
                Personality::Schemer => "Schemer".into(),
                Personality::Avenger => "Avenger".into(),
            },
        }
    }

    /// The words that name it in a line of the old man's (lower case; no trait's are in another's).
    pub fn keyword(self) -> String {
        match self {
            Trait::Strength(Strength::ElixirProof(p)) => format!("proof against {}", elixir_name(p)),
            Trait::Weakness(Weakness::Scarred(side)) => format!("scarred on the {}", side.name()),
            Trait::Weakness(Weakness::Fears(f)) => format!("fears {}", f.name()),
            Trait::Strength(Strength::PackLord) => "pack-lord".into(),
            Trait::Weakness(Weakness::OldWound) => "old wound".into(),
            t => t.name().to_lowercase(),
        }
    }
}

/// What the hero mostly does to him, turned into what he does about it (WARBAND.md §8.3).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Lesson {
    /// He parries: two startups for the same move, read the fill, not the rhythm.
    Parry,
    /// He evades: the wide arc or the floor move, and the leap's landing waited out.
    Leap,
    /// He blocks: the move with the most poise, and the shield worn ×1.3.
    Shield,
    /// He drinks this elixir: its effect on him halved.
    Antidote(PotionKind),
    /// He finishes with L3: the rival leaps on L3's startup when he can.
    Finisher,
}

impl Lesson {
    /// What the name card says.
    pub fn line(self, kind: EnemyKind) -> String {
        let he = names::pronoun(kind, true);
        match self {
            Lesson::Parry => format!("{he} remembers your parry."),
            Lesson::Leap => format!("{he} remembers your leap."),
            Lesson::Shield => format!("{he} remembers your shield."),
            Lesson::Antidote(p) => format!("{he} brought the antidote to {}.", elixir_name(p)),
            Lesson::Finisher => format!("{he} knows your finisher."),
        }
    }

    /// The parry's lesson: the two startups he picks between for `mv` (frames from its tell to its
    /// blow), never quicker than the move's own nor the floors. He holds his wind-up for the
    /// difference before he begins the move, so the move itself stays as it is; `None` for any
    /// other lesson, or a move that does not strike.
    pub fn startups(self, mv: MoveId) -> Option<(u16, u16)> {
        let d = mv.data();
        if self != Lesson::Parry || !d.strikes() {
            return None;
        }
        let floor = if d.projectile { FOE_FLOOR } else { FOE_FLOOR.max(MELEE_FLOOR) };
        let a = d.startup.max(floor);
        Some((a, a + PARRY_SPREAD))
    }

    /// The frames he holds his wind-up before beginning `mv`, for the long startup (`long`) or the
    /// short: the difference between the chosen startup and the move's own.
    pub fn hold(self, mv: MoveId, long: bool) -> u16 {
        match self.startups(mv) {
            Some((a, b)) => (if long { b } else { a }) - mv.data().startup,
            None => 0,
        }
    }

    /// The move of `moves` (his breed's own, in its order) the lesson makes him prefer when it is
    /// rested and in range: the leap's, his floor move or his widest arc; the shield's, the move
    /// with the most poise. `None` when the lesson has no preference.
    pub fn prefer(self, moves: &[MoveId]) -> Option<MoveId> {
        let striking = || moves.iter().copied().filter(|m| m.data().strikes());
        match self {
            Lesson::Leap => striking()
                .find(|m| matches!(m.data().shape, Shape::Mark { .. } | Shape::Quake { .. }))
                .or_else(|| striking().max_by(|a, b| a.data().half_angle.total_cmp(&b.data().half_angle))),
            Lesson::Shield => striking().max_by(|a, b| a.data().poise.total_cmp(&b.data().poise)),
            _ => None,
        }
    }

    /// The leap's lesson: frames he waits, after the hero's leap lands, before he asks to strike.
    pub fn waits_after_leap(self) -> u16 {
        if self == Lesson::Leap {
            LEAP_LANDING
        } else {
            0
        }
    }

    /// The shield's lesson: how much more his blows wear the shield.
    pub fn shield_wear(self) -> f64 {
        if self == Lesson::Shield {
            SHIELD_WEAR
        } else {
            1.0
        }
    }

    /// The antidote's lesson: how much of elixir `p`'s effect reaches him.
    pub fn elixir_scale(self, p: PotionKind) -> f64 {
        if self == Lesson::Antidote(p) {
            ELIXIR_PROOF
        } else {
            1.0
        }
    }

    /// The finisher's lesson: he leaps on the startup of L3 when he can (his own evade).
    pub fn evades_finisher(self) -> bool {
        self == Lesson::Finisher
    }
}

/// A piece of a slain rival for the workshop's wall.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Trophy {
    Helm,
    Feather,
    Jar,
    Hat,
    Claw,
    Fang,
    Skull,
    Blade,
    Silk,
    Dish,
    Hair,
    Tail,
    Horn,
    Ember,
    Root,
    Eye,
    Scale,
    Mask,
    Crown,
    Shroud,
    Bowl,
    Thorn,
    Coin,
    Mirror,
}

impl Trophy {
    pub const ALL: [Trophy; 24] = [
        Trophy::Helm,
        Trophy::Feather,
        Trophy::Jar,
        Trophy::Hat,
        Trophy::Claw,
        Trophy::Fang,
        Trophy::Skull,
        Trophy::Blade,
        Trophy::Silk,
        Trophy::Dish,
        Trophy::Hair,
        Trophy::Tail,
        Trophy::Horn,
        Trophy::Ember,
        Trophy::Root,
        Trophy::Eye,
        Trophy::Scale,
        Trophy::Mask,
        Trophy::Crown,
        Trophy::Shroud,
        Trophy::Bowl,
        Trophy::Thorn,
        Trophy::Coin,
        Trophy::Mirror,
    ];

    pub fn of(kind: EnemyKind) -> Trophy {
        use EnemyKind as K;
        match kind {
            K::Skeleton | K::Helmeted | K::FrostGiant => Trophy::Helm,
            K::Bat | K::Vampire | K::LargeBat | K::Tulukus | K::Tengu => Trophy::Feather,
            K::Slime => Trophy::Jar,
            K::Witch => Trophy::Hat,
            K::Minion | K::Albasti | K::Karakoncolos => Trophy::Claw,
            K::Werewolf | K::DireWolf | K::Cobra | K::Nidhogg => Trophy::Fang,
            K::Blemmyes | K::Gashadokuro => Trophy::Skull,
            K::Rasetsu | K::Surtr => Trophy::Blade,
            K::Jorogumo => Trophy::Silk,
            K::Kappa => Trophy::Dish,
            K::Rokurokubi => Trophy::Hair,
            K::Kitsune => Trophy::Tail,
            K::Oni => Trophy::Horn,
            K::Cin => Trophy::Ember,
            K::Arcura => Trophy::Root,
            K::Tepegoz => Trophy::Eye,
            K::Ejder => Trophy::Scale,
            K::Goryo => Trophy::Mask,
            K::Hel | K::Mammon => Trophy::Crown,
            // The other princes, and what they raise (for now a crown and a coin, as Mammon's).
            K::Lucifer | K::Leviathan | K::Amon | K::Belphegor | K::Beelzebub | K::Asmodeus => Trophy::Crown,
            K::Reflection | K::Serpentling | K::Raven | K::Nightmare | K::FlySwarm => Trophy::Coin,
            // Krampus's lair: his horn, and a Percht's carved mask.
            K::Krampus => Trophy::Horn,
            K::Percht => Trophy::Mask,
            K::Ganglot => Trophy::Shroud,
            K::Gaki => Trophy::Bowl,
            K::Ellepige => Trophy::Thorn,
            K::GoldGolem => Trophy::Coin,
            K::Peerless => Trophy::Mirror,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Trophy::Helm => "a dented helm",
            Trophy::Feather => "a black feather",
            Trophy::Jar => "a jar of bog",
            Trophy::Hat => "a witch's hat",
            Trophy::Claw => "a claw",
            Trophy::Fang => "a fang",
            Trophy::Skull => "a skull",
            Trophy::Blade => "a curved blade",
            Trophy::Silk => "a skein of spider-silk",
            Trophy::Dish => "a kappa's dish",
            Trophy::Hair => "a long lock of hair",
            Trophy::Tail => "a fox's tail",
            Trophy::Horn => "an oni's horn",
            Trophy::Ember => "an ember that will not die",
            Trophy::Root => "a thorny root",
            Trophy::Eye => "a great eye, dried",
            Trophy::Scale => "a dragon's scale",
            Trophy::Mask => "a noble's mask",
            Trophy::Crown => "a crown",
            Trophy::Shroud => "a grey shroud",
            Trophy::Bowl => "a begging bowl",
            Trophy::Thorn => "a thorn",
            Trophy::Coin => "a gold coin",
            Trophy::Mirror => "a mirror's shard",
        }
    }
}

// ------------------------------------------------------------------ memory

/// Where a fight with a rival took place.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Place {
    Realm(Sin),
    Court(Sin),
    Dungeon(u8),
    Ring,
    Hub,
}

impl Place {
    pub fn name(self) -> String {
        match self {
            Place::Realm(s) => s.realm().replacen("The ", "the ", 1),
            Place::Court(s) => format!("the court of {}", s.prince()),
            Place::Dungeon(n) => format!("dungeon {}", n + 1),
            Place::Ring => "the Sparring Ring".to_string(),
            Place::Hub => "the Portals of Hell".to_string(),
        }
    }
}

/// How a fight with a rival ended.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Outcome {
    /// The hero slew him.
    HeroWon,
    /// He slew the hero.
    RivalWon,
    /// He got away, low (routed, or ran to rank up).
    RivalFled,
    /// The hero walked off (the leash ran out with him alive).
    HeroLeft,
}

/// The hero's answers to a rival in one fight.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub struct Verbs {
    pub parries: u16,
    pub blocks: u16,
    pub evades: u16,
    pub finishers: u16,
    /// His guard broken by the rival.
    pub guard_breaks: u16,
    /// His blows that landed on the rival.
    pub wounds: u16,
}

impl Verbs {
    fn add(&mut self, o: &Verbs) {
        self.parries += o.parries;
        self.blocks += o.blocks;
        self.evades += o.evades;
        self.finishers += o.finishers;
        self.guard_breaks += o.guard_breaks;
        self.wounds += o.wounds;
    }
}

/// One fight a rival remembers: where, how it ended, which hero with which blade, the elixirs he
/// drank, and his answers.
#[derive(Clone, PartialEq, Debug)]
pub struct Encounter {
    pub place: Place,
    pub outcome: Outcome,
    pub hero: HeroKind,
    pub family: Family,
    pub tier: u8,
    /// Distinct, in the order he drank them.
    pub elixirs: Vec<PotionKind>,
    pub verbs: Verbs,
}

impl Encounter {
    /// The board's line about it: "Slew Ragnar in the Vaults of Mammon".
    pub fn line(&self) -> String {
        let n = self.hero.name();
        let what = match self.outcome {
            Outcome::HeroWon => format!("Fell to {n}"),
            Outcome::RivalWon => format!("Slew {n}"),
            Outcome::RivalFled => format!("Fled from {n}"),
            Outcome::HeroLeft => format!("{n} walked off"),
        };
        format!("{what} in {}", self.place.name())
    }
}

// ------------------------------------------------------------------ rivals

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub struct RivalId(pub u32);

/// A foe who became somebody.
#[derive(Clone, PartialEq, Debug)]
pub struct Rival {
    pub id: RivalId,
    pub name: String,
    pub epithet: String,
    pub kind: EnemyKind,
    /// 1 to 5.
    pub rank: u8,
    /// What made him somebody, and against whom.
    pub deed: Deed,
    pub hero: HeroKind,
    pub strength: Strength,
    pub weakness: Weakness,
    pub personality: Personality,
    pub lesson: Option<Lesson>,
    /// Up to `MEMORY` encounters, oldest first.
    pub memory: Vec<Encounter>,
    /// Which traits the hero has found (or paid for): `KNOW_*` bits.
    pub known: u8,
    /// The silver he took from the hero.
    pub silver_held: u32,
    pub trophy: Trophy,
    pub alive: bool,
}

/// What slaying a rival gives.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Reward {
    pub runes: u32,
    pub silver: u32,
    pub trophy: Trophy,
}

/// The name card, as text (`render::hud::ledger` draws it).
#[derive(Clone, PartialEq, Debug)]
pub struct Card {
    /// "Kara Ayaz the Shield-Breaker".
    pub title: String,
    pub breed: &'static str,
    pub rank: u8,
    /// "He remembers your parry."
    pub lesson: Option<String>,
    /// Strength, weakness, personality: the name when found, else `None` ("?").
    pub traits: [(&'static str, Option<String>); 3],
}

impl Rival {
    /// His name with his epithet: "Kara Ayaz the Shield-Breaker", "Kolgrim, Ragnar's Bane".
    pub fn full_name(&self) -> String {
        if self.epithet.is_empty() {
            self.name.clone()
        } else if self.epithet.starts_with("the ") {
            format!("{} {}", self.name, self.epithet)
        } else {
            format!("{}, {}", self.name, self.epithet)
        }
    }

    /// His three traits, as they truly are.
    pub fn true_traits(&self) -> [Trait; 3] {
        [Trait::Strength(self.strength), Trait::Weakness(self.weakness), Trait::Personality(self.personality)]
    }

    pub fn knows(&self, t: Trait) -> bool {
        self.known & t.bit() != 0
    }

    /// His last encounter.
    pub fn last(&self) -> Option<&Encounter> {
        self.memory.last()
    }

    /// His answers, all his memory long.
    pub fn verbs(&self) -> Verbs {
        let mut v = Verbs::default();
        for e in &self.memory {
            v.add(&e.verbs);
        }
        v
    }

    /// His health, times his breed's: +8% a rank.
    pub fn health_scale(&self) -> f64 {
        1.0 + HEALTH_PER_RANK * self.rank as f64
    }

    /// Armor he adds (Unflinching).
    pub fn armor_bonus(&self) -> f64 {
        if self.strength == Strength::Unflinching {
            UNFLINCHING_ARMOR
        } else {
            0.0
        }
    }

    /// His poise, times his breed's (an old wound takes 30%).
    pub fn poise_scale(&self) -> f64 {
        if self.weakness == Weakness::OldWound {
            OLD_WOUND_POISE
        } else {
            1.0
        }
    }

    /// Frames his tracking of `mv` runs longer (Patient): up to 4, never into the last 12 before
    /// the blow, so a step aside still beats it.
    pub fn track_bonus(&self, mv: MoveId) -> u16 {
        if self.strength != Strength::Patient {
            return 0;
        }
        let d = mv.data();
        PATIENT_TRACK.min(d.startup.saturating_sub(COMMIT_FOR + d.track))
    }

    /// How much of elixir `p`'s effect reaches him: halved if he is proof against it, or brought
    /// its antidote (the two do not stack).
    pub fn elixir_scale(&self, p: PotionKind) -> f64 {
        let proof = if self.strength == Strength::ElixirProof(p) { ELIXIR_PROOF } else { 1.0 };
        let antidote = self.lesson.map_or(1.0, |l| l.elixir_scale(p));
        proof.min(antidote)
    }

    /// The pack he leads, if he is a pack-lord.
    pub fn pack_size(&self) -> Option<usize> {
        (self.strength == Strength::PackLord).then_some(PACK_LORD)
    }

    /// Elite affixes he carries (one at rank 2, two at rank 4; `logic::war::pack` names them).
    pub fn affixes(&self) -> u8 {
        (self.rank >= 2) as u8 + (self.rank >= 4) as u8
    }

    /// Dresses a duelist as him: his health by rank, his armor, his poise. Never a move's frames.
    pub fn dress(&self, du: &mut Duelist) {
        du.max_hp *= self.health_scale();
        du.hp = du.max_hp;
        du.f.armor += self.armor_bonus();
        du.f.max_poise *= self.poise_scale();
        du.f.poise = du.f.max_poise;
        if self.strength == Strength::Patient {
            du.patient = PATIENT_TRACK;
        }
    }

    /// His name card.
    pub fn card(&self) -> Card {
        let traits = self.true_traits();
        let shown = |k: usize| self.knows(traits[k]).then(|| traits[k].name());
        Card {
            title: self.full_name(),
            breed: names::breed_name(self.kind),
            rank: self.rank,
            lesson: self.lesson.map(|l| l.line(self.kind)),
            traits: [("STRENGTH", shown(0)), ("WEAKNESS", shown(1)), ("NATURE", shown(2))],
        }
    }
}

/// The lesson a rival takes from the hero's most used answer to him (rank 3 and up; one at a time).
pub fn choose_lesson(r: &Rival) -> Option<Lesson> {
    if r.rank < LESSON_FROM || r.memory.is_empty() {
        return None;
    }
    let v = r.verbs();
    let mut best: Option<(u32, Lesson)> = None;
    let mut consider = |score: u32, l: Lesson| {
        if score >= LESSON_MIN && best.is_none_or(|(s, _)| score > s) {
            best = Some((score, l));
        }
    };
    consider(v.parries as u32, Lesson::Parry);
    consider(v.evades as u32, Lesson::Leap);
    consider(v.blocks as u32, Lesson::Shield);
    consider(v.finishers as u32, Lesson::Finisher);
    for p in BITING {
        let drunk = r.memory.iter().filter(|e| e.elixirs.contains(&p)).count() as u32;
        consider(drunk * ELIXIR_WEIGHT, Lesson::Antidote(p));
    }
    best.map(|(_, l)| l)
}

/// What the old man says of a rival: one trait he truly has (the first the hero has found, of
/// weakness, strength and nature; with none found, his weakness, which is what the old man sells).
/// Built from the rival's own traits, so it is true by construction (rule I8).
pub fn counsel_line_for(r: &Rival) -> String {
    let traits = r.true_traits();
    let t = [traits[1], traits[0], traits[2]].into_iter().find(|&t| r.knows(t)).unwrap_or(traits[1]);
    let (n, he, his) = (r.name.as_str(), names::pronoun(r.kind, false), names::possessive(r.kind, false));
    let he_cap = names::pronoun(r.kind, true);
    let him = if names::feminine(r.kind) { "her" } else { "him" };
    let k = t.keyword();
    match t {
        Trait::Weakness(w) => match w {
            Weakness::Proud => format!("{n} is {k}. When {his} blow lands, {he} stops to gloat. Strike then."),
            Weakness::Scarred(_) => format!("{n} is {k}. Come at {him} from that side; it bites deeper."),
            Weakness::Fears(_) => format!("{n} {k}. Bring it, and {he} reels at the first touch."),
            Weakness::Greedy => format!("{n} is {k}. Drop a coin and {he} stoops for it."),
            Weakness::OldWound => format!("{n} carries an {k}. Lean on {him} and {he} breaks sooner."),
        },
        Trait::Strength(s) => match s {
            Strength::Unflinching => format!("{n} is {k}. Light blows will not stop {him}; finish your string."),
            Strength::Patient => format!("{n} is {k}. {he_cap} turns with you longer; step aside late."),
            Strength::ElixirProof(_) => format!("{n} is {k}. Leave that bottle on the belt."),
            Strength::PackLord => format!("{n} is a {k}. {his} pack is five; thin it before you face {him}."),
            Strength::Vengeful => format!("{n} is {k}. Kill {his} pack last, or {he} comes in raging."),
        },
        Trait::Personality(p) => match p {
            Personality::Braggart => format!("{n} is a {k}. Let {him} talk; {he} talks instead of striking."),
            Personality::Coward => format!("{n} is a {k}. Wound {him} and {he} runs; cut off the door."),
            Personality::Duelist => format!("{n} is a {k}. {he_cap} wants you alone. Give {him} that, if you dare."),
            Personality::Schemer => format!("{n} is a {k}. Hurt, {he} reaches for a horn. Stop {his} breath."),
            Personality::Avenger => format!("{n} is an {k}. You slew {his} kin, and {he} has come for you."),
        },
    }
}

// ------------------------------------------------------------------ the ledger

/// Every rival of the run, the living and the slain (WARBAND.md §8).
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Ledger {
    pub rivals: Vec<Rival>,
    pub next_id: u32,
    /// Rivals the hero has slain this run.
    pub slain: u32,
}

impl Ledger {
    pub fn live(&self) -> impl Iterator<Item = &Rival> {
        self.rivals.iter().filter(|r| r.alive)
    }

    pub fn get(&self, id: RivalId) -> Option<&Rival> {
        self.rivals.iter().find(|r| r.id == id)
    }

    pub fn get_mut(&mut self, id: RivalId) -> Option<&mut Rival> {
        self.rivals.iter_mut().find(|r| r.id == id)
    }

    /**
     * A foe of `kind` did `deed` to `hero`: he becomes a rival of rank 1, named from his breed's
     * folklore, his epithet from the deed, his traits rolled from `seed`. With nine already living,
     * the lowest rank (the oldest of equals) is forgotten to make room.
     */
    pub fn promote(&mut self, kind: EnemyKind, deed: Deed, hero: HeroKind, seed: u32) -> RivalId {
        if self.live().count() >= MAX_LIVE {
            if let Some(low) = self.live().min_by_key(|r| (r.rank, r.id)).map(|r| r.id) {
                self.rivals.retain(|r| r.id != low);
            }
        }
        let mut rng = Rng::new(seed ^ 0x5249_5641);
        let strength = match rng.index(5) {
            0 => Strength::Unflinching,
            1 => Strength::Patient,
            2 => Strength::ElixirProof(BITING[rng.index(BITING.len())]),
            3 => Strength::PackLord,
            _ => Strength::Vengeful,
        };
        let weakness = match rng.index(5) {
            0 => Weakness::Proud,
            1 => Weakness::Scarred(if rng.next() < 0.5 { Side::Left } else { Side::Right }),
            2 => Weakness::Fears(Family::ALL[rng.index(3)]),
            3 => Weakness::Greedy,
            _ => Weakness::OldWound,
        };
        let personality =
            [Personality::Braggart, Personality::Coward, Personality::Duelist, Personality::Schemer, Personality::Avenger][rng.index(5)];
        // No two rivals of the run share a name, living or slain (the board shows both).
        let mut name = names::name(kind, seed);
        for k in 1..64u32 {
            let given = |n: &str| n.split_whitespace().next().unwrap_or("").to_string();
            if !self.rivals.iter().any(|r| given(&r.name) == given(&name)) {
                break;
            }
            name = names::name(kind, seed.wrapping_add(k.wrapping_mul(0x9e37_79b9)));
        }
        let id = RivalId(self.next_id);
        self.next_id += 1;
        self.rivals.push(Rival {
            id,
            name,
            epithet: names::epithet(deed, hero, seed),
            kind,
            rank: 1,
            deed,
            hero,
            strength,
            weakness,
            personality,
            lesson: None,
            memory: Vec::new(),
            known: 0,
            silver_held: 0,
            trophy: Trophy::of(kind),
            alive: true,
        });
        id
    }

    /// He won again: a rank up (to 5), and from rank 3 his lesson is (re)drawn.
    pub fn rank_up(&mut self, id: RivalId) {
        if let Some(r) = self.get_mut(id) {
            r.rank = (r.rank + 1).min(MAX_RANK);
            r.lesson = choose_lesson(r);
        }
    }

    /// He remembers a fight (the oldest of nine forgotten), and redraws his lesson from it.
    pub fn record(&mut self, id: RivalId, e: Encounter) {
        if let Some(r) = self.get_mut(id) {
            r.memory.push(e);
            if r.memory.len() > MEMORY {
                r.memory.remove(0);
            }
            r.lesson = choose_lesson(r);
        }
    }

    /// A fight with him is over: it is remembered, and unless the hero slew him he ranks up: he won
    /// it, fled it low, or saw it end with the hero gone (walked off, or fallen to another). WARBAND.md
    /// W9: with only his wins and his flights counted, no rival of fifteen whole runs reached a lesson.
    pub fn after_fight(&mut self, id: RivalId, e: Encounter) {
        let won = matches!(e.outcome, Outcome::RivalWon | Outcome::RivalFled | Outcome::HeroLeft);
        self.record(id, e);
        if won {
            self.rank_up(id);
        }
    }

    /// He took `amount` of the hero's silver at a death.
    pub fn took_silver(&mut self, id: RivalId, amount: u32) {
        if let Some(r) = self.get_mut(id) {
            r.silver_held += amount;
        }
    }

    /// The hero found (or paid for) one of his traits.
    pub fn reveal(&mut self, id: RivalId, t: Trait) {
        if let Some(r) = self.get_mut(id) {
            r.known |= t.bit();
        }
    }

    /// Has a rival of `kind` been slain (an avenger of that breed hunts him)?
    pub fn brother_slain(&self, kind: EnemyKind) -> bool {
        self.rivals.iter().any(|r| !r.alive && r.kind == kind)
    }

    /**
     * The rivals waiting for him in a realm visit at `tier` (1 to 10): one, two from tier 5. An
     * avenger whose brother fell comes first, then the highest ranks; `seed` (the realm's) turns
     * the choice among the first three, so the same visit always meets the same rivals.
     */
    pub fn offer(&self, tier: usize, seed: u32) -> Vec<RivalId> {
        let count = if tier >= 5 { 2 } else { 1 };
        let hunting = |r: &Rival| r.personality == Personality::Avenger && self.brother_slain(r.kind);
        let mut first: Vec<&Rival> = self.live().filter(|r| hunting(r)).collect();
        let mut rest: Vec<&Rival> = self.live().filter(|r| !hunting(r)).collect();
        first.sort_by_key(|r| (std::cmp::Reverse(r.rank), r.id));
        rest.sort_by_key(|r| (std::cmp::Reverse(r.rank), r.memory.len(), r.id));
        let head = rest.len().min(3);
        if head > 0 {
            let turn = Rng::new(seed ^ 0x4f46_4652).index(head);
            rest[..head].rotate_left(turn);
        }
        first.into_iter().chain(rest).take(count).map(|r| r.id).collect()
    }

    /// He is slain: the runes of his rank, the silver he took ×1.5, and his trophy.
    pub fn slain(&mut self, id: RivalId) -> Option<Reward> {
        let r = self.get_mut(id).filter(|r| r.alive)?;
        r.alive = false;
        let reward =
            Reward { runes: RUNES_PER_RANK * r.rank as u32, silver: (r.silver_held as f64 * SILVER_BACK).round() as u32, trophy: r.trophy };
        r.silver_held = 0;
        self.slain += 1;
        // The oldest of the fallen are forgotten.
        while self.rivals.iter().filter(|r| !r.alive).count() > MAX_FALLEN {
            if let Some(i) = self.rivals.iter().position(|r| !r.alive) {
                self.rivals.remove(i);
            }
        }
        Some(reward)
    }

    /// The rival who would take a slain general's place (W6): the highest rank of 4 or more.
    pub fn usurper(&self) -> Option<RivalId> {
        self.live().filter(|r| r.rank >= 4).max_by_key(|r| (r.rank, r.memory.len(), std::cmp::Reverse(r.id))).map(|r| r.id)
    }
}

// ------------------------------------------------------------------ the fight's tracker

/**
 * What one fight with one foe came to, read from the war log: the hero's answers to it, the
 * elixirs he drank, how low it fell, and whether it ended him. The brawl makes one per foe it
 * cares about (every pack leader, every rival) and feeds it each event with the hero's health
 * and the foe's after it; when the fight is over, `deed` says whether the foe is promoted and
 * `encounter` what a rival remembers.
 */
#[derive(Clone, PartialEq, Debug)]
pub struct EncounterTracker {
    pub foe: u32,
    pub kind: EnemyKind,
    pub rival: Option<RivalId>,
    pub verbs: Verbs,
    pub elixirs: Vec<PotionKind>,
    /// The lowest share of its health it fell to.
    pub low: f64,
    pub killed_hero: bool,
    pub slain: bool,
    /// It took part (a tell, a blow either way).
    pub met: bool,
}

impl EncounterTracker {
    pub fn new(foe: u32, kind: EnemyKind, rival: Option<RivalId>) -> Self {
        Self { foe, kind, rival, verbs: Verbs::default(), elixirs: Vec::new(), low: 1.0, killed_hero: false, slain: false, met: false }
    }

    /// Reads one event of the war log: `hero_hp` is his health after it (0 or less: dead),
    /// `foe_hp_frac` the foe's share of its health after it. Events about other foes are passed
    /// over; the elixirs he drinks count whoever they were for.
    pub fn observe(&mut self, e: &WarEvent, hero_hp: f64, foe_hp_frac: f64) {
        if let WarEvent::Drank { potion } = *e {
            if !self.elixirs.contains(&potion) {
                self.elixirs.push(potion);
            }
            return;
        }
        if e.foe() != Some(self.foe) {
            return;
        }
        self.met = true;
        self.low = self.low.min(foe_hp_frac.clamp(0.0, 1.0));
        match *e {
            WarEvent::Landed { how, dmg, lands, .. } => {
                if lands {
                    match how {
                        Defense::Parried => self.verbs.parries += 1,
                        Defense::Blocked => self.verbs.blocks += 1,
                        Defense::Evaded => self.verbs.evades += 1,
                        Defense::GuardBroken => self.verbs.guard_breaks += 1,
                        Defense::Open | Defense::Immune => {}
                    }
                }
                if dmg > 0.0 && hero_hp <= 0.0 && matches!(how, Defense::Open | Defense::GuardBroken) {
                    self.killed_hero = true;
                }
            }
            WarEvent::HeroHit { finisher, .. } => {
                self.verbs.wounds += 1;
                if finisher {
                    self.verbs.finishers += 1;
                }
            }
            WarEvent::Slain { .. } => {
                self.slain = true;
                self.low = 0.0;
            }
            _ => {}
        }
    }

    /// How the fight ended; `left` when it is over because the hero walked off (the leash ran out).
    pub fn outcome(&self, left: bool) -> Outcome {
        if self.slain {
            Outcome::HeroWon
        } else if self.killed_hero {
            Outcome::RivalWon
        } else if !left && self.low < LOW {
            Outcome::RivalFled
        } else {
            Outcome::HeroLeft
        }
    }

    /**
     * The deed that makes it somebody, if any: the killing blow (or, taking `silver_taken` of 100
     * or more, the silver), three guard breaks, living on below a quarter of its health, or being a
     * pack leader (`leader`) he walked away from (`left`).
     */
    pub fn deed(&self, silver_taken: u32, leader: bool, left: bool) -> Option<Deed> {
        if self.killed_hero {
            Some(if silver_taken >= SILVER_DEED { Deed::TookSilver } else { Deed::KillingBlow })
        } else if self.slain {
            None
        } else if self.verbs.guard_breaks >= GUARD_BREAKS {
            Some(Deed::ThreeGuardBreaks)
        } else if self.low < LOW {
            Some(Deed::SurvivedLow)
        } else if leader && left {
            Some(Deed::LeftBehind)
        } else {
            None
        }
    }

    /// What the rival remembers of it.
    pub fn encounter(&self, place: Place, hero: HeroKind, tier: u8, left: bool) -> Encounter {
        Encounter {
            place,
            outcome: self.outcome(left),
            hero,
            family: Family::of(hero, tier),
            tier,
            elixirs: self.elixirs.clone(),
            verbs: self.verbs,
        }
    }
}

// ------------------------------------------------------------------ barks

/// When a rival speaks.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Situation {
    /// Within ten tiles of the hero, his card up.
    Meet,
    /// He won: the hero fell.
    Win,
    /// He runs (a rout, or the coward's flight).
    Flee,
    /// He is slain.
    Die,
}

impl Situation {
    pub const ALL: [Situation; 4] = [Situation::Meet, Situation::Win, Situation::Flee, Situation::Die];
}

/// The lines of each nature, by situation (`{hero}` his name, `{title}` what he is, `{me}` the
/// rival's name).
fn lines(p: Personality, s: Situation) -> &'static [&'static str] {
    use Personality as P;
    use Situation as S;
    match (p, s) {
        (P::Braggart, S::Meet) => &[
            "{hero}! They will sing of the day {me} ate {title}.",
            "Look at you, {title}. I expected more.",
            "Kneel, {hero}, and I'll make it quick.",
        ],
        (P::Braggart, S::Win) => {
            &["Tell them who did it, {hero}. Tell them {me}!", "Too easy. Again tomorrow?", "Another notch. {title} was a small one."]
        }
        (P::Braggart, S::Flee) => {
            &["This isn't over. I'm letting you live!", "I'll be back, and bigger!", "Remember my name, {hero}. I will be back for you."]
        }
        (P::Braggart, S::Die) => &["No... they'll laugh at me...", "Lucky... blow...", "Tell them... I nearly had you..."],
        (P::Coward, S::Meet) => {
            &["You again? Stay back, {title}!", "I've friends near. Many friends.", "We needn't fight, {hero}. Need we?"]
        }
        (P::Coward, S::Win) => &["Ha! Ha! I did it! Did you see?", "Is he down? Is he really down?", "I'll take that, thank you, {hero}."],
        (P::Coward, S::Flee) => &["Not today! Not today!", "Run, run, run!", "You'll never catch me, {title}!"],
        (P::Coward, S::Die) => &["Mercy, {hero}! Merc...", "I should have run sooner...", "Not fair... not fair..."],
        (P::Duelist, S::Meet) => {
            &["Stand back, all of you. {hero} is mine.", "Face me alone, {title}, as it should be.", "One blade against one. Come."]
        }
        (P::Duelist, S::Win) => {
            &["A fair fight, {hero}. You lost it.", "Your form was good. Mine was better.", "Rise when you can. I'll wait."]
        }
        (P::Duelist, S::Flee) => {
            &["We'll finish this another day.", "Not like this. Next time, properly.", "Keep your blade sharp, {title}."]
        }
        (P::Duelist, S::Die) => &["Well struck... {hero}.", "A good death. Thank you.", "At last... a worthy blade."],
        (P::Schemer, S::Meet) => {
            &["Welcome, {hero}. You're right on time.", "Go on, {title}, step closer.", "Everything is ready for you."]
        }
        (P::Schemer, S::Win) => {
            &["All according to plan.", "You walked into every one of my snares, {hero}.", "I knew which way you'd turn."]
        }
        (P::Schemer, S::Flee) => {
            &["Fall back! To the next room!", "You've won a room, {title}. Not the hall.", "Sound the horn! Wake them all!"]
        }
        (P::Schemer, S::Die) => &["This... wasn't the plan...", "Clever, {hero}... too clever...", "My horn... where is my horn..."],
        (P::Avenger, S::Meet) => {
            &["My kin told me of you, {hero}.", "I know your face, {title}. I've dreamt of it.", "For my kin. For every one of them."]
        }
        (P::Avenger, S::Win) => &["That was for my kin.", "Now we're even, {title}.", "Rest easy, my kin. It is done."],
        (P::Avenger, S::Flee) => &["I'll have your head yet, {hero}.", "My vengeance can wait a little.", "Not yet... not yet..."],
        (P::Avenger, S::Die) => &["Brother... I'm sorry...", "I came so close...", "Avenge... me..."],
    }
}

/// Capitalises the first letter.
fn capital(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// What he says of his last fight with `hero`, if he remembers one: his lesson first, then how
/// it went.
fn memory_line(r: &Rival, hero: HeroKind) -> Option<String> {
    let last = r.memory.iter().rev().find(|e| e.hero == hero)?;
    let title = names::hero_title(hero);
    let v = r.verbs();
    Some(match r.lesson {
        Some(Lesson::Parry) => format!("You parried me {} times, {}. I have changed my rhythm.", v.parries, hero.name()),
        Some(Lesson::Leap) => format!("Leap again, {title}. I'll be waiting where you land."),
        Some(Lesson::Shield) if v.guard_breaks > 0 => format!("{}'s shield again? It broke last time.", capital(title)),
        Some(Lesson::Shield) => format!("{}'s shield again? I've learned to lean on it.", capital(title)),
        Some(Lesson::Antidote(p)) => format!("Drink your {}, {}. I brought the antidote.", elixir_name(p), hero.name()),
        Some(Lesson::Finisher) => format!("I know how your string ends, {}.", hero.name()),
        None if last.verbs.guard_breaks > 0 => format!("{}'s shield again? It broke last time.", capital(title)),
        None => match last.outcome {
            Outcome::RivalWon => format!("Back for more, {}? You fell easily last time.", hero.name()),
            Outcome::RivalFled => format!("You nearly had me, {title}. Nearly."),
            Outcome::HeroLeft | Outcome::HeroWon => "You walked away from me once. Not again.".to_string(),
        },
    })
}

/**
 * What a rival says: on meeting, half the time what he remembers of the hero (the lesson he took,
 * the shield that broke), else a line of his nature. `roll` comes from the presentation's rng
 * (`PresRng::index`), never the simulation's (rule I5).
 */
pub fn bark(r: &Rival, situation: Situation, hero: HeroKind, roll: usize) -> String {
    if situation == Situation::Meet && roll.is_multiple_of(2) {
        if let Some(line) = memory_line(r, hero) {
            return line;
        }
    }
    let set = lines(r.personality, situation);
    let line = set[(roll / 2) % set.len()];
    line.replace("{hero}", hero.name()).replace("{title}", names::hero_title(hero)).replace("{me}", &r.name)
}

// ------------------------------------------------------------------ the save

/// Characters the save uses as separators, written as %XX inside names.
const RESERVED: &str = "%|;,/.:+~=";

fn esc(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if RESERVED.contains(c) || c.is_control() {
            let mut b = [0u8; 4];
            for byte in c.encode_utf8(&mut b).bytes() {
                out.push_str(&format!("%{byte:02X}"));
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn unesc(s: &str) -> Option<String> {
    let mut bytes = Vec::new();
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = s.get(i + 1..i + 3)?;
            bytes.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            bytes.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(bytes).ok()
}

fn strength_code(s: Strength) -> String {
    match s {
        Strength::Unflinching => "u".into(),
        Strength::Patient => "p".into(),
        Strength::ElixirProof(p) => format!("e{}", index_of(&ELIXIRS, &p)),
        Strength::PackLord => "l".into(),
        Strength::Vengeful => "v".into(),
    }
}

fn strength_of(c: &str) -> Option<Strength> {
    Some(match c {
        "u" => Strength::Unflinching,
        "p" => Strength::Patient,
        "l" => Strength::PackLord,
        "v" => Strength::Vengeful,
        _ => Strength::ElixirProof(*ELIXIRS.get(c.strip_prefix('e')?.parse::<usize>().ok()?)?),
    })
}

fn weakness_code(w: Weakness) -> String {
    match w {
        Weakness::Proud => "p".into(),
        Weakness::Scarred(Side::Left) => "sL".into(),
        Weakness::Scarred(Side::Right) => "sR".into(),
        Weakness::Fears(f) => format!("f{}", index_of(&Family::ALL, &f)),
        Weakness::Greedy => "g".into(),
        Weakness::OldWound => "o".into(),
    }
}

fn weakness_of(c: &str) -> Option<Weakness> {
    Some(match c {
        "p" => Weakness::Proud,
        "sL" => Weakness::Scarred(Side::Left),
        "sR" => Weakness::Scarred(Side::Right),
        "g" => Weakness::Greedy,
        "o" => Weakness::OldWound,
        _ => Weakness::Fears(*Family::ALL.get(c.strip_prefix('f')?.parse::<usize>().ok()?)?),
    })
}

const NATURES: [Personality; 5] =
    [Personality::Braggart, Personality::Coward, Personality::Duelist, Personality::Schemer, Personality::Avenger];

fn lesson_code(l: Option<Lesson>) -> String {
    match l {
        None => "-".into(),
        Some(Lesson::Parry) => "p".into(),
        Some(Lesson::Leap) => "l".into(),
        Some(Lesson::Shield) => "s".into(),
        Some(Lesson::Antidote(p)) => format!("a{}", index_of(&ELIXIRS, &p)),
        Some(Lesson::Finisher) => "f".into(),
    }
}

fn lesson_of(c: &str) -> Option<Option<Lesson>> {
    Some(match c {
        "-" => None,
        "p" => Some(Lesson::Parry),
        "l" => Some(Lesson::Leap),
        "s" => Some(Lesson::Shield),
        "f" => Some(Lesson::Finisher),
        _ => Some(Lesson::Antidote(*ELIXIRS.get(c.strip_prefix('a')?.parse::<usize>().ok()?)?)),
    })
}

fn place_code(p: Place) -> String {
    match p {
        Place::Realm(s) => format!("r{}", s.index()),
        Place::Court(s) => format!("c{}", s.index()),
        Place::Dungeon(n) => format!("d{n}"),
        Place::Ring => "g".into(),
        Place::Hub => "h".into(),
    }
}

fn place_of(c: &str) -> Option<Place> {
    let n = || c.get(1..)?.parse::<usize>().ok();
    Some(match c.chars().next()? {
        'r' => Place::Realm(*SINS.get(n()?)?),
        'c' => Place::Court(*SINS.get(n()?)?),
        'd' => Place::Dungeon(u8::try_from(n()?).ok()?),
        'g' => Place::Ring,
        'h' => Place::Hub,
        _ => return None,
    })
}

const OUTCOMES: [Outcome; 4] = [Outcome::HeroWon, Outcome::RivalWon, Outcome::RivalFled, Outcome::HeroLeft];

fn encounter_code(e: &Encounter) -> String {
    let elixirs: Vec<String> = e.elixirs.iter().map(|p| index_of(&ELIXIRS, p).to_string()).collect();
    let v = &e.verbs;
    [
        place_code(e.place),
        index_of(&OUTCOMES, &e.outcome).to_string(),
        e.hero.slot().to_string(),
        index_of(&Family::ALL, &e.family).to_string(),
        e.tier.to_string(),
        elixirs.join("+"),
        v.parries.to_string(),
        v.blocks.to_string(),
        v.evades.to_string(),
        v.finishers.to_string(),
        v.guard_breaks.to_string(),
        v.wounds.to_string(),
    ]
    .join(".")
}

fn encounter_of(c: &str) -> Option<Encounter> {
    let f: Vec<&str> = c.split('.').collect();
    if f.len() < 12 {
        return None;
    }
    let num = |i: usize| f[i].parse::<u16>().ok();
    let elixirs = if f[5].is_empty() {
        Vec::new()
    } else {
        f[5].split('+').map(|i| i.parse::<usize>().ok().and_then(|i| ELIXIRS.get(i).copied())).collect::<Option<Vec<_>>>()?
    };
    Some(Encounter {
        place: place_of(f[0])?,
        outcome: *OUTCOMES.get(f[1].parse::<usize>().ok()?)?,
        hero: *ROSTER.get(f[2].parse::<usize>().ok()?)?,
        family: *Family::ALL.get(f[3].parse::<usize>().ok()?)?,
        tier: f[4].parse().ok()?,
        elixirs,
        verbs: Verbs { parries: num(6)?, blocks: num(7)?, evades: num(8)?, finishers: num(9)?, guard_breaks: num(10)?, wounds: num(11)? },
    })
}

fn rival_code(r: &Rival) -> String {
    let memory: Vec<String> = r.memory.iter().map(encounter_code).collect();
    [
        r.id.0.to_string(),
        index_of(KINDS, &r.kind).to_string(),
        esc(&r.name),
        esc(&r.epithet),
        r.rank.to_string(),
        index_of(&Deed::ALL, &r.deed).to_string(),
        r.hero.slot().to_string(),
        strength_code(r.strength),
        weakness_code(r.weakness),
        index_of(&NATURES, &r.personality).to_string(),
        lesson_code(r.lesson),
        r.known.to_string(),
        r.silver_held.to_string(),
        index_of(&Trophy::ALL, &r.trophy).to_string(),
        (r.alive as u8).to_string(),
        memory.join("/"),
    ]
    .join(",")
}

fn rival_of(c: &str) -> Option<Rival> {
    let f: Vec<&str> = c.split(',').collect();
    if f.len() < 16 {
        return None;
    }
    let memory = if f[15].is_empty() { Vec::new() } else { f[15].split('/').map(encounter_of).collect::<Option<Vec<_>>>()? };
    Some(Rival {
        id: RivalId(f[0].parse().ok()?),
        kind: *KINDS.get(f[1].parse::<usize>().ok()?)?,
        name: unesc(f[2])?,
        epithet: unesc(f[3])?,
        rank: f[4].parse::<u8>().ok()?.clamp(1, MAX_RANK),
        deed: *Deed::ALL.get(f[5].parse::<usize>().ok()?)?,
        hero: *ROSTER.get(f[6].parse::<usize>().ok()?)?,
        strength: strength_of(f[7])?,
        weakness: weakness_of(f[8])?,
        personality: *NATURES.get(f[9].parse::<usize>().ok()?)?,
        lesson: lesson_of(f[10])?,
        known: f[11].parse().ok()?,
        silver_held: f[12].parse().ok()?,
        trophy: *Trophy::ALL.get(f[13].parse::<usize>().ok()?)?,
        alive: f[14] == "1",
        memory,
    })
}

impl Ledger {
    /// The Ledger as one section of `vh.warband`: "next;slain;rival;rival...", with no '|'.
    pub fn encode(&self) -> String {
        let mut parts = vec![self.next_id.to_string(), self.slain.to_string()];
        parts.extend(self.rivals.iter().map(rival_code));
        parts.join(";")
    }

    /// Reads a Ledger back; a rival it cannot read is left out rather than losing the rest.
    pub fn decode(text: &str) -> Option<Ledger> {
        let mut f = text.split(';');
        let next_id: u32 = f.next()?.parse().ok()?;
        let slain: u32 = f.next()?.parse().ok()?;
        let rivals: Vec<Rival> = f.filter(|s| !s.is_empty()).filter_map(rival_of).collect();
        let next_id = next_id.max(rivals.iter().map(|r| r.id.0 + 1).max().unwrap_or(0));
        Some(Ledger { rivals, next_id, slain })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::combat::fsm::Effect;
    use crate::logic::combat::Breed;
    use crate::logic::types::V;

    fn encounter(outcome: Outcome, verbs: Verbs, elixirs: Vec<PotionKind>) -> Encounter {
        Encounter { place: Place::Realm(Sin::Greed), outcome, hero: HeroKind::Mehmed, family: Family::Steel, tier: 3, elixirs, verbs }
    }

    #[test]
    fn nine_live_rivals_at_most_and_a_tenth_replaces_the_lowest_rank() {
        let mut l = Ledger::default();
        let ids: Vec<RivalId> = (0..9).map(|k| l.promote(KINDS[k * 3], Deed::KillingBlow, HeroKind::Ragnar, 100 + k as u32)).collect();
        for (k, &id) in ids.iter().enumerate() {
            for _ in 0..(1 + k % 4) {
                l.rank_up(id);
            }
        }
        assert_eq!(l.live().count(), 9);
        // The lowest rank is 2 (ranked up once); the oldest of those is the first.
        let lowest = ids[0];
        assert_eq!(l.get(lowest).unwrap().rank, 2);
        let new = l.promote(EnemyKind::Oni, Deed::ThreeGuardBreaks, HeroKind::Ragnar, 7);
        assert_eq!(l.live().count(), 9, "still nine");
        assert!(l.get(lowest).is_none(), "the lowest rank, the oldest of equals, is forgotten");
        assert_eq!(l.get(new).unwrap().rank, 1);
        // Ranks stop at five; names never repeat among the living.
        for _ in 0..10 {
            l.rank_up(new);
        }
        assert_eq!(l.get(new).unwrap().rank, MAX_RANK);
        let mut l2 = Ledger::default();
        for s in 0..9 {
            l2.promote(EnemyKind::Kappa, Deed::SurvivedLow, HeroKind::Miyamoto, s % 2);
        }
        let mut names: Vec<&str> = l2.live().map(|r| r.name.as_str()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 9, "no two living rivals share a name");
    }

    #[test]
    fn promotion_is_deterministic_from_its_seed() {
        let mut a = Ledger::default();
        let mut b = Ledger::default();
        for s in 0..20 {
            a.promote(KINDS[s as usize % KINDS.len()], Deed::ALL[s as usize % 5], HeroKind::Mehmed, s * 31);
            b.promote(KINDS[s as usize % KINDS.len()], Deed::ALL[s as usize % 5], HeroKind::Mehmed, s * 31);
        }
        assert_eq!(a, b);
    }

    #[test]
    fn the_lesson_comes_from_the_heros_most_used_answer() {
        let mut l = Ledger::default();
        let id = l.promote(EnemyKind::Karakoncolos, Deed::ThreeGuardBreaks, HeroKind::Mehmed, 3);
        let parry = Verbs { parries: 6, blocks: 2, evades: 1, ..Verbs::default() };
        l.record(id, encounter(Outcome::RivalFled, parry, vec![]));
        assert_eq!(l.get(id).unwrap().lesson, None, "no lesson below rank 3");
        l.rank_up(id);
        l.rank_up(id);
        assert_eq!(l.get(id).unwrap().lesson, Some(Lesson::Parry));
        // He changes his answer: blocks from now on. The lesson follows, one at a time.
        let block = Verbs { blocks: 9, ..Verbs::default() };
        l.record(id, encounter(Outcome::RivalFled, block, vec![]));
        assert_eq!(l.get(id).unwrap().lesson, Some(Lesson::Shield));
        // The leap, the finisher.
        let mut r = l.get(id).unwrap().clone();
        r.memory = vec![encounter(Outcome::RivalWon, Verbs { evades: 4, parries: 1, ..Verbs::default() }, vec![])];
        assert_eq!(choose_lesson(&r), Some(Lesson::Leap));
        r.memory = vec![encounter(Outcome::RivalWon, Verbs { finishers: 5, evades: 4, ..Verbs::default() }, vec![])];
        assert_eq!(choose_lesson(&r), Some(Lesson::Finisher));
        // An elixir drunk against him in two fights outweighs a few answers.
        r.memory = vec![
            encounter(Outcome::RivalFled, Verbs { blocks: 3, ..Verbs::default() }, vec![PotionKind::Fire]),
            encounter(Outcome::RivalFled, Verbs { blocks: 2, ..Verbs::default() }, vec![PotionKind::Fire, PotionKind::Health]),
        ];
        assert_eq!(choose_lesson(&r), Some(Lesson::Antidote(PotionKind::Fire)));
        // Health is no elixir a foe can answer.
        r.memory = vec![encounter(Outcome::RivalFled, Verbs::default(), vec![PotionKind::Health]); 3];
        assert_eq!(choose_lesson(&r), None, "too few answers for a lesson");
    }

    #[test]
    fn every_rivals_lesson_changes_choices_not_frame_data() {
        let mut lessons = vec![Lesson::Parry, Lesson::Leap, Lesson::Shield, Lesson::Finisher];
        lessons.extend(ELIXIRS.map(Lesson::Antidote));
        let mut l = Ledger::default();
        for (k, &kind) in KINDS.iter().enumerate() {
            let breed = Breed::of(kind);
            let moves = breed.moves();
            for lesson in &lessons {
                for &mv in moves {
                    let d = mv.data();
                    if let Some((a, b)) = lesson.startups(mv) {
                        // Both startups at least the move's own and the floors; far enough apart
                        // that a parry by rhythm misses one; bounded.
                        assert!(a >= d.startup && b >= d.startup, "{kind:?} {}: {a}/{b} under {}", d.name, d.startup);
                        assert!(a >= FOE_FLOOR && b >= FOE_FLOOR && a >= MELEE_FLOOR, "{}", d.name);
                        assert!(b - a >= 8 && b <= d.startup.max(FOE_FLOOR) + 16, "{}: {a}/{b}", d.name);
                        assert_eq!(lesson.hold(mv, false) + d.startup, a);
                        assert_eq!(lesson.hold(mv, true) + d.startup, b);
                    } else {
                        assert_eq!(lesson.hold(mv, true), 0);
                    }
                }
                // A preferred move is always one of his breed's own.
                if let Some(mv) = lesson.prefer(moves) {
                    assert!(moves.contains(&mv), "{kind:?}: {lesson:?} prefers {mv:?}, not his");
                }
            }
            // Dressed as a rival with any lesson and any traits, every move he knows is the move
            // the table holds: startup, active, recovery, damage, all of it.
            for seed in 0..12u32 {
                let id = l.promote(kind, Deed::ALL[seed as usize % 5], HeroKind::Ragnar, seed * 7 + k as u32);
                let mut r = l.get(id).unwrap().clone();
                r.rank = MAX_RANK;
                r.lesson = Some(lessons[seed as usize % lessons.len()]);
                let mut du = Duelist::bred(1, breed, V::new(0.0, 0.0), V::new(1.0, 0.0));
                let before: Vec<_> = moves.iter().map(|&m| du.data(m)).collect();
                r.dress(&mut du);
                let after: Vec<_> = moves.iter().map(|&m| du.data(m)).collect();
                assert_eq!(before, after, "{kind:?}: a rival's traits touched a move's frames");
                for &m in moves {
                    // Patience never tracks into the last 12 frames.
                    let d = m.data();
                    let bonus = r.track_bonus(m);
                    assert!(bonus <= PATIENT_TRACK && (bonus == 0 || d.track + bonus + COMMIT_FOR <= d.startup), "{}", d.name);
                }
                // The numbers stay bounded.
                assert!(r.health_scale() <= 1.0 + HEALTH_PER_RANK * MAX_RANK as f64 + 1e-9);
                assert!(r.armor_bonus() <= UNFLINCHING_ARMOR);
                for p in ELIXIRS {
                    assert!(r.elixir_scale(p) >= ELIXIR_PROOF);
                }
            }
        }
        // The lessons' own numbers: the leap's wait, the shield's wear, the antidote.
        assert_eq!(Lesson::Leap.waits_after_leap(), LEAP_LANDING);
        assert_eq!(Lesson::Shield.shield_wear(), SHIELD_WEAR);
        assert_eq!(Lesson::Antidote(PotionKind::Fire).elixir_scale(PotionKind::Fire), 0.5);
        assert_eq!(Lesson::Antidote(PotionKind::Fire).elixir_scale(PotionKind::Fury), 1.0);
        assert!(Lesson::Finisher.evades_finisher() && !Lesson::Parry.evades_finisher());
    }

    #[test]
    fn the_old_man_never_lies() {
        let all = Trait::all();
        // No trait's words turn up inside another's.
        for a in &all {
            for b in all.iter().filter(|b| *b != a) {
                assert!(!a.keyword().contains(&b.keyword()), "{a:?} / {b:?}");
            }
        }
        let mut l = Ledger::default();
        for seed in 0..200u32 {
            let kind = KINDS[seed as usize % KINDS.len()];
            let id = l.promote(kind, Deed::ALL[seed as usize % 5], ROSTER[seed as usize % 3], seed.wrapping_mul(2_654_435_761));
            // Some found, some not.
            l.get_mut(id).unwrap().known = (seed % 8) as u8;
            let r = l.get(id).unwrap();
            let line = counsel_line_for(r);
            let truths = r.true_traits();
            let named: Vec<&Trait> = all.iter().filter(|t| line.contains(&t.keyword())).collect();
            assert_eq!(named.len(), 1, "{line}: names {named:?}");
            assert!(truths.contains(named[0]), "the old man lied about {}: {line}", r.name);
            // What the hero has found comes first, the weakness when nothing is.
            if r.known == 0 {
                assert_eq!(*named[0], truths[1]);
            } else {
                assert!(r.knows(*named[0]), "{line}");
            }
            assert!(names::drawable(&line), "{line}");
        }
    }

    #[test]
    fn a_rival_remembers_and_says_so() {
        let mut l = Ledger::default();
        let id = l.promote(EnemyKind::Karakoncolos, Deed::KillingBlow, HeroKind::Mehmed, 11);
        assert_eq!(l.get(id).unwrap().card().lesson, None);
        let broke = Verbs { blocks: 7, guard_breaks: 2, ..Verbs::default() };
        l.after_fight(id, encounter(Outcome::RivalWon, broke, vec![]));
        l.after_fight(id, encounter(Outcome::RivalFled, broke, vec![]));
        let r = l.get(id).unwrap();
        assert_eq!(r.rank, 3);
        assert_eq!(r.lesson, Some(Lesson::Shield));
        let card = r.card();
        assert_eq!(card.lesson.as_deref(), Some("He remembers your shield."), "the lesson is announced on the card");
        // And in his mouth.
        assert_eq!(bark(r, Situation::Meet, HeroKind::Mehmed, 0), "The janissary's shield again? It broke last time.");
        // Every lesson has its line, in her voice for a she.
        let mut s = r.clone();
        s.kind = EnemyKind::Albasti;
        s.lesson = Some(Lesson::Parry);
        assert_eq!(s.card().lesson.as_deref(), Some("She remembers your parry."));
        // Unknown traits are "?" until found.
        assert!(card.traits.iter().all(|t| t.1.is_none()));
        l.reveal(id, Trait::Weakness(r.weakness));
        let card = l.get(id).unwrap().card();
        assert_eq!(card.traits[1].1, Some(Trait::Weakness(l.get(id).unwrap().weakness).name()));
        assert!(card.traits[0].1.is_none() && card.traits[2].1.is_none());
    }

    #[test]
    fn barks_have_three_lines_for_every_nature_and_moment() {
        let mut l = Ledger::default();
        for (k, p) in NATURES.iter().enumerate() {
            for s in Situation::ALL {
                let set = lines(*p, s);
                assert!(set.len() >= 3, "{p:?} {s:?}");
            }
            let id = l.promote(KINDS[k], Deed::KillingBlow, HeroKind::Ragnar, k as u32);
            let r = l.get_mut(id).unwrap();
            r.personality = *p;
            let r = r.clone();
            for s in Situation::ALL {
                let said: Vec<String> = (0..6).map(|roll| bark(&r, s, HeroKind::Ragnar, roll)).collect();
                for line in &said {
                    assert!(!line.contains('{') && names::drawable(line), "{line}");
                }
                let mut distinct = said.clone();
                distinct.sort();
                distinct.dedup();
                assert!(distinct.len() >= 3, "{p:?} {s:?}: {said:?}");
            }
        }
    }

    #[test]
    fn the_tracker_folds_a_fight_into_answers_and_deeds() {
        let dir = V::new(1.0, 0.0);
        let landed =
            |how: Defense, dmg: f64| WarEvent::Landed { foe: 4, kind: Some(EnemyKind::Oni), how, dmg, dir, lands: true, bolt: false };
        let hit = |finisher: bool| WarEvent::HeroHit {
            foe: 4,
            kind: EnemyKind::Oni,
            effect: Effect::Flinched(10),
            dmg: 10.0,
            finisher,
            dir,
            at: dir,
            warded: 0.0,
        };
        let mut t = EncounterTracker::new(4, EnemyKind::Oni, None);
        // Another foe's blows are not his.
        t.observe(&WarEvent::Landed { foe: 9, kind: None, how: Defense::Parried, dmg: 0.0, dir, lands: true, bolt: false }, 80.0, 1.0);
        assert!(!t.met);
        for _ in 0..3 {
            t.observe(&landed(Defense::GuardBroken, 0.0), 80.0, 1.0);
        }
        t.observe(&landed(Defense::Parried, 0.0), 80.0, 1.0);
        t.observe(&WarEvent::Drank { potion: PotionKind::Fury }, 80.0, 1.0);
        t.observe(&WarEvent::Drank { potion: PotionKind::Fury }, 80.0, 1.0);
        t.observe(&hit(false), 80.0, 0.6);
        t.observe(&hit(true), 80.0, 0.2);
        assert_eq!(t.verbs, Verbs { parries: 1, guard_breaks: 3, wounds: 2, finishers: 1, ..Verbs::default() });
        assert_eq!(t.elixirs, vec![PotionKind::Fury]);
        assert_eq!(t.deed(0, false, false), Some(Deed::ThreeGuardBreaks));
        // Low, and he walked off: it survived.
        let mut low = t.clone();
        low.verbs.guard_breaks = 0;
        assert_eq!(low.deed(0, false, true), Some(Deed::SurvivedLow));
        assert_eq!(low.outcome(false), Outcome::RivalFled);
        // The killing blow; with his silver, the silver.
        t.observe(&landed(Defense::Open, 30.0), 0.0, 0.2);
        assert!(t.killed_hero);
        assert_eq!(t.deed(20, false, false), Some(Deed::KillingBlow));
        assert_eq!(t.deed(SILVER_DEED, false, false), Some(Deed::TookSilver));
        assert_eq!(t.outcome(false), Outcome::RivalWon);
        // A leader left behind, whole.
        let mut whole = EncounterTracker::new(5, EnemyKind::Oni, None);
        whole.observe(&WarEvent::Tell { foe: 5, kind: EnemyKind::Oni, mv: MoveId::Kanabo, at: dir }, 80.0, 1.0);
        assert_eq!(whole.deed(0, true, true), Some(Deed::LeftBehind));
        assert_eq!(whole.deed(0, false, true), None);
        // Slain: no deed, and the hero won.
        whole.observe(&WarEvent::Slain { foe: 5, kind: EnemyKind::Oni, at: dir, dir, last_of_pack: true }, 80.0, 0.0);
        assert_eq!((whole.deed(0, true, true), whole.outcome(true)), (None, Outcome::HeroWon));
        let e = t.encounter(Place::Court(Sin::Greed), HeroKind::Ragnar, 2, false);
        assert_eq!((e.family, e.outcome), (Family::Thrown, Outcome::RivalWon), "Ragnar's second tier is thrown");
    }

    #[test]
    fn offers_slayings_and_the_usurper() {
        let mut l = Ledger::default();
        assert!(l.offer(1, 1).is_empty());
        let a = l.promote(EnemyKind::Oni, Deed::KillingBlow, HeroKind::Ragnar, 1);
        let b = l.promote(EnemyKind::Kappa, Deed::KillingBlow, HeroKind::Ragnar, 2);
        let c = l.promote(EnemyKind::Cin, Deed::KillingBlow, HeroKind::Ragnar, 3);
        for _ in 0..4 {
            l.rank_up(b);
        }
        assert_eq!(l.offer(1, 9).len(), 1);
        assert_eq!(l.offer(5, 9).len(), 2, "two from tier 5");
        assert_eq!(l.offer(3, 9), l.offer(3, 9), "the same visit, the same rivals");
        assert_eq!(l.usurper(), Some(b));
        l.took_silver(b, 200);
        let reward = l.slain(b).unwrap();
        assert_eq!(reward, Reward { runes: 5, silver: 300, trophy: Trophy::Dish });
        assert!(l.slain(b).is_none(), "slain once");
        assert_eq!(l.usurper(), None);
        // An avenger of a slain rival's breed comes first.
        let d = l.promote(EnemyKind::Kappa, Deed::SurvivedLow, HeroKind::Ragnar, 4);
        l.get_mut(d).unwrap().personality = Personality::Avenger;
        assert!(l.brother_slain(EnemyKind::Kappa));
        for seed in 0..10 {
            assert_eq!(l.offer(1, seed), vec![d]);
        }
        assert!(l.offer(5, 0).contains(&a) || l.offer(5, 0).contains(&c));
    }

    #[test]
    fn a_ledger_survives_the_save() {
        let mut l = Ledger::default();
        let a = l.promote(EnemyKind::Karakoncolos, Deed::ThreeGuardBreaks, HeroKind::Mehmed, 5);
        let b = l.promote(EnemyKind::Goryo, Deed::TookSilver, HeroKind::Miyamoto, 6);
        l.promote(EnemyKind::Witch, Deed::LeftBehind, HeroKind::Ragnar, 8);
        // Turkish letters, and the save's own separators, in a name.
        l.get_mut(a).unwrap().name = "Kızıl Börü; the, 50% / İnci.Çakır|Şş:Ü~ö=+".to_string();
        l.get_mut(b).unwrap().epithet = "Who Took Miyamoto's Silver".to_string();
        l.after_fight(
            a,
            encounter(
                Outcome::RivalWon,
                Verbs { parries: 3, blocks: 4, evades: 1, finishers: 2, guard_breaks: 3, wounds: 9 },
                vec![PotionKind::Fire, PotionKind::Ragnarok],
            ),
        );
        let mut e = encounter(Outcome::HeroLeft, Verbs::default(), vec![]);
        e.place = Place::Dungeon(4);
        l.after_fight(a, e);
        l.after_fight(a, encounter(Outcome::RivalFled, Verbs { evades: 7, ..Verbs::default() }, vec![]));
        l.reveal(a, Trait::Personality(Personality::Coward));
        l.took_silver(b, 77);
        l.slain(b);
        let text = l.encode();
        assert!(!text.contains('|') && !text.contains('\n'), "{text}");
        assert_eq!(Ledger::decode(&text), Some(l.clone()));
        assert_eq!(Ledger::decode(&Ledger::default().encode()), Some(Ledger::default()));
        // A rival that cannot be read is left out; the rest load.
        let broken = text.replacen(";0,", ";0,x", 1);
        let back = Ledger::decode(&broken).unwrap();
        assert_eq!(back.rivals.len(), l.rivals.len() - 1);
        assert_eq!(Ledger::decode("rubbish"), None);
    }
}
