//! The four archangels. Their shrines stand in every dungeon: a little angel
//! of marble on a plinth, flags about it in the colour of whichever archangel
//! it honours now. A blow switches it to the next; stepping onto it takes
//! that archangel's blessing, a power used with LT (or R) a few times. Each
//! archangel gives once a level; when all four have, the shrines stand grey.
//!
//! - Michael (blue): four bolts of blue fire; a foe struck turns to fight on
//!   your side for ten seconds.
//! - Gabriel (green): three thunderclaps; every foe within ten tiles is
//!   struck dead, a boss loses five.
//! - Raphael (purple): two beams; everything in their path dies, a boss loses ten.
//! - Uriel (red): one great wave rolling out from you, killing all it
//!   passes; a boss loses forty. Uriel grants it only once in ten minutes.

/// One of the four archangels.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Archangel {
    Michael,
    Gabriel,
    Raphael,
    Uriel,
}

/// In the order a shrine switches through them.
pub const ARCHANGELS: [Archangel; 4] = [Archangel::Michael, Archangel::Gabriel, Archangel::Raphael, Archangel::Uriel];

/// Gabriel's thunder strikes every foe within this many tiles of the hero.
pub const THUNDER_RADIUS: f64 = 10.0;
/// Raphael's beam reaches this far, stopped only by solid rock, and kills within this much of its line.
pub const BEAM_REACH: f64 = 16.0;
pub const BEAM_HALF_WIDTH: f64 = 0.9;
/// Uriel's wave rolls out this far, this fast (tiles, tiles a second).
pub const WAVE_RADIUS: f64 = 20.0;
pub const WAVE_SPEED: f64 = 14.0;
/// Seconds of play between two of Uriel's blessings.
pub const URIEL_REST: f64 = 600.0;
/// Seconds a foe turned by Michael's fire fights on the hero's side.
pub const ALLY_SECONDS: f64 = 10.0;
/// A turned foe kills a foe it touches within this many tiles.
pub const ALLY_REACH: f64 = 0.7;

impl Archangel {
    pub fn name(self) -> &'static str {
        match self {
            Archangel::Michael => "Michael",
            Archangel::Gabriel => "Gabriel",
            Archangel::Raphael => "Raphael",
            Archangel::Uriel => "Uriel",
        }
    }

    /// Their colour: the flags, the halo, the power itself.
    pub fn color(self) -> u32 {
        match self {
            Archangel::Michael => 0x3a86ff,
            Archangel::Gabriel => 0x3ed46a,
            Archangel::Raphael => 0xa65cff,
            Archangel::Uriel => 0xff3a30,
        }
    }

    /// How many times the blessing can be used.
    pub fn charges(self) -> u8 {
        match self {
            Archangel::Michael => 4,
            Archangel::Gabriel => 3,
            Archangel::Raphael => 2,
            Archangel::Uriel => 1,
        }
    }

    /// What one use of it does to a boss.
    pub fn boss_damage(self) -> i32 {
        match self {
            Archangel::Michael => 3,
            Archangel::Gabriel => 5,
            Archangel::Raphael => 10,
            Archangel::Uriel => 40,
        }
    }

    /// The power in a few words, for the HUD and the tour.
    pub fn power(self) -> &'static str {
        match self {
            Archangel::Michael => "Blue fire: the foe it strikes fights for you",
            Archangel::Gabriel => "Thunder: every foe near you dies",
            Archangel::Raphael => "A beam: all in its path dies",
            Archangel::Uriel => "A great wave: all it passes dies",
        }
    }

    /// The next archangel a shrine switches to.
    pub fn next(self) -> Archangel {
        ARCHANGELS[(self.index() + 1) % ARCHANGELS.len()]
    }

    pub fn index(self) -> usize {
        ARCHANGELS.iter().position(|&a| a == self).unwrap_or(0)
    }
}

/// An archangel's blessing carried by the hero: whose, and how many uses are left.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Blessing {
    pub angel: Archangel,
    pub charges: u8,
}

impl Blessing {
    pub fn of(angel: Archangel) -> Self {
        Self { angel, charges: angel.charges() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_archangels_in_their_colours_with_their_uses() {
        let names: Vec<_> = ARCHANGELS.iter().map(|a| a.name()).collect();
        assert_eq!(names, ["Michael", "Gabriel", "Raphael", "Uriel"]);
        let uses: Vec<_> = ARCHANGELS.iter().map(|a| a.charges()).collect();
        assert_eq!(uses, [4, 3, 2, 1]);
        assert_eq!(Archangel::Uriel.next(), Archangel::Michael);
        assert_eq!((Archangel::Gabriel.boss_damage(), Archangel::Raphael.boss_damage(), Archangel::Uriel.boss_damage()), (5, 10, 40));
        // Blue, green, purple, red.
        let c = |a: Archangel| a.color();
        let rgb = |x: u32| ((x >> 16) & 255, (x >> 8) & 255, x & 255);
        let (mr, _, mb) = rgb(c(Archangel::Michael));
        let (gr, gg, gb) = rgb(c(Archangel::Gabriel));
        let (rr, _, rb) = rgb(c(Archangel::Raphael));
        let (ur, ug, ub) = rgb(c(Archangel::Uriel));
        assert!(mb > mr && gg > gr && gg > gb && rb > 150 && rr > 150 && ur > ug && ur > ub);
    }
}
