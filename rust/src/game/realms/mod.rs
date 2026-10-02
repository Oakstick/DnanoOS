//! The six realms after the vaults in play (`logic::realms` lays them out; REALMS.md has their
//! designs). Each is entered through its sin's portal as the vaults are:
//! - he arrives before the portal he came through, and the camera looks down from above;
//! - its natives idle in packs at their dens and fight by frame data (`brawl`, under the vaults'
//!   rules: his real health, his shield, his blade);
//! - the portal to the prince's court stays sealed until the realm's last trap has been survived.
//!
//! Each realm's own rules are in its module. This one hands them every tick, fall and cheat, and
//! keeps what they share: the packs, the meters on the HUD, the prince's offer and the seal.

mod feast;
mod field;
mod grotto;
mod mire;
mod mirrors;
mod thorns;

use super::brawl::{Brawl, Field};
use super::{Game, MenuItem, Screen};
use crate::input::Action;
use crate::logic::combat::duelist::Den;
use crate::logic::combat::{Breed, Duelist};
use crate::logic::enemy::{spec_of, Enemy, EnemyMode};
use crate::logic::realms::{weight, Pack, Run};
use crate::logic::sins::Sin;
use crate::logic::types::{Dir, V};
use crate::render::view::CameraMode;

/// A gauge a realm shows on the HUD: its name, how full of how much, and its colour. A count is
/// drawn as a number rather than a bar; a warning makes it pulse.
#[derive(Clone, Debug, PartialEq)]
pub struct Meter {
    pub label: String,
    pub value: f32,
    pub max: f32,
    pub color: u32,
    pub count: bool,
    pub warn: bool,
}

impl Meter {
    pub fn bar(label: &str, value: f32, max: f32, color: u32) -> Self {
        Self { label: label.to_string(), value, max, color, count: false, warn: false }
    }

    pub fn count(label: &str, value: u32, color: u32) -> Self {
        Self { label: label.to_string(), value: value as f32, max: 0.0, color, count: true, warn: false }
    }

    /// The same, pulsing.
    pub fn warning(mut self, on: bool) -> Self {
        self.warn = on;
        self
    }
}

/// The prince's offer, when a realm makes one (as Mammon makes his toll): its title, what is
/// said, and the answers, the one he would have chosen first.
#[derive(Clone, Debug, PartialEq)]
pub struct Offer {
    pub title: String,
    pub lines: Vec<String>,
    pub choices: Vec<MenuItem>,
    pub color: u32,
}

impl Game {
    /// In one of the six realms after the vaults.
    pub fn in_run(&self) -> bool {
        self.run.is_some()
    }

    /// Lays out the realm of `sin` as its level is loaded: its packs asleep at their dens, the
    /// camera above, the far portal sealed.
    pub(super) fn enter_run(&mut self, sin: Sin) {
        let Some(run) = Run::generate(sin, self.realm_tier()) else { return };
        // Seen from above, as the vaults are; the camera goes back to what it was on leaving.
        if self.camera_before_vault.is_none() {
            self.camera_before_vault = Some(self.view.camera_mode);
        }
        self.view.camera_mode = CameraMode::Top;
        let b = run.base();
        self.start_facing = b.entry.facing;
        self.level.hero_spawn = b.checkpoint;
        let (tier, packs) = (b.tier, b.packs.clone());
        let mut brawl = Brawl::new(Field::Vault);
        brawl.reset(self.hero_kit());
        self.enemies = self.raise_packs(&packs, tier, &mut brawl);
        self.brawl = Some(brawl);
        self.run = Some(run);
        self.gate_open = false;
        match sin {
            Sin::Pride => self.enter_mirrors(),
            Sin::Envy => self.enter_grotto(),
            Sin::Wrath => self.enter_field(),
            Sin::Sloth => self.enter_mire(),
            Sin::Gluttony => self.enter_feast(),
            Sin::Lust => self.enter_thorns(),
            Sin::Greed => {}
        }
    }

    /// Every foe of `packs` at its post in its den, asleep there: a duelist on its archetype,
    /// weighed for `tier` (`logic::realms::weight`).
    pub(super) fn raise_packs(&self, packs: &[Pack], tier: usize, brawl: &mut Brawl) -> Vec<Enemy> {
        let mut out = Vec::new();
        for pack in packs {
            let r = pack.den;
            let lo = V::new(r.x0 as f64 - 0.5, r.y0 as f64 - 0.5);
            let hi = V::new(r.x1 as f64 + 0.5, r.y1 as f64 + 0.5);
            let (cx, cy) = r.center();
            for &(kind, stage, post) in &pack.foes {
                let mut spec = spec_of(&self.level.grid, kind);
                spec.name = format!("{} {}", spec.name, out.len() + 1);
                let mut e = Enemy::new(&spec, post, self.level.data.enemy_speed);
                e.post = Some(post);
                e.b.mode = EnemyMode::Chase;
                // Facing into its den, asleep at its post.
                let facing = V::new(cx as f64 - post.x as f64, cy as f64 - post.y as f64 + 0.01);
                let mut d = Duelist::bred(brawl.next_id(), Breed::of(kind), post.v(), facing);
                let (hp, damage) = weight(if kind.realm_own() { 0 } else { stage }, tier);
                d.max_hp *= hp;
                d.hp = d.max_hp;
                d.breed.damage *= damage;
                d.den = Some(Den { lo, hi, home: post.v(), awake: false, returning: false });
                e.duel = Some(Box::new(d));
                out.push(e);
            }
        }
        out
    }

    /// The realm's rules each tick: what is shared (the fallen cleared away), then its own.
    pub(super) fn update_run(&mut self, dt: f64) {
        let Some(sin) = self.run.as_ref().map(Run::sin) else { return };
        // What falls stays down, as in the vaults: a draugr lies in its pieces a while, and is then gone.
        for e in self.enemies.iter_mut() {
            if e.mode() == EnemyMode::Scattered && e.b.scatter_timer < 0.4 {
                e.b.mode = EnemyMode::Eaten;
            }
        }
        if self.enemies.iter().any(|e| e.mode() == EnemyMode::Eaten) {
            self.enemies.retain(|e| e.mode() != EnemyMode::Eaten);
            self.refresh_foe_views();
        }
        match sin {
            Sin::Pride => self.update_mirrors(dt),
            Sin::Envy => self.update_grotto(dt),
            Sin::Wrath => self.update_field(dt),
            Sin::Sloth => self.update_mire(dt),
            Sin::Gluttony => self.update_feast(dt),
            Sin::Lust => self.update_thorns(dt),
            Sin::Greed => {}
        }
    }

    /// The view's foes, rebuilt after the cast changed (a foe gone for good, or a new one raised).
    pub(super) fn refresh_foe_views(&mut self) {
        self.view.clear_enemies();
        for e in &self.enemies {
            self.view.add_enemy(e);
        }
    }

    /// The controls as the realm leaves them, before he moves this tick: his free-movement wish
    /// (`hero.wish`, changed in place) and the way he is steered on the grid (returned).
    pub(super) fn steer_run(&mut self, wanted: Dir) -> Dir {
        let Some(sin) = self.run.as_ref().map(Run::sin) else { return wanted };
        match sin {
            Sin::Pride => self.steer_mirrors(wanted),
            Sin::Envy => self.steer_grotto(wanted),
            Sin::Wrath => self.steer_field(wanted),
            Sin::Sloth => self.steer_mire(wanted),
            Sin::Gluttony => self.steer_feast(wanted),
            Sin::Lust => self.steer_thorns(wanted),
            Sin::Greed => wanted,
        }
    }

    /// After a fall, as he rises at the realm's checkpoint (and as the realm is first entered).
    pub(super) fn reset_run(&mut self) {
        let Some(sin) = self.run.as_ref().map(Run::sin) else { return };
        match sin {
            Sin::Pride => self.reset_mirrors(),
            Sin::Envy => self.reset_grotto(),
            Sin::Wrath => self.reset_field(),
            Sin::Sloth => self.reset_mire(),
            Sin::Gluttony => self.reset_feast(),
            Sin::Lust => self.reset_thorns(),
            Sin::Greed => {}
        }
    }

    /// F2 in a realm: straight to its last trap.
    pub(super) fn cheat_run(&mut self) {
        let Some(sin) = self.run.as_ref().map(Run::sin) else { return };
        match sin {
            Sin::Pride => self.cheat_mirrors(),
            Sin::Envy => self.cheat_grotto(),
            Sin::Wrath => self.cheat_field(),
            Sin::Sloth => self.cheat_mire(),
            Sin::Gluttony => self.cheat_feast(),
            Sin::Lust => self.cheat_thorns(),
            Sin::Greed => {}
        }
    }

    /// Before the far portal (the cheat's default): he stands in front of it, facing it.
    pub(super) fn cheat_to_far_portal(&mut self) {
        let Some(exit) = self.run.as_ref().map(|r| r.base().exit) else { return };
        let face = exit.facing.opposite();
        self.hero.place(exit.front());
        self.hero.face(face);
        self.view.aim_along(face);
    }

    /// What the HUD shows of the realm's rules.
    pub fn run_meters(&self) -> Vec<Meter> {
        let Some(sin) = self.run.as_ref().map(Run::sin) else { return Vec::new() };
        match sin {
            Sin::Pride => self.mirrors_meters(),
            Sin::Envy => self.grotto_meters(),
            Sin::Wrath => self.field_meters(),
            Sin::Sloth => self.mire_meters(),
            Sin::Gluttony => self.feast_meters(),
            Sin::Lust => self.thorns_meters(),
            Sin::Greed => Vec::new(),
        }
    }

    /// The line under the countdown: what this realm asks of him.
    pub fn run_where(&self) -> Option<String> {
        let run = self.run.as_ref()?;
        Some(
            match run.sin() {
                Sin::Pride => "THE HALL OF MIRRORS: TRUST THE MIRROR THAT SHOWS YOU AS YOU ARE",
                Sin::Envy => "THE SERPENT GROTTO: THE OTHER SIDE ALWAYS LOOKS RICHER",
                Sin::Wrath => "THE BURNING FIELD: THE FIELD KEEPS SCORE OF EVERY BLOW",
                Sin::Sloth => "THE GREY MIRE: THE GOAL COMES TO THOSE WHO WAIT, AND SO DOES THE SLEEP",
                Sin::Gluttony => "THE ROTTING FEAST: EAT JUST ENOUGH",
                Sin::Lust => "THE GARDEN OF THORNS: TO LEAVE, WALK TOWARD WHAT YOU DO NOT WANT",
                Sin::Greed => return None,
            }
            .to_string(),
        )
    }

    /// The prince's offer standing before him now, if his realm makes one.
    pub fn run_offer(&self) -> Option<Offer> {
        let sin = self.run.as_ref()?.sin();
        match sin {
            Sin::Sloth => self.mire_offer(),
            _ => None,
        }
    }

    /// The offer is put to him: play stops for the question, the realm's first answer highlighted.
    #[allow(dead_code)] // (for the realms whose princes make an offer: the Grey Mire's bed)
    pub(super) fn ask_offer(&mut self) {
        self.enter(Screen::Toll);
        self.menu_index = 0;
        self.input.consume(Action::Confirm);
        self.input.consume(Action::Jump);
        self.input.consume(Action::Attack);
    }

    /// His answer to the offer.
    pub(super) fn answer_run_offer(&mut self, choice: MenuItem) {
        let Some(sin) = self.run.as_ref().map(Run::sin) else {
            self.enter(Screen::Playing);
            return;
        };
        match sin {
            Sin::Sloth => self.answer_mire(choice),
            _ => self.enter(Screen::Playing),
        }
    }

    /// What the sealed far portal tells him when he comes up to it.
    pub(super) fn run_seal_hint(&self) -> String {
        let Some(sin) = self.run.as_ref().map(Run::sin) else { return String::new() };
        format!("SEALED: {} HAS NOT LET YOU GO", sin.realm().to_uppercase())
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::game::{save, READY_SECONDS};
    use crate::logic::sins::SINS;
    pub(crate) const STEP: f64 = crate::app::STEP;

    /// A game in the realm of `sin`, play under way.
    pub(crate) fn in_realm(sin: Sin) -> Game {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        let mut g = Game::new();
        g.map_jump(save::Stage::Realm(sin));
        for _ in 0..((READY_SECONDS + 0.1) / STEP) as usize {
            g.update(STEP);
        }
        assert_eq!(g.screen, Screen::Playing, "{sin:?}");
        g
    }

    #[test]
    fn every_realm_after_the_vaults_is_entered_with_its_packs_asleep_at_their_dens() {
        for sin in SINS.into_iter().filter(|&s| s != Sin::Greed) {
            let g = in_realm(sin);
            let run = g.run.as_ref().unwrap_or_else(|| panic!("{sin:?} has a realm"));
            assert_eq!(run.sin(), sin);
            assert_eq!(g.level.data.name, sin.realm());
            assert_eq!(g.realm, Some(sin));
            assert!(g.vault.is_none());
            assert_eq!(g.gate, Some(run.base().exit.tile), "{sin:?}: the far portal is the level's gate");
            assert!(g.brawl.as_ref().is_some_and(|b| b.field == Field::Vault), "{sin:?} is fought by frame data");
            assert!(g.enemies.iter().all(|e| e.duel.as_ref().is_some_and(|d| d.den.is_some_and(|d| !d.awake))), "{sin:?}: asleep");
            assert_eq!(g.view.camera_mode, CameraMode::Top);
            assert_eq!(g.keys_total, 0, "{sin:?}: no keys; the realm's own trap seals the way");
            assert!(g.relic_shrine.is_none() && g.potions.is_empty());
        }
    }
}
