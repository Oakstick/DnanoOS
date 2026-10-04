//! Packs (WARBAND.md §7): the foes of a den as one band, each with a job, a leader to follow and a
//! heart that can break.
//!
//! A foe's archetype says how it attacks; its **role** says what it does for the pack. The leader
//! picks a **play** every half second, and a play only asks: where a waiting foe would like to
//! stand, whether it asks for a token, and how eagerly. The threat manager still decides who
//! strikes and when, so no play can bring two strikes down inside the reaction window.
//!
//! Morale falls as the pack is hurt; at nothing, a pack breaks by its temperament: the cowardly
//! flee to their den, the stalwart fight on, the frenzied quicken. Either way the fight has turned.

use super::WarEvent;
use crate::logic::combat::archetype::{Attack, Breed};
use crate::logic::combat::fsm::Effect;
use crate::logic::enemy::EnemyKind;
use crate::logic::rng::Rng;
use crate::logic::types::{PotionKind, V};
use std::f64::consts::{FRAC_PI_2, PI};

/// What a foe does for its pack.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    /// A brute: holds the front, screens the casters.
    Vanguard,
    /// A biter: presses his sides.
    Harrier,
    /// A caster: punishes him for standing still.
    Caller,
}

impl Role {
    pub fn of(kind: EnemyKind) -> Role {
        match Breed::of(kind).attack() {
            Attack::Melee => Role::Vanguard,
            Attack::Body => Role::Harrier,
            Attack::Distance => Role::Caller,
        }
    }
}

/// How a pack takes being broken.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Temperament {
    /// Flees to its den.
    Cowardly,
    /// Fights on.
    Stalwart,
    /// Quickens: walks faster and rests less after its moves.
    Frenzied,
}

impl Temperament {
    pub fn of(kind: EnemyKind) -> Temperament {
        use EnemyKind as K;
        match kind {
            K::Bat | K::Vampire | K::LargeBat | K::Minion | K::Cin | K::Kitsune | K::Witch | K::Slime | K::Arcura => Temperament::Cowardly,
            K::Werewolf | K::Rasetsu | K::DireWolf | K::Oni | K::Cobra | K::Kappa => Temperament::Frenzied,
            _ => Temperament::Stalwart,
        }
    }
}

/// What a pack's leader carries beyond its body (one from tier 3, two from tier 6).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Affix {
    /// +15 armor.
    Ironhide,
    /// Its pack's poise comes back half again as fast near it.
    Bannered,
    /// When one of its pack falls it takes heart: 20 poise, and it rests less for 4 s.
    Vengeful,
    /// Casters near it take the bolts he turns back on them at half.
    Warding,
    /// Its spoils come twice over; it flees at 30% health.
    Gilded,
}

impl Affix {
    pub const ALL: [Affix; 5] = [Affix::Ironhide, Affix::Bannered, Affix::Vengeful, Affix::Warding, Affix::Gilded];

    pub fn name(self) -> &'static str {
        match self {
            Affix::Ironhide => "IRONHIDE",
            Affix::Bannered => "BANNERED",
            Affix::Vengeful => "VENGEFUL",
            Affix::Warding => "WARDING",
            Affix::Gilded => "GILDED",
        }
    }
}

/// Armor an Ironhide leader adds (the finisher's 45 poise still flinches every breed's body).
pub const IRONHIDE_ARMOR: f64 = 15.0;
/// A Bannered leader's reach, and the poise it gives back a frame (as a share of the max).
pub const BANNER_RADIUS: f64 = 4.0;
pub const BANNER_REGEN: f64 = 1.0 / 180.0;
/// A Vengeful leader's heart when one of its pack falls.
pub const VENGEANCE_POISE: f64 = 20.0;
pub const VENGEANCE_FRAMES: u16 = 240;
/// A Warding leader's reach, and what a turned-back bolt deals a caster in it.
pub const WARD_RADIUS: f64 = 3.0;
pub const WARD_SHARE: f64 = 0.5;
/// A Gilded leader flees below this share of its health.
pub const GILDED_FLEE: f64 = 0.3;

/// The pack's leader picks a play this often (frames).
pub const THINK_EVERY: u16 = 30;
/// Morale: whole, and what each blow to the pack takes from it.
pub const MORALE: f64 = 100.0;
pub const LEADER_SLAIN: f64 = 35.0;
pub const ALLY_SLAIN: f64 = 20.0;
pub const BROKEN_SEEN: f64 = 10.0;
pub const RALLIED: f64 = 10.0;
/// Morale a cowardly pack loses when he drinks a potion of strength or Ragnarok in its sight.
pub const DREAD_DRINK: f64 = 15.0;
/// Frames a cowardly pack flees, and a frenzied one quickens; how much it quickens.
pub const ROUT_FRAMES: u16 = 180;
pub const FRENZY_FRAMES: u16 = 360;
pub const FRENZY_PACE: f64 = 1.15;
/// A foe below this share of its health falls back (Rotate) while another takes its place; how far
/// back, and for how long it keeps out of the fight.
pub const ROTATE_AT: f64 = 0.3;
pub const ROTATE_BACK: f64 = 1.5;
pub const ROTATE_FRAMES: u16 = 240;
/// A caller asks more eagerly while he hides behind his shield.
pub const CALLER_EAGER: f64 = 0.4;
/// The rally: the leader below this share of its health with this many of its pack standing; what
/// it gives back (a share of each one's poise), and its reach.
pub const RALLY_AT: f64 = 0.6;
pub const RALLY_ALLIES: usize = 2;
pub const RALLY_POISE: f64 = 0.3;
pub const RALLY_RADIUS: f64 = 6.0;
/// Frames before a pack may rally again.
pub const RALLY_REST: u16 = 600;
/// The hero within this of a caller: a vanguard screens it.
pub const SCREEN_NEAR: f64 = 3.5;

/// What the leader has the pack do.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub enum Play {
    /// Each to its own sector (the threat manager's sectors as they are).
    #[default]
    Hold,
    /// Two biters take his two sides.
    Pincer,
    /// A brute stands between him and a caster.
    Screen,
    /// The leader sounds its rally.
    Rally,
}

impl Play {
    pub fn name(self) -> &'static str {
        match self {
            Play::Hold => "HOLD",
            Play::Pincer => "PINCER",
            Play::Screen => "SCREEN",
            Play::Rally => "RALLY",
        }
    }
}

/// One foe of a pack.
#[derive(Clone, Copy, Debug)]
pub struct Member {
    pub id: u32,
    pub kind: EnemyKind,
    pub role: Role,
    /// Frames it keeps out of the fight, fallen back to heal its nerve (Rotate).
    pub resting: u16,
    pub alive: bool,
}

/// A foe of the pack as the leader sees it this frame.
#[derive(Clone, Copy, Debug)]
pub struct Seen {
    pub id: u32,
    pub pos: V,
    /// Its health as a share of its whole.
    pub hp: f64,
    pub engaged: bool,
    /// Free to act: neutral, holding no token, rested.
    pub free: bool,
}

/// What a foe of the pack is asked to do this frame (all of it a wish the threat manager and the
/// sectors may refuse).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Directive {
    /// The bearing round the hero it would like to wait at (radians, world).
    pub want: Option<f64>,
    /// Tiles added to its waiting ring.
    pub back: f64,
    /// It asks for a token at all.
    pub ask: bool,
    /// Added to its request's score.
    pub bias: f64,
}

impl Default for Directive {
    fn default() -> Self {
        Self { want: None, back: 0.0, ask: true, bias: 0.0 }
    }
}

/// What came of the war log for a pack.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PackCue {
    /// Its morale broke: it routs (cowardly), quickens (frenzied) or fights on (stalwart).
    Broke(Temperament),
}

/// A den's foes.
#[derive(Clone, Debug)]
pub struct Pack {
    /// Its den's corners (the key the duelists carry).
    pub lo: V,
    pub hi: V,
    pub leader: u32,
    pub members: Vec<Member>,
    pub affixes: Vec<Affix>,
    pub morale: f64,
    pub play: Play,
    pub temperament: Temperament,
    /// Frames left fleeing (cowardly), quickened (frenzied), and of a vengeful leader's heart.
    pub rout: u16,
    pub frenzy: u16,
    pub vengeance: u16,
    /// It has broken once (a pack breaks only once).
    pub broken: bool,
    /// Frames before its leader may rally again.
    pub rally_rest: u16,
    think: u16,
}

/// Composes a pack: roles by archetype, the toughest its leader, its affixes by the realm's tier,
/// and the temperament most of its foes share. `foes` are (id, kind, health).
pub fn compose(lo: V, hi: V, foes: &[(u32, EnemyKind, f64)], tier: usize, seed: u32) -> Pack {
    let leader = foes.iter().max_by(|a, b| a.2.total_cmp(&b.2).then(b.0.cmp(&a.0))).map(|f| f.0).unwrap_or(0);
    let members: Vec<Member> =
        foes.iter().map(|&(id, kind, _)| Member { id, kind, role: Role::of(kind), resting: 0, alive: true }).collect();
    let mut rng = Rng::new(seed);
    let n = if tier >= 6 {
        2
    } else if tier >= 3 {
        1
    } else {
        0
    };
    let mut affixes = Vec::new();
    while affixes.len() < n.min(foes.len().max(1)) {
        let a = Affix::ALL[rng.index(Affix::ALL.len())];
        if !affixes.contains(&a) {
            affixes.push(a);
        }
    }
    let count = |t: Temperament| members.iter().filter(|m| Temperament::of(m.kind) == t).count();
    let temperament = [Temperament::Cowardly, Temperament::Frenzied, Temperament::Stalwart]
        .into_iter()
        .max_by_key(|&t| (count(t), t == Temperament::Stalwart))
        .unwrap_or(Temperament::Stalwart);
    Pack {
        lo,
        hi,
        leader,
        members,
        affixes,
        morale: MORALE,
        play: Play::Hold,
        temperament,
        rout: 0,
        frenzy: 0,
        vengeance: 0,
        broken: false,
        rally_rest: 0,
        think: 0,
    }
}

impl Pack {
    pub fn has(&self, id: u32) -> bool {
        self.members.iter().any(|m| m.id == id)
    }

    pub fn has_affix(&self, a: Affix) -> bool {
        self.affixes.contains(&a)
    }

    pub fn standing(&self) -> usize {
        self.members.iter().filter(|m| m.alive).count()
    }

    /// The war log, as the pack hears it. `near`: he is in its sight (it is awake).
    pub fn hear(&mut self, e: &WarEvent, near: bool) -> Option<PackCue> {
        match *e {
            WarEvent::Slain { foe, .. } => {
                let m = self.members.iter_mut().find(|m| m.id == foe)?;
                m.alive = false;
                if self.has_affix(Affix::Vengeful) && foe != self.leader {
                    self.vengeance = VENGEANCE_FRAMES;
                }
                self.lose(if foe == self.leader { LEADER_SLAIN } else { ALLY_SLAIN })
            }
            WarEvent::HeroHit { foe, effect: Effect::Broken, .. } if self.has(foe) => self.lose(BROKEN_SEEN),
            WarEvent::Drank { potion: PotionKind::Strength | PotionKind::Ragnarok }
                if near && self.temperament == Temperament::Cowardly =>
            {
                self.lose(DREAD_DRINK)
            }
            _ => None,
        }
    }

    /// Its leader's rally was sounded.
    pub fn rallied(&mut self) {
        self.morale = (self.morale + RALLIED).min(MORALE);
    }

    fn lose(&mut self, amount: f64) -> Option<PackCue> {
        self.morale = (self.morale - amount).max(0.0);
        if self.morale > 0.0 || self.broken || self.standing() == 0 {
            return None;
        }
        self.broken = true;
        match self.temperament {
            Temperament::Cowardly => self.rout = ROUT_FRAMES,
            Temperament::Frenzied => self.frenzy = FRENZY_FRAMES,
            Temperament::Stalwart => {}
        }
        Some(PackCue::Broke(self.temperament))
    }

    /// One frame: its clocks, and (every `THINK_EVERY` frames) the leader's play. `seen` holds its
    /// standing foes; `hero` and `aim` where he stands and faces; `blocking` whether his shield is
    /// up. Returns each member's directive, and whether the leader should sound its rally now.
    pub fn frame(&mut self, seen: &[Seen], hero: V, aim: V, blocking: bool) -> (Vec<(u32, Directive)>, bool) {
        self.rout = self.rout.saturating_sub(1);
        self.frenzy = self.frenzy.saturating_sub(1);
        self.vengeance = self.vengeance.saturating_sub(1);
        self.rally_rest = self.rally_rest.saturating_sub(1);
        for m in self.members.iter_mut() {
            m.resting = m.resting.saturating_sub(1);
        }
        let find = |id: u32| seen.iter().find(|s| s.id == id);
        // Rotate: a wounded foe falls back while another fit one of the pack can take its place.
        let fit = seen.iter().filter(|s| s.engaged && s.hp >= ROTATE_AT).count();
        for m in self.members.iter_mut() {
            if let Some(s) = seen.iter().find(|s| s.id == m.id) {
                if m.alive && m.id != self.leader && s.engaged && s.free && s.hp < ROTATE_AT && fit >= 1 && m.resting == 0 {
                    m.resting = ROTATE_FRAMES;
                }
            }
        }
        let mut rally = false;
        if self.think == 0 {
            self.think = THINK_EVERY;
            self.play = self.choose(seen, hero);
            if self.play == Play::Rally {
                rally = find(self.leader).is_some_and(|l| l.free);
                if rally {
                    self.rally_rest = RALLY_REST;
                }
            }
        }
        self.think -= 1;
        let side = |sign: f64| aim.y.atan2(aim.x) + sign * FRAC_PI_2;
        let mut harriers = 0;
        let mut out = Vec::new();
        for m in self.members.iter().filter(|m| m.alive) {
            let Some(s) = find(m.id) else { continue };
            let mut d = Directive::default();
            if self.rout > 0 {
                d.ask = false;
            }
            if m.resting > 0 {
                d.ask = false;
                d.back = ROTATE_BACK;
            }
            match (self.play, m.role) {
                (Play::Pincer, Role::Harrier) => {
                    d.want = Some(side(if harriers % 2 == 0 { 1.0 } else { -1.0 }));
                    harriers += 1;
                }
                (Play::Screen, Role::Vanguard) => {
                    if let Some(c) = self.screened(seen, hero) {
                        d.want = Some((c.y - hero.y).atan2(c.x - hero.x));
                    }
                }
                _ => {}
            }
            if m.role == Role::Caller && blocking {
                d.bias += CALLER_EAGER;
            }
            let _ = s;
            out.push((m.id, d));
        }
        (out, rally)
    }

    /// The caster he is closing on, if a brute of the pack could screen it.
    fn screened(&self, seen: &[Seen], hero: V) -> Option<V> {
        let callers = self.members.iter().filter(|m| m.alive && m.role == Role::Caller);
        callers
            .filter_map(|m| seen.iter().find(|s| s.id == m.id && s.engaged))
            .map(|s| s.pos)
            .find(|p| ((p.x - hero.x).powi(2) + (p.y - hero.y).powi(2)).sqrt() < SCREEN_NEAR)
    }

    fn choose(&self, seen: &[Seen], hero: V) -> Play {
        let alive =
            |r: Role| self.members.iter().filter(|m| m.alive && m.role == r && seen.iter().any(|s| s.id == m.id && s.engaged)).count();
        let leader = seen.iter().find(|s| s.id == self.leader);
        if let Some(l) = leader {
            if l.engaged && l.hp < RALLY_AT && self.standing() > RALLY_ALLIES && self.rout == 0 && self.rally_rest == 0 {
                return Play::Rally;
            }
        }
        if alive(Role::Vanguard) >= 1 && self.screened(seen, hero).is_some() {
            return Play::Screen;
        }
        if alive(Role::Harrier) >= 2 {
            return Play::Pincer;
        }
        Play::Hold
    }
}

/// Wraps an angle into (-π, π].
pub fn wrap(a: f64) -> f64 {
    let mut a = (a + PI).rem_euclid(2.0 * PI) - PI;
    if a <= -PI {
        a += 2.0 * PI;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen(id: u32, x: f64, y: f64, hp: f64) -> Seen {
        Seen { id, pos: V::new(x, y), hp, engaged: true, free: true }
    }

    fn pack(kinds: &[EnemyKind], tier: usize) -> Pack {
        let foes: Vec<(u32, EnemyKind, f64)> = kinds.iter().enumerate().map(|(i, &k)| (i as u32 + 1, k, Breed::of(k).hp)).collect();
        compose(V::new(0.0, 0.0), V::new(8.0, 8.0), &foes, tier, 7)
    }

    #[test]
    fn a_pack_has_its_roles_its_leader_and_its_tiers_affixes() {
        use EnemyKind as K;
        let p = pack(&[K::Blemmyes, K::Werewolf, K::Witch], 1);
        assert_eq!(p.leader, 1, "the toughest leads");
        assert_eq!(p.members.iter().map(|m| m.role).collect::<Vec<_>>(), vec![Role::Vanguard, Role::Harrier, Role::Caller]);
        assert!(p.affixes.is_empty());
        assert_eq!(pack(&[K::Blemmyes, K::Witch], 3).affixes.len(), 1);
        let two = pack(&[K::Blemmyes, K::Witch], 6).affixes;
        assert_eq!(two.len(), 2);
        assert_ne!(two[0], two[1]);
    }

    #[test]
    fn a_broken_pack_routs_quickens_or_fights_on_by_its_temperament() {
        use EnemyKind as K;
        let slay = |p: &mut Pack, id: u32| {
            p.hear(&WarEvent::Slain { foe: id, kind: K::Bat, at: V::new(0.0, 0.0), dir: V::new(1.0, 0.0), last_of_pack: false }, true)
        };
        let mut cowards = pack(&[K::Bat; 6], 1);
        assert_eq!(cowards.temperament, Temperament::Cowardly);
        let leader = cowards.leader;
        assert_eq!(slay(&mut cowards, leader), None, "65 left");
        for left in [45.0, 25.0, 5.0] {
            let other = cowards.members.iter().find(|m| m.alive).unwrap().id;
            assert_eq!(slay(&mut cowards, other), None, "{left} left");
        }
        let other = cowards.members.iter().find(|m| m.alive).unwrap().id;
        assert_eq!(slay(&mut cowards, other), Some(PackCue::Broke(Temperament::Cowardly)), "the last bat flees");
        assert_eq!(cowards.rout, ROUT_FRAMES);
        let (d, _) = cowards.frame(
            &[seen(cowards.members.iter().find(|m| m.alive).unwrap().id, 3.0, 0.0, 1.0)],
            V::new(0.0, 0.0),
            V::new(1.0, 0.0),
            false,
        );
        assert!(d.iter().all(|(_, d)| !d.ask), "a routed pack asks for nothing");
        let mut wolves = pack(&[K::Werewolf, K::Werewolf], 1);
        assert_eq!(wolves.temperament, Temperament::Frenzied);
        wolves.lose(100.0);
        assert!(wolves.frenzy > 0 && wolves.rout == 0);
        let mut bones = pack(&[K::Helmeted, K::Skeleton], 1);
        bones.lose(100.0);
        assert!(bones.broken && bones.rout == 0 && bones.frenzy == 0);
        assert_eq!(bones.lose(100.0), None, "a pack breaks once");
    }

    #[test]
    fn two_biters_take_his_two_sides_and_a_brute_screens_the_caster() {
        use EnemyKind as K;
        let mut p = pack(&[K::Werewolf, K::Werewolf], 1);
        let s = [seen(1, 2.0, 1.0, 1.0), seen(2, 2.0, -1.0, 1.0)];
        let (d, _) = p.frame(&s, V::new(0.0, 0.0), V::new(1.0, 0.0), false);
        assert_eq!(p.play, Play::Pincer);
        let wants: Vec<f64> = d.iter().filter_map(|(_, d)| d.want).collect();
        assert_eq!(wants.len(), 2);
        assert!((wrap(wants[0] - wants[1]).abs() - PI).abs() < 1e-9, "opposite sides: {wants:?}");
        let mut p = pack(&[K::Blemmyes, K::Witch], 1);
        let s = [seen(1, -3.0, 0.0, 1.0), seen(2, 0.0, 3.0, 1.0)];
        let (d, _) = p.frame(&s, V::new(0.0, 0.0), V::new(1.0, 0.0), true);
        assert_eq!(p.play, Play::Screen);
        let brute = d.iter().find(|(id, _)| *id == 1).unwrap().1;
        assert!((brute.want.unwrap() - FRAC_PI_2).abs() < 1e-9, "between him and the witch");
        let witch = d.iter().find(|(id, _)| *id == 2).unwrap().1;
        assert!(witch.bias > 0.0, "the caster presses a hero behind his shield");
    }

    #[test]
    fn a_wounded_leader_rallies_and_a_wounded_foe_falls_back() {
        use EnemyKind as K;
        let mut p = pack(&[K::Blemmyes, K::Werewolf, K::Werewolf, K::Witch], 1);
        let s = [seen(1, 2.0, 0.0, 0.5), seen(2, 0.0, 2.0, 1.0), seen(3, 0.0, -2.0, 0.2), seen(4, -6.0, 0.0, 1.0)];
        let (d, rally) = p.frame(&s, V::new(0.0, 0.0), V::new(1.0, 0.0), false);
        assert_eq!(p.play, Play::Rally);
        assert!(rally);
        let hurt = d.iter().find(|(id, _)| *id == 3).unwrap().1;
        assert!(!hurt.ask && hurt.back > 0.0, "{hurt:?}");
    }
}
