//! The first-run tour: before the very first countdown the game holds still
//! and the camera swoops from the hero to the goal (the gate), to one foe and
//! to the objective (a golden key), while a see-through card explains each.
//! Enter or a click moves on, Esc skips; it plays once and is remembered in
//! the settings file.

use glam::Vec3;

use super::{Game, Screen};
use crate::audio::Sfx;
use crate::input::Action;
use crate::logic::enemy::EnemyKind;
use crate::logic::heroes::HeroKind;
use crate::logic::types::{Tile, P};
use crate::render::view::floor_at;
use crate::{logic, platform};

/// Settings keys that record the tours have been seen: the first dungeon's, and the Portals of Hell's.
const TOUR_KEY: &str = "vh.tour_done";
/// Remembers that the Portals of Hell's tour has been shown.
const HUB_TOUR_KEY: &str = "vh.tour_hub";
/// Seconds a stop holds before moving on by itself.
pub const TOUR_STOP_SECONDS: f64 = 3.3;

/// What the card shows beside the words.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TourArt {
    Hero,
    Nugget,
    Gate,
    Enemy(EnemyKind),
    /// A portal gate of the Portals of Hell, in its sin's colour.
    Portal(u32),
    /// The Seal of the Seven: its sockets, lit for the keys he holds.
    Seal,
    /// The volcano and the House of the Devil.
    Volcano,
}

#[derive(Clone, Debug)]
pub struct TourStop {
    /// The point the camera circles, in world space.
    pub target: Vec3,
    pub title: &'static str,
    pub body: String,
    pub color: u32,
    pub art: TourArt,
}

#[derive(Clone, Debug)]
pub struct Tour {
    pub stops: Vec<TourStop>,
    pub index: usize,
    /// Seconds at the current stop.
    pub t: f64,
    /// Seconds since the tour began (for the letterbox sliding in).
    pub age: f64,
}

impl Tour {
    pub fn stop(&self) -> &TourStop {
        &self.stops[self.index.min(self.stops.len() - 1)]
    }
}

/// Has this player seen the tour yet?
pub fn tour_seen() -> bool {
    platform::load(TOUR_KEY).map(|v| v.trim() == "1").unwrap_or(false)
}

/// Has this player been shown round the Portals of Hell?
pub fn hub_tour_seen() -> bool {
    cfg!(test) || platform::load(HUB_TOUR_KEY).map(|v| v.trim() == "1").unwrap_or(false)
}

/// Starting over: the tours are owed again.
pub fn forget_tour() {
    platform::save(TOUR_KEY, "0");
    platform::save(HUB_TOUR_KEY, "0");
}

impl Game {
    /// The stops for the first dungeon, kept short: the hero, the goal, one foe and the objective.
    fn tour_stops(&self) -> Vec<TourStop> {
        if self.hub.is_some() {
            return self.hub_tour_stops();
        }
        let g = &self.level.grid;
        let at = |p: P, lift: f32| Vec3::new(p.x as f32, floor_at(g, p.x as f32, p.y as f32) + lift, p.y as f32);
        let spawn = self.level.hero_spawn;
        let nearest = |want: Tile| {
            g.cells()
                .filter(|&(_, _, t)| t == want)
                .map(|(x, y, _)| P::new(x, y))
                .min_by_key(|p| (p.x - spawn.x).abs() + (p.y - spawn.y).abs())
        };
        let mut stops = vec![TourStop {
            target: at(spawn, 0.5),
            title: match self.hero_kind {
                HeroKind::Ragnar => "RAGNAR",
                HeroKind::Mehmed => "MEHMED",
                HeroKind::Miyamoto => "MIYAMOTO",
            },
            body: "That's you, with a blade, a shield and a friend to win back. Move with WASD or the stick, strike with left click, jump with Space. C switches the view."
                .into(),
            color: 0xf2e3c4,
            art: TourArt::Hero,
        }];
        // The goal: the gate, and what waits beyond it.
        if let Some(gate) = self.gate {
            stops.push(TourStop {
                target: at(P::new(gate.x, gate.y + 1), 1.0),
                title: "THE GOAL",
                body: "The gate in the north wall. Beyond it waits the Cyclops, and beyond him the road to your friend.".into(),
                color: 0xf2c53d,
                art: TourArt::Gate,
            });
        }
        // One foe, the first to come.
        if let Some(e) = self.enemies.first() {
            let p = e.pos();
            let lift = if e.m.flying { 1.0 } else { 0.5 };
            let target = Vec3::new(p.x as f32, floor_at(g, p.x as f32, p.y as f32) + lift, p.y as f32);
            let (title, body, color) = match e.kind() {
                EnemyKind::Skeleton => ("DRAUGR", "An undead viking. It follows your footsteps by sound. Cut it down and it falls apart, then pulls itself together and rises again.", 0xe9e9f2),
                EnemyKind::Bat => ("RAVEN", "Fast, and it flies over pits and ledges where you cannot follow.", 0xc9a0ff),
                EnemyKind::Vampire => ("NIGHT RAVEN", "The raven's red-eyed kin: bigger, bolder, and it hunts where the raven drives you.", 0xff6a6a),
                EnemyKind::Slime => ("BOG SLIME", "Slow, but it leaves a trail of bog slime behind it that slows you down. Step round the trail, or leap it.", 0x5cff6a),
                EnemyKind::Witch => ("WITCH", "She keeps her distance and cackles up minions to do her biting. Two blows fell her; the first only makes her shriek.", 0xb066ff),
                EnemyKind::Minion => ("WITCH MINION", "Knee-high, quick and eager. One blow is enough, but they come in numbers.", 0xd08cff),
                EnemyKind::Helmeted => ("HELMETED SKELETON", "A draugr's cousin in an iron cap: the first blow rings off the helmet. It hears your footsteps, and rises again when it falls.", 0xff5a4a),
                EnemyKind::Werewolf => ("LOUP-GAROU", "Fast, and faster still once wounded: the first cut only makes it howl.", 0xc0885a),
                EnemyKind::Blemmyes => ("BLEMMYES", "A headless brute with his face in his chest. Slow, but he takes three blows, and his hand hits twice as hard.", 0xe8b494),
                EnemyKind::LargeBat => ("LARGE BAT", "Flies over everything, swerving as it comes, and swoops at where you are going.", 0xc04a70),
                EnemyKind::Tengu => ("TENGU", "A mountain goblin and a master of the sword. Melee: he closes in fast and his katana cuts high and wide. Wait out the swing, then strike while he recovers.", 0xe8503e),
                EnemyKind::Rasetsu => ("RASETSU", "A horned red man-eater with a curved blade in each fist. Melee: slow to start and hard to stagger. Block or leap the chop, then cut him down.", 0xd0402e),
                EnemyKind::DireWolf => ("DIRE WOLF", "Bigger than the loup-garou, and it attacks with its whole body: it circles, crouches, then throws itself at you. Raise your guard as it springs.", 0xa4acb8),
                EnemyKind::Jorogumo => ("JOROGUMO", "A spider-woman who fights from a distance, hurling fire and backing away when you come near. Close in on her.", 0xff8a3a),
                EnemyKind::Cobra => ("COBRA", "Coiled and patient, it strikes with its body: it rears back, then lunges. Parry the bite as it lands.", 0x9ac84a),
                EnemyKind::Oni => ("ONI", "An ogre with an iron club: \"oni ni kanabō\", the strong made unstoppable. Melee: once the club is raised nothing stops it but a parry, and when it lands the ground shakes all round him. Leap the tremor.", 0xd8402c),
                EnemyKind::Kappa => ("KAPPA", "A river imp with a dish of water on its head. Body: it charges like a sumo wrestler and shoves you back, shield or no shield. Bow to it (parry the charge) and it bows back, spilling its dish: weak until it fills again.", 0x5ab07a),
                EnemyKind::Rokurokubi => ("ROKUROKUBI", "By day a woman; by night her neck stretches across the room. Body: her head lashes out down a lane from far away. Step out of the lane or parry the bite, then close in: up close she is slow.", 0xe8d8e8),
                EnemyKind::Kitsune => ("KITSUNE", "An evil fox spirit. Distance: it throws three foxfires at once. Wounded, it hides among illusions of itself: one blow undoes an illusion, and their fire is only seeming.", 0xff8a3a),
                EnemyKind::Karakoncolos => ("KARAKONCOLOS", "A winter demon of the zemheri, the coldest nights. Melee: it roars your name in a loved one's voice, and the roar dooms you (your next wound is worse). Get out of earshot as it throws its head back, or take no blow until the doom passes.", 0x8ec8ff),
                EnemyKind::Tulukus => ("TÜLÜKUŞ", "An owl demon. Body: it takes wing out of reach and drops without a sound onto the mark below. Its cry as it rises dooms you (your next wound is worse). Step out of the circle.", 0xd09a5a),
                EnemyKind::Albasti => ("ALBASTI", "The night hag, the weight on a sleeper's chest. Body: a shadow with red eyes until she pounces, and she lands with all the weight of a nightmare. Watch for the red eyes, then parry or leap the pounce.", 0xc0283a),
                EnemyKind::Cin => ("CIN", "A spirit of smokeless fire. Distance: its fire flies at you out of nowhere (cin çarpması, the jinn's blow). Come near and it is gone in smoke, to stand far off. Raise your shield against the fire.", 0xffe6a0),
                EnemyKind::Arcura => ("ARÇURA", "A forest spirit as tall as a tree or as small as a blade of grass. Distance: roots burst from the floor under you and lash you. Leave the ring of leaves before they break the ground.", 0x6ad050),
                EnemyKind::Tepegoz => ("TEPEGÖZ", "The one-eyed giant of the Book of Dede Korkut. Melee: his mother's ring turns every blade, so only his eye can be hurt. Strike when he is bent over after a blow, or reeling from your parry.", 0xe8a070),
                EnemyKind::Gashadokuro => ("GASHADOKURO", "A giant skeleton of the starved, heard before it is seen. Melee: a ringing in your ears, then its hands close and crush you all at once, and no shield holds them. Parry, or leap away.", 0xdcd8b8),
                EnemyKind::Ejder => ("YEDİ BAŞLI EJDER", "The seven-headed dragon that sat on the town's spring. Body: its heads strike one after another. Felled, it begs \"Bir daha vur!\" (strike again): strike and it rises; hold your hand and it dies.", 0x7ac04a),
                EnemyKind::Goryo => ("GORYŌ", "The vengeful ghost of a wronged noble. Distance: it calls lightning down on three marks. Nothing touches it but while its wrath is upon it: strike as it calls the storm.", 0x8ab0ff),
                EnemyKind::FrostGiant => ("FROST GIANT", "A rime-giant of Jötunheim. Melee: its blows frostbite you even through your shield, and its wave of ice runs down a lane. The frost keeps biting for a while after: keep out of its reach and step out of the lane.", 0xa8dcff),
                EnemyKind::Surtr => ("SURTR", "The fire giant of Muspelheim. Melee: his flaming sword reaches far, and the floor burns where it falls. Do not stand in the fire.", 0xff6a1a),
                EnemyKind::Nidhogg => ("NIDHOGG", "The dragon that gnaws the roots of the world. Body: it burrows under the floor and bursts up beneath you, and its bite is venom. Leave the mark before the ground breaks.", 0x8ad04a),
                EnemyKind::Peerless => ("THE PEERLESS", "Your own reflection, stepped out of the mirror. It never acts on its own: it repeats every move you make, a moment later.", 0xd8c8ff),
                EnemyKind::Ganglot => ("GANGLÖT", "One of Hel's shrouded servants, Ganglati and Ganglöt: slow as a snail, and they never stop. The noise of killing them keeps you awake.", 0x9aa4b0),
                EnemyKind::Gaki => ("GAKI", "A hungry ghost, all swollen belly and needle throat. Body: they swarm whoever smells of food.", 0xc8b49a),
                EnemyKind::Ellepige => ("ELLEPIGE", "An elf-maid, lovely from the front and hollow as a kneading trough behind. No blow tells on her face: strike her back.", 0xf0e0ea),
                EnemyKind::Hel => ("HEL", "Queen of the dead, half living and half corpse. Distance: her bolts drain your life into hers, and she raises the dead from the floor. When she falls, they fall with her.", 0x7aa0e8),
            };
            stops.push(TourStop { target, title, body: body.into(), color, art: TourArt::Enemy(e.kind()) });
        }
        // The objective: the keys that open the gate.
        if let Some(p) = nearest(Tile::Nodule) {
            stops.push(TourStop {
                target: at(p, 0.2),
                title: "THE OBJECTIVE",
                body: format!("Find the {} golden keys hidden across the garden. With every one, the gate opens.", self.keys_total),
                color: 0xf2c53d,
                art: TourArt::Nugget,
            });
        }
        stops
    }

    /// The Portals of Hell: where he came in, the Seal of the Seven, each of the seven doors, and the House of the Devil.
    fn hub_tour_stops(&self) -> Vec<TourStop> {
        use logic::hub;
        let g = &self.level.grid;
        let at = |p: P, lift: f32| Vec3::new(p.x as f32, floor_at(g, p.x as f32, p.y as f32) + lift, p.y as f32);
        let mut stops = vec![
            TourStop {
                target: at(hub::arrival(), 0.5),
                title: "THE PORTALS OF HELL",
                body: "Down from the Cyclops' meadow into the pit of the underworld. Eight platforms float over the lava round the House of the Devil, joined by wooden bridges.".into(),
                color: 0xff6a3a,
                art: TourArt::Hero,
            },
            TourStop {
                target: at(hub::altar(), 0.9),
                title: "THE SEAL OF THE SEVEN",
                body: "A socket for each sin's key. Take a key from the prince of every realm: with all seven, the seal breaks and the way to the House of the Devil rises from the lava.".into(),
                color: 0xffb040,
                art: TourArt::Seal,
            },
        ];
        for gate in hub::gates() {
            let sin = gate.sin;
            stops.push(TourStop {
                target: at(gate.tile, 1.3),
                title: sin.title(),
                body: format!(
                    "Behind this door lies {}, where {} keeps the key of {}. On the door, {}. Walk up to it and it opens.",
                    sin.realm(),
                    sin.prince(),
                    sin.name().to_lowercase(),
                    sin.door()
                ),
                color: sin.color(),
                art: TourArt::Portal(sin.color()),
            });
        }
        let (cx, cy) = hub::volcano().center();
        stops.push(TourStop {
            target: Vec3::new(cx, 2.4, cy + 2.0),
            title: "THE HOUSE OF THE DEVIL",
            body:
                "Where Krampus carried your friend, to his father. A wall of fire guards the volcano until the seal of the seven is broken."
                    .into(),
            color: 0xff3a2a,
            art: TourArt::Volcano,
        });
        stops
    }

    /// Starts the tour (the first new game on this machine).
    pub(super) fn begin_tour(&mut self) {
        self.tour = Some(Tour { stops: self.tour_stops(), index: 0, t: 0.0, age: 0.0 });
        self.audio.play(Sfx::Whoosh);
        self.enter(Screen::Tour);
    }

    pub(super) fn update_tour(&mut self, dt: f64) {
        let Some(tour) = &mut self.tour else {
            self.enter(Screen::Ready);
            return;
        };
        tour.t += dt;
        tour.age += dt;
        let skip = self.input.was_pressed(&[Action::Pause]);
        let next = tour.t > TOUR_STOP_SECONDS || self.input.was_pressed(&[Action::Confirm, Action::Tap, Action::Attack, Action::Jump]);
        if skip {
            self.finish_tour();
            return;
        }
        if next && tour.t > 0.15 {
            tour.index += 1;
            tour.t = 0.0;
            if tour.index >= tour.stops.len() {
                self.finish_tour();
            } else {
                self.audio.play(Sfx::Whoosh);
            }
        }
        self.input.consume(Action::Attack);
    }

    /// Remembers the tour was seen and starts the countdown.
    fn finish_tour(&mut self) {
        self.tour = None;
        self.tour_pending = false;
        platform::save(if self.hub.is_some() { HUB_TOUR_KEY } else { TOUR_KEY }, "1");
        self.enter(Screen::Ready);
    }
}
