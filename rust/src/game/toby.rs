//! Toby's trail in play (B08, `logic::tale`): one of his things lies in every realm, a few steps
//! from where the hero comes in, glowing a little. He picks it up by walking over it, as he would a
//! coin, and keeps it with the run (the Warband's save: Continue keeps it; a new adventure begins
//! without). It is a flag, nothing more: no stat, no path, no foe knows of it. The pause shows the
//! seven, the bestiary has Toby's page, the scene after each prince speaks of the one just found,
//! and Toby knows them all when he is free.

use super::{Game, Screen};
use crate::audio::Sfx;
use crate::logic::realms::Run;
use crate::logic::tale::{Token, Tokens, TOBY_COLOR};
use crate::logic::types::{Tile, P};

/// How near (tiles) he must come to take it.
const REACH: f64 = 0.75;
/// It lies this many steps from where he comes in, along the ground he can walk.
const NEAR: usize = 7;
const FAR: usize = 16;

impl Game {
    /// Toby's things found so far this run.
    pub fn tokens(&self) -> Tokens {
        Tokens(self.warband.save.toby)
    }

    /// Lays this realm's token on its ground, if it is still to be found (not in a court, the hub
    /// or anywhere else): on open floor a few steps along the ground from where he comes in, chosen
    /// by the realm alone (no roll of the game's dice), off every pickup.
    pub(super) fn lay_token(&mut self) {
        self.token = None;
        let Some(sin) = self.realm else { return };
        let token = Token::of(sin);
        if self.level.data.boss || self.tokens().has(token) {
            return;
        }
        self.token = token_spot(self, sin.index() as u32).map(|p| (token, p));
    }

    /// He walks over it: it is his to keep.
    pub(super) fn update_token(&mut self) {
        let Some((token, at)) = self.token else { return };
        if self.screen != Screen::Playing || !self.hero.alive {
            return;
        }
        let hp = self.hero.pos();
        let (dx, dy) = (hp.x - at.x as f64, hp.y - at.y as f64);
        if dx * dx + dy * dy > REACH * REACH {
            return;
        }
        self.token = None;
        self.find_token(token);
        let (x, y) = (at.x as f64, at.y as f64);
        self.view.sparkle(&self.level.grid, x, y, 0.8, TOBY_COLOR, 30);
        self.view.sparkle(&self.level.grid, x, y, 0.6, 0xfff4d8, 16);
        self.audio.play(Sfx::Soul);
        self.floater(x, y - 1.2, token.found(), 2.6, Some(TOBY_COLOR));
        let n = self.tokens().count();
        self.floater(x, y - 0.6, format!("TOBY'S TRAIL  {n} OF 7"), 2.6, Some(0xfff4d8));
        // The boy knows it at once (B07).
        let hero = self.hero_kind;
        self.boy_says(super::counsel::Speaker::Boy(hero), token.note().to_string());
    }

    /// The boy shows in the warrior's soul-glow (B07): where he falls, or at a shrine.
    pub(super) fn boy_glow(&mut self, fall: bool) {
        let hero = self.hero_kind;
        self.view.boy_glow = Some(crate::render::view::toby::BoyGlow {
            kid: crate::logic::tale::boy_of(hero),
            tint: crate::logic::tale::soul_color(hero),
            since: self.time,
            fall,
        });
    }

    /// Capture hook (`after:S:N:T`): the scene at Hızır's after `sin`'s prince, at beat `beat`, `t`
    /// seconds in, its line shown in full.
    pub fn debug_after(&mut self, sin: crate::logic::sins::Sin, beat: usize, t: f64) {
        if !matches!(self.story.as_ref().map(|s| s.scene), Some(crate::logic::story::Scene::After(s)) if s == sin) {
            self.load_court(sin);
            self.play_after(sin);
        }
        self.story_jump(beat, t);
    }

    /// Capture hook (`toby:BITS`): Toby's things held.
    pub fn debug_toby(&mut self, bits: u8) {
        self.warband.save.toby = bits & 0x7f;
        self.view.ending_ball = self.tokens().has(Token::Ball);
    }

    /// Capture hook (`token`): the hero a step from this realm's token, facing it.
    pub fn debug_token(&mut self) {
        if let Some((_, at)) = self.token {
            let p = [P::new(at.x, at.y + 2), P::new(at.x + 2, at.y), P::new(at.x - 2, at.y), P::new(at.x, at.y - 2)]
                .into_iter()
                .find(|p| self.level.grid.is_walkable(p.x, p.y, false))
                .unwrap_or(at);
            self.hero.place(p);
        }
    }

    /// Capture hook (`boyglow:1` or `boyglow:0`): the boy shows as the warrior falls, or at a shrine.
    pub fn debug_boy_glow(&mut self, fall: bool) {
        if fall {
            self.hero.alive = false;
        }
        self.boy_glow(fall);
    }

    /// One of his things is the hero's (kept with the run's Warband).
    pub(super) fn find_token(&mut self, token: Token) {
        if self.tokens().has(token) {
            return;
        }
        self.warband.save.toby = self.tokens().with(token).0;
        self.save_warband();
    }
}

/// Where a realm's token lies: open floor between `NEAR` and `FAR` steps from where he comes in
/// (walking: no climbing more than a step), not on anything else he might pick up; the first by a
/// hash of the place and the realm.
fn token_spot(g: &Game, salt: u32) -> Option<P> {
    let grid = &g.level.grid;
    let start = g.level.hero_spawn;
    if !grid.in_bounds(start.x, start.y) {
        return None;
    }
    let (w, h) = (grid.width, grid.height);
    let mut dist = vec![usize::MAX; (w * h) as usize];
    let idx = |p: P| (p.y * w + p.x) as usize;
    let mut queue = std::collections::VecDeque::from([start]);
    dist[idx(start)] = 0;
    let taken: Vec<P> = g.diamonds.iter().map(|d| d.tile).chain(g.potions.iter().map(|p| p.tile)).collect();
    let mut best: Option<(u32, P)> = None;
    while let Some(p) = queue.pop_front() {
        let d = dist[idx(p)];
        if d > FAR {
            break;
        }
        // (On the Burning Field, only on the isles that never drift: a slab may carry it into the lava.)
        let firm = match &g.run {
            Some(Run::Wrath(field)) => field.on_isle(p.v()),
            _ => true,
        };
        if d >= NEAR && firm && grid.get(p.x, p.y) == Tile::Open && !taken.contains(&p) && grid.is_walkable(p.x, p.y, false) {
            let key = hash(p.x as u32, p.y as u32, salt);
            if best.is_none_or(|(k, _)| key < k) {
                best = Some((key, p));
            }
        }
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let q = P::new(p.x + dx, p.y + dy);
            if !grid.in_bounds(q.x, q.y) || dist[idx(q)] != usize::MAX || !grid.is_walkable(q.x, q.y, false) {
                continue;
            }
            if (grid.elevation(q.x, q.y) - grid.elevation(p.x, p.y)).abs() > 1 {
                continue;
            }
            dist[idx(q)] = d + 1;
            queue.push_back(q);
        }
    }
    best.map(|(_, p)| p)
}

fn hash(x: u32, y: u32, salt: u32) -> u32 {
    let mut k = x.wrapping_mul(0x9e37_79b1) ^ y.wrapping_mul(0x85eb_ca77) ^ salt.wrapping_mul(0xc2b2_ae3d);
    k ^= k >> 15;
    k = k.wrapping_mul(0x2c1b_3c6d);
    k ^ (k >> 12)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::save::Stage;
    use crate::logic::sins::SINS;
    use crate::logic::tale::TOKENS;

    /// Every realm has its token on walkable ground near the way in; walking onto it takes it, and
    /// nothing else of the game changes with it.
    #[test]
    fn every_realm_lays_its_token_and_walking_over_it_takes_it() {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test-toby"));
        for sin in SINS {
            let mut g = Game::new();
            g.map_jump(Stage::Realm(sin));
            let (token, at) = g.token.unwrap_or_else(|| panic!("{sin:?} has no token"));
            assert_eq!(token, Token::of(sin));
            assert!(g.level.grid.is_walkable(at.x, at.y, false), "{sin:?}: on walkable floor");
            assert_ne!(at, g.level.hero_spawn);
            g.enter(Screen::Playing);
            g.update(crate::app::STEP);
            let (hp, lives, silver, score) = (g.hero.hp, g.score.lives, g.progress.silver, g.score.score);
            g.hero.place(at);
            g.update(crate::app::STEP);
            assert!(g.tokens().has(token), "{sin:?}: taken");
            assert!(g.token.is_none());
            assert_eq!(
                (g.hero.hp, g.score.lives, g.progress.silver, g.score.score),
                (hp, lives, silver, score),
                "{sin:?}: a flag, nothing more"
            );
            // The court has none, and a realm found is not laid again.
            g.map_jump(Stage::Court(sin));
            assert!(g.token.is_none());
            let kept = g.warband.save.toby;
            g.map_jump(Stage::Realm(sin));
            assert!(g.token.is_none(), "{sin:?}: found once");
            assert_eq!(g.warband.save.toby, kept);
        }
        assert_eq!(TOKENS.len(), 7);
    }

    /// After a prince, the scene at Hızır's (B07): it speaks of the token (Quitmer fetched it if it
    /// was walked past), plays through with Enter or is skipped with Esc, and the wares follow.
    #[test]
    fn after_a_prince_the_scene_at_hizirs_plays_and_the_wares_follow() {
        use crate::input::Action;
        use crate::logic::story::Scene;
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test-after"));
        let step = crate::app::STEP;
        for (k, sin) in SINS.into_iter().enumerate() {
            let mut g = Game::new();
            g.map_jump(Stage::Court(sin));
            g.court.as_mut().unwrap().slain = true;
            g.enter(Screen::Won);
            g.input.inject(Action::Confirm);
            g.update(step);
            assert_eq!(g.screen, Screen::Story, "{sin:?}");
            assert_eq!(g.story.as_ref().map(|s| s.scene), Some(Scene::After(sin)));
            assert!(g.tokens().has(Token::of(sin)), "{sin:?}: Quitmer fetched it");
            let mut t = 0.0;
            while g.screen == Screen::Story && t < 90.0 {
                // Half of them skipped at once, half played through a press at a time.
                if k % 2 == 0 && t > 1.0 {
                    g.input.inject(Action::Pause);
                } else if ((t / step) as u64).is_multiple_of(60) {
                    g.input.inject(Action::Confirm);
                }
                g.update(step);
                t += step;
            }
            assert_eq!(g.screen, Screen::Shop, "{sin:?}: the wares after it ({t:.0} s)");
            if k % 2 == 1 {
                assert!(t > 10.0 && t < 60.0, "{sin:?}: a short scene played through, {t:.0} s");
            }
        }
    }
}
