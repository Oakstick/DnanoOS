//! The first-run tour: before the very first countdown the game holds still
//! and the camera swoops from Ragnar to each kind of thing in the
//! dungeon (gold, crosses, bottles, shrines, a pit) and then to each foe,
//! while a see-through card explains it. Enter or a click moves on, Esc
//! skips; it plays once and is remembered in the settings file.

use glam::Vec3;

use super::{Game, Screen};
use crate::audio::Sfx;
use crate::input::Action;
use crate::logic::enemy::EnemyKind;
use crate::logic::heroes::HeroKind;
use crate::logic::items::DiamondKind;
use crate::logic::types::{PotionKind, Tile, P};
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
    Fountain,
    Nugget,
    Gate,
    Cross,
    Potion(PotionKind),
    AngelShrine,
    ItemShrine,
    Pit,
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
    /// The stops for the current dungeon: the hero, one of each thing, then each foe.
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
            body: match self.hero_kind {
                HeroKind::Ragnar => "That's you: a viking with a blade, a round shield, and a friend to win back. Move with WASD or the stick, attack with left click, jump with Space, bomb with right click.",
                HeroKind::Mehmed => "That's you: a janissary with a blade, a kalkan, and a friend to win back. Move with WASD or the stick, attack with left click, jump with Space, bomb with right click.",
                HeroKind::Miyamoto => "That's you: a samurai with a blade, a tate, and a friend to win back. Move with WASD or the stick, attack with left click, jump with Space, bomb with right click.",
            }
            .into(),
            color: 0xf2e3c4,
            art: TourArt::Hero,
        }];
        if let Some(f) = self.garden.as_ref().and_then(|g| g.fountain) {
            stops.push(TourStop {
                target: at(f.p, 1.3),
                title: "THE KRAMPUS FOUNTAIN",
                body: "The heart of the garden. Its four channels run out to every side of the estate: cross them by the bridges, which slimes will not set foot on.".into(),
                color: 0x5cd4ff,
                art: TourArt::Fountain,
            });
        }
        if let Some(p) = nearest(Tile::Nodule) {
            stops.push(TourStop {
                target: at(p, 0.2),
                title: "GOLDEN KEY",
                body: "Eight are hidden across the estate. Each one lights a lock on the gate and is worth 30 coins at the old gatekeeper's. Look for their golden glow.".into(),
                color: 0xf2c53d,
                art: TourArt::Nugget,
            });
        }
        if let Some(gate) = self.gate {
            stops.push(TourStop {
                target: at(P::new(gate.x, gate.y + 1), 1.0),
                title: "THE GATE",
                body: "In the north wall. With all eight keys its doors swing open: go through to face the Cyclops.".into(),
                color: 0xf2c53d,
                art: TourArt::Gate,
            });
        }
        if let Some(p) = nearest(Tile::Charge) {
            stops.push(TourStop {
                target: at(p, 0.3),
                title: "GREEN CROSS",
                body: "Walk over it to restore 50 health.".into(),
                color: 0x5cff6a,
                art: TourArt::Cross,
            });
        }
        if let Some(pot) = self.potions.iter().filter(|p| !p.taken).min_by_key(|p| (p.tile.x - spawn.x).abs() + (p.tile.y - spawn.y).abs())
        {
            stops.push(TourStop {
                target: at(pot.tile, 0.2),
                title: "POTION",
                body: "Bottles go on your belt, up to five. Drink with Q, or 1 to 5 for a slot. Only the first dungeon has them lying around; after that, buy them from the old gatekeeper.".into(),
                color: pot.def(self.hero_kind).color,
                art: TourArt::Potion(pot.kind),
            });
        }
        for d in &self.diamonds {
            let angel = d.kind == DiamondKind::Angel;
            if stops.iter().any(|s| s.art == if angel { TourArt::AngelShrine } else { TourArt::ItemShrine }) {
                continue;
            }
            stops.push(if angel {
                TourStop {
                    target: at(d.tile, 0.4),
                    title: "ARCHANGEL'S SHRINE",
                    body: "Strike it to switch between Michael (blue), Gabriel (green), Raphael (purple) and Uriel (red); step on it for that archangel's power, used with R or LT. Each archangel gives once a level, and Uriel only once in ten minutes.".into(),
                    color: 0xe8dcc0,
                    art: TourArt::AngelShrine,
                }
            } else {
                TourStop {
                    target: at(d.tile, 0.4),
                    title: "ITEM SHRINE",
                    body: "Offers a shield repair, a spell of invincibility, or a burst of speed. Strike to change, step on to take.".into(),
                    color: 0xffd84a,
                    art: TourArt::ItemShrine,
                }
            });
        }
        if let Some(p) = nearest(Tile::Pit) {
            stops.push(TourStop {
                target: at(p, 0.0),
                title: "PIT",
                body: "You cannot walk over a pit. Jump it with Space; a jump also climbs one ledge and leaps over foes.".into(),
                color: 0xc8b89a,
                art: TourArt::Pit,
            });
        }
        let mut seen: Vec<EnemyKind> = Vec::new();
        for e in &self.enemies {
            if seen.contains(&e.kind()) {
                continue;
            }
            seen.push(e.kind());
            let p = e.pos();
            let lift = if e.m.flying { 1.0 } else { 0.5 };
            let target = Vec3::new(p.x as f32, floor_at(g, p.x as f32, p.y as f32) + lift, p.y as f32);
            let (title, body, color) = match e.kind() {
                EnemyKind::Skeleton => ("DRAUGR", "An undead viking. It follows your footsteps by sound. Cut it down and it falls apart, then pulls itself together and rises again.", 0xe9e9f2),
                EnemyKind::Bat => ("RAVEN", "Fast, and it flies over pits and ledges where you cannot follow.", 0xc9a0ff),
                EnemyKind::Vampire => ("NIGHT RAVEN", "The raven's red-eyed kin: bigger, bolder, and it hunts where the raven drives you.", 0xff6a6a),
                EnemyKind::Slime => ("BOG SLIME", "Slow, but it leaves a trail of slime behind it that slows you down.", 0x5cff6a),
                EnemyKind::Witch => ("WITCH", "She keeps her distance and cackles up minions to do her biting. Two blows fell her; the first only makes her shriek.", 0xb066ff),
                EnemyKind::Minion => ("WITCH MINION", "Knee-high, quick and eager. One blow is enough, but they come in numbers.", 0xd08cff),
                EnemyKind::Helmeted => ("HELMETED SKELETON", "A draugr's cousin in an iron cap: the first blow rings off the helmet. It hears your footsteps, and rises again when it falls.", 0xff5a4a),
                EnemyKind::Werewolf => ("LOUP-GAROU", "Fast, and faster still once wounded: the first cut only makes it howl.", 0xc0885a),
                EnemyKind::Blemmyes => ("BLEMMYES", "A headless brute with his face in his chest. Slow, but he takes three blows, and his hand hits twice as hard.", 0xe8b494),
                EnemyKind::LargeBat => ("LARGE BAT", "Flies over everything, swerving as it comes, and swoops at where you are going.", 0xc04a70),
                EnemyKind::Tengu => ("TENGU", "A mountain goblin and a master of the sword. Melee: he closes in fast and his katana cuts high and wide. Wait out the swing, then strike while he recovers.", 0xe8503e),
                EnemyKind::Oni => ("ONI", "A horned red demon with a curved blade in each fist. Melee: slow to start and hard to stagger. Block or leap the chop, then cut him down.", 0xd0402e),
                EnemyKind::DireWolf => ("DIRE WOLF", "Bigger than the loup-garou, and it attacks with its whole body: it circles, crouches, then throws itself at you. Raise your guard as it springs.", 0xa4acb8),
                EnemyKind::Jorogumo => ("JOROGUMO", "A spider-woman who fights from a distance, hurling fire and backing away when you come near. Close in on her.", 0xff8a3a),
                EnemyKind::Cobra => ("COBRA", "Coiled and patient, it strikes with its body: it rears back, then lunges. Parry the bite as it lands.", 0x9ac84a),
            };
            stops.push(TourStop { target, title, body: body.into(), color, art: TourArt::Enemy(e.kind()) });
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
