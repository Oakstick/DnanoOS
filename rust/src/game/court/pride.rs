//! Pride's court in play: the Throne of the Morning Star (`logic::court::pride` lays it out) and
//! Lucifer, its prince (`logic::combat::prince::pride`: his tables, his hide, Vainglory).
//!
//! What his fight does besides his frame data is here. His Mirror Step marks the mirror nearest
//! the hero through its tracking frames and takes him out of it as it comes due; his fall carries
//! him on his wings to his mark and brings him down there, shaking the floor; his stars fall from
//! the height of the hall. As the second phase begins two Reflections step out of the mirrors
//! nearest the hero (at his side from the start, if his Chamberlain's decree was done; never, if
//! the Chamberlain was slain): cold copies of him that cut as he does at half his weight, and
//! shatter at one blow. As he falls, they shatter with him.

use crate::audio::Sfx;
use crate::game::Game;
use crate::logic::combat::ails::{Flash, FlashKind};
use crate::logic::combat::duelist::Lore;
use crate::logic::combat::prince::pride::{FALL_RISE, REFLECTIONS, REFLECTION_RISE};
use crate::logic::combat::{math, MoveId, Phase};
use crate::logic::court::{adds_weight, pride as plan};
use crate::logic::enemy::{EnemyKind, EnemyMode};
use crate::logic::types::{dist_sq, P, V};

/// The colours of his court's words: the morning star's gold, and the mirrors' silver.
const GOLD: u32 = 0xffe08a;
const SILVER: u32 = 0xd8e4ff;

impl Game {
    /// As Lucifer is set at the foot of his steps: the one thing to know, over his head.
    pub(super) fn pride_enter(&mut self, _prince: u32) {
        let at = plan::LUCIFER;
        self.floater(at.0 as f64, at.1 as f64 - 2.4, "PRIDE NEVER LOOKS BEHIND HIM", 3.0, Some(GOLD));
    }

    /// Each tick of the fight in his court: his mirror step marks its mirror, his fall carries him
    /// on his wings to his mark, his stars fall from the height of the hall.
    pub(super) fn pride_tick(&mut self, _dt: f64) {
        let Some(id) = self.court.as_ref().filter(|c| !c.slain).map(|c| c.prince) else { return };
        let hp = self.hero.pos();
        let Some(d) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == id && d.alive()) else {
            return;
        };
        match d.f.attack() {
            Some((MoveId::MirrorStep, Phase::Startup, frame)) if frame < MoveId::MirrorStep.data().track => {
                d.lore.mark = mirror_for(hp, d.pos);
            }
            Some((MoveId::PridesFall, Phase::Startup, frame)) if frame >= FALL_RISE => {
                // Up on his wings and along to where he will come down, there as it lands.
                let s = MoveId::PridesFall.data().startup;
                let u = (frame - FALL_RISE) as f64 / (s - FALL_RISE).max(1) as f64;
                let u = u * u * (3.0 - 2.0 * u);
                d.pos = math::add(d.lore.from, math::scale(math::sub(d.lore.mark, d.lore.from), u));
            }
            _ => {}
        }
        if let Some(b) = self.brawl.as_mut() {
            for t in b.traps.iter_mut().filter(|t| t.owner == id) {
                t.fell = true;
            }
        }
    }

    /// As his health falls into `phase`: as the second begins, his Reflections step out of the
    /// mirrors (unless they already have, or the Chamberlain's death has lost them).
    pub(super) fn pride_phase(&mut self, prince: u32, at: V, phase: u8) {
        if phase >= 2 && self.adds_unsummoned(prince) {
            self.raise_reflections(prince, at, false);
        }
        if phase >= 3 {
            self.floater(at.x, at.y - 2.6, "HIS WINGS BURN", 1.8, Some(0xff8a3a));
        }
    }

    /// After a fall (his Reflections are already gone).
    pub(super) fn pride_reset(&mut self) {}

    /// As he falls: the morning star brought down, his Reflections shattering with him.
    pub(super) fn pride_slain(&mut self, at: V) {
        let id = self.court.as_ref().map(|c| c.prince);
        let mut shards = Vec::new();
        for e in self.enemies.iter_mut() {
            let Some(d) = e.duel.as_deref_mut().filter(|d| d.alive() && d.lore.master.is_some() && d.lore.master == id) else { continue };
            d.f.kill();
            shards.push(d.pos);
            e.b.mode = EnemyMode::Eaten;
        }
        for p in &shards {
            self.view.sparkle(&self.level.grid, p.x, p.y, 1.2, SILVER, 30);
        }
        if !shards.is_empty() {
            self.audio.play(Sfx::Bells);
        }
        self.floater(at.x, at.y - 2.0, "HOW ART THOU FALLEN", 2.4, Some(GOLD));
        self.view.sparkle(&self.level.grid, at.x, at.y, 2.6, GOLD, 60);
    }

    /// The phase banner's line under his name, and its colour.
    pub(super) fn pride_banner(phase: u8) -> (&'static str, u32) {
        match phase {
            2 => ("HE STEPS THROUGH THE MIRRORS   ONE BLOW SHATTERS A REFLECTION", SILVER),
            _ => ("THE STARS FALL   STEP OFF HIS MARK, STRIKE AS HE LANDS", 0xff8a3a),
        }
    }

    /// His adds at once (his Chamberlain's decree done): his Reflections, at his side.
    pub(super) fn pride_raise(&mut self, prince: u32, at: V) {
        self.raise_reflections(prince, at, true);
    }

    /// Two Reflections step out of the glass: out of the mirrors nearest the hero (not too near
    /// him), or, `beside` him, either side of the prince. One blow shatters each; they fall when he
    /// does.
    fn raise_reflections(&mut self, id: u32, at: V, beside: bool) {
        let hp = self.hero.pos();
        let grid = &self.level.grid;
        let open = |p: V| grid.is_walkable(p.x.round() as i32, p.y.round() as i32, false);
        let spots: Vec<V> = if beside || !self.in_court() {
            let across = math::norm(V::new(-(hp.y - at.y), hp.x - at.x));
            [-1.0, 1.0].iter().map(|s| math::add(at, math::scale(across, 2.2 * s))).map(|p| if open(p) { p } else { at }).collect()
        } else {
            let mut doors: Vec<V> = plan::mirror_doors()[..plan::MIRRORS.len()].to_vec();
            doors.sort_by(|a, b| {
                let score = |p: &V| (dist_sq(*p, hp).sqrt() - 4.0).abs();
                score(a).total_cmp(&score(b))
            });
            doors.into_iter().take(REFLECTIONS as usize).collect()
        };
        let tier = self.realm_tier();
        let (_, weight_damage) = if self.in_court() { adds_weight(crate::logic::sins::Sin::Pride, tier) } else { (1.0, 1.0) };
        for p in spots {
            let facing = math::norm(math::sub(hp, p));
            let lore = Lore { master: Some(id), rise: REFLECTION_RISE, ..Lore::default() };
            if let Some(rid) = self.spawn_duelist(EnemyKind::Reflection, p, facing, lore) {
                if let Some(r) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == rid) {
                    // Glass: one good blow and it shatters.
                    r.max_hp = 1.0;
                    r.hp = 1.0;
                    r.breed.damage *= weight_damage;
                }
            }
            self.view.sparkle(&self.level.grid, p.x, p.y, 2.0, SILVER, 34);
        }
        self.floater(hp.x, hp.y - 1.5, "HIS REFLECTIONS STEP OUT", 1.6, Some(SILVER));
        self.audio.play(Sfx::Bells);
    }

    /// Lucifer's mirror step comes due (its first active frame): out of the mirror it marked, a
    /// flash of light at both mirrors, and he faces the hero, long in the open as he comes out.
    pub(in crate::game) fn lucifer_steps_out(&mut self, i: usize) {
        let hp = self.hero.pos();
        let in_pride = self.court_sin() == Some(crate::logic::sins::Sin::Pride);
        let Some(d) = self.enemies.get_mut(i).and_then(|e| e.duel.as_deref_mut()) else { return };
        let from = d.pos;
        let to = if in_pride { d.lore.mark } else { mirror_for(hp, d.pos) };
        let to = if self.level.grid.is_walkable(to.x.round() as i32, to.y.round() as i32, false) { to } else { from };
        d.pos = to;
        d.facing = math::norm(math::sub(hp, to));
        d.aim = d.facing;
        d.knock = V::new(0.0, 0.0);
        self.enemies[i].m.tile = P::new(to.x.round() as i32, to.y.round() as i32);
        for p in [from, to] {
            self.view.sparkle(&self.level.grid, p.x, p.y, 2.6, 0xfff4d8, 36);
        }
        self.view.shake(0.12);
        self.audio.play(Sfx::Portal);
    }

    /// Lucifer comes down on his mark (Pride's Fall's first active frame): there, the floor
    /// shaking round him.
    pub(in crate::game) fn lucifer_lands(&mut self, i: usize) {
        let Some(d) = self.enemies.get_mut(i).and_then(|e| e.duel.as_deref_mut()) else { return };
        let mark = d.lore.mark;
        d.pos = mark;
        let radius = match MoveId::PridesFall.data().shape {
            crate::logic::combat::frames::Shape::Mark { radius } => radius,
            _ => 1.4,
        };
        self.enemies[i].m.tile = P::new(mark.x.round() as i32, mark.y.round() as i32);
        let mut f = Flash::new(FlashKind::Quake, mark);
        f.length = radius + 0.6;
        if let Some(b) = self.brawl.as_mut() {
            b.flashes.push(f);
        }
        self.feel_quake(mark);
        self.view.dust(&self.level.grid, mark.x, mark.y, 0xe8dcc0, 30, 3.0);
        self.view.sparkle(&self.level.grid, mark.x, mark.y, 1.0, 0xffb060, 30);
        self.audio.play(Sfx::BossSlam);
    }

    /// Lucifer's lance of dawn blazes (its first active frame): the hall flashes white.
    pub(in crate::game) fn lucifer_lance(&mut self, i: usize) {
        let Some(d) = self.enemies.get(i).and_then(|e| e.duel.as_deref()) else { return };
        let end = math::add(d.pos, math::scale(d.aim, d.lore.span));
        self.view.shake(0.18);
        self.view.sparkle(&self.level.grid, end.x, end.y, 1.0, 0xfff4d8, 24);
        self.audio.play(Sfx::RaphaelBeam);
    }
}

/// The place before a mirror (or at the foot of the throne's) nearest the hero at `hero`, but not
/// on top of him, nor the one Lucifer at `at` stands before.
fn mirror_for(hero: V, at: V) -> V {
    let doors = plan::mirror_doors();
    let near = |p: &V| dist_sq(*p, hero).sqrt();
    doors
        .iter()
        .copied()
        .filter(|p| near(p) >= 2.4 && dist_sq(*p, at) > 4.0)
        .min_by(|a, b| near(a).total_cmp(&near(b)))
        .or_else(|| doors.iter().copied().max_by(|a, b| near(a).total_cmp(&near(b))))
        .unwrap_or(at)
}

#[cfg(test)]
mod tests {
    use super::super::testing::{self, run};
    use super::super::{COURT_CADENCE, COURT_CAPACITY};
    use super::*;
    use crate::game::{save, Screen};
    use crate::logic::combat::archetype::{Archetype, Power};
    use crate::logic::combat::frames::{Shape, REACTION};
    use crate::logic::combat::prince::{phase_of, COURT_HIDE};
    use crate::logic::combat::{Duelist, State};
    use crate::logic::sins::Sin;
    use crate::logic::types::Tile;
    use crate::logic::war::general::Record;

    fn court() -> Game {
        let mut g = testing::court(Sin::Pride);
        // (The stick is the world's way round, as the bots play.)
        g.view.camera_mode = crate::render::view::CameraMode::Top;
        g
    }

    /// He pushes the stick along `dir` for one tick.
    fn walk(g: &mut Game, dir: V) {
        g.input.set_pad(Some(crate::input::PadState { left: (dir.x as f32, dir.y as f32), ..Default::default() }));
        g.update(crate::app::STEP);
    }

    fn lucifer(g: &mut Game) -> &mut Duelist {
        testing::prince(g)
    }

    fn reflections(g: &Game) -> usize {
        g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.alive() && d.breed.kind == EnemyKind::Reflection).count()
    }

    /// Lucifer begins `mv` at `at`, facing the hero, his turn to strike.
    fn begin(g: &mut Game, at: V, mv: MoveId) {
        let hp = g.hero.pos();
        let d = lucifer(g);
        d.pos = at;
        d.facing = math::norm(math::sub(hp, at));
        d.rest = u16::MAX;
        d.token = true;
        d.f.state = State::Neutral;
        assert!(d.begin(mv, hp));
    }

    /// Frames of the brawl (two ticks each).
    fn frames(g: &mut Game, n: u16) {
        for _ in 0..n * 2 {
            g.update(crate::app::STEP);
        }
    }

    #[test]
    fn the_court_of_pride_is_the_throne_of_the_morning_star_and_lucifer_fights_there_by_frame_data() {
        let g = court();
        assert_eq!(g.level.data.name, "The Throne of the Morning Star");
        assert_eq!(g.realm, Some(Sin::Pride));
        assert!(g.boss.is_none(), "no Cyclops");
        assert!(g.arena() && g.brawling(), "a boss's arena, fought by frame data");
        let b = g.brawl.as_ref().unwrap();
        assert_eq!((b.threat.capacity, b.threat.cadence), (COURT_CAPACITY, COURT_CADENCE));
        let d = g.prince().expect("Lucifer stands");
        assert_eq!((d.breed.kind, d.arch, d.breed.phase, d.breed.power), (EnemyKind::Lucifer, Archetype::Prince, 1, Power::Vainglory));
        assert_eq!(d.pos, P::new(plan::LUCIFER.0, plan::LUCIFER.1).v(), "at the foot of his steps");
        assert_eq!(g.enemies.len(), 1, "he waits alone");
        let bar = g.prince_bar().unwrap();
        assert_eq!((bar.name.as_str(), bar.title), ("LUCIFER", "THE MORNING STAR"));
        assert_eq!(g.keys_total, 1, "one key opens the gate");
    }

    #[test]
    fn his_phases_change_at_the_thresholds_and_the_reflections_step_out_of_the_mirrors_once() {
        let mut g = court();
        let max = lucifer(&mut g).max_hp;
        lucifer(&mut g).rest = u16::MAX;
        let from = lucifer(&mut g).pos;
        lucifer(&mut g).hp = max * 0.75;
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        let hp = lucifer(&mut g).hp;
        assert_eq!(lucifer(&mut g).breed.phase, phase_of(hp, max));
        assert_eq!(reflections(&g), 0);
        // Under two thirds: the Mirror, his Reflections out of the glass, a green cross.
        lucifer(&mut g).hp = max * 0.6;
        lucifer(&mut g).rest = u16::MAX;
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        assert_eq!(lucifer(&mut g).breed.phase, 2);
        assert_eq!(reflections(&g), REFLECTIONS as usize, "two Reflections");
        let doors = plan::mirror_doors();
        for d in g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.breed.kind == EnemyKind::Reflection) {
            assert!(doors.iter().any(|p| dist_sq(*p, d.pos) < 0.01), "out of a mirror: {:?}", d.pos);
            assert!(d.lore.rise > 0 && d.lore.master == g.court.as_ref().map(|c| c.prince));
            assert_eq!(d.breed.moves(), &[MoveId::RadiantCut], "they cut as he does");
        }
        assert!(g.level.grid.count(Tile::Charge) >= 1, "a breather");
        assert!(g.juice.banner.as_ref().is_some_and(|b| b.text.contains("THE MIRROR")));
        assert_eq!(lucifer(&mut g).breed.moves()[0], MoveId::MirrorStep);
        // Under a third: the Fallen. No more Reflections.
        lucifer(&mut g).hp = max * 0.3;
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        assert_eq!(lucifer(&mut g).breed.phase, 3);
        assert_eq!(reflections(&g), REFLECTIONS as usize, "raised once");
        assert_eq!(lucifer(&mut g).breed.moves()[0], MoveId::Starfall);
        assert!(g.juice.banner.as_ref().is_some_and(|b| b.text.contains("THE FALLEN")));
    }

    #[test]
    fn his_vainglory_turns_a_blow_from_before_him_but_not_from_behind_nor_in_his_recovery() {
        let mut g = court();
        let d = lucifer(&mut g);
        d.rest = u16::MAX;
        d.facing = V::new(0.0, 1.0);
        let (pos, max) = (d.pos, d.max_hp);
        let blow = |g: &mut Game, from: V| {
            lucifer(g).hp = max;
            lucifer(g).pos = pos;
            lucifer(g).facing = V::new(0.0, 1.0);
            g.call_duel_hit(0, from);
            max - lucifer(g).hp
        };
        let before = blow(&mut g, math::add(pos, V::new(0.0, 1.5)));
        let side = blow(&mut g, math::add(pos, V::new(1.5, 0.0)));
        let back = blow(&mut g, math::add(pos, V::new(0.3, -1.5)));
        assert!(before > 0.0 && back > before, "his light turns the rest: before him {before}, at his back {back}");
        assert!((side - before).abs() < 1e-9, "beside him he still sees it: {side}");
        // In a recovery his light is open from every side.
        lucifer(&mut g).f.state = State::Attack { mv: MoveId::RadiantCut, frame: 60 };
        let open = blow(&mut g, math::add(pos, V::new(0.0, 1.5)));
        assert!(open > before && before <= (open * COURT_HIDE).ceil(), "his light turns the rest: {before} of {open}");
    }

    #[test]
    fn the_lance_of_dawn_follows_him_then_locks_and_strikes_along_its_line_unless_he_steps_out() {
        // He stands in its line: it strikes.
        let mut g = court();
        let hp = g.hero.pos();
        begin(&mut g, V::new(hp.x, hp.y - 6.0), MoveId::LanceOfDawn);
        let start = g.hero.hp;
        frames(&mut g, MoveId::LanceOfDawn.data().startup + 4);
        assert!(lucifer(&mut g).lore.span > 5.5, "it runs across the floor: {}", lucifer(&mut g).lore.span);
        assert!(g.hero.hp < start, "standing in its line, he is struck");
        // He steps out of its line as it locks: it passes him by.
        let mut g = court();
        let hp = g.hero.pos();
        begin(&mut g, V::new(hp.x, hp.y - 6.0), MoveId::LanceOfDawn);
        let d = MoveId::LanceOfDawn.data();
        frames(&mut g, d.track + REACTION);
        let aim = lucifer(&mut g).aim;
        let start = g.hero.hp;
        let side = V::new(-aim.y, aim.x);
        for _ in 0..(d.startup + d.active + 4 - d.track - REACTION) * 2 {
            walk(&mut g, side);
        }
        assert_eq!(g.hero.hp, start, "out of the line in time");
        assert!(matches!(lucifer(&mut g).f.attack(), Some((MoveId::LanceOfDawn, Phase::Recovery, _))), "and his recovery is open");
        assert!(lucifer(&mut g).exposed());
    }

    #[test]
    fn his_mirror_step_takes_him_out_of_the_mirror_nearest_the_hero_open_as_he_comes_out() {
        let mut g = court();
        // The hero by the east mirrors.
        let target = plan::before_mirror(plan::MIRRORS[1]);
        g.hero.place(P::new(target.x.round() as i32 - 3, target.y.round() as i32));
        let hp = g.hero.pos();
        let home = lucifer(&mut g).pos;
        begin(&mut g, home, MoveId::MirrorStep);
        frames(&mut g, MoveId::MirrorStep.data().startup + 2);
        let d = lucifer(&mut g);
        let doors = plan::mirror_doors();
        assert!(doors.iter().any(|p| dist_sq(*p, d.pos) < 0.01), "before a mirror: {:?}", d.pos);
        let near = |p: V| dist_sq(p, hp).sqrt();
        assert!(near(d.pos) < near(home) - 4.0, "nearer the hero: {:?}", d.pos);
        assert!(near(d.pos) >= 2.3, "not on top of him");
        assert!(d.exposed(), "coming out of the glass, his light is open");
        assert!(math::dot(d.facing, math::norm(math::sub(hp, d.pos))) > 0.9, "facing him");
    }

    #[test]
    fn a_reflection_shatters_at_one_blow_and_they_shatter_with_him() {
        let mut g = court();
        let (id, at) = (g.court.as_ref().unwrap().prince, lucifer(&mut g).pos);
        g.raise_reflections(id, at, true);
        assert_eq!(reflections(&g), 2);
        run(&mut g, 1.0);
        let k = g.enemies.iter().position(|e| e.kind() == EnemyKind::Reflection).unwrap();
        let from = g.enemies[k].duel.as_ref().unwrap().pos;
        g.call_duel_hit(k, from);
        assert_eq!(reflections(&g), 1, "one blow, and it is glass on the floor");
        // As he falls, the other shatters with him.
        lucifer(&mut g).hp = 1.0;
        lucifer(&mut g).f.state = State::Attack { mv: MoveId::RadiantCut, frame: 60 };
        let p = g.enemies.iter().position(|e| e.kind() == EnemyKind::Lucifer).unwrap();
        g.call_duel_hit(p, at);
        assert_eq!(reflections(&g), 0);
        assert!(g.prince().is_none());
    }

    #[test]
    fn his_fall_follows_the_hero_then_holds_and_brings_him_down_on_it_and_he_can_walk_out() {
        // He stands still: the fall comes down on him.
        let mut g = court();
        let hp = g.hero.pos();
        begin(&mut g, V::new(hp.x, hp.y - 5.0), MoveId::PridesFall);
        let start = g.hero.hp;
        let d = MoveId::PridesFall.data();
        frames(&mut g, FALL_RISE + 2);
        assert!(lucifer(&mut g).untouchable(), "up on his wings");
        frames(&mut g, d.startup - FALL_RISE + 2);
        assert!(g.hero.hp < start, "it fell on him");
        assert!(dist_sq(lucifer(&mut g).pos, hp) < 0.3, "he came down where the hero stood: {:?}", lucifer(&mut g).pos);
        // He walks out of it as he sees it lock: it falls behind him.
        let mut g = court();
        let hp = g.hero.pos();
        begin(&mut g, V::new(hp.x, hp.y - 5.0), MoveId::PridesFall);
        let start = g.hero.hp;
        frames(&mut g, REACTION);
        for _ in 0..(d.startup - REACTION + d.active + 2) * 2 {
            walk(&mut g, V::new(1.0, 0.0));
        }
        assert_eq!(g.hero.hp, start, "out of it in time");
        assert!(lucifer(&mut g).exposed(), "and he is open where he landed");
    }

    #[test]
    fn his_stars_fall_from_above_round_the_hero_and_burst() {
        let mut g = court();
        let hp = g.hero.pos();
        begin(&mut g, V::new(hp.x, hp.y - 4.0), MoveId::Starfall);
        let Shape::Traps { count, .. } = MoveId::Starfall.data().shape else { panic!("traps") };
        let start = g.hero.hp;
        let mut seen = 0;
        for _ in 0..400 {
            g.update(crate::app::STEP);
            let b = g.brawl.as_ref().unwrap();
            seen = seen.max(b.traps.len());
            if !b.traps.is_empty() {
                // (Marked as they come, a tick after they are thrown.)
                let flying = b.traps.iter().filter(|t| t.flight < crate::logic::combat::prince::COIN_FLIGHT);
                assert!(flying.clone().all(|t| t.fell), "they fall from the height of the hall");
            }
            if b.traps.iter().any(|t| t.burst.is_some()) {
                break;
            }
        }
        assert!(seen >= count as usize - 1 && seen <= count as usize, "{seen} stars");
        run(&mut g, 0.2);
        assert!(g.hero.hp < start, "standing on one, he is burnt");
    }

    #[test]
    fn lucifer_slain_drops_the_key_of_pride_and_the_way_on_opens() {
        let mut g = court();
        let from = lucifer(&mut g).pos;
        lucifer(&mut g).hp = 1.0;
        lucifer(&mut g).f.state = State::Attack { mv: MoveId::RadiantCut, frame: 60 };
        lucifer(&mut g).breed.phase = 3;
        g.call_duel_hit(0, from);
        assert!(g.prince().is_none(), "he has fallen");
        assert!(g.court.as_ref().unwrap().slain);
        assert!(g.red_key.is_some(), "the Key of Pride lies on the floor");
        assert!(g.sins_cleared & Sin::Pride.bit() != 0, "Pride's bit is set");
        assert_eq!(g.key_name(), "THE KEY OF PRIDE");
        assert!(g.saved_run.as_ref().is_some_and(|r| r.stage == save::Stage::SinShop(Sin::Pride)), "and the checkpoint is the old man's");
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
    fn a_fall_brings_lucifer_back_with_his_wounds_and_continue_comes_back_to_his_court() {
        let mut g = court();
        let max = lucifer(&mut g).max_hp;
        let (id, at) = (g.court.as_ref().unwrap().prince, lucifer(&mut g).pos);
        g.raise_reflections(id, at, false);
        lucifer(&mut g).hp = max * 0.5;
        lucifer(&mut g).breed.phase = 2;
        lucifer(&mut g).pos = V::new(20.0, 20.0);
        g.reset_round();
        assert_eq!(reflections(&g), 0, "his Reflections gone");
        let d = lucifer(&mut g);
        assert_eq!((d.hp, d.breed.phase), (max * 0.5, 2), "his wounds kept (WARBAND.md W10)");
        assert_eq!(d.pos, P::new(plan::LUCIFER.0, plan::LUCIFER.1).v(), "back at the foot of his steps");
        // As the fight begins again he takes up his Mirror: its banner, his Reflections, no breather.
        let crosses = g.level.grid.count(Tile::Charge);
        g.enter(Screen::Playing);
        run(&mut g, 0.05);
        assert_eq!(reflections(&g), 2, "they step out of the glass again");
        assert!(g.juice.banner.as_ref().is_some_and(|b| b.text.contains("THE MIRROR")));
        assert_eq!(g.level.grid.count(Tile::Charge), crosses, "no second breather for the same phase");
        assert!(g.floaters.iter().any(|f| f.text == super::super::WOUNDS_REMAIN), "said over him");
        g.checkpoint(save::Stage::Court(Sin::Pride));
        let run = g.saved_run.clone().unwrap();
        let mut h = Game::new();
        h.saved_run = Some(run);
        h.continue_game();
        assert!(h.in_court() && h.prince().is_some_and(|d| d.hp == d.max_hp), "a game over gives him back his health");
        assert_eq!(h.level.data.name, "The Throne of the Morning Star");
    }

    /// A game in his court with his Chamberlain's record as `r`.
    fn court_after(r: Record) -> Game {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test-court"));
        let mut g = Game::new();
        let mut all = g.general_records();
        all.set(Sin::Pride, r);
        g.warband.save.generals = all.encode();
        g.map_jump(save::Stage::Court(Sin::Pride));
        g.enter(Screen::Playing);
        g.hero.free_move = true;
        g
    }

    #[test]
    fn the_chamberlain_slain_he_has_no_reflections_and_his_decree_done_they_stand_at_his_side() {
        // Slain: no Reflections when the Mirror comes.
        let mut g = court_after(Record { met: true, slain: true, ..Default::default() });
        assert!(lucifer(&mut g).lore.summoned, "his Reflections are lost");
        let (max, from) = (lucifer(&mut g).max_hp, lucifer(&mut g).pos);
        lucifer(&mut g).hp = max * 0.6;
        g.call_duel_hit(0, from);
        run(&mut g, 0.1);
        assert_eq!(lucifer(&mut g).breed.phase, 2);
        assert_eq!(reflections(&g), 0, "the glass stays empty");
        // The throne-mirror polished: they stand at his side as the fight begins, and not again.
        let mut g = court_after(Record { met: true, decree_done: true, ..Default::default() });
        run(&mut g, 0.05);
        assert_eq!(reflections(&g), REFLECTIONS as usize, "at his side from the start");
        let at = lucifer(&mut g).pos;
        for d in g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.breed.kind == EnemyKind::Reflection) {
            assert!(dist_sq(d.pos, at).sqrt() < 2.6, "beside him: {:?}", d.pos);
        }
        let (max, from) = (lucifer(&mut g).max_hp, lucifer(&mut g).pos);
        lucifer(&mut g).hp = max * 0.6;
        g.call_duel_hit(0, from);
        run(&mut g, 0.1);
        assert_eq!(reflections(&g), REFLECTIONS as usize, "not again at the Mirror");
    }

    /// PRINCES.md §1: Lucifer in his court, by the bots: with the kit a hero has on reaching him
    /// (tiers 1 and 2) the expert brings him down through all three phases with lives to spare; a
    /// masher loses every life to him.
    #[test]
    fn the_expert_beats_lucifer_in_his_court_and_a_masher_does_not() {
        use crate::game::bot::{court_fight, Loadout, Skill};
        use crate::logic::heroes::HeroKind;
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        for (tier, seed) in [(1, 1), (2, 2)] {
            let kit = (tier > 1).then(|| Loadout::all(HeroKind::Ragnar, tier));
            let f = court_fight(Sin::Pride, Skill::Expert, seed, kit, 200.0);
            assert!(f.slain && f.deaths <= 1 && f.phase == 3, "tier {tier}: {f:?}");
            assert_eq!(f.stats.overlaps, 0, "{f:?}");
        }
        let f = court_fight(Sin::Pride, Skill::Novice, 1, Some(Loadout::all(HeroKind::Ragnar, 2)), 200.0);
        assert!(!f.slain && f.game_over, "{f:?}");
    }
}
