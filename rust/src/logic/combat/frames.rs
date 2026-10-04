//! Frame data: every move broken into startup, active and recovery frames (60 Hz), with what
//! it deals and how it strikes. The table is COMBAT.md §1 (and §11 for the folklore moves of the
//! later stages); the tests below hold it to the design limits of §0 (the reaction floor, the
//! punish window, the parry by reaction).

use std::f64::consts::PI;

/// Human visual reaction, in frames (about 250 ms).
pub const REACTION: u16 = 15;

/// Every move in the game.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub enum MoveId {
    /// The hero's training sword: a string of three.
    L1,
    L2,
    L3,
    /// The Brute's overhead slam: slow, far, narrow.
    Slam,
    /// The Brute's sweep, for a hero at his flank.
    Sweep,
    /// The Skirmisher's bite: from beside him, with nothing thrown across the floor.
    Bite,
    /// The Hexer's bolt.
    Hex,
    // ---- the folklore moves of the later stages (COMBAT.md §11), each a breed's own
    /// The kappa's sumo shove: a palm thrust from beside him that shoves him back, blocked or not.
    Shove,
    /// The rokurokubi's bite: her head dips a little way on her neck and bites, close.
    NeckBite,
    /// The kitsune's foxfire: three bolts in a fan.
    Foxfire,
    /// The oni's iron club: an unstoppable slam that shakes the ground all round.
    Kanabo,
    /// The tülükuş's talons: up on its wings over him, and down talons first.
    Talons,
    /// The albastı's weight: she rears up beside him and falls on him with all of it.
    Smother,
    /// The cin's smokeless fire: a bolt out of nowhere.
    Smokeless,
    /// The arçura's root dart: a thorny seed that bursts into roots where it strikes.
    Roots,
    /// The karakoncolos's call: his name roared in a loved one's voice, and it dooms him.
    Call,
    /// The gashadokuro's grab: two great hands, unblockable; it crushes what it closes on.
    Grab,
    /// The seven heads of the ejder: four bites one after another.
    Heads,
    /// The goryō's thunderbolt: lightning flung at him.
    Thunder,
    /// The frost giant's slam: both fists down, and ice bursts from the floor; it frostbites.
    FrostSlam,
    /// Níðhöggr dives under the floor and bursts up beneath him: venom.
    Burrow,
    /// Hel raises the dead.
    Raise,
    /// A pack's leader sounds its rally (WARBAND.md §7.2): its pack's poise comes back. No blow;
    /// a long startup the hero can punish.
    Rally,
    // ---- the princes' moves (COMBAT.md §13), each phase of a prince a table of them
    /// Mammon's sceptre swept wide round him.
    Sceptre,
    /// Mammon's coins tossed at him: they land on the floor round him and burst into gold spikes.
    CoinToss,
    /// Mammon's grasp: his fist closes on him, and holds him while it drains his silver.
    Grasp,
    /// Mammon's coins flung in a fan, from afar.
    CoinVolley,
    /// Mammon's coins rained down from above all round him, to burst where they fall.
    CoinRain,
    /// Mammon's hoard brought down on the floor: a slam that shakes the ground.
    HoardSlam,
    // Lucifer's (Pride)
    /// Lucifer's radiant cut: a blade of light swept before him (a placeholder for his table).
    RadiantCut,
    // Leviathan's (Envy)
    /// Leviathan's coil lash: his coils whipped round before him (a placeholder for his table).
    CoilLash,
    // Amon's (Wrath)
    /// Amon's flame breath: fire breathed in a cone before him (a placeholder for his table).
    FlameBreath,
    // Belphegor's (Sloth)
    /// Belphegor's swat: a lazy backhand, heavy as sleep (a placeholder for his table).
    Swat,
    // Beelzebub's (Gluttony)
    /// Beelzebub's cleaver: a butcher's chop before him (a placeholder for his table).
    Cleaver,
    // Asmodeus's (Lust)
    /// Asmodeus's lance thrust: his lance driven before him (a placeholder for his table).
    LanceThrust,
}

/// Where in a move a frame falls.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Startup,
    Active,
    Recovery,
}

/// The shape of what a move strikes.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Shape {
    /// The cone of `reach` tiles and `half_angle` either side of the aim (every blow of the lab).
    Cone,
    /// A circle of `radius` on the floor, on the hero, following him through the tracking frames;
    /// the blow falls in it on its active frames (an eruption). Nothing guards against it: he
    /// steps out, or leaps.
    Mark { radius: f64 },
    /// `count` bolts in a fan `spread` radians either side of the aim.
    Volley { count: u8, spread: f64 },
    /// The cone, and the ground shaking `radius` tiles all round it at `share` of the blow.
    Quake { radius: f64, share: f64 },
    /// No blow: the move works another way (a call, the dead raised).
    Spell,
    /// `count` coins on the floor round him (the first where he stands), each bursting into gold
    /// spikes `radius` tiles wide `fuse` frames after they land. Nothing guards against them: he
    /// steps out, or leaps.
    Traps { count: u8, radius: f64, fuse: u16 },
}

/// What a move does besides its damage.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Rider {
    None,
    /// Shoves him this many tiles back, blocked or not (a sumo charge).
    Shove(f64),
    /// Frostbites him: the cold bites for a while, blocked or not (it never slows him).
    Frost,
    /// Poisons him for this many frames.
    Venom(u16),
    /// What it deals heals the one who dealt it.
    Drain,
    /// Raises this many of the dead.
    Raise(u8),
    /// Holds him in a fist until he strikes the one who holds him (Mammon's grasp).
    Grasp,
}

/// One move's frame data and hitbox.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoveData {
    pub name: &'static str,
    pub startup: u16,
    pub active: u16,
    pub recovery: u16,
    /// Base damage (before power, state, and proration).
    pub damage: f64,
    /// Base poise damage.
    pub poise: f64,
    /// Frames the struck reel (before decay).
    pub hitstun: u16,
    /// The hitbox: a cone of `reach` tiles and `half_angle` radians either side of the aim.
    pub reach: f64,
    pub half_angle: f64,
    /// Poise damage a blow needs to interrupt this move in its startup and active frames.
    pub armor: f64,
    /// Startup frames in which the attacker still turns toward its target; after them the aim is committed.
    pub track: u16,
    /// The next move of the string, for an attack buffered in the cancel window.
    pub chain: Option<MoveId>,
    /// Recovery frame from which the move may be cancelled (into its chain, an evade or the guard).
    pub cancel: u16,
    /// Freeze frames when it lands.
    pub hitstop: u16,
    /// It looses a projectile on its first active frame instead of striking.
    pub projectile: bool,
    /// A foe begins it with the hero between these distances (centre to centre), this far off its
    /// facing (radians, from and to), and, if `line`, nothing between them.
    pub range: (f64, f64),
    pub off: (f64, f64),
    pub line: bool,
    /// What it strikes, and what its blow does besides.
    pub shape: Shape,
    pub rider: Rider,
    /// Blows in its active frames, evenly spaced (a flurry of heads); 1 for most.
    pub hits: u8,
    /// Frames before a foe may begin it again.
    pub cooldown: u16,
    /// No block stops it (a grab): only the parry, the leap, or distance.
    pub unblockable: bool,
}

const fn deg(d: f64) -> f64 {
    d * PI / 180.0
}

const BASE: MoveData = MoveData {
    name: "",
    startup: 1,
    active: 1,
    recovery: 1,
    damage: 0.0,
    poise: 0.0,
    hitstun: 0,
    reach: 1.0,
    half_angle: deg(45.0),
    armor: 0.0,
    track: 0,
    chain: None,
    cancel: u16::MAX,
    hitstop: 0,
    projectile: false,
    range: (0.0, 1.0),
    off: (0.0, PI),
    line: false,
    shape: Shape::Cone,
    rider: Rider::None,
    hits: 1,
    cooldown: 0,
    unblockable: false,
};

/// The Hexer's bolt: tiles per second, frames it flies, and its radius.
pub const BOLT_SPEED: f64 = 7.0;
pub const BOLT_LIFE: u16 = 90;
pub const BOLT_RADIUS: f64 = 0.3;
/// How far a bolt flies.
pub const BOLT_REACH: f64 = BOLT_SPEED * BOLT_LIFE as f64 / 60.0;
/// A parried bolt flies back this much faster, and deals this many times its damage to foes.
pub const REFLECT_SPEED: f64 = 1.6;
/// A parried bolt's poise damage to the foe it meets.
pub const REFLECT_POISE: f64 = 30.0;
pub const REFLECT_DAMAGE: f64 = 1.5;
/// The farthest (centre to centre) a body foe bites from: beside him, no farther.
pub const BITE_RANGE: f64 = 1.3;
/// Unstoppable: armor no blow of his reaches (the oni's club; only the parry stops it).
pub const UNSTOPPABLE: f64 = 1e6;
/// A mark on the floor follows the hero this fast (tiles per second) through its tracking frames:
/// half his pace, so one who walks away at once leaves it behind.
pub const MARK_FOLLOW: f64 = 2.6;

/// A prince's first move before his fight is built: a wide cone swept before him like Mammon's
/// sceptre, slow to start, readable, and punishable in its long recovery (a big body: his arm adds
/// to its reach, `Breed::reach`).
const PRINCE_PLACEHOLDER: MoveData = MoveData {
    name: "",
    startup: 32,
    active: 8,
    recovery: 42,
    damage: 24.0,
    reach: 1.3,
    half_angle: deg(105.0),
    track: 18,
    hitstop: 6,
    range: (0.0, 1.3),
    off: (0.0, deg(80.0)),
    ..BASE
};

impl MoveId {
    pub const HERO: [MoveId; 3] = [MoveId::L1, MoveId::L2, MoveId::L3];
    /// Every foe's move: the archetypes' four, the folklore moves, then the princes', each
    /// prince's in a group of its own.
    pub const FOE: &[MoveId] = &[
        MoveId::Slam,
        MoveId::Sweep,
        MoveId::Bite,
        MoveId::Hex,
        MoveId::Shove,
        MoveId::NeckBite,
        MoveId::Foxfire,
        MoveId::Kanabo,
        MoveId::Talons,
        MoveId::Smother,
        MoveId::Smokeless,
        MoveId::Roots,
        MoveId::Call,
        MoveId::Grab,
        MoveId::Heads,
        MoveId::Thunder,
        MoveId::FrostSlam,
        MoveId::Burrow,
        MoveId::Raise,
        MoveId::Rally,
        MoveId::Sceptre,
        MoveId::CoinToss,
        MoveId::Grasp,
        MoveId::CoinVolley,
        MoveId::CoinRain,
        MoveId::HoardSlam,
        // Lucifer's (Pride)
        MoveId::RadiantCut,
        // Leviathan's (Envy)
        MoveId::CoilLash,
        // Amon's (Wrath)
        MoveId::FlameBreath,
        // Belphegor's (Sloth)
        MoveId::Swat,
        // Beelzebub's (Gluttony)
        MoveId::Cleaver,
        // Asmodeus's (Lust)
        MoveId::LanceThrust,
    ];

    pub const fn data(self) -> MoveData {
        match self {
            MoveId::L1 => MoveData {
                name: "L1",
                startup: 6,
                active: 4,
                recovery: 14,
                damage: 10.0,
                poise: 20.0,
                hitstun: 14,
                reach: 1.5,
                half_angle: deg(60.0),
                track: 4,
                chain: Some(MoveId::L2),
                cancel: 4,
                hitstop: 3,
                ..BASE
            },
            MoveId::L2 => MoveData {
                name: "L2",
                startup: 5,
                active: 4,
                recovery: 16,
                damage: 11.0,
                poise: 20.0,
                hitstun: 14,
                reach: 1.5,
                half_angle: deg(60.0),
                track: 3,
                chain: Some(MoveId::L3),
                cancel: 4,
                hitstop: 3,
                ..BASE
            },
            MoveId::L3 => MoveData {
                name: "L3",
                startup: 9,
                active: 5,
                recovery: 22,
                damage: 16.0,
                poise: 45.0,
                hitstun: 24,
                reach: 1.7,
                half_angle: deg(75.0),
                track: 5,
                cancel: 10,
                hitstop: 6,
                ..BASE
            },
            MoveId::Slam => MoveData {
                name: "SLAM",
                startup: 42,
                active: 6,
                recovery: 48,
                damage: 30.0,
                reach: 1.8,
                half_angle: deg(35.0),
                armor: 35.0,
                track: 30,
                hitstop: 6,
                range: (0.0, 1.7),
                off: (0.0, deg(60.0)),
                ..BASE
            },
            MoveId::Sweep => MoveData {
                name: "SWEEP",
                startup: 30,
                active: 8,
                recovery: 36,
                damage: 20.0,
                reach: 1.6,
                half_angle: deg(110.0),
                armor: 35.0,
                track: 16,
                hitstop: 5,
                // A hero at his flank (outside the slam's cone), close.
                range: (0.0, 1.6),
                off: (deg(60.0), PI),
                ..BASE
            },
            MoveId::Bite => MoveData {
                name: "BITE",
                startup: 18,
                active: 6,
                recovery: 24,
                damage: 20.0,
                reach: 1.1,
                half_angle: deg(70.0),
                track: 12,
                hitstop: 4,
                range: (0.0, BITE_RANGE),
                off: (0.0, deg(35.0)),
                ..BASE
            },
            MoveId::Hex => MoveData {
                name: "HEX",
                startup: 36,
                active: 1,
                recovery: 30,
                damage: 15.0,
                reach: BOLT_REACH,
                half_angle: 0.0,
                track: 30,
                hitstop: 3,
                projectile: true,
                range: (2.5, 7.5),
                off: (0.0, deg(45.0)),
                line: true,
                ..BASE
            },
            MoveId::Shove => MoveData {
                name: "SUMO SHOVE",
                startup: 22,
                active: 6,
                recovery: 28,
                damage: 18.0,
                reach: 1.2,
                half_angle: deg(60.0),
                track: 14,
                hitstop: 5,
                range: (0.0, BITE_RANGE + 0.1),
                off: (0.0, deg(35.0)),
                rider: Rider::Shove(2.2),
                ..BASE
            },
            MoveId::NeckBite => MoveData {
                name: "NECK BITE",
                startup: 26,
                active: 6,
                recovery: 30,
                damage: 20.0,
                reach: 1.2,
                half_angle: deg(55.0),
                track: 18,
                hitstop: 5,
                range: (0.0, BITE_RANGE),
                off: (0.0, deg(35.0)),
                ..BASE
            },
            MoveId::Foxfire => MoveData {
                name: "FOXFIRE",
                startup: 34,
                active: 1,
                recovery: 32,
                damage: 9.0,
                reach: BOLT_REACH,
                half_angle: 0.0,
                track: 28,
                hitstop: 3,
                projectile: true,
                range: (2.5, 7.5),
                off: (0.0, deg(45.0)),
                line: true,
                shape: Shape::Volley { count: 3, spread: 0.28 },
                ..BASE
            },
            MoveId::Kanabo => MoveData {
                name: "KANABO",
                startup: 46,
                active: 6,
                recovery: 52,
                damage: 32.0,
                reach: 1.9,
                half_angle: deg(35.0),
                armor: UNSTOPPABLE,
                track: 32,
                hitstop: 7,
                range: (0.0, 1.8),
                off: (0.0, deg(60.0)),
                shape: Shape::Quake { radius: 2.3, share: 0.45 },
                ..BASE
            },
            MoveId::Talons => MoveData {
                name: "TALONS",
                startup: 30,
                active: 6,
                recovery: 34,
                damage: 26.0,
                reach: 1.2,
                half_angle: deg(60.0),
                track: 20,
                hitstop: 6,
                range: (0.0, BITE_RANGE),
                off: (0.0, deg(40.0)),
                ..BASE
            },
            MoveId::Smother => MoveData {
                name: "SMOTHER",
                startup: 26,
                active: 6,
                recovery: 34,
                damage: 24.0,
                reach: 1.2,
                half_angle: deg(65.0),
                track: 16,
                hitstop: 6,
                range: (0.0, BITE_RANGE),
                off: (0.0, deg(35.0)),
                ..BASE
            },
            MoveId::Smokeless => MoveData {
                name: "SMOKELESS FIRE",
                startup: 36,
                active: 1,
                recovery: 30,
                damage: 14.0,
                reach: BOLT_REACH,
                half_angle: 0.0,
                track: 30,
                hitstop: 3,
                projectile: true,
                range: (2.5, 7.5),
                off: (0.0, deg(45.0)),
                line: true,
                ..BASE
            },
            MoveId::Roots => MoveData {
                name: "ROOT DART",
                startup: 38,
                active: 1,
                recovery: 30,
                damage: 18.0,
                reach: BOLT_REACH,
                half_angle: 0.0,
                track: 30,
                hitstop: 3,
                projectile: true,
                range: (2.5, 7.5),
                off: (0.0, deg(45.0)),
                line: true,
                ..BASE
            },
            MoveId::Call => MoveData {
                name: "THE CALL",
                startup: 30,
                active: 45,
                recovery: 12,
                reach: 7.0,
                half_angle: 0.0,
                track: 30,
                range: (2.8, 7.0),
                off: (0.0, deg(50.0)),
                line: true,
                shape: Shape::Spell,
                cooldown: 420,
                ..BASE
            },
            MoveId::Grab => MoveData {
                name: "GRAB",
                startup: 40,
                active: 8,
                recovery: 44,
                damage: 30.0,
                reach: 2.0,
                half_angle: deg(45.0),
                track: 26,
                hitstop: 6,
                range: (0.0, 2.0),
                off: (0.0, deg(50.0)),
                cooldown: 240,
                unblockable: true,
                ..BASE
            },
            MoveId::Heads => MoveData {
                name: "SEVEN HEADS",
                startup: 30,
                active: 24,
                recovery: 34,
                damage: 11.0,
                reach: 1.4,
                half_angle: deg(55.0),
                track: 18,
                hitstop: 3,
                range: (0.0, BITE_RANGE + 0.1),
                off: (0.0, deg(40.0)),
                hits: 4,
                ..BASE
            },
            MoveId::Thunder => MoveData {
                name: "THUNDERBOLT",
                startup: 40,
                active: 1,
                recovery: 32,
                damage: 22.0,
                reach: BOLT_REACH,
                half_angle: 0.0,
                track: 30,
                hitstop: 6,
                projectile: true,
                range: (2.5, 7.5),
                off: (0.0, deg(45.0)),
                line: true,
                ..BASE
            },
            MoveId::FrostSlam => MoveData {
                name: "FROST SLAM",
                startup: 40,
                active: 6,
                recovery: 44,
                damage: 22.0,
                reach: 1.8,
                half_angle: deg(40.0),
                armor: 35.0,
                track: 28,
                hitstop: 6,
                range: (0.0, 1.7),
                off: (0.0, deg(50.0)),
                rider: Rider::Frost,
                cooldown: 300,
                ..BASE
            },
            MoveId::Burrow => MoveData {
                name: "BURROW",
                startup: 60,
                active: 6,
                recovery: 40,
                damage: 26.0,
                reach: 10.0,
                half_angle: 0.0,
                track: 46,
                hitstop: 7,
                range: (3.5, 10.0),
                shape: Shape::Mark { radius: 1.0 },
                rider: Rider::Venom(120),
                cooldown: 360,
                ..BASE
            },
            MoveId::Rally => MoveData {
                name: "RALLY",
                startup: 40,
                active: 1,
                recovery: 36,
                reach: 6.0,
                half_angle: 0.0,
                track: 0,
                range: (0.0, 99.0),
                shape: Shape::Spell,
                cooldown: 600,
                ..BASE
            },
            MoveId::Raise => MoveData {
                name: "RAISE THE DEAD",
                startup: 50,
                active: 1,
                recovery: 40,
                reach: 12.0,
                half_angle: 0.0,
                range: (0.0, 12.0),
                shape: Shape::Spell,
                rider: Rider::Raise(2),
                cooldown: 600,
                ..BASE
            },
            // Mammon's (a big body: his arm adds to the reach of his blows, `Breed::reach`).
            MoveId::Sceptre => MoveData {
                name: "SCEPTRE SWEEP",
                startup: 32,
                active: 8,
                recovery: 42,
                damage: 24.0,
                reach: 1.3,
                half_angle: deg(105.0),
                track: 18,
                hitstop: 6,
                range: (0.0, 1.3),
                off: (0.0, deg(80.0)),
                ..BASE
            },
            MoveId::CoinToss => MoveData {
                name: "COIN TOSS",
                startup: 30,
                active: 1,
                recovery: 36,
                damage: 18.0,
                reach: 7.0,
                half_angle: 0.0,
                track: 26,
                hitstop: 5,
                range: (2.4, 7.0),
                off: (0.0, deg(60.0)),
                shape: Shape::Traps { count: 5, radius: 0.8, fuse: 48 },
                cooldown: 240,
                ..BASE
            },
            MoveId::Grasp => MoveData {
                name: "GRASP",
                startup: 36,
                active: 8,
                recovery: 46,
                damage: 10.0,
                reach: 1.4,
                half_angle: deg(45.0),
                track: 24,
                hitstop: 6,
                range: (0.0, 1.4),
                off: (0.0, deg(50.0)),
                rider: Rider::Grasp,
                cooldown: 300,
                unblockable: true,
                ..BASE
            },
            MoveId::CoinVolley => MoveData {
                name: "COIN VOLLEY",
                startup: 34,
                active: 1,
                recovery: 32,
                damage: 11.0,
                reach: BOLT_REACH,
                half_angle: 0.0,
                track: 28,
                hitstop: 3,
                projectile: true,
                range: (3.0, 8.0),
                off: (0.0, deg(45.0)),
                line: true,
                shape: Shape::Volley { count: 5, spread: 0.42 },
                cooldown: 150,
                ..BASE
            },
            MoveId::CoinRain => MoveData {
                name: "COIN RAIN",
                startup: 40,
                active: 1,
                recovery: 34,
                damage: 16.0,
                reach: 12.0,
                half_angle: 0.0,
                track: 30,
                hitstop: 5,
                range: (0.0, 12.0),
                shape: Shape::Traps { count: 7, radius: 0.85, fuse: 42 },
                cooldown: 300,
                ..BASE
            },
            MoveId::HoardSlam => MoveData {
                name: "HOARD SLAM",
                startup: 42,
                active: 6,
                recovery: 50,
                damage: 32.0,
                reach: 1.3,
                half_angle: deg(35.0),
                track: 28,
                hitstop: 8,
                range: (0.0, 1.2),
                off: (0.0, deg(60.0)),
                shape: Shape::Quake { radius: 2.4, share: 0.45 },
                ..BASE
            },
            // Lucifer's (Pride)
            MoveId::RadiantCut => MoveData { name: "RADIANT CUT", ..PRINCE_PLACEHOLDER },
            // Leviathan's (Envy)
            MoveId::CoilLash => MoveData { name: "COIL LASH", ..PRINCE_PLACEHOLDER },
            // Amon's (Wrath)
            MoveId::FlameBreath => MoveData { name: "FLAME BREATH", ..PRINCE_PLACEHOLDER },
            // Belphegor's (Sloth)
            MoveId::Swat => MoveData { name: "SWAT", ..PRINCE_PLACEHOLDER },
            // Beelzebub's (Gluttony)
            MoveId::Cleaver => MoveData { name: "CLEAVER", ..PRINCE_PLACEHOLDER },
            // Asmodeus's (Lust)
            MoveId::LanceThrust => MoveData { name: "LANCE THRUST", ..PRINCE_PLACEHOLDER },
        }
    }

    /// One of the hero's.
    pub fn is_hero(self) -> bool {
        Self::HERO.contains(&self)
    }
}

impl MoveData {
    pub const fn total(&self) -> u16 {
        self.startup + self.active + self.recovery
    }

    /// The phase of frame `frame` (0 = the first startup frame).
    pub fn phase(&self, frame: u16) -> Phase {
        if frame < self.startup {
            Phase::Startup
        } else if frame < self.startup + self.active {
            Phase::Active
        } else {
            Phase::Recovery
        }
    }

    /// The first frame from which the move may be cancelled.
    pub fn cancel_frame(&self) -> u16 {
        (self.startup + self.active).saturating_add(self.cancel)
    }

    /// A blow, not a spell: it deals damage.
    pub fn strikes(&self) -> bool {
        self.damage > 0.0
    }

    /// Falls on a mark on the floor (or bursts from coins lying on it): no guard stops it, nor parries it.
    pub fn from_the_ground(&self) -> bool {
        matches!(self.shape, Shape::Mark { .. } | Shape::Traps { .. })
    }

    /// The guard can block it (it is neither a grab nor from the ground).
    pub fn blockable(&self) -> bool {
        !self.unblockable && !self.from_the_ground()
    }

    /// The parry can turn it (anything but what bursts from the ground).
    pub fn parriable(&self) -> bool {
        !self.from_the_ground()
    }

    /// The blow (of `hits`) under way on active frame `frame` (0-based from the first active frame).
    pub fn hit_index(&self, frame: u16) -> u8 {
        let per = (self.active / self.hits.max(1) as u16).max(1);
        ((frame / per) as u8).min(self.hits.max(1) - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::combat::fsm::{GUARD_RAISE, PARRY_FROM, PARRY_TO};

    #[test]
    fn every_foe_attack_can_be_read_before_it_lands() {
        for m in MoveId::FOE {
            let d = m.data();
            assert!(d.startup >= 18, "{}: {} f of startup is under the 300 ms reaction floor", d.name, d.startup);
        }
    }

    #[test]
    fn every_foe_attack_can_be_punished_when_it_misses() {
        let l1 = MoveId::L1.data();
        for m in MoveId::FOE {
            let d = m.data();
            assert!(
                d.recovery >= l1.startup + l1.active,
                "{}: recovery {} f is shorter than L1 ({} f to land)",
                d.name,
                d.recovery,
                l1.startup + l1.active
            );
        }
        // The slam's recovery holds a whole string if the punish starts by frame 12.
        let slam = MoveId::Slam.data();
        let l3_lands = 14 + 13 + MoveId::L3.data().startup;
        assert!(slam.recovery >= 12 + l3_lands, "slam {} f vs {} f", slam.recovery, 12 + l3_lands);
    }

    #[test]
    fn every_melee_foe_attack_can_be_parried_on_reaction() {
        const { assert!(PARRY_TO - PARRY_FROM + 1 == 8 && GUARD_RAISE == PARRY_FROM) };
        for m in MoveId::FOE.iter().filter(|m| !m.data().projectile) {
            let d = m.data();
            // Guard pressed REACTION frames after the tell begins is up before the hit, and a
            // later press can still put the hit inside the window.
            assert!(d.startup >= REACTION + PARRY_FROM, "{}: {} f is too quick to parry on sight", d.name, d.startup);
        }
    }

    #[test]
    fn what_cannot_be_guarded_gives_time_to_step_out_of_it() {
        // A mark on the floor is answered by walking out of it. Setting out on seeing it, a hero at
        // his own pace (less what getting going costs) gains on a mark that follows at
        // MARK_FOLLOW, then on one fixed, enough to clear its radius and his own with a fifth to spare.
        let walk = 6.5 * crate::logic::levels::SPEED_SCALE;
        for m in MoveId::FOE {
            let d = m.data();
            let Shape::Mark { radius } = d.shape else { continue };
            let following = d.track.saturating_sub(REACTION) as f64 * (walk - MARK_FOLLOW);
            let fixed = (d.startup - d.track.max(REACTION)) as f64 * walk;
            let gained = (following + fixed) / 60.0 - 0.25;
            assert!(gained >= (radius + 0.3) * 1.2, "{}: {gained:.2} t to clear {radius} t", d.name);
        }
    }

    #[test]
    fn a_coin_on_the_floor_bursts_late_enough_to_step_off_it() {
        // Coins land where he stands; setting out on seeing them, at his own pace (less what getting
        // going costs), he clears a burst's radius and his own with a fifth to spare.
        let walk = 6.5 * crate::logic::levels::SPEED_SCALE;
        let mut seen = 0;
        for m in MoveId::FOE {
            let d = m.data();
            let Shape::Traps { radius, fuse, count } = d.shape else { continue };
            seen += 1;
            assert!(count >= 2 && d.from_the_ground() && !d.blockable() && !d.parriable(), "{}", d.name);
            let gained = fuse.saturating_sub(REACTION) as f64 * walk / 60.0 - 0.25;
            assert!(gained >= (radius + 0.3) * 1.2, "{}: {gained:.2} t to clear {radius} t", d.name);
        }
        assert_eq!(seen, 2, "the coin toss and the coin rain");
    }

    #[test]
    fn the_flurry_spreads_its_bites_over_its_active_frames() {
        let d = MoveId::Heads.data();
        let bites: Vec<u8> = (0..d.active).map(|f| d.hit_index(f)).collect();
        assert_eq!(bites.first(), Some(&0));
        assert_eq!(bites.last(), Some(&(d.hits - 1)));
        assert_eq!(MoveId::Slam.data().hit_index(5), 0);
        // Spells deal nothing; the ground's blows go round the guard.
        assert!(!MoveId::Call.data().strikes() && !MoveId::Raise.data().strikes());
        assert!(!MoveId::Burrow.data().blockable() && !MoveId::Burrow.data().parriable());
        assert!(!MoveId::Grab.data().blockable() && MoveId::Grab.data().parriable());
        assert!(MoveId::Slam.data().blockable() && MoveId::Slam.data().parriable());
    }

    #[test]
    fn the_strings_frames_and_damage_add_up() {
        let (l1, l2, l3) = (MoveId::L1.data(), MoveId::L2.data(), MoveId::L3.data());
        assert_eq!((l1.total(), l2.total(), l3.total()), (24, 25, 36));
        assert_eq!((l1.cancel_frame(), l2.cancel_frame()), (14, 13), "the chain comes out at f14, then f27");
        let string = l1.cancel_frame() + l2.cancel_frame() + l3.total();
        assert_eq!(string, 63, "1.05 s");
        let dps = (l1.damage + l2.damage + l3.damage) / (string as f64 / 60.0);
        assert!((dps - 35.24).abs() < 0.01, "{dps}");
        assert_eq!(l1.phase(5), Phase::Startup);
        assert_eq!(l1.phase(6), Phase::Active);
        assert_eq!(l1.phase(10), Phase::Recovery);
    }
}
