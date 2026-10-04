//! The prince's cheats (WARBAND.md §10.3): always lies, always with a tell.
//!
//! The old man is never one of them. The prince lies in his own voice and his own colour: he never
//! takes the old man's form, his name, his shop or his dog (the user's rule: the old man always
//! leads to good).
//!
//! Every cheat is a struct of **the lie, its tells, its counter, the reward for seeing through it,
//! and what it costs if believed** (`Cheat`), and `every_cheat_has_a_tell_a_counter_and_a_bounded_cost`
//! checks each part is there. They come only in Rest and Build, one at a time, at most two in a
//! realm, and never take an owned tier, a relic or a life: what they take is time, health, silver
//! carried, or a better road (`Stake`).
//!
//! The whisper is false counsel, and false by construction: it names an answer that does not hold
//! for the breed (`Answer::holds`), a trait the rival does not have, or the way the portal is not.

use super::counsel::{address, Answer};
use super::drum::DrumPhase;
use super::ledger::{counsel_line_for, Family, Rival, RivalId, Side, Trait, Weakness};
use crate::logic::enemy::EnemyKind;
use crate::logic::heroes::HeroKind;
use crate::logic::sins::Sin;
use crate::logic::types::Dir;

/// At most this many cheats in one realm.
pub const MAX_PER_REALM: usize = 2;
/// A whisper is "live" this long: ask the old man within it to see through it.
pub const ASK_WINDOW: f64 = 40.0;
/// Runes for each lie seen through.
pub const SEEN_RUNES: u32 = 2;
/// The turned sign points wrong this long.
pub const SIGN_SECONDS: f64 = 20.0;
/// The share of his health a mimic's first bite may take at most (the duelist's own blow, capped by
/// its breed's damage; checked as a bound, not applied).
pub const BITE_SHARE: f64 = 0.3;

/// The four cheats.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub enum CheatKind {
    /// False counsel, in the prince's own voice.
    Whisper,
    /// A deal mid-realm, its price bigger than it says.
    CrookedBargain,
    /// A mimic chest.
    LoadedDice,
    /// The compass turned toward a trap.
    TurnedSign,
}

impl CheatKind {
    pub const ALL: [CheatKind; 4] = [CheatKind::Whisper, CheatKind::CrookedBargain, CheatKind::LoadedDice, CheatKind::TurnedSign];

    pub fn name(self) -> &'static str {
        match self {
            CheatKind::Whisper => "THE WHISPER",
            CheatKind::CrookedBargain => "THE CROOKED BARGAIN",
            CheatKind::LoadedDice => "LOADED DICE",
            CheatKind::TurnedSign => "THE TURNED SIGN",
        }
    }

    /// Where it may be dealt: the dice only where there are chests to mimic (Greed, Gluttony); the
    /// bargain only where the realm has a road to shorten (not the vaults).
    pub fn fits(self, sin: Sin) -> bool {
        match self {
            CheatKind::LoadedDice => matches!(sin, Sin::Greed | Sin::Gluttony),
            CheatKind::CrookedBargain => sin != Sin::Greed,
            _ => true,
        }
    }
}

/// How a cheat gives itself away. The old man's signs are amber and Quitmer's bark; the prince's
/// voice has neither.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Tell {
    /// The sin's colour where the old man's amber should be.
    SinColour,
    /// A reversed echo after the words.
    ReversedEcho,
    /// No bark from Quitmer.
    NoBark,
    /// No face in the frame.
    NoPortrait,
    /// The old man said it early: the prince's offers cost more than they say.
    CostsMore,
    /// The chest breathes.
    Breathes,
    /// Its coins never glint.
    NoGlint,
    /// Its hinge creaks.
    Creaks,
    /// The needle flickers.
    Flickers,
}

/// How it is seen through.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Counter {
    /// Call the old man (H): "I said no such thing."
    AskTheOldMan,
    /// Refuse the deal.
    Refuse,
    /// Strike the chest before opening it.
    StrikeFirst,
    /// Follow the old man's amber marker, which is always true.
    FollowTheLamp,
}

/// What seeing through it gives.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Reward {
    /// A lie seen through: the count, and runes.
    SeenThrough { runes: u32 },
    /// The chest struck first gives twice (and the mark).
    DoubleLoot,
    /// Refusing a bargain gives nothing: it is a decision, not a trap.
    Nothing,
}

/// What a believed cheat may take: never what the hero owns.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stake {
    /// Time.
    Time,
    /// Health (never a life).
    Health,
    /// Silver he carries (never what is banked, never a tier).
    SilverCarried,
    /// A better road.
    Route,
}

/// What it costs if believed.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Cost {
    /// A wasted elixir or a walk into a den.
    WrongCounsel,
    /// The gain is real, and so is the hidden price: the realm's dens all wake.
    HiddenPrice,
    /// It bites: one body strike, planned by the threat manager.
    Bite,
    /// A detour for `SIGN_SECONDS`.
    Detour,
}

impl Cost {
    /// What it can take.
    pub fn stakes(self) -> &'static [Stake] {
        match self {
            Cost::WrongCounsel => &[Stake::Time, Stake::Health],
            Cost::HiddenPrice => &[Stake::Health, Stake::Time],
            Cost::Bite => &[Stake::Health],
            Cost::Detour => &[Stake::Time, Stake::Route],
        }
    }

    /// How long it lasts at most, in seconds (the realm's length for the wares and the price).
    pub fn bound(self) -> Bound {
        match self {
            Cost::WrongCounsel => Bound::Seconds(ASK_WINDOW),
            Cost::HiddenPrice => Bound::TheRealm,
            Cost::Bite => Bound::Share(BITE_SHARE),
            Cost::Detour => Bound::Seconds(SIGN_SECONDS),
        }
    }
}

/// How far a cost reaches: some seconds, a share of his health, or until he leaves the realm.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Bound {
    Seconds(f64),
    Share(f64),
    TheRealm,
}

/// A cheat, whole.
#[derive(Clone, PartialEq, Debug)]
pub struct Cheat {
    pub kind: CheatKind,
    pub sin: Sin,
    /// What the prince says (his sin's flavour).
    pub lie: String,
    pub tells: Vec<Tell>,
    pub counter: Counter,
    pub reward: Reward,
    pub cost: Cost,
}

/// The cheat `kind` as `sin`'s prince plays it.
pub fn cheat(kind: CheatKind, sin: Sin) -> Cheat {
    let (tells, counter, reward, cost) = match kind {
        CheatKind::Whisper => (
            vec![Tell::SinColour, Tell::ReversedEcho, Tell::NoBark, Tell::NoPortrait],
            Counter::AskTheOldMan,
            Reward::SeenThrough { runes: SEEN_RUNES },
            Cost::WrongCounsel,
        ),
        CheatKind::CrookedBargain => (vec![Tell::CostsMore, Tell::SinColour], Counter::Refuse, Reward::Nothing, Cost::HiddenPrice),
        CheatKind::LoadedDice => (vec![Tell::Breathes, Tell::NoGlint, Tell::Creaks], Counter::StrikeFirst, Reward::DoubleLoot, Cost::Bite),
        CheatKind::TurnedSign => {
            (vec![Tell::SinColour, Tell::Flickers], Counter::FollowTheLamp, Reward::SeenThrough { runes: SEEN_RUNES }, Cost::Detour)
        }
    };
    Cheat { kind, sin, lie: lie(kind, sin), tells, counter, reward, cost }
}

/// What the prince says, in his sin's flavour: Mammon's are all prices, Lucifer's flatter,
/// Leviathan offers what another hero had, Amon goads, Belphegor offers rest, Beelzebub food,
/// Asmodeus what the hero wants most.
pub fn lie(kind: CheatKind, sin: Sin) -> String {
    use CheatKind as C;
    let s = match (sin, kind) {
        (Sin::Greed, C::Whisper) => "Listen. I would charge for this, but you may have it free:",
        (Sin::Greed, C::CrookedBargain) => "A fair price, and a short road: the door, now.",
        (Sin::Greed, C::LoadedDice) => "A chest, unguarded. Whose? Yours, if you are quick.",
        (Sin::Greed, C::TurnedSign) => "The door is this way, and it costs you nothing.",
        (Sin::Pride, C::Whisper) => "Listen. A hero like you needs only half of this:",
        (Sin::Pride, C::CrookedBargain) => "You have proven yourself. Why walk the long way? The door is open.",
        (Sin::Pride, C::LoadedDice) => "A prize worthy of you. Take it.",
        (Sin::Pride, C::TurnedSign) => "The bold way is this way. It suits you.",
        (Sin::Envy, C::Whisper) => "Listen. The other one knew this, and it kept him alive:",
        (Sin::Envy, C::CrookedBargain) => "The samurai took this road. Will you let him be first?",
        (Sin::Envy, C::LoadedDice) => "The northman left this chest behind. It is yours now.",
        (Sin::Envy, C::TurnedSign) => "The others went this way. Follow them.",
        (Sin::Wrath, C::Whisper) => "Listen, unless you are afraid to:",
        (Sin::Wrath, C::CrookedBargain) => "Through them, now, the door open. Unless you would rather hide.",
        (Sin::Wrath, C::LoadedDice) => "A chest. Kick it open, if you dare.",
        (Sin::Wrath, C::TurnedSign) => "The fight is this way. Go on.",
        (Sin::Sloth, C::Whisper) => "Listen, and save yourself the trouble:",
        (Sin::Sloth, C::CrookedBargain) => "Why walk? Rest, and the door opens by itself.",
        (Sin::Sloth, C::LoadedDice) => "A chest, and no work to open it.",
        (Sin::Sloth, C::TurnedSign) => "The easy way is this way.",
        (Sin::Gluttony, C::Whisper) => "Listen, and eat your fill of this:",
        (Sin::Gluttony, C::CrookedBargain) => "One more course, and then the door. Just one.",
        (Sin::Gluttony, C::LoadedDice) => "A chest, and it is full. Help yourself.",
        (Sin::Gluttony, C::TurnedSign) => "The banquet is this way.",
        (Sin::Lust, C::Whisper) => "Listen. You want this more than anything:",
        (Sin::Lust, C::CrookedBargain) => "The boy you came for is just beyond that door. I'll open it now.",
        (Sin::Lust, C::LoadedDice) => "What you most desire is in that chest.",
        (Sin::Lust, C::TurnedSign) => "What you want is this way.",
    };
    s.to_string()
}

/// The cheats a realm deals (`MAX_PER_REALM`, never the same twice, each where it fits), from `seed`.
pub fn deal(sin: Sin, seed: u32) -> Vec<CheatKind> {
    let fits: Vec<CheatKind> = CheatKind::ALL.iter().copied().filter(|c| c.fits(sin)).collect();
    let mut h = seed.wrapping_mul(2_654_435_761) ^ 0x9e37_79b9;
    let mut out = Vec::new();
    while out.len() < MAX_PER_REALM.min(fits.len()) {
        h ^= h << 13;
        h ^= h >> 17;
        h ^= h << 5;
        let c = fits[(h as usize) % fits.len()];
        if !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

/// May a cheat begin in this phase of the Drum? Only in Rest and Build.
pub fn may_begin(phase: DrumPhase) -> bool {
    matches!(phase, DrumPhase::Rest | DrumPhase::Build)
}

// ------------------------------------------------------------------ the whisper

/// What a whisper claims, false by construction.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FalseFact {
    /// An answer that does not hold for the breed.
    Answer(EnemyKind, Answer),
    /// A weakness the rival does not have.
    Trait(RivalId, Trait),
    /// The portal lies this way; it does not.
    Way(Dir),
}

/// A whisper: what it says, what it claims, and the truth the old man puts in its place.
#[derive(Clone, PartialEq, Debug)]
pub struct Whisper {
    pub sin: Sin,
    pub text: String,
    pub claim: FalseFact,
    pub truth: String,
}

/// Steps a seed (xorshift), for the choices a whisper makes.
fn roll(seed: u32) -> u32 {
    let mut h = seed.wrapping_mul(2_654_435_761) | 1;
    h ^= h << 13;
    h ^= h >> 17;
    h ^= h << 5;
    h
}

/// A false answer for `kind`, in the old man's words but the prince's mouth.
pub fn whisper_breed(kind: EnemyKind, sin: Sin, hero: HeroKind, seed: u32) -> Whisper {
    let wrong: Vec<Answer> = Answer::PLAUSIBLE.iter().copied().filter(|a| !a.holds(kind)).collect();
    let a = wrong[roll(seed) as usize % wrong.len()];
    let truth = Answer::of(kind).body(kind);
    Whisper {
        sin,
        text: format!("{} {} Trust me, {}.", lie(CheatKind::Whisper, sin), a.body(kind), address(hero)),
        claim: FalseFact::Answer(kind, a),
        truth,
    }
}

/// A weakness the rival does not have, said as the old man would say a true one.
pub fn whisper_rival(r: &Rival, sin: Sin, hero: HeroKind, seed: u32) -> Whisper {
    let pool = [
        Weakness::Proud,
        Weakness::Greedy,
        Weakness::OldWound,
        Weakness::Scarred(Side::Left),
        Weakness::Scarred(Side::Right),
        Weakness::Fears(Family::Steel),
        Weakness::Fears(Family::Fire),
        Weakness::Fears(Family::Thrown),
    ];
    let wrong: Vec<Weakness> = pool.iter().copied().filter(|&w| w != r.weakness).collect();
    let w = wrong[roll(seed) as usize % wrong.len()];
    // The line his own would be, had he that weakness and nothing else known.
    let mut fake = r.clone();
    fake.weakness = w;
    fake.known = super::ledger::KNOW_WEAKNESS;
    let mut true_one = r.clone();
    true_one.known = super::ledger::KNOW_WEAKNESS;
    Whisper {
        sin,
        text: format!("{} {} Trust me, {}.", lie(CheatKind::Whisper, sin), counsel_line_for(&fake), address(hero)),
        claim: FalseFact::Trait(r.id, Trait::Weakness(w)),
        truth: counsel_line_for(&true_one),
    }
}

/// The wrong way to the portal: it lies `truth`, the whisper says the opposite.
pub fn whisper_way(truth: Dir, sin: Sin, hero: HeroKind) -> Whisper {
    let wrong = truth.opposite();
    let name = |d: Dir| match d {
        Dir::Up => "north",
        Dir::Down => "south",
        Dir::Left => "west",
        Dir::Right => "east",
        Dir::None => "nowhere",
    };
    Whisper {
        sin,
        text: format!("{} The way out lies {}. Trust me, {}.", lie(CheatKind::Whisper, sin), name(wrong), address(hero)),
        claim: FalseFact::Way(wrong),
        truth: format!("The way out lies {}. Follow my lamp.", name(truth)),
    }
}

impl FalseFact {
    /// Is it false? (`truth`: where the portal really lies, for a way; `rival`: the rival, for a
    /// trait.) Every whisper's claim must be.
    pub fn is_false(&self, rival: Option<&Rival>, way: Option<Dir>) -> bool {
        match *self {
            FalseFact::Answer(kind, a) => !a.holds(kind),
            FalseFact::Trait(id, t) => rival.is_some_and(|r| r.id == id && !r.true_traits().contains(&t)),
            FalseFact::Way(d) => way.is_some_and(|w| w != d),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::heroes::ROSTER;
    use crate::logic::sins::SINS;
    use crate::logic::war::ledger::{Ledger, KINDS};
    use crate::logic::war::names::{self, Deed};

    #[test]
    fn every_cheat_has_a_tell_a_counter_and_a_bounded_cost() {
        for sin in SINS {
            for kind in CheatKind::ALL {
                let c = cheat(kind, sin);
                assert!(!c.lie.is_empty() && names::drawable(&c.lie), "{kind:?} in {sin:?}: a lie");
                assert!(!c.tells.is_empty(), "{kind:?}: a tell");
                // Each faked sign of the old man's (amber, Quitmer, the lantern) or a thing seen.
                assert!(c.tells.len() >= 2, "{kind:?}: more than one tell, so one missed is not the end");
                assert!(!c.cost.stakes().is_empty(), "{kind:?}: a cost");
                match c.cost.bound() {
                    Bound::Seconds(s) => assert!(s > 0.0 && s <= 60.0, "{kind:?}: {s} s"),
                    Bound::Share(f) => assert!(f > 0.0 && f <= 0.35, "{kind:?}: {f} of his health"),
                    Bound::TheRealm => {}
                }
                // Every counter but the refusal is rewarded; the bargain is a decision.
                assert_eq!(c.reward == Reward::Nothing, kind == CheatKind::CrookedBargain, "{kind:?}");
            }
            // Never in Peak or Release; never more than two in a realm; the dice only with chests.
            for seed in 0..50 {
                let dealt = deal(sin, seed);
                assert_eq!(dealt.len(), MAX_PER_REALM);
                assert!(dealt[0] != dealt[1]);
                assert!(dealt.iter().all(|c| c.fits(sin)), "{sin:?}: {dealt:?}");
            }
        }
        assert!(may_begin(DrumPhase::Rest) && may_begin(DrumPhase::Build));
        assert!(!may_begin(DrumPhase::Peak) && !may_begin(DrumPhase::Release));
        // Each prince flavours his own: no two sins tell the same lie.
        for kind in CheatKind::ALL {
            let mut lies: Vec<String> = SINS.iter().map(|&s| lie(kind, s)).collect();
            lies.sort();
            lies.dedup();
            assert_eq!(lies.len(), 7, "{kind:?}");
        }
    }

    #[test]
    fn the_prince_never_takes_what_the_hero_owns() {
        // What a believed cheat can take is time, health, silver carried, or the road; there is no
        // stake for a tier, a relic, banked silver or a life, and every cost has a bound.
        for kind in CheatKind::ALL {
            let c = cheat(kind, Sin::Lust);
            for s in c.cost.stakes() {
                assert!(matches!(s, Stake::Time | Stake::Health | Stake::SilverCarried | Stake::Route), "{kind:?}: {s:?}");
            }
        }
    }

    /// The old man is never one of the prince's cheats: none of them takes his form, his name,
    /// his shop or his dog, and none speaks with his face or Quitmer's bark.
    #[test]
    fn no_cheat_takes_the_old_mans_form() {
        for kind in CheatKind::ALL {
            assert!(!kind.name().contains("GATEKEEPER") && !kind.name().contains("OLD MAN"), "{kind:?}");
            for sin in SINS {
                let c = cheat(kind, sin);
                let lie = c.lie.to_lowercase();
                assert!(!lie.contains("old man") && !lie.contains("quitmer") && !lie.contains("half price"), "{kind:?} {sin:?}: {lie}");
            }
        }
        let tells = cheat(CheatKind::Whisper, Sin::Greed).tells;
        assert!(tells.contains(&Tell::NoBark) && tells.contains(&Tell::NoPortrait), "the prince's voice has neither");
    }

    #[test]
    fn the_whisper_is_always_false_and_always_tellable() {
        let tells = cheat(CheatKind::Whisper, Sin::Pride).tells;
        assert!(tells.contains(&Tell::SinColour) && tells.contains(&Tell::ReversedEcho) && tells.contains(&Tell::NoBark));
        for sin in SINS {
            for (k, kind) in KINDS.into_iter().enumerate() {
                for hero in ROSTER {
                    let w = whisper_breed(kind, sin, hero, k as u32 * 31 + sin.index() as u32);
                    assert!(w.claim.is_false(None, None), "{kind:?}: {:?} holds", w.claim);
                    // It names the false answer, and not the true one.
                    let text = w.text.to_lowercase();
                    let FalseFact::Answer(_, a) = w.claim else { panic!() };
                    assert!(text.contains(a.keyword()) && !text.contains(Answer::of(kind).keyword()), "{text}");
                    assert!(w.truth.to_lowercase().contains(Answer::of(kind).keyword()));
                    assert!(names::drawable(&w.text), "{}", w.text);
                }
            }
        }
        let mut l = Ledger::default();
        for seed in 0..100u32 {
            let id = l.promote(KINDS[seed as usize % KINDS.len()], Deed::ALL[seed as usize % 5], HeroKind::Ragnar, seed * 7919);
            let r = l.get(id).unwrap();
            let w = whisper_rival(r, Sin::Envy, HeroKind::Ragnar, seed);
            assert!(w.claim.is_false(Some(r), None), "{:?} is his", w.claim);
            let FalseFact::Trait(_, t) = w.claim else { panic!() };
            assert!(w.text.contains(&t.keyword()), "{}", w.text);
            assert!(w.truth.contains(&Trait::Weakness(r.weakness).keyword()), "{}", w.truth);
        }
        for d in [Dir::Up, Dir::Down, Dir::Left, Dir::Right] {
            let w = whisper_way(d, Sin::Wrath, HeroKind::Miyamoto);
            assert!(w.claim.is_false(None, Some(d)));
        }
    }
}
