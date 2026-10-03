//! Loading a level and starting a round: the level itself, the hero's kit, the foes at their
//! places, the hidden relic shrine, and where he starts.

use super::{save, Game, MODE_SCHEDULE, SPAWN_FIRST};
use crate::audio::Soundtrack;
use crate::logic::boss::{Cyclops, CYCLOPS_HP, CYCLOPS_HP_STEP};
use crate::logic::enemy::Enemy;
use crate::logic::garden;
use crate::logic::gear::ShieldGear;
use crate::logic::hero::Hero;
use crate::logic::items::{Diamond, DiamondKind, Potion, WeaponKind, BOMBS_PER_LIFE};
use crate::logic::level::{parse_level, LevelData, ParsedLevel};
use crate::logic::levels::{boss_arena, level_for, ARENA};
use crate::logic::types::{manhattan, Dir, PotionKind, Tile, CARDINALS, P, V};
use crate::platform;
use std::collections::HashMap;

/**
 * The open floor tile nearest the dungeon's center (never a shrine, bottle,
 * ramp or the hollow, nor the middle row or column, where the fountain's
 * channels run), and the direction with the longest clear run from it.
 */
pub fn center_spawn(level: &ParsedLevel) -> (P, Dir) {
    let g = &level.grid;
    let (cx, cy) = ((g.width - 1) as f64 / 2.0, (g.height - 1) as f64 / 2.0);
    let taken: Vec<P> =
        level.angel_shrines.iter().chain(level.item_shrines.iter()).copied().chain(level.potions.iter().map(|(p, _)| *p)).collect();
    let mut best = level.hero_spawn;
    let mut best_d = f64::INFINITY;
    for (x, y, t) in g.cells() {
        let p = P::new(x, y);
        if t != Tile::Open || g.is_ramp(x, y) || taken.contains(&p) || y == (g.height - 1) / 2 || x == (g.width - 1) / 2 {
            continue;
        }
        let d = (x as f64 - cx).powi(2) + (y as f64 - cy).powi(2);
        if d < best_d {
            best_d = d;
            best = p;
        }
    }
    let run = |d: Dir| {
        let mut n = 0;
        let mut p = best;
        while n < 12 && g.can_move(p, d, false, false) {
            p = g.neighbor(p, d);
            n += 1;
        }
        n
    };
    let facing = [Dir::Down, Dir::Up, Dir::Left, Dir::Right].into_iter().max_by_key(|&d| (run(d), d == Dir::Down)).unwrap();
    (best, facing)
}

impl Game {
    /// Dungeon `index`: the labyrinth with its 25 nuggets.
    pub fn load_level(&mut self, index: usize) {
        self.load(level_for(index), index);
    }

    /// The Cyclops' arena that follows dungeon `index`.
    pub fn load_boss(&mut self, index: usize) {
        self.load(boss_arena(index), index);
    }

    pub(super) fn load(&mut self, data: LevelData, index: usize) {
        let started = web_time::Instant::now();
        self.load_inner(data, index);
        self.begin_loading(self.level.data.name.clone(), started);
    }

    fn load_inner(&mut self, data: LevelData, index: usize) {
        self.score.level = index;
        self.dungeon = index;
        // Every level begins with the patron's power ready.
        self.patron_wait = 0.0;
        self.level = parse_level(&data);
        self.vault = None;
        self.run = None;
        if let Some(mode) = self.spar.as_mut().and_then(|s| s.camera_before.take()) {
            self.view.camera_mode = mode;
        }
        self.spar = data.theme.is_gym().then(super::sparring::Sparring::new);
        // (The vaults raise their own brawl as their packs are laid out.)
        self.brawl = data.theme.is_gym().then(|| super::brawl::Brawl::new(super::brawl::Field::Ring));
        let vault_level = data.theme.is_vault();
        // One of the six realms after the vaults (`game::realms`).
        let run_sin = match data.theme {
            crate::logic::level::Theme::Realm(sin) if !vault_level => Some(sin),
            _ => None,
        };
        let run_level = run_sin.is_some();
        if !vault_level && !run_level {
            if let Some(mode) = self.camera_before_vault.take() {
                self.view.camera_mode = mode;
            }
        }
        if self.level.data.boss {
            // In the arena he starts at the south edge, facing the Cyclops.
            self.start_facing = Dir::Up;
        } else if self.theme().is_hub() {
            // In the Portals of Hell he comes in from the cliffs to the south, facing the volcano.
            self.start_facing = Dir::Up;
        } else if self.theme().is_gym() {
            // In the gym he arrives at the west end, facing the foes.
            self.start_facing = Dir::Right;
        } else if vault_level || run_level {
            // In the Vaults of Mammon he comes through the west portal, facing into the antechamber
            // (and in the realms after them through their own portal: `enter_run` turns him).
            self.start_facing = Dir::Right;
        } else {
            // Ragnar starts in the heart of the dungeon, facing its longest corridor.
            let (spawn, facing) = center_spawn(&self.level);
            self.level.hero_spawn = spawn;
            self.start_facing = facing;
        }
        // Dungeon I is a landscape garden: a fountain and its bridged channels, a churchyard, temples and flower beds.
        self.garden = None;
        if self.level.data.boss {
            // The arena's stream runs in a stone channel under stone bridges too.
            self.garden = garden::stream_plan(&self.level.grid);
            if let Some(plan) = &self.garden {
                for b in &plan.bridges {
                    self.level.grid.set_bridge(b.x, b.y, true);
                }
            }
        } else if index == 0 {
            let l = &self.level;
            let mut keep = vec![l.hero_spawn, l.enemy_spawn];
            keep.extend(l.angel_shrines.iter().chain(&l.item_shrines).copied());
            keep.extend(l.potions.iter().map(|(p, _)| *p));
            keep.extend(l.gate);
            self.garden = garden::plan(&l.grid, &keep);
            if let Some(plan) = &self.garden {
                garden::apply(&mut self.level.grid, plan);
            }
        }
        // The estate's gate takes its golden keys; the arena's, the one red key the Cyclops drops.
        self.keys_total = if self.level.data.boss { 1 } else { self.level.grid.count(Tile::Nodule) as u32 };
        self.keys_got = 0;
        self.gate = self.level.gate;
        self.gate_open = false;
        self.door = 0.0;
        self.door_opening = false;
        self.gate_hint = 0.0;
        self.audio.set_tempo_step(0);
        // Its song: dungeon I's own, or one drawn at random (not the last level's) from the bosses'
        // or the other levels' songs.
        self.level_track = Some(if self.level.data.boss {
            self.audio.pick_track(&Soundtrack::BOSSES, self.level_track)
        } else if index == 0 && self.theme() == crate::logic::level::Theme::Estate {
            Soundtrack::Level1
        } else {
            self.audio.pick_track(&Soundtrack::LEVELS, self.level_track)
        });

        // Those waiting in the hollow stand along its middle row: at either end and in the middle.
        let c = self.level.nest_center;
        let g = &self.level.grid;
        let row: Vec<P> =
            (0..g.width).map(|x| P::new(x, c.y)).filter(|p| g.get(p.x, p.y) == Tile::Nest && (p.x - c.x).abs() <= 3).collect();
        let (first, last) = (row.first().copied().unwrap_or(c), row.last().copied().unwrap_or(c));
        self.nest_slots = vec![self.level.enemy_spawn, first, c, last];

        // Clearing a dungeon keeps the held weapon and its ammo; only death takes them.
        let carried = if self.hero.alive && self.hero.holding_pickup() { Some((self.hero.weapon, self.hero.ammo)) } else { None };
        // His shield, dents and all, goes with him, and so does the way he moves (free or on the grid).
        let (shield, free_move) = (self.hero.shield, self.hero.free_move);
        // Each hero runs at his own pace: Miyamoto quicker than Ragnar, Mehmed slower.
        self.hero = Hero::new(self.level.hero_spawn, self.level.data.hero_speed * self.hero_kind.speed());
        self.hero.shield = shield;
        self.hero.free_move = free_move;
        self.equip_from_progress();
        // No foes in a boss arena (but the boss), nor in the Portals of Hell; the gym's stand at their posts.
        let hell = self.theme().is_hub();
        let gym = self.theme().is_gym();
        let specs = if self.level.data.boss || hell || gym || vault_level || run_level { Vec::new() } else { self.dungeon_cast(index) };
        self.enemies = specs
            .iter()
            .enumerate()
            .map(|(i, s)| Enemy::new(s, self.nest_slots.get(i).copied().unwrap_or(c), self.level.data.enemy_speed))
            .collect();
        if gym {
            self.enemies = self.gym_garrison();
        }
        if vault_level {
            self.enter_vault();
        }
        if let Some(sin) = run_sin {
            self.enter_run(sin);
        }
        self.boss = if self.level.data.boss {
            let (x0, y0, x1, y1) = ARENA;
            let hp = CYCLOPS_HP + CYCLOPS_HP_STEP * index as i32;
            let min = V::new(x0 as f64 + 1.0, y0 as f64 + 0.5);
            let max = V::new(x1 as f64 - 1.0, y1 as f64 - 0.5);
            Some(Cyclops::new(self.level.enemy_spawn.v(), min, max, hp, self.rng.index(1 << 30) as u32))
        } else {
            None
        };
        self.boss_shots.clear();
        self.red_key = None;
        self.angels_used = [false; 4];
        self.diamonds = self
            .level
            .angel_shrines
            .iter()
            .enumerate()
            .map(|(i, &t)| Diamond::new(DiamondKind::Angel, t, i))
            .chain(self.level.item_shrines.iter().enumerate().map(|(i, &t)| Diamond::new(DiamondKind::Item, t, i)))
            .collect();
        self.potions = self.level.potions.iter().map(|&(t, k)| Potion::new(t, k)).collect();
        if self.level.data.boss {
            // No bottles in a boss arena: bring your own.
            self.potions.clear();
        }
        if index >= 1 {
            // From the second dungeon on, gear only comes from the old gatekeeper;
            // the archangels' shrines stand in every dungeon.
            self.potions.clear();
            self.diamonds.retain(|d| d.kind == DiamondKind::Angel);
        }
        self.boss_bleed = 0.0;
        self.decoy = None;
        self.decoy_cooldown = 0.0;
        if !self.level.data.boss {
            // The Valkyrie and the World Tree answer once in each dungeon (its Cyclops included).
            self.revived = false;
            self.restored = false;
            self.stood = false;
        }
        self.poison = (0, 0.0, 0.0);
        self.fires.clear();
        self.spirits.clear();
        self.relic_shrine = if self.level.data.boss || hell || gym || vault_level || run_level { None } else { self.hidden_shrine_spot() };
        self.spawn_timer = SPAWN_FIRST;
        self.spawned = 0;
        if let Some(g) = self.level.gate {
            for dx in -1..=1 {
                self.level.grid.set_landmark(g.x + dx, g.y, true);
            }
        }
        self.view.build_level(
            &self.level.grid,
            &self.enemies,
            &self.diamonds,
            self.theme(),
            self.garden.as_ref(),
            self.level.gate,
            self.vault.as_ref(),
            self.run.as_ref(),
        );
        self.hub = None;
        self.realm = self.theme().sin();
        if hell {
            self.open_hub();
        }
        self.juice.boss_trail = self.boss.as_ref().map(|b| b.hp as f32).unwrap_or(0.0);
        self.reset_round();
        if let Some((WeaponKind::Ranged(w), ammo)) = carried.filter(|_| index == 0) {
            self.hero.equip(w);
            self.hero.ammo = ammo;
        }
    }

    /// Puts everyone back at spawn without touching collected gold.
    pub(super) fn reset_round(&mut self) {
        // In a realm he rises where its rules have carried his checkpoint.
        if let Some(r) = &self.run {
            self.level.hero_spawn = r.base().checkpoint;
        }
        self.hero.respawn(self.level.hero_spawn);
        // On his tile's own height from the first frame: the start can be raised (the fountain's terrace),
        // and until he moves nothing else lifts him out of the stone.
        let spawn = self.level.hero_spawn;
        self.hero.altitude = self.level.grid.elevation(spawn.x, spawn.y) as f64;
        self.hero.face(self.start_facing);
        self.view.aim_along(self.hero.facing);
        self.juice.coins.clear();
        self.juice.banner = None;
        self.juice.hp_trail = self.hero.hp as f32;
        self.hitstop = 0.0;
        self.kill_streak = 0;
        self.double_points = 0.0;
        self.was_airborne = false;
        let center = self.level.nest_center;
        let graves = self.grave_spawns();
        // Under the frame data the fight begins afresh: the vaults' packs back asleep at their posts, whole.
        let kit = self.hero_kit();
        if let Some(b) = self.brawl.as_mut() {
            b.respawn(kit);
        }
        for (i, e) in self.enemies.iter_mut().enumerate() {
            if let Some(d) = e.duel.as_deref_mut() {
                d.reset_home();
                continue;
            }
            match (e.post, graves.get(i % graves.len().max(1))) {
                // The vaults' packs go back to idling in their rooms; the gym's foes stand whole at their posts.
                (Some(_), _) if e.b.den.is_some() => Self::settle(e, e.b.den.unwrap()),
                (Some(post), _) => e.stand_at(post),
                // Draugr wait in the churchyard rather than the hollow, standing among the graves
                // (those that come later climb out of them).
                (None, Some(&g)) if e.kind().reassembles() => e.stand_at(g),
                _ => e.reset(self.nest_slots.get(i).copied().unwrap_or(center)),
            }
        }
        self.mode_index = 0;
        self.mode_timer = MODE_SCHEDULE[0].1;
        self.global_mode = MODE_SCHEDULE[0].0;
        self.frenzy.cancel();
        self.audio.set_frenzy(false);
        self.floaters.clear();
        self.puddles.clear();
        self.trail_tile.clear();
        self.input.clear_direction();
        self.cam_dir = Dir::None;
        self.bombs.clear();
        self.bombs_left = BOMBS_PER_LIFE;
        self.projectiles.clear();
        // The Cyclops flies back up; it keeps the wounds it has taken.
        let home = self.level.enemy_spawn.v();
        if let Some(b) = &mut self.boss {
            b.reset(home);
        }
        self.boss_shots.clear();
        self.reset_run();
    }

    pub(super) fn start_game(&mut self) {
        self.score.reset();
        self.blessing = None;
        self.uriel_rest = 0.0;
        self.holy_wave = None;
        self.new_kit();
        self.hero.respawn(self.level.hero_spawn); // a new game starts with the bare axe
        self.load_level(0);
        self.juice.score_shown = 0.0;
        self.checkpoint(save::Stage::Dungeon(0));
        // The camera first, then the tour (the first time) or the countdown.
        self.ask_view(self.tour_pending);
    }

    /// A fresh adventure: the weapon and shield he chose at the old man's, one health potion.
    /// Silver, runes and what is unlocked are his from adventure to adventure.
    pub(super) fn new_kit(&mut self) {
        self.belt = vec![PotionKind::Health];
        self.belt_index = 0;
        let h = self.progress.hero(self.hero_kind);
        let carry = h.carry;
        // A shield that broke stays broken until the old man reforges it.
        self.hero.shield = (!h.is_broken(carry)).then(|| ShieldGear::new(carry));
        self.equip_from_progress();
    }

    /// Puts the chosen weapon in his hand and his path's skills in his heart.
    pub fn equip_from_progress(&mut self) {
        let h = *self.progress.hero(self.hero_kind);
        self.hero.kind = self.hero_kind;
        self.hero.arm(h.wield);
        self.hero.path = h.path();
        self.hero.max_hp = self.hero.health_cap();
        self.hero.edge = h.sharpness(h.wield);
        self.hero.outfit = h.outfit;
    }

    /// Writes the silver, the runes and the unlocks to the save (not in tests).
    pub(super) fn save_progress(&self) {
        if !cfg!(test) {
            platform::save("vh.progress", &self.progress.encode());
        }
    }

    /**
     * Where this dungeon hides its shrine: the dead end farthest (by walking)
     * from where he starts, clear of keys, shrines, bottles and the gate.
     */
    pub(super) fn hidden_shrine_spot(&self) -> Option<P> {
        let g = &self.level.grid;
        let start = self.level.hero_spawn;
        let mut dist: HashMap<P, u32> = HashMap::new();
        let mut queue = std::collections::VecDeque::new();
        dist.insert(start, 0);
        queue.push_back(start);
        while let Some(t) = queue.pop_front() {
            let d = dist[&t];
            for dir in CARDINALS {
                if !g.can_move(t, dir, false, false) {
                    continue;
                }
                let n = g.neighbor(t, dir);
                if let std::collections::hash_map::Entry::Vacant(e) = dist.entry(n) {
                    e.insert(d + 1);
                    queue.push_back(n);
                }
            }
        }
        let taken: Vec<P> = self
            .level
            .angel_shrines
            .iter()
            .chain(&self.level.item_shrines)
            .copied()
            .chain(self.level.potions.iter().map(|(p, _)| *p))
            .chain(self.level.gate)
            .chain([self.level.enemy_spawn, self.level.nest_center])
            .collect();
        let mut best: Option<(u32, P)> = None;
        for (&t, &d) in &dist {
            if g.get(t.x, t.y) != Tile::Open || g.is_bridge(t.x, t.y) || taken.iter().any(|q| manhattan(q.v(), t.v()) < 2.0) {
                continue;
            }
            let exits = CARDINALS.iter().filter(|&&dir| g.can_move(t, dir, false, false)).count();
            // Dead ends first; ties go to the far one, then the lower-left, so it is always the same spot.
            let score = d + if exits == 1 { 1000 } else { 0 };
            if best.map(|(s, b)| score > s || (score == s && (t.y, t.x) > (b.y, b.x))).unwrap_or(true) {
                best = Some((score, t));
            }
        }
        best.map(|(_, t)| t)
    }
}
