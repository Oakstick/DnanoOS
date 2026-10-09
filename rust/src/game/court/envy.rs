//! Envy's court in play: the Drowned Bell (`logic::court::envy` lays it out) and Leviathan, its
//! prince (`logic::combat::prince::envy` holds his rules).
//!
//! - **The covet.** Every archangel the hero calls on is watched (a blessing's charge spent, his
//!   patron's wait begun again); when Leviathan begins his covet it becomes that angel's power in
//!   green (Raphael's, if none has been called), and the banner says so. Its strike is planned
//!   again by the threat manager as what it has become.
//! - **The serpentlings.** As his second phase begins two rise from the water at the springs
//!   nearest the hero (at once, if his general's decree was done; never, if his general fell).
//! - **The bell.** As his third phase begins he swims to the bell and coils round it, and keeps
//!   there (`prince::envy::COIL`); its heat has burnt his scales off on the side he turns to it
//!   (`Lore::bell`). His toll's mark is the bell's; his burning coil's fires are laid round it
//!   and burst together; Gabriel's coveted thunder falls on three marks.

use crate::audio::Sfx;
use crate::game::{Banner, Game};
use crate::logic::angels::Archangel;
use crate::logic::combat::ails::{Flash, FlashKind};
use crate::logic::combat::duelist::{lands_after, Lore, HERO_RADIUS};
use crate::logic::combat::frames::Shape;
use crate::logic::combat::prince::envy::{
    coiled, covet, coveted, fire_spots, thunder_marks, FIRE_RADIUS, SERPENTLINGS, SERPENTLING_RISE, SWIM,
};
use crate::logic::combat::{math, MoveId, State};
use crate::logic::court::adds_weight;
use crate::logic::court::envy::{springs_near, BELL};
use crate::logic::enemy::EnemyKind;
use crate::logic::types::V;

/// The court's green: the covet's tell, its banner, the serpent's light.
pub const ENVY_GREEN: u32 = 0x5adf7a;

/// What Envy's court keeps of its fight (`Court::envy`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EnvyState {
    /// The archangel whose power the hero last called on in the court.
    pub angel: Option<Archangel>,
    /// His blessing (its angel and charges) and his patron's wait as last seen, to see a call.
    seen: Option<(Option<(Archangel, u8)>, f64)>,
    /// The prince's move and its frame as last seen (each move's first frames are met once).
    last: Option<(MoveId, u16)>,
}

impl Game {
    /// As Leviathan rises before his altar.
    pub(super) fn envy_enter(&mut self, _prince: u32) {
        self.envy_watch_angels();
    }

    /**
     * Each tick of the fight under the bell: the archangels called on are watched; in his third
     * phase he swims to the bell and keeps coiled round it; his moves as they begin and as they
     * strike do what is Envy's own (`envy_moves`).
     */
    pub(super) fn envy_tick(&mut self, dt: f64) {
        self.envy_watch_angels();
        let Some(id) = self.court.as_ref().map(|c| c.prince) else { return };
        let Some(i) = self.enemies.iter().position(|e| e.duel.as_ref().is_some_and(|d| d.id == id && d.alive())) else {
            return;
        };
        // Coiled round the bell: he swims back to it whenever he is farther than his coil.
        if let Some(d) = self.enemies[i].duel.as_deref_mut().filter(|d| d.lore.bell.is_some()) {
            let goal = coiled(d.pos, BELL);
            let gap = math::sub(goal, d.pos);
            let step = SWIM * dt;
            if math::len(gap) > 1e-6 {
                d.pos = if math::len(gap) <= step { goal } else { math::add(d.pos, math::scale(math::norm(gap), step)) };
            }
        }
        self.envy_moves(i);
    }

    /// The archangels he calls on: a blessing's charge spent, or his patron's wait begun again.
    fn envy_watch_angels(&mut self) {
        let blessing = self.blessing.map(|b| (b.angel, b.charges));
        let wait = self.patron_wait;
        let patron = self.progress.hero(self.hero_kind).patron;
        let Some(c) = self.court.as_mut() else { return };
        if let Some((was, waited)) = c.envy.seen {
            if let Some((angel, charges)) = was {
                if blessing.is_none_or(|(a, n)| a != angel || n < charges) {
                    c.envy.angel = Some(angel);
                }
            }
            if wait > waited + 0.5 {
                c.envy.angel = patron.or(c.envy.angel);
            }
        }
        c.envy.seen = Some((blessing, wait));
    }

    /**
     * Leviathan's move, as it begins and as it strikes: his covet becomes the power he covets; his
     * toll's mark is the bell; his burning coil marks where the hero stood (its fires are reckoned
     * from it); and on their first active frame the fires burst, the thunder's two other marks
     * fall, the dive breaks the water.
     */
    fn envy_moves(&mut self, i: usize) {
        let hp = self.hero.pos();
        let Some(d) = self.enemies[i].duel.as_deref() else { return };
        let last = self.court.as_ref().and_then(|c| c.envy.last);
        let State::Attack { mv, frame } = d.f.state else {
            if let Some(c) = self.court.as_mut() {
                c.envy.last = None;
            }
            return;
        };
        let fresh = last.is_none_or(|(m, f)| m != mv || f > frame);
        let mut mv = mv;
        if mv == MoveId::Covet {
            mv = self.envy_covet(i);
        }
        let Some(d) = self.enemies[i].duel.as_deref_mut() else { return };
        match mv {
            MoveId::BellToll if frame < mv.data().startup => d.lore.mark = BELL,
            MoveId::BurningCoil if fresh => {
                d.lore.mark = hp;
                let p = d.pos;
                self.floater(p.x, p.y - 2.6, "THE BELL BURNS", 1.2, Some(0xff8a3a));
            }
            _ => {}
        }
        let startup = mv.data().startup;
        let striking = frame >= startup && last.is_none_or(|(m, f)| m != mv || f < startup);
        if striking {
            self.envy_strikes(i, mv);
        }
        if let Some(c) = self.court.as_mut() {
            c.envy.last = Some((mv, frame));
        }
    }

    /**
     * His covet as it begins: it becomes the power of the archangel the hero last called on (as it
     * starts up alike, only its landing may move: a volley's bolts fly), planned again by the
     * threat manager; if the coveted fire would land too near another strike, he copies Raphael's
     * beam, which lands when the covet would have. Returns what it has become.
     */
    fn envy_covet(&mut self, i: usize) -> MoveId {
        let hp = self.hero.pos();
        let angel = self.court.as_ref().and_then(|c| c.envy.angel);
        let Some(d) = self.enemies[i].duel.as_deref() else { return MoveId::Covet };
        let (id, frame, dist) = (d.id, d.f.attack().map(|a| a.2).unwrap_or(0), math::len(math::sub(hp, d.pos)));
        let mut mv = covet(angel);
        if let Some(b) = self.brawl.as_mut() {
            if let Some(at) = b.threat.landing(id) {
                let began = at.saturating_sub(MoveId::Covet.data().startup as u32);
                let lands = began + lands_after(&d.data(mv), dist) as u32;
                let left = lands.saturating_sub(b.threat.frame) as u16;
                if lands != at {
                    if b.threat.may_commit(id, left) {
                        b.threat.commit(id, left);
                    } else {
                        mv = MoveId::CovetBeam;
                    }
                }
            }
        }
        let Some(d) = self.enemies[i].duel.as_deref_mut() else { return mv };
        d.f.state = State::Attack { mv, frame };
        if mv == MoveId::CovetThunder {
            d.lore.mark = hp;
        }
        let p = d.pos;
        let name = coveted(mv).unwrap_or(Archangel::Raphael).name().to_uppercase();
        let sub = match mv {
            MoveId::CovetFire => "FOUR BOLTS OF GREEN FIRE   GUARD, OR STEP OFF THEIR LINES",
            MoveId::CovetThunder => "THREE MARKS ROUND YOU   GO TOWARD HIM, OR STRAIGHT BACK",
            MoveId::CovetWave => "THE FLOOR WILL SHAKE ROUND HIM   GUARD, OR GET CLEAR",
            _ => "A BEAM ALONG THE GREEN LINE   STEP OFF IT, OR BLOCK",
        };
        self.juice.banner =
            Some(Banner { text: format!("HE COVETS YOUR {name}"), sub: sub.to_string(), color: ENVY_GREEN, life: 1.9, max: 1.9 });
        self.view.sparkle(&self.level.grid, p.x, p.y, 2.6, ENVY_GREEN, 30);
        self.audio.play(Sfx::Whisper);
        mv
    }

    /// The first active frame of his move `mv`: what Envy's court does beyond the brawl's blow.
    fn envy_strikes(&mut self, i: usize, mv: MoveId) {
        let hp = self.hero.pos();
        let foe_damage = self.brawl.as_ref().map(|b| b.foe_damage).unwrap_or(1.0);
        let Some(d) = self.enemies[i].duel.as_deref() else { return };
        let (id, pos, mark, struck) = (d.id, d.pos, d.lore.mark, d.struck);
        let damage = mv.data().damage * d.breed.damage * foe_damage;
        // Where fire bursts or thunder falls besides the brawl's own blow: one blow at most.
        let spots: Vec<(V, f64)> = match mv {
            MoveId::BurningCoil => fire_spots(BELL, mark).into_iter().map(|p| (p, FIRE_RADIUS)).collect(),
            MoveId::CovetThunder => thunder_marks(mark, pos)[1..].iter().map(|p| (*p, 1.0)).collect(),
            _ => Vec::new(),
        };
        let hit = spots.iter().find(|(p, r)| math::len(math::sub(hp, *p)) <= r + HERO_RADIUS).map(|s| s.0);
        if let (Some(at), false) = (hit, struck) {
            if let Some(d) = self.enemies[i].duel.as_deref_mut() {
                d.struck = true;
            }
            self.general_hazard_blow(at, damage, id);
        }
        let flash = |g: &mut Game, f: Flash| {
            if let Some(b) = g.brawl.as_mut() {
                b.flashes.push(f);
            }
        };
        match mv {
            MoveId::BurningCoil => {
                for (p, _) in &spots {
                    self.view.sparkle(&self.level.grid, p.x, p.y, 1.0, 0xff8a3a, 12);
                }
                self.audio.play(Sfx::UrielWave);
                self.view.shake(0.18);
            }
            MoveId::CovetThunder => {
                for p in thunder_marks(mark, pos) {
                    flash(self, Flash::new(FlashKind::Lightning, p));
                }
                self.audio.play(Sfx::GabrielThunder);
                self.view.shake(0.3);
            }
            MoveId::Dive => {
                flash(self, Flash::new(FlashKind::Splash, mark));
                flash(self, Flash::new(FlashKind::Erupt, mark));
                self.audio.play(Sfx::BossSlam);
                self.feel_quake(mark);
                self.view.dust(&self.level.grid, mark.x, mark.y, 0x5ac0a0, 26, 3.0);
            }
            MoveId::BellToll => {
                let mut f = Flash::new(FlashKind::Quake, BELL);
                if let Shape::Mark { radius } = mv.data().shape {
                    f.length = radius;
                }
                flash(self, f);
                self.audio.play(Sfx::Toll);
                self.feel_quake(BELL);
                self.view.shake(0.5);
            }
            MoveId::CovetWave => {
                let mut f = Flash::new(FlashKind::Quake, pos);
                f.length = 3.0;
                flash(self, f);
                self.audio.play(Sfx::UrielWave);
                self.feel_quake(pos);
            }
            MoveId::CovetBeam => self.audio.play(Sfx::RaphaelBeam),
            MoveId::CovetFire => self.audio.play(Sfx::MichaelFire),
            _ => {}
        }
    }

    /// As his health falls into `phase`: as the second begins, two serpentlings rise from the water
    /// (unless they already have, or his general's death has lost them); as the third begins, he
    /// coils round the bell.
    pub(super) fn envy_phase(&mut self, prince: u32, at: V, phase: u8) {
        if phase >= 2 && self.adds_unsummoned(prince) {
            self.raise_serpentlings(prince, at);
        }
        if phase >= 3 && self.in_court() {
            if let Some(d) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == prince) {
                d.lore.bell = Some(BELL);
            }
            self.floater(at.x, at.y - 2.6, "HE COILS ROUND THE BELL", 1.8, Some(0xff8a3a));
            self.view.dust(&self.level.grid, at.x, at.y, 0x5ac0a0, 24, 2.4);
            self.audio.play(Sfx::Toll);
        }
    }

    /// After a fall (his serpentlings are already gone): nothing of his last move is remembered.
    pub(super) fn envy_reset(&mut self) {
        if let Some(c) = self.court.as_mut() {
            c.envy.last = None;
        }
    }

    /// As he falls: he sinks, and the bell is still.
    pub(super) fn envy_slain(&mut self, at: V) {
        self.floater(at.x, at.y - 1.8, "THE BELL FALLS SILENT", 2.0, Some(ENVY_GREEN));
        self.view.dust(&self.level.grid, at.x, at.y, 0x5ac0a0, 30, 3.0);
        self.audio.play(Sfx::Toll);
    }

    /// The phase banner's line under his name, and its colour.
    pub(super) fn envy_banner(phase: u8) -> (&'static str, u32) {
        match phase {
            2 => ("HE COVETS YOUR ARCHANGELS' POWERS   READ HIS GREEN TELL, THEN STRIKE", ENVY_GREEN),
            _ => ("HE COILS ROUND THE HOT BELL   STRIKE HIM FROM THE BELL'S SIDE", 0xff8a3a),
        }
    }

    /// His adds at once (his general's decree done): the serpentlings.
    pub(super) fn envy_raise(&mut self, prince: u32, at: V) {
        self.raise_serpentlings(prince, at);
    }

    /// Two serpentlings rise from the springs in the water nearest the hero (in the ring, out of
    /// the floor either side of the prince); they fall when he does.
    fn raise_serpentlings(&mut self, id: u32, at: V) {
        let hp = self.hero.pos();
        let spots: Vec<V> = if self.in_court() {
            springs_near(hp).into_iter().take(SERPENTLINGS as usize).map(|p| p.v()).collect()
        } else {
            let across = math::norm(V::new(-(hp.y - at.y), hp.x - at.x));
            [-1.0, 1.0].iter().map(|s| math::add(at, math::scale(across, 2.2 * s))).collect()
        };
        let tier = self.realm_tier();
        let (weight_hp, weight_damage) = if self.in_court() { adds_weight(crate::logic::sins::Sin::Envy, tier) } else { (1.0, 1.0) };
        for p in spots {
            let facing = math::norm(math::sub(hp, p));
            let lore = Lore { master: Some(id), rise: SERPENTLING_RISE, ..Lore::default() };
            if let Some(sid) = self.spawn_duelist(EnemyKind::Serpentling, p, facing, lore) {
                if let Some(s) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == sid) {
                    s.max_hp *= weight_hp;
                    s.hp = s.max_hp;
                    s.breed.damage *= weight_damage;
                }
            }
            if let Some(b) = self.brawl.as_mut() {
                b.flashes.push(Flash::new(FlashKind::Splash, p));
            }
            self.view.dust(&self.level.grid, p.x, p.y, 0x5ac0a0, 20, 1.8);
            self.view.sparkle(&self.level.grid, p.x, p.y, 1.0, ENVY_GREEN, 24);
        }
        self.floater(hp.x, hp.y - 1.5, "SERPENTLINGS RISE FROM THE WATER", 1.6, Some(ENVY_GREEN));
        self.audio.play(Sfx::Drip);
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{self, run};
    use super::super::{COURT_CADENCE, COURT_CAPACITY};
    use super::*;
    use crate::game::{save, Screen};
    use crate::input::PadState;
    use crate::logic::angels::Blessing;
    use crate::logic::combat::archetype::Archetype;
    use crate::logic::combat::prince::envy::{burnt_side, COIL, FIRES};
    use crate::logic::combat::prince::phase_of;
    use crate::logic::combat::{Duelist, Phase};
    use crate::logic::sins::Sin;
    use crate::logic::types::{Tile, P};

    fn court() -> Game {
        let mut g = testing::court(Sin::Envy);
        // (The stick is the world's way round, as the bots play.)
        g.view.camera_mode = crate::render::view::CameraMode::Top;
        g
    }

    fn leviathan(g: &mut Game) -> &mut Duelist {
        testing::prince(g)
    }

    /// Leviathan `dist` tiles north of the hero, facing him, resting, with his token: about to begin.
    fn poised(g: &mut Game, dist: f64) -> V {
        let hp = g.hero.pos();
        let d = leviathan(g);
        d.pos = V::new(hp.x, hp.y - dist);
        d.facing = V::new(0.0, 1.0);
        d.rest = u16::MAX;
        d.token = true;
        hp
    }

    /// The hero walks `dir` (world axes) for `secs`.
    fn walk(g: &mut Game, dir: V, secs: f64) {
        g.input.set_pad(Some(PadState { left: (dir.x as f32, dir.y as f32), ..Default::default() }));
        run(g, secs);
        g.input.set_pad(Some(PadState::default()));
    }

    /// `n` combat frames of play.
    fn frames(g: &mut Game, n: u32) {
        run(g, n as f64 / 60.0);
    }

    /// The hero has called on `angel`'s power: a blessing's charge, spent.
    fn called(g: &mut Game, angel: Archangel) {
        g.blessing = Some(Blessing { angel, charges: 2 });
        run(g, 0.05);
        g.blessing = Some(Blessing { angel, charges: 1 });
        run(g, 0.05);
    }

    #[test]
    fn the_court_of_envy_is_the_drowned_bell_and_leviathan_fights_there_by_frame_data() {
        let g = court();
        assert_eq!(g.level.data.name, "The Drowned Bell");
        assert_eq!(g.realm, Some(Sin::Envy));
        assert!(g.boss.is_none(), "no Cyclops");
        assert!(g.arena() && g.brawling(), "a boss's arena, fought by frame data");
        let b = g.brawl.as_ref().unwrap();
        assert_eq!((b.threat.capacity, b.threat.cadence), (COURT_CAPACITY, COURT_CADENCE));
        let d = g.prince().expect("Leviathan rises");
        assert_eq!((d.breed.kind, d.arch, d.breed.phase), (EnemyKind::Leviathan, Archetype::Prince, 1));
        assert_eq!(d.stats().cost, 3);
        assert!(d.lore.bell.is_none(), "not yet coiled round the bell");
        assert_eq!(g.enemies.len(), 1, "he waits alone");
        let bar = g.prince_bar().unwrap();
        assert_eq!((bar.name.as_str(), bar.title), ("LEVIATHAN", "THE CROOKED SERPENT"));
        assert_eq!(g.keys_total, 1, "one key opens the gate");
    }

    #[test]
    fn his_phases_change_at_the_thresholds_the_serpentlings_rise_once_and_the_bell_draws_him() {
        let mut g = court();
        let max = leviathan(&mut g).max_hp;
        leviathan(&mut g).rest = u16::MAX;
        let from = leviathan(&mut g).pos;
        let serpentlings = |g: &Game| g.enemies.iter().filter(|e| e.kind() == EnemyKind::Serpentling).count();
        leviathan(&mut g).hp = max * 0.75;
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        let hp = leviathan(&mut g).hp;
        assert_eq!(leviathan(&mut g).breed.phase, phase_of(hp, max));
        assert_eq!(serpentlings(&g), 0);
        // Under two thirds: the Evil Eye, two serpentlings out of the water, a green cross.
        leviathan(&mut g).hp = max * 0.6;
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        assert_eq!(leviathan(&mut g).breed.phase, 2);
        assert_eq!(serpentlings(&g), SERPENTLINGS as usize, "two serpentlings rise");
        let rose: Vec<V> =
            g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.breed.kind == EnemyKind::Serpentling).map(|d| d.pos).collect();
        assert!(rose.iter().all(|p| crate::logic::court::envy::in_water(p.x, p.y)), "from the water: {rose:?}");
        assert!(g.level.grid.count(Tile::Charge) >= 1, "a breather");
        assert!(g.juice.banner.as_ref().is_some_and(|b| b.text.contains("THE EVIL EYE") && b.sub.contains("COVETS")));
        assert_eq!(leviathan(&mut g).breed.moves()[0], MoveId::Covet);
        // Under a third: the Bell. He coils round it, swimming to it; no more serpentlings.
        leviathan(&mut g).hp = max * 0.3;
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        assert_eq!(leviathan(&mut g).breed.phase, 3);
        assert_eq!(serpentlings(&g), SERPENTLINGS as usize, "raised once");
        assert_eq!(leviathan(&mut g).lore.bell, Some(BELL));
        assert!(g.juice.banner.as_ref().is_some_and(|b| b.sub.contains("BELL'S SIDE")));
        assert_eq!(leviathan(&mut g).breed.moves()[0], MoveId::BellToll);
        run(&mut g, 3.0);
        let p = leviathan(&mut g).pos;
        assert!(math::len(math::sub(p, BELL)) <= COIL + 0.1, "coiled round the bell: {p:?}");
    }

    #[test]
    fn his_scales_turn_a_blow_but_not_in_his_recovery_nor_on_the_side_burnt_by_the_bell() {
        let mut g = court();
        let from = V::new(16.0, 10.0);
        {
            let d = leviathan(&mut g);
            d.pos = V::new(16.0, 12.5);
            d.rest = u16::MAX;
        }
        let blow = |g: &mut Game, from: V| {
            let before = leviathan(g).hp;
            g.call_duel_hit(0, from);
            let d = leviathan(g);
            let taken = before - d.hp;
            d.hp = before;
            taken
        };
        let warded = blow(&mut g, from);
        leviathan(&mut g).f.state = State::Attack { mv: MoveId::CoilLash, frame: 60 };
        let open = blow(&mut g, from);
        assert!(warded > 0.0 && open > warded * 1.5, "warded {warded}, open {open}");
        // Coiled round the hot bell: from the bell's side the whole blow lands, from the other his scales.
        leviathan(&mut g).f.state = State::Neutral;
        leviathan(&mut g).lore.bell = Some(BELL);
        let pos = leviathan(&mut g).pos;
        let bell_side = V::new(17.2, 13.6);
        assert!(burnt_side(pos, bell_side, BELL) && !burnt_side(pos, from, BELL));
        let burnt = blow(&mut g, bell_side);
        let scaled = blow(&mut g, from);
        assert!(burnt > scaled * 1.5, "burnt {burnt}, scaled {scaled}");
    }

    #[test]
    fn he_covets_the_archangel_last_called_on_raphael_if_none_and_the_banner_says_so() {
        // None called yet: Raphael's beam.
        let mut g = court();
        let hp = poised(&mut g, 5.0);
        leviathan(&mut g).breed.phase = 2;
        assert!(leviathan(&mut g).begin(MoveId::Covet, hp));
        run(&mut g, 0.02);
        assert_eq!(leviathan(&mut g).f.attack().map(|a| a.0), Some(MoveId::CovetBeam));
        let text = g.juice.banner.as_ref().map(|b| b.text.clone()).unwrap_or_default();
        assert_eq!(text, "HE COVETS YOUR RAPHAEL");
        // A blessing's charge spent on Gabriel: the thunder.
        let mut g = court();
        called(&mut g, Archangel::Gabriel);
        assert_eq!(g.court.as_ref().unwrap().envy.angel, Some(Archangel::Gabriel));
        let hp = poised(&mut g, 5.0);
        leviathan(&mut g).breed.phase = 2;
        assert!(leviathan(&mut g).begin(MoveId::Covet, hp));
        run(&mut g, 0.02);
        assert_eq!(leviathan(&mut g).f.attack().map(|a| a.0), Some(MoveId::CovetThunder));
        assert!(g.juice.banner.as_ref().is_some_and(|b| b.text == "HE COVETS YOUR GABRIEL"));
        // His patron's power called (its wait begun again): Uriel's wave.
        let mut g = court();
        let kind = g.hero_kind;
        g.progress.hero_mut(kind).patron = Some(Archangel::Uriel);
        run(&mut g, 0.05);
        g.patron_wait = 45.0;
        run(&mut g, 0.05);
        assert_eq!(g.court.as_ref().unwrap().envy.angel, Some(Archangel::Uriel));
        // Michael's fire, the last blessing's charge spent: a volley of four, its landing planned
        // again for the bolts' flight.
        let mut g = court();
        g.blessing = Some(Blessing { angel: Archangel::Michael, charges: 1 });
        run(&mut g, 0.05);
        g.blessing = None;
        run(&mut g, 0.05);
        assert_eq!(g.court.as_ref().unwrap().envy.angel, Some(Archangel::Michael));
        let hp = poised(&mut g, 6.0);
        leviathan(&mut g).breed.phase = 2;
        let id = leviathan(&mut g).id;
        let lands = leviathan(&mut g).lands_in(MoveId::Covet, hp);
        let fire = leviathan(&mut g).lands_in(MoveId::CovetFire, hp);
        assert!(fire > lands, "the fire's bolts take their flight");
        assert!(leviathan(&mut g).begin(MoveId::Covet, hp));
        let b = g.brawl.as_mut().unwrap();
        let now = b.threat.frame;
        b.threat.grant(&[crate::logic::combat::threat::Request {
            id,
            cost: 3,
            eta: lands as u32,
            dist: 6.0,
            front: 1.0,
            wait: 0,
            bias: 0.0,
        }]);
        b.threat.commit(id, lands);
        run(&mut g, 0.02);
        assert_eq!(leviathan(&mut g).f.attack().map(|a| a.0), Some(MoveId::CovetFire));
        let planned = g.brawl.as_ref().unwrap().threat.landing(id).unwrap();
        assert!(planned >= now + fire as u32 - 1, "planned again: {planned} for {now} + {fire}");
        let mut bolts = 0;
        for _ in 0..120 {
            g.update(crate::app::STEP);
            bolts = bolts.max(g.brawl.as_ref().unwrap().bolts.len());
        }
        assert_eq!(bolts, 4, "a volley of four");
    }

    #[test]
    fn his_dive_goes_under_comes_up_beneath_him_and_can_be_walked_out_of() {
        // Standing still, he is caught.
        let mut g = court();
        g.hero.hp = 1000;
        let hp = poised(&mut g, 5.0);
        assert!(leviathan(&mut g).begin(MoveId::Dive, hp));
        frames(&mut g, 20);
        assert!(leviathan(&mut g).untouchable(), "under the water, out of reach");
        let before = g.hero.hp;
        frames(&mut g, 50);
        assert!(g.hero.hp < before, "caught: {} of {before}", g.hero.hp);
        let d = leviathan(&mut g);
        assert!(!d.untouchable() && math::len(math::sub(d.pos, hp)) < 1.2, "he rose beneath him: {:?}", d.pos);
        assert!(matches!(d.f.attack(), Some((MoveId::Dive, Phase::Recovery, _))) && d.exposed(), "and lies open");
        // Walking off as soon as it is seen, he is clear of it.
        let mut g = court();
        g.hero.hp = 1000;
        let hp = poised(&mut g, 5.0);
        assert!(leviathan(&mut g).begin(MoveId::Dive, hp));
        frames(&mut g, 15);
        let before = g.hero.hp;
        walk(&mut g, V::new(1.0, 0.0), 1.2);
        assert_eq!(g.hero.hp, before, "stepped out of it in time");
    }

    /// Leviathan coiled by the bell in his third phase, beginning `mv` on the hero at `at`.
    fn at_the_bell(at: P, mv: MoveId) -> Game {
        let mut g = court();
        g.hero.hp = 1000;
        g.hero.place(at);
        let hp = g.hero.pos();
        let d = leviathan(&mut g);
        d.breed.phase = 3;
        d.lore.bell = Some(BELL);
        d.pos = V::new(18.6, 13.4);
        d.rest = u16::MAX;
        d.token = true;
        assert!(d.begin(mv, hp));
        g
    }

    #[test]
    fn the_bell_tolls_round_itself_and_its_fires_burst_round_it_and_both_can_be_left_in_time() {
        for stay in [true, false] {
            let mut g = at_the_bell(P::new(16, 19), MoveId::BellToll);
            frames(&mut g, 2);
            assert_eq!(leviathan(&mut g).lore.mark, BELL, "the toll's ring is the bell's");
            frames(&mut g, 13);
            let before = g.hero.hp;
            if stay {
                frames(&mut g, 70);
                assert!(g.hero.hp < before, "the floor shakes under him");
            } else {
                walk(&mut g, V::new(0.0, 1.0), 1.3);
                assert_eq!(g.hero.hp, before, "out of its ring in time");
            }
        }
        for stay in [true, false] {
            let mut g = at_the_bell(P::new(16, 23), MoveId::BurningCoil);
            let hp = g.hero.pos();
            frames(&mut g, 2);
            let fires = fire_spots(BELL, leviathan(&mut g).lore.mark);
            assert_eq!(fires.len(), FIRES + 1, "a ring round the bell and one on him");
            assert!(fires.iter().any(|p| math::len(math::sub(*p, hp)) < 0.1));
            frames(&mut g, 13);
            let before = g.hero.hp;
            if stay {
                frames(&mut g, 45);
                assert!(g.hero.hp < before, "the fire bursts under him");
            } else {
                walk(&mut g, V::new(1.0, 0.0), 0.9);
                assert_eq!(g.hero.hp, before, "off the fire in time");
            }
        }
    }

    #[test]
    fn the_coveted_thunder_falls_on_three_marks() {
        let mut g = court();
        g.hero.hp = 1000;
        called(&mut g, Archangel::Gabriel);
        let hp = poised(&mut g, 4.0);
        leviathan(&mut g).breed.phase = 2;
        assert!(leviathan(&mut g).begin(MoveId::Covet, hp));
        // Past the tracking, he is set down on one of the marks behind him.
        frames(&mut g, 32);
        let (mark, from) = (leviathan(&mut g).lore.mark, leviathan(&mut g).pos);
        let marks = thunder_marks(mark, from);
        g.hero.place(P::new(marks[1].x.round() as i32, marks[1].y.round() as i32));
        let before = g.hero.hp;
        frames(&mut g, 20);
        assert!(g.hero.hp < before, "the second mark's thunder: {} of {before}", g.hero.hp);
    }

    #[test]
    fn leviathan_slain_drops_the_key_of_envy_and_seals_envy() {
        let mut g = court();
        let from = leviathan(&mut g).pos;
        leviathan(&mut g).hp = 1.0;
        leviathan(&mut g).f.state = State::Attack { mv: MoveId::CoilLash, frame: 60 };
        leviathan(&mut g).breed.phase = 3;
        g.call_duel_hit(0, from);
        assert!(g.prince().is_none(), "he has fallen");
        assert!(g.court.as_ref().unwrap().slain);
        assert!(g.red_key.is_some(), "the Key of Envy lies on the floor");
        assert!(g.sins_cleared & Sin::Envy.bit() != 0, "Envy's bit is set");
        assert_eq!(g.key_name(), "THE KEY OF ENVY");
        assert!(g.saved_run.as_ref().is_some_and(|r| r.stage == save::Stage::SinShop(Sin::Envy)), "and the checkpoint is the old man's");
        let k = g.red_key.unwrap();
        g.hero.place(k);
        run(&mut g, 0.3);
        assert!(g.red_key.is_none() && g.gate_open);
        let gate = g.gate.unwrap();
        g.hero.place(P::new(gate.x, gate.y - 1));
        for _ in 0..1200 {
            g.update(crate::app::STEP);
            if g.screen == Screen::Won {
                break;
            }
            let gp = P::new(gate.x, gate.y);
            if g.door >= 1.0 && g.hero.occupied() != gp {
                g.hero.place(gp);
            }
        }
        assert_eq!(g.screen, Screen::Won);
    }

    #[test]
    fn a_fall_brings_leviathan_back_with_his_wounds_and_continue_comes_back_to_his_court() {
        let mut g = court();
        let max = leviathan(&mut g).max_hp;
        leviathan(&mut g).hp = max * 0.2;
        leviathan(&mut g).breed.phase = 3;
        leviathan(&mut g).lore.bell = Some(BELL);
        g.reset_round();
        let home = P::new(crate::logic::court::envy::PRINCE.0, crate::logic::court::envy::PRINCE.1).v();
        let d = leviathan(&mut g);
        assert_eq!((d.hp, d.breed.phase, d.lore.bell), (max * 0.2, 3, None), "his wounds kept (WARBAND.md W10)");
        assert_eq!(d.pos, home, "before his altar");
        // As the fight begins again he takes up the Bell: he coils round it once more.
        g.enter(Screen::Playing);
        run(&mut g, 0.05);
        assert_eq!(leviathan(&mut g).lore.bell, Some(BELL));
        g.checkpoint(save::Stage::Court(Sin::Envy));
        let saved = g.saved_run.clone().unwrap();
        let mut h = Game::new();
        h.saved_run = Some(saved);
        h.continue_game();
        assert!(h.in_court() && h.prince().is_some_and(|d| d.hp == d.max_hp), "a game over gives him back his health");
        assert_eq!(h.level.data.name, "The Drowned Bell");
    }

    #[test]
    fn his_general_slain_no_serpentlings_and_his_decree_done_they_rise_at_once() {
        use crate::logic::war::general::{Record, Records};
        let set = |g: &mut Game, r: Record| {
            let mut all = Records::decode(&g.warband.save.generals);
            all.set(Sin::Envy, r);
            g.warband.save.generals = all.encode();
            g.map_jump(save::Stage::Court(Sin::Envy));
            g.enter(Screen::Playing);
            g.hero.free_move = true;
        };
        let serpentlings = |g: &Game| g.enemies.iter().filter(|e| e.kind() == EnemyKind::Serpentling).count();
        // The Bell-Warden slain: none at his second phase.
        let mut g = court();
        set(&mut g, Record { met: true, slain: true, ..Default::default() });
        assert!(leviathan(&mut g).lore.summoned, "his phase-two adds are lost");
        let (max, from) = (leviathan(&mut g).max_hp, leviathan(&mut g).pos);
        leviathan(&mut g).hp = max * 0.6;
        g.call_duel_hit(0, from);
        run(&mut g, 0.1);
        assert_eq!(leviathan(&mut g).breed.phase, 2);
        assert_eq!(serpentlings(&g), 0);
        // The bell lowered by decree: they rise as the fight begins, and not again.
        let mut g = court();
        set(&mut g, Record { met: true, decree_done: true, ..Default::default() });
        run(&mut g, 0.1);
        assert_eq!(serpentlings(&g), SERPENTLINGS as usize, "at once");
        let (max, from) = (leviathan(&mut g).max_hp, leviathan(&mut g).pos);
        leviathan(&mut g).hp = max * 0.6;
        g.call_duel_hit(0, from);
        run(&mut g, 0.1);
        assert_eq!(serpentlings(&g), SERPENTLINGS as usize, "once");
    }

    #[test]
    fn the_expert_beats_leviathan_in_his_court_and_a_masher_does_not() {
        use crate::game::bot::{court_fight, Loadout, Skill};
        use crate::logic::heroes::HeroKind;
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        for (tier, seed) in [(1, 1), (2, 2)] {
            let kit = (tier > 1).then(|| Loadout::all(HeroKind::Ragnar, tier));
            let f = court_fight(Sin::Envy, Skill::Expert, seed, kit, 200.0);
            assert!(f.slain && f.deaths == 0 && f.phase == 3, "tier {tier}: {f:?}");
            assert_eq!(f.stats.overlaps, 0, "{f:?}");
        }
        let f = court_fight(Sin::Envy, Skill::Novice, 1, Some(Loadout::all(HeroKind::Ragnar, 2)), 200.0);
        assert!(!f.slain && f.game_over, "{f:?}");
    }
}
