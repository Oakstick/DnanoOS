"""Posing Ragnar by script: a pose is every bone's turn relative to its parent (in the game's axes,
as `vh_blender.set_local` keys them) and the hips' offset. Built the way an animator thinks:

* the torso by angles (degrees: x leans forward, y turns toward his left, z tips his left side up);
* the legs by where each ankle goes, solved by a two-bone IK that keeps the knee forward, the foot
  turned as asked in the model (flat on the floor unless told otherwise);
* the arms and the weapon by where they point: a limb's direction (and, about it, which way its
  front, or the shield's face, or the blade's flat looks), given in the chest's frame, so a turn of
  the torso carries them.

Coordinates are the game's: X his left, Y up, Z forward, metres from his feet's middle.
"""
import math

from mathutils import Matrix, Quaternion, Vector

import vh_blender as vh

X, Y, Z = Vector((1, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1))


def v(*c):
    return Vector(c)


def basis(p, s):
    """An orthonormal frame whose first axis is `p` and second as near `s` as can be."""
    p = p.normalized()
    s = s - p * s.dot(p)
    if s.length < 1e-6:
        s = Z if abs(p.dot(Z)) < 0.9 else X
        s = s - p * s.dot(p)
    s.normalize()
    return Matrix((p, s, p.cross(s))).transposed()


def look(p_rest, p_world, s_rest, s_world):
    """The turn taking `p_rest` exactly onto `p_world`, and `s_rest` as near `s_world` as it can."""
    return (basis(Vector(p_world), Vector(s_world)) @ basis(Vector(p_rest), Vector(s_rest)).transposed()).to_quaternion()


class Rig:
    """Ragnar's joints at rest (game axes), read off the armature."""

    def __init__(self, arm):
        self.j = {b: vh.joint(arm, b) for b in vh.BONES}
        j = self.j
        self.rest_dir = {
            "upper_arm.L": (j["forearm.L"] - j["upper_arm.L"]).normalized(),
            "forearm.L": (j["hand.L"] - j["forearm.L"]).normalized(),
            "hand.L": (j["offhand"] - j["hand.L"]).normalized(),
            "upper_arm.R": (j["forearm.R"] - j["upper_arm.R"]).normalized(),
            "forearm.R": (j["hand.R"] - j["forearm.R"]).normalized(),
            "hand.R": (j["blade"] - j["hand.R"]).normalized(),
            # The weapons point up out of the fist as they are sculpted, leaning forward.
            "blade": vh.WEAPON_REST.copy(),
            "offhand": vh.WEAPON_REST.copy(),
        }
        self.thigh = (j["shin.L"] - j["thigh.L"]).length
        self.shin = (j["foot.L"] - j["shin.L"]).length


class Pose:
    def __init__(self, rig):
        self.rig = rig
        self.q = {b: Quaternion() for b in vh.BONES}
        self.loc = Vector((0, 0, 0))

    def copy(self):
        p = Pose(self.rig)
        p.q = {b: q.copy() for b, q in self.q.items()}
        p.loc = self.loc.copy()
        return p

    def euler(self, bone, x=0.0, y=0.0, z=0.0):
        self.q[bone] = vh.quat_g(x, y, z)
        return self

    def world(self):
        """Every bone's turn in the model, and every joint's place, as the pose has them."""
        j = self.rig.j
        W, P = {}, {}
        for b in vh.BONES:
            parent = vh.PARENTS.get(b)
            if parent is None:
                W[b] = self.q[b].copy()
                P[b] = j[b] + self.loc
            else:
                W[b] = W[parent] @ self.q[b]
                P[b] = P[parent] + W[parent] @ (j[b] - j[parent])
        return W, P

    def aim(self, bone, d, s_rest, s, frame="chest"):
        """Points `bone` along `d` with its `s_rest` side looking toward `s` (both in `frame`'s axes)."""
        W, _ = self.world()
        f = W[frame] if frame else Quaternion()
        want = look(self.rig.rest_dir[bone], f @ Vector(d), Vector(s_rest), f @ Vector(s))
        self.q[bone] = W[vh.PARENTS[bone]].inverted() @ want
        return self

    def arm(self, side, upper, fore, upper_front=(0, 0, 1), fore_front=(0, 0, 1), fore_rest_front=Z, frame="chest"):
        """An arm: the upper arm along `upper`, the forearm along `fore` (directions in the chest's
        frame), each limb's front (rest +Z) toward the `*_front` given; the shield arm can turn its
        shield's face (the forearm's +X) instead with `fore_rest_front=X`."""
        self.aim(f"upper_arm.{side}", upper, Z, upper_front, frame)
        self.aim(f"forearm.{side}", fore, fore_rest_front, fore_front, frame)
        # The fist carries on along the forearm.
        self.q[f"hand.{side}"] = Quaternion()
        return self

    def weapon(self, d, flat, bone="blade", frame="chest"):
        """The weapon in the fist pointing along `d`, the flat of its blade (its rest +Z) toward `flat`."""
        return self.aim(bone, d, Z, flat, frame)

    def leg(self, side, ankle, foot_pitch=0.0, foot_yaw=0.0):
        """Puts the ankle at `ankle` (model space), knee forward; the foot turned `foot_pitch`
        degrees (toe down positive) and `foot_yaw` in the model."""
        r, j = self.rig, self.rig.j
        hips_q = self.q["hips"]
        hip = j["hips"] + self.loc + hips_q @ (j[f"thigh.{side}"] - j["hips"])
        t = hips_q.inverted() @ (Vector(ankle) - hip)
        gamma = math.atan2(t.x, -t.y)
        tr = Quaternion(Z, -gamma) @ t
        l1, l2 = r.thigh, r.shin
        d = min(max(tr.length, abs(l1 - l2) + 1e-4), l1 + l2 - 1e-4)
        phi = math.atan2(-tr.z, -tr.y)
        delta = math.acos(max(-1.0, min(1.0, (l1 * l1 + d * d - l2 * l2) / (2 * l1 * d))))
        inner = math.acos(max(-1.0, min(1.0, (l1 * l1 + l2 * l2 - d * d) / (2 * l1 * l2))))
        alpha, beta = phi - delta, math.pi - inner
        self.q[f"thigh.{side}"] = Quaternion(Z, gamma) @ Quaternion(X, alpha)
        self.q[f"shin.{side}"] = Quaternion(X, beta)
        want = Quaternion(Y, math.radians(foot_yaw)) @ Quaternion(X, math.radians(foot_pitch))
        self.q[f"foot.{side}"] = (hips_q @ self.q[f"thigh.{side}"] @ self.q[f"shin.{side}"]).inverted() @ want
        return self

    def leg_fk(self, side, thigh, shin, foot=0.0, roll=0.0):
        """A leg by angles (degrees): the thigh forward (negative) or back, the shin's bend, the foot."""
        self.q[f"thigh.{side}"] = Quaternion(Z, math.radians(roll)) @ Quaternion(X, math.radians(thigh))
        self.q[f"shin.{side}"] = Quaternion(X, math.radians(shin))
        self.q[f"foot.{side}"] = Quaternion(X, math.radians(foot))
        return self

    def key(self, arm, frame, interp="BEZIER", easing="AUTO"):
        pose = dict(self.q)
        pose["loc"] = tuple(self.loc)
        return vh.key_pose(arm, frame, pose, interp, easing)


def mix(a, b, u):
    """Pose `a` turned `u` of the way to pose `b`."""
    p = a.copy()
    for k in p.q:
        p.q[k] = a.q[k].slerp(b.q[k], u)
    p.loc = a.loc.lerp(b.loc, u)
    return p
