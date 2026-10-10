//! Joint tables built at start-up rather than written out as constants (ANIMATION.md §11): for the
//! later foes, whose limbs are sculpted bent (a kappa's arms thrust forward, a giant's knees bowed),
//! each limb is given as the chain of its joints and the cuts between its bones are planes through
//! each joint square to the bend (the bisector of the bones on either side), so an arm reaching
//! forward is cut across its own length, not across the floor's. A body may also have bones of its
//! own past the heroes' twenty (a jaw, a tail, wings, the kappa's water), which hang from one of
//! them. Built once per foe and kept for the game's life (`Box::leak`).

use super::skeleton::{Bone, Cut, Joints, SkeletonDef, BONES, HUMANOID, PARENTS};

/// A joint table being built.
pub struct Build {
    bones: Vec<&'static str>,
    parents: Vec<Option<usize>>,
    legs: Vec<usize>,
    at: Vec<[f32; 3]>,
    cuts: Vec<Cut>,
    from: Vec<(&'static str, usize)>,
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn norm(a: [f32; 3]) -> [f32; 3] {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt().max(1e-6);
    [a[0] / l, a[1] / l, a[2] / l]
}

/// The plane between two bones at their joint: square to the bisector of their directions.
pub fn bend(a: [f32; 3], joint: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let (u, v) = (norm(sub(joint, a)), norm(sub(c, joint)));
    norm([u[0] + v[0], u[1] + v[1], u[2] + v[2]])
}

impl Build {
    /// The heroes' twenty bones, the humanoid foes' code parts mapped onto them (`leftArm` and
    /// `leftLeg` at the sculpt's low x: the foe's own right), every joint to be set.
    pub fn humanoid() -> Build {
        Build {
            bones: BONES.to_vec(),
            parents: PARENTS.to_vec(),
            legs: HUMANOID.legs.to_vec(),
            at: vec![[0.0; 3]; BONES.len()],
            cuts: Vec::new(),
            from: vec![
                ("body", Bone::Hips.index()),
                ("leftLeg", Bone::ThighR.index()),
                ("rightLeg", Bone::ThighL.index()),
                ("leftArm", Bone::UpperArmR.index()),
                ("rightArm", Bone::UpperArmL.index()),
                ("head", Bone::Head.index()),
            ],
        }
    }

    /// A body of its own: its root.
    pub fn new(root: &'static str, at: [f32; 3]) -> Build {
        Build { bones: vec![root], parents: vec![None], legs: Vec::new(), at: vec![at], cuts: Vec::new(), from: vec![("body", 0)] }
    }

    /// Bone `name` hanging from `parent`, its joint at `at`. Returns its index.
    pub fn bone(&mut self, name: &'static str, parent: usize, at: [f32; 3]) -> usize {
        self.bones.push(name);
        self.parents.push(Some(parent));
        self.at.push(at);
        self.bones.len() - 1
    }

    /// Bone `b`'s joint.
    pub fn at(&mut self, b: Bone, at: [f32; 3]) -> &mut Self {
        self.at[b.index()] = at;
        self
    }

    pub fn set(&mut self, b: usize, at: [f32; 3]) -> &mut Self {
        self.at[b] = at;
        self
    }

    /// The code rig's part `name` starts in bone `b`.
    pub fn part(&mut self, name: &'static str, b: usize) -> &mut Self {
        self.from.retain(|(n, _)| *n != name);
        self.from.push((name, b));
        self
    }

    /// Cuts applied before every other (a weapon out of an arm before its elbow is cut).
    pub fn cuts_first(&mut self, cuts: &[Cut]) -> &mut Self {
        self.cuts.splice(0..0, cuts.iter().copied());
        self
    }

    pub fn cut(&mut self, c: Cut) -> &mut Self {
        self.cuts.push(c);
        self
    }

    /// The bones the flinch leaves to the gait (the hips and legs; none for a small body).
    pub fn legs(&mut self, legs: &[usize]) -> &mut Self {
        self.legs = legs.to_vec();
        self
    }

    /**
     * A limb: `bones` (each the child of the one before, the first's parent already set) with their
     * joints at `points[0..n]`, and `points[n]` the limb's end; each joint's cut is a plane square to
     * the bend there, the voxels past it going down the limb (sleeved). `limit` keeps each cut to a
     * box (min, max in the grid), so a plane through a joint never takes another limb's voxels.
     */
    pub fn chain(&mut self, bones: &[usize], points: &[[f32; 3]], limit: Option<([f32; 3], [f32; 3])>) -> &mut Self {
        assert_eq!(points.len(), bones.len() + 1);
        for (k, &b) in bones.iter().enumerate() {
            self.at[b] = points[k];
        }
        for k in 1..bones.len() {
            let n = bend(points[k - 1], points[k], points[k + 1]);
            let mut c = Cut::slanted(bones[k - 1], bones[k], points[k], n, 0.0, false);
            if let Some((lo, hi)) = limit {
                c = c.across(lo, hi);
            }
            self.cuts.push(c);
        }
        self
    }

    /// The table, kept for the game's life.
    pub fn finish(self, name: &'static str, mirrored: bool) -> &'static Joints {
        let def: &'static SkeletonDef = Box::leak(Box::new(SkeletonDef {
            name,
            bones: Box::leak(self.bones.into_boxed_slice()),
            parents: Box::leak(self.parents.into_boxed_slice()),
            legs: Box::leak(self.legs.into_boxed_slice()),
        }));
        Box::leak(Box::new(Joints {
            def,
            at: Box::leak(self.at.into_boxed_slice()),
            cuts: Box::leak(self.cuts.into_boxed_slice()),
            from: Box::leak(self.from.into_boxed_slice()),
            mirrored,
            fists: false,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bend_is_square_to_both_bones_alike() {
        // An arm down to the elbow, then forward: the plane leans halfway between.
        let n = bend([0.0, 10.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 10.0]);
        let s = std::f32::consts::FRAC_1_SQRT_2;
        assert!((n[1] + s).abs() < 1e-5 && (n[2] - s).abs() < 1e-5, "{n:?}");
        // Straight, it is square to the limb.
        assert_eq!(bend([0.0, 2.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]), [0.0, -1.0, 0.0]);
    }
}
