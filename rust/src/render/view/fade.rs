//! Seeing him through the scenery: trees (and the vaults' walls) standing between the camera
//! and the hero dissolve, and grow back once the view is clear.

use super::*;

/// How see-through a tree gets while it blocks the view of Ragnar.
pub const TREE_FADE_MIN: f32 = 0.22;

/// Fade speed, in full fades per second.
const TREE_FADE_RATE: f32 = 6.0;

/// A tree on a wall tile, approximated as an upright cylinder for the camera test.
pub(super) struct Tree {
    pub(super) range: (usize, usize),
    pub(super) base: Vec3,
    pub(super) radius: f32,
    pub(super) height: f32,
    pub(super) fade: f32,
    pub(super) alive: bool,
    /// Each instance's own shade, for models with voxel materials (None: all plain lit).
    pub(super) shades: Option<Vec<f32>>,
}

impl Tree {
    /// Does the sight line from `eye` to `to` pass through this tree?
    pub(super) fn blocks(&self, eye: Vec3, to: Vec3) -> bool {
        // Cheap reject: the tree has to lie between the two ends in the ground plane.
        let r = self.radius;
        if self.base.x < eye.x.min(to.x) - r || self.base.x > eye.x.max(to.x) + r {
            return false;
        }
        if self.base.z < eye.z.min(to.z) - r || self.base.z > eye.z.max(to.z) + r {
            return false;
        }
        const SAMPLES: usize = 24;
        // Stop a little short of Ragnar so a tree right behind him never counts.
        for i in 0..SAMPLES {
            let p = eye.lerp(to, i as f32 / SAMPLES as f32 * 0.95);
            if p.y < self.base.y || p.y > self.base.y + self.height {
                continue;
            }
            let (dx, dz) = (p.x - self.base.x, p.z - self.base.z);
            if dx * dx + dz * dz < r * r {
                return true;
            }
        }
        false
    }
}

impl WorldView {
    /**
     * Trees standing between the camera and Ragnar dissolve so he is never
     * lost behind the woods, then grow back once the view is clear. The fade
     * is a screen-door dither in the opaque pass, so depth stays correct.
     */
    pub(super) fn update_tree_fade(&mut self, dt: f32, hero: &Hero) {
        let hp = hero.pos();
        // During the tour, clear the view of whatever the camera is showing.
        let feet = match self.tour {
            Some((target, _, _)) => target - Vec3::new(0.0, 0.3, 0.0),
            None => Vec3::new(hp.x as f32, hero.altitude as f32 * STEP, hp.y as f32),
        };
        let targets = [feet + Vec3::new(0.0, 0.25, 0.0), feet + Vec3::new(0.0, 0.6, 0.0), feet + Vec3::new(0.0, 1.0, 0.0)];
        let eye = self.cam_pos;
        let step = TREE_FADE_RATE * dt;
        for i in 0..self.trees.len() {
            let t = &self.trees[i];
            if !t.alive {
                continue;
            }
            // What the tour is showing never fades out of its own shot.
            let shown = self.tour.is_some() && Vec2::new(t.base.x - feet.x, t.base.z - feet.z).length() < t.radius + 0.3;
            let blocking = (hero.alive || self.tour.is_some()) && !shown && targets.iter().any(|&to| t.blocks(eye, to));
            let goal = if blocking { TREE_FADE_MIN } else { 1.0 };
            let fade = if t.fade < goal { (t.fade + step).min(goal) } else { (t.fade - step).max(goal) };
            if fade == t.fade {
                continue;
            }
            let (start, len) = t.range;
            self.trees[i].fade = fade;
            let solid = fade >= 0.999;
            let shades = self.trees[i].shades.as_deref();
            for (k, inst) in self.static_cubes[start..start + len].iter_mut().enumerate() {
                inst.color[3] = fade;
                inst.emissive[3] = if !solid { SHADE_DITHER } else { shades.map_or(SHADE_LIT, |s| s[k]) };
            }
            self.static_patches.push((start, len));
        }
    }

    /// How faded each standing tree is (1 = solid), for tests and tools.
    pub fn tree_fades(&self) -> impl Iterator<Item = f32> + '_ {
        self.trees.iter().filter(|t| t.alive).map(|t| t.fade)
    }

    /// Tiles with a tree standing on them (for tests).
    #[cfg(test)]
    pub fn tree_tiles(&self) -> Vec<P> {
        self.tree_index.keys().copied().collect()
    }
}
