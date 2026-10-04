//! Testing cheats and debug hooks: F1, F2 and 0 on the keyboard, and the capture tool's commands.

use super::{Game, Screen, CHEAT_COINS};
use crate::input::Action;
use crate::logic::boss::BossState;
use crate::logic::enemy::{enemy_specs, Enemy, EnemyKind, EnemyMode};
use crate::logic::gear::ShieldGear;
use crate::logic::hero::CONTACT_DAMAGE;
use crate::logic::progression;
use crate::logic::systems::PUDDLE_SECONDS;
use crate::logic::types::{Dir, PotionKind, Tile, P};

/// ...and the runes up to this.
const CHEAT_RUNES: u32 = 10;

impl Game {
    /// F2: every key found at once, and Ragnar at the gate (testing cheat).
    pub fn cheat_to_gate(&mut self) {
        if self.vault.is_some() {
            self.cheat_to_vault_exit();
            return;
        }
        if self.run.is_some() {
            self.cheat_run();
            return;
        }
        if self.pit.is_some() {
            self.cheat_pit();
            return;
        }
        let Some(g) = self.gate else { return };
        for (x, y, t) in self.level.grid.cells().collect::<Vec<_>>() {
            if t == Tile::Nodule {
                self.level.grid.set(x, y, Tile::Open);
                self.view.remove_collectible(x, y);
            }
        }
        self.keys_got = self.keys_total;
        self.unlock_gate();
        self.hero.m.teleport(P::new(g.x, g.y + 2));
        self.hero.face(Dir::Up);
        self.view.aim_along(Dir::Up);
    }

    /// F1: skip straight to the first Cyclops, from anywhere.
    pub fn cheat_to_boss(&mut self) {
        self.audio.unlock();
        self.tour = None;
        if matches!(self.screen, Screen::Title | Screen::GameOver) {
            self.score.reset();
            self.juice.score_shown = 0.0;
            self.new_kit();
        }
        self.load_boss(0);
        self.juice.fade = 1.0;
        self.enter(Screen::Ready);
        self.input.consume(Action::Attack);
    }

    /// 0: straight to the old man's wares, from anywhere, with silver and runes to test with.
    pub fn cheat_to_shop(&mut self) {
        self.audio.unlock();
        self.tour = None;
        if matches!(self.screen, Screen::Title | Screen::GameOver) {
            self.score.reset();
            self.juice.score_shown = 0.0;
            self.new_kit();
            self.load_level(0);
        }
        self.progress.silver = self.progress.silver.max(CHEAT_COINS);
        self.progress.runes = self.progress.runes.max(CHEAT_RUNES);
        self.open_shop();
        self.juice.fade = 1.0;
        self.input.consume(Action::Attack);
    }

    /// Test and capture hook: drop a nugget under Ragnar and pick it up.
    pub fn debug_collect_key(&mut self) {
        let o = self.hero.occupied();
        self.level.grid.set(o.x, o.y, Tile::Nodule);
        self.collect();
    }

    /// Test and capture hook: `amount` damage straight to the boss (or the prince in his court,
    /// through his hide: his phases change as they would).
    pub fn debug_wound_boss(&mut self, amount: i32) {
        if let Some(b) = self.boss.as_ref().filter(|b| b.active()) {
            let at = b.pos;
            self.wound_boss(amount, at);
        }
        if let Some(i) =
            self.prince().map(|d| d.id).and_then(|id| self.enemies.iter().position(|e| e.duel.as_ref().is_some_and(|d| d.id == id)))
        {
            let Some(d) = self.enemies[i].duel.as_mut() else { return };
            let from = d.pos;
            d.hp -= (amount as f64 - 1.0).max(0.0);
            let hit = crate::logic::combat::fsm::Hit { damage: 1.0 / d.taken(), ..Default::default() };
            self.duel_hit(i, hit, 0, from);
        }
    }

    /// Capture hook: the prince stands where he is, faces the hero and begins move `m` of his
    /// (Mammon's: 0 sceptre, 1 coin toss, 2 grasp, 3 coin volley, 4 coin rain, 5 hoard slam; any
    /// other prince's: his moves through his three phases' tables, in the order they first come).
    pub fn debug_prince_move(&mut self, m: usize) {
        use crate::logic::combat::{math, prince, MoveId};
        use crate::logic::enemy::EnemyKind;
        const MAMMON: [MoveId; 6] =
            [MoveId::Sceptre, MoveId::CoinToss, MoveId::Grasp, MoveId::CoinVolley, MoveId::CoinRain, MoveId::HoardSlam];
        let hp = self.hero.pos();
        let Some((id, kind)) = self.prince().map(|d| (d.id, d.breed.kind)) else { return };
        let moves: Vec<MoveId> = if kind == EnemyKind::Mammon {
            MAMMON.to_vec()
        } else {
            let mut all: Vec<MoveId> = Vec::new();
            for mv in (1..=3).flat_map(|phase| prince::table(kind, phase).iter().copied()) {
                if !all.contains(&mv) {
                    all.push(mv);
                }
            }
            all
        };
        let Some(&mv) = moves.get(m.min(moves.len().saturating_sub(1))) else { return };
        let Some(d) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == id) else { return };
        if mv.data().range.1 <= 2.0 {
            // (Within his reach for a blow: just north of him, as he stands before his throne.)
            d.pos = crate::logic::types::V::new(hp.x, hp.y - 1.9);
        }
        d.facing = math::norm(math::sub(hp, d.pos));
        d.f.state = crate::logic::combat::State::Neutral;
        d.rest = u16::MAX;
        d.token = true;
        d.begin(mv, hp);
    }

    /// Capture hook: three quick kills, for a BRUTAL streak.
    pub fn debug_streak(&mut self) {
        for i in 0..self.enemies.len().min(3) {
            self.slay_enemy(i);
        }
    }

    /// Capture hook: own everything up to `tier` in every tree, and take tier `tier` weapon and shield.
    pub fn debug_arm(&mut self, tier: u8) {
        let tier = tier.clamp(1, progression::TIERS);
        let h = self.progress.hero_mut(self.hero_kind);
        h.owned = [tier; 4];
        h.wield = tier;
        h.carry = tier;
        self.hero.shield = Some(ShieldGear::new(tier));
        self.equip_from_progress();
    }

    /// Capture hook: drink a potion without owning it.
    pub fn debug_drink(&mut self, kind: PotionKind) {
        self.quaff(kind);
    }

    /// Capture hook: knock the boss down as if a dive had missed.
    pub fn debug_daze_boss(&mut self) {
        if let Some(b) = self.boss.as_mut().filter(|b| b.active()) {
            b.state = BossState::Dazed(0.0);
            b.hover = 0.05;
        }
    }

    /// Test and capture hook: an enemy's touch.
    /// Capture hook: straight into dungeon `index` (0 is the first) with a fresh kit, after a countdown.
    pub fn debug_dungeon(&mut self, index: usize) {
        self.score.reset();
        self.new_kit();
        self.load_level(index);
        self.enter(Screen::Ready);
    }

    /// Capture hook: `rows` rows of one of each foe of `stage` (none: this level's) standing a few
    /// tiles before the hero, facing him.
    pub fn debug_foes(&mut self, rows: usize, stage: Option<u8>) {
        let stage = stage.unwrap_or_else(|| self.foe_stage());
        let specs = enemy_specs(&self.level.grid, stage);
        let kinds = EnemyKind::of_stage(stage);
        let n = kinds.len();
        let p = self.hero.occupied();
        // (Giants want room: twice the spacing.)
        let gap = if kinds.iter().any(|&k| crate::logic::combat::Breed::of(k).radius > 0.7) { 4 } else { 2 };
        self.enemies.clear();
        self.view.clear_enemies();
        for (i, kind) in kinds.repeat(rows.max(1)).into_iter().enumerate() {
            let Some(spec) = specs.iter().find(|s| s.kind == kind) else { continue };
            let (col, row) = ((i % n) as i32, (i / n) as i32);
            let at = P::new(p.x - (n as i32 - 1) * gap / 2 + col * gap, p.y - gap - row * gap);
            let mut e = Enemy::new(spec, at, self.level.data.enemy_speed);
            e.b.mode = EnemyMode::Chase;
            e.m.dir = Dir::Down;
            self.view.add_enemy(&e);
            self.enemies.push(e);
        }
    }

    /// A fresh slime trail along the tiles around Ragnar, for screenshots.
    pub fn debug_goo(&mut self) {
        let p = self.hero.occupied();
        let g = &self.level.grid;
        let mut tiles: Vec<P> = vec![];
        for (dx, dy) in [(0, 0), (1, 0), (2, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1)] {
            let t = P::new(p.x + dx, p.y + dy);
            if g.is_walkable(t.x, t.y, false) {
                tiles.push(t);
            }
        }
        for (i, t) in tiles.into_iter().enumerate() {
            self.puddles.add(t, PUDDLE_SECONDS * (1.0 - i as f64 * 0.08));
        }
    }

    pub fn debug_hurt(&mut self) {
        self.hero.invuln = 0.0;
        self.hurt_hero(CONTACT_DAMAGE);
    }
}
