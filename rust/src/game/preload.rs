//! What a level can bring into the fight, made ready while it loads (no stalls mid-fight; the
//! renderer's half is `WorldView::prepare_rigs`): every kind its dealer, its waves, its general,
//! its rivals and its foes' own summons can produce.

use super::Game;
use crate::logic::enemy::{enemy_specs, EnemyKind};
use crate::logic::realms::{roster, Run};

/// What a foe of `kind` can bring into the fight by itself: its illusions, its raised dead, its
/// conjured minions, a prince's adds.
pub(crate) fn brings(kind: EnemyKind) -> &'static [EnemyKind] {
    use EnemyKind as K;
    match kind {
        K::Witch => &[K::Minion],
        K::Kitsune => &[K::Kitsune],
        K::Hel => &[K::Skeleton],
        K::Mammon => &[K::GoldGolem],
        K::Lucifer => &[K::Reflection],
        K::Leviathan => &[K::Serpentling],
        K::Amon => &[K::Raven],
        K::Belphegor => &[K::Nightmare],
        K::Beelzebub => &[K::FlySwarm],
        K::Asmodeus => &[K::Ellepige],
        K::Krampus => &[K::Percht],
        _ => &[],
    }
}

/// `kinds` with everything they bring, and what that brings in turn, each once, in order.
pub(crate) fn with_summons(mut kinds: Vec<EnemyKind>) -> Vec<EnemyKind> {
    let mut i = 0;
    while i < kinds.len() {
        for &k in brings(kinds[i]) {
            if !kinds.contains(&k) {
                kinds.push(k);
            }
        }
        i += 1;
    }
    kinds
}

impl Game {
    /// Every kind of foe this level can show while it lives: those standing in it now; a dungeon's
    /// reinforcements; a realm's or the vaults' whole roster (the packs, the general's musters and
    /// patrols, a mimic's biter), its own kind and its rivals; the Pit's waves on every level from
    /// this one down; and whatever each of those brings (`brings`).
    pub(crate) fn level_kinds(&self) -> Vec<EnemyKind> {
        let mut kinds: Vec<EnemyKind> = self.enemies.iter().map(|e| e.kind()).collect();
        let theme = self.theme();
        let realm = self.vault.is_some() || self.run.is_some();
        if realm {
            let tier = self.realm_tier();
            kinds.extend(roster(tier).into_iter().map(|(k, _)| k));
            // (The general's puppet line, the witches' minions.)
            kinds.push(EnemyKind::Minion);
            kinds.extend(self.ledger().live().map(|r| r.kind));
            kinds.extend(self.general.as_ref().map(|g| g.body));
            match &self.run {
                Some(Run::Sloth(_)) => kinds.push(EnemyKind::Ganglot),
                // (A mimic's biter in the feast is a gaki too.)
                Some(Run::Gluttony(_)) => kinds.push(EnemyKind::Gaki),
                // (The elf-maids, and the fox that watches from the brambles.)
                Some(Run::Lust(_)) => kinds.extend([EnemyKind::Ellepige, EnemyKind::Kitsune]),
                _ => {}
            }
            // A mimic's biter in the feast is a gaki.
            kinds.push(EnemyKind::Gaki);
        } else if let Some(p) = &self.pit {
            for level in p.level..=crate::logic::pit::LEVELS {
                kinds.extend(crate::logic::pit::arrivals(level).into_iter().map(|a| a.kind));
            }
        } else if !self.level.data.boss && !theme.is_hub() && !theme.is_gym() && !theme.is_pit() {
            // A dungeon's reinforcements: its stage's own foes (each cycle's kind is one of them, or
            // the stage's own in its place: `update_spawns`).
            kinds.extend(enemy_specs(&self.level.grid, self.foe_stage()).into_iter().map(|s| s.kind));
        }
        let mut seen = Vec::new();
        kinds.retain(|k| {
            let new = !seen.contains(k);
            seen.push(*k);
            new
        });
        with_summons(kinds)
    }

    /// Makes ready, as the level loads, every rig it can show (`level_kinds`, the hero's own):
    /// meshed now, sent to the GPU before the loading screen lifts, kept until the next level.
    pub(crate) fn prepare_level(&mut self) {
        if std::env::var_os("VH_NO_PRELOAD").is_some() {
            return;
        }
        let kinds = self.level_kinds();
        self.view.prepare_rigs(&kinds, self.hero_kind, &self.hero);
        crate::render::hud::warm(&kinds);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::parse_script;
    use crate::driver::Driver;
    use crate::game::Game;

    /// Plays `script` for `secs` with the expert bot, and the kinds of foe that came into the fight
    /// that the level had not made ready as it loaded (none, if the preload covers them).
    fn unready(script: &str, secs: f64) -> (Vec<EnemyKind>, Vec<EnemyKind>) {
        let mut g = Game::new();
        g.reseed(3);
        g.tour_pending = false;
        g.voices.deceit.off = true;
        let mut d = Driver::new(parse_script(script));
        let mut seen: Vec<EnemyKind> = Vec::new();
        let mut t = 0.0;
        while t < secs {
            d.run(&mut g, t + 0.5);
            t += 0.5;
            for e in &g.enemies {
                if !seen.contains(&e.kind()) {
                    seen.push(e.kind());
                }
            }
        }
        eprintln!("{script}: {seen:?}");
        let missed = seen.iter().copied().filter(|&k| !g.view.prepared(k)).collect();
        (missed, seen)
    }

    #[test]
    fn what_each_kind_brings_is_made_ready_with_it() {
        let all = with_summons(vec![EnemyKind::Hel, EnemyKind::Witch, EnemyKind::Krampus]);
        assert_eq!(
            all,
            vec![EnemyKind::Hel, EnemyKind::Witch, EnemyKind::Krampus, EnemyKind::Skeleton, EnemyKind::Minion, EnemyKind::Percht]
        );
        assert_eq!(with_summons(vec![EnemyKind::Kitsune]), vec![EnemyKind::Kitsune], "its illusions are its own kind");
    }

    #[test]
    fn the_vaults_make_ready_every_foe_their_packs_and_general_bring() {
        let (missed, seen) = unready("0.1:realm:4,0.5:bot:2", 60.0);
        assert!(seen.len() >= 4, "{seen:?}");
        assert!(missed.is_empty(), "came unready: {missed:?} of {seen:?}");
    }

    #[test]
    fn a_realm_makes_ready_its_own_kind_and_its_dealers() {
        // Sloth's mire, where the dead climb out of the bog as he goes.
        let (missed, seen) = unready("0.1:sins:17,0.2:realm:3,0.6:bot:2", 60.0);
        assert!(missed.is_empty(), "came unready: {missed:?} of {seen:?}");
    }

    #[test]
    fn a_court_makes_ready_its_prince_and_what_he_raises() {
        // (Wounded down through his phases, where his adds come.)
        for court in [4, 2, 0, 1] {
            let (missed, seen) = unready(&format!("0.1:court:{court},0.6:bot:2,4:wound:200,12:wound:200,20:wound:200"), 40.0);
            assert!(seen.len() >= 2, "court {court}: his adds came: {seen:?}");
            assert!(missed.is_empty(), "court {court}: came unready: {missed:?} of {seen:?}");
        }
    }

    #[test]
    fn the_pit_makes_ready_every_level_below_as_it_is_entered() {
        let mut g = Game::new();
        g.load_pit(2);
        for level in 2..=crate::logic::pit::LEVELS {
            for a in crate::logic::pit::arrivals(level) {
                assert!(g.view.prepared(a.kind), "level {level}: {:?}", a.kind);
            }
        }
        // And in play: the sixth level's fight run on into the seventh's.
        let (missed, seen) = unready("0.1:pit:6,0.3:bot:2,8:pitclock:112", 40.0);
        assert!(seen.contains(&EnemyKind::Surtr) || seen.contains(&EnemyKind::FrostGiant), "{seen:?}");
        assert!(missed.is_empty(), "came unready: {missed:?} of {seen:?}");
    }

    #[test]
    fn the_lair_makes_ready_krampus_and_his_perchten() {
        let (missed, seen) = unready("0.1:lair,0.5:bot:2,4:wound:300,12:wound:300,20:wound:300", 40.0);
        assert!(seen.contains(&EnemyKind::Percht), "{seen:?}");
        assert!(missed.is_empty(), "came unready: {missed:?} of {seen:?}");
        let mut g = Game::new();
        g.load_lair();
        assert!(g.view.prepared(EnemyKind::Percht));
    }

    #[test]
    fn a_new_level_lets_the_last_ones_rigs_go() {
        let mut g = Game::new();
        g.load_pit(7);
        let (gen, pit) = (g.view.resident().gen, g.view.resident().meshes.len());
        assert!(g.view.prepared(EnemyKind::Surtr));
        g.load_level(0);
        assert!(g.view.resident().gen > gen);
        assert!(!g.view.prepared(EnemyKind::Surtr), "the Pit's giants are not the Garden's");
        assert!(g.view.resident().meshes.len() < pit);
    }
}
