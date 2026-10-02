//! The hero's kit in frame data (COMBAT.md §8, phase 3): his blade, his shield and his path as
//! numbers on the same three verbs. A blade sets how hard, how far, how wide and how quick the
//! string is (and whether its finisher sweeps all round or is thrown); a shield sets the parry
//! window, what a block costs and what a parry does back; the path adds what fits the state
//! machine and leaves out what bypasses it (the audit in COMBAT.md §8).
//!
//! The training sword of §1.1 is Ragnar's first blade with his first shield and no path, so a
//! fresh hero fights exactly the phase-1 table (`Kit::of(Ragnar, 1, 1.0, Some(1), 0) == Kit::training()`).

use super::frames::{MoveData, MoveId, REFLECT_DAMAGE};
use super::fsm::{DEFLECT, EVADE_LAND, GUARD_BREAK, GUARD_BREAK_SHARE, PARRY_FROM, PARRY_TO};
use super::math;
use crate::logic::heroes::HeroKind;
use crate::logic::progression::{self, ARROW_RANGE};
use std::f64::consts::PI;

/// The training sword's blade: the reach, spread and cooldown the table of §1.1 is drawn for.
const BASE_REACH: f64 = 1.2;
const BASE_SPREAD: f64 = 0.7;
const BASE_COOLDOWN: f64 = 0.42;
/// A blade's cooldown counts toward its weight up to this: beyond it, the wait paid for a gift the
/// ring cuts (a sun-blade's flash, a bow's volley).
const MAX_COOLDOWN: f64 = 0.6;
/// Each tier above the first makes a blade this much quicker in the hand.
const MASTERY: f64 = 0.02;
/// Tempo, reach and arc stay inside these bounds whatever the blade.
const TEMPO: (f64, f64) = (0.7, 1.25);
const MAX_REACH: f64 = 0.9;
/// Poise damage grows a little with each tier, and to at most this multiple: a light blow never
/// beats the Brute's armor (35) on its own; only a piercing blade's does.
const POISE_PER_TIER: f64 = 0.04;
const MAX_POISE: f64 = 1.5;
/// A stunning blade (a stun, a flash) breaks this much harder.
const STUNNING: f64 = 1.15;
/// The share of the hero's might the ring counts (the janissary's real edge is his health), and
/// the janissary's heavier hand on the tempo of any blade.
const MIGHT_SHARE: f64 = 0.25;
const JANISSARY_TEMPO: f64 = 1.08;
/// A blade whose gift the ring cuts (a trick, a bleed, a burn, venom, lightning, stealth, a fire
/// trail) bites this much harder instead, so no tier is worth less in the ring for its gift.
const FOLDED_GIFT: f64 = 1.1;
const ARC: (f64, f64) = (-0.26, 0.5);

/// A finisher that flies: tiles per second and how far.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Thrown {
    pub speed: f64,
    pub range: f64,
}

/// What the blade (and the hand that holds it) makes of the string.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Strike {
    /// Damage scalar: the tier, the edge, the hero's might.
    pub power: f64,
    /// Poise damage scalar: the tier and the blade's weight.
    pub poise: f64,
    /// Startup and recovery scale (under 1 is quicker): the blade's weight, its tier, the hero's tempo, his
    /// rage. A swing's stamina scales with it too, so a quick blade buys more blows from the same breath.
    pub tempo: f64,
    /// Tiles added to every hit's reach, and to the finisher's on top.
    pub reach: f64,
    pub finisher_reach: f64,
    /// The finisher's damage multiple (a blinding flash lands half again).
    pub finisher_power: f64,
    /// Radians added to every hit's half-angle.
    pub arc: f64,
    /// The finisher sweeps all round him (a whirl, a blinding flash).
    pub whirl: bool,
    /// The finisher is thrown down his aim instead of swung (an axe, a spear, a bow).
    pub thrown: Option<Thrown>,
    /// Poise added to his blows' armor checks (not to their poise damage): armor-piercing.
    pub pierce: f64,
    /// Added to the multiplier on an opening (staggered, deflected, recovering): critical blows.
    pub crit: f64,
    /// A foe struck at least `back_from` radians off its facing takes this many times the damage.
    pub back: f64,
    pub back_from: f64,
    /// Health each blow that lands costs him (a cursed blade).
    pub blood: f64,
}

/// What the shield on his arm (or his bare hands, without one) makes of the guard.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Guard {
    /// Frames of the parry window, from the guard's second frame.
    pub parry: u16,
    /// Stamina a block costs per point of the blow's damage.
    pub cost: f64,
    /// Poise a parry takes from the attacker on top of the deflect's own.
    pub riposte: f64,
    /// Frames a parried attacker reels.
    pub deflect: u16,
    /// Damage a parried bolt does to the foes, as a multiple of its own.
    pub reflect: f64,
    /// A bolt caught on the guard costs no stamina.
    pub bolt_proof: bool,
    /// A broken guard lets this share of the blow through, and reels him this many frames.
    pub break_share: f64,
    pub break_frames: u16,
    /// Health a parry gives him back, and damage it does to the attacker.
    pub aegis: f64,
    pub thorns: f64,
}

/// The whole kit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kit {
    pub strike: Strike,
    pub guard: Guard,
    /// Frames on landing from the evade before he can act.
    pub evade_land: u16,
    /// What the blows he takes are multiplied by.
    pub taken: f64,
    /// Once a bout, the blow that would end it leaves him this share of his health instead.
    pub second_wind: f64,
    /// A kill shakes the foes around it: they lose half their poise.
    pub terror: bool,
}

impl Strike {
    /// The training sword in the hand of no one in particular.
    pub const TRAINING: Strike = Strike {
        power: 1.0,
        poise: 1.0,
        tempo: 1.0,
        reach: 0.0,
        finisher_reach: 0.0,
        finisher_power: 1.0,
        arc: 0.0,
        whirl: false,
        thrown: None,
        pierce: 0.0,
        crit: 0.0,
        back: 1.0,
        back_from: PI,
        blood: 0.0,
    };

    /// `hero`'s blade of `tier` at `edge` (0 to 1), with his path at `path`.
    pub fn of(hero: HeroKind, tier: u8, edge: f64, path: u8) -> Self {
        let tier = tier.clamp(1, progression::TIERS);
        let b = progression::blade(hero, tier);
        // A heavier blade (a longer cooldown) swings slower and breaks more; a finer one is quicker in the hand.
        let weight = (b.cooldown.min(MAX_COOLDOWN) / BASE_COOLDOWN).sqrt();
        let mastery = 1.0 - MASTERY * (tier - 1) as f64;
        let rage = if path >= 1 { progression::path::RAGE_COOLDOWN.sqrt() } else { 1.0 };
        let style = if hero == HeroKind::Mehmed { JANISSARY_TEMPO } else { 1.0 };
        let tempo = (weight * mastery * hero.tempo().sqrt() * rage * style).clamp(TEMPO.0, TEMPO.1);
        let stunning = if b.stun > 0.0 || b.flash > 0.0 { STUNNING } else { 1.0 };
        let cut = b.trick || b.bleed > 0.0 || b.burn > 0.0 || b.poison || b.chain > 0 || b.stealth > 0.0 || b.fire_trail;
        let (reach, arc, whirl) = match b.swing {
            Some((r, w, behind)) => {
                let spread = w.atan2(r) - BASE_SPREAD.atan2(BASE_REACH);
                ((r - BASE_REACH).clamp(0.0, MAX_REACH), spread.clamp(ARC.0, ARC.1), behind >= 1.0)
            }
            // A thrown weapon or a bow still swings its first two blows, a little longer for its haft.
            None => (0.3, 0.0, false),
        };
        let thrown = b
            .throw
            .map(|t| Thrown { speed: t.speed, range: t.range.min(9.0) })
            .or((b.arrows > 0).then_some(Thrown { speed: 16.0, range: ARROW_RANGE }));
        let seer = path >= 3 && hero != HeroKind::Miyamoto;
        // The fang (Ragnar's level 6) bites from behind; the others' flanking blow comes from the side too.
        let (back, back_from) = match (path >= 6, hero) {
            (false, _) => (1.0, PI),
            (true, HeroKind::Ragnar) => (1.5, 2.0 * PI / 3.0),
            (true, _) => (1.25, PI / 3.0),
        };
        Strike {
            power: math::power(tier, edge) * hero.might().powf(MIGHT_SHARE) * if cut { FOLDED_GIFT } else { 1.0 },
            poise: ((1.0 + POISE_PER_TIER * (tier - 1) as f64) * weight * stunning).min(MAX_POISE),
            tempo,
            reach,
            finisher_reach: if b.wave { 1.0 } else { 0.0 },
            finisher_power: if b.flash > 0.0 { 1.5 } else { 1.0 },
            arc,
            whirl: whirl || b.flash > 0.0,
            thrown,
            pierce: if b.pierce_armor { 15.0 } else { 0.0 },
            crit: if b.crit { 0.2 } else { 0.0 } + if seer { 0.1 } else { 0.0 },
            back,
            back_from,
            blood: b.blood as f64,
        }
    }
}

impl Guard {
    /// No shield: a bare-handed guard, a short parry and a block that costs the whole blow.
    pub const BARE: Guard = Guard {
        parry: 5,
        cost: 1.0,
        riposte: 0.0,
        deflect: DEFLECT,
        reflect: REFLECT_DAMAGE,
        bolt_proof: false,
        break_share: GUARD_BREAK_SHARE,
        break_frames: GUARD_BREAK,
        aegis: 0.0,
        thorns: 0.0,
    };

    /// The training buckler: the guard of §1.2.
    pub const TRAINING: Guard = Guard { parry: PARRY_TO - PARRY_FROM + 1, cost: super::fsm::BLOCK_COST, ..Guard::BARE };

    /// A shield of `tier`: each tier keeps every gift of the tiers below it.
    pub fn of(tier: u8) -> Self {
        let tier = tier.clamp(1, progression::TIERS);
        let w = progression::ward(tier);
        let first = progression::ward(1).block;
        let last = progression::ward(progression::TIERS).block;
        let (break_share, break_frames) = if w.unbreakable { (0.25, GUARD_BREAK / 2) } else { (GUARD_BREAK_SHARE, GUARD_BREAK) };
        Guard {
            parry: Guard::TRAINING.parry + (tier as u16 - 1) / 3,
            // The share of a blow it takes: from the buckler's 0.7 stamina a point down to 0.3.
            cost: Guard::TRAINING.cost - 0.4 * (w.block - first) / (last - first),
            riposte: if w.bash { 10.0 } else { 0.0 },
            deflect: DEFLECT + if w.stagger { 20 } else { 0 },
            reflect: if w.reflect { 2.0 } else { REFLECT_DAMAGE },
            bolt_proof: w.missile_proof,
            break_share,
            break_frames,
            aegis: w.aegis as f64,
            thorns: w.thorns as f64 * 5.0,
        }
    }

    /// The last guard frame on which a blow is parried.
    pub fn parry_to(&self) -> u16 {
        PARRY_FROM + self.parry - 1
    }
}

impl Kit {
    /// The kit of COMBAT.md §1: the training sword, the training buckler, no path.
    pub fn training() -> Self {
        Kit { strike: Strike::TRAINING, guard: Guard::TRAINING, evade_land: EVADE_LAND, taken: 1.0, second_wind: 0.0, terror: false }
    }

    /// `hero` with his blade of `blade` at `edge`, his shield of `shield` (none: bare hands) and his path at `path`.
    pub fn of(hero: HeroKind, blade: u8, edge: f64, shield: Option<u8>, path: u8) -> Self {
        // Wind Step (Miyamoto's level 3) lands him light; Bifrost Step (level 5) lets him act the moment he touches down.
        let evade_land = if path >= 5 {
            0
        } else if path >= 3 && hero == HeroKind::Miyamoto {
            2
        } else {
            EVADE_LAND
        };
        // The second wind: Ragnar's Valkyrie Call (9), the others' last stand (8), and the World Tree (10) for all.
        let second_wind = match hero {
            HeroKind::Ragnar if path >= 9 => 0.5,
            HeroKind::Mehmed | HeroKind::Miyamoto if path >= 8 => 0.3,
            _ => 0.0,
        } + if path >= 10 { 0.25 } else { 0.0 };
        Kit {
            strike: Strike::of(hero, blade, edge, path),
            guard: shield.map(Guard::of).unwrap_or(Guard::BARE),
            evade_land,
            taken: if path >= 1 && hero == HeroKind::Ragnar { progression::path::RAGE_TAKEN } else { 1.0 },
            second_wind,
            terror: path >= 8 && hero == HeroKind::Ragnar,
        }
    }

    /// The string as this kit swings it: the table of §1.1 scaled (frames by tempo, damage by
    /// power, poise by weight, reach and arc widened), the finisher swept or thrown.
    pub fn moves(&self) -> [MoveData; 3] {
        let s = &self.strike;
        MoveId::HERO.map(|mv| {
            let mut d = mv.data();
            d.startup = ((d.startup as f64 * s.tempo).round() as u16).max(3);
            d.recovery = ((d.recovery as f64 * s.tempo).round() as u16).max(8);
            d.cancel = ((d.cancel as f64 * s.tempo).round() as u16).min(d.recovery - 1);
            d.damage *= s.power;
            d.poise *= s.poise;
            d.reach += s.reach;
            d.half_angle = (d.half_angle + s.arc).max(0.3);
            if mv == MoveId::L3 {
                d.reach += s.finisher_reach;
                d.damage *= s.finisher_power;
                if s.whirl {
                    d.half_angle = PI;
                }
                if s.thrown.is_some() {
                    d.projectile = true;
                }
            }
            d
        })
    }

    /// Stamina a swing costs with this kit.
    pub fn swing_cost(&self) -> f64 {
        progression::STAMINA_PER_BLOW * self.strike.tempo
    }

    /// The string's damage per second, flat out (each hit at its proration in the combo).
    pub fn dps(&self) -> f64 {
        let [l1, l2, l3] = self.moves();
        let frames = l1.cancel_frame() + l2.cancel_frame() + l3.total();
        (l1.damage * math::proration(1) + l2.damage * math::proration(2) + l3.damage * math::proration(3)) / (frames as f64 / 60.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::heroes::ROSTER;

    #[test]
    fn a_fresh_ragnar_fights_the_training_table() {
        assert_eq!(Kit::of(HeroKind::Ragnar, 1, 1.0, Some(1), 0), Kit::training());
        let table = Kit::training().moves();
        for (k, mv) in MoveId::HERO.into_iter().enumerate() {
            assert_eq!(table[k], mv.data(), "{mv:?}");
        }
    }

    #[test]
    fn every_kit_stays_inside_the_tables_bounds() {
        for hero in ROSTER {
            for tier in 1..=progression::TIERS {
                for path in [0, 5, 10] {
                    let k = Kit::of(hero, tier, 1.0, Some(tier), path);
                    for d in k.moves() {
                        assert!(d.startup >= 3 && d.recovery >= 8, "{hero:?} {tier}: {d:?}");
                        assert!(d.cancel < d.recovery, "{hero:?} {tier}: a cancel inside the recovery");
                        assert!(d.reach <= 1.7 + MAX_REACH + 1.0 + 1e-9, "{hero:?} {tier}: reach {}", d.reach);
                        assert!(d.damage > 0.0 && d.poise > 0.0);
                        if !d.projectile && d.name != "L3" {
                            assert!(d.poise < 35.0, "{hero:?} {tier}: a light blow beats the Brute's armor by poise alone");
                        }
                    }
                    assert!(k.guard.cost > 0.0, "{hero:?} {tier}: no free block");
                    assert!(k.guard.parry_to() < 15, "{hero:?} {tier}: a parry window {} wide", k.guard.parry);
                }
            }
        }
    }

    #[test]
    fn a_better_blade_strikes_harder_and_a_better_shield_guards_better() {
        for hero in ROSTER {
            let dps: Vec<f64> = (1..=progression::TIERS).map(|t| Kit::of(hero, t, 1.0, Some(t), 0).dps()).collect();
            assert!(dps[9] > dps[0] * 1.8, "{hero:?}: tier 10 {:.1} vs tier 1 {:.1}", dps[9], dps[0]);
            for t in 1..10 {
                assert!(dps[t] >= dps[t - 1] * 0.85, "{hero:?}: tier {} drops to {:.1} from {:.1}", t + 1, dps[t], dps[t - 1]);
            }
        }
        let guards: Vec<Guard> = (1..=progression::TIERS).map(Guard::of).collect();
        for t in 1..10 {
            assert!(guards[t].parry >= guards[t - 1].parry && guards[t].cost <= guards[t - 1].cost);
        }
        assert_eq!(guards[0], Guard::TRAINING);
        // A shield is worth carrying.
        const { assert!(Guard::BARE.parry < Guard::TRAINING.parry && Guard::BARE.cost > Guard::TRAINING.cost) };
    }

    #[test]
    fn a_rusted_blade_bites_less_and_the_path_adds_what_fits() {
        let keen = Kit::of(HeroKind::Ragnar, 5, 1.0, Some(5), 0);
        let rusted = Kit::of(HeroKind::Ragnar, 5, 0.0, Some(5), 0);
        assert!((rusted.strike.power / keen.strike.power - 0.4).abs() < 1e-9);
        // Rage quickens him; the World Tree and Valkyrie Call give him a second wind; Bifrost lands him ready.
        let raged = Kit::of(HeroKind::Ragnar, 1, 1.0, Some(1), 1);
        assert!(raged.strike.tempo < 1.0 && raged.taken > 1.0);
        let full = Kit::of(HeroKind::Ragnar, 1, 1.0, Some(1), 10);
        assert_eq!((full.second_wind, full.evade_land, full.terror), (0.75, 0, true));
        // A thrown weapon throws its finisher; a whirl sweeps all round.
        let axe = Kit::of(HeroKind::Ragnar, 2, 1.0, None, 0);
        assert!(axe.strike.thrown.is_some() && axe.moves()[2].projectile && !axe.moves()[0].projectile);
        let whirl = Kit::of(HeroKind::Ragnar, 5, 1.0, None, 0);
        assert_eq!(whirl.moves()[2].half_angle, PI);
        // The samurai is quickest, the janissary hits hardest.
        let (r, m, y) = (
            Kit::of(HeroKind::Ragnar, 1, 1.0, None, 0),
            Kit::of(HeroKind::Mehmed, 1, 1.0, None, 0),
            Kit::of(HeroKind::Miyamoto, 1, 1.0, None, 0),
        );
        assert!(y.strike.tempo < r.strike.tempo && m.strike.power > r.strike.power);
    }
}
