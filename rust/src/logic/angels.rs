//! The four archangels. Their shrines stand in every dungeon: a little angel
//! of marble on a plinth, turning through the four (a blow turns it on at
//! once). Stepping onto one, the first time, brings all four archangels to the
//! hero for the rest of the adventure; after that a shrine readies all four at
//! once. Each shrine gives once a level, then stands grey.
//!
//! The angel button (R / RT) calls the archangel chosen; the next button (F /
//! D-pad up) chooses the next of the four. Each power comes back 25 seconds
//! after it is called, each on its own.
//!
//! - Michael (blue): a bolt of blue fire; a foe struck fights on your side
//!   for a while (a duelist is awed instead: all its poise goes).
//! - Gabriel (green): a thunderclap; every foe near you is struck dead.
//! - Raphael (purple): a beam; everything in its path dies.
//! - Uriel (red): a great wave rolling out from you, killing all it passes.
//!
//! Where fights go by frame data (the realms, the courts, the Pit) the powers
//! are blows through the foes' fighters instead (`Archangel::brawl_blow`).
//! Before the first shrine only the patron sworn to at Hızır's is with him.

/// One of the four archangels.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Archangel {
    Michael,
    Gabriel,
    Raphael,
    Uriel,
}

/// In the order a shrine turns through them, the HUD shows them and the next button chooses them.
pub const ARCHANGELS: [Archangel; 4] = [Archangel::Michael, Archangel::Gabriel, Archangel::Raphael, Archangel::Uriel];

/// Seconds an archangel's power takes to come back after it is called (each on its own).
pub const RECHARGE: f64 = 25.0;
/// Seconds an archangel stays away when the Evil Eye steals his call (or until the thief falls).
pub const STOLEN_WAIT: f64 = 75.0;

/// Gabriel's thunder strikes every foe within this many tiles of the hero.
pub const THUNDER_RADIUS: f64 = 8.0;
/// Raphael's beam reaches this far, stopped only by solid rock, and kills within this much of its line.
pub const BEAM_REACH: f64 = 16.0;
pub const BEAM_HALF_WIDTH: f64 = 0.9;
/// Uriel's wave rolls out this far, this fast (tiles, tiles a second).
pub const WAVE_RADIUS: f64 = 14.0;
pub const WAVE_SPEED: f64 = 14.0;
/// Seconds a foe turned by Michael's fire fights on the hero's side.
pub const ALLY_SECONDS: f64 = 8.0;
/// A turned foe kills a foe it touches within this many tiles.
pub const ALLY_REACH: f64 = 0.7;
/// A prince (or Krampus) takes this share of a power's damage (all of its poise), and of the poise
/// Michael's awe takes.
pub const PRINCE_SHARE: f64 = 0.3;

impl Archangel {
    pub fn name(self) -> &'static str {
        match self {
            Archangel::Michael => "Michael",
            Archangel::Gabriel => "Gabriel",
            Archangel::Raphael => "Raphael",
            Archangel::Uriel => "Uriel",
        }
    }

    /// His initial, on the HUD's emblem: the four told apart without their colours.
    pub fn initial(self) -> &'static str {
        &self.name()[..1]
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

    /// What one call of it does to the Cyclops (exactly, dazed or not).
    pub fn boss_damage(self) -> i32 {
        match self {
            Archangel::Michael => 2,
            Archangel::Gabriel => 3,
            Archangel::Raphael => 5,
            Archangel::Uriel => 8,
        }
    }

    /// Where fights go by frame data: the blow it is through a foe's fighter, as damage and poise
    /// damage (Michael's fire awes instead: all the foe's poise, `Game::turn_foe`). A prince takes
    /// `PRINCE_SHARE` of the damage.
    pub fn brawl_blow(self) -> (f64, f64) {
        match self {
            Archangel::Michael => (0.0, 0.0),
            Archangel::Gabriel => (16.0, 40.0),
            Archangel::Raphael => (30.0, 50.0),
            Archangel::Uriel => (22.0, 45.0),
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

    /// The next of the four (a shrine's turn, the next button).
    pub fn next(self) -> Archangel {
        ARCHANGELS[(self.index() + 1) % ARCHANGELS.len()]
    }

    pub fn index(self) -> usize {
        ARCHANGELS.iter().position(|&a| a == self).unwrap_or(0)
    }
}

/// The archangels with the hero: whether all four have come (at a shrine), each one's wait until
/// he can be called again, the one chosen, and the calls made (for what watches them: Envy's
/// court, the Mire's giant, the Garden of Thorns' hedges).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Angels {
    /// All four are with him (from the first shrine on).
    pub granted: bool,
    /// Seconds until each can be called again (0: ready), in `ARCHANGELS`' order.
    pub wait: [f64; 4],
    /// The one the angel button calls.
    pub chosen: Archangel,
    /// Calls made so far (all, and each one's), and the last one called. They only count up.
    pub calls: u32,
    pub count: [u32; 4],
    pub last: Option<Archangel>,
}

impl Default for Angels {
    fn default() -> Self {
        Self { granted: false, wait: [0.0; 4], chosen: Archangel::Michael, calls: 0, count: [0; 4], last: None }
    }
}

impl Angels {
    /// An adventure begun or taken up again: all four with him or not yet, all ready, his patron
    /// chosen (the calls counted so far stay counted).
    pub fn restart(&mut self, granted: bool, patron: Option<Archangel>) {
        self.granted = granted;
        self.wait = [0.0; 4];
        self.chosen = patron.unwrap_or(Archangel::Michael);
    }

    /// Is `a` with him: all four after a shrine, and his patron always.
    pub fn has(&self, a: Archangel, patron: Option<Archangel>) -> bool {
        self.granted || patron == Some(a)
    }

    /// Any archangel at all with him.
    pub fn any(&self, patron: Option<Archangel>) -> bool {
        self.granted || patron.is_some()
    }

    /// Can `a` be called now.
    pub fn ready(&self, a: Archangel, patron: Option<Archangel>) -> bool {
        self.has(a, patron) && self.wait[a.index()] <= 0.0
    }

    /// The one the button would call: the chosen one if he is with him, else his patron.
    pub fn current(&self, patron: Option<Archangel>) -> Option<Archangel> {
        if self.has(self.chosen, patron) {
            Some(self.chosen)
        } else {
            patron
        }
    }

    /// The next button: the next of the four that is with him (with only the patron, nothing moves).
    pub fn choose_next(&mut self, patron: Option<Archangel>) -> bool {
        let Some(from) = self.current(patron) else { return false };
        let mut a = from.next();
        while a != from {
            if self.has(a, patron) {
                self.chosen = a;
                return true;
            }
            a = a.next();
        }
        self.chosen = from;
        false
    }

    /// `a` is called: he rests `RECHARGE` seconds.
    pub fn call(&mut self, a: Archangel) {
        self.wait[a.index()] = RECHARGE;
        self.calls = self.calls.wrapping_add(1);
        self.count[a.index()] += 1;
        self.last = Some(a);
    }

    /// All four ready at once (a shrine after the first).
    pub fn refresh(&mut self) {
        self.wait = [0.0; 4];
    }

    /// `dt` seconds of play: the waits run down. Returns those who have just come back.
    pub fn tick(&mut self, dt: f64) -> Vec<Archangel> {
        let mut back = Vec::new();
        for a in ARCHANGELS {
            let w = &mut self.wait[a.index()];
            if *w > 0.0 {
                *w = (*w - dt).max(0.0);
                if *w == 0.0 {
                    back.push(a);
                }
            }
        }
        back
    }

    /// How far `a`'s power has come back, 0 to 1 (1: ready).
    pub fn fill(&self, a: Archangel) -> f64 {
        (1.0 - self.wait[a.index()] / RECHARGE).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_archangels_in_their_colours() {
        let names: Vec<_> = ARCHANGELS.iter().map(|a| a.name()).collect();
        assert_eq!(names, ["Michael", "Gabriel", "Raphael", "Uriel"]);
        let initials: Vec<_> = ARCHANGELS.iter().map(|a| a.initial()).collect();
        assert_eq!(initials, ["M", "G", "R", "U"], "told apart without colour");
        assert_eq!(Archangel::Uriel.next(), Archangel::Michael);
        assert_eq!((Archangel::Gabriel.boss_damage(), Archangel::Raphael.boss_damage(), Archangel::Uriel.boss_damage()), (3, 5, 8));
        // A full volley wounds the first Cyclops (50) well short of death.
        let volley: i32 = ARCHANGELS.iter().map(|a| a.boss_damage()).sum();
        assert!(volley * 2 < crate::logic::boss::CYCLOPS_HP, "{volley}");
        // Blue, green, purple, red.
        let c = |a: Archangel| a.color();
        let rgb = |x: u32| ((x >> 16) & 255, (x >> 8) & 255, x & 255);
        let (mr, _, mb) = rgb(c(Archangel::Michael));
        let (gr, gg, gb) = rgb(c(Archangel::Gabriel));
        let (rr, _, rb) = rgb(c(Archangel::Raphael));
        let (ur, ug, ub) = rgb(c(Archangel::Uriel));
        assert!(mb > mr && gg > gr && gg > gb && rb > 150 && rr > 150 && ur > ug && ur > ub);
    }

    #[test]
    fn each_power_comes_back_25_seconds_after_its_call_on_its_own() {
        let mut a = Angels::default();
        assert!(!a.any(None) && a.current(None).is_none(), "none before a shrine, without a patron");
        // A patron alone: his power only, and the next button has nothing to choose.
        assert_eq!(a.current(Some(Archangel::Raphael)), Some(Archangel::Raphael));
        assert!(!a.choose_next(Some(Archangel::Raphael)));
        assert!(a.ready(Archangel::Raphael, Some(Archangel::Raphael)) && !a.ready(Archangel::Gabriel, Some(Archangel::Raphael)));
        // A shrine: all four.
        a.granted = true;
        assert!(ARCHANGELS.iter().all(|&x| a.ready(x, None)));
        a.call(Archangel::Gabriel);
        assert_eq!((a.calls, a.last), (1, Some(Archangel::Gabriel)));
        assert!(!a.ready(Archangel::Gabriel, None) && a.ready(Archangel::Uriel, None), "each on its own");
        assert!(a.tick(RECHARGE - 0.5).is_empty());
        assert!((a.fill(Archangel::Gabriel) - 0.98).abs() < 0.01);
        assert_eq!(a.tick(0.6), vec![Archangel::Gabriel], "back after 25 seconds");
        assert!(a.ready(Archangel::Gabriel, None));
        // The next button goes round the four.
        a.chosen = Archangel::Raphael;
        assert!(a.choose_next(None));
        assert_eq!(a.chosen, Archangel::Uriel);
        a.choose_next(None);
        assert_eq!(a.chosen, Archangel::Michael);
        // A shrine after the first readies all four.
        a.call(Archangel::Uriel);
        a.call(Archangel::Michael);
        a.refresh();
        assert!(ARCHANGELS.iter().all(|&x| a.ready(x, None)));
    }
}
