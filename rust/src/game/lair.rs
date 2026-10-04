//! Krampus's lair in play (FINALE.md §3): the bottom of the Pit (`logic::lair` lays it out),
//! Krampus, and the friend in the cage.
//!
//! **Why a module of its own, and not a court.** Krampus fights on the princes' machinery: a
//! duelist of the Prince archetype in a brawl of the vaults' rules, three tables of moves by his
//! health, a hide, a token of three, his phase banners and breathers (`court::prince_phase` hands
//! his to `krampus_phase`), the prince's bar and the chase camera turned on him (`court::prince`
//! finds him here too). But a court is a sin's: its `Court` is keyed by the sin throughout (its
//! hooks, its colour, its general's terms, its key, the gate and the old man's wares after it),
//! and Krampus has none of them. He holds no key, no general weighs on him, no gate opens behind
//! him; his fight ends in the cage coming down. So the lair keeps its own state (`Lair`) beside
//! the court's, and the court's shared pieces learn of it in three small places (the prince found,
//! his bar, his phase).
//!
//! What the lair does of its own, beyond his frame data:
//!
//! - **The charge's daze.** His hoof charge runs straight along its locked aim; one that runs into
//!   stone (an ice column, the cage, a brazier, the wall) leaves him dazed (staggered, open).
//! - **The Perchten.** As the second phase begins, two of them come out of the clefts in the wall
//!   (once); they flee into the dark when he falls.
//! - **The cage taunt.** In Krampusnacht he goes to the cage now and then, rattles it at the child
//!   and laughs while the boy cries (`MoveId::CageRattle`): a long recovery, his pelt open all the
//!   while. Nothing he does there touches the boy, and nothing in the fight can: the cage is stone
//!   to every blow, coal and step.
//! - **His death** is slow: he sinks to his knees and the bells on his chains fall silent one by
//!   one, under the juice (a freeze, time slowed, the screen shaken); then the cage's lock breaks,
//!   the cage comes down, the boy climbs out and runs to the hero, and they hold each other
//!   (`lair_won`).

use super::brawl::{Brawl, Field};
use super::court::{COURT_CADENCE, COURT_CAPACITY};
use super::save::Stage;
use super::{Banner, Game, Screen};
use crate::audio::{Cry, Sfx};
use crate::logic::combat::ails::{Flash, FlashKind};
use crate::logic::combat::archetype::Archetype;
use crate::logic::combat::duelist::{Flow, GridTerrain, Lore, Terrain};
use crate::logic::combat::frames::Shape;
use crate::logic::combat::prince::krampus::{
    BELLS, DAZE, PERCHTEN, PERCHT_DAMAGE, PERCHT_HEALTH, PERCHT_RISE, TAUNTS, TAUNT_EVERY, TAUNT_FIRST, TAUNT_WALK,
};
use crate::logic::combat::prince::phase_name;
use crate::logic::combat::{math, Breed, Duelist, MoveId, Phase, Reel, State};
use crate::logic::court::weight;
use crate::logic::enemy::{spec_of, Enemy, EnemyKind, EnemyMode};
use crate::logic::juice::{DilationKind, Stop, Trauma};
use crate::logic::lair::{self as plan, CAGE_MIDDLE, CAGE_STAND, CLEFTS, KRAMPUS};
use crate::logic::sins::ALL_SINS;
use crate::logic::types::{dist_sq, P, V};

/// His colours: his name's red (the bar), the bells' brass, the snow's.
pub const KRAMPUS_RED: u32 = 0xc8202a;
const BRASS: u32 = 0xe8c060;
const FROST: u32 = 0xc8e4ff;

/// His health and his blows at the bottom of the Pit, by the tier of the hero who reaches it (all
/// seven keys held: tier 8): the courts' weight (`court::weight`), and his own breed's on top.
pub fn lair_weight(tier: usize) -> (f64, f64) {
    weight(tier)
}

/// His death, as it plays out (seconds since the killing blow): the bells fall silent one every
/// `BELL_EVERY`; he sinks into the snow; the cage's lock breaks at `LOCK_BREAKS`, the cage is
/// down `CAGE_FALL` after, its door open at `DOOR_OPENS`; the boy climbs out at `CLIMBS_OUT` and
/// runs, and once he has reached the hero they hold each other for `EMBRACE` before the end.
pub const BELL_EVERY: f64 = 0.6;
pub const LOCK_BREAKS: f64 = 5.0;
pub const CAGE_FALL: f64 = 0.7;
pub const DOOR_OPENS: f64 = 6.3;
pub const CLIMBS_OUT: f64 = 6.9;
pub const CLIMB: f64 = 0.8;
pub const EMBRACE: f64 = 2.0;
/// The boy's pace, running to the hero (tiles a second), and how near he comes before he holds him.
pub const CHILD_PACE: f64 = 3.8;
pub const CHILD_REACH: f64 = 0.85;

/// Krampus fallen: where, which way he faced, seconds since, and how many of his bells still ring.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fall {
    pub at: V,
    pub facing: V,
    pub age: f64,
    pub bells: u8,
}

/// The cage: its lock broken, how far it has come down (0 hanging, 1 on the floor) and how far its
/// door stands open (0 to 1).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Cage {
    pub broken: bool,
    pub down: f64,
    pub door: f64,
}

/// Where the boy is in it all.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChildState {
    /// In the cage.
    #[default]
    Caged,
    /// Climbing out of its door.
    Climbing,
    /// Running to the hero.
    Running,
    /// In the hero's arms.
    Held,
}

/// The stolen friend: where he is, what he is doing, seconds into it, and (in the cage) how
/// frightened he is (0 to 1) and how long he goes on crying.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Child {
    pub pos: V,
    pub state: ChildState,
    pub age: f64,
    pub fear: f32,
    pub cry: f32,
    /// Which way he faces.
    pub facing: V,
}

/// Krampus at the cage: the frames he has walked to it, and whether he has begun rattling it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Taunt {
    pub walked: u32,
    pub rattling: bool,
}

/// What the lair keeps of its fight: who Krampus is, his phase as last seen, his health as the bar
/// showed it a moment ago (its pale trail), his taunts, and once he has fallen his death, the cage
/// and the boy.
#[derive(Clone, Debug, PartialEq)]
pub struct Lair {
    pub krampus: u32,
    pub phase: u8,
    pub trail: f32,
    pub slain: bool,
    /// The last frame of the fight the lair has seen (its work is done once a frame).
    pub frame: u32,
    /// A taunt under way, how many he has made, and the frames before the next may begin.
    pub taunt: Option<Taunt>,
    pub taunts: u32,
    pub taunt_wait: u32,
    /// Charges begun, and charges that ended dazed against stone (for the harness).
    pub charges: u32,
    pub dazes: u32,
    pub fall: Option<Fall>,
    pub cage: Cage,
    pub child: Child,
    /// The boy has reached the hero and the end has been called.
    pub won: bool,
}

impl Lair {
    fn new(krampus: u32, trail: f32) -> Self {
        Lair {
            krampus,
            phase: 1,
            trail,
            slain: false,
            frame: 0,
            taunt: None,
            taunts: 0,
            taunt_wait: TAUNT_FIRST,
            charges: 0,
            dazes: 0,
            fall: None,
            cage: Cage::default(),
            child: Child {
                pos: V::new(CAGE_MIDDLE.0, CAGE_MIDDLE.1),
                state: ChildState::Caged,
                age: 0.0,
                fear: 0.4,
                cry: 0.0,
                facing: V::new(0.0, 1.0),
            },
            won: false,
        }
    }
}

impl Game {
    /// In Krampus's lair.
    pub fn in_lair(&self) -> bool {
        self.lair.is_some()
    }

    /// Krampus's lair as a level, as hard as the keys held make it (all seven, at the bottom of the
    /// Pit: no one comes here without them).
    pub fn load_lair(&mut self) {
        self.sins_cleared |= ALL_SINS;
        let tier = self.realm_tier();
        self.load(plan::level(tier), tier);
    }

    /**
     * The way into the lair: the Pit's platform has come to rest at the bottom (FINALE.md §3).
     * The lair is loaded with the hero standing on the platform, the adventure saved there (a
     * fall, or Continue, brings him back to Krampus whole), and the round's countdown begins.
     */
    pub fn enter_lair(&mut self) {
        self.load_lair();
        self.checkpoint(Stage::Lair);
        self.juice.fade = 1.0;
        self.juice.banner = Some(Banner {
            text: "THE BOTTOM OF THE PIT".into(),
            sub: "KRAMPUS WAITS. YOUR FRIEND IS IN HIS CAGE".into(),
            color: FROST,
            life: 4.0,
            max: 4.0,
        });
        self.enter(Screen::Ready);
    }

    /**
     * The fight is over: Krampus is slain, the cage is open, the friend has reached the hero.
     * The end begins: Scene II, the way up (`ending`).
     */
    pub fn lair_won(&mut self) {
        if let Some(l) = self.lair.as_mut() {
            l.won = true;
        }
        self.begin_ending();
    }

    /// Lays out the fight as the lair is loaded: Krampus before the cage, the fight's rules.
    pub(super) fn setup_lair(&mut self) {
        let tier = self.realm_tier();
        let mut brawl = Brawl::new(Field::Vault);
        brawl.reset(self.hero_kit());
        brawl.threat.capacity = COURT_CAPACITY;
        brawl.threat.cadence = COURT_CADENCE;
        brawl.foe_damage = 1.0;
        let id = brawl.next_id();
        let at = P::new(KRAMPUS.0, KRAMPUS.1);
        let (hp, damage) = lair_weight(tier);
        let spec = spec_of(&self.level.grid, EnemyKind::Krampus);
        let mut e = Enemy::new(&spec, at, self.level.data.enemy_speed);
        e.post = Some(at);
        e.b.mode = EnemyMode::Chase;
        e.b.name = spec.name.to_uppercase();
        let mut d = Duelist::bred(id, Breed::of(EnemyKind::Krampus), at.v(), V::new(0.0, 1.0));
        d.max_hp *= hp;
        d.hp = d.max_hp;
        d.breed.damage *= damage;
        d.rest = 90;
        let trail = d.max_hp as f32;
        e.duel = Some(Box::new(d));
        self.enemies = vec![e];
        self.brawl = Some(brawl);
        self.lair = Some(Lair::new(id, trail));
    }

    /// Krampus, while he stands.
    pub fn krampus(&self) -> Option<&Duelist> {
        let id = self.lair.as_ref()?.krampus;
        self.enemies.iter().filter_map(|e| e.duel.as_deref()).find(|d| d.id == id && d.alive())
    }

    /// The lair each tick: the fallen swept away, Krampus's own doings once a frame of the fight
    /// (the charge's daze, the taunt, the boy's fear), the bar's trail; once he has fallen, his
    /// death, the cage and the boy.
    pub(super) fn update_lair(&mut self, dt: f64) {
        if self.enemies.iter().any(|e| e.mode() == EnemyMode::Eaten) {
            self.enemies.retain(|e| e.mode() != EnemyMode::Eaten);
            self.refresh_foe_views();
        }
        let Some(l) = self.lair.as_ref() else { return };
        if l.slain {
            self.lair_aftermath(dt);
            return;
        }
        let frame = self.brawl.as_ref().map(|b| b.threat.frame).unwrap_or(0);
        if l.frame != frame {
            if let Some(l) = self.lair.as_mut() {
                l.frame = frame;
            }
            self.krampus_frame();
        }
        let hp = self.krampus().map(|d| d.hp.max(0.0) as f32);
        if let (Some(l), Some(hp)) = (self.lair.as_mut(), hp) {
            // The pale trail of what the last blows cost, catching up after a moment.
            l.trail = if l.trail > hp { (l.trail - (dt as f32) * 160.0).max(hp) } else { hp };
            // The boy's fear settles while nothing frightens him anew; his crying stops.
            l.child.fear = (l.child.fear - dt as f32 * 0.12).max(0.35);
            l.child.cry = (l.child.cry - dt as f32).max(0.0);
        }
    }

    /// One frame of Krampus's own: a charge run into stone dazes him; in Krampusnacht, the cage.
    fn krampus_frame(&mut self) {
        let Some(id) = self.lair.as_ref().map(|l| l.krampus) else { return };
        let Some(i) = self.enemies.iter().position(|e| e.duel.as_ref().is_some_and(|d| d.id == id && d.alive())) else { return };
        let Some(d) = self.enemies[i].duel.as_deref_mut() else { return };
        let charging = matches!(d.f.state, State::Attack { mv: MoveId::HoofCharge, .. });
        if matches!(d.f.state, State::Attack { mv: MoveId::HoofCharge, frame: 0 }) {
            if let Some(l) = self.lair.as_mut() {
                l.charges += 1;
            }
        }
        let Some(d) = self.enemies[i].duel.as_deref_mut() else { return };
        if charging && d.crashed {
            // Into stone: he reels, dazed, his horns ringing.
            d.f.state = State::Reel { kind: Reel::Staggered, left: DAZE, total: DAZE };
            d.vel = V::new(0.0, 0.0);
            let at = d.pos;
            if let Some(l) = self.lair.as_mut() {
                l.dazes += 1;
            }
            self.floater(at.x, at.y - 2.4, "DAZED ON THE STONE: STRIKE NOW", 1.6, Some(FROST));
            self.audio.foe_cry(EnemyKind::Krampus, Cry::Stagger, at);
            self.view.shake(0.45);
            self.view.sparkle(&self.level.grid, at.x, at.y, 1.6, FROST, 30);
            return;
        }
        let near_cage = dist_sq(d.pos, V::new(CAGE_STAND.0, CAGE_STAND.1)).sqrt();
        let phase = d.breed.phase;
        // The boy is frightened by him near the cage, and by his roar.
        if near_cage < 4.0 {
            if let Some(l) = self.lair.as_mut() {
                l.child.fear = l.child.fear.max(0.75);
            }
        }
        if phase >= 3 {
            self.cage_taunt(i);
        }
    }

    /**
     * Krampusnacht's taunt: between his moves, now and then, he walks to the cage (a puppet of
     * the lair, his token given back) and rattles it at the boy: a long recovery, his pelt open
     * all the while. Broken off by a blow that staggers him, or given up after a long walk.
     */
    fn cage_taunt(&mut self, i: usize) {
        let Some(l) = self.lair.as_ref() else { return };
        let (taunt, taunts, wait) = (l.taunt, l.taunts, l.taunt_wait);
        let hp = self.hero.pos();
        let Some(du) = self.enemies[i].duel.as_deref() else { return };
        let (id, pos) = (du.id, du.pos);
        let Some(mut t) = taunt else {
            if let Some(l) = self.lair.as_mut() {
                l.taunt_wait = wait.saturating_sub(1);
            }
            let free = du.f.is_neutral() && !du.puppet && du.lore.rise == 0;
            if !free || wait > 0 || taunts >= TAUNTS {
                return;
            }
            if let Some(du) = self.enemies[i].duel.as_deref_mut() {
                du.puppet = true;
                du.token = false;
            }
            if let Some(b) = self.brawl.as_mut() {
                b.threat.release(id);
            }
            if let Some(l) = self.lair.as_mut() {
                l.taunt = Some(Taunt::default());
            }
            self.floater(pos.x, pos.y - 2.6, "HE GOES TO THE CAGE", 1.4, Some(KRAMPUS_RED));
            return;
        };
        let stand = V::new(CAGE_STAND.0, CAGE_STAND.1);
        if !t.rattling {
            let Some(du) = self.enemies[i].duel.as_deref_mut() else { return };
            if t.walked > TAUNT_WALK {
                du.puppet = false;
                if let Some(l) = self.lair.as_mut() {
                    l.taunt = None;
                    l.taunt_wait = TAUNT_EVERY / 2;
                }
                return;
            }
            t.walked += 1;
            if !du.f.is_neutral() {
                // (Reeling from a blow on his way: he waits it out.)
                if let Some(l) = self.lair.as_mut() {
                    l.taunt = Some(t);
                }
                return;
            }
            let st = du.stats();
            let step = st.walk / 60.0;
            let left = math::len(math::sub(stand, du.pos));
            if left > step + 0.05 {
                let flow = Flow::new(&self.level.grid, P::new(stand.x.round() as i32, stand.y.round() as i32), 64);
                let ground = GridTerrain { grid: &self.level.grid, flow: &flow };
                let Some(du) = self.enemies[i].duel.as_deref_mut() else { return };
                let goal = if ground.clear(du.pos, stand) { stand } else { ground.toward(du.pos, stand) };
                let way = math::norm(math::sub(goal, du.pos));
                du.facing = math::norm(math::add(math::scale(du.facing, 0.8), math::scale(way, 0.2)));
                du.vel = math::scale(way, st.walk);
                du.pos = ground.slide(du.pos, math::scale(way, step), st.radius);
                if let Some(l) = self.lair.as_mut() {
                    l.taunt = Some(t);
                }
                return;
            }
            // At the cage: he turns to the bars, grips them and shakes them.
            du.vel = V::new(0.0, 0.0);
            du.pos = stand;
            du.facing = math::norm(math::sub(V::new(CAGE_MIDDLE.0, CAGE_MIDDLE.1), stand));
            if du.begin(MoveId::CageRattle, hp) {
                t.rattling = true;
                let at = du.pos;
                let listener = self.audio.listener();
                self.audio.tell(EnemyKind::Krampus, MoveId::CageRattle, at, listener);
                self.floater(at.x, at.y - 2.8, "HE RATTLES THE CAGE: STRIKE HIM!", 1.6, Some(0xffd040));
            }
            if let Some(l) = self.lair.as_mut() {
                l.taunt = Some(t);
            }
            return;
        }
        // Rattling: the bars ring, the boy cries; when it is over (or broken off), he is himself again.
        let Some(du) = self.enemies[i].duel.as_deref_mut() else { return };
        match du.f.state {
            State::Attack { mv: MoveId::CageRattle, frame } => {
                let d = du.data(MoveId::CageRattle);
                if d.phase(frame) == Phase::Recovery && (frame - d.startup).is_multiple_of(24) {
                    // The bars shaken again and again, the boy whimpering (the rattle's own sound).
                    let at = du.pos;
                    if (frame - d.startup).is_multiple_of(48) {
                        self.audio.swing(EnemyKind::Krampus, MoveId::CageRattle, at);
                    } else {
                        self.audio.play(Sfx::Chain);
                    }
                    self.view.shake(0.08);
                    if let Some(l) = self.lair.as_mut() {
                        l.child.fear = 1.0;
                        l.child.cry = 1.2;
                    }
                }
                if frame == d.startup {
                    let c = V::new(CAGE_MIDDLE.0, CAGE_MIDDLE.1);
                    self.floater(c.x, c.y - 2.0, "THE CHILD CRIES", 1.4, Some(FROST));
                    self.audio.play(Sfx::Bells);
                }
                if let Some(l) = self.lair.as_mut() {
                    l.taunt = Some(t);
                }
            }
            _ => {
                du.puppet = false;
                if let Some(l) = self.lair.as_mut() {
                    l.taunt = None;
                    l.taunts += 1;
                    l.taunt_wait = TAUNT_EVERY;
                }
            }
        }
    }

    /**
     * Krampus's health has fallen into `phase` (`court::prince_phase` hands it here): his bells
     * and his roar, the banner, a green cross for a breather; as the Chains begin his Perchten
     * come out of the dark (once), and Krampusnacht sets the cage's taunt going.
     */
    pub(super) fn krampus_phase(&mut self, i: usize, phase: u8) {
        let Some(du) = self.enemies.get(i).and_then(|e| e.duel.as_deref()) else { return };
        if du.arch != Archetype::Prince {
            return;
        }
        let (id, pos, kind, summoned) = (du.id, du.pos, du.breed.kind, du.lore.summoned);
        if let Some(l) = self.lair.as_mut() {
            l.phase = phase;
            l.child.fear = 1.0;
            if phase >= 3 {
                l.taunt_wait = TAUNT_FIRST;
            }
        }
        if let Some(b) = self.brawl.as_mut() {
            b.threat.release(id);
        }
        if let Some(d) = self.enemies[i].duel.as_deref_mut() {
            d.token = false;
            d.rest = 75;
        }
        self.audio.play(Sfx::BossRoar);
        self.audio.play(Sfx::Bells);
        self.audio.set_tempo_step(phase as u32 - 1);
        self.view.shake(0.55);
        self.feel_phase();
        self.view.sparkle(&self.level.grid, pos.x, pos.y, 1.6, BRASS, 44);
        let (sub, color) = Self::krampus_banner(phase);
        self.juice.banner = Some(Banner {
            text: format!("{} - {}", self.boss_name(), phase_name(kind, phase)),
            sub: sub.to_string(),
            color,
            life: 2.8,
            max: 2.8,
        });
        if phase >= 2 && !summoned {
            self.raise_perchten(id);
        }
        if let Some(d) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == id) {
            d.lore.summoned |= phase >= 2;
        }
        self.drop_breather(pos);
    }

    /// The phase banner's line under his name, and its colour.
    pub(super) fn krampus_banner(phase: u8) -> (&'static str, u32) {
        match phase {
            2 => ("HIS PERCHTEN COME   STEP OFF THE CHAIN'S LINE, BAIT HIS CHARGE INTO THE ICE", 0xd8a040),
            _ => ("THE COALS BURN ON: STEP OFF THEM   STRIKE HIM AT THE CAGE", 0xff4a2a),
        }
    }

    /// His Perchten come out of the two clefts in the wall (in the ring, either side of him),
    /// weighted as he is; they flee when he falls.
    fn raise_perchten(&mut self, id: u32) {
        let hp = self.hero.pos();
        let at = self.enemies.iter().filter_map(|e| e.duel.as_deref()).find(|d| d.id == id).map(|d| d.pos).unwrap_or(hp);
        let spots: Vec<V> = if self.in_lair() {
            CLEFTS.iter().map(|&c| plan::out_of_cleft(c).v()).collect()
        } else {
            let across = math::norm(V::new(-(hp.y - at.y), hp.x - at.x));
            [-1.0, 1.0].iter().map(|s| math::add(at, math::scale(across, 2.4 * s))).collect()
        };
        let (weight_hp, weight_damage) = lair_weight(self.realm_tier());
        for p in spots.into_iter().take(PERCHTEN as usize) {
            let facing = math::norm(math::sub(hp, p));
            let lore = Lore { rise: PERCHT_RISE, ..Lore::default() };
            if let Some(pid) = self.spawn_duelist(EnemyKind::Percht, p, facing, lore) {
                if let Some(d) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == pid) {
                    d.max_hp *= weight_hp * PERCHT_HEALTH;
                    d.hp = d.max_hp;
                    d.breed.damage *= weight_damage * PERCHT_DAMAGE;
                }
            }
            self.view.dust(&self.level.grid, p.x, p.y, 0xd8e4f0, 24, 1.8);
            self.audio.foe_cry(EnemyKind::Percht, Cry::Alert, p);
        }
        self.audio.play(Sfx::Bells);
        self.floater(hp.x, hp.y - 1.5, "HIS PERCHTEN COME OUT OF THE DARK", 1.8, Some(BRASS));
    }

    /**
     * Krampus falls (`brawl::slay`): the juice of the last blow (a freeze, time slowed, the screen
     * shaken), his Perchten fleeing, the floor cleared of his coals, and his slow death begun.
     */
    pub(super) fn krampus_slain(&mut self, at: V) {
        let facing = self
            .lair
            .as_ref()
            .and_then(|l| self.enemies.iter().filter_map(|e| e.duel.as_deref()).find(|d| d.id == l.krampus))
            .map(|d| d.facing)
            .unwrap_or(V::new(0.0, 1.0));
        if let Some(l) = self.lair.as_mut() {
            l.slain = true;
            l.trail = 0.0;
            l.taunt = None;
            l.fall = Some(Fall { at, facing, age: 0.0, bells: BELLS });
        }
        if let Some(b) = self.brawl.as_mut() {
            b.traps.clear();
            b.bolts.clear();
            b.held = None;
        }
        let perchten: Vec<usize> = (0..self.enemies.len())
            .filter(|&k| self.enemies[k].duel.as_ref().is_some_and(|d| d.alive() && d.breed.kind == EnemyKind::Percht))
            .collect();
        for k in perchten {
            let p = self.enemies[k].duel.as_ref().map(|d| d.pos).unwrap_or(at);
            if let Some(d) = self.enemies[k].duel.as_mut() {
                d.f.kill();
            }
            self.enemies[k].b.mode = EnemyMode::Eaten;
            self.view.dust(&self.level.grid, p.x, p.y, 0xd8e4f0, 18, 2.2);
        }
        self.freeze(Stop::Trophy);
        self.dilate(DilationKind::Trophy);
        self.view.trauma(Trauma::Trophy);
        self.view.shake(1.0);
        self.view.sparkle(&self.level.grid, at.x, at.y, 1.8, BRASS, 60);
        self.view.dust(&self.level.grid, at.x, at.y, 0xe8f0f8, 40, 3.6);
        self.audio.play(Sfx::BossDeath);
        self.audio.set_threat(0.0);
        self.award(crate::logic::boss::POINTS_CYCLOPS * 5);
        self.events.push(super::GameEvent::BossSlain);
        self.save_progress();
        self.juice.banner = Some(Banner {
            text: "KRAMPUS FALLS".into(),
            sub: "HIS BELLS FALL SILENT, ONE BY ONE".into(),
            color: BRASS,
            life: 4.0,
            max: 4.0,
        });
    }

    /// After his fall: the bells fall silent one by one, the cage's lock breaks and the cage comes
    /// down, the boy climbs out and runs to the hero; once they hold each other, the end.
    fn lair_aftermath(&mut self, dt: f64) {
        let hp = self.hero.pos();
        let Some(l) = self.lair.as_mut() else { return };
        let Some(mut fall) = l.fall else { return };
        let before = fall.age;
        fall.age += dt;
        let t = fall.age;
        // The bells, one by one.
        let silent = ((t / BELL_EVERY).floor() as u8).min(BELLS);
        let rang = BELLS - fall.bells;
        fall.bells = BELLS - silent;
        l.fall = Some(fall);
        let mut cage = l.cage;
        let crossed = |at: f64| before < at && t >= at;
        let lock = crossed(LOCK_BREAKS);
        let landed = crossed(LOCK_BREAKS + CAGE_FALL);
        let door = crossed(DOOR_OPENS);
        if t >= LOCK_BREAKS {
            cage.broken = true;
            cage.down = ((t - LOCK_BREAKS) / CAGE_FALL).min(1.0);
            cage.down *= cage.down;
        }
        if t >= DOOR_OPENS {
            cage.door = ((t - DOOR_OPENS) / 0.5).min(1.0);
        }
        l.cage = cage;
        // The boy.
        let mut c = l.child;
        c.age += dt;
        let mut held = false;
        match c.state {
            ChildState::Caged => {
                c.fear = (c.fear - dt as f32 * 0.4).max(0.0);
                if t >= CLIMBS_OUT {
                    c.state = ChildState::Climbing;
                    c.age = 0.0;
                }
            }
            ChildState::Climbing => {
                // Out of the door and down onto the floor before it.
                let from = V::new(CAGE_MIDDLE.0, CAGE_MIDDLE.1);
                let to = V::new(CAGE_STAND.0, CAGE_STAND.1 - 0.6);
                let u = (c.age / CLIMB).min(1.0);
                c.pos = math::add(from, math::scale(math::sub(to, from), u));
                c.facing = V::new(0.0, 1.0);
                if c.age >= CLIMB {
                    c.state = ChildState::Running;
                    c.age = 0.0;
                }
            }
            ChildState::Running => {
                let to = math::sub(hp, c.pos);
                let d = math::len(to);
                if d <= CHILD_REACH {
                    c.state = ChildState::Held;
                    c.age = 0.0;
                    held = true;
                } else {
                    let flow = Flow::new(&self.level.grid, P::new(hp.x.round() as i32, hp.y.round() as i32), 64);
                    let ground = GridTerrain { grid: &self.level.grid, flow: &flow };
                    let goal = if ground.clear(c.pos, hp) { hp } else { ground.toward(c.pos, hp) };
                    let way = math::norm(math::sub(goal, c.pos));
                    let step = (CHILD_PACE * dt).min(d - CHILD_REACH * 0.9);
                    // (Out of the cage's bars first: from inside its stone, straight to the floor.)
                    c.pos = if self.level.grid.is_walkable(c.pos.x.round() as i32, c.pos.y.round() as i32, false) {
                        ground.slide(c.pos, math::scale(way, step), 0.25)
                    } else {
                        math::add(c.pos, math::scale(way, step))
                    };
                    c.facing = way;
                }
            }
            ChildState::Held => {
                c.facing = math::norm(math::sub(hp, c.pos));
            }
        }
        let embraced = c.state == ChildState::Held && c.age >= EMBRACE;
        if let Some(l) = self.lair.as_mut() {
            l.child = c;
        }
        // What is heard and seen of it.
        if silent > rang {
            let p = fall.at;
            self.audio.play(Sfx::Toll);
            self.view.sparkle(&self.level.grid, p.x, p.y, 1.4, BRASS, 6);
            if silent == BELLS {
                self.floater(p.x, p.y - 1.6, "THE LAST BELL FALLS SILENT", 2.2, Some(BRASS));
                self.view.dust(&self.level.grid, p.x, p.y, 0x2a2228, 50, 2.4);
            }
        }
        let cage_at = V::new(CAGE_MIDDLE.0, CAGE_MIDDLE.1);
        if lock {
            self.audio.play(Sfx::Clang);
            self.view.sparkle(&self.level.grid, cage_at.x, cage_at.y, 1.2, BRASS, 30);
            self.floater(cage_at.x, cage_at.y - 2.2, "THE LOCK BREAKS", 1.8, Some(BRASS));
            self.audio.play(Sfx::Chain);
        }
        if landed {
            self.audio.play(Sfx::BossSlam);
            self.view.dust(&self.level.grid, cage_at.x, cage_at.y + 0.6, 0xe8f0f8, 40, 3.0);
            self.view.shake(0.5);
        }
        if door {
            self.audio.play(Sfx::DoorGrind);
        }
        if held {
            let p = math::scale(math::add(hp, c.pos), 0.5);
            self.view.sparkle(&self.level.grid, p.x, p.y, 0.8, 0xfff4d8, 40);
            self.floater(p.x, p.y - 1.6, "TOGETHER AGAIN", 2.4, Some(0xfff4d8));
            self.audio.play(Sfx::Soul);
        }
        if embraced && !self.lair.as_ref().is_some_and(|l| l.won) {
            self.lair_won();
        }
    }

    /// A move of Krampus's comes to its first active frame (`brawl::first_active`): the bells
    /// shaken and the floor with them, the sack brought down, the chain cracking out along its line.
    pub(super) fn krampus_strikes(&mut self, i: usize, mv: MoveId) {
        let Some(du) = self.enemies.get(i).and_then(|e| e.duel.as_deref()) else { return };
        let (pos, aim, span) = (du.pos, du.aim, du.lore.span);
        let d = du.data(mv);
        match mv {
            MoveId::TollOfBells | MoveId::SackSwing => {
                let at = if mv == MoveId::TollOfBells { pos } else { math::add(pos, math::scale(aim, d.reach * 0.7)) };
                let mut f = Flash::new(FlashKind::Quake, at);
                if let Shape::Quake { radius, .. } = d.shape {
                    f.length = radius;
                }
                if let Some(b) = self.brawl.as_mut() {
                    b.flashes.push(f);
                }
                self.audio.play(Sfx::BossSlam);
                if mv == MoveId::TollOfBells {
                    self.audio.play(Sfx::Bells);
                    self.view.sparkle(&self.level.grid, pos.x, pos.y, 1.6, BRASS, 18);
                }
                self.feel_quake(at);
                self.view.dust(&self.level.grid, at.x, at.y, 0xe8f0f8, 30, 3.0);
                if let Some(l) = self.lair.as_mut() {
                    l.child.fear = l.child.fear.max(0.8);
                }
            }
            MoveId::ChainLash => {
                let end = math::add(pos, math::scale(aim, span));
                self.view.shake(0.18);
                self.view.sparkle(&self.level.grid, end.x, end.y, 0.4, 0xb8c0cc, 20);
                self.view.dust(&self.level.grid, end.x, end.y, 0xe8f0f8, 14, 1.6);
                self.audio.play(Sfx::Chain);
            }
            _ => self.audio.play(Sfx::Chain),
        }
    }

    /// Capture hook: Krampus's last blow, whatever his health (his death, the cage and the boy follow).
    pub fn debug_fell(&mut self) {
        let Some(id) = self.lair.as_ref().map(|l| l.krampus) else { return };
        let Some(i) = self.enemies.iter().position(|e| e.duel.as_ref().is_some_and(|d| d.id == id && d.alive())) else { return };
        let from = self.hero.pos();
        if let Some(d) = self.enemies[i].duel.as_deref_mut() {
            d.hp = 1.0;
            d.breed.phase = 3;
            d.lore.summoned = true;
            d.f.state = State::Attack { mv: MoveId::SackSwing, frame: 70 };
        }
        let d = MoveId::L1.data();
        let hit = crate::logic::combat::fsm::Hit { damage: d.damage, poise: d.poise, hitstun: d.hitstun, ..Default::default() };
        self.duel_hit(i, hit, 0, from);
    }

    /// Capture hook: Krampusnacht, and Krampus goes to the cage now.
    pub fn debug_taunt(&mut self) {
        let Some(id) = self.lair.as_ref().map(|l| l.krampus) else { return };
        if let Some(d) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == id) {
            d.hp = d.hp.min(d.max_hp * 0.39);
            d.breed.phase = 3;
            d.lore.summoned = true;
            d.f.state = State::Neutral;
        }
        if let Some(l) = self.lair.as_mut() {
            l.phase = 3;
            l.taunt_wait = 0;
            l.taunts = 0;
        }
    }

    /// After a fall: Krampus whole again before the cage, his Perchten gone, the floor cleared, the
    /// boy back at the bars (unless Krampus has already fallen).
    pub(super) fn reset_lair(&mut self) {
        let Some(l) = self.lair.as_ref() else { return };
        if l.slain {
            return;
        }
        let keep = l.krampus;
        self.enemies.retain(|e| e.duel.as_ref().is_some_and(|d| d.id == keep));
        if let Some(d) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).next() {
            d.reset_home();
            d.pos = P::new(KRAMPUS.0, KRAMPUS.1).v();
            d.facing = V::new(0.0, 1.0);
            d.rest = 90;
            d.puppet = false;
            d.lore.summoned = false;
        }
        if let Some(b) = self.brawl.as_mut() {
            b.traps.clear();
            b.bolts.clear();
        }
        let max = self.enemies.first().and_then(|e| e.duel.as_ref()).map(|d| d.max_hp as f32).unwrap_or(0.0);
        if let Some(l) = self.lair.as_mut() {
            *l = Lair { charges: l.charges, dazes: l.dazes, frame: l.frame, ..Lair::new(keep, max) };
        }
        self.refresh_foe_views();
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    /// A game in Krampus's lair, playing, the hero free to walk.
    pub fn lair() -> Game {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test-lair"));
        let mut g = Game::new();
        g.map_jump(Stage::Lair);
        g.enter(Screen::Playing);
        g.hero.free_move = true;
        g
    }

    /// Krampus, in the lair `g` is in.
    pub fn krampus(g: &mut Game) -> &mut Duelist {
        let id = g.lair.as_ref().unwrap().krampus;
        g.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == id).unwrap()
    }

    /// `secs` of play.
    pub fn run(g: &mut Game, secs: f64) {
        for _ in 0..(secs / crate::app::STEP) as usize {
            g.update(crate::app::STEP);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{krampus, lair, run};
    use super::*;
    use crate::logic::combat::frames::REACTION;
    use crate::logic::combat::prince::krampus::{phase_of, PHASE_AT};
    use crate::logic::combat::prince::LINGER_EVERY;
    use crate::logic::lair::{column_middle, COLUMNS, NAME};
    use crate::logic::types::Tile;

    fn perchten(g: &Game) -> usize {
        g.enemies.iter().filter(|e| e.kind() == EnemyKind::Percht && e.duel.as_ref().is_some_and(|d| d.alive())).count()
    }

    /// Krampus still, out of the threat manager's way, at `at` facing `facing`, about to begin a move.
    fn set(g: &mut Game, at: V, facing: V) {
        let d = krampus(g);
        d.pos = at;
        d.facing = facing;
        d.rest = u16::MAX;
        d.token = true;
        d.f.state = State::Neutral;
    }

    #[test]
    fn the_lair_is_entered_on_the_platform_and_krampus_fights_there_by_frame_data() {
        let mut g = lair();
        assert_eq!(g.level.data.name, NAME);
        assert!(g.in_lair() && !g.in_court() && g.boss.is_none(), "no Cyclops, no court");
        assert_eq!(g.realm, None);
        assert_eq!(g.realm_tier(), 8, "all seven keys held");
        assert!(g.arena() && g.brawling(), "a boss's arena, fought by frame data");
        let b = g.brawl.as_ref().unwrap();
        assert_eq!((b.threat.capacity, b.threat.cadence), (COURT_CAPACITY, COURT_CADENCE));
        let p = g.hero.pos();
        assert!(plan::on_platform(p.x.round() as i32, p.y.round() as i32), "he stands on the platform");
        let d = g.prince().expect("Krampus stands, found as a prince is");
        assert_eq!((d.breed.kind, d.arch, d.breed.phase), (EnemyKind::Krampus, Archetype::Prince, 1));
        assert_eq!(d.stats().cost, 3);
        assert_eq!(d.pos, P::new(KRAMPUS.0, KRAMPUS.1).v());
        assert_eq!(g.enemies.len(), 1, "he waits alone");
        let bar = g.prince_bar().unwrap();
        assert_eq!((bar.name.as_str(), bar.title), ("KRAMPUS", "THE BELLS"));
        assert_eq!(bar.notches, [PHASE_AT[1] as f32, PHASE_AT[0] as f32], "his bar notched at his own thresholds");
        assert!(bar.hide);
        // He is heavier than any prince at the tier he is met at.
        let max = krampus(&mut g).max_hp;
        let (w, _) = weight(8);
        assert!(max > 900.0 * w * 1.3, "{max}");
        // The way in: the platform comes to rest, the adventure saved there.
        let mut h = Game::new();
        h.enter_lair();
        assert!(h.in_lair() && h.screen == Screen::Ready);
        assert_eq!(h.saved_run.as_ref().map(|r| r.stage), Some(Stage::Lair));
    }

    #[test]
    fn his_phases_fall_at_seventy_and_forty_and_the_perchten_come_once() {
        let mut g = lair();
        let max = krampus(&mut g).max_hp;
        krampus(&mut g).rest = u16::MAX;
        let from = krampus(&mut g).pos;
        krampus(&mut g).hp = max * 0.72;
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        let hp = krampus(&mut g).hp;
        assert_eq!(krampus(&mut g).breed.phase, phase_of(hp, max));
        assert_eq!(perchten(&g), 0);
        // Under 70%: the Chains, his Perchten, a green cross.
        krampus(&mut g).hp = max * 0.65;
        krampus(&mut g).rest = u16::MAX;
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        assert_eq!(krampus(&mut g).breed.phase, 2);
        assert_eq!(g.lair.as_ref().unwrap().phase, 2);
        assert_eq!(perchten(&g), PERCHTEN as usize, "his Perchten come");
        assert!(g.level.grid.count(Tile::Charge) >= 1, "a breather");
        assert!(g.juice.banner.as_ref().is_some_and(|b| b.text.contains("THE CHAINS")));
        assert_eq!(krampus(&mut g).breed.moves()[0], MoveId::ChainLash);
        // Under 40%: Krampusnacht. The Perchten are not called again.
        krampus(&mut g).hp = max * 0.38;
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        assert_eq!(krampus(&mut g).breed.phase, 3);
        assert_eq!(perchten(&g), PERCHTEN as usize, "called once");
        assert!(g.juice.banner.as_ref().is_some_and(|b| b.text.contains("KRAMPUSNACHT")));
        assert_eq!(krampus(&mut g).breed.moves(), &[MoveId::CoalFire, MoveId::HoofCharge, MoveId::ChainLash, MoveId::SackSwing]);
    }

    #[test]
    fn his_pelt_turns_most_of_a_blow_but_not_in_his_recoveries() {
        let mut g = lair();
        let from = krampus(&mut g).pos;
        krampus(&mut g).rest = u16::MAX;
        let before = krampus(&mut g).hp;
        g.call_duel_hit(0, from);
        let shut = before - krampus(&mut g).hp;
        let d = krampus(&mut g);
        d.f = d.breed.fighter();
        d.f.state = State::Attack { mv: MoveId::BirchLash, frame: 60 };
        assert!(d.exposed());
        let before = d.hp;
        g.call_duel_hit(0, from);
        let open = before - krampus(&mut g).hp;
        assert!(shut > 0.0 && open > shut * 2.5, "pelt {shut}, open {open}");
    }

    /// Krampus west of a column of ice, the hero standing before it; he charges, the hero steps
    /// aside as the aim locks, and the charge runs on into the ice.
    #[test]
    fn a_charge_run_into_the_ice_dazes_him_and_on_open_floor_it_does_not() {
        let mut g = lair();
        g.hero.hp = 5000;
        let column = COLUMNS[1];
        let m = column_middle(column);
        let stand = P::new(column.0 - 1, column.1);
        g.hero.place(stand);
        let hp = g.hero.pos();
        set(&mut g, V::new(m.0 - 7.0, hp.y), V::new(1.0, 0.0));
        krampus(&mut g).breed.phase = 2;
        krampus(&mut g).lore.summoned = true;
        assert!(krampus(&mut g).begin(MoveId::HoofCharge, hp));
        let track = MoveId::HoofCharge.data().track;
        for _ in 0..(track as usize + 2) * 2 {
            g.update(crate::app::STEP);
        }
        g.hero.place(P::new(stand.x - 1, stand.y + 3));
        let mut dazed = false;
        for _ in 0..240 {
            g.update(crate::app::STEP);
            if krampus(&mut g).f.reeling() == Some(Reel::Staggered) {
                dazed = true;
                break;
            }
        }
        assert!(dazed, "into the ice, and dazed: {:?}", krampus(&mut g).pos);
        let d = krampus(&mut g);
        assert!(d.exposed(), "dazed and open");
        assert!(d.pos.x < m.0 - 1.0, "stopped short of the column: {:?}", d.pos);
        let frames = match d.f.state {
            State::Reel { left, .. } => left,
            _ => 0,
        };
        assert!(frames > 120, "{frames}");
        assert!(d.f.vulnerability() > 1.0);
        assert_eq!(g.lair.as_ref().unwrap().dazes, 1);

        // On open floor the charge runs its length and he recovers, undazed.
        let mut g = lair();
        g.hero.hp = 5000;
        g.hero.place(P::new(16, 21));
        let hp = g.hero.pos();
        set(&mut g, V::new(16.0, 12.0), V::new(0.0, 1.0));
        assert!(krampus(&mut g).begin(MoveId::HoofCharge, hp));
        for _ in 0..(track as usize + 2) * 2 {
            g.update(crate::app::STEP);
        }
        g.hero.place(P::new(12, 25));
        run(&mut g, 1.6);
        let d = krampus(&mut g);
        assert!(d.f.reeling().is_none() && !d.crashed, "{:?}", d.f.state);
        assert!(d.pos.y > 17.0, "he ran on: {:?}", d.pos);
    }

    #[test]
    fn his_coals_glow_late_enough_to_step_off_and_burn_on_while_he_stands_in_them() {
        let mut g = lair();
        g.hero.hp = 5000;
        g.hero.place(P::new(16, 21));
        let hp = g.hero.pos();
        set(&mut g, V::new(16.0, 14.0), V::new(0.0, 1.0));
        krampus(&mut g).breed.phase = 3;
        krampus(&mut g).lore.summoned = true;
        g.lair.as_mut().unwrap().taunts = TAUNTS;
        assert!(krampus(&mut g).begin(MoveId::CoalFire, hp));
        let mut laid = Vec::new();
        for _ in 0..400 {
            g.update(crate::app::STEP);
            let b = g.brawl.as_ref().unwrap();
            if b.traps.iter().any(|t| t.armed()) {
                laid = b.traps.clone();
                break;
            }
        }
        assert!(laid.len() >= 4, "a row of coals: {}", laid.len());
        assert!(laid.iter().all(|t| (t.pos.x - 16.0).abs() < 0.3 && t.pos.y > 14.0), "along his aim");
        let on = laid.iter().find(|t| t.reaches(hp, 0.3)).expect("the row runs under him");
        let walk = 6.5 * crate::logic::levels::SPEED_SCALE;
        let gained = on.fuse.saturating_sub(REACTION) as f64 * walk / 60.0 - 0.25;
        assert!(gained >= (on.radius + 0.3) * 1.2, "{gained}");
        // Stepped off at once: untouched by the burst.
        let start = g.hero.hp;
        g.hero.place(P::new(18, 21));
        let mut burst = false;
        for _ in 0..300 {
            g.update(crate::app::STEP);
            if g.brawl.as_ref().unwrap().traps.iter().any(|t| t.burst.is_some()) {
                burst = true;
                break;
            }
        }
        assert!(burst);
        run(&mut g, 0.2);
        assert_eq!(g.hero.hp, start, "off the coals, untouched");
        // Stood in them as they burn on, they sting again and again (and never hold him).
        let t = g.brawl.as_ref().unwrap().traps[2];
        g.hero.place(P::new(t.pos.x.round() as i32, t.pos.y.round() as i32));
        krampus(&mut g).rest = u16::MAX;
        krampus(&mut g).token = false;
        let before = g.hero.hp;
        run(&mut g, LINGER_EVERY as f64 * 3.0 / 60.0);
        assert!(g.hero.hp < before, "burnt: {} of {before}", g.hero.hp);
        assert_eq!(g.hero.root, 1.0, "never slowed");
    }

    #[test]
    fn in_krampusnacht_he_goes_to_the_cage_rattles_it_and_stands_open_while_the_child_cries() {
        let mut g = lair();
        g.hero.hp = 5000;
        g.hero.place(P::new(16, 24));
        let max = krampus(&mut g).max_hp;
        let from = krampus(&mut g).pos;
        krampus(&mut g).hp = max * 0.39;
        krampus(&mut g).rest = u16::MAX;
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        assert_eq!(krampus(&mut g).breed.phase, 3);
        g.lair.as_mut().unwrap().taunt_wait = 0;
        krampus(&mut g).f.state = State::Neutral;
        let mut rattling = false;
        for _ in 0..(12.0 / crate::app::STEP) as usize {
            g.update(crate::app::STEP);
            g.hero.hp = 5000;
            if matches!(krampus(&mut g).f.attack(), Some((MoveId::CageRattle, Phase::Recovery, _))) {
                rattling = true;
                break;
            }
        }
        assert!(rattling, "he rattles the cage: {:?}", krampus(&mut g).pos);
        let d = krampus(&mut g);
        let stand = V::new(CAGE_STAND.0, CAGE_STAND.1);
        assert!(math::len(math::sub(d.pos, stand)) < 0.3, "before its door");
        assert!(d.exposed() && d.taken() == 1.0, "his pelt open while he rattles it");
        run(&mut g, 0.5);
        let l = g.lair.as_ref().unwrap();
        assert!(l.child.fear >= 0.9 && l.child.cry > 0.0, "the child cries");
        assert_eq!(l.child.state, ChildState::Caged, "and is untouched");
        // A blow there lands whole, as a punish.
        let d = krampus(&mut g);
        let before = d.hp;
        let at = d.pos;
        g.call_duel_hit(0, math::add(at, V::new(0.0, 2.0)));
        assert!(before - krampus(&mut g).hp > 0.0);
        // Once it is over he is himself again, and the next waits.
        run(&mut g, 4.0);
        let l = g.lair.as_ref().unwrap();
        assert!(l.taunt.is_none() && l.taunts == 1 && l.taunt_wait > 0, "{l:?}");
        assert!(!krampus(&mut g).puppet);
    }

    #[test]
    fn nothing_in_the_fight_touches_the_child_or_the_cage() {
        let mut g = lair();
        g.hero.hp = 5000;
        // The hero stands just before the cage: coals flung at him, a charge, a sweep.
        g.hero.place(P::new(16, 6));
        let hp = g.hero.pos();
        set(&mut g, V::new(16.0, 13.0), V::new(0.0, -1.0));
        krampus(&mut g).breed.phase = 3;
        krampus(&mut g).lore.summoned = true;
        g.lair.as_mut().unwrap().taunts = TAUNTS;
        assert!(krampus(&mut g).begin(MoveId::CoalFire, hp));
        let cage: Vec<(i32, i32)> =
            (plan::CAGE.1..=plan::CAGE.3).flat_map(|y| (plan::CAGE.0..=plan::CAGE.2).map(move |x| (x, y))).collect();
        for _ in 0..(10.0 / crate::app::STEP) as usize {
            g.update(crate::app::STEP);
            g.hero.hp = 5000;
            let b = g.brawl.as_ref().unwrap();
            for t in &b.traps {
                let at = (t.pos.x.round() as i32, t.pos.y.round() as i32);
                assert!(!cage.contains(&at), "no coal on the cage: {at:?}");
            }
            let l = g.lair.as_ref().unwrap();
            assert_eq!(l.child.state, ChildState::Caged);
            assert_eq!(l.child.pos, V::new(CAGE_MIDDLE.0, CAGE_MIDDLE.1), "the boy stays where he is");
            for &(x, y) in &cage {
                assert_eq!(g.level.grid.get(x, y), Tile::Rock, "the cage stands");
            }
        }
        // He never chooses the boy for a target: every one of his moves is begun on the hero.
        assert!(krampus(&mut g).alive());
    }

    #[test]
    fn slain_his_bells_fall_silent_the_cage_opens_and_the_child_runs_to_the_hero() {
        let mut g = lair();
        g.hero.place(P::new(16, 20));
        let max = krampus(&mut g).max_hp;
        let from = krampus(&mut g).pos;
        krampus(&mut g).hp = max * 0.3;
        krampus(&mut g).breed.phase = 3;
        krampus(&mut g).lore.summoned = true;
        let ids = g.spawn_duelist(EnemyKind::Percht, V::new(10.0, 14.0), V::new(1.0, 0.0), Lore::default());
        assert!(ids.is_some());
        krampus(&mut g).hp = 1.0;
        krampus(&mut g).f.state = State::Attack { mv: MoveId::SackSwing, frame: 70 };
        g.call_duel_hit(0, from);
        assert!(g.prince().is_none() && g.krampus().is_none(), "he has fallen");
        let l = g.lair.as_ref().unwrap();
        assert!(l.slain && l.fall.is_some_and(|f| f.bells == BELLS));
        assert_eq!(perchten(&g), 0, "his Perchten flee");
        assert!(g.red_key.is_none(), "no key: he is no sin's prince");
        // The bells, one by one.
        run(&mut g, BELL_EVERY * 2.5);
        let bells = g.lair.as_ref().unwrap().fall.unwrap().bells;
        assert!(bells < BELLS && bells > 0, "{bells}");
        assert!(!g.lair.as_ref().unwrap().cage.broken, "the cage still hangs");
        // The lock breaks and the cage comes down; the door opens; the boy climbs out and runs.
        // (Time runs slow a moment after the killing blow: the death plays out on the world's clock.)
        for _ in 0..(10.0 / crate::app::STEP) as usize {
            g.update(crate::app::STEP);
            if g.lair.as_ref().unwrap().cage.down >= 1.0 {
                break;
            }
            assert_eq!(g.lair.as_ref().unwrap().child.state, ChildState::Caged, "the boy waits for the cage to come down");
        }
        let l = g.lair.as_ref().unwrap();
        assert!(l.cage.broken && l.cage.down >= 1.0 && l.fall.unwrap().bells == 0);
        let mut reached = false;
        for _ in 0..(12.0 / crate::app::STEP) as usize {
            g.update(crate::app::STEP);
            if g.lair.as_ref().unwrap().child.state == ChildState::Held {
                reached = true;
                break;
            }
            assert_ne!(g.screen, Screen::Won, "not before the boy has reached him");
        }
        assert!(reached, "the boy reaches the hero: {:?}", g.lair.as_ref().unwrap().child);
        let c = g.lair.as_ref().unwrap().child;
        assert!(math::len(math::sub(c.pos, g.hero.pos())) <= CHILD_REACH + 0.05);
        run(&mut g, EMBRACE + 0.2);
        assert!(g.lair.as_ref().unwrap().won);
        assert!(g.finished && g.story.is_some(), "lair_won: the end begins");
    }

    #[test]
    fn a_fall_brings_krampus_back_whole_and_continue_comes_back_to_his_lair() {
        let mut g = lair();
        let max = krampus(&mut g).max_hp;
        krampus(&mut g).hp = max * 0.5;
        krampus(&mut g).breed.phase = 2;
        krampus(&mut g).pos = V::new(10.0, 18.0);
        g.spawn_duelist(EnemyKind::Percht, V::new(10.0, 14.0), V::new(1.0, 0.0), Lore::default());
        g.lair.as_mut().unwrap().taunts = 2;
        g.reset_round();
        let d = krampus(&mut g);
        assert_eq!((d.hp, d.breed.phase), (max, 1));
        assert_eq!(d.pos, P::new(KRAMPUS.0, KRAMPUS.1).v(), "back before the cage");
        assert_eq!(perchten(&g), 0, "his Perchten gone");
        let l = g.lair.as_ref().unwrap();
        assert_eq!((l.phase, l.taunts), (1, 0));
        g.checkpoint(Stage::Lair);
        let saved = g.saved_run.clone().unwrap();
        let mut h = Game::new();
        h.saved_run = Some(saved);
        h.continue_game();
        assert!(h.in_lair() && h.prince().is_some());
        assert_eq!(h.level.data.name, NAME);
        assert_eq!(h.realm_tier(), 8);
    }

    #[test]
    fn the_map_has_the_lair_and_the_old_man_tells_of_it() {
        let stops = crate::game::map_stops();
        assert!(stops.iter().any(|s| s.stage == Stage::Lair));
        let g = lair();
        assert_eq!(g.guide_place(), crate::logic::guide::GuidePlace::Lair);
        let (page, _) = g.guide_now();
        assert!(page.tale.iter().any(|l| l.contains("Krampus")));
    }

    /// FINALE.md §3: Krampus by the bots, with the kit a hero has at the bottom of the Pit (tiers 8
    /// and 7, all seven keys held): the expert brings him down through all three phases with lives
    /// to spare, in an epic's time; a masher loses every life to him.
    #[test]
    fn the_expert_beats_krampus_in_his_lair_and_a_masher_does_not() {
        use crate::game::bot::{lair_fight, Loadout, Skill};
        use crate::logic::heroes::HeroKind;
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        for (tier, seed) in [(8, 1), (7, 2)] {
            let f = lair_fight(Skill::Expert, seed, Some(Loadout::all(HeroKind::Ragnar, tier)), 300.0);
            assert!(f.fight.slain && f.fight.deaths <= 1 && f.fight.phase == 3, "tier {tier}: {f:?}");
            assert!(f.fight.secs > 120.0 && f.fight.secs < 260.0, "tier {tier}: an epic's length, {f:?}");
            assert_eq!(f.fight.stats.overlaps, 0, "{f:?}");
        }
        let f = lair_fight(Skill::Novice, 1, Some(Loadout::all(HeroKind::Ragnar, 8)), 300.0);
        assert!(!f.fight.slain && f.fight.game_over, "{f:?}");
    }
}
