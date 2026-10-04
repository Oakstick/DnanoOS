//! Game feel by the numbers (WARBAND.md §4): how long the world holds on a hit and how much of
//! that a fight may spend, when time slows and by how much, how hard each kind of blow shakes the
//! camera, what each body is made of when it breaks, and how the springs are tuned. Numbers and
//! pure functions only: `game::juice` and the view apply them, and the audio shares `Matter` (the
//! thud of a blow is the thud of what it struck).
//!
//! None of it touches the simulation's rng or its frame data. The one thing here that reaches
//! the simulation is the time scale, which slows everything evenly (WARBAND.md I6).

use super::enemy::EnemyKind;

/// The combat's frames per second: every freeze here is counted in them.
pub const FPS: f64 = 60.0;

// ------------------------------------------------------------------ 4.1 hit pause

/// Why the world holds for a beat (WARBAND.md §4.1's table).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stop {
    /// The hero's blow lands: the move's own freeze (3 for a light blow, 6 for a finisher).
    Blow(u16),
    /// A foe's poise broken.
    Break,
    /// A parry: the game's best moment, and longer still against a prince's move.
    Parry {
        prince: bool,
    },
    Kill,
    /// The last foe of a pack: the release beat (with a slowing of time, `DilationKind::LastOfPack`).
    LastOfPack,
    /// A rival or a general slain: the trophy beat.
    Trophy,
    /// The hero is struck: short, so being hit never feels sticky. It carries the blow's own
    /// freeze, which is what the brawl held for before the table (`legacy_frames`).
    Struck(u16),
    /// A blow taken on the shield.
    Blocked,
    /// His guard broken.
    GuardBroken,
}

impl Stop {
    /// Freeze frames (60 Hz) by the table.
    pub fn frames(self) -> u16 {
        match self {
            Stop::Blow(own) => own,
            Stop::Break => 8,
            Stop::Parry { prince } => 8 + if prince { 4 } else { 0 },
            Stop::Kill | Stop::LastOfPack => 6,
            Stop::Trophy => 12,
            Stop::Struck(_) => 3,
            Stop::Blocked => 2,
            Stop::GuardBroken => 5,
        }
    }

    /// The freeze the brawl held for before the table: what the harnesses use with the juice off,
    /// so their numbers stay what they were.
    pub fn legacy_frames(self) -> u16 {
        match self {
            Stop::Blow(own) | Stop::Struck(own) => own,
            Stop::Break | Stop::Parry { .. } => 8,
            Stop::Kill | Stop::LastOfPack | Stop::Trophy => 6,
            Stop::Blocked | Stop::GuardBroken => 0,
        }
    }
}

/// The budget: freezes add up to at most this many frames in any window of `BUDGET_WINDOW`
/// frames; past it, each new freeze is halved. A sweep that meets four foes must not stall the
/// game for half a second.
pub const BUDGET_FRAMES: f64 = 14.0;
pub const BUDGET_WINDOW: f64 = 60.0;

/// How much of a freeze of `frames` is held when `used` frames of freeze already fell in the
/// window: all of it while it fits the budget; past it, half (or what is left of the budget,
/// if that is more).
pub fn budgeted(used: f64, frames: f64) -> f64 {
    if used + frames <= BUDGET_FRAMES {
        frames
    } else {
        (BUDGET_FRAMES - used).max(frames * 0.5)
    }
}

// ------------------------------------------------------------------ 4.2 time dilation

/// What slows time (WARBAND.md §4.2's table).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DilationKind {
    /// The last foe of a pack slain.
    LastOfPack,
    /// A rival or a general slain.
    Trophy,
    /// A prince's phase falls.
    PrinceFalls,
    /// A prince's move parried.
    PrinceParry,
    /// A blow passes under his leap with his health under `NARROW_HEALTH`.
    NarrowEscape,
    /// A rival of the Ledger comes within ten tiles (WARBAND.md §8.4).
    RivalMet,
}

/// His share of health under which a blow passing under his leap slows time.
pub const NARROW_HEALTH: f64 = 0.15;

/// At most one slowing of time in this many seconds.
pub const DILATION_EVERY: f64 = 3.0;

/// One slowing of time: the scale it holds, for how long (seconds, real time), and how long it
/// takes to ease back to 1.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Dilation {
    pub scale: f64,
    pub hold: f64,
    pub ease: f64,
}

impl DilationKind {
    pub const ALL: [DilationKind; 6] = [
        DilationKind::LastOfPack,
        DilationKind::Trophy,
        DilationKind::PrinceFalls,
        DilationKind::PrinceParry,
        DilationKind::NarrowEscape,
        DilationKind::RivalMet,
    ];

    pub fn dilation(self) -> Dilation {
        let (scale, hold, ease) = match self {
            DilationKind::LastOfPack => (0.35, 0.20, 0.25),
            DilationKind::Trophy => (0.2, 0.45, 0.35),
            DilationKind::PrinceFalls => (0.3, 0.35, 0.3),
            DilationKind::PrinceParry => (0.5, 0.12, 0.15),
            DilationKind::NarrowEscape => (0.5, 0.15, 0.2),
            DilationKind::RivalMet => (0.5, 0.25, 0.2),
        };
        Dilation { scale, hold, ease }
    }
}

impl Dilation {
    /// The time scale `age` seconds after it began: held, then eased back to 1 (smoothstep);
    /// None once it is over.
    pub fn scale_at(&self, age: f64) -> Option<f64> {
        if age < self.hold {
            return Some(self.scale);
        }
        let u = (age - self.hold) / self.ease.max(1e-6);
        if u >= 1.0 {
            return None;
        }
        let e = u * u * (3.0 - 2.0 * u);
        Some(self.scale + (1.0 - self.scale) * e)
    }

    /// Seconds from its start to its end.
    pub fn length(&self) -> f64 {
        self.hold + self.ease
    }
}

/// The pitch the sound effects drop to under a time scale (WARBAND.md §4.2: `scale^0.25`).
pub fn pitch_of(scale: f64) -> f64 {
    scale.clamp(0.0, 1.0).powf(0.25)
}

// ------------------------------------------------------------------ 4.3 camera trauma

/// What shakes the camera, and how hard (WARBAND.md §4.3's table). Each adds its trauma, but
/// never pushes the total past its own cap (a flurry of light blows never shakes like a quake).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Trauma {
    /// The hero struck, for `dmg`.
    Struck { dmg: f64 },
    /// The hero's light blow lands.
    Blow,
    /// The hero's finisher lands.
    Finisher,
    /// A foe's poise broken.
    Break,
    /// A foe slain.
    Kill,
    /// His guard broken.
    GuardBroken,
    /// A quake (the kanabō, the frost slam, the hoard slam) `dist` tiles away.
    Quake { dist: f64 },
    /// A giant's footfall `dist` tiles away.
    Footfall { dist: f64 },
    /// A general's muster horn.
    Horn,
    /// A rival or a general slain.
    Trophy,
}

/// How far a quake's trauma reaches (falling off evenly to nothing), and a giant's footfall.
pub const QUAKE_REACH: f64 = 6.0;
pub const FOOTFALL_REACH: f64 = 4.0;

impl Trauma {
    /// The trauma it adds (0 to 1).
    pub fn amount(self) -> f32 {
        let a = match self {
            Trauma::Struck { dmg } => 0.22 + dmg.max(0.0) / 200.0,
            Trauma::Blow => 0.08,
            Trauma::Finisher => 0.12,
            Trauma::Break => 0.2,
            Trauma::Kill => 0.18,
            Trauma::GuardBroken => 0.3,
            Trauma::Quake { dist } => 0.45 * (1.0 - dist / QUAKE_REACH).max(0.0),
            Trauma::Footfall { dist } if dist <= FOOTFALL_REACH => 0.05,
            Trauma::Footfall { .. } => 0.0,
            Trauma::Horn => 0.15,
            Trauma::Trophy => 0.4,
        };
        a as f32
    }

    /// The most trauma it may bring the total to.
    pub fn cap(self) -> f32 {
        match self {
            Trauma::Struck { .. } | Trauma::GuardBroken => 0.5,
            Trauma::Blow => 0.2,
            Trauma::Finisher | Trauma::Break | Trauma::Kill => 0.35,
            Trauma::Quake { .. } => 0.6,
            Trauma::Footfall { .. } => 0.1,
            Trauma::Horn => 0.15,
            Trauma::Trophy => 0.4,
        }
    }

    /// The total after this source adds to `now`: never past its cap, and never less than it was.
    pub fn add_to(self, now: f32) -> f32 {
        now.max((now + self.amount()).min(self.cap())).min(1.0)
    }
}

/// The camera's roll at full trauma, in degrees (scaled by trauma squared, as the shake is).
pub const ROLL_DEGREES: f32 = 2.5;

/// FOV punches: degrees and seconds. Narrower for a finisher or a break (the eye leans in), wider
/// for a quake (the ground heaves out).
pub const FOV_FINISHER: (f32, f32) = (-3.0, 0.08);
pub const FOV_QUAKE: (f32, f32) = (2.0, 0.2);

/// The camera's kick along a blow (world units), and its spring: kicked, it swings back past
/// rest and settles (§4.5).
pub const KICK_STRUCK: f32 = 0.16;
pub const KICK_BREAK: f32 = 0.1;
pub const KICK_FINISHER: f32 = 0.07;
pub const KICK_ZETA: f32 = 0.4;
pub const KICK_SETTLE: f32 = 0.3;

// ------------------------------------------------------------------ 4.4 kinetic debris

/// What a body is made of: what flies off it when struck, how its pieces fall when it breaks,
/// and (for the audio) what a blow on it sounds like.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Matter {
    /// Ichor drops.
    Flesh,
    /// Chips of bone.
    Bone,
    /// Sparks off iron.
    Iron,
    Stone,
    /// Slow to fall, quick to stop.
    Feather,
    /// Rises, never bounces.
    Smoke,
    /// Glowing, and lights the floor where it falls.
    Ember,
    /// Shards.
    Ice,
    /// Coins that ring as they land.
    Gold,
    /// Globs that stick where they fall, and fade.
    Slime,
    /// Splinters.
    Wood,
    /// Plates of shell and scale.
    Chitin,
}

/// How a matter's pieces fly: gravity (a share of the world's), drag per second, how much of a
/// bounce they keep (restitution), how much of their slide the floor takes (friction), how much
/// they glow, how big a chip is, and their shape (x, y, z stretch of a cube).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Grain {
    pub gravity: f32,
    pub drag: f32,
    pub bounce: f32,
    pub friction: f32,
    pub glow: f32,
    pub chip: f32,
    pub shape: [f32; 3],
}

/// The restitution and friction a shattered body's pieces have unless their matter says
/// otherwise (WARBAND.md §4.4).
pub const RESTITUTION: f32 = 0.3;
pub const FRICTION: f32 = 0.55;

impl Matter {
    pub const ALL: [Matter; 12] = [
        Matter::Flesh,
        Matter::Bone,
        Matter::Iron,
        Matter::Stone,
        Matter::Feather,
        Matter::Smoke,
        Matter::Ember,
        Matter::Ice,
        Matter::Gold,
        Matter::Slime,
        Matter::Wood,
        Matter::Chitin,
    ];

    pub fn grain(self) -> Grain {
        let g = Grain { gravity: 1.0, drag: 0.3, bounce: RESTITUTION, friction: FRICTION, glow: 0.0, chip: 0.06, shape: [1.0, 1.0, 1.0] };
        match self {
            Matter::Flesh => Grain { bounce: 0.1, friction: 0.85, chip: 0.045, ..g },
            Matter::Bone => Grain { bounce: 0.35, chip: 0.055, shape: [1.0, 0.6, 1.6], ..g },
            Matter::Iron => Grain { bounce: 0.4, friction: 0.4, chip: 0.04, glow: 2.2, ..g },
            Matter::Stone => Grain { bounce: 0.25, chip: 0.075, ..g },
            Matter::Feather => Grain { gravity: 0.12, drag: 3.2, bounce: 0.0, friction: 0.9, chip: 0.06, shape: [1.4, 0.2, 0.8], ..g },
            Matter::Smoke => Grain { gravity: -0.12, drag: 2.4, bounce: 0.0, friction: 1.0, chip: 0.08, ..g },
            Matter::Ember => Grain { gravity: 0.7, drag: 0.8, bounce: 0.3, glow: 3.0, chip: 0.04, ..g },
            Matter::Ice => Grain { bounce: 0.25, friction: 0.2, chip: 0.06, shape: [0.6, 1.8, 0.6], ..g },
            Matter::Gold => Grain { bounce: 0.45, friction: 0.5, chip: 0.06, glow: 0.25, shape: [1.0, 0.3, 1.0], ..g },
            Matter::Slime => Grain { bounce: 0.0, friction: 1.0, chip: 0.07, shape: [1.2, 0.7, 1.2], ..g },
            Matter::Wood => Grain { bounce: 0.3, chip: 0.05, shape: [0.5, 0.5, 2.0], ..g },
            Matter::Chitin => Grain { bounce: 0.3, chip: 0.06, shape: [1.3, 0.35, 1.0], ..g },
        }
    }
}

/// What each foe is made of (for its chips, its shatter and the sound of a blow on it).
pub fn matter_of(kind: EnemyKind) -> Matter {
    use EnemyKind as K;
    match kind {
        K::Skeleton | K::Gashadokuro | K::Hel => Matter::Bone,
        K::Helmeted => Matter::Iron,
        K::Slime | K::Kappa => Matter::Slime,
        K::Bat | K::Vampire | K::Witch | K::Minion | K::Werewolf | K::Blemmyes | K::LargeBat => Matter::Flesh,
        K::Rasetsu | K::DireWolf | K::Rokurokubi | K::Oni | K::Gaki => Matter::Flesh,
        K::Tengu | K::Kitsune | K::Tulukus => Matter::Feather,
        K::Jorogumo | K::Cobra | K::Ejder | K::Nidhogg => Matter::Chitin,
        K::Albasti | K::Goryo | K::Ganglot => Matter::Smoke,
        K::Cin | K::Surtr => Matter::Ember,
        K::Arcura | K::Ellepige => Matter::Wood,
        K::Karakoncolos | K::FrostGiant | K::Peerless => Matter::Ice,
        K::Tepegoz => Matter::Stone,
        K::Mammon | K::GoldGolem => Matter::Gold,
        // The other princes and what they raise, for now: flesh (and the reflections glass).
        K::Lucifer | K::Leviathan | K::Amon | K::Belphegor | K::Beelzebub | K::Asmodeus => Matter::Flesh,
        K::Reflection => Matter::Ice,
        K::Serpentling | K::FlySwarm => Matter::Chitin,
        K::Raven => Matter::Feather,
        K::Nightmare => Matter::Smoke,
        // Krampus's lair: his black fur and goat's hide, and his Perchten's.
        K::Krampus | K::Percht => Matter::Flesh,
    }
}

/// The giants: their shatter is bigger, and raises a ring of dust.
pub fn is_giant(kind: EnemyKind) -> bool {
    use EnemyKind as K;
    matches!(
        kind,
        K::Tepegoz
            | K::Gashadokuro
            | K::Ejder
            | K::FrostGiant
            | K::Surtr
            | K::Nidhogg
            | K::Mammon
            | K::Lucifer
            | K::Leviathan
            | K::Amon
            | K::Belphegor
            | K::Beelzebub
            | K::Asmodeus
            | K::Krampus
    )
}

/// Chips a blow throws off (fewer from a light blow, more from a finisher), before the ±25% of
/// variance the view draws from its own rng.
pub const CHIPS_LIGHT: usize = 5;
pub const CHIPS_FINISHER: usize = 9;
pub const CHIPS_RANGE: (usize, usize) = (4, 10);

/// A slain foe's shatter: how many cubes (the giants more), how fast they fly (along the blow,
/// out from the body, and up, tiles a second), how long before they rest, and the seconds they
/// take to sink and fade.
pub const SHATTER_CUBES: (usize, usize) = (24, 60);
pub const SHATTER_GIANT: usize = 90;
pub const SHATTER_ALONG: f32 = 3.0;
pub const SHATTER_RADIAL: f32 = 1.5;
pub const SHATTER_UP: f32 = 2.5;
pub const SHATTER_FLIGHT: f32 = 1.2;
pub const SHATTER_FADE: f32 = 2.5;

/// How many cubes a body of `voxels` coarse voxels breaks into.
pub fn shatter_cubes(voxels: usize, giant: bool) -> usize {
    if giant {
        SHATTER_GIANT
    } else {
        (voxels / 4).clamp(SHATTER_CUBES.0, SHATTER_CUBES.1)
    }
}

// ------------------------------------------------------------------ 4.5 overshoot

/// A slain foe tips along the blow for this many frames before it breaks.
pub const TOPPLE_FRAMES: f32 = 6.0;
/// How far it tips by then (radians).
pub const TOPPLE_ANGLE: f32 = 0.55;

/// A foe's flinch: kicked back along the blow (tiles of push at the kick, and its spring).
pub const FLINCH_PUSH: f32 = 0.16;
pub const FLINCH_ZETA: f32 = 0.35;
pub const FLINCH_SETTLE: f32 = 0.15;

/// The hero's blade follows through past its end pose by this share, then comes back.
pub const BLADE_OVERSHOOT: f32 = 0.12;

/// A foe's anticipation: it pulls back by this share in the first third of its startup, and
/// holds it until its commit frames.
pub const ANTICIPATION: f32 = 0.08;
pub const ANTICIPATION_BY: f32 = 1.0 / 3.0;
/// The pose is fully wound by this share of the startup at the latest (the tell must read).
pub const WOUND_BY: f32 = 0.6;

/// A brute's slam lands: squashed to this, springing past rest to `SLAM_PEAK`.
pub const SLAM_SQUASH: f32 = 0.85;
pub const SLAM_PEAK: f32 = 1.06;
/// The hero's leap lands: squashed to this, with the same overshoot.
pub const LAND_SQUASH: f32 = 0.9;

/// The shield knocked back on a block (radians) and its spring.
pub const SHIELD_JOLT: f32 = 0.1745;
pub const SHIELD_ZETA: f32 = 0.4;
pub const SHIELD_SETTLE: f32 = 0.25;

/// A spring's damping that overshoots a step by `share` of the step (0 < share < 1).
pub fn zeta_for_overshoot(share: f32) -> f32 {
    let l = share.clamp(1e-4, 0.999).ln();
    -l / (std::f32::consts::PI * std::f32::consts::PI + l * l).sqrt()
}

/// A spring's stiffness (ω², per second squared) that settles within `secs` at damping `zeta`
/// (to about 2% of the kick: `4 / (ζ ω)`).
pub fn stiffness_for(zeta: f32, secs: f32) -> f32 {
    let w = 4.0 / (zeta.max(0.05) * secs.max(1e-3));
    w * w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_freeze_table_matches_the_plan() {
        assert_eq!(Stop::Blow(3).frames(), 3);
        assert_eq!(Stop::Blow(6).frames(), 6);
        assert_eq!(Stop::Break.frames(), 8);
        assert_eq!(Stop::Parry { prince: false }.frames(), 8);
        assert_eq!(Stop::Parry { prince: true }.frames(), 12);
        assert_eq!(Stop::Kill.frames(), 6);
        assert_eq!(Stop::LastOfPack.frames(), 6);
        assert_eq!(Stop::Trophy.frames(), 12);
        assert_eq!(Stop::Struck(9).frames(), 3, "being hit is never sticky");
        assert_eq!(Stop::Blocked.frames(), 2);
        assert_eq!(Stop::GuardBroken.frames(), 5);
        // The legacy table is the brawl's constants as they stood.
        assert_eq!(Stop::Struck(9).legacy_frames(), 9);
        assert_eq!(Stop::Blocked.legacy_frames(), 0);
        assert_eq!(Stop::Parry { prince: true }.legacy_frames(), 8);
    }

    #[test]
    fn the_budget_halves_what_comes_past_it() {
        assert_eq!(budgeted(0.0, 8.0), 8.0);
        assert_eq!(budgeted(8.0, 6.0), 6.0, "exactly at the budget");
        assert_eq!(budgeted(14.0, 8.0), 4.0, "past it, halved");
        assert_eq!(budgeted(10.0, 8.0), 4.0);
        assert_eq!(budgeted(6.0, 12.0), 8.0, "what is left of the budget, when that is more than half");
        // A sweep of four foes, each a kill (6 f), one frame apart: well under half a second.
        let mut used = 0.0;
        for _ in 0..4 {
            used += budgeted(used, 6.0);
        }
        assert!(used <= 20.0, "{used}");
    }

    #[test]
    fn a_dilation_holds_then_eases_back() {
        for kind in DilationKind::ALL {
            let d = kind.dilation();
            assert!(d.scale > 0.0 && d.scale < 1.0);
            assert_eq!(d.scale_at(0.0), Some(d.scale));
            assert_eq!(d.scale_at(d.hold * 0.99), Some(d.scale));
            let mid = d.scale_at(d.hold + d.ease * 0.5).unwrap();
            assert!(mid > d.scale && mid < 1.0, "{kind:?} {mid}");
            assert_eq!(d.scale_at(d.length() + 1e-6), None);
            // Eased: no step anywhere along it.
            let mut last = d.scale;
            for k in 0..200 {
                let s = d.scale_at(d.length() * k as f64 / 200.0).unwrap_or(1.0);
                assert!(s >= last - 1e-9 && s - last < 0.05, "{kind:?} jumps at {k}");
                last = s;
            }
        }
        assert_eq!(DilationKind::LastOfPack.dilation(), Dilation { scale: 0.35, hold: 0.2, ease: 0.25 });
        assert!((pitch_of(0.35) - 0.769).abs() < 1e-3);
    }

    #[test]
    fn trauma_never_passes_its_sources_cap() {
        let mut t = 0.0;
        for _ in 0..20 {
            t = Trauma::Blow.add_to(t);
        }
        assert!((t - 0.2).abs() < 1e-6, "a flurry of light blows stops at its cap: {t}");
        // A bigger source on top goes on to its own cap; a smaller one never lowers it.
        t = Trauma::Struck { dmg: 0.0 }.add_to(t);
        assert!((t - 0.42).abs() < 1e-6, "{t}");
        t = Trauma::Struck { dmg: 40.0 }.add_to(t);
        assert!((t - 0.5).abs() < 1e-6, "{t}");
        assert_eq!(Trauma::Footfall { dist: 1.0 }.add_to(t), t);
        assert_eq!(Trauma::Quake { dist: 7.0 }.amount(), 0.0, "out of a quake's reach");
        assert!((Trauma::Quake { dist: 3.0 }.amount() - 0.225).abs() < 1e-6);
        assert_eq!(Trauma::Footfall { dist: 5.0 }.amount(), 0.0);
    }

    #[test]
    fn every_foe_is_made_of_something() {
        // (The match in `matter_of` is exhaustive; here each matter is used by someone.)
        let kinds = [
            EnemyKind::Skeleton,
            EnemyKind::Helmeted,
            EnemyKind::Slime,
            EnemyKind::Vampire,
            EnemyKind::Tengu,
            EnemyKind::Jorogumo,
            EnemyKind::Goryo,
            EnemyKind::Surtr,
            EnemyKind::Arcura,
            EnemyKind::FrostGiant,
            EnemyKind::Tepegoz,
            EnemyKind::Mammon,
        ];
        for m in Matter::ALL {
            assert!(kinds.iter().any(|&k| matter_of(k) == m), "no foe of {m:?}");
            let g = m.grain();
            assert!(g.bounce >= 0.0 && g.bounce < 1.0 && g.friction > 0.0 && g.chip > 0.0);
        }
        assert!(is_giant(EnemyKind::Surtr) && !is_giant(EnemyKind::Oni));
        assert_eq!(shatter_cubes(30, false), 24);
        assert_eq!(shatter_cubes(400, false), 60);
        assert_eq!(shatter_cubes(10, true), 90);
    }

    #[test]
    fn spring_tunings_hit_their_marks() {
        // The slam's squash springs past rest to 1.06.
        let z = zeta_for_overshoot((SLAM_PEAK - 1.0) / (1.0 - SLAM_SQUASH));
        let over = (-z * std::f32::consts::PI / (1.0 - z * z).sqrt()).exp();
        assert!(((1.0 - SLAM_SQUASH) * over - (SLAM_PEAK - 1.0)).abs() < 1e-4, "{z} {over}");
        let k = stiffness_for(FLINCH_ZETA, FLINCH_SETTLE);
        assert!((4.0 / (FLINCH_ZETA * k.sqrt()) - FLINCH_SETTLE).abs() < 1e-4);
        // The anticipation is wound well before the commit flash.
        const { assert!(ANTICIPATION_BY <= WOUND_BY) };
    }
}
