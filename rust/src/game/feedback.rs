//! Feedback and score: floating text, points and kill streaks, silver in the purse, the screen's
//! juice (banners, flashes, flying coins) and the effects that tick while the world moves.

use super::Game;
use crate::logic::heroes::HeroKind;
use crate::logic::types::{dist_sq, manhattan};

/// Kills closer together than this count as a streak.
pub(super) const STREAK_SECONDS: f64 = 2.5;

/// Every third kill in a streak is BRUTAL: a big shake and double points for a while.
pub const BRUTAL_STREAK: u32 = 3;

/**
 * What a streak is called, tier by tier: at 3 kills, 6, 9, and 12 or more.
 * Each call picks one of its tier's words (never the one just shouted), so a
 * long fight does not keep saying BRUTAL; at the top, the hero's own war cry
 * joins them.
 */
pub const STREAK_CALLS: [&[&str]; 4] = [
    &["BRUTAL!", "SAVAGE!", "RUTHLESS!", "VICIOUS!", "BLOODY!"],
    &["MERCILESS!", "FEROCIOUS!", "BLOODBATH!", "RELENTLESS!"],
    &["RAMPAGE!", "MASSACRE!", "UNSTOPPABLE!", "CARNAGE!"],
    &["GODLIKE!", "LEGENDARY!", "ANNIHILATION!"],
];

/// Each tier's colour: red, then orange, violet and gold.
pub(super) const STREAK_COLORS: [u32; 4] = [0xff3a2a, 0xff7a1a, 0xc04aff, 0xffd23a];

/// The streak's tier: 0 at 3 kills, 1 at 6, 2 at 9, 3 from 12 on.
pub fn streak_tier(streak: u32) -> usize {
    ((streak / BRUTAL_STREAK).saturating_sub(1) as usize).min(STREAK_CALLS.len() - 1)
}

/// A hero's own cry, added to the top tier's words.
pub(super) fn war_cry(hero: HeroKind) -> &'static str {
    match hero {
        HeroKind::Ragnar => "BERSERKER!",
        HeroKind::Mehmed => "FURY OF THE SULTAN!",
        HeroKind::Miyamoto => "WAY OF THE SWORD!",
    }
}

/// How long a BRUTAL streak doubles every point.
pub const DOUBLE_POINTS_SECONDS: f64 = 10.0;

/// A collected nugget takes this long to fly into the HUD counter.
pub const COIN_FLIGHT_SECONDS: f32 = 0.55;

pub struct Floater {
    pub x: f64,
    pub y: f64,
    pub text: String,
    pub life: f64,
    pub max_life: f64,
    pub color: Option<u32>,
}

/// A nugget on its way from the world to the HUD counter, in screen space.
pub struct FlyingCoin {
    pub from: (f32, f32),
    pub t: f32,
}

/// A big centered announcement ("10 NUGGETS").
pub struct Banner {
    pub text: String,
    pub sub: String,
    pub color: u32,
    pub life: f32,
    pub max: f32,
}

/// Presentation state for rewards and feedback; none of it affects the rules.
#[derive(Default)]
pub struct Juice {
    /// The score as displayed, rolling up toward the real one.
    pub score_shown: f64,
    /// 1 when a nugget lands in the counter, fading to 0.
    pub key_punch: f32,
    /// The health bar's trailing chunk after a hit, in HP.
    pub hp_trail: f32,
    /// Flash on the health bar: color and 1..0 strength.
    pub hp_flash: f32,
    pub hp_flash_color: u32,
    /// Red screen-edge flash when hurt.
    pub hurt: f32,
    pub coins: Vec<FlyingCoin>,
    pub banner: Option<Banner>,
    /// Seconds "GO!" has left on screen.
    pub go: f32,
    /// Black over everything, 1..0, fading in after a level change.
    pub fade: f32,
    /// The boss health bar's trailing chunk, in HP.
    pub boss_trail: f32,
    /// 1 when coins come in, fading: the purse counter jumps.
    pub coin_punch: f32,
    /// 1 on a BRUTAL streak, fading: a hot flash over the screen.
    pub brutal: f32,
}

impl Game {
    pub(super) fn floater(&mut self, x: f64, y: f64, text: impl Into<String>, life: f64, color: Option<u32>) {
        self.floaters.push(Floater { x, y, text: text.into(), life, max_life: life, color });
    }

    /// Adds points, doubled while a BRUTAL streak's reward runs. Returns what was added.
    pub(super) fn award(&mut self, points: u32) -> u32 {
        let points = if self.double_points > 0.0 { points * 2 } else { points };
        self.score.add(points);
        points
    }

    /**
     * A third kill in quick succession: BRUTAL. The screen jolts and flashes,
     * the world holds for a beat, and every point is doubled for a while.
     */
    pub(super) fn brutal(&mut self) {
        let tier = streak_tier(self.kill_streak);
        let mut calls: Vec<&'static str> = STREAK_CALLS[tier].to_vec();
        if tier == STREAK_CALLS.len() - 1 {
            calls.push(war_cry(self.hero_kind));
        }
        calls.retain(|&c| c != self.last_call);
        let call = self.rng.pick(&calls);
        self.last_call = call;
        let color = STREAK_COLORS[tier];
        self.double_points = DOUBLE_POINTS_SECONDS;
        // The longer the streak, the harder the jolt and the longer the world holds.
        self.view.shake(0.85 + 0.1 * tier as f32);
        self.hitstop = 0.14 + 0.02 * tier as f64;
        self.juice.brutal = 1.0;
        self.audio.play_brutal(tier as u32);
        self.juice.banner = Some(Banner {
            text: call.into(),
            sub: format!("x{} KILL STREAK   DOUBLE POINTS FOR {} SECONDS", self.kill_streak, DOUBLE_POINTS_SECONDS as u32),
            color,
            life: 2.2 + 0.2 * tier as f32,
            max: 2.2 + 0.2 * tier as f32,
        });
        let hp = self.hero.pos();
        self.view.sparkle(&self.level.grid, hp.x, hp.y, 0.8 + 0.1 * tier as f32, color, 30 + 10 * tier);
    }

    /// Silver in the purse, and the counter jumps.
    pub(super) fn earn(&mut self, coins: u32) {
        self.progress.silver += coins;
        self.juice.coin_punch = 1.0;
    }

    pub(super) fn update_effects(&mut self, dt: f64) {
        self.floaters.retain_mut(|f| {
            f.life -= dt;
            f.y -= dt * 0.8;
            f.life > 0.0
        });
        let hp = self.hero.pos();
        let nearest = self.enemies.iter().filter(|e| e.is_dangerous()).map(|e| manhattan(e.pos(), hp)).fold(f64::INFINITY, f64::min);
        let mut threat = if nearest.is_infinite() { 0.0 } else { (1.0 - nearest / 10.0).max(0.0) };
        if let Some(b) = self.boss.as_ref().filter(|b| b.active()) {
            // A boss fight is never calm; the closer the eye, the harder the drums.
            threat = 0.55 + 0.45 * (1.0 - dist_sq(b.pos, hp).sqrt() / 10.0).max(0.0);
        }
        self.audio.set_threat(threat);
        let frac = self.hero.hp as f64 / self.hero.max_hp as f64;
        self.audio.set_danger(if frac < 0.35 { 1.0 - frac / 0.35 } else { 0.0 });
        self.streak_timer -= dt;
        self.double_points = (self.double_points - dt).max(0.0);
    }

    /// Rewards and feedback that live only on screen: rolling score, flying coins, flashes.
    pub(super) fn update_juice(&mut self, dt: f64) {
        let dtf = dt as f32;
        let j = &mut self.juice;
        let target = self.score.score as f64;
        if j.score_shown < target {
            j.score_shown = (j.score_shown + (target - j.score_shown) * (dt * 9.0).min(1.0) + dt * 40.0).min(target);
        } else {
            j.score_shown = target;
        }
        let hp = self.hero.hp as f32;
        if j.hp_trail > hp {
            j.hp_trail = (j.hp_trail - dtf * 60.0).max(hp);
        } else {
            j.hp_trail = hp;
        }
        j.key_punch = (j.key_punch - dtf * 3.5).max(0.0);
        j.hp_flash = (j.hp_flash - dtf * 3.0).max(0.0);
        j.hurt = (j.hurt - dtf * 2.2).max(0.0);
        j.go = (j.go - dtf).max(0.0);
        j.fade = (j.fade - dtf * 1.6).max(0.0);
        j.coin_punch = (j.coin_punch - dtf * 3.0).max(0.0);
        j.brutal = (j.brutal - dtf * 1.8).max(0.0);
        let boss_hp = self.boss.as_ref().map(|b| b.hp as f32).unwrap_or(0.0);
        if j.boss_trail > boss_hp {
            j.boss_trail = (j.boss_trail - dtf * 12.0).max(boss_hp);
        } else {
            j.boss_trail = boss_hp;
        }
        let mut landed = 0;
        j.coins.retain_mut(|c| {
            c.t += dtf / COIN_FLIGHT_SECONDS;
            if c.t >= 1.0 {
                landed += 1;
                false
            } else {
                true
            }
        });
        if landed > 0 {
            j.key_punch = 1.0;
        }
        if let Some(b) = &mut j.banner {
            b.life -= dtf;
            if b.life <= 0.0 {
                j.banner = None;
            }
        }
    }
}
