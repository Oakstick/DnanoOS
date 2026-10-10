//! The scenes at Hızır's (B07): after each prince falls, back at the old man's workshop, a short
//! scene with the three boys and the souls of the three warriors standing behind them, Hızır and
//! Quitmer. Each reacts to the prince just beaten and to the thing of Toby's just found (B08), and
//! the three grow closer. Skippable, as every scene; the wares open after it.
//!
//! Every one has the same shape (so the camera and the cast play them all the same way):
//!
//! 0. **Gather** (action): the room, the fire, the boys round the anvil with Toby's thing on it.
//! 1. Hızır speaks of the prince.
//! 2. **Show**: the boy whose warrior was played speaks of the thing he found (the camera on it).
//! 3. and 4. The other two boys.
//! 5. The last word (Hızır's, or a boy's).
//! 6. **Leave** (action): the camera draws back; the wares come next.
//!
//! The boys speak by their part, not their name: `Speaker::Boy(0)` is the boy whose warrior is
//! played (he found it), 1 and 2 the other two in the order of the cast (`kid_of_role`).

use super::heroes::HeroKind;
use super::sins::Sin;
use super::story::{Beat, Card, Shot, Speaker};
use super::tale::boy_of;

/// The kid (`render::cast::kid`) who speaks as the boys' part `role` when `hero` is played: 0 the
/// hero's own boy, 1 and 2 the other two, lowest first.
pub fn kid_of_role(hero: HeroKind, role: u8) -> usize {
    let mine = boy_of(hero);
    if role == 0 {
        return mine;
    }
    let others: Vec<usize> = (1..=3).filter(|&k| k != mine).collect();
    others[(role as usize - 1).min(1)]
}

/// The card as the scene opens.
const fn card(sub: &'static str) -> Option<Card> {
    Some(Card { title: "AT HIZIR'S", sub })
}

use Shot::{Gather, Leave, Show, Talk};
use Speaker::{Boy, OldMan};

const GATHER: f64 = 4.2;
const LEAVE: f64 = 3.0;

/// After Lucifer (Pride): the drawing.
const PRIDE: &[Beat] = &[
    Beat::action(Gather, GATHER, card("Lucifer is beaten")),
    Beat::say(Talk, OldMan, "warmly", "Lucifer is down. He wanted every eye on him, and in the end no one was looking."),
    Beat::say(Show, Boy(0), "unfolding it", "Toby left this between the mirrors. It's us. All four of us, holding hands."),
    Beat::say(Talk, Boy(1), "grinning", "He drew me taller than I am. He always does that."),
    Beat::say(Talk, Boy(2), "softly", "And he drew himself the smallest. Typical Toby."),
    Beat::say(Talk, Boy(0), "firmly", "When we find him, he can draw a new one. All four of us, going home."),
    Beat::action(Leave, LEAVE, None),
];

/// After Leviathan (Envy): the tin soldier.
const ENVY: &[Beat] = &[
    Beat::action(Gather, GATHER, card("Leviathan is beaten")),
    Beat::say(Talk, OldMan, "gently", "Leviathan wanted whatever anyone else had, and it ate him up from inside."),
    Beat::say(Show, Boy(0), "", "Look. Toby's tin soldier. The one with the chipped hat."),
    Beat::say(Talk, Boy(1), "quietly", "We all wanted that soldier. He let us take turns."),
    Beat::say(Talk, Boy(2), "", "I chipped the hat. He said it made him look braver."),
    Beat::say(Talk, OldMan, "warmly", "A friend who shares his best thing... that is a friend worth going all this way for."),
    Beat::action(Leave, LEAVE, None),
];

/// After Amon (Wrath): the mitten.
const WRATH: &[Beat] = &[
    Beat::action(Gather, GATHER, card("Amon is beaten")),
    Beat::say(Talk, OldMan, "gravely", "Amon lived on anger. You would not give him any, so he starved."),
    Beat::say(Show, Boy(0), "", "His mitten. Red, like his hat. The fire got the end of it."),
    Beat::say(Talk, Boy(1), "looking away", "I was so angry in there. I wanted to smash everything."),
    Beat::say(Talk, Boy(2), "", "Toby says you can be angry and still be kind. He's annoying like that."),
    Beat::say(Talk, Boy(1), "smiling", "...He's right, though."),
    Beat::action(Leave, LEAVE, None),
];

/// After Belphegor (Sloth): the chalk.
const SLOTH: &[Beat] = &[
    Beat::action(Gather, GATHER, card("Belphegor sleeps for good")),
    Beat::say(Talk, OldMan, "gently", "The grey wanted you to lie down and never get up. You did not."),
    Beat::say(Show, Boy(0), "", "Toby's chalk. He wrote on a stone with it: TOBY WAS HERE. And an arrow, pointing down."),
    Beat::say(Talk, Boy(1), "", "He was still awake down there. Still thinking about us finding him."),
    Beat::say(Talk, Boy(2), "firmly", "Then we don't stop either. Not to sleep. Not for anything."),
    Beat::say(Talk, OldMan, "kindly", "Sit by my fire for a moment, then. Only a moment."),
    Beat::action(Leave, LEAVE, None),
];

/// After Mammon (Greed): the football.
const GREED: &[Beat] = &[
    Beat::action(Gather, GATHER, card("Mammon is beaten")),
    Beat::say(Talk, OldMan, "", "Mammon is beaten, and all his gold could not buy him one more minute. What did you bring back?"),
    Beat::say(Show, Boy(0), "", "Not gold. This. Toby's ball. It was lying in the coins like it was nothing."),
    Beat::say(Talk, Boy(1), "", "That's the ball from the graveyard. He held on to it the whole way down."),
    Beat::say(Talk, Boy(2), "grinning", "He kicks it at my head every single day. I want him to do it again."),
    Beat::say(Talk, OldMan, "warmly", "Then keep it safe. A boy who holds on to his ball has not given up."),
    Beat::action(Leave, LEAVE, None),
];

/// After Beelzebub (Gluttony): the bell off Krampus's sack.
const GLUTTONY: &[Beat] = &[
    Beat::action(Gather, GATHER, card("Beelzebub is beaten")),
    Beat::say(Talk, OldMan, "", "Beelzebub ate and ate, and he was never full. Now he is nothing at all."),
    Beat::say(Show, Boy(0), "", "This was in the feast. One of Krampus's bells, torn right off his sack."),
    Beat::say(Talk, Boy(1), "", "Toby did that. He tore it off so it would fall where we'd find it."),
    Beat::say(Talk, Boy(2), "softly", "He's leaving us a trail. Even in a sack. Even scared."),
    Beat::say(Talk, OldMan, "warmly", "Your friend is braver than he knows. So are the three of you."),
    Beat::action(Leave, LEAVE, None),
];

/// After Asmodeus (Lust): the scarf. (The Book of Tobit: Raphael walked beside young Tobias, and
/// Asmodeus was driven off.)
const LUST: &[Beat] = &[
    Beat::action(Gather, GATHER, card("Asmodeus is beaten")),
    Beat::say(
        Talk,
        OldMan,
        "gently",
        "Long ago a boy called Tobias drove Asmodeus off with smoke, and an angel walked beside him all the way.",
    ),
    Beat::say(Show, Boy(0), "", "His scarf. The end of it, caught on the thorns. It still smells like his house."),
    Beat::say(Talk, Boy(1), "quietly", "Everything in there wanted me to turn round and stay. I nearly did."),
    Beat::say(Talk, Boy(2), "", "Then I thought of Toby's silly laugh. And I kept going."),
    Beat::say(Talk, OldMan, "smiling", "That Tobias had one angel. Your Toby has three friends. I like his chances."),
    Beat::action(Leave, LEAVE, None),
];

/// The scene after `sin`'s prince.
pub fn beats(sin: Sin) -> &'static [Beat] {
    match sin {
        Sin::Pride => PRIDE,
        Sin::Envy => ENVY,
        Sin::Wrath => WRATH,
        Sin::Sloth => SLOTH,
        Sin::Greed => GREED,
        Sin::Gluttony => GLUTTONY,
        Sin::Lust => LUST,
    }
}

/// Its name, over the letterbox.
pub fn title(sin: Sin) -> Card {
    Card {
        title: "AT HIZIR'S",
        sub: match sin {
            Sin::Pride => "AFTER LUCIFER",
            Sin::Envy => "AFTER LEVIATHAN",
            Sin::Wrath => "AFTER AMON",
            Sin::Sloth => "AFTER BELPHEGOR",
            Sin::Greed => "AFTER MAMMON",
            Sin::Gluttony => "AFTER BEELZEBUB",
            Sin::Lust => "AFTER ASMODEUS",
        },
    }
}

/// When things happen in the action beats, seconds from their start.
pub mod stage {
    /// The gathering: the picture comes up, Quitmer lifts his head, the token's glow swells.
    pub const FADE_IN: f32 = 0.9;
    pub const GLOW: f32 = 1.6;
    /// The leaving: the boys turn to the door.
    pub const TURN: f32 = 0.6;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::heroes::ROSTER;
    use crate::logic::sins::SINS;
    use crate::logic::tale::Token;

    #[test]
    fn seven_scenes_of_one_shape_each_speaking_of_its_prince_and_its_token() {
        let shape: Vec<(Shot, bool)> = beats(Sin::Greed).iter().map(|b| (b.shot, b.waits())).collect();
        for sin in SINS {
            let b = beats(sin);
            assert_eq!(b.iter().map(|b| (b.shot, b.waits())).collect::<Vec<_>>(), shape, "{sin:?}: the same shape");
            // The prince named (in the card or Hızır's first line), the token in the finder's line.
            let said: String = b.iter().map(|b| b.line).collect::<Vec<_>>().join(" ");
            let card = b[0].card.unwrap().sub;
            assert!(card.contains(sin.prince()) || said.contains(sin.prince()), "{sin:?}");
            let word = match Token::of(sin) {
                Token::Drawing => "us",
                Token::Soldier => "soldier",
                Token::Mitten => "mitten",
                Token::Chalk => "chalk",
                Token::Ball => "ball",
                Token::Bell => "bells",
                Token::Scarf => "scarf",
            };
            assert!(b[2].line.contains(word), "{sin:?}: the finder speaks of {word}");
            assert!(b[2].speaker == Speaker::Boy(0));
            // All three boys speak; a short scene (each line read at about 15 letters a second).
            for role in 0..3 {
                assert!(b.iter().any(|x| x.speaker == Speaker::Boy(role)), "{sin:?}: boy {role}");
            }
            let secs: f64 = b.iter().map(|x| x.hold + x.line.chars().count() as f64 / 15.0).sum();
            assert!((20.0..=40.0).contains(&secs), "{sin:?}: about {secs:.0} s");
            for x in b {
                assert!(x.line.chars().count() <= 110 && x.line.is_ascii(), "{sin:?}: {}", x.line);
                assert!(!x.line.to_lowercase().contains(&["dev", "il"].concat()), "{sin:?}");
            }
        }
    }

    #[test]
    fn the_three_parts_are_the_three_boys() {
        for hero in ROSTER {
            let mut kids: Vec<usize> = (0..3).map(|r| kid_of_role(hero, r)).collect();
            assert_eq!(kids[0], boy_of(hero));
            kids.sort();
            assert_eq!(kids, vec![1, 2, 3]);
        }
    }
}
