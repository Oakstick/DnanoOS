//! The first-run tour: before the very first countdown the game holds still
//! and the camera swoops from Sebastian to each kind of thing in the
//! dungeon (gold, crosses, bottles, shrines, a pit) and then to each foe,
//! while a see-through card explains it. Enter or a click moves on, Esc
//! skips; it plays once and is remembered in the settings file.

use glam::Vec3;

use super::{Game, Screen};
use crate::audio::Sfx;
use crate::input::Action;
use crate::logic::enemy::EnemyKind;
use crate::logic::types::{PotionKind, Tile, P};
use crate::platform;
use crate::render::view::floor_at;

/// Settings key that records the tour has been seen.
const TOUR_KEY: &str = "vh.tour_done";
/// Seconds a stop holds before moving on by itself.
pub const TOUR_STOP_SECONDS: f64 = 3.3;

/// What the card shows beside the words.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TourArt {
    Hero,
    Nugget,
    Gate,
    Cross,
    Potion(PotionKind),
    WeaponShrine,
    ItemShrine,
    Pit,
    Enemy(EnemyKind),
}

#[derive(Clone, Debug)]
pub struct TourStop {
    /// The point the camera circles, in world space.
    pub target: Vec3,
    pub title: &'static str,
    pub body: &'static str,
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

impl Game {
    /// The stops for the current dungeon: the hero, one of each thing, then each foe.
    fn tour_stops(&self) -> Vec<TourStop> {
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
            title: "SEBASTIAN",
            body: "That's you: a viking with an axe, a round shield, and a hoard to win. Move with WASD or the stick, attack with left click, jump with Space, bomb with right click.",
            color: 0xf2e3c4,
            art: TourArt::Hero,
        }];
        if let Some(p) = nearest(Tile::Nodule) {
            stops.push(TourStop {
                target: at(p, 0.2),
                title: "GOLDEN KEY",
                body: "Eight are hidden across the estate. Each one lights a lock on the gate and is worth 30 coins for the blacksmith. Look for their golden glow.",
                color: 0xf2c53d,
                art: TourArt::Nugget,
            });
        }
        if let Some(gate) = self.gate {
            stops.push(TourStop {
                target: at(P::new(gate.x, gate.y + 1), 1.0),
                title: "THE GATE",
                body: "In the north wall. With all eight keys its doors swing open: go through to face the Cyclops.",
                color: 0xf2c53d,
                art: TourArt::Gate,
            });
        }
        if let Some(p) = nearest(Tile::Charge) {
            stops.push(TourStop {
                target: at(p, 0.3),
                title: "GREEN CROSS",
                body: "Walk over it to restore 50 health.",
                color: 0x5cff6a,
                art: TourArt::Cross,
            });
        }
        if let Some(pot) = self.potions.iter().filter(|p| !p.taken).min_by_key(|p| (p.tile.x - spawn.x).abs() + (p.tile.y - spawn.y).abs()) {
            stops.push(TourStop {
                target: at(pot.tile, 0.2),
                title: "POTION",
                body: "Bottles go on your belt, up to five. Drink with Q, or 1 to 5 for a slot. Only the first dungeon has them lying around; after that, buy them from the blacksmith.",
                color: pot.def().color,
                art: TourArt::Potion(pot.kind),
            });
        }
        for d in &self.diamonds {
            let weapon = d.kind == crate::logic::items::DiamondKind::Weapon;
            if stops.iter().any(|s| s.art == if weapon { TourArt::WeaponShrine } else { TourArt::ItemShrine }) {
                continue;
            }
            stops.push(if weapon {
                TourStop {
                    target: at(d.tile, 0.4),
                    title: "WEAPON SHRINE",
                    body: "Strike it to change its gift, step on it to take it. The weapon replaces your own until its ammo runs out.",
                    color: 0x9ad1ff,
                    art: TourArt::WeaponShrine,
                }
            } else {
                TourStop {
                    target: at(d.tile, 0.4),
                    title: "ITEM SHRINE",
                    body: "Offers a shield repair, a spell of invincibility, or a burst of speed. Strike to change, step on to take.",
                    color: 0xffd84a,
                    art: TourArt::ItemShrine,
                }
            });
        }
        if let Some(p) = nearest(Tile::Pit) {
            stops.push(TourStop {
                target: at(p, 0.0),
                title: "PIT",
                body: "You cannot walk over a pit. Jump it with Space; a jump also climbs one ledge and leaps over foes.",
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
            };
            stops.push(TourStop { target, title, body, color, art: TourArt::Enemy(e.kind()) });
        }
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
        platform::save(TOUR_KEY, "1");
        self.enter(Screen::Ready);
    }
}
