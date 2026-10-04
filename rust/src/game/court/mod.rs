//! A prince's court in play, once it is built to its own plan (`logic::court` lays each out): the
//! prince is a duelist like every foe (`brawl`, COMBAT.md §13): the fourth archetype, a table of
//! moves for each phase of his health, a hide only his recoveries open, a token of three. The
//! court is fought under the vaults' rules (his real health, his shield and his blade), with a
//! deeper pool of threat for the prince and what he raises.
//!
//! What every court shares is here: the prince set before his throne as the level loads, his bar,
//! the banner and the green cross for a breather as his health falls into a new phase, his adds
//! sent ahead by his general's decree, his fall and his key. When he falls he drops the key of his
//! sin, and the way on is as from any boss: the key opens the gate behind him, then the old man's
//! wares, then the Portals of Hell with the sin sealed. If the hero falls, the prince is whole
//! again and back where he began.
//!
//! What each prince does of his own is his sin's module, through a few hooks it answers (called by
//! `match` on the court's sin): `greed` (Mammon in the Counting House: his golems climb out of his
//! heaps, his hoard spills as he falls), and `pride`, `envy`, `wrath`, `sloth`, `gluttony` and
//! `lust`, empty until their courts are built. Each module's hooks are `<sin>_enter` (after the
//! prince is set down), `<sin>_tick` (every tick of the fight), `<sin>_phase` (as his phase
//! changes), `<sin>_reset` (after a fall), `<sin>_slain` (as he falls), `<sin>_banner` (the phase
//! banner's line and colour) and `<sin>_raise` (his adds, when they come at once).
//!
//! Krampus at the bottom of the Pit is no sin's prince but fights as one (`game::lair` keeps his
//! fight): `prince` and `prince_bar` find him in his lair too, and `prince_phase` hands his phases
//! to `krampus_phase`.

mod envy;
mod gluttony;
mod greed;
mod lust;
mod pride;
mod sloth;
mod wrath;

use super::brawl::{Brawl, Field};
use super::{Banner, Game};
use crate::audio::Sfx;
use crate::logic::combat::archetype::Archetype;
use crate::logic::combat::prince::krampus::PHASE_AT as KRAMPUS_AT;
use crate::logic::combat::prince::{phase_name, PHASE_AT};
use crate::logic::combat::{Breed, Duelist};
use crate::logic::court::{prince_spot, weight};
use crate::logic::enemy::{spec_of, Enemy, EnemyMode};
use crate::logic::sins::Sin;
use crate::logic::types::{dist_sq, Tile, P, V};

/// The court's pace: threat points (the prince's three: while he strikes, what he raises waits) and
/// frames between attack tokens.
pub const COURT_CAPACITY: u8 = 3;
pub const COURT_CADENCE: u32 = 45;

/// What the court keeps of its fight: whose court it is, who the prince is, his phase as last
/// seen, whether he has fallen, and his health as the bar showed it a moment ago (its pale trail).
#[derive(Clone, Debug, PartialEq)]
pub struct Court {
    pub sin: Sin,
    pub prince: u32,
    pub phase: u8,
    pub slain: bool,
    pub trail: f32,
    /// His adds are to come as his fight begins (his general's decree was done, WARBAND.md §9).
    pub adds_due: bool,
    /// Lust's court: its censers, and the kiss on the hero (`lust`).
    bower: lust::Bower,
    /// Beelzebub's banquet: his platters, his meal, his flies (`gluttony`).
    pub banquet: gluttony::Banquet,
    /// What Envy's court keeps of its fight (`envy`): the archangel the hero last called, and the
    /// prince's move as last seen.
    pub envy: envy::EnvyState,
}

/// The prince's bar on the HUD: his name, the phase's title, his health and its trail, and
/// whether his hide is open (`render::hud::court`).
#[derive(Clone, Debug, PartialEq)]
pub struct PrinceBar {
    pub name: String,
    pub phase: u8,
    pub title: &'static str,
    pub hp: f32,
    pub max: f32,
    pub trail: f32,
    pub open: bool,
    pub flash: bool,
    /// His sin's colour, for his health.
    pub color: u32,
    /// He wears a hide that turns blows outside his openings (every prince but Amon, whose wrath
    /// has none: he is only ever open or not).
    pub hide: bool,
    /// Where on the bar his phases change (shares of his health, the lower first): the thirds, or
    /// Krampus's own.
    pub notches: [f32; 2],
}

impl Game {
    /// In a prince's court built to its own plan (the Counting House, and each court as it is built).
    pub fn in_court(&self) -> bool {
        self.court.is_some()
    }

    /// The sin whose court he is in.
    pub fn court_sin(&self) -> Option<Sin> {
        self.court.as_ref().map(|c| c.sin)
    }

    /// The prince, while he stands (or Krampus in his lair, who fights as a prince: `lair`).
    pub fn prince(&self) -> Option<&Duelist> {
        let id = self.court.as_ref().map(|c| c.prince).or(self.lair.as_ref().map(|l| l.krampus))?;
        self.enemies.iter().filter_map(|e| e.duel.as_deref()).find(|d| d.id == id && d.alive())
    }

    /// Where the chase camera turns to in a prince's court: the prince, while he stands and the
    /// fight is on.
    pub(super) fn prince_focus(&self) -> Option<glam::Vec3> {
        if !matches!(self.screen, super::Screen::Playing | super::Screen::Paused) {
            return None;
        }
        self.prince().map(|d| glam::Vec3::new(d.pos.x as f32, 0.0, d.pos.y as f32))
    }

    /// What the HUD shows of the prince's health (none once he has fallen): in a court, in his sin's
    /// colour; in the lair, Krampus's, notched where his own phases fall.
    pub fn prince_bar(&self) -> Option<PrinceBar> {
        let (trail, color, notches) = match (self.court.as_ref(), self.lair.as_ref()) {
            (Some(c), _) => (c.trail, c.sin.color(), [PHASE_AT[1] as f32, PHASE_AT[0] as f32]),
            (None, Some(l)) => (l.trail, super::lair::KRAMPUS_RED, [KRAMPUS_AT[1] as f32, KRAMPUS_AT[0] as f32]),
            (None, None) => return None,
        };
        let d = self.prince()?;
        Some(PrinceBar {
            name: self.boss_name(),
            phase: d.breed.phase,
            title: phase_name(d.breed.kind, d.breed.phase),
            hp: d.hp.max(0.0) as f32,
            max: d.max_hp as f32,
            trail,
            open: d.exposed(),
            flash: d.flash > 0,
            color,
            hide: d.breed.power.is_hide(),
            notches,
        })
    }

    /// Lays out the court of `sin` as the level is loaded: the prince where he begins, the fight's
    /// rules.
    pub(super) fn enter_court(&mut self, sin: Sin) {
        let tier = self.realm_tier();
        let mut brawl = Brawl::new(Field::Vault);
        brawl.reset(self.hero_kit());
        brawl.threat.capacity = COURT_CAPACITY;
        brawl.threat.cadence = COURT_CADENCE;
        brawl.foe_damage = 1.0;
        let id = brawl.next_id();
        let spot = prince_spot(sin);
        let at = P::new(spot.0, spot.1);
        let (hp, damage) = weight(tier);
        let kind = sin.prince_kind();
        let spec = spec_of(&self.level.grid, kind);
        let mut e = Enemy::new(&spec, at, self.level.data.enemy_speed);
        e.post = Some(at);
        e.b.mode = EnemyMode::Chase;
        e.b.name = spec.name.to_uppercase();
        let mut d = Duelist::bred(id, Breed::of(kind), at.v(), V::new(0.0, 1.0));
        d.max_hp *= hp;
        d.hp = d.max_hp;
        d.breed.damage *= damage;
        d.rest = 90;
        // His general's realm weighs on his fight: slain, his phase-two adds are lost; his decree
        // done, his adds come at once.
        let terms = self.prince_terms(sin);
        d.lore.summoned = !terms.phase_two_adds;
        e.duel = Some(Box::new(d));
        self.enemies = vec![e];
        self.brawl = Some(brawl);
        let trail = self.enemies[0].duel.as_ref().map(|d| d.max_hp as f32).unwrap_or(0.0);
        self.court = Some(Court {
            sin,
            prince: id,
            phase: 1,
            slain: false,
            trail,
            adds_due: terms.adds_at_start,
            bower: lust::Bower::default(),
            banquet: Default::default(),
            envy: Default::default(),
        });
        match sin {
            Sin::Greed => self.greed_enter(id),
            Sin::Pride => self.pride_enter(id),
            Sin::Envy => self.envy_enter(id),
            Sin::Wrath => self.wrath_enter(id),
            Sin::Sloth => self.sloth_enter(id),
            Sin::Gluttony => self.gluttony_enter(id),
            Sin::Lust => self.lust_enter(id),
        }
    }

    /// After a fall: the prince whole again where he began, his adds gone (unless he has already
    /// fallen, and his key waits).
    pub(super) fn reset_court(&mut self) {
        let Some(c) = self.court.clone() else { return };
        if c.slain {
            return;
        }
        let keep = c.prince;
        self.enemies.retain(|e| e.duel.as_ref().is_some_and(|d| d.id == keep));
        let spot = prince_spot(c.sin);
        let home = P::new(spot.0, spot.1).v();
        let terms = self.prince_terms(c.sin);
        if let Some(d) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).next() {
            d.reset_home();
            d.pos = home;
            d.facing = V::new(0.0, 1.0);
            d.rest = 90;
            d.lore.summoned = !terms.phase_two_adds;
        }
        if let Some(c) = self.court.as_mut() {
            c.phase = 1;
            c.adds_due = terms.adds_at_start;
            c.trail = self.enemies.first().and_then(|e| e.duel.as_ref()).map(|d| d.max_hp as f32).unwrap_or(0.0);
        }
        match c.sin {
            Sin::Greed => self.greed_reset(),
            Sin::Pride => self.pride_reset(),
            Sin::Envy => self.envy_reset(),
            Sin::Wrath => self.wrath_reset(),
            Sin::Sloth => self.sloth_reset(),
            Sin::Gluttony => self.gluttony_reset(),
            Sin::Lust => self.lust_reset(),
        }
        self.refresh_foe_views();
    }

    /// The court each tick: the fallen cleared away, the adds his general's decree sent ahead, the
    /// prince's own doings, the bar's trail catching up, the key taken.
    pub(super) fn update_court(&mut self, dt: f64) {
        if self.enemies.iter().any(|e| e.mode() == EnemyMode::Eaten) {
            self.enemies.retain(|e| e.mode() != EnemyMode::Eaten);
            self.refresh_foe_views();
        }
        let Some(sin) = self.court_sin() else { return };
        // The adds his general's decree sent ahead, as the fight begins.
        let due = self.court.as_ref().is_some_and(|c| c.adds_due) && self.screen == super::Screen::Playing;
        if let Some((id, pos)) = self.prince().map(|d| (d.id, d.pos)).filter(|_| due) {
            if let Some(c) = self.court.as_mut() {
                c.adds_due = false;
            }
            if let Some(d) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == id) {
                d.lore.summoned = true;
            }
            self.raise_adds(sin, id, pos);
        }
        match sin {
            Sin::Greed => self.greed_tick(dt),
            Sin::Pride => self.pride_tick(dt),
            Sin::Envy => self.envy_tick(dt),
            Sin::Wrath => self.wrath_tick(dt),
            Sin::Sloth => self.sloth_tick(dt),
            Sin::Gluttony => self.gluttony_tick(dt),
            Sin::Lust => self.lust_tick(dt),
        }
        let hp = self.prince().map(|d| d.hp.max(0.0) as f32);
        if let (Some(c), Some(hp)) = (self.court.as_mut(), hp) {
            // The pale trail of what the last blows cost, catching up after a moment.
            c.trail = if c.trail > hp { (c.trail - (dt as f32) * 120.0).max(hp) } else { hp };
        }
        self.take_red_key();
    }

    /// The prince of `sin` raises his adds about `at` (`id` is the prince).
    fn raise_adds(&mut self, sin: Sin, id: u32, at: V) {
        match sin {
            Sin::Greed => self.greed_raise(id, at),
            Sin::Pride => self.pride_raise(id, at),
            Sin::Envy => self.envy_raise(id, at),
            Sin::Wrath => self.wrath_raise(id, at),
            Sin::Sloth => self.sloth_raise(id, at),
            Sin::Gluttony => self.gluttony_raise(id, at),
            Sin::Lust => self.lust_raise(id, at),
        }
    }

    /**
     * A prince's health has fallen into `phase` (in his court or in the ring): his roar and its
     * banner, his own doings as the phase begins (Mammon's golems climbing out of the gold as the
     * second begins), and a green cross for a breather.
     */
    pub(super) fn prince_phase(&mut self, i: usize, phase: u8) {
        let Some(du) = self.enemies.get(i).and_then(|e| e.duel.as_deref()) else { return };
        if du.arch != Archetype::Prince {
            return;
        }
        let (id, pos, kind) = (du.id, du.pos, du.breed.kind);
        if kind == crate::logic::enemy::EnemyKind::Krampus {
            // Krampus is no sin's prince: his lair has its own banners and adds (`lair`).
            self.krampus_phase(i, phase);
            return;
        }
        let sin = Sin::of_prince(kind).unwrap_or(Sin::Greed);
        if let Some(c) = self.court.as_mut() {
            c.phase = phase;
        }
        if let Some(b) = self.brawl.as_mut() {
            b.threat.release(id);
        }
        if let Some(d) = self.enemies[i].duel.as_deref_mut() {
            d.token = false;
            d.rest = 75;
        }
        self.audio.play(Sfx::BossRoar);
        self.audio.set_tempo_step(phase as u32 - 1);
        self.view.shake(0.45);
        self.feel_phase();
        self.view.sparkle(&self.level.grid, pos.x, pos.y, 1.4, 0xf2c53d, 40);
        let (sub, color) = match sin {
            Sin::Greed => Self::greed_banner(phase),
            Sin::Pride => Self::pride_banner(phase),
            Sin::Envy => Self::envy_banner(phase),
            Sin::Wrath => Self::wrath_banner(phase),
            Sin::Sloth => Self::sloth_banner(phase),
            Sin::Gluttony => Self::gluttony_banner(phase),
            Sin::Lust => Self::lust_banner(phase),
        };
        self.juice.banner = Some(Banner {
            text: format!("{} - {}", self.boss_name(), phase_name(kind, phase)),
            sub: sub.to_string(),
            color,
            life: 2.6,
            max: 2.6,
        });
        // (His hook sees whether his phase-two adds have come yet; after it, they have.)
        match sin {
            Sin::Greed => self.greed_phase(id, pos, phase),
            Sin::Pride => self.pride_phase(id, pos, phase),
            Sin::Envy => self.envy_phase(id, pos, phase),
            Sin::Wrath => self.wrath_phase(id, pos, phase),
            Sin::Sloth => self.sloth_phase(id, pos, phase),
            Sin::Gluttony => self.gluttony_phase(id, pos, phase),
            Sin::Lust => self.lust_phase(id, pos, phase),
        }
        if let Some(d) = self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == id) {
            d.lore.summoned |= phase >= 2;
        }
        self.drop_breather(pos);
    }

    /// The prince `id`'s phase-two adds have not come yet (his general lives, and no decree or
    /// earlier phase has brought them).
    fn adds_unsummoned(&self, id: u32) -> bool {
        self.enemies.iter().filter_map(|e| e.duel.as_deref()).find(|d| d.id == id).is_some_and(|d| !d.lore.summoned)
    }

    /// A green cross on open floor a few steps from the hero and away from the prince: a breather
    /// between phases.
    pub(super) fn drop_breather(&mut self, prince: V) {
        let hp = self.hero.pos();
        let g = &self.level.grid;
        let ring = self.in_ring().then(crate::logic::gym::ring);
        let best = g
            .cells()
            .filter(|&(x, y, t)| t == Tile::Open && g.is_walkable(x, y, false) && ring.is_none_or(|r| r.contains(P::new(x, y))))
            .map(|(x, y, _)| P::new(x, y))
            .filter(|p| p.v() != hp)
            .max_by(|a, b| {
                let score = |p: &P| {
                    let from_hero = dist_sq(p.v(), hp).sqrt();
                    dist_sq(p.v(), prince).sqrt().min(6.0) - (from_hero - 3.5).abs() * 1.5
                };
                score(a).total_cmp(&score(b)).then(b.y.cmp(&a.y)).then(b.x.cmp(&a.x))
            });
        if let Some(p) = best {
            self.level.grid.set(p.x, p.y, Tile::Charge);
            self.view.add_charge(&self.level.grid, p.x, p.y);
            self.view.sparkle(&self.level.grid, p.x as f64, p.y as f64, 0.4, 0x5cff6a, 20);
            self.floater(p.x as f64, p.y as f64, "HEALTH", 1.6, Some(0x5cff6a));
        }
    }

    /// The prince falls in his court: everything a boss's death brings, and his key.
    pub(super) fn prince_slain(&mut self, at: V) {
        if let Some(c) = self.court.as_mut() {
            c.slain = true;
            c.trail = 0.0;
        }
        if let Some(b) = self.brawl.as_mut() {
            b.traps.clear();
            b.bolts.clear();
            b.held = None;
        }
        match self.court_sin().unwrap_or(Sin::Greed) {
            Sin::Greed => self.greed_slain(at),
            Sin::Pride => self.pride_slain(at),
            Sin::Envy => self.envy_slain(at),
            Sin::Wrath => self.wrath_slain(at),
            Sin::Sloth => self.sloth_slain(at),
            Sin::Gluttony => self.gluttony_slain(at),
            Sin::Lust => self.lust_slain(at),
        }
        self.boss_slain(at);
    }
}

/// What the courts' tests share: a game standing in a sin's court, its prince, and time passing.
#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use crate::game::{save, Screen};

    /// A game in the court of `sin`, playing, the hero free to walk.
    pub fn court(sin: Sin) -> Game {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test-court"));
        let mut g = Game::new();
        g.map_jump(save::Stage::Court(sin));
        g.enter(Screen::Playing);
        g.hero.free_move = true;
        g
    }

    /// The prince of the court `g` stands in.
    pub fn prince(g: &mut Game) -> &mut Duelist {
        let id = g.court.as_ref().unwrap().prince;
        g.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == id).unwrap()
    }

    /// `secs` of play.
    pub fn run(g: &mut Game, secs: f64) {
        for _ in 0..(secs / crate::app::STEP) as usize {
            g.update(crate::app::STEP);
        }
    }
}
