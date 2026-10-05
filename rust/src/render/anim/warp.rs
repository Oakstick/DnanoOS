//! The time warp: frame data rules the clip, never the reverse (COMBAT.md §0). A strike's clip is
//! stretched and squeezed piecewise linearly between its markers so that its wind-up ends exactly
//! when the move's startup ends, its strike spans the active frames and its follow-through the
//! recovery, at whatever tempo the move is swung.

/// Time `t` (seconds into a move) as time in its clip: the move's phase boundaries `game` (from 0,
/// ascending: the startup's end, the active frames' end, the recovery's end) go to the clip's
/// markers `clip` (the same number, from 0), and the time between two boundaries maps linearly onto
/// the time between their markers. Before the first boundary it is the clip's first; past the last,
/// the clip's last. A phase of no length is skipped.
pub fn warp(t: f32, game: &[f32], clip: &[f32]) -> f32 {
    debug_assert_eq!(game.len(), clip.len());
    let n = game.len().min(clip.len());
    if n == 0 {
        return 0.0;
    }
    if t <= game[0] {
        return clip[0];
    }
    for k in 1..n {
        if t <= game[k] {
            let span = game[k] - game[k - 1];
            if span <= 0.0 {
                return clip[k];
            }
            let u = (t - game[k - 1]) / span;
            return clip[k - 1] + (clip[k] - clip[k - 1]) * u;
        }
    }
    clip[n - 1]
}

/// The phase boundaries of a move of `startup`, `active` and `recovery` seconds: where its strike
/// begins, where it ends, and where the move ends.
pub fn move_bounds(startup: f32, active: f32, recovery: f32) -> [f32; 4] {
    [0.0, startup, startup + active, startup + active + recovery]
}

#[cfg(test)]
mod tests {
    use super::*;

    const F: f32 = 1.0 / 60.0;

    #[test]
    fn the_markers_land_on_the_moves_frames_at_every_tempo() {
        // A clip: the wind-up ends on frame 12, the strike on 18, the settle on 40.
        let clip = [0.0, 12.0 * F, 18.0 * F, 40.0 * F];
        // L1 as COMBAT.md §1.1 has it, then swung faster and slower (haste, a heavy blade).
        for tempo in [1.0f32, 0.67, 0.75, 1.3, 2.0] {
            let (s, a, r) = ((6.0 * tempo).round() * F, (4.0 * tempo).round().max(1.0) * F, (14.0 * tempo).round() * F);
            let game = move_bounds(s, a, r);
            for (g, c) in game.iter().zip(clip) {
                assert!((warp(*g, &game, &clip) - c).abs() < 1e-6, "tempo {tempo}: {g} went to {}", warp(*g, &game, &clip));
            }
            // The first active frame shows the strike begun, never the wind-up.
            assert!(warp(s + 1e-4, &game, &clip) > clip[1]);
            // Halfway through recovery, halfway through the follow-through.
            let mid = warp(s + a + r / 2.0, &game, &clip);
            assert!((mid - (clip[2] + clip[3]) / 2.0).abs() < 1e-5);
            // Time only runs forward.
            let mut last = -1.0;
            for k in 0..=200 {
                let t = game[3] * k as f32 / 200.0;
                let w = warp(t, &game, &clip);
                assert!(w >= last);
                last = w;
            }
        }
    }

    #[test]
    fn outside_the_move_it_holds_the_ends() {
        let (game, clip) = (move_bounds(0.1, 0.05, 0.2), [0.0, 0.3, 0.4, 0.9]);
        assert_eq!(warp(-1.0, &game, &clip), 0.0);
        assert_eq!(warp(5.0, &game, &clip), 0.9);
        // A move with no active frames jumps from the wind-up's end to the strike's.
        let none = move_bounds(0.1, 0.0, 0.2);
        assert_eq!(warp(0.1, &none, &clip), 0.3);
        assert!(warp(0.1001, &none, &clip) > 0.4);
    }
}
