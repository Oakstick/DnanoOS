//! The heroes' mastery of technique (WARBAND.md §11.1): beside the path the old man sells, a
//! mastery each hero earns, by deeds in fights and by the old man's trials, because mastery that
//! is bought is not mastery.
//!
//! * **Ragnar, the Berserker's Way**: punishes, poise breaks and finishers.
//! * **Mehmed, the Discipline of the Ocak**: blocks, and blows that land with the last 0.3 tiles
//!   of his reach (the halberd's tip).
//! * **Miyamoto, the Way of Two Heavens**: parries, and leaps a blow passes under.
//!
//! Five ranks each. A rank wants its deeds (`DEEDS_FOR`) **and** its trial: a short bout in the
//! Sparring Ring's rules, set by the old man in the training yard (`TRIALS`). What each rank does is
//! a number on the kit (`Kit::with_mastery`, from a `Stance`) or a bounded rule the game keeps
//! (`game::mastery`): no rank touches a foe's frame data, and rank 0 changes nothing.

use super::super::combat::kit::Kit;
use crate::logic::heroes::HeroKind;

/// Ranks of each mastery.
pub const RANKS: u8 = 5;
/// The deeds (of his way, all kinds together) each rank wants before its trial is set.
pub const DEEDS_FOR: [u32; RANKS as usize] = [25, 75, 150, 300, 500];

// ---- what the ranks do, as numbers

/// Ragnar 2: the finisher's poise damage, added.
pub const FINISHER_POISE: f64 = 5.0;
/// Ragnar 3: stamina a kill gives back while time is slowed.
pub const DILATION_REFUND: f64 = 16.0;
/// Ragnar 5 and Mehmed 5: once a pack, a guard break leaves him guarded.
pub const UNBROKEN_RANK: u8 = 5;
/// Mehmed 1: what a block costs, as a share of the shield's cost.
pub const BLOCK_COST: f64 = 0.9;
/// Mehmed 2: the tip of his reach (tiles), and the poise a blow there takes on top.
pub const TIP: f64 = 0.3;
pub const TIP_POISE: f64 = 0.25;
/// Mehmed 3, Hold the Line: a body foe whose blow he blocks is shoved this many tiles back.
pub const HOLD_THE_LINE: f64 = 0.5;
/// Mehmed 4: blocks in a row that fill the discipline; full, his next chain costs no stamina.
pub const DISCIPLINE: u8 = 5;
/// Miyamoto 2, Riposte: a swing begun this many frames after a parry lands this much harder.
pub const RIPOSTE_FRAMES: u32 = 20;
pub const RIPOSTE: f64 = 1.15;
/// Miyamoto 3, Mushin: parries without a wound, and the next swing lands this much harder.
pub const MUSHIN_PARRIES: u8 = 3;
pub const MUSHIN: f64 = 1.3;
/// Miyamoto 4: stamina a leap gives back when a blow passes under it.
pub const EVADE_REFUND: f64 = 10.0;
/// Miyamoto 5: frames added to the parry window.
pub const PARRY_PLUS: u16 = 1;

/// Ragnar 4, the War Cry: a move of his own, in frames (COMBAT.md §0: startup, active, recovery;
/// a blow in its startup ends it, and its recovery is open). On its active frames every body foe
/// within `radius` of him (centre to edge) is staggered, as a broken poise staggers it. It costs
/// stamina, and rests between cries.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cry {
    pub startup: u16,
    pub active: u16,
    pub recovery: u16,
    pub radius: f64,
    pub stamina: f64,
    /// Frames from one cry's start before the next may begin.
    pub rest: u32,
}

impl Cry {
    pub const fn total(&self) -> u16 {
        self.startup + self.active + self.recovery
    }
}

pub const WAR_CRY: Cry = Cry { startup: 30, active: 2, recovery: 30, radius: 2.5, stamina: 24.0, rest: 360 };

/// A kind of deed a mastery counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Feat {
    /// A blow into a foe's opening: its recovery, or its reel from a parry (the first of each).
    Punish,
    /// A blow that breaks a foe's poise.
    Break,
    /// His finisher landing.
    Finisher,
    /// A blow caught on the guard after the parry window.
    Block,
    /// A blow landed with the last `TIP` tiles of his reach.
    Tip,
    Parry,
    /// A leap a blow passed under.
    Evade,
}

impl Feat {
    pub fn name(self) -> &'static str {
        match self {
            Feat::Punish => "PUNISHES",
            Feat::Break => "POISE BREAKS",
            Feat::Finisher => "FINISHERS",
            Feat::Block => "BLOCKS",
            Feat::Tip => "TIP HITS",
            Feat::Parry => "PARRIES",
            Feat::Evade => "BLOWS PASSED UNDER",
        }
    }
}

/// A hero's mastery: its name, the deeds it counts, and what each rank gives.
#[derive(Clone, Copy, Debug)]
pub struct Way {
    pub name: &'static str,
    /// Counted in this order (`Mastery::deeds`).
    pub feats: &'static [Feat],
    pub ranks: [&'static str; RANKS as usize],
}

const BERSERKER: Way = Way {
    name: "THE BERSERKER'S WAY",
    feats: &[Feat::Punish, Feat::Break, Feat::Finisher],
    ranks: [
        "A gold glint when a foe's blow is spent: punish it",
        "The finisher breaks harder (+5 poise)",
        "A kill while time is slowed gives back 16 stamina",
        "WAR CRY (F / L3): body foes about him stagger",
        "Once a pack, a broken guard leaves him guarded",
    ],
};

const OCAK: Way = Way {
    name: "THE DISCIPLINE OF THE OCAK",
    feats: &[Feat::Block, Feat::Tip],
    ranks: [
        "A block costs 10% less",
        "A blow at the tip of his reach breaks 25% harder",
        "HOLD THE LINE: a blocked body foe is shoved back half a tile",
        "Five blocks in a row: his next chain costs no stamina",
        "Once a pack, a broken guard leaves him guarded",
    ],
};

const TWO_HEAVENS: Way = Way {
    name: "THE WAY OF TWO HEAVENS",
    feats: &[Feat::Parry, Feat::Evade],
    ranks: [
        "The parry's moment shows as a white edge on the telegraph",
        "RIPOSTE: the swing begun within 20 frames of a parry lands x1.15",
        "MUSHIN: three parries without a wound, the next swing x1.3",
        "A leap a blow passes under gives back 10 stamina",
        "The parry window grows a frame",
    ],
};

/// `hero`'s way.
pub fn way(hero: HeroKind) -> &'static Way {
    match hero {
        HeroKind::Ragnar => &BERSERKER,
        HeroKind::Mehmed => &OCAK,
        HeroKind::Miyamoto => &TWO_HEAVENS,
    }
}

/// One hero's mastery, kept across runs (`HeroProgress`): his rank (his trials passed, in
/// order), his deeds of each kind his way counts, and each trial's tries and best count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mastery {
    pub rank: u8,
    pub deeds: [u32; 3],
    pub tries: [u8; RANKS as usize],
    pub best: [u8; RANKS as usize],
}

impl Mastery {
    /// His deeds of every kind together.
    pub fn total(&self) -> u32 {
        self.deeds.iter().sum()
    }

    /// The deeds of `feat` (0 if his way does not count it).
    pub fn of(&self, hero: HeroKind, feat: Feat) -> u32 {
        way(hero).feats.iter().position(|&f| f == feat).map(|i| self.deeds[i]).unwrap_or(0)
    }

    /// A deed done. True when it is the one that meets the next rank's deeds (the old man would
    /// test him now).
    pub fn count(&mut self, hero: HeroKind, feat: Feat) -> bool {
        let Some(i) = way(hero).feats.iter().position(|&f| f == feat) else { return false };
        let before = self.ready();
        self.deeds[i] = self.deeds[i].saturating_add(1);
        before.is_none() && self.ready().is_some()
    }

    /// The deeds the next rank wants, and how many he has: none once he is a master.
    pub fn progress(&self) -> Option<(u32, u32)> {
        (self.rank < RANKS).then(|| (self.total(), DEEDS_FOR[self.rank as usize]))
    }

    /// The rank whose trial is set (its deeds are met), if any.
    pub fn ready(&self) -> Option<u8> {
        self.progress().filter(|(have, need)| have >= need).map(|_| self.rank + 1)
    }

    /// A trial of `rank` fought, with `count` deeds of its kind: tried, its best kept, and the
    /// rank granted on a pass (only the trial that is set can grant it).
    pub fn tried(&mut self, rank: u8, count: u8, passed: bool) -> bool {
        let k = (rank.clamp(1, RANKS) - 1) as usize;
        self.tries[k] = self.tries[k].saturating_add(1);
        self.best[k] = self.best[k].max(count);
        let grant = passed && self.ready() == Some(rank);
        if grant {
            self.rank = rank;
        }
        grant
    }

    /// Fourteen numbers for the save: rank, the deeds, the tries, the best counts.
    pub fn encode(&self) -> String {
        let mut out = format!("{} {} {} {}", self.rank, self.deeds[0], self.deeds[1], self.deeds[2]);
        for v in self.tries.iter().chain(self.best.iter()) {
            out.push_str(&format!(" {v}"));
        }
        out
    }

    pub fn decode(n: &[u32]) -> Option<Mastery> {
        if n.len() != 4 + 2 * RANKS as usize {
            return None;
        }
        let r = RANKS as usize;
        Some(Mastery {
            rank: (n[0] as u8).min(RANKS),
            deeds: [n[1], n[2], n[3]],
            tries: std::array::from_fn(|k| n[4 + k].min(255) as u8),
            best: std::array::from_fn(|k| n[4 + r + k].min(255) as u8),
        })
    }
}

/// Numbers in `Mastery::encode`.
pub const SAVED_NUMBERS: usize = 4 + 2 * RANKS as usize;

// ---- in the fight

/// What his mastery makes of his kit this frame: his rank, and what his temper has charged
/// (a riposte's swing, Mushin's swing, a free chain).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stance {
    pub hero: HeroKind,
    pub rank: u8,
    pub riposte: bool,
    pub mushin: bool,
    pub free_chain: bool,
}

impl Stance {
    /// No mastery at all.
    pub fn none(hero: HeroKind) -> Self {
        Stance { hero, rank: 0, riposte: false, mushin: false, free_chain: false }
    }
}

impl Kit {
    /// The kit as his mastery has it. Rank 0 (and nothing charged) is the kit itself.
    pub fn with_mastery(mut self, s: &Stance) -> Kit {
        match s.hero {
            HeroKind::Ragnar => {
                if s.rank >= 2 {
                    self.strike.finisher_poise += FINISHER_POISE;
                }
            }
            HeroKind::Mehmed => {
                if s.rank >= 1 {
                    self.guard.cost *= BLOCK_COST;
                }
                if s.rank >= 4 && s.free_chain {
                    self.strike.stamina = 0.0;
                }
            }
            HeroKind::Miyamoto => {
                if s.rank >= 2 && s.riposte {
                    self.strike.power *= RIPOSTE;
                }
                if s.rank >= 3 && s.mushin {
                    self.strike.power *= MUSHIN;
                }
                if s.rank >= 5 {
                    self.guard.parry += PARRY_PLUS;
                }
            }
        }
        self
    }
}

/// What his mastery keeps count of within a fight (frames on the brawl's clock): the last parry
/// (Riposte), the parries since his last wound (Mushin), the blocks in a row (discipline), what
/// the swing under way was charged with, and the once-a-pack unbroken guard.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Temper {
    pub parried_at: Option<u32>,
    pub calm: u8,
    pub discipline: u8,
    /// The swing under way began inside a riposte's window / was Mushin's.
    pub riposte: bool,
    pub mushin: bool,
    /// The chain under way costs nothing.
    pub free: bool,
    pub unbroken_spent: bool,
}

impl Temper {
    /// A blow parried on frame `frame`.
    pub fn parry(&mut self, frame: u32) {
        self.parried_at = Some(frame);
        self.calm = (self.calm + 1).min(MUSHIN_PARRIES);
    }

    /// A blow blocked.
    pub fn block(&mut self) {
        self.discipline = (self.discipline + 1).min(DISCIPLINE);
    }

    /// A blow got through (open, or through a broken guard): the calm and the discipline go.
    pub fn wound(&mut self) {
        self.calm = 0;
        self.discipline = 0;
    }

    /// His guard broke: the discipline goes (and the calm, as a wound).
    pub fn broken(&mut self) {
        self.wound();
    }

    /// A swing began on frame `frame`: what it is charged with. (The riposte is the one swing.)
    pub fn swing(&mut self, frame: u32) {
        self.riposte = self.parried_at.is_some_and(|p| frame.saturating_sub(p) <= RIPOSTE_FRAMES);
        if self.riposte {
            self.parried_at = None;
        }
        self.mushin = self.calm >= MUSHIN_PARRIES;
        if self.mushin {
            self.calm = 0;
        }
        if self.discipline >= DISCIPLINE {
            self.free = true;
            self.discipline = 0;
        }
    }

    /// His chain is over (no swing for a chain's grace): nothing charged carries on.
    pub fn rest(&mut self) {
        self.riposte = false;
        self.mushin = false;
        self.free = false;
    }

    /// The stance for his kit: the swing's charges, and a full discipline already makes the chain
    /// free from its first swing (it is paid for on the frame it begins).
    pub fn stance(&self, hero: HeroKind, rank: u8) -> Stance {
        Stance { hero, rank, riposte: self.riposte, mushin: self.mushin, free_chain: self.free || self.discipline >= DISCIPLINE }
    }
}

// ---- the old man's trials

/// What a trial forbids: nothing, a broken guard, or any wound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    Free,
    NoGuardBreak,
    NoWound,
}

impl Rule {
    pub fn name(self) -> &'static str {
        match self {
            Rule::Free => "",
            Rule::NoGuardBreak => "NO GUARD BROKEN",
            Rule::NoWound => "NO WOUND",
        }
    }
}

/// A trial of a rank: a bout of the Sparring Ring (`gym::BOUTS`), the deed it asks for and how
/// many, what it forbids, and the seconds it allows. The bout ends when the deeds are done; all
/// foes down before that is a failure (the slain teach nothing).
#[derive(Clone, Copy, Debug)]
pub struct Trial {
    pub hero: HeroKind,
    pub rank: u8,
    pub name: &'static str,
    pub bout: usize,
    pub feat: Feat,
    pub count: u8,
    pub rule: Rule,
    pub limit: f64,
    /// What the old man says of it.
    pub line: &'static str,
}

/// The seconds a trial allows.
pub const TRIAL_LIMIT: f64 = 90.0;

const fn t(hero: HeroKind, rank: u8, name: &'static str, bout: usize, feat: Feat, count: u8, rule: Rule, line: &'static str) -> Trial {
    Trial { hero, rank, name, bout, feat, count, rule, limit: TRIAL_LIMIT, line }
}

use Feat as F;
use HeroKind::{Mehmed as Me, Miyamoto as Mi, Ragnar as Ra};

/// Every trial: five for each hero, rank by rank.
pub const TRIALS: [Trial; 15] = [
    t(Ra, 1, "THREE SLAMS PUNISHED", 0, F::Punish, 3, Rule::Free, "Let the brute swing. When the blow is spent, it is yours. Three times."),
    t(Ra, 2, "THE BRUTE BROKEN TWICE", 0, F::Break, 2, Rule::Free, "Break him, let him stand, and break him again."),
    t(Ra, 3, "FIVE FINISHERS", 3, F::Finisher, 5, Rule::Free, "Finish what you start. Five times, against three of them."),
    t(Ra, 4, "SIX PUNISHES IN THE GAUNTLET", 4, F::Punish, 6, Rule::Free, "Five of them, and every one leaves an opening. Take six."),
    t(Ra, 5, "THE GAUNTLET, UNBROKEN", 4, F::Break, 4, Rule::NoGuardBreak, "Break four of them, and never let them break you."),
    t(Me, 1, "TEN BLOWS BLOCKED", 1, F::Block, 10, Rule::NoGuardBreak, "Ten bites on the shield, and the shield holds. Mind your breath."),
    t(Me, 2, "SIX BLOWS AT THE TIP", 0, F::Tip, 6, Rule::Free, "The halberd's strength is at its end. Strike him with the tip, six times."),
    t(
        Me,
        3,
        "FIFTEEN BLOCKED, NONE BROKEN",
        3,
        F::Block,
        15,
        Rule::NoGuardBreak,
        "Three at once. Fifteen blows held, and not one through.",
    ),
    t(Me, 4, "TEN AT THE TIP", 2, F::Tip, 10, Rule::Free, "Keep the wolf at your weapon's length, and the witch in its reach. Ten."),
    t(Me, 5, "THE LINE HOLDS", 4, F::Block, 20, Rule::NoGuardBreak, "Five of them. Twenty blows, and the line holds."),
    t(Mi, 1, "THREE BLOWS PARRIED", 0, F::Parry, 3, Rule::Free, "Meet the slam at the last moment, not before. Three times."),
    t(Mi, 2, "FIVE BITES PARRIED", 1, F::Parry, 5, Rule::Free, "The wolves are quick. Be quicker. Five bites turned."),
    t(Mi, 3, "FIVE BLOWS PASSED UNDER", 3, F::Evade, 5, Rule::Free, "Be where the blow is not. Leap five of them."),
    t(Mi, 4, "EIGHT PARRIED, NO WOUND", 2, F::Parry, 8, Rule::NoWound, "Bites and hexes alike. Turn eight, and take no wound."),
    t(Mi, 5, "TWELVE IN THE GAUNTLET", 4, F::Parry, 12, Rule::NoGuardBreak, "No mind, no fear. Twelve parries among five of them."),
];

/// `hero`'s trial of `rank` (1 to 5).
pub fn trial(hero: HeroKind, rank: u8) -> &'static Trial {
    TRIALS.iter().find(|t| t.hero == hero && t.rank == rank.clamp(1, RANKS)).expect("five trials a hero")
}

/// How a trial went: passed (and whether it gave him the rank), the deeds done of those asked,
/// and why it failed if it did.
#[derive(Clone, Debug, PartialEq)]
pub struct TrialResult {
    pub hero: HeroKind,
    pub rank: u8,
    pub passed: bool,
    pub granted: bool,
    pub count: u8,
    pub of: u8,
    pub secs: f64,
    pub why: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::combat::fsm::PARRY_FROM;
    use crate::logic::gym;
    use crate::logic::heroes::ROSTER;
    use crate::logic::progression;

    fn ready_for(m: &mut Mastery, hero: HeroKind) {
        let feat = way(hero).feats[0];
        while m.ready().is_none() {
            m.count(hero, feat);
        }
    }

    #[test]
    fn a_rank_wants_its_deeds_and_its_trial() {
        let hero = HeroKind::Miyamoto;
        let mut m = Mastery::default();
        assert!(!m.count(hero, Feat::Punish), "not his way's deed");
        assert_eq!(m.total(), 0);
        for k in 0..DEEDS_FOR[0] {
            let met = m.count(hero, if k % 2 == 0 { Feat::Parry } else { Feat::Evade });
            assert_eq!(met, k + 1 == DEEDS_FOR[0], "the deed that meets the rank says so, once");
        }
        assert_eq!(m.ready(), Some(1));
        assert!(!m.tried(1, 2, false) && m.rank == 0, "a failed trial grants nothing");
        assert!(!m.tried(2, 9, true) && m.rank == 0, "only the trial that is set");
        assert!(m.tried(1, 3, true) && m.rank == 1);
        assert_eq!((m.tries[0], m.best[0]), (2, 3));
        assert_eq!(m.ready(), None, "rank 2 wants more deeds");
        for r in 2..=RANKS {
            ready_for(&mut m, hero);
            assert!(m.tried(r, 1, true));
        }
        assert_eq!((m.rank, m.progress(), m.ready()), (RANKS, None, None), "a master");
    }

    #[test]
    fn a_mastery_goes_into_the_save_and_back() {
        let m = Mastery { rank: 3, deeds: [12, 400, 7], tries: [1, 2, 3, 0, 0], best: [3, 5, 4, 0, 0] };
        let n: Vec<u32> = m.encode().split_whitespace().map(|v| v.parse().unwrap()).collect();
        assert_eq!(n.len(), SAVED_NUMBERS);
        assert_eq!(Mastery::decode(&n), Some(m));
        assert_eq!(Mastery::decode(&n[1..]), None);
    }

    #[test]
    fn rank_zero_changes_nothing_and_every_rank_stays_inside_the_tables_bounds() {
        assert_eq!(Kit::of(HeroKind::Ragnar, 1, 1.0, Some(1), 0).with_mastery(&Stance::none(HeroKind::Ragnar)), Kit::training());
        for hero in ROSTER {
            for tier in 1..=progression::TIERS {
                let kit = Kit::of(hero, tier, 1.0, Some(tier), tier);
                // Nothing charged at rank 0 is the kit itself, whatever the temper holds.
                let charged = Stance { hero, rank: 0, riposte: true, mushin: true, free_chain: true };
                assert_eq!(kit.with_mastery(&charged), kit, "{hero:?} {tier}");
                for rank in 1..=RANKS {
                    let k = kit.with_mastery(&Stance { rank, ..charged });
                    assert!(k.guard.parry_to() < 15 && k.guard.parry_to() >= PARRY_FROM, "{hero:?} {tier} {rank}");
                    assert!(k.guard.cost > 0.0, "no free block");
                    for (d, base) in k.moves().iter().zip(kit.moves()) {
                        assert_eq!((d.startup, d.active, d.recovery), (base.startup, base.active, base.recovery), "frames untouched");
                        if !d.projectile && d.name != "L3" {
                            assert!(d.poise < 35.0, "{hero:?} {tier} {rank}: a light blow beats the Brute's armor by poise alone");
                        }
                    }
                    assert!(k.strike.power <= kit.strike.power * RIPOSTE * MUSHIN + 1e-9);
                }
            }
        }
    }

    #[test]
    fn the_war_cry_keeps_the_frame_rules() {
        // Readable as any move: a long startup to be struck out of, and a recovery open to a punish.
        const { assert!(WAR_CRY.startup >= 18 && WAR_CRY.recovery >= 10) };
        const { assert!(WAR_CRY.rest > WAR_CRY.total() as u32, "never two at once") };
        const { assert!(WAR_CRY.radius < 3.0 && WAR_CRY.stamina > 0.0) };
    }

    #[test]
    fn the_temper_charges_riposte_mushin_and_the_discipline() {
        let mut t = Temper::default();
        t.parry(100);
        t.swing(115);
        assert!(t.riposte && !t.mushin, "inside the window");
        t.swing(118);
        assert!(!t.riposte, "one swing");
        t.parry(120);
        t.swing(141);
        assert!(!t.riposte, "past it");
        t.parry(200);
        t.parry(300);
        t.swing(400);
        assert!(t.mushin && t.calm == 0, "three parries unwounded");
        t.swing(420);
        assert!(!t.mushin, "one swing");
        t.parry(500);
        t.parry(501);
        t.wound();
        t.parry(502);
        t.swing(600);
        assert!(!t.mushin, "a wound breaks the calm");
        for _ in 0..DISCIPLINE {
            t.block();
        }
        assert!(t.stance(HeroKind::Mehmed, 4).free_chain, "full: the first swing is free");
        t.swing(700);
        assert!(t.free && t.discipline == 0);
        t.rest();
        assert!(!t.stance(HeroKind::Mehmed, 4).free_chain);
    }

    #[test]
    fn every_trial_is_a_bout_of_the_ring_with_a_deed_of_its_way() {
        for hero in ROSTER {
            for rank in 1..=RANKS {
                let tr = trial(hero, rank);
                assert!(tr.bout < gym::BOUTS.len() && gym::BOUTS[tr.bout].tier == 1, "{}", tr.name);
                assert!(way(hero).feats.contains(&tr.feat), "{}", tr.name);
                assert!(tr.count > 0 && tr.limit > 0.0 && !tr.line.is_empty());
            }
        }
    }
}
