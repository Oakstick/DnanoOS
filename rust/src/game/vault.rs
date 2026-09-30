//! The Vaults of Mammon in play (the layout is `logic::vault`). Through
//! Greed's portal he arrives in the west antechamber; the packs idle in their
//! rooms until he comes near, then hunt him as long as he stays close, and go
//! back to their rooms when he is gone, as the packs of Diablo do; what falls
//! stays down. Every room is strewn with gold he can take: coins, heaps of
//! them, chests and gems, all counted into his hoard. At the far portal
//! Mammon takes his toll: drop everything and the portal opens on his court;
//! keep it and the vaults close, and he starts the realm again with every
//! coin back where it lay. F2 brings him straight to the far portal.

use super::save::Stage;
use super::{Banner, Game, MenuItem, Screen};
use crate::audio::Sfx;
use crate::input::Action;
use crate::logic::enemy::{enemy_specs, Enemy, EnemyMode};
use crate::logic::hub::Rect;
use crate::logic::sins::Sin;
use crate::logic::types::{dist_sq, Dir, P, V};
use crate::logic::vault::{LootKind, Vault, LEASH, SEED, WAKE_RANGE};
use crate::render::view::CameraMode;

/// What he carries out of the vaults: its worth, and what it is made of.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Hoard {
    pub gold: u32,
    pub coins: u32,
    pub piles: u32,
    pub chests: u32,
    pub gems: u32,
}

impl Hoard {
    pub fn is_empty(&self) -> bool {
        self.gold == 0
    }
}

impl Game {
    /// In the Vaults of Mammon.
    pub fn in_vault(&self) -> bool {
        self.vault.is_some()
    }

    /// Lays out the vault as this level is loaded: its packs at their posts, its gold where it lies.
    pub(super) fn enter_vault(&mut self) {
        let vault = Vault::generate(SEED);
        // The vaults are seen from above, as Diablo's are; the camera goes back to what it was on leaving.
        if self.camera_before_vault.is_none() {
            self.camera_before_vault = Some(self.view.camera_mode);
        }
        self.view.camera_mode = CameraMode::Top;
        self.start_facing = Dir::Right;
        self.level.hero_spawn = vault.arrival;
        self.hoard = Hoard::default();
        self.toll_asked = false;
        self.vault = Some(vault);
        self.enemies = self.vault_garrison();
    }

    /// The packs: every foe at its post in its room, idling there.
    fn vault_garrison(&self) -> Vec<Enemy> {
        let Some(v) = &self.vault else { return Vec::new() };
        let specs = [enemy_specs(&self.level.grid, 1), enemy_specs(&self.level.grid, 2)];
        let mut out = Vec::new();
        for pack in &v.packs {
            for (k, &(kind, stage, post)) in pack.foes.iter().enumerate() {
                let mut spec = specs[stage as usize - 1].iter().find(|s| s.kind == kind).unwrap().clone();
                spec.name = format!("{} {}", spec.name, out.len() + 1);
                let _ = k;
                let mut e = Enemy::new(&spec, post, self.level.data.enemy_speed);
                e.post = Some(post);
                Self::settle(&mut e, pack.den);
                out.push(e);
            }
        }
        out
    }

    /// Puts a foe at its post, idling in its den.
    pub(super) fn settle(e: &mut Enemy, den: Rect) {
        if let Some(post) = e.post {
            e.m.teleport(post);
        }
        e.b.den = Some(den);
        e.b.keep = Some(den);
        e.b.returning = false;
        e.b.mode = EnemyMode::Chase;
        e.prime();
    }

    /// The vault's rules each tick: packs wake and give up, gold is taken, the fallen are cleared away.
    pub(super) fn update_vault(&mut self, _dt: f64) {
        if self.vault.is_none() {
            return;
        }
        let hp = self.hero.pos();
        let ho = self.hero.occupied();
        // Packs wake together: when he steps into their room, or comes close to any one of them.
        let mut stirred: Vec<Rect> = Vec::new();
        for e in &self.enemies {
            if !e.lurking() || !e.is_dangerous() {
                continue;
            }
            let den = e.b.den.unwrap();
            if (den.contains(ho) || dist_sq(e.pos(), hp) < WAKE_RANGE * WAKE_RANGE) && !stirred.contains(&den) {
                stirred.push(den);
            }
        }
        for den in &stirred {
            let (cx, cy) = den.center();
            self.floater(cx as f64, cy as f64, "THE PACK STIRS", 1.4, Some(0xffc85a));
            self.audio.play(Sfx::Growl);
        }
        for e in self.enemies.iter_mut() {
            if e.lurking() && stirred.iter().any(|d| Some(*d) == e.b.den) {
                e.rouse();
            }
        }
        // The hunt is given up when he is far from the room; taken up again if he comes back.
        for e in self.enemies.iter_mut() {
            let Some(den) = e.b.den else { continue };
            if e.b.keep.is_some() {
                continue;
            }
            let (cx, cy) = den.center();
            let far = dist_sq(V::new(cx as f64, cy as f64), hp) > LEASH * LEASH;
            if far && !e.b.returning {
                e.go_home();
            } else if e.b.returning && dist_sq(e.pos(), hp) < WAKE_RANGE * WAKE_RANGE {
                e.rouse();
            }
        }
        // What falls in the vaults stays down: a draugr lies in its pieces a while, and is then gone too.
        for e in self.enemies.iter_mut() {
            if e.mode() == EnemyMode::Scattered && e.b.scatter_timer < 0.4 {
                e.b.mode = EnemyMode::Eaten;
            }
        }
        if self.enemies.iter().any(|e| e.mode() == EnemyMode::Eaten) {
            self.enemies.retain(|e| e.mode() != EnemyMode::Eaten);
            self.view.clear_enemies();
            for e in &self.enemies {
                self.view.add_enemy(e);
            }
        }
        self.take_loot();
    }

    /// A blow lands on one of a pack: the whole pack wakes.
    pub(super) fn wake_pack_of(&mut self, i: usize) {
        let Some(den) = self.enemies.get(i).and_then(|e| e.b.den) else { return };
        for e in self.enemies.iter_mut() {
            if e.b.den == Some(den) && e.lurking() {
                e.rouse();
            }
        }
    }

    /// Gold underfoot goes into the hoard.
    fn take_loot(&mut self) {
        if self.hero.airborne() {
            return;
        }
        let o = self.hero.occupied();
        let Some(v) = &mut self.vault else { return };
        let Some(i) = v.loot.iter().position(|l| !l.taken && l.at == o) else { return };
        v.loot[i].taken = true;
        let kind = v.loot[i].kind;
        let worth = kind.worth();
        self.hoard.gold += worth;
        let (ox, oy) = (o.x as f64, o.y as f64);
        let grid = &self.level.grid;
        let text = match kind {
            LootKind::Coin => {
                self.hoard.coins += 1;
                self.audio.play(Sfx::Nodule);
                self.view.sparkle(grid, ox, oy, 0.3, 0xffd860, 6);
                format!("+{worth}")
            }
            LootKind::Pile => {
                self.hoard.piles += 1;
                self.audio.play(Sfx::Charge);
                self.view.sparkle(grid, ox, oy, 0.4, 0xffd860, 14);
                format!("A HEAP OF GOLD +{worth}")
            }
            LootKind::Chest => {
                self.hoard.chests += 1;
                self.audio.play(Sfx::Anvil);
                self.audio.play_reward(self.hoard.chests * 3);
                self.view.shake(0.2);
                self.view.sparkle(grid, ox, oy, 0.7, 0xffd860, 40);
                self.view.spark(grid, ox, oy, 0xffe890);
                format!("A CHEST OF GOLD +{worth}")
            }
            LootKind::Gem(c) => {
                self.hoard.gems += 1;
                self.audio.play(Sfx::Milestone);
                self.view.sparkle(grid, ox, oy, 0.8, c, 30);
                let name = match c {
                    0xe0203a => "A RUBY",
                    0x3a6aff => "A SAPPHIRE",
                    _ => "AN EMERALD",
                };
                format!("{name} +{worth}")
            }
        };
        self.juice.coin_punch = 1.0;
        self.floater(ox, oy - 0.4, text, 1.1, Some(if let LootKind::Gem(c) = kind { c } else { 0xffd860 }));
    }

    /// Before the far portal: Mammon's toll. Drop everything, or keep it.
    pub(super) fn ask_toll(&mut self) {
        self.toll_asked = true;
        self.audio.play(Sfx::Toll);
        self.enter(Screen::Toll);
        self.menu_index = 0;
        self.input.consume(Action::Confirm);
        self.input.consume(Action::Jump);
        self.input.consume(Action::Attack);
    }

    /// His answer. Drop: the hoard is left where he stands and the portal opens. Keep: the vaults
    /// close on him, and the realm begins again with every coin back where it lay.
    pub(super) fn pay_toll(&mut self, choice: MenuItem) {
        let hp = self.hero.pos();
        match choice {
            MenuItem::DropAll => {
                let was = self.hoard;
                self.hoard = Hoard::default();
                self.gate_open = true;
                self.audio.play(Sfx::Win);
                self.view.sparkle(&self.level.grid, hp.x, hp.y, 0.8, 0xffd860, 60);
                self.view.dust(&self.level.grid, hp.x, hp.y, 0xf2c53d, 24, 2.0);
                self.juice.banner = Some(Banner {
                    text: "THE HOARD LIES WHERE IT FELL".into(),
                    sub: if was.is_empty() {
                        "EMPTY-HANDED HE CAME, AND EMPTY-HANDED HE GOES THROUGH".into()
                    } else {
                        format!("{} GOLD LEFT BEHIND   THE PORTAL TO MAMMON'S COURT OPENS", was.gold)
                    },
                    color: 0xf2c53d,
                    life: 3.5,
                    max: 3.5,
                });
                self.enter(Screen::Playing);
            }
            _ => {
                self.audio.play(Sfx::Denied);
                self.audio.play(Sfx::DoorGrind);
                self.view.shake(0.6);
                self.load_realm(Sin::Greed);
                self.checkpoint(Stage::Realm(Sin::Greed));
                self.juice.fade = 1.0;
                self.juice.banner = Some(Banner {
                    text: "THE VAULTS CLOSE ON THE GREEDY".into(),
                    sub: "EVERY COIN IS BACK WHERE IT LAY   FROM THE BEGINNING".into(),
                    color: 0xff8a5a,
                    life: 4.0,
                    max: 4.0,
                });
                self.enter(Screen::Ready);
            }
        }
    }

    /// F2 in the vaults: straight to the far portal, where the toll is asked.
    pub(super) fn cheat_to_vault_exit(&mut self) {
        let Some(v) = &self.vault else { return };
        let gate = v.exit_gate;
        self.hero.place(P::new(gate.x - 1, gate.y));
        self.hero.face(Dir::Right);
        self.view.aim_along(Dir::Right);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{save, Screen, CLEARED_SECONDS, READY_SECONDS};
    use crate::input::Action;
    use crate::logic::vault::RoomKind;
    const STEP: f64 = crate::app::STEP;

    fn in_the_vault() -> Game {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        let mut g = Game::new();
        g.map_jump(save::Stage::Realm(Sin::Greed));
        for _ in 0..((READY_SECONDS + 0.1) / STEP) as usize {
            g.update(STEP);
        }
        assert_eq!(g.screen, Screen::Playing);
        g
    }

    #[test]
    fn greeds_realm_is_the_vault_full_of_gold_and_idle_packs() {
        let g = in_the_vault();
        let v = g.vault.as_ref().expect("the Vaults of Mammon");
        assert_eq!(g.level.data.name, "The Vaults of Mammon");
        assert_eq!(g.hero.occupied(), v.arrival, "he arrives in the antechamber");
        assert_eq!(g.keys_total, 0, "no keys here: the far portal asks a toll instead");
        assert!(g.enemies.len() >= 60);
        assert!(g.enemies.iter().all(|e| e.lurking() && e.b.den.is_some()), "every foe idles in its room");
        assert!(g.relic_shrine.is_none() && g.potions.is_empty());
        assert!(g.hoard.is_empty());
    }

    #[test]
    fn a_pack_stirs_when_he_comes_near_and_gives_up_when_he_is_gone() {
        let mut g = in_the_vault();
        let v = g.vault.clone().unwrap();
        let hall = v.rooms.iter().find(|r| r.kind == RoomKind::Hall).unwrap().rect;
        let pack: Vec<usize> = (0..g.enemies.len()).filter(|&i| g.enemies[i].b.den == Some(hall)).collect();
        assert!(pack.len() >= 6);
        let others: Vec<usize> = (0..g.enemies.len()).filter(|&i| g.enemies[i].b.den != Some(hall)).collect();
        // Idle, they wander their room and never leave it.
        for _ in 0..(4.0 / STEP) as usize {
            g.update(STEP);
        }
        for &i in &pack {
            assert!(hall.contains(g.enemies[i].occupied()), "{} keeps to the hall", g.enemies[i].b.name);
            assert!(g.enemies[i].lurking());
        }
        // Into the hall: the whole pack wakes, and nobody else's.
        let (cx, cy) = hall.center();
        g.hero.place(P::new(cx as i32, cy as i32));
        g.hero.invuln = 1.0;
        g.update(STEP);
        for &i in &pack {
            assert!(!g.enemies[i].lurking(), "{} is roused", g.enemies[i].b.name);
        }
        assert!(others.iter().all(|&i| g.enemies[i].lurking()), "the other packs sleep on");
        assert!(g.floaters.iter().any(|f| f.text == "THE PACK STIRS"));
        // Far away again: they give up and go home.
        g.hero.place(v.arrival);
        g.update(STEP);
        for &i in &pack {
            assert!(g.enemies[i].b.returning, "{} goes home", g.enemies[i].b.name);
        }
        for _ in 0..(30.0 / STEP) as usize {
            g.update(STEP);
            g.hero.invuln = 1.0;
        }
        assert!(pack.iter().all(|&i| g.enemies[i].lurking()), "back in the hall, idling again");
    }

    #[test]
    fn gold_underfoot_goes_into_the_hoard_and_the_fallen_stay_down() {
        let mut g = in_the_vault();
        let v = g.vault.clone().unwrap();
        let coin = v.loot.iter().find(|l| l.kind == LootKind::Coin).unwrap();
        let chest = v.loot.iter().find(|l| l.kind == LootKind::Chest).unwrap();
        let gem = v.loot.iter().find(|l| matches!(l.kind, LootKind::Gem(_))).unwrap();
        for l in [coin, chest, gem] {
            g.hero.place(l.at);
            g.hero.invuln = 1.0;
            g.update(STEP);
        }
        assert_eq!(g.hoard, Hoard { gold: 710, coins: 1, piles: 0, chests: 1, gems: 1 });
        let taken = g.vault.as_ref().unwrap().loot.iter().filter(|l| l.taken).count();
        assert_eq!(taken, 3);
        // Slain foes are gone for good (a draugr after it has lain in pieces a while).
        let n = g.enemies.len();
        let draugr = (0..n).find(|&i| g.enemies[i].kind().reassembles()).unwrap();
        let other = (0..n).find(|&i| !g.enemies[i].kind().reassembles()).unwrap();
        g.enemies[draugr].kill();
        g.enemies[other].kill();
        g.update(STEP);
        assert_eq!(g.enemies.len(), n - 1, "the other falls at once");
        for _ in 0..(3.5 / STEP) as usize {
            g.update(STEP);
            g.hero.invuln = 1.0;
        }
        assert_eq!(g.enemies.len(), n - 2, "the draugr never rises");
    }

    #[test]
    fn f2_brings_him_to_the_far_portal_where_mammon_asks_his_toll() {
        let mut g = in_the_vault();
        let v = g.vault.clone().unwrap();
        // A coin in hand, so there is something to answer for.
        let coin = v.loot.iter().find(|l| l.kind == LootKind::Coin).unwrap();
        g.hero.place(coin.at);
        g.update(STEP);
        assert_eq!(g.hoard.gold, 10);
        g.input.key_down(winit::keyboard::KeyCode::F2, false);
        g.update(STEP);
        assert_eq!(g.hero.occupied(), P::new(v.exit_gate.x - 1, v.exit_gate.y));
        g.update(STEP);
        assert_eq!(g.screen, Screen::Toll, "before the portal, the question");
        assert_eq!(g.menu_items(), vec![MenuItem::DropAll, MenuItem::KeepAll]);
        // Keep everything: the vaults close and the realm starts again, the coin back where it lay.
        g.input.nav = Some(Dir::Down);
        g.update(STEP);
        g.input.inject(Action::Confirm);
        g.update(STEP);
        assert_eq!(g.screen, Screen::Ready);
        assert!(g.hoard.is_empty());
        assert_eq!(g.hero.occupied(), v.arrival);
        assert!(g.vault.as_ref().unwrap().loot.iter().all(|l| !l.taken));
        for _ in 0..((READY_SECONDS + 0.1) / STEP) as usize {
            g.update(STEP);
        }
        // Drop everything: the portal opens on Mammon's court.
        g.cheat_to_gate();
        g.update(STEP);
        assert_eq!(g.screen, Screen::Toll);
        g.input.inject(Action::Confirm);
        g.update(STEP);
        assert_eq!(g.screen, Screen::Playing);
        assert!(g.gate_open && g.hoard.is_empty());
        for _ in 0..(3.0 / STEP) as usize {
            g.update(STEP);
            g.hero.invuln = 1.0;
        }
        assert!(g.door >= 1.0, "the doors swing open");
        g.hero.m.teleport(v.exit_gate);
        g.update(STEP);
        assert_eq!(g.screen, Screen::Cleared);
        for _ in 0..((CLEARED_SECONDS + 0.1) / STEP) as usize {
            g.update(STEP);
        }
        assert_eq!((g.screen, g.realm, g.boss.is_some()), (Screen::Ready, Some(Sin::Greed), true), "Mammon's court");
        assert!(g.vault.is_none());
    }
}
