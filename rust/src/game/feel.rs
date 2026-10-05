//! The feel of the brawl (WARBAND.md §4): what each outcome of a fight does to time, the camera,
//! the debris and the bodies. The brawl calls these where the outcome is decided, so the freeze
//! lands on the frame it always did; the numbers are `logic::juice`'s tables, and the view's
//! springs, chips and shatters draw from the presentation's own rng (rule I5).

use super::Game;
use crate::audio::voices::matter_of as sound_matter;
use crate::audio::{HitSound, ShieldTier, SoundMatter};
use crate::logic::combat::fsm::Effect;
use crate::logic::combat::Archetype;
use crate::logic::enemy::EnemyKind;
use crate::logic::juice::{
    matter_of, DilationKind, Stop, Trauma, CHIPS_FINISHER, CHIPS_LIGHT, FOV_FINISHER, FOV_QUAKE, KICK_BREAK, KICK_FINISHER, KICK_STRUCK,
    NARROW_HEALTH,
};
use crate::logic::types::{dist_sq, V};

/// How a foe's strike met the hero, as the feel needs it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Met {
    Evaded,
    Parried,
    Blocked,
    GuardBroken,
    Struck,
}

impl Game {
    /// Duelist `id` is a prince.
    fn is_prince(&self, id: u32) -> bool {
        self.enemies.iter().filter_map(|e| e.duel.as_deref()).any(|d| d.id == id && d.arch == Archetype::Prince)
    }

    /// Duelist `id`'s body (a witch's, for a bolt whose caster is gone).
    fn body_of(&self, id: u32) -> EnemyKind {
        self.enemies.iter().filter_map(|e| e.duel.as_deref()).find(|d| d.id == id).map(|d| d.breed.kind).unwrap_or(EnemyKind::Witch)
    }

    /**
     * His blow lands on duelist `i` (a `kind`, at `at`), the way `dir` goes: its freeze (the move's
     * own, longer on a break), the camera's jolt, chips of its body, and its recoil.
     */
    #[allow(clippy::too_many_arguments)]
    pub(super) fn feel_hero_hit(&mut self, i: usize, kind: EnemyKind, effect: Effect, finisher: bool, dir: V, at: V, hitstop: u16) {
        self.freeze(Stop::Blow(hitstop));
        let id = self.enemies[i].duel.as_ref().map(|d| d.id).unwrap_or(0);
        let matter = matter_of(kind);
        match effect {
            Effect::Broken => {
                self.freeze(Stop::Break);
                self.view.trauma(Trauma::Break);
                self.view.kick(dir, KICK_BREAK);
                self.view.fov_punch(FOV_FINISHER.0, FOV_FINISHER.1);
                self.view.chips(&self.level.grid, at, dir, matter, CHIPS_FINISHER);
                self.view.recoil(id, dir);
            }
            Effect::Armored => {}
            _ => {
                if finisher {
                    self.view.trauma(Trauma::Finisher);
                    self.view.kick(dir, KICK_FINISHER);
                    self.view.fov_punch(FOV_FINISHER.0, FOV_FINISHER.1);
                } else {
                    self.view.trauma(Trauma::Blow);
                }
                let n = if finisher { CHIPS_FINISHER } else { CHIPS_LIGHT };
                self.view.chips(&self.level.grid, at, dir, matter, n);
                self.view.recoil(id, dir);
            }
        }
    }

    /// Duelist `i` is slain, struck the way `dir` goes: its body breaks into its voxels, the kill's
    /// freeze, and the release beat when it was the last of its pack.
    pub(super) fn feel_slain(&mut self, i: usize, dir: V, last_of_pack: bool) {
        if let Some(d) = self.enemies[i].duel.as_deref() {
            let (kind, at) = (d.breed.kind, d.pos);
            self.audio.hit(HitSound::Kill { streak: 0 }, sound_matter(kind), kind, Some(at));
        }
        self.view.shatter(i, dir);
        self.view.trauma(Trauma::Kill);
        if last_of_pack {
            self.freeze(Stop::LastOfPack);
            self.dilate(DilationKind::LastOfPack);
        } else {
            self.freeze(Stop::Kill);
        }
    }

    /// A foe's strike (`by`, a blow of `hitstop` freeze frames for `dmg`, coming the way `dir` goes)
    /// met him as `met`.
    pub(super) fn feel_struck(&mut self, met: Met, by: u32, dir: V, dmg: f64, hitstop: u16) {
        let hp = self.hero.pos();
        let kind = self.body_of(by);
        let from = crate::logic::types::V::new(hp.x - dir.x, hp.y - dir.y);
        let sound = match met {
            Met::Evaded => None,
            Met::Parried => Some(HitSound::Parry { prince: self.is_prince(by) }),
            Met::Blocked => Some(HitSound::Block(ShieldTier::of_level(self.hero.shield.map(|s| s.tier).unwrap_or(0)))),
            Met::GuardBroken => Some(HitSound::GuardBroken),
            Met::Struck => Some(HitSound::HeroStruck(self.hero_kind)),
        };
        if let Some(sound) = sound {
            let matter = if met == Met::Struck { SoundMatter::Flesh } else { SoundMatter::Iron };
            let at = (met != Met::Struck).then_some(from);
            self.audio.hit(sound, matter, kind, at);
        }
        match met {
            Met::Evaded => {
                if (self.hero.hp as f64) < self.hero.max_hp as f64 * NARROW_HEALTH {
                    self.dilate(DilationKind::NarrowEscape);
                }
            }
            Met::Parried => {
                let prince = self.is_prince(by);
                self.freeze(Stop::Parry { prince });
                if prince {
                    self.dilate(DilationKind::PrinceParry);
                }
                self.view.sparks_block(&self.level.grid, hp, dir);
                self.view.shield_jolt();
                self.view.hero_parried();
            }
            Met::Blocked => {
                self.freeze(Stop::Blocked);
                self.view.sparks_block(&self.level.grid, hp, dir);
                self.view.shield_jolt();
            }
            Met::GuardBroken => {
                self.freeze(Stop::GuardBroken);
                self.view.trauma(Trauma::GuardBroken);
                self.view.shield_jolt();
            }
            Met::Struck => {
                self.freeze(Stop::Struck(hitstop));
                self.view.trauma(Trauma::Struck { dmg });
                self.view.kick(dir, KICK_STRUCK);
            }
        }
    }

    /// The ground shaken at `at` (a kanabō, a hoard slam, a frost slam, a burrow bursting up).
    pub(super) fn feel_quake(&mut self, at: V) {
        let dist = dist_sq(at, self.hero.pos()).sqrt();
        self.view.trauma(Trauma::Quake { dist });
        if dist < 4.0 {
            self.view.fov_punch(FOV_QUAKE.0, FOV_QUAKE.1);
        }
    }

    /// A prince's phase falls.
    pub(super) fn feel_phase(&mut self) {
        self.dilate(DilationKind::PrinceFalls);
        self.view.trauma(Trauma::Trophy);
    }
}
