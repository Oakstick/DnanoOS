//! The Vaults of Mammon in play (the layout is `logic::vault`). Through
//! Greed's portal he arrives in the west antechamber; the packs idle in their
//! rooms until he comes near, then hunt him as long as he stays close, and go
//! back to their rooms when he is gone, as the packs of Diablo do; what falls
//! stays down. They fight by frame data (`brawl`, COMBAT.md §9): every foe is a
//! duelist on its archetype, paced by the threat manager. Every room is strewn with gold he can take: coins, heaps of
//! them, treasure heaps, chests and gems, all counted into his hoard. A chest
//! also throws out one to three items as it opens (potions, weapons, boons,
//! runes, bombs, in Diablo's four rarities), which land beside it and are his
//! for the walking over. The far portal leads to Mammon's court; the gold he
//! carries through it is counted into his score. Mammon makes no deal for it
//! (BACKLOG B40: the princes never make a deal; his toll is gone). F2 brings
//! him straight to the far portal.

use super::brawl::{Brawl, Field};
use super::{Banner, Game};
use crate::audio::Sfx;
use crate::logic::combat::duelist::Den;
use crate::logic::combat::{Breed, Duelist};
use crate::logic::enemy::{spec_of, Enemy, EnemyMode};
use crate::logic::gear;
use crate::logic::hub::Rect;
use crate::logic::items::ItemKind;
use crate::logic::progression;
use crate::logic::types::{Dir, Tile, P, V};
use crate::logic::vault::{roll_drops, Drop, DropItem, LootKind, Rarity, Vault, SEED};
use crate::render::view::CameraMode;

/// What he carries out of the vaults: its worth, and what it is made of.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Hoard {
    pub gold: u32,
    pub coins: u32,
    pub piles: u32,
    pub treasures: u32,
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
        let vault = Vault::generate(SEED, self.realm_tier());
        // The vaults are seen from above, as Diablo's are; the camera goes back to what it was on leaving.
        if self.camera_before_vault.is_none() {
            self.camera_before_vault = Some(self.view.camera_mode);
        }
        self.view.camera_mode = CameraMode::Top;
        self.start_facing = Dir::Right;
        self.level.hero_spawn = vault.arrival;
        self.hoard = Hoard::default();
        self.vault = Some(vault);
        let mut brawl = Brawl::new(Field::Vault);
        brawl.reset(self.hero_kit());
        self.enemies = self.vault_garrison(&mut brawl);
        self.brawl = Some(brawl);
    }

    /// The packs: every foe at its post in its room, asleep there, a duelist on its archetype.
    fn vault_garrison(&self, brawl: &mut Brawl) -> Vec<Enemy> {
        let Some(v) = &self.vault else { return Vec::new() };
        let mut out = Vec::new();
        for pack in &v.packs {
            let r = pack.den;
            let lo = V::new(r.x0 as f64 - 0.5, r.y0 as f64 - 0.5);
            let hi = V::new(r.x1 as f64 + 0.5, r.y1 as f64 + 0.5);
            let (cx, cy) = r.center();
            for &(kind, _, post) in &pack.foes {
                let mut spec = spec_of(&self.level.grid, kind);
                spec.name = format!("{} {}", spec.name, out.len() + 1);
                let mut e = Enemy::new(&spec, post, self.level.data.enemy_speed);
                e.post = Some(post);
                e.b.mode = EnemyMode::Chase;
                // Facing into the room, asleep at its post.
                let facing = V::new(cx as f64 - post.x as f64, cy as f64 - post.y as f64 + 0.01);
                let mut d = Duelist::bred(brawl.next_id(), Breed::of(kind), post.v(), facing);
                let (hp, damage) = self.ng_foe((super::brawl::VAULT_HP, 1.0));
                d.max_hp *= hp;
                d.hp = d.max_hp;
                d.breed.damage *= damage * self.realm_blows();
                d.den = Some(Den { lo, hi, home: post.v(), awake: false, returning: false });
                e.duel = Some(Box::new(d));
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
    pub(super) fn update_vault(&mut self, dt: f64) {
        let Some(v) = &mut self.vault else { return };
        for d in v.drops.iter_mut() {
            d.age += dt;
        }
        // (The packs' waking and giving up is the brawl's: `Game::update_dens`.)
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
        self.take_drops();
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
            LootKind::Treasure => {
                self.hoard.treasures += 1;
                self.audio.play(Sfx::Charge);
                self.audio.play(Sfx::Nodule);
                self.view.sparkle(grid, ox, oy, 0.5, 0xffd860, 24);
                format!("A TREASURE +{worth}")
            }
            LootKind::Chest => {
                self.hoard.chests += 1;
                self.audio.play(Sfx::Anvil);
                self.audio.play_reward(self.hoard.chests * 3);
                self.view.shake(0.2);
                self.view.sparkle(grid, ox, oy, 0.7, 0xffd860, 40);
                self.view.spark(grid, ox, oy, 0xffe890);
                self.open_chest(o);
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

    /// A chest opens: one to three items fly out and land on the floor beside it, the rarer the
    /// deeper the room. Each is named where it lands, in its rarity's colour.
    pub(super) fn open_chest(&mut self, at: P) {
        let Some(v) = &self.vault else { return };
        let depth = v.room_at(at).map(|r| r.depth).unwrap_or(0.5);
        let drops = roll_drops(&mut self.rng, depth);
        // Where they can land: open floor within two tiles, nearest first, clear of other things lying about.
        let g = &self.level.grid;
        let busy = |p: P| v.loot.iter().any(|l| !l.taken && l.at == p) || v.drops.iter().any(|d| !d.taken && d.at == p);
        let mut spots: Vec<P> = Vec::new();
        for dy in -2..=2 {
            for dx in -2..=2 {
                let p = P::new(at.x + dx, at.y + dy);
                if (dx, dy) != (0, 0) && g.get(p.x, p.y) == Tile::Open && !busy(p) {
                    spots.push(p);
                }
            }
        }
        self.rng.shuffle(&mut spots);
        spots.sort_by_key(|p| (p.x - at.x).abs().max((p.y - at.y).abs()));
        let hero = self.hero_kind;
        let mut best = None;
        let mut thrown = Vec::new();
        for (k, (item, rarity)) in drops.into_iter().enumerate() {
            let land = spots.get(k).copied().unwrap_or(at);
            thrown.push(Drop { at: land, from: at, item, rarity, age: 0.0, taken: false });
            let label = item.label(|p| gear::potion(hero, p).name);
            self.floater(land.x as f64, land.y as f64 - 0.5, label, 2.6, Some(rarity.color()));
            best = best.max(Some(rarity));
        }
        if let Some(v) = &mut self.vault {
            v.drops.extend(thrown);
        }
        match best {
            Some(Rarity::Legendary) => {
                self.audio.play(Sfx::Milestone);
                self.view.shake(0.4);
                self.view.sparkle(&self.level.grid, at.x as f64, at.y as f64, 1.0, Rarity::Legendary.color(), 50);
            }
            Some(Rarity::Rare) => self.audio.play(Sfx::Milestone),
            _ => self.audio.play(Sfx::Whoosh),
        }
    }

    /// Walking over an item that has landed takes it.
    fn take_drops(&mut self) {
        if self.hero.airborne() {
            return;
        }
        let o = self.hero.occupied();
        let Some(v) = &mut self.vault else { return };
        let Some(i) = v.drops.iter().position(|d| !d.taken && d.landed() && d.at == o) else { return };
        v.drops[i].taken = true;
        let Drop { item, rarity, .. } = v.drops[i];
        let (ox, oy) = (o.x as f64, o.y as f64);
        self.view.sparkle(&self.level.grid, ox, oy, 0.6, rarity.color(), 18 + 8 * rarity as usize);
        match item {
            DropItem::Potion(kind) => self.take_potion(kind, o),
            DropItem::Weapon(w) => {
                self.hero.equip(w);
                self.audio.play(Sfx::Start);
                self.floater(ox, oy, item.label(|_| ""), 1.4, Some(rarity.color()));
            }
            DropItem::Boon(b) => {
                match b {
                    ItemKind::Shield => self.hero.grant_shield(),
                    ItemKind::Invincible => self.hero.grant_invincibility(),
                    ItemKind::Speed => self.hero.grant_speed(),
                }
                self.audio.play(Sfx::Start);
                self.floater(ox, oy, item.label(|_| ""), 1.4, Some(rarity.color()));
            }
            DropItem::Runes(n) => {
                self.progress.runes += n;
                self.audio.play(Sfx::Milestone);
                self.floater(ox, oy, progression::gems(self.hero_kind, n), 1.6, Some(0x9ad1ff));
            }
            DropItem::Bombs(n) => {
                self.bombs_left += n;
                self.audio.play(Sfx::Fuse);
                self.floater(ox, oy, format!("+{n} BOMB{}", if n > 1 { "S" } else { "" }), 1.4, Some(rarity.color()));
            }
        }
        if rarity == Rarity::Legendary {
            self.juice.banner = Some(Banner {
                text: format!("LEGENDARY: {}", item.label(|p| gear::potion(self.hero_kind, p).name)),
                sub: "FROM THE VAULTS OF MAMMON".into(),
                color: rarity.color(),
                life: 2.4,
                max: 2.4,
            });
        }
    }

    /// Through the far portal: the gold he carries is counted into his score (Mammon asks nothing
    /// for it: no prince makes a deal).
    pub(super) fn count_hoard(&mut self) {
        if self.hoard.is_empty() {
            return;
        }
        let gold = self.hoard.gold;
        let points = self.award(gold);
        self.hoard = Hoard::default();
        let hp = self.hero.pos();
        self.floater(hp.x, hp.y - 1.0, format!("THE HOARD IS COUNTED +{points}"), 2.0, Some(0xffd860));
    }

    /// F2 in the vaults: straight to the far portal.
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
    use crate::logic::sins::Sin;
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

    /// Asleep in its den (a duelist that has not been woken).
    fn asleep(e: &Enemy) -> bool {
        e.duel.as_ref().and_then(|d| d.den).is_some_and(|d| !d.awake)
    }

    /// The foes whose den is `room`.
    fn pack_of(g: &Game, room: Rect) -> Vec<usize> {
        let (cx, cy) = room.center();
        let c = V::new(cx as f64, cy as f64);
        (0..g.enemies.len()).filter(|&i| g.enemies[i].duel.as_ref().and_then(|d| d.den).is_some_and(|d| d.contains(c))).collect()
    }

    #[test]
    fn greeds_realm_is_the_vault_full_of_gold_and_idle_packs() {
        let g = in_the_vault();
        let v = g.vault.as_ref().expect("the Vaults of Mammon");
        assert_eq!(g.level.data.name, "The Vaults of Mammon");
        assert_eq!(g.hero.occupied(), v.arrival, "he arrives in the antechamber");
        assert_eq!(g.keys_total, 0, "no golden keys in the vaults");
        assert!(g.enemies.len() >= 50, "packs sized for frame-data fights");
        assert!(g.enemies.iter().all(asleep), "every foe sleeps in its room");
        assert!(g.brawl.as_ref().is_some_and(|b| b.field == Field::Vault), "the vaults are fought by frame data");
        assert!(g.relic_shrine.is_none() && g.potions.is_empty());
        assert!(g.hoard.is_empty());
    }

    #[test]
    fn a_pack_stirs_when_he_comes_near_and_gives_up_when_he_is_gone() {
        let mut g = in_the_vault();
        let v = g.vault.clone().unwrap();
        let hall = v.rooms.iter().find(|r| r.kind == RoomKind::Hall).unwrap().rect;
        let pack = pack_of(&g, hall);
        assert!(pack.len() >= 6);
        let others: Vec<usize> = (0..g.enemies.len()).filter(|i| !pack.contains(i)).collect();
        let posts: Vec<V> = pack.iter().map(|&i| g.enemies[i].pos()).collect();
        // Asleep, they stand at their posts.
        for _ in 0..(4.0 / STEP) as usize {
            g.update(STEP);
        }
        for (k, &i) in pack.iter().enumerate() {
            assert_eq!(g.enemies[i].pos(), posts[k], "{} sleeps at its post", g.enemies[i].b.name);
            assert!(asleep(&g.enemies[i]));
        }
        // Into the hall: the whole pack wakes, and nobody else's.
        let (cx, cy) = hall.center();
        g.hero.place(P::new(cx as i32, cy as i32));
        g.update(STEP);
        g.update(STEP);
        for &i in &pack {
            assert!(g.enemies[i].duel.as_ref().unwrap().engaged(), "{} is roused", g.enemies[i].b.name);
        }
        assert!(others.iter().all(|&i| asleep(&g.enemies[i])), "the other packs sleep on");
        assert!(g.floaters.iter().any(|f| f.text == "THE PACK STIRS"));
        // Far away again: they give up and go home, and fall asleep there.
        g.hero.place(v.arrival);
        g.update(STEP);
        g.update(STEP);
        for &i in &pack {
            assert!(!g.enemies[i].duel.as_ref().unwrap().engaged(), "{} gives up the hunt", g.enemies[i].b.name);
        }
        for _ in 0..(30.0 / STEP) as usize {
            g.update(STEP);
        }
        assert!(pack.iter().all(|&i| asleep(&g.enemies[i])), "back at their posts, asleep again");
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
        assert_eq!(g.hoard, Hoard { gold: 710, coins: 1, piles: 0, treasures: 0, chests: 1, gems: 1 });
        let taken = g.vault.as_ref().unwrap().loot.iter().filter(|l| l.taken).count();
        assert_eq!(taken, 3);
        // Slain foes are gone for good, a draugr too, with a kill's spoils.
        let n = g.enemies.len();
        let draugr = (0..n).find(|&i| g.enemies[i].kind().reassembles()).unwrap();
        let other = (0..n).find(|&i| !g.enemies[i].kind().reassembles()).unwrap();
        let coins = g.progress.silver;
        for i in [draugr, other] {
            g.enemies[i].duel.as_mut().unwrap().hp = 1.0;
            g.stray_blow(i);
        }
        assert!(g.progress.silver > coins, "a kill's coins");
        for _ in 0..(0.3 / STEP) as usize {
            g.update(STEP); // (through the kill's freeze-frame)
        }
        assert_eq!(g.enemies.len(), n - 2, "both fall, and the draugr never rises");
        for _ in 0..(3.5 / STEP) as usize {
            g.update(STEP);
            g.hero.invuln = 1.0;
        }
        assert_eq!(g.enemies.len(), n - 2);
    }

    /// The vault with only one foe left in it, awake and `gap` tiles north of him (he faces north),
    /// asking for no token, and beginning a slam if `slam`.
    fn one_on_one(gap: f64, slam: bool) -> Game {
        let mut g = in_the_vault();
        let keep = g.enemies.iter().position(|e| e.duel.is_some()).unwrap();
        let e = g.enemies.swap_remove(keep);
        g.enemies = vec![e];
        g.view.clear_enemies();
        g.view.add_enemy(&g.enemies[0]);
        let v = g.vault.clone().unwrap();
        let hall = v.rooms.iter().find(|r| r.kind == RoomKind::Hall).unwrap().rect;
        let (cx, cy) = hall.center();
        g.hero.place(P::new(cx as i32, cy as i32));
        g.hero.face(Dir::Up);
        let hp = g.hero.pos();
        let d = g.enemies[0].duel.as_mut().unwrap();
        d.pos = V::new(hp.x, hp.y - gap);
        d.facing = V::new(0.0, 1.0);
        d.rest = u16::MAX;
        if let Some(den) = d.den.as_mut() {
            den.awake = true;
        }
        if slam {
            d.token = true;
            assert!(d.begin(crate::logic::combat::MoveId::Slam, hp));
        }
        g
    }

    #[test]
    fn in_the_vaults_his_shield_wears_as_it_blocks_and_breaks_for_good() {
        let mut g = one_on_one(1.4, true);
        let tier = g.progress.hero(g.hero_kind).carry;
        g.hero.shield = Some(gear::ShieldGear { durability: 5, ..gear::ShieldGear::new(tier) });
        let hp = g.hero.hp;
        g.input.inject(Action::Block);
        for _ in 0..(1.2 / STEP) as usize {
            g.update(STEP);
        }
        assert_eq!(g.brawl.as_ref().unwrap().stats.blocks, 1, "{:?}", g.brawl.as_ref().unwrap().stats);
        // (Only the health the shield itself gave him went with it.)
        assert_eq!(g.hero.hp, hp.min(g.hero.max_hp), "the guard took the blow");
        assert!(g.hero.shield.is_none(), "and wore through");
        assert!(g.progress.hero(g.hero_kind).is_broken(tier), "struck off until the old man reforges it");
        let guard = g.brawl.as_ref().unwrap().kit.guard;
        assert_eq!(guard, crate::logic::combat::kit::Guard::BARE, "bare hands now");
    }

    #[test]
    fn a_foe_in_the_vaults_can_slay_him_and_the_pack_goes_home_with_its_wounds() {
        let mut g = one_on_one(1.4, true);
        g.enemies[0].duel.as_mut().unwrap().hp = 5.0;
        let lives = g.score.lives;
        g.hero.hp = 10;
        for _ in 0..(1.2 / STEP) as usize {
            g.update(STEP);
        }
        assert_eq!(g.screen, Screen::Dying, "slain for real in the vaults");
        for _ in 0..(2.5 / STEP) as usize {
            g.update(STEP);
        }
        assert_eq!(g.score.lives, lives - 1);
        let d = g.enemies[0].duel.as_ref().unwrap();
        let den = d.den.unwrap();
        // (WARBAND.md W11: as a prince does, it keeps the wounds he gave it.)
        assert!(!den.awake && d.pos == den.home && d.hp == 5.0, "home, asleep, and wounded still: {}", d.hp);
    }

    #[test]
    fn the_old_mans_carvings_spare_the_shield_a_quarter_of_each_blocked_blow_apiece() {
        let worn = |carved: u8| {
            let mut g = one_on_one(1.4, true);
            let tier = g.progress.hero(g.hero_kind).carry;
            g.progress.hero_mut(g.hero_kind).carved[1][(tier - 1) as usize] = carved;
            g.hero.shield = Some(gear::ShieldGear::new(tier));
            let full = g.hero.shield.unwrap().durability;
            g.input.inject(Action::Block);
            for _ in 0..(1.2 / STEP) as usize {
                g.update(STEP);
            }
            assert_eq!(g.brawl.as_ref().unwrap().stats.blocks, 1);
            full - g.hero.shield.unwrap().durability
        };
        let (bare, three) = (worn(0), worn(3));
        assert!(bare > 0, "a block wears the shield");
        let want = (bare as f64 * crate::logic::progression::carved_share(3, crate::logic::progression::CARVE_WEAR)).round() as i32;
        assert!((three - want).abs() <= 1, "three carvings: {three} of {bare}");
    }

    /// BACKLOG B40: Mammon makes no deal. F2 brings him to the far portal; it opens on his court
    /// with no question asked, and the gold he carries through is counted into his score.
    #[test]
    fn f2_brings_him_to_the_far_portal_and_it_opens_with_no_toll() {
        let mut g = in_the_vault();
        let v = g.vault.clone().unwrap();
        let coin = v.loot.iter().find(|l| l.kind == LootKind::Coin).unwrap();
        g.hero.place(coin.at);
        g.update(STEP);
        assert_eq!(g.hoard.gold, 10);
        let score = g.score.score;
        g.input.key_down(winit::keyboard::KeyCode::F2, false);
        g.update(STEP);
        assert_eq!(g.hero.occupied(), P::new(v.exit_gate.x - 1, v.exit_gate.y));
        for _ in 0..(3.0 / STEP) as usize {
            g.update(STEP);
            g.hero.invuln = 1.0;
            assert_ne!(g.screen, Screen::Toll, "no question at the portal");
        }
        assert!(g.gate_open && g.door >= 1.0, "the doors swing open");
        g.hero.m.teleport(v.exit_gate);
        g.update(STEP);
        assert_eq!(g.screen, Screen::Cleared);
        assert!(g.hoard.is_empty() && g.score.score >= score + 10, "the hoard counted into his score");
        for _ in 0..((CLEARED_SECONDS + 0.1) / STEP) as usize {
            g.update(STEP);
        }
        assert_eq!((g.screen, g.realm, g.in_court()), (Screen::Ready, Some(Sin::Greed), true), "Mammon's court");
        assert!(g.boss.is_none() && g.prince().is_some(), "Mammon himself, by frame data");
        assert!(g.vault.is_none());
    }

    #[test]
    fn a_chest_throws_out_items_that_land_beside_it_and_are_his_for_the_walking_over() {
        let mut g = in_the_vault();
        let v = g.vault.clone().unwrap();
        let chest = v.loot.iter().find(|l| l.kind == LootKind::Chest).unwrap().at;
        g.hero.place(chest);
        g.hero.invuln = 1.0;
        g.update(STEP);
        let drops = g.vault.as_ref().unwrap().drops.clone();
        assert!((1..=3).contains(&drops.len()), "{} items", drops.len());
        for d in &drops {
            assert_eq!(d.from, chest);
            assert!(d.at != chest && (d.at.x - chest.x).abs() <= 2 && (d.at.y - chest.y).abs() <= 2, "lands beside it");
            assert_eq!(g.level.grid.get(d.at.x, d.at.y), Tile::Open);
        }
        // Not to be had in the air; once landed, walking over takes it.
        let d = drops[0];
        g.vault.as_mut().unwrap().drops[0].item = DropItem::Bombs(2);
        let bombs = g.bombs_left;
        g.hero.place(d.at);
        g.update(STEP);
        assert!(!g.vault.as_ref().unwrap().drops[0].taken, "still flying");
        for _ in 0..(0.7 / STEP) as usize {
            g.update(STEP);
            g.hero.invuln = 1.0;
        }
        assert!(g.vault.as_ref().unwrap().drops[0].taken);
        assert_eq!(g.bombs_left, bombs + 2);
    }
}
