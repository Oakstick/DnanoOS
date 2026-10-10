//! The tale between the two scenes (STORY.md, backlog B07 and B08): who the boys are in play, what
//! each says as he comes into a place, and Toby's trail.
//!
//! Scene I sends three boys down after their friend Toby, each carrying a warrior's soul; Scene II
//! brings Toby home. In between the boys stay in play: the boy whose warrior is played speaks one
//! line as he comes into each place (`arrival`), in the voices' panel with his face and his soul's
//! colour; his outline shows in the warrior's soul-glow (the view); and Toby left something of his
//! in every realm (`Token`), which the hero picks up as he walks over it and keeps.
//!
//! Pure data. Nothing here touches the fight: a token is a flag (`Tokens`), nothing more.

use super::heroes::HeroKind;
use super::sins::{Sin, SINS};

/// The friend Krampus took (D1: after the Book of Tobit, where the archangel Raphael walks beside
/// young Tobias on his road).
pub const TOBY: &str = "Toby";
/// His colour: his red bobble cap and scarf.
pub const TOBY_COLOR: u32 = 0xf06a52;

/// The boy who carries `hero`'s soul, as the cast's kid he is (`render::cast::kid`): in Scene I the
/// souls rise Mehmed, Ragnar, Miyamoto before the three boys left to right, and go into them.
/// (Kid 0 is Toby.)
pub fn boy_of(hero: HeroKind) -> usize {
    match hero {
        HeroKind::Mehmed => 1,
        HeroKind::Ragnar => 2,
        HeroKind::Miyamoto => 3,
    }
}

/// The warrior whose soul kid `kid` (1 to 3) carries.
pub fn warrior_of(kid: usize) -> HeroKind {
    match kid {
        1 => HeroKind::Mehmed,
        3 => HeroKind::Miyamoto,
        _ => HeroKind::Ragnar,
    }
}

/// The boys are not named yet (STORY.md): each goes by the warrior he carries.
pub fn boy_tag(hero: HeroKind) -> &'static str {
    match hero {
        HeroKind::Ragnar => "RAGNAR'S BOY",
        HeroKind::Mehmed => "MEHMED'S BOY",
        HeroKind::Miyamoto => "MIYAMOTO'S BOY",
    }
}

/// The colour of `hero`'s soul, as it rose in Scene I: jade for the janissary, steel blue for the
/// viking, rose for the samurai. His boy's name and words are in it, and his outline in the glow.
pub fn soul_color(hero: HeroKind) -> u32 {
    match hero {
        HeroKind::Mehmed => 0x7ae8c8,
        HeroKind::Ragnar => 0x7ab4ff,
        HeroKind::Miyamoto => 0xff98b0,
    }
}

// ------------------------------------------------------------------ the boys' lines

/// A place the boys come into.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Place {
    /// Dungeon I, the Garden of Krampus: the first depth.
    Garden,
    /// The Portals of Hell.
    Hub,
    /// A sin's realm.
    Realm(Sin),
    /// The Pit, from its first level.
    Pit,
    /// The bottom of the Pit: Krampus's lair, Toby in the cage.
    Lair,
}

/// What the boy whose warrior is `hero` says as he comes into `place`: one short line, of fear,
/// resolve or a memory of Toby, tied to the place (each realm to its sin).
pub fn arrival(hero: HeroKind, place: Place) -> &'static str {
    let k = match hero {
        HeroKind::Mehmed => 0,
        HeroKind::Ragnar => 1,
        HeroKind::Miyamoto => 2,
    };
    let lines: [&str; 3] = match place {
        Place::Garden => [
            "His bells. I can hear Krampus's bells. We're coming, Toby.",
            "He dragged Toby through here. We're taking him back.",
            "It's like the park at night. Only the statues watch you.",
        ],
        Place::Hub => [
            "Seven doors. Seven keys. One Toby. Pick a door.",
            "Lava everywhere. Toby would call this the best bad day ever.",
            "Toby, I hope you're not as scared as I am.",
        ],
        Place::Realm(Sin::Pride) => [
            "A hundred of me in the glass, and not one of them is Toby. Keep walking.",
            "I always had to win. Toby always let me. I never said thanks.",
            "The mirrors make me look big and brave. I'm not. I'm going anyway.",
        ],
        Place::Realm(Sin::Envy) => [
            "I used to wish I had Toby's bike. Now I just wish I had Toby.",
            "Everyone liked Toby best. I minded that once. Not any more.",
            "Toby shared everything. Even his last sweet. Even with me.",
        ],
        Place::Realm(Sin::Wrath) => [
            "Stay cool. Angry kids make mistakes. Toby told me that.",
            "I'm so angry I could burn too. No. Toby needs me thinking.",
            "It's so hot. Toby hates the heat. He'd be moaning by now.",
        ],
        Place::Realm(Sin::Sloth) => [
            "Everything here wants me to lie down. I won't. Not till he's home.",
            "My legs are so heavy. Toby would race me through this. Come on, legs.",
            "So quiet. So grey. If I stop, I'll forget why I came. Toby. Toby. Toby.",
        ],
        Place::Realm(Sin::Greed) => [
            "All this gold. I'd swap every coin of it for one more game with Toby.",
            "Don't touch the gold. We came for Toby, not for this.",
            "Toby would fill his pockets and give it all away by tea time.",
        ],
        Place::Realm(Sin::Gluttony) => [
            "I'm starving. Toby always saved me half his sandwich.",
            "Don't eat anything. Don't even look at it.",
            "It smells like Mum's kitchen, and then it doesn't. Stay close, Toby.",
        ],
        Place::Realm(Sin::Lust) => [
            "Pretty roses, sharp thorns. Toby would say don't touch. So I won't.",
            "Something keeps calling me to turn round. I won't look back.",
            "It smells sweet. It wants me to forget. I won't forget you, Toby.",
        ],
        Place::Pit => [
            "Down and down. He's at the bottom. So that's where we go.",
            "No devil. Just a hole. And Toby at the bottom of it.",
            "Each floor is colder. Hold on, Toby. Nearly there.",
        ],
        Place::Lair => ["TOBY! We're here! We came for you!", "TOBY! Hold on! We're getting you out!", "TOBY! It's us! We're here!"],
    };
    lines[k]
}

/// Toby, from the cage, as the boys come into the lair.
pub const TOBY_IN_THE_CAGE: &str = "I knew you'd come. I knew it!";

// ------------------------------------------------------------------ Toby's trail

/// One of Toby's things, left in a realm on the way down (B08). Each lies on the ground of its
/// realm, off nothing and in nobody's way; he picks it up by walking over it, and keeps it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Token {
    /// Pride: a drawing of the four of them, holding hands, signed TOBY.
    Drawing,
    /// Envy: his tin soldier, the one with the chipped hat that they all took turns with.
    Soldier,
    /// Wrath: one of his red mittens, singed at the end.
    Mitten,
    /// Sloth: his stub of chalk, by a stone where he wrote TOBY WAS HERE and an arrow down.
    Chalk,
    /// Greed: his football, the one from the graveyard.
    Ball,
    /// Gluttony: a bell he tore off Krampus's sack, so it would fall where they would find it.
    Bell,
    /// Lust: the end of his red scarf, caught on the thorns.
    Scarf,
}

/// The seven, in the order of the sins round the hub.
pub const TOKENS: [Token; 7] = [Token::Drawing, Token::Soldier, Token::Mitten, Token::Chalk, Token::Ball, Token::Bell, Token::Scarf];

impl Token {
    /// The token left in `sin`'s realm.
    pub fn of(sin: Sin) -> Token {
        TOKENS[sin.index()]
    }

    /// The realm it lies in.
    pub fn sin(self) -> Sin {
        SINS[self.index()]
    }

    pub fn index(self) -> usize {
        TOKENS.iter().position(|&t| t == self).unwrap_or(0)
    }

    /// Its bit among those held (`Tokens`).
    pub fn bit(self) -> u8 {
        1 << self.index()
    }

    /// Its name, as the pause and the book show it.
    pub fn name(self) -> &'static str {
        match self {
            Token::Drawing => "His drawing",
            Token::Soldier => "His tin soldier",
            Token::Mitten => "His mitten",
            Token::Chalk => "His chalk",
            Token::Ball => "His football",
            Token::Bell => "A bell off the sack",
            Token::Scarf => "His scarf",
        }
    }

    /// The banner as he picks it up.
    pub fn found(self) -> &'static str {
        match self {
            Token::Drawing => "TOBY'S DRAWING",
            Token::Soldier => "TOBY'S TIN SOLDIER",
            Token::Mitten => "TOBY'S MITTEN",
            Token::Chalk => "TOBY'S CHALK",
            Token::Ball => "TOBY'S FOOTBALL",
            Token::Bell => "A BELL FROM KRAMPUS'S SACK",
            Token::Scarf => "TOBY'S SCARF",
        }
    }

    /// A line on it, for the book's page.
    pub fn note(self) -> &'static str {
        match self {
            Token::Drawing => "The four of us, holding hands. He drew me taller than I am, and himself the smallest.",
            Token::Soldier => "The chipped hat. We all wanted it, and he let us take turns.",
            Token::Mitten => "Red, like his hat. The fire got the end of it.",
            Token::Chalk => "He wrote on a stone with it: TOBY WAS HERE, and an arrow pointing down.",
            Token::Ball => "The ball from the graveyard. He held on to it all the way down.",
            Token::Bell => "He tore it off Krampus's sack, so it would fall where we would find it.",
            Token::Scarf => "The end of it, caught on the thorns. It still smells like his house.",
        }
    }
}

/// The tokens held, a bit each (`Token::bit`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Tokens(pub u8);

impl Tokens {
    pub fn has(self, t: Token) -> bool {
        self.0 & t.bit() != 0
    }

    pub fn with(self, t: Token) -> Tokens {
        Tokens(self.0 | t.bit())
    }

    pub fn count(self) -> usize {
        (self.0 & 0x7f).count_ones() as usize
    }

    /// Those held, in the order of the sins.
    pub fn held(self) -> impl Iterator<Item = Token> {
        TOKENS.into_iter().filter(move |&t| self.has(t))
    }
}

/// What Toby says as the cage opens and he sees what the hero carries (the reunion).
pub fn toby_recognises(tokens: Tokens) -> String {
    let short = |t: Token| match t {
        Token::Drawing => "my drawing",
        Token::Soldier => "my soldier",
        Token::Mitten => "my mitten",
        Token::Chalk => "my chalk",
        Token::Ball => "my ball",
        Token::Bell => "the bell",
        Token::Scarf => "my scarf",
    };
    // The ball first: it is the one from the graveyard.
    let mut held: Vec<Token> = tokens.held().collect();
    held.sort_by_key(|&t| if t == Token::Ball { 0 } else { 1 + t.index() });
    match held.len() {
        0 => "You came. You really came!".to_string(),
        1 => format!("You came! And you found {}!", short(held[0])),
        7 => "You found everything! You followed my trail all the way down!".to_string(),
        n => {
            let named: Vec<&str> = held.iter().take(n.min(3)).map(|&t| short(t)).collect();
            let list = if n > 3 {
                format!("{}...", named.join(", "))
            } else {
                format!("{} and {}", named[..named.len() - 1].join(", "), named[named.len() - 1])
            };
            format!("You came! You found {list} - you followed my trail!")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::heroes::ROSTER;

    #[test]
    fn every_boy_has_a_line_for_every_place_short_and_plain() {
        let mut places = vec![Place::Garden, Place::Hub, Place::Pit, Place::Lair];
        places.extend(SINS.iter().map(|&s| Place::Realm(s)));
        let mut all = std::collections::HashSet::new();
        for place in &places {
            for hero in ROSTER {
                let l = arrival(hero, *place);
                assert!(!l.is_empty() && l.chars().count() <= 80, "{hero:?} {place:?}: {l}");
                assert!(l.is_ascii(), "{l}");
                assert!(!l.to_lowercase().contains(&["sat", "an"].concat()));
                assert!(all.insert(l), "a line said twice: {l}");
            }
        }
        // 7 realms x 3 boys, and the Garden, the hub, the Pit and the lair.
        assert_eq!(all.len(), 33);
        // Every realm's lines think of Toby, or of what the realm wants of them.
        for sin in SINS {
            let toby = ROSTER.iter().filter(|&&h| arrival(h, Place::Realm(sin)).contains("Toby")).count();
            assert!(toby >= 2, "{sin:?}: Toby in {toby} of 3");
        }
    }

    #[test]
    fn each_boy_carries_one_warrior_and_toby_is_the_fourth() {
        for hero in ROSTER {
            assert_eq!(warrior_of(boy_of(hero)), hero);
            assert!((1..=3).contains(&boy_of(hero)));
        }
    }

    #[test]
    fn one_token_a_realm_and_toby_knows_them() {
        for sin in SINS {
            assert_eq!(Token::of(sin).sin(), sin);
        }
        let mut t = Tokens::default();
        assert_eq!(toby_recognises(t), "You came. You really came!");
        t = t.with(Token::Scarf);
        assert!(toby_recognises(t).contains("my scarf"));
        t = t.with(Token::Ball).with(Token::Soldier);
        let said = toby_recognises(t);
        assert!(said.starts_with("You came! You found my ball, "), "{said}");
        assert_eq!(t.count(), 3);
        for tok in TOKENS {
            t = t.with(tok);
        }
        assert!(toby_recognises(t).contains("everything"));
        assert!(TOKENS.iter().all(|t| t.note().len() < 100 && t.found().is_ascii()));
    }
}
