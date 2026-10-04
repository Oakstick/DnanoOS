//! The old man's guide (B): what he tells the hero about the place he stands in. The tale or the
//! myth behind it, what he must do there, and what to watch for. He always leads to good: every
//! word is true of the game (see the tests).
//!
//! Pure data. The game adds what it knows now (the realm's general, keys held) under these.

use crate::logic::heroes::HeroKind;
use crate::logic::sins::Sin;

/// Where the hero stands, as the guide tells it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum GuidePlace {
    /// Dungeon I, the Garden of Krampus.
    Garden,
    /// Dungeon II, the Cisterns.
    Cisterns,
    /// Dungeon III, the Bone Vaults.
    BoneVaults,
    /// The Cyclops' meadow after dungeon `n` (0, 1, 2).
    Cyclops(usize),
    /// The Portals of Hell, the hub over the lava.
    Hub,
    /// A sin's realm.
    Realm(Sin),
    /// A sin's prince in his court.
    Court(Sin),
    /// The gym and its Sparring Ring.
    Gym,
    /// Level `n` (1 to 7) of the Pit under the House of the Devil.
    Pit(u8),
}

/// What he says of a place.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Guide {
    /// The place's name, as a heading.
    pub title: String,
    /// The tale, the myth or the sin behind it.
    pub tale: Vec<String>,
    /// What the hero must do here.
    pub task: Vec<String>,
    /// What to watch for: its foes, its ground, its tricks, and how to meet them.
    pub watch: Vec<String>,
}

/// The guide to `place`, in the old man's words to `hero`.
pub fn guide(place: GuidePlace, hero: HeroKind) -> Guide {
    let hail = hail(hero);
    match place {
        GuidePlace::Garden => page("THE GARDEN OF KRAMPUS", hail, GARDEN_TALE, GARDEN_TASK, GARDEN_WATCH),
        GuidePlace::Cisterns => page("THE CISTERNS", hail, CISTERNS_TALE, CISTERNS_TASK, CISTERNS_WATCH),
        GuidePlace::BoneVaults => page("THE BONE VAULTS", hail, BONES_TALE, BONES_TASK, BONES_WATCH),
        GuidePlace::Cyclops(n) => cyclops(n, hail),
        GuidePlace::Hub => page("THE PORTALS OF HELL", hail, HUB_TALE, HUB_TASK, HUB_WATCH),
        GuidePlace::Realm(sin) => realm(sin, hail),
        GuidePlace::Court(sin) => court(sin, hail),
        GuidePlace::Gym => page("THE GYM", hail, GYM_TALE, GYM_TASK, GYM_WATCH),
        GuidePlace::Pit(n) => pit(n, hail),
    }
}

/// How he greets each hero, once, at the head of the tale. The facts are the same for all three.
fn hail(hero: HeroKind) -> &'static str {
    match hero {
        HeroKind::Ragnar => "Ragnar",
        HeroKind::Mehmed => "Janissary",
        HeroKind::Miyamoto => "Swordsman",
    }
}

/// A page: the tale's first line opens with his greeting.
fn page<A: AsRef<str>, B: AsRef<str>, C: AsRef<str>>(title: &str, hail: &str, tale: &[A], task: &[B], watch: &[C]) -> Guide {
    let mut tale: Vec<String> = tale.iter().map(|l| l.as_ref().to_string()).collect();
    if let Some(first) = tale.first_mut() {
        *first = format!("{hail}, {first}");
    }
    Guide {
        title: title.to_string(),
        tale,
        task: task.iter().map(|l| l.as_ref().to_string()).collect(),
        watch: watch.iter().map(|l| l.as_ref().to_string()).collect(),
    }
}

// ------------------------------------------------------------------ the road down

const GARDEN_TALE: &[&str] = &[
    "in the Alpine valleys they tell of Krampus: horned and shaggy, the dark companion of St Nicholas, with chains, bells and birch switches.",
    "On Krampusnacht, the night of 5 December, he walks the villages, and a naughty child goes off in the sack or basket on his back.",
    "At dusk, at the graveyard, he took one of four boys. The other three ran after him through the gate and fell into my gateway.",
    "I gave them the souls of Ragnar, Mehmed and Miyamoto, and you carry one. Krampus is Satan's son: the road runs down to the House of the Devil.",
    "This garden is the first depth. His statue stands on the fountain at its heart.",
];

const GARDEN_TASK: &[&str] = &[
    "Find the eight golden keys hidden across the garden. Each one lights a lock on the gate in the north wall.",
    "With all eight the gate opens. Go through it: the Cyclops' meadow lies beyond, and the way down.",
    "A hidden shrine waits in the farthest dead end. Its relic unseals a sealed tier of one of my trees.",
];

const GARDEN_WATCH: &[&str] = &[
    "Draugr fall to pieces and pull themselves together again, and hunt by the sound of your steps. Ravens fly over the pits.",
    "Bog slimes leave a trail that slows you, but they cannot cross the bridges. The slain gather in the hollow and come out again.",
    "Every key angers the garden: more foes come out, and faster. Strike with E or a left click; leap the pits with Space or A.",
    "An archangel's shrine stands in each quarter. Strike one to change its angel, step on it for his blessing, and use it with R or LT.",
    "Raise your shield with Shift or LB, drink with Q or RB, and call me with H whenever you need. B brings you back to these words.",
];

const CISTERNS_TALE: &[&str] = &[
    "under the old cities men cut cisterns to hold water against siege and drought: forests of columns standing in the dark.",
    "Beneath Constantinople lies the greatest, the Sunken Palace. Two of its columns rest on heads of Medusa, one sideways, one upside down.",
    "Here, a depth further down, the water is long gone and the dead keep the halls. This road lies off the main way, but it still goes down.",
];

const CISTERNS_TASK: &[&str] = &[
    "As in the garden: find the eight golden keys, and the gate in the north wall opens.",
    "A Cyclops waits in the meadow beyond. Bring it down and take its red diamond key, and the road goes on to the Bone Vaults.",
    "The hidden shrine waits in the farthest dead end, as in every dungeon. Its relic unseals a sealed tier.",
];

const CISTERNS_WATCH: &[&str] = &[
    "Helmeted skeletons wear iron caps that ring off your first blow. The blemmyes, a headless brute, takes three blows and hits twice as hard.",
    "The loup-garou is fast, and faster once wounded. Large bats swerve and swoop at where you are going, not where you are.",
    "A witch keeps her distance and conjures minions, each of which falls to one blow. Go after the witch herself.",
    "A foe of the garden comes out with them. The foes here are quicker than in the garden, and every key angers the place.",
];

const BONES_TALE: &[&str] = &[
    "when the graveyards grew too full, the old bones were dug up and laid in vaults: skulls in rows, long bones stacked like firewood.",
    "So it was in the catacombs under Paris, and in the charnel house of Hallstatt, where the skulls are painted with their names.",
    "Here the dead do not rest in their rows. This road lies off the main way, and the deeper it goes, the older the things that walk it.",
];

const BONES_TASK: &[&str] = &[
    "Find the eight golden keys, and the gate in the north wall opens on the next Cyclops' meadow.",
    "Bring that Cyclops down and take its key. After my wares the road goes on, deeper still.",
    "The hidden shrine waits in the farthest dead end. Its relic unseals a sealed tier.",
];

const BONES_WATCH: &[&str] = &[
    "The tengu is a goblin swordsman, quick on his feet. The rasetsu, a horned man-eater with a blade in each fist, takes three blows.",
    "The dire wolf circles, then bites. The cobra rears and strikes, and its bite hurts twice over. Each takes two blows.",
    "The jorogumo, a spider-woman, keeps her distance and hurls fire. Close on her, or she will keep backing away.",
    "Foes of the depths above come out with them. This is the fastest dungeon yet, and every key angers it.",
];

const CYCLOPS_TALE: &[&str] = &[
    "Hesiod sang of three Cyclopes, sons of Earth and Sky, who forged the thunderbolt of Zeus. Each had one round eye in his brow.",
    "Homer's Cyclops was Polyphemus, a shepherd giant. He shut Odysseus and his men in his cave, and ate them two at a time.",
    "Odysseus gave him strong wine and said his name was Nobody. When the giant slept, they drove a burning stake into his eye.",
    "Polyphemus cried that Nobody was killing him, so no one came. This one is a floating eye the size of a hut, and it guards the way down.",
];

/// The Cyclops' meadow after dungeon `n`.
fn cyclops(n: usize, hail: &str) -> Guide {
    let title = if n == 0 { "THE CYCLOPS' MEADOW".to_string() } else { format!("THE CYCLOPS' MEADOW {}", n + 1) };
    let next = match n {
        0 => "From there the road goes down to the Portals of Hell, where the seven gates wait.",
        1 => "From there the road goes on to the Bone Vaults.",
        _ => "From there the road goes on, deeper still.",
    };
    let task = [
        "Bring the Cyclops down. When it falls it drops a red diamond key on the grass.",
        "Walk over the key, and the gate in the north hedge opens. Go through, and I will meet you with my wares.",
        next,
    ];
    let hide = if n == 0 {
        "This first one is the weakest. The archangels' powers (R or LT) wound it exactly, dazed or not."
    } else {
        "From this depth on its hide turns a point of every blow, unless your blade cuts through armour. Each Cyclops is tougher than the last."
    };
    let watch = [
        "When its eye squints and glows it is about to throw: fans and rings of bolts, a stream that follows you, boulders that roll.",
        "Keep the pillars and the low walls between you. Raise your shield with Shift or LB, but you walk slowly and cannot strike behind it.",
        "Wounded, it grows faster and starts to dive at you. A dive that misses leaves it dazed on the grass: then every blow counts double.",
        "Its touch hurts, except while it lies dazed or rises again. Close in when it is down, and get clear before it is up.",
        hide,
    ];
    page(&title, hail, CYCLOPS_TALE, &task, &watch)
}

// ------------------------------------------------------------------ the Portals of Hell

const HUB_TALE: &[&str] = &[
    "these are the Portals of Hell: eight platforms over a sea of lava, round the volcano where the House of the Devil stands.",
    "Seven gates, one for each deadly sin, stand round it in the order of Dante's terraces of Purgatory, from Pride to Lust.",
    "In 1589 Peter Binsfeld gave each sin a prince. Wrath he gave to Satan himself; here Amon, who stirs up feuds, keeps it for him.",
    "Pride is Lucifer's, envy Leviathan's, wrath Amon's, sloth Belphegor's, greed Mammon's, gluttony Beelzebub's, lust Asmodeus's.",
    "Each prince keeps his sin's key. Krampus waits in the House with his father, behind the seal of the seven.",
];

const HUB_TASK: &[&str] = &[
    "Choose any gate and go through. Each realm ends in its prince's court, and the prince drops his sin's key.",
    "A realm you have taken keeps its doors shut, its key in the lock. Bring all seven keys, and the seal of the seven breaks.",
    "Then causeways rise from the lava to the House of the Devil. Its doors do not open yet, but that is where the road ends.",
];

const HUB_WATCH: &[&str] = &[
    "No foe walks these platforms. Only the wooden bridges cross the lava. Walk up to a gate and its doors open on its realm.",
    "Every realm you take makes the next one harder, and its packs bring the beasts of one more depth. Choose your order with care.",
    "In the realms the princes lie, in their own voices and colours. My words are amber, and Quitmer barks with them.",
    "If a whisper comes in another colour, press H and I will tell you the truth. Stand still in a realm, and my amber lamp shows the far portal.",
];

// ------------------------------------------------------------------ the realms

/// Every realm has its prince's general (the game names him under these words).
const GENERAL: &str =
    "A general runs this realm by orders, each through an instrument: three blows break one. Slay the general, and his prince is weaker.";

/// The realm of `sin`.
fn realm(sin: Sin, hail: &str) -> Guide {
    let (tale, task, watch): (&[&str], [&str; 3], &[&str]) = match sin {
        Sin::Pride => (PRIDE_TALE, PRIDE_TASK, PRIDE_WATCH),
        Sin::Envy => (ENVY_TALE, ENVY_TASK, ENVY_WATCH),
        Sin::Wrath => (WRATH_TALE, WRATH_TASK, WRATH_WATCH),
        Sin::Sloth => (SLOTH_TALE, SLOTH_TASK, SLOTH_WATCH),
        Sin::Greed => (GREED_TALE, GREED_TASK, GREED_WATCH),
        Sin::Gluttony => (GLUTTONY_TALE, GLUTTONY_TASK, GLUTTONY_WATCH),
        Sin::Lust => (LUST_TALE, LUST_TASK, LUST_WATCH),
    };
    let task: Vec<&str> = task.into_iter().chain([GENERAL]).collect();
    page(&sin.realm().to_uppercase(), hail, tale, &task, watch)
}

const PRIDE_TALE: &[&str] = &[
    "pride is the first sin and the root of the rest: to set yourself above all. On Dante's first terrace the proud bend under stones.",
    "Isaiah tells of Lucifer, son of the morning, who said: I will exalt my throne. And then: thou shalt be brought down to the pit.",
    "Binsfeld made Lucifer the prince of pride. Andersen told of a devil's mirror that shrank the good and swelled the bad, and shattered.",
    "This spire is built of its shards. Its tengu were proud monks once: in Japan, to grow conceited is to become a tengu.",
];

const PRIDE_TASK: [&str; 3] = [
    "Climb the seven galleries through the mirror-doors. A true mirror climbs one gallery; a vain one loops you one or two down.",
    "At the top the Morning Star says: \"Take your throne.\" That is a lie. Step onto the dais, and your reflection rises: the Peerless.",
    "Shatter it, and the floor gives way: you fall the spire's length to the court's portal, open now, and Lucifer waits beyond.",
];

const PRIDE_WATCH: &[&str] = &[
    "A true mirror shows you as you are, cracked and as hurt as you are. A vain one shows you crowned and whole. Trust your wounds.",
    "Glory: clear a gallery unhurt and your blows bite harder. At three stacks the beams track you; at five the vain mirrors fake cracks.",
    "A hit or a drink humbles you and ends your Glory. Leap the beams: one that burns you lodges a splinter, and the mirrors swap faces.",
    "The Peerless repeats your moves a moment later, mirrored, and grows with your Glory. Throw one blow, then guard: parry its echo.",
    "Strike only while it reels; a drink it cannot copy leaves it posing, open. Each blow it lands drops a ring of floor. At three, you fall.",
];

const ENVY_TALE: &[&str] = &[
    "envy is sorrow at another's good. On Dante's second terrace the envious sit with their eyes sewn shut, who once looked with spite.",
    "Its prince is Leviathan, the crooked serpent, the dragon that is in the sea. His coils thread this flooded cave above and below.",
    "In Anatolia envy strikes through the eye, and a blue nazar bead takes the blow and cracks. That Eye opens in this water.",
    "At Dōjō-ji, jealous Kiyohime became a serpent, coiled round the temple bell and burned the man hiding inside. That bell hangs here.",
];

const ENVY_TASK: [&str; 3] = [
    "Cross between the Waterline and the Drowned Floor: deep pools only go down, geyser vents only go up. Four crossings reach the bell.",
    "At the bell the water whispers: \"All you want lies below.\" It is a lie. Dive, and the bell drops over you while the serpent heats it.",
    "Strike the white-hot patch where his head presses, three times with the last blow of your string. The bell lifts one second: dive then.",
];

const ENVY_WATCH: &[&str] = &[
    "The other side always looks richer. Every pool off the true way is a lure, under treasure the water shows that is not there.",
    "A dive lands about a tile and a half beyond where its twin shows through the water. A geyser holds you four seconds, open to blows.",
    "The Eye opens every twenty seconds. Seen, your next wound bites deeper, and what you drink or call on is stolen until you kill the thief.",
    "A nazar bead on your belt takes one gaze whole, then cracks: gather them. The serpent's humps are bridges only while they pass.",
    "Under the bell the safe water shrinks and the rim scorches. Leap the steam as it bursts, and trust your blade alone in there.",
];

const WRATH_TALE: &[&str] = &[
    "wrath is the hunger for vengeance, the fire that wants to strike. On Dante's third terrace the wrathful walk blind in black smoke.",
    "Binsfeld gave wrath to Satan himself; here Amon keeps it for him, a wolf with a serpent's tail breathing fire, who stirs up feuds.",
    "The Einherjar of Valhalla fight all day and rise each evening to fight again. On this field the slain rise at every flash of lightning.",
    "The berserkers' rage waits for you here too. It does not end battles; it feeds them.",
];

const WRATH_TASK: [&str; 3] = [
    "Cross the drifting slabs to the herald's isle. A raven-headed herald says: \"Sound the horn and the battle ends.\" It is a lie.",
    "Strike the horn and it calls the whole field: five slabs rise toward the hill, and the last begins to sink. Cross within sixty seconds.",
    "On the hill stands the truce stone, the true gate. Stand by it five seconds without one swing, and the far portal opens.",
];

const WRATH_WATCH: &[&str] = &[
    "The slabs drift in three bands, each at its own pace, and a gap can widen while you leap. Watch them move before you jump.",
    "Fury: every blow you land feeds it. At ten stacks the berserk rage takes your hands six seconds, with no guard. Two quiet seconds cool it.",
    "Every blow, yours or theirs, cracks the slab it lands on. At the third crack it splits, and the smaller piece sinks.",
    "The slain rise at the next flash unless they went into the lava. A foe whose poise breaks within a tile of an edge topples in for good.",
    "The lava costs a third of your health. Foes will taunt you across the gaps: do not leap after them. Parry, topple, move.",
];

const SLOTH_TALE: &[&str] = &[
    "sloth is more than rest: it is the weary heart that will not do the good it should. On Dante's fourth terrace the slothful run unresting.",
    "Belphegor is its prince: he tempts with clever inventions and riches without labour. His shortcuts lie all over this mire.",
    "Thor's men once slept in a hall that was a giant's glove, and the night's quakes were Skrymir's snores. This bog is a sleeping giant.",
    "Hel's bed is Kör, the sickbed, and her servants are Ganglati and Ganglöt, the slow-walkers. They wade here.",
    "I know this sin. I slept the night Krampus slipped past my gate.",
];

const SLOTH_TASK: [&str; 3] = [
    "Reach the Stopped Hourglass on the giant's palm. Stand still a moment and his fingers rise toward you across the moat; hurry and they sink.",
    "At its foot Hel's bed says: \"Lie down. The door opens when you wake.\" It is a lie: you wake at the mire's edge, three hundred years older.",
    "Stand instead. The hourglass turns: hold the island sixty seconds, and when the sand runs out the giant's hand opens the door.",
];

const SLOTH_WATCH: &[&str] = &[
    "Torpor fills while you stand still, wade or lie on moss, and drains as you move on firm ground; each kill shakes it off. Full, you sleep.",
    "Moss heals, but every ten seconds rested the mire grows older: the tide rises, a plank rots, a servant climbs out of the bog.",
    "Marsh-lights hover over the bog holes the duckweed hides. A loud fight by the giant's head turns the land a quarter turn round the palm.",
    "Belphegor's cart, ferry and lift carry you beautifully, halfway, then want twenty seconds of standing still to wind.",
    "By the bed a yawn comes every five seconds: leap its crest, or guard and be pushed one tile, not three. Keep moving, keep killing.",
];

const GREED_TALE: &[&str] = &[
    "greed is the love of having more than you need. On Dante's fifth terrace the greedy lie face down, eyes on the earth they loved.",
    "Mammon means wealth. No man can serve God and mammon, said Christ, and Binsfeld made Mammon the prince of greed.",
    "Milton's Mammon walked heaven with his eyes on its golden pavement, and in hell he led the digging for gold.",
    "These are his vaults: rooms of gold and marble, every one of them heaped with coin.",
];

const GREED_TASK: [&str; 3] = [
    "Cross the vaults from the west antechamber to the far portal in the east wall. Take what gold and chests you like on the way.",
    "At the far portal Mammon asks his toll: a chance, he says, to keep the gold. It is a lie. Keep it, and the vaults close and begin again.",
    "Leave the gold, and the portal opens on his court. What his chests gave you, you keep.",
];

const GREED_WATCH: &[&str] = &[
    "The packs sleep in their rooms and wake as you come near. They hunt you while you stay close, and go home when you are long gone.",
    "Every foe fights by its frames: watch its wind-up, raise your guard as the blow lands to parry it, then strike while it reels.",
    "Your shield wears as it blocks and can break for good, and your blade dulls with every blow. Call me with H to mend and sharpen.",
    "A chest that breathes, whose coins never glint and whose hinge creaks, is a mimic. Strike it first, and it pays double.",
    "A whisper without Quitmer's bark is Mammon's, never mine: press H and I will set it right. Stand still, and my amber lamp shows the way.",
];

const GLUTTONY_TALE: &[&str] = &[
    "gluttony is to take more than you need and never be filled. On Dante's sixth terrace the gluttons starve beneath fruit they cannot reach.",
    "Its prince is Beelzebub: Baal-Zebub, the lord of the flies. His swarm drifts through this palace.",
    "In Japan the gaki are hungry ghosts with swollen bellies and needle throats, whose food turns to fire. They hunt whoever smells of food.",
    "Hansel and Gretel found a house of bread, built as bait. This banquet hall is the same: it is a gullet, and it runs one way.",
];

const GLUTTONY_TASK: [&str; 3] = [
    "Go down the gullet: the Hall of Courses, the Kitchens, the Larder, the Stomach. Each archway closes behind you.",
    "At the Stomach's heart a dome says: \"Finish your plate, and you may leave.\" It is a lie: the plate is empty, and the meal is you.",
    "The Stomach closes in over forty-five seconds. Its ring of teeth opens two seconds in every ten, and only for 3 Bloat or less.",
];

const GLUTTONY_WATCH: &[&str] = &[
    "Here you starve: hunger eats your greatest health. Food you walk over is eaten, and each bite heals but adds a stack of Bloat.",
    "Bloat slows you. At 3 the gaki smell you, at 6 the narrow arches bar you, at 8 the Kitchens' crust floors break under you.",
    "Leap over food you do not want: in the air you eat nothing. Every last blow of your string that lands burns a stack of Bloat.",
    "A purge font takes five stacks and twenty health, and stuns you three seconds. The floors drift you on, faster in every zone.",
    "In the last course the gut hurls food at you: parry it into the gaki. If the walls catch you, you wake in the Hall, hungry.",
];

const LUST_TALE: &[&str] = &[
    "lust is love bent back on the self, desire that uses another. On Dante's seventh terrace the lustful walk through fire to be made clean.",
    "Its prince is Asmodeus. In the Book of Tobit he slew seven bridegrooms of Sarah, each on his wedding night.",
    "Tobias drove him off with the smoke of a fish's heart and liver, and the archangel Raphael bound him. The fish's gall cured blindness.",
    "The Danish elf-maid is lovely from the front and hollow behind. Everything lovely here is a facade, the Bride most of all.",
];

const LUST_TASK: [&str; 3] = [
    "Walk to the Black Thorn, the one dead tree that never moves; the court's gate stands at its foot. Walk toward what you do not want.",
    "At the Thorn the gate grinds open for ten seconds, and the Bride walks up behind you: \"Turn around.\" It is a lie, and her embrace.",
    "Keep your back to her until the gate is open. Or leap through her, land behind her, and strike her hollow back three times.",
];

const LUST_WATCH: &[&str] = &[
    "Charm fills while you face the Bride or stand in perfume mist. Full, you are enthralled three seconds: your controls turn, and she pulls.",
    "The rose paths heal you, but every one points at her. The thorn paths bite and slow you, and turn your back on her: they are honest.",
    "Stand two seconds in a koi pond to catch a fish, and burn it at a censer: ten seconds of smoke that blocks charm and lights the true paths.",
    "The elf-maids cannot be hurt from the front: strike their hollow backs. A ring of dancing elf-maids drags in whoever comes near.",
    "If she takes you, you are her eighth groom: a heavy wound, doom, and you wake in the innermost bower. Raphael's beam burns through hedges.",
];

// ------------------------------------------------------------------ the courts

/// The prince of `sin` in his court.
fn court(sin: Sin, hail: &str) -> Guide {
    if sin == Sin::Greed {
        return page("THE COUNTING HOUSE", hail, MAMMON_TALE, MAMMON_TASK, MAMMON_WATCH);
    }
    // Pride's court, as built (its page to come here).
    const PRIDE_COURT_TALE: &[&str] = &[
        "this is the Throne of the Morning Star. Isaiah sang it: how art thou fallen from heaven, O Lucifer, son of the morning!",
        "He was the light-bearer, the most beautiful of the angels, and would set his throne above the stars. It stands empty behind him still.",
        "He fights you in three moods: the Morning Star, the Mirror, and the Fallen, his six wings burnt black.",
    ];
    const PRIDE_COURT_TASK: &[&str] = &[
        "Bring Lucifer down. He drops the Key of Pride: take it, and the gate in the south wall opens.",
        "Through it you come to me, and then back to the Portals of Hell, where Pride's door shuts with its key in the lock.",
        "Each new mood is named as it comes, and a green cross falls for a breather. If you fall, he stands whole again at the foot of his steps.",
    ];
    const PRIDE_COURT_WATCH: &[&str] = &[
        "His light turns most of a blow. But pride never looks behind him: strike his back, or strike in his recoveries, and the whole blow lands.",
        "His Lance of Dawn burns along a line on the floor. A shield holds it, no parry turns it: when the line stops following you, step off it.",
        "The Mirror comes out of the glass nearest you and stands long in the open. His two silver Reflections break at one blow; he has the gold halo.",
        "The Fallen calls stars down round you: step off them, or leap. He comes down on a mark that follows you: walk out of it, strike as he lands.",
        "If his Chamberlain of Glass fell, no Reflections come; if the throne-mirror was polished, they stand at his side from the start.",
    ];
    if sin == Sin::Pride {
        return page("THE THRONE OF THE MORNING STAR", hail, PRIDE_COURT_TALE, PRIDE_COURT_TASK, PRIDE_COURT_WATCH);
    }

    // Envy's court, as built (its page to come here).
    const LEVIATHAN_TALE: &[&str] = &[
        "this is the Drowned Bell, Leviathan's court: a temple sunk under the sea of envy, a great bronze bell hanging in its middle.",
        "Leviathan is Isaiah's crooked serpent, the dragon that is in the sea. Envy is his: he begrudges every creature what it has.",
        "In Japan they tell of Kiyohime, who became a serpent for jealousy and coiled round the bell of Dōjō-ji till it burned. This is that bell.",
    ];
    const LEVIATHAN_TASK: &[&str] = &[
        "Bring Leviathan down. He drops the Key of Envy: take it, and the gate in the south wall opens.",
        "Through it you come to me, and then back to the Portals of Hell, where Envy's door shuts with its key in the lock.",
        "Each new mood is named as it comes, and a green cross falls for a breather. If you fall, he rises whole again before his altar.",
    ];
    const LEVIATHAN_WATCH: &[&str] = &[
        "His scales turn most of a blow: strike in his recoveries, or when a parry leaves him reeling. The water is shallow and never slows you.",
        "The Crooked Serpent lashes wide, spits brine in threes, and dives: a ring follows you on the floor. Walk out of it; he rises there, open.",
        "The Evil Eye covets the archangel you called on last, in green: fire, thunder, a beam or a wave. Read it as your own. Two serpentlings rise, once.",
        "The Bell coils round the bronze, and its heat burns his scales away on that side: strike him from the bell's side. Leave the pale ring when he tolls.",
        "If the Bell-Warden fell in the grotto, no serpentlings come; if the bell was lowered by decree, they come at once.",
    ];
    if sin == Sin::Envy {
        return page("THE DROWNED BELL", hail, LEVIATHAN_TALE, LEVIATHAN_TASK, LEVIATHAN_WATCH);
    }

    // Wrath's court, as built (its page to come here).
    const AMON_TALE: &[&str] = &[
        "this is the Anvil of Wrath: black basalt cracked over fire, the weapons of the war's dead stuck where they fell, banners burning.",
        "The old books call Amon a marquis of Hell: a wolf with a serpent's tail who vomits flame, and at a word a man with a raven's head.",
        "Here he is both, and a great axe besides. He fights in three moods: the Wolf, the Raven, the Fury. Rage is all he has, and rage can be turned.",
    ];
    const AMON_TASK: &[&str] = &[
        "Bring Amon down. He drops the Key of Wrath: take it, and the gate in the south wall opens.",
        "Through it you come to me, and then back to the Portals of Hell, where Wrath's door shuts with its key in the lock.",
        "Each new mood is named as it comes, and a green cross falls for a breather. If you fall, he stands whole again before his anvil.",
    ];
    const AMON_WATCH: &[&str] = &[
        "He has no hide: every blow lands whole. But every blow you land makes him angrier and quicker, five steps of rage at most.",
        "He lowers his head to charge, and the line of his run shows on the floor. When it stops following you, step aside or leap: he runs straight on.",
        "Four broken pillars stand in the ring. Charged into stone, or the wall, or his anvil, he stands dazed, his rage gone out of him: strike then.",
        "The Raven splits the floor toward you: the seam glows, then bursts into lava that burns a while. Step off it, and stay off it.",
        "At two thirds his ravens come, once: none if Marshal Gorebrand fell, at once if his bridges burnt. The Fury charges twice; the floor erupts.",
    ];
    if sin == Sin::Wrath {
        return page("THE ANVIL OF WRATH", hail, AMON_TALE, AMON_TASK, AMON_WATCH);
    }

    // Sloth's court, as built (its page to come here).
    const BELPHEGOR_TALE: &[&str] = &[
        "this is Belphegor's Hall of Stopped Clocks: a grey bedchamber the size of a hall, fog on its floor, every clock stopped at the same hour.",
        "He was the Baal of Peor, and Binsfeld made him the prince of sloth. He gives men clever inventions and riches without labour, and they never wake.",
        "He sleeps in his great chair behind the giant hourglass, and fights in three moods: the Sleeper, the Dreamer, the Waking.",
    ];
    const BELPHEGOR_TASK: &[&str] = &[
        "Bring Belphegor down. He drops the Key of Sloth: take it, and the gate in the south wall opens.",
        "Through it you come to me, and then back to the Portals of Hell, where Sloth's door shuts with its key in the lock.",
        "Each new mood is named as it comes, and a green cross falls for a breather. If you fall, he sleeps whole again in his chair.",
    ];
    const BELPHEGOR_WATCH: &[&str] = &[
        "Asleep his hide is open: your first blow lands whole and wakes him. Awake it turns most of a blow, except in his recoveries.",
        "The Sleeper yawns you back (leap it, or parry it and stand fast), swats wide with his great hand, and lets sand drift down to burst: step off it.",
        "At two thirds he lies down to dream and two nightmares come. Asleep he mends: do not stand back and wait. Strike him hard, or finish a string, and he wakes.",
        "At one third he stands for good, slow but vast: the hourglass slams the floor round him, the clock's hands sweep all round from far. Step back, then punish.",
        "If his general, the Lamplighter, fell, no nightmares come. If the hourglass was wound, he wakes rested, with no free blow, and his nightmares come at once.",
    ];
    if sin == Sin::Sloth {
        return page("THE HALL OF STOPPED CLOCKS", hail, BELPHEGOR_TALE, BELPHEGOR_TASK, BELPHEGOR_WATCH);
    }

    // Gluttony's court, as built (its page to come here).
    const BEELZEBUB_TALE: &[&str] = &[
        "this is the Banquet of Flies: the feast laid round his hall in a U, and all of it gone grey with mould.",
        "Beelzebub, Baal-Zebub, the lord of the flies: a bloated lord with a fly's head, a cleaver in his hand. He is never full.",
        "Three moods: the Lord of Flies, the Swarm, and the Glutton. Every one of them is hungry.",
    ];
    const BEELZEBUB_TASK: &[&str] = &[
        "Bring Beelzebub down. He drops the Key of Gluttony: take it, and the gate in the south wall opens to me.",
        "Hurt, he goes to his table to eat, and it mends him. Strike him while he eats: he chokes, open, and the platter spoils.",
        "Take the green crosses yourself, and soon: he eats one left lying on the floor. If you fall, he and his table stand whole again.",
    ];
    const BEELZEBUB_WATCH: &[&str] = &[
        "His bloat turns most of a blow. Strike in his recoveries, when a parry leaves him reeling, or while he chokes.",
        "The Lord of Flies chops with his cleaver and flings slow flies in a fan: block them, or parry them back.",
        "The Swarm bursts into flies and falls on a ring that follows you: walk out of it at once. Two swarms rise from the food.",
        "The Glutton rolls at you in a line, spews bile that burns where it lies, and slams his belly: step aside, step off.",
        "If his steward fell in the feast, no swarms come; if the last course was laid, six platters wait and the swarms come at once.",
    ];
    if sin == Sin::Gluttony {
        return page("THE BANQUET OF FLIES", hail, BEELZEBUB_TALE, BEELZEBUB_TASK, BEELZEBUB_WATCH);
    }

    // Lust's court, as built (its page to come here).
    const ASMODEUS_TALE: &[&str] = &[
        "this is the Rose Bower: crimson roses on black thorn, a fountain, and the bridal arch where Asmodeus waits. Seven grooms died under it.",
        "Three heads he has, a smiling man's, a bull's and a ram's, and a serpent's tail. His moods: the Suitor, the Three Heads, the Bridegroom of Ashes.",
        "Tobias drove him off with smoke on the embers of incense, as Raphael taught him. The four censers here are cold. Light them.",
    ];
    const ASMODEUS_TASK: &[&str] = &[
        "Bring Asmodeus down. He drops the Key of Lust: take it, and the gate in the south hedge opens. Through it you come to me, then to the Portals.",
        "Strike a cold censer and it smokes ten seconds. While any smokes his allure is gone, every blow lands whole, and he will not come near the smoke.",
        "Each censer lights twice. Each new mood is named as it comes, a green cross falls for a breather, and if you fall he stands whole again.",
    ];
    const ASMODEUS_WATCH: &[&str] = &[
        "Outside the smoke his allure turns most of a blow. Strike in his recoveries, or when a parry leaves him reeling.",
        "His blown kiss is a pink heart no shield stops. Leap it or cut it from the air: if it touches you, you are smitten and cannot guard a while.",
        "His lance reaches far but narrow: step aside. Thorns burst from the ground where they glow, in a row toward you or a ring round him: step out, or leap.",
        "At two thirds two elf-maids step from the hedges, once: only their hollow backs can be hurt. The bull's head charges straight; the ram's shoves.",
        "If the Bridesmaid fell in the garden, no maids come. If her vows were spoken, they come at once, and each censer lights only once.",
    ];
    if sin == Sin::Lust {
        return page("THE ROSE BOWER", hail, ASMODEUS_TALE, ASMODEUS_TASK, ASMODEUS_WATCH);
    }

    let (who, colour) = match sin {
        Sin::Pride => {
            ("Lucifer fell from the morning star to the foot of the pit, and here at the foot of his spire he keeps his court.", "violet")
        }
        Sin::Envy => (
            "Leviathan, the serpent of the deep, begrudges every creature its place. You came to him through the water under the bell.",
            "green",
        ),
        Sin::Wrath => {
            ("Amon, who stirs up feuds, watched you cross his whole field. Beyond the truce stone he waits, and he is not at peace.", "red")
        }
        Sin::Sloth => ("Belphegor, who promised you rest, is awake. Beyond the hourglass's door he keeps his court.", "slate blue"),
        Sin::Gluttony => ("Beelzebub, the lord of the flies, waits past the ring of teeth, and he is always hungry.", "orange"),
        _ => ("Asmodeus, who slew seven grooms, waits beyond the Black Thorn. Raphael bound him once; he can be beaten.", "rose"),
    };
    let tale = [
        who.to_string(),
        format!("For now he takes the shape of a great one-eyed beast, like the Cyclops, burning in {}'s {colour}.", sin.name()),
        "Fight him as you fought the Cyclops: its tricks are his.".to_string(),
    ];
    let task = [
        format!("Bring {} down. He drops the Key of {}: take it, and the gate in the north hedge opens.", sin.prince(), sin.name()),
        "Through it you come to me, and then back to the Portals of Hell, where his door shuts with its key in the lock.".to_string(),
        "If his general fell in the realm he comes to you weaker; if the general's decree reached the court, stronger.".to_string(),
    ];
    let watch = [
        "When his eye squints and glows he is about to throw: fans and rings of bolts, a stream that follows you, boulders that roll.",
        "Keep the pillars and the low walls between you. Wounded, he grows faster and starts to dive at you.",
        "A dive that misses leaves him dazed on the grass: close in then, for every blow counts double. His touch hurts, except while dazed.",
        "His hide turns a point of every blow, unless your blade cuts through armour. The archangels' powers (R or LT) wound him exactly.",
    ];
    page(&format!("THE COURT OF {}", sin.prince().to_uppercase()), hail, &tale, &task, &watch)
}

const MAMMON_TALE: &[&str] = &[
    "this is Mammon's Counting House: gold-veined marble, heaps of coin, and his throne beneath the great brass scales.",
    "He weighs every soul by what it holds. Here he fights you himself, a demon-king of gold, in three moods: the Banker, the Collector, the Hoard.",
    "You left his gold at the door. Now take his key.",
];

const MAMMON_TASK: &[&str] = &[
    "Bring Mammon down. He drops the Key of Greed: take it, and the gate in the south wall opens.",
    "Through it you come to me, and then back to the Portals of Hell, where Greed's door shuts with its key in the lock.",
    "Each new mood is named as it comes, and a green cross falls for a breather. If you fall, he stands whole again before his throne.",
];

const MAMMON_WATCH: &[&str] = &[
    "His gold hide turns most of a blow. Strike in his recoveries, or when a parry leaves him reeling: then the whole blow lands.",
    "The Banker tosses coins that lie on the floor and burst. No shield stops them: step off them, or leap. His sceptre sweeps wide.",
    "The Collector's grasp can only be parried. If his fist closes, it holds your feet and drains your silver: strike him at once to break it.",
    "At two thirds two gold golems climb from his heaps, once. The Hoard rains coins, slams the floor and throws coins in fives, and is quicker.",
    "If his general fell in the vaults, no golems come; if the treasurers' decree reached the court, they come at once.",
];

// ------------------------------------------------------------------ the gym

const GYM_TALE: &[&str] = &[
    "every warrior should train before the road. This is my yard over the lava: one of every foe, stage by stage, each on its platform.",
    "The slain pull themselves together at their posts, so the yard never empties. West of where you arrive lies the Sparring Ring.",
    "In the ring a fight is counted in frames: every blow has its wind-up, its moment and its recovery. Learn them, and you learn every foe.",
];

const GYM_TASK: &[&str] = &[
    "Step into the ring and a bout begins: first the Brute, the Skirmisher and the Hexer, then each later stage's foes, and last the princes.",
    "Lose, and the bout begins again with you on your feet. Win, and the next one comes. Nothing here touches your real shield, lives or gold.",
    "F9 skips to the next bout, F7 to the next stage's bouts, and F8 shows the frame readout.",
];

const GYM_WATCH: &[&str] = &[
    "Your string is three blows on E or a left click, the third the heaviest. A press during a swing comes out as the next blow.",
    "Guard with Shift or LB: in its first frames it parries and the foe reels; after that it blocks for stamina, until the stamina runs out.",
    "The leap (Space or A) leaves you untouchable in the air. A foe glows as it winds up, flashes as it strikes, and dims in recovery: punish then.",
    "The Brute swings slow: wait, then punish. The Skirmisher bites from beside you: parry it. The Hexer keeps away: close in, or parry its bolt.",
    "A red pip marks a foe that holds an attack token. The rest wait their turn on a ring round you.",
];

// ------------------------------------------------------------------ the Pit

const PIT_TALE: &[&str] = &[
    "under the House of the Devil there is no throne and no devil. There is only this: a prison that goes down, one stone cell for every level.",
    "In each cell's floor a square hole, and in the hole a stone platform with a long table on it. Those above eat first; each level below finds less.",
    "Krampus took the boy to the very bottom. Seven levels lie between you and him, and the platform is the only way down.",
];

const PIT_TASK: &[&str] = &[
    "Every level lasts two minutes; the clock at the top counts them down. At zero the platform sinks to the next level, with you on it.",
    "Every foe standing on the platform at zero rides down with you and joins the next level's. What stands on the ledge stays behind.",
    "In the last fifteen seconds the bell tolls and the platform's iron glows: clear the platform, and be on it when it goes.",
    "If you are on the ledge at zero, you leap after it and land hurt, a tenth of your health. A green cross waits on the platform as each level begins.",
];

/// What each level holds: its stage's foes, and the one thing to know of each.
const PIT_FOES: [&str; 7] = [
    "This level holds the restless dead of the garden: draugr, bats, vampires and bog slimes, every one of them fought blow for blow.",
    "This level holds the witch's brood: helmeted draugr whose caps ring off your first blow, large bats, the witch, the loup-garou and the blemmyes.",
    "This level holds the mountain's demons: the quick tengu, the rasetsu with a blade in each fist, the circling dire wolf, the jorogumo's fire, the cobra.",
    "This level holds the yokai: the oni whose club shakes the ground, the kappa (parry it and it bows, spilling its dish), the rokurokubi, the kitsune.",
    "This level holds the Anatolian night: the karakoncolos who roars your name, the silent tülükuş, the albastı, the cin who vanishes, the arçura.",
    "This level holds the legends: Tepegöz (only his eye can be hurt), the gashadokuro, the ejder (never give it the second blow it begs for), the goryō.",
    "This level holds Ragnarok: the frost giant's frostbite, Surtr's sword that leaves the floor burning, Nidhogg's venom, and Hel, who raises the dead.",
];

const PIT_WATCH: &[&str] = &[
    "They come out of the iron doors in the walls, pack by pack. No more than ten stand here at once; the rest wait behind the doors until there is room.",
    "The doors shut with twenty seconds left, and what is still behind them stays there. Fall, and the level begins again with what rode down with you.",
];

/// Level `n` of the Pit: its tale and task are the same at every level; what it holds is its own.
fn pit(n: u8, hail: &str) -> Guide {
    let n = n.clamp(1, 7);
    let mut watch = vec![PIT_FOES[(n - 1) as usize]];
    watch.extend_from_slice(PIT_WATCH);
    page(&format!("THE PIT: LEVEL {n}"), hail, PIT_TALE, PIT_TASK, &watch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::sins::SINS;

    const HEROES: [HeroKind; 3] = [HeroKind::Ragnar, HeroKind::Mehmed, HeroKind::Miyamoto];

    /// The letters the page's font can draw besides ASCII.
    const EXTRA: [char; 12] = ['Ç', 'ç', 'Ö', 'ö', 'Ü', 'ü', 'Ş', 'ş', 'İ', 'ı', 'Ō', 'ō'];

    fn places() -> Vec<GuidePlace> {
        let mut out = vec![GuidePlace::Garden, GuidePlace::Cisterns, GuidePlace::BoneVaults, GuidePlace::Hub, GuidePlace::Gym];
        out.extend((0..3).map(GuidePlace::Cyclops));
        out.extend(SINS.iter().map(|&s| GuidePlace::Realm(s)));
        out.extend(SINS.iter().map(|&s| GuidePlace::Court(s)));
        out.extend((1..=7).map(GuidePlace::Pit));
        out
    }

    fn lines(g: &Guide) -> impl Iterator<Item = &String> {
        std::iter::once(&g.title).chain(g.tale.iter()).chain(g.task.iter()).chain(g.watch.iter())
    }

    #[test]
    fn every_place_has_a_title_and_two_to_five_lines_in_each_section_for_every_hero() {
        for place in places() {
            for hero in HEROES {
                let g = guide(place, hero);
                assert!(!g.title.trim().is_empty(), "{place:?}");
                for (name, section) in [("tale", &g.tale), ("task", &g.task), ("watch", &g.watch)] {
                    assert!((2..=5).contains(&section.len()), "{place:?} {name}: {} lines", section.len());
                    assert!(section.iter().all(|l| !l.trim().is_empty()), "{place:?} {name}: an empty line");
                }
            }
        }
    }

    #[test]
    fn no_line_is_too_long_and_every_letter_can_be_drawn() {
        for place in places() {
            for hero in HEROES {
                let g = guide(place, hero);
                for l in lines(&g) {
                    assert!(l.chars().count() <= 160, "{place:?}: too long ({}): {l}", l.chars().count());
                    let bad: Vec<char> = l.chars().filter(|&c| !(c.is_ascii() && !c.is_ascii_control()) && !EXTRA.contains(&c)).collect();
                    assert!(bad.is_empty(), "{place:?}: the font cannot draw {bad:?} in: {l}");
                }
            }
        }
    }

    #[test]
    fn the_facts_are_the_same_for_every_hero_and_only_his_greeting_differs() {
        for place in places() {
            let r = guide(place, HeroKind::Ragnar);
            for hero in HEROES {
                let g = guide(place, hero);
                assert_eq!((&g.title, &g.tale[1..], &g.task, &g.watch), (&r.title, &r.tale[1..], &r.task, &r.watch), "{place:?}");
                let greeting = format!("{}, ", hail(hero));
                assert!(g.tale[0].starts_with(&greeting), "{place:?}: {}", g.tale[0]);
                assert_eq!(g.tale[0][greeting.len()..], r.tale[0][hail(HeroKind::Ragnar).len() + 2..], "{place:?}");
            }
        }
    }

    #[test]
    fn every_realm_warns_that_its_princes_offer_is_a_lie() {
        for sin in SINS {
            let g = guide(GuidePlace::Realm(sin), HeroKind::Ragnar);
            let said: String = g.task.iter().chain(g.watch.iter()).cloned().collect::<Vec<_>>().join(" ");
            let offer = match sin {
                Sin::Pride => "Take your throne",
                Sin::Envy => "All you want lies below",
                Sin::Wrath => "Sound the horn and the battle ends",
                Sin::Sloth => "Lie down. The door opens when you wake",
                Sin::Greed => "toll",
                Sin::Gluttony => "Finish your plate, and you may leave",
                Sin::Lust => "Turn around",
            };
            assert!(said.contains(offer), "{sin:?} does not name its prince's offer");
            assert!(said.contains("a lie"), "{sin:?} does not say the offer is a lie");
        }
    }

    #[test]
    fn the_garden_tells_of_krampus_and_the_meadow_of_odysseus() {
        for hero in HEROES {
            assert!(guide(GuidePlace::Garden, hero).tale.iter().any(|l| l.contains("Krampus")));
            for n in 0..3 {
                let tale = guide(GuidePlace::Cyclops(n), hero).tale;
                assert!(tale.iter().any(|l| l.contains("Odysseus") || l.contains("Polyphemus")), "meadow {n}");
            }
        }
    }

    #[test]
    fn the_old_man_never_cheats_nor_speaks_as_anything_but_truthful() {
        const NEVER: [&str; 8] =
            ["half price", "i lied", "i will lie", "i may lie", "do not trust me", "don't trust me", "trust me not", "i deceive"];
        for place in places() {
            for hero in HEROES {
                let g = guide(place, hero);
                for l in lines(&g) {
                    let low = l.to_lowercase();
                    assert!(NEVER.iter().all(|n| !low.contains(n)), "{place:?}: {l}");
                }
            }
        }
    }

    #[test]
    fn each_place_is_named_as_the_game_names_it() {
        assert_eq!(guide(GuidePlace::Realm(Sin::Greed), HeroKind::Ragnar).title, "THE VAULTS OF MAMMON");
        assert_eq!(guide(GuidePlace::Court(Sin::Greed), HeroKind::Ragnar).title, "THE COUNTING HOUSE");
        assert_eq!(guide(GuidePlace::Court(Sin::Lust), HeroKind::Ragnar).title, "THE ROSE BOWER");
        assert_eq!(guide(GuidePlace::Cyclops(0), HeroKind::Ragnar).title, "THE CYCLOPS' MEADOW");
        assert_eq!(guide(GuidePlace::Cyclops(1), HeroKind::Ragnar).title, "THE CYCLOPS' MEADOW 2");
        for sin in SINS {
            assert_eq!(guide(GuidePlace::Realm(sin), HeroKind::Mehmed).title, sin.realm().to_uppercase());
        }
        assert_eq!(guide(GuidePlace::Pit(3), HeroKind::Ragnar).title, "THE PIT: LEVEL 3");
    }

    #[test]
    fn the_pit_says_what_rides_down_and_what_each_level_holds() {
        use crate::logic::enemy::EnemyKind;
        for n in 1..=7u8 {
            let g = guide(GuidePlace::Pit(n), HeroKind::Ragnar);
            let said = g.task.join(" ").to_lowercase();
            for fact in ["two minutes", "rides down with you", "stays behind", "a tenth of your health", "green cross", "bell"] {
                assert!(said.contains(fact), "level {n}: the task does not say {fact:?}");
            }
            assert!(g.tale.iter().any(|l| l.contains("no devil")), "there is no Devil");
            // Each level's line names some of its own stage's foes, and none of another's alone.
            let watch = g.watch[0].to_lowercase();
            let named = EnemyKind::of_stage(n).iter().filter(|k| watch.contains(&format!("{k:?}").to_lowercase())).count();
            assert!(named >= 1 || n <= 2, "level {n}: {watch}");
            assert!(g.watch.join(" ").contains("ten"), "the cap");
        }
    }
}
