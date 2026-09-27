//! Each hero's four trees of progression: an armory of ten legendary
//! weapons, a heroic path of ten skills, ten shields and ten elixirs. The
//! names and legends are each hero's own (Norse for Ragnar, Ottoman and
//! Turkic for Mehmed, Japanese for Miyamoto); what a tier does in play is the
//! same for all three, so tier 4 of any armory stuns and tier 10 of any path
//! heals in full.
//!
//! Tiers 1 to 5 are sold by the old gatekeeper for silver. Tiers 6 to 10 are
//! sealed: a relic found in a hidden shrine or taken from a fallen Cyclops
//! unseals the next one, or the old man trades it for silver and rune
//! fragments. What is unlocked, the silver and the runes are kept between
//! adventures.

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
            (HeroKind::Mehmed, Tree::Armory) => "Forge of the Ocak",
            (HeroKind::Mehmed, Tree::Path) => "Path of the Janissary",
            (HeroKind::Mehmed, Tree::Shield) => "Kalkans of the Hearth",
            (HeroKind::Mehmed, Tree::Elixir) => "Sherbets of the Saray",
            (HeroKind::Miyamoto, Tree::Armory) => "Blades of the Sacred Forge",
            (HeroKind::Miyamoto, Tree::Path) => "Way of the Two Heavens",
            (HeroKind::Miyamoto, Tree::Shield) => "Guards of the Castle",
            (HeroKind::Miyamoto, Tree::Elixir) => "Brews of the Mountain Shrine",
        }
    }

    /// What a level of the tree is called ("TIER 4", "LEVEL 4").
    pub fn step(self) -> &'static str {
        if self == Tree::Path { "LEVEL" } else { "TIER" }
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
    Entry("Mistilteinn Magic Blade", "The shining branch that once felled Baldr; radiates pure fatal light.", "Baleful Glow", 0xc8ff9a, Sword),
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
    Entry("Recruit's Yatagan", "The short forward-curved blade handed to every new boy of the Ocak.", "Basic Strike", 0xd8c8a8, Knife),
    Entry("Balkan Teber", "A light frontier battle-axe, weighted to fly out and come home.", "Returning Arc", 0xa8b8c8, Axe),
    Entry("Damascus Kilij", "Watered steel folded a hundred times; its edge sings as it cuts.", "Crescent Cut", 0x9ad1ff, Saber),
    Entry("Bozdogan Mace", "The grey-falcon mace of the pashas: one blow and the world spins.", "Pasha's Verdict", 0xc8c8d8, Mace),
    Entry("Twin Karabela Sabers", "Two sabers turning like the whirling dervishes of Konya.", "Dervish Whirl", 0xff9a4a, TwinBlades),
    Entry("Shahmaran's Fang", "A serpent blade from the Queen of Snakes, dripping with dreams.", "Serpent's Mirage", 0x6ae8a0, Saber),
    Entry("Sword of Tepegoz", "The one-eyed giant's own blade, the only steel his hide could not turn.", "Basat's Justice", 0xc8ff9a, Sword),
    Entry("Ejder Pala", "A broad scimitar tempered in the blood of the Anatolian dragon.", "Dragon's Blood", 0xe0303a, Saber),
    Entry("Golden Bow of Oghuz Khan", "The golden bow of the first Khan; the grey wolf guides its arrows.", "Grey Wolf's Arrow", 0xffd84a, Bow),
    Entry("Tengri's Sky Mace", "Forged by the Eternal Blue Sky; thunder answers when it is thrown.", "Wrath of the Blue Sky", 0x7ae8ff, Mace),
];

const MEHMED_PATH: [Entry; 10] = [
    Entry("Akinci Charge", "The headlong fury of the frontier raiders.", "Raider's Fury", 0xff5a3a, Rune(0)),
    Entry("Hearth Discipline", "The iron order of the Janissary hearth, the Ocak.", "Wall of the Ocak", 0x8fb4ff, Rune(1)),
    Entry("Huma's Shadow", "The bird of fortune casts its shadow, and what was hidden is shown.", "Eye of the Huma", 0xc89aff, Rune(2)),
    Entry("Mehter March", "The war band's drums and zurnas, that never let an army tire.", "Mehter's Drum", 0xffd27a, Rune(3)),
    Entry("Tulpar Stride", "The leap of the winged horse of the steppe.", "Tulpar's Leap", 0x9af0ff, Rune(4)),
    Entry("Asena's Ambush", "The cunning of the grey she-wolf who led the Turks out of the valley.", "Asena's Fang", 0xb8c0cc, Rune(5)),
    Entry("Taurus Endurance", "The iron lungs of the shepherds of the Taurus peaks.", "Unbroken March", 0xff8a2a, Rune(6)),
    Entry("The Crimson Banner", "The raised crimson standard that sent whole armies into rout.", "Rout", 0xc8202a, Rune(7)),
    Entry("Hizir's Hand", "The green-cloaked wanderer who comes in the darkest hour.", "Hizir's Return", 0x6ae86a, Rune(8)),
    Entry("Tree of Life", "Strength drawn from Bay-Terek, the tree at the centre of the world.", "Bay-Terek's Blessing", 0xa6f04a, Rune(9)),
];

const MEHMED_SHIELD: [Entry; 10] = [
    Entry("Wicker Kalkan", "Fig-wood cane wound in a spiral with silk thread.", "Brace", 0xc8a060, Spiral),
    Entry("Iron-Boss Kalkan", "A spiral kalkan with a hammered iron boss.", "Iron Heart", 0xb8c0cc, Spiral),
    Entry("Silk-Wound Kalkan", "Its threads draw tight again after every blow.", "Self-Mend", 0xd8483a, Spiral),
    Entry("Janissary Steel Kalkan", "Pressed steel with a sharp rim for pushing back.", "Rim Strike", 0xa8b0bc, Round),
    Entry("Crescent Buckler", "A steel crescent that catches arrows in its horns.", "Arrow Ward", 0xd8d8e0, Round),
    Entry("Mirror of Iskender", "Alexander's polished mirror, turning every strike back on its sender.", "Iskender's Mirror", 0x9ad1ff, Mirror),
    Entry("Seljuk Star Shield", "Its eight-pointed star rings like a bell when struck.", "Ringing Star", 0x3a8ad8, Round),
    Entry("Rumeli Bastion", "Plated like the walls of the Rumelian fortress; it will not fall.", "Unbreachable", 0x8fa8c8, Kite),
    Entry("Ejderha Scale Shield", "Dragon scales that burn whatever touches them.", "Dragon Scales", 0x4a8a4a, Kite),
    Entry("Seal of Suleyman", "The wise king's ring-seal set in gold; blows that strike it heal the bearer.", "Seal of Wisdom", 0xffd84a, Round),
];

const MEHMED_ELIXIR: [Entry; 10] = [
    Entry("Rose Sherbet", "Rosewater and sugar, cold from the palace snow-wells.", "Instant Restoration", 0xff8ab0, Flask),
    Entry("Barracks Salep", "Hot orchid-root milk from the Janissary cauldron.", "Warm Breath", 0xf2e3c4, Gourd),
    Entry("Mesir Paste", "The palace paste of forty-one spices, said to give a soldier strength.", "Forty-One Spices", 0xa0502a, Gourd),
    Entry("Janissary Coffee", "Black, thick and bitter, boiled three times over the coals.", "Coffee Rush", 0x5a3a24, Vial),
    Entry("Healing Bowl Draught", "Water drunk from a sifa tasi, a bowl engraved with charms.", "Warded Skin", 0x9aa4b0, Flask),
    Entry("Deli's Draught", "The drink of the Deli horsemen, whose name means mad.", "Madman's Courage", 0xff4a8a, Gourd),
    Entry("Naphtha Tincture", "Distilled from the burning springs of the Caspian shore.", "Burning Skin", 0xff7a2a, Vial),
    Entry("Water of Life", "Ab-i Hayat, the spring at the edge of darkness that makes the drinker deathless.", "Deathless Moment", 0xa6e0ff, Flask),
    Entry("Kaf Mountain Tonic", "Brewed from the snows of Mount Kaf, that rings the edge of the world.", "World's Edge", 0x9a2aff, Vial),
    Entry("Anka Feather Elixir", "A single feather of the Anka, steeped until it glows.", "Anka's Embrace", 0xffb84a, Flask),
];

// ------------------------------------------------------------ Miyamoto

const MIYAMOTO_ARMORY: [Entry; 10] = [
    Entry("Oar Bokken", "The wooden sword Musashi carved from a boat's oar on his way to a duel.", "Basic Strike", 0xc8a070, Katana),
    Entry("Iron Kusarigama", "A sickle on a weighted chain: it flies out and snaps back.", "Chain Arc", 0xa8b8c8, Sickle),
    Entry("Monohoshizao", "Kojiro's drying pole, a blade so long it cuts beyond its reach.", "Swallow Cut", 0x9ad1ff, Katana),
    Entry("Munisai's Jitte", "His father's iron truncheon, that hooks a blade and stuns its bearer.", "Disarming Hook", 0xc8c8d8, Knife),
    Entry("Daisho of Two Heavens", "Long sword and short sword together, the style of Niten Ichi-ryu.", "Two Heavens as One", 0xff9a4a, TwinBlades),
    Entry("Kitsune-bi Wakizashi", "A blade lit with fox-fire, the lantern of the trickster foxes.", "Fox's Illusion", 0x7ac8ff, Katana),
    Entry("Muramasa", "A cursed katana that hungers to cut, and stops for no armour.", "Cursed Edge", 0xc83a5a, Katana),
    Entry("Kusanagi", "The grass-cutting sword drawn from the tail of the eight-headed serpent.", "Orochi's Wake", 0x6ae8a0, Sword),
    Entry("Ame-no-Nuboko", "The jewelled spear of heaven that stirred the sea to make the islands.", "Heaven's Aim", 0xffd84a, Spear),
    Entry("Raijin's Thunder Mallet", "The drum-beating mallet of the thunder god.", "Heaven's Drum", 0x7ae8ff, Hammer),
];

const MIYAMOTO_PATH: [Entry; 10] = [
    Entry("Kiai", "The spirit shout that turns breath into a blow.", "Spirit Shout", 0xff5a3a, Rune(0)),
    Entry("Fudoshin", "The immovable heart: no fear behind the guard.", "Immovable Heart", 0x8fb4ff, Rune(1)),
    Entry("Kan no Me", "The perceiving eye of the Book of Five Rings, that sees what is not shown.", "Perceiving Eye", 0xc89aff, Rune(2)),
    Entry("Ibuki Breath", "The deep breathing of the mountain ascetics.", "Mountain Breath", 0xffd27a, Rune(3)),
    Entry("Shukuchi", "The legendary step that shrinks the earth between two points.", "Shrinking Step", 0x9af0ff, Rune(4)),
    Entry("Shinobi's Shadow", "The patience of the unseen blade, that strikes where the eye does not watch.", "Shadow Fang", 0xb8c0cc, Rune(5)),
    Entry("Oni's Endurance", "A body toughened like a mountain demon's.", "Iron Body", 0xff8a2a, Rune(6)),
    Entry("Hannya's Dread", "The horned mask of a jealous spirit; the weak flee from it.", "Hannya's Glare", 0xc8202a, Rune(7)),
    Entry("Return from Yomi", "As Izanagi climbed back out of the land of the dead.", "Izanagi's Return", 0xfff1a8, Rune(8)),
    Entry("Shinboku's Grace", "The sacred tree of the shrine, where the kami come down to earth.", "Sacred Tree's Grace", 0x6ae86a, Rune(9)),
];

const MIYAMOTO_SHIELD: [Entry; 10] = [
    Entry("Bamboo Tate", "A standing shield of lashed bamboo.", "Brace", 0xc8b870, Tate),
    Entry("Lacquered Sode", "A great shoulder guard of lacquered iron scales.", "Iron Heart", 0xb8222a, Tate),
    Entry("Rope-Bound Tate", "Hemp that tightens as it dries, mending its gaps.", "Self-Mend", 0xa08a5a, Tate),
    Entry("Tetsu Kote Guard", "Iron armguards that strike as they block.", "Rim Strike", 0x8a8a94, Round),
    Entry("Temple Gate Shield", "Cut from the heavy doors of a mountain temple.", "Arrow Ward", 0x8a3a2a, Tate),
    Entry("Yata no Kagami", "The sun goddess's sacred mirror: what strikes it sees itself, and turns back.", "Mirror of Truth", 0x9ad1ff, Mirror),
    Entry("Oni-Face Shield", "A demon's snarl beaten into iron; foes reel when it rings.", "Demon's Snarl", 0xd84a3a, Round),
    Entry("Himeji Wall", "Plastered like the White Heron castle, that never fell.", "Unfallen", 0xeeeae0, Tate),
    Entry("Dragon-Scale Tate", "Scales shed by a river dragon, cold and cutting.", "Dragon Scales", 0x4a8a8a, Tate),
    Entry("Shield of Hachiman", "Blessed by the god of war, protector of warriors: its blocks renew the bearer.", "Hachiman's Blessing", 0xffd84a, Round),
];

const MIYAMOTO_ELIXIR: [Entry; 10] = [
    Entry("Matcha Tonic", "Whisked green tea from a temple garden.", "Instant Restoration", 0x7ac84a, Flask),
    Entry("Umeboshi Broth", "Sour pickled plum in hot broth, the soldier's quick restorative.", "Sour Breath", 0xe05a6a, Gourd),
    Entry("Oni's Sake", "Rice wine brewed strong enough for a mountain demon.", "Demon's Might", 0xa0302a, Gourd),
    Entry("Tengu Wind Draught", "Mountain wind bottled by the long-nosed tengu.", "Tengu's Wind", 0xffd23a, Vial),
    Entry("Lacquer Tincture", "It hardens the skin like a coat of lacquered armour.", "Lacquer Skin", 0x9aa4b0, Flask),
    Entry("Fugu Distillate", "The poison of the puffer fish, taken in a deadly measure.", "Death's Edge", 0xff4a8a, Vial),
    Entry("Kagutsuchi's Ember", "A spark of the fire kami, whose birth burned the world.", "Burning Spirit", 0xff7a2a, Vial),
    Entry("Horai Elixir", "The elixir of immortality from Mount Horai, sent by the moon princess.", "Deathless Moment", 0xffe066, Flask),
    Entry("Susanoo's Storm Sake", "The storm god's brew, drunk before he slew the eight-headed serpent.", "Storm Unbound", 0x9a2aff, Gourd),
    Entry("Kodama Dew", "Dew gathered from the trees where the forest spirits live.", "Kodama's Embrace", 0xa6f0c8, Flask),
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

/// What tier `tier` (1..=10) of a tree does in play, the same for every hero.
pub fn effect(tree: Tree, tier: u8) -> &'static str {
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
        "F or LB: a flash of light carries you three tiles ahead.",
        "Blows from behind strike the Cyclops for triple damage.",
        "When your stamina runs out: six seconds of free swings, and nothing slows you.",
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
    let i = (tier.clamp(1, TIERS) - 1) as usize;
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
    Node { name: e.0, lore: e.1, ability: e.2, effect: effect(tree, tier), color: e.3, form: e.4 }
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
    /// The swing: tiles of reach ahead, half-width to either side, reach behind. None for thrown weapons.
    pub swing: Option<(f64, f64, f64)>,
    /// Thrown: flies out and back (returns), or on until it hits (homing spear).
    pub throw: Option<Throw>,
    pub cooldown: f64,
    /// Each swing also sends a piercing wave down the line.
    pub wave: bool,
    /// Seconds foes around the blow are stunned, and how far around.
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
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Throw {
    pub speed: f64,
    pub range: f64,
    pub radius: f64,
    pub returns: bool,
    pub homing: bool,
}

pub const STUN_RADIUS: f64 = 2.6;
pub const TRICK_RADIUS: f64 = 3.5;

pub fn blade(tier: u8) -> Blade {
    let base = Blade {
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
    };
    match tier.clamp(1, TIERS) {
        1 => base,
        2 => Blade { damage: 2, swing: None, throw: Some(Throw { speed: 11.0, range: 6.0, radius: 0.7, returns: true, homing: false }), cooldown: 0.45, ..base },
        3 => Blade { damage: 3, swing: Some((1.4, 0.8, 0.25)), cooldown: 0.5, wave: true, ..base },
        4 => Blade { damage: 3, swing: Some((1.5, 0.95, 0.3)), cooldown: 0.48, stun: 2.5, ..base },
        5 => Blade { damage: 3, swing: Some((1.6, 1.6, 1.6)), cooldown: 0.46, ..base },
        6 => Blade { damage: 4, swing: Some((1.6, 1.0, 0.3)), cooldown: 0.45, trick: true, ..base },
        7 => Blade { damage: 5, swing: Some((1.7, 1.1, 0.35)), cooldown: 0.45, pierce_armor: true, ..base },
        8 => Blade { damage: 5, swing: Some((1.8, 1.3, 0.5)), cooldown: 0.5, bleed: 5.0, ..base },
        9 => Blade { damage: 6, swing: None, throw: Some(Throw { speed: 15.0, range: 14.0, radius: 0.65, returns: false, homing: true }), cooldown: 0.55, crit: true, ..base },
        _ => Blade { damage: 7, swing: None, throw: Some(Throw { speed: 17.0, range: 7.0, radius: 0.8, returns: true, homing: false }), cooldown: 0.5, chain: 5, ..base },
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
}

pub fn ward(tier: u8) -> Ward {
    let t = tier.clamp(1, TIERS);
    const BLOCK: [f64; 10] = [0.4, 0.48, 0.52, 0.56, 0.6, 0.65, 0.7, 0.75, 0.8, 0.85];
    const DURABILITY: [i32; 10] = [40, 70, 90, 120, 150, 180, 220, 260, 300, 360];
    const MEND: [f64; 10] = [0.0, 0.0, 1.0, 1.0, 1.0, 1.5, 1.5, 2.0, 2.0, 3.0];
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
}

impl Default for HeroProgress {
    fn default() -> Self {
        Self { owned: [1, 0, 1, 1], relics: [0; 4], wield: 1, carry: 1 }
    }
}

impl HeroProgress {
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

    // A line of text for the save file: "silver runes | a p s e r0 r1 r2 r3 w c | ...".
    pub fn encode(&self) -> String {
        let mut out = format!("v1 {} {}", self.silver, self.runes);
        for h in &self.heroes {
            out.push_str(&format!(
                " | {} {} {} {} {} {} {} {} {} {}",
                h.owned[0], h.owned[1], h.owned[2], h.owned[3], h.relics[0], h.relics[1], h.relics[2], h.relics[3], h.wield, h.carry
            ));
        }
        out
    }

    pub fn decode(text: &str) -> Option<Progress> {
        let mut parts = text.split('|');
        let mut head = parts.next()?.split_whitespace();
        if head.next()? != "v1" {
            return None;
        }
        let mut p = Progress { silver: head.next()?.parse().ok()?, runes: head.next()?.parse().ok()?, ..Default::default() };
        for h in p.heroes.iter_mut() {
            let n: Vec<u32> = parts.next()?.split_whitespace().map(|v| v.parse().ok()).collect::<Option<_>>()?;
            if n.len() != 10 {
                return None;
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
