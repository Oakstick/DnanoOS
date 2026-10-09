//! Gluttony's court in play: the Banquet of Flies (`logic::court::gluttony` lays it out) and
//! Beelzebub, its prince (PRINCES.md §5).
//!
//! What the court does of its own, beyond his frame data:
//!
//! - **His meal** (`Gorge`, Gluttony's own). Hurt, between his moves, he walks to the nearest
//!   platter still laden on his table (or to a green cross the hero has left lying on the floor)
//!   and eats: a long spell that heals him a share of his health on its first active frame. Struck
//!   while he eats, he chokes: he reels, staggered and open, and the platter is spoiled. Each
//!   platter feeds him once; the table bears four, six once his general's decree has laid the feast.
//! - **His scatter.** He bursts into flies (out of reach, drawn as a cloud) and gathers again on
//!   the mark that followed the hero, where he lands with a belly slam.
//! - **His flies** fly slower than a bolt: each fan's landing is planned again by the threat
//!   manager for their own pace as they leave his hand.
//! - **His swarms** rise from the food as his second phase begins (once; at once if the decree
//!   was done; never if his general is slain), and scatter when he falls.

use crate::audio::{Cry, Sfx};
use crate::game::Game;
use crate::logic::combat::duelist::{BoltLook, Cue, Flow, GridTerrain, Lore, Terrain, BOLT_RADIUS_SUM};
use crate::logic::combat::frames::{BOLT_SPEED, MARK_FOLLOW};
use crate::logic::combat::prince::gluttony::{
    CHOKE, CROSS_GRACE, CROSS_HEAL, FLY_PACE, GORGE_BELOW, GORGE_HEAL, GORGE_REST, GORGE_WALK, SCATTER_GATHER, SCATTER_SINK, SWARMS,
    SWARM_RISE,
};
use crate::logic::combat::prince::PHASE_AT;
use crate::logic::combat::{math, MoveId, Reel, State};
use crate::logic::court::adds_weight;
use crate::logic::court::gluttony::{beside_platter, platter_stand, PLATTERS, PLATTERS_LAID, PLATTERS_SET};
use crate::logic::enemy::EnemyKind;
use crate::logic::sins::Sin;
use crate::logic::types::{Tile, P, V};

/// The girth his walk to his food keeps clear of the table and the walls (less than his body's:
/// his belly may brush a corner).
const WALK_BODY: f64 = 0.5;

/// What lies on one of his platters.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Dish {
    /// Laden with food grey with mould.
    #[default]
    Laden,
    /// Eaten bare: it fed him.
    Eaten,
    /// Spoiled: he choked on it.
    Spoiled,
}

/// What he is going to eat: a platter on his table (by its place in `PLATTERS`), or a green cross
/// on the floor.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Food {
    Platter(usize),
    Cross(P),
}

/// A meal under way: what, where he stands to eat it, the frames he has walked to it, his health
/// as he began to eat (none: still walking), and whether it has fed him.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Meal {
    pub food: Food,
    pub stand: V,
    pub walked: u32,
    pub eating: Option<f64>,
    pub fed: bool,
}

/// What the court keeps of Beelzebub's banquet.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Banquet {
    /// Each platter on the table (four, or six with the feast laid), in the order of `PLATTERS`.
    pub dishes: Vec<Dish>,
    /// His meal under way.
    pub meal: Option<Meal>,
    /// Frames before he may set out to eat again.
    pub rest: u32,
    /// The green crosses on the floor, and the frames each has lain there.
    pub crosses: Vec<(P, u32)>,
    /// The last frame of the fight the court has seen (its work is done once a frame).
    pub frame: u32,
    /// The fans of flies already slowed (by volley).
    pub slowed: Vec<u32>,
    /// A scatter is under way: he is (or was) gone in flies.
    pub scatter: bool,
    /// Meals eaten and meals choked on, this fight.
    pub fed: u32,
    pub choked: u32,
    /// The view has been shown the table.
    pub shown: bool,
}

impl Game {
    /// As Beelzebub is set before his throne: the table laid, four platters or six.
    pub(super) fn gluttony_enter(&mut self, _prince: u32) {
        let laid = self.general_records().get(Sin::Gluttony).decree_done;
        let n = if laid { PLATTERS_LAID } else { PLATTERS_SET };
        if let Some(c) = self.court.as_mut() {
            c.banquet = Banquet { dishes: vec![Dish::Laden; n], ..Banquet::default() };
        }
    }

    /// Each tick of the fight in the Banquet of Flies (its work once a frame of the fight): the
    /// crosses on the floor counted, his flies slowed, his scatter, his meals.
    pub(super) fn gluttony_tick(&mut self, _dt: f64) {
        let Some(frame) = self.brawl.as_ref().map(|b| b.threat.frame) else { return };
        let Some(c) = self.court.as_mut() else { return };
        if c.slain || c.banquet.frame == frame {
            return;
        }
        c.banquet.frame = frame;
        let id = c.prince;
        let mut b = std::mem::take(&mut c.banquet);
        if !b.shown {
            // (The view is built after the court is entered: it learns the table on the first frame.)
            b.shown = true;
            self.show_dishes(&b);
        }
        if let Some(i) = self.enemies.iter().position(|e| e.duel.as_ref().is_some_and(|d| d.id == id && d.alive())) {
            self.count_crosses(&mut b);
            self.slow_flies(&mut b, id);
            self.scatter(&mut b, i);
            self.meal(&mut b, i);
        }
        if let Some(c) = self.court.as_mut() {
            c.banquet = b;
        }
    }

    /// As his health falls into `phase`: as the second begins, his swarms rise from the food
    /// (unless they already have, or his general's death has lost them).
    pub(super) fn gluttony_phase(&mut self, prince: u32, at: V, phase: u8) {
        if phase >= 2 && self.adds_unsummoned(prince) {
            self.raise_swarms(prince, at);
        }
    }

    /// After a fall: the table laid again as it was, his meal and his flies forgotten.
    pub(super) fn gluttony_reset(&mut self) {
        let Some(c) = self.court.as_mut() else { return };
        let n = c.banquet.dishes.len();
        let id = c.prince;
        c.banquet = Banquet { dishes: vec![Dish::Laden; n], frame: c.banquet.frame, ..Banquet::default() };
        if let Some(d) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == id) {
            d.puppet = false;
            d.lore.vanish = 0;
        }
    }

    /// As he falls: his flies scatter, and the swarms with them.
    pub(super) fn gluttony_slain(&mut self, at: V) {
        let swarms: Vec<usize> = (0..self.enemies.len())
            .filter(|&k| self.enemies[k].duel.as_ref().is_some_and(|d| d.alive() && d.breed.kind == EnemyKind::FlySwarm))
            .collect();
        for k in swarms {
            let p = self.enemies[k].duel.as_ref().map(|d| d.pos).unwrap_or(at);
            if let Some(d) = self.enemies[k].duel.as_mut() {
                d.f.kill();
            }
            self.enemies[k].b.mode = crate::logic::enemy::EnemyMode::Eaten;
            self.view.dust(&self.level.grid, p.x, p.y, 0x2a2a1a, 18, 2.4);
        }
        if let Some(c) = self.court.as_mut() {
            c.banquet.meal = None;
        }
        self.view.dust(&self.level.grid, at.x, at.y, 0x1a1a10, 40, 3.2);
        self.floater(at.x, at.y - 1.8, "THE FLIES DISPERSE", 2.0, Some(0x9ab030));
        self.audio.play(Sfx::Whoosh);
    }

    /// The phase banner's line under his name, and its colour.
    pub(super) fn gluttony_banner(phase: u8) -> (&'static str, u32) {
        match phase {
            2 => ("HE BURSTS INTO FLIES   STEP OFF THE RING WHERE HE GATHERS", 0x9ab030),
            _ => ("SWOLLEN, HE ROLLS AND SPEWS BILE   STRIKE HIM AS HE EATS", 0xd8a040),
        }
    }

    /// His adds at once (his general's decree done): the swarms.
    pub(super) fn gluttony_raise(&mut self, prince: u32, at: V) {
        self.raise_swarms(prince, at);
    }

    /// Two swarms of flies rise from the food on the platters nearest the hero (in the ring, out of
    /// the air either side of him); they fall when he does.
    fn raise_swarms(&mut self, id: u32, at: V) {
        let hp = self.hero.pos();
        let spots: Vec<V> = if self.in_court() {
            let n = self.court.as_ref().map(|c| c.banquet.dishes.len()).unwrap_or(PLATTERS_SET).max(SWARMS as usize);
            let mut near: Vec<P> = PLATTERS[..n].iter().map(|&p| beside_platter(p)).collect();
            near.sort_by(|a, b| math::len(math::sub(a.v(), hp)).total_cmp(&math::len(math::sub(b.v(), hp))));
            near.iter().take(SWARMS as usize).map(|p| p.v()).collect()
        } else {
            let across = math::norm(V::new(-(hp.y - at.y), hp.x - at.x));
            [-1.0, 1.0].iter().map(|s| math::add(at, math::scale(across, 2.2 * s))).collect()
        };
        let (weight_hp, weight_damage) =
            if self.in_court() { self.ng_foe(adds_weight(crate::logic::sins::Sin::Gluttony, self.realm_tier())) } else { (1.0, 1.0) };
        for p in spots {
            let facing = math::norm(math::sub(hp, p));
            let lore = Lore { master: Some(id), rise: SWARM_RISE, ..Lore::default() };
            if let Some(sid) = self.spawn_duelist(EnemyKind::FlySwarm, p, facing, lore) {
                if let Some(s) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == sid) {
                    s.max_hp *= weight_hp;
                    s.hp = s.max_hp;
                    s.breed.damage *= weight_damage;
                }
            }
            self.view.dust(&self.level.grid, p.x, p.y, 0x1a1a10, 30, 1.6);
        }
        self.floater(hp.x, hp.y - 1.5, "THE FLIES RISE FROM THE FOOD", 1.6, Some(0x9ab030));
        self.audio.play(Sfx::Whoosh);
    }

    /// The green crosses lying on the floor, and how long each has lain there.
    fn count_crosses(&mut self, b: &mut Banquet) {
        let g = &self.level.grid;
        let now: Vec<P> = g.cells().filter(|&(_, _, t)| t == Tile::Charge).map(|(x, y, _)| P::new(x, y)).collect();
        b.crosses = now.into_iter().map(|p| (p, b.crosses.iter().find(|c| c.0 == p).map(|c| c.1 + 1).unwrap_or(0))).collect();
    }

    /**
     * His fans of flies, as they leave his hand: each is planned again to land when flies at
     * `FLY_PACE` of a bolt's pace would reach the hero, if the threat manager can fit it there
     * (else they fly at a bolt's pace, on the frame it first planned).
     */
    fn slow_flies(&mut self, b: &mut Banquet, id: u32) {
        let hp = self.hero.pos();
        let Some(br) = self.brawl.as_mut() else { return };
        let now = br.threat.frame;
        let mut fresh: Vec<u32> = Vec::new();
        for bolt in br.bolts.iter().filter(|x| x.owner == id && x.look == BoltLook::Flies && !x.reflected && x.volley != 0) {
            if !b.slowed.contains(&bolt.volley) && !fresh.contains(&bolt.volley) {
                fresh.push(bolt.volley);
            }
        }
        for v in fresh {
            b.slowed.push(v);
            let Some(first) = br.bolts.iter().find(|x| x.volley == v) else { continue };
            let travel = (math::len(math::sub(hp, first.pos)) - BOLT_RADIUS_SUM + 0.5).max(0.0);
            let frames = (travel / (BOLT_SPEED * FLY_PACE) * 60.0).ceil() as u16;
            // (A fan loosed outside the plan, as a capture's, has no landing to move: it only slows.)
            let planned = br.threat.landing(id).is_some();
            if planned && !br.threat.may_commit(id, frames) {
                continue;
            }
            if planned {
                br.threat.commit(id, frames);
            }
            for bolt in br.bolts.iter_mut().filter(|x| x.volley == v) {
                bolt.due = planned.then_some(now + frames as u32);
                bolt.vel = math::scale(math::norm(bolt.vel), BOLT_SPEED * FLY_PACE);
            }
        }
        let live: Vec<u32> = br.bolts.iter().map(|x| x.volley).collect();
        b.slowed.retain(|v| live.contains(v));
    }

    /**
     * His scatter: through its tracking frames the ring on the floor follows the hero (no faster
     * than `MARK_FOLLOW`); from `SCATTER_SINK` he is a cloud of flies (out of reach) drifting to
     * it, and `SCATTER_GATHER` frames before his slam he gathers on it again.
     */
    fn scatter(&mut self, b: &mut Banquet, i: usize) {
        let hp = self.hero.pos();
        let Some(du) = self.enemies[i].duel.as_deref_mut() else { return };
        let State::Attack { mv: MoveId::Scatter, frame } = du.f.state else {
            if b.scatter {
                // (Over, or broken off: he stands where he is, himself again.)
                b.scatter = false;
                if du.lore.vanish > 1 {
                    du.lore.vanish = 1;
                }
                if b.meal.is_none() {
                    du.puppet = false;
                }
            }
            return;
        };
        let d = du.data(MoveId::Scatter);
        if !b.scatter {
            b.scatter = true;
            du.puppet = true;
        }
        if frame < d.track {
            let to = math::sub(hp, du.lore.from);
            let goal = if math::len(to) > d.reach { math::add(du.lore.from, math::scale(math::norm(to), d.reach)) } else { hp };
            let gap = math::sub(goal, du.lore.mark);
            let step = MARK_FOLLOW / 60.0;
            du.lore.mark = if math::len(gap) <= step { goal } else { math::add(du.lore.mark, math::scale(math::norm(gap), step)) };
        }
        let gather = d.startup - SCATTER_GATHER;
        if (SCATTER_SINK..gather).contains(&frame) {
            if du.lore.vanish == 0 {
                du.cues.push(Cue::Vanished { from: du.pos, to: du.lore.mark });
            }
            du.lore.vanish = u16::MAX;
            let u = (frame - SCATTER_SINK) as f64 / (gather - SCATTER_SINK).max(1) as f64;
            let u = u * u * (3.0 - 2.0 * u);
            du.pos = math::add(du.lore.from, math::scale(math::sub(du.lore.mark, du.lore.from), u));
        } else if frame >= gather && du.lore.vanish > 1 {
            // (On the next frame he stands on the mark again: `Duelist::step`.)
            du.lore.vanish = 1;
        }
        if frame == d.startup {
            let at = du.lore.mark;
            self.view.dust(&self.level.grid, at.x, at.y, 0x4a4028, 26, 2.8);
            self.view.shake(0.3);
            self.feel_quake(at);
            self.audio.play(Sfx::BossSlam);
        }
    }

    /**
     * His meals. Hurt and between moves, he sets out for the nearest green cross the hero has
     * left lying (`CROSS_GRACE`) or else the nearest laden platter, walking round the table; there
     * he begins `Gorge`. A blow while he eats chokes him (staggered and open) and spoils the
     * platter; eaten through, it heals him (never past the threshold of his phase).
     */
    fn meal(&mut self, b: &mut Banquet, i: usize) {
        b.rest = b.rest.saturating_sub(1);
        let hp = self.hero.pos();
        let Some(du) = self.enemies[i].duel.as_deref() else { return };
        let (id, pos, kind, health, max) = (du.id, du.pos, du.breed.kind, du.hp, du.max_hp);
        let Some(mut m) = b.meal else {
            // Hungry? Between his moves, hurt, rested from his last meal, not in a scatter.
            let free = du.f.is_neutral() && !du.puppet && !b.scatter && du.lore.rise == 0;
            if !free || b.rest > 0 || health >= max * GORGE_BELOW {
                return;
            }
            let near = |p: V| math::len(math::sub(p, pos));
            let cross = b.crosses.iter().filter(|c| c.1 >= CROSS_GRACE).map(|c| c.0).min_by(|a, c| near(a.v()).total_cmp(&near(c.v())));
            let food = match cross {
                Some(p) => Some((Food::Cross(p), p.v())),
                None => (0..b.dishes.len())
                    .filter(|&k| b.dishes[k] == Dish::Laden)
                    .map(|k| (Food::Platter(k), platter_stand(PLATTERS[k])))
                    .min_by(|a, c| near(a.1).total_cmp(&near(c.1))),
            };
            let Some((food, stand)) = food else { return };
            b.meal = Some(Meal { food, stand, walked: 0, eating: None, fed: false });
            if let Some(du) = self.enemies[i].duel.as_deref_mut() {
                du.puppet = true;
                du.token = false;
            }
            if let Some(br) = self.brawl.as_mut() {
                br.threat.release(id);
            }
            let text = if matches!(food, Food::Cross(_)) { "HE GOES FOR YOUR CROSS!" } else { "HE GOES TO EAT" };
            self.floater(pos.x, pos.y - 2.2, text, 1.4, Some(0x9ab030));
            return;
        };
        let gone = match m.food {
            Food::Platter(k) => b.dishes.get(k) != Some(&Dish::Laden),
            Food::Cross(p) => self.level.grid.get(p.x, p.y) != Tile::Charge,
        };
        match m.eating {
            None => {
                if (gone && !m.fed) || m.walked > GORGE_WALK {
                    // Taken from under his nose, or out of his way: he leaves it.
                    b.meal = None;
                    b.rest = GORGE_REST / 3;
                    if let Some(du) = self.enemies[i].duel.as_deref_mut() {
                        du.puppet = false;
                    }
                    return;
                }
                m.walked += 1;
                let flow = Flow::new(&self.level.grid, P::new(m.stand.x.round() as i32, m.stand.y.round() as i32), 64);
                let ground = GridTerrain { grid: &self.level.grid, flow: &flow };
                let Some(du) = self.enemies[i].duel.as_deref_mut() else { return };
                if !du.f.is_neutral() {
                    // (Reeling from a blow on his way: he waits it out.)
                    b.meal = Some(m);
                    return;
                }
                let st = du.stats();
                let step = st.walk / 60.0;
                let left = math::len(math::sub(m.stand, du.pos));
                if left > step + 0.05 {
                    // Straight there while his bulk clears the way, else from tile to tile along the
                    // shortest way round (the table's corners would catch him cutting across).
                    let grid = &self.level.grid;
                    let fits = |p: V| {
                        let r = WALK_BODY * 0.9;
                        [(-r, -r), (r, -r), (-r, r), (r, r)]
                            .iter()
                            .all(|&(dx, dy)| grid.is_walkable((p.x + dx).round() as i32, (p.y + dy).round() as i32, false))
                    };
                    let n = (left / 0.25).ceil().max(1.0) as usize;
                    let clear = (0..=n).all(|k| fits(math::add(du.pos, math::scale(math::sub(m.stand, du.pos), k as f64 / n as f64))));
                    let here = P::new(du.pos.x.round() as i32, du.pos.y.round() as i32);
                    let goal = if clear { m.stand } else { flow.step(here).map(|t| t.v()).unwrap_or(m.stand) };
                    let way = math::norm(math::sub(goal, du.pos));
                    du.facing = math::norm(math::add(math::scale(du.facing, 0.8), math::scale(way, 0.2)));
                    du.vel = math::scale(way, st.walk);
                    du.pos = ground.slide(du.pos, math::scale(way, step), WALK_BODY);
                    b.meal = Some(m);
                    return;
                }
                // At the table (or over the cross): he bends to it and eats.
                du.vel = V::new(0.0, 0.0);
                let food_at = match m.food {
                    Food::Platter(k) => P::new(PLATTERS[k].0, PLATTERS[k].1).v(),
                    Food::Cross(p) => p.v(),
                };
                if math::len(math::sub(food_at, du.pos)) > 0.1 {
                    du.facing = math::norm(math::sub(food_at, du.pos));
                }
                if du.begin(MoveId::Gorge, hp) {
                    m.eating = Some(du.hp);
                    let at = du.pos;
                    let listener = self.audio.listener();
                    self.audio.tell(kind, MoveId::Gorge, at, listener);
                    self.floater(at.x, at.y - 2.4, "HE EATS: STRIKE HIM!", 1.4, Some(0xffd040));
                }
                b.meal = Some(m);
            }
            Some(before) => {
                let Some(du) = self.enemies[i].duel.as_deref() else { return };
                let frame = match du.f.state {
                    State::Attack { mv: MoveId::Gorge, frame } => Some(frame),
                    _ => None,
                };
                let (now, at, phase) = (du.hp, du.pos, du.breed.phase);
                if !m.fed && (now < before - 1e-6 || frame.is_none()) {
                    // A blow as he eats: he chokes on it, and the platter is spoiled.
                    if let Some(du) = self.enemies[i].duel.as_deref_mut() {
                        du.f.state = State::Reel { kind: Reel::Staggered, left: CHOKE, total: CHOKE };
                        du.puppet = false;
                    }
                    b.meal = None;
                    b.rest = GORGE_REST;
                    b.choked += 1;
                    if let Food::Platter(k) = m.food {
                        b.dishes[k] = Dish::Spoiled;
                    }
                    self.audio.foe_cry(kind, Cry::Stagger, at);
                    self.audio.play(Sfx::Drip);
                    self.floater(at.x, at.y - 2.4, "HE CHOKES!", 1.4, Some(0xffd040));
                    if matches!(m.food, Food::Platter(_)) {
                        self.floater(at.x, at.y - 1.8, "THE PLATTER IS SPOILED", 1.4, Some(0x9ab030));
                    }
                    self.view.dust(&self.level.grid, at.x, at.y, 0x6a7a20, 16, 1.6);
                    self.show_dishes(b);
                    return;
                }
                if !m.fed && frame.is_some_and(|f| f >= MoveId::Gorge.data().startup) {
                    // Eaten through: it heals him, never past his phase's threshold.
                    m.fed = true;
                    b.fed += 1;
                    let share = if matches!(m.food, Food::Cross(_)) { CROSS_HEAL } else { GORGE_HEAL };
                    let cap = match phase {
                        0 | 1 => max,
                        2 => max * PHASE_AT[0],
                        _ => max * PHASE_AT[1],
                    };
                    let healed = (now + (max * share).round()).min(cap).max(now);
                    if let Some(du) = self.enemies[i].duel.as_deref_mut() {
                        du.hp = healed;
                    }
                    match m.food {
                        Food::Platter(k) => b.dishes[k] = Dish::Eaten,
                        Food::Cross(p) => {
                            self.level.grid.consume(p.x, p.y);
                            self.view.remove_collectible(p.x, p.y);
                        }
                    }
                    self.audio.play(Sfx::Eat);
                    self.floater(at.x, at.y - 2.4, format!("+{}", (healed - now).round()), 1.0, Some(0x9ab030));
                    self.view.sparkle(&self.level.grid, at.x, at.y, 1.6, 0x9ab030, 18);
                    self.show_dishes(b);
                }
                if m.fed && frame.is_none() {
                    // Done, and back to the fight.
                    if let Some(du) = self.enemies[i].duel.as_deref_mut() {
                        du.puppet = false;
                    }
                    b.meal = None;
                    b.rest = GORGE_REST;
                    return;
                }
                b.meal = Some(m);
            }
        }
    }

    /// The table as the view draws it: each platter laden (0), eaten (1) or spoiled (2).
    fn show_dishes(&mut self, b: &Banquet) {
        let n = b.dishes.len();
        let states: Vec<u8> = b.dishes.iter().map(|d| *d as u8).collect();
        self.view.set_banquet(&PLATTERS[..n], &states);
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{self, run};
    use super::super::{COURT_CADENCE, COURT_CAPACITY};
    use super::*;
    use crate::game::{save, Screen};
    use crate::logic::combat::archetype::Archetype;
    use crate::logic::combat::prince::phase_of;
    use crate::logic::combat::{Duelist, Phase};
    use crate::logic::war::general::{Record, Records};

    fn court() -> Game {
        testing::court(Sin::Gluttony)
    }

    fn beelzebub(g: &mut Game) -> &mut Duelist {
        testing::prince(g)
    }

    fn banquet(g: &Game) -> &Banquet {
        &g.court.as_ref().unwrap().banquet
    }

    fn swarms(g: &Game) -> usize {
        g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.alive() && d.breed.kind == EnemyKind::FlySwarm).count()
    }

    /// He stands on the floor `dist` tiles north of the hero, facing him, and begins `mv`.
    fn begin(g: &mut Game, mv: MoveId, dist: f64) {
        let hp = g.hero.pos();
        let d = beelzebub(g);
        d.pos = V::new(hp.x, hp.y - dist);
        d.facing = V::new(0.0, 1.0);
        d.rest = u16::MAX;
        d.token = true;
        d.f.state = State::Neutral;
        assert!(d.begin(mv, hp));
    }

    #[test]
    fn the_court_of_gluttony_is_the_banquet_and_beelzebub_fights_there_by_frame_data() {
        let g = court();
        assert_eq!(g.level.data.name, "The Banquet of Flies");
        assert_eq!(g.realm, Some(Sin::Gluttony));
        assert!(g.boss.is_none(), "no Cyclops");
        assert!(g.arena() && g.brawling(), "a boss's arena, fought by frame data");
        let b = g.brawl.as_ref().unwrap();
        assert_eq!((b.threat.capacity, b.threat.cadence), (COURT_CAPACITY, COURT_CADENCE));
        let d = g.prince().expect("Beelzebub stands");
        assert_eq!((d.breed.kind, d.arch, d.breed.phase), (EnemyKind::Beelzebub, Archetype::Prince, 1));
        assert_eq!(g.enemies.len(), 1, "he waits alone");
        let bar = g.prince_bar().unwrap();
        assert_eq!((bar.name.as_str(), bar.title), ("BEELZEBUB", "THE LORD OF FLIES"));
        assert_eq!(g.keys_total, 1, "one key opens the gate");
        assert_eq!(banquet(&g).dishes, vec![Dish::Laden; PLATTERS_SET], "four platters laid");
    }

    #[test]
    fn his_phases_change_at_the_thresholds_and_the_swarms_rise_once() {
        let mut g = court();
        let max = beelzebub(&mut g).max_hp;
        let from = beelzebub(&mut g).pos;
        beelzebub(&mut g).rest = u16::MAX;
        beelzebub(&mut g).hp = max * 0.75;
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        let hp = beelzebub(&mut g).hp;
        assert_eq!(beelzebub(&mut g).breed.phase, phase_of(hp, max));
        assert_eq!(swarms(&g), 0);
        // Under two thirds: the Swarm, his flies rising from the food, a green cross.
        beelzebub(&mut g).hp = max * 0.6;
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        assert_eq!(beelzebub(&mut g).breed.phase, 2);
        assert_eq!(swarms(&g), SWARMS as usize, "two swarms");
        assert!(g.level.grid.count(Tile::Charge) >= 1, "a breather");
        assert!(g.juice.banner.as_ref().is_some_and(|b| b.text.contains("THE SWARM")));
        assert_eq!(beelzebub(&mut g).breed.moves()[0], MoveId::Scatter);
        // The swarms rose beside the platters.
        for d in g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.breed.kind == EnemyKind::FlySwarm) {
            assert!(PLATTERS.iter().any(|&p| math::len(math::sub(beside_platter(p).v(), d.pos)) < 0.6), "{:?}", d.pos);
        }
        // Under a third: the Glutton. The swarms are not raised again.
        beelzebub(&mut g).hp = max * 0.3;
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        assert_eq!(beelzebub(&mut g).breed.phase, 3);
        assert_eq!(swarms(&g), SWARMS as usize, "raised once");
        assert_eq!(beelzebub(&mut g).breed.moves()[0], MoveId::BellyRoll);
        assert!(g.juice.banner.as_ref().is_some_and(|b| b.text.contains("THE GLUTTON")));
    }

    #[test]
    fn his_bloat_turns_a_blow_but_not_in_his_recovery() {
        let mut g = court();
        let from = beelzebub(&mut g).pos;
        beelzebub(&mut g).rest = u16::MAX;
        let before = beelzebub(&mut g).hp;
        g.call_duel_hit(0, from);
        let warded = before - beelzebub(&mut g).hp;
        let d = beelzebub(&mut g);
        d.f.state = State::Attack { mv: MoveId::Cleaver, frame: 60 };
        let before = d.hp;
        g.call_duel_hit(0, from);
        let open = before - beelzebub(&mut g).hp;
        assert!(warded > 0.0 && open > warded * 1.5, "warded {warded}, open {open}");
    }

    /// He is hurt, rested and free, the hero far off: he goes to the nearest platter and eats.
    fn hungry() -> Game {
        let mut g = court();
        g.immortal = true;
        let d = beelzebub(&mut g);
        d.hp = d.max_hp * 0.75;
        d.pos = V::new(10.0, 12.0);
        d.rest = 30;
        g.hero.place(P::new(16, 28));
        g
    }

    /// Runs until he is eating, and returns the frame of his meal he is at.
    fn until_eating(g: &mut Game) -> u16 {
        for _ in 0..2400 {
            g.update(crate::app::STEP);
            beelzebub(g).token = false;
            if let State::Attack { mv: MoveId::Gorge, frame } = beelzebub(g).f.state {
                return frame;
            }
        }
        panic!("he never ate: {:?}", banquet(g).meal);
    }

    #[test]
    fn hurt_he_walks_to_a_platter_and_eats_and_it_heals_him_once() {
        let mut g = hungry();
        let before = beelzebub(&mut g).hp;
        until_eating(&mut g);
        let meal = banquet(&g).meal.unwrap();
        assert_eq!(meal.food, Food::Platter(0), "the west platter, nearest him");
        assert!(math::len(math::sub(beelzebub(&mut g).pos, platter_stand(PLATTERS[0]))) < 0.3, "at the table");
        // He eats through it: healed, the platter bare, and back to the fight.
        run(&mut g, 2.0);
        let d = beelzebub(&mut g);
        assert!(d.hp > before + d.max_hp * GORGE_HEAL * 0.9, "healed: {} from {before}", d.hp);
        assert!(!d.puppet);
        assert_eq!(banquet(&g).dishes[0], Dish::Eaten);
        assert_eq!(banquet(&g).fed, 1);
        // Each platter feeds him once: hurt again, he goes to another.
        beelzebub(&mut g).hp = before;
        if let Some(c) = g.court.as_mut() {
            c.banquet.rest = 0;
        }
        until_eating(&mut g);
        assert!(matches!(banquet(&g).meal.unwrap().food, Food::Platter(k) if k != 0));
    }

    #[test]
    fn struck_as_he_eats_he_chokes_open_and_the_platter_is_spoiled() {
        let mut g = hungry();
        until_eating(&mut g);
        let at = beelzebub(&mut g).pos;
        let before = beelzebub(&mut g).hp;
        g.call_duel_hit(0, at);
        run(&mut g, 0.05);
        let d = beelzebub(&mut g);
        assert!(matches!(d.f.state, State::Reel { kind: Reel::Staggered, .. }), "he chokes: {:?}", d.f.state);
        assert!(d.exposed(), "his bloat open");
        assert!(d.hp < before, "and no meal");
        assert_eq!(banquet(&g).dishes[0], Dish::Spoiled);
        assert_eq!(banquet(&g).choked, 1);
        // A blow on him choking lands whole.
        let before = beelzebub(&mut g).hp;
        g.call_duel_hit(0, at);
        let open = before - beelzebub(&mut g).hp;
        assert!(open >= MoveId::L1.data().damage * 0.95, "whole: {open}");
        // He never eats from a spoiled platter.
        run(&mut g, 3.0);
        assert_ne!(banquet(&g).meal.map(|m| m.food), Some(Food::Platter(0)));
    }

    #[test]
    fn he_eats_a_green_cross_the_hero_leaves_lying_but_not_one_he_takes_first() {
        let mut g = hungry();
        let cross = P::new(12, 18);
        g.level.grid.set(cross.x, cross.y, Tile::Charge);
        g.view.add_charge(&g.level.grid, cross.x, cross.y);
        // (Not hungry until the cross has lain there a while.)
        g.court.as_mut().unwrap().banquet.rest = CROSS_GRACE + 20;
        until_eating(&mut g);
        assert_eq!(banquet(&g).meal.unwrap().food, Food::Cross(cross), "he goes for the cross left lying");
        run(&mut g, 2.0);
        assert_eq!(g.level.grid.get(cross.x, cross.y), Tile::Open, "eaten");
        assert_eq!(banquet(&g).dishes, vec![Dish::Laden; PLATTERS_SET], "the platters untouched");
        // A cross the hero takes first is no meal of his.
        let mut g = hungry();
        g.level.grid.set(cross.x, cross.y, Tile::Charge);
        run(&mut g, 0.5);
        assert!(banquet(&g).crosses.iter().any(|c| c.0 == cross), "seen on the floor");
        g.hero.place(cross);
        run(&mut g, 0.2);
        assert_eq!(g.level.grid.get(cross.x, cross.y), Tile::Open, "he took it");
        until_eating(&mut g);
        assert!(matches!(banquet(&g).meal.unwrap().food, Food::Platter(_)));
    }

    #[test]
    fn he_scatters_into_flies_out_of_reach_and_gathers_on_the_mark_which_can_be_stepped_out_of() {
        let mut g = court();
        g.hero.hp = 1000;
        begin(&mut g, MoveId::Scatter, 6.0);
        let data = MoveId::Scatter.data();
        let mut gone = false;
        let mut start = g.hero.hp;
        let mut walked = false;
        for _ in 0..400 {
            g.update(crate::app::STEP);
            let (frame, untouchable) = match beelzebub(&mut g).f.state {
                State::Attack { mv: MoveId::Scatter, frame } => (frame, beelzebub(&mut g).untouchable()),
                _ => break,
            };
            if frame > SCATTER_SINK + 2 && frame < data.startup - SCATTER_GATHER {
                gone |= untouchable;
                assert!(untouchable, "a cloud of flies at frame {frame}");
            }
            // He walks out of the ring as soon as he sees it (a fifth of the time to spare).
            if frame >= 15 && !walked {
                g.input.set_pad(Some(crate::input::PadState { left: (1.0, 0.0), ..Default::default() }));
                walked = true;
            }
            if frame == data.startup - 1 {
                start = g.hero.hp;
            }
        }
        g.input.set_pad(None);
        assert!(gone, "he was out of reach");
        let d = beelzebub(&mut g);
        assert!(d.lore.vanish == 0 && !d.untouchable(), "himself again");
        assert!(math::len(math::sub(d.pos, d.lore.mark)) < 0.05, "on his mark");
        assert_eq!(g.hero.hp, start, "stepped out of it in time");
        // Stood on, it lands.
        let mut g = court();
        g.hero.hp = 1000;
        begin(&mut g, MoveId::Scatter, 5.0);
        let before = g.hero.hp;
        run(&mut g, 1.4);
        assert!(g.hero.hp < before, "the slam lands on him");
    }

    #[test]
    fn his_bile_lies_on_the_floor_bursts_and_burns_and_his_flies_come_slow() {
        let mut g = court();
        g.hero.hp = 1000;
        begin(&mut g, MoveId::BileSpew, 5.0);
        let mut seen = 0;
        for _ in 0..400 {
            g.update(crate::app::STEP);
            let b = g.brawl.as_ref().unwrap();
            seen = seen.max(b.traps.len());
            if b.traps.iter().any(|t| t.burst.is_some()) {
                break;
            }
        }
        assert_eq!(seen, 4, "four marks of bile");
        assert!(g.brawl.as_ref().unwrap().traps.iter().all(|t| t.linger > 60), "that linger");
        let before = g.hero.hp;
        beelzebub(&mut g).rest = u16::MAX;
        run(&mut g, 2.0);
        assert!(g.hero.hp < before, "standing in it, he burns");
        // The flies: slower than a bolt, landing as planned.
        let mut g = court();
        g.immortal = true;
        begin(&mut g, MoveId::Swarm, 6.0);
        let mut speed = 0.0;
        for _ in 0..200 {
            g.update(crate::app::STEP);
            if let Some(b) = g.brawl.as_ref().unwrap().bolts.first() {
                speed = math::len(b.vel);
                if banquet(&g).slowed.contains(&b.volley) {
                    break;
                }
            }
        }
        assert!(speed > 0.0 && speed < BOLT_SPEED * 0.9, "slow flies: {speed}");
        assert_eq!(g.brawl.as_ref().unwrap().bolts.len(), 5);
        assert!(g.brawl.as_ref().unwrap().bolts.iter().all(|b| b.look == BoltLook::Flies));
    }

    #[test]
    fn his_roll_runs_along_his_aim_and_the_wall_stops_it_without_a_daze() {
        let mut g = court();
        g.immortal = true;
        begin(&mut g, MoveId::BellyRoll, 4.0);
        let from = beelzebub(&mut g).pos;
        run(&mut g, 1.2);
        let d = beelzebub(&mut g);
        assert!(math::len(math::sub(d.pos, from)) > 2.0, "he rolled");
        assert!(!matches!(d.f.state, State::Reel { .. }), "no daze");
        assert!(matches!(d.f.state, State::Attack { mv: MoveId::BellyRoll, .. }) || d.f.is_neutral());
        let data = MoveId::BellyRoll.data();
        assert!(data.recovery >= 30 && data.phase(data.startup + data.active) == Phase::Recovery);
    }

    #[test]
    fn beelzebub_slain_drops_the_key_of_gluttony_and_the_way_on_opens() {
        let mut g = court();
        let from = beelzebub(&mut g).pos;
        beelzebub(&mut g).hp = max_hp_share(&mut g, 0.5);
        g.call_duel_hit(0, from);
        run(&mut g, 0.05);
        assert_eq!(swarms(&g), SWARMS as usize);
        beelzebub(&mut g).hp = 1.0;
        beelzebub(&mut g).f.state = State::Attack { mv: MoveId::Cleaver, frame: 60 };
        g.call_duel_hit(0, from);
        assert!(g.prince().is_none(), "he has fallen");
        assert!(g.court.as_ref().unwrap().slain);
        assert_eq!(swarms(&g), 0, "his swarms scatter with him");
        assert!(g.red_key.is_some(), "the Key of Gluttony lies on the floor");
        assert!(g.sins_cleared & Sin::Gluttony.bit() != 0);
        assert_eq!(g.key_name(), "THE KEY OF GLUTTONY");
        assert!(g.saved_run.as_ref().is_some_and(|r| r.stage == save::Stage::SinShop(Sin::Gluttony)), "the old man's next");
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

    fn max_hp_share(g: &mut Game, share: f64) -> f64 {
        beelzebub(g).max_hp * share
    }

    #[test]
    fn a_fall_brings_him_back_with_his_wounds_and_his_table_laid_and_continue_comes_back() {
        let mut g = hungry();
        until_eating(&mut g);
        run(&mut g, 2.0);
        assert_eq!(banquet(&g).dishes[0], Dish::Eaten);
        let max = beelzebub(&mut g).max_hp;
        beelzebub(&mut g).hp = max * 0.5;
        beelzebub(&mut g).breed.phase = 2;
        g.reset_round();
        let d = beelzebub(&mut g);
        assert_eq!((d.hp, d.breed.phase), (max * 0.5, 2), "his wounds kept (WARBAND.md W10)");
        assert!(!d.puppet && d.lore.vanish == 0);
        assert_eq!(banquet(&g).dishes, vec![Dish::Laden; PLATTERS_SET], "the table laid again");
        g.checkpoint(save::Stage::Court(Sin::Gluttony));
        let saved = g.saved_run.clone().unwrap();
        let mut h = Game::new();
        h.saved_run = Some(saved);
        h.continue_game();
        assert!(h.in_court() && h.prince().is_some_and(|d| d.hp == d.max_hp), "a game over gives him back his health");
        assert_eq!(h.level.data.name, "The Banquet of Flies");
    }

    /// A game in his court with his general's record as given.
    fn with_general(decree_done: bool, slain: bool) -> Game {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test-court"));
        let mut g = Game::new();
        let mut all: Records = g.general_records();
        all.set(Sin::Gluttony, Record { met: true, decree_done, slain, ..Default::default() });
        g.warband.save.generals = all.encode();
        g.map_jump(save::Stage::Court(Sin::Gluttony));
        g.enter(Screen::Playing);
        g.hero.free_move = true;
        g
    }

    #[test]
    fn his_general_slain_no_swarms_and_the_feast_laid_six_platters_and_the_swarms_at_once() {
        // The Steward slain: no swarms in his second phase.
        let mut g = with_general(false, true);
        assert!(beelzebub(&mut g).lore.summoned, "his phase-two adds are lost");
        let from = beelzebub(&mut g).pos;
        let max = beelzebub(&mut g).max_hp;
        beelzebub(&mut g).hp = max * 0.6;
        g.call_duel_hit(0, from);
        run(&mut g, 0.2);
        assert_eq!(beelzebub(&mut g).breed.phase, 2);
        assert_eq!(swarms(&g), 0, "no swarms");
        assert_eq!(banquet(&g).dishes.len(), PLATTERS_SET);
        // The feast laid: six platters, and the swarms at once.
        let mut g = with_general(true, false);
        run(&mut g, 0.1);
        assert_eq!(swarms(&g), SWARMS as usize, "the swarms come at once");
        assert_eq!(banquet(&g).dishes, vec![Dish::Laden; PLATTERS_LAID], "six platters");
        // And not again at his second phase.
        let from = beelzebub(&mut g).pos;
        let max = beelzebub(&mut g).max_hp;
        beelzebub(&mut g).hp = max * 0.6;
        g.call_duel_hit(0, from);
        run(&mut g, 0.2);
        assert_eq!(swarms(&g), SWARMS as usize);
    }

    /// PRINCES.md §0: Beelzebub in the Banquet of Flies, by the bots: with the kit a hero has on
    /// reaching him (tiers 1 and 2) the expert brings him down through all three phases without
    /// losing every life; a masher does not.
    #[test]
    fn the_expert_beats_beelzebub_in_his_court_and_a_masher_does_not() {
        use crate::game::bot::{court_fight, Loadout, Skill};
        use crate::logic::heroes::HeroKind;
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        for (tier, seed) in [(1, 1), (2, 2)] {
            let kit = (tier > 1).then(|| Loadout::all(HeroKind::Ragnar, tier));
            let f = court_fight(Sin::Gluttony, Skill::Expert, seed, kit, 200.0);
            assert!(f.slain && f.deaths < 3 && f.phase == 3, "tier {tier}: {f:?}");
            assert_eq!(f.stats.overlaps, 0, "{f:?}");
        }
        let f = court_fight(Sin::Gluttony, Skill::Novice, 1, Some(Loadout::all(HeroKind::Ragnar, 2)), 200.0);
        assert!(!f.slain && f.game_over, "{f:?}");
    }
}
