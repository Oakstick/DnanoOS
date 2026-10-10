//! B78: the leap's untouchable frames, shown on the hero. Since the evade is the jump (the same
//! leap and the same `jump` clip everywhere), the frames a blow passes under him show as a soft
//! pale glow on him and a short trail of faint afterimages along the leap: subtle, but readable
//! from either camera. Near white, it reads the same under every colour-blind palette and stays
//! apart from the fight's danger and safety colours, from the haste trail's gold and from the
//! potions' auras. A bare leap (out of a recovery or a block's stun) has neither.

use super::*;
use crate::logic::combat::State;

/// Seconds between afterimages while he is untouchable, how many are kept, and how fast each fades.
const TRAIL_EVERY: f32 = 0.05;
const TRAIL_MAX: usize = 3;
const TRAIL_FADE: f32 = 4.5;
/// How fast the glow comes on as he takes off, and goes as the untouchable frames end.
const GLOW_IN: f32 = 14.0;
const GLOW_OUT: f32 = 8.0;
/// The glow's colour and strength, and the afterimages' opacity.
const GLOW: u32 = 0xdce8ff;
const GLOW_STRENGTH: f32 = 0.32;
const TRAIL_ALPHA: f32 = 0.22;

/// The glow and trail's state between frames.
#[derive(Default)]
pub(super) struct LeapGlow {
    /// 0 to 1: how much of the glow is on.
    w: f32,
    clock: f32,
    /// The afterimages: where, his yaw, what is left of each (1 to 0).
    trail: Vec<(Vec3, f32, f32)>,
}

/// Whether the hero is untouchable by his leap right now: in the air in an evade (not a bare leap).
pub(super) fn untouchable(s: &SceneState) -> bool {
    s.hero.airborne() && s.hero_fighter.is_some_and(|f| matches!(f.state, State::Evade { .. }) && !f.bare_leap())
}

impl WorldView {
    /// Brings the glow on or off and lays the trail while he is untouchable.
    pub(super) fn update_leap_glow(&mut self, dt: f32, s: &SceneState) {
        let on = untouchable(s);
        let (sink, yaw) = (self.pit_sink(), self.hero_yaw);
        let g = &mut self.leap_glow;
        g.w = if on { (g.w + dt * GLOW_IN).min(1.0) } else { (g.w - dt * GLOW_OUT).max(0.0) };
        for t in g.trail.iter_mut() {
            t.2 -= dt * TRAIL_FADE;
        }
        g.trail.retain(|t| t.2 > 0.0);
        g.clock += dt;
        if on && g.clock >= TRAIL_EVERY {
            g.clock = 0.0;
            let p = s.hero.pos();
            g.trail.insert(0, (Vec3::new(p.x as f32, s.hero.altitude as f32 * STEP - sink, p.y as f32), yaw, 1.0));
            g.trail.truncate(TRAIL_MAX);
        }
        if !on && g.w <= 0.0 && g.trail.is_empty() {
            g.clock = TRAIL_EVERY; // the first afterimage of the next leap at once
        }
    }

    /// What the glow adds to his rig's emissive this frame.
    pub(super) fn leap_glow(&self) -> Vec3 {
        lin(GLOW) * GLOW_STRENGTH * self.leap_glow.w
    }

    /// The afterimages behind him, in his pose of the moment, pale and fading.
    pub(super) fn emit_leap_trail(&self, f: &mut Frame) {
        let Some((jointed, mats)) = self.clip_pose() else { return };
        for (p, yaw, life) in &self.leap_glow.trail {
            let root = Mat4::from_translation(*p) * Mat4::from_rotation_y(*yaw) * Mat4::from_scale(self.hero_squash());
            let alpha = TRAIL_ALPHA * life * life;
            f.rigs_blend.push_matrices(jointed, mats, root, lin(GLOW), alpha, lin(GLOW) * 0.5, rig_look::TINT_ALL, 0.0);
        }
    }

    /// The glow's weight and the afterimages kept (tests).
    #[cfg(test)]
    pub fn leap_glow_state(&self) -> (f32, usize) {
        (self.leap_glow.w, self.leap_glow.trail.len())
    }
}
