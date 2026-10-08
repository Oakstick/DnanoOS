//! The bestiary (the old man's book of every foe met): what is in it, grouped as the game groups
//! its foes, each entry's words (its name, where it comes from, how it fights as the game has it,
//! and a line from the old man), and what the player has done with each (met, felled, felled by),
//! as it is saved (`vh.bestiary`).
//!
//! Pure data and rules, free of the game: `game::bestiary` watches the fight and fills the book,
//! `render::hud::bestiary` draws it. Every claim of how a foe fights is the code's (COMBAT.md §11,
//! PRINCES.md, FINALE.md, `combat::archetype`); where the game takes a liberty with the legend the
//! words say "here". There is no Devil in it, and nothing of the end.

use std::collections::HashMap;

use super::combat::archetype::{Archetype, Attack};
use super::combat::Breed;
use super::enemy::EnemyKind;
use super::sins::{Sin, SINS};
use super::war::general::General;

/// Where the save keeps the book.
pub const BOOK_KEY: &str = "vh.bestiary";
/// The save's version: written as "b1"; any "bN" is read as far as it can be.
const VERSION: &str = "b1";
/// The most rivals the book remembers from runs gone by (the newest kept).
pub const REMEMBERED_RIVALS: usize = 30;

/// One page of the book (a rival's is kept apart: `RivalNote`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Entry {
    /// A foe of this kind.
    Foe(EnemyKind),
    /// The Cyclops of the meadows (`logic::boss`).
    Cyclops,
    /// The mimic chest, a prince's cheat (`game::deceit`).
    Mimic,
    /// The general of this sin's realm (`war::general`).
    General(Sin),
}

/// A group of pages, as the game groups its foes: the seven stages' casts, the Cyclops, the
/// realms' own, the generals, the princes' courts, the bottom of the Pit, and the rivals.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Group {
    Stage(u8),
    Cyclops,
    Realms,
    Generals,
    Courts,
    Pit,
    Rivals,
}

/// The groups in the book's order.
pub const GROUPS: [Group; 13] = [
    Group::Stage(1),
    Group::Stage(2),
    Group::Stage(3),
    Group::Stage(4),
    Group::Stage(5),
    Group::Stage(6),
    Group::Stage(7),
    Group::Cyclops,
    Group::Realms,
    Group::Generals,
    Group::Courts,
    Group::Pit,
    Group::Rivals,
];

impl Group {
    /// Its heading.
    pub fn title(self) -> &'static str {
        match self {
            Group::Stage(1) => "THE RESTLESS DEAD",
            Group::Stage(2) => "THE WITCH'S BROOD",
            Group::Stage(3) => "THE MOUNTAIN'S DEMONS",
            Group::Stage(4) => "THE YŌKAI",
            Group::Stage(5) => "ANATOLIAN NIGHTS",
            Group::Stage(6) => "THE LEGENDS",
            Group::Stage(_) => "RAGNARÖK",
            Group::Cyclops => "THE CYCLOPS' MEADOWS",
            Group::Realms => "THE REALMS' OWN",
            Group::Generals => "THE PRINCES' HANDS",
            Group::Courts => "THE PRINCES' COURTS",
            Group::Pit => "THE BOTTOM OF THE PIT",
            Group::Rivals => "THE LEDGER'S NAMES",
        }
    }

    /// Its tab's short name.
    pub fn short(self) -> &'static str {
        match self {
            Group::Stage(1) => "I",
            Group::Stage(2) => "II",
            Group::Stage(3) => "III",
            Group::Stage(4) => "IV",
            Group::Stage(5) => "V",
            Group::Stage(6) => "VI",
            Group::Stage(_) => "VII",
            Group::Cyclops => "CYCLOPS",
            Group::Realms => "REALMS",
            Group::Generals => "GENERALS",
            Group::Courts => "PRINCES",
            Group::Pit => "THE PIT",
            Group::Rivals => "RIVALS",
        }
    }

    /// A line under its heading.
    pub fn blurb(self) -> &'static str {
        match self {
            Group::Stage(1) => "The first stage: the garden's dead and the birds of the dusk.",
            Group::Stage(2) => "The second stage: old Europe's night, a witch, a wolf-man and a headless brute.",
            Group::Stage(3) => "The third stage: swords and fangs from Japan, the north and the Nile.",
            Group::Stage(4) => "The fourth stage: Japan's folk demons, each with a power from its tale.",
            Group::Stage(5) => "The fifth stage: Turkish and Turkic spirits of the dark.",
            Group::Stage(6) => "The sixth stage: two giants, a dragon and a ghost.",
            Group::Stage(_) => "The seventh stage: the Norse end of the world.",
            Group::Cyclops => "The one-eyed guardian of the meadows on the way down.",
            Group::Realms => "Born of the sins' realms, and a cheat of the princes.",
            Group::Generals => "Each realm's general, who runs it against you by his orders.",
            Group::Courts => "The seven princes of the sins, and what each one raises.",
            Group::Pit => "What waits at the very bottom.",
            Group::Rivals => "Foes who did a deed against you, and earned a name for it.",
        }
    }

    /// Its fixed pages (the rivals' are the player's own: none here).
    pub fn entries(self) -> Vec<Entry> {
        match self {
            Group::Stage(s) => EnemyKind::of_stage(s).iter().map(|&k| Entry::Foe(k)).collect(),
            Group::Cyclops => vec![Entry::Cyclops],
            Group::Realms => EnemyKind::REALMS_OWN.iter().map(|&k| Entry::Foe(k)).chain([Entry::Mimic]).collect(),
            Group::Generals => SINS.iter().map(|&s| Entry::General(s)).collect(),
            Group::Courts => EnemyKind::COURTS_OWN.iter().map(|&k| Entry::Foe(k)).collect(),
            Group::Pit => EnemyKind::LAIR_OWN.iter().map(|&k| Entry::Foe(k)).collect(),
            Group::Rivals => Vec::new(),
        }
    }
}

/// Every kind of foe in the game, by its group (each once).
pub fn all_kinds() -> Vec<EnemyKind> {
    GROUPS
        .iter()
        .flat_map(|g| g.entries())
        .filter_map(|e| match e {
            Entry::Foe(k) => Some(k),
            _ => None,
        })
        .collect()
}

/// Every fixed page of the book, in its order.
pub fn all_entries() -> Vec<Entry> {
    GROUPS.iter().flat_map(|g| g.entries()).collect()
}

/// The group a page is in.
pub fn group_of(entry: Entry) -> Group {
    GROUPS.iter().copied().find(|g| g.entries().contains(&entry)).unwrap_or(Group::Rivals)
}

// ------------------------------------------------------------------ the words

/// A page's words.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Lore {
    /// Its name as the game gives it.
    pub name: &'static str,
    /// Its name in the tongue of its tale, and what that means ("" when it has none of its own).
    pub native: &'static str,
    /// Where its tale comes from.
    pub origin: &'static str,
    /// The legend, two to four short sentences ("here" where the game takes a liberty).
    pub legend: &'static str,
    /// How it fights, as the game has it.
    pub fights: &'static str,
    /// What the old man says of it.
    pub old_man: &'static str,
}

/// The archetype a page's foe fights as, and how its blows reach the hero (none for the mimic).
pub fn archetype(entry: Entry) -> Option<(Archetype, Attack)> {
    let kind = match entry {
        Entry::Foe(k) => k,
        Entry::General(s) => General::of(s).body,
        Entry::Cyclops | Entry::Mimic => return None,
    };
    let b = Breed::of(kind);
    Some((b.arch, b.attack()))
}

/// The question an archetype asks (COMBAT.md §4.3), as the book shows it.
pub fn question(arch: Archetype) -> &'static str {
    match arch {
        Archetype::Brute => "THE BRUTE: CAN YOU WAIT?",
        Archetype::Skirmisher => "THE SKIRMISHER: CAN YOU READ THE TIMING?",
        Archetype::Hexer => "THE HEXER: CAN YOU CLOSE THE DISTANCE?",
        Archetype::Prince => "A PRINCE: THREE MOODS, AND A HIDE THAT OPENS",
    }
}

/// The words of a page.
pub fn lore(entry: Entry) -> Lore {
    match entry {
        Entry::Foe(k) => foe_lore(k),
        Entry::Cyclops => CYCLOPS,
        Entry::Mimic => MIMIC,
        Entry::General(s) => general_lore(s),
    }
}

const fn l(
    name: &'static str,
    native: &'static str,
    origin: &'static str,
    legend: &'static str,
    fights: &'static str,
    old_man: &'static str,
) -> Lore {
    Lore { name, native, origin, legend, fights, old_man }
}

const CYCLOPS: Lore = l(
    "The Cyclops",
    "Greek: Kyklops, \"round eye\"",
    "Greek myth: Hesiod and Homer",
    "Hesiod sang of three Cyclopes, sons of Earth and Sky, who forged the thunderbolt of Zeus. Homer's Cyclops, Polyphemus, shut Odysseus and his men in his cave, until Odysseus said his name was Nobody and put out the giant's one eye. Here the Cyclops is a floating eye the size of a hut, and it guards the meadows on the way down.",
    "It drifts round its meadow keeping its distance, and squints and glows before it throws: fans and rings of bolts, a stream that follows you, boulders that roll. Wounded, it grows faster and dives at you; a dive that misses leaves it dazed on the grass, and then every blow counts double. Its touch hurts, except while it lies dazed. From the second meadow on its hide turns a point of every blow.",
    "Odysseus beat his Cyclops with his wits, not his arm. Keep the pillars between you, let it miss, and strike when it lies in the grass.",
);

const MIMIC: Lore = l(
    "Mimic Chest",
    "",
    "This place: a cheat of the princes",
    "No old tale tells of it. Here it is one of the princes' cheats in their realms: a chest of loot that is really a mouth, set out for the greedy hand.",
    "It breathes, its coins never glint, and its hinge creaks when you come near. Strike it first, and it dies with a shriek and pays twice what a chest pays. Open it, and it was a mouth: a foe of that depth comes out of it biting.",
    "Gold that does not shine is not gold. Try a chest with your blade before you try it with your hand.",
);

fn foe_lore(kind: EnemyKind) -> Lore {
    use EnemyKind as K;
    match kind {
        // ---- the first stage
        K::Skeleton => l(
            "Draugr",
            "Old Norse: draugr",
            "Norse sagas, Iceland and Scandinavia",
            "In the Icelandic sagas a draugr is one of the dead who will not stay in his grave mound: heavy, strong, and jealous of what was buried with him. In Grettir's saga the draugr Glámr haunts a farm until Grettir wrestles him down by moonlight. Here the draugr are bare bones, and they will not stay down.",
            "In the garden and the dungeons it falls to pieces when struck down, and pulls itself together a few seconds later. It has no eyes: it hunts by the sound of your steps, and goes where it last heard you. Where foes fight blow for blow it is a small Brute that chops, slow and heavy: wait out the blow, then punish.",
            "Do not be afraid of the rattle. Bones that rise again are only bones; it is the heart that counts, and yours still beats.",
        ),
        K::Bat => l(
            "Raven",
            "Old Norse: hrafn",
            "Norse myth",
            "Odin kept two ravens, Huginn and Muninn, Thought and Memory, who flew over the world every day and came back to tell him all they saw. In the old poems the raven is the bird of the battlefield. Here the ravens serve the garden, not Odin.",
            "In the garden it flies over the pits and swoops at where you are going, not where you are. Where foes fight blow for blow it is a light Skirmisher that pecks from beside you: parry the peck, or step out of its reach and punish.",
            "Odin's ravens brought him news. These bring nothing but their beaks. Keep moving, and let them strike the place you were going to be.",
        ),
        K::Vampire => l(
            "Night Raven",
            "German: Nachtkrapp, \"night raven\"",
            "Swabian and Austrian folklore",
            "Parents in Swabia and Austria warned of the Nachtkrapp, a raven bigger than any other, that came in the night for children still awake past their bedtime. It was a bogeyman's tale, told to send children to bed. Here it is true.",
            "In the garden it flies, and it hunts with the draugr: it makes for the far side of you from the draugr, to catch you between them. Where foes fight blow for blow it is a Skirmisher that pecks from beside you: parry it, and it reels.",
            "It was a tale to send children to bed. You are awake for a good reason, so do not let it frighten you.",
        ),
        K::Slime => l(
            "Bog Slime",
            "",
            "This place: the garden's own",
            "No old tale tells of the bog slime. It is the garden's own, the green ooze of its standing water, and here it moves as if it wanted something.",
            "In the garden it oozes toward you from afar and slinks back to its corner when you come close. It leaves a slick trail that slows you, and it will not cross the bridges. Where foes fight blow for blow it is a Hexer that spits from a distance: close in between its spits.",
            "Mind your boots. Slime washes off; fear does not. Leave the fear behind, and keep to the bridges.",
        ),
        // ---- the second stage
        K::Helmeted => l(
            "Helmeted Skeleton",
            "Old Norse: draugr",
            "Norse sagas",
            "The old warriors were laid in their mounds with their war gear about them. Here a draugr of the Cisterns still wears his iron cap, and the cap still does its work.",
            "Its iron cap rings off your first blow: it takes two to fell. Like every draugr it falls to pieces, pulls itself together and hunts by the sound of your steps. Where foes fight blow for blow it is a small Brute, quick for one: wait out its chop, then punish.",
            "A helmet is only as brave as the head inside it, and there is no head inside. Strike twice.",
        ),
        K::LargeBat => l(
            "Large Bat",
            "",
            "European folk belief",
            "In old Europe bats were creatures of the dusk, of church towers and ruined halls, and many thought them unlucky. The vampire bats of the Americas were named after the legend, not the other way round. Here, in the Cisterns, the bats grow large.",
            "It swoops at where you are heading, a little short of it, and swerves off its line now and then, so it is hard to read. Where foes fight blow for blow it is a Skirmisher that bites from beside you: parry, or step aside and punish.",
            "It does not know where it is going either. Stand your ground and let it come to you.",
        ),
        K::Witch => l(
            "Witch",
            "",
            "European folklore",
            "In the folk tales of old Europe the witch works harm by her spells, and sends small spirits, her familiars, to do her bidding. Here she is the hag of the Cisterns.",
            "In the dungeons she keeps her distance, backs away when you come close, and conjures minions at her feet while you are within her reach. It takes two blows to fell her. Where foes fight blow for blow she is the Hexer herself, sending bolts from afar: close in between them, and a parry throws a bolt back.",
            "Go for the witch, not her little ones. Cut the root and the weeds stop growing.",
        ),
        K::Minion => l(
            "Witch Minion",
            "",
            "European folklore: the familiar",
            "A familiar was a witch's helper spirit, said to take the shape of a small animal. Here the witch's minions are small, quick things she calls up in a puff of violet.",
            "Small and quick, it runs straight at you, and one blow is enough. Where foes fight blow for blow it is the lightest Skirmisher, biting from beside you and breaking at a touch.",
            "They are many and they are small. Count them later. For now, swing.",
        ),
        K::Werewolf => l(
            "Loup-Garou",
            "French: loup-garou",
            "French and French-Canadian folklore",
            "The loup-garou is a man who becomes a wolf by night, under a curse or by his own wicked doing. In many tales the curse is broken by drawing a drop of his blood, and the man is free again.",
            "In the dungeons it runs straight for you and takes two blows; wound it, and it howls and comes on faster for a few seconds. Where foes fight blow for blow it is the Skirmisher itself, biting from beside you: parry the bite, or step out of its reach and punish its recovery.",
            "Under the fur is a man who lost his way. Be quick, and keep a little kindness in your heart, even while your arm has none.",
        ),
        K::Blemmyes => l(
            "Blemmyes",
            "Latin and Greek: Blemmyae",
            "Ancient and medieval travellers' tales",
            "Pliny and the medieval travellers wrote of the Blemmyae beyond the Nile, a people with no heads, their eyes and mouths set in their chests. The real Blemmyes were a people of the Nubian desert, and the monster grew out of rumour.",
            "A headless brute: three blows to fell, and his touch hurts twice as much. Where foes fight blow for blow he is the Brute himself, a slow slam ahead and a sweep when you flank him: step out of the slam, punish its long recovery, and break him with your heaviest blow.",
            "He has no head, so he cannot be talked out of anything. Be patient instead: his blow is slow, and yours is not.",
        ),
        // ---- the third stage
        K::Tengu => l(
            "Tengu",
            "Japanese: tengu",
            "Japanese folklore",
            "Tengu are the mountain goblins of Japan, part bird and part man, often red-faced and long-nosed, and masters of the sword. Proud monks were said to become tengu, and the tengu of Mount Kurama is said to have taught the young Minamoto no Yoshitsune to fight.",
            "Quick on his feet, he takes two blows. Where foes fight blow for blow he is a Brute who moves half again as fast as one, his sword swung slow and hard: wait for the cut, step out of it, and punish.",
            "Pride made him what he is. Fight him with humility: let him swing first.",
        ),
        K::Rasetsu => l(
            "Rasetsu",
            "Japanese: rasetsu, from the Sanskrit rakshasa",
            "Buddhist tales of India and Japan",
            "In Buddhist tales the rasetsu, the rakshasa of India, are man-eating demons; Japan pictured them as horned, red-skinned brutes. Some were tamed by the faith and became its guardians.",
            "Horned and red, with a curved blade in each fist: three blows to fell, and his touch hurts twice over. Where foes fight blow for blow he is a heavy Brute, harder to stagger than most: wait out his slam, punish, and finish with your heaviest blow.",
            "Even a rasetsu was turned to guard the good, once. Not this one. Be careful.",
        ),
        K::DireWolf => l(
            "Dire Wolf",
            "Old Norse: vargr, \"wolf\"",
            "Norse myth",
            "Norse myth is full of great wolves: Fenrir, whom the gods bound with a ribbon made of impossible things, and his kin Sköll and Hati, who chase the sun and the moon across the sky. Here their lesser kin hunt the Bone Vaults.",
            "Bigger than the loup-garou, it takes two blows. Where foes fight blow for blow it is a heavy Skirmisher whose jaws hit hard, biting from beside you: parry the bite, or step out and punish.",
            "A wolf hunts the one who runs. Do not run: turn, and let it meet your shield.",
        ),
        K::Jorogumo => l(
            "Jorogumo",
            "Japanese: jorōgumo, \"entangling bride\"",
            "Japanese folklore",
            "In Japanese tales a spider that has lived four hundred years becomes a jorōgumo and can take the shape of a lovely woman, who lures travellers into her house. At the Jōren Falls in Izu, a woodcutter resting by the water was nearly pulled in by her silk.",
            "She keeps her distance, comes on when you are far and backs away when you are near; it takes two blows to fell her. Where foes fight blow for blow she is a Hexer who hurls fire from afar: close in between her throws.",
            "Never follow a lovely stranger into the dark. Make her come into the light, and then be quick.",
        ),
        K::Cobra => l(
            "Cobra",
            "",
            "Egypt and India",
            "The cobra is the hooded snake of Africa and India. Old Egypt set it on the pharaoh's crown as the uraeus, rearing up to spit fire at his enemies.",
            "Coiled, it rears and strikes: it takes two blows, and its bite hurts twice over. Where foes fight blow for blow it is a slow Skirmisher whose fangs land hard: parry the strike, or step out of its reach.",
            "A snake rears before it strikes. That is its honesty. Watch for it.",
        ),
        // ---- the fourth stage: the yōkai
        K::Oni => l(
            "Oni",
            "Japanese: oni",
            "Japanese folklore",
            "Oni are the ogres of Japan: horned, red or blue, with an iron club, the kanabō. \"Oni ni kanabō\", an iron club for an oni, is said of the strong made stronger still. On Setsubun families throw roasted beans to drive them out: \"Oni out, good fortune in!\"",
            "A Brute with the iron club. Its Kanabō is a slam no blow of yours can interrupt; only a parry stops it, and the ground shakes all round it as it lands. Stand back from the club, then step in and punish the long recovery.",
            "On Setsubun the children drive the oni out with a handful of beans. You have more than beans. Stand back from the club, then step in.",
        ),
        K::Kappa => l(
            "Kappa",
            "Japanese: kappa, \"river child\"",
            "Japanese folklore",
            "The kappa is a river imp of Japan, the size of a child, with a dish of water on its head that holds its strength. It loves cucumbers and sumo, and it is very polite: bow to it, and it bows back and spills its water, and its strength with it.",
            "A Skirmisher: its Sumo Shove, from beside you, pushes you back. Parry the shove, and it bows and spills its dish: for a few seconds it takes far more from every blow and moves slower. That is your moment.",
            "Courtesy is a weapon too. Meet it well, and it bows to you.",
        ),
        K::Rokurokubi => l(
            "Rokurokubi",
            "Japanese: rokurokubi",
            "Japanese folklore",
            "By day the rokurokubi seems an ordinary woman. By night, while she sleeps, her neck stretches long and her head wanders. Some tales say she does not know it herself.",
            "A slow Skirmisher: beside you, her neck bows forward a little way and her head bites; it never leaves her neck. Watch the neck draw back, parry the bite, and punish.",
            "Perhaps she does not know what she is. That is sad, but it does not make her bite any gentler.",
        ),
        K::Kitsune => l(
            "Kitsune",
            "Japanese: kitsune, \"fox\"",
            "Japanese folklore",
            "Japan's foxes grow wiser and stronger with age, up to nine tails. Some are the good messengers of Inari; the wild ones play tricks, light foxfire, kitsune-bi, in the night and weave illusions. Here it is no messenger of Inari.",
            "A Hexer, quick on its feet, whose Foxfire flies in a fan of three. Wounded deep, it conjures two illusions of itself and swaps places with one. An illusion pops at one blow, its foxfire is harmless, and it casts no light: strike the one that glows.",
            "Look for the light it casts. An illusion has none. Truth always has a little.",
        ),
        // ---- the fifth stage: Anatolian nights
        K::Karakoncolos => l(
            "Karakoncolos",
            "Turkish: karakoncolos",
            "Anatolian and Balkan folklore",
            "On the coldest nights of winter, the zemheri, the karakoncolos walks the streets of Anatolia and the Balkans. It calls people out of their houses by name, in the voice of someone they love, and whoever goes out to answer comes to harm.",
            "A heavy Brute with claws. Its Call roars your name: hear it within its reach as the roar begins and it dooms you, so your next wound bites deeper for a few seconds. It never draws you in. Then come its slam and its sweep.",
            "When the dark calls your name in a loved one's voice, stay by the fire. The ones who love you will knock.",
        ),
        K::Tulukus => l(
            "Tülükuş",
            "Turkish: tülükuş",
            "Anatolian folk belief",
            "Across Anatolia the owl's cry at night was long heard as an omen of death or ruin, and an owl on the roof was not welcome. Here the tülükuş is that dread given wings, and it flies without a sound.",
            "A Skirmisher with talons. Beside you it rises straight up on its wings, out of reach, and drops talons first; its cry as it takes wing dooms you, so your next wound bites deeper. Watch it rise, then step out or parry as it falls.",
            "An owl's cry is only a cry. Step out from under it, and the omen falls on empty ground.",
        ),
        K::Albasti => l(
            "Albastı",
            "Turkish: albastı, also al karısı",
            "Turkish and Turkic folk belief",
            "The albastı, or al karısı, is a night hag of Turkic belief who troubles mothers and their newborns in their first forty days, and sits heavy on a sleeper's chest. Families kept watch, and hung red ribbons and laid iron by the bed to keep her away.",
            "A Skirmisher who is only a shadow with red eyes until she rears up. Beside you she rises and falls on you with all her weight; she never pins you. Parry the fall, or step out of it.",
            "They tied red ribbons and kept watch for her. You are the watch now. Keep your eyes on the red.",
        ),
        K::Cin => l(
            "Cin",
            "Turkish: cin, from the Arabic jinn",
            "Islamic and Anatolian belief",
            "The Quran tells of the jinn, made of smokeless fire, unseen unless they wish it, some good and some wicked. In Anatolia a sudden harm with no cause was called cin çarpması, \"struck by a cin\".",
            "A Hexer whose Smokeless Fire comes out of nowhere, an ordinary bolt you barely see begun. Come within a few steps and it vanishes in smoke and stands farther off, though only once every few seconds. Make it vanish, then rush it where it stands again.",
            "Not every cin is wicked, the old women said. This one is. Close in twice, and quickly: it cannot vanish again so soon.",
        ),
        K::Arcura => l(
            "Arçura",
            "Turkish: arçura",
            "Turkic forest lore",
            "In Turkic belief the arçura is a spirit of the forest that can grow as tall as the tallest tree, or shrink as small as a blade of grass. It guards the woods.",
            "A Hexer that grows tall as a tree to cast, flinging thorny seeds; roots burst up where one lands, though they never hold you. Then it shrinks to the grass and slips away half again as fast. Catch it small.",
            "Big or small, it is the same spirit. Do not let its size tell you how afraid to be.",
        ),
        // ---- the sixth stage: the legends
        K::Tepegoz => l(
            "Tepegöz",
            "Turkish: tepegöz, \"eye on the crown\"",
            "The Book of Dede Korkut, Oghuz Turkic",
            "In the Book of Dede Korkut, Tepegöz is a one-eyed giant, son of a shepherd and a peri. His peri mother put a ring on his finger so that no arrow would pierce him and no sword would cut him, and he made the Oghuz pay him tribute until the hero Basat put out his one eye with a heated spit.",
            "A giant Brute. No blade cuts his body: only a small part of every blow tells, unless his eye comes within reach, bent low in his recovery or reeling. Then the whole blow lands, and his eye lights up. Wait for his blow, then strike the eye.",
            "Basat did not try to cut the giant's skin. He waited, and he struck the eye. Be Basat.",
        ),
        K::Gashadokuro => l(
            "Gashadokuro",
            "Japanese: gashadokuro, \"starving skeleton\"",
            "Japanese folklore, of the twentieth century",
            "The gashadokuro is a giant skeleton gathered from the bones of those who died of hunger and lay unburied. It is a young yōkai, first written of in the twentieth century, and often pictured after Kuniyoshi's print of a great skeleton. A ringing in the ears is its only warning.",
            "A giant Brute, heard before it is seen: a bell rings as it moves to strike. Its Grab is two hands that no shield holds; parry them or leap, or they crush you all at once and let go. Its slam and sweep come after.",
            "When you hear the bell, do not lift your shield. Lift your feet, or meet its hands at the last moment.",
        ),
        K::Ejder => l(
            "Yedi Başlı Ejder",
            "Turkish: yedi başlı ejder, \"seven-headed dragon\"",
            "Anatolian folk tales",
            "In Anatolian folk tales a dragon with seven heads sits on the town's spring and lets no water flow without a tribute. The hero cuts it down, and when it begs \"Bir daha vur!\", \"Strike once more!\", he answers \"My mother bore me but once\" and holds his blade: a second blow would bring it back.",
            "A huge Skirmisher whose Seven Heads bite four times from beside you, one strike that a single parry stops whole. Felled, it begs for a moment: strike again and it rises with a head grown back; hold your blade, and it dies.",
            "When it begs, remember the story. Your mother bore you but once. Lower your blade, and let it go.",
        ),
        K::Goryo => l(
            "Goryō",
            "Japanese: goryō, \"honoured spirit\"",
            "Japanese belief of the Heian court",
            "Goryō are the angry ghosts of nobles who died wronged. When Sugawara no Michizane died in exile, lightning struck the palace and misfortune fell on those who had wronged him; the court restored his honour, and he is worshipped still as Tenjin, the god of learning.",
            "A Hexer whose Thunderbolt is flung at you and comes down as lightning where it lands. Nothing touches it but while it rages, in the middle of its own moves: strike it as it casts.",
            "Michizane's anger was quieted when the wrong done to him was set right. I cannot set this one right. Strike while it is angry: that is when it is real.",
        ),
        // ---- the seventh stage: Ragnarök
        K::FrostGiant => l(
            "Frost Giant",
            "Old Norse: hrímþurs, \"rime-giant\"",
            "Norse myth",
            "In Norse myth the first of all beings was Ymir, a giant made of frost, and from him came the rime-giants of Jötunheim, ever at odds with the gods. At Ragnarök they march against Asgard.",
            "A giant Brute whose blows frostbite, even through your shield: a little of your health, again and again, for a few seconds. It never slows you. Its Frost Slam brings both fists down beside you, and ice bursts from the floor as far as they reach. Step back, then punish.",
            "Cold makes the hands slow, and the heart slower. Not yours. Keep moving, and keep warm.",
        ),
        K::Surtr => l(
            "Surtr",
            "Old Norse: Surtr, \"the black one\"",
            "Norse myth: the Völuspá",
            "Surtr is the fire giant of Muspelheim, who stands at its border with a flaming sword. At Ragnarök he rides against the gods, fights Freyr, and his fire burns the world. The old poem says the earth rises again afterwards, green out of the sea.",
            "A giant Brute: his slam and his sweep leave fire on the floor for a few seconds, and it burns whoever stands in it. Step out of the flames before you punish him.",
            "After the fire the earth rises again, green. Remember that, and step out of the flames.",
        ),
        K::Nidhogg => l(
            "Nidhogg",
            "Old Norse: Níðhöggr, \"malice striker\"",
            "Norse myth",
            "Níðhöggr is the dragon that gnaws at the roots of Yggdrasil, the world tree, from below. The squirrel Ratatoskr runs up and down the trunk carrying insults between him and the eagle in the branches.",
            "A huge Skirmisher. It burrows into the floor and comes up beneath you on a mark that follows you, then holds still: walk out of the mark, or leap. Its bites carry venom, a little of your health again and again for a few seconds.",
            "It gnaws at roots because it cannot reach the branches. Do not let it reach yours. When the ground swells, move.",
        ),
        K::Hel => l(
            "Hel",
            "Old Norse: Hel",
            "Norse myth: Snorri's Edda",
            "Hel is Loki's daughter, whom Odin sent to rule the dead in Niflheim; Snorri says she is half living flesh and half dark. When the gods begged her to give back Baldr, she asked that every thing in the world weep for him first.",
            "A Hexer, queen of the dead. Her bolts drain: what they take from you, she heals. She raises two draugr out of the floor about you, and they fall when she does. Go for her, not her dead.",
            "Every thing in the world wept for Baldr, but one. Do not grieve here: fight. There will be time for tears in the light.",
        ),
        // ---- the realms' own
        K::Peerless => l(
            "The Peerless",
            "",
            "This place: Pride's spire",
            "Ovid tells of Narcissus, who fell in love with his own reflection in a pool and could not leave it. Here, at the top of Pride's spire, your reflection steps out of the glass to take the throne in your place.",
            "It repeats your moves a moment later, mirrored, and grows stronger with your Glory. Throw one blow, then guard, and parry its echo. Strike only while it reels; a drink it cannot copy leaves it posing, open. Each blow it lands drops a ring of the floor, and at the third you fall.",
            "It has your face and none of your heart. Be humble, and it has nothing left to copy.",
        ),
        K::Ganglot => l(
            "Ganglöt",
            "Old Norse: Ganglati and Ganglöt, \"slow-walkers\"",
            "Norse myth: Snorri's Edda",
            "Snorri tells that Hel's servants are Ganglati and Ganglöt, the slow-walkers, in her hall where the dish is called Hunger and the knife Famine. Here they wade through the Grey Mire of Sloth.",
            "Snail-slow but hard to stop: a Brute that wades in and keeps coming, its poise so deep that light blows hardly shake it. In the Grey Mire they climb out of the bog the longer you rest. Keep moving, and break them with your heaviest blows.",
            "They are slow, and they never stop. That is the lesson of this place: neither should you.",
        ),
        K::Gaki => l(
            "Gaki",
            "Japanese: gaki, from the Sanskrit preta, \"hungry ghost\"",
            "Buddhist belief of Japan",
            "In Buddhist belief the gaki are hungry ghosts, reborn for their greed with swollen bellies and throats as thin as needles, so that they can never be filled; their food turns to fire in their mouths. The Segaki rite offers them food to ease their hunger.",
            "A Skirmisher that comes in numbers, biting from beside you. In the Rotting Feast they smell food on you: carry three or more Bloat and they come for you. Parry the bites, and keep yourself lean.",
            "Pity them: they are always hungry. But do not feed them, and least of all with yourself.",
        ),
        K::Ellepige => l(
            "Ellepige",
            "Danish: ellepige, \"elf-maid\"",
            "Danish folklore and ballads",
            "In Danish folklore the elf-maids dance in the meadows by moonlight. From the front they are lovely; from behind they are hollow as a kneading trough. In the ballad of Elveskud, Sir Oluf refused the elf-king's daughter a dance on the way to his wedding, and never reached it.",
            "A Skirmisher, lovely from the front, where no blow tells: strike only her hollow back. Step round her, or leap past her, then strike. Asmodeus calls two of them to his side.",
            "What is lovely from the front may be hollow behind. Turn her round, and see her as she is.",
        ),
        // ---- the princes' courts
        K::Mammon => l(
            "Mammon",
            "Aramaic: māmōnā, \"wealth\"",
            "The Gospels, and Binsfeld's princes of sin (1589)",
            "Mammon is an old word for wealth: \"Ye cannot serve God and mammon\", says the Gospel of Matthew. Later writers made the word into a spirit of greed, and in 1589 Peter Binsfeld named Mammon the prince of greed. Here he weighs every soul by what it holds.",
            "A prince whose gold hide turns most of every blow, except in his recoveries or when a parry leaves him reeling. The Banker tosses coins that burst on the floor (no shield stops them: step off or leap) and sweeps his sceptre wide. The Collector's grasp can only be parried; if his fist closes it holds your feet and takes your silver until you strike him, and two gold golems climb from his heaps. The Hoard rains coins, slams the floor and throws coins in fives.",
            "He weighs every soul by what it holds. You left his gold at the door; now keep your silver, and strike him the moment his fist closes.",
        ),
        K::GoldGolem => l(
            "Gold Golem",
            "Hebrew: golem, \"unformed\"",
            "Jewish folklore",
            "In Jewish folklore a golem is a figure of clay brought to life by holy words; the best known is the golem of Prague, said to have been made by Rabbi Judah Loew to protect his people. Here Mammon raises golems of coins and bars from his own heaps, and they protect nothing but his gold.",
            "A slow, heavy Brute, hard to stagger. Two climb out of Mammon's heaps as his second mood begins, once; none if his general fell, at once if his general's decree reached the court. Wait out its slam, punish, and keep your eyes on Mammon.",
            "Gold that walks is still only gold. It cannot love what it guards.",
        ),
        K::Lucifer => l(
            "Lucifer",
            "Latin: lucifer, \"light-bearer\", the morning star",
            "Isaiah, and Binsfeld's princes of sin (1589)",
            "Lucifer is the Latin name of the morning star. Isaiah's song mocks a proud king as the morning star who said \"I will exalt my throne above the stars\", and was brought low. Binsfeld named Lucifer the prince of pride; here he keeps his court on the Throne of the Morning Star.",
            "A prince whose light turns most of a blow; but pride never looks behind him, and a blow on his back, or in his recoveries, lands whole. The Morning Star burns a Lance of Dawn along the floor (a shield holds it, no parry turns it) and cuts wide. The Mirror steps out of the glass nearest you, and two Reflections with him. The Fallen calls stars down round you, and crashes down on a mark that follows you.",
            "He would sit above the stars. Let him look up; you look at his back.",
        ),
        K::Reflection => l(
            "Reflection",
            "",
            "This place: Pride's court",
            "A likeness of Lucifer stepped out of the light, cold silver and left-handed, with a silver halo. Here pride makes copies of itself.",
            "A Brute that cuts with Lucifer's Radiant Cut at half his strength, and shatters at one good blow. Two step out of the mirrors at his second mood, once; none if the Chamberlain of Glass fell, at his side from the start if the throne-mirror was polished. The real one has the gold halo.",
            "Break the glass and the copy is gone. Only the one with the gold halo is real.",
        ),
        K::Leviathan => l(
            "Leviathan",
            "Hebrew: livyatan",
            "The Hebrew Bible, and Binsfeld's princes of sin (1589)",
            "Leviathan is the great serpent of the sea in the Hebrew Bible: in Job no hook can draw him out, and Isaiah calls him the crooked serpent. Binsfeld named him the prince of envy. Here his court is a drowned temple round the bell of Dōjō-ji, the bell the jealous Kiyohime coiled round as a serpent.",
            "A prince with a hide of scales: strike in his recoveries, or when a parry leaves him reeling. The Crooked Serpent spits brine in threes, lashes his tail wide, and dives to come up under a ring that follows you. The Evil Eye covets the archangel you called on last and turns his power on you in green. The Bell coils round the hot bronze, whose heat burns his scales away on that side.",
            "Envy wants what you have, even your angels. Use them with care, and strike him from the bell's side.",
        ),
        K::Serpentling => l(
            "Serpentling",
            "",
            "This place: Envy's court",
            "One of Leviathan's brood, a sea snake risen from the springs of his drowned court. Here they are envy in small.",
            "A Skirmisher that bites from beside you. Two rise from the springs nearest you at his second mood, once; none if the Bell-Warden fell, at once if the bell was lowered by his decree.",
            "Small snakes of a great one. Parry them, and keep your eyes on the father.",
        ),
        K::Amon => l(
            "Amon",
            "Amon, a marquis of the Ars Goetia",
            "The Ars Goetia, a book of spirits of the seventeenth century",
            "The Ars Goetia names Amon a great marquis who comes as a wolf with a serpent's tail, vomiting flame, and at the conjurer's word takes the shape of a man with a raven's head. The old book says he stirred up feuds, and could mend quarrels between friends. Here he only feeds them, as the prince of wrath.",
            "A prince with no hide: every blow lands whole, but every blow you land makes him angrier and quicker, five steps of rage at most. He lowers his head and charges in a straight line shown on the floor; step aside as it locks, and if he runs into a pillar, the wall or his anvil he stands dazed and his rage goes out of him. The Raven splits the floor into lava; the Fury charges twice, and the floor erupts.",
            "Anger runs in straight lines. Stand before the stone, step aside, and let his own rage stun him.",
        ),
        K::Raven => l(
            "Raven of Amon",
            "",
            "This place: Wrath's court",
            "Amon's black birds. The Ars Goetia gives him a raven's head when he takes a man's shape; here his ravens come down from the burning banners to harry you.",
            "A light Skirmisher that pecks from beside you. His ravens come down at his second mood, once; none if Marshal Gorebrand fell, at once if the bridges were burnt by his decree.",
            "Birds of war. Parry them, and they scatter.",
        ),
        K::Belphegor => l(
            "Belphegor",
            "Hebrew: Baal-Peor, \"lord of Peor\"",
            "The Book of Numbers, and Binsfeld's princes of sin (1589)",
            "Belphegor began as Baal-Peor, a god worshipped on Mount Peor in Moab, named in the Book of Numbers. Later demonology made him the prince of sloth, who tempts people with clever inventions and riches without labour. Here he sleeps in the Hall of Stopped Clocks.",
            "A prince who begins asleep in his chair: your first blow lands whole and wakes him. Awake, his torpor turns most of a blow except in his recoveries. The Sleeper yawns you back, swats wide and lets sand drift down to burst. The Dreamer lies down and mends while two nightmares fight: strike him hard and he wakes. The Waking stands for good, slow but vast.",
            "I know his sin: I slept the night Krampus slipped past my gate. Do not stand back and wait. Wake him, and keep him awake.",
        ),
        K::Nightmare => l(
            "Nightmare",
            "Old English: mære; Old Norse: mara",
            "Germanic folk belief",
            "In old Germanic belief the mare was a spirit that sat on a sleeper's chest and brought bad dreams; it gave the nightmare its name. Here the nightmares are Belphegor's dreams, sent out while he sleeps.",
            "A Hexer, a dream-wisp that keeps its distance and sends its hex. Two come out of his dream at his second mood, once; none if the Lamplighter fell, at once if the hourglass was wound. They fall when he does.",
            "Bad dreams end when the sleeper wakes. Wake him.",
        ),
        K::Beelzebub => l(
            "Beelzebub",
            "Hebrew: Baal-Zebub, \"lord of the flies\"",
            "The Book of Kings, and Binsfeld's princes of sin (1589)",
            "Baal-Zebub, \"lord of the flies\", was the god of the Philistine city of Ekron in the Book of Kings. Collin de Plancy's Dictionnaire infernal pictured him as a great fly, and Binsfeld named him the prince of gluttony.",
            "A prince with a hide of bloat. The Lord of Flies chops with his cleaver and flings slow flies in a fan. The Swarm bursts into flies and lands on a ring that follows you. The Glutton rolls, spews bile that burns where it lies, and slams his belly. Hurt, he walks to his table to eat and mends: strike him while he eats, and he chokes, open.",
            "He is never full. Take the green crosses before he does, and spoil his supper.",
        ),
        K::FlySwarm => l(
            "Fly Swarm",
            "",
            "This place: Gluttony's court",
            "A cloud of Beelzebub's flies, gathered into one hungry body. Here even the air is greedy.",
            "A quick, light Skirmisher. Two rise from the food beside the platters nearest you at his second mood, once; none if the Steward fell, at once if the last course was laid.",
            "Flies go where the food is. Do not linger by the table.",
        ),
        K::Asmodeus => l(
            "Asmodeus",
            "Hebrew: Ashmedai",
            "The Book of Tobit, and Binsfeld's princes of sin (1589)",
            "In the Book of Tobit, Asmodeus loved Sarah and killed each of her seven bridegrooms on the wedding night, until Tobias, taught by the archangel Raphael, burned a fish's heart and liver on the embers of incense and the smoke drove him off to Egypt, where Raphael bound him. The Ars Goetia gives him three heads, a bull's, a man's and a ram's.",
            "A prince whose allure turns most of a blow, except in his recoveries and while a censer smokes. Strike a cold censer and it smokes for ten seconds: his allure is gone, and he will not come near it. His blown kiss is a heart no shield stops; leap it or cut it from the air, or you cannot guard for a while. His lance reaches far but narrow, thorns burst in lines and rings, the bull's head charges and the ram's shoves.",
            "Raphael taught Tobias with smoke. I teach you the same: light the censers, and keep your heart your own.",
        ),
        // ---- the bottom of the Pit
        K::Krampus => l(
            "Krampus",
            "German: Krampus, from Krampen, \"claw\"",
            "Alpine folklore",
            "In the Alps, Krampus is the horned, shaggy companion of St Nicholas. On Krampusnacht, the night of 5 December, the saint brings gifts to good children while Krampus rattles his chains and bells, swishes his birch at the naughty, and carries the worst off in the basket on his back. Here he took your friend at the graveyard at dusk.",
            "The last and greatest of them, on a prince's rules: his pelt turns most of a blow except in his recoveries, or when a parry leaves him reeling. The Bells: his toll shakes the floor round him, his chain sweeps wide, his birch lashes. The Chains: a chain lashed along a line (a shield holds it, no parry turns it) and a hoof charge that runs straight; let him run into the ice, and he stands dazed. Krampusnacht: live coals in a row that burn on, and the sack swung down. When he goes to rattle the cage, strike him.",
            "Nothing he does can harm the boy. Keep your eyes on Krampus, and keep your heart on your friend.",
        ),
        K::Percht => l(
            "Percht",
            "German: Percht, plural Perchten",
            "Alpine folklore",
            "In the winter nights between Christmas and Epiphany, the Perchten go through the Alpine villages: masked, shaggy figures with horns and cowbells, some fair and some ugly, driving out the winter's spirits. They are named after Perchta, an old figure of the midwinter nights.",
            "A Brute with a birch switch: a slow slam ahead, and a sweep when you flank it. Two come out of the clefts as Krampus's Chains begin, once. Wait out the switch, then punish.",
            "Under the mask is only a mask. The villagers made them to frighten the winter away; frighten them back.",
        ),
    }
}

/// A general's page: his tale is the game's own; how he fights comes from his orders.
fn general_lore(sin: Sin) -> Lore {
    match sin {
        Sin::Greed => l(
            "The Assessor",
            "",
            "This place: the Vaults of Mammon",
            "No old tale knows him. Here he is Mammon's hand in the Vaults, a gilded blemmyes who counts what you carry.",
            "He waits at his command post with his guard, and fights as a blemmyes does, far tougher; at half his health he calls his guard to him. He runs the vaults by orders, each through an instrument three blows will break: he musters at the counting-bell, kindles coin traps with the coin-bellows, and sends his decree down the treasury chain. Slay him, and Mammon fights without his golems; let the decree reach the court, and they rise at once.",
            "He counts your coins. Let him count: what you carry down here is not in your purse.",
        ),
        Sin::Pride => l(
            "The Chamberlain of Glass",
            "",
            "This place: the Hall of Mirrors",
            "No old tale knows him. Here he is Lucifer's hand in the Hall of Mirrors, a tengu in white and gold; in Japan the proud were said to become tengu.",
            "He waits at his command post with his guard, and fights as a tengu does, far tougher; at half his health he calls his guard to him. He sets the watch at the mirror-fire, kindles light beams with the lens-bellows, and sends his decree up the throne chain to polish the throne-mirror. Each instrument breaks under three blows. Slay him, and no Reflections come to Lucifer's side; let the decree be done, and they stand beside him from the start.",
            "A chamberlain of glass breaks like glass. Break his instruments first, then the man.",
        ),
        Sin::Envy => l(
            "The Bell-Warden",
            "",
            "This place: the Serpent Grotto",
            "No old tale knows him. Here he is Leviathan's hand in the Serpent Grotto, a kappa green as old bronze, keeper of the drowned bell.",
            "He waits at his command post with his guard, and fights as a kappa does, far tougher; at half his health he calls his guard to him. He kindles geysers at the geyser valve, musters the drowned at the drowned bell, and lowers the bell by the bell chain. Each instrument breaks under three blows. Slay him, and no serpentlings rise for Leviathan; let the bell be lowered, and they come at once.",
            "Bow to him as you would to any kappa. Then strike while his dish is spilled.",
        ),
        Sin::Wrath => l(
            "Marshal Gorebrand",
            "",
            "This place: the Burning Field",
            "No old tale knows him. Here he is Amon's hand on the Burning Field, a scarred oni who has never once called a truce.",
            "He waits at his command post with his guard, and fights as an oni does, far tougher; at half his health he calls his guard to him. He musters at the war horn, kindles the lava sluice, and sends his sappers to burn the bridges by the bridge chain. Each instrument breaks under three blows. Slay him, and Amon's ravens never come; let the bridges burn, and they come at once.",
            "A marshal of wrath. Break his horn, and he shouts at no one.",
        ),
        Sin::Sloth => l(
            "The Lamplighter",
            "",
            "This place: the Grey Mire",
            "No old tale knows him. Here he is Belphegor's hand in the Grey Mire, a hooded karakoncolos with a lantern, who lights lamps for sleepers.",
            "He waits at his command post with his guard, and fights as a karakoncolos does, far tougher; at half his health he calls his guard to him. He sets the watch at the lantern-fire, kindles smouldering lamps with the lamp-bellows, and winds the hourglass by the hourglass chain. Each instrument breaks under three blows. Slay him, and Belphegor dreams without nightmares; let the hourglass be wound, and Belphegor wakes rested, with no free blow, and his nightmares come at once.",
            "He lights lamps so that others may sleep by them. Put out his lamps, and keep your own eyes open.",
        ),
        Sin::Gluttony => l(
            "The Steward of the Last Course",
            "",
            "This place: the Rotting Feast",
            "No old tale knows him. Here he is Beelzebub's hand in the Rotting Feast, an aproned gaki who serves what he can never eat.",
            "He waits at his command post with his guard, and fights as a gaki does, far tougher; at half his health he calls his guard to him. He musters at the dinner gong, kindles acid tides at the acid valve, and has the cooks lay the prince's feast by the larder chain. Each instrument breaks under three blows. Slay him, and no fly swarms come; let the feast be laid, and six platters wait for Beelzebub and the swarms come at once.",
            "Even the steward is hungry. Leave his table, and he has nothing to serve you.",
        ),
        Sin::Lust => l(
            "The Bridesmaid",
            "",
            "This place: the Garden of Thorns",
            "No old tale knows her. Here she is Asmodeus's hand in the Garden of Thorns, a veiled albastı who leads the wedding procession.",
            "She waits at her command post with her guard, and fights as an albastı does, far tougher; at half her health she calls her guard to her. She sets the watch with the procession's torch, kindles censer smoke at the censer, and speaks the vows by the vow-chain. Each instrument breaks under three blows. Slay her, and no elf-maids come to Asmodeus; let the vows be spoken, and they come at once, and each censer lights only once.",
            "No wedding down here is worth your heart. Snuff her torch, and walk on.",
        ),
    }
}

/// A rival's page, made from what the Ledger knows of him: his tale, and the old man's word.
pub fn rival_words(breed: &str, alive: bool) -> (String, &'static str) {
    let tale = format!(
        "No tale tells of him but yours. He is a {breed} who did a deed against you down here, and the deed gave him a name; he remembers it, and he remembers you."
    );
    let word = if alive {
        "He learns from every fight, as you do. Learn faster, and do not let his name grow longer than yours."
    } else {
        "He is gone, and his name with him. Do not carry his grudge any further than he could."
    };
    (tale, word)
}

/// Every page's words as a Markdown text, group by group (for reading and correcting the
/// writing outside the game: `VH_BESTIARY_OUT=entries.md cargo test -- --ignored write_the_book_out`).
pub fn as_markdown() -> String {
    let mut out = String::from(
        "# The bestiary: every page's words

",
    );
    out.push_str("Generated from `src/logic/bestiary.rs`. Each page: its name, its name in its own tongue, where its tale comes from, the archetype it fights as (from the code), the legend, how it fights, and the old man's line.
");
    for g in GROUPS {
        out.push_str(&format!(
            "
## {} ({})

_{}_
",
            g.title(),
            g.short(),
            g.blurb()
        ));
        if g == Group::Rivals {
            let (tale, alive) = rival_words("[breed]", true);
            let slain = rival_words("[breed]", false).1;
            out.push_str(&format!(
                "
The rivals' pages are made from the Ledger of Grudges: the name the game gave him, his breed and rank, what the player found of him.

- **The legend:** {tale}
- **The old man, while he lives:** {alive}
- **The old man, once he is slain:** {slain}
"
            ));
        }
        for e in g.entries() {
            let l = lore(e);
            out.push_str(&format!(
                "
### {}

",
                l.name
            ));
            if !l.native.is_empty() {
                out.push_str(&format!(
                    "- **Its own name:** {}
",
                    l.native
                ));
            }
            out.push_str(&format!(
                "- **Where it comes from:** {}
",
                l.origin
            ));
            if let Some((a, how)) = archetype(e) {
                out.push_str(&format!(
                    "- **Fights as:** {} ({})
",
                    question(a),
                    how.name()
                ));
            }
            out.push_str(&format!(
                "- **The legend:** {}
- **How it fights:** {}
- **The old man says:** {}
",
                l.legend, l.fights, l.old_man
            ));
        }
    }
    out
}

// ------------------------------------------------------------------ the record

/// What the player has done with one page's foe.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Tally {
    /// Times met (each foe once a level).
    pub met: u32,
    /// Times felled.
    pub felled: u32,
    /// Times it felled the hero.
    pub felled_by: u32,
}

impl Tally {
    /// Anything at all: the page is open.
    pub fn any(&self) -> bool {
        self.met > 0 || self.felled > 0 || self.felled_by > 0
    }
}

/// A rival the book remembers: as the Ledger named him, and what the player did with him.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RivalNote {
    /// His full name with his epithet.
    pub name: String,
    pub kind: EnemyKind,
    pub rank: u8,
    pub alive: bool,
    pub tally: Tally,
}

/// The book as saved: a tally for each page met, and the rivals.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Book {
    tallies: HashMap<Entry, Tally>,
    pub rivals: Vec<RivalNote>,
}

impl Book {
    /// What the player has done with a page's foe.
    pub fn tally(&self, entry: Entry) -> Tally {
        self.tallies.get(&entry).copied().unwrap_or_default()
    }

    /// The page is open: met, felled or felled by.
    pub fn unlocked(&self, entry: Entry) -> bool {
        self.tally(entry).any()
    }

    /// Pages open, of every fixed page, and rivals remembered.
    pub fn count(&self) -> usize {
        self.tallies.values().filter(|t| t.any()).count() + self.rivals.len()
    }

    /// Nothing in it yet.
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    pub fn met(&mut self, entry: Entry) {
        self.tallies.entry(entry).or_default().met += 1;
    }

    pub fn felled(&mut self, entry: Entry) {
        let t = self.tallies.entry(entry).or_default();
        t.felled += 1;
        t.met = t.met.max(1);
    }

    pub fn felled_by(&mut self, entry: Entry) {
        let t = self.tallies.entry(entry).or_default();
        t.felled_by += 1;
        t.met = t.met.max(1);
    }

    /// The rival of this name, if the book remembers him.
    pub fn rival(&self, name: &str) -> Option<&RivalNote> {
        self.rivals.iter().find(|r| r.name == name)
    }

    pub fn rival_mut(&mut self, name: &str) -> Option<&mut RivalNote> {
        self.rivals.iter_mut().find(|r| r.name == name)
    }

    /// A rival as the Ledger has him now: remembered (or brought up to date). True if anything changed.
    pub fn remember_rival(&mut self, name: &str, kind: EnemyKind, rank: u8, alive: bool) -> bool {
        let name = clean(name);
        if let Some(r) = self.rival_mut(&name) {
            if (r.kind, r.rank, r.alive) == (kind, rank, alive) {
                return false;
            }
            (r.kind, r.rank, r.alive) = (kind, rank, alive);
            return true;
        }
        self.rivals.push(RivalNote { name, kind, rank, alive, tally: Tally::default() });
        if self.rivals.len() > REMEMBERED_RIVALS {
            self.rivals.remove(0);
        }
        true
    }

    /// The save's line: "b1;slug=met,felled,by;...;r=kind,rank,alive,met,felled,by,name".
    pub fn encode(&self) -> String {
        let mut items: Vec<String> = all_entries()
            .into_iter()
            .filter_map(|e| {
                let t = self.tally(e);
                t.any().then(|| format!("{}={},{},{}", slug(e), t.met, t.felled, t.felled_by))
            })
            .collect();
        items.sort();
        let mut out = String::from(VERSION);
        for i in items {
            out.push(';');
            out.push_str(&i);
        }
        for r in &self.rivals {
            let t = r.tally;
            out.push_str(&format!(
                ";r={},{},{},{},{},{},{}",
                kind_slug(r.kind),
                r.rank,
                r.alive as u8,
                t.met,
                t.felled,
                t.felled_by,
                clean(&r.name)
            ));
        }
        out
    }

    /// The book from its saved line; whatever cannot be read is left out (a book from a later
    /// version keeps what this one knows). Anything not a book at all reads as an empty one.
    pub fn decode(text: &str) -> Book {
        let mut book = Book::default();
        let mut items = text.trim().split(';');
        let version = items.next().unwrap_or("");
        if !(version.starts_with('b') && version[1..].parse::<u32>().is_ok()) {
            return book;
        }
        let num = |s: Option<&str>| s.and_then(|v| v.trim().parse::<u32>().ok());
        for item in items {
            let Some((key, value)) = item.split_once('=') else { continue };
            if key == "r" {
                let mut f = value.splitn(7, ',');
                let kind = f.next().and_then(kind_from_slug);
                let rank = num(f.next());
                let alive = num(f.next());
                let (met, felled, by) = (num(f.next()), num(f.next()), num(f.next()));
                let name = f.next().map(clean).filter(|n| !n.is_empty());
                if let (Some(kind), Some(rank), Some(alive), Some(met), Some(felled), Some(felled_by), Some(name)) =
                    (kind, rank, alive, met, felled, by, name)
                {
                    if book.rival(&name).is_none() && book.rivals.len() < REMEMBERED_RIVALS {
                        let tally = Tally { met, felled, felled_by };
                        book.rivals.push(RivalNote { name, kind, rank: rank.clamp(1, 5) as u8, alive: alive != 0, tally });
                    }
                }
                continue;
            }
            let Some(entry) = entry_from_slug(key) else { continue };
            let mut f = value.split(',');
            if let (Some(met), Some(felled), Some(felled_by)) = (num(f.next()), num(f.next()), num(f.next())) {
                book.tallies.insert(entry, Tally { met, felled, felled_by });
            }
        }
        book
    }
}

/// A name made safe for the save's line (no separators, no line breaks).
fn clean(name: &str) -> String {
    name.chars().map(|c| if matches!(c, ';' | '\n' | '\r') { ' ' } else { c }).collect::<String>().trim().to_string()
}

/// A kind's name in the save (stable: never renamed).
pub fn kind_slug(kind: EnemyKind) -> &'static str {
    use EnemyKind as K;
    match kind {
        K::Skeleton => "skeleton",
        K::Bat => "bat",
        K::Vampire => "vampire",
        K::Slime => "slime",
        K::Witch => "witch",
        K::Minion => "minion",
        K::Helmeted => "helmeted",
        K::Werewolf => "werewolf",
        K::Blemmyes => "blemmyes",
        K::LargeBat => "largebat",
        K::Tengu => "tengu",
        K::Rasetsu => "rasetsu",
        K::DireWolf => "direwolf",
        K::Jorogumo => "jorogumo",
        K::Cobra => "cobra",
        K::Kappa => "kappa",
        K::Rokurokubi => "rokurokubi",
        K::Kitsune => "kitsune",
        K::Oni => "oni",
        K::Tulukus => "tulukus",
        K::Albasti => "albasti",
        K::Cin => "cin",
        K::Arcura => "arcura",
        K::Karakoncolos => "karakoncolos",
        K::Tepegoz => "tepegoz",
        K::Gashadokuro => "gashadokuro",
        K::Ejder => "ejder",
        K::Goryo => "goryo",
        K::FrostGiant => "frostgiant",
        K::Surtr => "surtr",
        K::Nidhogg => "nidhogg",
        K::Hel => "hel",
        K::Peerless => "peerless",
        K::Ganglot => "ganglot",
        K::Gaki => "gaki",
        K::Ellepige => "ellepige",
        K::Mammon => "mammon",
        K::GoldGolem => "goldgolem",
        K::Lucifer => "lucifer",
        K::Reflection => "reflection",
        K::Leviathan => "leviathan",
        K::Serpentling => "serpentling",
        K::Amon => "amon",
        K::Raven => "raven",
        K::Belphegor => "belphegor",
        K::Nightmare => "nightmare",
        K::Beelzebub => "beelzebub",
        K::FlySwarm => "flyswarm",
        K::Asmodeus => "asmodeus",
        K::Krampus => "krampus",
        K::Percht => "percht",
    }
}

fn kind_from_slug(s: &str) -> Option<EnemyKind> {
    all_kinds().into_iter().find(|&k| kind_slug(k) == s)
}

/// A page's name in the save.
pub fn slug(entry: Entry) -> String {
    match entry {
        Entry::Foe(k) => kind_slug(k).to_string(),
        Entry::Cyclops => "cyclops".to_string(),
        Entry::Mimic => "mimic".to_string(),
        Entry::General(s) => format!("general.{}", s.name().to_lowercase()),
    }
}

fn entry_from_slug(s: &str) -> Option<Entry> {
    all_entries().into_iter().find(|&e| slug(e) == s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::parse_level;
    use crate::logic::levels::level_for;

    /// Every piece of a page's words.
    fn words(lore: &Lore) -> [&'static str; 6] {
        [lore.name, lore.native, lore.origin, lore.legend, lore.fights, lore.old_man]
    }

    /// Sentences in `text` (each ends in a full stop, a question or an exclamation, outside the quotes too).
    fn sentences(text: &str) -> usize {
        let t = text.replace("!\"", "\"").replace("?\"", "\"");
        t.matches(". ").count() + t.matches("! ").count() + t.matches("? ").count() + 1
    }

    #[test]
    fn every_kind_of_foe_has_a_page_in_one_group() {
        let kinds = all_kinds();
        // (The casts of the seven stages, the realms' own, the courts', the lair's.)
        let cast: usize = (1..=EnemyKind::STAGES).map(|s| EnemyKind::of_stage(s).len()).sum::<usize>()
            + EnemyKind::REALMS_OWN.len()
            + EnemyKind::COURTS_OWN.len()
            + EnemyKind::LAIR_OWN.len();
        assert_eq!(kinds.len(), cast);
        assert_eq!(kinds.len(), 51);
        for k in &kinds {
            assert_eq!(kinds.iter().filter(|&&o| o == *k).count(), 1, "{k:?} is in one group only");
            let entry = Entry::Foe(*k);
            assert_ne!(group_of(entry), Group::Rivals, "{k:?}");
            let lore = lore(entry);
            assert!(!lore.name.is_empty() && !lore.legend.is_empty() && !lore.fights.is_empty() && !lore.old_man.is_empty(), "{k:?}");
        }
        // And the Cyclops, the mimic, the seven generals.
        assert_eq!(all_entries().len(), 51 + 2 + 7);
        assert_eq!(GROUPS.iter().filter(|g| g.entries().is_empty()).count(), 1, "only the rivals' group is the player's own");
    }

    #[test]
    fn each_page_is_named_as_the_game_names_it() {
        let level = parse_level(&level_for(0));
        for k in all_kinds() {
            let spec = crate::logic::enemy::spec_of(&level.grid, k);
            assert_eq!(foe_lore(k).name, spec.name, "{k:?}");
        }
        for s in SINS {
            let name = General::of(s).name.to_lowercase();
            assert_eq!(general_lore(s).name.to_lowercase(), name, "{s:?}");
        }
    }

    #[test]
    fn the_words_are_short_and_whole_and_every_letter_can_be_drawn() {
        let drawable = |c: char| (' '..='~').contains(&c) || ('\u{a0}'..'\u{180}').contains(&c) || "–—‘’“”•…→←↑↓×·♥".contains(c);
        for e in all_entries() {
            let lore = lore(e);
            assert!((2..=4).contains(&sentences(lore.legend)), "{e:?}: the legend is {} sentences", sentences(lore.legend));
            assert!((1..=6).contains(&sentences(lore.fights)), "{e:?}: how it fights is {} sentences", sentences(lore.fights));
            assert!((1..=3).contains(&sentences(lore.old_man)), "{e:?}: the old man's word is {} sentences", sentences(lore.old_man));
            assert!(lore.legend.chars().count() <= 420, "{e:?}: the legend is long");
            assert!(lore.fights.chars().count() <= 520, "{e:?}: how it fights is long");
            assert!(lore.old_man.chars().count() <= 170, "{e:?}: the old man's word is long");
            for w in words(&lore) {
                let bad: Vec<char> = w.chars().filter(|&c| !drawable(c)).collect();
                assert!(bad.is_empty(), "{e:?}: the type cannot draw {bad:?}");
                assert_eq!(w.trim(), w, "{e:?}: stray spaces");
                assert!(!w.contains("  "), "{e:?}: a double space");
            }
            for w in [lore.legend, lore.fights, lore.old_man] {
                assert!(w.ends_with('.') || w.ends_with('!') || w.ends_with('"'), "{e:?}: {w}");
            }
        }
    }

    #[test]
    fn no_page_names_a_devil_and_none_tells_the_end() {
        let banned = [
            "devil",
            "satan",
            "lucifer is the devil",
            "fallen angel",
            "never a devil",
            "the light we choose",
            "together again",
            "cage breaks",
            "lock breaks",
        ];
        let mut texts: Vec<String> = all_entries().into_iter().flat_map(|e| words(&lore(e)).map(String::from)).collect();
        for g in GROUPS {
            texts.extend([g.title().to_string(), g.blurb().to_string()]);
        }
        texts.extend([rival_words("draugr", true).0, rival_words("draugr", true).1.to_string(), rival_words("oni", false).1.to_string()]);
        for a in [Archetype::Brute, Archetype::Skirmisher, Archetype::Hexer, Archetype::Prince] {
            texts.push(question(a).to_string());
        }
        for t in &texts {
            let low = t.to_lowercase();
            for b in banned {
                assert!(!low.contains(b), "\"{b}\" in: {t}");
            }
        }
        // The old man is never a foe: no page is his.
        assert!(all_entries().iter().all(|&e| !lore(e).name.to_lowercase().contains("old man")));
    }

    #[test]
    fn where_the_game_takes_a_liberty_the_page_says_so() {
        // The pages of what no old tale knows say so, and say "here".
        for e in [Entry::Mimic, Entry::Foe(EnemyKind::Slime), Entry::Foe(EnemyKind::Peerless)]
            .into_iter()
            .chain(SINS.iter().map(|&s| Entry::General(s)))
        {
            let lore = lore(e);
            assert!(lore.origin.starts_with("This place"), "{e:?}");
            assert!(lore.legend.contains("Here") || lore.legend.contains("here"), "{e:?}");
        }
    }

    #[test]
    fn every_general_page_names_his_instruments() {
        for s in SINS {
            let g = General::of(s);
            let fights = general_lore(s).fights.to_lowercase();
            for i in g.instruments {
                let i = i.to_lowercase();
                let i = i.trim_start_matches("the ");
                assert!(fights.contains(i), "{s:?}: {i} is not in {fights}");
            }
        }
    }

    #[test]
    fn the_archetype_shown_is_the_code_s() {
        assert_eq!(archetype(Entry::Foe(EnemyKind::Oni)), Some((Archetype::Brute, Attack::Melee)));
        assert_eq!(archetype(Entry::Foe(EnemyKind::Kappa)), Some((Archetype::Skirmisher, Attack::Body)));
        assert_eq!(archetype(Entry::Foe(EnemyKind::Hel)), Some((Archetype::Hexer, Attack::Distance)));
        assert_eq!(archetype(Entry::Foe(EnemyKind::Krampus)).map(|a| a.0), Some(Archetype::Prince));
        assert_eq!(archetype(Entry::General(Sin::Envy)).map(|a| a.0), Some(Archetype::Skirmisher), "the Bell-Warden is a kappa");
        assert_eq!(archetype(Entry::Mimic), None);
        // The words agree with the archetype each foe fights as.
        for k in all_kinds() {
            let (arch, _) = archetype(Entry::Foe(k)).unwrap();
            let fights = foe_lore(k).fights;
            let word = match arch {
                Archetype::Brute => "Brute",
                Archetype::Skirmisher => "Skirmisher",
                Archetype::Hexer => "Hexer",
                Archetype::Prince => "prince",
            };
            // (The Peerless fights with his own kit: the hero's moves turned on him.)
            if k != EnemyKind::Peerless {
                assert!(fights.contains(word), "{k:?} fights as a {word}: {fights}");
            }
            for other in ["Brute", "Skirmisher", "Hexer"].into_iter().filter(|w| *w != word) {
                assert!(!fights.contains(other), "{k:?} is a {word}, not a {other}");
            }
        }
    }

    #[test]
    fn the_book_round_trips_through_its_save_line() {
        let mut b = Book::default();
        assert!(b.is_empty());
        b.met(Entry::Foe(EnemyKind::Oni));
        b.met(Entry::Foe(EnemyKind::Oni));
        b.felled(Entry::Foe(EnemyKind::Oni));
        b.felled_by(Entry::Cyclops);
        b.met(Entry::Mimic);
        b.felled(Entry::General(Sin::Gluttony));
        assert!(b.remember_rival("Kolgrim, Ragnar's Bane", EnemyKind::Karakoncolos, 3, true));
        assert!(!b.remember_rival("Kolgrim, Ragnar's Bane", EnemyKind::Karakoncolos, 3, true), "nothing new");
        assert!(b.remember_rival("Kolgrim, Ragnar's Bane", EnemyKind::Karakoncolos, 4, true), "ranked up");
        b.rival_mut("Kolgrim, Ragnar's Bane").unwrap().tally.felled_by = 2;
        b.remember_rival("Kara; Ayaz\nthe Shield-Breaker", EnemyKind::Oni, 1, false);
        let line = b.encode();
        assert!(line.starts_with("b1;"));
        assert!(!line.contains('\n'));
        let back = Book::decode(&line);
        assert_eq!(back, b.clone_clean());
        assert_eq!(back.tally(Entry::Foe(EnemyKind::Oni)), Tally { met: 2, felled: 1, felled_by: 0 });
        assert_eq!(back.tally(Entry::Cyclops), Tally { met: 1, felled: 0, felled_by: 1 }, "felled by it: met");
        assert!(back.unlocked(Entry::General(Sin::Gluttony)));
        assert!(!back.unlocked(Entry::Foe(EnemyKind::Hel)));
        let k = back.rival("Kolgrim, Ragnar's Bane").unwrap();
        assert_eq!((k.kind, k.rank, k.alive, k.tally.felled_by), (EnemyKind::Karakoncolos, 4, true, 2));
        assert!(back.rival("Kara  Ayaz the Shield-Breaker").is_some(), "the separators made spaces");
        assert_eq!(back.count(), 4 + 2);
    }

    #[test]
    fn an_old_save_or_a_broken_one_reads_as_what_it_can() {
        assert_eq!(Book::decode(""), Book::default(), "no book saved: an empty one");
        assert_eq!(Book::decode("nonsense"), Book::default());
        assert_eq!(Book::decode("v2 0 d 0 0 3 - 0"), Book::default(), "a run's line is not a book");
        // A later version's book: what this one knows of it, and nothing of the rest.
        let b = Book::decode(
            "b7;oni=1,0,0;wyvern=3,3,3;cyclops=x,1,1;hel=2,1;r=oni,2,1,1,0,0,Ulfar the Grim;r=dragon,1,1,0,0,0,X;mimic=1,1,0,9",
        );
        assert!(b.unlocked(Entry::Foe(EnemyKind::Oni)));
        assert!(!b.unlocked(Entry::Cyclops), "a broken number");
        assert!(!b.unlocked(Entry::Foe(EnemyKind::Hel)), "a short tally");
        assert!(b.unlocked(Entry::Mimic), "a longer tally keeps what it knows");
        assert_eq!(b.rivals.len(), 1);
        assert_eq!(b.rivals[0].name, "Ulfar the Grim");
    }

    #[test]
    fn the_book_remembers_only_so_many_rivals_the_newest_kept() {
        let mut b = Book::default();
        for i in 0..REMEMBERED_RIVALS + 5 {
            b.remember_rival(&format!("Rival {i}"), EnemyKind::Oni, 1, true);
        }
        assert_eq!(b.rivals.len(), REMEMBERED_RIVALS);
        assert!(b.rival("Rival 0").is_none() && b.rival(&format!("Rival {}", REMEMBERED_RIVALS + 4)).is_some());
    }

    #[test]
    fn every_slug_is_its_own() {
        let slugs: Vec<String> = all_entries().into_iter().map(slug).collect();
        for (i, s) in slugs.iter().enumerate() {
            assert!(!s.contains([';', '=', ',', ' ']), "{s}");
            assert!(!slugs[i + 1..].contains(s), "{s} twice");
            assert_eq!(entry_from_slug(s).map(slug).as_deref(), Some(s.as_str()));
        }
    }

    #[test]
    #[ignore]
    fn write_the_book_out() {
        let path = std::env::var("VH_BESTIARY_OUT").unwrap_or_else(|_| "entries.md".into());
        std::fs::write(&path, as_markdown()).expect("written");
    }

    #[test]
    fn the_text_for_reading_holds_every_page() {
        let md = as_markdown();
        for e in all_entries() {
            assert!(md.contains(&format!("### {}", lore(e).name)), "{e:?}");
        }
    }

    impl Book {
        /// The book as it reads back (names cleaned as the save cleans them).
        fn clone_clean(&self) -> Book {
            let mut b = self.clone();
            for r in &mut b.rivals {
                r.name = clean(&r.name);
            }
            b
        }
    }
}
