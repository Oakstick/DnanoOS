//! The story. Scene I, "The Threshold", plays after Play on the title, and the
//! road the whole adventure follows from there is set out here too.
//!
//! Four friends play by the iron gates of an old graveyard at dusk until
//! Krampus bursts out of the fog and drags one of them back into the graves.
//! The other three chase him through the gates and fall straight through into
//! the underworld, where an old gatekeeper and his dog Quitmer keep an iron
//! door. The two of them slept while Krampus slipped past; out of guilt the
//! old man lends the boys the souls of three great warriors (Ragnar the
//! viking, Mehmed the janissary and Miyamoto the samurai), swears to ask the
//! archangels to send them help, and promises to come whenever they call. From there the road only goes down, depth
//! by depth, to Satan himself: Krampus is his son.

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
}

impl Shot {
    pub fn place(self) -> Place {
        match self {
            Shot::Title | Shot::Establish | Shot::Taken | Shot::Scream | Shot::Pursuit => Place::Graveyard,
            _ => Place::Gateway,
        }
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
/// The last words: the road down to Satan, and why.
pub const ROUTE_LINE: &str =
    "The boys took up the warrior souls and went down into the dark, depth by depth, after their stolen friend. The trail led to Satan himself... for Krampus is his son.";

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
    Beat::say(Shot::OldMan, Speaker::OldMan, "turning to the boys", "Living children... at my door? No. No, no. You cannot be here. Nothing warm passes this gate."),
    Beat::say(Shot::Boys, Speaker::Boys, "breathless", "A monster came out of the fog - horns, chains, bells... It dragged our friend into the graves!"),
    Beat::say(Shot::OldMan, Speaker::OldMan, "", "Horns and bells... Krampus. So he slipped past me. I closed my eyes for one moment... one moment."),
    Beat::say(Shot::Boys, Speaker::Boys, "", "Then help us! We have to get him back!"),
    Beat::say(Shot::OldMan, Speaker::OldMan, "", "Beyond this door lies the world of the dead. The living do not walk there... not for long."),
    Beat::say(Shot::Boys, Speaker::Boys, "angrily", "You were supposed to guard it! How could you let him out?"),
    Beat::say(Shot::OldMan, Speaker::OldMan, "looking away", "Quitmer and I have kept this door a thousand years... and tonight, of all nights, we slept."),
    Beat::say(Shot::Boys, Speaker::Boys, "", "We are not going home without him. There has to be a way!"),
    Beat::say(Shot::OldMan, Speaker::OldMan, "gravely", "There is one way. Only one. But hear me: die down there, and you will never see the sun again."),
    Beat::say(Shot::Boys, Speaker::Boys, "", "Then we will not die. Whatever it takes - we are going."),
    Beat::say(Shot::Souls, Speaker::OldMan, "", SOULS_LINE),
    Beat::say(Shot::Boys, Speaker::Boys, "", "We will. For our friend."),
    Beat::say(Shot::Promise, Speaker::OldMan, "heavily", PROMISE_LINE),
    Beat::say(Shot::Descent, Speaker::Narrator, "", ROUTE_LINE),
];

/// The first beat of a shot, if the scene has one.
pub fn first(shot: Shot) -> Option<usize> {
    SCENE_ONE.iter().position(|b| b.shot == shot)
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

// ----------------------------------------------------------------- the road

/**
 * The road the adventure takes. Past the gatekeeper's door each dungeon lies
 * a depth further down the underworld than the last, a Cyclops keeps the way
 * down out of each, and at the very bottom Satan waits: Krampus carried the
 * boys' friend down to his father.
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
    destination: "Satan, at the bottom of the underworld",
    reveal: "Krampus is the son of Satan",
};

/// Roman numerals, for the depths.
pub fn roman(mut n: usize) -> String {
    const TABLE: [(usize, &str); 13] =
        [(1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"), (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")];
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
    "Follow Krampus down through the underworld to his father, Satan, and bring your friend home."
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
    fn the_old_man_promises_help_and_the_road_leads_down_to_satan() {
        let promise = SCENE_ONE[first(Shot::Promise).unwrap()];
        assert_eq!(promise.speaker, Speaker::OldMan);
        assert!(promise.line.contains("call me"));
        assert!(promise.line.contains("guilt"));
        assert!(promise.line.contains("I will ask the archangels to send you help"));
        let last = SCENE_ONE.last().unwrap();
        assert_eq!(last.shot, Shot::Descent);
        assert!(last.line.contains("Satan") && last.line.contains("Krampus is his son"));
        assert!(ROUTE.reveal.contains("son of Satan"));
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
    fn depths_count_in_roman_numerals() {
        assert_eq!(roman(1), "I");
        assert_eq!(roman(4), "IV");
        assert_eq!(roman(9), "IX");
        assert_eq!(roman(14), "XIV");
        assert_eq!(depth_line(0, false), "DEPTH I OF THE UNDERWORLD");
        assert!(depth_line(2, true).starts_with("DEPTH III"));
    }
}
