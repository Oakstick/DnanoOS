//! Each hero's four trees of progression: an armory of ten legendary
//! weapons, a path of ten skills, ten shields and ten elixirs, in his own
//! legend (Norse for Ragnar, Turkish for Mehmed, Japanese for Miyamoto).
//! Weapons and path skills are each hero's own; shields and elixirs work the
//! same for all three, tier for tier.
//!
//! Tiers 1 to 5 are sold by the old gatekeeper for silver. Tiers 6 to 10 are
//! sealed: a relic found in a hidden shrine or taken from a fallen Cyclops
//! unseals the next one, or the old man trades it for silver and rune
//! fragments. What is unlocked, the silver and the runes are kept between
//! adventures.

use super::angels::Archangel;
use super::heroes::HeroKind;
use super::types::PotionKind;

/// The four trees, in the order of the old man's tabs.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Tree {
    Armory,
    Path,
    Shield,
    Elixir,
}

pub const TREES: [Tree; 4] = [Tree::Armory, Tree::Path, Tree::Shield, Tree::Elixir];
pub const TIERS: u8 = 10;
/// From this tier on a tree is sealed: found as a relic, or traded for runes.
pub const SEALED_FROM: u8 = 6;

impl Tree {
    pub fn index(self) -> usize {
        match self {
            Tree::Armory => 0,
            Tree::Path => 1,
            Tree::Shield => 2,
            Tree::Elixir => 3,
        }
    }

    /// The tab's short label.
    pub fn label(self) -> &'static str {
        match self {
            Tree::Armory => "WEAPONS",
            Tree::Path => "PATH",
            Tree::Shield => "SHIELDS",
            Tree::Elixir => "ELIXIRS",
        }
    }

    /// What the tree is called in the hero's own legend.
    pub fn title(self, hero: HeroKind) -> &'static str {
        match (hero, self) {
            (HeroKind::Ragnar, Tree::Armory) => "Divine Armory",
            (HeroKind::Ragnar, Tree::Path) => "Heroic Path",
            (HeroKind::Ragnar, Tree::Shield) => "The Shield-Wall",
            (HeroKind::Ragnar, Tree::Elixir) => "Rune-Carved Elixirs",
            (HeroKind::Mehmed, Tree::Armory) => "Imperial Armory",
            (HeroKind::Mehmed, Tree::Path) => "Janissary Path",
            (HeroKind::Mehmed, Tree::Shield) => "Kalkans of the Hearth",
            (HeroKind::Mehmed, Tree::Elixir) => "Corbaci's Elixirs",
            (HeroKind::Miyamoto, Tree::Armory) => "Legendary Armory",
            (HeroKind::Miyamoto, Tree::Path) => "Bushido Path",
            (HeroKind::Miyamoto, Tree::Shield) => "Guards of the Castle",
            (HeroKind::Miyamoto, Tree::Elixir) => "Spirit Brew Elixirs",
        }
    }

    /// What a level of the tree is called ("TIER 4", "LEVEL 4").
    pub fn step(self) -> &'static str {
        if self == Tree::Path {
            "LEVEL"
        } else {
            "TIER"
        }
    }
}

/// How an entry's icon (and, for weapons and shields, its model) is shaped.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Form {
    Knife,
    Sword,
    Saber,
    Katana,
    Axe,
    TwinAxes,
    TwinBlades,
    Wand,
    Spear,
    Hammer,
    Mace,
    Bow,
    Sickle,
    /// A flintlock pistol.
    Gun,
    /// A double-pointed greatsword.
    Forked,
    Round,
    Kite,
    Spiral,
    Tate,
    Mirror,
    Flask,
    Vial,
    Gourd,
    /// A skill: a rune stone carved with glyph n.
    Rune(u8),
}

/// One entry of a tree, as the old man's shelf shows it.
#[derive(Clone, Copy, Debug)]
pub struct Node {
    pub name: &'static str,
    /// Its legend, in a line.
    pub lore: &'static str,
    /// The name of what it lets the hero do.
    pub ability: &'static str,
    /// What that does in play.
    pub effect: &'static str,
    pub color: u32,
    pub form: Form,
}

struct Entry(&'static str, &'static str, &'static str, u32, Form);

use Form::*;

// -------------------------------------------------------------- Ragnar

const RAGNAR_ARMORY: [Entry; 10] = [
    Entry("Bronze Hrotti", "A primitive forged short-blade bearing basic rune scratches.", "Basic Strike", 0xd89a5a, Knife),
    Entry("Iron Skofnung", "Folded iron infused with cold northern river water.", "Throwing Arc", 0xa8b8c8, Sword),
    Entry("Steel Angurvadel", "The \"Stream of Anguish\", radiating a dull vengeful hum.", "Stream of Anguish", 0x9ad1ff, Sword),
    Entry("Tyr's Rune-Etched Axe", "Dedicated to the One-Armed God; strikes true against chaos.", "Binding Oath", 0xe0c070, Axe),
    Entry("Dwarven Masterwork Dual Axes", "Forged deep within Svartalfheim by master smith brothers.", "Spinning Fury", 0xff9a4a, TwinAxes),
    Entry("Laevateinn Wand-Sword", "Harvested from the tree of death, harboring dark arcane flames.", "Loki's Trick", 0x9a4aff, Wand),
    Entry(
        "Mistilteinn Magic Blade",
        "The shining branch that once felled Baldr; radiates pure fatal light.",
        "Baleful Glow",
        0xc8ff9a,
        Sword,
    ),
    Entry("Gram, Sigurd's Bane", "The dragon-slayer, split and reforged to cleave through scales.", "Dragon-Blood Cleave", 0xe0303a, Sword),
    Entry("Gungnir, Odin's Spear", "The celestial spear that never misses its chosen mark.", "All-Father's Guidance", 0xffd84a, Spear),
    Entry("Mjolnir, Thor's Hammer", "The ultimate thunder-bringer forged in dying star-fire.", "Thunderstrike", 0x7ae8ff, Hammer),
];

const RAGNAR_PATH: [Entry; 10] = [
    Entry("Berserker's Rage", "Channels raw primal fury into immediate physical output.", "Frenzied Adrenaline", 0xff5a3a, Rune(0)),
    Entry("Shield-Maiden's Guard", "Defensive positioning grounded in iron-wall discipline.", "Phalanx Posture", 0x8fb4ff, Rune(1)),
    Entry("Seer's Sight", "Third-eye perception gifted by communion with the Norns.", "Odin's All-Sight", 0xc89aff, Rune(2)),
    Entry("Skald's Song", "Epic verse chanted in battle to uplift allied morale.", "Heroic Hymn", 0xffd27a, Rune(3)),
    Entry("Valkyrie's Flight", "Leaping bounds mimicking the choosers of the slain.", "Bifrost Step", 0x9af0ff, Rune(4)),
    Entry("Fenris-Bite", "Predatory focus modeled after the great wolf bound by Gleipnir.", "Wolf's Fang", 0xb8c0cc, Rune(5)),
    Entry("Dwarven Frenzy", "Unyielding stamina channeled straight from mountain stone.", "Forge-Heart", 0xff8a2a, Rune(6)),
    Entry("Blood-Eagle Execution", "Terrifying ritualistic execution that shatters enemy morale.", "Terrify", 0xc8202a, Rune(7)),
    Entry("Valkyrie Call", "Invoking celestial aid from the halls of Valhalla.", "Revive", 0xfff1a8, Rune(8)),
    Entry("Yggdrasil Heal", "Drawing direct sap-line energy from the World Tree roots.", "Cosmic Restoration", 0x6ae86a, Rune(9)),
];

const RAGNAR_SHIELD: [Entry; 10] = [
    Entry("Linden Board Shield", "Light planks of linden wood, rimmed in rawhide.", "Brace", 0xb8864a, Round),
    Entry("Iron-Boss Round Shield", "A hammered iron boss at its heart turns a blade aside.", "Iron Heart", 0xb8c0cc, Round),
    Entry("Rawhide Warboard", "Wet hides stretched over oak, that tighten again as they dry.", "Self-Mend", 0xa0784a, Round),
    Entry("Jarl's Painted Shield", "Painted in a jarl's colours; its rim has broken many noses.", "Rim Strike", 0xe0443a, Round),
    Entry("Oak of the Shield-Wall", "Cut from the oak that stood behind the last shield-wall.", "Arrow Wall", 0x8a6a3a, Kite),
    Entry("Ullr's Ship", "The skalds call a shield Ullr's ship; this one sails back what strikes it.", "Ullr's Rebound", 0x9ad1ff, Round),
    Entry("Hildr's Warboard", "Carried by the Valkyrie whose very name means battle.", "Valkyrie's Clash", 0xfff1a8, Kite),
    Entry("Brokkr's Bulwark", "Riveted by the dwarf who forged Mjolnir; no blow can split it.", "Unsplittable", 0x8fa8c8, Kite),
    Entry("Hel's Bone Shield", "Built from the bones of Helheim; its touch bites back.", "Grave Chill", 0xe2d6bc, Round),
    Entry("Svalinn", "The shield that stands before the sun, without which the world would burn.", "Sunshield", 0xffd84a, Round),
];

const RAGNAR_ELIXIR: [Entry; 10] = [
    Entry("Minor Health Potion", "Spring water mixed with crushed rowan berries.", "Instant Restoration", 0x3fd24a, Flask),
    Entry("Minor Stamina Potion", "Brewed mint and deer blood extract.", "Quick Breath", 0x4ac8ff, Flask),
    Entry("Strength Brew", "Dark ale spiked with crushed bear claw powder.", "Boar's Might", 0xa0302a, Gourd),
    Entry("Elixir of Haste", "Lightning-struck moss steeped in mead.", "Swift Strider", 0xffd23a, Vial),
    Entry("Shield Potion", "Liquid iron tincture that hardens external skin layers.", "Bark Skin", 0x9aa4b0, Flask),
    Entry("Berserker's Draught", "Highly toxic mushroom distillate.", "Pain Nullification", 0xff4a8a, Gourd),
    Entry("Dragonfire Serum", "Filtered bile glands from cold-drake corpses.", "Ignited Blood", 0xff7a2a, Vial),
    Entry("Invulnerability Elixir", "Mystic mixture blessed under a total lunar eclipse.", "Baldr's Blessing", 0xffe066, Flask),
    Entry("Ragnarok Tonic", "The ultimate volatility brew hinting at end-times.", "World's End", 0x9a2aff, Vial),
    Entry("Healing Spirit Elixir", "Pure distillation of Einherjar soul-essence.", "Valkyrie's Embrace", 0xa6f0ff, Flask),
];

// -------------------------------------------------------------- Mehmed

const MEHMED_ARMORY: [Entry; 10] = [
    Entry(
        "Bronze Yatagan",
        "A standard-issue curved short sword of the frontier guards bearing simple crescent stamps.",
        "Swift Crescent",
        0xd89a5a,
        Knife,
    ),
    Entry("Iron Kilic", "A traditional cavalry sabre forged for sweeping battlefield maneuvers.", "Saber Sweep", 0xa8b8c8, Saber),
    Entry(
        "Steel Pata Guard",
        "A gauntlet-sword combination offering unbreakable wrist control and piercing power.",
        "Gauntlet Thrust",
        0x9ad1ff,
        Sword,
    ),
    Entry(
        "Dervish Crescent Blade",
        "A sacred blade blessed with whirling dervish devotion, humming with spiritual kinetic force.",
        "Whirling Devotion",
        0xf2e3c4,
        Saber,
    ),
    Entry(
        "Koroglu's Steel Flintlock",
        "The legendary bandit hero's firearm, firing enchanted lead slugs that never miss.",
        "Bandit's Shot",
        0x8a8a94,
        Gun,
    ),
    Entry(
        "Janissary Mace of Conquest",
        "A heavy flanged iron mace designed to shatter armor and fortifications.",
        "Shattering Blow",
        0xc8c8d8,
        Mace,
    ),
    Entry(
        "Zulfikar-Infused Greatsword",
        "A sacred double-pointed divine blade modeled after Ali's legendary sword.",
        "Divine Split",
        0x6ae8a0,
        Forked,
    ),
    Entry(
        "Simurg Feather Bow",
        "A composite bow strung with a tail feather of the mythical bird of wisdom.",
        "Simurg's Gaze",
        0xff9a4a,
        Bow,
    ),
    Entry(
        "Masterwork Djezzar Saber",
        "Folded steel carved with ancient victory runes from the imperial archives.",
        "Imperial Authority",
        0xeef4ff,
        Saber,
    ),
    Entry(
        "Kaf Dagi Sun-Blade",
        "Forged from meteoric iron at the edge of the mythical Mount Qaf, channeling eternal light.",
        "Mountain of Light",
        0xffd84a,
        Sword,
    ),
];

const MEHMED_PATH: [Entry; 10] = [
    Entry("Ocak Discipline", "Strict military training instilling unbreakable battlefield focus.", "Drill Adrenaline", 0xff5a3a, Rune(0)),
    Entry("Kalkan Guard", "Large battle shield formation rooted in imperial vanguard tactics.", "Iron Rampart", 0x8fb4ff, Rune(1)),
    Entry("Anatolian Seer Sight", "Mystical intuition inspired by legendary dervish visions.", "Hizir's Guidance", 0xc89aff, Rune(2)),
    Entry(
        "Mehteran War Chant",
        "Traditional marching drums and zurna frequencies that inspire fierce morale.",
        "Janissary Hymn",
        0xffd27a,
        Rune(3),
    ),
    Entry(
        "Alp Step",
        "Legendary warrior footwork allowing lightning-fast evasion and tactical repositioning.",
        "Zephyr Dash",
        0xe8d09a,
        Rune(4),
    ),
    Entry(
        "Devshirme Critical Focus",
        "Unflinching precision forged through a lifetime of elite martial trial.",
        "Vanguard's Strike",
        0xb8c0cc,
        Rune(5),
    ),
    Entry(
        "Koroglu's Defiance",
        "Rebellious spirit drawn from mountain folklore, shaking off suppression.",
        "Rebel Heart",
        0xff8a2a,
        Rune(6),
    ),
    Entry(
        "Heroic Last Stand",
        "Oath-bound defense that converts mortal peril into explosive retaliation.",
        "Blood of the Empire",
        0xc8202a,
        Rune(7),
    ),
    Entry("Ancestor's Call", "Invoking the protective spirits of past empire commanders.", "Janissary Vanguard", 0xa8c4ff, Rune(8)),
    Entry(
        "Rose Garden Restoration",
        "Channeling pure spiritual healing from the mystical gardens of paradise.",
        "Eternal Blossom",
        0xff8ab0,
        Rune(9),
    ),
];

const MEHMED_SHIELD: [Entry; 10] = [
    Entry("Wicker Kalkan", "Fig-wood cane wound in a spiral with silk thread.", "Brace", 0xc8a060, Spiral),
    Entry("Iron-Boss Kalkan", "A spiral kalkan with a hammered iron boss.", "Iron Heart", 0xb8c0cc, Spiral),
    Entry("Silk-Wound Kalkan", "Its threads draw tight again after every blow.", "Self-Mend", 0xd8483a, Spiral),
    Entry("Janissary Steel Kalkan", "Pressed steel with a sharp rim for pushing back.", "Rim Strike", 0xa8b0bc, Round),
    Entry("Crescent Buckler", "A steel crescent that catches arrows in its horns.", "Arrow Ward", 0xd8d8e0, Round),
    Entry(
        "Mirror of Iskender",
        "Alexander's polished mirror, turning every strike back on its sender.",
        "Iskender's Mirror",
        0x9ad1ff,
        Mirror,
    ),
    Entry("Seljuk Star Shield", "Its eight-pointed star rings like a bell when struck.", "Ringing Star", 0x3a8ad8, Round),
    Entry("Rumeli Bastion", "Plated like the walls of the Rumelian fortress; it will not fall.", "Unbreachable", 0x8fa8c8, Kite),
    Entry("Ejderha Scale Shield", "Dragon scales that burn whatever touches them.", "Dragon Scales", 0x4a8a4a, Kite),
    Entry(
        "Seal of Suleyman",
        "The wise king's ring-seal set in gold; blows that strike it heal the bearer.",
        "Seal of Wisdom",
        0xffd84a,
        Round,
    ),
];

const MEHMED_ELIXIR: [Entry; 10] = [
    Entry("Minor Sherbet Draught", "Sweetened pomegranate and rosewater spring blend.", "Instant Restoration", 0xff6a8a, Flask),
    Entry("Minor Stamina Infusion", "Brewed mountain mint and wild honey extract.", "Quick Breath", 0x8ae07a, Gourd),
    Entry("Janissary Strength Coffee", "Dark, thick Turkish coffee spiked with crushed spice root.", "Sultan's Might", 0x7a5030, Vial),
    Entry("Elixir of the Desert Wind", "Distilled cactus sap mixed with swift-running herbs.", "Swift Strider", 0xffd23a, Vial),
    Entry("Brass Shield Tincture", "Liquid brass tincture that hardens external skin layers.", "Armor of the Gate", 0xc8a040, Flask),
    Entry("Berserker's Dervish Draught", "Fermented grape juice infused with wild steppe roots.", "Whirling Frenzy", 0x9a2a7a, Gourd),
    Entry("Simurg Flame Elixir", "Filtered extract from mythical phoenix-feather ash.", "Simurg's Fire", 0xff7a2a, Vial),
    Entry("Invulnerability Oil", "Mystic oil blessed under a total solar eclipse.", "Iron Body", 0xffe066, Flask),
    Entry("Mount Qaf Volatility Tonic", "Rare mineral distillation from the legendary peaks.", "Peak Surge", 0x9a2aff, Vial),
    Entry("Ab-i Hayat, Water of Life", "Pure mythical elixir granting eternal rejuvenation.", "Immortal Grace", 0xa6e0ff, Flask),
];

// ------------------------------------------------------------ Miyamoto

const MIYAMOTO_ARMORY: [Entry; 10] = [
    Entry("Katana of the Iron Gate", "A standard-issue steel blade folded with basic iron sand.", "Swift Slash", 0xc8d0da, Katana),
    Entry("Oni-Slayer Nodachi", "A massive field-clearing sword designed to sever ogre limbs.", "Heavy Sweep", 0xa8b8c8, Sword),
    Entry("Tengu Wind-Blade", "Infused with mountain spirit air, humming with kinetic force.", "Gale Dash", 0x9af0d0, Saber),
    Entry("Kusanagi-no-Tsurugi Fragment", "A jagged shard of the legendary Grass-Cutter sword.", "Divine Ignition", 0xff9a4a, Knife),
    Entry(
        "Dual Ninjato of the Shadow",
        "Matched short blades forged for silent, lightning-fast execution.",
        "Shadow Spin",
        0x7a7a9a,
        TwinBlades,
    ),
    Entry(
        "Yari of the Shifting Serpent",
        "A venom-tipped spear associated with the mythical Yamata no Orochi.",
        "Venom Thrust",
        0x6ae86a,
        Spear,
    ),
    Entry("Muramasa Cursed Blade", "A blade driven by bloodlust that feeds on its wielder's vitality.", "Blood Drinker", 0xc83a5a, Katana),
    Entry("Yumi of the Heavenly Bow", "A composite spirit-wood bow that calls down celestial arrows.", "Storm of Feathers", 0xc8e0ff, Bow),
    Entry(
        "Masamune Masterpiece Katana",
        "Forged with absolute purity, cutting only what the wielder truly intends.",
        "Purity Strike",
        0xeef4ff,
        Katana,
    ),
    Entry(
        "Amaterasu's Sun-Forged Blade",
        "Channeling the blinding solar light of the celestial sun goddess.",
        "Solar Flash",
        0xffd84a,
        Sword,
    ),
];

const MIYAMOTO_PATH: [Entry; 10] = [
    Entry("Samurai's Focus", "Centering the mind through rigorous martial meditation.", "Adrenaline Rush", 0xff5a3a, Rune(0)),
    Entry("Guardian's Stance", "Defensive footing grounded in heavy armor discipline.", "Iron Wall", 0x8fb4ff, Rune(1)),
    Entry("Tengu's Agility", "Mountain-spirit footwork enabling impossible evasions.", "Wind Step", 0x9af0d0, Rune(2)),
    Entry("Bards of the Court", "Inspiring war chants recalling historic feudal victories.", "Ronin's Resolve", 0xffd27a, Rune(3)),
    Entry("Shadow Step", "Movement technique mirroring ninja arts and smoke concealment.", "Blink Dash", 0xffb0d0, Rune(4)),
    Entry("Kitsune's Cunning", "Trickster-fox focus targeting enemy blind spots.", "Fox's Fang", 0xffa050, Rune(5)),
    Entry("Temple Guardian Frenzy", "Unyielding spirit drawn from sacred shrine stone.", "Zenith Heart", 0xff8a2a, Rune(6)),
    Entry("Seppuku Resolve", "Ultimate commitment to honor that converts near-death into power.", "Last Stand", 0xc8202a, Rune(7)),
    Entry("Spirit Summon", "Calling upon ancestral guardian spirits of the clan.", "Ancestor's Guard", 0xa8c4ff, Rune(8)),
    Entry("Lotus Blossom Restoration", "Channeling pure spiritual energy from the sacred pond.", "Nirvana Cleanse", 0xffb0e0, Rune(9)),
];

const MIYAMOTO_SHIELD: [Entry; 10] = [
    Entry("Bamboo Tate", "A standing shield of lashed bamboo.", "Brace", 0xc8b870, Tate),
    Entry("Lacquered Sode", "A great shoulder guard of lacquered iron scales.", "Iron Heart", 0xb8222a, Tate),
    Entry("Rope-Bound Tate", "Hemp that tightens as it dries, mending its gaps.", "Self-Mend", 0xa08a5a, Tate),
    Entry("Tetsu Kote Guard", "Iron armguards that strike as they block.", "Rim Strike", 0x8a8a94, Round),
    Entry("Temple Gate Shield", "Cut from the heavy doors of a mountain temple.", "Arrow Ward", 0x8a3a2a, Tate),
    Entry(
        "Yata no Kagami",
        "The sun goddess's sacred mirror: what strikes it sees itself, and turns back.",
        "Mirror of Truth",
        0x9ad1ff,
        Mirror,
    ),
    Entry("Oni-Face Shield", "A demon's snarl beaten into iron; foes reel when it rings.", "Demon's Snarl", 0xd84a3a, Round),
    Entry("Himeji Wall", "Plastered like the White Heron castle, that never fell.", "Unfallen", 0xeeeae0, Tate),
    Entry("Dragon-Scale Tate", "Scales shed by a river dragon, cold and cutting.", "Dragon Scales", 0x4a8a8a, Tate),
    Entry(
        "Shield of Hachiman",
        "Blessed by the god of war, protector of warriors: its blocks renew the bearer.",
        "Hachiman's Blessing",
        0xffd84a,
        Round,
    ),
];

const MIYAMOTO_ELIXIR: [Entry; 10] = [
    Entry("Minor Health Draught", "Spring water infused with crushed green tea leaves.", "Instant Restoration", 0x7ac84a, Flask),
    Entry("Minor Stamina Dew", "Mountain mist captured in bamboo flasks.", "Quick Breath", 0x9ae0ff, Gourd),
    Entry("Tiger Strength Sake", "Aged rice wine spiked with fiery ginger and pepper.", "Byakko's Might", 0xf0a040, Gourd),
    Entry("Elixir of the Wind", "Essence harvested from flying Tengu feathers.", "Swift Strider", 0xffd23a, Vial),
    Entry("Turtle Shell Tincture", "Liquid iron tincture that hardens external skin layers.", "Genbu's Shell", 0x6a8a7a, Flask),
    Entry("Berserker's Oni Brew", "Toxic distillate brewed in volcanic ogre caves.", "Oni's Rage", 0xff4a4a, Gourd),
    Entry("Dragonfire Elixir", "Filtered bile glands from mythological eastern dragons.", "Ryujin's Fire", 0xff7a2a, Vial),
    Entry("Invulnerability Tincture", "Mystic mixture blessed under a blood-moon eclipse.", "Kami's Blessing", 0xffe066, Flask),
    Entry("Yomi Volatility Tonic", "Underworld fluid hinting at the realm of the dead.", "Underworld Surge", 0x9a2aff, Vial),
    Entry("Amrita Spirit Essence", "Pure distillation of glowing soul-stones.", "Celestial Embrace", 0xa6f0ff, Flask),
];

fn table(hero: HeroKind, tree: Tree) -> &'static [Entry; 10] {
    match (hero, tree) {
        (HeroKind::Ragnar, Tree::Armory) => &RAGNAR_ARMORY,
        (HeroKind::Ragnar, Tree::Path) => &RAGNAR_PATH,
        (HeroKind::Ragnar, Tree::Shield) => &RAGNAR_SHIELD,
        (HeroKind::Ragnar, Tree::Elixir) => &RAGNAR_ELIXIR,
        (HeroKind::Mehmed, Tree::Armory) => &MEHMED_ARMORY,
        (HeroKind::Mehmed, Tree::Path) => &MEHMED_PATH,
        (HeroKind::Mehmed, Tree::Shield) => &MEHMED_SHIELD,
        (HeroKind::Mehmed, Tree::Elixir) => &MEHMED_ELIXIR,
        (HeroKind::Miyamoto, Tree::Armory) => &MIYAMOTO_ARMORY,
        (HeroKind::Miyamoto, Tree::Path) => &MIYAMOTO_PATH,
        (HeroKind::Miyamoto, Tree::Shield) => &MIYAMOTO_SHIELD,
        (HeroKind::Miyamoto, Tree::Elixir) => &MIYAMOTO_ELIXIR,
    }
}

/// What tier `tier` (1..=10) of `hero`'s tree does in play.
pub fn effect(hero: HeroKind, tree: Tree, tier: u8) -> &'static str {
    const MEHMED_ARMORY: [&str; 10] = [
        "DMG 1. A quick slicing draw.",
        "DMG 2. A wide horizontal arc through every foe in front.",
        "DMG 3. A piercing thrust drives a line of force down the corridor.",
        "DMG 3. Every strike is a 360-degree spin.",
        "DMG 4. A pistol shot down the line that always staggers the Cyclops.",
        "DMG 5. Shatters armour: the Cyclops' hide turns nothing.",
        "DMG 5. A huge sweeping cut that leaves a trail of holy fire.",
        "DMG 2 an arrow. Rains fiery arrows on up to five foes and the Cyclops, setting it alight.",
        "DMG 6. Every hit is critical, and stuns the foes around the blow.",
        "DMG 7. A flash of solar fire obliterates every foe around you, and the blade is back in your grip.",
    ];
    const MIYAMOTO_ARMORY: [&str; 10] = [
        "DMG 1. A quick draw: the fastest strike there is.",
        "DMG 2. A wide horizontal arc through every foe in front.",
        "DMG 3. Lunges forward in a whirlwind slash.",
        "DMG 3. Sacred flames: the foes around the blow burn, and so does the Cyclops.",
        "DMG 3. Every strike is a 360-degree whirlwind, and you vanish from sight for a moment.",
        "DMG 4. A long thrust whose venom stacks on the Cyclops, hit after hit.",
        "DMG 5. Ignores armour, but every blow that lands drinks 2 of your health.",
        "DMG 2 an arrow. Rains spectral arrows on up to five foes and the Cyclops.",
        "DMG 6. Every hit is critical and ignores armour.",
        "DMG 7. A flash of sunlight obliterates every foe around you, and the blade is back in your grip.",
    ];
    const SAMURAI_PATH: [&str; 10] = [
        "Attacks 15% faster.",
        "+50 health.",
        "Every jump is a dodge (a moment nothing can touch you), and the blink costs half its stamina.",
        "Stamina comes back 60% faster.",
        "F or L3 (press the left stick): a cloud of cherry blossoms carries you three tiles ahead.",
        "Blows from behind strike the Cyclops for triple damage, from the flank for double.",
        "When you are hurt: six seconds in which slime cannot burn you and your blows cost no stamina.",
        "Once per dungeon, when your health hits zero: six seconds in which you cannot die and deal triple damage.",
        "A spectral samurai fights at your side.",
        "Once per dungeon, at a quarter health: fully healed and every ill purged.",
    ];
    const JANISSARY_PATH: [&str; 10] = [
        "Attacks 15% faster.",
        "+50 health.",
        "See hidden vaults and potions through walls, see in the dark, and land critical hits on the Cyclops.",
        "Stamina comes back 60% faster.",
        "F or L3 (press the left stick): a gust of desert wind carries you three tiles ahead.",
        "Blows from behind strike the Cyclops for triple damage, from the flank for double.",
        "When you are hurt: six seconds in which slime cannot burn you and your blows cost no stamina.",
        "Once per dungeon, when your health hits zero: six seconds in which you cannot die and deal triple damage.",
        "A spectral squad of three janissaries fights at your side.",
        "Once per dungeon, at a quarter health: fully healed and every ill purged.",
    ];
    let i = (tier.clamp(1, TIERS) - 1) as usize;
    match (hero, tree) {
        (HeroKind::Mehmed, Tree::Armory) => return MEHMED_ARMORY[i],
        (HeroKind::Miyamoto, Tree::Armory) => return MIYAMOTO_ARMORY[i],
        (HeroKind::Mehmed, Tree::Path) => return JANISSARY_PATH[i],
        (HeroKind::Miyamoto, Tree::Path) => return SAMURAI_PATH[i],
        _ => {}
    }
    const ARMORY: [&str; 10] = [
        "DMG 1. A close strike in front of you.",
        "DMG 2. Thrown in an arc, it flies back to your hand.",
        "DMG 3. Each swing sends a piercing wave down the corridor.",
        "DMG 3. Stuns every foe around the blow.",
        "DMG 3. Every swing is a 360-degree whirlwind.",
        "DMG 4. Leaves a decoy the foes chase, and confuses them.",
        "DMG 5. Ignores the Cyclops' armour.",
        "DMG 5. Its cuts bleed the Cyclops heavily over time.",
        "DMG 6. Thrown, it never misses: every hit is critical.",
        "DMG 7. Thrown, chain lightning leaps from foe to foe, and it returns at once.",
    ];
    const PATH: [&str; 10] = [
        "Attacks 15% faster, at the cost of 10% more damage taken.",
        "+50 health while you carry a shield.",
        "See hidden shrines and potions through walls, see in the dark, and land critical hits on the Cyclops.",
        "Stamina comes back 60% faster.",
        "F or L3 (press the left stick): a flash of light carries you three tiles ahead.",
        "Blows from behind strike the Cyclops for triple damage.",
        "When your stamina runs out: six seconds of free swings, and slime cannot burn you.",
        "Every kill sends the lesser foes around you fleeing in panic.",
        "Once per dungeon, a fatal blow raises you again with half your health.",
        "Once per dungeon, at a quarter health: fully healed and every ill purged.",
    ];
    const SHIELD: [&str; 10] = [
        "Takes 40% of every hit.",
        "Takes 48% of every hit, and lasts longer.",
        "Takes 52%, and mends itself as you go.",
        "Takes 56%; a foe whose blow it catches is struck down.",
        "Takes 60%; the Cyclops' missiles cannot get past it.",
        "Takes 65%; the missiles it catches fly back at the Cyclops.",
        "Takes 70%; every block stuns the foes around you.",
        "Takes 75%, and it can never break.",
        "Takes 80%; the Cyclops is wounded when it touches you.",
        "Takes 85%, and every block heals you.",
    ];
    const ELIXIR: [&str; 10] = [
        "Restores 35 health.",
        "Refills your stamina at once.",
        "+2 weapon damage for 20 seconds.",
        "Double speed and attack speed for 10 seconds.",
        "A 50-health shield over your skin.",
        "15 seconds: half damage taken, and every hit taken adds +1 to your blows.",
        "15 seconds: foes that touch you burn to ash.",
        "Nothing can hurt you for 5 seconds.",
        "20 seconds: double damage and faster everything, while your health drains.",
        "Heals 3 health a second for 30 seconds.",
    ];
    match tree {
        Tree::Armory => ARMORY[i],
        Tree::Path => PATH[i],
        Tree::Shield => SHIELD[i],
        Tree::Elixir => ELIXIR[i],
    }
}

/// Tier `tier` (1..=10) of `hero`'s `tree`.
pub fn node(hero: HeroKind, tree: Tree, tier: u8) -> Node {
    let e = &table(hero, tree)[(tier.clamp(1, TIERS) - 1) as usize];
    Node { name: e.0, lore: e.1, ability: e.2, effect: effect(hero, tree, tier), color: e.3, form: e.4 }
}

// -------------------------------------------------------------- currencies

/// What the hero calls his coin: silver pieces, akce, gold mon.
pub fn coin(hero: HeroKind) -> &'static str {
    match hero {
        HeroKind::Ragnar => "SILVER",
        HeroKind::Mehmed => "AKCE",
        HeroKind::Miyamoto => "MON",
    }
}

/// What he calls the rare stones that buy sealed tiers: rune fragments, turquoise gems, spirit crystals.
pub fn gems(hero: HeroKind, n: u32) -> String {
    let (one, many) = match hero {
        HeroKind::Ragnar => ("RUNE FRAGMENT", "RUNE FRAGMENTS"),
        HeroKind::Mehmed => ("TURQUOISE GEM", "TURQUOISE GEMS"),
        HeroKind::Miyamoto => ("SPIRIT CRYSTAL", "SPIRIT CRYSTALS"),
    };
    if n == 1 {
        one.to_string()
    } else {
        many.to_string()
    }
}

/// Where a dungeon hides its relic: a shrine, a vault, a grotto.
pub fn hidden_place(hero: HeroKind) -> &'static str {
    match hero {
        HeroKind::Ragnar => "hidden shrine",
        HeroKind::Mehmed => "hidden vault",
        HeroKind::Miyamoto => "hidden grotto",
    }
}

// ------------------------------------------------------------------ prices

/// What unlocking a tier costs at the old man's.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Price {
    pub silver: u32,
    pub runes: u32,
}

/// Silver to unlock each tier; tier 1 of every tree but the path is his from the start.
const SILVER: [u32; 10] = [0, 150, 300, 480, 700, 900, 1100, 1300, 1500, 1800];
/// Rune fragments on top, for the sealed tiers.
const RUNES: [u32; 10] = [0, 0, 0, 0, 0, 2, 3, 4, 5, 6];

pub fn price(tree: Tree, tier: u8) -> Price {
    let i = (tier.clamp(1, TIERS) - 1) as usize;
    let silver = if tree == Tree::Path && tier == 1 { 120 } else { SILVER[i] };
    Price { silver, runes: RUNES[i] }
}

pub fn sealed(tier: u8) -> bool {
    tier >= SEALED_FROM
}

/// Silver for one bottle of an unlocked elixir, and for an extra life.
pub fn bottle_price(kind: PotionKind) -> u32 {
    match kind {
        PotionKind::Health => 30,
        PotionKind::Stamina => 25,
        PotionKind::Strength => 60,
        PotionKind::Haste => 70,
        PotionKind::Bark => 80,
        PotionKind::Fury => 100,
        PotionKind::Fire => 110,
        PotionKind::Invulnerability => 140,
        PotionKind::Ragnarok => 160,
        PotionKind::Spirit => 150,
        PotionKind::Life => 350,
    }
}

/// The elixir tier a potion belongs to (1..=10); None for an extra life.
pub fn elixir_tier(kind: PotionKind) -> Option<u8> {
    ELIXIRS.iter().position(|&k| k == kind).map(|i| i as u8 + 1)
}

/// The brews of the elixir tree, tier 1 first.
pub const ELIXIRS: [PotionKind; 10] = [
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
];

// ---------------------------------------------------------- the weapons

/// How an armory weapon fights: a swing, a throw, and what rides on its blows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blade {
    /// Damage to a boss per blow (ordinary foes fall to any blow).
    pub damage: i32,
    /// The swing: tiles of reach ahead, half-width to either side, reach behind. None for thrown weapons and bows.
    pub swing: Option<(f64, f64, f64)>,
    /// Thrown or shot: flies out and back (returns), or on until it hits.
    pub throw: Option<Throw>,
    pub cooldown: f64,
    /// Each swing also sends a piercing wave down the line.
    pub wave: bool,
    /// Seconds foes around the blow are stunned.
    pub stun: f64,
    /// Leaves a decoy and confuses nearby foes.
    pub trick: bool,
    /// Its blows ignore the Cyclops' armour.
    pub pierce_armor: bool,
    /// Seconds its cuts bleed the Cyclops.
    pub bleed: f64,
    /// Every hit is critical.
    pub crit: bool,
    /// Foes chain lightning strikes from where it hits.
    pub chain: usize,
    /// Tiles it lunges forward before the swing.
    pub lunge: f64,
    /// Seconds its flames burn the Cyclops (and the foes round the blow catch fire).
    pub burn: f64,
    /// Each hit adds a stack of venom to the Cyclops.
    pub poison: bool,
    /// Health each blow that lands drinks from its wielder.
    pub blood: i32,
    /// Arrows it rains down on the nearest foes and the Cyclops (a bow).
    pub arrows: usize,
    /// The radius of the blinding flash that obliterates foes around him.
    pub flash: f64,
    /// Each swing leaves a trail of holy fire ahead of him.
    pub fire_trail: bool,
    /// Seconds each swing hides him from sight.
    pub stealth: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Throw {
    pub speed: f64,
    pub range: f64,
    pub radius: f64,
    pub returns: bool,
    pub homing: bool,
    /// Keeps flying through what it hits.
    pub pierce: bool,
    /// Staggers the Cyclops when it hits.
    pub stagger: bool,
}

pub const STUN_RADIUS: f64 = 2.6;
pub const TRICK_RADIUS: f64 = 3.5;
/// A bow's arrows seek foes within this many tiles.
pub const ARROW_RANGE: f64 = 7.0;

const BASE_BLADE: Blade = Blade {
    damage: 1,
    swing: Some((1.2, 0.7, 0.25)),
    throw: None,
    cooldown: 0.42,
    wave: false,
    stun: 0.0,
    trick: false,
    pierce_armor: false,
    bleed: 0.0,
    crit: false,
    chain: 0,
    lunge: 0.0,
    burn: 0.0,
    poison: false,
    blood: 0,
    arrows: 0,
    flash: 0.0,
    fire_trail: false,
    stealth: 0.0,
};

/// Tier `tier` of `hero`'s armory.
pub fn blade(hero: HeroKind, tier: u8) -> Blade {
    let base = BASE_BLADE;
    let t = tier.clamp(1, TIERS);
    let wide = Some((1.5, 1.5, 0.3));
    let whirl = Some((1.6, 1.6, 1.6));
    let flash = Blade { damage: 7, swing: Some((1.5, 1.2, 0.6)), cooldown: 1.0, flash: 3.8, ..base };
    match hero {
        HeroKind::Ragnar => match t {
            1 => base,
            2 => Blade {
                damage: 2,
                swing: None,
                throw: Some(Throw { speed: 11.0, range: 6.0, radius: 0.7, returns: true, homing: false, pierce: true, stagger: false }),
                cooldown: 0.45,
                ..base
            },
            3 => Blade { damage: 3, swing: Some((1.4, 0.8, 0.25)), cooldown: 0.5, wave: true, ..base },
            4 => Blade { damage: 3, swing: Some((1.5, 0.95, 0.3)), cooldown: 0.48, stun: 2.5, ..base },
            5 => Blade { damage: 3, swing: whirl, cooldown: 0.46, ..base },
            6 => Blade { damage: 4, swing: Some((1.6, 1.0, 0.3)), cooldown: 0.45, trick: true, ..base },
            7 => Blade { damage: 5, swing: Some((1.7, 1.1, 0.35)), cooldown: 0.45, pierce_armor: true, ..base },
            8 => Blade { damage: 5, swing: Some((1.8, 1.3, 0.5)), cooldown: 0.5, bleed: 5.0, ..base },
            9 => Blade {
                damage: 6,
                swing: None,
                throw: Some(Throw { speed: 15.0, range: 14.0, radius: 0.65, returns: false, homing: true, pierce: true, stagger: false }),
                cooldown: 0.55,
                crit: true,
                ..base
            },
            _ => Blade {
                damage: 7,
                swing: None,
                throw: Some(Throw { speed: 17.0, range: 7.0, radius: 0.8, returns: true, homing: false, pierce: true, stagger: false }),
                cooldown: 0.5,
                chain: 5,
                ..base
            },
        },
        HeroKind::Mehmed => match t {
            1 => Blade { cooldown: 0.34, ..base },
            2 => Blade { damage: 2, swing: wide, cooldown: 0.5, ..base },
            3 => Blade { damage: 3, swing: Some((1.4, 0.6, 0.2)), cooldown: 0.5, wave: true, ..base },
            4 => Blade { damage: 3, swing: whirl, cooldown: 0.46, ..base },
            5 => Blade {
                damage: 4,
                swing: None,
                throw: Some(Throw { speed: 22.0, range: 12.0, radius: 0.45, returns: false, homing: false, pierce: false, stagger: true }),
                cooldown: 0.7,
                ..base
            },
            6 => Blade { damage: 5, swing: Some((1.5, 1.0, 0.3)), cooldown: 0.5, pierce_armor: true, ..base },
            7 => Blade { damage: 5, swing: Some((1.9, 1.6, 0.6)), cooldown: 0.55, fire_trail: true, ..base },
            8 => Blade { damage: 2, swing: None, cooldown: 0.8, arrows: 5, burn: 3.0, ..base },
            9 => Blade { damage: 6, swing: Some((1.8, 1.2, 0.4)), cooldown: 0.48, crit: true, stun: 2.5, ..base },
            _ => flash,
        },
        HeroKind::Miyamoto => match t {
            1 => Blade { cooldown: 0.34, ..base },
            2 => Blade { damage: 2, swing: wide, cooldown: 0.5, ..base },
            3 => Blade { damage: 3, swing: Some((1.3, 1.2, 1.0)), cooldown: 0.55, lunge: 2.0, ..base },
            4 => Blade { damage: 3, swing: Some((1.5, 1.0, 0.3)), cooldown: 0.48, burn: 4.0, ..base },
            5 => Blade { damage: 3, swing: whirl, cooldown: 0.46, stealth: 0.6, ..base },
            6 => Blade { damage: 4, swing: Some((2.3, 0.55, 0.2)), cooldown: 0.5, poison: true, ..base },
            7 => Blade { damage: 5, swing: Some((1.7, 1.1, 0.35)), cooldown: 0.45, pierce_armor: true, blood: 2, ..base },
            8 => Blade { damage: 2, swing: None, cooldown: 0.8, arrows: 5, ..base },
            9 => Blade { damage: 6, swing: Some((1.8, 1.2, 0.4)), cooldown: 0.45, crit: true, pierce_armor: true, ..base },
            _ => flash,
        },
    }
}

// ---------------------------------------------------------- the shields

/// How a shield of each tier guards. Each keeps every gift of the tiers below it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ward {
    /// Share of each hit it takes instead of him.
    pub block: f64,
    /// Damage it can soak up before it breaks.
    pub durability: i32,
    /// Durability it mends per second.
    pub mend: f64,
    /// A foe whose blow it catches is struck down.
    pub bash: bool,
    /// The Cyclops' missiles never get past it.
    pub missile_proof: bool,
    /// Missiles it catches fly back at the Cyclops.
    pub reflect: bool,
    /// Each block stuns the foes around him.
    pub stagger: bool,
    /// It never breaks.
    pub unbreakable: bool,
    /// The Cyclops is wounded by this much when it touches him.
    pub thorns: i32,
    /// Health each block gives back.
    pub aegis: i32,
    /// Health it adds to his own while it is on his arm.
    pub vigor: i32,
}

pub fn ward(tier: u8) -> Ward {
    let t = tier.clamp(1, TIERS);
    const BLOCK: [f64; 10] = [0.4, 0.48, 0.52, 0.56, 0.6, 0.65, 0.7, 0.75, 0.8, 0.85];
    const DURABILITY: [i32; 10] = [40, 70, 90, 120, 150, 180, 220, 260, 300, 360];
    const MEND: [f64; 10] = [0.0, 0.0, 1.0, 1.0, 1.0, 1.5, 1.5, 2.0, 2.0, 3.0];
    const VIGOR: [i32; 10] = [5, 10, 15, 20, 25, 30, 35, 40, 50, 60];
    let i = (t - 1) as usize;
    Ward {
        block: BLOCK[i],
        durability: DURABILITY[i],
        mend: MEND[i],
        bash: t >= 4,
        missile_proof: t >= 5,
        reflect: t >= 6,
        stagger: t >= 7,
        unbreakable: t >= 8,
        thorns: if t >= 9 { 2 } else { 0 },
        aegis: if t >= 10 { 5 } else { 0 },
        vigor: VIGOR[i],
    }
}

/// Silver to hammer a shield of `tier` back to full from `fraction` left (0 when broken).
pub fn repair_price(tier: u8, fraction: f32) -> u32 {
    let missing = (1.0 - fraction).clamp(0.0, 1.0);
    if missing <= 0.0 {
        return 0;
    }
    (((20 + tier as u32 * 14) as f32 * missing).ceil() as u32).max(5)
}

// ------------------------------------------------------------ the path

/// The path's numbers, level by level.
pub mod path {
    /// Level 1: swings come this much sooner, and hits land this much harder on him.
    pub const RAGE_COOLDOWN: f64 = 0.85;
    pub const RAGE_TAKEN: f64 = 1.1;
    /// Level 2: extra health while a shield is on his arm.
    pub const GUARD_HP: i32 = 50;
    /// Level 3: the chance of a critical hit on the Cyclops.
    pub const SIGHT_CRIT: f64 = 0.2;
    /// Level 4: stamina comes back this much faster.
    pub const SONG_REGEN: f64 = 1.6;
    /// Level 5: the blink: how far, how often, what it costs.
    pub const STEP_TILES: f64 = 3.0;
    pub const STEP_COOLDOWN: f64 = 1.4;
    pub const STEP_STAMINA: f64 = 25.0;
    /// Level 6: a blow from behind.
    pub const FANG_MULT: i32 = 3;
    /// Level 7: free swings when his stamina runs dry, and how soon it can come again.
    pub const HEART_SECONDS: f64 = 6.0;
    pub const HEART_COOLDOWN: f64 = 25.0;
    /// Level 8: how far a kill's terror reaches and how long the foes flee.
    pub const TERROR_RADIUS: f64 = 3.5;
    pub const TERROR_SECONDS: f64 = 3.0;
    /// Level 10: the World Tree answers at this share of health.
    pub const TREE_AT: f64 = 0.25;
    /// Mehmed's and Miyamoto's level 6: a blow from the flank.
    pub const FLANK_MULT: i32 = 2;
    /// Miyamoto's level 3: the moment a jump leaves him untouchable, and the blink's cost.
    pub const DODGE_SECONDS: f64 = 0.35;
    pub const WIND_STEP_COST: f64 = 0.5;
    /// Mehmed's and Miyamoto's level 7: how long the unyielding heart lasts once hurt, and how often.
    pub const DEFIANCE_SECONDS: f64 = 6.0;
    pub const DEFIANCE_COOLDOWN: f64 = 20.0;
    /// Their level 8: the last stand's length and what it multiplies.
    pub const STAND_SECONDS: f64 = 6.0;
    pub const STAND_MULT: i32 = 3;
    /// Their level 9: the spirits (how many, and seconds between each one's blows) and their reach.
    pub const SPIRIT_REACH: f64 = 2.6;
}

/// How many spirits fight beside `hero` once he reaches level 9, and seconds between each one's blows.
pub fn spirits(hero: HeroKind) -> (usize, f64) {
    match hero {
        HeroKind::Ragnar => (0, 0.0),
        HeroKind::Mehmed => (3, 3.0),
        HeroKind::Miyamoto => (1, 1.2),
    }
}

/// Stamina: what he has, what a blow costs, how fast it comes back once he rests a moment.
pub const STAMINA_MAX: f64 = 100.0;
pub const STAMINA_PER_BLOW: f64 = 16.0;
pub const STAMINA_REGEN: f64 = 30.0;
pub const STAMINA_REST: f64 = 0.45;

// ------------------------------------------------------------ progress

/// One hero's unlocks: what is his in each tree, which sealed tiers he has found
/// relics for, and the weapon and shield he takes into the dungeon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeroProgress {
    /// Highest tier owned per tree (0 = none); tiers are owned in order.
    pub owned: [u8; 4],
    /// Bit `t` set: the relic for tier `t` of that tree has been found.
    pub relics: [u16; 4],
    pub wield: u8,
    pub carry: u8,
    /// The archangel he has sworn to at the old man's, whose power comes back to him on a timer.
    pub patron: Option<Archangel>,
    /// The outfit he wears (0 is his own; see `render::heroes::outfit_names`).
    pub outfit: u8,
    /// Bit `t` set: his shield of tier `t` broke, and must be reforged before he carries it again.
    pub broken: u16,
    /// How sharp each blade of his armory is, 0 (rusted through) to 100 (keen): index `tier - 1`.
    pub edge: [u8; 10],
}

impl Default for HeroProgress {
    fn default() -> Self {
        Self { owned: [1, 0, 1, 1], relics: [0; 4], wield: 1, carry: 1, patron: None, outfit: 0, broken: 0, edge: [100; 10] }
    }
}

impl HeroProgress {
    /// Is his shield of `tier` broken (to be reforged before it is carried again)?
    pub fn is_broken(&self, tier: u8) -> bool {
        self.broken & (1 << tier) != 0
    }

    /// How sharp his blade of `tier` is, 0 to 1.
    pub fn sharpness(&self, tier: u8) -> f64 {
        self.edge[(tier.clamp(1, TIERS) - 1) as usize] as f64 / 100.0
    }

    pub fn owned(&self, tree: Tree) -> u8 {
        self.owned[tree.index()]
    }
    pub fn has(&self, tree: Tree, tier: u8) -> bool {
        tier >= 1 && self.owned(tree) >= tier
    }
    pub fn found(&self, tree: Tree, tier: u8) -> bool {
        self.relics[tree.index()] & (1 << tier) != 0
    }
    /// The next tier he could take in a tree, if any is left.
    pub fn next(&self, tree: Tree) -> Option<u8> {
        let n = self.owned(tree) + 1;
        (n <= TIERS).then_some(n)
    }
    /// The path level he has reached.
    pub fn path(&self) -> u8 {
        self.owned(Tree::Path)
    }

    /// Takes a tier, and every found tier right after it (a relic waits until its turn).
    fn take(&mut self, tree: Tree) {
        let i = tree.index();
        self.owned[i] = (self.owned[i] + 1).min(TIERS);
        while self.owned[i] < TIERS && self.found(tree, self.owned[i] + 1) {
            self.owned[i] += 1;
        }
    }
}

/// Everything kept between adventures: the purse, the runes, and each hero's unlocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Progress {
    pub silver: u32,
    pub runes: u32,
    pub heroes: [HeroProgress; 3],
}

/// Why the old man will not hand something over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// It is already his, or the tree is complete.
    Owned,
    /// The tier below it is not his yet.
    Locked,
    Silver(u32),
    Runes(u32),
}

fn slot(hero: HeroKind) -> usize {
    match hero {
        HeroKind::Ragnar => 0,
        HeroKind::Mehmed => 1,
        HeroKind::Miyamoto => 2,
    }
}

impl Progress {
    pub fn hero(&self, hero: HeroKind) -> &HeroProgress {
        &self.heroes[slot(hero)]
    }
    pub fn hero_mut(&mut self, hero: HeroKind) -> &mut HeroProgress {
        &mut self.heroes[slot(hero)]
    }

    /// Can `hero` take `tier` of `tree` now? A found relic makes a sealed tier free.
    pub fn can_unlock(&self, hero: HeroKind, tree: Tree, tier: u8) -> Result<Price, Refusal> {
        let h = self.hero(hero);
        if h.has(tree, tier) {
            return Err(Refusal::Owned);
        }
        if h.next(tree) != Some(tier) {
            return Err(Refusal::Locked);
        }
        let p = if h.found(tree, tier) { Price { silver: 0, runes: 0 } } else { price(tree, tier) };
        if self.runes < p.runes {
            return Err(Refusal::Runes(p.runes - self.runes));
        }
        if self.silver < p.silver {
            return Err(Refusal::Silver(p.silver - self.silver));
        }
        Ok(p)
    }

    /// Pays for and takes the next tier of a tree. Returns the highest tier now owned.
    pub fn unlock(&mut self, hero: HeroKind, tree: Tree, tier: u8) -> Result<u8, Refusal> {
        let p = self.can_unlock(hero, tree, tier)?;
        self.silver -= p.silver;
        self.runes -= p.runes;
        let h = self.hero_mut(hero);
        h.take(tree);
        // A new weapon or shield goes straight into his hand.
        match tree {
            Tree::Armory => h.wield = tier,
            Tree::Shield => h.carry = tier,
            _ => {}
        }
        Ok(h.owned(tree))
    }

    /**
     * A relic found in the world: it unseals the lowest sealed tier of `tree`
     * not yet found. If that tier is next in line it is his at once; otherwise
     * it waits until the tiers before it are his. Returns the tier it unseals,
     * or None if the tree has nothing sealed left.
     */
    pub fn find_relic(&mut self, hero: HeroKind, tree: Tree) -> Option<u8> {
        let h = self.hero_mut(hero);
        let tier = (SEALED_FROM..=TIERS).find(|&t| !h.has(tree, t) && !h.found(tree, t))?;
        h.relics[tree.index()] |= 1 << tier;
        if h.next(tree) == Some(tier) {
            h.take(tree);
        }
        Some(tier)
    }

    /// The tree a found relic belongs to: the one with the most sealed tiers still unfound.
    pub fn relic_tree(&self, hero: HeroKind) -> Option<Tree> {
        let h = self.hero(hero);
        let left = |tree: Tree| (SEALED_FROM..=TIERS).filter(|&t| !h.has(tree, t) && !h.found(tree, t)).count();
        let mut best = None;
        for tree in TREES {
            let n = left(tree);
            if n > 0 && best.map(|(_, m)| n > m).unwrap_or(true) {
                best = Some((tree, n));
            }
        }
        best.map(|(t, _)| t)
    }

    // A line of text for the save file:
    // "v2 silver runes | a p s e r0 r1 r2 r3 wield carry patron outfit broken edge1 .. edge10 | ...".
    // (A "v1" line, from before the loadout, stops after carry and still loads.)
    pub fn encode(&self) -> String {
        let mut out = format!("v2 {} {}", self.silver, self.runes);
        for h in &self.heroes {
            out.push_str(&format!(
                " | {} {} {} {} {} {} {} {} {} {}",
                h.owned[0], h.owned[1], h.owned[2], h.owned[3], h.relics[0], h.relics[1], h.relics[2], h.relics[3], h.wield, h.carry
            ));
            let patron = h.patron.map(|a| a.index() + 1).unwrap_or(0);
            out.push_str(&format!(" {} {} {}", patron, h.outfit, h.broken));
            for e in h.edge {
                out.push_str(&format!(" {e}"));
            }
        }
        out
    }

    pub fn decode(text: &str) -> Option<Progress> {
        let mut parts = text.split('|');
        let mut head = parts.next()?.split_whitespace();
        let v2 = match head.next()? {
            "v1" => false,
            "v2" => true,
            _ => return None,
        };
        let mut p = Progress { silver: head.next()?.parse().ok()?, runes: head.next()?.parse().ok()?, ..Default::default() };
        for h in p.heroes.iter_mut() {
            let n: Vec<u32> = parts.next()?.split_whitespace().map(|v| v.parse().ok()).collect::<Option<_>>()?;
            if n.len() != if v2 { 23 } else { 10 } {
                return None;
            }
            if v2 {
                h.patron = (1..=4).contains(&n[10]).then(|| crate::logic::angels::ARCHANGELS[n[10] as usize - 1]);
                h.outfit = (n[11] as u8).min(3);
                h.broken = n[12] as u16;
                for k in 0..10 {
                    h.edge[k] = (n[13 + k] as u8).min(100);
                }
            }
            for i in 0..4 {
                h.owned[i] = (n[i] as u8).min(TIERS);
                h.relics[i] = n[4 + i] as u16;
            }
            h.owned[0] = h.owned[0].max(1);
            h.owned[2] = h.owned[2].max(1);
            h.owned[3] = h.owned[3].max(1);
            h.wield = (n[8] as u8).clamp(1, h.owned[0]);
            h.carry = (n[9] as u8).clamp(1, h.owned[2]);
        }
        Some(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_loadout_is_saved_and_a_save_from_before_it_still_loads() {
        let mut p = Progress { silver: 77, runes: 3, ..Default::default() };
        let h = p.hero_mut(HeroKind::Mehmed);
        h.patron = Some(Archangel::Raphael);
        h.outfit = 2;
        h.broken = 1 << 3;
        h.edge[0] = 41;
        let back = Progress::decode(&p.encode()).expect("it loads");
        assert_eq!(back, p);
        assert!(back.hero(HeroKind::Mehmed).is_broken(3) && !back.hero(HeroKind::Mehmed).is_broken(2));
        assert!((back.hero(HeroKind::Mehmed).sharpness(1) - 0.41).abs() < 1e-9);
        // A v1 save: no patron, his own outfit, nothing broken, every blade keen.
        let old = Progress::decode("v1 5 1 | 2 0 1 1 0 0 0 0 2 1 | 1 0 1 1 0 0 0 0 1 1 | 1 0 1 1 0 0 0 0 1 1").expect("a v1 save");
        assert_eq!(old.silver, 5);
        assert_eq!(old.hero(HeroKind::Ragnar).wield, 2);
        assert_eq!((old.hero(HeroKind::Ragnar).patron, old.hero(HeroKind::Ragnar).edge), (None, [100; 10]));
    }
    use crate::logic::heroes::ROSTER;

    #[test]
    fn every_hero_has_four_full_trees_with_names_of_his_own() {
        for hero in ROSTER {
            for tree in TREES {
                for tier in 1..=TIERS {
                    let n = node(hero, tree, tier);
                    assert!(!n.name.is_empty() && !n.lore.is_empty() && !n.ability.is_empty() && !n.effect.is_empty());
                }
            }
        }
        // Tier 7 of the armory is a different legend for each of them, but the same power.
        let names: Vec<_> = ROSTER.iter().map(|&h| node(h, Tree::Armory, 7).name).collect();
        assert!(names[0] != names[1] && names[1] != names[2] && names[0] != names[2]);
        assert_eq!(node(HeroKind::Ragnar, Tree::Armory, 10).name, "Mjolnir, Thor's Hammer");
        assert_eq!(node(HeroKind::Ragnar, Tree::Path, 5).ability, "Bifrost Step");
        assert_eq!(node(HeroKind::Ragnar, Tree::Elixir, 8).ability, "Baldr's Blessing");
    }

    #[test]
    fn weapons_grow_stronger_and_shields_sturdier_tier_by_tier() {
        let blade = |t| blade(HeroKind::Ragnar, t);
        for t in 1..TIERS {
            assert!(blade(t + 1).damage >= blade(t).damage);
            let (a, b) = (ward(t), ward(t + 1));
            assert!(b.block > a.block && b.durability > a.durability);
            assert!(price(Tree::Armory, t + 1).silver > price(Tree::Armory, t).silver);
        }
        assert!(blade(2).throw.unwrap().returns && blade(10).throw.unwrap().returns);
        assert!(blade(4).stun > 0.0 && blade(6).trick && blade(7).pierce_armor && blade(8).bleed > 0.0 && blade(9).crit);
        assert_eq!(blade(5).swing.unwrap().2, blade(5).swing.unwrap().0, "the whirlwind reaches behind as far as ahead");
        assert!(ward(10).bash && ward(10).reflect && ward(10).unbreakable && ward(10).aegis > 0, "each shield keeps the gifts below it");
        assert!(!ward(3).bash);
        // Mehmed's and Miyamoto's armories are their own.
        let (m, y) = (|t| super::blade(HeroKind::Mehmed, t), |t| super::blade(HeroKind::Miyamoto, t));
        assert!(m(5).throw.unwrap().stagger && m(7).fire_trail && m(8).arrows > 0 && m(10).flash > 0.0);
        assert!(y(3).lunge > 0.0 && y(4).burn > 0.0 && y(6).poison && y(7).blood > 0 && y(8).arrows > 0 && y(10).flash > 0.0);
        assert_eq!(node(HeroKind::Miyamoto, Tree::Armory, 7).ability, "Blood Drinker");
        assert_eq!(node(HeroKind::Mehmed, Tree::Path, 8).ability, "Blood of the Empire");
    }

    #[test]
    fn the_low_tiers_cost_silver_and_the_sealed_ones_runes_too() {
        let hero = HeroKind::Ragnar;
        let mut p = Progress::default();
        assert_eq!(p.can_unlock(hero, Tree::Armory, 1), Err(Refusal::Owned));
        assert_eq!(p.can_unlock(hero, Tree::Armory, 3), Err(Refusal::Locked), "tiers come in order");
        assert_eq!(p.can_unlock(hero, Tree::Armory, 2), Err(Refusal::Silver(150)));
        p.silver = 10_000;
        for t in 2..=5 {
            assert_eq!(p.unlock(hero, Tree::Armory, t), Ok(t));
        }
        assert_eq!(p.hero(hero).wield, 5, "a new weapon goes into his hand");
        assert_eq!(p.can_unlock(hero, Tree::Armory, 6), Err(Refusal::Runes(2)));
        p.runes = 2;
        assert_eq!(p.unlock(hero, Tree::Armory, 6), Ok(6));
        assert_eq!(p.runes, 0);
        // Another hero's trees are his own.
        assert_eq!(p.hero(HeroKind::Miyamoto).owned(Tree::Armory), 1);
    }

    #[test]
    fn a_relic_unseals_a_tier_at_once_or_waits_its_turn() {
        let hero = HeroKind::Ragnar;
        let mut p = Progress::default();
        // Found early, the relic for shield tier 6 waits for tiers 2 to 5.
        assert_eq!(p.find_relic(hero, Tree::Shield), Some(6));
        assert_eq!(p.hero(hero).owned(Tree::Shield), 1);
        p.silver = 5000;
        for t in 2..=5 {
            p.unlock(hero, Tree::Shield, t).unwrap();
        }
        assert_eq!(p.hero(hero).owned(Tree::Shield), 6, "the found tier came with tier 5");
        // Next in line, a relic is his on the spot, runes or not.
        assert_eq!(p.find_relic(hero, Tree::Shield), Some(7));
        assert_eq!(p.hero(hero).owned(Tree::Shield), 7);
        assert_eq!(p.runes, 0);
        // Relics go to the tree with the most still sealed.
        assert_ne!(p.relic_tree(hero), Some(Tree::Shield));
    }

    #[test]
    fn progress_survives_the_save_file() {
        let mut p = Progress { silver: 1234, runes: 7, ..Default::default() };
        p.silver += 5000;
        p.unlock(HeroKind::Mehmed, Tree::Path, 1).unwrap();
        p.find_relic(HeroKind::Ragnar, Tree::Elixir);
        let back = Progress::decode(&p.encode()).expect("decodes");
        assert_eq!(back, p);
        assert_eq!(Progress::decode("nonsense"), None);
    }
}
