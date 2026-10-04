//! Springs for overshoot (WARBAND.md §4.5): a pose that is kicked swings past where it rests and
//! settles, so a blow has weight and a swing has mass. Two kinds: a `Spring` with state, stepped on
//! the view's time while the game runs (frozen with it: a hitstop or a slowing of time holds it
//! too), and `step_response`, the same spring worked out in closed form from a count of frames,
//! for a pose driven by a fighter's frame (a slam's squash on its first active frame).

use glam::Vec3;

/// A damped spring about rest (0): position, velocity, stiffness (ω², per second squared) and
/// damping ratio (below 1 it overshoots).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring {
    pub x: f32,
    pub v: f32,
    pub k: f32,
    pub zeta: f32,
}

/// The longest sub-step a spring takes (stiff springs stay stable at any frame time).
const SUBSTEP: f32 = 1.0 / 240.0;

impl Spring {
    pub fn new(k: f32, zeta: f32) -> Self {
        Self { x: 0.0, v: 0.0, k, zeta }
    }

    /// A spring that settles within `secs` at damping `zeta`.
    pub fn settling(zeta: f32, secs: f32) -> Self {
        Self::new(crate::logic::juice::stiffness_for(zeta, secs), zeta)
    }

    /// A blow: `speed` added to its velocity.
    pub fn kick(&mut self, speed: f32) {
        self.v += speed;
    }

    /// `dt` seconds on: exactly, for a spring that overshoots (so steps of any size agree), else
    /// by sub-steps of semi-implicit Euler.
    pub fn step(&mut self, dt: f32) {
        if dt <= 0.0 || self.settled() {
            return;
        }
        let w = self.k.sqrt();
        if self.zeta < 0.999 {
            let a = self.zeta * w;
            let wd = w * (1.0 - self.zeta * self.zeta).sqrt();
            let (e, c, s) = ((-a * dt).exp(), (wd * dt).cos(), (wd * dt).sin());
            let (x0, v0) = (self.x, self.v);
            self.x = e * (x0 * c + (v0 + a * x0) / wd * s);
            self.v = e * (v0 * c - (a * v0 + self.k * x0) / wd * s);
        } else {
            let c = 2.0 * self.zeta * w;
            let n = (dt / SUBSTEP).ceil().clamp(1.0, 64.0);
            let h = dt / n;
            for _ in 0..n as usize {
                self.v += (-self.k * self.x - c * self.v) * h;
                self.x += self.v * h;
            }
        }
        if self.settled() {
            self.x = 0.0;
            self.v = 0.0;
        }
    }

    /// At rest, near enough.
    pub fn settled(&self) -> bool {
        self.x.abs() < 1e-4 && self.v.abs() < 1e-3
    }

    /// The speed that throws it out to about `reach` at its first swing (for a kick sized by
    /// how far it should go rather than how fast).
    pub fn speed_for(&self, reach: f32) -> f32 {
        let w = self.k.sqrt();
        let z = self.zeta.clamp(0.0, 0.99);
        let wd = w * (1.0 - z * z).sqrt();
        // The impulse response peaks at t = atan(wd / (z w)) / wd.
        let t = (wd / (z * w).max(1e-6)).atan() / wd.max(1e-6);
        let peak = (-z * w * t).exp() * (wd * t).sin() / wd.max(1e-6);
        reach / peak.max(1e-6)
    }
}

/// Three springs, one per axis: the camera's kick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring3 {
    pub x: Spring,
    pub y: Spring,
    pub z: Spring,
}

impl Spring3 {
    pub fn settling(zeta: f32, secs: f32) -> Self {
        let s = Spring::settling(zeta, secs);
        Self { x: s, y: s, z: s }
    }

    pub fn kick(&mut self, v: Vec3) {
        self.x.kick(v.x);
        self.y.kick(v.y);
        self.z.kick(v.z);
    }

    pub fn step(&mut self, dt: f32) {
        self.x.step(dt);
        self.y.step(dt);
        self.z.step(dt);
    }

    pub fn value(&self) -> Vec3 {
        Vec3::new(self.x.x, self.y.x, self.z.x)
    }
}

/**
 * A spring's answer to a step, `t` seconds after it: from 0 it rises to 1, past it by the
 * overshoot its damping gives, and settles. In closed form, so a pose driven by a fighter's frame
 * gets the same curve whatever the frame time, and holds still when the frames do.
 */
pub fn step_response(t: f32, k: f32, zeta: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    let w = k.sqrt();
    let z = zeta.clamp(0.0, 0.999);
    let wd = w * (1.0 - z * z).sqrt();
    1.0 - (-z * w * t).exp() * ((wd * t).cos() + z / (1.0 - z * z).sqrt() * (wd * t).sin())
}

/// A squash that springs back: `from` at `t` = 0 (seconds), through rest (1) and past it by the
/// overshoot `zeta` gives, settling at 1 within about `settle` seconds.
pub fn squash(t: f32, from: f32, zeta: f32, settle: f32) -> f32 {
    let k = crate::logic::juice::stiffness_for(zeta, settle);
    from + (1.0 - from) * step_response(t, k, zeta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::juice::*;

    #[test]
    fn a_kicked_spring_overshoots_and_settles() {
        let mut s = Spring::settling(FLINCH_ZETA, FLINCH_SETTLE);
        s.kick(s.speed_for(1.0));
        let (mut peak, mut low) = (0.0f32, 0.0f32);
        for _ in 0..60 {
            s.step(1.0 / 240.0);
            peak = peak.max(s.x);
            low = low.min(s.x);
        }
        assert!((peak - 1.0).abs() < 0.05, "it reaches about the reach asked: {peak}");
        assert!(low < -0.1, "and swings back past rest: {low}");
        for _ in 0..240 {
            s.step(1.0 / 240.0);
        }
        assert!(s.settled() && s.x == 0.0);
    }

    #[test]
    fn a_frozen_spring_holds() {
        let mut s = Spring::settling(0.4, 0.25);
        s.kick(3.0);
        s.step(0.02);
        let held = s;
        s.step(0.0);
        assert_eq!(s, held);
    }

    #[test]
    fn steps_of_any_size_agree() {
        // The view's step varies; a spring must land in the same place.
        let (mut a, mut b) = (Spring::settling(0.4, 0.25), Spring::settling(0.4, 0.25));
        a.kick(2.0);
        b.kick(2.0);
        for _ in 0..12 {
            a.step(1.0 / 120.0);
        }
        b.step(0.1);
        assert!((a.x - b.x).abs() < 1e-3, "{} {}", a.x, b.x);
    }

    #[test]
    fn the_slam_squash_overshoots_to_its_peak() {
        let z = zeta_for_overshoot((SLAM_PEAK - 1.0) / (1.0 - SLAM_SQUASH));
        assert_eq!(squash(0.0, SLAM_SQUASH, z, 0.25), SLAM_SQUASH);
        let peak = (1..200).map(|k| squash(k as f32 / 400.0, SLAM_SQUASH, z, 0.25)).fold(0.0f32, f32::max);
        assert!((peak - SLAM_PEAK).abs() < 2e-3, "{peak}");
        assert!((squash(2.0, SLAM_SQUASH, z, 0.25) - 1.0).abs() < 1e-3);
    }
}
