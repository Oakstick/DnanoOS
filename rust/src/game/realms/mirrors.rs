//! The Hall of Mirrors in play (Pride's realm, REALMS.md §1; laid out in `logic::realms::mirrors`).
//!
//! He climbs the Spire of the Morning Star through its mirror-doors:
//! - walking into a door carries him at once, with no fade, out of a mirror of another gallery. A
//!   true door climbs one gallery; a vain one loops him one or two down, where the packs he slew
//!   stand again and the applause is louder. The light-well counts the galleries above him, and
//!   every vain loop adds one;
//! - the true mirrors show him as he is, cracked and as hurt as he is; the vain show him crowned,
//!   gilded and whole. A beam's Splinter swaps their faces for a while;
//! - Glory: a gallery cleared without a hit adds a stack (his blows bite 8% harder for each). A hit
//!   or a bottle humbles him, and every stack is lost. At three the beams track him and the tengu
//!   press harder; at five the vain mirrors fake the cracks. The statue of him before a mirror
//!   shatters under three blows, and costs a stack;
//! - beams: relay mirrors by the well throw the Morning Star's light across the floor, telegraphed,
//!   burning while he stands in them (a leap clears them).
//!
//! The throne is the lie: "You stand above them all. Take your throne." Stepping onto the dais
//! raises the Peerless, his reflection, a puppet on his own kit. It repeats every move he makes
//! 24 frames later, mirrored across the throne; each blow it lands drops a ring of the floor into
//! the well, and with three gone he falls to the first gallery. Its echo parried, it reels and
//! drops the moves it missed; a bottle is not mirrored (it poses, open). When it shatters the
//! floor gives way, and he falls the spire's length to the court's portal at its foot.

use super::super::{Banner, Game};
use super::Meter;
use crate::audio::Sfx;
use crate::game::brawl::VAULT_CAPACITY;
use crate::logic::combat::fsm::{Reel, State};
use crate::logic::combat::{math, Breed, Duelist, Fighter, Phase};
use crate::logic::enemy::{spec_of, Enemy, EnemyKind, EnemyMode};
use crate::logic::realms::mirrors::*;
use crate::logic::realms::{weight, Run};
use crate::logic::types::{Dir, Tile, P, V};

/// The Peerless's health before the tier's weight and his Glory swell it.
pub const PEERLESS_HP: f64 = 220.0;
/// What its armor holds against: none of his blows make it flinch while it stands (only a parry
/// opens it).
const PEERLESS_ARMOR: f64 = 1.0e4;
/// How fast he falls down the spire (levels a second, squared).
const FALL_PULL: f64 = 9.0;
/// Pride's violet, the gold of the vain, the white of the Morning Star.
const VIOLET: u32 = 0x9a5aff;
const GILT: u32 = 0xffd860;
const STARLIGHT: u32 = 0xeee6ff;

impl Game {
    /// The Hall of Mirrors, while he is in it.
    pub(in crate::game) fn pride(&self) -> Option<&Mirrors> {
        match self.run.as_ref() {
            Some(Run::Pride(m)) => Some(m),
            _ => None,
        }
    }

    /// Is `p` a tile he is sent on from the moment he sets foot on it (a mirror-door's recess: he
    /// is through the glass at once)? The camera check never stands him there (QA.md Q9).
    pub fn passes_through(&self, p: P) -> bool {
        self.pride().is_some_and(|m| m.door_at(p).is_some())
    }

    fn pride_mut(&mut self) -> Option<&mut Mirrors> {
        match self.run.as_mut() {
            Some(Run::Pride(m)) => Some(m),
            _ => None,
        }
    }

    /// As the realm is entered (its packs raised, the camera above): the court's portal sealed.
    pub(in crate::game) fn enter_mirrors(&mut self) {
        self.gate_open = false;
    }

    /// The realm's own rules each tick.
    pub(in crate::game) fn update_mirrors(&mut self, dt: f64) {
        if self.pride().is_none() {
            return;
        }
        self.pride_record();
        if self.pride_fall(dt) {
            return;
        }
        self.pride_where();
        self.pride_doors();
        self.pride_statues();
        let drank = self.pride_glory(dt);
        self.pride_beams(dt);
        self.pride_throne(dt, drank);
    }

    /// The controls as the realm leaves them (`Game::steer_run`): nothing while he falls.
    pub(in crate::game) fn steer_mirrors(&mut self, wanted: Dir) -> Dir {
        let falling = self.pride().is_some_and(|m| m.fall.is_some());
        self.hero.pace = if falling { 0.0 } else { 1.0 };
        if falling {
            self.hero.wish = V::new(0.0, 0.0);
            return Dir::None;
        }
        wanted
    }

    /// After a fall, as he rises at the checkpoint (and as the realm is first entered): the
    /// Peerless gone and the throne's floor whole, no beam, no Splinter, no Glory (a death humbles
    /// him), and his visit begun in the gallery he rises in.
    pub(in crate::game) fn reset_mirrors(&mut self) {
        self.pride_end_fight();
        if let Some(Run::Pride(m)) = self.run.as_mut() {
            for &p in &m.throne_floor {
                self.level.grid.set(p.x, p.y, Tile::Open);
            }
            m.rings = 0;
            m.ring_age = 99.0;
            m.fall = None;
            m.beam = None;
            m.beam_clock = 0.0;
            m.splinter = 0.0;
            m.glory = 0;
            m.echo = Echo::default();
            m.spoken = false;
            m.warps += 1;
        }
        let g = self.pride().and_then(|m| m.gallery_at(m.base.checkpoint)).unwrap_or(0);
        if let Some(m) = self.pride_mut() {
            m.gallery = g;
        }
        self.pride_arrive(g);
        if let Some(b) = self.brawl.as_mut() {
            b.might = 1.0;
            b.threat.capacity = VAULT_CAPACITY;
        }
    }

    /// F2: straight to the throne, a few steps north of the dais.
    pub(in crate::game) fn cheat_mirrors(&mut self) {
        let Some(m) = self.pride_mut() else { return };
        let at = P::new(m.throne.x, m.throne.y - 4);
        m.base.checkpoint = m.landing;
        m.warps += 1;
        self.hero.place(at);
        self.hero.face(Dir::Down);
        self.view.aim_along(Dir::Down);
        self.pride_arrive(THRONE);
    }

    /// What the HUD shows of the realm's rules: his Glory, the galleries the well shows above him,
    /// a Splinter while one is lodged, and the throne's rings while the Peerless stands.
    pub(in crate::game) fn mirrors_meters(&self) -> Vec<Meter> {
        let Some(m) = self.pride() else { return Vec::new() };
        let mut out = vec![
            Meter::count("GLORY", m.glory, GILT).warning(m.glory >= GLORY_FAKES),
            Meter::count("GALLERIES ABOVE", m.shown_above(), VIOLET),
        ];
        if m.splinter > 0.0 {
            out.push(Meter::bar("SPLINTER", m.splinter as f32, SPLINTER_SECONDS as f32, STARLIGHT).warning(true));
        }
        if m.echo.fight {
            out.push(Meter::count("RINGS STANDING", (RINGS.len() as u8).saturating_sub(m.rings) as u32, 0xd8c8ff).warning(m.rings > 0));
        }
        out
    }

    // ------------------------------------------------------------------ where he is

    /// His fighter's frame, remembered for the echo: where he stood and faced, what he began.
    fn pride_record(&mut self) {
        let Some(b) = self.brawl.as_ref() else { return };
        let (frame, state, guard) = (b.threat.frame, b.hero.state, b.hero.guard_held);
        let (pos, aim, altitude) = (self.hero.pos(), self.hero.aim(), self.hero.altitude);
        let Some(m) = self.pride_mut() else { return };
        let e = &mut m.echo;
        if frame == e.frame && !e.snaps.is_empty() {
            return; // (no new frame this step)
        }
        let began = match (e.last, state) {
            (Some(State::Attack { mv: was, frame: f0 }), State::Attack { mv, frame: f }) if was == mv && f0 < f => None,
            (_, State::Attack { mv, .. }) => Some(mv),
            _ => None,
        };
        let evade = matches!(state, State::Evade { .. }) && !matches!(e.last, Some(State::Evade { .. }));
        if began.is_some() {
            e.swing += 1;
        }
        e.last = Some(state);
        e.frame = frame;
        e.record(Snap { frame, pos, aim, began, evade, guard, altitude });
    }

    /// The gallery he stands in, should he have come there by other means than a mirror.
    fn pride_where(&mut self) {
        let o = self.hero.occupied();
        let Some(m) = self.pride() else { return };
        if let Some(g) = m.gallery_at(o).filter(|&g| g != m.gallery) {
            self.pride_arrive(g);
        }
    }

    /// His visit to gallery `g` begins: unhurt so far, its clearing still to be counted (if there is
    /// anything left in it to clear). Leaving the throne ends the Peerless's fight.
    fn pride_arrive(&mut self, g: usize) {
        let hurt = self.brawl.as_ref().map(|b| b.stats.hurt).unwrap_or(0);
        let belt = self.belt.len();
        let living = self.pride_living(g);
        if g != THRONE && self.pride().is_some_and(|m| m.echo.fight) {
            self.pride_end_fight();
            self.pride_mend_throne();
        }
        let Some(m) = self.pride_mut() else { return };
        if m.gallery != g {
            m.beam = None;
            m.beam_clock = 0.0;
        }
        m.gallery = g;
        m.visit = Visit { clean: true, awarded: living == 0, hurt, belt };
        if g != THRONE {
            m.spoken = false;
        }
    }

    /// The foes of gallery `g` still standing.
    fn pride_living(&self, g: usize) -> usize {
        let Some(m) = self.pride() else { return 0 };
        let Some(r) = m.galleries.get(g).map(|g| g.floor) else { return 0 };
        self.enemies
            .iter()
            .filter_map(|e| e.duel.as_deref())
            .filter(|d| d.counts() && !d.puppet && r.contains(P::new(d.pos.x.round() as i32, d.pos.y.round() as i32)))
            .count()
    }

    // ------------------------------------------------------------------ the mirror-doors

    /// Into a mirror-door's recess: through it.
    fn pride_doors(&mut self) {
        if self.hero.airborne() {
            return;
        }
        let (p, o) = (self.hero.pos(), self.hero.occupied());
        let Some(m) = self.pride() else { return };
        let Some(d) = m.door_at(o) else { return };
        let col = (p.x - m.doors[d].center().x as f64).round() as i32;
        self.step_through_mirror(d, col);
    }

    /**
     * He walks into mirror-door `d` in column `col` of its recess, and steps out of the mirror it
     * leads to, at once and with no fade, facing back into the room as a reflection would. A
     * vain door loops him down: the spire grows by a gallery in the well's count, the galleries he
     * is sent back through stand full again, and the applause is louder.
     */
    pub(in crate::game) fn step_through_mirror(&mut self, d: usize, col: i32) {
        let Some(m) = self.pride() else { return };
        let Some(door) = m.doors.get(d).cloned() else { return };
        let (at, face) = match door.out {
            Some(o) => (m.doors[o].front(col), Dir::Down),
            None => (m.base.arrival, m.base.entry.facing),
        };
        let vain = !door.truth;
        if let Some(m) = self.pride_mut() {
            m.warps += 1;
            if vain {
                m.loops += 1;
            }
            m.base.checkpoint = at;
        }
        self.hero.place(at);
        self.hero.face(face);
        self.view.aim_along(face);
        if vain {
            for g in door.to..door.gallery.max(door.to + 1) {
                self.pride_repopulate(g);
            }
            let hp = self.hero.pos();
            self.audio.play(Sfx::Bells);
            self.floater(hp.x, hp.y - 1.0, "THEY APPLAUD YOU", 1.4, Some(GILT));
            self.view.sparkle(&self.level.grid, hp.x, hp.y, 1.2, GILT, 18);
        } else {
            self.audio.play(Sfx::Whoosh);
        }
        self.pride_arrive(door.to);
    }

    /// Gallery `g`'s pack, slain or not, stands at its posts again, asleep and whole.
    fn pride_repopulate(&mut self, g: usize) {
        let Some(m) = self.pride() else { return };
        let Some(pack) = m.base.packs.get(g).cloned() else { return };
        let tier = m.base.tier;
        let r = pack.den;
        let lo = V::new(r.x0 as f64 - 0.5, r.y0 as f64 - 0.5);
        let gone: Vec<u32> = self
            .enemies
            .iter()
            .filter_map(|e| e.duel.as_deref())
            .filter(|d| !d.puppet && d.den.is_some_and(|den| den.lo == lo))
            .map(|d| d.id)
            .collect();
        self.enemies.retain(|e| !e.duel.as_ref().is_some_and(|d| gone.contains(&d.id)));
        let Some(mut brawl) = self.brawl.take() else { return };
        for id in gone {
            brawl.threat.release(id);
        }
        let fresh = self.raise_packs(&[pack], tier, &mut brawl);
        self.brawl = Some(brawl);
        self.enemies.extend(fresh);
        self.refresh_foe_views();
    }

    // ------------------------------------------------------------------ Glory

    /// A blow of his swing on a statue of him before a mirror: three, and it shatters (and the
    /// view of the glass is clear), costing a stack of Glory.
    fn pride_statues(&mut self) {
        let Some(b) = self.brawl.as_ref() else { return };
        let Some((mv, Phase::Active, _)) = b.hero.attack() else { return };
        let (d, aim) = (b.hero.data(mv), b.aim);
        let hp = self.hero.pos();
        let Some(m) = self.pride() else { return };
        if m.statue_swing == m.echo.swing || m.gallery >= THRONE {
            return;
        }
        let struck = m.galleries[m.gallery].doors.iter().copied().find(|&i| {
            let door = &m.doors[i];
            door.statue_hp > 0 && door.statue.is_some_and(|s| math::in_cone(hp, aim, s.v(), 0.35, d.reach, d.half_angle))
        });
        let Some(i) = struck else { return };
        let Some(m) = self.pride_mut() else { return };
        m.statue_swing = m.echo.swing;
        m.doors[i].statue_hp -= 1;
        let (left, Some(s)) = (m.doors[i].statue_hp, m.doors[i].statue) else { return };
        let (sx, sy) = (s.x as f64, s.y as f64);
        if left > 0 {
            self.audio.play(Sfx::Clang);
            self.view.spark(&self.level.grid, sx, sy, 0xd8d4e8);
            self.floater(sx, sy - 1.2, "YOUR LIKENESS CRACKS", 0.9, Some(0xd8d4e8));
            return;
        }
        m.glory = m.glory.saturating_sub(1);
        self.level.grid.set(s.x, s.y, Tile::Open);
        self.audio.play(Sfx::ShieldBreak);
        self.view.shake(0.25);
        self.view.dust(&self.level.grid, sx, sy, 0xc8c4d4, 26, 2.4);
        self.view.sparkle(&self.level.grid, sx, sy, 1.0, STARLIGHT, 16);
        self.floater(sx, sy - 1.3, "YOUR LIKENESS SHATTERS: -1 GLORY", 1.6, Some(STARLIGHT));
    }

    /**
     * Glory: a gallery cleared without a hit adds a stack; a hit or a bottle takes them all. Its
     * light lends his blows their might and the tengu their boldness. Returns whether he drank.
     */
    fn pride_glory(&mut self, dt: f64) -> bool {
        let hurt = self.brawl.as_ref().map(|b| b.stats.hurt).unwrap_or(0);
        let belt = self.belt.len();
        let Some(g) = self.pride().map(|m| m.gallery) else { return false };
        let living = self.pride_living(g);
        let Some(m) = self.pride_mut() else { return false };
        m.splinter = (m.splinter - dt).max(0.0);
        let hit = hurt > m.visit.hurt;
        let drank = belt < m.visit.belt;
        m.visit.hurt = hurt;
        m.visit.belt = belt;
        if hit {
            m.visit.clean = false;
        }
        let humbled = (hit || drank) && m.glory > 0;
        if hit || drank {
            m.glory = 0;
        }
        let rose = !m.visit.awarded && living == 0 && m.visit.clean && g < THRONE;
        if living == 0 {
            m.visit.awarded = true;
        }
        if rose {
            m.glory += 1;
        }
        let glory = m.glory;
        let hp = self.hero.pos();
        if humbled {
            self.floater(hp.x, hp.y - 1.4, "HUMBLED: YOUR GLORY IS LOST", 1.4, Some(0xb8a8d8));
        }
        if rose {
            self.audio.play(Sfx::Milestone);
            self.view.sparkle(&self.level.grid, hp.x, hp.y, 1.2, GILT, 30);
            self.juice.banner = Some(Banner {
                text: format!("GLORY {glory}"),
                sub: "CLEARED UNTOUCHED   A STATUE OF YOU RISES".into(),
                color: GILT,
                life: 2.2,
                max: 2.2,
            });
        }
        if let Some(b) = self.brawl.as_mut() {
            b.might = 1.0 + GLORY_MIGHT * glory as f64;
            b.threat.capacity = VAULT_CAPACITY + u8::from(glory >= GLORY_TRACKS);
        }
        if glory >= GLORY_TRACKS {
            // Tengu ni naru: the proud monks press a proud hero harder.
            for d in self.enemies.iter_mut().filter(|e| e.kind() == EnemyKind::Tengu).filter_map(|e| e.duel.as_deref_mut()) {
                d.rest = d.rest.min(12);
            }
        }
        drank
    }

    // ------------------------------------------------------------------ the beams

    /// Where a beam thrown by relay `r` of gallery `g` runs: out from its stand the way the stand
    /// faces (or, tracking him, toward him, so long as that is outward), until a wall stops it.
    fn pride_beam_line(&self, g: usize, r: usize, track: Option<V>) -> Option<(V, V)> {
        let m = self.pride()?;
        let &(stand, out) = m.galleries.get(g)?.relays.get(r)?;
        let from = stand.v();
        let radial = out.vecf();
        let dir = match track {
            Some(p) => {
                let to = math::norm(math::sub(p, from));
                if math::dot(to, radial) > 0.25 {
                    to
                } else {
                    radial
                }
            }
            None => radial,
        };
        let start = math::add(from, math::scale(dir, 0.5));
        let mut end = start;
        let mut t = 0.0;
        while t < 16.0 {
            let q = math::add(start, math::scale(dir, t + 0.25));
            if self.level.grid.get(q.x.round() as i32, q.y.round() as i32) == Tile::Rock {
                break;
            }
            end = q;
            t += 0.25;
        }
        Some((start, end))
    }

    /// The Morning Star's beams in his gallery: one at a time, telegraphed, then burning. One that
    /// catches him on his feet burns him and lodges a Splinter.
    fn pride_beams(&mut self, dt: f64) {
        let hp = self.hero.pos();
        let Some(m) = self.pride() else { return };
        let g = m.gallery;
        if g >= THRONE || m.galleries[g].relays.is_empty() {
            if let Some(m) = self.pride_mut() {
                m.beam = None;
            }
            return;
        }
        let tracks = m.glory >= GLORY_TRACKS;
        let relays = m.galleries[g].relays.clone();
        let fresh = if m.beam.is_none() && m.beam_clock + dt >= BEAM_EVERY {
            let r = if tracks {
                (0..relays.len())
                    .min_by(|&a, &b| {
                        let da = math::len(math::sub(relays[a].0.v(), hp));
                        let db = math::len(math::sub(relays[b].0.v(), hp));
                        da.total_cmp(&db)
                    })
                    .unwrap_or(0)
            } else {
                m.beam_turn % relays.len()
            };
            self.pride_beam_line(g, r, tracks.then_some(hp)).map(|l| (r, l))
        } else {
            None
        };
        let retrack = match m.beam {
            Some(b) if tracks && b.age < BEAM_TRACK => self.pride_beam_line(g, b.relay, Some(hp)),
            _ => None,
        };
        let airborne = self.hero.airborne() || self.hero.invincible();
        let Some(m) = self.pride_mut() else { return };
        m.beam_clock += dt;
        if let Some((relay, (from, to))) = fresh {
            m.beam = Some(Beam { relay, from, to, age: 0.0, touched: false, bite: 0.0 });
            m.beam_clock = 0.0;
            m.beam_turn += 1;
        }
        let mut burn = 0.0;
        let mut lit = false;
        let mut first = false;
        if let Some(b) = m.beam.as_mut() {
            let was = b.burning();
            b.age += dt;
            if let Some((from, to)) = retrack {
                (b.from, b.to) = (from, to);
            }
            lit = b.burning() && !was;
            if b.burning() && b.covers(hp, 0.3) && !airborne {
                if !b.touched {
                    b.touched = true;
                    first = true;
                    b.bite = BEAM_BITE_EVERY;
                    burn = BEAM_DAMAGE;
                } else {
                    b.bite -= dt;
                    if b.bite <= 0.0 {
                        b.bite += BEAM_BITE_EVERY;
                        burn = BEAM_BITE;
                    }
                }
            }
            if b.age >= BEAM_TELL + BEAM_BURN {
                m.beam = None;
            }
        }
        if first {
            m.splinter = SPLINTER_SECONDS;
        }
        if lit {
            self.audio.play(Sfx::Zap);
        }
        if burn > 0.0 {
            self.wound_hero(burn, 0xe8d8ff);
            if first {
                self.audio.play(Sfx::Hurt);
                self.view.sparkle(&self.level.grid, hp.x, hp.y, 1.0, STARLIGHT, 20);
                self.floater(hp.x, hp.y - 1.4, "A SPLINTER OF THE MIRROR: THE FACES SWAP", 1.8, Some(STARLIGHT));
            }
        }
    }

    // ------------------------------------------------------------------ the throne

    /// The throne: the Morning Star's lie, the dais, and the Peerless's fight.
    fn pride_throne(&mut self, dt: f64, drank: bool) {
        let o = self.hero.occupied();
        let Some(m) = self.pride() else { return };
        if m.gallery != THRONE {
            return;
        }
        if !m.spoken {
            if let Some(m) = self.pride_mut() {
                m.spoken = true;
            }
            self.audio.play(Sfx::Bells);
            self.juice.banner = Some(Banner {
                text: "YOU STAND ABOVE THEM ALL. TAKE YOUR THRONE.".into(),
                sub: "THE MORNING STAR".into(),
                color: STARLIGHT,
                life: 4.0,
                max: 4.0,
            });
        }
        let Some(m) = self.pride() else { return };
        if !m.echo.fight {
            if !m.won && m.on_dais(o) && !self.hero.airborne() {
                self.raise_peerless();
            }
            return;
        }
        if let Some(m) = self.pride_mut() {
            m.ring_age += dt;
        }
        self.pride_play_echo();
        if drank {
            self.pride_pose();
        }
        // Every blow it lands drops a ring of the floor into the well.
        let hurt = self.brawl.as_ref().map(|b| b.stats.hurt).unwrap_or(0);
        if self.pride().is_some_and(|m| hurt > m.echo.hurt) {
            if let Some(m) = self.pride_mut() {
                m.echo.hurt = hurt;
            }
            self.pride_drop_ring();
            if self.pride().is_some_and(|m| m.fall.is_some()) {
                return;
            }
        }
        if self.level.grid.get(o.x, o.y) == Tile::Pit && !self.hero.airborne() {
            self.pride_cast_down();
            return;
        }
        if self.pride_peerless().is_none() {
            self.pride_victory();
        }
    }

    /// The Peerless, standing.
    fn pride_peerless(&self) -> Option<usize> {
        self.enemies.iter().position(|e| e.kind() == EnemyKind::Peerless && e.duel.as_ref().is_some_and(|d| d.alive()))
    }

    /**
     * He steps onto the dais, and his reflection rises out of the floor across the throne: the
     * Peerless, a puppet on his own kit (his blade's string, his shield's parry window), swollen
     * by the Glory he carried in (+10% health and damage a stack).
     */
    pub(in crate::game) fn raise_peerless(&mut self) {
        let kit = self.hero_kit();
        let hp = self.hero.pos();
        let Some(m) = self.pride() else { return };
        let (glory, tier, centre) = (m.glory, m.base.tier, m.throne.v());
        let mut at = m.across_throne(hp);
        if !self.level.grid.is_walkable(at.x.round() as i32, at.y.round() as i32, false) {
            at = V::new(centre.x, centre.y - 1.0);
        }
        let Some(mut brawl) = self.brawl.take() else { return };
        let id = brawl.next_id();
        let spec = spec_of(&self.level.grid, EnemyKind::Peerless);
        let mut e = Enemy::new(&spec, P::new(at.x.round() as i32, at.y.round() as i32), self.level.data.enemy_speed);
        e.b.mode = EnemyMode::Chase;
        let facing = math::norm(math::sub(hp, at));
        let mut d = Duelist::bred(id, Breed::of(EnemyKind::Peerless), at, facing);
        let swell = 1.0 + ECHO_GLORY * glory as f64;
        d.puppet = true;
        d.breed.reach = 0.0;
        d.breed.damage = swell / brawl.foe_damage.max(1e-6);
        d.max_hp = (PEERLESS_HP * weight(0, tier).0 * swell).round();
        d.hp = d.max_hp;
        d.f = Fighter::with_kit(&kit);
        d.f.armor = PEERLESS_ARMOR;
        d.f.mercy_frames = 0;
        d.rest = u16::MAX;
        e.duel = Some(Box::new(d));
        let (frame, hurt) = (brawl.threat.frame, brawl.stats.hurt);
        self.brawl = Some(brawl);
        self.enemies.push(e);
        self.refresh_foe_views();
        if let Some(m) = self.pride_mut() {
            m.echo.fight = true;
            m.echo.rise = frame + RISE_FRAMES as u32;
            m.echo.played = frame.saturating_sub(ECHO_DELAY);
            m.echo.hurt = hurt;
            m.rings = 0;
        }
        self.audio.play(Sfx::BossRoar);
        self.view.shake(0.4);
        self.view.sparkle(&self.level.grid, at.x, at.y, 1.4, 0xd8c8ff, 40);
        self.juice.banner = Some(Banner {
            text: "THE PEERLESS".into(),
            sub: if glory > 0 { format!("YOUR REFLECTION RISES, SWOLLEN BY {glory} GLORY") } else { "YOUR REFLECTION RISES".into() },
            color: 0xd8c8ff,
            life: 3.0,
            max: 3.0,
        });
    }

    /**
     * The echo: what he did `ECHO_DELAY` frames ago, done again by the Peerless, mirrored across
     * the throne (leaning a step out of the glass toward him). A move it cannot begin because it
     * reels is dropped, not saved for later.
     */
    fn pride_play_echo(&mut self) {
        let hp = self.hero.pos();
        let Some(i) = self.pride_peerless() else { return };
        let grid = &self.level.grid;
        let Some(Run::Pride(m)) = self.run.as_mut() else { return };
        let centre = m.throne.v();
        let upto = m.echo.frame.saturating_sub(ECHO_DELAY);
        let played = m.echo.played;
        let snaps: Vec<Snap> = m.echo.snaps.iter().filter(|s| s.frame > played && s.frame <= upto).copied().collect();
        m.echo.played = m.echo.played.max(upto);
        let rise = m.echo.rise;
        let Some(du) = self.enemies[i].duel.as_deref_mut() else { return };
        let mut struck = false;
        for s in snaps {
            let mirror = m.across_throne(s.pos);
            let toward = math::sub(s.pos, mirror);
            let lean = ECHO_CLOSE.min((math::len(toward) - ECHO_APART).max(0.0));
            let mut at = math::add(mirror, math::scale(math::norm(toward), lean));
            let off = math::sub(at, hp);
            if math::len(off) < ECHO_APART {
                let away = if math::len(off) > 1e-6 { math::norm(off) } else { math::norm(math::sub(centre, hp)) };
                let away = if math::len(away) > 1e-6 { away } else { V::new(0.0, -1.0) };
                at = math::add(hp, math::scale(away, ECHO_APART));
            }
            if grid.is_walkable(at.x.round() as i32, at.y.round() as i32, false) {
                du.pos = at;
            }
            let facing = math::norm(math::scale(s.aim, -1.0));
            if math::len(facing) > 0.5 {
                du.facing = facing;
            }
            if !matches!(du.f.attack(), Some((_, Phase::Active | Phase::Recovery, _))) {
                du.aim = du.facing;
            }
            if s.frame < rise {
                continue; // still rising out of the floor
            }
            let free = !matches!(du.f.state, State::Reel { .. } | State::Dead);
            du.f.guard_held = s.guard && free;
            if let (Some(mv), true) = (s.began, free) {
                du.f.state = State::Neutral;
                du.begin(mv, hp);
                struck = true;
            }
            if s.evade && free {
                du.f.state = State::Neutral;
                du.f.evade();
            }
        }
        // A ring fallen under it: it keeps to what floor is left.
        let tile = P::new(du.pos.x.round() as i32, du.pos.y.round() as i32);
        if !grid.is_walkable(tile.x, tile.y, false) {
            let r = RINGS.get(m.rings.saturating_sub(1) as usize).copied().unwrap_or(RINGS[0]) - 0.7;
            let out = math::sub(du.pos, centre);
            du.pos = math::add(centre, math::scale(math::norm(out), r.min(math::len(out))));
        }
        let p = du.pos;
        self.enemies[i].m.tile = P::new(p.x.round() as i32, p.y.round() as i32);
        if struck {
            self.audio.play(Sfx::Slash);
        }
    }

    /// He drank: grace cannot be mirrored, and the Peerless stands posing, open.
    fn pride_pose(&mut self) {
        let Some(i) = self.pride_peerless() else { return };
        let Some(du) = self.enemies[i].duel.as_deref_mut() else { return };
        du.f.state = State::Reel { kind: Reel::Deflected, left: POSE_FRAMES, total: POSE_FRAMES };
        du.f.guard_held = false;
        let p = du.pos;
        self.floater(p.x, p.y - 1.4, "GRACE CANNOT BE MIRRORED: IT POSES, OPEN", 1.6, Some(0xd8c8ff));
    }

    /// A blow of the Peerless landed: the throne's next ring falls into the well. With the third
    /// gone, so is he.
    fn pride_drop_ring(&mut self) {
        let Some(Run::Pride(m)) = self.run.as_mut() else { return };
        let k = m.rings;
        m.rings += 1;
        m.ring_age = 0.0;
        for &p in &m.throne_floor {
            if m.ring_of(p) == k {
                self.level.grid.set(p.x, p.y, Tile::Pit);
            }
        }
        let gone = m.rings as usize >= RINGS.len();
        let c = m.throne;
        self.audio.play(Sfx::BossSlam);
        self.view.shake(0.5);
        self.view.dust(&self.level.grid, c.x as f64, c.y as f64, 0x6a5a8a, 30, RINGS[0] as f32);
        if gone {
            self.pride_cast_down();
        } else {
            let left = RINGS.len() - k as usize - 1;
            self.floater(
                c.x as f64,
                c.y as f64 - 2.0,
                format!("THE FLOOR FALLS AWAY: {left} RING{} LEFT", if left == 1 { "" } else { "S" }),
                1.8,
                Some(0xd8c8ff),
            );
        }
    }

    /// Beaten: the Peerless takes his throne, and he falls to the first gallery to climb again.
    fn pride_cast_down(&mut self) {
        self.pride_end_fight();
        if let Some(m) = self.pride_mut() {
            m.fall = Some(Fall { t: 0.0, win: false });
        }
        self.audio.play(Sfx::Denied);
        self.juice.banner =
            Some(Banner { text: "CAST DOWN".into(), sub: "THE PEERLESS TAKES YOUR THRONE".into(), color: 0xb8a8d8, life: 2.6, max: 2.6 });
    }

    /// The Peerless shatters: the floor gives way under him, and the court's portal opens at the
    /// foot of the spire.
    fn pride_victory(&mut self) {
        if let Some(Run::Pride(m)) = self.run.as_mut() {
            m.echo.fight = false;
            m.won = true;
            m.fall = Some(Fall { t: 0.0, win: true });
            m.ring_age = 0.0;
            for &p in &m.throne_floor {
                self.level.grid.set(p.x, p.y, Tile::Pit);
            }
        }
        self.gate_open = true;
        self.audio.play(Sfx::ShieldBreak);
        self.audio.play(Sfx::BossDeath);
        self.view.shake(0.8);
        self.juice.banner = Some(Banner {
            text: "THE PEERLESS SHATTERS".into(),
            sub: "AND THE FLOOR GIVES WAY".into(),
            color: STARLIGHT,
            life: 2.0,
            max: 2.0,
        });
    }

    /// The Peerless gone from the scene, the fight over.
    fn pride_end_fight(&mut self) {
        if self.enemies.iter().any(|e| e.kind() == EnemyKind::Peerless) {
            self.enemies.retain(|e| e.kind() != EnemyKind::Peerless);
            self.refresh_foe_views();
        }
        if let Some(m) = self.pride_mut() {
            m.echo.fight = false;
        }
    }

    /// The throne's floor whole again.
    fn pride_mend_throne(&mut self) {
        if let Some(Run::Pride(m)) = self.run.as_mut() {
            for &p in &m.throne_floor {
                self.level.grid.set(p.x, p.y, Tile::Open);
            }
            m.rings = 0;
        }
    }

    /**
     * The fall down the spire's length (true while it lasts). The victor lands before the court's
     * portal at the foot of the spire: "How art thou fallen". The beaten lands in the first
     * gallery, by the portal he came in by, to climb again.
     */
    fn pride_fall(&mut self, dt: f64) -> bool {
        let Some(f) = self.pride().and_then(|m| m.fall) else { return false };
        let t = f.t + dt;
        if t < FALL_SECONDS {
            if let Some(m) = self.pride_mut() {
                m.fall = Some(Fall { t, ..f });
            }
            self.hero.altitude = -FALL_PULL * t * t;
            return true;
        }
        self.pride_mend_throne();
        let Some(m) = self.pride_mut() else { return false };
        m.fall = None;
        m.warps += 1;
        let (at, face) = if f.win { (m.base.exit.front(), m.base.exit.facing.opposite()) } else { (m.base.arrival, m.base.entry.facing) };
        m.base.checkpoint = at;
        self.hero.place(at);
        self.hero.altitude = self.level.grid.elevation(at.x, at.y) as f64;
        self.hero.face(face);
        self.view.aim_along(face);
        self.juice.fade = 1.0;
        self.audio.play(Sfx::Land);
        self.view.shake(0.5);
        self.view.dust(&self.level.grid, at.x as f64, at.y as f64, 0x8a7aa8, 24, 2.0);
        self.juice.banner = Some(if f.win {
            Banner {
                text: "HOW ART THOU FALLEN".into(),
                sub: "LUCIFER'S COURT LAY AT THE FOOT OF THE SPIRE ALL ALONG".into(),
                color: VIOLET,
                life: 4.5,
                max: 4.5,
            }
        } else {
            Banner { text: "BROUGHT DOWN TO THE SIDES OF THE PIT".into(), sub: "CLIMB AGAIN".into(), color: 0xb8a8d8, life: 3.0, max: 3.0 }
        });
        self.pride_arrive(0);
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::realms::tests::{in_realm, STEP};
    use crate::game::Screen;
    use crate::input::Action;
    use crate::logic::sins::Sin;

    fn mirrors(g: &Game) -> &Mirrors {
        g.pride().expect("the Hall of Mirrors")
    }

    /// The door of gallery `gallery` that is true (or the first vain one).
    fn door_of(g: &Game, gallery: usize, truth: bool) -> usize {
        let m = mirrors(g);
        m.galleries[gallery].doors.iter().copied().find(|&d| m.doors[d].truth == truth).unwrap()
    }

    /// Into door `d` (the west column of its recess).
    fn walk_into(g: &mut Game, d: usize) {
        let t = mirrors(g).doors[d].tiles[0];
        g.hero.place(t);
        g.update(STEP);
    }

    fn run(g: &mut Game, seconds: f64) {
        for _ in 0..(seconds / STEP).round() as usize {
            g.update(STEP);
        }
    }

    /// The fight's frame counter.
    fn frame(g: &Game) -> u32 {
        g.brawl.as_ref().unwrap().threat.frame
    }

    #[test]
    fn a_true_mirror_climbs_and_a_vain_one_loops_him_down_with_no_way_back() {
        let mut g = in_realm(Sin::Pride);
        assert!(!g.gate_open, "the court's portal is sealed");
        assert_eq!(mirrors(&g).gallery, 0);
        assert_eq!(mirrors(&g).shown_above(), 6);
        // Up through the true mirrors, gallery by gallery.
        for want in 1..=3 {
            let d = door_of(&g, want - 1, true);
            walk_into(&mut g, d);
            let m = mirrors(&g);
            assert_eq!(m.gallery, want);
            let out = m.doors[d].out.unwrap();
            assert!(m.doors[out].front(-1) == g.hero.occupied() || m.doors[out].front(1) == g.hero.occupied());
            assert_eq!(g.hero.facing, Dir::Down, "he steps out of the glass, facing back into the room");
            assert_eq!(m.base.checkpoint, g.hero.occupied(), "he would rise here");
        }
        assert_eq!(mirrors(&g).loops, 0);
        // The mirror he stepped out of never leads back down where he came from.
        let came = mirrors(&g).doors[door_of(&g, 2, true)].out.unwrap();
        assert_ne!(mirrors(&g).doors[came].to, 2);
        // A vain mirror: down one or two, the well counting one gallery more above him.
        let above = mirrors(&g).shown_above();
        let d = door_of(&g, 3, false);
        let to = mirrors(&g).doors[d].to;
        walk_into(&mut g, d);
        let m = mirrors(&g);
        assert!(to < 3 && m.gallery == to);
        assert_eq!(m.loops, 1);
        assert_eq!(m.shown_above(), above + (3 - to) as u32 + 1, "the spire grows as he is flattered");
        assert!(g.floaters.iter().any(|f| f.text == "THEY APPLAUD YOU"));
        // The first gallery's vain mirrors send him back to the portal he came in by.
        let mut g = in_realm(Sin::Pride);
        let d = door_of(&g, 0, false);
        walk_into(&mut g, d);
        assert_eq!((mirrors(&g).gallery, g.hero.occupied()), (0, mirrors(&g).base.arrival));
    }

    #[test]
    fn a_vain_loop_stands_the_slain_packs_up_again() {
        let mut g = in_realm(Sin::Pride);
        let d = door_of(&g, 0, true);
        walk_into(&mut g, d);
        let lower = mirrors(&g).galleries[0].floor;
        let count = |g: &Game| {
            g.enemies
                .iter()
                .filter_map(|e| e.duel.as_deref())
                .filter(|d| lower.contains(P::new(d.pos.x.round() as i32, d.pos.y.round() as i32)))
                .count()
        };
        let pack = count(&g);
        assert!(pack >= 2);
        g.enemies.retain(|e| !e.duel.as_ref().is_some_and(|d| lower.contains(P::new(d.pos.x.round() as i32, d.pos.y.round() as i32))));
        g.refresh_foe_views();
        assert_eq!(count(&g), 0);
        let d = door_of(&g, 1, false);
        walk_into(&mut g, d);
        assert_eq!(mirrors(&g).gallery, 0);
        assert_eq!(count(&g), pack, "replayed and repopulated");
        assert!(g.enemies.iter().filter_map(|e| e.duel.as_deref()).all(|d| d.den.is_some_and(|d| !d.awake) || d.puppet));
    }

    #[test]
    fn glory_rises_for_a_gallery_cleared_untouched_and_a_hit_or_a_bottle_takes_it_all() {
        let mut g = in_realm(Sin::Pride);
        let floor = mirrors(&g).galleries[0].floor;
        g.enemies.retain(|e| !e.duel.as_ref().is_some_and(|d| floor.contains(P::new(d.pos.x.round() as i32, d.pos.y.round() as i32))));
        g.refresh_foe_views();
        g.update(STEP);
        assert_eq!(mirrors(&g).glory, 1, "cleared without a hit");
        assert!((g.brawl.as_ref().unwrap().might - 1.08).abs() < 1e-9, "his blows bite 8% harder");
        g.update(STEP);
        assert_eq!(mirrors(&g).glory, 1, "counted once");
        g.wound_hero(5.0, 0xff0000);
        g.update(STEP);
        assert_eq!(mirrors(&g).glory, 0, "a hit humbles him");
        assert_eq!(g.brawl.as_ref().unwrap().might, 1.0);
        // A bottle humbles him too.
        if let Some(m) = g.pride_mut() {
            m.glory = 4;
        }
        g.belt = vec![crate::logic::types::PotionKind::Stamina];
        g.update(STEP);
        assert_eq!(mirrors(&g).glory, 4);
        g.input.inject(Action::Drink);
        g.update(STEP);
        assert_eq!(mirrors(&g).glory, 0, "and so does a bottle");
        // A gallery cleared after a hit adds nothing.
        let mut g = in_realm(Sin::Pride);
        g.wound_hero(5.0, 0xff0000);
        g.update(STEP);
        g.enemies.retain(|e| !e.duel.as_ref().is_some_and(|d| floor.contains(P::new(d.pos.x.round() as i32, d.pos.y.round() as i32))));
        g.refresh_foe_views();
        g.update(STEP);
        assert_eq!(mirrors(&g).glory, 0);
    }

    #[test]
    fn the_glass_tells_the_truth_until_a_splinter_or_too_much_glory_hides_it() {
        let mut g = in_realm(Sin::Pride);
        let (t, v) = (door_of(&g, 0, true), door_of(&g, 0, false));
        let m = mirrors(&g);
        assert!(m.shows_true(t) && !m.shows_true(v));
        assert!(m.cracked(t) && !m.cracked(v));
        if let Some(m) = g.pride_mut() {
            m.splinter = 3.0;
        }
        let m = mirrors(&g);
        assert!(!m.shows_true(t) && m.shows_true(v), "a Splinter swaps the faces");
        if let Some(m) = g.pride_mut() {
            m.splinter = 0.0;
            m.glory = GLORY_FAKES;
        }
        assert!(mirrors(&g).cracked(v), "at five stacks the vain fake the cracks");
        let meters = g.mirrors_meters();
        assert_eq!(meters[0].label, "GLORY");
        assert_eq!(meters[0].value, GLORY_FAKES as f32);
        assert_eq!(meters[1].label, "GALLERIES ABOVE");
    }

    #[test]
    fn a_statue_of_him_shatters_under_three_blows_and_costs_a_stack() {
        let mut g = in_realm(Sin::Pride);
        let d = door_of(&g, 0, true);
        let statue = mirrors(&g).doors[d].statue.unwrap();
        if let Some(m) = g.pride_mut() {
            m.glory = 2;
            m.visit.awarded = true;
        }
        g.hero.place(P::new(statue.x, statue.y + 1));
        g.hero.face(Dir::Up);
        for _ in 0..3 {
            g.input.inject(Action::Attack);
            run(&mut g, 0.6);
            g.hero.stamina = 100.0;
        }
        let m = mirrors(&g);
        assert_eq!(m.doors[d].statue_hp, 0);
        assert_eq!(m.glory, 1);
        assert!(g.level.grid.is_walkable(statue.x, statue.y, false), "its plinth is clear floor");
    }

    #[test]
    fn a_beam_burns_him_where_he_stands_lodges_a_splinter_and_a_leap_clears_it() {
        let mut g = in_realm(Sin::Pride);
        let m = mirrors(&g);
        let (stand, _) = m.galleries[0].relays[0];
        let spot = P::new(stand.x, stand.y + 2);
        let line = g.pride_beam_line(0, 0, None).unwrap();
        g.hero.place(spot);
        let hp = g.hero.hp;
        if let Some(m) = g.pride_mut() {
            m.beam = Some(Beam { relay: 0, from: line.0, to: line.1, age: BEAM_TELL, touched: false, bite: 0.0 });
        }
        g.update(STEP);
        assert_eq!(g.hero.hp, hp - BEAM_DAMAGE as i32, "it burns");
        assert!(mirrors(&g).splinter > 0.0, "and lodges a Splinter");
        run(&mut g, BEAM_BITE_EVERY + 0.05);
        assert!(g.hero.hp < hp - BEAM_DAMAGE as i32, "and keeps burning");
        // In the air of a leap it passes under him.
        let mut g = in_realm(Sin::Pride);
        g.hero.place(spot);
        g.hero.face(Dir::Left);
        let hp = g.hero.hp;
        assert!(g.hero.try_jump(&g.level.grid));
        g.update(STEP);
        if let Some(m) = g.pride_mut() {
            m.beam = Some(Beam { relay: 0, from: line.0, to: line.1, age: BEAM_TELL, touched: false, bite: 0.0 });
        }
        g.update(STEP);
        assert_eq!(g.hero.hp, hp);
    }

    /// At the throne with the Peerless risen across it, he standing on the dais facing north and
    /// it risen, `glory` carried in.
    fn at_the_throne(glory: u32) -> Game {
        let mut g = in_realm(Sin::Pride);
        g.cheat_mirrors();
        g.update(STEP);
        assert_eq!(mirrors(&g).gallery, THRONE);
        assert!(g.juice.banner.as_ref().is_some_and(|b| b.text == "YOU STAND ABOVE THEM ALL. TAKE YOUR THRONE."));
        if let Some(m) = g.pride_mut() {
            m.glory = glory;
        }
        let c = mirrors(&g).throne;
        g.hero.place(P::new(c.x, c.y + 1));
        g.hero.face(Dir::Up);
        g.update(STEP);
        assert!(mirrors(&g).echo.fight, "the dais raises it");
        // Through its rising, standing still.
        run(&mut g, (RISE_FRAMES as f64 + ECHO_DELAY as f64 + 4.0) / 60.0);
        g
    }

    fn peerless(g: &Game) -> &Duelist {
        let i = g.pride_peerless().expect("the Peerless");
        g.enemies[i].duel.as_deref().unwrap()
    }

    #[test]
    fn the_dais_raises_the_peerless_swollen_by_his_glory() {
        let plain = at_the_throne(0);
        let proud = at_the_throne(3);
        let (a, b) = (peerless(&plain), peerless(&proud));
        assert!(a.puppet && b.puppet);
        assert!((b.max_hp / a.max_hp - 1.3).abs() < 0.02, "{} vs {}", b.max_hp, a.max_hp);
        assert!((b.breed.damage / a.breed.damage - 1.3).abs() < 1e-9);
        // Its string is his.
        let kit = plain.hero_kit();
        assert_eq!(a.f.table, kit.moves());
        // Across the throne from him, facing him.
        let c = mirrors(&plain).throne.v();
        let hp = plain.hero.pos();
        assert!(a.pos.y <= c.y && hp.y > c.y && math::len(math::sub(a.pos, hp)) >= ECHO_APART);
        assert!(math::dot(a.facing, math::norm(math::sub(hp, a.pos))) > 0.9);
    }

    #[test]
    fn the_echo_repeats_his_move_twenty_four_frames_later_mirrored() {
        let mut g = at_the_throne(0);
        g.input.inject(Action::Attack);
        let (mut his, mut its) = (None, None);
        for _ in 0..240 {
            g.update(STEP);
            let f = frame(&g);
            if his.is_none() && g.brawl.as_ref().unwrap().hero.active().is_some() {
                his = Some(f);
            }
            if its.is_none() && peerless(&g).f.active().is_some() {
                its = Some(f);
            }
        }
        let (his, its) = (his.expect("his swing"), its.expect("its echo"));
        assert_eq!(its - his, ECHO_DELAY, "the same swing, {ECHO_DELAY} frames on");
        let aim = g.hero.aim();
        assert!(math::dot(peerless(&g).facing, aim) < -0.9, "mirrored: it faces back at him");
        // It struck him (he stood still): the outer ring has fallen into the well.
        assert_eq!(mirrors(&g).rings, 1);
        let landing = mirrors(&g).landing;
        assert_eq!(g.level.grid.get(landing.x, landing.y), Tile::Pit);
    }

    #[test]
    fn a_parried_echo_reels_and_drops_the_moves_it_missed() {
        let mut g = at_the_throne(0);
        g.input.inject(Action::Attack);
        // Raise the guard as the swing ends, so the echo lands in the parry window.
        let mut f0 = None;
        for _ in 0..400 {
            g.update(STEP);
            let b = g.brawl.as_ref().unwrap();
            if f0.is_none() && b.hero.attack().is_some() {
                f0 = Some(b.threat.frame);
            }
            let held = f0.is_some_and(|f| b.threat.frame >= f + 20) && b.stats.parries == 0;
            if held && !g.input.held(Action::Block) {
                g.input.inject(Action::Block);
            } else if !held {
                g.input.inject_release(Action::Block);
            }
            if b.stats.parries > 0 {
                break;
            }
        }
        assert_eq!(g.brawl.as_ref().unwrap().stats.parries, 1, "the echo parried");
        assert_eq!(peerless(&g).f.reeling(), Some(Reel::Deflected));
        g.input.inject_release(Action::Block);
        // Punish it with a full chain while it reels: none of it comes back.
        let hp = g.hero.hp;
        let before = peerless(&g).hp;
        for _ in 0..3 {
            g.input.inject(Action::Attack);
            run(&mut g, 0.2);
        }
        run(&mut g, 1.4);
        assert!(peerless(&g).hp < before, "punished");
        assert_eq!(g.hero.hp, hp, "the moves it missed are dropped, not saved");
        assert_eq!(mirrors(&g).rings, 0);
    }

    #[test]
    fn three_blows_of_the_peerless_drop_three_rings_and_cast_him_down() {
        let mut g = at_the_throne(0);
        let landing = mirrors(&g).landing;
        g.wound_hero(1.0, 0xff0000);
        g.update(STEP);
        assert_eq!(mirrors(&g).rings, 1);
        assert_eq!(g.level.grid.get(landing.x, landing.y), Tile::Pit, "the outer ring falls");
        assert!(g.level.grid.is_walkable(mirrors(&g).throne.x, mirrors(&g).throne.y, false));
        g.wound_hero(1.0, 0xff0000);
        g.update(STEP);
        assert_eq!(mirrors(&g).rings, 2);
        g.wound_hero(1.0, 0xff0000);
        g.update(STEP);
        assert!(mirrors(&g).fall.is_some_and(|f| !f.win), "cast down");
        assert!(g.pride_peerless().is_none());
        run(&mut g, FALL_SECONDS + 0.1);
        let m = mirrors(&g);
        assert_eq!((m.gallery, g.hero.occupied()), (0, m.base.arrival), "to the first gallery, to climb again");
        assert!(g.level.grid.is_walkable(landing.x, landing.y, false), "the throne whole again for the next climb");
        assert!(!g.gate_open);
    }

    #[test]
    fn the_peerless_shattered_he_falls_the_spires_length_to_the_open_court_portal() {
        let mut g = at_the_throne(0);
        let i = g.pride_peerless().unwrap();
        g.enemies[i].duel.as_mut().unwrap().hp = 1.0;
        g.stray_blow(i);
        run(&mut g, 0.3);
        assert!(g.gate_open, "the court's portal opens");
        assert!(mirrors(&g).won && mirrors(&g).fall.is_some_and(|f| f.win));
        assert!(g.hero.altitude < 0.0, "falling");
        // (The last foe's fall slows time for a beat: the fall takes that much longer.)
        run(&mut g, FALL_SECONDS + 0.6);
        let m = mirrors(&g);
        assert_eq!(g.hero.occupied(), m.base.exit.front(), "at the foot of the spire, before the court");
        assert_eq!(m.gallery, 0);
        assert!(g.juice.banner.as_ref().is_some_and(|b| b.text == "HOW ART THOU FALLEN"));
        // Through the portal to Lucifer's court.
        for _ in 0..(3.0 / STEP) as usize {
            g.update(STEP);
            g.hero.invuln = 1.0;
        }
        assert!(g.door >= 1.0, "the doors swing open");
        let exit = mirrors(&g).base.exit.tile;
        g.hero.m.teleport(exit);
        g.update(STEP);
        assert_eq!(g.screen, Screen::Cleared);
    }

    #[test]
    fn slain_at_the_throne_he_rises_on_its_landing_and_the_fight_begins_afresh() {
        let mut g = at_the_throne(2);
        g.wound_hero(1.0, 0xff0000);
        g.update(STEP);
        g.hero.hp = 1;
        g.wound_hero(5.0, 0xff0000);
        assert_eq!(g.screen, Screen::Dying);
        run(&mut g, 3.0);
        let m = mirrors(&g);
        assert_eq!(g.hero.occupied(), m.landing);
        assert_eq!((m.gallery, m.glory, m.rings, m.echo.fight), (THRONE, 0, 0, false));
        assert!(g.pride_peerless().is_none());
        assert!(g.level.grid.is_walkable(m.landing.x, m.landing.y, false));
    }

    /// A mirror-door's recess sends him through the glass the moment he sets foot in it: no
    /// camera ever frames him standing there (the camera check passes over those tiles, QA.md Q9).
    #[test]
    fn a_mirror_doors_recess_is_passed_through_at_once() {
        let mut g = in_realm(Sin::Pride);
        let tiles: Vec<P> = mirrors(&g).doors.iter().flat_map(|d| d.tiles.iter().copied()).collect();
        assert!(!tiles.is_empty());
        for &t in &tiles {
            assert!(g.passes_through(t), "{t:?}");
        }
        assert!(!g.passes_through(mirrors(&g).base.arrival), "the floor he arrives on is not");
        let (t, d) = (tiles[0], mirrors(&g).door_at(tiles[0]).unwrap());
        g.hero.place(t);
        g.update(STEP);
        assert!(!mirrors(&g).doors[d].tiles.contains(&g.hero.occupied()), "through the glass at once");
    }
}
