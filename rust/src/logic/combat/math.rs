//! The formulas of COMBAT.md §2: power, damage with its state multiplier and proration,
//! hitstun decay, poise regeneration, and the cone every melee hitbox is.

use crate::logic::types::V;

/// Damage on a staggered or deflected foe, and on one caught in its recovery.
pub const VULNERABLE_MULT: f64 = 1.5;
pub const PUNISH_MULT: f64 = 1.25;
/// Each hit of a combo after the first deals this much less, down to the floor.
pub const PRORATION_STEP: f64 = 0.08;
pub const PRORATION_FLOOR: f64 = 0.6;
/// Each hit of a combo keeps the struck reeling this share of the hit before's hitstun, never under the floor.
pub const HITSTUN_DECAY: f64 = 0.8;
pub const HITSTUN_FLOOR: u16 = 4;
/// Frames after the last hit before poise starts to come back, and frames it takes from empty to full.
pub const POISE_REGEN_DELAY: u16 = 60;
pub const POISE_REGEN_FRAMES: f64 = 90.0;
/// Frames out of hitstun and stagger after which a combo is over (the count starts again).
pub const COMBO_RESET: u16 = 30;

/// How hard the hero's blade bites: its tier and its edge (COMBAT.md §2, `P`).
pub fn power(tier: u8, edge: f64) -> f64 {
    (1.0 + 0.15 * (tier.max(1) - 1) as f64) * (0.4 + 0.6 * edge.clamp(0.0, 1.0))
}

/// Damage share of the `n`-th hit of a combo (1-based).
pub fn proration(n: u16) -> f64 {
    (1.0 - PRORATION_STEP * (n.max(1) - 1) as f64).max(PRORATION_FLOOR)
}

/// What a blow of `base` deals: its power, the state of the struck (`state`) and its place in the combo.
pub fn damage(base: f64, power: f64, state: f64, n: u16) -> f64 {
    (base * power * state * proration(n)).round()
}

/// Hitstun of the `n`-th hit of a combo, from the move's own.
pub fn hitstun(base: u16, n: u16) -> u16 {
    ((base as f64 * HITSTUN_DECAY.powi(n.max(1) as i32 - 1)).round() as u16).max(HITSTUN_FLOOR)
}

/// The hero's hitstun from a blow of `damage`.
pub fn hero_hitstun(damage: f64) -> u16 {
    10 + (damage / 2.0).round() as u16
}

/// The hero's blockstun from a blow of `damage`.
pub fn blockstun(damage: f64) -> u16 {
    6 + (damage / 3.0).round() as u16
}

// ------------------------------------------------------------------ vectors

pub fn add(a: V, b: V) -> V {
    V::new(a.x + b.x, a.y + b.y)
}
pub fn sub(a: V, b: V) -> V {
    V::new(a.x - b.x, a.y - b.y)
}
pub fn scale(a: V, k: f64) -> V {
    V::new(a.x * k, a.y * k)
}
pub fn dot(a: V, b: V) -> f64 {
    a.x * b.x + a.y * b.y
}
pub fn len(a: V) -> f64 {
    dot(a, a).sqrt()
}
/// `a` made one tile long (or zero, if it has no length).
pub fn norm(a: V) -> V {
    let l = len(a);
    if l < 1e-9 {
        V::new(0.0, 0.0)
    } else {
        scale(a, 1.0 / l)
    }
}
/// The unit vector at angle `a` (radians, in the grid's x/y plane).
pub fn from_angle(a: f64) -> V {
    V::new(a.cos(), a.sin())
}
pub fn angle(a: V) -> f64 {
    a.y.atan2(a.x)
}

/// Turns unit vector `from` toward `to` by at most `max` radians.
pub fn turn_toward(from: V, to: V, max: f64) -> V {
    if len(to) < 1e-9 {
        return from;
    }
    let (a, b) = (angle(from), angle(to));
    let mut d = b - a;
    while d > std::f64::consts::PI {
        d -= std::f64::consts::TAU;
    }
    while d < -std::f64::consts::PI {
        d += std::f64::consts::TAU;
    }
    from_angle(a + d.clamp(-max, max))
}

/**
 * The melee hitbox: is a body of `radius` at `target` inside the cone from `origin` along
 * unit `aim`, `reach` tiles long and `half_angle` either side? The body's own width counts at
 * both the rim and the edges.
 */
pub fn in_cone(origin: V, aim: V, target: V, radius: f64, reach: f64, half_angle: f64) -> bool {
    let to = sub(target, origin);
    let d = len(to);
    if d > reach + radius {
        return false;
    }
    if d <= radius {
        return true;
    }
    let off = (dot(to, aim) / d).clamp(-1.0, 1.0).acos();
    off <= half_angle + (radius / d).min(1.0).asin()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_scales_with_power_state_and_the_combo() {
        assert_eq!(power(1, 1.0), 1.0);
        assert!((power(10, 1.0) - 2.35).abs() < 1e-9);
        assert!((power(1, 0.0) - 0.4).abs() < 1e-9, "a rusted blade bites at 40%");
        assert_eq!(damage(10.0, 1.0, 1.0, 1), 10.0);
        assert_eq!(damage(16.0, 1.0, VULNERABLE_MULT, 1), 24.0, "L3 on a staggered foe");
        assert_eq!(damage(10.0, 1.0, PUNISH_MULT, 1), 13.0, "a whiff punish (12.5 rounds up)");
        assert_eq!(proration(3), 0.84);
        assert_eq!(proration(20), PRORATION_FLOOR);
    }

    #[test]
    fn hitstun_decays_so_no_string_is_infinite() {
        let seq: Vec<u16> = (1..=8).map(|n| hitstun(14, n)).collect();
        assert_eq!(seq, [14, 11, 9, 7, 6, 5, 4, 4]);
        // L1 -> L2 is a true combo; by the fourth hit the hitstun no longer covers the gap between swings.
        let (l1, l2) = (crate::logic::combat::MoveId::L1.data(), crate::logic::combat::MoveId::L2.data());
        let gap = l1.cancel_frame() + l2.startup - l1.startup;
        assert!(hitstun(14, 1) > gap, "{} > {gap}", hitstun(14, 1));
        assert!(hitstun(14, 4) < gap, "{} < {gap}", hitstun(14, 4));
        assert_eq!(hero_hitstun(30.0), 25);
        assert_eq!(blockstun(15.0), 11);
    }

    #[test]
    fn the_cone_counts_the_bodys_width() {
        let o = V::new(0.0, 0.0);
        let east = V::new(1.0, 0.0);
        assert!(in_cone(o, east, V::new(1.4, 0.0), 0.3, 1.5, 0.5));
        assert!(in_cone(o, east, V::new(1.75, 0.0), 0.3, 1.5, 0.5), "the rim reaches the body");
        assert!(!in_cone(o, east, V::new(1.9, 0.0), 0.3, 1.5, 0.5));
        assert!(!in_cone(o, east, V::new(-1.0, 0.0), 0.3, 1.5, 1.0), "nothing behind");
        assert!(in_cone(o, east, V::new(1.0, 0.75), 0.3, 1.5, 0.5), "the edge of the body inside the edge of the cone");
        let t = turn_toward(east, V::new(0.0, 1.0), 0.1);
        assert!((angle(t) - 0.1).abs() < 1e-9);
    }
}
