//! The three archetypes (COMBAT.md §4.3), each asking the player one question: the Brute
//! (*can you wait?*), the Skirmisher (*can you read the timing?*) and the Hexer (*can you
//! close the distance?*). Each wears the body of a foe already in the game, and each reaches
//! the hero its own way: the Brute's blows are melee, the Skirmisher throws its body, the
//! Hexer strikes from a distance (§10: every stage's cast has all three).

use super::frames::{MoveId, Rider};
use super::fsm::Fighter;
use crate::logic::enemy::EnemyKind;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Archetype {
    Brute,
    Skirmisher,
    Hexer,
}

/// How an archetype's blows reach the hero: with a weapon swung at arm's length, with its whole
/// body thrown at him, or from afar.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Attack {
    Melee,
    Body,
    Distance,
}

impl Attack {
    pub fn name(self) -> &'static str {
        match self {
            Attack::Melee => "MELEE",
            Attack::Body => "BODY",
            Attack::Distance => "DISTANCE",
        }
    }
}

/// An archetype's numbers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stats {
    pub hp: f64,
    pub poise: f64,
    pub armor: f64,
    pub stagger: u16,
    pub resolve: u16,
    /// Tiles per second walking to its target, strafing on its ring, backing off.
    pub walk: f64,
    pub strafe: f64,
    pub retreat: f64,
    /// Radians per second it turns.
    pub turn: f64,
    /// The radius of its ring while it waits for a token.
    pub ring: f64,
    /// Threat points its token costs.
    pub cost: u8,
    /// Its body's radius.
    pub radius: f64,
    /// Frames before it may ask for a token again after its attack.
    pub rest: u16,
}

impl Archetype {
    pub const ALL: [Archetype; 3] = [Archetype::Brute, Archetype::Skirmisher, Archetype::Hexer];

    pub const fn stats(self) -> Stats {
        match self {
            Archetype::Brute => Stats {
                hp: 200.0,
                poise: 100.0,
                armor: 35.0,
                stagger: 90,
                resolve: 120,
                walk: 1.8,
                strafe: 1.2,
                retreat: 1.2,
                turn: 150.0 * std::f64::consts::PI / 180.0,
                ring: 3.6,
                cost: 2,
                radius: 0.5,
                rest: 70,
            },
            Archetype::Skirmisher => Stats {
                hp: 80.0,
                poise: 25.0,
                armor: 0.0,
                stagger: 50,
                resolve: 60,
                walk: 4.6,
                strafe: 2.8,
                retreat: 4.0,
                turn: 540.0 * std::f64::consts::PI / 180.0,
                ring: 4.2,
                cost: 1,
                radius: 0.38,
                rest: 60,
            },
            Archetype::Hexer => Stats {
                hp: 55.0,
                poise: 20.0,
                armor: 0.0,
                stagger: 60,
                resolve: 60,
                walk: 2.2,
                strafe: 2.0,
                retreat: 3.2,
                turn: 360.0 * std::f64::consts::PI / 180.0,
                ring: 6.0,
                cost: 1,
                radius: 0.35,
                rest: 75,
            },
        }
    }

    /// How its blows reach him: the Brute swings, the Skirmisher throws itself, the Hexer casts.
    pub const fn attack(self) -> Attack {
        match self {
            Archetype::Brute => Attack::Melee,
            Archetype::Skirmisher => Attack::Body,
            Archetype::Hexer => Attack::Distance,
        }
    }

    /// The body it wears.
    pub fn kind(self) -> EnemyKind {
        match self {
            Archetype::Brute => EnemyKind::Blemmyes,
            Archetype::Skirmisher => EnemyKind::Werewolf,
            Archetype::Hexer => EnemyKind::Witch,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Archetype::Brute => "BRUTE",
            Archetype::Skirmisher => "SKIRMISHER",
            Archetype::Hexer => "HEXER",
        }
    }

    /// Its moves.
    pub fn moves(self) -> &'static [MoveId] {
        match self {
            Archetype::Brute => &[MoveId::Slam, MoveId::Sweep],
            Archetype::Skirmisher => &[MoveId::Lunge],
            Archetype::Hexer => &[MoveId::Hex],
        }
    }

    /// The farthest it attacks from (centre to centre), to plan its approach.
    pub fn range(self) -> f64 {
        match self {
            Archetype::Brute => 1.7,
            Archetype::Skirmisher => LUNGE_FAR,
            Archetype::Hexer => HEX_FAR,
        }
    }

    /// A fresh fighter for it.
    pub fn fighter(self) -> Fighter {
        let s = self.stats();
        Fighter::new(s.poise, s.armor, s.stagger, s.resolve)
    }
}

/**
 * A folklore power (COMBAT.md §11): what a body of the later stages does besides its moves.
 * Each is one rule on top of the frame data, drawn from what its legend says it does.
 */
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Power {
    None,
    /// The kappa's dish: parried, it bows back and spills it, and is weak until it fills again.
    Dish,
    /// The kitsune's illusions: wounded, it conjures copies of itself; one blow and they are gone.
    Illusions,
    /// The tülükuş's omen: its cry as it takes wing dooms him (his next wound is worse).
    Omen,
    /// The albastı's shade: a shadow with red eyes until she pounces.
    Shade,
    /// The cin's vanishing: come near and it is gone, and stands elsewhere.
    Vanish,
    /// The arçura's size: it grows tall as a tree to cast and shrinks to the grass to slip away.
    Grow,
    /// Tepegöz's ring: no blade cuts his body; only when his eye comes within reach (bent in his
    /// recovery, or reeling) does a blow tell.
    Ward,
    /// The gashadokuro's rattle: a ringing in the ears as it moves to strike.
    Rattle,
    /// The ejder's plea: felled, it begs for a second blow, and a second blow brings it back.
    Plea,
    /// The goryō's wrath: nothing touches it but while it rages (its moves).
    Spirit,
    /// The frost giant's rime: its blows chill, blocked or not.
    Rime,
    /// Surtr's sword: its blows leave the floor burning.
    Fire,
    /// Hel's rule over the dead: those she raised fall when she does.
    Necromancy,
    /// The elf-maid's back: lovely from the front, where no blow tells, and hollow behind.
    Hollow,
}

/**
 * A foe's body on an archetype (COMBAT.md §9, §10, §11): every kind in the game fights as one of
 * the three, its numbers scaled. The later stages' bodies bring their own moves (each a variation
 * on its archetype's verb: a lunge that shoves, a bolt that stuns, a slam that shakes the ground),
 * a rider on their blows and a folklore power.
 */
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Breed {
    pub kind: EnemyKind,
    pub arch: Archetype,
    /// Multiples of the archetype's health, damage, poise (and armor), and pace.
    pub hp: f64,
    pub damage: f64,
    pub poise: f64,
    pub speed: f64,
    /// Its body's radius, and the threat points its token costs.
    pub radius: f64,
    pub cost: u8,
    /// Tiles a big body adds to the reach of its blows (a giant's arm).
    pub reach: f64,
    /// What every blow of its carries besides its damage (unless the move has its own).
    pub rider: Rider,
    pub power: Power,
}

impl Breed {
    pub const fn of(kind: EnemyKind) -> Breed {
        use Archetype::*;
        use EnemyKind as K;
        let (arch, hp, damage, poise, speed, radius, cost) = match kind {
            // The second stage's bodies are the archetypes themselves...
            K::Blemmyes => (Brute, 1.0, 1.0, 1.0, 1.0, 0.5, 2),
            K::Werewolf => (Skirmisher, 1.0, 1.0, 1.0, 1.0, 0.38, 1),
            K::Witch => (Hexer, 1.0, 1.0, 1.0, 1.0, 0.35, 1),
            // ...with lighter cousins: the helmeted skeleton a small brute, the large bat a flying skirmisher.
            K::Helmeted => (Brute, 0.6, 0.7, 0.6, 1.15, 0.38, 1),
            K::LargeBat => (Skirmisher, 0.8, 0.85, 0.8, 1.1, 0.4, 1),
            K::Minion => (Skirmisher, 0.35, 0.55, 0.4, 1.0, 0.3, 1),
            // The Garden's four: a draugr chops like a brute, the ravens swoop, the bog slime spits.
            K::Skeleton => (Brute, 0.45, 0.6, 0.5, 1.1, 0.36, 1),
            K::Vampire => (Skirmisher, 0.6, 0.7, 0.6, 1.0, 0.32, 1),
            K::Bat => (Skirmisher, 0.45, 0.6, 0.5, 1.05, 0.3, 1),
            K::Slime => (Hexer, 0.8, 0.8, 0.8, 0.7, 0.38, 1),
            // The third stage's, stronger than the archetypes' own: two blades (the tengu quick on
            // his feet, the rasetsu heavy), two bodies thrown (the dire wolf's pounce, the cobra's
            // strike), and fire from afar (the jorogumo).
            K::Tengu => (Brute, 1.0, 1.15, 0.9, 1.5, 0.42, 2),
            K::Rasetsu => (Brute, 1.5, 1.2, 1.25, 1.0, 0.52, 2),
            K::DireWolf => (Skirmisher, 1.5, 1.2, 1.3, 1.1, 0.45, 1),
            K::Cobra => (Skirmisher, 1.25, 1.3, 1.0, 0.8, 0.4, 1),
            K::Jorogumo => (Hexer, 1.6, 1.2, 1.5, 1.0, 0.46, 1),
            // The fourth stage (the yōkai): the oni's club, the kappa's charge, the rokurokubi's
            // neck, the kitsune's foxfire.
            K::Oni => (Brute, 1.9, 1.35, 1.25, 1.0, 0.6, 2),
            K::Kappa => (Skirmisher, 1.9, 1.35, 1.4, 0.95, 0.42, 1),
            K::Rokurokubi => (Skirmisher, 1.7, 1.4, 1.2, 0.7, 0.4, 1),
            K::Kitsune => (Hexer, 2.0, 1.35, 1.5, 1.3, 0.38, 1),
            // The fifth (Anatolian nights): the karakoncolos's claws, the tülükuş's dive, the
            // albastı's pounce, the cin's fire, the arçura's roots.
            K::Karakoncolos => (Brute, 2.3, 1.35, 1.25, 1.15, 0.6, 2),
            K::Tulukus => (Skirmisher, 2.2, 1.5, 1.3, 1.1, 0.42, 1),
            K::Albasti => (Skirmisher, 2.4, 1.45, 1.4, 1.05, 0.42, 1),
            K::Cin => (Hexer, 2.6, 1.5, 1.6, 1.0, 0.38, 1),
            K::Arcura => (Hexer, 2.8, 1.45, 1.8, 0.95, 0.45, 1),
            // The sixth (the legends): two giants, the dragon, the ghost.
            K::Tepegoz => (Brute, 4.0, 1.7, 1.25, 0.85, 0.85, 2),
            K::Gashadokuro => (Brute, 3.0, 1.65, 1.25, 0.9, 0.9, 2),
            K::Ejder => (Skirmisher, 4.0, 1.6, 1.6, 0.75, 0.9, 2),
            K::Goryo => (Hexer, 3.0, 1.7, 2.0, 1.0, 0.4, 1),
            // The seventh (Ragnarök).
            K::FrostGiant => (Brute, 3.6, 1.9, 1.25, 0.9, 0.9, 2),
            K::Surtr => (Brute, 4.0, 2.0, 1.25, 0.85, 0.95, 2),
            K::Nidhogg => (Skirmisher, 5.0, 1.7, 1.6, 0.85, 0.8, 2),
            K::Hel => (Hexer, 4.0, 1.9, 2.0, 1.0, 0.42, 1),
            // The realms' own (REALMS.md), grown with the realm's tier as they are placed
            // (`logic::realms::weight`). The Peerless fights with his own kit (`game::realms::mirrors`).
            K::Peerless => (Brute, 1.6, 1.0, 1.25, 1.0, 0.4, 2),
            // Hel's servants wade in, slow and hard to stop.
            K::Ganglot => (Brute, 1.3, 0.9, 2.0, 0.45, 0.45, 1),
            // The gaki swarm.
            K::Gaki => (Skirmisher, 0.9, 0.95, 0.8, 1.05, 0.38, 1),
            // The elf-maids: hollow behind, and only there to be struck.
            K::Ellepige => (Skirmisher, 1.3, 1.1, 1.0, 1.0, 0.36, 1),
        };
        // A big body's arm reaches as far again as its girth outgrows a man's.
        let reach = if radius > 0.55 { (radius - 0.5) * 2.0 } else { 0.0 };
        let (rider, power) = match kind {
            K::Kappa => (Rider::None, Power::Dish),
            K::Kitsune => (Rider::None, Power::Illusions),
            K::Tulukus => (Rider::None, Power::Omen),
            K::Albasti => (Rider::None, Power::Shade),
            K::Cin => (Rider::None, Power::Vanish),
            K::Arcura => (Rider::None, Power::Grow),
            K::Tepegoz => (Rider::None, Power::Ward),
            K::Gashadokuro => (Rider::None, Power::Rattle),
            K::Ejder => (Rider::None, Power::Plea),
            K::Goryo => (Rider::None, Power::Spirit),
            K::FrostGiant => (Rider::Chill, Power::Rime),
            K::Surtr => (Rider::None, Power::Fire),
            K::Nidhogg => (Rider::Venom(120), Power::None),
            K::Hel => (Rider::Drain, Power::Necromancy),
            K::Ellepige => (Rider::None, Power::Hollow),
            _ => (Rider::None, Power::None),
        };
        Breed { kind, arch, hp, damage, poise, speed, radius, cost, reach, rider, power }
    }

    /// The moves it knows, in the order it prefers them: its archetype's own, or its legend's.
    pub fn moves(&self) -> &'static [MoveId] {
        use EnemyKind as K;
        match self.kind {
            K::Oni => &[MoveId::Kanabo, MoveId::Sweep],
            K::Kappa => &[MoveId::Shove],
            K::Rokurokubi => &[MoveId::Lash],
            K::Kitsune => &[MoveId::Foxfire],
            K::Karakoncolos => &[MoveId::Call, MoveId::Slam, MoveId::Sweep],
            K::Tulukus => &[MoveId::Dive],
            K::Albasti => &[MoveId::Pounce],
            K::Cin => &[MoveId::Smokeless],
            K::Arcura => &[MoveId::Roots],
            K::Gashadokuro => &[MoveId::Grab, MoveId::Slam, MoveId::Sweep],
            K::Ejder => &[MoveId::Heads],
            K::Goryo => &[MoveId::Thunder],
            K::FrostGiant => &[MoveId::Frostwave, MoveId::Slam, MoveId::Sweep],
            K::Nidhogg => &[MoveId::Burrow, MoveId::Lunge],
            K::Hel => &[MoveId::Raise, MoveId::Hex],
            _ => self.arch.moves(),
        }
    }

    /// The move it closes in to use (the first it knows that strikes, not a spell or one from afar).
    pub fn main_move(&self) -> MoveId {
        let moves = self.moves();
        moves.iter().copied().find(|m| m.data().strikes() && m.data().cooldown == 0).unwrap_or(moves[0])
    }

    /// How its blows reach the hero.
    pub fn attack(&self) -> Attack {
        self.arch.attack()
    }

    /// The archetype's numbers as this body has them.
    pub fn stats(&self) -> Stats {
        let s = self.arch.stats();
        Stats {
            hp: s.hp * self.hp,
            poise: s.poise * self.poise,
            armor: s.armor * self.poise,
            walk: s.walk * self.speed,
            strafe: s.strafe * self.speed,
            retreat: s.retreat * self.speed,
            radius: self.radius,
            cost: self.cost,
            ..s
        }
    }

    /// A fresh fighter for it.
    pub fn fighter(&self) -> Fighter {
        let s = self.stats();
        Fighter::new(s.poise, s.armor, s.stagger, s.resolve)
    }
}

/// The Brute sweeps a hero this close outside its slam's cone.
pub const SWEEP_NEAR: f64 = 1.3;
/// The Skirmisher lunges from between these distances, and steps back when nearer.
pub const LUNGE_NEAR: f64 = 1.6;
pub const LUNGE_FAR: f64 = 3.2;
/// The Hexer casts from no farther than this, and backs off from a hero nearer than `FLEE`.
pub const HEX_FAR: f64 = 7.5;
pub const HEX_NEAR: f64 = 2.5;
pub const FLEE: f64 = 3.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_archetypes_three_questions() {
        let (b, s, h) = (Archetype::Brute.stats(), Archetype::Skirmisher.stats(), Archetype::Hexer.stats());
        // The Brute can't be interrupted by light blows; the others flinch from everything.
        assert!(b.armor > MoveId::L1.data().poise && b.armor < MoveId::L3.data().poise);
        assert!(s.armor == 0.0 && h.armor == 0.0);
        // The Skirmisher's poise breaks inside one string; the Brute's takes two.
        let string = MoveId::L1.data().poise + MoveId::L2.data().poise + MoveId::L3.data().poise;
        assert!(s.poise <= MoveId::L1.data().poise + MoveId::L2.data().poise);
        assert!(b.poise > string && b.poise <= 2.0 * string);
        // The Skirmisher is a little slower than the hero (6.5 x 0.8 = 5.2 t/s), so he can always get away,
        // and more than twice as quick as the Brute.
        assert!(s.walk < 6.5 * 0.8 && s.walk > b.walk * 2.0);
        // The Hexer waits beyond the others, outside the hero's reach.
        assert!(h.ring > s.ring && h.ring > b.ring && FLEE > MoveId::L3.data().reach);
        // A Brute's token is worth two of the others.
        assert_eq!((b.cost, s.cost, h.cost), (2, 1, 1));
    }

    #[test]
    fn every_body_fights_as_one_of_the_three_and_the_archetypes_own_bodies_are_full_strength() {
        for arch in Archetype::ALL {
            assert_eq!(Breed::of(arch.kind()).stats(), arch.stats(), "{arch:?}");
        }
        let all = [EnemyKind::STAGE_ONE.as_slice(), EnemyKind::STAGE_TWO.as_slice()].concat();
        for kind in all {
            let b = Breed::of(kind);
            let s = b.stats();
            assert!(b.hp <= 1.0 && b.damage <= 1.0 && s.hp > 0.0, "{kind:?}: no body is stronger than its archetype");
            // Every brute keeps armor against the training sword's light blows... except the lightest.
            if b.arch == Archetype::Brute && b.poise >= 0.6 {
                assert!(s.armor > MoveId::L1.data().poise, "{kind:?}");
            }
        }
        // The third stage is stronger: every body hits harder than its archetype's own and none is
        // frailer; yet every brute still flinches from the finisher, and every skirmisher is still
        // slower than the hero and broken inside one string.
        for kind in EnemyKind::STAGE_THREE {
            let b = Breed::of(kind);
            let s = b.stats();
            assert!(b.hp >= 1.0 && b.damage > 1.0, "{kind:?}");
            if b.arch == Archetype::Brute {
                assert!(s.armor > MoveId::L1.data().poise && s.armor < MoveId::L3.data().poise, "{kind:?}: {}", s.armor);
            }
            if b.arch == Archetype::Skirmisher {
                assert!(s.walk < 6.5 * 0.8, "{kind:?}: {}", s.walk);
                assert!(s.poise <= MoveId::L1.data().poise + MoveId::L2.data().poise, "{kind:?}: {}", s.poise);
            }
        }
    }

    #[test]
    fn every_stage_is_stronger_than_the_last_and_every_breed_inside_its_archetypes_limits() {
        // From the third stage on, each stage's bodies are tougher and hit harder, on average,
        // than the stage before's.
        let mean = |stage: u8, f: &dyn Fn(&Breed) -> f64| {
            let cast = EnemyKind::of_stage(stage);
            cast.iter().map(|&k| f(&Breed::of(k))).sum::<f64>() / cast.len() as f64
        };
        for stage in 4..=EnemyKind::STAGES {
            for (what, f) in [("health", &(|b: &Breed| b.hp) as &dyn Fn(&Breed) -> f64), ("damage", &|b: &Breed| b.damage)] {
                assert!(
                    mean(stage, f) > mean(stage - 1, f),
                    "stage {stage}'s {what} {:.2} is not over {:.2}",
                    mean(stage, f),
                    mean(stage - 1, f)
                );
            }
            for &kind in EnemyKind::of_stage(stage) {
                let b = Breed::of(kind);
                let s = b.stats();
                // The finisher still breaks through a brute's armor in neutral; a skirmisher is still
                // slower than he is; a body as big as a giant's reaches like one.
                if b.arch == Archetype::Brute {
                    assert!(s.armor < MoveId::L3.data().poise, "{kind:?}: {}", s.armor);
                }
                if b.arch == Archetype::Skirmisher {
                    assert!(s.walk < 6.5 * 0.8, "{kind:?}: {}", s.walk);
                }
                assert!(b.reach >= 0.0 && b.reach <= 1.0, "{kind:?}: {}", b.reach);
                // Every move it knows is a move a foe may use, and the first that strikes is the one it closes in for.
                assert!(b.moves().iter().all(|m| MoveId::FOE.contains(m)), "{kind:?}");
                assert!(b.main_move().data().strikes(), "{kind:?}");
            }
        }
    }

    #[test]
    fn every_stages_cast_attacks_all_three_ways() {
        for stage in 1..=EnemyKind::STAGES {
            let ways: Vec<Attack> = EnemyKind::of_stage(stage).iter().map(|&k| Breed::of(k).attack()).collect();
            for way in [Attack::Melee, Attack::Body, Attack::Distance] {
                assert!(ways.contains(&way), "stage {stage} has no {way:?} attacker: {ways:?}");
            }
        }
    }
}
