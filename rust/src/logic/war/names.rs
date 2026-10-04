//! The Ledger's names (WARBAND.md §8.1): a rival is named from his breed's folklore, Norse for
//! the draugr and the giants, Japanese for the yōkai, Turkish and Turkic for the Anatolian nights,
//! old European for the witches and the loup-garou, and hellish titles for the realms' own. His
//! epithet comes from the deed that made him somebody, and names the hero where it fits.
//!
//! Every name here is made up for the game (no real person's), and every letter of it is one the
//! HUD's font can draw (ASCII, and Ç Ö Ü Ş İ ı Ō with their small letters).

use crate::logic::enemy::EnemyKind;
use crate::logic::heroes::HeroKind;
use crate::logic::rng::Rng;

/// Whose folklore a breed belongs to, and so whose names its captains bear.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Culture {
    Norse,
    Japanese,
    Turkic,
    OldEuropean,
    /// The realms' own and the courts': not born anywhere, so titled.
    Hellish,
}

impl Culture {
    pub const ALL: [Culture; 5] = [Culture::Norse, Culture::Japanese, Culture::Turkic, Culture::OldEuropean, Culture::Hellish];

    pub fn of(kind: EnemyKind) -> Culture {
        use EnemyKind as K;
        match kind {
            K::Skeleton | K::DireWolf | K::FrostGiant | K::Surtr | K::Nidhogg | K::Hel => Culture::Norse,
            K::Tengu | K::Rasetsu | K::Jorogumo | K::Oni | K::Kappa | K::Rokurokubi | K::Kitsune | K::Gashadokuro | K::Goryo => {
                Culture::Japanese
            }
            K::Karakoncolos | K::Tulukus | K::Albasti | K::Cin | K::Arcura | K::Tepegoz | K::Ejder | K::Cobra => Culture::Turkic,
            K::Witch | K::Werewolf | K::Blemmyes | K::Helmeted | K::Bat | K::Vampire | K::LargeBat | K::Slime | K::Minion => {
                Culture::OldEuropean
            }
            K::Peerless | K::Ganglot | K::Gaki | K::Ellepige | K::Mammon | K::GoldGolem => Culture::Hellish,
            K::Lucifer
            | K::Reflection
            | K::Leviathan
            | K::Serpentling
            | K::Amon
            | K::Raven
            | K::Belphegor
            | K::Nightmare
            | K::Beelzebub
            | K::FlySwarm
            | K::Asmodeus => Culture::Hellish,
        }
    }

    /// Its given names, for a he and for a she.
    fn given(self, she: bool) -> &'static [&'static str] {
        match (self, she) {
            (Culture::Norse, false) => NORSE,
            (Culture::Norse, true) => NORSE_SHE,
            (Culture::Japanese, false) => JAPANESE,
            (Culture::Japanese, true) => JAPANESE_SHE,
            (Culture::Turkic, false) => TURKIC,
            (Culture::Turkic, true) => TURKIC_SHE,
            (Culture::OldEuropean, false) => EUROPEAN,
            (Culture::OldEuropean, true) => EUROPEAN_SHE,
            (Culture::Hellish, false) => HELLISH,
            (Culture::Hellish, true) => HELLISH_SHE,
        }
    }
}

/// A she: the witch, the spider-woman, the long-necked woman, the fox, the night hag, Hel, the elf-maid.
pub fn feminine(kind: EnemyKind) -> bool {
    use EnemyKind as K;
    matches!(kind, K::Witch | K::Jorogumo | K::Rokurokubi | K::Kitsune | K::Albasti | K::Hel | K::Ellepige)
}

/// "he" or "she", and "He" or "She".
pub fn pronoun(kind: EnemyKind, capital: bool) -> &'static str {
    match (feminine(kind), capital) {
        (false, false) => "he",
        (false, true) => "He",
        (true, false) => "she",
        (true, true) => "She",
    }
}

/// "his" or "her", and "His" or "Her".
pub fn possessive(kind: EnemyKind, capital: bool) -> &'static str {
    match (feminine(kind), capital) {
        (false, false) => "his",
        (false, true) => "His",
        (true, false) => "her",
        (true, true) => "Her",
    }
}

/// The breed as the name card says it, under the name: *karakoncolos*, *draugr*, *loup-garou*.
pub fn breed_name(kind: EnemyKind) -> &'static str {
    use EnemyKind as K;
    match kind {
        K::Skeleton => "draugr",
        K::Bat => "raven",
        K::Vampire => "dark raven",
        K::Slime => "bog slime",
        K::Witch => "witch",
        K::Minion => "witch's minion",
        K::Helmeted => "helmeted skeleton",
        K::Werewolf => "loup-garou",
        K::Blemmyes => "blemmyes",
        K::LargeBat => "great bat",
        K::Tengu => "tengu",
        K::Rasetsu => "rasetsu",
        K::DireWolf => "dire wolf",
        K::Jorogumo => "jorogumo",
        K::Cobra => "cobra",
        K::Kappa => "kappa",
        K::Rokurokubi => "rokurokubi",
        K::Kitsune => "kitsune",
        K::Oni => "oni",
        K::Tulukus => "tülükuş",
        K::Albasti => "albastı",
        K::Cin => "cin",
        K::Arcura => "arçura",
        K::Karakoncolos => "karakoncolos",
        K::Tepegoz => "tepegöz",
        K::Gashadokuro => "gashadokuro",
        K::Ejder => "ejder",
        K::Goryo => "goryō",
        K::FrostGiant => "frost giant",
        K::Surtr => "fire giant",
        K::Nidhogg => "root-dragon",
        K::Hel => "queen of the dead",
        K::Peerless => "peerless",
        K::Ganglot => "ganglöt",
        K::Gaki => "gaki",
        K::Ellepige => "ellepige",
        K::Mammon => "prince of greed",
        K::GoldGolem => "gold golem",
        K::Lucifer => "prince of pride",
        K::Reflection => "reflection",
        K::Leviathan => "prince of envy",
        K::Serpentling => "serpentling",
        K::Amon => "prince of wrath",
        K::Raven => "raven of amon",
        K::Belphegor => "prince of sloth",
        K::Nightmare => "nightmare",
        K::Beelzebub => "prince of gluttony",
        K::FlySwarm => "fly swarm",
        K::Asmodeus => "prince of lust",
    }
}

// ------------------------------------------------------------------ the tables

const NORSE: &[&str] = &[
    "Kolgrim",
    "Thorgaut",
    "Ketil",
    "Asvald",
    "Hallbjörn",
    "Ulfar",
    "Eyvind",
    "Svartkel",
    "Thrand",
    "Vebrand",
    "Geirmund",
    "Bjarnulf",
    "Ormar",
    "Grimkel",
    "Torfi",
    "Isolf",
    "Hrafn",
    "Gunnolf",
    "Sölvar",
    "Steinar",
    "Kalfr",
];
const NORSE_SHE: &[&str] = &[
    "Ulfhild", "Sigvör", "Gunnhvit", "Ragnfrid", "Asgerd", "Thorunn", "Hallveig", "Svanlaug", "Ingunn", "Ketilrid", "Hrefna", "Jorunn",
    "Valgerd", "Bergljot", "Arnkatla", "Kolfinna", "Heimlaug",
];
/// Fathers' names, for "son of" and "daughter of".
const NORSE_FATHERS: &[&str] = &[
    "Ketil", "Grim", "Hrafn", "Orm", "Kol", "Thord", "Ulf", "Bjarn", "Stein", "Geir", "Svart", "Hall", "Vemund", "Asgaut", "Eyolf", "Tosti",
];

const JAPANESE: &[&str] = &[
    "Kurokane",
    "Akaishi",
    "Sabimaru",
    "Hizame",
    "Kagenui",
    "Honedachi",
    "Yamigiri",
    "Kirisame",
    "Tsuragane",
    "Ōkuma",
    "Kagerō",
    "Shibuki",
    "Tetsuzō",
    "Gorōmaru",
    "Akagane",
    "Kurosabi",
    "Nagidachi",
    "Uzumaru",
];
const JAPANESE_SHE: &[&str] = &[
    "Kagari", "Shizuku", "Tsuyuha", "Hotarubi", "Akane", "Murasame", "Kiriko", "Nagisa", "Tamayura", "Sayogiri", "Yomigiku", "Hisame",
    "Kurenai", "Usugumo", "Mizore", "Tsukuyo", "Ayame",
];

/// The Turkic names go with a byname before them: *Kara* Ayaz, *Deli* Poyraz.
const TURKIC_BY: &[&str] = &["Kara", "Ak", "Boz", "Deli", "Kızıl", "Sarı", "Topal", "Gök", "Demir", "Çolak", "Kör", "Yaman", "Alaca"];
const TURKIC: &[&str] = &[
    "Ayaz", "Poyraz", "Alaz", "Yalaz", "Tolun", "Bora", "Kurt", "Çakır", "Tunç", "Sungur", "Kartal", "Tuman", "Bulut", "Kaya", "Börü",
    "Ilgaz", "Sancar", "Tepeli",
];
const TURKIC_SHE: &[&str] = &[
    "Aysıla", "Gülsüm", "Şirin", "Nazlı", "Peri", "Çiçek", "Selcan", "Tülin", "Yıldız", "Ayten", "Sevim", "Gülizar", "Narin", "Zümrüt",
    "Bengü", "İnci", "Ülker",
];

/// Old European names, half of them with a place after them: Corbin *of Crowmoor*.
const EUROPEAN: &[&str] = &[
    "Malvern", "Corbin", "Lothar", "Anselm", "Ivo", "Bertrand", "Wolfram", "Gerolt", "Thibault", "Raimbaut", "Hugolin", "Grisel", "Fulk",
    "Odric", "Baldric", "Ancelin", "Morcant", "Ulrich",
];
const EUROPEAN_SHE: &[&str] = &[
    "Agathe",
    "Grizel",
    "Mabyn",
    "Hedwige",
    "Ysolde",
    "Benedetta",
    "Ottilie",
    "Gudula",
    "Aldegonde",
    "Ermengarde",
    "Sibylle",
    "Mechtild",
    "Perrine",
    "Gisla",
    "Hildeburg",
    "Morwenna",
    "Ilsabet",
];
const EUROPEAN_PLACES: &[&str] = &[
    "Crowmoor",
    "Blackthorn",
    "Mournwood",
    "Vasseloup",
    "Hollowmere",
    "Ashgrave",
    "Duskwater",
    "Ravensgard",
    "Grimbald",
    "Wolfsheim",
    "Saint-Ombre",
    "Coldharbour",
    "Nettlefold",
    "Morteau",
    "Graufels",
    "Weepingford",
];

/// Hellish names go after a title: *Marquis* Vashrel, *Prioress* Iblith.
const HELL_TITLES: &[&str] = &[
    "Duke",
    "Marquis",
    "Count",
    "Prefect",
    "Warden",
    "Reeve",
    "Steward",
    "Provost",
    "Baron",
    "Tallyman",
    "Chancellor",
    "Margrave",
    "Castellan",
    "Bailiff",
    "Viscount",
    "Abbot",
];
const HELL_TITLES_SHE: &[&str] =
    &["Duchess", "Marquise", "Countess", "Prioress", "Abbess", "Baroness", "Margravine", "Wardeness", "Chatelaine", "Canoness"];
const HELLISH: &[&str] = &[
    "Vashrel",
    "Ormogh",
    "Kezzarin",
    "Gharzul",
    "Ulzeth",
    "Vexmoor",
    "Thraxiel",
    "Azgaroth",
    "Sarkovel",
    "Morvane",
    "Carnivex",
    "Zhulmorath",
    "Ashrabel",
    "Druvain",
    "Skelloth",
    "Quorvex",
    "Belzorach",
];
const HELLISH_SHE: &[&str] = &[
    "Iblith",
    "Vesperine",
    "Morgaunt",
    "Sylvexa",
    "Ashmeret",
    "Callowyn",
    "Nyxandre",
    "Ombrelle",
    "Thessavine",
    "Ravelline",
    "Lilivane",
    "Zerafine",
    "Mordessa",
    "Cindrelle",
    "Vaelith",
    "Grimaline",
];

/// A rival's name, made from his breed's folklore and `seed` (the same seed, the same name).
pub fn name(kind: EnemyKind, seed: u32) -> String {
    let mut rng = Rng::new(seed ^ 0x4e41_4d45);
    let culture = Culture::of(kind);
    let she = feminine(kind);
    let pick = |rng: &mut Rng, list: &[&'static str]| list[rng.index(list.len())];
    let given = pick(&mut rng, culture.given(she));
    match culture {
        Culture::Norse => {
            // Half of them are known by their father's name too.
            if rng.next() < 0.5 {
                let father = pick(&mut rng, NORSE_FATHERS);
                format!("{given} {father}{}", if she { "sdottir" } else { "sson" })
            } else {
                given.to_string()
            }
        }
        Culture::Japanese => given.to_string(),
        Culture::Turkic => format!("{} {given}", pick(&mut rng, TURKIC_BY)),
        Culture::OldEuropean => {
            if rng.next() < 0.5 {
                format!("{given} of {}", pick(&mut rng, EUROPEAN_PLACES))
            } else {
                given.to_string()
            }
        }
        Culture::Hellish => {
            let title = pick(&mut rng, if she { HELL_TITLES_SHE } else { HELL_TITLES });
            format!("{title} {given}")
        }
    }
}

/// The given names a culture's he (or she) may bear, for the tests.
pub fn given_names(culture: Culture, she: bool) -> &'static [&'static str] {
    culture.given(she)
}

/// What a foe did to become somebody (WARBAND.md §8.1).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Deed {
    /// It dealt the killing blow.
    KillingBlow,
    /// It broke his guard three times in one fight.
    ThreeGuardBreaks,
    /// It lived through a fight below a quarter of its health (routed, or he walked off).
    SurvivedLow,
    /// A pack's leader he left behind, alive, when the leash ran out.
    LeftBehind,
    /// It took his silver when it killed him.
    TookSilver,
}

impl Deed {
    pub const ALL: [Deed; 5] = [Deed::KillingBlow, Deed::ThreeGuardBreaks, Deed::SurvivedLow, Deed::LeftBehind, Deed::TookSilver];
}

/// What the hero is called, in a rival's mouth: the viking, the janissary, the samurai.
pub fn hero_title(hero: HeroKind) -> &'static str {
    match hero {
        HeroKind::Ragnar => "the viking",
        HeroKind::Mehmed => "the janissary",
        HeroKind::Miyamoto => "the samurai",
    }
}

/// The epithets a deed earns against `hero`: those that name him (by name, by what he is, by his
/// way of fighting), and those that need no hero.
fn epithets(deed: Deed, hero: HeroKind) -> Vec<String> {
    let n = hero.name();
    match deed {
        Deed::KillingBlow => {
            let mut v = vec![format!("{n}'s Bane"), format!("Who Felled {n}")];
            v.extend(
                match hero {
                    HeroKind::Ragnar => ["Who Ate the Viking", "Bane of the Northman", "Who Split the Shield-Wall"],
                    HeroKind::Mehmed => ["Who Ate the Janissary", "Bane of the Ocak", "Who Spilled the Cauldron"],
                    HeroKind::Miyamoto => ["Bane of the Two Swords", "Who Ate the Samurai", "Who Broke the Two Heavens"],
                }
                .map(String::from),
            );
            v
        }
        Deed::ThreeGuardBreaks => {
            let mut v: Vec<String> = ["the Shield-Breaker", "the Shield-Splitter", "the Guard-Breaker"].map(String::from).to_vec();
            v.push(match hero {
                HeroKind::Ragnar => "Who Split the Linden Shield".to_string(),
                HeroKind::Mehmed => "Breaker of the Kalkan".to_string(),
                HeroKind::Miyamoto => "Who Beat Down the Guard".to_string(),
            });
            v
        }
        Deed::SurvivedLow => ["the Twice-Wounded", "Who Fled the Blade", "the Scarred", "the Hard to Kill"].map(String::from).to_vec(),
        Deed::LeftBehind => {
            let mut v: Vec<String> = ["the Unfinished", "the Unbowed", "Who Was Left Standing"].map(String::from).to_vec();
            v.push(format!("Whom {n} Walked Past"));
            v
        }
        Deed::TookSilver => {
            let mut v: Vec<String> = ["the Silver-Drinker", "the Purse-Cutter", "the Coin-Swallower"].map(String::from).to_vec();
            v.push(format!("Who Took {n}'s Silver"));
            v
        }
    }
}

/// The epithet `deed` earns against `hero`, from `seed`.
pub fn epithet(deed: Deed, hero: HeroKind, seed: u32) -> String {
    let all = epithets(deed, hero);
    let mut rng = Rng::new(seed ^ 0x4550_4954);
    all[rng.index(all.len())].clone()
}

/// Every epithet `deed` may earn against `hero`, for the tests.
pub fn every_epithet(deed: Deed, hero: HeroKind) -> Vec<String> {
    epithets(deed, hero)
}

/// Can the HUD's font draw every letter of `text`?
pub fn drawable(text: &str) -> bool {
    text.chars().all(|c| (' '..='~').contains(&c) || "ÇçÖöÜüŞşİıŌō".contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::heroes::ROSTER;

    fn every_kind() -> Vec<EnemyKind> {
        crate::logic::war::ledger::KINDS.to_vec()
    }

    #[test]
    fn names_are_deterministic_and_of_the_breeds_folklore() {
        for kind in every_kind() {
            let culture = Culture::of(kind);
            let given = culture.given(feminine(kind));
            assert!(given.len() >= 16, "{culture:?}: {} names", given.len());
            for seed in 0..60u32 {
                let a = name(kind, seed);
                assert_eq!(a, name(kind, seed), "the same seed, the same name");
                assert!(drawable(&a), "{a}: a letter the font cannot draw");
                // The name is built round one of its culture's given names, in its culture's way.
                assert!(given.iter().any(|g| a.split(' ').any(|w| w == *g)), "{kind:?} {a}: not a {culture:?} name");
                match culture {
                    Culture::Turkic => assert!(TURKIC_BY.contains(&a.split(' ').next().unwrap()), "{a}"),
                    Culture::Hellish => {
                        let title = a.split(' ').next().unwrap();
                        assert!(HELL_TITLES.contains(&title) || HELL_TITLES_SHE.contains(&title), "{a}");
                    }
                    Culture::Norse if a.contains(' ') => assert!(a.ends_with("sson") || a.ends_with("sdottir"), "{a}"),
                    Culture::OldEuropean if a.contains(' ') => assert!(a.contains(" of "), "{a}"),
                    _ => {}
                }
            }
            // Many seeds, many names.
            let mut seen: Vec<String> = (0..200u32).map(|s| name(kind, s)).collect();
            seen.sort();
            seen.dedup();
            assert!(seen.len() >= 12, "{kind:?}: only {} names in 200 seeds", seen.len());
        }
        // The cultures are the ones the plan gives.
        assert_eq!(Culture::of(EnemyKind::Karakoncolos), Culture::Turkic);
        assert_eq!(Culture::of(EnemyKind::Skeleton), Culture::Norse);
        assert_eq!(Culture::of(EnemyKind::Oni), Culture::Japanese);
        assert_eq!(Culture::of(EnemyKind::Werewolf), Culture::OldEuropean);
        assert_eq!(Culture::of(EnemyKind::Gaki), Culture::Hellish);
        // No name belongs to two cultures.
        for a in Culture::ALL {
            for b in Culture::ALL.into_iter().filter(|&b| b != a) {
                for she in [false, true] {
                    for n in a.given(she) {
                        assert!(!b.given(false).contains(n) && !b.given(true).contains(n), "{n} is both {a:?} and {b:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn every_epithet_fits_its_deed_and_names_the_hero_where_it_fits() {
        for hero in ROSTER {
            for deed in Deed::ALL {
                let all = every_epithet(deed, hero);
                assert!(all.len() >= 3, "{deed:?}");
                for e in &all {
                    assert!(drawable(e), "{e}");
                    // Another hero's name never turns up in his epithets.
                    for other in ROSTER.into_iter().filter(|&o| o != hero) {
                        assert!(!e.contains(other.name()), "{e} against {}", hero.name());
                    }
                }
                for seed in 0..20 {
                    assert!(all.contains(&epithet(deed, hero, seed)));
                }
            }
            assert!(every_epithet(Deed::KillingBlow, hero).iter().any(|e| e.contains(hero.name())));
        }
        assert!(every_epithet(Deed::KillingBlow, HeroKind::Ragnar).contains(&"Ragnar's Bane".to_string()));
        assert!(every_epithet(Deed::KillingBlow, HeroKind::Mehmed).contains(&"Who Ate the Janissary".to_string()));
        assert!(every_epithet(Deed::KillingBlow, HeroKind::Miyamoto).contains(&"Bane of the Two Swords".to_string()));
        assert!(every_epithet(Deed::ThreeGuardBreaks, HeroKind::Mehmed).contains(&"the Shield-Breaker".to_string()));
        assert!(every_epithet(Deed::SurvivedLow, HeroKind::Mehmed).contains(&"the Twice-Wounded".to_string()));
        assert!(every_epithet(Deed::LeftBehind, HeroKind::Mehmed).contains(&"the Unfinished".to_string()));
        assert!(every_epithet(Deed::TookSilver, HeroKind::Mehmed).contains(&"the Silver-Drinker".to_string()));
    }

    #[test]
    fn every_breed_name_can_be_drawn() {
        for kind in every_kind() {
            assert!(drawable(breed_name(kind)), "{kind:?}");
        }
    }
}
