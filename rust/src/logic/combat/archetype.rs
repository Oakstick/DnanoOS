//! The three archetypes (COMBAT.md §4.3), each asking the player one question: the Brute
//! (*can you wait?*), the Skirmisher (*can you read the timing?*) and the Hexer (*can you
//! close the distance?*). Each wears the body of a foe already in the game, and each reaches
//! the hero its own way: the Brute's blows are melee, the Skirmisher bites from beside him, the
//! Hexer sends bolts from a distance (§10: every stage's cast has all three). Only the Hexer
//! strikes from afar; nothing of the others' darts across the floor or stretches out to him.
//!
//! The fourth, the Prince (COMBAT.md §13), is a sin's boss: the same fighter and the same rules,
//! with a move table for each phase of his health, armour that only his recoveries open, and a
//! token that costs three (`prince`).

use super::frames::{MoveId, Rider};
use super::fsm::Fighter;
use crate::logic::enemy::EnemyKind;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Archetype {
    Brute,
    Skirmisher,
    Hexer,
    /// A sin's prince: the boss of its court (`prince`).
    Prince,
}

/// How an archetype's blows reach the hero: with a weapon swung at arm's length, with its own
/// body (teeth, claws, weight) from beside him, or with bolts from afar.
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
                ring: 3.2,
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
            Archetype::Prince => Stats {
                hp: 900.0,
                poise: 150.0,
                armor: 40.0,
                stagger: 90,
                resolve: 300,
                walk: 2.0,
                strafe: 1.3,
                retreat: 1.3,
                turn: 130.0 * std::f64::consts::PI / 180.0,
                ring: 3.8,
                cost: 3,
                radius: 0.95,
                rest: 50,
            },
        }
    }

    /// How its blows reach him: the Brute swings, the Skirmisher bites, the Hexer casts (and a
    /// prince swings: whatever else he has, it is his weapon he closes in for).
    pub const fn attack(self) -> Attack {
        match self {
            Archetype::Brute | Archetype::Prince => Attack::Melee,
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
            Archetype::Prince => EnemyKind::Mammon,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Archetype::Brute => "BRUTE",
            Archetype::Skirmisher => "SKIRMISHER",
            Archetype::Hexer => "HEXER",
            Archetype::Prince => "PRINCE",
        }
    }

    /// Its moves.
    pub fn moves(self) -> &'static [MoveId] {
        match self {
            Archetype::Brute => &[MoveId::Slam, MoveId::Sweep],
            Archetype::Skirmisher => &[MoveId::Bite],
            Archetype::Hexer => &[MoveId::Hex],
            Archetype::Prince => super::prince::table(EnemyKind::Mammon, 1),
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
    /// The frost giant's rime: its blows frostbite, blocked or not.
    Rime,
    /// Surtr's sword: its blows leave the floor burning.
    Fire,
    /// Hel's rule over the dead: those she raised fall when she does.
    Necromancy,
    /// The elf-maid's back: lovely from the front, where no blow tells, and hollow behind.
    Hollow,
    /// Mammon's hide, sewn with coins: it turns most of a blow except in his recoveries, reeling,
    /// or while his fist is closed on him.
    GoldHide,
    // The six princes' powers: for now each a hide like Mammon's (`is_hide`), until his fight is built.
    /// Lucifer's vainglory (Pride).
    Vainglory,
    /// Leviathan's scales (Envy).
    Scales,
    /// Amon's fury (Wrath): no hide; every blow he takes quickens him (`Lore::rage`,
    /// `prince::wrath::enraged`), and a charge that ends in stone dazes him and spends it.
    Fury,
    /// Belphegor's torpor (Sloth): a hide as Mammon's, open while he sleeps and as he wakes.
    Torpor,
    /// Beelzebub's bloat (Gluttony).
    Bloat,
    /// Asmodeus's allure (Lust).
    Allure,
}

impl Power {
    /// A prince's hide: it turns most of a blow outside his recoveries (`prince::HIDE_SHARE`).
    /// Mammon's gold, and for now every other prince's power too.
    pub const fn is_hide(self) -> bool {
        matches!(self, Power::GoldHide | Power::Vainglory | Power::Scales | Power::Torpor | Power::Bloat | Power::Allure)
    }
}

/**
 * A foe's body on an archetype (COMBAT.md §9, §10, §11): every kind in the game fights as one of
 * the three, its numbers scaled. The later stages' bodies bring their own moves (each a variation
 * on its archetype's verb: a bite that shoves, a bolt of lightning, a slam that shakes the ground),
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
    /// The phase of a prince's fight (1 to 3, by his health; `prince::phase_of`): which of his move
    /// tables he fights with. 1 for every other body.
    pub phase: u8,
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
            // ...with lighter cousins: the helmeted skeleton a small brute, the large bat a biting skirmisher.
            K::Helmeted => (Brute, 0.6, 0.7, 0.6, 1.15, 0.38, 1),
            K::LargeBat => (Skirmisher, 0.8, 0.85, 0.8, 1.1, 0.4, 1),
            K::Minion => (Skirmisher, 0.35, 0.55, 0.4, 1.0, 0.3, 1),
            // The Garden's four: a draugr chops like a brute, the ravens peck, the bog slime spits.
            K::Skeleton => (Brute, 0.45, 0.6, 0.5, 1.1, 0.36, 1),
            K::Vampire => (Skirmisher, 0.6, 0.7, 0.6, 1.0, 0.32, 1),
            K::Bat => (Skirmisher, 0.45, 0.6, 0.5, 1.05, 0.3, 1),
            K::Slime => (Hexer, 0.8, 0.8, 0.8, 0.7, 0.38, 1),
            // The third stage's, stronger than the archetypes' own: two blades (the tengu quick on
            // his feet, the rasetsu heavy), two that bite (the dire wolf's jaws, the cobra's
            // fangs), and fire from afar (the jorogumo).
            K::Tengu => (Brute, 1.0, 1.15, 0.9, 1.5, 0.42, 2),
            K::Rasetsu => (Brute, 1.5, 1.2, 1.25, 1.0, 0.52, 2),
            K::DireWolf => (Skirmisher, 1.5, 1.2, 1.3, 1.1, 0.45, 1),
            K::Cobra => (Skirmisher, 1.25, 1.3, 1.0, 0.8, 0.4, 1),
            K::Jorogumo => (Hexer, 1.6, 1.2, 1.5, 1.0, 0.46, 1),
            // The fourth stage (the yōkai): the oni's club, the kappa's shove, the rokurokubi's
            // bite, the kitsune's foxfire.
            K::Oni => (Brute, 1.9, 1.35, 1.25, 1.0, 0.6, 2),
            K::Kappa => (Skirmisher, 1.9, 1.35, 1.4, 0.95, 0.42, 1),
            K::Rokurokubi => (Skirmisher, 1.7, 1.4, 1.2, 0.7, 0.4, 1),
            K::Kitsune => (Hexer, 2.0, 1.35, 1.5, 1.3, 0.38, 1),
            // The fifth (Anatolian nights): the karakoncolos's claws, the tülükuş's talons, the
            // albastı's weight, the cin's fire, the arçura's root darts.
            K::Karakoncolos => (Brute, 2.3, 1.35, 1.25, 1.15, 0.6, 2),
            K::Tulukus => (Skirmisher, 3.8, 1.5, 1.3, 1.1, 0.42, 1),
            K::Albasti => (Skirmisher, 3.4, 1.45, 1.4, 1.05, 0.42, 1),
            K::Cin => (Hexer, 2.6, 1.5, 1.6, 1.0, 0.38, 1),
            K::Arcura => (Hexer, 3.8, 1.45, 1.8, 0.95, 0.45, 1),
            // The sixth (the legends): two giants, the dragon, the ghost.
            K::Tepegoz => (Brute, 4.0, 1.7, 1.25, 0.85, 0.85, 2),
            K::Gashadokuro => (Brute, 3.0, 1.65, 1.25, 0.9, 0.9, 2),
            K::Ejder => (Skirmisher, 5.0, 1.6, 1.6, 0.75, 0.9, 2),
            K::Goryo => (Hexer, 4.2, 1.7, 2.0, 1.0, 0.4, 1),
            // The seventh (Ragnarök).
            K::FrostGiant => (Brute, 3.6, 1.9, 1.25, 0.9, 0.9, 2),
            K::Surtr => (Brute, 4.0, 2.0, 1.25, 0.85, 0.95, 2),
            K::Nidhogg => (Skirmisher, 5.0, 1.7, 1.6, 0.85, 0.8, 2),
            K::Hel => (Hexer, 4.0, 1.9, 2.0, 1.0, 0.42, 1),
            // The realms' own (REALMS.md), grown with the realm's tier as they are placed
            // (`logic::realms::weight`). The Peerless fights with his own kit (`game::realms::mirrors`).
            K::Peerless => (Brute, 1.6, 1.0, 1.25, 1.0, 0.4, 2),
            // Hel's servants wade in, slow and hard to stop.
            K::Ganglot => (Brute, 1.3, 0.9, 3.0, 0.4, 0.45, 1),
            // The gaki swarm.
            K::Gaki => (Skirmisher, 0.9, 0.95, 0.8, 1.05, 0.38, 1),
            // The elf-maids: hollow behind, and only there to be struck.
            K::Ellepige => (Skirmisher, 1.3, 1.1, 1.0, 1.0, 0.36, 1),
            // The courts' own: Mammon is the Prince himself; his golems, slow and heavy brutes.
            K::Mammon => (Prince, 1.0, 1.0, 1.0, 1.0, 0.95, 3),
            K::GoldGolem => (Brute, 0.9, 0.8, 1.2, 0.6, 0.55, 1),
            // The six other princes, each the Prince himself; what they raise, lighter than the golems.
            // Pride's court: Lucifer, and his reflections, swordsmen.
            K::Lucifer => (Prince, 1.0, 1.0, 1.0, 1.0, 0.95, 3),
            K::Reflection => (Brute, 0.7, 0.5, 0.9, 1.0, 0.5, 1),
            // Envy's court: Leviathan, and his brood, that bite.
            K::Leviathan => (Prince, 1.0, 1.0, 1.0, 1.0, 0.95, 3),
            K::Serpentling => (Skirmisher, 0.6, 0.7, 0.7, 1.0, 0.36, 1),
            // Wrath's court: Amon, and his ravens, that peck.
            K::Amon => (Prince, 1.3, 1.0, 1.0, 1.0, 0.95, 3),
            K::Raven => (Skirmisher, 0.45, 0.65, 0.5, 1.05, 0.3, 1),
            // Sloth's court: Belphegor, and his nightmares, dream-wisps that keep their distance and hex.
            K::Belphegor => (Prince, 1.0, 1.0, 1.0, 1.0, 0.95, 3),
            K::Nightmare => (Hexer, 1.4, 0.9, 1.0, 1.0, 0.36, 1),
            // Gluttony's court: Beelzebub, and his swarms.
            K::Beelzebub => (Prince, 1.0, 1.0, 1.0, 1.0, 0.95, 3),
            K::FlySwarm => (Skirmisher, 0.5, 0.6, 0.5, 1.0, 0.4, 1),
            // Lust's court: Asmodeus (his elf-maids are the realm's own).
            K::Asmodeus => (Prince, 1.0, 1.0, 1.0, 1.0, 0.95, 3),
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
            K::FrostGiant => (Rider::Frost, Power::Rime),
            K::Surtr => (Rider::None, Power::Fire),
            K::Nidhogg => (Rider::Venom(120), Power::None),
            K::Hel => (Rider::Drain, Power::Necromancy),
            K::Ellepige => (Rider::None, Power::Hollow),
            K::Mammon => (Rider::None, Power::GoldHide),
            K::Lucifer => (Rider::None, Power::Vainglory),
            K::Leviathan => (Rider::None, Power::Scales),
            K::Amon => (Rider::None, Power::Fury),
            K::Belphegor => (Rider::None, Power::Torpor),
            K::Beelzebub => (Rider::None, Power::Bloat),
            K::Asmodeus => (Rider::None, Power::Allure),
            _ => (Rider::None, Power::None),
        };
        Breed { kind, arch, hp, damage, poise, speed, radius, cost, reach, rider, power, phase: 1 }
    }

    /// The moves it knows, in the order it prefers them: its archetype's own, or its legend's.
    pub fn moves(&self) -> &'static [MoveId] {
        use EnemyKind as K;
        match self.kind {
            K::Oni => &[MoveId::Kanabo, MoveId::Sweep],
            K::Kappa => &[MoveId::Shove],
            K::Rokurokubi => &[MoveId::NeckBite],
            K::Kitsune => &[MoveId::Foxfire],
            K::Karakoncolos => &[MoveId::Call, MoveId::Slam, MoveId::Sweep],
            K::Tulukus => &[MoveId::Talons],
            K::Albasti => &[MoveId::Smother],
            K::Cin => &[MoveId::Smokeless],
            K::Arcura => &[MoveId::Roots],
            K::Gashadokuro => &[MoveId::Grab, MoveId::Slam, MoveId::Sweep],
            K::Ejder => &[MoveId::Heads],
            K::Goryo => &[MoveId::Thunder],
            K::FrostGiant => &[MoveId::FrostSlam, MoveId::Slam, MoveId::Sweep],
            K::Nidhogg => &[MoveId::Burrow, MoveId::Bite],
            K::Hel => &[MoveId::Raise, MoveId::Hex],
            // Pride's reflections cut as he does, left-handed.
            K::Reflection => &[MoveId::RadiantCut],
            _ if self.arch == Archetype::Prince => super::prince::table(self.kind, self.phase),
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

    /// The archetype's numbers as this body has them (a prince in his last phase quicker).
    pub fn stats(&self) -> Stats {
        let s = self.arch.stats();
        let (pace, rest) = if self.arch == Archetype::Prince { super::prince::pace(self.kind, self.phase) } else { (1.0, 1.0) };
        let speed = self.speed * pace;
        Stats {
            hp: s.hp * self.hp,
            poise: s.poise * self.poise,
            armor: s.armor * self.poise,
            walk: s.walk * speed,
            strafe: s.strafe * speed,
            retreat: s.retreat * speed,
            radius: self.radius,
            cost: self.cost,
            rest: (s.rest as f64 * rest).round() as u16,
            ..s
        }
    }

    /// A fresh fighter for it.
    pub fn fighter(&self) -> Fighter {
        let s = self.stats();
        Fighter::new(s.poise, s.armor, s.stagger, s.resolve)
    }
}

/// The Hexer backs off from a hero nearer than this.
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

    /// Every body the game has, on its archetype.
    fn every_breed() -> Vec<Breed> {
        (1..=EnemyKind::STAGES)
            .flat_map(|s| EnemyKind::of_stage(s).iter().copied())
            .chain([EnemyKind::Gaki, EnemyKind::Ellepige])
            .map(Breed::of)
            .collect()
    }

    #[test]
    fn a_body_foe_only_bites_from_beside_him_and_only_a_distance_foe_strikes_from_afar() {
        use super::super::frames::{Shape, BITE_RANGE};
        for b in every_breed() {
            for &mv in b.moves() {
                let d = mv.data();
                if !d.strikes() {
                    continue;
                }
                // (A big body's reach is its arm and its girth, not a throw.)
                let far = d.range.1 + b.reach;
                match b.attack() {
                    // Teeth, claws, a shove, its weight: from beside him (a burrow comes up under him).
                    Attack::Body if matches!(d.shape, Shape::Mark { .. }) => assert_eq!(mv, MoveId::Burrow),
                    Attack::Body => {
                        assert!(!d.projectile && far <= BITE_RANGE + 0.1 + b.reach + 1e-9, "{:?} {}: from {far}", b.kind, d.name);
                        assert!(d.reach <= 1.4 && d.range.0 == 0.0, "{:?} {}", b.kind, d.name);
                    }
                    // A weapon at arm's length (the ground shaking round the club is close too).
                    Attack::Melee => assert!(!d.projectile && far <= 2.0 + b.reach + 1e-9, "{:?} {}: from {far}", b.kind, d.name),
                    // From afar, only by bolts.
                    Attack::Distance => assert!(d.projectile, "{:?} {} is not a bolt", b.kind, d.name),
                }
            }
        }
    }

    #[test]
    fn every_distance_foes_bolt_has_its_own_look() {
        use super::super::duelist::BoltLook;
        let mut seen: Vec<(EnemyKind, BoltLook)> = Vec::new();
        for b in every_breed().into_iter().filter(|b| b.attack() == Attack::Distance) {
            let look = BoltLook::of(b.kind);
            assert_ne!(look, BoltLook::Blade, "{:?}", b.kind);
            if let Some(&(other, _)) = seen.iter().find(|s| s.1 == look) {
                panic!("{:?} and {other:?} loose the same bolt", b.kind);
            }
            assert!(seen.iter().all(|s| s.1.tint() != look.tint()), "{:?}: a colour of its own", b.kind);
            seen.push((b.kind, look));
        }
        assert_eq!(seen.len(), 8, "the slime, the witch, the jorogumo, the kitsune, the cin, the arçura, the goryō, Hel");
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
