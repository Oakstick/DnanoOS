//! The foes: which stage haunts a level, the gym's garrison, witches and their minions,
//! reinforcements, the hunt's schedule, and what happens when a blow lands or a claw does.

use super::feedback::STREAK_SECONDS;
use super::{Game, BRUTAL_STREAK, MODE_SCHEDULE, WOUND_STAGGER};
use crate::audio::Sfx;
use crate::logic::enemy::{enemy_specs, Enemy, EnemyKind, EnemyMode, GlobalMode, REASSEMBLE_SECONDS, WITCH_CAST_EVERY, WITCH_CAST_RANGE};
use crate::logic::gear::{COINS_KILL, RUNE_DROP};
use crate::logic::hero::CONTACT_DAMAGE;
use crate::logic::heroes::HeroKind;
use crate::logic::progression::path;
use crate::logic::systems::{POINTS_KILL, PUDDLE_SECONDS};
use crate::logic::types::{dist_sq, Tile, CARDINALS, P};
use crate::logic::{gym, progression};

/// Seconds between reinforcements (the first comes after `SPAWN_FIRST`).
const SPAWN_EVERY: f64 = 25.0;

/// With every key taken the wait between reinforcements shrinks, down to this share at the last.
const SPAWN_EVERY_ENRAGED: f64 = 0.25;

/// The most foes that roam at once before any key is taken.
const MAX_ENEMIES: usize = 12;

/// Every key raises the cap on foes by this many.
const MAX_ENEMIES_PER_KEY: usize = 2;

/// The Garden's reinforcements, in turn.
const SPAWN_CYCLE: [EnemyKind; 4] = [EnemyKind::Bat, EnemyKind::Slime, EnemyKind::Vampire, EnemyKind::Skeleton];

/// The realms' reinforcements (the witches bring their own minions).
const SPAWN_CYCLE_TWO: [EnemyKind; 5] =
    [EnemyKind::LargeBat, EnemyKind::Helmeted, EnemyKind::Werewolf, EnemyKind::Witch, EnemyKind::Blemmyes];

/// Freeze-frame on a kill, the classic "impact" beat.
const HITSTOP_KILL: f64 = 0.05;

/// A shield's stagger (tier 7): how far and how long the foes around him reel.
const STAGGER_RADIUS: f64 = 2.5;

/// How long the foes around him reel from a shield's stagger.
const STAGGER_SECONDS: f64 = 2.0;

/// A foe this close (squared, in tiles) lays a claw on him.
const COLLISION_RADIUS_SQ: f64 = 0.45 * 0.45;

impl Game {
    /// Which cast haunts this level. For now only the Garden's: the second stage's foes train in
    /// the gym until the realms are built, and every level keeps the first stage's cast.
    pub(super) fn foe_stage(&self) -> u8 {
        self.foe_stage_of(self.dungeon)
    }

    pub(super) fn foe_stage_of(&self, _dungeon: usize) -> u8 {
        1
    }

    /// The gym's foes: one of every kind, stage by stage, each at its post.
    pub(super) fn gym_garrison(&self) -> Vec<Enemy> {
        gym::garrison()
            .into_iter()
            .map(|(kind, stage, post)| {
                let spec = enemy_specs(&self.level.grid, stage).into_iter().find(|s| s.kind == kind).unwrap();
                let mut e = Enemy::new(&spec, post, self.level.data.enemy_speed);
                e.post = Some(post);
                e.b.keep = Some(gym::platform(stage as usize - 1));
                e.b.mode = EnemyMode::Chase;
                e.prime();
                e
            })
            .collect()
    }

    /// In the gym a slain foe rises again at its post; a conjured minion, with no post, is gone for good.
    pub(super) fn update_gym(&mut self, dt: f64) {
        if !self.theme().is_gym() {
            return;
        }
        let _ = dt;
        let mut gone = false;
        for e in self.enemies.iter_mut() {
            if e.mode() != EnemyMode::Eaten {
                continue;
            }
            match e.post {
                Some(post) => e.rise_at(post, REASSEMBLE_SECONDS),
                None => gone = true,
            }
        }
        if gone {
            self.enemies.retain(|e| e.mode() != EnemyMode::Eaten);
            self.view.clear_enemies();
            for e in &self.enemies {
                self.view.add_enemy(e);
            }
        }
    }

    /// A witch on the hunt cackles now and then and conjures a minion at her feet.
    pub(super) fn update_witches(&mut self, dt: f64) {
        let hp = self.hero.pos();
        // In the vaults the packs are many: the witches may add a dozen minions between them.
        let vault = self.vault.is_some();
        let minions = self.enemies.iter().filter(|e| e.kind() == EnemyKind::Minion).count();
        let cap = if vault {
            if minions < 12 {
                usize::MAX
            } else {
                0
            }
        } else {
            self.max_enemies() + 2
        };
        for i in 0..self.enemies.len() {
            let e = &mut self.enemies[i];
            if e.kind() != EnemyKind::Witch || !e.is_dangerous() || e.lurking() || e.b.returning || e.duel.is_some() {
                continue;
            }
            e.b.cast -= dt;
            if e.b.cast > 0.0 || dist_sq(e.pos(), hp) > WITCH_CAST_RANGE * WITCH_CAST_RANGE {
                continue;
            }
            e.b.cast = WITCH_CAST_EVERY;
            if self.enemies.len() >= cap {
                continue;
            }
            let at = self.enemies[i].occupied();
            let spec = enemy_specs(&self.level.grid, 2).into_iter().find(|s| s.kind == EnemyKind::Minion).unwrap();
            let mut minion = Enemy::new(&spec, at, self.level.data.enemy_speed);
            minion.b.mode = EnemyMode::Chase;
            minion.b.keep = self.enemies[i].b.keep;
            minion.b.name = format!("{}'s minion", self.enemies[i].b.name);
            minion.prime();
            self.view.add_enemy(&minion);
            self.enemies.push(minion);
            self.audio.play(Sfx::Cackle);
            self.view.sparkle(&self.level.grid, at.x as f64, at.y as f64, 0.8, 0xb066ff, 24);
            self.floater(at.x as f64, at.y as f64 - 1.0, "HEE HEE HEE!", 1.4, Some(0xd08cff));
        }
    }

    /// Reinforcements crawl out of the hollow on a schedule, cycling through the cast.
    pub(super) fn update_spawns(&mut self, dt: f64) {
        if self.boss.is_some() || self.hub.is_some() || self.theme().is_gym() || self.vault.is_some() {
            return; // the arena belongs to the Cyclops; no foes come to the Portals of Hell, and the gym's and the vaults' are all there
        }
        self.spawn_timer -= dt;
        if self.spawn_timer > 0.0 || self.enemies.len() >= self.max_enemies() {
            return;
        }
        self.spawn_timer = self.spawn_every();
        let stage = self.foe_stage();
        let cycle: &[EnemyKind] = if stage == 1 { &SPAWN_CYCLE } else { &SPAWN_CYCLE_TWO };
        let kind = cycle[self.spawned % cycle.len()];
        let mut spec = enemy_specs(&self.level.grid, stage).into_iter().find(|s| s.kind == kind).unwrap();
        spec.exit_delay = 0.5;
        spec.name = format!("{} {}", spec.name, self.spawned / cycle.len() + 2);
        let graves = self.grave_spawns();
        let mut enemy = Enemy::new(&spec, self.level.nest_center, self.level.data.enemy_speed);
        let risen = if kind.reassembles() && !graves.is_empty() {
            // A draugr claws its way up out of a grave.
            let g = graves[((self.rng.next() * graves.len() as f64) as usize).min(graves.len() - 1)];
            enemy.rise_at(g, REASSEMBLE_SECONDS);
            Some(g)
        } else {
            None
        };
        self.view.add_enemy(&enemy);
        self.enemies.push(enemy);
        self.spawned += 1;
        self.audio.play(Sfx::Fuse);
        match risen {
            Some(g) => self.floater(g.x as f64, g.y as f64 - 0.6, "THE DEAD RISE", 1.6, Some(0xe9e9f2)),
            None => {
                let c = self.level.nest_center;
                self.floater(c.x as f64, c.y as f64 - 1.0, "SOMETHING STIRS", 1.6, Some(0xb06cff));
            }
        }
    }

    /// 0..1 how enraged the dungeon is: the share of its keys taken.
    pub fn rage(&self) -> f64 {
        if self.boss.is_some() || self.keys_total == 0 {
            0.0
        } else {
            self.keys_got as f64 / self.keys_total as f64
        }
    }

    /// Seconds between reinforcements, shrinking with every key.
    pub(super) fn spawn_every(&self) -> f64 {
        SPAWN_EVERY * (1.0 - (1.0 - SPAWN_EVERY_ENRAGED) * self.rage())
    }

    /// How many foes may roam at once, more with every key.
    pub(super) fn max_enemies(&self) -> usize {
        MAX_ENEMIES + MAX_ENEMIES_PER_KEY * self.keys_got as usize
    }

    /// Walkable tiles beside the churchyard's graves, where draugr rise (none without a churchyard).
    pub(super) fn grave_spawns(&self) -> Vec<P> {
        let Some(plan) = &self.garden else { return vec![] };
        let g = &self.level.grid;
        let mut out: Vec<P> = vec![];
        for p in &plan.graveyard {
            for d in CARDINALS {
                let n = g.neighbor(*p, d);
                if g.is_walkable(n.x, n.y, false) && !g.is_bridge(n.x, n.y) && g.get(n.x, n.y) == Tile::Open && !out.contains(&n) {
                    out.push(n);
                }
            }
        }
        out.sort_by_key(|p| (p.y, p.x));
        out
    }

    /// Moving slimes mark every tile they cross; standing in the slime slows the hero.
    pub(super) fn update_slime_trails(&mut self, dt: f64) {
        self.puddles.update(dt);
        for (i, e) in self.enemies.iter().enumerate() {
            if e.kind() != EnemyKind::Slime || matches!(e.mode(), EnemyMode::Nest | EnemyMode::Eaten | EnemyMode::Leaving) {
                continue;
            }
            let o = e.occupied();
            if self.trail_tile.get(&i) != Some(&o) {
                self.trail_tile.insert(i, o);
                if self.level.grid.get(o.x, o.y) != Tile::Nest {
                    self.puddles.add(o, PUDDLE_SECONDS);
                }
            }
        }
        if self.hero.alive && !self.hero.airborne() && self.puddles.has(self.hero.occupied()) {
            self.hero.slow();
        }
    }

    pub(super) fn update_mode_schedule(&mut self, dt: f64) {
        if self.frenzy.active() {
            return;
        }
        self.mode_timer -= dt;
        if self.global_mode != GlobalMode::Chase {
            // They hunt without a break: anything that set them scattering gives way to the hunt.
            self.global_mode = GlobalMode::Chase;
        }
        if self.mode_timer <= 0.0 && self.mode_index < MODE_SCHEDULE.len() - 1 {
            self.mode_index += 1;
            let (mode, secs) = MODE_SCHEDULE[self.mode_index];
            // Scatter spells shorten as the rage grows; hunts lengthen.
            self.mode_timer = if mode == GlobalMode::Scatter { secs * (1.0 - self.rage()) } else { secs * (1.0 + self.rage()) };
            self.global_mode = mode;
            for e in self.enemies.iter_mut() {
                if e.is_dangerous() {
                    e.reverse(&self.level.grid);
                }
            }
        }
    }

    /// A blow of the hero's lands on foe `i`: it falls, unless it is tough enough to take another,
    /// in which case it staggers (the loup-garou howls and comes on faster instead).
    pub(super) fn strike_enemy(&mut self, i: usize) {
        if self.enemies[i].duel.is_some() {
            self.stray_blow(i);
            return;
        }
        self.wake_pack_of(i);
        if self.enemies[i].hits_left() <= 1 {
            self.slay_enemy(i);
            return;
        }
        let p = self.enemies[i].pos();
        let kind = self.enemies[i].kind();
        self.enemies[i].wound();
        self.hitstop = self.hitstop.max(HITSTOP_KILL * 0.6);
        match kind {
            EnemyKind::Werewolf => {
                self.audio.play(Sfx::Howl);
                self.view.sparkle(&self.level.grid, p.x, p.y, 0.6, 0xff3a2a, 14);
                self.floater(p.x, p.y - 0.9, "IT HOWLS!", 1.4, Some(0xff6a4a));
            }
            EnemyKind::Helmeted => {
                self.audio.play(Sfx::Clang);
                self.view.spark(&self.level.grid, p.x, p.y, 0xffe0a0);
                self.floater(p.x, p.y - 0.9, "CLANG!", 1.2, Some(0xc8d0da));
                self.enemies[i].scare(&self.level.grid, WOUND_STAGGER);
            }
            EnemyKind::Witch => {
                self.audio.play(Sfx::Shriek);
                self.view.sparkle(&self.level.grid, p.x, p.y, 0.6, 0xb066ff, 14);
                self.enemies[i].scare(&self.level.grid, WOUND_STAGGER);
            }
            _ => {
                self.audio.play(Sfx::Growl);
                self.view.spark(&self.level.grid, p.x, p.y, 0xff8a6a);
                self.floater(p.x, p.y - 0.9, format!("{} MORE", self.enemies[i].hits_left()), 1.0, Some(0xffb090));
                self.enemies[i].scare(&self.level.grid, WOUND_STAGGER * 0.7);
            }
        }
    }

    pub(super) fn slay_enemy(&mut self, i: usize) {
        if self.enemies[i].duel.is_some() {
            self.stray_blow(i);
            return;
        }
        self.wake_pack_of(i);
        let base = if self.enemies[i].is_stunned() { self.frenzy.consume_value() } else { POINTS_KILL };
        self.earn(COINS_KILL);
        self.enemies[i].kill();
        let at = self.enemies[i].pos();
        if self.hero.path >= 8 && self.hero_kind == HeroKind::Ragnar {
            // Terrify: the lesser foes around the kill flee in blind panic.
            let grid = &self.level.grid;
            for (k, e) in self.enemies.iter_mut().enumerate() {
                if k != i && e.is_dangerous() && dist_sq(e.pos(), at) <= path::TERROR_RADIUS * path::TERROR_RADIUS {
                    e.scare(grid, path::TERROR_SECONDS);
                }
            }
        }
        if self.rng.next() < RUNE_DROP {
            self.progress.runes += 1;
            self.audio.play(Sfx::Milestone);
            self.view.sparkle(&self.level.grid, at.x, at.y, 0.8, 0x9ad1ff, 20);
            self.floater(at.x, at.y - 1.2, progression::gems(self.hero_kind, 1), 1.6, Some(0x9ad1ff));
        }
        self.kill_streak = if self.streak_timer > 0.0 { self.kill_streak + 1 } else { 1 };
        self.streak_timer = STREAK_SECONDS;
        self.audio.play_kill(self.kill_streak - 1);
        if self.kill_streak.is_multiple_of(BRUTAL_STREAK) {
            self.brutal();
        }
        let value = self.award(base);
        let p = self.enemies[i].pos();
        let color = self.enemies[i].b.color;
        self.view.spark(&self.level.grid, p.x, p.y, 0xffb0b0);
        self.view.dust(&self.level.grid, p.x, p.y, color, 14, 2.4);
        self.view.sparkle(&self.level.grid, p.x, p.y, 0.5, 0xfff1c0, 8);
        self.view.shake(0.1);
        self.hitstop = HITSTOP_KILL;
        let label = if self.double_points > 0.0 { format!("{value} x2") } else { value.to_string() };
        self.floater(p.x, p.y, label, 1.0, if self.double_points > 0.0 { Some(0xff5a3a) } else { None });
        if self.kill_streak >= 2 {
            let text = format!("x{} STREAK", self.kill_streak);
            self.floater(p.x, p.y - 0.6, text, 1.2, Some(0xff8a2a));
        }
    }

    /// Enemy contact damages the hero; a dead hero starts the dying sequence.
    pub(super) fn resolve_contact(&mut self) {
        if self.hero.invisible() || self.hero.airborne() {
            return; // they cannot find what they cannot see, or reach what sails overhead
        }
        if self.theme().is_gym() {
            return; // in the gym nothing lays a claw on him
        }
        let hp = self.hero.pos();
        let ho = self.hero.occupied();
        for i in 0..self.enemies.len() {
            let e = &self.enemies[i];
            if !e.is_dangerous() {
                continue;
            }
            let same_tile = e.occupied() == ho;
            if !same_tile && dist_sq(e.pos(), hp) > COLLISION_RADIUS_SQ {
                continue;
            }
            if self.hero.fx.fire > 0.0 {
                // Dragonfire: whatever lays a claw on him burns to ash.
                let p = self.enemies[i].pos();
                self.view.sparkle(&self.level.grid, p.x, p.y, 0.8, 0xff7a2a, 24);
                self.slay_enemy(i);
                continue;
            }
            let bite = self.enemies[i].kind().bite();
            if bite > 1 {
                self.view.shake(0.3);
            }
            let died = self.hurt_hero(CONTACT_DAMAGE * bite);
            if self.hero.blocked_hit {
                self.hero.blocked_hit = false;
                let ward = self.hero.shield.map(|s| progression::ward(s.tier));
                if let Some(w) = ward {
                    if w.bash && self.enemies[i].is_dangerous() {
                        // The rim strikes back: the foe whose blow it caught goes down.
                        self.slay_enemy(i);
                    }
                    if w.stagger {
                        self.stun_around(hp, STAGGER_RADIUS, STAGGER_SECONDS);
                    }
                }
            }
            if died {
                return;
            }
        }
    }
}
