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
use crate::logic::court::envy::{springs_near, BELL};
use crate::logic::court::weight;
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
        let (weight_hp, weight_damage) = if self.in_court() { weight(tier) } else { (1.0, 1.0) };
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
