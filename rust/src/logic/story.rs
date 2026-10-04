//! The story. Scene I, "The Threshold", plays after Play on the title; Scene II, "The Way Up",
//! plays at the very end, after Krampus falls; and the road the whole adventure follows between
//! them is set out here too.
//!
//! Four friends play by the iron gates of an old graveyard at dusk until
//! Krampus bursts out of the fog and drags one of them back into the graves.
//! The other three chase him through the gates and fall straight through into
//! the underworld, where an old gatekeeper and his dog Quitmer keep an iron
//! door. The two of them slept while Krampus slipped past; out of guilt the
//! old man lends the boys the souls of three great warriors (Ragnar the
//! viking, Mehmed the janissary and Miyamoto the samurai), swears to ask the
//! archangels to send them help, and promises to come whenever they call. From there the road
//! only goes down, depth by depth, past the House of the Devil (which no one has ever seen the
//! inside of) to the bottom of the world, where Krampus takes the children he steals.
//!
//! At the bottom the friend is freed, and Scene II plays (FINALE.md §4): the two hold each other,
//! a white light falls down the shaft, voices sing, the platform carries them up through the
//! seven levels of the Pit into the white, and the words come: there was never a devil. Then the
//! credits, and the title.

/// Who says a line.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Speaker {
    /// No one: the words run under the picture.
    Narrator,
    /// The three friends still free, as one.
    Boys,
    /// The old gatekeeper.
    OldMan,
    /// Quitmer, the gatekeeper's dog.
    Dog,
}

impl Speaker {
    /// The name on the dialogue box ("" for the narrator).
    pub fn name(self) -> &'static str {
        match self {
            Speaker::Narrator => "",
            Speaker::Boys => "THE BOYS",
            Speaker::OldMan => "THE OLD MAN",
            Speaker::Dog => "THE DOG",
        }
    }
}

/// Where a beat takes place.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Place {
    /// The edge of the old graveyard, in the overworld, at dusk.
    Graveyard,
    /// The dungeon gateway at the top of the underworld, dimly lit.
    Gateway,
    /// The shaft of the Pit, from its bottom up through its seven levels into the light.
    Pit,
}

/// What the picture does during a beat: the view stages each one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shot {
    /// The scene's name over black.
    Title,
    /// The graveyard at dusk; the friends kick a ball about until it rolls through the gate.
    Establish,
    /// Krampus bursts out of the fog, seizes the boy at the gate and drags him into the graves.
    Taken,
    /// The three left behind: "He took him!"
    Scream,
    /// They sprint through the gates after him, into the fog, and fall through.
    Pursuit,
    /// They tumble out on the flagstones of a dungeon; the way back closes behind them.
    Arrive,
    /// The dog lunges at them on his chain.
    Snarl,
    /// The old man calms him.
    Easy,
    /// The old man speaks.
    OldMan,
    /// The boys answer.
    Boys,
    /// The old man offers the three warriors' souls, which rise as he names them.
    Souls,
    /// His promise of help.
    Promise,
    /// The boys take up the souls, the iron door opens, and they go down.
    Descent,
    // ---- Scene II
    /// At the bottom of the Pit: the freed boy runs to the hero, who kneels, and they hold each other.
    Embrace,
    /// A white light falls down the shaft from far above, and the voices begin.
    Light,
    /// The platform rises with the two of them through the seven levels, into the white.
    Rise,
    /// Nothing but white.
    White,
    /// The words, one line at a time.
    Words,
    /// The credits, rolling.
    Credits,
}

impl Shot {
    pub fn place(self) -> Place {
        match self {
            Shot::Title | Shot::Establish | Shot::Taken | Shot::Scream | Shot::Pursuit => Place::Graveyard,
            Shot::Embrace | Shot::Light | Shot::Rise | Shot::White | Shot::Words | Shot::Credits => Place::Pit,
            _ => Place::Gateway,
        }
    }

    /// The scene the shot belongs to.
    pub fn scene(self) -> Scene {
        match self.place() {
            Place::Pit => Scene::Two,
            _ => Scene::One,
        }
    }
}

/// Which of the story's two scenes.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Scene {
    /// "The Threshold": the graveyard and the gateway, at the start.
    #[default]
    One,
    /// "The Way Up": the bottom of the Pit and the rise into the light, at the end.
    Two,
}

impl Scene {
    pub fn beats(self) -> &'static [Beat] {
        match self {
            Scene::One => SCENE_ONE,
            Scene::Two => SCENE_TWO,
        }
    }

    /// Its name, on its title card and over the letterbox.
    pub fn title(self) -> Card {
        match self {
            Scene::One => SCENE_TITLE,
            Scene::Two => SCENE_TWO_TITLE,
        }
    }

    /// Seconds from the start of the scene to `t` seconds into beat `beat` (its action beats
    /// count their whole hold; Scene II has nothing else).
    pub fn time_at(self, beat: usize, t: f64) -> f64 {
        self.beats().iter().take(beat).map(|b| b.hold).sum::<f64>() + t
    }

    /// The beat playing `secs` seconds into the scene, and how far into it: the inverse of
    /// `time_at` (past the end, the last beat at its end).
    pub fn beat_at(self, secs: f64) -> (usize, f64) {
        let beats = self.beats();
        let mut left = secs.max(0.0);
        for (i, b) in beats.iter().enumerate() {
            if i + 1 == beats.len() {
                return (i, left.min(b.hold));
            }
            if left < b.hold {
                return (i, left);
            }
            left -= b.hold;
        }
        (0, 0.0)
    }
}

/// A card naming the place, shown as its scene opens.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Card {
    pub title: &'static str,
    pub sub: &'static str,
}

/// One step of the scene: a piece of action, or a line of dialogue.
#[derive(Clone, Copy, Debug)]
pub struct Beat {
    pub shot: Shot,
    pub speaker: Speaker,
    /// A stage direction shown beside the speaker's name ("screaming").
    pub cue: &'static str,
    /// What is said; empty for a beat that is all action.
    pub line: &'static str,
    /// An action beat moves on by itself after this many seconds; 0 waits for the player.
    pub hold: f64,
    pub card: Option<Card>,
}

impl Beat {
    const fn action(shot: Shot, hold: f64, card: Option<Card>) -> Self {
        Self { shot, speaker: Speaker::Narrator, cue: "", line: "", hold, card }
    }

    const fn say(shot: Shot, speaker: Speaker, cue: &'static str, line: &'static str) -> Self {
        Self { shot, speaker, cue, line, hold: 0.0, card: None }
    }

    /// Does the player move this beat on (rather than time)?
    pub fn waits(&self) -> bool {
        self.hold <= 0.0
    }
}

/// The scene's name, on the title card.
pub const SCENE_TITLE: Card = Card { title: "THE THRESHOLD", sub: "SCENE I" };

const GRAVEYARD: Card = Card { title: "THE OLD GRAVEYARD", sub: "Dusk" };
const GATEWAY: Card = Card { title: "THE DUNGEON GATEWAY", sub: "The threshold of the underworld" };

/// The line in which the old man names the three warriors, whose souls rise as he does.
pub const SOULS_LINE: &str =
    "Then you will not go alone. Rise, old souls... Ragnar (the Viking)... Mehmed (the Janissary)... Miyamoto (the Samurai). Will you carry them?";
/// His promise, made out of guilt: the archangels' help (their shrines in every dungeon), and his own.
pub const PROMISE_LINE: &str =
    "So it begins... This is my failure, and I feel the guilt of it. I will ask the archangels to send you help. And call me whenever you need - I will come.";
/// The last words: the road down past the House of the Devil, and why it goes so deep.
pub const ROUTE_LINE: &str =
    "The boys took up the warrior souls and went down into the dark, depth by depth, after their stolen friend. The trail led past the House of the Devil, which no one has ever seen inside... for Krampus takes the children he steals to the bottom of the world.";

/// Scene I, "The Threshold", beat by beat.
pub const SCENE_ONE: &[Beat] = &[
    Beat::action(Shot::Title, 3.4, None),
    // EXT. GRAVEYARD - DUSK
    Beat::action(Shot::Establish, 7.4, Some(GRAVEYARD)),
    Beat::action(Shot::Taken, 4.6, None),
    Beat::say(Shot::Scream, Speaker::Boys, "screaming", "NO! He took him! LET HIM GO!"),
    Beat::action(Shot::Pursuit, 3.4, None),
    // INT. DUNGEON GATEWAY - CONTINUOUS
    Beat::action(Shot::Arrive, 5.6, Some(GATEWAY)),
    Beat::say(Shot::Snarl, Speaker::Dog, "snarls viciously", "GRRRRR... RRRAAAHH!"),
    Beat::say(Shot::Easy, Speaker::OldMan, "hoarsely", "Easy, Quitmer... easy. Down, old friend."),
    Beat::say(
        Shot::OldMan,
        Speaker::OldMan,
        "turning to the boys",
        "Living children... at my door? No. No, no. You cannot be here. Nothing warm passes this gate.",
    ),
    Beat::say(
        Shot::Boys,
        Speaker::Boys,
        "breathless",
        "A monster came out of the fog - horns, chains, bells... It dragged our friend into the graves!",
    ),
    Beat::say(
        Shot::OldMan,
        Speaker::OldMan,
        "",
        "Horns and bells... Krampus. So he slipped past me. I closed my eyes for one moment... one moment.",
    ),
    Beat::say(Shot::Boys, Speaker::Boys, "", "Then help us! We have to get him back!"),
    Beat::say(
        Shot::OldMan,
        Speaker::OldMan,
        "",
        "Beyond this door lies the world of the dead. The living do not walk there... not for long.",
    ),
    Beat::say(Shot::Boys, Speaker::Boys, "angrily", "You were supposed to guard it! How could you let him out?"),
    Beat::say(
        Shot::OldMan,
        Speaker::OldMan,
        "looking away",
        "Quitmer and I have kept this door a thousand years... and tonight, of all nights, we slept.",
    ),
    Beat::say(Shot::Boys, Speaker::Boys, "", "We are not going home without him. There has to be a way!"),
    Beat::say(
        Shot::OldMan,
        Speaker::OldMan,
        "gravely",
        "There is one way. Only one. But hear me: die down there, and you will never see the sun again.",
    ),
    Beat::say(Shot::Boys, Speaker::Boys, "", "Then we will not die. Whatever it takes - we are going."),
    Beat::say(Shot::Souls, Speaker::OldMan, "", SOULS_LINE),
    Beat::say(Shot::Boys, Speaker::Boys, "", "We will. For our friend."),
    Beat::say(Shot::Promise, Speaker::OldMan, "heavily", PROMISE_LINE),
    Beat::say(Shot::Descent, Speaker::Narrator, "", ROUTE_LINE),
];

/// The first beat of a shot in its own scene, if the scene has one.
pub fn first(shot: Shot) -> Option<usize> {
    shot.scene().beats().iter().position(|b| b.shot == shot)
}

/// Seconds the typewriter holds after letter `i` of `line`: a moment on a comma or colon,
/// longer at the end of a sentence; a run of dots trails off slowly.
pub fn pause_after(line: &str, i: usize) -> f64 {
    match line.as_bytes().get(i) {
        Some(b'.') if line.as_bytes().get(i + 1) == Some(&b'.') => 0.05,
        Some(b'.') | Some(b'!') | Some(b'?') => 0.28,
        Some(b',') | Some(b':') | Some(b'-') => 0.14,
        _ => 0.0,
    }
}

/**
 * When things happen inside the action beats, in seconds from the start of
 * the beat: the picture and the sound both keep to these.
 */
pub mod stage {
    /// The friends pass the ball round: when each kick lands.
    pub const KICKS: [f32; 5] = [0.35, 1.15, 1.95, 2.75, 3.5];
    /// The last kick goes wide: when it is struck, and when the ball rattles through the gate.
    pub const MISKICK: f32 = 4.2;
    pub const BALL_AT_GATE: f32 = 5.1;
    /// The boy nearest runs after it and reaches the gate.
    pub const FETCH: f32 = 5.35;
    pub const AT_GATE: f32 = 6.5;
    /// Krampus: his bells in the fog, his eyes, his rush, the gate bursting open, the grab,
    /// the turn back, and when the fog has him again.
    pub const BELLS: f32 = 0.15;
    pub const EYES: f32 = 0.35;
    pub const RUSH: f32 = 1.0;
    pub const GATE_BURST: f32 = 1.3;
    pub const GRAB: f32 = 1.5;
    pub const TURN: f32 = 2.05;
    pub const GONE: f32 = 3.1;
    /// The chase through the gate, and the white flash as they fall through.
    pub const FLASH: f32 = 2.55;
    /// Arriving: each boy tumbling out, the way back closing, and the dog seen at last.
    pub const TUMBLE: [f32; 3] = [0.3, 0.48, 0.66];
    pub const CLOSE: f32 = 1.75;
    pub const REVEAL: f32 = 2.9;
    pub const GROWL: f32 = 3.3;
    /// The end: the souls go to the boys, the door grinds open, they walk down.
    pub const SOULS_TAKEN: f32 = 0.8;
    pub const DOOR: f32 = 2.3;
    pub const WALK: f32 = 3.6;
}

// ----------------------------------------------------------------- Scene II

/// Scene II's name, over the letterbox.
pub const SCENE_TWO_TITLE: Card = Card { title: "THE WAY UP", sub: "SCENE II" };

const BOTTOM: Card = Card { title: "THE BOTTOM OF THE PIT", sub: "Beneath everything" };

/// Scene II, "The Way Up", beat by beat: all action, nothing said. It plays once Krampus is
/// dead and the friend he took has reached the hero (`Game::begin_ending`).
pub const SCENE_TWO: &[Beat] = &[
    Beat::action(Shot::Embrace, 8.0, Some(BOTTOM)),
    Beat::action(Shot::Light, 7.0, None),
    Beat::action(Shot::Rise, 15.0, None),
    Beat::action(Shot::White, 3.0, None),
    Beat::action(Shot::Words, 34.0, None),
    Beat::action(Shot::Credits, 44.0, None),
];

/// The words, after the white (FINALE.md §4): each appears on its own and stays, the poem
/// gathering line by line.
pub const WORDS: [&str; 6] = [
    "There was never a devil.",
    "Only the dark we carry, and the light we choose.",
    "Every monster in these depths was made of us,",
    "and so was every hand that reached down to lift another up.",
    "The good and the bad were never below.",
    "They were always within.",
];

/// Where a blank line falls in the poem: before each of these lines.
pub const STANZA_BREAKS: [usize; 3] = [1, 2, 4];

/**
 * When things happen inside Scene II's beats, in seconds from the start of the
 * beat; the picture, the sound and the words keep to these.
 */
pub mod ascent {
    /// The embrace: the picture comes up out of black, the boy runs to him, he kneels, they hold.
    pub const FADE_IN: f32 = 1.4;
    pub const RUN: f32 = 0.9;
    pub const KNEEL: f32 = 1.8;
    pub const REACH: f32 = 2.9;
    /// The light: a thread of it far above, the first chord of the voices, the light reaching
    /// them, the two looking up into it, and the hero standing with the boy's hand in his.
    pub const LIGHT_FAR: f32 = 0.5;
    pub const VOICES: f32 = 1.6;
    pub const LIGHT_TOUCH: f32 = 3.0;
    pub const LOOK_UP: f32 = 3.3;
    pub const STAND: f32 = 4.6;
    /// The rise: the platform lifts, passes each level, and reaches the top; the white floods in
    /// from `WHITEOUT` to the end of the beat.
    pub const LIFT: f32 = 0.8;
    pub const TOP: f32 = 13.8;
    pub const WHITEOUT: f32 = 10.8;
    /// The words: when each line begins to appear, and when they all fade.
    pub const WORDS_AT: [f32; 6] = [1.2, 5.6, 10.6, 14.4, 20.0, 24.2];
    pub const WORDS_FADE: f32 = 30.0;
    /// Seconds a line takes to come up.
    pub const LINE_IN: f32 = 1.6;
    /// The credits: the dark comes in, the roll runs, and THE END holds.
    pub const ROLL_FROM: f32 = 1.5;
    pub const ROLL_SECONDS: f32 = 34.0;
    pub const THE_END: f32 = 37.0;
}

/// How far the platform has risen in the rise, 0 at the bottom to 1 at the top.
pub fn rise(t: f32) -> f32 {
    let u = ((t - ascent::LIFT) / (ascent::TOP - ascent::LIFT)).clamp(0.0, 1.0);
    // Slow away, steady through the levels, slowing into the light.
    u * u * (3.0 - 2.0 * u) * 0.35 + u * 0.65
}

/// A line of the credits.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Credit {
    /// The game's name, as on the title.
    Title,
    /// A heading over a group: "THE WARRIORS".
    Heading(&'static str),
    /// A name, and what they were.
    Name(String, String),
    /// A line on its own.
    Line(String),
    /// Space between groups.
    Gap,
}

/// The credits, top to bottom: the game, the three warriors, the gatekeeper and his dog, the
/// friend who was taken, the Pit and its keeper, the princes, the archangels, and how it was made.
pub fn credits() -> Vec<Credit> {
    use crate::logic::angels::ARCHANGELS;
    use crate::logic::heroes::HeroKind;
    use crate::logic::sins::SINS;
    let mut out = vec![Credit::Title, Credit::Gap, Credit::Heading("THE THREE WARRIORS")];
    for h in [HeroKind::Ragnar, HeroKind::Mehmed, HeroKind::Miyamoto] {
        out.push(Credit::Name(h.name().to_string(), h.title().to_string()));
    }
    out.extend([
        Credit::Gap,
        Credit::Heading("THE GATEKEEPER"),
        Credit::Name("The Old Man".into(), "who kept the door, and kept his word".into()),
        Credit::Name("Quitmer".into(), "his dog".into()),
        Credit::Gap,
        Credit::Line("and the friend who was taken".into()),
        Credit::Gap,
        Credit::Heading("THE PIT"),
        Credit::Line("Seven levels, and what waited in them".into()),
        Credit::Name("Krampus".into(), "at the bottom of the world".into()),
        Credit::Gap,
        Credit::Heading("THE PRINCES OF THE SEVEN SINS"),
    ]);
    for s in SINS {
        out.push(Credit::Name(s.prince().to_string(), s.name().to_string()));
    }
    out.extend([Credit::Gap, Credit::Heading("WHO SENT HELP")]);
    for a in ARCHANGELS {
        out.push(Credit::Name(a.name().to_string(), "archangel".into()));
    }
    out.extend([Credit::Gap, Credit::Gap, Credit::Line("Made with Claude".into())]);
    out
}

// ----------------------------------------------------------------- the road

/**
 * The road the adventure takes. Past the gatekeeper's door each dungeon lies
 * a depth further down the underworld than the last, a Cyclops keeps the way
 * down out of each, and the trail runs on past the House of the Devil, which
 * no one has seen the inside of, to the bottom of the world: that is where
 * Krampus takes the children he steals.
 */
pub struct Route {
    /// What the boys want.
    pub goal: &'static str,
    /// Whom they follow.
    pub quarry: &'static str,
    /// Where the trail ends.
    pub destination: &'static str,
    /// Why it ends there.
    pub reveal: &'static str,
}

pub const ROUTE: Route = Route {
    goal: "Bring back the friend Krampus stole",
    quarry: "Krampus",
    destination: "The bottom of the world, down past the House of the Devil",
    reveal: "Krampus takes the children he steals to the bottom of the world",
};

/// Roman numerals, for the depths.
pub fn roman(mut n: usize) -> String {
    const TABLE: [(usize, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut out = String::new();
    for (v, s) in TABLE {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    out
}

/// How far down the underworld dungeon `index` (and the Cyclops after it) lies: Dungeon I is the first depth.
pub fn depth(dungeon: usize) -> usize {
    dungeon + 1
}

/// Where the road stands, for the countdown: the depth, going down after Krampus.
pub fn depth_line(dungeon: usize, boss: bool) -> String {
    let d = roman(depth(dungeon));
    if boss {
        format!("DEPTH {d}: A CYCLOPS GUARDS THE WAY DOWN")
    } else {
        format!("DEPTH {d} OF THE UNDERWORLD")
    }
}

/// What the boys are doing, for the pause screen.
pub fn objective() -> &'static str {
    "Follow Krampus down through the underworld, past the House of the Devil to the bottom of the world, and bring your friend home."
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::heroes::ROSTER;

    #[test]
    fn the_old_man_offers_the_heroes_of_the_character_screen() {
        let line = SCENE_ONE[first(Shot::Souls).unwrap()].line;
        for hero in ROSTER {
            let offer = format!("{} ({})", hero.name(), hero.title());
            assert!(line.to_lowercase().contains(&offer.to_lowercase()), "{offer} missing from {line:?}");
        }
        // Ragnar, Mehmed, Miyamoto: named in that order.
        let at = |n: &str| line.find(n).unwrap();
        assert!(at("Ragnar") < at("Mehmed") && at("Mehmed") < at("Miyamoto"));
    }

    #[test]
    fn the_old_man_promises_help_and_the_road_leads_down_to_the_bottom_of_the_world() {
        let promise = SCENE_ONE[first(Shot::Promise).unwrap()];
        assert_eq!(promise.speaker, Speaker::OldMan);
        assert!(promise.line.contains("call me"));
        assert!(promise.line.contains("guilt"));
        assert!(promise.line.contains("I will ask the archangels to send you help"));
        let last = SCENE_ONE.last().unwrap();
        assert_eq!(last.shot, Shot::Descent);
        assert!(last.line.contains("House of the Devil") && last.line.contains("bottom of the world"));
        assert!(ROUTE.reveal.contains("bottom of the world"));
    }

    #[test]
    fn the_scene_runs_from_the_graveyard_to_the_gateway_and_every_line_can_be_drawn() {
        assert_eq!(SCENE_ONE[0].shot, Shot::Title);
        let places: Vec<Place> = SCENE_ONE.iter().map(|b| b.shot.place()).collect();
        let turn = places.iter().position(|&p| p == Place::Gateway).unwrap();
        assert!(places[..turn].iter().all(|&p| p == Place::Graveyard));
        assert!(places[turn..].iter().all(|&p| p == Place::Gateway));
        for b in SCENE_ONE {
            // The bitmap font is plain ASCII.
            assert!(b.line.is_ascii() && b.cue.is_ascii(), "{:?}", b.line);
            assert!(b.waits() == !b.line.is_empty(), "lines wait for the player, action does not: {b:?}");
        }
    }

    #[test]
    fn scene_two_runs_from_the_embrace_through_the_light_and_the_words_to_the_credits() {
        let shots: Vec<Shot> = SCENE_TWO.iter().map(|b| b.shot).collect();
        assert_eq!(shots, vec![Shot::Embrace, Shot::Light, Shot::Rise, Shot::White, Shot::Words, Shot::Credits]);
        for b in SCENE_TWO {
            assert_eq!(b.shot.scene(), Scene::Two);
            assert!(!b.waits() && b.line.is_empty(), "Scene II plays by itself: {b:?}");
        }
        assert!(SCENE_ONE.iter().all(|b| b.shot.scene() == Scene::One));
        // Each moment inside a beat falls inside it, in order.
        let hold = |s: Shot| SCENE_TWO[first(s).unwrap()].hold as f32;
        let in_order = |times: &[f32]| times.windows(2).all(|w| w[0] < w[1]);
        assert!(in_order(&[ascent::RUN, ascent::KNEEL, ascent::REACH, hold(Shot::Embrace)]));
        assert!(in_order(&[ascent::LIGHT_FAR, ascent::VOICES, ascent::LIGHT_TOUCH, ascent::LOOK_UP, ascent::STAND, hold(Shot::Light)]));
        assert!(in_order(&[ascent::LIFT, ascent::WHITEOUT, ascent::TOP, hold(Shot::Rise)]));
        assert!(ascent::WORDS_AT.windows(2).all(|w| w[1] - w[0] >= 3.5), "each line held long enough to read");
        assert!(in_order(&[ascent::WORDS_AT[5] + ascent::LINE_IN + 4.0, ascent::WORDS_FADE, hold(Shot::Words) - 2.0]));
        assert!(in_order(&[ascent::ROLL_FROM + ascent::ROLL_SECONDS, ascent::THE_END, hold(Shot::Credits) - 4.0]));
        // The platform rises from the bottom to the top, never back.
        assert_eq!(rise(0.0), 0.0);
        assert!((rise(ascent::TOP) - 1.0).abs() < 1e-6);
        let mut last = 0.0;
        for k in 0..200 {
            let r = rise(k as f32 * 0.1);
            assert!(r >= last);
            last = r;
        }
        // Times and beats agree both ways.
        for (i, b) in SCENE_TWO.iter().enumerate() {
            let at = Scene::Two.time_at(i, b.hold * 0.5);
            assert_eq!(Scene::Two.beat_at(at), (i, b.hold * 0.5));
        }
        assert_eq!(Scene::Two.beat_at(1e6).0, SCENE_TWO.len() - 1);
    }

    #[test]
    fn the_words_say_there_was_never_a_devil_and_the_good_and_bad_are_within() {
        assert_eq!(WORDS[0], "There was never a devil.");
        let all = WORDS.join(" ");
        assert!(all.contains("The good and the bad") && all.contains("within"));
        assert!(all.is_ascii());
        assert!(STANZA_BREAKS.iter().all(|&i| i > 0 && i < WORDS.len()));
        let credits = credits();
        assert_eq!(credits.first(), Some(&Credit::Title));
        let text: Vec<String> = credits
            .iter()
            .map(|c| match c {
                Credit::Heading(h) => h.to_string(),
                Credit::Name(a, b) => format!("{a} {b}"),
                Credit::Line(l) => l.clone(),
                _ => String::new(),
            })
            .collect();
        let has = |s: &str| text.iter().any(|t| t.contains(s));
        for name in ["Ragnar", "Mehmed", "Miyamoto", "The Old Man", "Quitmer", "the friend who was taken", "THE PIT", "Mammon", "Asmodeus"]
        {
            assert!(has(name), "{name} missing from the credits");
        }
        assert_eq!(text.last().map(String::as_str), Some("Made with Claude"));
        assert!(text.iter().all(|t| t.is_ascii()));
    }

    /// FINALE.md §5: there is no Devil, and no lord of devils. Krampus is simply Krampus.
    #[test]
    fn nothing_in_the_story_or_the_old_mans_words_names_a_lord_of_devils() {
        // (Spelt in pieces, so this test does not find itself.)
        let banned = [["sat", "an"].concat(), ["his ", "father"].concat(), ["krampus is ", "the son"].concat()];
        let sources = [
            ("logic/story.rs", include_str!("story.rs")),
            ("logic/guide.rs", include_str!("guide.rs")),
            ("game/story.rs", include_str!("../game/story.rs")),
            ("game/ending.rs", include_str!("../game/ending.rs")),
            ("game/tour.rs", include_str!("../game/tour.rs")),
            ("game/hub.rs", include_str!("../game/hub.rs")),
            ("render/hud/story.rs", include_str!("../render/hud/story.rs")),
            ("render/hud/ending.rs", include_str!("../render/hud/ending.rs")),
        ];
        for (name, text) in sources {
            let low = text.to_lowercase();
            for b in &banned {
                assert!(!low.contains(b.as_str()), "{name} still says {b:?}");
            }
        }
        // And what the game shows: every line of both scenes, the road, the words, the credits, every page of the guide.
        let mut shown: Vec<String> = SCENE_ONE.iter().chain(SCENE_TWO).map(|b| b.line.to_string()).collect();
        shown.extend([ROUTE.goal, ROUTE.quarry, ROUTE.destination, ROUTE.reveal, objective()].map(String::from));
        shown.extend(WORDS.map(String::from));
        shown.extend(credits().into_iter().map(|c| format!("{c:?}")));
        use crate::logic::guide::{guide, GuidePlace};
        use crate::logic::sins::SINS;
        let mut places = vec![GuidePlace::Garden, GuidePlace::Cisterns, GuidePlace::BoneVaults, GuidePlace::Hub, GuidePlace::Gym];
        places.extend((0..3).map(GuidePlace::Cyclops));
        places.extend(SINS.iter().flat_map(|&s| [GuidePlace::Realm(s), GuidePlace::Court(s)]));
        for place in places {
            let g = guide(place, crate::logic::heroes::HeroKind::Ragnar);
            shown.extend(std::iter::once(g.title).chain(g.tale).chain(g.task).chain(g.watch));
        }
        for line in shown {
            let low = line.to_lowercase();
            assert!(banned.iter().all(|b| !low.contains(b.as_str())), "{line}");
        }
    }

    #[test]
    fn depths_count_in_roman_numerals() {
        assert_eq!(roman(1), "I");
        assert_eq!(roman(4), "IV");
        assert_eq!(roman(9), "IX");
        assert_eq!(roman(14), "XIV");
        assert_eq!(depth_line(0, false), "DEPTH I OF THE UNDERWORLD");
        assert!(depth_line(2, true).starts_with("DEPTH III"));
    }
}
